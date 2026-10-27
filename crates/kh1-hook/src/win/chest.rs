//! Chest Opening (KH1 | KH2), the Config menu's Combat > Tweaks row (the tester Mr Bugs, thread
//! kh2-chest: "quickly open and move from the chest opening animation like in kh2 ... open the chest,
//! move then the pop up for the item plays at the top of the game letting you know what you got").
//! `docs/kh1-chest.md`.
//!
//! What KH1 does (Steam, Checked (decomp + exe)):
//!
//! - The Open command (Triangle box, command type `0x37`, offered by the chest's touch callback
//!   `0x1402B5A30`) runs `0x1402B58E0 (sora, chest)`. It reads the chest's entry in KH1's chest table
//!   (`0x140528D60`, a u16 per chest id: bits 1..3 the kind, bits 4.. the item or amount) and gives
//!   the contents itself: kind 0 an item through KH1's stock add (`0x140284E60 (item, 0xC)`, 0xFF
//!   when the stock is full), kind 3 munny (amount x 10, `0x1402E7E10`), kind 7 a reward record (an
//!   item one becomes kind 0), kind 1 `0x14020D3F0`; then marks the chest opened (`0x140285800`).
//!   It stores the open's arguments at `0x142DB7B28` (chest handle, chest id, kind `+8`, item or
//!   amount `+0xC`, "stock full" `+0x10`), sets Sora's `+0x444` bit 1 (his control step is skipped:
//!   locked), asks for his open motion (`0x14029F980 (sora, 0x900A6, 0)`) and queues event 5 on the
//!   chest (`0x14028DD80`).
//! - Event 5 (`0x1401A1A10`) starts KH1's treasure script (`exchange/<lang>_treasure.ev`, loaded at
//!   `0x1423BBCB0` by `0x1401A42C0`) in its own script contexts (`0x1423BF0B0`, `0x360` each, the
//!   count at `0x1423AB2DC`; the script in use at `0x1423BBBE0`): entry 0 for every kind but 5 (an
//!   event chest, whose entry is its item word). The script opens the lid, shows the "obtained"
//!   message window and releases Sora (Assumption: the script itself was not readable in this
//!   session; the release is KH1's actor action kind 0xF, `0x14028E6E0`, which only clears that bit).
//!
//! KH2 (this row on KH2), for a plain chest only (kind 0 or 3 with the item given, the stock not full):
//!
//! - KH1's own handler runs whole: the item or munny is given, the chest marked, the treasure script
//!   started, exactly as KH1 does. Afterwards Sora's lock bit is cleared, so KH1's own control step
//!   runs him at once (the open motion KH1 asked for plays until he moves).
//! - The treasure script runs whole too, but its message windows are not opened: while the open is
//!   recent (`FAST_MS`), KH1's message functions 0 and 177 (open a window) and 1 and 605 (add a line /
//!   an item line) take their own skip-mode branch when called from a treasure context (pop their
//!   arguments, open nothing), as KH1 does for them in skip mode. The script's waits on windows (33,
//!   107) then pass, as in skip mode, and closing (2) a window that never opened does nothing
//!   (Checked (decomp)). The four entries of KH1's event function table (`0x1404FA870`) point at
//!   wrappers of ours.
//! - What was found is shown in KH1's own non-blocking notice box (the level-up box, `menu::queue_notice`,
//!   as the mod's "New Ability" boxes): "Obtained" and the item's name or the munny.
//!
//! Anything else (an event chest, a full stock, a reward record that is not an item, kind 1, any other
//! script using those contexts) is KH1's as before. KH1: nothing of this is installed until the row is
//! first set to KH2; then one flag check per Open and per message function call.

use super::addr::Build;
use super::{hook, log};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

/// KH2 (`chest_style = kh2`).
static KH2: AtomicBool = AtomicBool::new(false);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static ORIG_OPEN: AtomicUsize = AtomicUsize::new(0);
/// KH1's message functions as the table held them (called for anything not ours).
static ORIG_FN: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];
/// When the last fast open happened (ms since the first look, plus 1; 0 = none).
static FAST_AT: AtomicU64 = AtomicU64::new(0);
/// Message calls skipped for the current open (for the log).
static SKIPPED: AtomicU32 = AtomicU32::new(0);

