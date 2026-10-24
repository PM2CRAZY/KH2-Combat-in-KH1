//! KH2's sounds as banks of KH1's own sound engine (the tester, "KH2 sounds through KH1's sound
//! system": question 1 (a); notes and addresses in `docs/kh1-sound-native.md`).
//!
//! Each KH2 sound the mod uses is cut out of its KH2 bank into a one-sound bank in the form of
//! KH1's own effect files (`kh2_formats::scd_cut`), named `se9NNNNN.win32.scd` (file number
//! 900000 and up: KH1's own run to ~60131, so none can be the same), and registered into KH1's
//! bank list (`0x1404D65D8`) with KH1's own registration (Steam `0x1400E0F00 (name, bank, data,
//! flags)`, the one KH1's room-sound loads use: it makes the bank entry, loads the bank with
//! the engine's loader `0x14004D530` and reads the file number from the name). KH1 then knows
//! them as it knows its own: `SslSeCreate (-1, no, id)` finds the entry by its file number and
//! creates its sound 0.
//!
//! KH1 releases every bank of the list at once (`0x1400D2C70 (1)` -> `0x1400DBAC0`, from its map
//! tear-down steps `0x14016FFA0`, `0x140170230`, `0x140170300`, `0x140170D50`), freeing each
//! bank's data with `_aligned_free`. So each registration gets its own copy of the bank made with
//! KH1's own `_aligned_malloc` (its import, alignment 16 as KH1's loads), and once the list has
//! changed and stood still for `SETTLE` frames, the banks missing from it are registered again,
//! a few a frame. Nothing of KH1's is hooked; it runs from the frame's end (`debugmenu`), on the
//! game's thread, as KH1's own game-thread registrations do (`0x1400DB910`).
//!
//! The engine has a fixed number of bank slots (`0x14004F570`: remaining, `0x14004F5F0`: in
//! use, Checked (decomp)); its size isn't known here, so ours are registered only while more than
//! `KEEP_FREE` stay free for KH1's own (logged, both counts). Steam build only (the addresses are
//! the Steam exe's); elsewhere nothing is registered and the sounds play as before.

use super::addr::Build;
use super::log;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

/// KH1's bank registration (Steam `0x1400E0F00`): `push rdi; push r14; push r15; sub rsp, 0x40`.
const REGISTER_STEAM: usize = 0xE0F00;
const REGISTER_PROLOGUE: [u8; 10] = [0x40, 0x57, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x40];
/// The engine's bank slots left / in use (Steam `0x14004F570` / `0x14004F5F0`): `push rbx; sub
/// rsp, 0x30` both.
const BANKS_LEFT_STEAM: usize = 0x4F570;
const BANKS_USED_STEAM: usize = 0x4F5F0;
const COUNT_PROLOGUE: [u8; 6] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x30];
/// KH1's sound system object (Steam `0x1421AAFD8`, non-zero once up: the registration checks it
/// too). Its `+0x18` is the semaphore KH1's bank-adding thread (`0x1400DBA40` -> "_add_scd_thread"
/// `0x1400E0A50`) holds while it adds a room's banks (taken with `0x1400E43B0`, given back with
/// `0x1400E42A0`); ours are added only while we can take it, so the two never add at once.
const SOUND_UP_STEAM: usize = 0x21AAFD8;
/// The bank list's head and its count (Steam `0x1404D65D8`, `0x1404D65E8`).
const BANK_LIST_STEAM: usize = 0x4D65D8;
const BANK_COUNT_STEAM: usize = 0x4D65E8;
/// KH1's `_aligned_malloc` / `_aligned_free` imports (Steam IAT `0x1403B0778` / `0x1403B0780`,
/// api-ms-win-crt-heap).
const ALIGNED_MALLOC_IAT_STEAM: usize = 0x3B0778;
const ALIGNED_FREE_IAT_STEAM: usize = 0x3B0780;

