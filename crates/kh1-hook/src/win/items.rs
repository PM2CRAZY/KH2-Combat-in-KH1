//! KH2's Drive Recovery and High Drive Recovery as real KH1 items (`crate::driveitems` has the
//! numbers and their sources; `docs/kh1-items.md` how KH1's item, menu and shop code works).
//!
//! - **The records.** Two of KH1's unused item records (ids `0x91`, `0x92`, kind 12 like Tent)
//!   are written in place in KH1's item table (`*0x142D23738`, in the loaded `btltbl.bin`) after
//!   each load (`menu`'s table-load hook calls `ensure`): kind, prices, amount, sort key and the
//!   help text's handle; their names go into KH1's name table (`0x142D23998`, the records' own
//!   name words). The inventory is KH1's own (`save + 0x499 + id`, up to 99).
//! - **The icon** is the consumables' (kind 0's): the icon draw's kind read (`0x1402D3ADD`) answers
//!   0 for ours.
//! - **Usable** (`usable`, KH1's per-member test `0x1402CB1F0`): for Sora while the Drive gauge is
//!   shown and not full (KH2's test), so the Items page greys "Use" otherwise.
//! - **Used** (`use_item`, the Items page's use `0x14030CD20`): on Sora, what KH1 does for a
//!   Potion from the menu, with the gauge for HP: one taken from the inventory, the menu's stats
//!   refreshed, the recovery sound, the result window with our line. On anyone else, KH1's own
//!   path (its use switch does nothing for these ids: the "can't use" buzz and window).
//! - **Sold** (`shop_open`, KH1's shop `0x1402FEC60`): in the Traverse Town Item Shop (world 3,
//!   room 9, script `tw10`), once a Drive Form is open, the list KH1 opens gets ours after its own. Buying,
//!   selling (KH1's sell price, a fifth) and the counts are KH1's.

use super::addr::Rva;
use super::{hook, log};
use crate::driveitems::{self as di, Item, RECORD};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::OnceLock;

/// KH1's item table pointer and its name handles (one u32 per name word).
const ITEM_TABLE_PTR_RVA: Rva = Rva(0x2D23738);
const NAME_HANDLES_RVA: Rva = Rva(0x2D23998);
const NAME_WORDS: usize = 0x200;
/// KH1's text pointer -> handle.
const TO_HANDLE_RVA: Rva = Rva(0x38B080);
/// KH1's per-member usable test `(item, member) -> bits` (0: no).
const USABLE_RVA: Rva = Rva(0x2CB1F0);
/// `mov [rsp + 0x20], rsi; push rdi; sub rsp, 0x130`.
const USABLE_PROLOGUE: [u8; 13] = [0x48, 0x89, 0x74, 0x24, 0x20, 0x57, 0x48, 0x81, 0xEC, 0x30, 0x01, 0x00, 0x00];
/// The Items page's use `(page, slot member or -1, slot or item, target slot) -> used`.
const USE_RVA: Rva = Rva(0x30CD20);
/// `mov [rsp + 0x10], rbx; mov [rsp + 0x18], rbp; mov [rsp + 0x20], rsi`.
const USE_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x6C, 0x24, 0x18, 0x48, 0x89, 0x74, 0x24, 0x20];
/// What KH1's use calls on success: the menu's stats refresh, a menu sound, a window from its
/// descriptor (the result window's, `0x140549A00`), the window's parent.
const MENU_STATS_RVA: Rva = Rva(0x2E5A80);
const SOUND_RVA: Rva = Rva(0x177F60);
const WINDOW_NEW_RVA: Rva = Rva(0x2E3660);
const WINDOW_PARENT_RVA: Rva = Rva(0x2E3AF0);
const RESULT_WINDOW_RVA: Rva = Rva(0x549A00);
/// The menu text's line count `(text) -> extra lines`.
const LINES_RVA: Rva = Rva(0x2CE890);
/// The recovery sound (KH1's use plays 0xE for HP and MP).
const RECOVERY_SOUND: u32 = 0xE;
/// The menu message id -> text (`0x1402CE7C0`), and the call of it in the window text draw
/// (`0x1402EAEA0`), which the result window uses for its line.
const MESSAGE_RVA: Rva = Rva(0x2CE7C0);
const MESSAGE_CALL_RVA: Rva = Rva(0x2EAECB);
/// KH1's "HP restored" message (logged once, for the docs).
const HP_MESSAGE: u32 = 0x9A;
/// Our line's message id (past KH1's; only the call above answers it).
const OUR_MESSAGE: u32 = 0x7F01;
/// The item kind read of the item icon draw (`0x1402D3AB0`), and the read.
const KIND_CALL_RVA: Rva = Rva(0x2D3ADD);
const KIND_RVA: Rva = Rva(0x2E6680);
/// KH1's shop `(kind, list, a, b)`; kind 0 is the item shop, the list a count and ids (u32).
const SHOP_RVA: Rva = Rva(0x2FEC60);
/// `mov [rsp + 8], rbx; mov [rsp + 0x10], rbp; mov [rsp + 0x18], rsi`.
const SHOP_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18];
/// KH1's world and room (`takeover`'s).
const WORLD_RVA: Rva = Rva(0x2340E5C);
const ROOM_RVA: Rva = Rva(0x2340EC4);
/// The party slots in the save (member ids).
const PARTY_OFF: usize = 0x48E;

