//! Phase 7: KH2's abilities from KH1's menu.
//!
//! KH1 keeps Sora's abilities in the save block (`0x142DE9D60`, which `0x1428695A0` points
//! at): party member `n` at `n * 0x74`, its level the byte at `+4`, its abilities the 48 bytes
//! at `+0x44` (an ability id each, 0 = empty, bit 7 set = learned but not equipped; KH1 counts
//! an equipped ability with `0x140282A70`), and the shared abilities (High Jump, Mermaid Kick,
//! Glide, Superglide) at `+0x599`, the same way. Ability ids as the Archipelago KH1 world and
//! its Lua connector use them (gaithern, MIT): 1 High Jump, 3 Glide, 4 Superglide; 6 Combo
//! Plus, 7 Air Combo Plus, 17 Vortex, 18 Aerial Sweep, 19 Counterattack, 20 Blitz, 21 Guard,
//! 22 Dodge Roll; Final Mix's 53 Slapshot, 54 Sliding Dash, 55 Hurricane Blast, 56 Ripple
//! Drive, 57 Stun Impact, 58 Gravity Break.
//!
//! - What KH1 has, KH2 takes from it while equipped: High Jump's levels, Glide (and each
//!   Superglide a Glide level more), Guard, Dodge Roll, each Combo Plus / Air Combo Plus, and
//!   KH1's combo modifiers as KH2's nearest one (Vortex: Flash Step, Aerial Sweep, Counterattack:
//!   Counterguard, Blitz: Finishing Leap, Slapshot, Sliding Dash: Slide Dash, Hurricane Blast:
//!   Explosion, Ripple Drive: Vicinity Break, Stun Impact: Guard Break, Gravity Break: Aerial
//!   Finish).
//! - What only KH2 has (Quick Run, Aerial Dodge, Aerial Recovery, the dodge roll's levels past
//!   the first, Finishing Plus, Combo Master and KH2's own modifiers) comes with Sora's level
//!   (`LEVELS`), until KH1's menu lists them.

use character_core::Abilities;

/// KH1 ability ids.
pub mod kh1 {
    pub const HIGH_JUMP: u8 = 1;
    pub const GLIDE: u8 = 3;
    pub const SUPERGLIDE: u8 = 4;
    pub const COMBO_PLUS: u8 = 6;
    pub const AIR_COMBO_PLUS: u8 = 7;
    pub const VORTEX: u8 = 17;
    pub const AERIAL_SWEEP: u8 = 18;
    pub const COUNTERATTACK: u8 = 19;
    pub const BLITZ: u8 = 20;
    pub const GUARD: u8 = 21;
    pub const DODGE_ROLL: u8 = 22;
    pub const SLAPSHOT: u8 = 53;
    pub const SLIDING_DASH: u8 = 54;
    pub const HURRICANE_BLAST: u8 = 55;
    pub const RIPPLE_DRIVE: u8 = 56;
    pub const STUN_IMPACT: u8 = 57;
    pub const GRAVITY_BREAK: u8 = 58;
}

/// KH2 ability numbers (the combo rows' `ability`, from the install's item table).
pub mod kh2 {
    pub const GUARD: u16 = 1;
    pub const UPPER_SLASH: u16 = 18;
    pub const SLAPSHOT: u16 = 90;
    pub const DODGE_SLASH: u16 = 91;
    pub const SLIDE_DASH: u16 = 92;
    pub const GUARD_BREAK: u16 = 93;
    pub const EXPLOSION: u16 = 94;
    pub const FINISHING_LEAP: u16 = 95;
    pub const COUNTERGUARD: u16 = 96;
    pub const AERIAL_SWEEP: u16 = 97;
    pub const AERIAL_SPIRAL: u16 = 98;
    pub const HORIZONTAL_SLASH: u16 = 99;
    pub const AERIAL_FINISH: u16 = 100;
    pub const RETALIATING_SLASH: u16 = 101;
    pub const FLASH_STEP: u16 = 177;
    pub const AERIAL_DIVE: u16 = 178;
    pub const MAGNET_BURST: u16 = 179;
    pub const VICINITY_BREAK: u16 = 180;
}

/// KH1's modifiers and KH2's nearest.
pub const MODIFIERS: [(u8, u16, &str); 10] = [
    (kh1::VORTEX, kh2::FLASH_STEP, "Vortex: Flash Step"),
    (kh1::AERIAL_SWEEP, kh2::AERIAL_SWEEP, "Aerial Sweep"),
    (kh1::COUNTERATTACK, kh2::COUNTERGUARD, "Counterattack: Counterguard"),
    (kh1::BLITZ, kh2::FINISHING_LEAP, "Blitz: Finishing Leap"),
    (kh1::SLAPSHOT, kh2::SLAPSHOT, "Slapshot"),
    (kh1::SLIDING_DASH, kh2::SLIDE_DASH, "Sliding Dash: Slide Dash"),
    (kh1::HURRICANE_BLAST, kh2::EXPLOSION, "Hurricane Blast: Explosion"),
    (kh1::RIPPLE_DRIVE, kh2::VICINITY_BREAK, "Ripple Drive: Vicinity Break"),
    (kh1::STUN_IMPACT, kh2::GUARD_BREAK, "Stun Impact: Guard Break"),
    (kh1::GRAVITY_BREAK, kh2::AERIAL_FINISH, "Gravity Break: Aerial Finish"),
];

