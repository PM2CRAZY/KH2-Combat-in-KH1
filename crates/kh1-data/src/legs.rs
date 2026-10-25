//! KH1's own leg rig, kept on a baked motion (`bake`, `docs/kh1-anim-baking.md` §8): KH1's
//! two-bone leg IK and its ground lift (feet on slopes and stairs) run on our motion, aimed at
//! the baked motion's own ankles and feet.
//!
//! KH1 Sora's legs, as every one of his motions' joint words has them (`docs/kh1-anim-slots.md`
//! §1.6; Checked (files), `xa_ex_0010.mset`): a hip bone aimed at a foot helper (`lo` 1, H9 for
//! bones 16..20, H8 for 42..46), the thigh in two-bone IK mode 2 (thigh and shin solved to the
//! helper's position), the ankle placed where the IK ends and turned by its own rotation against
//! the model (world parent, keep position), the foot taking the helper's turn (`lo` 2). The bone
//! pass (Steam `0x1403900B0`, the solver `0x14038FC20`; Checked (decomp + exe)) computes, at scale
//! 1 (all of Sora's bones), in glam's terms (KH1's matrices are D3D rows, glam's columns; its
//! multiply `0x1401DB4D0 (out, a, b)` is `a * b`, its point transform `0x1401DAD10 (out, m, v)`
//! is `m * v`, its x turn `0x1400E6AC0` is `Rx * m`):
//! - **Hip** (`aim_local`): x at the helper from the bone's sampled translation, in the parent's
//!   space; y the parent's y made square to it; z = x × y; then turned about x by its sampled
//!   rotation x.
//! - **Thigh and shin** (`solve`): `d` from the thigh's sampled translation to the target in the
//!   hip's space, `p = (-d.y, d.x, 0)` normalised, `z = d × p`; that frame turned about x by the
//!   thigh's sampled rotation x; the shin's local is a turn about z by the knee angle (law of
//!   cosines on the bind lengths, `l1` the shin's bind x, `l2` the ankle's; sine positive for
//!   bend flag 0, the legs) and a shift of `l1` along x, then turned about x by the shin's sampled
//!   rotation x; the thigh turned about z so the chain ends on `d`.
//! - **Ankle**: at the shin's end (`world(shin) * bind(ankle)`), turned by its own sampled
//!   rotation (Euler X→Y→Z, `0x1401DB430`) against the model (helper matrix 0).
//! - **Foot**: the helper's turn, at `world(ankle) * its sampled translation`.
//! - **Ground lift** (in the thigh's step, only with the instance's `+0x14 & 3` and a thigh listed
//!   in the model's lift list): KH1 casts down from the helper's position and, when the floor is
//!   above the target (less the model's ankle height), lifts the target onto it (`+0x14 |=
//!   0x200000`). Sora's list (his model header's info block, `xa_ex_0010.mdls`; Checked (files)):
//!   the thighs 17 and 43, height 17.434 (his ankle's bind height). Nothing of it is ours: it acts
//!   on whatever target the motion's helper gives.
//!
//! So a baked leg is: the helper keyed to the baked ankle's position and the baked foot's turn
//! (in the model, its parent made matrix 0), the hip's rotation x keyed so the knee lands on the
//! baked knee (KH1's own motions key the hip's roll and leave the thigh's at the bind), the shin's
//! rotation x keyed to the baked shin's roll, the ankle's rotation keyed to the baked ankle's turn
//! in the model. On flat ground KH1 then shows the baked leg; on uneven ground its lift bends it.

use glam::{Mat4, Vec3};
use kh2_formats::anim::Bone;

/// Joint word fields (`docs/kh1-anim-slots.md` §1.6).
pub fn mode(w: u32) -> u32 {
    (w >> 21) & 3
}
pub fn lo(w: u32) -> u32 {
    w & 3
}
/// Helper `A` (1-based: matrix `A - 1`).
pub fn a_field(w: u32) -> u32 {
    ((w >> 16 & 0x800) | (w & 0x7FC)) >> 2
}
/// Helper `P` (1-based: matrix `P - 1`).
pub fn p_field(w: u32) -> u32 {
    ((w >> 8 & 0x10_0000) | (w & 0xF_F800)) >> 11
}
const KEEP_POS: u32 = 0x80_0000;
const WORLD_PARENT: u32 = 0x100_0000;
const SKIP: u32 = 0x400_0000;

