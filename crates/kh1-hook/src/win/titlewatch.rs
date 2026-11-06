//! The title watch: a log of KH1's way back to its title (the owner: "going back to the title
//! screen still gets you stuck in a endless loading screen"; notes in `docs/kh1-softreset.md`,
//! "The way back to the title").
//!
//! KH1's way back to the title (a save menu's or the system's "return to title", Steam): the
//! request (`[0x1421AAF18] + 0x1290` set to 0x100 by the PC's confirm dialog `0x1400D94C0`) makes
//! the field frame (`0x140170D50`; the pause loop's `0x14016FFA0` has the same code) set
//! `0x1422EC090` to 2 and then wait, a frame at a time, for: a movie to end (`0x1422EC094` /
//! `0x1423408D0`), a save to be written (`0x142E205BC`), and KH1's two job slots to empty
//! (`0x14234AFA8`, 0x18 bytes each: the function and its argument; its file reader `0x14028C390`
//! and `0x1401FA880` clear their own slot when done). Then, in one frame, it tears the game down
//! (among it `0x1400D2C70 (1)` -> `0x1400DBAC0`, which releases every bank of KH1's bank list, each
//! bank's destructor `0x1400DB470` / `0x1400DB7A0` sleeping 10 ms at a time on the game thread
//! until KH1's sound engine lets the bank go), sets `0x14233FBEC` to 0 and returns 2 to the
//! game's loop. Checked (decomp). On the way the owner took (state 1, from a save point) the
//! frame first starts `WaitTitleProject.exe` (`0x1400D9800 (1)`), and the 2 ends the game thread's
//! loop (`0x140103460`: the shutdown `0x140170CA0`, then the thread ends): KH1's way back to its
//! title is a restart of the game, not its reset (Checked (decomp); `docs/kh1-softreset.md`, "The
//! way back is a restart"). `detach` logs the game thread's end and the process's end.
//!
//! Nothing here changes what KH1 does. Loaded with the debug tools only; a thread of ours looks
//! once a second, and only while the Loading & Cache log is on writes a line when:
//! - KH1's return-to-title or quit is pending (`0x1422EC090` not 0, or the request at `+0x1290`
//!   0 or more): which of KH1's waits is still false;
//! - KH1's frame end hasn't run for 2 s (the game thread stuck in a call): where the game thread
//!   is (its instruction and KH1's return addresses on its stack, Steam addresses, innermost
//!   first) with the same state; a thread that can't be read says why (ended, with its exit code,
//!   or Windows' error); and once per stretch, on the way back to the title (pending, or the game
//!   thread ended), one line with every thread of the process that has KH1's code or ours on its
//!   stack (where it is) and a count of the rest;
//! - KH1 has drawn its Loading line for 5 s (it draws it while `0x1423404EC` or `0x1423404F4` is
//!   1, `0x14017B190`, Checked (decomp)).
//!
//! Per frame: one clock store at KH1's frame end (`frame_end`), only with the debug tools.

use super::addr::St;
use super::log;
use super::sys::*;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

/// KH1's game mode (the mode machine's state, `0x14233FBC0`; `loadscreen` / `softreset` use it).
const MODE: St = St(0x233FBC0);
/// The mode change's countdown (`0x1422EC0AC`).
const MODE_COUNTDOWN: St = St(0x22EC0AC);
/// The field frame's quit / return-to-title state: 0 none, 1 quit to the launcher, 2 the request.
const QUIT: St = St(0x22EC090);
/// A movie was playing when it came: the wait for it (`0x1422EC094`) and the movie (`0x1423408D0`).
const MOVIE_WAIT: St = St(0x22EC094);
const MOVIE: St = St(0x23408D0);
/// A save being written (the save menu's, `0x142E205BC`).
const SAVING: St = St(0x2E205BC);
/// KH1's job slots (two of 0x18 bytes: function, argument, done).
const JOBS: St = St(0x234AFA8);
/// KH1's file read job (`0x14028C390`; its argument's `+4` is the file's name) and its other one.
const FILE_JOB: St = St(0x28C390);
/// The game object (`0x1421AAF18`); its `+0x1290` is the return-to-title request (below 0: none).
const APP: St = St(0x21AAF18);
const REQUEST: usize = 0x1290;
/// 0 once the field frame's teardown has run (`0x14233FBEC`).
const UP: St = St(0x233FBEC);
/// KH1's Loading line's flags (`0x1423404EC`, a copy of the script runner's skip bit
/// `0x1423AB2D0` & 1; `0x1423404F4`, a room's data load).
const LOADING_A: St = St(0x23404EC);
const LOADING_B: St = St(0x23404F4);
/// KH1's bank list's count and its stream deletes pending (`0x1404D65E8`, `0x1404D6608`).
const BANK_COUNT: St = St(0x4D65E8);
const STREAM_DELETES: St = St(0x4D6608);

