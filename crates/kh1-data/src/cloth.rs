//! KH1 Sora's hood, coat and shorts moved by KH2's hidden cloth keys, smoothed and kept clear of
//! his body, worked out over a whole baked motion (Mr Bugs, "Missing sleeves animation": "do (c)",
//! "it should be like kh2" (no lag, no settling), "just do a test bake into soras running and
//! walking animations for now"). `docs/kh1-secondary-motion.md`, "Hood, coat and shorts from KH2's
//! hidden keys (test bake)".
//!
//! KH2's motions key its hood (72 / 73), coat ring (70, 129-135, 221-227) and trouser bones (27,
//! 29, 34 / 56, 58, 63) in every move, but its model draws no mesh on them (Checked (files)), so
//! 1:1 with what KH2 shows left KH1's hood and shorts rigid. Here, for the bakes that ask for it:
//!
//! - **Hood**: each of KH1's hood joints (80, 81 / 82, 83) takes KH2's hood bone's turn (72 / 73)
//!   against KH2's chest, on KH1's chest (as `Secondary`'s hood before `KH2_SHOWN_ON`, t375).
//!   Checked (files) for the keys; using them on a hood KH2 doesn't show is ours.
//! - **Coat**: the turns `Retarget::coat_kh2` already gives it from KH2's coat ring.
//! - **Shorts**: each of KH1's eight panels (joint `r + 1` of 26, 30, 34, 38 / 52, 56, 60, 64)
//!   sways as KH2's trouser bones on that thigh sway about their mean over the motion (their keys
//!   sit 10-60 degrees off their bind, Checked (files), so the sway is measured from the mean,
//!   not the bind: ours), blended by where each sits round the thigh in the binds (as the sleeve
//!   flaps).
//!
//! Then, ours and not KH2's (KH2 keeps nothing off anything):
//! - **Smoothing**: every part's turn against its carrier is averaged over the frames around it
//!   (Gaussian, `SMOOTH_SIGMA` frames, both ways in time, round the loop for a cycle). Centred,
//!   so nothing lags behind KH2's keys; it takes out KH2's one-frame jumps on the hidden bones
//!   (the trouser bone 56 turns up to 33 degrees between two frames of KH2's battle run).
//! - **Clearance**: what each part is kept off: the hood off the spine (7) and the head's centre
//!   (10), as `Secondary`'s hood guards; the coat outside the shorts round each thigh (6.4 at the
//!   hip to 11.2 from mid-thigh, plus 1, never more than its bind distance) and no more than 5
//!   nearer the hips' up axis than in the bind (KH1's own run takes it 5.7 nearer); each shorts
//!   panel no nearer its thigh's axis than 0.95 of its bind distance. Where a joint would come
//!   nearer, its chain is swung about a joint, round one axis fixed for the whole motion (in the
//!   carrier's frame: of eight round the chain, the one that needs the least swing over the
//!   motion; the side memory, so it never flips over to the other side of a thigh within a
//!   frame), just far enough. A panel (coat or shorts) takes half the swing it needs at its top
//!   and bends at its middle for the rest, so it drapes round a knee instead of standing out
//!   straight. Each swing is spread over the frames around it (the largest within `DILATE`
//!   frames, then smoothed), so it starts before the contact and lets go after, never less than
//!   needed.

use glam::{Mat4, Quat, Vec3, Vec4};
use kh2_formats::anim::Bone;

