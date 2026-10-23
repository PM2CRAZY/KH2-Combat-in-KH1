//! The joint pass: expressions, constraints, limits and IK, in the order
//! KH2's motion evaluator applies them. Plain-language description in
//! `docs/animation.md`; this is our own implementation of it.
//!
//! Matrices here are glam's column-vector convention: `world = parent * local`.

use glam::{EulerRot, Mat3, Mat4, Vec3, Vec4};
use kh2_formats::anim::{Bone, Constraint, Interpolated, Limiter};

use crate::{parent_index, scaled, Srt};

const DEG: f32 = std::f32::consts::PI / 180.0;
const RAD: f32 = 180.0 / std::f32::consts::PI;

// Joint-table flags (third byte of each entry).
const IK_MASK: u8 = 0x03;
const F_TRANSLATION: u8 = 0x04;
const F_ROTATION: u8 = 0x08;
const F_CALCULATED: u8 = 0x20;
const F_MATRIX_TO_ROT: u8 = 0x40;
const F_EXTERNAL: u8 = 0x80;

// Constraint kinds.
const C_POSITION: u8 = 0;
const C_ORIENTATION: u8 = 2;
const C_DIRECTION: u8 = 3;
const C_UP: u8 = 4;
const C_TWO_POINTS: u8 = 5;
const C_SCALE: u8 = 6;
const C_POSITION_LIMIT: u8 = 12;
const C_ROTATION_LIMIT: u8 = 13;

/// Euler XYZ (X applied first) from a rotation matrix.
pub fn euler_from_matrix(m: &Mat4) -> Vec3 {
    let r = Mat3::from_cols(m.x_axis.truncate().normalize_or_zero(), m.y_axis.truncate().normalize_or_zero(), m.z_axis.truncate().normalize_or_zero());
    let (z, y, x) = glam::Quat::from_mat3(&r).to_euler(EulerRot::ZYX);
    Vec3::new(x, y, z)
}

fn rot_x(a: f32) -> Mat4 {
    if a == 0.0 {
        Mat4::IDENTITY
    } else {
        Mat4::from_rotation_x(a)
    }
}

/// KH2's analytic IK. `origin` and `target` are in the chain's parent frame.
/// Returns the upper joint's rotation (no translation) and, for two-bone
/// chains (`l2` given), the lower joint's rotation relative to the upper.
///
/// - The upper joint's x axis aims at the target; the bend plane comes from
///   `pole` (up-vector constraint) or else from the aim direction alone.
/// - `roll1` / `roll2` are the joints' own X rotations, applied as a twist.
/// - The knee bends by the law of cosines; `flip` bends the other way.
pub fn ik_solve(origin: Vec3, target: Vec3, l1: f32, l2: Option<f32>, flip: bool, roll1: f32, roll2: f32, pole: Option<Vec3>) -> (Mat4, Option<Mat4>) {
    let mut x = target - origin;
    let d = x.length();
    if d != 0.0 {
        x /= d;
    }
    let (r1, r2) = match pole {
        None => {
            let w = Vec3::new(-x.y, x.x, 0.0).normalize_or_zero();
            (w, x.cross(w).normalize_or_zero())
        }
        Some(p) => {
            let u = (p - origin).normalize_or_zero();
            let r2 = x.cross(u).normalize_or_zero();
            (r2.cross(x).normalize_or_zero(), r2)
        }
    };
    let mut basis = Mat4::from_cols(x.extend(0.0), r1.extend(0.0), r2.extend(0.0), Vec4::W);
    basis *= rot_x(roll1);
    let Some(l2) = l2 else { return (basis, None) };
    let denom = 2.0 * l1 * l2;
    let mut c = if denom != 0.0 { (d * d - l1 * l1 - l2 * l2) / denom } else { 1.0 };
    c = c.clamp(-1.0, 1.0);
    let mut s = (1.0 - c * c).max(0.0).sqrt();
    if flip {
        s = -s;
    }
    let lower = Mat4::from_cols(Vec4::new(c, s, 0.0, 0.0), Vec4::new(-s, c, 0.0, 0.0), Vec4::Z, Vec4::W) * rot_x(roll2);
    let a = (c * l2 + l1) * d;
    let b = -(s * l2 * d);
    let n = (a * a + b * b).sqrt();
    let (cp, sp) = if n == 0.0 { (1.0, 0.0) } else { (a / n, b / n) };
    let r = Mat4::from_cols(Vec4::new(cp, sp, 0.0, 0.0), Vec4::new(-sp, cp, 0.0, 0.0), Vec4::Z, Vec4::W);
    (basis * r, Some(lower))
}

