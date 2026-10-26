//! Sora's recoil when his attack meets an enemy's attack (the tester, thread
//! kh2-recoil-and-parry-system, plan step 2, answers 1 "(b)" and 4; `docs/kh1-kh2-recoil.md` §6.2).
//! The Config menu's Recoil row (`styles::Kind::Recoil`, ini `recoil`): at KH1 everything stays
//! KH1's; at KH2 base Sora on KH1's states gets KH2's recoil on KH1's own recoil reactions.
//!
//! KH1's attack clash puts a kind 6 / 7 hit record on Sora, and its hit pass (Steam `0x14029BBD0`)
//! runs a reaction for it (Steam, Checked (decomp)):
//! - kind 6 on the ground, `0x14029D720`: request 0x6E, the idle queued, the no-damage timer at 8
//!   (`0x140297F30 (actor, 8.0)`: `+0xC` = 8.0, `+0` flag `0x80` on, `0x100000` off), KH1's combo
//!   count (combat state `+0x41`) less one, and when the record's `+8` is below 0 a push back along
//!   minus his facing by the priority gap × btltbl `+0x1C` × his push factor (`0x1402C7300`);
//! - kind 7 on the ground, `0x14029D5F0`: request 0x6F (KH1's big recoil), the same without the push;
//! - in the air, `0x14029D510`: request 0x70 then the fall (6), the same timer, a hop of 20
//!   (`0x1402AF960`) and a drift back from his facing by the gap (btltbl `+0x30`, capped `+0x34`).
//!
//! KH2's (`0x1403D61D0` → `0x1403D4650`, Steam KH2, Checked (decomp)): motion 41 on the ground,
//! pushed back 50 along minus his facing (`0x1403D4960`: 0.1 × 50 a frame, × 0.9 a frame); motion
//! 43 in the air with the hop `0x1403D4800` (50 up, 50 × sstm's `hit_back_to_jump` back from his
//! facing); his combo counter `+0xD74` less one (as KH1's); no protection of any kind.
//!
//! So at KH2: the motions are `kh2slots::RECOILS` (41 on 0x6E, 43 on 0x70; 0x6F keeps KH1's big
//! recoil, answer 1); after the pass KH1's push is replaced by KH2's 50 (through
//! `combomods::push_back`, KH1's per-frame push `+0x120`), the air hop by KH2's, and the no-damage
//! timer the recoil started is put back as it was before the pass (KH2 gives none, answer 4). No
//! hook of its own: the hit pass is `hurt`'s.

use super::addr::Build;
use super::styles::{self, Kind};
use super::sys::*;
use super::{log, pose};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// KH1's motion queue: count `+0x178`, first entry's request `+0x17C` (`0x14029F980`).
const QUEUE: usize = 0x178;
/// KH1's no-damage timer and its flags (`0x140297F30`, Checked (decomp)).
const TIMER: usize = 0xC;
const PROTECTED: u32 = 0x80;
const CLEARED_WITH: u32 = 0x10_0000;
/// KH1's push (`0x1402C7300`: speed `+0x3FC`, flag `+0x88` bit `0x40`; Checked (decomp)).
const PUSH_SPEED: usize = 0x3FC;
const PUSH_FLAG_FIELD: usize = 0x88;
const PUSH_FLAG: u32 = 0x40;
/// KH1's recoil requests (above).
const GROUND: u32 = 0x6E;
const BIG: u32 = 0x6F;
const AIR: u32 = 0x70;
/// KH2's recoil distance (`0x1403D4650`: -1 → 50.0) and air hop (`0x1403D4800`: 50.0).
const KH2_DISTANCE: f32 = 50.0;
const KH2_HOP: f32 = 50.0;

static LOGGED: AtomicU32 = AtomicU32::new(0);

