//! KH2's glide in KH1's own glide state 0x15 (testing branch, base Sora in KH1's own states, Glide
//! at KH2). The tester (thread kh2-glide-in-kh1s-glide-state: "go ahead", the hover a sub-mode of
//! 0x15, a glide into a wall slides along it, Donald and Goofy glide after him with KH2's Glide):
//! KH2's start, speed, fall, turn, banking, hover and release run inside KH1's state 0x15 through
//! wrappers on its entry in KH1's state table, for Sora only (Donald and Goofy run the same
//! handlers with KH1's glide). KH1's mover and collision move him; no borrow of KH2's controller.
//! `docs/kh1-glide-native.md` §8.
//!
//! KH1 (Steam KH1 exe, Checked (decomp)): the state table (Steam `0x142DB3840`, 0x40 bytes a
//! state, registered by `0x1402B1B80`: `+0` flags, `+8` enter, `+0x10` exit, `+0x18` pre-step
//! `(actor, input)`, `+0x20` step `(actor, &displacement)`, `+0x28` transition `(actor, airborne)
//! -> state`, `+0x30` the button check `(actor, buttons, input)` (only from the player's control
//! step `0x1402A7B30`), `+0x38` the jump press). State 0x15 (`0x1402B26F0`): enter `0x1402B2D00`
//! (KH1's 46-frame minimum glide in the combat state's `+0x100`, `+0x1CC` bit 0 off), exit
//! `0x1402B29F0` (its 24-frame re-glide wait in `+0x100`, `+0x1CC` bit 0 on), pre-step
//! `0x1402B2880`, step `0x1402B2A20`, transition `0x1402B2780`, button check `0x1402B2BF0`, no jump
//! press. The fall state's jump press (`0x1402AFD60`, state 2's `+0x38`) starts KH1's Glide when
//! Sora's ability bits (`0x142D5EC10`) have `0x80`, or Neverland's flight (world flag `0x20`).
//!
//! KH2 (Steam KH2 exe, `ACTION_GLIDE` vtable `0x1405CDBF0`, `ACTION_GLIDEHOVER` `0x1405CDB98`,
//! Checked (decomp)): no minimum time and no re-glide wait (none in either action's functions).

use super::addr::{Build, Rva};
use super::styles::{self, Kind};
use super::{log, takeover};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

/// KH1's glide state.
pub const STATE: i32 = 0x15;
/// KH1's fall state.
const FALL_STATE: i32 = 2;

const TABLE_RVA: Rva = Rva(0x2DB4240);
const ENTRY: usize = 0x40;
const ENTER: usize = 0x08;
const EXIT: usize = 0x10;
const PRE_STEP: usize = 0x18;
const STEP: usize = 0x20;
const TRANSITION: usize = 0x28;
const BUTTONS: usize = 0x30;
const JUMP_PRESS: usize = 0x38;
const SET_STATE_RVA: Rva = Rva(0x2AF7D0);
/// KH1's motion request that leaves a motion alone when its slot is already playing (Steam
/// `0x14029F7C0`, the one KH1's glide pre-step asks with every frame; Checked (decomp)).
const REQUEST_RVA: Rva = Rva(0x29D630);
/// KH1's world flags (Steam `0x142867364`): `0x20` Neverland's flight.
const WORLD_FLAGS_RVA: Rva = Rva(0x2867CD8);
const NEVERLAND: u32 = 0x20;

/// KH2's physics tick: the glide's velocity step runs once every two 60 Hz frames with dt 2
/// (`0x1403B89A0`; per-frame velocity its result halved, as `quickrun`).
const TICK: f32 = 2.0;
/// The hover's step (`0x140423650`): his velocity x 0.9 a tick.
const HOVER_DRAG: f32 = 0.9;

