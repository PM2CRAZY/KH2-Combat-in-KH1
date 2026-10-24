//! Stage 8: KH2's controller drives Sora (movement first: walk, run, jump, high jump, glide,
//! aerial dodge, dodge roll, quick run, guard), with KH1's own collision underneath.
//!
//! From Ghidra (the user's Epic exe) and the stage 7 probe:
//! - `0x140298230` `(actor)` is an actor's physics step: a displacement from its movement state
//!   (handlers registered by `0x1402AF9F0`, 59 states, the state at `actor + 0x70`), platform
//!   carry, then `0x140297710` `(actor, displacement vec4, 0)` moves it with the level's
//!   collision (`0x140299350` sub-steps the move through the wall solver `0x140298A70`) and
//!   records the floor under it (`+0x320` point, `+0x3D4` height above it), then the state's
//!   transition handler runs. The probe's watchpoints: Sora's position (`+0x10`) is written
//!   about 57 times a second by that move, nothing else.
//! - `0x142D37C80`: the party's actors, Sora first. `0x14298D3A0` is set while an event runs
//!   (the think step `0x1402957F0` forces its event state then).
//! - Facing: `+0x30` forward (sin, 0, cos), `+0x3C` yaw, `+0x150` the yaw it turns toward;
//!   `+0x20` / `+0x2C` the move direction and its speed, written by KH1's input before the
//!   physics step (the probe's dumps).
//!
//! When it is Sora's step, no event runs and KH1 shows him standing, walking or running (KH1's
//! slots 0-7: anything else is KH1's own action, which keeps control until it ends), KH2's
//! controller takes the frame: the stick's direction from KH1's move direction and its tilt
//! from the pad, jump on Circle, KH2's action button (guard / dodge roll / quick run / aerial
//! dodge / glide) on Square; KH2's displacement goes through KH1's mover; landing and steps
//! come from the floor it reports. KH1 never sees Circle or Square meanwhile (`probe::pad`).
//! The pose drawn is KH2's animation player's (`POSE`, read by `pose`).

use super::addr::Rva;
use super::{combat, hook, log, menu, pose};
use character_core::{Button, Character, Env, EventKind, Input, Output, StateName, TargetInfo};
use glam::Mat4;
use kh2_rig::RigAnim;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

const PHYS_RVA: Rva = Rva(0x298230);
const PHYS_PROLOGUE: [u8; 12] = [0x40, 0x55, 0x53, 0x41, 0x56, 0x48, 0x8B, 0xEC, 0x48, 0x83, 0xEC, 0x50];
const MOVE_RVA: Rva = Rva(0x297710);
/// KH1's motion request on an actor (`0x14029D7F0 (actor, request, blend)`).
const SET_MOTION_RVA: Rva = Rva(0x29D7F0);
const PARTY_RVA: Rva = Rva(0x2D37C80);
const EVENT_RVA: Rva = Rva(0x298D3A0);
const BATTLE_RVA: Rva = Rva(0x2867CD8);
/// KH1's move vector this frame (the stick through its camera, `w` the tilt): written by its
/// think (`0x1402A4090` from `0x14028A650`).
const MOVE_VEC_RVA: Rva = Rva(0x2D5F630);

/// KH1's camera angles its think turns the stick by (`0x14028A650`, its usual camera modes):
/// the stick rotated by minus the first (wrapped), x mirrored, then rotated by the second.
const CAM_A_RVA: Rva = Rva(0x299C504);
const CAM_B_RVA: Rva = Rva(0x299CB94);
/// KH1's camera mode (2 and 0xB turn the stick otherwise).
const CAM_MODE_RVA: Rva = Rva(0x299C508);

/// Which of the 8 sign conventions (stick x, stick y, rotation) matches KH1's own move vector,
/// learned on frames that have one: a running score per convention (lower is better).
static STICK_FIT: Mutex<[f32; 8]> = Mutex::new([0.0; 8]);

fn stick_through_camera(c: usize, lx: f32, ly: f32, a: f32, b: f32) -> glam::Vec2 {
    let u = if c & 1 != 0 { -1.0 } else { 1.0 };
    let t = if c & 2 != 0 { -1.0 } else { 1.0 };
    let s = if c & 4 != 0 { -1.0 } else { 1.0 };
    let rot = |v: glam::Vec2, ang: f32| {
        let (sn, cs) = (ang * s).sin_cos();
        glam::Vec2::new(v.x * cs + v.y * sn, -v.x * sn + v.y * cs)
    };
    let v = rot(glam::Vec2::new(u * lx, t * ly), -a);
    let v = glam::Vec2::new(-v.x, v.y);
    rot(v, b).normalize_or_zero()
}

/// The stick's direction in the world: KH1's move vector (`MOVE_VEC_RVA`), not his `+0x20`,
/// which KH1's own walk only turns toward the stick slowly. Under KH1's own dead zone (a slow
/// walk, which KH1 still shows as idle) its vector is empty: then the raw stick turned through
/// KH1's camera the same way (`stick_through_camera`, its signs checked against KH1's vector
/// whenever there is one; the user: walking, he kept the direction he started in). `+0x20`
/// only if neither.
unsafe fn stick_dir(actor: usize, tilt: f32) -> glam::Vec2 {
    let base = BASE.load(Ordering::Relaxed);
    let mv = rd::<[f32; 4]>(base + MOVE_VEC_RVA);
    let old = glam::Vec2::new(rd::<f32>(actor + 0x20), rd::<f32>(actor + 0x28)).normalize_or_zero();
    // (KH1's think refreshes it only while he can act, `0x1402A4090`: during one of its casts it
    // keeps the direction from before, the user: in Wisdom's casts of KH1 spells he slid the way
    // he had been going.)
    let can_act: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + CAN_ACT_RVA);
    let have_mv = mv[0].is_finite() && mv[2].is_finite() && mv[0].hypot(mv[2]) > 1e-3 && can_act(actor) != 0;
    // (KH2's pad direction and tilt, `character_core::pad`: the same dead zone as `move_tilt`.)
    let st = RAW_STICKS.load(Ordering::Relaxed).to_le_bytes();
    let ([lx, ly], raw_tilt) = character_core::pad::stick(st[2], st[3]);
    let (ca, cb, mode) = (rd::<f32>(base + CAM_A_RVA), rd::<f32>(base + CAM_B_RVA), rd::<u32>(base + CAM_MODE_RVA));
    let usual = !matches!(mode, 2 | 0xB) && ca.is_finite() && cb.is_finite();
    let raw_on = raw_tilt > 0.0;
    let mvdir = glam::Vec2::new(mv[0], mv[2]).normalize_or_zero();
    let mut best = 0;
    if let Ok(mut fit) = STICK_FIT.lock() {
        if have_mv && usual && raw_tilt > 0.5 {
            for c in 0..8 {
                let e = 1.0 - stick_through_camera(c, lx, ly, ca, cb).dot(mvdir);
                fit[c] = fit[c] * 0.95 + e;
            }
        }
        best = (0..8).min_by(|&i, &j| fit[i].total_cmp(&fit[j])).unwrap_or(0);
    }
    let dir = if have_mv {
        mvdir
    } else if usual && raw_on {
        stick_through_camera(best, lx, ly, ca, cb)
    } else {
        old
    };
    if tilt > 0.0 && !have_mv {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 12 {
            log(&format!("stick direction {dir:?} from the raw stick ({lx:.2}, {ly:.2}) through KH1's camera (convention {best}, mode {mode}); his +0x20 {old:?}, tilt {tilt:.2}"));
        }
    }
    dir
}
/// Flight (Neverland): the world flags' bit 0x20 lets Sora fly; KH1's fall state (2) starts
/// it once he is more than 100 above the floor (`0x1402AFD30`: motion 0x75, state 8, his rise
/// speed `+0x3C4` cut to 0.3) or, in a few rooms, above a set height (`0x1402AFC10`, the
/// table at `0x140529970`; its next-state step `0x1402ADA30` then goes to 8).
const FLIGHT_FLAG: u32 = 0x20;
const FLIGHT_START_RVA: Rva = Rva(0x2AFD30);
const FLIGHT_ROOM_RVA: Rva = Rva(0x2AFC10);
const SET_STATE_RVA: Rva = Rva(0x2AF7D0);
const FLIGHT_STATE: i32 = 8;
/// KH1's flight starts above this height over the floor (`0x1404308D4`).
const FLIGHT_ABOVE: f32 = 100.0;
/// Attack pressed while KH1 flies him (`combat::exec`): KH2 takes him in the air for its
/// aerial combo (the user: KH2's aerial attacks while flying).
pub static FLY_ATTACK: AtomicBool = AtomicBool::new(false);
/// That press, for KH2's first frame (after its fresh start drops older presses).
static FLY_PRESS: AtomicBool = AtomicBool::new(false);
/// The same for an Attack pressed during KH1's own move (`ATTACK_AFTER`), on the ground.
static FLY_PRESS_GROUND: AtomicBool = AtomicBool::new(false);
/// Frames an Attack carried out of a KH1 move may wait for KH1 to let him act (it holds him a
/// frame or two after a cut: the press was dropped there and he stood in idle, the user: "when I
/// try to go for an attack after I use a kh1 exclusive magic it just resets me to idle").
static CARRY_FRAMES: AtomicU32 = AtomicU32::new(0);
const CARRY_MAX: u32 = 10;
/// KH1's own move ended with him in the air: KH2 starts falling.
static AIR_START: AtomicBool = AtomicBool::new(false);
/// KH1's fall: its gravity (units per 60 Hz frame², `0x1403EAB54`) and fastest fall per frame
/// (20 × its step, `0x1404308B8`).
const KH1_FALL_ACCEL: f32 = 0.484_693_88;
const KH1_FALL_MAX: f32 = 20.0;
/// KH2 steps after a cut of a KH1 move still to log KH1's motion fields for.
static AFTER_CUT_LOG: AtomicU32 = AtomicU32::new(0);
/// KH1's glide (state 0x15, `0x1402B0BA0`): never while KH2 has him.
const KH1_GLIDE_STATE: i32 = 0x15;
/// Flying when KH2 took him: back into flight after the combo at any height (down to
/// `FLEW_ABOVE`), until he lands.
static FLEW: AtomicBool = AtomicBool::new(false);
const FLEW_ABOVE: f32 = 5.0;
/// In a flight combo he doesn't come down below this over the floor (the user: an attack that
/// touched the ground ended the flight), unless Square (flight's descend) is held.
const FLY_FLOOR: f32 = 15.0;
/// A flight combo's way to its target's height (the user: home in on it, down too): the feet
/// go toward the target point less this, at this share of the gap a frame, at most this fast.
const FLY_AIM_BELOW: f32 = 60.0;
const FLY_AIM_RATE: f32 = 0.15;
const FLY_AIM_MAX: f32 = 14.0;
const FLY_AIM_REACH: f32 = 600.0;
/// The flight sparkle (Tinker Bell's dust): Sora's step `0x140295E50` keeps its effect
/// (`+0xB0`, effect 0x14 on bone 0x8003, `0x1402ABA90`) while his state is 8 and ends it
/// otherwise. Its `cmp [rdi+0x70],8; jnz; mov eax,[rdi]` at `0x14029602E` calls a stub that
/// also says "flying" while this flag byte (in the stub) is set: during a flight combo.
const SPARKLE_RVA: Rva = Rva(0x29602E);
static SPARKLE_FLAG: AtomicUsize = AtomicUsize::new(0);
const LOCK_STATE_RVA: Rva = Rva(0x2DB3F10);
const LOCK_TARGET_RVA: Rva = Rva(0x2DB3F18);
/// A lock point's world position (`0x1402AE330 (actor, point, out) -> radius`, KH1 y-down).
const LOCK_POINT_RVA: Rva = Rva(0x2AE330);
const DRAW_WEAPON_RVA: Rva = Rva(0x2A0CA0);
const PUT_AWAY_RVA: Rva = Rva(0x2A10F0);
/// Actor flag: the weapon is out (drawn).
const WEAPON_OUT: u32 = 0x4000;
/// After an attack in the field, how long Sora keeps KH2's battle stance (Keyblade in hand)
/// before KH2's field idle puts it away (the user: a combo ended in the empty-handed idle).
const BATTLE_LINGER: f32 = 6.0;
/// How far below the feet the floor may drop and still be stepped down onto (KH2's floor ray
/// reaches 24 below).
const STEP_DOWN: f32 = 24.0;
/// KH1's air-move checks (see `grab`).
const CAN_ACT_RVA: Rva = Rva(0x2956E0);
/// KH1's tally of the save (`0x14028A4D0(id)`: `+0x848 + id * 2`).
const COUNT_RVA: Rva = Rva(0x28A4D0);
/// KH1's control step for the player (`0x1402A4090 (actor, move vector)`, from the think step
/// `0x1402A1170`): it reads the pad and starts commands only while Sora can act
/// (`0x1402956E0`); events, tutorial windows and scripted moments skip it or stop him acting.
const CONTROL_RVA: Rva = Rva(0x2A4090);
const CONTROL_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x40];
static ORIG_CONTROL: AtomicUsize = AtomicUsize::new(0);
/// KH1's control step ran for Sora since the last physics step, with him able to act.
static CONTROLLED: AtomicBool = AtomicBool::new(true);
/// KH1 holds him still (logged on a change).
static FROZEN: AtomicBool = AtomicBool::new(false);
static CONTROL_HOOKED: AtomicBool = AtomicBool::new(false);
static FROZEN_FRAMES: AtomicU32 = AtomicU32::new(0);
const VINE_RVA: Rva = Rva(0x2C5710);
const LEDGE_RVA: Rva = Rva(0x2C68A0);
const BAR_RVA: Rva = Rva(0x2C7E00);
const NO_BARS_RVA: Rva = Rva(0x298D3A8);
const HANG_RVA: Rva = Rva(0x2C17B0);
const SWIM_RVA: Rva = Rva(0x2BF410);
/// KH1 has him for one of its own moves (`grab`); KH2 starts over afterwards (`FRESH`).
static HANDOFF: AtomicBool = AtomicBool::new(false);
/// When `takeover` last saw KH1 still in its move (`kh1_move` holds while it does: KH2 draws
/// nothing meanwhile, so `active` lapses after 150 ms; the user: an Attack on the first free frame
/// after a spell was KH1's own attack).
static HANDOFF_SEEN_MS: AtomicU64 = AtomicU64::new(0);
/// Who had Sora on his last physics frame: 0 KH1 (or nobody), 1 KH2, 2 KH1 for one of its own
/// moves (`HANDOFF`). Game frames, not time: a pause keeps it (the user: after the battle pause, an
/// aerial combo went to KH1's moves, KH2 having been counted gone after 150 ms of pause).
static FRAME_KIND: AtomicU8 = AtomicU8::new(0);
static HANDOFF_FRAME: AtomicBool = AtomicBool::new(false);
static FRESH: AtomicBool = AtomicBool::new(false);
/// KH2 cut one of KH1's moves short: until KH1's motion slot is a plain one again (0..7), its
/// leftover isn't taken for an action of KH1's own.
static CUT_LEFT: AtomicBool = AtomicBool::new(false);

/// KH2 has Sora while what's left of a KH1 move it cut plays on in KH1 (`CUT_LEFT`): KH1's menu
/// greys its commands meanwhile.
pub fn cut_left() -> bool {
    CUT_LEFT.load(Ordering::Relaxed) && kh2_has()
}
/// KH2's first frames after KH1's own move: KH1's push speed isn't carried.
static NO_CARRY: AtomicU32 = AtomicU32::new(0);
static HANDOFF_FRAMES: AtomicU32 = AtomicU32::new(0);
/// The same in 60 Hz frames (`steps`).
static HANDOFF_TICKS: AtomicU32 = AtomicU32::new(0);
/// Frames of KH2's fall left in which KH1's grabs aren't tried: after KH1 lets go of him in the
/// air (off a ledge, a ladder, a pole), as KH1 skips them for 15 frames after jumping off
/// something (its flag 0x10000; the user: "when sora lets go of a ledge he instantly regrabs
/// the ledge").
static NO_GRAB: AtomicU32 = AtomicU32::new(0);
/// KH2's fall taken over from KH1's free jump or fall: its Aerial Dodge is there.
static AIR_MOVES: AtomicBool = AtomicBool::new(false);
const NO_GRAB_FRAMES: u32 = 20;
/// Where KH1's move had him last frame (his speed while it moves him).
static HANDOFF_POS: Mutex<Option<[f32; 3]>> = Mutex::new(None);
/// KH1's move ended in its fall: his speed then (per frame: sideways, up), for KH2's fall.
static HANDOFF_FALL: Mutex<Option<([f32; 2], f32)>> = Mutex::new(None);
/// Out of a Drive Form's KH1 cast on the ground: the speed it slid at (KH2 x / z per frame), for
/// KH2's first frame (`Character::carry_on_ground`).
static HANDOFF_SLIDE: Mutex<Option<[f32; 2]>> = Mutex::new(None);
/// Square was pressed as KH1's move ended: KH2 takes it as a press (not one already held).
static ROLL_NOW: AtomicBool = AtomicBool::new(false);
/// KH1's fall state (after a jump's rise, or off an edge): its collision is the air kind
/// (`0x140297F20` sets flag 1, no snapping onto the floor), and the wall solver's only extra
/// for it (`0x1402AD260`) clears its fall speed `+0x3C4` at a ceiling. No enter / exit handlers.
const AIR_STATE: i32 = 3;
/// KH1's generic air state (what its physics step puts an airborne actor in): its object-top
/// finder (`0x1402B3950`) looks below only in it.
const FALL_STATE: i32 = 2;
/// KH1's leap off a ladder or a pole (after its climb state 15; the user's s205 log).
const LEAP_OFF_STATE: i32 = 16;
/// KH1's object-top finder `0x1402B3950 (actor, move) -> object`: the standable object (box,
/// barrel: collision shapes of type 2 / 3) whose top the move ends at or under, the move's
/// height cut so it ends on the top. The map floor (`+0x320`) doesn't know objects.
const OBJECT_TOP_RVA: Rva = Rva(0x2B3950);
/// Sets the object the actor stands on (`+0x398`).
const STAND_ON_RVA: Rva = Rva(0x295C60);

pub const CIRCLE: u16 = 0x2000;
pub const SQUARE: u16 = 0x8000;
/// KH1's shortcut button (L1 + a face button).
pub const L1: u16 = 0x0400;
/// L1 in KH1's own button state (`0x14298D4A0`), its pad table's bit for the raw `0x400`.
const KH1_L1: u32 = 0x4_0000;
#[allow(dead_code)]
pub const CROSS: u16 = 0x4000;

pub static ON: AtomicBool = AtomicBool::new(true);
/// KH1's area script is moving Sora (`pose` shows KH1's own animation then).
pub static SCRIPTED: AtomicBool = AtomicBool::new(false);
/// KH2 has the Keyblade out (the thumbs grip) / Sora is on the ground (the feet meet the floor).
pub static GRIP: AtomicBool = AtomicBool::new(false);
pub static GROUNDED: AtomicBool = AtomicBool::new(false);
static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_PHYS: AtomicUsize = AtomicUsize::new(0);
/// The pad as the player pressed it (before KH1's copy is masked): buttons, then sticks
/// (rx, ry, lx, ly) packed in a u32.
pub static RAW_BUTTONS: AtomicU32 = AtomicU32::new(0);
pub static RAW_STICKS: AtomicU32 = AtomicU32::new(0x8080_8080);
static ACTIVE_MS: AtomicU64 = AtomicU64::new(0);
/// (t371) The mod's clock: real time less the time Frame Advance held the game (`frameadv`), so
/// the gaps and windows here (a pause seen, KH2 active, a handoff's grace) read a stepped frame
/// the way they read one played at full speed.
fn now_ms() -> u64 {
    super::frameadv::clock_ms()
}

/// Sora's last physics frame, and when the game last came back from a pause (a gap of more than
/// 250 ms between them).
static LAST_PHYS_MS: AtomicU64 = AtomicU64::new(0);
static RESUMED_MS: AtomicU64 = AtomicU64::new(0);

/// The game came back from a pause in the last `ms` milliseconds: Sora's model wasn't drawn
/// meanwhile (the pause menu draws a copy elsewhere), which is no sign of another body (the
/// user: paused mid KH2 move, resumed in KH1's; the log: "Sora is in another body" on resume).
pub fn resumed_within(ms: u64) -> bool {
    now_ms().saturating_sub(RESUMED_MS.load(Ordering::Relaxed)) < ms
}

/// The game stands still now: no physics frame of Sora's for over 120 ms (a pause, or a menu
/// that stops the field; `model` draws nothing then). Unknown (no frame yet): no.
pub fn paused() -> bool {
    // (t371) Frame Advance holding the game isn't a pause: KH2's model stays drawn.
    if super::frameadv::holding() {
        return false;
    }
    let last = LAST_PHYS_MS.load(Ordering::Relaxed);
    last != 0 && now_ms().saturating_sub(last) > 120
}

/// KH2 had Sora on his last physics frame (game frames: a pause keeps it).
pub fn kh2_frame() -> bool {
    FRAME_KIND.load(Ordering::Relaxed) == 1
}

/// KH2 has Sora this moment (its last frame was under 150 ms ago).
pub fn active() -> bool {
    let a = ACTIVE_MS.load(Ordering::Relaxed);
    a != 0 && now_ms().saturating_sub(a) < 150
}

struct Driver {
    character: Character,
    anim: RigAnim,
    held: (bool, bool),
    grounded: bool,
    last_state: Option<StateName>,
    logged: u32,
    strikes: combat::Strikes,
    /// Last attack (for `BATTLE_LINGER`).
    fought: Option<Instant>,
    /// KH2's last frame (dodge-roll invulnerability, guard), for `hurt`.
    last_out: Option<Output>,
    growth: menu::Growth,
    /// The KH2 attack motion playing and its frame (`swing_fx`).
    swing: Option<u16>,
    swing_frame: u32,
    /// Standing on top of something above the map floor (a box): its height (KH2 y-up).
    on_top: Option<f32>,
    /// A Drive Form (`form`): the change, the form, its gauge.
    drive: super::form::Drive,
}

static DRIVER: Mutex<Option<Driver>> = Mutex::new(None);
/// KH2's pose this frame (KH2 model space): Sora's bones and the Keyblade's (its keychain drives
/// KH1's, `kh1_data::secondary`), for `pose` to draw.
pub static POSE: Mutex<Option<(Vec<Mat4>, Vec<Mat4>, Instant)>> = Mutex::new(None);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

type PhysFn = unsafe extern "system" fn(usize);
type MoveFn = unsafe extern "system" fn(usize, *const [f32; 4], u32);

/// KH1's frame scale (`0x1423405DC`, its 60 Hz frames per game frame: 1.0 at 60 fps, 2.0 at 30;
/// the play logs show 1, 2 and a 4 at a hitch). KH2's side runs in whole 60 Hz frames: its
/// controller, animation, hit and guard windows, effects and sounds step this many times a game
/// frame (`STEPS`), so a KH2 move takes the same time at 30 fps as at 60 (the user: "when the game
/// is running at 30 fps anytime a kh2 move is used its effectively makes the game run in
/// slowmotion").
const DT_RVA: Rva = Rva(0x23405DC);
/// At most this many 60 Hz steps in one game frame (a longer hitch is not caught up).
const MAX_STEPS: u32 = 4;
/// 60 Hz steps in this game frame (set on Sora's physics frame) and the fraction carried over.
static STEPS: AtomicU32 = AtomicU32::new(1);
static STEP_REST: AtomicU32 = AtomicU32::new(0);
/// 60 Hz frames so far (`STEPS` added each game frame): for frame counts in KH2's terms (hurt's
/// re-hit grace, the chains' swing).
pub static TICKS: AtomicU64 = AtomicU64::new(0);

/// 60 Hz frames in this game frame (1 at 60 fps, 2 at 30).
pub fn steps() -> u32 {
    STEPS.load(Ordering::Relaxed).clamp(1, MAX_STEPS)
}

/// This game frame's 60 Hz steps from KH1's frame scale: whole ones, a fraction carried to the next
/// frame (exactly 1 a frame at a scale of 1.0, so 60 fps runs as before).
unsafe fn count_steps(base: usize) -> u32 {
    let dt: f32 = rd(base + DT_RVA);
    let dt = if dt.is_finite() && dt > 0.0 { dt.min(MAX_STEPS as f32) } else { 1.0 };
    let dt = if (dt - dt.round()).abs() < 0.02 { dt.round() } else { dt };
    let acc = f32::from_bits(STEP_REST.load(Ordering::Relaxed)) + dt;
    let n = (acc.floor() as u32).clamp(1, MAX_STEPS);
    STEP_REST.store((acc - n as f32).clamp(0.0, 0.99).to_bits(), Ordering::Relaxed);
    if n != STEPS.swap(n, Ordering::Relaxed) {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 20 {
            log(&format!("frame scale {dt:.2}: KH2 steps {n} 60 Hz frame{} a game frame", if n == 1 { "" } else { "s" }));
        }
    }
    n
}

unsafe extern "system" fn physics(actor: usize) {
    let sora = actor > 0x10000 && actor == rd::<u64>(BASE.load(Ordering::Relaxed) + PARTY_RVA) as usize;
    // Where he starts the frame (`trace`).
    let start = if sora { rd::<[f32; 4]>(actor + 0x10) } else { [0.0; 4] };
    if sora {
        let n = count_steps(BASE.load(Ordering::Relaxed));
        TICKS.fetch_add(n as u64, Ordering::Relaxed);
        HANDOFF_FRAME.store(false, Ordering::Relaxed);
        // A gap in Sora's physics frames: the game was paused (or a menu held it).
        let now = now_ms();
        if now.saturating_sub(LAST_PHYS_MS.swap(now, Ordering::Relaxed)) > 250 {
            RESUMED_MS.store(now, Ordering::Relaxed);
        }
    }
    if sora && ON.load(Ordering::Relaxed) {
        super::shared::physics::install(BASE.load(Ordering::Relaxed));
        // (Our Quick Run's state in KH1's table, `quickrun`.)
        super::quickrun::install(BASE.load(Ordering::Relaxed));
        // (KH2's glide in KH1's glide state, `glide`.)
        super::glide::install(BASE.load(Ordering::Relaxed));
    }
    let took = ON.load(Ordering::Relaxed) && actor > 0x10000 && takeover(actor);
    if sora {
        GAME_FRAME.fetch_add(1, Ordering::Relaxed);
        FRAME_KIND.store(if took { 1 } else if HANDOFF_FRAME.load(Ordering::Relaxed) { 2 } else { 0 }, Ordering::Relaxed);
        room_watch(actor, took);
    }
    if took {
        if sora {
            trace_frame(actor, start);
        }
        return;
    }
    // KH1 has him this frame: a Keyblade trail KH2's swing asked for ends, and KH2's effects
    // play on (those of the move KH1 cut short end; the user: a spell cast during a move with a
    // KH2 effect froze the effect where it was).
    if actor > 0x10000 && actor == rd::<u64>(BASE.load(Ordering::Relaxed) + PARTY_RVA) as usize {
        combat::trail_off(actor);
        super::reflect::kh1_frame();
        // (At KH2's 60 Hz: `steps`.)
        // (Our KH2 slot playing in KH1's motion table, `kh2slots`: its effects go on with it.)
        let motion = super::kh2slots::fx_motion().unwrap_or(super::kh2fx::NO_MOTION);
        // (One tick of this frame's delta, as the game takes one update of delta 2 at 30 fps:
        // the effect clock adds the delta up itself, `kh2_fx::Effects::tick`.)
        super::kh2fx::tick(motion, steps() as f32);
        // KH2's spell scripts of a cast in KH1's own cast state (`formmagic::kh1_spell_start`).
        native_spells(actor);
    }
    if sora {
        // KH2's hop out of KH1's ground combo: its sideways speed, which KH1's carry into the air
        // replaced (`shared::physics::ground_hop_carry`).
        super::shared::physics::ground_hop_carry(actor);
        // KH2's turn toward the target in our slot's KH2 motion (`kh2slots::turn_to_lock`).
        super::kh2slots::turn_to_lock(actor);
        // KH2's guard repel's push back (`combomods::guard_frame`).
        super::combomods::guard_frame(actor);
        // KH2's Aerial Dodge's fall at its apex (`combomods::dodge_apex`).
        super::combomods::dodge_apex(actor);
        // KH2's push on KH1's push after a hit (`hitreact`).
        super::hitreact::push_frame(actor);
    }
    let f: PhysFn = std::mem::transmute(ORIG_PHYS.load(Ordering::Acquire));
    f(actor);
    if sora {
        // A KH1 air attack out of KH2's air move: its lunge (`shared::physics::seed_kh1_lunge`).
        super::shared::physics::seed_kh1_lunge(actor);
        trace_frame(actor, start);
    }
}

