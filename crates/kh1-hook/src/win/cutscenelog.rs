//! What KH1 does for a cutscene, written to the log only (the user: "add an auto cutscene skipper
//! in qol", then "make sure the log is toggled in the debug menu so we don't bloat performance").
//! The first step toward it, no behaviour change. `docs/kh1-cutscene-log.md`.
//!
//! Debug tools only (`debug_on`), Steam only, and every line is under the Cutscenes log category
//! (the debug menu's Logs page, `log_cutscenes`, off by default): a category that is off costs one
//! atomic load in each hook and nothing else. Three hooks, each checking its function's first bytes:
//!
//! - KH1's scene prepare (Steam `0x14017CB10 (kind, sub, flag)`, Checked (decomp)): the arguments, KH1's
//!   event flag (`0x14298C9A0`: a kind 9 scene, or kind 0x10 sub 0x15, is running) and its scene triple.
//! - KH1's event script starter (`0x1401A38A0 (script file, entry)`, Checked (decomp); t706 took its first
//!   argument for the context, which is why those lines read nothing): a line for each start with the
//!   scene flag words and what the script holds (t708: its sub-scripts, length and distinct 4-byte
//!   instructions, class in the last byte, 24-bit operand).
//! - KH1's movie opener (`0x14011C640 (reader, wide path)`, Checked (decomp)): the file it opens.
//!   Assumption: its second argument is the path and it is the only movie path; the log shows it.

use super::addr::Build;
use super::{hook, log, log_on, LogCat};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

const SCENE_PREPARE_STEAM: usize = 0x17CB10;
/// `mov [rsp+0x18], rsi`.
const SCENE_PREPARE_PROLOGUE: [u8; 5] = [0x48, 0x89, 0x74, 0x24, 0x18];
const RUNNER_STEAM: usize = 0x1A38A0;
/// `mov [rsp+8], rbx`.
const RUNNER_PROLOGUE: [u8; 5] = [0x48, 0x89, 0x5C, 0x24, 0x08];
const MOVIE_STEAM: usize = 0x11C640;
/// `mov rax, rsp; push rbp; push rdi`.
const MOVIE_PROLOGUE: [u8; 5] = [0x48, 0x8B, 0xC4, 0x55, 0x57];

/// KH1's event flag, and its scene triple (Steam RVAs, Checked (decomp)).
const EVENT_FLAG_STEAM: usize = 0x298C9A0;
const SCENE_TRIPLE_STEAM: [usize; 3] = [0x233FE84, 0x233FE8C, 0x233FE90];

/// The script contexts (Steam RVAs, Checked (decomp)): KH1's runner starts entry 0 of a script file
/// in the first and any other entry in the second. A context: its code (4-byte instructions, the
/// class in the last byte and a 24-bit operand), the number of sub-scripts (each ends at class 5),
/// the finished flag and the instruction counter.
const CONTEXT_STEAM: [usize; 2] = [0x2382AD0, 0x23902D0];
const CTX_CODE: usize = 0x80;
const CTX_SUBS: usize = 0x88;
const CTX_DONE: usize = 0x8C;
const CTX_PC: usize = 0x190;
/// KH1's scene flag words (`0x142382590`, `0x142382594`: event mode and its friends).
const FLAGS_STEAM: [usize; 2] = [0x2382590, 0x2382594];
const CLASS_END: u8 = 5;
/// The most instructions looked through for one script, and distinct instructions written.
const SCAN_MAX: usize = 4096;
const WORDS_MAX: usize = 96;

/// Lines written at most (a long session on the log would grow without end).
const MAX_LINES: u32 = 40_000;
static LINES: AtomicU32 = AtomicU32::new(0);

