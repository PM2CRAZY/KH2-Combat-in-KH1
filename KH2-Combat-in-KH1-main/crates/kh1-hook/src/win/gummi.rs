//! Skipping the Gummi Ship's flights between worlds (the user: "can you put an option in the
//! config to skip the gummi routes?", written from our own reading of the game, not from any
//! other mod). `docs/kh1-gummi.md`.
//!
//! KH1's world map runs in the "Gummi side" scene (`0x14020F6D0`, a mode at `0x14268E3C4`:
//! 1 the world map, 2 a flight, 3 landing in a world, 4 back to the field). The world map's task
//! (`0x140219490`) gets the chosen world from the map (`0x140218E30`, which leaves it at
//! `0x140507C94`), looks up the route between the two worlds and starts the flight with
//! `0x14020EF10 (route kind, route stage)` (mode 2). A flight cleared (`0x14023DA40`) asks where
//! to land in the destination world (`0x140213B20 (world)`: KH1's landing list, the landing
//! index, or a negative value if there's none or it was backed out of, which returns to the
//! world map at the destination, `0x14050B1B4` its current world, `0x14020EF60`), then lands
//! (`0x140266920 (4)`, `0x14020ED90 (landing)`: mode 3, the field loads the world and room).
//!
//! With `gummi = skip` the flight's start is hooked: instead of mode 2, the destination's
//! landing comes at once, the same calls the cleared flight makes, so the world loads as after
//! a flight (its arrival events, story and first visits are the field's, as always). Flights
//! started otherwise (the story's own, `0x14020EEB0`) are left alone.

use super::addr::Rva;
use super::{hook, log};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// On: arrive at once (`gummi = skip`, the Config menu's Gummi Routes row).
pub static SKIP: AtomicBool = AtomicBool::new(false);

/// The world map's start of a flight (`(route kind, route stage)`; it never returns: the scene
/// changes to the flight).
const ROUTE_START_RVA: Rva = Rva(0x20EF10);
/// `mov [rsp+8], rbx; push rdi; sub rsp, 0x20`.
const ROUTE_START_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];
/// The world the map was left with (i32, `0x140218E30`'s answer).
const MAP_CHOICE_RVA: Rva = Rva(0x507C94);
/// The world map's current world (i32).
const MAP_WORLD_RVA: Rva = Rva(0x50B1B4);
/// Where to land in a world (its landing list): the landing, or negative (none / backed out).
const LANDING_RVA: Rva = Rva(0x213B20);
/// What the cleared flight calls before landing (`(4)`).
const BEFORE_LANDING_RVA: Rva = Rva(0x266920);
/// Lands at a landing (scene mode 3; never returns).
const LAND_RVA: Rva = Rva(0x20ED90);
/// Back to the world map (scene mode 1; never returns).
const WORLD_MAP_RVA: Rva = Rva(0x20EF60);
/// KH1's worlds (the map's 0x1C entries).
const WORLDS: i32 = 0x1C;

static ORIG_ROUTE_START: AtomicUsize = AtomicUsize::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

pub unsafe fn install() {
    let on = super::settings::get().gummi_skip;
    SKIP.store(on, Ordering::Relaxed);
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let _ = hook::install(base + ROUTE_START_RVA, &ROUTE_START_PROLOGUE, route_start as *const () as usize, &ORIG_ROUTE_START, "Gummi route start (skip)");
    log(&format!("Gummi routes: {}", if on { "skipped (gummi = skip)" } else { "flown (gummi = vanilla)" }));
}

/// The Config menu's Gummi Routes row: skip (true) or fly them, kept in the ini.
pub fn set(skip: bool) {
    SKIP.store(skip, Ordering::Relaxed);
    super::settings::save("gummi", if skip { "skip" } else { "vanilla" });
    log(&format!("Config menu: Gummi routes {}", if skip { "skipped" } else { "flown" }));
}

unsafe extern "system" fn route_start(kind: i32, stage: i32) {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let dest = rd::<i32>(base + MAP_CHOICE_RVA);
    if SKIP.load(Ordering::Relaxed) && (0..WORLDS).contains(&dest) {
        log(&format!("Gummi route (kind {kind}, stage {stage}) to world {dest:#x} skipped: its landing at once"));
        let landing: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(base + LANDING_RVA);
        let at = landing(dest);
        if at < 0 {
            // (No landing, or backed out of it: the world map again, at the destination, as
            // after a flight.)
            log(&format!("Gummi skip: no landing in world {dest:#x} ({at}): the world map"));
            wr(base + MAP_WORLD_RVA, dest);
            let map: unsafe extern "system" fn() = std::mem::transmute(base + WORLD_MAP_RVA);
            map();
        } else {
            let before: unsafe extern "system" fn(i32) = std::mem::transmute(base + BEFORE_LANDING_RVA);
            before(4);
            let land: unsafe extern "system" fn(i32) = std::mem::transmute(base + LAND_RVA);
            land(at);
        }
        // (Neither returns: the scene changes. Should one, the flight as KH1 has it.)
    }
    let f: unsafe extern "system" fn(i32, i32) = std::mem::transmute(ORIG_ROUTE_START.load(Ordering::Acquire));
    f(kind, stage)
}
