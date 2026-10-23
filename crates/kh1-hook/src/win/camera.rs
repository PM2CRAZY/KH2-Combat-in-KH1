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
//!
//! KH2's camera as KH1's own camera mode (`camera_calls = kh2`, `docs/kh1-kh2-camera.md`): the
//! user, "we need to instead write kh2s camera into kh1 so that kh1 can handle the kh2 camera
//! while still keeping the cameras movements and fov". KH1's gameplay calls on its camera module
//! drive KH2's camera instead of taking the view from it:
//! - its movement-state cameras (ledge, climbing, poles: modes 4 / 5 / 6 / 8, which mode 1's end
//!   `0x1401E3320` switches to from Sora's state) are KH2's follow camera too;
//! - its switches between two of our modes (lock-on on and off) don't blend from a still copy
//!   of our camera (KH2's own camera eases into the lock-on);
//! - its own moves holding Sora still, and its area scripts walking him (doors, zone entries),
//!   no longer count as a scene: only an event, and the KH1-only states (the Jungle Slider, the
//!   carpet, water), hand mode 1 / 3 to KH1's own updates;
//! - its hand-overs from its other cameras (area cameras, first person, events) blend into ours
//!   with the field of view eased along with the eye.
//! Scripted cameras stay KH1's: mode 0 (an event's hold or release), conversations (mode 7),
//! the event camera tasks writing the public pair, area cameras (mode 9). First person (mode 2)
//! is KH2's when entered from KH2's follow camera (Select: `first_person_mode`), KH1's own else.

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
/// Logical buttons: `+8` pressed this frame. R3 `0x200`, Select `0x8000`.
const BUTTONS_RVA: Rva = Rva(0x298D4A0);
const R3: u32 = 0x200;
const SELECT: u32 = 0x8000;
/// Steam only: KH1's raw pad word as its pad step (`0x14028b0b0`) leaves it for the button remap
/// (slot 0's `+4` held, PS2 order: the pads and the keyboard merged, the stick direction bits
/// above). R2 is `0x200` there; KH1's remap (`0x140528290`) gives R2 no bit of its own (L2 and R2
/// both set `0x400000`).
const RAW_HELD_STEAM: usize = 0x298C9B4;
/// R2 in the PS2 order.
const RAW_R2: u32 = 0x200;
/// Steam only: KH1 PC's mouse, the same code as KH2 PC's (`0x1404e51f0` there):
/// `0x140138b30 (x*, y*, f32*, f32*, sensitivity u16*) -> moved`: the mouse record's motion this
/// frame, each axis held to ±1 (y flipped), and the sensitivity setting (`0x1404D8614 + 0x24`).
/// Its prologue is checked before the first call (`MOUSE_PROLOGUE`, from the Steam exe).
const MOUSE_STEAM: usize = 0x138B30;
const MOUSE_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10];
/// Steam only: KH1's keyboard / mouse mode (`0x140139590` returns it; 1 after a key or the mouse,
/// 0 after a pad button, `0x14013a2c0`), as KH2's `0x142b44a88`.
const KBM_STEAM: usize = 0x22DB3E8;
/// 0 not checked yet, 1 KH1's mouse function is where expected, 2 it isn't.
static MOUSE_OK: AtomicU32 = AtomicU32::new(0);
/// KH1's config block (`+8`: field camera 0 Auto / 1 Manual).
const CONFIG_PTR_RVA: Rva = Rva(0x2E1FE70);
/// The camera left/right option's sign (+1 normal).
const TURN_SIGN_RVA: Rva = Rva(0x507AAC);
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
/// The FOV as the setter keeps it (KH1's PS2 degrees).
const FOV_RVA: Rva = Rva(0x299C500);
/// KH1's camera mode (`docs/kh1-camera.md` 3).
const MODE_RVA: Rva = Rva(0x299C508);
/// KH1's mode-switch blend: time left, duration (the field camera task counts it down after the
/// mode's update and eases the public pair from the blend-from pair by `1 - cos(left/T · π/2)`).
const BLEND_LEFT_RVA: Rva = Rva(0x2538A20);
const BLEND_TIME_RVA: Rva = Rva(0x2538A1C);
/// KH1's movement-state cameras (mode, its update): vines / climbing surfaces (states 9 / 10),
/// state 0xF (ladders, poles), ledges (0xD / 0xE), state 0x12.
const STATE_MODES: [(usize, Rva); 4] = [(4, Rva(0x1E0C80)), (5, Rva(0x1E0E50)), (6, Rva(0x1E0F80)), (8, Rva(0x1E1390))];
static ORIG_STATE: [AtomicUsize; 4] = [AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0)];
/// KH1's first-person mode (2): its update and exit (Steam `0x1401E2390` / `0x1401E2610`), whose
/// entries run KH2's first person instead while it is on (`first_person_mode`).
const FP_UPDATE_RVA: Rva = Rva(0x1E0230);
const FP_EXIT_RVA: Rva = Rva(0x1E04B0);
static ORIG_FP: AtomicUsize = AtomicUsize::new(0);
static ORIG_FP_EXIT: AtomicUsize = AtomicUsize::new(0);
/// KH2's first-person view is on (Select from KH2's follow camera): KH1's mode 2 runs it.
static FP: AtomicBool = AtomicBool::new(false);
/// KH2's player fade in and out of first person (`0x1403b7810` on the player's `+0xa10`, from
/// `0x140165110` / `0x140165e30`): 30 frames.
const FP_FADE: f32 = 30.0;

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
    // (Modes 1 to 8: the follow, lock-on and movement-state cameras' entries.)
    super::sys::VirtualProtect(follow as *mut _, 0x80, 0x04, &mut old);
    wr(follow, follow_mode as *const () as u64);
    wr(lock, lock_mode as *const () as u64);
    // KH1's movement-state cameras: KH2's follow camera in their place (`state_mode`), each only
    // if its entry is KH1's own.
    let wrappers: [unsafe extern "system" fn(); 4] = [state_mode::<0>, state_mode::<1>, state_mode::<2>, state_mode::<3>];
    let mut swapped = Vec::new();
    for (i, (mode, orig)) in STATE_MODES.iter().enumerate() {
        let slot = base + MODES_RVA + mode * 0x10;
        let cur = rd::<u64>(slot) as usize;
        if cur != base + *orig {
            log(&format!("KH2 camera: KH1's camera mode {mode} isn't where expected ({cur:#x}): left to KH1"));
            continue;
        }
        ORIG_STATE[i].store(cur, Ordering::Release);
        wr(slot, wrappers[i] as *const () as u64);
        swapped.push(*mode);
    }
    // KH1's first person (mode 2): KH2's first person in it when entered from KH2's camera
    // (`first_person_mode`), only if both its entries are KH1's own.
    let fp = base + MODES_RVA + 0x20;
    let (u, x) = (rd::<u64>(fp) as usize, rd::<u64>(fp + 8) as usize);
    if u == base + FP_UPDATE_RVA && x == base + FP_EXIT_RVA {
        ORIG_FP.store(u, Ordering::Release);
        ORIG_FP_EXIT.store(x, Ordering::Release);
        wr(fp, first_person_mode as *const () as u64);
        wr(fp + 8, first_person_exit as *const () as u64);
        swapped.push(2);
    } else {
        log(&format!("KH2 camera: KH1's first-person mode isn't where expected ({u:#x}, {x:#x}): KH1's own first person"));
    }
    log(&format!(
        "KH2 camera: KH1's follow, lock-on and movement-state cameras ({swapped:?}) swapped for KH2's ({}; KH1's camera calls: {})",
        if KH2.load(Ordering::Relaxed) { "on" } else { "KH1's chosen" },
        if calls_kh2() { "KH2's camera" } else { "KH1's camera" }
    ));
}

