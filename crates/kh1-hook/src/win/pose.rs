//! Stage 3: KH2 Sora's motions drawn on KH1 Sora, following what KH1 has him do.
//!
//! KH1 poses a character in `0x140391E20` (`(world matrix, instance, time f32, motion)`:
//! samples the motion, then builds the bone matrices in `0x140390260`) or, for characters with
//! ground IK, `0x140392710` (`(world matrix, instance, flags)`). The instance (`*u64`):
//! - `+0` points at a slot that points at the model header: `bone_count u16`, then a handle
//!   to the 0x30-byte bone table (`kh1_data::mdls`).
//! - `+0x1B0`: per bone (0x40 bytes) the sampled channels: scale `+0`, accumulated scale
//!   `+0x10`, Euler rotation `+0x20`, translation `+0x30`. Kept between frames (only animated
//!   channels are rewritten), so never written by us.
//! - `+0x1B4`: per bone, the world matrix the game draws (D3D row-vector layout, i.e. glam's
//!   column-major memory; the world is y-down). The user's test (stage 2): writing a pose here
//!   moves Sora's mesh.
//! - `+0x1B8`: per bone, its matrix relative to its parent (bone 0: the root's own, which the
//!   motion drives: hips height and a yaw, not the bind's rotation).
//! Handles are the game's 32-bit addresses: `table[(h & 0x7FFFFFFF) >> 25] | (h & 0x1FFFFFF)`,
//! the table at `exe + 0x2EE4730` (`0x14038B230`).
//!
//! The motion playing is at `instance + 8` (stored by the skeleton pass); its counts name the
//! slot of KH1 Sora's motion table (`kh1_data::mset`), and standing, walking, running and
//! jumping slots get KH2's motion for the same thing (`follow`); every other slot (attacks,
//! magic, reactions, cutscenes) keeps KH1's own animation for now.
//!
//! Model space → world is taken from the game each frame: the root's world matrix times the
//! inverse of its own matrix (`+0x1B4[0] * +0x1B8[0]⁻¹`). Our model-space pose (KH1's bind for
//! the T-pose test, or a KH2 motion through `kh1_data::retarget`) then goes into `+0x1B4`.
//! Written after the game's pass, so the Keyblade (attached inside the pass) still follows
//! KH1's hand for now.

use super::{hook, log, sys};
use glam::Mat4;
use kh1_data::retarget::Retarget;
use kh2_data::sora::Rig;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

const SORA_BONES: u32 = 293;
const POSE_RVA: usize = 0x391E20;
const POSE_IK_RVA: usize = 0x392710;
const HANDLE_TABLE_RVA: usize = 0x2EE4730;
const POSE_PROLOGUE: [u8; 7] = [0x4C, 0x8B, 0xDC, 0x53, 0x56, 0x41, 0x54];
const POSE_IK_PROLOGUE: [u8; 9] = [0x4C, 0x8B, 0xDC, 0x55, 0x53, 0x41, 0x54, 0x41, 0x57];
const CHANNELS: usize = 0x1B0;
const WORLD: usize = 0x1B4;
const LOCAL: usize = 0x1B8;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_POSE: AtomicUsize = AtomicUsize::new(0);
static ORIG_POSE_IK: AtomicUsize = AtomicUsize::new(0);

/// What Insert cycles through (starts on the first).
pub const MODES: [&str; 5] = [
    "KH2 animations follow Sora (standing, walking, running, jumping)",
    "off (KH1's own animation)",
    "KH1 bind pose (T-pose), upright",
    "KH2 idle (battle stance)",
    "KH2 run",
];
pub static MODE: AtomicU32 = AtomicU32::new(0);

/// KH1 Sora's motion slots (`xa_ex_0010.mset` table order) by what they look like, from
/// their IK targets (`kh1_data::mset`; hips height, foot lifts, looping): to be confirmed in
/// play (every slot change is logged with his speed).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Stand,
    Walk,
    Run,
    Air,
    Land,
}

