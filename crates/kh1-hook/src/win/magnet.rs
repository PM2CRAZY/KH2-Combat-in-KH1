//! KH2's Magnet catch (ATKP reaction 5: Magnet Burst's body hits) as a KH1 movement state of its
//! own (testing branch, t862; the owner, chat: make "Magnet Burst's pull" native, answers 1 (a),
//! 2 (a), 3 (a), 4 (a)). `docs/kh1-hit-reactions-native.md` §17.
//!
//! Before: an enemy the Magnet window hit went into a list of ours (`bodyhits`), and every frame
//! `takeover::kh2_hits` worked KH2's suction out for it and added it into KH1's one-frame push
//! (`+0x120`) while the enemy's own KH1 state (its gravity, its motion, its think) went on under it.
//! Now the catch is a KH1 state: KH1's own physics step steps it, KH1's mover and walls move it,
//! KH1's state change ends it. None of our code runs while no enemy is held.
//!
//! KH2 (Steam KH2 exe, Checked (decomp)):
//! - the reaction dispatch (`0x1403A8DE0` / `0x1403D2FE0`, case 5; `0x1403D4510`) calls the catch
//!   `0x14040FBF0 (enemy, magnet attack, dot(forward, hit direction) < 0)` on every hit;
//! - the catch: the attack kept on the enemy (`+0xCF8`), the orbit radius by KH2's random number
//!   between the ATKP's `+0xC` and `+0xE`, the swirl its `+0x10`, motion 20 (hit from the front) or
//!   21 (from behind) with no blend, the enemy turned to the attack (`0x14040FA50`; 21 turned away),
//!   action 0xE (0xF for some enemies, by a flag of their object: the ones that don't fall);
//! - its step `0x14040F8E0` (both actions' slot): toward the attack's place along the 3D line at
//!   `(distance - radius) × 0.1`, at most 20 either way, plus the line turned a quarter about the
//!   vertical, flattened, × the swirl; no gravity; returned × 2 (Assumption: KH2's 30 Hz tick, so
//!   the same per 60 Hz frame);
//! - every frame while the attack lasts it is turned to the attack again (`0x14040FB10` /
//!   `0x14040FB60` → `0x14040FA50`); with the attack gone, 0xE goes to action 5 and 0xF does a
//!   recovery of its own (motion 4, blend 8).
//!
//! KH1 (Steam KH1 exe, Checked (decomp)):
//! - the state table `0x142DB3840` (59 entries, `0x1402B1A40` clears 0x3B), filled by
//!   `0x1402B1B80`; ours are `STATE` (0x2B) and, since t863, `FLIER_STATE` (0x2C, flags 2, for an
//!   enemy caught from a class-2 state: KH1's hit pass then treats it as it treats its fliers);
//!   neither is registered nor switched on by number (as Quick Run's 0x2A, `quickrun`); written
//!   each Sora physics frame into an empty slot or ours;
//! - flags 1: collision class 1 (`0x1402B1B10`: KH1's air states' class, so its mover collides it
//!   as air with no snap to the floor, its push runs in 3D, its hit pass gives air reactions); no
//!   platform carry;
//! - the physics step `0x14029A3C0` (for every actor under physics, `0x140292010`) calls our step for
//!   the displacement, moves it through KH1's mover (`0x1402998A0`), then our transition;
//! - the state change `0x1402B1960` (exit of the old state, enter of the new);
//! - the enemy hit pass `0x1402B8E10` gives a kind 3 hit on an air-class state KH1's air flinch
//!   `0x1402B8B90` (requests 0x4E / 0x4F with the action flag, a hop of 20 that puts it in state 2,
//!   the fall 6 queued after); kind 1 it gives no reaction (no case for it), though `0x1402B7B90`
//!   still deals the damage and drops a physical hit's prizes for it (kinds 1..3 alike). So the
//!   record of a Magnet hit gets kind 1 here: KH1 adds no reaction of its own to KH2's catch.
//!
//! Choices (the owner's answers): (1 a) the enemy plays its own KH1 air-hit motion (0x4E front /
//! 0x4F behind, as KH1's air flinch picks it), asked again whenever it ends or another replaces it;
//! (2 a) the radius rolled with KH1's random number (`kh1picker::random`); (3 a) the let-go follows
//! the Knock-back Numbers row: KH1 → KH1's fall from rest (`0x1402AF8F0 (actor, 0)`), KH2 or Scaled →
//! that fall on KH2's blow with no speed (`knockback::fly_blow`, KH2's let-go in the OG rewrite's
//! `npc`); (4 a) an enemy caught in an air-class-2 state of KH1's (`0x1402B1AE0`: the states KH1
//! never launches, its fliers; Assumption that those are KH2's 0xF enemies) goes back into that state.