/// KH1's glide request (slow; `0x1402B2D30` asks it at the start, blend 16) and KH2's blend for
/// its glide motions (16.0: `0x140423A70`, `0x140423840`).
const GLIDE_REQUEST: u32 = 0x71;
const BLEND: f32 = 16.0;
/// KH1's glide requests for KH2's motions (`kh2slots`: at Glide KH2 they play KH2's): 0x71 the
/// glide (205), 0x73 / 0x74 KH1's two banking requests (206 / 207). The hover (208) is a request of
/// ours (`kh1rows`), else KH1's 0x71.
const TURN_A_REQUEST: u32 = 0x73;
const TURN_B_REQUEST: u32 = 0x74;
/// KH2's banking test (`0x140423A70`): the stick more than this off his facing (a dot).
const BANK_DOT: f32 = 0.98;
/// KH1's facing set (Steam `0x140297E10 (actor, angle)`: `+0x30..+0x3C`, not while `+0x374` bit
/// `0x1000` holds it; the glide pre-step's own call) and its water check from the glide (Steam
/// `0x1402C12B0 (actor, input)`: water ends the glide into the swim state 0x11 with its splash, not
/// in world 0xD room 8). Checked (decomp). Steam only.
const SET_FACING_STEAM: usize = 0x297E10;
const WATER_STEAM: usize = 0x2C12B0;
const SLOT_TIME: usize = 0x16C;
/// KH1's fall from rest (Steam `0x1402AF8F0 (actor, rise)` with 0: `+0x3B0` / `+0x3B8` / `+0x3BC`
/// / `+0x3D0` 0, `+0x3B4` his height, state 2, flag `0x10000` off) and Neverland's flight-room test
/// (Steam `0x1402B1DA0 (actor)`, the glide transition's own). Checked (decomp). Steam only.
const FALL_FROM_REST_STEAM: usize = 0x2AF8F0;
const FLIGHT_ROOM_STEAM: usize = 0x2B1DA0;
/// KH1's fall request (its ground jump's base 0, + 6), with KH2's blend into its fall motion 4
/// (16.0, `0x1404235B0`).
const FALL_REQUEST: u32 = 0x06;
/// KH1's flight state and the action flag its glide transition clears going into it.
const FLIGHT_STATE: u32 = 8;
const ACTION_FLAG: u32 = 4;

/// KH1's combat state's glide timer (`[+0x148] +0x100`) and `+0x1CC` bit 0 (enter clears it, exit
/// sets it).
const TIMER: usize = 0x100;
const LEVEL_FLAGS: usize = 0x1CC;

/// The glide playing (KH2's action fields).
pub(crate) struct Glide {
    /// KH2's velocity `+0x20` (x, z), per 60 Hz frame.
    pub vel: [f32; 2],
    /// KH2's stored launch speed (`+0xE0` with its flag) at the start: the hover puts it back when
    /// the stick takes it into the glide (`0x140423840` -> the glide's enter `0x1403CBD30`).
    pub launch: Option<[f32; 2]>,
    /// The hover (ACTION_GLIDEHOVER, motion 208): `None` until the first pre-step decides it (the
    /// stick neutral at the start, `0x1403A8369`).
    pub hover: Option<bool>,
    /// KH2's physics tick timer (`+0xBA8`).
    pub tick: f32,
    /// KH2's fall this tick (per 60 Hz frame).
    pub fall: f32,
}

pub(crate) static RUN: Mutex<Option<Glide>> = Mutex::new(None);

static ORIG_ENTER: AtomicUsize = AtomicUsize::new(0);
static ORIG_EXIT: AtomicUsize = AtomicUsize::new(0);
static ORIG_PRE: AtomicUsize = AtomicUsize::new(0);
static ORIG_STEP: AtomicUsize = AtomicUsize::new(0);
static ORIG_TRANSITION: AtomicUsize = AtomicUsize::new(0);
static ORIG_BUTTONS: AtomicUsize = AtomicUsize::new(0);
static ORIG_FALL_JUMP: AtomicUsize = AtomicUsize::new(0);

