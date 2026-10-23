//! KH1 Sora's hair, hood and keychain following KH2's own animation of those parts, while KH2's
//! moves pose him (the user: "add the animations for sora's hair and chain on each KH2
//! animation instead [of the physics], don't affect the KH1 animations").
//!
//! KH2 animates these parts by hand in every motion: eleven hair spikes on the head, the hood
//! on the back, and the Keyblade's chain (the weapon's own motion, same id). `Retarget` turns
//! the body only, so KH1's spikes, hood and chain used to keep their rest shape (or swing by
//! `jiggle`'s physics). Here each KH1 link takes the movement of the KH2 bone matched to it:
//! - **Hair and hood: the turn, measured against the head (hair) or the chest (hood).** KH2's
//!   spikes are single bones with no child to point at, so a direction can't be read from
//!   positions; instead the KH2 bone's turn away from its bind pose, taken relative to the
//!   head (or chest) so the head's own turn is left out, is put on the KH1 link the same way,
//!   relative to KH1's head as posed now. Both binds face +z with the same axes; KH2's spikes
//!   rest within 20° of their bind in its idle, and both hoods bind straight back, so the bind
//!   is a shared neutral. KH1 keeps its own hair style; it moves the way KH2's moves.
//!   Guards: a spike is held short of the head, the hood swung off the back and the head.
//! - **Keychain: the direction of each link.** The two chains bind with the same layout (nine
//!   links from the pommel along -z, within a unit of each other), so each KH1 link points
//!   along its KH2 link in model space, hanging from KH1's own pommel. KH1's Keyblade is held
//!   by KH1's grip, a little off KH2's: the directions are first turned by the swing from KH2's
//!   handle (grip → pommel) to KH1's, so the chain leaves the pommel as KH2's does. The last
//!   link (the charm) is a leaf in KH2: its direction is its bind direction turned as the bone
//!   turned.
//! Each KH1 bone is turned about its own joint with its subtree, so KH1's link lengths stay.
//!
//! The matches are ours, worked out from the two bind poses and from which bones the two
//! games' motions actually turn (KH1: the spikes' middle bones, hood 80-83, chain 283-291;
//! KH2: the spikes' second bones, hood 72-73, weapon 3-11). See `docs/kh1-secondary-motion.md`.

use glam::{Mat4, Quat, Vec3};
use kh2_formats::anim::Bone;

/// KH1's head and the bone its hood hangs from (the neck base, which the chest carries).
pub const KH1_HEAD: usize = 151;
pub const KH1_NECK_BASE: usize = 78;
/// KH2's head and chest.
pub const KH2_HEAD: usize = 141;
pub const KH2_CHEST: usize = 68;
/// KH1's hood: the bones its motions turn, and the KH2 hood bone each follows (KH2's hood is
/// two bones from the chest, 72 → 73 straight back 7.8 in its bind, 73 a leaf; KH1's runs 4.3,
/// 7.9, 12.3 and 16 back from the neck base, KH2's 5 and 12.8 from its chest, so its first
/// half follows 72 and its second 73).
pub const HOOD: [(usize, usize); 4] = [(80, 72), (81, 72), (82, 73), (83, 73)];
/// KH1's keychain: 283 (at the pommel) to 291 (the charm, its tip 292); KH2's Keyblade
/// (`W_EX010`): 3 to 11, the same places in the bind.
pub const KH1_CHAIN: std::ops::RangeInclusive<usize> = 283..=291;
pub const KH2_CHAIN_FIRST: usize = 3;
/// The grip and the pommel, on both (KH1: Sora's bones; KH2: the weapon's).
pub const KH1_GRIP: usize = 280;
pub const KH1_POMMEL: usize = 283;
pub const KH2_GRIP: usize = 0;
pub const KH2_POMMEL: usize = 3;

/// The most a spike may turn from its bind relative to the head (KH1's own motions turn them
/// up to 173°).
const HAIR_MAX: f32 = 150.0 * std::f32::consts::PI / 180.0;
/// A spike's middle and tip may come no nearer the head's centre than this share of the nearest
/// its root, middle or tip is in the bind: further in, the spike would be inside the head.
const HAIR_IN: f32 = 0.9;
/// The hood's joints keep at least this far off the spine (KH1's rest pose has them 7-13 off,
/// which the user found clear of the back; KH2's idle lays its hood nearer, 5.8 on KH1).
const HOOD_CLEAR: f32 = 7.0;
/// ... and at least this far from the head's centre (the spike roots lie 8-13 from it).
const HOOD_HEAD: f32 = 10.0;

