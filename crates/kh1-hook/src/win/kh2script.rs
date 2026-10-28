//! KH2's spell scripts as KH1 script processes (testing branch; the tester, thread
//! kh2-magic-and-reflect-native m12 question 1 "(a)": "Teach KH1's script machine KH2's
//! instruction set, so a KH2 spell runs as a KH1 script process"; `docs/kh1-magic-native.md` §8).
//!
//! KH1's script machine (Steam KH1, Checked (decomp)):
//! - A spell's script starts with KH1's spell start `0x14028F5E0 (actor, target, arg, effects,
//!   code, data, second data)`: a task in the spell task list `0x142D22D08` (stepped every frame by
//!   the task `0x14028F6B0`, which runs the list through `0x14028A710`), whose step `0x14028F6D0`
//!   steps the task's process with `0x1402BCEC0` while the game isn't paused (`0x142867364` bit
//!   0x4000) and ends the task (4) once the process is gone.
//! - The process comes from `0x1402BD420 (code)`: one of 64 records of 0x758 bytes at
//!   `0x142DC7BE0` (`0x1402BD5B0`) and one of 32 work slots of 0x800 bytes at `0x142DB7BE0`
//!   (in use: the bytes at `0x142DE51E0`). A record: `+0` flags (1 in use, 2 running, 4 a child,
//!   8 stopped), `+0x10` the program (the code it was started on), `+0x18` the pc, `+0x20` the stack
//!   pointer (its stack is the record's `+0x40..+0x440`, four zero words pushed first), `+0x28`
//!   the evaluation stack pointer (`+0x440..+0x740`), `+0x30` the work memory (the slot), `+0x38`
//!   the call tables, `+0x740` the next child.
//! - `0x1402BCEC0` runs the process, then each child, through the interpreter `0x1402CC620
//!   (record + 0x18, callback, record)`: 0 when the script halted for the frame, `0xFFFFFFFF` when it
//!   ended (the process is then freed by `0x1402BD390` once no child runs).
//! - KH1's instructions put the instruction class in the low 4 bits (push 2, call 11, ...); KH2's
//!   (Steam KH2's interpreter `0x14041B400`, Checked (decomp)) push 0 and call 10, with an 8-byte
//!   evaluation stack entry (value and a tag: "@INT", "@FLT", "@ADR", "@???"). Both address memory
//!   with the same 32-bit handles (`0x80000000 | segment << 25 | offset`, KH1's `0x14038AD90` /
//!   `0x14038ADC0`, KH2's `0x1404AD240` / `0x1404AD270`).
//!
//! So a KH2 spell now runs as a KH1 script process: KH1's spell start makes its task and process
//! (record, stack, evaluation stack, work slot from KH1's own pools) on the spell's KH2 code, which
//! is loaded once into a block of ours; KH1's task list steps it every frame through KH1's
//! interpreter, which is taught KH2's instruction set: a process whose program is in our block is
//! decoded with KH2's instructions (`kh2_bdx`'s machine, on the record's registers and KH1's
//! memory), any other with KH1's own. Everything the script's calls make (records, vectors, effect
//! and attack records) sits in a block of ours that KH1's handles address.
//!
//! A thread a script starts (`trap_magic_start_thread`: Blizzard's and Thunder's) is a KH1 child
//! process of the cast's (KH1's child start `0x1402BD4D0`: its own record and stack, the parent's
//! work memory, on the parent's chain), which KH1's step runs after its parent, t799. KH2's calls
//! are a KH1 call-table list on every record of ours (`+0x38`, KH1's layout), which the call
//! instruction goes through, t800. What each call does is ours (`kh2_magic`'s calls on KH1's
//! systems: attacks as KH1 attack objects, effects through KH1's effect system).
//!
//! Steam only; a script runs on our own machine, as before, if any hook or check here fails or KH1
//! has no free process for it (logged).

use super::addr::Build;
use super::sys::*;
use super::{log, pose};
use kh2_magic::kh2_bdx::{Cell, CtxId, Memory, Native, Tag, Vm};
use kh2_magic::{NativeHost, SpellFile};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

