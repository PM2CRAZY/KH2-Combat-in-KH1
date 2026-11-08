//! Third-party add-ons (testing-merge tm44; the owner, chat 2026-11-07: "Support third-party
//! add-ons: yes", "each add-on gets its own row", "(c) both": a data file for the data and a small
//! versioned C API for runtime questions). Author's guide: `docs/addons.md`; C header:
//! `crates/kh1-hook/include/kh1kh2_addon.h`.
//!
//! An add-on is a folder with an `addon.toml` (`parse`): its row in KH1's own Abilities menu (after
//! "KH2", `win/kh2list.rs`), its abilities (name, help, AP, when Sora learns them) and, optionally,
//! a finisher per ability (a KH2 motion file and its hits). The mod reads the files once at start
//! (`load`); nothing of an add-on's runs in the game's frame. Add-on code, if there is any, only
//! asks questions through the C API (`kh1kh2_addon_api`), answered from a copy of the lists the
//! menu's once-a-second step makes (`set_snapshot`), never from game memory on the caller's thread.
//!
//! **Ids.** KH1 keeps abilities as 7-bit ids (bit 7: learned, not equipped; 0 ends a list): KH1's
//! own are 1..=65, the mod's KH2 ones 66..=92 (`growth::MENU`). An add-on names its abilities with
//! ids of its own, 1..=127 (`Ability::id`, fixed by its author forever), and those local ids are
//! what the save keeps, in the add-on's own 48-byte list. At start, each loaded ability is given a
//! runtime id in KH1's ability table from the free ones, 93..=127 (`FIRST_GLOBAL`, 35 in all,
//! in load order); KH1's menu code only ever sees runtime ids, and only for the length of one of
//! its own calls (`Addon::to_global` going in, `Addon::to_local` coming back). So adding, removing
//! or reordering add-ons changes runtime ids but never what a saved byte means.
//!
//! **Save.** KH1's config block (save sub-block 5, `0x400` bytes, saved with the slot) is used by
//! KH1 only up to `+0x78` (Checked (decomp, Steam): every access through its pointer
//! `0x142E1F4D8` is below `+0x78`, `docs/kh1-config-menu.md` 1.1); the mod's KH2 list is
//! `+0x3C0..+0x3EF`. The add-ons' area is `+0x78..+0x38F` (`SAVE_OFF`, `AREA_LEN`): an 8-byte
//! header ("K2AD", layout 1, the slot count) and 14 slots of 56 bytes, each the add-on's key hash
//! (8 bytes, `key_hash`) and its 48-byte list of local ids. An add-on finds its slot by its hash or
//! takes a free one (hash 0). A removed add-on's slot is left as it is (its row simply isn't
//! shown); put back, it finds its slot and its abilities again. The area is only written once an
//! add-on is installed, and only when it is all zero (a new or never-touched save) or already ours.

use crate::growth::{self, BossGift};
use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

/// The first runtime id (after `growth::MENU`'s 66..=92).
pub const FIRST_GLOBAL: u8 = 93;
/// The last runtime id (bit 7 of a list byte is "not equipped").
pub const LAST_GLOBAL: u8 = growth::MAX_ID;
/// Runtime ids for all add-ons together.
pub const GLOBAL_IDS: usize = (LAST_GLOBAL - FIRST_GLOBAL + 1) as usize;
/// Add-on rows in KH1's Abilities menu (after the KH2 row). Each is a member id of ours past the
/// save's ten party records (11, 12, 13; the KH2 row is 10), whose record arithmetic in KH1's page
/// lands on save bytes the page doesn't read (`win/kh2list.rs`), and its row count goes past KH1's
/// four (rows 4..7, the bytes after KH1's array kept and put back around each page call).
/// Assumption: three more rows fit KH1's member picker on screen (not seen in game yet).
pub const MAX_ROWS: usize = 3;
/// A list's length (KH1's: 48 bytes, as Sora's own and the KH2 list).
pub const LIST_LEN: usize = 48;
/// The add-ons' area in KH1's config block.
pub const SAVE_OFF: usize = 0x78;
pub const SAVE_MAGIC: [u8; 4] = *b"K2AD";
pub const SAVE_LAYOUT: u8 = 1;
pub const SLOTS: usize = 14;
pub const SLOT_LEN: usize = 8 + LIST_LEN;
pub const AREA_LEN: usize = 8 + SLOTS * SLOT_LEN;
/// The data file's format version (`[addon] format`): format 1 is never changed; new fields come
/// as format 2 beside it.
pub const FORMAT: i64 = 1;
/// Longest texts KH1's menu shows well (Assumption: from KH1's own longest names and help lines).
pub const NAME_MAX: usize = 24;
pub const TITLE_MAX: usize = 12;
pub const HELP_MAX: usize = 96;

// ---- the data ----

/// When Sora learns an ability (`learned` in the file).
#[derive(Clone, Copy, Debug)]
pub enum Learn {
    /// From the start (any save, at once).
    Start,
    /// At Sora's level.
    Level(u8),
    /// Once a boss is beaten (a world's story progress byte, `growth::beaten`).
    Boss(BossGift),
}

/// What KH1's menu shows it as: the icon and kind of KH1's own entry it is made from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    /// As KH1's Slapshot (an action ability).
    Action,
    /// As KH1's MP Haste (a support ability).
    Support,
}

impl Icon {
    /// KH1's ability whose table entry it is made from (as `growth::MENU`'s templates).
    pub fn template(self) -> u8 {
        match self {
            Icon::Action => growth::kh1::SLAPSHOT,
            Icon::Support => growth::kh1::MP_HASTE,
        }
    }
}

/// Where a finisher looks for its foe (KH2's combo row kinds, `character_core::ComboRow::kind`):
/// none (kind 3, no range check), the main target (kind 1) or any foe (kind 2), `near..far` units
/// away (KH2's units, less the foe's radius).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Range {
    None,
    Primary { near: f32, far: f32 },
    Any { near: f32, far: f32 },
}

/// How a finisher's hits move a foe (KH1's reaction kinds on our attack data, `win/kh1rows.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reaction {
    /// KH1's flinch (kind 3).
    Flinch,
    /// KH1's launch (kind 5).
    Launch,
}

/// An ability's finisher (`[ability.finisher]`).
#[derive(Clone, Debug, PartialEq)]
pub struct Finisher {
    /// Sora's body motion: a KH2 ANB file (`docs/addons.md`), relative to the add-on's folder.
    pub motion: String,
    /// The Keyblade's motion (an ANB for KH2's Keyblade bones), if the move moves it on its own.
    pub keyblade: Option<String>,
    /// An air finisher (else on the ground).
    pub air: bool,
    pub range: Range,
    /// Hit windows, 60 Hz frames from the motion's start (start inclusive, end exclusive).
    pub hits: Vec<(u32, u32)>,
    /// KH2's attack power factor for its hits (as KH2's ATKP `power`).
    pub power: f32,
    pub reaction: Reaction,
    /// (frame, element) of Sora's KH2 effect package (`obj/P_EX100.a.us`).
    pub effects: Vec<(u32, u32)>,
    /// (frame, sound id) of KH2's sound banks the mod reads (`win/kh2sound.rs`).
    pub sounds: Vec<(u32, u32)>,
    /// The motion's length in 60 Hz frames, once its file was read (`load`).
    pub frames: Option<u32>,
}