/// Sora's frame for the trace (`trace`), once his physics frame is over (KH2's or KH1's): KH1's
/// actor, KH2's controller, the pad. `start`: where he was when the frame began (KH1's y-down).
unsafe fn trace_frame(actor: usize, start: [f32; 4]) {
    if !super::trace::on() {
        return;
    }
    let base = BASE.load(Ordering::Relaxed);
    let p = rd::<[f32; 4]>(actor + 0x10);
    // (KH2's side only on KH2's own frames: on KH1's its last move stays in the controller.)
    // (t354: and only while the trace records KH2's state.)
    let kh2_frame = FRAME_KIND.load(Ordering::Relaxed) == 1 && super::trace::wants(super::trace::cat::KH2_STATE);
    let kh2 = DRIVER.try_lock().ok().filter(|_| kh2_frame).and_then(|g| {
        let kh2 = g.as_ref().map(|d| {
            let o = d.last_out.as_ref();
            super::trace::Kh2 {
                state: format!("{:?}", d.character.state_name()),
                motion: o.map_or(0, |o| o.motion),
                frame: o.map_or(0, |o| o.clip_frame),
                air: d.character.airborne(),
                grounded: d.grounded,
                on_top: d.on_top,
                facing: o.map_or(f32::NAN, |o| o.facing),
                combo: o.map_or(0, |o| o.combo_count),
                lock: o.and_then(|o| o.locked),
                invulnerable: o.is_some_and(|o| o.invulnerable),
                guarding: o.is_some_and(|o| o.guarding),
            }
        });
        kh2
    });
    let sticks = RAW_STICKS.load(Ordering::Relaxed).to_le_bytes();
    super::trace::frame(super::trace::Row {
        frame: GAME_FRAME.load(Ordering::Relaxed),
        ticks: TICKS.load(Ordering::Relaxed),
        steps: steps(),
        ms: now_ms(),
        // (Set by `trace::frame`.)
        wall_ms: 0,
        owner: match FRAME_KIND.load(Ordering::Relaxed) {
            1 => "KH2",
            2 => "handoff",
            _ => "KH1",
        },
        shared: super::shared::active(),
        world: rd::<u32>(base + WORLD_RVA),
        room: rd::<u32>(base + ROOM_RVA),
        event: rd::<u32>(base + EVENT_RVA) != 0,
        // (Filled by `trace::frame`.)
        config: 0,
        pos: [p[0], -p[1], p[2]],
        moved: [p[0] - start[0], start[1] - p[1], p[2] - start[2]],
        asked: super::trace::take_asked(),
        above: rd::<f32>(actor + 0x3D4),
        floor_up: -rd::<f32>(actor + 0x324),
        collision: rd::<u32>(actor + 0x2D0),
        stand_on: rd::<u32>(actor + 0x398),
        platform: rd::<u32>(actor + 0x160) & 4 != 0,
        yaw: rd::<f32>(actor + 0x3C),
        kh1_state: rd::<i32>(actor + 0x70),
        kh1_motion: rd::<u32>(actor + 0x164),
        flags: rd::<u32>(actor),
        combo_window: rd::<u32>(actor + 0x1CC),
        velocity: rd::<[f32; 4]>(actor + 0x3C0),
        arc: rd::<[f32; 4]>(actor + 0x3B0),
        push: rd::<[f32; 3]>(actor + 0x120),
        kh2,
        form: if super::trace::wants(super::trace::cat::FORM) { super::form::current().map(|k| format!("{k:?}")) } else { None },
        buttons: RAW_BUTTONS.load(Ordering::Relaxed) as u16,
        kh1_buttons: rd::<u32>(base + BUTTONS_RVA + 4),
        sticks,
        draw: Default::default(),
        cam: Default::default(),
        feet: Default::default(),
        events: String::new(),
        // (Filled by `trace::frame`.)
        cats: 0,
    });
}

/// KH1's ground state's jump press (`0x1402CA3F0`, state 0's `+0x38`: its ground test, then the
/// jump start `0x1402AD860`, jump or high jump motion, state 1).
#[allow(dead_code)]
const JUMP_PRESS_RVA: Rva = Rva(0x2CA3F0);
/// KH1's Glide and Superglide starts (`0x1402B0BA0` under both: its glide motion 0x71, air speed,
/// state 0x15; nothing below its height over the floor).
const GLIDE_RVA: Rva = Rva(0x2B0530);
const SUPERGLIDE_RVA: Rva = Rva(0x2B05C0);
/// KH1's ability test (`(ability, 0xB)`): 3 Glide, 4 Superglide (`0x1402A4600`'s use).
const HAS_ABILITY_RVA: Rva = Rva(0x282A70);

/// KH1's jump or glide instead of KH2's, on a fresh jump press (`stylejump`): what KH1 started,
/// or `None` (KH2's as before). KH1 has him from this frame.
unsafe fn style_air_start(base: usize, actor: usize, d: &Driver, glide_press: bool) -> Option<&'static str> {
    if rd::<i32>(actor + 0x70) != 0 || FLEW.load(Ordering::Relaxed) || rd::<u32>(base + BATTLE_RVA) & FLIGHT_FLAG != 0 {
        return None;
    }
    let st = d.character.state_name();
    let up = d.last_out.as_ref().map_or(0.0, |o| o.displacement[1]);
    // (KH2's sideways speed `v` (kept to 30, `k`) is in KH1's fields already: `shared::physics::mirror`.)
    let set: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + SET_STATE_RVA);
    // (Off for now, s251, the user: KH1's own jump start, its speed and take-off height:)
    // if !glide_press && super::stylejump::kh1_jump() && d.grounded && !d.character.airborne() && matches!(st, StateName::Idle | StateName::Walk | StateName::Run | StateName::Land) {
    //     let press: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(base + JUMP_PRESS_RVA);
    //     let y = rd::<f32>(actor + 0x14);
    //     press(actor);
    //     if rd::<i32>(actor + 0x70) != 1 {
    //         static N: AtomicU32 = AtomicU32::new(0);
    //         if N.fetch_add(1, Ordering::Relaxed) < 10 {
    //             log(&format!("KH1's jump didn't start (KH1 state {}): KH2's", rd::<i32>(actor + 0x70)));
    //         }
    //         return None;
    //     }
    //     // KH2's ground speed carried into KH1's air speed (`+0x3C0`: x, y, z, its length); on
    //     // KH2's first frame back (out of KH1's own landing) KH1's own is left.
    //     if d.last_state.is_some() {
    //         wr(actor + 0x3C0, [v[0] * k, 0.0f32, v[1] * k, n * k]);
    //     }
    //     wr(actor + 0x3B4, y);
    //     return Some("KH1's jump (the Jump style is KH1's)");
    // }
    let falling = up <= 0.0;
    // (KH1's glide as KH2's glide button has it: Superglide on a fresh Square, Glide on a fresh
    // jump press once the Aerial Dodge isn't there for it. In KH2's Aerial Dodge a fresh press of
    // the glide button is a glide at once (`controller`: "The glide button mid-dodge: straight
    // into Glide"), not only past the rise.)
    let in_dodge = matches!(st, StateName::AerialDodge);
    let jump_free = in_dodge || !d.character.air_dodge_available();
    if super::stylejump::kh1_glide()
        && d.character.airborne()
        && (in_dodge || falling && matches!(st, StateName::Air | StateName::Jump))
        && if glide_press { glide_on_square_now(base) } else { !glide_on_square_now(base) && jump_free }
    {
        // KH1's air state as an air command sets it (`combat::command`), then its glide.
        // (Once a hold of Square: a glide that doesn't start isn't tried every frame.)
        if !kh1_glide_open(base) || GLIDE_TRIED.swap(true, Ordering::Relaxed) {
            return None;
        }
        set(actor, 2);
        // (KH2's speed and arc, written into KH1's fields by its steps: the glide starts from them.)
        super::shared::physics::kh1_takes_air(actor);
        wr(actor + 0x3D0, 0u32);
        wr(actor, rd::<u32>(actor) & !0x1_0000);
        return match kh1_glide_start(base, actor) {
            Some(w) => Some(w),
            None => {
                set(actor, 0);
                None
            }
        };
    }
    None
}

/// Sora has KH1's Glide or Superglide.
unsafe fn kh1_glide_open(base: usize) -> bool {
    let has: unsafe extern "system" fn(i32, i32) -> i32 = std::mem::transmute(base + HAS_ABILITY_RVA);
    has(4, 0xB) != 0 || has(3, 0xB) != 0
}

/// Sora has KH1's own Glide (`glide::party`).
pub(crate) unsafe fn kh1_has_glide(base: usize) -> bool {
    let has: unsafe extern "system" fn(i32, i32) -> i32 = std::mem::transmute(base + HAS_ABILITY_RVA);
    has(3, 0xB) != 0
}

/// Sora has KH1's Superglide: his glide is on Square (vanilla KH1: Glide on the jump button,
/// Superglide on Square). `glide = square` in the ini (`glide_on_square`, off) forces Square too.
unsafe fn glide_on_square_now(base: usize) -> bool {
    if super::settings::get().glide_on_square {
        return true;
    }
    let has: unsafe extern "system" fn(i32, i32) -> i32 = std::mem::transmute(base + HAS_ABILITY_RVA);
    has(4, 0xB) != 0
}

/// KH1's Superglide (when he has it) or Glide, from KH1's air state: what started, if it did.
unsafe fn kh1_glide_start(base: usize, actor: usize) -> Option<&'static str> {
    let has: unsafe extern "system" fn(i32, i32) -> i32 = std::mem::transmute(base + HAS_ABILITY_RVA);
    let (superglide, glide) = (has(4, 0xB) != 0, has(3, 0xB) != 0);
    if !superglide && !glide {
        return None;
    }
    let start: unsafe extern "system" fn(usize) = std::mem::transmute(base + if superglide { SUPERGLIDE_RVA } else { GLIDE_RVA });
    start(actor);
    if rd::<i32>(actor + 0x70) != KH1_GLIDE_STATE {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 10 {
            log(&format!("KH1's glide didn't start ({:.0} above the floor, KH1 state {}): KH2 keeps him", rd::<f32>(actor + 0x3D4), rd::<i32>(actor + 0x70)));
        }
        return None;
    }
    // (Glide at KH1 only, t321: KH1's own glide, its own motions and numbers whatever the physics
    // setting; Glide at KH2 is KH2's glide in KH1's glide state, `glide`.)
    Some(if superglide { "KH1's Superglide (Glide style KH1, KH1's own numbers)" } else { "KH1's glide (Glide style KH1, KH1's own numbers)" })
}

/// KH1's roll (`styleroll`): its start, the stick's angle, the facing setter, its deep-water
/// test, and its ability bits (`0x20` Dodge Roll).
const ROLL_START_RVA: Rva = Rva(0x2C7CB0);
const DIR_ANGLE_RVA: Rva = Rva(0x295E20);
const SET_FACING_RVA: Rva = Rva(0x295C80);
const DEEP_WATER_RVA: Rva = Rva(0x2BF530);
/// (`0x142D5F610`, its handler's `mov eax, [rip+...]` at `0x1402A5A65`; s240 read `0x142F5F610`,
/// which never had the bit: the user, "unable to input a kh1 dodge roll".)
const ROLL_ABILITY_RVA: Rva = Rva(0x2D5F610);
const ROLL_ABILITY: u32 = 0x20;

/// KH1's dodge roll instead of KH2's, on a fresh Square with the stick (`styleroll`): whether it
/// started. KH1 has him from this frame.
unsafe fn style_roll_start(base: usize, actor: usize, d: &Driver, dir: glam::Vec2, tilt: f32) -> bool {
    if !super::styleroll::kh1_roll() || tilt <= 0.15 || !d.grounded || d.character.airborne() || rd::<i32>(actor + 0x70) != 0 {
        return false;
    }
    if !matches!(d.character.state_name(), StateName::Idle | StateName::Walk | StateName::Run | StateName::Land) {
        return false;
    }
    let deep: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + DEEP_WATER_RVA);
    if rd::<u32>(base + ROLL_ABILITY_RVA) & ROLL_ABILITY == 0 || deep(actor) != 0 {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 10 {
            log(&format!("KH1's roll not open (ability bits {:#x}): KH2's Square", rd::<u32>(base + ROLL_ABILITY_RVA)));
        }
        return false;
    }
    let v = [dir.x, 0.0f32, dir.y, tilt];
    let angle: unsafe extern "system" fn(usize, *const f32) -> f32 = std::mem::transmute(base + DIR_ANGLE_RVA);
    let face: unsafe extern "system" fn(usize, f32) = std::mem::transmute(base + SET_FACING_RVA);
    let start: unsafe extern "system" fn(usize) = std::mem::transmute(base + ROLL_START_RVA);
    let a = angle(actor, v.as_ptr());
    // (The angle is its second argument: the second float register, as KH1's handler passes it.)
    face(actor, a);
    start(actor);
    true
}

/// KH1's player step (`0x1402A75B0`, Sora's per-frame step, from the actor kinds' switch at
/// `0x14029593C`). On a frame his state changed (flag `0x400`, set by its state change
/// `0x1402AF7D0`) it picks the motion for the new state: down from the air (state 2 to 0, the old
/// state at `+0x378`) KH1's landing (`(+0x3D0) + 7 | 0x1000000`, then `0x14029D8A0 (0x40000000)`),
/// into the air its fall (6). That is where KH1's air spell is hard-wired to KH1's landing.
#[allow(dead_code)]
const PLAYER_STEP_RVA: Rva = Rva(0x2A75B0);
/// `push rbx; sub rsp, 0x50` (the next instruction is rip-relative, not moved).
#[allow(dead_code)]
const PLAYER_STEP_PROLOGUE: [u8; 6] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x50];
#[allow(dead_code)]
static ORIG_PLAYER_STEP: AtomicUsize = AtomicUsize::new(0);
/// KH1 set Sora down out of an air spell, and its landing motion wasn't asked for: KH2 takes him
/// into KH2's landing (`takeover`'s hand-off, its next frame).
static AIR_SPELL_LANDED: AtomicBool = AtomicBool::new(false);

#[allow(dead_code)]
unsafe extern "system" fn player_step(actor: usize) {
    let base = BASE.load(Ordering::Relaxed);
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_PLAYER_STEP.load(Ordering::Acquire));
    // KH1's air spell (KH1 has him for it) landing this frame: KH1's own landing isn't asked for
    // (its old state hidden from the step for the call), KH2's is, from his spell's air state (the
    // user: "code soras kh1s aerial magic animation states so that they are able to call kh2's
    // animation states and call kh2's landing animation").
    let landing = actor > 0x10000
        && actor == rd::<u64>(base + PARTY_RVA) as usize
        && ON.load(Ordering::Relaxed)
        && HANDOFF.load(Ordering::Relaxed)
        && air_cast().is_some()
        && rd::<u32>(actor) & 0x400 != 0
        && rd::<i32>(actor + 0x70) == 0
        && rd::<i32>(actor + 0x378) == 2;
    if !landing {
        return f(actor);
    }
    wr(actor + 0x378, 0i32);
    f(actor);
    wr(actor + 0x378, 2i32);
    AIR_SPELL_LANDED.store(true, Ordering::Relaxed);
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 20 {
        log(&format!(
            "KH1's air spell landed: KH1's landing motion not asked for, KH2's landing next (KH1 motion {:#x}, {:.0} above the floor)",
            rd::<u32>(actor + 0x164),
            rd::<f32>(actor + 0x3D4)
        ));
    }
}

unsafe extern "system" fn control(actor: usize, dir: usize) {
    // KH2's first person with the mouse, base Sora: KH1's own turn is fed the mouse for its step
    // (`first_person_mouse_turn`), its left stick put back after.
    let base = BASE.load(Ordering::Relaxed);
    let saved = first_person_mouse_turn(base, actor);
    control_step(actor, dir);
    if let Some(l) = saved {
        wr(base + LEFT_STICK_RVA, l);
    }
}

unsafe fn control_step(actor: usize, dir: usize) {
    let base = BASE.load(Ordering::Relaxed);
    if actor > 0x10000 && actor == rd::<u64>(base + PARTY_RVA) as usize {
        combat::resume_command();
        let can: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + CAN_ACT_RVA);
        if can(actor) != 0 {
            CONTROLLED.store(true, Ordering::Relaxed);
        }
    }
    let f: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_CONTROL.load(Ordering::Acquire));
    // KH1's own jump / cancel (bits 6) and Square action (`0x100000`) are KH2's while Sora is
    // KH2's, or KH1 has him for a moment outside one of its own moves (a reaction, examining
    // something): taken out of KH1's button state (`0x14298D4A0`: +4 held, +8 pressed, +0x10 /
    // +0x14 repeats; all pads merged, the confirm swap applied) for its control step only, put
    // back after (menus keep them). The user: KH1 jumped (and glided, and its air combo) on the
    // first frame after a pause or an examine, with two pads (Steam Input), or with the swap.
    // Base Sora in KH1's own states (`shared`): KH1 sees its buttons as unmodded, but for the
    // presses that are KH2's (its moves' borrows) and KH1's glide inputs the move styles give
    // elsewhere (`shared::control`).
    let is_sora = actor > 0x10000 && actor == rd::<u64>(base + PARTY_RVA) as usize;
    if is_sora && super::shared::active() && kh1_borrows(actor) {
        return shared_control(base, actor, dir, f);
    }
    // KH2 has him for one of its moves (`shared`): a fresh jump on the ground where KH2's move can
    // be left is KH1's own jump (its states have the jump), not stripped (`JUMP_OUT`).
    let jump_out = is_sora && super::shared::on() && !HANDOFF.load(Ordering::Relaxed) && {
        let held = rd::<u32>(base + BUTTONS_RVA + 4) & 6 != 0;
        let fresh = held && !JUMP_OUT_PREV.swap(held, Ordering::Relaxed);
        fresh && rd::<i32>(actor + 0x70) == 0 && kh2_jump_open()
    };
    if !jump_out {
        JUMP_OUT_PREV.store(rd::<u32>(base + BUTTONS_RVA + 4) & 6 != 0, Ordering::Relaxed);
    }
    JUMP_OUT.store(jump_out, Ordering::Relaxed);
    let strip = actor > 0x10000
        && actor == rd::<u64>(base + PARTY_RVA) as usize
        // (Also all through one of KH1's commands, a spell or an item: on the frame it lets him
        // act again KH1 clears its action flag inside its own step and then read a jump or Square
        // as its own jump, high jump or dodge roll, the user. Those presses are KH2's when it
        // takes him back.)
        // (Off for now, s251: KH1's own jump's buttons kept from it, `stylejump::JUMP`; KH1's glide
        // still is, its Square held fed as its jump held.)
        // && (... || super::stylejump::active())
        && (!HANDOFF.load(Ordering::Relaxed) || kh1_move_done(base, actor) || kh1_air_free(base, actor) || FROM_COMMAND.load(Ordering::Relaxed) || super::stylejump::mode() == super::stylejump::GLIDE)
        && kh1_borrows(actor);
    // With L1 down (KH1's shortcuts) or a submenu open, the presses are KH1's (a shortcut, backing
    // out): only the held and repeat bits go then. A jump held while L1 came down was KH1's held
    // jump: KH1 jumped as KH2 landed him and went on to its own air moves (the user: "holding the
    // jump button and hitting L1 does crazy stuff"; the log: KH2's landing with KH1's state 2
    // and jump slot 12, then state 21). (L1 as KH1's own button state has it this frame,
    // `0x40000` from the raw `0x400` in its pad table `0x140528F90`; the raw pad's could differ
    // from it for a frame.)
    // (A submenu open keeps only its back-out, the jump / cancel press: Square there was KH1's
    // own guard, in the air too (spec 3.2); KH2's guard follows its own rules.)
    let l1_menu = super::shortcuts::menu_on_l1(if rd::<u32>(base + BUTTONS_RVA + 4) & KH1_L1 != 0 { L1 } else { 0 });
    let submenu = super::shortcuts::submenu_open();
    // (KH1's jump with KH1's glide, `stylejump`: the jump button is KH1's there, its glide.)
    let kh2_bits: u32 = if super::stylejump::kh1_keeps_jump() { 0x10_0000 } else if jump_out { 0x10_0000 } else { 0x6 | 0x10_0000 };
    // KH1's own jump (`stylejump::JUMP`) rising (its state 1, going up): its jump held is KH1's,
    // for its height (the user: "I also dont as high of a jump as i should be getting because it
    // cuts off the input to the jump button"); from its top on it isn't, so holding it doesn't
    // glide (KH1's glide is on Square here: "as soon as i jump i go into a glide").
    // (Off for now, s251: `false &&`.)
    let kh1_rise = false && super::stylejump::mode() == super::stylejump::JUMP && actor > 0x10000 && rd::<i32>(actor + 0x70) == 1 && {
        let p = rd::<[f32; 3]>(actor + 0x10)[1];
        let rising = RISE_Y.swap(p.to_bits(), Ordering::Relaxed);
        p <= f32::from_bits(rising)
    };
    let bits = |off: usize| -> u32 {
        if off == 4 && kh1_rise {
            kh2_bits & !0x6
        } else if off != 8 {
            kh2_bits
        } else if l1_menu {
            0
        } else if submenu {
            0x10_0000 & kh2_bits
        } else {
            kh2_bits
        }
    };
    let state = base + BUTTONS_RVA;
    let saved = rd::<[u32; 6]>(state);
    // KH1's glide at the Glide style KH1 (`stylejump::GLIDE`): its jump button held is Square held
    // (it glides on Square, the user), for its held state only.
    let glide_btn_held = if glide_on_square_now(base) { buttons() & SQUARE != 0 } else { saved[1] & 0x6 != 0 };
    let inject: u32 = if strip && super::stylejump::mode() == super::stylejump::GLIDE && glide_btn_held { 0x6 } else { 0 };
    if strip {
        for off in [4usize, 8, 0x10, 0x14] {
            wr(state + off, rd::<u32>(state + off) & !bits(off));
        }
        wr(state + 4, rd::<u32>(state + 4) | inject);
    }
    f(actor, dir);
    if strip {
        // KH1's own changes to the state meanwhile kept; only our bits come back.
        wr(state + 4, rd::<u32>(state + 4) & !inject);
        for off in [4usize, 8, 0x10, 0x14] {
            wr(state + off, rd::<u32>(state + off) | (saved[off / 4] & bits(off)));
        }
    }
}

/// A fresh jump on the ground let through to KH1's control step out of KH2's move (`shared`):
/// KH1's own jump, KH1 takes him (`takeover`, its state 1 next).
static JUMP_OUT: AtomicBool = AtomicBool::new(false);
static JUMP_OUT_PREV: AtomicBool = AtomicBool::new(true);

/// KH2's move can be left for a jump now: free (idle, walking, running, landing) or in a window
/// that takes a command (a combo's), on the ground.
fn kh2_jump_open() -> bool {
    DRIVER
        .try_lock()
        .ok()
        .and_then(|d| {
            d.as_ref().map(|d| d.grounded && !d.character.airborne() && (matches!(d.character.state_name(), StateName::Idle | StateName::Walk | StateName::Run | StateName::Land) || d.character.can_take_command()))
        })
        .unwrap_or(false)
}

/// Square on the pad (or the keyboard): KH1's action bit, but not the one its held jump sets in
/// the air (s247's filter).
pub fn pad_square() -> bool {
    let base = BASE.load(Ordering::Relaxed);
    let raw = RAW_BUTTONS.load(Ordering::Relaxed) as u16 & SQUARE != 0;
    let held = if base == 0 { 0 } else { unsafe { rd::<u32>(base + BUTTONS_RVA + 4) } };
    raw || held & 0x10_0000 != 0 && held & 6 == 0
}

/// KH2's Aerial Dodge is equipped (the controller's abilities, from KH1's menu).
fn kh2_air_dodge() -> bool {
    // (The controller busy this step: the last answer, not "no dodge", which let KH1 see the
    // dodge's jump press as its glide.)
    static LAST: AtomicBool = AtomicBool::new(false);
    match DRIVER.try_lock().ok().and_then(|d| d.as_ref().map(|d| d.character.abilities.aerial_dodge > 0)) {
        Some(v) => {
            LAST.store(v, Ordering::Relaxed);
            v
        }
        None => LAST.load(Ordering::Relaxed),
    }
}

/// KH2's Aerial Dodge at Sora's level (KH2's `fmab`: its height and its speed a frame), if
/// equipped (`combomods::aerial_dodge`). The controller busy: the last answer.
pub fn kh2_air_dodge_growth() -> Option<(f32, f32)> {
    static LAST: Mutex<Option<(f32, f32)>> = Mutex::new(None);
    let read = DRIVER.try_lock().ok().and_then(|d| {
        d.as_ref().map(|d| {
            let level = d.character.abilities.aerial_dodge;
            d.character.growth_level(level).map(|g| (g.air_dodge_height, g.air_dodge_speed)).filter(|g| g.0.is_finite() && g.1.is_finite() && g.0 > 0.0)
        })
    });
    match read {
        Some(v) => {
            if let Ok(mut l) = LAST.lock() {
                *l = v;
            }
            v
        }
        None => LAST.lock().ok().and_then(|l| *l),
    }
}

/// KH2's dodge roll level and its invulnerable frames (`fmab` `dodge_roll_star_time`, 10 / 16 / 24 /
/// 32 by level) for Sora's Dodge Roll now (`combomods::roll_press`, the native roll). The last answer
/// while the controller is busy. `None` without KH2's Dodge Roll.
pub fn kh2_dodge_roll_growth() -> Option<(u8, f32)> {
    static LAST: Mutex<Option<(u8, f32)>> = Mutex::new(None);
    let read = DRIVER.try_lock().ok().and_then(|d| {
        d.as_ref().map(|d| {
            let level = d.character.abilities.dodge_roll;
            (level > 0).then(|| (level, d.character.growth_level(level).map_or(0.0, |g| g.dodge_roll_star_time).max(0.0)))
        })
    });
    match read {
        Some(v) => {
            if let Ok(mut l) = LAST.lock() {
                *l = v;
            }
            v
        }
        None => LAST.lock().ok().and_then(|l| *l),
    }
}

/// KH2's Quick Run numbers for Sora's Quick Run level now (`quickrun`, its native state): the
/// level (1 LV1 .. 4 MAX) and its `fmab` record (KH2's start `0x1403A9830` reads `+0xC` the time,
/// `+0x10` the speed, `+0x14` the brake, `+0x18` the stop brake, `+0x1C` the invulnerable frames;
/// Checked (decomp)). The last answer while the controller is busy. `None` without Quick Run.
pub fn kh2_quick_run_growth() -> Option<(u8, character_core::GrowthLevel)> {
    static LAST: Mutex<Option<(u8, character_core::GrowthLevel)>> = Mutex::new(None);
    let read = DRIVER.try_lock().ok().and_then(|d| {
        d.as_ref().map(|d| {
            let level = d.character.abilities.quick_run;
            d.character.growth_level(level).cloned().filter(|g| g.quick_run_speed.is_finite() && g.quick_run_speed > 0.0).map(|g| (level, g))
        })
    });
    match read {
        Some(v) => {
            if let Ok(mut l) = LAST.lock() {
                *l = v.clone();
            }
            v
        }
        None => LAST.lock().ok().and_then(|l| l.clone()),
    }
}

/// KH2's glide numbers for Sora's Glide level (`glide`, KH2's glide in KH1's state): (level, its
/// growth record). Its level kept aside counts (`KH2_GLIDE_KEPT`). The controller busy this step:
/// as last read.
pub fn kh2_glide_growth() -> Option<(u8, character_core::GrowthLevel)> {
    static LAST: Mutex<Option<(u8, character_core::GrowthLevel)>> = Mutex::new(None);
    let read = DRIVER.try_lock().ok().and_then(|d| {
        d.as_ref().map(|d| {
            let level = (d.character.abilities.glide as u32).max(KH2_GLIDE_KEPT.load(Ordering::Relaxed)).min(u8::MAX as u32) as u8;
            d.character.growth_level(level).cloned().filter(|g| g.glide_speed.is_finite() && g.glide_speed > 0.0).map(|g| (level, g))
        })
    });
    match read {
        Some(v) => {
            if let Ok(mut l) = LAST.lock() {
                *l = v.clone();
            }
            v
        }
        None => LAST.lock().ok().and_then(|l| l.clone()),
    }
}

/// KH2's stored launch speed this air time, if KH2 has one (`LAUNCH`): what KH2's glide enter puts
/// back (`0x1403CBD30`, `+0x20 = +0xE0` while its flag is set).
pub fn launch_kept() -> Option<[f32; 2]> {
    LAUNCH.lock().ok().and_then(|l| *l).flatten()
}

/// KH2's glide ended into the fall: the launch flag cleared (`0x1403CBC30`), for the rest of this
/// air time.
pub fn launch_used() {
    if let Ok(mut l) = LAUNCH.lock() {
        *l = Some(None);
    }
}

/// KH2's stored launch velocity set by a move KH1's states run (`combomods::aerial_dodge`: the
/// Aerial Dodge's), for KH2's glide later in the same air time (as `shared_give_back` keeps the
/// controller's).
pub fn note_launch(vel: [f32; 2]) {
    if let Ok(mut l) = LAUNCH.lock() {
        *l = Some(Some(vel));
    }
}