pub(crate) unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

pub(crate) unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

pub(crate) fn note(s: String) {
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 300 {
        log(&format!("glide (native): {s}"));
    }
}

/// Base Sora can take KH2's glide in KH1's state (the Steam build: the KH1 addresses below are its
/// own; not in a Drive Form, whose moveset is KH2's controller's).
pub fn built() -> bool {
    Build::current() == Build::Steam && !super::form::in_form()
}

/// The wrappers act for `actor`: Sora, at Glide KH2, base Sora.
pub(crate) unsafe fn ours(actor: usize) -> bool {
    actor > 0x10000 && actor == takeover::sora_actor() && styles::kh2(Kind::Glide) && built()
}

/// He is in KH1's glide state with KH2's glide (ours) now.
pub unsafe fn running(actor: usize) -> bool {
    ours(actor) && rd::<i32>(actor + 0x70) == STATE && RUN.try_lock().ok().is_some_and(|g| g.is_some())
}

/// Puts our wrappers in KH1's state table (each Sora physics frame: KH1 fills the table itself when
/// it sets its states up, which would drop them). Steam only.
pub unsafe fn install(base: usize) {
    if Build::current() != Build::Steam || base == 0 {
        return;
    }
    let table = base + TABLE_RVA;
    let slots: [(usize, &AtomicUsize, usize); 7] = [
        (table + STATE as usize * ENTRY + ENTER, &ORIG_ENTER, enter as *const () as usize),
        (table + STATE as usize * ENTRY + EXIT, &ORIG_EXIT, exit as *const () as usize),
        (table + STATE as usize * ENTRY + PRE_STEP, &ORIG_PRE, pre_step as *const () as usize),
        (table + STATE as usize * ENTRY + STEP, &ORIG_STEP, step as *const () as usize),
        (table + STATE as usize * ENTRY + TRANSITION, &ORIG_TRANSITION, transition as *const () as usize),
        (table + STATE as usize * ENTRY + BUTTONS, &ORIG_BUTTONS, buttons as *const () as usize),
        (table + FALL_STATE as usize * ENTRY + JUMP_PRESS, &ORIG_FALL_JUMP, fall_jump as *const () as usize),
    ];
    wrap(&slots);
    party(base);
}

/// Sora's ability bits (Steam `0x142D5EC10`, rebuilt from KH1's menu by `0x1402A6380` when a menu
/// closes or a map is set up: `0x80` KH1's Glide, `0x100` its Superglide). Checked (decomp).
const ABILITY_BITS_RVA: Rva = Rva(0x2D5F610);
const KH1_GLIDE_BIT: u32 = 0x80;
const KH1_SUPERGLIDE_BIT: u32 = 0x100;
/// We set `KH1_GLIDE_BIT` (Sora doesn't have KH1's Glide himself).
static PARTY_BIT: AtomicBool = AtomicBool::new(false);

