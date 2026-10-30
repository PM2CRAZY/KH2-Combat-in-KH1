//! Windows (64-bit) side: the KH1 check, the log, and two ways in: Panacea's DLL folder
//! (`OnInit`, the installed route) or a `dinput8.dll` proxy (the launcher failed to start
//! with that in the shared KH folder, so it is not used).

mod addr;
mod bodyhits;
mod camera;
mod chest;
mod cmdmenu;
mod combat;
mod combomods;
mod config;
mod cosmetics;
mod crash;
mod cutscenelog;
mod debugfont;
mod debugmenu;
mod drive;
mod drivegauge;
mod driveorbs;
mod equipmenu;
mod fapanel;
mod feet;
mod form;
mod formexp;
mod formown;
mod frameadv;
mod formmagic;
mod gates;
mod gaugehud;
mod glide;
mod fx12;
mod hook;
mod hitreact;
mod hurt;
mod recoil;
mod rehit;
mod kh2fx;
mod kh2hud;
mod kh2script;
mod kh2dpx;
mod kh2ppp;
mod kh2slots;
mod kh2table;
mod kh1bank;
mod kh1picker;
mod kh1rows;
mod kh2damage;
mod kh1voice;
mod knockback;
mod kh2sound;
mod leftblade;
mod lefthits;
mod magnet;
mod holder;
mod loadprof;
mod loadscreen;
mod menu;
mod kh2list;
mod mpcost;
mod model;
mod pose;
mod quickrun;
mod settings;
mod softreset;
mod cutsceneskip;
mod ingameskip;
mod shared;
mod shortcuts;
mod soundlog;
mod gummi;
mod styles;
mod stylecombo;
mod probe;
mod reflect;
mod rumble;
mod takeover;
mod tint;
mod trace;
mod valor;
mod valorstats;
mod volmute;
mod equip;
mod sys;

use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;
use sys::*;

type Hresult = i32;
type DInputCreate = unsafe extern "system" fn(*mut c_void, u32, *const c_void, *mut *mut c_void, *mut c_void) -> Hresult;

static REAL_CREATE: AtomicUsize = AtomicUsize::new(0);
static LOG: Mutex<Option<std::fs::File>> = Mutex::new(None);
/// The host is KH1 (everything else only gets the proxy).
static IN_KH1: AtomicBool = AtomicBool::new(false);
/// The ini's `debug_menu` as the game started (the tester, ram-use thread m1 answer 3: "if debug
/// menu is off then we don't inject or load anything related to the debug"): read once, before
/// any hook goes in, and never changed in the session (a later save of the ini doesn't install or
/// remove hooks). Off: no debug menu, Frame Advance, trace, sound log hooks or heartbeat thread.
static DEBUG: AtomicBool = AtomicBool::new(false);

/// The debug tools are loaded this session (`DEBUG`; `docs/kh1-debug-menu.md`).
pub fn debug_on() -> bool {
    DEBUG.load(Ordering::Relaxed)
}

pub const KH1_EXE: &str = "KINGDOM HEARTS FINAL MIX.exe";

/// What the log holds, by system (the tester, ram-use thread m1 answer 3: "have our logs attached
/// to the debug menu where you can turn on and off each log and all should be off, if debug menu
/// is off then none of the logs are injected into the game"). Each line is written by `log!(Cat,
/// ...)`, which formats nothing unless its category is on (`log_on`); the categories are the debug
/// menu's Logs page, kept in the ini (`log_*`, all off by default), and only with the debug tools
/// loaded (`debug_on`). Which module logs under which category: `docs/kh1-debug-menu.md`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogCat {
    /// The mod's start, the ini, the hooks going in, the heartbeat.
    General,
    /// The loading screen and the asset cache (`loadscreen`, the slot cache lines in `pose`).
    Loading,
    /// Who has Sora, the shared KH1 / KH2 states, input, Quick Run, glide, exits.
    Takeover,
    /// KH2's motion slots, the pose, the left Keyblade, the form recolours.
    Animation,
    /// Combos, hits, damage, guards, recoil, reactions.
    Combat,
    /// KH2's spells, Reflect, MP costs.
    Magic,
    /// Drive Forms, Valor's Keyblade.
    Forms,
    /// KH2's sounds and voices through KH1's sound system.
    Sound,
    /// Every KH1 sound call (menu sounds, voices, sound effects: the old `sound_log`).
    Kh1Sounds,
    /// KH2's effects.
    Effects,
    /// The KH2 camera.
    Camera,
    /// KH1's menus as we change them: abilities, Config, shortcuts, equipment.
    Menus,
    /// The debug menu, Frame Advance, the trace recorder.
    Debug,
    /// Crash notes, panics, and the version and game-build lines at the start.
    Crash,
    /// KH1's cutscenes: scene prepares, event scripts, movies (`cutscenelog`).
    Cutscenes,
    /// Each frame of every enemy held by Magnet Burst's catch (`magnet`, t864): what moves it.
    Magnet,
}

