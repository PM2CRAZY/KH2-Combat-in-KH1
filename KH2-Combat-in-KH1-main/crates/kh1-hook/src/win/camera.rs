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
//! Scenes: KH1's own camera, taken over with KH1's blend from KH2's view (`begin_handover`);
//! a room exit's scene keeps KH2's until KH1 holds the camera for its door camera or the room
//! changes (`exit_owns`; `docs/kh1-camera.md` 9.3).
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
/// Right stick across: dead zone for KH2's camera (of full tilt), kept small so gentle tilts
/// turn (the user).
const STICK_DEAD: f32 = 0.15;
/// Right stick up / down: KH2's own rule. KH2's camera zooms only while the stick's digital
/// up / down bit is set, a raw axis byte below 0x40 or above 0xC0: past half tilt.
const KH2_AXIS: f32 = 64.0 / 127.0;
/// Logical buttons: `+8` pressed this frame. R3 `0x200`, Select `0x8000`.
const BUTTONS_RVA: Rva = Rva(0x298D4A0);
const R3: u32 = 0x200;
const SELECT: u32 = 0x8000;
/// KH1's config block (`+8`: field camera 0 Auto / 1 Manual).
const CONFIG_PTR_RVA: Rva = Rva(0x2E1FE70);
/// KH1's Config camera options as its camera applies them (`docs/kh1-camera.md` 5.4): the
/// up/down option's sign (-1 Normal, +1 Reverse; KH1 multiplies its pitch input by it) and the
/// left/right option's (+1 Normal, -1 Reverse; its yaw input). The Config setter writes them at
/// once, a load or a new game from the save's config block. Pad and mouse alike: the mouse
/// reaches KH1's camera as the right stick.
const PITCH_SIGN_RVA: Rva = Rva(0x507AA8);
const TURN_SIGN_RVA: Rva = Rva(0x507AAC);
/// KH1 PC's mouse sensitivity, `u16 ()`: 0..40, default 20, in the PC settings (not the save).
const MOUSE_SENS_RVA: Rva = Rva(0x138B70);
const FLAGS_RVA: Rva = Rva(0x2867CD8);
const FLAGS2_RVA: Rva = Rva(0x2867CDC);
/// Nonzero while an event runs.
const EVENT_RVA: Rva = Rva(0x298D3A0);
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
/// KH1's mode-switch blend, run by its camera task after the mode's update: the view eases
/// from the blend-from eye / target to the internal pair over the duration (in 60 Hz frames).
const BLEND_FROM_EYE_RVA: Rva = Rva(0x2538AD0);
const BLEND_FROM_TARGET_RVA: Rva = Rva(0x2538AE0);
const BLEND_TIME_RVA: Rva = Rva(0x2538A1C);
const BLEND_LEFT_RVA: Rva = Rva(0x2538A20);
/// The view's own eye / target and field of view (what the frame is drawn from).
const VIEW_EYE_RVA: Rva = Rva(0x299C4E0);
const VIEW_TARGET_RVA: Rva = Rva(0x299C4F0);
const VIEW_FOV_RVA: Rva = Rva(0x299C500);
/// KH2's camera handing the follow camera to KH1's own mid-scene: KH1's blend this long (as its
/// own switches back to the follow camera, 30).
const HANDOVER_FRAMES: f32 = 30.0;
/// A room exit's scene keeps KH2's camera at most this long before handing it to KH1's.
const EXIT_MAX_MS: u64 = 4000;

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