/// One KH1 link and the KH2 bone it follows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Link {
    /// The KH1 bone turned (with everything under it).
    pub kh1: usize,
    /// Its child: the link runs from `kh1` to here.
    pub tip: usize,
    /// The KH2 bone (body bones; on the chain, the weapon's).
    pub kh2: usize,
}

/// Which parts follow KH2.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Parts {
    pub hair: bool,
    pub hood: bool,
    pub chain: bool,
}

impl Parts {
    pub const ALL: Parts = Parts { hair: true, hood: true, chain: true };

    /// Whether this KH1 bone belongs to one of the parts set (hair 152-193, hood 79-84, chain
    /// 283-292).
    pub fn covers(&self, bone: usize) -> bool {
        (self.hair && (152..=193).contains(&bone)) || (self.hood && (79..=84).contains(&bone)) || (self.chain && (283..=292).contains(&bone))
    }
}

/// What one `apply` did (for checks and the log).
#[derive(Clone, Copy, Debug, Default)]
pub struct Report {
    pub parts: Parts,
    /// Spikes whose turn was held back short of the head, hood joints off the spine.
    pub clamped: usize,
}

pub struct Secondary {
    parent: Vec<Option<usize>>,
    /// Bones under each KH1 bone (index order).
    below: Vec<Vec<usize>>,
    kh1_bind: Vec<Mat4>,
    kh2_bind: Vec<Mat4>,
    weapon_bind: Vec<Mat4>,
    hair: Vec<Link>,
    /// Per spike: the second nearest KH2 spike and its share of the turn.
    hair_mix: Vec<(usize, f32)>,
    hood: Vec<Link>,
    chain: Vec<Link>,
    /// The head's centre (KH1 bind, the spike roots' middle) and, per spike, how near it the
    /// tip may come.
    head_centre: Vec3,
    hair_in: Vec<f32>,
}

fn rot(m: &Mat4) -> Quat {
    let (_, r, _) = m.to_scale_rotation_translation();
    r.normalize()
}

fn at(m: &Mat4) -> Vec3 {
    m.w_axis.truncate()
}

/// Children of each bone.
fn children(sk: &[Bone]) -> Vec<Vec<usize>> {
    let mut c = vec![Vec::new(); sk.len()];
    for (i, b) in sk.iter().enumerate() {
        if b.parent >= 0 && (b.parent as usize) < i {
            c[b.parent as usize].push(i);
        }
    }
    c
}

/// Spikes on a head: (root, moving bone, tip if any). A spike is a child of the head with a
/// child at the same place (the bone the motions turn); KH1's has a tip below that, KH2's none.
fn spikes(sk: &[Bone], bind: &[Mat4], head: usize) -> Vec<(usize, usize, Option<usize>)> {
    let kids = children(sk);
    let mut v = Vec::new();
    for &r in kids.get(head).map_or(&[][..], |k| k.as_slice()) {
        let [j] = kids[r][..] else { continue };
        if at(&bind[j]).distance(at(&bind[r])) > 0.5 {
            continue;
        }
        let tip = match kids[j][..] {
            [] => None,
            [t] => Some(t),
            _ => continue,
        };
        v.push((r, j, tip));
    }
    v
}

/// Positions relative to their middle, each axis scaled by its spread (two heads of different
/// sizes and proportions brought to one).
fn normalised(p: &[Vec3]) -> Vec<Vec3> {
    let n = p.len().max(1) as f32;
    let mid = p.iter().copied().sum::<Vec3>() / n;
    let spread = (p.iter().map(|v| (*v - mid) * (*v - mid)).sum::<Vec3>() / n).max(Vec3::splat(1e-4)).powf(0.5);
    p.iter().map(|v| (*v - mid) / spread).collect()
}