/// One ability of an add-on.
#[derive(Clone, Debug, PartialEq)]
pub struct Ability {
    pub key: String,
    /// Its id in the add-on's own list in the save (1..=127), fixed by the author.
    pub id: u8,
    pub name: String,
    pub help: String,
    pub ap: u8,
    pub learned: Learn,
    pub icon: Icon,
    pub finisher: Option<Finisher>,
    /// Its runtime id in KH1's ability table (`FIRST_GLOBAL..`), given at load; 0 before.
    pub global: u8,
}

/// One add-on.
#[derive(Clone, Debug, PartialEq)]
pub struct Addon {
    pub key: String,
    pub hash: u64,
    pub name: String,
    /// Its row's title in KH1's Abilities menu.
    pub title: String,
    pub version: String,
    /// Where it was read from (for the log).
    pub folder: String,
    pub abilities: Vec<Ability>,
}

/// Every add-on loaded, in row order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Loaded {
    pub addons: Vec<Addon>,
}

/// An add-on key's hash as the save keeps it (FNV-1a 64 of the key; never 0, 0 is a free slot).
pub fn key_hash(key: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in key.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    if h == 0 {
        1
    } else {
        h
    }
}

/// A key: 3..=64 of `a-z 0-9 . _ -`.
pub fn valid_key(k: &str) -> bool {
    (3..=64).contains(&k.len()) && k.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
}

/// Characters KH1's text codes have (`win/menu.rs` `encode`; others show as a space).
pub fn kh1_text_ok(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_alphanumeric() || " -.,'!?()/:+&".contains(c))
}

/// Every boss the file may name for `learned = { boss = "..." }`: the bosses whose story points the
/// mod already uses (`growth`'s `BONUS_SCHEDULE`, `SPLIT_SCHEDULE`, `GENIE_JAFAR`, `CHERNABOG`),
/// with the same world and progress values (Assumption there: main's story bytes, `growth`).
pub fn bosses() -> Vec<BossGift> {
    let mut out: Vec<BossGift> = Vec::new();
    let mut add = |g: BossGift| {
        if !out.iter().any(|o| o.boss == g.boss) {
            out.push(g);
        }
    };
    for (_, l) in growth::BONUS_SCHEDULE.iter() {
        if let growth::Learned::Boss(g) = l {
            add(*g);
        }
    }
    for (_, g) in growth::SPLIT_SCHEDULE.iter() {
        match g {
            growth::Gift::Main(growth::Learned::Boss(b)) | growth::Gift::LevelOrBoss(_, b) => add(*b),
            _ => {}
        }
    }
    add(growth::GENIE_JAFAR);
    add(growth::CHERNABOG);
    out
}

/// A boss by its name (case and spaces ignored).
pub fn boss(name: &str) -> Option<BossGift> {
    let norm = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_lowercase();
    let n = norm(name);
    bosses().into_iter().find(|b| norm(b.boss) == n)
}

impl PartialEq for Learn {
    fn eq(&self, o: &Learn) -> bool {
        match (self, o) {
            (Learn::Start, Learn::Start) => true,
            (Learn::Level(a), Learn::Level(b)) => a == b,
            (Learn::Boss(a), Learn::Boss(b)) => a.boss == b.boss && a.world == b.world && a.progress == b.progress,
            _ => false,
        }
    }
}

impl Learn {
    /// Whether it is due at Sora's `level` and the story `progress`.
    pub fn due(&self, level: u8, progress: &[u8]) -> bool {
        match self {
            Learn::Start => true,
            Learn::Level(l) => level >= *l,
            Learn::Boss(g) => growth::beaten(progress, g),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Learn::Start => "from the start".into(),
            Learn::Level(l) => format!("at level {l}"),
            Learn::Boss(g) => format!("for {}", g.boss),
        }
    }
}

// ---- reading a file ----

type Table = toml_edit::Table;
type Item = toml_edit::Item;