/// KH1's gameplay camera calls drive KH2's camera (`camera_calls = kh2`); off, the older way:
/// they hand the view to KH1's own camera.
fn calls_kh2() -> bool {
    super::settings::get().camera_calls_kh2
}

/// An event runs (a cutscene, a talk with text boxes): KH1's camera, the user's rule from stage
/// 70 ("so the camera doesn't end up inside things").
fn event_running(base: usize) -> bool {
    base != 0 && unsafe { rd::<u32>(base + EVENT_RVA) } != 0
}

/// Sora in one of KH1's own movement modes that keep KH1's camera (the Jungle Slider, the
/// magic carpet, swimming: `takeover::kh1_camera_mode`).
fn kh1_only_state(base: usize) -> bool {
    if base == 0 {
        return false;
    }
    let sora = unsafe { rd::<u64>(base + SORA_RVA) } as usize;
    sora > 0x10000 && super::takeover::kh1_camera_mode(unsafe { rd::<i32>(sora + 0x70) })
}

/// A scripted moment for the camera: with `camera_calls = kh2`, only an event or a KH1-only
/// state (not KH1's own moves holding Sora still, nor its area scripts walking him through a
/// door); otherwise the older test (`takeover::kh1_scene`).
fn scene() -> bool {
    if !calls_kh2() {
        return super::takeover::kh1_scene();
    }
    let base = BASE.load(Ordering::Relaxed);
    event_running(base) || kh1_only_state(base)
}

/// KH1 moves Sora itself in a scene (see `RESET_OWNS`).
fn moves_sora() -> bool {
    if !calls_kh2() {
        return super::takeover::kh1_moves_sora();
    }
    kh1_only_state(BASE.load(Ordering::Relaxed))
}

