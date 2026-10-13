//! Stage 7 probe, for KH2's controller taking Sora over: his actor, KH1's pad record, and
//! which KH1 code moves him.
//!
//! - `0x14029CF10` `(actor, dt f32)` updates an actor's animation (it calls the skeleton
//!   passes); `actor + 0x15C` is the handle of its animation instance, so the actor whose
//!   instance is Sora's is his. Its fields holding his position are found by searching it for
//!   the placement the skeleton pass draws him at (`pose::sora_world`).
//! - Hardware write watchpoints (debug registers, on every thread of the game) on that
//!   position name the code that writes it: KH1's movement, gravity and collision. Logged by
//!   address for 25 seconds, then removed.
//! - `0x140112C70` `(device state, PADDATA out, config)` is where every controller (XInput,
//!   DualShock / DualSense over HID, ...) becomes KH1's PS2-style pad record: `+0` buttons u16
//!   (PS2 order: Select, L3, R3, Start, Up, Right, Down, Left, L2, R2, L1, R1, Triangle,
//!   Circle, Cross, Square), `+2..+5` the four stick bytes. Logged on change.

use super::sys::*;
use super::{hook, log, pose};
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

/// The position watch ran (2026-10-13: `0x140386D00` from `0x140299350`, the collision mover);
/// off now.
const WATCH_ON: bool = false;
const ACTOR_RVA: usize = 0x29CF10;
const ACTOR_PROLOGUE: [u8; 8] = [0x40, 0x55, 0x57, 0x48, 0x8D, 0x6C, 0x24, 0xE8];
const PAD_RVA: usize = 0x112C70;
const PAD_PROLOGUE: [u8; 5] = [0x48, 0x89, 0x5C, 0x24, 0x08];

static ORIG_ACTOR: AtomicUsize = AtomicUsize::new(0);
static ORIG_PAD: AtomicUsize = AtomicUsize::new(0);
pub static SORA_ACTOR: AtomicUsize = AtomicUsize::new(0);
static ACTOR_CALLS: AtomicU32 = AtomicU32::new(0);
/// Offset in the actor of the first position-like triple found.
static POS_OFF: AtomicUsize = AtomicUsize::new(0);
static BASE: AtomicUsize = AtomicUsize::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

type ActorFn = unsafe extern "system" fn(usize, f32);
type PadFn = unsafe extern "system" fn(usize, usize, usize);

unsafe extern "system" fn actor(a: usize, dt: f32) {
    let f: ActorFn = std::mem::transmute(ORIG_ACTOR.load(Ordering::Acquire));
    f(a, dt);
    if a < 0x10000 {
        return;
    }
    let inst = pose::ptr(rd::<u32>(a + 0x15C));
    if inst == 0 || inst != pose::SORA_INSTANCE.load(Ordering::Relaxed) {
        return;
    }
    if SORA_ACTOR.swap(a, Ordering::Relaxed) != a {
        log(&format!("Sora's actor at {a:#x} (dt {dt:.3})"));
        dump(a, "first seen");
    }
    let n = ACTOR_CALLS.fetch_add(1, Ordering::Relaxed);
    if n % 120 == 60 && n < 120 * 12 {
        find_position(a);
    }
    if n == 120 * 8 {
        dump(a, "8 s later");
    }
}

/// The actor's first 0x600 bytes as u32 words (for comparing two moments).
unsafe fn dump(a: usize, when: &str) {
    let words: Vec<String> = (0..0x180).map(|i| format!("{:08x}", rd::<u32>(a + i * 4))).collect();
    log(&format!("Sora's actor {when}: {}", words.join(" ")));
}

/// Offsets in the actor holding the placement the skeleton pass draws him at.
unsafe fn find_position(a: usize) {
    let Some(w) = pose::sora_world() else { return };
    let mut hits = Vec::new();
    for off in (0..0x1000).step_by(4) {
        let v: [f32; 3] = rd(a + off);
        if v.iter().all(|x| x.is_finite()) && (v[0] - w[0]).abs() < 0.5 && (v[1] - w[1]).abs() < 0.5 && (v[2] - w[2]).abs() < 0.5 {
            hits.push(off);
        }
    }
    // Also with y flipped (the pass flips the world's y), and x/z only.
    let mut flat = Vec::new();
    for off in (0..0x1000).step_by(4) {
        let v: [f32; 3] = rd(a + off);
        if v.iter().all(|x| x.is_finite()) && (v[0] - w[0]).abs() < 0.5 && (v[2] - w[2]).abs() < 0.5 {
            flat.push((off, v[1]));
        }
    }
    log(&format!("drawn at {w:.1?}: exact at actor+{:x?}; same x/z at {:x?}", hits, flat.iter().map(|(o, y)| format!("{o:x} (y {y:.1})")).collect::<Vec<_>>()));
    if POS_OFF.load(Ordering::Relaxed) == 0 {
        if let Some(&o) = hits.first().or(flat.first().map(|(o, _)| o)) {
            POS_OFF.store(o, Ordering::Relaxed);
        }
    }
}

