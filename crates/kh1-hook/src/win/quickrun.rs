//! KH2's Quick Run as a KH1 movement state of its own (testing branch, base Sora in KH1's own
//! states). The tester (thread quick-run-native: "do your first purposed route ... make behave
//! exactly 1:1 with kh2"): Quick Run in a free slot of KH1's state table, KH2's start, hold timer,
//! brakes, stop brake, motions 203 / 204 and invulnerable time, its speed kept in KH1's momentum
//! field `+0x3C0`, no borrow of KH2's controller. `docs/kh1-quick-run-native.md` §6.
//!
//! KH2 (Steam KH2 exe, `ACTION_AIRSLIDE`, vtable `0x1405CDB40`; Checked (decomp)):
//! - the start `0x1403A9830`: velocity = direction × `fmab +0x10`, brake `+0x14`, timer `+0xC`,
//!   stop brake `+0x18`; motion 203 with no blend; action 0x10 (a ground action: its object's
//!   `+0xC` bit 0 is clear, so the air actions' enter `0x1403CBD30` gives the Aerial Dodge back
//!   out of it); the held bit; invulnerable time `+0x1C` (`0x1403D7E40`, `+0xD70`);
//! - its update (vtable slot 7, `0x140423500`): while the timer is above 0, it counts down by the
//!   frame scale (1.0 a 60 Hz frame on PC, `0x140717480`) while Square is held (the input step
//!   `0x1403D6FC0` clears the held bit when it isn't) and goes to 0 at once when it isn't; at 0
//!   the brake becomes the stop brake and motion 204 plays (no blend), motion 4 queued after it
//!   with blend 8;
//! - its velocity (slot 8, `0x140423480`): velocity × brake, moved by velocity × dt. KH2 calls it
//!   once every two 60 Hz frames with dt 2 (the player's physics tick `0x1403B89A0`: its timer
//!   `+0xBA8`, 0 at an action change, counts down by the frame scale; below 0 the velocity step
//!   runs with 2.0 and the per-frame velocity is its result halved);
//! - motions 203 / 204 (`P_EX100.mset` A150 / A151, Checked (files)): 203 30 frames at 30 fps
//!   (60 at 60 Hz), no gravity all through (range id 3), reaction commands from frame 8 (range 14),
//!   no free state (range 0 / 1 / 2); 204 18 frames (36), no gravity 0..28, reaction commands all
//!   through, a sound at 6 (0 / 303). With no free state, KH2's input step neither jumps (slot 9,
//!   only with the motion state's bits `0x1C`, i.e. range 0 / 1 / 2) nor takes a command
//!   (`0x1403D6CA0`, the same bits) during a Quick Run: only letting go of Square ends it early.
//!
//! KH1 (Steam KH1 exe): the state table `0x142DB3840` (59 entries of 0x40: `+0` flags, `+8`
//! enter, `+0x10` exit, `+0x18` pre-step, `+0x20` step `(actor, &displacement)`, `+0x28`
//! transition `(actor, airborne) -> state`, `+0x30`, `+0x38` the jump press; registration
//! `0x1402B1B80`), Checked (decomp). Ours is `STATE` (0x2A, never registered nor compared by
//! number in KH1, `docs/kh1-quick-run-native.md` §3), written each Sora physics frame (as
//! `shared::physics::install` does for its wrappers):
//! - flags `0x08` (the platform carry, as state 0's `0xFC` has it): collision class 0 (the
//!   ground's, as KH2's ground action), and none of the bits that open KH1's commands to him
//!   (`0x10` Attack in the menu fill `0x140294220`, `0x80` the shortcuts in `0x140288F30`, `0x40`)
//!   since KH2 takes none during it;
//! - no pre-step and no jump press (KH2 doesn't jump out of it); no enter / exit (we start and
//!   end it ourselves);
//! - its step: KH2's update and velocity above, the displacement through KH1's own ground step
//!   (`0x1402B1850`: `+0x20` laid on the floor's plane, × `+0x2C` × dt), so slopes are KH1's;
//!   `+0x3C0` = the velocity (KH1's momentum field);
//! - its transition: on the ground at 204's end KH1's landing; out of our motions (KH1 put
//!   another one on him: a hit's reaction, an event) back to KH1's ground state.
//!
//! Motions 203 / 204 are our requests of KH2's motions (`kh1rows::OURS`, request only, their
//! `kh2slots` slots), asked through KH1's motion request (`0x14029F980`).

