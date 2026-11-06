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

use super::addr::St;
use super::log;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

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
//
// t1059, a new room in a form (the owner's t1058 log, Checked (log): a room entered at f1166, the partners
// taken again only at f1185; one at f1547 never). KH1 (Steam, Checked (decomp)): an actor's start
// `0x140292660` (from its creation `0x140290D60` and the actor command 0x20 of `0x14028E6E0`) sets it acting
// again and zeroes its flag stack's depth `+0x388`, so a new room can rebuild a partner at the same address
// and our record of him still said "taken"; and `form::native_frame` runs only while KH1's states have Sora,
// not while a room's entry script moves him (f1166..f1185). Now a record whose actor's stack is below the
// depth our push left is his rebuild (dropped without a pop: the pushed flags went with the old actor) and
// he is taken again at once; on each room change (`room_entered`, from `takeover`'s room watch, once a room)
// the partners are taken at once (model to 0 over 0 frames, HUD slot hidden, flags pushed the same frame)
// and for the arrival window (`ARRIVAL_FRAMES`, also stepped while a script has Sora, `arrival_frame`) KH1's
// scene flag is ignored so no one pops in on arrival. KH1's room start `0x1401A38A0` clears the flag
// (Checked (decomp)); the window ends early once the flag has been on and cleared (the arrival's scene
// over), so a cutscene later in the room still brings them back. The Coliseum's rounds are event entries in
// the same room (`ingameskip`, Checked (files) at t764), so a won round's scene is never inside a window.

const PARTY_RVA: St = St(0x2D37280);
const SCENE_RVA: St = St(0x2382594);
const PUSH_RVA: St = St(0x292180);
const PUSH_BYTES: [u8; 7] = [0x4C, 0x63, 0x81, 0x88, 0x03, 0x00, 0x00];
const POP_RVA: St = St(0x292160);
const POP_BYTES: [u8; 7] = [0x48, 0x63, 0x91, 0x88, 0x03, 0x00, 0x00];
const FADE_RVA: St = St(0x2A0F40);
const FADE_BYTES: [u8; 6] = [0x0F, 0x57, 0xC0, 0x0F, 0x2F, 0xD0];
const HUD_FADE_RVA: St = St(0x268E30);
const HUD_FADE_BYTES: [u8; 3] = [0x48, 0x63, 0xC1];
const HUD_SLOT_RVA: St = St(0x285030);
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
/// After a room change in a form: KH1's scene flag ignored this long (60 Hz frames, 1 s; Assumption: an
/// arrival's script scene sets it within this, the owner's log had KH1's script moving Sora for 19 frames).
const ARRIVAL_FRAMES: u32 = 60;

#[derive(Clone, Copy)]
struct Away {
    actor: usize,
    member: u8,
    /// Frames of the fade out left; 0 once the flags are pushed.
    fading: u32,
    pushed: bool,
    /// KH1's flag stack depth right after our push (lower later: the actor was started again).
    depth: i32,
    /// "The stack is full" logged for this actor (once a room: a new room starts a new record).
    refused: bool,
}

static AWAY: std::sync::Mutex<[Option<Away>; 2]> = std::sync::Mutex::new([None, None]);
/// The form `frame` last ran for (KH2's id), 0 after a revert.
static FORM_NOW: AtomicU8 = AtomicU8::new(0);
/// 60 Hz frames left of a new room's arrival window; whether the scene flag was on in it.
static ARRIVAL: AtomicU32 = AtomicU32::new(0);
static ARRIVAL_SCENE: AtomicBool = AtomicBool::new(false);

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
    FORM_NOW.store(0, Ordering::Relaxed);
    ARRIVAL.store(0, Ordering::Relaxed);
    KEYBLADE_DUE.store(false, Ordering::Relaxed);
    WAITED_AT.store(0, Ordering::Relaxed);
    SORA_WAITS.store(0, Ordering::Relaxed);
    SORA_WAIT_LOGGED.store(false, Ordering::Relaxed);
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
    FORM_NOW.store(form, Ordering::Relaxed);
    let base = super::takeover::game_base();
    if base == 0 || !calls_ok(base) {
        return;
    }
    // (t1062: Sora's Keyblade back out in a new room.)
    keyblade_due(base);
    let Ok(mut g) = AWAY.try_lock() else { return };
    step(base, form, &mut g);
}

