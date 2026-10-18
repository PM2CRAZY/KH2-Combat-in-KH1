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
//! - Combat probe (stage 9): KH1's set-motion functions (`0x14029D630`, `0x14029D7F0`,
//!   `0x14029D8A0`, `(actor, slot | flags, ...)`) log the code asking for Sora's attack slots
//!   (return addresses found on the stack); `0x1402BD180` `(attack, target)` (an attack touched
//!   an actor) dumps Sora's attack records and the target's stats; the live actors (through
//!   `0x14028F990`, the actor iterator) are listed every few seconds.
//! - `0x140112C70` `(device state, PADDATA out, config)` is where every controller (XInput,
//!   DualShock / DualSense over HID, ...) becomes KH1's PS2-style pad record: `+0` buttons u16
//!   (PS2 order: Select, L3, R3, Start, Up, Right, Down, Left, L2, R2, L1, R1, Triangle,
//!   Circle, Cross, Square), `+2..+5` the four stick bytes. Logged on change.

use super::addr::Rva;
use super::sys::*;
use super::{hook, log, pose};
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

/// The position watch ran (2026-10-13: `0x140386D00` from `0x140299350`, the collision mover);
/// off now.
const WATCH_ON: bool = false;
const ACTOR_RVA: Rva = Rva(0x29CF10);
const ACTOR_PROLOGUE: [u8; 8] = [0x40, 0x55, 0x57, 0x48, 0x8D, 0x6C, 0x24, 0xE8];
const PAD_RVA: Rva = Rva(0x112C70);
const PAD_PROLOGUE: [u8; 5] = [0x48, 0x89, 0x5C, 0x24, 0x08];

static ORIG_ACTOR: AtomicUsize = AtomicUsize::new(0);
static ORIG_SETM: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];
static ORIG_HIT: AtomicUsize = AtomicUsize::new(0);
const SETM_RVA: [Rva; 3] = [Rva(0x29D630), Rva(0x29D7F0), Rva(0x29D8A0)];
const SETM_PROLOGUE: [u8; 5] = [0x48, 0x89, 0x5C, 0x24, 0x08];
const HIT_RVA: Rva = Rva(0x2BD180);
const HIT_PROLOGUE: [u8; 7] = [0x40, 0x55, 0x57, 0x48, 0x83, 0xEC, 0x48];
const ACTORS_RVA: Rva = Rva(0x28F990);
const PARTY_RVA: Rva = Rva(0x2D37C80);
/// Sora's attack record as KH1 last made it (0xC0 bytes), for KH2's hits.
pub static ATTACK_TEMPLATE: std::sync::Mutex<Option<Vec<u8>>> = std::sync::Mutex::new(None);
static ORIG_PAD: AtomicUsize = AtomicUsize::new(0);

/// KH1's event script dispatcher (`0x1401B13F0 (context, opcode)`: the handler table at
/// `0x1404FB550`, 764 opcodes). Traced to see what a waiting script (a tutorial window) polls.
const EVENT_OP_RVA: Rva = Rva(0x1B13F0);
const EVENT_OP_PROLOGUE: [u8; 6] = [0x40, 0x57, 0x48, 0x83, 0xEC, 0x30];
static ORIG_EVENT_OP: AtomicUsize = AtomicUsize::new(0);
const EVENT_OPS: usize = 768;
static EVENT_COUNTS: [AtomicU32; EVENT_OPS] = [const { AtomicU32::new(0) }; EVENT_OPS];

