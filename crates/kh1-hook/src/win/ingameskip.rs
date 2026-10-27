//! In-game Scene Skip, the Config menu's QoL row (the user: "you can make it a seperate config option
//! in qol, what I want you to do is have the cutscene skip ... it skips that in game cutscene fully,
//! then if any flags give you an item the game gives you the item after the fact").
//! `docs/kh1-ingame-skip.md`.
//!
//! The in-game scenes are the low-poly dialog scenes confirmed line by line, which KH1's pause menu
//! doesn't offer to skip. This uses KH1's own skip on them, the same one the pause menu's Skip
//! Cutscene runs (`cutsceneskip.rs`), so KH1 itself runs the rest of the scene. Steam only.
//!
//! What KH1 does (Checked (decomp + exe), Steam):
//! - An event scene is a script entry other than 0 (`0x142382524` is the entry in use); its threads are
//!   the contexts at `0x1423902D0` (`0x360` bytes each, count `0x14237F4E8`, code `+0x80`, sub-scripts
//!   `+0x88`, 4-byte instructions with the class in the last byte, class 5 ends a sub-script, class
//!   `0x18` calls the event function its operand names from the table at `0x1404FA870`).
//! - Bit 0 of the second scene flag word (`0x142382594`) marks a skippable scene; the script step
//!   (`0x1401A2320`) sees it with the pause menu's flag byte (`0x14232DF89`) and runs KH1's skip
//!   (`0x1401A22B0`): skip mode (bit 0 of `0x1423AB2D0`), a fade out, the message windows cleared.
//! - In skip mode the game's frame step is 128 (`0x14018D1F0`), so waits, moves and motions finish at
//!   once; the message functions (event functions 0, 1, 177, the item message 605) open nothing and
//!   the waits on them (33, 107) pass. Everything else runs: the item and ability gives (583) and the
//!   story flags (648 and the others) are KH1's own, in the scene's own order.
//! - When the scene's threads have all ended KH1 goes back to entry 0 and ends skip mode itself
//!   (`0x1401A2130`: skip mode off, the fade in over a second).
//!
//! What this does: with the row on, the first time an event entry is in event mode (bit `0x400` of the
//! second word, set by event function 513) the mod reads the scene's code once. A scene with the
//! pause menu's own skip (event functions 84, 139, 476, 489, 623) is the Cutscene Skip row's;
//! a scene with a choice in a message (88) or a shop or menu window (222, 268, 278, 392, 628, 629,
//! 673) always plays; with a talk's bit (`0x800`) set it plays. Any other scene: the mod sets bit 0
//! and the flag byte, and KH1's next script step skips it. When the entry is back to 0 (or KH1's skip
//! mode has ended) the mod clears the bit 0 it set. No hook; one atomic load in the pad read when off.
//!
//! Assumption (not run in game): the code read finds the pause-skip functions in every high-poly
//! scene (the t709 / t710 logs only listed each entry's first thread); a scene chained from a skipped
//! one is skipped too, as after the pause menu's skip; the item popup is not shown (KH1 doesn't in
//! skip mode), the item is given.

use super::addr::Build;
use super::log;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};

static ON: AtomicBool = AtomicBool::new(false);
static VERIFIED: AtomicBool = AtomicBool::new(false);
static CHECKED: AtomicBool = AtomicBool::new(false);
static BASE: AtomicUsize = AtomicUsize::new(0);
/// The entry already looked at (0: none), and whether the mod set bit 0 for it.
static JUDGED: AtomicI32 = AtomicI32::new(0);
static FORCED: AtomicBool = AtomicBool::new(false);
/// KH1's skip mode was seen on after the mod set the bit.
static SAW_SKIP: AtomicBool = AtomicBool::new(false);

