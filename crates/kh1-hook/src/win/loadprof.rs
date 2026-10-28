//! The load profiler (t783): where the game thread's time goes in a slow frame around a load.
//!
//! The owner's t782 traces: one frame of about half a second (486 ms, 423 ms) on the first frame
//! KH1's opening script hands Sora over after the save's first room loads; the mod off, it doesn't
//! happen (the owner). Nothing in the log timed that frame's work, so this measures it before any
//! fix (`docs/kh1-load-profiler.md`).
//!
//! - **Loaded** only with the debug tools (`debug_on`) and the log's Loading & Cache category on
//!   at the start: then a thread of ours samples KH1's game thread (the thread KH1's frame end
//!   runs on) about once a millisecond.
//! - **When**: from the game's start until 10 s after the first room is entered (the title, the
//!   save's load, the first seconds in the room), and again for 10 s after each later room is
//!   entered (`room_entered`, from `takeover`'s room line). Outside those windows the thread sleeps
//!   and KH1's frame end only reads the clock (`frame_end`); without the profiler, one load.
//! - **A sample**: the game thread is suspended, its registers and the top 16 KB of its stack
//!   copied into a buffer made beforehand (nothing allocated and no lock taken while it is
//!   suspended), and it is resumed. The copy's return addresses (values in the code of KH1's exe or
//!   of ours that follow a call instruction) are its call stack, innermost first.
//! - **A slow frame** (over 50 ms between two of KH1's frame ends) gets one log line (Loading
//!   & Cache): its length, the samples in it, the share with our code anywhere on the stack, our
//!   functions that were innermost (where our time went, with what they were calling: ours, KH1's or
//!   the system's), our outermost function (the hook KH1 entered us by), and KH1's own functions
//!   (Steam addresses, as the decomp names them) for the samples with none of ours.
//! - Our functions are named from the DLL's PDB through the system's `dbghelp.dll` (the PDB the
//!   build left at its path); without it, `ours+offset` only (the offset into our DLL).
//! - The sampling itself costs the game thread a few hundredths of a millisecond a sample, so a
//!   window runs a few percent slower (Assumption: not measured).

use super::log;
use super::sys::*;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

#[repr(C)]
struct RuntimeFunction {
    begin: u32,
    end: u32,
    unwind: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn RtlLookupFunctionEntry(pc: u64, image_base: *mut u64, history: *mut c_void) -> *const RuntimeFunction;
}

#[link(name = "winmm")]
extern "system" {
    fn timeBeginPeriod(ms: u32) -> u32;
    fn timeEndPeriod(ms: u32) -> u32;
}

/// A frame this long or longer is reported (ms).
const SLOW_MS: u64 = 50;
/// How long a window stays open after a room is entered (ms).
const WINDOW_MS: u64 = 10_000;
/// Slow frames reported in one window at most.
const REPORTS_PER_WINDOW: u32 = 30;
/// The samples kept (a ring: about 20 s at 1 kHz).
const RING: usize = 20_000;
/// Return addresses kept per sample.
const DEPTH: usize = 32;
/// The stack copied per sample (bytes).
const STACK_COPY: usize = 16 * 1024;

/// The thread KH1's frame end runs on (0 until it first runs).
static GAME_THREAD: AtomicU32 = AtomicU32::new(0);
/// Frames ended since the start (the samples' tag).
static FRAME: AtomicU32 = AtomicU32::new(0);
/// The last frame end (µs since `clock`'s start; 0 none yet).
static LAST_END: AtomicU64 = AtomicU64::new(0);
/// The window is open until this time (ms since `clock`'s start); `u64::MAX` from the start
/// until the first room.
static UNTIL_MS: AtomicU64 = AtomicU64::new(u64::MAX);
static STARTED: AtomicBool = AtomicBool::new(false);
/// Slow frames waiting for the sampler to report: (frame tag, its length in µs, the log's game frame).
static SLOW: Mutex<Vec<(u32, u64, u64)>> = Mutex::new(Vec::new());

fn clock() -> &'static Instant {
    static T0: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    T0.get_or_init(Instant::now)
}

fn now_us() -> u64 {
    clock().elapsed().as_micros() as u64 + 1
}

