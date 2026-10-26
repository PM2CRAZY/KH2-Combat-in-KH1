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
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
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
    log!(Combat, "Config menu: Knock-back Numbers {:?}", n);
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
    if !super::log_on(super::LogCat::Combat) {
        return;
    }
    if super::log_on(super::LogCat::Combat) && LOGGED.fetch_add(1, Ordering::Relaxed) < 60 {
        log!(Combat, "{}", s());
    }
}

/// KH2's reactions apply now: base Sora on KH1's states, Hit Reactions KH2.
fn on() -> bool {
    super::shared::active() && (super::form::native_now() || styles::kh2(Kind::HitReactions) && !super::form::in_form())
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
            // KH2's turn (the tester's m2 answer 2: "(a)"): toward the attacker from the front,
            // away from him from behind (`air_flinch_turn`).
            if kh1 > 1e-4 {
                air_flinch_turn(actor, request, dir);
            }
            note(|| format!("KH2's air flinch on KH1's (request {request:#x}, {how:?}): hop {hop:.0}, drift {speed:.1} a frame (KH1's own {kh1:.1})"));
        }
        _ => {}
    }
}

/// KH1's facing set (Steam `0x140297E10 (actor, angle)`: `+0x30..+0x3C` = sin, 0, cos, angle,
/// skipped while `+0x374` bit `0x1000` holds his facing; Checked (decomp)), as `glide` uses it.
const SET_FACING_STEAM: usize = 0x297E10;
/// His wanted facing (KH1's knock-back reaction writes it with the facing, `0x14029CEC0`).
const FACING_WANTED: usize = 0x150;