/// KH1's Open command handler `(sora, chest)` (Steam; `mov [rsp+8], rbx; mov [rsp+0x10], rbp`,
/// Checked (exe)).
const OPEN_STEAM: usize = 0x2B58E0;
const OPEN_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];
/// The open's arguments as the handler leaves them (Steam, Checked (decomp)): kind, item or amount,
/// stock full.
const ARG_KIND_STEAM: usize = 0x2DB7B30;
const ARG_ITEM_STEAM: usize = 0x2DB7B34;
const ARG_FULL_STEAM: usize = 0x2DB7B38;
/// Sora's lock: actor `+0x444` bit 1 (the handler sets it; KH1's control step is skipped while it is).
const ACTOR_LOCK: usize = 0x444;
const LOCK_BIT: u32 = 2;

/// KH1's event function table (8-byte pointers, by number) and the four message functions wrapped
/// (Steam RVAs, Checked (exe)), with how many arguments each pops in its skip-mode branch
/// (Checked (decomp)).
const TABLE_STEAM: usize = 0x4FA870;
const MESSAGE_FNS: [(usize, usize, i32); 4] = [(0, 0x1B93F0, 1), (1, 0x1B8DD0, 2), (177, 0x1B9170, 1), (605, 0x1B8AE0, 2)];
/// A script context's stack pointer (`+0x194`).
const CTX_SP: usize = 0x194;

/// The treasure script's buffer, the script in use, its contexts and their count (Steam, Checked
/// (decomp)).
const TREASURE_STEAM: usize = 0x23BBCB0;
const SCRIPT_IN_USE_STEAM: usize = 0x23BBBE0;
const CONTEXTS_STEAM: usize = 0x23BF0B0;
const COUNT_STEAM: usize = 0x23AB2DC;
const CTX_SIZE: usize = 0x360;
const CTX_MAX: u32 = 64;

/// How long after an open its message windows are still skipped. Assumption: the treasure script
/// reaches its window well within this (KH1's own open motion and lid take about a second).
const FAST_MS: u64 = 8000;

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

fn ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

fn base() -> usize {
    unsafe { super::sys::GetModuleHandleA(std::ptr::null()) as usize }
}

/// The row's value: KH2's chest opening.
pub fn kh2() -> bool {
    KH2.load(Ordering::Relaxed)
}

pub unsafe fn install() {
    let on = super::settings::get().chest_kh2;
    KH2.store(on, Ordering::Relaxed);
    if on {
        hook_in();
    }
    log!(Menus, "Chest Opening: {} (chest_style)", if on { "KH2" } else { "KH1" });
}

/// The Config menu's Chest Opening row, kept in the ini.
pub fn set(v: bool) {
    KH2.store(v, Ordering::Relaxed);
    if v {
        unsafe { hook_in() };
    }
    FAST_AT.store(0, Ordering::Relaxed);
    super::settings::save("chest_style", if v { "kh2" } else { "kh1" });
    log!(Menus, "Config menu: Chest Opening {}", if v { "KH2" } else { "KH1" });
}

/// The Open handler hook and the four table entries (Steam only, once).
unsafe fn hook_in() {
    if INSTALLED.swap(true, Ordering::Relaxed) {
        return;
    }
    if Build::current() != Build::Steam {
        log!(Menus, "Chest Opening: KH2 is Steam only (the addresses are the Steam build's): chests stay KH1's");
        return;
    }
    let base = base();
    // The table entries first: without them the open would free Sora with KH1's window up.
    let mut wrapped = 0;
    let ours: [usize; 4] = [msg0 as *const () as usize, msg1 as *const () as usize, msg177 as *const () as usize, msg605 as *const () as usize];
    for (i, &(n, rva, _)) in MESSAGE_FNS.iter().enumerate() {
        let at = base + TABLE_STEAM + n * 8;
        let was = (base + rva) as u64;
        ORIG_FN[i].store(base + rva, Ordering::Release);
        if hook::patch(at, &was.to_le_bytes(), &(ours[i] as u64).to_le_bytes(), "event function (chest)") {
            wrapped += 1;
        }
    }
    if wrapped != MESSAGE_FNS.len() {
        log!(Menus, "Chest Opening: only {wrapped} of KH1's message functions wrapped: the Open stays KH1's");
        return;
    }
    if hook::install(base + OPEN_STEAM, &OPEN_PROLOGUE, open as *const () as usize, &ORIG_OPEN, "KH1's Open command (chest)").is_none() {
        log!(Menus, "Chest Opening: KH1's Open handler not hooked: chests stay KH1's");
    }
}