use super::addr::{Build, Rva};
use super::{log, pose, takeover};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

/// Our state's id in KH1's state table (free: never registered, never compared by number).
pub const STATE: i32 = 0x2A;
/// KH2's motions: the run and its stop (`P_EX100` A150 / A151).
pub const RUN_MOTION: u16 = 203;
pub const STOP_MOTION: u16 = 204;

const TABLE_RVA: Rva = Rva(0x2DB4240);
const SET_STATE_RVA: Rva = Rva(0x2AF7D0);
const SET_MOTION_RVA: Rva = Rva(0x29D7F0);
const QUEUE_MOTION_RVA: Rva = Rva(0x29D8A0);
/// KH1's ground step (Steam `0x1402B1850`, state 0's `+0x20`, Checked (decomp)). Steam only.
const GROUND_STEP_STEAM: usize = 0x2B1850;
const ENTRY: usize = 0x40;
/// Our state's flags: the platform carry only (see the module notes).
const FLAGS: u32 = 0x08;
/// KH2's physics tick: the velocity step runs once every two 60 Hz frames (`0x1403B89A0`).
const TICK: f32 = 2.0;
/// KH2's blend into the stop (204: none, `0x140423500`) and into what follows it (8.0).
const STOP_BLEND: f32 = 0.0;
const AFTER_BLEND: f32 = 8.0;
/// KH1's roll's timer flags (`0x1402C9E60` sets them with its 12.0 at `+0xC`).
const KH1_ROLL_FLAGS: u32 = 0x10_0080;
/// KH1's landing request (its ground jump's base 0, + 7) and its idle (`0x4000_0000` queued).
const LANDING: u32 = 0x07;

/// The Quick Run playing: KH2's action fields.
struct Run {
    /// KH2's velocity `+0x20` (x, z), per 60 Hz frame.
    vel: [f32; 2],
    /// `+0x108`, `+0x10C`, `+0x110`.
    brake: f32,
    timer: f32,
    stop_brake: f32,
    /// KH2's physics tick timer (`+0xBA8`).
    tick: f32,
    /// 60 Hz frames since the start, and KH2's invulnerable time (`+0xD70`).
    age: f32,
    star: f32,
    /// In 204 (the timer ran out or Square was let go).
    stopping: bool,
    level: u8,
}

static RUN: Mutex<Option<Run>> = Mutex::new(None);
/// Our entry was refused once (KH1 registered something in our slot): logged once.
static REFUSED: AtomicBool = AtomicBool::new(false);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn note(s: String) {
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 200 {
        log(&format!("Quick Run (native): {s}"));
    }
}

/// Our motions' requests are built (`kh1rows`, `kh2slots`) and base Sora can take them.
pub fn built() -> bool {
    Build::current() == Build::Steam
        && !super::form::in_form()
        && super::kh1rows::installed()
        && [RUN_MOTION, STOP_MOTION].iter().all(|&m| super::kh1rows::by_motion(m).is_some_and(|n| super::kh2slots::has(super::kh1rows::request(n) as usize, m)))
}