#[link(name = "kernel32")]
extern "system" {
    fn GetLastError() -> u32;
    fn GetExitCodeThread(thread: *mut c_void, code: *mut u32) -> i32;
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> *mut c_void;
    fn Thread32First(snap: *mut c_void, entry: *mut ThreadEntry) -> i32;
    fn Thread32Next(snap: *mut c_void, entry: *mut ThreadEntry) -> i32;
}

/// Toolhelp's THREADENTRY32.
#[repr(C)]
#[derive(Default)]
struct ThreadEntry {
    size: u32,
    usage: u32,
    id: u32,
    owner: u32,
    base_pri: i32,
    delta_pri: i32,
    flags: u32,
}

static STARTED: AtomicBool = AtomicBool::new(false);
static GAME_THREAD: AtomicU32 = AtomicU32::new(0);
/// KH1's game thread has ended (its thread end reached our `DllMain`).
static GAME_ENDED: AtomicBool = AtomicBool::new(false);
/// When KH1's frame end last ran (`frameadv::wall_ms`; 0: not yet).
static LAST_END: AtomicU64 = AtomicU64::new(0);

/// Lines written at most (the whole session), and per stuck stretch.
const MAX_LINES: u32 = 600;
const MAX_PER_STRETCH: u32 = 90;
/// The stack copied from the game thread when it's stuck.
const STACK_COPY: usize = 16 * 1024;
const DEPTH: usize = 16;
/// The process's threads listed at most, and the return addresses shown for each.
const MAX_THREADS: usize = 64;
const THREAD_DEPTH: usize = 8;

/// The watch thread (the debug tools only).
pub unsafe fn start() {
    if !super::debug_on() || STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    CreateThread(std::ptr::null_mut(), 0, watch, std::ptr::null_mut(), 0, std::ptr::null_mut());
}

/// KH1's frame end (`debugmenu::frame_end`): when it last ran, and on which thread.
pub fn frame_end() {
    if !STARTED.load(Ordering::Relaxed) {
        return;
    }
    if GAME_THREAD.load(Ordering::Relaxed) == 0 {
        GAME_THREAD.store(unsafe { GetCurrentThreadId() }, Ordering::Relaxed);
    }
    LAST_END.store(super::frameadv::wall_ms().max(1), Ordering::Relaxed);
}

/// Our `DllMain`'s thread end (3) and process end (0): KH1's game thread ending and the process
/// ending are logged (Loading & Cache, the watch running), with nothing allocated and no lock
/// waited on (`log_line_quiet`). On Steam KH1's way back to its title ends the game thread and
/// starts `WaitTitleProject.exe` (`0x1400D9800`, `docs/kh1-softreset.md`): these lines tell how
/// far the old game got.
pub fn detach(reason: u32, process_ending: bool) {
    if !STARTED.load(Ordering::Relaxed) || !super::log_on(super::LogCat::Loading) {
        return;
    }
    if reason == 3 {
        let id = GAME_THREAD.load(Ordering::Relaxed);
        if id != 0 && id == unsafe { GetCurrentThreadId() } {
            GAME_ENDED.store(true, Ordering::Relaxed);
            super::log_line_quiet(b"title watch: KH1's game thread has ended (its thread end reached our DllMain)");
        }
    } else if process_ending {
        super::log_line_quiet(b"title watch: the game's process is ending (DLL_PROCESS_DETACH reached our DllMain; every other thread already stopped)");
    } else {
        super::log_line_quiet(b"title watch: our DLL is being unloaded (DLL_PROCESS_DETACH, FreeLibrary)");
    }
}

