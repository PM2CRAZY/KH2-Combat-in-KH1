//! KH2's Drive Recovery and High Drive Recovery as KH1 items `0x91` / `0x92` (the owner, the
//! main-branch port's round 5, row 4: answers 1 (a), 2 (b), 3 (a)). Notes:
//! `docs/kh1-drive-forms-native.md` §16. The rule and the records: `crate::driveitems`.
//!
//! Steam KH1, Checked (decomp) unless marked:
//! - **The records.** KH1's item table (`btltbl.bin`, loaded by `0x14028F9D0`; its pointer at
//!   `0x142D22D38`, item `id` at `+ (id − 1) × 0x14`) and the word handles the names come from
//!   (`0x142D22F98 + word × 4`, 0x200 of them; `0x14028F9B0`). After KH1's load (`menu`'s hook on
//!   it calls `records`), records `0x91` / `0x92`, unused in KH1 (Checked (files)), take KH2's
//!   values, their words our names and their help our text (handles made by KH1's `0x14038AD90`).
//!   The kind is 0 (the battle item slots take it) with Drive > Recovery in Battle on, else 12
//!   (menu only, KH2's rule).
//! - **The battle Items list** (t1082): KH1's battle lists hold command ids and name and run a row
//!   from its command record (`0x1402787B0` → the record's word; `0x1402912F0` → its type and
//!   argument; `docs/kh1-reflect-spell.md`, Checked (exe) there), not from the item record. Ours had
//!   none: the rows showed an empty record's name ("e") and ran nothing. Assumption (KH1's fill not
//!   read): an item's command is `item + 0x1E`, the number of its name's word; checked at run time
//!   on KH1's own items (type 3, their item's word, one argument offset) before records `0xAF` /
//!   `0xB0` (empty in KH1) get KH1's first item command's record with our word and argument.
//! - **Can it be used** `0x1402CD3A0 (item, member) -> bits`: KH1's per-item test (Potion: HP below
//!   the maximum, ...), which the Items page asks for each member to enable the item and pick its
//!   target (`0x1402E8B20`, `0x1402E8B90`). Ours: Sora (member 0) with the Drive gauge below its
//!   maximum (KH2's `0x1403C3DA0`), else 0 (the item is greyed, as KH2 greys it).
//! - **Using it** `0x1402CD780 (item, target) -> bits`: KH1's per-item effect, run by the Items
//!   page's use `0x14030F160` (through `0x1402CD590`, the target `0x10000 | member`) and by the
//!   battle item use (`0x1402C2880` → `0x1402C2B50` → `0x1402CD580`, the target actor's id). Ours:
//!   KH2's value × 100 points to the gauge (`drivegauge::add`, KH2's `0x1403C0AD0`), returning bit 0
//!   (KH1 then takes one from the inventory or the slot, plays its use sound 0xE and shows message
//!   0x99 + 0, `0x14030F160`). In battle the target is the actor's `+4`, which KH1's effect resolves
//!   with its actor lookup `0x140291D30` (`0x1402CD5E0`), as `is_sora` now does (Checked (decomp)).
//! - **The battle use's item id** (Checked (decomp)): KH1's menu run `0x140293480` writes the chosen
//!   command `0x142D53A40` as `type | (int)(char)arg << 8` (masked `0xFFFF00`): a signed byte, so
//!   our `0x91` / `0x92` came out as `0xFF91` / `0xFF92`. The command step (`0x1402A3DB0` /
//!   `0x1402A4150`, type 3) hands bits 8..23 to the item use `0x1402C2760`, where bit 10 means "not
//!   from a slot": nothing taken, nothing given back; the use `0x1402C2B50` clears bit 10 and runs
//!   the effect with `0xFB91`, which no case knows (nothing added). `command_taken` puts our id back
//!   right after KH1's run (`combat::run_entry`), so KH1 takes it from Sora's slot
//!   (`0x142D539E4`, the list row's slot) and runs our effect once. Its menu's command id is
//!   `item + 0x1E` (the fill `0x1402944C0`, Checked (decomp): t1082's Assumption confirmed).
//! - **The shops** (t950, round 5 answer 1 (a), main's design): KH1's shop open `0x1403010A0 (mode,
//!   list, ..)` (mode 0 the item shops, 2 the other lists; the list a count and item ids from KH1's
//!   table at `0x1404FEE00`, picked by the shop's event, `0x1401CA610`) is hooked: with Progression >
//!   Drive Recovery on and a Drive Form open, an item shop's list (a Potion and a Tent in it) is handed
//!   over as a copy of ours with the two items after KH1's. KH1's shop then shows, prices (`+8`) and
//!   sells them as its own; KH1's list is never written.
//! - **The drops** (t951, main's design): after KH1's enemy drop `0x1402ABC40 (drop, position, mode)`
//!   (`driveorbs`' hook calls `drop`), unless KH1 has prizes off, each item is rolled with KH1's own
//!   random `0x14031BA30` against `crate::driveitems::drop_chances` of the prizes KH1 gave × KH1's
//!   Lucky Strike factor `0x142D60FA8`, as KH1 rolls its own item drops, and made by KH1's item drop
//!   `0x1402BFF00 (item, position, mode)` (KH1's model for the record's `+3`), with Progression > Drive
//!   Recovery on and a Drive Form open.
//! - **The chests** (t952, main's design): KH1's chest table (`0x140528D60`, the exe's data, read when a
//!   chest is opened, `0x1402B58E0`) has the 17 chests of `crate::driveitems::CHESTS` set to ours (each
//!   only where KH1's value is the one expected), with Progression > Drive Recovery on, and put back when
//!   it is turned off. KH1 then gives them as its own items; the opened flags stay KH1's.
//! - Only with Drive on (the ini's `drive`), the Drive gauge as the form cost, and on Steam.

