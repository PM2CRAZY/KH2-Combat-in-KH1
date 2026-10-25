//! A cloth physics layer for KH1 Sora's coat, shorts panels, belt chain and belt-front pieces, as
//! a tool to make their motion on a KH2 move (Mr Bugs, "Missing sleeves animation", after ^m18:
//! "we'll have you run the program tuned to kh1's own animations ... I can see how it looks in
//! game before a baking starts"). The same code runs live in the game (`kh1-hook`'s `pose`, on
//! the one move being worked on) and offline (`examples/cloth_sim.rs`), so a later bake made
//! offline gives what was seen in game. `docs/kh1-secondary-motion.md`, "Cloth physics (live,
//! to tune before a bake)".
//!
//! Ours, not KH1's or KH2's: neither game runs physics on these parts (both key them by hand,
//! Checked (files)); this only makes motion to judge and bake.
//!
//! **The model.** Each part is a chain of KH1 bones hanging from a carrier bone the animation
//! moves (the coat from the chest 78, its roots held there as in the bind; a shorts panel from
//! its thigh 17 / 43, the belt chain from the hips 3, the belt-front pieces from 4). Each frame:
//! 1. The chain is put in its **rest**: each bone's relation to its parent as KH1's own matching
//!    move holds it on average (for KH2's field run: KH1's field run, slot 5, averaged over its
//!    36 frames), carried by the carrier as drawn. Where each link's tip is then is its target.
//! 2. Each tip keeps an offset from its target, moved as a damped spring in the target's moving
//!    frame: `velocity = velocity × (1 − damping) − weight × (the target's acceleration) −
//!    stiffness × offset`. A steady run moves nothing (no acceleration); the body's bob, sway, leg
//!    swing and turns do. `weight` is how much of the body's acceleration the part lags behind (0:
//!    rigid with the body, 1: free mass).
//! 3. Each link keeps its length, swings at most `swing` degrees off its target, and is pushed out
//!    of the body (t569's clearance rules: the coat and the belt pieces outside the shorts round
//!    each thigh, 6.4 at the hip to 11.2 from mid-thigh plus 1, never more than the bind's
//!    distance, and no more than 5 nearer the hips' up axis than in the bind; each shorts panel
//!    no nearer its thigh's axis than 0.95 of its bind distance).
//! 4. Each link's bone is turned (its subtree with it) so it points at its simulated tip; the bone
//!    keeps its rest roll.
//!
//! One step per 60 Hz game frame (two at the 30 fps setting, between last frame's targets and
//! this frame's), none on a frame that didn't run (Frame Advance holding): the last result is put
//! on again. Deterministic: the same poses and placements give the same result.

use glam::{Mat4, Quat, Vec3};
use kh2_formats::anim::Bone;

/// The parts, in the settings' order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Coat,
    Shorts,
    Belt,
    Front,
}

impl Part {
    pub const ALL: [Part; 4] = [Part::Coat, Part::Shorts, Part::Belt, Part::Front];
    pub fn name(self) -> &'static str {
        match self {
            Part::Coat => "Coat",
            Part::Shorts => "Shorts",
            Part::Belt => "Belt Chain",
            Part::Front => "Belt Front",
        }
    }
    fn index(self) -> usize {
        self as usize
    }
}

/// One part's physics numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// The pull back to the rest, a share of the offset per 60 Hz step (0..1).
    pub stiffness: f32,
    /// The swing's own motion lost per step (0..1).
    pub damping: f32,
    /// How much of the body's acceleration the part lags behind (0 rigid .. 1 free).
    pub weight: f32,
    /// The most a link swings off its rest, degrees.
    pub swing: f32,
}

/// Every part's numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub parts: [Params; 4],
}

impl Settings {
    /// Tuned offline against KH1's own field run (slot 5): each part's mean swing off its rest on
    /// KH2's field run, settled, as near as the search got to KH1's own mean swing of that part
    /// in its run (`examples/cloth_sim.rs -- ... fit`; numbers in the docs).
    pub const TUNED: Settings = Settings {
        parts: [
            Params { stiffness: 0.02, damping: 0.30, weight: 0.25, swing: 40.0 },
            Params { stiffness: 0.20, damping: 0.45, weight: 0.25, swing: 30.0 },
            Params { stiffness: 0.06, damping: 0.05, weight: 0.25, swing: 100.0 },
            Params { stiffness: 0.06, damping: 0.45, weight: 0.50, swing: 60.0 },
        ],
    };

