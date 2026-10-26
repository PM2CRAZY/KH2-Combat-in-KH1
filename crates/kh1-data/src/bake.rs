//! KH2 motions baked into KH1 motions (tier B, `docs/kh1-anim-slots.md` §1.6 and §3.1): KH1
//! plays the motion itself, with no pose of ours written over its pass.
//!
//! The input is the pose KH1 should show at every frame (KH1's model space, one matrix per KH1
//! bone: KH2's motion retargeted, its hair, hood and keychain on, as `pose.rs` draws it for a
//! tier A slot). The output is a KH1 motion in file layout, built on a template motion (the KH1
//! motion the KH2 one stands in for) retimed to the new length:
//!
//! - **Joint words: plain FK.** KH1's IK-driven root, spine, neck, arms and legs are turned off
//!   for this motion only (joint words are per motion): every bone KH1 doesn't skip gets word 0,
//!   or bit 25 (its local matrix built once, at the motion's start) when nothing on it moves.
//!   Skipped bones (bit 26, all leaves) keep the template's word.
//! - **Channels.** Each bone's local matrix (`parent⁻¹ × bone`) as Euler X→Y→Z and a
//!   translation, the order KH1's FK builds (`0x1401DB430`; `kh2_anim::euler_quat`). A channel
//!   that stays at the bind gets nothing (KH1 resets every channel to the bind when a motion
//!   starts); one that holds another value is a constant channel; the rest are linear keys
//!   (interpolation 1), one per frame, then dropped where a line between their neighbours stays
//!   within `ROT_TOL` / `POS_TOL`. Angles are kept continuous from frame to frame (the Euler
//!   triple nearer the last frame's, then each angle unwrapped), so a line between keys never
//!   turns the long way round.
//! - **Helpers.** The template's helper table and helper channels, retimed. No bone points at a
//!   helper any more.
//! - **Travel** (`travel` given, t365). The last helper's translation (H10 on Sora) is where
//!   KH1 takes a motion's travel from (`docs/kh1-anim-slots.md` §1.6 step 5): with the request's
//!   travel flag (file bit 15, `0x1402A01E0`) KH1 moves the actor by it each frame (`0x14029A3C0`
//!   adds `+0x200`) and pulls the model's placement back by the travel so far (`0x14029F0A0`,
//!   `+0x210`; put back at the motion's end, `0x14029FC40`), and its own motions carry the same
//!   travel in the body (H2). So the template's H10 translation channels
//!   are replaced by `travel` (model space, per frame), and the poses given carry it too.
//! - **Events.** Those given (as `slot::retimed`).

use crate::legs;
use crate::slot::{self, put, u32_at, EventTrack};
use glam::{Mat4, Vec3};
use kh2_formats::anim::Bone;
use kh2_formats::{Error, Result};
use std::f32::consts::{PI, TAU};

/// Joint word bits (`docs/kh1-anim-slots.md` §1.6).
pub const SKIP: u32 = 0x400_0000;
pub const STATIC: u32 = 0x200_0000;
/// The most a dropped key's angle (radians) or translation (units) may be off the line.
pub const ROT_TOL: f32 = 0.001;
pub const POS_TOL: f32 = 0.02;
/// Closer than this to the bind (or across the motion) is the same value.
const SAME_ROT: f32 = 1e-4;
const SAME_POS: f32 = 1e-3;
/// A channel's key count is a byte.
const MAX_KEYS: usize = 255;

/// What a bake made.
#[derive(Clone, Copy, Debug, Default)]
pub struct Report {
    pub keyed_channels: usize,
    pub constant_channels: usize,
    pub keys: usize,
    /// Bones built once at the motion's start (bit 25).
    pub static_bones: usize,
    /// Keys written for the travel helper (H10).
    pub travel_keys: usize,
    /// The worst distance (units) between a joint as KH1 will place it from the channels and
    /// the pose given, over every frame, and its bone.
    pub max_error: f32,
    pub worst_bone: usize,
    /// Legs kept on KH1's own leg IK and ground lift (`legs`), the keys their helpers got, and
    /// the most the hip (KH1 aims it at the ankle) and the thigh or shin KH1 builds turn from the
    /// pose given (radians).
    pub kh1_legs: usize,
    pub leg_keys: usize,
    pub leg_hip_turn: f32,
    pub leg_turn: f32,
}