fn get_str(t: &Table, k: &str) -> Option<String> {
    t.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn get_int(t: &Table, k: &str) -> Option<i64> {
    t.get(k).and_then(|v| v.as_integer())
}

fn get_num(t: &Table, k: &str) -> Option<f64> {
    t.get(k).and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
}

/// A table or an inline table as one (`learned = { level = 12 }` / `[ability.learned]`).
fn as_table(i: &Item) -> Option<Table> {
    match i {
        Item::Table(t) => Some(t.clone()),
        Item::Value(toml_edit::Value::InlineTable(t)) => Some(t.clone().into_table()),
        _ => None,
    }
}

/// A list of `[a, b]` integer pairs.
fn pairs(t: &Table, k: &str, what: &str) -> Result<Vec<(u32, u32)>, String> {
    let Some(v) = t.get(k) else { return Ok(Vec::new()) };
    let arr = v.as_array().ok_or_else(|| format!("{what}: `{k}` must be a list of [a, b] pairs"))?;
    arr.iter()
        .map(|p| {
            let p = p.as_array().filter(|p| p.len() == 2).ok_or_else(|| format!("{what}: each `{k}` entry must be [a, b]"))?;
            let n = |i: usize| p.get(i).and_then(|x| x.as_integer()).filter(|x| (0..=i64::from(u32::MAX)).contains(x)).map(|x| x as u32);
            match (n(0), n(1)) {
                (Some(a), Some(b)) => Ok((a, b)),
                _ => Err(format!("{what}: `{k}` entries must be two whole numbers, 0 or more")),
            }
        })
        .collect()
}

fn text(t: &Table, k: &str, max: usize, what: &str, warn: &mut Vec<String>) -> Option<String> {
    let mut s = get_str(t, k)?.trim().to_string();
    if s.chars().count() > max {
        warn.push(format!("{what}: `{k}` cut to {max} characters"));
        s = s.chars().take(max).collect();
    }
    if !kh1_text_ok(&s) {
        warn.push(format!("{what}: `{k}` has characters KH1's menu font doesn't have (shown as spaces): \"{s}\""));
    }
    Some(s)
}

fn learned(t: &Table, what: &str) -> Result<Learn, String> {
    let Some(i) = t.get("learned") else { return Err(format!("{what}: no `learned`")) };
    if let Some(s) = i.as_str() {
        return match s.trim().to_ascii_lowercase().as_str() {
            "start" => Ok(Learn::Start),
            _ => Err(format!("{what}: `learned = \"{s}\"`: write \"start\", {{ level = N }} or {{ boss = \"Name\" }}")),
        };
    }
    let lt = as_table(i).ok_or_else(|| format!("{what}: `learned` must be \"start\", {{ level = N }} or {{ boss = \"Name\" }}"))?;
    match (get_int(&lt, "level"), get_str(&lt, "boss")) {
        (Some(l), None) if (1..=100).contains(&l) => Ok(Learn::Level(l as u8)),
        (Some(l), None) => Err(format!("{what}: `learned` level {l} isn't 1..=100")),
        (None, Some(b)) => boss(&b).map(Learn::Boss).ok_or_else(|| {
            format!("{what}: `learned` boss \"{b}\" isn't one the mod knows ({})", bosses().iter().map(|b| b.boss).collect::<Vec<_>>().join(", "))
        }),
        _ => Err(format!("{what}: `learned` needs exactly one of `level` or `boss`")),
    }
}

fn finisher(t: &Table, what: &str) -> Result<Finisher, String> {
    let motion = get_str(t, "motion").filter(|s| !s.trim().is_empty()).ok_or_else(|| format!("{what}: finisher has no `motion`"))?;
    let rel_ok = |p: &str| !p.contains("..") && !p.starts_with('/') && !p.starts_with('\\') && !p.contains(':');
    if !rel_ok(&motion) {
        return Err(format!("{what}: finisher `motion` must be a path inside the add-on's folder"));
    }
    let keyblade = get_str(t, "keyblade");
    if keyblade.as_deref().is_some_and(|k| !rel_ok(k)) {
        return Err(format!("{what}: finisher `keyblade` must be a path inside the add-on's folder"));
    }
    let air = match get_str(t, "kind").as_deref().map(|s| s.to_ascii_lowercase()) {
        None => false,
        Some(k) if k == "ground" => false,
        Some(k) if k == "air" => true,
        Some(k) => return Err(format!("{what}: finisher `kind = \"{k}\"`: \"ground\" or \"air\"")),
    };
    let range = match t.get("range").and_then(as_table) {
        None => Range::None,
        Some(r) => {
            let kind = get_str(&r, "kind").unwrap_or_else(|| "none".into()).to_ascii_lowercase();
            let near = get_num(&r, "near").unwrap_or(0.0) as f32;
            let far = get_num(&r, "far").unwrap_or(0.0) as f32;
            let span = || -> Result<(), String> {
                if near.is_finite() && far.is_finite() && near >= 0.0 && far > near && far <= 5000.0 {
                    Ok(())
                } else {
                    Err(format!("{what}: finisher range needs 0 <= near < far <= 5000"))
                }
            };
            match kind.as_str() {
                "none" => Range::None,
                "primary" => {
                    span()?;
                    Range::Primary { near, far }
                }
                "any" => {
                    span()?;
                    Range::Any { near, far }
                }
                k => return Err(format!("{what}: finisher range kind \"{k}\": \"none\", \"primary\" or \"any\"")),
            }
        }
    };
    let hits = pairs(t, "hits", what)?;
    if hits.is_empty() {
        return Err(format!("{what}: finisher has no `hits`"));
    }
    if hits.iter().any(|(a, b)| b <= a || *b > 600) {
        return Err(format!("{what}: each hit window must be [start, end] with start < end <= 600 (60 Hz frames)"));
    }
    let power = get_num(t, "power").unwrap_or(1.0) as f32;
    if !(power.is_finite() && (0.0..=10.0).contains(&power)) {
        return Err(format!("{what}: finisher `power` must be 0..=10"));
    }
    let reaction = match get_str(t, "reaction").map(|s| s.to_ascii_lowercase()).as_deref() {
        None | Some("flinch") => Reaction::Flinch,
        Some("launch") => Reaction::Launch,
        Some(r) => return Err(format!("{what}: finisher `reaction = \"{r}\"`: \"flinch\" or \"launch\"")),
    };
    let effects = pairs(t, "effects", what)?;
    let sounds = pairs(t, "sounds", what)?;
    Ok(Finisher { motion, keyblade, air, range, hits, power, reaction, effects, sounds, frames: None })
}

/// Reads one `addon.toml` (`folder`: where it was, for the log). An error in the `[addon]` table
/// refuses the whole add-on; one in an ability refuses that ability; one in a finisher drops the
/// finisher and keeps the ability. The warnings say what was changed or left out.
pub fn parse(text_in: &str, folder: &str) -> Result<(Addon, Vec<String>), String> {
    let doc: toml_edit::DocumentMut = text_in.parse().map_err(|e: toml_edit::TomlError| format!("{folder}: not valid TOML: {}", e.to_string().trim()))?;
    let mut warn = Vec::new();
    let head = doc.get("addon").and_then(as_table).ok_or_else(|| format!("{folder}: no [addon] table"))?;
    match get_int(&head, "format") {
        Some(FORMAT) => {}
        Some(f) => return Err(format!("{folder}: data format {f}; this build reads format {FORMAT}")),
        None => return Err(format!("{folder}: [addon] needs `format = {FORMAT}`")),
    }
    let key = get_str(&head, "key").ok_or_else(|| format!("{folder}: [addon] has no `key`"))?;
    if !valid_key(&key) {
        return Err(format!("{folder}: key \"{key}\" must be 3..=64 of a-z 0-9 . _ -"));
    }
    let what = format!("add-on {key}");
    let name = text(&head, "name", 64, &what, &mut warn).unwrap_or_else(|| key.clone());
    let title = text(&head, "row", TITLE_MAX, &what, &mut warn).filter(|s| !s.is_empty()).ok_or_else(|| format!("{what}: [addon] has no `row` (its title in the Abilities menu)"))?;
    let version = get_str(&head, "version").unwrap_or_default();
    let mut abilities: Vec<Ability> = Vec::new();
    let list = match doc.get("ability") {
        None => Vec::new(),
        Some(Item::ArrayOfTables(a)) => a.iter().cloned().collect(),
        Some(_) => return Err(format!("{what}: `ability` must be [[ability]] tables")),
    };
    for (n, t) in list.iter().enumerate() {
        let one = (|| -> Result<Ability, String> {
            let akey = get_str(t, "key").ok_or_else(|| format!("{what}: ability #{} has no `key`", n + 1))?;
            if !valid_key(&akey) {
                return Err(format!("{what}: ability key \"{akey}\" must be 3..=64 of a-z 0-9 . _ -"));
            }
            let w = format!("{what}, ability {akey}");
            let id = get_int(t, "id").ok_or_else(|| format!("{w}: no `id`"))?;
            if !(1..=127).contains(&id) {
                return Err(format!("{w}: id {id} isn't 1..=127"));
            }
            let name = text(t, "name", NAME_MAX, &w, &mut warn).filter(|s| !s.is_empty()).ok_or_else(|| format!("{w}: no `name`"))?;
            let help = text(t, "help", HELP_MAX, &w, &mut warn).unwrap_or_default();
            let ap = get_int(t, "ap").ok_or_else(|| format!("{w}: no `ap`"))?;
            if !(0..=99).contains(&ap) {
                return Err(format!("{w}: ap {ap} isn't 0..=99"));
            }
            let learned = learned(t, &w)?;
            let icon = match get_str(t, "icon").map(|s| s.to_ascii_lowercase()).as_deref() {
                None | Some("action") => Icon::Action,
                Some("support") => Icon::Support,
                Some(i) => return Err(format!("{w}: icon \"{i}\": \"action\" or \"support\"")),
            };
            let finisher = match t.get("finisher").and_then(as_table) {
                None => None,
                Some(f) => match finisher(&f, &w) {
                    Ok(f) => Some(f),
                    Err(e) => {
                        warn.push(format!("{e}: the ability is kept, without its finisher"));
                        None
                    }
                },
            };
            Ok(Ability { key: akey, id: id as u8, name, help, ap: ap as u8, learned, icon, finisher, global: 0 })
        })();
        match one {
            Ok(a) if abilities.iter().any(|b| b.key == a.key) => warn.push(format!("{what}: ability key {} twice: the second left out", a.key)),
            Ok(a) if abilities.iter().any(|b| b.id == a.id) => warn.push(format!("{what}: ability id {} twice ({}): the second left out", a.id, a.key)),
            Ok(a) => abilities.push(a),
            Err(e) => warn.push(format!("{e}: ability left out")),
        }
    }
    if abilities.len() > LIST_LEN {
        warn.push(format!("{what}: {} abilities, a row holds {LIST_LEN}: the rest left out", abilities.len()));
        abilities.truncate(LIST_LEN);
    }
    abilities.sort_by_key(|a| a.id);
    Ok((Addon { hash: key_hash(&key), key, name, title, version, folder: folder.to_string(), abilities }, warn))
}

/// A KH2 motion file the mod can play (`docs/addons.md`): an ANB, one entry of a KH2 `.mset` (a BAR
/// with an interpolated motion, type 9, and optionally its triggers, type 16), as KH2's own
/// motions are read for KH1's motion slots (`kh2_formats::motion::Anb`, `kh2_data::sora`).
/// Its length in 60 Hz frames.
pub fn motion_frames(bytes: &[u8]) -> Result<u32, String> {
    let anb = kh2_formats::motion::Anb::parse(bytes).map_err(|e| format!("not a KH2 ANB ({e})"))?;
    let m = anb.motion.ok_or("no motion entry (BAR type 9) in it")?;
    if m.kind != 0 || anb.pose.is_none() {
        return Err("its motion isn't an interpolated one (KH2's raw motions aren't read)".into());
    }
    if !(m.fps > 0.0 && m.frame_end > m.frame_start) {
        return Err("its motion has no frame range".into());
    }
    Ok(((m.frame_end - m.frame_start) / m.fps * 60.0).round().max(1.0) as u32)
}

/// Loads add-ons from their files: `found` is (folder, its `addon.toml`'s text) in load order;
/// `read` reads a file in an add-on's folder (folder, relative path). Refused add-ons and
/// abilities are logged in the lines returned; nothing here is fatal. Row limit `MAX_ROWS`;
/// runtime ids from `FIRST_GLOBAL` (an add-on whose abilities don't all fit is refused whole).
pub fn load(found: &[(String, String)], read: &dyn Fn(&str, &str) -> Option<Vec<u8>>) -> (Loaded, Vec<String>) {
    let mut out = Loaded::default();
    let mut lines = Vec::new();
    let mut next = FIRST_GLOBAL as usize;
    for (folder, text) in found {
        let (mut a, warn) = match parse(text, folder) {
            Ok(x) => x,
            Err(e) => {
                lines.push(format!("refused: {e}"));
                continue;
            }
        };
        lines.extend(warn.into_iter().map(|w| format!("note: {w}")));
        if let Some(o) = out.addons.iter().find(|o| o.key == a.key || o.hash == a.hash) {
            lines.push(format!("refused: {} ({folder}): the key is {}'s too ({})", a.key, o.key, o.folder));
            continue;
        }
        if out.addons.len() >= MAX_ROWS {
            lines.push(format!("refused: {} ({folder}): {MAX_ROWS} add-on rows already (the Abilities menu's limit)", a.key));
            continue;
        }
        if next + a.abilities.len() > LAST_GLOBAL as usize + 1 {
            lines.push(format!("refused: {} ({folder}): {} abilities, {} ability ids left of {GLOBAL_IDS} for all add-ons", a.key, a.abilities.len(), LAST_GLOBAL as usize + 1 - next));
            continue;
        }
        for ab in a.abilities.iter_mut() {
            ab.global = next as u8;
            next += 1;
            if let Some(f) = ab.finisher.as_mut() {
                let checked = (|| -> Result<u32, String> {
                    let bytes = read(folder, &f.motion).ok_or_else(|| format!("its motion file {} isn't there", f.motion))?;
                    let frames = motion_frames(&bytes).map_err(|e| format!("its motion file {}: {e}", f.motion))?;
                    if let Some(k) = &f.keyblade {
                        let kb = read(folder, k).ok_or_else(|| format!("its Keyblade motion file {k} isn't there"))?;
                        motion_frames(&kb).map_err(|e| format!("its Keyblade motion file {k}: {e}"))?;
                    }
                    if let Some((a, b)) = f.hits.iter().find(|(_, b)| *b > frames) {
                        return Err(format!("its hit window [{a}, {b}] runs past the motion's {frames} frames"));
                    }
                    Ok(frames)
                })();
                match checked {
                    Ok(n) => f.frames = Some(n),
                    Err(e) => {
                        lines.push(format!("note: add-on {}, ability {}: {e}: kept without its finisher", a.key, ab.key));
                        ab.finisher = None;
                    }
                }
            }
        }
        lines.push(format!(
            "loaded: {} \"{}\" {} ({folder}): row \"{}\", {} abilities (ids {}), {} with a finisher",
            a.key,
            a.name,
            a.version,
            a.title,
            a.abilities.len(),
            a.abilities.iter().map(|x| format!("{}={}", x.id, x.global)).collect::<Vec<_>>().join(" "),
            a.abilities.iter().filter(|x| x.finisher.is_some()).count()
        ));
        out.addons.push(a);
    }
    (out, lines)
}

impl Addon {
    pub fn by_local(&self, id: u8) -> Option<&Ability> {
        self.abilities.iter().find(|a| a.id == id & 0x7F)
    }

    pub fn by_global(&self, id: u8) -> Option<&Ability> {
        let id = id & 0x7F;
        self.abilities.iter().find(|a| a.global == id && id != 0)
    }

    pub fn by_key(&self, key: &str) -> Option<&Ability> {
        self.abilities.iter().find(|a| a.key == key)
    }

    /// The saved list (local ids) as KH1's menu sees it (runtime ids, bit 7 kept), and the saved
    /// bytes of abilities this add-on no longer has (left out of the menu, kept in the save:
    /// `to_local` puts them back).
    pub fn to_global(&self, local: &[u8]) -> ([u8; LIST_LEN], Vec<u8>) {
        let mut out = [0u8; LIST_LEN];
        let mut hidden = Vec::new();
        let mut n = 0;
        for &b in local.iter().take(LIST_LEN).take_while(|b| **b != 0) {
            match self.by_local(b) {
                Some(a) => {
                    out[n] = a.global | (b & 0x80);
                    n += 1;
                }
                None => hidden.push(b),
            }
        }
        (out, hidden)
    }

    /// KH1's menu's list (runtime ids) back as the save keeps it (local ids), with `hidden` after.
    /// A byte that isn't one of this add-on's is dropped (KH1's page only moves bits and order).
    pub fn to_local(&self, global: &[u8], hidden: &[u8]) -> [u8; LIST_LEN] {
        let mut out = [0u8; LIST_LEN];
        let mut n = 0;
        let mapped = global.iter().take(LIST_LEN).take_while(|b| **b != 0).filter_map(|&b| self.by_global(b).map(|a| a.id | (b & 0x80)));
        for b in mapped.chain(hidden.iter().copied()) {
            if n < LIST_LEN {
                out[n] = b;
                n += 1;
            }
        }
        out
    }

    /// The abilities due now that its saved `list` doesn't hold (local ids, in id order).
    pub fn due(&self, list: &[u8], level: u8, progress: &[u8]) -> Vec<u8> {
        self.abilities.iter().filter(|a| a.learned.due(level, progress) && !growth::has(list, a.id)).map(|a| a.id).collect()
    }

    /// Equipped copies of its ability `id` in its saved list.
    pub fn equipped(&self, list: &[u8], id: u8) -> u8 {
        growth::equipped(list, id)
    }

    /// AP of the equipped abilities in its saved list.
    pub fn ap_on(&self, list: &[u8]) -> i32 {
        list.iter().take_while(|b| **b != 0).filter(|b| **b & 0x80 == 0).filter_map(|&b| self.by_local(b)).map(|a| a.ap as i32).sum()
    }
}

impl Loaded {
    /// The add-on and ability of runtime id `id`.
    pub fn by_global(&self, id: u8) -> Option<(&Addon, &Ability)> {
        self.addons.iter().find_map(|a| a.by_global(id).map(|ab| (a, ab)))
    }

    pub fn by_key(&self, key: &str) -> Option<&Addon> {
        self.addons.iter().find(|a| a.key == key)
    }
}

// ---- the save area ----

/// What the config block's add-on area holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaState {
    /// Our header.
    Ours,
    /// All zero (a new save, or one no build of ours has written to).
    Empty,
    /// Something else's bytes: left alone (no add-on rows on that save).
    Foreign,
}

