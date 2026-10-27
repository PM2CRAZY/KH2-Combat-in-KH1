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
//!
//! Progression is KH1's vanilla one (the tester, what-is-still-missing after m17: "make it all
//! vanilla, just give the player all the custom abilities we added, existing saves get updated,
//! remove extra ap for now"): KH1 gives its own abilities on its own schedule and the mod never
//! takes one out; every KH2 ability of ours is Sora's from the start (`Kh1Sora::grants`); KH2's
//! growth levels come only from the Drive Forms' levels (`FormGrowth`); saves the older builds
//! changed get KH1's abilities back once (`restore_vanilla`).

use character_core::Abilities;

/// KH1 ability ids.
pub mod kh1 {
    pub const HIGH_JUMP: u8 = 1;
    pub const MERMAID_KICK: u8 = 2;
    pub const GLIDE: u8 = 3;
    pub const SUPERGLIDE: u8 = 4;
    /// Zantetsuken (taken out of Sora's list until the tester's combo row selection m22: "bring
    /// back zantetsuken"; one taken out then is given back, `menu::given_back`).
    pub const ZANTETSUKEN: u8 = 59;
    pub const COMBO_PLUS: u8 = 6;
    pub const AIR_COMBO_PLUS: u8 = 7;
    pub const VORTEX: u8 = 17;
    pub const AERIAL_SWEEP: u8 = 18;
    pub const COUNTERATTACK: u8 = 19;
    pub const BLITZ: u8 = 20;
    pub const GUARD: u8 = 21;
    pub const DODGE_ROLL: u8 = 22;
    pub const MP_HASTE: u8 = 23;
    pub const MP_RAGE: u8 = 24;
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

/// Which list an ability is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum List {
    /// The shared abilities (`+0x599`: High Jump, Glide...), no AP.
    Shared,
    /// Sora's abilities (`+0x44`), with AP: KH1's own.
    Sora,
    /// The KH2 list (the tester, what-is-still-missing after m18, answer 1: "give kh2's its own
    /// ability list right next to sora, should be titled "KH2""): 48 bytes of its own in the save
    /// (`KH2_LIST_OFF` in KH1's config block), KH1's list format, shown by KH1's Abilities menu
    /// as a row "KH2" after Sora's (`win/kh2list.rs`), equipped with Sora's AP.
    Kh2,
}

/// The KH2 list's place: its offset in KH1's config block (save sub-block 5, `0x400` bytes,
/// pointer Steam `0x142E1F4D8`), whose `+0x78..+0x3FF` KH1 never reads or writes beyond the
/// new-game zeroing (`docs/kh1-config-menu.md` 1.1; Checked (decomp, Steam): every access through
/// the pointer is below `+0x78`). 48 bytes, like Sora's own list.
pub const KH2_LIST_OFF: usize = 0x3C0;
pub const KH2_LIST_LEN: usize = 48;

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
    /// `glide`: LV1, higher only from Final's form level (KH1's Glide, and KH1's Superglide too
    /// since the tester's m18 answer 3 (b)).
    Glide,
    /// KH2's own Glide (`ours::GLIDE`): LV1, higher only from Final's form level.
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
    // (KH2's Glide LV1, raised only by Final: the tester, what-is-still-missing after m18,
    // answer 3 (b), "treat it as LV1 so only Final raises Glide".)
    k(4, "Superglide", Kh2(Effect::Glide)),
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
    pub list: List,
    /// The KH1 ability whose table entry it is made from (its kind and place in the menu).
    pub template: u8,
    /// What it does for KH2's controller.
    pub effect: Effect,
    /// KH1's namesake (0: none), which stood for it before the split (`migrate` moves such a
    /// save over).
    pub with: u8,
    /// While KH1's own states move Sora.
    pub on_kh1: Side,
}

#[allow(clippy::too_many_arguments)]
const fn m(id: u8, name: &'static str, help: &'static str, ap: u8, template: u8, effect: Effect, with: u8, on_kh1: Side) -> MenuAbility {
    MenuAbility { id, name, help, ap, list: List::Kh2, template, effect, with, on_kh1 }
}

/// A story point: a world's story progress byte (`Kh1Sora::progress`) at or past the value it has
/// once the boss is beaten (as the Archipelago KH1 client reads them: gaithern's KH-1FM-AP-LUA,
/// MIT, after the RetroAchievements code notes). Only Chernabog's is used now (`CHERNABOG`); the
/// mod's own boss gifts are gone (vanilla progression, the tester, what-is-still-missing after
/// m17: "make it all vanilla, just give the player all the custom abilities we added").
#[derive(Clone, Copy, Debug)]
pub struct BossGift {
    pub boss: &'static str,
    /// The world's index in the progress bytes (0 Traverse Town, 1 Deep Jungle, 2 Olympus
    /// Coliseum, 3 Wonderland, 4 Agrabah, 5 Monstro, 6 Atlantica, 8 Halloween Town, 9 Neverland,
    /// 10 Hollow Bastion, 11 End of the World).
    pub world: usize,
    pub progress: u8,
    /// The ability KH1 gives there.
    pub ability: u8,
}

/// The first id past KH1's own (65 entries in its table).
pub const FIRST_ID: u8 = 66;
/// The last id an ability can have (bit 7 of a list byte is "not equipped").
pub const MAX_ID: u8 = 0x7F;

use Side::{Borrow, Kh1Combo};