struct Keyed {
    bone: u16,
    kind: u8,
    keys: Vec<(u32, f32)>,
}

fn parent(skeleton: &[Bone], i: usize) -> Option<usize> {
    let p = skeleton[i].parent;
    (p >= 0 && (p as usize) < skeleton.len() && p as usize != i).then_some(p as usize)
}

/// `a` plus the whole turns that bring it nearest `near`.
fn unwrap(a: f32, near: f32) -> f32 {
    a + ((near - a) / TAU).round() * TAU
}

/// [`continuous`], for `legs`.
pub(crate) fn continuous_euler(e: Vec3, prev: Vec3) -> Vec3 {
    continuous(e, prev)
}

/// The Euler triple for `e`'s rotation nearest `prev` (the other triple of the same rotation,
/// each angle unwrapped).
fn continuous(e: Vec3, prev: Vec3) -> Vec3 {
    let alt = Vec3::new(e.x + PI, PI - e.y, e.z + PI);
    let near = |v: Vec3| Vec3::new(unwrap(v.x, prev.x), unwrap(v.y, prev.y), unwrap(v.z, prev.z));
    let (a, b) = (near(e), near(alt));
    if (a - prev).length_squared() <= (b - prev).length_squared() { a } else { b }
}

/// Keys for one channel's values (one per frame): the first and last, and between them only
/// those a line between the kept neighbours misses by more than `tol`.
fn reduce(v: &[f32], tol: f32) -> Vec<(u32, f32)> {
    let n = v.len();
    let mut out = vec![(0u32, v[0])];
    let fits = |a: usize, b: usize| (a + 1..b).all(|i| (v[a] + (v[b] - v[a]) * ((i - a) as f32 / (b - a) as f32) - v[i]).abs() <= tol);
    let mut a = 0;
    while a + 1 < n {
        let mut b = a + 1;
        while b + 1 < n && fits(a, b + 1) {
            b += 1;
        }
        out.push((b as u32, v[b]));
        a = b;
    }
    out
}

/// One linear key (interpolation 1) in KH1's layout: `(frame*256)<<8 | 1`, the value, no tangents.
fn key(frame: u32, v: f32) -> [u8; 16] {
    let mut k = [0u8; 16];
    k[..4].copy_from_slice(&(((frame as i32 * 256) << 8) | 1).to_le_bytes());
    k[4..8].copy_from_slice(&v.to_le_bytes());
    k
}

/// A keyed channel's value at `frame` (linear between keys, held past the ends).
fn sample(keys: &[(u32, f32)], frame: f32) -> f32 {
    let (first, last) = (keys[0], keys[keys.len() - 1]);
    if frame <= first.0 as f32 {
        return first.1;
    }
    if frame >= last.0 as f32 {
        return last.1;
    }
    let i = keys.partition_point(|k| k.0 as f32 <= frame).saturating_sub(1).min(keys.len() - 2);
    let (a, b) = (keys[i], keys[i + 1]);
    a.1 + (b.1 - a.1) * (frame - a.0 as f32) / (b.0 - a.0) as f32
}

