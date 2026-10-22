//! Phase 7: KH2's abilities from KH1's menu (`docs/abilities.md`).
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
//! Every id has one owner (`Owner`) and one meaning (the user, testing: "every base ability
//! should be vanilla kh1 while we add extra ability slots for our kh2 abilities"):
//! - KH1's ids 1..=65 are KH1's own abilities: KH1's name, help and effect (`KH1_ABILITIES`).
//!   A few have a clear KH2 meaning while KH2 moves Sora (`Side::Kh2`: High Jump, Glide,
//!   Superglide, Guard, Dodge Roll, Combo Plus, Air Combo Plus, Combo Master); the rest are hooks.
//! - KH2's abilities are ours, ids 66 up (`MENU`), in an extended copy of KH1's ability table
//!   (`win/menu.rs`), with KH2's AP and text. Each says what it does while KH1's own states move
//!   Sora (`MenuAbility::on_kh1`).
//!
//! Until the split (testing s261) KH1's combo modifiers stood for KH2's (Hurricane Blast was
//! Explosion, renamed in the menu). A save from then is moved over once (`migrate`).

use character_core::Abilities;

/// KH1 ability ids.
pub mod kh1 {
    pub const HIGH_JUMP: u8 = 1;
    pub const MERMAID_KICK: u8 = 2;
    pub const GLIDE: u8 = 3;
    pub const SUPERGLIDE: u8 = 4;
    /// Zantetsuken: taken out of Sora's list (the user: room for KH2's abilities).
    pub const ZANTETSUKEN: u8 = 59;
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
    /// KH1's last id (its table's 65 entries).
    pub const LAST: u8 = 65;
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

/// Our ids for KH2's abilities (`MENU`).
pub mod ours {
    pub const QUICK_RUN: u8 = 66;
    pub const AERIAL_DODGE: u8 = 67;
    pub const AERIAL_RECOVERY: u8 = 68;
    pub const DODGE_SLASH: u8 = 69;
    pub const HORIZONTAL_SLASH: u8 = 70;
    pub const NEGATIVE_COMBO: u8 = 71;
    pub const AERIAL_SPIRAL: u8 = 72;
    pub const UPPER_SLASH: u8 = 73;
    pub const FINISHING_PLUS: u8 = 74;
    pub const RETALIATING_SLASH: u8 = 75;
    pub const AERIAL_DIVE: u8 = 76;
    pub const MAGNET_BURST: u8 = 77;
    // The split (testing s261): KH2's modifiers that were KH1's ids before.
    pub const FLASH_STEP: u8 = 78;
    pub const AERIAL_SWEEP: u8 = 79;
    pub const COUNTERGUARD: u8 = 80;
    pub const FINISHING_LEAP: u8 = 81;
    pub const SLAPSHOT: u8 = 82;
    pub const SLIDE_DASH: u8 = 83;
    pub const EXPLOSION: u8 = 84;
    pub const VICINITY_BREAK: u8 = 85;
    pub const GUARD_BREAK: u8 = 86;
    pub const AERIAL_FINISH: u8 = 87;
    /// KH2's own Glide (t323): in the shared list in place of KH1's Glide and Superglide while
    /// the Glide style is KH2's (`glide_swap`).
    pub const GLIDE: u8 = 88;
}

/// KH1's modifiers and the KH2 one each stood for before the split (Hurricane Blast was
/// Explosion). They are separate abilities now (`MenuAbility::with`); `migrate` uses the pairs.
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

/// Whose ability an id is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    Kh1,
    Kh2,
}

/// A value of KH2's controller (`character_core::Abilities`) an equipped ability sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// `high_jump`: its growth level by Sora's level (`high_jump_level`).
    HighJump,
    /// `glide`: LV1 to LV3 by Sora's level (`glide_level`).
    Glide,
    /// `glide` MAX.
    GlideMax,
    /// KH2's own Glide (`ours::GLIDE`): LV1 to LV3 by Sora's level, MAX once Chernabog is
    /// beaten (KH1's Superglide point).
    Kh2Glide,
    /// `guard`, and KH2's Guard in `equipped`.
    Guard,
    /// `dodge_roll`: its growth level by Sora's level (`dodge_roll_level`).
    DodgeRoll,
    /// `combo_plus` / `air_combo_plus`: one per equipped copy.
    ComboPlus,
    AirComboPlus,
    ComboMaster,
    QuickRun,
    AerialDodge,
    AerialRecovery,
    FinishingPlus,
    NegativeCombo,
    /// A combo modifier: its KH2 ability number in `equipped`.
    Modifier(u16),
}

/// What an equipped ability does on one side: while KH1's own states move Sora (the move
/// styles at KH1, `docs/shared-states.md`), or while KH2's controller has him.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// KH1's own code, unmodded. On KH2's side: code that isn't about how he moves (stats,
    /// drops, MP, the HUD, the party, KH1's own commands), which KH1 runs whoever moves him
    /// (not checked one by one in the game).
    Kh1Own,
    /// A KH2 controller value, set while it is equipped.
    Kh2(Effect),
    /// KH1's states own Sora, and KH2's controller borrows him for this move (Aerial Dodge).
    Borrow,
    /// KH1's own combo (a combo style at KH1) reads it from `Abilities` (`win/combat.rs`, the
    /// KH1-style abilities work).
    Kh1Combo,
    /// No meaning mapped yet: a hook for later (the user: "kh2 sora could use exclusive kh1
    /// abilities while kh1 sora can use exclusive kh2 abilities").
    Hook,
}

/// A KH1 ability: KH1's own on KH1's side; `on_kh2` while KH2's controller has Sora.
#[derive(Clone, Copy, Debug)]
pub struct Kh1Ability {
    pub id: u8,
    pub name: &'static str,
    pub on_kh2: Side,
}

const fn k(id: u8, name: &'static str, on_kh2: Side) -> Kh1Ability {
    Kh1Ability { id, name, on_kh2 }
}

use Side::{Hook, Kh1Own, Kh2};

