//! Drive Form levels and EXP (the tester, what-is-still-missing after m14, answer 6 (b): "build
//! form levels into KH1"). KH1 has no form levels; this is KH2's rule (Steam KH2, Checked
//! (decomp)) kept in KH1's save (`win/formexp.rs` stores and feeds it):
//!
//! - **The add** `0x1403E8380 (player, n)`: the form's save record (`0x1403A1410`: level byte
//!   `+2`, EXP `+4`) gets `n` EXP; then, while the level is under the cap (2 + the forms Sora owns,
//!   counted over forms 1..5 by their items: `0x1403C3EF0` / `0x1403C4720`), the fmlv row of
//!   (form, level) is looked up (`bsearch` on `form << 4 | level`); EXP under its `exp` keeps
//!   what there is; otherwise the level goes up, the row's EXP is taken off, and the loop goes
//!   on. Reaching the cap (or a missing row) sets EXP to 0.
//! - **What a level gives** (the same function): when the new level's row has a higher growth
//!   level (fmlv byte 1, low nibble) than the row before it, the form's own growth ability goes up
//!   to it (the form's record's ability list, entry of KH2's growth table at `0x1405C9030`: per
//!   form none, LV1, LV2, LV3, MAX), and Sora's own growth ability goes to one level under it (the
//!   same table, one entry down, replaced in his list, or put in an empty slot when it goes from
//!   none to LV1); the row's ability (fmlv `+2`, Auto Valor, Combo Plus, ...) goes to Sora
//!   (`0x1403C3F00`).
//! - **Base Sora's side** (`sora_growth`, `rewards_due`): his growth level and the rows'
//!   abilities, given in KH1's own ability lists by `win/menu.rs` (the tester, after m15,
//!   answer 6: "(c)").
//!
//! The rows come from the player's own KH2 `00battle.bin` (`fmlv`), never from the repo.

/// KH2's form ids (fmlv's high nibble, the save records' order).
pub mod form {
    pub const VALOR: u8 = 1;
    pub const WISDOM: u8 = 2;
    pub const LIMIT: u8 = 3;
    pub const MASTER: u8 = 4;
    pub const FINAL: u8 = 5;
}

/// The highest form level (KH2: 2 + 5 forms owned).
pub const MAX_LEVEL: u8 = 7;

/// One fmlv row: the growth ability's level in the form at this level, the ability given on
/// reaching it, and the EXP from this level to the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub form: u8,
    pub level: u8,
    pub growth: u8,
    pub reward: u16,
    pub exp: i32,
}

/// KH2's fmlv rows (forms 1..5 used here; summons and Anti are rows too, never looked up).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    rows: Vec<Row>,
}

impl Table {
    pub fn new(rows: impl IntoIterator<Item = Row>) -> Table {
        let mut rows: Vec<Row> = rows.into_iter().collect();
        rows.sort_by_key(|r| (r.form, r.level));
        Table { rows }
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The row of `form` at `level` (KH2's `bsearch` on `form << 4 | level`).
    pub fn row(&self, form: u8, level: u8) -> Option<&Row> {
        self.rows.iter().find(|r| r.form == form && r.level == level)
    }

    /// The form's growth ability level at `level` (fmlv; 0 with no row).
    pub fn growth(&self, form: u8, level: u8) -> u8 {
        self.row(form, level).map_or(0, |r| r.growth)
    }
}

/// A form's level and EXP as kept in the save, and the last level whose fmlv ability Sora was
/// given (`rewarded`, ours: KH2 gives it in the same step as the level, `0x1403E8380`; here
/// KH1's menu gives it a moment later, `win/menu.rs`, so the save says how far it got).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    pub level: u8,
    pub exp: u32,
    pub rewarded: u8,
}

impl Default for Record {
    /// A form Sora has never been in is at level 1 with no EXP (KH2's records start there).
    fn default() -> Record {
        Record { level: 1, exp: 0, rewarded: 1 }
    }
}

/// The bits a packed record keeps its level in (3, levels 1..7), the rewarded level in (3) and
/// its EXP in (10, under 1024: KH2's largest step is 672, Valor LV6, Checked (files)). t700's
/// saves kept EXP in 13 bits, always under 672, so their rewarded bits read 0: the rewards of
/// every level they reached are given once.
const LEVEL_SHIFT: u16 = 13;
const REWARDED_SHIFT: u16 = 10;
const EXP_MASK: u16 = (1 << REWARDED_SHIFT) - 1;

impl Record {
    /// Two bytes in the save: level in the top 3 bits, the rewarded level, EXP below. 0 (a save
    /// from before form levels) reads as level 1.
    pub fn pack(self) -> u16 {
        ((self.level.clamp(1, MAX_LEVEL) as u16) << LEVEL_SHIFT) | ((self.rewarded.min(MAX_LEVEL) as u16) << REWARDED_SHIFT) | (self.exp.min(EXP_MASK as u32) as u16)
    }