pub const MENU: [MenuAbility; 23] = [
    // Every one of these is simply Sora's (off until equipped, with AP) since the tester's
    // "make it all vanilla, just give the player all the custom abilities we added". AP as KH2's.
    m(66, "Quick Run", "Dash swiftly along the ground.", 2, kh1::DODGE_ROLL, Effect::QuickRun, 0, Hook),
    m(67, "Aerial Dodge", "Leap once more in midair.", 3, kh1::DODGE_ROLL, Effect::AerialDodge, 0, Borrow),
    m(68, "Aerial Recovery", "Right yourself in midair after being knocked back.", 2, kh1::COMBO_PLUS, Effect::AerialRecovery, 0, Hook),
    m(69, "Dodge Slash", "Ground combo: strike nearby foes all around.", 2, kh1::SLAPSHOT, Effect::Modifier(kh2::DODGE_SLASH), 0, Kh1Combo),
    m(70, "Horizontal Slash", "Air combo: a powerful side slash with Square.", 2, kh1::SLAPSHOT, Effect::Modifier(kh2::HORIZONTAL_SLASH), 0, Kh1Combo),
    m(71, "Negative Combo", "Shortens combos by one attack.", 2, kh1::COMBO_PLUS, Effect::NegativeCombo, 0, Kh1Combo),
    m(72, "Aerial Spiral", "Air combo: spin into foes in midair.", 2, kh1::SLAPSHOT, Effect::Modifier(kh2::AERIAL_SPIRAL), 0, Kh1Combo),
    m(73, "Upper Slash", "Combo: knock a foe into the air with Square.", 4, kh1::SLAPSHOT, Effect::Modifier(kh2::UPPER_SLASH), 0, Kh1Combo),
    m(74, "Finishing Plus", "Adds a finishing move to the combo.", 5, kh1::COMBO_PLUS, Effect::FinishingPlus, 0, Kh1Combo),
    m(75, "Retaliating Slash", "When knocked away, recover and strike back with Square.", 3, kh1::SLAPSHOT, Effect::Modifier(kh2::RETALIATING_SLASH), 0, Hook),
    m(76, "Aerial Dive", "From the ground: leap at a far airborne foe.", 2, kh1::SLAPSHOT, Effect::Modifier(kh2::AERIAL_DIVE), 0, Kh1Combo),
    m(77, "Magnet Burst", "Air combo finisher: pull in foes all around.", 3, kh1::SLAPSHOT, Effect::Modifier(kh2::MAGNET_BURST), 0, Kh1Combo),
    // The split: KH2's modifiers that were KH1's ids, each placed after its KH1 namesake in the
    // menu (`with`: the namesake, used by `migrate` for saves from before the split). AP from
    // KH2's item table.
    m(78, "Flash Step", "Ground combo: dash to a distant foe and strike.", 2, kh1::VORTEX, Effect::Modifier(kh2::FLASH_STEP), kh1::VORTEX, Kh1Combo),
    m(79, "Aerial Sweep", "KH2's air combo: a spinning sweep at airborne foes.", 2, kh1::AERIAL_SWEEP, Effect::Modifier(kh2::AERIAL_SWEEP), kh1::AERIAL_SWEEP, Kh1Combo),
    m(80, "Counterguard", "Counter: strike right after guarding.", 4, kh1::COUNTERATTACK, Effect::Modifier(kh2::COUNTERGUARD), kh1::COUNTERATTACK, Hook),
    m(81, "Finishing Leap", "Finisher: end a combo with Square to leap and slam.", 5, kh1::BLITZ, Effect::Modifier(kh2::FINISHING_LEAP), kh1::BLITZ, Kh1Combo),
    m(82, "Slapshot", "KH2's ground combo: rapid strikes at a nearby foe.", 2, kh1::SLAPSHOT, Effect::Modifier(kh2::SLAPSHOT), kh1::SLAPSHOT, Kh1Combo),
    m(83, "Slide Dash", "Ground combo: dash at a distant foe.", 2, kh1::SLIDING_DASH, Effect::Modifier(kh2::SLIDE_DASH), kh1::SLIDING_DASH, Kh1Combo),
    m(84, "Explosion", "Finisher: unleash a blast around you.", 3, kh1::HURRICANE_BLAST, Effect::Modifier(kh2::EXPLOSION), kh1::HURRICANE_BLAST, Kh1Combo),
    m(85, "Vicinity Break", "Ground combo: hit foes all around.", 2, kh1::RIPPLE_DRIVE, Effect::Modifier(kh2::VICINITY_BREAK), kh1::RIPPLE_DRIVE, Kh1Combo),
    m(86, "Guard Break", "Finisher: break through a foe's guard.", 3, kh1::STUN_IMPACT, Effect::Modifier(kh2::GUARD_BREAK), kh1::STUN_IMPACT, Kh1Combo),
    m(87, "Aerial Finish", "Air finisher: a powerful spinning strike.", 3, kh1::GRAVITY_BREAK, Effect::Modifier(kh2::AERIAL_FINISH), kh1::GRAVITY_BREAK, Kh1Combo),
    // KH2's own Glide (t323, the user: "if glide is set to kh2 both glide and super glide need to
    // be removed from the shared abilities list and only the original kh2 growth glide should
    // appear again as its own custom ability"): a shared ability (no AP), not learned on its own
    // (`grants` gives none): `glide_swap` puts it in place of KH1's Glide and Superglide at the
    // Glide style KH2, and back at KH1. Its KH1 side: KH2's glide is a borrow out of KH1's air.
    MenuAbility { id: 88, name: "Glide", help: "Glide through the air. Grows with Sora.", ap: 0, list: List::Shared, template: kh1::GLIDE, effect: Effect::Kh2Glide, with: 0, on_kh1: Borrow },
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

// Growth levels (the tester, what-is-still-missing after m16, answer 6 (b): base Sora's growth
// only from the Drive Forms' levels, as in KH2; after m17: "make it all vanilla"). Nothing grows
// with Sora's own level or with bosses any more. An ability he has is LV1; Valor's, Wisdom's and
// Final's levels raise High Jump, Quick Run and Glide (`FormGrowth`, KH2's `0x1403E8380`). KH1's
// own High Jump and Glide count as KH2's LV1 once KH1 has given them (Assumption taken for the
// tester: their question 3 wasn't answered). Dodge Roll and Aerial Dodge stay LV1: in KH2 they
// grow only with Limit and Master, which aren't on KH1's states here.

/// Quick Run's name in the menu and the level-up box ("Quick Run LV2", "Quick Run MAX").
pub fn quick_run_name(l: u8) -> String {
    level_name("Quick Run", l)
}

/// Chernabog beaten: End of the World's progress (index 11) at 0x33, where KH1 gives Superglide
/// (as the Archipelago KH1 client reads it, gaithern's KH-1FM-AP-LUA, MIT).
pub const CHERNABOG: BossGift = BossGift { boss: "Chernabog", world: 11, progress: 0x33, ability: kh1::SUPERGLIDE };


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

/// The debug menu's Growth Abilities rows (the quick-run-native thread, m6: "can you make it a
/// setting in the debug where I can adjust the growth abilities"), in its order.
pub const GROWTH_ROWS: [&str; 5] = ["High Jump", "Quick Run", "Dodge Roll", "Aerial Dodge", "Glide"];

/// One row's setting: Normal (what Sora's level and story give, `None` in `with_growth_override`),
/// Off, LV1 .. MAX. Stored as 0 Normal, 1 Off, 2..5 LV1..MAX.
pub const GROWTH_SETTINGS: [&str; 6] = ["Normal", "Off", "LV1", "LV2", "LV3", "MAX"];

/// The ini word for a row's setting (`GROWTH_SETTINGS`' index), and back.
pub fn growth_setting_word(v: u8) -> &'static str {
    ["normal", "off", "lv1", "lv2", "lv3", "max"].get(v as usize).copied().unwrap_or("normal")
}