pub fn area_state(a: &[u8]) -> AreaState {
    if a.len() < AREA_LEN {
        return AreaState::Foreign;
    }
    if a[..4] == SAVE_MAGIC && a[4] == SAVE_LAYOUT && a[5] as usize == SLOTS {
        AreaState::Ours
    } else if a[..AREA_LEN].iter().all(|b| *b == 0) {
        AreaState::Empty
    } else {
        AreaState::Foreign
    }
}

/// Our header written on an empty area; false if it is someone else's.
pub fn claim(a: &mut [u8]) -> bool {
    match area_state(a) {
        AreaState::Ours => true,
        AreaState::Empty => {
            a[..4].copy_from_slice(&SAVE_MAGIC);
            a[4] = SAVE_LAYOUT;
            a[5] = SLOTS as u8;
            true
        }
        AreaState::Foreign => false,
    }
}

fn slot_hash(a: &[u8], i: usize) -> u64 {
    let at = 8 + i * SLOT_LEN;
    u64::from_le_bytes(a[at..at + 8].try_into().unwrap_or([0; 8]))
}

/// The slot of the add-on with `hash`, if the area has one.
pub fn find_slot(a: &[u8], hash: u64) -> Option<usize> {
    (area_state(a) == AreaState::Ours).then(|| (0..SLOTS).find(|&i| slot_hash(a, i) == hash)).flatten()
}