/// KH1's abilities by id (names as the Archipelago KH1 world lists them).
pub const KH1_ABILITIES: [Kh1Ability; 65] = [
    k(1, "High Jump", Kh2(Effect::HighJump)),
    k(2, "Mermaid Kick", Kh1Own),
    k(3, "Glide", Kh2(Effect::Glide)),
    k(4, "Superglide", Kh2(Effect::GlideMax)),
    k(5, "Treasure Magnet", Kh1Own),
    k(6, "Combo Plus", Kh2(Effect::ComboPlus)),
    k(7, "Air Combo Plus", Kh2(Effect::AirComboPlus)),
    k(8, "Critical Plus", Kh1Own),
    k(9, "Second Wind", Hook),
    k(10, "Scan", Kh1Own),
    k(11, "Sonic Blade", Kh1Own),
    k(12, "Ars Arcanum", Kh1Own),
    k(13, "Strike Raid", Kh1Own),
    k(14, "Ragnarok", Kh1Own),
    k(15, "Trinity Limit", Kh1Own),
    k(16, "Cheer", Kh1Own),
    k(17, "Vortex", Hook),
    k(18, "Aerial Sweep", Hook),
    k(19, "Counterattack", Hook),
    k(20, "Blitz", Hook),
    k(21, "Guard", Kh2(Effect::Guard)),
    k(22, "Dodge Roll", Kh2(Effect::DodgeRoll)),
    k(23, "MP Haste", Kh1Own),
    k(24, "MP Rage", Kh1Own),
    k(25, "Second Chance", Kh1Own),
    k(26, "Berserk", Kh1Own),
    k(27, "Jackpot", Kh1Own),
    k(28, "Lucky Strike", Kh1Own),
    // 29..=52: Donald's and Goofy's (and the guests') abilities.
    k(29, "Charge", Kh1Own),
    k(30, "Rocket", Kh1Own),
    k(31, "Tornado", Kh1Own),
    k(32, "MP Gift", Kh1Own),
    k(33, "Raging Boar", Kh1Own),
    k(34, "Asp's Bite", Kh1Own),
    k(35, "Healing Herb", Kh1Own),
    k(36, "Wind Armor", Kh1Own),
    k(37, "Crescent", Kh1Own),
    k(38, "Sandstorm", Kh1Own),
    k(39, "Applause!", Kh1Own),
    k(40, "Blazing Fury", Kh1Own),
    k(41, "Icy Terror", Kh1Own),
    k(42, "Bolts of Sorrow", Kh1Own),
    k(43, "Ghostly Scream", Kh1Own),
    k(44, "Hummingbird", Kh1Own),
    k(45, "Time-Out", Kh1Own),
    k(46, "Storm's Eye", Kh1Own),
    k(47, "Ferocious Lunge", Kh1Own),
    k(48, "Furious Bellow", Kh1Own),
    k(49, "Spiral Wave", Kh1Own),
    k(50, "Thunder Potion", Kh1Own),
    k(51, "Cure Potion", Kh1Own),
    k(52, "Aero Potion", Kh1Own),
    k(53, "Slapshot", Hook),
    k(54, "Sliding Dash", Hook),
    k(55, "Hurricane Blast", Hook),
    k(56, "Ripple Drive", Hook),
    k(57, "Stun Impact", Hook),
    k(58, "Gravity Break", Hook),
    k(59, "Zantetsuken", Kh1Own),
    k(60, "Tech Boost", Kh1Own),
    k(61, "Encounter Plus", Kh1Own),
    k(62, "Leaf Bracer", Kh1Own),
    k(63, "Evolution", Kh1Own),
    k(64, "EXP Zero", Kh1Own),
    k(65, "Combo Master", Kh2(Effect::ComboMaster)),
];

/// A KH2 ability in KH1's menu.
#[derive(Clone, Copy, Debug)]
pub struct MenuAbility {
    pub id: u8,
    pub name: &'static str,
    pub help: &'static str,
    /// AP (KH2's, from its item table); shared abilities take none.
    pub ap: u8,
    /// Sora's level it is learned at (`BOSS`: not by level).
    pub level: u8,
    pub list: List,
    /// The KH1 ability whose table entry it is made from (its kind and place in the menu).
    pub template: u8,
    /// What it does for KH2's controller.
    pub effect: Effect,
    /// The KH1 ability it is also learned with (0: none): KH1's namesake, which stood for it
    /// before the split, so it keeps that learn point.
    pub with: u8,
    /// While KH1's own states move Sora.
    pub on_kh1: Side,
}

#[allow(clippy::too_many_arguments)]
const fn m(id: u8, name: &'static str, help: &'static str, ap: u8, level: u8, template: u8, effect: Effect, with: u8, on_kh1: Side) -> MenuAbility {
    MenuAbility { id, name, help, ap, level, list: List::Sora, template, effect, with, on_kh1 }
}

/// `MenuAbility::level` for an ability not learned by level.
pub const BOSS: u8 = 0;

/// A boss's gift (the user's list; KH2 gives its moves for beating bosses too): the ability, once
/// the world's story progress byte (`progress`, `Kh1Sora::progress`) reaches the value it has
/// once the boss is beaten (as the Archipelago KH1 client reads them: gaithern's KH-1FM-AP-LUA,
/// MIT, after the RetroAchievements code notes).
#[derive(Clone, Copy, Debug)]
pub struct BossGift {
    pub boss: &'static str,
    /// The world's index in the progress bytes (0 Traverse Town, 1 Deep Jungle, 2 Olympus
    /// Coliseum, 3 Wonderland, 4 Agrabah, 5 Monstro, 6 Atlantica, 8 Halloween Town, 9 Neverland,
    /// 10 Hollow Bastion, 11 End of the World).
    pub world: usize,
    pub progress: u8,
    /// One of ours (`MENU`).
    pub ability: u8,
}

pub const BOSS_GIFTS: [BossGift; 9] = [
    BossGift { boss: "Sabor", world: 1, progress: 0x42, ability: ours::UPPER_SLASH },
    // Flash Step for Sabor too (the user; it was a level 42 gift).
    BossGift { boss: "Sabor", world: 1, progress: 0x42, ability: ours::FLASH_STEP },
    BossGift { boss: "Clayton", world: 1, progress: 0x56, ability: ours::AERIAL_SWEEP },
    BossGift { boss: "Jafar", world: 4, progress: 0x49, ability: ours::AERIAL_FINISH },
    BossGift { boss: "Pot Centipede", world: 4, progress: 0x35, ability: ours::GUARD_BREAK },
    BossGift { boss: "Dragon Maleficent", world: 10, progress: 0x6E, ability: ours::EXPLOSION },
    BossGift { boss: "Captain Hook", world: 9, progress: 0x56, ability: ours::MAGNET_BURST },
    BossGift { boss: "Cerberus", world: 2, progress: 0x25, ability: ours::AERIAL_DIVE },
    // Genie Jafar in Agrabah (the user; earlier Riku the first time): Agrabah's progress is 0x46
    // during Jafar and 0x53 after Genie Jafar's cutscene (KH-1FM-AP-LUA's warp fixes).
    BossGift { boss: "Genie Jafar", world: 4, progress: 0x53, ability: ours::AERIAL_DODGE },
];

/// The first id past KH1's own (65 entries in its table).
pub const FIRST_ID: u8 = 66;
/// The last id an ability can have (bit 7 of a list byte is "not equipped").
pub const MAX_ID: u8 = 0x7F;

use Side::{Borrow, Kh1Combo};