/// Our push of KH1's flags on a partner (without acting, model and shadow, with `0x240`), if KH1's
/// stack has room; full: logged once for this record.
unsafe fn try_push(base: usize, a: &mut Away) {
    let depth = rd::<i32>(a.actor + STACK_DEPTH);
    if depth < STACK_MAX {
        let push: unsafe extern "system" fn(usize, u32) = std::mem::transmute(base + PUSH_RVA);
        push(a.actor, (rd::<u32>(a.actor) & !(ACTING | DRAWN)) | UNTOUCHED);
        a.pushed = true;
        a.depth = depth.max(0) + 1;
    } else if !a.refused {
        a.refused = true;
        log!(Forms, "Drive: partner {} (actor {:#x}) not taken yet: KH1's flag stack is full (depth {depth} of {STACK_MAX}, flags {:08x})", a.member, a.actor, rd::<u32>(a.actor));
    }
}

/// The partner check (`frame`, and in a new room's arrival window `arrival_frame` / `room_entered`).
unsafe fn step(base: usize, form: u8, g: &mut [Option<Away>; 2]) {
    let mut scene = rd::<u32>(base + SCENE_RVA) & 1 != 0;
    // (The Coliseum's win: t1073's log, a cup with Donald and Goofy in Master Form: the win after a
    // later round set no scene flag; KH1 started the partners again for its win scene (f19000 /
    // f19002) and we took them again at once, so its waits on their motions (event function 103,
    // f19046..) never ended. t1073's test (an event word set) can't be seen here: `takeover` doesn't
    // run the form's frame while that word is set. Now KH1's own wait on a taken partner's motion
    // brings him back (`wait_motion_fn`), and he stays back while that wait still runs or Sora can't
    // act: the scene's hold.)
    if !scene && script_waits() {
        scene = true;
    }
    // (t1059: a new room's first moments: KH1's scene flag ignored until its arrival scene is over.)
    if ARRIVAL.load(Ordering::Relaxed) > 0 {
        if scene {
            ARRIVAL_SCENE.store(true, Ordering::Relaxed);
            scene = false;
        } else if ARRIVAL_SCENE.load(Ordering::Relaxed) {
            ARRIVAL.store(0, Ordering::Relaxed);
        }
    }
    let arriving = ARRIVAL.load(Ordering::Relaxed) > 0;
    for i in 0..2 {
        let actor = rd::<u64>(base + PARTY_RVA + (1 + i) * 8) as usize;
        // (A new room's actor, or none: ours on the old one is gone with it. t1059: the same actor
        // started again by KH1, its flag stack below our push: our flags went with the old one.)
        let rebuilt = g[i].is_some_and(|a| a.actor == actor && a.pushed && rd::<i32>(actor + STACK_DEPTH) < a.depth);
        if rebuilt || g[i].is_some_and(|a| a.actor != actor) {
            g[i] = None;
        }
        let member = if actor >= 0x10000 { member_of(actor) } else { None };
        let want = !scene && member.is_some_and(|m| crate::driveparty::takes(form, m));
        match (g[i], want) {
            (None, true) => {
                let m = member.unwrap_or(0);
                // (t1059: in a new room or after a rebuild at once, no fade: he must not be seen.)
                let now = arriving || rebuilt;
                fade(base, actor, 0.0, if now { 0.0 } else { FADE_FRAMES });
                hud(base, m, 0);
                let mut a = Away { actor, member: m, fading: if now { 0 } else { FADE_FRAMES as u32 }, pushed: false, depth: 0, refused: false };
                log!(Forms, "Drive: partner {m} leaves for the form (actor {actor:#x}{})", if rebuilt { ", started again by KH1: at once" } else if now { ", a new room: at once" } else { "" });
                if now {
                    try_push(base, &mut a);
                }
                g[i] = Some(a);
            }
            (Some(a), false) => {
                g[i] = None;
                back(base, a, if scene { "a cutscene" } else { "not taken" });
            }
            (Some(mut a), true) if !a.pushed => {
                a.fading = a.fading.saturating_sub(1);
                if a.fading == 0 {
                    try_push(base, &mut a);
                }
                g[i] = Some(a);
            }
            _ => {}
        }
    }
}

