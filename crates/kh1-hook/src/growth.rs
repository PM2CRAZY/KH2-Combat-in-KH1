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
//! - What KH1 has, KH2 takes from it while equipped (the user's levels): High Jump is High
//!   Jump LV3, Glide is Glide LV3, Superglide Glide MAX (Sora keeps one of each: `one_only`),
//!   Guard, Dodge Roll, each Combo Plus / Air Combo Plus, and
//!   KH1's combo modifiers as KH2's nearest one (Vortex: Flash Step, Aerial Sweep, Counterattack:
//!   Counterguard, Blitz: Finishing Leap, Slapshot, Sliding Dash: Slide Dash, Hurricane Blast:
//!   Explosion, Ripple Drive: Vicinity Break, Stun Impact: Guard Break, Gravity Break: Aerial
//!   Finish).
//!   Final Mix's own Combo Master (65) is KH2's.
//! - What only KH2 has goes into KH1's menu as new abilities (`MENU`, ids 66 up, in an
//!   extended copy of KH1's ability table, `win/menu.rs`), learned at Sora's level: Quick Run
//!   and Aerial Dodge in the shared abilities (with High Jump and Glide), the rest in Sora's,
//!   with KH2's AP costs. Quick Run and Aerial Dodge are LV3; the dodge roll levels up with
//!   Sora (`GROWTH`).

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
    pub const COMBO_MASTER: u8 = 65;
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

/// Which KH1 list a menu ability is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum List {
    /// The shared abilities (`+0x599`: High Jump, Glide...), no AP.
    Shared,
    /// Sora's abilities (`+0x44`), with AP.
    Sora,
}

/// What a menu ability does for KH2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    QuickRun,
    AerialDodge,
    AerialRecovery,
    FinishingPlus,
    NegativeCombo,
    Modifier(u16),
}

/// A KH2 ability in KH1's menu.
#[derive(Clone, Copy, Debug)]
pub struct MenuAbility {
    pub id: u8,
    pub name: &'static str,
    pub help: &'static str,
    /// AP (KH2's); shared abilities take none.
    pub ap: u8,
    /// Sora's level it is learned at.
    pub level: u8,
    pub list: List,
    /// The KH1 ability whose table entry it is made from (its kind and place in the menu).
    pub template: u8,
    pub effect: Effect,
}

const fn m(id: u8, name: &'static str, help: &'static str, ap: u8, level: u8, list: List, template: u8, effect: Effect) -> MenuAbility {
    MenuAbility { id, name, help, ap, level, list, template, effect }
}

/// The first id past KH1's own (65 entries in its table).
pub const FIRST_ID: u8 = 66;

pub const MENU: [MenuAbility; 12] = [
    m(66, "Quick Run", "Dash swiftly along the ground.", 0, 7, List::Shared, kh1::GLIDE, Effect::QuickRun),
    m(67, "Aerial Dodge", "Leap once more in midair.", 0, 13, List::Shared, kh1::GLIDE, Effect::AerialDodge),
    m(68, "Aerial Recovery", "Right yourself in midair after being knocked back.", 2, 4, List::Sora, kh1::COMBO_PLUS, Effect::AerialRecovery),
    m(69, "Dodge Slash", "Ground combo: strike nearby foes all around.", 2, 10, List::Sora, kh1::SLAPSHOT, Effect::Modifier(kh2::DODGE_SLASH)),
    m(70, "Horizontal Slash", "Air combo: slash foes while spinning in midair.", 2, 16, List::Sora, kh1::SLAPSHOT, Effect::Modifier(kh2::HORIZONTAL_SLASH)),
    m(71, "Negative Combo", "Shortens combos by one attack.", 2, 20, List::Sora, kh1::COMBO_PLUS, Effect::NegativeCombo),
    m(72, "Aerial Spiral", "Air combo: spin into foes in midair.", 2, 25, List::Sora, kh1::SLAPSHOT, Effect::Modifier(kh2::AERIAL_SPIRAL)),
    m(73, "Upper Slash", "Counter: knock a foe into the air after guarding.", 4, 31, List::Sora, kh1::SLAPSHOT, Effect::Modifier(kh2::UPPER_SLASH)),
    m(74, "Finishing Plus", "Adds a finishing move to the combo.", 5, 34, List::Sora, kh1::COMBO_PLUS, Effect::FinishingPlus),
    m(75, "Retaliating Slash", "Counter: strike back while knocked away.", 3, 43, List::Sora, kh1::SLAPSHOT, Effect::Modifier(kh2::RETALIATING_SLASH)),
    m(76, "Aerial Dive", "Air combo: dive down at a foe below.", 2, 48, List::Sora, kh1::SLAPSHOT, Effect::Modifier(kh2::AERIAL_DIVE)),
    m(77, "Magnet Burst", "Finisher: pull in foes all around.", 3, 50, List::Sora, kh1::SLAPSHOT, Effect::Modifier(kh2::MAGNET_BURST)),
];

