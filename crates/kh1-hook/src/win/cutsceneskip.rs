//! Cutscene Skip, the Config menu's QoL row (the user: "can you add an auto cutscene skipper in qol
//! section"; "lets have a setting for 1. and a setting for 2."; the rules: never item shop text, NPC
//! talks or anything the player starts; skip the dialog cutscenes with high-poly models that the pause
//! menu's "Skip Cutscene" can skip, using the game's own function; never the low-poly ones that need a
//! confirm press for every line, because they sometimes give items). `docs/kh1-cutscene-skip.md`.
//!
//! What KH1 does (Steam, Checked (decomp + exe), and Checked (trace) from the user's t709 log):
//! - A story cutscene with high-poly models turns on bit 0 of the second scene flag word
//!   (`0x142382594`, set by script opcodes through `0x1401A0E60`). The pause menu shows Skip Cutscene
//!   only then. Low-poly dialog scenes, talks and shops never set it (trace).
//! - Choosing Skip Cutscene in the pause menu only sets one flag byte (`0x14232DF89`). KH1's own
//!   script step (`0x1401A2320`) sees the byte with the cutscene bit on, clears it and calls KH1's own
//!   skip (`0x1401A22B0`: runner bit 0 and engine bit `0x400`, fade out, the scene's end), exactly as
//!   the trace shows after the menu (runner 2 -> 3, engine + `0x400`).
//!
//! What this does: with the row on, the moment the cutscene bit turns on the mod writes that one byte
//! (t711: no wait; t710 waited a second and the user: "it should skip the first moment the cutscene
//! loads"). KH1 does the rest itself. No hook,
//! no per-frame work beyond reading one word in the mod's pad read (`probe::pad`), and one atomic load
//! when the row is off.
//!
//! Off: nothing. Seen: only a scene already watched is skipped; the first time through it plays. All:
//! every skippable scene is skipped. "Seen" is the mod's own list (KH1 keeps no record of watched scenes:
//! no seen flags were found in the decomp, Checked (decomp), searched for strings and the skip path),
//! keyed by KH1's room (three numbers), its event file and the scene's order in the room visit, and kept
//! in the ini (`cutscene_seen`), shared by all saves (Assumption: a new game wants that line deleted).
//!
//! Checked (trace, t710 log): all five skips worked, KH1's runner and engine words moved as after the menu.
//! Not checked in game: that skipping without opening the pause menu looks the same (the menu also
//! sets its own fade, Assumption: minor), that the key tells scenes apart in every room, and that the
//! order counter restarts when a room is entered again after a game over (Assumption).

use super::addr::Build;
use super::log;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// 0 off, 1 seen, 2 all (`cutscene_skip = off | seen | all`).
static MODE: AtomicU8 = AtomicU8::new(0);
static VERIFIED: AtomicBool = AtomicBool::new(false);
static CHECKED: AtomicBool = AtomicBool::new(false);
static BASE: AtomicUsize = AtomicUsize::new(0);
/// The cutscene bit is on (seen last look), a skip is due, and it was written for this scene.
static ACTIVE: AtomicBool = AtomicBool::new(false);
static PENDING: AtomicBool = AtomicBool::new(false);
static FIRED: AtomicBool = AtomicBool::new(false);
/// KH1's room (the scene triple) last seen, and how many cutscenes have started in it.
static ROOM: [AtomicI32; 3] = [AtomicI32::new(i32::MIN), AtomicI32::new(i32::MIN), AtomicI32::new(i32::MIN)];
static NTH: AtomicU32 = AtomicU32::new(0);

