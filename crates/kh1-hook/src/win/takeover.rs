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
        if super::log_on(super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 12 {
            log!(Takeover, "stick direction {dir:?} from the raw stick ({lx:.2}, {ly:.2}) through KH1's camera (convention {best}, mode {mode}); his +0x20 {old:?}, tilt {tilt:.2}");
        }
    }
    dir
}
const FLIGHT_STATE: i32 = 8;
/// KH1's glide (state 0x15, `0x1402B0BA0`): never while KH2 has him.
const KH1_GLIDE_STATE: i32 = 0x15;
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
/// KH1's air-move checks (see `grab`).
/// KH1's roll motion request (`0xDC`).
const KH1_ROLL_MOTION: u32 = 0xDC;
const CAN_ACT_RVA: Rva = Rva(0x2956E0);
/// KH1's control step for the player (`0x1402A4090 (actor, move vector)`, from the think step
/// `0x1402A1170`): it reads the pad and starts commands only while Sora can act
/// (`0x1402956E0`); events, tutorial windows and scripted moments skip it or stop him acting.
const CONTROL_RVA: Rva = Rva(0x2A4090);
const CONTROL_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x40];
static ORIG_CONTROL: AtomicUsize = AtomicUsize::new(0);
/// KH1's control step ran for Sora since the last physics step, with him able to act.
static CONTROLLED: AtomicBool = AtomicBool::new(true);
static CONTROL_HOOKED: AtomicBool = AtomicBool::new(false);
static FROZEN_FRAMES: AtomicU32 = AtomicU32::new(0);
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

