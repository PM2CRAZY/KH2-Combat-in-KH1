//! KH2 Sora's poses on KH1 Sora's skeleton.
//!
//! Both skeletons bind in a T-pose facing +z, arms along ±x, with the same chain layout for
//! legs, spine, neck, arms and fingers (`examples/skeletons.rs` prints them side by side).
//!
//! KH2's bones can't be copied by their rotations: many of them (root, pelvis, spine, the
//! first hip bone, the ankles, the arms) carry another built-in orientation in its motions than
//! in its bind pose, a constant 80-210° spin (the idle's pelvis comes out turned 210° from
//! rest), which twisted KH1's waist and shorts in the first in-game test. So every turn is
//! measured from KH2's joint *positions*, which its own evaluator gets right: a frame per body
//! part (`Part`), built the same way in the motion and in the bind pose, and the KH1 bones of
//! that part turn by the difference.
//! - Torso parts: a left-right line and an up line, both between bones that ride on that part
//!   (pelvis: hip joints, and the pelvis to a belt bone; chest: collarbone roots, and a back
//!   bone to the chest; head: two symmetric side bones, and the head to a face bone), blended
//!   up KH1's spine and neck.
//! - Limb parts: aimed along the segment, rolled by a pole: where the knee points for thigh
//!   and shin (the foot's direction only while the leg is nearly straight; a foot kicked back
//!   sole-up in KH2's run had spun the thigh 166° when the foot set the roll), the shin for the
//!   foot, where the elbow points for the upper arm (the knuckle line while the arm is nearly
//!   straight), the knuckle line for forearm, hand and finger segments.
//! KH1 bones outside every part (skirt, belt, pouches, face, hair, thumbs, the weapon chain)
//! turn with their parent. Bone lengths stay KH1's: each joint is placed from its parent along
//! its own bind offset; the hips' translation is scaled by the ratio of the two hip heights.
//!
//! The parts and the KH1 bone lists are ours, worked out from the two bind poses (positions
//! and parent links), not from either game's code.

use glam::{Mat3, Mat4, Quat, Vec3};
use kh2_formats::anim::Bone;

#[derive(Clone, Copy)]
enum Pole {
    /// From the first bone's position to the second's.
    Line(usize, usize),
    /// Where the middle joint `b` of `a`-`b`-`c` points, away from the `a`-`c` line (the knee's
    /// front, the elbow's back; negated when `flip`), falling back to the `Line` while the limb
    /// is nearly straight. For a hinge both segments share it, whatever the foot or hand does.
    Joint { a: usize, b: usize, c: usize, fallback: (usize, usize), flip: bool },
}

#[derive(Clone, Copy)]
enum Part {
    /// x along `right` (`.1` to `.0`), y along `up` (`.0` to `.1`) made square to x.
    Torso { right: (usize, usize), up: (usize, usize) },
    /// y along `from` → `to`, z along the pole made square to y.
    Aim { from: usize, to: usize, pole: Pole },
}

/// The joint's offset from the end-to-end line, as a share of that line's length: below the
/// first the limb counts as straight (fallback pole), above the second as bent.
const BEND_STRAIGHT: f32 = 0.04;
const BEND_BENT: f32 = 0.12;

fn perp(v: Vec3, axis: Vec3) -> Vec3 {
    v - axis * v.dot(axis)
}

impl Part {
    fn frame(&self, p: &[Vec3]) -> Option<Quat> {
        let at = |i: usize| p.get(i).copied();
        let (x, y, z) = match *self {
            Part::Torso { right, up } => {
                let x = (at(right.0)? - at(right.1)?).normalize_or_zero();
                let y = perp(at(up.1)? - at(up.0)?, x).normalize_or_zero();
                (x, y, x.cross(y))
            }
            Part::Aim { from, to, pole } => {
                let y = (at(to)? - at(from)?).normalize_or_zero();
                let line = |a: usize, b: usize| Some(perp(at(b)? - at(a)?, y).normalize_or_zero());
                let z = match pole {
                    Pole::Line(a, b) => line(a, b)?,
                    Pole::Joint { a, b, c, fallback, flip } => {
                        let fb = line(fallback.0, fallback.1)?;
                        let (pa, pc) = (at(a)?, at(c)?);
                        let span = pc - pa;
                        let len = span.length().max(1e-4);
                        let out = perp(at(b)? - pa, span / len);
                        let w = ((out.length() / len - BEND_STRAIGHT) / (BEND_BENT - BEND_STRAIGHT)).clamp(0.0, 1.0);
                        let dir = perp(out, y).normalize_or_zero() * if flip { -1.0 } else { 1.0 };
                        (fb * (1.0 - w) + dir * w).normalize_or_zero()
                    }
                };
                (y.cross(z), y, z)
            }
        };
        if x == Vec3::ZERO || y == Vec3::ZERO || z == Vec3::ZERO {
            return None;
        }
        Some(Quat::from_mat3(&Mat3::from_cols(x, y, z)).normalize())
    }
}