/// Its slot, a free one taken (and the area claimed) if it has none; `None` if every slot is
/// another add-on's or the area isn't ours to write.
pub fn slot_for(a: &mut [u8], hash: u64) -> Option<usize> {
    if let Some(i) = find_slot(a, hash) {
        return Some(i);
    }
    if !claim(a) {
        return None;
    }
    let i = (0..SLOTS).find(|&i| slot_hash(a, i) == 0)?;
    let at = 8 + i * SLOT_LEN;
    a[at..at + 8].copy_from_slice(&hash.to_le_bytes());
    a[at + 8..at + SLOT_LEN].fill(0);
    Some(i)
}

/// A slot's list bytes.
pub fn slot_list(a: &[u8], i: usize) -> &[u8] {
    let at = 8 + i * SLOT_LEN + 8;
    &a[at..at + LIST_LEN]
}

pub fn set_slot_list(a: &mut [u8], i: usize, list: &[u8]) {
    let at = 8 + i * SLOT_LEN + 8;
    for k in 0..LIST_LEN {
        a[at + k] = list.get(k).copied().unwrap_or(0);
    }
}

/// Adds `byte` at the list's first free place; false if it is full.
pub fn push(list: &mut [u8], byte: u8) -> bool {
    match list.iter().position(|b| *b == 0) {
        Some(i) => {
            list[i] = byte;
            true
        }
        None => false,
    }
}

// ---- the loaded set, the API's copy of the lists ----

static LOADED: OnceLock<Loaded> = OnceLock::new();

/// Sets the loaded add-ons (once, at start).
pub fn set_loaded(l: Loaded) {
    let _ = LOADED.set(l);
}

/// The loaded add-ons (`None` before start, or none installed).
pub fn loaded() -> Option<&'static Loaded> {
    LOADED.get().filter(|l| !l.addons.is_empty())
}

/// Each add-on's saved list (local ids) as the menu's last once-a-second step saw it.
static SNAPSHOT: Mutex<Option<Vec<(u64, [u8; LIST_LEN])>>> = Mutex::new(None);

/// The lists now (`None`: no save in play).
pub fn set_snapshot(s: Option<Vec<(u64, [u8; LIST_LEN])>>) {
    if let Ok(mut g) = SNAPSHOT.lock() {
        *g = s;
    }
}

fn snapshot_list(hash: u64) -> Option<[u8; LIST_LEN]> {
    SNAPSHOT.lock().ok()?.as_ref()?.iter().find(|(h, _)| *h == hash).map(|(_, l)| *l)
}

/// The add-on's log sink (`win` sets it: the mod's log, General).
static LOG_SINK: OnceLock<fn(&str)> = OnceLock::new();

pub fn set_log_sink(f: fn(&str)) {
    let _ = LOG_SINK.set(f);
}

// ---- the C API ----

/// The ready callback: the API table and the add-on's own pointer.
pub type ReadyFn = unsafe extern "C" fn(api: *const ApiV1, user: *mut c_void);