/// Applies a position or rotation limiter. `center` is the reference point
/// for local (non-global) limits.
fn limit(l: &Limiter, center: Vec3, value: Vec3) -> Vec3 {
    let kind = l.flags & 7;
    let global = l.flags & 8 != 0;
    let origin = if kind != 0 && !global { center } else { Vec3::ZERO };
    let v = value - origin;
    let (w, strength) = (l.damping_width, l.damping_strength);
    if kind == 1 {
        let radius = l.min[0];
        let len = v.length();
        if strength == 0.0 {
            // The game normalises and rescales by the same length here, so
            // a sphere limit without damping leaves the value unchanged.
            return value;
        }
        let r = radius - w;
        let excess = len - r;
        if excess <= 0.0 || len == 0.0 {
            return value;
        }
        let e = ((excess / w) * (strength - 1.0)).exp();
        return origin + v / len * ((1.0 - e) * w + r);
    }
    let mut out = value;
    let axes = [(0usize, 4u32, 5u32), (1, 6, 7), (2, 8, 9)];
    for (i, min_bit, max_bit) in axes {
        let (lo, hi) = (l.min[i], l.max[i]);
        let has_min = l.flags >> min_bit & 1 != 0;
        let has_max = l.flags >> max_bit & 1 != 0;
        if strength == 0.0 {
            if has_min && v[i] < lo {
                out[i] = lo + origin[i];
            } else if has_max && hi < v[i] {
                out[i] = hi + origin[i];
            }
        } else {
            let soft = |m: f32, over: f32| (1.0 - ((over / w) * (strength - 1.0)).exp()) * w + m;
            let m = w + lo;
            if has_min && m - v[i] > 0.0 {
                out[i] = soft(m, m - v[i]) + origin[i];
            } else {
                let m = hi - w;
                if has_max && v[i] - m > 0.0 {
                    out[i] = soft(m, v[i] - m) + origin[i];
                }
            }
        }
    }
    out
}

pub(crate) struct State<'a> {
    pub motion: &'a Interpolated,
    pub bc: usize,
    pub srt: Vec<Srt>,
    pub length: Vec<f32>,
    pub parent: Vec<i16>,
    pub local: Vec<Mat4>,
    pub unscaled: Vec<Mat4>,
    pub world: Vec<Mat4>,
    pub wscale: Vec<Vec3>,
    pub active: Vec<bool>,
    /// The key time in the motion's own frame rate (for expressions).
    pub lfr: f32,
}

