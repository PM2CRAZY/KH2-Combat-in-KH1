//! KH2's Drive gauge in KH1: the tables and rules the runtime (`win::gauge`) uses, kept apart so
//! they are easy to change and to test. The gauge itself is `character_core::drive::Drive`
//! (KH2's rules from the executable, `docs/drive-model.md`, "Drive gauge (Checked, exe)").
//!
//! - **Costs** (the user's, KH2's 03system `cmd` byte 0x1c): Valor 3, Wisdom 3, Limit 4, Master
//!   4, Final 5; Anti 9, which empties the gauge.
//! - **Maximum** (the user's): 3 bars, then more for each Drive Form unlocked (`MAX_TABLE`),
//!   KH2's 9 with every form open. Derived from the story flags each time, never stored.
//! - **The save**: bars and points live in two bytes of KH1's save block (`BARS_OFF`,
//!   `POINTS_OFF`), the padding byte after the weapon in Sora's and Donald's party records.

use crate::story::Form;

/// Bars before any form is open.
pub const BASE_BARS: u8 = 3;

/// Bars each unlocked form adds (the user: "3, then +1 for each of Valor, Wisdom, Limit and
/// Master, and +2 for Final, reaching KH2's 9 when every form is open"). Change the numbers here.
pub const MAX_TABLE: [(Form, u8); 5] = [(Form::Valor, 1), (Form::Wisdom, 1), (Form::Limit, 1), (Form::Master, 1), (Form::Final, 2)];

/// KH2's cap (each bonus-level Drive Gauge upgrade stops at 9).
pub const MAX_BARS: u8 = 9;

/// The gauge's maximum with the forms `open` says are unlocked.
pub fn max_bars(open: impl Fn(Form) -> bool) -> u8 {
    let n = MAX_TABLE.iter().filter(|(f, _)| open(*f)).map(|(_, n)| *n as u32).sum::<u32>();
    (BASE_BARS as u32 + n).min(MAX_BARS as u32) as u8
}

/// A Drive's cost in bars (KH2's `cmd` byte 0x1c; `None` is Anti Form, the secret code's).
pub fn cost(form: Option<Form>) -> u8 {
    match form {
        Some(Form::Valor) | Some(Form::Wisdom) => 3,
        Some(Form::Limit) | Some(Form::Master) => 4,
        Some(Form::Final) => 5,
        None => ANTI_COST,
    }
}

/// Anti Form's cost (KH2's 0x000d row): 9, more than any gauge holds, so it empties it.
pub const ANTI_COST: u8 = 9;
/// Full bars Anti Form wants before it starts (ours: KH2's Anti only comes out of a Drive the gauge
/// could pay for, and its cheapest is 3).
pub const ANTI_NEEDS: u8 = 3;

/// The save bytes (offsets in KH1's save block, `menu::save_block`): the full bars and the points
/// in the current bar (0..=100, 100 the full marker). Each is the byte after a party record's
/// weapon (record `+0x37`, between the weapon byte `+0x36` and the u16 at `+0x38`): KH1 reads and
/// writes the weapon as a byte and `+0x38` as a u16, and nothing touches `+0x37`
/// (`docs/drive-model.md`, "The Drive gauge in the save"). Sora's record holds the bars, Donald's
/// (record 1, `0x74` on) the points.
pub const BARS_OFF: usize = 0x37;
pub const POINTS_OFF: usize = 0x74 + 0x37;

/// The gauge as read from the save: garbage (an old save's stray byte) reads as empty, and the
/// bars are capped at `max` (a save with more forms than the story now has).
pub fn from_save(bars: u8, points: u8, max: u8) -> (u8, u8) {
    if bars > MAX_BARS || points > 100 {
        return (0, 0);
    }
    if bars >= max {
        return (max, if max == 0 { 0 } else { 100 });
    }
    // (100 marks a full gauge only.)
    (bars, points.min(99))
}

/// Drive from damage Sora takes (KH2's HP-change rule, `Drive gauge` in the docs): out of a
/// form, `damage / max HP × 50` points (KH2 gives `× 0` without Damage Drive and `× 50` with one
/// copy; here every Sora has it once, the user: "damage Sora takes" fills it), truncated.
pub const DAMAGE_DRIVE: f32 = 50.0;

pub fn damage_points(damage: i32, max_hp: i32) -> i32 {
    if damage <= 0 || max_hp <= 0 {
        return 0;
    }
    (damage.min(max_hp) as f32 / max_hp as f32 * DAMAGE_DRIVE) as i32
}

