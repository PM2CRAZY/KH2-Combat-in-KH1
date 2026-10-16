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

use super::{combat, hook, log, menu, pose};
use character_core::{Button, Character, Env, Input, Output, StateName, TargetInfo};
use glam::Mat4;
use kh2_rig::RigAnim;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

const PHYS_RVA: usize = 0x298230;
const PHYS_PROLOGUE: [u8; 12] = [0x40, 0x55, 0x53, 0x41, 0x56, 0x48, 0x8B, 0xEC, 0x48, 0x83, 0xEC, 0x50];
const MOVE_RVA: usize = 0x297710;
const PARTY_RVA: usize = 0x2D37C80;
const EVENT_RVA: usize = 0x298D3A0;
const BATTLE_RVA: usize = 0x2867CD8;
const LOCK_STATE_RVA: usize = 0x2DB3F10;
const LOCK_TARGET_RVA: usize = 0x2DB3F18;
/// A lock point's world position (`0x1402AE330 (actor, point, out) -> radius`, KH1 y-down).
const LOCK_POINT_RVA: usize = 0x2AE330;
const DRAW_WEAPON_RVA: usize = 0x2A0CA0;
const PUT_AWAY_RVA: usize = 0x2A10F0;
/// Actor flag: the weapon is out (drawn).
const WEAPON_OUT: u32 = 0x4000;
/// After an attack in the field, how long Sora keeps KH2's battle stance (Keyblade in hand)
/// before KH2's field idle puts it away (the user: a combo ended in the empty-handed idle).
const BATTLE_LINGER: f32 = 6.0;
/// How far below the feet the floor may drop and still be stepped down onto (KH2's floor ray
/// reaches 24 below).
const STEP_DOWN: f32 = 24.0;
/// KH1's air-move checks (see `grab`).
const CAN_ACT_RVA: usize = 0x2956E0;
/// KH1's tally of the save (`0x14028A4D0(id)`: `+0x848 + id * 2`).
const COUNT_RVA: usize = 0x28A4D0;
/// KH1's control step for the player (`0x1402A4090 (actor, move vector)`, from the think step
/// `0x1402A1170`): it reads the pad and starts commands only while Sora can act
/// (`0x1402956E0`); events, tutorial windows and scripted moments skip it or stop him acting.
const CONTROL_RVA: usize = 0x2A4090;
const CONTROL_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x40];
static ORIG_CONTROL: AtomicUsize = AtomicUsize::new(0);
/// KH1's control step ran for Sora since the last physics step, with him able to act.
static CONTROLLED: AtomicBool = AtomicBool::new(true);
/// KH1 holds him still (logged on a change).
static FROZEN: AtomicBool = AtomicBool::new(false);
static CONTROL_HOOKED: AtomicBool = AtomicBool::new(false);
static FROZEN_FRAMES: AtomicU32 = AtomicU32::new(0);
const VINE_RVA: usize = 0x2C5710;
const LEDGE_RVA: usize = 0x2C68A0;
const BAR_RVA: usize = 0x2C7E00;
const NO_BARS_RVA: usize = 0x298D3A8;
const HANG_RVA: usize = 0x2C17B0;
const SWIM_RVA: usize = 0x2BF410;
/// KH1 has him for one of its own moves (`grab`); KH2 starts over afterwards (`FRESH`).
static HANDOFF: AtomicBool = AtomicBool::new(false);
static FRESH: AtomicBool = AtomicBool::new(false);
static HANDOFF_FRAMES: AtomicU32 = AtomicU32::new(0);
/// Square was pressed as KH1's move ended: KH2 takes it as a press (not one already held).
static ROLL_NOW: AtomicBool = AtomicBool::new(false);
/// KH1's fall state (after a jump's rise, or off an edge): its collision is the air kind
/// (`0x140297F20` sets flag 1, no snapping onto the floor), and the wall solver's only extra
/// for it (`0x1402AD260`) clears its fall speed `+0x3C4` at a ceiling. No enter / exit handlers.
const AIR_STATE: i32 = 3;
/// KH1's generic air state (what its physics step puts an airborne actor in): its object-top
/// finder (`0x1402B3950`) looks below only in it.
const FALL_STATE: i32 = 2;
/// KH1's object-top finder `0x1402B3950 (actor, move) -> object`: the standable object (box,
/// barrel: collision shapes of type 2 / 3) whose top the move ends at or under, the move's
/// height cut so it ends on the top. The map floor (`+0x320`) doesn't know objects.
const OBJECT_TOP_RVA: usize = 0x2B3950;
/// Sets the object the actor stands on (`+0x398`).
const STAND_ON_RVA: usize = 0x295C60;