/// Steam: the second scene flag word (bit 0 = a skippable cutscene), the flag byte the pause menu sets
/// and the script step consumes, the room (scene triple) and the event file number (Checked (decomp)).
const FLAGS2_STEAM: usize = 0x2382594;
const SKIP_FLAG_STEAM: usize = 0x232DF89;
const ROOM_STEAM: [usize; 3] = [0x233FE84, 0x233FE8C, 0x233FE90];
const EVENT_FILE_STEAM: usize = 0x2382524;
/// KH1's skip (`sub rsp, 0x28; cmp dword [..], 0; je +5; call ..`), checked before use.
const SKIP_FN_STEAM: usize = 0x1A22B0;
const SKIP_FN_BYTES: [u8; 18] = [0x48, 0x83, 0xEC, 0x28, 0x83, 0x3D, 0x15, 0xE6, 0x19, 0x02, 0x00, 0x74, 0x05, 0xE8, 0xFE, 0x9F, 0xFE, 0xFF];
/// The script step's own read and clear of the flag byte (`cmp [rip+x], r12b` / `mov [rip+x], r12b`),
/// whose targets must be the flag byte used here.
const STEP_READ_STEAM: usize = 0x1A2871;
const STEP_READ_BYTES: [u8; 3] = [0x44, 0x38, 0x25];
const STEP_CLEAR_STEAM: usize = 0x1A287A;
const STEP_CLEAR_BYTES: [u8; 3] = [0x44, 0x88, 0x25];

/// The scenes already watched: (room 3 numbers, event file, order in the room).
static SEEN: OnceLock<Mutex<Vec<[i32; 5]>>> = OnceLock::new();

fn seen() -> &'static Mutex<Vec<[i32; 5]>> {
    SEEN.get_or_init(|| Mutex::new(Vec::new()))
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// The mode for the Config menu row (0 off, 1 seen, 2 all).
pub fn mode() -> u8 {
    MODE.load(Ordering::Relaxed)
}

fn word(m: u8) -> &'static str {
    match m {
        1 => "seen",
        2 => "all",
        _ => "off",
    }
}

fn parse_seen(text: &str) -> Vec<[i32; 5]> {
    text.split(',')
        .filter_map(|e| {
            let v: Vec<i32> = e.trim().split('.').filter_map(|n| n.trim().parse().ok()).collect();
            <[i32; 5]>::try_from(v).ok()
        })
        .collect()
}

fn write_seen(list: &[[i32; 5]]) -> String {
    list.iter().map(|k| format!("{}.{}.{}.{}.{}", k[0], k[1], k[2], k[3], k[4])).collect::<Vec<_>>().join(",")
}

pub unsafe fn install() {
    let s = super::settings::get();
    *seen().lock().unwrap() = parse_seen(&s.cutscene_seen);
    MODE.store(s.cutscene_skip, Ordering::Relaxed);
    if s.cutscene_skip != 0 {
        verify();
    }
    log!(Menus, "Cutscene Skip: {} (cutscene_skip = {}), {} scenes remembered", word(s.cutscene_skip), word(s.cutscene_skip), seen().lock().unwrap().len());
}

/// The Config menu's Cutscene Skip row, kept in the ini.
pub fn set(m: u8) {
    let m = m.min(2);
    MODE.store(m, Ordering::Relaxed);
    if m != 0 {
        unsafe { verify() };
    }
    ACTIVE.store(false, Ordering::Relaxed);
    PENDING.store(false, Ordering::Relaxed);
    super::settings::save("cutscene_skip", word(m));
    log!(Menus, "Config menu: Cutscene Skip {}", word(m));
}