/// The drive value of a hit of KH1's own (its combo, its limits, its spells), which has no ATKP
/// row: KH2's for the like (00battle `atkp` byte 0x29): Sora's ground combo swings are 3 (ids
/// 127, 128, 130, 134) and their finishers 4; his level-1 spells 5 (Thunder's 249 / 250) and 6
/// (Blizzard's 248). KH1's physical hits count as a swing (3), its spells as Thunder's (5).
pub const KH1_PHYSICAL: u8 = 3;
pub const KH1_SPELL: u8 = 5;

/// KH2's Drive orbs (its prize table at `0x1405C7590`): a small orb is 1 point, a big one 3.
pub const SMALL_ORB: u8 = 1;
pub const BIG_ORB: u8 = 3;

/// Drive orbs a KH1 enemy drops as it goes down (alongside its own HP / MP orbs and munny), drawn
/// as KH2's normal enemies' prize rows are (00battle `przt`, the 87 rows with an item drop): 62
/// drop none, 10 four small orbs, 10 two big ones, and the rest more (here six small and two big).
/// Each entry is (weight, small, big).
pub const DROPS: [(u32, u8, u8); 4] = [(62, 0, 0), (10, 4, 0), (10, 0, 2), (5, 6, 2)];

/// The drop for a roll in `0..DROPS`'s total weight.
pub fn drop_for(roll: u32) -> (u8, u8) {
    let total: u32 = DROPS.iter().map(|d| d.0).sum();
    let mut r = roll % total;
    for (w, s, b) in DROPS {
        if r < w {
            return (s, b);
        }
        r -= w;
    }
    (0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maximum_grows_with_the_forms_to_nine() {
        assert_eq!(max_bars(|_| false), 3);
        assert_eq!(max_bars(|f| f == Form::Valor), 4);
        assert_eq!(max_bars(|f| matches!(f, Form::Valor | Form::Wisdom | Form::Limit)), 6);
        assert_eq!(max_bars(|f| f != Form::Final), 7);
        assert_eq!(max_bars(|f| f == Form::Final), 5);
        assert_eq!(max_bars(|_| true), 9);
    }

    #[test]
    fn costs_are_kh2s() {
        assert_eq!(cost(Some(Form::Valor)), 3);
        assert_eq!(cost(Some(Form::Wisdom)), 3);
        assert_eq!(cost(Some(Form::Limit)), 4);
        assert_eq!(cost(Some(Form::Master)), 4);
        assert_eq!(cost(Some(Form::Final)), 5);
        assert_eq!(cost(None), 9);
        // Anti empties any gauge.
        let mut d = character_core::drive::Drive::new(max_bars(|_| true));
        d.add_points(900);
        d.spend(cost(None));
        assert_eq!((d.bars, d.points), (0, 0));
    }

    #[test]
    fn save_bytes_are_checked() {
        assert_eq!(from_save(0, 0, 3), (0, 0));
        assert_eq!(from_save(2, 40, 5), (2, 40));
        assert_eq!(from_save(5, 100, 5), (5, 100));
        assert_eq!(from_save(7, 20, 5), (5, 100), "capped at the maximum");
        assert_eq!(from_save(10, 0, 9), (0, 0), "garbage");
        assert_eq!(from_save(1, 0xFF, 9), (0, 0), "garbage");
        assert_eq!(from_save(1, 100, 9), (1, 99), "100 marks a full gauge only");
        // The two bytes are distinct and inside the first two records.
        assert!(BARS_OFF < 0x74 && POINTS_OFF < 2 * 0x74 && POINTS_OFF != BARS_OFF);
    }

    #[test]
    fn damage_drive() {
        assert_eq!(damage_points(10, 100), 5);
        assert_eq!(damage_points(1, 100), 0, "truncated");
        assert_eq!(damage_points(500, 100), 50, "at most the whole HP");
        assert_eq!(damage_points(0, 100), 0);
        assert_eq!(damage_points(5, 0), 0);
    }

    #[test]
    fn drops_follow_the_weights() {
        let total: u32 = DROPS.iter().map(|d| d.0).sum();
        let mut none = 0;
        for r in 0..total {
            if drop_for(r) == (0, 0) {
                none += 1;
            }
        }
        assert_eq!(none, 62);
        assert_eq!(drop_for(62), (4, 0));
        assert_eq!(drop_for(72), (0, 2));
        assert_eq!(drop_for(total - 1), (6, 2));
        assert_eq!(drop_for(total), (0, 0), "wraps");
    }
}