/// KH1 handed its camera back to Sora after holding or releasing it: its mode switch
/// `0x1401E3200` entered the follow (or lock-on) camera from mode 0. That is how every one of
/// KH1's camera resets ends (`docs/kh1-camera.md` 9.1): event script ops 394 / 395 (hold, then
/// "back behind Sora" `0x1401E2B50`, around examining, doors and scripted moments), an event or
/// an action's camera given back (`0x1401E2500`, after a Limit's or a summon's cinematic camera),
/// a room load (`0x1401E2F00`). KH1's follow camera then re-places itself at KH1's own default
/// (its distance and pitch, the yaw it kept or the yaw the cinematic camera ended on); the user:
/// with KH2's camera chosen, KH2's default position instead. The next KH2 camera started
/// (`run`) is placed at KH2's default: behind Sora at KH2's distance and height.
static RESET_PENDING: AtomicBool = AtomicBool::new(false);
/// Since such a reset, KH2's camera keeps the follow camera through the rest of the scene that
/// asked for it (`kh1_scene` lingering: the event's last frames, Sora still held after an
/// examine), until KH1 takes its camera elsewhere again, moves Sora itself, or the scene ends.
static RESET_OWNS: AtomicBool = AtomicBool::new(false);
/// When KH1 last ran its follow / lock-on camera mode (ours or its own), and Sora's feet then.
static LAST_CALL_MS: AtomicU64 = AtomicU64::new(0);
static LAST_FEET: Mutex<[f32; 3]> = Mutex::new([0.0; 3]);
/// KH1's internal eye / target as KH2's camera last wrote them (bit patterns): untouched since,
/// nothing else moved the camera (a hitch, not KH1's camera), and KH2's carries on.
static LAST_PAIR: Mutex<[u32; 6]> = Mutex::new([0; 6]);

unsafe fn internal_pair(base: usize) -> [u32; 6] {
    let (e, t) = (rd::<[u32; 3]>(base + EYE_RVA), rd::<[u32; 3]>(base + TARGET_RVA));
    [e[0], e[1], e[2], t[0], t[1], t[2]]
}

/// Each call of KH1's follow / lock-on mode, before deciding whose camera runs: notes KH1's
/// camera resets (see `RESET_PENDING`).
unsafe fn note_entry(lock: bool) {
    let base = BASE.load(Ordering::Relaxed);
    let now = now_ms();
    // KH1 had its camera elsewhere in between (an event camera, an area camera, a menu):
    // whatever owned the follow camera since the last reset is over.
    let away = now.saturating_sub(LAST_CALL_MS.swap(now, Ordering::Relaxed));
    if away > 150 {
        RESET_OWNS.store(false, Ordering::Relaxed);
        // A room exit's scene too: KH1 held or released its camera (a door's camera, the fade
        // to the next room), the view KH2's camera left on screen until then.
        let since = EXIT_SINCE.swap(0, Ordering::Relaxed);
        if since != 0 && since != EXIT_OVER {
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 40 {
                log(&format!(
                    "KH2 camera: the exit transition handed over: KH1 took its camera {} ms into it (its hold / door camera / the next room), for {away} ms; KH2's last view and field of view were on screen until then",
                    (now - away).saturating_sub(since)
                ));
            }
        }
    }
    // Any of KH1's mode entries ends a hand-over's blend: it is KH1's own from here.
    if rd::<u32>(BASE.load(Ordering::Relaxed) + ENTRY_FLAGS_RVA) & 2 != 0 {
        HANDOVER.store(false, Ordering::Relaxed);
    }
    // Sora walked by KH1 in a scene (an event moving him): KH1's camera again.
    let sora = rd::<u64>(base + SORA_RVA) as usize;
    if sora > 0x10000 {
        let feet = rd::<[f32; 3]>(sora + 0x10);
        if let Ok(mut last) = LAST_FEET.lock() {
            let moved = ((feet[0] - last[0]).powi(2) + (feet[2] - last[2]).powi(2)).sqrt();
            if moved > 3.0 && RESET_OWNS.load(Ordering::Relaxed) && super::takeover::kh1_scene() {
                RESET_OWNS.store(false, Ordering::Relaxed);
                static N: AtomicU32 = AtomicU32::new(0);
                if N.fetch_add(1, Ordering::Relaxed) < 20 {
                    log("KH2 camera: KH1 moves Sora in the scene after its camera reset: KH1's camera");
                }
            }
            *last = feet;
        }
    }
    // Mode-entry flags: bit 1 = the mode's first frame, bit 0 = entered from mode 0 (KH1's hold
    // or release), bit 2 = a snapshot restored (the view from before a save / load, not a reset).
    let flags = rd::<u32>(base + ENTRY_FLAGS_RVA);
    if flags & 2 == 0 || !KH2.load(Ordering::Relaxed) {
        return;
    }
    if flags & 7 != 3 {
        // Back from another of KH1's cameras (area camera, first person, climbing...): no reset.
        RESET_PENDING.store(false, Ordering::Relaxed);
        return;
    }
    RESET_PENDING.store(true, Ordering::Relaxed);
    EXIT_SINCE.store(0, Ordering::Relaxed);
    let scene = super::takeover::kh1_scene();
    let owns = scene && !super::takeover::kh1_moves_sora();
    RESET_OWNS.store(owns, Ordering::Relaxed);
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 60 {
        let state = if sora > 0x10000 { rd::<i32>(sora + 0x70) } else { -1 };
        log(&format!(
            "KH2 camera: KH1 reset its camera ({}, back from its hold / release; event {}, Sora state {state}): KH2's default position{}",
            if lock { "lock-on" } else { "follow" },
            rd::<u32>(base + EVENT_RVA) != 0,
            if owns {
                ", KH2's camera through the rest of the scene"
            } else if scene {
                " once KH1 lets go of Sora"
            } else {
                ""
            }
        ));
    }
}