use super::addr::{Build, Rva};
use super::{log, takeover};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::Mutex;

/// Our states' ids in KH1's state table: one for enemies caught from KH1's ground and air states,
/// one for those caught from a class-2 state (its fliers, t863).
pub const STATE: i32 = 0x2B;
pub const FLIER_STATE: i32 = 0x2C;
/// Collision class 1 (air) / class 2 (KH1's fliers: its hit pass knocks them back in place, never
/// launches them, `0x1402B77D0`), nothing else.
const FLAGS: u32 = 1;
const FLIER_FLAGS: u32 = 2;
/// An actor's action flag (`+0` bit 4: a motion asked with `ACTION` plays).
const IN_ACTION: u32 = 4;

const TABLE_RVA: Rva = Rva(0x2DB4240);
const ENTRY: usize = 0x40;
/// KH1's state change, motion request and queue, bone point (mapped; used as `quickrun` and
/// `bodyhits` use them).
const SET_STATE_RVA: Rva = Rva(0x2AF7D0);
const SET_MOTION_RVA: Rva = Rva(0x29D7F0);
const QUEUE_MOTION_RVA: Rva = Rva(0x29D8A0);
const BONE_POINT_RVA: Rva = Rva(0x29C990);
/// KH1's fall from rest `(actor, time to apex)`: state 2 with KH1's arc from where it is (Steam
/// `0x1402AF8F0`), its first bytes `push rbx; sub rsp, 0x20; movss [rcx+0x3B8], xmm1` (Checked (exe)).
const FALL_STEAM: usize = 0x2AF8F0;
const FALL_BYTES: [u8; 10] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0xF3, 0x0F, 0x11, 0x89];
/// KH1's "under physics" test (Steam `0x140292010`): the physics step runs the state table only
/// for these (Checked (decomp + exe)).
const PHYSICS_STEAM: usize = 0x292010;
const PHYSICS_BYTES: [u8; 10] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0x8B];
/// KH1's facing set `(actor, angle)` (Steam `0x140297E10`; leaves a held facing, `+0x374` bit
/// `0x1000`), Checked (decomp + exe).
const FACE_STEAM: usize = 0x297E10;
const FACE_BYTES: [u8; 10] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x30, 0xF7, 0x81, 0x74, 0x03];
/// KH1's body centre point (`0x14029EB20`'s 0x8003: the first body part's centre).
const BODY_POINT: u32 = 0x8003;
/// KH1's air flinch requests (`0x1402B8B90`: front, behind), with the action flag.
const HURT_FRONT: u32 = 0x4E;
const HURT_BEHIND: u32 = 0x4F;
const ACTION: u32 = 0x0100_0000;
/// What KH1 queues after its air flinch (the fall, blend 8) and its idle.
const FALL_AFTER: u32 = 0x4000_0006;
const IDLE_AFTER: u32 = 0x4000_0000;
const AFTER_BLEND: f32 = 8.0;
/// KH2's suction (`0x14040F8E0`).
const PULL_RATE: f32 = 0.1;
const PULL_MAX: f32 = 20.0;
/// KH1's momentum field and push (cleared by the catch, as KH2's catch drops a push and a hop).
const VEL: usize = 0x3C0;
const PUSH_FLAGS: usize = 0x88;
const PUSH_ON: u32 = 0x40;
const PUSH_SPEED: usize = 0x3FC;
/// A record's reaction kind (`+0xC` low 5 bits), and the one KH1's enemy hit pass takes no
/// reaction for.
const KIND_MASK: u32 = 0x1F;
const KIND_NONE: u32 = 1;
/// KH1 takes no reaction on an actor with these flags (`0x1402B7B90`).
const NO_REACTION: u32 = 0x0100_0088;
const HELD_MAX: usize = 16;

