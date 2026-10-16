//! Skipping the Gummi Ship's flights between worlds (the user: a Config menu option, skip or
//! vanilla). Ported from the KH1 Final Mix Archipelago companion's "Instant Gummi" script
//! (`1fmAPInstantGummi.lua`, by denhonator with edits from Gicu, in gaithern's KH-1FM-AP-LUA,
//! MIT): each frame the world map's chosen destination is made the flight's current one, so
//! the ship arrives at once, and the Deep Jungle / Neverland warp points follow the story.
//! Addresses are the Epic exe's, the user's (the script's first column).

use super::log;
use std::sync::atomic::{AtomicBool, Ordering};

/// On: arrive at once (`gummi = skip`, the Config menu's Gummi Routes row).
pub static SKIP: AtomicBool = AtomicBool::new(false);

/// The world map's chosen destination (i32).
const SELECTION_RVA: usize = 0x507D7C;
/// The flight's current destination (i32; 40 and up while flying a route).
const DESTINATION_RVA: usize = 0x508280;
const SELECTION_OTHER_RVA: usize = 0x507C90;
const OTHER_RVA: usize = 0x268A26C;
/// World warp points (Deep Jungle's at +0x2A / +0x2C, Neverland's at +0x9A / +0x9C).
const WARPS_RVA: usize = 0x50F9D0;
/// Story progress bytes (Deep Jungle `+0xB05`, Neverland `+0xB0D`).
const STORY_RVA: usize = 0x2DEA760;

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

pub fn install() {
    let on = super::settings::get().gummi_skip;
    SKIP.store(on, Ordering::Relaxed);
    log(&format!("Gummi routes: {}", if on { "skipped (gummi = skip)" } else { "flown (gummi = vanilla)" }));
}

/// Every frame (from the pad read).
pub unsafe fn tick(base: usize) {
    if !SKIP.load(Ordering::Relaxed) || base == 0 {
        return;
    }
    let mut selection = rd::<i32>(base + SELECTION_RVA);
    let real = selection;
    let neverland = rd::<u8>(base + STORY_RVA + 0xB0D) < 0x14;
    let jungle = rd::<u8>(base + STORY_RVA + 0xB05) < 0x10;
    let w = base + WARPS_RVA;
    wr(w + 0x2A, if jungle { 0u8 } else { 0x0E });
    wr(w + 0x2C, if jungle { 0u8 } else { 0x2D });
    wr(w + 0x9A, if neverland { 6u8 } else { 0x07 });
    wr(w + 0x9C, if neverland { 0x18u8 } else { 0x25 });
    // (Its warps to Hollow Bastion and Agrabah, 25 and 21, go by way of 1.)
    if selection == 25 || selection == 21 {
        selection = 1;
        wr(base + SELECTION_RVA, selection);
    }
    let current = rd::<i32>(base + DESTINATION_RVA);
    if current < 40 {
        let s = if selection > 20 { 0 } else { selection };
        wr(base + DESTINATION_RVA, s);
        wr(base + SELECTION_OTHER_RVA, s);
        wr(base + OTHER_RVA, 0i32);
    } else {
        wr(base + SELECTION_OTHER_RVA, real);
    }
}