    pub fn unpack(v: u16) -> Record {
        let level = (v >> LEVEL_SHIFT) as u8;
        if level == 0 {
            return Record::default();
        }
        Record { level, exp: (v & EXP_MASK) as u32, rewarded: ((v >> REWARDED_SHIFT) & 7) as u8 }
    }
}

/// Base Sora's own growth ability level from `form`'s level (KH2's `0x1403E8380`, Checked
/// (decomp + exe)): each time the form's growth level rises (fmlv byte 1) his own entry of the
/// growth table `0x1405C9030` goes from the level two under the form's new one to one under it
/// (from an empty slot to LV1), so with fmlv's rows rising one at a time his level is the form's
/// growth level less one: LV1 / LV2 / LV3 at form LV3 / 5 / 7, never MAX (the table's MAX entry is
/// only the form's). 0: none from this form.
pub fn sora_growth(t: &Table, form: u8, level: u8) -> u8 {
    t.growth(form, level).saturating_sub(1)
}

/// The fmlv abilities `rec` has reached but not been given (levels after `rewarded` up to its
/// level, in order): (level, KH2's ability id). Levels with none are left out.
pub fn rewards_due(t: &Table, form: u8, rec: &Record) -> Vec<(u8, u16)> {
    (rec.rewarded.max(1) + 1..=rec.level).filter_map(|l| t.row(form, l).filter(|r| r.reward != 0).map(|r| (l, r.reward))).collect()
}

/// What one level-up gave (KH2's `0x1403E8380`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelUp {
    pub level: u8,
    /// The form's growth ability rose to this level (fmlv's growth went up between the rows).
    pub form_growth: Option<u8>,
    /// Sora's own growth ability goes to this level (one under the form's), when it rose.
    pub sora_growth: Option<u8>,
    /// The fmlv row's ability (KH2's id; 0 none).
    pub reward: u16,
}

/// The level cap with `owned` forms (KH2: 2 + the forms 1..5 Sora owns).
pub fn cap(owned: u8) -> u8 {
    (2 + owned).min(MAX_LEVEL)
}

/// KH2's add: `n` EXP to `rec` of `form` with the level cap `cap`. Returns the levels gained, in
/// order.
pub fn add(rec: &mut Record, form: u8, n: u32, cap: u8, t: &Table) -> Vec<LevelUp> {
    let mut ups = Vec::new();
    let mut exp = rec.exp as i64 + n as i64;
    loop {
        if rec.level >= cap {
            break;
        }
        let Some(row) = t.row(form, rec.level) else { break };
        if exp < row.exp as i64 {
            rec.exp = exp.max(0) as u32;
            return ups;
        }
        let before = row.growth;
        rec.level += 1;
        let new = t.row(form, rec.level);
        let growth = new.map_or(before, |r| r.growth);
        let rose = growth > before;
        ups.push(LevelUp {
            level: rec.level,
            form_growth: rose.then_some(growth),
            sora_growth: rose.then(|| growth - 1).filter(|&g| g > 0),
            reward: new.map_or(0, |r| r.reward),
        });
        exp -= row.exp as i64;
    }
    rec.exp = 0;
    ups
}

/// KH2's object-id ranges for the defeat rules (`0x1403E8780`): Wisdom counts Heartless (ids
/// 1000..2011, not 1047..1059), Final Nobodies (3000..4014). Checked (decomp).
pub fn kh2_wisdom_counts(id: u32) -> bool {
    (1000..1000 + 0x3F4).contains(&id) && !(0x417..0x417 + 0xD).contains(&id)
}