/// KH2's glide would start now (t321, Glide at KH2): KH2's Glide equipped (its level kept aside
/// counts: `KH2_GLIDE_KEPT`) and `above` (his height over the floor) past its start height.
fn kh2_glide_open(above: f32) -> bool {
    if !super::styles::kh2(super::styles::Kind::Glide) || !above.is_finite() {
        return false;
    }
    // (The controller busy this step: its start height as last read, as `kh2_air_dodge` does.)
    static START: AtomicU32 = AtomicU32::new(u32::MAX);
    let read = DRIVER.try_lock().ok().and_then(|d| {
        d.as_ref().map(|d| {
            let level = (d.character.abilities.glide as u32).max(KH2_GLIDE_KEPT.load(Ordering::Relaxed)).min(u8::MAX as u32) as u8;
            d.character.growth_level(level).map(|g| g.glide_start_height)
        })
    });
    let start = match read {
        Some(h) => {
            START.store(h.map_or(u32::MAX, f32::to_bits), Ordering::Relaxed);
            h
        }
        None => Some(START.load(Ordering::Relaxed)).filter(|b| *b != u32::MAX).map(f32::from_bits),
    };
    start.is_some_and(|h| above > h)
}

/// What KH2 has for Square on the ground (the controller's abilities, from KH1's menu): (a
/// dodge roll, Guard or Quick Run; Quick Run).
fn kh2_square() -> (bool, bool) {
    DRIVER
        .try_lock()
        .ok()
        .and_then(|d| {
            d.as_ref().map(|d| {
                let a = &d.character.abilities;
                (a.dodge_roll > 0 || a.guard || a.quick_run > 0, a.quick_run > 0)
            })
        })
        // (Not known yet: as if, so the press is KH2's as before.)
        .unwrap_or((true, false))
}

/// The frame of KH2's dodge roll where held Square turns it into Quick Run (its clip's Quick Run
/// check event; 16 if the clip has none).
fn quick_run_check() -> f32 {
    static AT: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    let Some(kh2) = pose::kh2() else { return 16.0 };
    *AT.get_or_init(|| {
        kh2.character
            .moveset
            .clips
            .get(&character_core::motion::DODGE_ROLL)
            .and_then(|c| c.events.iter().find(|e| e.kind == character_core::EventKind::QuickRunCheck))
            .map_or(16.0, |e| e.frame as f32)
    })
}

/// KH2's stored launch speed (`Character::launch_vel`, the Steam exe's `+0xE0` with its flag) as KH2's
/// controller had it when it gave him back in the air (t331): the glide's borrow later in the same
/// air time starts from it, as if KH2 had kept him (an Aerial Dodge, then KH1's fall, then the
/// glide: the dodge's speed comes back). `Some(None)`: KH2's glide ended there (the flag cleared).
/// Cleared on the ground.
static LAUNCH: Mutex<Option<Option<[f32; 2]>>> = Mutex::new(None);

/// KH2 borrows him for its Quick Run out of KH1's roll (`shared::QUICK_RUN`): started on KH2's
/// first frame.
static QUICK_RUN_NOW: AtomicBool = AtomicBool::new(false);

/// KH1's control step with base Sora in its own states (`shared`): the borrows' presses and the
/// glide inputs the styles give elsewhere taken out of its button state for the step; KH1's glide
/// on Square (`glide = square`, Glide at KH1) fed as its held jump while it glides.
unsafe fn shared_control(base: usize, actor: usize, dir: usize, f: unsafe extern "system" fn(usize, usize)) {
    let can: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + CAN_ACT_RVA);
    let state = rd::<i32>(actor + 0x70);
    let y = rd::<[f32; 3]>(actor + 0x10)[1];
    let was = f32::from_bits(RISE_Y.swap(y.to_bits(), Ordering::Relaxed));
    let held = rd::<u32>(base + BUTTONS_RVA + 4);
    let kh2_sq = kh2_square();
    // (A KH2 cast cut by a jump or Square press in its reaction-command window: `formmagic::cast_cut`.)
    super::formmagic::cast_cut(actor, held & 6 != 0, pad_square());
    let seen = super::shared::Seen {
        state,
        can_act: can(actor) != 0,
        // (KH1's takeoff crouch, state 1, then its rise in state 2 until its air time reaches its
        // time to the apex, `+0x3B0` < `+0x3B8`; KH1's world is y-down.)
        rising: state == 1 || state == 2 && rd::<f32>(actor + 0x3B0) < rd::<f32>(actor + 0x3B8) && y <= was,
        jump: held & 6 != 0,
        square: pad_square(),
        air_dodge: kh2_air_dodge(),
        superglide: glide_on_square_now(base) && !super::settings::get().glide_on_square,
        kh2_glide: kh2_glide_open(rd::<f32>(actor + 0x3D4)),
        // (KH1's motion's combo window, `0x14029CE20`: its event flags `+0x1CC` bit 22.)
        combo_window: rd::<u32>(actor + 0x1CC) & super::combomods::KH1_COMBO_WINDOW != 0,
        launched: super::combomods::launched(actor),
        kh2_square: kh2_sq.0,
        quick_run: kh2_sq.1,
        kh1_rolling: state == 0 && rd::<u32>(actor + 0x164) == super::styleroll::ROLL_MOTION,
        kh1_roll: rd::<u32>(base + ROLL_ABILITY_RVA) & ROLL_ABILITY != 0,
        moving: move_tilt() > 0.15,
        quick_run_check: quick_run_check(),
        dt: super::shared::physics::dt(),
    };
    let hide = super::shared::control(&seen);
    // (KH1's glide state is base Sora's at Glide KH1 only, t321: at KH2 the glide is KH2's.)
    let gliding = state == KH1_GLIDE_STATE && (super::settings::get().glide_on_square || seen.superglide) && seen.square;
    let inject: u32 = if gliding { super::shared::KH1_JUMP_BITS | super::shared::KH1_ACTION_BIT } else { 0 };
    let st = base + BUTTONS_RVA;
    let saved = rd::<[u32; 6]>(st);
    for off in [4usize, 8, 0x10, 0x14] {
        wr(st + off, rd::<u32>(st + off) & !hide);
    }
    wr(st + 4, rd::<u32>(st + 4) | inject);
    let req_before = rd::<u32>(actor + 0x164);
    let words_masked = rd::<[u32; 6]>(st);
    f(actor, dir);
    let req_after = rd::<u32>(actor + 0x164);
    // (t320: KH1 asked for its glide motion (0x71..0x74) in this step; with what it saw, to find
    // the button word its glide reads when the dodge's press is meant to be hidden.)
    if matches!(req_after, 0x71..=0x74) && req_before != req_after && matches!(state, 1 | 2) {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 60 {
            let (dodged, up) = super::shared::dodge_flags();
            log(&format!(
                "glide asked in shared_control: KH1 state {state} -> {}, request {req_before:#x} -> {req_after:#x}, hide {hide:#x}, KH1 saw buttons {words_masked:08x?} (real {saved:08x?}), AIR_DODGED {dodged}, DODGE_JUMP_UP {up}, air_dodge {}, superglide {}, rising {}, can_act {}",
                rd::<i32>(actor + 0x70),
                seen.air_dodge,
                seen.superglide,
                seen.rising,
                seen.can_act,
            ));
        }
    }
    wr(st + 4, rd::<u32>(st + 4) & !inject);
    for off in [4usize, 8, 0x10, 0x14] {
        wr(st + off, rd::<u32>(st + off) | (saved[off / 4] & hide));
    }
}

/// Frames of the current rise, and whether a slope's contact (collision bit 3) has been there
/// since its first frames (a roof he takes off from, not a ceiling met on the way up).
static RISE_FRAMES: AtomicU32 = AtomicU32::new(0);
static SLOPE_RUN: AtomicBool = AtomicBool::new(false);

/// One of KH1's moves has him standing again and free (its ground state, KH1's own can-act
/// test): the jump button is KH2's from here (the user: after a spell, a jump on the first free
/// frame was KH1's high jump, "resulting in many bugs").
/// KH1 has him free in its jump or fall (state 1 / 2, its can-act test passes), not in a spell's
/// air cast.
unsafe fn kh1_air_free(base: usize, actor: usize) -> bool {
    let can_act: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + CAN_ACT_RVA);
    matches!(rd::<i32>(actor + 0x70), 1 | 2) && can_act(actor) != 0 && air_cast().is_none()
}

unsafe fn kh1_move_done(base: usize, actor: usize) -> bool {
    let can_act: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + CAN_ACT_RVA);
    rd::<i32>(actor + 0x70) == 0 && can_act(actor) != 0
}

/// KH1's move ended on a jump press: KH2 jumps at once (the press isn't taken as held over).
static JUMP_NOW: AtomicBool = AtomicBool::new(false);
/// Sora's height (KH1's y, down positive) on the last control step, to tell KH1's jump rising.
static RISE_Y: AtomicU32 = AtomicU32::new(0);
/// KH1's spell could be cut by another now (`combat`: a spell chosen then is taken, else refused).
pub static SPELL_CUT_OPEN: AtomicBool = AtomicBool::new(false);
/// KH2's glide level while the Glide style is KH1's (its ability set to 0 meanwhile).
static KH2_GLIDE_KEPT: AtomicU32 = AtomicU32::new(0);
/// KH1's glide was tried on this hold of Square (`style_air_start`, KH1's jump).
static GLIDE_TRIED: AtomicBool = AtomicBool::new(false);
/// A fresh jump press while KH1 has him on the ground in one of its moves (`takeover`).
static GROUND_JUMP: AtomicBool = AtomicBool::new(false);
static JUMP_PREV: AtomicBool = AtomicBool::new(false);
/// KH2 jumps on its first frame with input (frames left to wait for one).
static PENDING_JUMP: AtomicU32 = AtomicU32::new(0);
const PENDING_JUMP_FRAMES: u32 = 30;
/// The jump button for KH2 out of KH1's move: 1 held over (no jump), 2 not held, 0 nothing to set.
static JUMP_HELD_OVER: AtomicU8 = AtomicU8::new(0);
/// KH2's frames after KH1's move still logged in the air.
static JUMP_WATCH: AtomicU32 = AtomicU32::new(0);
/// KH1 landed him out of an air cast: KH2 starts in its landing.
static LAND_NOW: AtomicBool = AtomicBool::new(false);

/// A command KH1 runs (a spell): when it started and how many attack objects of Sora's were
/// live then (a new one is the spell's shot going out).
static COMMAND: Mutex<Option<Command>> = Mutex::new(None);
/// KH1's command was cut by an Attack in its end frames: KH2's combo goes on.
static COMBO_ON: AtomicBool = AtomicBool::new(false);
/// The combo slot the KH1 spell being cast took (`Character::magic_step`): 0 none (not a
/// spell), 1 a step, 2 a finisher with Finishing Plus leaving another, 3 the combo's last.
pub static SPELL_SLOT: AtomicU8 = AtomicU8::new(0);
/// A spell was chosen during KH1's cast (`combat::command` / `take_chosen_command`): waiting in
/// the command queue, it cuts the cast once the spell is out (a spell into a spell).
pub static MAGIC_AFTER: AtomicBool = AtomicBool::new(false);
/// KH1's spell was cut by, or had waiting through it, an Attack or a spell: the combo goes on.
static SPELL_CUT: AtomicBool = AtomicBool::new(false);
/// KH2's motion on its last frame (`u32::MAX` while KH1 has Sora): a command chosen during a KH2
/// move waits only while that move plays (KH2's input buffer), not on a timer of its own (the
/// user: magic came out however much later it was; it is part of the combo string).
pub static KH2_MOTION: AtomicU32 = AtomicU32::new(u32::MAX);

/// Whether KH2 is still in the move a command was chosen in (`motion`, from `KH2_MOTION`).
pub fn same_move(motion: u32) -> bool {
    motion != u32::MAX && KH2_MOTION.load(Ordering::Relaxed) == motion
}

/// KH1 is casting a spell (a command's hand-off, Sora not yet free to act: once he is, the
/// hand-off may wait for input, and a spell chosen then is KH1's to run at once).
pub fn kh1_casting() -> bool {
    if !(HANDOFF.load(Ordering::Relaxed) && FROM_COMMAND.load(Ordering::Relaxed)) {
        return false;
    }
    let base = BASE.load(Ordering::Relaxed);
    let actor = unsafe { rd::<u64>(base + PARTY_RVA) } as usize;
    if actor < 0x10000 {
        return false;
    }
    let can_act: unsafe extern "system" fn(usize) -> i32 = unsafe { std::mem::transmute(base + CAN_ACT_RVA) };
    unsafe { can_act(actor) == 0 }
}

/// KH2 may take Sora out of one of KH1's moves from this frame (`kh1_cut_frame`).
static KH1_WINDOW: AtomicBool = AtomicBool::new(false);
/// A Drive chosen in that window (`form::choose`): KH2 takes him to change.
pub static DRIVE_CUT: AtomicBool = AtomicBool::new(false);
/// The command KH1 runs was picked from the Items list (`combat::run_entry`).
pub static ITEM_COMMAND: AtomicBool = AtomicBool::new(false);

/// KH1 has Sora for an item's use now (an item command handed to KH1, `combat::command`).
pub fn item_use_now() -> bool {
    if !ITEM_COMMAND.load(Ordering::Relaxed) {
        return false;
    }
    if HANDOFF.load(Ordering::Relaxed) && FROM_COMMAND.load(Ordering::Relaxed) {
        return true;
    }
    // In KH1's own states (`shared`): from the command until its use motion has played (KH1's
    // use runs ~52 frames: `ITEM_CUT`).
    let in_use = super::shared::active() && ITEM_AT.try_lock().ok().and_then(|t| *t).is_some_and(|t| t.elapsed().as_secs_f32() < (ITEM_CUT as f32 + 4.0) / 60.0);
    if !in_use && super::shared::active() {
        ITEM_COMMAND.store(false, Ordering::Relaxed);
    }
    in_use
}

/// When an item's use started in KH1's own states (`shared`).
static ITEM_AT: Mutex<Option<Instant>> = Mutex::new(None);

pub fn item_started() {
    if let Ok(mut t) = ITEM_AT.lock() {
        *t = Some(Instant::now());
    }
}
/// Grounded KH1 Fire: KH2 may take him from this frame (the user, frame by frame).
const FIRE_CUT: u32 = 49;
/// An item's use: KH2's from this frame (the user: on its frame 52 KH1's roll, guard or jump
/// came out).
const ITEM_CUT: u32 = 51;

/// The frame of KH1's current move from which KH2 may take him, and whether it is an item's
/// (taken then whatever is pressed). For now grounded Fire (the user: "for now lets just add
/// this to kh1s fire until we get the logic setup correctly") and items.
unsafe fn kh1_cut_frame(state: i32) -> Option<(u32, bool)> {
    if state != 0 || air_cast().is_some() || super::form::in_form() {
        return None;
    }
    let c = COMMAND.try_lock().ok()?.as_ref().copied()?;
    match c.spell {
        Some(0) => Some((FIRE_CUT, false)),
        None if ITEM_COMMAND.load(Ordering::Relaxed) => Some((ITEM_CUT, true)),
        _ => None,
    }
}

/// KH2 may take Sora now out of KH1's move (its cut frame reached): a Drive chosen now is taken.
pub fn kh1_cut_window() -> bool {
    HANDOFF.load(Ordering::Relaxed) && KH1_WINDOW.load(Ordering::Relaxed)
}

pub fn command_started(shots: usize, spell: Option<u8>) {
    KH1_WINDOW.store(false, Ordering::Relaxed);
    DRIVE_CUT.store(false, Ordering::Relaxed);
    if let Ok(mut c) = COMMAND.lock() {
        *c = Some(Command { t: Instant::now(), low: shots, spell, logged: false });
    }
    // In a Drive Form, a KH1 spell it has no KH2 version of runs on the form's cast motion's
    // clock (the user: Gravity / Stop / Aero in Wisdom held KH1's whole animation; "the same logic
    // as thunder, available input window, when the animation is supposed to end"), and slides
    // as the form's casts do ("the same velocity in his casts as he does for the KH2 adjacent
    // spells").
    // (Base Sora too, with the Magic style at KH2 (`styles`): KH1's Gravity, Stop and Aero on his
    // own KH2 cast motions, the user's choice "KH1 spell, KH2 animation".)
    let air_now = AIR_CAST.try_lock().ok().is_some_and(|a| a.is_some());
    // (Unless that spell is set to cast KH1's whole way, `formmagic::kh1_cast`.)
    // (Not KH1's Cure with the Cure row at KH1: KH1's whole cast, its own animation and clock, the
    // tester, kh2-magic-and-reflect-native m4: "cure kh1 would cure exactly like kh1".)
    let base_look = spell.is_some_and(|sp| sp >= 4 && !super::formmagic::kh1_cast(sp)) && super::formmagic::base_kh2(air_now);
    super::formmagic::BASE_KH1_CAST.store(base_look, Ordering::Relaxed);
    let fc = spell.filter(|_| super::form::in_wisdom() || base_look).and_then(|sp| {
        let air = air_now;
        let level = unsafe { super::reflect::kh1_spell_level(sp) };
        let motion = super::formmagic::kh1_spell_motion(sp, level, air);
        let kh2 = pose::kh2()?;
        let form = if base_look { &kh2.character } else { &kh2.wisdom.as_ref()?.character };
        let clip = form.moveset.clips.get(&motion)?;
        let react = clip.first(|k| matches!(k, character_core::WindowKind::AllowReactionCommand)).unwrap_or(clip.length);
        let allow = clip.windows.iter().find(|w| matches!(w.kind, character_core::WindowKind::AllowMovement)).map(|w| (w.start, w.end.unwrap_or(clip.length)));
        let p = &form.params;
        log(&format!("{}: KH1 spell {sp} on the cast motion {motion}'s clock: {} frames, input from {react}, moving {allow:?}", if base_look { "KH2 magic style" } else { "Drive Form" }, clip.length));
        // (His speed going in, KH2's last frame: no halt at the cast's start, the user.)
        let v = with_kh2(|_, o| [o.displacement[0], o.displacement[2]]).unwrap_or([0.0; 2]);
        let v = if v[0].is_finite() && v[1].is_finite() && v[0].hypot(v[1]) < 40.0 { v } else { [0.0; 2] };
        Some(FormCast { len: clip.length, react, allow, accel: p.acceleration, brake: p.brake, walk: p.walk_speed, run: p.run_speed, threshold: p.run_threshold, v, air })
    });
    if let Ok(mut f) = FORM_CAST.lock() {
        *f = fc;
    }
}

/// A KH1 spell cast in a Drive Form (`command_started`): the form's cast motion's length, its
/// input window, its allow-movement range and the form's ground inertia, and the slide so far
/// (per frame, KH2 x / z).
#[derive(Clone, Copy, Debug)]
struct FormCast {
    len: u32,
    react: u32,
    allow: Option<(u32, u32)>,
    accel: f32,
    brake: f32,
    walk: f32,
    run: f32,
    threshold: f32,
    v: [f32; 2],
    air: bool,
}

static FORM_CAST: Mutex<Option<FormCast>> = Mutex::new(None);

/// One frame of a form's KH1 cast (`frames` into it): on the ground he moves as the form's cast
/// motion lets him (the stick in its allow-movement range with the form's inertia, braking after),
/// through KH1's mover. Returns the cast's (length, input window).
unsafe fn form_cast_frame(actor: usize, frames: u32) -> Option<(u32, u32)> {
    let mut g = FORM_CAST.try_lock().ok()?;
    let fc = g.as_mut()?;
    if !fc.air && rd::<i32>(actor + 0x70) == 0 {
        let tilt = move_tilt();
        let dir = stick_dir(actor, tilt);
        let moving = fc.allow.is_some_and(|(a, b)| (a..b).contains(&frames));
        let speed = if tilt <= 0.0 { 0.0 } else if tilt >= fc.threshold { fc.run } else { fc.walk };
        // (KH2's physics steps every other frame.)
        if frames % 2 == 0 {
            fc.v = if moving && fc.accel > 0.0 {
                let t = dir * speed;
                [t.x * (1.0 - fc.accel) + fc.v[0] * fc.accel, t.y * (1.0 - fc.accel) + fc.v[1] * fc.accel]
            } else if moving {
                (dir * speed).to_array()
            } else {
                let b = [fc.v[0] * fc.brake, fc.v[1] * fc.brake];
                if b[0].hypot(b[1]) < 0.05 { [0.0; 2] } else { b }
            };
        }
        if fc.v[0] != 0.0 || fc.v[1] != 0.0 {
            let base = BASE.load(Ordering::Relaxed);
            let mv: MoveFn = std::mem::transmute(base + MOVE_RVA);
            mv(actor, &[fc.v[0], 0.0, fc.v[1], 0.0], 0);
        }
    }
    Some((fc.len, fc.react))
}

/// The command handed to KH1: when, the fewest attack objects of Sora's seen since (a swing's
/// hitbox still out at the handoff ends during the cast, so the spell's shot can come back to
/// the count it started at), and its spell (Fire 0 .. Aero 6) for a magic command.
#[derive(Clone, Copy)]
struct Command {
    t: Instant,
    low: usize,
    spell: Option<u8>,
    logged: bool,
}

/// The command's spell is out (KH2's cast motions take Attack from about 8 frames after their
/// release, their "end frames"): a new attack object of Sora's. Only spells that make none
/// (Cure, Stop) fall back on 55% of KH1's cast motion; the others wait for their shot (the
/// user: Fire could be cancelled before it shot), or at most KH1's whole cast motion.
unsafe fn spell_out(actor: usize) -> bool {
    let Ok(mut g) = COMMAND.try_lock() else { return false };
    let Some(c) = g.as_mut() else { return true };
    let now = super::combat::sora_shots(actor);
    let len = pose::sora_slot().filter(|s| *s > 13).and_then(pose::slot_frames).map_or(36.0, |n| n as f32);
    let secs = c.t.elapsed().as_secs_f32();
    let shot = now > c.low;
    c.low = c.low.min(now);
    let no_shot = matches!(c.spell, Some(3) | Some(5)) || c.spell.is_none();
    let wait = if no_shot { (len * 0.55 / 60.0).clamp(0.3, 1.2) } else { (len / 60.0).clamp(0.5, 2.5) };
    let out = shot || secs >= wait;
    if out && !c.logged {
        c.logged = true;
        log(&format!("command's spell {:?} out after {secs:.2} s ({})", c.spell, if shot { "its shot" } else { "KH1's cast motion's time" }));
    }
    out
}

/// KH1 shows Sora standing, walking or running: his motion's slot, or for a motion we don't
/// know (another moveset's), the last one KH1 asked for.
fn kh1_moving() -> bool {
    match pose::sora_slot() {
        Some(s) => s <= 7,
        None => super::probe::SORA_LAST_SLOT.load(Ordering::Relaxed) & 0xFFFF <= 7,
    }
}

/// KH1 has him: our last KH2 pose is no longer drawn (it froze him for a moment, the user:
/// walking into a ladder, the walk stopped before the climb); `pose` blends into KH1's own.
fn drop_pose() {
    if let Ok(mut p) = POSE.try_lock() {
        *p = None;
    }
}

fn release(why: &str) {
    super::gates::released();
    KH2_MOTION.store(u32::MAX, Ordering::Relaxed);
    let mut changed = false;
    if let Ok(mut w) = LAST_WHY.try_lock() {
        if w.as_str() != why {
            w.clear();
            w.push_str(why);
            changed = true;
        }
    }
    KH2_AIR.store(false, Ordering::Relaxed);
    unsafe { set_sparkle(false) };
    drop_pose();
    if let Ok(mut d) = DRIVER.try_lock() {
        if let Some(dr) = d.as_mut() {
            // Also a new reason while KH1 already has him (the user, s155: after a spell's
            // handoff KH1's fall played on and the log never said why KH2 didn't take him).
            // (A new reason always (at most 4 a second): past the 400 lines the log never said why
            // KH1 kept him, the user: KH1's animations from a boss's end on.)
            if dr.last_state.is_some() || changed {
                static LAST_MS: AtomicU64 = AtomicU64::new(0);
                let now = now_ms();
                if changed && now.saturating_sub(LAST_MS.swap(now, Ordering::Relaxed)) >= 250 || dr.logged < 400 {
                    dr.logged = dr.logged.saturating_add(1);
                    log(&format!("KH1 has Sora ({why})"));
                }
            }
            if dr.last_state.is_some() {
                // When KH2 has him again, its controller starts over (standing): whatever KH1
                // did meanwhile, KH2's jump or blow from before is over.
                FRESH.store(true, Ordering::Relaxed);
            }
            dr.last_state = None;
            dr.held = (true, true);
        }
    }
}

/// Why KH1 last had Sora (`release`), for the room lines.
static LAST_WHY: Mutex<String> = Mutex::new(String::new());

/// KH1's current world and room (u32 each; `0x1402AFC10` compares its flight table with them;
/// KH-1FM-AP-LUA reads them too).
const WORLD_RVA: Rva = Rva(0x2340E5C);
const ROOM_RVA: Rva = Rva(0x2340EC4);

/// The room Sora is in, when he came in, and whether its first seconds were logged.
struct RoomSeen {
    world: u32,
    room: u32,
    at_ms: u64,
    frames: u32,
    kh2_frames: u32,
    reported: bool,
}
static ROOM: Mutex<Option<RoomSeen>> = Mutex::new(None);

/// Each room entered goes in the log (the user: the log should say what happens on entering
/// a room; a friend's rooms "broke the mod"), and 3 s later who had Sora meanwhile: KH2 for how
/// much of it, and if KH1 has him, why.
unsafe fn room_watch(actor: usize, kh2: bool) {
    // KH1 keeping him a long while (not in one of its moves for us): every 10 s, why, and his
    // state (the user: KH1's animations from a boss's end on, no reason in the log).
    {
        static SINCE: AtomicU64 = AtomicU64::new(0);
        static LAST: AtomicU64 = AtomicU64::new(0);
        let now = now_ms();
        if kh2 || HANDOFF.load(Ordering::Relaxed) {
            SINCE.store(0, Ordering::Relaxed);
        } else if SINCE.load(Ordering::Relaxed) == 0 {
            SINCE.store(now, Ordering::Relaxed);
        } else if now.saturating_sub(SINCE.load(Ordering::Relaxed)) > 5000 && now.saturating_sub(LAST.load(Ordering::Relaxed)) > 10000 {
            LAST.store(now, Ordering::Relaxed);
            let base = BASE.load(Ordering::Relaxed);
            let why = LAST_WHY.try_lock().map(|w| w.clone()).unwrap_or_default();
            log(&format!(
                "KH1 has had Sora {:.0} s: {why}; KH1 state {}, slot {:?}, flags {:08x}, event {:#x}, script {:#x}, body seen {}, ON {}",
                now.saturating_sub(SINCE.load(Ordering::Relaxed)) as f32 / 1000.0,
                rd::<i32>(actor + 0x70),
                pose::sora_slot(),
                rd::<u32>(actor),
                rd::<u32>(base + EVENT_RVA),
                rd::<u32>(actor + 0x440),
                pose::sora_body_seen(500),
                ON.load(Ordering::Relaxed)
            ));
        }
    }
    let base = BASE.load(Ordering::Relaxed);
    let (world, room) = (rd::<u32>(base + WORLD_RVA), rd::<u32>(base + ROOM_RVA));
    let Ok(mut seen) = ROOM.try_lock() else { return };
    let now = now_ms();
    if seen.as_ref().is_none_or(|s| s.world != world || s.room != room) {
        log(&format!(
            "room: world {world:#x} room {room:#x} entered; {} (KH1 state {}, flags {:08x}, script {})",
            if kh2 { "KH2 has Sora".to_string() } else { format!("KH1 has Sora ({})", LAST_WHY.try_lock().map(|w| w.clone()).unwrap_or_default()) },
            rd::<i32>(actor + 0x70),
            rd::<u32>(actor),
            pose::ptr(rd::<u32>(actor + 0x440)) != 0,
        ));
        *seen = Some(RoomSeen { world, room, at_ms: now, frames: 0, kh2_frames: 0, reported: false });
    }
    let Some(s) = seen.as_mut() else { return };
    s.frames += 1;
    if kh2 {
        s.kh2_frames += 1;
    }
    if !s.reported && now.saturating_sub(s.at_ms) >= 3000 {
        s.reported = true;
        log(&format!(
            "room {:#x}-{:#x}, first 3 s: KH2 had Sora {} of {} frames; now {} (KH1 state {}, body seen {})",
            s.world,
            s.room,
            s.kh2_frames,
            s.frames,
            if kh2 { "KH2 has him".to_string() } else { format!("KH1 has him: {}", LAST_WHY.try_lock().map(|w| w.clone()).unwrap_or_default()) },
            rd::<i32>(actor + 0x70),
            pose::sora_body_seen(500),
        ));
    }
}