/// One enemy held.
#[derive(Clone, Copy, Debug)]
struct Held {
    actor: usize,
    /// The Magnet's KH1 attack object (ours, `bodyhits`).
    attack: usize,
    radius: f32,
    swirl: f32,
    /// Hit from behind (KH2's 21, KH1's 0x4F): turned away from the attack.
    behind: bool,
    /// Its state before the catch (the let-go of a flier goes back to it).
    before: i32,
    frames: u32,
    /// (t864, Magnet Debug log) its place at our last step and the move we expected KH1 to give
    /// it from there.
    last: Option<([f32; 3], [f32; 3])>,
}

static HELD: Mutex<Vec<Held>> = Mutex::new(Vec::new());
/// KH1's functions are the ones we know: 0 not checked, 1 yes, 2 no.
static CHECKED: AtomicU8 = AtomicU8::new(0);
static REFUSED: AtomicBool = AtomicBool::new(false);

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
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 120 {
        log!(Combat, "Magnet (KH1 state of ours): {}", s());
    }
}

/// Steam, and KH1's functions we call have their known first bytes.
unsafe fn ready(base: usize) -> bool {
    if Build::current() != Build::Steam || base == 0 {
        return false;
    }
    match CHECKED.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => {
            let same = |at: usize, b: &[u8]| std::slice::from_raw_parts((base + at) as *const u8, b.len()) == b;
            let ok = same(FALL_STEAM, &FALL_BYTES) && same(PHYSICS_STEAM, &PHYSICS_BYTES) && same(FACE_STEAM, &FACE_BYTES);
            CHECKED.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
            if !ok {
                log!(Combat, "Magnet: KH1's fall, physics test or facing set has unexpected bytes: Magnet hits don't catch");
            }
            ok
        }
    }
}

/// Puts our state in KH1's state table (each Sora physics frame, as `quickrun::install`: KH1
/// fills the table itself when it sets its states up). Only into an empty slot or ours.
pub unsafe fn install(base: usize) {
    if !ready(base) {
        return;
    }
    for (state, flags) in [(STATE, FLAGS), (FLIER_STATE, FLIER_FLAGS)] {
        install_one(base, state, flags);
    }
}

unsafe fn install_one(base: usize, state: i32, flags: u32) {
    let e = base + TABLE_RVA + state as usize * ENTRY;
    let step_fn = step as *const () as usize;
    let trans_fn = transition as *const () as usize;
    let words: [usize; 8] = rd(e);
    let ours = words[4] == step_fn && words[5] == trans_fn;
    if !(ours || words.iter().all(|&w| w == 0)) {
        if !REFUSED.swap(true, Ordering::Relaxed) {
            log!(Combat, "Magnet: KH1's state {state:#x} is in use ({words:#x?}): not installed");
        }
        return;
    }
    if ours && rd::<u32>(e) == flags {
        return;
    }
    wr(e, [flags as usize, 0, 0, 0, step_fn, trans_fn, 0, 0]);
    note(|| format!("state {state:#x} in KH1's state table (flags {flags:#x}, step {step_fn:#x}, transition {trans_fn:#x})"));
}

unsafe fn installed(base: usize) -> bool {
    [STATE, FLIER_STATE].iter().all(|&s| rd::<usize>(base + TABLE_RVA + s as usize * ENTRY + 0x20) == step as *const () as usize)
}

/// One of our held states.
fn held_state(state: i32) -> bool {
    state == STATE || state == FLIER_STATE
}

/// KH1's body centre of `actor` (KH1 world, y down).
unsafe fn body_centre(base: usize, actor: usize) -> Option<[f32; 3]> {
    let point: unsafe extern "system" fn(usize, u32, f32, *mut [f32; 4]) = std::mem::transmute(base + BONE_POINT_RVA);
    let mut out = [0.0f32; 4];
    point(actor, BODY_POINT, 1.0, &mut out);
    let c = [out[0], out[1], out[2]];
    c.iter().all(|v| v.is_finite()).then_some(c)
}

/// KH1's state class 2 (`0x1402B1AE0`: flags `& 3 == 2`, or state 0x17): the states KH1's hit
/// pass never launches.
unsafe fn flier_state(base: usize, state: i32) -> bool {
    if !(0..0x3B).contains(&state) {
        return false;
    }
    state == 0x17 || rd::<u32>(base + TABLE_RVA + state as usize * ENTRY) & 3 == 2
}

