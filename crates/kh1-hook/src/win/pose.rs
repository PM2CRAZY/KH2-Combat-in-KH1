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
    /// KH2's 151 / 152 (its no-target rows) and 154 finishes; in the air 181 / 182 alternate and
    /// 183 finishes. `second`: which of the two ground swings KH1's slot is (KH1's second swing
    /// is KH2's second, the user: "sora base attack 2, kh2 base attack 2"), or `None` to
    /// alternate them swing by swing.
    Swing { air: bool, finisher: bool, second: Option<bool> },
    /// One of our slots in KH1's motion table (`kh2slots`): its KH2 motion (`field`: its field
    /// variant), at KH1's own time and blend (KH1 plays the slot; its length is the KH2 motion's).
    Slot { motion: u16, field: bool },
}

/// KH2's swing for KH1's ground swing slot (`play_for`): 98 (KH1's first and third swing) KH2's
/// first, 151; 99 (its second) KH2's second, 152; 101 (its finisher) KH2's finisher, 154 (the
/// user: "kh1 base finisher, kh2 base finishers"); 100 (assumed a third swing) KH2's first.
/// (`shared::physics` moves KH1's swing by this clip's root travel at Base Combo KH2.)
pub fn swing_motion(slot: usize) -> Option<u16> {
    Some(match slot {
        98 | 100 => 151,
        99 => 152,
        101 => 154,
        _ => return None,
    })
}

/// KH1 Sora's slots, from the user's play logs (2026-10-13): 0 / 2 / 5 field idle, walk, run;
/// 1 / 3 / 6 the battle ones; 12 → 13 → 10 → 11 High Jump, rise, fall, land (the plain jump's
/// takeoff and rise are 8 / 9, the t314 traces); 98 → 99 → 98 → 101 the
/// ground combo (100 assumed a third swing); 102 / 103 / 104 the air combo (the user: air
/// combos showed the glide they were first taken for); 75..78 KH1's glide (`shared::slot_kind`:
/// gliding slow / fast, banking one way / the other: KH2's glide, and its two banked glides).
/// 4 / 7 are unseen and taken as battle walk / run.
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
        // (KH1's High Jump plays requests 0x9 / 0xa, slots 12 / 13; its plain jump 0x4 / 0x5, slots
        // 8 / 9: the t314 traces. KH2's High Jump clip is motion 201.)
        12 | 13 => mo(m::HIGH_JUMP, false, false, 1),
        8 | 9 => mo(m::JUMP, false, false, 1),
        10 => mo(m::FALL, false, true, 0),
        11 => mo(m::LAND, false, false, 0),
        98 => Play::Swing { air: false, finisher: false, second: Some(false) },
        99 => Play::Swing { air: false, finisher: false, second: Some(true) },
        100 => Play::Swing { air: false, finisher: false, second: None },
        101 => Play::Swing { air: false, finisher: true, second: None },
        102 | 103 => Play::Swing { air: true, finisher: false, second: None },
        104 => Play::Swing { air: true, finisher: true, second: None },
        // (KH1's glide: one group, so its slow / fast slots go on in the same clip.)
        75 | 76 => mo(m::GLIDE, false, true, 5),
        77 => mo(m::GLIDE_TURN_A, false, true, 5),
        78 => mo(m::GLIDE_TURN_B, false, true, 5),
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
    /// Final Form (`form`, stage 231: `P_EX100_ULTF`, its floating Keyblades).
    pub final_form: Option<Form>,
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
    /// Its clips' root travel by motion (as `Kh2::roots` for Sora): taken out of a pose drawn or
    /// baked for KH1, which moves him (`Baker::poses_of`, `follow_baked`).
    pub roots: HashMap<u16, Vec<[f32; 3]>>,
}

/// One moveset's data for KH1's motion slots (`kh2slots`, `Baker::poses_of`): base Sora's, or a
/// Drive Form's run on KH1's own states (`form::Kind::native_capable`, `form::native_form`).
pub struct Src<'a> {
    pub rig: &'a Arc<Rig>,
    pub character: &'a character_core::Character,
    pub roots: &'a HashMap<u16, Vec<[f32; 3]>>,
    /// The Drive Form whose moveset it is (else base Sora's).
    pub form: Option<super::form::Kind>,
}

impl Kh2 {
    /// Base Sora's moveset.
    pub fn src(&self) -> Src<'_> {
        Src { rig: &self.rig, character: &self.character, roots: &self.roots, form: None }
    }

    /// Drive Form `kind`'s, if it loaded.
    pub fn form_src(&self, kind: super::form::Kind) -> Option<Src<'_>> {
        kind.data(self).map(|f| Src { rig: &f.rig, character: &f.character, roots: &f.roots, form: Some(kind) })
    }
}

/// The KH2 moveset whose moves KH1's motion slots play on Sora now: the Drive Form's while he is
/// in one on KH1's own states (`form::native_form`: Valor, Wisdom), else base Sora's. KH2's motion
/// ids are the same in every moveset (151, 152, ... 184), so a clip looked up by the motion of the
/// slot playing comes from here.
pub fn kh2_moves() -> Option<&'static character_core::Character> {
    let k = kh2()?;
    if let Some(f) = super::form::native_form().and_then(|kind| kind.data(k)) {
        return Some(&f.character);
    }
    Some(&k.character)
}

/// Sora's moves a form gets from his own tables (the user: "make sure that soras square launch
/// attack works in valor"): Upper Slash (combo row 37, kind 0, Square in a combo, motion 167,
/// ability 95). KH2's Valor has none of Sora's Square moves; its own kind-0 rows (12, 38, 40, 11)
/// are its finishers and guard, so their ids aren't taken.
pub const PORTED_TO_VALOR: [u8; 1] = [37];

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
        log!(Animation, "form: Sora's combo row {id} (motion {m}, ability {}) added", row.ability);
        f.character.moveset.combo_rows.push(row);
    }
    f.rig = Arc::new(rig);
}

fn load_form(install: &kh2_data::Install, source: kh2_data::Source, spec: kh2_data::sora::PlayerSpec, skeleton: usize) -> Result<Form, String> {
    load_form_noting(install, source, spec, skeleton, false)
}

