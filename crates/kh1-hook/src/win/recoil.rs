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

use super::styles::{self, Kind};
use super::{log, pose};
use std::sync::atomic::{AtomicU32, Ordering};

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
    if LOGGED.fetch_add(1, Ordering::Relaxed) < 60 {
        log(&s());
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
