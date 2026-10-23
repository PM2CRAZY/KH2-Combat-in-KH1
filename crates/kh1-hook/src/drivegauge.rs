//! KH2's Drive gauge in KH1: the tables and rules the runtime (`win::gauge`) uses, kept apart so
//! they are easy to change and to test. The gauge itself is `character_core::drive::Drive`
//! (KH2's rules from the executable, `docs/drive-model.md`, "Drive gauge (Checked, exe)").
//!
//! - **Costs** (the user's, KH2's 03system `cmd` byte 0x1c): Valor 3, Wisdom 3, Limit 3 (KH2's
//!   4; the user's, s331), Master 4, Final 5; Anti 9, which empties the gauge.
//! - **Maximum** (the user's, s325): 3 bars, then +1 for each of six fights won (`MAX_TABLE`:
//!   Sabor, Riku 1, Chernabog, Sephiroth, Kurt Zisa, the Unknown), KH2's 9 with all six.
//!   Derived from the story flags (`crate::story::Flags`) each time, never stored.
//! - **Shown** once the first Drive Form is open (`shown`).
//! - **The save**: bars and points live in two bytes of KH1's save block (`BARS_OFF`,
//!   `POINTS_OFF`), the padding byte after the weapon in Sora's and Donald's party records.

use crate::story::{Flags, Form};

/// Bars at the start (the user: "Let's have the drive gauge start at 3").
pub const BASE_BARS: u8 = 3;