impl Secondary {
    /// `kh1`: KH1 Sora's skeleton; `kh2`: KH2 Sora's; `weapon`: KH2's Keyblade (`W_EX010`).
    pub fn new(kh1: &[Bone], kh2: &[Bone], weapon: Option<&[Bone]>) -> Secondary {
        let kh1_bind = kh2_anim::Pose::bind(kh1).bones;
        let kh2_bind = kh2_anim::Pose::bind(kh2).bones;
        let weapon_bind = weapon.map(|w| kh2_anim::Pose::bind(w).bones).unwrap_or_default();
        let parent: Vec<Option<usize>> = kh1.iter().enumerate().map(|(i, b)| (b.parent >= 0 && (b.parent as usize) < i).then_some(b.parent as usize)).collect();
        let kids = children(kh1);
        let mut below = vec![Vec::new(); kh1.len()];
        for i in (0..kh1.len()).rev() {
            let mut v: Vec<usize> = Vec::new();
            for &c in &kids[i] {
                v.push(c);
                v.extend(below[c].iter().copied());
            }
            v.sort_unstable();
            below[i] = v;
        }
        // Hair: each KH1 spike follows the KH2 spike nearest it, their roots compared on heads
        // brought to one size.
        let mut hair = Vec::new();
        let mut hair_in = Vec::new();
        let mut hair_mix = Vec::new();
        let mut head_centre = Vec3::ZERO;
        if KH1_HEAD < kh1.len() && KH2_HEAD < kh2.len() {
            let s1: Vec<_> = spikes(kh1, &kh1_bind, KH1_HEAD).into_iter().filter_map(|(r, j, t)| Some((r, j, t?))).collect();
            let s2 = spikes(kh2, &kh2_bind, KH2_HEAD);
            if !s1.is_empty() && !s2.is_empty() {
                let n1 = normalised(&s1.iter().map(|s| at(&kh1_bind[s.0])).collect::<Vec<_>>());
                let n2 = normalised(&s2.iter().map(|s| at(&kh2_bind[s.0])).collect::<Vec<_>>());
                head_centre = s1.iter().map(|s| at(&kh1_bind[s.0])).sum::<Vec3>() / s1.len() as f32;
                for (k, &(r, j, t)) in s1.iter().enumerate() {
                    let mut by: Vec<usize> = (0..s2.len()).collect();
                    by.sort_by(|&a, &b| n1[k].distance(n2[a]).total_cmp(&n1[k].distance(n2[b])));
                    let (d0, d1) = (n1[k].distance(n2[by[0]]), by.get(1).map_or(f32::MAX, |&b| n1[k].distance(n2[b])));
                    // The second nearest's share (inverse square distances): neighbouring KH1
                    // spikes that share their nearest KH2 one still move a little apart.
                    let share = if d1 == f32::MAX { 0.0 } else { let (w0, w1) = (1.0 / (d0 * d0 + 1e-3), 1.0 / (d1 * d1 + 1e-3)); w1 / (w0 + w1) };
                    hair.push(Link { kh1: j, tip: t, kh2: s2[by[0]].1 });
                    hair_mix.push((by.get(1).map_or(s2[by[0]].1, |&b| s2[b].1), share));
                    let (pr, pt) = (at(&kh1_bind[r]), at(&kh1_bind[t]));
                    let d = [pr, (pr + pt) * 0.5, pt].iter().map(|p| p.distance(head_centre)).fold(f32::MAX, f32::min);
                    hair_in.push(d * HAIR_IN);
                }
            }
        }
        let hood = if KH2_CHEST < kh2.len() && KH1_NECK_BASE < kh1.len() {
            HOOD.iter().filter(|(b, k)| *b + 1 < kh1.len() && *k < kh2.len()).map(|&(b, k)| Link { kh1: b, tip: b + 1, kh2: k }).collect()
        } else {
            Vec::new()
        };
        let chain = if weapon_bind.len() > KH2_CHAIN_FIRST + (KH1_CHAIN.end() - KH1_CHAIN.start()) && *KH1_CHAIN.end() + 1 < kh1.len() {
            KH1_CHAIN.map(|b| Link { kh1: b, tip: b + 1, kh2: KH2_CHAIN_FIRST + (b - KH1_CHAIN.start()) }).collect()
        } else {
            Vec::new()
        };
        Secondary { parent, below, kh1_bind, kh2_bind, weapon_bind, hair, hair_mix, hood, chain, head_centre, hair_in }
    }

    pub fn hair(&self) -> &[Link] {
        &self.hair
    }
    /// Per spike (as `hair`): the second nearest KH2 spike and its share of the turn.
    pub fn hair_mix(&self) -> &[(usize, f32)] {
        &self.hair_mix
    }

