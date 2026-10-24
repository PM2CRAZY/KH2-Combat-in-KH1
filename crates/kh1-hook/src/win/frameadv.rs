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
//!   runs: the same tasks KH1's pause holds, held whatever the machine writes. (t378 to t426 held
//!   only those before priority 140000, as the picture went black; KH1's pause picture, below,
//!   is what the picture needed.) From KH1's frame
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
//! - **KH1's pause picture (the tester's t426 test: effects, magic and Keyblade trails not drawn
//!   on held frames).** KH1's battle pause (the exe, the pause machine's jump table) holds every
//!   task not flagged 1, hides and pauses the effects (`0x1401ED6D0 (1)`: `+0xD5` paused, `+0xDB`
//!   not drawn), and shows the frame it stopped on: its task at 190020 (`0x14017FD60`) captures the
//!   last frame (`0x14018B370`) and draws it (`0x140183CD0 (brightness, spread)`), and its task at
//!   10499 (`0x14017EF90`) draws it again every paused frame (spread 0x10 / 0x20: the pause's blur
//!   layers). Frame Advance now does the same: every task not flagged 1 is held, whatever its
//!   priority (as t372, and as KH1's pause), and a task of ours at 190020 (flags 1, so KH1's own
//!   rule keeps it running) captures the last frame that ran once per hold and draws it every held
//!   frame with spread 0 (the layers on top of each other: the frame as it was, no blur). So the
//!   picture is exactly the last frame that ran (map, models, effects, trails, HUD, lock-on,
//!   command menu, damage numbers) and nothing that frame's tasks step goes on. While KH1's own
//!   pause is up ours doesn't draw (KH1's picture and P A U S E stay).

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
/// The held tasks are written to the log once (priority, function's RVA).
static LOGGED_TASKS: AtomicBool = AtomicBool::new(false);
/// KH1's task registration on the main list (Steam `0x140284670 (priority, function) -> task`).
const REGISTER_TASK_STEAM: usize = 0x284670;
/// KH1's pause picture: the capture of the last frame (Steam `0x14018B370`, a jump to `0x14018B5D0`
/// with its record `0x1404DE790`) and its draw (Steam `0x140183CD0 (brightness, spread)`: 0x80 is
/// full brightness; the pause draws it with spread 0x10, then 0x20).
const PICTURE_CAPTURE_STEAM: usize = 0x18B370;
const PICTURE_DRAW_STEAM: usize = 0x183CD0;
/// The priority of KH1's pause task that captures and draws the picture (`0x14017FD60`, 190020).
const PICTURE_PRIORITY: u32 = 190020;
/// Our picture task (0: none); a capture is due (a hold began); the picture is wanted.
static PICTURE_TASK: AtomicUsize = AtomicUsize::new(0);
static CAPTURE_DUE: AtomicBool = AtomicBool::new(false);
static PICTURE_ON: AtomicBool = AtomicBool::new(false);
static LOGGED_PICTURE: AtomicBool = AtomicBool::new(false);
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

/// The tasks held now: (task, its own function, its list: 0 the main one, else the sub-list's RVA).
static HELD_TASKS: Mutex<Vec<(usize, usize, usize)>> = Mutex::new(Vec::new());

/// (t426) 120000's own task lists (Steam; run by it with mask 0, each task one animated part of the
/// map: `0x1401829B0` / `0x1401836E0` make them, `0x1401824C0` / `0x140183460` their steps: each moves
/// its part's animation time on by the frame scale, Checked (decomp)). Held while the game is held
/// (held with 120000 too), each a row of the Held Tasks page.
const SUB_LISTS: [(usize, &str); 2] = [(0x233E420, "map parts A"), (0x233E7D0, "map parts B")];

/// What the main list's tasks are (Steam RVAs), for the Held Tasks page and the log: the tester's
/// Held Tasks tests (t399, t426: Checked (in game)) and the decomp (`docs/kh1-frame-advance.md`).
const LABELS: [(usize, &str); 17] = [
    (0x1E1900, "fades"),
    (0x28D620, "system"),
    (0x2937F0, "command menu"),
    (0x29E790, "lights"),
    (0x298940, "actors"),
    (0x2B4460, "hit checks"),
    (0x1E5D90, "camera"),
    (0x28CD50, "camera 2"),
    (0xD2830, "map draw"),
    (0x2A23E0, "models"),
    (0x28CC10, "camera copy"),
    (0x2AEDA0, "fx binding"),
    (0x2B11B0, "lock-on"),
    (0x290A50, "delayed play"),
    (0x3970D0, "fx fade"),
    (0x2E26D0, "160000 pass"),
    (0x38D9B0, "2D / menus"),
];

fn label(rva: usize) -> &'static str {
    LABELS.iter().find(|l| l.0 == rva).map_or("", |l| l.1)
}

