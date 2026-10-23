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
//! - **Sold** (`shop_open`, KH1's shop `0x1402FEC60`): in KH1's item shops that sell consumables
//!   (a list with a Potion: the Traverse Town Item Shop, world 3 room 9, script `tw10`, and
//!   Agrabah's, world 8 room 18, `al19`), once a Drive Form is open, the list KH1 opens gets ours
//!   after its own. Buying, selling (KH1's sell price, a fifth) and the counts are KH1's.
//! - **Dropped** (`enemy_drop`, from `prize`'s hook on KH1's enemy drop `0x1402A9AB0`): once a
//!   Drive Form is open, after KH1's own drop, one roll by the drop's tier (`driveitems::TIERS`,
//!   times KH1's Lucky Strike factor `0x142D619A8`, as its own item chances), and KH1's own field
//!   item drop (`0x1402BDD50 (item, position, mode)`, the one its (chance, item) pairs use) makes
//!   it: KH1's model, bounce, pickup (`0x1402BE720`: the inventory add and the "obtained" notice).
//! - **In chests** (`patch_chests`): KH1's chest table (`0x140529A60`, the executable's data) has
//!   the chests of `driveitems::CHEST_LIST` changed in memory to ours (each checked against KH1's
//!   value first). The opened flags are KH1's, by chest index (`save + 0x5CC`), so a save is the
//!   same chests either way. The chest open (`0x1402B3740`) is hooked only for the log.

use super::addr::Rva;
use super::{hook, log};
use crate::driveitems::{self as di, Item, RECORD};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
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
/// KH1's chest table (512 u16s, `driveitems::CHESTS`).
const CHEST_TABLE_RVA: Rva = Rva(0x529A60);
/// The chest open `(actor, chest object)`: same prologue as the shop's.
const CHEST_OPEN_RVA: Rva = Rva(0x2B3740);
/// KH1's field item drop `(item, position, mode)`, and its item chances' factor (Lucky Strike).
const ITEM_DROP_RVA: Rva = Rva(0x2BDD50);
const LUCK_RVA: Rva = Rva(0x2D619A8);
/// An object's record handle, and in the record the chest's index.
const RECORD_HANDLE: usize = 0x130;
const RECORD_CHEST: usize = 0x48;

static ON: AtomicBool = AtomicBool::new(false);
static ORIG_USABLE: AtomicUsize = AtomicUsize::new(0);
static ORIG_USE: AtomicUsize = AtomicUsize::new(0);
static ORIG_SHOP: AtomicUsize = AtomicUsize::new(0);
static ORIG_CHEST: AtomicUsize = AtomicUsize::new(0);
static SEED: AtomicU64 = AtomicU64::new(0x2545_F491_4F6C_DD1D);
static DROPS_LOGGED: AtomicU32 = AtomicU32::new(0);
/// The changed chests opened this session (logged once each).
static CHESTS_LOGGED: AtomicU32 = AtomicU32::new(0);
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