/// Version 1 of the API (`kh1kh2_addon_api(1)`); never changed once released. Field for field as
/// `kh1kh2_addon_api_v1` in `include/kh1kh2_addon.h`.
#[repr(C)]
pub struct ApiV1 {
    /// `size_of::<ApiV1>()`.
    pub size: u32,
    /// 1.
    pub version: u32,
    /// Equipped copies of an ability (0 or more); -1 unknown (no save in play, add-on or ability not
    /// loaded, a bad argument).
    pub is_equipped: unsafe extern "C" fn(addon_key: *const c_char, ability_key: *const c_char) -> i32,
    /// 1 learned (in its row, equipped or not), 0 not; -1 unknown.
    pub is_learned: unsafe extern "C" fn(addon_key: *const c_char, ability_key: *const c_char) -> i32,
    /// 1 the add-on is loaded with its row, 0 not (not installed, or refused: see the log); -1 a bad
    /// argument.
    pub addon_loaded: unsafe extern "C" fn(addon_key: *const c_char) -> i32,
    /// The ability's id in its add-on's list (the file's `id`, 1..=127, the same every run), 0 not
    /// loaded; -1 a bad argument.
    pub ability_id: unsafe extern "C" fn(addon_key: *const c_char, ability_key: *const c_char) -> i32,
    /// Calls `cb(api, user)` once, on KH1's game thread at the end of a frame, after the add-ons are
    /// loaded (the first frame end after this call). 1 queued, 0 not (`cb` null, or 64 waiting).
    pub on_ready: unsafe extern "C" fn(cb: Option<ReadyFn>, user: *mut c_void) -> i32,
    /// A line in the mod's log (`kh1-kh2.log`, with the debug tools), as "[add-on <key>] <msg>".
    pub log: unsafe extern "C" fn(addon_key: *const c_char, msg: *const c_char),
}

struct Api(ApiV1);
// (Only function pointers and numbers.)
unsafe impl Sync for Api {}

static API_V1: Api = Api(ApiV1 {
    size: std::mem::size_of::<ApiV1>() as u32,
    version: 1,
    is_equipped: api_is_equipped,
    is_learned: api_is_learned,
    addon_loaded: api_addon_loaded,
    ability_id: api_ability_id,
    on_ready: api_on_ready,
    log: api_log,
});

/// The add-on API's table for `version` (1), or null for a version this build doesn't have. Safe to
/// call from any thread, any time after this DLL is loaded (not from the add-on's `DllMain`: the
/// loader's lock is held there, `docs/addons.md`).
#[no_mangle]
pub extern "C" fn kh1kh2_addon_api(version: u32) -> *const ApiV1 {
    match version {
        1 => &API_V1.0,
        _ => std::ptr::null(),
    }
}

/// A C string argument (null or not UTF-8: `None`).
unsafe fn arg<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    CStr::from_ptr(p).to_str().ok()
}

/// `f` with a panic of ours caught at the boundary (`fallback` then).
fn guard<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or(fallback)
}

fn lookup(addon: &str, ability: &str) -> Option<(u64, u8)> {
    let a = loaded()?.by_key(addon)?;
    a.by_key(ability).map(|ab| (a.hash, ab.id))
}

unsafe extern "C" fn api_is_equipped(addon: *const c_char, ability: *const c_char) -> i32 {
    guard(-1, || {
        let (Some(a), Some(b)) = (arg(addon), arg(ability)) else { return -1 };
        let Some((hash, id)) = lookup(a, b) else { return -1 };
        snapshot_list(hash).map_or(-1, |l| growth::equipped(&l, id) as i32)
    })
}

unsafe extern "C" fn api_is_learned(addon: *const c_char, ability: *const c_char) -> i32 {
    guard(-1, || {
        let (Some(a), Some(b)) = (arg(addon), arg(ability)) else { return -1 };
        let Some((hash, id)) = lookup(a, b) else { return -1 };
        snapshot_list(hash).map_or(-1, |l| growth::has(&l, id) as i32)
    })
}

unsafe extern "C" fn api_addon_loaded(addon: *const c_char) -> i32 {
    guard(-1, || match arg(addon) {
        None => -1,
        Some(a) => loaded().and_then(|l| l.by_key(a)).is_some() as i32,
    })
}

unsafe extern "C" fn api_ability_id(addon: *const c_char, ability: *const c_char) -> i32 {
    guard(-1, || {
        let (Some(a), Some(b)) = (arg(addon), arg(ability)) else { return -1 };
        lookup(a, b).map_or(0, |(_, id)| id as i32)
    })
}

/// Ready callbacks waiting for a frame end (`fire_ready`).
static READY_WAITING: Mutex<Vec<(ReadyFn, usize)>> = Mutex::new(Vec::new());
static READY_ANY: AtomicBool = AtomicBool::new(false);
/// The add-ons are loaded (`win`'s start sets it after `set_loaded`).
static LOAD_DONE: AtomicBool = AtomicBool::new(false);

pub fn set_load_done() {
    LOAD_DONE.store(true, Ordering::Release);
}

unsafe extern "C" fn api_on_ready(cb: Option<ReadyFn>, user: *mut c_void) -> i32 {
    guard(0, || {
        let Some(cb) = cb else { return 0 };
        let Ok(mut w) = READY_WAITING.lock() else { return 0 };
        if w.len() >= 64 {
            return 0;
        }
        w.push((cb, user as usize));
        READY_ANY.store(true, Ordering::Release);
        1
    })
}

/// Once a frame (KH1's frame end, `win/debugmenu.rs`; one atomic load): the waiting ready
/// callbacks called, once the add-ons are loaded. How many were called.
pub fn fire_ready() -> usize {
    if !READY_ANY.load(Ordering::Acquire) || !LOAD_DONE.load(Ordering::Acquire) {
        return 0;
    }
    let list: Vec<(ReadyFn, usize)> = match READY_WAITING.lock() {
        Ok(mut w) => {
            READY_ANY.store(false, Ordering::Release);
            std::mem::take(&mut *w)
        }
        Err(_) => return 0,
    };
    for (cb, user) in &list {
        // (The add-on's own code, on the game's thread, outside any of KH1's moves.)
        unsafe { cb(&API_V1.0, *user as *mut c_void) };
    }
    list.len()
}

/// Lines from the API's `log` (at most this many a session).
const LOG_MOST: u32 = 2000;
static LOGGED: AtomicU32 = AtomicU32::new(0);