/// (t399) One of the tasks Frame Advance would hold, for the debug menu's Held Tasks page.
#[derive(Clone, Copy)]
pub struct TaskRow {
    pub priority: i32,
    /// What it is (a sub-list's name, or a known task's), or "".
    pub label: &'static str,
    /// A sub-list of 120000's (its RVA in `rva`), not a task of the main list.
    pub sub: bool,
    /// Its function's RVA in the exe (a sub-list's own RVA).
    pub rva: usize,
    /// The tester let it run while the game is held.
    pub runs: bool,
}

/// The tasks seen the last time the game was held (priority order, as KH1's list keeps them).
static TASKS: Mutex<Vec<TaskRow>> = Mutex::new(Vec::new());
/// (t399) The functions (RVAs) the tester lets run while held (the Held Tasks page; this session).
static RUNS: Mutex<Vec<usize>> = Mutex::new(Vec::new());

/// The Held Tasks page's rows.
pub fn tasks() -> Vec<TaskRow> {
    TASKS.lock().map(|t| t.clone()).unwrap_or_default()
}

/// The Held Tasks page: row `i` held <-> runs while the game is held (to find a task the
/// picture needs: the tester's t378 test, still black). A task let run gets its function back at
/// once; one held again is held from the next held frame.
pub fn toggle_task(i: usize) {
    let Some(row) = TASKS.lock().ok().and_then(|t| t.get(i).copied()) else { return };
    let runs = !row.runs;
    if let Ok(mut r) = RUNS.lock() {
        r.retain(|&x| x != row.rva);
        if runs {
            r.push(row.rva);
        }
    }
    if let Ok(mut t) = TASKS.lock() {
        if let Some(x) = t.get_mut(i) {
            x.runs = runs;
        }
    }
    if runs {
        let base = BASE.load(Ordering::Relaxed);
        if let Ok(mut held) = HELD_TASKS.lock() {
            let ours = skip as *const () as usize;
            held.retain(|&(t, f, list)| {
                let this = if !row.sub { list == 0 && f.wrapping_sub(base) == row.rva } else { list == row.rva };
                if !this {
                    return true;
                }
                unsafe {
                    if super::crash::readable(t + 0x18) && rd::<u16>(t) != 0 && rd::<usize>(t + 0x10) == ours {
                        wr::<usize>(t + 0x10, f);
                    }
                }
                false
            });
        }
    }
    let list = RUNS.lock().map(|r| r.iter().map(|x| format!("{x:#x}")).collect::<Vec<_>>().join(", ")).unwrap_or_default();
    log(&format!("frame advance: task {} {:#x} {} while held (running while held: [{list}])", row.priority, row.rva, if runs { "runs" } else { "held" }));
}

/// A held task's function: does nothing, keeps the task.
unsafe extern "system" fn skip(_task: usize) -> u32 {
    0
}

/// Every task of the main list not flagged for a pause is held (its function swapped for
/// `skip`), whatever its priority, as KH1's pause holds them (mask 1); tasks added since the
/// last call too.
unsafe fn hold_tasks(base: usize) {
    let Ok(mut held) = HELD_TASKS.lock() else { return };
    let ours = skip as *const () as usize;
    let mut t: usize = rd(base + TASK_LIST_STEAM + 0x10);
    let mut n = 0;
    let runs = RUNS.lock().map(|r| r.clone()).unwrap_or_default();
    let mut seen: Vec<TaskRow> = Vec::new();
    while t > 0x10000 && n < 1024 && super::crash::readable(t + 0x18) {
        let f: usize = rd(t + 0x10);
        let candidate = rd::<u16>(t) != 0 && rd::<u16>(t + 2) & PAUSE_FLAG == 0 && f > 0x10000;
        if candidate {
            // (Ours already: the task's own function is in `held`.)
            let own = if f == ours { held.iter().find(|e| e.0 == t).map_or(0, |e| e.1) } else { f };
            let rva = own.wrapping_sub(base);
            if own != 0 && !seen.iter().any(|s| s.rva == rva) {
                seen.push(TaskRow { priority: rd::<i32>(t + 4), label: label(rva), sub: false, rva, runs: runs.contains(&rva) });
            }
        }
        if candidate && f != ours && !runs.contains(&f.wrapping_sub(base)) {
            // (A task freed and made again meanwhile: its new function is the one to give back.)
            match held.iter_mut().find(|e| e.0 == t) {
                Some(e) => e.1 = f,
                None => held.push((t, f, 0)),
            }
            wr::<usize>(t + 0x10, ours);
        }
        t = rd(t + 8);
        n += 1;
    }
    // (t426) 120000's own lists: every task in them, unless the page lets that list run.
    for (list, label) in SUB_LISTS {
        seen.push(TaskRow { priority: 120000, label, sub: true, rva: list, runs: runs.contains(&list) });
        if runs.contains(&list) {
            continue;
        }
        let mut t: usize = rd(base + list + 0x10);
        let mut n = 0;
        while t > 0x10000 && n < 1024 && super::crash::readable(t + 0x18) {
            let f: usize = rd(t + 0x10);
            if rd::<u16>(t) != 0 && f != ours && f > 0x10000 {
                match held.iter_mut().find(|e| e.0 == t) {
                    Some(e) => e.1 = f,
                    None => held.push((t, f, list)),
                }
                wr::<usize>(t + 0x10, ours);
            }
            t = rd(t + 8);
            n += 1;
        }
    }
    if let Ok(mut tl) = TASKS.lock() {
        *tl = seen;
    }
    if !held.is_empty() && !LOGGED_TASKS.swap(true, Ordering::Relaxed) {
        let list: Vec<String> = held
            .iter()
            .map(|&(t, f, l)| {
                let rva = f.wrapping_sub(base);
                if l != 0 {
                    format!("sub-list {l:#x} {rva:#x}")
                } else if label(rva).is_empty() {
                    format!("{} {rva:#x}", rd::<i32>(t + 4))
                } else {
                    format!("{} {rva:#x} ({})", rd::<i32>(t + 4), label(rva))
                }
            })
            .collect();
        log(&format!("frame advance: holding {} tasks, as KH1's pause (priority, function RVA): {}", held.len(), list.join(", ")));
    }
}