/// KH1's chest (the neck base, which carries the hood and the coat) and spine base.
const KH1_CHEST: usize = 78;
const KH1_SPINE: usize = 71;
const KH1_HEAD: usize = 151;
/// KH1's hips (the pelvis joint under the root).
const KH1_HIPS: usize = 3;
/// How much nearer the hips' axis than in the bind a coat joint may come (KH1's own coat goes 2
/// nearer in 12.7% of its own motions' frames: `coat_check`).
const HIPS_IN: f32 = 5.0;
/// KH1's thighs (hip joint, knee): -x, +x.
const KH1_THIGHS: [(usize, usize); 2] = [(17, 18), (43, 44)];
/// KH2's chest and thighs (-x 40, +x 11: KH2's -x leg is 39-63, +x 10-34).
const KH2_CHEST: usize = 68;
const KH2_THIGHS: [usize; 2] = [40, 11];
/// KH1's hood joints and the KH2 hood bone each follows (`secondary::HOOD`).
const HOOD: [(usize, usize); 4] = [(80, 72), (81, 72), (82, 73), (83, 73)];
/// The hood's clearance from the spine (71 → 78) and the head's centre (`secondary`'s guards).
const HOOD_CLEAR: f32 = 7.0;
const HOOD_HEAD: f32 = 10.0;
/// KH2's coat ring leaves each coat chain sways as (`retarget::COAT_KH2`).
const COAT_KH2: [usize; 9] = [70, 129, 131, 133, 135, 221, 223, 225, 227];

/// What the coat's chains hang from under KH2's coat ring sway.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoatRest {
    /// KH2's coat ring's turns as they are (`Retarget::coat_kh2`, t552).
    Ring,
    /// Their bind relation to KH1's chest, with the ring's sway about its mean.
    Chest,
    /// Their bind relation to KH1's hips, with the ring's sway about its mean.
    Hips,
}
const COAT_REST: CoatRest = CoatRest::Ring;

/// The coat's nine chains (root, joint at the same place, two joints below): their roots.
const COAT: [usize; 9] = [72, 248, 252, 256, 260, 264, 268, 272, 276];
/// KH1's shorts: four panels a thigh (root, joint at the same place, joint, tip): -x on 17, +x
/// on 43.
const SHORTS: [[usize; 4]; 2] = [[26, 30, 34, 38], [52, 56, 60, 64]];
/// KH2's trouser bones on each thigh: (pivot, the leaf its motions turn): front-out 26 / 27,
/// back-out 28 / 29, front 33 / 34 (+x); the -x leg's are + 29.
const KH2_TROUSERS: [[(usize, usize); 3]; 2] = [[(55, 56), (57, 58), (62, 63)], [(26, 27), (28, 29), (33, 34)]];
/// The shorts' surface round a thigh for the coat: 6.4 at the hip to 11.2 from mid-thigh (KH1's
/// panel bones in the bind; Assumption, the mesh isn't read), plus this margin.
const SHORTS_MARGIN: f32 = 1.0;
/// A shorts panel keeps this share of its bind distance from its thigh's axis.
const PANEL_CLEAR: f32 = 0.95;

/// Gaussian smoothing of the turns (frames), its reach, the clearance's spread before smoothing.
const SMOOTH_SIGMA: f32 = 1.2;
const SMOOTH_REACH: usize = 3;
const DILATE: usize = 3;
/// The most a chain is swung for clearance, and the search step.
const PUSH_MAX: f32 = 100.0 * std::f32::consts::PI / 180.0;
const PUSH_STEP: f32 = 2.0 * std::f32::consts::PI / 180.0;

/// A two-joint panel's clearance stages (`Chain::stages`): root `r`, joints `r + 1` (at its
/// place), `r + 2`, tip `r + 3`; nodes 0 (`r + 2`) and 1 (`r + 3`).
fn panel_stages(r: usize) -> Vec<(usize, Vec<usize>, f32)> {
    vec![(r + 1, vec![0, 1], PANEL_TOP_SHARE), (r + 2, vec![1], 1.0), (r + 1, vec![0], 1.0)]
}
/// The share of a panel's clearance swing taken at its top (the rest bends it at its middle).
const PANEL_TOP_SHARE: f32 = 0.5;

/// What drives a joint of a chain.
#[derive(Clone, Debug)]
enum Drive {
    /// KH2 bones' turn from their bind against KH2's `carrier`, blended (bone, weight).
    Keyed { carrier: usize, by: Vec<(usize, f32)> },
    /// The same, as a sway about its mean over the motion, on the joint's own relation to its
    /// carrier as the input pose has it.
    /// `rest`: the KH1 bone the joint keeps its bind relation to under the sway (else the
    /// input pose's relation).
    Sway { carrier: usize, by: Vec<(usize, f32)>, rest: Option<usize> },
    /// The input pose's turn (already KH2's).
    Kept,
}

