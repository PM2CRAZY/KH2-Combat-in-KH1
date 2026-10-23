//! KH1 Sora's hair, hood and keychain following KH2's own animation of those parts, while KH2's
//! moves pose him (the user: "add the animations for sora's hair and chain on each KH2
//! animation instead [of the physics], don't affect the KH1 animations").
//!
//! KH2 animates these parts by hand in every motion: eleven hair spikes on the head, the hood
//! on the back, and the Keyblade's chain (the weapon's own motion, same id). `Retarget` turns
//! the body only, so KH1's spikes, hood and chain used to keep their rest shape (or swing by a
//! physics swing of ours, removed in t388: KH2 has none). Here each KH1 link takes the movement
//! of the KH2 bone matched to it:
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
//! - **Sleeves: the sleeve as KH2's hangs, each flap as KH2's flaps.** KH2 keys each sleeve by
//!   hand: a root (111 / 203) turning about the shoulder against the upper arm, and two flaps
//!   on it (113 front, 115 back; 205 / 207). KH1's sleeve is four chains (front, under, back,
//!   top) of a joint at the shoulder and one part way down. The joints at the shoulder take
//!   the KH2 root's turn in model space, carried by the swing from KH2's upper arm to KH1's
//!   (KH1's upper arm points along KH2's, so this is nearly none); the joints down the sleeve
//!   take the flap's turn relative to the root, on KH1's root as just posed. The under and top
//!   chains blend the two flaps (by where they sit round the arm in the binds).
//! - **Necklace: the turn, measured against the chest.** KH2's necklace is one bone (165, from
//!   the chest 68, with its own mesh); KH1's link from the neck base (86) takes its turn as the
//!   hood does, the pendant below it (87) keeping its own relation to it. Guard: it is held off
//!   the chest.
//! Each KH1 bone is turned about its own joint with its subtree, so KH1's link lengths stay.
//!
//! **Squash and stretch** (`scales`): KH2's scale on a matched bone (its own, against its parent)
//! goes on the KH1 bone matched to it, laid onto the KH1 bone's axes. Both games scale a bone the
//! same way (KH1 `0x1403900B0` / `0x140394CF0`, KH2 `kh2_anim`): a bone's scale is its parent's
//! times its own, and it stretches that bone's axes only, never where its children sit. So a KH1
//! bone takes the scale of the KH2 bone it turns with, and its subtree carries it as KH2's does.
//!
//! The matches are ours, worked out from the two bind poses and from which bones the two
//! games' motions actually turn (KH1: the spikes' middle bones, hood 80-83, chain 283-291;
//! KH2: the spikes' second bones, hood 72-73, weapon 3-11; sleeves: KH1 129/130, 133/134,
//! 137/138, 141/142 and 233/234 ... 245/246, KH2 110, 113, 115 and 202, 205, 207; necklace: KH1
//! 86, KH2 165). See `docs/kh1-secondary-motion.md`.

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
/// KH1's necklace link (86, at the front of the neck base, its pendant 87 and tip 88 below) and
/// KH2's necklace (165, a leaf from the chest at its centre, the only KH2 bone its mesh is on:
/// the pendant hangs 3-12 in front of the chest). Both bind pointing down and forward (KH1's
/// link 50° below level, KH2's bone's x 56°).
pub const NECKLACE: (usize, usize) = (86, 165);
/// KH1's keychain: 283 (at the pommel) to 291 (the charm, its tip 292); KH2's Keyblade
/// (`W_EX010`): 3 to 11, the same places in the bind.
pub const KH1_CHAIN: std::ops::RangeInclusive<usize> = 283..=291;
pub const KH2_CHAIN_FIRST: usize = 3;
/// The grip and the pommel, on both (KH1: Sora's bones; KH2: the weapon's).
pub const KH1_GRIP: usize = 280;
pub const KH1_POMMEL: usize = 283;
pub const KH2_GRIP: usize = 0;
pub const KH2_POMMEL: usize = 3;

/// One side's sleeve: KH1's upper arm and elbow and its four chain roots (front, under, back,
/// top; each root, a joint at the same place that the motions turn, one part way down, a tip);
/// KH2's upper arm and elbow, the sleeve's root (turned about the shoulder through its parent)
/// and its two flaps (pivot, then the leaf the motions turn: front, back).
struct SleeveSide {
    kh1_arm: usize,
    kh1_elbow: usize,
    kh1_roots: [usize; 4],
    kh2_arm: usize,
    kh2_elbow: usize,
    kh2_root: usize,
    kh2_flaps: [(usize, usize); 2],
}

