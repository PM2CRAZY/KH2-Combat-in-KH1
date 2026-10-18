//! KH2's field camera in KH1 (`kh2_camera`, the user: KH2's camera, switchable with KH1's in
//! the Config menu). `docs/kh1-camera.md` has KH1's side.
//!
//! KH1's field camera task (`0x1401E3C30`, priority 100000) runs the current camera mode from a
//! table of update functions at `0x140507AD0` (16 bytes a mode). Mode 1 is the ordinary follow
//! camera and mode 3 the lock-on camera. Their entries are swapped for ours: each frame KH2's
//! camera places the eye and look point, written to KH1's internal pair `0x142538A30` /
//! `0x142538A40` with KH1's own yaw / pitch / distance kept in step, and KH1 does the rest as
//! usual (blends into and out of its other cameras, events, area cameras, the View build). With
//! the KH1 camera chosen, ours hands each frame to KH1's original, which carries on from where
//! KH2's camera was (no jump: its state was kept coherent).
//!
//! World: KH1's is y-down; `kh2_camera` is y-up, and here also divided by `camera_scale` (KH2's
//! distances suit a slightly bigger Sora).

use super::addr::Rva;
use super::{log, pose};
use glam::{Vec2, Vec3};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

const MODES_RVA: Rva = Rva(0x507AD0);
const SORA_RVA: Rva = Rva(0x2D37C80);
const SUBJECT_RVA: Rva = Rva(0x2538A10);
/// Mode-entry flags: bit 1 = this mode's first frame (bits 0 / 2: from mode 0 / a snapshot).
const ENTRY_FLAGS_RVA: Rva = Rva(0x25389E8);
const EYE_RVA: Rva = Rva(0x2538A30);
const TARGET_RVA: Rva = Rva(0x2538A40);
const PITCH_RVA: Rva = Rva(0x2538A50);
const YAW_RVA: Rva = Rva(0x2538A54);
const DIST_RVA: Rva = Rva(0x2538A5C);
/// KH1's camera timers and stick velocities that would fight ours (zeroed each frame).
const KH1_MOTION_RVAS: [Rva; 9] = [Rva(0x2538850), Rva(0x2538854), Rva(0x2538AB4), Rva(0x2538AA0), Rva(0x2538844), Rva(0x2538848), Rva(0x253884C), Rva(0x2538B50), Rva(0x2538B54)];
/// Frame step (2 at 30 frames a second, 1 at 60): KH2's camera counts 60 Hz frames.
const STEP_RVA: Rva = Rva(0x23405DC);
const RIGHT_STICK_RVA: Rva = Rva(0x298D4D0);
const LEFT_STICK_RVA: Rva = Rva(0x298D4E0);
/// Stick dead zone for KH2's camera (of full tilt).
const STICK_DEAD: f32 = 0.15;
/// Logical buttons: `+8` pressed this frame. R3 `0x200`, Select `0x8000`.
const BUTTONS_RVA: Rva = Rva(0x298D4A0);
const R3: u32 = 0x200;
const SELECT: u32 = 0x8000;
/// KH1's config block (`+8`: field camera 0 Auto / 1 Manual).
const CONFIG_PTR_RVA: Rva = Rva(0x2E1FE70);
/// The camera left/right option's sign (+1 normal).
const TURN_SIGN_RVA: Rva = Rva(0x507AAC);
const FLAGS_RVA: Rva = Rva(0x2867CD8);
const FLAGS2_RVA: Rva = Rva(0x2867CDC);
/// Mode 1's end: KH1's movement-state cameras (vines, ledges...).
const STATE_CAMERAS_RVA: Rva = Rva(0x1E3320);
/// Mode switch `(mode, blend)`.
const SET_MODE_RVA: Rva = Rva(0x1E3200);
/// KH1's camera ray (background and objects, collision locked): `0x140191640 (ray*)`.
const RAY_RVA: Rva = Rva(0x191640);
const FACE_NORMAL_RVA: Rva = Rva(0x18FC60);
const LOCK_TARGET_RVA: Rva = Rva(0x2DB3F18);
const LOCK_POINT_RVA: Rva = Rva(0x2AE330);
/// FOV setter (degrees) and KH1's default field FOV (mode 1's entry sets it; area cameras
/// change it).
const SET_FOV_RVA: Rva = Rva(0x28A960);
const DEFAULT_FOV_RVA: Rva = Rva(0x3EAC84);

