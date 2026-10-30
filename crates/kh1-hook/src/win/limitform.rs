//! Limit Form's own settings (the owner, the main-branch port's round 12: "all of limits settings
//! will be under a sub menu called limit in the drive submenu"; Config > Drive > Limit). Notes:
//! `docs/kh1-forms-research-r12.md`.
//!
//! - **Cost** (t972, answer 2 (c): the owner's 3 bars only behind a toggle). KH2 (Checked
//!   (decomp)): Limit Form's command costs 4 bars, as Master's (`crate::drivegauge::cost`, form 3).
//!   KH2 (the default) = 4; 3 Bars = the owner's main-branch cost (s331). Only with Form Cost on
//!   the Drive gauge (MP mode keeps testing's 3 MP for every form).

use super::log;
use std::sync::atomic::{AtomicU8, Ordering};

/// Config > Drive > Limit > Cost: 0 not read yet, 1 KH2, 2 three bars.
static THREE: AtomicU8 = AtomicU8::new(0);

/// The 3-bar cost is on (the ini's `limit_cost = 3`; `kh2` by default).
pub fn three_bars() -> bool {
    match THREE.load(Ordering::Relaxed) {
        0 => {
            let v = super::settings::get().limit_cost_three;
            THREE.store(1 + v as u8, Ordering::Relaxed);
            v
        }
        v => v == 2,
    }
}

/// Switches Config > Drive > Limit > Cost, kept in the ini.
pub fn set_three_bars(v: bool) {
    THREE.store(1 + v as u8, Ordering::Relaxed);
    super::settings::save("limit_cost", if v { "3" } else { "kh2" });
    log!(Forms, "Config menu: Limit Form's cost {}", if v { "3 bars" } else { "KH2 (4 bars)" });
}

/// What form `form` (KH2's ids) costs in bars now: KH2's, with Limit's 3 when that row is on.
pub fn cost(form: u8) -> u8 {
    if form == crate::formlevel::form::LIMIT && three_bars() {
        3
    } else {
        crate::drivegauge::cost(form)
    }
}
