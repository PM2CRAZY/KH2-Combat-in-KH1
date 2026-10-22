//! KH2 Sora's motions drawn on KH1 Sora, following what KH1 has him do.
//!
//! KH1 poses a character in `0x140391E20` (`(world matrix, instance, time f32, motion)`:
//! samples the motion, then builds the bone matrices in `0x140390260`) or, for characters with
//! ground IK, `0x140392710` (`(world matrix, instance, flags)`). The instance (`*u64`):
//! - `+0` points at a slot that points at the model header: `bone_count u16`, then a handle
//!   to the 0x30-byte bone table (`kh1_data::mdls`).
//! - `+8`: the motion playing (stored by the skeleton pass).
//! - `+0x1B0`: per bone (0x40 bytes) the sampled channels: scale `+0`, accumulated scale
//!   `+0x10`, Euler rotation `+0x20`, translation `+0x30`. Kept between frames (only animated
//!   channels are rewritten), so never written by us.
//! - `+0x1B4`: per bone, the world matrix the game draws (D3D row-vector layout, i.e. glam's
//!   column-major memory; the world is y-down).
//! - `+0x1B8`: per bone, its matrix relative to its parent (bone 0: the root's own, which the
//!   motion drives: hips height and a yaw, not the bind's rotation).
//! Handles are the game's 32-bit addresses: `table[(h & 0x7FFFFFFF) >> 25] | (h & 0x1FFFFFF)`,
//! the table at `exe + 0x2EE4730` (`0x14038B230`).
//!
//! The motion's counts name its slot in KH1 Sora's motion table (`kh1_data::mset`); `play_for`
//! gives KH2's motion for the same thing (slots read from the user's play log: field and
//! battle idle / walk / run, jump, fall, land, glide, the ground combo); every other slot
//! (magic, reactions, cutscenes...) keeps KH1's own animation for now.
//!
//! Model space → world is taken from the game each frame: the root's world matrix times the
//! inverse of its own matrix (`+0x1B4[0] * +0x1B8[0]⁻¹`), but KH1's blends between motions mix
//! the pose we drew back into that (the user: a split-second vanish at every change), so the
//! pass's own world matrix is used once it has agreed with that for 30 frames (`from_param`),
//! and meanwhile a one-frame jump of the bone-derived placement is ignored. Our model-space
//! pose goes into `+0x1B4` after the game's pass.
//!
//! The Keyblade is its own small model, placed from his hand. A small model whose world matrix
//! keeps a fixed relation to KH1's hand while the hand moves is taken as held, and from then
//! on is placed by the same relation to the hand we draw (`held`).

use super::addr::Rva;
use super::{hook, log, sys};
use glam::{Mat4, Quat, Vec3};
use kh1_data::retarget::Retarget;
use kh2_data::sora::Rig;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

const SORA_BONES: u32 = 293;
const POSE_RVA: Rva = Rva(0x391E20);
const POSE_IK_RVA: Rva = Rva(0x392710);
const HANDLE_TABLE_RVA: Rva = Rva(0x2EE4730);
const POSE_PROLOGUE: [u8; 7] = [0x4C, 0x8B, 0xDC, 0x53, 0x56, 0x41, 0x54];
const POSE_IK_PROLOGUE: [u8; 9] = [0x4C, 0x8B, 0xDC, 0x55, 0x53, 0x41, 0x54, 0x41, 0x57];
const WORLD: usize = 0x1B4;
const LOCAL: usize = 0x1B8;
/// KH1 Sora's hands (the −x side is his right, which holds the Keyblade).
const HANDS: [(usize, &str); 2] = [(101, "right"), (206, "left")];

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_POSE: AtomicUsize = AtomicUsize::new(0);
static ORIG_POSE_IK: AtomicUsize = AtomicUsize::new(0);

/// What Insert cycles through (starts on the first).
pub const MODES: [&str; 6] = [
    "KH2 controls Sora (KH2 movement; KH1's own actions keep KH1)",
    "KH1 controls Sora, KH2 animations on top",
    "off (KH1's own animation)",
    "KH1 bind pose (T-pose), upright",
    "KH2 idle (battle stance)",
    "KH2 run",
];
pub static MODE: AtomicU32 = AtomicU32::new(0);

/// How a KH1 slot is shown with KH2's motions.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Play {
    /// A KH2 motion; `field` takes its out-of-combat variant (no Keyblade) when there is one;
    /// slots sharing a `group` continue the same clip instead of restarting it.
    Motion { motion: u16, field: bool, looping: bool, group: u8 },
    /// A combo swing, stretched to the KH1 swing's length so hits stay in time: on the ground
    /// KH2's 151 / 152 alternate (its no-target rows) and 154 finishes; in the air 183 / 184
    /// alternate and 181 finishes.
    Swing { air: bool, finisher: bool },
}

/// KH1 Sora's slots, from the user's play logs (2026-10-13): 0 / 2 / 5 field idle, walk, run;
/// 1 / 3 / 6 the battle ones; 12 → 13 → 10 → 11 jump, rise, fall, land; 98 → 99 → 98 → 101 the
/// ground combo (100 assumed a third swing); 102 / 103 / 104 the air combo (the user: air
/// combos showed the glide they were first taken for). 4 / 7 are unseen and taken as battle
/// walk / run.
fn play_for(slot: usize) -> Option<Play> {
    use character_core::motion as m;
    let mo = |motion: u16, field: bool, looping: bool, group: u8| Play::Motion { motion, field, looping, group };
    Some(match slot {
        0 => mo(m::IDLE, true, true, 0),
        1 => mo(m::IDLE, false, true, 0),
        2 => mo(m::WALK, true, true, 0),
        3 | 4 => mo(m::WALK, false, true, 0),
        5 => mo(m::RUN, true, true, 0),
        6 | 7 => mo(m::RUN, false, true, 0),
        12 | 13 => mo(m::JUMP, false, false, 1),
        8 | 9 => mo(m::JUMP, false, false, 1),
        10 => mo(m::FALL, false, true, 0),
        11 => mo(m::LAND, false, false, 0),
        98..=100 => Play::Swing { air: false, finisher: false },
        101 => Play::Swing { air: false, finisher: true },
        102 | 103 => Play::Swing { air: true, finisher: false },
        104 => Play::Swing { air: true, finisher: true },
        _ => return None,
    })
}

/// A KH1 motion's counts, which tell Sora's motions apart (two pairs of unrelated slots
/// share theirs): frames, channels, helper channels, helpers.
type Signature = (u32, u32, u32, u32);
/// Slot by signature, and each slot's length in frames.
/// The skeleton pass's world matrix has been seen to place Sora right: from then on it is used
/// at once for every new Sora (each zone makes one). Re-learning it per zone placed him by
/// his bones meanwhile, which KH1's walk-in blends had mixed with our pose: he was drawn
/// turned or off to the side for a moment (the user: Sora occasionally turns invisible).
static PARAM_OK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SLOTS: OnceLock<(HashMap<Signature, usize>, Vec<u32>)> = OnceLock::new();
/// KH1's battle idle: its first frame is the rest pose of the bones no KH2 part turns (hood,
/// cloth, face, the Keyblade's grip and chain), `Retarget::set_rest`.
static REST: OnceLock<Option<kh1_data::mset::Motion>> = OnceLock::new();
const REST_SLOT: usize = 1;
/// KH1's field idle (slot 0): the thumbs' open-handed pose while no Keyblade is held.
static REST_OPEN: OnceLock<Option<kh1_data::mset::Motion>> = OnceLock::new();
/// KH1 Sora's whole motion table (`xa_ex_0010.mset`), for drawing his own jump under KH2
/// (`kh1_air_pose`).
static KH1_MOTIONS: OnceLock<Vec<Option<kh1_data::mset::Motion>>> = OnceLock::new();
/// The pose drawn this frame is one of KH1's own motions (`kh1_air_pose`): not turned as KH2's
/// Keyblade hold is.
static NATIVE_KH1: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Jump at KH1 (`takeover::KH1_AIR_SLOT`): KH1's own jump (12, then 13 once it has played), its
/// fall (10, looping) or its landing (11, then held), from KH1's motion table on his own
/// skeleton, timed from when KH2's state got there.
fn kh1_air_pose() -> Option<Vec<Mat4>> {
    let want = super::takeover::KH1_AIR_SLOT.load(Ordering::Relaxed) as usize;
    if want == 0 {
        return None;
    }
    let motions = KH1_MOTIONS.get()?;
    static CUR: Mutex<Option<(usize, Instant)>> = Mutex::new(None);
    let mut cur = CUR.lock().ok()?;
    // (The jump's take-off goes on into its rise; a new state starts its motion over.)
    let keep = matches!((cur.map(|c| c.0), want), (Some(12 | 13), 13) | (Some(12), 12));
    if !keep && cur.map(|c| c.0) != Some(want) {
        *cur = Some((want, Instant::now()));
    }
    let (mut slot, mut t0) = (*cur)?;
    let mut m = motions.get(slot)?.as_ref()?;
    let mut frame = t0.elapsed().as_secs_f32() * m.fps.max(1.0);
    if slot == 12 && frame >= m.frames as f32 {
        slot = 13;
        t0 = Instant::now();
        *cur = Some((slot, t0));
        m = motions.get(slot)?.as_ref()?;
        frame = 0.0;
    }
    let len = m.frames.max(1) as f32;
    let frame = if slot == 10 { frame % len } else { frame.min(len - 1.0) };
    let bones = KH1_BONES.lock().ok()?;
    if bones.is_empty() {
        return None;
    }
    let local = m.apply(&bones, frame);
    Some(kh2_anim::Pose::bind(&local).bones)
}

/// KH2 Sora: skeleton and motions, and each action clip's root travel per frame (KH1 moves
/// him itself, so a swing's lunge is taken out of the pose).
pub struct Kh2 {
    pub rig: Arc<Rig>,
    /// KH2 Sora's controller data (moveset, movement and growth numbers, combo rows).
    pub character: character_core::Character,
    /// KH2's attack rows (ATKP: power, reaction, re-hit interval, combo group).
    pub battle: Arc<kh2_formats::battle::Battle>,
    /// The spells' `magi` records (03system `pref`), by sub (`reflect`).
    pub magi: kh2_formats::pref::MagiTable,
    /// Wisdom Form (`form`): its model's moveset and motions, its script and effect archive.
    pub wisdom: Option<Form>,
    /// Valor Form (`form`, stage 211): its moveset (two Keyblades' swings) and motions.
    pub valor: Option<Form>,
    /// Limit Form (`form`: `P_EX100_KH1F`, moveset 0x42, one Keyblade).
    pub limit: Option<Form>,
    /// Master Form (`form`: `P_EX100_TRIF`, moveset 4, two Keyblades, the left one floating).
    pub master: Option<Form>,
    /// Final Form (`form`, stage 231: `P_EX100_ULTF`, its floating Keyblades).
    pub final_form: Option<Form>,
    /// Anti Form (`form`, the secret code's: `P_EX100_HTLF`, no Keyblade).
    pub anti: Option<Form>,
    roots: HashMap<u16, Vec<[f32; 3]>>,
    /// Walk and run (battle, field): ground speed in KH2 units per second, from the planted
    /// foot's travel against the hips.
    ground: HashMap<(u16, bool), f32>,
}

/// A Drive Form's data (KH2's `P_EX100_<form>` files): the same skeleton as Sora's.
pub struct Form {
    pub rig: Arc<Rig>,
    pub character: character_core::Character,
    pub battle: Arc<kh2_formats::battle::Battle>,
    /// The model's object script (BAR entry type 3: Wisdom's `wisd`, its shots).
    pub script: Option<Vec<u8>>,
    /// The form's effect archive (`.a.us`: the PAX "p_ex" its script's effects come from).
    pub archive: Option<Arc<Vec<u8>>>,
}

/// Sora's moves a form gets from his own tables (the user: "make sure that soras square launch
/// attack works in valor"): Upper Slash (combo row 37, kind 0, Square in a combo, motion 167,
/// ability 95). KH2's Valor has none of Sora's Square moves; its own kind-0 rows (12, 38, 40, 11)
/// are its finishers and guard, so their ids aren't taken.
const PORTED_TO_VALOR: [u8; 1] = [37];

/// Copies Sora's combo rows `rows` (and their motions: the clip, the body's and the first
/// Keyblade's animation) into a form's data, where the form has no row of that id (the same
/// skeleton, `load_form` checks it).
fn port_moves(f: &mut Form, sora: &character_core::Character, sora_rig: &Rig, rows: &[u8]) {
    let mut rig = (*f.rig).clone();
    for &id in rows {
        if f.character.moveset.combo_rows.iter().any(|r| r.id == id) {
            continue;
        }
        let Some(row) = sora.moveset.combo_rows.iter().find(|r| r.id == id).cloned() else { continue };
        let m = row.motion;
        if let Some(c) = sora.moveset.clips.get(&m) {
            f.character.moveset.clips.entry(m).or_insert_with(|| c.clone());
        }
        if let Some(a) = sora_rig.motions.get(&m) {
            rig.motions.entry(m).or_insert_with(|| a.clone());
        }
        if let (Some(w), Some(sw)) = (rig.weapons.get_mut(0), sora_rig.weapons.first()) {
            if let Some(a) = sw.motions.get(&m) {
                w.motions.entry(m).or_insert_with(|| a.clone());
            }
        }
        log(&format!("form: Sora's combo row {id} (motion {m}, ability {}) added", row.ability));
        f.character.moveset.combo_rows.push(row);
    }
    f.rig = Arc::new(rig);
}