/// Donald and Goofy glide after Sora whenever he can glide with KH2's (the tester, "(b)"). Their AI
/// (`0x1401CF9E0`, members 1 and 2 only, from the party think `0x1402A3300` -> `0x1401CDBD0`)
/// follows a jump of his with KH1's Glide (`0x1402B26C0`) or Superglide (`0x1402B2750`) on their own
/// bodies only when Sora's ability bits have `0x80` / `0x100`, and its glide steering ends it when
/// they catch up (`0x1401CD050`). So at Glide KH2, with KH2's Glide (any level) and none of KH1's,
/// `0x80` is set each Sora physics frame (KH1 clears it when it rebuilds the bits), and taken off
/// again (only if we set it and he still lacks KH1's Glide) when he can't glide with KH2's. Their
/// glide stays KH1's (KH1's handlers run for them). Sora's own reader of `0x80`, the fall state's
/// jump press (`0x1402AFD60`), is ours at Glide KH2 (`fall_jump`: no KH1 glide on his jump).
/// Checked (decomp).
unsafe fn party(base: usize) {
    let p = base + ABILITY_BITS_RVA;
    let v: u32 = rd(p);
    let want = styles::kh2(Kind::Glide) && built() && takeover::kh2_glide_growth().is_some();
    if want {
        if v & (KH1_GLIDE_BIT | KH1_SUPERGLIDE_BIT) == 0 {
            wr(p, v | KH1_GLIDE_BIT);
            if !PARTY_BIT.swap(true, Ordering::Relaxed) {
                note("Donald and Goofy glide after him (KH1's Glide bit set for their AI: Sora has KH2's Glide)".to_string());
            }
        }
    } else if PARTY_BIT.swap(false, Ordering::Relaxed) && v & KH1_GLIDE_BIT != 0 && !takeover::kh1_has_glide(base) {
        wr(p, v & !KH1_GLIDE_BIT);
        note("Donald and Goofy's glide back to KH1's rule (Sora has no glide of KH2's here)".to_string());
    }
}

pub(crate) unsafe fn wrap(slots: &[(usize, &AtomicUsize, usize)]) {
    for &(slot, orig, mine) in slots {
        let now: usize = rd(slot);
        if now != mine && now > 0x10000 {
            orig.store(now, Ordering::Release);
            wr(slot, mine);
            note(format!("KH1's state handler at {slot:#x} wrapped ({now:#x})"));
        }
    }
}

/// The combat state's glide timer (KH1's minimum glide / re-glide wait): 0 at Glide KH2, KH2 has
/// neither.
unsafe fn clear_timer(actor: usize) {
    let cs = super::pose::ptr(rd::<u32>(actor + 0x148));
    if cs > 0x10000 {
        wr(cs + TIMER, 0.0f32);
    }
}

type ActorFn = unsafe extern "system" fn(usize) -> usize;

/// State 0x15's enter: KH1's (`0x1402B2D00`: its 46-frame minimum glide, `+0x1CC` bit 0 off) for
/// Donald and Goofy; for Sora at Glide KH2 the bit as KH1 has it and no minimum (KH2 has none).
/// A glide KH1 started on him itself (not `start`: its Superglide on Square, `0x1402A7B30`) is
/// taken as KH2's start.
unsafe extern "system" fn enter(actor: usize) -> usize {
    if !ours(actor) {
        let f: ActorFn = std::mem::transmute(ORIG_ENTER.load(Ordering::Acquire));
        return f(actor);
    }
    clear_timer(actor);
    wr(actor + LEVEL_FLAGS, rd::<u32>(actor + LEVEL_FLAGS) & !1);
    if let Ok(mut g) = RUN.lock() {
        if g.is_none() {
            *g = Some(begin(actor));
            note("KH1 started its glide on him itself: KH2's glide from here".to_string());
        }
    }
    0
}

/// State 0x15's exit: KH1's (`0x1402B29F0`: its 24-frame re-glide wait, `+0x1CC` bit 0 on) for
/// Donald and Goofy; for Sora at Glide KH2 the bit as KH1 has it and no wait (KH2 has none), and
/// our glide over.
unsafe extern "system" fn exit(actor: usize) -> usize {
    if !ours(actor) {
        let f: ActorFn = std::mem::transmute(ORIG_EXIT.load(Ordering::Acquire));
        return f(actor);
    }
    clear_timer(actor);
    wr(actor + LEVEL_FLAGS, rd::<u32>(actor + LEVEL_FLAGS) | 1);
    if let Ok(mut g) = RUN.lock() {
        if g.take().is_some() {
            note(format!("out of KH1's glide state (now state {})", rd::<i32>(actor + 0x70)));
        }
    }
    0
}

