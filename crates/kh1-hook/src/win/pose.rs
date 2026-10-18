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
            Ok(Kh2 { rig, character: sora.character.clone(), battle: Arc::new(sora.combat.battle.clone()), magi: sora.magi.clone(), wisdom, roots, ground })
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
    match mode {
        0 => {
            // KH1's area script walks him (zone entries and exits): KH1's own animation.
            if super::takeover::SCRIPTED.load(Ordering::Relaxed) {
                s.follow.slot = slot;
                return None;
            }
            // KH2's last pose: recent, or the last frame of his was KH2's (a pause stops the
            // game's frames, not the clock: after one, the old pose went stale and KH1's slot was
            // drawn until KH2's next frame; the user: paused mid KH2 move, resumed in KH1's).
            let kh2_frame = super::takeover::kh2_frame();
            let fresh = super::takeover::POSE.try_lock().ok().and_then(|p| p.as_ref().filter(|(_, t)| kh2_frame || t.elapsed().as_secs_f32() < 0.15).map(|(b, _)| b.clone()));
            match (fresh, s.retarget.as_ref()) {
                (Some(b), Some(r)) => {
                    s.follow.slot = slot;
                    note_kh2(&b, to_world);
                    Some(r.pose_grip(&b, super::takeover::GRIP.load(Ordering::Relaxed)))
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
            Some(r.pose(&pose))
        }
        _ => None,
    }
}

fn decompose(v: &[Mat4]) -> Vec<(Vec3, Quat, Vec3)> {
    v.iter().map(|m| m.to_scale_rotation_translation()).collect()
}

/// KH2's motion for what KH1's slot shows Sora doing; `None` leaves KH1's own animation.
fn follow(s: &mut Sora, slot: Option<usize>, to_world: &Mat4) -> Option<Vec<Mat4>> {
    follow_play(s, slot, to_world, None)
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
        if !f.last.is_empty() && blend > 0.0 {
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
    let clip = field
        .then(|| kh2.rig.field_motions.get(&motion))
        .flatten()
        .or_else(|| kh2.rig.motions.get(&motion))
        .or_else(|| kh2.rig.motions.get(&m2::IDLE))?;
    let frames = f.since.map_or(0.0, |t| now.duration_since(t).as_secs_f32() * f.rate);
    let t = kh2_anim::key_time(clip, frames, looping);
    let mut pose = kh2_anim::evaluate(&kh2.rig.skeleton, clip, t, kh2_anim::Options::default()).bones;
    // A swing's lunge: KH1 moves him, so take KH2's root travel out of the pose.
    if matches!(play, Play::Swing { .. }) {
        if let Some(root) = kh2.roots.get(&motion).and_then(|r| r.get((frames as usize).min(r.len().saturating_sub(1)))) {
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
    note_kh2(&pose, to_world);
    Some(r.pose(&pose))
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
    if g.as_ref().map_or(true, |s| s.instance != instance) && !at_sora(instance, param_world) {
        // Another copy of Sora's model away from his actor: the pause menu's (the user: it
        // walked slowly in KH2's walk and showed a frame of KH2's pose). KH1's own animation.
        MENU_SORA.lock().map(|mut m| *m = Some(Instant::now())).ok();
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
        *g = Some(Sora { instance, retarget: None, bind, calls: 0, checked: false, follow: Follow::default(), to_world: None, wrote_root: None, last_drawn: None, param_agree: 0, param_ok: PARAM_OK.load(Ordering::Relaxed), jumps: 0, parents: kh1.iter().map(|b| b.parent).collect(), jiggle: None });
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
            } else {
                s.last_drawn = None;
            }
        }
        if let Some(o) = orig_hands {
            if let Ok(mut h) = HANDS_NOW.lock() {
                *h = Some(Hands { orig: o, drawn: o, at: Instant::now() });
            }
        }
        return;
    };
    let kh2_feet = MODE.load(Ordering::Relaxed) == 0 && super::takeover::active() && super::takeover::GROUNDED.load(Ordering::Relaxed);
    super::feet::place(BASE.load(Ordering::Relaxed), &mut pose, &s.parents, &to_world, kh2_feet);
    kh1_wrist(&mut pose, &s.parents, (!ours).then_some(world));
    keyblade_teeth_up(&mut pose, &s.parents);
    if super::settings::get().physics {
        // One step per game frame (Sora's physics frames), not by the clock: frames come 13..22
        // ms apart, and a draw with no step left the chains at his last place (the user: the
        // animations stuttered). A gap of more than half a second (KH1's own animation, a
        // pause) starts the swing over.
        let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
        let (j, last) = s.jiggle.get_or_insert_with(|| (kh1_data::jiggle::Jiggle::new(kh1_data::jiggle::sora_chains()), frame));
        let steps = frame.saturating_sub(*last);
        if steps > 30 {
            j.reset();
        }
        *last = frame;
        j.apply(&mut pose, &s.parents, &to_world, steps.min(3) as usize);
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
    if let Ok(mut h) = HANDS_NOW.lock() {
        let orig = orig_hands.or(h.map(|h| h.orig)).unwrap_or(drawn);
        *h = Some(Hands { orig, drawn, at: Instant::now() });
    }
}

static KH1_BONES: Mutex<Vec<kh2_formats::anim::Bone>> = Mutex::new(Vec::new());

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
        check(instance, false, world);
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
        check(instance, true, world);
    }
}

pub unsafe fn install() {
    let base = sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + POSE_RVA, &POSE_PROLOGUE, pose as *const () as usize, &ORIG_POSE, "skeleton pass");
    hook::install(base + POSE_IK_RVA, &POSE_IK_PROLOGUE, pose_ik as *const () as usize, &ORIG_POSE_IK, "skeleton pass with ground IK");
}
