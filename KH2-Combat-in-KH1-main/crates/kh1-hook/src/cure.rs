//! How much base Sora's KH2 Cure heals (the Magic style at KH2, `magic = kh2`): KH2's rule on
//! KH1's stats. `win/formmagic.rs` applies it when the spell's script strikes Sora or a partner.
//! Pure (no game memory), so it is tested here. `docs/kh1-move-styles.md` has the sources.
//!
//! - **KH2's rule** (KH2 FM's hit routine, read in the Steam executable; `combat_core`'s
//!   `compute_healing`): Cure, Cura and Curaga strike with ATKP 708 on the caster and 709 on each
//!   party member in range (their scripts, `magic/CURE_<level>.mag`). Both rows are kind 6
//!   ("C-Cure"), stat source 1 (Magic). A kind 6 row heals `ceil(power × caster's Magic / 100)`
//!   HP: no share of max HP, and the partners' rows are half Sora's. Base Sora's powers are
//!   400 / 500 / 600 (Cure / Cura / Curaga; 709: 200 / 250 / 300).
//! - **KH1's Magic.** KH1 has no Magic stat: its spells, its own Cure too, take their power from
//!   max MP (attack data `+0` = 1). So max MP stands in, times `MAGIC_PER_MP` and the ini's
//!   `magic_scale`. KH1's HP is on KH2's scale (both start near 20 and are near 60 midway), but
//!   its max MP is far below KH2's Magic: KH2's Sora has Magic 16 at level 20, 24 at 37 and 30
//!   at 50 (00battle's `lvup`) plus his Keyblade's, while KH1's has a few points of max MP (KH1's
//!   level table gives five in 99 levels; Keyblades give −2..+3, accessories a point or two).
//!   With KH2's own numbers Cure heals about 110..120% of max HP all through the game (4 × Magic
//!   against his HP then), so a Cure there is a full heal. KH2's formula on max MP alone would
//!   heal a fifth to a half of KH1's max HP; max MP × 3 gives KH2's: at level 37 (57 HP), 3 MP
//!   heals 36 / 45 / 54 and 5 MP a full 60 / 75 / 90.

use combat_core::{AttackDef, AttackKind, Combatant, StatSource, Stats, Team};

/// KH2 Magic per point of KH1 max MP (above).
pub const MAGIC_PER_MP: f32 = 3.0;

/// KH1's stand-in for KH2's Magic: max MP × `MAGIC_PER_MP` × `magic_scale`, rounded.
pub fn magic(max_mp: i32, magic_scale: f32) -> u16 {
    (max_mp.max(0) as f32 * MAGIC_PER_MP * magic_scale).round().clamp(0.0, u16::MAX as f32) as u16
}

/// HP an ATKP row heals (KH2's rule, `combat_core::compute_healing`): kind 5 a share of the
/// target's max HP by power × stat, kind 6 power × stat / 100; 0 for any other kind. `stat` 0 is
/// the caster's Strength, 1 his Magic, anything else none.
pub fn heal(kind: u8, stat: u8, power: u16, magic: u16, strength: u16, target_max: u32) -> u32 {
    let k = AttackKind::from_id(kind);
    if !k.heals() {
        return 0;
    }
    let a = AttackDef { kind: k, stat: StatSource::from_id(stat), power, ..Default::default() };
    let caster = Combatant::new(0, Team::Player, Stats { hp: 1, strength, magic, defense: 0 });
    let mut target = Combatant::new(1, Team::Player, Stats { hp: target_max, ..Default::default() });
    target.max_hp = target_max;
    combat_core::compute_healing(&a, &caster, &target)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 00battle's rows: 708 (Sora) and 709 (the party), base Sora's subs 0 / 2 / 4.
    const SORA: [u16; 3] = [400, 500, 600];
    const PARTY: [u16; 3] = [200, 250, 300];

    #[test]
    fn kh2_rule() {
        // KH2's Sora at level 20 (Magic 16 + a Keyblade's 2) heals 72 / 90 / 108.
        let got: Vec<u32> = SORA.iter().map(|&p| heal(6, 1, p, 18, 0, 60)).collect();
        assert_eq!(got, [72, 90, 108]);
        // Kind 5: a share of the target's max HP.
        assert_eq!(heal(5, 1, 1000, 10, 0, 57), 57);
        // Not a heal.
        assert_eq!(heal(0, 1, 400, 18, 0, 60), 0);
    }

    #[test]
    fn kh1_magic() {
        assert_eq!(magic(5, 1.0), 15);
        assert_eq!(magic(8, 1.0), 24);
        assert_eq!(magic(5, 0.5), 8);
        assert_eq!(magic(-1, 1.0), 0);
    }

    #[test]
    fn kh1_sora() {
        // Level 37 (KH1's base max HP 51 and max MP 3, a +6 HP accessory, 2 MP from his
        // equipment): Magic 15, Cure 60 = a full heal of 57; the s338 share (max × power / 3000)
        // was 8 / 10 / 12.
        let m = magic(5, 1.0);
        let got: Vec<u32> = SORA.iter().map(|&p| heal(6, 1, p, m, 0, 57)).collect();
        assert_eq!(got, [60, 75, 90]);
        let old: Vec<u32> = SORA.iter().map(|&p| (57.0 * p as f32 / 3000.0).ceil() as u32).collect();
        assert_eq!(old, [8, 10, 12]);
        // A partner in range: half.
        let got: Vec<u32> = PARTY.iter().map(|&p| heal(6, 1, p, m, 0, 50)).collect();
        assert_eq!(got, [30, 38, 45]);
    }
}