/// What only KH2 has, by Sora's level: (level, what).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kh2Only {
    QuickRun(u8),
    AerialDodge(u8),
    /// The dodge roll's level, once KH1's Dodge Roll is equipped.
    DodgeRoll(u8),
    AerialRecovery,
    FinishingPlus,
    ComboMaster,
    Modifier(u16, &'static str),
}

pub const LEVELS: [(u8, Kh2Only); 21] = [
    (4, Kh2Only::AerialRecovery),
    (7, Kh2Only::QuickRun(1)),
    (10, Kh2Only::Modifier(kh2::DODGE_SLASH, "Dodge Slash")),
    (13, Kh2Only::AerialDodge(1)),
    (16, Kh2Only::Modifier(kh2::HORIZONTAL_SLASH, "Horizontal Slash")),
    (19, Kh2Only::DodgeRoll(2)),
    (22, Kh2Only::QuickRun(2)),
    (25, Kh2Only::Modifier(kh2::AERIAL_SPIRAL, "Aerial Spiral")),
    (28, Kh2Only::AerialDodge(2)),
    (31, Kh2Only::Modifier(kh2::UPPER_SLASH, "Upper Slash")),
    (34, Kh2Only::FinishingPlus),
    (37, Kh2Only::DodgeRoll(3)),
    (40, Kh2Only::QuickRun(3)),
    (43, Kh2Only::Modifier(kh2::RETALIATING_SLASH, "Retaliating Slash")),
    (46, Kh2Only::AerialDodge(3)),
    (48, Kh2Only::Modifier(kh2::AERIAL_DIVE, "Aerial Dive")),
    (50, Kh2Only::Modifier(kh2::MAGNET_BURST, "Magnet Burst")),
    (55, Kh2Only::ComboMaster),
    (60, Kh2Only::DodgeRoll(4)),
    (65, Kh2Only::QuickRun(4)),
    (70, Kh2Only::AerialDodge(4)),
];

/// Sora's side of KH1's save block.
#[derive(Clone, Debug, PartialEq)]
pub struct Kh1Sora {
    pub level: u8,
    /// The 48 ability bytes (`+0x44`).
    pub abilities: Vec<u8>,
    /// The shared abilities (`+0x599`).
    pub shared: Vec<u8>,
}

/// Equipped copies of `id` in a KH1 ability list (as `0x140282A70`: up to the first 0).
pub fn equipped(list: &[u8], id: u8) -> u8 {
    list.iter().take_while(|b| **b != 0).filter(|b| **b == id).count().min(255) as u8
}

/// The ability modes (the ini's `abilities`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Testing (the default before Phase 7): every movement ability at MAX, guard, aerial
    /// recovery, combo master; no combo modifiers.
    Test,
    /// KH1's menu and Sora's level.
    Kh1,
    /// Everything, every modifier too.
    All,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Mode> {
        match s.trim().to_ascii_lowercase().as_str() {
            "test" | "max" => Some(Mode::Test),
            "kh1" | "menu" | "level" => Some(Mode::Kh1),
            "all" => Some(Mode::All),
            _ => None,
        }
    }
}

pub fn test() -> Abilities {
    Abilities { guard: true, aerial_recovery: true, combo_master: true, ..Abilities::max_growth() }
}

pub fn all() -> Abilities {
    let mut equipped: Vec<u16> = MODIFIERS.iter().map(|m| m.1).collect();
    for (_, l) in LEVELS {
        if let Kh2Only::Modifier(n, _) = l {
            equipped.push(n);
        }
    }
    equipped.push(kh2::GUARD);
    Abilities { guard: true, aerial_recovery: true, combo_master: true, finishing_plus: 1, equipped, ..Abilities::max_growth() }
}