use super::addr::St;
use super::log;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::OnceLock;

use crate::driveitems::{self as items, IDS, RECORD};

const ITEMS_PTR_RVA: St = St(0x2D22D38);
const WORDS_RVA: St = St(0x2D22F98);
const WORDS: usize = 0x200;
const TO_HANDLE_RVA: St = St(0x38AD90);
const CAN_USE_RVA: St = St(0x2CD3A0);
const CAN_USE_BYTES: [u8; 13] = [0x48, 0x89, 0x74, 0x24, 0x20, 0x57, 0x48, 0x81, 0xEC, 0x30, 0x01, 0x00, 0x00];
const APPLY_RVA: St = St(0x2CD780);
const APPLY_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];

const SHOP_RVA: St = St(0x3010A0);
const SHOP_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];

const RANDOM_RVA: St = St(0x31BA30);
const RANDOM_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];
const ITEM_DROP_RVA: St = St(0x2BFF00);
const ITEM_DROP_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];
const LUCKY_RVA: St = St(0x2D60FA8);
const COUNT_FACTOR_RVA: St = St(0x2D60FA4);
const PRIZES_OFF_RVA: St = St(0x2D60FAC);

const CHEST_TABLE_RVA: St = St(0x528D60);
/// KH1's command record `(id) -> table + id × 16` (Steam `0x14028F960`, Epic `0x14028D7D0`,
/// `steam_map.in`; what `drive` / `reflect` write their records through).
const COMMAND_RECORD_RVA: St = St(0x28F960);

static ORIG_SHOP: AtomicUsize = AtomicUsize::new(0);
/// The list handed to KH1's shop (a count, then the ids), kept while it is open.
static mut SHOP_LIST: [i32; 1 + items::SHOP_MAX] = [0; 1 + items::SHOP_MAX];
/// Progression > Drive Recovery: 0 not read yet, 1 off, 2 on.
static SOURCES: AtomicU8 = AtomicU8::new(0);
static ORIG_CAN_USE: AtomicUsize = AtomicUsize::new(0);
static ORIG_APPLY: AtomicUsize = AtomicUsize::new(0);
/// The records are ours (both taken).
static TAKEN: AtomicBool = AtomicBool::new(false);
/// The kind the records were last set with: 0 not yet, 1 menu only, 2 battle too.
static KIND_SET: AtomicU8 = AtomicU8::new(0);
/// Drive > Recovery in Battle: 0 not read yet, 1 off, 2 on.
static BATTLE: AtomicU8 = AtomicU8::new(0);
/// Our text handles: the two names, then each item's help (menu only, battle too).
static TEXT: OnceLock<[u32; 6]> = OnceLock::new();

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn on() -> bool {
    super::settings::get().drive && super::addr::Build::current() != super::addr::Build::Unknown
}