static HANDOFF_FRAMES: AtomicU32 = AtomicU32::new(0);
/// Where KH1's move had him last frame (his speed while it moves him).
static HANDOFF_POS: Mutex<Option<[f32; 3]>> = Mutex::new(None);

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
        if super::log_on(super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 20 {
            log!(Takeover, "frame scale {dt:.2}: KH2 steps {n} 60 Hz frame{} a game frame", if n == 1 { "" } else { "s" });
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
        // (KH2's Magnet catch as a KH1 state, `magnet`.)
        super::magnet::install(BASE.load(Ordering::Relaxed));
    }
    // KH2's re-hit intervals on Sora's attacks run out (`rehit`), before KH1's collision pass.
    if sora {
        super::rehit::tick();
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
        // KH2's glide let go in KH1's control step: its velocity, which KH1's carry into the fall
        // replaced (`glide::carry`).
        super::glide::carry(actor);
        // KH2's turn toward the target in our slot's KH2 motion (`kh2slots::turn_to_lock`).
        super::kh2slots::turn_to_lock(actor);
        // KH2's next attack on the Keyblade in the same swing hits again (`kh2slots::keyblade_rearm`).
        super::kh2slots::keyblade_rearm(actor);
        // (t680) The same for Valor's left Keyblade's own KH1 attack object (`lefthits`).
        super::lefthits::step(actor);
        // KH2's guard repel's push back (`combomods::guard_frame`).
        super::combomods::guard_frame(actor);
        // KH2's Aerial Dodge's fall at its apex (`combomods::dodge_apex`).
        super::combomods::dodge_apex(actor);
        // KH2's push on KH1's push after a hit (`hitreact`).
        super::hitreact::push_frame(actor);
    }
    // An enemy on KH2's push after a KH2 hit (`knockback`).
    if !sora && actor > 0x10000 {
        super::knockback::push_frame(actor);
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

/// KH1's Glide and Superglide starts (`0x1402B0BA0` under both: its glide motion 0x71, air speed,
/// state 0x15; nothing below its height over the floor).
const GLIDE_RVA: Rva = Rva(0x2B0530);
const SUPERGLIDE_RVA: Rva = Rva(0x2B05C0);
/// KH1's ability test (`(ability, 0xB)`): 3 Glide, 4 Superglide (`0x1402A4600`'s use).
const HAS_ABILITY_RVA: Rva = Rva(0x282A70);


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
        if super::log_on(super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 10 {
            log!(Takeover, "KH1's glide didn't start ({:.0} above the floor, KH1 state {}): KH2 keeps him", rd::<f32>(actor + 0x3D4), rd::<i32>(actor + 0x70));
        }
        return None;
    }
    // (Glide at KH1 only, t321: KH1's own glide, its own motions and numbers whatever the physics
    // setting; Glide at KH2 is KH2's glide in KH1's glide state, `glide`.)
    Some(if superglide { "KH1's Superglide (Glide style KH1, KH1's own numbers)" } else { "KH1's glide (Glide style KH1, KH1's own numbers)" })
}

/// (`0x142D5F610`, its handler's `mov eax, [rip+...]` at `0x1402A5A65`; s240 read `0x142F5F610`,
/// which never had the bit: the user, "unable to input a kh1 dodge roll".)
const ROLL_ABILITY_RVA: Rva = Rva(0x2D5F610);
const ROLL_ABILITY: u32 = 0x20;


/// KH1 set Sora down out of an air spell, and its landing motion wasn't asked for: KH2 takes him
/// into KH2's landing (`takeover`'s hand-off, its next frame).
static AIR_SPELL_LANDED: AtomicBool = AtomicBool::new(false);


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
    // Before KH1's states have him (`takeover` hands him over next): a fresh jump on the ground is
    // KH1's own jump, not stripped (`JUMP_OUT`). (Before t877 it also asked whether the old KH2
    // controller's move could be left; that controller is deleted, so there is no KH2 move to leave.)
    let jump_out = is_sora && !HANDOFF.load(Ordering::Relaxed) && {
        let held = rd::<u32>(base + BUTTONS_RVA + 4) & 6 != 0;
        let fresh = held && !JUMP_OUT_PREV.swap(held, Ordering::Relaxed);
        fresh && rd::<i32>(actor + 0x70) == 0
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
        && (!HANDOFF.load(Ordering::Relaxed) || kh1_move_done(base, actor) || kh1_air_free(base, actor) || FROM_COMMAND.load(Ordering::Relaxed))
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
    let kh2_bits: u32 = if jump_out { 0x10_0000 } else { 0x6 | 0x10_0000 };
    let bits = |off: usize| -> u32 {
        if off != 8 {
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
    if strip {
        for off in [4usize, 8, 0x10, 0x14] {
            wr(state + off, rd::<u32>(state + off) & !bits(off));
        }
    }
    f(actor, dir);
    if strip {
        // KH1's own changes to the state meanwhile kept; only our bits come back.
        for off in [4usize, 8, 0x10, 0x14] {
            wr(state + off, rd::<u32>(state + off) | (saved[off / 4] & bits(off)));
        }
    }
}

/// A fresh jump on the ground let through to KH1's control step out of KH2's move (`shared`):
/// KH1's own jump, KH1 takes him (`takeover`, its state 1 next).
static JUMP_OUT: AtomicBool = AtomicBool::new(false);
static JUMP_OUT_PREV: AtomicBool = AtomicBool::new(true);

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
        kh1_rolling: state == 0 && rd::<u32>(actor + 0x164) == KH1_ROLL_MOTION,
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
    super::shared::physics::kh1_held_jump(actor);
    // (A Drive Form's ground move rising by its root, Valor's 154: KH1's air state before the
    // state's step, as Finishing Leap's, `shared::physics::form_lift_off`, t758.)
    super::shared::physics::form_lift_off(actor);
    let req_after = rd::<u32>(actor + 0x164);
    // (t320: KH1 asked for its glide motion (0x71..0x74) in this step; with what it saw, to find
    // the button word its glide reads when the dodge's press is meant to be hidden.)
    if matches!(req_after, 0x71..=0x74) && req_before != req_after && matches!(state, 1 | 2) {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 60 {
            let (dodged, up) = super::shared::dodge_flags();
            log!(Takeover,
                "glide asked in shared_control: KH1 state {state} -> {}, request {req_before:#x} -> {req_after:#x}, hide {hide:#x}, KH1 saw buttons {words_masked:08x?} (real {saved:08x?}), AIR_DODGED {dodged}, DODGE_JUMP_UP {up}, air_dodge {}, superglide {}, rising {}, can_act {}",
                rd::<i32>(actor + 0x70),
                seen.air_dodge,
                seen.superglide,
                seen.rising,
                seen.can_act,
            );
        }
    }
    wr(st + 4, rd::<u32>(st + 4) & !inject);
    for off in [4usize, 8, 0x10, 0x14] {
        wr(st + off, rd::<u32>(st + off) | (saved[off / 4] & hide));
    }
}


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

/// Sora's height (KH1's y, down positive) on the last control step, to tell KH1's jump rising.
static RISE_Y: AtomicU32 = AtomicU32::new(0);
/// KH1's spell could be cut by another now (`combat`: a spell chosen then is taken, else refused).
pub static SPELL_CUT_OPEN: AtomicBool = AtomicBool::new(false);
/// KH2's glide level while the Glide style is KH1's (its ability set to 0 meanwhile).
static KH2_GLIDE_KEPT: AtomicU32 = AtomicU32::new(0);
/// KH1's glide was tried on this hold of Square (`style_air_start`, KH1's jump).
static GLIDE_TRIED: AtomicBool = AtomicBool::new(false);

/// A command KH1 runs (a spell): when it started and how many attack objects of Sora's were
/// live then (a new one is the spell's shot going out).
static COMMAND: Mutex<Option<Command>> = Mutex::new(None);
/// A spell was chosen during KH1's cast (`combat::command` / `take_chosen_command`): waiting in
/// the command queue, it cuts the cast once the spell is out (a spell into a spell).
pub static MAGIC_AFTER: AtomicBool = AtomicBool::new(false);
/// KH2's motion on its last frame (`u32::MAX` while KH1 has Sora): a command chosen during a KH2
/// move waits only while that move plays (KH2's input buffer), not on a timer of its own (the
/// user: magic came out however much later it was; it is part of the combo string).
pub static KH2_MOTION: AtomicU32 = AtomicU32::new(u32::MAX);



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
    // use runs ~52 frames: `ITEM_CUT`). With KH2's item use as KH1's own item slots
    // (`kh2slots::ITEMS`), KH1's own action end: while KH1's item slot (58 / 59) plays, after the
    // few frames before KH1's step starts it.
    let since = ITEM_AT.try_lock().ok().and_then(|t| *t).map(|t| t.elapsed().as_secs_f32());
    let in_use = super::shared::active()
        && since.is_some_and(|s| {
            if super::kh2slots::has(0x3E, super::kh2slots::KH2_ITEM_USE) {
                s < 10.0 / 60.0 || matches!(pose::sora_slot(), Some(58 | 59))
            } else {
                s < (ITEM_CUT as f32 + 4.0) / 60.0
            }
        });
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
/// An item's use: KH2's from this frame (the user: on its frame 52 KH1's roll, guard or jump
/// came out).
const ITEM_CUT: u32 = 51;







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
                    log!(Takeover, "KH1 has Sora ({why})");
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
            log!(Takeover,
                "KH1 has had Sora {:.0} s: {why}; KH1 state {}, slot {:?}, flags {:08x}, event {:#x}, script {:#x}, body seen {}, ON {}",
                now.saturating_sub(SINCE.load(Ordering::Relaxed)) as f32 / 1000.0,
                rd::<i32>(actor + 0x70),
                pose::sora_slot(),
                rd::<u32>(actor),
                rd::<u32>(base + EVENT_RVA),
                rd::<u32>(actor + 0x440),
                pose::sora_body_seen(500),
                ON.load(Ordering::Relaxed)
            );
        }
    }
    let base = BASE.load(Ordering::Relaxed);
    let (world, room) = (rd::<u32>(base + WORLD_RVA), rd::<u32>(base + ROOM_RVA));
    let Ok(mut seen) = ROOM.try_lock() else { return };
    let now = now_ms();
    if seen.as_ref().is_none_or(|s| s.world != world || s.room != room) {
        log!(Takeover,
            "room: world {world:#x} room {room:#x} entered; {} (KH1 state {}, flags {:08x}, script {})",
            if kh2 { "KH2 has Sora".to_string() } else { format!("KH1 has Sora ({})", LAST_WHY.try_lock().map(|w| w.clone()).unwrap_or_default()) },
            rd::<i32>(actor + 0x70),
            rd::<u32>(actor),
            pose::ptr(rd::<u32>(actor + 0x440)) != 0,
        );
        *seen = Some(RoomSeen { world, room, at_ms: now, frames: 0, kh2_frames: 0, reported: false });
        // (t783) The load profiler's window opens for this room's first 10 s.
        super::loadprof::room_entered();
    }
    let Some(s) = seen.as_mut() else { return };
    s.frames += 1;
    if kh2 {
        s.kh2_frames += 1;
    }
    if !s.reported && now.saturating_sub(s.at_ms) >= 3000 {
        s.reported = true;
        log!(Takeover,
            "room {:#x}-{:#x}, first 3 s: KH2 had Sora {} of {} frames; now {} (KH1 state {}, body seen {})",
            s.world,
            s.room,
            s.kh2_frames,
            s.frames,
            if kh2 { "KH2 has him".to_string() } else { format!("KH1 has him: {}", LAST_WHY.try_lock().map(|w| w.clone()).unwrap_or_default()) },
            rd::<i32>(actor + 0x70),
            pose::sora_body_seen(500),
        );
    }
}

