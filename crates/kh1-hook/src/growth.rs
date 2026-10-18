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
//!   Jump LV3, Glide grows with Sora (LV2 at 40, LV3 at 52) and is MAX once Chernabog falls,
//!   in place of KH1's Superglide, which is taken out (`GLIDE_LEVELS`, `CHERNABOG`; Sora keeps
//!   one of each: `one_only`),
//!   Guard, Dodge Roll, each Combo Plus / Air Combo Plus, and
//!   KH1's combo modifiers as KH2's nearest one (Vortex: Flash Step, Aerial Sweep, Counterattack:
//!   Counterguard, Blitz: Finishing Leap, Slapshot, Sliding Dash: Slide Dash, Hurricane Blast:
//!   Explosion, Ripple Drive: Vicinity Break, Stun Impact: Guard Break, Gravity Break: Aerial
//!   Finish).
//!   Final Mix's own Combo Master (65) is KH2's.
//! - What only KH2 has goes into KH1's menu as new abilities (`MENU`, ids 66 up, in an
//!   extended copy of KH1's ability table, `win/menu.rs`), learned at Sora's level: Quick Run
//!   and Aerial Dodge in the shared abilities (with High Jump and Glide), the rest in Sora's,
//!   with KH2's AP costs (Quick Run and Aerial Dodge too, in Sora's list; Aerial Dodge only once
//!   Sora has High Jump). Quick Run, Aerial Dodge, High Jump and the dodge roll level up with
//!   Sora to MAX (`QUICK_RUN_LEVELS`, `AERIAL_DODGE_LEVELS`, `HIGH_JUMP_LEVELS`, `GROWTH`).
//!   (Earlier builds put Quick Run and Aerial Dodge in the shared list: `win/menu.rs` moves them.)

use character_core::Abilities;

/// KH1 ability ids.
pub mod kh1 {
    pub const HIGH_JUMP: u8 = 1;
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

/// `MenuAbility::level` for an ability a boss gives (`BOSS_GIFTS`), not a level.
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
    /// Ours (`MENU`) or KH1's (a modifier shown by its KH2 name, `RENAMED`).
    pub ability: u8,
}

pub const BOSS_GIFTS: [BossGift; 9] = [
    BossGift { boss: "Sabor", world: 1, progress: 0x42, ability: 73 },
    // Flash Step (KH1's Vortex) for Sabor too (the user; it was a level 42 gift).
    BossGift { boss: "Sabor", world: 1, progress: 0x42, ability: kh1::VORTEX },
    BossGift { boss: "Clayton", world: 1, progress: 0x56, ability: kh1::AERIAL_SWEEP },
    BossGift { boss: "Jafar", world: 4, progress: 0x49, ability: kh1::GRAVITY_BREAK },
    BossGift { boss: "Pot Centipede", world: 4, progress: 0x35, ability: kh1::STUN_IMPACT },
    BossGift { boss: "Dragon Maleficent", world: 10, progress: 0x6E, ability: kh1::HURRICANE_BLAST },
    BossGift { boss: "Captain Hook", world: 9, progress: 0x56, ability: 77 },
    BossGift { boss: "Cerberus", world: 2, progress: 0x25, ability: 76 },
    // Genie Jafar in Agrabah (the user; earlier Riku the first time): Agrabah's progress is 0x46
    // during Jafar and 0x53 after Genie Jafar's cutscene (KH-1FM-AP-LUA's warp fixes).
    BossGift { boss: "Genie Jafar", world: 4, progress: 0x53, ability: 67 },
];

/// The first id past KH1's own (65 entries in its table).
pub const FIRST_ID: u8 = 66;

