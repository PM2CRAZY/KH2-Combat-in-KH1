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
//! even with leaf bracer its an instant kill"): KH1 lets an attack hit an actor once (the queue
//! `0x1402BD480` skips a target whose `+0x3E0` has the attack's bit, and sets it), but a lasting
//! attack re-arms on an interval of its own (`0x1402B11F0` clears its bit from every actor each
//! `+0x50`), so a pillar he stands in hits him again and again. KH1's own reactions knock him out
//! of it; KH2's, played instead, kept him there, and every re-hit's damage was taken (Second
//! Chance only spares the first, from full HP). So, as KH2 does:
//! - knocked down (KH2's blow, from launch to getting up): no hit lands (`Fate::Down`);
//! - flinching from a hit: the same attacker's hits within `REHIT_GRACE` frames don't land
//!   (`Fate::Rehit`, the re-armed attack, not a new swing).

use super::addr::Rva;
use super::{combat, hook, log, pose, takeover};
use character_core::reaction::Hit;
use character_core::StateName;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

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
/// The last hit taken: its attacker's handle and the 60 Hz frame (`takeover::TICKS`: the grace is
/// the same time at 30 fps).
static LAST_FROM: AtomicU32 = AtomicU32::new(0);
static LAST_AT: AtomicU64 = AtomicU64::new(0);
/// While flinching, the same attacker's hits this soon after the last one don't land (60 Hz
/// frames, whatever the frame rate).
const REHIT_GRACE: u64 = 20;

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
    /// The same attacker again while he still flinches from its last hit.
    Rehit,
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
    let frame = takeover::TICKS.load(Ordering::Relaxed);
    let (last_from, last_at) = (LAST_FROM.load(Ordering::Relaxed), LAST_AT.load(Ordering::Relaxed));
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
            let from_h = rd::<u32>(r + 4);
            let fate = if out.invulnerable {
                Fate::Evaded
            } else if reacting == StateName::Blown {
                Fate::Down
            } else if reacting == StateName::Hurt && from_h != 0 && from_h == last_from && frame.saturating_sub(last_at) < REHIT_GRACE {
                Fate::Rehit
            } else if super::reflect::barrier_up() {
                // KH2's Reflect: the barrier round him stops any attack (from every side) and
                // counts it.
                Fate::Reflected
            } else if reacting == StateName::Magic && super::reflect::cast_motion(out.motion) {
                // The rest of Reflect's cast (before the barrier stands and after it falls): no
                // harm all the way through (the user), not counted for the counter.
                Fate::Evaded
            } else if (out.guarding || !out.guard_boxes.is_empty())
                && (ch.abilities.guard_all_round
                    || if out.guard_boxes.is_empty() { toward.dot(fwd) > 0.0 } else { out.guard_boxes.iter().any(|g| guard_faces(g.attack, r, out.facing, toward)) })
            {
                // KH2's guard reaction by its windows' ATKP "refact self" (1 the guard repel, 2
                // nothing: the Aerial Dodge's 406 and Aerial Recovery's 124 go on; the guard
                // press's 161 repels); which side it holds is its ATKP's reflect range and angle
                // (`guard_faces`: 161 120 degrees each side, 162 180, the body guards 406 / 124 0,
                // all round).
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
    let Some((fates, reacting)) = fates else {
        // KH1's own states have base Sora (`shared`): KH1's reaction, as unmodded. A knock-back
        // or launch of KH1's is where KH2's Aerial Recovery and Retaliating Slash can borrow him
        // (`combomods::launched`).
        // KH2's guard on his body in the KH2 move KH1 plays now (our slot, `kh2slots::body_guard`:
        // Flash Step's frames 0..19, ATKP 406), as KH2's guard takes a hit there (above: its
        // `refact_self` 2, no reaction, the move goes on; on his body, all round): each attack
        // with no effect and KH1's clash spark, the damage, the reaction and the rest of KH1's
        // pass for it not run.
        // KH2's invulnerability in the KH2 move KH1 plays now (our slot, range id 27,
        // `kh2slots::invulnerable`: Explosion's frames 32..64), as KH2's own hit pass finds no
        // hurt shape of his there (`Fate::Evaded` above): every attack with no effect, no spark.
        // Before the guard, as KH2's test order has it. Also KH2's Quick Run on its KH1 state
        // (`quickrun::invulnerable`: its `fmab` frames from its start, 0 / 8 / 12 / 16 by level).
        // And KH2's dodge roll on KH1's roll (`combomods::roll_invulnerable`: the level's
        // `dodge_roll_star_time` frames, 10 / 16 / 24 / 32).
        if super::shared::active() && (super::kh2slots::invulnerable(actor) || super::quickrun::invulnerable(actor) || super::combomods::roll_invulnerable(actor)) {
            for &r in &ready {
                if rd::<u8>(r) & NO_EFFECT != 0 || (rd::<u32>(r + 0xC) >> 6) & 0xF >= 9 {
                    continue;
                }
                wr(r, rd::<u8>(r) | NO_EFFECT);
                note(format!("Sora hit by {:#x} in KH1's states: KH2's invulnerability (the KH2 move KH1 plays), no effect (frame {frame})", pose::ptr(rd::<u32>(r + 4))));
            }
        }
        // KH2's guard on the Keyblade in the KH2 move KH1 plays now (our guard slots,
        // `kh2slots::keyblade_guard`: the guard press's 161, its repel's 162), as KH2's own hit
        // pass takes it (KH2's controller's rule above: a hit from in front, or from anywhere with
        // Guard all round): no effect, KH1's clash spark, and the guard's reaction by its "refact
        // self" (1 the repel, `combomods::guard_repel`; 2 nothing).
        if super::shared::active() {
            if let Some((atk, refact)) = super::kh2slots::keyblade_guard(actor) {
                let f = rd::<[f32; 4]>(actor + 0x30);
                let fwd = glam::Vec2::new(f[0], f[2]).normalize_or(glam::Vec2::Y);
                let yaw = if f[3].is_finite() { f[3] } else { fwd.x.atan2(fwd.y) };
                let all_round = takeover::with_abilities(|a| a.guard_all_round) == Some(true);
                let mut repel: Option<(glam::Vec2, Option<i32>)> = None;
                for &r in &ready {
                    if rd::<u8>(r) & NO_EFFECT != 0 || (rd::<u32>(r + 0xC) >> 6) & 0xF >= 9 {
                        continue;
                    }
                    let from = pose::ptr(rd::<u32>(r + 4));
                    let toward = if from > 0x10000 && from != actor {
                        let p = rd::<[f32; 3]>(from + 0x10);
                        glam::Vec2::new(p[0] - me[0], p[2] - me[2]).normalize_or(fwd)
                    } else {
                        fwd
                    };
                    if !(all_round || guard_faces(atk, r, yaw, toward)) {
                        continue;
                    }
                    wr(r, rd::<u8>(r) | NO_EFFECT);
                    combat::clang([me[0] + toward.x * 40.0, me[1] - 60.0, me[2] + toward.y * 40.0]);
                    // (The attack's priority, for KH1's block at Guard Repel KH1: its data's
                    // `+0x3C`, the record's `+0x2C` handle.)
                    let data = pose::ptr(rd::<u32>(r + 0x2C));
                    repel.get_or_insert((toward, (data > 0x10000).then(|| rd::<i32>(data + 0x3C))));
                    note(format!("Sora hit by {from:#x} in KH1's states: KH2's guard on the Keyblade (ATKP {atk}, refact self {refact}), no effect (frame {frame})"));
                }
                if let Some((t, priority)) = repel {
                    super::combomods::guard_repel(actor, t.to_array(), refact, priority);
                }
            }
        }
        if super::shared::active() {
            if let Some(atk) = super::kh2slots::body_guard(actor) {
                for &r in &ready {
                    if rd::<u8>(r) & NO_EFFECT != 0 || (rd::<u32>(r + 0xC) >> 6) & 0xF >= 9 {
                        continue;
                    }
                    let from = pose::ptr(rd::<u32>(r + 4));
                    let toward = if from > 0x10000 && from != actor {
                        let p = rd::<[f32; 3]>(from + 0x10);
                        glam::Vec2::new(p[0] - me[0], p[2] - me[2]).normalize_or(glam::Vec2::Y)
                    } else {
                        glam::Vec2::Y
                    };
                    wr(r, rd::<u8>(r) | NO_EFFECT);
                    combat::clang([me[0] + toward.x * 40.0, me[1] - 60.0, me[2] + toward.y * 40.0]);
                    note(format!("Sora hit by {from:#x} in KH1's states: KH2's guard on his body (ATKP {atk}, the KH2 move KH1 plays), no effect (frame {frame})"));
                }
            }
        }
        // (KH1's middle hits as flinches, the Medium Hits row: `hitreact::medium_hits`; before the
        // knock-back note below, which then doesn't see them.)
        super::hitreact::medium_hits(actor, &ready, kind_of);
        if super::shared::active() && ready.iter().any(|&r| rd::<u8>(r) & NO_EFFECT == 0 && (rd::<u32>(r + 0xC) >> 6) & 0xF < 9 && matches!(kind_of(actor, r), 4 | 5)) {
            super::combomods::kh1_blown();
        }
        // KH2's push and air flinch on the reaction KH1's pass starts (`hitreact`, Hit Reactions
        // KH2).
        let was = super::hitreact::before_pass(actor);
        // (And KH2's recoil on the attack-clash reaction it starts, `recoil`, Recoil KH2.)
        let clash = super::recoil::before_pass(actor);
        let r = f(actor);
        super::hitreact::after_pass(actor, was);
        super::recoil::after_pass(actor, clash);
        return r;
    };
    // KH2's reaction for the strongest taken hit (after KH1's damage, unless he went down).
    let mut react: Option<(u32, glam::Vec2)> = None;
    for (&r, fate) in ready.iter().zip(&fates) {
        let Some((fate, toward)) = *fate else { continue };
        note(format!(
            "Sora hit by {:#x}: damage {}, record kind {}, {fate:?} (frame {frame}, {reacting:?})",
            pose::ptr(rd::<u32>(r + 4)),
            rd::<i32>(r + 8),
            rd::<u32>(r + 0xC) & 0x1F
        ));
        match fate {
            Fate::Evaded | Fate::Down | Fate::Rehit => wr(r, rd::<u8>(r) | NO_EFFECT),
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
                LAST_FROM.store(rd::<u32>(r + 4), Ordering::Relaxed);
                LAST_AT.store(frame, Ordering::Relaxed);
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
    // Already flinching (a blow lets no hit land, above): the damage counts, but a flinch
    // doesn't restart KH2's reaction (nor anything restart a blow), so a crowd can't lock him in
    // place.
    if matches!(reacting, StateName::Blown) || (matches!(reacting, StateName::Hurt) && kind < 5) {
        note(format!("KH2 reaction skipped (already {reacting:?}), HP {}", if stats > 0x10000 { rd::<i32>(stats + 0x3C) } else { 0 }));
        return r;
    }
    // KH1's knock-backs (4) and launches (5) knock him away as KH2's blows do (so Aerial
    // Recovery has something to recover from; the user: it never came out, every 4 was a flinch).
    // (The Medium Hits row at Flinch: KH1's kind 4 a flinch, base Sora only; `hitreact`.)
    let blow = kind >= 5 || (kind == 4 && (super::form::in_form() || !super::hitreact::medium_flinch()));
    // KH1's voice and rumble: 3 a flinch's, 4 / 5 a knock-back's.
    let big = blow;
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

/// KH2's guard takes the hit in hit record `r` from this side (Steam KH2 `0x1403D1F40`, Checked
/// (decomp): the guard's ATKP reflect range `+0x22`, 0 any side; else the other attack's direction
/// against the guard owner's facing turned by the ATKP's reflect angle `+0x23` x 2 degrees must be
/// below -cos(range): `character_core::reaction::clash_angle_ok`). Sora's guard ATKPs (`00battle.bin`,
/// Checked (files)): the guard press's 161 range 120 (120 degrees either side of his facing, not 90),
/// its repel's 162 180 (every side), the body guards 406 / 124 / 455 0 (every side); all angle 0. The
/// attack's direction is KH1's push direction in the record (`+0x10`, KH1's own front / back test
/// reads it, `docs/kh1-hit-reactions-native.md`), else the line from the attacker (`toward`, from
/// Sora to it). `yaw`: Sora's facing (KH1's `+0x3C`). An ATKP not found: the front half, as before.
unsafe fn guard_faces(atkp: u16, r: usize, yaw: f32, toward: glam::Vec2) -> bool {
    let push = rd::<[f32; 3]>(r + 0x10);
    let push = glam::Vec2::new(push[0], push[2]);
    let dir = if push.is_finite() && push.length() > 0.1 { push.normalize() } else { -toward };
    match pose::kh2().and_then(|k| k.battle.attack_for_level(atkp, 0, 0)) {
        Some(a) => character_core::reaction::clash_angle_ok(a.reflect_range, a.reflect_angle, yaw, dir.to_array()),
        None => toward.dot(glam::Vec2::new(yaw.sin(), yaw.cos())) > 0.0,
    }
}
