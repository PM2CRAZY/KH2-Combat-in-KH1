//! Skeletons (MDLX bones) and full interpolated motions (ANB motion data):
//! everything needed to pose a character, but no meshes or textures.
//!
//! Layouts follow OpenKH's docs (`docs/kh2/file/type/mdlx.md`,
//! `docs/kh2/file/anb/motion.md`, Apache-2.0), which credit Kenjiuno's
//! msetDoc research. The reading code here is our own.

use crate::bar::Bar;
use crate::reader::Reader;
use crate::{bad, Result};

/// BAR entry type of a model inside an MDLX.
pub const BAR_TYPE_MODEL: u16 = 4;
const RESERVED: usize = 0x90;

/// One joint of a skeleton or IK-helper chain. Rotations are Euler angles
/// in radians, applied X, then Y, then Z; then the translation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bone {
    pub index: i16,
    pub sibling: i16,
    pub parent: i16,
    pub child: i16,
    pub flags: i32,
    pub scale: [f32; 3],
    /// The scale vector's W. The IK solver uses it as the bone's length.
    pub length: f32,
    pub rotation: [f32; 3],
    pub translation: [f32; 3],
}

impl Bone {
    pub const SIZE: usize = 0x40;

    fn read(buf: &[u8], at: usize) -> Result<Bone> {
        let mut r = Reader::at(buf, at);
        let index = r.i16()?;
        let sibling = r.i16()?;
        let parent = r.i16()?;
        let child = r.i16()?;
        let _reserved = r.i32()?;
        let flags = r.i32()?;
        let mut v = [[0f32; 4]; 3];
        for row in &mut v {
            for x in row.iter_mut() {
                *x = r.f32()?;
            }
        }
        let three = |a: [f32; 4]| [a[0], a[1], a[2]];
        Ok(Bone { index, sibling, parent, child, flags, scale: three(v[0]), length: v[0][3], rotation: three(v[1]), translation: three(v[2]) })
    }
}

/// Reads the bone list of the first skeletal model in an MDLX.
pub fn parse_skeleton(mdlx: &[u8]) -> Result<Vec<Bone>> {
    let bar = Bar::parse(mdlx)?;
    let Some(model) = bar.entries.iter().find(|e| e.kind == BAR_TYPE_MODEL) else {
        return bad("MDLX has no model entry");
    };
    let d = model.data;
    let mut h = Reader::at(d, RESERVED + 0x10);
    let count = h.u16()? as usize;
    let _textures = h.u16()?;
    let offset = h.u32()? as usize;
    (0..count).map(|i| Bone::read(d, RESERVED + offset + i * Bone::SIZE)).collect()
}

/// Behaviour of a curve before its first / after its last key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Cycle {
    /// Hold the first / last key.
    #[default]
    Hold,
    /// Continue in a straight line along the end key's tangent.
    Extrapolate,
    Repeat,
    Zero,
}

impl Cycle {
    fn from_bits(b: u8) -> Cycle {
        match b & 3 {
            1 => Cycle::Extrapolate,
            2 => Cycle::Repeat,
            3 => Cycle::Zero,
            _ => Cycle::Hold,
        }
    }
}

/// A key: time (60 Hz frames from the motion's key origin), value, ease-in and ease-out tangents, and
/// interpolation to the next key (0 constant, 1 linear, 2 Hermite).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    pub time: f32,
    pub value: f32,
    pub tangent_in: f32,
    pub tangent_out: f32,
    pub interp: u8,
}

/// One animated channel of one joint.
#[derive(Clone, Debug, PartialEq)]
pub struct FCurve {
    /// Bone index (forward curves) or IK-helper index (inverse curves).
    pub joint: i16,
    /// 0-2 scale, 3-5 rotation, 6-8 translation (x, y, z).
    pub channel: u8,
    pub pre: Cycle,
    pub post: Cycle,
    pub keys: Vec<Key>,
}