fn open_now() -> bool {
    STARTED.load(Ordering::Relaxed) && now_us() / 1000 < UNTIL_MS.load(Ordering::Relaxed)
}

/// Starts the sampler thread (the debug tools and the Loading & Cache log on).
pub unsafe fn start() {
    if !super::debug_on() || !super::log_on(super::LogCat::Loading) || STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    clock();
    CreateThread(std::ptr::null_mut(), 0, sampler, std::ptr::null_mut(), 0, std::ptr::null_mut());
    log!(Loading, "load profiler: on until 10 s after the first room (then 10 s after each room entered); frames over {SLOW_MS} ms are reported");
}

/// A room was entered (`takeover`'s room line): the window opens (again) for 10 s.
pub fn room_entered() {
    if !STARTED.load(Ordering::Relaxed) {
        return;
    }
    UNTIL_MS.store(now_us() / 1000 + WINDOW_MS, Ordering::Relaxed);
}

/// KH1's frame end (`debugmenu::frame_end`, before KH1's own): the frame's length, and a slow
/// one queued for the sampler. Outside a window: a clock read (one load without the profiler).
pub fn frame_end() {
    if !open_now() {
        LAST_END.store(0, Ordering::Relaxed);
        return;
    }
    if GAME_THREAD.load(Ordering::Relaxed) == 0 {
        GAME_THREAD.store(unsafe { GetCurrentThreadId() }, Ordering::Relaxed);
    }
    let now = now_us();
    let tag = FRAME.fetch_add(1, Ordering::Relaxed);
    let last = LAST_END.swap(now, Ordering::Relaxed);
    if last != 0 && now - last >= SLOW_MS * 1000 {
        if let Ok(mut v) = SLOW.lock() {
            v.push((tag, now - last, super::takeover::GAME_FRAME.load(Ordering::Relaxed)));
        }
    }
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

/// A module's base, size and executable sections (from its PE headers).
#[derive(Clone, Default)]
struct Module {
    base: usize,
    size: usize,
    code: Vec<(usize, usize)>,
}

impl Module {
    unsafe fn read(base: usize) -> Module {
        if base == 0 {
            return Module::default();
        }
        let nt = base + rd::<u32>(base + 0x3C) as usize;
        let sections = rd::<u16>(nt + 6) as usize;
        let opt = nt + 24;
        let size = rd::<u32>(opt + 56) as usize;
        let first = opt + rd::<u16>(nt + 20) as usize;
        let mut code = Vec::new();
        for i in 0..sections {
            let s = first + i * 40;
            if rd::<u32>(s + 36) & 0x2000_0000 != 0 {
                let at = base + rd::<u32>(s + 12) as usize;
                code.push((at, at + rd::<u32>(s + 8) as usize));
            }
        }
        Module { base, size, code }
    }

    fn holds(&self, a: usize) -> bool {
        a >= self.base && a < self.base + self.size
    }

    /// `a` is in its code, 8 bytes in at least (the call before it can be read).
    fn in_code(&self, a: usize) -> bool {
        self.code.iter().any(|&(s, e)| a >= s + 8 && a < e)
    }
}

/// The value is a return address: in a module's code, after a call instruction (`E8 rel32`, or
/// `FF /2` with a register, a register and a byte or four of displacement, or RIP-relative).
unsafe fn after_call(m: &Module, v: usize) -> bool {
    if !m.in_code(v) {
        return false;
    }
    let b = |back: usize| rd::<u8>(v - back);
    let call_rm = |op: usize| b(op) == 0xFF && (b(op - 1) >> 3) & 7 == 2;
    b(5) == 0xE8 || call_rm(2) || call_rm(3) || call_rm(6) || call_rm(7)
}

#[derive(Clone, Copy)]
struct Sample {
    tag: u32,
    rip: u64,
    n: u8,
    frames: [u64; DEPTH],
}

#[repr(C, align(16))]
struct Context([u8; 1232]);

unsafe extern "system" fn sampler(_: *mut c_void) -> u32 {
    let exe = Module::read(GetModuleHandleA(std::ptr::null()) as usize);
    let mut ours_handle: *mut c_void = std::ptr::null_mut();
    GetModuleHandleExW(6, start as *const () as *const u16, &mut ours_handle);
    let ours = Module::read(ours_handle as usize);
    let mut ring: Vec<Sample> = vec![Sample { tag: 0, rip: 0, n: 0, frames: [0; DEPTH] }; RING];
    let mut stack = vec![0u64; STACK_COPY / 8];
    let mut ctx = Box::new(Context([0; 1232]));
    let mut next = 0usize;
    let mut taken = 0u64;
    let mut thread: *mut c_void = std::ptr::null_mut();
    let mut fine = false;
    let mut reports = 0u32;
    let mut syms = Symbols::default();
    loop {
        if !open_now() {
            if fine {
                timeEndPeriod(1);
                fine = false;
                log!(Loading, "load profiler: window over ({taken} samples, {reports} slow frames reported)");
                taken = 0;
                reports = 0;
                if let Ok(mut v) = SLOW.lock() {
                    v.clear();
                }
            }
            Sleep(50);
            continue;
        }
        if !fine {
            timeBeginPeriod(1);
            fine = true;
        }
        if thread.is_null() {
            let id = GAME_THREAD.load(Ordering::Relaxed);
            if id == 0 || id == GetCurrentThreadId() {
                Sleep(5);
                continue;
            }
            // (THREAD_SUSPEND_RESUME 0x2, THREAD_GET_CONTEXT 0x8, THREAD_QUERY_INFORMATION 0x40.)
            thread = OpenThread(0x4A, 0, id);
            if thread.is_null() {
                log!(Loading, "load profiler: KH1's game thread {id} not opened; off");
                return 0;
            }
            log!(Loading, "load profiler: sampling KH1's game thread {id} (exe at {:#x}, ours at {:#x})", exe.base, ours.base);
            // (t786) Our symbols now, not on the first report: loading the PDB held the sampler for
            // the frames around it (the t785 log's 1 and 4 samples in 133 ms and 210 ms).
            syms.load(&ours);
        }
        // ---- the sample (nothing allocated, no lock, while the thread is held) ----
        let tag = FRAME.load(Ordering::Relaxed);
        // (CONTEXT_AMD64 | CONTROL | INTEGER at +0x30.)
        std::ptr::write_unaligned(ctx.0.as_mut_ptr().add(0x30) as *mut u32, 0x10_0003);
        let mut copied = 0usize;
        let mut rip = 0u64;
        if SuspendThread(thread) != u32::MAX {
            if GetThreadContext(thread, ctx.0.as_mut_ptr() as *mut c_void) != 0 {
                rip = rd::<u64>(ctx.0.as_ptr() as usize + 0xF8);
                let rsp = rd::<u64>(ctx.0.as_ptr() as usize + 0x98) as usize;
                let mut info = [0u64; 6];
                if rsp != 0 && VirtualQuery(rsp as *const c_void, info.as_mut_ptr() as *mut c_void, 48) != 0 && info[4] as u32 == 0x1000 {
                    let top = info[0] as usize + info[3] as usize;
                    copied = (top.saturating_sub(rsp)).min(STACK_COPY) / 8;
                    std::ptr::copy_nonoverlapping(rsp as *const u64, stack.as_mut_ptr(), copied);
                }
            }
            ResumeThread(thread);
        }
        // ---- held no more ----
        if rip != 0 {
            let s = &mut ring[next];
            s.tag = tag;
            s.rip = rip;
            s.n = 0;
            for &v in &stack[..copied] {
                if s.n as usize == DEPTH {
                    break;
                }
                let v = v as usize;
                if (exe.holds(v) && after_call(&exe, v)) || (ours.holds(v) && after_call(&ours, v)) {
                    s.frames[s.n as usize] = v as u64;
                    s.n += 1;
                }
            }
            next = (next + 1) % RING;
            taken += 1;
        }
        let due: Vec<(u32, u64, u64)> = SLOW.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default();
        for (tag, us, game_frame) in due {
            if reports < REPORTS_PER_WINDOW {
                reports += 1;
                report(&ring, tag, us, game_frame, &exe, &ours, &mut syms);
            }
        }
        Sleep(1);
    }
}

/// Where an address is: ours, KH1's exe, or anything else (the system's DLLs, the driver).
#[derive(Clone, Copy, PartialEq)]
enum Place {
    Ours,
    Kh1,
    Other,
}

fn place(exe: &Module, ours: &Module, a: usize) -> Place {
    if ours.holds(a) {
        Place::Ours
    } else if exe.holds(a) {
        Place::Kh1
    } else {
        Place::Other
    }
}

/// The start of the function holding `a` (its unwind entry; `a` itself without one).
unsafe fn function_of(a: usize) -> usize {
    let mut base = 0u64;
    let f = RtlLookupFunctionEntry(a as u64, &mut base, std::ptr::null_mut());
    if f.is_null() {
        a
    } else {
        base as usize + (*f).begin as usize
    }
}

fn pct(n: usize, of: usize) -> u32 {
    if of == 0 {
        0
    } else {
        (n * 100 / of) as u32
    }
}

/// Counts by key, most first.
fn ranked<K: Copy + Eq + std::hash::Hash>(keys: impl Iterator<Item = K>) -> Vec<(K, usize)> {
    let mut m: std::collections::HashMap<K, usize> = std::collections::HashMap::new();
    for k in keys {
        *m.entry(k).or_default() += 1;
    }
    let mut v: Vec<(K, usize)> = m.into_iter().collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    v
}

unsafe fn report(ring: &[Sample], tag: u32, us: u64, game_frame: u64, exe: &Module, ours: &Module, syms: &mut Symbols) {
    let ms = us as f32 / 1000.0;
    let samples: Vec<&Sample> = ring.iter().filter(|s| s.rip != 0 && s.tag == tag).collect();
    let n = samples.len();
    if n == 0 {
        log!(Loading, "load profiler: slow frame (log frame f{game_frame}): {ms:.0} ms, no samples in it");
        return;
    }
    let at = |s: &Sample| place(exe, ours, s.rip as usize);
    let stack_of = |s: &Sample| -> Vec<usize> { std::iter::once(s.rip as usize).chain(s.frames[..s.n as usize].iter().map(|&f| f as usize)).collect() };
    let with_ours: Vec<&&Sample> = samples.iter().filter(|s| stack_of(s).iter().any(|&a| ours.holds(a))).collect();
    let rip_split = |set: &[&&Sample]| {
        let o = set.iter().filter(|s| at(s) == Place::Ours).count();
        let k = set.iter().filter(|s| at(s) == Place::Kh1).count();
        format!("in ours {}%, KH1 {}%, system {}%", pct(o, set.len()), pct(k, set.len()), pct(set.len() - o - k, set.len()))
    };
    let all: Vec<&&Sample> = samples.iter().collect();
    let mut line = format!(
        "load profiler: slow frame (log frame f{game_frame}): {ms:.0} ms, {n} samples; our code on the stack in {}% ({}); running {}",
        pct(with_ours.len(), n),
        with_ours.len(),
        rip_split(&all)
    );
    // Our innermost function (where our time went) and what it was in.
    let inner = ranked(with_ours.iter().map(|s| function_of(*stack_of(s).iter().find(|&&a| ours.holds(a)).unwrap())));
    line.push_str(" | ours, innermost:");
    for (f, c) in inner.iter().take(6) {
        let set: Vec<&&Sample> = with_ours.iter().copied().filter(|s| function_of(*stack_of(s).iter().find(|&&a| ours.holds(a)).unwrap()) == *f).collect();
        // (t786) Its source lines most samples were at: the instruction running (ours) or the call
        // out of it (a return address, the call before it).
        let spots = ranked(set.iter().map(|s| {
            let a = *stack_of(s).iter().find(|&&a| ours.holds(a)).unwrap();
            if a == s.rip as usize { a } else { a - 1 }
        }));
        let lines: Vec<String> = spots.iter().take(3).map(|(a, k)| format!("{} {}", syms.line(*a).unwrap_or_else(|| format!("ours+{:x}", a - ours.base)), k)).collect();
        line.push_str(&format!(" {}% {} ({}; at {});", pct(*c, n), syms.name(ours, *f), rip_split(&set), lines.join(", ")));
    }
    // Our outermost function (the hook KH1 entered us by).
    let outer = ranked(with_ours.iter().map(|s| function_of(*stack_of(s).iter().rev().find(|&&a| ours.holds(a)).unwrap())));
    line.push_str(" | ours, entered by:");
    for (f, c) in outer.iter().take(4) {
        line.push_str(&format!(" {}% {};", pct(*c, n), syms.name(ours, *f)));
    }
    // KH1 alone: its innermost function (Steam addresses, 0x140000000-based as in the decomp).
    let alone: Vec<&&Sample> = samples.iter().filter(|s| !stack_of(s).iter().any(|&a| ours.holds(a))).collect();
    if !alone.is_empty() {
        let k1 = ranked(alone.iter().filter_map(|s| stack_of(s).into_iter().find(|&a| exe.holds(a))).map(|a| function_of(a)));
        line.push_str(&format!(" | without ours {}% ({}), KH1's innermost:", pct(alone.len(), n), rip_split(&alone)));
        for (f, c) in k1.iter().take(5) {
            line.push_str(&format!(" {}% {:#x};", pct(*c, n), 0x1_4000_0000 + (f - exe.base)));
        }
    }
    log!(Loading, "{line}");
}

/// Our functions' names, from our PDB through the system's `dbghelp.dll` (loaded on the first
/// report, on the sampler's thread).
#[derive(Default)]
struct Symbols {
    tried: bool,
    from_addr: usize,
    line_from_addr: usize,
    process: usize,
    names: std::collections::HashMap<usize, String>,
}

type SymInitializeW = unsafe extern "system" fn(*mut c_void, *const u16, i32) -> i32;
type SymSetOptions = unsafe extern "system" fn(u32) -> u32;
type SymLoadModuleExW = unsafe extern "system" fn(*mut c_void, *mut c_void, *const u16, *const u16, u64, u32, *mut c_void, u32) -> u64;
type SymFromAddrW = unsafe extern "system" fn(*mut c_void, u64, *mut u64, *mut c_void) -> i32;
type SymGetLineFromAddrW64 = unsafe extern "system" fn(*mut c_void, u64, *mut u32, *mut c_void) -> i32;

impl Symbols {
    unsafe fn load(&mut self, ours: &Module) {
        if self.tried {
            return;
        }
        self.tried = true;
        let mut buf = [0u8; 300];
        let n = GetSystemDirectoryA(buf.as_mut_ptr(), 260) as usize;
        let mut path = buf[..n].to_vec();
        path.extend_from_slice(b"\\dbghelp.dll\0");
        let m = LoadLibraryA(path.as_ptr());
        if m.is_null() {
            log!(Loading, "load profiler: the system's dbghelp.dll not loaded; our functions by offset only");
            return;
        }
        let get = |name: &[u8]| GetProcAddress(m, name.as_ptr()) as usize;
        let (init, opts, load, from) = (get(b"SymInitializeW\0"), get(b"SymSetOptions\0"), get(b"SymLoadModuleExW\0"), get(b"SymFromAddrW\0"));
        if init == 0 || opts == 0 || load == 0 || from == 0 {
            return;
        }
        // (A handle of our own for the session: any value works with no process invaded.)
        let process = 0x6B68_3170usize;
        std::mem::transmute::<usize, SymSetOptions>(opts)(0x2 | 0x4 | 0x10);
        if std::mem::transmute::<usize, SymInitializeW>(init)(process as *mut c_void, std::ptr::null(), 0) == 0 {
            log!(Loading, "load profiler: dbghelp didn't start; our functions by offset only");
            return;
        }
        let mut file = [0u16; 1024];
        let len = GetModuleFileNameW(ours.base as *mut c_void, file.as_mut_ptr(), 1023) as usize;
        file[len] = 0;
        let got = std::mem::transmute::<usize, SymLoadModuleExW>(load)(process as *mut c_void, std::ptr::null_mut(), file.as_ptr(), std::ptr::null(), ours.base as u64, ours.size as u32, std::ptr::null_mut(), 0);
        log!(Loading, "load profiler: our symbols {}", if got != 0 { "loaded (names from the build's PDB, if it is still at its path)" } else { "not loaded; our functions by offset only" });
        if got != 0 {
            self.from_addr = from;
            self.line_from_addr = get(b"SymGetLineFromAddrW64\0");
            self.process = process;
        }
    }

    /// `file.rs:line` of the instruction at `a`, from the PDB's line tables (t786: the release build
    /// keeps line tables, `Cargo.toml`), if it has them.
    unsafe fn line(&mut self, a: usize) -> Option<String> {
        if self.line_from_addr == 0 {
            return None;
        }
        // IMAGEHLP_LINEW64: SizeOfStruct (40), Key, LineNumber at +16, FileName (wide) at +24, Address.
        let mut info = [0u64; 5];
        let p = info.as_mut_ptr() as usize;
        std::ptr::write_unaligned(p as *mut u32, 40);
        let mut disp = 0u32;
        if std::mem::transmute::<usize, SymGetLineFromAddrW64>(self.line_from_addr)(self.process as *mut c_void, a as u64, &mut disp, p as *mut c_void) == 0 {
            return None;
        }
        let number = rd::<u32>(p + 16);
        let file = rd::<u64>(p + 24) as usize;
        if file == 0 {
            return None;
        }
        let n = (0..1024).take_while(|&i| *((file + i * 2) as *const u16) != 0).count();
        let path = String::from_utf16_lossy(std::slice::from_raw_parts(file as *const u16, n));
        let short = path.rsplit(['\\', '/']).next().unwrap_or(&path).to_string();
        Some(format!("{short}:{number}"))
    }

    /// `name (ours+offset)`, or `ours+offset` alone.
    unsafe fn name(&mut self, ours: &Module, f: usize) -> String {
        if let Some(s) = self.names.get(&f) {
            return s.clone();
        }
        if !self.tried {
            self.load(ours);
        }
        let off = format!("ours+{:x}", f - ours.base);
        let mut s = off.clone();
        if self.from_addr != 0 {
            // SYMBOL_INFOW: 88 bytes, then the name (MaxNameLen at +80, NameLen at +76, Name at +84).
            let mut info = vec![0u64; (88 + 1024) / 8];
            let p = info.as_mut_ptr() as usize;
            std::ptr::write_unaligned(p as *mut u32, 88);
            std::ptr::write_unaligned((p + 80) as *mut u32, 500);
            let mut disp = 0u64;
            if std::mem::transmute::<usize, SymFromAddrW>(self.from_addr)(self.process as *mut c_void, f as u64, &mut disp, p as *mut c_void) != 0 && disp < 0x1_0000 {
                let len = (rd::<u32>(p + 76) as usize).min(500);
                let wide = std::slice::from_raw_parts((p + 84) as *const u16, len);
                // (t786) A function with no name of its own in the PDB shows as the one before it, plus.
                let plus = if disp != 0 { format!("+{disp:#x}") } else { String::new() };
                s = format!("{}{plus} ({off})", demangle(&String::from_utf16_lossy(wide)));
            }
        }
        self.names.insert(f, s.clone());
        s
    }
}

/// Rust's legacy mangling (`_ZN` length-prefixed parts `E`) as a path, its hash dropped.
fn demangle(s: &str) -> String {
    let Some(mut rest) = s.strip_prefix("_ZN").or_else(|| s.strip_prefix("__ZN")) else { return s.to_string() };
    let mut parts: Vec<String> = Vec::new();
    while !rest.starts_with('E') && !rest.is_empty() {
        let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
        let Ok(len) = rest[..digits].parse::<usize>() else { return s.to_string() };
        if digits == 0 || rest.len() < digits + len {
            return s.to_string();
        }
        parts.push(rest[digits..digits + len].to_string());
        rest = &rest[digits + len..];
    }
    if parts.last().is_some_and(|p| p.len() == 17 && p.starts_with('h') && p[1..].chars().all(|c| c.is_ascii_hexdigit())) {
        parts.pop();
    }
    let joined = parts.join("::");
    let mut out = joined.replace("$LT$", "<").replace("$GT$", ">").replace("$RF$", "&").replace("$BP$", "*").replace("$LP$", "(").replace("$RP$", ")").replace("$C$", ",").replace("$u20$", " ").replace("$u7b$", "{").replace("$u7d$", "}").replace("$u27$", "'").replace("..", "::");
    if out.starts_with('_') && out[1..].starts_with('$') {
        out.remove(0);
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn demangles() {
        assert_eq!(super::demangle("_ZN7dinput83win8takeover10new_driver17h0123456789abcdefE"), "dinput8::win::takeover::new_driver");
        assert_eq!(super::demangle("plain"), "plain");
    }
}