static ON: AtomicBool = AtomicBool::new(false);
static ORIG_USABLE: AtomicUsize = AtomicUsize::new(0);
static ORIG_USE: AtomicUsize = AtomicUsize::new(0);
static ORIG_SHOP: AtomicUsize = AtomicUsize::new(0);
/// Our text handles: (name, help) per item; and our result line.
static TEXT: OnceLock<Vec<(u32, u32)>> = OnceLock::new();
static LINE: OnceLock<usize> = OnceLock::new();
/// Our shop list (a count and 52 ids, as KH1's).
static SHOP_LIST: std::sync::Mutex<[u32; 1 + di::SHOP_LIST_MAX]> = std::sync::Mutex::new([0; 1 + di::SHOP_LIST_MAX]);
static LOGGED: AtomicU32 = AtomicU32::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}
fn base() -> usize {
    unsafe { super::sys::GetModuleHandleA(std::ptr::null()) as usize }
}

/// KH1's text codes as text, for the log (`menu::encode`'s letters; others as `{xx}`).
unsafe fn decode(p: usize) -> String {
    let mut s = String::new();
    if p < 0x10000 || !super::crash::readable(p) {
        return s;
    }
    for i in 0..96 {
        let b = rd::<u8>(p + i);
        match b {
            0 => break,
            0x01 => s.push(' '),
            0x21..=0x2A => s.push((b'0' + b - 0x21) as char),
            0x2B..=0x44 => s.push((b'A' + b - 0x2B) as char),
            0x45..=0x5E => s.push((b'a' + b - 0x45) as char),
            _ => s.push_str(&format!("{{{b:02x}}}")),
        }
    }
    s
}

fn leak(text: &str) -> usize {
    Box::leak(super::menu::encode(text).into_boxed_slice()).as_ptr() as usize
}

/// Our records and names in KH1's tables (again whenever they aren't ours: a reload).
pub unsafe fn ensure(base: usize) {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    let table = rd::<u64>(base + ITEM_TABLE_PTR_RVA) as usize;
    if table < 0x10000 || !super::crash::readable(table) || !super::crash::readable(table + 255 * RECORD - 1) {
        return;
    }
    let to_handle: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(base + TO_HANDLE_RVA);
    let text = TEXT.get_or_init(|| di::ITEMS.iter().map(|i| (to_handle(leak(i.name)), to_handle(leak(i.help)))).collect());
    let names = base + NAME_HANDLES_RVA;
    let mut wrote = false;
    for (k, item) in di::ITEMS.iter().enumerate() {
        let at = table + (item.id as usize - 1) * RECORD;
        let old: [u8; RECORD] = rd(at);
        let rec = item.record(old, text[k].1);
        if old != rec {
            wr(at, rec);
            wrote = true;
        }
        let word = u16::from_le_bytes([old[0], old[1]]) as usize;
        if word < NAME_WORDS && rd::<u32>(names + word * 4) != text[k].0 {
            wr(names + word * 4, text[k].0);
            wrote = true;
        }
    }
    if wrote && LOGGED.fetch_add(1, Ordering::Relaxed) < 6 {
        let line = |id: usize| {
            let r = table + (id - 1) * RECORD;
            let word = rd::<u16>(r) as usize;
            let name = if word < NAME_WORDS { decode(super::pose::ptr(rd::<u32>(names + word * 4))) } else { "?".into() };
            format!(
                "{id:#x} \"{name}\" kind {} price {} sell {} amount {} help \"{}\"",
                rd::<u8>(r + 2),
                rd::<u16>(r + 8),
                rd::<u16>(r + 10),
                rd::<i16>(r + 6),
                decode(super::pose::ptr(rd::<u32>(r + 12)))
            )
        };
        log(&format!(
            "Drive Recovery items: written into KH1's item table at {table:#x}: {} | {} (KH1's own: {} | {})",
            line(di::DRIVE_RECOVERY as usize),
            line(di::HIGH_DRIVE_RECOVERY as usize),
            line(3),
            line(0x8E)
        ));
    }
}

/// Sora is the member, and our item can be used on him now.
fn can_use(member: i32) -> bool {
    member == 0 && super::settings::get().drive && super::gauge::recovery_usable()
}

