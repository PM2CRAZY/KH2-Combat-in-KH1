//! Frame Advance (t371, t372, the tester Mr. Bugs, `docs/tester-requests.md`): a debug menu row that
//! holds the game still; R2 or the keyboard's period key plays one game frame, and held for a
//! second plays at full speed until let go. Notes and addresses: `docs/kh1-frame-advance.md`.
//!
//! - **Holding the game, as KH1's pause does.** KH1's field loop (Steam `0x140170D50`) runs its
//!   task list once a frame (`0x140284690` -> `0x14028A710 (list 0x142867380, mask)`): a task runs
//!   when its flags (`+2`) hold every bit of the mask; KH1's pause runs only the tasks flagged 1
//!   (the draws, the pause menu's own). Every actor is one task of a second list (`0x142D5AB00`),
//!   run by the main list's task at 80000 (`0x140298940`), which is not flagged 1.
//!   (t372) t371 wrote the mask, but the pause machine (`0x14017F150`, run each frame before the
//!   list) sets it every frame itself, so only the effects held (the tester: Sora went on moving).
//!   Now, while the game is held, each task of the main list not flagged 1 has its function
//!   (`+0x10`) swapped for ours, which does nothing (`skip`), and gets it back for a frame that
//!   runs: the same tasks KH1's pause holds, held whatever the machine writes. From KH1's frame
//!   end (`debugmenu::frame_end`, after the frame's list), so the next frame is held or runs. The
//!   effect instances are paused too (`0x1401ED6D0 (1)`, the pause machine's call as its pause
//!   opens; 0 puts them back). The field loop's own work around the list (the pad, the pause
//!   machine, the drawing, the pacing, the present) runs every frame, so the picture, the HUD and
//!   the debug menu stay up. Nothing of KH1's is hooked for it; only data is written.
//! - **One frame at any frame rate.** KH1's pacing (`0x14018D1F0`, at the end of every pass of the
//!   loop) sets the frame scale (`0x14233FBDC`) from the 60 Hz vblanks since the last pass: 1 at
//!   60 fps, 2 at the 30 fps setting. It runs on held frames too, so the stepped frame gets the
//!   same scale as a frame played at that setting (no catch-up after a long hold), and so do our
//!   KH2 steps (`takeover::steps`, from that scale).
//! - **Buttons.** KH1's pad record (`0x14298CA90`: `+4` held, `+8` pressed, `+0xC` released, the
//!   repeat timers at `+0x1C` / `+0x20`) is rebuilt every frame from the one before (`0x14028B430`:
//!   pressed = held now and not held in the record). Held frames still rebuild it, so before a
//!   stepped frame the record's held buttons and timers are put back to the last frame that ran:
//!   the stepped frame then sees a press against what the game last saw. A button held over five
//!   steps is one press; one pressed and let go between steps isn't seen; one let go is released
//!   on the next step (the tester's rule).
//! - **R2 and period** step the game and nothing else sees them while Frame Advance is on: R2 is
//!   taken off every pad record (`probe::pad`, as the debug menu hides its buttons), so KH1 and
//!   the KH2 side (the lock-on switch) don't see it. The period key counts only while the game's
//!   window is in front. (t377) KH1 doesn't see the period key either: KH1 reads the keyboard
//!   into two 256-byte buffers (`0x1400DA9F0`: input object `+0x12A0`, the one this frame at
//!   `+0x14A0`) and maps the keys to its pad in `0x1400D7E80 (input object)`; a hook on that
//!   mapping's start clears the period key's byte (DirectInput `DIK_PERIOD`, 0x34) in both
//!   buffers while Frame Advance is on.
//! - **The mod's clock** (`clock_ms`) stops while the game is held, so the gaps and windows the
//!   mod times in real time (a pause seen, KH2 active, the trace's frame gaps) read a stepped
//!   frame as one played at full speed; `age` does the same for the draw's "posed lately" checks
//!   (KH2's model, its pose, the hands, the left Keyblade), which would otherwise go stale and
//!   hand Sora back to KH1's model while the game is held.
//! - **Steam only**: the task list and the effect pause call are Steam addresses, with no Epic
//!   counterpart checked (the row says so on another build).

