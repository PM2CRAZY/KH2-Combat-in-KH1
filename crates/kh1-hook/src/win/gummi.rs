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
//!
//! (t1068) The route's own event scripts too (`route_scripts`): a flight runs its stage's
//! departure script (event 0x40, `worldmap/ev/wld_out_*`) when it starts and its arrival script
//! (event 0x41, `wld_in_*`) after the landing list, with KH1's flag word `0x142867364` bit 8 set
//! between (Steam `0x14023FBB0`, Checked (decomp)). The skip ran neither, so the story steps those
//! scripts set never happened (the owner, answer (b): a story step after the first arrival).
//! Steam addresses only.

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
    log!(Menus, "Gummi routes: {}", if on { "skipped (gummi = skip)" } else { "flown (gummi = vanilla)" });
}

/// The ini read again (t1086: `experimental` turned on or off): skip or fly, without writing the ini.
pub fn reload(skip: bool) {
    if SKIP.swap(skip, Ordering::Relaxed) != skip {
        log!(Menus, "Gummi routes: {} (the ini read again)", if skip { "skipped" } else { "flown" });
    }
}

/// The Config menu's Gummi Routes row: skip (true) or fly them, kept in the ini.
pub fn set(skip: bool) {
    SKIP.store(skip, Ordering::Relaxed);
    super::settings::save("gummi", if skip { "skip" } else { "vanilla" });
    log!(Menus, "Config menu: Gummi routes {}", if skip { "skipped" } else { "flown" });
}

// ---------------------------------------------------------------------------------------------
// (t1068) The route's event scripts (Steam KH1, Checked (decomp)):
// - The route start `0x140211070 (route, _)` keeps its first argument at `0x14268C47C`; the flight's
//   setup (`0x140241310`) reads the route's record from `0x14050AF00 + route * 0x58` (its name
//   `route_tw_1`, ...) and `0x140213AA0` loads two scripts named in it with `0x140214780 (buffer,
//   name)` (`worldmap/ev/<name>`) into the scene's script buffer (`0x14268C438`, 0x1000 a script):
//   `+0x34` (`wld_out_*`) as event 0x40 (`0x14268E110`), `+0x14` (`wld_in_*`) as event 0x41
//   (`0x14268E118`); a name `-` or empty is none (Checked (exe): routes 3..15 have both, 16..18
//   only 0x40, the others none).
// - `0x140213150 (event, mode)` runs one (KH1's script machine, waiting frames while it runs). The
//   flight runs 0x40 at its start (`0x14023DF30`) and, once cleared (`0x14023FBB0`): the landing
//   list, `0x142867364 |= 8`, then 0x41, then the landing or the world map.
// The skip loads both into the buffer's first two slots (the world map's own two scripts there,
// `wld_st_event` / `gumi_manual_2` from `0x140213CA0`, are loaded again when the map next starts:
// Assumption) after the landing list and runs them in the flight's order.
// ---------------------------------------------------------------------------------------------

const LOAD_SCRIPT_STEAM: usize = 0x214780;
/// `push rbx; sub rsp, 0x90; mov rax, [rip+..]`.
const LOAD_SCRIPT_BYTES: [u8; 9] = [0x40, 0x53, 0x48, 0x81, 0xEC, 0x90, 0x00, 0x00, 0x00];
const RUN_EVENT_STEAM: usize = 0x213150;
/// `mov [rsp+18h], rbx; mov [rsp+20h], rbp; mov [rsp+8], ecx; push rdi`.
const RUN_EVENT_BYTES: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x18, 0x48, 0x89, 0x6C, 0x24, 0x20, 0x89, 0x4C, 0x24, 0x08, 0x57];
const SCRIPT_BUFFER_STEAM: usize = 0x268C438;
const EVENT_40_STEAM: usize = 0x268E110;
const EVENT_41_STEAM: usize = 0x268E118;
const FLAGS_STEAM: usize = 0x2867364;
const ROUTES_STEAM: usize = 0x50AF00;
const ROUTE_SIZE: usize = 0x58;
/// The route records with names (`test1` .. `route_pirate_1`, Checked (exe)).
const ROUTES: i32 = 21;

unsafe fn route_scripts(base: usize, route: i32) {
    if super::addr::Build::current() != super::addr::Build::Steam || !(0..ROUTES).contains(&route) {
        return;
    }
    if !hook::prologue_is(base + LOAD_SCRIPT_STEAM, &LOAD_SCRIPT_BYTES) || !hook::prologue_is(base + RUN_EVENT_STEAM, &RUN_EVENT_BYTES) {
        log!(Menus, "Gummi skip: KH1's route script calls aren't as expected: route {route}'s scripts not run");
        return;
    }
    let buffer = rd::<usize>(base + SCRIPT_BUFFER_STEAM);
    if buffer < 0x10000 {
        log!(Menus, "Gummi skip: no script buffer: route {route}'s scripts not run");
        return;
    }
    let load: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + LOAD_SCRIPT_STEAM);
    let run: unsafe extern "system" fn(i32, i32) = std::mem::transmute(base + RUN_EVENT_STEAM);
    let record = base + ROUTES_STEAM + route as usize * ROUTE_SIZE;
    let name = |off: usize| {
        let p = record + off;
        let s = std::slice::from_raw_parts(p as *const u8, 0x20);
        let n = s.iter().position(|&c| c == 0).unwrap_or(0);
        (n > 0 && s[0] != b'-').then(|| (p, String::from_utf8_lossy(&s[..n]).into_owned()))
    };
    let mut said = Vec::new();
    for (slot, (off, at, event)) in [(0x34usize, EVENT_40_STEAM, 0x40), (0x14, EVENT_41_STEAM, 0x41)].into_iter().enumerate() {
        wr(base + at, 0usize);
        if event == 0x41 {
            wr(base + FLAGS_STEAM, rd::<u32>(base + FLAGS_STEAM) | 8);
        }
        let Some((p, n)) = name(off) else { continue };
        let into = buffer + slot * 0x1000;
        load(into, p);
        wr(base + at, into);
        run(event, 0);
        said.push(format!("{n} (event {event:#x})"));
    }
    log!(Menus, "Gummi skip: route {route}'s scripts run as after a flight: {}", if said.is_empty() { "none".to_string() } else { said.join(", ") });
}

unsafe extern "system" fn route_start(kind: i32, stage: i32) {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let dest = rd::<i32>(base + MAP_CHOICE_RVA);
    if SKIP.load(Ordering::Relaxed) && (0..WORLDS).contains(&dest) {
        log!(Menus, "Gummi route (kind {kind}, stage {stage}) to world {dest:#x} skipped: its landing at once");
        let landing: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(base + LANDING_RVA);
        let at = landing(dest);
        // (t1068) The route's departure and arrival scripts, as the flight runs them (the arrival
        // one after the landing list, whatever it answered).
        route_scripts(base, kind);
        if at < 0 {
            // (No landing, or backed out of it: the world map again, at the destination, as
            // after a flight.)
            log!(Menus, "Gummi skip: no landing in world {dest:#x} ({at}): the world map");
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
