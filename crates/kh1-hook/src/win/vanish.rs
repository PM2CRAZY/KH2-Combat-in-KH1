//! The partners a Drive Form takes vanish while it lasts (the user: "lets make it so that the
//! party member associated with the drive from also disappears (along with their hud icon) for
//! the duration of the drive from"). KH2's rule: Valor takes Goofy, Wisdom Donald, Master and
//! Final both, Limit no one, Anti both (`crate::partners::taken`); a guest in their place never
//! vanishes. They go as the change starts and come back beside Sora when the form ends, however
//! it ends. `docs/drive-model.md`, "Partners vanish in the form".
//!
//! How (KH1's own bits, `docs/drive-model.md`):
//! - the actor's flags word (`+0`): `0x8000` off (KH1's actor step `0x140295E50` skips all of an
//!   actor without it: his control and AI, motion and physics; the hit pass `0x14028FEE0` and the
//!   body push `0x14028FF80` leave him out; KH1's game over freezes everyone so), `0x1` and `0x2`
//!   off (his model and his shadow, `0x14029F421` / `0x14029F466`), `0x200` and `0x40` on (no
//!   enemy takes him for a foe, `0x14028FE30`, and nothing touches him: what KH1's own script
//!   op for a member leaving sets, `0x1402B9160`);
//! - his HUD entry: KH1's slot fade (`0x140266CC0`) set to hidden (mode 3, as that op does), kept
//!   hidden while he is away, and on his return faded in as KH1's revive does (mode 1) once Sora's
//!   own entry shows.
//! Nothing in the save is touched (its party list stays as it was), so a save in the form has
//! them, and a new room's actors come in whole. His HP, MP and state stay on his actor.
//!
//! Each of Sora's frames checks it (`takeover`'s physics step), and the form's end at once
//! (`form::revert`, `revert`). Every check works on the actor in the field party now for his
//! member id, never a remembered one. The decisions are `crate::partners::vanish`'s (tested
//! there): a hide of ours stays owed until we show him again or KH1 draws him itself, so KH1's
//! actor reset (s333: a Coliseum round's placing turns acting back on and leaves the model and
//! shadow off) can't leave him invisible.
//!
//! Whenever KH1's script has Sora (`takeover::script_has_sora`: an event, its script's actions
//! for him, its hold bit, or its hold `+0x444`) they are back at once: its script may command
//! them, and an actor without `0x8000` never steps, so a wait on him never ends (the user, s314:
//! the Pegasus Cup's round in Valor Form froze on its victory scene, event op 103 waiting for
//! Goofy's pose).
//! If the form outlasts the scene they go again once KH1 has let go of Sora for `QUIET_FRAMES`.

use super::addr::Rva;
use super::{log, pose};
use crate::partners::vanish::{self as rule, Act, Seen, ACTS, AWAY_BITS, MODEL};
use crate::partners::Partner;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;

/// KH1's field party (Sora, then the two others).
const PARTY_RVA: Rva = Rva(0x2D37C80);
/// Frames with no scene of KH1's (`takeover::script_has_sora`) before a form takes them again
/// (0.5 s: a scene's own short gaps don't flicker them).
const QUIET_FRAMES: u64 = 30;
/// KH1's HUD gauge slots: a u32 "bound" flag per slot at the start, and slot n's record at
/// `+ n * 0xDF0` (its actor `+0x28`, its alpha `+0xABC`, its fade speed `+0xAC0`).
const HUD_RVA: Rva = Rva(0x28123E0);
const HUD_SLOT: usize = 0xDF0;
/// KH1's HUD slot fade `(slot, mode)`: 0 fade out, 1 fade in, 2 shown, 3 hidden.
const HUD_MODE_RVA: Rva = Rva(0x266CC0);

/// KH1's actor flag stack (`0x14028FFF0` pushes the flags word to `+0x37C + depth * 4` and sets a
/// new one, `0x14028FFD0` pops it; depth `+0x388`), used by its game over.
const STACK: usize = 0x37C;
const DEPTH: usize = 0x388;
const STACK_MAX: u32 = 3;
/// KH1's actor fade (`0x14029EDB0 (actor, to, frames)`: `+0x60` the value, `+0x64` its step),
/// which its model-on command and its partner warp use; for the log.
const FADE: usize = 0x60;