use super::addr::{Build, Rva};
use super::log;
use super::sys::*;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// KH1's main task list (Steam `0x142867380`): `+0x10` the first task; a task (0x18 bytes):
/// `+0` in use (0 free), `+2` flags (u16), `+4` priority, `+8` the next, `+0x10` its function
/// `(task) -> u32` (bit 2: remove it, bit 3: stop the list).
const TASK_LIST_STEAM: usize = 0x2867380;
/// The tasks KH1's pause runs have this flag.
const PAUSE_FLAG: u16 = 1;
/// KH1's effect instances paused or not (Steam `0x1401ED6D0 (on)`).
const EFFECT_PAUSE_STEAM: usize = 0x1ED6D0;
/// KH1's pause machine (0 while the game runs).
const PAUSE_STATE_RVA: Rva = Rva(0x232E9B4);
/// KH1's pad record (held `+4`, the repeat timers `+0x1C` / `+0x20`).
const BUTTONS_RVA: Rva = Rva(0x298D4A0);
/// KH1's keyboard mapping (Steam `0x1400D7E80 (input object)`): `push rsi; push r14; sub rsp, 0x48`,
/// read in the Steam exe.
const KEY_MAP_STEAM: usize = 0xD7E80;
const KEY_MAP_PROLOGUE: [u8; 8] = [0x40, 0x56, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x48];
static ORIG_KEY_MAP: AtomicUsize = AtomicUsize::new(0);
/// The period key in DirectInput's key state.
const DIK_PERIOD: usize = 0x34;
/// R2 in the pad's PS2 order.
const R2: u16 = 1 << 9;
/// The period key (`VK_OEM_PERIOD`).
const VK_PERIOD: i32 = 0xBE;
/// Held this long, the button plays the game at full speed.
const FAST_AFTER: Duration = Duration::from_secs(1);

static ON: AtomicBool = AtomicBool::new(false);
/// R2 on any pad record this frame (`pad`).
static R2_HELD: AtomicBool = AtomicBool::new(false);
static BASE: AtomicUsize = AtomicUsize::new(0);

static START: OnceLock<Instant> = OnceLock::new();
/// Real time the game was held, before the hold going on now (ms).
static HELD_TOTAL_MS: AtomicU64 = AtomicU64::new(0);
/// When the hold going on now began (real ms since `START`, +1; 0: not held).
static HELD_SINCE_MS: AtomicU64 = AtomicU64::new(0);

struct State {
    /// Our hold is up (mask 1 for the next frame).
    held: bool,
    /// The step button last frame, and since when it is held.
    was_down: bool,
    down_at: Option<Instant>,
    /// KH1's pad record as the last frame that ran left it (held, timers).
    pad: Option<(u32, u32, u32)>,
    /// KH1's pause was up during our hold (it gave the models back as it closed).
    kh1_paused: bool,
}

/// The tasks held now: (task, its own function).
static HELD_TASKS: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());

/// A held task's function: does nothing, keeps the task.
unsafe extern "system" fn skip(_task: usize) -> u32 {
    0
}

/// Every task of the main list not flagged for a pause is held (its function swapped for
/// `skip`); tasks added since the last call too.
unsafe fn hold_tasks(base: usize) {
    let Ok(mut held) = HELD_TASKS.lock() else { return };
    let ours = skip as *const () as usize;
    let mut t: usize = rd(base + TASK_LIST_STEAM + 0x10);
    let mut n = 0;
    while t > 0x10000 && n < 1024 && super::crash::readable(t + 0x18) {
        let f: usize = rd(t + 0x10);
        if rd::<u16>(t) != 0 && rd::<u16>(t + 2) & PAUSE_FLAG == 0 && f != ours && f > 0x10000 {
            // (A task freed and made again meanwhile: its new function is the one to give back.)
            match held.iter_mut().find(|e| e.0 == t) {
                Some(e) => e.1 = f,
                None => held.push((t, f)),
            }
            wr::<usize>(t + 0x10, ours);
        }
        t = rd(t + 8);
        n += 1;
    }
}