/// A fight that raises the maximum: its name (for the log), whether the story flags have it won
/// (`crate::story`, each flag's evidence there), and the bars it adds.
pub type Raise = (&'static str, fn(&Flags) -> bool, u8);

/// The fights that raise the maximum (the user: "Have the drive gauge level up at these points:
/// 1. defeating Sabor (boss) 2. defeating Riku I 3. Beating the game 4. defeating Sephiroth 5.
/// Defeating Kurt Zisa 6. Defeating Unknown"; then, for the third, "instead of beating the game,
/// make it after defeating Chernabog"). Change the rows here.
pub const MAX_TABLE: [Raise; 6] = [
    ("Sabor", |f| f.sabor, 1),
    ("Riku 1", |f| f.riku_1, 1),
    ("Chernabog", |f| f.chernabog, 1),
    ("Sephiroth", |f| f.sephiroth, 1),
    ("Kurt Zisa", |f| f.kurt_zisa, 1),
    ("the Unknown", |f| f.unknown, 1),
];

/// KH2's cap (each bonus-level Drive Gauge upgrade stops at 9).
pub const MAX_BARS: u8 = 9;

/// The Drive list's forms (Anti is the secret code's, not one that opens).
pub const FORMS: [Form; 5] = [Form::Valor, Form::Wisdom, Form::Limit, Form::Master, Form::Final];

/// Whether the gauge is shown: once the first Drive Form is open (the user: "don't show the drive
/// gauge until the first drive form is unlocked"), with `open` saying which are
/// (`story::unlocked`).
pub fn shown(open: impl Fn(Form) -> bool) -> bool {
    FORMS.iter().any(|&f| open(f))
}

/// The gauge's maximum with these story flags.
pub fn max_bars(flags: &Flags) -> u8 {
    let n = MAX_TABLE.iter().filter(|(_, won, _)| won(flags)).map(|(_, _, n)| *n as u32).sum::<u32>();
    (BASE_BARS as u32 + n).min(MAX_BARS as u32) as u8
}

/// The fights `flags` has won, as "Sabor +1, Riku 1 +1" (the log).
pub fn raised_by(flags: &Flags) -> Vec<String> {
    MAX_TABLE.iter().filter(|(_, won, _)| won(flags)).map(|(name, _, n)| format!("{name} +{n}")).collect()
}

/// A form's level (the user: "if sora has High Jump level 2, this means Valor Form has high jump
/// level 3 Which makes Valor Form level 3"): its growth ability's level in the form (base Sora's
/// learned level + 1, 4 = MAX: `win::form::form_abilities`), 1 at least. Its form gauge is KH2's
/// `(level + 3) × 600` frames (`character_core::drive::Drive::enter`): 4 bars at level 1, 7 at MAX.
pub fn form_level(growth: u8) -> u8 {
    growth.clamp(1, FORM_LEVEL_MAX)
}

/// The highest form level here (growth abilities stop at MAX, 4; KH2's forms reach 7).
pub const FORM_LEVEL_MAX: u8 = 4;

/// Form-gauge frames per bar (KH2's 600).
pub const BAR_FRAMES: f32 = 600.0;

/// The form gauge as the HUD draws it, from its frames left and its length: the draining bar's
/// share (1 while full) and KH2's number, the full bars left with the draining one not counted.
pub fn form_view(left: f32, max: f32) -> (f32, i32) {
    let left = left.max(0.0);
    let full = (left / BAR_FRAMES).floor();
    if left >= max {
        let bars = (max / BAR_FRAMES).round().max(1.0) as i32;
        return (1.0, (bars - 1).max(0));
    }
    ((left - full * BAR_FRAMES) / BAR_FRAMES, full as i32)
}

/// A Drive's cost in bars (KH2's `cmd` byte 0x1c; `None` is Anti Form, the secret code's).
pub fn cost(form: Option<Form>) -> u8 {
    match form {
        // (Limit 3, not KH2's 4: the user, "Make limit form only cost 3 bars to enter".)
        Some(Form::Valor) | Some(Form::Wisdom) | Some(Form::Limit) => 3,
        Some(Form::Master) => 4,
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
/// The bars byte's mark that the gauge has been written (the user: "By default, the drive gauge
/// should be full"): a byte without it (a new game, or a save from before s323) reads as a full
/// gauge.
pub const SET: u8 = 0x80;

/// The gauge as read from the save: never written (no `SET`) reads as full, garbage (an old
/// save's stray byte) as empty, and the bars are capped at `max` (a save with more forms than
/// the story now has).
pub fn from_save(bars: u8, points: u8, max: u8) -> (u8, u8) {
    if bars & SET == 0 {
        return (max, if max == 0 { 0 } else { 100 });
    }
    let bars = bars & !SET;
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
/// form, with Damage Drive equipped, `damage / max HP × 50` points per copy
/// (`crate::support::damage_drive_rate`; KH2 gives nothing without it), truncated.
pub const DAMAGE_DRIVE: f32 = crate::support::DAMAGE_DRIVE_RATE;

pub fn damage_points(damage: i32, max_hp: i32, copies: u8) -> i32 {
    if damage <= 0 || max_hp <= 0 || copies == 0 {
        return 0;
    }
    (damage.min(max_hp) as f32 / max_hp as f32 * crate::support::damage_drive_rate(copies)) as i32
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

    fn all() -> Flags {
        Flags { sabor: true, riku_1: true, chernabog: true, sephiroth: true, kurt_zisa: true, unknown: true, ..Default::default() }
    }

    #[test]
    fn maximum_grows_with_the_six_fights_to_nine() {
        assert_eq!(max_bars(&Flags::default()), 3, "the start");
        assert_eq!(max_bars(&Flags { sabor: true, ..Default::default() }), 4);
        assert_eq!(max_bars(&Flags { sabor: true, riku_1: true, ..Default::default() }), 5);
        assert_eq!(max_bars(&Flags { sabor: true, riku_1: true, chernabog: true, ..Default::default() }), 6);
        assert_eq!(max_bars(&Flags { unknown: true, ..Default::default() }), 4, "any order");
        assert_eq!(max_bars(&all()), 9);
        // Each row counts once, for its own flag only.
        for (i, (_, won, n)) in MAX_TABLE.iter().enumerate() {
            assert_eq!(*n, 1);
            let one = [
                Flags { sabor: true, ..Default::default() },
                Flags { riku_1: true, ..Default::default() },
                Flags { chernabog: true, ..Default::default() },
                Flags { sephiroth: true, ..Default::default() },
                Flags { kurt_zisa: true, ..Default::default() },
                Flags { unknown: true, ..Default::default() },
            ];
            for (j, f) in one.iter().enumerate() {
                assert_eq!(won(f), i == j, "row {i}, flag {j}");
            }
        }
        assert_eq!(BASE_BARS as usize + MAX_TABLE.len(), MAX_BARS as usize);
        assert_eq!(raised_by(&Flags { sabor: true, kurt_zisa: true, ..Default::default() }), ["Sabor +1", "Kurt Zisa +1"]);
    }

    #[test]
    fn forms_and_reflect_dont_raise_it() {
        let f = Flags { guard_armor: true, opposite_armor: true, pegasus_cup: true, anti_sora: true, riku_2: true, pot_centipede: true, hercules_cup: true, ice_titan: true, ..Default::default() };
        assert_eq!(max_bars(&f), 3);
    }

    #[test]
    fn shown_with_the_first_form() {
        assert!(!shown(|_| false));
        for form in FORMS {
            assert!(shown(|f| f == form));
        }
        // The maximum doesn't decide it: a save past Sabor with no form yet hides it.
        assert!(max_bars(&Flags { sabor: true, ..Default::default() }) > BASE_BARS && !shown(|_| false));
    }

    #[test]
    fn form_level_is_its_growth_ability() {
        use character_core::drive::{form, Drive};
        // Sora's learned growth level -> the form's (one above, MAX 4) -> the form level.
        assert_eq!(form_level(1), 1, "none learned: LV1 in the form");
        assert_eq!(form_level(3), 3, "the user's: High Jump LV2 learned, Valor's LV3, Valor Form level 3");
        assert_eq!(form_level(4), 4, "MAX");
        assert_eq!(form_level(0), 1, "never below 1");
        assert_eq!(form_level(9), 4);
        // KH2's (level + 3) × 600.
        for (level, frames) in [(1u8, 2400.0), (2, 3000.0), (3, 3600.0), (4, 4200.0)] {
            let mut d = Drive::new(9);
            d.enter(form::VALOR, form_level(level));
            assert_eq!(d.gauge.unwrap().max, frames);
        }
        // Anti keeps its own: 6 bars whatever the level.
        for level in [1u8, 4] {
            let mut d = Drive::new(9);
            d.enter(form::ANTI, level);
            assert_eq!(d.gauge.unwrap().max, 3600.0);
        }
    }

    #[test]
    fn the_hud_follows_the_form_gauge_length() {
        // Full: the bar full, the number the bars after it (KH2's count).
        assert_eq!(form_view(2400.0, 2400.0), (1.0, 3), "level 1: 4 bars");
        assert_eq!(form_view(3600.0, 3600.0), (1.0, 5), "level 3: 6 bars");
        assert_eq!(form_view(4200.0, 4200.0), (1.0, 6), "MAX: 7 bars");
        // Draining: the bar's share and the full bars under it.
        assert_eq!(form_view(3300.0, 3600.0), (0.5, 5));
        assert_eq!(form_view(1500.0, 3600.0), (0.5, 2));
        assert_eq!(form_view(300.0, 4200.0), (0.5, 0));
        assert_eq!(form_view(-5.0, 4200.0), (0.0, 0), "run out");
    }

    #[test]
    fn costs_are_kh2s() {
        assert_eq!(cost(Some(Form::Valor)), 3);
        assert_eq!(cost(Some(Form::Wisdom)), 3);
        assert_eq!(cost(Some(Form::Limit)), 3);
        assert_eq!(cost(Some(Form::Master)), 4);
        assert_eq!(cost(Some(Form::Final)), 5);
        assert_eq!(cost(None), 9);
        // Anti empties any gauge.
        let mut d = character_core::drive::Drive::new(max_bars(&all()));
        d.add_points(900);
        d.spend(cost(None));
        assert_eq!((d.bars, d.points), (0, 0));
    }

    #[test]
    fn save_bytes_are_checked() {
        assert_eq!(from_save(0, 0, 4), (4, 100), "never written: full");
        assert_eq!(from_save(2, 40, 5), (5, 100), "an unmarked (pre-s323) save: full");
        assert_eq!(from_save(SET, 0, 3), (0, 0));
        assert_eq!(from_save(SET | 2, 40, 5), (2, 40));
        assert_eq!(from_save(SET | 5, 100, 5), (5, 100));
        assert_eq!(from_save(SET | 7, 20, 5), (5, 100), "capped at the maximum");
        assert_eq!(from_save(SET | 10, 0, 9), (0, 0), "garbage");
        assert_eq!(from_save(SET | 1, 0xFF, 9), (0, 0), "garbage");
        assert_eq!(from_save(SET | 1, 100, 9), (1, 99), "100 marks a full gauge only");
        // The two bytes are distinct and inside the first two records.
        assert!(BARS_OFF < 0x74 && POINTS_OFF < 2 * 0x74 && POINTS_OFF != BARS_OFF);
    }

    #[test]
    fn damage_drive() {
        assert_eq!(damage_points(10, 100, 1), 5);
        assert_eq!(damage_points(1, 100, 1), 0, "truncated");
        assert_eq!(damage_points(500, 100, 1), 50, "at most the whole HP");
        assert_eq!(damage_points(0, 100, 1), 0);
        assert_eq!(damage_points(5, 0, 1), 0);
        // Per copy, and nothing without one (KH2's vanilla Sora).
        assert_eq!(damage_points(10, 100, 2), 10);
        assert_eq!(damage_points(3, 100, 2), 3, "the rate adds up before the truncation");
        assert_eq!(damage_points(50, 100, 0), 0);
        assert_eq!(DAMAGE_DRIVE, 50.0);
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
