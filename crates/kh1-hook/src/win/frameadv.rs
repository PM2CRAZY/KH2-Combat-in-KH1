//! Frame Advance (t371, the tester Mr. Bugs, `docs/tester-requests.md`): a debug menu row that
//! holds the game still; R2 or the keyboard's period key plays one game frame, and held for a
//! second plays at full speed until let go. Notes and addresses: `docs/kh1-frame-advance.md`.
//!
//! - **Holding the game, KH1's own way.** KH1's field loop (Steam `0x140170D50`) runs its task
//!   list once a frame (`0x140284690` -> `0x14028A710 (list 0x142867380, mask)`): a task runs only
//!   when its flags hold every bit of the mask `0x142867370`. KH1's pause sets the mask to 1, so
//!   only the tasks flagged for a pause run (the draws and the pause menu's own), and marks every
//!   model object paused (`0x1401ED6D0 (1)`, the pause machine's call as the pause menu opens; 0
//!   puts them back). Frame Advance does the same between frames, from KH1's frame end
//!   (`debugmenu::frame_end`, after the frame's task list): the next frame runs (mask 0) or is
//!   held (mask 1). The field loop's own work around the list (the pad, the pause machine, the
//!   drawing, the pacing, the present) runs every frame, so the picture, the HUD and the debug
//!   menu stay up. Nothing of KH1's is hooked for it.
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
//!   window is in front. (Not done: hiding the period key from KH1's own keyboard reading, which
//!   needs a hook in KH1's key mapping, `docs/kh1-frame-advance.md`.)
//! - **The mod's clock** (`clock_ms`) stops while the game is held, so the gaps and windows the
//!   mod times in real time (a pause seen, KH2 active, the trace's frame gaps) read a stepped
//!   frame as one played at full speed; `age` does the same for the draw's "posed lately" checks
//!   (KH2's model, its pose, the hands, the left Keyblade), which would otherwise go stale and
//!   hand Sora back to KH1's model while the game is held.
//! - **Steam only**: the task mask and the model pause call are Steam addresses, with no Epic
//!   counterpart checked (the row says so on another build).

use super::addr::{Build, Rva};
use super::log;
use super::sys::*;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// KH1's task mask (Steam `0x142867370`, u32): the field loop's task list runs the tasks whose
/// flags hold all of it (0: all; 1: KH1's pause).
const TASK_MASK_STEAM: usize = 0x2867370;
/// KH1's model objects paused or not (Steam `0x1401ED6D0 (on)`).
const MODEL_PAUSE_STEAM: usize = 0x1ED6D0;
/// KH1's pause machine (0 while the game runs): while it is up KH1 owns the mask.
const PAUSE_STATE_RVA: Rva = Rva(0x232E9B4);
/// KH1's pad record (held `+4`, the repeat timers `+0x1C` / `+0x20`).
const BUTTONS_RVA: Rva = Rva(0x298D4A0);
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

unsafe fn pause_models(base: usize, on: bool) {
    let f: unsafe extern "system" fn(i32) = std::mem::transmute(base + MODEL_PAUSE_STEAM);
    f(on as i32);
}

/// Ends our hold: the next frame runs.
unsafe fn release(base: usize, st: &mut State, kh1_paused: bool) {
    if !st.held {
        return;
    }
    st.held = false;
    // (While KH1's pause is up its machine owns the mask and the models: it gives them back when
    // it closes.)
    if !kh1_paused {
        wr::<u32>(base + TASK_MASK_STEAM, 0);
        pause_models(base, false);
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
        // itself; ours waits.
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
        // (Every frame: KH1's pause machine puts the mask back to 0 as its menu closes.)
        wr::<u32>(base + TASK_MASK_STEAM, 1);
        if !st.held {
            st.held = true;
            pause_models(base, true);
            HELD_SINCE_MS.store(wall_ms() + 1, Ordering::Relaxed);
        } else if after_kh1 {
            pause_models(base, true);
        }
        // Select + Triangle writes the trace while the game is held (no rows come then).
        super::trace::frozen_combo(super::takeover::RAW_BUTTONS.load(Ordering::Relaxed) as u16);
    }
}