impl<'a> State<'a> {
    pub fn new(skeleton: &'a [Bone], motion: &'a Interpolated, t: f32) -> State<'a> {
        let bc = motion.bone_count.min(skeleton.len());
        let n = bc + motion.ik_helpers.len();
        let mut srt = Vec::with_capacity(n);
        let mut length = Vec::with_capacity(n);
        let mut parent = Vec::with_capacity(n);
        for b in skeleton[..bc].iter().chain(&motion.ik_helpers) {
            srt.push(Srt::from_bone(b));
            length.push(b.length);
            parent.push(b.parent);
        }
        for &(j, ch, v) in &motion.initial_pose {
            if let Some(s) = srt.get_mut(j.max(0) as usize) {
                s.set(ch as u8, v);
            }
        }
        for c in &motion.forward {
            if let Some(s) = srt.get_mut(c.joint.max(0) as usize) {
                s.set(c.channel, c.eval(t));
            }
        }
        for c in &motion.inverse {
            if let Some(s) = srt.get_mut(bc + c.joint.max(0) as usize) {
                s.set(c.channel, c.eval(t));
            }
        }
        let local = srt.iter().map(|s| s.rt_matrix()).collect();
        let active = motion
            .constraints
            .iter()
            .map(|c| constraint_active(motion, c, t))
            .collect();
        State {
            motion,
            bc,
            srt,
            length,
            parent,
            local,
            unscaled: vec![Mat4::IDENTITY; n],
            world: vec![Mat4::IDENTITY; n],
            wscale: vec![Vec3::ONE; n],
            active,
            lfr: t * motion.fps / 60.0,
        }
    }

    fn p(&self, j: usize) -> Option<usize> {
        parent_index(self.parent[j], j, self.srt.len())
    }

    fn wpos(&self, j: usize) -> Vec3 {
        self.world.get(j).map_or(Vec3::ZERO, |m| m.w_axis.truncate())
    }

    fn order(&self) -> Vec<(usize, u8)> {
        let n = self.srt.len();
        if self.motion.joints.is_empty() {
            return (0..n).map(|j| (j, 0)).collect();
        }
        self.motion
            .joints
            .iter()
            .filter(|e| e.joint >= 0 && (e.joint as usize) < n)
            .map(|e| (e.joint as usize, e.flags))
            .collect()
    }

    pub fn run(&mut self, constraints: bool) {
        let order = self.order();
        let cons = &self.motion.constraints;
        let exprs = &self.motion.expressions;
        let (mut ci, mut ei, mut xi) = (0usize, 0usize, 0usize);
        for (j, flags) in order {
            let mut f = if constraints { flags } else { 0 };
            let p = self.p(j);

            if constraints {
                while ei < exprs.len() && exprs[ei].joint as usize == j {
                    let v = self.expr(exprs[ei].node);
                    let ch = exprs[ei].channel;
                    let v = if (3..=5).contains(&ch) { v * DEG } else { v };
                    self.srt[j].set(ch as u8, v);
                    ei += 1;
                    f &= !F_CALCULATED;
                }
            }

            // Constraints on this joint.
            let (mut pos1, mut pos2) = (None::<usize>, None::<usize>);
            let mut rotc: Option<&Constraint> = None;
            let mut scalec: Option<&Constraint> = None;
            let mut plim: Option<(usize, usize)> = None;
            let mut rlim: Option<usize> = None;
            while constraints && ci < cons.len() && cons[ci].constrained as usize == j {
                let c = &cons[ci];
                if self.active[ci] {
                    let src = c.source.max(0) as usize;
                    match c.kind {
                        C_POSITION => {
                            if pos1.is_none() {
                                pos1 = Some(src);
                            } else if pos2.is_none() {
                                pos2 = Some(src);
                            }
                        }
                        C_TWO_POINTS => {
                            pos1 = Some(src);
                            pos2 = Some(c.limiter.max(0) as usize);
                            rotc = Some(c);
                        }
                        C_ORIENTATION | C_DIRECTION | C_UP => rotc = Some(c),
                        C_SCALE => scalec = Some(c),
                        C_POSITION_LIMIT => plim = Some((c.limiter.max(0) as usize, src)),
                        C_ROTATION_LIMIT => rlim = Some(c.limiter.max(0) as usize),
                        _ => {}
                    }
                }
                ci += 1;
            }

            // IK chain end and its target constraint.
            let mut ik = f & IK_MASK;
            let mut target: Option<&Constraint> = None;
            if ik != 0 {
                let end = if f & F_EXTERNAL != 0 {
                    let e = self.motion.external_effectors.get(xi).map_or(0, |&e| e.max(0) as usize);
                    xi += 1;
                    e
                } else {
                    j + if ik == 1 { 1 } else { 2 }
                };
                let mut k = ci;
                while k < cons.len() {
                    let c = &cons[k];
                    if self.active[k] && c.constrained as usize == end && c.kind & 0xFA == 0 && c.kind != C_UP {
                        target = Some(c);
                        break;
                    }
                    k += 1;
                }
                ci = if target.is_some() { k + 1 } else { cons.len() };
                if target.is_none() {
                    ik = 0;
                }
            }

            // Accumulated scale.
            if let Some(c) = scalec {
                let src = c.source.max(0) as usize;
                self.wscale[j] = self.wscale.get(src).copied().unwrap_or(Vec3::ONE);
                self.srt[j].scale = match p {
                    Some(p) => self.wscale[j] / self.wscale[p],
                    None => self.wscale[j],
                };
            } else {
                self.wscale[j] = p.map_or(self.srt[j].scale, |p| self.wscale[p] * self.srt[j].scale);
            }

            if f & F_CALCULATED != 0 {
                // Already placed by an IK chain earlier in the pass.
                if f & F_MATRIX_TO_ROT != 0 {
                    self.srt[j].rotation = euler_from_matrix(&self.local[j]);
                }
                continue;
            }

            if ik != 0 {
                self.ik_chain(j, p, ik, f, target.expect("ik target"), rotc, &mut ci);
                continue;
            }

            self.plain(j, p, f, pos1, pos2, rotc, plim, rlim);
        }
    }

    fn ik_chain(&mut self, j: usize, p: Option<usize>, ik: u8, f: u8, target: &Constraint, rotc: Option<&Constraint>, ci: &mut usize) {
        let lpos = self.srt[j].translation;
        let wpos = p.map_or(lpos, |p| self.world[p].transform_point3(lpos));
        let iup = p.map_or(Mat4::IDENTITY, |p| self.unscaled[p].inverse());
        let origin = iup.transform_point3(wpos);
        let tw = if target.kind == C_TWO_POINTS {
            (self.wpos(target.source.max(0) as usize) + self.wpos(target.limiter.max(0) as usize)) * 0.5
        } else {
            self.wpos(target.source.max(0) as usize)
        };
        let tgt = iup.transform_point3(tw);
        let pole = rotc.filter(|c| c.kind == C_UP).map(|c| iup.transform_point3(self.wpos(c.source.max(0) as usize)));
        let l1 = self.length[j] * self.wscale[j].x;
        let end = target.constrained.max(0) as usize;

        if ik == 1 {
            let (mut upper, _) = ik_solve(origin, tgt, l1, None, false, self.srt[j].rotation.x, 0.0, pole);
            if f & F_MATRIX_TO_ROT != 0 {
                self.srt[j].rotation = euler_from_matrix(&upper);
            }
            upper.w_axis = lpos.extend(1.0);
            self.local[j] = upper;
            self.unscaled[j] = p.map_or(upper, |p| self.unscaled[p] * upper);
            self.world[j] = scaled(self.unscaled[j], self.wscale[j], wpos);
            if end < self.world.len() {
                let e = self.world[j].transform_point3(self.srt[end].translation);
                self.world[end].w_axis = e.extend(1.0);
            }
            return;
        }

        // Two-bone chain: j (upper), k (lower), then the end effector.
        let k = j + 1;
        let cons = &self.motion.constraints;
        while *ci < cons.len() && cons[*ci].constrained as usize == k {
            *ci += 1;
        }
        if k >= self.srt.len() {
            return;
        }
        self.wscale[k] = self.wscale[j] * self.srt[k].scale;
        let l2 = self.length[k] * self.wscale[k].x;
        let (mut upper, lower) = ik_solve(origin, tgt, l1, Some(l2), ik == 3, self.srt[j].rotation.x, self.srt[k].rotation.x, pole);
        let mut lower = lower.unwrap_or(Mat4::IDENTITY);
        if f & F_MATRIX_TO_ROT != 0 {
            self.srt[j].rotation = euler_from_matrix(&upper);
        }
        upper.w_axis = lpos.extend(1.0);
        self.local[j] = upper;
        self.unscaled[j] = p.map_or(upper, |p| self.unscaled[p] * upper);
        self.world[j] = scaled(self.unscaled[j], self.wscale[j], wpos);

        lower.w_axis = Vec4::new(self.length[j], 0.0, 0.0, 1.0);
        self.local[k] = lower;
        self.unscaled[k] = self.unscaled[j] * lower;
        let kpos = self.world[j].transform_point3(lower.w_axis.truncate());
        self.world[k] = scaled(self.unscaled[k], self.wscale[k], kpos);
        if end < self.world.len() {
            let e = self.world[k].transform_point3(self.srt[end].translation);
            self.world[end].w_axis = e.extend(1.0);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn plain(&mut self, j: usize, p: Option<usize>, f: u8, pos1: Option<usize>, pos2: Option<usize>, rotc: Option<&Constraint>, plim: Option<(usize, usize)>, rlim: Option<usize>) {
        let limiters = &self.motion.limiters;
        let ip = p.map_or(Mat4::IDENTITY, |p| self.world[p].inverse());
        let to_world = |s: &Self, v: Vec3| p.map_or(v, |p| s.world[p].transform_point3(v));

        let (lpos, wpos) = if f & F_TRANSLATION != 0 {
            let w = self.wpos(j);
            (ip.transform_point3(w), w)
        } else if pos1.is_none() && pos2.is_none() {
            let mut l = self.srt[j].translation;
            if let Some((li, src)) = plim {
                if let Some(lim) = limiters.get(li) {
                    l = limit(lim, ip.transform_point3(self.wpos(src)), l);
                }
            }
            (l, to_world(self, l))
        } else {
            let a = self.wpos(pos1.unwrap_or(0));
            let w = match pos2 {
                Some(b) => (a + self.wpos(b)) * 0.5,
                None => a,
            };
            let mut l = ip.transform_point3(w);
            let mut w = w;
            if let Some((li, src)) = plim {
                if let Some(lim) = limiters.get(li) {
                    l = limit(lim, ip.transform_point3(self.wpos(src)), l);
                    w = to_world(self, l);
                }
            }
            (l, w)
        };

        let kind = rotc.map(|c| c.kind);
        match kind {
            Some(C_ORIENTATION) => {
                let src = rotc.unwrap().source.max(0) as usize;
                self.unscaled[j] = self.unscaled.get(src).copied().unwrap_or(Mat4::IDENTITY);
            }
            Some(C_DIRECTION) | Some(C_TWO_POINTS) => {
                let c = rotc.unwrap();
                let src = if c.kind == C_DIRECTION { c.source } else { c.limiter }.max(0) as usize;
                let d = ip.transform_point3(self.wpos(src)) - lpos;
                let x = d.normalize_or_zero();
                let side = x.cross(Vec3::Y);
                let y = side.cross(d).normalize_or_zero();
                let z = x.cross(y).normalize_or_zero();
                let mut m = Mat4::from_cols(x.extend(0.0), y.extend(0.0), z.extend(0.0), Vec4::W) * rot_x(self.srt[j].rotation.x);
                m.w_axis = lpos.extend(1.0);
                self.unscaled[j] = p.map_or(m, |p| self.unscaled[p] * m);
            }
            _ => {
                if let Some(li) = rlim {
                    if let Some(lim) = limiters.get(li) {
                        let r = self.srt[j].rotation;
                        self.srt[j].rotation = limit(lim, r, r);
                    }
                }
                let mut l = Mat4::from_quat(self.srt[j].quat());
                l.w_axis = lpos.extend(1.0);
                self.local[j] = l;
                self.unscaled[j] = match p {
                    Some(p) if f & F_ROTATION == 0 => self.unscaled[p] * l,
                    _ => l,
                };
            }
        }
        self.world[j] = scaled(self.unscaled[j], self.wscale[j], wpos);
        if matches!(kind, Some(C_ORIENTATION | C_DIRECTION | C_TWO_POINTS)) || f & F_ROTATION != 0 {
            let inv = p.map_or(Mat4::IDENTITY, |p| self.unscaled[p].inverse());
            let mut l = inv * self.unscaled[j];
            l.w_axis = lpos.extend(1.0);
            self.local[j] = l;
            if f & F_MATRIX_TO_ROT != 0 {
                self.srt[j].rotation = euler_from_matrix(&l);
            }
        }
    }

    /// Evaluates expression node `i` (degrees for angles).
    fn expr(&mut self, i: i16) -> f32 {
        let nodes = &self.motion.nodes;
        let Some(n) = (i >= 0).then(|| nodes.get(i as usize)).flatten().copied() else { return 0.0 };
        let car = |s: &mut Self| s.expr(n.car);
        let cdr = |s: &mut Self| s.expr(n.cdr);
        let b = |x: bool| if x { 1.0 } else { 0.0 };
        match n.kind {
            0 => (car(self) * DEG).sin(),
            1 => (car(self) * DEG).cos(),
            2 => (car(self) * DEG).tan(),
            3 => {
                let x = car(self);
                if x > 1.0 { 90.0 } else if x < -1.0 { -90.0 } else { x.asin() * RAD }
            }
            4 => {
                let x = car(self);
                if x > 1.0 { 0.0 } else if x < -1.0 { 180.0 } else { x.acos() * RAD }
            }
            5 => car(self).atan() * RAD,
            6 => car(self).ln(),
            7 => car(self).exp(),
            8 => car(self).abs(),
            9 => {
                let a = car(self);
                a.powf(cdr(self))
            }
            10 => car(self).abs().sqrt(),
            11 => {
                let a = car(self);
                a.min(cdr(self))
            }
            12 => {
                let a = car(self);
                a.max(cdr(self))
            }
            13 => {
                // Average of a LIST chain.
                let (mut sum, mut count) = (0.0, 0);
                let mut li = n.car;
                while let Some(l) = (li >= 0).then(|| nodes.get(li as usize)).flatten().copied() {
                    sum += self.expr(l.car);
                    count += 1;
                    li = l.cdr;
                }
                if count > 0 { sum / count as f32 } else { 0.0 }
            }
            14 => {
                // COND: car -> (cond, (then, (else)))
                let get = |k: i16| (k >= 0).then(|| nodes.get(k as usize)).flatten().copied();
                let Some(l1) = get(n.car) else { return 0.0 };
                let pick = if self.expr(l1.car) != 0.0 { n.car } else { l1.cdr };
                let Some(lp) = get(pick) else { return 0.0 };
                let Some(lv) = get(lp.cdr) else { return 0.0 };
                self.expr(lv.car)
            }
            15 | 43 => {
                let frame = car(self) * 60.0 / self.motion.fps.max(1e-3);
                let ci = n.cdr.max(0) as usize;
                let fwd = &self.motion.forward;
                let curve = if ci < fwd.len() { fwd.get(ci) } else { self.motion.inverse.get(ci - fwd.len()) };
                let v = curve.map_or(0.0, |c| c.eval(frame));
                if n.kind == 43 { v * RAD } else { v }
            }
            16 => (self.wpos(n.car.max(0) as usize) - self.wpos(n.cdr.max(0) as usize)).length(),
            17 => {
                let a = car(self);
                a % cdr(self)
            }
            18 => {
                let r = cdr(self);
                car(self) + r
            }
            19 => {
                let a = car(self);
                a - cdr(self)
            }
            20 => {
                let r = cdr(self);
                car(self) * r
            }
            21 => {
                let a = car(self);
                a / cdr(self)
            }
            22 => {
                let r = cdr(self) as i32;
                let a = car(self) as i32;
                if r == 0 { 0.0 } else { (a % r) as f32 }
            }
            23 => {
                let a = car(self);
                b(a == cdr(self))
            }
            24 => {
                let a = car(self);
                b(a > cdr(self))
            }
            25 => {
                let a = car(self);
                b(a >= cdr(self))
            }
            26 => {
                let a = car(self);
                b(a < cdr(self))
            }
            27 => {
                let a = car(self);
                b(a <= cdr(self))
            }
            28 => b(car(self) != 0.0 && cdr(self) != 0.0),
            29 => b(car(self) != 0.0 || cdr(self) != 0.0),
            30 => self.lfr,
            31 => n.value,
            32..=34 => {
                let j = n.element.max(0) as usize;
                let axis = (n.kind - 32) as usize;
                let m = if n.global { self.world.get(j) } else { self.local.get(j) };
                m.map_or(0.0, |m| m.w_axis[axis])
            }
            35..=37 => {
                let j = n.element.max(0) as usize;
                let axis = (n.kind - 35) as usize;
                if n.global {
                    self.unscaled.get(j).map_or(0.0, |m| euler_from_matrix(m)[axis] * RAD)
                } else {
                    self.srt.get(j).map_or(0.0, |s| s.rotation[axis] * RAD)
                }
            }
            38..=40 => {
                let j = n.element.max(0) as usize;
                let axis = (n.kind - 38) as usize;
                if n.global {
                    self.wscale.get(j).map_or(0.0, |s| s[axis])
                } else {
                    self.srt.get(j).map_or(0.0, |s| s.scale[axis])
                }
            }
            _ => 0.0,
        }
    }
}

/// Whether a constraint is on at key time `t` (its activation table, if any,
/// overrides the default flag).
fn constraint_active(m: &Interpolated, c: &Constraint, t: f32) -> bool {
    let n = c.activation_count.max(0) as usize;
    if n == 0 {
        return c.temporary_active != 0;
    }
    let start = c.activation_start.max(0) as usize;
    let Some(acts) = m.activations.get(start..start + n) else { return c.temporary_active != 0 };
    if t < acts[0].time {
        return acts[0].active == 0;
    }
    if t >= acts[n - 1].time {
        return acts[n - 1].active != 0;
    }
    let i = acts.partition_point(|a| a.time <= t).saturating_sub(1);
    acts[i].active != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_bone_reaches_target() {
        // Thigh at origin, 40 + 40 long, target 60 away along -y.
        let (up, low) = ik_solve(Vec3::ZERO, Vec3::new(0.0, -60.0, 0.0), 40.0, Some(40.0), false, 0.0, 0.0, None);
        let low = low.unwrap();
        let mut l = low;
        l.w_axis = Vec4::new(40.0, 0.0, 0.0, 1.0);
        let knee = up * l;
        let end = knee.transform_point3(Vec3::new(40.0, 0.0, 0.0));
        assert!((end - Vec3::new(0.0, -60.0, 0.0)).length() < 1e-3, "{end}");
    }

    #[test]
    fn euler_round_trip() {
        let r = Vec3::new(0.3, -0.7, 1.1);
        let m = Mat4::from_quat(crate::euler_quat(r));
        assert!((euler_from_matrix(&m) - r).length() < 1e-4);
    }
}