fn note(s: impl FnOnce() -> String) {
    if !super::log_on(super::LogCat::Combat) {
        return;
    }
    if super::log_on(super::LogCat::Combat) && LOGGED.fetch_add(1, Ordering::Relaxed) < 60 {
        log!(Combat, "{}", s());
    }
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// KH2's recoil rules (the Recoil row).
pub fn kh2() -> bool {
    styles::kh2(Kind::Recoil)
}

/// Whether a KH2 guard window of `motion` on `weapon` (KH2's index: 0 his body, 1 / 2 the
/// Keyblades) guards (the tester, thread kh2-recoil-and-parry-system m2: "guard break and other kh
/// moves still use the kh2s recoil mechanics when I set recoil to kh1 ... everything should work how
/// kh1 handles when you hit an enemy"; his trace and log: Guard Break's own Keyblade guard, ATKP 162
/// frames 5..42, stopped the enemy's three hits at Recoil KH1). KH1's swings carry no guard: an
/// enemy attack meeting one is KH1's clash by priority (`0x1402B4BC0`, Checked (decomp)), or it
/// hits him. So at Recoil KH1 base Sora's Keyblade guard windows guard only in his guard moves (the
/// guard press 173 and its repel 42, KH1's Block's place); in his attacks (Guard Break 165, Vicinity
/// Break 170) they don't. His body guards (Aerial Dodge, Flash Step, Aerial Recovery, Counterguard,
/// Vicinity Break's 406) are untouched here. Recoil KH2 and the Drive Forms: every window, as KH2.
pub fn guard_window_on(motion: u16, weapon: u8) -> bool {
    use character_core::motion::{GUARD, GUARD_PRESS};
    kh2() || super::form::in_form() || weapon == 0 || motion == GUARD_PRESS || motion == GUARD
}

/// Base Sora on KH1's states (a Drive Form's recoils are its KH2 controller's).
fn on() -> bool {
    super::shared::active() && !super::form::in_form()
}

/// What KH1's hit pass may change: its queue head, the no-damage timer and its flags.
pub struct Before {
    queue: (u32, u32),
    timer: f32,
    flags: u32,
}

pub unsafe fn before_pass(actor: usize) -> Before {
    Before { queue: (rd(actor + QUEUE), rd(actor + QUEUE + 4)), timer: rd(actor + TIMER), flags: rd(actor) }
}

/// After KH1's hit pass on Sora (`hurt`): a recoil it started, given KH2's push, hop and no
/// protection.
pub unsafe fn after_pass(actor: usize, b: Before) {
    if !on() || !kh2() {
        return;
    }
    let now: (u32, u32) = (rd(actor + QUEUE), rd(actor + QUEUE + 4));
    if now == b.queue || now.0 == 0 {
        return;
    }
    let request = now.1 & 0xFFFF;
    if !matches!(request, GROUND | BIG | AIR) {
        return;
    }
    // KH2's recoil gives no protection: KH1's 8 frames undone (whatever ran before stays).
    let keep = PROTECTED | CLEARED_WITH;
    let flags: u32 = rd(actor);
    wr(actor, (flags & !keep) | (b.flags & keep));
    wr(actor + TIMER, b.timer);
    let f: [f32; 4] = rd(actor + 0x30);
    let len = f[0].hypot(f[2]);
    let back = if len > 1e-4 { [-f[0] / len, -f[2] / len] } else { [0.0, 0.0] };
    match request {
        GROUND => {
            // KH1's gap push (when it started one) replaced by KH2's 50.
            let kh1 = if rd::<u32>(actor + PUSH_FLAG_FIELD) & PUSH_FLAG != 0 { rd::<f32>(actor + PUSH_SPEED) } else { 0.0 };
            wr(actor + PUSH_SPEED, 0.0f32);
            wr(actor + PUSH_FLAG_FIELD, rd::<u32>(actor + PUSH_FLAG_FIELD) & !PUSH_FLAG);
            super::combomods::push_back(back, KH2_DISTANCE);
            note(|| format!("recoil (KH2): motion 41 on KH1's request {request:#x}, pushed back {KH2_DISTANCE:.0} (KH1's push speed was {kh1:.1}), no protection (timer back to {:.0})", b.timer));
        }
        AIR => {
            if rd::<i32>(actor + 0x70) != 2 {
                return;
            }
            let to_jump = pose::kh2().map_or(0.048, |k| k.character.reaction.hit_back_to_jump);
            let speed = KH2_DISTANCE * to_jump;
            super::shared::physics::kh2_hop(actor, KH2_HOP, [back[0] * speed, back[1] * speed]);
            note(|| format!("recoil (KH2): motion 43 on KH1's request {request:#x}, hop {KH2_HOP:.0}, drift {speed:.1} a frame back, no protection"));
        }
        _ => note(|| format!("recoil (KH2): KH1's big recoil (request {request:#x}, its own animation, answer 1), no protection")),
    }
}

// ---------------------------------------------------------------------------------------------
// KH2's "refact self" 2 against a guard (the tester, thread kh2-recoil-and-parry-system m2, item 2:
// "add it and include in the kh2 config for recoil"; `docs/kh1-kh2-recoil.md` §6.9).
//
// KH2 (`0x1403D2140`, Steam KH2, Checked (decomp)): when a normal attack with refact self 2 meets a
// guard, the attack goes on (refact self 2: not stopped) and the guard's side, the guard and its
// owner, becomes immune to that attack (the exemption is only for an *attack* that is itself a
// refact-2 guard). So the guarded enemy takes no hit from that swing.
//
// KH1 (Steam, Checked (decomp)): the collision pass `0x1402B3BE0` calls the meeting
// `0x1402B4A20 (entry, entry)` for two attack entries that overlap and pass KH1's pairing table;
// the meeting runs each side's directional test `0x1402B4640 (own, other)` and returns if either
// fails, then the two halves `0x1402B4BC0`. Our refact-2 rows are at priority 0x62 or more at
// Recoil KH2 (`kh1rows::kh2_clash`) and a guard is 100, so both halves return 0 at once (one side
// must be under 0x62): nothing happens and the swing reaches the body behind the guard. A body is
// skipped by the same pass when the actor's hit-once bits (`+0x3E0`) hold the attack's bit
// (`+0x54`); KH1 sets that bit when a hit lands and takes it off when the attack re-arms
// (`0x1402B3380`) or is freed (the in-use mask, `0x1402B4460`). That is KH1's own "this actor is
// immune to this attack".
//
// So the meeting is hooked: after KH1's own, when one side is Sora's attack of a refact-2 row
// (`kh1rows::unstoppable`) and the other an enemy's guard (priority 100, owner not Sora), both
// directional tests pass and the Recoil row is KH2, the guard's owner gets the swing's hit-once
// bit (immune until the swing re-arms or ends, as KH2's immunity lasts the attack's life), with
// KH1's clash spark standing in for KH2's clash effect (KH2's `0x1403D61D0` plays effect 0xD only
// for refact self 2). Recoil KH1: nothing (KH1's rule: the swing hits through).
// ---------------------------------------------------------------------------------------------

/// KH1's attack meeting (Steam `0x1402B4A20`): `push rbx; push rsi; push rdi; push r14;
/// sub rsp, 0x78` (Checked (exe)).
const MEET_STEAM: usize = 0x2B4A20;
const MEET_PROLOGUE: [u8; 10] = [0x40, 0x53, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x78];
/// KH1's directional test (Steam `0x1402B4640 (own, other) -> 0 / 1`; it only reads; Checked
/// (decomp)), its first bytes checked before a call (Checked (exe)).
const FACES_STEAM: usize = 0x2B4640;
const FACES_BYTES: [u8; 8] = [0x4C, 0x8B, 0xDC, 0x49, 0x89, 0x5B, 0x20, 0x57];
/// A KH1 guard's priority (`combat::GUARD_PRIORITY`; KH1's half `0x1402B4BC0` treats 100 as a
/// guard).
const GUARD_PRIORITY: i32 = 100;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_MEET: AtomicUsize = AtomicUsize::new(0);

/// Hooks KH1's attack meeting (Steam only; the hook checks its bytes first).
pub unsafe fn install() {
    if Build::current() != Build::Steam {
        return;
    }
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    super::hook::install(base + MEET_STEAM, &MEET_PROLOGUE, meet as *const () as usize, &ORIG_MEET, "KH1's attack meeting (KH2's refact self 2 against guards)");
}

unsafe extern "system" fn meet(a: usize, b: usize) {
    let orig: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_MEET.load(Ordering::Relaxed));
    orig(a, b);
    if !kh2() || !on() {
        return;
    }
    let (oa, ob) = (rd::<usize>(a + 0x18), rd::<usize>(b + 0x18));
    if oa < 0x10000 || ob < 0x10000 {
        return;
    }
    let sora = super::takeover::sora_actor();
    if sora < 0x10000 {
        return;
    }
    let owner = |o: usize| pose::ptr(rd::<u32>(o + 0x34));
    let mine = |o: usize| owner(o) == sora && super::kh1rows::unstoppable(o);
    let guard = |o: usize| rd::<i32>(o + 0x68) == GUARD_PRIORITY && { let w = owner(o); w > 0x10000 && w != sora };
    let (swing, g, ge, se) = if mine(oa) && guard(ob) {
        (oa, ob, b, a)
    } else if mine(ob) && guard(oa) {
        (ob, oa, a, b)
    } else {
        return;
    };
    let base = BASE.load(Ordering::Relaxed);
    let faces_at = base + FACES_STEAM;
    if std::slice::from_raw_parts(faces_at as *const u8, FACES_BYTES.len()) != FACES_BYTES {
        return;
    }
    let faces: unsafe extern "system" fn(usize, usize) -> i32 = std::mem::transmute(faces_at);
    if faces(g, swing) == 0 || faces(swing, g) == 0 {
        return;
    }
    let target = owner(g);
    let bit: u32 = rd(swing + 0x54);
    let had: u32 = rd(target + 0x3E0);
    if bit == 0 || had & bit != 0 {
        return;
    }
    wr(target + 0x3E0, had | bit);
    // (KH1's spark where the two entries met, as KH1's meeting places its own: halfway between
    // the entries' `+0x30`.)
    let p: [f32; 4] = rd(ge + 0x30);
    let q: [f32; 4] = rd(se + 0x30);
    super::combat::clang([(p[0] + q[0]) * 0.5, (p[1] + q[1]) * 0.5, (p[2] + q[2]) * 0.5]);
    note(|| format!("recoil (KH2): Sora's refact-self-2 swing {swing:#x} met the guard {g:#x} of actor {target:#x}: not stopped, that actor immune to it (hit-once bit {bit:#x})"));
}