/// KH2's camera this frame: chosen, and no cutscene (the user: KH1's camera for cutscenes and
/// scenes with text, so the camera doesn't end up inside things), except after KH1 has given
/// its camera back to Sora with a reset (`RESET_OWNS`) and through a room exit's scene
/// (`exit_owns`).
fn ours() -> bool {
    let kh2 = KH2.load(Ordering::Relaxed);
    let scene = kh2 && super::takeover::kh1_scene();
    if !scene || super::takeover::kh1_moves_sora() {
        RESET_OWNS.store(false, Ordering::Relaxed);
    }
    if !scene {
        EXIT_SINCE.store(0, Ordering::Relaxed);
    }
    let mut on = kh2 && (!scene || RESET_OWNS.load(Ordering::Relaxed));
    if kh2 && scene && !on {
        on = exit_owns();
    }
    static WAS: AtomicBool = AtomicBool::new(true);
    static N: AtomicU32 = AtomicU32::new(0);
    if WAS.swap(on, Ordering::Relaxed) != on && KH2.load(Ordering::Relaxed) && N.fetch_add(1, Ordering::Relaxed) < 60 {
        log(if on { "KH2 camera: back from KH1's" } else { "KH2 camera: KH1's for a scene" });
    }
    on
}

/// When a room exit's scene began with KH2's camera up (`now_ms`, 0: none, `EXIT_OVER`: this
/// scene's exit window ran out).
static EXIT_SINCE: AtomicU64 = AtomicU64::new(0);
const EXIT_OVER: u64 = u64::MAX;

/// A room exit (the user: "several rooms in the game have this camera glitch"): Sora walks into
/// an exit's trigger (a map face event of KH1's second area script, 100 and up:
/// `docs/kh1-exits-push.md`), and the exit's script takes him (a scene: KH1 holds or walks
/// him). In KH1 the follow camera stays on him until the script holds the camera (op 394) and
/// its door camera cuts in (ops 36 / 35, with their own field of view), or the room fades out
/// and changes. That follow camera is KH2's here, so KH2's keeps it through the exit: KH1's
/// hold then freezes KH2's view as it was, and KH1's door camera cuts from it. (Handing over to
/// KH1's own follow camera as the scene began snapped the view in for the frame or two before
/// the hold: KH1's narrower field of view, its lower look point.) Over at once when KH1 has its
/// camera elsewhere (`note_entry`), the room changes (a reset), the scene ends, or after
/// `EXIT_MAX_MS` (KH1's own then, blended in).
fn exit_owns() -> bool {
    let now = now_ms();
    let since = EXIT_SINCE.load(Ordering::Relaxed);
    if since == EXIT_OVER {
        return false;
    }
    if since != 0 {
        if now.saturating_sub(since) < EXIT_MAX_MS {
            return true;
        }
        // (Not again for this scene.)
        EXIT_SINCE.store(EXIT_OVER, Ordering::Relaxed);
        log("KH2 camera: the exit's scene still runs the follow camera after 4 s (a scene, not a room change): KH1's camera");
        return false;
    }
    // Starting: KH2's camera ran a moment ago and Sora just touched an exit.
    let Some((event, ago)) = super::gates::exit_touched() else { return false };
    if ago > 500 || now.saturating_sub(LAST_MS.load(Ordering::Relaxed)) > 300 {
        return false;
    }
    EXIT_SINCE.store(now, Ordering::Relaxed);
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 40 {
        let base = BASE.load(Ordering::Relaxed);
        let sora = unsafe { rd::<u64>(base + SORA_RVA) } as usize;
        let state = if sora > 0x10000 { unsafe { rd::<i32>(sora + 0x70) } } else { -1 };
        log(&format!(
            "KH2 camera: exit transition (map face event {event}, {ago} ms ago; event {}, KH1 walks Sora {}, Sora state {state}): KH2's camera keeps the follow camera until KH1 takes its own (its door camera) or the room changes",
            unsafe { rd::<u32>(base + EVENT_RVA) } != 0,
            super::takeover::kh1_moves_sora()
        ));
    }
    true
}

