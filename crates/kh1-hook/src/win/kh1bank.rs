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
//! bank's data with `_aligned_free` if the entry's data pointer (`+0x20`) isn't 0 (its
//! destructors `0x1400DB470` / `0x1400DB7A0`; nothing else of KH1's reads `+0x20`, Checked
//! (decomp)). Each of ours is one buffer for the whole session, made once with KH1's own
//! `_aligned_malloc` (its import, alignment 16 as KH1's loads); right after KH1 takes it, the
//! entry's `+0x20` is set to 0, so KH1's release unloads the bank from its engine but leaves the
//! buffer, and once the list has changed and stood still for `SETTLE` frames, the banks missing
//! from it are registered again from the same buffers, a few a frame. So only one copy of each
//! exists (the tester: RAM use; before, ours and KH1's). KH1's engine writes into a bank as it
//! loads it (`0x1400748F0`: byte `+0xF` of each sound record to 0xFF); `scd_cut::as_loaded` does
//! the same up front, and a bank registered again is checked against its first bytes' hash
//! (logged, and left out, if anything else in it changed). Nothing of KH1's is hooked; it runs from the frame's end (`debugmenu`), on the
//! game's thread, as KH1's own game-thread registrations do (`0x1400DB910`).
//!
//! The engine has a fixed number of bank slots (`0x14004F570`: remaining, `0x14004F5F0`: in
//! use, Checked (decomp)); its size isn't known here, so ours are registered only while more than
//! `KEEP_FREE` stay free for KH1's own (logged, both counts). Steam build only (the addresses are
//! the Steam exe's); elsewhere nothing is registered and KH2's sounds aren't heard (the mod has no
//! player of its own: `kh2sound`).

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
/// KH1's `_aligned_malloc` import (Steam IAT `0x1403B0778`, api-ms-win-crt-heap; its
/// `_aligned_free`, which KH1's bank release calls, is at `0x1403B0780`).
const ALIGNED_MALLOC_IAT_STEAM: usize = 0x3B0778;

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
    /// The bank until its buffer is made (then empty).
    bytes: Vec<u8>,
    /// Its buffer (KH1's `_aligned_malloc`, kept for the session: KH1 never frees it) and size.
    data: usize,
    len: usize,
    /// The bank's hash as cut (and as KH1's engine leaves it after a release).
    hash: u64,
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

fn note(line: impl FnOnce() -> String) {
    if !super::log_on(super::LogCat::Sound) {
        return;
    }
    if super::log_on(super::LogCat::Sound) && LOGS.fetch_add(1, Ordering::Relaxed) < 200 {
        log!(Sound, "{}", line());
    }
}

/// Adds a one-sound bank (from the loader thread); its file number, or `None` off Steam.
pub fn add(mut bytes: Vec<u8>, what: String) -> Option<i32> {
    if Build::current() != Build::Steam {
        return None;
    }
    kh2_formats::scd_cut::as_loaded(&mut bytes);
    let hash = hash(&bytes);
    let mut b = BANKS.lock().ok()?;
    let no = FIRST_NO + b.len() as i32;
    let len = bytes.len();
    b.push(Bank { no, bytes, data: 0, len, hash, what, present: false, refused: false });
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
                && rd::<usize>(base + ALIGNED_MALLOC_IAT_STEAM) > 0x10000;
            if !ok {
                log!(Sound, "KH1 banks: KH1's bank registration isn't where the Steam build has it; KH2's sounds won't play");
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

/// A quick 64-bit hash of a bank's bytes (FNV-1a over 8-byte words).
fn hash(b: &[u8]) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    let mut words = b.chunks_exact(8);
    for w in &mut words {
        h = (h ^ u64::from_le_bytes([w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]])).wrapping_mul(0x0000_0100_0000_01B3);
    }
    for &x in words.remainder() {
        h = (h ^ x as u64).wrapping_mul(0x0000_0100_0000_01B3);
    }
    h ^ b.len() as u64
}