/// The fall state's jump press (`0x1402AFD60`): KH1's for Donald and Goofy, and Neverland's flight
/// for everyone; for Sora at Glide KH2 never KH1's glide on the jump (KH2's glide is on Square, the
/// jump in the air its Aerial Dodge).
unsafe extern "system" fn fall_jump(actor: usize) -> usize {
    let neverland = rd::<u32>(takeover::game_base() + WORLD_FLAGS_RVA) & NEVERLAND != 0;
    if !ours(actor) || neverland {
        let f: ActorFn = std::mem::transmute(ORIG_FALL_JUMP.load(Ordering::Acquire));
        return f(actor);
    }
    0
}

/// KH2's start on his speed now: the stored launch speed put back when KH2 has one (`0x1403CBD30`:
/// `+0x20 = +0xE0` with its flag; ours `takeover::launch_kept`), else his velocity as it is.
unsafe fn begin(actor: usize) -> Glide {
    let v: [f32; 4] = rd(actor + 0x3C0);
    let launch = takeover::launch_kept();
    let vel = launch.unwrap_or([v[0], v[2]]);
    Glide { vel: if vel.iter().all(|c| c.is_finite()) { vel } else { [0.0; 2] }, launch, hover: None, tick: 0.0, fall: 0.0 }
}

/// KH2's glide start (`0x1403A8369`, the air action button) on KH1's states: from KH1's jump or
/// fall state (1 / 2), above KH2's start height for Sora's Glide level (growth `+0x34`; KH1's own
/// 50 not used), with no minimum time or wait (KH1's timer not read) and his speed not halved
/// (KH1's start halves it). KH1's glide state, its slow glide request (KH2's 205, or the hover
/// 208, picked by the stick in the first pre-step). False (nothing done) otherwise.
pub unsafe fn start(actor: usize, why: &str) -> bool {
    if actor < 0x10000 || !ours(actor) || !matches!(rd::<i32>(actor + 0x70), 1 | 2) {
        return false;
    }
    let Some((level, g)) = takeover::kh2_glide_growth() else { return false };
    let above = rd::<f32>(actor + 0x3D4);
    if !(above > g.glide_start_height) {
        return false;
    }
    let glide = begin(actor);
    let speed = glide.vel[0].hypot(glide.vel[1]);
    let launched = glide.launch.is_some();
    if let Ok(mut l) = RUN.lock() {
        *l = Some(glide);
    }
    let base = takeover::game_base();
    let set_state: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + SET_STATE_RVA);
    set_state(actor, STATE);
    if rd::<i32>(actor + 0x70) != STATE {
        if let Ok(mut l) = RUN.lock() {
            *l = None;
        }
        return false;
    }
    let request: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + REQUEST_RVA);
    request(actor, GLIDE_REQUEST, BLEND);
    note(format!(
        "{why}: KH2's Glide LV{level} in KH1's state {STATE:#x}, {above:.0} above the floor (start height {:.0}), speed {speed:.1} ({})",
        g.glide_start_height,
        if launched { "KH2's stored launch speed" } else { "his speed as it was" }
    ));
    true
}

type StepFn = unsafe extern "system" fn(usize, *mut [f32; 4]) -> usize;