pub fn growth_setting_parse(w: &str) -> u8 {
    match w.trim().to_ascii_lowercase().as_str() {
        "off" | "0" => 1,
        "lv1" | "1" => 2,
        "lv2" | "2" => 3,
        "lv3" | "3" => 4,
        "max" | "4" => 5,
        _ => 0,
    }
}

/// The level a row's setting gives (`None`: Normal, `Some(0)`: Off, `Some(1..=4)`: LV1..MAX).
pub fn growth_setting_level(v: u8) -> Option<u8> {
    match v {
        1 => Some(0),
        2..=5 => Some(v - 1),
        _ => None,
    }
}

/// KH2's controller's abilities with the debug menu's growth overrides (`GROWTH_ROWS`' order):
/// a row set to a level gives that level whether or not Sora learned or equipped it (the tester's
/// answer 3 (a)); Off takes it away; Normal leaves what KH1's menu and his level gave.
pub fn with_growth_override(a: &Abilities, ov: &[Option<u8>; 5]) -> Abilities {
    let mut out = a.clone();
    let fields: [&mut u8; 5] = [&mut out.high_jump, &mut out.quick_run, &mut out.dodge_roll, &mut out.aerial_dodge, &mut out.glide];
    for (f, o) in fields.into_iter().zip(ov.iter()) {
        if let Some(l) = o {
            *f = (*l).min(4);
        }
    }
    out
}

/// Whether a list has `id` in it at all (equipped or not).
pub fn has(list: &[u8], id: u8) -> bool {
    list.iter().take_while(|b| **b != 0).any(|b| b & 0x7F == id)
}

/// An ability's name (KH1's or ours).
pub fn name(id: u8) -> String {
    row(id).map_or(format!("ability {}", id & 0x7F), |r| r.name.to_string())
}


/// Free places in a list (up to its first 0).
pub fn free(list: &[u8]) -> usize {
    list.len() - list.iter().position(|b| *b == 0).unwrap_or(list.len())
}

/// Whether a list byte is one of our KH2 abilities.
pub fn ours_byte(b: u8) -> bool {
    b != 0 && MENU.iter().any(|a| a.id == b & 0x7F)
}

/// Base Sora's growth levels from the Drive Forms' levels (KH2's form level-up, Steam KH2
/// `0x1403E8380`, `formlevel::sora_growth`, Checked (decomp)): Valor's High Jump, Wisdom's Quick
/// Run, Final's Glide (0: none). KH2 has no other source of them, and neither has the mod now:
/// Sora's own level and bosses give none (the tester, after m16, answer 6 (b)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FormGrowth {
    pub high_jump: u8,
    pub quick_run: u8,
    pub glide: u8,
}