/// Growth levels past the first, by Sora's level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Growth {
    QuickRun(u8),
    AerialDodge(u8),
    /// The dodge roll's level, with KH1's Dodge Roll equipped.
    DodgeRoll(u8),
}

/// (Quick Run and Aerial Dodge are LV3 as soon as they are learned, the user's choice.)
pub const GROWTH: [(u8, Growth); 3] = [(19, Growth::DodgeRoll(2)), (37, Growth::DodgeRoll(3)), (60, Growth::DodgeRoll(4))];

/// Shared abilities Sora keeps only one of (the user: one High Jump, one Glide, one
/// Superglide).
pub const ONE_ONLY: [u8; 3] = [kh1::HIGH_JUMP, kh1::GLIDE, kh1::SUPERGLIDE];

/// The shared list with the spare copies of `ONE_ONLY` taken out (an equipped copy is the one
/// kept), and the ids taken out. The list keeps its length (zeros at the end).
pub fn one_only(list: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let end = list.iter().position(|b| *b == 0).unwrap_or(list.len());
    let mut keep: Vec<Option<usize>> = vec![None; ONE_ONLY.len()];
    for (k, id) in ONE_ONLY.iter().enumerate() {
        let at = |on: bool| list[..end].iter().position(|b| b & 0x7F == *id && (b & 0x80 == 0) == on);
        keep[k] = at(true).or_else(|| at(false));
    }
    let mut out = Vec::with_capacity(list.len());
    let mut removed = Vec::new();
    for (i, b) in list[..end].iter().enumerate() {
        match ONE_ONLY.iter().position(|id| *id == b & 0x7F) {
            Some(k) if keep[k] != Some(i) => removed.push(b & 0x7F),
            _ => out.push(*b),
        }
    }
    out.resize(list.len(), 0);
    (out, removed)
}

/// Whether a list has `id` in it at all (equipped or not).
pub fn has(list: &[u8], id: u8) -> bool {
    list.iter().take_while(|b| **b != 0).any(|b| b & 0x7F == id)
}

/// KH1's ability names by id (1..=65), as the Archipelago KH1 world lists them (for the log).
pub const KH1_NAMES: [&str; 65] = [
    "High Jump", "Mermaid Kick", "Glide", "Superglide", "Treasure Magnet", "Combo Plus", "Air Combo Plus", "Critical Plus", "Second Wind", "Scan",
    "Sonic Blade", "Ars Arcanum", "Strike Raid", "Ragnarok", "Trinity Limit", "Cheer", "Vortex", "Aerial Sweep", "Counterattack", "Blitz",
    "Guard", "Dodge Roll", "MP Haste", "MP Rage", "Second Chance", "Berserk", "Jackpot", "Lucky Strike", "Charge", "Rocket",
    "Tornado", "MP Gift", "Raging Boar", "Asp's Bite", "Healing Herb", "Wind Armor", "Crescent", "Sandstorm", "Applause!", "Blazing Fury",
    "Icy Terror", "Bolts of Sorrow", "Ghostly Scream", "Hummingbird", "Time-Out", "Storm's Eye", "Ferocious Lunge", "Furious Bellow", "Spiral Wave", "Thunder Potion",
    "Cure Potion", "Aero Potion", "Slapshot", "Sliding Dash", "Hurricane Blast", "Ripple Drive", "Stun Impact", "Gravity Break", "Zantetsuken", "Tech Boost",
    "Encounter Plus", "Leaf Bracer", "Evolution", "EXP Zero", "Combo Master",
];

/// An ability's name (KH1's or ours).
pub fn name(id: u8) -> String {
    let id = id & 0x7F;
    if let Some(ab) = MENU.iter().find(|a| a.id == id) {
        return ab.name.to_string();
    }
    if let Some(r) = RENAMED.iter().find(|r| r.0 == id) {
        return r.1.to_string();
    }
    KH1_NAMES.get((id as usize).wrapping_sub(1)).map_or(format!("ability {id}"), |n| n.to_string())
}