pub const MENU: [MenuAbility; 23] = [
    // (The user: Quick Run from level 12, growing with Sora; both with AP in Sora's own
    // list, not the shared one. AP as KH2's.)
    m(66, "Quick Run", "Dash swiftly along the ground.", 2, 12, kh1::DODGE_ROLL, Effect::QuickRun, 0, Hook),
    // (The user: Aerial Dodge for beating Genie Jafar, `BOSS_GIFTS`.)
    m(67, "Aerial Dodge", "Leap once more in midair.", 3, BOSS, kh1::DODGE_ROLL, Effect::AerialDodge, 0, Borrow),
    m(68, "Aerial Recovery", "Right yourself in midair after being knocked back.", 2, 4, kh1::COMBO_PLUS, Effect::AerialRecovery, 0, Hook),
    m(69, "Dodge Slash", "Ground combo: strike nearby foes all around.", 2, 10, kh1::SLAPSHOT, Effect::Modifier(kh2::DODGE_SLASH), 0, Kh1Combo),
    m(70, "Horizontal Slash", "Air combo: a powerful side slash with Square.", 2, 16, kh1::SLAPSHOT, Effect::Modifier(kh2::HORIZONTAL_SLASH), 0, Kh1Combo),
    m(71, "Negative Combo", "Shortens combos by one attack.", 2, 20, kh1::COMBO_PLUS, Effect::NegativeCombo, 0, Kh1Combo),
    m(72, "Aerial Spiral", "Air combo: spin into foes in midair.", 2, 25, kh1::SLAPSHOT, Effect::Modifier(kh2::AERIAL_SPIRAL), 0, Kh1Combo),
    m(73, "Upper Slash", "Combo: knock a foe into the air with Square.", 4, BOSS, kh1::SLAPSHOT, Effect::Modifier(kh2::UPPER_SLASH), 0, Kh1Combo),
    m(74, "Finishing Plus", "Adds a finishing move to the combo.", 5, 34, kh1::COMBO_PLUS, Effect::FinishingPlus, 0, Kh1Combo),
    m(75, "Retaliating Slash", "When knocked away, recover and strike back with Square.", 3, 43, kh1::SLAPSHOT, Effect::Modifier(kh2::RETALIATING_SLASH), 0, Hook),
    m(76, "Aerial Dive", "From the ground: leap at a far airborne foe.", 2, BOSS, kh1::SLAPSHOT, Effect::Modifier(kh2::AERIAL_DIVE), 0, Kh1Combo),
    m(77, "Magnet Burst", "Air combo finisher: pull in foes all around.", 3, BOSS, kh1::SLAPSHOT, Effect::Modifier(kh2::MAGNET_BURST), 0, Kh1Combo),
    // The split: KH2's modifiers that were KH1's ids, each placed after its KH1 namesake in the
    // menu and learned with it, and at the level or boss the mod gave that KH1 id before (the
    // user's: Counterguard at 22, Aerial Sweep at 28 or Clayton, Slide Dash at 30, Vicinity
    // Break at 36, Finishing Leap at 48; Flash Step for Sabor, Aerial Finish for Jafar, Guard
    // Break for Pot Centipede, Explosion for Dragon Maleficent). AP from KH2's item table.
    m(78, "Flash Step", "Ground combo: dash to a distant foe and strike.", 2, BOSS, kh1::VORTEX, Effect::Modifier(kh2::FLASH_STEP), kh1::VORTEX, Kh1Combo),
    m(79, "Aerial Sweep", "KH2's air combo: a spinning sweep at airborne foes.", 2, 28, kh1::AERIAL_SWEEP, Effect::Modifier(kh2::AERIAL_SWEEP), kh1::AERIAL_SWEEP, Kh1Combo),
    m(80, "Counterguard", "Counter: strike right after guarding.", 4, 22, kh1::COUNTERATTACK, Effect::Modifier(kh2::COUNTERGUARD), kh1::COUNTERATTACK, Hook),
    m(81, "Finishing Leap", "Finisher: end a combo with Square to leap and slam.", 5, 48, kh1::BLITZ, Effect::Modifier(kh2::FINISHING_LEAP), kh1::BLITZ, Kh1Combo),
    m(82, "Slapshot", "KH2's ground combo: rapid strikes at a nearby foe.", 2, BOSS, kh1::SLAPSHOT, Effect::Modifier(kh2::SLAPSHOT), kh1::SLAPSHOT, Kh1Combo),
    m(83, "Slide Dash", "Ground combo: dash at a distant foe.", 2, 30, kh1::SLIDING_DASH, Effect::Modifier(kh2::SLIDE_DASH), kh1::SLIDING_DASH, Kh1Combo),
    m(84, "Explosion", "Finisher: unleash a blast around you.", 3, BOSS, kh1::HURRICANE_BLAST, Effect::Modifier(kh2::EXPLOSION), kh1::HURRICANE_BLAST, Kh1Combo),
    m(85, "Vicinity Break", "Ground combo: hit foes all around.", 2, 36, kh1::RIPPLE_DRIVE, Effect::Modifier(kh2::VICINITY_BREAK), kh1::RIPPLE_DRIVE, Kh1Combo),
    m(86, "Guard Break", "Finisher: break through a foe's guard.", 3, BOSS, kh1::STUN_IMPACT, Effect::Modifier(kh2::GUARD_BREAK), kh1::STUN_IMPACT, Kh1Combo),
    m(87, "Aerial Finish", "Air finisher: a powerful spinning strike.", 3, BOSS, kh1::GRAVITY_BREAK, Effect::Modifier(kh2::AERIAL_FINISH), kh1::GRAVITY_BREAK, Kh1Combo),
    // KH2's own Glide (t323, the user: "if glide is set to kh2 both glide and super glide need to
    // be removed from the shared abilities list and only the original kh2 growth glide should
    // appear again as its own custom ability"): a shared ability (no AP), not learned on its own
    // (`earned` gives none): `glide_swap` puts it in place of KH1's Glide and Superglide at the
    // Glide style KH2, and back at KH1. Its KH1 side: KH2's glide is a borrow out of KH1's air.
    MenuAbility { id: 88, name: "Glide", help: "Glide through the air. Grows with Sora.", ap: 0, level: BOSS, list: List::Shared, template: kh1::GLIDE, effect: Effect::Kh2Glide, with: 0, on_kh1: Borrow },
];

/// Our Quick Run's ability id (`MENU`).
pub const QUICK_RUN_ID: u8 = ours::QUICK_RUN;

/// One row of the whole ability table: every id, its owner, and what it does on each side.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub id: u8,
    pub name: &'static str,
    pub owner: Owner,
    /// While KH1's own states move Sora.
    pub on_kh1: Side,
    /// While KH2's controller has him.
    pub on_kh2: Side,
}

/// The ability table: KH1's 65, then ours.
pub fn table() -> Vec<Row> {
    let mut rows: Vec<Row> = KH1_ABILITIES.iter().map(|a| Row { id: a.id, name: a.name, owner: Owner::Kh1, on_kh1: Kh1Own, on_kh2: a.on_kh2 }).collect();
    rows.extend(MENU.iter().map(|a| Row { id: a.id, name: a.name, owner: Owner::Kh2, on_kh1: a.on_kh1, on_kh2: Kh2(a.effect) }));
    rows
}

/// An id's row, if it has one.
pub fn row(id: u8) -> Option<Row> {
    let id = id & 0x7F;
    if (1..=kh1::LAST).contains(&id) {
        let a = KH1_ABILITIES[id as usize - 1];
        return Some(Row { id, name: a.name, owner: Owner::Kh1, on_kh1: Kh1Own, on_kh2: a.on_kh2 });
    }
    MENU.iter().find(|a| a.id == id).map(|a| Row { id, name: a.name, owner: Owner::Kh2, on_kh1: a.on_kh1, on_kh2: Kh2(a.effect) })
}

/// Quick Run's level by Sora's level (the user: it levels up with him, MAX 4): LV1 when learned.
pub const QUICK_RUN_LEVELS: [(u8, u8); 4] = [(12, 1), (20, 2), (32, 3), (45, 4)];

/// Quick Run's level at Sora's level (0 before it is learned).
pub fn quick_run_level(level: u8) -> u8 {
    QUICK_RUN_LEVELS.iter().filter(|(at, _)| level >= *at).map(|(_, l)| *l).max().unwrap_or(0)
}

/// Its name in the menu and the level-up box ("Quick Run LV2", "Quick Run MAX").
pub fn quick_run_name(l: u8) -> String {
    if l >= 4 {
        "Quick Run MAX".to_string()
    } else {
        format!("Quick Run LV{}", l.max(1))
    }
}