/// Steam: the entry in use, the second scene flag word, the pause menu's flag byte, the runner word
/// (bit 0 skip mode), the event contexts and their count (Checked (decomp)).
const ENTRY_STEAM: usize = 0x2382524;
const FLAGS2_STEAM: usize = 0x2382594;
const SKIP_FLAG_STEAM: usize = 0x232DF89;
const RUNNER_STEAM: usize = 0x23AB2D0;
const CONTEXTS_STEAM: usize = 0x23902D0;
const COUNT_STEAM: usize = 0x237F4E8;
const CTX_SIZE: usize = 0x360;
const CTX_CODE: usize = 0x80;
const CTX_SUBS: usize = 0x88;
const CLASS_END: u8 = 5;
const CLASS_CALL: u8 = 0x18;
/// The second word's bits: skippable, event mode, a talk.
const BIT_SKIPPABLE: u32 = 1;
const BIT_EVENT: u32 = 0x400;
const BIT_TALK: u32 = 0x800;
/// The most threads and instructions read for one scene.
const CTX_MAX: i32 = 64;
const SCAN_MAX: usize = 8192;

/// KH1's skip (`sub rsp, 0x28; cmp dword [..], 0; je +5; call ..`), as in `cutsceneskip.rs`.
const SKIP_FN_STEAM: usize = 0x1A22B0;
const SKIP_FN_BYTES: [u8; 18] = [0x48, 0x83, 0xEC, 0x28, 0x83, 0x3D, 0x15, 0xE6, 0x19, 0x02, 0x00, 0x74, 0x05, 0xE8, 0xFE, 0x9F, 0xFE, 0xFF];
/// The script step's test of bit 0 (`test byte [rip+x], 1`), its read and clear of the flag byte
/// (`cmp [rip+x], r12b` / `mov [rip+x], r12b`), and where each must point.
const STEP_BIT_STEAM: usize = 0x1A2835;
const STEP_BIT_BYTES: [u8; 2] = [0xF6, 0x05];
const STEP_READ_STEAM: usize = 0x1A2871;
const STEP_READ_BYTES: [u8; 3] = [0x44, 0x38, 0x25];
const STEP_CLEAR_STEAM: usize = 0x1A287A;
const STEP_CLEAR_BYTES: [u8; 3] = [0x44, 0x88, 0x25];
/// The event function table, and the functions this reads scenes by (number, Steam RVA), checked
/// against the table before use so the numbers are this exe's.
const TABLE_STEAM: usize = 0x4FA870;
/// The pause menu's skip turned on (the high-poly scenes, the Cutscene Skip row's). Not 27 and 317: they
/// turn it on only under a condition (`0x1401A19B0`), and 27 is in the low-poly Traverse Town scene
/// (Checked (decomp + trace)); 139 was in all ten high-poly scenes of the t709 / t710 logs (trace).
const PAUSE_SKIP: [(u32, usize); 5] = [(84, 0x1A8B60), (139, 0x1AA0D0), (476, 0x1B02A0), (489, 0x1B05D0), (623, 0x1A8B90)];
/// A choice in a message, and the shop and menu windows: the player answers, so the scene plays.
const PLAYS: [(u32, usize); 8] = [(88, 0x1B8EC0), (222, 0x1CA610), (268, 0x1CA730), (278, 0x1ACC00), (392, 0x1AEAD0), (628, 0x1CA340), (629, 0x1CA450), (673, 0x1CA4F0)];

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// The Config menu row's value.
pub fn enabled() -> bool {
    ON.load(Ordering::Relaxed)
}

/// Bit 0 of the second scene flag word is the mod's own just now (the Cutscene Skip row leaves it).
pub fn forced() -> bool {
    FORCED.load(Ordering::Relaxed)
}

pub unsafe fn install() {
    let on = super::settings::get().ingame_skip;
    ON.store(on, Ordering::Relaxed);
    if on {
        verify();
    }
    log!(Menus, "In-game Scene Skip: {} (ingame_skip)", if on { "on" } else { "off" });
}