/// KH2's field of view is ours (KH1's own goes back when KH1's camera runs).
static FOV_SET: AtomicBool = AtomicBool::new(false);
/// KH2's camera handed the follow (or lock-on) camera to KH1's own mid-scene, KH1's blend from
/// KH2's view running; the field of view eases back with it from `FOV_FROM`.
static HANDOVER: AtomicBool = AtomicBool::new(false);
static FOV_FROM: AtomicU32 = AtomicU32::new(0);

/// KH1's own follow / lock-on camera runs this frame (`on`: KH2's would have, but couldn't).
unsafe fn kh1_frame(lock: bool, on: bool) {
    let base = BASE.load(Ordering::Relaxed);
    if !on {
        begin_handover(base, lock);
    }
    let f: unsafe extern "system" fn() = std::mem::transmute(if lock { ORIG_LOCK.load(Ordering::Acquire) } else { ORIG_FOLLOW.load(Ordering::Acquire) });
    f();
    if !on {
        ease_fov(base);
    }
}

/// KH1's camera takes the view from KH2's mid-scene (the scene began while KH1's follow or
/// lock-on mode runs, no mode switch): as KH1 does from its other cameras, a blend from the view
/// on screen (KH2's) into its own, and KH2's field of view eased back to KH1's with it. Before,
/// the view snapped: KH1's field of view at once (KH2's 76.1 to KH1's 53, the picture 1.57
/// times bigger) and its lower look point. Not when KH1 enters the mode this frame (its entry
/// places the camera and sets its field of view) or a blend of its own already runs.
unsafe fn begin_handover(base: usize, lock: bool) {
    if !FOV_SET.swap(false, Ordering::Relaxed) {
        return;
    }
    let kh1_fov = rd::<f32>(base + DEFAULT_FOV_RVA);
    let recent = now_ms().saturating_sub(LAST_MS.load(Ordering::Relaxed)) < 300;
    let entry = rd::<u32>(base + ENTRY_FLAGS_RVA) & 2 != 0;
    let blending = rd::<f32>(base + BLEND_LEFT_RVA) > 0.0;
    let (eye, at) = (rd::<[f32; 4]>(base + VIEW_EYE_RVA), rd::<[f32; 4]>(base + VIEW_TARGET_RVA));
    let fov = rd::<f32>(base + VIEW_FOV_RVA);
    if !recent || entry || blending || !eye.iter().chain(at.iter()).all(|v| v.is_finite()) || !(20.0..120.0).contains(&fov) {
        set_fov(kh1_fov);
        return;
    }
    wr(base + BLEND_FROM_EYE_RVA, [eye[0], eye[1], eye[2], 1.0f32]);
    wr(base + BLEND_FROM_TARGET_RVA, [at[0], at[1], at[2], 1.0f32]);
    wr(base + BLEND_TIME_RVA, HANDOVER_FRAMES);
    wr(base + BLEND_LEFT_RVA, HANDOVER_FRAMES);
    FOV_FROM.store(fov.to_bits(), Ordering::Relaxed);
    HANDOVER.store(true, Ordering::Relaxed);
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 40 {
        log(&format!(
            "KH2 camera: handed to KH1's {} camera mid-scene: KH1 blends from KH2's view over {HANDOVER_FRAMES} frames, field of view {fov:.1} -> {kh1_fov:.1} with it",
            if lock { "lock-on" } else { "follow" }
        ));
    }
}

