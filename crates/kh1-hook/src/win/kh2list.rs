//! The KH2 list in KH1's Abilities menu (the tester, what-is-still-missing after m18, answer 1:
//! "give kh2's its own ability list right next to sora, should be titled "KH2""). Steam only.
//! Notes: `docs/abilities.md` "The KH2 list".
//!
//! - **Where it lives.** 48 bytes in KH1's config block (save sub-block 5, its pointer at Steam
//!   `0x142E1F4D8`), at `growth::KH2_LIST_OFF`, in KH1's list format (an id, bit 7 = not
//!   equipped, 0 ends it). The block is saved and loaded with the save slot and zeroed at a new
//!   game; KH1 reads and writes only its first `0x78` bytes (Checked (decomp, Steam): every
//!   access through `0x142E1F4D8`).
//! - **KH1's own Abilities menu shows it.** The page (Steam, Checked (decomp + exe)): init
//!   `0x140311850` (the page table's first entry, `0x14052ADB0`), main window `0x140310EF0`, the
//!   member picker `0x140310B60`, the list window `0x140310A90` (`0x140311C20` for a member,
//!   `0x1403120F0` for the shared list), the header `0x1403108E0`, the help line `0x140310AB0`,
//!   the sort window `0x140310400`, the equip window `0x140310650`. The picker's rows are the
//!   party slots (`save + 0x48E + row`, count `0x142E941D0`, a 4th slot `+0x491` for a guest)
//!   then the shared list; a member row's list is `save + 4 + member * 0x74 + 0x40`, its count in
//!   `0x142E9CBC8[row]`, the chosen member in `0x142E9CB80`, its record in `0x142E9CB88`.
//!   Each of those windows' update pointers (their definitions' first qword, `.data`) is wrapped:
//!   for the length of KH1's call, the party seen by the page has a row "KH2" after Sora's, a
//!   member id of ours (`MEMBER`, 10: the save's ten records are 0..9), whose record arithmetic
//!   lands on `save + 0x4CC`; the KH2 list is put there for the call and taken back after, with
//!   the bytes that were there (KH1's item counts `save + 0x499 + item`, items 0x33..0x62, which
//!   the page doesn't read) put back. Nothing else runs in between (the game's thread), so outside
//!   the page's own calls the save is exactly KH1's. KH1's own code then counts, draws, sorts and
//!   equips the KH2 list, with its AP check, its sounds and its windows.
//! - **AP.** KH1's AP left (`0x1402E8660 (member)`) and used (`0x1402E8710`) for Sora (and our
//!   member) add the KH2 list's equipped abilities' AP (KH1's ability table `+0`, our entries
//!   hold KH2's AP); max AP (`0x1402E8560`) and the name (`0x1402E86F0`) take our member as Sora's
//!   AP and "KH2". So the KH2 abilities are equipped with Sora's AP, and every AP KH1 shows or
//!   checks (Abilities, Status, Equipment) counts both lists.
//! - KH1's party-stats refresh (`0x1402E7EC0`, which recounts the party) runs with the real party
//!   and items (we step out of the page's view for it), then the page's count is set again.
//!   The picker's cursor clamp (`0x1403119D0`, from the pause menu) clamps to the page's count.

use super::addr::{Build, Rva};
use super::log;
use crate::growth::{self, Mode};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

/// Our member id for the KH2 row (the save's party records are 0..9).
pub const MEMBER: u8 = 10;
/// Where KH1's record arithmetic puts member 10's list: `save + 4 + 10 * 0x74 + 0x40`.
const SWAP_OFF: usize = 4 + MEMBER as usize * 0x74 + 0x40;
const LEN: usize = growth::KH2_LIST_LEN;

/// KH1's config block pointer (Steam `0x142E1F4D8`).
const CONFIG_PTR_RVA: Rva = Rva(0x2E1FE70);
/// KH1's ability table pointer (Steam `0x142D36D48`): 12-byte entries, `+0` the AP.
const TABLE_PTR_RVA: Rva = Rva(0x2D37748);