/// KH1's movement-state camera `STATE_MODES[I]`: Sora on a ledge, a vine, a climbing surface, a
/// ladder or a pole. With KH2's camera running and `camera_calls = kh2`, KH1 goes back to its
/// follow mode at once (no blend: the view is ours on both sides) and KH2's follow camera runs
/// this frame; KH1 only gets here when it entered the mode itself (mode 1's own end with KH1's
/// camera running, its lock-on's end `0x1401E1FB0`, a conversation's end `0x1401E1F60`).
unsafe extern "system" fn state_mode<const I: usize>() {
    let base = BASE.load(Ordering::Relaxed);
    let sora = rd::<u64>(base + SORA_RVA) as usize;
    let subject_sora = sora > 0x10000 && rd::<u64>(base + SUBJECT_RVA) as usize == sora;
    if calls_kh2() && subject_sora && ours() {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 20 {
            let state = rd::<i32>(sora + 0x70);
            log(&format!("KH2 camera: KH1's movement-state camera (mode {}, Sora state {state}) taken by KH2's follow camera", STATE_MODES[I].0));
        }
        let set: unsafe extern "system" fn(i32, f32) = std::mem::transmute(base + SET_MODE_RVA);
        set(1, 0.0);
        VIA_STATE.store(STATE_MODES[I].0, Ordering::Relaxed);
        follow_mode();
        return;
    }
    let f: unsafe extern "system" fn() = std::mem::transmute(ORIG_STATE[I].load(Ordering::Acquire));
    f();
    let via = ["state4", "state5", "state6", "state8"][I];
    let why = if !calls_kh2() { "older_camera_calls" } else if !subject_sora { "subject_not_sora" } else { "kh1_camera" };
    record(base, via, "kh1", why, rd::<u32>(base + ENTRY_FLAGS_RVA));
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
    if now.saturating_sub(LAST_CALL_MS.swap(now, Ordering::Relaxed)) > 150 {
        RESET_OWNS.store(false, Ordering::Relaxed);
    }
    // Sora walked by KH1 in a scene (an event moving him): KH1's camera again.
    let sora = rd::<u64>(base + SORA_RVA) as usize;
    if sora > 0x10000 {
        let feet = rd::<[f32; 3]>(sora + 0x10);
        if let Ok(mut last) = LAST_FEET.lock() {
            let moved = ((feet[0] - last[0]).powi(2) + (feet[2] - last[2]).powi(2)).sqrt();
            if moved > 3.0 && RESET_OWNS.load(Ordering::Relaxed) && scene() {
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
    let scene = scene();
    let owns = scene && !moves_sora();
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
/// its camera back to Sora with a reset (`RESET_OWNS`). A cutscene is `scene`: with
/// `camera_calls = kh2` an event or a KH1-only state, not KH1's own moves (shared states) holding
/// Sora still (the user: "any move that goes through kh1s animation tree ... take away the camera
/// from kh2").
fn ours() -> bool {
    let kh2 = KH2.load(Ordering::Relaxed);
    let scene = kh2 && scene();
    if !scene || moves_sora() {
        RESET_OWNS.store(false, Ordering::Relaxed);
    }
    let on = kh2 && (!scene || RESET_OWNS.load(Ordering::Relaxed));
    static WAS: AtomicBool = AtomicBool::new(true);
    static N: AtomicU32 = AtomicU32::new(0);
    if WAS.swap(on, Ordering::Relaxed) != on && KH2.load(Ordering::Relaxed) && N.fetch_add(1, Ordering::Relaxed) < 60 {
        let base = BASE.load(Ordering::Relaxed);
        let why = if !calls_kh2() {
            "the older test: an event, an area script or KH1 holding Sora still"
        } else if event_running(base) {
            "an event runs"
        } else {
            "a KH1-only state"
        };
        log(&if on { "KH2 camera: back from KH1's".to_string() } else { format!("KH2 camera: KH1's for a scene ({why})") });
    }
    if !on && FOV_SET.swap(false, Ordering::Relaxed) {
        // KH1's own field of view back for KH1's camera.
        unsafe { set_fov(rd::<f32>(BASE.load(Ordering::Relaxed) + DEFAULT_FOV_RVA)) };
    }
    on
}

/// KH2's field of view is ours (KH1's own goes back when KH1's camera runs).
static FOV_SET: AtomicBool = AtomicBool::new(false);
/// The FOV KH1's camera had when it began blending into ours (bits; NaN: no blend from KH1's).
static FOV_FROM: AtomicU32 = AtomicU32::new(0x7FC0_0000);

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
    camera_call(false);
}

unsafe extern "system" fn lock_mode() {
    note_entry(true);
    camera_call(true);
}

/// One call of KH1's follow (or lock-on) camera mode: ours when it is ours and runs, else KH1's
/// own; either way noted for the trace (`seen`).
unsafe fn camera_call(lock: bool) {
    let base = BASE.load(Ordering::Relaxed);
    // (Before `run` clears the first-frame bits.)
    let entry = rd::<u32>(base + ENTRY_FLAGS_RVA);
    // (KH2's first person left without KH1's mode switch, which calls its exit: Sora back.)
    if FP.load(Ordering::Relaxed) {
        first_person_exit();
    }
    set_why("");
    if let Ok(mut o) = OWN.lock() {
        *o = None;
    }
    let on = ours();
    let ran = on && run(lock).is_some();
    let why = if on { take_why() } else { kh1_why(base) };
    if !ran {
        let orig = if lock { &ORIG_LOCK } else { &ORIG_FOLLOW };
        let f: unsafe extern "system" fn() = std::mem::transmute(orig.load(Ordering::Acquire));
        f();
    }
    let state = VIA_STATE.swap(0, Ordering::Relaxed);
    let via = match (lock, state) {
        (true, _) => "lock",
        (false, 4) => "follow<-state4",
        (false, 5) => "follow<-state5",
        (false, 6) => "follow<-state6",
        (false, 8) => "follow<-state8",
        _ => "follow",
    };
    record(base, via, if ran { "kh2" } else { "kh1" }, why, entry);
}

/// Why it isn't KH2's camera this call.
fn kh1_why(base: usize) -> &'static str {
    if !KH2.load(Ordering::Relaxed) {
        "kh1_chosen"
    } else if event_running(base) {
        "event"
    } else if kh1_only_state(base) {
        "kh1_only_state"
    } else {
        "scene"
    }
}

/// A KH1 movement-state camera (mode 4 / 5 / 6 / 8) that KH2's follow camera took this call.
static VIA_STATE: AtomicUsize = AtomicUsize::new(0);
/// A note from `run` on this call (why it handed back to KH1, or that it went to first person).
static WHY: Mutex<&'static str> = Mutex::new("");

fn set_why(w: &'static str) {
    if let Ok(mut g) = WHY.lock() {
        *g = w;
    }
}
fn take_why() -> &'static str {
    WHY.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or("")
}
/// `run` handing the frame to KH1's camera, and why.
fn decline(why: &'static str) -> Option<()> {
    set_why(why);
    None
}

/// KH2's camera's own numbers on its last run (for the trace).
#[derive(Clone, Copy, Default)]
pub struct Own {
    /// The camera's yaw, the yaw it is turning to, and its distance (KH2 units: KH1's / scale).
    pub yaw: f32,
    pub goal_yaw: f32,
    pub dist: f32,
    /// The right stick (look) and left stick (move) as it read them, after the dead zone.
    pub look: [f32; 2],
    pub mv: [f32; 2],
    pub reset: bool,
    pub auto: bool,
    /// KH2's camera hold button (R2) held.
    pub hold: bool,
    /// KH2 PC's mouse this frame (its motion, when it moved) and keyboard / mouse mode.
    pub mouse: Option<[f32; 2]>,
    pub kbm: bool,
    /// Lock-on: the distance from Sora's feet to the target's point (KH1 units).
    pub lock_dist: Option<f32>,
}
static OWN: Mutex<Option<Own>> = Mutex::new(None);

/// The camera as the last follow / lock-on / movement-state call left it (for the trace, the
/// Select + Triangle recording). Sora's physics frame, which writes the trace row, runs before the
/// camera in a game frame, so this is the camera's frame before: `frame` says which.
#[derive(Clone, Copy, Default)]
pub struct Seen {
    /// The game frame (`takeover::GAME_FRAME`) and time of the call.
    pub frame: u64,
    pub ms: u64,
    /// "kh2" (ours placed it) or "kh1" (KH1's own mode ran); empty: no call yet.
    pub who: &'static str,
    /// Which KH1 mode entry was called: follow, lock, follow<-stateN (a movement-state camera ours
    /// took), stateN (one KH1 ran as its own).
    pub via: &'static str,
    /// Why KH1's, or a note ("first_person", "subject_not_sora", ...).
    pub why: &'static str,
    /// Mode-entry flags on the call (bit 1 first frame, bit 0 from mode 0, bit 2 from a snapshot).
    pub entry: u32,
    /// KH1's eye and look point after the call (KH2's way up: `up` is minus KH1's y), Sora's feet
    /// then, and how far each moved since the call before.
    pub eye: [f32; 3],
    pub at: [f32; 3],
    pub feet: [f32; 3],
    pub eye_step: f32,
    pub at_step: f32,
    /// KH1's own yaw / pitch / distance fields, its field of view, the mode-switch blend.
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    pub fov: f32,
    pub blend_left: f32,
    pub blend_time: f32,
    /// KH2's camera's numbers when it ran.
    pub own: Option<Own>,
    /// KH1's camera mode and blend left when the trace row was made (any mode, not only ours).
    pub mode_now: i32,
    pub blend_now: f32,
}
static SEEN: Mutex<Option<Seen>> = Mutex::new(None);

/// Notes the call that just ended.
unsafe fn record(base: usize, via: &'static str, who: &'static str, why: &'static str, entry: u32) {
    // (t354) Only for the trace, while it records a camera category.
    if !super::trace::wants(super::trace::cat::KH1_CAMERA | super::trace::cat::KH2_CAMERA) {
        return;
    }
    let sora = rd::<u64>(base + SORA_RVA) as usize;
    let feet = if sora > 0x10000 { rd::<[f32; 3]>(sora + 0x10) } else { [0.0; 3] };
    let up = |v: [f32; 4]| [v[0], -v[1], v[2]];
    let (eye, at) = (up(rd::<[f32; 4]>(base + EYE_RVA)), up(rd::<[f32; 4]>(base + TARGET_RVA)));
    let (ms, frame) = (now_ms(), super::takeover::GAME_FRAME.load(Ordering::Relaxed));
    let own = OWN.lock().ok().and_then(|o| *o);
    let Ok(mut g) = SEEN.lock() else { return };
    let step = |a: [f32; 3], b: [f32; 3]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
    let (eye_step, at_step) = match g.as_ref() {
        Some(p) if ms.saturating_sub(p.ms) < 250 => (step(eye, p.eye), step(at, p.at)),
        _ => (0.0, 0.0),
    };
    *g = Some(Seen {
        frame,
        ms,
        who,
        via,
        why,
        entry,
        eye,
        at,
        feet: [feet[0], -feet[1], feet[2]],
        eye_step,
        at_step,
        yaw: rd::<f32>(base + YAW_RVA),
        pitch: rd::<f32>(base + PITCH_RVA),
        dist: rd::<f32>(base + DIST_RVA),
        fov: rd::<f32>(base + FOV_RVA),
        blend_left: rd::<f32>(base + BLEND_LEFT_RVA),
        blend_time: rd::<f32>(base + BLEND_TIME_RVA),
        own,
        mode_now: 0,
        blend_now: 0.0,
    });
}

/// The camera as the trace row sees it (`Seen::who` empty before the camera has run once).
pub fn seen() -> Seen {
    let base = BASE.load(Ordering::Relaxed);
    let mut s = SEEN.lock().ok().and_then(|g| *g).unwrap_or_default();
    if base != 0 {
        unsafe {
            s.mode_now = rd::<i32>(base + MODE_RVA);
            s.blend_now = rd::<f32>(base + BLEND_LEFT_RVA);
        }
    }
    s
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
        return decline("subject_not_sora");
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
    let dt = rd::<f32>(base + STEP_RVA);
    let dt = if dt.is_finite() { dt.clamp(0.0, 4.0) } else { 1.0 };
    let feet = rd::<[f32; 3]>(sora + 0x10);
    let facing = rd::<f32>(sora + 0x3C);
    if !feet.iter().all(|v| v.is_finite()) || !facing.is_finite() {
        return decline("bad_feet");
    }
    // Lowered as a whole (the user: Sora near the middle of the screen; KH2's look point, 170
    // above the feet, is above KH1 Sora's head).
    let lower = super::settings::get().camera_lower;
    // KH2's swing-behind and spring rates grow with the player's run speed (`+0x12C`, the form's:
    // Steam KH2 `0x140165110`): the form's KH2 run speed while in one, else Sora's (8).
    let run_speed = pose::kh2()
        .map(|kh2| super::form::current().and_then(|k| k.data(kh2)).map_or(&kh2.character, |f| &f.character).params.run_speed)
        .filter(|v| v.is_finite() && *v > 0.0)
        .unwrap_or(8.0);
    let subject = kh2_camera::Subject { feet: to_cam(feet, s) - Vec3::Y * (lower / s), facing, run_speed };

    // Input: the sticks (`sticks`) and buttons. Sora unable to act: no right stick.
    let free = rd::<u32>(sora + 0x444) & 1 == 0;
    let m = mouse(base);
    let (r, l) = sticks(base, free, &m);
    let pressed = rd::<u32>(base + BUTTONS_RVA + 8);
    let turn = turn_sign(base);
    let config = rd::<u64>(base + CONFIG_PTR_RVA) as usize;
    let auto = config < 0x10000 || rd::<i32>(config + 8) == 0;
    let hold = r2_held(base);
    let ctl = kh2_camera::Controls {
        // The crate's y is forward; KH1's (like KH2's pad) is positive pulled back. Passed
        // unflipped, "pulled straight back" read as forward and the swing-behind ran while Sora
        // ran toward the camera, turning him in a circle (tester trace f571370).
        // (Not held to 1: in a mouse frame KH2's pad scales both tilts by the sensitivity.)
        move_stick: Vec2::new(l[0], -l[1]),
        look_stick: look(r, turn),
        reset: free && pressed & R3 != 0,
        auto,
        hold,
        mouse: m.motion.map(Vec2::from),
        kbm: m.kbm,
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

    let Ok(mut g) = CAM.lock() else { return decline("cam_locked") };
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
    // KH1 switching between two of our modes this frame (lock-on on or off, one of its
    // movement-state cameras given back): its blend would ease from a still copy of our camera
    // (60 frames into the lock-on, 45 out), which held the view back while Sora ran. KH2's own
    // camera eases into and out of the lock-on, so the blend goes.
    let our_switch = calls_kh2() && fresh_mode && flags & 5 == 0 && gap < 300 && untouched;
    if our_switch && rd::<f32>(base + BLEND_LEFT_RVA) > 0.0 {
        wr(base + BLEND_LEFT_RVA, 0.0f32);
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 20 {
            log(&format!("KH2 camera: KH1's switch into its {} camera, from ours: no blend (KH2's camera goes on)", if lock { "lock-on" } else { "follow" }));
        }
    }
    // KH2's field of view, every frame (KH1's area cameras, first person and event cameras set
    // their own). While KH1 blends from one of its cameras into ours, eased with it from that
    // camera's.
    {
        let left = rd::<f32>(base + BLEND_LEFT_RVA);
        if fresh_mode {
            let from = rd::<f32>(base + FOV_RVA);
            FOV_FROM.store(if left > 0.0 && from.is_finite() { from.to_bits() } else { f32::NAN.to_bits() }, Ordering::Relaxed);
        }
        let target = kh2_fov();
        let from = f32::from_bits(FOV_FROM.load(Ordering::Relaxed));
        let time = rd::<f32>(base + BLEND_TIME_RVA);
        let fov = if left > 0.0 && from.is_finite() && time > 0.0 {
            // The weight KH1's task gives the blend-from camera after this frame's count.
            let k = ((left - dt).max(0.0) / time).clamp(0.0, 1.0);
            let w = 1.0 - (k * std::f32::consts::FRAC_PI_2).cos();
            from * w + target * (1.0 - w)
        } else {
            target
        };
        set_fov(fov);
        FOV_SET.store(true, Ordering::Relaxed);
    }
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
    let Some(cam) = g.as_mut() else { return decline("no_camera") };
    cam.update(&subject, &ctl, lock_at, dt, &wall);
    let (eye, at) = (cam.eye, cam.at);
    if !eye.is_finite() || !at.is_finite() || (eye - at).length() < 1.0 {
        *g = None;
        return decline("degenerate_camera");
    }
    if let Ok(mut o) = OWN.lock() {
        *o = Some(Own {
            yaw: cam.yaw,
            goal_yaw: cam.goal_yaw,
            dist: cam.dist,
            look: [r[0], r[1]],
            mv: [l[0], l[1]],
            reset: ctl.reset,
            auto: ctl.auto,
            hold: ctl.hold,
            mouse: m.motion,
            kbm: m.kbm,
            lock_dist: lock_at.map(|t| (t - subject.feet).length() * s),
        });
    }
    jerk_note(eye * s, at * s, subject.feet * s, dt, cam.yaw);
    write_view(base, eye, at, s);
    if !lock && free && pressed & SELECT != 0 && first_person_allowed(base, sora) && rd::<u32>(base + FLAGS_RVA) & 0x2000 == 0 {
        // Select: first person, where KH1 allows it (KH1's own test, Steam `0x1401E1920`; KH2's
        // is `0x1403ba300`). KH2's own first person in KH1's first-person mode (2) when its
        // entries are ours; else KH1's (blend 30).
        let set: unsafe extern "system" fn(i32, f32) = std::mem::transmute(base + SET_MODE_RVA);
        if ORIG_FP.load(Ordering::Acquire) != 0 {
            cam.enter_first_person();
            drop(g);
            enter_first_person(sora);
            set(2, 0.0);
        } else {
            drop(g);
            set(2, 30.0);
        }
        set_why("first_person");
        return Some(());
    }
    drop(g);
    if !lock {
        // KH1's climbing / ledge cameras: with `camera_calls = kh2` KH2's follow camera goes on
        // through them (the user: ledges took the camera), so KH1's switch into them
        // (`0x1401E3320`, from Sora's state) isn't called; otherwise they take over as they
        // would from its own.
        if !calls_kh2() {
            let states: unsafe extern "system" fn() = std::mem::transmute(base + STATE_CAMERAS_RVA);
            states();
        } else if MODE_LOGGED.swap(false, Ordering::Relaxed) {
            log("KH2 camera: KH1's movement-state cameras (ledges, climbing, poles) are KH2's follow camera");
        }
    }
    Some(())
}

/// Logged once, the first time KH2's follow camera runs with `camera_calls = kh2`.
static MODE_LOGGED: AtomicBool = AtomicBool::new(true);

/// KH2 PC's mouse for this frame (Steam: KH1's own, the same code; other builds: none).
#[derive(Clone, Copy, Default)]
struct Mouse {
    /// Its motion (x, y within ±1) when it moved this frame.
    motion: Option<[f32; 2]>,
    /// The tilt factor KH2's pad gives both sticks in a mouse frame: `1 + s · 0.02` (Steam KH2
    /// `0x14039c180`, `s` the sensitivity; KH1's own stick step has no such factor).
    tilt: f32,
    /// Keyboard / mouse mode.
    kbm: bool,
}

unsafe fn mouse(base: usize) -> Mouse {
    let none = Mouse { tilt: 1.0, ..Default::default() };
    if base == 0 || super::addr::Build::current() != super::addr::Build::Steam {
        return none;
    }
    let kbm = rd::<u32>(base + KBM_STEAM) != 0;
    if MOUSE_OK.load(Ordering::Relaxed) == 0 {
        let ok = rd::<[u8; 10]>(base + MOUSE_STEAM) == MOUSE_PROLOGUE;
        MOUSE_OK.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
        log(&format!("KH2 camera: KH1's mouse (0x140138b30) {}", if ok { "found: KH2's mouse camera" } else { "isn't where expected: the mouse as a tilted stick only" }));
    }
    if MOUSE_OK.load(Ordering::Relaxed) != 1 {
        return Mouse { kbm, ..none };
    }
    let get: unsafe extern "system" fn(*mut f32, *mut f32, *mut f32, *mut f32, *mut u16) -> i32 = std::mem::transmute(base + MOUSE_STEAM);
    let (mut x, mut y, mut a, mut b, mut s) = (0.0f32, 0.0f32, 1.0f32, 1.0f32, 0u16);
    if get(&mut x, &mut y, &mut a, &mut b, &mut s) == 0 || !x.is_finite() || !y.is_finite() {
        return Mouse { kbm, ..none };
    }
    Mouse { motion: Some([x, y]), tilt: 1.0 + s as f32 * 0.02, kbm }
}

/// The right and left sticks for KH2's camera (KH1's way: -1..1, y positive pulled back). The
/// pad's own sticks as KH2's pad builds them (`character_core::pad`, Steam KH2 `0x14039c180`: 60
/// of 127.5 off each axis, a unit direction and a tilt), 1:1 (the tester). KH1's processed sticks
/// are built the same way (Steam KH1 `0x14028b4f0`) and stand in when the pads' own rest: the
/// keyboard's keys. In a frame the mouse moved, KH2's pad writes the mouse over the right stick
/// and takes a dead zone of 1 for both: KH1's stick step does the same, so KH1's processed sticks
/// are KH2's then, times KH2's sensitivity factor (`Mouse::tilt`). `free` false (Sora unable to
/// act): no right stick.
unsafe fn sticks(base: usize, free: bool, m: &Mouse) -> ([f32; 2], [f32; 2]) {
    if m.motion.is_some() {
        let k = |v: [f32; 2]| [v[0] * m.tilt, v[1] * m.tilt];
        let r = if free { k(rd::<[f32; 2]>(base + RIGHT_STICK_RVA)) } else { [0.0; 2] };
        return (r, k(rd::<[f32; 2]>(base + LEFT_STICK_RVA)));
    }
    let raw = super::takeover::RAW_STICKS.load(Ordering::Relaxed).to_le_bytes();
    let raw_r = character_core::pad::stick_vec(raw[0], raw[1]);
    let r = if !free {
        [0.0; 2]
    } else if raw_r != [0.0; 2] {
        raw_r
    } else {
        rd::<[f32; 2]>(base + RIGHT_STICK_RVA)
    };
    let raw_l = character_core::pad::stick_vec(raw[2], raw[3]);
    let l = if raw_l != [0.0; 2] { raw_l } else { rd::<[f32; 2]>(base + LEFT_STICK_RVA) };
    (r, l)
}

/// KH2's camera hold button (bit 32 of KH2's held word; the tester: "its R2"): R2 held as KH1
/// sees it. Steam: KH1's raw pad word (`RAW_HELD_STEAM`), so the keyboard's R2 key counts; else
/// the pads' own records (`takeover::RAW_BUTTONS`). Either way Frame Advance's R2 isn't seen:
/// while it is on, `probe::pad` takes R2 off every pad record (and `RAW_BUTTONS`).
unsafe fn r2_held(base: usize) -> bool {
    if super::addr::Build::current() == super::addr::Build::Steam && base != 0 {
        rd::<u32>(base + RAW_HELD_STEAM) & RAW_R2 != 0
    } else {
        super::takeover::RAW_BUTTONS.load(Ordering::Relaxed) & RAW_R2 != 0
    }
}

/// KH1's camera left / right option's sign (+1 normal).
unsafe fn turn_sign(base: usize) -> f32 {
    if rd::<f32>(base + TURN_SIGN_RVA) < 0.0 { -1.0 } else { 1.0 }
}

/// The right stick as the camera crate's look stick (x right, y up).
fn look(r: [f32; 2], turn: f32) -> Vec2 {
    // (Negated: KH1 world turns the other way round from the camera crate's; the user: x was
    // inverted.) (Not held to 1: KH2's mouse sensitivity factor can take the tilt past it.)
    Vec2::new(-r[0] * turn, -r[1])
}

/// Writes KH2's camera's eye and look point (crate units) to KH1's internal pair, with KH1's own
/// yaw / pitch / distance kept in step, and notes it as ours.
unsafe fn write_view(base: usize, eye: Vec3, at: Vec3, s: f32) {
    LAST_MS.store(now_ms(), Ordering::Relaxed);
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
}

/// KH1's own test for being in first person (its mode 2, Steam `0x1401E2390`, and its entry from
/// mode 1, `0x1401E1920`): Sora able to act (`+0x444` bit 0 clear), in movement state 0, 8 or
/// 0x18, and the world not a special-movement one (`0x142867CDC` bit 4). KH2's own test
/// (`0x1403ba300`: the player's kind allows it, two of his status bits clear) is on KH2's player,
/// which KH1 doesn't have: KH1's is the stand-in.
unsafe fn first_person_allowed(base: usize, sora: usize) -> bool {
    let state = rd::<i32>(sora + 0x70);
    rd::<u32>(sora + 0x444) & 1 == 0 && matches!(state, 0 | 8 | 0x18) && rd::<u32>(base + FLAGS2_RVA) & 0x10 == 0
}

/// KH2's first-person view is on (`takeover`: KH2's player doesn't walk in it, `0x1403a8f00`,
/// and turns by the stick, `0x1403a9250`).
pub fn first_person() -> bool {
    FP.load(Ordering::Relaxed)
}

/// Sora's fade through KH1's own actor alpha (`+0x60`, moved by `+0x64` a frame times the frame
/// step and stopped at 0 or 1, KH1 `0x140297fe0`; KH1's own first person fades him the same way,
/// `0x1401E2390` / `0x1401E2610`), with KH2's: from 1 to 0 over 30 frames into first person, from
/// 0 to 1 over 30 out of it (`0x1403b7810`, which starts the value where it says, then steps).
unsafe fn fade(sora: usize, out: bool) {
    if out {
        // (KH1 removes an actor faded to 0 with its flag bit 3 set, `0x140292390`: not Sora,
        // but kept off then. KH1's own first person fades him all the same.)
        if rd::<u32>(sora) & 8 == 0 {
            wr(sora + 0x60, 1.0f32);
            wr(sora + 0x64, -1.0f32 / FP_FADE);
        } else {
            wr(sora, rd::<u32>(sora) & !1);
        }
    } else {
        // (KH1's draw bit back: hidden at the end of the fade out, below.)
        wr(sora, rd::<u32>(sora) | 1);
        wr(sora + 0x60, 0.0f32);
        wr(sora + 0x64, 1.0f32 / FP_FADE);
    }
}

/// Into KH2's first person (`0x140165110`: Select where the player may; the mode set to first
/// person, Sora faded out over 30 frames). KH1's mode switch follows with no blend (KH2's spring
/// eases the view into his head).
unsafe fn enter_first_person(sora: usize) {
    FP.store(true, Ordering::Relaxed);
    fade(sora, true);
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 20 {
        log(&format!("KH2 camera: KH2's first person (Sora state {})", rd::<i32>(sora + 0x70)));
    }
}

/// KH1's first-person mode (2): KH2's first person when KH2's camera entered it, else KH1's own.
unsafe extern "system" fn first_person_mode() {
    let base = BASE.load(Ordering::Relaxed);
    let entry = rd::<u32>(base + ENTRY_FLAGS_RVA);
    if !FP.load(Ordering::Relaxed) {
        let f: unsafe extern "system" fn() = std::mem::transmute(ORIG_FP.load(Ordering::Acquire));
        f();
        record(base, "first_person", "kh1", "kh1_first_person", entry);
        return;
    }
    let sora = rd::<u64>(base + SORA_RVA) as usize;
    let subject_sora = sora > 0x10000 && rd::<u64>(base + SUBJECT_RVA) as usize == sora;
    let pressed = rd::<u32>(base + BUTTONS_RVA + 8);
    // Out on Select again, or when he may no longer be in it (KH2 `0x140165e30`: `0x1403ba300`
    // false or the first-person button). R3 is KH2's level-the-view here, not KH1's way out.
    let stay = subject_sora && ours() && first_person_allowed(base, sora) && pressed & SELECT == 0;
    if !stay || run_first_person(base, sora).is_none() {
        let set: unsafe extern "system" fn(i32, f32) = std::mem::transmute(base + SET_MODE_RVA);
        // (KH1's switch calls this mode's exit, `first_person_exit`: Sora fades back in.)
        set(1, 0.0);
        record(base, "first_person", "kh2", "first_person_left", entry);
        return;
    }
    record(base, "first_person", "kh2", "", entry);
}

/// One frame of KH2's first person (`camera_calls` either way). None: leave it.
unsafe fn run_first_person(base: usize, sora: usize) -> Option<()> {
    wr(base + ENTRY_FLAGS_RVA, rd::<u32>(base + ENTRY_FLAGS_RVA) & !7);
    let s = super::settings::get().camera_scale;
    let dt = rd::<f32>(base + STEP_RVA);
    let dt = if dt.is_finite() { dt.clamp(0.0, 4.0) } else { 1.0 };
    let feet = rd::<[f32; 3]>(sora + 0x10);
    let facing = rd::<f32>(sora + 0x3C);
    if !feet.iter().all(|v| v.is_finite()) || !facing.is_finite() {
        return None;
    }
    // The form's run speed (the spring's rate) and first-person height (`prty`
    // subjective_offset, KH2 `0x1403b5c50`), else Sora's.
    let p = pose::kh2().map(|kh2| super::form::current().and_then(|k| k.data(kh2)).map_or(&kh2.character, |f| &f.character).params.clone());
    let run_speed = p.as_ref().map(|p| p.run_speed).filter(|v| v.is_finite() && *v > 0.0).unwrap_or(8.0);
    let eye_height = p.as_ref().map(|p| p.subjective_offset).filter(|v| v.is_finite() && *v > 0.0).unwrap_or(150.0);
    let subject = kh2_camera::Subject { feet: to_cam(feet, s), facing, run_speed };
    let m = mouse(base);
    let (r, _) = sticks(base, true, &m);
    let pressed = rd::<u32>(base + BUTTONS_RVA + 8);
    let ctl = kh2_camera::Controls { look_stick: look(r, turn_sign(base)), reset: pressed & R3 != 0, mouse: m.motion.map(Vec2::from), kbm: m.kbm, ..Default::default() };
    let mut g = CAM.lock().ok()?;
    let cam = g.as_mut()?;
    cam.update_first_person(&subject, eye_height, &ctl, dt);
    let (eye, at) = (cam.eye, cam.at);
    if !eye.is_finite() || !at.is_finite() || (eye - at).length() < 1.0 {
        return None;
    }
    write_view(base, eye, at, s);
    drop(g);
    set_fov(kh2_fov());
    FOV_SET.store(true, Ordering::Relaxed);
    // Faded out: KH1's own first person then clears his draw bit (his shadow and Keyblade go
    // too); ours does the same.
    if rd::<f32>(sora + 0x60) <= 0.0 && rd::<f32>(sora + 0x64) == 0.0 {
        wr(sora, rd::<u32>(sora) & !1);
    }
    Some(())
}

/// KH1's first-person mode's exit (KH1's mode switch calls it on leaving mode 2, whoever
/// switches): out of KH2's first person, Sora fades back in over 30 frames (KH2 `0x140165e30`);
/// else KH1's own exit.
unsafe extern "system" fn first_person_exit() {
    if FP.swap(false, Ordering::Relaxed) {
        let base = BASE.load(Ordering::Relaxed);
        let sora = rd::<u64>(base + SORA_RVA) as usize;
        if sora > 0x10000 {
            fade(sora, false);
        }
        return;
    }
    let f: unsafe extern "system" fn() = std::mem::transmute(ORIG_FP_EXIT.load(Ordering::Acquire));
    f();
}
