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