/// KH2's fmlv abilities (Steam KH2 `00battle.bin` fmlv, Checked (files)) as KH1's own (the
/// tester, after m15, answer 6 (c)): KH2's Combo Plus (162) and Air Combo Plus (163) are KH1's
/// Combo Plus / Air Combo Plus (their KH2 side is KH2's, `Effect::ComboPlus`); KH2's MP Rage
/// (412) is KH1's MP Rage (KH1's own effect: MP when hurt; KH2's MP Rage is the same idea with
/// other numbers, Assumption that it counts as its counterpart). KH2's MP Haste (413, Wisdom
/// LV6: KH2's shortens MP Charge, which KH1 has no system for) is KH1's own MP Haste (the tester,
/// after m16, answer 4 (a); once a KH2 MP system is a Config setting, it becomes MP Charge's
/// MP Haste there). None yet: the Auto forms (385 Valor, 386 Wisdom, 387 Master, 388 Final, 568
/// Limit; to be built natively later, answer 3), Form Boost (398, waits for the Drive gauge,
/// answer 5), Draw (405), Lucky Lucky (407).
pub fn kh1_reward(kh2_id: u16) -> Option<u8> {
    match kh2_id {
        162 => Some(kh1::COMBO_PLUS),
        163 => Some(kh1::AIR_COMBO_PLUS),
        412 => Some(kh1::MP_RAGE),
        413 => Some(kh1::MP_HASTE),
        _ => None,
    }
}

/// A KH2 fmlv ability's name, for the log (the ones the forms here give).
pub fn kh2_reward_name(kh2_id: u16) -> &'static str {
    match kh2_id {
        162 => "Combo Plus",
        163 => "Air Combo Plus",
        385 => "Auto Valor",
        386 => "Auto Wisdom",
        387 => "Auto Master",
        388 => "Auto Final",
        568 => "Auto Limit",
        398 => "Form Boost",
        405 => "Draw",
        407 => "Lucky Lucky",
        412 => "MP Rage",
        413 => "MP Haste",
        _ => "a KH2 ability",
    }
}

/// Sora's side of KH1's save block.
#[derive(Clone, Debug, PartialEq)]
pub struct Kh1Sora {
    pub level: u8,
    /// The 48 ability bytes (`+0x44`).
    pub abilities: Vec<u8>,
    /// The shared abilities (`+0x599`).
    pub shared: Vec<u8>,
    /// The KH2 list (`List::Kh2`, 48 bytes; all 0 before a save is moved over to it).
    pub kh2: Vec<u8>,
    /// The worlds' story progress bytes (save `+0x1504`, 12).
    pub progress: Vec<u8>,
    /// His growth from the Drive Forms' levels (`win/formexp.rs`).
    pub form_growth: FormGrowth,
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
            List::Kh2 => &self.kh2,
        }
    }

    /// Equipped copies of one of ours: in its list, plus any copy still in Sora's own (a save not
    /// moved over to the KH2 list yet, `move_to_kh2_list`, keeps working).
    fn ours_equipped(&self, ab: &MenuAbility) -> u8 {
        let n = equipped(self.list(ab.list), ab.id);
        if ab.list == List::Kh2 {
            n.saturating_add(equipped(&self.abilities, ab.id))
        } else {
            n
        }
    }

    /// Whether he has one of ours at all (on or off), in its list or still in Sora's own.
    pub fn has_ours(&self, id: u8) -> bool {
        MENU.iter().find(|a| a.id == id).is_some_and(|ab| has(self.list(ab.list), id) || (ab.list == List::Kh2 && has(&self.abilities, id)))
    }

    /// One equipped ability's effect on KH2's controller (`copies`: how many are equipped).
    fn apply(&self, out: &mut Abilities, e: Effect, copies: u8) {
        let push = |out: &mut Abilities, n: u16| {
            if !out.equipped.contains(&n) {
                out.equipped.push(n);
            }
        };
        match e {
            // (LV1 from the ability, higher only from the Drive Forms' levels; KH1's Superglide is
            // KH2's Glide LV1 too, after m18 answer 3 (b).)
            Effect::HighJump => out.high_jump = out.high_jump.max(self.high_jump_now()),
            Effect::Glide => out.glide = out.glide.max(self.form_growth.glide.max(1)),
            Effect::Kh2Glide => out.glide = out.glide.max(self.kh2_glide_now()),
            Effect::Guard => {
                out.guard = true;
                push(out, kh2::GUARD);
            }
            Effect::DodgeRoll => out.dodge_roll = out.dodge_roll.max(1),
            Effect::ComboPlus => out.combo_plus = out.combo_plus.saturating_add(copies),
            Effect::AirComboPlus => out.air_combo_plus = out.air_combo_plus.saturating_add(copies),
            Effect::ComboMaster => out.combo_master = true,
            Effect::QuickRun => out.quick_run = out.quick_run.max(self.quick_run_now()),
            Effect::AerialDodge => out.aerial_dodge = out.aerial_dodge.max(1),
            Effect::AerialRecovery => out.aerial_recovery = true,
            Effect::FinishingPlus => out.finishing_plus = 1,
            Effect::NegativeCombo => out.negative_combo = 1,
            Effect::Modifier(n) => push(out, n),
        }
    }

    /// High Jump's level once he has it: LV1, or what Valor's form level gives (`FormGrowth`).
    pub fn high_jump_now(&self) -> u8 {
        self.form_growth.high_jump.max(1)
    }

    /// Quick Run's level once he has it: LV1, or Wisdom's.
    pub fn quick_run_now(&self) -> u8 {
        self.form_growth.quick_run.max(1)
    }

    /// KH2's own Glide's level (`ours::GLIDE`, which stands for KH1's Glide and Superglide at the
    /// Glide style KH2): LV1, or Final's form level (no longer MAX past Chernabog: after m18,
    /// answer 3 (b)).
    pub fn kh2_glide_now(&self) -> u8 {
        self.form_growth.glide.max(1)
    }

    /// KH2's controller's abilities from KH1's menu and Sora's level: every equipped ability
    /// whose KH2 side is a KH2 value (`Side::Kh2`), KH1's and ours.
    pub fn kh2(&self) -> Abilities {
        let mut out = Abilities { guard: false, ..Abilities::default() };
        for a in KH1_ABILITIES {
            if let Kh2(e) = a.on_kh2 {
                let n = equipped(self.list(list_of(a.id)), a.id);
                if n > 0 {
                    self.apply(&mut out, e, n);
                }
            }
        }
        for ab in MENU {
            let n = self.ours_equipped(&ab);
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
        MENU.iter().filter(|a| a.on_kh1 == Hook && self.ours_equipped(a) > 0).map(|a| a.id).collect()
    }

    /// Our KH2 abilities his lists don't hold yet (the tester, after m17: "just give the player
    /// all the custom abilities we added"): every one of `MENU` in the KH2 list. KH2's own Glide
    /// (shared) only comes with the Glide style (`glide_swap`).
    pub fn grants(&self) -> Vec<MenuAbility> {
        MENU.iter().filter(|ab| ab.list == List::Kh2 && !self.has_ours(ab.id)).copied().collect()
    }
}

