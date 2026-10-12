//! Windows (64-bit) side: the proxy export, the KH1 check, the log.

mod hook;
mod pose;
mod sys;

use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use sys::*;

type Hresult = i32;
type DInputCreate = unsafe extern "system" fn(*mut c_void, u32, *const c_void, *mut *mut c_void, *mut c_void) -> Hresult;

static REAL_CREATE: AtomicUsize = AtomicUsize::new(0);
static LOG: Mutex<Option<std::fs::File>> = Mutex::new(None);
/// The host is KH1 (everything else only gets the proxy).
static IN_KH1: AtomicBool = AtomicBool::new(false);

pub const KH1_EXE: &str = "KINGDOM HEARTS FINAL MIX.exe";

/// Appends a line to `kh1-kh2.log` next to the game.
pub fn log(line: &str) {
    if let Ok(mut g) = LOG.lock() {
        if let Some(f) = g.as_mut() {
            let _ = writeln!(f, "{line}");
            let _ = f.flush();
        }
    }
}

/// The host executable's full path.
fn exe_path() -> String {
    let mut buf = [0u16; 1024];
    let n = unsafe { GetModuleFileNameW(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32) } as usize;
    String::from_utf16_lossy(&buf[..n])
}

pub fn exe_dir() -> String {
    let p = exe_path();
    p.rsplit_once('\\').map_or(".".to_string(), |(d, _)| d.to_string())
}

fn real_create() -> Option<DInputCreate> {
    let p = REAL_CREATE.load(Ordering::Acquire);
    if p != 0 {
        return Some(unsafe { std::mem::transmute::<usize, DInputCreate>(p) });
    }
    let mut buf = [0u8; 300];
    let n = unsafe { GetSystemDirectoryA(buf.as_mut_ptr(), 260) } as usize;
    let mut path = buf[..n].to_vec();
    path.extend_from_slice(b"\\dinput8.dll\0");
    let m = unsafe { LoadLibraryA(path.as_ptr()) };
    if m.is_null() {
        log("could not load the system dinput8.dll");
        return None;
    }
    let f = unsafe { GetProcAddress(m, b"DirectInput8Create\0".as_ptr()) };
    if f.is_null() {
        return None;
    }
    REAL_CREATE.store(f as usize, Ordering::Release);
    Some(unsafe { std::mem::transmute::<*mut c_void, DInputCreate>(f) })
}

/// The proxied DirectInput entry point.
#[no_mangle]
pub unsafe extern "system" fn DirectInput8Create(inst: *mut c_void, version: u32, riid: *const c_void, out: *mut *mut c_void, outer: *mut c_void) -> Hresult {
    match real_create() {
        Some(f) => f(inst, version, riid, out, outer),
        None => 0x8007_007Eu32 as i32,
    }
}

/// A heartbeat (which other mods are loaded), and the test key: Insert cycles the pose
/// override (`pose::MODE`).
unsafe extern "system" fn monitor(_: *mut c_void) -> u32 {
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    let mut n = 0u32;
    let mut was = false;
    loop {
        Sleep(50);
        n += 1;
        let down = (GetAsyncKeyState(0x2D) as u16 & 0x8000) != 0;
        if down && !was {
            let m = (pose::MODE.load(Ordering::Relaxed) + 1) % 6;
            pose::MODE.store(m, Ordering::Relaxed);
            log(&format!("Insert: pose override {}", if m == 0 { "off".to_string() } else { format!("on, T-pose written into array {m}") }));
        }
        was = down;
        if n == 100 || n % 2400 == 0 {
            let others: Vec<&str> = ["DBGHELP.dll", "LuaBackend.dll"].into_iter().filter(|m| !GetModuleHandleA(format!("{m}\0").as_ptr()).is_null()).collect();
            log(&format!("alive {}s; KH1 at {base:#x}; other mods loaded: {others:?}", n / 20));
        }
    }
}

#[no_mangle]
pub unsafe extern "system" fn DllMain(_inst: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    if reason == 1 {
        let exe = exe_path();
        let kh1 = exe.rsplit('\\').next().is_some_and(|n| n.eq_ignore_ascii_case(KH1_EXE));
        IN_KH1.store(kh1, Ordering::Relaxed);
        if !kh1 {
            return 1;
        }
        let path = format!("{}\\kh1-kh2.log", exe_dir());
        if let Ok(f) = OpenOptions::new().create(true).write(true).truncate(true).open(&path) {
            *LOG.lock().unwrap() = Some(f);
        }
        log(&format!("kh1-kh2 {} loaded (build {}) in {exe}", env!("CARGO_PKG_VERSION"), env!("KH1_KH2_BUILD")));
        std::panic::set_hook(Box::new(|info| log(&format!("PANIC in kh1-kh2: {info}"))));
        pose::install();
        CreateThread(std::ptr::null_mut(), 0, monitor, std::ptr::null_mut(), 0, std::ptr::null_mut());
    }
    1
}