type UsableFn = unsafe extern "system" fn(u32, i32) -> u32;

/// KH1's per-member usable test: ours on Sora while the gauge takes it (bit 0, as KH1's own
/// answer for "usable"); KH1's for everything else.
unsafe extern "system" fn usable(item: u32, member: i32) -> u32 {
    if di::get(item).is_some() {
        ensure(base());
        return can_use(member) as u32;
    }
    let f: UsableFn = std::mem::transmute(ORIG_USABLE.load(Ordering::Acquire));
    f(item, member)
}

type UseFn = unsafe extern "system" fn(usize, i32, i32, i32) -> u32;

/// The Items page's use. Ours on Sora: KH1's success path for a recovery item, the gauge for HP.
unsafe extern "system" fn use_item(page: usize, slot_member: i32, slot_or_item: i32, target: i32) -> u32 {
    let f: UseFn = std::mem::transmute(ORIG_USE.load(Ordering::Acquire));
    let base = base();
    let Some(save) = super::menu::save_block(base) else { return f(page, slot_member, slot_or_item, target) };
    let party = |slot: i32| if (0..4).contains(&slot) { rd::<u8>(save + PARTY_OFF + slot as usize) as i32 } else { -1 };
    // (The item: from a member's item slot, or by id.)
    let id = if slot_member != -1 {
        let m = party(slot_member);
        if !(0..10).contains(&m) || !(0..16).contains(&slot_or_item) {
            return f(page, slot_member, slot_or_item, target);
        }
        rd::<u8>(save + m as usize * 0x74 + 0x26 + slot_or_item as usize) as u32
    } else {
        slot_or_item as u32
    };
    let Some(item) = di::get(id) else { return f(page, slot_member, slot_or_item, target) };
    let member = party(target);
    let count = rd::<u8>(save + crate::story::INVENTORY + item.id as usize);
    if !can_use(member) || (slot_member == -1 && count == 0) {
        log(&format!(
            "{} not used: {} (Drive gauge {}, {} owned)",
            item.name,
            if member != 0 { "not on Sora" } else { "the gauge doesn't take it" },
            super::gauge::describe_bars(),
            count
        ));
        return f(page, slot_member, slot_or_item, target);
    }
    used(base, save, page, item, slot_member, slot_or_item, target);
    1
}

/// What KH1's use does when its effect lands (read from `0x14030CD20`): the item taken, the menu's
/// stats refreshed, the sound, the result window with its line and the member.
unsafe fn used(base: usize, save: usize, page: usize, item: &Item, slot_member: i32, slot: i32, target: i32) {
    let (before, after) = super::gauge::recover(item.points());
    if slot_member != -1 {
        let m = rd::<u8>(save + PARTY_OFF + slot_member as usize) as usize;
        wr::<u8>(save + m * 0x74 + 0x26 + slot as usize, 0);
    } else {
        let at = save + crate::story::INVENTORY + item.id as usize;
        wr::<u8>(at, rd::<u8>(at).saturating_sub(1));
    }
    let left = rd::<u8>(save + crate::story::INVENTORY + item.id as usize);
    log(&format!("{} used on Sora (+{} points): Drive gauge {before} -> {after}; {left} left", item.name, item.points()));
    let stats: unsafe extern "system" fn() = std::mem::transmute(base + MENU_STATS_RVA);
    stats();
    let sound: unsafe extern "system" fn(u32) = std::mem::transmute(base + SOUND_RVA);
    sound(RECOVERY_SOUND);
    let new: unsafe extern "system" fn(usize, u32) -> usize = std::mem::transmute(base + WINDOW_NEW_RVA);
    let win = new(base + RESULT_WINDOW_RVA, 6);
    if win < 0x10000 {
        return;
    }
    let parent: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + WINDOW_PARENT_RVA);
    parent(win, page);
    let line = *LINE.get_or_init(|| leak("Sora's Drive Gauge was restored."));
    let lines: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + LINES_RVA);
    let grow = lines(line).wrapping_mul(0x170);
    wr::<i32>(win + 0xE0, rd::<i32>(win + 0xE0).wrapping_sub(grow));
    wr::<u16>(win + 0xE6, rd::<u16>(win + 0xE6).wrapping_add(grow as u16));
    wr::<u16>(win + 0xEC, rd::<u16>(win + 0xEC).wrapping_add(grow as u16));
    wr::<u32>(win + 0x48, OUR_MESSAGE);
    wr::<i32>(win + 0x4C, target);
    static ONCE: AtomicBool = AtomicBool::new(false);
    if !ONCE.swap(true, Ordering::Relaxed) {
        let msg: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base + MESSAGE_RVA);
        log(&format!("Drive Recovery items: KH1's own line for a Potion reads \"{}\"", decode(msg(HP_MESSAGE))));
    }
}