// Steam RVAs (plain, used only on the Steam build; `base + rva`). Absolute addresses in the
// comments are with the image at 0x140000000.
/// The page's party count (`0x1402E7EC0` sets it: slots `+0x48E..+0x490` until 0xFF, +1 for a
/// guest in `+0x491`).
const COUNT: usize = 0x2E941D0;
/// The page's per-row list counts are 4 ints at Steam `0x142E9CBC8`; what follows them is another
/// page's window pointer (set by its own init): a 5th row's count goes there for the length of
/// our calls only.
const ROW_COUNT_5: usize = 0x2E9CBD8;
/// The picker's cursor row.
const PICK_ROW: usize = 0x2E9CBA8;
/// Members' max AP (stats copy, 0x100 apart).
const MAX_AP: usize = 0x2E942C4;
/// The member names' text ids (u16 each) and KH1's lookup for them.
const NAME_IDS: usize = 0x5452B8;
const NAME_TEXT: usize = 0x2D1510;

const AP_LEFT: usize = 0x2E8660;
const AP_LEFT_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18];
const AP_USED: usize = 0x2E8710;
const AP_USED_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20];
/// Replaced outright (RIP-relative; the trampoline is never called).
const MAX_AP_FN: usize = 0x2E8560;
const MAX_AP_PROLOGUE: [u8; 10] = [0x48, 0x63, 0xC1, 0x48, 0x8D, 0x0D, 0x5A, 0xBD, 0xBA, 0x02];
const NAME_FN: usize = 0x2E86F0;
const NAME_PROLOGUE: [u8; 10] = [0x48, 0x63, 0xC1, 0x48, 0x8D, 0x0D, 0xBE, 0xCB, 0x25, 0x00];
const REFRESH: usize = 0x2E7EC0;
const REFRESH_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];
const CLAMP: usize = 0x3119D0;
const CLAMP_PROLOGUE: [u8; 6] = [0x8B, 0x0D, 0xD2, 0xB1, 0xB8, 0x02];

/// The page's windows (RVAs): (definition's update pointer, KH1's function, name).
const WINDOWS: [(usize, usize, &str); 8] = [
    (0x052ADB0, 0x0311850, "page init"),
    (0x05490F0, 0x0310EF0, "page"),
    (0x05490C0, 0x0310B60, "member picker"),
    (0x0549090, 0x0310A90, "list"),
    (0x0548FA0, 0x0310AB0, "help line"),
    (0x0548F40, 0x03108E0, "header"),
    (0x0549030, 0x0310400, "sort"),
    (0x0549060, 0x0310650, "equip"),
];

static ORIG: [AtomicUsize; 8] = [const { AtomicUsize::new(0) }; 8];
static ORIG_AP_LEFT: AtomicUsize = AtomicUsize::new(0);
static ORIG_AP_USED: AtomicUsize = AtomicUsize::new(0);
static ORIG_MAX_AP: AtomicUsize = AtomicUsize::new(0);
static ORIG_NAME: AtomicUsize = AtomicUsize::new(0);
static ORIG_REFRESH: AtomicUsize = AtomicUsize::new(0);
static ORIG_CLAMP: AtomicUsize = AtomicUsize::new(0);
/// Every hook and window wrapper is in (the KH2 list can be used).
static READY: AtomicBool = AtomicBool::new(false);
/// "KH2" in KH1's text codes.
static KH2_TEXT: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// The image base (set at install).
static BASE: AtomicUsize = AtomicUsize::new(0);

fn base() -> usize {
    BASE.load(Ordering::Relaxed)
}

/// The KH2 list can be used: the Steam build, every hook in.
pub fn available() -> bool {
    READY.load(Ordering::Relaxed)
}

/// The KH2 list's 48 bytes in the save, if the config block is there.
pub unsafe fn list_ptr(base: usize) -> Option<usize> {
    if Build::current() != Build::Steam {
        return None;
    }
    let config = rd::<u64>(base + CONFIG_PTR_RVA) as usize;
    if config < 0x10000 || !super::crash::readable(config) || !super::crash::readable(config + 0x3FF) {
        return None;
    }
    Some(config + growth::KH2_LIST_OFF)
}