#[derive(Clone, Copy, Debug)]
enum Obstacle {
    /// A bone-to-bone segment.
    Segment(usize, usize),
    /// A point carried by a bone (its bind-pose model-space place).
    Point(usize, Vec3),
}

#[derive(Clone, Copy, Debug)]
enum Clear {
    Fixed(f32),
    /// Outside the shorts round this thigh segment (`SHORTS_MARGIN`), never more than this.
    Shorts(f32),
}

#[derive(Clone, Debug)]
struct Chain {
    part: Part,
    carrier: usize,
    /// The joints turned, in order, and what drives each.
    links: Vec<(usize, Drive)>,
    /// The joints checked, and against what.
    nodes: Vec<usize>,
    obstacles: Vec<(Obstacle, Vec<Clear>)>,
    /// The clearance swings, in order: each about a joint, for some of the nodes (indices into
    /// `nodes`) below it. A two-joint panel is swung at its top for its middle, then bent at its
    /// middle for its tip, so it drapes round what it meets: half the swing the whole panel would
    /// need at its top, the tip's rest as a bend at its middle, then the middle's own if left.
    /// Each with the share of the swing it needs that it takes (the rest left to the next).
    stages: Vec<(usize, Vec<usize>, f32)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Hood,
    Coat,
    Shorts,
}

/// Which parts a bake moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Parts {
    pub hood: bool,
    pub coat: bool,
    pub shorts: bool,
}

impl Parts {
    pub const ALL: Parts = Parts { hood: true, coat: true, shorts: true };
    fn has(&self, p: Part) -> bool {
        match p {
            Part::Hood => self.hood,
            Part::Coat => self.coat,
            Part::Shorts => self.shorts,
        }
    }
}

/// What one `bake` did (for the log and the checks).
#[derive(Clone, Copy, Debug, Default)]
pub struct Report {
    pub chains: usize,
    /// Chain-frames that needed a push, and the largest push (degrees).
    pub pushed: usize,
    pub max_push: f32,
    /// The deepest any checked joint stays inside its clearance after the bake (0 when clear).
    pub worst_inside: f32,
}

pub struct ClothBake {
    parent: Vec<Option<usize>>,
    below: Vec<Vec<usize>>,
    kh1_bind: Vec<Mat4>,
    kh2_bind: Vec<Mat4>,
    chains: Vec<Chain>,
}

fn rot(m: &Mat4) -> Quat {
    m.to_scale_rotation_translation().1.normalize()
}

fn at(m: &Mat4) -> Vec3 {
    m.w_axis.truncate()
}

fn nearest_on_segment(p: Vec3, a: Vec3, b: Vec3) -> (Vec3, f32) {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    (a + ab * t, t)
}

fn shorts_r(t: f32) -> f32 {
    if t < 0.5 { 6.4 + (11.2 - 6.4) * t / 0.5 } else { 11.2 }
}

/// A weighted average of turns (sign-aligned to the first).
fn average(qs: impl Iterator<Item = (Quat, f32)>) -> Option<Quat> {
    let mut first: Option<Vec4> = None;
    let mut s = Vec4::ZERO;
    for (q, w) in qs {
        let v = Vec4::from(q);
        let r = *first.get_or_insert(v);
        s += if v.dot(r) < 0.0 { -v } else { v } * w;
    }
    let q = Quat::from_vec4(s);
    (q.length_squared() > 1e-12).then(|| q.normalize())
}

/// Index `i + k` of a sequence of `n`, round the loop (`cycle`: frames `0..cycle` repeat, a last
/// frame `cycle` being frame 0 again) or clamped to its ends.
fn index(i: usize, k: isize, n: usize, cycle: Option<usize>) -> usize {
    match cycle {
        Some(c) if c > 0 => ((i % c) as isize + k).rem_euclid(c as isize) as usize,
        _ => (i as isize + k).clamp(0, n as isize - 1) as usize,
    }
}

