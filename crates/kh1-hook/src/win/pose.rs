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

use super::{hook, log, sys};
use glam::{Mat4, Quat, Vec3};
use kh1_data::retarget::Retarget;
use kh2_data::sora::Rig;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

const SORA_BONES: u32 = 293;
const POSE_RVA: usize = 0x391E20;
const POSE_IK_RVA: usize = 0x392710;
const HANDLE_TABLE_RVA: usize = 0x2EE4730;
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
pub const MODES: [&str; 5] = [
    "KH2 animations follow Sora (movement, jump, glide, ground combo)",
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
static SLOTS: OnceLock<(HashMap<Signature, usize>, Vec<u32>)> = OnceLock::new();

/// KH2 Sora: skeleton and motions, and each action clip's root travel per frame (KH1 moves
/// him itself, so a swing's lunge is taken out of the pose).
struct Kh2 {
    rig: Arc<Rig>,
    roots: HashMap<u16, Vec<[f32; 3]>>,
    /// Walk and run (battle, field): ground speed in KH2 units per second, from the planted
    /// foot's travel against the hips.
    ground: HashMap<(u16, bool), f32>,
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
    /// The pass's own world matrix (its y and z flipped, axes unit length) agreed with the
    /// pose-derived placement for this many frames; trusted from 30 on.
    param_agree: u32,
    param_ok: bool,
    /// Pose-derived placements that jumped from the last (kept out unless they persist).
    jumps: u32,
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

/// KH2's blends: 8 frames between movement motions, 16 into a glide, none into a swing.
const BLEND_MOVE: f32 = 8.0 / 60.0;
const BLEND_GLIDE: f32 = 16.0 / 60.0;
/// Swings more than this apart start a new combo.
const COMBO_GAP: f32 = 1.2;

static SORA: Mutex<Option<Sora>> = Mutex::new(None);
static CLOCK: OnceLock<Instant> = OnceLock::new();

/// KH1's hands this frame (as the game posed them) and as we drew them.
#[derive(Clone, Copy)]
struct Hands {
    orig: [Mat4; 2],
    drawn: [Mat4; 2],
    at: Instant,
}
static HANDS_NOW: Mutex<Option<Hands>> = Mutex::new(None);

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
            Ok(Kh2 { rig, roots, ground })
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
            Ok((map, lengths))
        })();
        match slots {
            Ok(m) => {
                let _ = SLOTS.set(m);
            }
            Err(e) => log(&format!("KH1 Sora's motion table did not load: {e}")),
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

/// The model-space pose to draw for the current mode, if any.
fn wanted(s: &mut Sora, slot: Option<usize>, to_world: &Mat4) -> Option<Vec<Mat4>> {
    let mode = MODE.load(Ordering::Relaxed);
    match mode {
        0 => follow(s, slot, to_world),
        2 => Some(s.bind.clone()),
        3 | 4 => {
            let kh2 = KH2.get()?.as_ref().ok()?;
            let r = s.retarget.as_ref()?;
            let m = kh2.rig.motions.get(&(if mode == 3 { character_core::motion::IDLE } else { character_core::motion::RUN }))?;
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
    let play = slot.and_then(play_for);
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

unsafe fn check(instance: usize, via_ik: bool, param_world: usize) {
    let Some(bones) = bone_count(instance) else { return };
    note_model(bones, instance, via_ik);
    if bones != SORA_BONES {
        return;
    }
    let header = rd::<u64>(rd::<u64>(instance) as usize) as usize;
    let Ok(mut g) = SORA.try_lock() else { return };
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
        *g = Some(Sora { instance, retarget: None, bind, calls: 0, checked: false, follow: Follow::default(), to_world: None, wrote_root: None, param_agree: 0, param_ok: false, jumps: 0 });
        KH1_BONES.lock().map(|mut b| *b = kh1).ok();
    }
    let s = g.as_mut().unwrap();
    s.calls += 1;
    if s.retarget.is_none() {
        if let Some(Ok(kh2)) = KH2.get() {
            if let Ok(b) = KH1_BONES.lock() {
                let r = Retarget::new(&b, &kh2.rig.skeleton);
                log(&format!("retarget ready: {} of {} KH1 bones follow KH2", r.mapped(), b.len()));
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
    let orig_hands = (!ours).then(|| HANDS.map(|(b, _)| mat(world + b * 0x40)));
    let slot = slot_of(instance);
    let Some(pose) = wanted(s, slot, &to_world) else {
        s.wrote_root = None;
        s.to_world = Some(to_world);
        if let Some(o) = orig_hands {
            if let Ok(mut h) = HANDS_NOW.lock() {
                *h = Some(Hands { orig: o, drawn: o, at: Instant::now() });
            }
        }
        return;
    };
    for (i, m) in pose.iter().enumerate().take(SORA_BONES as usize) {
        std::ptr::write_unaligned((world + i * 0x40) as *mut [f32; 16], (to_world * *m).to_cols_array());
    }
    s.wrote_root = Some(rd(world));
    s.to_world = Some(to_world);
    let drawn = HANDS.map(|(b, _)| to_world * pose[b]);
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
type PoseIk = unsafe extern "system" fn(usize, usize, u32);

unsafe extern "system" fn pose(world: usize, instance: usize, t: f32, motion: usize) {
    let f: Pose = std::mem::transmute(ORIG_POSE.load(Ordering::Acquire));
    let moved = if instance != 0 && MODE.load(Ordering::Relaxed) == 0 { held(instance, world) } else { None };
    match moved {
        Some(m) => {
            let a = m.to_cols_array();
            f(a.as_ptr() as usize, instance, t, motion);
        }
        None => f(world, instance, t, motion),
    }
    if instance != 0 {
        check(instance, false, world);
    }
}

unsafe extern "system" fn pose_ik(world: usize, instance: usize, flags: u32) {
    let f: PoseIk = std::mem::transmute(ORIG_POSE_IK.load(Ordering::Acquire));
    let moved = if instance != 0 && MODE.load(Ordering::Relaxed) == 0 { held(instance, world) } else { None };
    match moved {
        Some(m) => {
            let a = m.to_cols_array();
            f(a.as_ptr() as usize, instance, flags);
        }
        None => f(world, instance, flags),
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