/// The KH2 list is the one in use: `abilities = kh1`, the save moved over (`LAYOUT_KH2_LIST`),
/// every hook in.
pub unsafe fn active(base: usize) -> bool {
    if !available() || super::settings::get().abilities != Mode::Kh1 {
        return false;
    }
    let Some(save) = super::menu::save_block(base) else { return false };
    rd::<u16>(save + growth::LAYOUT_OFF) == growth::LAYOUT_KH2_LIST && list_ptr(base).is_some()
}

/// The KH2 list's bytes now (where it is: in the page's view during one of its calls).
pub unsafe fn read(base: usize) -> Option<Vec<u8>> {
    let at = current(base)?;
    Some((0..LEN).map(|i| rd::<u8>(at + i)).collect())
}

/// Writes the KH2 list (48 bytes, padded with 0).
pub unsafe fn write(base: usize, list: &[u8]) {
    let Some(at) = current(base) else { return };
    for i in 0..LEN {
        wr(at + i, list.get(i).copied().unwrap_or(0));
    }
}

/// Where the KH2 list's live bytes are: the page's view while a page call has it, else its own.
unsafe fn current(base: usize) -> Option<usize> {
    if let Ok(v) = VIEW.try_lock() {
        if v.is_some() {
            return super::menu::save_block(base).map(|s| s + SWAP_OFF);
        }
    }
    list_ptr(base)
}

/// AP of the KH2 list's equipped abilities, by KH1's ability table.
unsafe fn ap_on(base: usize) -> i32 {
    let Some(list) = read(base) else { return 0 };
    let table = rd::<u64>(base + TABLE_PTR_RVA) as usize;
    if table < 0x10000 {
        return 0;
    }
    let mut ap = 0i32;
    for b in list.iter().take_while(|b| **b != 0) {
        if b & 0x80 == 0 {
            let e = table + ((*b & 0x7F) as usize - 1) * 12;
            if super::crash::readable(e) {
                ap += rd::<u8>(e) as i32;
            }
        }
    }
    ap
}

// ---- the page's view ----

/// What a page call changed, to put back after it.
struct View {
    party: [u8; 5],
    count: i32,
    items: [u8; LEN],
    /// The 5th row's count was written over `ROW_COUNT_5` (its 8 bytes before).
    fifth: Option<u64>,
}

static VIEW: std::sync::Mutex<Option<View>> = std::sync::Mutex::new(None);
static DEPTH: AtomicU32 = AtomicU32::new(0);
/// The 5th row's count between calls.
static FIFTH: AtomicU32 = AtomicU32::new(0);

/// Puts the page's view in: the KH2 row after Sora's, its list at `save + SWAP_OFF`.
unsafe fn enter(base: usize) {
    let Some(save) = super::menu::save_block(base) else { return };
    let Some(list) = list_ptr(base) else { return };
    let Ok(mut v) = VIEW.lock() else { return };
    if v.is_some() {
        return;
    }
    let n = rd::<i32>(base + COUNT);
    if !(1..=4).contains(&n) {
        return;
    }
    let mut party = [0u8; 5];
    for (i, p) in party.iter_mut().enumerate() {
        *p = rd::<u8>(save + 0x48E + i);
    }
    let real = &party[..n as usize];
    let at = real.iter().position(|m| *m == 0).map_or(0, |i| i + 1);
    let mut virt: Vec<u8> = real.to_vec();
    virt.insert(at, MEMBER);
    for (i, m) in virt.iter().enumerate() {
        wr(save + 0x48E + i, *m);
    }
    let mut items = [0u8; LEN];
    for (i, b) in items.iter_mut().enumerate() {
        *b = rd::<u8>(save + SWAP_OFF + i);
        wr(save + SWAP_OFF + i, rd::<u8>(list + i));
    }
    wr(base + COUNT, n + 1);
    let fifth = if n + 1 >= 5 {
        let old = rd::<u64>(base + ROW_COUNT_5);
        wr(base + ROW_COUNT_5, FIFTH.load(Ordering::Relaxed));
        Some(old)
    } else {
        None
    };
    *v = Some(View { party, count: n, items, fifth });
    static SAID: AtomicBool = AtomicBool::new(false);
    if !SAID.swap(true, Ordering::Relaxed) {
        log!(Menus, "KH2 list: Abilities menu shows a KH2 row at row {at} (party {:?} -> {:?})", real, virt);
    }
}