/// (KH1's pause picture) Our task at 190020, flagged 1 as KH1's pause task is: while the game is
/// held, the last frame that ran captured once (`0x14018B370`, as KH1's pause task `0x14017FD60`
/// does) and drawn every held frame (`0x140183CD0 (0x80, 0)`: full brightness, no blur). Not while
/// KH1's own pause is up (its picture and P A U S E show). Removed (4) once the game runs.
unsafe extern "system" fn picture_task(_task: usize) -> u32 {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 || !PICTURE_ON.load(Ordering::Relaxed) {
        PICTURE_TASK.store(0, Ordering::Relaxed);
        return 4;
    }
    if rd::<i32>(base + PAUSE_STATE_RVA) != 0 {
        return 0;
    }
    if CAPTURE_DUE.swap(false, Ordering::Relaxed) {
        let capture: unsafe extern "system" fn() = std::mem::transmute(base + PICTURE_CAPTURE_STEAM);
        capture();
    }
    let draw: unsafe extern "system" fn(u32, u32) = std::mem::transmute(base + PICTURE_DRAW_STEAM);
    draw(0x80, 0);
    0
}

/// Our picture task is in KH1's list (made again if KH1's list lost it).
unsafe fn ensure_picture(base: usize) {
    let ours = picture_task as *const () as usize;
    let t = PICTURE_TASK.load(Ordering::Relaxed);
    if t != 0 && super::crash::readable(t + 0x18) && rd::<u16>(t) != 0 && rd::<usize>(t + 0x10) == ours {
        return;
    }
    let register: unsafe extern "system" fn(u32, usize) -> usize = std::mem::transmute(base + REGISTER_TASK_STEAM);
    let t = register(PICTURE_PRIORITY, ours);
    if t == 0 || !super::crash::readable(t + 0x18) {
        PICTURE_TASK.store(0, Ordering::Relaxed);
        if !LOGGED_PICTURE.swap(true, Ordering::Relaxed) {
            log("frame advance: KH1's task list is full; the held frames have no picture");
        }
        return;
    }
    wr::<u16>(t + 2, PAUSE_FLAG);
    PICTURE_TASK.store(t, Ordering::Relaxed);
    if !LOGGED_PICTURE.swap(true, Ordering::Relaxed) {
        log("frame advance: held frames show KH1's pause picture (task 190020: capture 0x14018B370, draw 0x140183CD0 (0x80, 0))");
    }
}

/// The held tasks get their functions back (those still in use and still ours).
unsafe fn release_tasks() {
    let Ok(mut held) = HELD_TASKS.lock() else { return };
    let ours = skip as *const () as usize;
    for (t, f, _) in held.drain(..) {
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
    // (Our picture task takes itself off the list the next frame.)
    PICTURE_ON.store(false, Ordering::Relaxed);
    CAPTURE_DUE.store(false, Ordering::Relaxed);
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
            // The frame that just ran is the picture of the hold.
            CAPTURE_DUE.store(true, Ordering::Relaxed);
            PICTURE_ON.store(true, Ordering::Relaxed);
        }
        ensure_picture(base);
        if after_kh1 {
            // (KH1's pause gave the effects back as it closed.)
            pause_effects(base, true);
        }
        // Select + Triangle writes the trace while the game is held (no rows come then).
        super::trace::frozen_combo(super::takeover::RAW_BUTTONS.load(Ordering::Relaxed) as u16);
    }
}