/// While KH1's blend from KH2's view runs: the field of view along KH1's eased weight
/// (`1 - sin(left / time · π/2)`, as its camera task eases the view), KH1's own at the end.
unsafe fn ease_fov(base: usize) {
    if !HANDOVER.load(Ordering::Relaxed) {
        return;
    }
    let kh1_fov = rd::<f32>(base + DEFAULT_FOV_RVA);
    let step = rd::<f32>(base + STEP_RVA);
    let step = if step.is_finite() { step.clamp(0.0, 4.0) } else { 1.0 };
    let (time, left) = (rd::<f32>(base + BLEND_TIME_RVA), rd::<f32>(base + BLEND_LEFT_RVA) - step);
    if !(time > 0.0) || !(left > 0.0) {
        HANDOVER.store(false, Ordering::Relaxed);
        set_fov(kh1_fov);
        return;
    }
    let w = (1.0 - ((left / time).min(1.0) * std::f32::consts::FRAC_PI_2).sin()).clamp(0.0, 1.0);
    let from = f32::from_bits(FOV_FROM.load(Ordering::Relaxed));
    set_fov(from + (kh1_fov - from) * w);
}

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
    note_entry(false);
    let on = ours();
    if !on || run(false).is_none() {
        kh1_frame(false, on);
    }
}