/// Puts our state in KH1's state table (each Sora physics frame: KH1 fills the table itself when
/// it sets its states up). Only into an empty slot or ours.
pub unsafe fn install(base: usize) {
    if Build::current() != Build::Steam || base == 0 {
        return;
    }
    let e = base + TABLE_RVA + STATE as usize * ENTRY;
    let step_fn = step as *const () as usize;
    let trans_fn = transition as *const () as usize;
    let words: [usize; 8] = rd(e);
    let ours = words[4] == step_fn && words[5] == trans_fn;
    let empty = words.iter().all(|&w| w == 0);
    if !(ours || empty) {
        if !REFUSED.swap(true, Ordering::Relaxed) {
            note(format!("KH1's state {STATE:#x} is in use ({:#x?}): not installed", words));
        }
        return;
    }
    if ours && rd::<u32>(e) == FLAGS {
        return;
    }
    wr(e, [FLAGS as usize, 0, 0, 0, step_fn, trans_fn, 0, 0]);
    note(format!("KH1's state {STATE:#x} is ours (flags {FLAGS:#x}, step {step_fn:#x}, transition {trans_fn:#x})"));
}

/// He is in our state now.
pub unsafe fn running(actor: usize) -> bool {
    actor > 0x10000 && rd::<i32>(actor + 0x70) == STATE
}

/// KH2's start (`0x1403A9830`) on KH1's states: from KH1's ground state (0), with Sora's Quick
/// Run level's numbers, along `dir` (x / z; his facing if it has none). Our state, KH1's request
/// of motion 203 with its action flag and no blend, his facing turned to `dir`. False (the
/// caller's borrow then) without Quick Run, our requests not built, or out of KH1's ground state.
pub unsafe fn start(actor: usize, dir: [f32; 2], why: &str) -> bool {
    if actor < 0x10000 || !built() || rd::<i32>(actor + 0x70) != 0 {
        return false;
    }
    let Some((level, g)) = takeover::kh2_quick_run_growth() else { return false };
    let Some(n) = super::kh1rows::by_motion(RUN_MOTION) else { return false };
    let request = super::kh1rows::request(n);
    let len = dir[0].hypot(dir[1]);
    let d = if len > 1e-3 {
        [dir[0] / len, dir[1] / len]
    } else {
        let f: [f32; 4] = rd(actor + 0x30);
        let l = f[0].hypot(f[2]);
        if l > 1e-3 { [f[0] / l, f[2] / l] } else { [0.0, 1.0] }
    };
    let base = takeover::game_base();
    let set_state: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + SET_STATE_RVA);
    let set: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
    install(base);
    // Out of KH1's own roll (Square held through it): its 12-frame timer's flags off, as KH1's
    // think clears them when the timer runs out (`0x140297FE0`, flags `0x80 | 0x100000`, Checked
    // (decomp)). Assumption: nothing else of KH1's roll needs ending (its motion is replaced).
    wr(actor, rd::<u32>(actor) & !KH1_ROLL_FLAGS);
    set_state(actor, STATE);
    if rd::<i32>(actor + 0x70) != STATE {
        return false;
    }
    set(actor, request | 0x0100_0000, 0.0);
    let f = d[0].atan2(d[1]);
    wr(actor + 0x30, [f.sin(), 0.0, f.cos(), f]);
    wr(actor + 0x150, f);
    let vel = [d[0] * g.quick_run_speed, d[1] * g.quick_run_speed];
    let run = Run {
        vel,
        brake: g.quick_run_brake,
        timer: g.quick_run_time,
        stop_brake: g.quick_run_stop_brake,
        tick: 0.0,
        age: 0.0,
        star: g.quick_run_star_time.max(0.0),
        stopping: false,
        level,
    };
    note(format!(
        "{why}: KH2's Quick Run LV{level} on KH1's state {STATE:#x} (request {request:#x}): speed {:.0}, brake {:.2}, time {:.0}, stop brake {:.2}, invulnerable {:.0}",
        g.quick_run_speed, run.brake, run.timer, run.stop_brake, run.star
    ));
    if let Ok(mut r) = RUN.lock() {
        *r = Some(run);
    }
    wr(actor + 0x3C0, [vel[0], 0.0, vel[1], g.quick_run_speed]);
    true
}

/// KH2's invulnerable time of the Quick Run playing (`fmab +0x1C`, its frames from the start).
pub unsafe fn invulnerable(actor: usize) -> bool {
    running(actor) && RUN.try_lock().ok().and_then(|r| r.as_ref().map(|r| r.age < r.star)).unwrap_or(false)
}