/// KH1's script interpreter (Steam): `mov r11, rsp; mov [r11+0x20], rbx; push rbp` (Checked (exe)).
const INTERP_STEAM: usize = 0x2CC620;
const INTERP_PROLOGUE: [u8; 8] = [0x4C, 0x8B, 0xDC, 0x49, 0x89, 0x5B, 0x20, 0x55];
/// KH1's spell start (Steam), its first bytes checked before a call (Checked (exe)).
const SPELL_START_STEAM: usize = 0x28F5E0;
const SPELL_START_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];
/// KH1's pointer → handle (Steam `0x14038AD90`; it takes a new segment into its table when it
/// has to), its first bytes checked (Checked (exe)).
const TO_HANDLE_STEAM: usize = 0x38AD90;
const TO_HANDLE_BYTES: [u8; 9] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9];
/// KH1's process records, work slots and their in-use bytes, and the spell task list (Steam).
const PROCS_STEAM: usize = 0x2DC7BE0;
const PROC_SIZE: usize = 0x758;
const PROC_COUNT: usize = 64;
const WORK_STEAM: usize = 0x2DB7BE0;
const WORK_SIZE: usize = 0x800;
const WORK_COUNT: usize = 32;
const WORK_USED_STEAM: usize = 0x2DE51E0;
const TASKS_STEAM: usize = 0x2D22D08;
const PAUSE_STEAM: usize = 0x2867364;
/// A process record's fields (above).
const R_FLAGS: usize = 0;
const R_PROGRAM: usize = 0x10;
const R_PC: usize = 0x18;
const R_SP: usize = 0x20;
const R_TP: usize = 0x28;
const R_WP: usize = 0x30;
const STACK_AT: usize = 0x40;
const STACK_SIZE: usize = 0x400;
const EVAL_AT: usize = 0x440;
const EVAL_END: usize = 0x740;
/// KH2's evaluation stack entry tags (Steam KH2 `0x14041B400`, Checked (decomp)).
const TAG_INT: u32 = 0x544E_4940;
const TAG_FLT: u32 = 0x544C_4640;
const TAG_ADR: u32 = 0x5244_4140;
const TAG_UNK: u32 = 0x3F3F_3F40;
/// Our blocks (one for the programs, one per cast in flight): 128 KB on a 128 KB boundary, so one
/// never straddles one of KH1's 32 MB handle segments (a script's address arithmetic stays in one).
const BLOCK: usize = 0x20000;
const MAX_BLOCKS: usize = 16;
/// The interpreter's "ended" result.
const END: u64 = 0xFFFF_FFFF;
/// Our steps in a row without KH1 stepping a process: it's gone (a room change or reset dropped
/// its task).
const MISSED_MAX: u32 = 30;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG: AtomicUsize = AtomicUsize::new(0);
static ON: AtomicBool = AtomicBool::new(false);
/// The programs' block (0 until the first cast).
static PROG: AtomicUsize = AtomicUsize::new(0);
/// The casts' blocks (0: not made yet), for `resolve`.
#[allow(clippy::declare_interior_mutable_const)]
const NO_BLOCK: AtomicUsize = AtomicUsize::new(0);
static BLOCKS: [AtomicUsize; MAX_BLOCKS] = [NO_BLOCK; MAX_BLOCKS];
static LOGGED: AtomicU32 = AtomicU32::new(0);

/// Which runtime a process's cast is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `reflect`'s.
    Reflect,
    /// `formmagic`'s (Fire, Blizzard, Thunder, Cure; Wisdom's).
    Form,
}

/// A KH2 script's KH1 process: a cast's main one, or a child for one of its threads.
struct Entry {
    proc: usize,
    /// The record's program (`+0x10`): the code its cast's main process was started on (KH1's
    /// child start copies its parent's).
    code: usize,
    /// Its program's place in our block.
    prog: usize,
    kind: Kind,
    id: u64,
    /// Its context in the cast's machine.
    ctx: CtxId,
    /// The cast's main process (not a thread's).
    main: bool,
    started: bool,
    missed: u32,
    stepped: u32,
    /// The record's call-table list before ours (put back when it ends).
    calls_was: usize,
}

struct Programs {
    /// (the program's bytes, its place in the block), and the bytes used.
    list: Vec<(Vec<u8>, usize)>,
    used: usize,
}