    pub fn get(&self, p: Part) -> Params {
        self.parts[p.index()]
    }

    /// A short number naming these settings (for the trace and the panel): the tuned ones are 0,
    /// any other a hash of the numbers (FNV-1a, 16 bits, never 0).
    pub fn id(&self) -> u32 {
        if *self == Self::TUNED {
            return 0;
        }
        let mut h: u32 = 0x811C_9DC5;
        for p in &self.parts {
            for v in [p.stiffness, p.damping, p.weight, p.swing] {
                for b in v.to_bits().to_le_bytes() {
                    h = (h ^ b as u32).wrapping_mul(0x0100_0193);
                }
            }
        }
        ((h ^ (h >> 16)) & 0xFFFF).max(1)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::TUNED
    }
}

/// KH1 bones (`examples/zz_probe` / the skeleton, Checked (files)).
const SPINE: usize = 71;
/// The chest (neck base), which carries the jacket above the coat: the coat's roots are held on it
/// as in the bind. KH1 holds 71 and 78 rigid in its motions and never turns the coat's roots
/// (Checked (files), its field run: 0.0007 degrees), so on KH1 the coat hangs from the chest as
/// much as from 71; our retarget turns 71 only 67% toward KH2's chest, and a coat hung on 71
/// sheared against the jacket above it (the tester's stretch by the necklace).
const CHEST: usize = 78;
const HIPS: usize = 3;
const THIGHS: [(usize, usize); 2] = [(17, 18), (43, 44)];
const COAT: [usize; 9] = [72, 248, 252, 256, 260, 264, 268, 272, 276];
const SHORTS: [[usize; 4]; 2] = [[26, 30, 34, 38], [52, 56, 60, 64]];
/// The belt chain (on the hips): 10, 11 at its place, then 12..15.
const BELT: [usize; 6] = [10, 11, 12, 13, 14, 15];
/// The belt-front pieces (on 4): 5 → 6, then 7 / 8 at 6's place → 9.
const FRONT: [usize; 5] = [5, 6, 7, 8, 9];
const FRONT_LINKS: [(usize, usize); 2] = [(5, 6), (8, 9)];
const SHORTS_MARGIN: f32 = 1.0;
const PANEL_CLEAR: f32 = 0.95;
const HIPS_IN: f32 = 5.0;
/// A target moving this far in a frame, or this many frames missed: started over (a warp).
const WARP: f32 = 200.0;
const MAX_STEPS: u32 = 30;

fn shorts_r(t: f32) -> f32 {
    if t < 0.5 { 6.4 + (11.2 - 6.4) * t / 0.5 } else { 11.2 }
}

fn at(m: &Mat4) -> Vec3 {
    m.w_axis.truncate()
}

fn nearest(p: Vec3, a: Vec3, b: Vec3) -> (Vec3, f32) {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    (a + ab * t, t)
}

#[derive(Clone, Copy, Debug)]
enum Clear {
    /// No nearer than this.
    Fixed(f32),
    /// Outside the shorts round this thigh (`SHORTS_MARGIN`), never more than this.
    Shorts(f32),
}

#[derive(Clone, Debug)]
struct Chain {
    part: Part,
    carrier: usize,
    /// The bones put in their rest, parent before child.
    bones: Vec<usize>,
    /// The first bone held on the carrier at this relation instead of its parent's (the coat's
    /// root on the chest, as in the bind).
    held: Option<Mat4>,
    /// (bone turned, its tip): the simulated points are the tips.
    links: Vec<(usize, usize)>,
    /// Segments (a, b) and each tip's clearance from them.
    obstacles: Vec<((usize, usize), Vec<Clear>)>,
}

#[derive(Clone, Debug, Default)]
struct State {
    offset: Vec<Vec3>,
    velocity: Vec<Vec3>,
    /// Last step's targets and their motion over it.
    target: Vec<Vec3>,
    moved: Vec<Vec3>,
}

/// What a step did (the trace's and the log's numbers).
#[derive(Clone, Copy, Debug, Default)]
pub struct Report {
    pub steps: u32,
    /// Started over this frame (first frame, a warp, a long gap).
    pub restarted: bool,
    /// The largest link swing off its rest after the step (degrees), and the tips pushed out.
    pub max_swing: f32,
    pub pushed: u32,
}