/// One of KH1's legs in a template's joint words: bones `hip..=hip + 4` (hip, thigh, shin,
/// ankle, foot) and the helper record its IK aims at (`helper`, the matrix index, `P - 1`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Leg {
    pub hip: usize,
    pub helper: usize,
}

fn parent(skeleton: &[Bone], i: usize) -> Option<usize> {
    let p = skeleton[i].parent;
    (p >= 0 && (p as usize) < skeleton.len() && p as usize != i).then_some(p as usize)
}

/// The helpers a joint word reads (1-based).
fn reads(w: u32) -> [u32; 2] {
    if w & SKIP != 0 {
        return [0, 0];
    }
    let a = if lo(w) != 0 || mode(w) == 3 { a_field(w) } else { 0 };
    [a, p_field(w)]
}

/// KH1's legs in `words` (a template's joint words), each kept only when its layout is KH1's
/// leg rig exactly and its helper is its own: a hip aimed at helper `k` (mode 0, `lo` 1, no `P`,
/// no keep-position or world-parent bit), a thigh in mode 2 with no pole (`lo` 0), an ankle with
/// `P` = `k`, keep position and world parent (`lo` 0), a foot copying `k`'s turn (`lo` 2, no `P`);
/// the five bones a chain in the skeleton, the thigh at the hip's point, the shin and ankle on
/// their parents' x, scale 1; no other bone reads `k`; `k` a helper of the motion (`helpers`
/// records, their parents `helper_parents`) that no helper hangs from, and not `travel` (the last).
pub fn find(words: &[u32], skeleton: &[Bone], helper_parents: &[i32]) -> Vec<Leg> {
    let n = words.len().min(skeleton.len());
    let mut out = Vec::new();
    for h in 0..n.saturating_sub(4) {
        let w = &words[h..h + 5];
        if w.iter().any(|w| w & SKIP != 0) {
            continue;
        }
        let k = a_field(w[0]);
        let hip_ok = mode(w[0]) == 0 && lo(w[0]) == 1 && p_field(w[0]) == 0 && w[0] & (KEEP_POS | WORLD_PARENT) == 0;
        let thigh_ok = mode(w[1]) == 2 && lo(w[1]) == 0;
        let ankle_ok = mode(w[3]) == 0 && lo(w[3]) == 0 && p_field(w[3]) == k && w[3] & KEEP_POS != 0 && w[3] & WORLD_PARENT != 0;
        let foot_ok = mode(w[4]) == 0 && lo(w[4]) == 2 && a_field(w[4]) == k && p_field(w[4]) == 0 && w[4] & (KEEP_POS | WORLD_PARENT) == 0;
        if !(hip_ok && thigh_ok && ankle_ok && foot_ok) || k < 2 {
            continue;
        }
        let helper = k as usize - 1;
        if helper + 1 >= helper_parents.len() || helper_parents.iter().any(|&p| p == helper as i32) {
            continue;
        }
        let chain = (1..5).all(|j| parent(skeleton, h + j) == Some(h + j - 1));
        let b = |j: usize| &skeleton[h + j];
        let near = |v: f32| v.abs() < 1e-3;
        let shape = b(1).translation.iter().all(|&v| near(v)) && (2..4).all(|j| near(b(j).translation[1]) && near(b(j).translation[2]) && b(j).translation[0] > 1.0) && (0..5).all(|j| b(j).scale.iter().all(|&s| (s - 1.0).abs() < 1e-4));
        let own = (0..n).filter(|i| !(h..h + 5).contains(i)).all(|i| !reads(words[i]).contains(&k));
        if chain && shape && own {
            out.push(Leg { hip: h, helper });
        }
    }
    out
}

/// KH1's hip aim (`lo` 1): the local of a bone at `t` (its sampled translation, in the parent's
/// space) whose x points at `target` (the parent's space), rolled by `roll` about x.
pub fn aim_local(t: Vec3, target: Vec3, roll: f32) -> Mat4 {
    let a = (target - t).normalize_or_zero();
    let y = a.cross(Vec3::Y).cross(a).normalize_or_zero();
    let z = a.cross(y).normalize_or_zero();
    Mat4::from_cols(a.extend(0.0), y.extend(0.0), z.extend(0.0), t.extend(1.0)) * rot_x(roll)
}