    /// Spike `k`'s turn relative to KH2's head: its nearest KH2 spike's, blended toward the
    /// second nearest's by its share.
    fn hair_turn(&self, k: usize, body: &[Mat4]) -> Option<Quat> {
        let a = Self::relative(body, &self.kh2_bind, KH2_HEAD, self.hair.get(k)?.kh2)?;
        let (other, share) = *self.hair_mix.get(k)?;
        let b = Self::relative(body, &self.kh2_bind, KH2_HEAD, other)?;
        Some(a.slerp(b, share).normalize())
    }
    pub fn hood(&self) -> &[Link] {
        &self.hood
    }
    pub fn chain(&self) -> &[Link] {
        &self.chain
    }

    /// Turns bone `b` and everything under it by `q` about its joint.
    fn turn(&self, pose: &mut [Mat4], b: usize, q: Quat) {
        if !q.is_finite() || b >= pose.len() {
            return;
        }
        let here = at(&pose[b]);
        let m = Mat4::from_translation(here) * Mat4::from_quat(q) * Mat4::from_translation(-here);
        pose[b] = m * pose[b];
        for &j in &self.below[b] {
            if j < pose.len() {
                pose[j] = m * pose[j];
            }
        }
    }

    /// A KH2 bone's turn from its bind, relative to its reference bone (`refr`), in the bind's
    /// axes: what it would be with the reference at rest.
    fn relative(now: &[Mat4], bind: &[Mat4], refr: usize, k: usize) -> Option<Quat> {
        let (rn, rb, kn, kb) = (rot(now.get(refr)?), rot(bind.get(refr)?), rot(now.get(k)?), rot(bind.get(k)?));
        let q = (rb * rn.inverse() * kn * kb.inverse()).normalize();
        q.is_finite().then_some(q)
    }

    /// The KH1 links as KH2's pose (`body`, model space; `weapon`: the Keyblade's bones in the
    /// same space) has them. `pose`: KH1's model-space bones as `Retarget` and the hand's
    /// adjustments left them. Only the `want`ed parts move; returns which did.
    pub fn apply(&self, pose: &mut [Mat4], body: &[Mat4], weapon: Option<&[Mat4]>, want: Parts) -> Report {
        let mut rep = Report::default();
        if pose.len() < self.kh1_bind.len() || !pose.iter().all(|m| m.is_finite()) {
            return rep;
        }
        if want.hair && !self.hair.is_empty() && body.len() > KH2_HEAD {
            rep.parts.hair = true;
            // KH1's head: its turn from the bind now.
            let head = (rot(&pose[KH1_HEAD]) * rot(&self.kh1_bind[KH1_HEAD]).inverse()).normalize();
            let head_m = pose[KH1_HEAD] * self.kh1_bind[KH1_HEAD].inverse();
            let centre = head_m.transform_point3(self.head_centre);
            for (k, (l, &near)) in self.hair.iter().zip(&self.hair_in).enumerate() {
                let Some(d) = self.hair_turn(k, body) else { continue };
                // Capped, then held back while the tip would go into the head.
                let d = cap(d, HAIR_MAX);
                let want_rot = |t: f32| (head * Quat::IDENTITY.slerp(d, t) * rot(&self.kh1_bind[l.kh1])).normalize();
                let joint = at(&pose[l.kh1]);
                let tip_rel = at(&pose[l.tip]) - joint;
                let cur = rot(&pose[l.kh1]);
                // Inside: the link's middle or tip nearer the centre than `near`.
                let inside = |t: f32| {
                    let v = (want_rot(t) * cur.inverse()) * tip_rel;
                    [0.5, 1.0].iter().any(|s| (joint + v * *s).distance(centre) < near)
                };
                let mut t = 1.0;
                if inside(1.0) {
                    rep.clamped += 1;
                    let (mut lo, mut hi) = (0.0f32, 1.0f32);
                    for _ in 0..8 {
                        let m = 0.5 * (lo + hi);
                        if inside(m) { hi = m } else { lo = m }
                    }
                    t = lo;
                }
                self.turn(pose, l.kh1, (want_rot(t) * cur.inverse()).normalize());
            }
        }
        if want.hood && !self.hood.is_empty() && body.len() > KH2_CHEST {
            rep.parts.hood = true;
            let chest = (rot(&pose[KH1_NECK_BASE]) * rot(&self.kh1_bind[KH1_NECK_BASE]).inverse()).normalize();
            // Each joint as KH2's hood turns it, relative to the chest.
            let target: Option<Vec<Quat>> = self.hood.iter().map(|l| Some((chest * Self::relative(body, &self.kh2_bind, KH2_CHEST, l.kh2)? * rot(&self.kh1_bind[l.kh1])).normalize())).collect();
            match target {
                Some(target) => {
                    for (l, want) in self.hood.iter().zip(&target) {
                        self.turn(pose, l.kh1, (*want * rot(&pose[l.kh1]).inverse()).normalize());
                    }
                    if self.keep_hood_clear(pose) {
                        rep.clamped += 1;
                    }
                }
                None => rep.parts.hood = false,
            }
        }
        if let (true, false, Some(w)) = (want.chain, self.chain.is_empty(), weapon) {
            if w.len() >= self.weapon_bind.len() && w.iter().all(|m| m.is_finite()) {
                rep.parts.chain = true;
                // KH2's handle onto KH1's: the swing between them.
                let h2 = (at(&w[KH2_POMMEL]) - at(&w[KH2_GRIP])).normalize_or_zero();
                let h1 = (at(&pose[KH1_POMMEL]) - at(&pose[KH1_GRIP])).normalize_or_zero();
                let onto = if h1 == Vec3::ZERO || h2 == Vec3::ZERO { Quat::IDENTITY } else { Quat::from_rotation_arc(h2, h1) };
                let last = self.chain.last().map(|l| l.kh2);
                for l in &self.chain {
                    let dir = if Some(l.kh2) == last {
                        // The charm: a leaf; its bind direction (along the chain) as it turned.
                        let bind_dir = (at(&self.weapon_bind[l.kh2]) - at(&self.weapon_bind[l.kh2 - 1])).normalize_or_zero();
                        (rot(&w[l.kh2]) * rot(&self.weapon_bind[l.kh2]).inverse()) * bind_dir
                    } else {
                        at(&w[l.kh2 + 1]) - at(&w[l.kh2])
                    };
                    let want_dir = (onto * dir).normalize_or_zero();
                    let cur = (at(&pose[l.tip]) - at(&pose[l.kh1])).normalize_or_zero();
                    if want_dir == Vec3::ZERO || cur == Vec3::ZERO || !want_dir.is_finite() {
                        continue;
                    }
                    self.turn(pose, l.kh1, Quat::from_rotation_arc(cur, want_dir));
                }
            }
        }
        rep
    }

