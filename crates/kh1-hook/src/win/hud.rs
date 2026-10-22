//! Page Down hides the whole HUD and shows it again (the user: "can you also make it so that pg dn
//! hides the hud?"). `docs/kh1-hud-hide.md`.
//!
//! KH1 has no single HUD switch: each piece fades itself. The flag events set (`0x142867CD8` bit
//! 3, event opcodes `0x1401A77C0` set / `0x1401A77B0` clear) stops one gauge task drawing and is
//! read by no other HUD task. Every piece of the field HUD is a task of KH1's HUD module (all
//! registered by its setup `0x140264C50`, the sixteen in `TASKS`): the command menu with its lists
//! and shortcuts (and so our Drive page, list and title), the gauges, the lock-on cursors, the
//! target's gauge, the info messages. Each draws at once, through KH1's immediate draw
//! (`0x1400FBBA0`).
//!
//! So the HUD tasks run as before (their fades and timers, the gauges' sounds, the message queue:
//! nothing waits), and while the HUD is hidden every draw one of them makes is turned into an
//! empty draw (count 0, KH1's own path for a packet with nothing in it). Each task is put behind a
//! stub of ours as KH1 registers it (`register`), and KH1's task lookup sees the stub as the task
//! (`find`), so the HUD setup, whenever KH1 runs it again, finds its tasks and doesn't add them
//! twice (and a task KH1 removes and adds back is a stub again). The pause menu, events, their subtitles and message windows, and the
//! system menus are other tasks: never touched.
//!
//! The toggle is ours alone: KH1 showing or hiding its pieces (events) doesn't change it, and
//! nothing of KH1's is written, so nothing can be left hidden: a press shows it all again.

use super::addr::Rva;
use super::sys::*;
use super::{hook, log};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

/// Registers a task in the main task list `(priority, fn)` (`0x142867D80`; the node: `+8` next,
/// `+0x10` the function, called with the node, bit 2 of its return removes it).
const REGISTER_RVA: Rva = Rva(0x282500);
/// `mov r8, rdx; xor r9d, r9d; mov edx, ecx`.
const REGISTER_PROLOGUE: [u8; 8] = [0x4C, 0x8B, 0xC2, 0x45, 0x33, 0xC9, 0x8B, 0xD1];
/// Finds a task by its function `(list, fn)` (KH1's "is it there yet" and its kill by function).
const FIND_RVA: Rva = Rva(0x288750);
/// `mov rax, [rcx + 0x10]; test rax, rax` (the `je` after it reads these flags).
const FIND_PROLOGUE: [u8; 7] = [0x48, 0x8B, 0x41, 0x10, 0x48, 0x85, 0xC0];
/// KH1's immediate draw `(renderer, count, ...)` (`0x1400FBB60` and `0x1400FBCF0` come here).
const DRAW_RVA: Rva = Rva(0xFBBA0);
/// `mov [rsp + 0x10], rbx`.
const DRAW_PROLOGUE: [u8; 5] = [0x48, 0x89, 0x5C, 0x24, 0x10];

/// The HUD module's tasks (priority 165001 / 170001).
const TASKS: [Rva; 16] = [
    Rva(0x265F80),
    Rva(0x266330),
    Rva(0x26B990), // the HP/MP gauges (reads the event flag; `gaugehud` draws after it)
    Rva(0x26DD10),
    Rva(0x26E4B0),
    Rva(0x26EE10),
    Rva(0x26F880),
    Rva(0x2701C0),
    Rva(0x270DA0),
    Rva(0x271860), // the info messages (their queue `0x14284E360`)
    Rva(0x272290), // the lock-on cursor (`0x1402729B0`)
    Rva(0x2723D0),
    Rva(0x272660), // the target's gauge (lock state `0x142DB3F10`)
    Rva(0x273740),
    Rva(0x273990),
    Rva(0x279700), // the command menu, its lists and the shortcuts
];

static ORIG_REGISTER: AtomicUsize = AtomicUsize::new(0);
static ORIG_FIND: AtomicUsize = AtomicUsize::new(0);
static ORIG_DRAW: AtomicUsize = AtomicUsize::new(0);
/// Each task's function in the running build.
static FUNCS: [AtomicUsize; 16] = [const { AtomicUsize::new(0) }; 16];
/// Our stub for each (`mov edx, i; jmp task`).
static STUBS: [AtomicUsize; 16] = [const { AtomicUsize::new(0) }; 16];
/// All three hooks are in place.
static READY: AtomicBool = AtomicBool::new(false);
/// The HUD is hidden (the toggle).
static HIDDEN: AtomicBool = AtomicBool::new(false);
/// The thread running a HUD task while the HUD is hidden (0: none): its draws are emptied.
static HUD_THREAD: AtomicU32 = AtomicU32::new(0);
/// The toggle key was down at the last look.
static WAS_DOWN: AtomicBool = AtomicBool::new(false);

fn base() -> usize {
    unsafe { GetModuleHandleA(std::ptr::null()) as usize }
}

/// The stub standing for KH1's task function `f`, if `f` is one of the HUD's.
fn stub_for(f: usize) -> Option<usize> {
    (0..TASKS.len()).find(|&i| FUNCS[i].load(Ordering::Relaxed) == f).map(|i| STUBS[i].load(Ordering::Relaxed)).filter(|&s| s != 0)
}

