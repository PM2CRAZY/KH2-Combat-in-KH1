//! KH2's knock-back on enemies, through KH1's own enemy reactions (the tester, thread
//! kh2-knockback-on-enemies: "(b) but make it a toggle in the config menu / (a) / (a) / go ahead").
//! `docs/kh1-launch.md` §4.
//!
//! KH1's reactions stay the only system: KH1's launch (`0x1402B8530`), its flight in its air
//! state, its push and its air flinch run on the enemy as in KH1, with KH2's numbers in them for
//! the hits KH2's moves land. Hits of KH1's own moves keep KH1's numbers.
//!
//! **The enemy's weight** (question 1 (b), the Config menu's Combat > Tweaks > Enemy Weight row,
//! ini `enemy_weight`, on by default): KH1 scales its reactions by one stat of the enemy's, stats
//! `+0xC4` (the actor's `+0x6C` block; Checked (decomp)):
//! - its push (`0x1402B7E60`): btltbl's 80 × it, and its air flinch's drift (`0x1402B8B90`): 4 × it;
//! - its launch (`0x1402B8530`): a speed of 20 + (35 − 20) × it, its height 1.35 × that.
//!
//! On, KH2's numbers are multiplied the same way: pushes and drifts by the stat itself, launches
//! (both speeds) by KH1's launch speed for this enemy over KH1's fastest (35, the stat at 1).
//! **Assumption:** the stat's 1.0 is KH1's "full" weight (the launch's top speed, the push's full
//! 80), so an enemy at 1.0 takes KH2's numbers as they are. Off, every enemy takes KH2's.

use super::{log, pose};
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

/// btltbl's reaction block pointer (Steam `0x142D22D30`; `0x1402B8530` reads its u16 `+0x14` /
/// `+0x16`, the launch speed's two ends; Checked (decomp)).
const REACTION_BLOCK_STEAM: usize = 0x2D22D30;
const LAUNCH_LOW: f32 = 20.0;
const LAUNCH_HIGH: f32 = 35.0;
/// The enemy's reaction weight in its stats block (`+0x6C`, a handle).
const STATS: usize = 0x6C;
const WEIGHT: usize = 0xC4;

/// The Enemy Weight setting: 0 not read yet, 1 off, 2 on.
static WEIGHT_ON: AtomicU8 = AtomicU8::new(0);

pub fn weight() -> bool {
    match WEIGHT_ON.load(Ordering::Relaxed) {
        0 => {
            let w = super::settings::get().enemy_weight;
            WEIGHT_ON.store(1 + w as u8, Ordering::Relaxed);
            w
        }
        v => v == 2,
    }
}

/// The Config row (Combat > Tweaks > Enemy Weight): live, kept in the ini.
pub fn set_weight(on: bool) {
    WEIGHT_ON.store(1 + on as u8, Ordering::Relaxed);
    super::settings::save("enemy_weight", if on { "on" } else { "off" });
    log!(Combat, "Config menu: Enemy Weight {}", if on { "On" } else { "Off" });
}

/// Read again from the settings on the next call (`experimental` switched in the ini, t922).
pub fn forget_weight() {
    WEIGHT_ON.store(0, Ordering::Relaxed);
}

/// KH1's weight stat of this enemy (stats `+0xC4`), if it reads as a number.
pub unsafe fn kh1_weight(actor: usize) -> Option<f32> {
    if actor < 0x10000 {
        return None;
    }
    let stats = pose::ptr(rd::<u32>(actor + STATS));
    if stats < 0x10000 || !super::crash::readable(stats + WEIGHT) {
        return None;
    }
    let w = rd::<f32>(stats + WEIGHT);
    let w = w.is_finite().then_some(w.clamp(0.0, 8.0));
    note_weight(actor, w);
    w
}

/// Each enemy's weight, logged the first time it is read (by its object), so the log tells what
/// KH1 gives its enemies.
unsafe fn note_weight(actor: usize, w: Option<f32>) {
    static SEEN: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());
    let obj = rd::<u32>(actor + 0x130);
    let Ok(mut seen) = SEEN.lock() else { return };
    if seen.contains(&obj) || seen.len() > 200 {
        return;
    }
    seen.push(obj);
    log!(Combat, "enemy weight: actor {actor:#x} (object {obj:#x}) KH1 stat +0xC4 = {w:?}");
}

