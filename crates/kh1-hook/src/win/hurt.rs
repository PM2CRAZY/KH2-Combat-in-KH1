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

use super::addr::Rva;
use super::{combat, hook, log, pose, takeover};
use character_core::reaction::Hit;
use character_core::StateName;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

const HURT_RVA: Rva = Rva(0x299A40);
const HURT_PROLOGUE: [u8; 11] = [0x48, 0x8B, 0xC4, 0x55, 0x57, 0x41, 0x54, 0x41, 0x56, 0x41, 0x57];
const KIND_RVA: Rva = Rva(0x2BD0E0);
const VOICE_RVA: Rva = Rva(0x28E060);
const RUMBLE_RVA: Rva = Rva(0x28D740);
const COUNT_RVA: Rva = Rva(0x28A4D0);
const PARTY_RVA: Rva = Rva(0x2D37C80);
const COMMAND_RVA: Rva = Rva(0x2D54440);
/// Hit record flag: no effect.
const NO_EFFECT: u8 = 0x20;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG: AtomicUsize = AtomicUsize::new(0);
static LOGGED: AtomicU32 = AtomicU32::new(0);

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
    Taken(u32),
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
    let kind_of: unsafe extern "system" fn(usize, usize) -> u32 = std::mem::transmute(base + KIND_RVA);
    let me = rd::<[f32; 3]>(actor + 0x10);
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
            } else if super::reflect::barrier_up() {
                // KH2's Reflect: the barrier round him stops any attack (from every side) and
                // counts it.
                Fate::Reflected
            } else if reacting == StateName::Magic && super::reflect::cast_motion(out.motion) {
                // The rest of Reflect's cast (before the barrier stands and after it falls): no
                // harm all the way through (the user), not counted for the counter.
                Fate::Evaded
            } else if (out.guarding || !out.guard_boxes.is_empty()) && (toward.dot(fwd) > 0.0 || ch.abilities.guard_all_round) {
                ch.clash(1, 1, toward.to_array());
                Fate::Guarded
            } else if matches!(kind, 6 | 7) {
                // An attack clash (KH1's recoil): KH2's recoil (not its guard repel).
                ch.clash(0, 0, toward.to_array());
                Fate::Guarded
            } else if matches!(kind, 3..=5) {
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
    // KH2's reaction for the strongest taken hit (after KH1's damage, unless he went down).
    let mut react: Option<(u32, glam::Vec2)> = None;
    for (&r, fate) in ready.iter().zip(&fates) {
        let Some((fate, toward)) = *fate else { continue };
        note(format!("Sora hit by {:#x}: damage {}, record kind {}, {fate:?}", pose::ptr(rd::<u32>(r + 4)), rd::<i32>(r + 8), rd::<u32>(r + 0xC) & 0x1F));
        match fate {
            Fate::Evaded => wr(r, rd::<u8>(r) | NO_EFFECT),
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
    // Already reeling: the damage counts, but a flinch doesn't restart KH2's reaction (nor
    // anything restart a blow), so a crowd can't lock him in place.
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
