//! KH2's Drive support abilities' rules (the owner, the main-branch port's round 6, row 5: answers
//! 1 (a), 2 (a), 3 (a), "go ahead"). Notes: `docs/kh1-drive-forms-native.md` §17. Steam KH2,
//! Checked (decomp), the sstm values Checked (files) (`03system.bin`'s `pref` / `sstm`):
//!
//! - **Damage Drive** (`0x18C`): the ability block `+0x58` += sstm `+0x138` (50) a copy, from 0; out of a
//!   form a loss of HP gives `loss / max HP × +0x58` points (`0x1403A75F0` → `0x1403A9CF0`, truncated).
//! - **Form Boost** (`0x18E`): `+0x44` ×= sstm `+0x140` (0.8) a copy, from 1; the form gauge's time
//!   drain × `+0x44` (`0x1403A92C0`).
//! - **Drive Converter** (`0x21C`): `+0x74` ×= sstm `+0x170` (0.5) a copy, from 1; each munny kind's
//!   count keeps `(int)(count × +0x74)`, the rest are Drive orbs (`0x1403DC170`, by `0x1405C7978`:
//!   big and middle munny → big orbs, small → small).
//! - **Damage Control** (`0x21E`): `+0x78` += sstm `+0x174` (0.25) a copy, from 0; KH2's damage
//!   (`0x1403C12F0`, its end) is × sstm `+0x178` (50) %, rounded up, while the target's HP is at most
//!   max HP × `+0x78`.
//!
//! Pure Rust, unit tested on any host; the game side is `win::driveabilities`.

pub const DAMAGE_DRIVE: f32 = 50.0;
pub const FORM_BOOST: f32 = 0.8;
pub const DRIVE_CONVERTER: f32 = 0.5;
pub const DAMAGE_CONTROL_HP: f32 = 0.25;
pub const DAMAGE_CONTROL_PERCENT: i32 = 50;

/// Damage Drive's points for a loss of `loss` HP of `max` (0 without a copy).
pub fn damage_drive(loss: i32, max: i32, copies: u8) -> i32 {
    if loss <= 0 || max <= 0 || copies == 0 {
        return 0;
    }
    (loss as f32 / max as f32 * DAMAGE_DRIVE * copies as f32) as i32
}

/// Form Boost's factor on the form gauge's drain.
pub fn form_boost(copies: u8) -> f32 {
    FORM_BOOST.powi(copies as i32)
}

/// Drive Converter on one munny kind's count: (munny kept, orbs).
pub fn drive_converter(count: i32, copies: u8) -> (i32, i32) {
    if count <= 0 || copies == 0 {
        return (count.max(0), 0);
    }
    let keep = (count as f32 * DRIVE_CONVERTER.powi(copies as i32)) as i32;
    (keep, count - keep)
}

/// KH1's munny prize types (1, 5, 20 munny) and the orb each becomes: the two larger big orbs, the
/// smallest a small one (KH2's `0x1405C7978` by size).
pub const MUNNY_TYPES: [(usize, bool); 3] = [(0, false), (1, true), (2, true)];

/// Damage Control on a hit of `damage` to a target at `hp` of `max`.
pub fn damage_control(damage: i32, hp: i32, max: i32, copies: u8) -> i32 {
    if damage <= 0 || copies == 0 || hp as f32 > max as f32 * DAMAGE_CONTROL_HP * copies as f32 {
        return damage;
    }
    (DAMAGE_CONTROL_PERCENT * damage + 99) / 100
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kh2s_numbers() {
        assert_eq!(damage_drive(30, 120, 1), 12);
        assert_eq!(damage_drive(30, 120, 2), 25);
        assert_eq!(damage_drive(30, 120, 0), 0);
        assert!((form_boost(2) - 0.64).abs() < 1e-6 && form_boost(0) == 1.0);
        assert_eq!(drive_converter(5, 1), (2, 3));
        assert_eq!(drive_converter(5, 2), (1, 4));
        assert_eq!(drive_converter(5, 0), (5, 0));
        // At 30 of 120 HP (25 %): one copy halves (rounded up), above it doesn't; two copies reach 50 %.
        assert_eq!(damage_control(9, 30, 120, 1), 5);
        assert_eq!(damage_control(9, 31, 120, 1), 9);
        assert_eq!(damage_control(9, 60, 120, 2), 5);
    }
}