/// KH1's launch speed ends (btltbl, read from the game; 20 / 35 if it can't be).
unsafe fn launch_ends() -> (f32, f32) {
    if super::addr::Build::current() != super::addr::Build::Steam {
        return (LAUNCH_LOW, LAUNCH_HIGH);
    }
    let block = rd::<u64>(super::takeover::game_base() + REACTION_BLOCK_STEAM) as usize;
    if block < 0x10000 || !super::crash::readable(block + 0x18) {
        return (LAUNCH_LOW, LAUNCH_HIGH);
    }
    let (lo, hi) = (rd::<u16>(block + 0x14) as f32, rd::<u16>(block + 0x16) as f32);
    if hi > 0.0 { (lo, hi) } else { (LAUNCH_LOW, LAUNCH_HIGH) }
}

/// The factor on a KH2 launch's speeds for this enemy: KH1's launch speed for it over KH1's
/// fastest; 1 with Enemy Weight off.
pub unsafe fn launch_factor(actor: usize) -> f32 {
    if !weight() {
        return 1.0;
    }
    let Some(w) = kh1_weight(actor) else { return 1.0 };
    let (lo, hi) = launch_ends();
    let k = (lo + (hi - lo) * w) / hi;
    static N: AtomicU32 = AtomicU32::new(0);
    if super::log_on(super::LogCat::Combat) && N.fetch_add(1, Ordering::Relaxed) < 20 {
        log!(Combat, "enemy weight on a KH2 launch: stat {w:.2}, KH1's launch {:.1} of {hi:.0}: x{k:.2}", lo + (hi - lo) * w);
    }
    k
}

// ---------------------------------------------------------------------------------------------
// The flight: KH2's blow rules in KH1's air state's step.
// ---------------------------------------------------------------------------------------------

/// KH1's arc fields (`docs/kh1-physics.md`): time to the apex and the apex's offset (y down).
const ARC_UP_T: usize = 0x3B8;
const ARC_H: usize = 0x3BC;
/// Its sideways velocity (vec4, w its length) and the frame's drop.
const VEL: usize = 0x3C0;
const DROP: usize = 0x3C4;
/// KH1's movement state (2: in the air).
const STATE: usize = 0x70;
/// A flight is let go after this many frames whatever happens (a stuck enemy).
const MAX_FRAMES: u32 = 600;

/// How an enemy KH2 knocked into the air moves.
#[derive(Clone, Copy, Debug)]
enum Fly {
    /// KH2's blow (`character_core::reaction::Blow`): one 30 Hz tick every second 60 Hz frame,
    /// half of it a frame (as KH2's controller and `character_core::npc` run it).
    Blow { blow: character_core::reaction::Blow, step: [f32; 3], phase: u32 },
    /// KH2's air flinch (Steam KH2 `0x1403D4800`): a hop of 50 on KH2's jump curve and gravity
    /// (`character_core::Arc`), drifting at a constant `vel` (per 60 Hz frame) along the push.
    Hop { arc: character_core::Arc, vel: [f32; 2] },
}

#[derive(Clone, Copy, Debug)]
struct Flight {
    actor: usize,
    fly: Fly,
    /// The arc fields written for it: KH1 writing others (a new reaction of its own) ends ours.
    up_t: f32,
    h: f32,
    frames: u32,
}

static FLIGHTS: std::sync::Mutex<Vec<Flight>> = std::sync::Mutex::new(Vec::new());

/// `combat::launched`: KH1 launched `actor` for a KH2 blow and its arc is KH2's (`up_t`, `h` as
/// written): its flight from here is KH2's blow with these starting speeds (per 60 Hz frame,
/// `vel_h` along KH1's launch direction, `up` up).
pub unsafe fn fly_blow(actor: usize, vel_h: [f32; 2], up: f32, up_t: f32, h: f32) {
    let Some(p) = pose::kh2().map(|k| k.character.reaction.clone()) else { return };
    let blow = character_core::reaction::Blow::new(vel_h, up, &p);
    start(Flight { actor, fly: Fly::Blow { blow, step: [0.0; 3], phase: 0 }, up_t, h, frames: 0 });
}