/// KH1's control step this frame (read once per frame, before anything can return early).
static FROZEN_NOW: AtomicBool = AtomicBool::new(false);
/// When KH1's control of him last came or went, and until when it is taken for a ride.
static FLICKER: std::sync::Mutex<std::collections::VecDeque<u64>> = std::sync::Mutex::new(std::collections::VecDeque::new());
static RIDE_UNTIL: AtomicU64 = AtomicU64::new(0);
static LAST_FROZEN: AtomicBool = AtomicBool::new(false);

/// KH2's controller for Sora. Its abilities: `menu` (the ini's `abilities`: testing MAX, or KH1's
/// menu and level).
fn new_driver(kh2: &pose::Kh2) -> Driver {
    let character = kh2.character.clone();
    log("KH2's controller ready for Sora");
    let mut anim = RigAnim::new(kh2.rig.clone());
    // KH2 snaps a finished action straight into idle; softened (the user: smoother out of a
    // combo).
    anim.queued_blend = 14.0;
    Driver { character, anim, held: (true, true), grounded: true, last_state: None, logged: 0, strikes: combat::Strikes::default(), fought: None, last_out: None, growth: menu::Growth::new(), swing: None, swing_frame: 0, on_top: None, drive: Default::default() }
}

/// KH2's movement numbers with Sora's abilities now (KH1's menu), for KH2's physics on KH1's own
/// states (`shared::physics`).
unsafe fn refresh_kh2_numbers(base: usize) {
    let Some(kh2) = pose::kh2() else { return };
    let Ok(mut g) = DRIVER.try_lock() else { return };
    let d = g.get_or_insert_with(|| new_driver(kh2));
    d.growth.update(base, &mut d.character);
    // (KH2 Sora's own numbers, not the controller's now: with physics KH1 those are KH1's,
    // `shared::physics::kh2_controller`.)
    let own = &kh2.character;
    let p = &own.params;
    let high = d.character.abilities.high_jump;
    let (jump, high_jump) = (p.jump_height, if high == 0 { p.jump_height } else { own.growth_level(high).map_or(p.jump_height, |g| g.high_jump_height) });
    super::shared::physics::set_numbers(super::shared::physics::Numbers {
        walk: p.walk_speed,
        run: p.run_speed,
        jump,
        high_jump,
        gravity: p.gravity,
        fall_max: p.fall_max,
        air_keep: p.air_keep,
        air_drag: p.air_drag,
    });
}

/// KH1's lock-on (R1) as KH2's lock: while it holds a target (`0x142DB3F10` its state, the
/// target's handle at `0x142DB3F18`, which the command menu aims every command at), KH2 locks the
/// same enemy, so its swings home in on and turn toward it. KH1 locks a point of the target
/// (`0x142DB3F1C`, an index into its collision record's lock points; `docs/kh1-lockon.md`): KH2
/// homes in on that point (the user: locked on Darkside's hand, Sora went for his body), wherever
/// the actor's root is; `targets` gets it.
unsafe fn kh1_lock(base: usize, targets: &mut Vec<TargetInfo>) -> Option<u32> {
    let locked = if rd::<u32>(base + LOCK_STATE_RVA) != 0 { pose::ptr(rd::<u32>(base + LOCK_TARGET_RVA)) } else { 0 };
    if locked > 0x10000 {
        let point: unsafe extern "system" fn(usize, i32, *mut [f32; 4]) -> f32 = std::mem::transmute(base + LOCK_POINT_RVA);
        let mut v = [0.0f32; 4];
        let r = point(locked, rd::<i32>(base + LOCK_TARGET_RVA + 4).max(0), &mut v);
        if v[..3].iter().all(|x| x.is_finite()) && r.is_finite() && r >= 0.0 {
            let r = r.min(500.0);
            // Its point and size as KH2's target shape (as `combat::bodies` has every target).
            let t = TargetInfo { id: locked as u32, pos: [v[0], -v[1], v[2]], radius: r.max(10.0), height: r.max(10.0) };
            match targets.iter_mut().find(|x| x.id == t.id) {
                Some(x) => *x = t,
                None => targets.push(t),
            }
        }
    }
    (locked > 0x10000).then(|| locked as u32).filter(|id| targets.iter().any(|t| t.id == *id))
}

/// What KH2's combo would take for a press now (`Character::would_take`, `combomods`), with base
/// Sora's targets and KH1's lock as KH2's step sees them: `from`, where KH1's own combo stands
/// (KH1 has him: KH2's controller is set where he is, facing as KH1 has him), or `None` for KH2's
/// own combo as it is (KH2 has him). With KH2's combo length there (Combo Plus, Negative Combo)
/// and its Negative Combo. `playing`: KH2's motion KH1 plays now for its own swing (`kh2slots`),
/// passed over by KH2's anti-repeat as KH2's own swing playing would be.
pub unsafe fn kh2_would_take(actor: usize, from: Option<character_core::ComboState>, playing: Option<u16>, air: bool, square: bool) -> Option<(character_core::ComboRow, u8, u8)> {
    kh2_would_take_launch(actor, from, playing, air, square).map(|(r, len, neg, _)| (r, len, neg))
}

/// The next `kh2_would_take*` tests KH2's line ahead (`combomods::drop_ahead`, along the
/// selector's direction there, `Character::selector_aim`) for row 1 (`Character::set_drop_ahead`):
/// the ground combo's press only, so the air, Square and hand-back asks never see it.
static DROP_TEST: AtomicBool = AtomicBool::new(false);

pub fn test_drop_ahead(on: bool) {
    DROP_TEST.store(on, Ordering::Relaxed);
}

/// `kh2_would_take`, with the hop KH2's row starts there (`Character::would_take_launch`: a row
/// with `jump`, its height and sideways velocity for that target).
pub unsafe fn kh2_would_take_launch(actor: usize, from: Option<character_core::ComboState>, playing: Option<u16>, air: bool, square: bool) -> Option<(character_core::ComboRow, u8, u8, Option<character_core::Launch>)> {
    let base = BASE.load(Ordering::Relaxed);
    let kh2 = pose::kh2()?;
    let mut g = DRIVER.try_lock().ok()?;
    let d = g.get_or_insert_with(|| new_driver(kh2));
    d.growth.update(base, &mut d.character);
    let mut targets = d.strikes.look(actor);
    let lock = kh1_lock(base, &mut targets);
    let tilt = move_tilt();
    let stick = if tilt > 0.0 { (stick_dir(actor, tilt) * tilt).to_array() } else { [0.0, 0.0] };
    // (Set as KH2's step sets them; KH1 has him, so KH2's controller starts over from KH1's facing
    // when it next has him anyway.)
    if from.is_some() {
        d.character.set_facing(rd::<f32>(actor + 0x3C));
    }
    d.character.set_lock(lock);
    d.character.set_auto_target(super::settings::get().auto_aim);
    d.character.set_face_lock(true);
    d.character.set_combo_direction_lock(true);
    let p = rd::<[f32; 3]>(actor + 0x10);
    let above = rd::<f32>(actor + 0x3D4);
    let env = Env { pos: [p[0], -p[1], p[2]], grounded: !air, height_above_ground: above.max(0.0), targets: &targets, ledges: &[] };
    // (KH2's selector's direction, to the primary target or along his facing, `0x1404028D0`.)
    let drop = DROP_TEST.swap(false, Ordering::Relaxed) && !air && super::combomods::drop_ahead(actor, d.character.selector_aim(&env));
    d.character.set_drop_ahead(drop);
    let (row, launch) = d.character.would_take_launch(from, playing, air, square, stick, &env)?;
    Some((row, d.character.combo_length(air), d.character.abilities.negative_combo, launch))
}

/// KH2's forced row `id` for base Sora now, KH1's own states having him (`Character::
/// would_take_row`: Retaliating Slash's row out of KH1's knock-away, `combomods`): whether KH2
/// would take it, at a new combo (the exe's counts reset to 1 / 1, `docs/aerial-recovery.md`), and
/// its hop, with base Sora's targets and KH1's lock as KH2's step sees them.
pub unsafe fn kh2_would_take_row(actor: usize, id: u8, air: bool) -> Option<(character_core::ComboRow, Option<character_core::Launch>)> {
    let base = BASE.load(Ordering::Relaxed);
    let kh2 = pose::kh2()?;
    let mut g = DRIVER.try_lock().ok()?;
    let d = g.get_or_insert_with(|| new_driver(kh2));
    d.growth.update(base, &mut d.character);
    let mut targets = d.strikes.look(actor);
    let lock = kh1_lock(base, &mut targets);
    let tilt = move_tilt();
    let stick = if tilt > 0.0 { (stick_dir(actor, tilt) * tilt).to_array() } else { [0.0, 0.0] };
    d.character.set_facing(rd::<f32>(actor + 0x3C));
    d.character.set_lock(lock);
    d.character.set_auto_target(super::settings::get().auto_aim);
    d.character.set_face_lock(true);
    d.character.set_combo_direction_lock(true);
    let p = rd::<[f32; 3]>(actor + 0x10);
    let above = rd::<f32>(actor + 0x3D4);
    let env = Env { pos: [p[0], -p[1], p[2]], grounded: !air, height_above_ground: above.max(0.0), targets: &targets, ledges: &[] };
    d.character.set_drop_ahead(false);
    let from = character_core::ComboState { count: 0, finishers: 1, hit: false, finisher_next: false };
    d.character.would_take_row(id, Some(from), air, stick, &env)
}

/// The stick now as KH2 reads it for base Sora (the world direction times its tilt, KH2's x / z;
/// 0 below KH1's dead zone): Aerial Recovery's direction (`combomods`).
pub unsafe fn stick_now(actor: usize) -> [f32; 2] {
    let tilt = move_tilt();
    if tilt > 0.0 {
        (stick_dir(actor, tilt) * tilt).to_array()
    } else {
        [0.0, 0.0]
    }
}

/// KH2's primary target for base Sora now (KH2's `+0xDC8`: KH1's lock as KH2's lock, else KH2's
/// auto target among his targets, as `kh2_would_take_launch` sees them), its point (KH2's y-up):
/// what KH2's turn windows turn him toward (`kh2slots::turn_to_lock`). Only asked inside such a
/// window (it looks his targets up).
pub unsafe fn kh2_primary_target(actor: usize) -> Option<[f32; 3]> {
    let base = BASE.load(Ordering::Relaxed);
    let kh2 = pose::kh2()?;
    let mut g = DRIVER.try_lock().ok()?;
    let d = g.get_or_insert_with(|| new_driver(kh2));
    let mut targets = d.strikes.look(actor);
    let lock = kh1_lock(base, &mut targets);
    d.character.set_lock(lock);
    d.character.set_auto_target(super::settings::get().auto_aim);
    let p = rd::<[f32; 3]>(actor + 0x10);
    let env = Env { pos: [p[0], -p[1], p[2]], grounded: rd::<i32>(actor + 0x70) == 0, height_above_ground: rd::<f32>(actor + 0x3D4).max(0.0), targets: &targets, ledges: &[] };
    d.character.primary_target_pos(&env)
}

/// KH2's hits on his body in the KH2 motion KH1's own states play on base Sora (our slot,
/// `kh2slots::body_strike`: Vicinity Break, Explosion, Magnet Burst), the play they belong to, and
/// whether one was live last frame (so the end of a window reaches the hit records once).
struct NativeHits {
    anim: RigAnim,
    motion: u16,
    last_t: f32,
    play: u64,
    live: bool,
}

static NATIVE_HITS: Mutex<Option<NativeHits>> = Mutex::new(None);

/// KH2's spell scripts on a KH1 frame of base Sora's in KH1's own states (`formmagic`: a KH2 spell
/// cast in KH1's own cast state, `formmagic::kh1_spell_start`): their frame as KH2's step runs it
/// (`formmagic::shots_step`), with KH2's motion KH1 plays in our slot (`kh2slots::now_playing`) and
/// KH1's time in it, the clip's magic-cast event when KH1's time passes it (the release, trigger
/// 21), KH1's facing, KH1's lock and targets. Nothing when no script of base Sora's runs.
pub unsafe fn native_spells(actor: usize) {
    static LAST: Mutex<Option<(u16, f32)>> = Mutex::new(None);
    // (Reflect's on KH1's cast state too, `reflect::native_step`.)
    let reflect = super::reflect::busy();
    if !super::shared::active() || super::form::in_form() || !(super::formmagic::busy() || reflect) {
        if let Ok(mut l) = LAST.try_lock() {
            *l = None;
        }
        return;
    }
    let Some(kh2) = pose::kh2() else { return };
    let playing = super::kh2slots::now_playing(actor);
    let mut events = Vec::new();
    if let (Some((m, t)), Ok(mut l)) = (playing, LAST.try_lock()) {
        let from = match *l {
            Some((lm, lt)) if lm == m && lt <= t => lt,
            _ => -1.0,
        };
        if let Some(c) = kh2.character.moveset.clips.get(&m) {
            events.extend(c.events.iter().filter(|e| (e.frame as f32) > from && (e.frame as f32) <= t).map(|e| e.kind));
        }
        *l = Some((m, t));
    }
    let Ok(mut dg) = DRIVER.try_lock() else { return };
    let d = dg.get_or_insert_with(|| new_driver(kh2));
    let base = BASE.load(Ordering::Relaxed);
    let mut targets = d.strikes.look(actor);
    let lock = kh1_lock(base, &mut targets);
    let air = matches!(rd::<i32>(actor + 0x70), 1 | 2 | 8);
    let yaw = rd::<f32>(actor + 0x3C);
    let (motion, t) = playing.unwrap_or((u16::MAX, 0.0));
    let out = Output {
        displacement: [0.0; 3],
        facing: yaw,
        state: StateName::Magic,
        motion,
        clip_frame: t as u32,
        hitboxes: Vec::new(),
        invulnerable: false,
        guarding: false,
        guard_boxes: Vec::new(),
        weapon_trail: false,
        locked: None,
        combo_count: 0,
        combo_live: false,
        events,
    };
    let battle = super::form::battle(&d.drive, kh2).clone();
    super::formmagic::shots_step(actor, &out, air, &mut d.strikes, &battle, lock);
    if reflect {
        super::reflect::native_step(actor, &out, air, &mut d.strikes, &battle, lock);
    }
}

