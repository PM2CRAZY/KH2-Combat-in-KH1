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
//! Trinities (the user: "can you also make it skip trinity animations?"): a Trinity mark is a context of
//! the room's own script (entry 0, contexts at `0x142382AD0`, count `0x14237F4E4`) that offers the
//! Triangle command 0x53 through event function 454 (`push 0x53, push 1, call 454`: the object table at
//! `0x142382938`, command `+0xC`, Checked (exe + files)) and, when it is used, starts an event entry
//! through event function 126 (`push N, call 126`; N was 50 in every room read, Checked (files)). That
//! entry is the Trinity itself (Sora, Donald and Goofy bound to its first three threads, placed 0, 120
//! and 240 degrees round the mark, Checked (files)). Some story versions of a room run it without event
//! mode (no event function 513), and a talk earlier in the room leaves `0x800` on, so the two rules above
//! never let it through. An entry a Trinity mark starts is judged at once, `0x800` aside, and skipped by
//! the same KH1 skip unless it has the pause skip (the Cutscene Skip row's) or a choice, shop or menu.
//! Assumption (not run in game): that 0x53 is the "Trinity" command (it is offered in 17 rooms of six
//! worlds, every one starting such an entry), and that every Trinity type's result (the jump's ledge,
//! the ladder, the charged wall, the pushed object, the detected chest, the journal's count) is set by
//! the entry's own script steps, which skip mode runs.
//!
//! t764, where the mod's skip ends (the tester: "you get stuck to a black screen during Olympus
//! coliseums tournament"; "(b)", the room script's follow-ups). KH1 ends its own skip in two ways
//! (Checked (decomp + exe)): when every thread of the entry has ended and it goes back to entry 0
//! (`0x1401A2130` from the script step), and at the end of a skippable section (event functions 85,
//! 140, 490 and 624, the pairs of 84, 139, 489 and 623: bit 0 off, then `0x1401A2130`). The Coliseum's
//! tournament rounds are event entries with play in them (a short lead-in, event function 512 giving
//! Sora back, the fight, then the next entry through 126 from the entry), so a skip the mod started
//! never reached either end: the fights ran in skip mode, faded out at KH1's frame step (the log's frame
//! scale 4 from f64556 on). Now event function 512 (`0x1401A7660`, through the event table) ends the mod's
//! skip the way 140 does. In every room script read (110 script files in 38 rooms of nine worlds) 512 is an in-game
//! scene's last step but for the tournament and the Coliseum's other fight entries, where play follows (and the Trinity mark's thread inside Phil's entry in the Coliseum gates, after its Trinity).
//! The room script's follow-up (it carries on in event mode after an entry it started, the Thunder lines
//! after Phil's scene) is read from the context that started that entry (`push n, call 126` just before
//! its next instruction `+0x190`) to its first 514 or 512, with every sub-script it wakes (`21/c 22/s`):
//! no choice, shop or menu on the way and an end found, it gets KH1's skip too (bit 0 and the flag byte;
//! the script step's skip start works with entry 0), and 514 or 512 (event function 514, `0x1401A7750`,
//! through the table) ends it. `crate::kh1script::read_follow_up`, unit-tested.
//!
//! Assumption (not run in game): the code read finds the pause-skip functions in every high-poly
//! scene (the t709 / t710 logs only listed each entry's first thread); a scene chained from a skipped
//! one is skipped too, as after the pause menu's skip; the item popup is not shown (KH1 doesn't in
//! skip mode), the item is given.

use super::addr::Build;
use crate::kh1script::{read_follow_up, started_entry, trinity_call, Call, Follow, CMD_TRINITY, FN_EVENT_OFF, FN_OFFER, FN_RELEASE, FN_START_ENTRY, FOLLOW_SUBS};
use super::log;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};