/// State 0x15's step (`(actor, &displacement)`, before KH1's mover): KH1's (`0x1402B2A20`: its +0.4
/// a frame speed ramp to 8 / 16, its saw-tooth fall) for Donald and Goofy; for Sora KH2's, on KH2's
/// physics tick:
/// - the glide (`0x140423920`): velocity = his facing x `(1 - a) * speed + a * |v|` (growth
///   `+0x30` a, `+0x20` speed), whatever the stick's tilt;
/// - the hover (`0x140423650`): velocity x 0.9;
/// - both: the fall `(height over the floor - +0x28) * +0x24`, at most `+0x2C`, 0 below `+0x28`
///   (`0x1404237A0`), the height KH1's own (`+0x3D4`).
///
/// The frame's displacement is that velocity (x / z) and the fall (y, KH1's y-down) times KH1's
/// frame step; KH1's mover and collision move him by it (a wall: he slides along it, as KH2's
/// glide has no wall rule). `+0x3C0` / `+0x3C4` (KH1's air speed, read by its fall state after)
/// carry KH2's velocity, as KH1's step writes them.
unsafe extern "system" fn step(actor: usize, disp: *mut [f32; 4]) -> usize {
    let mine = ours(actor) && !disp.is_null() && RUN.try_lock().ok().is_some_and(|g| g.is_some());
    let g = if mine { takeover::kh2_glide_growth() } else { None };
    let Some((_, g)) = g else {
        let f: StepFn = std::mem::transmute(ORIG_STEP.load(Ordering::Acquire));
        return f(actor, disp);
    };
    let dt = super::shared::physics::dt();
    let above = rd::<f32>(actor + 0x3D4);
    let facing: [f32; 4] = rd(actor + 0x30);
    let Ok(mut lock) = RUN.lock() else { return 0 };
    let Some(r) = lock.as_mut() else { return 0 };
    r.tick -= dt;
    while r.tick < 0.0 {
        if r.hover == Some(true) {
            r.vel = [r.vel[0] * HOVER_DRAG, r.vel[1] * HOVER_DRAG];
        } else {
            let a = g.glide_acceleration;
            let speed = (1.0 - a) * g.glide_speed + a * r.vel[0].hypot(r.vel[1]);
            r.vel = [facing[0] * speed, facing[2] * speed];
        }
        r.fall = if above.is_finite() { ((above.max(0.0) - g.glide_fall_height).max(0.0) * g.glide_fall_ratio).min(g.glide_fall_max) } else { 0.0 };
        r.tick += TICK;
    }
    let (vel, fall) = (r.vel, r.fall);
    drop(lock);
    *disp = [vel[0] * dt, fall * dt, vel[1] * dt, 0.0];
    wr(actor + 0x3C0, [vel[0], fall, vel[1], vel[0].hypot(vel[1])]);
    0
}

type PreFn = unsafe extern "system" fn(usize, usize) -> usize;

fn wrap_angle(a: f32) -> f32 {
    let mut a = a % std::f32::consts::TAU;
    if a > std::f32::consts::PI {
        a -= std::f32::consts::TAU;
    } else if a < -std::f32::consts::PI {
        a += std::f32::consts::TAU;
    }
    a
}

/// The request that plays KH2's `motion` here (`kh2slots`).
fn request_for(motion: u16) -> u32 {
    use character_core::motion::{GLIDE_HOVER, GLIDE_TURN_A, GLIDE_TURN_B};
    match motion {
        GLIDE_TURN_A => TURN_A_REQUEST,
        GLIDE_TURN_B => TURN_B_REQUEST,
        GLIDE_HOVER => super::kh1rows::by_motion(GLIDE_HOVER)
            .map(super::kh1rows::request)
            .filter(|&r| super::kh1rows::installed() && super::kh2slots::has(r as usize, GLIDE_HOVER))
            .unwrap_or(GLIDE_REQUEST),
        _ => GLIDE_REQUEST,
    }
}