/// The words watched each frame (Steam RVAs, Checked (decomp)): KH1's scene flags (`0x142382590`,
/// `0x142382594`: bit 0 of the second is set by the scripts' cutscene-mode opcodes (their calls of
/// `0x1401A0E60`, 1 at the start of a cutscene and 0 at its end), `0x800` is a talk the player started
/// (the trigger type 0 of `0x1401A1xxx` sets it)), the script runner's own word (`0x1423AB2D0`), the
/// engine word `0x142867364`, the event file in use (`0x142382524`), and the number of script
/// contexts (`0x14237F4E4`). Assumption: what each bit means beyond those; the log is for finding out.
const WATCH_STEAM: [usize; 6] = [0x2382590, 0x2382594, 0x23AB2D0, 0x2867364, 0x2382524, 0x237F4E4];
static LAST_WATCH: [AtomicU32; 7] = [const { AtomicU32::new(0) }; 7];
static WATCH_SET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

static ORIG_SCENE: AtomicUsize = AtomicUsize::new(0);
static ORIG_RUNNER: AtomicUsize = AtomicUsize::new(0);
static ORIG_MOVIE: AtomicUsize = AtomicUsize::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

fn base() -> usize {
    unsafe { super::sys::GetModuleHandleA(std::ptr::null()) as usize }
}

/// A line, while the category is on and under the cap.
fn line(text: String) {
    let n = LINES.fetch_add(1, Ordering::Relaxed);
    if n < MAX_LINES {
        log!(Cutscenes, "{text}");
        if n + 1 == MAX_LINES {
            log!(Cutscenes, "(cutscene log stopped at {MAX_LINES} lines)");
        }
    }
}

unsafe fn state() -> String {
    let b = base();
    let flag = rd::<u32>(b + EVENT_FLAG_STEAM);
    let t = SCENE_TRIPLE_STEAM.map(|o| rd::<i32>(b + o));
    format!("event flag {flag}, scene triple {:#x} {:#x} {:#x}", t[0], t[1], t[2])
}

pub unsafe fn install() {
    if !super::debug_on() || Build::current() != Build::Steam {
        return;
    }
    let b = base();
    let _ = hook::install(b + SCENE_PREPARE_STEAM, &SCENE_PREPARE_PROLOGUE, scene as *const () as usize, &ORIG_SCENE, "scene prepare (cutscene log)");
    let _ = hook::install(b + RUNNER_STEAM, &RUNNER_PROLOGUE, runner as *const () as usize, &ORIG_RUNNER, "event script runner (cutscene log)");
    let _ = hook::install(b + MOVIE_STEAM, &MOVIE_PROLOGUE, movie as *const () as usize, &ORIG_MOVIE, "movie opener (cutscene log)");
}

type Orig4 = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;

unsafe extern "system" fn scene(a: usize, b: usize, c: usize, d: usize) -> usize {
    if log_on(LogCat::Cutscenes) {
        line(format!("scene prepare (kind {:#x}, sub {:#x}, flag {}): {}", a as i32, b as i32, c as u8, state()));
    }
    let f: Orig4 = std::mem::transmute(ORIG_SCENE.load(Ordering::Acquire));
    let r = f(a, b, c, d);
    if log_on(LogCat::Cutscenes) {
        line(format!("scene prepare (kind {:#x}, sub {:#x}) done: {}", a as i32, b as i32, state()));
    }
    r
}