/// KH2 bones of one side: hip, knee, ankle, heel, toe; clavicle, shoulder, elbow, wrist, the
/// middle knuckle, the index and little finger bases; finger chains (base, then two joints).
struct Side {
    hip: usize,
    knee: usize,
    ankle: usize,
    heel: usize,
    toe: usize,
    clavicle: usize,
    shoulder: usize,
    elbow: usize,
    wrist: usize,
    knuckle: usize,
    index: usize,
    little: usize,
}

/// +x side; the −x side is +29 on the legs and +92 on the arms.
const PLUS_X: Side = Side { hip: 11, knee: 12, ankle: 13, heel: 16, toe: 17, clavicle: 74, shoulder: 76, elbow: 79, wrist: 80, knuckle: 97, index: 93, little: 105 };

fn minus_x() -> Side {
    let s = PLUS_X;
    Side {
        hip: s.hip + 29,
        knee: s.knee + 29,
        ankle: s.ankle + 29,
        heel: s.heel + 29,
        toe: s.toe + 29,
        clavicle: s.clavicle + 92,
        shoulder: s.shoulder + 92,
        elbow: s.elbow + 92,
        wrist: s.wrist + 92,
        knuckle: s.knuckle + 92,
        index: s.index + 92,
        little: s.little + 92,
    }
}

/// Every part, and the KH1 bones each turns: `(KH1 bone, part a, part b, blend toward b)`.
fn layout() -> (Vec<Part>, Vec<(usize, usize, usize, f32)>) {
    let mut parts = vec![
        // Each from bones that ride on that part (its children), so another part's bend
        // can't tilt it.
        Part::Torso { right: (10, 39), up: (3, 4) },      // 0 pelvis: hip joints; a belt bone
        Part::Torso { right: (74, 166), up: (69, 68) },   // 1 chest: collarbone roots; back to chest
        Part::Torso { right: (146, 152), up: (141, 160) }, // 2 head: two side bones; a face bone
    ];
    let (pelvis, chest, head) = (0, 1, 2);
    let mut bones: Vec<(usize, usize, usize, f32)> = vec![];
    let one = |b: usize, p: usize, bones: &mut Vec<(usize, usize, usize, f32)>| bones.push((b, p, p, 0.0));
    for b in [0, 1, 2, 3, 68, 69] {
        one(b, pelvis, &mut bones);
    }
    bones.push((70, pelvis, chest, 0.33));
    bones.push((71, pelvis, chest, 0.67));
    bones.push((76, pelvis, chest, 0.67));
    for b in [77, 78] {
        one(b, chest, &mut bones);
    }
    for (i, b) in (144..=148).enumerate() {
        bones.push((b, chest, head, 0.2 + 0.15 * i as f32));
    }
    for b in [149, 150, 151] {
        one(b, head, &mut bones);
    }
    // KH1 per side: hips (2), knee, ankle (3), heel, toe (2); clavicle (2), shoulder (3),
    // elbow, wrist (3), palm (3), knuckles; finger bases (+x 212 / 217 / 222 / 227).
    struct K1 {
        hip: usize,
        clavicle: usize,
        finger_shift: usize,
    }
    for (s, k) in [(PLUS_X, K1 { hip: 42, clavicle: 194, finger_shift: 0 }), (minus_x(), K1 { hip: 16, clavicle: 89, finger_shift: 104 })] {
        let mut add = |p: Part| {
            parts.push(p);
            parts.len() - 1
        };
        let knee = Pole::Joint { a: s.hip, b: s.knee, c: s.ankle, fallback: (s.heel, s.toe), flip: false };
        let thigh = add(Part::Aim { from: s.hip, to: s.knee, pole: knee });
        let shin = add(Part::Aim { from: s.knee, to: s.ankle, pole: knee });
        let foot = add(Part::Aim { from: s.heel, to: s.toe, pole: Pole::Line(s.ankle, s.knee) });
        let clav = add(Part::Aim { from: s.clavicle, to: s.shoulder, pole: Pole::Line(67, 137) });
        let upper = add(Part::Aim { from: s.shoulder, to: s.elbow, pole: Pole::Joint { a: s.shoulder, b: s.elbow, c: s.wrist, fallback: (s.little, s.index), flip: true } });
        let fore = add(Part::Aim { from: s.elbow, to: s.wrist, pole: Pole::Line(s.little, s.index) });
        let hand = add(Part::Aim { from: s.wrist, to: s.knuckle, pole: Pole::Line(s.little, s.index) });
        let h = k.hip;
        for b in [h, h + 1] {
            one(b, thigh, &mut bones);
        }
        one(h + 2, shin, &mut bones);
        for b in h + 3..=h + 8 {
            one(b, foot, &mut bones);
        }
        let c = k.clavicle;
        for b in [c, c + 1] {
            one(b, clav, &mut bones);
        }
        for b in c + 2..=c + 4 {
            one(b, upper, &mut bones);
        }
        one(c + 5, fore, &mut bones);
        for b in c + 6..=c + 8 {
            one(b, fore, &mut bones);
        }
        for b in c + 9..=c + 12 {
            one(b, hand, &mut bones);
        }
        // Fingers: KH2 base, a co-located joint, then two joints; KH1 base, a co-located
        // joint, then three.
        for &(k1, k2) in &[(212usize, 93usize), (217, 97), (222, 101), (227, 105)] {
            let k1 = k1 - k.finger_shift;
            let k2 = k2 + (s.index - PLUS_X.index);
            let first = add(Part::Aim { from: k2 + 1, to: k2 + 2, pole: Pole::Line(s.little, s.index) });
            let second = add(Part::Aim { from: k2 + 2, to: k2 + 3, pole: Pole::Line(s.little, s.index) });
            one(k1, hand, &mut bones);
            one(k1 + 1, first, &mut bones);
            for b in k1 + 2..=k1 + 4 {
                one(b, second, &mut bones);
            }
        }
    }
    (parts, bones)
}