fn load_form(install: &kh2_data::Install, source: kh2_data::Source, spec: kh2_data::sora::PlayerSpec, skeleton: usize) -> Result<Form, String> {
    let data = install.load_player(source, spec).map_err(|e| e.to_string())?;
    let rig = data.rig.clone().ok_or_else(|| data.rig_error.clone().unwrap_or_else(|| "no rig".into()))?;
    if rig.skeleton.len() != skeleton {
        return Err(format!("{} bones, Sora has {skeleton}", rig.skeleton.len()));
    }
    let script = install.read_any(source, &format!("obj/{}.mdlx", spec.model)).ok().and_then(|b| kh2_magic::SpellFile::bar_entry(&b, 3));
    let archive = install.read_any(source, &format!("obj/{}.a.us", spec.model)).ok().map(Arc::new);
    Ok(Form { rig, character: data.character.clone(), battle: Arc::new(data.combat.battle.clone()), script, archive })
}

/// One line per form: the collision groups its attack and guard windows switch on (`b` the
/// body, `w1`/`w2` a Keyblade) and any of them with no shape, with the windows that would hit
/// nothing (motion:ATKP). A modded model can carry another's table (Anti with Sora's has no
/// shape in groups 1, 4 and 7, `docs/drive-model.md`).
fn log_shape_groups(name: &str, rig: &Rig, moveset: &character_core::Moveset) {
    let groups = rig.shape_groups(moveset);
    let at = |g: &kh2_data::sora::ShapeGroup| if g.weapon == 0 { format!("b{}", g.group) } else { format!("w{}.{}", g.weapon, g.group) };
    let used: Vec<String> = groups.iter().map(at).collect();
    let gaps: Vec<String> = groups
        .iter()
        .filter(|g| g.shapes == 0)
        .map(|g| format!("{} ({})", at(g), g.windows.iter().map(|(m, a)| format!("{m}:{a}")).collect::<Vec<_>>().join(" ")))
        .collect();
    if gaps.is_empty() {
        log(&format!("KH2 {name} Form attack groups {}: each has a shape", used.join(" ")));
    } else {
        log(&format!("KH2 {name} Form attack groups {}: NO SHAPE for {}", used.join(" "), gaps.join(", ")));
    }
}

/// How fast a looping gait covers ground: each frame, the lower foot's horizontal travel
/// relative to bone 0 while it is within 4 units of its lowest (planted), averaged, × 60.
fn ground_speed(rig: &Rig, clip: &kh2_formats::anim::Interpolated) -> f32 {
    let feet = [16usize, 45]; // heels
    let len = kh2_anim::length_frames(clip).max(2.0) as usize;
    let poses: Vec<Vec<Mat4>> = (0..=len).map(|f| kh2_anim::evaluate(&rig.skeleton, clip, kh2_anim::key_time(clip, f as f32, true), kh2_anim::Options::default()).bones).collect();
    let at = |p: &Vec<Mat4>, b: usize| p.get(b).map_or(Vec3::ZERO, |m| m.w_axis.truncate());
    let mut sum = 0.0;
    let mut n = 0;
    for &foot in &feet {
        let low = poses.iter().map(|p| at(p, foot).y).fold(f32::MAX, f32::min);
        for w in poses.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            if at(a, foot).y < low + 4.0 && at(b, foot).y < low + 4.0 {
                let d = (at(b, foot) - at(b, 0)) - (at(a, foot) - at(a, 0));
                sum += Vec3::new(d.x, 0.0, d.z).length();
                n += 1;
            }
        }
    }
    if n == 0 { 0.0 } else { sum / n as f32 * 60.0 }
}
static KH2: OnceLock<Result<Kh2, String>> = OnceLock::new();

/// KH2 Sora, once loaded.
pub fn kh2() -> Option<&'static Kh2> {
    KH2.get()?.as_ref().ok()
}

/// The slot of KH1 Sora's motion playing now (as of his last skeleton pass).
static SLOT_NOW: AtomicUsize = AtomicUsize::new(usize::MAX);
/// A KH1 motion slot's length in frames (KH1 Sora's motion table).
pub fn slot_frames(slot: usize) -> Option<u32> {
    SLOTS.get()?.1.get(slot).copied().filter(|n| *n > 0)
}

pub fn sora_slot() -> Option<usize> {
    let s = SLOT_NOW.load(Ordering::Relaxed);
    (s != usize::MAX).then_some(s)
}

struct Sora {
    instance: usize,
    retarget: Option<Retarget>,
    bind: Vec<Mat4>,
    calls: u64,
    checked: bool,
    follow: Follow,
    /// The last model → world, and the root matrix we wrote with it.
    to_world: Option<Mat4>,
    wrote_root: Option<[f32; 16]>,
    /// The last pose we drew (model space) and when: KH1's own animation blends in from it
    /// (the user: going into KH1's ladder climb and other moves of its own snapped).
    last_drawn: Option<(Vec<Mat4>, Instant)>,
    /// The pass's own world matrix (its y and z flipped, axes unit length) agreed with the
    /// pose-derived placement for this many frames; trusted from 30 on.
    param_agree: u32,
    param_ok: bool,
    /// Pose-derived placements that jumped from the last (kept out unless they persist).
    jumps: u32,
    /// KH1's bone parents (for the feet).
    parents: Vec<i16>,
    /// The hair, hood and keychain's swing under KH2's poses (`kh1_data::jiggle`), and when it
    /// last moved.
    jiggle: Option<(kh1_data::jiggle::Jiggle, u64)>,
    /// KH1's hair, hood and keychain following KH2's own animation of them
    /// (`kh1_data::secondary`), built with the retarget.
    secondary: Option<kh1_data::secondary::Secondary>,
    /// KH1's poses on KH2's skeleton (`kh1_data::inverse`), for KH2's model while KH1 animates
    /// him; built with the retarget.
    inverse: Option<kh1_data::inverse::Inverse>,
    /// The KH2 pose (Sora's bones, the Keyblade's) the pose being drawn came from, if KH2's.
    kh2_now: Option<(Vec<Mat4>, Vec<Mat4>)>,
    /// Where this frame's pose comes from (`wanted`), and last frame's (`src`).
    src_now: u8,
    src: u8,
    /// The pose drawn last frame (model space), whoever drew it.
    shown: Vec<Mat4>,
    /// A change of source after one of KH1's grabs, blended from `shown` (`HANDBACK_BLEND`).
    handback: Option<(Vec<(Vec3, Quat, Vec3)>, Instant)>,
}

/// Who drew Sora's pose (`Sora::src`): nobody yet, KH1's own animation, KH2's motion over KH1's
/// slot (`follow`), KH2's controller (`takeover::POSE`), KH1's own jump over KH2's (`kh1_air_pose`).
mod src {
    pub const NONE: u8 = 0;
    pub const KH1: u8 = 1;
    pub const FOLLOW: u8 = 2;
    pub const KH2: u8 = 3;
    pub const KH1_AIR: u8 = 4;
}

/// Out of one of KH1's grabs (`takeover::grab_blend`: a ledge's pull-up, a ladder, a pole), the
/// change from KH1's own animation to ours, and from ours over KH1's slot to KH2's own frames,
/// blends in from the pose drawn just before over this long (seconds, eased): KH2's own 8 frames
/// between movement motions, a little longer. KH1's climb ends in its own fall and landing, then
/// KH2's idle took over at once (the user: "a small visual bug that happens when you go up from
/// a ledge grab").
const HANDBACK_BLEND: f32 = 10.0 / 60.0;

fn src_name(s: u8) -> &'static str {
    match s {
        src::KH1 => "KH1's own animation",
        src::FOLLOW => "KH2's motion over KH1's slot",
        src::KH2 => "KH2",
        src::KH1_AIR => "KH1's jump over KH2's",
        _ => "nothing",
    }
}

/// The pass's world matrix as the model → world the bones are drawn with: the pass flips the
/// world's second and third rows (`× -1`, `0x140430938`) and takes the rows' lengths out as
/// scale (`0x140391E20`).
unsafe fn from_param(world: usize) -> Option<Mat4> {
    if world < 0x10000 {
        return None;
    }
    let p = mat(world);
    let (x, y, z) = (p.x_axis.truncate(), p.y_axis.truncate(), p.z_axis.truncate());
    if !(x.length() > 1e-4 && y.length() > 1e-4 && z.length() > 1e-4) {
        return None;
    }
    let m = Mat4::from_cols(x.normalize().extend(0.0), (-y.normalize()).extend(0.0), (-z.normalize()).extend(0.0), p.w_axis);
    affine(&m).then_some(m)
}

