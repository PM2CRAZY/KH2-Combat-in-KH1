//! KH2's support abilities for the Drive gauge and damage taken: Damage Control, Damage Drive,
//! Form Boost and Drive Converter, learned at Sora's level into KH1's menu (`growth::MENU`) and
//! counted while equipped on base Sora (`growth::Kh1Sora::support`). The rules are KH2's own
//! (the Steam KH2FM executable and its 03system `sstm`, `docs/drive-model.md`, "Support
//! abilities"); the runtime uses them in `win::hurt` (Damage Control), `win::gauge` (Damage
//! Drive, Form Boost) and `win::prize` (Drive Converter).
//!
//! KH2 sums an equipped ability's effect into the player's ability record once per copy
//! (base Sora's support abilities count in a Drive Form too): Damage Drive adds 50 to its
//! drive-from-damage rate (from 0), Form Boost multiplies the form gauge's drain by 0.8 (from 1),
//! Drive Converter multiplies the munny kept by 0.5 (from 1), Damage Control adds 0.25 to its HP
//! share (from 0).

/// Equipped copies of each (on base Sora, KH1's menu).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Support {
    pub damage_control: u8,
    pub damage_drive: u8,
    pub form_boost: u8,
    pub drive_converter: u8,
}

impl Support {
    /// One of each (`abilities = all`).
    pub const ONE_EACH: Support = Support { damage_control: 1, damage_drive: 1, form_boost: 1, drive_converter: 1 };

    /// Packed into a u32 (one byte each), for an atomic.
    pub fn pack(self) -> u32 {
        u32::from_le_bytes([self.damage_control, self.damage_drive, self.form_boost, self.drive_converter])
    }

    pub fn unpack(v: u32) -> Support {
        let [a, b, c, d] = v.to_le_bytes();
        Support { damage_control: a, damage_drive: b, form_boost: c, drive_converter: d }
    }

    /// Each ability whose copies differ from `old`'s: (name, copies now, what it does now), for
    /// the log's one line per change.
    pub fn changes(self, old: Support) -> Vec<String> {
        let mut out = Vec::new();
        let mut one = |name: &str, now: u8, was: u8, what: String| {
            if now != was {
                out.push(if now == 0 {
                    format!("{name}: unequipped (was {was}), no effect")
                } else {
                    format!("{name}: {now} equipped (was {was}): {what}")
                });
            }
        };
        one(
            "Damage Control",
            self.damage_control,
            old.damage_control,
            format!("damage taken halved at {}% of max HP or less", (DAMAGE_CONTROL_SHARE * self.damage_control as f32 * 100.0).round()),
        );
        one(
            "Damage Drive",
            self.damage_drive,
            old.damage_drive,
            format!("damage taken out of a form fills the Drive gauge, damage / max HP x {}", damage_drive_rate(self.damage_drive)),
        );
        one(
            "Form Boost",
            self.form_boost,
            old.form_boost,
            format!("the form gauge drains x{:.3} a frame", form_drain(self.form_boost)),
        );
        one(
            "Drive Converter",
            self.drive_converter,
            old.drive_converter,
            format!("munny drops kept x{} (the rest become Drive orbs: 20 and 5 munny big, 1 munny small)", munny_kept(self.drive_converter)),
        );
        out
    }
}

/// Damage Drive's drive-from-damage rate per copy (KH2's `sstm` +0x138).
pub const DAMAGE_DRIVE_RATE: f32 = 50.0;
/// Form Boost's form-gauge drain factor per copy (`sstm` +0x140).
pub const FORM_BOOST: f32 = 0.8;
/// Drive Converter's munny-kept factor per copy (`sstm` +0x170).
pub const DRIVE_CONVERTER: f32 = 0.5;
/// Damage Control's HP share per copy (`sstm` +0x174), and the damage it leaves in percent
/// (`sstm` +0x178, an int).
pub const DAMAGE_CONTROL_SHARE: f32 = 0.25;
pub const DAMAGE_CONTROL_PERCENT: i32 = 50;

/// The drive-from-damage rate with `copies` equipped (0 without: KH2's vanilla Sora gains nothing
/// from damage).
pub fn damage_drive_rate(copies: u8) -> f32 {
    DAMAGE_DRIVE_RATE * copies as f32
}

/// The form gauge's drain factor with `copies` of Form Boost.
pub fn form_drain(copies: u8) -> f32 {
    FORM_BOOST.powi(copies as i32)
}

/// The share of munny kept with `copies` of Drive Converter.
pub fn munny_kept(copies: u8) -> f32 {
    DRIVE_CONVERTER.powi(copies as i32)
}