/// A hide of ours on an actor.
struct Away {
    actor: usize,
    /// His flags' away bits before.
    was: u32,
    /// The flag stack's depth when he went.
    depth: u32,
    /// Where he was when he went (back for a scene: left where KH1's script put him, if it did).
    at: [f32; 3],
}

/// One partner's state.
struct Member {
    /// Our hide, while it is on his actor (`rule::held`).
    away: Option<Away>,
    /// A hide of ours may still be on him (`rule::owed`), set in room `room`.
    owed: bool,
    room: (u32, u32),
    /// His HUD entry hidden by us and not shown since.
    hud: bool,
    /// Away for the form (the log).
    gone: bool,
}

impl Member {
    const NEW: Member = Member { away: None, owed: false, room: (0, 0), hud: false, gone: false };
    fn idle(&self) -> bool {
        self.away.is_none() && !self.owed && !self.hud && !self.gone
    }
}

struct State {
    members: [Member; 2],
    /// The field party's actors last seen (the log of their changes).
    slots: [usize; 3],
}

static STATE: Mutex<State> = Mutex::new(State { members: [Member::NEW, Member::NEW], slots: [0; 3] });

/// The last game frame (`takeover::GAME_FRAME`) KH1's script had Sora, or waited on a partner
/// away (`script_waits_on`).
static SCENE_FRAME: AtomicU64 = AtomicU64::new(u64::MAX);
/// A KH1 script waited on a partner away (`script_waits_on`), for the log.
static WAITED: AtomicBool = AtomicBool::new(false);

/// KH1's event script waits for `actor`'s motion to end (event op 103, `0x1401A7430`: its script
/// actor `context + 8`, `0x14029CE30` the motion's end; the op yields until then). A partner away
/// never steps, so that wait would never end: all of them back now, as for a scene (a backstop to
/// `takeover::script_has_sora`, for a script that commands them without holding Sora).
/// They come back there and then (not on Sora's next frame: his step may not run in a scene).
pub fn script_waits_on(actor: usize) {
    {
        let Ok(st) = STATE.try_lock() else { return };
        if !st.members.iter().any(|m| m.away.as_ref().is_some_and(|a| a.actor == actor)) {
            return;
        }
    }
    SCENE_FRAME.store(super::takeover::GAME_FRAME.load(Ordering::Relaxed), Ordering::Relaxed);
    WAITED.store(true, Ordering::Relaxed);
    frame();
}

const BOTH: [Partner; 2] = [Partner::Donald, Partner::Goofy];

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn base() -> usize {
    unsafe { super::sys::GetModuleHandleA(std::ptr::null()) as usize }
}

/// A field party slot's actor, if it is one we can read.
unsafe fn slot_actor(base: usize, slot: usize) -> Option<usize> {
    let a = rd::<u64>(base + PARTY_RVA + slot * 8) as usize;
    (a >= 0x10000 && super::crash::readable(a) && super::crash::readable(a + 0x4AF)).then_some(a)
}

/// An actor's party member id (its object's `+0x4C`).
unsafe fn member_id(actor: usize) -> Option<u16> {
    let obj = pose::ptr(rd::<u32>(actor + 0x130));
    (obj > 0x10000 && super::crash::readable(obj + 0x4D)).then(|| rd::<u16>(obj + 0x4C))
}

/// The actor of partner `p` in the field party now (live, his object's member id his), if there.
unsafe fn actor_of(base: usize, p: Partner) -> Option<usize> {
    (1..3).filter_map(|slot| slot_actor(base, slot)).find(|&a| {
        let live = rd::<u32>(a + 0x374);
        live & 1 != 0 && live & 2 == 0 && member_id(a) == Some(p.member())
    })
}

/// His HUD slot (1..3, bound to his actor).
unsafe fn hud_slot(base: usize, actor: usize) -> Option<usize> {
    let hud = base + HUD_RVA;
    (1..4).find(|&s| rd::<u32>(hud + s * 4) != 0 && rd::<u64>(hud + s * HUD_SLOT + 0x28) as usize == actor)
}