fn gauss(k: isize) -> f32 {
    (-(k * k) as f32 / (2.0 * SMOOTH_SIGMA * SMOOTH_SIGMA)).exp()
}

fn smooth_turns(v: &[Quat], cycle: Option<usize>) -> Vec<Quat> {
    let r = SMOOTH_REACH as isize;
    (0..v.len()).map(|i| average((-r..=r).map(|k| (v[index(i, k, v.len(), cycle)], gauss(k)))).unwrap_or(v[i])).collect()
}

/// The largest within `DILATE` frames, then Gaussian-smoothed (never below the input: every
/// frame the smoothing reaches holds at least this frame's value after the spread).
fn spread(v: &[f32], cycle: Option<usize>) -> Vec<f32> {
    let n = v.len();
    let d = DILATE as isize;
    let dil: Vec<f32> = (0..n).map(|i| (-d..=d).map(|k| v[index(i, k, n, cycle)]).fold(0.0, f32::max)).collect();
    let r = SMOOTH_REACH.min(DILATE) as isize;
    (0..n)
        .map(|i| {
            let (mut s, mut w) = (0.0, 0.0);
            for k in -r..=r {
                s += dil[index(i, k, n, cycle)] * gauss(k);
                w += gauss(k);
            }
            (s / w).max(v[i])
        })
        .collect()
}

impl ClothBake {
    /// `kh1`: KH1 Sora's skeleton; `kh2`: KH2 Sora's. A part whose bones either lacks is left out.
    pub fn new(kh1: &[Bone], kh2: &[Bone]) -> ClothBake {
        Self::with_coat(kh1, kh2, COAT_REST)
    }