/// KH2's camera is on (the ini's `camera`, the Config menu's row).
pub static KH2: AtomicBool = AtomicBool::new(true);
static ORIG_FOLLOW: AtomicUsize = AtomicUsize::new(0);
static ORIG_LOCK: AtomicUsize = AtomicUsize::new(0);
static BASE: AtomicUsize = AtomicUsize::new(0);
static LAST_MS: AtomicU64 = AtomicU64::new(0);
static LOGGED: AtomicU32 = AtomicU32::new(0);
static CAM: Mutex<Option<kh2_camera::Camera>> = Mutex::new(None);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn now_ms() -> u64 {
    static T0: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    T0.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64 + 1
}

/// Swaps KH1's follow and lock-on camera modes for ours.
pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    KH2.store(super::settings::get().kh2_camera, Ordering::Relaxed);
    let follow = base + MODES_RVA + 0x10;
    let lock = base + MODES_RVA + 0x30;
    let (f, l) = (rd::<u64>(follow) as usize, rd::<u64>(lock) as usize);
    // The entries should point at KH1's own mode 1 / mode 3 (`0x1401DF7C0` / `0x1401E04F0`).
    if f != base + Rva(0x1DF7C0) || l != base + Rva(0x1E04F0) {
        log(&format!("KH2 camera: KH1's camera modes aren't where expected ({f:#x}, {l:#x}): not installed"));
        return;
    }
    ORIG_FOLLOW.store(f, Ordering::Release);
    ORIG_LOCK.store(l, Ordering::Release);
    let mut old = 0u32;
    super::sys::VirtualProtect(follow as *mut _, 0x30, 0x04, &mut old);
    wr(follow, follow_mode as *const () as u64);
    wr(lock, lock_mode as *const () as u64);
    log(&format!("KH2 camera: KH1's follow and lock-on cameras swapped for KH2's ({})", if KH2.load(Ordering::Relaxed) { "on" } else { "KH1's chosen" }));
}

/// KH2's camera this frame: chosen, and no cutscene (the user: KH1's camera for cutscenes and
/// scenes with text, so the camera doesn't end up inside things).
fn ours() -> bool {
    let on = KH2.load(Ordering::Relaxed) && !super::takeover::kh1_scene();
    static WAS: AtomicBool = AtomicBool::new(true);
    static N: AtomicU32 = AtomicU32::new(0);
    if WAS.swap(on, Ordering::Relaxed) != on && KH2.load(Ordering::Relaxed) && N.fetch_add(1, Ordering::Relaxed) < 60 {
        log(if on { "KH2 camera: back from KH1's" } else { "KH2 camera: KH1's for a scene" });
    }
    if !on && FOV_SET.swap(false, Ordering::Relaxed) {
        // KH1's own field of view back for KH1's camera.
        unsafe { set_fov(rd::<f32>(BASE.load(Ordering::Relaxed) + DEFAULT_FOV_RVA)) };
    }
    on
}

/// KH2's field of view is ours (KH1's own goes back when KH1's camera runs).
static FOV_SET: AtomicBool = AtomicBool::new(false);

unsafe fn set_fov(deg: f32) {
    if (20.0..120.0).contains(&deg) {
        let set: unsafe extern "system" fn(f32) = std::mem::transmute(BASE.load(Ordering::Relaxed) + SET_FOV_RVA);
        set(deg);
    }
}

