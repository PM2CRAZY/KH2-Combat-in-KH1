//! The loading screen (what-is-still-missing, m7 answer 2 (a), the tester: "when the game starts
//! there should be a loading screen showing what the extraction is doing before continuing into
//! the menu, this should render in engine and be native, and not as a layer like the debug menu").
//! Notes and addresses: `docs/kh1-asset-cache.md` §2.
//!
//! - **Held where KH1 itself waits for nothing.** KH1's field frame (Steam `0x140170D50`) runs its
//!   game mode machine (`0x140170870`, called only from `0x140170E1A`) first. In state 2 (the
//!   i32 at `0x14233FBC0`) the machine loads the title program (`dktitle.rel`) and moves on to 3;
//!   the systems the title needs (text, sound, the task list) were set up by state 1's
//!   `0x140170630`. While our loader thread is building something (the motion slot cache missed:
//!   a bake ran), that one call is skipped in state 2: KH1 keeps running the rest of its frame
//!   (the window, the pad, the task list, the 2D work, the pacing, the present), and the title
//!   comes the frame the building ends. Nothing in KH1 reads state 2 or times it.
//! - **KH1's own text, in KH1's font, on KH1's frame.** After KH1's 2D work (`0x14017B190`, called
//!   from `0x140170FF0`; KH1's own debug lines are drawn at its end), the lines are drawn with
//!   KH1's text routine `0x1402CFBF0 (align, colour, x, y, text, scale)`, the one behind
//!   `0x1402D02F0` that draws KH1's own loading line, centred (align 2) on KH1's 640 x 448
//!   screen. Text in KH1's codes (`menu::encode`).
//! - **Nothing after it.** Both call sites get their original bytes back once released (the title
//!   started, or nothing to build), so no code of ours runs in KH1's frame from then on.
//! - **Never stuck.** Released when the loader ends (a guard released on a failure or a panic
//!   too), when there was nothing to build, and after `MAX_HOLD` whatever happens (the rest then
//!   builds behind the title, as it did before). Steam only.

use super::addr::Build;
use super::log;
use super::sys::*;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// The mode machine's call site in the field frame (`E8` -> `0x140170870`), Steam.
const MODE_CALL_STEAM: usize = 0x170E1A;
const MODE_MACHINE_STEAM: usize = 0x170870;
/// The 2D work's call site in the field frame (`E8` -> `0x14017B190`), Steam.
const WORK_2D_CALL_STEAM: usize = 0x170FF0;
const WORK_2D_STEAM: usize = 0x17B190;
/// The mode machine's state (i32), Steam.
const MODE_STATE_STEAM: usize = 0x233FBC0;
/// KH1's text routine (align, colour, x, y, text, scale), Steam.
const TEXT_STEAM: usize = 0x2CFBF0;
/// The state that loads the title.
const LOADS_TITLE: i32 = 2;
/// The longest hold, whatever the loader is doing.
const MAX_HOLD_MS: u64 = 90_000;

/// What the loader thread is doing: still working (not known yet whether anything is built),
/// building (a bake ran: the screen shows), or done.
const WORKING: u8 = 0;
const BUILDING: u8 = 1;
const DONE: u8 = 2;
static PHASE: AtomicU8 = AtomicU8::new(WORKING);
/// The step shown: 0 baking, 1 saving.
static STEP: AtomicU8 = AtomicU8::new(0);
static MOTION: AtomicU32 = AtomicU32::new(0);
static COUNT: AtomicU32 = AtomicU32::new(0);
static TOTAL: AtomicU32 = AtomicU32::new(0);

static BASE: AtomicUsize = AtomicUsize::new(0);
/// The two call sites are ours (until released).
static ON: AtomicBool = AtomicBool::new(false);
/// When the hold began (the first held frame; `None`: not held yet).
static HELD_SINCE: Mutex<Option<Instant>> = Mutex::new(None);
static HELD_FRAMES: AtomicU64 = AtomicU64::new(0);
/// The call sites' original 5 bytes.
static ORIGINAL: Mutex<[[u8; 5]; 2]> = Mutex::new([[0; 5]; 2]);

// --- The loader thread's side (any build; only atomics) ---

/// The slot build starts: `total` motions to make (baked or from the cache).
pub fn plan(total: usize) {
    TOTAL.store(total as u32, Ordering::Relaxed);
    COUNT.store(0, Ordering::Relaxed);
}

/// The next motion is being made (from the cache or baked).
pub fn making(motion: u16) {
    MOTION.store(motion as u32, Ordering::Relaxed);
    COUNT.fetch_add(1, Ordering::Relaxed);
}

/// A bake really runs (the cache didn't have it): from now on the screen shows.
pub fn baking() {
    STEP.store(0, Ordering::Relaxed);
    let _ = PHASE.compare_exchange(WORKING, BUILDING, Ordering::Relaxed, Ordering::Relaxed);
}

/// The cache is being written.
pub fn saving() {
    STEP.store(1, Ordering::Relaxed);
}

/// The loader is done (or failed).
pub fn finished() {
    PHASE.store(DONE, Ordering::Release);
}

/// Calls `finished` when dropped: the loader thread holds one, so a failure or a panic in it
/// releases the hold too.
pub struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        finished();
    }
}

// --- The game thread's side (Steam) ---