    /// As `new`, the coat's rest chosen (for the checks).
    pub fn with_coat(kh1: &[Bone], kh2: &[Bone], coat: CoatRest) -> ClothBake {
        let kh1_bind = kh2_anim::Pose::bind(kh1).bones;
        let kh2_bind = kh2_anim::Pose::bind(kh2).bones;
        let parents = |sk: &[Bone]| -> Vec<Option<usize>> { sk.iter().enumerate().map(|(i, b)| (b.parent >= 0 && (b.parent as usize) < i).then_some(b.parent as usize)).collect() };
        let parent = parents(kh1);
        let p2 = parents(kh2);
        let mut kids = vec![Vec::new(); kh1.len()];
        for (i, p) in parent.iter().enumerate() {
            if let Some(p) = p {
                kids[*p].push(i);
            }
        }
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
        let ok1 = |b: usize| b < kh1.len();
        let ok2 = |b: usize| b < kh2.len();
        let bind_d = |j: usize, (a, b): (usize, usize)| nearest_on_segment(at(&kh1_bind[j]), at(&kh1_bind[a]), at(&kh1_bind[b])).0.distance(at(&kh1_bind[j]));
        let mut chains = Vec::new();
        // The hood: one chain, swung about its root; kept off the spine and the head.
        if HOOD.iter().all(|&(a, b)| ok1(a + 1) && ok2(b)) && ok1(KH1_CHEST) && ok1(KH1_HEAD) && ok2(KH2_CHEST) {
            let roots: Vec<usize> = (152..=191).step_by(3).filter(|&r| ok1(r)).collect();
            let centre = roots.iter().map(|&r| at(&kh1_bind[r])).sum::<Vec3>() / roots.len().max(1) as f32;
            let nodes: Vec<usize> = HOOD.iter().map(|&(a, _)| a + 1).collect();
            chains.push(Chain {
                part: Part::Hood,
                carrier: KH1_CHEST,
                links: HOOD.iter().map(|&(a, b)| (a, Drive::Keyed { carrier: KH2_CHEST, by: vec![(b, 1.0)] })).collect(),
                obstacles: vec![
                    (Obstacle::Segment(KH1_SPINE, KH1_CHEST), vec![Clear::Fixed(HOOD_CLEAR); nodes.len()]),
                    (Obstacle::Point(KH1_HEAD, centre), vec![Clear::Fixed(HOOD_HEAD); nodes.len()]),
                ],
                stages: vec![(HOOD[0].0, (0..nodes.len()).collect(), 1.0)],
                nodes,
            });
        }
        // The coat: nine chains, as the retarget turned them, kept outside the shorts.
        if COAT.iter().all(|&r| ok1(r + 3)) && ok1(KH1_SPINE) && KH1_THIGHS.iter().all(|&(a, b)| ok1(a) && ok1(b)) {
            for &r in &COAT {
                let nodes = vec![r + 2, r + 3];
                let mut obstacles: Vec<(Obstacle, Vec<Clear>)> = KH1_THIGHS.iter().map(|&t| (Obstacle::Segment(t.0, t.1), nodes.iter().map(|&j| Clear::Shorts(bind_d(j, t))).collect())).collect();
                // ... and off the hips' up axis (hips 3 → spine base 71) no nearer than
                // `HIPS_IN` inside its bind radius (the belt and the shorts' top).
                obstacles.push((Obstacle::Segment(KH1_HIPS, KH1_SPINE), nodes.iter().map(|&j| Clear::Fixed(bind_d(j, (KH1_HIPS, KH1_SPINE)) - HIPS_IN)).collect()));
                let k = COAT.iter().position(|&x| x == r).unwrap_or(0);
                let drive = match coat {
                    CoatRest::Ring => Drive::Kept,
                    CoatRest::Chest => Drive::Sway { carrier: KH2_CHEST, by: vec![(COAT_KH2[k], 1.0)], rest: Some(KH1_CHEST) },
                    CoatRest::Hips => Drive::Sway { carrier: KH2_CHEST, by: vec![(COAT_KH2[k], 1.0)], rest: Some(KH1_HIPS) },
                };
                chains.push(Chain { part: Part::Coat, carrier: KH1_CHEST, links: vec![(r + 1, drive)], nodes, obstacles, stages: panel_stages(r) });
            }
        }
        // The shorts: each panel sways as KH2's trouser bones round that thigh, blended by where
        // they sit round it (as the sleeve flaps), kept off its own thigh.
        for (side, panels) in SHORTS.iter().enumerate() {
            let (t1, k1) = KH1_THIGHS[side];
            let t2 = KH2_THIGHS[side];
            let tr = &KH2_TROUSERS[side];
            let kh2_ok = tr.iter().all(|&(pv, leaf)| ok2(leaf) && p2[leaf] == Some(pv)) && ok2(t2 + 1);
            if !kh2_ok || !panels.iter().all(|&r| ok1(r + 3) && parent[r] == Some(t1)) || !ok1(k1) {
                continue;
            }
            let round = |b: &[Mat4], (a, e): (usize, usize), i: usize| {
                let axis = (at(&b[e]) - at(&b[a])).normalize_or_zero();
                let o = at(&b[i]) - at(&b[a]);
                (o - axis * o.dot(axis)).normalize_or_zero()
            };
            let dirs: Vec<Vec3> = tr.iter().map(|&(pv, _)| round(&kh2_bind, (t2, t2 + 1), pv)).collect();
            for &r in panels {
                let o = round(&kh1_bind, (t1, k1), r + 2);
                let w: Vec<f32> = dirs.iter().map(|d| 1.0 / (o.distance(*d).powi(2) + 1e-3)).collect();
                let sum: f32 = w.iter().sum();
                let by = tr.iter().zip(&w).map(|(&(_, leaf), w)| (leaf, w / sum)).collect();
                let nodes = vec![r + 2, r + 3];
                let clear = nodes.iter().map(|&j| Clear::Fixed(bind_d(j, (t1, k1)) * PANEL_CLEAR)).collect();
                chains.push(Chain { part: Part::Shorts, carrier: t1, links: vec![(r + 1, Drive::Sway { carrier: t2, by, rest: None })], nodes, obstacles: vec![(Obstacle::Segment(t1, k1), clear)], stages: panel_stages(r) });
            }
        }
        ClothBake { parent, below, kh1_bind, kh2_bind, chains }
    }