pub fn kh2_final_counts(id: u32) -> bool {
    (3000..=3000 + 0x3F6).contains(&id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up table shaped as KH2's (growth 1, 1, 2, 2, 3, 3, 4; rewards on even levels).
    fn table() -> Table {
        let steps = [10, 20, 30, 40, 50, 60, 0];
        let growth = [1, 1, 2, 2, 3, 3, 4];
        Table::new((1..=7u8).map(|l| Row { form: form::VALOR, level: l, growth: growth[l as usize - 1], reward: if l % 2 == 0 { 100 + l as u16 } else { 0 }, exp: steps[l as usize - 1] }))
    }

    #[test]
    fn exp_under_the_step_is_kept() {
        let mut r = Record::default();
        assert!(add(&mut r, form::VALOR, 9, 7, &table()).is_empty());
        assert_eq!(r, Record { level: 1, exp: 9, rewarded: 1 });
    }

    #[test]
    fn levels_take_their_step_and_give_growth_and_rewards() {
        let mut r = Record::default();
        let ups = add(&mut r, form::VALOR, 35, 7, &table());
        assert_eq!(r, Record { level: 3, exp: 5, rewarded: 1 });
        assert_eq!(ups[0], LevelUp { level: 2, form_growth: None, sora_growth: None, reward: 102 });
        assert_eq!(ups[1], LevelUp { level: 3, form_growth: Some(2), sora_growth: Some(1), reward: 0 });
    }

    #[test]
    fn the_cap_stops_and_clears_exp() {
        let mut r = Record::default();
        let ups = add(&mut r, form::VALOR, 1000, 4, &table());
        assert_eq!(ups.len(), 3);
        assert_eq!(r, Record { level: 4, exp: 0, rewarded: 1 });
        assert!(add(&mut r, form::VALOR, 5, 4, &table()).is_empty());
        assert_eq!(r.exp, 0, "at the cap EXP is dropped");
        let ups = add(&mut r, form::VALOR, 1000, 7, &table());
        assert_eq!(r, Record { level: 7, exp: 0, rewarded: 1 });
        assert_eq!(ups.last().unwrap().form_growth, Some(4));
        assert_eq!(ups.last().unwrap().sora_growth, Some(3));
    }

    #[test]
    fn packs_into_two_bytes() {
        let r = Record { level: 5, exp: 671, rewarded: 3 };
        assert_eq!(Record::unpack(r.pack()), r);
        assert_eq!(Record::unpack(0), Record::default());
        // t700's packing (level << 13 | EXP): its rewards are all still to give.
        assert_eq!(Record::unpack((4 << 13) | 300), Record { level: 4, exp: 300, rewarded: 0 });
    }

    #[test]
    fn sora_grows_one_under_the_form() {
        let t = table();
        let got: Vec<u8> = (1..=7).map(|l| sora_growth(&t, form::VALOR, l)).collect();
        assert_eq!(got, [0, 0, 1, 1, 2, 2, 3]);
    }

    #[test]
    fn rewards_due_once() {
        let t = table();
        let rec = Record { level: 6, exp: 0, rewarded: 0 };
        assert_eq!(rewards_due(&t, form::VALOR, &rec), [(2, 102), (4, 104), (6, 106)]);
        let rec = Record { level: 6, exp: 0, rewarded: 4 };
        assert_eq!(rewards_due(&t, form::VALOR, &rec), [(6, 106)]);
        assert!(rewards_due(&t, form::VALOR, &Record::default()).is_empty());
    }

    #[test]
    fn kh2_id_ranges() {
        assert!(kh2_wisdom_counts(1000) && kh2_wisdom_counts(2011) && !kh2_wisdom_counts(2012));
        assert!(!kh2_wisdom_counts(1047) && !kh2_wisdom_counts(1059) && kh2_wisdom_counts(1060));
        assert!(kh2_final_counts(3000) && kh2_final_counts(4014) && !kh2_final_counts(4015));
        assert_eq!(cap(2), 4);
        assert_eq!(cap(5), 7);
    }
}