/// KH1's control step this frame (read once per frame, before anything can return early).
static FROZEN_NOW: AtomicBool = AtomicBool::new(false);
static RIDE_UNTIL: AtomicU64 = AtomicU64::new(0);

/// KH2's controller for Sora. Its abilities: `menu` (the ini's `abilities`: testing MAX, or KH1's
/// menu and level).
fn new_driver(kh2: &pose::Kh2) -> Driver {
    let character = kh2.character.clone();
    log!(Takeover, "KH2's controller ready for Sora");
    let mut anim = RigAnim::new(kh2.rig.clone());
    // KH2 snaps a finished action straight into idle; softened (the user: smoother out of a
    // combo).
    anim.queued_blend = 14.0;
    Driver { character, anim, held: (true, true), grounded: true, last_state: None, logged: 0, strikes: combat::Strikes::default(), fought: None, last_out: None, growth: menu::Growth::new(), swing: None, swing_frame: 0, on_top: None, drive: Default::default() }
}

/// A Drive Form on KH1's own states (`form::native_frame`): each physics frame of Sora's while
/// KH1's states have him, with KH2's controller's data (only read: the form's rows and growth).
unsafe fn form_native_frame(base: usize, actor: usize) {
    if !super::form::native_running() {
        return;
    }
    let Some(kh2) = pose::kh2() else { return };
    let Ok(mut g) = DRIVER.try_lock() else { return };
    let d = g.get_or_insert_with(|| new_driver(kh2));
    let battle = rd::<u32>(base + BATTLE_RVA) & 1 != 0;
    super::form::native_frame(&mut d.drive, &mut d.character, kh2, actor, battle, steps());
}