pub struct Retarget {
    parent: Vec<Option<usize>>,
    /// KH1 bind: position, rotation, scale (model space).
    pos: Vec<Vec3>,
    rot: Vec<Quat>,
    scale: Vec<Vec3>,
    parts: Vec<Part>,
    /// Each part's frame in KH2's bind pose, inverted.
    rest_inv: Vec<Option<Quat>>,
    /// Per KH1 bone: (part a, part b, blend).
    turns: Vec<Option<(usize, usize, f32)>>,
    /// KH1 hip height / KH2 hip height.
    hip_ratio: f32,
}

fn positions(m: &[Mat4]) -> Vec<Vec3> {
    m.iter().map(|m| m.w_axis.truncate()).collect()
}

impl Retarget {
    pub fn new(kh1: &[Bone], kh2: &[Bone]) -> Retarget {
        let b1 = kh2_anim::Pose::bind(kh1).bones;
        let b2 = kh2_anim::Pose::bind(kh2).bones;
        let (mut pos, mut rot, mut scale) = (Vec::new(), Vec::new(), Vec::new());
        for m in &b1 {
            let (s, r, t) = m.to_scale_rotation_translation();
            pos.push(t);
            rot.push(r.normalize());
            scale.push(s);
        }
        let (parts, bones) = layout();
        let rest = positions(&b2);
        let rest_inv = parts.iter().map(|p| p.frame(&rest).map(|q| q.inverse())).collect();
        let mut turns = vec![None; kh1.len()];
        for (b, a, c, t) in bones {
            if b < kh1.len() {
                turns[b] = Some((a, c, t));
            }
        }
        let parent = kh1.iter().enumerate().map(|(i, b)| (b.parent >= 0 && (b.parent as usize) < i).then_some(b.parent as usize)).collect();
        let h2 = b2.first().map_or(1.0, |m| m.w_axis.y);
        let hip_ratio = if h2.abs() > 1.0 { pos.first().map_or(1.0, |p| p.y) / h2 } else { 1.0 };
        Retarget { parent, pos, rot, scale, parts, rest_inv, turns, hip_ratio }
    }

    /// KH1's hip height over KH2's (KH2 distances to KH1 ones).
    pub fn hip_ratio(&self) -> f32 {
        self.hip_ratio
    }

    /// How many KH1 bones a measured part turns.
    pub fn mapped(&self) -> usize {
        self.turns.iter().filter(|t| t.is_some()).count()
    }

    /// KH1 model-space bone matrices for a KH2 model-space pose.
    pub fn pose(&self, kh2: &[Mat4]) -> Vec<Mat4> {
        let now = positions(kh2);
        let part: Vec<Option<Quat>> = self.parts.iter().zip(&self.rest_inv).map(|(p, r)| Some(p.frame(&now)? * (*r)?)).collect();
        let n = self.pos.len();
        let mut turn = vec![Quat::IDENTITY; n];
        let mut at = vec![Vec3::ZERO; n];
        let mut out = vec![Mat4::IDENTITY; n];
        for i in 0..n {
            let own = self.turns[i].and_then(|(a, b, t)| Some(part[a]?.slerp(part[b]?, t)));
            match self.parent[i] {
                None => {
                    turn[i] = own.unwrap_or(Quat::IDENTITY);
                    at[i] = now.first().map_or(self.pos[i], |p| *p * self.hip_ratio);
                }
                Some(p) => {
                    turn[i] = own.unwrap_or(turn[p]);
                    at[i] = at[p] + turn[p] * (self.pos[i] - self.pos[p]);
                }
            }
            out[i] = Mat4::from_scale_rotation_translation(self.scale[i], (turn[i] * self.rot[i]).normalize(), at[i]);
        }
        out
    }

    /// The bind pose (a T-pose), as the game would draw it.
    pub fn bind(&self) -> Vec<Mat4> {
        (0..self.pos.len()).map(|i| Mat4::from_scale_rotation_translation(self.scale[i], self.rot[i], self.pos[i])).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kh1_bone_has_one_part() {
        let mut seen = std::collections::HashSet::new();
        for (b, ..) in layout().1 {
            assert!(seen.insert(b), "KH1 bone {b} listed twice");
        }
    }
}
