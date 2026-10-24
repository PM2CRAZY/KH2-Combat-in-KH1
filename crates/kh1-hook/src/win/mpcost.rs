//! KH2's MP costs scaled onto KH1's MP (the tester, thread kh2-magic-and-reflect-native, answer
//! 6 (b): "scale KH2's costs onto KH1's MP"). Base Sora's KH2 spells only; Drive Forms keep
//! their own (1 MP a spell).
//!
//! - KH2's costs (03system `cmd`, Checked (files)): Fire 12, Blizzard 15, Thunder 18 at every
//!   level (commands 49 / 119 / 120, 51 / 121 / 122, 50 / 123 / 124), Cure 255 = all of it (52 /
//!   125 / 126), Reflect 10 (177..179), out of KH2 Sora's MP of 100 (00battle `plrp`, every row of
//!   character 1, Checked (files)).
//! - KH1's MP (Checked (decomp), Steam): Sora's stats `+0x44` whole bars of `+0x48`, and the
//!   charge part `+0xBE` (u16), which fills toward the next bar: bar `n + 1` comes at
//!   `(n + 1) × 30` (`0x1402A4CB0`). KH1's payment `0x1402A5B60 (actor, cost)`: from 100 up
//!   `cost / 100` whole bars; below 100 `cost` charge units, and when they run out the top bar
//!   breaks into its own worth (bar `n` gives `n × 30`). KH1's test `0x1402A4C10` counts
//!   `Σ k × 30 + charge` for costs below 100. So KH1's MP is a pool of units in which bar `k` is
//!   worth `k × 30`, and the charge part is a share of the next bar.
//! - The scale: a spell costs `KH2 cost / 100` of Sora's KH1 maximum, in bars, taken from the
//!   top down (the charge part first, at the next bar's worth, then each bar at its own), paid in
//!   units through KH1's own payment in steps of at most 30 (the smallest bar's worth, so each
//!   step breaks at most one bar, as KH1's payment does). Cure takes everything and needs
//!   something to take (KH2: all of the MP).

use super::pose;
use std::ptr::read_unaligned as rd;

/// KH2 Sora's maximum MP (00battle `plrp`, Checked (files)).
const KH2_MAX_MP: f32 = 100.0;
/// A charge unit's worth per bar index (`0x1402A4CB0`, Checked (decomp)).
const UNITS_PER_INDEX: i32 = 30;

/// A KH2 cost: a share of the pool, or all of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cost {
    Part(u8),
    All,
}

/// KH2's cost of a spell (KH1's numbering: Fire 0, Blizzard 1, Thunder 2, Cure 3; Reflect 7).
pub fn kh2_cost(spell: u8) -> Option<Cost> {
    Some(match spell {
        0 => Cost::Part(12),
        1 => Cost::Part(15),
        2 => Cost::Part(18),
        3 => Cost::All,
        7 => Cost::Part(10),
        _ => return None,
    })
}

/// Sora's MP now: (bars, maximum, charge units).
unsafe fn mp(sora: usize) -> Option<(i32, i32, i32)> {
    if sora < 0x10000 {
        return None;
    }
    let stats = pose::ptr(rd((sora + 0x6C) as *const u32));
    if stats < 0x10000 {
        return None;
    }
    Some((rd((stats + 0x44) as *const i32), rd((stats + 0x48) as *const i32), rd((stats + 0xBE) as *const u16) as i32))
}

/// The units a cost takes from (bars `n`, maximum `max`, charge `y`), or None if it can't be paid.
pub fn units_of(cost: Cost, n: i32, max: i32, y: i32) -> Option<i32> {
    if n < 0 || max <= 0 {
        return None;
    }
    let total = (1..=n).map(|k| k * UNITS_PER_INDEX).sum::<i32>() + y.max(0);
    let mut left = match cost {
        Cost::All => return (total > 0).then_some(total),
        Cost::Part(c) => c as f32 / KH2_MAX_MP * max as f32,
    };
    let mut u = 0.0f32;
    // The charge part, at the next bar's worth.
    let next = ((n + 1) * UNITS_PER_INDEX) as f32;
    let take = left.min(y.max(0) as f32 / next);
    u += take * next;
    left -= take;
    for k in (1..=n).rev() {
        if left <= 1e-4 {
            break;
        }
        let take = left.min(1.0);
        u += take * (k * UNITS_PER_INDEX) as f32;
        left -= take;
    }
    if left > 1e-4 {
        return None;
    }
    Some((u.ceil() as i32).clamp(1, total.max(1)))
}

/// The units `spell`'s KH2 cost takes from Sora now, if he can pay it.
pub unsafe fn units(sora: usize, spell: u8) -> Option<i32> {
    let (n, max, y) = mp(sora)?;
    units_of(kh2_cost(spell)?, n, max, y)
}

/// Pays `units` through KH1's own payment, at most 30 at a time.
pub unsafe fn pay_units(sora: usize, units: i32) {
    let mut left = units.max(0);
    while left > 0 {
        let step = left.min(UNITS_PER_INDEX);
        super::reflect::pay_mp(sora, step);
        left -= step;
    }
}

/// Pays `spell`'s KH2 cost if Sora can; returns the units paid.
pub unsafe fn pay(sora: usize, spell: u8) -> Option<i32> {
    let u = units(sora, spell)?;
    let before = mp(sora);
    pay_units(sora, u);
    super::log(&format!("KH2 MP cost of spell {spell} ({:?}): {u} of KH1's units paid, MP {before:?} -> {:?}", kh2_cost(spell), mp(sora)));
    Some(u)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fire_on_two_bars() {
        // 12% of 2 bars = 0.24 bar, from the top bar (worth 60): 14.4 -> 15 units.
        assert_eq!(units_of(Cost::Part(12), 2, 2, 0), Some(15));
        // 12% of 3 bars = 0.36 bar: the charge toward bar 3 (30 of its 90, a third of a bar),
        // then 0.0267 of bar 2 (worth 60): 30 + 1.6 -> 32 units.
        assert_eq!(units_of(Cost::Part(12), 2, 3, 30), Some(32));
    }

    #[test]
    fn not_enough() {
        assert_eq!(units_of(Cost::Part(18), 0, 5, 10), None);
        assert_eq!(units_of(Cost::All, 0, 5, 0), None);
    }

    #[test]
    fn cure_takes_everything() {
        assert_eq!(units_of(Cost::All, 3, 5, 7), Some(30 + 60 + 90 + 7));
    }
}
