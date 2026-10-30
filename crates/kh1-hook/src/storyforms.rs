//! When each Drive Form opens with the Config menu's Progression > Story Unlocks on (the owner's
//! own design from the main branch, kept behind that row: "a config ... with it on or off, so we
//! can preserve the changes but work on it later, same goes for the progression changes in the
//! main branch"). KH2 itself gives a form as an item at its story's points; KH1 has no forms, so
//! these are the owner's KH1 fights (main s279, s307: "dont use the archipelago work just make it
//! yourself"). Notes: `docs/kh1-drive-forms-native.md` §12, `docs/main-port-inventory.md` round 1.
//!
//! - First, second and third form: after Guard Armor, Opposite Armor and the Pegasus Cup, in an
//!   order by the Dive to the Heart's choice (Sword: Valor, Limit, Wisdom; Shield: Limit, Wisdom,
//!   Valor; Rod: Wisdom, Valor, Limit). Each fight stands alone (one won out of order opens its own
//!   form).
//! - Master after Anti-Sora, Final after Ansem-Riku (Riku 2).
//!
//! The flags (all in Sora's save; main's findings from KH1's room scripts and save snapshots,
//! **Assumption** on testing until rechecked; the offsets are Checked (decomp) on Steam, see §12):
//! Traverse Town's story byte `+0x1504` >= 0x2B (Guard Armor); journal entry 0x9F (Opposite Armor,
//! `+0x16E3`, high bit first); the Pegasus Cup's state `+0xF37` >= 1; Neverland's story byte
//! `+0x150D` >= 0x32 (Anti-Sora); Hollow Bastion's `+0x150E` >= 0x82 (Riku 2). The Dive choice is
//! `+0x48C`: 0 Sword, 1 Shield, 2 Rod (Assumption: the dream weapons' order), anything else read as
//! the Sword's.

use crate::formlevel::{form, owned};

pub const PROGRESS: usize = 0x1504;
pub const TRAVERSE_TOWN: usize = 0;
pub const NEVERLAND: usize = 9;
pub const HOLLOW_BASTION: usize = 10;
pub const GUARD_ARMOR_TT: u8 = 0x2B;
pub const ANTI_SORA_NL: u8 = 0x32;
pub const RIKU_2_HB: u8 = 0x82;
pub const JOURNAL: usize = 0x16E3;
pub const OPPOSITE_ARMOR_ENTRY: u8 = 0x9F;
pub const PEGASUS_CUP: usize = 0xF37;
pub const DIVE_CHOICE: usize = 0x48C;
/// The save bytes read, from `PROGRESS` on (the furthest is the journal's 0x9F byte).
pub const SPAN_END: usize = JOURNAL + 0x20;

/// The six fights of the owner's Drive gauge growth (main s325, s326: "Let's have the drive gauge start
/// at 3. Have the drive gauge level up at these points: ... Sabor ... Riku I ... (Chernabog) ...
/// Sephiroth ... Kurt Zisa ... Unknown"), behind Progression > Drive Gauge Growth (t935). Main's flags
/// (Assumption on testing): Deep Jungle's story byte `+0x1505` >= 0x42 (Sabor), Hollow Bastion's
/// `+0x150E` >= 0x32 (Riku 1), End of the World's `+0x150F` >= 0x33 (Chernabog, testing's
/// `growth::CHERNABOG` too), journal entry 0xA5 (Sephiroth), Agrabah's `+0x1508` >= 0x96 (Kurt Zisa),
/// journal entry 0xA9 with the Unknown's prize 0x3C owned (`+0x499 + 0x3C`).
pub fn gauge_fights(save: &[u8]) -> u8 {
    if save.len() < SPAN_END {
        return 0;
    }
    let journal = |e: u8| save[JOURNAL + (e >> 3) as usize] & (0x80 >> (e & 7)) != 0;
    let p = |w: usize| save[PROGRESS + w];
    [p(1) >= 0x42, p(HOLLOW_BASTION) >= 0x32, p(11) >= 0x33, journal(0xA5), p(4) >= 0x96, journal(0xA9) && save[0x499 + 0x3C] >= 1].into_iter().filter(|b| *b).count() as u8
}

/// The fights the table reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    pub guard_armor: bool,
    pub opposite_armor: bool,
    pub pegasus_cup: bool,
    pub anti_sora: bool,
    pub riku_2: bool,
    /// The Dive's choice (0 Sword, 1 Shield, 2 Rod).
    pub choice: u8,
}

/// The first, second and third forms for each Dive choice (KH2's form ids).
pub const TABLE: [[u8; 3]; 3] = [[form::VALOR, form::LIMIT, form::WISDOM], [form::LIMIT, form::WISDOM, form::VALOR], [form::WISDOM, form::VALOR, form::LIMIT]];