    /// The hood off the back and the head: where a joint comes nearer the spine (71 → 78) than
    /// `HOOD_CLEAR`, or the head's centre than `HOOD_HEAD` (KH2's hood flips up over its head in
    /// spins), the whole hood swings about its root away, step by step (the joints
    /// turn on circles about the root, so one step falls short; a push off the back can also
    /// bring another joint near) until clear. Its own bends, KH2's, stay. Returns whether
    /// it had to.
    fn keep_hood_clear(&self, pose: &mut [Mat4]) -> bool {
        let Some(root) = self.hood.first().map(|l| l.kh1) else { return false };
        let head = pose[KH1_HEAD] * self.kh1_bind[KH1_HEAD].inverse();
        let centre = (!self.hair.is_empty()).then(|| head.transform_point3(self.head_centre));
        let mut moved = false;
        for _ in 0..8 {
            let (a, b) = (at(&pose[71]), at(&pose[KH1_NECK_BASE]));
            // The joint furthest inside, and where it should be.
            let mut worst: Option<(f32, Vec3, Vec3)> = None;
            for l in &self.hood {
                let j = at(&pose[l.tip]);
                let near = nearest_on_segment(j, a, b);
                let mut cands = vec![(HOOD_CLEAR - j.distance(near), j, (j - near).normalize_or_zero())];
                if let Some(c) = centre {
                    cands.push((HOOD_HEAD - j.distance(c), j, (j - c).normalize_or_zero()));
                }
                for (deficit, j, away) in cands {
                    if deficit > 1e-3 && away != Vec3::ZERO && worst.map_or(true, |w| deficit > w.0) {
                        worst = Some((deficit, j, j + away * deficit));
                    }
                }
            }
            let Some((_, j, to)) = worst else { break };
            let r = at(&pose[root]);
            let (from, to) = ((j - r).normalize_or_zero(), (to - r).normalize_or_zero());
            if from == Vec3::ZERO || to == Vec3::ZERO {
                break;
            }
            self.turn(pose, root, Quat::from_rotation_arc(from, to));
            moved = true;
        }
        moved
    }