static PROCS: Mutex<Vec<Entry>> = Mutex::new(Vec::new());
/// Each cast's block (index into `BLOCKS`), by cast (0: made, its cast not started yet).
static CAST_BLOCKS: Mutex<Vec<(u64, usize, usize)>> = Mutex::new(Vec::new());
static PROGRAMS: Mutex<Programs> = Mutex::new(Programs { list: Vec::new(), used: 0 });
/// Which of `BLOCKS` a cast holds.
static BLOCK_USED: Mutex<[bool; MAX_BLOCKS]> = Mutex::new([false; MAX_BLOCKS]);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn note(s: impl FnOnce() -> String) {
    if super::log_on(super::LogCat::Magic) && LOGGED.fetch_add(1, Ordering::Relaxed) < 400 {
        log!(Magic, "{}", s());
    }
}

/// Hooks KH1's interpreter (Steam only; every first byte checked).
pub unsafe fn install() {
    if Build::current() != Build::Steam {
        return;
    }
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    for (what, at, want) in [("spell start", SPELL_START_STEAM, &SPELL_START_BYTES[..]), ("pointer → handle", TO_HANDLE_STEAM, &TO_HANDLE_BYTES[..])] {
        let have = std::slice::from_raw_parts((base + at) as *const u8, want.len());
        if have != want {
            log!(Magic, "KH2 spell scripts: KH1's {what} has unexpected bytes {have:02x?}: the scripts stay on our own machine");
            return;
        }
    }
    if super::hook::install(base + INTERP_STEAM, &INTERP_PROLOGUE, interp as *const () as usize, &ORIG, "KH1's script interpreter (KH2's instruction set)").is_some() {
        ON.store(true, Ordering::Relaxed);
        log!(Magic, "KH2 spell scripts run as KH1 script processes (KH1's interpreter taught KH2's instructions)");
    }
}

/// KH2's spell scripts run on KH1's script machine.
pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// A host for a runtime's `Magic` (`kh2_magic::NativeHost`), when the hook is in.
pub fn machine(kind: Kind) -> Option<Box<dyn NativeHost>> {
    on().then(|| Box::new(Kh1Machine { kind, pending: None }) as Box<dyn NativeHost>)
}

/// KH1's handle for a pointer (KH1's own function: a new segment is taken in as KH1 takes one).
unsafe fn handle(p: usize) -> u32 {
    if p == 0 {
        return 0;
    }
    let f: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(BASE.load(Ordering::Relaxed) + TO_HANDLE_STEAM);
    f(p)
}

/// Script memory: KH1's process records and work slots, and our blocks. Anything else a script
/// names fails as outside script memory (never a stray read).
fn resolve(a: u32, n: u32) -> Option<usize> {
    let p = pose::ptr(a);
    if p < 0x10000 {
        return None;
    }
    let end = p.checked_add(n as usize)?;
    let within = |lo: usize, len: usize| lo != 0 && p >= lo && end <= lo + len;
    let base = BASE.load(Ordering::Relaxed);
    if within(base + PROCS_STEAM, PROC_SIZE * PROC_COUNT) || within(base + WORK_STEAM, WORK_SIZE * WORK_COUNT) || within(PROG.load(Ordering::Relaxed), BLOCK) {
        return Some(p);
    }
    BLOCKS.iter().any(|b| within(b.load(Ordering::Relaxed), BLOCK)).then_some(p)
}

/// A block of ours (`BLOCK`), KH1-addressable (its handle resolves back to it). Kept for the
/// game's life.
unsafe fn new_block() -> Option<usize> {
    // (VirtualAlloc gives 64 KB alignment: half a block more, and the aligned part of it.)
    let raw = VirtualAlloc(std::ptr::null_mut(), BLOCK + BLOCK / 2, 0x3000, 0x04) as usize;
    if raw == 0 {
        return None;
    }
    let p = (raw + BLOCK - 1) & !(BLOCK - 1);
    let h = handle(p);
    if h == 0 || pose::ptr(h) != p || pose::ptr(h + (BLOCK - 1) as u32) != p + BLOCK - 1 {
        log!(Magic, "KH2 spell scripts: our block {p:#x} has no KH1 handle ({h:#x})");
        VirtualFree(raw as *mut _, 0, 0x8000);
        return None;
    }
    Some(p)
}