/// KH2's turn to the attack (`0x14040FA50`) through KH1's facing set; `d` the flat line from the
/// enemy to the attack (KH1 x / z).
unsafe fn face(base: usize, actor: usize, d: [f32; 2], behind: bool) {
    if d[0].hypot(d[1]) < 1e-3 {
        return;
    }
    let d = if behind { [-d[0], -d[1]] } else { d };
    let a = d[0].atan2(d[1]);
    let set: unsafe extern "system" fn(usize, f32) = std::mem::transmute(base + FACE_STEAM);
    set(actor, a);
    wr(actor + 0x150, a);
}

/// The held motion is playing or asked for (KH1 applies a request in its animation update:
/// `+0x178` pending, `+0x17C` the request; `+0x164` the one playing).
unsafe fn held_motion_on(actor: usize, request: u32) -> bool {
    if rd::<u32>(actor + 0x164) & 0xFFFF == request {
        return true;
    }
    rd::<u32>(actor + 0x178) != 0 && rd::<u32>(actor + 0x17C) & 0xFFFF == request
}

fn request_of(behind: bool) -> u32 {
    if behind {
        HURT_BEHIND
    } else {
        HURT_FRONT
    }
}

/// `bodyhits::hit`: KH1's pass made hit record `rec` with the Magnet attack object `attack` (ATKP
/// knock-back values `k1` / `k2`, swirl) on `target`: KH2's catch (`0x14040FBF0`), on every hit.
pub unsafe fn catch(attack: usize, target: usize, rec: usize, k1: i16, k2: i16, swirl: i16) {
    let base = takeover::game_base();
    if target < 0x10000 || !ready(base) {
        return;
    }
    if !installed(base) {
        install(base);
        if !installed(base) {
            return;
        }
    }
    // (One KH1 takes no reaction on, one KH1's physics step doesn't step, the big and immovable:
    // not caught, as before.)
    if rd::<u32>(target) & NO_REACTION != 0 {
        return;
    }
    let physics: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(base + PHYSICS_STEAM);
    if physics(target) & 0xFF == 0 {
        note(|| format!("actor {target:#x} isn't under KH1's physics: not caught"));
        return;
    }
    let sora = takeover::sora_actor();
    if super::combat::immovable(sora, target) {
        return;
    }
    let Ok(Some(c)) = super::bodyhits::magnet_centre(attack) else { return };
    let Some(b) = body_centre(base, target) else { return };
    // Hit from behind: KH2's `dot(forward, hit direction) >= 0` (motion 21), the hit's direction
    // KH1's record's (`+0x10`, from the attack to the target), as KH1's air flinch reads it.
    let f: [f32; 4] = rd(target + 0x30);
    let hd: [f32; 4] = if rec > 0x10000 { rd(rec + 0x10) } else { [b[0] - c[0], 0.0, b[2] - c[2], 0.0] };
    let behind = f[0] * hd[0] + f[2] * hd[2] >= 0.0;
    // KH1 adds no reaction of its own: KH2's catch is this hit's reaction. (A kind of 0, no hit
    // prize either, stays 0.)
    if rec > 0x10000 {
        let w = rd::<u32>(rec + 0xC);
        if w & KIND_MASK != 0 {
            wr(rec + 0xC, (w & !KIND_MASK) | KIND_NONE);
        }
    }
    let r = super::kh1picker::random().unwrap_or(0.5).clamp(0.0, 1.0);
    let (a, z) = (k1 as f32, k2 as f32);
    let radius = a + (z - a) * r;
    let now = rd::<i32>(target + 0x70);
    let Ok(mut g) = HELD.try_lock() else { return };
    // (Holds KH1 ended meanwhile, by a state change of its own, forgotten.)
    g.retain(|h| held_state(rd::<i32>(h.actor + 0x70)));
    // Caught again while held (KH2 catches on every hit): its state from before the first catch.
    let before = g.iter().find(|h| h.actor == target).map_or(now, |h| h.before);
    // (One of ours with no record of ours: KH1's air state stands for what it was.)
    let before = if held_state(before) { 2 } else { before };
    // (t863) A flier (KH1's class 2) is held in our class-2 state, so KH1's hit pass treats it as
    // KH1 treats its fliers (Magnet Burst's 1901 / 1903 knock-backs: KH1's knock-back in place,
    // `0x1402B8CC0`, not its airborne one into state 2 with gravity, `0x1402B77D0`).
    let into = if flier_state(base, before) { FLIER_STATE } else { STATE };
    let set_state: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + SET_STATE_RVA);
    if now != into {
        set_state(target, into);
        if rd::<i32>(target + 0x70) != into {
            return;
        }
    }
    g.retain(|h| h.actor != target);
    if g.len() >= HELD_MAX {
        g.remove(0);
    }
    let held = Held { actor: target, attack, radius, swirl: swirl as f32, behind, before, frames: 0, last: None };
    g.push(held);
    drop(g);
    // Its KH1 air-hit motion from its start (KH2's 20 / 21 with no blend), turned to the attack, no
    // momentum, push or KH2 flight of before.
    let set: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
    set(target, request_of(behind) | ACTION, 0.0);
    face(base, target, [c[0] - b[0], c[2] - b[2]], behind);
    wr(target + VEL, [0.0f32; 4]);
    wr(target + PUSH_FLAGS, rd::<u32>(target + PUSH_FLAGS) & !PUSH_ON);
    wr(target + PUSH_SPEED, 0.0f32);
    super::knockback::land_kh1(target);
    dbg(|| format!("caught {target:#x} into state {into:#x} (state {before:#x} before, now {now:#x}), orbit {radius:.0}, swirl {swirl}, {}", if behind { "behind" } else { "front" }));
    note(|| format!(
        "actor {target:#x} caught by {attack:#x} into state {into:#x} (from {}, state {before:#x} before): orbit {radius:.0} (between {k1} and {k2}), swirl {swirl}, KH1 request {:#x}",
        if behind { "behind" } else { "the front" },
        request_of(behind)
    ));
}