/// KH2's dodge roll level at Sora's level, with KH1's Dodge Roll equipped (`GROWTH`: LV2 at 19,
/// LV3 at 37, MAX at 60).
pub fn dodge_roll_level(level: u8) -> u8 {
    GROWTH.iter().filter_map(|(at, g)| match g {
        Growth::DodgeRoll(l) if level >= *at => Some(*l),
        _ => None,
    }).max().unwrap_or(1)
}

/// Its name in the "Ability Up" box ("Dodge Roll LV2", "Dodge Roll MAX"); KH1's menu keeps
/// KH1's own name.
pub fn dodge_roll_name(l: u8) -> String {
    if l >= 4 {
        "Dodge Roll MAX".to_string()
    } else {
        format!("Dodge Roll LV{}", l.max(1))
    }
}

/// KH2's High Jump levels past LV1 with KH1's High Jump equipped, by Sora's level (the user:
/// it levels up with him too, MAX 4): LV1 when Sora gets it, LV2 at 30, LV3 at 44, MAX at 58.
pub const HIGH_JUMP_LEVELS: [u8; 3] = [30, 44, 58];

/// Aerial Dodge's levels past LV1 (LV1 when Genie Jafar falls): LV2 at 30, LV3 at 42, MAX at 54
/// (the user: its growth from 30).
pub const AERIAL_DODGE_LEVELS: [u8; 3] = [30, 42, 54];

/// KH2's Glide levels past LV1 with KH1's Glide equipped (LV1 from KH1's Glide, Neverland): LV2
/// at 40, LV3 at 52. MAX is KH1's Superglide (Chernabog's reward).
pub const GLIDE_LEVELS: [u8; 2] = [40, 52];

/// Chernabog beaten: End of the World's progress (index 11) at 0x33, where KH1 gives Superglide
/// (as the Archipelago KH1 client reads it, gaithern's KH-1FM-AP-LUA, MIT).
pub const CHERNABOG: BossGift = BossGift { boss: "Chernabog", world: 11, progress: 0x33, ability: kh1::SUPERGLIDE };

/// KH2's glide level at Sora's level, with KH1's Glide equipped.
pub fn glide_level(level: u8) -> u8 {
    1 + GLIDE_LEVELS.iter().filter(|at| level >= **at).count() as u8
}

/// KH2's own Glide's level (`ours::GLIDE`): by Sora's level, MAX once Chernabog is beaten.
pub fn kh2_glide_level(level: u8, progress: &[u8]) -> u8 {
    if beaten(progress, &CHERNABOG) {
        4
    } else {
        glide_level(level)
    }
}

/// The shared list for the Glide style (t323): at KH2 (`kh2`), KH1's Glide and Superglide taken
/// out and KH2's own Glide (`ours::GLIDE`) in the place of the first of them, on if either was;
/// at KH1, KH2's Glide taken out and KH1's Glide back in its place (on as it was), with
/// Superglide too once Chernabog is beaten (on with Glide). The list keeps its length; what
/// changed, for the log.
pub fn glide_swap(shared: &[u8], kh2: bool, progress: &[u8]) -> (Vec<u8>, Vec<String>) {
    let end = shared.iter().position(|b| *b == 0).unwrap_or(shared.len());
    let mut l: Vec<u8> = shared[..end].to_vec();
    let mut lines = Vec::new();
    let at = |l: &[u8], ids: &[u8]| l.iter().position(|b| ids.contains(&(b & 0x7F)));
    let on = |l: &[u8], ids: &[u8]| l.iter().any(|b| ids.contains(b));
    if kh2 {
        let kh1_ids = [kh1::GLIDE, kh1::SUPERGLIDE];
        if let Some(i) = at(&l, &kh1_ids) {
            let was_on = on(&l, &kh1_ids);
            let had = has(&l, ours::GLIDE);
            let names: Vec<&str> = [(kh1::GLIDE, "Glide"), (kh1::SUPERGLIDE, "Superglide")].iter().filter(|(id, _)| has(&l, *id)).map(|(_, n)| *n).collect();
            l.retain(|b| !kh1_ids.contains(&(b & 0x7F)));
            if !had {
                l.insert(i.min(l.len()), if was_on { ours::GLIDE } else { ours::GLIDE | 0x80 });
            }
            lines.push(format!(
                "Glide style KH2: KH1's {} taken out of the shared abilities{}",
                names.join(" and "),
                if had { String::new() } else { format!(", KH2's Glide in its place ({})", if was_on { "on" } else { "off" }) }
            ));
        }
    } else if let Some(i) = at(&l, &[ours::GLIDE]) {
        let was_on = l[i] & 0x80 == 0;
        l.remove(i);
        let bit = if was_on { 0 } else { 0x80 };
        let mut back = Vec::new();
        if !has(&l, kh1::GLIDE) {
            l.insert(i.min(l.len()), kh1::GLIDE | bit);
            back.push("Glide");
        }
        if beaten(progress, &CHERNABOG) && !has(&l, kh1::SUPERGLIDE) && l.len() < shared.len() {
            let j = l.iter().position(|b| b & 0x7F == kh1::GLIDE).map_or(l.len(), |j| j + 1);
            l.insert(j, kh1::SUPERGLIDE | bit);
            back.push("Superglide");
        }
        lines.push(format!("Glide style KH1: KH2's Glide taken out of the shared abilities, KH1's {} back ({})", if back.is_empty() { "own".to_string() } else { back.join(" and ") }, if was_on { "on" } else { "off" }));
    }
    l.truncate(shared.len());
    l.resize(shared.len(), 0);
    (l, lines)
}

pub fn glide_name(l: u8) -> String {
    level_name("Glide", l)
}

fn grown(level: u8, steps: &[u8; 3]) -> u8 {
    1 + steps.iter().filter(|at| level >= **at).count() as u8
}

/// High Jump's level at Sora's level, once he has it.
pub fn high_jump_level(level: u8) -> u8 {
    grown(level, &HIGH_JUMP_LEVELS)
}

/// Aerial Dodge's level at Sora's level, once he has learned it.
pub fn aerial_dodge_level(level: u8) -> u8 {
    grown(level, &AERIAL_DODGE_LEVELS)
}

/// A growing ability's name with its level ("High Jump LV2", "High Jump MAX").
pub fn level_name(name: &str, l: u8) -> String {
    if l >= 4 {
        format!("{name} MAX")
    } else {
        format!("{name} LV{}", l.max(1))
    }
}

pub fn high_jump_name(l: u8) -> String {
    level_name("High Jump", l)
}

pub fn aerial_dodge_name(l: u8) -> String {
    level_name("Aerial Dodge", l)
}

/// Growth levels past the first, by Sora's level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Growth {
    QuickRun(u8),
    AerialDodge(u8),
    /// The dodge roll's level, with KH1's Dodge Roll equipped.
    DodgeRoll(u8),
}

/// (Quick Run, Aerial Dodge and High Jump have their own tables above.)
pub const GROWTH: [(u8, Growth); 3] = [(19, Growth::DodgeRoll(2)), (37, Growth::DodgeRoll(3)), (60, Growth::DodgeRoll(4))];

/// Shared abilities Sora keeps only one of (the user: one High Jump, one Glide, one
/// Superglide).
pub const ONE_ONLY: [u8; 4] = [kh1::HIGH_JUMP, kh1::GLIDE, kh1::SUPERGLIDE, ours::GLIDE];

/// The shared list with the spare copies of `ONE_ONLY` taken out (an equipped copy is the one
/// kept), and the ids taken out. The list keeps its length (zeros at the end).
pub fn one_only(list: &[u8]) -> (Vec<u8>, Vec<u8>) {
    keep_one(list, &ONE_ONLY)
}