/// KH1's effect queue (Steam `0x140179AC0 (no, id, position pointer, volume)`, the one its hit,
/// object and actor sounds use): `mov [rsp+8], rbx`.
const QUEUE_STEAM: usize = 0x179AC0;
const QUEUE_PROLOGUE: [u8; 5] = [0x48, 0x89, 0x5C, 0x24, 0x08];
/// KH1's stop of a queued sound (Steam `0x140179D90 (no, id)`: frees its slot and stops / deletes
/// its record, `0x1400DEF50`) and its "still playing" (Steam `0x1401789F0 (no, id)`, via
/// `0x1400DEEE0`). Both start `mov r9d, [rip+..]` (the slot count) then `xor r8d, r8d`.
const STOP_STEAM: usize = 0x179D90;
const PLAYING_STEAM: usize = 0x1789F0;
/// The slot table and its slot count (Steam `0x14232D560` pointer, `0x14232D554` count): 0x20
/// bytes a slot, `+0` no, `+0x18` id, `+0x1C` state (0 free).
const SLOTS_STEAM: usize = 0x232D560;
const SLOT_COUNT_STEAM: usize = 0x232D554;

/// Our banks' file numbers (KH1's own are under 100000).
pub const FIRST_NO: i32 = 900_000;
/// The bank number our entries carry (`+0x10`). KH1 looks entries up by it with numbers from its
/// own data (2 the indexed system bank, -8 the music ducking test, -1, 90..103 Sora's voice
/// banks, and the 16-bit numbers its room / event data pass), so ours is out of 16-bit range.
const BANK_NUMBER: i32 = 0x4B32_0000;
/// Frames the bank list must stand still after a change before ours are put back.
const SETTLE: u32 = 30;
/// Registrations a frame.
const PER_FRAME: usize = 16;
/// Bank slots always left to KH1.
const KEEP_FREE: i32 = 128;

struct Bank {
    no: i32,
    bytes: Vec<u8>,
    what: String,
    /// In KH1's bank list now.
    present: bool,
    /// KH1's engine didn't load it (handle 0): never played through KH1.
    refused: bool,
}

static BANKS: Mutex<Vec<Bank>> = Mutex::new(Vec::new());
/// Banks waiting to be put in (a new bank, or the list changed).
static DIRTY: AtomicBool = AtomicBool::new(false);
static LAST_COUNT: AtomicI32 = AtomicI32::new(-1);
static LAST_HEAD: AtomicUsize = AtomicUsize::new(0);
/// Frames since the list last changed.
static STILL: AtomicU32 = AtomicU32::new(0);
static LOGS: AtomicU32 = AtomicU32::new(0);
/// Ours have been put in once (a later shortfall is KH1's reset).
static SEEN: AtomicBool = AtomicBool::new(false);

fn note(line: String) {
    if LOGS.fetch_add(1, Ordering::Relaxed) < 200 {
        log(&line);
    }
}

/// Adds a one-sound bank (from the loader thread); its file number, or `None` off Steam.
pub fn add(bytes: Vec<u8>, what: String) -> Option<i32> {
    if Build::current() != Build::Steam {
        return None;
    }
    let mut b = BANKS.lock().ok()?;
    let no = FIRST_NO + b.len() as i32;
    b.push(Bank { no, bytes, what, present: false, refused: false });
    DIRTY.store(true, Ordering::Relaxed);
    Some(no)
}

/// Whether bank `no` is in KH1's bank list now (as last seen: false from the frame the list
/// shrinks until ours are back).
pub fn present(no: i32) -> bool {
    BANKS.try_lock().ok().and_then(|b| b.get((no - FIRST_NO) as usize).map(|x| x.present && !x.refused)).unwrap_or(false)
}

unsafe fn rd<T: Copy>(at: usize) -> T {
    std::ptr::read_unaligned(at as *const T)
}

unsafe fn starts_with(at: usize, bytes: &[u8]) -> bool {
    std::slice::from_raw_parts(at as *const u8, bytes.len()) == bytes
}