fn kind_of(slot: usize) -> Option<Kind> {
    Some(match slot {
        0 | 1 => Kind::Stand,
        2..=4 => Kind::Walk,
        5..=7 => Kind::Run,
        8..=10 => Kind::Air,
        11 => Kind::Land,
        _ => return None,
    })
}

/// A KH1 motion's counts, which tell Sora's motions apart (two pairs of unrelated slots
/// share theirs): frames, channels, helper channels, helpers.
type Signature = (u32, u32, u32, u32);
static SLOTS: OnceLock<HashMap<Signature, usize>> = OnceLock::new();

/// KH2 Sora's skeleton and motions, from the KH2 install in the same folder (loaded on a
/// thread at start).
static KH2: OnceLock<Result<Arc<Rig>, String>> = OnceLock::new();

struct Sora {
    instance: usize,
    retarget: Option<Retarget>,
    bind: Vec<Mat4>,
    calls: u64,
    checked: bool,
    follow: Follow,
}

/// Which KH2 motion follows KH1's, and how far into it.
#[derive(Default)]
struct Follow {
    slot: Option<usize>,
    kind: Option<Kind>,
    motion: u16,
    since: Option<Instant>,
    /// Last KH2 pose (model space), and the pose blended from on a change.
    last: Vec<Mat4>,
    from: Vec<(glam::Vec3, glam::Quat, glam::Vec3)>,
    blend_start: Option<Instant>,
    /// Movement measured from the model origin (world units per second), smoothed.
    at: Option<(glam::Vec3, Instant)>,
    speed: f32,
    climb: f32,
    logged: u32,
}

/// Blend into a new KH2 motion over 8 frames, as KH2 does between movement motions.
const BLEND_SECS: f32 = 8.0 / 60.0;

