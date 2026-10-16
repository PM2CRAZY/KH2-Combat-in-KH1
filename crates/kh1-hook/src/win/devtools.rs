//! Dev tools: switches for testing, not for play.
//!
//! - Insta-kill: every hit of Sora's (his swings, KH2's or KH1's, his magic) takes all of the
//!   foe's HP. `insta_kill = on` in the ini starts with it on; F8, or Select while L1 and R1 are
//!   held, toggles it in game (the ini keeps the choice). KH1's menu sound says which: "open"
//!   (2) for on, "back" (3) for off.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::log;

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

pub static INSTA_KILL: AtomicBool = AtomicBool::new(false);
/// A sound to play on the game's thread (0: none).
static SOUND: AtomicU32 = AtomicU32::new(0);
const SOUND_RVA: usize = 0x177F60;
/// F8.
const VK_TOGGLE: i32 = 0x77;
const SELECT: u16 = 1;
const L1: u16 = 1 << 10;
const R1: u16 = 1 << 11;
/// The least an insta-kill hit does (KH1's foes have far less HP).
const FALLBACK_DAMAGE: i32 = 9999;

pub fn install() {
    let on = super::settings::get().insta_kill;
    INSTA_KILL.store(on, Ordering::Relaxed);
    log(&format!("dev tools: insta-kill {} (F8, or L1 + R1 + Select, toggles)", if on { "on" } else { "off" }));
}

fn toggle(how: &str) {
    let on = !INSTA_KILL.load(Ordering::Relaxed);
    INSTA_KILL.store(on, Ordering::Relaxed);
    super::settings::save("insta_kill", if on { "on" } else { "off" });
    SOUND.store(if on { 2 } else { 3 }, Ordering::Relaxed);
    log(&format!("dev tools: insta-kill {} ({how})", if on { "on" } else { "off" }));
}

/// The monitor thread's keyboard check (every 50 ms).
pub unsafe fn keys() {
    static WAS: AtomicBool = AtomicBool::new(false);
    let down = (super::sys::GetAsyncKeyState(VK_TOGGLE) as u16 & 0x8000) != 0;
    if down && !WAS.swap(down, Ordering::Relaxed) {
        toggle("F8");
    } else if !down {
        WAS.store(false, Ordering::Relaxed);
    }
}

/// The pad hook (the game's thread): the pad combo, and the toggle's sound.
pub unsafe fn pad(base: usize, buttons: u16) {
    static WAS: AtomicBool = AtomicBool::new(false);
    let combo = buttons & (SELECT | L1 | R1) == SELECT | L1 | R1;
    if combo && !WAS.swap(true, Ordering::Relaxed) {
        toggle("L1 + R1 + Select");
    } else if !combo {
        WAS.store(false, Ordering::Relaxed);
    }
    let s = SOUND.swap(0, Ordering::Relaxed);
    if s != 0 {
        let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
        sound(s as i32);
    }
}

/// A hit record of Sora's on `target`, just made: with insta-kill on, its damage is all the
/// foe's HP (its stats `+0x3C`, as Sora's).
pub unsafe fn hit(target: usize, rec: usize) {
    if !INSTA_KILL.load(Ordering::Relaxed) || rec < 0x10000 || target < 0x10000 {
        return;
    }
    let stats = super::pose::ptr(rd::<u32>(target + 0x6C));
    let hp = if stats > 0x10000 { rd::<i32>(stats + 0x3C) } else { 0 };
    let d = rd::<i32>(rec + 8);
    // At least FALLBACK_DAMAGE, in case that field isn't HP for every foe.
    let kill = if (1..1_000_000).contains(&hp) { hp.max(FALLBACK_DAMAGE) } else { FALLBACK_DAMAGE };
    if d >= 0 && d < kill {
        wr(rec + 8, kill);
    }
}