/// KH1's FOV setting (its PS2 "horizontal" degrees) that gives KH2's vertical field of view
/// (69.9°: 1.5 rad across a 4:3 frame; the PC versions keep the vertical one). KH1's vertical
/// is `2 atan(tan(fov / 2) · 384 / (2 · 0.84 · 256))` at 16:9 (`docs/kh1-render.md` 1.2), so
/// 76.1 here; KH1's own 53 is 48° vertical, a much narrower view (the user: it still felt off).
fn kh2_fov() -> f32 {
    let v = kh2_camera::Params::FIELD.fov_v();
    let k = 384.0 / (2.0 * 0.84 * 256.0);
    (2.0 * ((v * 0.5).tan() / k).atan()).to_degrees()
}

unsafe extern "system" fn follow_mode() {
    if !ours() || run(false).is_none() {
        let f: unsafe extern "system" fn() = std::mem::transmute(ORIG_FOLLOW.load(Ordering::Acquire));
        f();
    }
}

unsafe extern "system" fn lock_mode() {
    if !ours() || run(true).is_none() {
        let f: unsafe extern "system" fn() = std::mem::transmute(ORIG_LOCK.load(Ordering::Acquire));
        f();
    }
}

/// The camera's smoothness (the user: jittery in Dive to the Heart): every 5 s while it runs,
/// its updates per second, the spread of the time between them, and the biggest change in the
/// eye's, the look point's and Sora's motion between frames; and single jerks past 12 units.
fn jerk_note(eye: Vec3, at: Vec3, feet: Vec3, dt: f32, yaw: f32) {
    use std::sync::Mutex;
    struct S {
        last: Option<(Vec3, Vec3, Vec3, Vec3, Vec3, Vec3, u64)>,
        since: u64,
        n: u32,
        gap: (u64, u64),
        acc: (f32, f32, f32),
        dts: (f32, f32),
    }
    static ST: Mutex<S> = Mutex::new(S { last: None, since: 0, n: 0, gap: (u64::MAX, 0), acc: (0.0, 0.0, 0.0), dts: (f32::MAX, 0.0) });
    static LINES: AtomicU32 = AtomicU32::new(0);
    let Ok(mut st) = ST.lock() else { return };
    let now = now_ms();
    if let Some((e0, a0, f0, ve, va, vf, t0)) = st.last {
        let gap = now - t0;
        if gap > 200 {
            st.last = Some((eye, at, feet, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, now));
            return;
        }
        let (ve2, va2, vf2) = (eye - e0, at - a0, feet - f0);
        let (je, ja, jf) = ((ve2 - ve).length(), (va2 - va).length(), (vf2 - vf).length());
        st.n += 1;
        st.gap = (st.gap.0.min(gap), st.gap.1.max(gap));
        st.acc = (st.acc.0.max(je), st.acc.1.max(ja), st.acc.2.max(jf));
        st.dts = (st.dts.0.min(dt), st.dts.1.max(dt));
        if (je > 12.0 || ja > 12.0) && LINES.fetch_add(1, Ordering::Relaxed) < 120 {
            log(&format!(
                "KH2 camera jerk: eye {:.0} -> {:.0}, look {:.0} -> {:.0}, Sora {:.0} -> {:.0} per frame; dt {dt:.2}, {gap}ms; eye {:.0?} feet {:.0?} yaw {yaw:.2}",
                ve.length(), ve2.length(), va.length(), va2.length(), vf.length(), vf2.length(), eye.to_array(), feet.to_array()
            ));
        }
        st.last = Some((eye, at, feet, ve2, va2, vf2, now));
        if now - st.since >= 5000 {
            if st.n > 10 && LINES.fetch_add(1, Ordering::Relaxed) < 120 {
                log(&format!(
                    "KH2 camera, last {:.1}s: {:.0} updates/s, {}..{}ms apart, dt {:.2}..{:.2}; biggest change per frame: eye {:.1}, look {:.1}, Sora {:.1}",
                    (now - st.since) as f32 / 1000.0, st.n as f32 * 1000.0 / (now - st.since) as f32, st.gap.0, st.gap.1, st.dts.0, st.dts.1, st.acc.0, st.acc.1, st.acc.2
                ));
            }
            st.since = now;
            st.n = 0;
            st.gap = (u64::MAX, 0);
            st.acc = (0.0, 0.0, 0.0);
            st.dts = (f32::MAX, 0.0);
        }
    } else {
        st.since = now;
        st.last = Some((eye, at, feet, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, now));
    }
}

