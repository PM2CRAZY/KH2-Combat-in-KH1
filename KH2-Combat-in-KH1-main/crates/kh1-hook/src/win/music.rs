//! Music fully off (the user: "Can you also make it where music can be fully muted in the
//! settings?"). `docs/kh1-music-mute.md`.
//!
//! KH1's settings keep four volume steps (`0x14012AC50` returns the block): master `+0x10`,
//! music `+0x12`, effects `+0x14`, voice `+0x16` (Steam: 4 bytes later), 1..10 each. The menu's
//! step (`0x140150C60`) holds them at 1 or more, and the music step's apply (`0x1401169B0`)
//! takes only 1..10: sound category 1 (the music's) to the step's entry in a shared table
//! (`0x1403D9B90`: 0.0, 0.1 .. 1.0). So the lowest step is 0.1, still heard. The other place
//! that sets category 1 is the "all categories back" call (`0x1400DDD00 (level, fade)`, six
//! callers, one with a fade): level > 0, the music to its step / 10.
//!
//! With `music_mute = lowest` (the default) the lowest music step is silence: both are hooked,
//! the step's apply sets category 1 to 0, and "all back" sees step 0 for the length of the
//! call (0 / 10, with its own fade). `on`: music silent at every step; `off`: KH1's own. A BGM
//! starting or changing (rooms, battles, bosses, scenes) plays through category 1 and doesn't
//! set it, so the silence holds. Effects and voices (categories 2..8) are not touched.

use super::addr::{Build, Rva};
use super::{hook, log};
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

/// When the music is silent (`music_mute`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicMute {
    /// At the game's lowest Music volume step (the default).
    Lowest,
    /// Always.
    On,
    /// Never: KH1's own steps (the lowest one is 0.1).
    Off,
}

impl MusicMute {
    pub fn parse(v: &str) -> MusicMute {
        match v {
            "on" | "always" | "true" | "1" | "yes" => MusicMute::On,
            "off" | "kh1" | "false" | "0" | "no" => MusicMute::Off,
            _ => MusicMute::Lowest,
        }
    }

    /// Silent at music step `step`?
    fn silent(self, step: i32) -> bool {
        match self {
            MusicMute::Lowest => step <= 1,
            MusicMute::On => true,
            MusicMute::Off => false,
        }
    }
}

/// The music step's apply `(step)`: category 1 to the table's entry, steps 1..10 only.
const MUSIC_STEP_RVA: Rva = Rva(0x1169B0);
/// `push rbx; sub rsp, 0x50`.
const MUSIC_STEP_PROLOGUE: [u8; 6] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x50];
/// Every category back `(level, fade)`: level > 0, music / effects / voice to their steps / 10.
const ALL_BACK_RVA: Rva = Rva(0xDDD00);
/// `mov rax, rsp; push r14; sub rsp, 0x80`.
const ALL_BACK_PROLOGUE: [u8; 12] = [0x48, 0x8B, 0xC4, 0x41, 0x56, 0x48, 0x81, 0xEC, 0x80, 0x00, 0x00, 0x00];
/// One category's volume now `(category, volume)` (no fade).
const CATEGORY_VOLUME_RVA: Rva = Rva(0x117A60);
/// KH1's settings block (its volume steps).
const SETTINGS_RVA: Rva = Rva(0x12AC50);
/// The music's sound category.
const MUSIC: i32 = 1;

static ORIG_MUSIC_STEP: AtomicUsize = AtomicUsize::new(0);
static ORIG_ALL_BACK: AtomicUsize = AtomicUsize::new(0);
/// The music's state last set: -1 not yet, 0 heard, 1 silent (logged when it changes).
static STATE: AtomicI32 = AtomicI32::new(-1);

fn base() -> usize {
    unsafe { super::sys::GetModuleHandleA(std::ptr::null()) as usize }
}

fn mode() -> MusicMute {
    super::settings::get().music_mute
}

/// Where the music step is kept (the settings block's `+0x12`, Steam's `+0x16`).
unsafe fn step_at() -> Option<usize> {
    let get: unsafe extern "system" fn() -> usize = std::mem::transmute(base() + SETTINGS_RVA);
    let s = get();
    if s < 0x10000 {
        return None;
    }
    Some(s + 0x12 + if Build::current() == Build::Steam { 4 } else { 0 })
}

/// Logs the music's state when it changes.
fn note(silent: bool, step: i32, how: &str) {
    if STATE.swap(silent as i32, Ordering::Relaxed) != silent as i32 {
        let m = mode();
        if silent {
            let why = if m == MusicMute::On { "music_mute = on".to_string() } else { format!("Music volume at its lowest step ({step})") };
            log(&format!("music: muted ({why}; {how})"));
        } else {
            log(&format!("music: heard again (Music volume step {step}; {how})"));
        }
    }
}

pub unsafe fn install() {
    let m = mode();
    log(&format!(
        "music_mute = {}",
        match m {
            MusicMute::Lowest => "lowest (the Music volume's lowest step is silence)",
            MusicMute::On => "on (no music at any step)",
            MusicMute::Off => "off (KH1's own steps; the lowest is quiet, not silent)",
        }
    ));
    if m == MusicMute::Off {
        return;
    }
    let b = base();
    let _ = hook::install(b + MUSIC_STEP_RVA, &MUSIC_STEP_PROLOGUE, music_step as *const () as usize, &ORIG_MUSIC_STEP, "music volume step (music_mute)");
    let _ = hook::install(b + ALL_BACK_RVA, &ALL_BACK_PROLOGUE, all_back as *const () as usize, &ORIG_ALL_BACK, "sound categories back (music_mute)");
}

/// The music step's apply (the menu's slider as it moves, and the settings applied at start).
unsafe extern "system" fn music_step(step: i32) {
    if (1..=10).contains(&step) && mode().silent(step) {
        let set: unsafe extern "system" fn(i32, f32) = std::mem::transmute(base() + CATEGORY_VOLUME_RVA);
        set(MUSIC, 0.0);
        note(true, step, "the Music volume set");
        return;
    }
    if (1..=10).contains(&step) {
        note(false, step, "the Music volume set");
    }
    let f: unsafe extern "system" fn(i32) = std::mem::transmute(ORIG_MUSIC_STEP.load(Ordering::Acquire));
    f(step)
}

/// Every category back: the music's step seen as 0 for the call when it is to be silent.
unsafe extern "system" fn all_back(level: f32, fade: i32) {
    let f: unsafe extern "system" fn(f32, i32) = std::mem::transmute(ORIG_ALL_BACK.load(Ordering::Acquire));
    let Some(at) = (if level > 0.0 { step_at() } else { None }) else {
        return f(level, fade);
    };
    let step = std::ptr::read_unaligned(at as *const i16);
    if !mode().silent(step as i32) {
        note(false, step as i32, "sounds back");
        return f(level, fade);
    }
    std::ptr::write_unaligned(at as *mut i16, 0);
    f(level, fade);
    std::ptr::write_unaligned(at as *mut i16, step);
    note(true, step as i32, "sounds back");
}