/// KH1's functions this needs, checked once.
unsafe fn ready(base: usize) -> bool {
    static CHECKED: AtomicU32 = AtomicU32::new(0);
    match CHECKED.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => {
            let ok = starts_with(base + REGISTER_STEAM, &REGISTER_PROLOGUE)
                && starts_with(base + BANKS_LEFT_STEAM, &COUNT_PROLOGUE)
                && starts_with(base + BANKS_USED_STEAM, &COUNT_PROLOGUE)
                && starts_with(base + QUEUE_STEAM, &QUEUE_PROLOGUE)
                && [STOP_STEAM, PLAYING_STEAM].iter().all(|&f| starts_with(base + f, &[0x44, 0x8B, 0x0D]) && starts_with(base + f + 7, &[0x45, 0x33, 0xC0]))
                && rd::<usize>(base + ALIGNED_MALLOC_IAT_STEAM) > 0x10000
                && rd::<usize>(base + ALIGNED_FREE_IAT_STEAM) > 0x10000;
            if !ok {
                log("KH1 banks: KH1's bank registration isn't where the Steam build has it; KH2's sounds stay on the mod's player");
            }
            CHECKED.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
            ok
        }
    }
}

unsafe fn banks_left(base: usize) -> i32 {
    let f: unsafe extern "system" fn() -> i32 = std::mem::transmute(base + BANKS_LEFT_STEAM);
    f()
}

unsafe fn banks_used(base: usize) -> i32 {
    let f: unsafe extern "system" fn() -> i32 = std::mem::transmute(base + BANKS_USED_STEAM);
    f()
}

/// The file numbers of the entries in KH1's bank list that are ours.
unsafe fn ours_in_list(base: usize) -> Vec<i32> {
    let mut out = Vec::new();
    let mut node: usize = rd(base + BANK_LIST_STEAM);
    let mut n = 0;
    while node >= 0x10000 && n < 8192 && super::crash::readable(node) && super::crash::readable(node + 0x20) {
        let no: i32 = rd(node + 0x14);
        if no >= FIRST_NO {
            out.push(no);
        }
        node = rd(node + 8);
        n += 1;
    }
    out
}

/// Registers one bank: a copy in KH1's own allocation, handed to KH1's registration. True if it
/// went into the list.
unsafe fn register(base: usize, b: &mut Bank) -> bool {
    let malloc: unsafe extern "C" fn(usize, usize) -> *mut u8 = std::mem::transmute(rd::<usize>(base + ALIGNED_MALLOC_IAT_STEAM));
    let free: unsafe extern "C" fn(*mut u8) = std::mem::transmute(rd::<usize>(base + ALIGNED_FREE_IAT_STEAM));
    let data = malloc(b.bytes.len(), 0x10);
    if data.is_null() {
        return false;
    }
    std::ptr::copy_nonoverlapping(b.bytes.as_ptr(), data, b.bytes.len());
    let name = format!("se{}.win32.scd\0", b.no);
    let count: i32 = rd(base + BANK_COUNT_STEAM);
    let f: unsafe extern "system" fn(*const u8, i32, *mut u8, i32) -> u32 = std::mem::transmute(base + REGISTER_STEAM);
    let handle = f(name.as_ptr(), BANK_NUMBER, data, 0);
    let head: usize = rd(base + BANK_LIST_STEAM);
    let took = rd::<i32>(base + BANK_COUNT_STEAM) == count + 1 && head >= 0x10000 && rd::<i32>(head + 0x14) == b.no && rd::<usize>(head + 0x20) == data as usize;
    if !took {
        // (Not taken: KH1's sound system not up, or the same entry already there. The data is
        // still ours.)
        free(data);
        note(format!("KH1 banks: se{} ({}) not taken (handle {handle:#x})", b.no, b.what));
    } else if handle == 0 {
        // (In the list, but KH1's engine didn't load it: KH1 frees it with the rest; it plays on
        // the mod's player.)
        b.refused = true;
        note(format!("KH1 banks: se{} ({}): KH1's engine didn't load it (handle 0); it stays on the mod's player", b.no, b.what));
    }
    took
}