/// The program's place in our programs' block (copied in once).
unsafe fn program_ptr(bytes: &[u8]) -> Option<usize> {
    let mut g = PROGRAMS.lock().ok()?;
    if let Some((_, p)) = g.list.iter().find(|(b, _)| b.as_slice() == bytes) {
        return Some(*p);
    }
    let mut block = PROG.load(Ordering::Relaxed);
    if block == 0 {
        block = new_block()?;
        PROG.store(block, Ordering::Relaxed);
    }
    let at = (g.used + 15) & !15;
    if at + bytes.len() > BLOCK {
        log!(Magic, "KH2 spell scripts: no room left for program {} bytes", bytes.len());
        return None;
    }
    let p = block + at;
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), p as *mut u8, bytes.len());
    g.used = at + bytes.len();
    g.list.push((bytes.to_vec(), p));
    note(|| format!("KH2 spell scripts: a program of {} bytes loaded at {p:#x}", bytes.len()));
    Some(p)
}

/// A free cast block (made on first use): its index.
unsafe fn take_block() -> Option<usize> {
    let mut used = BLOCK_USED.lock().ok()?;
    for i in 0..MAX_BLOCKS {
        if used[i] {
            continue;
        }
        if BLOCKS[i].load(Ordering::Relaxed) == 0 {
            let p = new_block()?;
            BLOCKS[i].store(p, Ordering::Relaxed);
        }
        used[i] = true;
        return Some(i);
    }
    None
}

fn give_block(i: usize) {
    if let Ok(mut used) = BLOCK_USED.lock() {
        if i < MAX_BLOCKS {
            used[i] = false;
        }
    }
}

/// KH1 has room for one more spell process: a free record, a free work slot and a free entry in
/// the spell task list (KH1's own spell start checks none of these).
unsafe fn room() -> bool {
    let base = BASE.load(Ordering::Relaxed);
    let record = (0..PROC_COUNT).any(|i| rd::<u32>(base + PROCS_STEAM + i * PROC_SIZE + R_FLAGS) & 1 == 0);
    let slot = (0..WORK_COUNT).any(|i| rd::<u8>(base + WORK_USED_STEAM + i) == 0);
    let list = base + TASKS_STEAM;
    let (size, count, entries) = (rd::<u16>(list) as usize, rd::<u16>(list + 2) as usize, rd::<usize>(list + 8));
    let task = entries > 0x10000 && size > 0 && (0..count).any(|i| rd::<u16>(entries + i * size) == 0);
    record && slot && task
}

/// Whether `p` is one of KH1's process records.
fn is_record(p: usize) -> bool {
    let lo = BASE.load(Ordering::Relaxed) + PROCS_STEAM;
    p >= lo && p < lo + PROC_SIZE * PROC_COUNT && (p - lo) % PROC_SIZE == 0
}

fn tag_word(t: Tag) -> u32 {
    match t {
        Tag::Int => TAG_INT,
        Tag::Float => TAG_FLT,
        Tag::Addr => TAG_ADR,
        Tag::Unknown => TAG_UNK,
    }
}

fn tag_of(w: u32) -> Tag {
    match w {
        TAG_INT => Tag::Int,
        TAG_FLT => Tag::Float,
        TAG_ADR => Tag::Addr,
        _ => Tag::Unknown,
    }
}

/// `kh2_magic`'s host: KH1's script machine.
struct Kh1Machine {
    kind: Kind,
    /// The process `machine` made, until `started` names its cast.
    pending: Option<usize>,
}

/// KH1's child process start (Steam `0x1402BD4D0 (parent, code)`: a record of its own, flag 4,
/// the parent's program and work memory, appended to the parent's chain at `+0x740`), its first
/// bytes checked (Checked (exe)).
const CHILD_STEAM: usize = 0x2BD4D0;
const CHILD_BYTES: [u8; 10] = [0x48, 0x83, 0xEC, 0x28, 0x48, 0x8B, 0xC2, 0x4C, 0x8B, 0xD1];
const R_NEXT: usize = 0x740;
/// A record's call-table list (`+0x38`), and KH1's own list that its reset (`0x1402BCFD0`) gives
/// every record (Steam `0x140529FA0`, Checked (decomp)).
const R_CALLS: usize = 0x38;
const KH1_CALLS_STEAM: usize = 0x529FA0;
/// Records left free for KH1's own scripts: a thread takes one only above this.
const RESERVE: usize = 8;