unsafe fn hud_mode(base: usize, slot: usize, mode: i32) {
    let f: unsafe extern "system" fn(i32, i32) = std::mem::transmute(base + HUD_MODE_RVA);
    f(slot as i32, mode);
}

/// A HUD slot's alpha and fade speed.
unsafe fn hud_fade(base: usize, slot: usize) -> (f32, f32) {
    let r = base + HUD_RVA + slot * HUD_SLOT;
    (rd::<f32>(r + 0xABC), rd::<f32>(r + 0xAC0))
}

/// A HUD slot shown or fading (not hidden: alpha and speed 0).
unsafe fn hud_showing(base: usize, slot: usize) -> bool {
    let (alpha, speed) = hud_fade(base, slot);
    alpha > 0.0 || speed != 0.0
}

unsafe fn seen(actor: usize) -> Seen {
    Seen {
        flags: rd::<u32>(actor),
        depth: rd::<u32>(actor + DEPTH).min(STACK_MAX),
        left: rd::<u32>(actor + 0x374) & LEFT_FOR_GOOD != 0,
    }
}

/// The field party for the log: each slot's actor, member, flags, flag stack, fade and HUD entry.
unsafe fn party_line(base: usize) -> String {
    (0..3)
        .map(|slot| {
            let Some(a) = slot_actor(base, slot) else {
                return format!("slot {slot}: none ({:#x})", rd::<u64>(base + PARTY_RVA + slot * 8));
            };
            let f = rd::<u32>(a);
            let who = member_id(a).map_or("?".to_string(), crate::partners::member_name);
            let hud = match hud_slot(base, a) {
                Some(s) => {
                    let (alpha, speed) = hud_fade(base, s);
                    format!("HUD slot {s} alpha {alpha:.2} speed {speed:.3}")
                }
                None => "no HUD slot".to_string(),
            };
            format!(
                "slot {slot}: {who} actor {a:#x} flags {f:08x} ({}), +0x374 {:08x}, stack {}, fade {:.2}, {hud}",
                rule::describe(f),
                rd::<u32>(a + 0x374),
                rd::<u32>(a + DEPTH),
                rd::<f32>(a + FADE)
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Who the form now (or the change under way) takes, and its name.
fn wanted() -> Option<(&'static [Partner], &'static str)> {
    let kind = super::form::vanishing()?;
    Some((super::form::takes_partners(kind), kind.name()))
}

/// A few lines of each kind, not one a frame.
fn limited(n: &AtomicU32, max: u32, line: impl FnOnce() -> String) {
    if n.fetch_add(1, Ordering::Relaxed) < max {
        log(&line());
    }
}

/// The form's end (`form::revert`): the partners back now, the party logged before and after.
pub fn revert() {
    let base = base();
    log(&format!("Drive: the party at the revert, before: {}", unsafe { party_line(base) }));
    frame();
    log(&format!("Drive: the party at the revert, after: {}", unsafe { party_line(base) }));
}

/// Each of Sora's frames (and at a form's end): partners away while the form takes them, back
/// when it doesn't.
pub fn frame() {
    let base = base();
    let Ok(mut st) = STATE.try_lock() else { return };
    let want = wanted();
    // KH1's script has Sora now or had him a moment ago: no one away (`QUIET_FRAMES`).
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    if super::takeover::script_has_sora() {
        SCENE_FRAME.store(frame, Ordering::Relaxed);
    }
    let last = SCENE_FRAME.load(Ordering::Relaxed);
    let scene = last != u64::MAX && frame.saturating_sub(last) < QUIET_FRAMES;
    let room = super::cup::world_room();
    // The field party's actors changed while a partner is (or may be) hidden by us: logged.
    let slots: [usize; 3] = std::array::from_fn(|s| unsafe { rd::<u64>(base + PARTY_RVA + s * 8) as usize });
    if slots != st.slots {
        if st.members.iter().any(|m| !m.idle()) {
            static N: AtomicU32 = AtomicU32::new(0);
            limited(&N, 40, || {
                format!("Drive: the field party's actors changed ({:x?} -> {slots:x?}) with a partner away: {}", st.slots, unsafe { party_line(base) })
            });
        }
        st.slots = slots;
    }
    let why = if WAITED.swap(false, Ordering::Relaxed) { "a KH1 script waits on him" } else { "KH1's script takes Sora" };
    for (i, &p) in BOTH.iter().enumerate() {
        let m = &mut st.members[i];
        let takes = want.is_some_and(|(w, _)| w.contains(&p));
        unsafe {
            let actor = actor_of(base, p);
            if let Some(actor) = actor {
                step(base, m, p, actor, takes, scene, room, want.map_or("", |w| w.1), why);
            }
        }
        if !takes && m.gone {
            m.gone = false;
            log(&format!("Drive: {} is back", p.name()));
        }
    }
}

/// One partner's frame, on his actor now.
#[allow(clippy::too_many_arguments)]
unsafe fn step(base: usize, m: &mut Member, p: Partner, actor: usize, takes: bool, scene: bool, room: (u32, u32), form: &str, why: &str) {
    let s = seen(actor);
    let held = rule::held(m.away.as_ref().map(|a| a.actor), actor, s.flags);
    // Our hide isn't on his actor any more: a new actor for him, or KH1 gave it acting back (its
    // actor reset: a round's placing, its reset of every actor, its partner warp; or the pop of
    // its game over). Its model and shadow may still be off from us: `owed` keeps that.
    if let Some(a) = m.away.as_ref().filter(|_| !held) {
        static N: AtomicU32 = AtomicU32::new(0);
        let what = if a.actor != actor {
            format!("a new actor for him ({:#x} -> {actor:#x})", a.actor)
        } else {
            "KH1 gave his actor acting back (its actor reset, or its game over's pop)".to_string()
        };
        limited(&N, 30, || format!("Drive: {} while away: {what}; flags {:08x} ({})", p.name(), s.flags, rule::describe(s.flags)));
        m.away = None;
    }
    // (A room's load makes its actors whole: nothing of ours carries into another room.)
    if !held && m.room != room {
        m.owed = false;
    }
    m.room = room;
    let was_owed = m.owed;
    m.owed = rule::owed(m.owed, held, s);
    if s.left {
        m.hud = false;
    }
    match rule::decide(takes && !scene, held, m.owed, s) {
        Act::Hide => {
            let (a, hud) = hide(base, actor, s.flags);
            m.away = Some(a);
            m.owed = true;
            m.hud |= hud;
            if !m.gone {
                m.gone = true;
                log(&format!("Drive: {} leaves for {form}", p.name()));
            } else {
                static N: AtomicU32 = AtomicU32::new(0);
                limited(&N, 30, || {
                    format!(
                        "Drive: {} taken again for {form} (actor {actor:#x}, flags {:08x} ({}){})",
                        p.name(),
                        s.flags,
                        rule::describe(s.flags),
                        if was_owed && s.flags & MODEL == 0 { ": acting but undrawn after a hide of ours, KH1's reset" } else { "" }
                    )
                });
            }
        }
        Act::Keep => {
            // (Kept so if KH1 turns his model or shadow back on.)
            let hidden = rule::hidden(s.flags);
            if s.flags != hidden {
                wr(actor, hidden);
            }
            // (KH1 shows the whole HUD again at times, e.g. after an event or its game over:
            // his entry stays hidden.)
            if let Some(slot) = hud_slot(base, actor) {
                if hud_showing(base, slot) {
                    hud_mode(base, slot, 3);
                    m.hud = true;
                }
            }
        }
        Act::Restore => {
            let a = m.away.take();
            back(base, actor, a.as_ref(), scene && takes);
            m.owed = false;
            if a.is_none() {
                static N: AtomicU32 = AtomicU32::new(0);
                limited(&N, 30, || {
                    format!(
                        "Drive: {} shown again: acting but undrawn after a hide of ours (flags {:08x} ({}) -> {:08x}, actor {actor:#x}){}",
                        p.name(),
                        s.flags,
                        rule::describe(s.flags),
                        rd::<u32>(actor),
                        if super::form::in_form() { " (the form doesn't take him, or KH1's script has Sora)" } else { "" }
                    )
                });
            }
            if takes && m.gone {
                m.gone = false;
                log(&format!("Drive: {} is back ({why})", p.name()));
            }
        }
        Act::Nothing => {}
    }
    // His HUD entry back once he is (drawn, not ours to hide) and Sora's own entry shows.
    if m.hud && m.away.is_none() {
        let flags = rd::<u32>(actor);
        match hud_slot(base, actor) {
            Some(slot) if hud_showing(base, slot) => m.hud = false,
            Some(slot) if flags & MODEL != 0 && hud_showing(base, 0) => {
                hud_mode(base, slot, 1);
                m.hud = false;
            }
            _ => {}
        }
    }
}

/// KH1's member-leaving mark (`+0x374` bit 28, set by its script op `0xCF` with the same `0x240`
/// bits; nothing clears it): such a member is KH1's own business.
const LEFT_FOR_GOOD: u32 = 1 << 28;

/// Our hide on `actor`; whether his HUD entry was hidden.
unsafe fn hide(base: usize, actor: usize, flags: u32) -> (Away, bool) {
    let a = Away { actor, was: flags & AWAY_BITS, depth: rd::<u32>(actor + DEPTH).min(STACK_MAX), at: rd::<[f32; 3]>(actor + 0x10) };
    wr(actor, rule::hidden(flags));
    let hud = hud_slot(base, actor).map(|s| hud_mode(base, s, 3)).is_some();
    (a, hud)
}

/// His flags back on his actor now (`rule::restored`: acting, model and shadow on, whatever was
/// saved), beside Sora. `a`: our hide's record, if it is still on this actor. `scene`: back for
/// KH1's script with the form still on: beside Sora only if nothing moved him while away (a
/// script that placed him for its scene keeps him there; one that reset him placed him).
///
/// The bits we turn off always come back on (s331, the player: "In the Olympus Coliseum, if Sora
/// uses Drive Forms mid-match, in the next round Sora reverts to normal and allies become
/// invisible"; s333, the user: "after reverting from a drive form inside a coliseum cup the party
/// members that disappeared remain invisible"). KH1's own scripts turn them off again after this
/// if they want him hidden. His HUD entry comes back in `step` once Sora's shows.
unsafe fn back(base: usize, actor: usize, a: Option<&Away>, scene: bool) {
    let saved = a.map(|a| a.was);
    let restore = |f: u32| rule::restored(f, saved);
    let from = a.map_or(0, |a| a.depth);
    let depth = rd::<u32>(actor + DEPTH).min(STACK_MAX);
    if depth > from {
        // KH1 pushed his flags while he was away (its game over freezes every actor and pops
        // them at Continue): what it pops is his own again, and it holds `0x8000` off till then.
        for i in from..depth {
            let p = actor + STACK + i as usize * 4;
            wr(p, restore(rd::<u32>(p)));
        }
        let f = rd::<u32>(actor);
        wr(actor, (restore(f) & !ACTS) | (f & ACTS));
    } else {
        wr(actor, restore(rd::<u32>(actor)));
    }
    // Beside Sora, on the side he was on (KH2: they reappear by him).
    let sora = rd::<u64>(base + PARTY_RVA) as usize;
    let placed = scene && a.is_none_or(|a| rd::<[f32; 3]>(actor + 0x10) != a.at);
    if !placed && sora > 0x10000 && super::crash::readable(sora + 0x20) {
        let s = rd::<[f32; 3]>(sora + 0x10);
        let mut at = rd::<[f32; 3]>(actor + 0x10);
        let (dx, dz) = (at[0] - s[0], at[2] - s[2]);
        let n = dx.hypot(dz);
        let (ux, uz) = if n.is_finite() && n > 1.0 { (dx / n, dz / n) } else { (1.0, 0.0) };
        if s.iter().all(|v| v.is_finite()) {
            at = [s[0] + ux * BESIDE, s[1], s[2] + uz * BESIDE];
            wr(actor + 0x10, at);
        }
    }
}

/// How far beside Sora a partner comes back.
const BESIDE: f32 = 60.0;