unsafe extern "C" fn api_log(addon: *const c_char, msg: *const c_char) {
    guard((), || {
        let Some(m) = arg(msg) else { return };
        if LOGGED.fetch_add(1, Ordering::Relaxed) >= LOG_MOST {
            return;
        }
        let who = arg(addon).unwrap_or("?");
        let line: String = format!("[add-on {who}] {m}").chars().take(400).collect();
        if let Some(f) = LOG_SINK.get() {
            f(&line);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../addons-example/kh3d-example/addon.toml");

    fn reader(files: &'static [(&'static str, &'static [u8])]) -> impl Fn(&str, &str) -> Option<Vec<u8>> {
        move |_, rel| files.iter().find(|(n, _)| *n == rel).map(|(_, b)| b.to_vec())
    }

    fn mini(key: &str, ids: &[u8]) -> String {
        let mut s = format!("[addon]\nformat = 1\nkey = \"{key}\"\nname = \"{key}\"\nrow = \"R\"\n");
        for id in ids {
            s += &format!("[[ability]]\nkey = \"abl{id}\"\nid = {id}\nname = \"A {id}\"\nap = 2\nlearned = \"start\"\n");
        }
        s
    }

    #[test]
    fn the_example_parses() {
        let (a, warn) = parse(EXAMPLE, "kh3d-example").expect("the example is valid");
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(a.key, "kh3d.example");
        assert_eq!(a.title, "3D");
        assert!(a.abilities.len() >= 3);
        let f = a.abilities.iter().find_map(|x| x.finisher.as_ref()).expect("a finisher");
        assert!(!f.hits.is_empty());
        assert!(a.abilities.iter().any(|x| matches!(x.learned, Learn::Boss(_))));
        assert!(a.abilities.iter().any(|x| matches!(x.learned, Learn::Level(_))));
        // (No motion file in the repo: loaded, its finishers dropped, each with a note.)
        let (l, lines) = load(&[("kh3d-example".into(), EXAMPLE.into())], &reader(&[]));
        assert_eq!(l.addons.len(), 1);
        assert!(l.addons[0].abilities.iter().all(|x| x.finisher.is_none()));
        assert!(lines.iter().any(|s| s.contains("isn't there")), "{lines:?}");
    }

    #[test]
    fn bad_files_are_refused_not_fatal() {
        assert!(parse("not = [toml", "x").is_err());
        assert!(parse("[addon]\nkey = \"abc\"\nrow = \"R\"\n", "x").unwrap_err().contains("format"));
        assert!(parse("[addon]\nformat = 2\nkey = \"abc\"\nrow = \"R\"\n", "x").unwrap_err().contains("format 2"));
        assert!(parse("[addon]\nformat = 1\nkey = \"Bad Key\"\nrow = \"R\"\n", "x").is_err());
        assert!(parse("[addon]\nformat = 1\nkey = \"abc\"\n", "x").unwrap_err().contains("row"));
        // A bad ability is left out, the rest kept.
        let t = "[addon]\nformat = 1\nkey = \"abc\"\nrow = \"R\"\n\
                 [[ability]]\nkey = \"okay\"\nid = 1\nname = \"Ok\"\nap = 1\nlearned = { level = 5 }\n\
                 [[ability]]\nkey = \"noid\"\nname = \"No\"\nap = 1\nlearned = \"start\"\n\
                 [[ability]]\nkey = \"boss\"\nid = 2\nname = \"B\"\nap = 1\nlearned = { boss = \"Nobody\" }\n\
                 [[ability]]\nkey = \"dup\"\nid = 1\nname = \"D\"\nap = 1\nlearned = \"start\"\n\
                 [[ability]]\nkey = \"fin\"\nid = 3\nname = \"F\"\nap = 1\nlearned = \"start\"\n[ability.finisher]\nmotion = \"../x.anb\"\nhits = [[1, 2]]\n";
        let (a, warn) = parse(t, "x").unwrap();
        assert_eq!(a.abilities.iter().map(|x| x.key.as_str()).collect::<Vec<_>>(), ["okay", "fin"]);
        assert!(a.abilities[1].finisher.is_none(), "a finisher outside the folder is dropped");
        assert_eq!(warn.len(), 4, "{warn:?}");
        // A load with one bad file and one good one: the good one loads.
        let (l, lines) = load(&[("bad".into(), "[addon".into()), ("good".into(), mini("good", &[1]))], &reader(&[]));
        assert_eq!(l.addons.len(), 1);
        assert!(lines[0].starts_with("refused"));
    }

    #[test]
    fn learned_rules() {
        let jafar = boss("jafar").unwrap();
        assert_eq!(jafar.world, 4);
        assert!(boss("Genie  Jafar").is_some() && boss("Chernabog").is_some() && boss("Sabor").is_some());
        let mut progress = [0u8; 12];
        assert!(Learn::Start.due(1, &progress));
        assert!(!Learn::Level(10).due(9, &progress) && Learn::Level(10).due(10, &progress));
        assert!(!Learn::Boss(jafar).due(99, &progress));
        progress[jafar.world] = jafar.progress;
        assert!(Learn::Boss(jafar).due(1, &progress));
        let (a, _) = parse(&(mini("abc", &[1]) + "[[ability]]\nkey = \"later\"\nid = 2\nname = \"L\"\nap = 1\nlearned = { level = 20 }\n"), "x").unwrap();
        let mut list = [0u8; LIST_LEN];
        assert_eq!(a.due(&list, 5, &progress), vec![1]);
        list[0] = 1 | 0x80;
        assert_eq!(a.due(&list, 5, &progress), Vec::<u8>::new());
        assert_eq!(a.due(&list, 20, &progress), vec![2]);
    }

    #[test]
    fn row_and_id_limits() {
        let files: Vec<(String, String)> = (0..5).map(|i| (format!("f{i}"), mini(&format!("addon{i}"), &[1, 2]))).collect();
        let (l, lines) = load(&files, &reader(&[]));
        assert_eq!(l.addons.len(), MAX_ROWS);
        assert_eq!(lines.iter().filter(|s| s.contains("rows already")).count(), 5 - MAX_ROWS);
        // 35 runtime ids for all: a second add-on that doesn't fit whole is refused.
        let big: Vec<u8> = (1..=30).collect();
        let (l, lines) = load(&[("a".into(), mini("big", &big)), ("b".into(), mini("next", &[1, 2, 3, 4, 5, 6])), ("c".into(), mini("small", &[9]))], &reader(&[]));
        assert_eq!(l.addons.iter().map(|a| a.key.as_str()).collect::<Vec<_>>(), ["big", "small"]);
        assert!(lines.iter().any(|s| s.contains("ability ids left")));
        assert_eq!(l.addons[1].abilities[0].global, FIRST_GLOBAL + 30);
        // Runtime ids come after every one of ours.
        assert!(growth::MENU.iter().all(|m| m.id < FIRST_GLOBAL));
        assert_eq!(GLOBAL_IDS, 35);
        assert!(SAVE_OFF + AREA_LEN <= growth::KH2_LIST_OFF);
        // The same key twice: the second refused.
        let (l, _) = load(&[("a".into(), mini("same", &[1])), ("b".into(), mini("same", &[1]))], &reader(&[]));
        assert_eq!(l.addons.len(), 1);
    }

    /// Adding, removing or reordering add-ons changes runtime ids, never a saved byte's meaning.
    #[test]
    fn ids_stay_stable() {
        let a = mini("first", &[1, 2, 5]);
        let b = mini("second", &[1, 3]);
        let c = mini("third", &[7]);
        let mut area = vec![0u8; AREA_LEN];
        // Run 1: first, second. Sora has first's 5 (equipped) and 1 (off), second's 3.
        let (l1, _) = load(&[("a".into(), a.clone()), ("b".into(), b.clone())], &reader(&[]));
        let sa = slot_for(&mut area, l1.addons[0].hash).unwrap();
        let sb = slot_for(&mut area, l1.addons[1].hash).unwrap();
        // (What KH1's page wrote, in runtime ids, back to the save.)
        let page_a = [l1.addons[0].by_local(5).unwrap().global, l1.addons[0].by_local(1).unwrap().global | 0x80];
        let back = l1.addons[0].to_local(&page_a, &[]);
        set_slot_list(&mut area, sa, &back);
        let page_b = [l1.addons[1].by_local(3).unwrap().global];
        set_slot_list(&mut area, sb, &l1.addons[1].to_local(&page_b, &[]));
        let saved = area.clone();
        // Run 2: third first, then second; first removed.
        let (l2, _) = load(&[("c".into(), c), ("b".into(), b.clone())], &reader(&[]));
        assert_ne!(l2.addons[1].abilities[1].global, l1.addons[1].abilities[1].global, "runtime ids moved");
        let sb2 = slot_for(&mut area, l2.addons[1].hash).unwrap();
        assert_eq!(sb2, sb);
        let (g, hidden) = l2.addons[1].to_global(slot_list(&area, sb2));
        assert!(hidden.is_empty());
        assert_eq!(l2.addons[1].by_global(g[0]).unwrap().key, "abl3");
        let st = slot_for(&mut area, l2.addons[0].hash).unwrap();
        assert!(st != sa && st != sb, "a new add-on takes a free slot, not a removed one's");
        assert_eq!(slot_list(&area, sa), slot_list(&saved, sa), "the removed add-on's bytes are left alone");
        // Run 3: first back (an ability of it gone from its file: kept hidden in the save).
        let a_less = mini("first", &[1, 2]);
        let (l3, _) = load(&[("a".into(), a_less), ("b".into(), b)], &reader(&[]));
        let s = find_slot(&area, l3.addons[0].hash).unwrap();
        assert_eq!(s, sa);
        let (g, hidden) = l3.addons[0].to_global(slot_list(&area, s));
        assert_eq!(hidden, vec![5]);
        assert_eq!(l3.addons[0].by_global(g[0]).unwrap().id, 1);
        assert_eq!(g[0] & 0x80, 0x80, "still off");
        let again = l3.addons[0].to_local(&g, &hidden);
        assert_eq!(&again[..2], &[1 | 0x80, 5]);
    }

    #[test]
    fn the_area_is_ours_or_left_alone() {
        let mut a = vec![0u8; AREA_LEN];
        assert_eq!(area_state(&a), AreaState::Empty);
        assert!(find_slot(&a, 7).is_none());
        let s = slot_for(&mut a, 7).unwrap();
        assert_eq!(area_state(&a), AreaState::Ours);
        assert_eq!(find_slot(&a, 7), Some(s));
        for h in 100..100 + SLOTS as u64 - 1 {
            assert!(slot_for(&mut a, h).is_some());
        }
        assert!(slot_for(&mut a, 999).is_none(), "every slot taken");
        let mut foreign = vec![0u8; AREA_LEN];
        foreign[200] = 3;
        assert_eq!(area_state(&foreign), AreaState::Foreign);
        assert!(slot_for(&mut foreign, 7).is_none());
        assert_eq!(foreign.iter().filter(|b| **b != 0).count(), 1, "not written");
    }

    #[test]
    fn equipped_and_ap() {
        let (l, _) = load(&[("a".into(), mini("abc", &[1, 2]))], &reader(&[]));
        let a = &l.addons[0];
        let list = [1, 2 | 0x80, 1, 0];
        assert_eq!(a.equipped(&list, 1), 2);
        assert_eq!(a.equipped(&list, 2), 0);
        assert_eq!(a.ap_on(&list), 4);
    }

    #[test]
    fn motion_files_are_checked() {
        assert!(motion_frames(b"not a bar").is_err());
        let (l, lines) = load(&[("kh3d-example".into(), EXAMPLE.into())], &reader(&[("motions/blizzard_edge.anb", b"BAR\x01junk")]));
        assert!(l.addons[0].abilities.iter().all(|x| x.finisher.is_none()));
        assert!(lines.iter().any(|s| s.contains("blizzard_edge.anb:")), "{lines:?}");
    }

    #[test]
    fn api_versions() {
        let p = kh1kh2_addon_api(1);
        assert!(!p.is_null());
        let api = unsafe { &*p };
        assert_eq!(api.version, 1);
        assert_eq!(api.size as usize, std::mem::size_of::<ApiV1>());
        assert!(kh1kh2_addon_api(0).is_null());
        assert!(kh1kh2_addon_api(2).is_null());
        // Bad arguments and nothing loaded: -1 / 0, never a crash.
        unsafe {
            assert_eq!((api.is_equipped)(std::ptr::null(), std::ptr::null()), -1);
            assert_eq!((api.addon_loaded)(std::ptr::null()), -1);
            assert_eq!((api.addon_loaded)(b"nobody\0".as_ptr() as *const c_char), 0);
            assert_eq!((api.ability_id)(b"nobody\0".as_ptr() as *const c_char, b"x\0".as_ptr() as *const c_char), 0);
            assert_eq!((api.on_ready)(None, std::ptr::null_mut()), 0);
            (api.log)(std::ptr::null(), std::ptr::null());
        }
    }

    static FIRED: AtomicU32 = AtomicU32::new(0);
    unsafe extern "C" fn ready(api: *const ApiV1, user: *mut c_void) {
        assert_eq!((*api).version, 1);
        FIRED.fetch_add(user as u32, Ordering::Relaxed);
    }

    #[test]
    fn ready_waits_for_the_load_and_fires_once() {
        let api = unsafe { &*kh1kh2_addon_api(1) };
        unsafe { assert_eq!((api.on_ready)(Some(ready), 5 as *mut c_void), 1) };
        assert_eq!(fire_ready(), 0, "not before the add-ons are loaded");
        set_load_done();
        assert_eq!(fire_ready(), 1);
        assert_eq!(FIRED.load(Ordering::Relaxed), 5);
        assert_eq!(fire_ready(), 0, "once");
    }

    #[test]
    fn the_header_matches_the_table() {
        let h = include_str!("../include/kh1kh2_addon.h");
        for f in ["is_equipped", "is_learned", "addon_loaded", "ability_id", "on_ready", "log", "kh1kh2_addon_api"] {
            assert!(h.contains(f), "{f} not in the header");
        }
        // Six function pointers after two u32s (x64).
        assert_eq!(std::mem::size_of::<ApiV1>(), 8 + 6 * 8);
    }
}
