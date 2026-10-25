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
pub(crate) enum Pole {
    /// From the first bone's position to the second's.
    Line(usize, usize),
    /// Where the middle joint `b` of `a`-`b`-`c` points, away from the `a`-`c` line (the knee's
    /// front, the elbow's back; negated when `flip`), falling back to the `Line` while the limb
    /// is nearly straight. For a hinge both segments share it, whatever the foot or hand does.
    Joint { a: usize, b: usize, c: usize, fallback: (usize, usize), flip: bool },
    /// A knee: as `Joint`, but nearly straight it falls back to the hips' forward (the line
    /// between the two hip joints `.0` → `.1`, crossed with up). The foot's direction, the
    /// first fallback, turns against the knee whenever a raised thigh points the toes down
    /// (falls, landings, jumps, combos), and the thigh then rolled 180° within a frame as the
    /// leg straightened (the user: a trouser leg snapped while falling).
    Knee { a: usize, b: usize, c: usize, hips: (usize, usize) },
    /// A foot: the ankle's hinge (the shin part's x, so the foot rolls with the shin as a real
    /// ankle does), turning to the model's up while the foot lies flat near the ground (a
    /// planted sole stays flat when KH2's shins lean out: the user's contorted shoes). `sign`
    /// makes the hinge's pole agree with up in the bind (set in `Retarget::new`). The up rule
    /// alone, used everywhere before, turned a gliding foot (pointed back, sole up) over: the
    /// user's shoes spun round in the glide.
    Foot { shin: usize, ankle: usize, sign: f32 },
}

/// The ankle's height (KH2 model units) below which a flat foot counts as on the ground, and
/// above which it doesn't at all.
const FOOT_LOW: f32 = 24.0;
const FOOT_HIGH: f32 = 40.0;

#[derive(Clone, Copy)]
pub(crate) enum Part {
    /// x along `right` (`.1` to `.0`), y along `up` (`.0` to `.1`) made square to x.
    Torso { right: (usize, usize), up: (usize, usize) },
    /// y along `from` → `to`, z along the pole made square to y.
    Aim { from: usize, to: usize, pole: Pole },
}

/// The sine of a joint's bend: below the first the limb counts as straight (fallback pole),
/// above the second as bent.
const BEND_STRAIGHT: f32 = 0.08;
const BEND_BENT: f32 = 0.25;

/// A hinge `a`-`b`-`c` (hip, knee, ankle): how bent it is (0 straight .. 1 bent) and the
/// direction its middle joint points (the knee's front, the elbow's back), square to `y`.
/// Taken from the plane of the two segments (their cross product, the hinge axis): the joint's
/// offset from the end-to-end line, used before, lies along the thigh when the thigh is raised
/// and the shin folded back (a fall's tucked legs), and its square to the thigh then spun
/// from frame to frame (the user: a trouser leg snapped while falling).
fn bend(a: Vec3, b: Vec3, c: Vec3, y: Vec3) -> (f32, Vec3) {
    let (u, v) = ((b - a).normalize_or_zero(), (c - b).normalize_or_zero());
    let axis = u.cross(v);
    let w = ((axis.length() - BEND_STRAIGHT) / (BEND_BENT - BEND_STRAIGHT)).clamp(0.0, 1.0);
    (w, perp(y.cross(axis), y).normalize_or_zero())
}

fn perp(v: Vec3, axis: Vec3) -> Vec3 {
    v - axis * v.dot(axis)
}