/// The Config menu's In-game Scene Skip row, kept in the ini.
pub fn set(v: bool) {
    ON.store(v, Ordering::Relaxed);
    if v {
        unsafe { verify() };
    } else {
        unsafe { release("the row was turned off") };
    }
    super::settings::save("ingame_skip", if v { "on" } else { "off" });
    log!(Menus, "Config menu: In-game Scene Skip {}", if v { "on" } else { "off" });
}

/// Steam only: KH1's skip, the script step's test of bit 0 and its read and clear of the flag byte are
/// the code this was read from, and the event functions named here are where this exe has them.
/// Nothing is changed.
unsafe fn verify() {
    if CHECKED.swap(true, Ordering::Relaxed) {
        return;
    }
    if Build::current() != Build::Steam {
        log!(Menus, "In-game Scene Skip: Steam only (the addresses are the Steam build's)");
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let at = |rva: usize, n: usize| std::slice::from_raw_parts((base + rva) as *const u8, n);
    if at(SKIP_FN_STEAM, SKIP_FN_BYTES.len()) != SKIP_FN_BYTES {
        log!(Menus, "In-game Scene Skip: unexpected bytes at KH1's skip {:02x?}, not used", at(SKIP_FN_STEAM, SKIP_FN_BYTES.len()));
        return;
    }
    // (`test byte [rip+x], 1` is 7 bytes: the displacement counts from its end; the compares 7 too.)
    let target = |site: usize, disp_at: usize, len: usize| (site + len).wrapping_add(rd::<i32>(base + site + disp_at) as isize as usize);
    if at(STEP_BIT_STEAM, 2) != STEP_BIT_BYTES || rd::<u8>(base + STEP_BIT_STEAM + 6) != 1 || target(STEP_BIT_STEAM, 2, 7) != FLAGS2_STEAM {
        log!(Menus, "In-game Scene Skip: the script step's test of the skippable bit isn't where expected, not used");
        return;
    }
    for (site, bytes, what) in [(STEP_READ_STEAM, STEP_READ_BYTES, "read"), (STEP_CLEAR_STEAM, STEP_CLEAR_BYTES, "clear")] {
        if at(site, 3) != bytes || target(site, 3, 7) != SKIP_FLAG_STEAM {
            log!(Menus, "In-game Scene Skip: the script step's {what} of the flag byte isn't where expected, not used");
            return;
        }
    }
    for (n, rva) in PAUSE_SKIP.iter().chain(PLAYS.iter()) {
        let entry = rd::<u64>(base + TABLE_STEAM + *n as usize * 8) as usize;
        if entry.wrapping_sub(base) != *rva {
            log!(Menus, "In-game Scene Skip: event function {n} is at {:#x}, not {rva:#x}, not used", entry.wrapping_sub(base));
            return;
        }
    }
    BASE.store(base, Ordering::Relaxed);
    VERIFIED.store(true, Ordering::Relaxed);
}

/// Each pad read (`probe::pad`).
pub fn pad() {
    if !ON.load(Ordering::Relaxed) || !VERIFIED.load(Ordering::Relaxed) {
        return;
    }
    unsafe { step() }
}

unsafe fn step() {
    let base = BASE.load(Ordering::Relaxed);
    let entry = rd::<i32>(base + ENTRY_STEAM);
    if FORCED.load(Ordering::Relaxed) {
        let skipping = rd::<u32>(base + RUNNER_STEAM) & 1 != 0;
        if skipping {
            SAW_SKIP.store(true, Ordering::Relaxed);
        }
        if entry == 0 {
            release("the scene is over");
        } else if SAW_SKIP.load(Ordering::Relaxed) && !skipping {
            release("KH1's skip mode has ended");
        }
    }
    if entry == 0 {
        JUDGED.store(0, Ordering::Relaxed);
        return;
    }
    if JUDGED.load(Ordering::Relaxed) == entry {
        return;
    }
    let flags = rd::<u32>(base + FLAGS2_STEAM);
    if flags & BIT_EVENT == 0 {
        // (Not in event mode yet: the scene's first frame sets it, event function 513.)
        return;
    }
    JUDGED.store(entry, Ordering::Relaxed);
    if flags & BIT_SKIPPABLE != 0 {
        log!(Cutscenes, "in-game skip: entry {entry} is a pause-skippable cutscene (the Cutscene Skip row's)");
        return;
    }
    if flags & BIT_TALK != 0 {
        log!(Cutscenes, "in-game skip: entry {entry} plays (a talk's bit is set, so the player may have started it)");
        return;
    }
    match read_scene(base) {
        Scene::Skip(n) => {
            wr(base + FLAGS2_STEAM, rd::<u32>(base + FLAGS2_STEAM) | BIT_SKIPPABLE);
            wr(base + SKIP_FLAG_STEAM, 1u8);
            SAW_SKIP.store(false, Ordering::Relaxed);
            FORCED.store(true, Ordering::Relaxed);
            log!(Cutscenes, "in-game skip: entry {entry} ({n} instructions read) skipped: bit 0 and the flag byte set, KH1's script step runs its own skip");
        }
        Scene::PauseSkip(f) => {
            log!(Cutscenes, "in-game skip: entry {entry} has the pause menu's skip (event function {f}): the Cutscene Skip row's");
        }
        Scene::Plays(f) => {
            log!(Cutscenes, "in-game skip: entry {entry} plays (event function {f}: a choice, shop or menu)");
        }
        Scene::Unread => {
            log!(Cutscenes, "in-game skip: entry {entry} plays (its code couldn't be read)");
        }
    }
}

/// The bit 0 the mod set is taken back, and a flag byte it wrote and KH1 didn't use.
unsafe fn release(why: &str) {
    if !FORCED.swap(false, Ordering::Relaxed) {
        return;
    }
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 {
        return;
    }
    let flags = rd::<u32>(base + FLAGS2_STEAM);
    if flags & BIT_SKIPPABLE != 0 {
        wr(base + FLAGS2_STEAM, flags & !BIT_SKIPPABLE);
    }
    if rd::<u8>(base + SKIP_FLAG_STEAM) != 0 {
        wr(base + SKIP_FLAG_STEAM, 0u8);
    }
    log!(Cutscenes, "in-game skip: bit 0 taken back ({why})");
}

enum Scene {
    /// No pause skip, no choice or menu: skip it (instructions read).
    Skip(usize),
    PauseSkip(u32),
    Plays(u32),
    Unread,
}

/// The event functions the scene's threads call, read once from their code.
unsafe fn read_scene(base: usize) -> Scene {
    use super::crash::readable;
    let count = rd::<i32>(base + COUNT_STEAM);
    if !(1..=CTX_MAX).contains(&count) {
        return Scene::Unread;
    }
    let mut total = 0usize;
    let mut plays = None;
    for i in 0..count as usize {
        let ctx = base + CONTEXTS_STEAM + i * CTX_SIZE;
        let code = rd::<u64>(ctx + CTX_CODE) as usize;
        let subs = rd::<i32>(ctx + CTX_SUBS);
        if code < 0x10000 || !(0..=64).contains(&subs) {
            return Scene::Unread;
        }
        let (mut ends, mut n) = (0, 0usize);
        while ends < subs && n < SCAN_MAX {
            let at = code + n * 4;
            if !readable(at) || !readable(at + 3) {
                return Scene::Unread;
            }
            let w = rd::<u32>(at);
            let class = (w >> 24) as u8;
            if class == CLASS_END {
                ends += 1;
            } else if class == CLASS_CALL {
                let f = w & 0xFF_FFFF;
                if PAUSE_SKIP.iter().any(|(n, _)| *n == f) {
                    return Scene::PauseSkip(f);
                }
                if plays.is_none() && PLAYS.iter().any(|(n, _)| *n == f) {
                    plays = Some(f);
                }
            }
            n += 1;
        }
        total += n;
    }
    match plays {
        Some(f) => Scene::Plays(f),
        None => Scene::Skip(total),
    }
}