pub struct ClothSim {
    below: Vec<Vec<usize>>,
    parent: Vec<Option<usize>>,
    rest: Vec<Mat4>,
    chains: Vec<Chain>,
    state: Vec<State>,
    started: bool,
    /// Every simulated tip (model space) as last put on, chain by chain: for frames with no step.
    last: Vec<Vec<Vec3>>,
}

/// A weighted average of turns (sign-aligned to the first).
fn average(qs: impl Iterator<Item = Quat>) -> Option<Quat> {
    let mut first: Option<glam::Vec4> = None;
    let mut s = glam::Vec4::ZERO;
    for q in qs {
        let v = glam::Vec4::from(q);
        let r = *first.get_or_insert(v);
        s += if v.dot(r) < 0.0 { -v } else { v };
    }
    let q = Quat::from_vec4(s);
    (q.length_squared() > 1e-12).then(|| q.normalize())
}

impl ClothSim {
    /// `kh1`: KH1 Sora's skeleton; `rest`: KH1's own motion whose average relation of each part
    /// to its parent is the rest (None: the bind's).
    pub fn new(kh1: &[Bone], motion: Option<&crate::mset::Motion>) -> ClothSim {
        let n = kh1.len();
        let parent: Vec<Option<usize>> = kh1.iter().enumerate().map(|(i, b)| (b.parent >= 0 && (b.parent as usize) < i).then_some(b.parent as usize)).collect();
        let mut kids = vec![Vec::new(); n];
        for (i, p) in parent.iter().enumerate() {
            if let Some(p) = p {
                kids[*p].push(i);
            }
        }
        let mut below = vec![Vec::new(); n];
        for i in (0..n).rev() {
            let mut v: Vec<usize> = Vec::new();
            for &c in &kids[i] {
                v.push(c);
                v.extend(below[c].iter().copied());
            }
            below[i] = v;
        }
        let bind = kh2_anim::Pose::bind(kh1).bones;
        let local = |p: &[Mat4], b: usize| match parent[b] {
            Some(q) => p[q].inverse() * p[b],
            None => p[b],
        };
        // The rest: each bone's relation to its parent, averaged over KH1's motion.
        let mut rest: Vec<Mat4> = (0..n).map(|b| local(&bind, b)).collect();
        let mut chains = Vec::new();
        let ok = |b: usize| b < n;
        if !(ok(279) && ok(SPINE) && ok(CHEST)) {
            return ClothSim { below, parent, rest, chains, state: Vec::new(), started: false, last: Vec::new() };
        }
        let bind_d = |j: usize, (a, b): (usize, usize)| nearest(at(&bind[j]), at(&bind[a]), at(&bind[b])).0.distance(at(&bind[j]));
        let outside_shorts = |tips: &[usize]| -> Vec<((usize, usize), Vec<Clear>)> {
            let mut o: Vec<((usize, usize), Vec<Clear>)> = THIGHS.iter().map(|&t| (t, tips.iter().map(|&j| Clear::Shorts(bind_d(j, t))).collect())).collect();
            o.push(((HIPS, SPINE), tips.iter().map(|&j| Clear::Fixed(bind_d(j, (HIPS, SPINE)) - HIPS_IN)).collect()));
            o
        };
        for &r in &COAT {
            let tips = [r + 2, r + 3];
            let held = bind[CHEST].inverse() * bind[r];
            chains.push(Chain { part: Part::Coat, carrier: CHEST, bones: (r..=r + 3).collect(), held: Some(held), links: vec![(r + 1, r + 2), (r + 2, r + 3)], obstacles: outside_shorts(&tips) });
        }
        for (side, panels) in SHORTS.iter().enumerate() {
            let thigh = THIGHS[side];
            for &r in panels {
                let tips = [r + 2, r + 3];
                chains.push(Chain { part: Part::Shorts, carrier: thigh.0, bones: (r..=r + 3).collect(), held: None, links: vec![(r + 1, r + 2), (r + 2, r + 3)], obstacles: vec![(thigh, tips.iter().map(|&j| Clear::Fixed(bind_d(j, thigh) * PANEL_CLEAR)).collect())] });
            }
        }
        {
            let tips: Vec<usize> = BELT[2..].to_vec();
            chains.push(Chain { part: Part::Belt, carrier: HIPS, bones: BELT.to_vec(), held: None, links: BELT[1..].windows(2).map(|w| (w[0], w[1])).collect(), obstacles: outside_shorts(&tips) });
        }
        {
            let tips: Vec<usize> = FRONT_LINKS.iter().map(|l| l.1).collect();
            chains.push(Chain { part: Part::Front, carrier: 4, bones: FRONT.to_vec(), held: None, links: FRONT_LINKS.to_vec(), obstacles: outside_shorts(&tips) });
        }
        // (Every chain bone's parent is the one before it, or the carrier; a held root may have
        // another parent.)
        chains.retain(|c| c.bones.iter().enumerate().all(|(i, &b)| (i == 0 && c.held.is_some()) || parent[b] == Some(if i == 0 { c.carrier } else { c.bones[i - 1] })));
        if let Some(m) = motion {
            let frames = m.frames.max(1);
            let poses: Vec<Vec<Mat4>> = (0..frames).map(|f| kh2_anim::Pose::bind(&m.apply(kh1, f as f32)).bones).collect();
            for c in &chains {
                for &b in c.bones.iter().skip(c.held.is_some() as usize) {
                    let ls: Vec<Mat4> = poses.iter().map(|p| local(p, b)).collect();
                    let q = average(ls.iter().map(|l| l.to_scale_rotation_translation().1));
                    let t = ls.iter().map(|l| at(l)).sum::<Vec3>() / ls.len() as f32;
                    if let Some(q) = q.filter(|q| q.is_finite()) {
                        rest[b] = Mat4::from_rotation_translation(q, t);
                    }
                }
            }
        }
        let state = vec![State::default(); chains.len()];
        ClothSim { below, parent, rest, chains, state, started: false, last: Vec::new() }
    }