/// The held tasks get their functions back (those still in use and still ours).
unsafe fn release_tasks() {
    let Ok(mut held) = HELD_TASKS.lock() else { return };
    let ours = skip as *const () as usize;
    for (t, f) in held.drain(..) {
        if super::crash::readable(t + 0x18) && rd::<u16>(t) != 0 && rd::<usize>(t + 0x10) == ours {
            wr::<usize>(t + 0x10, f);
        }
    }
}

static STATE: Mutex<State> = Mutex::new(State { held: false, was_down: false, down_at: None, pad: None, kh1_paused: false });

fn start() -> Instant {
    *START.get_or_init(Instant::now)
}

/// Real time (ms since the mod's start).
pub fn wall_ms() -> u64 {
    start().elapsed().as_millis() as u64
}

/// The mod's clock (ms): real time less the time Frame Advance held the game.
pub fn clock_ms() -> u64 {
    let now = wall_ms();
    let since = HELD_SINCE_MS.load(Ordering::Relaxed);
    let now_held = if since != 0 { now.saturating_sub(since - 1) } else { 0 };
    now.saturating_sub(HELD_TOTAL_MS.load(Ordering::Relaxed) + now_held)
}

/// Frame Advance holds the game now.
pub fn holding() -> bool {
    HELD_SINCE_MS.load(Ordering::Relaxed) != 0
}

/// How long ago `t` was, not counting the hold going on now (`t` from before it: its age when
/// the hold began).
pub fn age(t: Instant) -> Duration {
    let since = HELD_SINCE_MS.load(Ordering::Relaxed);
    if since == 0 {
        return t.elapsed();
    }
    let began = start() + Duration::from_millis(since - 1);
    began.saturating_duration_since(t)
}

/// Frame Advance works in this build (the Steam one).
pub fn supported() -> bool {
    Build::current() == Build::Steam
}

/// The debug menu's Frame Advance row.
pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// Turns it on or off (the debug menu; off when the debug menu is turned off). On also turns
/// the trace's Game Frames row on (the tester's pick); it can be turned back off there.
pub fn set_on(on: bool) {
    if on && !supported() {
        return;
    }
    if ON.swap(on, Ordering::Relaxed) != on {
        if on {
            super::trace::set_game_frames(true);
        }
        log(&format!("debug menu: frame advance {}", if on { "on (R2 / period: one frame, held 1 s: full speed)" } else { "off" }));
    }
}

/// Hooks KH1's keyboard mapping (Steam only; the hook checks its bytes first).
pub unsafe fn install() {
    if !supported() {
        return;
    }
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    super::hook::install(base + KEY_MAP_STEAM, &KEY_MAP_PROLOGUE, key_map as *const () as usize, &ORIG_KEY_MAP, "keyboard mapping (frame advance's period key)");
}

/// KH1's keys to its pad: with Frame Advance on, the period key reads as up in both of KH1's
/// keyboard buffers (`+0x12A0`, `+0x13A0`), so only Frame Advance sees it.
unsafe extern "system" fn key_map(input: usize) {
    if ON.load(Ordering::Relaxed) && input > 0x10000 {
        wr::<u8>(input + 0x12A0 + DIK_PERIOD, 0);
        wr::<u8>(input + 0x13A0 + DIK_PERIOD, 0);
    }
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_KEY_MAP.load(Ordering::Acquire));
    f(input);
}