/// Points KH1's mode machine and 2D work calls at ours (Steam only; each call site is checked).
pub unsafe fn install() {
    if Build::current() != Build::Steam {
        return;
    }
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    let mut orig = [[0u8; 5]; 2];
    for (i, site) in [MODE_CALL_STEAM, WORK_2D_CALL_STEAM].into_iter().enumerate() {
        std::ptr::copy_nonoverlapping((base + site) as *const u8, orig[i].as_mut_ptr(), 5);
    }
    if let Ok(mut o) = ORIGINAL.lock() {
        *o = orig;
    }
    if !super::hook::redirect_call(base + MODE_CALL_STEAM, base + MODE_MACHINE_STEAM, mode_machine as *const () as usize, "game mode machine (loading screen)") {
        return;
    }
    if !super::hook::redirect_call(base + WORK_2D_CALL_STEAM, base + WORK_2D_STEAM, work_2d as *const () as usize, "2D work (loading screen)") {
        restore(base, 0);
        return;
    }
    ON.store(true, Ordering::Relaxed);
    log("loading screen: ready (KH1 waits before its title while the mod builds from your files)");
}

/// Puts call site `i`'s original bytes back.
unsafe fn restore(base: usize, i: usize) {
    let Ok(o) = ORIGINAL.lock() else { return };
    let site = base + [MODE_CALL_STEAM, WORK_2D_CALL_STEAM][i];
    let mut now = [0u8; 5];
    std::ptr::copy_nonoverlapping(site as *const u8, now.as_mut_ptr(), 5);
    if now != o[i] {
        super::hook::patch(site, &now, &o[i], "loading screen release");
    }
}

/// Both call sites back to KH1's: none of ours runs in KH1's frame after this.
unsafe fn release(why: &str) {
    if !ON.swap(false, Ordering::Relaxed) {
        return;
    }
    let base = BASE.load(Ordering::Relaxed);
    restore(base, 0);
    restore(base, 1);
    let held = HELD_SINCE.lock().ok().and_then(|h| *h).map_or(0.0, |t| t.elapsed().as_secs_f32() * 1000.0);
    let frames = HELD_FRAMES.load(Ordering::Relaxed);
    if frames > 0 {
        log(&format!("loading screen: title held {held:.0} ms ({frames} frames, {}); released: {why}", if PHASE.load(Ordering::Relaxed) == WORKING { "nothing shown" } else { "shown" }));
    } else {
        log(&format!("loading screen: not needed ({why})"));
    }
}

/// KH1's mode machine, skipped in state 2 while the loader works.
unsafe extern "system" fn mode_machine() {
    let base = BASE.load(Ordering::Relaxed);
    let machine: unsafe extern "system" fn() = std::mem::transmute(base + MODE_MACHINE_STEAM);
    if !ON.load(Ordering::Relaxed) {
        return machine();
    }
    let state = std::ptr::read_volatile((base + MODE_STATE_STEAM) as *const i32);
    let phase = PHASE.load(Ordering::Acquire);
    if phase == DONE {
        release(if state == LOADS_TITLE { "the build is done" } else { "the loader finished before the title" });
        return machine();
    }
    if state > LOADS_TITLE {
        // (The DLL came in after the title had started: nothing to hold.)
        release("the title had already started");
        return machine();
    }
    if state == LOADS_TITLE {
        let since = {
            let mut h = HELD_SINCE.lock().unwrap_or_else(|e| e.into_inner());
            *h.get_or_insert_with(Instant::now)
        };
        if since.elapsed().as_millis() as u64 >= MAX_HOLD_MS {
            release("held the longest it may; the rest builds behind the title");
            return machine();
        }
        if HELD_FRAMES.fetch_add(1, Ordering::Relaxed) == 0 {
            log("loading screen: KH1's title waits for the mod's loader");
        }
        return;
    }
    machine()
}

/// The lines on screen, in KH1's text codes, made again when what they say changes.
struct Lines {
    key: (u8, u32, u32, u32),
    text: [Vec<u8>; 3],
}
static LINES: Mutex<Option<Lines>> = Mutex::new(None);

/// KH1's 2D work, then our lines while held and building.
unsafe extern "system" fn work_2d(mode: i32) {
    let base = BASE.load(Ordering::Relaxed);
    let work: unsafe extern "system" fn(i32) = std::mem::transmute(base + WORK_2D_STEAM);
    work(mode);
    if !ON.load(Ordering::Relaxed) || PHASE.load(Ordering::Relaxed) != BUILDING || HELD_FRAMES.load(Ordering::Relaxed) == 0 {
        return;
    }
    let key = (STEP.load(Ordering::Relaxed), MOTION.load(Ordering::Relaxed), COUNT.load(Ordering::Relaxed), TOTAL.load(Ordering::Relaxed));
    let Ok(mut g) = LINES.try_lock() else { return };
    if g.as_ref().map_or(true, |l| l.key != key) {
        let (step, motion, n, total) = key;
        let doing = if step == 1 { "Saving the built moves".to_string() } else if total > 0 { format!("Building KH2 move {motion} ({} of {total})", n.min(total)) } else { format!("Building KH2 move {motion}") };
        *g = Some(Lines {
            key,
            text: [
                super::menu::encode("KH2 Combat: building from your KH1 and KH2 files"),
                super::menu::encode(&doing),
                super::menu::encode("This happens once, after an update or a change to your files"),
            ],
        });
    }
    let Some(l) = g.as_ref() else { return };
    let text: unsafe extern "system" fn(i32, u32, i32, i32, *const u8, u32) = std::mem::transmute(base + TEXT_STEAM);
    // (Centred on x 320; KH1's own loading line sits at y 404, scale 15, in this blue.)
    text(2, 0x8080_8080, 320, 196, l.text[0].as_ptr(), 16);
    text(2, 0x80C0_612C, 320, 224, l.text[1].as_ptr(), 15);
    text(2, 0x8080_8080, 320, 404, l.text[2].as_ptr(), 13);
}
