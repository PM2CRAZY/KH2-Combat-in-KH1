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
//! - **The picture is drawn live (the tester, after t447: "the actual pause seems to hold the image
//!   in the frame buffer so I wouldn't be using it, focus on getting the live version working").**
//!   KH1's battle pause shows a captured picture of the frame it stopped on (its task at 190020,
//!   `0x14017FD60`, and the capture `0x14018B370`); t445's copy of it is gone, and with it the Held
//!   Picture row: every draw runs each held frame. As t426, only the tasks before priority 140000
//!   are held (and two known update tasks after it, the delayed plays 145001 `0x140290A50` and an
//!   effect fade 159999 `0x1403970D0`), and 120000 (the map) and 130000 (the models) run. What of
//!   those draws steps the game is held natively where KH1 has a way: the effects by KH1's system
//!   stop (`0x142867368` bit 0 and `0x142432130`, which the effects' held flag `0x142E9D7E0`
//!   follows: every effect program's step stops, the effect pass still draws; the HUD fades
//!   `0x14027B870` & co. stop too), not by the effect pause `0x1401ED6D0` (which also hides them:
//!   `+0xDB`); the shape trails KeShpTail3 / 3X (KH1's programs 156 / 157), whose draws step their
//!   animation time on every draw, get the step taken back (their draw slots in KH1's table point
//!   at ours while held); 120000's own steps (the sky's turn `0x14233E174`, the map fades
//!   `0x142356370` and their done bits `0x14232DE00`) are put back after it on held frames, and its
//!   sub-lists stay held. What isn't covered: the notes.
//! - **Sound, as KH1's pause holds it.** KH1's pause machine calls its sound pause (`0x1400D2F80`, a
//!   jump to `0x1400DE010 (mode)`) with 1 as it stops (state 3: playing sound effects paused, the
//!   streamed music at half its volume) and 0 as it goes (state 4: resumed, the volume back). Frame
//!   Advance makes the same calls as a hold begins and ends (a step plays one frame's worth of
//!   sound), unless the debug menu's Held Sound row is Plays. KH1's own pause makes its own calls
//!   meanwhile, and ours is made again after it.

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
/// KH1's sound pause (Steam `0x1400D2F80 (mode)`: 1 pauses, 0 resumes).
const SOUND_PAUSE_STEAM: usize = 0xD2F80;
/// The Held Sound row: sound paused while held (KH1's pause's way); we paused it.
static HOLD_SOUND: AtomicBool = AtomicBool::new(true);
static SOUND_PAUSED: AtomicBool = AtomicBool::new(false);
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
    /// KH1's pause was up during our hold (it gave the effects and the sound back as it closed).
    kh1_paused: bool,
}

/// The tasks held now: (task, its own function, its list: 0 the main one, else the sub-list's RVA).
static HELD_TASKS: Mutex<Vec<(usize, usize, usize)>> = Mutex::new(Vec::new());

/// (t426) 120000's own task lists (Steam; run by it with mask 0, each task one animated part of the
/// map: `0x1401829B0` / `0x1401836E0` make them, `0x1401824C0` / `0x140183460` their steps: each moves
/// its part's animation time on by the frame scale, Checked (decomp)). Held while the game is held
/// (held with 120000 too), each a row of the Held Tasks page.
const SUB_LISTS: [(usize, &str); 2] = [(0x233E420, "map parts A"), (0x233E7D0, "map parts B")];

/// The first of KH1's draw passes: tasks from here on run while held, but these.
const FIRST_DRAW: i32 = 140000;
const LIVE_HOLD: [usize; 2] = [0x290A50, 0x3970D0];
/// Tasks that run while held by default: the models (130000) and the map (120000), the tester's
/// t399 test.
const MODELS_RVA: usize = 0x2A23E0;
const MAP_RVA: usize = 0xD2830;
/// 120000's own steps, put back after it on held frames: the sky's turn (f32), the map fades
/// (0x20 x (timer, alpha, rate), `0x14019BE70`), their done bits (u32).
const SKY_TURN_STEAM: usize = 0x233E174;
const MAP_FADES_STEAM: usize = 0x2356370;
const MAP_FADES_LEN: usize = 0x20 * 0xC;
const MAP_FADES_DONE_STEAM: usize = 0x232DE00;
/// KH1's system stop: the status word's bit 0 and the effects' stop `0x142432130` (which task
/// 140000 copies into the effects' held flag `0x142E9D7E0`).
const SYSTEM_STATUS_STEAM: usize = 0x2867368;
const SYSTEM_STOP_FX_STEAM: usize = 0x2432130;
/// KH1's program table (0x50 an entry, the draw at `+0x18`) and its KeShpTail3 / 3X draws.
const PPP_TABLE_STEAM: usize = 0x54FF00;
const TAIL3_DRAW_STEAM: usize = 0x337450;
const TAIL3X_DRAW_STEAM: usize = 0x338160;
/// What the system stop words were before our hold.
static STOP_SAVED: Mutex<Option<(u32, u32)>> = Mutex::new(None);
static TAILS_ON: AtomicBool = AtomicBool::new(false);