/// KH1 world (y down) ↔ the camera's (y up, divided by the scale).
fn to_cam(p: [f32; 3], s: f32) -> Vec3 {
    Vec3::new(p[0], -p[1], p[2]) / s
}
fn to_kh1(v: Vec3, s: f32) -> [f32; 3] {
    [v.x * s, -v.y * s, v.z * s]
}

/// One frame of KH2's camera in KH1's follow (`lock` false) or lock-on mode. None: leave it to
/// KH1's own this frame.
unsafe fn run(lock: bool) -> Option<()> {
    let base = BASE.load(Ordering::Relaxed);
    let sora = rd::<u64>(base + SORA_RVA) as usize;
    if sora < 0x10000 || rd::<u64>(base + SUBJECT_RVA) as usize != sora {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 10 {
            log(&format!("KH2 camera: KH1's own this frame (its subject isn't Sora, {})", if lock { "lock-on" } else { "follow" }));
        }
        return None;
    }
    let s = super::settings::get().camera_scale;
    // The lock-on target's point; when it can't be had this frame (the target between forms,
    // gone for a moment), KH2's camera carries on as the follow camera rather than handing the
    // frame to KH1's lock-on camera (the user: it jumped about in a boss fight).
    let lock_at = if lock {
        let actor = pose::ptr(rd::<u32>(base + LOCK_TARGET_RVA));
        let mut v = [0.0f32; 4];
        if actor > 0x10000 {
            let point: unsafe extern "system" fn(usize, i32, *mut [f32; 4]) -> f32 = std::mem::transmute(base + LOCK_POINT_RVA);
            point(actor, rd::<i32>(base + LOCK_TARGET_RVA + 4).max(0), &mut v);
        }
        let ok = actor > 0x10000 && v[..3].iter().all(|x| x.is_finite() && x.abs() < 1.0e6);
        if !ok {
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 10 {
                log(&format!("KH2 camera: lock-on target {actor:#x} has no point this frame: following"));
            }
        }
        ok.then(|| to_cam([v[0], v[1], v[2]], s))
    } else {
        None
    };
    let flags = rd::<u32>(base + ENTRY_FLAGS_RVA);
    wr(base + ENTRY_FLAGS_RVA, flags & !7);
    let fresh_mode = flags & 2 != 0;
    // KH2's field of view, every frame (KH1's area cameras and mode entries set their own).
    let _ = fresh_mode;
    set_fov(kh2_fov());
    FOV_SET.store(true, Ordering::Relaxed);
    let dt = rd::<f32>(base + STEP_RVA);
    let dt = if dt.is_finite() { dt.clamp(0.0, 4.0) } else { 1.0 };
    let feet = rd::<[f32; 3]>(sora + 0x10);
    let facing = rd::<f32>(sora + 0x3C);
    if !feet.iter().all(|v| v.is_finite()) || !facing.is_finite() {
        return None;
    }
    // Lowered as a whole (the user: Sora near the middle of the screen; KH2's look point, 170
    // above the feet, is above KH1 Sora's head).
    let lower = super::settings::get().camera_lower;
    let subject = kh2_camera::Subject { feet: to_cam(feet, s) - Vec3::Y * (lower / s), facing, run_speed: 8.0 };

    // Input: KH1's processed sticks (-1..1, y down) and buttons. Sora unable to act: no stick.
    let free = rd::<u32>(sora + 0x444) & 1 == 0;
    // The pad's own sticks with a small dead zone, as KH2 reads them: KH1's processed ones
    // drop the inner 47% (60 of 127.5), so gentle tilts did nothing. KH1's processed right
    // stick still counts when the raw one is idle (the mouse writes there).
    let raw = super::takeover::RAW_STICKS.load(Ordering::Relaxed).to_le_bytes();
    let axis = |b: u8| (b as f32 - 128.0) / 127.0;
    let dead = |v: [f32; 2]| -> [f32; 2] {
        let m = (v[0] * v[0] + v[1] * v[1]).sqrt();
        if m < STICK_DEAD {
            return [0.0; 2];
        }
        let k = ((m - STICK_DEAD) / (1.0 - STICK_DEAD)).min(1.0) / m;
        [v[0] * k, v[1] * k]
    };
    let raw_r = dead([axis(raw[0]), axis(raw[1])]);
    let r = if !free {
        [0.0; 2]
    } else if raw_r != [0.0; 2] {
        raw_r
    } else {
        rd::<[f32; 2]>(base + RIGHT_STICK_RVA)
    };
    let raw_l = dead([axis(raw[2]), axis(raw[3])]);
    let l = if raw_l != [0.0; 2] { raw_l } else { rd::<[f32; 2]>(base + LEFT_STICK_RVA) };
    let pressed = rd::<u32>(base + BUTTONS_RVA + 8);
    let turn = rd::<f32>(base + TURN_SIGN_RVA);
    let turn = if turn < 0.0 { -1.0 } else { 1.0 };
    let config = rd::<u64>(base + CONFIG_PTR_RVA) as usize;
    let auto = config < 0x10000 || rd::<i32>(config + 8) == 0;
    let ctl = kh2_camera::Controls {
        move_stick: Vec2::new(l[0], l[1]).clamp_length_max(1.0),
        // (Negated: KH1 world turns the other way round from the camera crate's; the user: x was
        // inverted.)
        look_stick: Vec2::new(-r[0] * turn, -r[1]).clamp_length_max(1.0),
        reset: free && pressed & R3 != 0,
        auto,
    };
    let wall = |a: Vec3, b: Vec3| -> Option<(Vec3, Vec3)> {
        let (fa, fb) = (to_kh1(a, s), to_kh1(b, s));
        let mut ray = [0u8; 0x40];
        let p = ray.as_mut_ptr() as usize;
        wr(p, [fa[0], fa[1], fa[2], 1.0f32]);
        wr(p + 0x10, [fb[0] - fa[0], fb[1] - fa[1], fb[2] - fa[2], 0.0f32]);
        let cast: unsafe extern "system" fn(*mut [u8; 0x40]) = std::mem::transmute(base + RAY_RVA);
        cast(&mut ray);
        let face = rd::<i16>(p + 0x30) as i32;
        if face < 0 {
            return None;
        }
        let hit = rd::<[f32; 3]>(p + 0x20);
        if !hit.iter().all(|v| v.is_finite()) {
            return None;
        }
        let owner = rd::<i32>(p + 0x34);
        let back = (a - b).normalize_or_zero();
        let mut n = back;
        if owner == -1 {
            let normal: unsafe extern "system" fn(i32, i32, *mut [f32; 4]) = std::mem::transmute(base + FACE_NORMAL_RVA);
            let mut m = [0.0f32; 4];
            normal(owner, face, &mut m);
            let v = Vec3::new(m[0], -m[1], m[2]).normalize_or_zero();
            // Facing the camera's side of the wall.
            if v != Vec3::ZERO {
                n = if v.dot(back) < 0.0 { -v } else { v };
            }
        }
        Some((to_cam(hit, s), n))
    };

    let Ok(mut g) = CAM.lock() else { return None };
    // A new camera when KH1 comes back to its follow camera from something not ours (an event,
    // an area camera, a menu, a cutscene), facing the way KH1's camera was looking from.
    let gap = now_ms().saturating_sub(LAST_MS.load(Ordering::Relaxed));
    // (A pause in between doesn't count: the pause menu's Sora was posed meanwhile; the user:
    // spamming pause while locked on started the camera over unlocked each time, shaking it.)
    let was_ours = gap < 300 || (super::pose::menu_sora_within(gap + 200) && gap < 60_000);
    // (Or after a while of KH1's own: a cutscene had it.)
    let _ = fresh_mode;
    // (Or Sora is far from where it was looking: a new room, a warp.)
    // (The eye, not the look point: locked on, that is 30% of the way to the target, and a far
    // target put it 600 away, so the camera was made anew over and over, the user's boss fight.)
    let far = g.as_ref().is_some_and(|c| (c.eye - subject.feet).length() > 1200.0 / s);
    if g.is_none() || !was_ours || far {
        let f = kh2_camera::Params::FIELD;
        let mut c = kh2_camera::Camera::new(f, &subject);
        let (eye, at) = (rd::<[f32; 3]>(base + EYE_RVA), rd::<[f32; 3]>(base + TARGET_RVA));
        let d = Vec2::new(eye[0] - at[0], eye[2] - at[2]);
        // At KH2's default distance (the user: entering a room it started on the ground looking
        // up: KH1's pair was left over and close together, and KH2's closest distance puts the
        // eye 150 below the look point). Facing the way KH1's camera looked when that camera
        // was looking at Sora from a usual distance, else behind him.
        let near = Vec2::new(at[0] - feet[0], at[2] - feet[2]).length() < 200.0;
        let usual = d.length() / s >= f.dist_min && d.length() / s <= f.dist_max * 1.5;
        if near && usual && d.is_finite() {
            c.snap_yaw(d.x.atan2(d.y));
        }
        // Locked on: behind Sora facing the target at once, where the lock-on camera settles
        // (the user: after a pause, the battle pause, a limit, the save menu or examining
        // something, it came back unlocked and swung round to the target).
        if let Some(t) = lock_at {
            let to = Vec2::new(t.x - subject.feet.x, t.z - subject.feet.z);
            if to.length() > 1.0 && to.is_finite() {
                c.snap_yaw((-to.x).atan2(-to.y));
            }
        }
        *g = Some(c);
        if LOGGED.fetch_add(1, Ordering::Relaxed) < 20 {
            let why = if !was_ours { "after KH1's" } else if far { "Sora far away" } else { "first" };
            log(&format!("KH2 camera: started ({}, {why}), yaw {:.2}", if lock { "lock-on" } else { "follow" }, g.as_ref().map_or(0.0, |c| c.yaw)));
        }
    }
    let cam = g.as_mut()?;
    cam.update(&subject, &ctl, lock_at, dt, &wall);
    let (eye, at) = (cam.eye, cam.at);
    if !eye.is_finite() || !at.is_finite() || (eye - at).length() < 1.0 {
        *g = None;
        return None;
    }
    LAST_MS.store(now_ms(), Ordering::Relaxed);
    jerk_note(eye * s, at * s, subject.feet * s, dt, cam.yaw);
    let (e, t) = (to_kh1(eye, s), to_kh1(at, s));
    wr(base + EYE_RVA, [e[0], e[1], e[2], 1.0f32]);
    wr(base + TARGET_RVA, [t[0], t[1], t[2], 1.0f32]);
    // KH1's own view of it (its follow camera carries on from here when chosen again).
    let d = Vec3::new(e[0] - t[0], e[1] - t[1], e[2] - t[2]);
    let dist = d.length();
    wr(base + DIST_RVA, dist);
    wr(base + YAW_RVA, d.x.atan2(d.z));
    wr(base + PITCH_RVA, (-d.y / dist).clamp(-1.0, 1.0).asin());
    for a in KH1_MOTION_RVAS {
        wr(base + a, 0.0f32);
    }
    drop(g);
    if !lock {
        // Select: KH1's first-person view, where KH1 allows it.
        let state = rd::<i32>(sora + 0x70);
        if free && pressed & SELECT != 0 && matches!(state, 0 | 8 | 0x18) && rd::<u32>(base + FLAGS_RVA) & 0x2000 == 0 && rd::<u32>(base + FLAGS2_RVA) & 0x10 == 0 {
            let set: unsafe extern "system" fn(i32, f32) = std::mem::transmute(base + SET_MODE_RVA);
            set(2, 30.0);
            return Some(());
        }
        // KH1's climbing / ledge / swimming cameras take over as they would from its own.
        let states: unsafe extern "system" fn() = std::mem::transmute(base + STATE_CAMERAS_RVA);
        states();
    }
    Some(())
}