fn get(actor: usize) -> Option<Held> {
    HELD.try_lock().ok()?.iter().find(|h| h.actor == actor).copied()
}

/// Our state's step (KH1's physics step calls it with the displacement to fill): KH2's suction
/// toward the Magnet's place, no gravity; KH1's mover moves it after.
unsafe extern "system" fn step(actor: usize, disp: *mut [f32; 4]) -> usize {
    if disp.is_null() {
        return 0;
    }
    *disp = [0.0; 4];
    let base = takeover::game_base();
    let Some(h) = get(actor) else { return 0 };
    let c = match super::bodyhits::magnet_centre(h.attack) {
        Ok(Some(c)) => c,
        // (Our table busy this frame: held where it is.)
        Err(()) => return 0,
        Ok(None) => {
            let_go(base, actor, &h);
            return 0;
        }
    };
    let Some(b) = body_centre(base, actor) else { return 0 };
    // KH2's frame (y up): from the body's centre to the attack's place.
    let d = [c[0] - b[0], -(c[1] - b[1]), c[2] - b[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if len > 1e-3 {
        let n = [d[0] / len, d[1] / len, d[2] / len];
        let radial = ((len - h.radius) * PULL_RATE).clamp(-PULL_MAX, PULL_MAX);
        // The line turned a quarter about the vertical, flattened.
        let tl = n[2].hypot(n[0]);
        let t = if tl > 1e-4 { [n[2] / tl, -n[0] / tl] } else { [0.0, 0.0] };
        let v = [n[0] * radial + t[0] * h.swirl, n[1] * radial, n[2] * radial + t[1] * h.swirl];
        let dt = super::shared::physics::dt();
        // (KH1 is y down.)
        *disp = [v[0] * dt, -v[1] * dt, v[2] * dt, 0.0];
    }
    face(base, actor, [c[0] - b[0], c[2] - b[2]], h.behind);
    let request = request_of(h.behind);
    // (Asked again only once nothing of KH1's plays with the action flag: a reaction KH1 gives a
    // held flier, its knock-back in place, plays out first, t863.)
    if !held_motion_on(actor, request) && rd::<u32>(actor) & IN_ACTION == 0 {
        let set: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
        set(actor, request | ACTION, 0.0);
    }
    wr(actor + VEL, [0.0f32; 4]);
    let frames = h.frames + 1;
    let pos: [f32; 3] = rd(actor + POS);
    let expect = if super::log_on(super::LogCat::Magnet) { debug_frame(actor, &h, *disp, len, frames, pos) } else { [0.0; 3] };
    if let Ok(mut g) = HELD.try_lock() {
        if let Some(x) = g.iter_mut().find(|x| x.actor == actor) {
            x.frames = frames;
            x.last = Some((pos, expect));
        }
    }
    if frames == 1 || frames % 30 == 0 {
        note(|| format!("actor {actor:#x} held {frames} frames, {len:.0} from the Magnet (orbit {:.0})", h.radius));
    }
    0
}

/// Our state's transition (after KH1's mover): ours while held; otherwise KH1's own default.
unsafe extern "system" fn transition(actor: usize, airborne: i32) -> u32 {
    let now = rd::<i32>(actor + 0x70);
    if held_state(now) && get(actor).is_some() {
        return now as u32;
    }
    if airborne != 0 {
        2
    } else {
        0
    }
}

/// The Magnet is gone: KH2's let-go through KH1's states (the owner's answers 3 (a) and 4 (a)).
unsafe fn let_go(base: usize, actor: usize, h: &Held) {
    if let Ok(mut g) = HELD.try_lock() {
        g.retain(|x| x.actor != actor);
    }
    dbg(|| format!("let go {actor:#x} after {} frames (state before the catch {:#x})", h.frames, h.before));
    wr(actor + VEL, [0.0f32; 4]);
    let queue: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + QUEUE_MOTION_RVA);
    if flier_state(base, h.before) {
        let set_state: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + SET_STATE_RVA);
        set_state(actor, h.before);
        queue(actor, IDLE_AFTER, AFTER_BLEND);
        note(|| format!("actor {actor:#x} let go after {} frames: back in its state {:#x} (KH1's class 2)", h.frames, h.before));
        return;
    }
    let fall: unsafe extern "system" fn(usize, f32) -> u64 = std::mem::transmute(base + FALL_STEAM);
    fall(actor, 0.0);
    wr(actor + VEL, [0.0f32; 4]);
    let kh2 = super::hitreact::numbers() != super::hitreact::Numbers::Kh1;
    if kh2 {
        // (KH1's fall from rest: no time to the apex, no height; KH2's blow with no speed on it.)
        super::knockback::fly_blow(actor, [0.0, 0.0], 0.0, rd::<f32>(actor + 0x3B8), -rd::<f32>(actor + 0x3BC));
    }
    queue(actor, FALL_AFTER, AFTER_BLEND);
    note(|| format!("actor {actor:#x} let go after {} frames: {} fall from rest (Knock-back Numbers {:?})", h.frames, if kh2 { "KH2's" } else { "KH1's" }, super::hitreact::numbers()));
}