/// Once a game frame with base Sora in KH1's own states (`kh2slots::frame`, after KH1 posed him):
/// the hit windows on his body (`Hitbox` with weapon 0) of the KH2 motion our slot plays, at KH1's
/// time in it (the clip's frame), landed as KH2 lands them and as KH2's controller's hits are
/// landed here (`combat::Strikes::strike_as`): KH2's rig posed at that frame where KH1 has him
/// (`RigAnim::pose_at`: his position, KH1's yaw as KH2's facing, the clip's root travel taken out
/// as KH1 moved him by it), the window's group of KH2 shapes swept from last frame's against
/// KH1's enemies' body parts, each hit through KH1's contact with Sora's KH1 row data and KH2's
/// ATKP (power, launch, hold, Magnet's pull), once per window and target as KH2's hit records
/// allow. KH2 updates once a game frame (delta 2 at 30 fps), so its windows are read at the
/// frame's time and the records' clocks move on by the frame's 60 Hz steps. A hit landed counts as
/// KH1's swing connecting (the combat info's `+0x50`, which KH1's Attack step reads for the next
/// press, `combomods`). Nothing when no such window is open (a few reads a frame while our slot
/// plays). The Keyblade's windows stay KH1's sphere's (`kh2slots::kh1_tracks`).
pub unsafe fn kh2_hits(actor: usize) {
    let Some(kh2) = pose::kh2() else { return };
    let Ok(mut g) = NATIVE_HITS.try_lock() else { return };
    let playing = if super::shared::active() && !super::form::in_form() { super::kh2slots::now_playing(actor) } else { None };
    let clip = playing.and_then(|(m, _)| kh2.character.moveset.clips.get(&m).map(|c| (m, c)));
    // (A new play: another motion, or the same from its start again.)
    let n = g.get_or_insert_with(|| NativeHits { anim: RigAnim::new(kh2.rig.clone()), motion: u16::MAX, last_t: 0.0, play: 0, live: false });
    // (KH1's time held, the game held (Frame Advance draws without updates): nothing new.)
    if playing.is_some_and(|(m, t)| m == n.motion && t == n.last_t) {
        return;
    }
    // KH2's guard windows on the Keyblade in that motion (the guard press's, its repel's, Guard
    // Break's): KH1 guard objects, as for KH2's controller's guard (`combat::guard_step`), so an
    // attack meeting one is stopped before it reaches him (`combomods::guard_repel` then). Those on
    // his body stay `hurt`'s (`kh2slots::body_guard`), but for the Aerial Dodge's (ATKP 406 on his
    // body all through 202, the tester's aerial-dodge-native plan item 3): KH1 guard objects too,
    // as KH2's controller made them for its dodge (`combat::guard_step`, all round), so an enemy's
    // attack object meets it (the parry) and not only his hit pass.
    let guards: Vec<(usize, u16, u8, u8)> = match (playing, clip) {
        (Some((m, t)), Some((_, c))) => c
            .windows
            .iter()
            .enumerate()
            .filter(|(_, w)| w.contains(t as u32))
            .filter_map(|(i, w)| match w.kind {
                character_core::WindowKind::Guard { attack, group, weapon } if weapon > 0 || m == character_core::motion::AERIAL_DODGE => Some((i, attack, group, weapon)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    let (open, motion, t) = match (playing, clip) {
        (Some((m, t)), Some((_, c))) => {
            if m != n.motion || t < n.last_t {
                n.play = n.play.wrapping_add(1);
            }
            n.motion = m;
            n.last_t = t;
            let f = t as u32;
            let open: Vec<(usize, u16, u8)> = c
                .windows
                .iter()
                .enumerate()
                .filter(|(_, w)| w.contains(f))
                .filter_map(|(i, w)| match w.kind {
                    character_core::WindowKind::Hitbox { attack, group, weapon: 0, .. } => Some((i, attack, group)),
                    _ => None,
                })
                .collect();
            (open, m, t)
        }
        _ => {
            n.motion = u16::MAX;
            (Vec::new(), 0, 0.0)
        }
    };
    if open.is_empty() && guards.is_empty() && !n.live {
        return;
    }
    let Ok(mut dg) = DRIVER.try_lock() else { return };
    let d = dg.get_or_insert_with(|| new_driver(kh2));
    let air = matches!(rd::<i32>(actor + 0x70), 1 | 2);
    let p = rd::<[f32; 3]>(actor + 0x10);
    let yaw = rd::<f32>(actor + 0x3C);
    if (!open.is_empty() || !guards.is_empty()) && !n.anim.pose_at(&d.character, motion, t, yaw, [p[0], -p[1], p[2]]) {
        return;
    }
    let _ = d.strikes.look(actor);
    // (Our swing ids: the play and the window, marked apart from KH2's controller's own.)
    let hitboxes = open.iter().map(|&(i, attack, group)| character_core::ActiveHitbox { swing: (1 << 63) | (n.play << 8) | i as u64, attack, group, weapon: 0 }).collect();
    let out = Output {
        displacement: [0.0; 3],
        facing: yaw,
        state: StateName::Attack,
        motion,
        clip_frame: t as u32,
        hitboxes,
        invulnerable: false,
        guarding: false,
        guard_boxes: guards.iter().map(|&(i, attack, group, weapon)| character_core::ActiveHitbox { swing: (1 << 63) | (n.play << 8) | i as u64, attack, group, weapon }).collect(),
        weapon_trail: false,
        locked: None,
        combo_count: 0,
        combo_live: false,
        events: Vec::new(),
    };
    let battle = super::form::battle(&d.drive, kh2).clone();
    let landed = d.strikes.strike_as(actor, &out, &n.anim, &mut d.character, &battle, air, steps() as f32, true);
    n.live = !open.is_empty() || !guards.is_empty();
    if landed > 0 {
        let info = pose::ptr(rd::<u32>(actor + 0x14C));
        if info >= 0x10000 {
            wr(info + 0x50, rd::<u32>(info + 0x50) | 1);
        }
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 60 {
            log(&format!("KH2's hits on his body in KH1's states: KH2 motion {motion} frame {t:.0}, {landed} landed (ATKP {:?})", open.iter().map(|o| o.1).collect::<Vec<_>>()));
        }
    }
}

/// Sora's KH2 abilities now (from KH1's menu), whoever has him (`combomods`).
pub fn with_abilities<R>(f: impl FnOnce(&character_core::Abilities) -> R) -> Option<R> {
    DRIVER.try_lock().ok().and_then(|d| d.as_ref().map(|d| f(&d.character.abilities)))
}

/// KH2's Retaliating Slash row, if equipped (`combomods`).
pub fn kh2_retaliating_slash() -> Option<u8> {
    DRIVER.try_lock().ok().and_then(|d| d.as_ref().and_then(|d| d.character.retaliating_slash_row()))
}

/// KH2's controller's combo now (KH2 has him, `combomods`): where it stands, whether it is going
/// (`combo_going`), whether Combo Master is on, and whether a finisher is playing.
pub fn kh2_combo() -> Option<(character_core::ComboState, bool, bool, bool)> {
    with_kh2(|c, _| (c.combo_state(), c.combo_going().0, c.abilities.combo_master, c.in_finisher()))
}

/// KH1's own combo out of KH2's move (`combat::command`, base Sora, shared states): KH1's own
/// states have him from now, not after its swing (so KH2's abilities still apply to the rest of
/// that combo, `combomods`), as the hand-off after a command does once KH1 lets him act.
pub fn shared_now() {
    FROM_COMMAND.store(false, Ordering::Relaxed);
    ITEM_COMMAND.store(false, Ordering::Relaxed);
    set_air_cast(None);
    super::stylecombo::clear();
    super::stylejump::clear();
    super::styleroll::clear();
    super::shared::started();
}

/// KH1's own states have base Sora from now (`shared`): a hand-off of ours, KH2's controller let
/// go (it starts over when it borrows him again).
fn to_kh1_states(why: &str) {
    hand_to_kh1(why);
    super::shared::started();
    HANDOFF_FRAME.store(true, Ordering::Relaxed);
    release(why);
}

/// One frame of KH1's own states with base Sora (`shared`): KH1 keeps him (false), or KH2's
/// controller borrows him from this frame for one of its moves (true: its step runs now, with
/// the press). A borrow: the Attack command with the combo style at KH2 where he is (`combat`'s
/// `ATTACK_AFTER`), a command of KH2's (`shared::want_kh2`: a KH2 spell, Reflect, a Drive), or
/// the buttons' (`shared::control`: Square, the jump in the air). Only while KH1 lets him act
/// (no buffer: a press KH1 can't let go for is dropped).
unsafe fn shared_frame(base: usize, actor: usize) -> bool {
    let state = rd::<i32>(actor + 0x70);
    refresh_kh2_numbers(base);
    if state != 2 {
        super::shared::physics::off_air();
    }
    if state == 0 {
        if let Ok(mut l) = LAUNCH.try_lock() {
            *l = None;
        }
    }
    let can_act: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + CAN_ACT_RVA);
    let free = can_act(actor) != 0;
    // His speed while KH1 moves him (per frame, KH2's way up): an air borrow goes on from it.
    let p = rd::<[f32; 3]>(actor + 0x10);
    let vel = {
        let mut last = HANDOFF_POS.lock().unwrap();
        let v = last.map(|l| ([p[0] - l[0], p[2] - l[2]], l[1] - p[1]));
        *last = Some(p);
        v
    };
    // KH1 holding him still (a cutscene's moment), for KH2's camera (`kh1_scene`).
    if FROZEN_NOW.load(Ordering::Relaxed) {
        FROZEN_FRAMES.fetch_add(1, Ordering::Relaxed);
    } else {
        FROZEN_FRAMES.store(0, Ordering::Relaxed);
    }
    // KH1's glide on Square (`glide = square`; at either Glide style, a shared move: KH2's glide
    // motions and numbers on it at KH2): a fresh Square while KH1 falls.
    // (A fresh press, as KH2's glide button is: a Square held through the Aerial Dodge doesn't
    // glide when KH1 has him back; the press has to be new.)
    let square = pad_square();
    static SQUARE_WAS: AtomicBool = AtomicBool::new(true);
    let square_fresh = square && !SQUARE_WAS.swap(square, Ordering::Relaxed);
    SQUARE_WAS.store(square, Ordering::Relaxed);
    if !square {
        GLIDE_TRIED.store(false, Ordering::Relaxed);
    }
    let falling = vel.is_some_and(|(_, up)| up < 0.0);
    // (Glide at KH1 only, t321: at KH2 Square is KH2's glide in KH1's glide state, `glide`.)
    if state == 2 && free && square_fresh && falling && super::stylejump::kh1_glide() && glide_on_square_now(base) && kh1_glide_open(base) && !GLIDE_TRIED.swap(true, Ordering::Relaxed) {
        if let Some(what) = kh1_glide_start(base, actor) {
            log(&format!("{what}, on Square (shared states)"));
        }
    }
    let attack = ATTACK_AFTER.swap(false, Ordering::Relaxed);
    let command = super::shared::take_want_kh2();
    let borrow = super::shared::take_borrow();
    // One of KH2's ability moves out of KH1's own combo or knock-back (`combomods`): taken where
    // KH2 would use it (a press KH1's combo took, Square in its combo window, the jump or Square
    // while knocked away), so not only on a free frame.
    let ability = super::combomods::take_want();
    // KH1's roll held into KH2's Quick Run: KH2 takes him then (KH1 let him act; what is left of
    // KH1's roll motion, if any, is set to its idle as when KH2 cuts a KH1 move).
    let quick_run = borrow == super::shared::QUICK_RUN && free;
    let what = if ability.is_some() {
        Some("one of KH2's ability moves out of KH1's own combo or knock-back")
    } else if quick_run {
        Some("Square held through KH1's roll: KH2's Quick Run (Quick Run equipped)")
    } else if !free {
        if attack {
            note_dropped_attack(state);
        }
        None
    } else if attack {
        Some("an Attack: KH2's combo (the combo style is KH2's)")
    } else if command {
        Some("a command of KH2's")
    } else if borrow == super::shared::SQUARE {
        Some(if super::styles::kh2(super::styles::Kind::DodgeRoll) {
            "Square: KH2's dodge roll / guard / Quick Run (the Dodge Roll style is KH2's)"
        } else {
            "Square while moving: KH2's Quick Run (KH1's Dodge Roll isn't equipped; Quick Run is)"
        })
    } else if borrow == super::shared::AIR_JUMP {
        Some("the jump in the air: KH2's Aerial Dodge")
    } else {
        None
    };
    let Some(what) = what else {
        release("KH1's own states (base Sora, shared)");
        HANDOFF_FRAME.store(true, Ordering::Relaxed);
        return false;
    };
    // KH2 borrows him: its controller starts over (`FRESH`) where he is, in the air if he is.
    let above = rd::<f32>(actor + 0x3D4);
    let air = matches!(state, 1 | 2) || above > STEP_DOWN;
    if air {
        // His speed from KH1's own fields in its air state (per 60 Hz frame); else his travel over
        // the frame, per 60 Hz frame too (`kh1_fall_speed`).
        *HANDOFF_FALL.lock().unwrap() = Some(kh1_fall_speed(actor, vel));
        AIR_START.store(true, Ordering::Relaxed);
        if borrow == super::shared::AIR_JUMP {
            AIR_MOVES.store(true, Ordering::Relaxed);
        }
        NO_GRAB.store(NO_GRAB_FRAMES, Ordering::Relaxed);
    }
    // The press that borrowed him is KH2's fresh press on its first frame.
    // (KH2's glide is no borrow any more: KH1's glide state runs it, `glide`.)
    JUMP_NOW.store(borrow == super::shared::AIR_JUMP, Ordering::Relaxed);
    ROLL_NOW.store(borrow == super::shared::SQUARE, Ordering::Relaxed);
    QUICK_RUN_NOW.store(quick_run, Ordering::Relaxed);
    if quick_run {
        if rd::<u32>(actor + 0x164) == super::styleroll::ROLL_MOTION {
            let setm: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
            setm(actor, 0, 0.0);
        }
        // (KH1's roll slot still up for a frame or two: not KH1's own action to take him back for.)
        CUT_LEFT.store(true, Ordering::Relaxed);
    }
    JUMP_HELD_OVER.store(if borrow == super::shared::AIR_JUMP { 2 } else { 0 }, Ordering::Relaxed);
    FLY_PRESS_GROUND.store(attack, Ordering::Relaxed);
    CARRY_FRAMES.store(if attack { CARRY_MAX } else { 0 }, Ordering::Relaxed);
    HANDOFF.store(false, Ordering::Relaxed);
    KH1_WINDOW.store(false, Ordering::Relaxed);
    ITEM_COMMAND.store(false, Ordering::Relaxed);
    set_air_cast(None);
    if state != 0 {
        let set: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + SET_STATE_RVA);
        set(actor, 0);
    }
    if let Some(m) = ability {
        // KH1's swing or reaction ends here, as a spell's end frames cut does: its "action
        // running" flag off, its motion request its idle (after the state change, which queues
        // KH1's own landing when it leaves the air state).
        if !free {
            wr(actor, rd::<u32>(actor) & !4);
            let setm: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
            setm(actor, 0, 0.0);
            AFTER_CUT_LOG.store(12, Ordering::Relaxed);
        }
        super::combomods::lent(m);
    }
    FRESH.store(true, Ordering::Relaxed);
    HANDOFF_POS.lock().unwrap().take();
    super::shared::lent();
    super::shared::physics::off_air();
    log(&format!("KH2 borrows him: {what} (KH1 state {state}, {above:.0} above the floor)"));
    true
}

/// His speed (per 60 Hz frame: sideways, up) as KH2's controller takes him in the air from KH1:
/// KH1's own fields in its air state (`shared::physics::kh1_velocity`: its air speed and its arc's
/// slope), else `vel`, his travel over the last game frame, divided by the frame scale (it was
/// taken as one 60 Hz frame's, so at 30 fps KH2 started at twice his speed). A frame missed would
/// read as a jump in speed: kept to what a fall can be.
unsafe fn kh1_fall_speed(actor: usize, vel: Option<([f32; 2], f32)>) -> ([f32; 2], f32) {
    let (h, up, from) = match super::shared::physics::kh1_velocity(actor) {
        Some((h, up)) => (h, up, "KH1's air speed and arc"),
        None => {
            let dt = super::shared::physics::dt();
            let (h, up) = vel.unwrap_or(([0.0; 2], 0.0));
            ([h[0] / dt, h[1] / dt], up / dt, "his travel")
        }
    };
    let n = h[0].hypot(h[1]);
    let k = if n > 20.0 { 20.0 / n } else { 1.0 };
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 30 {
        log(&format!("KH2 takes him in the air at {from}: sideways {:.1}/frame, up {up:.1}/frame (KH1 state {})", n * k, rd::<i32>(actor + 0x70)));
    }
    ([h[0] * k, h[1] * k], up.clamp(-30.0, 30.0))
}

/// KH2's move is over (`shared::give_back`): KH1's own states again, on the ground or falling
/// (KH1's fall state as an air command sets it: the air-time fields, KH2's sideways speed).
unsafe fn shared_give_back(base: usize, actor: usize, d: &mut Driver, back: super::shared::Back, out: &Output) {
    // (KH2's stored launch speed kept for a glide later in this air time, t331: from any borrow
    // that launched him, the Aerial Dodge's; KH1's glide state takes it, `glide::start`.)
    if matches!(back, super::shared::Back::Air) && d.character.launch_vel().is_some() {
        if let Ok(mut l) = LAUNCH.lock() {
            *l = Some(d.character.launch_vel());
        }
    }
    if matches!(back, super::shared::Back::Air | super::shared::Back::Land) {
        // (KH1's air state from KH2's motion, which this step wrote into its fields: KH1's own fall,
        // or KH2's arc on it with physics KH2, goes on at KH2's speed, `shared::physics`. For
        // `Land`, with the floor under him: KH1's air state lands him itself, its landing request
        // and hold, as out of any fall of its own.)
        let set: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + SET_STATE_RVA);
        set(actor, 2);
        super::shared::physics::kh1_takes_air(actor);
        wr(actor + 0x3D0, 0u32);
        wr(actor, rd::<u32>(actor) & !0x1_0000);
        // Falling: KH1's fall request now. Its leaving the ground state (the borrow's `set(0)`)
        // queued its landing, 0x07, which stood through KH2's move and showed as the landing pose
        // for the 1-2 frames before KH1's air step asked for its fall, 0x06 (the t314 Aerial Dodge
        // traces). Out of a landing (`Land`) it stays KH1's own.
        if matches!(back, super::shared::Back::Air) && rd::<u32>(actor + 0x164) == 0x07 {
            let setm: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
            setm(actor, 0x06, 0.0);
            // (The request raises KH1's "action running" bit, which would lock him out of acting
            // for the whole fall and landing, t315: off, as for the idle request elsewhere.)
            wr(actor, rd::<u32>(actor) & !4);
        }
    }
    d.last_state = None;
    d.held = (true, true);
    let how = match back {
        super::shared::Back::Air => "falling",
        super::shared::Back::Land => "landing (KH1's fall state on the floor: KH1's own landing)",
        super::shared::Back::Ground => "on the ground",
    };
    to_kh1_states(&format!("KH2's move is over ({:?}): KH1's own states {how}", out.state));
}

unsafe fn takeover(actor: usize) -> bool {
    let base = BASE.load(Ordering::Relaxed);
    if actor != rd::<u64>(base + PARTY_RVA) as usize {
        return false;
    }
    let frozen = !CONTROLLED.swap(false, Ordering::Relaxed) && CONTROL_HOOKED.load(Ordering::Relaxed);
    FROZEN_NOW.store(frozen, Ordering::Relaxed);
    // KH1's control of him coming and going every few frames while KH2 has him is one of its
    // rides (the user: the magic carpet out of the Cave of Wonders didn't move while KH2 had him
    // between): KH1 has him for 5 s (counted only while KH2 has him: KH1's own frames flicker
    // too, and counting them kept him KH1's in plain rooms, the user).
    let now = now_ms();
    if now < RIDE_UNTIL.load(Ordering::Relaxed) {
        release("a ride: KH1's control comes and goes");
        return false;
    }
    if rd::<u32>(base + EVENT_RVA) != 0 {
        SCRIPTED.store(false, Ordering::Relaxed);
        release("an event runs");
        return false;
    }
    // Not in his own body (the Heartless form): KH1 has him, its own moves and animations
    // (KH2's moves would leave that body standing in KH1's idle).
    if !pose::sora_body_seen(500) {
        release("Sora is in another body (not his own model)");
        return false;
    }
    // KH1's area scripts walk / turn him (zone entries and exits, doors): their pending
    // actions hang off `+0x440` (`0x14028CD60`); KH1 moves and animates him meanwhile.
    // Flag `0x1000000` (script commands 0x24 / 0x23, `0x14028C550`) is set from a door's turn
    // until the walk through it ends, and at a zone entry until he has walked in.
    let script = pose::ptr(rd::<u32>(actor + 0x440));
    let held_by_script = rd::<u32>(actor) & 0x0100_0000 != 0;
    SCRIPTED.store(script != 0 || held_by_script, Ordering::Relaxed);
    if script != 0 || held_by_script {
        let mut kinds = Vec::new();
        let mut a = script;
        while a > 0x10000 && kinds.len() < 8 {
            kinds.push(rd::<u8>(a + 0x10));
            a = rd::<u64>(a + 0x58) as usize;
        }
        release(&format!("KH1's script moves him, actions {kinds:?}, flags {:08x}", rd::<u32>(actor)));
        return false;
    }
    // Base Sora (`shared`): KH1's own states have him unless KH2's controller is in one of its
    // own moves. A jump let through out of KH2's move (`JUMP_OUT`) that KH1 took: KH1's jump.
    if super::shared::on() && !HANDOFF.load(Ordering::Relaxed) {
        let jumped = JUMP_OUT.swap(false, Ordering::Relaxed) && FRAME_KIND.load(Ordering::Relaxed) == 1 && rd::<i32>(actor + 0x70) == 1;
        if jumped || !super::shared::kh2_move() {
            to_kh1_states(if jumped { "KH1's jump out of KH2's move (shared states)" } else { "base Sora: KH1's own states (shared)" });
            return false;
        }
    }
    // KH1 jumped him on its own while KH2 had him (the user: holding jump and pressing L1; the
    // log: L1 + Circle, KH1's jump motion set from its own state step, its state 2 and jump slot
    // 12 as KH2 landed, then its air moves): KH2 keeps him, KH1 back in its ground state (its
    // jump slot is then a leftover, below).
    if !HANDOFF.load(Ordering::Relaxed) && FRAME_KIND.load(Ordering::Relaxed) == 1 {
        let state = rd::<i32>(actor + 0x70);
        if matches!(state, 1 | 2) && matches!(pose::sora_slot(), Some(8 | 9 | 12 | 13)) {
            let set: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + SET_STATE_RVA);
            set(actor, 0);
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 20 {
                log(&format!("KH1 jumped him under KH2 (state {state}, slot {:?}): taken back", pose::sora_slot()));
            }
        }
    }
    // KH1 took him for one of its own air moves (`grab`): until it has him standing again.
    // Base Sora (`shared`): any of KH1's moves, once it lets him act (a command's cut windows
    // until then), goes on in KH1's own states; KH2 borrows him only for its moves.
    if HANDOFF.load(Ordering::Relaxed) && super::shared::on() && !super::shared::active() {
        let can_act: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + CAN_ACT_RVA);
        let command = FROM_COMMAND.load(Ordering::Relaxed);
        if !command || can_act(actor) != 0 && !ATTACK_AFTER.load(Ordering::Relaxed) && !MAGIC_AFTER.load(Ordering::Relaxed) {
            FROM_COMMAND.store(false, Ordering::Relaxed);
            ITEM_COMMAND.store(false, Ordering::Relaxed);
            set_air_cast(None);
            super::stylecombo::clear();
            super::stylejump::clear();
            super::styleroll::clear();
            super::shared::started();
            log(&format!("KH1's move (state {}) goes on in KH1's own states (shared)", rd::<i32>(actor + 0x70)));
        }
    }
    if HANDOFF.load(Ordering::Relaxed) && super::shared::active() {
        if !shared_frame(base, actor) {
            return false;
        }
    } else if HANDOFF.load(Ordering::Relaxed) {
        let state = rd::<i32>(actor + 0x70);
        let frames = HANDOFF_FRAMES.fetch_add(1, Ordering::Relaxed);
        // The same in 60 Hz frames (`steps`: KH1's motions and KH2's clocks run at that rate
        // whatever the frame rate; at 60 fps the same count): `ticks` before this frame, this
        // frame's `n` of them after.
        let n = steps();
        let ticks = if frames == 0 {
            HANDOFF_TICKS.store(n, Ordering::Relaxed);
            0
        } else {
            HANDOFF_TICKS.fetch_add(n, Ordering::Relaxed)
        };
        // (Reached this frame: by its last 60 Hz frame.)
        let last_tick = ticks + n - 1;
        // Square there (a dodge roll right after jumping off a pole) is KH2's: KH1 doesn't see
        // it (`kh1_keeps_square`), and KH2 takes him at once to roll (the user: KH1 had started
        // its own roll, and he slid backwards when KH2 took over mid-way).
        let square = buttons() & SQUARE != 0;
        let attack = ATTACK_AFTER.load(Ordering::Relaxed);
        // Done with its move and free (KH1's own test `0x1402956E0`: no action running (flag
        // 4), not reeling, not held) with the stick held: KH2's, whatever animation KH1 walks
        // him in (the user: after a Hollow Bastion crystal, moving within a second walked him
        // in KH1's animations, slots it has besides its plain walk and run).
        let can_act: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + CAN_ACT_RVA);
        let stick = move_tilt() > 0.15;
        let free = can_act(actor) != 0 && stick;
        // Its states as it goes, for the log (what a fall out of it looked like).
        static LAST_STATE: AtomicU32 = AtomicU32::new(u32::MAX);
        if frames == 0 {
            LAST_STATE.store(u32::MAX, Ordering::Relaxed);
        }
        if LAST_STATE.swap(state as u32, Ordering::Relaxed) != state as u32 {
            log(&format!("KH1's move: state {state} at {:.0} above the floor (can act {})", rd::<f32>(actor + 0x3D4), can_act(actor) != 0));
        }
        // His speed while KH1 moves him (per frame, KH2's way up), for a fall handed over.
        let p = rd::<[f32; 3]>(actor + 0x10);
        let vel = {
            let mut last = HANDOFF_POS.lock().unwrap();
            let v = last.map(|l| ([p[0] - l[0], p[2] - l[2]], l[1] - p[1]));
            *last = Some(p);
            v
        };
        // Out of its move into its plain fall (state 2: off a bar or a pole, down from a ledge,
        // after a spell in the air), dropping: KH2's fall from there, at KH1's speed (the user:
        // jumping off a pole he fell in KH1's falling animation, until he landed).
        let above = rd::<f32>(actor + 0x3D4);
        // A spell in the air is cast in KH1's air state 2 itself, sinking as it goes (the log:
        // command 0x2 in the air, state 2 from its first frame, slot 80 the cast); taken then,
        // the spell was cut off and KH1's fall played on (the user, s155). So a command's fall is
        // KH2's only once the spell is done (he can act again), in state 1 or 2; a grab's (a bar,
        // a pole, a ledge) as soon as KH1 drops him in state 2.
        // A spell cast in the air: KH1 holds its cast pose (and won't let him act) until he
        // lands (the user: still KH1's animation, then its idle on landing). Its cast motion
        // played through (KH1's frames, at least 20) counts as done: the spell is out by then.
        static CAST: Mutex<(usize, u32)> = Mutex::new((usize::MAX, 0));
        let cast_done = {
            let slot = pose::sora_slot().unwrap_or(usize::MAX);
            let mut c = CAST.lock().unwrap();
            if frames == 0 || c.0 != slot {
                *c = (slot, 0);
            }
            c.1 += n;
            let len = pose::slot_frames(slot).unwrap_or(u32::MAX / 2).max(20);
            !(8..=13).contains(&slot) && slot != usize::MAX && c.1 >= len
        };
        // (A cast in the air plays through to the ground now, the user: "fire isn't playing the
        // full animation"; KH2 takes him on landing, `cast_landed` below.)
        let done = can_act(actor) != 0 || cast_done && state == 2 && air_cast().is_none();
        let command = FROM_COMMAND.load(Ordering::Relaxed);
        // (KH1's leap off a ladder or a pole, state 16, too: once it drops, KH2's fall, the user:
        // "jumping off a ladder or a pole puts sora into his KH1 fall"; the log: state 15, 16,
        // then 2 only some 35 above the floor.)
        let fall_state = if command { matches!(state, 1 | 2) && done } else { matches!(state, 2 | LEAP_OFF_STATE) || state == 1 && done };
        let falling = fall_state && above > STEP_DOWN && vel.is_some_and(|(_, up)| up < 0.0);
        // A jump out of KH1's move is a fresh press made while KH1 has him on the ground (its
        // landing, a ground move), not the button held from before (the user: "if you do kh1 air
        // attack then hold jump before you land you'll buffer a jump, this is unintended"): a
        // press in the air doesn't count, and once on the ground only a new press does.
        let jump_raw = buttons() & jump_button() != 0;
        if frames == 0 {
            GROUND_JUMP.store(false, Ordering::Relaxed);
            JUMP_PREV.store(jump_raw, Ordering::Relaxed);
        }
        let jump_fresh = jump_raw && !JUMP_PREV.swap(jump_raw, Ordering::Relaxed);
        // (No buffer, s249, the user: "comment out any code that is related to queueing/buffering
        // moves": only this frame's fresh press on the ground counts; s241 kept a press made
        // earlier in KH1's move for its end:)
        // if state != 0 {
        //     GROUND_JUMP.store(false, Ordering::Relaxed);
        // } else if jump_fresh {
        //     GROUND_JUMP.store(true, Ordering::Relaxed);
        // }
        // let ground_jump = GROUND_JUMP.load(Ordering::Relaxed);
        let _ = &GROUND_JUMP;
        let ground_jump = jump_fresh && state == 0;
        // KH1 jumped him itself out of its move (its air attack's landing reads the jump press
        // outside its control step, where the press is kept from it; the user's log: state 0, then
        // state 1 "can act" on the floor, KH1's high jump, and KH2 took him rising 25 a frame: "sora
        // still gets launched into the air", or on the stick KH2's run: "overwrites high jump with
        // KH2 run"). Caught on its first frame, still on the floor: KH1's jump undone, KH2's jump.
        let kh1_took_off = command && state == 1 && super::stylejump::mode() != super::stylejump::JUMP && above < STEP_DOWN && vel.map_or(true, |(_, up)| up >= -0.5);
        if kh1_took_off {
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 20 {
                log(&format!("KH1 jumped him out of its move ({above:.1} above the floor): KH2's jump instead"));
            }
        }
        let jump = ground_jump && kh1_move_done(base, actor) || kh1_took_off;
        // Free in KH1's jump or fall (state 1 / 2, KH1 lets him act, not a spell's air cast): its
        // jump or high jump off a frame KH2 should have had (the first frame after an air cast's
        // landing or an item, the user), or the fall after a ledge, a ladder or a pole. KH2 takes
        // him at once, rising or falling, with its own air moves (Aerial Dodge, Glide); KH1's
        // glide and Superglide never come out of it (its jump / Square are KH2's meanwhile,
        // `kh1_air_free`).
        let air_free = kh1_air_free(base, actor) && above > STEP_DOWN && vel.is_some();
        // (Off, s241, the user: "comment out anything related to having kh1 magic animations
        // transition into kh2 landing animation, its unneeded at the moment and causes more bugs
        // then fixes anything". KH1's air spell lands in KH1's own landing, and KH2 takes him
        // after it as after KH1's other moves. s238's way, kept for later:)
        // // A spell cast in the air (KH1's animation all through): when KH1 sets him down, its
        // // player step asks for KH2's landing instead of KH1's (`player_step`), and KH2 takes him
        // // here into it, its short hold and a jump out of it.
        // // (Should that step not have seen it, KH1's ground state on a later frame with the spell
        // // still KH1's does the same, logged: KH1's landing must not hold him.)
        // static GROUND: AtomicU32 = AtomicU32::new(0);
        // if frames == 0 || state != 0 {
        //     GROUND.store(0, Ordering::Relaxed);
        // }
        // let seen = AIR_SPELL_LANDED.swap(false, Ordering::Relaxed);
        // let late = air_cast().is_some() && state == 0 && !seen && GROUND.fetch_add(1, Ordering::Relaxed) >= 1;
        // if late {
        //     log("KH1's air spell on the ground without its landing seen: KH2's landing");
        // }
        // let cast_landed = air_cast().is_some() && (seen || late);
        let cast_landed = false;
        // KH1's jump or glide (`stylejump`, the Jump / Glide styles at KH1): which, if any.
        let style_air = super::stylejump::mode();
        // KH1's glide out of KH2's jump, down on the ground: KH2's landing, as an air spell's.
        let glide_landed = style_air == super::stylejump::GLIDE && state == 0;
        if glide_landed {
            log("KH1's glide landed: KH2's landing");
        }
        let cast_landed = cast_landed || glide_landed;
        // An Attack during a spell: KH2's once the spell is out (its end frames; the user: KH2's
        // magic can be cut by an attack to go on with the combo), not before (it would cut the
        // spell). In the air too.
        // The rest of KH1's cast can be cancelled the same way (the user: "animation cancel the
        // KH1 spells"): a jump, Square (roll, guard, glide) or moving the stick, once the spell
        // is out; none of them before (Square or the stick 10 frames in cut the cast itself).
        let command = FROM_COMMAND.load(Ordering::Relaxed);
        // What may cut a KH1 spell short, once it is out (spec: KH1 magic is a step of the combo
        // string): an Attack (the combo's next swing) or another spell (its next step), in the air
        // too; a jump or Square only on the ground and only out of a step (s168). Never the stick
        // (spec rule D: a direction pressed cancelled the cast), and nothing out of the combo's
        // last step (rule E: it plays to its end; a new combo after it).
        // An Attack or a spell chosen meanwhile: KH1 runs no command during its cast, so they
        // are read off its command record here (the user: no attack out of magic).
        if command && !done {
            match combat::take_chosen_command() {
                Some(1) => {
                    ATTACK_AFTER.store(true, Ordering::Relaxed);
                }
                Some(2) => {
                    MAGIC_AFTER.store(true, Ordering::Relaxed);
                }
                _ => {}
            }
        }
        let attack = attack || ATTACK_AFTER.load(Ordering::Relaxed);
        let slot = SPELL_SLOT.load(Ordering::Relaxed);
        // KH1 moves with a frame from which KH2 may take him (`kh1_cut_frame`; the user: "by a
        // specified frame kh2 is allowed to take over kh1 sora only if conditions are met": at
        // that frame or later, an Attack or a Drive chosen). An item: KH2's at its frame, any
        // input or none (the user: its frame 52 gave KH1's roll, guard or jump).
        let cut_frame = if command && !done { kh1_cut_frame(state) } else { None };
        let in_cut = cut_frame.is_some_and(|(f, _)| last_tick + 1 >= f);
        // (KH1's own combo, `stylecombo`: a Drive may be chosen all through it, KH2 taking him out
        // of it to change, the user: "if sora is inputting his kh1 attacks he is unable to select
        // drive (it is blacked out)".)
        let combo_drive = super::stylecombo::active() && !super::form::in_form();
        KH1_WINDOW.store(in_cut || combo_drive, Ordering::Relaxed);
        let drive_cut = (in_cut || combo_drive) && DRIVE_CUT.swap(false, Ordering::Relaxed);
        let item_end = in_cut && cut_frame.is_some_and(|(_, item)| item);
        let last = slot == 3 && !in_cut;
        // (KH1's own combo plays as in KH1: none of the spell cuts, `stylecombo`.)
        let kh1_combo = super::stylecombo::active();
        let out_any = command && !done && !kh1_combo && (spell_out(actor) || in_cut);
        let out = out_any && air_cast().is_none();
        let jump_pressed = ground_jump;
        let magic_after = MAGIC_AFTER.load(Ordering::Relaxed);
        // In a Drive Form: the form's cast motion's clock (its input window, then its end).
        // (Its slide one 60 Hz frame at a time.)
        let form_cast = if command && !done { (ticks..ticks + n).fold(None, |_, t| form_cast_frame(actor, t)) } else { None };
        let window = form_cast.map_or(true, |(_, react)| last_tick >= react) && (cut_frame.is_none() || in_cut);
        // (At the form motion's end, KH2's idle, whether or not a shot of the spell was counted:
        // the user, Wisdom's Thunder ends at frame 49 and is in idle by 50, and KH1 held 17
        // frames more waiting for Aero's or Gravity's "shot".)
        // (`frames` counts from 0 on the cast's first frame: frame 50 of a 50-frame motion is
        // KH2's.)
        let form_clock = form_cast.is_some_and(|(len, _)| last_tick + 1 >= len) && command && !done;
        // (In the air too: KH2's fall from there.)
        let form_end = form_clock && air_cast().is_none();
        if form_end && !out_any {
            log("form's cast motion ended before a shot of the KH1 spell was counted: KH2 takes him anyway");
        }
        // (A form's KH1 spell takes an Attack at its cast motion's input frame, as the form's
        // Thunder does in a combo, whether or not KH1 has counted the spell's shot yet: the user,
        // "allow kh1 exclusive magic to be able to have its animation be interrupted by an attack
        // on the same frame as thunder".)
        let form_attack = form_cast.is_some() && command && !done;
        let attack_cut = attack && (out_any || form_attack) && !last && window;
        let magic_cut = magic_after && out_any && !last && window;
        // (A spell is taken during KH1's cast only while it could cut it now, `combat`: no queue.)
        SPELL_CUT_OPEN.store(out_any && !last && window, Ordering::Relaxed);
        let other_cut = out && (slot <= 1 || form_cast.is_some()) && window && (jump_pressed || square);
        let cancel = attack_cut || magic_cut || other_cut || form_end || drive_cut || item_end;
        let attack = attack && (!command || done || attack_cut);
        let square = square && (!command || done || other_cut);
        let jump = jump || other_cut && jump_pressed;
        if cancel {
            // KH1's cast action ends here: its "action running" flag off (as at its own end).
            wr(actor, rd::<u32>(actor) & !4);
            if slot == 0 {
                COMBO_ON.store(attack_cut, Ordering::Relaxed);
            }
            log(&format!(
                "spell's end frames cut by {} (combo slot {slot})",
                if attack_cut { "an Attack: the combo's next swing" } else if magic_cut { "a spell: the combo's next step" } else if other_cut && jump_pressed { "a jump" } else if other_cut { "Square" } else if drive_cut { "a Drive" } else if item_end { "the item's end frame" } else { "the form's cast motion ending" }
            ));
        }
        // (A spell chosen during the cast, which is now over: it runs at once from the queue.)
        let magic_waits = magic_after && done;
        // KH1's jump (`stylejump::JUMP`): KH1 keeps him in the air, KH2 takes him there only for
        // one of its own moves (an Attack for KH2's aerial combo, Square: Aerial Dodge or KH2's
        // glide, the jump press for KH2's glide on the jump button, a command chosen); no taking
        // him on the stick in the rise (state 1), only on the ground.
        let (square_pressed, jump_pressed_air) = super::stylejump::presses(buttons() & SQUARE != 0, buttons() & jump_button() != 0);
        let in_jump = style_air == super::stylejump::JUMP;
        // (The jump press in KH1's air is KH2's: its Aerial Dodge, or its glide on the jump button.
        // Square: KH1's glide with the Glide style at KH1, held while falling; else KH2's.)
        let kh2_glide_jump = in_jump && jump_pressed_air && matches!(state, 1 | 2);
        if buttons() & SQUARE == 0 {
            GLIDE_TRIED.store(false, Ordering::Relaxed);
        }
        let kh1_glide_here = in_jump && super::stylejump::kh1_glide() && buttons() & SQUARE != 0 && state == 2 && vel.is_some_and(|(_, up)| up < 0.0) && kh1_glide_open(base) && !GLIDE_TRIED.swap(true, Ordering::Relaxed);
        if kh1_glide_here {
            if let Some(what) = kh1_glide_start(base, actor) {
                super::stylejump::started(super::stylejump::GLIDE, true, buttons() & jump_button() != 0);
                log(&format!("{what}, out of KH1's jump"));
            }
        }
        let square_pressed = square_pressed && !super::stylejump::kh1_glide();
        let kh2_air = in_jump && (attack || square_pressed || kh2_glide_jump || super::stylejump::wanted());
        let air_free = if in_jump { air_free && kh2_air } else { air_free };
        let falling = falling && !in_jump;
        let jump = jump || kh2_glide_jump;
        let square = if in_jump { square_pressed || square && state == 0 } else { square };
        let ground_states = if in_jump { state == 0 } else { matches!(state, 0 | 1) };
        if in_jump && kh2_air && !air_free && matches!(state, 1 | 2) {
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 20 {
                log(&format!("KH1's jump: KH2's move asked, but KH1 doesn't let him act yet (state {state}, {above:.0} above the floor)"));
            }
        }
        // KH1's roll (`styleroll`): KH1 keeps him while its motion plays (its request still
        // pending on the first frames); an Attack KH1 took in it for KH2's combo hands him over at
        // once with the press.
        let in_roll = super::styleroll::active();
        let roll_attack = in_roll && super::styleroll::take_attack();
        let rolling = in_roll && state == 0 && (rd::<u32>(actor + 0x164) == super::styleroll::ROLL_MOTION || frames < 3 && rd::<u32>(actor + 0x178) != 0);
        if in_roll && frames < 4 {
            log(&format!("KH1's roll: frame {frames}, motion {:#x}, flags {:08x}", rd::<u32>(actor + 0x164), rd::<u32>(actor)));
        }
        let hold_roll = rolling && !roll_attack;
        if !hold_roll && (frames >= 1 && (cast_landed || roll_attack || kh1_took_off) || frames >= 2 && air_free || last_tick >= 10 && (falling || jump || cancel || magic_waits || ground_states && (kh1_moving() || free || square || attack || in_jump && super::stylejump::wanted() && can_act(actor) != 0 || in_roll && super::styleroll::wanted() && can_act(actor) != 0))) {
            let falling = falling || air_free;
            JUMP_NOW.store(jump || ground_jump, Ordering::Relaxed);
            if kh1_took_off {
                // (KH1's take-off undone: on the floor, no rise.)
                wr(actor + 0x3B0, 0.0f32);
                wr(actor + 0x3B8, 0.0f32);
                wr(actor + 0x3BC, 0.0f32);
            }
            // The press is KH2's jump on its first frame that takes input, however long KH1 holds
            // him still first (the user: "hold a direction + jump the jump will be overwritten by
            // the running animation": a stick hand-back in KH1's landing, then frozen frames
            // dropped the press). A button held from before KH1's move stays held for KH2 (its
            // last frame's press was the jump KH1's air attack came out of).
            // (No buffer, s249: the jump isn't kept for KH2's first frame with input.)
            // PENDING_JUMP.store(if jump || ground_jump { PENDING_JUMP_FRAMES } else { 0 }, Ordering::Relaxed);
            let _ = (&PENDING_JUMP, PENDING_JUMP_FRAMES);
            JUMP_HELD_OVER.store(if jump_raw && !(jump || ground_jump) { 1 } else { 2 }, Ordering::Relaxed);
            JUMP_WATCH.store(40, Ordering::Relaxed);
            // The spell's combo goes on only with an Attack or a spell that cut it or was chosen
            // during it (else he is back in idle: the combo is over, the user).
            SPELL_CUT.store(command && (attack_cut || magic_cut || magic_after || ATTACK_AFTER.load(Ordering::Relaxed)), Ordering::Relaxed);
            // An Attack waiting through a spell (pressed during it): the combo goes on after it,
            // as after KH2's own casts (the user: magic is part of the combo's queue).
            // (Not out of KH1's own combo: KH2's starts over after it.)
            if command && attack && SPELL_SLOT.load(Ordering::Relaxed) == 0 && !kh1_combo {
                COMBO_ON.store(true, Ordering::Relaxed);
            }
            // A spell chosen during the cast runs from the queue once KH2 has him (`resume_command`).
            MAGIC_AFTER.store(false, Ordering::Relaxed);
            // (Cut short by KH2: what's left of KH1's motion isn't KH1's own action.)
            CUT_LEFT.store(cancel, Ordering::Relaxed);
            // KH1's own motion of the move KH2 cut (an air spell's cast: its `+0x164` stayed the
            // cast's, 0x84 for Blizzard, then KH1 went on to its own landing 0x07 and its logic;
            // the user: "we can't have soras animation state and logic from that state act when we
            // try to set the kh2 landing animation"): KH1 is set to its idle motion at once
            // (`0x14029D7F0 (actor, 0, 0)`, its own motion request; no action flag).
            // (Set below, after KH1's state change: leaving its air state queues its own landing,
            // 0x07, which took the place of the idle asked before it; the user, s220: "nothing has
            // changed".)
            if cast_landed {
                // KH1's cast action ends here (it would at the end of its own landing): its
                // "action running" flag off, so its can-act test lets KH2's input through.
                wr(actor, rd::<u32>(actor) & !4);
                LAND_NOW.store(true, Ordering::Relaxed);
            }
            if falling {
                *HANDOFF_FALL.lock().unwrap() = vel.map(|_| kh1_fall_speed(actor, vel));
            }
            if form_cast.is_some() && !falling {
                *HANDOFF_SLIDE.lock().unwrap() = FORM_CAST.try_lock().ok().and_then(|f| f.map(|f| f.v));
            }
            let carried = ATTACK_AFTER.swap(false, Ordering::Relaxed);
            FLY_PRESS_GROUND.store(carried, Ordering::Relaxed);
            CARRY_FRAMES.store(if carried { CARRY_MAX } else { 0 }, Ordering::Relaxed);
            HANDOFF.store(false, Ordering::Relaxed);
            KH1_WINDOW.store(false, Ordering::Relaxed);
            ITEM_COMMAND.store(false, Ordering::Relaxed);
            set_air_cast(None);
            // KH1's state back to the ground one KH2 keeps him in, its exit steps run (the
            // user: after a spell in the air KH1 still had him in its air state 1 / 2 and its
            // air step glided when the jump button was held, KH1's glide under KH2), and KH2
            // starts in the air when he is.
            if state != 0 {
                let set: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + SET_STATE_RVA);
                set(actor, 0);
            }
            if cancel {
                // KH1's motion request (`0x14029D7F0` queues it: `+0x178` pending, `+0x17C` the
                // motion), the last one asked this frame: its idle.
                let setm: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
                let before = (rd::<u32>(actor + 0x164), rd::<u32>(actor + 0x178), rd::<u32>(actor + 0x17C));
                setm(actor, 0, 0.0);
                static N: AtomicU32 = AtomicU32::new(0);
                if N.fetch_add(1, Ordering::Relaxed) < 20 {
                    log(&format!(
                        "KH1's motion {:#x} of the cut move set to its idle (queued before: {} {:#x}; KH1 state {})",
                        before.0, before.1, before.2, rd::<i32>(actor + 0x70)
                    ));
                }
                AFTER_CUT_LOG.store(12, Ordering::Relaxed);
            }
            if rd::<f32>(actor + 0x3D4) > STEP_DOWN {
                AIR_START.store(true, Ordering::Relaxed);
            }
            ROLL_NOW.store(square, Ordering::Relaxed);
            FRESH.store(true, Ordering::Relaxed);
            HANDOFF_POS.lock().unwrap().take();
            if cast_landed {
                log("KH1's air cast landed: KH2's landing");
            } else if falling {
                NO_GRAB.store(NO_GRAB_FRAMES, Ordering::Relaxed);
                if air_free {
                    AIR_MOVES.store(true, Ordering::Relaxed);
                }
                log(&format!("KH1's move ended in its {} ({above:.0} above the floor): KH2's fall", if state == 1 { "jump" } else { "fall" }));
            } else {
                log(&format!("KH1's move ended (state {state}): KH2 again"));
            }
        } else {
            // An Attack pressed during KH1's move counts only on the frame it was pressed: if it
            // couldn't cut the move then (the spell not out yet, the combo's last step), it is
            // dropped, not kept for the move's end (the user: "We don't want a buffer type
            // system to be in the game and only inputs to be accepted when its in a acceptable
            // window").
            if ATTACK_AFTER.swap(false, Ordering::Relaxed) {
                note_dropped_attack(state);
            }
            HANDOFF_SEEN_MS.store(now_ms().max(1), Ordering::Relaxed);
            HANDOFF_FRAME.store(true, Ordering::Relaxed);
            release(&format!("KH1's own move, KH1 state {state}"));
            return false;
        }
    }
    // KH1 plays his death (`hurt`: HP gone, the stats' dying flag 4).
    let stats = pose::ptr(rd::<u32>(actor + 0x6C));
    if stats > 0x10000 && (rd::<u8>(stats) & 4 != 0 || rd::<i32>(stats + 0x3C) < 1) {
        release("he is down");
        return false;
    }
    // KH1's own movement modes KH2 has no part in: the Jungle Slider (state 30; the user: he
    // walked along the vines instead of sliding).
    // Flight (the user: Sora couldn't fly in Neverland): KH2 has his jumps, so KH1's fall state,
    // where flight starts, never ran. KH2's jump past its top, as high as KH1 wants, starts
    // KH1's own flight, which then has him (state 8) until he lands.
    let set_state: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + SET_STATE_RVA);
    let fly_attack = FLY_ATTACK.swap(false, Ordering::Relaxed);
    let flying = rd::<i32>(actor + 0x70) == FLIGHT_STATE;
    if fly_attack && flying {
        // Out of KH1's flight (its exit step runs) to the ground state KH2 keeps him in; KH2
        // starts in the air with the press.
        set_state(actor, 0);
        FLEW.store(true, Ordering::Relaxed);
        FLY_PRESS.store(true, Ordering::Relaxed);
        log(&format!("flight: Attack, KH2 takes him for an aerial combo ({:.0} above the floor)", rd::<f32>(actor + 0x3D4)));
    }
    if KH2_AIR.load(Ordering::Relaxed) && rd::<u32>(base + BATTLE_RVA) & FLIGHT_FLAG != 0 && matches!(rd::<i32>(actor + 0x70), 0 | 1) {
        // Coming down out of a jump or a finished air move (not mid-combo: KH2's aerial
        // attacks sink a little).
        let falling = DRIVER.try_lock().ok().and_then(|d| d.as_ref().and_then(|d| d.last_out.as_ref().map(|o| o.displacement[1] <= 0.0 && matches!(d.character.state_name(), StateName::Air | StateName::Jump)))).unwrap_or(false);
        let room: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(base + FLIGHT_ROOM_RVA);
        let above = rd::<f32>(actor + 0x3D4);
        let flew = FLEW.load(Ordering::Relaxed) && above > FLEW_ABOVE;
        if falling && (above > FLIGHT_ABOVE || flew || room(actor) != 0) {
            wr(actor + 0x3C4, 0.0f32);
            FLEW.store(false, Ordering::Relaxed);
            if above > FLIGHT_ABOVE {
                let start: unsafe extern "system" fn(usize) = std::mem::transmute(base + FLIGHT_START_RVA);
                start(actor);
            } else {
                set_state(actor, FLIGHT_STATE);
            }
            log(&format!("flight: KH1 flies him ({:.0} above the floor; state now {})", above, rd::<i32>(actor + 0x70)));
        }
    }
    if !HANDOFF.load(Ordering::Relaxed) && rd::<i32>(actor + 0x70) == KH1_GLIDE_STATE && KH2_AIR.load(Ordering::Relaxed) {
        set_state(actor, 0);
        log("KH1 started its glide under KH2: back to KH2's");
    }
    let state = rd::<i32>(actor + 0x70);
    let own_mode = kh1_mode(state);
    KH1_MODE.store(own_mode, Ordering::Relaxed);
    if own_mode {
        release(&format!("KH1's own movement mode (KH1 state {state})"));
        return false;
    }
    let slot = pose::sora_slot();
    // (Also a fall just handed over from one of KH1's moves, before KH2's first frame of it: KH1's
    // fall slot is still up then, and was taken for KH1's own action, so KH1 kept him falling
    // in its animation every time (the user: KH1's fall after a spell; the log: "KH2's fall",
    // then at once "KH1's own action, slot 10").)
    let handed_fall = AIR_START.load(Ordering::Relaxed) || HANDOFF_FALL.try_lock().map_or(false, |f| f.is_some());
    let airborne = handed_fall || DRIVER.try_lock().ok().and_then(|d| d.as_ref().map(|d| d.last_state.is_some() && d.character.airborne())).unwrap_or(false);
    // (A motion we don't know, as the Dream Shield's own moveset's, is not taken for KH1's own
    // action: the user saw KH1's idle and attacks with the shield.)
    // KH1's own jump / fall / landing slots (8..13) with KH1's ground state: what's left of a
    // fall KH2 has taken over (KH1 changes slot a few frames later), not an action of KH1's
    // (the user: on landing after a spell, KH1's idle, then a snap to KH2).
    let leftover_fall = matches!(slot, Some(8..=13)) && rd::<i32>(actor + 0x70) == 0;
    // A KH1 move KH2 cut short (`CUT_LEFT`): its motion plays on in KH1 a while, and isn't KH1's
    // to take him back for (the user: past frame 50 of a form's KH1 spell, KH1 still had him in
    // its Aero motion until the stick moved; an Attack then waited 21-25 frames).
    let cut_left = CUT_LEFT.load(Ordering::Relaxed) && matches!(slot, Some(s) if s > 7);
    if !matches!(slot, Some(s) if s > 7) {
        CUT_LEFT.store(false, Ordering::Relaxed);
    }
    if matches!(slot, Some(s) if s > 7) && !leftover_fall && !airborne && !FLY_PRESS.load(Ordering::Relaxed) && !cut_left {
        release(&format!("KH1's own action, slot {}, KH1 state {}", slot.map_or("?".into(), |s| s.to_string()), rd::<i32>(actor + 0x70)));
        return false;
    }
    let Some(kh2) = pose::kh2() else { return false };
    let Ok(mut g) = DRIVER.try_lock() else { return false };
    let d = g.get_or_insert_with(|| new_driver(kh2));
    if FRESH.swap(false, Ordering::Relaxed) {
        // Out of KH1's move: the jump button as held now, not as KH2 last saw it (`JUMP_HELD_OVER`).
        match JUMP_HELD_OVER.swap(0, Ordering::Relaxed) {
            1 => d.held.0 = true,
            2 => d.held.0 = false,
            _ => {}
        }
        // (KH1's own move left its speed in his push (`+0x120`): not carried into KH2's first
        // frames, the user: "it teleports sora forward at the start of the attack" after a KH1
        // spell.)
        NO_CARRY.store(3, Ordering::Relaxed);
        wr(actor + 0x120, [0.0f32; 3]);
        // Back from a command of KH1's (a spell, an item): that was fighting, so KH2's battle
        // stance with the Keyblade, as after an attack (the user: after a standing spell he went
        // back to the plain idle).
        if FROM_COMMAND.swap(false, Ordering::Relaxed) {
            d.fought = Some(Instant::now());
        }
        // Back from KH1's own move: KH2's controller starts over (standing), keeping its
        // abilities.
        let abilities = d.character.abilities.clone();
        let combo = d.character.combo_state();
        // (In a Drive Form, the form's.)
        d.character = super::form::character(&d.drive, kh2).clone();
        d.character.abilities = abilities;
        // After a KH1 spell: the combo goes on from the slot it took (not after the combo's last).
        let spell = SPELL_SLOT.swap(0, Ordering::Relaxed);
        // (Only when something went on from it: an Attack or a spell that cut it or waited
        // through it, `SPELL_CUT`. A spell played to its end leaves him in idle, and back in idle
        // the combo is over, the user.)
        let cut = SPELL_CUT.swap(false, Ordering::Relaxed);
        if COMBO_ON.swap(false, Ordering::Relaxed) && spell == 0 || matches!(spell, 1 | 2) && cut {
            d.character.set_combo_state(combo);
        }
        d.grounded = true;
        // An Attack press from before (during KH1's move) is not KH2's (one asked of KH1 then
        // is: `FLY_PRESS_GROUND`).
        combat::take_press();
    }
    if FLY_PRESS.load(Ordering::Relaxed) || AIR_START.swap(false, Ordering::Relaxed) {
        // Out of KH1's flight or one of its moves in the air: in the air where it had him.
        d.grounded = false;
    }
    if let Some(v) = HANDOFF_SLIDE.lock().unwrap().take() {
        // Out of a form's KH1 cast: his slide goes on (KH2's idle / walk / run by the stick).
        if d.grounded && !d.character.airborne() {
            d.character.carry_on_ground(v);
        }
    }
    if let Some((h, up)) = HANDOFF_FALL.lock().unwrap().take() {
        // Out of KH1's fall: falling on as KH1 had him (KH2's fall animation).
        d.grounded = false;
        d.character.carry_into_air(h, up);
        if AIR_MOVES.swap(false, Ordering::Relaxed) {
            d.character.allow_air_dodge();
        }
    }
    if ROLL_NOW.swap(false, Ordering::Relaxed) {
        d.held.1 = false;
    }
    if JUMP_NOW.swap(false, Ordering::Relaxed) {
        d.held.0 = false;
    }
    if LAND_NOW.swap(false, Ordering::Relaxed) {
        d.grounded = true;
        d.character.land_now();
        d.held.0 = false;
    }
    d.growth.update(base, &mut d.character);
    super::form::abilities(&mut d.drive, &mut d.character);
    // The physics setting: KH1's numbers in KH2's controller at physics KH1 (base Sora), the
    // moveset's own otherwise (`shared::physics::kh2_controller`).
    super::shared::physics::kh2_controller(actor, &mut d.character, super::form::character(&d.drive, kh2));
    // Glide at KH1 (`stylejump`): KH1's glide, none of KH2's.
    // (Its level kept aside and given back with the style at KH2: Sora's abilities are only set
    // again when KH1's menu changes, so a 0 left here took KH2's glide away for good, the user:
    // "unable to input a kh2 glide when glide is set to KH2, nothing comes out".)
    // (Base Sora in KH1's own states, `shared`: at Glide KH2 the glide is KH1's glide state with
    // KH2's glide in it (the tester, kh2-glide-in-kh1s-glide-state, `glide`), at Glide KH1 KH1's own;
    // KH2's controller glides only a Drive Form (its moveset's), so none of its glide in a borrow.)
    if super::stylejump::kh1_glide() || super::shared::on() {
        if d.character.abilities.glide != 0 {
            KH2_GLIDE_KEPT.store(d.character.abilities.glide as u32, Ordering::Relaxed);
            d.character.abilities.glide = 0;
        }
    } else if d.character.abilities.glide == 0 {
        let kept = KH2_GLIDE_KEPT.swap(0, Ordering::Relaxed);
        if kept != 0 {
            d.character.abilities.glide = kept as _;
        }
    }
    // KH1's roll held into KH2's Quick Run (`shared::QUICK_RUN`): along KH1's facing, from KH2's
    // first frame (Square held keeps it going, as out of KH2's own roll).
    if QUICK_RUN_NOW.swap(false, Ordering::Relaxed) {
        d.character.set_facing(rd::<f32>(actor + 0x3C));
        let ok = d.character.quick_run_now();
        log(&format!("KH2's Quick Run out of KH1's roll: {}", if ok { "started" } else { "not started (no Quick Run, or in the air)" }));
    }
    // Glide on Square, as KH1's Superglide was (the user), or on the jump button (ini `glide`).
    // (Not out of a flight: there Square is flight's descend.)
    // Air casts (Reflect) fall as they play, as KH1's spells do (the user).
    d.character.air_magic_falls = true;
    // KH1's own fall for them (the user: Reflect should land with KH1's gravity, closer to KH1's
    // physics): KH1's air state falls `KH1_FALL_ACCEL` × t² / 2 a frame at its 60 Hz step
    // (`0x1402AD3A0`: 0.4847 × 0.5 × t², the step `0x1423405DC` 1.0), at most `KH1_FALL_MAX` a
    // frame; `air_cast_fall` scales it.
    let g = d.character.params.gravity;
    d.character.air_magic_gravity = if g > 0.0 { super::settings::get().air_cast_fall * KH1_FALL_ACCEL / g } else { 1.0 };
    d.character.air_magic_fall_max = Some(KH1_FALL_MAX);
    // Final Form: KH2's own (its cast motions' no-gravity frames, KH2's gravity), as KH2 has it
    // (the user: "aerial final form reflect is not 1:1 physics wise with kh2").
    // Base Sora with physics KH2 (`shared::physics::source`, whatever the Aerial Magic style):
    // KH2's own air casts too (Reflect's with them), not KH1's fall. With physics KH1, KH1's fall
    // (the gravity above is KH1's then: `air_cast_fall` of it).
    let base_kh2 = !super::form::in_form() && super::shared::physics::source() == super::shared::physics::Rule::Kh2;
    if super::form::in_final() || base_kh2 {
        d.character.air_magic_falls = false;
        d.character.air_magic_gravity = 1.0;
        d.character.air_magic_fall_max = None;
    }
    // (t327: at Glide KH2, Square, as the KH2 rewrite binds it; at KH1 the glide is KH1's own.)
    let glide_square = super::styles::kh2(super::styles::Kind::Glide) || glide_on_square_now(base);
    d.character.glide_button = if glide_square && !FLEW.load(Ordering::Relaxed) { character_core::GlideButton::Action } else { character_core::GlideButton::Jump };
    if d.last_state.is_none() {
        // (Re)starting: face where KH1 had him facing.
        d.character.set_facing(rd::<f32>(actor + 0x3C));
    }

    // Input: direction from KH1's camera-relative move direction, tilt from the left stick.
    let buttons = buttons();
    let tilt = move_tilt();
    let dir = stick_dir(actor, tilt);
    let stick = if tilt > 0.0 { (dir * tilt).to_array() } else { [0.0, 0.0] };
    // KH2's first person: he stands (no stick, `move_tilt`) and turns 3° a frame while the stick
    // is past its dead zone (Steam KH2 `0x1403a9250`, `0x1403b7a80`).
    if let Some(t) = first_person_turn() {
        d.character.set_facing(d.character.facing() + t * super::shared::physics::dt());
    }
    // With L1 held the face buttons are KH1's shortcuts, not KH2's jump / action; a button
    // still held when L1 lets go stays KH1's until released (`held` sees it raw).
    // (A command submenu open: Circle backs out of it, not KH2's jump.)
    let l1_menu = super::shortcuts::menu_on_l1(buttons);
    let shortcut = l1_menu || super::shortcuts::submenu_open();
    let jump_raw = buttons & jump_button() != 0;
    let action_raw = buttons & SQUARE != 0;
    let jump = jump_raw && !shortcut;
    // (Square stays KH2's with a submenu open: only the L1 shortcut menu uses it; spec 3.2.)
    let action = action_raw && !l1_menu;
    // KH1's Attack command (Cross with Attack chosen in the menu) is KH2's attack button.
    let attack = combat::take_press() | FLY_PRESS.swap(false, Ordering::Relaxed);
    // A spell chosen and waiting for the move's window goes first: an Attack pressed meanwhile
    // doesn't take that window from it (the user: in a second finisher, magic then Attack, and
    // the Attack overrode the magic).
    let spell_first = combat::spell_waiting() || super::reflect::waiting() || super::formmagic::waiting();
    let attack = attack && !spell_first;
    // KH1 holding him still (its control step skipped or him unable to act: cutscene moments,
    // tutorial windows; the user: he moved anyway): KH2 gets no input (he still falls, lands
    // and finishes what he was doing).
    // (t327: one of KH2's moves in the air out of KH1's states, a borrow: the Aerial Dodge, KH2's
    // glide. The borrow sets KH1's state to 0 in the air, which queues KH1's landing and raises
    // its "action running" bit, so KH1's control step stops for the whole move: that isn't KH1
    // holding him still. Every borrowed glide ended on its second frame (its button read as let
    // go), and the dodge had no input at all, so Square couldn't glide out of it.)
    let borrow_air = super::shared::kh2_move() && d.character.airborne() && rd::<i32>(actor + 0x70) == 0 && rd::<u32>(actor) & 4 != 0;
    let frozen = FROZEN_NOW.load(Ordering::Relaxed) && !borrow_air;
    if LAST_FROZEN.swap(frozen, Ordering::Relaxed) != frozen {
        if let Ok(mut q) = FLICKER.lock() {
            q.push_back(now);
            while q.front().is_some_and(|t| now.saturating_sub(*t) > 3000) {
                q.pop_front();
            }
            if q.len() >= 10 {
                q.clear();
                RIDE_UNTIL.store(now + 5000, Ordering::Relaxed);
                log(&format!("KH1's control comes and goes (10 times in 3 s): KH1 has him for 5 s (KH1 state {}, flags {:08x})", rd::<i32>(actor + 0x70), rd::<u32>(actor)));
            }
        }
    }
    if FROZEN.swap(frozen, Ordering::Relaxed) != frozen {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        if n < 400 {
            // What he is doing (the user: the magic carpet after Genie Jafar didn't move).
            let on = pose::ptr(rd::<u32>(actor + 0x398));
            let detail = format!(
                "KH1 state {}, slot {:?} (last {}), flags {:08x}, +0x444 {:08x}, motion {:#x}, on {:#x} (object {})",
                rd::<i32>(actor + 0x70),
                pose::sora_slot(),
                super::probe::SORA_LAST_SLOT.load(Ordering::Relaxed) & 0xFFFF,
                rd::<u32>(actor),
                rd::<u32>(actor + 0x444),
                rd::<u16>(actor + 0x3EA),
                on,
                if on > 0x10000 { super::probe::object_id(on) as i64 } else { -1 }
            );
            log(&format!("{}; {detail}", if frozen { "KH1 holds Sora still: no input for KH2" } else { "KH1 lets Sora act again" }));
        }
    }
    // Held still on the ground for more than a frame (a cutscene starting, the user: he slid
    // on and snapped back): KH1 has him, so nothing of KH2's (a run's slide, a lunge) moves him.
    let still = if frozen { FROZEN_FRAMES.fetch_add(1, Ordering::Relaxed) + 1 } else { FROZEN_FRAMES.store(0, Ordering::Relaxed); 0 };
    if still >= 2 && !d.character.airborne() {
        if d.last_state.is_some() {
            if d.logged < 400 {
                d.logged += 1;
                log("KH1 has Sora (KH1 holds him still)");
            }
            FRESH.store(true, Ordering::Relaxed);
        }
        d.last_state = None;
        d.held = (true, true);
        // (A carried Attack waits these frames too, `CARRY_MAX` at most.)
        if FLY_PRESS_GROUND.load(Ordering::Relaxed) && CARRY_FRAMES.fetch_sub(1, Ordering::Relaxed) <= 1 {
            FLY_PRESS_GROUND.store(false, Ordering::Relaxed);
        }
        drop_pose();
        return false;
    }
    // An Attack carried out of a KH1 move's cut: KH2's on the first frame KH1 lets him act (else
    // dropped after `CARRY_MAX`), whatever waits (it was the cut's own press).
    let carried_attack = if FLY_PRESS_GROUND.load(Ordering::Relaxed) {
        if frozen {
            // (No buffer, s249: dropped at once when KH1 holds him still; s234 kept it up to
            // `CARRY_MAX` frames:)
            // if CARRY_FRAMES.fetch_sub(1, Ordering::Relaxed) <= 1 {
            FLY_PRESS_GROUND.store(false, Ordering::Relaxed);
            log("an Attack carried out of KH1's move: dropped (KH1 holds him still, no buffer)");
            // }
            false
        } else {
            FLY_PRESS_GROUND.store(false, Ordering::Relaxed);
            true
        }
    } else {
        false
    };
    // A jump pressed in KH1's move (`PENDING_JUMP`): KH2's first frame with input jumps, held or
    // tapped; dropped if KH1 holds him still for longer.
    // (Off, s249: no buffer.)
    // let forced_jump = if frozen {
    //     let _ = PENDING_JUMP.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| Some(n.saturating_sub(1)));
    //     false
    // } else {
    //     PENDING_JUMP.swap(0, Ordering::Relaxed) > 0
    // };
    let forced_jump = false;
    if forced_jump {
        log("a jump pressed in KH1's move: KH2's jump now");
    }
    let (stick, jump, action, attack) = if frozen { ([0.0, 0.0], false, false, false) } else { (stick, jump, action, attack || carried_attack) };
    if carried_attack {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 30 {
            log(&format!("an Attack carried out of KH1's move: KH2's now ({:?})", d.character.state_name()));
        }
    }
    let input = Input {
        stick,
        jump: Button { pressed: jump && !d.held.0 || forced_jump, held: jump },
        attack: Button { pressed: attack, held: attack },
        action: Button { pressed: action && !d.held.1, held: action },
        lock: Button::default(),
    };
    d.held = (jump_raw, action_raw);
    // Jump / Glide at KH1 (`stylejump`): KH1's jump on the fresh jump press, its glide on Square
    // held while falling.
    if !action_raw {
        GLIDE_TRIED.store(false, Ordering::Relaxed);
    }
    // (A fresh press of the glide button: Square with Superglide, else the jump button. Held
    // Square tried the glide every frame before t320.)
    let style_press = if glide_on_square_now(base) {
        if input.action.pressed { Some(true) } else { None }
    } else if input.jump.pressed {
        Some(false)
    } else {
        None
    };
    // (Not with KH1's own states owning base Sora, `shared`: KH1 jumps and glides him itself.)
    if let Some(glide_press) = style_press.filter(|_| !frozen && !shortcut && !super::shared::on()) {
        if let Some(what) = style_air_start(base, actor, d, glide_press) {
            hand_to_kh1(what);
            FROM_COMMAND.store(false, Ordering::Relaxed);
            super::stylejump::started(if what.contains("glide") { super::stylejump::GLIDE } else { super::stylejump::JUMP }, action_raw, jump_raw);
            d.last_state = None;
            d.held = (true, true);
            drop_pose();
            return false;
        }
    }
    // Dodge Roll at KH1 (`styleroll`): KH1's own roll, on the fresh Square with the stick.
    if input.action.pressed && !frozen && !shortcut && !super::shared::on() && style_roll_start(base, actor, d, dir, tilt) {
        hand_to_kh1("KH1's dodge roll (the Dodge Roll style is KH1's)");
        FROM_COMMAND.store(false, Ordering::Relaxed);
        super::styleroll::started();
        d.last_state = None;
        d.held = (true, true);
        drop_pose();
        return false;
    }

    let above = rd::<f32>(actor + 0x3D4);
    // KH1's own motion on KH2's first steps after a cut of a KH1 move (what KH1 still plays).
    if AFTER_CUT_LOG.load(Ordering::Relaxed) > 0 {
        let n = AFTER_CUT_LOG.fetch_sub(1, Ordering::Relaxed);
        log(&format!(
            "after the cut, step {}: KH1 motion {:#x} (+0x168 {:#x}, time {:.1}, queued {} {:#x}), KH1 state {}, {:.0} above the floor, KH2 {:?} grounded {}",
            13 - n,
            rd::<u32>(actor + 0x164),
            rd::<u32>(actor + 0x168),
            rd::<f32>(actor + 0x16C),
            rd::<u32>(actor + 0x178),
            rd::<u32>(actor + 0x17C),
            rd::<i32>(actor + 0x70),
            above,
            d.character.state_name(),
            d.grounded
        ));
    }
    let mut targets = d.strikes.look(actor);
    let lock = kh1_lock(base, &mut targets);
    if lock != d.character.lock() && d.logged < 400 {
        d.logged += 1;
        log(&format!("lock-on: {lock:?}"));
    }
    d.character.set_lock(lock);
    // KH2 aims a swing at any enemy in reach; here only at KH1's own target, the R1 lock or the
    // small cursor's soft target (the user: Sora swung toward points he wasn't targeting). The
    // other enemies near him still count for the combo rows that look at every enemy (the user:
    // Magnet Burst hardly came out: its row wants a foe off to the side; Explosion's and
    // Vicinity Break's likewise), so they stay in the list.
    let auto_aim = super::settings::get().auto_aim;
    d.character.set_auto_target(auto_aim);
    // KH2's combo keeps one direction (the user): the stick aims only a combo's first swing; a
    // target (R1's lock or KH1's cursor) is faced at the start of each swing, never mid-swing.
    d.character.set_face_lock(true);
    d.character.set_combo_direction_lock(true);
    // KH2's side steps in whole 60 Hz frames (`steps`: 2 a game frame at 30 fps), every step whole:
    // its controller, his move through KH1's collision, the animation, the hits and guard windows,
    // the effects and sounds, so no window or event is skipped and a KH2 move takes as long as at
    // 60 fps. The presses go to the first step (the next ones see them held); a KH1 move or
    // KH1's states taking him over ends the frame's steps there.
    let steps_now = steps();
    // KH2's guard windows open on any step of this frame, for KH1's guard objects and `hurt`
    // (KH1 collides once a game frame), and its invulnerability and guarding likewise.
    let mut frame_guards: Vec<character_core::ActiveHitbox> = Vec::new();
    let (mut invulnerable, mut guarding) = (false, false);
    let mut back = None;
    let mut last: Option<Output> = None;
    // Steps run this frame, and where Sora's pose was last asked for (the pose and the effects
    // are worked out once per game frame, after the steps).
    let mut ran = 0u32;
    let mut anim_pos = [0.0f32; 3];
    for step in 0..steps_now {
        ran += 1;
        let input = if step == 0 {
            input
        } else {
            Input { jump: Button { pressed: false, held: input.jump.held }, attack: Button::default(), action: Button { pressed: false, held: input.action.held }, ..input }
        };
        // Where he is, KH2's way up (KH1's world is y-down).
        let p = rd::<[f32; 3]>(actor + 0x10);
        let pos = [p[0], -p[1], p[2]];
        let above = rd::<f32>(actor + 0x3D4);
        let env = Env { pos, grounded: d.grounded, height_above_ground: above.max(0.0), targets: &targets, ledges: &[] };
        // A Reflect chosen from the Magic menu: KH2's cast as soon as its move allows (`reflect`).
        super::reflect::before_step(actor, &mut d.character);
        // A Drive chosen from page 2 (`form`): the change, the form's end.
        super::form::before_step(&mut d.drive, &mut d.character, &mut d.anim, kh2, actor);
        // A KH2 spell chosen in Wisdom Form: its cast (`formmagic`).
        super::formmagic::before_step(actor, &mut d.character);
        // KH2's ability move it borrowed him for out of KH1's own combo (`combomods`): its row and
        // KH1's combo count, the press (the frame's first step only).
        let input = if step == 0 { super::combomods::before_step(&mut d.character, input) } else { input };
        let mut out = d.character.step(&input, &env);
        // Out of flight for a combo: he stays up through it (the user: Sora should remain in
        // flight throughout the combo); KH2's aerial attacks sink between hits, so no sinking while
        // attacking (rises, as an upward finisher's, stay). Flight takes him again once it's done.
        let flew = FLEW.load(Ordering::Relaxed) && d.character.airborne();
        if flew && matches!(d.character.state_name(), StateName::Attack | StateName::Magic | StateName::AerialDodge) {
            out.displacement[1] = out.displacement[1].max(0.0);
            // Toward the height of what the swing goes for (KH2's own approach is level).
            if matches!(d.character.state_name(), StateName::Attack) {
                let aim = targets
                    .iter()
                    .filter(|t| Some(t.id) == lock || (lock.is_none() && auto_aim))
                    .map(|t| (t, (t.pos[0] - pos[0]).hypot(t.pos[2] - pos[2])))
                    .filter(|(_, h)| *h < FLY_AIM_REACH)
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((t, _)) = aim {
                    let gap = (t.pos[1] - FLY_AIM_BELOW) - pos[1];
                    if gap.abs() > 8.0 {
                        out.displacement[1] = (gap * FLY_AIM_RATE).clamp(-FLY_AIM_MAX, FLY_AIM_MAX);
                    }
                }
            }
        }
        // In flight's combo: not down onto the floor unless Square (descend) is held.
        let descend = buttons & SQUARE != 0;
        if flew && !descend && out.displacement[1] < 0.0 {
            out.displacement[1] = out.displacement[1].max(-(above - FLY_FLOOR).max(0.0));
        }
        set_sparkle(flew);
        KH2_AIR.store(d.character.airborne(), Ordering::Relaxed);

        // KH2's displacement: the sideways part through KH1's mover (walls, slopes, steps up); the
        // height is KH2's while he is in the air (KH1's mover snaps a grounded actor back onto the
        // floor every frame, which undid a jump's first small rises: the user, stage 8).
        let air = d.character.airborne();
        let rising = out.displacement[1] > 0.0;
        let up_before = -p[1];
        // (Where the game frame started from, and the objects touched over all of its steps.)
        if step == 0 {
            wr(actor + 0x490, rd::<[f32; 4]>(actor + 0x10));
        }
        // What KH1's physics step adds to a move (`0x140298230`), as it would: a carrying
        // platform's motion (`+0x200`, while `+0x160 & 4`) and the frame's push (`+0x120`,
        // cleared once used), and the touched-objects mask (`+0x90`) cleared before the move.
        let settings = super::settings::get();
        let fresh_frame = NO_CARRY.load(Ordering::Relaxed) > 0;
        if fresh_frame {
            NO_CARRY.fetch_sub(1, Ordering::Relaxed);
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 30 {
                log(&format!("KH2's first frames after KH1's move: {:?} (motion {}, frame {}), move {:.1?}, KH1's push {:.1?} not carried", out.state, out.motion, out.clip_frame, out.displacement, rd::<[f32; 3]>(actor + 0x120)));
            }
        }
        let mut carry = if settings.carry && !fresh_frame && step == 0 { carried(actor, &mut d.logged) } else { [0.0; 3] };
        // (In the air just after one of KH1's moves, KH1's push doesn't lift him: a push KH1's move or
        // landing left is no platform's, and a jump then went far too high, the user.)
        if JUMP_WATCH.load(Ordering::Relaxed) > 0 && d.character.airborne() && rd::<u32>(actor + 0x160) & 4 == 0 {
            carry[1] = 0.0;
        }
        // (The first KH2 frames after one of KH1's moves, in the air, logged: the user, KH2's high jump
        // out of KH1's landing after a KH1 air attack went "way higher than it should".)
        if JUMP_WATCH.load(Ordering::Relaxed) > 0 {
            if JUMP_WATCH.fetch_sub(1, Ordering::Relaxed) > 0 && d.character.airborne() {
                log(&format!(
                    "after KH1's move: {:?} up {:.2} carried {:.2?} push {:.2?} above floor {:.1} y {:.1}",
                    out.state, out.displacement[1], carry, rd::<[f32; 3]>(actor + 0x120), rd::<f32>(actor + 0x3D4), -rd::<f32>(actor + 0x14)
                ));
            }
        }
        if settings.carry && step == 0 {
            wr(actor + 0x90, 0u64);
        }
        // What KH2 asks KH1's mover for (`trace`).
        super::trace::asked(out.displacement, carry);
        let mv: MoveFn = std::mem::transmute(base + MOVE_RVA);
        // Rising: the rise goes through KH1's mover too, in KH1's fall state for the move so that
        // it collides as in the air (its collision query `0x140297F20` takes that from the movement
        // state; in the ground state it snaps him back onto the floor), which stops him at a
        // ceiling (KH1's own jump: `0x1402AD260`).
        // Falling too (the user: he couldn't land on boxes): KH1's fall state's mover stops him on
        // whatever is under him, objects included, as KH1's own fall does; the map floor alone
        // (`+0x320`) doesn't know boxes.
        let rise3d = settings.ceilings && (rising || air);
        // An attack's lunge doesn't walk him into a room exit (`gates`): its face event dropped,
        // this frame's move undone.
        let attack_move = matches!(out.state, StateName::Attack);
        let before_move = rd::<[f32; 4]>(actor + 0x10);
        super::gates::pre_move(actor, attack_move);
        super::gates::attacking(actor, attack_move);
        if rise3d {
            let state = rd::<i32>(actor + 0x70);
            wr(actor + 0x70, AIR_STATE);
            mv(actor, &[out.displacement[0] + carry[0], -out.displacement[1] + carry[1], out.displacement[2] + carry[2], 0.0], 0);
            wr(actor + 0x70, state);
        } else {
            mv(actor, &[out.displacement[0] + carry[0], 0.0, out.displacement[2] + carry[2], 0.0], 0);
        }
        if let Some(e) = super::gates::post_move() {
            wr(actor + 0x10, before_move);
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 40 {
                log(&format!("an attack's lunge touched map event {e}: kept out of it (the move undone)"));
            }
        }
        if settings.carry {
            wr(actor + 0x120, [0.0f32; 4]);
        }
        // The floor under where he is now (KH1 y-down: floor point `+0x320`).
        let floor_up = -rd::<f32>(actor + 0x324);
        let walked_up = -rd::<f32>(actor + 0x14);
        if air || rising {
            let mut y = up_before + out.displacement[1];
            let mut stopped = false;
            let mut on: Option<usize> = None;
            if rise3d && !rising {
                // Stopped short of a fall (less than half of it happened): standing on something.
                let got = -rd::<f32>(actor + 0x14);
                // Only with KH1's collision saying he is on ground (`+0x2D0` bit 0 clear, as its
                // physics step reads it): a wall beside him also stops the air move, and he hung on
                // the side of crates and tents (the user's video).
                let ground = rd::<u32>(actor + 0x2D0) & 1 == 0;
                stopped = ground && out.displacement[1] < -0.5 && (up_before - got) < -out.displacement[1] * 0.5;
                if stopped {
                    y = got;
                }
            }
            if !rising {
                // A box's top crossed by the fall (the user, twice: he couldn't land on boxes; KH1's
                // fall finds them with its object-top finder, not the mover in our states).
                let fell = (up_before - y).max(0.0);
                if let Some((top, obj)) = object_top(base, actor, up_before, fell + 0.5) {
                    if top > floor_up + 0.5 && top >= y - 0.5 {
                        y = top;
                        stopped = true;
                        on = Some(obj);
                    }
                }
            }
            if rising {
                RISE_FRAMES.fetch_add(1, Ordering::Relaxed);
            } else {
                RISE_FRAMES.store(0, Ordering::Relaxed);
                SLOPE_RUN.store(false, Ordering::Relaxed);
            }
            if rise3d && rising {
                // KH2's ceiling rule: less than 49% of the rise happened.
                // (KH1's mover may cut the rise short, never add to it: pushed out of an enemy's
                // body it lifted him, and swing after swing he rode up the target he was locked on
                // until he was above it and his swings missed, the user.)
                let got = (-rd::<f32>(actor + 0x14)).min(up_before + out.displacement[1]);
                if got < -rd::<f32>(actor + 0x14) - 0.5 {
                    static RODE: AtomicU32 = AtomicU32::new(0);
                    if RODE.fetch_add(1, Ordering::Relaxed) < 20 {
                        log(&format!("rise: KH1's mover lifted him {:.1} past KH2's rise {:.1} (an actor's body): kept to KH2's", -rd::<f32>(actor + 0x14) - got, out.displacement[1]));
                    }
                }
                y = got;
                // KH1's collision bit 3 (`+0x2D0` 9 = in the air + on a slope's surface): a steep roof
                // he is on or sliding off, not a ceiling. KH1's air mover pushes him along it rather
                // than up, and every jump was read as a head bump and cut short until a roll (the
                // user's rooftops; the logs: "rose 0.0 of 7.5 at height 620.1, flags 9", jump after
                // jump, where real ceilings show flags 1 and a partial rise). KH2's own rise there.
                // Only a contact there from the start of the rise and unbroken since: a ceiling met
                // later in a jump can carry bit 3 too (the user: in Olympus's lobby he jumped through
                // the roof; the log: "rise on a slope's contact (flags 9)" there).
                let slope_now = rd::<u32>(actor + 0x2D0) & 8 != 0;
                let n = RISE_FRAMES.load(Ordering::Relaxed);
                let run = slope_now && (n <= 3 || SLOPE_RUN.load(Ordering::Relaxed));
                SLOPE_RUN.store(run, Ordering::Relaxed);
                let slope = run;
                if slope && out.displacement[1] > 0.5 && got - up_before < out.displacement[1] * 0.49 {
                    y = up_before + out.displacement[1];
                    static SLOPES: AtomicU32 = AtomicU32::new(0);
                    if SLOPES.fetch_add(1, Ordering::Relaxed) < 20 {
                        log(&format!("rise on a slope's contact (flags {:08x}): KH2's own rise {:.1}, not a ceiling", rd::<u32>(actor + 0x2D0), out.displacement[1]));
                    }
                } else if out.displacement[1] > 0.5 && got - up_before < out.displacement[1] * 0.49 {
                    d.character.hit_ceiling();
                    static BONKS: AtomicU32 = AtomicU32::new(0);
                    if BONKS.fetch_add(1, Ordering::Relaxed) < 40 {
                        log(&format!("ceiling: rose {:.1} of {:.1} at height {:.1} (collision flags {:08x})", got - up_before, out.displacement[1], up_before - floor_up, rd::<u32>(actor + 0x2D0)));
                    }
                }
            }
            if stopped && !rising && y > floor_up {
                wr(actor + 0x14, -y);
                d.grounded = true;
                d.on_top = Some(y);
                if let Some(obj) = on {
                    let stand: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + STAND_ON_RVA);
                    stand(actor, obj);
                }
                static ON_TOP: AtomicU32 = AtomicU32::new(0);
                if ON_TOP.fetch_add(1, Ordering::Relaxed) < 40 {
                    log(&format!("landed on something {:.1} above the floor (object {:?}, standing on {:#x})", y - floor_up, on, pose::ptr(rd::<u32>(actor + 0x398))));
                }
            } else if y <= floor_up && !rising {
                wr(actor + 0x14, -floor_up);
                d.grounded = true;
                d.on_top = None;
            } else {
                wr(actor + 0x14, -y.max(floor_up));
                d.grounded = false;
                d.on_top = None;
            }
        } else {
            // On the ground: where the mover walked him, stepping down onto a floor up to 24 below
            // (KH2's floor ray), else walking off the edge into a fall. On top of something (a
            // box): as long as the ground mover keeps him at its height (it stands him on objects;
            // the map floor doesn't know them).
            let drop = walked_up - floor_up;
            // An object's top under him (KH1's finder, from where he is down a step).
            let start = walked_up.max(up_before) + 1.0;
            let top = object_top(base, actor, start, STEP_DOWN + 1.0).filter(|(t, _)| *t > floor_up + 1.0 && start - *t <= STEP_DOWN + 1.0);
            // Stood on something the map floor doesn't know that isn't an object KH1's finder sees
            // (the fall's landing above): kept there while KH1's collision has him on ground at
            // that height.
            let kept = top.is_none()
                && d.on_top.is_some_and(|t| walked_up > floor_up + 1.0 && (walked_up - t).abs() <= STEP_DOWN)
                && rd::<u32>(actor + 0x2D0) & 1 == 0;
            if kept {
                d.on_top = Some(walked_up);
                d.grounded = true;
            } else if let Some((t, obj)) = top {
                if d.on_top.is_none() {
                    static STOOD: AtomicU32 = AtomicU32::new(0);
                    if STOOD.fetch_add(1, Ordering::Relaxed) < 40 {
                        log(&format!("standing on object {obj:#x}, {:.1} above the floor", t - floor_up));
                    }
                }
                d.on_top = Some(t);
                wr(actor + 0x14, -t);
                let stand: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + STAND_ON_RVA);
                stand(actor, obj);
                d.grounded = true;
            } else if drop <= STEP_DOWN {
                d.on_top = None;
                wr(actor + 0x14, -floor_up);
                d.grounded = true;
            } else {
                d.on_top = None;
                d.grounded = false;
            }
        }
        // Height above the floor for KH2's next step.
        wr(actor + 0x3D4, (-rd::<f32>(actor + 0x14)) - floor_up);
        // Facing.
        let f = out.facing;
        wr(actor + 0x30, [f.sin(), 0.0, f.cos(), f]);
        wr(actor + 0x150, f);
        // KH2's motion into KH1's own physics fields (its air speed and arc), each step: KH1's
        // states go on from it when they take him (`shared::physics::mirror`, `docs/kh1-physics.md`).
        super::shared::physics::mirror(actor, &d.character, &out, d.grounded && !d.character.airborne());
        // KH1's own air moves (its jump's input step `0x1402ADA90` tries them in this order):
        // vines and climbable walls (state 10), ledges (13), bars (`0x1402C7E00`), objects to
        // hang from, and water to swim in (17). Tried while KH2 has him falling, the stick held.
        // As KH1's jump step tries them (`0x1402ADA90`): every frame of the jump, rising too (KH1
        // skips them only in the 15 frames after jumping off something, flag 0x10000, a jump KH1
        // has itself; the user: jumping into a ladder didn't catch it until the top of the jump),
        // the stick held or not (KH1 passes its input either way; the user: ladders and poles were
        // hit or miss from a jump).
        let no_grab = NO_GRAB.load(Ordering::Relaxed);
        if no_grab > 0 {
            NO_GRAB.store(no_grab - 1, Ordering::Relaxed);
        }
        // (Also out of an Aerial Dodge or a glide: KH2's fall, the user: "KH2 soras falling animation
        // is unable to grab onto ladders/vines".)
        if settings.grabs && no_grab == 0 && d.character.airborne() && matches!(out.state, StateName::Air | StateName::Jump | StateName::AerialDodge | StateName::Glide) {
            // KH1 tries them in its jump / fall states, whose collision (`0x140297F20`: flag 1)
            // finds the contacts they read; ours ran in its ground state. So: KH1's fall state (3,
            // which has these checks too) and a standing-still move there to refresh the contacts.
            // The move presses a little into where the stick points: KH1's climb check reads the wall
            // face the move touched (`+0x2DE`), which a standing-still move cleared (the user: he
            // slid down a ladder he jumped into, caught only at the bottom). Put back if nothing
            // takes him.
            // With the stick neutral the frame's own air move (KH1's fall state) left the contacts.
            let state = rd::<i32>(actor + 0x70);
            let at = rd::<[f32; 4]>(actor + 0x10);
            wr(actor + 0x70, AIR_STATE);
            // With the stick let go, the press goes along his drift (or his facing): KH1's climb
            // check needs a wall face the move touched (a ladder or vine he falls past otherwise
            // slipped by).
            let drift = glam::Vec2::new(out.displacement[0], out.displacement[2]);
            let toward = if tilt > 0.0 { dir } else if drift.length() > 0.5 { drift.normalize() } else { glam::Vec2::new(out.facing.sin(), out.facing.cos()) };
            mv(actor, &[toward.x * 4.0, 0.0, toward.y * 4.0, 0.0], 0);
            let (gx, gz) = if tilt > 0.0 { (dir.x, dir.y) } else { (0.0, 0.0) };
            let took = grab(base, actor, [gx, 0.0, gz, tilt]);
            if took.is_none() {
                wr(actor + 0x10, at);
                wr(actor + 0x70, state);
            }
            if let Some(what) = took {
                ATTACK_AFTER.store(false, Ordering::Relaxed);
                HANDOFF_FRAMES.store(0, Ordering::Relaxed);
                HANDOFF.store(true, Ordering::Relaxed);
                set_air_cast(None);
                log(&format!("KH1 takes him: {what} (KH1 state {}, slot {:?})", rd::<i32>(actor + 0x70), pose::sora_slot()));
            }
        }

        let q = rd::<[f32; 3]>(actor + 0x10);
        // Battle or field stance by KH1's battle mode (bit 0 of `0x142867CD8`), as KH2 does by its
        // own: out of battle KH2's field motions carry no Keyblade, and attacks bring it out.
        let battle = rd::<u32>(base + BATTLE_RVA) & 1 != 0;
        if matches!(out.state, StateName::Attack) || !out.hitboxes.is_empty() {
            d.fought = Some(Instant::now());
        }
        let lingering = d.fought.is_some_and(|t| t.elapsed().as_secs_f32() < BATTLE_LINGER);
        // (In a Drive Form always the battle stance, Keyblades out, as KH2's forms: the user, "dont
        // let soras keyblades disappear in valor form".)
        d.anim.field = !battle && !lingering && !super::form::in_form();
        // (The game takes one update of delta 2 at its 30 fps and poses Sora once; our extra 60 Hz
        // steps keep the track and the blend clock but leave the pose to the frame's last step,
        // unless a hit or guard window is open, which reads the Keyblade's place.)
        anim_pos = [q[0], -q[1], q[2]];
        if step + 1 < steps_now && out.hitboxes.is_empty() && out.guard_boxes.is_empty() {
            d.anim.update_deferred(&out, &d.character, anim_pos);
        } else {
            d.anim.update(&out, &d.character, anim_pos);
        }
        kh2_footsteps(actor, &out, &d.anim);
        keyblade(base, actor, d.anim.weapons_out(), &mut d.logged);
        GRIP.store(d.anim.weapons_out(), Ordering::Relaxed);
        GROUNDED.store(d.grounded && !d.character.airborne(), Ordering::Relaxed);
        if d.grounded && !d.character.airborne() {
            FLEW.store(false, Ordering::Relaxed);
        }
        let battle_rows = super::form::battle(&d.drive, kh2).clone();
        for g in &out.guard_boxes {
            if !frame_guards.contains(g) {
                frame_guards.push(*g);
            }
        }
        if frame_guards.len() > out.guard_boxes.len() {
            out.guard_boxes = frame_guards.clone();
        }
        invulnerable |= out.invulnerable;
        guarding |= out.guarding;
        d.strikes.strike(actor, &out, &d.anim, &mut d.character, &battle_rows);
        // Reflect's script: its cast, barrier, counter and effects.
        super::reflect::after_step(actor, &out, d.character.airborne(), &mut d.strikes, &battle_rows, lock);
        // KH2's own sounds for the effect elements this frame starts. In Wisdom Form a frame
        // trigger 1 is a shot from its model's script instead (`form`).
        for e in &out.events {
            if super::form::takes(&d.drive, e) {
                if let character_core::EventKind::Effect(n) = e {
                    super::form::effect(out.motion, *n);
                }
                continue;
            }
            super::kh2sound::event(out.motion, *e);
            super::kh2fx::event(out.motion, *e);
        }
        super::form::shots_step(actor, &out, d.character.airborne(), &mut d.strikes, &battle_rows, lock);
        super::form::after_step(&mut d.drive, actor, rd::<u32>(base + BATTLE_RVA) & 1 != 0);
        super::kh2sound::tick();
        {
            // A KH2 swing starting: a new attack motion, or the same one again from its start.
            let attacking = matches!(out.state, StateName::Attack);
            let recoil = matches!(out.motion, character_core::reaction::motion::REPELLED | character_core::reaction::motion::REPELLED_AIR);
            if attacking && !recoil && (d.swing != Some(out.motion) || out.clip_frame < d.swing_frame) {
                // KH1's attack step counts each attack in the save's tallies (`0x14028A4D0(0x13)`,
                // `0x1402A4760`); event scripts read them (`0x14028A4B0`): the Dive to the Heart's
                // "attack" tutorial waits for it (the user's softlock).
                let count: unsafe extern "system" fn(i32) = std::mem::transmute(base + COUNT_RVA);
                count(0x13);
            }
        }
        if settings.swing_fx {
            // A KH2 swing starting: a new attack motion, or the same one again from its start.
            let attacking = matches!(out.state, StateName::Attack);
            // (A repel's recoil is not a swing.)
            let recoil = matches!(out.motion, character_core::reaction::motion::REPELLED | character_core::reaction::motion::REPELLED_AIR);
            let started = attacking && !recoil && (d.swing != Some(out.motion) || out.clip_frame < d.swing_frame);
            if started {
                // (KH2's own voice for the moves whose KH2 sounds play, not KH1's.)
                d.strikes.swing_start(actor, d.character.airborne(), out.combo_count, d.character.in_finisher(), !super::kh2sound::covers(out.motion));
            }
            // (And the guard's swish: its motion's trail window, KH2's Keyblade trail there.)
            let guard_swish = matches!(out.state, StateName::Guard) && out.weapon_trail;
            if guard_swish && !d.strikes.has_trail() {
                d.strikes.trail_for_guard(actor);
            }
            d.strikes.trail(actor, attacking && !out.hitboxes.is_empty() || guard_swish);
        }
        let attacking = matches!(out.state, StateName::Attack);
        d.swing = attacking.then_some(out.motion);
        d.swing_frame = out.clip_frame;
        KH2_MOTION.store(out.motion as u32, Ordering::Relaxed);
        // KH1's footsteps only once KH2 has been walking or running a moment, in its own walk / run
        // motion (the user: steps heard walking forward mid-combo; between two swings KH2 can be in
        // its run for a few frames, still showing the swing, and KH1's run cycle stepped then).
        static STEPPING: AtomicU32 = AtomicU32::new(0);
        let walking = matches!(out.state, StateName::Walk | StateName::Run) && matches!(out.motion, character_core::motion::WALK | character_core::motion::RUN);
        let n = if walking { STEPPING.load(Ordering::Relaxed).saturating_add(1) } else { 0 };
        STEPPING.store(n, Ordering::Relaxed);
        KH2_STEPS.store(n >= STEPS_AFTER, Ordering::Relaxed);
        if d.last_state != Some(out.state) {
            // KH1's own grunt as KH2's dodge roll starts (a random one of his first swing's, the user:
            // "Sora is missing a kh1 quote for when he activates dodge roll").
            if out.state == StateName::DodgeRoll {
                super::kh1voice::dodge_roll(actor);
            }
            if d.logged < 400 {
                d.logged += 1;
                log(&format!("KH2 has Sora: {:?} (motion {}), height above floor {above:.1}, KH1 slot {:?}, KH1 state {}, flags {:08x}; {}", out.state, out.motion, slot, rd::<i32>(actor + 0x70), rd::<u32>(actor), pose::weapon_report()));
            }
            d.last_state = Some(out.state);
        }
        // Base Sora (`shared`): KH2's move over, KH1's own states again (after this step).
        if super::shared::on() {
            let waiting = combat::command_waiting() || super::reflect::waiting() || super::formmagic::waiting() || super::form::busy() || FLY_PRESS_GROUND.load(Ordering::Relaxed);
            let falling = out.displacement[1] <= 0.0;
            back = super::shared::give_back(out.state, d.grounded && !d.character.airborne(), falling, d.character.combo_going(), waiting);
        }
        // (KH1 took him on this step: one of its air moves, a command of KH1's.)
        let over = back.is_some() || HANDOFF.load(Ordering::Relaxed);
        last = Some(out);
        if over {
            break;
        }
    }
    let Some(mut out) = last else { return false };
    // The pose of a last step that was left (the loop ended early), and KH2's effects by the
    // steps run.
    d.anim.settle(&d.character, out.facing, anim_pos);
    super::kh2fx::tick(out.motion, ran as f32);
    out.invulnerable |= invulnerable;
    out.guarding |= guarding;
    d.last_out = Some(out.clone());
    if let Ok(mut pz) = POSE.try_lock() {
        *pz = Some((d.anim.bones.clone(), d.anim.weapons.first().cloned().unwrap_or_default(), Instant::now()));
    }
    // Valor's left Keyblade (the form rig's second weapon), and the clip playing for its trail,
    // for `leftblade`.
    super::leftblade::kh2_left(d.anim.weapons.get(1).map(|w| w.as_slice()), &d.anim.bones, out.motion, out.clip_frame, out.weapon_trail);
    // The Keyblade during attacks (the user: invisible while attacking): every 15 frames.
    static ATTACK_FRAMES: AtomicU32 = AtomicU32::new(0);
    if matches!(out.state, StateName::Attack) {
        let n = ATTACK_FRAMES.fetch_add(1, Ordering::Relaxed);
        if n % 15 == 7 && n < 15 * 40 {
            log(&format!("attacking (motion {}, frame {}): {}", out.motion, out.clip_frame, pose::weapon_report()));
        }
    }
    ACTIVE_MS.store(now_ms().max(1), Ordering::Relaxed);
    if let Some(back) = back {
        shared_give_back(base, actor, d, back, &out);
        // (This frame was KH2's step; KH1's states from the next.)
    }
    true
}

