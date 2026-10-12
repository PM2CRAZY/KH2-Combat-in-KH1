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
}

#[link(name = "user32")]
extern "system" {
    pub fn GetAsyncKeyState(vk: i32) -> i16;
}