/// Writes the stubs and hooks the task list's register and find and the immediate draw.
pub unsafe fn install() {
    let Some(vk) = super::settings::get().hud_toggle_key else {
        log("hud_toggle_key = off: no HUD toggle");
        return;
    };
    let b = base();
    let Some(mem) = hook::alloc_near(b + DRAW_RVA, 32 * TASKS.len()) else {
        log("HUD toggle: no memory for its stubs; not installed");
        return;
    };
    for (i, rva) in TASKS.iter().enumerate() {
        FUNCS[i].store(b + *rva, Ordering::Relaxed);
        // mov edx, i; jmp [rip + 0]; dq hud_task
        let s = mem + 32 * i;
        let mut code = [0u8; 19];
        code[0] = 0xBA;
        code[1..5].copy_from_slice(&(i as u32).to_le_bytes());
        code[5] = 0xFF;
        code[6] = 0x25;
        code[11..19].copy_from_slice(&(hud_task as *const () as u64).to_le_bytes());
        std::ptr::copy_nonoverlapping(code.as_ptr(), s as *mut u8, code.len());
        STUBS[i].store(s, Ordering::Relaxed);
    }
    FlushInstructionCache(GetCurrentProcess(), mem as *const _, 32 * TASKS.len());
    if hook::install(b + DRAW_RVA, &DRAW_PROLOGUE, draw as *const () as usize, &ORIG_DRAW, "immediate draw (HUD toggle)").is_none() {
        return;
    }
    if hook::install(b + FIND_RVA, &FIND_PROLOGUE, find as *const () as usize, &ORIG_FIND, "task find (HUD toggle)").is_none() {
        return;
    }
    if hook::install(b + REGISTER_RVA, &REGISTER_PROLOGUE, register as *const () as usize, &ORIG_REGISTER, "task register (HUD toggle)").is_none() {
        return;
    }
    READY.store(true, Ordering::Release);
    log(&format!("HUD toggle: {} hides and shows the HUD", crate::keys::name(vk)));
}

/// KH1 registering a task: a HUD task goes in as its stub.
unsafe extern "system" fn register(priority: u32, f: usize) -> usize {
    let orig: unsafe extern "system" fn(u32, usize) -> usize = std::mem::transmute(ORIG_REGISTER.load(Ordering::Acquire));
    orig(priority, stub_for(f).unwrap_or(f))
}

/// KH1 looking a task up by its function: for a HUD task, its stub (or KH1's own function, for
/// one registered before the hooks were in).
unsafe extern "system" fn find(list: usize, f: usize) -> usize {
    let orig: unsafe extern "system" fn(usize, usize) -> usize = std::mem::transmute(ORIG_FIND.load(Ordering::Acquire));
    if let Some(s) = stub_for(f) {
        let node = orig(list, s);
        if node != 0 {
            return node;
        }
    }
    orig(list, f)
}

/// A HUD task's turn (from its stub, `i` its place in `TASKS`): KH1's function runs as ever; with
/// the HUD hidden, the draws it makes meanwhile on this thread are emptied.
unsafe extern "system" fn hud_task(node: usize, i: usize) -> usize {
    let f: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(FUNCS[i].load(Ordering::Relaxed));
    if !HIDDEN.load(Ordering::Relaxed) {
        return f(node);
    }
    HUD_THREAD.store(GetCurrentThreadId(), Ordering::Relaxed);
    let r = f(node);
    HUD_THREAD.store(0, Ordering::Relaxed);
    r
}

/// KH1's immediate draw: inside a HUD task with the HUD hidden, drawn with no vertices.
unsafe extern "system" fn draw(renderer: usize, count: usize, a: usize, b: usize) -> usize {
    let orig: unsafe extern "system" fn(usize, usize, usize, usize) -> usize = std::mem::transmute(ORIG_DRAW.load(Ordering::Acquire));
    let me = GetCurrentThreadId();
    let t = HUD_THREAD.load(Ordering::Relaxed);
    let asked = count;
    let count = if t != 0 && t == me { 0 } else { count };
    if COUNT_THREAD.load(Ordering::Relaxed) == me {
        ASKED.fetch_add(asked as u32, Ordering::Relaxed);
        DRAWN.fetch_add(count as u32, Ordering::Relaxed);
    }
    orig(renderer, count, a, b)
}

/// Counting the vertices a piece of ours hands KH1's immediate draw (`gaugehud`'s proof in the
/// log): the thread counted, and what it asked for and what was drawn (0 with the HUD hidden).
static COUNT_THREAD: AtomicU32 = AtomicU32::new(0);
static ASKED: AtomicU32 = AtomicU32::new(0);
static DRAWN: AtomicU32 = AtomicU32::new(0);

/// Starts counting this thread's immediate draws; `false` without the draw hook (no count).
pub fn count_start() -> bool {
    ASKED.store(0, Ordering::Relaxed);
    DRAWN.store(0, Ordering::Relaxed);
    COUNT_THREAD.store(unsafe { GetCurrentThreadId() }, Ordering::Relaxed);
    READY.load(Ordering::Acquire)
}

/// Stops counting: the vertices asked for and drawn since `count_start`.
pub fn count_end() -> (u32, u32) {
    COUNT_THREAD.store(0, Ordering::Relaxed);
    (ASKED.load(Ordering::Relaxed), DRAWN.load(Ordering::Relaxed))
}

/// The toggle key (the monitor thread, every 50 ms): a press while the game has the keyboard
/// flips the HUD once; holding it does nothing more.
pub fn poll() {
    let Some(vk) = super::settings::get().hud_toggle_key else {
        return;
    };
    let down = unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 } && game_focused();
    if !down || WAS_DOWN.swap(down, Ordering::Relaxed) {
        WAS_DOWN.store(down, Ordering::Relaxed);
        return;
    }
    if !READY.load(Ordering::Acquire) {
        log(&format!("{}: the HUD toggle isn't in place (see the hook lines above)", crate::keys::name(vk)));
        return;
    }
    let hidden = !HIDDEN.load(Ordering::Relaxed);
    HIDDEN.store(hidden, Ordering::Relaxed);
    log(if hidden { "HUD hidden" } else { "HUD shown" });
}