/// +x (KH1 198, KH2 78) and -x (KH1 93, KH2 170): KH2's -x side is +92.
const SLEEVES: [SleeveSide; 2] = [
    SleeveSide { kh1_arm: 198, kh1_elbow: 199, kh1_roots: [232, 236, 240, 244], kh2_arm: 78, kh2_elbow: 79, kh2_root: 111, kh2_flaps: [(112, 113), (114, 115)] },
    SleeveSide { kh1_arm: 93, kh1_elbow: 94, kh1_roots: [128, 132, 136, 140], kh2_arm: 170, kh2_elbow: 171, kh2_root: 203, kh2_flaps: [(204, 205), (206, 207)] },
];

/// One KH1 sleeve chain and what it follows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sleeve {
    /// The joint at the shoulder (turned with everything under it), the one part way down,
    /// and the tip.
    pub joint: usize,
    pub mid: usize,
    pub tip: usize,
    /// KH1's and KH2's upper arm and elbow.
    pub kh1_arm: (usize, usize),
    pub kh2_arm: (usize, usize),
    /// KH2's sleeve root.
    pub kh2_root: usize,
    /// KH2's two flap leaves and the second's share of the flap turn.
    pub flaps: (usize, usize),
    pub share: f32,
}

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
/// The necklace's pendant and tip keep at least this share of their bind distance from the
/// spine (71 → 78): the chest's front lies about there.
const NECKLACE_CLEAR: f32 = 0.85;

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
    pub sleeves: bool,
    pub necklace: bool,
}

impl Parts {
    pub const ALL: Parts = Parts { hair: true, hood: true, chain: true, sleeves: true, necklace: true };

    /// Whether this KH1 bone belongs to one of the parts set (hair 152-193, hood 79-84, chain
    /// 283-292, sleeves 128-143 and 232-247, necklace 86-88).
    pub fn covers(&self, bone: usize) -> bool {
        (self.hair && (152..=193).contains(&bone))
            || (self.hood && (79..=84).contains(&bone))
            || (self.chain && (283..=292).contains(&bone))
            || (self.sleeves && ((128..=143).contains(&bone) || (232..=247).contains(&bone)))
            || (self.necklace && (86..=88).contains(&bone))
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
    /// KH2's and its Keyblade's parents (for each bone's own scale).
    kh2_parent: Vec<Option<usize>>,
    weapon_parent: Vec<Option<usize>>,
    hair: Vec<Link>,
    /// Per spike: the second nearest KH2 spike and its share of the turn.
    hair_mix: Vec<(usize, f32)>,
    hood: Vec<Link>,
    chain: Vec<Link>,
    sleeves: Vec<Sleeve>,
    /// The necklace's link (86 → 87, KH2 165) and how near the spine its pendant and tip may
    /// come.
    necklace: Option<(Link, [f32; 2])>,
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
        let parents = |sk: &[Bone]| -> Vec<Option<usize>> { sk.iter().enumerate().map(|(i, b)| (b.parent >= 0 && (b.parent as usize) < i).then_some(b.parent as usize)).collect() };
        let parent = parents(kh1);
        let (kh2_parent, weapon_parent) = (parents(kh2), weapon.map(parents).unwrap_or_default());
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
        let sleeves = sleeve_links(kh1, kh2, &kh1_bind, &kh2_bind);
        // The necklace: KH1's link from the neck base with a pendant and a tip below it; KH2's
        // bone a leaf under the chest's child.
        let (n1, n2) = NECKLACE;
        let necklace = (n1 + 2 < kh1.len()
            && kh1[n1 + 1].parent == n1 as i16
            && kh1[n1 + 2].parent == n1 as i16 + 1
            && n2 < kh2.len()
            && kh2[n2].parent >= 0
            && kh2.get(kh2[n2].parent as usize).is_some_and(|p| p.parent == KH2_CHEST as i16)
            && 78 < kh1.len())
        .then(|| {
            let clear = [n1 + 1, n1 + 2].map(|j| at(&kh1_bind[j]).distance(nearest_on_segment(at(&kh1_bind[j]), at(&kh1_bind[71]), at(&kh1_bind[KH1_NECK_BASE]))) * NECKLACE_CLEAR);
            (Link { kh1: n1, tip: n1 + 1, kh2: n2 }, clear)
        });
        Secondary { parent, below, kh1_bind, kh2_bind, weapon_bind, kh2_parent, weapon_parent, hair, hair_mix, hood, chain, sleeves, necklace, head_centre, hair_in }
    }

