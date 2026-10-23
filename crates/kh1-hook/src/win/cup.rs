//! A Drive Form through the Olympus Coliseum's rounds (the user: "in the coliseum, you should be
//! able to maintain your drive form throughout rounds. if you enter the cup in a drive form you
//! should be reverted before the first battle, but if you defeat a round in a drive form it should
//! carry over into the next round"). `docs/drive-model.md`, "Drive Forms in the Coliseum".
//!
//! KH1's Coliseum is world `0xB` (`WORLD_RVA`): its gates are room 0, its lobby (Phil, the cups'
//! menu) room 1, and every match is fought in an arena room of its own set (2 and up; the Pegasus
//! Cup's is 5, the s314 / s315 logs). The cases:
//! - **the cup's start**: into the arena from the gates or the lobby in a form: back to Sora there
//!   and then, before the first battle (`room`);
//! - **a round won**: its victory scene (KH1's script holds Sora, `takeover::scene_frame`) and the
//!   next round's setup don't end the form (`carries`); its form gauge doesn't drain while KH1's
//!   script has him there (`gauge_held`). The partners the form took are back for the scene
//!   (`vanish`: it waits on their poses) and go again once it is over;
//! - **the cup's end**: a cup's state byte (`crate::story::CUPS`, stepped by the cup's script when
//!   it is won) changing while in the arena: the scene rules apply again, so the trophy scene ends
//!   the form as any battle's end does;
//! - **leaving the arena** (the cup over, lost or quit): back to Sora (`room`).

use super::addr::Rva;
use super::log;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

/// KH1's world and room now (`takeover`'s room line reads them too).
const WORLD_RVA: Rva = Rva(0x2340E5C);
const ROOM_RVA: Rva = Rva(0x2340EC4);
/// Olympus Coliseum's world id (the s315 log: its gates room 0, lobby 1, the Pegasus Cup's arena 5).
pub const COLISEUM: u32 = 0xB;
/// The Coliseum's rooms from this one on are its arena's (each match set's own room).
const FIRST_ARENA_ROOM: u32 = 2;
/// The four cups' state bytes (`crate::story::CUPS`: Phil, Pegasus, Hercules, Hades).
const CUP_COUNT: usize = 4;

/// The last world and room seen, and the cups' states as the arena was entered (or last seen).
struct Seen {
    world: u32,
    room: u32,
    cups: Option<[u8; CUP_COUNT]>,
}

static SEEN: Mutex<Option<Seen>> = Mutex::new(None);
/// A cup's state changed in the arena (the cup won): its scenes end the form again.
static CUP_DONE: AtomicBool = AtomicBool::new(false);
/// The round scene that last kept the form (`carries`), for a line per scene.
static CARRIED_LOGGED: AtomicU32 = AtomicU32::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

fn base() -> usize {
    unsafe { super::sys::GetModuleHandleA(std::ptr::null()) as usize }
}

/// Whether `(world, room)` is one of the Coliseum's arena rooms.
pub fn arena(world: u32, room: u32) -> bool {
    world == COLISEUM && room >= FIRST_ARENA_ROOM
}

unsafe fn now() -> (u32, u32) {
    let b = base();
    (rd::<u32>(b + WORLD_RVA), rd::<u32>(b + ROOM_RVA))
}

unsafe fn cups() -> Option<[u8; CUP_COUNT]> {
    let save = super::menu::save_block(base())?;
    let at = save + crate::story::CUPS;
    super::crash::readable(at + CUP_COUNT - 1).then(|| rd::<[u8; CUP_COUNT]>(at))
}

/// Sora in the arena now, with no cup won since he came in: a round's end keeps the form.
pub fn in_rounds() -> bool {
    let (w, r) = unsafe { now() };
    arena(w, r) && !CUP_DONE.load(Ordering::Relaxed)
}

/// A scene would end the form now (`takeover::scene_frame`, `why` its reason): in the arena's rounds
/// it doesn't (a line per scene). Returns whether the form is kept.
pub fn carries(why: &str) -> bool {
    if !in_rounds() || !super::form::in_form() {
        return false;
    }
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed) as u32;
    // (Once per scene: a frame not right after the last one logged is a new scene.)
    let last = CARRIED_LOGGED.swap(frame, Ordering::Relaxed);
    if frame.wrapping_sub(last) > 1 {
        log(&format!(
            "Coliseum: a round's scene ({why}) in {}: the form carries over into the next round (no revert)",
            super::form::current().map_or("the form", |k| k.name())
        ));
    }
    true
}

/// The form gauge waits while KH1's script has Sora between the arena's rounds (its victory scene,
/// the next round's setup), in a form.
pub fn gauge_held() -> bool {
    super::form::in_form() && super::takeover::script_has_sora() && in_rounds()
}

/// Each of Sora's frames (`takeover`): the room's change and the cups' states. Returns why the form
/// ends now (into the arena from outside it, out of it), if it does.
pub fn room() -> Option<String> {
    let (world, room) = unsafe { now() };
    let cups = unsafe { cups() };
    let Ok(mut g) = SEEN.try_lock() else { return None };
    let Some(s) = g.as_mut() else {
        *g = Some(Seen { world, room, cups });
        return None;
    };
    let mut why = None;
    if (s.world, s.room) != (world, room) {
        let (was, is) = (arena(s.world, s.room), arena(world, room));
        if is && !was {
            CUP_DONE.store(false, Ordering::Relaxed);
            if super::form::in_form() || super::form::busy() {
                log(&format!("Coliseum: into the arena (room {:#x} -> {room:#x}) in a form: back to Sora before the first battle", s.room));
                why = Some("the Coliseum's arena entered (a cup starts)".to_string());
            }
        } else if was && !is {
            if super::form::in_form() || super::form::busy() {
                log(&format!("Coliseum: out of the arena (world {:#x} room {:#x} -> world {world:#x} room {room:#x}) in a form: back to Sora", s.world, s.room));
                why = Some("the Coliseum's arena left (the cup over)".to_string());
            }
        } else if was && is {
            log(&format!("Coliseum: arena room {:#x} -> {room:#x}: still the arena (the form, if any, carries on)", s.room));
        }
        s.world = world;
        s.room = room;
        s.cups = cups;
        return why;
    }
    // A cup's state stepped while in the arena: the cup won (the trophy scene ends the form).
    if arena(world, room) {
        if let (Some(before), Some(now)) = (s.cups, cups) {
            if before != now && !CUP_DONE.swap(true, Ordering::Relaxed) {
                log(&format!(
                    "Coliseum: a cup's state changed ({before:?} -> {now:?}): the cup won, its scene ends the form as a battle's end does"
                ));
            }
        }
    }
    if cups.is_some() {
        s.cups = cups;
    }
    why
}