/// Each pad read (`probe::pad`, KH1's records merged): R2 noted; true: the game must not see R2.
pub fn pad(buttons: u16) -> bool {
    let on = ON.load(Ordering::Relaxed);
    R2_HELD.store(on && buttons & R2 != 0, Ordering::Relaxed);
    on
}

/// R2 in the pad's PS2 order (for `probe::pad` to take off).
pub const R2_BIT: u16 = R2;

/// The period key is down and the game's window is in front.
unsafe fn period_down() -> bool {
    if (GetAsyncKeyState(VK_PERIOD) as u16 & 0x8000) == 0 {
        return false;
    }
    let w = GetForegroundWindow();
    if w.is_null() {
        return false;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(w, &mut pid);
    pid == GetCurrentProcessId()
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

unsafe fn pause_effects(base: usize, on: bool) {
    let f: unsafe extern "system" fn(i32) = std::mem::transmute(base + EFFECT_PAUSE_STEAM);
    f(on as i32);
}

/// Ends our hold: the next frame runs.
unsafe fn release(base: usize, st: &mut State, kh1_paused: bool) {
    if !st.held {
        return;
    }
    st.held = false;
    release_tasks();
    // (While KH1's pause is up it holds the effects itself, and gives them back as it closes.)
    if !kh1_paused {
        pause_effects(base, false);
    }
    let since = HELD_SINCE_MS.swap(0, Ordering::Relaxed);
    if since != 0 {
        HELD_TOTAL_MS.fetch_add(wall_ms().saturating_sub(since - 1), Ordering::Relaxed);
    }
}

/// KH1's frame end, once a frame after the frame's task list (`debugmenu::frame_end`): whether
/// the next frame runs or is held.
pub unsafe fn frame_end(base: usize) {
    if !supported() {
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    let Ok(mut st) = STATE.try_lock() else { return };
    let st = &mut *st;
    let kh1_paused = rd::<i32>(base + PAUSE_STATE_RVA) != 0;
    let rec = base + BUTTONS_RVA;
    if !st.held {
        // This frame ran: its pad is what the game last saw.
        st.pad = Some((rd(rec + 4), rd(rec + 0x1C), rd(rec + 0x20)));
    }
    if !ON.load(Ordering::Relaxed) {
        st.was_down = false;
        st.down_at = None;
        release(base, st, kh1_paused);
        return;
    }
    let down = R2_HELD.load(Ordering::Relaxed) || period_down();
    let now = Instant::now();
    let pressed = down && !st.was_down;
    if pressed {
        st.down_at = Some(now);
    } else if !down {
        st.down_at = None;
    }
    st.was_down = down;
    let fast = st.down_at.is_some_and(|t| now.duration_since(t) >= FAST_AFTER);
    if kh1_paused {
        // KH1's pause menu (or a menu that stops the field the same way) is up: it holds the game
        // itself; ours waits (our held tasks stay held).
        st.kh1_paused = st.held;
        return;
    }
    let after_kh1 = std::mem::take(&mut st.kh1_paused);
    if pressed || fast {
        if st.held {
            // Out of the hold: the stepped frame's presses are against the last frame that ran.
            if let Some((held, t1, t2)) = st.pad {
                wr::<u32>(rec + 4, held);
                wr::<u32>(rec + 0x1C, t1);
                wr::<u32>(rec + 0x20, t2);
            }
            release(base, st, false);
        }
    } else {
        // (Every frame: tasks added meanwhile are held too.)
        hold_tasks(base);
        if !st.held {
            st.held = true;
            pause_effects(base, true);
            HELD_SINCE_MS.store(wall_ms() + 1, Ordering::Relaxed);
        } else if after_kh1 {
            // (KH1's pause gave the effects back as it closed.)
            pause_effects(base, true);
        }
        // Select + Triangle writes the trace while the game is held (no rows come then).
        super::trace::frozen_combo(super::takeover::RAW_BUTTONS.load(Ordering::Relaxed) as u16);
    }
}
