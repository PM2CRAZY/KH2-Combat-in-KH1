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
}
