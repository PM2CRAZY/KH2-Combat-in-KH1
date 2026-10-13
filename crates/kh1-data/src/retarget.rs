//! KH2 Sora's poses on KH1 Sora's skeleton.
//!
//! Both skeletons bind in a T-pose facing +z, arms along ±x, with the same chain layout for
//! legs, spine, neck, arms and fingers (`examples/skeletons.rs` prints them side by side). So a
//! KH2 pose carries over as rotations from rest, bone by bone: a mapped KH1 bone turns, in
//! model space, by the turn its KH2 counterpart makes from KH2's bind pose
//! (`anim * bind⁻¹`); an unmapped one (KH1's skirt, belt, pouches, face, weapon chain) turns
//! with its parent. Bone lengths stay KH1's: each joint is placed from its parent along its
//! own bind offset. The hips' translation is scaled by the ratio of the two hip heights.
//!
//! The map is ours, worked out from the two bind poses (positions and parent links), not from
//! either game's code.

use glam::{Mat4, Quat, Vec3};
use kh2_formats::anim::Bone;

/// KH1 bone ← KH2 bone. Runs of KH1 bones are listed with their KH2 counterpart; the left
/// arm (−x) is the right arm's map shifted (KH1 −105 / −104 past the hand, KH2 −92).
fn pairs() -> Vec<(usize, usize)> {
    let mut v: Vec<(usize, usize)> = vec![
        // Root, hips, pelvis.
        (0, 0),
        (1, 1),
        (2, 2),
        (3, 3),
        // Spine to chest.
        (68, 64),
        (69, 65),
        (70, 66),
        (71, 67),
        (76, 67),
        (77, 67),
        (78, 68),
        // Neck and head.
        (144, 136),
        (145, 136),
        (146, 137),
        (147, 138),
        (148, 139),
        (149, 140),
        (150, 140),
        (151, 141),
    ];
    // Legs: hip, hip, knee, ankle ×3, heel, toe ×2 (identical layouts).
    for i in 0..9 {
        v.push((16 + i, 39 + i)); // −x
        v.push((42 + i, 10 + i)); // +x
    }
    // +x arm: clavicle ×2, shoulder ×3, elbow, wrist / palm ×6, knuckles.
    let mut arm: Vec<(usize, usize)> = vec![(194, 74), (195, 75), (196, 76), (197, 77), (198, 78), (199, 79)];
    for b in 200..=205 {
        arm.push((b, 85));
    }
    arm.push((206, 86));
    // Thumb, then index, middle, ring, little (KH1 has one more bone per finger: the last
    // repeats the KH2 tip).
    let mut fingers: Vec<(usize, usize)> = (0..5).map(|i| (207 + i, 88 + i.min(4))).collect();
    for (k1, k2) in [(212, 93), (217, 97), (222, 101), (227, 105)] {
        for i in 0..5 {
            fingers.push((k1 + i, k2 + i.min(3)));
        }
    }
    for &(a, b) in &arm {
        v.push((a, b));
        v.push((a - 105, b + 92));
    }
    for &(a, b) in &fingers {
        v.push((a, b));
        v.push((a - 104, b + 92));
    }
    v
}

pub struct Retarget {
    parent: Vec<Option<usize>>,
    /// KH1 bind: position, rotation, scale (model space).
    pos: Vec<Vec3>,
    rot: Vec<Quat>,
    scale: Vec<Vec3>,
    /// KH2 bind rotations, inverted.
    kh2_rest_inv: Vec<Quat>,
    map: Vec<Option<usize>>,
    /// KH1 hip height / KH2 hip height.
    hip_ratio: f32,
}

fn srt(m: &Mat4) -> (Vec3, Quat, Vec3) {
    let (s, r, t) = m.to_scale_rotation_translation();
    (s, r.normalize(), t)
}

impl Retarget {
    pub fn new(kh1: &[Bone], kh2: &[Bone]) -> Retarget {
        let b1 = kh2_anim::Pose::bind(kh1).bones;
        let b2 = kh2_anim::Pose::bind(kh2).bones;
        let (mut pos, mut rot, mut scale) = (Vec::new(), Vec::new(), Vec::new());
        for m in &b1 {
            let (s, r, t) = srt(m);
            pos.push(t);
            rot.push(r);
            scale.push(s);
        }
        let kh2_rest_inv = b2.iter().map(|m| srt(m).1.inverse()).collect();
        let mut map = vec![None; kh1.len()];
        for (a, b) in pairs() {
            if a < kh1.len() && b < kh2.len() {
                map[a] = Some(b);
            }
        }
        let parent = kh1.iter().enumerate().map(|(i, b)| (b.parent >= 0 && (b.parent as usize) < i).then_some(b.parent as usize)).collect();
        let h2 = b2.first().map_or(1.0, |m| m.w_axis.y);
        let hip_ratio = if h2.abs() > 1.0 { pos.first().map_or(1.0, |p| p.y) / h2 } else { 1.0 };
        Retarget { parent, pos, rot, scale, kh2_rest_inv, map, hip_ratio }
    }

    /// How many KH1 bones take a KH2 bone's turn.
    pub fn mapped(&self) -> usize {
        self.map.iter().filter(|m| m.is_some()).count()
    }

    /// KH1 model-space bone matrices for a KH2 model-space pose.
    pub fn pose(&self, kh2: &[Mat4]) -> Vec<Mat4> {
        let n = self.pos.len();
        let mut turn = vec![Quat::IDENTITY; n];
        let mut at = vec![Vec3::ZERO; n];
        let mut out = vec![Mat4::IDENTITY; n];
        for i in 0..n {
            let own = self.map[i].and_then(|k| Some(srt(kh2.get(k)?).1 * self.kh2_rest_inv[k]));
            match self.parent[i] {
                None => {
                    turn[i] = own.unwrap_or(Quat::IDENTITY);
                    at[i] = match self.map[i].and_then(|k| kh2.get(k)) {
                        Some(m) => m.w_axis.truncate() * self.hip_ratio,
                        None => self.pos[i],
                    };
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
    fn map_is_one_to_one_on_kh1() {
        let mut seen = std::collections::HashSet::new();
        for (a, _) in pairs() {
            assert!(seen.insert(a), "KH1 bone {a} mapped twice");
        }
    }
}
