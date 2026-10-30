//! Drive > Party Required (the owner, round 7: "make a toggle in drives for parties being required
//! to go into drive forms"): with it on (KH2, the default), a form can be chosen only with the
//! partners KH2's rule needs (`crate::driveparty`, KH2's `0x140400740`): Valor Goofy, Wisdom Donald,
//! Master / Final / Anti both slots filled; Limit no one. Off: as before, whoever is in the party.
//! Notes: `docs/kh1-drive-forms-native.md` §18.
//!
//! KH1 (Steam, Checked (decomp)): the party slots `save + 0x48F` / `+0x490` (0xFF none) hold member ids;
//! a member's record is `save + member × 0x74`, its HP byte `+5` (KH1's HP add for the menu,
//! `0x1402CD5E0`). A partner "can act" while his HP is above 0 (Assumption: KH2's `0x1403C38F0`, which
//! greys the form, is its KO test). Read when the Drive list is built (`drive::usable`), nothing per frame.

use super::log;
use std::sync::atomic::{AtomicU8, Ordering};

const SLOTS_OFF: usize = 0x48E;
const MEMBER_SIZE: usize = 0x74;
const MEMBER_HP: usize = 5;
const MEMBERS: u8 = 10;

/// Drive > Party Required: 0 not read yet, 1 off, 2 on.
static REQUIRED: AtomicU8 = AtomicU8::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

/// Drive > Party Required (the ini's `drive_party = kh2`, the default; `off`).
pub fn required() -> bool {
    match REQUIRED.load(Ordering::Relaxed) {
        0 => {
            let v = super::settings::get().drive_party;
            REQUIRED.store(1 + v as u8, Ordering::Relaxed);
            v
        }
        v => v == 2,
    }
}

/// Switches Drive > Party Required (the Config menu), kept in the ini.
pub fn set_required(v: bool) {
    REQUIRED.store(1 + v as u8, Ordering::Relaxed);
    super::settings::save("drive_party", if v { "kh2" } else { "off" });
    log!(Forms, "Config menu: Party Required {}", if v { "KH2" } else { "Off" });
}

/// The two partner slots: (member, can act), `None` empty.
pub unsafe fn slots() -> [Option<(u8, bool)>; 2] {
    let base = super::takeover::game_base();
    let Some(save) = (base != 0).then(|| super::menu::save_block(base)).flatten() else { return [None, None] };
    let mut out = [None, None];
    for (i, o) in out.iter_mut().enumerate() {
        let m = rd::<u8>(save + SLOTS_OFF + 1 + i);
        if m < MEMBERS && m != 0 {
            *o = Some((m, rd::<u8>(save + m as usize * MEMBER_SIZE + MEMBER_HP) > 0));
        }
    }
    out
}

/// Whether KH2's form `form` (1..6) can be chosen now as far as the party goes.
pub unsafe fn ok(form: u8) -> bool {
    !required() || crate::driveparty::availability(form, slots()) == 0
}

// ---------------------------------------------------------------- the partners leaving (t959)
//
// KH2 (Checked (decomp)): the change takes the partners of `crate::driveparty::takes` with a 15-frame
// fade out (`0x140400990`), and the revert makes them again near Sora with a 15-frame fade in
// (`0x140400440`). Answer 1 (a): they are back while one of KH1's cutscenes runs, and go again after it
// while the form lasts.
//
// On KH1 (Steam, Checked (decomp) and (exe)), each with KH1's own code, as its game over and its revival
// do: out = the model faded to 0 over 15 frames (`0x1402A0F40 (actor, to, frames)`) and the HUD slot faded
// out (`0x140268E30 (slot, 0)`, the slot `0x140285030 (member)`), then the actor's flags pushed on KH1's
// flag stack without "acting" (`0x8000`), model (`0x1`) and shadow (`0x2`), with `0x240` (no foe takes him,
// nothing touches him: KH1's own member-out op `0x1402BB310` sets it) (`0x140292180`; KH1's game over
// pushes its freeze the same way, `0x14028A2B0`); back = the flags popped (`0x140292160`), the model faded
// in over 15 frames and the HUD slot faded in (mode 1), KH1's revival `0x140289520`. KH1's party actors are
// at `0x142D37280` (Sora, then the two). A cutscene: KH1's scene flag `0x142382594` bit 0 (the scripts'
// cutscene-mode opcodes, `cutscenelog`; Assumption: on for every scene that moves a partner). A new room's
// actors come in whole and are taken again.