/// What a script holds (its context, after the runner has started it): the sub-scripts, the length
/// and the distinct instructions (class / operand, in the order first seen), so a story scene can be
/// told from a talk by what it is made of.
unsafe fn describe(ctx: usize) -> String {
    use super::crash::readable;
    if !readable(ctx) || !readable(ctx + CTX_PC + 3) {
        return "context unreadable".into();
    }
    let code = rd::<u64>(ctx + CTX_CODE) as usize;
    let subs = rd::<i32>(ctx + CTX_SUBS);
    let done = rd::<u32>(ctx + CTX_DONE);
    let pc = rd::<u32>(ctx + CTX_PC);
    if code < 0x10000 || !(0..=64).contains(&subs) {
        return format!("code {code:#x}, {subs} sub-scripts: not read");
    }
    let (mut ends, mut n) = (0, 0usize);
    let mut words: Vec<u32> = Vec::new();
    while ends < subs && n < SCAN_MAX {
        let at = code + n * 4;
        if !readable(at) || !readable(at + 3) {
            break;
        }
        let w = rd::<u32>(at);
        if (w >> 24) as u8 == CLASS_END {
            ends += 1;
        } else if !words.contains(&w) && words.len() < WORDS_MAX {
            words.push(w);
        }
        n += 1;
    }
    let list: Vec<String> = words.iter().map(|w| format!("{}/{:#x}", w >> 24, w & 0xFF_FFFF)).collect();
    format!("{subs} sub-scripts, {n} instructions, counter {pc}, finished {done}; instructions (class/operand): {}", list.join(" "))
}

unsafe extern "system" fn runner(file: usize, idx: usize, c: usize, d: usize) -> usize {
    let on = log_on(LogCat::Cutscenes);
    let f: Orig4 = std::mem::transmute(ORIG_RUNNER.load(Ordering::Acquire));
    let r = f(file, idx, c, d);
    if on {
        let b = base();
        let slot = if idx as i32 == 0 { 0 } else { 1 };
        let flags = FLAGS_STEAM.map(|o| rd::<u32>(b + o));
        line(format!(
            "script start (file {file:#x}, entry {}): flags {:#x} {:#x}, {}; {}",
            idx as i32,
            flags[0],
            flags[1],
            state(),
            describe(b + CONTEXT_STEAM[slot])
        ));
    }
    r
}

unsafe extern "system" fn movie(reader: usize, path: usize, c: usize, d: usize) -> usize {
    if log_on(LogCat::Cutscenes) {
        let mut text = String::from("?");
        if path != 0 && super::crash::readable(path) {
            let mut w = Vec::new();
            for i in 0..260 {
                if !super::crash::readable(path + i * 2 + 1) {
                    break;
                }
                let ch = rd::<u16>(path + i * 2);
                if ch == 0 {
                    break;
                }
                w.push(ch);
            }
            text = String::from_utf16_lossy(&w);
        }
        line(format!("movie open (reader {reader:#x}): {text}; {}", state()));
    }
    let f: Orig4 = std::mem::transmute(ORIG_MOVIE.load(Ordering::Acquire));
    f(reader, path, c, d)
}

/// Each frame (the debug tools' frame end, `debugmenu::frame_end`), with the Cutscenes log on: a line
/// when any watched word or the pad's buttons (the pad as pressed, `takeover::RAW_BUTTONS`: Start
/// `0x0008`, Select `0x0001`, Cross `0x4000`, Circle `0x2000`, Triangle `0x1000`, Square `0x8000`)
/// changes, so a pause, a Skip Cutscene and each confirm press show what KH1 does.
pub unsafe fn frame(base: usize) {
    if !log_on(LogCat::Cutscenes) {
        WATCH_SET.store(false, Ordering::Relaxed);
        return;
    }
    let mut now = [0u32; 7];
    for (i, o) in WATCH_STEAM.iter().enumerate() {
        now[i] = rd::<u32>(base + o);
    }
    now[6] = super::takeover::RAW_BUTTONS.load(Ordering::Relaxed) & 0xFFFF;
    let first = !WATCH_SET.swap(true, Ordering::Relaxed);
    let mut changed = first;
    for (i, v) in now.iter().enumerate() {
        if LAST_WATCH[i].swap(*v, Ordering::Relaxed) != *v {
            changed = true;
        }
    }
    if changed {
        line(format!(
            "watch: scene flags {:#x} {:#x}, runner {:#x}, engine {:#x}, event file {}, contexts {}, pad {:#06x}; {}",
            now[0],
            now[1],
            now[2],
            now[3],
            now[4] as i32,
            now[5] as i32,
            now[6],
            state()
        ));
    }
}