/// Takes the page's view back out: the KH2 list (as KH1 left it) to its own place, the save as it
/// was.
unsafe fn leave(base: usize) {
    let Some(save) = super::menu::save_block(base) else { return };
    let Ok(mut v) = VIEW.lock() else { return };
    let Some(view) = v.take() else { return };
    if let Some(list) = list_ptr(base) {
        for i in 0..LEN {
            wr(list + i, rd::<u8>(save + SWAP_OFF + i));
        }
    }
    for (i, b) in view.items.iter().enumerate() {
        wr(save + SWAP_OFF + i, *b);
    }
    if let Some(old) = view.fifth {
        FIFTH.store(rd::<u32>(base + ROW_COUNT_5), Ordering::Relaxed);
        wr(base + ROW_COUNT_5, old);
    }
    for (i, b) in view.party.iter().enumerate() {
        wr(save + 0x48E + i, *b);
    }
    wr(base + COUNT, view.count);
}

/// Runs a page call of KH1's in the page's view.
unsafe fn in_view(f: impl FnOnce() -> usize) -> usize {
    let base = base();
    if !active(base) {
        return f();
    }
    let outer = DEPTH.fetch_add(1, Ordering::AcqRel) == 0;
    if outer {
        enter(base);
    }
    let r = f();
    if outer {
        leave(base);
    }
    DEPTH.fetch_sub(1, Ordering::AcqRel);
    r
}

macro_rules! window {
    ($name:ident, $i:expr) => {
        unsafe extern "system" fn $name(win: usize) -> usize {
            let f: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(ORIG[$i].load(Ordering::Acquire));
            in_view(|| f(win))
        }
    };
}
window!(w0, 0);
window!(w1, 1);
window!(w2, 2);
window!(w3, 3);
window!(w4, 4);
window!(w5, 5);
window!(w6, 6);
window!(w7, 7);
const WRAPPERS: [unsafe extern "system" fn(usize) -> usize; 8] = [w0, w1, w2, w3, w4, w5, w6, w7];

// ---- KH1's functions ----

/// AP left (`0x1402E8660 (member)`): Sora's and our member's take both lists.
unsafe extern "system" fn ap_left(member: i32) -> i32 {
    let f: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(ORIG_AP_LEFT.load(Ordering::Acquire));
    let base = base();
    if (member != 0 && member != MEMBER as i32) || !active(base) {
        return f(member);
    }
    let used = ap_used(0);
    (max_ap(0) - used).max(0)
}

/// AP used (`0x1402E8710 (member)`): Sora's own 48, plus the KH2 list for Sora and our member.
unsafe extern "system" fn ap_used(member: i32) -> i32 {
    let f: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(ORIG_AP_USED.load(Ordering::Acquire));
    let base = base();
    if (member != 0 && member != MEMBER as i32) || !active(base) {
        return f(member);
    }
    f(0) + ap_on(base)
}

/// Max AP (`0x1402E8560 (member)`, re-made: the stats copy's `+0xE4`); our member's is Sora's.
unsafe extern "system" fn max_ap(member: i32) -> i32 {
    let m = if member == MEMBER as i32 { 0 } else { member };
    rd::<i32>(base() + MAX_AP + (m as isize as usize).wrapping_mul(0x100))
}

/// A member's name (`0x1402E86F0 (member)`, re-made: KH1's name text by its id table); ours is
/// "KH2".
unsafe extern "system" fn name(member: i32) -> usize {
    if member == MEMBER as i32 {
        return *KH2_TEXT.get_or_init(|| Box::leak(super::menu::encode("KH2").into_boxed_slice()).as_ptr() as usize);
    }
    let id = rd::<u16>(base() + NAME_IDS + (member as isize as usize).wrapping_mul(2));
    let f: unsafe extern "system" fn(i32) -> usize = std::mem::transmute(base() + NAME_TEXT);
    f(id as i32)
}

/// KH1's party-stats refresh (`0x1402E7EC0`), with the real party and items under it during a
/// page call; the page's count after.
unsafe extern "system" fn refresh() {
    let f: unsafe extern "system" fn() = std::mem::transmute(ORIG_REFRESH.load(Ordering::Acquire));
    let base = base();
    let viewed = VIEW.try_lock().map(|v| v.is_some()).unwrap_or(false);
    if !viewed {
        f();
        return;
    }
    leave(base);
    f();
    enter(base);
}