unsafe extern "system" fn event_op(ctx: usize, op: i32, c: usize, d: usize) -> u64 {
    if (0..EVENT_OPS as i32).contains(&op) {
        EVENT_COUNTS[op as usize].fetch_add(1, Ordering::Relaxed);
    }
    // Opcode 322 (`0x1401AB6E0`) pushes the script actor's motion request (`+0x164`): note
    // each new value (a tutorial waits on it).
    if op == 322 && ctx > 0x10000 {
        static LAST: AtomicU64 = AtomicU64::new(u64::MAX);
        static N: AtomicU32 = AtomicU32::new(0);
        let actor = rd::<u64>(ctx + 8) as usize;
        if actor > 0x10000 {
            let v = rd::<u32>(actor + 0x164);
            let key = (actor as u64) << 16 ^ v as u64;
            if LAST.swap(key, Ordering::Relaxed) != key && N.fetch_add(1, Ordering::Relaxed) < 200 {
                let sora = rd::<u64>(BASE.load(Ordering::Relaxed) + PARTY_RVA) as usize;
                log(&format!("event op 322: actor {actor:#x}{} motion request {v:#x} (state {}, takeover {})", if actor == sora { " (Sora)" } else { "" }, rd::<i32>(actor + 0x70), super::takeover::ON.load(Ordering::Relaxed)));
            }
        }
    }
    let f: unsafe extern "system" fn(usize, i32, usize, usize) -> u64 = std::mem::transmute(ORIG_EVENT_OP.load(Ordering::Acquire));
    let r = f(ctx, op, c, d);
    // Opcode 322 just pushed the script actor's `+0x164` (stack: count `+0x194`, slots from
    // `+0x198`). For Sora, right after a KH2 attack: the motion KH1's swing would show
    // (`combat::attack_seen`), so scripts waiting for an attack (tutorials) see one.
    if op == 322 && ctx > 0x10000 {
        let actor = rd::<u64>(ctx + 8) as usize;
        let sora = rd::<u64>(BASE.load(Ordering::Relaxed) + PARTY_RVA) as usize;
        let top = rd::<i32>(ctx + 0x194);
        if actor > 0x10000 && actor == sora && (0..32).contains(&top) {
            if let Some(m) = super::combat::attack_seen() {
                std::ptr::write_unaligned((ctx + 0x198 + top as usize * 4) as *mut u32, m);
                static N: AtomicU32 = AtomicU32::new(0);
                if N.fetch_add(1, Ordering::Relaxed) < 20 {
                    log(&format!("event op 322: Sora's motion shown to the script as {m:#x} (KH2 attacked; stack count {top})"));
                }
            }
        }
    }
    r
}

