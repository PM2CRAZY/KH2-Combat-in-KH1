//! KH2's combo row records for KH1's own row pickers (testing branch, t888; the owner: "the
//! combo. Which attack a press gives, and the Finishing Plus counter, are still ours", plan part 1).
//! `docs/kh1-anim-slots.md` §7.z.11.
//!
//! One table a moveset (base Sora's, and each Drive Form's on KH1's own states), built once from
//! KH2's own data as the mod loaded it from the player's files (`pose::kh2`: Sora's PTYA rows,
//! `00battle.bin`, and the moveset's clips, Checked (files)): every PTYA row in KH2's table order
//! with what KH2's row walk reads of it (kind, `sub`, `combo_offset`, flags, the distance, height
//! and facing windows, the ability), the KH1 row of ours that plays it (`kh1rows`: by its PTYA row,
//! else by its motion, an attack row of that motion), which motions KH2 has a clip of, and the
//! moveset's combo rules (KH2's base lengths and finishers) and jump height. KH1's pickers walk it
//! on KH1's own state at each Attack press (`combomods::Kh1Press`, t889), with KH2's walk
//! (`character_core::HostPress`); nothing of it changes after it is built, and nothing runs every
//! frame.

use super::{form, pose};
use character_core::{Abilities, Character, ComboRow};
use std::collections::HashSet;
use std::sync::OnceLock;

/// One moveset's records.
pub struct Table {
    /// KH2's rows, in KH2's table order.
    pub rows: Vec<ComboRow>,
    /// The KH1 row of ours that plays each row (`kh1rows::OURS` index), by the row's place.
    pub kh1: Vec<Option<usize>>,
    /// The motions KH2 has a clip of in this moveset (a row with none is no attack, KH2's).
    pub clips: HashSet<u16>,
    pub ground_base: u8,
    pub air_base: u8,
    /// KH2's most finishers in a row before Finishing Plus (`[+0x5C0] + 0x202` less its copies).
    pub finisher_base: u8,
    /// KH2's jump height (row flag 2 raises a row's height window by it).
    pub jump_height: f32,
}

impl Table {
    fn of(ch: &Character) -> Table {
        use super::kh1rows as k;
        let rows = ch.moveset.combo_rows.clone();
        let kh1 = rows
            .iter()
            .map(|r| {
                let plays = |n: usize| k::OURS[n].attack && k::OURS[n].motion == r.motion;
                k::by_kh2_row(r.id).filter(|&n| plays(n)).or_else(|| k::by_combo_motion(r.motion).filter(|&n| plays(n)))
            })
            .collect();
        Table {
            rows,
            kh1,
            clips: ch.moveset.clips.keys().copied().collect(),
            ground_base: ch.combo_rules.ground_base,
            air_base: ch.combo_rules.air_base,
            finisher_base: ch.combo_rules.finisher_base,
            jump_height: ch.params.jump_height,
        }
    }

    /// KH2's combo length here with these abilities (`Character::combo_length`'s rule: the base,
    /// plus Combo Plus or Air Combo Plus, less Negative Combo, at least 1).
    pub fn length(&self, a: &Abilities, air: bool) -> u8 {
        let (base, plus) = if air { (self.air_base, a.air_combo_plus) } else { (self.ground_base, a.combo_plus) };
        base.saturating_add(plus).saturating_sub(a.negative_combo).max(1)
    }

    /// KH2's most finishers in a row with these abilities (one plus Finishing Plus's copies).
    pub fn most_finishers(&self, a: &Abilities) -> u8 {
        self.finisher_base.saturating_add(a.finishing_plus)
    }

    /// The KH1 row of ours that plays KH2's row `id` (`kh1rows`), if it has one.
    pub fn kh1_row(&self, id: u8) -> Option<usize> {
        let i = self.rows.iter().position(|r| r.id == id)?;
        self.kh1.get(i).copied().flatten()
    }
}

static BASE: OnceLock<Table> = OnceLock::new();
static FORMS: [OnceLock<Option<Table>>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];

/// The table of the moveset now: a Drive Form's on KH1's own states (`form::native_form`), else
/// base Sora's. Built the first time it is asked for. `None` without KH2's data.
pub fn now() -> Option<&'static Table> {
    let kh2 = pose::kh2()?;
    match form::native_form() {
        Some(kind) => {
            let i = match kind {
                form::Kind::Valor => 0,
                form::Kind::Wisdom => 1,
                form::Kind::Final => 2,
            };
            FORMS[i].get_or_init(|| kind.data(kh2).map(|f| Table::of(&f.character))).as_ref()
        }
        None => Some(BASE.get_or_init(|| Table::of(&kh2.character))),
    }
}
