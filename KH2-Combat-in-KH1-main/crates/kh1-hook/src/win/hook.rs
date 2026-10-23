//! Inline hooks on 64-bit code: the first bytes of a function become a `jmp rel32` to a stub
//! near the executable (an absolute jump to our function); a trampoline holds the moved bytes
//! and jumps back. The moved bytes must be whole, position-independent instructions (checked
//! against what we expect first).

use super::log;
use super::sys::*;
use std::ffi::c_void;

/// An absolute `jmp [rip+0]; dq addr` (14 bytes).
fn abs_jmp(to: usize) -> [u8; 14] {
    let mut b = [0u8; 14];
    b[0] = 0xFF;
    b[1] = 0x25;
    b[6..14].copy_from_slice(&(to as u64).to_le_bytes());
    b
}

/// Executable memory within 2 GB below `near` (for `rel32` jumps from the game's code).
pub unsafe fn alloc_near(near: usize, size: usize) -> Option<usize> {
    let mut at = (near & !0xFFFF).saturating_sub(0x1000_0000);
    let low = near.saturating_sub(0x7000_0000);
    while at > low {
        let p = VirtualAlloc(at as *mut c_void, size, 0x3000, 0x40);
        if !p.is_null() {
            return Some(p as usize);
        }
        at -= 0x10000;
    }
    None
}

/// Hooks the function at `target` (whose first `expect.len()` bytes must be `expect`, at least
/// 5, whole instructions) to `ours`. Returns the trampoline (call it to run the original).
pub unsafe fn install(target: usize, expect: &[u8], ours: usize, original: &std::sync::atomic::AtomicUsize, what: &str) -> Option<usize> {
    let have = std::slice::from_raw_parts(target as *const u8, expect.len());
    if have != expect {
        log(&format!("hook {what} at {target:#x}: unexpected bytes {have:02x?}, not hooked"));
        return None;
    }
    let Some(mem) = alloc_near(target, 64) else {
        log(&format!("hook {what}: no memory near {target:#x}"));
        return None;
    };
    // Trampoline: the moved bytes, then back to the rest of the function.
    let tramp = mem;
    std::ptr::copy_nonoverlapping(expect.as_ptr(), tramp as *mut u8, expect.len());
    let back = abs_jmp(target + expect.len());
    std::ptr::copy_nonoverlapping(back.as_ptr(), (tramp + expect.len()) as *mut u8, back.len());
    // Stub: an absolute jump to our function, reachable by rel32 from the target.
    let stub = mem + 32;
    let go = abs_jmp(ours);
    std::ptr::copy_nonoverlapping(go.as_ptr(), stub as *mut u8, go.len());
    // The original must be callable before the jump goes in.
    original.store(tramp, std::sync::atomic::Ordering::Release);
    let mut old = 0u32;
    if VirtualProtect(target as *mut c_void, expect.len(), 0x40, &mut old) == 0 {
        log(&format!("hook {what}: VirtualProtect failed"));
        return None;
    }
    let rel = (stub as i64 - (target as i64 + 5)) as i32;
    let p = target as *mut u8;
    *p = 0xE9;
    std::ptr::copy_nonoverlapping(rel.to_le_bytes().as_ptr(), p.add(1), 4);
    for i in 5..expect.len() {
        *p.add(i) = 0x90;
    }
    let mut tmp = 0u32;
    VirtualProtect(target as *mut c_void, expect.len(), old, &mut tmp);
    FlushInstructionCache(GetCurrentProcess(), target as *const c_void, expect.len());
    log(&format!("hooked {what} at {target:#x}"));
    Some(tramp)
}

/// Writes `new` over `at` (code or data) if the bytes there are `expect`.
pub unsafe fn patch(at: usize, expect: &[u8], new: &[u8], what: &str) -> bool {
    let have = std::slice::from_raw_parts(at as *const u8, expect.len());
    if have != expect {
        log(&format!("patch {what} at {at:#x}: unexpected bytes {have:02x?}, not patched"));
        return false;
    }
    let mut old = 0u32;
    if VirtualProtect(at as *mut c_void, new.len(), 0x40, &mut old) == 0 {
        log(&format!("patch {what}: VirtualProtect failed"));
        return false;
    }
    std::ptr::copy_nonoverlapping(new.as_ptr(), at as *mut u8, new.len());
    let mut tmp = 0u32;
    VirtualProtect(at as *mut c_void, new.len(), old, &mut tmp);
    FlushInstructionCache(GetCurrentProcess(), at as *const c_void, new.len());
    true
}

/// Points the `call rel32` at `site` (which must call `callee`) at `ours`, through a stub near
/// the executable. `ours` takes the same arguments; it calls `callee` itself for the original.
pub unsafe fn redirect_call(site: usize, callee: usize, ours: usize, what: &str) -> bool {
    if *(site as *const u8) != 0xE8 {
        log(&format!("redirect {what} at {site:#x}: not a call"));
        return false;
    }
    let rel = std::ptr::read_unaligned((site + 1) as *const i32);
    let to = (site as i64 + 5 + rel as i64) as usize;
    if to != callee {
        log(&format!("redirect {what} at {site:#x}: calls {to:#x}, not {callee:#x}"));
        return false;
    }
    let Some(stub) = alloc_near(site, 16) else {
        log(&format!("redirect {what}: no memory near {site:#x}"));
        return false;
    };
    let go = abs_jmp(ours);
    std::ptr::copy_nonoverlapping(go.as_ptr(), stub as *mut u8, go.len());
    let mut call = [0u8; 5];
    call[0] = 0xE8;
    call[1..].copy_from_slice(&((stub as i64 - (site as i64 + 5)) as i32).to_le_bytes());
    let mut want = [0u8; 5];
    want[0] = 0xE8;
    want[1..].copy_from_slice(&rel.to_le_bytes());
    patch(site, &want, &call, what)
}
