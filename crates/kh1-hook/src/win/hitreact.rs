//! KH2's push on KH1's push for Sora's hit reactions (the tester, thread soras-hit-reactions-native,
//! plan step 2: "KH2's push on KH1's push: KH2's distance and its 0.1 / 0.9 curve, for Sora only";
//! `docs/kh1-hit-reactions-native.md` §11). With Hit Reactions at KH2 (`styles::Kind::HitReactions`),
//! base Sora on KH1's states.
//!
//! KH1's flinch reactions push him through KH1's own push (Steam, Checked (decomp)):
//! - `0x1402C7300 (actor, {dir, distance})` starts it: direction `+0x3F0`, speed `+0x3FC` =
//!   distance × 0.22314355 (−ln 0.8), flag `+0x88` bit `0x40`;
//! - `0x1402C7070`, from his physics step (`0x14029A3C0`), each frame moves him by speed × the
//!   frame step (`0x14233FBDC`, 1.0 at 60 fps) through his collision, then speed × 0.8^step, and
//!   clears the flag once the speed is below 1.
//!
//! KH2's (Steam KH2, Checked (decomp)): `0x1403D4960` starts its push at distance × 0.1;
//! `0x1403D4570` each frame stops it below 1, else moves by speed × the frame step and then
//! speed × 0.9^step. The same shape as KH1's, other numbers. So KH1's push runs KH2's curve here:
//! before each of KH1's physics frames (`takeover::physics`) KH1's speed field holds KH2's speed for
//! the frame (KH1's push moves him by it, with his collision), and KH2's decay is kept here. No hook
//! of its own: the hit pass and the physics step are already ours.
//!
//! The distance (the Config menu's Knock-back Numbers row, ini `knockback_numbers`; the tester:
//! "make it a switch in the config"): KH1's enemy attacks carry no KH2 knock-back numbers (no ATKP),
//! so
//! - **KH2** (default): KH2's own rule for an attack with none, sstm's hit-back (200), every hit;
//! - **KH1**: KH1's own number for the hit (btltbl's 750 × the damage's share of his max HP, at most
//!   150, × his push factor, stats `+0xC4`, as `0x14029D210` works it out) on KH2's curve;
//! - **Scaled**: KH2's 200 × KH1's number over KH1's cap (btltbl `+0x24`, 150): a light hit pushes
//!   little, a hit near the flinch's top (20% of his HP) the whole 200.
//!
//! The air flinch's hop and drift the same way: KH2's (`0x1403D4800`, Checked (decomp)): a hop of
//! 50 and drift = the hit-back distance × sstm's 0.048 (`hit_back_to_jump`) along the push; KH1's
//! (`0x14029CD80`): a hop of 20 (`0x1402AF960`) and drift min(40 × share, 8) × the push factor
//! (`0x14029D800`, btltbl `+0x30` / `+0x34`). Through `shared::physics::kh2_hop` (KH2's arc at
//! physics KH2, KH1's gravity at KH1).

use super::shared::physics;
use super::styles::{self, Kind};
use super::{log, pose};
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Mutex;

/// The Knock-back Numbers setting (`knockback_numbers`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Numbers {
    Kh1 = 0,
    Scaled = 1,
    Kh2 = 2,
}

impl Numbers {
    pub const ALL: [Numbers; 3] = [Numbers::Kh1, Numbers::Scaled, Numbers::Kh2];

    pub fn key(self) -> &'static str {
        match self {
            Numbers::Kh1 => "kh1",
            Numbers::Scaled => "scaled",
            Numbers::Kh2 => "kh2",
        }
    }

    /// The ini's word (KH2's by default).
    pub fn parse(v: Option<&str>) -> Numbers {
        match v {
            Some("kh1") => Numbers::Kh1,
            Some("scaled") => Numbers::Scaled,
            _ => Numbers::Kh2,
        }
    }
}

/// 0 not read yet, else 1 + `Numbers`.
static NUMBERS: AtomicU8 = AtomicU8::new(0);

