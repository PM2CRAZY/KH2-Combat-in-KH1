//! Stage 1 probe: KH1's skeleton pass, and a test of overriding Sora's pose.
//!
//! KH1 poses a character in `0x140391E20` (thiscall-like Win64: `(world matrix, instance,
//! time f32, motion)`: samples the motion into per-bone rotation / translation arrays, then
//! builds the bone matrices in `0x140390260`) or, for characters with ground IK, in
//! `0x140392710` (`(world matrix, instance, flags)`). The instance (`*u64`):
//! - `+0` points at a slot that points at the model header: `bone_count u16`, then a handle
//!   to the 0x30-byte bone table (`kh1_data::mdls`).
//! - `+0x1B0`, `+0x1B4`, `+0x1B8`, `+0x1BC`, `+0x1C0`: handles to per-bone 0x40-byte matrix
//!   arrays; `+0x1C4`, `+0x1C8`: per-bone 0x10-byte translation and rotation.
//! Handles are the game's 32-bit addresses: `table[(h & 0x7FFFFFFF) >> 25] | (h & 0x1FFFFFF)`,
//! the table at `exe + 0x2EE4730` (`0x14038B230`).
//!
//! This stage logs those arrays for Sora (the 293-bone model) and, with Insert, writes his
//! bind pose (a T-pose) into one of them, cycling which, to learn which array the game draws.

use super::{hook, log, sys};
use glam::Mat4;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

const SORA_BONES: u32 = 293;
const POSE_RVA: usize = 0x391E20;
const POSE_IK_RVA: usize = 0x392710;
const HANDLE_TABLE_RVA: usize = 0x2EE4730;
const POSE_PROLOGUE: [u8; 7] = [0x4C, 0x8B, 0xDC, 0x53, 0x56, 0x41, 0x54];
const POSE_IK_PROLOGUE: [u8; 9] = [0x4C, 0x8B, 0xDC, 0x55, 0x53, 0x41, 0x54, 0x41, 0x57];
/// The matrix arrays tried by the override, by instance offset.
const ARRAYS: [usize; 5] = [0x1B0, 0x1B4, 0x1B8, 0x1BC, 0x1C0];

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_POSE: AtomicUsize = AtomicUsize::new(0);
static ORIG_POSE_IK: AtomicUsize = AtomicUsize::new(0);
/// 0 = off; 1..=5 = write the T-pose into `ARRAYS[n-1]`.
pub static MODE: AtomicU32 = AtomicU32::new(0);

struct Seen {
    instance: usize,
    bind: Vec<Mat4>,
    logged: u32,
    calls: u64,
    via_ik: bool,
}

static SORA: Mutex<Option<Seen>> = Mutex::new(None);

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

/// Every model size posed (the first 40 kinds), logged once each: what the hooks see.
fn note_model(bones: u32, instance: usize, via_ik: bool) {
    static SEEN: Mutex<Vec<(u32, bool)>> = Mutex::new(Vec::new());
    let Ok(mut v) = SEEN.try_lock() else { return };
    if v.len() < 40 && !v.contains(&(bones, via_ik)) {
        v.push((bones, via_ik));
        log(&format!("posed: a {bones}-bone model (instance {instance:#x}) by {}", if via_ik { "the ground-IK pass" } else { "the plain pass" }));
    }
}

/// Sora's model (293 bones): his bind pose, built once from the game's own bone table.
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
    let fresh = g.as_ref().map_or(true, |s| s.instance != instance);
    if fresh {
        let table = ptr(rd::<u32>(header + 4));
        if table == 0 {
            return;
        }
        let bytes = std::slice::from_raw_parts(table as *const u8, SORA_BONES as usize * 0x30);
        let bind = match kh1_data::mdls::parse_bones(bytes, 0, SORA_BONES as usize) {
            Ok(b) => kh2_anim::Pose::bind(&b).bones,
            Err(e) => {
                log(&format!("Sora's bone table at {table:#x} did not read: {e}"));
                return;
            }
        };
        log(&format!("Sora's instance {instance:#x} (model header {header:#x}, bones at {table:#x}), posed by {}", if via_ik { "0x140392710 (ground IK)" } else { "0x140391E20" }));
        *g = Some(Seen { instance, bind, logged: 0, calls: 0, via_ik });
    }
    let s = g.as_mut().unwrap();
    s.calls += 1;
    s.via_ik = via_ik;
    if s.logged < 4 && (s.calls == 30 || s.calls % 600 == 0) {
        s.logged += 1;
        let m = |p: usize| -> String {
            if p == 0 {
                return "null".into();
            }
            let f: [f32; 16] = rd(p);
            format!("[{:.2} {:.2} {:.2} | {:.2} {:.2} {:.2} | {:.2} {:.2} {:.2} | t {:.2} {:.2} {:.2} w {:.2}]", f[0], f[1], f[2], f[4], f[5], f[6], f[8], f[9], f[10], f[12], f[13], f[14], f[15])
        };
        for off in ARRAYS {
            let a = ptr(rd::<u32>(instance + off));
            log(&format!("  +{off:#x} -> {a:#x}: bone 0 {}; bone 3 {}", m(a), if a != 0 { m(a + 3 * 0x40) } else { "-".into() }));
        }
        for off in [0x1C4usize, 0x1C8] {
            let a = ptr(rd::<u32>(instance + off));
            if a != 0 {
                let v: [f32; 8] = rd(a);
                log(&format!("  +{off:#x} -> {a:#x}: {v:.3?}"));
            }
        }
    }
    let mode = MODE.load(Ordering::Relaxed) as usize;
    if (1..=ARRAYS.len()).contains(&mode) {
        let a = ptr(rd::<u32>(instance + ARRAYS[mode - 1]));
        // Only arrays of affine matrices (`+0x1B0` holds the sampled scale / rotation /
        // translation channels, not matrices).
        let affine = |p: usize| {
            let f: [f32; 16] = rd(p);
            f[3] == 0.0 && f[7] == 0.0 && f[11] == 0.0 && (f[15] - 1.0).abs() < 1e-4
        };
        if a != 0 && affine(a) && affine(a + 3 * 0x40) {
            // Keep the root where the game put it; everything else in the bind pose from there.
            let g0 = Mat4::from_cols_array(&rd::<[f32; 16]>(a));
            let to = g0 * s.bind[0].inverse();
            for (i, b) in s.bind.iter().enumerate() {
                let m = to * *b;
                std::ptr::write_unaligned((a + i * 0x40) as *mut [f32; 16], m.to_cols_array());
            }
        }
    }
}

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
