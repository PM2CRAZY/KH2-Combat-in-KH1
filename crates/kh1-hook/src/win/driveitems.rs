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
//! - **Can it be used** `0x1402CD3A0 (item, member) -> bits`: KH1's per-item test (Potion: HP below
//!   the maximum, ...), which the Items page asks for each member to enable the item and pick its
//!   target (`0x1402E8B20`, `0x1402E8B90`). Ours: Sora (member 0) with the Drive gauge below its
//!   maximum (KH2's `0x1403C3DA0`), else 0 (the item is greyed, as KH2 greys it).
//! - **Using it** `0x1402CD780 (item, target) -> bits`: KH1's per-item effect, run by the Items
//!   page's use `0x14030F160` (through `0x1402CD590`, the target `0x10000 | member`) and by the
//!   battle item use (`0x1402C2880` → `0x1402C2B50` → `0x1402CD580`, the target actor's id). Ours:
//!   KH2's value × 100 points to the gauge (`drivegauge::add`, KH2's `0x1403C0AD0`), returning bit 0
//!   (KH1 then takes one from the inventory or the slot, plays its use sound 0xE and shows message
//!   0x99 + 0, `0x14030F160`). Assumption: Sora's battle actor id is `0x10000 | 0` as the menu's
//!   target is (`0x1402CD5E0` takes that form for a party member).
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

use super::log;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::OnceLock;

use crate::driveitems::{self as items, IDS, RECORD};

const ITEMS_PTR_RVA: usize = 0x2D22D38;
const WORDS_RVA: usize = 0x2D22F98;
const WORDS: usize = 0x200;
const TO_HANDLE_RVA: usize = 0x38AD90;
const CAN_USE_RVA: usize = 0x2CD3A0;
const CAN_USE_BYTES: [u8; 13] = [0x48, 0x89, 0x74, 0x24, 0x20, 0x57, 0x48, 0x81, 0xEC, 0x30, 0x01, 0x00, 0x00];
const APPLY_RVA: usize = 0x2CD780;
const APPLY_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];

const SHOP_RVA: usize = 0x3010A0;
const SHOP_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];

const RANDOM_RVA: usize = 0x31BA30;
const RANDOM_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];
const ITEM_DROP_RVA: usize = 0x2BFF00;
const ITEM_DROP_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];
const LUCKY_RVA: usize = 0x2D60FA8;
const COUNT_FACTOR_RVA: usize = 0x2D60FA4;
const PRIZES_OFF_RVA: usize = 0x2D60FAC;

const CHEST_TABLE_RVA: usize = 0x528D60;

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
    super::settings::get().drive && super::addr::Build::current() == super::addr::Build::Steam
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
    if !TAKEN.swap(true, Ordering::Relaxed) {
        chests(base, sources());
    }
    if KIND_SET.swap(1 + battle as u8, Ordering::Relaxed) != 1 + battle as u8 {
        log!(Forms, "Drive Recovery: KH1's items {:#x} / {:#x} are KH2's Drive Recovery / High Drive Recovery (+3 / +9 bars, {})", IDS[0], IDS[1], if battle { "menu and battle" } else { "menu only" });
    }
}

/// Whether Sora can use one now (KH2's test, on the gauge that pays for the forms).
unsafe fn gauge_usable() -> bool {
    super::form::gauge_cost() && super::drivegauge::get().is_some_and(|g| items::usable(g.bars, g.max))
}

/// KH1's use test, ours for Drive Recovery.
unsafe extern "system" fn can_use(item: u32, member: i32) -> u8 {
    if items::which(item).is_some() && TAKEN.load(Ordering::Relaxed) {
        let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
        if KIND_SET.load(Ordering::Relaxed) != 1 + battle() as u8 {
            records(base);
        }
        return (member == 0 && gauge_usable()) as u8;
    }
    let f: unsafe extern "system" fn(u32, i32) -> u8 = std::mem::transmute(ORIG_CAN_USE.load(Ordering::Relaxed));
    f(item, member)
}

/// KH1's item effect, ours for Drive Recovery.
unsafe extern "system" fn apply(item: u32, target: u64) -> u64 {
    if let Some(k) = items::which(item).filter(|_| TAKEN.load(Ordering::Relaxed)) {
        let t = target as u32;
        let sora = t & 0xFF_0000 == 0x1_0000 && t & 0xFFFF == 0;
        if sora && gauge_usable() {
            super::drivegauge::add(items::points(k), items::NAMES[k]);
            return 1;
        }
        log!(Forms, "Drive Recovery: {} not used (target {t:#x}, the gauge full or not the form cost)", items::NAMES[k]);
        return 0;
    }
    let f: unsafe extern "system" fn(u32, u64) -> u64 = std::mem::transmute(ORIG_APPLY.load(Ordering::Relaxed));
    f(item, target)
}