    /// The KH2 bones driving each shorts panel and their shares (for the docs and checks):
    /// (KH1 joint, [(KH2 leaf, share)]).
    pub fn shorts_drivers(&self) -> Vec<(usize, Vec<(usize, f32)>)> {
        self.chains
            .iter()
            .filter(|c| c.part == Part::Shorts)
            .filter_map(|c| match &c.links[0] {
                (j, Drive::Sway { by, .. }) => Some((*j, by.clone())),
                _ => None,
            })
            .collect()
    }

    fn turn(&self, pose: &mut [Mat4], b: usize, q: Quat, about: Vec3) {
        if !q.is_finite() || b >= pose.len() {
            return;
        }
        let m = Mat4::from_translation(about) * Mat4::from_quat(q) * Mat4::from_translation(-about);
        pose[b] = m * pose[b];
        for &j in &self.below[b] {
            if j < pose.len() {
                pose[j] = m * pose[j];
            }
        }
    }

    /// KH1 bone `b`'s turn from its bind in `pose`.
    fn turn1(&self, pose: &[Mat4], b: usize) -> Quat {
        (rot(&pose[b]) * rot(&self.kh1_bind[b]).inverse()).normalize()
    }

    /// KH2 bone `k`'s turn from its bind against `carrier`, in the bind's axes, blended.
    fn keyed(&self, body: &[Mat4], carrier: usize, by: &[(usize, f32)]) -> Option<Quat> {
        let (rn, rb) = (rot(body.get(carrier)?), rot(self.kh2_bind.get(carrier)?));
        average(by.iter().filter_map(|&(k, w)| Some(((rb * rn.inverse() * rot(body.get(k)?) * rot(self.kh2_bind.get(k)?).inverse()).normalize(), w))))
    }

    /// How far inside its clearance the chain's worst joint is (negative: clear by that much).
    fn inside(&self, pose: &[Mat4], c: &Chain, only: Option<&[usize]>) -> f32 {
        let mut worst = f32::MIN;
        for (o, clears) in &c.obstacles {
            for (k, (&j, cl)) in c.nodes.iter().zip(clears).enumerate() {
                if only.is_some_and(|o| !o.contains(&k)) {
                    continue;
                }
                let p = at(&pose[j]);
                let (d, t) = match *o {
                    Obstacle::Segment(a, b) => {
                        let (n, t) = nearest_on_segment(p, at(&pose[a]), at(&pose[b]));
                        (p.distance(n), t)
                    }
                    Obstacle::Point(b, q) => (p.distance((pose[b] * self.kh1_bind[b].inverse()).transform_point3(q)), 0.0),
                };
                let need = match *cl {
                    Clear::Fixed(r) => r,
                    Clear::Shorts(cap) => (shorts_r(t) + SHORTS_MARGIN).min(cap),
                };
                worst = worst.max(need - d);
            }
        }
        worst
    }

    /// The swing about `axis` (the carrier's bind frame) each frame needs to clear the chain: the
    /// smallest angle up to `PUSH_MAX` that clears it, or the one that comes nearest; and how
    /// many frames none clears.
    fn needs(&self, poses: &[Vec<Mat4>], c: &Chain, (pivot, only): (usize, &[usize]), axis: Vec3) -> (Vec<f32>, usize) {
        let mut fails = 0;
        let mut q: Vec<Mat4> = Vec::new();
        let need = poses
            .iter()
            .map(|p| {
                if self.inside(p, c, Some(only)) <= 0.0 {
                    return 0.0;
                }
                let a = self.turn1(p, c.carrier) * axis;
                let piv = at(&p[pivot]);
                let mut try_at = |th: f32| {
                    q.clone_from(p);
                    self.turn(&mut q, pivot, Quat::from_axis_angle(a, th), piv);
                    self.inside(&q, c, Some(only))
                };
                let (mut best, mut best_in) = (0.0, f32::MAX);
                let mut th = PUSH_STEP;
                while th <= PUSH_MAX + 1e-6 {
                    let i = try_at(th);
                    if i <= 0.0 {
                        // Refined between the last step and this one.
                        let (mut lo, mut hi) = (th - PUSH_STEP, th);
                        for _ in 0..8 {
                            let m = 0.5 * (lo + hi);
                            if try_at(m) <= 0.0 { hi = m } else { lo = m }
                        }
                        return hi;
                    }
                    if i < best_in {
                        (best, best_in) = (th, i);
                    }
                    th += PUSH_STEP;
                }
                fails += 1;
                best
            })
            .collect();
        (need, fails)
    }