static PAD_LOGGED: AtomicU32 = AtomicU32::new(0);
static PAD_LAST: AtomicU64 = AtomicU64::new(0);

unsafe extern "system" fn pad(src: usize, out: usize, cfg: usize) {
    let f: PadFn = std::mem::transmute(ORIG_PAD.load(Ordering::Acquire));
    f(src, out, cfg);
    if out < 0x10000 {
        return;
    }
    let b: u16 = rd(out);
    let st: [u8; 4] = rd(out + 2);
    // The pad as pressed goes to KH2; while KH2 has Sora, KH1 doesn't see its buttons.
    super::takeover::RAW_BUTTONS.store(b as u32, Ordering::Relaxed);
    super::takeover::RAW_STICKS.store(u32::from_le_bytes(st), Ordering::Relaxed);
    if super::takeover::active() {
        std::ptr::write_unaligned(out as *mut u16, b & !(super::takeover::CIRCLE | super::takeover::SQUARE));
    }
    // Sticks to 3 levels each, so drift doesn't flood the log.
    let q = |v: u8| -> u64 { if v < 0x50 { 0 } else if v > 0xB0 { 2 } else { 1 } };
    let key = b as u64 | q(st[0]) << 16 | q(st[1]) << 18 | q(st[2]) << 20 | q(st[3]) << 22 | ((out as u64 & 0xFFFF) << 32);
    if PAD_LAST.swap(key, Ordering::Relaxed) != key && PAD_LOGGED.fetch_add(1, Ordering::Relaxed) < 120 {
        const NAMES: [&str; 16] = ["Select", "L3", "R3", "Start", "Up", "Right", "Down", "Left", "L2", "R2", "L1", "R1", "Triangle", "Circle", "Cross", "Square"];
        let held: Vec<&str> = (0..16).filter(|i| b & (1 << i) != 0).map(|i| NAMES[i]).collect();
        log(&format!("pad {out:#x}: buttons {b:04x} {held:?}, sticks {:02x} {:02x} {:02x} {:02x}", st[0], st[1], st[2], st[3]));
    }
}

// ---- hardware write watchpoints ----

const SLOTS: usize = 48;
static HIT_RIP: [AtomicU64; SLOTS] = [const { AtomicU64::new(0) }; SLOTS];
static HIT_N: [AtomicU32; SLOTS] = [const { AtomicU32::new(0) }; SLOTS];
static HIT_WHICH: [AtomicU32; SLOTS] = [const { AtomicU32::new(0) }; SLOTS];

#[repr(C, align(16))]
struct Context([u8; 1232]);
const CTX_FLAGS: usize = 0x30;
const CTX_DR0: usize = 0x48;
const CTX_DR6: usize = 0x68;
const CTX_DR7: usize = 0x70;
const CONTEXT_DEBUG_REGISTERS: u32 = 0x0010_0010;

unsafe extern "system" fn on_exception(info: *mut c_void) -> i32 {
    let rec = *(info as *const usize);
    let ctx = *((info as *const usize).add(1));
    if rec == 0 || ctx == 0 || rd::<u32>(rec) != 0x8000_0004 {
        return 0;
    }
    let dr6: u64 = rd(ctx + CTX_DR6);
    if dr6 & 0x7 == 0 {
        return 0;
    }
    let rip: u64 = rd(rec + 0x10);
    let which = (dr6 & 0x7) as u32;
    let mut i = (rip as usize ^ (rip as usize >> 7)) % SLOTS;
    for _ in 0..SLOTS {
        let cur = HIT_RIP[i].load(Ordering::Relaxed);
        if cur == rip || (cur == 0 && HIT_RIP[i].compare_exchange(0, rip, Ordering::Relaxed, Ordering::Relaxed).is_ok()) {
            HIT_N[i].fetch_add(1, Ordering::Relaxed);
            HIT_WHICH[i].fetch_or(which, Ordering::Relaxed);
            break;
        }
        i = (i + 1) % SLOTS;
    }
    std::ptr::write_unaligned((ctx + CTX_DR6) as *mut u64, 0);
    -1
}

#[repr(C)]
struct ThreadEntry {
    size: u32,
    usage: u32,
    id: u32,
    owner: u32,
    base_pri: i32,
    delta_pri: i32,
    flags: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> *mut c_void;
    fn Thread32First(snap: *mut c_void, e: *mut ThreadEntry) -> i32;
    fn Thread32Next(snap: *mut c_void, e: *mut ThreadEntry) -> i32;
    fn GetCurrentProcessId() -> u32;
}