/// The Quick Run's level playing (for the higher levels' start effect).
pub fn level() -> u8 {
    RUN.try_lock().ok().and_then(|r| r.as_ref().map(|r| r.level)).unwrap_or(0)
}

/// One of our motions plays on him, or one of their requests is pending (KH1 applies a request
/// in its animation update, after the physics step: `+0x178` pending, `+0x17C` the request).
unsafe fn ours(actor: usize) -> bool {
    if matches!(super::kh2slots::playing_motion(actor), Some(RUN_MOTION | STOP_MOTION)) {
        return true;
    }
    let pending = rd::<u32>(actor + 0x178) != 0;
    let req = rd::<u32>(actor + 0x17C) & 0xFFFF;
    pending && [RUN_MOTION, STOP_MOTION].iter().any(|&m| super::kh1rows::by_motion(m).is_some_and(|n| super::kh1rows::request(n) == req))
}

/// Our state's step: KH2's update (the hold timer, the stop) and its velocity step on KH2's
/// physics tick, moved through KH1's ground step. KH1's physics step calls it.
unsafe extern "system" fn step(actor: usize, disp: *mut [f32; 4]) -> usize {
    if disp.is_null() {
        return 0;
    }
    *disp = [0.0; 4];
    let dt = super::shared::physics::dt();
    let Ok(mut g) = RUN.lock() else { return 0 };
    let Some(r) = g.as_mut() else { return 0 };
    if !ours(actor) {
        // (Not ours any more: the transition takes him back to KH1's.)
        return 0;
    }
    // KH2's update (`0x140423500`): the hold timer.
    if r.timer > 0.0 {
        if takeover::pad_square() {
            r.timer -= dt;
        } else {
            r.timer = 0.0;
        }
        if r.timer <= 0.0 {
            r.timer = 0.0;
            r.brake = r.stop_brake;
            r.stopping = true;
            if let Some(n) = super::kh1rows::by_motion(STOP_MOTION) {
                let set: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(takeover::game_base() + SET_MOTION_RVA);
                set(actor, super::kh1rows::request(n) | 0x0100_0000, STOP_BLEND);
            }
            note(format!("the stop (motion 204) after {:.0} frames, speed {:.1}", r.age, r.vel[0].hypot(r.vel[1])));
        }
    }
    // KH2's velocity step on its physics tick (`0x1403B89A0` → `0x140423480`).
    r.tick -= dt;
    while r.tick < 0.0 {
        r.vel = [r.vel[0] * r.brake, r.vel[1] * r.brake];
        r.tick += TICK;
    }
    r.age += dt;
    let speed = r.vel[0].hypot(r.vel[1]);
    let (dir, speed) = if speed > 1e-4 { ([r.vel[0] / speed, 0.0, r.vel[1] / speed, 0.0], speed) } else { ([0.0; 4], 0.0) };
    wr(actor + 0x20, dir);
    wr(actor + 0x2C, speed);
    wr(actor + 0x3C0, [r.vel[0], 0.0, r.vel[1], speed]);
    drop(g);
    let ground: unsafe extern "system" fn(usize, *mut [f32; 4]) -> usize = std::mem::transmute(takeover::game_base() + GROUND_STEP_STEAM);
    ground(actor, disp)
}

