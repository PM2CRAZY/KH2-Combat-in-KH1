//! Phase 5: Sora takes hits the KH2 way while KH2 has him.
//!
//! From Ghidra (the user's Epic exe):
//! - An enemy attack touching an actor queues a hit record on it (`0x1402BD480`): a pool entry
//!   linked from the actor's `+0x3EC` (handle), next at record `+0x30`; `+0` flags (`0x20`: no
//!   effect, as for an attack a guard stopped, `0x1402B2A30`), `+4` the attacker's handle, `+8`
//!   the damage, `+0xC` low 5 bits the reaction kind, bits 12..15 a delay in frames.
//! - `0x140299A40` `(actor, out vector)` takes them each frame: pops ready records (`0x1402BCFB0`; a record
//!   still delayed counts down and stops the pass), applies the damage (`0x1402BCD30`; Second
//!   Chance, MP from damage, the HP bar and the dying flag 4 on the stats `+0x6C` live here
//!   too), maps the kind (`0x1402BD0E0`: kind 1 becomes 3 / 4 / 5 by the damage's share of max
//!   HP; 3 a flinch, 4 / 5 knocked back / down, 6 / 7 an attack clash) and runs KH1's reaction
//!   for it by movement state, then the hurt voice (`0x14028E060` `(actor, 0 | 1, 0)`), the pad
//!   rumble (`0x14028D740`) and the journal's hit count (`0x14028A4D0(0xD)`).
//!
//! While KH2 has Sora, before KH1's pass runs, each ready record is decided with KH2's last
//! frame:
//! - a dodge roll's (or any) invulnerable frames: evaded, the record marked no-effect;
//! - KH2's guard (its guard state or live guard windows) with the attacker in front (anywhere
//!   with Guard Plus): KH2's guard repel, KH1's clash spark, no effect;
//! - else the hit is taken: KH1 still applies its damage (and everything around it), but its
//!   reaction kind is cleared so KH1 plays no reaction of its own; KH2 plays its reaction
//!   instead (a flinch for 3 / 4, a blow for 5), and the voice, rumble and hit count KH1 would
//!   have played for it are played here.
//! Sora's HP gone: KH1 had its dying pass (kind kept), and `takeover` lets KH1 have him.
//!
//! Repeated hits (the user: Sephiroth's flame pillar "hits sora way too fast to react to ...
//! even with leaf bracer its an instant kill"; a player: a Large Body's ground slam "nuked"
//! Sora, "kh2 sora doesn't have damage immunity after getting hit"): KH1 lets an attack hit an
//! actor once (the queue `0x1402BD480` skips a target whose `+0x3E0` has the attack's bit, and
//! sets it), but a lasting attack re-arms on an interval of its own (`0x1402B11F0` clears its bit
//! from every actor each `+0x50`), and an attack of several objects (a slam and its shockwave)
//! has a bit for each. What keeps KH1's Sora alive is its no-damage time: each of his reactions
//! sets his flag `0x80` with a 30-frame timer (`0x140295DA0`), counted down once the reaction's
//! action is over (`0x1402962EB`); while it is set KH1's damage step takes no HP
//! (`0x1402BCD30`) and its pass plays no reaction but a clash (`0x140299D2C`). KH2's reactions,
//! played in KH1's place, never set it, so every part and every re-arm landed in full. KH2 has
//! the same time (its player's group timer, 30 frames, held through reaction motions), so, as
//! both do (`crate::mercy`):
//! - one record a pass is taken; the pass's other records deal nothing;
//! - after it, while KH2's reaction plays (flinch, blow, landing, getting up) and 30 frames
//!   after, no hit deals damage or a reaction (`Fate::Mercy`), whoever's (as in both games);
//!   guards, Reflect and attack clashes go on;
//! - KH1's own flag (`0x80`, or a script's `0x1000000`), when it is set, is honoured
//!   (`Fate::Immune`): KH1 takes no HP then, and KH2 plays no reaction for it;
//! - knocked down (KH2's blow, launched): no hit lands (`Fate::Down`).