    /// The direction each mapped KH1 link should have for this KH2 pose, computed apart from
    /// `apply` (for checks): hair and hood from KH1's head / neck base as posed in `pose`
    /// (before any cap), the chain from KH2's links.
    pub fn expected(&self, pose: &[Mat4], body: &[Mat4], weapon: Option<&[Mat4]>) -> Vec<(Link, Vec3)> {
        let mut v = Vec::new();
        let bind_dir = |l: &Link| (at(&self.kh1_bind[l.tip]) - at(&self.kh1_bind[l.kh1])).normalize_or_zero();
        let head = rot(&pose[KH1_HEAD]) * rot(&self.kh1_bind[KH1_HEAD]).inverse();
        for (k, l) in self.hair.iter().enumerate() {
            if let Some(d) = self.hair_turn(k, body) {
                v.push((*l, (head * d * bind_dir(l)).normalize()));
            }
        }
        let chest = rot(&pose[KH1_NECK_BASE]) * rot(&self.kh1_bind[KH1_NECK_BASE]).inverse();
        for l in &self.hood {
            if let Some(d) = Self::relative(body, &self.kh2_bind, KH2_CHEST, l.kh2) {
                v.push((*l, (chest * d * bind_dir(l)).normalize()));
            }
        }
        if let Some(w) = weapon {
            let h2 = (at(&w[KH2_POMMEL]) - at(&w[KH2_GRIP])).normalize_or_zero();
            let h1 = (at(&pose[KH1_POMMEL]) - at(&pose[KH1_GRIP])).normalize_or_zero();
            let onto = Quat::from_rotation_arc(h2, h1);
            for l in &self.chain {
                if l.kh2 + 1 < w.len() && l.kh2 + 1 < self.weapon_bind.len() {
                    v.push((*l, (onto * (at(&w[l.kh2 + 1]) - at(&w[l.kh2]))).normalize()));
                }
            }
        }
        v
    }

    /// KH1's parent of a bone (for checks).
    pub fn parent(&self, b: usize) -> Option<usize> {
        self.parent.get(b).copied().flatten()
    }
}

/// A turn limited to `max` radians.
fn cap(q: Quat, max: f32) -> Quat {
    let a = q.angle_between(Quat::IDENTITY);
    if a > max { Quat::IDENTITY.slerp(q, max / a).normalize() } else { q }
}