/// KH1's footstep sound (`0x140293C70 (actor, kind)`: the floor's sound from `footstep.bin`
/// by world and material), played from his motion's frames. Under KH2, KH1 keeps him in its
/// walk / run slots whenever the stick is held, so its steps sounded during KH2's attacks and
/// rolls (the user: footsteps mid-animation): Sora's are KH1's only while KH2 walks or runs
/// him (or KH1 has him).
const FOOTSTEP_RVA: Rva = Rva(0x293C70);
const FOOTSTEP_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x56, 0x48, 0x83, 0xEC, 0x20];
static ORIG_FOOTSTEP: AtomicUsize = AtomicUsize::new(0);
/// KH2 has Sora walking or running this frame (for `STEPS_AFTER` frames or more).
static KH2_STEPS: AtomicBool = AtomicBool::new(false);
/// Frames of KH2's walk / run before KH1's footsteps sound.
const STEPS_AFTER: u32 = 10;

unsafe extern "system" fn footstep(actor: usize, kind: i32) -> usize {
    let base = BASE.load(Ordering::Relaxed);
    if actor > 0x10000 && actor == rd::<u64>(base + PARTY_RVA) as usize && FRAME_KIND.load(Ordering::Relaxed) == 1 && !KH2_STEPS.load(Ordering::Relaxed) {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 10 {
            log("KH1's footstep held back (KH2 isn't walking him)");
        }
        return 0;
    }
    let f: unsafe extern "system" fn(usize, i32) -> usize = std::mem::transmute(ORIG_FOOTSTEP.load(Ordering::Acquire));
    f(actor, kind)
}

