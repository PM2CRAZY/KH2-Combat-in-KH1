//! KH1's own combos for base Sora (`styles`: Ground Combo / Aerial Combo set to KH1; the user:
//! "toggle what sora uses for his ground and aerial moves"). The Attack command, instead of
//! KH2's attack button, goes to KH1 as its other commands do (`combat::command`'s hand-off):
//! KH1's command step picks its own ground or air row, KH1 runs its combo (its rows, motions,
//! hitboxes, finishers and abilities), the Attack presses meanwhile stay KH1's (they chain its
//! combo), and KH2 takes him back once KH1 lets him act (idle, moving, a jump, Square, his fall
//! after an air combo). Which style is by where he is when the press comes. Drive Forms keep
//! KH2's combos (`styles::kh1_now`). `docs/kh1-move-styles.md`.

use super::styles::{self, Kind};
use super::takeover;
use std::sync::atomic::{AtomicBool, Ordering};

/// The hand-off now running is a KH1 combo's (set by `started`, cleared by any other hand-off).
static COMBO: AtomicBool = AtomicBool::new(false);
/// Inside our call of KH1's command step for an Attack it will run (`combat::command`): its
/// attack row is KH1's (`combat::exec`).
static STARTING: AtomicBool = AtomicBool::new(false);

/// KH1's combo for an Attack where he is (in the air or on the ground).
pub fn kh1_here(air: bool) -> bool {
    styles::kh1_now(if air { Kind::AirCombo } else { Kind::GroundCombo })
}

/// KH1 has him for one of its combos now.
pub fn active() -> bool {
    COMBO.load(Ordering::Relaxed) && takeover::handing_off() && takeover::FROM_COMMAND.load(Ordering::Relaxed)
}

/// Any other hand-off to KH1 (`takeover::hand_to_kh1`).
pub fn clear() {
    COMBO.store(false, Ordering::Relaxed);
}

/// Around KH1's command step for an Attack it is to run.
pub fn starting(on: bool) {
    STARTING.store(on, Ordering::Relaxed);
}

/// KH1 took him for its combo (after `hand_to_kh1`).
pub fn started() {
    COMBO.store(true, Ordering::Relaxed);
}

/// An Attack row KH1's command step runs on Sora (`combat::exec`) is KH1's own: the press that
/// starts a KH1 combo, or one during it (its next swing, or a new combo once he is free) where
/// the style is KH1's (KH1's air state 1 / 2 for the air one).
pub unsafe fn kh1_row(actor: usize) -> bool {
    if STARTING.load(Ordering::Relaxed) {
        return true;
    }
    active() && kh1_here(matches!(std::ptr::read_unaligned((actor + 0x70) as *const i32), 1 | 2))
}
