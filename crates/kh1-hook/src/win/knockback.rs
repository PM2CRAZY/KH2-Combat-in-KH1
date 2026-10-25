//! KH2's knock-back on enemies, through KH1's own enemy reactions (the tester, thread
//! kh2-knockback-on-enemies: "(b) but make it a toggle in the config menu / (a) / (a) / go ahead").
//! `docs/kh1-launch.md` §4.
//!
//! KH1's reactions stay the only system: KH1's launch (`0x1402B8530`), its flight in its air
//! state, its push and its air flinch run on the enemy as in KH1, with KH2's numbers in them for
//! the hits KH2's moves land. Hits of KH1's own moves keep KH1's numbers.
//!
//! **The enemy's weight** (question 1 (b), the Config menu's Combat > Tweaks > Enemy Weight row,
//! ini `enemy_weight`, on by default): KH1 scales its reactions by one stat of the enemy's, stats
//! `+0xC4` (the actor's `+0x6C` block; Checked (decomp)):
//! - its push (`0x1402B7E60`): btltbl's 80 × it, and its air flinch's drift (`0x1402B8B90`): 4 × it;
//! - its launch (`0x1402B8530`): a speed of 20 + (35 − 20) × it, its height 1.35 × that.
//!
//! On, KH2's numbers are multiplied the same way: pushes and drifts by the stat itself, launches
//! (both speeds) by KH1's launch speed for this enemy over KH1's fastest (35, the stat at 1).
//! **Assumption:** the stat's 1.0 is KH1's "full" weight (the launch's top speed, the push's full
//! 80), so an enemy at 1.0 takes KH2's numbers as they are. Off, every enemy takes KH2's.

use super::{log, pose};
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

/// btltbl's reaction block pointer (Steam `0x142D22D30`; `0x1402B8530` reads its u16 `+0x14` /
/// `+0x16`, the launch speed's two ends; Checked (decomp)).
const REACTION_BLOCK_STEAM: usize = 0x2D22D30;
const LAUNCH_LOW: f32 = 20.0;
const LAUNCH_HIGH: f32 = 35.0;
/// The enemy's reaction weight in its stats block (`+0x6C`, a handle).
const STATS: usize = 0x6C;
const WEIGHT: usize = 0xC4;

/// The Enemy Weight setting: 0 not read yet, 1 off, 2 on.
static WEIGHT_ON: AtomicU8 = AtomicU8::new(0);

pub fn weight() -> bool {
    match WEIGHT_ON.load(Ordering::Relaxed) {
        0 => {
            let w = super::settings::get().enemy_weight;
            WEIGHT_ON.store(1 + w as u8, Ordering::Relaxed);
            w
        }
        v => v == 2,
    }
}

/// KH1's weight stat of this enemy (stats `+0xC4`), if it reads as a number.
pub unsafe fn kh1_weight(actor: usize) -> Option<f32> {
    if actor < 0x10000 {
        return None;
    }
    let stats = pose::ptr(rd::<u32>(actor + STATS));
    if stats < 0x10000 || !super::crash::readable(stats + WEIGHT) {
        return None;
    }
    let w = rd::<f32>(stats + WEIGHT);
    let w = w.is_finite().then_some(w.clamp(0.0, 8.0));
    note_weight(actor, w);
    w
}

/// Each enemy's weight, logged the first time it is read (by its object), so the log tells what
/// KH1 gives its enemies.
unsafe fn note_weight(actor: usize, w: Option<f32>) {
    static SEEN: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());
    let obj = rd::<u32>(actor + 0x130);
    let Ok(mut seen) = SEEN.lock() else { return };
    if seen.contains(&obj) || seen.len() > 200 {
        return;
    }
    seen.push(obj);
    log(&format!("enemy weight: actor {actor:#x} (object {obj:#x}) KH1 stat +0xC4 = {w:?}"));
}

/// KH1's launch speed ends (btltbl, read from the game; 20 / 35 if it can't be).
unsafe fn launch_ends() -> (f32, f32) {
    if super::addr::Build::current() != super::addr::Build::Steam {
        return (LAUNCH_LOW, LAUNCH_HIGH);
    }
    let block = rd::<u64>(super::takeover::game_base() + REACTION_BLOCK_STEAM) as usize;
    if block < 0x10000 || !super::crash::readable(block + 0x18) {
        return (LAUNCH_LOW, LAUNCH_HIGH);
    }
    let (lo, hi) = (rd::<u16>(block + 0x14) as f32, rd::<u16>(block + 0x16) as f32);
    if hi > 0.0 { (lo, hi) } else { (LAUNCH_LOW, LAUNCH_HIGH) }
}

/// The factor on a KH2 launch's speeds for this enemy: KH1's launch speed for it over KH1's
/// fastest; 1 with Enemy Weight off.
pub unsafe fn launch_factor(actor: usize) -> f32 {
    if !weight() {
        return 1.0;
    }
    let Some(w) = kh1_weight(actor) else { return 1.0 };
    let (lo, hi) = launch_ends();
    let k = (lo + (hi - lo) * w) / hi;
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 20 {
        log(&format!("enemy weight on a KH2 launch: stat {w:.2}, KH1's launch {:.1} of {hi:.0}: x{k:.2}", lo + (hi - lo) * w));
    }
    k
}

// ---------------------------------------------------------------------------------------------
// The flight: KH2's blow rules in KH1's air state's step.
// ---------------------------------------------------------------------------------------------