/// KH1's shop: an item shop's list that sells consumables gets ours (once a Drive Form is open).
unsafe extern "system" fn shop_open(kind: u32, list: usize, a: usize, b: usize) {
    let f: ShopFn = std::mem::transmute(ORIG_SHOP.load(Ordering::Acquire));
    let base = base();
    let (world, room) = (rd::<u32>(base + WORLD_RVA), rd::<u32>(base + ROOM_RVA));
    if kind != 0 || list < 0x10000 || !super::crash::readable(list + 4 * (1 + di::SHOP_LIST_MAX) - 1) {
        return f(kind, list, a, b);
    }
    let n = rd::<u32>(list) as usize;
    let kh1: Vec<u32> = (0..n.min(di::SHOP_LIST_MAX)).map(|i| rd::<u32>(list + 4 + i * 4)).collect();
    let here = di::sells_consumables(&kh1);
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

/// A small xorshift: a float in [0, 1).
fn roll() -> f32 {
    let mut x = SEED.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    SEED.store(x, Ordering::Relaxed);
    (x >> 40) as f32 / (1u64 << 24) as f32
}

/// An enemy's drop (`prize`'s hook on KH1's `0x1402A9AB0`, after KH1's own and the Drive orbs):
/// once a Drive Form is open, maybe one of ours, made by KH1's field item drop.
pub unsafe fn enemy_drop(base: usize, counts: [i32; 7], pos: usize, mode: u32) {
    if !ON.load(Ordering::Relaxed) || pos < 0x10000 || !super::settings::get().drive || !super::gauge::form_open() {
        return;
    }
    let Some(tier) = di::tier(&counts) else { return };
    let luck = rd::<f32>(base + LUCK_RVA);
    let Some(id) = di::drop_item(tier, roll(), roll(), luck) else { return };
    ensure(base);
    let spawn: unsafe extern "system" fn(u32, usize, u32) = std::mem::transmute(base + ITEM_DROP_RVA);
    spawn(id as u32, pos, mode);
    if DROPS_LOGGED.fetch_add(1, Ordering::Relaxed) < 60 {
        let p: [f32; 3] = rd(pos);
        let name = di::get(id as u32).map_or("?", |i| i.name);
        log(&format!(
            "{name} dropped by {} (KH1's counts {counts:?}, chances {:.1}% / {:.2}% times luck {luck:.2}) at ({:.0}, {:.0}, {:.0}), world {} room {}",
            tier.name,
            tier.drive * 100.0,
            tier.high * 100.0,
            p[0],
            p[1],
            p[2],
            rd::<u32>(base + WORLD_RVA),
            rd::<u32>(base + ROOM_RVA)
        ));
    }
}

/// KH1's chest table with our chests (each only where KH1's value is the expected one).
unsafe fn patch_chests(base: usize) {
    let table = base + CHEST_TABLE_RVA;
    let mut done = Vec::new();
    for c in di::CHEST_LIST {
        let at = table + c.index as usize * 2;
        let ours = di::chest_item(c.item);
        if rd::<u16>(at) == ours {
            done.push(c.index);
            continue;
        }
        if hook::patch(at, &c.vanilla.to_le_bytes(), &ours.to_le_bytes(), &format!("chest {} ({} {})", c.index, c.world, c.room)) {
            done.push(c.index);
        }
    }
    log(&format!(
        "Drive Recovery items: {} of KH1's {} changed chests hold ours ({done:?}); the others are KH1's",
        done.len(),
        di::CHEST_LIST.len()
    ));
}

type ChestFn = unsafe extern "system" fn(usize, usize) -> u64;

/// KH1's chest open: as ever; one of ours opened is logged (once each this session) with the count.
unsafe extern "system" fn chest_open(actor: usize, obj: usize) -> u64 {
    let f: ChestFn = std::mem::transmute(ORIG_CHEST.load(Ordering::Acquire));
    let base = base();
    let index = if obj > 0x10000 && super::crash::readable(obj + RECORD_HANDLE) {
        let rec = super::pose::ptr(rd::<u32>(obj + RECORD_HANDLE));
        if rec > 0x10000 && super::crash::readable(rec + RECORD_CHEST) {
            rd::<i16>(rec + RECORD_CHEST) as i32
        } else {
            -1
        }
    } else {
        -1
    };
    let Some((k, c)) = di::CHEST_LIST.iter().enumerate().find(|(_, c)| c.index as i32 == index) else { return f(actor, obj) };
    ensure(base);
    let save = super::menu::save_block(base);
    let count = || save.map_or(-1, |s| rd::<u8>(s + crate::story::INVENTORY + c.item as usize) as i32);
    let before = count();
    let r = f(actor, obj);
    if CHESTS_LOGGED.fetch_or(1 << k, Ordering::Relaxed) & (1 << k) == 0 {
        let value = rd::<u16>(base + CHEST_TABLE_RVA + c.index as usize * 2);
        log(&format!(
            "chest {} opened ({} {}, KH1's {}): {} {before} -> {} (chest value {value:#06x})",
            c.index,
            c.world,
            c.room,
            c.was,
            di::get(c.item as u32).map_or("?", |i| i.name),
            count()
        ));
    }
    r
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
    patch_chests(base);
    if hook::install(base + CHEST_OPEN_RVA, &SHOP_PROLOGUE, chest_open as *const () as usize, &ORIG_CHEST, "chest open (Drive Recovery log)").is_none() {
        log("Drive Recovery items: KH1's chest open wasn't as expected: chests aren't logged");
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
    SEED.store(0x2545_F491_4F6C_DD1D ^ now | 1, Ordering::Relaxed);
    let tiers: Vec<String> = di::TIERS.iter().map(|t| format!("{} {:.1}% / {:.2}%", t.name, t.drive * 100.0, t.high * 100.0)).collect();
    log(&format!(
        "Drive Recovery items: KH1 items {:#x} and {:#x} (+{} / +{} points), sold in KH1's item shops for {} / {} munny once a Drive Form is open; enemy drops then ({}; times Lucky Strike's factor)",
        di::DRIVE_RECOVERY,
        di::HIGH_DRIVE_RECOVERY,
        di::ITEMS[0].points(),
        di::ITEMS[1].points(),
        di::ITEMS[0].buy(),
        di::ITEMS[1].buy(),
        tiers.join(", ")
    ));
}