/// The user: KH2's abilities come first. When Sora's list (48, KH1's most) is full, KH1
/// abilities that are learned but off make room, least missed first: a spare copy of one he
/// has twice or more, then these, in order. Nothing equipped, nothing of ours, and not Guard,
/// Dodge Roll, the specials or the combo modifiers.
pub const MAKE_ROOM: [u8; 15] = [
    64, // EXP Zero
    16, // Cheer
    10, // Scan
    63, // Evolution
    27, // Jackpot
    28, // Lucky Strike
    5,  // Treasure Magnet
    9,  // Second Wind
    60, // Tech Boost
    62, // Leaf Bracer
    23, // MP Haste
    24, // MP Rage
    26, // Berserk
    8,  // Critical Plus
    25, // Second Chance
];

/// KH1 abilities that are KH2 moves here, shown in the menu by their KH2 name and help (the
/// user: where is Explosion? It was Hurricane Blast in the menu).
pub const RENAMED: [(u8, &str, &str); 8] = [
    (kh1::VORTEX, "Flash Step", "Ground combo: dash to a distant foe and strike."),
    (kh1::COUNTERATTACK, "Counterguard", "Counter: strike right after guarding."),
    (kh1::BLITZ, "Finishing Leap", "Finisher: leap and slam the ground."),
    (kh1::SLIDING_DASH, "Slide Dash", "Ground combo: dash at a distant foe."),
    (kh1::HURRICANE_BLAST, "Explosion", "Finisher: unleash a blast around you."),
    (kh1::RIPPLE_DRIVE, "Vicinity Break", "Ground combo: hit foes all around."),
    (kh1::STUN_IMPACT, "Guard Break", "Finisher: break through a foe's guard."),
    (kh1::GRAVITY_BREAK, "Aerial Finish", "Air finisher: a powerful spinning strike."),
];