    /// Moves the parts `want` asks for on a whole motion's KH1 poses (model space, as `Retarget`
    /// and `Secondary` left them; unscaled), `body` KH2's pose of each frame. `cycle`: the motion
    /// loops every that many frames (frame `cycle`, if given, is frame 0 again).
    pub fn bake(&self, poses: &mut [Vec<Mat4>], body: &[Vec<Mat4>], cycle: Option<usize>, want: Parts) -> Report {
        let mut rep = Report::default();
        let n = poses.len();
        if n == 0 || body.len() != n || poses.iter().any(|p| p.len() < self.kh1_bind.len() || !p.iter().all(|m| m.is_finite())) {
            return rep;
        }
        for c in self.chains.iter().filter(|c| want.has(c.part)) {
            if c.links.iter().any(|(_, d)| matches!(d, Drive::Keyed { carrier, .. } | Drive::Sway { carrier, .. } if body.iter().any(|b| b.len() <= *carrier))) {
                continue;
            }
            // 1. Each joint's turn against the carrier (bind axes), per frame.
            let mut rel: Vec<Vec<Quat>> = Vec::new();
            let mut ok = true;
            for (j, d) in &c.links {
                let seq: Option<Vec<Quat>> = match d {
                    Drive::Keyed { carrier, by } => (0..n).map(|f| self.keyed(&body[f], *carrier, by)).collect(),
                    Drive::Sway { carrier, by, rest } => {
                        // The sway: each driver's turn over its mean, on the input's relation.
                        let per: Option<Vec<Vec<Quat>>> = by.iter().map(|&(k, _)| (0..n).map(|f| self.keyed(&body[f], *carrier, &[(k, 1.0)])).collect()).collect();
                        per.and_then(|per| {
                            let span = cycle.unwrap_or(n).min(n);
                            let means: Option<Vec<Quat>> = per.iter().map(|s| average(s[..span].iter().map(|q| (*q, 1.0)))).collect();
                            let means = means?;
                            (0..n)
                                .map(|f| {
                                    let sway = average(per.iter().zip(&means).zip(by).map(|((s, m), &(_, w))| ((s[f] * m.inverse()).normalize(), w)))?;
                                    let base = (self.turn1(&poses[f], c.carrier).inverse() * self.turn1(&poses[f], rest.unwrap_or(*j))).normalize();
                                    Some((sway * base).normalize())
                                })
                                .collect()
                        })
                    }
                    Drive::Kept => Some((0..n).map(|f| (self.turn1(&poses[f], c.carrier).inverse() * self.turn1(&poses[f], *j)).normalize()).collect()),
                };
                match seq {
                    Some(s) if s.iter().all(|q| q.is_finite()) => rel.push(smooth_turns(&s, cycle)),
                    _ => ok = false,
                }
            }
            if !ok {
                continue;
            }
            rep.chains += 1;
            // 2. Put on (in order: a joint below another is set after it).
            for f in 0..n {
                let carry = self.turn1(&poses[f], c.carrier);
                for ((j, _), r) in c.links.iter().zip(&rel) {
                    let want_rot = (carry * r[f] * rot(&self.kh1_bind[*j])).normalize();
                    let q = (want_rot * rot(&poses[f][*j]).inverse()).normalize();
                    let about = at(&poses[f][*j]);
                    self.turn(&mut poses[f], *j, q, about);
                }
            }
            // 3. Clearance, stage by stage: one axis per stage (in the carrier's bind frame) for
            // the whole motion.
            for (pivot, only, share) in &c.stages {
                let (pivot, only, share) = (*pivot, only.as_slice(), *share);
                let (mut hang, mut out) = (Vec3::ZERO, Vec3::ZERO);
                for p in poses.iter() {
                    let back = self.turn1(p, c.carrier).inverse();
                    let piv = at(&p[pivot]);
                    for &k in only {
                        let pj = at(&p[c.nodes[k]]);
                        hang += back * (pj - piv);
                        for (o, _) in &c.obstacles {
                            let near = match *o {
                                Obstacle::Segment(a, b) => nearest_on_segment(pj, at(&p[a]), at(&p[b])).0,
                                Obstacle::Point(b, q) => (p[b] * self.kh1_bind[b].inverse()).transform_point3(q),
                            };
                            let v = pj - near;
                            let d = v.length().max(1e-3);
                            out += back * (v / d) / (d * d);
                        }
                    }
                }
                // (Only the part of "out" square to the chain can be reached by swinging it.) The
                // swing's axis: of the one straight out and seven more round the chain, the one
                // needing the least swing over the motion, clear on every frame where any is.
                let hang = hang.normalize_or_zero();
                let out = (out - hang * out.dot(hang)).normalize_or_zero();
                let side = hang.cross(out).normalize_or_zero();
                let candidates: Vec<Vec3> = if side != Vec3::ZERO { (0..8).map(|k| Quat::from_axis_angle(hang, k as f32 * std::f32::consts::FRAC_PI_4) * side).collect() } else { vec![Vec3::X] };
                let mut best: Option<(f32, Vec3, Vec<f32>)> = None;
                for axis in candidates {
                    let (need, fails) = self.needs(poses, c, (pivot, only), axis);
                    let cost = need.iter().map(|a| a * a).sum::<f32>() + fails as f32 * 1e3;
                    if best.as_ref().map_or(true, |b| cost < b.0) {
                        best = Some((cost, axis, need));
                    }
                }
                let Some((_, axis, need)) = best else { continue };
                if need.iter().any(|a| *a > 0.0) {
                    let push: Vec<f32> = spread(&need, cycle).into_iter().map(|a| a * share).collect();
                    for (f, &th) in push.iter().enumerate() {
                        if th > 0.0 {
                            rep.pushed += 1;
                            rep.max_push = rep.max_push.max(th.to_degrees());
                            let a = self.turn1(&poses[f], c.carrier) * axis;
                            let piv = at(&poses[f][pivot]);
                            self.turn(&mut poses[f], pivot, Quat::from_axis_angle(a, th), piv);
                        }
                    }
                }
            }
            for p in poses.iter() {
                rep.worst_inside = rep.worst_inside.max(self.inside(p, c, None).max(0.0));
            }
        }
        rep
    }

    /// KH1's parent of a bone (for checks).
    pub fn parent(&self, b: usize) -> Option<usize> {
        self.parent.get(b).copied().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothing_is_centred_and_loops() {
        // A turn that jumps for one frame: smoothing spreads it both ways, the same before as
        // after (no lag), and round the loop.
        let mut v = vec![Quat::IDENTITY; 10];
        v[0] = Quat::from_rotation_x(0.5);
        let s = smooth_turns(&v, Some(10));
        let a = |q: Quat| q.angle_between(Quat::IDENTITY);
        assert!((a(s[1]) - a(s[9])).abs() < 1e-4, "{} {}", a(s[1]), a(s[9]));
        assert!(a(s[0]) < 0.5 && a(s[0]) > 0.1);
    }

    #[test]
    fn spread_never_drops_below_the_need() {
        let v = vec![0.0, 0.0, 0.0, 0.3, 0.0, 0.0, 0.0, 0.0, 0.6, 0.0, 0.0, 0.0];
        let s = spread(&v, None);
        for (a, b) in v.iter().zip(&s) {
            assert!(b >= a);
        }
        // It starts before the contact and lets go after it.
        assert!(s[1] > 0.0 && s[5] > 0.0);
    }
}
