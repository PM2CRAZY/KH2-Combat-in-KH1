//! The few Windows API functions we need.

use std::ffi::c_void;

#[link(name = "kernel32")]
extern "system" {
    pub fn LoadLibraryA(name: *const u8) -> *mut c_void;
    pub fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    pub fn GetSystemDirectoryA(buf: *mut u8, size: u32) -> u32;
    pub fn GetModuleFileNameW(module: *mut c_void, buf: *mut u16, size: u32) -> u32;
    pub fn GetModuleHandleA(name: *const u8) -> *mut c_void;
    pub fn CreateThread(attrs: *mut c_void, stack: usize, start: unsafe extern "system" fn(*mut c_void) -> u32, param: *mut c_void, flags: u32, id: *mut u32) -> *mut c_void;
    pub fn Sleep(ms: u32);
    pub fn VirtualAlloc(addr: *mut c_void, size: usize, kind: u32, prot: u32) -> *mut c_void;
    pub fn VirtualProtect(addr: *mut c_void, size: usize, prot: u32, old: *mut u32) -> i32;
    pub fn FlushInstructionCache(process: *mut c_void, addr: *const c_void, size: usize) -> i32;
    pub fn GetCurrentProcess() -> *mut c_void;
    pub fn GetCurrentThreadId() -> u32;
    pub fn GetCurrentProcessId() -> u32;
    pub fn OpenThread(access: u32, inherit: i32, id: u32) -> *mut c_void;
    pub fn SuspendThread(thread: *mut c_void) -> u32;
    pub fn ResumeThread(thread: *mut c_void) -> u32;
    pub fn GetThreadContext(thread: *mut c_void, ctx: *mut c_void) -> i32;
    pub fn SetThreadContext(thread: *mut c_void, ctx: *const c_void) -> i32;
    pub fn CloseHandle(h: *mut c_void) -> i32;
    pub fn VirtualQuery(addr: *const c_void, info: *mut c_void, size: usize) -> usize;
    pub fn CreateEventW(attrs: *mut c_void, manual: i32, initial: i32, name: *const u16) -> *mut c_void;
    pub fn WaitForSingleObject(h: *mut c_void, ms: u32) -> u32;
    pub fn SetThreadPriority(thread: *mut c_void, priority: i32) -> i32;
    pub fn GetCurrentThread() -> *mut c_void;
    pub fn AddVectoredExceptionHandler(first: u32, handler: unsafe extern "system" fn(*mut c_void) -> i32) -> *mut c_void;
}

#[link(name = "user32")]
extern "system" {
    pub fn GetAsyncKeyState(vk: i32) -> i16;
    pub fn GetForegroundWindow() -> *mut c_void;
    pub fn GetWindowThreadProcessId(window: *mut c_void, pid: *mut u32) -> u32;
}

/// Whether the foreground window (the one with the keyboard) is the game's.
pub fn game_focused() -> bool {
    unsafe {
        let w = GetForegroundWindow();
        if w.is_null() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(w, &mut pid);
        pid == GetCurrentProcessId()
    }
}

/// `WAVEFORMATEX`.
#[repr(C, packed(2))]
pub struct WaveFormatEx {
    pub format_tag: u16,
    pub channels: u16,
    pub samples_per_sec: u32,
    pub avg_bytes_per_sec: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,
    pub cb_size: u16,
}

/// `WAVEHDR` (64-bit layout).
#[repr(C)]
pub struct WaveHdr {
    pub data: *mut u8,
    pub buffer_length: u32,
    pub bytes_recorded: u32,
    pub user: usize,
    pub flags: u32,
    pub loops: u32,
    pub next: *mut WaveHdr,
    pub reserved: usize,
}

pub const WAVE_MAPPER: u32 = 0xFFFF_FFFF;
pub const WHDR_DONE: u32 = 1;
/// `waveOutOpen`'s callback kind: an event signalled as each buffer is done.
pub const CALLBACK_EVENT: u32 = 0x0005_0000;

#[link(name = "winmm")]
extern "system" {
    pub fn waveOutOpen(out: *mut usize, device: u32, format: *const WaveFormatEx, callback: usize, instance: usize, flags: u32) -> u32;
    pub fn waveOutPrepareHeader(out: usize, hdr: *mut WaveHdr, size: u32) -> u32;
    pub fn waveOutWrite(out: usize, hdr: *mut WaveHdr, size: u32) -> u32;
}