/// The KH1 motion that shows `poses` (one per frame, `0..=frames`, KH1 model space, one matrix
/// per bone of `skeleton`), built on `template` (a KH1 motion in file layout) lasting `frames`,
/// with `events`. `travel` (empty, or one per pose): the motion's travel for H10, already in
/// `poses`. `kh1_legs`: the template's legs stay on KH1's own leg IK and ground lift
/// (`legs`), keyed to the poses' ankles, knees and feet; else plain FK like every other bone.
pub fn baked(template: &[u8], frames: u32, events: &[EventTrack], skeleton: &[Bone], poses: &[Vec<Mat4>], travel: &[Vec3], kh1_legs: bool) -> Result<(Vec<u8>, Report)> {
    let bad = |m: String| Error::Format(format!("bake: {m}"));
    let n = skeleton.len();
    if frames == 0 || frames as usize + 1 > MAX_KEYS {
        return Err(bad(format!("{frames} frames")));
    }
    if poses.len() != frames as usize + 1 || poses.iter().any(|p| p.len() != n) {
        return Err(bad(format!("{} poses for {frames} frames, {n} bones", poses.len())));
    }
    if poses.iter().flatten().any(|m| !m.is_finite()) {
        return Err(bad("a pose is not finite".into()));
    }
    if !travel.is_empty() && (travel.len() != poses.len() || travel.iter().any(|t| !t.is_finite())) {
        return Err(bad(format!("{} travel values for {} poses", travel.len(), poses.len())));
    }
    let r = slot::retimed(template, frames, events)?;
    if u32_at(&r, 0x10)? as usize != n {
        return Err(bad(format!("the template has {} bones, the skeleton {n}", u32_at(&r, 0x10)?)));
    }
    let words_at = u32_at(&r, 0x14)? as usize;
    let words: Vec<u32> = (0..n).map(|i| u32_at(&r, words_at + i * 4)).collect::<Result<_>>()?;

    // Each bone's local rotation (continuous) and translation, per frame.
    let frames_n = poses.len();
    let mut rot = vec![vec![Vec3::ZERO; frames_n]; n];
    let mut pos = vec![vec![Vec3::ZERO; frames_n]; n];
    for i in 0..n {
        let mut prev = Vec3::from(skeleton[i].rotation);
        for (f, p) in poses.iter().enumerate() {
            let local = match parent(skeleton, i) {
                Some(q) => p[q].inverse() * p[i],
                None => p[i],
            };
            prev = continuous(kh2_anim::euler_from_matrix(&local), prev);
            rot[i][f] = prev;
            pos[i][f] = local.w_axis.truncate();
        }
    }

    // KH1's own legs (`legs`, t-legs): where the template has KH1's leg rig, its joint words stay,
    // and the channels its pass reads are keyed so that it rebuilds the pose's legs on flat
    // ground: the hip's rotation x (the knee's side), the shin's rotation x, the ankle's rotation
    // in the model; the thigh and the other channels of these bones stay at the bind (the IK and
    // the helper place them). The helper itself (below) gets the ankle's position and the foot's
    // turn.
    let (h_count, h_at) = (u32_at(&r, 0x2C)? as usize, u32_at(&r, 0x30)? as usize);
    let helper_parents: Vec<i32> = (0..h_count).map(|k| u32_at(&r, h_at + k * 40 + 0xC).map(|v| v as i32)).collect::<Result<_>>()?;
    let legs = if kh1_legs { legs::find(&words, skeleton, &helper_parents) } else { Vec::new() };
    let mut leg_frames: Vec<Vec<legs::Frame>> = Vec::new();
    let mut report = Report::default();
    for leg in &legs {
        let h = leg.hip;
        let mut v: Vec<legs::Frame> = Vec::with_capacity(frames_n);
        for (f, p) in poses.iter().enumerate() {
            let fr = legs::frame(*leg, skeleton, p, pos[h][f], v.last());
            report.leg_hip_turn = report.leg_hip_turn.max(fr.turn_error[0]);
            report.leg_turn = report.leg_turn.max(fr.turn_error[1]).max(fr.turn_error[2]);
            v.push(fr);
        }
        for f in 0..frames_n {
            let fr = &v[f];
            let bind = |j: usize| (Vec3::from(skeleton[h + j].rotation), Vec3::from(skeleton[h + j].translation));
            let (r0, _) = bind(0);
            rot[h][f] = Vec3::new(fr.hip_roll, r0.y, r0.z);
            (rot[h + 1][f], pos[h + 1][f]) = bind(1);
            let (r2, t2) = bind(2);
            (rot[h + 2][f], pos[h + 2][f]) = (Vec3::new(fr.shin_roll, r2.y, r2.z), t2);
            (rot[h + 3][f], pos[h + 3][f]) = (fr.ankle, bind(3).1);
            rot[h + 4][f] = bind(4).0;
        }
        leg_frames.push(v);
    }
    report.kh1_legs = legs.len();

    // Channels: nothing at the bind, a constant, or keys.
    let mut keyed: Vec<Keyed> = Vec::new();
    let mut constants: Vec<(u16, u16, f32)> = Vec::new();
    let mut new_words = words.clone();
    let mut moved = vec![false; n];
    for i in 0..n {
        if words[i] & SKIP != 0 {
            continue;
        }
        let b = &skeleton[i];
        let mut moves = false;
        for c in 0..6 {
            let (values, bind, same, tol): (Vec<f32>, f32, f32, f32) = if c < 3 {
                (rot[i].iter().map(|v| v[c]).collect(), b.rotation[c], SAME_ROT, ROT_TOL)
            } else {
                (pos[i].iter().map(|v| v[c - 3]).collect(), b.translation[c - 3], SAME_POS, POS_TOL)
            };
            let kind = 4 + c as u8;
            let (lo, hi) = values.iter().fold((f32::MAX, f32::MIN), |(l, h), v| (l.min(*v), h.max(*v)));
            if values.iter().all(|v| (v - bind).abs() <= same) {
                continue;
            }
            if hi - lo <= same {
                constants.push((i as u16, kind as u16, values[0]));
                continue;
            }
            moves = true;
            keyed.push(Keyed { bone: i as u16, kind, keys: reduce(&values, tol) });
        }
        new_words[i] = if moves { 0 } else { STATIC };
        moved[i] = moves;
        report.static_bones += !moves as usize;
    }
    // The legs' own words (the template's), the hip built once only when nothing on it moves,
    // as KH1's own motions have it (slot 0's hip `0x02000029`, slot 5's `0x00000029`).
    for leg in &legs {
        for j in 0..5 {
            new_words[leg.hip + j] = words[leg.hip + j];
        }
        new_words[leg.hip] = (words[leg.hip] & !STATIC) | if moved[leg.hip] { 0 } else { STATIC };
    }
    report.keyed_channels = keyed.len();
    report.constant_channels = constants.len();
    report.keys = keyed.iter().map(|k| k.keys.len()).sum();

    // The template's helper channels and their keys (retimed), but for the travel helper's
    // translation when `travel` is given: ours.
    if !travel.is_empty() && h_count == 0 {
        return Err(bad("travel given, but the template has no helpers".into()));
    }
    let last = h_count.saturating_sub(1) as u16;
    let (hc_count, hc_at, keys_at) = (u32_at(&r, 0x20)? as usize, u32_at(&r, 0x24)? as usize, u32_at(&r, 0x28)? as usize);
    let mut helper_channels: Vec<[u8; 6]> = Vec::new();
    let mut helper_keys: Vec<Vec<u8>> = Vec::new();
    for c in 0..hc_count {
        let e = hc_at + c * 6;
        let s: [u8; 6] = r.get(e..e + 6).ok_or_else(|| bad("helper channel past the end".into()))?.try_into().unwrap();
        if !travel.is_empty() && u16::from_le_bytes([s[0], s[1]]) == last && (7..=9).contains(&(s[2] & 15)) {
            continue;
        }
        // (A leg's helper: ours, below.)
        if legs.iter().any(|l| l.helper == u16::from_le_bytes([s[0], s[1]]) as usize) {
            continue;
        }
        let (count, first) = (s[3] as usize, u16::from_le_bytes([s[4], s[5]]) as usize);
        let k = keys_at + first * 16;
        helper_keys.push(r.get(k..k + count * 16).ok_or_else(|| bad("helper key past the end".into()))?.to_vec());
        helper_channels.push(s);
    }
    if !travel.is_empty() {
        for c in 0..3 {
            let values: Vec<f32> = travel.iter().map(|t| t[c]).collect();
            if values.iter().all(|v| v.abs() <= SAME_POS) {
                continue;
            }
            let keys = reduce(&values, POS_TOL);
            let mut s = [0u8; 6];
            s[..2].copy_from_slice(&last.to_le_bytes());
            // (Kind 7..9, translation x..z; held outside the keys, bits 4..7 clear.)
            s[2] = 7 + c as u8;
            s[3] = keys.len() as u8;
            helper_channels.push(s);
            helper_keys.push(keys.iter().flat_map(|&(f, v)| key(f, v)).collect());
            report.travel_keys += keys.len();
        }
    }
    let mut helpers = r.get(h_at..h_at + h_count * 40).ok_or_else(|| bad("helpers past the end".into()))?.to_vec();
    // A leg's helper: under helper matrix 0 (the model, as the poses), scale 1, the first frame's
    // values in its record and keys where they move (KH1 samples a helper channel into the
    // motion's own record each frame, `0x140391C70`, and builds matrix k = matrix[parent] × its
    // scale, Euler X→Y→Z and translation, `0x1401DB080`).
    let mut leg_helper: Vec<[Option<Vec<(u32, f32)>>; 6]> = Vec::new();
    for (leg, v) in legs.iter().zip(&leg_frames) {
        let e = leg.helper * 40;
        let first = [v[0].helper_rot, v[0].helper_pos];
        let rec = &mut helpers[e..e + 40];
        for c in 0..3 {
            rec[c * 4..c * 4 + 4].copy_from_slice(&1f32.to_le_bytes());
            rec[0x10 + c * 4..0x14 + c * 4].copy_from_slice(&first[0][c].to_le_bytes());
            rec[0x1C + c * 4..0x20 + c * 4].copy_from_slice(&first[1][c].to_le_bytes());
        }
        rec[0xC..0x10].copy_from_slice(&0i32.to_le_bytes());
        let mut chans: [Option<Vec<(u32, f32)>>; 6] = Default::default();
        for c in 0..6 {
            let values: Vec<f32> = v.iter().map(|fr| if c < 3 { fr.helper_rot[c] } else { fr.helper_pos[c - 3] }).collect();
            let (same, tol) = if c < 3 { (SAME_ROT, ROT_TOL) } else { (SAME_POS, POS_TOL) };
            if values.iter().all(|x| (x - values[0]).abs() <= same) {
                continue;
            }
            let keys = reduce(&values, tol);
            let mut s = [0u8; 6];
            s[..2].copy_from_slice(&(leg.helper as u16).to_le_bytes());
            s[2] = 4 + c as u8;
            s[3] = keys.len() as u8;
            helper_channels.push(s);
            helper_keys.push(keys.iter().flat_map(|&(f, x)| key(f, x)).collect());
            report.leg_keys += keys.len();
            chans[c] = Some(keys);
        }
        leg_helper.push(chans);
    }
    let total_keys = report.keys + helper_keys.iter().map(|k| k.len() / 16).sum::<usize>();
    if total_keys > u16::MAX as usize {
        return Err(bad(format!("{total_keys} keys")));
    }
    let tracks = slot::events(&r)?;

    // The block: the template's header, then joint words, helpers, channels, helper channels,
    // keys, constants and events.
    let mut out = r.get(..0x50).ok_or_else(|| bad("template too short".into()))?.to_vec();
    let align = |v: &mut Vec<u8>| v.resize((v.len() + 15) & !15, 0);
    let at = out.len() as u32;
    put(&mut out, 0x14, at);
    for w in &new_words {
        out.extend_from_slice(&w.to_le_bytes());
    }
    align(&mut out);
    let at = out.len() as u32;
    put(&mut out, 0x30, at);
    out.extend_from_slice(&helpers);
    align(&mut out);
    put(&mut out, 0x18, keyed.len() as u32);
    let at = out.len() as u32;
    put(&mut out, 0x1C, at);
    let mut first = 0usize;
    for k in &keyed {
        out.extend_from_slice(&k.bone.to_le_bytes());
        out.extend_from_slice(&[k.kind, k.keys.len() as u8]);
        out.extend_from_slice(&(first as u16).to_le_bytes());
        first += k.keys.len();
    }
    align(&mut out);
    let at = out.len() as u32;
    put(&mut out, 0x24, at);
    // The helper channel count: the channels kept and added (it was left at the template's).
    put(&mut out, 0x20, helper_channels.len() as u32);
    for (s, k) in helper_channels.iter().zip(&helper_keys) {
        out.extend_from_slice(&s[..4]);
        out.extend_from_slice(&(first as u16).to_le_bytes());
        first += k.len() / 16;
    }
    align(&mut out);
    let at = out.len() as u32;
    put(&mut out, 0x28, at);
    for k in &keyed {
        for &(f, v) in &k.keys {
            out.extend_from_slice(&key(f, v));
        }
    }
    for k in &helper_keys {
        out.extend_from_slice(k);
    }
    align(&mut out);
    put(&mut out, 0x34, constants.len() as u32);
    let at = out.len() as u32;
    put(&mut out, 0x38, at);
    for (bone, kind, v) in &constants {
        out.extend_from_slice(&bone.to_le_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&v.to_le_bytes());
    }
    slot::append_events(&mut out, &tracks);

    // How close KH1 comes: its FK from the channels (and, on the legs, its leg pass, `legs::build`,
    // with no ground lift) against the poses given.
    let mut world = vec![Mat4::IDENTITY; n];
    let mut sampled = vec![(Vec3::ZERO, Vec3::ZERO); n];
    for (f, p) in poses.iter().enumerate() {
        for i in 0..n {
            let b = &skeleton[i];
            let mut r3 = Vec3::from(b.rotation);
            let mut t3 = Vec3::from(b.translation);
            for &(_, kind, v) in constants.iter().filter(|c| c.0 as usize == i) {
                match kind {
                    4..=6 => r3[kind as usize - 4] = v,
                    _ => t3[kind as usize - 7] = v,
                }
            }
            for k in keyed.iter().filter(|k| k.bone as usize == i) {
                let v = sample(&k.keys, f as f32);
                match k.kind {
                    4..=6 => r3[k.kind as usize - 4] = v,
                    _ => t3[k.kind as usize - 7] = v,
                }
            }
            sampled[i] = (r3, t3);
        }
        for i in 0..n {
            if let Some(li) = legs.iter().position(|l| (l.hip..l.hip + 5).contains(&i)) {
                let leg = legs[li];
                if i == leg.hip {
                    let ch = &leg_helper[li];
                    let at = |c: usize, first: f32| ch[c].as_ref().map_or(first, |k| sample(k, f as f32));
                    let helper_rot = Vec3::new(at(0, leg_frames[li][0].helper_rot.x), at(1, leg_frames[li][0].helper_rot.y), at(2, leg_frames[li][0].helper_rot.z));
                    let helper_pos = Vec3::new(at(3, leg_frames[li][0].helper_pos.x), at(4, leg_frames[li][0].helper_pos.y), at(5, leg_frames[li][0].helper_pos.z));
                    let s = |j: usize| sampled[leg.hip + j];
                    let built = legs::build(leg, skeleton, parent(skeleton, i).map_or(Mat4::IDENTITY, |q| world[q]), &legs::Sampled {
                        hip_t: s(0).1,
                        hip_roll: s(0).0.x,
                        thigh_t: s(1).1,
                        thigh_roll: s(1).0.x,
                        shin_roll: s(2).0.x,
                        ankle: s(3).0,
                        foot_t: s(4).1,
                        helper_rot,
                        helper_pos,
                    });
                    world[i..i + 5].copy_from_slice(&built);
                }
            } else {
                let (r3, t3) = sampled[i];
                let local = Mat4::from_rotation_translation(kh2_anim::euler_quat(r3), t3);
                world[i] = parent(skeleton, i).map_or(local, |q| world[q] * local);
            }
            if words[i] & SKIP == 0 {
                let e = (world[i].w_axis - p[i].w_axis).truncate().length();
                if e > report.max_error {
                    report.max_error = e;
                    report.worst_bone = i;
                }
            }
        }
    }
    Ok((out, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;

    #[test]
    fn continuous_angles_take_the_short_way() {
        let a = continuous(Vec3::new(-3.1, 0.2, 0.0), Vec3::new(3.1, 0.2, 0.0));
        assert!((a.x - (TAU - 3.1)).abs() < 1e-5, "{a}");
        // The other triple of the same rotation, when it is the nearer one.
        let q = kh2_anim::euler_quat(Vec3::new(0.3, 2.9, -0.4));
        let e = kh2_anim::euler_from_matrix(&Mat4::from_quat(q));
        let c = continuous(e, Vec3::new(0.3, 2.9, -0.4));
        assert!((c - Vec3::new(0.3, 2.9, -0.4)).length() < 1e-3, "{e} -> {c}");
        assert!(kh2_anim::euler_quat(c).angle_between(q) < 1e-4);
    }

    #[test]
    fn reduction_keeps_lines_and_corners() {
        let v: Vec<f32> = (0..=20).map(|i| if i <= 10 { i as f32 } else { 20.0 - i as f32 }).collect();
        assert_eq!(reduce(&v, 1e-4), vec![(0, 0.0), (10, 10.0), (20, 0.0)]);
        for (i, x) in v.iter().enumerate() {
            assert!((sample(&reduce(&v, 1e-4), i as f32) - x).abs() < 1e-4);
        }
    }

    /// A three-bone chain posed over 10 frames, baked onto a small template, comes back within
    /// the tolerances from the channels written.
    #[test]
    fn bakes_a_chain() {
        let bone = |parent: i16, t: [f32; 3]| Bone { index: 0, sibling: -1, parent, child: -1, flags: 0, scale: [1.0; 3], length: 1.0, rotation: [0.0; 3], translation: t };
        let skeleton = vec![bone(-1, [0.0, 90.0, 0.0]), bone(0, [0.0, 0.0, 20.0]), bone(1, [0.0, 0.0, 20.0])];
        let frames = 10u32;
        let poses: Vec<Vec<Mat4>> = (0..=frames)
            .map(|f| {
                let a = f as f32 * 0.3;
                let root = Mat4::from_rotation_translation(Quat::from_rotation_y(a), Vec3::new(f as f32, 90.0, 0.0));
                let mid = root * Mat4::from_rotation_translation(Quat::from_rotation_x(a * 0.5), Vec3::new(0.0, 0.0, 20.0));
                let tip = mid * Mat4::from_translation(Vec3::new(0.0, 0.0, 20.0));
                vec![root, mid, tip]
            })
            .collect();
        // Template: 3 bones, word 0, no channels, one helper (no channels), no events.
        let mut t = vec![0u8; 0x100];
        put(&mut t, 4, 5);
        put(&mut t, 8, 60f32.to_bits());
        put(&mut t, 0xC, 1);
        put(&mut t, 0x10, 3);
        put(&mut t, 0x14, 0x50);
        put(&mut t, 0x1C, 0x60);
        put(&mut t, 0x24, 0x60);
        put(&mut t, 0x28, 0x60);
        put(&mut t, 0x2C, 1);
        put(&mut t, 0x30, 0x60);
        put(&mut t, 0x38, 0x90);
        put(&mut t, 0x3C, 0x90);
        put(&mut t, 0x90, slot::END);
        let (b, rep) = baked(&t, frames, &[], &skeleton, &poses, &[], true).unwrap();
        assert!(rep.max_error < 0.05, "{rep:?}");
        assert_eq!(u32_at(&b, 4).unwrap(), frames);
        // The tip never moves against its parent: built once (static, bit 25).
        let words = u32_at(&b, 0x14).unwrap() as usize;
        assert_eq!(u32_at(&b, words + 8).unwrap(), STATIC);
        assert_eq!(u32_at(&b, words).unwrap(), 0);
        assert!(slot::events(&b).unwrap().is_empty());
        assert_eq!(u32_at(&b, 0x20).unwrap(), 0);

        // With travel: the last helper (here H0) gets a z channel, two keys for a line.
        let travel: Vec<Vec3> = (0..=frames).map(|f| Vec3::new(0.0, 0.0, f as f32 * 3.0)).collect();
        let moved: Vec<Vec<Mat4>> = poses.iter().zip(&travel).map(|(p, t)| p.iter().map(|m| Mat4::from_translation(*t) * *m).collect()).collect();
        let (b, rep) = baked(&t, frames, &[], &skeleton, &moved, &travel, true).unwrap();
        assert!(rep.max_error < 0.05, "{rep:?}");
        assert_eq!(rep.travel_keys, 2);
        assert_eq!(u32_at(&b, 0x20).unwrap(), 1);
        let hc = u32_at(&b, 0x24).unwrap() as usize;
        assert_eq!(&b[hc..hc + 4], &[0, 0, 9, 2]);
        let k = u32_at(&b, 0x28).unwrap() as usize + u16::from_le_bytes([b[hc + 4], b[hc + 5]]) as usize * 16;
        assert_eq!(u32_at(&b, k + 16).unwrap(), ((frames as i32 * 256) << 8 | 1) as u32);
        assert_eq!(f32::from_bits(u32_at(&b, k + 20).unwrap()), frames as f32 * 3.0);
    }
}