const PARTY_RVA: usize = 0x2D37280;
const SCENE_RVA: usize = 0x2382594;
const PUSH_RVA: usize = 0x292180;
const PUSH_BYTES: [u8; 7] = [0x4C, 0x63, 0x81, 0x88, 0x03, 0x00, 0x00];
const POP_RVA: usize = 0x292160;
const POP_BYTES: [u8; 7] = [0x48, 0x63, 0x91, 0x88, 0x03, 0x00, 0x00];
const FADE_RVA: usize = 0x2A0F40;
const FADE_BYTES: [u8; 6] = [0x0F, 0x57, 0xC0, 0x0F, 0x2F, 0xD0];
const HUD_FADE_RVA: usize = 0x268E30;
const HUD_FADE_BYTES: [u8; 3] = [0x48, 0x63, 0xC1];
const HUD_SLOT_RVA: usize = 0x285030;
const HUD_SLOT_BYTES: [u8; 3] = [0x4C, 0x8B, 0x0D];
/// The actor's data handle and in it the member id; the flag stack's depth (KH1 keeps 3).
const ACTOR_DATA: usize = 0x130;
const DATA_MEMBER: usize = 0x4C;
const STACK_DEPTH: usize = 0x388;
const STACK_MAX: i32 = 3;
const ACTING: u32 = 0x8000;
const DRAWN: u32 = 0x3;
const UNTOUCHED: u32 = 0x240;
const FADE_FRAMES: f32 = 15.0;

#[derive(Clone, Copy)]
struct Away {
    actor: usize,
    member: u8,
    /// Frames of the fade out left; 0 once the flags are pushed.
    fading: u32,
    pushed: bool,
}

static AWAY: std::sync::Mutex<[Option<Away>; 2]> = std::sync::Mutex::new([None, None]);

unsafe fn calls_ok(base: usize) -> bool {
    super::hook::prologue_is(base + PUSH_RVA, &PUSH_BYTES)
        && super::hook::prologue_is(base + POP_RVA, &POP_BYTES)
        && super::hook::prologue_is(base + FADE_RVA, &FADE_BYTES)
        && super::hook::prologue_is(base + HUD_FADE_RVA, &HUD_FADE_BYTES)
        && super::hook::prologue_is(base + HUD_SLOT_RVA, &HUD_SLOT_BYTES)
}

unsafe fn member_of(actor: usize) -> Option<u8> {
    let data = super::pose::ptr(rd::<u32>(actor + ACTOR_DATA));
    (data >= 0x10000 && super::crash::readable(data + DATA_MEMBER + 1)).then(|| rd::<u16>(data + DATA_MEMBER) as u8)
}

unsafe fn fade(base: usize, actor: usize, to: f32, frames: f32) {
    let f: unsafe extern "system" fn(usize, f32, f32) = std::mem::transmute(base + FADE_RVA);
    f(actor, to, frames);
}

unsafe fn hud(base: usize, member: u8, mode: i32) {
    let slot_of: unsafe extern "system" fn(u32) -> u64 = std::mem::transmute(base + HUD_SLOT_RVA);
    let slot = slot_of(member as u32) as i64 as i32;
    if (0..4).contains(&slot) {
        let f: unsafe extern "system" fn(i32, i32) = std::mem::transmute(base + HUD_FADE_RVA);
        f(slot, mode);
    }
}

/// One partner back (his flags popped if we pushed them, faded in, his HUD slot shown).
unsafe fn back(base: usize, a: Away, why: &str) {
    if !super::crash::readable(a.actor + STACK_DEPTH + 3) {
        return;
    }
    if a.pushed && rd::<i32>(a.actor + STACK_DEPTH) > 0 {
        let pop: unsafe extern "system" fn(usize) = std::mem::transmute(base + POP_RVA);
        pop(a.actor);
    }
    fade(base, a.actor, 0.0, 0.0);
    fade(base, a.actor, 1.0, FADE_FRAMES);
    hud(base, a.member, 1);
    log!(Forms, "Drive: partner {} back ({why})", a.member);
}

/// Every partner the form took back at once (the revert, however it comes).
pub unsafe fn back_all(why: &str) {
    let base = super::takeover::game_base();
    let Ok(mut g) = AWAY.lock() else { return };
    for slot in g.iter_mut() {
        if let Some(a) = slot.take() {
            // (Only an actor still in the party: a room left has new ones.)
            if base != 0 && (1..3).any(|i| rd::<u64>(base + PARTY_RVA + i * 8) as usize == a.actor) {
                back(base, a, why);
            }
        }
    }
}

