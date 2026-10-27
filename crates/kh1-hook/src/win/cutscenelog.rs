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
//! - KH1's event script runner (`0x1401A38A0 (context, index)`, Checked (decomp)): its instructions are 4
//!   bytes with the opcode in the last; a line when a call moves the script on (the script's code, its
//!   counter at the start and the end of the call, the opcode there), and when the script ends.
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

/// The script context's code pointer, counter and cursor (Checked (decomp)), and the opcode's
/// ending (the instruction's byte 3).
const CTX_CODE: usize = 0x80;
const CTX_PC: usize = 0x90;
const CTX_CURSOR: usize = 0x190;
const OP_END: u8 = 5;

/// Lines written at most (a long session on the log would grow without end).
const MAX_LINES: u32 = 40_000;
static LINES: AtomicU32 = AtomicU32::new(0);

static ORIG_SCENE: AtomicUsize = AtomicUsize::new(0);
static ORIG_RUNNER: AtomicUsize = AtomicUsize::new(0);
static ORIG_MOVIE: AtomicUsize = AtomicUsize::new(0);
/// The last (code, counter) written for the runner, so a script waiting on a frame says it once.
static LAST_CODE: AtomicUsize = AtomicUsize::new(0);
static LAST_PC: AtomicU32 = AtomicU32::new(u32::MAX);

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

/// The script's counter and the opcode there (`None` if it can't be read).
unsafe fn at(ctx: usize) -> Option<(usize, u32, u8)> {
    if !super::crash::readable(ctx) || !super::crash::readable(ctx + CTX_CURSOR + 3) {
        return None;
    }
    let code = rd::<u64>(ctx + CTX_CODE) as usize;
    let pc = rd::<u32>(ctx + CTX_PC);
    let ins = code + pc as usize * 4;
    if code < 0x10000 || !super::crash::readable(ins) || !super::crash::readable(ins + 3) {
        return Some((code, pc, 0xFF));
    }
    Some((code, pc, rd::<u8>(ins + 3)))
}

unsafe extern "system" fn runner(ctx: usize, idx: usize, c: usize, d: usize) -> usize {
    let on = log_on(LogCat::Cutscenes);
    let before = if on { at(ctx) } else { None };
    let f: Orig4 = std::mem::transmute(ORIG_RUNNER.load(Ordering::Acquire));
    let r = f(ctx, idx, c, d);
    if let (true, Some((code, pc, op))) = (on, before) {
        if let Some((code2, pc2, op2)) = at(ctx) {
            let moved = pc2 != pc || code2 != code;
            let new = LAST_CODE.swap(code2, Ordering::Relaxed) != code2 || LAST_PC.swap(pc2, Ordering::Relaxed) != pc2;
            if moved || new {
                let end = if op2 == OP_END { " (end)" } else { "" };
                line(format!(
                    "script run (ctx {:#x}, index {}): code {:#x} counter {pc} op {op:#x} -> counter {pc2} op {op2:#x}{end}, cursor {}",
                    ctx.wrapping_sub(base()),
                    idx as i32,
                    code2.wrapping_sub(base()),
                    rd::<u32>(ctx + CTX_CURSOR)
                ));
            }
        }
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
