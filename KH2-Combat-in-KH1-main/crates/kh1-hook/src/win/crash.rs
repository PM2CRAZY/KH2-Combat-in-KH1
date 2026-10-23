//! A crash note: the first few fatal exceptions in the game are written to the log (code,
//! where, the registers, and the return addresses on the stack, in the game's executable or
//! ours), then passed on untouched. KH1 writes no dump of its own for them.

use super::log;
use super::sys::*;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

static EXE: AtomicUsize = AtomicUsize::new(0);
static OURS: AtomicUsize = AtomicUsize::new(0);
static SEEN: AtomicU32 = AtomicU32::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

/// The base of the module holding `addr`.
unsafe fn module_of(addr: usize) -> usize {
    let mut info = [0u8; 48];
    if VirtualQuery(addr as *const c_void, info.as_mut_ptr() as *mut c_void, 48) == 0 {
        return 0;
    }
    rd::<u64>(info.as_ptr() as usize + 8) as usize
}

/// Readable memory (committed, not a guard or no-access page).
pub unsafe fn readable(addr: usize) -> bool {
    let mut info = [0u8; 48];
    if VirtualQuery(addr as *const c_void, info.as_mut_ptr() as *mut c_void, 48) == 0 {
        return false;
    }
    let state = rd::<u32>(info.as_ptr() as usize + 32);
    let protect = rd::<u32>(info.as_ptr() as usize + 36);
    state == 0x1000 && protect & 0x101 == 0 && protect != 0
}

fn name(a: usize) -> String {
    let (exe, ours) = (EXE.load(Ordering::Relaxed), OURS.load(Ordering::Relaxed));
    if a >= exe && a < exe + 0x400_0000 {
        format!("exe+{:x}", a - exe)
    } else if ours != 0 && a >= ours && a < ours + 0x100_0000 {
        format!("ours+{:x}", a - ours)
    } else {
        format!("{a:#x}")
    }
}

unsafe extern "system" fn on_exception(p: *mut c_void) -> i32 {
    let ptrs = p as usize;
    let rec = rd::<u64>(ptrs) as usize;
    let ctx = rd::<u64>(ptrs + 8) as usize;
    let code = rd::<u32>(rec);
    // Fatal kinds only (access violation, illegal instruction, divide by zero, stack overflow,
    // a failed fast-fail); debugger and C++ exceptions pass by.
    if !matches!(code, 0xC000_0005 | 0xC000_001D | 0xC000_0094 | 0xC000_00FD | 0xC000_0409 | 0xC000_0096) {
        return 0;
    }
    if SEEN.fetch_add(1, Ordering::Relaxed) >= 4 {
        return 0;
    }
    let at = rd::<u64>(rec + 0x10) as usize;
    let info = if code == 0xC000_0005 { format!(" ({} {:#x})", ["reading", "writing", "", "", "", "", "", "", "executing"].get(rd::<u64>(rec + 0x20) as usize).unwrap_or(&"?"), rd::<u64>(rec + 0x28)) } else { String::new() };
    let regs = ["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15"];
    let r: Vec<String> = regs.iter().enumerate().map(|(i, n)| format!("{n}={:x}", rd::<u64>(ctx + 0x78 + i * 8))).collect();
    let rsp = rd::<u64>(ctx + 0x98) as usize;
    let mut stack = Vec::new();
    let mut i = 0;
    while i < 0x300 && stack.len() < 24 && readable(rsp + i * 8) {
        let v = rd::<u64>(rsp + i * 8) as usize;
        let n = name(v);
        if !n.starts_with("0x") {
            stack.push(n);
        }
        i += 1;
    }
    log(&format!("CRASH {code:#x} at {}{info}; {}; stack: {}", name(at), r.join(" "), stack.join(" ")));
    log(&format!("at the crash: {}", super::fx12::crash_note(EXE.load(Ordering::Relaxed))));
    super::flush_log();
    0
}

pub unsafe fn install() {
    EXE.store(GetModuleHandleA(std::ptr::null()) as usize, Ordering::Relaxed);
    OURS.store(module_of(install as *const () as usize), Ordering::Relaxed);
    AddVectoredExceptionHandler(0, on_exception);
    log(&format!("crash notes on (our code at {:#x})", OURS.load(Ordering::Relaxed)));
}
