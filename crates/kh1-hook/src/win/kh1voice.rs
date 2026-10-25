//! Sora's KH1 voice (the user: "sfx is calling kh2 battle quotes but I need kh1 battle quotes to
//! be called"), played by KH1's own code, `docs/kh1-voice.md`:
//!
//! - **Groups** (`0x14028E060 (actor, group, other)`): a speaker's voice table is in `btltbl.bin`
//!   at the offset `0x140529560[character]` gives (Sora's `0xBC48`, 20 groups), 11 bytes a
//!   group: a priority, then five (clip, weight) pairs. KH1 picks one clip: a weight above 0 is a
//!   percentage chance, 0 an equal share of what's left; clip 0xFD "the other's line", 0xFE
//!   silence, 0xFF none. So a group is already a random choice of clips.
//! - **Clips** (`0x14028E7A0 (actor, clip, priority)`): one clip of the speaker's bank, played if
//!   nothing of a higher priority is speaking for him.
//!
//! Sora's groups, by what calls them: 0 / 1 hurt (light, heavy), 2 knocked out, 4 his combo
//! swings (motion 200's row: clips 21, 22, 23, 24, 28, an equal share each), 5 the finishers,
//! 6 the air finishers, 7..13 the seven spells (Cure's and Aero's on an ally another line), 15 / 17
//! the summons' calls (the summon sequences `0x1403959B0`.. for xs_dumbo .. xs_simba): 17 is clip
//! 37 three times in four, else the plain grunt 30; clip 37, which no other group plays, is his
//! "Give me strength!".

// (`give_me_strength` and `dodge_roll` are called from the Drive's start and KH2's dodge roll.)
#![allow(dead_code)]

use super::addr::Rva;
use super::log;
use std::sync::atomic::{AtomicU32, Ordering};

/// KH1's voice group play `(actor, group, other actor or 0)`.
const GROUP_RVA: Rva = Rva(0x28E060);
/// KH1's voice clip play `(actor, clip, priority)`.
const CLIP_RVA: Rva = Rva(0x28E7A0);
/// Sora's combo swing grunts (motion 200's attack row, its voice byte `+0xF`).
pub const SWING_GROUP: i32 = 4;
/// "Give me strength!": the summons' call (group 17)'s own clip, and that group's priority.
pub const GIVE_ME_STRENGTH_CLIP: i32 = 37;
const CALL_PRIORITY: i32 = 3;
/// "Defense!": one of Sora's two own Aero clips (group 13: clips 46 / 47, 50 / 50, `btltbl.bin`,
/// Checked (files)); which of the two is "Defense!" is not heard yet (**Assumption**: 46), so
/// every play is logged with its clip. The spell rows' priority is 3.
pub const DEFENSE_CLIP: i32 = 46;
const SPELL_PRIORITY: i32 = 3;

fn base() -> usize {
    unsafe { super::sys::GetModuleHandleA(std::ptr::null()) as usize }
}

/// Whether `actor` can speak now: a live actor with its stats and object (the voice code reads
/// `+0x6C` and `+0x130`).
unsafe fn speaker(actor: usize) -> bool {
    actor >= 0x10000
        && super::crash::readable(actor)
        && super::crash::readable(actor + 0x370)
        && std::ptr::read_unaligned((actor + 0x6C) as *const u32) != 0
        && std::ptr::read_unaligned((actor + 0x130) as *const u32) != 0
}

/// Logs the first few calls of each kind.
fn note(n: &AtomicU32, line: impl FnOnce() -> String) {
    if n.fetch_add(1, Ordering::Relaxed) < 6 {
        log(&format!("KH1 voice: {}", line()));
    }
}

/// KH1's voice group `group` on `actor` (KH1 picks the clip). Does nothing for no actor.
pub unsafe fn play(actor: usize, group: i32) {
    if !speaker(actor) || group < 0 {
        return;
    }
    let f: unsafe extern "system" fn(usize, i32, usize) = std::mem::transmute(base() + GROUP_RVA);
    f(actor, group, 0);
}

/// KH1's voice clip `clip` on `actor` at `priority`.
pub unsafe fn clip(actor: usize, clip: i32, priority: i32) {
    if !speaker(actor) || clip < 0 {
        return;
    }
    let f: unsafe extern "system" fn(usize, i32, i32) = std::mem::transmute(base() + CLIP_RVA);
    f(actor, clip, priority);
}

/// One of his combo swing grunts, at random (KH1's own pick of group 4).
pub unsafe fn swing_grunt(actor: usize) {
    static N: AtomicU32 = AtomicU32::new(0);
    note(&N, || format!("a swing grunt (group {SWING_GROUP})"));
    play(actor, SWING_GROUP);
}

/// "Give me strength!" (the user: when he changes into a Drive Form).
pub unsafe fn give_me_strength(actor: usize) {
    static N: AtomicU32 = AtomicU32::new(0);
    note(&N, || format!("\"Give me strength!\" (clip {GIVE_ME_STRENGTH_CLIP})"));
    clip(actor, GIVE_ME_STRENGTH_CLIP, CALL_PRIORITY);
}

/// The dodge roll's call (the user: "make the sound one of the grunts during his first keyblade
/// swing and allow him to randomize which grunts he picks").
pub unsafe fn dodge_roll(actor: usize) {
    static N: AtomicU32 = AtomicU32::new(0);
    note(&N, || "the dodge roll's grunt".into());
    play(actor, SWING_GROUP);
}

/// Reflect's finisher with Battle Quotes KH1 (the tester: "the finisher will say "defence!" its
/// one of the lines he picks when casting aero"): KH1's Aero clip, through KH1's clip play.
pub unsafe fn defense(actor: usize) {
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 40 {
        log(&format!("KH1 voice: Reflect finisher, KH1's Aero line \"Defense!\" (clip {DEFENSE_CLIP}, priority {SPELL_PRIORITY}; if it isn't \"Defense!\" it is the other Aero clip, 47)"));
    }
    clip(actor, DEFENSE_CLIP, SPELL_PRIORITY);
}