static ON: AtomicBool = AtomicBool::new(false);
static VERIFIED: AtomicBool = AtomicBool::new(false);
static CHECKED: AtomicBool = AtomicBool::new(false);
static BASE: AtomicUsize = AtomicUsize::new(0);
/// The entry already looked at (0: none), and whether the mod set bit 0 for it.
static JUDGED: AtomicI32 = AtomicI32::new(0);
static FORCED: AtomicBool = AtomicBool::new(false);
/// KH1's skip mode was seen on after the mod set the bit.
static SAW_SKIP: AtomicBool = AtomicBool::new(false);
/// The entry last checked for a Trinity (0: none), and whether it was one.
static TRIN_ENTRY: AtomicI32 = AtomicI32::new(0);
static TRIN_YES: AtomicBool = AtomicBool::new(false);
/// The entries (one byte each, 0: none) the Trinity marks of the event entry now running start: a mark
/// can be a context of an event entry too (Olympus's Coliseum after the barrel game: Phil's entry 1 has
/// one), and its Trinity entry replaces that entry's contexts, so they are read beforehand.
static NEXT_TRIN: AtomicU32 = AtomicU32::new(0);
/// What the mod's bit 0 is for: an event entry (KIND_ENTRY) or the room script's follow-up after one
/// (KIND_FOLLOW, t764).
static KIND: AtomicU8 = AtomicU8::new(0);
const KIND_ENTRY: u8 = 1;
const KIND_FOLLOW: u8 = 2;
/// The entry in use at the last pad read, and the first entry started from the room script (the one
/// its context waits on: a chained entry replaces it, `0x1401A38A0`).
static LAST_ENTRY: AtomicI32 = AtomicI32::new(0);
static FIRST_ENTRY: AtomicI32 = AtomicI32::new(0);
/// The room-script context that started it and the instruction it carries on at (context << 32 |
/// instruction, + 1; 0: none), noted when the entry starts (the room script doesn't step while an
/// entry runs, so that is where its follow-up begins).
static STARTER: AtomicU64 = AtomicU64::new(0);
/// Times the flag byte was set again (for the log).
static REARMED: AtomicU32 = AtomicU32::new(0);
/// KH1's event functions 512 and 514 as the table held them (t764: the mod's skip ends there).
static ORIG_RELEASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_EVENT_OFF: AtomicUsize = AtomicUsize::new(0);
/// KH1's section ends (event functions 85, 140, 490, 624) as the table held them (t765).
static ORIG_SECTION_END: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];