/// `m` moved onto Sora's position (`+0x10`) when KH1's placement is off it (the game, not the
/// viewer modes; a gap from half a unit to 1500).
unsafe fn on_actor(m: Mat4) -> Mat4 {
    if MODE.load(Ordering::Relaxed) != 0 {
        return m;
    }
    // (Only over KH1's roll and just after it: elsewhere KH1's placement is its own, a ledge, a
    // ride, the water.)
    static LAST_ROLL: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
    let recent = match LAST_ROLL.try_lock() {
        Ok(mut g) => {
            if super::styleroll::active() {
                *g = Some(Instant::now());
            }
            g.is_some_and(|t| t.elapsed().as_secs_f32() < 0.6)
        }
        Err(_) => false,
    };
    if !recent {
        return m;
    }
    let a = super::takeover::sora_actor();
    if a < 0x10000 || !super::crash::readable(a + 0x10) {
        return m;
    }
    let p = Vec3::from(rd::<[f32; 3]>(a + 0x10));
    let gap = (p - m.w_axis.truncate()).length();
    if !p.is_finite() || !(0.5..1500.0).contains(&gap) {
        return m;
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 30 {
        log(&format!("placement {gap:.1} off Sora's position (KH1 slot {:?}): drawn at his position", SLOT_NOW.load(Ordering::Relaxed)));
    }
    let mut m = m;
    m.w_axis = p.extend(1.0);
    m
}

fn close(a: &Mat4, b: &Mat4, units: f32) -> bool {
    (a.w_axis - b.w_axis).length() < units && (a.x_axis - b.x_axis).length() < 0.05 && (a.y_axis - b.y_axis).length() < 0.05 && (a.z_axis - b.z_axis).length() < 0.05
}

/// Which KH2 motion follows KH1's, and how far into it.
#[derive(Default)]
struct Follow {
    slot: Option<usize>,
    play: Option<Play>,
    motion: u16,
    field: bool,
    since: Option<Instant>,
    /// Clip frames per real second (60, or stretched for swings).
    rate: f32,
    /// Ground combo swings in a row, and when the last one started.
    swings: u32,
    last_swing: Option<Instant>,
    /// Last KH2 pose (model space) and when it was drawn, and the pose blended from on a change.
    last: Vec<Mat4>,
    last_at: Option<Instant>,
    from: Vec<(Vec3, Quat, Vec3)>,
    blend_start: Option<Instant>,
    blend_secs: f32,
    /// Movement measured from the model origin (world units per second), smoothed.
    at: Option<(Vec3, Instant)>,
    speed: f32,
    climb: f32,
    logged: u32,
}

/// How long KH1's own animation takes to blend in from KH2's pose (seconds).
const KH1_BLEND: f32 = 0.2;
/// KH2's blends: 8 frames between movement motions, 16 into a glide, none into a swing.
const BLEND_MOVE: f32 = 8.0 / 60.0;
const BLEND_GLIDE: f32 = 16.0 / 60.0;
/// Swings more than this apart start a new combo.
const COMBO_GAP: f32 = 1.2;

static SORA: Mutex<Option<Sora>> = Mutex::new(None);
/// Sora's animation instance, and where the pass last placed him (model origin, world).
pub static SORA_INSTANCE: AtomicUsize = AtomicUsize::new(0);
static SORA_AT: Mutex<Option<[f32; 3]>> = Mutex::new(None);

/// Where the skeleton pass last placed Sora (his model origin in the world).
pub fn sora_world() -> Option<[f32; 3]> {
    *SORA_AT.lock().ok()?
}
static CLOCK: OnceLock<Instant> = OnceLock::new();

/// KH1's hands this frame (as the game posed them) and as we drew them.
#[derive(Clone, Copy)]
struct Hands {
    orig: [Mat4; 2],
    drawn: [Mat4; 2],
    at: Instant,
}
static HANDS_NOW: Mutex<Option<Hands>> = Mutex::new(None);

/// Sora's own body (KH1 Sora's 293-bone model) was posed within `ms` milliseconds: not while
/// he is in another body (the Heartless form in Hollow Bastion, the user: no jump animation).
pub fn sora_body_seen(ms: u128) -> bool {
    if super::takeover::resumed_within(800) {
        return true;
    }
    HANDS_NOW.try_lock().ok().and_then(|h| h.as_ref().map(|h| h.at.elapsed().as_millis() < ms)).unwrap_or(true)
}

/// Small models (≤ 40 bones) posed near Sora: when, where, and where their placement matrix
/// comes from (a bone of Sora's drawn matrices, if it points into them).
struct Small {
    bones: u32,
    at: Instant,
    pos: Vec3,
    from_bone: Option<usize>,
    dist_hand: f32,
}
static SMALL: Mutex<Option<HashMap<usize, Small>>> = Mutex::new(None);

unsafe fn note_small(instance: usize, world: usize) {
    let Some(bones) = bone_count(instance) else { return };
    if bones > 40 || bones == SORA_BONES || world < 0x10000 {
        return;
    }
    let w = mat(world);
    let pos = w.w_axis.truncate();
    let sora = SORA_INSTANCE.load(Ordering::Relaxed);
    let arr = if sora > 0x10000 { ptr(rd::<u32>(sora + WORLD)) } else { 0 };
    let from_bone = (arr > 0x10000 && world >= arr && world < arr + SORA_BONES as usize * 0x40).then(|| (world - arr) / 0x40);
    let Some(hands) = HANDS_NOW.lock().ok().and_then(|h| *h) else { return };
    let dist_hand = (pos - hands.drawn[0].w_axis.truncate()).length();
    let near_sora = SORA_AT.try_lock().ok().and_then(|a| *a).map_or(false, |a| (pos - Vec3::from(a)).length() < 400.0);
    if !near_sora && from_bone.is_none() {
        return;
    }
    if let Ok(mut g) = SMALL.try_lock() {
        let m = g.get_or_insert_with(HashMap::new);
        if m.len() < 64 || m.contains_key(&instance) {
            m.insert(instance, Small { bones, at: Instant::now(), pos, from_bone, dist_hand });
        }
    }
}

/// Sora's weapon bones (280, the grip under the right hand; 283 the first of the chain), as
/// KH1 posed them and as we drew them, and whether this frame's matrices were already ours.
static BLADE: Mutex<Option<(Mat4, Mat4, Mat4, Mat4, bool)>> = Mutex::new(None);

fn blade_report() -> String {
    let Some(Some((o0, o3, d0, d3, ours))) = BLADE.lock().ok().map(|b| *b) else { return "weapon bones not seen".into() };
    let h = HANDS_NOW.lock().ok().and_then(|h| *h);
    let det = |m: &Mat4| glam::Mat3::from_mat4(*m).determinant();
    let p = |m: &Mat4| m.w_axis.truncate();
    format!(
        "weapon bones: KH1's 280 at {:.0?} (det {:.2}, {:.0} from KH1's hand), 283 at {:.0?} (det {:.2}); drawn 280 at {:.0?} (det {:.2}, {:.0} from the drawn hand), 283 at {:.0?} (det {:.2}){}",
        p(&o0), det(&o0), h.map_or(-1.0, |h| (p(&o0) - h.orig[0].w_axis.truncate()).length()), p(&o3), det(&o3),
        p(&d0), det(&d0), h.map_or(-1.0, |h| (p(&d0) - h.drawn[0].w_axis.truncate()).length()), p(&d3), det(&d3),
        if ours { " (KH1 did not repose him this frame)" } else { "" }
    )
}

/// Where the Keyblade was last posed, against the hands.
pub fn weapon_report() -> String {
    let small = SMALL.lock().ok().map(|g| {
        let mut v: Vec<String> = g
            .as_ref()
            .map(|m| {
                m.iter()
                    .filter(|(_, s)| s.at.elapsed().as_secs_f32() < 5.0)
                    .map(|(i, s)| format!("{i:#x} {}b {}ms ago at {:.0?} {:.0} from drawn hand{}", s.bones, s.at.elapsed().as_millis(), s.pos, s.dist_hand, s.from_bone.map_or(String::new(), |b| format!(", placed by Sora's bone {b}"))))
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v.join(" | ")
    });
    format!("{}; small models near Sora: [{}]", blade_report(), small.unwrap_or_default())
}


/// Small models seen near Sora: whether each is held, by which hand, with what relation.
#[derive(Default)]
struct Watch {
    rel: Option<(usize, Mat4)>,
    steady: u32,
    hand_moved: f32,
    last_hand: Option<Vec3>,
    held: Option<usize>,
    logged: bool,
}
static WATCH: Mutex<Option<HashMap<usize, Watch>>> = Mutex::new(None);

/// A game handle to a pointer.
pub fn ptr(h: u32) -> usize {
    if h == 0 {
        return 0;
    }
    let table = BASE.load(Ordering::Relaxed) + HANDLE_TABLE_RVA;
    let hi = unsafe { *((table + (((h & 0x7FFF_FFFF) >> 25) as usize) * 8) as *const u64) } as usize;
    hi | (h & 0x1FF_FFFF) as usize
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn mat(p: usize) -> Mat4 {
    Mat4::from_cols_array(&rd::<[f32; 16]>(p))
}

fn affine(m: &Mat4) -> bool {
    let a = m.to_cols_array();
    a.iter().all(|v| v.is_finite()) && a[3] == 0.0 && a[7] == 0.0 && a[11] == 0.0 && (a[15] - 1.0).abs() < 1e-4
}

/// Loads KH2 Sora from `KINGDOM HEARTS II FINAL MIX.exe`'s install (the originals the Mods
/// Manager backed up, if any, so a randomizer's edits don't leak in), and KH1 Sora's motion
/// table as the game loads it.
pub fn load_kh2() {
    std::thread::spawn(|| {
        let t = Instant::now();
        let r = (|| -> Result<Kh2, String> {
            let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
            let install = kh2_data::Install::locate(&exe).map_err(|e| format!("KH2 install: {e}"))?;
            let source = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
            let sora = install.load_sora(source).map_err(|e| format!("KH2 Sora: {e}"))?;
            let rig = sora.rig.clone().ok_or_else(|| sora.rig_error.clone().unwrap_or_else(|| "no rig".into()))?;
            let roots = sora.character.moveset.clips.iter().map(|(m, c)| (*m, c.root.clone())).collect();
            let mut ground = HashMap::new();
            for m in [character_core::motion::WALK, character_core::motion::RUN] {
                for field in [false, true] {
                    let clip = if field { rig.field_motions.get(&m) } else { rig.motions.get(&m) };
                    if let Some(c) = clip {
                        let g = ground_speed(&rig, c);
                        log(&format!("KH2 {} {}: ground speed {g:.0} units/s", if field { "field" } else { "battle" }, if m == character_core::motion::WALK { "walk" } else { "run" }));
                        ground.insert((m, field), g);
                    }
                }
            }
            log(&format!("KH2 Sora loaded ({source:?} files): {} bones, {} motions ({} with field variants), in {:.1}s", rig.skeleton.len(), rig.motions.len(), rig.field_motions.len(), t.elapsed().as_secs_f32()));
            let wisdom = if super::settings::get().drive {
                let t = Instant::now();
                match load_form(&install, source, kh2_data::sora::FORMS[1], rig.skeleton.len()) {
                    Ok(f) => {
                        log(&format!(
                            "KH2 Wisdom Form loaded: {} motions, {} combo rows, script {}, effects {}, in {:.1}s",
                            f.rig.motions.len(),
                            f.character.moveset.combo_rows.len(),
                            f.script.as_ref().map_or("missing".into(), |s| format!("{} bytes", s.len())),
                            if f.archive.is_some() { "read" } else { "missing" },
                            t.elapsed().as_secs_f32()
                        ));
                        Some(f)
                    }
                    Err(e) => {
                        log(&format!("KH2 Wisdom Form did not load: {e}"));
                        None
                    }
                }
            } else {
                None
            };
            let valor = if super::settings::get().drive {
                let t = Instant::now();
                match load_form(&install, source, kh2_data::sora::FORMS[0], rig.skeleton.len()) {
                    Ok(mut f) => {
                        port_moves(&mut f, &sora.character, &rig, &PORTED_TO_VALOR);
                        log(&format!("KH2 Valor Form loaded: {} motions, {} combo rows, {} weapons, in {:.1}s", f.rig.motions.len(), f.character.moveset.combo_rows.len(), f.rig.weapons.len(), t.elapsed().as_secs_f32()));
                        Some(f)
                    }
                    Err(e) => {
                        log(&format!("KH2 Valor Form did not load: {e}"));
                        None
                    }
                }
            } else {
                None
            };
            // Limit Form (the user: "lets begin working on limit form"): its own moveset (0x42),
            // motions, combo rows, movement numbers and effect archive, the same skeleton as
            // Sora's, his Keyblade (`W_EX010_KH1F`'s motions).
            let limit = if super::settings::get().drive {
                let t = Instant::now();
                match load_form(&install, source, kh2_data::sora::FORMS[2], rig.skeleton.len()) {
                    Ok(f) => {
                        log(&format!(
                            "KH2 Limit Form loaded: {} motions, {} combo rows, {} weapons, run {}, guard motion {}, dodge roll motion {}, script {}, effects {}, in {:.1}s",
                            f.rig.motions.len(),
                            f.character.moveset.combo_rows.len(),
                            f.rig.weapons.len(),
                            f.character.params.run_speed,
                            if f.character.moveset.clips.contains_key(&character_core::motion::GUARD) { "yes" } else { "none" },
                            if f.character.moveset.clips.contains_key(&character_core::motion::DODGE_ROLL) { "yes" } else { "none" },
                            f.script.as_ref().map_or("missing".into(), |s| format!("{} bytes", s.len())),
                            if f.archive.is_some() { "read" } else { "missing" },
                            t.elapsed().as_secs_f32()
                        ));
                        Some(f)
                    }
                    Err(e) => {
                        log(&format!("KH2 Limit Form did not load: {e}"));
                        None
                    }
                }
            } else {
                None
            };
            // Master Form (the user: "lets take the rules and procedures we've done for wisdom form
            // and valor form, and lets make master form"): its own moveset (4), motions, combo rows,
            // movement numbers, magic casts and effect archive, the same skeleton as Sora's, two
            // Keyblades (`W_EX010_TRIF_R` in his right hand, `_L` floating by his left).
            let master = if super::settings::get().drive {
                let t = Instant::now();
                match load_form(&install, source, kh2_data::sora::FORMS[3], rig.skeleton.len()) {
                    Ok(f) => {
                        log(&format!(
                            "KH2 Master Form loaded: {} motions, {} combo rows, {} weapons (on bones {:?}), run {}, aerial dodge motion {}, effects {}, in {:.1}s",
                            f.rig.motions.len(),
                            f.character.moveset.combo_rows.len(),
                            f.rig.weapons.len(),
                            f.rig.weapons.iter().map(|w| w.bone).collect::<Vec<_>>(),
                            f.character.params.run_speed,
                            if f.character.moveset.clips.contains_key(&character_core::motion::AERIAL_DODGE) { "yes" } else { "none" },
                            if f.archive.is_some() { "read" } else { "missing" },
                            t.elapsed().as_secs_f32()
                        ));
                        Some(f)
                    }
                    Err(e) => {
                        log(&format!("KH2 Master Form did not load: {e}"));
                        None
                    }
                }
            } else {
                None
            };
            let final_form = if super::settings::get().drive {
                let t = Instant::now();
                match load_form(&install, source, kh2_data::sora::FORMS[4], rig.skeleton.len()) {
                    Ok(f) => {
                        log(&format!(
                            "KH2 Final Form loaded: {} motions, {} combo rows, {} weapons (on bones {:?}), effects {}, in {:.1}s",
                            f.rig.motions.len(),
                            f.character.moveset.combo_rows.len(),
                            f.rig.weapons.len(),
                            f.rig.weapons.iter().map(|w| w.bone).collect::<Vec<_>>(),
                            if f.archive.is_some() { "read" } else { "missing" },
                            t.elapsed().as_secs_f32()
                        ));
                        Some(f)
                    }
                    Err(e) => {
                        log(&format!("KH2 Final Form did not load: {e}"));
                        None
                    }
                }
            } else {
                None
            };
            // Anti Form (the user: "I want you to add anti form. I want 1:1 combat parity"): its
            // own moveset, motions, combo rows, movement numbers and effect archive, the same
            // skeleton as Sora's (`load_form` checks it), no weapon.
            let anti = if super::settings::get().drive {
                let t = Instant::now();
                match load_form(&install, source, kh2_data::sora::FORMS[5], rig.skeleton.len()) {
                    Ok(f) => {
                        log(&format!(
                            "KH2 Anti Form loaded: {} motions, {} combo rows, {} weapons, run {}, guard motion {}, effects {}, in {:.1}s",
                            f.rig.motions.len(),
                            f.character.moveset.combo_rows.len(),
                            f.rig.weapons.len(),
                            f.character.params.run_speed,
                            if f.character.moveset.clips.contains_key(&character_core::motion::GUARD) { "yes" } else { "none" },
                            if f.archive.is_some() { "read" } else { "missing" },
                            t.elapsed().as_secs_f32()
                        ));
                        Some(f)
                    }
                    Err(e) => {
                        log(&format!("KH2 Anti Form did not load: {e}"));
                        None
                    }
                }
            } else {
                None
            };
            for (name, f) in [("Valor", &valor), ("Wisdom", &wisdom), ("Limit", &limit), ("Master", &master), ("Final", &final_form), ("Anti", &anti)] {
                if let Some(f) = f {
                    log_shape_groups(name, &f.rig, &f.character.moveset);
                }
            }
            Ok(Kh2 { rig, character: sora.character.clone(), battle: Arc::new(sora.combat.battle.clone()), magi: sora.magi.clone(), wisdom, valor, limit, master, final_form, anti, roots, ground })
        })();
        if let Err(e) = &r {
            log(&format!("KH2 Sora did not load: {e}"));
        }
        let _ = KH2.set(r);
        let slots = (|| -> Result<(HashMap<Signature, usize>, Vec<u32>), String> {
            let install = kh1_data::Kh1Install::locate(super::exe_dir()).map_err(|e| e.to_string())?;
            let bytes = install.read(kh1_data::Source::Live, "xa_ex_0010.mset").map_err(|e| e.to_string())?;
            let motions = kh1_data::mset::parse(&bytes).map_err(|e| e.to_string())?;
            let mut map = HashMap::new();
            let mut lengths = Vec::new();
            for (i, m) in motions.iter().enumerate() {
                lengths.push(m.as_ref().map_or(0, |m| m.frames));
                if let Some(m) = m {
                    map.entry((m.frames, m.channels.len() as u32, m.helper_channels, m.helpers)).or_insert(i);
                }
            }
            log(&format!("KH1 Sora's motion table: {} slots", motions.len()));
            let _ = REST.set(motions.get(REST_SLOT).cloned().flatten());
            let _ = KH1_MOTIONS.set(motions.clone());
            let _ = REST_OPEN.set(motions.first().cloned().flatten());
            Ok((map, lengths))
        })();
        match slots {
            Ok(m) => {
                let _ = SLOTS.set(m);
            }
            Err(e) => {
                let _ = REST.set(None);
                let _ = REST_OPEN.set(None);
                log(&format!("KH1 Sora's motion table did not load: {e}"))
            }
        }
    });
}

/// Every model size posed (the first 40 kinds), logged once each.
fn note_model(bones: u32, instance: usize, via_ik: bool) {
    static SEEN: Mutex<Vec<(u32, bool)>> = Mutex::new(Vec::new());
    let Ok(mut v) = SEEN.try_lock() else { return };
    if v.len() < 40 && !v.contains(&(bones, via_ik)) {
        v.push((bones, via_ik));
        log(&format!("posed: a {bones}-bone model (instance {instance:#x}) by {}", if via_ik { "the ground-IK pass" } else { "the plain pass" }));
    }
}

/// KH2 Sora's last pose (KH2 model space) and the model → world it was drawn with, for KH2's
/// effects (`kh2fx`), and when.
static KH2_POSE: Mutex<Option<(Vec<Mat4>, Mat4, Instant)>> = Mutex::new(None);

fn note_kh2(pose: &[Mat4], to_world: &Mat4) {
    if let Ok(mut g) = KH2_POSE.try_lock() {
        match g.as_mut() {
            Some(k) if k.0.len() == pose.len() => {
                k.0.copy_from_slice(pose);
                k.1 = *to_world;
                k.2 = Instant::now();
            }
            _ => *g = Some((pose.to_vec(), *to_world, Instant::now())),
        }
    }
}

/// KH2 Sora's pose (model space) and model → world, if one was drawn in the last half second.
pub fn kh2_pose<R>(f: impl FnOnce(&[Mat4], &Mat4) -> R) -> Option<R> {
    let g = KH2_POSE.lock().ok()?;
    let (pose, w, at) = g.as_ref()?;
    (at.elapsed().as_secs_f32() < 0.5).then(|| f(pose, w))
}

/// The blend argument of KH1's last ground-IK pass (diagnostics).
static LAST_T: AtomicU32 = AtomicU32::new(0);

/// KH1's wrist (the weapon hand relative to its forearm) for the weapon held KH1's way, while
/// KH1 posed him this frame (else the last one seen).
static KH1_WRIST: Mutex<Option<Mat4>> = Mutex::new(None);

/// Weapons KH2's sword grip holds the wrong way round (the ini's `flip_weapons`, by Sora's
/// weapon id, save `+0x36`; the user: the Dream Shield's front faced Sora) are held with
/// KH1's own wrist: the weapon hand (the grip's parent) takes KH1's turn relative to the
/// forearm, carrying the fingers and the weapon with it; KH2's arm swings as before. Only KH1's
/// turn of the wrist is taken (the hand stays our bone's length from the forearm), and only from
/// frames where KH1's hand is a rigid bone's length from its forearm.
unsafe fn kh1_wrist(pose: &mut [Mat4], parents: &[i16], orig_world: Option<usize>) {
    let flip = &super::settings::get().flip_weapons;
    if flip.is_empty() {
        return;
    }
    let save = rd::<u64>(BASE.load(Ordering::Relaxed) + Rva(0x28695A0)) as usize;
    if save < 0x10000 || !flip.contains(&rd::<u8>(save + 0x36)) {
        return;
    }
    let grip = kh1_data::retarget::GRIP_BONE;
    let parent = |i: usize| parents.get(i).copied().filter(|p| *p >= 0).map(|p| p as usize);
    let Some(hand) = parent(grip) else { return };
    let Some(arm) = parent(hand) else { return };
    let n = pose.len().min(SORA_BONES as usize);
    if hand >= n || arm >= n {
        return;
    }
    // Our own forearm-to-hand (KH1's bone length, as the retarget keeps it).
    let ours = pose[arm].inverse() * pose[hand];
    let bone = ours.w_axis.truncate();
    if let Some(w) = orig_world {
        let l = mat(w + arm * 0x40).inverse() * mat(w + hand * 0x40);
        // (Only a wrist that is one: on the frames KH1 blends between its motions its hand
        // matrix can lie thousands of units off (the s328 log with the Dream Shield: KH1's hand
        // and grip at twice his position, its 283 squashed, det 0..-0.8), and that "wrist" put
        // the hand drawn there, the arm's and the shield's mesh stretched to it across the
        // field, the user: "having the dream shield equipped leads to some pretty bad
        // stretching issues". A frame whose hand isn't a bone's length from the forearm, or
        // isn't rigid, keeps the last good wrist.)
        let (sc, _, t) = l.to_scale_rotation_translation();
        let rigid = l.is_finite() && (sc - Vec3::ONE).abs().max_element() < 0.1 && glam::Mat3::from_mat4(l).determinant() > 0.0;
        let len_ok = (t.length() - bone.length()).abs() <= (bone.length() * 0.25).max(2.0);
        if rigid && len_ok {
            if let Ok(mut g) = KH1_WRIST.lock() {
                *g = Some(l);
            }
            static LOGGED: AtomicU32 = AtomicU32::new(u32::MAX);
            let w = held_weapon().unwrap_or(0) as u32;
            if LOGGED.swap(w, Ordering::Relaxed) != w {
                let (_, q, _) = (ours.inverse() * l).to_scale_rotation_translation();
                log(&format!("weapon {w:#x} (model {}): held by KH1's wrist (`flip_weapons`), turned {:.2} rad from KH2's hand, KH1's forearm-to-hand {:.1} (ours {:.1})", w.wrapping_sub(0x50), q.angle_between(Quat::IDENTITY), t.length(), bone.length()));
            }
        } else {
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 10 {
                log(&format!("KH1's wrist this frame not used (its hand {:.0} from the forearm, ours {:.1}; scale {sc:.2?}): the last good one kept", t.length(), bone.length()));
            }
        }
    }
    let Some(local) = KH1_WRIST.lock().ok().and_then(|g| *g) else { return };
    // KH1's turn of the wrist, our bone's length and offset (never KH1's translation).
    let (_, q, _) = local.to_scale_rotation_translation();
    let local = Mat4::from_rotation_translation(q, bone);
    let want = pose[arm] * local;
    let delta = want * pose[hand].inverse();
    if !delta.is_finite() {
        return;
    }
    // The hand and every bone under it.
    for i in hand..n {
        let mut p = Some(i);
        for _ in 0..24 {
            match p {
                Some(q) if q == hand => {
                    pose[i] = delta * pose[i];
                    break;
                }
                Some(q) if q < hand => break,
                Some(q) => p = parent(q),
                None => break,
            }
        }
    }
}

/// The dream weapons and the wooden sword (KH1 items `0x52..=0x55`): not Keyblades, so held as
/// KH1 holds them.
fn dream_weapon(w: u8) -> bool {
    (0x52..=0x55).contains(&w)
}

/// KH1's hold of a dream weapon, by weapon: the grip relative to the hand, then each weapon bone
/// after the grip relative to the grip, from KH1's own pose of a frame (`dream_hold`).
static DREAM_HOLD: Mutex<Option<(u8, Vec<Mat4>)>> = Mutex::new(None);
const WEAPON_BONES: usize = 13;
/// How far from the hand / grip a dream weapon's bones can be in a hold that is noted.
const DREAM_NEAR: f32 = 250.0;
/// The weapon bones (after the grip) the weapon's mesh is skinned to: 282..291, the ones Sora's
/// bind flags for KH1's model queue (`docs/kh1-weapon-second.md` §1.4); 281 and 292 aren't.
const SKINNED: std::ops::RangeInclusive<usize> = 2..=11;

/// A dream weapon held rigidly as KH1 holds it (the tester: the Dream Sword "stretches" the
/// split second a movement begins, the rest of him not, and the Dream Shield is held another
/// way in battle than in the cutscenes): its bones were posed one by one (blends, the keychain
/// swing), so the mesh between them stretched. `kh1_world`: KH1's world matrices when it posed
/// him this frame (its hold is noted from them); then every weapon bone is put where KH1's hold
/// puts it from the hand drawn.
unsafe fn dream_hold(pose: &mut [Mat4], parents: &[i16], bind: &[Mat4], kh1_world: Option<usize>) {
    let Some(w) = held_weapon().filter(|w| dream_weapon(*w)) else { return };
    let grip = kh1_data::retarget::GRIP_BONE;
    let Some(hand) = parents.get(grip).copied().filter(|p| *p >= 0).map(|p| p as usize) else { return };
    if pose.len() < grip + WEAPON_BONES || hand >= pose.len() || bind.len() < grip + WEAPON_BONES {
        return;
    }
    // Each weapon bone relative to the grip as in Sora's bind (rigid, unswung).
    let bind_rel = |i: usize| bind[grip].inverse() * bind[grip + i];
    if let Some(world) = kh1_world {
        let g = mat(world + grip * 0x40);
        let mut rel = vec![mat(world + hand * 0x40).inverse() * g];
        let gi = g.inverse();
        rel.extend((1..WEAPON_BONES).map(|i| gi * mat(world + (grip + i) * 0x40)));
        // (Only a hold whose bones lie where a weapon can: on the frame an animation starts some
        // weapon bones are still the last drawn ones, far from the hand, and a hold noted then
        // put the shield there with the mesh stretched to it, the user: "tap move, the split
        // second any animations begin, he stretches ... you can see his un-stretched shield
        // appears at the end of the stretched mesh".)
        // Only the bones the weapon's mesh is skinned to count (`SKINNED`, 282..291). The s328 log:
        // the Dream Shield's hold was refused through the Dive's first three rooms for "a bone 526
        // / 17605 from the grip", about the grip's own distance from the world's origin, so most
        // likely an unskinned bone (281 or 292) KH1 leaves at the origin; no hold was noted until
        // room 4 and the shield was drawn on the Keyblade's swung chain meanwhile. Those two take
        // the bind's place (nothing is drawn with them); the log now names the bone refused.
        let bad = |i: usize, m: &Mat4| -> bool {
            let far = m.w_axis.truncate().length() >= DREAM_NEAR;
            !m.is_finite() || ((i == 0 || SKINNED.contains(&i)) && far)
        };
        match rel.iter().enumerate().find(|(i, m)| bad(*i, m)) {
            Some((i, m)) => {
                static N: AtomicU32 = AtomicU32::new(0);
                if N.fetch_add(1, Ordering::Relaxed) < 20 {
                    log(&format!(
                        "weapon {w:#x}: KH1's hold this frame not noted (bone {} {:.0} from the {}, det {:.2})",
                        grip + i,
                        m.w_axis.truncate().length(),
                        if i == 0 { "hand" } else { "grip" },
                        glam::Mat3::from_mat4(*m).determinant()
                    ));
                }
            }
            None => {
                for i in 1..WEAPON_BONES {
                    if !SKINNED.contains(&i) {
                        rel[i] = bind_rel(i);
                    }
                }
                if let Ok(mut h) = DREAM_HOLD.lock() {
                    let new = h.as_ref().map_or(true, |(id, _)| *id != w);
                    if new {
                        let reach = rel[1..].iter().map(|m| m.w_axis.truncate().length()).fold(0.0f32, f32::max);
                        let (sc, q, t) = rel[0].to_scale_rotation_translation();
                        log(&format!("weapon {w:#x} (model {}): held rigidly as KH1 holds it (its hold noted from KH1's pose: grip {t:.1?} from the hand, turned {:.2} rad, scale {sc:.2?}; its bones reach {reach:.0} from the grip)", w - 0x50, q.angle_between(Quat::IDENTITY)));
                    }
                    *h = Some((w, rel));
                }
            }
        }
    }
    let Some(rel) = DREAM_HOLD.lock().ok().and_then(|h| h.as_ref().filter(|(id, _)| *id == w).map(|(_, r)| r.clone())) else {
        // No hold of KH1's for this weapon yet: the grip as posed, every bone after it rigid as
        // in the bind (the Keyblade's chain swing would bend the mesh).
        static LOGGED: AtomicU32 = AtomicU32::new(u32::MAX);
        if LOGGED.swap(w as u32, Ordering::Relaxed) != w as u32 {
            log(&format!("weapon {w:#x} (model {}): no hold of KH1's noted yet: held rigid as in the bind until one is", w - 0x50));
        }
        let g = pose[grip];
        for i in 1..WEAPON_BONES {
            let m = g * bind_rel(i);
            if m.is_finite() {
                pose[grip + i] = m;
            }
        }
        return;
    };
    let g = pose[hand] * rel[0];
    pose[grip] = g;
    for i in 1..WEAPON_BONES {
        pose[grip + i] = g * rel[i];
    }
}

/// KH1's own hold of dream weapon `w`, if one was noted this session (`dream_hold`: the grip
/// relative to the hand, then each weapon bone after the grip relative to the grip), for the
/// same weapon in Valor's left hand (`leftblade`, the Dream Sword).
pub fn dream_hold_of(w: u8) -> Option<Vec<Mat4>> {
    DREAM_HOLD.try_lock().ok().and_then(|h| h.as_ref().filter(|(id, _)| *id == w).map(|(_, r)| r.clone()))
}

/// The same on world matrices (KH1's own animation blending back in).
unsafe fn dream_hold_world(world: usize, parents: &[i16]) {
    let Some(w) = held_weapon().filter(|w| dream_weapon(*w)) else { return };
    let grip = kh1_data::retarget::GRIP_BONE;
    let Some(hand) = parents.get(grip).copied().filter(|p| *p >= 0).map(|p| p as usize) else { return };
    let Some(rel) = DREAM_HOLD.lock().ok().and_then(|h| h.as_ref().filter(|(id, _)| *id == w).map(|(_, r)| r.clone())) else { return };
    let g = mat(world + hand * 0x40) * rel[0];
    for i in 0..WEAPON_BONES {
        let m = if i == 0 { g } else { g * rel[i] };
        if m.is_finite() {
            std::ptr::write_unaligned((world + (grip + i) * 0x40) as *mut [f32; 16], m.to_cols_array());
        }
    }
}

/// Sora's weapon (KH1 item id, save `+0x36`), if the save reads.
unsafe fn held_weapon() -> Option<u8> {
    let save = rd::<u64>(BASE.load(Ordering::Relaxed) + Rva(0x28695A0)) as usize;
    (save >= 0x10000).then(|| rd::<u8>(save + 0x36))
}

/// Every Keyblade turned half round about its own length in his hand (the user: "every keyblade
/// is held backwards", the teeth to the ground): the grip (`GRIP_BONE`) hangs from the hand as
/// in KH1's rest pose, whose roll of the blade is not KH2's. The blade and its keychain lie along
/// the grip's x (the keychain's bones at -21..-56 on it in the bind), so the turn is about that
/// axis, the subtree with it. Not the dream weapons and the wooden sword (`0x52..=0x55`), nor
/// those `flip_weapons` holds by KH1's wrist (the Dream Shield).
unsafe fn keyblade_teeth_up(pose: &mut [Mat4], parents: &[i16]) {
    let save = rd::<u64>(BASE.load(Ordering::Relaxed) + Rva(0x28695A0)) as usize;
    if save < 0x10000 {
        return;
    }
    let w = rd::<u8>(save + 0x36);
    let set = super::settings::get();
    if !set.keyblade_turn || (0x52..=0x55).contains(&w) || set.flip_weapons.contains(&w) {
        return;
    }
    let grip = kh1_data::retarget::GRIP_BONE;
    if grip >= pose.len() {
        return;
    }
    let turn = pose[grip] * Mat4::from_rotation_x(std::f32::consts::PI) * pose[grip].inverse();
    if !turn.is_finite() {
        return;
    }
    for i in grip..pose.len().min(SORA_BONES as usize) {
        let mut p = Some(i);
        for _ in 0..24 {
            match p {
                Some(q) if q == grip => {
                    pose[i] = turn * pose[i];
                    break;
                }
                Some(q) if q < grip => break,
                Some(q) => p = parents.get(q).copied().filter(|x| *x >= 0).map(|x| x as usize),
                None => break,
            }
        }
    }
}

/// The model-space pose to draw for the current mode, if any.
fn wanted(s: &mut Sora, slot: Option<usize>, to_world: &Mat4) -> Option<Vec<Mat4>> {
    let mode = MODE.load(Ordering::Relaxed);
    s.kh2_now = None;
    s.src_now = src::FOLLOW;
    match mode {
        0 => {
            // KH1's area script walks him (zone entries and exits): KH1's own animation.
            if super::takeover::SCRIPTED.load(Ordering::Relaxed) {
                s.follow.slot = slot;
                return None;
            }
            // Jump at KH1: KH1's own jump, fall and landing over KH2's (the user: "turning jump to
            // kh1 doesn't change soras jump animation back to kh1"; asked: KH1 animations only,
            // KH2's physics).
            if super::takeover::kh2_frame() {
                if let Some(p) = kh1_air_pose() {
                    s.follow.slot = slot;
                    s.src_now = src::KH1_AIR;
                    NATIVE_KH1.store(true, Ordering::Relaxed);
                    return Some(p);
                }
            }
            // Misc Animations at KH1: KH1's own idle / walk / run while KH2 stands, walks or runs
            // him (`takeover::MISC_KH1`), blended in from our last pose.
            if super::takeover::MISC_KH1.load(Ordering::Relaxed) && super::takeover::kh2_frame() && matches!(slot, Some(0..=7)) {
                s.follow.slot = slot;
                return None;
            }
            // KH2's last pose: recent, or the last frame of his was KH2's (a pause stops the
            // game's frames, not the clock: after one, the old pose went stale and KH1's slot was
            // drawn until KH2's next frame; the user: paused mid KH2 move, resumed in KH1's).
            let kh2_frame = super::takeover::kh2_frame();
            // (KH1's own combo, `stylecombo`: KH1's swings from their first frame, not KH2's
            // last pose over them.)
            // (Nor over KH1's own dodge roll, `styleroll`: its roll from its first frame.)
            // (Nor over an item's use drawn with KH2's motion, Misc Animations.)
            let kh1_combo = super::stylecombo::active() || super::styleroll::active() || (misc_kh2() && super::takeover::item_use_now());
            let fresh = if kh1_combo { None } else { super::takeover::POSE.try_lock().ok().and_then(|p| p.as_ref().filter(|(_, _, t)| kh2_frame || t.elapsed().as_secs_f32() < 0.15).map(|(b, w, _)| (b.clone(), w.clone()))) };
            match (fresh, s.retarget.as_ref()) {
                (Some((b, w)), Some(r)) => {
                    s.follow.slot = slot;
                    note_kh2(&b, to_world);
                    let pose = r.pose_grip(&b, super::takeover::GRIP.load(Ordering::Relaxed));
                    s.kh2_now = Some((b, w));
                    s.src_now = src::KH2;
                    Some(pose)
                }
                // KH1's own jump, fall and landing while KH1 has him (dropping off a ladder or a
                // vine, out of water): KH1's own animation (the user: KH2's fall, its arms
                // flung out, looked like a T-pose there).
                _ if matches!(slot, Some(8..=13)) => {
                    s.follow.slot = slot;
                    None
                }
                _ => follow(s, slot, to_world),
            }
        }
        1 => follow(s, slot, to_world),
        3 => Some(s.bind.clone()),
        4 | 5 => {
            let kh2 = KH2.get()?.as_ref().ok()?;
            let r = s.retarget.as_ref()?;
            let m = kh2.rig.motions.get(&(if mode == 4 { character_core::motion::IDLE } else { character_core::motion::RUN }))?;
            let frames = CLOCK.get_or_init(Instant::now).elapsed().as_secs_f32() * 60.0;
            let t = kh2_anim::key_time(m, frames, true);
            let pose = kh2_anim::evaluate(&kh2.rig.skeleton, m, t, kh2_anim::Options::default()).bones;
            let weapon = kh2_weapon(&kh2.rig, if mode == 4 { character_core::motion::IDLE } else { character_core::motion::RUN }, frames, true, &pose);
            let kh1 = r.pose(&pose);
            s.kh2_now = Some((pose, weapon));
            Some(kh1)
        }
        _ => None,
    }
}

/// KH2's Keyblade (the rig's first weapon) in Sora's model space for a body pose: its own motion
/// of the same id at the same time, on its hand bone, as `kh2_rig::RigAnim` places it.
fn kh2_weapon(rig: &Rig, motion: u16, frames: f32, looping: bool, body: &[Mat4]) -> Vec<Mat4> {
    let Some(w) = rig.weapons.first() else { return Vec::new() };
    let Some(hand) = w.bone.and_then(|b| body.get(b)).copied() else { return Vec::new() };
    let local = match w.motions.get(&motion).or_else(|| w.motions.get(&0)) {
        Some(m) => kh2_anim::evaluate(&w.skeleton, m, kh2_anim::key_time(m, frames, looping), kh2_anim::Options::default()).bones,
        None => kh2_anim::Pose::bind(&w.skeleton).bones,
    };
    local.iter().map(|l| hand * *l).collect()
}

fn decompose(v: &[Mat4]) -> Vec<(Vec3, Quat, Vec3)> {
    v.iter().map(|m| m.to_scale_rotation_translation()).collect()
}

/// KH2's motion for what KH1's slot shows Sora doing; `None` leaves KH1's own animation.
fn follow(s: &mut Sora, slot: Option<usize>, to_world: &Mat4) -> Option<Vec<Mat4>> {
    // One of KH1's limits in Limit Form (`limits::kh1_limit_holds`): KH1's own animation all
    // through it as KH1 plays it (Limit Form's model is KH1 Sora's), none of the form's KH2
    // motions drawn over a slot of it (a walk, a run, a jump).
    if super::limits::kh1_limit_holds() {
        s.follow.slot = slot;
        return None;
    }
    // Misc Animations at KH2 (`styles`, base Sora): KH1 uses the item (its effect, its timing, the
    // item's logic) and KH2's item-use motion is drawn over it (P_EX100 motion 9, A110: the
    // right hand raised overhead at frames ~20-30 of its 50, a voice at 0 and a sound at 10; KH1's
    // own use runs ~52 frames, `takeover::ITEM_CUT`), the user: "a setting for misc animations to
    // be KH1 or KH2? Like item throwing animation". (Group 9: a change of KH1's slot meanwhile
    // doesn't restart it.)
    if misc_kh2() && super::takeover::item_use_now() {
        return follow_play(s, slot, to_world, Some(Play::Motion { motion: KH2_ITEM_USE, field: false, looping: false, group: 9 }), false);
    }
    // Anti Form's own ledge grab (the user: "antiform has it's own unique ledge grab animation
    // please bring that in"): KH1 grabs the ledge and pulls him up as ever (its states 15, then 19
    // and 18: the log), drawn with Anti's hang (motion 7, looping) and its climb (8).
    if super::form::in_anti() {
        if let Some(play) = anti_ledge() {
            return follow_play(s, slot, to_world, Some(play), false);
        }
    }
    // KH1's own combo (`stylecombo`, the move style set to KH1): KH1's swings, not KH2's drawn
    // over them.
    if super::stylecombo::active() && matches!(slot, Some(98..=104)) {
        s.follow.slot = slot;
        return None;
    }
    // In Wisdom Form, KH1's spells with no KH2 counterpart (Gravity, Stop, Aero: KH1 casts them)
    // show the form's own cast (the user: "any kh1 magic that doesn't have a kh2 equivalent
    // (gravity, stop) should use the normal thunder/cure wisdom form animation but still cast
    // their respected magic"): Gravity Thunder's, Stop and Aero Cure's; the air ones in the air.
    // (And base Sora's KH1 Gravity, Stop and Aero with the Magic style at KH2: his own KH2 cast
    // motions, `formmagic::BASE_KH1_CAST`.)
    // (Final Form as Wisdom, with Final's own cast motions under the same ids: `form::casting_form`,
    // the rig below picking the form's clip.)
    let base_look = super::formmagic::BASE_KH1_CAST.load(std::sync::atomic::Ordering::Relaxed) && !super::form::in_form();
    if super::form::casting_form().is_some() || base_look {
        if let Some(spell @ 3..=6) = super::takeover::kh1_spell_now() {
            let air = super::takeover::air_cast().is_some();
            let motion = match (spell, air) {
                (4, false) => 62,
                (4, true) => 64,
                (_, false) => 65,
                (_, true) => 67,
            };
            // (A cast motion that travels, Master's, moves him through KH1's mover,
            // `takeover::form_cast_frame`: its root travel comes out of the pose.)
            return follow_play(s, slot, to_world, Some(Play::Motion { motion, field: false, looping: false, group: 0 }), true);
        }
    }
    follow_play(s, slot, to_world, None, false)
}

/// Anti's gripping ankle sits this far under the ledge (KH1's highest bone in its hang), so the
/// foot hooks over the edge.
const ANKLE_UNDER_EDGE: f32 = 6.0;

/// This frame's pose is Anti's ledge hang or climb (`anti_ledge`): `check` puts its hands on KH1's.
static ANTI_LEDGE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// KH1's ledge grab while Anti Form has him: its hang (KH1 state 15, as `takeover::grab`'s "bar"
/// finds it; 13 too) as Anti's hang, its pull-up (19, then 18 on top of the ledge for its last
/// motions) as Anti's climb. `None`: not on a ledge, or Anti has no such motion.
fn anti_ledge() -> Option<Play> {
    use character_core::motion as m2;
    static CLIMB_AT: Mutex<Option<Instant>> = Mutex::new(None);
    let actor = super::takeover::sora_actor();
    if actor < 0x10000 {
        return None;
    }
    let state = unsafe { rd::<i32>(actor + 0x70) };
    let mut climb = CLIMB_AT.lock().ok()?;
    let motion = match state {
        13 | 15 => {
            *climb = None;
            m2::HANG
        }
        19 => {
            climb.get_or_insert_with(Instant::now);
            m2::CLIMB
        }
        // (On top of the ledge, finishing the pull-up: the climb plays on, a second at most.)
        18 if climb.is_some_and(|t| t.elapsed().as_secs_f32() < 1.0) => m2::CLIMB,
        _ => {
            *climb = None;
            return None;
        }
    };
    let has = KH2.get()?.as_ref().ok()?.anti.as_ref().is_some_and(|f| f.rig.motions.contains_key(&motion));
    ANTI_LEDGE.store(has, Ordering::Relaxed);
    has.then_some(Play::Motion { motion, field: false, looping: motion == m2::HANG, group: if motion == m2::HANG { 7 } else { 8 } })
}

/// KH2 Sora's item-use motion (P_EX100 motion 9, `A110`).
const KH2_ITEM_USE: u16 = 9;

/// KH2's misc animations for base Sora (the Misc Animations row at KH2, not in a form).
fn misc_kh2() -> bool {
    super::styles::kh2(super::styles::Kind::MiscAnims) && !super::form::in_form()
}

/// `follow` with the motion chosen by the caller (`forced`) instead of from KH1's slot; `cast`:
/// a form's KH1 cast on its cast motion, whose root travel KH1's mover gives him (`unroot`).
fn follow_play(s: &mut Sora, slot: Option<usize>, to_world: &Mat4, forced: Option<Play>, cast: bool) -> Option<Vec<Mat4>> {
    use character_core::motion as m2;
    let now = Instant::now();
    let f = &mut s.follow;
    // Movement of the model origin: horizontal speed and climb (along the model's up).
    let pos = to_world.w_axis.truncate();
    let up = to_world.y_axis.truncate().normalize_or_zero();
    if let Some((p, t)) = f.at {
        let dt = now.duration_since(t).as_secs_f32();
        if dt > 4e-3 && dt < 0.25 {
            let d = pos - p;
            let climb = d.dot(up) / dt;
            let speed = (d - up * d.dot(up)).length() / dt;
            f.speed += (speed - f.speed) * 0.2;
            f.climb += (climb - f.climb) * 0.3;
            f.at = Some((pos, now));
        } else if dt >= 0.25 {
            f.at = Some((pos, now));
        }
    } else {
        f.at = Some((pos, now));
    }
    let play = forced.or_else(|| slot.and_then(play_for));
    let new_slot = slot != f.slot;
    if new_slot {
        if f.logged < 800 {
            f.logged += 1;
            log(&format!(
                "KH1 slot {} ({}) at speed {:.0}/s, climb {:.0}/s",
                slot.map_or("?".into(), |n| n.to_string()),
                play.map_or("KH1's own animation".into(), |p| format!("{p:?}")),
                f.speed,
                f.climb
            ));
        }
        f.slot = slot;
    }
    let Some(play) = play else {
        f.play = None;
        return None;
    };
    let kh2 = KH2.get()?.as_ref().ok()?;
    let r = s.retarget.as_ref()?;
    let f = &mut s.follow;
    let (motion, field, looping, restart, rate, blend) = match play {
        Play::Motion { motion, field, looping, group } => {
            let same_group = matches!(f.play, Some(Play::Motion { group: g, .. }) if g == group && group != 0);
            let restart = f.motion != motion || f.field != field || (new_slot && !same_group && !looping);
            let blend = if motion == m2::GLIDE { BLEND_GLIDE } else { BLEND_MOVE };
            // Walk and run step with his real speed, so the planted foot doesn't slide.
            let rate = match kh2.ground.get(&(motion, field)) {
                Some(&g) if g > 1.0 && (motion == m2::WALK || motion == m2::RUN) => (60.0 * f.speed / (g * r.hip_ratio())).clamp(25.0, 150.0),
                _ => 60.0,
            };
            (motion, field, looping, restart, rate, blend)
        }
        Play::Swing { air, finisher } => {
            if new_slot {
                let fresh = f.last_swing.map_or(true, |t| now.duration_since(t).as_secs_f32() > COMBO_GAP);
                f.swings = if fresh { 0 } else { f.swings + 1 };
            }
            f.last_swing = Some(now);
            let motion = match (air, finisher, f.swings % 2 == 0) {
                (false, true, _) => 154,
                (false, false, true) => 151,
                (false, false, false) => 152,
                (true, true, _) => 181,
                (true, false, true) => 183,
                (true, false, false) => 184,
            };
            // Stretch KH2's swing over KH1's, so the blade comes round when KH1's hit lands.
            let kh1_len = slot.and_then(|n| SLOTS.get()?.1.get(n).copied()).unwrap_or(0) as f32;
            let kh2_len = kh2.rig.motions.get(&motion).map_or(0.0, |m| kh2_anim::length_frames(m));
            let rate = if kh1_len > 1.0 && kh2_len > 1.0 { 60.0 * kh2_len / kh1_len } else { 60.0 };
            (motion, false, false, new_slot, rate, 0.0)
        }
    };
    if !restart && matches!(play, Play::Motion { .. }) {
        // Loops change speed as he does: carry the clip position over at the new rate.
        if let Some(t0) = f.since {
            let done = now.duration_since(t0).as_secs_f32() * f.rate;
            f.rate = f.rate + (rate - f.rate) * 0.15;
            let back = std::time::Duration::from_secs_f32((done / f.rate.max(1.0)).clamp(0.0, 3600.0));
            f.since = now.checked_sub(back).or(f.since);
        }
    }
    if restart {
        // (Out of one of KH1's grabs, only from a pose this drew just now: frames of KH1's own
        // animation since left `last` stale, and blending from it brought back what he did
        // before, the hang before the climb. That change of source blends in `check`.)
        let recent = f.last_at.is_some_and(|t| now.duration_since(t).as_secs_f32() < 0.1) || !super::takeover::grab_blend();
        if !f.last.is_empty() && blend > 0.0 && recent {
            f.from = decompose(&f.last);
            f.blend_start = Some(now);
            f.blend_secs = blend;
        } else {
            f.blend_start = None;
        }
        f.motion = motion;
        f.field = field;
        f.since = Some(now);
        f.rate = rate;
    }
    f.play = Some(play);
    // (In a Drive Form, the form's motions: KH1's own moves and spells then look like the form's.)
    let form = super::form::current().and_then(|k| k.data(kh2)).filter(|w| w.rig.motions.contains_key(&motion));
    let rig: &kh2_data::sora::Rig = match form {
        Some(w) => &w.rig,
        _ => &kh2.rig,
    };
    // (The drawn clip's root curve, the form's or base Sora's, for a cast's `unroot`.)
    let cast_root = cast.then(|| form.map_or(&kh2.character, |w| &w.character).moveset.clips.get(&motion)).flatten().filter(|c| super::takeover::travels(c)).map(|c| &c.root);
    let clip = field
        .then(|| rig.field_motions.get(&motion))
        .flatten()
        .or_else(|| rig.motions.get(&motion))
        .or_else(|| rig.motions.get(&m2::IDLE))?;
    let frames = f.since.map_or(0.0, |t| now.duration_since(t).as_secs_f32() * f.rate);
    let t = kh2_anim::key_time(clip, frames, looping);
    let mut pose = kh2_anim::evaluate(&rig.skeleton, clip, t, kh2_anim::Options::default()).bones;
    // The Keyblade's bones after Sora's (the lunge and the blend apply to both; split off below).
    let body_len = pose.len();
    let weapon = kh2_weapon(rig, motion, frames, looping, &pose);
    pose.extend(weapon);
    // A swing's lunge: KH1 moves him, so take KH2's root travel out of the pose. (So too a form's
    // KH1 cast on a cast motion that travels, Master's: `takeover::form_cast_frame` moves him.)
    let roots = if matches!(play, Play::Swing { .. }) { kh2.roots.get(&motion) } else { cast_root };
    if roots.is_some() {
        if let Some(root) = roots.and_then(|r| r.get((frames as usize).min(r.len().saturating_sub(1)))) {
            let un = Mat4::from_translation(-Vec3::from(*root));
            for m in pose.iter_mut() {
                *m = un * *m;
            }
        }
    }
    if let Some(b) = f.blend_start {
        let w = now.duration_since(b).as_secs_f32() / f.blend_secs.max(1e-3);
        if w >= 1.0 || f.from.len() != pose.len() {
            f.blend_start = None;
        } else {
            for (m, (fs, fr, ft)) in pose.iter_mut().zip(&f.from) {
                let (ss, sr, st) = m.to_scale_rotation_translation();
                *m = Mat4::from_scale_rotation_translation(fs.lerp(ss, w), fr.slerp(sr, w), ft.lerp(st, w));
            }
        }
    }
    f.last = pose.clone();
    f.last_at = Some(now);
    let weapon = pose.split_off(body_len.min(pose.len()));
    note_kh2(&pose, to_world);
    let kh1 = r.pose(&pose);
    s.kh2_now = Some((pose, weapon));
    Some(kh1)
}

/// The slot of the KH1 motion the instance plays.
unsafe fn slot_of(instance: usize) -> Option<usize> {
    let m = rd::<u64>(instance + 8) as usize;
    if m < 0x10000 {
        return None;
    }
    let sig = (rd::<u32>(m + 4), rd::<u32>(m + 0x18), rd::<u32>(m + 0x20), rd::<u32>(m + 0x2C));
    SLOTS.get()?.0.get(&sig).copied()
}

/// The model's bone count (`**instance`), if it reads.
unsafe fn bone_count(instance: usize) -> Option<u32> {
    let slot = rd::<u64>(instance) as usize;
    if slot < 0x10000 {
        return None;
    }
    let header = rd::<u64>(slot) as usize;
    if header < 0x10000 {
        return None;
    }
    Some(rd::<u16>(header) as u32)
}

/// When a copy of Sora's model away from his actor (the pause menu's) was last posed.
static MENU_SORA: Mutex<Option<Instant>> = Mutex::new(None);

/// The pause menu's Sora was posed within the last `ms` (the menu is up).
pub fn menu_sora_within(ms: u64) -> bool {
    MENU_SORA.lock().ok().and_then(|m| *m).is_some_and(|t| t.elapsed().as_millis() as u64 <= ms)
}

/// Whether a Sora model instance stands at his actor (the field's Sora), as far as can be told
/// (unknown: yes). The pass's world matrix, else bone 0's, against the actor's `+0x10`.
unsafe fn at_sora(instance: usize, param_world: usize) -> bool {
    // Sora's actor from KH1's party slot, current every frame (the probe's copy could still be the
    // last room's actor as a room loads, and Sora's new model was refused, left in KH1's
    // animations; the user: "sometimes entering a room").
    let actor = super::takeover::sora_actor();
    if actor < 0x10000 || !super::crash::readable(actor + 0x20) {
        return true;
    }
    let at = from_param(param_world).map(|m| m.w_axis.truncate()).or_else(|| {
        let world = ptr(rd::<u32>(instance + WORLD));
        (world != 0).then(|| Mat4::from_cols_array(&rd::<[f32; 16]>(world)).w_axis.truncate())
    });
    let Some(p) = at.filter(|p| p.is_finite()) else { return true };
    let a = rd::<[f32; 3]>(actor + 0x10);
    let d = Vec3::new(p.x - a[0], p.y - a[1], p.z - a[2]).length();
    static N: AtomicU32 = AtomicU32::new(0);
    if d > 150.0 && N.fetch_add(1, Ordering::Relaxed) < 10 {
        log(&format!("Sora's model {instance:#x} drawn {d:.0} away from his actor: another copy (the pause menu's), left to KH1"));
    }
    d <= 150.0
}

unsafe fn check(instance: usize, via_ik: bool, param_world: usize) {
    let Some(bones) = bone_count(instance) else { return };
    note_model(bones, instance, via_ik);
    if bones != SORA_BONES {
        return;
    }
    let header = rd::<u64>(rd::<u64>(instance) as usize) as usize;
    let Ok(mut g) = SORA.try_lock() else { return };
    // Another Sora model while KH1's pause machine is up is the pause menu's copy, wherever it
    // stands. The menu draws it about the world's origin, so with Sora himself near there it
    // passed `at_sora`, was taken for him and given KH2's last pose, frozen with the game paused
    // (the user: "pausing in final form plays sora's Final form idle ... right now its freeze
    // framed"; the log: the menu's model taken as "Sora's instance", his own taken back on
    // resume). Left to KH1, as asked: "make sora do his normal KH1 walk in the menu like
    // vanilla". (His own model, kept, stays ours: nothing of his is reset by the pause.)
    let other = g.as_ref().map_or(true, |s| s.instance != instance);
    let in_menu = other && g.is_some() && super::kh2fx::pause_menu_up(BASE.load(Ordering::Relaxed));
    if in_menu {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 3 {
            log(&format!("Sora's model {instance:#x} drawn while KH1's pause machine is up: the pause menu's copy, left to KH1"));
        }
    }
    if in_menu || (other && !at_sora(instance, param_world)) {
        // Another copy of Sora's model away from his actor: the pause menu's (the user: it
        // walked slowly in KH2's walk and showed a frame of KH2's pose). KH1's own animation.
        MENU_SORA.lock().map(|mut m| *m = Some(Instant::now())).ok();
        // KH2's model (`model`) over it: KH1's menu animation as posed, on KH2's skeleton. Taken
        // in KH1's world space as it stands (placement included), so drawn with no other.
        if let (true, Some(s)) = (super::model::on(), g.as_ref()) {
            if let Some(inv) = s.inverse.as_ref() {
                let world = ptr(rd::<u32>(instance + WORLD));
                if world != 0 && super::crash::readable(world) && super::crash::readable(world + SORA_BONES as usize * 0x40 - 1) {
                    let kh1: Vec<Mat4> = (0..SORA_BONES as usize).map(|i| mat(world + i * 0x40)).collect();
                    if kh1.iter().all(|m| m.is_finite()) {
                        super::model::note_menu(instance, inv.pose(&kh1), r_ratio(s.retarget.as_ref()));
                    }
                }
            }
        }
        return;
    }
    if g.as_ref().map_or(true, |s| s.instance != instance) {
        let table = ptr(rd::<u32>(header + 4));
        if table == 0 {
            return;
        }
        let bytes = std::slice::from_raw_parts(table as *const u8, SORA_BONES as usize * 0x30);
        let kh1 = match kh1_data::mdls::parse_bones(bytes, 0, SORA_BONES as usize) {
            Ok(b) => b,
            Err(e) => {
                log(&format!("Sora's bone table at {table:#x} did not read: {e}"));
                return;
            }
        };
        log(&format!("Sora's instance {instance:#x} (bones at {table:#x})"));
        let bind = kh2_anim::Pose::bind(&kh1).bones;
        *g = Some(Sora { instance, retarget: None, bind, calls: 0, checked: false, follow: Follow::default(), to_world: None, wrote_root: None, last_drawn: None, param_agree: 0, param_ok: PARAM_OK.load(Ordering::Relaxed), jumps: 0, parents: kh1.iter().map(|b| b.parent).collect(), jiggle: None, secondary: None, inverse: None, kh2_now: None, src_now: src::NONE, src: src::NONE, shown: Vec::new(), handback: None });
        KH1_BONES.lock().map(|mut b| *b = kh1).ok();
    }
    let s = g.as_mut().unwrap();
    s.calls += 1;
    SORA_INSTANCE.store(instance, Ordering::Relaxed);
    if s.retarget.is_none() {
        if let (Some(Ok(kh2)), Some(rest)) = (KH2.get(), REST.get()) {
            if let Ok(b) = KH1_BONES.lock() {
                let mut r = Retarget::new(&b, &kh2.rig.skeleton);
                if let Some(m) = rest {
                    r.set_rest(&m.apply(&b, 0.0));
                }
                if let Some(Some(m)) = REST_OPEN.get() {
                    r.set_rest_open(&m.apply(&b, 0.0));
                }
                log(&format!("retarget ready: {} of {} KH1 bones follow KH2 (the rest hang as in KH1's battle idle: {})", r.mapped(), b.len(), rest.is_some()));
                s.retarget = Some(r);
                let sec = kh1_data::secondary::Secondary::new(&b, &kh2.rig.skeleton, kh2.rig.weapons.first().map(|w| w.skeleton.as_slice()));
                log(&format!(
                    "KH2's own hair, hood and keychain: {} spikes ({}), {} hood joints, {} chain links",
                    sec.hair().len(),
                    sec.hair().iter().map(|l| format!("{}<-{}", l.kh1, l.kh2)).collect::<Vec<_>>().join(" "),
                    sec.hood().len(),
                    sec.chain().len()
                ));
                s.secondary = Some(sec);
                let grip = kh1_data::retarget::GRIP_BONE;
                if let Some(r) = s.retarget.as_ref() {
                    let inv = kh1_data::inverse::Inverse::new(&b, &kh2.rig.skeleton, r, |i| i < grip && r.follows_one_part(i));
                    log(&format!("KH1's poses on KH2's skeleton: {} of {} KH2 bones turn with a measured part", inv.mapped(), kh2.rig.skeleton.len()));
                    s.inverse = Some(inv);
                }
            }
        }
    }
    let (world, local) = (ptr(rd::<u32>(instance + WORLD)), ptr(rd::<u32>(instance + LOCAL)));
    if world == 0 || local == 0 {
        return;
    }
    let raw0: [f32; 16] = rd(world);
    let ours = s.wrote_root == Some(raw0);
    let derived = (!ours)
        .then(|| {
            let g0 = Mat4::from_cols_array(&raw0);
            let l0 = mat(local);
            (affine(&g0) && affine(&l0) && l0.determinant().abs() > 1e-6).then(|| g0 * l0.inverse())
        })
        .flatten();
    let param = from_param(param_world);
    // Learn whether the pass's world matrix is the placement (it can't be thrown by blends).
    if let (Some(d), Some(p), false) = (derived, param, s.param_ok) {
        if close(&d, &p, 1.0) {
            s.param_agree += 1;
            if s.param_agree >= 30 {
                s.param_ok = true;
                PARAM_OK.store(true, Ordering::Relaxed);
                log("placement: the skeleton pass's own world matrix matches; using it from now on");
            }
        } else {
            if s.param_agree > 0 || s.calls == 30 {
                log(&format!("placement: pass matrix {:?} vs from the bones {:?}", p.to_cols_array().map(|v| (v * 100.0).round() / 100.0), d.to_cols_array().map(|v| (v * 100.0).round() / 100.0)));
            }
            s.param_agree = 0;
        }
    }
    let to_world = match (s.param_ok, param, derived, s.to_world) {
        (true, Some(p), ..) => p,
        // From the bones, unless it jumped (a KH1 blend mixing in the pose we drew): a real
        // move is a few units a frame; keep the last unless the jump persists.
        (_, _, Some(d), Some(last)) => {
            if close(&d, &last, 30.0) || s.jumps >= 3 {
                s.jumps = 0;
                d
            } else {
                s.jumps += 1;
                last
            }
        }
        (_, _, Some(d), None) => d,
        (_, _, None, Some(last)) => last,
        _ => return,
    };
    // (KH1's own placement can lag his position: KH1's roll moves the actor and pulls its model
    // matrix back by the roll's root travel, its root bone putting him back, and at the roll's
    // end that pull lingers for some frames. KH2's poses, and the pose we last drew blending into
    // KH1's, sat up to ~300 behind him there, then snapped: the user, "soras model decouples when
    // inputting a kh1 dodge roll". The placement is his position.)
    let to_world = on_actor(to_world);
    if !s.checked && s.calls >= 30 && !ours {
        s.checked = true;
        let chain = mat(world) * mat(local + 0x40) * mat(local + 0x80) * mat(local + 0xC0);
        let err = (chain.w_axis - mat(world + 0xC0).w_axis).length();
        log(&format!("check: bone 3 from the locals is {err:.3} units from the game's (should be ~0)"));
    }
    if let Ok(mut at) = SORA_AT.try_lock() {
        *at = Some(to_world.w_axis.truncate().to_array());
    }
    let orig_hands = (!ours).then(|| HANDS.map(|(b, _)| mat(world + b * 0x40)));
    let orig_blade = (!ours).then(|| (mat(world + 280 * 0x40), mat(world + 283 * 0x40)));
    let slot = slot_of(instance);
    SLOT_NOW.store(slot.unwrap_or(usize::MAX), Ordering::Relaxed);
    // Which weapon and how KH1 holds it, logged once a weapon (KH1's grip itself is not used:
    // it moves with KH1's motions, stage 53).
    if let (false, Some(0..=7), Some(&Some(p)), Some(r)) = (ours, slot, s.parents.get(kh1_data::retarget::GRIP_BONE).map(|p| (*p >= 0).then_some(*p as usize)).as_ref(), s.retarget.as_ref()) {
        let live = mat(world + p * 0x40).inverse() * mat(world + kh1_data::retarget::GRIP_BONE * 0x40);
        // Which weapon (Sora's save entry `+0x36`) and how KH1 holds it, once per weapon.
        static WEAPON: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);
        let save = rd::<u64>(BASE.load(Ordering::Relaxed) + Rva(0x28695A0)) as usize;
        if save > 0x10000 {
            let w = rd::<u8>(save + 0x36) as u32;
            if WEAPON.swap(w, Ordering::Relaxed) != w {
                let (_, q, t) = live.to_scale_rotation_translation();
                let (ax, ang) = q.to_axis_angle();
                log(&format!("weapon {w:#x}: KH1's grip turned {:.2} rad from the Keyblade's rest (KH1's grip: axis {ax:.2?} angle {ang:.2}, offset {t:.1?})", r.grip_turn(&live).unwrap_or(-1.0)));
            }
        }
    }
    let Some(mut pose) = wanted(s, slot, &to_world) else {
        s.wrote_root = None;
        s.to_world = Some(to_world);
        // KH1's own animation takes over: blended in from the pose we drew last.
        if let (false, Some((last, at))) = (ours, s.last_drawn.as_ref()) {
            let w = at.elapsed().as_secs_f32() / KH1_BLEND;
            if w < 1.0 && last.len() >= SORA_BONES as usize {
                let inv = to_world.inverse();
                // (Eased out: KH1's motion shows at once, our last pose fades.)
                let w = 1.0 - (1.0 - w) * (1.0 - w);
                for (i, from) in last.iter().enumerate().take(SORA_BONES as usize) {
                    let kh1 = inv * mat(world + i * 0x40);
                    let (fs, fr, ft) = from.to_scale_rotation_translation();
                    let (ks, kr, kt) = kh1.to_scale_rotation_translation();
                    let m = Mat4::from_scale_rotation_translation(fs.lerp(ks, w), fr.slerp(kr, w), ft.lerp(kt, w));
                    if m.is_finite() {
                        std::ptr::write_unaligned((world + i * 0x40) as *mut [f32; 16], (to_world * m).to_cols_array());
                    }
                }
                // (The dream weapons kept whole through the blend.)
                dream_hold_world(world, &s.parents);
            } else {
                s.last_drawn = None;
            }
        }
        // (As drawn, for a blend out of it: `handback`.)
        if !ours {
            let inv = to_world.inverse();
            s.shown.clear();
            s.shown.extend((0..SORA_BONES as usize).map(|i| inv * mat(world + i * 0x40)));
            s.src = src::KH1;
            s.handback = None;
        }
        if let Some(o) = orig_hands {
            if let Ok(mut h) = HANDS_NOW.lock() {
                *h = Some(Hands { orig: o, drawn: o, at: Instant::now() });
            }
        }
        // KH2's model (`model`) on KH1's own animation, as drawn.
        if let (true, Some(inv)) = (super::model::on(), s.inverse.as_ref()) {
            let back = to_world.inverse();
            let kh1: Vec<Mat4> = (0..SORA_BONES as usize).map(|i| back * mat(world + i * 0x40)).collect();
            super::model::note_pose(&inv.pose(&kh1), &[], &to_world, r_ratio(s.retarget.as_ref()));
        }
        return;
    };
    // Anti Form's hang and climb on KH1's ledge grab (`anti_ledge`): KH1 places him for its own
    // hang, Anti's hands sit elsewhere on its body (the user's screenshot: hanging well down the
    // wall, "looks like his collision is a little off"). The pose is moved so its hands are where
    // KH1's own hands are this frame (KH1's pass drew them on the ledge).
    if ANTI_LEDGE.swap(false, Ordering::Relaxed) {
        // (This frame's bones are KH1's own pass's, not ours from before: `ours`.)
        let ours_frame = ours;
        let kh1_hands = orig_hands.or_else(|| HANDS_NOW.lock().ok().and_then(|h| h.map(|h| h.orig)));
        if let Some(h) = kh1_hands {
            let inv = to_world.inverse();
            let kh1_mid = (inv * h[0]).w_axis.truncate().lerp((inv * h[1]).w_axis.truncate(), 0.5);
            // (Anti hangs upside down by a foot, the user: "sora should be hanging from his foot, so
            // he should be significantly lower in this hang animation": the pose's topmost bone,
            // whatever it is, goes where KH1's hands hold the ledge.)
            let up = inv.transform_vector3(glam::Vec3::new(0.0, -1.0, 0.0)).normalize_or_zero();
            // (Of his two feet: any bone at all picked a hood string or the hidden Keyblade's, drawn
            // up at the ledge while his foot hung far below, the user's video.)
            let ours = super::feet::LEGS.iter().filter_map(|l| pose.get(l[2]).map(|m| m.w_axis.truncate())).filter(|p| p.is_finite()).max_by(|a, b| a.dot(up).total_cmp(&b.dot(up))).unwrap_or_default();
            // The ledge's height: KH1's own hang reaches over the edge (its highest bone this frame,
            // its fingers), higher than its hands' roots (the user, s279: "he is still hanging too
            // low, his foot should be hooked on the edge of the roof"). The ankle a little under it,
            // so the foot hooks over.
            static KH1_TOP: AtomicU32 = AtomicU32::new(u32::MAX);
            if !ours_frame {
                let top = (0..SORA_BONES as usize)
                    .map(|i| (inv * mat(world + i * 0x40)).w_axis.truncate())
                    .filter(|p| p.is_finite() && (*p - kh1_mid).length() < 80.0)
                    .map(|p| p.dot(up))
                    .fold(f32::MIN, f32::max);
                if top.is_finite() && top > f32::MIN {
                    KH1_TOP.store(top.to_bits(), Ordering::Relaxed);
                }
            }
            let top = f32::from_bits(KH1_TOP.load(Ordering::Relaxed));
            let target = if top.is_finite() { kh1_mid + up * (top - ANKLE_UNDER_EDGE - kh1_mid.dot(up)) } else { kh1_mid };
            let d = target - ours;
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 30 {
                log(&format!("Anti Form's ledge pose: its ankle {ours:.0?}, KH1's hands {kh1_mid:.0?} ({}), KH1's top {top:.0}: moved {d:.0?}", if ours_frame { "kept from an earlier frame" } else { "KH1's pass this frame" }));
            }
            // (s274 kept the move under 300 and so never made it: the hang's hands sit further.)
            if d.is_finite() && d.length() < 3000.0 {
                let t = Mat4::from_translation(d);
                for m in pose.iter_mut() {
                    *m = t * *m;
                }
            }
        }
    }
    // Out of one of KH1's grabs, a change of who draws him blends in from what was drawn last
    // frame (`HANDBACK_BLEND`): KH1's climb, fall or landing into ours, ours over KH1's idle into
    // KH2's own. (Not out of KH1's commands: an Attack cutting a spell swings at once, as KH2's
    // swings take no blend.)
    let now_src = s.src_now;
    if now_src != s.src && s.src != src::NONE && s.shown.len() >= SORA_BONES as usize && super::takeover::grab_blend() {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 40 {
            log(&format!("drawn by {} after {} out of KH1's grab: blended in (slot {slot:?})", src_name(now_src), src_name(s.src)));
        }
        s.handback = Some((decompose(&s.shown[..SORA_BONES as usize]), Instant::now()));
    }
    s.src = now_src;
    if let Some((from, t0)) = s.handback.as_ref() {
        let w = t0.elapsed().as_secs_f32() / HANDBACK_BLEND;
        if w >= 1.0 {
            s.handback = None;
        } else {
            // (Eased: the new pose shows at once, the old fades.)
            let w = 1.0 - (1.0 - w) * (1.0 - w);
            for (m, (fs, fr, ft)) in pose.iter_mut().zip(from) {
                let (ps, pr, pt) = m.to_scale_rotation_translation();
                let b = Mat4::from_scale_rotation_translation(fs.lerp(ps, w), fr.slerp(pr, w), ft.lerp(pt, w));
                if b.is_finite() {
                    *m = b;
                }
            }
        }
    }
    let kh2_feet = MODE.load(Ordering::Relaxed) == 0 && super::takeover::active() && super::takeover::GROUNDED.load(Ordering::Relaxed);
    super::feet::place(BASE.load(Ordering::Relaxed), &mut pose, &s.parents, &to_world, kh2_feet);
    // (KH1's own motion, `kh1_air_pose`: its hold is KH1's already.)
    if !NATIVE_KH1.swap(false, Ordering::Relaxed) {
        kh1_wrist(&mut pose, &s.parents, (!ours).then_some(world));
        keyblade_teeth_up(&mut pose, &s.parents);
    }
    let set = super::settings::get();
    if set.physics {
        // KH2's own animation of the hair, hood and keychain (the user: "add the animations for
        // sora's hair and chain on each KH2 animation instead"), on every pose KH2 gave; the
        // keychain only on a Keyblade (not the dream weapons or the wooden sword).
        let mut driven = kh1_data::secondary::Parts::default();
        if let (true, Some((body, weapon)), Some(sec)) = (set.kh2_hair, s.kh2_now.as_ref(), s.secondary.as_ref()) {
            let want = kh1_data::secondary::Parts { hair: true, hood: true, chain: !held_weapon().is_some_and(|w| (0x52..=0x55).contains(&w)) };
            let rep = sec.apply(&mut pose, body, (!weapon.is_empty()).then_some(weapon.as_slice()), want);
            driven = rep.parts;
            static LOGGED: AtomicU32 = AtomicU32::new(0);
            if LOGGED.fetch_add(1, Ordering::Relaxed) == 0 {
                log(&format!("hair, hood and keychain from KH2's own animation: {:?} (asked {want:?})", rep.parts));
            }
        }
        // The swing (`kh1_data::jiggle`) for any chain KH2 didn't pose. One step per game frame
        // (Sora's physics frames), not by the clock: frames come 13..22 ms apart, and a draw
        // with no step left the chains at his last place (the user: the animations stuttered).
        // A gap of more than half a second (KH1's own animation, a pause) starts it over.
        let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
        let (j, last) = s.jiggle.get_or_insert_with(|| (kh1_data::jiggle::Jiggle::new(kh1_data::jiggle::sora_chains()), frame));
        let steps = frame.saturating_sub(*last);
        if steps > 30 {
            j.reset();
        }
        *last = frame;
        j.apply_except(&mut pose, &s.parents, &to_world, steps.min(3) as usize, |b| driven.covers(b));
    }
    // A dream weapon whole, as KH1 holds it.
    dream_hold(&mut pose, &s.parents, &s.bind, (!ours).then_some(world));
    // Final Form's Keyblade floating where KH2's weapon motion has it.
    if let (Some((body, weapon)), true) = (s.kh2_now.as_ref(), super::form::in_final()) {
        super::leftblade::final_float(&mut pose, &s.bind, body, weapon, s.retarget.as_ref().map_or(1.0, |r| r.hip_ratio()), &to_world);
    }
    // Limit Form's Keyblade: thrown (Strike Raid) drawn where it flies; where it is drawn noted for
    // the effects on it.
    if s.kh2_now.is_some() {
        super::limits::pose_hook(&mut pose, &s.bind, &to_world, s.retarget.as_ref().map_or(1.0, |r| r.hip_ratio()));
    }
    for (i, m) in pose.iter().enumerate().take(SORA_BONES as usize) {
        std::ptr::write_unaligned((world + i * 0x40) as *mut [f32; 16], (to_world * *m).to_cols_array());
    }
    s.wrote_root = Some(rd(world));
    match s.last_drawn.as_mut() {
        Some((v, t)) if v.len() == pose.len() => {
            v.copy_from_slice(&pose);
            *t = Instant::now();
        }
        _ => s.last_drawn = Some((pose.clone(), Instant::now())),
    }
    s.shown.clear();
    s.shown.extend_from_slice(&pose[..pose.len().min(SORA_BONES as usize)]);
    s.to_world = Some(to_world);
    let drawn = HANDS.map(|(b, _)| to_world * pose[b]);
    if let Ok(mut k) = BLADE.try_lock() {
        let prev = k.map(|k| (k.0, k.1));
        if let Some(o) = orig_blade.or(prev) {
            *k = Some((o.0, o.1, to_world * pose[280], to_world * pose[283], ours));
        }
    }
    // Valor's second Keyblade (`leftblade`): the left hand's weapon bones, from this pose and
    // KH2's, handed to its draw (it does nothing outside Valor or with it off).
    // KH2's model (`model`): KH2's own pose, or KH1's (its own motion drawn) on KH2's skeleton.
    match (s.kh2_now.as_ref(), s.inverse.as_ref()) {
        (Some((body, weapon)), _) => super::model::note_pose(body, weapon, &to_world, r_ratio(s.retarget.as_ref())),
        (None, Some(inv)) if super::model::on() => super::model::note_pose(&inv.pose(&pose), &[], &to_world, r_ratio(s.retarget.as_ref())),
        _ => {}
    }
    if let Some((body, weapon)) = s.kh2_now.as_ref() {
        super::leftblade::note_pose(&pose, &s.bind, body, weapon, &to_world, s.retarget.as_ref().map_or(1.0, |r| r.hip_ratio()));
    }
    if let Ok(mut h) = HANDS_NOW.lock() {
        let orig = orig_hands.or(h.map(|h| h.orig)).unwrap_or(drawn);
        *h = Some(Hands { orig, drawn, at: Instant::now() });
    }
}

static KH1_BONES: Mutex<Vec<kh2_formats::anim::Bone>> = Mutex::new(Vec::new());

/// KH1's size against KH2's (`Retarget::hip_ratio`), 1 without a retarget.
fn r_ratio(r: Option<&Retarget>) -> f32 {
    r.map_or(1.0, |r| r.hip_ratio())
}

/// A small model posed while Sora's hands are known: is it held (its world matrix a fixed
/// relation to KH1's hand while the hand moves)? Returns the world matrix to pose it with
/// instead, once it is held and Sora is drawn with KH2's motions.
unsafe fn held(instance: usize, world: usize) -> Option<Mat4> {
    let bones = bone_count(instance)?;
    if bones == SORA_BONES || bones > 40 || world < 0x10000 {
        return None;
    }
    let hands = (*HANDS_NOW.lock().ok()?)?;
    if hands.at.elapsed().as_secs_f32() > 0.1 {
        return None;
    }
    let w = mat(world);
    if !affine(&w) {
        return None;
    }
    let mut g = WATCH.lock().ok()?;
    let map = g.get_or_insert_with(HashMap::new);
    if map.len() > 64 && !map.contains_key(&instance) {
        return None;
    }
    let v = map.entry(instance).or_default();
    if let Some(hand) = v.held {
        let rel = v.rel?.1;
        let o = hands.orig[hand];
        // Still held? (A dropped weapon or a model reused for something else lets go.)
        let now_rel = o.inverse() * w;
        if (now_rel.w_axis - rel.w_axis).length() > 8.0 {
            v.held = None;
            v.steady = 0;
            log(&format!("the {bones}-bone model at {instance:#x} let go of the {} hand", HANDS[hand].1));
            return None;
        }
        return Some(hands.drawn[hand] * rel);
    }
    // Learning: the relation to each hand, steady over frames in which the hand moved.
    let mut best: Option<(usize, Mat4, f32)> = None;
    for (h, o) in hands.orig.iter().enumerate() {
        let d = (w.w_axis - o.w_axis).truncate().length();
        if d < 60.0 && best.map_or(true, |b| d < b.2) {
            best = Some((h, o.inverse() * w, d));
        }
    }
    let Some((hand, rel, dist)) = best else {
        v.rel = None;
        v.steady = 0;
        return None;
    };
    let hand_pos = hands.orig[hand].w_axis.truncate();
    if let Some(p) = v.last_hand {
        v.hand_moved += (hand_pos - p).length();
    }
    v.last_hand = Some(hand_pos);
    match v.rel {
        Some((h, r)) if h == hand && (r.w_axis - rel.w_axis).length() < 0.5 && (r.x_axis - rel.x_axis).length() < 0.02 => v.steady += 1,
        _ => {
            v.rel = Some((hand, rel));
            v.steady = 0;
            v.hand_moved = 0.0;
        }
    }
    if v.steady >= 20 && v.hand_moved > 10.0 {
        // (Placed from a hand learnt while KH1's own pose showed; a model placed from the hand
        // we draw never looks steady against KH1's while KH2's motions show, and needs nothing.)
        v.held = Some(hand);
        if !v.logged {
            v.logged = true;
            log(&format!("the {bones}-bone model at {instance:#x} is held in Sora's {} hand ({dist:.1} units from it); it now follows the hand drawn", HANDS[hand].1));
        }
    }
    None
}

type Pose = unsafe extern "system" fn(usize, usize, f32, usize);
/// The third argument is a float (XMM2; the hook audit of stage 49).
type PoseIk = unsafe extern "system" fn(usize, usize, f32);

unsafe extern "system" fn pose(world: usize, instance: usize, t: f32, motion: usize) {
    let f: Pose = std::mem::transmute(ORIG_POSE.load(Ordering::Acquire));
    let moved = if instance != 0 && MODE.load(Ordering::Relaxed) <= 1 { held(instance, world) } else { None };
    match moved {
        Some(m) => {
            let a = m.to_cols_array();
            f(a.as_ptr() as usize, instance, t, motion);
        }
        None => f(world, instance, t, motion),
    }
    if instance != 0 && MODE.load(Ordering::Relaxed) <= 1 {
        note_small(instance, world);
    }
    if instance != 0 {
        let tp = Instant::now();
        check(instance, false, world);
        if instance == SORA_INSTANCE.load(Ordering::Relaxed) {
            super::prof::add(super::prof::POSE, tp);
        }
    }
}

unsafe extern "system" fn pose_ik(world: usize, instance: usize, flags: f32) {
    LAST_T.store(flags.to_bits(), Ordering::Relaxed);
    let f: PoseIk = std::mem::transmute(ORIG_POSE_IK.load(Ordering::Acquire));
    let moved = if instance != 0 && MODE.load(Ordering::Relaxed) <= 1 { held(instance, world) } else { None };
    match moved {
        Some(m) => {
            let a = m.to_cols_array();
            f(a.as_ptr() as usize, instance, flags);
        }
        None => f(world, instance, flags),
    }
    if instance != 0 && MODE.load(Ordering::Relaxed) <= 1 {
        note_small(instance, world);
    }
    if instance != 0 {
        let tp = Instant::now();
        check(instance, true, world);
        if instance == SORA_INSTANCE.load(Ordering::Relaxed) {
            super::prof::add(super::prof::POSE, tp);
        }
    }
}

pub unsafe fn install() {
    let base = sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + POSE_RVA, &POSE_PROLOGUE, pose as *const () as usize, &ORIG_POSE, "skeleton pass");
    hook::install(base + POSE_IK_RVA, &POSE_IK_PROLOGUE, pose_ik as *const () as usize, &ORIG_POSE_IK, "skeleton pass with ground IK");
}