unsafe extern "system" fn lock_mode() {
    note_entry(true);
    let on = ours();
    if !on || run(true).is_none() {
        kh1_frame(true, on);
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

/// KH1's camera options, read each frame so a change in the Config menu counts at once (the
/// player: KH2's camera ignored the inverted X / Y setting): (up/down reversed, left/right
/// reversed, mouse sensitivity). Logged at the start and on each change.
unsafe fn kh1_options(base: usize) -> (bool, bool, u16) {
    let y_rev = rd::<f32>(base + PITCH_SIGN_RVA) > 0.0;
    let x_rev = rd::<f32>(base + TURN_SIGN_RVA) < 0.0;
    let sens: unsafe extern "system" fn() -> u16 = std::mem::transmute(base + MOUSE_SENS_RVA);
    let sens = sens().min(40);
    static LAST: AtomicU32 = AtomicU32::new(u32::MAX);
    static N: AtomicU32 = AtomicU32::new(0);
    let packed = y_rev as u32 | (x_rev as u32) << 1 | (sens as u32) << 8;
    if LAST.swap(packed, Ordering::Relaxed) != packed && N.fetch_add(1, Ordering::Relaxed) < 40 {
        let word = |r: bool| if r { "Reverse" } else { "Normal" };
        log(&format!("KH2 camera: KH1's camera options: up/down {}, left/right {}, mouse sensitivity {sens}", word(y_rev), word(x_rev)));
    }
    (y_rev, x_rev, sens)
}

/// The mouse's speed for KH2's camera from KH1's mouse sensitivity. KH1's own camera turns at
/// most `2.5° + 0.0875° · sensitivity` a frame with the mouse (2.5° with the pad); ours keeps
/// that proportion, 1 at the default 20 (0.59 at 0, 1.41 at 40).
fn mouse_speed(sens: u16) -> f32 {
    (2.5 + 0.0875 * sens as f32) / (2.5 + 0.0875 * 20.0)
}

/// The view never looks up more steeply than KH2's free camera can (the player: it looked
/// straight at the sky at times): at its closest, the eye 150 below the look point and 250
/// away, 31°. Steeper (the eye pulled in low behind Sora by a slope or a wall, the look point
/// rising ahead of the eye in a high jump, a lock-on target overhead) lowers the look point,
/// the eye staying where it is. Some: it was clamped.
fn clamp_look_up(eye: Vec3, at: Vec3, yaw: f32) -> Option<Vec3> {
    let tan = kh2_camera::EYE_DOWN / kh2_camera::Params::FIELD.dist_min;
    let d = at - eye;
    let flat = Vec2::new(d.x, d.z);
    let h = flat.length();
    if d.y <= h * tan || !d.is_finite() {
        return None;
    }
    // (Straight under it: along the camera's facing, the eye being at `at + dist·(sin, cos)`.)
    let dir = if h > 1.0 { flat / h } else { -Vec2::new(yaw.sin(), yaw.cos()) };
    let c = 1.0 / (1.0 + tan * tan).sqrt();
    Some(eye + Vec3::new(dir.x * c, tan * c, dir.y * c) * d.length())
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
    // Back before a hand-over's blend into KH1's camera ended: ours at once (KH1's own blends,
    // from its other cameras, are left to run).
    if HANDOVER.swap(false, Ordering::Relaxed) {
        wr(base + BLEND_LEFT_RVA, 0.0f32);
    }
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
    // The right stick an axis at a time (the player: the camera looked up at the sky at random
    // times; one dead zone round the middle let any slant of the stick while turning zoom it
    // in, down to the ground looking up). Across: the small dead zone. Up / down: KH2's rule,
    // nothing until past half tilt, then the tilt.
    let across = |v: f32| -> f32 {
        if v.abs() < STICK_DEAD {
            0.0
        } else {
            v.signum() * ((v.abs() - STICK_DEAD) / (1.0 - STICK_DEAD)).min(1.0)
        }
    };
    let updown = |v: f32| -> f32 {
        if v.abs() > KH2_AXIS {
            v.clamp(-1.0, 1.0)
        } else {
            0.0
        }
    };
    let raw_r = [across(axis(raw[0])), updown(axis(raw[1]))];
    // KH1's processed right stick with the pad's idle: the mouse (KH1 PC writes it there; the
    // pad's own would be idle there too), KH2's up / down rule for it as well (a mouse moved
    // across always moves a little up or down).
    let mut mouse = false;
    let r = if !free {
        [0.0; 2]
    } else if raw_r != [0.0; 2] {
        raw_r
    } else {
        let m = rd::<[f32; 2]>(base + RIGHT_STICK_RVA);
        let m = if m.iter().all(|v| v.is_finite()) { [m[0].clamp(-1.0, 1.0), updown(m[1])] } else { [0.0; 2] };
        mouse = m != [0.0; 2];
        m
    };
    let raw_l = dead([axis(raw[2]), axis(raw[3])]);
    let l = if raw_l != [0.0; 2] { raw_l } else { rd::<[f32; 2]>(base + LEFT_STICK_RVA) };
    let pressed = rd::<u32>(base + BUTTONS_RVA + 8);
    // KH1's Config camera options, as KH1's camera applies them: to the pad and the mouse alike,
    // and the mouse sensitivity to the mouse.
    let (y_rev, x_rev, sens) = kh1_options(base);
    let turn = if x_rev { -1.0 } else { 1.0 };
    let tilt = if y_rev { -1.0 } else { 1.0 };
    let speed = if mouse { mouse_speed(sens) } else { 1.0 };
    let config = rd::<u64>(base + CONFIG_PTR_RVA) as usize;
    let auto = config < 0x10000 || rd::<i32>(config + 8) == 0;
    let ctl = kh2_camera::Controls {
        move_stick: Vec2::new(l[0], l[1]).clamp_length_max(1.0),
        // (Negated: KH1 world turns the other way round from the camera crate's; the user: x was
        // inverted. And KH1's y is down: up zooms in, as KH1's Normal looks up.)
        look_stick: Vec2::new(-r[0] * turn, -r[1] * tilt) * speed,
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
    // an area camera, a menu, a cutscene) or resets its camera: at KH2's default position,
    // unless taking over from another of KH1's cameras this frame.
    let gap = now_ms().saturating_sub(LAST_MS.load(Ordering::Relaxed));
    // (A pause in between doesn't count: the pause menu's Sora was posed meanwhile; the user:
    // spamming pause while locked on started the camera over unlocked each time, shaking it.)
    // (Nor a moment nothing else moved the camera in: KH1's internal pair still as written, a
    // hitch, not KH1's camera; that started the camera over behind Sora with the reset below.)
    let untouched = gap < 5000 && LAST_PAIR.lock().is_ok_and(|p| *p == internal_pair(base));
    let was_ours = gap < 300 || (super::pose::menu_sora_within(gap + 200) && gap < 60_000) || untouched;
    // (Or after a while of KH1's own: a cutscene had it.)
    // (Or KH1 reset its camera: KH2's default position, see `RESET_PENDING`.)
    let reset = RESET_PENDING.swap(false, Ordering::Relaxed);
    // KH1 handing over from one of its other cameras this very frame (an area camera, first
    // person, climbing / ledge, a conversation; or the view restored after a load): KH1 eases
    // from it into ours, so ours faces the way it looked. Not after a reset (bit 0: from mode 0).
    let handover = fresh_mode && (flags & 1 == 0 || flags & 4 != 0);
    // (Or Sora is far from where it was looking: a new room, a warp.)
    // (The eye, not the look point: locked on, that is 30% of the way to the target, and a far
    // target put it 600 away, so the camera was made anew over and over, the user's boss fight.)
    let far = g.as_ref().is_some_and(|c| (c.eye - subject.feet).length() > 1200.0 / s);
    if g.is_none() || !was_ours || far || reset {
        let f = kh2_camera::Params::FIELD;
        // KH2's default position: behind Sora at KH2's default distance and height (the user:
        // entering a room it started on the ground looking up, KH1's pair left over; and after
        // a Limit or examining something it took KH1's reset placement's direction).
        let mut c = kh2_camera::Camera::new(f, &subject);
        let (eye, at) = (rd::<[f32; 3]>(base + EYE_RVA), rd::<[f32; 3]>(base + TARGET_RVA));
        let d = Vec2::new(eye[0] - at[0], eye[2] - at[2]);
        // Taking over from another of KH1's cameras this frame: facing the way it looked, when
        // it was looking at Sora from a usual distance.
        let near = Vec2::new(at[0] - feet[0], at[2] - feet[2]).length() < 200.0;
        let usual = d.length() / s >= f.dist_min && d.length() / s <= f.dist_max * 1.5;
        if handover && !reset && near && usual && d.is_finite() {
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
            let why = if reset {
                "KH1 reset its camera: KH2's default"
            } else if !was_ours && handover {
                "after KH1's, facing its way"
            } else if !was_ours {
                "after KH1's: KH2's default"
            } else if far {
                "Sora far away"
            } else {
                "first"
            };
            log(&format!("KH2 camera: started ({}, {why}), yaw {:.2}", if lock { "lock-on" } else { "follow" }, g.as_ref().map_or(0.0, |c| c.yaw)));
        }
    }
    let cam = g.as_mut()?;
    let dist_was = cam.dist;
    cam.update(&subject, &ctl, lock_at, dt, &wall);
    let (eye, mut at) = (cam.eye, cam.at);
    if !eye.is_finite() || !at.is_finite() || (eye - at).length() < 1.0 {
        *g = None;
        return None;
    }
    // (What drove a zoom all the way in, for reports of the camera looking up.)
    if cam.dist <= cam.params.dist_min && dist_was > cam.params.dist_min && ctl.look_stick.y > 0.0 {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 30 {
            log(&format!(
                "KH2 camera: zoomed all the way in by the {} (up/down {:.2}, across {:.2}; raw right stick {:02x} {:02x})",
                if mouse { "mouse" } else { "right stick" },
                ctl.look_stick.y,
                ctl.look_stick.x,
                raw[0],
                raw[1]
            ));
        }
    }
    let clamped = clamp_look_up(eye, at, cam.yaw);
    static CLAMPED: AtomicBool = AtomicBool::new(false);
    let onset = !CLAMPED.swap(clamped.is_some(), Ordering::Relaxed);
    if let Some(lower) = clamped {
        static N: AtomicU32 = AtomicU32::new(0);
        if onset && N.fetch_add(1, Ordering::Relaxed) < 30 {
            let d = at - eye;
            log(&format!(
                "KH2 camera: looked up {:.0}° ({}, distance {:.0}, eye {:.0} above the feet): held at KH2's 31°",
                d.y.atan2(Vec2::new(d.x, d.z).length()).to_degrees(),
                if lock_at.is_some() { "locked on" } else { "following" },
                (eye - at).length(),
                eye.y - subject.feet.y
            ));
        }
        at = lower;
    }
    LAST_MS.store(now_ms(), Ordering::Relaxed);
    jerk_note(eye * s, at * s, subject.feet * s, dt, cam.yaw);
    let (e, t) = (to_kh1(eye, s), to_kh1(at, s));
    wr(base + EYE_RVA, [e[0], e[1], e[2], 1.0f32]);
    wr(base + TARGET_RVA, [t[0], t[1], t[2], 1.0f32]);
    if let Ok(mut p) = LAST_PAIR.lock() {
        *p = internal_pair(base);
    }
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