pub const MENU: [MenuAbility; 12] = [
    // (The user: Quick Run from level 12, growing with Sora; both with AP in Sora's own
    // list, not the shared one. AP as KH2's.)
    m(66, "Quick Run", "Dash swiftly along the ground.", 2, 12, List::Sora, kh1::DODGE_ROLL, Effect::QuickRun),
    // (The user: Aerial Dodge for beating Genie Jafar, `BOSS_GIFTS`.)
    m(67, "Aerial Dodge", "Leap once more in midair.", 3, BOSS, List::Sora, kh1::DODGE_ROLL, Effect::AerialDodge),
    m(68, "Aerial Recovery", "Right yourself in midair after being knocked back.", 2, 4, List::Sora, kh1::COMBO_PLUS, Effect::AerialRecovery),
    m(69, "Dodge Slash", "Ground combo: strike nearby foes all around.", 2, 10, List::Sora, kh1::SLAPSHOT, Effect::Modifier(kh2::DODGE_SLASH)),
    m(70, "Horizontal Slash", "Air combo: a powerful side slash with Square.", 2, 16, List::Sora, kh1::SLAPSHOT, Effect::Modifier(kh2::HORIZONTAL_SLASH)),
    m(71, "Negative Combo", "Shortens combos by one attack.", 2, 20, List::Sora, kh1::COMBO_PLUS, Effect::NegativeCombo),
    m(72, "Aerial Spiral", "Air combo: spin into foes in midair.", 2, 25, List::Sora, kh1::SLAPSHOT, Effect::Modifier(kh2::AERIAL_SPIRAL)),
    m(73, "Upper Slash", "Combo: knock a foe into the air with Square.", 4, BOSS, List::Sora, kh1::SLAPSHOT, Effect::Modifier(kh2::UPPER_SLASH)),
    m(74, "Finishing Plus", "Adds a finishing move to the combo.", 5, 34, List::Sora, kh1::COMBO_PLUS, Effect::FinishingPlus),
    m(75, "Retaliating Slash", "When knocked away, recover and strike back with Square.", 3, 43, List::Sora, kh1::SLAPSHOT, Effect::Modifier(kh2::RETALIATING_SLASH)),
    m(76, "Aerial Dive", "From the ground: leap at a far airborne foe.", 2, BOSS, List::Sora, kh1::SLAPSHOT, Effect::Modifier(kh2::AERIAL_DIVE)),
    m(77, "Magnet Burst", "Air combo finisher: pull in foes all around.", 3, BOSS, List::Sora, kh1::SLAPSHOT, Effect::Modifier(kh2::MAGNET_BURST)),
];

/// Our Quick Run's ability id (`MENU`).
pub const QUICK_RUN_ID: u8 = 66;

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

/// Dodge Roll's level at Sora's level, once he has KH1's Dodge Roll (`GROWTH`: LV2 at 19, LV3 at
/// 37, MAX at 60).
pub fn dodge_roll_level(level: u8) -> u8 {
    GROWTH.iter().filter_map(|(at, g)| match g {
        Growth::DodgeRoll(l) if level >= *at => Some(*l),
        _ => None,
    }).max().unwrap_or(1)
}

/// Its name in the menu and the level-up box ("Dodge Roll LV2", "Dodge Roll MAX").
pub fn dodge_roll_name(l: u8) -> String {
    if l >= 4 {
        "Dodge Roll MAX".to_string()
    } else {
        format!("Dodge Roll LV{}", l.max(1))
    }
}

/// High Jump's levels past LV1, by Sora's level (the user: it levels up with him too, MAX 4):
/// LV1 when Sora gets it, LV2 at 30, LV3 at 44, MAX at 58 (the user: LV2 at 30).
pub const HIGH_JUMP_LEVELS: [u8; 3] = [30, 44, 58];

/// Aerial Dodge's levels past LV1 (LV1 when Genie Jafar falls): LV2 at 30, LV3 at 42, MAX at 54
/// (the user: its growth from 30).
pub const AERIAL_DODGE_LEVELS: [u8; 3] = [30, 42, 54];

/// Glide's levels past LV1 (LV1 from KH1's Glide, Neverland): LV2 at 40, LV3 at 52; MAX only for
/// beating Chernabog (the user: no Superglide; Glide grows from 40 to LV3, Chernabog gives MAX).
pub const GLIDE_LEVELS: [u8; 2] = [40, 52];

/// Chernabog beaten: End of the World's progress (index 11) at 0x33, where KH1 gives Superglide
/// (as the Archipelago KH1 client reads it, gaithern's KH-1FM-AP-LUA, MIT).
pub const CHERNABOG: BossGift = BossGift { boss: "Chernabog", world: 11, progress: 0x33, ability: kh1::GLIDE };