/// Drive > Recovery in Battle (the ini's `drive_recovery_battle`, off by default: KH2's items are
/// menu only).
pub fn battle() -> bool {
    match BATTLE.load(Ordering::Relaxed) {
        0 => {
            let v = super::settings::get().drive_recovery_battle;
            BATTLE.store(1 + v as u8, Ordering::Relaxed);
            v
        }
        v => v == 2,
    }
}

/// Switches Drive > Recovery in Battle (the Config menu), kept in the ini; the records follow.
pub fn set_battle(v: bool) {
    BATTLE.store(1 + v as u8, Ordering::Relaxed);
    super::settings::save("drive_recovery_battle", if v { "on" } else { "off" });
    log!(Forms, "Config menu: Recovery in Battle {}", if v { "On" } else { "Off" });
    let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    unsafe { records(base) };
}

/// Progression > Drive Recovery (the ini's `drive_recovery_sources`, on by default): the items sold,
/// dropped and found (main's design, round 5 answer 1 (a)).
pub fn sources() -> bool {
    match SOURCES.load(Ordering::Relaxed) {
        0 => {
            let v = super::settings::get().drive_recovery_sources;
            SOURCES.store(1 + v as u8, Ordering::Relaxed);
            v
        }
        v => v == 2,
    }
}

/// Switches Progression > Drive Recovery (the Config menu), kept in the ini.
pub fn set_sources(v: bool) {
    SOURCES.store(1 + v as u8, Ordering::Relaxed);
    super::settings::save("drive_recovery_sources", if v { "on" } else { "off" });
    log!(Forms, "Config menu: Drive Recovery sources {}", if v { "On" } else { "Off" });
    if TAKEN.load(Ordering::Relaxed) {
        let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
        unsafe { chests(base, v) };
    }
}

/// KH1's chest table with ours (`on`) or KH1's own values again.
unsafe fn chests(base: usize, on: bool) {
    let table = base + CHEST_TABLE_RVA;
    let mut done = 0;
    for &(index, kh1, k, what) in items::CHESTS.iter() {
        let at = table + index as usize * 2;
        let ours = items::chest_value(k);
        let (from, to) = if on { (kh1, ours) } else { (ours, kh1) };
        if rd::<u16>(at) == to {
            done += 1;
        } else if rd::<u16>(at) == from && super::hook::patch(at, &from.to_le_bytes(), &to.to_le_bytes(), &format!("chest {index} ({what})")) {
            done += 1;
        }
    }
    log!(Forms, "Drive Recovery: {done} of {} chests {}", items::CHESTS.len(), if on { "hold ours" } else { "KH1's again" });
}

pub unsafe fn install(base: usize) {
    if !on() {
        return;
    }
    super::hook::install(base + CAN_USE_RVA, &CAN_USE_BYTES, can_use as *const () as usize, &ORIG_CAN_USE, "KH1's item use test (Drive Recovery)");
    super::hook::install(base + APPLY_RVA, &APPLY_BYTES, apply as *const () as usize, &ORIG_APPLY, "KH1's item effect (Drive Recovery)");
    super::hook::install(base + SHOP_RVA, &SHOP_BYTES, shop as *const () as usize, &ORIG_SHOP, "KH1's shop open (Drive Recovery)");
}

