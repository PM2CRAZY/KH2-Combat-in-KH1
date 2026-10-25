//! KH1 Sora's poses on KH2 Sora's skeleton: the reverse of [`crate::retarget`], so KH2's model
//! can be drawn while KH1 animates him (its own moves, cut scenes).
//!
//! The same body parts as `retarget` (pelvis, chest, head; thighs, shins, feet; collarbones,
//! upper arms, forearms, hands; thumbs and finger segments), measured the same way, but on KH1's
//! joints: each part's frame comes from joint *positions* (a limb's direction, its knee or
//! elbow for the roll; the torso's left-right and up lines), taken from the KH1 joints at the
//! places of the KH2 joints that define the part (in the two binds, KH2's scaled to KH1's size).
//! Each part turns by its frame now against its frame in KH1's bind, and the KH2 bones riding it
//! turn with it. KH1's bone *rotations* are never read: KH1's own rolls and twist helpers, and
//! the ground IK, which moves leg joints without turning every bone the same way, twisted the
//! KH2 mesh when they were copied (the user: KH1's animations "super contorted" on the KH2
//! model).
//!
//! - The torso parts (pelvis, chest, head) take the turn of a KH1 bone that part alone turns
//!   (its own rotation from the bind): KH1's spine has no joints where KH2's torso lines are.
//! - A KH2 bone that starts a part turns with it: a limb segment's first joint, the ankle for
//!   the foot, the bone a torso part's joints hang from (the root with the pelvis). The spine
//!   between the pelvis and the chest, and the neck between the chest and the head, blend the
//!   two by their place along the chain, as `retarget` does for KH1's.
//! - Every other bone (twist joints, hair, face, hood, cloth, fingertips) keeps KH2's own bind
//!   relation to the bone it hangs from.
//! - Joints are placed from the parent along KH2's own bind offsets, so the mesh keeps KH2's
//!   proportions; the hips go where KH1's are, scaled back to KH2's size.
//!
//! The pairing is ours, worked out from the two bind poses, not from either game's code.

use crate::retarget::{frames, Part, Pole, Retarget};
use glam::{Mat4, Quat, Vec3};
use kh2_formats::anim::Bone;

/// How a KH2 bone turns: with a part, a blend of two, or with its parent.
#[derive(Clone, Copy, Debug)]
enum Ride {
    Part(usize),
    Blend(usize, usize, f32),
    Parent,
}

pub struct Inverse {
    parts: Vec<Part>,
    /// Per torso part: the KH1 bone whose turn it takes (the highest one that part alone turns),
    /// and that bone's bind rotation, inverted.
    torso: Vec<Option<(usize, Quat)>>,
    /// Per KH2 joint a part reads: the KH1 joint at its place.
    landmark: Vec<Option<usize>>,
    /// Each part's frame in KH1's bind (measured on the landmarks), inverted.
    rest_inv: Vec<Option<Quat>>,
    ride: Vec<Ride>,
    parent: Vec<Option<usize>>,
    /// KH2 bind: scale, rotation, position (model space, KH2 units).
    scale: Vec<Vec3>,
    rot: Vec<Quat>,
    pos: Vec<Vec3>,
    /// KH1 distances over KH2's (`Retarget::hip_ratio`).
    ratio: f32,
}

fn parents(bones: &[Bone]) -> Vec<Option<usize>> {
    bones.iter().enumerate().map(|(i, b)| (b.parent >= 0 && (b.parent as usize) < i).then_some(b.parent as usize)).collect()
}

/// The KH2 joints a part's frame reads.
fn joints(p: &Part) -> Vec<usize> {
    match *p {
        Part::Torso { right, up } => vec![right.0, right.1, up.0, up.1],
        Part::Aim { from, to, pole } => {
            let mut v = vec![from, to];
            match pole {
                Pole::Line(a, b) => v.extend([a, b]),
                Pole::Joint { a, b, c, fallback, .. } => v.extend([a, b, c, fallback.0, fallback.1]),
                Pole::Knee { a, b, c, hips } => v.extend([a, b, c, hips.0, hips.1]),
                Pole::Foot { ankle, .. } => v.push(ankle),
            }
            v
        }
    }
}