impl NativeHost for Kh1Machine {
    fn machine(&mut self, file: &SpellFile, _spell: usize, key: i32) -> Option<(Vm, CtxId)> {
        unsafe {
            let base = BASE.load(Ordering::Relaxed);
            let actor = super::takeover::sora_actor();
            if !on() || actor < 0x10000 {
                return None;
            }
            let entry = file.program.trigger(key)?;
            let prog = program_ptr(file.program.bytes())?;
            if !room() {
                note(|| format!("KH2 spell scripts: KH1 has no free process for {} (it runs on our machine)", file.name));
                return None;
            }
            let block = take_block()?;
            let code = prog + 0x10 + entry as usize * 2;
            let start: unsafe extern "system" fn(usize, usize, u32, usize, usize, usize, usize) -> usize = std::mem::transmute(base + SPELL_START_STEAM);
            let task = start(actor, 0, 0, 0, code, 0, 0);
            let proc = if task > 0x10000 { rd::<usize>(task + 0x20) } else { 0 };
            if !is_record(proc) || rd::<usize>(proc + R_PROGRAM) != code {
                log!(Magic, "KH2 spell scripts: KH1's spell start gave task {task:#x} process {proc:#x} for {}: not used", file.name);
                give_block(block);
                return None;
            }
            let bp = BLOCKS[block].load(Ordering::Relaxed);
            let mem = Memory::native(Native { resolve, block: handle(bp), size: BLOCK as u32, used: 0 });
            let mut vm = Vm::on_memory(file.program.clone(), mem, handle(prog));
            // (KH2's calls through the record's call tables, t800.)
            if call_tables() != 0 {
                vm.calls = Some(lookup);
            }
            let main = vm.spawn_at(handle(proc + STACK_AT), STACK_SIZE as u32, handle(rd::<usize>(proc + R_WP)));
            if let Ok(mut g) = PROCS.lock() {
                g.retain(|e| e.proc != proc);
                g.push(Entry { proc, code, prog, kind: self.kind, id: 0, ctx: main, main: true, started: false, missed: 0, stepped: 0, calls_was: own_calls(proc) });
            }
            if let Ok(mut g) = CAST_BLOCKS.lock() {
                g.push((0, block, proc));
            }
            self.pending = Some(proc);
            note(|| format!("KH2 spell scripts: {} trigger {key} as KH1 process {proc:#x} (task {task:#x}, work {:#x})", file.name, rd::<usize>(proc + R_WP)));
            Some((vm, main))
        }
    }

    fn started(&mut self, id: u64, vm: &Vm, main: CtxId) -> bool {
        let Some(proc) = self.pending.take() else { return false };
        let Ok(mut g) = PROCS.lock() else { return false };
        let Some(e) = g.iter_mut().find(|e| e.proc == proc && e.main && e.ctx == main) else { return false };
        e.id = id;
        e.started = true;
        let (prog, kind) = (e.prog, e.kind);
        drop(g);
        if let Ok(mut b) = CAST_BLOCKS.lock() {
            if let Some(c) = b.iter_mut().find(|c| c.0 == 0 && c.2 == proc) {
                c.0 = id;
            }
        }
        unsafe { put_regs(proc, prog, vm, main) };
        note(|| format!("KH2 spell scripts: cast {id} ({kind:?}) handed to KH1 process {proc:#x} at game frame {}", super::takeover::GAME_FRAME.load(Ordering::Relaxed)));
        true
    }