// ---------------------------------------------------------------------------------------------
// The Magnet Debug log (t864; the owner, chat: "make it and add it as a toggle in the logs
// section on debug"): per held enemy, per frame, everything KH1's physics step adds to its move.
// Off (the Logs page's Magnet Debug row, ini `log_magnet`): one atomic load a held frame.
// ---------------------------------------------------------------------------------------------

/// An actor's place (KH1 world, y down): what KH1's mover adds the displacement to.
const POS: usize = 0x10;
/// What KH1's physics step `0x14029A3C0` adds after its state's step (Checked (decomp)): the push
/// (`+0x88` bit 0x40: `0x1402C7070`, direction `+0x3F0` × frame scale × speed `+0x3FC`, its
/// height 0), the vector at `+0x200` (while `+0x160` bit 4), the one-frame push `+0x120`
/// (cleared at the step's end).
const PUSH_DIR: usize = 0x3F0;
const EXTRA_FLAGS: usize = 0x160;
const EXTRA_ON: u32 = 4;
const EXTRA: usize = 0x200;
const ONE_FRAME: usize = 0x120;
/// KH1's current motion (`+0x164`).
const MOTION: usize = 0x164;
/// At most this many Magnet Debug lines a session.
const DEBUG_LINES: u32 = 20_000;