/// `load_form`; with `note`, every file read for it noted for the motion slot cache's manifest
/// (`KH2_INPUTS`: a form baked into KH1 motion slots, Valor's, `kh2slots`).
fn load_form_noting(install: &kh2_data::Install, source: kh2_data::Source, spec: kh2_data::sora::PlayerSpec, skeleton: usize, note: bool) -> Result<Form, String> {
    let data = if note {
        install.load_player_noting(source, spec, &|name, bytes| {
            if let Ok(mut v) = KH2_INPUTS.lock() {
                let name = format!("{} {name}", spec.model);
                v.retain(|(n, _, _)| *n != name);
                v.push((name, crate::slotcache::hash(bytes), bytes.len()));
            }
        })
    } else {
        install.load_player(source, spec)
    }
    .map_err(|e| e.to_string())?;
    let rig = data.rig.clone().ok_or_else(|| data.rig_error.clone().unwrap_or_else(|| "no rig".into()))?;
    if rig.skeleton.len() != skeleton {
        return Err(format!("{} bones, Sora has {skeleton}", rig.skeleton.len()));
    }
    let script = install.read_any(source, &format!("obj/{}.mdlx", spec.model)).ok().and_then(|b| kh2_magic::SpellFile::bar_entry(&b, 3));
    let archive = install.read_any(source, &format!("obj/{}.a.us", spec.model)).ok().map(Arc::new);
    let roots = data.character.moveset.clips.iter().map(|(m, c)| (*m, c.root.clone())).collect();
    Ok(Form { rig, character: data.character.clone(), battle: Arc::new(data.combat.battle.clone()), script, archive, roots })
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
    // (Our slot standing in for KH1's: the length that plays, `kh2slots`.)
    if let Some(n) = super::kh2slots::frames_for(slot) {
        return Some(n);
    }
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
    if super::log_on(super::LogCat::Animation) && N.fetch_add(1, Ordering::Relaxed) < 30 {
        log!(Animation, "placement {gap:.1} off Sora's position (KH1 slot {:?}): drawn at his position", SLOT_NOW.load(Ordering::Relaxed));
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
    /// Last KH2 pose (model space), and the pose blended from on a change.
    last: Vec<Mat4>,
    from: Vec<(Vec3, Quat, Vec3)>,
    /// When `last` was drawn.
    last_at: Option<Instant>,
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
    HANDS_NOW.try_lock().ok().and_then(|h| h.as_ref().map(|h| super::frameadv::age(h.at).as_millis() < ms)).unwrap_or(true)
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

/// A pointer to a game handle, as KH1 makes them (Steam `0x14038AD90`): `0x80000000 | (handle table index <<
/// 25) | (low 25 bits)`, the index of the table entry holding its high bits; 0 if none does (t345).
pub fn handle(p: usize) -> u32 {
    if p == 0 {
        return 0;
    }
    let table = BASE.load(Ordering::Relaxed) + HANDLE_TABLE_RVA;
    let hi = p & !0x1FF_FFFF;
    for i in 0..64usize {
        let e = unsafe { *((table + i * 8) as *const u64) } as usize;
        if e == hi {
            return 0x8000_0000 | ((i as u32) << 25) | (p & 0x1FF_FFFF) as u32;
        }
        if e == usize::MAX {
            break;
        }
    }
    0
}

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
        // (KH1's title waits while this builds something, `loadscreen`; released when this
        // thread ends, however it ends.)
        let _loading = super::loadscreen::Guard;
        let t = Instant::now();
        let r = (|| -> Result<Kh2, String> {
            let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
            let install = kh2_data::Install::locate(&exe).map_err(|e| format!("KH2 install: {e}"))?;
            let source = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
            // (Every file read for him noted with its hash: the motion slot cache's manifest,
            // `slot_cache_manifest`.)
            let sora = install
                .load_sora_noting(source, &|name, bytes| {
                    if let Ok(mut v) = KH2_INPUTS.lock() {
                        v.retain(|(n, _, _)| n != name);
                        v.push((name.to_string(), crate::slotcache::hash(bytes), bytes.len()));
                    }
                })
                .map_err(|e| format!("KH2 Sora: {e}"))?;
            if let Ok(mut v) = KH2_INPUTS.lock() {
                v.push((format!("(KH2 files: {source:?})"), 0, 0));
            }
            let rig = sora.rig.clone().ok_or_else(|| sora.rig_error.clone().unwrap_or_else(|| "no rig".into()))?;
            let roots = sora.character.moveset.clips.iter().map(|(m, c)| (*m, c.root.clone())).collect();
            let mut ground = HashMap::new();
            for m in [character_core::motion::WALK, character_core::motion::RUN] {
                for field in [false, true] {
                    let clip = if field { rig.field_motions.get(&m) } else { rig.motions.get(&m) };
                    if let Some(c) = clip {
                        let g = ground_speed(&rig, c);
                        log!(Animation, "KH2 {} {}: ground speed {g:.0} units/s", if field { "field" } else { "battle" }, if m == character_core::motion::WALK { "walk" } else { "run" });
                        ground.insert((m, field), g);
                    }
                }
            }
            log!(Animation, "KH2 Sora loaded ({source:?} files): {} bones, {} motions ({} with field variants), in {:.1}s", rig.skeleton.len(), rig.motions.len(), rig.field_motions.len(), t.elapsed().as_secs_f32());
            let wisdom = if super::settings::get().drive {
                let t = Instant::now();
                // (Its files noted for the slot cache: Wisdom's moves are baked into KH1 motion slots
                // too, `kh2slots::FORM_SLOTS`, Wisdom Form native.)
                match load_form_noting(&install, source, kh2_data::sora::FORMS[1], rig.skeleton.len(), true) {
                    Ok(f) => {
                        log!(Animation,
                            "KH2 Wisdom Form loaded: {} motions, {} combo rows, script {}, effects {}, in {:.1}s",
                            f.rig.motions.len(),
                            f.character.moveset.combo_rows.len(),
                            f.script.as_ref().map_or("missing".into(), |s| format!("{} bytes", s.len())),
                            if f.archive.is_some() { "read" } else { "missing" },
                            t.elapsed().as_secs_f32()
                        );
                        Some(f)
                    }
                    Err(e) => {
                        log!(Animation, "KH2 Wisdom Form did not load: {e}");
                        None
                    }
                }
            } else {
                None
            };
            let valor = if super::settings::get().drive {
                let t = Instant::now();
                // (Its files noted for the slot cache: Valor's moves are baked into KH1 motion
                // slots, `kh2slots`, Valor Form native.)
                match load_form_noting(&install, source, kh2_data::sora::FORMS[0], rig.skeleton.len(), true) {
                    Ok(mut f) => {
                        port_moves(&mut f, &sora.character, &rig, &PORTED_TO_VALOR);
                        log!(Animation, "KH2 Valor Form loaded: {} motions, {} combo rows, {} weapons, in {:.1}s", f.rig.motions.len(), f.character.moveset.combo_rows.len(), f.rig.weapons.len(), t.elapsed().as_secs_f32());
                        Some(f)
                    }
                    Err(e) => {
                        log!(Animation, "KH2 Valor Form did not load: {e}");
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
                        log!(Animation,
                            "KH2 Final Form loaded: {} motions, {} combo rows, {} weapons (on bones {:?}), effects {}, in {:.1}s",
                            f.rig.motions.len(),
                            f.character.moveset.combo_rows.len(),
                            f.rig.weapons.len(),
                            f.rig.weapons.iter().map(|w| w.bone).collect::<Vec<_>>(),
                            if f.archive.is_some() { "read" } else { "missing" },
                            t.elapsed().as_secs_f32()
                        );
                        Some(f)
                    }
                    Err(e) => {
                        log!(Animation, "KH2 Final Form did not load: {e}");
                        None
                    }
                }
            } else {
                None
            };
            Ok(Kh2 { rig, character: sora.character.clone(), battle: Arc::new(sora.combat.battle.clone()), magi: sora.magi.clone(), wisdom, valor, final_form, roots, ground })
        })();
        if let Err(e) = &r {
            log!(Animation, "KH2 Sora did not load: {e}");
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
            log!(Animation, "KH1 Sora's motion table: {} slots", motions.len());
            // KH2's moves as new slots of that table (`kh2slots`, ini `kh1_slots`).
            if let Some(Ok(k)) = KH2.get() {
                // Baked into KH1 motions (`kh1_bake`): KH1's skeleton and the retarget for it.
                let bake = if super::settings::get().kh1_bake {
                    match baker(&install, k, &motions) {
                        Ok(b) => Some(b),
                        Err(e) => {
                            log!(Animation, "KH2 slots: not baked ({e}); drawn over KH1's as before");
                            None
                        }
                    }
                } else {
                    None
                };
                // The motion slot cache (`slotcache`, `docs/kh1-asset-cache.md`): the baked blocks
                // from the mod's folder when everything they were baked from is unchanged.
                let mut cache = bake.as_ref().and_then(|_| open_slot_cache(&install, &bytes));
                super::kh2slots::prepare(&bytes, k, bake.as_ref(), cache.as_mut());
                if let Some(c) = cache {
                    save_slot_cache(&c);
                }
            }
            let _ = REST.set(motions.get(REST_SLOT).cloned().flatten());
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
                log!(Animation, "KH1 Sora's motion table did not load: {e}")
            }
        }
    });
}

/// The KH2 files read for KH2 Sora (name, hash, size), for the slot cache's manifest.
static KH2_INPUTS: Mutex<Vec<(String, u128, usize)>> = Mutex::new(Vec::new());

/// The motion slot cache's folder: beside the DLL (the mod's folder), `kh1-kh2-cache` (the effect
/// cache's too, `kh2dpx`).
pub(super) fn slot_cache_dir() -> std::path::PathBuf {
    std::path::Path::new(&super::dll_dir()).join("kh1-kh2-cache")
}

/// Everything the bake reads (`slotcache::Manifest`): the game build, KH1 Sora's motion table
/// (`mset`, also his idles for the retarget's rest) and skeleton, every KH2 file read for KH2
/// Sora, and the ini keys the bake reads. `None` (no cache this start) if any can't be read.
fn slot_cache_manifest(install: &kh1_data::Kh1Install, mset: &[u8]) -> Option<crate::slotcache::Manifest> {
    let mut m = crate::slotcache::Manifest::new();
    m.set("game build", format!("{:?}", super::addr::Build::current()));
    m.file("kh1 xa_ex_0010.mset", mset);
    let mdls = install.read(kh1_data::Source::Live, "xa_ex_0010.mdls").ok()?;
    m.file("kh1 xa_ex_0010.mdls", &mdls);
    let inputs = KH2_INPUTS.lock().ok()?;
    if inputs.iter().all(|(_, h, _)| *h == 0) {
        return None;
    }
    for (name, h, len) in inputs.iter() {
        if *h == 0 {
            m.set(name.clone(), "");
        } else {
            m.set(format!("file kh2 {name}"), format!("{h:032x} {len}"));
        }
    }
    let set = super::settings::get();
    m.set("ini kh1_slots", format!("{:?}", set.kh1_slots));
    m.set("ini kh1_leg_ik", set.kh1_leg_ik.to_string());
    m.set("ini keyblade_turn", set.keyblade_turn.to_string());
    Some(m)
}