unsafe fn rd<T: Copy + Default>(at: usize) -> T {
    if !super::crash::readable(at) || !super::crash::readable(at + std::mem::size_of::<T>() - 1) {
        return T::default();
    }
    std::ptr::read_unaligned(at as *const T)
}

/// A C string of KH1's (64 bytes at most).
unsafe fn name(at: usize) -> String {
    let mut out = String::new();
    for k in 0..64 {
        if !super::crash::readable(at + k) {
            break;
        }
        let c: u8 = std::ptr::read_unaligned((at + k) as *const u8);
        if c == 0 {
            break;
        }
        out.push(if (0x20..0x7F).contains(&c) { c as char } else { '?' });
    }
    out
}

#[repr(C, align(16))]
struct Context([u8; 1232]);

/// What KH1 is waiting on now, in words.
unsafe fn state(base: usize) -> (bool, bool, String) {
    let quit: i32 = rd(base + QUIT);
    let app: usize = rd(base + APP);
    let request: i32 = if app >= 0x10000 { rd(app + REQUEST) } else { -1 };
    let pending = quit != 0 || request >= 0;
    let loading = rd::<i32>(base + LOADING_A) == 1 || rd::<i32>(base + LOADING_B) == 1;
    let mut waits = Vec::new();
    if rd::<i32>(base + MOVIE_WAIT) != 0 && rd::<i32>(base + MOVIE) != 0 {
        waits.push("a movie to end (0x1423408D0)".to_string());
    }
    if rd::<i32>(base + SAVING) != 0 {
        waits.push("the save being written (0x142E205BC)".to_string());
    }
    for k in 0..2 {
        let slot = base + JOBS + k * 0x18;
        let f: usize = rd(slot);
        if f == 0 {
            continue;
        }
        let arg: usize = rd(slot + 8);
        let what = if f == base + FILE_JOB && arg >= 0x10000 {
            format!("KH1's file read of \"{}\"", name(arg + 4))
        } else {
            format!("{:#x}", f.wrapping_sub(base) + 0x1_4000_0000)
        };
        waits.push(format!("job slot {k}: {what}"));
    }
    let line = format!(
        "mode {} (change in {}), quit/return {quit} (request {request}), teardown {}; waiting on: {}; KH1's Loading line {} (0x1423404EC {}, 0x1423404F4 {}); bank list {}, stream deletes pending {}",
        rd::<i32>(base + MODE),
        rd::<i32>(base + MODE_COUNTDOWN),
        if rd::<i32>(base + UP) == 0 { "done" } else { "not yet" },
        if waits.is_empty() { "nothing".to_string() } else { waits.join(", ") },
        if loading { "shown" } else { "not shown" },
        rd::<i32>(base + LOADING_A),
        rd::<i32>(base + LOADING_B),
        rd::<i32>(base + BANK_COUNT),
        rd::<i32>(base + STREAM_DELETES),
    );
    (pending, loading, line)
}