impl FCurve {
    /// Rotation channels skip interpolation across a near-2π jump between
    /// keys at most two frames apart (the game's wrap-around guard).
    // 3.14 is the game's own threshold, not an approximation of pi.
    #[allow(clippy::approx_constant)]
    fn segment(&self, t: f32) -> f32 {
        let k = &self.keys;
        if t <= k[0].time {
            return k[0].value;
        }
        let last = k[k.len() - 1];
        if t >= last.time {
            return last.value;
        }
        let i = k.partition_point(|key| key.time <= t) - 1;
        let (a, b) = (k[i], k[i + 1]);
        if t == a.time {
            return a.value;
        }
        let dt = b.time - a.time;
        if a.interp != 0 && (3..=5).contains(&self.channel) && dt <= 2.0 && (b.value - a.value).abs() >= 3.14 {
            return a.value;
        }
        match a.interp {
            1 => a.value + (b.value - a.value) * (t - a.time) / dt,
            2 => {
                let n = (t - a.time) / dt;
                let (n2, n3) = (n * n, n * n * n);
                (2.0 * n3 - 3.0 * n2 + 1.0) * a.value
                    + (-2.0 * n3 + 3.0 * n2) * b.value
                    + (n3 - 2.0 * n2 + n) * a.tangent_out * dt
                    + (n3 - n2) * b.tangent_in * dt
            }
            _ => a.value,
        }
    }

    /// Value at `t` (60 Hz frames on the motion's key timeline), with the
    /// pre/post behaviour.
    pub fn eval(&self, t: f32) -> f32 {
        let k = &self.keys;
        match k.len() {
            0 => return 0.0,
            1 => return k[0].value,
            _ => {}
        }
        let (first, last) = (k[0], k[k.len() - 1]);
        let span = last.time - first.time;
        let mut t = t;
        if t < first.time {
            match self.pre {
                Cycle::Hold => return first.value,
                Cycle::Extrapolate => return first.value - (first.time - t) * first.tangent_in,
                Cycle::Zero => return 0.0,
                Cycle::Repeat if span > 0.0 => {
                    while t < first.time {
                        t += span;
                    }
                }
                Cycle::Repeat => return first.value,
            }
        } else if t > last.time {
            match self.post {
                Cycle::Hold => return last.value,
                Cycle::Extrapolate => return last.value + (t - last.time) * last.tangent_out,
                Cycle::Zero => return 0.0,
                Cycle::Repeat if span > 0.0 => {
                    while t > last.time {
                        t -= span;
                    }
                }
                Cycle::Repeat => return last.value,
            }
        }
        self.segment(t)
    }
}