impl Kh1Sora {
    /// KH2's abilities from KH1's menu and Sora's level.
    pub fn kh2(&self) -> Abilities {
        let a = &self.abilities;
        let s = &self.shared;
        let mut out = Abilities { guard: false, ..Abilities::default() };
        out.high_jump = match equipped(s, kh1::HIGH_JUMP) {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 4,
        };
        if equipped(s, kh1::GLIDE) > 0 {
            out.glide = (1 + equipped(s, kh1::SUPERGLIDE)).min(4);
        }
        out.guard = equipped(a, kh1::GUARD) > 0;
        if out.guard {
            out.equipped.push(kh2::GUARD);
        }
        let roll = equipped(a, kh1::DODGE_ROLL) > 0;
        if roll {
            out.dodge_roll = 1;
        }
        out.combo_plus = equipped(a, kh1::COMBO_PLUS);
        out.air_combo_plus = equipped(a, kh1::AIR_COMBO_PLUS);
        for (k1, k2, _) in MODIFIERS {
            if equipped(a, k1) > 0 && !out.equipped.contains(&k2) {
                out.equipped.push(k2);
            }
        }
        for (at, what) in LEVELS {
            if self.level < at {
                continue;
            }
            match what {
                Kh2Only::QuickRun(l) => out.quick_run = out.quick_run.max(l),
                Kh2Only::AerialDodge(l) => out.aerial_dodge = out.aerial_dodge.max(l),
                Kh2Only::DodgeRoll(l) if roll => out.dodge_roll = out.dodge_roll.max(l),
                Kh2Only::DodgeRoll(_) => {}
                Kh2Only::AerialRecovery => out.aerial_recovery = true,
                Kh2Only::FinishingPlus => out.finishing_plus = 1,
                Kh2Only::ComboMaster => out.combo_master = true,
                Kh2Only::Modifier(n, _) => {
                    if !out.equipped.contains(&n) {
                        out.equipped.push(n);
                    }
                }
            }
        }
        out
    }
}

/// One line for the log.
pub fn describe(a: &Abilities) -> String {
    let mut names: Vec<&str> = Vec::new();
    for (_, n, name) in MODIFIERS {
        if a.equipped.contains(&n) {
            names.push(name);
        }
    }
    for (_, l) in LEVELS {
        if let Kh2Only::Modifier(n, name) = l {
            if a.equipped.contains(&n) {
                names.push(name);
            }
        }
    }
    format!(
        "high jump {}, quick run {}, aerial dodge {}, glide {}, dodge roll {}, guard {}, combo plus {}, air combo plus {}, finishing plus {}, combo master {}, aerial recovery {}; modifiers [{}]",
        a.high_jump,
        a.quick_run,
        a.aerial_dodge,
        a.glide,
        a.dodge_roll,
        a.guard,
        a.combo_plus,
        a.air_combo_plus,
        a.finishing_plus,
        a.combo_master,
        a.aerial_recovery,
        names.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sora(level: u8, abilities: &[u8], shared: &[u8]) -> Kh1Sora {
        let mut a = abilities.to_vec();
        a.resize(48, 0);
        let mut s = shared.to_vec();
        s.resize(10, 0);
        Kh1Sora { level, abilities: a, shared: s }
    }

    #[test]
    fn fresh_save_has_nothing() {
        let a = sora(1, &[], &[]).kh2();
        assert_eq!((a.high_jump, a.glide, a.dodge_roll, a.quick_run, a.aerial_dodge), (0, 0, 0, 0, 0));
        assert!(!a.guard && !a.aerial_recovery && a.equipped.is_empty());
    }

    #[test]
    fn unequipped_abilities_do_nothing() {
        let a = sora(1, &[kh1::GUARD | 0x80, kh1::DODGE_ROLL | 0x80], &[kh1::HIGH_JUMP | 0x80]).kh2();
        assert!(!a.guard);
        assert_eq!((a.dodge_roll, a.high_jump), (0, 0));
    }

    #[test]
    fn kh1_menu_maps() {
        let a = sora(
            1,
            &[kh1::GUARD, kh1::DODGE_ROLL, kh1::COMBO_PLUS, kh1::COMBO_PLUS, kh1::AIR_COMBO_PLUS, kh1::VORTEX, kh1::SLAPSHOT | 0x80],
            &[kh1::HIGH_JUMP, kh1::HIGH_JUMP, kh1::GLIDE, kh1::SUPERGLIDE],
        )
        .kh2();
        assert!(a.guard);
        assert_eq!((a.dodge_roll, a.combo_plus, a.air_combo_plus, a.high_jump, a.glide), (1, 2, 1, 2, 2));
        assert!(a.equipped.contains(&kh2::FLASH_STEP) && !a.equipped.contains(&kh2::SLAPSHOT));
        assert!(a.equipped.contains(&kh2::GUARD));
    }

    #[test]
    fn list_ends_at_first_empty() {
        let mut list = vec![kh1::GUARD, 0, kh1::DODGE_ROLL];
        list.resize(48, 0);
        assert_eq!(equipped(&list, kh1::DODGE_ROLL), 0);
    }

    #[test]
    fn levels_grant_kh2_only() {
        let a = sora(99, &[kh1::DODGE_ROLL], &[kh1::HIGH_JUMP, kh1::HIGH_JUMP, kh1::HIGH_JUMP]).kh2();
        assert_eq!((a.quick_run, a.aerial_dodge, a.dodge_roll, a.high_jump), (4, 4, 4, 4));
        assert!(a.aerial_recovery && a.combo_master && a.finishing_plus == 1);
        // No KH1 Dodge Roll: no roll at any level.
        assert_eq!(sora(99, &[], &[]).kh2().dodge_roll, 0);
    }

    #[test]
    fn modes_parse() {
        assert_eq!(Mode::parse(" KH1 "), Some(Mode::Kh1));
        assert_eq!(Mode::parse("all"), Some(Mode::All));
        assert_eq!(Mode::parse("x"), None);
    }
}