/// Damage Control (KH2's damage routine, its last step): with Sora's HP (before the hit) at or
/// below `max HP × 0.25 × copies` (truncated), the hit's damage is `(damage × 50 + 99) / 100`,
/// half rounded up. Copies add up (two: half his HP or less). Returns the damage dealt.
pub fn damage_control(damage: i32, hp: i32, max_hp: i32, copies: u8) -> i32 {
    if copies == 0 || damage <= 0 || max_hp <= 0 {
        return damage;
    }
    let at = (max_hp as f32 * (DAMAGE_CONTROL_SHARE * copies as f32)) as i32;
    if hp <= at {
        (DAMAGE_CONTROL_PERCENT * damage + 99) / 100
    } else {
        damage
    }
}

/// Drive Converter (KH2's enemy prize drop): with `copies` equipped, each munny size's count
/// keeps `count × 0.5^copies` (truncated) and the rest become Drive orbs, KH2's big and medium
/// munny big orbs and its small munny small orbs. KH1's three munny types (0: 1 munny, 1: 5,
/// 2: 20) are KH2's small, medium and big. Returns the counts kept and the orbs (small, big).
pub fn convert_munny(munny: [i32; 3], copies: u8) -> ([i32; 3], i32, i32) {
    if copies == 0 {
        return (munny, 0, 0);
    }
    let f = munny_kept(copies);
    let mut kept = munny;
    let (mut small, mut big) = (0, 0);
    for (t, n) in munny.iter().enumerate() {
        if *n <= 0 {
            continue;
        }
        let k = (*n as f32 * f) as i32;
        kept[t] = k;
        if t == 0 {
            small += n - k;
        } else {
            big += n - k;
        }
    }
    (kept, small, big)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_control_halves_at_low_hp() {
        // One copy: a quarter of max HP or less.
        assert_eq!(damage_control(40, 25, 100, 1), 20);
        assert_eq!(damage_control(40, 26, 100, 1), 40);
        assert_eq!(damage_control(41, 10, 100, 1), 21, "half, rounded up");
        assert_eq!(damage_control(1, 10, 100, 1), 1);
        // Two: half or less.
        assert_eq!(damage_control(40, 50, 100, 2), 20);
        assert_eq!(damage_control(40, 51, 100, 2), 40);
        // The share is truncated (max 30: a quarter is 7).
        assert_eq!(damage_control(10, 7, 30, 1), 5);
        assert_eq!(damage_control(10, 8, 30, 1), 10);
        // None equipped, or no damage: as it is.
        assert_eq!(damage_control(40, 1, 100, 0), 40);
        assert_eq!(damage_control(0, 1, 100, 1), 0);
    }

    #[test]
    fn rates_per_copy() {
        assert_eq!(damage_drive_rate(0), 0.0);
        assert_eq!(damage_drive_rate(2), 100.0);
        assert_eq!(form_drain(0), 1.0);
        assert!((form_drain(2) - 0.64).abs() < 1e-6);
        assert_eq!(munny_kept(1), 0.5);
        // The same factor as the gauge's own Form Boost field.
        let mut d = character_core::drive::Drive::new(9);
        d.form_boost = 1;
        d.enter(character_core::drive::form::VALOR, 1);
        d.tick(true);
        assert!((d.gauge.unwrap().left - (2400.0 - form_drain(1))).abs() < 1e-3);
    }

    #[test]
    fn drive_converter_turns_munny_into_orbs() {
        assert_eq!(convert_munny([3, 2, 1], 0), ([3, 2, 1], 0, 0));
        // One copy: half kept (truncated), the rest orbs; one munny of a size is all orbs.
        assert_eq!(convert_munny([3, 2, 1], 1), ([1, 1, 0], 2, 2));
        assert_eq!(convert_munny([1, 0, 0], 1), ([0, 0, 0], 1, 0));
        // Two: a quarter kept.
        assert_eq!(convert_munny([8, 4, 0], 2), ([2, 1, 0], 6, 3));
        assert_eq!(convert_munny([0, 0, 0], 3), ([0, 0, 0], 0, 0));
    }

    #[test]
    fn packs_and_tells_changes() {
        let s = Support { damage_control: 1, damage_drive: 2, form_boost: 0, drive_converter: 3 };
        assert_eq!(Support::unpack(s.pack()), s);
        let lines = s.changes(Support::default());
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("Damage Control: 1 equipped") && lines[0].contains("25%"), "{lines:?}");
        assert!(lines[1].contains("x 100"), "{lines:?}");
        let off = Support::default().changes(s);
        assert!(off.iter().all(|l| l.contains("unequipped")) && off.len() == 3);
        assert!(s.changes(s).is_empty());
    }
}