/// State 0x15's pre-step (`(actor, input)`, KH1's think, before the step): KH1's (`0x1402B2880`:
/// its turn of 2° / 1° a frame, its motion picks at 30°) for Donald and Goofy; for Sora KH2's:
/// - the hover (`0x140423840`): the stick takes it into the glide, KH2's stored launch speed put
///   back (the glide's enter `0x1403CBD30`); the glide never goes back to the hover;
/// - the turn (the glide's input step `0x140423BC0` at the turn speed its enter set, growth
///   `+0x3C`, `0x140423B90`): toward the stick, at most that much a 60 Hz frame;
/// - the motion (`0x140423A70`): 205, or 206 / 207 when the stick is more than dot 0.98 off his
///   facing (206 with the turn's cross product up in KH2's y-down world, so the stick at a positive
///   angle from his facing, as KH1's), 208 in the hover, blend 16 (`0x140423840`), each frame (a
///   motion already playing is left alone, as KH1's request does);
/// - then KH1's water check, as its pre-step ends.
unsafe extern "system" fn pre_step(actor: usize, input: usize) -> usize {
    let mine = ours(actor) && RUN.try_lock().ok().is_some_and(|g| g.is_some());
    let g = if mine { takeover::kh2_glide_growth() } else { None };
    let Some((_, g)) = g else {
        let f: PreFn = std::mem::transmute(ORIG_PRE.load(Ordering::Acquire));
        return f(actor, input);
    };
    use character_core::motion::{GLIDE, GLIDE_HOVER, GLIDE_TURN_A, GLIDE_TURN_B};
    let base = takeover::game_base();
    let dt = super::shared::physics::dt();
    let stick = takeover::stick_now(actor);
    let tilt = stick[0].hypot(stick[1]);
    let moving = tilt > 0.0;
    let hover = {
        let Ok(mut lock) = RUN.lock() else { return 0 };
        let Some(r) = lock.as_mut() else { return 0 };
        let was = r.hover;
        let hover = was.unwrap_or(!moving) && !moving;
        if was == Some(true) && !hover {
            if let Some(v) = r.launch {
                r.vel = v;
            }
            note("the stick takes the hover into the glide".to_string());
        } else if was.is_none() {
            note(if hover { "the stick neutral: the hover (208)".to_string() } else { "the glide (205)".to_string() });
        }
        r.hover = Some(hover);
        hover
    };
    let mut facing: f32 = rd(actor + 0x3C);
    if !hover && moving {
        let want = stick[0].atan2(stick[1]);
        let most = (g.glide_turn_speed * dt).max(0.0);
        let f = wrap_angle(facing + wrap_angle(want - facing).clamp(-most, most));
        let set: unsafe extern "system" fn(usize, f32) = std::mem::transmute(base + SET_FACING_STEAM);
        set(actor, f);
        wr(actor + 0x150, f);
        facing = rd(actor + 0x3C);
    }
    let motion = if hover {
        GLIDE_HOVER
    } else if moving {
        let (fx, fz) = (facing.sin(), facing.cos());
        let (sx, sz) = (stick[0] / tilt, stick[1] / tilt);
        if fx * sx + fz * sz < BANK_DOT {
            if fz * sx - fx * sz > 0.0 { GLIDE_TURN_A } else { GLIDE_TURN_B }
        } else {
            GLIDE
        }
    } else {
        GLIDE
    };
    let request: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + REQUEST_RVA);
    request(actor, request_for(motion), BLEND);
    // (The hover's slot is made on an air swing of KH1's, which holds at its end: kept going round
    // as KH2 loops it. KH1's glide slots loop as KH1's own glide has them.)
    if let Some((GLIDE_HOVER, t)) = super::kh2slots::now_playing(actor) {
        let len = super::pose::kh2().and_then(|k| k.character.moveset.clips.get(&GLIDE_HOVER).map(|c| c.length as f32));
        if len.is_some_and(|len| len > 1.0 && t + dt >= len) {
            wr(actor + SLOT_TIME, 0.0f32);
        }
    }
    let water: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + WATER_STEAM);
    water(actor, input);
    0
}