    fn thread(&mut self, id: u64, vm: &mut Vm, wp: u32) -> Option<CtxId> {
        unsafe {
            let base = BASE.load(Ordering::Relaxed);
            let (parent, code, prog, kind) = {
                let g = PROCS.lock().ok()?;
                let e = g.iter().find(|e| e.id == id && e.main && e.started)?;
                (e.proc, e.code, e.prog, e.kind)
            };
            let have = std::slice::from_raw_parts((base + CHILD_STEAM) as *const u8, CHILD_BYTES.len());
            if have != CHILD_BYTES {
                note(|| format!("KH2 spell scripts: KH1's child start has unexpected bytes {have:02x?}: the thread runs in its parent's step"));
                return None;
            }
            let free = (0..PROC_COUNT).filter(|i| rd::<u32>(base + PROCS_STEAM + i * PROC_SIZE + R_FLAGS) & 1 == 0).count();
            if free <= RESERVE {
                note(|| format!("KH2 spell scripts: {free} KH1 process(es) free: cast {id}'s thread runs in its parent's step"));
                return None;
            }
            let start: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + CHILD_STEAM);
            start(parent, code);
            // (KH1's start returns nothing: the new child is the chain's last.)
            let mut child = rd::<usize>(parent + R_NEXT);
            let mut guard = 0;
            while child != 0 && rd::<usize>(child + R_NEXT) != 0 && guard < PROC_COUNT {
                child = rd::<usize>(child + R_NEXT);
                guard += 1;
            }
            if !is_record(child) || rd::<u32>(child + R_FLAGS) & 4 == 0 || rd::<usize>(child + R_PROGRAM) != code {
                log!(Magic, "KH2 spell scripts: KH1's child start gave {child:#x} for cast {id}: the thread runs in its parent's step");
                return None;
            }
            let ctx = vm.spawn_at(handle(child + STACK_AT), STACK_SIZE as u32, wp);
            if let Ok(mut g) = PROCS.lock() {
                g.retain(|e| e.proc != child);
                g.push(Entry { proc: child, code, prog, kind, id, ctx, main: false, started: true, missed: 0, stepped: 0, calls_was: own_calls(child) });
            }
            note(|| format!("KH2 spell scripts: cast {id}'s thread as KH1 child process {child:#x} of {parent:#x}"));
            Some(ctx)
        }
    }

    fn load(&mut self, id: u64, vm: &mut Vm, ctx: CtxId) {
        let Some((proc, prog)) = find(id, ctx) else { return };
        unsafe { get_regs(proc, prog, vm, ctx) };
    }

    fn store(&mut self, id: u64, vm: &Vm, ctx: CtxId) {
        let Some((proc, prog)) = find(id, ctx) else { return };
        unsafe { put_regs(proc, prog, vm, ctx) };
    }

    fn alive(&self, id: u64) -> bool {
        let Ok(mut g) = PROCS.lock() else { return true };
        let Some(e) = g.iter_mut().find(|e| e.id == id && e.main && e.started) else { return false };
        // (KH1 steps no spell process while the game is paused, `0x14028F6D0`.)
        if !unsafe { paused() } {
            e.missed += 1;
        }
        let ours = unsafe { rd::<u32>(e.proc + R_FLAGS) & 1 != 0 && rd::<usize>(e.proc + R_PROGRAM) == e.code };
        ours && e.missed <= MISSED_MAX
    }

    fn ended(&mut self, id: u64) {
        // (Its processes end at KH1's next step of them: the interpreter finds no cast.)
        if let Ok(mut g) = PROCS.lock() {
            let steps: u32 = g.iter().filter(|e| e.id == id && e.main).map(|e| e.stepped).sum();
            for e in g.iter().filter(|e| e.id == id) {
                unsafe { give_calls(e) };
            }
            g.retain(|e| e.id != id);
            note(|| format!("KH2 spell scripts: cast {id} over ({steps} step(s) by KH1)"));
        }
        free_cast_block(|c| c.0 == id);
    }
}

/// Our call-table list on a record (KH2's calls, `call_tables`); what it had is returned (to put
/// back when the process ends: KH1 gives a new process no list of its own, `0x1402BD5B0`).
fn own_calls(proc: usize) -> usize {
    unsafe {
        let list = call_tables();
        let was = rd::<usize>(proc + R_CALLS);
        if list == 0 {
            return was;
        }
        wr(proc + R_CALLS, list);
        if was == list {
            BASE.load(Ordering::Relaxed) + KH1_CALLS_STEAM
        } else {
            was
        }
    }
}

/// KH1's call-table list back on a record of ours that ends.
unsafe fn give_calls(e: &Entry) {
    let list = CALLS.load(Ordering::Relaxed);
    if list != 0 && is_record(e.proc) && rd::<usize>(e.proc + R_CALLS) == list {
        wr(e.proc + R_CALLS, e.calls_was);
    }
}