/// After KH1's enemy drop (`driveorbs`' hook): Drive Recovery items rolled (see the module notes).
pub unsafe fn drop(counts: usize, pos: usize, mode: i32) {
    if counts < 0x10000 || pos < 0x10000 || !TAKEN.load(Ordering::Relaxed) || !sources() || super::formown::owned() == 0 {
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    if rd::<i32>(base + PRIZES_OFF_RVA) != 0 || !super::hook::prologue_is(base + RANDOM_RVA, &RANDOM_BYTES) || !super::hook::prologue_is(base + ITEM_DROP_RVA, &ITEM_DROP_BYTES) {
        return;
    }
    let factor = rd::<f32>(base + COUNT_FACTOR_RVA);
    let mut given = [0i32; 7];
    for (i, g) in given.iter_mut().enumerate() {
        *g = (rd::<i32>(counts + i * 4) as f32 * factor) as i32;
    }
    let chances = items::drop_chances(&given);
    let lucky = rd::<f32>(base + LUCKY_RVA);
    let random: unsafe extern "system" fn() -> f32 = std::mem::transmute(base + RANDOM_RVA);
    let make: unsafe extern "system" fn(u32, usize, i32) = std::mem::transmute(base + ITEM_DROP_RVA);
    for (k, &id) in IDS.iter().enumerate() {
        let r = random();
        if r < lucky * chances[k] {
            make(id as u32, pos, mode);
            log!(Forms, "Drive Recovery: an enemy dropped {} (roll {r:.3} < {:.4}: {:.4} × Lucky Strike {lucky})", items::NAMES[k], lucky * chances[k], chances[k]);
        }
    }
}

/// KH1's shop open, an item shop's list with ours in it (see the module notes).
unsafe extern "system" fn shop(mode: i32, list: usize, a: usize, b: usize) {
    let mut given = list;
    if (mode == 0 || mode == 2) && list >= 0x10000 && TAKEN.load(Ordering::Relaxed) && sources() && super::formown::owned() != 0 {
        let n = rd::<i32>(list);
        if (0..=items::SHOP_MAX as i32).contains(&n) {
            let kh1: Vec<i32> = (0..n as usize).map(|i| rd::<i32>(list + 4 + i * 4)).collect();
            if let Some(v) = items::shop_with_ours(&kh1) {
                let ours = std::ptr::addr_of_mut!(SHOP_LIST) as *mut i32;
                *ours = v.len() as i32;
                for (i, &id) in v.iter().enumerate() {
                    *ours.add(1 + i) = id;
                }
                given = ours as usize;
                log!(Forms, "Drive Recovery: KH1's shop list at {list:#x} ({n} items) sells them too");
            }
        }
    }
    let f: unsafe extern "system" fn(i32, usize, usize, usize) = std::mem::transmute(ORIG_SHOP.load(Ordering::Relaxed));
    f(mode, given, a, b);
}

/// After KH1 loads its item table (and when the kind setting changes): records `0x91` / `0x92` made
/// KH2's items, if KH1 doesn't use them.
pub unsafe fn records(base: usize) {
    if !on() {
        return;
    }
    let table = rd::<u64>(base + ITEMS_PTR_RVA) as usize;
    if table < 0x10000 || !super::crash::readable(table + 0x100 * RECORD) {
        return;
    }
    let battle = battle();
    for (k, &id) in IDS.iter().enumerate() {
        let rec = table + (id as usize - 1) * RECORD;
        let bytes = std::slice::from_raw_parts(rec as *const u8, RECORD);
        if !items::unused(bytes) && !items::ours(bytes, k) {
            TAKEN.store(false, Ordering::Relaxed);
            log!(Forms, "Drive Recovery: KH1's item {id:#x} is in use ({bytes:02x?}); not added");
            return;
        }
    }
    let texts = TEXT.get_or_init(|| {
        let to_handle: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(base + TO_HANDLE_RVA);
        let make = |s: &str| to_handle(Box::leak(super::menu::encode(s).into_boxed_slice()).as_ptr() as usize);
        [make(items::NAMES[0]), make(items::NAMES[1]), make(&items::help(0, false)), make(&items::help(0, true)), make(&items::help(1, false)), make(&items::help(1, true))]
    });
    for (k, &id) in IDS.iter().enumerate() {
        let rec = table + (id as usize - 1) * RECORD;
        let bytes = std::slice::from_raw_parts_mut(rec as *mut u8, RECORD);
        items::set(bytes, k, battle);
        wr(rec + 0xC, texts[2 + k * 2 + battle as usize]);
        let word = rd::<u16>(rec) as usize;
        if word > 0 && word < WORDS {
            wr(base + WORDS_RVA + word * 4, texts[k]);
        }
    }
    commands(base, table);
    if !TAKEN.swap(true, Ordering::Relaxed) {
        chests(base, sources());
    }
    if KIND_SET.swap(1 + battle as u8, Ordering::Relaxed) != 1 + battle as u8 {
        log!(Forms, "Drive Recovery: KH1's items {:#x} / {:#x} are KH2's Drive Recovery / High Drive Recovery (+3 / +9 bars, {})", IDS[0], IDS[1], if battle { "menu and battle" } else { "menu only" });
    }
}

/// KH1's battle Items list names and runs a row from its command record (t1082): the commands of
/// ours (`items::command_of`, empty in KH1) get KH1's Potion's command with our word and argument,
/// when KH1's own item commands follow the rule (`items::item_command_arg`); else none, logged.
unsafe fn commands(base: usize, table: usize) {
    static LOGGED: AtomicU8 = AtomicU8::new(0);
    let record: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base + COMMAND_RECORD_RVA);
    let mut kh1 = Vec::new();
    for id in items::KH1_ITEMS {
        let item = table + (id as usize - 1) * RECORD;
        let r = record(items::command_of(id));
        if r < 0x10000 || rd::<u8>(item + 2) != items::KIND_ITEM || rd::<[u8; items::COMMAND]>(r) == [0; items::COMMAND] {
            continue;
        }
        kh1.push((id, rd::<u16>(item), rd::<[u8; items::COMMAND]>(r)));
    }
    let Some(offset) = items::item_command_arg(&kh1) else {
        if LOGGED.swap(1, Ordering::Relaxed) != 1 {
            log!(Forms, "Drive Recovery: KH1's item commands don't follow item + {:#x} (seen {:02x?}): no battle commands for ours", items::COMMAND_OF_ITEM, kh1);
        }
        return;
    };
    let template = kh1[0].2;
    for (k, &id) in IDS.iter().enumerate() {
        let (cmd, word) = (items::command_of(id), rd::<u16>(table + (id as usize - 1) * RECORD));
        let Ok(arg) = u8::try_from(id as i32 + offset) else { return };
        let r = record(cmd);
        if r < 0x10000 {
            return;
        }
        let want = items::command_record(&template, word, arg);
        let have = rd::<[u8; items::COMMAND]>(r);
        if have != want {
            if have != [0; items::COMMAND] && have[4..8] != want[4..8] {
                if LOGGED.swap(2, Ordering::Relaxed) != 2 {
                    log!(Forms, "Drive Recovery: command record {cmd:#x} is in use ({have:02x?}): {} has no battle command", items::NAMES[k]);
                }
                return;
            }
            wr(r, want);
        }
    }
    if LOGGED.swap(3, Ordering::Relaxed) != 3 {
        log!(Forms, "Drive Recovery: battle commands {:#x} / {:#x} (from KH1's item {:#x}'s command {:02x?}, argument item {offset:+})", items::command_of(IDS[0]), items::command_of(IDS[1]), kh1[0].0, template);
    }
}