fn start(f: Flight) {
    if let Ok(mut v) = FLIGHTS.lock() {
        v.retain(|g| g.actor != f.actor);
        if v.len() > 64 {
            v.remove(0);
        }
        v.push(f);
    }
}

/// KH1 gave `actor` a reaction of its own (not KH2's): its flight is KH1's again.
pub fn land_kh1(actor: usize) {
    if let Ok(mut v) = FLIGHTS.lock() {
        v.retain(|g| g.actor != actor);
    }
}

/// After KH1's air state's step (`shared::physics::air_step`, state 2's `0x1402AD510` through
/// KH1's state table, for every actor) on an enemy: KH2's flight replaces the step's displacement
/// (`disp`, y down) while the flight is ours. KH1's mover, collision, landing and get-up then run
/// on it as on KH1's own flight.
pub unsafe fn air_step(actor: usize, disp: *mut [f32; 4]) {
    let Ok(mut v) = FLIGHTS.try_lock() else { return };
    let Some(i) = v.iter().position(|g| g.actor == actor) else { return };
    let g = &mut v[i];
    let ours = rd::<i32>(actor + STATE) == 2 && rd::<f32>(actor + ARC_UP_T) == g.up_t && rd::<f32>(actor + ARC_H) == -g.h && g.frames < MAX_FRAMES;
    if !ours {
        static E: AtomicU32 = AtomicU32::new(0);
        if super::log_on(super::LogCat::Combat) && E.fetch_add(1, Ordering::Relaxed) < 30 {
            log!(Combat, "KH2 flight on {actor:#x} ended after {} frames (state {}, KH1's arc {})", g.frames, rd::<i32>(actor + STATE), if rd::<f32>(actor + ARC_UP_T) == g.up_t { "ours" } else { "its own" });
        }
        v.remove(i);
        return;
    }
    let dt = super::shared::physics::dt();
    let n = (dt.round() as u32).clamp(1, 4);
    let Some(p) = pose::kh2().map(|k| k.character.reaction.clone()) else { return };
    let mut total = [0.0f32; 3];
    let vel_h = match &mut g.fly {
        Fly::Blow { blow, step, phase } => {
            for _ in 0..n {
                if *phase % 2 == 0 {
                    let d = blow.tick(2.0, &p);
                    *step = [d[0] / 2.0, d[1] / 2.0, d[2] / 2.0];
                }
                *phase += 1;
                for k in 0..3 {
                    total[k] += step[k];
                }
            }
            blow.vel_h
        }
        Fly::Hop { arc, vel } => {
            for _ in 0..n {
                let dy = arc.advance(1.0, character_core::KH2_GRAVITY, p.fall_max);
                total[0] += vel[0];
                total[1] += dy;
                total[2] += vel[1];
            }
            *vel
        }
    };
    g.frames += n;
    // (KH1 is y down: KH2's rise is a negative drop.)
    (*disp)[0] = total[0];
    (*disp)[1] = -total[1];
    (*disp)[2] = total[2];
    let s = vel_h[0].hypot(vel_h[1]);
    std::ptr::write_unaligned((actor + VEL) as *mut [f32; 4], [vel_h[0], 0.0, vel_h[1], s]);
    std::ptr::write_unaligned((actor + DROP) as *mut f32, -total[1]);
    static N: AtomicU32 = AtomicU32::new(0);
    if g.frames <= n || g.frames % 10 == 0 {
        if super::log_on(super::LogCat::Combat) && N.fetch_add(1, Ordering::Relaxed) < 60 {
            log!(Combat, "KH2 flight on {actor:#x}: frame {}, moved {:.1} sideways, {:.1} up", g.frames, total[0].hypot(total[2]), total[1]);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// KH2's hit numbers on KH1's hit records.
// ---------------------------------------------------------------------------------------------

/// What KH2's reaction routine reads from a hit's ATKP: its reaction id and two knock-back numbers.
#[derive(Clone, Copy, Debug)]
pub struct Kh2Hit {
    pub reaction: u8,
    pub k1: i16,
    pub k2: i16,
}

/// The KH2 hit being landed now (`combat`'s swing path), for `combat::scale_record`.
static CURRENT: std::sync::Mutex<Option<Kh2Hit>> = std::sync::Mutex::new(None);
/// KH1's hit records made by KH2's hits: (record, hit, when). KH1 reuses its records, so only
/// fresh ones count (its reactions run within a frame or two).
static RECORDS: std::sync::Mutex<Vec<(usize, Kh2Hit, u64)>> = std::sync::Mutex::new(Vec::new());
const FRESH_MS: u64 = 250;

fn now_ms() -> u64 {
    static T0: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    T0.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64
}

pub fn set_current(hit: Option<Kh2Hit>) {
    if let Ok(mut c) = CURRENT.lock() {
        *c = hit;
    }
}

pub fn current() -> Option<Kh2Hit> {
    CURRENT.lock().ok().and_then(|c| *c)
}

/// `combat::scale_record`: KH1's record `rec` was made by this KH2 hit. (t1053) A blow distance left
/// on the record by an earlier hit (`note_blow_flinch`) goes: KH1 reuses its records.
pub fn note_record(rec: usize, hit: Kh2Hit) {
    if let Ok(mut v) = RECORDS.lock() {
        let now = now_ms();
        v.retain(|(r, _, t)| *r != rec && now.saturating_sub(*t) < FRESH_MS);
        v.push((rec, hit, now));
    }
    if let Ok(mut v) = BLOW_FLINCH.lock() {
        v.retain(|(r, _, _)| *r != rec);
    }
}

/// (t1053) KH2 blows `combat::scale_record` sent to KH1's flinch (kind 3): (record, KH2's whole
/// sideways travel for the blow, negative toward Sora; when).
static BLOW_FLINCH: std::sync::Mutex<Vec<(usize, f32, u64)>> = std::sync::Mutex::new(Vec::new());

/// (t1053) `combat::scale_record`: KH1's record `rec` is a KH2 blow sent to KH1's flinch; its push
/// distance is `d` (KH2's blow flown out, `combat::Launch::travel`; negative pulls toward Sora).
pub fn note_blow_flinch(rec: usize, d: f32) {
    if let Ok(mut v) = BLOW_FLINCH.lock() {
        let now = now_ms();
        v.retain(|(r, _, t)| *r != rec && now.saturating_sub(*t) < FRESH_MS);
        if v.len() > 64 {
            v.remove(0);
        }
        v.push((rec, d, now));
    }
}

/// (t1053) The blow distance on a fresh record (`note_blow_flinch`), taken off the list.
fn take_blow_flinch(rec: usize) -> Option<f32> {
    let now = now_ms();
    BLOW_FLINCH.lock().ok().and_then(|mut v| {
        v.retain(|(_, _, t)| now.saturating_sub(*t) < FRESH_MS);
        v.iter().position(|(r, _, _)| *r == rec).map(|i| v.remove(i).1)
    })
}

/// (t1053) Whether `rec` has a fresh blow distance waiting (left on the list).
fn has_blow_flinch(rec: usize) -> bool {
    let now = now_ms();
    BLOW_FLINCH.lock().ok().is_some_and(|v| v.iter().any(|(r, _, t)| *r == rec && now.saturating_sub(*t) < FRESH_MS))
}

fn record(rec: usize) -> Option<Kh2Hit> {
    let now = now_ms();
    RECORDS.lock().ok()?.iter().find(|(r, _, t)| *r == rec && now.saturating_sub(*t) < FRESH_MS).map(|(_, h, _)| *h)
}

/// KH2's push distance for a hit (`character_core::reaction::hit_back_distance`: the ATKP's first
/// knock-back number, sstm's 200 when it has none (32767), halved for reactions 0xC / 0xD; Steam KH2
/// `0x1403CEAB0`), and none for a plain flinch (group 2). `None` for a blow (KH1's fallback for an
/// enemy it can't launch stays KH1's).
fn push_distance(h: &Kh2Hit) -> Option<f32> {
    use character_core::reaction::{category, hit_back_distance, Hit};
    let p = pose::kh2().map(|k| k.character.reaction.clone()).unwrap_or_default();
    match category(h.reaction) {
        4 | 5 => None,
        2 => Some(0.0),
        _ => Some(hit_back_distance(&Hit { reaction: h.reaction, knockback: (h.k1, h.k2), dir: [0.0, 1.0], part: 0 }, &p)),
    }
}

// ---------------------------------------------------------------------------------------------
// The flinch push: KH2's distance and curve in KH1's enemy push.
// ---------------------------------------------------------------------------------------------

/// KH1's enemy flinch push (Steam `0x1402B7E60 (actor, record)`, Checked (decomp)): direction the
/// record's `+0x10`, distance btltbl `+0x2C` (80) × the weight, through KH1's push `0x1402C7300`.
/// `mov [rsp+0x18], rbx; push rdi; sub rsp, 0x40` (Checked (exe)).
const FLINCH_PUSH_STEAM: usize = 0x2B7E60;
const FLINCH_PUSH_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x40];
static ORIG_FLINCH_PUSH: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// KH1's push (`0x1402C7300` starts it, `0x1402C7070` steps it from the physics step; Checked
/// (decomp)): speed `+0x3FC` (KH1 starts it at distance × 0.22314355 and keeps × 0.8 a frame),
/// on while `+0x88` bit 0x40.
const PUSH_SPEED: usize = 0x3FC;
/// (t1053) Its direction (`0x1402C7300` copies the vector's x / y / z there; Checked (decomp)).
const PUSH_DIR: usize = 0x3F0;
const PUSH_FLAGS: usize = 0x88;
const PUSH_ON: u32 = 0x40;
const KH1_DECAY: f32 = 0.8;
/// KH2's (Steam KH2 `0x1403D4960` / `0x1403D4570`, Checked (decomp)).
const KH2_START: f32 = 0.1;
const KH2_DECAY: f32 = 0.9;

/// Enemies on KH2's push: (actor, KH2's speed for the next frame, what was last written).
static PUSHES: std::sync::Mutex<Vec<(usize, f32, f32)>> = std::sync::Mutex::new(Vec::new());

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// (t1053) A KH2 blow sent to KH1's flinch (`note_blow_flinch`, the owner's answer (a)): its push is
/// KH2's whole sideways travel for the blow, through the same KH2 curve and weight as a flinch's;
/// negative, the push is turned to point back toward the attacker (KH1's push takes a direction and a
/// distance: the record's direction reversed, flattened). A blow on an enemy the Magnet holds ends the
/// catch first, as KH2's blow does (`magnet::knocked_back`, t866: a held flier back in its own state).
/// (t1055) A plain KH2 flinch with a negative push (its ATKP's first knock-back number below 0) is
/// pulled toward the attacker the same way: KH2's push `0x1403D4960` flips the direction for a
/// negative distance (Checked (decomp), `docs/research/kh2-hit-pipeline.md`). Before, only blows
/// were turned and a plain negative push read as "too slow" and turned KH1's own push off too.
/// Sora's moves with one (Checked (files) `W_EX010_ULTF_R` / `_L`, `00battle.bin`): Final's first
/// ground attack 151, its spinning hits 1394 / 1398 (right) and 1393 / 1397 (left), −25.
unsafe extern "system" fn flinch_push(actor: usize, rec: usize) {
    let f: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_FLINCH_PUSH.load(Ordering::Acquire));
    if actor > 0x10000 && has_blow_flinch(rec) && super::magnet::holds(actor) {
        super::magnet::knocked_back(actor);
    }
    f(actor, rec);
    let Some(hit) = record(rec) else {
        drop_push(actor);
        return;
    };
    let blow = take_blow_flinch(rec);
    let Some(d) = blow.or_else(|| push_distance(&hit)) else { return };
    // KH1's weight: none means KH1 doesn't move this enemy (big and fixed ones): KH1's own stays.
    let w = kh1_weight(actor);
    if w == Some(0.0) {
        return;
    }
    let d = if weight() { d * w.unwrap_or(1.0) } else { d };
    // (t1053) A pull: KH1's push direction reversed (flattened), its distance as a length; (t1055)
    // for a plain flinch's negative push as for a blow's.
    let pull = d < 0.0;
    let d = if pull {
        let r: [f32; 4] = rd(rec + 0x10);
        let n = r[0].hypot(r[2]);
        if n > 1e-4 {
            wr(actor + PUSH_DIR, [-r[0] / n, 0.0, -r[2] / n]);
        }
        -d
    } else {
        d
    };
    let v = d * KH2_START;
    wr(actor + PUSH_SPEED, v);
    let flags = rd::<u32>(actor + PUSH_FLAGS);
    wr(actor + PUSH_FLAGS, if v >= 1.0 { flags | PUSH_ON } else { flags & !PUSH_ON });
    if let Ok(mut p) = PUSHES.lock() {
        p.retain(|(a, _, _)| *a != actor);
        if v >= 1.0 {
            if p.len() > 64 {
                p.remove(0);
            }
            p.push((actor, v, v));
        }
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if super::log_on(super::LogCat::Combat) && N.fetch_add(1, Ordering::Relaxed) < 40 {
        log!(Combat, "KH2's push on {actor:#x} (reaction {}, knock-back {} / {}{}): {d:.0}{} (weight {:?}{}), {v:.1} a frame, x{KH2_DECAY} a frame", hit.reaction, hit.k1, hit.k2, blow.map_or(String::new(), |b| format!(", a KH2 blow as KH1's flinch: its sideways travel {:.1}{}", b.abs(), if b < 0.0 { " toward Sora" } else { " away" })), if pull { " pulled toward Sora" } else { "" }, w, if weight() { "" } else { ", off" });
    }
}

fn drop_push(actor: usize) {
    if let Ok(mut p) = PUSHES.lock() {
        p.retain(|(a, _, _)| *a != actor);
    }
}

/// Before KH1's physics step for an enemy (`takeover::physics`): KH2's push speed for this frame
/// in KH1's push (which moves it by that × the frame step, with its collision), KH2's decay for
/// the next. Dropped when KH1 started a push of its own meanwhile (its field isn't what KH1's push
/// leaves of ours).
pub unsafe fn push_frame(actor: usize) {
    let Ok(mut p) = PUSHES.try_lock() else { return };
    let Some(i) = p.iter().position(|(a, _, _)| *a == actor) else { return };
    let (_, v, written) = p[i];
    let field: f32 = rd(actor + PUSH_SPEED);
    let dt = super::shared::physics::dt();
    let kh1_left = written * KH1_DECAY.powf(dt);
    let ours = (field - written).abs() < 1e-3 || (field - kh1_left).abs() < 1e-3 * written.max(1.0);
    if !ours {
        p.remove(i);
        return;
    }
    if v < 1.0 {
        wr(actor + PUSH_SPEED, 0.0f32);
        wr(actor + PUSH_FLAGS, rd::<u32>(actor + PUSH_FLAGS) & !PUSH_ON);
        p.remove(i);
        return;
    }
    wr(actor + PUSH_SPEED, v);
    wr(actor + PUSH_FLAGS, rd::<u32>(actor + PUSH_FLAGS) | PUSH_ON);
    p[i] = (actor, v * KH2_DECAY.powf(dt), v);
}

// ---------------------------------------------------------------------------------------------
// The air flinch: KH2's hop and drift in KH1's.
// ---------------------------------------------------------------------------------------------

/// KH1's enemy air flinch (Steam `0x1402B8B90 (actor, record, front)`, the third a float in XMM2:
/// its sign picks motion 0x4E / 0x4F; Checked (decomp)): a hop of 20 (`0x1402AF960`) and
/// `+0x3C0` = the record's direction × btltbl `+0x38` (4) × the weight. `mov [rsp+0x18], rbx;
/// mov [rsp+0x20], rsi; push rdi; sub rsp, 0x60` (Checked (exe)).
const AIR_FLINCH_STEAM: usize = 0x2B8B90;
const AIR_FLINCH_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x18, 0x48, 0x89, 0x74, 0x24, 0x20, 0x57, 0x48, 0x83, 0xEC, 0x60];
static ORIG_AIR_FLINCH: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// KH2's air flinch hop (Steam KH2 `0x1403D4800`: 50.0, Checked (decomp)).
const KH2_AIR_HOP: f32 = 50.0;
const ARC_T: usize = 0x3B0;
const ARC_Y0: usize = 0x3B4;

/// (t1053) A KH2 blow sent to KH1's flinch (`note_blow_flinch`) on an enemy in the air: this air
/// flinch, KH2's drift rule with the blow's whole sideways travel as its distance (negative: the
/// drift points back toward the attacker). KH2's air flinch's own hop stays, as for any flinch.
/// (t1055) A plain flinch's negative push drifts toward the attacker too: its sign carries into the
/// drift, as KH2's `0x1403D4800` takes the signed distance (Checked (decomp)).
unsafe extern "system" fn air_flinch(actor: usize, rec: usize, front: f32) {
    let f: unsafe extern "system" fn(usize, usize, f32) = std::mem::transmute(ORIG_AIR_FLINCH.load(Ordering::Acquire));
    f(actor, rec, front);
    let Some(hit) = record(rec) else {
        land_kh1(actor);
        return;
    };
    let blow = take_blow_flinch(rec);
    let Some(d) = blow.or_else(|| push_distance(&hit)) else {
        land_kh1(actor);
        return;
    };
    let w = kh1_weight(actor);
    // (KH1 never moves an enemy of weight 0: its own stays.)
    if w == Some(0.0) || rd::<i32>(actor + STATE) != 2 {
        land_kh1(actor);
        return;
    }
    let to_jump = pose::kh2().map_or(0.048, |k| k.character.reaction.hit_back_to_jump);
    let speed = d * to_jump * if weight() { w.unwrap_or(1.0) } else { 1.0 };
    let r: [f32; 4] = rd(rec + 0x10);
    let n = r[0].hypot(r[2]);
    let dir = if n > 1e-4 { [r[0] / n, r[2] / n] } else { [0.0, 0.0] };
    let vel = [dir[0] * speed, dir[1] * speed];
    let arc = character_core::Arc::new(KH2_AIR_HOP, character_core::KH2_GRAVITY);
    // KH1's arc fields as KH2's hop (its apex time and height, from where it is), so KH1's own
    // timing off them (its apex) follows KH2's.
    let y: f32 = rd(actor + 0x14);
    wr(actor + ARC_T, 0.0f32);
    wr(actor + ARC_Y0, y);
    wr(actor + ARC_UP_T, arc.uptime);
    wr(actor + ARC_H, -KH2_AIR_HOP);
    wr(actor + VEL, [vel[0], 0.0, vel[1], speed]);
    start(Flight { actor, fly: Fly::Hop { arc, vel }, up_t: arc.uptime, h: KH2_AIR_HOP, frames: 0 });
    static N: AtomicU32 = AtomicU32::new(0);
    if super::log_on(super::LogCat::Combat) && N.fetch_add(1, Ordering::Relaxed) < 40 {
        log!(Combat, "KH2's air flinch on {actor:#x} (reaction {}, knock-back {} / {}{}): hop {KH2_AIR_HOP:.0}, drift {speed:.1} a frame (push {d:.0}, weight {w:?}{})", hit.reaction, hit.k1, hit.k2, if blow.is_some() { ", a KH2 blow as KH1's flinch: its sideways travel" } else { "" }, if weight() { "" } else { ", off" });
    }
}

pub unsafe fn install() {
    if super::addr::Build::current() != super::addr::Build::Steam {
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    super::hook::install(base + AIR_FLINCH_STEAM, &AIR_FLINCH_PROLOGUE, air_flinch as *const () as usize, &ORIG_AIR_FLINCH, "enemy air flinch (KH2's hop and drift)");
    super::hook::install(base + FLINCH_PUSH_STEAM, &FLINCH_PUSH_PROLOGUE, flinch_push as *const () as usize, &ORIG_FLINCH_PUSH, "enemy flinch push (KH2's push)");
}