/// The chain from `b` up to the root.
fn ancestors(parent: &[Option<usize>], b: usize) -> Vec<usize> {
    let mut v = vec![b];
    let mut c = b;
    while let Some(p) = parent.get(c).copied().flatten() {
        v.push(p);
        c = p;
        if v.len() > parent.len() {
            break;
        }
    }
    v
}

/// The deepest bone every one of `bones` hangs from (or is).
fn common_ancestor(parent: &[Option<usize>], bones: &[usize]) -> Option<usize> {
    let first = ancestors(parent, *bones.first()?);
    first.into_iter().find(|a| bones.iter().all(|&b| ancestors(parent, b).contains(a)))
}

impl Inverse {
    /// `usable(kh1_bone)`: the KH1 joints a KH2 joint may be paired with (the body).
    pub fn new(kh1: &[Bone], kh2: &[Bone], retarget: &Retarget, usable: impl Fn(usize) -> bool) -> Inverse {
        let ratio = retarget.hip_ratio();
        let ratio = if ratio.is_finite() && ratio > 0.1 { ratio } else { 1.0 };
        let b1 = kh2_anim::Pose::bind(kh1).bones;
        let b2 = kh2_anim::Pose::bind(kh2).bones;
        let p1: Vec<Vec3> = b1.iter().map(|m| m.w_axis.truncate()).collect();
        let (mut scale, mut rot, mut pos) = (Vec::new(), Vec::new(), Vec::new());
        for m in &b2 {
            let (s, r, t) = m.to_scale_rotation_translation();
            scale.push(s);
            rot.push(r.normalize());
            pos.push(t);
        }
        let parts: Vec<Part> = retarget.parts().to_vec();
        let n2 = kh2.len();
        // The KH1 joint nearest each KH2 joint a part reads (KH2's bind scaled to KH1's size).
        let mut landmark = vec![None; n2];
        for p in &parts {
            for j in joints(p) {
                if j >= n2 || landmark[j].is_some() {
                    continue;
                }
                let at = pos[j] * ratio;
                landmark[j] = (0..kh1.len()).filter(|&i| usable(i)).min_by(|&a, &b| p1[a].distance(at).total_cmp(&p1[b].distance(at)));
            }
        }
        // Torso parts take a KH1 torso bone's own turn: KH1's spine has no joints placed like
        // KH2's (its chest's up line read off pelvis joints and leaned the shoulders 20 units).
        let torso = parts
            .iter()
            .enumerate()
            .map(|(i, p)| match p {
                Part::Torso { .. } => (0..kh1.len()).find(|&b| usable(b) && retarget.single_part(b) == Some(i)).map(|b| (b, b1[b].to_scale_rotation_translation().1.normalize().inverse())),
                _ => None,
            })
            .collect();
        let inv = Inverse { parts, torso, landmark, rest_inv: Vec::new(), ride: Vec::new(), parent: parents(kh2), scale, rot, pos, ratio };
        let rest = inv.measure(&p1);
        let rest_inv = rest.into_iter().map(|q| q.map(|q| q.inverse())).collect();
        let ride = inv.rides(n2);
        Inverse { rest_inv, ride, ..inv }
    }