/// Once a frame, at the frame's end (`debugmenu::frame_end`): watches KH1's bank list and puts
/// ours (back) in.
pub unsafe fn frame_end(base: usize) {
    if Build::current() != Build::Steam || base == 0 {
        return;
    }
    if rd::<usize>(base + SOUND_UP_STEAM) != 0 && ready(base) {
        tend(base);
    }
    let count: i32 = rd(base + BANK_COUNT_STEAM);
    let head: usize = rd(base + BANK_LIST_STEAM);
    let before = LAST_COUNT.swap(count, Ordering::Relaxed);
    if count != before || head != LAST_HEAD.swap(head, Ordering::Relaxed) {
        if count < before {
            // (KH1 released banks: ours may be gone; none counts as there until seen again.)
            if let Ok(mut b) = BANKS.try_lock() {
                b.iter_mut().for_each(|x| x.present = false);
            }
        }
        STILL.store(0, Ordering::Relaxed);
        DIRTY.store(true, Ordering::Relaxed);
        return;
    }
    let still = STILL.fetch_add(1, Ordering::Relaxed) + 1;
    if !DIRTY.load(Ordering::Relaxed) || still < SETTLE || rd::<usize>(base + SOUND_UP_STEAM) == 0 || !ready(base) {
        return;
    }
    let Ok(mut banks) = BANKS.try_lock() else { return };
    if banks.is_empty() {
        return;
    }
    let sys_obj: usize = rd(base + SOUND_UP_STEAM);
    if !super::crash::readable(sys_obj + 0x18) {
        return;
    }
    let sem: usize = rd(sys_obj + 0x18);
    if sem == 0 || super::sys::WaitForSingleObject(sem as *mut _, 0) != 0 {
        // (KH1 is adding a room's banks: next frame.)
        return;
    }
    put_in(base, &mut banks);
    super::sys::ReleaseSemaphore(sem as *mut _, 1, std::ptr::null_mut());
    // (The registrations moved the list: count the next change from here, not as a new one.)
    LAST_COUNT.store(rd(base + BANK_COUNT_STEAM), Ordering::Relaxed);
    LAST_HEAD.store(rd(base + BANK_LIST_STEAM), Ordering::Relaxed);
}

/// Puts ours that are missing from KH1's bank list in (holding KH1's adding semaphore).
unsafe fn put_in(base: usize, banks: &mut [Bank]) {
    // Which of ours are in the list now.
    let have = ours_in_list(base);
    let was = banks.iter().filter(|b| b.present).count();
    for b in banks.iter_mut() {
        b.present = have.contains(&b.no);
    }
    let now = banks.iter().filter(|b| b.present).count();
    if now < was || was == 0 && now < banks.len() && SEEN.load(Ordering::Relaxed) {
        note(format!("KH1 banks: KH1 released banks ({now} of our {} still in its list); putting the rest back", banks.len()));
    }
    let mut put = 0;
    let mut full = false;
    SEEN.store(true, Ordering::Relaxed);
    for b in banks.iter_mut().filter(|b| !b.present && !b.refused) {
        if put == PER_FRAME {
            break;
        }
        if banks_left(base) <= KEEP_FREE {
            full = true;
            break;
        }
        if register(base, b) {
            b.present = true;
            put += 1;
        } else {
            // (Left out until the list changes again.)
            break;
        }
    }
    let missing = banks.iter().filter(|b| !b.present && !b.refused).count();
    if put > 0 && (missing == 0 || full) || full && put == 0 {
        note(format!(
            "KH1 banks: {} of {} KH2 sound banks in KH1's engine ({} bank slots left, {} in use{})",
            banks.len() - missing,
            banks.len(),
            banks_left(base),
            banks_used(base),
            if full { format!("; {missing} left out to keep {KEEP_FREE} slots for KH1") } else { String::new() }
        ));
    }
    if missing == 0 || full || put == 0 {
        DIRTY.store(false, Ordering::Relaxed);
    }
}

// ---- playing through KH1's effect queue (plan step 2) ----