/// A room entered (`takeover`'s room watch, once a room; `was` the room before): in a form, the
/// arrival window opens and the partners are taken at once (t1059). Not where the form ends on
/// arrival anyway (the Coliseum's cup start, a world left with Gauge Refill KH2: `form::native_frame`).
pub unsafe fn room_entered(world: u32, room: u32, was: Option<(u32, u32)>) {
    let form = FORM_NOW.load(Ordering::Relaxed);
    if form == 0 || !super::form::in_form() {
        return;
    }
    if let Some((was_world, was_room)) = was {
        let cup = world == COLISEUM && was_world == COLISEUM && was_room < FIRST_ARENA_ROOM && room >= FIRST_ARENA_ROOM;
        if cup || (was_world != world && super::form::gauge_refill()) {
            return;
        }
    }
    let base = super::takeover::game_base();
    if base == 0 || !calls_ok(base) {
        return;
    }
    ARRIVAL_SCENE.store(false, Ordering::Relaxed);
    ARRIVAL.store(ARRIVAL_FRAMES, Ordering::Relaxed);
    // (t1062: his Keyblade out again once the form's own frame runs in this room, `keyblade_due`.)
    KEYBLADE_DUE.store(true, Ordering::Relaxed);
    log!(Forms, "Drive: world {world:#x} room {room:#x} entered in a form: the partners taken at once, KH1's scene flag ignored up to {ARRIVAL_FRAMES} frames");
    let Ok(mut g) = AWAY.try_lock() else { return };
    step(base, form, &mut g);
}

/// Each of Sora's frames (`takeover::physics`, whoever has him): nothing but in a new room's arrival
/// window, where the partner check runs even while KH1's script moves Sora (t1059).
pub unsafe fn arrival_frame() {
    let left = ARRIVAL.load(Ordering::Relaxed);
    if left == 0 {
        return;
    }
    ARRIVAL.store(left.saturating_sub(super::takeover::steps().max(1)), Ordering::Relaxed);
    let form = FORM_NOW.load(Ordering::Relaxed);
    let base = super::takeover::game_base();
    if form == 0 || !super::form::in_form() || base == 0 || !calls_ok(base) {
        ARRIVAL.store(0, Ordering::Relaxed);
        return;
    }
    let Ok(mut g) = AWAY.try_lock() else { return };
    step(base, form, &mut g);
}

// ---------------------------------------------------------------- KH1's scripts waiting on a partner
//
// The owner: "in Olympus coliseum during the win animation if your in a drive form then the game soft
// locks". KH1 (Steam, Checked (decomp + exe), `docs/kh1-chest.md`): event function 103 (`0x1401A96F0`,
// entry 103 of the event function table `0x1404FA870`) waits (returns 4, run again next frame) until the
// motion of the context's actor (`ctx + 8`) is over, then goes on (2). The Coliseum's win scene waits so
// on each member's pose (Checked (log): t1073's, event function 103 from f19046 on with both partners
// taken). A partner we took has his flags pushed without "acting" (`0x8000`), so his motion never steps
// and never ends. t1073 counted "an event runs and Sora can't act" as a cutscene for the partner check,
// but the check runs in the form's frame, which `takeover` skips while KH1's event word is set
// (`takeover::takeover`, "an event runs"), so it could never be true there (Checked (code)).
// Now entry 103 of KH1's table points at ours (one data word; KH1's function still does the wait): a
// wait on a partner the form took brings him back there and then (`back`, KH1's pop, fade in and HUD
// fade), and the partner check counts it as a cutscene while such a wait ran in the last `WAIT_GRACE`
// frames or Sora can't act (KH1's can-act test, the scene's hold); then the form takes them again with
// KH2's 15-frame fade (`step`). Anywhere, not only in the Coliseum: a script that waits on a partner's
// motion would hang the same way. A wait on Sora himself in a form that is still waiting after
// `SORA_WAIT_LOG` calls is logged once with his motion (Forms), in case the form's own motion is what
// doesn't end in a scene. Per frame: nothing outside KH1's motion waits; in one, a word read and a
// compare with the two partner actors.