/// KH1's footstep / material sound from a motion's sound event (`0x1402939B0 (actor, kind)`:
/// kinds 0..7 the feet (0 run, 4 walk; it alternates the foot itself), 10 a landing, others
/// walls and objects), reached from KH1's motion events (`0x140293290`, from `0x14029DA4E`).
/// KH1 keeps Sora in its own walk / run cycle under KH2 whenever the stick is held, so its steps
/// sounded whatever KH2 played (the user: "footsteps currently follow whatever the kh1
/// animations are doing ... we need footstep sfxs to follow what kh2 animations are doing").
/// While KH2 has Sora, KH1's are dropped, and KH2's own footstep triggers (frame trigger 2, the
/// same kinds) play KH1's floor sound through it instead (`kh2_footsteps`).
const STEP_SOUND_RVA: Rva = Rva(0x2939B0);
/// `mov [rsp+0x20], rsi; push rdi; sub rsp, 0x20`.
const STEP_SOUND_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x74, 0x24, 0x20, 0x57, 0x48, 0x83, 0xEC, 0x20];
static ORIG_STEP_SOUND: AtomicUsize = AtomicUsize::new(0);
/// Set while we call it ourselves (a KH2 footstep).
static OUR_STEP: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn step_sound(actor: usize, kind: i32) {
    let base = BASE.load(Ordering::Relaxed);
    if !OUR_STEP.load(Ordering::Relaxed) && actor > 0x10000 && actor == rd::<u64>(base + PARTY_RVA) as usize && FRAME_KIND.load(Ordering::Relaxed) == 1 {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 10 {
            log(&format!("KH1's step sound (kind {kind}) dropped: KH2 has Sora, KH2's own steps sound"));
        }
        return;
    }
    let f: unsafe extern "system" fn(usize, i32) = std::mem::transmute(ORIG_STEP_SOUND.load(Ordering::Acquire));
    f(actor, kind)
}