impl LogCat {
    pub const ALL: [LogCat; 16] = [
        LogCat::General,
        LogCat::Loading,
        LogCat::Takeover,
        LogCat::Animation,
        LogCat::Combat,
        LogCat::Magic,
        LogCat::Forms,
        LogCat::Sound,
        LogCat::Kh1Sounds,
        LogCat::Effects,
        LogCat::Camera,
        LogCat::Menus,
        LogCat::Debug,
        LogCat::Crash,
        LogCat::Cutscenes,
        LogCat::Magnet,
    ];

    pub const fn bit(self) -> u32 {
        1 << self as u32
    }

    /// Its key in `kh1-kh2.ini`.
    pub const fn key(self) -> &'static str {
        match self {
            LogCat::General => "log_general",
            LogCat::Loading => "log_loading",
            LogCat::Takeover => "log_takeover",
            LogCat::Animation => "log_animation",
            LogCat::Combat => "log_combat",
            LogCat::Magic => "log_magic",
            LogCat::Forms => "log_forms",
            LogCat::Sound => "log_kh2_sounds",
            LogCat::Kh1Sounds => "log_kh1_sounds",
            LogCat::Effects => "log_effects",
            LogCat::Camera => "log_camera",
            LogCat::Menus => "log_menus",
            LogCat::Debug => "log_debug",
            LogCat::Crash => "log_crash",
            LogCat::Cutscenes => "log_cutscenes",
            LogCat::Magnet => "log_magnet",
        }
    }

    /// Its name on the debug menu's Logs page.
    pub const fn name(self) -> &'static str {
        match self {
            LogCat::General => "General",
            LogCat::Loading => "Loading & Cache",
            LogCat::Takeover => "Takeover & Input",
            LogCat::Animation => "Animation & Pose",
            LogCat::Combat => "Combat & Hits",
            LogCat::Magic => "Magic",
            LogCat::Forms => "Drive Forms",
            LogCat::Sound => "KH2 Sounds",
            LogCat::Kh1Sounds => "KH1 Sound Calls",
            LogCat::Effects => "Effects",
            LogCat::Camera => "Camera",
            LogCat::Menus => "Menus & Config",
            LogCat::Debug => "Debug Tools",
            LogCat::Crash => "Crash & Version",
            LogCat::Cutscenes => "Cutscenes & Events",
            LogCat::Magnet => "Magnet Debug",
        }
    }
}

/// (Open question to the tester) true: the crash notes and the version / build lines are written
/// whatever the debug menu and the Logs page say (the log file is then opened every start). The
/// tester's rule as written ("all should be off", "none of the logs are injected") is false.
pub const CRASH_NOTES_ALWAYS: bool = false;

/// The log categories on now (`LogCat::bit`): 0 without the debug tools.
static LOG_CATS: AtomicU32 = AtomicU32::new(if CRASH_NOTES_ALWAYS { LogCat::Crash.bit() } else { 0 });

/// The category's lines are written now (one load: what `log!` asks before formatting).
#[inline]
pub fn log_on(c: LogCat) -> bool {
    LOG_CATS.load(Ordering::Relaxed) & c.bit() != 0
}

/// The categories on now (the debug menu's Logs page).
pub fn log_cats() -> u32 {
    LOG_CATS.load(Ordering::Relaxed)
}

/// The debug menu's Logs page: the categories in `bits` on or off, each one's ini key written.
pub fn set_log_cats(bits: u32, on: bool) {
    if !debug_on() {
        return;
    }
    let now = if on { log_cats() | bits } else { log_cats() & !bits };
    LOG_CATS.store(now, Ordering::Relaxed);
    for c in LogCat::ALL.iter().filter(|c| c.bit() & bits != 0) {
        settings::save(c.key(), if on { "on" } else { "off" });
    }
    log!(Debug, "debug menu: log categories {now:#06x}");
}

/// Writes a line to `kh1-kh2.log` next to the game, with the game frame it was written on
/// (`[f123]`, Sora's physics frames: `takeover::GAME_FRAME`, the trace's `frame`), and keeps it
/// for that frame's trace row (`trace`: KH1's sound calls as its Sounds category, the rest as its
/// Log Lines). Called by `log!` once the category is on, and by `kh2hud`'s
/// outcome lines whatever their category says (t941).
pub fn log_line(c: LogCat, line: &str) {
    if let Ok(mut g) = LOG.lock() {
        if let Some(f) = g.as_mut() {
            let _ = writeln!(f, "[f{}] {line}", takeover::GAME_FRAME.load(Ordering::Relaxed));
            let _ = f.flush();
        }
    }
    trace::note_as(if c == LogCat::Kh1Sounds { trace::cat::SOUNDS } else { trace::cat::LOG }, line);
}

/// `log!(Category, "format", args...)`: a line of that category (`LogCat`), formatted and written
/// only while the category is on (`log_on`), so a category that is off costs one load.
macro_rules! log {
    ($cat:ident, $($t:tt)*) => {
        if $crate::win::log_on($crate::win::LogCat::$cat) {
            $crate::win::log_line($crate::win::LogCat::$cat, &::std::format!($($t)*))
        }
    };
}
pub(crate) use log;

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