use super::addr::Rva;
use super::{combat, hook, log, pose, takeover};
use crate::mercy::{Mercy, Repeat, Swing};
use character_core::reaction::Hit;
use character_core::StateName;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

const HURT_RVA: Rva = Rva(0x299A40);
const HURT_PROLOGUE: [u8; 11] = [0x48, 0x8B, 0xC4, 0x55, 0x57, 0x41, 0x54, 0x41, 0x56, 0x41, 0x57];
const KIND_RVA: Rva = Rva(0x2BD0E0);
const VOICE_RVA: Rva = Rva(0x28E060);
const RUMBLE_RVA: Rva = Rva(0x28D740);
const COUNT_RVA: Rva = Rva(0x28A4D0);
const PARTY_RVA: Rva = Rva(0x2D37C80);
const COMMAND_RVA: Rva = Rva(0x2D54440);
/// KH1's command flags (`0x142D54438`, `limits`' and `reflect`'s): with `2` set (no limits
/// either) its damage step takes no HP (`0x1402BCD30`) and its pass plays no reaction
/// (`0x140299D34`).
const NO_DAMAGE_RVA: Rva = Rva(0x2D54438);
/// `btltbl.bin`'s attack data (`combat`'s): a record's data entry is logged as an offset in it.
const ATTACK_DATA_RVA: Rva = Rva(0x2D23740);
/// Hit record flag: no effect.
const NO_EFFECT: u8 = 0x20;
/// Sora's actor flags (`+0`) in which KH1 takes no HP from a hit: `0x80` its no-damage time after
/// a reaction (and during its dodge roll, `0x1402C7CB0`), `0x1000000` a script's.
const KH1_NO_DAMAGE: u32 = 0x0100_0080;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG: AtomicUsize = AtomicUsize::new(0);
static LOGGED: AtomicU32 = AtomicU32::new(0);
/// Hits that came during the no-damage time, logged (at most `REPEATS_LOGGED`).
static REPEATS: AtomicU32 = AtomicU32::new(0);
const REPEATS_LOGGED: u32 = 80;
/// Sora's no-damage time after a hit (`crate::mercy`), and how many hits it has stopped.
static MERCY: Mutex<(Mercy, u32)> = Mutex::new((Mercy::new(), 0));

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn note(s: String) {
    if LOGGED.fetch_add(1, Ordering::Relaxed) < 300 {
        log(&s);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Fate {
    Evaded,
    /// Stopped by a Reflect barrier (`reflect`), counted for its counter.
    Reflected,
    Guarded,
    /// Knocked down (KH2's blow): no hit lands.
    Down,
    /// In the no-damage time after a hit (its reaction and 30 frames after), or another record
    /// of the pass a hit was taken in: nothing.
    Mercy(Repeat),
    /// KH1's own no-damage flag is set (`KH1_NO_DAMAGE`): KH1 takes no HP, no KH2 reaction.
    Immune,
    Taken(u32),
}

fn swing(r: usize) -> Swing {
    unsafe { Swing { from: rd::<u32>(r + 4), data: rd::<u32>(r + 0x2C), bit: rd::<u32>(r + 0x28) } }
}

unsafe extern "system" fn hurt(actor: usize, out: usize) -> usize {
    // (KH1's pass returns nothing; whatever it leaves in rax is passed back all the same.) Its
    // second argument (rdx) is where its reactions write a vector (the knock-back one,
    // `0x14029AD30`, writes the actor's position there): it must reach KH1 intact (a hit while
    // KH1 had Sora, casting, crashed on it).
    let f0: unsafe extern "system" fn(usize, usize) -> usize = std::mem::transmute(ORIG.load(Ordering::Acquire));
    let f = |a: usize| f0(a, out);
    let base = BASE.load(Ordering::Relaxed);
    if actor < 0x10000 || actor != rd::<u64>(base + PARTY_RVA) as usize || !super::settings::get().hurt {
        return f(actor);
    }
    // The no-damage time holds while KH2's reaction plays (its motion on its last frame).
    let frame = takeover::GAME_FRAME.load(Ordering::Relaxed);
    if let Ok(mut m) = MERCY.lock() {
        m.0.see(frame, takeover::KH2_MOTION.load(Ordering::Relaxed));
    }
    // The ready records (those KH1's pass pops this frame).
    let mut ready = Vec::new();
    let mut rec = pose::ptr(rd::<u32>(actor + 0x3EC));
    while rec > 0x10000 && ready.len() < 32 && (rd::<u32>(rec + 0xC) >> 12) & 0xF == 0 {
        ready.push(rec);
        rec = pose::ptr(rd::<u32>(rec + 0x30));
    }
    if ready.is_empty() {
        return f(actor);
    }
    // Anti Form takes 1.5 times the damage (KH2's exe, `character_core::drive::DriveParams::
    // anti_damage`, `docs/drive-model.md`; the user: "I want 1:1 combat parity"): each attack's
    // damage (not drains or heals, damage types 9..12) raised, rounded up, before KH1 applies it.
    // A record evaded or guarded below is marked no-effect and deals none anyway.
    if super::form::in_anti() {
        let k = character_core::drive::DriveParams::default().anti_damage;
        for &r in &ready {
            let d = rd::<i32>(r + 8);
            if rd::<u8>(r) & NO_EFFECT == 0 && (rd::<u32>(r + 0xC) >> 6) & 0xF < 9 && d > 0 {
                let up = (d as f32 * k).ceil() as i32;
                wr(r + 8, up);
                note(format!("Anti Form: damage {d} -> {up}"));
            }
        }
    }
    let kind_of: unsafe extern "system" fn(usize, usize) -> u32 = std::mem::transmute(base + KIND_RVA);
    let me = rd::<[f32; 3]>(actor + 0x10);
    let mut mercy = MERCY.lock().map(|m| m.0).unwrap_or_default();
    // KH1's own no-damage time (or a script's, or its global switch): KH1 takes no HP and plays
    // no reaction, so KH2 plays none either.
    let kh1_immune = rd::<u32>(actor) & KH1_NO_DAMAGE != 0 || rd::<u8>(base + NO_DAMAGE_RVA) & 2 != 0;
    let fates = takeover::with_kh2(|ch, out| {
        let reacting = out.state;
        let fwd = glam::Vec2::new(out.facing.sin(), out.facing.cos());
        let mut fates = Vec::new();
        for &r in &ready {
            // Effects other than attacks (damage types 9..12 at `+0xC` bits 6..9: drains, heals,
            // KH1's specials) stay KH1's, as do records already without effect.
            if rd::<u8>(r) & NO_EFFECT != 0 || (rd::<u32>(r + 0xC) >> 6) & 0xF >= 9 {
                fates.push(None);
                continue;
            }
            let from = pose::ptr(rd::<u32>(r + 4));
            let toward = if from > 0x10000 && from != actor {
                let p = rd::<[f32; 3]>(from + 0x10);
                glam::Vec2::new(p[0] - me[0], p[2] - me[2]).normalize_or(fwd)
            } else {
                fwd
            };
            let kind = kind_of(actor, r);
            let fate = if out.invulnerable {
                Fate::Evaded
            } else if reacting == StateName::Blown {
                Fate::Down
            } else if super::reflect::barrier_up() {
                // KH2's Reflect: the barrier round him stops any attack (from every side) and
                // counts it.
                Fate::Reflected
            } else if reacting == StateName::Magic && super::reflect::cast_motion(out.motion) {
                // The rest of Reflect's cast (before the barrier stands and after it falls): no
                // harm all the way through (the user), not counted for the counter.
                Fate::Evaded
            } else if !super::form::in_anti() && (out.guarding || !out.guard_boxes.is_empty()) && (toward.dot(fwd) > 0.0 || ch.abilities.guard_all_round || out.guard_boxes.iter().any(|g| g.weapon == 0)) {
                // (Never in Anti Form, which guards nothing: the user.)
                // KH2's guard reaction by its windows' ATKP "refact self" (1 the guard repel, 2
                // nothing: the Aerial Dodge's 406 and Aerial Recovery's 124 go on; the guard
                // press's 161 repels); a guard on his body guards all round (`combat::guard_step`).
                let refact = super::pose::kh2()
                    .and_then(|k| out.guard_boxes.iter().filter_map(|g| k.battle.attack_for_level(g.attack, 0, 0)).map(|a| a.refact_self).min())
                    .unwrap_or(1);
                if refact == 2 {
                    ch.clash(2, 0, toward.to_array());
                } else {
                    ch.clash(1, 1, toward.to_array());
                }
                Fate::Guarded
            } else if matches!(kind, 6 | 7) {
                // An attack clash (KH1's recoil): KH2's recoil (not its guard repel).
                ch.clash(0, 0, toward.to_array());
                Fate::Guarded
            } else if matches!(kind, 3..=5) && kh1_immune {
                Fate::Immune
            } else if matches!(kind, 3..=5) && mercy.active(frame) {
                // (Also the pass's records after the one taken: `landed` below.)
                Fate::Mercy(mercy.repeat(swing(r)))
            } else if matches!(kind, 3..=5) {
                mercy.landed(frame, swing(r));
                Fate::Taken(kind)
            } else {
                // Kinds KH2 has no reaction for (none, or KH1's special ones): KH1's, as is.
                fates.push(None);
                continue;
            };
            fates.push(Some((fate, toward)));
        }
        (fates, reacting)
    });
    let Some((fates, reacting)) = fates else { return f(actor) };
    // The attack data entry as an offset into `btltbl.bin` (as `combat`'s rows give it), else
    // its address.
    let data_of = |r: usize| {
        let d = pose::ptr(rd::<u32>(r + 0x2C));
        let table = base + ATTACK_DATA_RVA;
        if d >= table && d < table + 0x12000 { d - table } else { d }
    };
    if let Ok(mut m) = MERCY.lock() {
        // (How many hits each no-damage time stopped, told when the next hit lands.)
        if fates.iter().any(|f| matches!(f, Some((Fate::Taken(_), _)))) {
            if m.1 > 0 {
                note(format!("(Sora's last no-damage time stopped {} hits)", m.1));
            }
            m.1 = 0;
        }
        m.0 = mercy;
        m.1 += fates.iter().filter(|f| matches!(f, Some((Fate::Mercy(_), _)))).count() as u32;
    }
    // KH2's reaction for the hit taken (after KH1's damage, unless he went down).
    let mut react: Option<(u32, glam::Vec2)> = None;
    for (&r, fate) in ready.iter().zip(&fates) {
        let Some((fate, toward)) = *fate else { continue };
        if let Fate::Mercy(repeat) = fate {
            // (Logged on their own, a shockwave or a pillar meets him every few frames.)
            if REPEATS.fetch_add(1, Ordering::Relaxed) < REPEATS_LOGGED {
                log(&format!(
                    "Sora hit again in his no-damage time ({} frames after the reaction, KH2 motion {}): {repeat:?}, by {:#x}, data {:#x}, bit {:#x}, damage {} not taken (frame {frame})",
                    mercy.since(frame),
                    takeover::KH2_MOTION.load(Ordering::Relaxed),
                    pose::ptr(rd::<u32>(r + 4)),
                    data_of(r),
                    rd::<u32>(r + 0x28),
                    rd::<i32>(r + 8),
                ));
            }
        } else {
            note(format!(
                "Sora hit by {:#x}: damage {}, record kind {}, data {:#x}, bit {:#x}, {fate:?} (frame {frame}, {reacting:?})",
                pose::ptr(rd::<u32>(r + 4)),
                rd::<i32>(r + 8),
                rd::<u32>(r + 0xC) & 0x1F,
                data_of(r),
                rd::<u32>(r + 0x28),
            ));
        }
        match fate {
            // (KH1's own: it takes no HP and plays no reaction, the record as it is.)
            Fate::Immune => {}
            Fate::Evaded | Fate::Down | Fate::Mercy(_) => wr(r, rd::<u8>(r) | NO_EFFECT),
            Fate::Reflected => {
                wr(r, rd::<u8>(r) | NO_EFFECT);
                super::reflect::blocked(pose::ptr(rd::<u32>(r + 4)));
            }
            Fate::Guarded => {
                wr(r, rd::<u8>(r) | NO_EFFECT);
                let at = [me[0] + toward.x * 40.0, me[1] - 60.0, me[2] + toward.y * 40.0];
                combat::clang(at);
            }
            Fate::Taken(kind) => {
                wr(r + 0xC, rd::<u32>(r + 0xC) & !0x1F);
                if kind >= 3 && react.is_none_or(|(k, _)| kind > k) {
                    react = Some((kind, toward));
                }
            }
        }
    }
    let r = f(actor);
    let stats = pose::ptr(rd::<u32>(actor + 0x6C));
    let down = stats > 0x10000 && (rd::<u8>(stats) & 4 != 0 || rd::<i32>(stats + 0x3C) < 1);
    let Some((kind, toward)) = react else { return r };
    if down {
        note(format!("Sora is down (HP {})", if stats > 0x10000 { rd::<i32>(stats + 0x3C) } else { 0 }));
        // (Out of a Drive Form at once, as KH2's KO: `form`.)
        super::form::knocked_out();
        return r;
    }
    // Already in a KH2 reaction no hit of ours started (a guard's repel; a flinch's and a blow's
    // own let no hit land, above): the damage counts, but a flinch doesn't restart KH2's
    // reaction (nor anything restart a blow), so a crowd can't lock him in place.
    if matches!(reacting, StateName::Blown) || (matches!(reacting, StateName::Hurt) && kind < 5) {
        note(format!("KH2 reaction skipped (already {reacting:?}), HP {}", if stats > 0x10000 { rd::<i32>(stats + 0x3C) } else { 0 }));
        return r;
    }
    // KH1's knock-backs (4) and launches (5) knock him away as KH2's blows do (so Aerial
    // Recovery has something to recover from; the user: it never came out, every 4 was a flinch).
    let blow = kind >= 4;
    // KH1's voice and rumble: 3 a flinch's, 4 / 5 a knock-back's.
    let big = kind >= 4;
    let dir = [-toward.x, -toward.y];
    let (reaction, knockback) = if blow { (4, (0, 0)) } else { (3, (i16::MAX, 0)) };
    takeover::with_kh2(|ch, _| ch.take_hit(&Hit { reaction, knockback, dir, part: 0 }));
    // What KH1's pass plays with its own reactions (the player's party slot only).
    let voice: unsafe extern "system" fn(usize, i32, usize) = std::mem::transmute(base + VOICE_RVA);
    let rumble: unsafe extern "system" fn(i32) = std::mem::transmute(base + RUMBLE_RVA);
    let count: unsafe extern "system" fn(i32) = std::mem::transmute(base + COUNT_RVA);
    rumble(if big { 0 } else { 1 });
    count(0xD);
    wr(base + COMMAND_RVA, 0u64);
    wr(base + COMMAND_RVA + 8, 0u32);
    voice(actor, if big { 1 } else { 0 }, 0);
    note(format!("KH2 reaction: {} toward {:.2?}, HP {}", if blow { "blow" } else { "flinch" }, dir, if stats > 0x10000 { rd::<i32>(stats + 0x3C) } else { 0 }));
    r
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + HURT_RVA, &HURT_PROLOGUE, hurt as *const () as usize, &ORIG, "hit processing");
}