impl Part {
    fn frame(&self, p: &[Vec3], done: &[Option<Quat>]) -> Option<Quat> {
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
                        let (w, dir) = bend(at(a)?, at(b)?, at(c)?, y);
                        let dir = dir * if flip { -1.0 } else { 1.0 };
                        (fb * (1.0 - w) + dir * w).normalize_or_zero()
                    }
                    Pole::Knee { a, b, c, hips } => {
                        // The hips' forward, swung along with the segment from hanging straight
                        // down to where it points: square to the segment however far the thigh
                        // is raised (a raised thigh lies along the plain forward, and its roll
                        // then spun from frame to frame).
                        let right = (at(hips.1)? - at(hips.0)?).normalize_or_zero();
                        let up = perp(Vec3::Y, right).normalize_or_zero();
                        let forward = right.cross(up);
                        let fb = perp(Quat::from_rotation_arc(-up, y) * forward, y).normalize_or_zero();
                        let (w, dir) = bend(at(a)?, at(b)?, at(c)?, y);
                        (fb * (1.0 - w) + dir * w).normalize_or_zero()
                    }
                    Pole::Foot { shin, ankle, sign } => {
                        let hinge = (*done.get(shin)?)? * Vec3::X;
                        let z_h = (hinge.cross(y) * sign).normalize_or_zero();
                        let up = perp(Vec3::Y, y).normalize_or_zero();
                        let flat = ((1.0 - y.y.abs()) / 0.5).clamp(0.0, 1.0);
                        let low = ((FOOT_HIGH - at(ankle)?.y) / (FOOT_HIGH - FOOT_LOW)).clamp(0.0, 1.0);
                        let w = flat * low;
                        let z = z_h * (1.0 - w) + up * w;
                        if z.length_squared() < 1e-6 { z_h } else { z.normalize() }
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
fn layout() -> (Vec<Part>, Vec<(usize, usize, usize, f32)>, Vec<(usize, Part)>) {
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
    let mut thigh_plus = 0;
    let mut thigh_minus = 0;
    // Parts whose rest frame is measured on KH1's bind (not KH2's): the KH1 bones then take
    // KH2's direction outright, where the two binds hold that limb differently.
    let mut kh1_rest: Vec<(usize, Part)> = vec![];
    for (s, k) in [(PLUS_X, K1 { hip: 42, clavicle: 194, finger_shift: 0 }), (minus_x(), K1 { hip: 16, clavicle: 89, finger_shift: 104 })] {
        let mut add = |p: Part| {
            parts.push(p);
            parts.len() - 1
        };
        let knee = Pole::Knee { a: s.hip, b: s.knee, c: s.ankle, hips: (minus_x().hip, PLUS_X.hip) };
        let thigh = add(Part::Aim { from: s.hip, to: s.knee, pole: knee });
        let shin = add(Part::Aim { from: s.knee, to: s.ankle, pole: knee });
        let foot = add(Part::Aim { from: s.heel, to: s.toe, pole: Pole::Foot { shin, ankle: s.ankle, sign: 1.0 } });
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
        if k.hip == 42 {
            thigh_plus = thigh;
        } else {
            thigh_minus = thigh;
        }
        // The thumb: KH2's is one segment (base → tip, 89 → 92 / 181 → 184), lying along the
        // fingers in its bind; KH1's (103-107 / 207-211) points straight forward in its bind.
        // Turned by the difference from KH2's bind it kept KH1's forward thumb (the user:
        // unnatural thumbs); aimed from KH1's bind it lies as KH2's thumb lies.
        let (t2, t1) = (89 + (s.index - PLUS_X.index), 207 - k.finger_shift);
        let (index1, little1) = (212 - k.finger_shift, 227 - k.finger_shift);
        let thumb = add(Part::Aim { from: t2, to: t2 + 3, pole: Pole::Line(s.little, s.index) });
        kh1_rest.push((thumb, Part::Aim { from: t1, to: t1 + 4, pole: Pole::Line(little1, index1) }));
        for b in t1..=t1 + 4 {
            one(b, thumb, &mut bones);
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
    // The jacket's hem: nine cloth chains from the spine base (71) down over the shorts: back
    // centre 72, and per side front, side-front, side-back and back (248 / 252 / 256 / 260 on
    // +x, 264 / 268 / 272 / 276 on -x). KH2 parents its own hem to the pelvis (its bones 4-9,
    // 35-38), so ours turns with the hips, not the chest, and a thigh pushes the panels in front
    // of it along as it rises (front most, back least). Before, the hem rode on the chest and
    // stuck out, or cut into the thighs, whenever KH2 leaned or strode (the user: make the
    // jacket mesh with KH2's motions).
    let _ = chest;
    bones.push((72, pelvis, pelvis, 0.0));
    let _ = thigh_plus;
    for (base, thigh) in [(248, thigh_plus), (264, thigh_minus)] {
        for (k, share) in [0.25, 0.15, 0.05, 0.0].into_iter().enumerate() {
            bones.push((base + 4 * k, pelvis, thigh, share));
        }
    }
    (parts, bones, kh1_rest)
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
    /// Bones no part turns: their matrix relative to the parent in a KH1 rest pose
    /// (`set_rest`), else they keep the bind's relation.
    rest_local: Vec<Option<Mat4>>,
    /// The thumbs' relation in KH1's open-handed (field) pose, used while not gripping.
    rest_open: Vec<Option<Mat4>>,
    /// Cloth that hangs (the hood): `(part, share)` turns with the part's yaw but only
    /// `share` of its lean.
    hang: Vec<Option<(usize, f32, f32)>>,
    /// Cloth kept off the thighs (`keep_clear`).
    cloth: Vec<Cloth>,
    /// The bind's matrices (for `attach`).
    bind_m: Vec<Mat4>,
    /// Last turn of each hanging bone and last swing of each cloth chain: they move at most
    /// `CLOTH_STEP` a call, so a flip of the hips' yaw (somersaults) or of which side of a thigh
    /// the cloth is pushed to doesn't snap them round (a little lag, like cloth's own).
    memory: std::sync::Mutex<(Vec<Option<Quat>>, Vec<Quat>)>,
    /// The weapon's grip as KH1 holds it now (`set_grip`), when it is turned well away from the
    /// rest pose's (another weapon held another way: the Dream Sword).
    grip_live: std::sync::Mutex<Option<Mat4>>,
}

/// The weapon's grip bone (the Keyblade's, also every other weapon's).
pub const GRIP_BONE: usize = 280;
/// How far (radians) KH1's live grip must turn from the rest pose's to be used instead.
const GRIP_TURN: f32 = 1.0;

const CLOTH_STEP: f32 = 0.35;

fn limit(prev: Option<Quat>, want: Quat) -> Quat {
    match prev {
        Some(p) => {
            let a = p.angle_between(want);
            if a > CLOTH_STEP { p.slerp(want, CLOTH_STEP / a).normalize() } else { want }
        }
        None => want,
    }
}

/// A cloth chain: its root, every bone it moves, and per joint the thigh segment (hip, knee)
/// and the distance from it the joint keeps (its bind distance, at most `CLEAR_MAX`).
struct Cloth {
    root: usize,
    bones: Vec<usize>,
    nodes: Vec<(usize, (usize, usize), f32)>,
}

/// KH1's thighs (hip joint, knee) and the cloth chains kept off them.
const THIGHS: [(usize, usize); 2] = [(43, 44), (17, 18)];
const CLEAR_MAX: f32 = 14.0;
/// Chain roots and joints: the belt chain (10: 12-15), the hem's back centre (72: 73-75) and
/// its eight panels (root: root + 2, + 3).
/// The belt chain (10-15) runs from the belt down over the +x shorts panel to the back pocket:
/// it is carried by that thigh (43) as in the bind, as the shorts' panels are, so it keeps
/// lying on them however the leg goes (hanging free it fell inside the shorts whenever the
/// battle stance spread the legs: the user's chain clipping through the pants).
const ATTACHED: [(usize, usize); 1] = [(10, 43)];

/// Each: root, joints, what they keep off (bone pairs), the most distance kept, and a margin
/// added to the bind's distance.
fn cloth_chains() -> Vec<(usize, Vec<usize>, Vec<(usize, usize)>, f32, f32)> {
    let thighs = THIGHS.to_vec();
    let mut v = vec![
        // The belt chain, carried by the +x thigh, also stands CHAIN_OUT further off it than in
        // the bind: its back part (skinned with the hips, which the thigh turns against in
        // KH2's stances) lay in the back of the shorts (the user: the back of the chain clips
        // into the shorts, in every stance).
        (10usize, vec![14usize, 15], vec![THIGHS[0]], 22.0, CHAIN_OUT),
        (72, vec![73, 74, 75], thighs.clone(), CLEAR_MAX, 0.0),
    ];
    for root in [248usize, 252, 256, 260, 264, 268, 272, 276] {
        v.push((root, vec![root + 2, root + 3], thighs.clone(), CLEAR_MAX, 0.0));
    }
    v
}

const CHAIN_OUT: f32 = 4.0;

/// Bones that hang as in KH1's rest pose (`set_rest`): the hood (root and cloth joints) and
/// the Keyblade's grip and chain. Everything else keeps the bind's relation (the belt chain
/// hung out in a loop from the battle idle's swing: the user's "floating chain"). The hood
/// lies on the back as in KH1's battle idle, carried by the neck base (78, the chest): a hood
/// hanging by the chest's yaw alone, then pushed off the back, clipped wildly in KH2's
/// hunched battle stance (the user); carried by the chest it can't meet the back.
const REST_BONES: [std::ops::RangeInclusive<usize>; 2] = [79..=84, 280..=292];
/// The hood's root, and how far it is folded down onto the back from KH1's pose: KH1's own
/// pose holds it straight back (the user: at 90° again); folded 0.7 rad its tip lies about
/// 7 below the neck base, its joints 7-13 off the spine (clear of the back).
const HOOD_ROOT: usize = 79;
const HOOD_TIP: usize = 84;
const HOOD_FOLD: f32 = 0.7;
/// The thumbs (103-107 / 207-211). Now aimed along KH2's thumb from KH1's own bind (see
/// `layout`), so this open-hand fallback (`set_rest_open`, `pose_grip`) only applies if that
/// part can't be measured (KH1's own motions leave the thumbs at their bind, which is why KH1's
/// rest poses never changed them: the user's "thumbs have still not changed").
const THUMBS: [std::ops::RangeInclusive<usize>; 2] = [103..=107, 207..=211];

/// A turn's part about the model's up (y) axis (swing-twist).
fn yaw_of(q: Quat) -> Quat {
    let t = Quat::from_xyzw(0.0, q.y, 0.0, q.w);
    if t.length_squared() < 1e-8 { Quat::IDENTITY } else { t.normalize() }
}

/// Every part's frame, in order (a foot reads its shin's).
pub(crate) fn frames(parts: &[Part], p: &[Vec3]) -> Vec<Option<Quat>> {
    let mut done: Vec<Option<Quat>> = Vec::with_capacity(parts.len());
    for part in parts {
        let f = part.frame(p, &done);
        done.push(f);
    }
    done
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
        let (mut parts, bones, kh1_rest) = layout();
        let rest = positions(&b2);
        // Feet: their hinge pole points as up does in the bind.
        let rest_frames = frames(&parts, &rest);
        for p in parts.iter_mut() {
            if let Part::Aim { from, to, pole: Pole::Foot { shin, ankle, sign } } = p {
                let y = (rest[*to] - rest[*from]).normalize_or_zero();
                let hinge = rest_frames[*shin].map_or(Vec3::X, |q| q * Vec3::X);
                if hinge.cross(y).dot(Vec3::Y) < 0.0 {
                    *p = Part::Aim { from: *from, to: *to, pole: Pole::Foot { shin: *shin, ankle: *ankle, sign: -*sign } };
                }
            }
        }
        let mut rest_inv: Vec<Option<Quat>> = frames(&parts, &rest).into_iter().map(|q| q.map(|q| q.inverse())).collect();
        let rest1 = positions(&b1);
        for (i, p) in kh1_rest {
            if let Some(r) = rest_inv.get_mut(i) {
                *r = p.frame(&rest1, &[]).map(|q| q.inverse());
            }
        }
        let mut turns = vec![None; kh1.len()];
        for (b, a, c, t) in bones {
            if b < kh1.len() {
                turns[b] = Some((a, c, t));
            }
        }
        let parent = kh1.iter().enumerate().map(|(i, b)| (b.parent >= 0 && (b.parent as usize) < i).then_some(b.parent as usize)).collect();
        let h2 = b2.first().map_or(1.0, |m| m.w_axis.y);
        let hip_ratio = if h2.abs() > 1.0 { pos.first().map_or(1.0, |p| p.y) / h2 } else { 1.0 };
        let rest_local = vec![None; kh1.len()];
        let hang = vec![None; kh1.len()];
        let rest_open = vec![None; kh1.len()];
        let mut cloth = Vec::new();
        for (root, joints, keep_off, cap, margin) in cloth_chains() {
            if joints.iter().chain([&root]).chain(keep_off.iter().flat_map(|(a, b)| [a, b])).any(|&j| j >= kh1.len()) {
                continue;
            }
            let bones: Vec<usize> = (0..kh1.len()).filter(|&i| {
                let mut p = i;
                for _ in 0..16 {
                    if p == root {
                        return true;
                    }
                    match kh1[p].parent {
                        q if q >= 0 => p = q as usize,
                        _ => return false,
                    }
                }
                false
            }).collect();
            let mut nodes = Vec::new();
            for &j in &joints {
                for &(a, b) in &keep_off {
                    let (pa, pb, n) = (pos[a], pos[b], pos[j]);
                    let ab = pb - pa;
                    let t = ((n - pa).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
                    let d = (n - (pa + ab * t)).length();
                    nodes.push((j, (a, b), (d + margin).min(cap)));
                }
            }
            cloth.push(Cloth { root, bones, nodes });
        }
        let memory = std::sync::Mutex::new((vec![None; kh1.len()], vec![Quat::IDENTITY; cloth.len()]));
        let bind_m = b1.clone();
        Retarget { parent, pos, rot, scale, parts, rest_inv, turns, hip_ratio, rest_local, rest_open, hang, cloth, memory, bind_m, grip_live: std::sync::Mutex::new(None) }
    }

    /// KH1's hip height over KH2's (KH2 distances to KH1 ones).
    pub fn hip_ratio(&self) -> f32 {
        self.hip_ratio
    }

    /// Bones no part turns (hood, belt chain, the shorts' cloth joints, face, hair, the weapon
    /// chain) hang as in this KH1 pose (local bones, e.g. a KH1 motion's frame applied to the
    /// skeleton) instead of the bind's T-pose (the user: the hood stood out at 90°).
    pub fn set_rest(&mut self, rest: &[Bone]) {
        let m = kh2_anim::Pose::bind(rest).bones;
        for i in 0..self.rest_local.len().min(m.len()) {
            if self.turns[i].is_some() || self.hang[i].is_some() || !REST_BONES.iter().any(|r| r.contains(&i)) {
                continue;
            }
            if let Some(p) = self.parent[i] {
                let mut mi = m[i];
                if i == HOOD_ROOT && HOOD_TIP < m.len() {
                    // Folded down onto the back: turned about the level axis square to the hood.
                    let root = mi.w_axis.truncate();
                    let dir = (m[HOOD_TIP].w_axis.truncate() - root).normalize_or_zero();
                    let axis = Vec3::Y.cross(dir).normalize_or_zero();
                    if axis != Vec3::ZERO {
                        mi = Mat4::from_translation(root) * Mat4::from_axis_angle(axis, HOOD_FOLD) * Mat4::from_translation(-root) * mi;
                    }
                }
                let l = m[p].inverse() * mi;
                self.rest_local[i] = l.is_finite().then_some(l);
            }
        }
    }

    /// KH1's own grip this frame (`GRIP_BONE` relative to its parent, from KH1's pose), or
    /// `None` when KH1 didn't pose him. A grip turned more than `GRIP_TURN` from the rest pose's
    /// (KH1's battle idle with the Keyblade) is used instead of it: another weapon is held
    /// another way (the user: the Dream Sword was drawn backwards). Returns whether it is used.
    /// The angle (radians) between a grip and the rest pose's (diagnostics).
    pub fn grip_turn(&self, live: &Mat4) -> Option<f32> {
        let r = self.rest_local.get(GRIP_BONE).copied().flatten()?;
        let (_, lq, _) = live.to_scale_rotation_translation();
        let (_, rq, _) = r.to_scale_rotation_translation();
        Some(lq.angle_between(rq))
    }

    pub fn set_grip(&self, live: Option<Mat4>) -> bool {
        let use_live = match (live, self.rest_local.get(GRIP_BONE).copied().flatten()) {
            (Some(l), Some(r)) if l.is_finite() => {
                let (_, lq, _) = l.to_scale_rotation_translation();
                let (_, rq, _) = r.to_scale_rotation_translation();
                (lq.angle_between(rq) > GRIP_TURN).then_some(l)
            }
            _ => None,
        };
        if live.is_some() {
            if let Ok(mut g) = self.grip_live.lock() {
                *g = use_live;
            }
        }
        use_live.is_some()
    }

    /// The thumbs' open-handed pose (KH1's field idle), for `pose_grip(.., false)`.
    pub fn set_rest_open(&mut self, rest: &[Bone]) {
        let m = kh2_anim::Pose::bind(rest).bones;
        for i in 0..self.rest_open.len().min(m.len()) {
            if self.turns[i].is_some() || !THUMBS.iter().any(|r| r.contains(&i)) {
                continue;
            }
            if let Some(p) = self.parent[i] {
                let l = m[p].inverse() * m[i];
                self.rest_open[i] = l.is_finite().then_some(l);
            }
        }
    }

    /// Whether a measured part turns this KH1 bone.
    pub fn is_mapped(&self, bone: usize) -> bool {
        self.turns.get(bone).is_some_and(|t| t.is_some())
    }

    /// The measured parts (KH2 bone indices), as `pose` measures them.
    pub(crate) fn parts(&self) -> &[Part] {
        &self.parts
    }

    /// The measured part that alone turns this KH1 bone, if one does.
    pub(crate) fn single_part(&self, bone: usize) -> Option<usize> {
        self.turns.get(bone).copied().flatten().and_then(|(a, b, t)| (a == b || t == 0.0).then_some(a))
    }

    /// Whether one measured part alone turns this KH1 bone (not a blend of two: the spine's
    /// middle, the neck), so its turn is that part's (`inverse` pairs KH2 bones with these).
    pub fn follows_one_part(&self, bone: usize) -> bool {
        self.turns.get(bone).copied().flatten().is_some_and(|(a, b, t)| a == b || t == 0.0)
    }

    /// How many KH1 bones a measured part turns.
    pub fn mapped(&self) -> usize {
        self.turns.iter().filter(|t| t.is_some()).count()
    }

    /// Each part's turn for a KH2 pose (debugging).
    pub fn part_turns(&self, kh2: &[Mat4]) -> Vec<Option<Quat>> {
        let now = positions(kh2);
        frames(&self.parts, &now).into_iter().zip(&self.rest_inv).map(|(f, r)| Some(f? * (*r)?)).collect()
    }

    /// KH1 model-space bone matrices for a KH2 model-space pose (hands gripping).
    pub fn pose(&self, kh2: &[Mat4]) -> Vec<Mat4> {
        self.pose_grip(kh2, true)
    }

    /// As `pose`; `grip` false gives the thumbs KH1's open-handed pose.
    pub fn pose_grip(&self, kh2: &[Mat4], grip: bool) -> Vec<Mat4> {
        let now = positions(kh2);
        let part: Vec<Option<Quat>> = frames(&self.parts, &now).into_iter().zip(&self.rest_inv).map(|(f, r)| Some(f? * (*r)?)).collect();
        let n = self.pos.len();
        let mut turn = vec![Quat::IDENTITY; n];
        let mut at = vec![Vec3::ZERO; n];
        let mut out = vec![Mat4::IDENTITY; n];
        for i in 0..n {
            let own = self.turns[i].and_then(|(a, b, t)| Some(part[a]?.slerp(part[b]?, t))).or_else(|| {
                let (a, share, drop) = self.hang[i]?;
                let want = yaw_of(part[a]?).slerp(part[a]?, share) * Quat::from_rotation_x(-drop);
                let mut mem = self.memory.lock().ok()?;
                let q = limit(mem.0[i], want);
                mem.0[i] = Some(q);
                Some(q)
            });
            let rest = if grip { self.rest_local[i] } else { self.rest_open[i].or(self.rest_local[i]) };
            let rest = if i == GRIP_BONE { self.grip_live.lock().ok().and_then(|g| *g).or(rest) } else { rest };
            if let (None, Some(p), Some(l)) = (own, self.parent[i], rest) {
                out[i] = out[p] * l;
                let (_, r, t) = out[i].to_scale_rotation_translation();
                turn[i] = (r * self.rot[i].inverse()).normalize();
                at[i] = t;
                continue;
            }
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
        self.attach(&mut out);
        self.keep_clear(&mut out);
        out
    }

    /// Bones carried rigidly by another as in the bind (`ATTACHED`).
    fn attach(&self, out: &mut [Mat4]) {
        for &(root, to) in ATTACHED.iter() {
            if root >= out.len() || to >= out.len() {
                continue;
            }
            let carry = out[to] * self.bind_m[to].inverse();
            for j in 0..out.len() {
                let mut p = Some(j);
                for _ in 0..16 {
                    match p {
                        Some(q) if q == root => {
                            out[j] = carry * self.bind_m[j];
                            break;
                        }
                        Some(q) => p = self.parent[q],
                        None => break,
                    }
                }
            }
        }
    }

    /// Cloth (the belt chain, the jacket's hem panels) never comes nearer a thigh than it was in
    /// the bind: where a joint would, its chain swings about its root just far enough out
    /// (KH1 animates this cloth by hand in each of its own motions, so nothing of KH1's fits
    /// KH2's strides; the user: the chain went through the shorts, the jacket should mesh).
    fn keep_clear(&self, out: &mut [Mat4]) {
        let at = |v: &[Mat4], i: usize| v[i].w_axis.truncate();
        let Ok(mut mem) = self.memory.lock() else { return };
        for (ci, c) in self.cloth.iter().enumerate() {
            let mut swing = Quat::IDENTITY;
            for _ in 0..2 {
                let root = at(out, c.root);
                let mut best: Option<(f32, Quat)> = None;
                for &(node, (a, b), clear) in &c.nodes {
                    let n = at(out, node);
                    let (pa, pb) = (at(out, a), at(out, b));
                    let ab = pb - pa;
                    let t = ((n - pa).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
                    let near = pa + ab * t;
                    let d = (n - near).length();
                    if d >= clear {
                        continue;
                    }
                    let away = (n - near).normalize_or_zero();
                    let away = if away == Vec3::ZERO { (n - root).cross(ab).normalize_or_zero() } else { away };
                    let target = n + away * (clear - d);
                    let (from, to) = ((n - root).normalize_or_zero(), (target - root).normalize_or_zero());
                    if from == Vec3::ZERO || to == Vec3::ZERO {
                        continue;
                    }
                    let q = Quat::from_rotation_arc(from, to);
                    let angle = q.angle_between(Quat::IDENTITY);
                    if best.map_or(true, |(a, _)| angle > a) {
                        best = Some((angle, q));
                    }
                }
                let Some((_, q)) = best else { break };
                swing = q * swing;
                let m = Mat4::from_translation(root) * Mat4::from_quat(q) * Mat4::from_translation(-root);
                for &j in &c.bones {
                    out[j] = m * out[j];
                }
            }
            // Eased: undo the swing just made and make the limited one instead.
            let eased = limit(Some(mem.1[ci]), swing);
            mem.1[ci] = eased;
            let root = at(out, c.root);
            let m = Mat4::from_translation(root) * Mat4::from_quat(eased * swing.inverse()) * Mat4::from_translation(-root);
            for &j in &c.bones {
                out[j] = m * out[j];
            }
        }
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