/// Glide's level at Sora's level, once he has it: MAX once Chernabog is beaten.
pub fn glide_level(level: u8, progress: &[u8]) -> u8 {
    if beaten(progress, &CHERNABOG) {
        4
    } else {
        1 + GLIDE_LEVELS.iter().filter(|at| level >= **at).count() as u8
    }
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
pub const ONE_ONLY: [u8; 3] = [kh1::HIGH_JUMP, kh1::GLIDE, kh1::SUPERGLIDE];

/// The shared list with the spare copies of `ONE_ONLY` taken out (an equipped copy is the one
/// kept), and the ids taken out. The list keeps its length (zeros at the end).
pub fn one_only(list: &[u8]) -> (Vec<u8>, Vec<u8>) {
    keep_one(list, &ONE_ONLY)
}

/// KH1 abilities of Sora's that the mod gives before KH1 does (`LEVEL_GIFTS`, the KH1 ones in
/// `BOSS_GIFTS`): KH1's own copy later is a spare (the user: a second Slide Dash from
/// Maleficent), and one copy is all a KH2 modifier needs.
pub fn early_kh1() -> Vec<u8> {
    let mut ids: Vec<u8> = LEVEL_GIFTS.iter().map(|(_, id)| *id).collect();
    ids.extend(BOSS_GIFTS.iter().map(|g| g.ability).filter(|id| *id < FIRST_ID));
    ids.sort_unstable();
    ids.dedup();
    ids
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

/// KH1 abilities that are KH2 moves here, shown in the menu by their KH2 name and help (the
/// user: where is Explosion? It was Hurricane Blast in the menu).
pub const RENAMED: [(u8, &str, &str); 8] = [
    (kh1::VORTEX, "Flash Step", "Ground combo: dash to a distant foe and strike."),
    (kh1::COUNTERATTACK, "Counterguard", "Counter: strike right after guarding."),
    (kh1::BLITZ, "Finishing Leap", "Finisher: end a combo with Square to leap and slam."),
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
    // A full list of a finished game (the user's level 100 save: no Aerial Dodge, the list full
    // of KH1's, most of them on): any other KH1 ability that's off, but never Guard, Dodge Roll,
    // Combo Plus / Air Combo Plus, the specials or the combo modifiers (KH2 uses them)...
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
/// specials (Sonic Blade, Ars Arcanum, Strike Raid, Ragnarok, Trinity Limit) and every KH1
/// ability that is a KH2 move here (`RENAMED`, Slapshot, Aerial Sweep).
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
    /// KH1's own copy of `id` would be a spare: the mod gives it before KH1 does (its level
    /// reached or its boss beaten) and Sora has it (the user: no "learned" box for those).
    pub fn spare_gift(&self, id: u8) -> bool {
        let early = LEVEL_GIFTS.iter().any(|(l, i)| *i == id && self.level >= *l) || BOSS_GIFTS.iter().any(|g| g.ability == id && beaten(&self.progress, g));
        early && has(&self.abilities, id)
    }

    /// KH2's abilities from KH1's menu and Sora's level.
    pub fn kh2(&self) -> Abilities {
        let a = &self.abilities;
        let s = &self.shared;
        let mut out = Abilities { guard: false, one_swing_on_miss: true, ..Abilities::default() };
        // (The user: one High Jump, growing with Sora to MAX; one Glide, growing to LV3, MAX for
        // Chernabog; no Superglide, which `win/menu.rs` turns into Glide.)
        if equipped(s, kh1::HIGH_JUMP) > 0 {
            out.high_jump = high_jump_level(self.level);
        }
        if equipped(s, kh1::GLIDE) > 0 || equipped(s, kh1::SUPERGLIDE) > 0 {
            out.glide = glide_level(self.level, &self.progress);
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
                // (The user: they grow with Sora, `QUICK_RUN_LEVELS`, `AERIAL_DODGE_LEVELS`.)
                Effect::QuickRun => quick_run = quick_run_level(self.level).max(1),
                Effect::AerialDodge => aerial_dodge = aerial_dodge_level(self.level),
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
            .filter(|ab| ab.level != BOSS && self.level >= ab.level)
            .filter(|ab| !has(if ab.list == List::Shared { &self.shared } else { &self.abilities }, ab.id))
            .copied()
            .collect()
    }
}

/// KH1 abilities (shown by their KH2 names) also learned at a level, earlier than KH1's own
/// level-up gives them (the user: Aerial Sweep at about 28; KH1's Dream Shield gives it at 84).
/// (The user: Slide Dash at 30, Vicinity Break at 69 was absurd; the rest set to match. KH1's
/// Dream Shield gives them at 45 / 39 / 69 / 78 / 60.)
pub const LEVEL_GIFTS: [(u8, u8); 5] = [
    (22, kh1::COUNTERATTACK),
    (28, kh1::AERIAL_SWEEP),
    (30, kh1::SLIDING_DASH),
    (36, kh1::RIPPLE_DRIVE),
    (48, kh1::BLITZ),
];

impl Kh1Sora {
    /// The bosses beaten (and levels reached, `LEVEL_GIFTS`) whose gift Sora doesn't have yet:
    /// (why, ability).
    pub fn boss_gifts(&self) -> Vec<(String, u8)> {
        let mut out: Vec<(String, u8)> = BOSS_GIFTS.iter().filter(|g| beaten(&self.progress, g)).map(|g| (format!("{} beaten", g.boss), g.ability)).collect();
        out.extend(LEVEL_GIFTS.iter().filter(|(l, _)| self.level >= *l).map(|(l, id)| (format!("level {l}"), *id)));
        let mut seen = Vec::new();
        out.retain(|(_, id)| !has(&self.abilities, *id) && !seen.contains(id) && {
            seen.push(*id);
            true
        });
        out
    }
}

/// Whether a gift's boss is beaten.
pub fn beaten(progress: &[u8], g: &BossGift) -> bool {
    progress.get(g.world).is_some_and(|p| *p >= g.progress)
}

/// The AP Sora gets on top of KH1's by his level: each KH2 ability he has learned brings its
/// own AP, so taking the KH2 abilities costs none of KH1's.
/// Ours a boss gives count once it is beaten.
pub fn bonus_ap(level: u8, progress: &[u8]) -> u8 {
    let earned = |ab: &MenuAbility| {
        if ab.level == BOSS {
            BOSS_GIFTS.iter().any(|g| g.ability == ab.id && beaten(progress, g))
        } else {
            level >= ab.level
        }
    };
    MENU.iter().filter(|ab| earned(ab)).map(|ab| ab.ap).sum()
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

    #[test]
    fn bonus_ap_follows_the_learned_abilities() {
        assert_eq!(bonus_ap(1, &[]), 0);
        assert_eq!(bonus_ap(4, &[]), 2);
        assert_eq!(bonus_ap(10, &[]), 4);
        let all = [0xFFu8; 12];
        assert_eq!(bonus_ap(50, &all), MENU.iter().map(|ab| ab.ap).sum::<u8>());
        assert_eq!(bonus_ap(50, &all), bonus_ap(100, &all));
        // Boss gifts' AP only once beaten (Upper Slash, Aerial Dive, Magnet Burst, Aerial Dodge).
        assert_eq!(bonus_ap(100, &all), bonus_ap(100, &[]) + 4 + 2 + 3 + 3);
    }

    fn sora(level: u8, abilities: &[u8], shared: &[u8]) -> Kh1Sora {
        let mut a = abilities.to_vec();
        a.resize(48, 0);
        let mut s = shared.to_vec();
        s.resize(10, 0);
        Kh1Sora { level, abilities: a, shared: s, progress: vec![0; 12] }
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
        assert_eq!((a.dodge_roll, a.combo_plus, a.air_combo_plus, a.high_jump, a.glide), (1, 2, 1, 1, 1));
        // Glide grows with Sora (LV2 at 40, LV3 at 52), MAX once Chernabog falls.
        assert_eq!(sora(39, &[], &[kh1::GLIDE]).kh2().glide, 1);
        assert_eq!(sora(40, &[], &[kh1::GLIDE]).kh2().glide, 2);
        assert_eq!(sora(99, &[], &[kh1::GLIDE]).kh2().glide, 3);
        let mut c = sora(1, &[], &[kh1::GLIDE]);
        c.progress[11] = 0x33;
        assert_eq!(c.kh2().glide, 4);
        assert!(a.equipped.contains(&kh2::FLASH_STEP) && !a.equipped.contains(&kh2::SLAPSHOT));
        assert!(a.equipped.contains(&kh2::GUARD));
    }

    #[test]
    fn early_gifts_keep_one_copy() {
        let ids = early_kh1();
        assert!(ids.contains(&kh1::SLIDING_DASH) && ids.contains(&kh1::GRAVITY_BREAK) && !ids.contains(&67));
        // The equipped copy stays; KH1's later (unequipped) copy goes.
        let list = [kh1::GUARD, kh1::SLIDING_DASH, kh1::COMBO_PLUS, kh1::SLIDING_DASH | 0x80, kh1::COMBO_PLUS, 0];
        let (out, removed) = keep_one(&list, &ids);
        assert_eq!(out, vec![kh1::GUARD, kh1::SLIDING_DASH, kh1::COMBO_PLUS, kh1::COMBO_PLUS, 0, 0]);
        assert_eq!(removed, vec![kh1::SLIDING_DASH]);
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
    fn grants_follow_level_and_lists() {
        let g = sora(13, &[], &[kh1::HIGH_JUMP]).grants();
        let ids: Vec<u8> = g.iter().map(|a| a.id).collect();
        // (Aerial Dodge is Genie Jafar's gift, not a level's.)
        assert_eq!(ids, vec![66, 68, 69]);
        // Already learned (even unequipped): not again.
        let g = sora(13, &[68 | 0x80, 66 | 0x80], &[kh1::HIGH_JUMP]).grants();
        assert_eq!(g.iter().map(|a| a.id).collect::<Vec<_>>(), vec![69]);
        // Genie Jafar beaten: Aerial Dodge.
        let mut jafar = sora(30, &[], &[]);
        jafar.progress = vec![0; 12];
        jafar.progress[4] = 0x52;
        assert!(!jafar.boss_gifts().iter().any(|(_, id)| *id == 67));
        jafar.progress[4] = 0x53;
        assert!(jafar.boss_gifts().iter().any(|(_, id)| *id == 67));
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
