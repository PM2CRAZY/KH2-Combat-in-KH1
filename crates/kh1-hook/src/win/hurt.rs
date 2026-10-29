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
const PARTY_RVA: Rva = Rva(0x2D37C80);
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

fn note(s: impl FnOnce() -> String) {
    if !super::log_on(super::LogCat::Combat) {
        return;
    }
    if super::log_on(super::LogCat::Combat) && LOGGED.fetch_add(1, Ordering::Relaxed) < 300 {
        log!(Combat, "{}", s());
    }
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
    // (t877: KH1's own states always have base Sora; the old KH2 controller's own reactions for him,
    // its evade / guard / blow fates, are deleted.)
    // KH1's own states have base Sora (`shared`): KH1's reaction, as unmodded. A knock-back
    // or launch of KH1's is where KH2's Aerial Recovery and Retaliating Slash can borrow him
    // (`combomods::launched`).
    // KH2's guard on his body in the KH2 move KH1 plays now (our slot, `kh2slots::body_guard`:
    // Flash Step's frames 0..19, ATKP 406), as KH2's guard takes a hit there (above: its
    // `refact_self` 2, no reaction, the move goes on; on his body, all round): each attack
    // with no effect and KH1's clash spark, the damage, the reaction and the rest of KH1's
    // pass for it not run.
    // KH2's invulnerability in the KH1 states (the dodge roll t858, Quick Run t859, the KH2
    // moves' range 27 t860: Explosion, Aerial Recovery, Counterguard, Finishing Leap) is KH1's
    // own no-damage timer now (`combomods::kh1_no_damage`), which KH1's pass below honours
    // itself (no damage, no reaction but a clash, the hit landing as KH1's kind 4, as KH2's
    // lands as its kind 4, `0x1403D1730`). Before t858 the hits were thrown away here.
    // KH2's guard on the Keyblade in the KH2 move KH1 plays now (our guard slots,
    // `kh2slots::keyblade_guard`: the guard press's 161, its repel's 162), as KH2's own hit
    // pass takes it (KH2's controller's rule above: a hit from in front, or from anywhere with
    // Guard all round): no effect, KH1's clash spark, and the guard's reaction by its "refact
    // self" (1 the repel, `combomods::guard_repel`; 2 nothing).
    // (t861) Only where the guard can't go through KH1's builder and meeting (not Steam, or
    // their hooks not in): there KH1's own guard objects on KH2's shapes decide the block
    // (`combat::keyblade_guard_native`), and a hit that reaches his body is taken.
    if super::shared::active() && !super::combat::keyblade_guard_native() {
        if let Some((atk, refact)) = super::kh2slots::keyblade_guard(actor) {
            let f = rd::<[f32; 4]>(actor + 0x30);
            let fwd = glam::Vec2::new(f[0], f[2]).normalize_or(glam::Vec2::Y);
            let yaw = if f[3].is_finite() { f[3] } else { fwd.x.atan2(fwd.y) };
            let all_round = super::combomods::abilities_now().map(|a| a.guard_all_round) == Some(true);
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
                note(|| format!("Sora hit by {from:#x} in KH1's states: KH2's guard on the Keyblade (ATKP {atk}, refact self {refact}), no effect (frame {frame})"));
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
                note(|| format!("Sora hit by {from:#x} in KH1's states: KH2's guard on his body (ATKP {atk}, the KH2 move KH1 plays), no effect (frame {frame})"));
            }
        }
    }
    // (KH1's middle hits as flinches, the Medium Hits row: `hitreact::medium_hits`; before the
    // knock-back note below, which then doesn't see them.)
    super::hitreact::medium_hits(actor, &ready, kind_of);
    // (Not under KH1's no-damage timer: KH1's pass starts no knock-back then, t858.)
    if super::shared::active() && !super::combomods::kh1_no_damage_on(actor) && ready.iter().any(|&r| rd::<u8>(r) & NO_EFFECT == 0 && (rd::<u32>(r + 0xC) >> 6) & 0xF < 9 && matches!(kind_of(actor, r), 4 | 5)) {
        super::combomods::kh1_blown();
    }
    // KH2's push and air flinch on the reaction KH1's pass starts (`hitreact`, Hit Reactions
    // KH2).
    let was = super::hitreact::before_pass(actor);
    // (And KH2's recoil on the attack-clash reaction it starts, `recoil`, Recoil KH2.)
    let clash = super::recoil::before_pass(actor);
    // (Its hurt voice KH2's with Battle Quotes KH2, `hitreact::hurt_line`.)
    super::hitreact::in_pass(true);
    let r = f(actor);
    super::hitreact::in_pass(false);
    super::hitreact::after_pass(actor, was);
    super::recoil::after_pass(actor, clash);
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