/// Links one joint to another (follow position, aim at, ...). Joint indices
/// count model bones first, then IK helpers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraint {
    pub kind: u8,
    pub temporary_active: u8,
    pub constrained: i16,
    pub source: i16,
    pub limiter: i16,
    pub activation_count: i16,
    pub activation_start: i16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Activation {
    pub time: f32,
    pub active: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limiter {
    pub flags: u32,
    pub damping_width: f32,
    pub damping_strength: f32,
    pub min: [f32; 4],
    pub max: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Expression {
    pub joint: i16,
    pub channel: i16,
    pub node: i16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExprNode {
    pub kind: i8,
    pub global: bool,
    pub element: i16,
    pub value: f32,
    pub car: i16,
    pub cdr: i16,
}

/// Evaluation order entry: a joint and its flags.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointEntry {
    pub joint: i16,
    pub flags: u8,
}

/// An interpolated (keyframed) motion, fully read.
#[derive(Clone, Debug, PartialEq)]
pub struct Interpolated {
    pub bone_count: usize,
    pub total_joints: usize,
    pub frame_count: i32,
    /// Frame range and loop point in the motion's own rate (`fps`). Key
    /// times are in 60 Hz frames: multiply these by `60 / fps`.
    pub frame_start: f32,
    pub frame_end: f32,
    pub fps: f32,
    pub frame_return: f32,
    /// (joint, channel, value) applied before the curves.
    pub initial_pose: Vec<(i16, i16, f32)>,
    pub forward: Vec<FCurve>,
    pub inverse: Vec<FCurve>,
    pub ik_helpers: Vec<Bone>,
    pub joints: Vec<JointEntry>,
    pub constraints: Vec<Constraint>,
    pub activations: Vec<Activation>,
    pub limiters: Vec<Limiter>,
    pub expressions: Vec<Expression>,
    pub nodes: Vec<ExprNode>,
    /// Joints that end IK chains flagged "external effector", in order.
    pub external_effectors: Vec<i16>,
}

impl Interpolated {
    /// Parses a motion (BAR type 9 entry). Returns `None` for raw motions.
    pub fn parse(buf: &[u8]) -> Result<Option<Interpolated>> {
        let mut r = Reader::at(buf, RESERVED);
        if r.i32()? != 0 {
            return Ok(None);
        }
        let h = RESERVED + 0x10;
        let at = |o: usize| Reader::at(buf, h + o);
        let i32_at = |o: usize| at(o).i32();
        let mut c = at(0);
        let bone_count = c.i16()?.max(0) as usize;
        let total_joints = c.i16()?.max(0) as usize;
        let frame_count = i32_at(0x04)?;
        let ik_off = i32_at(0x08)? as usize;
        let joints_off = i32_at(0x0C)? as usize;
        let pose_off = i32_at(0x14)? as usize;
        let pose_count = i32_at(0x18)?.max(0) as usize;
        let fwd_off = i32_at(0x20)? as usize;
        let fwd_count = i32_at(0x24)?.max(0) as usize;
        let inv_off = i32_at(0x28)? as usize;
        let inv_count = i32_at(0x2C)?.max(0) as usize;
        let keys_off = i32_at(0x30)? as usize;
        let times_off = i32_at(0x34)? as usize;
        let values_off = i32_at(0x38)? as usize;
        let tangents_off = i32_at(0x3C)? as usize;
        let cons_off = i32_at(0x40)? as usize;
        let cons_count = i32_at(0x44)?.max(0) as usize;
        let act_off = i32_at(0x48)? as usize;
        let lim_off = i32_at(0x4C)? as usize;
        let expr_off = i32_at(0x50)? as usize;
        let expr_count = i32_at(0x54)?.max(0) as usize;
        let node_off = i32_at(0x58)? as usize;
        let node_count = i32_at(0x5C)?.max(0) as usize;
        let mut t = at(0x80);
        let frame_start = t.f32()?;
        let frame_end = t.f32()?;
        let fps = t.f32()?;
        let frame_return = t.f32()?;
        let ext_off = i32_at(0x90)? as usize;

        let base = |o: usize| RESERVED + o;
        let f32_at = |off: usize, i: usize| Reader::at(buf, base(off) + i * 4).f32();

        let initial_pose = (0..pose_count)
            .map(|i| {
                let mut p = Reader::at(buf, base(pose_off) + i * 8);
                Ok((p.i16()?, p.i16()?, p.f32()?))
            })
            .collect::<Result<Vec<_>>>()?;

        let curves = |off: usize, n: usize| -> Result<Vec<FCurve>> {
            (0..n)
                .map(|i| {
                    let mut c = Reader::at(buf, base(off) + i * 6);
                    let joint = c.i16()?;
                    let bits = c.u8()?;
                    let n = c.u8()? as usize;
                    let start = c.i16()?.max(0) as usize;
                    let keys = (0..n)
                        .map(|k| {
                            let mut kr = Reader::at(buf, base(keys_off) + (start + k) * 8);
                            let tt = kr.u16()?;
                            let vi = kr.i16()?.max(0) as usize;
                            let li = kr.i16()?.max(0) as usize;
                            let ri = kr.i16()?.max(0) as usize;
                            Ok(Key {
                                time: f32_at(times_off, (tt >> 2) as usize)?,
                                value: f32_at(values_off, vi)?,
                                tangent_in: f32_at(tangents_off, li)?,
                                tangent_out: f32_at(tangents_off, ri)?,
                                interp: (tt & 3) as u8,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    Ok(FCurve {
                        joint,
                        channel: bits & 0xF,
                        pre: Cycle::from_bits(bits >> 4),
                        post: Cycle::from_bits(bits >> 6),
                        keys,
                    })
                })
                .collect()
        };
        let forward = curves(fwd_off, fwd_count)?;
        let inverse = curves(inv_off, inv_count)?;

        let helper_count = total_joints.saturating_sub(bone_count);
        let ik_helpers = (0..helper_count).map(|i| Bone::read(buf, base(ik_off) + i * Bone::SIZE)).collect::<Result<Vec<_>>>()?;
        let joints = (0..total_joints)
            .map(|i| {
                let mut j = Reader::at(buf, base(joints_off) + i * 4);
                Ok(JointEntry { joint: j.i16()?, flags: j.u8()? })
            })
            .collect::<Result<Vec<_>>>()?;

        let constraints = (0..cons_count)
            .map(|i| {
                let mut c = Reader::at(buf, base(cons_off) + i * 12);
                Ok(Constraint {
                    kind: c.u8()?,
                    temporary_active: c.u8()?,
                    constrained: c.i16()?,
                    source: c.i16()?,
                    limiter: c.i16()?,
                    activation_count: c.i16()?,
                    activation_start: c.i16()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let act_count = constraints
            .iter()
            .map(|c| (c.activation_start.max(0) + c.activation_count.max(0)) as usize)
            .max()
            .unwrap_or(0);
        let activations = (0..act_count)
            .map(|i| {
                let mut a = Reader::at(buf, base(act_off) + i * 8);
                Ok(Activation { time: a.f32()?, active: a.i32()? })
            })
            .collect::<Result<Vec<_>>>()?;
        let lim_count = constraints.iter().map(|c| (c.limiter + 1).max(0) as usize).max().unwrap_or(0);
        let limiters = (0..lim_count)
            .map(|i| {
                let mut l = Reader::at(buf, base(lim_off) + i * 0x30);
                let flags = l.u32()?;
                let _pad = l.i32()?;
                let damping_width = l.f32()?;
                let damping_strength = l.f32()?;
                let mut min = [0f32; 4];
                let mut max = [0f32; 4];
                for x in &mut min {
                    *x = l.f32()?;
                }
                for x in &mut max {
                    *x = l.f32()?;
                }
                Ok(Limiter { flags, damping_width, damping_strength, min, max })
            })
            .collect::<Result<Vec<_>>>()?;
        let expressions = (0..expr_count)
            .map(|i| {
                let mut e = Reader::at(buf, base(expr_off) + i * 8);
                let joint = e.i16()?;
                let channel = e.i16()?;
                let _r = e.i16()?;
                Ok(Expression { joint, channel, node: e.i16()? })
            })
            .collect::<Result<Vec<_>>>()?;
        let nodes = (0..node_count)
            .map(|i| {
                let mut n = Reader::at(buf, base(node_off) + i * 12);
                let kind = n.i8()?;
                let global = n.u8()? & 1 != 0;
                let element = n.i16()?;
                let value = n.f32()?;
                Ok(ExprNode { kind, global, element, value, car: n.i16()?, cdr: n.i16()? })
            })
            .collect::<Result<Vec<_>>>()?;

        let ext_count = joints.iter().filter(|j| j.flags & 0x80 != 0).count();
        let external_effectors = (0..ext_count)
            .map(|i| Reader::at(buf, base(ext_off) + i * 2).i16())
            .collect::<Result<Vec<_>>>()?;

        Ok(Some(Interpolated {
            bone_count,
            total_joints,
            frame_count,
            frame_start,
            frame_end,
            fps,
            frame_return,
            initial_pose,
            forward,
            inverse,
            ik_helpers,
            joints,
            constraints,
            activations,
            limiters,
            expressions,
            nodes,
            external_effectors,
        }))
    }

    /// Parses the motion inside an ANB (a BAR holding a type-9 entry).
    pub fn from_anb(anb: &[u8]) -> Result<Option<Interpolated>> {
        let bar = Bar::parse(anb)?;
        match bar.entries.iter().find(|e| e.kind == crate::motion::BAR_TYPE_MOTION && !e.data.is_empty()) {
            Some(e) => Interpolated::parse(e.data),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles() {
        let k = |time, value| Key { time, value, tangent_in: 0.0, tangent_out: 0.0, interp: 1 };
        let mut c = FCurve { joint: 0, channel: 3, pre: Cycle::Hold, post: Cycle::Repeat, keys: vec![k(0.0, 0.0), k(10.0, 1.0)] };
        assert_eq!(c.eval(5.0), 0.5);
        assert_eq!(c.eval(15.0), 0.5);
        c.post = Cycle::Extrapolate;
        c.keys[1].tangent_out = 0.2;
        assert_eq!(c.eval(15.0), 2.0);
        c.post = Cycle::Hold;
        assert_eq!(c.eval(15.0), 1.0);
    }
}