/// What the main list's tasks are (Steam RVAs), for the Held Tasks page and the log: the tester's
/// Held Tasks tests (t399, t426: Checked (in game)) and the decomp (`docs/kh1-frame-advance.md`).
const LABELS: [(usize, &str); 15] = [
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
/// (The tester's switches: function RVA, runs; a task not switched takes the mode's default.)
static RUNS: Mutex<Vec<(usize, bool)>> = Mutex::new(Vec::new());

/// Whether a task runs while held when the page hasn't switched it.
fn runs_by_default(rva: usize) -> bool {
    rva == MODELS_RVA || rva == MAP_RVA
}

fn runs_now(overrides: &[(usize, bool)], rva: usize) -> bool {
    overrides.iter().find(|o| o.0 == rva).map_or(runs_by_default(rva), |o| o.1)
}

/// The Held Tasks page's rows.
pub fn tasks() -> Vec<TaskRow> {
    TASKS.lock().map(|t| t.clone()).unwrap_or_default()
}

/// The Held Tasks page: row `i` held <-> runs while the game is held (a debugging tool: the
/// tester's t399 and t426 tests). Applied at this frame's end (`hold_tasks`).
pub fn toggle_task(i: usize) {
    let Some(row) = TASKS.lock().ok().and_then(|t| t.get(i).copied()) else { return };
    let runs = !row.runs;
    if let Ok(mut r) = RUNS.lock() {
        r.retain(|x| x.0 != row.rva);
        r.push((row.rva, runs));
    }
    if let Ok(mut t) = TASKS.lock() {
        if let Some(x) = t.get_mut(i) {
            x.runs = runs;
        }
    }
    let list = RUNS.lock().map(|r| r.iter().map(|x| format!("{:#x} {}", x.0, if x.1 { "runs" } else { "held" })).collect::<Vec<_>>().join(", ")).unwrap_or_default();
    log(&format!("frame advance: task {} {:#x} {} while held (switched: [{list}])", row.priority, row.rva, if runs { "runs" } else { "held" }));
}

/// A held task's function: does nothing, keeps the task.
unsafe extern "system" fn skip(_task: usize) -> u32 {
    0
}

/// (Live) 120000 (`0x1400D2830`, the map draw) on a held frame: KH1's own, then what it stepped put
/// back (the sky's turn, the map fades and their done bits; its draws read them before stepping).
unsafe extern "system" fn map_live(task: usize) -> u32 {
    let base = BASE.load(Ordering::Relaxed);
    let sky: u32 = rd(base + SKY_TURN_STEAM);
    let done: u32 = rd(base + MAP_FADES_DONE_STEAM);
    let mut fades = [0u8; MAP_FADES_LEN];
    std::ptr::copy_nonoverlapping((base + MAP_FADES_STEAM) as *const u8, fades.as_mut_ptr(), MAP_FADES_LEN);
    let f: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(base + MAP_RVA);
    let r = f(task);
    wr::<u32>(base + SKY_TURN_STEAM, sky);
    wr::<u32>(base + MAP_FADES_DONE_STEAM, done);
    std::ptr::copy_nonoverlapping(fades.as_ptr(), (base + MAP_FADES_STEAM) as *mut u8, MAP_FADES_LEN);
    r
}

/// (Live) KH1's system stop on (saved first) or put back as it was.
unsafe fn system_stop(base: usize, on: bool) {
    let Ok(mut saved) = STOP_SAVED.lock() else { return };
    let status = base + SYSTEM_STATUS_STEAM;
    let fx = base + SYSTEM_STOP_FX_STEAM;
    if on {
        if saved.is_none() {
            *saved = Some((rd::<u32>(status) & 1, rd::<u32>(fx)));
        }
        // (Every held frame: KH1's own toggle, task 30000, could have taken it off.)
        wr::<u32>(status, rd::<u32>(status) | 1);
        wr::<u32>(fx, 1);
    } else if let Some((bit, was)) = saved.take() {
        wr::<u32>(status, rd::<u32>(status) & !1 | bit);
        wr::<u32>(fx, was);
    }
}

/// (Live) KeShpTail3's and KeShpTail3X's draw slots (programs 156 and 157 of KH1's table) at ours
/// or back at KH1's (only when they hold KH1's own).
unsafe fn tails(base: usize, on: bool) {
    if TAILS_ON.swap(on, Ordering::Relaxed) == on {
        return;
    }
    for (i, kh1, ours) in [(156usize, TAIL3_DRAW_STEAM, tail3_live as *const () as usize), (157, TAIL3X_DRAW_STEAM, tail3x_live as *const () as usize)] {
        let slot = base + PPP_TABLE_STEAM + i * 0x50 + 0x18;
        let (from, to) = if on { (base + kh1, ours) } else { (ours, base + kh1) };
        if rd::<usize>(slot) == from {
            wr::<usize>(slot, to);
        }
    }
}

/// The trail time step KH1's KeShpTail3 / 3X draws make on every draw with a model (work 0
/// `+0x1F0` += the key's `+8`, Checked (decomp) `0x140337450`, `0x140338160`), taken back.
unsafe fn tail_back(obj: usize, key: usize, prog: usize) {
    if rd::<i32>(key + 4) != 0xFFFF {
        let list = (prog as isize + rd::<i32>(prog + 0xC) as isize) as usize;
        let t = (obj as isize + 0x290 + rd::<i32>(list) as isize) as usize;
        wr::<i32>(t, rd::<i32>(t).wrapping_sub(rd::<i32>(key + 8)));
    }
}

unsafe extern "system" fn tail3_live(obj: usize, key: usize, prog: usize) {
    let f: unsafe extern "system" fn(usize, usize, usize) = std::mem::transmute(BASE.load(Ordering::Relaxed) + TAIL3_DRAW_STEAM);
    f(obj, key, prog);
    tail_back(obj, key, prog);
}

unsafe extern "system" fn tail3x_live(obj: usize, key: usize, prog: usize) {
    let f: unsafe extern "system" fn(usize, usize, usize) = std::mem::transmute(BASE.load(Ordering::Relaxed) + TAIL3X_DRAW_STEAM);
    f(obj, key, prog);
    tail_back(obj, key, prog);
}

/// The tasks of the main list not flagged for a pause (those KH1's pause holds, mask 1) before
/// 140000, and `LIVE_HOLD`, are held (the function swapped for `skip`); tasks added since the last
/// call too. 120000 runs through `map_live`. Tasks the page lets run get their own function back.
unsafe fn hold_tasks(base: usize) {
    let Ok(mut held) = HELD_TASKS.lock() else { return };
    let skip_f = skip as *const () as usize;
    let map_f = map_live as *const () as usize;
    let ours = |f: usize| f == skip_f || f == map_f;
    let mut t: usize = rd(base + TASK_LIST_STEAM + 0x10);
    let mut n = 0;
    let overrides = RUNS.lock().map(|r| r.clone()).unwrap_or_default();
    let mut seen: Vec<TaskRow> = Vec::new();
    while t > 0x10000 && n < 1024 && super::crash::readable(t + 0x18) {
        let f: usize = rd(t + 0x10);
        let next: usize = rd(t + 8);
        if rd::<u16>(t) != 0 && rd::<u16>(t + 2) & PAUSE_FLAG == 0 && f > 0x10000 {
            // (Ours already: the task's own function is in `held`.)
            let own = if ours(f) { held.iter().find(|e| e.0 == t).map_or(0, |e| e.1) } else { f };
            let rva = own.wrapping_sub(base);
            let prio = rd::<i32>(t + 4);
            if own == 0 {
            } else if prio >= FIRST_DRAW && !LIVE_HOLD.contains(&rva) {
                // (A draw: not held; given back if it was.)
                if ours(f) {
                    wr::<usize>(t + 0x10, own);
                    held.retain(|e| e.0 != t);
                }
            } else {
                let runs = runs_now(&overrides, rva);
                if !seen.iter().any(|s| s.rva == rva) {
                    seen.push(TaskRow { priority: prio, label: label(rva), sub: false, rva, runs });
                }
                let want = if !runs { skip_f } else if rva == MAP_RVA { map_f } else { own };
                if f != want {
                    if want == own {
                        wr::<usize>(t + 0x10, own);
                        held.retain(|e| e.0 != t);
                    } else {
                        // (A task freed and made again meanwhile: its new function is the one to give back.)
                        match held.iter_mut().find(|e| e.0 == t) {
                            Some(e) => e.1 = own,
                            None => held.push((t, own, 0)),
                        }
                        wr::<usize>(t + 0x10, want);
                    }
                }
            }
        }
        t = next;
        n += 1;
    }
    // (t426) 120000's own lists: every task in them, unless the page lets that list run.
    for (list, label) in SUB_LISTS {
        let runs = runs_now(&overrides, list);
        seen.push(TaskRow { priority: 120000, label, sub: true, rva: list, runs });
        let mut t: usize = rd(base + list + 0x10);
        let mut n = 0;
        while t > 0x10000 && n < 1024 && super::crash::readable(t + 0x18) {
            let f: usize = rd(t + 0x10);
            if rd::<u16>(t) != 0 && f > 0x10000 {
                if runs {
                    if f == skip_f {
                        if let Some(own) = held.iter().find(|e| e.0 == t).map(|e| e.1) {
                            wr::<usize>(t + 0x10, own);
                        }
                        held.retain(|e| e.0 != t);
                    }
                } else if f != skip_f {
                    match held.iter_mut().find(|e| e.0 == t) {
                        Some(e) => e.1 = f,
                        None => held.push((t, f, list)),
                    }
                    wr::<usize>(t + 0x10, skip_f);
                }
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
        log(&format!("frame advance: holding {} tasks (priority, function RVA): {}", held.len(), list.join(", ")));
    }
}

/// The held tasks get their functions back (those still in use and still ours).
unsafe fn release_tasks() {
    let Ok(mut held) = HELD_TASKS.lock() else { return };
    let (skip_f, map_f) = (skip as *const () as usize, map_live as *const () as usize);
    for (t, f, _) in held.drain(..) {
        let now = if super::crash::readable(t + 0x18) && rd::<u16>(t) != 0 { rd::<usize>(t + 0x10) } else { 0 };
        if now == skip_f || now == map_f {
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

/// KH1's sound pause (`0x1400D2F80`): on pauses (once), off resumes what we paused.
unsafe fn pause_sound(base: usize, on: bool) {
    if SOUND_PAUSED.swap(on, Ordering::Relaxed) == on {
        return;
    }
    let f: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_PAUSE_STEAM);
    f(on as i32);
}

/// The debug menu's Held Sound row: Paused (KH1's pause's way) or Plays.
pub fn hold_sound() -> bool {
    HOLD_SOUND.load(Ordering::Relaxed)
}

/// Switches the Held Sound row (applied at the next frame end).
pub fn set_hold_sound(on: bool) {
    if HOLD_SOUND.swap(on, Ordering::Relaxed) != on {
        log(&format!("debug menu: frame advance held sound {}", if on { "paused (KH1's pause: 0x1400D2F80 (1), music at half volume)" } else { "plays" }));
    }
}

/// The draws' side of a hold: KH1's system stop (the effects held, still drawn), the trails'
/// steps taken back.
unsafe fn hold_draws(base: usize) {
    system_stop(base, true);
    tails(base, true);
}

unsafe fn release_draws(base: usize) {
    system_stop(base, false);
    tails(base, false);
}

/// Ends our hold: the next frame runs.
unsafe fn release(base: usize, st: &mut State, kh1_paused: bool) {
    if !st.held {
        return;
    }
    st.held = false;
    release_tasks();
    release_draws(base);
    // (While KH1's pause is up it holds the sound itself, and gives it back as it closes.)
    if !kh1_paused {
        pause_sound(base, false);
    } else {
        SOUND_PAUSED.store(false, Ordering::Relaxed);
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
            HELD_SINCE_MS.store(wall_ms() + 1, Ordering::Relaxed);
            hold_draws(base);
        }
        // (Every held frame: KH1's own toggle, task 30000, could have taken it off.)
        system_stop(base, true);
        if after_kh1 {
            // (KH1's pause gave the sound back as it closed; its effect pause it gave back too.)
            SOUND_PAUSED.store(false, Ordering::Relaxed);
        }
        pause_sound(base, HOLD_SOUND.load(Ordering::Relaxed));
        // Select + Triangle writes the trace while the game is held (no rows come then).
        super::trace::frozen_combo(super::takeover::RAW_BUTTONS.load(Ordering::Relaxed) as u16);
    }
}