/// Where a thread is: its instruction and the return addresses on its stack (KH1's as Steam
/// addresses, ours as `ours+offset`), `depth` of them at most. Nothing allocated while it is held.
/// A thread that can't be read says why: ended (its exit code), or the call that failed and
/// Windows' error.
unsafe fn where_is(thread: *mut c_void, depth: usize, stack: &mut [u64], ctx: &mut Context, exe: &super::loadprof::Module, ours: &super::loadprof::Module) -> (String, bool) {
    std::ptr::write_unaligned(ctx.0.as_mut_ptr().add(0x30) as *mut u32, 0x10_0003);
    let mut copied = 0usize;
    let mut rip = 0u64;
    let mut failed: Option<(&str, u32)> = None;
    if SuspendThread(thread) != u32::MAX {
        if GetThreadContext(thread, ctx.0.as_mut_ptr() as *mut c_void) != 0 {
            rip = std::ptr::read_unaligned(ctx.0.as_ptr().add(0xF8) as *const u64);
            let rsp = std::ptr::read_unaligned(ctx.0.as_ptr().add(0x98) as *const u64) as usize;
            let mut info = [0u64; 6];
            if rsp != 0 && VirtualQuery(rsp as *const c_void, info.as_mut_ptr() as *mut c_void, 48) != 0 && info[4] as u32 == 0x1000 {
                let top = info[0] as usize + info[3] as usize;
                copied = top.saturating_sub(rsp).min(stack.len() * 8) / 8;
                std::ptr::copy_nonoverlapping(rsp as *const u64, stack.as_mut_ptr(), copied);
            }
        } else {
            failed = Some(("its registers not read", GetLastError()));
        }
        ResumeThread(thread);
    } else {
        failed = Some(("not suspended", GetLastError()));
    }
    if let Some((what, err)) = failed {
        let mut code = 0u32;
        if GetExitCodeThread(thread, &mut code) != 0 && code != 0x103 {
            return (format!("ended (exit code {code:#x})"), true);
        }
        return (format!("not readable: {what} (error {err})"), true);
    }
    let at = |v: usize| {
        if exe.holds(v) {
            format!("{:#x}", v - exe.base + 0x1_4000_0000)
        } else if ours.holds(v) {
            format!("ours+{:#x}", v - ours.base)
        } else {
            format!("{v:#x}")
        }
    };
    let mut out = vec![format!("at {}", at(rip as usize))];
    let mut game_code = exe.holds(rip as usize) || ours.holds(rip as usize);
    for &v in &stack[..copied] {
        if out.len() > depth {
            break;
        }
        let v = v as usize;
        if (exe.holds(v) && super::loadprof::after_call(exe, v)) || (ours.holds(v) && super::loadprof::after_call(ours, v)) {
            out.push(at(v));
            game_code = true;
        }
    }
    (out.join(" <- "), game_code)
}

/// Every thread of the game's process but this one (ids, KH1's game thread marked), from
/// Windows' list; `MAX_THREADS` at most.
unsafe fn threads(game: u32) -> Vec<u32> {
    let mut out = Vec::with_capacity(MAX_THREADS);
    let me = GetCurrentThreadId();
    let pid = GetCurrentProcessId();
    // (TH32CS_SNAPTHREAD 4.)
    let snap = CreateToolhelp32Snapshot(4, 0);
    if snap.is_null() || snap as isize == -1 {
        return out;
    }
    let mut e = ThreadEntry { size: std::mem::size_of::<ThreadEntry>() as u32, ..Default::default() };
    let mut more = Thread32First(snap, &mut e) != 0;
    while more && out.len() < MAX_THREADS {
        if e.owner == pid && e.id != me {
            // (The game thread first.)
            if e.id == game {
                out.insert(0, e.id);
            } else {
                out.push(e.id);
            }
        }
        e.size = std::mem::size_of::<ThreadEntry>() as u32;
        more = Thread32Next(snap, &mut e) != 0;
    }
    CloseHandle(snap);
    out
}

/// Where every thread of the process is (once per stuck stretch on KH1's way back to its title,
/// the game's frame stopped): a line for each thread with KH1's code or ours on its stack (KH1's
/// window thread, its render thread, a sound thread, or one of ours), a count for the rest (only
/// in Windows' or a driver's code, a pool's idle wait say), so a process that hangs on its way out
/// shows what holds it.
unsafe fn every_thread(stack: &mut [u64], ctx: &mut Context, exe: &super::loadprof::Module, ours: &super::loadprof::Module) {
    let game = GAME_THREAD.load(Ordering::Relaxed);
    let ids = threads(game);
    let total = ids.len();
    let mut lines = Vec::new();
    let mut others = 0usize;
    for id in ids {
        // (THREAD_SUSPEND_RESUME 0x2, THREAD_GET_CONTEXT 0x8, THREAD_QUERY_INFORMATION 0x40.)
        let h = OpenThread(0x4A, 0, id);
        let (place, game_code) = if h.is_null() { ("not opened".to_string(), true) } else { where_is(h, THREAD_DEPTH, stack, ctx, exe, ours) };
        if !h.is_null() {
            CloseHandle(h);
        }
        if game_code || id == game {
            lines.push(format!("{id}{} {place}", if id == game { " (KH1's game thread)" } else { "" }));
        } else {
            others += 1;
        }
    }
    log!(
        Loading,
        "title watch: the process's threads ({total}{}; {others} only in Windows' or a driver's code), innermost first: {}",
        if total == MAX_THREADS { " or more" } else { "" },
        lines.join(" | ")
    );
}