/// Whether Sora can use one now (KH2's test, on the gauge that pays for the forms).
unsafe fn gauge_usable() -> bool {
    super::form::gauge_cost() && super::drivegauge::get().is_some_and(|g| items::usable(g.bars, g.max))
}

/// Sora's actor (KH1's first party slot, Steam `0x142D37280`, as `cmdmenu` / `chest` read it).
const SORA_RVA: St = St(0x2D37280);
/// KH1's chosen command word (`type | arg << 8`) and the record it came from (Steam `0x142D53A40`,
/// `0x142D539E8`, written by the menu run `0x140293480`; Checked (decomp)).
const CHOSEN_RVA: St = St(0x2D53A40);
const CHOSEN_RECORD_RVA: St = St(0x2D539E8);
/// KH1's battle item use's effect per item (`0x1402C2B50` plays `byte [0x14052A0A7 + item]`;
/// Checked (decomp)). Its data ends before `0x14052A130` (another table's floats), so KH1's items
/// reach `0x88`; ours read past it (logged, an Assumption what it plays).
const USE_EFFECT_RVA: St = St(0x52A0A7);
/// KH1's actor lookup `0x140291D30 (id)` (as `chest` calls it; Checked (decomp + exe) there): what
/// KH1's battle item effect (`0x1402CD5E0`) resolves the use's target id with.
const FIND_ACTOR_RVA: St = St(0x291D30);
const FIND_ACTOR_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];