/// Registers one bank from its buffer (made in KH1's own allocation the first time), handed to
/// KH1's registration. True if it went into the list.
unsafe fn register(base: usize, b: &mut Bank) -> bool {
    if b.data == 0 {
        let malloc: unsafe extern "C" fn(usize, usize) -> *mut u8 = std::mem::transmute(rd::<usize>(base + ALIGNED_MALLOC_IAT_STEAM));
        let data = malloc(b.len, 0x10);
        if data.is_null() {
            return false;
        }
        std::ptr::copy_nonoverlapping(b.bytes.as_ptr(), data, b.len);
        b.data = data as usize;
        // (The buffer is the only copy from here.)
        b.bytes = Vec::new();
    } else if {
        // (What KH1's load writes, its sound records' `+0xF`, it writes again: put back first.)
        let bank = std::slice::from_raw_parts_mut(b.data as *mut u8, b.len);
        kh2_formats::scd_cut::as_loaded(bank);
        hash(bank) != b.hash
    } {
        // (KH1's engine changed more in it than its load writes: not given to it again.)
        b.refused = true;
        note(|| format!("KH1 banks: se{} ({}) changed while KH1 had it (not only what its load writes); left out", b.no, b.what));
        return false;
    }
    let data = b.data as *mut u8;
    let name = format!("se{}.win32.scd\0", b.no);
    let count: i32 = rd(base + BANK_COUNT_STEAM);
    let f: unsafe extern "system" fn(*const u8, i32, *mut u8, i32) -> u32 = std::mem::transmute(base + REGISTER_STEAM);
    let handle = f(name.as_ptr(), BANK_NUMBER, data, 0);
    let head: usize = rd(base + BANK_LIST_STEAM);
    let took = rd::<i32>(base + BANK_COUNT_STEAM) == count + 1 && head >= 0x10000 && rd::<i32>(head + 0x14) == b.no && rd::<usize>(head + 0x20) == data as usize;
    if took {
        // (KH1's release frees `+0x20` if it isn't 0: the buffer stays ours, for the next
        // registration. The engine keeps its own pointer, `0x14005E0B0`'s bank `+8`.)
        std::ptr::write_unaligned((head + 0x20) as *mut usize, 0);
    }
    if !took {
        // (Not taken: KH1's sound system not up, or the same entry already there. The buffer
        // is still ours.)
        note(|| format!("KH1 banks: se{} ({}) not taken (handle {handle:#x})", b.no, b.what));
    } else if handle == 0 {
        // (In the list, but KH1's engine didn't load it: KH1 releases the entry with the rest;
        // it isn't played.)
        b.refused = true;
        note(|| format!("KH1 banks: se{} ({}): KH1's engine didn't load it (handle 0); it won't play", b.no, b.what));
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
        tend_voice(base);
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
        note(|| format!("KH1 banks: KH1 released banks ({now} of our {} still in its list); putting the rest back", banks.len()));
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
        note(|| format!(
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
    /// Game time left (ms; not counting KH1's pause or Frame Advance holds); for a sound of
    /// unknown length, the time it may take to start.
    left_ms: i64,
    /// Its length is known (`left_ms` counts it down); else it is stopped once KH1 has said it
    /// plays and then that it's done.
    timed: bool,
    /// KH1 said it plays (for an untimed one).
    seen: bool,
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
/// not in KH1's list, no Sora, the queue full): it isn't played, as KH1 drops its own then.
/// `ms` is its length when known.
pub unsafe fn play(no: i32, ms: Option<u32>) -> bool {
    play_id(no, ms).is_some()
}

/// `play`, with the id KH1's queue holds it by (for `fade_stop`), if played.
pub unsafe fn play_id(no: i32, ms: Option<u32>) -> Option<i32> {
    if Build::current() != Build::Steam {
        return None;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    if base == 0 || rd::<usize>(base + SOUND_UP_STEAM) == 0 || !ready(base) || !present(no) {
        return None;
    }
    let sora = super::takeover::sora_actor();
    if sora < 0x10000 {
        return None;
    }
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let queue: unsafe extern "system" fn(i32, i32, usize, i32) = std::mem::transmute(base + QUEUE_STEAM);
    queue(no, id, sora + 0x10, 0);
    if !queue_has(base, no, id) {
        static N: AtomicU32 = AtomicU32::new(0);
        if super::log_on(super::LogCat::Sound) && N.fetch_add(1, Ordering::Relaxed) < 10 {
            log!(Sound, "KH1 banks: KH1's effect queue is full; se{no} isn't played");
        }
        return None;
    }
    if let Ok(mut p) = PLAYING.lock() {
        p.push(match ms {
            Some(ms) => Playing { no, id, left_ms: ms as i64 + 100, timed: true, seen: false },
            None => Playing { no, id, left_ms: 2000, timed: false, seen: false },
        });
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 40 {
        note(|| format!("KH1 banks: se{no} queued in KH1's effect queue (id {id:#x}, {})", ms.map_or("length unknown".to_string(), |m| format!("{m} ms"))));
    }
    Some(id)
}

/// t1014: KH1's fading stop of a queued sound (Steam `0x140179A20 (no, id, fade)`, `sub rsp, 38h;
/// mov r10d, [rip+..]`, Checked (decomp) and (exe)): it frees the slot (`+0x1C` = 0) and calls
/// `0x1400D2FF0 (record, no, id, -1, fade)`, the engine's SslSeFade to nothing, a stop over `fade`.
/// KH2 stops an effect element's sound so as its bind node goes (`0x1401DE500 (handle, 0x78)`,
/// Checked (decomp)); Assumption: the fade's unit is the same in both (both go to the same
/// engine call).
const FADE_STOP_STEAM: usize = 0x179A20;
const FADE_STOP_PROLOGUE: [u8; 7] = [0x48, 0x83, 0xEC, 0x38, 0x44, 0x8B, 0x15];

pub unsafe fn fade_stop(no: i32, id: i32, fade: i32) {
    if Build::current() != Build::Steam {
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    if base == 0 || !starts_with(base + FADE_STOP_STEAM, &FADE_STOP_PROLOGUE) || !queue_has(base, no, id) {
        return;
    }
    let stop: unsafe extern "system" fn(i32, i32, i32) = std::mem::transmute(base + FADE_STOP_STEAM);
    stop(no, id, fade);
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
        let now_playing = playing(x.no, x.id) & 0xFF != 0;
        let done = if x.timed {
            // (Its length played; KH1 says it's done, or it's well past it.)
            x.left_ms <= 0 && (!now_playing || x.left_ms < -2000)
        } else if now_playing {
            // (Of unknown length: KH1 says it plays; done when it says it doesn't, 30 s at most.)
            if !x.seen {
                x.seen = true;
                x.left_ms = 30_000;
            }
            x.left_ms <= 0
        } else {
            // (Played and done, or never started within its 2 s.)
            x.seen || x.left_ms <= 0
        };
        if done {
            stop(x.no, x.id);
            return false;
        }
        true
    });
}

// ---- battle quotes through KH1's voice path (plan step 4) ----

/// KH1's registration of a voice bank from a buffer it copies (Steam `0x1400E0110 (size, bank
/// number, data, name)`; KH1 itself never calls it; its own voice banks go through `0x1400DFE60`,
/// which also takes the entry's name in r9, Checked (decomp / exe)): marks an entry of the same number released (0xFFFF, freed by `0x1400DC8C0`),
/// copies the data with `_aligned_malloc`, loads it with `0x14004D530`, pushes the entry on the
/// voice bank list): `push rdi; push r14; push r15; sub rsp, 0x40`.
const VOICE_REGISTER_STEAM: usize = 0xE0110;
/// Our voice bank's entry name (the registration's fourth argument): under 0x20 bytes with its
/// end and no `%`, in the form of KH1's own voice entries' names (`0x1400D2D10`: the file's name
/// up to `_vsb`, then `.win32.scd`).
const VOICE_NAME: &[u8] = b"kh2_tt0_sora.win32.scd\0";
const _: () = assert!(VOICE_NAME.len() <= 0x20);
/// The voice bank list and its count (Steam `0x1404D65B8`, `0x1404D65C8`; `+0x10` the bank
/// number, `+0x18` the engine handle, `+0x20` the data).
const VOICE_LIST_STEAM: usize = 0x4D65B8;
const VOICE_COUNT_STEAM: usize = 0x4D65C8;
/// Each party member's voice bank number (Steam `0x142D37270`, i8 by member id `< 10`): KH1's
/// clip request `0x140290080` reads Sora's from it into his voice channel.
const MEMBER_BANKS_STEAM: usize = 0x2D37270;
/// KH1's voice channels (Steam `0x142D371D0`, 4 of 0x28 bytes: `+0` state (2 asked), `+1` kind
/// (0 a member's bank), `+2` bank, `+3` clip, `+8` the actor); an actor's channel is its data's
/// `+0xBD` (data = handle `+0x6C`).
const CHANNELS_STEAM: usize = 0x2D371D0;

/// KH1's live sound list (Steam `0x1404D6618`, next at `+8`): each record (`Axa::SoundEntry`,
/// 0x50 bytes) has its engine sound at `+0x10`, bank `+0x30`, clip `+0x34`, state `+0x3C` (3 a
/// voice clip, `0x1400DBCF0`). Checked (decomp).
const LIVE_LIST_STEAM: usize = 0x4D6618;
/// The engine's sound stop (Steam `0x1400565A0 (sound, &fade)`: `movd xmm0, [rdx]; cvtdq2ps xmm0,
/// xmm0`, then the stop `0x1400565D0` under the engine's lock), the call KH1's own `SslSeStop`
/// (`0x1400DEF50`) and its frame clean-up (`0x1400DC8C0`, at `0x1400DCA9C`: `lea rdx, [rsp+0x20]`
/// = 0; `lea rcx, [rbx+0x10]`) make on a record. Checked (exe, decomp).
const SOUND_STOP_STEAM: usize = 0x565A0;
const SOUND_STOP_PROLOGUE: [u8; 7] = [0x66, 0x0F, 0x6E, 0x02, 0x0F, 0x5B, 0xC0];

/// KH2's last quote priority taken (KH2's quote request `0x1403CE580`, its object's `+0x10`: set
/// on each line taken, never reset; a new line is taken while one plays only if its priority is
/// at least this). Checked (decomp, KH2).
static LAST_PRIORITY: AtomicI32 = AtomicI32::new(0);

/// Stops every voice clip record of KH1's live list on bank `no` (only Sora's KH2 quotes use it),
/// with the engine's stop at no fade, as KH1's own stop does; KH1's frame clean-up
/// (`0x1400DC8C0`) then takes each one out of the list and deletes it once it no longer plays,
/// as it does with its own finished records. The list itself is left to KH1. `None` if the stop
/// isn't the one expected (nothing done).
unsafe fn stop_voice_records(base: usize, no: i32) -> Option<u32> {
    if !starts_with(base + SOUND_STOP_STEAM, &SOUND_STOP_PROLOGUE) {
        return None;
    }
    let stop: unsafe extern "system" fn(usize, *const i32) = std::mem::transmute(base + SOUND_STOP_STEAM);
    let fade: i32 = 0;
    let mut stopped = 0;
    let mut node: usize = rd(base + LIVE_LIST_STEAM);
    let mut n = 0;
    while node >= 0x10000 && n < 4096 && super::crash::readable(node) && super::crash::readable(node + 0x4F) {
        let next: usize = rd(node + 8);
        if rd::<i32>(node + 0x3C) == 3 && rd::<i32>(node + 0x30) == no {
            stop(node + 0x10, &fade);
            stopped += 1;
        }
        node = next;
        n += 1;
    }
    Some(stopped)
}

/// Sora's KH2 battle voice bank (whole, as KH1 keeps a character's voices in one bank), its
/// number in KH1's voice list and the data KH1 copied it to (to know it's still there).
static VOICE_BANK: Mutex<Option<Vec<u8>>> = Mutex::new(None);
static VOICE_NO: AtomicI32 = AtomicI32::new(-1);
static VOICE_DATA: AtomicUsize = AtomicUsize::new(0);
static VOICE_COUNT: AtomicI32 = AtomicI32::new(-1);

/// Hands over Sora's KH2 battle voice bank (from the loader thread).
pub fn set_voice_bank(bytes: Vec<u8>) {
    if Build::current() != Build::Steam {
        return;
    }
    if let Ok(mut v) = VOICE_BANK.lock() {
        *v = Some(bytes);
    }
    VOICE_COUNT.store(-1, Ordering::Relaxed);
}

/// Each frame (from `frame_end`): keeps our voice bank in KH1's voice list.
unsafe fn tend_voice(base: usize) {
    if !starts_with(base + VOICE_REGISTER_STEAM, &REGISTER_PROLOGUE) {
        return;
    }
    let count: i32 = rd(base + VOICE_COUNT_STEAM);
    if count == VOICE_COUNT.swap(count, Ordering::Relaxed) {
        return;
    }
    // (The list changed, or the bank is new: is ours still in it?)
    let mut used = Vec::new();
    let mut ours = false;
    let (no, data) = (VOICE_NO.load(Ordering::Relaxed), VOICE_DATA.load(Ordering::Relaxed));
    let mut node: usize = rd(base + VOICE_LIST_STEAM);
    let mut n = 0;
    while node >= 0x10000 && n < 4096 && super::crash::readable(node) && super::crash::readable(node + 0x28) {
        let b: i32 = rd(node + 0x10);
        used.push(b);
        if b == no && rd::<usize>(node + 0x20) == data && data != 0 {
            ours = true;
        }
        node = rd(node + 8);
        n += 1;
    }
    if ours {
        return;
    }
    VOICE_DATA.store(0, Ordering::Relaxed);
    let Ok(v) = VOICE_BANK.try_lock() else {
        // (The loader is handing it over: next frame.)
        VOICE_COUNT.store(-1, Ordering::Relaxed);
        return;
    };
    let Some(bytes) = v.as_ref() else { return };
    // A bank number no voice bank of KH1's has now (the channel keeps it as an i8).
    let Some(pick) = (104..=127).rev().find(|b| !used.contains(b)) else {
        note(|| "KH1 banks: no free voice bank number; KH2's battle quotes won't play".into());
        return;
    };
    // (Its fourth argument, r9, is the entry's name: KH1 keeps it in r15 and passes it as the
    // format of `sprintf (entry + 0x30, name)` (`0x140051260`) before linking the entry, Checked
    // (exe) at `0x1400E0132` and `0x1400E0279..0x1400E0280`; the decomp shows three arguments.
    // The entry is 0x50 bytes (`0x14034B54C (0x50)`), so 0x20 for the name, and no `%`. Without
    // it KH1 formatted whatever r9 held.)
    if bytes.is_empty() || bytes.len() > i32::MAX as usize {
        return;
    }
    let f: unsafe extern "system" fn(i32, i32, *const u8, *const u8) = std::mem::transmute(base + VOICE_REGISTER_STEAM);
    f(bytes.len() as i32, pick, bytes.as_ptr(), VOICE_NAME.as_ptr());
    let head: usize = rd(base + VOICE_LIST_STEAM);
    if head >= 0x10000 && rd::<i32>(head + 0x10) == pick && rd::<u32>(head + 0x18) != 0 {
        VOICE_NO.store(pick, Ordering::Relaxed);
        VOICE_DATA.store(rd(head + 0x20), Ordering::Relaxed);
        note(|| format!("KH1 banks: KH2's battle voice bank in KH1's voice list as bank {pick} ({} bytes)", bytes.len()));
    } else {
        VOICE_NO.store(-1, Ordering::Relaxed);
        note(|| format!("KH1 banks: KH2's battle voice bank not taken as bank {pick}; quotes won't play"));
    }
    VOICE_COUNT.store(rd(base + VOICE_COUNT_STEAM), Ordering::Relaxed);
}

/// Asks KH1 for Sora's battle quote `clip` (of KH2's voice bank) at KH2's `priority`, through
/// KH1's own clip request (`kh1voice::clip`, `0x140290930`: it plays only over a line of lower
/// standing, 0 none, 1 another speaker's, 2 his own, and KH1's voice channel plays it, with its
/// mouth motion), at `priority` (1 at least: KH1 has nothing below 0): Sora's member bank number
/// is ours for that call only.
///
/// KH2's rule over its own lines (`0x1403CE580` / `0x1403CE260`): while one of Sora's KH2 quotes
/// is on his channel, a new one of priority at least the last taken stops it and takes the
/// channel; a lower one is dropped. The playing line's records are stopped
/// (`stop_voice_records`) and the channel set free (state 0, what KH1's channel step
/// `0x140290A50` writes when a line ends), then KH1's request takes the new line as on a free
/// channel. A line of KH1's own on the channel (another bank or speaker) keeps KH1's rule.
/// False if it can't go that way (not heard); true also when a rule drops it.
pub unsafe fn voice(clip: u8, priority: i32) -> bool {
    if Build::current() != Build::Steam || clip > 127 {
        return false;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let no = VOICE_NO.load(Ordering::Relaxed);
    if base == 0 || no < 0 || VOICE_DATA.load(Ordering::Relaxed) == 0 || rd::<usize>(base + SOUND_UP_STEAM) == 0 {
        return false;
    }
    let sora = super::takeover::sora_actor();
    if sora < 0x10000 {
        return false;
    }
    let data = super::pose::ptr(rd(sora + 0x130));
    let me = super::pose::ptr(rd(sora + 0x6C));
    if data < 0x10000 || me < 0x10000 || !super::crash::readable(data + 0x4C) || !super::crash::readable(me + 0xBD) {
        return false;
    }
    let member: u16 = rd(data + 0x4C);
    let ch: i8 = rd(me + 0xBD);
    if member >= 10 || !(0..4).contains(&ch) {
        return false;
    }
    let slot = base + MEMBER_BANKS_STEAM + member as usize;
    let old: i8 = rd(slot);
    if old < 0 {
        // (KH1 gives Sora no voice bank now: no quotes, as KH1's own.)
        return true;
    }
    let c = base + CHANNELS_STEAM + ch as usize * 0x28;
    // (Channel: `+0` state (2 asked, 3 made, 4 playing), `+1` kind (0 a member's bank), `+2`
    // bank, `+3` clip, `+8` the speaker.)
    let state: u8 = rd(c);
    let ours = (2..=4).contains(&state) && rd::<u8>(c + 1) == 0 && rd::<i8>(c + 2) == no as i8 && rd::<usize>(c + 8) == sora;
    if ours {
        static CUTS: AtomicU32 = AtomicU32::new(0);
        let (last, playing) = (LAST_PRIORITY.load(Ordering::Relaxed), rd::<i8>(c + 3));
        if priority < last {
            if CUTS.fetch_add(1, Ordering::Relaxed) < 40 {
                note(|| format!("KH2 quotes: clip {clip} (priority {priority}) dropped: KH2's rule keeps clip {playing} (priority {last}) playing"));
            }
            return true;
        }
        match stop_voice_records(base, no) {
            Some(n) => {
                std::ptr::write_unaligned(c as *mut u8, 0);
                if CUTS.fetch_add(1, Ordering::Relaxed) < 40 {
                    note(|| format!("KH2 quotes: clip {playing} (priority {last}, channel state {state}) cut for clip {clip} (priority {priority}), {n} KH1 voice record(s) stopped"));
                }
            }
            None => {
                if CUTS.fetch_add(1, Ordering::Relaxed) < 40 {
                    note(|| "KH2 quotes: KH1's sound stop isn't the one expected; KH1's rule keeps the line".into());
                }
            }
        }
    }
    std::ptr::write_unaligned(slot as *mut i8, no as i8);
    super::kh1voice::clip(sora, clip as i32, priority.max(1));
    std::ptr::write_unaligned(slot as *mut i8, old);
    let took = rd::<u8>(c) == 2 && rd::<i8>(c + 2) == no as i8 && rd::<u8>(c + 3) == clip && rd::<usize>(c + 8) == sora;
    if took {
        LAST_PRIORITY.store(priority, Ordering::Relaxed);
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 40 {
        note(|| format!("KH1 banks: KH2 quote clip {clip} asked of KH1's voice channel {ch} at priority {priority}: {}", if took { "taken" } else { "KH1 kept the line playing" }));
    }
    true
}