fn open_slot_cache(install: &kh1_data::Kh1Install, mset: &[u8]) -> Option<crate::slotcache::Cache> {
    let t = Instant::now();
    let Some(manifest) = slot_cache_manifest(install, mset) else {
        log!(Loading, "slot cache: its inputs couldn't all be read; baking without it");
        return None;
    };
    let dir = slot_cache_dir();
    let bytes = crate::slotcache::read(&dir);
    let (c, miss) = crate::slotcache::Cache::open(manifest, bytes);
    let why = match miss {
        None => "matches this start's files and settings".to_string(),
        Some(crate::slotcache::Miss::Missing) => "none yet".to_string(),
        Some(crate::slotcache::Miss::Damaged(e)) => format!("unusable ({e}), rebuilt"),
        Some(crate::slotcache::Miss::Changed(d)) => format!("rebuilt: {}", d.join(", ")),
    };
    log!(Loading, "slot cache: {} ({why}; checked in {:.0} ms)", dir.join(crate::slotcache::FILE_NAME).display(), t.elapsed().as_secs_f32() * 1000.0);
    Some(c)
}

fn save_slot_cache(c: &crate::slotcache::Cache) {
    log!(Loading, "slot cache: {} block(s) from the cache, {} baked, {} the same bake again (shared)", c.hits, c.misses, c.shared);
    if !c.needs_write() {
        return;
    }
    super::loadscreen::saving();
    match c.write(&slot_cache_dir()) {
        Ok(p) => log!(Loading, "slot cache: written ({})", p.display()),
        Err(e) => log!(Loading, "slot cache: not written ({e}); baked again next start"),
    }
}

/// Every model size posed (the first 40 kinds), logged once each.
fn note_model(bones: u32, instance: usize, via_ik: bool) {
    static SEEN: Mutex<Vec<(u32, bool)>> = Mutex::new(Vec::new());
    let Ok(mut v) = SEEN.try_lock() else { return };
    if v.len() < 40 && !v.contains(&(bones, via_ik)) {
        v.push((bones, via_ik));
        log!(Animation, "posed: a {bones}-bone model (instance {instance:#x}) by {}", if via_ik { "the ground-IK pass" } else { "the plain pass" });
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
    (super::frameadv::age(*at).as_secs_f32() < 0.5).then(|| f(pose, w))
}

/// The blend argument of KH1's last ground-IK pass (diagnostics).
static LAST_T: AtomicU32 = AtomicU32::new(0);

/// KH1's wrist (the weapon hand relative to its forearm) for the weapon held KH1's way, while
/// KH1 posed him this frame (else the last one seen).
static KH1_WRIST: Mutex<Option<Mat4>> = Mutex::new(None);

/// Weapons KH2's sword grip holds the wrong way round (the ini's `flip_weapons`, by Sora's
/// weapon id, save `+0x36`; the user: the Dream Shield's front faced Sora) are held with
/// KH1's own wrist: the weapon hand (the grip's parent) takes KH1's turn relative to the
/// forearm, carrying the fingers and the weapon with it; KH2's arm swings as before.
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
    if let Some(w) = orig_world {
        let l = mat(w + arm * 0x40).inverse() * mat(w + hand * 0x40);
        if l.is_finite() {
            if let Ok(mut g) = KH1_WRIST.lock() {
                *g = Some(l);
            }
        }
    }
    let Some(local) = KH1_WRIST.lock().ok().and_then(|g| *g) else { return };
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

/// A dream weapon held rigidly as KH1 holds it (the tester: the Dream Sword "stretches" the
/// split second a movement begins, the rest of him not, and the Dream Shield is held another
/// way in battle than in the cutscenes): its bones were posed one by one (blends, the keychain
/// swing), so the mesh between them stretched. `kh1_world`: KH1's world matrices when it posed
/// him this frame (its hold is noted from them); then every weapon bone is put where KH1's hold
/// puts it from the hand drawn.
unsafe fn dream_hold(pose: &mut [Mat4], parents: &[i16], kh1_world: Option<usize>) {
    let Some(w) = held_weapon().filter(|w| dream_weapon(*w)) else { return };
    let grip = kh1_data::retarget::GRIP_BONE;
    let Some(hand) = parents.get(grip).copied().filter(|p| *p >= 0).map(|p| p as usize) else { return };
    if pose.len() < grip + WEAPON_BONES || hand >= pose.len() {
        return;
    }
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
        let near = rel[0].w_axis.truncate().length() < DREAM_NEAR && rel[1..].iter().all(|m| m.w_axis.truncate().length() < DREAM_NEAR);
        if !near {
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 10 {
                let far = rel.iter().map(|m| m.w_axis.truncate().length()).fold(0.0f32, f32::max);
                log!(Animation, "weapon {w:#x}: KH1's hold this frame not noted (a bone {far:.0} from the grip)");
            }
        }
        if near && rel.iter().all(|m| m.is_finite()) {
            if let Ok(mut h) = DREAM_HOLD.lock() {
                let new = h.as_ref().map_or(true, |(id, _)| *id != w);
                *h = Some((w, rel));
                if new {
                    log!(Animation, "weapon {w:#x}: held rigidly as KH1 holds it (its hold noted from KH1's pose)");
                }
            }
        }
    }
    let Some(rel) = DREAM_HOLD.lock().ok().and_then(|h| h.as_ref().filter(|(id, _)| *id == w).map(|(_, r)| r.clone())) else { return };
    let g = pose[hand] * rel[0];
    pose[grip] = g;
    for i in 1..WEAPON_BONES {
        pose[grip + i] = g * rel[i];
    }
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
    turn_grip(pose, parents);
}