/// Which entry of a full list gives up its place (its index), if any.
pub fn make_room(list: &[u8]) -> Option<usize> {
    let end = list.iter().position(|b| *b == 0).unwrap_or(list.len());
    let l = &list[..end];
    let ours = |b: u8| MENU.iter().any(|a| a.id == b & 0x7F);
    let off = |b: u8| b & 0x80 != 0 && !ours(b);
    // A spare copy that's off: of the ability with the most copies.
    let copies = |id: u8| l.iter().filter(|b| *b & 0x7F == id).count();
    if let Some(i) = (0..end).filter(|i| off(l[*i]) && copies(l[*i] & 0x7F) > 1).max_by_key(|i| (copies(l[*i] & 0x7F), *i)) {
        return Some(i);
    }
    for id in MAKE_ROOM {
        if let Some(i) = (0..end).rev().find(|i| off(l[*i]) && l[*i] & 0x7F == id) {
            return Some(i);
        }
    }
    None
}

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
    for ab in MENU {
        if let Effect::Modifier(n) = ab.effect {
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
        // (The user: one High Jump, LV3; one Glide, LV3, and one Superglide, MAX.)
        if equipped(s, kh1::HIGH_JUMP) > 0 {
            out.high_jump = 3;
        }
        if equipped(s, kh1::SUPERGLIDE) > 0 {
            out.glide = 4;
        } else if equipped(s, kh1::GLIDE) > 0 {
            out.glide = 3;
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
        if equipped(a, kh1::COMBO_MASTER) > 0 {
            out.combo_master = true;
        }
        let mut quick_run = 0;
        let mut aerial_dodge = 0;
        for ab in MENU {
            let list = match ab.list {
                List::Shared => s,
                List::Sora => a,
            };
            if equipped(list, ab.id) == 0 {
                continue;
            }
            match ab.effect {
                // (The user: both at LV3.)
                Effect::QuickRun => quick_run = 3,
                Effect::AerialDodge => aerial_dodge = 3,
                Effect::AerialRecovery => out.aerial_recovery = true,
                Effect::FinishingPlus => out.finishing_plus = 1,
                Effect::NegativeCombo => out.negative_combo = 1,
                Effect::Modifier(n) => {
                    if !out.equipped.contains(&n) {
                        out.equipped.push(n);
                    }
                }
            }
        }
        for (at, g) in GROWTH {
            if self.level < at {
                continue;
            }
            match g {
                Growth::QuickRun(l) if quick_run > 0 => quick_run = quick_run.max(l),
                Growth::AerialDodge(l) if aerial_dodge > 0 => aerial_dodge = aerial_dodge.max(l),
                Growth::DodgeRoll(l) if roll => out.dodge_roll = out.dodge_roll.max(l),
                _ => {}
            }
        }
        out.quick_run = quick_run;
        out.aerial_dodge = aerial_dodge;
        out
    }

    /// The menu abilities Sora's level has earned that his lists don't hold yet.
    pub fn grants(&self) -> Vec<MenuAbility> {
        MENU.iter()
            .filter(|ab| self.level >= ab.level)
            .filter(|ab| !has(if ab.list == List::Shared { &self.shared } else { &self.abilities }, ab.id))
            .copied()
            .collect()
    }
}

/// The AP Sora gets on top of KH1's by his level: each KH2 ability he has learned brings its
/// own AP, so taking the KH2 abilities costs none of KH1's.
pub fn bonus_ap(level: u8) -> u8 {
    MENU.iter().filter(|ab| level >= ab.level).map(|ab| ab.ap).sum()
}

/// One line for the log.
pub fn describe(a: &Abilities) -> String {
    let mut names: Vec<&str> = Vec::new();
    for (_, n, name) in MODIFIERS {
        if a.equipped.contains(&n) {
            names.push(name);
        }
    }
    for ab in MENU {
        if let Effect::Modifier(n) = ab.effect {
            if a.equipped.contains(&n) {
                names.push(ab.name);
            }
        }
    }
    format!(
        "high jump {}, quick run {}, aerial dodge {}, glide {}, dodge roll {}, guard {}, combo plus {}, air combo plus {}, finishing plus {}, negative combo {}, combo master {}, aerial recovery {}; modifiers [{}]",
        a.high_jump,
        a.quick_run,
        a.aerial_dodge,
        a.glide,
        a.dodge_roll,
        a.guard,
        a.combo_plus,
        a.air_combo_plus,
        a.finishing_plus,
        a.negative_combo,
        a.combo_master,
        a.aerial_recovery,
        names.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bonus_ap_follows_the_learned_abilities() {
        assert_eq!(bonus_ap(1), 0);
        assert_eq!(bonus_ap(4), 2);
        assert_eq!(bonus_ap(10), 4);
        assert_eq!(bonus_ap(50), MENU.iter().map(|ab| ab.ap).sum::<u8>());
        assert_eq!(bonus_ap(50), bonus_ap(100));
    }

    fn sora(level: u8, abilities: &[u8], shared: &[u8]) -> Kh1Sora {
        let mut a = abilities.to_vec();
        a.resize(48, 0);
        let mut s = shared.to_vec();
        s.resize(10, 0);
        Kh1Sora { level, abilities: a, shared: s }
    }

    #[test]
    fn one_of_each_only() {
        let hj = kh1::HIGH_JUMP;
        let list = [hj, kh1::GLIDE, hj, kh1::SUPERGLIDE, hj | 0x80, kh1::SUPERGLIDE | 0x80, kh1::SUPERGLIDE | 0x80, 66 | 0x80, 67 | 0x80, 0];
        let (out, removed) = one_only(&list);
        assert_eq!(out, vec![hj, kh1::GLIDE, kh1::SUPERGLIDE, 66 | 0x80, 67 | 0x80, 0, 0, 0, 0, 0]);
        assert_eq!(removed, vec![hj, hj, kh1::SUPERGLIDE, kh1::SUPERGLIDE]);
        // An equipped copy is kept over an earlier unequipped one.
        let (out, _) = one_only(&[kh1::SUPERGLIDE | 0x80, kh1::SUPERGLIDE, 0]);
        assert_eq!(out, vec![kh1::SUPERGLIDE, 0, 0]);
        // Nothing to do.
        assert!(one_only(&[hj, 66, 0]).1.is_empty());
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
        assert_eq!((a.dodge_roll, a.combo_plus, a.air_combo_plus, a.high_jump, a.glide), (1, 2, 1, 3, 4));
        let g = sora(1, &[], &[kh1::GLIDE]).kh2();
        assert_eq!(g.glide, 3);
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
    fn menu_abilities_work_when_equipped() {
        let a = sora(99, &[kh1::DODGE_ROLL, 68, 74, 69, 77 | 0x80, kh1::COMBO_MASTER], &[kh1::HIGH_JUMP, kh1::HIGH_JUMP, kh1::HIGH_JUMP, 66, 67]).kh2();
        assert_eq!((a.quick_run, a.aerial_dodge, a.dodge_roll, a.high_jump), (3, 3, 4, 3));
        assert!(a.aerial_recovery && a.combo_master && a.finishing_plus == 1);
        assert!(a.equipped.contains(&kh2::DODGE_SLASH) && !a.equipped.contains(&kh2::MAGNET_BURST));
        // Learned but off: nothing.
        let off = sora(99, &[68 | 0x80], &[66 | 0x80]).kh2();
        assert!(!off.aerial_recovery && off.quick_run == 0);
        // No KH1 Dodge Roll: no roll at any level.
        assert_eq!(sora(99, &[], &[]).kh2().dodge_roll, 0);
        // Quick Run is LV3 from the start.
        assert_eq!(sora(7, &[], &[66]).kh2().quick_run, 3);
        assert_eq!(sora(99, &[], &[67]).kh2().aerial_dodge, 3);
    }

    #[test]
    fn grants_follow_level_and_lists() {
        let g = sora(13, &[], &[]).grants();
        let ids: Vec<u8> = g.iter().map(|a| a.id).collect();
        assert_eq!(ids, vec![66, 67, 68, 69]);
        // Already learned (even unequipped): not again.
        let g = sora(13, &[68 | 0x80], &[66]).grants();
        assert_eq!(g.iter().map(|a| a.id).collect::<Vec<_>>(), vec![67, 69]);
        assert!(sora(1, &[], &[]).grants().is_empty());
    }

    #[test]
    fn menu_ids_are_distinct_and_new() {
        let mut ids: Vec<u8> = MENU.iter().map(|a| a.id).collect();
        ids.dedup();
        assert_eq!(ids.len(), MENU.len());
        assert!(ids.iter().all(|i| *i >= FIRST_ID && *i < 0x80));
        assert!(MENU.iter().all(|a| a.name.len() <= 24 && a.help.len() <= 60));
    }

    #[test]
    fn room_comes_from_spare_copies_then_the_least_missed() {
        // The user's level-51 list (full): four Tech Boosts off, two Treasure Magnets off, a
        // Critical Plus on and one off.
        let mut l = vec![10, 62, 25, 14 | 0x80, 24, 54, 60 | 0x80, 60 | 0x80, 8, 23, 13 | 0x80, 5 | 0x80, 53, 28 | 0x80, 60 | 0x80, 6, 26 | 0x80, 21 | 0x80, 5 | 0x80, 57, 11 | 0x80, 19, 6, 16 | 0x80, 60 | 0x80, 6, 59 | 0x80, 6, 18, 7, 20, 58, 65, 22, 12 | 0x80, 17, 56, 7, 8, 15 | 0x80, 55 | 0x80, 8 | 0x80, 27 | 0x80, 61 | 0x80, 68 | 0x80, 69 | 0x80, 70 | 0x80, 71 | 0x80];
        assert_eq!(l.len(), 48);
        let mut freed = Vec::new();
        while let Some(i) = make_room(&l) {
            freed.push(name(l[i]));
            l.remove(i);
            l.push(0);
            if freed.len() == 8 {
                break;
            }
        }
        // Spares first (three Tech Boosts, a Treasure Magnet and a Critical Plus, those with
        // the most copies first), then Encounter Plus, Cheer, Jackpot.
        let mut spares = freed[..5].to_vec();
        spares.sort();
        assert_eq!(spares, ["Critical Plus", "Tech Boost", "Tech Boost", "Tech Boost", "Treasure Magnet"]);
        assert_eq!(freed[5..], ["Cheer", "Jackpot", "Lucky Strike"]);
        // Never Guard (off here), Encounter Plus (the user), the specials, ours, or anything
        // equipped.
        assert!(l.contains(&(61 | 0x80)) && l.contains(&(21 | 0x80)) && l.contains(&(11 | 0x80)) && l.contains(&(68 | 0x80)) && l.contains(&10));
    }

    #[test]
    fn modes_parse() {
        assert_eq!(Mode::parse(" KH1 "), Some(Mode::Kh1));
        assert_eq!(Mode::parse("all"), Some(Mode::All));
        assert_eq!(Mode::parse("x"), None);
    }
}