/// The window text draw's message read: our line for our id.
unsafe extern "system" fn message(id: u32) -> usize {
    if id == OUR_MESSAGE {
        if let Some(&p) = LINE.get() {
            return p;
        }
    }
    let f: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base() + MESSAGE_RVA);
    f(id)
}

/// The item icon draw's kind read: the consumables' icon for ours.
unsafe extern "system" fn icon_kind(item: u32) -> u32 {
    if di::get(item).is_some() {
        ensure(base());
        return di::KIND_CONSUMABLE as u32;
    }
    let f: unsafe extern "system" fn(u32) -> u32 = std::mem::transmute(base() + KIND_RVA);
    f(item)
}

type ShopFn = unsafe extern "system" fn(u32, usize, usize, usize);

/// KH1's shop: the Traverse Town Item Shop's list gets ours (once a Drive Form is open).
unsafe extern "system" fn shop_open(kind: u32, list: usize, a: usize, b: usize) {
    let f: ShopFn = std::mem::transmute(ORIG_SHOP.load(Ordering::Acquire));
    let base = base();
    let (world, room) = (rd::<u32>(base + WORLD_RVA), rd::<u32>(base + ROOM_RVA));
    if kind != 0 || list < 0x10000 || !super::crash::readable(list + 4 * (1 + di::SHOP_LIST_MAX) - 1) {
        return f(kind, list, a, b);
    }
    let n = rd::<u32>(list) as usize;
    let kh1: Vec<u32> = (0..n.min(di::SHOP_LIST_MAX)).map(|i| rd::<u32>(list + 4 + i * 4)).collect();
    let here = (world, room) == (di::SHOP_WORLD, di::SHOP_ROOM);
    let open = here && super::settings::get().drive && super::gauge::form_open();
    log(&format!(
        "item shop opened: world {world} room {room}, KH1's list {kh1:x?}{}",
        if !here {
            String::new()
        } else if open {
            " + Drive Recovery, High Drive Recovery".into()
        } else {
            " (no Drive Recovery items: no Drive Form open yet)".into()
        }
    ));
    if !open {
        return f(kind, list, a, b);
    }
    ensure(base);
    let ids = di::shop_list(&kh1);
    let Ok(mut buf) = SHOP_LIST.lock() else { return f(kind, list, a, b) };
    *buf = [0; 1 + di::SHOP_LIST_MAX];
    buf[0] = ids.len() as u32;
    buf[1..1 + ids.len()].copy_from_slice(&ids);
    let p = buf.as_ptr() as usize;
    drop(buf);
    f(kind, p, a, b)
}

pub unsafe fn install() {
    let base = base();
    if !super::settings::get().drive {
        return;
    }
    ON.store(true, Ordering::Relaxed);
    if hook::install(base + USABLE_RVA, &USABLE_PROLOGUE, usable as *const () as usize, &ORIG_USABLE, "item usable test (Drive Recovery)").is_none() {
        ON.store(false, Ordering::Relaxed);
        log("Drive Recovery items: KH1's usable test wasn't as expected: no Drive Recovery items");
        return;
    }
    if hook::install(base + USE_RVA, &USE_PROLOGUE, use_item as *const () as usize, &ORIG_USE, "Items page use (Drive Recovery)").is_none() {
        log("Drive Recovery items: KH1's item use wasn't as expected: they can't be used");
    }
    if !hook::redirect_call(base + MESSAGE_CALL_RVA, base + MESSAGE_RVA, message as *const () as usize, "window text (Drive Recovery line)") {
        log("Drive Recovery items: the window text read wasn't as expected: KH1's line shows");
    }
    if !hook::redirect_call(base + KIND_CALL_RVA, base + KIND_RVA, icon_kind as *const () as usize, "item icon kind (Drive Recovery)") {
        log("Drive Recovery items: the icon's kind read wasn't as expected: they show Tent's icon");
    }
    if hook::install(base + SHOP_RVA, &SHOP_PROLOGUE, shop_open as *const () as usize, &ORIG_SHOP, "shop open (Drive Recovery)").is_none() {
        log("Drive Recovery items: KH1's shop wasn't as expected: not sold");
    }
    ensure(base);
    log(&format!(
        "Drive Recovery items: KH1 items {:#x} and {:#x} (+{} / +{} points), sold in the Traverse Town Item Shop for {} / {} munny once a Drive Form is open",
        di::DRIVE_RECOVERY,
        di::HIGH_DRIVE_RECOVERY,
        di::ITEMS[0].points(),
        di::ITEMS[1].points(),
        di::ITEMS[0].buy(),
        di::ITEMS[1].buy()
    ));
}