/// Whether a gift's boss is beaten.
pub fn beaten(progress: &[u8], g: &BossGift) -> bool {
    progress.get(g.world).is_some_and(|p| *p >= g.progress)
}

/// The save's ability-layout mark: a u16 at save `+0x48A` (the spare bytes after the ten party
/// records, `docs/kh1-equipment.md` section 5; Valor's Keyblade uses `+0x488`). `LAYOUT_SPLIT`
/// once the save has been moved over to the split ids (`migrate`), or was never on the old ones.
/// `LAYOUT_VANILLA` once the save has also been put back on KH1's vanilla progression
/// (`restore_vanilla`, the tester, what-is-still-missing after m17: "existing saves get updated").
pub const LAYOUT_OFF: usize = 0x48A;
pub const LAYOUT_SPLIT: u16 = 0x4B02;
pub const LAYOUT_VANILLA: u16 = 0x4B03;
/// Once the save's KH2 abilities are in the KH2 list (`move_to_kh2_list`, the tester, after m18,
/// answer 1), so Sora's own 48 are KH1's alone.
pub const LAYOUT_KH2_LIST: u16 = 0x4B04;

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
    /// The KH2 list.
    pub kh2: Vec<u8>,
    /// What changed, for the log.
    pub lines: Vec<String>,
    /// What couldn't be done, for the log.
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
    let mut k2 = s.kh2.clone();
    let mut lines = Vec::new();
    let removed = Vec::new();
    for ab in MENU.iter().filter(|ab| ab.with != 0) {
        if !has(&a, ab.with) || has(&a, ab.id) || has(&k2, ab.id) {
            continue;
        }
        let on = equipped(&a, ab.with) > 0;
        let byte = if on { ab.id } else { ab.id | 0x80 };
        // (In the KH2 list, never in a KH1 ability's place.)
        if !push(&mut k2, byte) {
            lines.push(format!("{}: no room for {} in the KH2 list", name(ab.with), ab.name));
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
    Migrated { abilities: a, shared: sh, kh2: k2, lines, removed }
}

/// Moves every one of ours (`List::Kh2`) out of Sora's own 48 into the KH2 list, as it was (on or
/// off), once per save (`LAYOUT_KH2_LIST`; the tester, what-is-still-missing after m18, answer 1:
/// "give kh2's its own ability list right next to sora"): Sora's 48 are KH1's alone after it, and
/// keep their order with the gaps closed. A copy the KH2 list already holds isn't doubled (the
/// spare is dropped).
pub fn move_to_kh2_list(s: &Kh1Sora) -> Migrated {
    let end = s.abilities.iter().position(|b| *b == 0).unwrap_or(s.abilities.len());
    let mut k2 = s.kh2.clone();
    let mut kept: Vec<u8> = Vec::new();
    let mut lines = Vec::new();
    let mut removed = Vec::new();
    for &b in &s.abilities[..end] {
        let ours = MENU.iter().any(|ab| ab.list == List::Kh2 && ab.id == b & 0x7F);
        if !ours {
            kept.push(b);
            continue;
        }
        let on = if b & 0x80 == 0 { "on" } else { "off" };
        if has(&k2, b & 0x7F) {
            removed.push(format!("{} ({on}): a second copy, dropped (the KH2 list has it)", name(b)));
        } else if push(&mut k2, b) {
            lines.push(format!("{} ({on}) moved to the KH2 list", name(b)));
        } else {
            removed.push(format!("{} ({on}): the KH2 list is full, dropped", name(b)));
        }
    }
    kept.resize(s.abilities.len(), 0);
    Migrated { abilities: kept, shared: s.shared.clone(), kh2: k2, lines, removed }
}

/// What `kh1-kh2-removed.txt` (next to the game; every KH1 ability the mod's older builds took
/// out of a save was noted there) says was taken out, up to Sora's level `level` (each line starts
/// "level N:"; a line without it counts): Sora's own abilities (`(id N)` lines, Zantetsuken's
/// "Zantetsuken taken out") and the shared ones (the "spare shared abilities taken out" lines, by
/// name), one entry per copy taken out.
pub fn removed_by_mod(text: &str, level: u8) -> (Vec<u8>, Vec<u8>) {
    let mut sora = Vec::new();
    let mut shared = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("level ") {
            let n = rest.split(':').next().and_then(|n| n.trim().parse::<u16>().ok());
            if n.is_some_and(|n| n > level as u16) {
                continue;
            }
        }
        if line.contains("Zantetsuken taken out") {
            sora.push(kh1::ZANTETSUKEN);
            continue;
        }
        if let Some(i) = line.find("spare shared abilities taken out") {
            let names = line[i..].split_once(':').map_or("", |(_, n)| n);
            for n in names.split(',').map(str::trim) {
                if let Some(a) = KH1_ABILITIES.iter().find(|a| a.name == n && list_of(a.id) == List::Shared) {
                    shared.push(a.id);
                }
            }
            continue;
        }
        let mut rest = line;
        while let Some(i) = rest.find("(id ") {
            rest = &rest[i + 4..];
            if let Some(id) = rest.split(')').next().and_then(|n| n.trim().parse::<u8>().ok()) {
                if (1..=kh1::LAST).contains(&id) && list_of(id) == List::Sora {
                    sora.push(id);
                }
            }
        }
    }
    (sora, shared)
}