pub fn numbers() -> Numbers {
    match NUMBERS.load(Ordering::Relaxed) {
        0 => {
            let n = super::settings::get().knockback_numbers;
            NUMBERS.store(1 + n as u8, Ordering::Relaxed);
            n
        }
        v => Numbers::ALL[(v as usize - 1).min(2)],
    }
}

/// The Config menu's row: live, kept in the ini.
pub fn set_numbers(n: Numbers) {
    NUMBERS.store(1 + n as u8, Ordering::Relaxed);
    super::settings::save("knockback_numbers", n.key());
    log(&format!("Config menu: Knock-back Numbers {:?}", n));
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// KH1's push (Checked (decomp), `0x1402C7300` / `0x1402C7070`).
const PUSH_SPEED: usize = 0x3FC;
const PUSH_FLAG_FIELD: usize = 0x88;
const PUSH_FLAG: u32 = 0x40;
const KH1_START: f32 = 0.223_143_55;
const KH1_DECAY: f32 = 0.8;
/// KH2's (Checked (decomp), `0x1403D4960` / `0x1403D4570`).
const KH2_START: f32 = 0.1;
const KH2_DECAY: f32 = 0.9;
/// KH2's air flinch hop (`0x1403D4800`: 50.0, Checked (decomp)) and KH1's (`0x14029CD80`: 20.0).
const KH2_AIR_HOP: f32 = 50.0;
const KH1_AIR_HOP: f32 = 20.0;
/// KH1's motion queue: count `+0x178`, first entry's request `+0x17C` (`0x14029F980`, Checked
/// (decomp)).
const QUEUE: usize = 0x178;
/// btltbl's reaction block pointer (Steam `0x142D22D30`, `0x14029D210` reads its `+0x20` / `+0x24`;
/// Checked (decomp)): the flinch push's cap `+0x24` and the air drift's cap `+0x34`.
const STEAM_REACTION_BLOCK: usize = 0x2D22D30;
const KH1_PUSH_CAP: f32 = 150.0;
const KH1_AIR_CAP: f32 = 8.0;

/// KH2's push running on KH1's: (KH2's speed this frame, what was written into KH1's field).
static PUSH: Mutex<Option<(f32, f32)>> = Mutex::new(None);
static LOGGED: AtomicU32 = AtomicU32::new(0);

fn note(s: impl FnOnce() -> String) {
    if LOGGED.fetch_add(1, Ordering::Relaxed) < 60 {
        log(&s());
    }
}

/// KH2's reactions apply now: base Sora on KH1's states, Hit Reactions KH2.
fn on() -> bool {
    super::shared::active() && styles::kh2(Kind::HitReactions) && !super::form::in_form()
}

/// KH1's queue head and count, before its hit pass (`after_pass` tells a reaction it set).
pub unsafe fn before_pass(actor: usize) -> (u32, u32) {
    (rd(actor + QUEUE), rd(actor + QUEUE + 4))
}

unsafe fn caps() -> (f32, f32) {
    let base = super::takeover::game_base();
    if super::addr::Build::current() != super::addr::Build::Steam {
        return (KH1_PUSH_CAP, KH1_AIR_CAP);
    }
    let block = rd::<u64>(base + STEAM_REACTION_BLOCK) as usize;
    if block < 0x10000 || !super::crash::readable(block + 0x38) {
        return (KH1_PUSH_CAP, KH1_AIR_CAP);
    }
    let (a, b) = (rd::<f32>(block + 0x24), rd::<f32>(block + 0x34));
    (if a.is_finite() && a > 0.0 { a } else { KH1_PUSH_CAP }, if b.is_finite() && b > 0.0 { b } else { KH1_AIR_CAP })
}

/// KH2's hit-back distance and its air drift factor (sstm, `pose::kh2`; KH2's 200 / 0.048 without).
fn kh2_numbers() -> (f32, f32) {
    pose::kh2().map_or((200.0, 0.048), |k| (k.character.reaction.hit_back_small, k.character.reaction.hit_back_to_jump))
}

/// After KH1's hit pass on Sora (`hurt`): the reaction it started, given KH2's push or hop.
/// `was`: `before_pass`.
pub unsafe fn after_pass(actor: usize, was: (u32, u32)) {
    if !on() {
        return;
    }
    let now: (u32, u32) = (rd(actor + QUEUE), rd(actor + QUEUE + 4));
    if now == was || now.0 == 0 {
        return;
    }
    let request = now.1 & 0xFFFF;
    let (hit_back, to_jump) = kh2_numbers();
    let (push_cap, air_cap) = caps();
    let how = numbers();
    match request {
        // The ground flinches (`kh2slots::REACTIONS`): KH1's push just started.
        0x48..=0x4B => {
            if rd::<u32>(actor + PUSH_FLAG_FIELD) & PUSH_FLAG == 0 {
                return;
            }
            let kh1 = rd::<f32>(actor + PUSH_SPEED) / KH1_START;
            if !kh1.is_finite() {
                return;
            }
            let d = match how {
                Numbers::Kh2 => hit_back,
                Numbers::Kh1 => kh1,
                Numbers::Scaled => hit_back * (kh1 / push_cap).clamp(0.0, 1.0),
            };
            let v = d * KH2_START;
            wr(actor + PUSH_SPEED, v);
            if let Ok(mut p) = PUSH.lock() {
                *p = Some((v, v));
            }
            note(|| format!("KH2's push on KH1's (request {request:#x}, {how:?}): {d:.0} (KH1's own {kh1:.0}), {v:.1} a frame, x {KH2_DECAY} a frame"));
        }
        // The air flinches: KH1's hop (state 2) and drift just set.
        0x4E | 0x4F => {
            if rd::<i32>(actor + 0x70) != 2 {
                return;
            }
            let vel: [f32; 4] = rd(actor + 0x3C0);
            let kh1 = vel[0].hypot(vel[2]);
            let dir = if kh1 > 1e-4 {
                [vel[0] / kh1, vel[2] / kh1]
            } else {
                let f: [f32; 4] = rd(actor + 0x30);
                [-f[0], -f[2]]
            };
            let (hop, speed) = match how {
                Numbers::Kh2 => (KH2_AIR_HOP, hit_back * to_jump),
                Numbers::Kh1 => (KH1_AIR_HOP, kh1),
                Numbers::Scaled => (KH2_AIR_HOP, hit_back * to_jump * (kh1 / air_cap).clamp(0.0, 1.0)),
            };
            physics::kh2_hop(actor, hop, [dir[0] * speed, dir[1] * speed]);
            note(|| format!("KH2's air flinch on KH1's (request {request:#x}, {how:?}): hop {hop:.0}, drift {speed:.1} a frame (KH1's own {kh1:.1})"));
        }
        _ => {}
    }
}

/// Before KH1's physics frame for Sora (`takeover::physics`): KH2's push speed for this frame in
/// KH1's push, and KH2's decay for the next. Dropped when KH1 started a push of its own meanwhile
/// (its field isn't what KH1's push leaves of ours) or KH2's states have him.
pub unsafe fn push_frame(actor: usize) {
    let Ok(mut g) = PUSH.lock() else { return };
    let Some((v, written)) = *g else { return };
    let field: f32 = rd(actor + PUSH_SPEED);
    let dt = physics::dt();
    let kh1_left = written * KH1_DECAY.powf(dt);
    let ours = (field - written).abs() < 1e-3 || (field - kh1_left).abs() < 1e-3 * written.max(1.0);
    if !on() || !ours {
        *g = None;
        return;
    }
    if v < 1.0 {
        wr(actor + PUSH_SPEED, 0.0f32);
        wr(actor + PUSH_FLAG_FIELD, rd::<u32>(actor + PUSH_FLAG_FIELD) & !PUSH_FLAG);
        *g = None;
        return;
    }
    // (KH1's push moves him by `v` × the step this frame; KH2's speed after it.)
    wr(actor + PUSH_SPEED, v);
    wr(actor + PUSH_FLAG_FIELD, rd::<u32>(actor + PUSH_FLAG_FIELD) | PUSH_FLAG);
    *g = Some((v * KH2_DECAY.powf(dt), v));
}