/// KH2's movement numbers with Sora's abilities now (KH1's menu), for KH2's physics on KH1's own
/// states (`shared::physics`).
unsafe fn refresh_kh2_numbers(base: usize) {
    let Some(kh2) = pose::kh2() else { return };
    let Ok(mut g) = DRIVER.try_lock() else { return };
    let d = g.get_or_insert_with(|| new_driver(kh2));
    d.growth.update(base, &mut d.character);
    super::form::abilities(&mut d.drive, &mut d.character);
    // (KH2 Sora's own numbers, not the controller's now: with physics KH1 those are KH1's,
    // `shared::physics::kh2_controller`. In a Drive Form on KH1's own states, the form's: Valor's
    // run 12 and jump, its High Jump at its form level (`formexp`), `prty` moveset 2, Checked (files).)
    let own = match super::form::current().filter(|_| super::form::native_now()).and_then(|k| k.data(kh2)) {
        Some(f) => &f.character,
        None => &kh2.character,
    };
    let p = &own.params;
    let high = d.character.abilities.high_jump;
    let (jump, high_jump) = (p.jump_height, if high == 0 { p.jump_height } else { own.growth_level(high).map_or(p.jump_height, |g| g.high_jump_height) });
    // (KH2's jump start with High Jump is its High Jump, whichever of KH1's takeoffs asked: Valor
    // always has it, so KH1's plain takeoff too, `kh2slots::VALOR_SPECS`.)
    let jump = if super::form::native_now() && high > 0 { high_jump } else { jump };
    super::shared::physics::set_numbers(super::shared::physics::Numbers {
        walk: p.walk_speed,
        run: p.run_speed,
        jump,
        high_jump,
        gravity: p.gravity,
        fall_max: p.fall_max,
        air_keep: p.air_keep,
        air_drag: p.air_drag,
        acceleration: p.acceleration,
        brake: p.brake,
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


/// The next `kh2_would_take*` tests KH2's line ahead (`combomods::drop_ahead`, along the
/// selector's direction there, `Character::selector_aim`) for row 1 (`Character::set_drop_ahead`):
/// the ground combo's press only, so the air, Square and hand-back asks never see it.
static DROP_TEST: AtomicBool = AtomicBool::new(false);

pub fn test_drop_ahead(on: bool) {
    DROP_TEST.store(on, Ordering::Relaxed);
}

/// KH2's ability rows inside KH1's own row picker (`combomods::picker`, Base / Aerial Base Combo
/// KH1): the first equipped ability row of the kind KH1's count is at (`finisher`), playable on
/// KH1's rows (`usable`), not the KH2 motion playing (`playing`), that passes KH2's own row checks
/// against base Sora's targets and KH1's lock as KH2's step sees them, with its hop
/// (`Character::ability_row`). KH2's combo length and selector walk aren't run.
pub unsafe fn kh2_ability_row(actor: usize, finisher: bool, air: bool, playing: Option<u16>, usable: &dyn Fn(&character_core::ComboRow) -> bool) -> Option<(character_core::ComboRow, Option<character_core::Launch>)> {
    kh2_rows(actor, air, |ch, stick, env| ch.ability_row(finisher, air, playing, usable, stick, env)).flatten()
}

/// KH2's row checks for KH1's own walk (`combomods::finisher_walk`, Base / Aerial Base Combo KH1):
/// `f` gets KH2's rows and checks (`Character::ability_row_id`, `row_in_range`) with base Sora's
/// targets and KH1's lock as KH2's step sees them, the stick, and his place; nothing of KH2's
/// controller moves him. `None` without KH2's data.
pub unsafe fn kh2_rows<R>(actor: usize, air: bool, f: impl FnOnce(&mut Character, [f32; 2], &Env) -> R) -> Option<R> {
    let base = BASE.load(Ordering::Relaxed);
    let kh2 = pose::kh2()?;
    let mut g = DRIVER.try_lock().ok()?;
    let d = g.get_or_insert_with(|| new_driver(kh2));
    d.growth.update(base, &mut d.character);
    super::form::abilities(&mut d.drive, &mut d.character);
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
    Some(f(&mut d.character, stick, &env))
}

/// `kh2_would_take`, with the hop KH2's row starts there (`Character::would_take_launch`: a row
/// with `jump`, its height and sideways velocity for that target).
pub unsafe fn kh2_would_take_launch(actor: usize, from: Option<character_core::ComboState>, playing: Option<u16>, air: bool, square: bool) -> Option<(character_core::ComboRow, u8, u8, Option<character_core::Launch>)> {
    kh2_walk(actor, from, playing, air, square).flatten()
}

/// `kh2_would_take_launch`, telling "KH2's walk takes nothing" (`Some(None)`) from "KH2's walk
/// couldn't be asked" (`None`: no KH2 data loaded, or its tables busy this instant), which KH1's
/// picker treats apart (`kh1picker::at_kh2`: a refusal is no attack, as KH2's; an unasked press
/// keeps KH1's own row).
pub unsafe fn kh2_walk(actor: usize, from: Option<character_core::ComboState>, playing: Option<u16>, air: bool, square: bool) -> Option<Option<(character_core::ComboRow, u8, u8, Option<character_core::Launch>)>> {
    let base = BASE.load(Ordering::Relaxed);
    let kh2 = pose::kh2()?;
    let mut g = DRIVER.try_lock().ok()?;
    let d = g.get_or_insert_with(|| new_driver(kh2));
    d.growth.update(base, &mut d.character);
    super::form::abilities(&mut d.drive, &mut d.character);
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
    let Some((row, launch)) = d.character.would_take_launch(from, playing, air, square, stick, &env) else { return Some(None) };
    Some(Some((row, d.character.combo_length(air), d.character.abilities.negative_combo, launch)))
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
    super::form::abilities(&mut d.drive, &mut d.character);
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

/// KH2's Square move in a combo for Sora now, KH1's own states having him (`Character::
/// would_take_square_exe`: the exe's own order and checks, Steam KH2 `0x1403A81F0` /
/// `0x1403A9950`), from KH1's combo `from`, KH2's combo / finisher window open at the motion's
/// frame (`combo`, `finisher`), KH2's free state there (`free`), rows `skip` left out; with the
/// targets and KH1's lock as KH2's step sees them (`kh2_would_take_launch`). Base Sora's and Valor
/// Form's Square moves in a combo (`combomods::control`, `combomods::valor_square`): one path.
pub unsafe fn kh2_would_take_square(actor: usize, from: character_core::ComboState, combo: bool, finisher: bool, free: bool, skip: &[u8], air: bool) -> Option<(character_core::ComboRow, Option<character_core::Launch>)> {
    let base = BASE.load(Ordering::Relaxed);
    let kh2 = pose::kh2()?;
    let mut g = DRIVER.try_lock().ok()?;
    let d = g.get_or_insert_with(|| new_driver(kh2));
    d.growth.update(base, &mut d.character);
    super::form::abilities(&mut d.drive, &mut d.character);
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
    d.character.would_take_square_exe(from, combo, finisher, free, skip, air, stick, &env)
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
    // (KH2's star a spell script gave him, on KH1's own states, t817; Reflect's step has it then.)
    if super::shared::active() && !reflect {
        super::reflect::star_step(actor);
    }
    // (Wisdom Form on KH1's own states too: its shots' script, `formmagic`, from its motions' frame
    // trigger 1 that KH1's motion pass fires, `kh2slots::sound_event`. Forms still on KH2's
    // controller run theirs from its step.)
    if !super::shared::active() || !(super::formmagic::busy() || reflect) {
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
        // (The clip of the moveset playing: a native Drive Form's, `pose::kh2_moves`.)
        if let Some(c) = pose::kh2_moves().unwrap_or(&kh2.character).moveset.clips.get(&m) {
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
/// ATKP (power, launch, hold), once per window and target as KH2's hit records
/// allow. KH2 updates once a game frame (delta 2 at 30 fps), so its windows are read at the
/// frame's time and the records' clocks move on by the frame's 60 Hz steps. A hit landed counts as
/// KH1's swing connecting (the combat info's `+0x50`, which KH1's Attack step reads for the next
/// press, `combomods`). Nothing when no such window is open (a few reads a frame while our slot
/// plays). The Keyblade's windows stay KH1's sphere's (`kh2slots::kh1_tracks`).
///
/// (t-native body hits) On Steam the body windows are KH1 attack objects of ours that KH1's own
/// collision pass collides (`bodyhits`); this sweep then takes no body window and only keeps the
/// guard windows (the Magnet's catch is KH1's state of ours, `magnet`, t862). Elsewhere it runs as
/// before, without the Magnet's pull.
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
    // KH2's guard windows in that motion: KH1 guard objects, as KH2's controller makes them for
    // every `Guard` window of its moves (`combat::guard_step`), so an enemy's attack object meeting
    // one is stopped before it reaches him (the parry; `combomods::guard_repel` then, by the ATKP's
    // refact self). On the Keyblade: the guard press's, its repel's, Guard Break's. On his body
    // (all round, refact self 2: stopped, no reaction, the move goes on): the Aerial Dodge's 406
    // all through 202 (t494), and now every other one, as KH2 has them (the tester, thread "Body
    // guards on KH2 moves", from aerial-dodge-native's question 7: "(a)"): Aerial Recovery 40's 124
    // at 0..16, Explosion 166's 124 at 32..64, Magnet Burst 197's 406 at 20..70 (Checked (files):
    // `P_EX100.mset`, 00battle.bin's ATKP 124 kind 3 / 406 kind 2, both refact self 2, reflect range
    // 0). Hits his hit pass finds there are still `hurt`'s too (`kh2slots::body_guard`).
    let guards: Vec<(usize, u16, u8, u8)> = match (playing, clip) {
        (Some((_, t)), Some((_, c))) => c
            .windows
            .iter()
            .enumerate()
            .filter(|(_, w)| w.contains(t as u32))
            .filter_map(|(i, w)| match w.kind {
                character_core::WindowKind::Guard { attack, group, weapon } => Some((i, attack, group, weapon)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    // (KH2's hit windows on his body are KH1 attack objects of ours when `bodyhits` is in: KH1's
    // own collision pass hits with them, not this sweep.)
    let native_body = super::bodyhits::native();
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
                    character_core::WindowKind::Hitbox { attack, group, weapon: 0, .. } if !native_body => Some((i, attack, group)),
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
        if super::log_on(super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 60 {
            log!(Takeover, "KH2's hits on his body in KH1's states: KH2 motion {motion} frame {t:.0}, {landed} landed (ATKP {:?})", open.iter().map(|o| o.1).collect::<Vec<_>>());
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
/// `ATTACK_AFTER`), a command (a KH2 spell, Reflect, a Drive: each on KH1's own states), or
/// the buttons' (`shared::control`: Square, the jump in the air). Only while KH1 lets him act
/// (no buffer: a press KH1 can't let go for is dropped).
unsafe fn shared_frame(base: usize, actor: usize) -> bool {
    // A Drive Form on KH1's own states (Valor, `form::native_frame`): its change, its MP, its end.
    form_native_frame(base, actor);
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
    if state == 2 && free && square_fresh && falling && super::styles::kh1_now(super::styles::Kind::Glide) && glide_on_square_now(base) && kh1_glide_open(base) && !GLIDE_TRIED.swap(true, Ordering::Relaxed) {
        if let Some(what) = kh1_glide_start(base, actor) {
            log!(Takeover, "{what}, on Square (shared states)");
        }
    }
    // (t875: KH2's controller no longer borrows him for anything; KH1's own states keep him.)
    release("KH1's own states (base Sora, shared)");
    HANDOFF_FRAME.store(true, Ordering::Relaxed);
    false
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
    // (t874) KH1's own states always have base Sora: the old KH2 controller's driving of him (its
    // frame, the hand-off and take-back around KH1's moves, its borrows) is deleted (the owner,
    // chat: "deprecate the old controller", answer "2": delete it outright). What KH1's states
    // run each frame with KH2's data is `shared_frame` (a form on KH1's states, KH2's numbers, the
    // glide on Square); no borrow of KH2's controller is left (t875).
    if !HANDOFF.load(Ordering::Relaxed) {
        to_kh1_states("base Sora: KH1's own states (shared)");
        return false;
    }
    // One of KH1's moves from a command: KH1's own states take it on once KH1 lets him act (a
    // command's cut windows until then), as before.
    if !super::shared::active() {
        let can_act: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + CAN_ACT_RVA);
        let command = FROM_COMMAND.load(Ordering::Relaxed);
        if !command || can_act(actor) != 0 {
            FROM_COMMAND.store(false, Ordering::Relaxed);
            ITEM_COMMAND.store(false, Ordering::Relaxed);
            ATTACK_AFTER.store(false, Ordering::Relaxed);
            MAGIC_AFTER.store(false, Ordering::Relaxed);
            set_air_cast(None);
            super::stylecombo::clear();
            super::shared::started();
            log!(Takeover, "KH1's move (state {}) goes on in KH1's own states (shared)", rd::<i32>(actor + 0x70));
        } else {
            HANDOFF_SEEN_MS.store(now_ms().max(1), Ordering::Relaxed);
            HANDOFF_FRAME.store(true, Ordering::Relaxed);
            release(&format!("KH1's own move, KH1 state {}", rd::<i32>(actor + 0x70)));
            return false;
        }
    }
    let _ = shared_frame(base, actor);
    false
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

unsafe extern "system" fn footstep(actor: usize, kind: i32) -> usize {
    let base = BASE.load(Ordering::Relaxed);
    if actor > 0x10000 && actor == rd::<u64>(base + PARTY_RVA) as usize && FRAME_KIND.load(Ordering::Relaxed) == 1 && !KH2_STEPS.load(Ordering::Relaxed) {
        static N: AtomicU32 = AtomicU32::new(0);
        if super::log_on(super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 10 {
            log!(Takeover, "KH1's footstep held back (KH2 isn't walking him)");
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
        if super::log_on(super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 10 {
            log!(Takeover, "KH1's step sound (kind {kind}) dropped: KH2 has Sora, KH2's own steps sound");
        }
        return;
    }
    let f: unsafe extern "system" fn(usize, i32) = std::mem::transmute(ORIG_STEP_SOUND.load(Ordering::Acquire));
    f(actor, kind)
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


/// Sora's physics frames so far (one per game frame; `TICKS` counts them in 60 Hz frames, which
/// `pose`'s hair and cloth swing steps by).
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

/// KH2's hit state (`combat::Strikes`) when nothing else holds it (a spell script's call, made
/// from KH1's script machine after the frame's actor update, t825).
pub fn with_strikes<R>(f: impl FnOnce(&mut combat::Strikes) -> R) -> Option<R> {
    let mut g = DRIVER.try_lock().ok()?;
    g.as_mut().map(|d| f(&mut d.strikes))
}

/// KH1's lock-on target now (`0x142DB3F10` set, `0x142DB3F18` its actor) and the lock point it is
/// on (`+4`), or `None` (Checked (code): as `kh1_lock` reads them). For the spell scripts' world
/// (`reflect::KhWorld`), read when a script asks.
pub unsafe fn kh1_locked() -> Option<(usize, i32)> {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 || rd::<u32>(base + LOCK_STATE_RVA) == 0 {
        return None;
    }
    let a = pose::ptr(rd::<u32>(base + LOCK_TARGET_RVA));
    (a > 0x10000).then(|| (a, rd::<i32>(base + LOCK_TARGET_RVA + 4).max(0)))
}

/// KH1's lock point `point` of `actor` (`0x1402AE330`, KH1 world, y down), if KH1 gives one.
pub unsafe fn kh1_lock_point(actor: usize, point: i32) -> Option<[f32; 3]> {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 || actor < 0x10000 {
        return None;
    }
    let f: unsafe extern "system" fn(usize, i32, *mut [f32; 4]) -> f32 = std::mem::transmute(base + LOCK_POINT_RVA);
    let mut v = [0.0f32; 4];
    let r = f(actor, point, &mut v);
    (r.is_finite() && r >= 0.0 && v[..3].iter().all(|x| x.is_finite())).then(|| [v[0], v[1], v[2]])
}

/// KH1's point on an actor's body (`0x14029C990 (actor, code, t, out)`, Checked (decomp)): `0x8003`
/// its body's centre (its collision body's middle, else `+0x50`), `0x30` along its Keyblade (`t`
/// 0 its start, 1 its tip), `0x4000 | b` bone `b`. KH1 world, y down.
pub unsafe fn kh1_bone_point(actor: usize, code: u32, t: f32) -> Option<[f32; 3]> {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 || actor < 0x10000 {
        return None;
    }
    let f: unsafe extern "system" fn(usize, u32, f32, *mut [f32; 4]) = std::mem::transmute(base + BONE_POINT_RVA);
    let mut v = [0.0f32; 4];
    f(actor, code, t, &mut v);
    v[..3].iter().all(|x| x.is_finite()).then(|| [v[0], v[1], v[2]])
}
const BONE_POINT_RVA: Rva = Rva(0x29C990);

/// KH1's count of ability `id` equipped by party member `member` (`0x140282A70 (id, member)`,
/// Checked (decomp): member 0..9 its own list, 10 and up the shared list).
pub unsafe fn kh1_ability(id: i32, member: i32) -> i32 {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 {
        return 0;
    }
    let has: unsafe extern "system" fn(i32, i32) -> i32 = std::mem::transmute(base + HAS_ABILITY_RVA);
    has(id, member)
}


static KH2_AIR: AtomicBool = AtomicBool::new(false);


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
    AIR_SPELL_LANDED.store(false, Ordering::Relaxed);
    SPELL_CUT_OPEN.store(false, Ordering::Relaxed);
    CUT_LEFT.store(false, Ordering::Relaxed);
    ATTACK_AFTER.store(false, Ordering::Relaxed);
    HANDOFF_FRAMES.store(0, Ordering::Relaxed);
    HANDOFF.store(true, Ordering::Relaxed);
    log!(Takeover, "KH1 takes him: {why}");
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
        log!(Takeover, "flight sparkle: no memory near its test");
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
