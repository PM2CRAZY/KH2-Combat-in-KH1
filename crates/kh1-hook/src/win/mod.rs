//! Windows (64-bit) side: the KH1 check, the log, and two ways in: Panacea's DLL folder
//! (`OnInit`, the installed route) or a `dinput8.dll` proxy (the launcher failed to start
//! with that in the shared KH folder, so it is not used).

mod addr;
mod camera;
mod combat;
mod config;
mod crash;
mod drive;
mod feet;
mod form;
mod formmagic;
mod gates;
mod fx12;
mod hook;
mod hurt;
mod kh2fx;
mod kh2sound;
mod menu;
mod pose;
mod settings;
mod shortcuts;
mod probe;
mod reflect;
mod takeover;
mod tint;
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

/// Panacea (OpenKH's mod loader, `DBGHELP.dll`) loads every DLL in `<mod_path>\kh1\dll\` in
/// KH1 only, then calls `OnInit` with that game's mod folder (and `OnFrame` each frame, if
/// exported). This is how the mod is installed: the launcher and the other games in the
/// folder never load it. (Our work starts in `DllMain`; this only notes the route.)
#[no_mangle]
pub unsafe extern "C" fn OnInit(mod_path: *const u16) {
    let mut s = String::new();
    if !mod_path.is_null() {
        let n = (0..1024).take_while(|&i| *mod_path.add(i) != 0).count();
        s = String::from_utf16_lossy(std::slice::from_raw_parts(mod_path, n));
    }
    log(&format!("loaded by Panacea (mod folder {s})"));
}

/// A heartbeat (which other mods are loaded), and the test key: Insert cycles what Sora is
/// drawn with (`pose::MODES`).
unsafe extern "system" fn monitor(_: *mut c_void) -> u32 {
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    let mut n = 0u32;
    let mut was = false;
    loop {
        Sleep(50);
        n += 1;
        let down = (GetAsyncKeyState(0x2D) as u16 & 0x8000) != 0;
        if down && !was {
            let m = (pose::MODE.load(Ordering::Relaxed) + 1) % pose::MODES.len() as u32;
            pose::MODE.store(m, Ordering::Relaxed);
            takeover::ON.store(m == 0, Ordering::Relaxed);
            log(&format!("Insert: {}", pose::MODES[m as usize]));
        }
        was = down;
        probe::tick(n as f32 / 20.0);
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
            // Other games and the launcher share the folder: only forward DirectInput, and note
            // that we were here (one line, for diagnosing a launch problem).
            let name = exe.rsplit('\\').next().unwrap_or("?").to_string();
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(format!("{}\\kh1-kh2-other.log", exe_dir())) {
                let _ = writeln!(f, "loaded in {name} (build {}); only forwarding DirectInput", env!("KH1_KH2_BUILD"));
            }
            return 1;
        }
        let path = format!("{}\\kh1-kh2.log", exe_dir());
        // The last runs' logs are kept beside it (a problem seen before a restart stays
        // readable): .prev.log the last, .prev2 .. .prev5 before it (the user's sessions were
        // lost to two quick restarts with one kept).
        for n in (2..=5).rev() {
            let from = if n == 2 { format!("{}\\kh1-kh2.prev.log", exe_dir()) } else { format!("{}\\kh1-kh2.prev{}.log", exe_dir(), n - 1) };
            let _ = std::fs::rename(&from, format!("{}\\kh1-kh2.prev{n}.log", exe_dir()));
        }
        let _ = std::fs::rename(&path, format!("{}\\kh1-kh2.prev.log", exe_dir()));
        if let Ok(f) = OpenOptions::new().create(true).write(true).truncate(true).open(&path) {
            *LOG.lock().unwrap() = Some(f);
        }
        log(&format!("kh1-kh2 {} ({}) loaded (build {}) in {exe}", crate::VERSION, env!("CARGO_PKG_VERSION"), env!("KH1_KH2_BUILD")));
        std::panic::set_hook(Box::new(|info| log(&format!("PANIC in kh1-kh2: {info}"))));
        let base = GetModuleHandleA(std::ptr::null()) as usize;
        let build = addr::detect(base);
        if build == addr::Build::Unknown {
            log(&format!("unsupported game version (link stamp {}): this mod is for the Epic Games Store and Steam releases of KINGDOM HEARTS FINAL MIX (1.5+2.5) as of July 2024; nothing was changed", addr::stamp(base)));
            return 1;
        }
        log(&format!("game build: {}", build.name()));
        crash::install();
        pose::install();
        probe::install();
        takeover::install();
        combat::install();
        hurt::install();
        menu::install();
        camera::install();
        config::install();
        shortcuts::install();
        reflect::install();
        drive::install();
        tint::install();
        gates::install();
        kh2sound::start();
        kh2fx::start();
        pose::load_kh2();
        CreateThread(std::ptr::null_mut(), 0, monitor, std::ptr::null_mut(), 0, std::ptr::null_mut());
    }
    1
}