static SORA: Mutex<Option<Sora>> = Mutex::new(None);
static CLOCK: OnceLock<Instant> = OnceLock::new();

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
/// Manager backed up, if any, so a randomizer's edits don't leak in).
pub fn load_kh2() {
    std::thread::spawn(|| {
        let t = Instant::now();
        let r = (|| -> Result<Arc<Rig>, String> {
            let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
            let install = kh2_data::Install::locate(&exe).map_err(|e| format!("KH2 install: {e}"))?;
            let source = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
            let sora = install.load_sora(source).map_err(|e| format!("KH2 Sora: {e}"))?;
            let rig = sora.rig.ok_or_else(|| sora.rig_error.unwrap_or_else(|| "no rig".into()))?;
            log(&format!("KH2 Sora loaded ({source:?} files): {} bones, {} motions, in {:.1}s", rig.skeleton.len(), rig.motions.len(), t.elapsed().as_secs_f32()));
            Ok(rig)
        })();
        if let Err(e) = &r {
            log(&format!("KH2 Sora did not load: {e}"));
        }
        let _ = KH2.set(r);
        // KH1 Sora's own motion table, as the game loads it (to name the slot playing).
        let slots = (|| -> Result<HashMap<Signature, usize>, String> {
            let install = kh1_data::Kh1Install::locate(super::exe_dir()).map_err(|e| e.to_string())?;
            let bytes = install.read(kh1_data::Source::Live, "xa_ex_0010.mset").map_err(|e| e.to_string())?;
            let motions = kh1_data::mset::parse(&bytes).map_err(|e| e.to_string())?;
            let mut map = HashMap::new();
            for (i, m) in motions.iter().enumerate() {
                if let Some(m) = m {
                    map.entry((m.frames, m.channels.len() as u32, m.helper_channels, m.helpers)).or_insert(i);
                }
            }
            log(&format!("KH1 Sora's motion table: {} slots", motions.len()));
            Ok(map)
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
            let rig = KH2.get()?.as_ref().ok()?;
            let r = s.retarget.as_ref()?;
            let m = rig.motions.get(&(if mode == 3 { character_core::motion::IDLE } else { character_core::motion::RUN }))?;
            let frames = CLOCK.get_or_init(Instant::now).elapsed().as_secs_f32() * 60.0;
            let t = kh2_anim::key_time(m, frames, true);
            let kh2 = kh2_anim::evaluate(&rig.skeleton, m, t, kh2_anim::Options::default()).bones;
            Some(r.pose(&kh2))
        }
        _ => None,
    }
}

fn decompose(v: &[Mat4]) -> Vec<(glam::Vec3, glam::Quat, glam::Vec3)> {
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
        if dt > 1e-3 && dt < 0.25 {
            let d = pos - p;
            let climb = d.dot(up) / dt;
            let speed = (d - up * d.dot(up)).length() / dt;
            f.speed += (speed - f.speed) * 0.2;
            f.climb += (climb - f.climb) * 0.3;
        }
    }
    f.at = Some((pos, now));
    let kind = slot.and_then(kind_of);
    if slot != f.slot {
        if f.logged < 600 {
            f.logged += 1;
            log(&format!("KH1 slot {} ({}) at speed {:.0}/s, climb {:.0}/s", slot.map_or("?".into(), |n| n.to_string()), kind.map_or("KH1's own animation".into(), |k| format!("{k:?}")), f.speed, f.climb));
        }
        f.slot = slot;
    }
    let Some(kind) = kind else {
        f.kind = None;
        return None;
    };
    let rig = KH2.get()?.as_ref().ok()?.clone();
    let r = s.retarget.as_ref()?;
    let f = &mut s.follow;
    let motion = match kind {
        Kind::Stand => m2::IDLE,
        Kind::Walk => m2::WALK,
        Kind::Run => m2::RUN,
        // Rising plays the jump, then the fall loop.
        Kind::Air => {
            if f.climb > 20.0 && f.motion != m2::FALL {
                m2::JUMP
            } else {
                m2::FALL
            }
        }
        Kind::Land => m2::LAND,
    };
    if f.kind != Some(kind) || f.motion != motion {
        if !f.last.is_empty() {
            f.from = decompose(&f.last);
            f.blend_start = Some(now);
        }
        f.kind = Some(kind);
        f.motion = motion;
        f.since = Some(now);
    }
    let clip = rig.motions.get(&motion).or_else(|| rig.motions.get(&m2::IDLE))?;
    let looping = matches!(motion, m2::IDLE | m2::WALK | m2::RUN | m2::FALL);
    let frames = f.since.map_or(0.0, |t| now.duration_since(t).as_secs_f32() * 60.0);
    let t = kh2_anim::key_time(clip, frames, looping);
    let mut kh2 = kh2_anim::evaluate(&rig.skeleton, clip, t, kh2_anim::Options::default()).bones;
    if let Some(b) = f.blend_start {
        let w = now.duration_since(b).as_secs_f32() / BLEND_SECS;
        if w >= 1.0 || f.from.len() != kh2.len() {
            f.blend_start = None;
        } else {
            for (m, (fs, fr, ft)) in kh2.iter_mut().zip(&f.from) {
                let (ss, sr, st) = m.to_scale_rotation_translation();
                *m = Mat4::from_scale_rotation_translation(fs.lerp(ss, w), fr.slerp(sr, w), ft.lerp(st, w));
            }
        }
    }
    f.last = kh2.clone();
    Some(r.pose(&kh2))
}

/// The slot of the KH1 motion the instance plays (`instance + 8`, set by the skeleton pass).
unsafe fn slot_of(instance: usize) -> Option<usize> {
    let m = rd::<u64>(instance + 8) as usize;
    if m < 0x10000 {
        return None;
    }
    let sig = (rd::<u32>(m + 4), rd::<u32>(m + 0x18), rd::<u32>(m + 0x20), rd::<u32>(m + 0x2C));
    SLOTS.get()?.get(&sig).copied()
}

unsafe fn check(instance: usize, via_ik: bool) {
    // `*instance` points at the model's slot, which points at the model header (bone count
    // u16, then the bone-table handle): `0x140390260` reads `**(short **)*instance`.
    let slot = rd::<u64>(instance) as usize;
    if slot < 0x10000 {
        return;
    }
    let header = rd::<u64>(slot) as usize;
    if header < 0x10000 {
        return;
    }
    let bones = rd::<u16>(header) as u32;
    note_model(bones, instance, via_ik);
    if bones != SORA_BONES {
        return;
    }
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
        *g = Some(Sora { instance, retarget: None, bind, calls: 0, checked: false, follow: Follow::default() });
        // Kept for the retarget, built once KH2 Sora is loaded.
        KH1_BONES.lock().map(|mut b| *b = kh1).ok();
    }
    let s = g.as_mut().unwrap();
    s.calls += 1;
    if s.retarget.is_none() {
        if let Some(Ok(rig)) = KH2.get() {
            if let Ok(b) = KH1_BONES.lock() {
                let r = Retarget::new(&b, &rig.skeleton);
                log(&format!("retarget ready: {} of {} KH1 bones follow a KH2 bone", r.mapped(), b.len()));
                s.retarget = Some(r);
            }
        }
    }
    let (world, local) = (ptr(rd::<u32>(instance + WORLD)), ptr(rd::<u32>(instance + LOCAL)));
    if world == 0 || local == 0 {
        return;
    }
    let g0 = mat(world);
    let l0 = mat(local);
    if !affine(&g0) || !affine(&l0) || l0.determinant().abs() < 1e-6 {
        return;
    }
    let to_world = g0 * l0.inverse();
    if !s.checked && s.calls >= 30 {
        // Is `+0x1B8` really each bone's matrix relative to its parent? Then the game's world
        // matrix of bone 3 is bone 0's times the locals of 1, 2 and 3.
        s.checked = true;
        let chain = g0 * mat(local + 0x40) * mat(local + 0x80) * mat(local + 0xC0);
        let g3 = mat(world + 0xC0);
        let err = (chain.w_axis - g3.w_axis).length();
        log(&format!(
            "check: bone 3 from the locals is {err:.3} units from the game's (should be ~0); model->world {:?}; channels at {:#x}",
            to_world.to_cols_array().map(|v| (v * 100.0).round() / 100.0),
            ptr(rd::<u32>(instance + CHANNELS))
        ));
    }
    let slot = slot_of(instance);
    let Some(pose) = wanted(s, slot, &to_world) else { return };
    for (i, m) in pose.iter().enumerate().take(SORA_BONES as usize) {
        std::ptr::write_unaligned((world + i * 0x40) as *mut [f32; 16], (to_world * *m).to_cols_array());
    }
}

static KH1_BONES: Mutex<Vec<kh2_formats::anim::Bone>> = Mutex::new(Vec::new());

type Pose = unsafe extern "system" fn(usize, usize, f32, usize);
type PoseIk = unsafe extern "system" fn(usize, usize, u32);

unsafe extern "system" fn pose(world: usize, instance: usize, t: f32, motion: usize) {
    let f: Pose = std::mem::transmute(ORIG_POSE.load(Ordering::Acquire));
    f(world, instance, t, motion);
    if instance != 0 {
        check(instance, false);
    }
}

unsafe extern "system" fn pose_ik(world: usize, instance: usize, flags: u32) {
    let f: PoseIk = std::mem::transmute(ORIG_POSE_IK.load(Ordering::Acquire));
    f(world, instance, flags);
    if instance != 0 {
        check(instance, true);
    }
}

pub unsafe fn install() {
    let base = sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + POSE_RVA, &POSE_PROLOGUE, pose as *const () as usize, &ORIG_POSE, "skeleton pass");
    hook::install(base + POSE_IK_RVA, &POSE_IK_PROLOGUE, pose_ik as *const () as usize, &ORIG_POSE_IK, "skeleton pass with ground IK");
}