/// Steam: the entry in use, the second scene flag word, the pause menu's flag byte, the runner word
/// (bit 0 skip mode), the event contexts and their count (Checked (decomp)).
const ENTRY_STEAM: usize = 0x2382524;
const FLAGS2_STEAM: usize = 0x2382594;
const SKIP_FLAG_STEAM: usize = 0x232DF89;
const RUNNER_STEAM: usize = 0x23AB2D0;
const CONTEXTS_STEAM: usize = 0x23902D0;
const COUNT_STEAM: usize = 0x237F4E8;
/// The room's own script (entry 0): its contexts and their count (Checked (decomp): the script step
/// `0x1401A2320` and `0x1401A4D00` take these when `0x142382524` is 0).
const CONTEXTS0_STEAM: usize = 0x2382AD0;
const COUNT0_STEAM: usize = 0x237F4E4;
const CTX_SIZE: usize = 0x360;
const CTX_CODE: usize = 0x80;
const CTX_SUBS: usize = 0x88;
/// A context's sub-script starts (`+0x90`, an i32 each) and its next instruction (`+0x190`) (Checked
/// (decomp): the loader `0x1401A38A0` fills them, the script step `0x1401A2320` reads `+0x190`).
const CTX_SUB_STARTS: usize = 0x90;
const CTX_PC: usize = 0x190;
const CLASS_END: u8 = 5;
const CLASS_CALL: u8 = 0x18;
/// Event functions 454 (offer a Triangle command) and 126 (start an event entry), where this exe has
/// them (`crate::kh1script`).
const TRINITY_FNS: [(u32, usize); 2] = [(FN_OFFER, 0x1AF530), (FN_START_ENTRY, 0x1A9D90)];
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
/// KH1's end of a skip (`0x1401A2130`: skip mode off, the message windows closed, a fade in over a
/// second; returns 1 if skip mode was on), what KH1's own section ends call after bit 0 is off (event
/// functions 85, 140, 490 and 624: `0x1401A8BD0`, `0x1401AA120`, `0x1401B05F0`, `0x1401A8C00`). Its first
/// bytes (`push rbx; sub rsp, 0x40; mov rax, [rip+x]`) and event function 140's call of it (`+0x15`).
/// Checked (decomp + exe).
const SKIP_END_STEAM: usize = 0x1A2130;
const SKIP_END_BYTES: [u8; 9] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x40, 0x48, 0x8B, 0x05];
const FN140_STEAM: usize = 0x1AA120;
const FN140_CALL: usize = 0x15;
/// Event functions 512 (Sora given back) and 514 (event mode off), where this exe has them (wrapped
/// in the table, t764).
const END_FNS: [(u32, usize); 2] = [(FN_RELEASE, 0x1A7660), (FN_EVENT_OFF, 0x1A7750)];
/// KH1's skippable sections' ends (the pairs of 84, 139, 489 and 623), where this exe has them (wrapped
/// in the table, t765: a scene that goes on in event mode after its section is the row's).
const SECTION_ENDS: [(u32, usize); 4] = [(85, 0x1A8BD0), (140, 0x1AA120), (490, 0x1B05F0), (624, 0x1A8C00)];
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
    for (n, rva) in PAUSE_SKIP.iter().chain(PLAYS.iter()).chain(TRINITY_FNS.iter()) {
        let entry = rd::<u64>(base + TABLE_STEAM + *n as usize * 8) as usize;
        if entry.wrapping_sub(base) != *rva {
            log!(Menus, "In-game Scene Skip: event function {n} is at {:#x}, not {rva:#x}, not used", entry.wrapping_sub(base));
            return;
        }
    }
    // KH1's end of a skip, the way its own section end (event function 140) calls it.
    if at(SKIP_END_STEAM, SKIP_END_BYTES.len()) != SKIP_END_BYTES || at(FN140_STEAM + FN140_CALL, 1) != [0xE8] || target(FN140_STEAM + FN140_CALL, 1, 5) != SKIP_END_STEAM {
        log!(Menus, "In-game Scene Skip: KH1's end of a skip isn't as expected, not used");
        return;
    }
    // Event functions 512 and 514 point at ours (they call KH1's first, then end the mod's skip).
    let ours = [release_fn as *const () as usize, event_off_fn as *const () as usize];
    let origs = [&ORIG_RELEASE, &ORIG_EVENT_OFF];
    for (i, &(n, rva)) in END_FNS.iter().enumerate() {
        let at = base + TABLE_STEAM + n as usize * 8;
        origs[i].store(base + rva, Ordering::Release);
        if !super::hook::patch(at, &((base + rva) as u64).to_le_bytes(), &(ours[i] as u64).to_le_bytes(), "event function (in-game skip)") {
            log!(Menus, "In-game Scene Skip: event function {n} not wrapped, not used");
            return;
        }
    }
    // The section ends (t765): the part of a scene after its skippable section. Not needed for the rest,
    // so a mismatch here only leaves that part playing.
    let ours = [section_end_85 as *const () as usize, section_end_140 as *const () as usize, section_end_490 as *const () as usize, section_end_624 as *const () as usize];
    for (i, &(n, rva)) in SECTION_ENDS.iter().enumerate() {
        let at = base + TABLE_STEAM + n as usize * 8;
        ORIG_SECTION_END[i].store(base + rva, Ordering::Release);
        if !super::hook::patch(at, &((base + rva) as u64).to_le_bytes(), &(ours[i] as u64).to_le_bytes(), "event function (in-game skip)") {
            log!(Menus, "In-game Scene Skip: event function {n} not wrapped: the part after a section plays");
            ORIG_SECTION_END[i].store(0, Ordering::Release);
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
    let last = LAST_ENTRY.swap(entry, Ordering::Relaxed);
    if FORCED.load(Ordering::Relaxed) {
        let skipping = rd::<u32>(base + RUNNER_STEAM) & 1 != 0;
        if skipping {
            SAW_SKIP.store(true, Ordering::Relaxed);
        }
        if entry == 0 && KIND.load(Ordering::Relaxed) == KIND_ENTRY {
            release("the scene is over");
        } else if SAW_SKIP.load(Ordering::Relaxed) && !skipping {
            release("KH1's skip mode has ended");
        } else if !SAW_SKIP.load(Ordering::Relaxed) && !skipping && rd::<u8>(base + SKIP_FLAG_STEAM) == 0 {
            // t767: the skip hasn't started and the flag byte is gone (cleared by something else before
            // KH1's script step read it): bit 0 and the byte are put back, so the step starts it.
            wr(base + FLAGS2_STEAM, rd::<u32>(base + FLAGS2_STEAM) | BIT_SKIPPABLE);
            wr(base + SKIP_FLAG_STEAM, 1u8);
            if REARMED.fetch_add(1, Ordering::Relaxed) < 4 {
                log!(Cutscenes, "in-game skip: the flag byte was cleared before KH1's skip started, set again");
            }
        }
    }
    if entry == 0 {
        JUDGED.store(0, Ordering::Relaxed);
        TRIN_ENTRY.store(0, Ordering::Relaxed);
        NEXT_TRIN.store(0, Ordering::Relaxed);
        if last != 0 {
            follow_up(base, FIRST_ENTRY.load(Ordering::Relaxed));
        }
        return;
    }
    if last == 0 {
        FIRST_ENTRY.store(entry, Ordering::Relaxed);
        let starter = Ctxs::room(base).and_then(|room| room.starter(entry));
        STARTER.store(starter.map_or(0, |(c, pc)| ((c as u64) << 32 | pc as u64) + 1), Ordering::Relaxed);
    }
    if JUDGED.load(Ordering::Relaxed) == entry {
        return;
    }
    let trinity = trinity_entry(base, entry);
    let flags = rd::<u32>(base + FLAGS2_STEAM);
    if !trinity && flags & BIT_EVENT == 0 {
        // (Not in event mode yet: the scene's first frame sets it, event function 513. A Trinity is
        // judged at once: some story versions of a room run it without event mode.)
        return;
    }
    JUDGED.store(entry, Ordering::Relaxed);
    let what = if trinity { "a Trinity" } else { "a scene" };
    if flags & BIT_SKIPPABLE != 0 {
        log!(Cutscenes, "in-game skip: entry {entry} ({what}) is a pause-skippable cutscene (the Cutscene Skip row's)");
        return;
    }
    if !trinity && flags & BIT_TALK != 0 {
        log!(Cutscenes, "in-game skip: entry {entry} plays (a talk's bit is set, so the player may have started it)");
        return;
    }
    match read_scene(base) {
        Scene::Skip(n) => {
            wr(base + FLAGS2_STEAM, rd::<u32>(base + FLAGS2_STEAM) | BIT_SKIPPABLE);
            wr(base + SKIP_FLAG_STEAM, 1u8);
            SAW_SKIP.store(false, Ordering::Relaxed);
            KIND.store(KIND_ENTRY, Ordering::Relaxed);
            FORCED.store(true, Ordering::Relaxed);
            log!(Cutscenes, "in-game skip: entry {entry} ({what}, {n} instructions read, flags {flags:#x}) skipped: bit 0 and the flag byte set, KH1's script step runs its own skip");
        }
        Scene::PauseSkip(f) => {
            log!(Cutscenes, "in-game skip: entry {entry} ({what}) has the pause menu's skip (event function {f}): the Cutscene Skip row's");
        }
        Scene::Plays(f) => {
            log!(Cutscenes, "in-game skip: entry {entry} ({what}) plays (event function {f}: a choice, shop or menu)");
        }
        Scene::Unread => {
            log!(Cutscenes, "in-game skip: entry {entry} ({what}) plays (its code couldn't be read)");
        }
    }
}

/// The bit 0 the mod set is taken back, and a flag byte it wrote and KH1 didn't use.
unsafe fn release(why: &str) {
    if !FORCED.swap(false, Ordering::Relaxed) {
        return;
    }
    KIND.store(0, Ordering::Relaxed);
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

/// t764: the room script carrying on after an event entry it started (`first`), still in event mode
/// (the Thunder lines after Phil's scene in the Coliseum gates). KH1's skip runs on it too (bit 0 and
/// the flag byte, the script step's `0x1401A22B0` works with any entry), unless the follow-up, or a
/// sub-script it wakes, has a choice, shop or menu, or no 514 / 512 on its way to end the skip at.
unsafe fn follow_up(base: usize, first: i32) {
    let flags = rd::<u32>(base + FLAGS2_STEAM);
    if flags & BIT_EVENT == 0 || flags & BIT_SKIPPABLE != 0 || FORCED.load(Ordering::Relaxed) || rd::<u32>(base + RUNNER_STEAM) & 1 != 0 || first <= 0 {
        return;
    }
    if flags & BIT_TALK != 0 {
        log!(Cutscenes, "in-game skip: the room script goes on in event mode after entry {first}, plays (a talk's bit is set)");
        return;
    }
    let Some(room) = Ctxs::room(base) else { return };
    let noted = STARTER.swap(0, Ordering::Relaxed);
    let Some((ctx, pc)) = (if noted != 0 { Some((((noted - 1) >> 32) as usize, ((noted - 1) & 0xFFFF_FFFF) as usize)) } else { room.starter(first) }) else {
        log!(Cutscenes, "in-game skip: the room script goes on in event mode after entry {first}, plays (no context of it started that entry)");
        return;
    };
    skip_follow_up(base, &room, ctx, pc, &format!("the room script goes on in event mode after entry {first}"));
}

/// Reads a follow-up from context `ctx`'s instruction `pc` and, with no choice, shop or menu on the way
/// and a 514 / 512 to end at, gives it KH1's skip (bit 0 and the flag byte; ended by `end_skip`).
unsafe fn skip_follow_up(base: usize, ctxs: &Ctxs, ctx: usize, pc: usize, what: &str) {
    // A choice, shop or menu makes it play; so does a pause-skippable section ahead (t766: the high-poly
    // scenes are the Cutscene Skip row's alone, the owner: "the goal is to not skip high poly scenes").
    let plays: Vec<u32> = PLAYS.iter().chain(PAUSE_SKIP.iter()).map(|(n, _)| *n).collect();
    match read_follow_up(ctx, pc, |c, i| ctxs.word(c, i), |c, s| ctxs.sub(c, s), &plays) {
        Follow::Skip(n) => {
            wr(base + FLAGS2_STEAM, rd::<u32>(base + FLAGS2_STEAM) | BIT_SKIPPABLE);
            wr(base + SKIP_FLAG_STEAM, 1u8);
            SAW_SKIP.store(false, Ordering::Relaxed);
            KIND.store(KIND_FOLLOW, Ordering::Relaxed);
            FORCED.store(true, Ordering::Relaxed);
            log!(Cutscenes, "in-game skip: {what} (context {ctx}, instruction {pc}, {n} instructions read) skipped: bit 0 and the flag byte set, ends at event function 514 or 512");
        }
        Follow::Plays(f) if PAUSE_SKIP.iter().any(|(n, _)| *n == f) => log!(Cutscenes, "in-game skip: {what} (context {ctx}), plays (event function {f}: a pause-skippable section ahead, the Cutscene Skip row's)"),
        Follow::Plays(f) => log!(Cutscenes, "in-game skip: {what} (context {ctx}), plays (event function {f}: a choice, shop or menu)"),
        Follow::NoEnd => log!(Cutscenes, "in-game skip: {what} (context {ctx}), plays (no 514 or 512 to end a skip at)"),
        Follow::Unread => log!(Cutscenes, "in-game skip: {what} (context {ctx}), plays (its code couldn't be read, or it wakes more than {FOLLOW_SUBS} sub-scripts)"),
    }
}

/// t765: a skippable section's end (event functions 85, 140, 490, 624) through the table: KH1's own,
/// then, if the scene goes on in event mode (Wonderland's Queen's castle: the high-poly part, 140, then
/// lines confirmed one by one up to 512 / 514), that part is read from the next instruction like the
/// room script's follow-up and skipped the same way. Whether KH1's skip ran on the section (the
/// Cutscene Skip row) or it played doesn't matter.
unsafe fn section_end(i: usize, ctx: usize) -> u64 {
    let f: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(ORIG_SECTION_END[i].load(Ordering::Acquire));
    let r = f(ctx);
    if r & 2 == 0 || !ON.load(Ordering::Relaxed) || FORCED.load(Ordering::Relaxed) {
        return r;
    }
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 {
        return r;
    }
    let flags = rd::<u32>(base + FLAGS2_STEAM);
    if flags & BIT_EVENT == 0 || flags & BIT_SKIPPABLE != 0 || flags & BIT_TALK != 0 {
        return r;
    }
    let entry = rd::<i32>(base + ENTRY_STEAM);
    let ctxs = if entry == 0 { Ctxs::room(base) } else { Ctxs::entry(base) };
    let Some(ctxs) = ctxs else { return r };
    let Some(c) = ctxs.index(ctx) else { return r };
    // (The script step moves the context past the call when this returns: the next instruction is the
    // one after the call, `+0x190` + 1.)
    let Ok(pc) = usize::try_from(rd::<i32>(ctx + CTX_PC)) else { return r };
    skip_follow_up(base, &ctxs, c, pc + 1, &format!("entry {entry} goes on in event mode after its skippable section (event function {})", SECTION_ENDS[i].0));
    r
}

unsafe extern "system" fn section_end_85(ctx: usize) -> u64 {
    section_end(0, ctx)
}
unsafe extern "system" fn section_end_140(ctx: usize) -> u64 {
    section_end(1, ctx)
}
unsafe extern "system" fn section_end_490(ctx: usize) -> u64 {
    section_end(2, ctx)
}
unsafe extern "system" fn section_end_624(ctx: usize) -> u64 {
    section_end(3, ctx)
}

/// Script words known readable (t784): the readable region last asked about (`crash::readable_span`),
/// so a scan asks Windows once per region instead of twice per 4-byte word. The owner's t783 log: the
/// scans on the save's first scene (up to 64 threads of up to 8192 words, room and entry) held the
/// game 337 ms and 829 ms in two runs, almost all of it in those queries (`docs/kh1-load-profiler.md`).
#[derive(Default)]
struct Reach(std::cell::Cell<(usize, usize)>);

impl Reach {
    /// `n` bytes at `at` can be read.
    unsafe fn ok(&self, at: usize, n: usize) -> bool {
        self.byte(at) && self.byte(at.saturating_add(n - 1))
    }
    unsafe fn byte(&self, a: usize) -> bool {
        let (s, e) = self.0.get();
        if a >= s && a < e {
            return true;
        }
        match super::crash::readable_span(a) {
            Some(span) => {
                self.0.set(span);
                true
            }
            None => false,
        }
    }
}

/// A script's contexts read in place: the room script's (entry 0) or the event entry's.
struct Ctxs {
    at: usize,
    count: usize,
    reach: Reach,
}

impl Ctxs {
    unsafe fn room(base: usize) -> Option<Ctxs> {
        Ctxs::read(base + CONTEXTS0_STEAM, base + COUNT0_STEAM)
    }
    unsafe fn entry(base: usize) -> Option<Ctxs> {
        Ctxs::read(base + CONTEXTS_STEAM, base + COUNT_STEAM)
    }
    unsafe fn read(at: usize, count: usize) -> Option<Ctxs> {
        let count = rd::<i32>(count);
        (1..=CTX_MAX).contains(&count).then_some(Ctxs { at, count: count as usize, reach: Reach::default() })
    }
    /// The index of the context at `ctx`, if it is one of these.
    fn index(&self, ctx: usize) -> Option<usize> {
        let off = ctx.checked_sub(self.at)?;
        (off % CTX_SIZE == 0 && off / CTX_SIZE < self.count).then_some(off / CTX_SIZE)
    }
    fn ctx(&self, c: usize) -> usize {
        self.at + c * CTX_SIZE
    }
    /// Context `c`'s instruction `i`.
    fn word(&self, c: usize, i: usize) -> Option<u32> {
        if c >= self.count {
            return None;
        }
        unsafe {
            let code = rd::<u64>(self.ctx(c) + CTX_CODE) as usize;
            let at = code.checked_add(i.checked_mul(4)?)?;
            if code < 0x10000 || !self.reach.ok(at, 4) {
                return None;
            }
            Some(rd::<u32>(at))
        }
    }
    /// The first instruction of context `c`'s sub-script `s`.
    fn sub(&self, c: usize, s: usize) -> Option<usize> {
        if c >= self.count {
            return None;
        }
        unsafe {
            if s >= rd::<i32>(self.ctx(c) + CTX_SUBS).clamp(0, 64) as usize {
                return None;
            }
            usize::try_from(rd::<i32>(self.ctx(c) + CTX_SUB_STARTS + s * 4)).ok()
        }
    }
    /// The context whose next instruction follows `push entry, call 126`, and that instruction.
    fn starter(&self, entry: i32) -> Option<(usize, usize)> {
        let entry = u32::try_from(entry).ok()?;
        (0..self.count).find_map(|c| {
            let pc = usize::try_from(unsafe { rd::<i32>(self.ctx(c) + CTX_PC) }).ok()?;
            started_entry(pc, entry, |i| self.word(c, i)).then_some((c, pc))
        })
    }
}

/// Event function 512 (Sora given back to the player) through the table: KH1's own, then the mod's skip
/// (either kind) ends there. An event entry with play in it (the Coliseum's tournament rounds: a short
/// lead-in, 512, the fight, then the next entry) would otherwise run its fight in skip mode, faded out.
unsafe extern "system" fn release_fn(ctx: usize) -> u64 {
    let f: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(ORIG_RELEASE.load(Ordering::Acquire));
    let r = f(ctx);
    end_skip(r, FN_RELEASE)
}

/// Event function 514 (event mode off) through the table: KH1's own, then a follow-up's skip ends there.
unsafe extern "system" fn event_off_fn(ctx: usize) -> u64 {
    let f: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(ORIG_EVENT_OFF.load(Ordering::Acquire));
    let r = f(ctx);
    end_skip(r, FN_EVENT_OFF)
}

/// Ends the mod's skip the way KH1's own section ends do (event function 140: bit 0 off, then
/// `0x1401A2130`, and 6 instead of 2 when that ended a skip, so the thread waits a frame for the fade).
unsafe fn end_skip(r: u64, n: u32) -> u64 {
    if r & 2 == 0 || !ON.load(Ordering::Relaxed) || !FORCED.load(Ordering::Relaxed) {
        return r;
    }
    let kind = KIND.load(Ordering::Relaxed);
    if n == FN_EVENT_OFF && kind != KIND_FOLLOW {
        return r;
    }
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 {
        return r;
    }
    if rd::<u32>(base + RUNNER_STEAM) & 1 == 0 {
        // Reached before KH1's script step started the skip (a short tail run in the same step as the
        // section end): nothing to skip any more, so the bit and the flag byte are taken back before the
        // next step could start a skip in play.
        release(&format!("event function {n} reached before KH1's skip started"));
        return r;
    }
    FORCED.store(false, Ordering::Relaxed);
    KIND.store(0, Ordering::Relaxed);
    wr(base + FLAGS2_STEAM, rd::<u32>(base + FLAGS2_STEAM) & !BIT_SKIPPABLE);
    if rd::<u8>(base + SKIP_FLAG_STEAM) != 0 {
        wr(base + SKIP_FLAG_STEAM, 0u8);
    }
    let end: unsafe extern "system" fn() -> u64 = std::mem::transmute(base + SKIP_END_STEAM);
    let ended = end() as u32 != 0;
    log!(Cutscenes, "in-game skip: event function {n} reached ({}), KH1's end of a skip run (fade in)", if kind == KIND_FOLLOW { "the follow-up's end" } else { "Sora given back" });
    if ended { r | 4 } else { r }
}

enum Scene {
    /// No pause skip, no choice or menu: skip it (instructions read).
    Skip(usize),
    PauseSkip(u32),
    Plays(u32),
    Unread,
}

/// Whether `entry` is a Trinity: a context that offers the Trinity command (`push 0x53, push 1, call
/// 454`) starts it (`push entry, call 126`), in the room's own script or in the event entry that ran
/// just before. Read once per entry; the marks of this entry are noted for the next one.
unsafe fn trinity_entry(base: usize, entry: i32) -> bool {
    if TRIN_ENTRY.load(Ordering::Relaxed) == entry {
        return TRIN_YES.load(Ordering::Relaxed);
    }
    let e = entry as u32;
    let room = marks(base, CONTEXTS0_STEAM, COUNT0_STEAM).into_iter().find(|&(_, n)| n == e);
    let before = NEXT_TRIN.load(Ordering::Relaxed).to_le_bytes().contains(&(e as u8)) && e < 0x100;
    let mut next = 0u32;
    for (k, (_, n)) in marks(base, CONTEXTS_STEAM, COUNT_STEAM).into_iter().filter(|&(_, n)| n > 0 && n < 0x100).take(4).enumerate() {
        next |= n << (k * 8);
    }
    NEXT_TRIN.store(next, Ordering::Relaxed);
    let yes = room.is_some() || before;
    TRIN_ENTRY.store(entry, Ordering::Relaxed);
    TRIN_YES.store(yes, Ordering::Relaxed);
    if let Some((ctx, _)) = room {
        log!(Cutscenes, "in-game skip: entry {entry} is a Trinity (started by context {ctx} of the room's script, which offers command {CMD_TRINITY:#x})");
    } else if before {
        log!(Cutscenes, "in-game skip: entry {entry} is a Trinity (started by a mark of the event entry before it, which offers command {CMD_TRINITY:#x})");
    }
    yes
}

/// In a context array (the room's script or the event entry's), each context that offers the Trinity
/// command, with every entry it starts: (context, entry).
unsafe fn marks(base: usize, contexts: usize, count: usize) -> Vec<(usize, u32)> {
    let reach = Reach::default();
    let mut out = Vec::new();
    let count = rd::<i32>(base + count);
    if !(1..=CTX_MAX).contains(&count) {
        return out;
    }
    for i in 0..count as usize {
        let ctx = base + contexts + i * CTX_SIZE;
        let code = rd::<u64>(ctx + CTX_CODE) as usize;
        let subs = rd::<i32>(ctx + CTX_SUBS);
        if code < 0x10000 || !(0..=64).contains(&subs) {
            continue;
        }
        let (mut ends, mut n) = (0, 0usize);
        // The last three instructions read, newest last.
        let mut last = [0u32; 3];
        let mut offers = false;
        let mut starts = Vec::new();
        while ends < subs && n < SCAN_MAX {
            let at = code + n * 4;
            if !reach.ok(at, 4) {
                break;
            }
            let w = rd::<u32>(at);
            last = [last[1], last[2], w];
            match trinity_call(&last) {
                Some(Call::OfferTrinity) => offers = true,
                Some(Call::StartEntry(e)) => starts.push(e),
                None => {}
            }
            if (w >> 24) as u8 == CLASS_END {
                ends += 1;
            }
            n += 1;
        }
        if offers {
            out.extend(starts.into_iter().map(|e| (i, e)));
        }
    }
    out
}

/// The event functions the scene's threads call, read once from their code.
unsafe fn read_scene(base: usize) -> Scene {
    let reach = Reach::default();
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
            if !reach.ok(at, 4) {
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