/// Puts a save the mod's older builds changed back on KH1's vanilla progression (once, by
/// `LAYOUT_VANILLA`; the tester, after m17: "existing saves get updated"):
/// - each KH1 ability `removed_by_mod` lists comes back, off, one copy per copy taken out (KH1's
///   own abilities it took out for room, its spare copies, Zantetsuken), and the shared spares
///   (High Jump, Glide, Superglide) the same way;
/// - Superglide comes back if Chernabog is beaten and KH2's own Glide isn't standing for it.
///
/// Ours are in the KH2 list by then (`move_to_kh2_list` runs first), so they take none of Sora's
/// places.
///
/// Assumption: `kh1-kh2-removed.txt` is one install's file, not one save's; with several saves
/// each gets what was noted up to its own level.
pub fn restore_vanilla(s: &Kh1Sora, removed_text: &str) -> Migrated {
    let mut a = s.abilities.clone();
    let mut sh = s.shared.clone();
    let mut lines = Vec::new();
    let gave_up = Vec::new();
    let (sora_ids, shared_ids) = removed_by_mod(removed_text, s.level);
    for id in sora_ids {
        if push(&mut a, id | 0x80) {
            lines.push(format!("KH1's {} given back (off): the mod had taken it out", name(id)));
        } else {
            lines.push(format!("KH1's {} could not be given back: Sora's list is full of KH1's own", name(id)));
        }
    }
    let kh2_glide = has(&sh, ours::GLIDE);
    for id in shared_ids {
        if kh2_glide && (id == kh1::GLIDE || id == kh1::SUPERGLIDE) {
            continue;
        }
        if push(&mut sh, id | 0x80) {
            lines.push(format!("KH1's {} given back to the shared abilities (off): a spare copy the mod had taken out", name(id)));
        }
    }
    if beaten(&s.progress, &CHERNABOG) && !has(&sh, kh1::SUPERGLIDE) && !kh2_glide {
        let on = equipped(&sh, kh1::GLIDE) > 0;
        if push(&mut sh, if on { kh1::SUPERGLIDE } else { kh1::SUPERGLIDE | 0x80 }) {
            lines.push(format!("Superglide given back (Chernabog beaten; {})", if on { "on" } else { "off" }));
        }
    }
    Migrated { abilities: a, shared: sh, kh2: s.kh2.clone(), lines, removed: gave_up }
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
    /// The debug menu's growth overrides: a level is given even without the ability, Off takes
    /// it away, Normal leaves it.
    #[test]
    fn growth_override_replaces_levels() {
        let a = Abilities { high_jump: 2, quick_run: 0, dodge_roll: 3, aerial_dodge: 1, glide: 2, ..Abilities::default() };
        let o = with_growth_override(&a, &[None, Some(4), Some(0), None, Some(1)]);
        assert_eq!((o.high_jump, o.quick_run, o.dodge_roll, o.aerial_dodge, o.glide), (2, 4, 0, 1, 1));
        assert_eq!(with_growth_override(&a, &[None; 5]), a);
        for v in 0..6u8 {
            assert_eq!(growth_setting_parse(growth_setting_word(v)), v);
        }
        assert_eq!(growth_setting_level(0), None);
        assert_eq!(growth_setting_level(1), Some(0));
        assert_eq!(growth_setting_level(5), Some(4));
    }

    use super::*;

    fn sora(level: u8, abilities: &[u8], shared: &[u8]) -> Kh1Sora {
        let mut a = abilities.to_vec();
        a.resize(48, 0);
        let mut s = shared.to_vec();
        s.resize(10, 0);
        Kh1Sora { level, abilities: a, shared: s, kh2: vec![0; KH2_LIST_LEN], progress: vec![0; 12], form_growth: FormGrowth::default() }
    }

    /// Growth only from form levels (KH2's `0x1403E8380`): LV1 from the ability, higher from the
    /// form; his own level gives nothing, and nothing KH1 is given by a form.
    #[test]
    fn growth_only_from_form_levels() {
        let mut s = sora(99, &[ours::QUICK_RUN, kh1::DODGE_ROLL, ours::AERIAL_DODGE], &[kh1::HIGH_JUMP, kh1::GLIDE]);
        let a = s.kh2();
        assert_eq!((a.high_jump, a.quick_run, a.glide, a.dodge_roll, a.aerial_dodge), (1, 1, 1, 1, 1));
        s.form_growth = FormGrowth { high_jump: 2, quick_run: 3, glide: 1 };
        let a = s.kh2();
        assert_eq!((a.high_jump, a.quick_run, a.glide), (2, 3, 1));
        // A form's level without KH1's High Jump gives none (KH1's list stays vanilla).
        let mut bare = sora(5, &[], &[]);
        bare.form_growth = FormGrowth { high_jump: 3, quick_run: 0, glide: 0 };
        assert_eq!(bare.kh2().high_jump, 0);
        assert!(bare.grants().iter().all(|a| a.list == List::Kh2));
    }

    #[test]
    fn kh2_rewards_as_kh1_abilities() {
        assert_eq!(kh1_reward(162), Some(kh1::COMBO_PLUS));
        assert_eq!(kh1_reward(163), Some(kh1::AIR_COMBO_PLUS));
        assert_eq!(kh1_reward(412), Some(kh1::MP_RAGE));
        assert_eq!(KH1_ABILITIES[kh1::MP_RAGE as usize - 1].name, "MP Rage");
        assert_eq!(kh1_reward(385), None);
        assert_eq!(kh1_reward(413), Some(kh1::MP_HASTE));
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

    /// KH2's own Glide is LV1, raised only by Final's level; Chernabog no longer makes it MAX
    /// (after m18, answer 3 (b)).
    #[test]
    fn kh2_glide_only_from_final() {
        let mut s = sora(99, &[], &[ours::GLIDE]);
        assert_eq!(s.kh2().glide, 1);
        s.progress[11] = 0x33;
        assert_eq!(s.kh2().glide, 1);
        s.form_growth.glide = 3;
        assert_eq!(s.kh2().glide, 3);
        // Off: nothing; and never given on its own.
        assert_eq!(sora(60, &[], &[ours::GLIDE | 0x80]).kh2().glide, 0);
        assert!(!sora(99, &[], &[kh1::GLIDE]).grants().iter().any(|a| a.id == ours::GLIDE));
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
        // Glide doesn't grow with Sora's level; KH1's Superglide is LV1 too (after m18, answer 3
        // (b)): only Final's level raises either.
        assert_eq!(sora(99, &[], &[kh1::GLIDE]).kh2().glide, 1);
        assert_eq!(sora(1, &[], &[kh1::GLIDE, kh1::SUPERGLIDE]).kh2().glide, 1);
        assert_eq!(sora(1, &[], &[kh1::SUPERGLIDE]).kh2().glide, 1);
        assert_eq!(sora(1, &[], &[kh1::GLIDE, kh1::SUPERGLIDE | 0x80]).kh2().glide, 1);
        let mut f = sora(1, &[], &[kh1::GLIDE, kh1::SUPERGLIDE]);
        f.form_growth.glide = 2;
        assert_eq!(f.kh2().glide, 2);
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
        assert_eq!((a.quick_run, a.aerial_dodge, a.dodge_roll, a.high_jump), (1, 1, 1, 1));
        assert!(a.aerial_recovery && a.combo_master && a.finishing_plus == 1);
        assert!(a.equipped.contains(&kh2::DODGE_SLASH) && !a.equipped.contains(&kh2::MAGNET_BURST));
        // Learned but off: nothing.
        let off = sora(99, &[68 | 0x80, 66 | 0x80], &[]).kh2();
        assert!(!off.aerial_recovery && off.quick_run == 0);
        // No KH1 Dodge Roll: no roll at any level.
        assert_eq!(sora(99, &[], &[]).kh2().dodge_roll, 0);
        assert_eq!(quick_run_name(4), "Quick Run MAX");
        assert_eq!((high_jump_name(3), aerial_dodge_name(4)), ("High Jump LV3".to_string(), "Aerial Dodge MAX".to_string()));
    }

    /// Every KH2 ability of Sora's own list is his at once, whatever his level or story.
    #[test]
    fn grants_are_every_kh2_ability() {
        let all: Vec<u8> = MENU.iter().filter(|a| a.list == List::Kh2).map(|a| a.id).collect();
        assert_eq!(all.len(), 22);
        assert_eq!(sora(1, &[], &[]).grants().iter().map(|a| a.id).collect::<Vec<_>>(), all);
        // Already there (even unequipped): not again.
        let g = sora(1, &[66 | 0x80, 68], &[]).grants();
        assert!(!g.iter().any(|a| a.id == 66 || a.id == 68));
        assert_eq!(g.len(), 20);
    }

    /// The KH2 list (after m18, answer 1): ours move out of Sora's 48 as they were, KH1's keep
    /// their order, a second copy isn't doubled; the KH2 list's copies count and grants see them.
    #[test]
    fn ours_move_to_the_kh2_list() {
        let mut s = sora(40, &[16 | 0x80, 66 | 0x80, 87, 6, 70 | 0x80, 21], &[]);
        s.kh2[0] = 70;
        let m = move_to_kh2_list(&s);
        assert_eq!(m.abilities[..4], [16 | 0x80, 6, 21, 0]);
        assert_eq!(m.abilities.len(), 48);
        assert_eq!(m.kh2[..4], [70, 66 | 0x80, 87, 0]);
        assert_eq!((m.lines.len(), m.removed.len()), (2, 1));
        let after = Kh1Sora { abilities: m.abilities, kh2: m.kh2, ..s };
        assert!(after.kh2().equipped.contains(&kh2::AERIAL_FINISH) && after.kh2().equipped.contains(&kh2::HORIZONTAL_SLASH));
        assert!(after.has_ours(66) && !has(&after.abilities, 66));
        assert!(!after.grants().iter().any(|a| [66, 70, 87].contains(&a.id)));
        assert_eq!(after.grants().len(), 19);
        // Nothing more to move.
        let again = move_to_kh2_list(&after);
        assert!(again.lines.is_empty() && again.abilities == after.abilities && again.kh2 == after.kh2);
    }

    /// An older build's removals come back from `kh1-kh2-removed.txt`, by level, one per copy.
    #[test]
    fn removed_abilities_come_back() {
        let text = "level 30: removed Tech Boost (id 60) for Explosion\n\
            level 31: Sora's list was full: x\n\
            level 31: removed Tech Boost (id 60) for Slide Dash\n\
            level 12: spare shared abilities taken out (one of each counts): High Jump, Glide\n\
            level 40: Zantetsuken taken out (the mod has none)\n\
            level 80: removed Cheer (id 16) for Aerial Finish\n\
            level 33: removed Cheer (id 16) to give back Encounter Plus\n";
        let (sora_ids, shared) = removed_by_mod(text, 50);
        assert_eq!(sora_ids, vec![60, 60, 59, 16]);
        assert_eq!(shared, vec![kh1::HIGH_JUMP, kh1::GLIDE]);
        // A nearly full list: ours move to the KH2 list first (as the menu does), then KH1's come
        // back off in the places they freed.
        let mut l = vec![60, 66 | 0x80, 67 | 0x80, 68 | 0x80];
        l.resize(47, 10);
        let mut s = sora(50, &l, &[kh1::HIGH_JUMP, kh1::GLIDE]);
        s.progress[11] = 0x33;
        let moved = move_to_kh2_list(&s);
        let s = Kh1Sora { abilities: moved.abilities, kh2: moved.kh2, ..s };
        let m = restore_vanilla(&s, text);
        assert_eq!(m.abilities.iter().filter(|b| **b == 60 | 0x80).count(), 2);
        assert!(m.abilities.contains(&(59 | 0x80)) && m.abilities.contains(&(16 | 0x80)));
        assert!(!has(&m.abilities, 66) && !has(&m.abilities, 67) && !has(&m.abilities, 68));
        assert!(has(&m.kh2, 66) && has(&m.kh2, 67) && has(&m.kh2, 68));
        assert!(m.removed.is_empty());
        assert_eq!(m.shared[..5], [kh1::HIGH_JUMP, kh1::GLIDE, kh1::HIGH_JUMP | 0x80, kh1::GLIDE | 0x80, kh1::SUPERGLIDE]);
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
    }

    #[test]
    fn old_saves_move_over() {
        // An old save: Explosion was Hurricane Blast (on), Flash Step was Vortex (off), Upper
        // Slash ours already; Superglide turned into Glide past Chernabog.
        let mut s = sora(60, &[kh1::GUARD, kh1::HURRICANE_BLAST, kh1::VORTEX | 0x80, ours::UPPER_SLASH | 0x80], &[kh1::HIGH_JUMP, kh1::GLIDE]);
        s.progress[11] = 0x33;
        assert!(old_layout(&s));
        let m = migrate(&s);
        let after = Kh1Sora { abilities: m.abilities.clone(), shared: m.shared.clone(), kh2: m.kh2.clone(), ..s.clone() };
        assert!(after.kh2.contains(&ours::EXPLOSION) && after.abilities.contains(&(kh1::HURRICANE_BLAST | 0x80)));
        assert!(after.kh2.contains(&(ours::FLASH_STEP | 0x80)) && after.abilities.contains(&(kh1::VORTEX | 0x80)));
        assert!(after.shared.contains(&kh1::SUPERGLIDE));
        // KH2 sees the same moves as before the split.
        assert!(after.kh2().equipped.contains(&kh2::EXPLOSION) && !after.kh2().equipped.contains(&kh2::FLASH_STEP));
        assert_eq!(after.kh2().glide, 1);
        // Once is enough: nothing more to do.
        let again = migrate(&after);
        assert!(again.lines.is_empty());
        // A plain KH1 save isn't of the old layout.
        assert!(!old_layout(&sora(60, &[kh1::HURRICANE_BLAST], &[kh1::GLIDE])));
    }

    /// The split's migration never takes a KH1 ability's place.
    #[test]
    fn migration_never_takes_kh1_places() {
        let mut l = vec![kh1::HURRICANE_BLAST, ours::QUICK_RUN | 0x80];
        l.resize(48, 16 | 0x80); // Cheer, off, filling the list.
        let m = migrate(&sora(30, &l, &[]));
        assert!(!m.abilities.contains(&ours::EXPLOSION) && m.kh2.contains(&ours::EXPLOSION));
        // (KH1's Hurricane Blast is turned off for it, as before; nothing else changes.)
        let mut want = l.clone();
        want[0] |= 0x80;
        assert_eq!(m.abilities, want);
    }

    #[test]
    fn modes_parse() {
        assert_eq!(Mode::parse(" KH1 "), Some(Mode::Kh1));
        assert_eq!(Mode::parse("all"), Some(Mode::All));
        assert_eq!(Mode::parse("x"), None);
    }
}