/// The picker's cursor clamp (`0x1403119D0`, re-made): to the page's count (one more with the KH2
/// row).
unsafe extern "system" fn clamp() {
    let base = base();
    let mut cap = rd::<u32>(base + COUNT);
    if DEPTH.load(Ordering::Acquire) == 0 && active(base) {
        cap += 1;
    }
    if rd::<u32>(base + PICK_ROW) > cap {
        wr(base + PICK_ROW, cap);
    }
}

pub unsafe fn install() {
    if Build::current() != Build::Steam {
        log!(Menus, "KH2 list: not on this build (Steam only): KH2 abilities stay in Sora's own list");
        return;
    }
    let b = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(b, Ordering::Relaxed);
    let rebase = |rva: usize| b + rva;
    // Everything is checked before anything goes in.
    let mut ok = true;
    for (def, func, what) in WINDOWS {
        if rd::<u64>(rebase(def)) as usize != rebase(func) {
            log!(Menus, "KH2 list: the {what} window's definition {def:#x} doesn't hold {func:#x}");
            ok = false;
        }
    }
    let checks: [(usize, &[u8]); 6] = [
        (AP_LEFT, &AP_LEFT_PROLOGUE),
        (AP_USED, &AP_USED_PROLOGUE),
        (MAX_AP_FN, &MAX_AP_PROLOGUE),
        (NAME_FN, &NAME_PROLOGUE),
        (REFRESH, &REFRESH_PROLOGUE),
        (CLAMP, &CLAMP_PROLOGUE),
    ];
    for (at, bytes) in checks {
        if std::slice::from_raw_parts(rebase(at) as *const u8, bytes.len()) != bytes {
            log!(Menus, "KH2 list: unexpected bytes at {at:#x}");
            ok = false;
        }
    }
    if !ok {
        log!(Menus, "KH2 list: not installed: KH2 abilities stay in Sora's own list");
        return;
    }
    let hooks: [(usize, &[u8], usize, &AtomicUsize, &str); 6] = [
        (AP_LEFT, &AP_LEFT_PROLOGUE, ap_left as *const () as usize, &ORIG_AP_LEFT, "AP left (KH2 list)"),
        (AP_USED, &AP_USED_PROLOGUE, ap_used as *const () as usize, &ORIG_AP_USED, "AP used (KH2 list)"),
        (MAX_AP_FN, &MAX_AP_PROLOGUE, max_ap as *const () as usize, &ORIG_MAX_AP, "max AP (KH2 list)"),
        (NAME_FN, &NAME_PROLOGUE, name as *const () as usize, &ORIG_NAME, "member name (KH2 list)"),
        (REFRESH, &REFRESH_PROLOGUE, refresh as *const () as usize, &ORIG_REFRESH, "party stats refresh (KH2 list)"),
        (CLAMP, &CLAMP_PROLOGUE, clamp as *const () as usize, &ORIG_CLAMP, "Abilities cursor clamp (KH2 list)"),
    ];
    for (at, bytes, ours, orig, what) in hooks {
        if super::hook::install(rebase(at), bytes, ours, orig, what).is_none() {
            // (The earlier ones stay in: each passes through while the KH2 list isn't ready.)
            log!(Menus, "KH2 list: a hook failed: KH2 abilities stay in Sora's own list");
            return;
        }
    }
    for (i, (def, func, what)) in WINDOWS.iter().enumerate() {
        ORIG[i].store(rebase(*func), Ordering::Release);
        let new = (WRAPPERS[i] as usize as u64).to_le_bytes();
        let old = (rebase(*func) as u64).to_le_bytes();
        if !super::hook::patch(rebase(*def), &old, &new, what) {
            log!(Menus, "KH2 list: the {what} window couldn't be wrapped");
            return;
        }
    }
    READY.store(true, Ordering::Release);
    log!(Menus, "KH2 list: installed (KH1's Abilities menu, a KH2 row after Sora's; AP from both lists)");
}