/// Sets (or clears, with `addr` 0) write watchpoints on `addr`, `+4`, `+8` on every other
/// thread of the game.
unsafe fn watch(addr: usize) -> u32 {
    let me = GetCurrentThreadId();
    let pid = GetCurrentProcessId();
    let snap = CreateToolhelp32Snapshot(4, 0);
    if snap.is_null() || snap as isize == -1 {
        return 0;
    }
    let mut e = ThreadEntry { size: std::mem::size_of::<ThreadEntry>() as u32, usage: 0, id: 0, owner: 0, base_pri: 0, delta_pri: 0, flags: 0 };
    let mut n = 0;
    let mut ok = Thread32First(snap, &mut e);
    while ok != 0 {
        if e.owner == pid && e.id != me {
            let t = OpenThread(0x1A, 0, e.id);
            if !t.is_null() {
                let mut c = Context([0; 1232]);
                std::ptr::write_unaligned(c.0.as_mut_ptr().add(CTX_FLAGS) as *mut u32, CONTEXT_DEBUG_REGISTERS);
                SuspendThread(t);
                if GetThreadContext(t, c.0.as_mut_ptr() as *mut c_void) != 0 {
                    let regs: [u64; 3] = if addr == 0 { [0; 3] } else { [addr as u64, addr as u64 + 4, addr as u64 + 8] };
                    for (k, r) in regs.iter().enumerate() {
                        std::ptr::write_unaligned(c.0.as_mut_ptr().add(CTX_DR0 + k * 8) as *mut u64, *r);
                    }
                    // L0-L2, write-only, 4 bytes each.
                    let dr7: u64 = if addr == 0 { 0 } else { 0x15 | 0xD << 16 | 0xD << 20 | 0xD << 24 };
                    std::ptr::write_unaligned(c.0.as_mut_ptr().add(CTX_DR7) as *mut u64, dr7);
                    std::ptr::write_unaligned(c.0.as_mut_ptr().add(CTX_DR6) as *mut u64, 0);
                    if SetThreadContext(t, c.0.as_ptr() as *const c_void) != 0 {
                        n += 1;
                    }
                }
                ResumeThread(t);
                CloseHandle(t);
            }
        }
        ok = Thread32Next(snap, &mut e);
    }
    CloseHandle(snap);
    n
}

fn report(label: &str) {
    let base = BASE.load(Ordering::Relaxed) as u64;
    let mut v: Vec<(u64, u32, u32)> = (0..SLOTS)
        .filter_map(|i| {
            let r = HIT_RIP[i].load(Ordering::Relaxed);
            (r != 0).then(|| (r, HIT_N[i].load(Ordering::Relaxed), HIT_WHICH[i].load(Ordering::Relaxed)))
        })
        .collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    let s: Vec<String> = v.iter().take(24).map(|(r, n, w)| {
        let what = format!("{}{}{}", if w & 1 != 0 { "x" } else { "" }, if w & 2 != 0 { "y" } else { "" }, if w & 4 != 0 { "z" } else { "" });
        if *r >= base && *r < base + 0x400_0000 { format!("exe+{:x} ({what}) x{n}", r - base) } else { format!("{r:#x} ({what}) x{n}") }
    }).collect();
    log(&format!("writers of Sora's position ({label}): {}", s.join(", ")));
}

/// The monitor thread's part: arm the watchpoints once the position is known, report, disarm.
pub unsafe fn tick(seconds: f32) {
    static ARMED_AT: AtomicU32 = AtomicU32::new(0);
    static DONE: AtomicU32 = AtomicU32::new(0);
    if DONE.load(Ordering::Relaxed) != 0 {
        return;
    }
    let a = SORA_ACTOR.load(Ordering::Relaxed);
    let off = POS_OFF.load(Ordering::Relaxed);
    let armed = ARMED_AT.load(Ordering::Relaxed);
    if WATCH_ON && armed == 0 && a != 0 && off != 0 && seconds > 20.0 {
        AddVectoredExceptionHandler(1, on_exception);
        let n = watch(a + off);
        ARMED_AT.store(seconds as u32, Ordering::Relaxed);
        log(&format!("watching actor+{off:x} (Sora's position) for writes on {n} threads, 25 s: walk, run, jump, fall off something, attack"));
    } else if armed != 0 {
        let t = seconds as u32 - armed;
        if t > 0 && t % 5 == 0 && t <= 25 {
            static LAST: AtomicU32 = AtomicU32::new(0);
            if LAST.swap(t, Ordering::Relaxed) != t {
                report(&format!("{t} s"));
            }
        }
        if t >= 25 {
            watch(0);
            DONE.store(1, Ordering::Relaxed);
            log("position watch removed");
        }
    }
}

pub unsafe fn install() {
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + ACTOR_RVA, &ACTOR_PROLOGUE, actor as *const () as usize, &ORIG_ACTOR, "actor animation update");
    hook::install(base + PAD_RVA, &PAD_PROLOGUE, pad as *const () as usize, &ORIG_PAD, "pad record");
}