unsafe extern "system" fn watch(_: *mut c_void) -> u32 {
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    let exe = super::loadprof::Module::read(base);
    let mut ours_handle: *mut c_void = std::ptr::null_mut();
    GetModuleHandleExW(6, start as *const () as *const u16, &mut ours_handle);
    let ours = super::loadprof::Module::read(ours_handle as usize);
    let mut stack = vec![0u64; STACK_COPY / 8];
    let mut ctx = Box::new(Context([0; 1232]));
    let mut thread: *mut c_void = std::ptr::null_mut();
    let mut lines = 0u32;
    let mut stretch = 0u32;
    let mut since = 0u64;
    let mut loading_since = 0u64;
    // (The process's threads listed this stuck stretch.)
    let mut listed = false;
    loop {
        Sleep(1000);
        if !super::log_on(super::LogCat::Loading) || base == 0 || super::addr::Build::current() == super::addr::Build::Unknown {
            continue;
        }
        let now = super::frameadv::wall_ms();
        let last = LAST_END.load(Ordering::Relaxed);
        let stuck = last != 0 && now.saturating_sub(last) >= 2000 && !super::frameadv::holding();
        let (pending, loading, line) = state(base);
        loading_since = if loading { if loading_since == 0 { now } else { loading_since } } else { 0 };
        let long_loading = loading_since != 0 && now - loading_since >= 5000;
        if !(stuck || pending || long_loading) {
            if stretch > 0 {
                log!(Loading, "title watch: over after {:.0} s: {line}", now.saturating_sub(since) as f32 / 1000.0);
            }
            stretch = 0;
            listed = false;
            continue;
        }
        if stretch == 0 {
            since = now;
        }
        stretch += 1;
        if stretch > MAX_PER_STRETCH || lines >= MAX_LINES {
            continue;
        }
        // (A frame end held up with nothing of KH1's pending, a modal box of Windows' say: every
        // 10 s only.)
        if stuck && !pending && !long_loading && (stretch - 1) % 10 != 0 {
            continue;
        }
        lines += 1;
        let why = if stuck && GAME_ENDED.load(Ordering::Relaxed) {
            format!("KH1's game thread has ended ({:.1} s since its last frame end)", now.saturating_sub(last) as f32 / 1000.0)
        } else if stuck {
            format!("KH1's frame end hasn't run for {:.1} s", now.saturating_sub(last) as f32 / 1000.0)
        } else if pending {
            "KH1's return to the title (or quit) is pending".to_string()
        } else {
            "KH1 has shown its Loading line for 5 s or more".to_string()
        };
        let mut place = String::new();
        if stuck {
            if thread.is_null() {
                let id = GAME_THREAD.load(Ordering::Relaxed);
                if id != 0 && id != GetCurrentThreadId() {
                    // (THREAD_SUSPEND_RESUME 0x2, THREAD_GET_CONTEXT 0x8, THREAD_QUERY_INFORMATION 0x40.)
                    thread = OpenThread(0x4A, 0, id);
                }
            }
            if !thread.is_null() {
                place = format!("; game thread {}", where_is(thread, DEPTH, &mut stack, &mut ctx, &exe, &ours).0);
            }
        }
        log!(Loading, "title watch ({:.0} s): {why}: {line}{place}", now.saturating_sub(since) as f32 / 1000.0);
        if stuck && !listed && (pending || GAME_ENDED.load(Ordering::Relaxed)) {
            listed = true;
            every_thread(&mut stack, &mut ctx, &exe, &ours);
        }
    }
}