/// Steam only: KH1's skip function and the script step's reads of the flag byte are the code this was
/// read from (exact bytes), and the step's read and clear name the flag byte used here. Nothing is
/// changed.
unsafe fn verify() {
    if CHECKED.swap(true, Ordering::Relaxed) {
        return;
    }
    if Build::current() != Build::Steam {
        log!(Menus, "Cutscene Skip: Steam only (the addresses are the Steam build's)");
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let at = |rva: usize, n: usize| std::slice::from_raw_parts((base + rva) as *const u8, n);
    if at(SKIP_FN_STEAM, SKIP_FN_BYTES.len()) != SKIP_FN_BYTES {
        log!(Menus, "Cutscene Skip: unexpected bytes at KH1's skip {:02x?}, not used", at(SKIP_FN_STEAM, SKIP_FN_BYTES.len()));
        return;
    }
    for (site, bytes, what) in [(STEP_READ_STEAM, STEP_READ_BYTES, "read"), (STEP_CLEAR_STEAM, STEP_CLEAR_BYTES, "clear")] {
        if at(site, 3) != bytes {
            log!(Menus, "Cutscene Skip: unexpected bytes at the script step's {what} {:02x?}, not used", at(site, 3));
            return;
        }
        let rel = rd::<i32>(base + site + 3);
        let target = (site + 7).wrapping_add(rel as isize as usize);
        if target != SKIP_FLAG_STEAM {
            log!(Menus, "Cutscene Skip: the script step's {what} is at {target:#x}, not where expected, not used");
            return;
        }
    }
    BASE.store(base, Ordering::Relaxed);
    VERIFIED.store(true, Ordering::Relaxed);
}

/// Each pad read (`probe::pad`): follows the cutscene bit; a skip is written as it comes on.
pub fn pad() {
    let mode = MODE.load(Ordering::Relaxed);
    if mode == 0 || !VERIFIED.load(Ordering::Relaxed) {
        return;
    }
    unsafe { step(mode) }
}

unsafe fn step(mode: u8) {
    let base = BASE.load(Ordering::Relaxed);
    if mode == 1 {
        // A new room restarts the count of scenes in it.
        let now = [rd::<i32>(base + ROOM_STEAM[0]), rd::<i32>(base + ROOM_STEAM[1]), rd::<i32>(base + ROOM_STEAM[2])];
        let mut changed = false;
        for i in 0..3 {
            if ROOM[i].swap(now[i], Ordering::Relaxed) != now[i] {
                changed = true;
            }
        }
        if changed {
            NTH.store(0, Ordering::Relaxed);
        }
    }
    // (t712: a bit 0 the In-game Scene Skip set is not a pause-skippable cutscene, `ingameskip.rs`.)
    let on = rd::<u32>(base + FLAGS2_STEAM) & 1 != 0 && !super::ingameskip::forced();
    if !on {
        if ACTIVE.swap(false, Ordering::Relaxed) {
            // The scene is over: a flag byte that was written and not used is dropped, so it can't skip the next one.
            // (t767: not when In-game Scene Skip has just set bit 0 and the byte itself for the scene's
            // line-by-line part after this section: that byte is its, `ingameskip.rs`.)
            if FIRED.swap(false, Ordering::Relaxed) && !super::ingameskip::forced() && rd::<u8>(base + SKIP_FLAG_STEAM) != 0 {
                wr(base + SKIP_FLAG_STEAM, 0u8);
            }
            PENDING.store(false, Ordering::Relaxed);
        }
        return;
    }
    if !ACTIVE.swap(true, Ordering::Relaxed) {
        begin(base, mode);
    }
    if PENDING.load(Ordering::Relaxed) && !FIRED.swap(true, Ordering::Relaxed) {
        PENDING.store(false, Ordering::Relaxed);
        wr(base + SKIP_FLAG_STEAM, 1u8);
        log!(Cutscenes, "cutscene skip: flag byte written, KH1's script step runs its own skip");
    }
}

/// The cutscene bit just came on: skip it (all), or only if it was watched before (seen, else it is
/// remembered now).
unsafe fn begin(base: usize, mode: u8) {
    FIRED.store(false, Ordering::Relaxed);
    if mode == 2 {
        PENDING.store(true, Ordering::Relaxed);
        log!(Cutscenes, "cutscene skip (all): a skippable cutscene started, skipping now");
        return;
    }
    let n = NTH.fetch_add(1, Ordering::Relaxed) as i32 + 1;
    let key = [rd::<i32>(base + ROOM_STEAM[0]), rd::<i32>(base + ROOM_STEAM[1]), rd::<i32>(base + ROOM_STEAM[2]), rd::<i32>(base + EVENT_FILE_STEAM), n];
    let mut list = seen().lock().unwrap();
    if list.contains(&key) {
        PENDING.store(true, Ordering::Relaxed);
        log!(Cutscenes, "cutscene skip (seen): {key:?} was watched before, skipping now");
    } else {
        PENDING.store(false, Ordering::Relaxed);
        list.push(key);
        let text = write_seen(&list);
        drop(list);
        super::settings::save("cutscene_seen", &text);
        log!(Cutscenes, "cutscene skip (seen): {key:?} is new, it plays and is remembered");
    }
}