fn rot_x(a: f32) -> Mat4 {
    if a == 0.0 { Mat4::IDENTITY } else { Mat4::from_rotation_x(a) }
}

/// KH1's two-bone solve (`0x14038FC20`, bend flag 0, no pole): the thigh's local (at `pos`, its
/// sampled translation) and the shin's, for `target` in the hip's space, with the bind lengths
/// `l1` / `l2` and the thigh's and shin's sampled rotations x (`twist1`, `twist2`).
pub fn solve(target: Vec3, pos: Vec3, l1: f32, l2: f32, twist1: f32, twist2: f32) -> (Mat4, Mat4) {
    let dv = target - pos;
    let dist = dv.length();
    let d = if dist > 0.0 { dv / dist } else { dv };
    let p = Vec3::new(-d.y, d.x, 0.0).normalize_or_zero();
    let z = d.cross(p).normalize_or_zero();
    let frame = Mat4::from_cols(d.extend(0.0), p.extend(0.0), z.extend(0.0), glam::Vec4::W) * rot_x(twist1);
    let c = ((dist * dist - l1 * l1 - l2 * l2) / (2.0 * l1 * l2)).clamp(-1.0, 1.0);
    let s = (1.0 - c * c).max(0.0).sqrt();
    let shin = Mat4::from_cols(glam::Vec4::new(c, s, 0.0, 0.0), glam::Vec4::new(-s, c, 0.0, 0.0), glam::Vec4::Z, glam::Vec4::new(l1, 0.0, 0.0, 1.0)) * rot_x(twist2);
    let (u, v) = ((c * l2 + l1) * dist, -(s * l2 * dist));
    let len = (u * u + v * v).sqrt();
    let (ca, sa) = if len > 0.0 { (u / len, v / len) } else { (1.0, 0.0) };
    let turn = Mat4::from_cols(glam::Vec4::new(ca, sa, 0.0, 0.0), glam::Vec4::new(-sa, ca, 0.0, 0.0), glam::Vec4::Z, glam::Vec4::W);
    let mut thigh = frame * turn;
    thigh.w_axis = pos.extend(1.0);
    (thigh, shin)
}

/// What one frame of a baked leg is keyed to (KH1's sampled values and the helper's).
#[derive(Clone, Copy, Debug, Default)]
pub struct Frame {
    /// The hip's rotation x (the thigh's left at its bind).
    pub hip_roll: f32,
    /// The shin's rotation x.
    pub shin_roll: f32,
    /// The ankle's rotation in the model (Euler X→Y→Z).
    pub ankle: Vec3,
    /// The helper: the foot's turn in the model (Euler) and the ankle's position.
    pub helper_rot: Vec3,
    pub helper_pos: Vec3,
    /// How far (radians) the hip, thigh and shin KH1 builds from these turn from the pose given.
    pub turn_error: [f32; 3],
}

fn rotation(m: &Mat4) -> Mat4 {
    let (_, q, _) = m.to_scale_rotation_translation();
    Mat4::from_quat(q)
}

fn angle(a: &Mat4, b: &Mat4) -> f32 {
    let (_, qa, _) = a.to_scale_rotation_translation();
    let (_, qb, _) = b.to_scale_rotation_translation();
    qa.angle_between(qb)
}

/// The nearest whole-turn copy of `a` to `near`.
fn unwrap(a: f32, near: f32) -> f32 {
    a + ((near - a) / std::f32::consts::TAU).round() * std::f32::consts::TAU
}