/// The tables of KH2's calls (`kh2_bdx`'s list: tables 0..=10), as KH1's script-call tables are
/// laid out (`0x1402CC620`'s call instruction, class 11: the record's `+0x38` a list of table
/// pointers by the instruction's bits 6..7, an entry of 0x10 bytes by its operand: the function at
/// `+0`, the flag word at `+8`, the argument count in the low 16 bits and `0x40000000` when it
/// returns; Checked (decomp)). Every entry's function is `kh2_bdx::run_call`, which runs that call
/// on the machine making it. Built once (0: couldn't be).
static CALLS: AtomicUsize = AtomicUsize::new(0);
const CALL_TABLES: usize = 11;

unsafe fn call_tables() -> usize {
    let have = CALLS.load(Ordering::Relaxed);
    if have != 0 {
        return have;
    }
    use kh2_magic::kh2_bdx::traps::TRAPS;
    let counts: Vec<usize> = (0..CALL_TABLES).map(|t| TRAPS.iter().filter(|r| r.0 as usize == t).map(|r| r.1 as usize + 1).max().unwrap_or(0)).collect();
    let size = CALL_TABLES * 8 + CALL_TABLES * 4 + counts.iter().map(|n| n * 0x10).sum::<usize>() + 0x10;
    let mem = VirtualAlloc(std::ptr::null_mut(), size, 0x3000, 0x04) as usize;
    if mem == 0 {
        return 0;
    }
    let mut at = (mem + CALL_TABLES * 12 + 15) & !15;
    let entry = kh2_magic::kh2_bdx::run_call as *const () as usize;
    for (t, &n) in counts.iter().enumerate() {
        wr(mem + t * 8, at);
        wr(mem + CALL_TABLES * 8 + t * 4, n as u32);
        for r in TRAPS.iter().filter(|r| r.0 as usize == t) {
            wr(at + r.1 as usize * 0x10, entry);
            wr(at + r.1 as usize * 0x10 + 8, r.3);
        }
        at += n * 0x10;
    }
    CALLS.store(mem, Ordering::Relaxed);
    note(|| format!("KH2 spell scripts: KH2's calls as KH1 call tables at {mem:#x} ({} entries)", TRAPS.len()));
    mem
}

/// The machine's call instruction looks its call up in the tables (`Vm::calls`).
fn lookup(t: u8, i: u16) -> Option<(usize, u32)> {
    let list = CALLS.load(Ordering::Relaxed);
    if list == 0 || t as usize >= CALL_TABLES {
        return None;
    }
    unsafe {
        let count = rd::<u32>(list + CALL_TABLES * 8 + t as usize * 4);
        if i as u32 >= count {
            return None;
        }
        let e = rd::<usize>(list + t as usize * 8) + i as usize * 0x10;
        Some((rd::<usize>(e), rd::<u32>(e + 8)))
    }
}

/// Gives back the block of the cast(s) `which` picks.
fn free_cast_block(which: impl Fn(&(u64, usize, usize)) -> bool) {
    if let Ok(mut b) = CAST_BLOCKS.lock() {
        b.retain(|c| {
            if which(c) {
                give_block(c.1);
                false
            } else {
                true
            }
        });
    }
}

/// KH1's pause (`0x142867364` bit 0x4000: its spell tasks and collision pass don't run).
unsafe fn paused() -> bool {
    rd::<u32>(BASE.load(Ordering::Relaxed) + PAUSE_STEAM) & 0x4000 != 0
}

fn find(id: u64, ctx: CtxId) -> Option<(usize, usize)> {
    let g = PROCS.lock().ok()?;
    g.iter().find(|e| e.id == id && e.ctx == ctx && e.started).map(|e| (e.proc, e.prog))
}