/// Our state's transition (`(actor, airborne) -> state`, after KH1's mover): on at once while our
/// motions play; at 204's end KH1's landing on the ground; else KH1's ground state (or its fall).
unsafe extern "system" fn transition(actor: usize, airborne: i32) -> u32 {
    let stopping = RUN.lock().ok().and_then(|g| g.as_ref().map(|r| r.stopping));
    if stopping.is_none() {
        // (No Quick Run of ours: KH1's own states, as its default transition.)
        return if airborne != 0 { 2 } else { 0 };
    }
    if !ours(actor) {
        let playing = super::kh2slots::playing_motion(actor);
        if let Ok(mut g) = RUN.lock() {
            *g = None;
        }
        note(format!("KH1 put another motion on him ({playing:?}): KH1's {}", if airborne != 0 { "fall" } else { "ground state" }));
        return if airborne != 0 { 2 } else { 0 };
    }
    if airborne != 0 {
        let vel = RUN.lock().ok().and_then(|mut g| g.take()).map_or([0.0; 2], |r| r.vel);
        off_ledge(actor, vel);
        return 2;
    }
    // 204 played to its end (KH2 then plays its queued fall, motion 4 with blend 8, which on the
    // floor is KH2's landing: Assumption, the end seen in play, s206).
    let end = super::kh2slots::now_playing(actor).filter(|(m, _)| *m == STOP_MOTION).and_then(|(_, t)| {
        let len = pose::kh2()?.character.moveset.clips.get(&STOP_MOTION)?.length as f32;
        Some(t + super::shared::physics::dt() >= len)
    });
    if stopping != Some(true) || end != Some(true) {
        return STATE as u32;
    }
    if let Ok(mut g) = RUN.lock() {
        *g = None;
    }
    let base = takeover::game_base();
    let set: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
    let queue: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + QUEUE_MOTION_RVA);
    wr(actor + 0x3D0, 0u32);
    wr(actor + 0x2C, 0.0f32);
    set(actor, LANDING | 0x0100_0000, AFTER_BLEND);
    queue(actor, 0x4000_0000, 0.0);
    note("the stop played out: KH1's landing (request 7)".to_string());
    0
}

/// KH1's fall's request (its ground jump's base 0, + 6) and KH2's blend into its fall (8.0, the
/// stop's queued motion 4).
const FALL: u32 = 0x06;

/// Off a ledge in a Quick Run: KH2's ground action loses the floor and goes to the fall action
/// (vtable slot 4, `0x140409B00`: `0x1403CBC30 (actor, 0)`, a fall from rest with no apex swap,
/// landing motion 5, then action 1; the slide's velocity `+0x20` left as it was; Checked
/// (decomp)), whose enter (`0x1403CBD30`) gives the Aerial Dodge back since Quick Run's action
/// isn't airborne (its object's `+0xC` bit 0, Checked (exe)). On KH1: its fall from rest as KH1's
/// `0x1402AF8F0 (actor, 0)` writes it (`+0x3B0`, `+0x3B8`, `+0x3BC`, `+0x3D0` 0, `+0x3B4` his
/// height, state 2, flag `0x10000` off), the action flag 4 off as KH1's own walk-off does, and
/// `+0x3C0` the slide's velocity uncapped (KH1's own walk-off caps it at 8, the physics step
/// `0x14029A3C0`, which runs only when the transition changes the state: we change it here, so it
/// sees state 2 already). KH1's fall request with KH2's blend 8 (Assumption: KH2's motion off a
/// ledge; the fall action itself asks none, so KH2 may keep 203 / 204 playing in the air: open).
/// The Aerial Dodge is still there (`shared::AIR_DODGED` is cleared on the ground before).
unsafe fn off_ledge(actor: usize, vel: [f32; 2]) {
    let base = takeover::game_base();
    let set_state: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + SET_STATE_RVA);
    let set: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
    let y = rd::<[f32; 3]>(actor + 0x10)[1];
    wr(actor + 0x3B0, 0.0f32);
    wr(actor + 0x3B4, y);
    wr(actor + 0x3B8, 0.0f32);
    wr(actor + 0x3BC, 0.0f32);
    wr(actor + 0x3D0, 0u32);
    set_state(actor, 2);
    wr(actor, rd::<u32>(actor) & !(0x1_0000 | 4));
    let speed = vel[0].hypot(vel[1]);
    wr(actor + 0x3C0, [vel[0], 0.0, vel[1], speed]);
    set(actor, FALL, AFTER_BLEND);
    note(format!("off the ground: KH1's fall at the slide's speed {speed:.1} (KH2's fall from rest, the Aerial Dodge left)"));
}