pub const CIRCLE: u16 = 0x2000;
pub const SQUARE: u16 = 0x8000;
/// KH1's shortcut button (L1 + a face button).
pub const L1: u16 = 0x0400;
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
static CLOCK: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn now_ms() -> u64 {
    CLOCK.get_or_init(Instant::now).elapsed().as_millis() as u64
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
}

static DRIVER: Mutex<Option<Driver>> = Mutex::new(None);
/// KH2's pose this frame (KH2 model space), for `pose` to draw.
pub static POSE: Mutex<Option<(Vec<Mat4>, Instant)>> = Mutex::new(None);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

type PhysFn = unsafe extern "system" fn(usize);
type MoveFn = unsafe extern "system" fn(usize, *const [f32; 4], u32);

unsafe extern "system" fn physics(actor: usize) {
    if ON.load(Ordering::Relaxed) && actor > 0x10000 && takeover(actor) {
        return;
    }
    // KH1 has him this frame: a Keyblade trail KH2's swing asked for ends.
    if actor > 0x10000 && actor == rd::<u64>(BASE.load(Ordering::Relaxed) + PARTY_RVA) as usize {
        combat::trail_off(actor);
    }
    let f: PhysFn = std::mem::transmute(ORIG_PHYS.load(Ordering::Acquire));
    f(actor);
}