/// KH1's party target list (menu 4, Steam `0x1405289E0`: `+0xC` cursor, `+0x10` count, `+0x14` the
/// rows' commands, `0x38 + member`) with each row's actor at `0x142D53A60 + row × 8`, the Items list
/// (menu 2, `0x140528970`; `+0x34` the row whose list is open below), and the menu chain's current
/// level (`0x142D53A50`, a pointer into the chain `0x142D539F0`). KH1's top fill `0x140294220` makes
/// the target list anew each frame before the menu's navigation (the party in order, Sora's row
/// `0x38` first), Checked (decomp).
const TARGETS_RVA: St = St(0x5289E0);
const TARGET_ACTORS_RVA: St = St(0x2D53A60);
const ITEMS_MENU_RVA: St = St(0x528970);
const LEVEL_RVA: St = St(0x2D53A50);
const CHAIN_RVA: St = St(0x2D539F0);
/// Sora's row in the party target list (`0x38 + member 0`, Checked (decomp)).
const SORA_ROW: u8 = 0x38;

/// After KH1's top fill (`cmdmenu`'s hook on it): while the party target list is open from the Items
/// list on one of ours, it holds only Sora's row (the owner: "when selecting the item, just have soras
/// name appear"), as KH1's own one-member list is. KH1 makes it anew each frame, so nothing is put
/// back. A few reads a frame while KH1's menu runs; a write only while ours is open.
pub unsafe fn sora_only_targets(base: usize, sora: usize) {
    if !TAKEN.load(Ordering::Relaxed) || !on() || !battle() {
        return;
    }
    let level = rd::<u64>(base + LEVEL_RVA) as usize;
    let chain = base + CHAIN_RVA;
    if level <= chain || level > chain + 7 * 8 || rd::<u64>(level) as usize != base + TARGETS_RVA || rd::<u64>(level - 8) as usize != base + ITEMS_MENU_RVA {
        return;
    }
    let items = base + ITEMS_MENU_RVA;
    let (n, row) = (rd::<i32>(items + 0x10), rd::<i32>(items + 0x34));
    if !(0..n.min(16)).contains(&row) || !IDS.iter().any(|&id| items::command_of(id) == rd::<u8>(items + 0x14 + row as usize) as u32) {
        return;
    }
    let targets = base + TARGETS_RVA;
    let count = rd::<i32>(targets + 0x10);
    if count <= 1 || rd::<u8>(targets + 0x14) != SORA_ROW || rd::<u64>(base + TARGET_ACTORS_RVA) as usize != sora {
        return;
    }
    wr(targets + 0x10, 1i32);
    if rd::<i32>(targets + 0xC) != 0 {
        wr(targets + 0xC, 0i32);
    }
    static LOGGED: AtomicBool = AtomicBool::new(false);
    if !LOGGED.swap(true, Ordering::Relaxed) {
        log!(Forms, "Drive Recovery: KH1's party target list shows only Sora for ours (was {count} rows)");
    }
}