/// Once a second: the event opcodes run in that second, when they differ from the last second's
/// (a script waiting on a condition runs the same few every frame).
fn event_trace(t: f32) {
    use std::sync::Mutex;
    static LAST: Mutex<Vec<usize>> = Mutex::new(Vec::new());
    static LINES: AtomicU32 = AtomicU32::new(0);
    static LAST_T: AtomicU32 = AtomicU32::new(0);
    let sec = t as u32;
    if LAST_T.swap(sec, Ordering::Relaxed) == sec {
        return;
    }
    let mut ops = Vec::new();
    let mut text = String::new();
    for (i, c) in EVENT_COUNTS.iter().enumerate() {
        let n = c.swap(0, Ordering::Relaxed);
        if n > 0 {
            ops.push(i);
            text.push_str(&format!(" {i}x{n}"));
        }
    }
    let Ok(mut last) = LAST.lock() else { return };
    if ops != *last && !ops.is_empty() && LINES.fetch_add(1, Ordering::Relaxed) < 600 {
        let base = BASE.load(Ordering::Relaxed);
        let sora = unsafe { rd::<u64>(base + PARTY_RVA) } as usize;
        let free = sora > 0x10000 && unsafe { rd::<u32>(sora + 0x444) } & 1 == 0;
        log(&format!("event ops at {t:.0}s (Sora can act: {free}):{text}"));
    }
    *last = ops;
}
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
    if n % 240 == 120 && n < 240 * 10 {
        list_actors();
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

/// The pads' records this moment, merged: buttons or'ed, each stick from the record that has it
/// furthest from the centre.
fn merge_pads(out: usize, b: u16, st: [u8; 4]) -> (u16, [u8; 4]) {
    use std::sync::Mutex;
    static PADS: Mutex<Vec<(usize, u16, [u8; 4], std::time::Instant)>> = Mutex::new(Vec::new());
    static SEEN: AtomicU32 = AtomicU32::new(0);
    let Ok(mut pads) = PADS.lock() else { return (b, st) };
    let now = std::time::Instant::now();
    match pads.iter_mut().find(|p| p.0 == out) {
        Some(p) => {
            p.1 = b;
            p.2 = st;
            p.3 = now;
        }
        None => {
            pads.push((out, b, st, now));
            // (Counted once per record: one going quiet a moment and coming back isn't new.)
            static EVER: Mutex<Vec<usize>> = Mutex::new(Vec::new());
            if let Ok(mut ever) = EVER.lock() {
                if !ever.contains(&out) {
                    ever.push(out);
                    let n = ever.len();
                    let _ = SEEN.fetch_add(1, Ordering::Relaxed);
                    if n > 1 && n <= 8 {
                        log(&format!("pad records: {n} seen ({out:#x} new): merged for KH2, as KH1 merges them"));
                    }
                }
            }
        }
    }
    pads.retain(|p| now.duration_since(p.3).as_millis() < 100);
    let mut buttons = 0u16;
    let mut sticks = [0x80u8; 4];
    let off = |v: u8| (v as i32 - 128).abs();
    let (mut right, mut left) = (-1, -1);
    for p in pads.iter() {
        buttons |= p.1;
        let r = off(p.2[0]).max(off(p.2[1]));
        if r > right {
            right = r;
            sticks[0] = p.2[0];
            sticks[1] = p.2[1];
        }
        let l = off(p.2[2]).max(off(p.2[3]));
        if l > left {
            left = l;
            sticks[2] = p.2[2];
            sticks[3] = p.2[3];
        }
    }
    (buttons, sticks)
}

unsafe extern "system" fn pad(src: usize, out: usize, cfg: usize) {
    let f: PadFn = std::mem::transmute(ORIG_PAD.load(Ordering::Acquire));
    f(src, out, cfg);
    if out < 0x10000 {
        return;
    }
    let b: u16 = rd(out);
    let st: [u8; 4] = rd(out + 2);
    // The pad as pressed goes to KH2; while KH2 has Sora, KH1 doesn't see its buttons. KH1 reads
    // a record per pad (Steam Input's virtual pad beside the real one, a keyboard...) and
    // merges them: so does KH2, every record seen in the last 100 ms (the user's tester: two
    // records, KH2 kept the idle one's and got no input at all while KH1 moved him).
    let (b_all, st_all) = merge_pads(out, b, st);
    super::takeover::RAW_BUTTONS.store(b_all as u32, Ordering::Relaxed);
    super::takeover::RAW_STICKS.store(u32::from_le_bytes(st_all), Ordering::Relaxed);
    // L1 held: KH1's shortcuts (L1 + a face button cast the spells set there), so KH1 keeps
    // Circle and Square then (the user: the Square shortcut didn't cast).
    // (Only when L1 opens KH1's shortcut menu: with no shortcut set it opens nothing, and KH1
    // jumped with the button, cancelling KH2's moves, the user.)
    // (And not while a command submenu is open: Circle backs out of it, KH1's.)
    // (A submenu keeps only its back-out button: Square there was KH1's own guard, in the air
    // too, spec 3.2.)
    let l1_menu = super::shortcuts::menu_on_l1(b_all);
    let shortcuts = l1_menu || super::shortcuts::submenu_open();
    if super::takeover::active() && !shortcuts {
        std::ptr::write_unaligned(out as *mut u16, b & !(super::takeover::jump_button() | super::takeover::SQUARE));
    } else if super::takeover::active() && !l1_menu {
        std::ptr::write_unaligned(out as *mut u16, b & !super::takeover::SQUARE);
    } else if !super::takeover::kh1_keeps_square() && !shortcuts {
        std::ptr::write_unaligned(out as *mut u16, b & !super::takeover::SQUARE);
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
    event_trace(seconds);
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

fn sora() -> usize {
    unsafe { rd::<u64>(BASE.load(Ordering::Relaxed) + PARTY_RVA) as usize }
}

/// Code addresses in the executable found on the stack above `here` (return addresses, most
/// recent first; a few may be stale).
unsafe fn callers(here: usize) -> String {
    let base = BASE.load(Ordering::Relaxed);
    let mut v = Vec::new();
    for i in 0..0x100 {
        // The stack may end within the scan (the crash going through a door, stage 12).
        let at = here + i * 8;
        if (i == 0 || at & 0xFFF == 0) && !super::crash::readable(at) {
            break;
        }
        let x: u64 = rd(at);
        let x = x as usize;
        if x > base + 0x1000 && x < base + 0x3B_0000 && !v.contains(&(x - base)) {
            v.push(x - base);
            if v.len() >= 10 {
                break;
            }
        }
    }
    v.iter().map(|r| format!("exe+{r:x}")).collect::<Vec<_>>().join(" ")
}

static SETM_LOGGED: AtomicU32 = AtomicU32::new(0);
/// KH1 set a motion on Sora (any slot), counted (`combat::command` sees whether a command did).
pub static SORA_MOTIONS: AtomicU32 = AtomicU32::new(0);
/// The last slot KH1 set on Sora.
pub static SORA_LAST_SLOT: AtomicU32 = AtomicU32::new(0);

unsafe fn setm_note(which: usize, actor: usize, req: u32, here: usize) {
    if actor == 0 || actor != sora() {
        return;
    }
    SORA_MOTIONS.fetch_add(1, Ordering::Relaxed);
    SORA_LAST_SLOT.store(req, Ordering::Relaxed);
    let slot = req & 0xFFFF;
    // Not standing / walking / running (the slots KH2 has).
    if slot <= 7 || SETM_LOGGED.load(Ordering::Relaxed) >= 80 {
        return;
    }
    SETM_LOGGED.fetch_add(1, Ordering::Relaxed);
    log(&format!("set motion #{which} slot {slot} (flags {:#x}) on Sora, from {}", req & 0xFFFF_0000, callers(here)));
}

/// `0x14029D7F0` / `0x14029D8A0` take a float third too (XMM2, stored at the actor's `+0x184`),
/// and return a pointer their callers use (the hook audit of stage 49: the hooks had it as an
/// integer, so KH1 got whatever XMM2 held).
type SetmF = unsafe extern "system" fn(usize, u32, f32) -> usize;

/// `0x14029D630` takes the blend time as a float (third argument, in XMM2).
type SetmBlendF = unsafe extern "system" fn(usize, u32, f32);

unsafe extern "system" fn setm0(a: usize, req: u32, blend: f32) {
    let here = &a as *const usize as usize;
    setm_note(0, a, req, here);
    let f: SetmBlendF = std::mem::transmute(ORIG_SETM[0].load(Ordering::Acquire));
    f(a, req, blend)
}
unsafe extern "system" fn setm1(a: usize, req: u32, c: f32) -> usize {
    let here = &a as *const usize as usize;
    setm_note(1, a, req, here);
    let f: SetmF = std::mem::transmute(ORIG_SETM[1].load(Ordering::Acquire));
    f(a, req, c)
}
unsafe extern "system" fn setm2(a: usize, req: u32, c: f32) -> usize {
    let here = &a as *const usize as usize;
    setm_note(2, a, req, here);
    let f: SetmF = std::mem::transmute(ORIG_SETM[2].load(Ordering::Acquire));
    f(a, req, c)
}

static HIT_LOGGED: AtomicU32 = AtomicU32::new(0);
type HitF = unsafe extern "system" fn(usize, usize) -> usize;

unsafe extern "system" fn hit(attack: usize, target: usize) -> usize {
    let f: HitF = std::mem::transmute(ORIG_HIT.load(Ordering::Acquire));
    let owner = if attack > 0x10000 { pose::ptr(rd::<u32>(attack + 0x34)) } else { 0 };
    let r = f(attack, target);
    if owner != 0 && owner == sora() && r > 0x10000 {
        super::combat::scale_record(r);
    }
    if owner != 0 && owner == sora() && target > 0x10000 && super::combat::POWER.load(Ordering::Relaxed) == 0 {
        let words = |p: usize, n: usize| (0..n).map(|i| format!("{:08x}", rd::<u32>(p + i * 4))).collect::<Vec<_>>().join(" ");
        if let Ok(mut t) = ATTACK_TEMPLATE.lock() {
            *t = Some(std::slice::from_raw_parts(attack as *const u8, 0xC0).to_vec());
        }
        if HIT_LOGGED.fetch_add(1, Ordering::Relaxed) < 24 {
            let stats = pose::ptr(rd::<u32>(target + 0x6C));
            let data = pose::ptr(rd::<u32>(attack + 0x5C));
            let rec = if r > 0x10000 { words(r, 16) } else { "none".into() };
            log(&format!(
                "Sora's attack {attack:#x} hit actor {target:#x} (kind {}, object {}) -> record [{rec}]; attack [{}]; attack data {data:#x} [{}]; target stats {stats:#x} [{}]",
                rd::<u8>(target + 6),
                object_id(target),
                words(attack, 0x30),
                if data > 0x10000 { words(data, 0x18) } else { "-".into() },
                if stats > 0x10000 { words(stats, 0x34) } else { "-".into() }
            ));
        }
    }
    r
}

pub unsafe fn object_id(a: usize) -> u32 {
    let o = pose::ptr(rd::<u32>(a + 0x130));
    if o > 0x10000 { rd::<u16>(o + 0x4C) as u32 } else { 0xFFFF }
}

/// The live actors, through KH1's iterator.
unsafe fn list_actors() {
    let next: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(BASE.load(Ordering::Relaxed) + ACTORS_RVA);
    let mut a = next(0);
    let mut lines = Vec::new();
    let mut n = 0;
    while a != 0 && n < 64 {
        n += 1;
        let p: [f32; 3] = rd(a + 0x10);
        let col = pose::ptr(rd::<u32>(a + 0x134));
        let radius = if col > 0x10000 && rd::<u8>(col + 5) != 0 {
            let sh = pose::ptr(rd::<u32>(col + 0x14));
            if sh > 0x10000 { rd::<f32>(sh + 0xC) } else { -1.0 }
        } else {
            -1.0
        };
        let stats = pose::ptr(rd::<u32>(a + 0x6C));
        let hp = if stats > 0x10000 { rd::<i32>(stats + 0x3C) } else { -1 };
        lines.push(format!("{a:#x} kind {} obj {} at ({:.0},{:.0},{:.0}) r {radius:.0} hp? {hp} flags {:08x}/{:08x}", rd::<u8>(a + 6), object_id(a), p[0], p[1], p[2], rd::<u32>(a), rd::<u32>(a + 0x374)));
        a = next(a);
    }
    log(&format!("actors ({n}): {}", lines.join(" | ")));
}

pub unsafe fn install() {
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    let fns = [setm0 as *const () as usize, setm1 as *const () as usize, setm2 as *const () as usize];
    for k in 0..3 {
        hook::install(base + SETM_RVA[k], &SETM_PROLOGUE, fns[k], &ORIG_SETM[k], "set motion");
    }
    hook::install(base + HIT_RVA, &HIT_PROLOGUE, hit as *const () as usize, &ORIG_HIT, "attack touched actor");
    hook::install(base + ACTOR_RVA, &ACTOR_PROLOGUE, actor as *const () as usize, &ORIG_ACTOR, "actor animation update");
    hook::install(base + PAD_RVA, &PAD_PROLOGUE, pad as *const () as usize, &ORIG_PAD, "pad record");
    hook::install(base + EVENT_OP_RVA, &EVENT_OP_PROLOGUE, event_op as *const () as usize, &ORIG_EVENT_OP, "event script opcodes (trace)");
}