/// A context's registers from its KH1 process record: pc, stack, KH2's evaluation stack (8-byte
/// entries, value and tag) and work memory.
unsafe fn get_regs(proc: usize, prog: usize, vm: &mut Vm, ctx: CtxId) {
    let Some(c) = vm.ctxs.get_mut(ctx) else { return };
    let pc = rd::<usize>(proc + R_PC);
    if pc >= prog + 0x10 {
        c.pc = ((pc - (prog + 0x10)) / 2) as u32;
    }
    c.sp = handle(rd::<usize>(proc + R_SP));
    c.wp = handle(rd::<usize>(proc + R_WP));
    let top = rd::<usize>(proc + R_TP);
    let n = if top >= proc + EVAL_AT && top <= proc + EVAL_END { (top - (proc + EVAL_AT)) / 8 } else { 0 };
    c.tp.clear();
    for i in 0..n {
        let at = proc + EVAL_AT + i * 8;
        c.tp.push(Cell { v: rd::<u32>(at), tag: tag_of(rd::<u32>(at + 4)) });
    }
}

/// A context's registers into its KH1 process record.
unsafe fn put_regs(proc: usize, prog: usize, vm: &Vm, ctx: CtxId) {
    let Some(c) = vm.ctxs.get(ctx) else { return };
    wr(proc + R_PC, prog + 0x10 + c.pc as usize * 2);
    let sp = pose::ptr(c.sp);
    if sp >= proc + STACK_AT && sp <= proc + STACK_AT + STACK_SIZE {
        wr(proc + R_SP, sp);
    }
    let room = (EVAL_END - EVAL_AT) / 8;
    if c.tp.len() > room {
        note(|| format!("KH2 spell scripts: {} evaluation stack entries, KH1's process has room for {room}", c.tp.len()));
    }
    let n = c.tp.len().min(room);
    for (i, cell) in c.tp.iter().take(n).enumerate() {
        let at = proc + EVAL_AT + i * 8;
        wr(at, cell.v);
        wr(at + 4, tag_word(cell.tag));
    }
    wr(proc + R_TP, proc + EVAL_AT + n * 8);
}

type InterpF = unsafe extern "system" fn(usize, usize, usize) -> u64;

/// KH1's interpreter `(record + 0x18, callback, record)`: a process on one of our KH2 programs is
/// run with KH2's instructions (its runtime's `kh1_step`), any other with KH1's.
unsafe extern "system" fn interp(cursor: usize, callback: usize, proc: usize) -> u64 {
    let orig: InterpF = std::mem::transmute(ORIG.load(Ordering::Acquire));
    let block = PROG.load(Ordering::Relaxed);
    if block == 0 || !is_record(proc) {
        return orig(cursor, callback, proc);
    }
    let program = rd::<usize>(proc + R_PROGRAM);
    if program < block || program >= block + BLOCK {
        // (A record KH1 took back from us without our list off it: KH1's own list again before
        // KH1's instructions run on it.)
        let list = CALLS.load(Ordering::Relaxed);
        if list != 0 && rd::<usize>(proc + R_CALLS) == list {
            wr(proc + R_CALLS, BASE.load(Ordering::Relaxed) + KH1_CALLS_STEAM);
        }
        return orig(cursor, callback, proc);
    }
    // One of ours: KH2's instruction set.
    let found = PROCS.lock().ok().and_then(|mut g| {
        let e = g.iter_mut().find(|e| e.proc == proc && e.code == program)?;
        e.missed = 0;
        e.stepped += 1;
        if e.stepped == 1 {
            let (id, main) = (e.id, e.main);
            note(|| format!("KH2 spell scripts: KH1 first stepped cast {id}'s {} (process {proc:#x}) at game frame {}", if main { "script" } else { "thread" }, super::takeover::GAME_FRAME.load(Ordering::Relaxed)));
        }
        Some((e.started, e.kind, e.id, e.ctx))
    });
    let goes = match found {
        Some((true, Kind::Reflect, id, ctx)) => super::reflect::kh1_step(id, ctx),
        Some((true, Kind::Form, id, ctx)) => super::formmagic::kh1_step(id, ctx),
        // (Made, but its cast never started, or long gone: it ends.)
        _ => false,
    };
    if goes {
        return 0;
    }
    if let Ok(mut g) = PROCS.lock() {
        if let Some(p) = g.iter().position(|e| e.proc == proc) {
            let e = g.remove(p);
            give_calls(&e);
            // (A cast that never started has no `ended` to give its block back.)
            if !e.started {
                free_cast_block(|c| c.0 == 0 && c.2 == proc);
            }
        }
    }
    END
}