const EVENT_TABLE_STEAM: St = St(0x4FA870);
const FN_WAIT_MOTION: usize = 103;
const WAIT_MOTION_STEAM: St = St(0x1A96F0);
/// A script context's actor (`ctx + 8`, what event function 103 tests; Checked (decomp), `chest.rs`).
const CTX_ACTOR: usize = 8;
/// After the last wait on a partner, 60 Hz frames before the form may take them again (with Sora
/// able to act). Assumption: KH1's waits on a pose come back every frame while they wait, so a scene
/// still waiting is never more than a frame or two apart.
const WAIT_GRACE: u64 = 30;
/// Calls of event function 103 on Sora in a form still waiting before the diagnostic line.
const SORA_WAIT_LOG: u32 = 600;
static ORIG_WAIT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// `takeover::TICKS` + 1 of the last wait on a partner while a form ran; 0 none.
static WAITED_AT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static SORA_WAITS: AtomicU32 = AtomicU32::new(0);
static SORA_WAIT_LOGGED: AtomicBool = AtomicBool::new(false);

/// Steam only: entry 103 of KH1's event function table pointed at `wait_motion_fn`, if it holds KH1's
/// function 103 where this exe has it.
pub unsafe fn install(base: usize) {
    if super::addr::Build::current() == super::addr::Build::Unknown {
        return;
    }
    let at = base + EVENT_TABLE_STEAM + FN_WAIT_MOTION * 8;
    let kh1 = (base + WAIT_MOTION_STEAM) as u64;
    ORIG_WAIT.store(kh1 as usize, Ordering::Release);
    if !super::hook::patch(at, &kh1.to_le_bytes(), &(wait_motion_fn as *const () as u64).to_le_bytes(), "event function 103 (Drive partners)") {
        log!(Forms, "Drive: KH1's event function 103 isn't where expected: a scene's wait on a partner the form took isn't seen");
    }
}

/// Event function 103 through the table: a wait on a partner the form took brings him back first; then
/// KH1's own wait.
unsafe extern "system" fn wait_motion_fn(ctx: usize) -> u64 {
    let f: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(ORIG_WAIT.load(Ordering::Acquire));
    let form = FORM_NOW.load(Ordering::Relaxed);
    let base = super::takeover::game_base();
    let actor = if form != 0 && base != 0 && super::crash::readable(ctx + CTX_ACTOR + 7) { rd::<u64>(ctx + CTX_ACTOR) as usize } else { 0 };
    if actor < 0x10000 {
        return f(ctx);
    }
    if (1..3).any(|i| rd::<u64>(base + PARTY_RVA + i * 8) as usize == actor) {
        let first = WAITED_AT.swap(super::takeover::TICKS.load(Ordering::Relaxed) + 1, Ordering::Relaxed) == 0;
        if let Ok(mut g) = AWAY.try_lock() {
            if let Some(slot) = g.iter_mut().find(|s| s.is_some_and(|a| a.actor == actor)) {
                let a = slot.take().unwrap();
                if calls_ok(base) {
                    back(base, a, "a KH1 script waits on his motion, event function 103");
                }
            } else if first {
                log!(Forms, "Drive: a KH1 script waits on partner actor {actor:#x}'s motion: the form doesn't take him while it does");
            }
        }
        return f(ctx);
    }
    let r = f(ctx);
    if actor == rd::<u64>(base + PARTY_RVA) as usize {
        if r == 4 {
            let n = SORA_WAITS.fetch_add(1, Ordering::Relaxed) + 1;
            if n >= SORA_WAIT_LOG && !SORA_WAIT_LOGGED.swap(true, Ordering::Relaxed) {
                log!(Forms, "Drive: a KH1 script has waited {n} calls on Sora's motion in a form (form {form}, state {}, request {:#x}, slot {:#x}, KH2 motion {:?}, flags {:08x})",
                    rd::<i32>(actor + 0x70), rd::<u32>(actor + 0x164), rd::<u32>(actor + 0x168), super::kh2slots::playing_motion(actor), rd::<u32>(actor));
            }
        } else {
            SORA_WAITS.store(0, Ordering::Relaxed);
        }
    }
    r
}