/// KH2's glide ends into the fall (`0x1403CBC30`, from the glide's end check `0x1404235B0` or on
/// letting go): his velocity kept, the launch flag cleared, no hop and no speed cut (KH1's release
/// `0x1402B2670` takes x0.7 and hops 8). On KH1: its fall from rest (`0x1402AF8F0` with 0, state 2;
/// our exit runs), `+0x3C0` his velocity, KH1's fall request with KH2's blend 16 (KH2's fall
/// motion 4). Nothing if he isn't in KH2's glide.
pub unsafe fn end(actor: usize, why: &str) -> bool {
    let Some(r) = RUN.lock().ok().and_then(|mut g| g.take()) else { return false };
    if !ours(actor) || rd::<i32>(actor + 0x70) != STATE {
        return false;
    }
    let base = takeover::game_base();
    let fall: unsafe extern "system" fn(usize, f32) -> u64 = std::mem::transmute(base + FALL_FROM_REST_STEAM);
    fall(actor, 0.0);
    let speed = r.vel[0].hypot(r.vel[1]);
    wr(actor + 0x3C0, [r.vel[0], 0.0, r.vel[1], speed]);
    let request: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + REQUEST_RVA);
    request(actor, FALL_REQUEST, BLEND);
    takeover::launch_used();
    note(format!("{why}: KH2's fall at speed {speed:.1} ({:.0} above the floor; KH1 state now {})", rd::<f32>(actor + 0x3D4), rd::<i32>(actor + 0x70)));
    true
}

type ButtonsFn = unsafe extern "system" fn(usize, usize, usize) -> usize;

/// State 0x15's button check (`(actor, buttons, input)`, the player's control step only): KH1's
/// (`0x1402B2BF0`: its 46-frame minimum, then its release when the button lets go) for Sora at
/// Glide KH1; for Sora at Glide KH2 KH2's: Square let go ends the glide (and the hover) into the
/// fall at once, with no minimum time (the KH2 rewrite's glide button, t327; KH2's controller ends
/// it so, `character-core`'s glide step, matched to the exe in t331).
unsafe extern "system" fn buttons(actor: usize, held: usize, input: usize) -> usize {
    if !running(actor) {
        let f: ButtonsFn = std::mem::transmute(ORIG_BUTTONS.load(Ordering::Acquire));
        return f(actor, held, input);
    }
    if !takeover::pad_square() {
        end(actor, "Square let go");
    }
    0
}

type TransitionFn = unsafe extern "system" fn(usize, i32) -> u32;

/// State 0x15's transition (`(actor, airborne) -> state`, after KH1's mover): KH1's
/// (`0x1402B2780`) for Donald and Goofy; for Sora at Glide KH2:
/// - Neverland's flight first, as KH1's (world flag `0x20` and its flight-room test: the action
///   flag 4 off, state 8);
/// - on the ground, or under KH2's end height over the floor (growth `+0x38`), KH2's end
///   (`0x1404235B0`) into the fall (on the ground KH1's fall state lands him the next frame, its
///   landing request, as KH2's fall action lands him);
/// - else still gliding. KH1's own end on a wall (the frame moved less than 70% of the last one)
///   isn't run: KH2's glide has no wall rule, he slides along it (§7).
unsafe extern "system" fn transition(actor: usize, airborne: i32) -> u32 {
    let g = if running(actor) { takeover::kh2_glide_growth() } else { None };
    let Some((_, g)) = g else {
        let f: TransitionFn = std::mem::transmute(ORIG_TRANSITION.load(Ordering::Acquire));
        return f(actor, airborne);
    };
    let base = takeover::game_base();
    let flight_room: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(base + FLIGHT_ROOM_STEAM);
    if rd::<u32>(base + WORLD_FLAGS_RVA) & NEVERLAND != 0 && flight_room(actor) as u32 != 0 {
        wr(actor, rd::<u32>(actor) & !ACTION_FLAG);
        note("Neverland: KH1's flight from the glide".to_string());
        return FLIGHT_STATE;
    }
    let above = rd::<f32>(actor + 0x3D4);
    if airborne == 0 {
        end(actor, "on the ground");
        return FALL_STATE as u32;
    }
    if above.is_finite() && above < g.glide_end_height {
        end(actor, &format!("under the end height {:.0}", g.glide_end_height));
        return FALL_STATE as u32;
    }
    STATE as u32
}