unsafe extern "system" fn control(actor: usize, dir: usize) {
    let base = BASE.load(Ordering::Relaxed);
    if actor > 0x10000 && actor == rd::<u64>(base + PARTY_RVA) as usize {
        let can: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + CAN_ACT_RVA);
        if can(actor) != 0 {
            CONTROLLED.store(true, Ordering::Relaxed);
        }
    }
    let f: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_CONTROL.load(Ordering::Acquire));
    f(actor, dir);
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
    drop_pose();
    if let Ok(mut d) = DRIVER.try_lock() {
        if let Some(dr) = d.as_mut() {
            if dr.logged < 400 && dr.last_state.is_some() {
                dr.logged += 1;
                log(&format!("KH1 has Sora ({why})"));
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

unsafe fn takeover(actor: usize) -> bool {
    let base = BASE.load(Ordering::Relaxed);
    if actor != rd::<u64>(base + PARTY_RVA) as usize {
        return false;
    }
    if rd::<u32>(base + EVENT_RVA) != 0 {
        SCRIPTED.store(false, Ordering::Relaxed);
        release("an event runs");
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
    // KH1 took him for one of its own air moves (`grab`): until it has him standing again.
    if HANDOFF.load(Ordering::Relaxed) {
        let state = rd::<i32>(actor + 0x70);
        let frames = HANDOFF_FRAMES.fetch_add(1, Ordering::Relaxed);
        // Square there (a dodge roll right after jumping off a pole) is KH2's: KH1 doesn't see
        // it (`kh1_keeps_square`), and KH2 takes him at once to roll (the user: KH1 had started
        // its own roll, and he slid backwards when KH2 took over mid-way).
        let square = RAW_BUTTONS.load(Ordering::Relaxed) as u16 & SQUARE != 0;
        if frames >= 10 && matches!(state, 0 | 1) && (kh1_moving() || square) {
            HANDOFF.store(false, Ordering::Relaxed);
            ROLL_NOW.store(square, Ordering::Relaxed);
            FRESH.store(true, Ordering::Relaxed);
            log(&format!("KH1's move ended (state {state}): KH2 again"));
        } else {
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
    let state = rd::<i32>(actor + 0x70);
    let own_mode = kh1_mode(state);
    KH1_MODE.store(own_mode, Ordering::Relaxed);
    if own_mode {
        release(&format!("KH1's own movement mode (KH1 state {state})"));
        return false;
    }
    let slot = pose::sora_slot();
    let airborne = DRIVER.try_lock().ok().and_then(|d| d.as_ref().map(|d| d.last_state.is_some() && d.character.airborne())).unwrap_or(false);
    // (A motion we don't know, as the Dream Shield's own moveset's, is not taken for KH1's own
    // action: the user saw KH1's idle and attacks with the shield.)
    if matches!(slot, Some(s) if s > 7) && !airborne {
        release(&format!("KH1's own action, slot {}, KH1 state {}", slot.map_or("?".into(), |s| s.to_string()), rd::<i32>(actor + 0x70)));
        return false;
    }
    let Some(kh2) = pose::kh2() else { return false };
    let Ok(mut g) = DRIVER.try_lock() else { return false };
    let d = g.get_or_insert_with(|| {
        // Its abilities: `menu` (the ini's `abilities`: testing MAX, or KH1's menu and level).
        let character = kh2.character.clone();
        log("KH2's controller ready for Sora");
        let mut anim = RigAnim::new(kh2.rig.clone());
        // KH2 snaps a finished action straight into idle; softened (the user: smoother out of
        // a combo).
        anim.queued_blend = 14.0;
        Driver { character, anim, held: (true, true), grounded: true, last_state: None, logged: 0, strikes: combat::Strikes::default(), fought: None, last_out: None, growth: menu::Growth::new(), swing: None, swing_frame: 0, on_top: None }
    });
    if FRESH.swap(false, Ordering::Relaxed) {
        // Back from KH1's own move: KH2's controller starts over (standing), keeping its
        // abilities.
        let abilities = d.character.abilities.clone();
        d.character = kh2.character.clone();
        d.character.abilities = abilities;
        d.grounded = true;
        // An Attack press from before (during KH1's move) is not KH2's.
        combat::take_press();
    }
    if ROLL_NOW.swap(false, Ordering::Relaxed) {
        d.held.1 = false;
    }
    d.growth.update(base, &mut d.character);
    if d.last_state.is_none() {
        // (Re)starting: face where KH1 had him facing.
        d.character.set_facing(rd::<f32>(actor + 0x3C));
    }

    // Input: direction from KH1's camera-relative move direction, tilt from the left stick.
    let buttons = RAW_BUTTONS.load(Ordering::Relaxed) as u16;
    let sticks = RAW_STICKS.load(Ordering::Relaxed).to_le_bytes();
    let (lx, ly) = ((sticks[2] as f32 - 128.0) / 127.0, (sticks[3] as f32 - 128.0) / 127.0);
    let tilt = ((lx * lx + ly * ly).sqrt().min(1.0) - 0.2).max(0.0) / 0.8;
    let dir = glam::Vec2::new(rd::<f32>(actor + 0x20), rd::<f32>(actor + 0x28)).normalize_or_zero();
    let stick = if tilt > 0.0 { (dir * tilt).to_array() } else { [0.0, 0.0] };
    // With L1 held the face buttons are KH1's shortcuts, not KH2's jump / action; a button
    // still held when L1 lets go stays KH1's until released (`held` sees it raw).
    let shortcut = buttons & L1 != 0;
    let jump_raw = buttons & CIRCLE != 0;
    let action_raw = buttons & SQUARE != 0;
    let jump = jump_raw && !shortcut;
    let action = action_raw && !shortcut;
    // KH1's Attack command (Cross with Attack chosen in the menu) is KH2's attack button.
    let attack = combat::take_press();
    // KH1 holding him still (its control step skipped or him unable to act: cutscene moments,
    // tutorial windows; the user: he moved anyway): KH2 gets no input (he still falls, lands
    // and finishes what he was doing).
    let frozen = !CONTROLLED.swap(false, Ordering::Relaxed) && CONTROL_HOOKED.load(Ordering::Relaxed);
    if FROZEN.swap(frozen, Ordering::Relaxed) != frozen {
        log(if frozen { "KH1 holds Sora still: no input for KH2" } else { "KH1 lets Sora act again" });
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
        drop_pose();
        return false;
    }
    let (stick, jump, action, attack) = if frozen { ([0.0, 0.0], false, false, false) } else { (stick, jump, action, attack) };
    let input = Input {
        stick,
        jump: Button { pressed: jump && !d.held.0, held: jump },
        attack: Button { pressed: attack, held: attack },
        action: Button { pressed: action && !d.held.1, held: action },
        lock: Button::default(),
    };
    d.held = (jump_raw, action_raw);

    // Where he is, KH2's way up (KH1's world is y-down).
    let p = rd::<[f32; 3]>(actor + 0x10);
    let pos = [p[0], -p[1], p[2]];
    let above = rd::<f32>(actor + 0x3D4);
    let mut targets = d.strikes.look(actor);
    // KH1's lock-on (R1): while it holds a target (`0x142DB3F10` its state, the target's handle
    // at `0x142DB3F18`, which the command menu aims every command at), KH2 locks the same
    // enemy, so its swings home in on and turn toward it.
    let locked = if rd::<u32>(base + LOCK_STATE_RVA) != 0 { pose::ptr(rd::<u32>(base + LOCK_TARGET_RVA)) } else { 0 };
    // KH1 locks a point of the target (`0x142DB3F1C`, an index into its collision record's lock
    // points; `docs/kh1-lockon.md`): KH2 homes in on that point (the user: locked on Darkside's
    // hand, Sora went for his body), wherever the actor's root is.
    if locked > 0x10000 {
        let point: unsafe extern "system" fn(usize, i32, *mut [f32; 4]) -> f32 = std::mem::transmute(base + LOCK_POINT_RVA);
        let mut v = [0.0f32; 4];
        let r = point(locked, rd::<i32>(base + LOCK_TARGET_RVA + 4).max(0), &mut v);
        if v[..3].iter().all(|x| x.is_finite()) && r.is_finite() && r >= 0.0 {
            let r = r.min(500.0);
            let t = TargetInfo { id: locked as u32, pos: [v[0], -v[1] - r, v[2]], radius: r.max(10.0), height: (2.0 * r).max(20.0) };
            match targets.iter_mut().find(|x| x.id == t.id) {
                Some(x) => *x = t,
                None => targets.push(t),
            }
        }
    }
    let lock = (locked > 0x10000).then(|| locked as u32).filter(|id| targets.iter().any(|t| t.id == *id));
    if lock != d.character.lock() && d.logged < 400 {
        d.logged += 1;
        log(&format!("lock-on: {lock:?}"));
    }
    d.character.set_lock(lock);
    // KH2 aims a swing at any enemy in reach; here only at KH1's own target, the R1 lock or the
    // small cursor's soft target (the user: Sora swung toward points he wasn't targeting).
    if !super::settings::get().auto_aim {
        targets.retain(|t| Some(t.id) == lock);
    }
    let env = Env { pos, grounded: d.grounded, height_above_ground: above.max(0.0), targets: &targets, ledges: &[] };
    let out = d.character.step(&input, &env);

    // KH2's displacement: the sideways part through KH1's mover (walls, slopes, steps up); the
    // height is KH2's while he is in the air (KH1's mover snaps a grounded actor back onto the
    // floor every frame, which undid a jump's first small rises: the user, stage 8).
    let air = d.character.airborne();
    let rising = out.displacement[1] > 0.0;
    let up_before = -p[1];
    wr(actor + 0x490, rd::<[f32; 4]>(actor + 0x10));
    // What KH1's physics step adds to a move (`0x140298230`), as it would: a carrying
    // platform's motion (`+0x200`, while `+0x160 & 4`) and the frame's push (`+0x120`,
    // cleared once used), and the touched-objects mask (`+0x90`) cleared before the move.
    let settings = super::settings::get();
    let carry = if settings.carry { carried(actor, &mut d.logged) } else { [0.0; 3] };
    if settings.carry {
        wr(actor + 0x90, 0u64);
    }
    let mv: MoveFn = std::mem::transmute(base + MOVE_RVA);
    // Rising: the rise goes through KH1's mover too, in KH1's fall state for the move so that
    // it collides as in the air (its collision query `0x140297F20` takes that from the movement
    // state; in the ground state it snaps him back onto the floor), which stops him at a
    // ceiling (KH1's own jump: `0x1402AD260`).
    // Falling too (the user: he couldn't land on boxes): KH1's fall state's mover stops him on
    // whatever is under him, objects included, as KH1's own fall does; the map floor alone
    // (`+0x320`) doesn't know boxes.
    let rise3d = settings.ceilings && (rising || air);
    if rise3d {
        let state = rd::<i32>(actor + 0x70);
        wr(actor + 0x70, AIR_STATE);
        mv(actor, &[out.displacement[0] + carry[0], -out.displacement[1] + carry[1], out.displacement[2] + carry[2], 0.0], 0);
        wr(actor + 0x70, state);
    } else {
        mv(actor, &[out.displacement[0] + carry[0], 0.0, out.displacement[2] + carry[2], 0.0], 0);
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
        if rise3d && rising {
            // KH2's ceiling rule: less than 49% of the rise happened.
            let got = -rd::<f32>(actor + 0x14);
            y = got;
            if out.displacement[1] > 0.5 && got - up_before < out.displacement[1] * 0.49 {
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
    // KH1's own air moves (its jump's input step `0x1402ADA90` tries them in this order):
    // vines and climbable walls (state 10), ledges (13), bars (`0x1402C7E00`), objects to
    // hang from, and water to swim in (17). Tried while KH2 has him falling, the stick held.
    // As KH1's jump step tries them (`0x1402ADA90`): not while rising fast (the top of the jump
    // counts), the stick held or not (KH1 passes its input either way; the user: ladders and
    // poles were hit or miss from a jump).
    if settings.grabs && d.character.airborne() && out.displacement[1] < 2.0 && matches!(out.state, StateName::Air | StateName::Jump) {
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
        if tilt > 0.0 {
            mv(actor, &[dir.x * 4.0, 0.0, dir.y * 4.0, 0.0], 0);
        }
        let (gx, gz) = if tilt > 0.0 { (dir.x, dir.y) } else { (0.0, 0.0) };
        let took = grab(base, actor, [gx, 0.0, gz, tilt]);
        if took.is_none() {
            wr(actor + 0x10, at);
            wr(actor + 0x70, state);
        }
        if let Some(what) = took {
            HANDOFF_FRAMES.store(0, Ordering::Relaxed);
            HANDOFF.store(true, Ordering::Relaxed);
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
    d.anim.field = !battle && !lingering;
    d.anim.update(&out, &d.character, [q[0], -q[1], q[2]]);
    keyblade(base, actor, d.anim.weapons_out(), &mut d.logged);
    GRIP.store(d.anim.weapons_out(), Ordering::Relaxed);
    GROUNDED.store(d.grounded && !d.character.airborne(), Ordering::Relaxed);
    d.strikes.strike(actor, &out, &d.anim, &mut d.character, &kh2.battle);
    // KH2's own sounds for the effect elements this frame starts.
    for e in &out.events {
        super::kh2sound::event(out.motion, *e);
        super::kh2fx::event(out.motion, *e);
    }
    super::kh2sound::tick();
    super::kh2fx::tick(out.motion);
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
        d.strikes.trail(actor, attacking && !out.hitboxes.is_empty());
    }
    let attacking = matches!(out.state, StateName::Attack);
    d.swing = attacking.then_some(out.motion);
    d.swing_frame = out.clip_frame;
    d.last_out = Some(out.clone());
    if let Ok(mut pz) = POSE.try_lock() {
        *pz = Some((d.anim.bones.clone(), Instant::now()));
    }
    // The Keyblade during attacks (the user: invisible while attacking): every 15 frames.
    static ATTACK_FRAMES: AtomicU32 = AtomicU32::new(0);
    if matches!(out.state, StateName::Attack) {
        let n = ATTACK_FRAMES.fetch_add(1, Ordering::Relaxed);
        if n % 15 == 7 && n < 15 * 40 {
            log(&format!("attacking (motion {}, frame {}): {}", out.motion, out.clip_frame, pose::weapon_report()));
        }
    }
    if d.last_state != Some(out.state) {
        if d.logged < 400 {
            d.logged += 1;
            log(&format!("KH2 has Sora: {:?} (motion {}), height above floor {above:.1}, KH1 slot {:?}, KH1 state {}, flags {:08x}; {}", out.state, out.motion, slot, rd::<i32>(actor + 0x70), rd::<u32>(actor), pose::weapon_report()));
        }
        d.last_state = Some(out.state);
    }
    ACTIVE_MS.store(now_ms().max(1), Ordering::Relaxed);
    true
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
    let tries: [(usize, &str); 5] = [(VINE_RVA, "vine / climb"), (LEDGE_RVA, "ledge"), (BAR_RVA, "bar"), (HANG_RVA, "object to hang from"), (SWIM_RVA, "water")];
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
/// slide, jumps and swings).
fn kh1_mode(state: i32) -> bool {
    state >= 30
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
    let own_mode = sora > 0x10000 && matches!(unsafe { rd::<i32>(sora + 0x70) }, 30 | 31);
    // Held still counts out of battle only: in battle it is a summon's or a command's moment
    // (the user: KH2's camera should stay through Simba's charge).
    let battle = unsafe { rd::<u32>(base + BATTLE_RVA) } & 1 != 0;
    let held = FROZEN_FRAMES.load(Ordering::Relaxed) >= 2 && !battle;
    event || own_mode || (active() && (SCRIPTED.load(Ordering::Relaxed) || held))
}

/// Whether KH1 may see Square: not while it has Sora back on the ground after one of its own
/// air moves (KH2 is about to take him; a roll pressed then is KH2's).
pub fn kh1_keeps_square() -> bool {
    if !HANDOFF.load(Ordering::Relaxed) {
        return true;
    }
    let base = BASE.load(Ordering::Relaxed);
    let sora = if base == 0 { 0 } else { (unsafe { rd::<u64>(base + PARTY_RVA) }) as usize };
    !(sora > 0x10000 && matches!(unsafe { rd::<i32>(sora + 0x70) }, 0 | 1))
}

/// KH2 has Sora this moment (not handed to KH1).
pub fn kh2_has() -> bool {
    active() && !HANDOFF.load(Ordering::Relaxed)
}

/// KH2 has Sora in the air this moment.
pub fn kh2_airborne() -> bool {
    active() && !HANDOFF.load(Ordering::Relaxed) && DRIVER.try_lock().ok().and_then(|d| d.as_ref().map(|d| d.last_state.is_some() && d.character.airborne())).unwrap_or(false)
}

/// KH1 takes Sora for one of its own actions (a command): it keeps him until it has him
/// standing in a movement motion again (see `HANDOFF`).
pub fn hand_to_kh1(why: &str) {
    HANDOFF_FRAMES.store(0, Ordering::Relaxed);
    HANDOFF.store(true, Ordering::Relaxed);
    log(&format!("KH1 takes him: {why}"));
}

/// While KH2 has Sora: his controller and its last frame (for `hurt`).
pub fn with_kh2<R>(f: impl FnOnce(&mut Character, &Output) -> R) -> Option<R> {
    if !ON.load(Ordering::Relaxed) || !active() || HANDOFF.load(Ordering::Relaxed) {
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
        let put_away: unsafe extern "system" fn(usize) = std::mem::transmute(base + PUT_AWAY_RVA);
        put_away(actor);
    }
    if *logged < 400 {
        *logged += 1;
        let colour = if cs > 0x10000 { rd::<[u8; 4]>(cs + 0xC) } else { [0; 4] };
        log(&format!("Keyblade {}: actor flags {flags:08x} -> {:08x}, colour {colour:02x?}", if out { "out" } else { "put away" }, rd::<u32>(actor)));
    }
}

pub unsafe fn install() {
    let base = sys_base();
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + PHYS_RVA, &PHYS_PROLOGUE, physics as *const () as usize, &ORIG_PHYS, "actor physics step");
    if hook::install(base + CONTROL_RVA, &CONTROL_PROLOGUE, control as *const () as usize, &ORIG_CONTROL, "player control step").is_some() {
        CONTROL_HOOKED.store(true, Ordering::Relaxed);
    }
}

unsafe fn sys_base() -> usize {
    super::sys::GetModuleHandleA(std::ptr::null()) as usize
}