/// The partner check (`step`): a KH1 script waited on a partner lately, or did and still holds Sora.
fn script_waits() -> bool {
    let at = WAITED_AT.load(Ordering::Relaxed);
    if at == 0 {
        return false;
    }
    let now = super::takeover::TICKS.load(Ordering::Relaxed);
    if now.saturating_sub(at - 1) < WAIT_GRACE || !super::takeover::sora_can_act() {
        return true;
    }
    WAITED_AT.store(0, Ordering::Relaxed);
    log!(Forms, "Drive: KH1's scripts no longer wait on the partners and Sora can act: the form takes them again");
    false
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

const WORLD_RVA: St = St(0x233FE84);
const ROOM_RVA: St = St(0x233FE8C);
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

// ---------------------------------------------------------------- Sora's Keyblade in a new room (t1062)
//
// The owner: in a Drive Form the Keyblades go away in the exit's walk-out as in vanilla, and are out
// again when Sora enters the new room. KH1 (Steam, Checked (decomp, exe, log)): a walk-out exit's script
// runs event op 142 (`0x1401AA210`, command table `0x1404FA870`), which clears Sora's `+0` bit 0x4000
// (`0x1402A3280`); KH1's room teardown (`0x14028AA30` -> `0x1402921A0`) keeps each party actor's bit in
// its member record (`0x142868BA0 + member * 0x74 + 0x3A` bit 0) and the next room makes the actor with
// it (`0x140290D60`), so Sora comes in with it away (the owner's t1061 log: flags `01008043` after each
// walk-out). Base Sora gets it back from his next motion whose request row's weapon mode is 1 (the motion
// start `0x14029FC40` calls `0x1402A2E30 (actor, 1)`); a form's moves are raw-slot KH2 requests, which
// skip that, so in a form it never came back. Now, after `room_entered` in a form, the first frame of the
// form's own step in the new room (`frame`: it runs only while KH1's states have Sora, not while the
// room's entry script moves him) with KH1's scene flag off brings it out with KH1's own `0x1402A2E30
// (sora, 1)`, the arguments of KH1's motion start (its colour write `ff ff ff 80` happens only when the
// bit was clear). While the flag is on (the arrival runs a cutscene) it waits and comes out on the first
// such frame after the cutscene ends. Once a room: a cutscene later in the room that puts it away is
// left as KH1 has it. The second Keyblade follows Sora's bit (`leftblade::put_away`, t1061).
// Per frame: one load of `KEYBLADE_DUE` in the form's frame; the rest only once a room.

const WEAPON_OUT_RVA: St = St(0x2A2E30);
/// `mov [rsp+8], rbx; push rdi; sub rsp, 0x20; mov rbx, rcx; mov edi, edx` (Checked (exe)).
const WEAPON_OUT_BYTES: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0x8B, 0xFA];
const WEAPON_OUT_BIT: u32 = 0x4000;
/// A new room entered in a form: Sora's Keyblade is to come out (`keyblade_due`).
static KEYBLADE_DUE: AtomicBool = AtomicBool::new(false);

/// The form's frame (`frame`) after a room change: outside a cutscene, Sora's Keyblade out with
/// KH1's own weapon out if it is away, once.
unsafe fn keyblade_due(base: usize) {
    if !KEYBLADE_DUE.load(Ordering::Relaxed) {
        return;
    }
    // (Anti Form has none, `antiform`.)
    if !super::form::native_now() || super::form::in_anti() {
        KEYBLADE_DUE.store(false, Ordering::Relaxed);
        return;
    }
    // (A cutscene on arrival: out when it is over.)
    if rd::<u32>(base + SCENE_RVA) & 1 != 0 {
        return;
    }
    KEYBLADE_DUE.store(false, Ordering::Relaxed);
    let sora = super::takeover::sora_actor();
    if sora < 0x10000 || !super::crash::readable(sora + 3) || rd::<u32>(sora) & WEAPON_OUT_BIT != 0 {
        return;
    }
    if !super::hook::prologue_is(base + WEAPON_OUT_RVA, &WEAPON_OUT_BYTES) {
        log!(Forms, "Drive: Sora's Keyblade not brought out in the new room (KH1's weapon out not where expected)");
        return;
    }
    let out: unsafe extern "system" fn(usize, i32) = std::mem::transmute(base + WEAPON_OUT_RVA);
    out(sora, 1);
    log!(Forms, "Drive: a new room in a form: Sora's Keyblade out again (KH1's weapon out, as a motion of weapon mode 1)");
}