/// The folder this DLL was loaded from (the mod's own folder when Panacea loads it from
/// `<mods>\\kh1\\dll\\`), else the game's folder.
pub fn dll_dir() -> String {
    let mut module: *mut c_void = std::ptr::null_mut();
    // (GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS 4 | _UNCHANGED_REFCOUNT 2: the module holding this
    // function's code.)
    let ok = unsafe { GetModuleHandleExW(6, dll_dir as *const () as *const u16, &mut module) };
    if ok != 0 && !module.is_null() {
        let mut buf = [0u16; 1024];
        let n = unsafe { GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32) } as usize;
        if n > 0 && n < buf.len() {
            let p = String::from_utf16_lossy(&buf[..n]);
            if let Some((d, _)) = p.rsplit_once('\\') {
                return d.to_string();
            }
        }
    }
    exe_dir()
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
        log!(General, "could not load the system dinput8.dll");
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
    log!(General, "loaded by Panacea (mod folder {s})");
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
            log!(General, "Insert: {}", pose::MODES[m as usize]);
        }
        was = down;
        probe::tick(n as f32 / 20.0);
        if n == 100 || n % 2400 == 0 {
            let others: Vec<&str> = ["DBGHELP.dll", "LuaBackend.dll"].into_iter().filter(|m| !GetModuleHandleA(format!("{m}\0").as_ptr()).is_null()).collect();
            log!(General, "alive {}s; KH1 at {base:#x}; other mods loaded: {others:?}", n / 20);
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
        // (The ini first: the log file and its categories only with the debug tools loaded,
        // `debug_menu = on`; the debug tools themselves go in only on a supported build, below.)
        let debug = settings::get().debug_menu;
        if debug {
            LOG_CATS.fetch_or(settings::get().log_cats, Ordering::Relaxed);
        }
        if debug || CRASH_NOTES_ALWAYS {
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
        }
        log!(Crash, "kh1-kh2 {} ({}) loaded (build {}) in {exe}", crate::VERSION, env!("CARGO_PKG_VERSION"), env!("KH1_KH2_BUILD"));
        log!(General, "settings at start: {:?}", settings::get());
        std::panic::set_hook(Box::new(|info| log!(Crash, "PANIC in kh1-kh2: {info}")));
        let base = GetModuleHandleA(std::ptr::null()) as usize;
        let build = addr::detect(base);
        if build == addr::Build::Unknown {
            log!(Crash, "unsupported game version (link stamp {}): this mod is for the Epic Games Store and Steam releases of KINGDOM HEARTS FINAL MIX (1.5+2.5) as of July 2024; nothing was changed", addr::stamp(base));
            return 1;
        }
        log!(Crash, "game build: {}", build.name());
        // (Before every install: the debug tools go in only with `debug_menu = on` at start.)
        DEBUG.store(debug, Ordering::Relaxed);
        log!(General, "debug tools {}", if debug_on() { "loaded (the ini's debug_menu = on)" } else { "not loaded (the ini's debug_menu = off)" });
        // (The crash note writes to the log: only when it can be written.)
        if debug_on() || CRASH_NOTES_ALWAYS {
            crash::install();
        }
        // (t783) The load profiler: with the debug tools and the Loading & Cache log on.
        loadprof::start();
        pose::install();
        kh2slots::install();
        probe::install();
        takeover::install();
        combat::install();
        kh1picker::install();
        knockback::install();
        kh2damage::install();
        recoil::install();
        bodyhits::install();
        hurt::install();
        menu::install();
        kh2list::install();
        camera::install();
        config::install();
        shortcuts::install();
        gummi::install();
        equipmenu::install();
        reflect::install();
        formmagic::install();
        kh2script::install();
        formexp::install(base);
        mpcost::install();
        drive::install();
        cmdmenu::install();
        tint::install();
        leftblade::install();
        holder::install();
        cosmetics::install();
        softreset::install();
        volmute::install();
        cutsceneskip::install();
        ingameskip::install();
        chest::install();
        model::install();
        gates::install();
        // (Its voice group hook carries gameplay and always goes in; the log hooks only with the
        // debug tools.)
        soundlog::install();
        // (KH1's frame end hook always goes in: KH2's sound banks are put back there, `kh1bank`;
        // the menu's input, stat locks and drawing only with the debug tools.)
        debugmenu::install();
        if debug_on() {
            frameadv::install();
            cutscenelog::install();
        }
        loadscreen::install();
        kh2sound::start();
        kh2hud::start();
        gaugehud::install(base);
        driveorbs::install(base);
        kh2dpx::start();
        pose::load_kh2();
        model::load();
        // (The heartbeat, the Insert test key and the event-opcode trace: debug tools.)
        if debug_on() {
            CreateThread(std::ptr::null_mut(), 0, monitor, std::ptr::null_mut(), 0, std::ptr::null_mut());
        }
    }
    1
}