/// The Dive's choice as `TABLE`'s row (KH1 writes 0..2; anything else the Sword's).
pub fn column(choice: u8) -> usize {
    if choice <= 2 {
        choice as usize
    } else {
        0
    }
}

pub fn choice_name(choice: u8) -> &'static str {
    ["Sword", "Shield", "Rod"][column(choice)]
}

impl Flags {
    /// From Sora's save block (`save` starting at the block's start, at least `SPAN_END` long).
    pub fn read(save: &[u8]) -> Option<Flags> {
        if save.len() < SPAN_END {
            return None;
        }
        let journal = |e: u8| save[JOURNAL + (e >> 3) as usize] & (0x80 >> (e & 7)) != 0;
        Some(Flags {
            guard_armor: save[PROGRESS + TRAVERSE_TOWN] >= GUARD_ARMOR_TT,
            opposite_armor: journal(OPPOSITE_ARMOR_ENTRY),
            pegasus_cup: save[PEGASUS_CUP] >= 1,
            anti_sora: save[PROGRESS + NEVERLAND] >= ANTI_SORA_NL,
            riku_2: save[PROGRESS + HOLLOW_BASTION] >= RIKU_2_HB,
            choice: save[DIVE_CHOICE],
        })
    }

    /// The forms these fights open (`formlevel::owned` bits).
    pub fn forms(&self) -> u8 {
        let row = TABLE[column(self.choice)];
        let mut b = 0;
        for (beaten, f) in [self.guard_armor, self.opposite_armor, self.pegasus_cup].into_iter().zip(row) {
            if beaten {
                b |= owned::bit(f);
            }
        }
        if self.anti_sora {
            b |= owned::bit(form::MASTER);
        }
        if self.riku_2 {
            b |= owned::bit(form::FINAL);
        }
        b
    }

    pub fn describe(&self) -> String {
        let s = |b: bool| if b { "beaten" } else { "not yet" };
        format!(
            "Dive choice {} ({}); Guard Armor {}, Opposite Armor {}, Pegasus Cup {}, Anti-Sora {}, Riku 2 {}",
            self.choice,
            choice_name(self.choice),
            s(self.guard_armor),
            s(self.opposite_armor),
            if self.pegasus_cup { "won" } else { "not yet" },
            s(self.anti_sora),
            s(self.riku_2)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn save() -> Vec<u8> {
        vec![0; SPAN_END]
    }

    #[test]
    fn nothing_beaten_opens_nothing() {
        assert_eq!(Flags::read(&save()).unwrap().forms(), 0);
    }

    #[test]
    fn each_fight_opens_its_dive_choices_form() {
        let mut s = save();
        s[PROGRESS + TRAVERSE_TOWN] = GUARD_ARMOR_TT;
        s[DIVE_CHOICE] = 2;
        assert_eq!(Flags::read(&s).unwrap().forms(), owned::bit(form::WISDOM));
        s[DIVE_CHOICE] = 1;
        assert_eq!(Flags::read(&s).unwrap().forms(), owned::bit(form::LIMIT));
        // (The Pegasus Cup alone, Sword: the third, Wisdom.)
        let mut s = save();
        s[PEGASUS_CUP] = 1;
        assert_eq!(Flags::read(&s).unwrap().forms(), owned::bit(form::WISDOM));
    }

    #[test]
    fn the_journal_reads_high_bit_first() {
        let mut s = save();
        s[0x16F6] = 0x01;
        let f = Flags::read(&s).unwrap();
        assert!(f.opposite_armor);
        assert_eq!(f.forms(), owned::bit(form::LIMIT));
    }

    #[test]
    fn gauge_fights_count() {
        let mut s = save();
        assert_eq!(gauge_fights(&s), 0);
        s[PROGRESS + 1] = 0x42;
        s[PROGRESS + 11] = 0x33;
        s[JOURNAL + (0xA5 >> 3)] |= 0x80 >> (0xA5 & 7);
        assert_eq!(gauge_fights(&s), 3);
        s[JOURNAL + (0xA9 >> 3)] |= 0x80 >> (0xA9 & 7);
        assert_eq!(gauge_fights(&s), 3);
        s[0x499 + 0x3C] = 1;
        assert_eq!(gauge_fights(&s), 4);
    }

    #[test]
    fn master_and_final() {
        let mut s = save();
        s[PROGRESS + NEVERLAND] = ANTI_SORA_NL;
        s[PROGRESS + HOLLOW_BASTION] = RIKU_2_HB;
        assert_eq!(Flags::read(&s).unwrap().forms(), owned::bit(form::MASTER) | owned::bit(form::FINAL));
        assert_eq!(column(7), 0);
    }
}