    /// Which part each KH2 bone rides.
    fn rides(&self, n2: usize) -> Vec<Ride> {
        let parent = &self.parent;
        // Bones that start a part: a limb segment's first joint (the foot's ankle too), and the
        // bone a torso part's joints hang from.
        let mut starts: Vec<(usize, usize)> = Vec::new();
        let mut torso = Vec::new();
        for (i, p) in self.parts.iter().enumerate() {
            match *p {
                Part::Aim { from, pole, .. } => {
                    starts.push((from, i));
                    if let Pole::Foot { ankle, .. } = pole {
                        starts.push((ankle, i));
                    }
                }
                Part::Torso { .. } => {
                    if let Some(b) = common_ancestor(parent, &joints(p)) {
                        starts.push((b, i));
                        torso.push((b, i));
                    }
                }
            }
        }
        let start_of = |b: usize| starts.iter().find(|(s, _)| *s == b).map(|(_, i)| *i);
        let mut ride = vec![Ride::Parent; n2];
        for (b, r) in ride.iter_mut().enumerate() {
            if let Some(p) = start_of(b) {
                *r = Ride::Part(p);
            } else if parent.get(b).copied().flatten().is_none() {
                // A root (above the pelvis's bone): the pelvis carries it.
                if let Some(&(_, p)) = torso.first() {
                    *r = Ride::Part(p);
                }
            }
        }
        // The spine and neck: between two torso parts' bones, blended by place on the chain.
        for w in torso.windows(2) {
            let ((lo, pa), (hi, pb)) = (w[0], w[1]);
            // (From the upper part's bone down to where the two branch: the pelvis's bone may
            // sit on a side branch, the hips', not the spine's.)
            let Some(fork) = common_ancestor(parent, &[lo, hi]) else { continue };
            let chain = ancestors(parent, hi);
            let Some(k) = chain.iter().position(|&c| c == fork) else { continue };
            for (step, &c) in chain[1..k].iter().enumerate() {
                let t = 1.0 - (step + 1) as f32 / k as f32;
                ride[c] = Ride::Blend(pa, pb, t);
            }
        }
        ride
    }

    /// Every part's frame on KH1's joint positions (KH1 units), in KH2's index space.
    fn measure(&self, kh1_pos: &[Vec3]) -> Vec<Option<Quat>> {
        let at: Vec<Vec3> = self.landmark.iter().map(|l| l.and_then(|i| kh1_pos.get(i).copied()).map_or(Vec3::ZERO, |p| p / self.ratio)).collect();
        frames(&self.parts, &at)
    }

    /// How many KH2 bones turn with a measured part.
    pub fn mapped(&self) -> usize {
        self.ride.iter().filter(|r| !matches!(r, Ride::Parent)).count()
    }

    /// KH2 model-space bone matrices (KH2 units) for KH1 model-space ones (KH1 units).
    pub fn pose(&self, kh1: &[Mat4]) -> Vec<Mat4> {
        let now: Vec<Vec3> = kh1.iter().map(|m| m.w_axis.truncate()).collect();
        let mut part: Vec<Option<Quat>> = self.measure(&now).into_iter().zip(&self.rest_inv).map(|(f, r)| Some((f? * (*r)?).normalize())).collect();
        for (p, t) in part.iter_mut().zip(&self.torso) {
            if let Some((b, inv)) = *t {
                *p = kh1.get(b).map(|m| (m.to_scale_rotation_translation().1.normalize() * inv).normalize()).filter(|q| q.is_finite());
            }
        }
        let n = self.pos.len();
        let mut turn = vec![Quat::IDENTITY; n];
        let mut at = vec![Vec3::ZERO; n];
        let mut out = vec![Mat4::IDENTITY; n];
        let root = kh1.first().map(|m| m.w_axis.truncate() / self.ratio);
        for b in 0..n {
            let own = match self.ride[b] {
                Ride::Part(p) => part[p],
                Ride::Blend(a, c, t) => match (part[a], part[c]) {
                    (Some(x), Some(y)) => Some(x.slerp(y, t)),
                    (x, y) => x.or(y),
                },
                Ride::Parent => None,
            };
            match self.parent[b] {
                None => {
                    turn[b] = own.unwrap_or(Quat::IDENTITY);
                    at[b] = match (b, root) {
                        (0, Some(r)) => r,
                        _ => at[0] + turn[0] * (self.pos[b] - self.pos[0]),
                    };
                }
                Some(p) => {
                    turn[b] = own.unwrap_or(turn[p]);
                    at[b] = at[p] + turn[p] * (self.pos[b] - self.pos[p]);
                }
            }
            out[b] = Mat4::from_scale_rotation_translation(self.scale[b], (turn[b] * self.rot[b]).normalize(), at[b]);
        }
        out
    }
}