/// A KH2 sound queued in KH1's effect queue: its slot is freed (KH1's own stop) once it has
/// played its length in game time and KH1 says it's done.
struct Playing {
    no: i32,
    id: i32,
    /// Game time left (ms; not counting KH1's pause or Frame Advance holds).
    left_ms: i64,
}

static PLAYING: Mutex<Vec<Playing>> = Mutex::new(Vec::new());
/// Each play's own id (KH1 tells sounds of a bank apart by (no, id); two of the same sound can
/// play at once, as two of KH1's do).
static NEXT_ID: AtomicI32 = AtomicI32::new(0x4B32_0000);
static LAST_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

unsafe fn queue_has(base: usize, no: i32, id: i32) -> bool {
    let slots: usize = rd(base + SLOTS_STEAM);
    let n: i32 = rd(base + SLOT_COUNT_STEAM);
    if slots < 0x10000 || !(1..=0x100).contains(&n) {
        return false;
    }
    (0..n as usize).any(|k| {
        let at = slots + k * 0x20;
        rd::<u8>(at + 0x1C) != 0 && rd::<i32>(at) == no && rd::<i32>(at + 0x18) == id
    })
}

/// Plays bank `no` (a KH2 sound, `ms` long) at Sora through KH1's effect queue, as KH1 plays its
/// own actors' sounds (`0x140179AC0 (no, id, Sora's position +0x10, 0)`: KH1's queue step
/// `0x1401791E0` creates and starts it with `SslSeCreate` / `SslSePlay` and pans and fades it by
/// its distance from the camera's two ears every frame). False if it can't (not Steam, the bank
/// not in KH1's list, no Sora, the queue full): the caller plays it the old way.
pub unsafe fn play(no: i32, ms: u32) -> bool {
    if Build::current() != Build::Steam || !super::settings::get().kh1_sound_engine {
        return false;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    if base == 0 || rd::<usize>(base + SOUND_UP_STEAM) == 0 || !ready(base) || !present(no) {
        return false;
    }
    let sora = super::takeover::sora_actor();
    if sora < 0x10000 {
        return false;
    }
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let queue: unsafe extern "system" fn(i32, i32, usize, i32) = std::mem::transmute(base + QUEUE_STEAM);
    queue(no, id, sora + 0x10, 0);
    if !queue_has(base, no, id) {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 10 {
            log(&format!("KH1 banks: KH1's effect queue is full; se{no} plays on the mod's player"));
        }
        return false;
    }
    if let Ok(mut p) = PLAYING.lock() {
        p.push(Playing { no, id, left_ms: ms as i64 + 100 });
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 40 {
        note(format!("KH1 banks: se{no} queued in KH1's effect queue (id {id:#x}, {ms} ms)"));
    }
    true
}

/// Once a frame: frees the slots of ours that have played (KH1's queue keeps a slot until its
/// sound is stopped by its (no, id), as KH1's own callers do with `0x140179D90`).
unsafe fn tend(base: usize) {
    let now = super::frameadv::clock_ms();
    let last = LAST_MS.swap(now, Ordering::Relaxed);
    let dt = if last == 0 || super::takeover::paused() || super::frameadv::holding() { 0 } else { now.saturating_sub(last) as i64 };
    let Ok(mut p) = PLAYING.try_lock() else { return };
    if p.is_empty() {
        return;
    }
    let playing: unsafe extern "system" fn(i32, i32) -> u32 = std::mem::transmute(base + PLAYING_STEAM);
    let stop: unsafe extern "system" fn(i32, i32) = std::mem::transmute(base + STOP_STEAM);
    p.retain_mut(|x| {
        x.left_ms -= dt;
        if !queue_has(base, x.no, x.id) {
            // (KH1 freed it: a room's queue reset, or its own stop.)
            return false;
        }
        // (Its length played; KH1 says it's done, or it's well past it.)
        if x.left_ms <= 0 && (playing(x.no, x.id) & 0xFF == 0 || x.left_ms < -2000) {
            stop(x.no, x.id);
            return false;
        }
        true
    });
}