/// After KH1's menu run took a command (`combat::run_entry`, KH1's `0x140293480`): our item's id put
/// back in KH1's chosen command (`items::battle_request`: KH1 read the argument signed). KH1's item
/// use then takes it from Sora's slot (`0x1402C2760`), gives it back if he's interrupted
/// (`0x140284E60`) and runs its effect (`0x1402C2B50` → `0x1402CD580` → `apply`) on its own. Once per
/// command taken; nothing per frame.
pub unsafe fn command_taken(base: usize) {
    if !TAKEN.load(Ordering::Relaxed) || !on() {
        return;
    }
    let word = rd::<u32>(base + CHOSEN_RVA);
    let Some(fixed) = items::battle_request(word) else { return };
    let id = (fixed >> 8 & 0xFF) as u8;
    let record = rd::<u64>(base + CHOSEN_RECORD_RVA) as usize;
    if record < 0x10000 || rd::<u8>(record + 6) != items::COMMAND_TYPE_ITEM || rd::<u8>(record + 7) != id {
        log!(Forms, "Drive Recovery: chosen command {word:#010x} not from our record ({record:#x}); left as KH1 wrote it");
        return;
    }
    wr(base + CHOSEN_RVA, fixed);
    let effect = rd::<u8>(base + USE_EFFECT_RVA + id as usize);
    log!(Forms, "Drive Recovery: battle use of item {id:#x} (KH1's command {word:#010x} -> {fixed:#010x}); KH1's use effect byte {effect:#x} (Assumption: past KH1's table)");
}

/// Whether a target of KH1's item use (or of its use test) is Sora (t1083): `0x10000 | 0` (Checked (decomp), `0x1402CD590`), or in battle his actor as a handle (KH1's
/// `0x14038AD90` form, `pose::ptr`) or its pointer's low half. Assumption: the battle use's "target
/// actor's id" (`0x1402CD580`) is one of these; before, only `0x10000 | 0` was taken, so a battle
/// use on Sora could be refused.
unsafe fn is_sora(t: u32) -> bool {
    if t == 0x1_0000 {
        return true;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let sora = rd::<u64>(base + SORA_RVA) as usize;
    if sora <= 0x10000 {
        return false;
    }
    // KH1's battle use passes the target actor's `+4` (`0x1402CD580`), which KH1's own effect
    // resolves with its actor lookup (`0x1402CD5E0` → `0x140291D30`; Checked (decomp)): the same here.
    if super::hook::prologue_is(base + FIND_ACTOR_RVA, &FIND_ACTOR_BYTES) {
        let find: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base + FIND_ACTOR_RVA);
        if find(t) == sora {
            return true;
        }
    }
    t as usize == sora & 0xFFFF_FFFF || (t & 0x8000_0000 != 0 && super::pose::ptr(t) == sora)
}

/// KH1's use test, ours for Drive Recovery.
unsafe extern "system" fn can_use(item: u32, member: i32) -> u8 {
    if items::which(item).is_some() && TAKEN.load(Ordering::Relaxed) {
        let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
        if KIND_SET.load(Ordering::Relaxed) != 1 + battle() as u8 {
            records(base);
        }
        return ((member == 0 || is_sora(member as u32)) && gauge_usable()) as u8;
    }
    let f: unsafe extern "system" fn(u32, i32) -> u8 = std::mem::transmute(ORIG_CAN_USE.load(Ordering::Relaxed));
    f(item, member)
}

/// KH1's item effect, ours for Drive Recovery.
unsafe extern "system" fn apply(item: u32, target: u64) -> u64 {
    if let Some(k) = items::which(item).filter(|_| TAKEN.load(Ordering::Relaxed)) {
        let t = target as u32;
        if is_sora(t) && gauge_usable() {
            super::drivegauge::add(items::points(k), items::NAMES[k]);
            log!(Forms, "Drive Recovery: {} used (target {t:#x})", items::NAMES[k]);
            return 1;
        }
        log!(Forms, "Drive Recovery: {} not used (target {t:#x}, the gauge full or not the form cost)", items::NAMES[k]);
        return 0;
    }
    let f: unsafe extern "system" fn(u32, u64) -> u64 = std::mem::transmute(ORIG_APPLY.load(Ordering::Relaxed));
    f(item, target)
}