/// The half turn of the grip and everything under it about the grip's x (the blade's length):
/// `keyblade_teeth_up`'s turn, also baked into KH2's slot motions (`Baker::poses`).
fn turn_grip(pose: &mut [Mat4], parents: &[i16]) {
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

/// What was drawn on Sora the last time (the trace's `draw_*` columns): which route chose the pose,
/// and for KH2's clips which one, how far into it, and the blend into it.
#[derive(Clone, Default)]
pub struct Drawn {
    /// "kh1_own" (KH1's animation, nothing of ours), "kh2_controller" (KH2's controller's pose) or
    /// "follow" (a KH2 clip drawn for KH1's slot, `follow_play`).
    pub route: &'static str,
    /// The KH1 motion slot of that draw.
    pub slot: Option<usize>,
    /// `Play` (Motion / Swing / Slot) with its numbers, "follow" only.
    pub play: String,
    /// The KH2 motion id drawn, "follow" only.
    pub motion: u16,
    /// Frames into that clip.
    pub frames: f32,
    /// The blend into it from the last pose (0..1), -1 none.
    pub blend: f32,
    /// The clip was (re)started this draw.
    pub restart: bool,
    pub at: Option<Instant>,
    /// Ms since that draw, when `drawn` read it.
    pub age_ms: u64,
}

/// 0: KH1's own, 1: KH2's controller's pose, 2: a KH2 clip for KH1's slot (set by `wanted`'s routes).
static DRAW_ROUTE: AtomicU32 = AtomicU32::new(0);
static DRAW_DETAIL: Mutex<Option<Drawn>> = Mutex::new(None);
static DRAW_LAST: Mutex<Option<Drawn>> = Mutex::new(None);

/// What was drawn last (for the trace); `at` is when.
pub fn drawn() -> Drawn {
    let mut d = DRAW_LAST.try_lock().ok().and_then(|d| d.clone()).unwrap_or_default();
    d.age_ms = d.at.map_or(0, |a| a.elapsed().as_millis() as u64);
    d
}

/// The model-space pose to draw for the current mode, if any, and a note of what was chosen.
fn wanted(s: &mut Sora, slot: Option<usize>, to_world: &Mat4) -> Option<Vec<Mat4>> {
    DRAW_ROUTE.store(0, Ordering::Relaxed);
    if let Ok(mut d) = DRAW_DETAIL.try_lock() {
        *d = None;
    }
    let pose = wanted_inner(s, slot, to_world);
    let mut d = match (pose.is_some(), DRAW_ROUTE.load(Ordering::Relaxed)) {
        (true, 1) => Drawn { route: "kh2_controller", ..Default::default() },
        (true, 2) => DRAW_DETAIL.try_lock().ok().and_then(|d| d.clone()).map(|d| Drawn { route: "follow", ..d }).unwrap_or(Drawn { route: "follow", ..Default::default() }),
        (false, 3) => DRAW_DETAIL.try_lock().ok().and_then(|d| d.clone()).map(|d| Drawn { route: "baked", ..d }).unwrap_or(Drawn { route: "baked", ..Default::default() }),
        _ => Drawn { route: "kh1_own", ..Default::default() },
    };
    d.slot = slot;
    d.at = Some(Instant::now());
    if let Ok(mut last) = DRAW_LAST.try_lock() {
        static N: AtomicU32 = AtomicU32::new(0);
        let changed = last.as_ref().map_or(true, |l| l.route != d.route || l.slot != d.slot || l.motion != d.motion || l.restart != d.restart && d.restart);
        if changed && N.fetch_add(1, Ordering::Relaxed) < 1500 {
            log!(Animation,
                "draw: {} (KH1 slot {}){}",
                d.route,
                slot.map_or("-".into(), |n| n.to_string()),
                if d.route == "follow" { format!(": KH2 motion {} {}, {:.0} frames in, blend {:.2}{}", d.motion, d.play, d.frames, d.blend, if d.restart { ", restarted" } else { "" }) } else { String::new() }
            );
        }
        *last = Some(d);
    }
    pose
}

fn wanted_inner(s: &mut Sora, slot: Option<usize>, to_world: &Mat4) -> Option<Vec<Mat4>> {
    let mode = MODE.load(Ordering::Relaxed);
    s.kh2_now = None;
    match mode {
        0 => {
            // KH1's area script walks him (zone entries and exits): KH1's own animation.
            if super::takeover::SCRIPTED.load(Ordering::Relaxed) {
                s.follow.slot = slot;
                return None;
            }
            // KH2's controller has him (a KH2 move, or a Drive Form): its pose. Recent, or the last
            // frame of his was KH2's (a pause stops the game's frames, not the clock: after one, the
            // old pose went stale and KH1's slot was drawn until KH2's next frame; the user: paused
            // mid KH2 move, resumed in KH1's).
            let kh2_frame = super::takeover::kh2_frame();
            // (Not over KH1's own moves from their first frame: its combo, its roll, an item's use.)
            let kh1_move = super::stylecombo::active() || super::styleroll::active() || super::takeover::item_use_now() || super::shared::active();
            let fresh = if kh1_move { None } else { super::takeover::POSE.try_lock().ok().and_then(|p| p.as_ref().filter(|(_, _, t)| kh2_frame || super::frameadv::age(*t).as_secs_f32() < 0.15).map(|(b, w, _)| (b.clone(), w.clone()))) };
            match (fresh, s.retarget.as_ref()) {
                (Some((b, w)), Some(r)) => {
                    s.follow.slot = slot;
                    DRAW_ROUTE.store(1, Ordering::Relaxed);
                    note_kh2(&b, to_world);
                    let pose = r.pose_grip(&b, super::takeover::GRIP.load(Ordering::Relaxed));
                    // (What KH2's clips blend from once KH1 has him again: this pose, not the last one
                    // a clip of ours drew, which was before KH2's move: the t317 trace, an Aerial
                    // Dodge's end blended in from the old jump pose.)
                    s.follow.last = b.iter().chain(w.iter()).copied().collect();
                    s.follow.last_at = Some(Instant::now());
                    s.follow.blend_start = None;
                    s.kh2_now = Some((b, w));
                    Some(pose)
                }
                // KH1's own states (`shared`): KH1's slot, drawn with KH2's motion for it where its
                // move style is KH2's (KH1 treats KH2's animation as its own), else KH1's own.
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
    kh2_weapon_n(rig, 0, motion, frames, looping, body)
}

/// The rig's weapon `n` (0 the right hand's, 1 a form's second) posed on its hand bone.
fn kh2_weapon_n(rig: &Rig, n: usize, motion: u16, frames: f32, looping: bool, body: &[Mat4]) -> Vec<Mat4> {
    let Some(w) = rig.weapons.get(n) else { return Vec::new() };
    let Some(hand) = w.bone.and_then(|b| body.get(b)).copied() else { return Vec::new() };
    let local = match w.motions.get(&motion).or_else(|| w.motions.get(&0)) {
        Some(m) => kh2_anim::evaluate(&w.skeleton, m, kh2_anim::key_time(m, frames, looping), kh2_anim::Options::default()).bones,
        None => kh2_anim::Pose::bind(&w.skeleton).bones,
    };
    local.iter().map(|l| hand * *l).collect()
}

/// What baking KH2's motions into KH1 motions takes at load (`kh2slots`, ini `kh1_bake`,
/// `kh1_data::bake`): KH1 Sora's skeleton from its own file, the retarget with KH1's battle and
/// field idles as its rest, and KH2's own hair, hood, sleeves and keychain (`secondary`), built
/// as `check` builds them for the field's Sora.
pub struct Baker {
    pub skeleton: Vec<kh2_formats::anim::Bone>,
    retarget: Retarget,
    secondary: kh1_data::secondary::Secondary,
}

impl Baker {
    pub fn new(kh2: &Kh2, skeleton: Vec<kh2_formats::anim::Bone>, rest: Option<&kh1_data::mset::Motion>, rest_open: Option<&kh1_data::mset::Motion>) -> Baker {
        let mut retarget = Retarget::new(&skeleton, &kh2.rig.skeleton);
        if let Some(m) = rest {
            retarget.set_rest(&m.apply(&skeleton, 0.0));
        }
        if let Some(m) = rest_open {
            retarget.set_rest_open(&m.apply(&skeleton, 0.0));
        }
        let secondary = kh1_data::secondary::Secondary::new(&skeleton, &kh2.rig.skeleton, kh2.rig.weapons.first().map(|w| w.skeleton.as_slice()));
        Baker { skeleton, retarget, secondary }
    }

    /// KH1's pose (model space) at each frame `0..=frames` of KH2's `motion`, as a tier A slot
    /// draws it (`follow_play`, `Play::Slot`): KH2's body and Keyblade, the root travel taken out
    /// (KH1 moves him), retargeted, KH2's hair, hood, sleeves and keychain on, the Keyblade's teeth-up turn (`keyblade_turn`). Nothing else of ours (feet,
    /// KH1's wrist, the swing).
    ///
    /// `travel` (t365, a move whose request has KH1's travel flag): KH2's root travel along the
    /// ground (from its first frame, at KH2's full distances: the user, t367) put back on the
    /// whole pose and returned per frame, for the bake's H10. KH1 moves him by H10
    /// and pulls his placement back by it; his body carries it, as KH1's own motions do (H2), so
    /// the model stays on him (`kh1_data::bake`). Else none (empty): the travel stays out.
    ///
    /// `field`: KH2's field (out-of-combat) variant of the motion, where it has one (its walk and
    /// run, `kh2slots::CYCLES`).
    ///
    /// `src`: the moveset (Sora's, `Kh2::src`, or a Drive Form's, `Kh2::form_src`: the same
    /// skeleton, its own clips, weapons and root travel; the form's moves baked into KH1 slots).
    #[allow(clippy::type_complexity)]
    pub fn poses_of(&self, src: &Src, motion: u16, field: bool, frames: u32, travel: bool) -> Option<(Vec<Vec<Mat4>>, Vec<Vec3>)> {
        let rig = src.rig;
        let clip = field.then(|| rig.field_motions.get(&motion)).flatten().or_else(|| rig.motions.get(&motion))?;
        let set = super::settings::get();
        let roots = src.roots.get(&motion);
        let parents: Vec<i16> = self.skeleton.iter().map(|b| b.parent).collect();
        let mut out = Vec::with_capacity(frames as usize + 1);
        let mut moved = Vec::new();
        let root_at = |f: u32| roots.and_then(|r| r.get((f as usize).min(r.len().saturating_sub(1)))).map(|p| Vec3::from(*p));
        let start = root_at(0).unwrap_or(Vec3::ZERO);
        // (A slot longer than the clip, `kh1rows::Ours::frames`: the clip loops over it from its
        // own loop point, as KH2 plays it; the Aerial Dodge's 202.)
        let looping = frames as f32 > kh2_anim::length_frames(clip) + 1.0;
        for f in 0..=frames {
            let at = f as f32;
            let mut body = kh2_anim::evaluate(&rig.skeleton, clip, kh2_anim::key_time(clip, at, looping), kh2_anim::Options::default()).bones;
            let mut weapon = kh2_weapon(rig, motion, at, looping, &body);
            if let Some(root) = roots.and_then(|r| r.get((f as usize).min(r.len().saturating_sub(1)))) {
                let un = Mat4::from_translation(-Vec3::from(*root));
                for m in body.iter_mut().chain(weapon.iter_mut()) {
                    *m = un * *m;
                }
            }
            let mut kh1 = self.retarget.pose(&body);
            // The Keyblade's teeth up (`keyblade_turn`; the user's t353 test: held the wrong way
            // round, as KH1's grip rolls the blade otherwise). Every weapon gets it in the bake:
            // the dream weapons and the wooden sword too (the user: they show the same). Before
            // the keychain, as `check` does: the chain hangs from the turned pommel.
            if set.keyblade_turn {
                turn_grip(&mut kh1, &parents);
            }
            {
                self.secondary.apply(&mut kh1, &body, (!weapon.is_empty()).then_some(weapon.as_slice()), kh1_data::secondary::Parts::ALL);
            }
            if travel {
                // (Along the ground only: the root's height stays out, as before.)
                let d = root_at(f).map_or(Vec3::ZERO, |p| p - start);
                // (KH2's units as they are: H10 moves him 1:1 in the world, Sora's placement
                // having scale 1 (t364's KH1 trace: swing 98's H10 1.3 and 6.32 a frame, moved
                // 1.30 and 6.32).)
                let t = Vec3::new(d.x, 0.0, d.z);
                for m in kh1.iter_mut() {
                    m.w_axis += t.extend(0.0);
                }
                moved.push(t);
            }
            out.push(kh1);
        }
        Some((out, moved))
    }
}

impl Baker {
    /// The holder motion's pose at each frame `0..=frames` of a two-Keyblade form's `motion`
    /// (Valor's; `kh2slots`, plan step 4 of `docs/kh1-weapon-second.md` §7.3): KH1 Sora's bind for
    /// every body bone, and the left hand's 13 weapon bones (`leftblade::left_bones`, from KH2's
    /// left Keyblade weapon motion, the form rig's second weapon, on KH1's pose `poses[f]` as the
    /// main bake gives it) where they sit from the left hand, put on the bind's left hand. KH1's
    /// pass poses it on an instance of ours (`leftblade::pose_holder`), and the bones are carried
    /// from that hand onto Sora's drawn one. `Err` names why it can't be made (no second weapon, a
    /// mirrored grip, which KH1's channels can't hold).
    pub fn holder_poses(&self, src: &Src, motion: u16, field: bool, poses: &[Vec<Mat4>]) -> Result<(Vec<Vec<Mat4>>, f32), String> {
        let rig = src.rig;
        let clip = field.then(|| rig.field_motions.get(&motion)).flatten().or_else(|| rig.motions.get(&motion)).ok_or("KH2 has no animation for it")?;
        let (Some(sr), Some(sl)) = (rig.weapons.first().and_then(|w| w.bone), rig.weapons.get(1).and_then(|w| w.bone)) else {
            return Err("the form's rig has no second weapon".into());
        };
        let frames = poses.len().saturating_sub(1) as u32;
        let looping = frames as f32 > kh2_anim::length_frames(clip) + 1.0;
        let kh1_bind = kh2_anim::Pose::bind(&self.skeleton).bones;
        let b2 = kh2_anim::Pose::bind(&rig.skeleton).bones;
        let (hand, grip, n) = (super::leftblade::LEFT_HAND_BONE, super::leftblade::GRIP_BONE, super::leftblade::WEAPON_BONE_COUNT);
        let ratio = self.retarget.hip_ratio();
        let mut out = Vec::with_capacity(poses.len());
        let mut reach = 0f32;
        for (f, kh1) in poses.iter().enumerate() {
            let at = f as f32;
            let body = kh2_anim::evaluate(&rig.skeleton, clip, kh2_anim::key_time(clip, at, looping), kh2_anim::Options::default()).bones;
            let right = kh2_weapon(rig, motion, at, looping, &body);
            let left = kh2_weapon_n(rig, 1, motion, at, looping, &body);
            let (Some(&r0), Some(&l0)) = (right.first(), left.first()) else {
                return Err("no Keyblade pose".into());
            };
            let (bones, from_hand) = super::leftblade::left_bones(kh1, &kh1_bind, &b2, (sr, sl), &body, r0, l0, ratio).ok_or("the left grip isn't finite")?;
            if glam::Mat3::from_mat4(bones[0]).determinant() < 0.0 {
                return Err(format!("the left grip is mirrored at frame {f}"));
            }
            reach = reach.max(from_hand);
            let mut p = kh1_bind.clone();
            let from = kh1_bind[hand] * kh1[hand].inverse();
            for i in 0..n {
                p[grip + i] = from * bones[i];
            }
            out.push(p);
        }
        Ok((out, reach))
    }
}

/// The baker for `kh2slots::prepare` (the loader thread): KH1 Sora's skeleton from
/// `xa_ex_0010.mdls`, his battle idle (slot 1) and field idle (slot 0) as the retarget's rest.
fn baker(install: &kh1_data::Kh1Install, kh2: &Kh2, motions: &[Option<kh1_data::mset::Motion>]) -> Result<Baker, String> {
    let mdls = install.read(kh1_data::Source::Live, "xa_ex_0010.mdls").map_err(|e| format!("xa_ex_0010.mdls: {e}"))?;
    let skeleton = kh1_data::mdls::parse_skeleton(&mdls).map_err(|e| format!("xa_ex_0010.mdls: {e}"))?;
    if skeleton.len() != SORA_BONES as usize {
        return Err(format!("xa_ex_0010.mdls has {} bones", skeleton.len()));
    }
    Ok(Baker::new(kh2, skeleton, motions.get(REST_SLOT).and_then(|m| m.as_ref()), motions.first().and_then(|m| m.as_ref())))
}

/// One of our slots baked into a KH1 motion (`kh2slots`, ini `kh1_bake`): KH1 plays it and shows
/// it with no pose of ours. KH2's pose for the same frame is still noted for KH2's effects on
/// his bones (`kh2fx`, `reflect`), as a tier A slot notes it; the next clip of ours doesn't blend
/// from a pose drawn before this one.
///
/// (t-bake) The KH2 pose of the baked frame (body and Keyblade, its root taken out, as `follow_play`
/// keeps its own) is kept as the last pose drawn, so a clip of ours drawn over KH1's next slot
/// blends from it as from one of its own: KH2's landing, still drawn, out of the baked fall.
fn follow_baked(s: &mut Sora, slot: Option<usize>, to_world: &Mat4, motion: u16) -> Option<Vec<Mat4>> {
    s.follow.slot = slot;
    s.follow.play = None;
    s.follow.last_at = None;
    s.follow.blend_start = None;
    s.follow.motion = motion;
    let actor = super::takeover::sora_actor();
    let frames = match slot {
        Some(n) if actor > 0x10000 => unsafe { super::kh2slots::kh1_time(actor, n) },
        _ => None,
    };
    // (A slot of a Drive Form's, `kh2slots::form_of`: the form's own rig and root travel.)
    let src = KH2.get().and_then(|k| k.as_ref().ok()).and_then(|k| match slot.and_then(super::kh2slots::form_of) {
        Some(kind) => k.form_src(kind),
        None => Some(k.src()),
    });
    if let (Some(src), Some(at)) = (src, frames) {
        let field = slot.is_some_and(super::kh2slots::is_field).then(|| src.rig.field_motions.get(&motion)).flatten();
        if let Some(clip) = field.or_else(|| src.rig.motions.get(&motion)) {
            let mut pose = kh2_anim::evaluate(&src.rig.skeleton, clip, kh2_anim::key_time(clip, at, false), kh2_anim::Options::default()).bones;
            let mut weapon = kh2_weapon(src.rig, motion, at, false, &pose);
            // (Valor's second Keyblade: the rig's second weapon on its hand bone; not on a slot with
            // a holder motion, whose left weapon bones KH1's own pass poses, `leftblade::pose_holder`.)
            let two = src.form.is_some_and(|k| k.two_blades()) && !slot.is_some_and(super::kh2slots::has_holder);
            let mut left = if two { kh2_weapon_n(src.rig, 1, motion, at, false, &pose) } else { Vec::new() };
            if let Some(root) = src.roots.get(&motion).and_then(|r| r.get((at as usize).min(r.len().saturating_sub(1)))) {
                let un = Mat4::from_translation(-Vec3::from(*root));
                for m in pose.iter_mut().chain(weapon.iter_mut()).chain(left.iter_mut()) {
                    *m = un * *m;
                }
            }
            note_kh2(&pose, to_world);
            s.follow.last = pose.iter().chain(weapon.iter()).copied().collect();
            s.follow.last_at = Some(Instant::now());
            // Valor on KH1's own states: its left Keyblade drawn by `leftblade` from this frame of
            // KH2's (the draw itself is still ours, a second queue of the model; after `wanted`,
            // `kh1_left_blade`).
            if two {
                let trail = src.character.moveset.clips.get(&motion).is_some_and(|c| c.has(at as u32, |k| matches!(k, character_core::WindowKind::WeaponTrail { .. })));
                super::leftblade::kh2_left((!left.is_empty()).then_some(left.as_slice()), &pose, motion, at as u32, trail);
                s.kh2_now = Some((pose, weapon));
            }
        }
    }
    DRAW_ROUTE.store(3, Ordering::Relaxed);
    if let Ok(mut d) = DRAW_DETAIL.try_lock() {
        *d = Some(Drawn { route: "baked", slot, play: format!("Baked {{ motion: {motion} }}"), motion, frames: frames.unwrap_or(-1.0), blend: -1.0, restart: false, at: None, age_ms: 0 });
    }
    None
}

fn decompose(v: &[Mat4]) -> Vec<(Vec3, Quat, Vec3)> {
    v.iter().map(|m| m.to_scale_rotation_translation()).collect()
}

/// KH2's motion for what KH1's slot shows Sora doing; `None` leaves KH1's own animation.
fn follow(s: &mut Sora, slot: Option<usize>, to_world: &Mat4) -> Option<Vec<Mat4>> {
    // One of our slots (`kh2slots`): its KH2 motion, whatever the styles now (KH1 plays it to its
    // end; its template underneath is never shown).
    if let Some(motion) = slot.and_then(super::kh2slots::kh2_motion) {
        // (Baked, `kh1_bake`: KH1's own pass shows it.)
        if slot.is_some_and(super::kh2slots::is_baked) {
            return follow_baked(s, slot, to_world, motion);
        }
        return follow_play(s, slot, to_world, Some(Play::Slot { motion, field: slot.is_some_and(super::kh2slots::is_field) }));
    }
    // Misc Animations at KH2 (`styles`, base Sora): KH1 uses the item (its effect, its timing, the
    // item's logic) and KH2's item-use motion is drawn over it (P_EX100 motion 9, A110: the
    // right hand raised overhead at frames ~20-30 of its 50, a voice at 0 and a sound at 10; KH1's
    // own use runs ~52 frames, `takeover::ITEM_CUT`), the user: "a setting for misc animations to
    // be KH1 or KH2? Like item throwing animation". (Group 9: a change of KH1's slot meanwhile
    // doesn't restart it.)
    if misc_kh2() && super::takeover::item_use_now() {
        return follow_play(s, slot, to_world, Some(Play::Motion { motion: KH2_ITEM_USE, field: false, looping: false, group: 9 }));
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
    let base_look = super::formmagic::BASE_KH1_CAST.load(std::sync::atomic::Ordering::Relaxed) && !super::form::in_form();
    if super::form::in_wisdom() || base_look {
        if let Some(spell @ 3..=6) = super::takeover::kh1_spell_now() {
            let air = super::takeover::air_cast().is_some();
            let motion = match (spell, air) {
                (4, false) => 62,
                (4, true) => 64,
                (_, false) => 65,
                (_, true) => 67,
            };
            return follow_play(s, slot, to_world, Some(Play::Motion { motion, field: false, looping: false, group: 0 }));
        }
    }
    // Every other slot by its move style (`shared::kh2_look`): KH2's motion for it at KH2, KH1's
    // own at KH1 (or with no style: KH1's magic, reactions, field actions).
    if MODE.load(Ordering::Relaxed) == 0 && !super::shared::kh2_look(slot) {
        s.follow.slot = slot;
        s.follow.play = None;
        return None;
    }
    follow_play(s, slot, to_world, None)
}

/// KH2 Sora's item-use motion (P_EX100 motion 9, `A110`).
const KH2_ITEM_USE: u16 = super::kh2slots::KH2_ITEM_USE;

/// KH2's misc animations for base Sora (the Misc Animations row at KH2, not in a form).
fn misc_kh2() -> bool {
    super::styles::kh2(super::styles::Kind::MiscAnims) && !super::form::in_form()
}

/// `follow` with the motion chosen by the caller (`forced`) instead of from KH1's slot.
fn follow_play(s: &mut Sora, slot: Option<usize>, to_world: &Mat4, forced: Option<Play>) -> Option<Vec<Mat4>> {
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
    // KH1's jump slots show KH2's jump while KH2's arc rises, then KH2's fall loop (KH2's own rule,
    // `kh2_rig`): KH1 keeps its rise / apex slot well into the fall, longest after KH2's early
    // release (the user: "letting go of jump still doesn't transition kh2 soras jump into his
    // falling animation"; trace t304: released at f5680, KH1's fall slot only at f5699).
    let play = match play {
        Some(Play::Motion { motion, .. }) if forced.is_none() && (motion == m2::JUMP || motion == m2::HIGH_JUMP) && super::shared::physics::kh2_arc_falling() => {
            Some(Play::Motion { motion: m2::FALL, field: false, looping: true, group: 0 })
        }
        // (No High Jump clip in KH2's files: its plain jump.)
        Some(Play::Motion { motion, field, looping, group })
            if motion == m2::HIGH_JUMP && KH2.get().and_then(|k| k.as_ref().ok()).map_or(true, |k| !k.rig.motions.contains_key(&motion)) =>
        {
            Some(Play::Motion { motion: m2::JUMP, field, looping, group })
        }
        p => p,
    };
    let new_slot = slot != f.slot;
    if new_slot {
        if f.logged < 800 {
            f.logged += 1;
            log!(Animation,
                "KH1 slot {} ({}) at speed {:.0}/s, climb {:.0}/s",
                slot.map_or("?".into(), |n| n.to_string()),
                play.map_or("KH1's own animation".into(), |p| format!("{p:?}")),
                f.speed,
                f.climb
            );
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
        Play::Swing { air, finisher, second } => {
            if new_slot {
                let fresh = f.last_swing.map_or(true, |t| now.duration_since(t).as_secs_f32() > COMBO_GAP);
                f.swings = if fresh { 0 } else { f.swings + 1 };
            }
            f.last_swing = Some(now);
            let first = second.map_or(f.swings % 2 == 0, |s| !s);
            let motion = match (air, finisher, first) {
                (false, true, _) => 154,
                (false, false, true) => 151,
                (false, false, false) => 152,
                (true, true, _) => 183,
                (true, false, true) => 181,
                (true, false, false) => 182,
            };
            // Stretch KH2's swing over KH1's, so the blade comes round when KH1's hit lands.
            let kh1_len = slot.and_then(|n| SLOTS.get()?.1.get(n).copied()).unwrap_or(0) as f32;
            let kh2_len = kh2.rig.motions.get(&motion).map_or(0.0, |m| kh2_anim::length_frames(m));
            let rate = if kh1_len > 1.0 && kh2_len > 1.0 { 60.0 * kh2_len / kh1_len } else { 60.0 };
            (motion, false, false, new_slot, rate, 0.0)
        }
        // (No rate: drawn at KH1's own time below; blended by KH1's blend clock.)
        // (Looping when our slot is longer than the clip, `kh1rows::Ours::frames`: the Aerial
        // Dodge's 202, drawn as baked.)
        Play::Slot { motion, field } => {
            let long = slot.and_then(super::kh2slots::frames_for).zip(kh2.rig.motions.get(&motion)).is_some_and(|(n, m)| n as f32 > kh2_anim::length_frames(m) + 1.0);
            (motion, field, long, new_slot || f.motion != motion || f.field != field, 60.0, 0.0)
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
    let actor = super::takeover::sora_actor();
    let kh1_time = match (play, slot) {
        (Play::Slot { .. }, Some(n)) if actor > 0x10000 => unsafe { super::kh2slots::kh1_time(actor, n) },
        _ => None,
    };
    if restart && matches!(play, Play::Slot { .. }) {
        // KH1's blend into our slot (if it asked for one) from the pose drawn last, if recent.
        f.from = if f.last_at.is_some_and(|t| now.duration_since(t).as_secs_f32() < 0.25) { decompose(&f.last) } else { Vec::new() };
        f.blend_start = None;
        f.motion = motion;
        f.field = false;
        f.since = Some(now);
        f.rate = 60.0;
    }
    if restart && !matches!(play, Play::Slot { .. }) {
        if !f.last.is_empty() && blend > 0.0 && f.last_at.is_some_and(|t| now.duration_since(t).as_secs_f32() < 0.25) {
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
    let rig: &kh2_data::sora::Rig = match super::form::current().and_then(|k| k.data(kh2)) {
        Some(w) if w.rig.motions.contains_key(&motion) => &w.rig,
        _ => &kh2.rig,
    };
    let clip = field
        .then(|| rig.field_motions.get(&motion))
        .flatten()
        .or_else(|| rig.motions.get(&motion))
        // (A banked glide the rig lacks: its plain glide.)
        .or_else(|| rig.motions.get(&m2::GLIDE).filter(|_| matches!(motion, m2::GLIDE_TURN_A | m2::GLIDE_TURN_B)))
        .or_else(|| rig.motions.get(&m2::IDLE))?;
    let frames = kh1_time.unwrap_or_else(|| f.since.map_or(0.0, |t| now.duration_since(t).as_secs_f32() * f.rate));
    let t = kh2_anim::key_time(clip, frames, looping);
    let mut pose = kh2_anim::evaluate(&rig.skeleton, clip, t, kh2_anim::Options::default()).bones;
    // The Keyblade's bones after Sora's (the lunge and the blend apply to both; split off below).
    let body_len = pose.len();
    let weapon = kh2_weapon(rig, motion, frames, looping, &pose);
    pose.extend(weapon);
    // A swing's lunge: KH1 moves him, so take KH2's root travel out of the pose.
    if matches!(play, Play::Swing { .. } | Play::Slot { .. }) {
        if let Some(root) = kh2.roots.get(&motion).and_then(|r| r.get((frames as usize).min(r.len().saturating_sub(1)))) {
            let un = Mat4::from_translation(-Vec3::from(*root));
            for m in pose.iter_mut() {
                *m = un * *m;
            }
        }
    }
    if let (Play::Slot { .. }, true) = (play, f.from.len() == pose.len()) {
        match (actor > 0x10000).then(|| unsafe { super::kh2slots::kh1_blend(actor) }).flatten() {
            Some(w) => {
                for (m, (fs, fr, ft)) in pose.iter_mut().zip(&f.from) {
                    let (ss, sr, st) = m.to_scale_rotation_translation();
                    *m = Mat4::from_scale_rotation_translation(fs.lerp(ss, w), fr.slerp(sr, w), ft.lerp(st, w));
                }
            }
            None => f.from.clear(),
        }
    }
    let blend_now = f.blend_start.map_or(-1.0, |b| (now.duration_since(b).as_secs_f32() / f.blend_secs.max(1e-3)).min(1.0));
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
    DRAW_ROUTE.store(2, Ordering::Relaxed);
    if let Ok(mut d) = DRAW_DETAIL.try_lock() {
        *d = Some(Drawn { route: "follow", slot, play: format!("{play:?}"), motion, frames, blend: blend_now, restart, at: None, age_ms: 0 });
    }
    Some(kh1)
}

/// The slot in KH1 Sora's file (`xa_ex_0010.mset`) of the loaded motion at `m`, by its
/// signature (length, channel, helper channel and helper counts), as `slot_of` finds it.
pub unsafe fn file_slot_of_motion(m: usize) -> Option<usize> {
    if m < 0x10000 || !super::crash::readable(m) || !super::crash::readable(m + 0x30) {
        return None;
    }
    let sig = (rd::<u32>(m + 4), rd::<u32>(m + 0x18), rd::<u32>(m + 0x20), rd::<u32>(m + 0x2C));
    SLOTS.get()?.0.get(&sig).copied()
}

/// The slot of the KH1 motion the instance plays.
unsafe fn slot_of(instance: usize) -> Option<usize> {
    let m = rd::<u64>(instance + 8) as usize;
    if m < 0x10000 {
        return None;
    }
    if let Some(s) = super::kh2slots::slot_of_motion(m) {
        return Some(s);
    }
    let sig = (rd::<u32>(m + 4), rd::<u32>(m + 0x18), rd::<u32>(m + 0x20), rd::<u32>(m + 0x2C));
    SLOTS.get()?.0.get(&sig).copied()
}

/// The model's bone count (`**instance`), if it reads.
pub(crate) unsafe fn bone_count(instance: usize) -> Option<u32> {
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
        log!(Animation, "Sora's model {instance:#x} drawn {d:.0} away from his actor: another copy (the pause menu's), left to KH1");
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
    if g.as_ref().map_or(true, |s| s.instance != instance) && !at_sora(instance, param_world) {
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
                log!(Animation, "Sora's bone table at {table:#x} did not read: {e}");
                return;
            }
        };
        log!(Animation, "Sora's instance {instance:#x} (bones at {table:#x})");
        let bind = kh2_anim::Pose::bind(&kh1).bones;
        *g = Some(Sora { instance, retarget: None, bind, calls: 0, checked: false, follow: Follow::default(), to_world: None, wrote_root: None, last_drawn: None, param_agree: 0, param_ok: PARAM_OK.load(Ordering::Relaxed), jumps: 0, parents: kh1.iter().map(|b| b.parent).collect(), jiggle: None, secondary: None, inverse: None, kh2_now: None });
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
                log!(Animation, "retarget ready: {} of {} KH1 bones follow KH2 (the rest hang as in KH1's battle idle: {})", r.mapped(), b.len(), rest.is_some());
                KH1_SIZE.store(r.hip_ratio().to_bits(), Ordering::Relaxed);
                s.retarget = Some(r);
                let sec = kh1_data::secondary::Secondary::new(&b, &kh2.rig.skeleton, kh2.rig.weapons.first().map(|w| w.skeleton.as_slice()));
                log!(Animation,
                    "KH2's own hair, hood, sleeves and keychain: {} spikes ({}), {} hood joints, {} sleeve chains, {} chain links",
                    sec.hair().len(),
                    sec.hair().iter().map(|l| format!("{}<-{}", l.kh1, l.kh2)).collect::<Vec<_>>().join(" "),
                    sec.hood().len(),
                    sec.sleeves().len(),
                    sec.chain().len()
                );
                s.secondary = Some(sec);
                let grip = kh1_data::retarget::GRIP_BONE;
                if let Some(r) = s.retarget.as_ref() {
                    let inv = kh1_data::inverse::Inverse::new(&b, &kh2.rig.skeleton, r, |i| i < grip && r.follows_one_part(i));
                    log!(Animation, "KH1's poses on KH2's skeleton: {} of {} KH2 bones turn with a measured part", inv.mapped(), kh2.rig.skeleton.len());
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
                log!(Animation, "placement: the skeleton pass's own world matrix matches; using it from now on");
            }
        } else {
            if s.param_agree > 0 || s.calls == 30 {
                log!(Animation, "placement: pass matrix {:?} vs from the bones {:?}", p.to_cols_array().map(|v| (v * 100.0).round() / 100.0), d.to_cols_array().map(|v| (v * 100.0).round() / 100.0));
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
        log!(Animation, "check: bone 3 from the locals is {err:.3} units from the game's (should be ~0)");
    }
    if let Ok(mut at) = SORA_AT.try_lock() {
        *at = Some(to_world.w_axis.truncate().to_array());
    }
    let orig_hands = (!ours).then(|| HANDS.map(|(b, _)| mat(world + b * 0x40)));
    let orig_blade = (!ours).then(|| (mat(world + 280 * 0x40), mat(world + 283 * 0x40)));
    // Our slots in KH1's motion table (`kh2slots`): the tables on his actor, our KH2 sounds.
    if MODE.load(Ordering::Relaxed) == 0 {
        super::kh2slots::frame(super::takeover::sora_actor());
    }
    let slot = slot_of(instance);
    // (Callers test KH1's slots: ours is reported as the one it stands in for.)
    SLOT_NOW.store(slot.map(|n| super::kh2slots::stands_for(n).unwrap_or(n)).unwrap_or(usize::MAX), Ordering::Relaxed);
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
                log!(Animation, "weapon {w:#x}: KH1's grip turned {:.2} rad from the Keyblade's rest (KH1's grip: axis {ax:.2?} angle {ang:.2}, offset {t:.1?})", r.grip_turn(&live).unwrap_or(-1.0));
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
        if let Some(o) = orig_hands {
            if let Ok(mut h) = HANDS_NOW.lock() {
                *h = Some(Hands { orig: o, drawn: o, at: Instant::now() });
            }
        }
        // Valor Form on KH1's own states (a baked slot of Valor's, `follow_baked` noted its KH2
        // frame): the left Keyblade from KH1's own drawing of the right one (`leftblade`).
        if let (true, Some((body, weapon))) = (super::form::native_now() && super::form::two_blades(), s.kh2_now.as_ref()) {
            let back = to_world.inverse();
            let kh1: Vec<Mat4> = (0..SORA_BONES as usize).map(|i| back * mat(world + i * 0x40)).collect();
            super::leftblade::note_pose(&kh1, &s.bind, body, weapon, &to_world, s.retarget.as_ref().map_or(1.0, |r| r.hip_ratio()));
        }
        // KH2's model (`model`) on KH1's own animation, as drawn.
        if let (true, Some(inv)) = (super::model::on(), s.inverse.as_ref()) {
            let back = to_world.inverse();
            let kh1: Vec<Mat4> = (0..SORA_BONES as usize).map(|i| back * mat(world + i * 0x40)).collect();
            super::model::note_pose(&inv.pose(&kh1), &[], &to_world, r_ratio(s.retarget.as_ref()));
        }
        return;
    };
    // (Our foot placement only with `foot_ik = on`; off, the default, it fades out and leaves the
    // legs as the clip has them: the tester, "lets actually have the foot iks off".)
    let kh2_feet = super::settings::get().foot_ik && MODE.load(Ordering::Relaxed) == 0 && super::takeover::active() && super::takeover::GROUNDED.load(Ordering::Relaxed);
    super::feet::place(BASE.load(Ordering::Relaxed), &mut pose, &s.parents, &to_world, kh2_feet);
    kh1_wrist(&mut pose, &s.parents, (!ours).then_some(world));
    keyblade_teeth_up(&mut pose, &s.parents);
    {
        // KH2's own animation of the hair, hood, sleeves and keychain (the user: "add the
        // animations for sora's hair and chain on each KH2 animation instead"; the sleeves: Mr
        // Bugs, "missing animations for the sleeves"), on every pose KH2 gave; the keychain only
        // on a Keyblade (not the dream weapons or the wooden sword).
        let mut driven = kh1_data::secondary::Parts::default();
        if let (Some((body, weapon)), Some(sec)) = (s.kh2_now.as_ref(), s.secondary.as_ref()) {
            let want = kh1_data::secondary::Parts { hair: true, hood: true, chain: !held_weapon().is_some_and(|w| (0x52..=0x55).contains(&w)), sleeves: true };
            let rep = sec.apply(&mut pose, body, (!weapon.is_empty()).then_some(weapon.as_slice()), want);
            driven = rep.parts;
            static LOGGED: AtomicU32 = AtomicU32::new(0);
            if LOGGED.fetch_add(1, Ordering::Relaxed) == 0 {
                log!(Animation, "hair, hood, sleeves and keychain from KH2's own animation: {:?} (asked {want:?})", rep.parts);
            }
        }
        // The swing (`kh1_data::jiggle`) for any chain KH2 didn't pose. One step per 60 Hz frame
        // of Sora's physics frames (`takeover::TICKS`: two a game frame at 30 fps), not by the
        // clock: frames come 13..22 ms apart, and a draw with no step left the chains at his last
        // place (the user: the animations stuttered). A gap of more than half a second (KH1's own
        // animation, a pause) starts it over.
        let frame = super::takeover::TICKS.load(Ordering::Relaxed);
        let (j, last) = s.jiggle.get_or_insert_with(|| (kh1_data::jiggle::Jiggle::new(kh1_data::jiggle::sora_chains()), frame));
        let steps = frame.saturating_sub(*last);
        if steps > 30 {
            j.reset();
        }
        *last = frame;
        j.apply_except(&mut pose, &s.parents, &to_world, steps.min(3) as usize, |b| driven.covers(b));
    }
    // A dream weapon whole, as KH1 holds it.
    dream_hold(&mut pose, &s.parents, (!ours).then_some(world));
    // Final Form's Keyblade floating where KH2's weapon motion has it.
    if let (Some((body, weapon)), true) = (s.kh2_now.as_ref(), super::form::in_final()) {
        super::leftblade::final_float(&mut pose, &s.bind, body, weapon, s.retarget.as_ref().map_or(1.0, |r| r.hip_ratio()));
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

/// KH1 Sora's size against KH2 Sora's (`Retarget::hip_ratio`, his hips' heights in the two bind
/// poses) once the retarget is made (bits of an `f32`; 0 before).
static KH1_SIZE: AtomicU32 = AtomicU32::new(0);

/// KH1 Sora's size against KH2 Sora's (KH2 distances to KH1 ones), once his retarget is made.
pub fn kh1_size() -> Option<f32> {
    let b = KH1_SIZE.load(Ordering::Relaxed);
    (b != 0).then(|| f32::from_bits(b)).filter(|k| k.is_finite() && *k > 0.0)
}

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
    if super::frameadv::age(hands.at).as_secs_f32() > 0.1 {
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
            log!(Animation, "the {bones}-bone model at {instance:#x} let go of the {} hand", HANDS[hand].1);
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
            log!(Animation, "the {bones}-bone model at {instance:#x} is held in Sora's {} hand ({dist:.1} units from it); it now follows the hand drawn", HANDS[hand].1);
        }
    }
    None
}

/// What KH1's own skeleton pass did to Sora's feet, read straight after the pass and before
/// anything of ours (the trace's `ik_ground_on`, `ik_lifted`, `foot_r_up`, `foot_l_up`,
/// `pose_path` columns; the user's test: does KH1 bend his legs onto a slope?). From the
/// decomp (`docs/kh1-anim-slots.md` §1.6): `instance + 0x14` bits 0-1 turn KH1's ground lift
/// on, and bit `0x200000` is set when a foot target was lifted onto the floor that pass.
#[derive(Clone, Copy, Default)]
pub struct FeetSeen {
    /// `instance + 0x14` after the last pass.
    pub flags: u32,
    /// The ankles' height (KH1's units, up = minus its y): right (19, the −x leg), left (45).
    pub ankle_up: [f32; 2],
    /// Passes since the last row: the plain one (`0x140391E20`), the blend / IK one
    /// (`0x140392710`).
    pub passes: [u32; 2],
    /// Read at least once since the last row.
    pub seen: bool,
}

static FEET: Mutex<FeetSeen> = Mutex::new(FeetSeen { flags: 0, ankle_up: [0.0; 2], passes: [0; 2], seen: false });
/// Sora's ankles: the bones his two-bone leg IK aims at its foot helpers (H9, H8).
const ANKLES: [usize; 2] = [19, 45];

/// After KH1's pass, for Sora's own instance only (not the pause menu's copy), while the trace
/// is on.
unsafe fn note_feet(instance: usize, via_ik: bool) {
    if instance == 0 || instance != SORA_INSTANCE.load(Ordering::Relaxed) || !super::trace::enabled() || !super::trace::wants(super::trace::cat::FEET) {
        return;
    }
    let world = ptr(rd::<u32>(instance + WORLD));
    if world == 0 || !super::crash::readable(world) || !super::crash::readable(world + SORA_BONES as usize * 0x40 - 1) {
        return;
    }
    let flags = rd::<u32>(instance + 0x14);
    let up = ANKLES.map(|b| -rd::<f32>(world + b * 0x40 + 0x34));
    if let Ok(mut g) = FEET.try_lock() {
        g.flags = flags;
        g.ankle_up = up;
        g.passes[via_ik as usize] += 1;
        g.seen = true;
    }
}

/// The feet as KH1's last pass left them, with the pass counts since the last call (cleared).
pub fn take_feet() -> FeetSeen {
    FEET.lock()
        .map(|mut g| {
            let old = *g;
            *g = FeetSeen { passes: [0; 2], seen: false, ..old };
            old
        })
        .unwrap_or_default()
}

type Pose = unsafe extern "system" fn(usize, usize, f32, usize);
/// The third argument is a float (XMM2; the hook audit of stage 49).
type PoseIk = unsafe extern "system" fn(usize, usize, f32);

/// Sora's own instance (not the pause menu's copy): `kh2dpx::posed` around his pass.
fn is_sora(instance: usize) -> bool {
    instance != 0 && instance == SORA_INSTANCE.load(Ordering::Relaxed)
}

unsafe extern "system" fn pose(world: usize, instance: usize, t: f32, motion: usize) {
    let f: Pose = std::mem::transmute(ORIG_POSE.load(Ordering::Acquire));
    if is_sora(instance) {
        super::kh2dpx::posed(false);
    }
    let moved = if instance != 0 && MODE.load(Ordering::Relaxed) <= 1 { held(instance, world) } else { None };
    let a = moved.map(|m| m.to_cols_array());
    let used = a.as_ref().map_or(world, |a| a.as_ptr() as usize);
    f(used, instance, t, motion);
    note_feet(instance, false);
    if instance != 0 && MODE.load(Ordering::Relaxed) <= 1 {
        note_small(instance, world);
    }
    if instance != 0 {
        check(instance, false, world);
    }
    if is_sora(instance) {
        super::kh2dpx::posed(true);
        // Valor's left Keyblade: its holder motion posed by this same pass (`leftblade`).
        super::leftblade::pose_holder(plain_pass, used, Some((motion, t)));
    }
}

/// KH1's own plain pose pass (the original, past our hook): `leftblade` poses its holder motion
/// with it.
unsafe fn plain_pass(world: usize, instance: usize, t: f32, motion: usize) {
    let f: Pose = std::mem::transmute(ORIG_POSE.load(Ordering::Acquire));
    f(world, instance, t, motion);
}

unsafe extern "system" fn pose_ik(world: usize, instance: usize, flags: f32) {
    LAST_T.store(flags.to_bits(), Ordering::Relaxed);
    let f: PoseIk = std::mem::transmute(ORIG_POSE_IK.load(Ordering::Acquire));
    super::glide::chain_source(instance);
    if is_sora(instance) {
        super::kh2dpx::posed(false);
    }
    let moved = if instance != 0 && MODE.load(Ordering::Relaxed) <= 1 { held(instance, world) } else { None };
    let a = moved.map(|m| m.to_cols_array());
    let used = a.as_ref().map_or(world, |a| a.as_ptr() as usize);
    f(used, instance, flags);
    note_feet(instance, true);
    if instance != 0 && MODE.load(Ordering::Relaxed) <= 1 {
        note_small(instance, world);
    }
    if instance != 0 {
        check(instance, true, world);
    }
    if is_sora(instance) {
        super::kh2dpx::posed(true);
        // (KH1's blend pass: the holder motion at his actor's motion and time.)
        super::leftblade::pose_holder(plain_pass, used, None);
    }
}

pub unsafe fn install() {
    let base = sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + POSE_RVA, &POSE_PROLOGUE, pose as *const () as usize, &ORIG_POSE, "skeleton pass");
    hook::install(base + POSE_IK_RVA, &POSE_IK_PROLOGUE, pose_ik as *const () as usize, &ORIG_POSE_IK, "skeleton pass with ground IK");
}