/// One frame of `leg` for the model-space `pose` (one matrix per bone), `hip_t` the hip's local
/// translation as KH1 samples it, `prev` the frame before (for continuous angles and a straight
/// leg's roll).
pub fn frame(leg: Leg, skeleton: &[Bone], pose: &[Mat4], hip_t: Vec3, prev: Option<&Frame>) -> Frame {
    let h = leg.hip;
    let parent_world = parent(skeleton, h).map_or(Mat4::IDENTITY, |q| pose[q]);
    let target = pose[h + 3].w_axis.truncate();
    let thigh_twist = skeleton[h + 1].rotation[0];
    let (l1, l2) = (skeleton[h + 2].translation[0], skeleton[h + 3].translation[0]);
    let hip0 = parent_world * aim_local(hip_t, parent_world.inverse().transform_point3(target), 0.0);
    // The roll that turns KH1's thigh as the pose's: KH1's thigh is `hip0 × Rx(roll) × Rz(α)`
    // (the solver's frame is the hip's own, the target on its x; `α` its turn onto `d`), so
    // `Rx(roll) = hip0⁻¹ × thigh × Rz(-α)`. Its knee then lies where the pose's does whenever the
    // pose bends the knee in the thigh's x-y plane, as the retarget does (its hinge, `Pole::Knee`);
    // with a nearly straight leg the thigh's roll is kept rather than a knee point that hardly
    // moves (the retarget's straight-leg roll).
    let dist = (target - hip0.w_axis.truncate()).length();
    let c = ((dist * dist - l1 * l1 - l2 * l2) / (2.0 * l1 * l2)).clamp(-1.0, 1.0);
    let s = (1.0 - c * c).max(0.0).sqrt();
    let alpha = (-(s * l2 * dist)).atan2((c * l2 + l1) * dist);
    let m = rotation(&hip0).inverse() * rotation(&pose[h + 1]) * Mat4::from_rotation_z(-alpha);
    let a = m.y_axis.z.atan2(m.y_axis.y);
    let total = prev.map_or(a, |p| unwrap(a, p.hip_roll + thigh_twist));
    let hip_roll = total - thigh_twist;
    let hip_world = hip0 * rot_x(hip_roll);
    let (thigh_l, shin_l) = solve(hip_world.inverse().transform_point3(target), Vec3::from(skeleton[h + 1].translation), l1, l2, thigh_twist, 0.0);
    let thigh_world = hip_world * thigh_l;
    let shin0 = thigh_world * shin_l;
    let delta = rotation(&shin0).inverse() * rotation(&pose[h + 2]);
    let r = (delta.y_axis.z).atan2(delta.y_axis.y);
    let shin_roll = prev.map_or(r, |p| unwrap(r, p.shin_roll));
    let shin_world = shin0 * rot_x(shin_roll);
    let cont = |e: Vec3, p: Option<Vec3>| p.map_or(e, |p| crate::bake::continuous_euler(e, p));
    let ankle = cont(kh2_anim::euler_from_matrix(&rotation(&pose[h + 3])), prev.map(|p| p.ankle));
    let helper_rot = cont(kh2_anim::euler_from_matrix(&rotation(&pose[h + 4])), prev.map(|p| p.helper_rot));
    Frame {
        hip_roll,
        shin_roll,
        ankle,
        helper_rot,
        helper_pos: target,
        turn_error: [angle(&hip_world, &pose[h]), angle(&thigh_world, &pose[h + 1]), angle(&shin_world, &pose[h + 2])],
    }
}

/// The leg's five bones as KH1 builds them (model space) from its sampled values: the hip's
/// translation and rotation x, the thigh's translation and rotation x, the shin's rotation x,
/// the ankle's rotation, the foot's translation, and the helper's turn and position.
pub struct Sampled {
    pub hip_t: Vec3,
    pub hip_roll: f32,
    pub thigh_t: Vec3,
    pub thigh_roll: f32,
    pub shin_roll: f32,
    pub ankle: Vec3,
    pub foot_t: Vec3,
    pub helper_rot: Vec3,
    pub helper_pos: Vec3,
}