/// Each frame of a form (`form::native_frame`): the partners KH2's form `form` takes go, and come back
/// for a cutscene.
pub unsafe fn frame(form: u8) {
    let base = super::takeover::game_base();
    if base == 0 || !calls_ok(base) {
        return;
    }
    let scene = rd::<u32>(base + SCENE_RVA) & 1 != 0;
    let Ok(mut g) = AWAY.try_lock() else { return };
    for i in 0..2 {
        let actor = rd::<u64>(base + PARTY_RVA + (1 + i) * 8) as usize;
        // (A new room's actor, or none: ours on the old one is gone with it.)
        if g[i].is_some_and(|a| a.actor != actor) {
            g[i] = None;
        }
        let member = if actor >= 0x10000 { member_of(actor) } else { None };
        let want = !scene && member.is_some_and(|m| crate::driveparty::takes(form, m));
        match (g[i], want) {
            (None, true) => {
                let m = member.unwrap_or(0);
                fade(base, actor, 0.0, FADE_FRAMES);
                hud(base, m, 0);
                g[i] = Some(Away { actor, member: m, fading: FADE_FRAMES as u32, pushed: false });
                log!(Forms, "Drive: partner {m} leaves for the form");
            }
            (Some(a), false) => {
                g[i] = None;
                back(base, a, if scene { "a cutscene" } else { "not taken" });
            }
            (Some(mut a), true) if !a.pushed => {
                a.fading = a.fading.saturating_sub(1);
                if a.fading == 0 && rd::<i32>(actor + STACK_DEPTH) < STACK_MAX {
                    let push: unsafe extern "system" fn(usize, u32) = std::mem::transmute(base + PUSH_RVA);
                    push(actor, (rd::<u32>(actor) & !(ACTING | DRAWN)) | UNTOUCHED);
                    a.pushed = true;
                }
                g[i] = Some(a);
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------- the Olympus Coliseum (t961)
//
// Row 7 (the owner, round 8 answer 3 (a), main's rule s332: "if you enter the cup in a drive form you
// should be reverted before the first battle, but if you defeat a round in a drive form it should carry
// over into the next round"). KH1 (Steam, Checked (decomp)): the world and room now at `0x14233FE84` /
// `0x14233FE8C` (KH1's own test for the Coliseum's arena, `0x1402A10B0`: world 0xB, room 2); the Coliseum
// is world 0xB, its gates room 0 and lobby room 1, its matches in rooms 2 and up (Assumption from main's
// logs: the Pegasus Cup's arena is room 5). A form carries through the rounds already (a cutscene keeps
// it, row 11; a battle's end doesn't revert, row 6); here only the cup's start: into an arena room from
// the gates or the lobby in a form, the form ends with its bars refilled and the partners back.

const WORLD_RVA: usize = 0x233FE84;
const ROOM_RVA: usize = 0x233FE8C;
const COLISEUM: u32 = 0xB;
const FIRST_ARENA_ROOM: u32 = 2;

/// The world and room last seen in a form.
static LAST_ROOM: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// In a form, each frame: true once when Sora goes from the Coliseum's gates or lobby into an arena room.
pub unsafe fn cup_start() -> bool {
    let base = super::takeover::game_base();
    if base == 0 {
        return false;
    }
    let (world, room) = (rd::<u32>(base + WORLD_RVA), rd::<u32>(base + ROOM_RVA));
    let now = (world as u64) << 32 | room as u64;
    let was = LAST_ROOM.swap(now, Ordering::Relaxed);
    let (was_world, was_room) = ((was >> 32) as u32, was as u32);
    was != 0 && world == COLISEUM && was_world == COLISEUM && was_room < FIRST_ARENA_ROOM && room >= FIRST_ARENA_ROOM
}

/// The form ended: no room remembered (the next form starts fresh).
pub fn forget_room() {
    LAST_ROOM.store(0, Ordering::Relaxed);
}

/// One of KH1's cutscenes runs (its scene flag `0x142382594` bit 0): the form gauge doesn't drain (KH2's
/// drain runs in the player's own step, which an event holds; Assumption: KH2's `0x1403A92C0` check
/// `0x1403AC220` is that), main's "no drain in the victory scene".
pub fn scene() -> bool {
    let base = super::takeover::game_base();
    base != 0 && unsafe { rd::<u32>(base + SCENE_RVA) } & 1 != 0
}