/// `list` with the spare copies of `ids` taken out (an equipped copy is the one kept), and the
/// ids taken out. The list keeps its length (zeros at the end).
pub fn keep_one(list: &[u8], ids: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let end = list.iter().position(|b| *b == 0).unwrap_or(list.len());
    let mut keep: Vec<Option<usize>> = vec![None; ids.len()];
    for (k, id) in ids.iter().enumerate() {
        let at = |on: bool| list[..end].iter().position(|b| b & 0x7F == *id && (b & 0x80 == 0) == on);
        keep[k] = at(true).or_else(|| at(false));
    }
    let mut out = Vec::with_capacity(list.len());
    let mut removed = Vec::new();
    for (i, b) in list[..end].iter().enumerate() {
        match ids.iter().position(|id| *id == b & 0x7F) {
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

/// An ability's name (KH1's or ours).
pub fn name(id: u8) -> String {
    row(id).map_or(format!("ability {}", id & 0x7F), |r| r.name.to_string())
}

/// The user: KH2's abilities come first. When Sora's list (48, KH1's most) is full, KH1
/// abilities that are learned but off make room, least missed first: a spare copy of one he
/// has twice or more, then these, in order. Nothing equipped, nothing of ours, and not Guard,
/// Dodge Roll, the specials or the combo modifiers.
pub const MAKE_ROOM: [u8; 14] = [
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
    // A full list of a finished game (the user's level 100 save: no Aerial Dodge, the list full
    // of KH1's, most of them on): any other KH1 ability that's off, but never Guard, Dodge Roll,
    // Combo Plus / Air Combo Plus, the specials or the combo modifiers...
    if let Some(i) = (0..end).rev().find(|i| off(l[*i]) && !KEEP.contains(&(l[*i] & 0x7F))) {
        return Some(i);
    }
    // ... then the least missed even when on.
    for id in MAKE_ROOM {
        if let Some(i) = (0..end).rev().find(|i| !ours(l[*i]) && l[*i] & 0x7F == id) {
            return Some(i);
        }
    }
    None
}

/// KH1 abilities never given up for room: Guard, Dodge Roll, Combo Plus, Air Combo Plus, the
/// specials (Sonic Blade, Ars Arcanum, Strike Raid, Ragnarok, Trinity Limit), KH1's combo
/// modifiers (its own combo uses them) and EXP Zero.
const KEEP: [u8; 20] = [21, 22, 6, 7, 11, 12, 13, 14, 15, 17, 18, 19, 20, 53, 54, 55, 56, 57, 58, 64];

/// Sora's side of KH1's save block.
#[derive(Clone, Debug, PartialEq)]
pub struct Kh1Sora {
    pub level: u8,
    /// The 48 ability bytes (`+0x44`).
    pub abilities: Vec<u8>,
    /// The shared abilities (`+0x599`).
    pub shared: Vec<u8>,
    /// The worlds' story progress bytes (save `+0x1504`, 12).
    pub progress: Vec<u8>,
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
            // ("remove": before uninstalling, the KH2 abilities come out of the save, as in test.)
            "test" | "max" | "remove" | "uninstall" => Some(Mode::Test),
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
    let mut equipped: Vec<u16> = Vec::new();
    for ab in MENU {
        if let Effect::Modifier(n) = ab.effect {
            if !equipped.contains(&n) {
                equipped.push(n);
            }
        }
    }
    equipped.push(kh2::GUARD);
    Abilities { guard: true, aerial_recovery: true, combo_master: true, finishing_plus: 1, equipped, ..Abilities::max_growth() }
}

/// Which of Sora's lists an id lives in: KH1's shared abilities are ids 1..=4, the rest his own.
fn list_of(id: u8) -> List {
    if (kh1::HIGH_JUMP..=kh1::SUPERGLIDE).contains(&id) {
        List::Shared
    } else {
        List::Sora
    }
}

impl Kh1Sora {
    fn list(&self, l: List) -> &[u8] {
        match l {
            List::Shared => &self.shared,
            List::Sora => &self.abilities,
        }
    }

    /// One equipped ability's effect on KH2's controller (`copies`: how many are equipped).
    fn apply(&self, out: &mut Abilities, e: Effect, copies: u8) {
        let push = |out: &mut Abilities, n: u16| {
            if !out.equipped.contains(&n) {
                out.equipped.push(n);
            }
        };
        match e {
            // (The user: one High Jump, growing with Sora to MAX; Glide growing to LV3, MAX with
            // Superglide; the dodge roll and Quick Run / Aerial Dodge grow too.)
            Effect::HighJump => out.high_jump = out.high_jump.max(high_jump_level(self.level)),
            Effect::Glide => out.glide = out.glide.max(glide_level(self.level)),
            Effect::GlideMax => out.glide = 4,
            Effect::Kh2Glide => out.glide = out.glide.max(kh2_glide_level(self.level, &self.progress)),
            Effect::Guard => {
                out.guard = true;
                push(out, kh2::GUARD);
            }
            Effect::DodgeRoll => out.dodge_roll = out.dodge_roll.max(dodge_roll_level(self.level)),
            Effect::ComboPlus => out.combo_plus = out.combo_plus.saturating_add(copies),
            Effect::AirComboPlus => out.air_combo_plus = out.air_combo_plus.saturating_add(copies),
            Effect::ComboMaster => out.combo_master = true,
            Effect::QuickRun => out.quick_run = out.quick_run.max(quick_run_level(self.level).max(1)),
            Effect::AerialDodge => out.aerial_dodge = out.aerial_dodge.max(aerial_dodge_level(self.level)),
            Effect::AerialRecovery => out.aerial_recovery = true,
            Effect::FinishingPlus => out.finishing_plus = 1,
            Effect::NegativeCombo => out.negative_combo = 1,
            Effect::Modifier(n) => push(out, n),
        }
    }

    /// KH2's controller's abilities from KH1's menu and Sora's level: every equipped ability
    /// whose KH2 side is a KH2 value (`Side::Kh2`), KH1's and ours.
    pub fn kh2(&self) -> Abilities {
        let mut out = Abilities { guard: false, one_swing_on_miss: true, ..Abilities::default() };
        for a in KH1_ABILITIES {
            if let Kh2(e) = a.on_kh2 {
                let n = equipped(self.list(list_of(a.id)), a.id);
                if n > 0 {
                    self.apply(&mut out, e, n);
                }
            }
        }
        for ab in MENU {
            let n = equipped(self.list(ab.list), ab.id);
            if n > 0 {
                self.apply(&mut out, ab.effect, n);
            }
        }
        out
    }

    /// Equipped KH1 abilities that mean nothing yet while KH2's controller has Sora (hooks).
    pub fn kh1_hooks(&self) -> Vec<u8> {
        let mut out: Vec<u8> = KH1_ABILITIES.iter().filter(|a| a.on_kh2 == Hook && equipped(self.list(list_of(a.id)), a.id) > 0).map(|a| a.id).collect();
        out.dedup();
        out
    }

    /// Equipped KH2 abilities that mean nothing yet while KH1's own states move Sora (hooks).
    pub fn kh2_hooks(&self) -> Vec<u8> {
        MENU.iter().filter(|a| a.on_kh1 == Hook && equipped(self.list(a.list), a.id) > 0).map(|a| a.id).collect()
    }

    /// Why Sora has earned `ab` now, if he has: his level, a boss beaten, or KH1's namesake
    /// learned (`MenuAbility::with`).
    pub fn earned(&self, ab: &MenuAbility) -> Option<String> {
        if ab.level != BOSS && self.level >= ab.level {
            return Some(format!("level {}", ab.level));
        }
        if let Some(g) = BOSS_GIFTS.iter().find(|g| g.ability == ab.id && beaten(&self.progress, g)) {
            return Some(format!("{} beaten", g.boss));
        }
        if ab.with != 0 && has(&self.abilities, ab.with) {
            return Some(format!("with KH1's {}", name(ab.with)));
        }
        None
    }

    /// The menu abilities Sora has earned that his lists don't hold yet: (why, ability).
    pub fn grants(&self) -> Vec<(String, MenuAbility)> {
        MENU.iter().filter(|ab| !has(self.list(ab.list), ab.id)).filter_map(|ab| self.earned(ab).map(|why| (why, *ab))).collect()
    }
}

/// Whether a gift's boss is beaten.
pub fn beaten(progress: &[u8], g: &BossGift) -> bool {
    progress.get(g.world).is_some_and(|p| *p >= g.progress)
}

/// The AP Sora gets on top of KH1's: each KH2 ability he has earned (`Kh1Sora::earned`) brings
/// its own AP, so taking the KH2 abilities costs none of KH1's.
pub fn bonus_ap(sora: &Kh1Sora) -> u8 {
    MENU.iter().filter(|ab| sora.earned(ab).is_some()).map(|ab| ab.ap as u32).sum::<u32>().min(255) as u8
}

/// The save's ability-layout mark: a u16 at save `+0x48A` (the spare bytes after the ten party
/// records, `docs/kh1-equipment.md` section 5; Valor's Keyblade uses `+0x488`). `LAYOUT_SPLIT`
/// once the save has been moved over to the split ids (`migrate`), or was never on the old ones.
pub const LAYOUT_OFF: usize = 0x48A;
pub const LAYOUT_SPLIT: u16 = 0x4B02;

/// Whether a save holds the old layout's ids: any of ours from before the split (66..=77), so
/// KH1's modifiers in it stood for KH2's.
pub fn old_layout(s: &Kh1Sora) -> bool {
    (ours::QUICK_RUN..=ours::MAGNET_BURST).any(|id| has(&s.abilities, id) || has(&s.shared, id))
}

/// A save moved over to the split ids.
#[derive(Clone, Debug, PartialEq)]
pub struct Migrated {
    pub abilities: Vec<u8>,
    pub shared: Vec<u8>,
    /// What changed, for the log and `kh1-kh2-removed.txt`.
    pub lines: Vec<String>,
    /// Lines of KH1 abilities given up for room (`(id n)`, as `kh1-kh2-removed.txt` takes them).
    pub removed: Vec<String>,
}

fn push(list: &mut [u8], byte: u8) -> bool {
    match list.iter().position(|b| *b == 0) {
        Some(i) => {
            list[i] = byte;
            true
        }
        None => false,
    }
}

/// Moves a save of the old layout over (once, by `LAYOUT_OFF`):
/// - each KH1 modifier Sora has (Hurricane Blast...) brings its KH2 ability (Explosion...), on if
///   KH1's was on, and KH1's copies are turned off (the same move stays on under KH2, and KH1's
///   AP is freed; KH1's own ability stays learned);
/// - Superglide, turned into Glide or taken out by the old build, comes back if Chernabog is
///   beaten (on if Glide is).
pub fn migrate(s: &Kh1Sora) -> Migrated {
    let mut a = s.abilities.clone();
    let mut sh = s.shared.clone();
    let mut lines = Vec::new();
    let mut removed = Vec::new();
    for ab in MENU.iter().filter(|ab| ab.with != 0) {
        if !has(&a, ab.with) || has(&a, ab.id) {
            continue;
        }
        let on = equipped(&a, ab.with) > 0;
        let byte = if on { ab.id } else { ab.id | 0x80 };
        let mut done = push(&mut a, byte);
        if !done {
            if let Some(i) = make_room(&a) {
                let gone = a.remove(i);
                a.push(0);
                removed.push(format!("removed {} (id {}) for {}", name(gone), gone & 0x7F, ab.name));
                done = push(&mut a, byte);
            }
        }
        if !done {
            lines.push(format!("{}: no room for {} (learned again when there's room)", name(ab.with), ab.name));
            continue;
        }
        if on {
            for b in a.iter_mut().take_while(|b| **b != 0) {
                if *b == ab.with {
                    *b |= 0x80;
                }
            }
        }
        lines.push(format!("KH1's {} is KH1's again: {} is its own ability now ({}){}", name(ab.with), ab.name, if on { "on" } else { "off" }, if on { format!(", {} turned off", name(ab.with)) } else { String::new() }));
    }
    if beaten(&s.progress, &CHERNABOG) && !has(&sh, kh1::SUPERGLIDE) {
        let on = equipped(&sh, kh1::GLIDE) > 0;
        if push(&mut sh, if on { kh1::SUPERGLIDE } else { kh1::SUPERGLIDE | 0x80 }) {
            lines.push(format!("Superglide given back (Chernabog beaten; {})", if on { "on" } else { "off" }));
        }
    }
    Migrated { abilities: a, shared: sh, lines, removed }
}

/// One line for the log.
pub fn describe(a: &Abilities) -> String {
    let mut names: Vec<&str> = Vec::new();
    for ab in MENU {
        if let Effect::Modifier(n) = ab.effect {
            if a.equipped.contains(&n) && !names.contains(&ab.name) {
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
    /// A tester's finished level 100 save (its log): 48 abilities, every one that's off is kept
    /// (Combo Plus, the specials, the modifiers, ours), so room comes from the least missed even
    /// though on (EXP Zero kept, the user): Cheer.
    #[test]
    fn full_finished_save_makes_room() {
        let raw: [u8; 48] = [64, 22, 6 | 0x80, 7 | 0x80, 53, 54, 17 | 0x80, 10, 61, 26, 15 | 0x80, 21, 19, 8, 8, 18, 20 | 0x80, 55, 57 | 0x80, 59, 5, 5, 27, 28, 65, 56, 23, 24, 24, 11 | 0x80, 13, 12, 14 | 0x80, 25, 62, 8, 58, 66 | 0x80, 68 | 0x80, 69 | 0x80, 70 | 0x80, 71 | 0x80, 72 | 0x80, 74 | 0x80, 75 | 0x80, 73 | 0x80, 77 | 0x80, 76 | 0x80];
        let i = make_room(&raw).expect("room");
        assert_eq!(raw[i] & 0x7F, 10);
    }

    use super::*;

    fn sora(level: u8, abilities: &[u8], shared: &[u8]) -> Kh1Sora {
        let mut a = abilities.to_vec();
        a.resize(48, 0);
        let mut s = shared.to_vec();
        s.resize(10, 0);
        Kh1Sora { level, abilities: a, shared: s, progress: vec![0; 12] }
    }

    #[test]
    fn bonus_ap_follows_the_learned_abilities() {
        assert_eq!(bonus_ap(&sora(1, &[], &[])), 0);
        assert_eq!(bonus_ap(&sora(4, &[], &[])), 2);
        assert_eq!(bonus_ap(&sora(10, &[], &[])), 4);
        let mut all = sora(50, &[], &[]);
        all.progress = vec![0xFF; 12];
        let mut done = sora(100, &[], &[]);
        done.progress = vec![0xFF; 12];
        // Slapshot's only learn point is KH1's own Slapshot.
        let slapshot = MENU.iter().find(|a| a.id == ours::SLAPSHOT).unwrap().ap;
        assert_eq!(bonus_ap(&all), MENU.iter().map(|ab| ab.ap).sum::<u8>() - slapshot);
        assert_eq!(bonus_ap(&all), bonus_ap(&done));
        // Boss gifts' AP only once beaten: Upper Slash, Aerial Dive, Magnet Burst, Aerial Dodge,
        // Flash Step, Explosion, Guard Break, Aerial Finish (Aerial Sweep comes at 28 too).
        assert_eq!(bonus_ap(&done), bonus_ap(&sora(100, &[], &[])) + 4 + 2 + 3 + 3 + 2 + 3 + 3 + 3);
        // KH1's namesake brings it too.
        assert_eq!(bonus_ap(&sora(1, &[kh1::SLAPSHOT | 0x80, kh1::HURRICANE_BLAST], &[])), 2 + 3);
    }

    #[test]
    fn glide_swaps_with_the_style() {
        let chern = |p: &mut Vec<u8>| p[11] = 0x33;
        let mut pr = vec![0u8; 12];
        // KH2: Glide (on) and Superglide (off) out, KH2's Glide in Glide's place, on.
        let sh = [kh1::HIGH_JUMP, kh1::GLIDE, 2, kh1::SUPERGLIDE | 0x80, 0, 0, 0, 0, 0, 0];
        let (k2, lines) = glide_swap(&sh, true, &pr);
        assert_eq!(k2, vec![kh1::HIGH_JUMP, ours::GLIDE, 2, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(lines.len(), 1);
        // Again: nothing to do.
        assert_eq!(glide_swap(&k2, true, &pr), (k2.clone(), Vec::new()));
        // KH1's own Superglide given later (Chernabog) while at KH2: taken out, KH2's Glide kept.
        let mut later = k2.clone();
        later[3] = kh1::SUPERGLIDE;
        assert_eq!(glide_swap(&later, true, &pr).0, k2);
        // Back to KH1 before Chernabog: Glide only.
        let (k1, _) = glide_swap(&k2, false, &pr);
        assert_eq!(k1, vec![kh1::HIGH_JUMP, kh1::GLIDE, 2, 0, 0, 0, 0, 0, 0, 0]);
        // After Chernabog: Superglide too, on as Glide was.
        chern(&mut pr);
        let (k1, _) = glide_swap(&k2, false, &pr);
        assert_eq!(k1, vec![kh1::HIGH_JUMP, kh1::GLIDE, kh1::SUPERGLIDE, 2, 0, 0, 0, 0, 0, 0]);
        let (k1, _) = glide_swap(&[ours::GLIDE | 0x80, 0, 0], false, &pr);
        assert_eq!(k1, vec![kh1::GLIDE | 0x80, kh1::SUPERGLIDE | 0x80, 0]);
        // Neither Glide: nothing either way.
        assert_eq!(glide_swap(&[kh1::HIGH_JUMP, 0], true, &pr).0, vec![kh1::HIGH_JUMP, 0]);
        assert_eq!(glide_swap(&[kh1::HIGH_JUMP, 0], false, &pr).0, vec![kh1::HIGH_JUMP, 0]);
        // A full list keeps its length.
        let full = [kh1::GLIDE, kh1::SUPERGLIDE, 1, 1, 1, 1, 1, 1, 1, 1];
        assert_eq!(glide_swap(&full, true, &pr).0.len(), 10);
        let back = glide_swap(&glide_swap(&full, true, &pr).0, false, &pr).0;
        assert_eq!(back.len(), 10);
        assert!(has(&back, kh1::GLIDE));
    }

    #[test]
    fn kh2_glide_grows_to_max_at_chernabog() {
        let mut s = sora(39, &[], &[ours::GLIDE]);
        assert_eq!(s.kh2().glide, 1);
        s.level = 52;
        assert_eq!(s.kh2().glide, 3);
        s.progress[11] = 0x33;
        assert_eq!(s.kh2().glide, 4);
        // Off: nothing; and never learned on its own.
        assert_eq!(sora(60, &[], &[ours::GLIDE | 0x80]).kh2().glide, 0);
        assert!(!sora(99, &[], &[kh1::GLIDE]).grants().iter().any(|(_, a)| a.id == ours::GLIDE));
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
            &[kh1::GUARD, kh1::DODGE_ROLL, kh1::COMBO_PLUS, kh1::COMBO_PLUS, kh1::AIR_COMBO_PLUS, kh1::VORTEX, kh1::HURRICANE_BLAST, kh1::SLAPSHOT | 0x80],
            &[kh1::HIGH_JUMP, kh1::HIGH_JUMP, kh1::GLIDE],
        )
        .kh2();
        assert!(a.guard);
        assert_eq!((a.dodge_roll, a.combo_plus, a.air_combo_plus, a.high_jump, a.glide), (1, 2, 1, 1, 1));
        // KH1's modifiers are KH1's own: nothing for KH2 (hooks).
        assert_eq!(a.equipped, vec![kh2::GUARD]);
        // Glide grows with Sora (LV2 at 40, LV3 at 52); KH1's Superglide is MAX.
        assert_eq!(sora(39, &[], &[kh1::GLIDE]).kh2().glide, 1);
        assert_eq!(sora(40, &[], &[kh1::GLIDE]).kh2().glide, 2);
        assert_eq!(sora(99, &[], &[kh1::GLIDE]).kh2().glide, 3);
        assert_eq!(sora(1, &[], &[kh1::GLIDE, kh1::SUPERGLIDE]).kh2().glide, 4);
        assert_eq!(sora(1, &[], &[kh1::GLIDE, kh1::SUPERGLIDE | 0x80]).kh2().glide, 1);
    }

    #[test]
    fn kh2_modifiers_are_their_own_ids() {
        let s = sora(1, &[ours::EXPLOSION, ours::FLASH_STEP, kh1::HURRICANE_BLAST, ours::GUARD_BREAK | 0x80], &[]);
        let a = s.kh2();
        assert!(a.equipped.contains(&kh2::EXPLOSION) && a.equipped.contains(&kh2::FLASH_STEP));
        assert!(!a.equipped.contains(&kh2::GUARD_BREAK));
        // Hurricane Blast is KH1's (a hook on KH2's side).
        assert_eq!(s.kh1_hooks(), vec![kh1::HURRICANE_BLAST]);
        assert_eq!(name(kh1::HURRICANE_BLAST), "Hurricane Blast");
        assert_eq!(name(ours::EXPLOSION), "Explosion");
    }

    #[test]
    fn table_has_every_id_once() {
        let t = table();
        let mut ids: Vec<u8> = t.iter().map(|r| r.id).collect();
        ids.sort_unstable();
        let want: Vec<u8> = (1..=kh1::LAST).chain(MENU.iter().map(|a| a.id)).collect();
        assert_eq!(ids, want);
        assert!(t.iter().all(|r| (r.owner == Owner::Kh1) == (r.id <= kh1::LAST)));
        assert!(KH1_ABILITIES.iter().enumerate().all(|(i, a)| a.id as usize == i + 1));
        // KH1's side of a KH1 ability is KH1's own; KH2's side of ours is a KH2 value.
        assert!(t.iter().all(|r| r.owner == Owner::Kh2 || r.on_kh1 == Side::Kh1Own));
        assert!(t.iter().all(|r| r.owner == Owner::Kh1 || matches!(r.on_kh2, Side::Kh2(_))));
        // No KH2 modifier on a KH1 id any more.
        assert!(KH1_ABILITIES.iter().all(|a| !matches!(a.on_kh2, Side::Kh2(Effect::Modifier(_)))));
        // Each KH1 modifier has its KH2 ability, learned with it.
        for (k1, k2, _) in MODIFIERS {
            assert!(MENU.iter().any(|a| a.with == k1 && a.effect == Effect::Modifier(k2)), "{k1}");
        }
    }

    #[test]
    fn list_ends_at_first_empty() {
        let mut list = vec![kh1::GUARD, 0, kh1::DODGE_ROLL];
        list.resize(48, 0);
        assert_eq!(equipped(&list, kh1::DODGE_ROLL), 0);
    }

    #[test]
    fn menu_abilities_work_when_equipped() {
        let a = sora(99, &[kh1::DODGE_ROLL, 68, 74, 69, 77 | 0x80, kh1::COMBO_MASTER, 66, 67], &[kh1::HIGH_JUMP, kh1::HIGH_JUMP, kh1::HIGH_JUMP]).kh2();
        assert_eq!((a.quick_run, a.aerial_dodge, a.dodge_roll, a.high_jump), (4, 4, 4, 4));
        assert!(a.aerial_recovery && a.combo_master && a.finishing_plus == 1);
        assert!(a.equipped.contains(&kh2::DODGE_SLASH) && !a.equipped.contains(&kh2::MAGNET_BURST));
        // Learned but off: nothing.
        let off = sora(99, &[68 | 0x80, 66 | 0x80], &[]).kh2();
        assert!(!off.aerial_recovery && off.quick_run == 0);
        // No KH1 Dodge Roll: no roll at any level.
        assert_eq!(sora(99, &[], &[]).kh2().dodge_roll, 0);
        // Quick Run grows with Sora (LV1 at 12 .. MAX at 45).
        assert_eq!(sora(12, &[66], &[]).kh2().quick_run, 1);
        assert_eq!(sora(32, &[66], &[]).kh2().quick_run, 3);
        assert_eq!(quick_run_name(4), "Quick Run MAX");
        assert_eq!((dodge_roll_level(18), dodge_roll_level(19), dodge_roll_level(60)), (1, 2, 4));
        // High Jump and Aerial Dodge grow too (the user).
        assert_eq!(sora(99, &[67], &[]).kh2().aerial_dodge, 4);
        assert_eq!((aerial_dodge_level(29), aerial_dodge_level(30), aerial_dodge_level(42), aerial_dodge_level(54)), (1, 2, 3, 4));
        assert_eq!((high_jump_level(29), high_jump_level(30), high_jump_level(44), high_jump_level(58)), (1, 2, 3, 4));
        assert_eq!(sora(30, &[], &[kh1::HIGH_JUMP]).kh2().high_jump, 2);
        assert_eq!((high_jump_name(3), aerial_dodge_name(4)), ("High Jump LV3".to_string(), "Aerial Dodge MAX".to_string()));
    }

    #[test]
    fn grants_follow_level_bosses_and_namesakes() {
        let ids = |s: &Kh1Sora| s.grants().iter().map(|(_, a)| a.id).collect::<Vec<u8>>();
        // (Aerial Dodge is Genie Jafar's gift, not a level's.)
        assert_eq!(ids(&sora(13, &[], &[kh1::HIGH_JUMP])), vec![66, 68, 69]);
        // Already learned (even unequipped): not again.
        assert_eq!(ids(&sora(13, &[68 | 0x80, 66 | 0x80], &[kh1::HIGH_JUMP])), vec![69]);
        // Genie Jafar beaten: Aerial Dodge.
        let mut jafar = sora(30, &[], &[]);
        jafar.progress[4] = 0x52;
        assert!(!ids(&jafar).contains(&ours::AERIAL_DODGE));
        jafar.progress[4] = 0x53;
        assert!(ids(&jafar).contains(&ours::AERIAL_DODGE));
        assert!(sora(1, &[], &[]).grants().is_empty());
        // KH1's Hurricane Blast (a level-up's, say) brings Explosion; Explosion leaves it alone.
        let g = sora(1, &[kh1::HURRICANE_BLAST | 0x80], &[]).grants();
        assert_eq!(g.iter().map(|(_, a)| a.id).collect::<Vec<_>>(), vec![ours::EXPLOSION]);
        assert_eq!(g[0].0, "with KH1's Hurricane Blast");
        // Level gifts at the old build's levels.
        assert!(ids(&sora(22, &[], &[])).contains(&ours::COUNTERGUARD));
        assert!(!ids(&sora(21, &[], &[])).contains(&ours::COUNTERGUARD));
    }

    #[test]
    fn menu_ids_are_distinct_and_new() {
        let mut ids: Vec<u8> = MENU.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), MENU.len());
        assert!(ids.iter().all(|i| *i >= FIRST_ID && *i <= MAX_ID));
        assert!(MENU.iter().all(|a| a.name.len() <= 24 && a.help.len() <= 60));
        assert!(MENU.iter().all(|a| a.template >= 1 && a.template <= kh1::LAST));
        assert!(BOSS_GIFTS.iter().all(|g| g.ability >= FIRST_ID));
    }

    #[test]
    fn old_saves_move_over() {
        // An old save: Explosion was Hurricane Blast (on), Flash Step was Vortex (off), Upper
        // Slash ours already; Superglide turned into Glide past Chernabog.
        let mut s = sora(60, &[kh1::GUARD, kh1::HURRICANE_BLAST, kh1::VORTEX | 0x80, ours::UPPER_SLASH | 0x80], &[kh1::HIGH_JUMP, kh1::GLIDE]);
        s.progress[11] = 0x33;
        assert!(old_layout(&s));
        let m = migrate(&s);
        let after = Kh1Sora { abilities: m.abilities.clone(), shared: m.shared.clone(), ..s.clone() };
        assert!(after.abilities.contains(&ours::EXPLOSION) && after.abilities.contains(&(kh1::HURRICANE_BLAST | 0x80)));
        assert!(after.abilities.contains(&(ours::FLASH_STEP | 0x80)) && after.abilities.contains(&(kh1::VORTEX | 0x80)));
        assert!(after.shared.contains(&kh1::SUPERGLIDE));
        // KH2 sees the same moves as before the split.
        assert!(after.kh2().equipped.contains(&kh2::EXPLOSION) && !after.kh2().equipped.contains(&kh2::FLASH_STEP));
        assert_eq!(after.kh2().glide, 4);
        // Once is enough: nothing more to do.
        let again = migrate(&after);
        assert!(again.lines.is_empty());
        // A plain KH1 save isn't of the old layout.
        assert!(!old_layout(&sora(60, &[kh1::HURRICANE_BLAST], &[kh1::GLIDE])));
    }

    #[test]
    fn migration_makes_room_in_a_full_list() {
        let mut l = vec![kh1::HURRICANE_BLAST, ours::QUICK_RUN | 0x80];
        l.resize(48, 16 | 0x80); // Cheer, off, filling the list.
        let s = sora(30, &l, &[]);
        let m = migrate(&s);
        assert!(m.abilities.contains(&ours::EXPLOSION));
        assert_eq!(m.removed.len(), 1);
        assert!(m.removed[0].contains("(id 16)"));
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