/// KH1's Open handler, then KH2's fast open for a plain chest.
unsafe extern "system" fn open(sora: usize, chest: usize) {
    let orig: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_OPEN.load(Ordering::Acquire));
    orig(sora, chest);
    if !kh2() || sora == 0 {
        FAST_AT.store(0, Ordering::Relaxed);
        return;
    }
    let base = base();
    let kind = rd::<u32>(base + ARG_KIND_STEAM);
    let what = rd::<u32>(base + ARG_ITEM_STEAM);
    let full = rd::<u32>(base + ARG_FULL_STEAM);
    if full != 0 || !(kind == 0 || kind == 3) {
        FAST_AT.store(0, Ordering::Relaxed);
        log!(Menus, "Chest Opening: KH1's open (kind {kind}, contents {what:#x}, stock full {full})");
        return;
    }
    let lock = sora + ACTOR_LOCK;
    std::ptr::write_unaligned(lock as *mut u32, rd::<u32>(lock) & !LOCK_BIT);
    FAST_AT.store(ms() + 1, Ordering::Relaxed);
    SKIPPED.store(0, Ordering::Relaxed);
    let line = if kind == 3 {
        format!("{what} Munny")
    } else {
        let name = if what < 0x100 { super::equip::item_name(base, what as u8) } else { None };
        match name {
            Some(n) => strip_codes(&n),
            None => format!("Item {what:#x}"),
        }
    };
    super::menu::queue_notice("Obtained", &line);
    log!(Menus, "Chest Opening: KH2 open (kind {kind}, contents {what:#x}): Sora free at once, notice \"Obtained / {line}\", the treasure script's windows skipped");
}

/// The item name without the codes `item_name` writes as `{xx}`.
fn strip_codes(s: &str) -> String {
    let mut out = String::new();
    let mut skip = false;
    for c in s.chars() {
        match c {
            '{' => skip = true,
            '}' => skip = false,
            _ if !skip => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// The call comes from the treasure script during a recent fast open.
unsafe fn ours_to_skip(ctx: usize) -> bool {
    let at = FAST_AT.load(Ordering::Relaxed);
    if at == 0 || !kh2() {
        return false;
    }
    if ms() + 1 - at > FAST_MS {
        FAST_AT.store(0, Ordering::Relaxed);
        log!(Menus, "Chest Opening: the open's window time is over ({} message call(s) skipped)", SKIPPED.load(Ordering::Relaxed));
        return false;
    }
    let base = base();
    if rd::<u64>(base + SCRIPT_IN_USE_STEAM) as usize != base + TREASURE_STEAM {
        return false;
    }
    let first = base + CONTEXTS_STEAM;
    let count = rd::<u32>(base + COUNT_STEAM).min(CTX_MAX) as usize;
    ctx >= first && ctx < first + count * CTX_SIZE && (ctx - first) % CTX_SIZE == 0
}

/// KH1's skip-mode branch of a message function: its arguments popped, nothing opened.
unsafe fn skip(ctx: usize, which: usize) -> u64 {
    let sp = ctx + CTX_SP;
    std::ptr::write_unaligned(sp as *mut i32, rd::<i32>(sp) - MESSAGE_FNS[which].2);
    if SKIPPED.fetch_add(1, Ordering::Relaxed) < 8 {
        log!(Menus, "Chest Opening: treasure script's message function {} skipped", MESSAGE_FNS[which].0);
    }
    2
}

unsafe fn call(ctx: usize, which: usize) -> u64 {
    if ours_to_skip(ctx) {
        return skip(ctx, which);
    }
    let f: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(ORIG_FN[which].load(Ordering::Acquire));
    f(ctx)
}

unsafe extern "system" fn msg0(ctx: usize) -> u64 {
    call(ctx, 0)
}
unsafe extern "system" fn msg1(ctx: usize) -> u64 {
    call(ctx, 1)
}
unsafe extern "system" fn msg177(ctx: usize) -> u64 {
    call(ctx, 2)
}
unsafe extern "system" fn msg605(ctx: usize) -> u64 {
    call(ctx, 3)
}

#[cfg(test)]
mod tests {
    use super::strip_codes;

    #[test]
    fn strips_unknown_codes() {
        assert_eq!(strip_codes("Hi-Potion"), "Hi-Potion");
        assert_eq!(strip_codes("Elixir{7f}"), "Elixir");
    }
}