    /// The chains built (0 if KH1's skeleton isn't Sora's).
    pub fn chains(&self) -> usize {
        self.chains.len()
    }

    /// Starts over (a new room, a warp, the move started again after a gap).
    pub fn reset(&mut self) {
        self.started = false;
    }

    /// The rest's relation of bone `b` to its parent (for the checks).
    pub fn rest_local(&self, b: usize) -> Option<Mat4> {
        self.rest.get(b).copied()
    }

    /// Every simulated bone and its tip: (part, carrier, bone turned, tip).
    pub fn links(&self) -> Vec<(Part, usize, usize, usize)> {
        self.chains.iter().flat_map(|c| c.links.iter().map(move |&(b, t)| (c.part, c.carrier, b, t))).collect()
    }

    fn turn(&self, pose: &mut [Mat4], b: usize, q: Quat, about: Vec3) {
        if !q.is_finite() {
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

    /// Puts every chain in its rest (on its carrier as `pose` has it).
    fn put_rest(&self, pose: &mut [Mat4]) {
        for c in &self.chains {
            for (i, &b) in c.bones.iter().enumerate() {
                match (i, c.held) {
                    (0, Some(h)) => pose[b] = pose[c.carrier] * h,
                    _ => {
                        if let Some(p) = self.parent[b] {
                            pose[b] = pose[p] * self.rest[b];
                        }
                    }
                }
            }
        }
    }

    /// One frame: `pose` (KH1 model space, unscaled), `to_world` its placement, `steps` the 60 Hz
    /// frames since the last call (0: none ran, the last result is put on again), `set` the
    /// numbers. The chains' bones in `pose` are replaced.
    pub fn step(&mut self, pose: &mut [Mat4], to_world: &Mat4, steps: u32, set: &Settings) -> Report {
        let mut rep = Report { steps, ..Default::default() };
        if self.chains.is_empty() || pose.len() < self.rest.len() || !pose.iter().all(|m| m.is_finite()) || !to_world.is_finite() {
            return rep;
        }
        self.put_rest(pose);
        let back = to_world.inverse();
        let scale = to_world.x_axis.truncate().length().max(1e-3);
        let world = |m: &Mat4| to_world.transform_point3(at(m));
        // The targets this frame (world).
        let targets: Vec<Vec<Vec3>> = self.chains.iter().map(|c| c.links.iter().map(|&(_, t)| world(&pose[t])).collect()).collect();
        let warp = self.started && self.state.iter().zip(&targets).any(|(s, t)| s.target.iter().zip(t).any(|(a, b)| a.distance(*b) > WARP));
        if !self.started || warp || steps > MAX_STEPS || self.last.len() != self.chains.len() {
            for (s, t) in self.state.iter_mut().zip(&targets) {
                *s = State { offset: vec![Vec3::ZERO; t.len()], velocity: vec![Vec3::ZERO; t.len()], target: t.clone(), moved: vec![Vec3::ZERO; t.len()] };
            }
            self.started = true;
            rep.restarted = true;
            self.last = targets.iter().map(|t| t.iter().map(|p| back.transform_point3(*p)).collect()).collect();
        }
        let n = steps.min(MAX_STEPS);
        for ci in 0..self.chains.len() {
            let c = &self.chains[ci];
            let pr = set.get(c.part);
            let swing = pr.swing.to_radians();
            // The chain's start (the first link's bone, as the carrier holds it) and its obstacles
            // this frame (world).
            let anchor = world(&pose[c.links[0].0]);
            let obst: Vec<(Vec3, Vec3)> = c.obstacles.iter().map(|((a, b), _)| (world(&pose[*a]), world(&pose[*b]))).collect();
            let st = &mut self.state[ci];
            let from = st.target.clone();
            for k in 1..=n {
                let w = k as f32 / n as f32;
                for i in 0..c.links.len() {
                    let t = from[i].lerp(targets[ci][i], w);
                    let moved = t - st.target[i];
                    let accel = moved - st.moved[i];
                    st.target[i] = t;
                    st.moved[i] = moved;
                    st.velocity[i] = st.velocity[i] * (1.0 - pr.damping) - accel * pr.weight - st.offset[i] * pr.stiffness;
                    let before = st.offset[i];
                    st.offset[i] += st.velocity[i];
                    // The link: its length, its swing, out of the body.
                    let start = if i == 0 { anchor } else { st.target[i - 1] + st.offset[i - 1] };
                    let rest_start = if i == 0 { anchor } else { st.target[i - 1] };
                    let rest_dir = (t - rest_start).normalize_or_zero();
                    let len = (t - rest_start).length();
                    let mut p = t + st.offset[i];
                    let mut d = (p - start).normalize_or(rest_dir);
                    let ang = d.angle_between(rest_dir);
                    if ang > swing && ang.is_finite() {
                        d = Quat::from_rotation_arc(rest_dir, d).slerp(Quat::IDENTITY, 1.0 - swing / ang) * rest_dir;
                    }
                    p = start + d * len;
                    for ((_, clears), (a, b)) in c.obstacles.iter().zip(&obst) {
                        let (q, s) = nearest(p, *a, *b);
                        let need = match clears[i] {
                            Clear::Fixed(r) => r,
                            Clear::Shorts(cap) => (shorts_r(s) + SHORTS_MARGIN).min(cap),
                        } * scale;
                        let v = p - q;
                        let dist = v.length();
                        if dist < need && dist > 1e-4 {
                            p = q + v * (need / dist);
                            p = start + (p - start).normalize_or(d) * len;
                            if k == n {
                                rep.pushed += 1;
                            }
                        }
                    }
                    st.offset[i] = p - t;
                    st.velocity[i] = st.offset[i] - before;
                }
            }
            if n > 0 {
                self.last[ci] = st.target.iter().zip(&st.offset).map(|(t, o)| back.transform_point3(*t + *o)).collect();
            }
        }
        // On: each link's bone aimed at its tip (model space). On a frame with no step, the tips
        // are where they were last (relative to the model).
        for (ci, c) in self.chains.iter().enumerate() {
            for (i, &(b, tip)) in c.links.iter().enumerate() {
                let Some(&want) = self.last[ci].get(i) else { continue };
                let piv = at(&pose[b]);
                let (now, to) = ((at(&pose[tip]) - piv).normalize_or_zero(), (want - piv).normalize_or_zero());
                if now == Vec3::ZERO || to == Vec3::ZERO {
                    continue;
                }
                rep.max_swing = rep.max_swing.max(now.angle_between(to).to_degrees());
                let q = Quat::from_rotation_arc(now, to);
                self.turn(pose, b, q, piv);
            }
        }
        rep
    }
}