/// The point of the segment `a`-`b` nearest `p`.
fn nearest_on_segment(p: Vec3, a: Vec3, b: Vec3) -> Vec3 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    a + ab * t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone(parent: i16, t: [f32; 3]) -> Bone {
        Bone { index: 0, sibling: -1, parent, child: -1, flags: 0, scale: [1.0; 3], length: 0.0, rotation: [0.0; 3], translation: t }
    }

    /// Skeletons with only the bones `Secondary` reads (the rest at the root): KH1 with two
    /// spikes, the hood and the keychain; KH2 with two spikes and the hood; KH2's Keyblade.
    fn rigs() -> (Vec<Bone>, Vec<Bone>, Vec<Bone>) {
        let mut k1 = vec![bone(-1, [0.0; 3]); 293];
        for b in k1.iter_mut().skip(1) {
            b.parent = 0;
        }
        k1[71] = bone(0, [0.0, 103.0, 0.0]);
        k1[78] = bone(0, [0.0, 114.0, -4.0]);
        k1[79] = bone(78, [0.0, 0.0, -6.0]);
        k1[80] = bone(79, [0.0; 3]);
        k1[81] = bone(80, [0.0, 0.0, -4.0]);
        k1[82] = bone(81, [0.0, -1.0, -4.0]);
        k1[83] = bone(82, [0.0, -1.0, -4.0]);
        k1[84] = bone(83, [0.0, -2.0, -6.0]);
        k1[151] = bone(0, [0.0, 136.0, 0.0]);
        // Spikes: out along +x and -x from either side of the head.
        k1[152] = bone(151, [5.0, 5.0, 0.0]);
        k1[153] = bone(152, [0.0; 3]);
        k1[154] = bone(153, [10.0, 0.0, 0.0]);
        k1[155] = bone(151, [-5.0, 5.0, 0.0]);
        k1[156] = bone(155, [0.0; 3]);
        k1[157] = bone(156, [-10.0, 0.0, 0.0]);
        k1[101] = bone(0, [-60.0, 108.0, -2.0]);
        k1[280] = bone(101, [0.0; 3]);
        k1[281] = bone(280, [0.0; 3]);
        k1[282] = bone(281, [0.0; 3]);
        k1[283] = bone(282, [0.0, 0.0, -21.0]);
        for b in 284..=292 {
            k1[b] = bone(b as i16 - 1, [0.0, 0.0, -3.0]);
        }
        let mut k2 = vec![bone(-1, [0.0; 3]); 256];
        for b in k2.iter_mut().skip(1) {
            b.parent = 0;
        }
        k2[68] = bone(0, [0.0, 126.0, -4.0]);
        k2[71] = bone(68, [0.0, 0.0, -5.0]);
        k2[72] = bone(71, [0.0; 3]);
        k2[73] = bone(72, [0.0, 0.0, -8.0]);
        k2[141] = bone(0, [0.0, 142.0, 0.0]);
        k2[142] = bone(141, [6.0, 3.0, 0.0]);
        k2[143] = bone(142, [0.0; 3]);
        k2[144] = bone(141, [-6.0, 3.0, 0.0]);
        k2[145] = bone(144, [0.0; 3]);
        let mut w = vec![bone(-1, [-60.0, 108.0, -2.0]), bone(0, [0.0; 3]), bone(1, [0.0; 3]), bone(2, [0.0, 0.0, -21.0])];
        for b in 4..12 {
            w.push(bone(b as i16 - 1, [0.0, 0.0, -3.0]));
        }
        (k1, k2, w)
    }

    fn binds() -> (Vec<Mat4>, Vec<Mat4>, Vec<Mat4>, Secondary) {
        let (k1, k2, w) = rigs();
        let s = Secondary::new(&k1, &k2, Some(&w));
        (kh2_anim::Pose::bind(&k1).bones, kh2_anim::Pose::bind(&k2).bones, kh2_anim::Pose::bind(&w).bones, s)
    }

    fn dir(p: &[Mat4], a: usize, b: usize) -> Vec3 {
        (at(&p[b]) - at(&p[a])).normalize()
    }

    #[test]
    fn matches_from_the_binds() {
        let (_, _, _, s) = binds();
        // Each KH1 spike follows the KH2 spike on its side.
        assert_eq!(s.hair().iter().map(|l| (l.kh1, l.tip, l.kh2)).collect::<Vec<_>>(), vec![(153, 154, 143), (156, 157, 145)]);
        assert_eq!(s.hood().iter().map(|l| (l.kh1, l.kh2)).collect::<Vec<_>>(), HOOD.to_vec());
        assert_eq!(s.chain().len(), 9);
        assert_eq!((s.chain()[0].kh1, s.chain()[0].kh2, s.chain()[8].kh1, s.chain()[8].kh2), (283, 3, 291, 11));
    }

    #[test]
    fn a_spike_turns_as_kh2_s_relative_to_the_head() {
        let (b1, b2, _, s) = binds();
        // KH2's head turned a quarter round, its +x spike tipped up 30° on top of that.
        let turn = Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let about = |p: Vec3| Mat4::from_translation(p) * turn * Mat4::from_translation(-p);
        let mut body: Vec<Mat4> = b2.iter().map(|m| about(at(&b2[141])) * *m).collect();
        body[143] = body[143] * Mat4::from_rotation_z(30f32.to_radians());
        // KH1's head turned the same.
        let mut pose: Vec<Mat4> = b1.clone();
        for i in 151..=157 {
            pose[i] = about(at(&b1[151])) * pose[i];
        }
        let before = pose.clone();
        let rep = s.apply(&mut pose, &body, None, Parts::ALL);
        assert!(rep.parts.hair && !rep.parts.chain);
        let want = turn.transform_vector3(Quat::from_rotation_z(30f32.to_radians()) * Vec3::X);
        assert!(dir(&pose, 153, 154).angle_between(want) < 1e-3, "{:?} vs {want:?}", dir(&pose, 153, 154));
        assert!((at(&pose[154]).distance(at(&pose[153])) - 10.0).abs() < 1e-3);
        // The other spike's KH2 match didn't move: it stays.
        assert!(dir(&pose, 156, 157).angle_between(dir(&before, 156, 157)) < 1e-3);
    }

    #[test]
    fn a_spike_turned_into_the_head_is_held_back() {
        let (b1, b2, _, s) = binds();
        let mut body = b2.clone();
        // Folded straight back over the head (180° about z): capped, then held short of it.
        body[143] = body[143] * Mat4::from_rotation_z(std::f32::consts::PI);
        let mut pose = b1.clone();
        let rep = s.apply(&mut pose, &body, None, Parts::ALL);
        assert_eq!(rep.clamped, 1);
        let a = dir(&pose, 153, 154).angle_between(Vec3::X).to_degrees();
        assert!(a > 90.0 && a < 130.0, "turned {a}°");
        let centre = (at(&b1[152]) + at(&b1[155])) * 0.5;
        assert!(at(&pose[154]).distance(centre) >= 4.4 && ((at(&pose[154]) + at(&pose[153])) * 0.5).distance(centre) >= 4.4);
    }

    #[test]
    fn the_hood_folds_as_kh2_s() {
        let (b1, b2, _, s) = binds();
        let mut body = b2.clone();
        body[72] = body[72] * Mat4::from_rotation_x(-60f32.to_radians());
        body[73] = body[72] * b2[72].inverse() * b2[73];
        let mut pose = b1.clone();
        let rep = s.apply(&mut pose, &body, None, Parts { hood: true, ..Parts::default() });
        assert!(rep.parts.hood && !rep.parts.hair);
        let want = Quat::from_rotation_x(-60f32.to_radians()) * -Vec3::Z;
        assert!(dir(&pose, 80, 81).angle_between(want) < 1e-3, "{:?}", dir(&pose, 80, 81));
        for b in 80..84 {
            assert!((at(&pose[b + 1]).distance(at(&pose[b])) - at(&b1[b + 1]).distance(at(&b1[b]))).abs() < 1e-3);
        }
    }

    #[test]
    fn a_hood_turned_into_the_back_is_pushed_off_it() {
        let (b1, b2, _, s) = binds();
        let mut body = b2.clone();
        // Down and forward, through the back.
        body[72] = body[72] * Mat4::from_rotation_x(-150f32.to_radians());
        body[73] = body[72] * b2[72].inverse() * b2[73];
        let mut pose = b1.clone();
        let rep = s.apply(&mut pose, &body, None, Parts { hood: true, ..Parts::default() });
        assert_eq!(rep.clamped, 1);
        for b in 81..=84 {
            let j = at(&pose[b]);
            let d = j.distance(nearest_on_segment(j, at(&pose[71]), at(&pose[78])));
            assert!(d > HOOD_CLEAR - 0.5, "joint {b} {d} off the spine");
        }
        // The bends between the joints are as KH2's hood gives them still (here none: KH1's own).
        let bend = |p: &[Mat4]| dir(p, 80, 81).angle_between(dir(p, 82, 83));
        assert!((bend(&pose) - bend(&b1)).abs() < 1e-3);
    }

    #[test]
    fn the_chain_hangs_as_kh2_s_from_kh1_s_pommel() {
        let (b1, _, wb, s) = binds();
        // KH2's chain hanging straight down from its pommel, the charm too.
        let mut w = wb.clone();
        let down = Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        for i in 4..12 {
            w[i] = Mat4::from_translation(at(&wb[3]) - Vec3::Y * 3.0 * (i - 3) as f32) * down;
        }
        let mut pose = b1.clone();
        let rep = s.apply(&mut pose, &[], Some(&w), Parts::ALL);
        assert!(rep.parts.chain && !rep.parts.hair && !rep.parts.hood);
        assert!(at(&pose[283]).distance(at(&b1[283])) < 1e-4, "the pommel stays");
        for b in 283..=291 {
            assert!(dir(&pose, b, b + 1).angle_between(-Vec3::Y) < 1e-3, "link {b}: {:?}", dir(&pose, b, b + 1));
            assert!((at(&pose[b + 1]).distance(at(&pose[b])) - at(&b1[b + 1]).distance(at(&b1[b]))).abs() < 1e-3);
        }
        // Without KH2's bones nothing moves.
        let mut p2 = b1.clone();
        assert_eq!(s.apply(&mut p2, &[], None, Parts::ALL).parts, Parts::default());
        assert!(p2 == b1);
    }

    #[test]
    fn parts_cover_their_bones() {
        let p = Parts { hair: true, ..Parts::default() };
        assert!(p.covers(153) && !p.covers(80) && !p.covers(285));
        assert!(Parts::ALL.covers(80) && Parts::ALL.covers(292) && !Parts::ALL.covers(151) && !Parts::ALL.covers(282));
    }
}
