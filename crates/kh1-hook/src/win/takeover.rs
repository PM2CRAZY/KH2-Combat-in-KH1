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

use super::{combat, hook, log, pose};
use character_core::{Abilities, Button, Character, Env, Input, StateName};
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
/// How far below the feet the floor may drop and still be stepped down onto (KH2's floor ray
/// reaches 24 below).
const STEP_DOWN: f32 = 24.0;

pub const CIRCLE: u16 = 0x2000;
pub const SQUARE: u16 = 0x8000;
#[allow(dead_code)]
pub const CROSS: u16 = 0x4000;

pub static ON: AtomicBool = AtomicBool::new(true);
/// KH1's area script is moving Sora (`pose` shows KH1's own animation then).
pub static SCRIPTED: AtomicBool = AtomicBool::new(false);
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
    let f: PhysFn = std::mem::transmute(ORIG_PHYS.load(Ordering::Acquire));
    f(actor);
}

fn release(why: &str) {
    if let Ok(mut d) = DRIVER.try_lock() {
        if let Some(dr) = d.as_mut() {
            if dr.logged < 400 && dr.last_state.is_some() {
                dr.logged += 1;
                log(&format!("KH1 has Sora ({why})"));
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
    let slot = pose::sora_slot();
    let airborne = DRIVER.try_lock().ok().and_then(|d| d.as_ref().map(|d| d.character.airborne())).unwrap_or(false);
    if !matches!(slot, Some(0..=7)) && !airborne {
        release(&format!("KH1's own action, slot {}", slot.map_or("?".into(), |s| s.to_string())));
        return false;
    }
    let Some(kh2) = pose::kh2() else { return false };
    let Ok(mut g) = DRIVER.try_lock() else { return false };
    let d = g.get_or_insert_with(|| {
        let mut character = kh2.character.clone();
        // Testing: every movement ability at MAX, guard and aerial recovery (KH1's level-ups
        // hand them out later).
        character.abilities = Abilities { guard: true, aerial_recovery: true, combo_master: true, ..Abilities::max_growth() };
        log("KH2's controller ready for Sora");
        Driver { character, anim: RigAnim::new(kh2.rig.clone()), held: (true, true), grounded: true, last_state: None, logged: 0, strikes: combat::Strikes::default() }
    });
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
    let jump = buttons & CIRCLE != 0;
    let action = buttons & SQUARE != 0;
    // KH1's Attack command (Cross with Attack chosen in the menu) is KH2's attack button.
    let attack = combat::take_press();
    let input = Input {
        stick,
        jump: Button { pressed: jump && !d.held.0, held: jump },
        attack: Button { pressed: attack, held: attack },
        action: Button { pressed: action && !d.held.1, held: action },
        lock: Button::default(),
    };
    d.held = (jump, action);

    // Where he is, KH2's way up (KH1's world is y-down).
    let p = rd::<[f32; 3]>(actor + 0x10);
    let pos = [p[0], -p[1], p[2]];
    let above = rd::<f32>(actor + 0x3D4);
    let targets = d.strikes.look(actor);
    let env = Env { pos, grounded: d.grounded, height_above_ground: above.max(0.0), targets: &targets, ledges: &[] };
    let out = d.character.step(&input, &env);

    // KH2's displacement: the sideways part through KH1's mover (walls, slopes, steps up); the
    // height is KH2's while he is in the air (KH1's mover snaps a grounded actor back onto the
    // floor every frame, which undid a jump's first small rises: the user, stage 8).
    let air = d.character.airborne();
    let rising = out.displacement[1] > 0.0;
    let up_before = -p[1];
    wr(actor + 0x490, rd::<[f32; 4]>(actor + 0x10));
    let mv: MoveFn = std::mem::transmute(base + MOVE_RVA);
    mv(actor, &[out.displacement[0], 0.0, out.displacement[2], 0.0], 0);
    // The floor under where he is now (KH1 y-down: floor point `+0x320`).
    let floor_up = -rd::<f32>(actor + 0x324);
    let walked_up = -rd::<f32>(actor + 0x14);
    if air || rising {
        let y = up_before + out.displacement[1];
        if y <= floor_up && !rising {
            wr(actor + 0x14, -floor_up);
            d.grounded = true;
        } else {
            wr(actor + 0x14, -y.max(floor_up));
            d.grounded = false;
        }
    } else {
        // On the ground: where the mover walked him, stepping down onto a floor up to 24 below
        // (KH2's floor ray), else walking off the edge into a fall.
        let drop = walked_up - floor_up;
        if drop <= STEP_DOWN {
            wr(actor + 0x14, -floor_up);
            d.grounded = true;
        } else {
            d.grounded = false;
        }
    }
    // Height above the floor for KH2's next step.
    wr(actor + 0x3D4, (-rd::<f32>(actor + 0x14)) - floor_up);
    // Facing.
    let f = out.facing;
    wr(actor + 0x30, [f.sin(), 0.0, f.cos(), f]);
    wr(actor + 0x150, f);

    let q = rd::<[f32; 3]>(actor + 0x10);
    d.anim.field = matches!(slot, Some(0 | 2 | 5));
    d.anim.update(&out, &d.character, [q[0], -q[1], q[2]]);
    d.strikes.strike(actor, &out, &d.anim, &mut d.character, &kh2.battle);
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
            log(&format!("KH2 has Sora: {:?} (motion {}), height above floor {above:.1}, KH1 slot {:?}, flags {:08x}; {}", out.state, out.motion, slot, rd::<u32>(actor), pose::weapon_report()));
        }
        d.last_state = Some(out.state);
    }
    ACTIVE_MS.store(now_ms().max(1), Ordering::Relaxed);
    true
}

pub unsafe fn install() {
    let base = sys_base();
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + PHYS_RVA, &PHYS_PROLOGUE, physics as *const () as usize, &ORIG_PHYS, "actor physics step");
}

unsafe fn sys_base() -> usize {
    super::sys::GetModuleHandleA(std::ptr::null()) as usize
}