/// KH2's air flinch turns him (Steam KH2 `0x1403D3840`, Checked (decomp)): along the hit's push
/// direction, negated when the hit is from the front (dot(facing, push) < 0), set as his facing at
/// once; so he faces the attacker when hit from the front and faces away when hit from behind
/// (`character_core::reaction::respond`'s `AirFlinch::face`, the same rule). KH1's air flinch
/// (`0x14029CD80`) doesn't turn him; its knock-back (`0x14029CEC0`) turns him by this very rule
/// with KH1's facing set and `+0x150` (Checked (decomp)), so the turn is made the way KH1 makes it.
/// `dir`: the push direction KH1's air flinch just gave his air velocity (`0x14029D800`: record
/// `+0x10` × its drift, Checked (decomp)); `request` 0x4E front, 0x4F behind (KH1's own front test,
/// the same as KH2's, §9). Only in KH1's air state 2 (the caller): flight and swimming keep KH1's
/// (the tester's m2 answer 3: "(b)").
unsafe fn air_flinch_turn(actor: usize, request: u32, dir: [f32; 2]) {
    if super::addr::Build::current() != super::addr::Build::Steam {
        return;
    }
    let face = if request == 0x4E { [-dir[0], -dir[1]] } else { dir };
    let angle = face[0].atan2(face[1]);
    let set: unsafe extern "system" fn(usize, f32) = std::mem::transmute(super::takeover::game_base() + SET_FACING_STEAM);
    set(actor, angle);
    wr(actor + FACING_WANTED, angle);
    note(|| format!("KH2's air flinch turn (request {request:#x}): facing {:.2} rad ({} the attacker)", angle, if request == 0x4E { "toward" } else { "away from" }));
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

/// KH1's middle kind of hit (kind 4: 20..29% of his max HP, btltbl `+0x10` / `+0x11`, mapped from
/// kind 1 by `0x1402BF290`, Checked (decomp)) as a KH2 flinch (the Config menu's Medium Hits row,
/// ini `medium_hits = blow | flinch`; the tester: "how sora receives knock back should also be a
/// config"): only kind 5 (30% and up) knocks him away. Blow (default): KH1's kind 4 as it is, a
/// knock-back (KH1's knock-down on the ground until KH2's blow is in, plan step 3; KH2's controller's
/// blow in a borrow). 0 not read yet, 1 blow, 2 flinch.
static MEDIUM: AtomicU8 = AtomicU8::new(0);

pub fn medium_flinch() -> bool {
    match MEDIUM.load(Ordering::Relaxed) {
        0 => {
            let f = super::settings::get().medium_hits_flinch;
            MEDIUM.store(1 + f as u8, Ordering::Relaxed);
            f
        }
        v => v == 2,
    }
}

/// The Config menu's row: live, kept in the ini.
pub fn set_medium_flinch(f: bool) {
    MEDIUM.store(1 + f as u8, Ordering::Relaxed);
    super::settings::save("medium_hits", if f { "flinch" } else { "blow" });
    log!(Combat, "Config menu: Medium Hits {}", if f { "Flinch" } else { "Blow" });
}

/// Where the Medium Hits row applies (the tester, m3 question 3: "(b)", only on the ground and in
/// normal air): the movement states (`+0x70`) for which KH1's hit pass (Steam `0x14029BBD0`) runs
/// its ground reactions (0, 1, 0xC, 0x1C, 0x20, 0x26) or its plain air ones (any state it has no
/// row for: the air states 2, 3, ...), Checked (decomp). Left out, KH1's own kind 4 kept: flight
/// (8), swimming (0x11), Atlantica's water states 0x18 / 0x19 (KH1 gives them its ground
/// functions, but they are swimming: stages 111 / 118), and the states with rows of their own,
/// 0x1D (the carpet), 0x1E / 0x1F (the Jungle Slider) and 0x27 (no reaction).
fn medium_state(state: u32) -> bool {
    !matches!(state, 8 | 0x11 | 0x18 | 0x19 | 0x1D | 0x1E | 0x1F | 0x27)
}

/// Before KH1's hit pass on Sora on KH1's states (`hurt`), Hit Reactions KH2 and Medium Hits
/// Flinch: each ready record KH1 would map to kind 4 is given kind 3, which `0x1402BF290` passes
/// through as it is (only kind 1 is mapped by the damage), so KH1's pass runs its flinch for it
/// (and its reaction order: a kind 5 in the same pass still wins). `kind_of` is KH1's mapping.
pub unsafe fn medium_hits(actor: usize, ready: &[usize], kind_of: unsafe extern "system" fn(usize, usize) -> u32) {
    if !on() || !medium_flinch() || !medium_state(rd::<u32>(actor + 0x70)) {
        return;
    }
    for &r in ready {
        if rd::<u8>(r) & 0x20 != 0 || (rd::<u32>(r + 0xC) >> 6) & 0xF >= 9 {
            continue;
        }
        if kind_of(actor, r) == 4 {
            wr(r + 0xC, (rd::<u32>(r + 0xC) & !0x1F) | 3);
            note(|| format!("Medium Hits Flinch: a kind 4 hit ({} damage) taken as KH1's flinch (kind 3)", rd::<i32>(r + 8)));
        }
    }
}

/// KH2's hurt line with the Battle Quotes row at KH2 (the tester, soras-hit-reactions-native,
/// m2 question 1: "(b)", KH2's). KH2 says it from its reaction motion's frame trigger 14 at frame
/// 0 (`P_EX100.mset`, Checked (files)): the flinches 10 / 12 / 14 battle voice 29, their
/// alternates 11 / 13 and the air flinch from behind 15 voice 30, the blows 16 / 18 voice 1;
/// `00battle.bin`'s VTBL for Sora (Checked (files)): 29 clip 6, 30 clip 5, 1 clip 7, each alone,
/// priority 3. KH1's hit pass (Steam `0x14029BBD0`) ends with its hurt voice group,
/// `0x1402901F0 (actor, group, 0)`: 0 a flinch (kind 3), 1 a knock-back (4 / 5), 0x14 his low-HP
/// line, 2 his HP gone (Checked (decomp)); the reaction it started is queued by then. So at that
/// call groups 0 / 1 become KH2's voice for the reaction KH1 just asked for, through KH1's own
/// voice path (`kh2sound::hurt_voice`, as every KH2 battle quote is), at KH2's moment (the
/// reaction's first frame). The low-HP and HP-gone lines stay KH1's (KH2's motions have none).
/// Whatever the Hit Reactions row says: the Battle Quotes row decides the voice.
const KH2_FLINCH_VOICE: i16 = 29;
const KH2_FLINCH_ALT_VOICE: i16 = 30;
const KH2_BLOW_VOICE: i16 = 1;

/// Sora's KH1 hit pass is running (`hurt`, KH1's states only).
static IN_PASS: AtomicBool = AtomicBool::new(false);

/// `hurt`: around KH1's hit pass for Sora on KH1's states.
pub fn in_pass(on: bool) {
    IN_PASS.store(on, Ordering::Relaxed);
}

/// KH1's voice group play (`soundlog`'s hook on `0x1402901F0`): true when it was Sora's hurt
/// voice in his hit pass and KH2's line was said instead (KH1's is skipped); false leaves KH1's
/// (Battle Quotes KH1, another group, or KH2's voice not loaded).
pub unsafe fn hurt_line(actor: usize, group: i32) -> bool {
    if !IN_PASS.load(Ordering::Relaxed) || !matches!(group, 0 | 1) || actor == 0 || actor != super::takeover::sora_actor() || !super::kh2sound::kh2_voice() {
        return false;
    }
    let (count, request): (u32, u32) = (rd(actor + QUEUE), rd::<u32>(actor + QUEUE + 4) & 0xFFFF);
    let v = if group == 1 {
        KH2_BLOW_VOICE
    } else if count != 0 && matches!(request, 0x49 | 0x4B | 0x4F) {
        KH2_FLINCH_ALT_VOICE
    } else {
        KH2_FLINCH_VOICE
    };
    let said = super::kh2sound::hurt_voice(v);
    note(|| format!("Sora's hurt voice (KH1's group {group}, request {request:#x}): {}", if said { format!("KH2's voice {v}") } else { format!("KH2's voice {v} not loaded, KH1's") }));
    said
}