fn dbg(s: impl FnOnce() -> String) {
    if !super::log_on(super::LogCat::Magnet) {
        return;
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < DEBUG_LINES {
        log!(Magnet, "Magnet debug: {}", s());
    }
}

fn mag(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn v3(v: [f32; 3]) -> String {
    format!("({:.1}, {:.1}, {:.1})", v[0], v[1], v[2])
}

/// One held frame's line: our displacement, what KH1's physics step will add to it this frame,
/// and how far it really moved since our last step (against what we expected then). Returns
/// this frame's expected move.
unsafe fn debug_frame(actor: usize, h: &Held, ours: [f32; 4], dist: f32, frames: u32, pos: [f32; 3]) -> [f32; 3] {
    let dt = super::shared::physics::dt();
    let ours = [ours[0], ours[1], ours[2]];
    let push_on = rd::<u32>(actor + PUSH_FLAGS) & PUSH_ON != 0;
    let speed: f32 = rd(actor + PUSH_SPEED);
    let dir: [f32; 3] = rd(actor + PUSH_DIR);
    let push = [dir[0] * speed * dt, 0.0, dir[2] * speed * dt];
    let extra_on = rd::<u32>(actor + EXTRA_FLAGS) & EXTRA_ON != 0;
    let extra: [f32; 3] = rd(actor + EXTRA);
    let one: [f32; 3] = rd(actor + ONE_FRAME);
    let mut sum = ours;
    for k in 0..3 {
        let p = if push_on { push[k] } else { 0.0 };
        let x = if extra_on { extra[k] } else { 0.0 };
        sum[k] += p + x + one[k];
    }
    let state = rd::<i32>(actor + 0x70);
    let motion = rd::<u32>(actor + MOTION);
    let action = rd::<u32>(actor) & IN_ACTION != 0;
    let last = match h.last {
        Some((p, e)) => {
            let moved = [pos[0] - p[0], pos[1] - p[1], pos[2] - p[2]];
            format!("last frame expected {} {:.1}, moved {} {:.1}", v3(e), mag(e), v3(moved), mag(moved))
        }
        None => "last frame -".into(),
    };
    dbg(|| format!(
        "{actor:#x} frame {frames} state {state:#x} motion {motion:#x}{} at {} {dist:.0} from the Magnet (orbit {:.0}, swirl {:.0}) dt {dt:.2} | ours {} {:.1} | push {} {} {:.1} | +0x200 {} {} {:.1} | +0x120 {} {:.1} | sum {:.1} | {last}",
        if action { " (action)" } else { "" },
        v3(pos),
        h.radius,
        h.swirl,
        v3(ours),
        mag(ours),
        if push_on { "on" } else { "off" },
        v3(push),
        mag(push),
        if extra_on { "on" } else { "off" },
        v3(extra),
        mag(extra),
        v3(one),
        mag(one),
        mag(sum)
    ));
    sum
}

/// (t866) `combat::knocked_back`, before KH1's knock-back (`0x1402B77D0`) runs on `actor`: KH2's
/// blow ends the catch (`0x1403D2FE0` case 4 plays the blow motion and sets the enemy's blow action
/// through `0x140409D30`, whatever its action was, 0xE / 0xF included; Checked (decomp) Steam KH2).
/// A held flier goes back into its own state first, so KH1's knock-back plays on it as on any of
/// its fliers, with its own travel (t865 dropped that travel while holding it, which slid its model
/// away from its place). A held grounder is left to KH1's knock-back, which takes it out of the
/// hold itself (its airborne branch into state 2, as since t862).
pub unsafe fn knocked_back(actor: usize) {
    let Some(h) = get(actor) else { return };
    if rd::<i32>(actor + 0x70) != FLIER_STATE {
        return;
    }
    let base = takeover::game_base();
    if !ready(base) || !flier_state(base, h.before) {
        return;
    }
    if let Ok(mut g) = HELD.try_lock() {
        g.retain(|x| x.actor != actor);
    }
    let set_state: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + SET_STATE_RVA);
    set_state(actor, h.before);
    dbg(|| format!("blown out of the hold {actor:#x} after {} frames: back in its state {:#x} for KH1's knock-back", h.frames, h.before));
    note(|| format!("actor {actor:#x} blown out after {} frames: back in its state {:#x} (KH1's class 2) before KH1's knock-back, as KH2's blow ends the catch", h.frames, h.before));
}