/// KH1's arc fields (`docs/kh1-physics.md`): time to the apex and the apex's offset (y down).
const ARC_UP_T: usize = 0x3B8;
const ARC_H: usize = 0x3BC;
/// Its sideways velocity (vec4, w its length) and the frame's drop.
const VEL: usize = 0x3C0;
const DROP: usize = 0x3C4;
/// KH1's movement state (2: in the air).
const STATE: usize = 0x70;
/// A flight is let go after this many frames whatever happens (a stuck enemy).
const MAX_FRAMES: u32 = 600;

/// How an enemy KH2 knocked into the air moves.
#[derive(Clone, Copy, Debug)]
enum Fly {
    /// KH2's blow (`character_core::reaction::Blow`): one 30 Hz tick every second 60 Hz frame,
    /// half of it a frame (as KH2's controller and `character_core::npc` run it).
    Blow { blow: character_core::reaction::Blow, step: [f32; 3], phase: u32 },
}

#[derive(Clone, Copy, Debug)]
struct Flight {
    actor: usize,
    fly: Fly,
    /// The arc fields written for it: KH1 writing others (a new reaction of its own) ends ours.
    up_t: f32,
    h: f32,
    frames: u32,
}

static FLIGHTS: std::sync::Mutex<Vec<Flight>> = std::sync::Mutex::new(Vec::new());

/// `combat::launched`: KH1 launched `actor` for a KH2 blow and its arc is KH2's (`up_t`, `h` as
/// written): its flight from here is KH2's blow with these starting speeds (per 60 Hz frame,
/// `vel_h` along KH1's launch direction, `up` up).
pub unsafe fn fly_blow(actor: usize, vel_h: [f32; 2], up: f32, up_t: f32, h: f32) {
    let Some(p) = pose::kh2().map(|k| k.character.reaction.clone()) else { return };
    let blow = character_core::reaction::Blow::new(vel_h, up, &p);
    start(Flight { actor, fly: Fly::Blow { blow, step: [0.0; 3], phase: 0 }, up_t, h, frames: 0 });
}

fn start(f: Flight) {
    if let Ok(mut v) = FLIGHTS.lock() {
        v.retain(|g| g.actor != f.actor);
        if v.len() > 64 {
            v.remove(0);
        }
        v.push(f);
    }
}

/// KH1 gave `actor` a reaction of its own (not KH2's): its flight is KH1's again.
pub fn land_kh1(actor: usize) {
    if let Ok(mut v) = FLIGHTS.lock() {
        v.retain(|g| g.actor != actor);
    }
}

/// After KH1's air state's step (`shared::physics::air_step`, state 2's `0x1402AD510` through
/// KH1's state table, for every actor) on an enemy: KH2's flight replaces the step's displacement
/// (`disp`, y down) while the flight is ours. KH1's mover, collision, landing and get-up then run
/// on it as on KH1's own flight.
pub unsafe fn air_step(actor: usize, disp: *mut [f32; 4]) {
    let Ok(mut v) = FLIGHTS.try_lock() else { return };
    let Some(i) = v.iter().position(|g| g.actor == actor) else { return };
    let g = &mut v[i];
    let ours = rd::<i32>(actor + STATE) == 2 && rd::<f32>(actor + ARC_UP_T) == g.up_t && rd::<f32>(actor + ARC_H) == -g.h && g.frames < MAX_FRAMES;
    if !ours {
        static E: AtomicU32 = AtomicU32::new(0);
        if E.fetch_add(1, Ordering::Relaxed) < 30 {
            log(&format!("KH2 flight on {actor:#x} ended after {} frames (state {}, KH1's arc {})", g.frames, rd::<i32>(actor + STATE), if rd::<f32>(actor + ARC_UP_T) == g.up_t { "ours" } else { "its own" }));
        }
        v.remove(i);
        return;
    }
    let dt = super::shared::physics::dt();
    let n = (dt.round() as u32).clamp(1, 4);
    let Some(p) = pose::kh2().map(|k| k.character.reaction.clone()) else { return };
    let mut total = [0.0f32; 3];
    let vel_h = match &mut g.fly {
        Fly::Blow { blow, step, phase } => {
            for _ in 0..n {
                if *phase % 2 == 0 {
                    let d = blow.tick(2.0, &p);
                    *step = [d[0] / 2.0, d[1] / 2.0, d[2] / 2.0];
                }
                *phase += 1;
                for k in 0..3 {
                    total[k] += step[k];
                }
            }
            blow.vel_h
        }
    };
    g.frames += n;
    // (KH1 is y down: KH2's rise is a negative drop.)
    (*disp)[0] = total[0];
    (*disp)[1] = -total[1];
    (*disp)[2] = total[2];
    let s = vel_h[0].hypot(vel_h[1]);
    std::ptr::write_unaligned((actor + VEL) as *mut [f32; 4], [vel_h[0], 0.0, vel_h[1], s]);
    std::ptr::write_unaligned((actor + DROP) as *mut f32, -total[1]);
    static N: AtomicU32 = AtomicU32::new(0);
    if g.frames <= n || g.frames % 10 == 0 {
        if N.fetch_add(1, Ordering::Relaxed) < 60 {
            log(&format!("KH2 flight on {actor:#x}: frame {}, moved {:.1} sideways, {:.1} up", g.frames, total[0].hypot(total[2]), total[1]));
        }
    }
}