/// KH1's bone pass on the leg (no ground lift): the hip's, thigh's, shin's, ankle's and foot's
/// matrices (model space), the hip's parent's given.
pub fn build(leg: Leg, skeleton: &[Bone], parent_world: Mat4, s: &Sampled) -> [Mat4; 5] {
    let h = leg.hip;
    let hip = parent_world * aim_local(s.hip_t, parent_world.inverse().transform_point3(s.helper_pos), s.hip_roll);
    let (l1, l2) = (skeleton[h + 2].translation[0], skeleton[h + 3].translation[0]);
    let (tl, sl) = solve(hip.inverse().transform_point3(s.helper_pos), s.thigh_t, l1, l2, s.thigh_roll, s.shin_roll);
    let thigh = hip * tl;
    let shin = thigh * sl;
    let at = shin.transform_point3(Vec3::from(skeleton[h + 3].translation));
    let mut ankle = Mat4::from_quat(kh2_anim::euler_quat(s.ankle));
    ankle.w_axis = at.extend(1.0);
    let mut foot = Mat4::from_quat(kh2_anim::euler_quat(s.helper_rot));
    foot.w_axis = ankle.transform_point3(s.foot_t).extend(1.0);
    [hip, thigh, shin, ankle, foot]
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;

    fn bone(parent: i16, t: [f32; 3]) -> Bone {
        Bone { index: 0, sibling: -1, parent, child: -1, flags: 0, scale: [1.0; 3], length: 1.0, rotation: [0.0; 3], translation: t }
    }

    /// A pelvis and one leg laid out as KH1 Sora's (hip 0 offset sideways, the thigh at its
    /// point, shin 29.6, ankle 32.0, foot at the ankle).
    fn leg_skeleton() -> Vec<Bone> {
        vec![bone(-1, [0.0, 90.0, 0.0]), bone(0, [0.0, 0.0, 7.0]), bone(1, [0.0, 0.0, 0.0]), bone(2, [29.6, 0.0, 0.0]), bone(3, [32.0, 0.0, 0.0]), bone(4, [0.0, 0.0, 0.0])]
    }

    #[test]
    fn finds_sora_s_leg_words() {
        let s = leg_skeleton();
        // Sora's words for 16..20 (xa_ex_0010.mset, slot 5): H9 = A / P 10.
        let words = [0, 0x0000_0029, 0x0240_0000, 0x0200_0000, 0x0380_5000, 0x0200_002A];
        let helpers = [-1, 0, 1, 2, 3, 3, 3, 3, 2, 2, 1];
        assert_eq!(find(&words, &s, &helpers), vec![Leg { hip: 1, helper: 9 }]);
        // Another bone aiming at H9 too: not KH1's own leg helper any more.
        let mut other = words;
        other[0] = 0x0000_0029;
        assert!(find(&other, &s, &helpers).is_empty());
    }

    /// Any bent leg posed in the model comes back from KH1's pass on the values worked out for
    /// it: hip, knee, ankle and foot where they were, the thigh and shin turned as given.
    #[test]
    fn kh1_rebuilds_a_bent_leg() {
        let s = leg_skeleton();
        let leg = Leg { hip: 1, helper: 9 };
        let pelvis = Mat4::from_rotation_translation(Quat::from_euler(glam::EulerRot::ZYX, 0.3, -1.57, -1.5), Vec3::new(0.0, 90.0, 0.0));
        for (bend, swing, roll) in [(0.6f32, 0.2f32, 0.0f32), (1.2, -0.4, 0.5), (0.3, 0.9, -1.0)] {
            let hip_t = Vec3::from(s[1].translation);
            let hip = pelvis * Mat4::from_rotation_translation(Quat::from_rotation_y(swing) * Quat::from_rotation_x(roll), hip_t);
            let thigh = hip * Mat4::from_rotation_z(-0.4);
            let shin = thigh * Mat4::from_rotation_translation(Quat::from_rotation_z(bend) * Quat::from_rotation_x(0.1), Vec3::new(29.6, 0.0, 0.0));
            let ankle = shin * Mat4::from_rotation_translation(Quat::from_rotation_y(0.2), Vec3::new(32.0, 0.0, 0.0));
            let foot = ankle * Mat4::from_rotation_z(0.3);
            let pose = [pelvis, hip, thigh, shin, ankle, foot];
            let f = frame(leg, &s, &pose, hip_t, None);
            let built = build(leg, &s, pelvis, &Sampled { hip_t, hip_roll: f.hip_roll, thigh_t: Vec3::ZERO, thigh_roll: 0.0, shin_roll: f.shin_roll, ankle: f.ankle, foot_t: Vec3::ZERO, helper_rot: f.helper_rot, helper_pos: f.helper_pos });
            for (j, m) in built.iter().enumerate() {
                let e = (m.w_axis - pose[j + 1].w_axis).truncate().length();
                assert!(e < 1e-2, "bend {bend}: bone {j} off by {e}");
            }
            // The shin's roll is the pose's (its x lies on the shin in both).
            assert!(angle(&built[2], &pose[3]) < 1e-3, "{}", angle(&built[2], &pose[3]));
            assert!(angle(&built[3], &pose[4]) < 1e-3 && angle(&built[4], &pose[5]) < 1e-3);
        }
    }
}