unsafe fn play_step(actor: usize, kind: i16) {
    let orig = ORIG_STEP_SOUND.load(Ordering::Acquire);
    if orig == 0 {
        return;
    }
    let f: unsafe extern "system" fn(usize, i32) = std::mem::transmute(orig);
    OUR_STEP.store(true, Ordering::Relaxed);
    f(actor, kind as i32);
    OUR_STEP.store(false, Ordering::Relaxed);
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 12 {
        log(&format!("KH2 footstep (kind {kind})"));
    }
}

/// KH2's footsteps this frame: its walk / run loops' footstep triggers as the rig's track
/// passes them (the controller plays those loops without frames of its own), and the footstep
/// events of its actions (the landing's). In Wisdom Form only the landing's (the user: "we
/// generally want footsteps disabled for sora in this form (with the exception of landing) as
/// he is gliding across the ground"; the form's own walk and run have none anyway).
unsafe fn kh2_footsteps(actor: usize, out: &Output, anim: &RigAnim) {
    static PREV: Mutex<Option<(u16, bool, f32)>> = Mutex::new(None);
    let in_form = super::form::in_wisdom();
    let looping = matches!(out.state, StateName::Walk | StateName::Run | StateName::Idle);
    let now = anim.playing();
    let mut steps: Vec<i16> = Vec::new();
    if let Ok(mut prev) = PREV.lock() {
        if let (true, Some((m, field, f, len)), Some((pm, pfield, pf))) = (looping, now, *prev) {
            if m == pm && field == pfield && len > 0.0 && !in_form {
                if let Some(list) = anim.rig.footsteps.get(&(m, field)) {
                    let (a, b) = (pf.rem_euclid(len), f.rem_euclid(len));
                    for &(at, kind) in list {
                        let crossed = if b >= a { at > a && at <= b } else { at > a || at <= b };
                        if crossed && (f - pf).abs() < len {
                            steps.push(kind);
                        }
                    }
                }
            }
        }
        *prev = now.map(|(m, field, f, _)| (m, field, f));
    }
    if !looping {
        for e in &out.events {
            if let EventKind::Footstep(kind) = e {
                if !in_form || out.motion == character_core::motion::LAND {
                    steps.push(*kind);
                }
            }
        }
    }
    for k in steps {
        play_step(actor, k);
    }
}

/// KH1's air moves, tried as its jump's input step does; the one that took him, if any.
unsafe fn grab(base: usize, actor: usize, stick: [f32; 4]) -> Option<&'static str> {
    let flags = rd::<u32>(actor);
    let fine: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + CAN_ACT_RVA);
    if flags & 0x2004 != 0 || fine(actor) == 0 {
        return None;
    }
    // (Some write into the vector they are given: room to spare.)
    let mut v = [0.0f32; 16];
    v[..4].copy_from_slice(&stick);
    let p = v.as_mut_ptr() as usize;
    type Try = unsafe extern "system" fn(usize, usize) -> i32;
    let tries: [(Rva, &str); 5] = [(VINE_RVA, "vine / climb"), (LEDGE_RVA, "ledge"), (BAR_RVA, "bar"), (HANG_RVA, "object to hang from"), (SWIM_RVA, "water")];
    for (rva, what) in tries {
        if rva == BAR_RVA && rd::<u32>(base + NO_BARS_RVA) != 0 {
            continue;
        }
        let f: Try = std::mem::transmute(base + rva);
        if f(actor, p) != 0 {
            return Some(what);
        }
    }
    None
}

/// The top of a standable object under him (KH2 height, up), from `from` (up) down to
/// `down` below it: KH1's own object-top finder, run in its fall state from that height.
unsafe fn object_top(base: usize, actor: usize, from: f32, down: f32) -> Option<(f32, usize)> {
    let find: unsafe extern "system" fn(usize, *mut [f32; 4]) -> usize = std::mem::transmute(base + OBJECT_TOP_RVA);
    let (state, y) = (rd::<i32>(actor + 0x70), rd::<f32>(actor + 0x14));
    wr(actor + 0x70, FALL_STATE);
    wr(actor + 0x14, -from);
    let mut mv = [0.0f32, down.max(0.5), 0.0, 0.0];
    let obj = find(actor, &mut mv);
    wr(actor + 0x14, y);
    wr(actor + 0x70, state);
    let top = from - mv[1];
    (obj > 0x10000 && top.is_finite() && top <= from + 0.5).then_some((top, obj))
}

/// A platform's carry and the frame's push (KH1 world), see the physics step.
unsafe fn carried(actor: usize, logged: &mut u32) -> [f32; 3] {
    let mut c = glam::Vec3::from(rd::<[f32; 3]>(actor + 0x120));
    let riding = rd::<u32>(actor + 0x160) & 4 != 0;
    if riding {
        c += glam::Vec3::from(rd::<[f32; 3]>(actor + 0x200));
    }
    if !c.is_finite() || c.length() > 400.0 {
        return [0.0; 3];
    }
    static SEEN: AtomicU32 = AtomicU32::new(0);
    if c.length_squared() > 1e-4 && *logged < 400 && SEEN.fetch_add(1, Ordering::Relaxed) < 40 {
        *logged += 1;
        log(&format!(
            "carried {c:.2?} (push {:.2?}, platform {:?}, standing on {:#x}, object {})",
            rd::<[f32; 3]>(actor + 0x120),
            riding.then(|| rd::<[f32; 3]>(actor + 0x200)),
            pose::ptr(rd::<u32>(actor + 0x398)),
            rd::<i32>(actor + 0x2CC)
        ));
    }
    c.to_array()
}

/// KH1 movement states that are KH1's alone: 30 and up (the Jungle Slider is 30 and 31: its
/// slide, jumps and swings), 29 (the magic carpet out of the Cave of Wonders: Agrabah room
/// 0x14 puts Sora in it, `0x1402957F0`; the user: the carpet didn't move, a softlock), 24
/// (swimming: Atlantica, and End of the World room 0x16's water, `0x140288920`), 25 (Mermaid
/// Kick's dash, from 24 with KH1 ability 2: `0x1402C9980`) and 8 (flying in Neverland: see
/// `FLIGHT_FLAG`; with 24 and 25 the states where jump means up, `0x140291660`'s 0x203000100
/// mask). KH2 has none of these.
fn kh1_mode(state: i32) -> bool {
    // (Not our Quick Run's state, `quickrun::STATE`: a state of ours in KH1's table, base Sora's.)
    (state >= 29 && state != super::quickrun::STATE) || matches!(state, 24 | 25 | FLIGHT_STATE)
}

/// The same, for KH2's camera (KH1's own camera for the carpet, the slider and water). Not
/// flight: KH1 flies in its ordinary follow camera (mode 1, ours with KH2's camera chosen, KH1's
/// yaw kept in step so its flight steering matches), the user: the camera snapped to KH1's.
pub fn kh1_camera_mode(state: i32) -> bool {
    matches!(state, 24 | 25 | 29 | 30 | 31)
}
/// Sora is in one of them (KH2's camera gives way too).
static KH1_MODE: AtomicBool = AtomicBool::new(false);

/// A cutscene or scripted moment: an event runs (talks with text boxes too), an area script
/// walks Sora, or KH1 holds him still (for KH2's camera to give way to KH1's).
pub fn kh1_scene() -> bool {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 {
        return false;
    }
    let event = unsafe { rd::<u32>(base + EVENT_RVA) } != 0;
    let sora = unsafe { rd::<u64>(base + PARTY_RVA) } as usize;
    // (The Jungle Slider's states only: a summon's own states (38 Simba's charge, 40 the call)
    // keep KH2's camera, the user.)
    let own_mode = sora > 0x10000 && kh1_camera_mode(unsafe { rd::<i32>(sora + 0x70) });
    // Held still counts out of battle only: in battle it is a summon's or a command's moment
    // (the user: KH2's camera should stay through Simba's charge).
    let battle = unsafe { rd::<u32>(base + BATTLE_RVA) } & 1 != 0;
    // (Nor the hold that ends one of KH1's own moves, a spell's landing: the user, KH2's
    // camera zoomed in to KH1's after a jump cast of Fire out of battle.)
    let after_move = HANDOFF.load(Ordering::Relaxed) && !super::shared::active() || now_ms().saturating_sub(HANDOFF_SEEN_MS.load(Ordering::Relaxed)) < 1500;
    let held = FROZEN_FRAMES.load(Ordering::Relaxed) >= 2 && !battle && !after_move;
    event || own_mode || ((active() || super::shared::active()) && (SCRIPTED.load(Ordering::Relaxed) || held))
}

/// KH1 moves Sora itself in a scene: one of its own movement modes (the Jungle Slider) or an
/// area script walking him. KH1's camera stays through these even after a camera reset that
/// KH2's camera takes (`camera.rs`).
pub fn kh1_moves_sora() -> bool {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 {
        return false;
    }
    let sora = unsafe { rd::<u64>(base + PARTY_RVA) } as usize;
    let own_mode = sora > 0x10000 && kh1_camera_mode(unsafe { rd::<i32>(sora + 0x70) });
    own_mode || (active() && SCRIPTED.load(Ordering::Relaxed))
}

/// Whether KH1 may see Square: not while it has Sora back on the ground after one of its own
/// air moves (KH2 is about to take him; a roll pressed then is KH2's).
pub fn kh1_keeps_square() -> bool {
    if !HANDOFF.load(Ordering::Relaxed) || super::shared::active() {
        return true;
    }
    let base = BASE.load(Ordering::Relaxed);
    let sora = if base == 0 { 0 } else { (unsafe { rd::<u64>(base + PARTY_RVA) }) as usize };
    !(sora > 0x10000 && matches!(unsafe { rd::<i32>(sora + 0x70) }, 0 | 1))
}

/// KH1 has Sora for one of its moves (`hand_to_kh1`, a grab), until KH2 takes him back.
pub fn handing_off() -> bool {
    HANDOFF.load(Ordering::Relaxed)
}

/// KH2 has Sora this moment (not handed to KH1).
/// KH1 has him for one of its own moves (`hand_to_kh1`), until he can be KH2's again.
pub fn kh1_move() -> bool {
    ON.load(Ordering::Relaxed) && HANDOFF.load(Ordering::Relaxed) && FRAME_KIND.load(Ordering::Relaxed) == 2
}

/// KH2 would have Sora but KH1 is playing one of its own motions on him (a reaction, or an
/// action KH1 started itself): his own body, no event or area script, no ride, not one of KH1's
/// own movement modes. An Attack then is KH2's once KH1 lets go, never KH1's own attack.
pub unsafe fn kh1_borrows(actor: usize) -> bool {
    let base = BASE.load(Ordering::Relaxed);
    ON.load(Ordering::Relaxed)
        && base != 0
        && actor == rd::<u64>(base + PARTY_RVA) as usize
        && rd::<u32>(base + EVENT_RVA) == 0
        && now_ms() >= RIDE_UNTIL.load(Ordering::Relaxed)
        && pose::sora_body_seen(500)
        && !SCRIPTED.load(Ordering::Relaxed)
        && !kh1_mode(rd::<i32>(actor + 0x70))
        && rd::<i32>(actor + 0x70) != FLIGHT_STATE
}

/// The button KH1 jumps with: Circle, or Cross when the options swap the confirm button to
/// Circle (KH1's pad step `0x140288F30` maps Cross to confirm (`0x210001`) and Circle to
/// jump / cancel (`6`), the other way round when `0x1402F4690` says so; the user: with the swap,
/// KH1 saw the jump button and took Sora from KH2).
pub fn jump_button() -> u16 {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 {
        return CIRCLE;
    }
    let swapped: unsafe extern "system" fn() -> u8 = unsafe { std::mem::transmute(base + CONFIRM_SWAP_RVA) };
    if unsafe { swapped() } & 1 != 0 {
        CROSS
    } else {
        CIRCLE
    }
}
const CONFIRM_SWAP_RVA: Rva = Rva(0x2F4690);
/// KH1's button state after its pad step (`0x140288F30`; `0x1402892A0` fills it).
const BUTTONS_RVA: Rva = Rva(0x298D4A0);
/// KH1's processed left stick (-1..1, its dead zone applied; the keyboard's keys too).
const LEFT_STICK_RVA: Rva = Rva(0x298D4E0);

/// The buttons as KH2 reads them: the pads' own records (`RAW_BUTTONS`, before KH1's copy is
/// masked) and KH1's merged state, which the keyboard feeds too (the user's tester played on
/// keyboard: the pad records stayed empty, so KH2 had no jump, roll or guard at all). KH1's jump
/// / cancel (`6`) is read as the jump button, its Square action (`0x100000`) as Square.
pub fn buttons() -> u16 {
    let mut b = RAW_BUTTONS.load(Ordering::Relaxed) as u16;
    let base = BASE.load(Ordering::Relaxed);
    if base != 0 {
        let held = unsafe { rd::<u32>(base + BUTTONS_RVA + 4) };
        if held & 6 != 0 {
            b |= jump_button();
        }
        // (KH1's action bit, `0x100000`, is Square's, but KH1 also sets it with its jump held in
        // the air: its glide. Read as Square then it cut KH2's jump short and started KH1's glide
        // on Square, the user: "multiple inputs are being sent ... unable to get the max height on
        // jump, and if glide is equipped you go straight into glide". With the jump held, only
        // the pad's own Square counts.)
        // (Off for now, s251: s247's filter.)
        // if held & 0x10_0000 != 0 && held & 6 == 0 && b & jump_button() == 0 {
        //     b |= SQUARE;
        // } else if held & 0x10_0000 != 0 && b & SQUARE == 0 {
        //     static N: AtomicU32 = AtomicU32::new(0);
        //     if N.fetch_add(1, Ordering::Relaxed) < 10 {
        //         log(&format!("KH1's action bit with the jump held (KH1 buttons {held:08x}): not taken for Square"));
        //     }
        // }
        if held & 0x10_0000 != 0 {
            b |= SQUARE;
        }
    }
    b
}

/// How far the left stick is pushed (0..1): the pads' own sticks as KH2's pad builds its tilt
/// (`character_core::pad`, Steam KH2 `0x14039c180`: 60 of 127.5 off each axis, the length
/// clamped to 1; it was a round 20% dead zone), or KH1's processed one when they rest (the
/// keyboard's movement keys; KH1 builds it the same way, `0x14028b4f0`).
pub fn move_tilt() -> f32 {
    // KH2's first person (`camera::first_person`): KH2's player doesn't move in it (Steam KH2
    // `0x1403a8f00` / `0x1403a7830`, his "can move" tests, are false while the camera is in it).
    if super::camera::first_person() {
        return 0.0;
    }
    let st = RAW_STICKS.load(Ordering::Relaxed).to_le_bytes();
    let raw = character_core::pad::stick(st[2], st[3]).1;
    if raw > 0.0 {
        return raw;
    }
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 {
        return 0.0;
    }
    let p = unsafe { rd::<[f32; 2]>(base + LEFT_STICK_RVA) };
    let m = p[0].hypot(p[1]);
    if m.is_finite() { m.min(1.0) } else { 0.0 }
}

/// KH2's first-person turn this frame, per 60 Hz frame (`camera::first_person`): Steam KH2
/// `0x1403a9250` turns a standing player by ±3° a frame (times the frame step) on two held pad
/// bits, whatever the tilt. **Assumption:** they are the right stick's left / right (KH2's
/// first person pitches by the right stick's up / down, `0x140165e30`, and the player can't
/// move in it). Right turns him (and the view) right, as the right stick turns KH2's camera
/// (`camera::look`). In a frame the mouse moved (Steam), the mouse's x instead: KH2's pad puts
/// the mouse over the right stick (`camera::first_person_mouse_x`), so the mouse turns him at
/// the same steady 3° (`0x1403a9250` has no mouse speed-up). For a Drive Form (KH2's controller
/// has him); None otherwise, out of first person, or with the stick within its dead zone.
pub fn first_person_turn() -> Option<f32> {
    // (Base Sora is KH1's own states': KH1's own first-person turn turns him, fed this, `probe`
    // and `first_person_mouse_turn`.)
    if !super::camera::first_person() || !super::form::in_form() {
        return None;
    }
    let x = match super::camera::first_person_mouse_x() {
        Some(x) => x,
        None => character_core::pad::axis(RAW_STICKS.load(Ordering::Relaxed).to_le_bytes()[0]),
    };
    (x != 0.0).then(|| x.signum() * 0.052_359_9)
}

/// Base Sora in KH2's first person, in a frame the mouse moved (Steam): KH1's own first-person
/// turn (`0x14028c7e0` in camera mode 2: 3° a frame times its processed left stick's x,
/// `0x14298cad0`, read inside this control step `0x1402a6220`) is given a full left stick toward
/// the mouse's x (none when the mouse moved only up / down), as `probe::pad` gives it the pads'
/// right stick in other frames (that ran before the mouse was read this frame, `0x1400da9f0`).
/// Returns the left stick to put back after KH1's step.
unsafe fn first_person_mouse_turn(base: usize, actor: usize) -> Option<[f32; 2]> {
    if !super::camera::first_person() || actor < 0x10000 || actor != rd::<u64>(base + PARTY_RVA) as usize || super::form::in_form() {
        return None;
    }
    let x = super::camera::first_person_mouse_x()?;
    let saved = rd::<[f32; 2]>(base + LEFT_STICK_RVA);
    let lx = if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 };
    wr(base + LEFT_STICK_RVA, [lx, 0.0f32]);
    Some(saved)
}

/// Attack pressed while KH1 had him for its own move (`combat::exec`): ends that move as soon
/// as he stands free (KH1 allowed the cancel) and is KH2's first press.
pub static ATTACK_AFTER: AtomicBool = AtomicBool::new(false);

fn note_dropped_attack(state: i32) {
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 30 {
        log(&format!("Attack during KH1's move (state {state}) dropped: it couldn't cut the move on that frame (no buffer)"));
    }
}

/// Sora's physics frames so far (one per game frame; `TICKS` counts them in 60 Hz frames).
pub static GAME_FRAME: AtomicU64 = AtomicU64::new(0);

/// The game's image base (0 before `install`).
pub fn game_base() -> usize {
    BASE.load(Ordering::Relaxed)
}

/// Sora's actor (KH1's party slot), 0 if none.
pub fn sora_actor() -> usize {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 {
        return 0;
    }
    unsafe { rd::<u64>(base + PARTY_RVA) as usize }
}

pub fn kh2_has() -> bool {
    ON.load(Ordering::Relaxed) && FRAME_KIND.load(Ordering::Relaxed) == 1 && !HANDOFF.load(Ordering::Relaxed)
}

/// KH2 has Sora in the air this moment.
/// (Kept each frame: KH1's command step can run while the driver is locked, and a failed
/// `try_lock` read as "on the ground", the user: Sonic Blade mid-jump.)
pub fn kh2_airborne() -> bool {
    kh2_has() && KH2_AIR.load(Ordering::Relaxed)
}
static KH2_AIR: AtomicBool = AtomicBool::new(false);

/// KH2's movement over its last step, sideways (KH1's x / z, per frame): an air spell's KH1 cast
/// goes on with it (`combat::command`).
pub fn kh2_sideways() -> [f32; 2] {
    DRIVER
        .try_lock()
        .ok()
        .and_then(|d| d.as_ref().and_then(|d| d.last_out.as_ref().map(|o| [o.displacement[0], o.displacement[2]])))
        .filter(|v| v.iter().all(|x| x.is_finite()))
        .unwrap_or([0.0; 2])
}

/// KH1 takes Sora for one of its own actions (a command): it keeps him until it has him
/// standing in a movement motion again (see `HANDOFF`).
/// The hand-off now running is a command's (`combat::command`): KH2 comes back in its battle
/// stance.
pub static FROM_COMMAND: AtomicBool = AtomicBool::new(false);

/// A spell KH1 is casting in the air (its index, when it started), for `pose` to draw KH2's
/// air cast, fall and landing meanwhile.
static AIR_CAST: Mutex<Option<(u8, Instant)>> = Mutex::new(None);

pub fn set_air_cast(spell: Option<u8>) {
    if let Ok(mut c) = AIR_CAST.lock() {
        *c = spell.map(|s| (s, Instant::now()));
    }
}

/// The spell KH1 casts in the air now (while it has him for it) and seconds since it started.
/// The spell KH1 is casting now (its command's, while KH1 has him for it: Fire 0 .. Aero 6).
pub fn kh1_spell_now() -> Option<u8> {
    if !(HANDOFF.load(Ordering::Relaxed) && FROM_COMMAND.load(Ordering::Relaxed)) {
        return None;
    }
    COMMAND.try_lock().ok()?.as_ref()?.spell
}

pub fn air_cast() -> Option<(u8, f32)> {
    if !HANDOFF.load(Ordering::Relaxed) {
        return None;
    }
    AIR_CAST.try_lock().ok()?.map(|(s, t)| (s, t.elapsed().as_secs_f32()))
}

pub fn hand_to_kh1(why: &str) {
    super::shared::clear();
    super::stylecombo::clear();
    super::stylejump::clear();
    super::styleroll::clear();
    AIR_SPELL_LANDED.store(false, Ordering::Relaxed);
    SPELL_CUT_OPEN.store(false, Ordering::Relaxed);
    CUT_LEFT.store(false, Ordering::Relaxed);
    ATTACK_AFTER.store(false, Ordering::Relaxed);
    HANDOFF_FRAMES.store(0, Ordering::Relaxed);
    HANDOFF.store(true, Ordering::Relaxed);
    log(&format!("KH1 takes him: {why}"));
}

/// While KH2 has Sora: his controller and its last frame (for `hurt`).
pub fn with_kh2<R>(f: impl FnOnce(&mut Character, &Output) -> R) -> Option<R> {
    if !kh2_has() {
        return None;
    }
    let mut g = DRIVER.try_lock().ok()?;
    let d = g.as_mut()?;
    d.last_state?;
    let out = d.last_out.clone()?;
    Some(f(&mut d.character, &out))
}

/// KH1 shows Sora's Keyblade while his actor's flag `0x4000` is set ("weapon out"):
/// `0x14029DAB0`, applying a motion, sets it with `0x1402A0CA0(actor, 1)` (flag, and the
/// combat state's bytes `+0xC..+0xF` = ff ff ff 80, the Keyblade's colour and alpha) for
/// motions whose table entry is of type 1 (the attacks, the battle stance), and clears it
/// with `0x1402A10F0` for type 0 (the field idle / walk / run); the flag also makes motion
/// requests map to their battle variants (`0x14029E050`). KH1's own motions never change
/// while KH2 has Sora (its attacks are KH2's), so the flag follows KH2's pose here: out while
/// KH2 has the Keyblade out, put away when KH2 puts it away.
unsafe fn keyblade(base: usize, actor: usize, out: bool, logged: &mut u32) {
    let flags = rd::<u32>(actor);
    let cs = pose::ptr(rd::<u32>(actor + 0x14C));
    let has = flags & WEAPON_OUT != 0;
    if has == out {
        return;
    }
    if out {
        let draw: unsafe extern "system" fn(usize, i32) = std::mem::transmute(base + DRAW_WEAPON_RVA);
        draw(actor, 1);
    } else {
        super::kh2fx::weapon_vanish();
        let put_away: unsafe extern "system" fn(usize) = std::mem::transmute(base + PUT_AWAY_RVA);
        put_away(actor);
    }
    if *logged < 400 {
        *logged += 1;
        let colour = if cs > 0x10000 { rd::<[u8; 4]>(cs + 0xC) } else { [0; 4] };
        log(&format!("Keyblade {}: actor flags {flags:08x} -> {:08x}, colour {colour:02x?}", if out { "out" } else { "put away" }, rd::<u32>(actor)));
    }
}

/// Sets the sparkle stub's flag (see `SPARKLE_RVA`).
unsafe fn set_sparkle(on: bool) {
    let f = SPARKLE_FLAG.load(Ordering::Relaxed);
    if f != 0 {
        std::ptr::write_volatile(f as *mut u8, on as u8);
    }
}

/// The flight sparkle's state test: a stub doing `cmp [rdi+0x70],8; je; cmp byte [flag],1`
/// (ZF set: flying), then the `mov eax,[rdi]` it replaced.
unsafe fn sparkle_stub(base: usize) {
    let site = base + SPARKLE_RVA;
    let Some(stub) = hook::alloc_near(site, 32) else {
        log("flight sparkle: no memory near its test");
        return;
    };
    let code: [u8; 17] = [0x83, 0x7F, 0x70, 0x08, 0x74, 0x07, 0x80, 0x3D, 0x03, 0x00, 0x00, 0x00, 0x01, 0x8B, 0x07, 0xC3, 0x00];
    std::ptr::copy_nonoverlapping(code.as_ptr(), stub as *mut u8, code.len());
    let mut new = [0u8; 8];
    new[0] = 0xE8;
    new[1..5].copy_from_slice(&((stub as i64 - (site as i64 + 5)) as i32).to_le_bytes());
    new[5..8].copy_from_slice(&[0x75, 0x5C, 0x90]);
    if hook::patch(site, &[0x83, 0x7F, 0x70, 0x08, 0x75, 0x5D, 0x8B, 0x07], &new, "flight sparkle test") {
        SPARKLE_FLAG.store(stub + 16, Ordering::Relaxed);
    }
}

pub unsafe fn install() {
    let base = sys_base();
    BASE.store(base, Ordering::Relaxed);
    sparkle_stub(base);
    hook::install(base + PHYS_RVA, &PHYS_PROLOGUE, physics as *const () as usize, &ORIG_PHYS, "actor physics step");
    let _ = hook::install(base + FOOTSTEP_RVA, &FOOTSTEP_PROLOGUE, footstep as *const () as usize, &ORIG_FOOTSTEP, "footstep sound");
    // (KH2's footsteps, KH1's dropped while KH2 has him: s229 had them off, the user then:
    // "revert the changes on footsteps".)
    let _ = hook::install(base + STEP_SOUND_RVA, &STEP_SOUND_PROLOGUE, step_sound as *const () as usize, &ORIG_STEP_SOUND, "step sound (KH2's footsteps)");
    // (Off, s241: the air spell's landing is KH1's again, see `takeover`'s hand-off.)
    // let _ = hook::install(base + PLAYER_STEP_RVA, &PLAYER_STEP_PROLOGUE, player_step as *const () as usize, &ORIG_PLAYER_STEP, "player step (an air spell's landing: KH2's)");
    if hook::install(base + CONTROL_RVA, &CONTROL_PROLOGUE, control as *const () as usize, &ORIG_CONTROL, "player control step").is_some() {
        CONTROL_HOOKED.store(true, Ordering::Relaxed);
    }
}

unsafe fn sys_base() -> usize {
    super::sys::GetModuleHandleA(std::ptr::null()) as usize
}