    pub fn sleeves(&self) -> &[Sleeve] {
        &self.sleeves
    }

    /// The necklace's link and the KH2 bone it follows.
    pub fn necklace(&self) -> Option<Link> {
        self.necklace.map(|n| n.0)
    }

    /// A sleeve chain's two turns (model space, from KH1's bind): the joint at the shoulder (as
    /// KH2's sleeve root, carried by the swing from KH2's upper arm onto KH1's as posed in
    /// `pose`), then the joint down the sleeve with that root applied (`None` where KH2's bones
    /// can't be read).
    fn sleeve_turns(&self, sl: &Sleeve, pose: &[Mat4], body: &[Mat4]) -> Option<(Quat, Quat)> {
        let dir = |p: &[Mat4], (a, b): (usize, usize)| Some((at(p.get(b)?) - at(p.get(a)?)).normalize_or_zero());
        let (d2, d1) = (dir(body, sl.kh2_arm)?, dir(pose, sl.kh1_arm)?);
        let onto = if d1 == Vec3::ZERO || d2 == Vec3::ZERO { Quat::IDENTITY } else { Quat::from_rotation_arc(d2, d1) };
        let root = (onto * rot(body.get(sl.kh2_root)?) * rot(self.kh2_bind.get(sl.kh2_root)?).inverse()).normalize();
        let a = Self::relative(body, &self.kh2_bind, sl.kh2_root, sl.flaps.0)?;
        let b = Self::relative(body, &self.kh2_bind, sl.kh2_root, sl.flaps.1)?;
        let flap = a.slerp(b, sl.share).normalize();
        let joint = (root * rot(&self.kh1_bind[sl.joint])).normalize();
        // The mid joint: the root's turn, then the flap's (both in the bind's axes).
        let mid = (root * flap * rot(&self.kh1_bind[sl.mid])).normalize();
        (joint.is_finite() && mid.is_finite()).then_some((joint, mid))
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
        if want.sleeves && !self.sleeves.is_empty() {
            let mut any = false;
            for sl in &self.sleeves {
                let Some((joint, mid)) = self.sleeve_turns(sl, pose, body) else { continue };
                self.turn(pose, sl.joint, (joint * rot(&pose[sl.joint]).inverse()).normalize());
                self.turn(pose, sl.mid, (mid * rot(&pose[sl.mid]).inverse()).normalize());
                any = true;
            }
            rep.parts.sleeves = any;
        }
        if let (true, Some((l, clear)), true) = (want.necklace, self.necklace, body.len() > KH2_CHEST.max(NECKLACE.1)) {
            let chest = (rot(&pose[KH1_NECK_BASE]) * rot(&self.kh1_bind[KH1_NECK_BASE]).inverse()).normalize();
            if let Some(d) = Self::relative(body, &self.kh2_bind, KH2_CHEST, l.kh2) {
                rep.parts.necklace = true;
                let target = (chest * d * rot(&self.kh1_bind[l.kh1])).normalize();
                self.turn(pose, l.kh1, (target * rot(&pose[l.kh1]).inverse()).normalize());
                if self.keep_necklace_clear(pose, l.kh1, clear) {
                    rep.clamped += 1;
                }
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

    /// Each matched KH1 bone's own scale (against its parent, KH1's bone layout: `apply_scale`)
    /// as KH2's pose gives its match: written into `out` (one per KH1 bone) for the parts `want`
    /// asks for; other bones are left as they are. The hair, hood and chain take it on the first
    /// KH1 bone each KH2 bone turns (its subtree carries it); a sleeve's shoulder joint its root's,
    /// the joint down the sleeve its flaps' (blended as their turns); the necklace's link 165's.
    pub fn scales(&self, body: &[Mat4], weapon: Option<&[Mat4]>, want: Parts, out: &mut [Vec3]) {
        let mut put = |kh1: usize, s: Option<Vec3>| {
            if let (Some(s), Some(o)) = (s, out.get_mut(kh1)) {
                if s.is_finite() && s.min_element() > 0.0 {
                    *o = s;
                }
            }
        };
        let body_s = |k: usize, kh1: usize| self.own_scale(body, &self.kh2_parent, &self.kh2_bind, k, kh1);
        if want.hair {
            for (k, l) in self.hair.iter().enumerate() {
                let (other, share) = self.hair_mix[k];
                put(l.kh1, Some(body_s(l.kh2, l.kh1).unwrap_or(Vec3::ONE).lerp(body_s(other, l.kh1).unwrap_or(Vec3::ONE), share)));
            }
        }
        if want.hood {
            let mut last = None;
            for l in &self.hood {
                if last != Some(l.kh2) {
                    put(l.kh1, body_s(l.kh2, l.kh1));
                    last = Some(l.kh2);
                }
            }
        }
        if want.sleeves {
            for sl in &self.sleeves {
                put(sl.joint, body_s(sl.kh2_root, sl.joint));
                let (a, b) = (body_s(sl.flaps.0, sl.mid), body_s(sl.flaps.1, sl.mid));
                put(sl.mid, a.zip(b).map(|(a, b)| a.lerp(b, sl.share)));
            }
        }
        if let (true, Some((l, _))) = (want.necklace, self.necklace) {
            put(l.kh1, body_s(l.kh2, l.kh1));
        }
        if let (true, Some(w)) = (want.chain, weapon) {
            for l in &self.chain {
                put(l.kh1, self.own_scale(w, &self.weapon_parent, &self.weapon_bind, l.kh2, l.kh1));
            }
        }
    }

    /// A KH2 bone's own scale in `pose` (its scale over its parent's), laid onto KH1 bone
    /// `kh1`'s axes: each KH1 axis takes KH2's three scales weighted by how much of it lies
    /// along each KH2 axis (squared, in the two binds, which face the same way). The same scale
    /// when the axes match.
    fn own_scale(&self, pose: &[Mat4], parents: &[Option<usize>], bind: &[Mat4], k: usize, kh1: usize) -> Option<Vec3> {
        let acc = |i: usize| pose.get(i).map(|m| m.to_scale_rotation_translation().0);
        let s = match parents.get(k).copied().flatten() {
            Some(p) => acc(k)? / acc(p)?.max(Vec3::splat(1e-6)),
            None => acc(k)?,
        };
        if (s - Vec3::ONE).abs().max_element() < 1e-5 {
            return Some(Vec3::ONE);
        }
        let (r2, r1) = (rot(bind.get(k)?), rot(self.kh1_bind.get(kh1)?));
        let along = |e: Vec3| {
            let v = r2.inverse() * (r1 * e);
            v.x * v.x * s.x + v.y * v.y * s.y + v.z * v.z * s.z
        };
        Some(Vec3::new(along(Vec3::X), along(Vec3::Y), along(Vec3::Z)))
    }

    /// The necklace off the chest: where its pendant (`link + 1`) or tip comes nearer the spine
    /// (71 → 78) than `clear`, the link swings about its joint away from it, step by step, until
    /// clear (KH2's necklace hangs from the middle of its chest, KH1's from the front of its neck,
    /// so a swing back that clears KH2's chest can reach into KH1's). Returns whether it had to.
    fn keep_necklace_clear(&self, pose: &mut [Mat4], link: usize, clear: [f32; 2]) -> bool {
        let mut moved = false;
        for _ in 0..8 {
            let (a, b) = (at(&pose[71]), at(&pose[KH1_NECK_BASE]));
            let mut worst: Option<(f32, Vec3, Vec3)> = None;
            for (j, c) in [link + 1, link + 2].into_iter().zip(clear) {
                let p = at(&pose[j]);
                let near = nearest_on_segment(p, a, b);
                let (deficit, away) = (c - p.distance(near), (p - near).normalize_or_zero());
                if deficit > 1e-3 && away != Vec3::ZERO && worst.map_or(true, |w| deficit > w.0) {
                    worst = Some((deficit, p, p + away * deficit));
                }
            }
            let Some((_, p, to)) = worst else { break };
            let r = at(&pose[link]);
            let (from, to) = ((p - r).normalize_or_zero(), (to - r).normalize_or_zero());
            if from == Vec3::ZERO || to == Vec3::ZERO {
                break;
            }
            self.turn(pose, link, Quat::from_rotation_arc(from, to));
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
        if let Some((l, _)) = self.necklace {
            if let Some(d) = Self::relative(body, &self.kh2_bind, KH2_CHEST, l.kh2) {
                v.push((l, (chest * d * bind_dir(&l)).normalize()));
            }
        }
        for sl in &self.sleeves {
            if let Some((joint, mid)) = self.sleeve_turns(sl, pose, body) {
                let d = |q: Quat, a: usize, b: usize| (q * rot(&self.kh1_bind[a]).inverse() * (at(&self.kh1_bind[b]) - at(&self.kh1_bind[a]))).normalize();
                v.push((Link { kh1: sl.joint, tip: sl.mid, kh2: sl.kh2_root }, d(joint, sl.joint, sl.mid)));
                v.push((Link { kh1: sl.mid, tip: sl.tip, kh2: sl.flaps.0 }, d(mid, sl.mid, sl.tip)));
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

/// The sleeve chains found on both skeletons: each side's KH1 chains (root → joint at the same
/// place → mid → tip, the root under the upper arm) and KH2's sleeve (root under the upper arm's
/// chain, each flap pivot under the root and its leaf under that). A side either lacks is left
/// out. Each chain's flap share: where its root and the two flap pivots sit round the arm in
/// the binds (directions square to the arm), blended by inverse squared distance, as the hair.
fn sleeve_links(kh1: &[Bone], kh2: &[Bone], b1: &[Mat4], b2: &[Mat4]) -> Vec<Sleeve> {
    let p1 = |i: usize| kh1.get(i).map(|b| b.parent as i64);
    let p2 = |i: usize| kh2.get(i).map(|b| b.parent as i64);
    let mut v = Vec::new();
    for s in &SLEEVES {
        let kh2_ok = s.kh2_flaps.iter().all(|&(pivot, leaf)| p2(pivot) == Some(s.kh2_root as i64) && p2(leaf) == Some(pivot as i64)) && p2(s.kh2_elbow).is_some() && p2(s.kh2_arm).is_some();
        let kh1_ok = s.kh1_roots.iter().all(|&r| p1(r) == Some(s.kh1_arm as i64) && p1(r + 1) == Some(r as i64) && p1(r + 2) == Some(r as i64 + 1) && p1(r + 3) == Some(r as i64 + 2)) && p1(s.kh1_elbow).is_some();
        if !kh1_ok || !kh2_ok {
            continue;
        }
        // Round the arm: the offset from the shoulder, square to the arm, made unit.
        let round = |b: &[Mat4], arm: usize, elbow: usize, i: usize| {
            let axis = (at(&b[elbow]) - at(&b[arm])).normalize_or_zero();
            let o = at(&b[i]) - at(&b[arm]);
            (o - axis * o.dot(axis)).normalize_or_zero()
        };
        let f: Vec<Vec3> = s.kh2_flaps.iter().map(|&(pivot, _)| round(b2, s.kh2_arm, s.kh2_elbow, pivot)).collect();
        for &r in &s.kh1_roots {
            let o = round(b1, s.kh1_arm, s.kh1_elbow, r);
            let (d0, d1) = (o.distance(f[0]), o.distance(f[1]));
            let (w0, w1) = (1.0 / (d0 * d0 + 1e-3), 1.0 / (d1 * d1 + 1e-3));
            v.push(Sleeve {
                joint: r + 1,
                mid: r + 2,
                tip: r + 3,
                kh1_arm: (s.kh1_arm, s.kh1_elbow),
                kh2_arm: (s.kh2_arm, s.kh2_elbow),
                kh2_root: s.kh2_root,
                flaps: (s.kh2_flaps[0].1, s.kh2_flaps[1].1),
                share: w1 / (w0 + w1),
            });
        }
    }
    v
}

/// Puts each bone's own scale (`local`, against its parent) on a model-space pose as both games
/// draw it: a bone's scale is its parent's times its own, stretching its axes only (its place and
/// its children's stay). `pose` is unscaled (as `Retarget` and `apply` give it); a bone whose
/// scale works out at 1 is left exactly as it was.
pub fn apply_scale(pose: &mut [Mat4], parents: &[i16], local: &[Vec3]) {
    if local.iter().all(|s| *s == Vec3::ONE) {
        return;
    }
    // (On the stack: this runs every frame Sora is posed.)
    let mut acc = [Vec3::ONE; 512];
    for i in 0..pose.len().min(acc.len()) {
        let own = local.get(i).copied().unwrap_or(Vec3::ONE);
        let s = match parents.get(i) {
            Some(&p) if p >= 0 && (p as usize) < i => acc[p as usize] * own,
            _ => own,
        };
        acc[i] = s;
        if s != Vec3::ONE && s.is_finite() {
            let m = &mut pose[i];
            m.x_axis *= s.x;
            m.y_axis *= s.y;
            m.z_axis *= s.z;
        }
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

    /// `rigs` with the +x sleeve on both: KH1's upper arm 198 → elbow 199 and its four chains
    /// (front, under, back, top) along the arm; KH2's upper arm 78 → elbow 79, the sleeve root
    /// 111 at the shoulder (under 109 and 110) and its front and back flaps.
    fn sleeve_rigs() -> (Vec<Bone>, Vec<Bone>) {
        let (mut k1, mut k2, _) = rigs();
        k1[198] = bone(0, [12.0, 112.0, -3.0]);
        k1[199] = bone(198, [19.0, 0.0, 0.0]);
        for (r, off) in [(232usize, [0.0, 0.0, 3.0]), (236, [0.0, -5.0, 0.0]), (240, [0.0, 0.0, -3.0]), (244, [0.0, 3.0, 0.0])] {
            k1[r] = bone(198, off);
            k1[r + 1] = bone(r as i16, [0.0; 3]);
            k1[r + 2] = bone(r as i16 + 1, [9.0, 0.0, 0.0]);
            k1[r + 3] = bone(r as i16 + 2, [6.0, 0.0, 0.0]);
        }
        k2[78] = bone(0, [10.0, 121.0, -5.0]);
        k2[79] = bone(78, [22.0, 0.0, 0.0]);
        k2[109] = bone(78, [22.0, 0.0, 0.0]);
        k2[110] = bone(109, [-22.0, 0.0, 0.0]);
        k2[111] = bone(110, [0.0; 3]);
        k2[112] = bone(111, [2.5, -0.8, 3.2]);
        k2[113] = bone(112, [0.0; 3]);
        k2[114] = bone(111, [2.5, -0.8, -3.0]);
        k2[115] = bone(114, [0.0; 3]);
        (k1, k2)
    }

    #[test]
    fn sleeves_match_their_flaps_round_the_arm() {
        let (k1, k2) = sleeve_rigs();
        let s = Secondary::new(&k1, &k2, None);
        let v: Vec<(usize, f32)> = s.sleeves().iter().map(|l| (l.joint, l.share)).collect();
        assert_eq!(v.iter().map(|x| x.0).collect::<Vec<_>>(), vec![233, 237, 241, 245]);
        // Front mostly the front flap, back mostly the back one, under and top between.
        assert!(v[0].1 < 0.1 && v[2].1 > 0.9, "{v:?}");
        assert!((v[1].1 - 0.5).abs() < 0.1 && (v[3].1 - 0.5).abs() < 0.1, "{v:?}");
        // The default rigs have no sleeve: none found.
        let (a, b, _) = rigs();
        assert!(Secondary::new(&a, &b, None).sleeves().is_empty());
    }

    #[test]
    fn a_sleeve_hangs_and_flaps_as_kh2_s() {
        let (k1, k2) = sleeve_rigs();
        let s = Secondary::new(&k1, &k2, None);
        let (b1, b2) = (kh2_anim::Pose::bind(&k1).bones, kh2_anim::Pose::bind(&k2).bones);
        // KH2's sleeve root dropped 30° about z at the shoulder, its front flap 20° more about y.
        let about = |p: Vec3, q: Quat| Mat4::from_translation(p) * Mat4::from_quat(q) * Mat4::from_translation(-p);
        let (drop, flap) = (Quat::from_rotation_z(-30f32.to_radians()), Quat::from_rotation_y(20f32.to_radians()));
        let mut body = b2.clone();
        for i in [111, 112, 113, 114, 115] {
            body[i] = about(at(&b2[111]), drop) * body[i];
        }
        body[113] = about(at(&body[113]), drop * flap * drop.inverse()) * body[113];
        let mut pose = b1.clone();
        let rep = s.apply(&mut pose, &body, None, Parts { sleeves: true, ..Parts::default() });
        assert!(rep.parts.sleeves && !rep.parts.hair && !rep.parts.chain);
        // Every chain's shoulder joint drops with KH2's root.
        for r in [233, 237, 241, 245] {
            assert!(dir(&pose, r, r + 1).angle_between(drop * Vec3::X) < 1e-3, "{r}: {:?}", dir(&pose, r, r + 1));
            for b in r..=r + 1 {
                assert!((at(&pose[b + 1]).distance(at(&pose[b])) - at(&b1[b + 1]).distance(at(&b1[b]))).abs() < 1e-3);
            }
        }
        // The front chain's mid joint takes the front flap; the back chain's stays with the root.
        assert!(dir(&pose, 234, 235).angle_between(drop * flap * Vec3::X) < 1e-2, "{:?}", dir(&pose, 234, 235));
        assert!(dir(&pose, 242, 243).angle_between(drop * Vec3::X) < 1e-2);
        // The arm and the rest stay.
        assert!(pose[198] == b1[198] && pose[199] == b1[199]);
        for (l, want) in s.expected(&b1, &body, None) {
            assert!(dir(&pose, l.kh1, l.tip).angle_between(want) < 1e-3, "{l:?}");
        }
    }

    /// `rigs` with the necklace on both: KH1's 85 at the front of the neck base, its link 86, the
    /// pendant 87 and tip 88 hanging down in front; KH2's 164 / 165 from the chest's middle.
    fn necklace_rigs() -> (Vec<Bone>, Vec<Bone>) {
        let (mut k1, mut k2, _) = rigs();
        k1[85] = bone(78, [0.0, -1.0, 7.0]);
        k1[86] = bone(85, [0.0; 3]);
        k1[87] = bone(86, [0.0, -4.0, 5.0]);
        k1[88] = bone(87, [0.0, -6.0, 1.0]);
        k2[164] = bone(68, [0.0, 0.0, -1.0]);
        k2[165] = bone(164, [0.0; 3]);
        (k1, k2)
    }

    #[test]
    fn the_necklace_swings_as_kh2_s() {
        let (k1, k2) = necklace_rigs();
        let s = Secondary::new(&k1, &k2, None);
        assert_eq!(s.necklace().map(|l| (l.kh1, l.tip, l.kh2)), Some((86, 87, 165)));
        let (b1, b2) = (kh2_anim::Pose::bind(&k1).bones, kh2_anim::Pose::bind(&k2).bones);
        // KH2's necklace swung 40° forward (up, away from the chest) about x.
        let mut body = b2.clone();
        let fwd = Quat::from_rotation_x(-40f32.to_radians());
        body[165] = Mat4::from_translation(at(&b2[165])) * Mat4::from_quat(fwd) * Mat4::from_translation(-at(&b2[165])) * body[165];
        let mut pose = b1.clone();
        let rep = s.apply(&mut pose, &body, None, Parts { necklace: true, ..Parts::default() });
        assert!(rep.parts.necklace && rep.clamped == 0, "{rep:?}");
        assert!(dir(&pose, 86, 87).angle_between(fwd * dir(&b1, 86, 87)) < 1e-3, "{:?}", dir(&pose, 86, 87));
        let want = s.expected(&b1, &body, None).into_iter().find(|(l, _)| l.kh1 == 86).map(|e| e.1).unwrap();
        assert!(dir(&pose, 86, 87).angle_between(want) < 1e-3);
        // The pendant keeps its own bend on the link, and every length.
        assert!(dir(&pose, 87, 88).angle_between(fwd * dir(&b1, 87, 88)) < 1e-3);
        for b in 86..88 {
            assert!((at(&pose[b + 1]).distance(at(&pose[b])) - at(&b1[b + 1]).distance(at(&b1[b]))).abs() < 1e-3);
        }
        // Swung back hard (into the chest): held off the spine.
        let mut body = b2.clone();
        let back = Quat::from_rotation_x(100f32.to_radians());
        body[165] = Mat4::from_translation(at(&b2[165])) * Mat4::from_quat(back) * Mat4::from_translation(-at(&b2[165])) * body[165];
        let mut pose = b1.clone();
        let rep = s.apply(&mut pose, &body, None, Parts { necklace: true, ..Parts::default() });
        assert!(rep.clamped == 1);
        for j in [87, 88] {
            let p = at(&pose[j]);
            let bind = at(&b1[j]).distance(nearest_on_segment(at(&b1[j]), at(&b1[71]), at(&b1[78])));
            assert!(p.distance(nearest_on_segment(p, at(&pose[71]), at(&pose[78]))) > bind * NECKLACE_CLEAR - 0.05, "{j}");
        }
        // Without KH2's necklace bone (the default rigs) there is none.
        let (a, b, _) = rigs();
        assert!(Secondary::new(&a, &b, None).necklace().is_none());
    }

    #[test]
    fn a_flap_s_squash_goes_on_its_sleeve() {
        let (k1, k2) = sleeve_rigs();
        let s = Secondary::new(&k1, &k2, None);
        let b2 = kh2_anim::Pose::bind(&k2).bones;
        // KH2's front flap squashed to 0.7 along its x (KH2 multiplies scale down the bones:
        // the flap's own is its scale over its pivot's).
        let mut body = b2.clone();
        body[113].x_axis *= 0.7;
        let mut out = vec![Vec3::ONE; k1.len()];
        s.scales(&body, None, Parts { sleeves: true, ..Parts::default() }, &mut out);
        // The front chain's joint down the sleeve takes it (its axes match the flap's here), the
        // back chain's not; the under and top chains about half; the shoulder joints none.
        assert!((out[234] - Vec3::new(0.7, 1.0, 1.0)).length() < 0.02, "{:?}", out[234]);
        assert!((out[242] - Vec3::ONE).length() < 0.02, "{:?}", out[242]);
        assert!((out[238].x - 0.85).abs() < 0.05, "{:?}", out[238]);
        assert!(out[233] == Vec3::ONE && out[1] == Vec3::ONE);
        // Not asked for: nothing.
        let mut none = vec![Vec3::ONE; k1.len()];
        s.scales(&body, None, Parts { hair: true, ..Parts::default() }, &mut none);
        assert!(none.iter().all(|v| *v == Vec3::ONE));
        // Laid onto other axes: a KH1 bone turned a quarter about z takes KH2's x scale on its y.
        let mut k1t = k1.clone();
        k1t[234].rotation = [0.0, 0.0, std::f32::consts::FRAC_PI_2];
        let st = Secondary::new(&k1t, &k2, None);
        let mut out = vec![Vec3::ONE; k1.len()];
        st.scales(&body, None, Parts { sleeves: true, ..Parts::default() }, &mut out);
        assert!((out[234] - Vec3::new(1.0, 0.7, 1.0)).length() < 0.02, "{:?}", out[234]);
    }

    #[test]
    fn scale_multiplies_down_the_bones_and_keeps_places() {
        let (k1, _, _) = rigs();
        let parents: Vec<i16> = k1.iter().map(|b| b.parent).collect();
        let b1 = kh2_anim::Pose::bind(&k1).bones;
        let mut pose = b1.clone();
        let mut local = vec![Vec3::ONE; k1.len()];
        local[283] = Vec3::new(1.0, 2.0, 1.0);
        local[285] = Vec3::new(1.0, 1.0, 0.5);
        apply_scale(&mut pose, &parents, &local);
        for i in 0..pose.len() {
            assert!(at(&pose[i]).distance(at(&b1[i])) < 1e-5, "bone {i} moved");
        }
        let sc = |i: usize| pose[i].to_scale_rotation_translation().0;
        assert!((sc(283) - Vec3::new(1.0, 2.0, 1.0)).length() < 1e-5);
        assert!((sc(284) - Vec3::new(1.0, 2.0, 1.0)).length() < 1e-5);
        assert!((sc(285) - Vec3::new(1.0, 2.0, 0.5)).length() < 1e-5);
        assert!(pose[282] == b1[282] && pose[100] == b1[100]);
        // All at 1: nothing touched.
        let mut p2 = b1.clone();
        apply_scale(&mut p2, &parents, &vec![Vec3::ONE; k1.len()]);
        assert!(p2 == b1);
    }

    #[test]
    fn parts_cover_their_bones() {
        let p = Parts { hair: true, ..Parts::default() };
        assert!(p.covers(153) && !p.covers(80) && !p.covers(285));
        assert!(Parts::ALL.covers(80) && Parts::ALL.covers(292) && !Parts::ALL.covers(151) && !Parts::ALL.covers(282));
        assert!(Parts::ALL.covers(129) && Parts::ALL.covers(247) && !Parts::ALL.covers(93) && !Parts::ALL.covers(198));
        assert!(Parts::ALL.covers(86) && Parts::ALL.covers(88) && !Parts::ALL.covers(85) && !p.covers(87));
    }
}
