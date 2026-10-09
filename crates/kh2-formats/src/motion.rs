//! Movesets (MSET), animation bundles (ANB), motion headers and motion
//! triggers: the data that says *when* during an animation a hitbox is live.
//!
//! Layouts follow OpenKH's docs (`docs/kh2/file/anb/{mset,anb,motion}.md`)
//! and `OpenKh.Kh2/MotionTrigger.cs` (Apache-2.0); the trigger tables are
//! documented there with credit to Kenjiuno's msetDoc research.
//!
//! - An MSET is a BAR of slots. Each slot is either a dummy (size 0) or an
//!   ANB, itself a BAR holding a motion (type 9) and motion triggers (type 16).
//! - For player movesets (BAR motionset field = 1), slot =
//!   `motion_id * 4 + relative` where relative is 0 battle+weapon,
//!   1 battle, 2 field, 3 field+weapon.

use crate::bar::Bar;
use crate::reader::Reader;
use crate::Result;

pub const BAR_TYPE_MOTION: u16 = 9;
pub const BAR_TYPE_MOTION_TRIGGERS: u16 = 16;
pub const BAR_TYPE_ANB: u16 = 17;

/// A trigger active over a frame range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RangeTrigger {
    pub start: i16,
    pub end: i16,
    pub id: u8,
    pub params: Vec<i16>,
}

/// A trigger that fires on one frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameTrigger {
    pub frame: i16,
    pub id: u8,
    pub params: Vec<i16>,
}

/// Range trigger ids with known combat meaning (OpenKH range trigger list).
pub mod range_id {
    pub const ENABLE_COLLISION: u8 = 4;
    pub const DISABLE_COLLISION: u8 = 5;
    /// params: atkp id, collision group
    pub const ATTACK_HITBOX: u8 = 10;
    pub const ALLOW_COMBO: u8 = 11;
    pub const WEAPON_TRAIL: u8 = 12;
    pub const ALLOW_RC: u8 = 14;
    pub const DISABLE_INVINCIBILITY: u8 = 19;
    pub const TURN_TO_TARGET: u8 = 22;
    /// "Hitbox off (can't be hit)": an invulnerability window.
    pub const HURTBOX_OFF: u8 = 27;
    pub const TURN_TO_LOCK: u8 = 28;
    /// params: atkp id, collision group, combo id
    pub const ATTACK_HITBOX_COMBO: u8 = 33;
    pub const ALLOW_COMBO_MAGIC: u8 = 35;
    pub const ALLOW_MOVEMENT: u8 = 41;
    pub const ALLOW_COMBO_FINISHER: u8 = 50;
}

/// Frame trigger ids with known combat meaning.
pub mod frame_id {
    pub const EFFECT_CASTER: u8 = 1;
    pub const SOUND: u8 = 8;
    pub const MAGIC_CAST: u8 = 17;
    pub const REVENGE_CHECK: u8 = 28;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MotionTriggers {
    pub ranges: Vec<RangeTrigger>,
    pub frames: Vec<FrameTrigger>,
}

impl MotionTriggers {
    pub fn parse(buf: &[u8]) -> Result<MotionTriggers> {
        let mut r = Reader::new(buf);
        let n_range = r.u8()? as usize;
        let n_frame = r.u8()? as usize;
        let frame_offset = r.i16()?;
        let mut ranges = Vec::with_capacity(n_range);
        for _ in 0..n_range {
            let start = r.i16()?;
            let end = r.i16()?;
            let id = r.u8()?;
            let n = r.u8()? as usize;
            let params = (0..n).map(|_| r.i16()).collect::<Result<_>>()?;
            ranges.push(RangeTrigger { start, end, id, params });
        }
        // Frame triggers start at the stored offset when it is valid;
        // otherwise directly after the range triggers.
        if frame_offset > 0 && (frame_offset as usize) <= buf.len() {
            r = Reader::at(buf, frame_offset as usize);
        }
        let mut frames = Vec::with_capacity(n_frame);
        for _ in 0..n_frame {
            let frame = r.i16()?;
            let id = r.u8()?;
            let n = r.u8()? as usize;
            let params = (0..n).map(|_| r.i16()).collect::<Result<_>>()?;
            frames.push(FrameTrigger { frame, id, params });
        }
        Ok(MotionTriggers { ranges, frames })
    }
}

/// One animation curve (F-curve) for a single channel.
#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    /// (time in local frames, value, ease-in tangent, ease-out tangent, interpolation)
    /// interpolation: 0 constant, 1 linear, 2 hermite.
    pub keys: Vec<(f32, f32, f32, f32, u8)>,
}

impl Curve {
    /// Evaluates at `t` local frames, clamping outside the key range.
    ///
    /// Hermite segments use the standard cubic Hermite basis with tangents
    /// scaled by the segment length (hypothesis: OpenKH's reference engine
    /// marks its own Hermite handling as uncertain).
    pub fn eval(&self, t: f32) -> f32 {
        let k = &self.keys;
        match k.len() {
            0 => 0.0,
            1 => k[0].1,
            _ => {
                if t <= k[0].0 {
                    return k[0].1;
                }
                let last = k[k.len() - 1];
                if t >= last.0 {
                    return last.1;
                }
                let i = k.partition_point(|key| key.0 <= t) - 1;
                let (a, b) = (k[i], k[i + 1]);
                let dt = (b.0 - a.0).max(f32::EPSILON);
                let n = (t - a.0) / dt;
                match a.4 {
                    0 => a.1,
                    1 => a.1 + (b.1 - a.1) * n,
                    _ => {
                        let (n2, n3) = (n * n, n * n * n);
                        (2.0 * n3 - 3.0 * n2 + 1.0) * a.1
                            + (n3 - 2.0 * n2 + n) * a.3 * dt
                            + (-2.0 * n3 + 3.0 * n2) * b.1
                            + (n3 - n2) * b.2 * dt
                    }
                }
            }
        }
    }
}

/// A channel that is either constant or animated.
#[derive(Clone, Debug, PartialEq)]
pub enum Channel {
    Const(f32),
    Animated(Curve),
}

impl Channel {
    pub fn eval(&self, t: f32) -> f32 {
        match self {
            Channel::Const(v) => *v,
            Channel::Animated(c) => c.eval(t),
        }
    }
}

/// The timing fields of a motion header, plus its root-motion curves.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionInfo {
    /// 0 = interpolated, 1 = raw.
    pub kind: i32,
    pub frame_start: f32,
    pub frame_end: f32,
    /// Local frame rate (LFR). Trigger frames are in 60 Hz global frames;
    /// see docs/movement-model.md for the evidence.
    pub fps: f32,
    pub frame_return: f32,
    /// Root translation x, y, z (root motion), in local frames.
    pub root_translation: Option<[Channel; 3]>,
}

impl MotionInfo {
    /// Root translation at a local frame time.
    pub fn root_at(&self, lfr: f32) -> [f32; 3] {
        match &self.root_translation {
            Some(c) => [c[0].eval(lfr), c[1].eval(lfr), c[2].eval(lfr)],
            None => [0.0; 3],
        }
    }

    /// Motion data: 0x90 reserved bytes, a 16-byte motion header at 0x90,
    /// then the interpolated header at 0xA0. Offsets inside are relative to
    /// 0x90. Raw motions are reported without timing.
    pub fn parse(buf: &[u8]) -> Result<MotionInfo> {
        const BASE: usize = 0x90;
        let mut r = Reader::at(buf, BASE);
        let kind = r.i32()?;
        if kind != 0 {
            return Ok(MotionInfo { kind, frame_start: 0.0, frame_end: 0.0, fps: 0.0, frame_return: 0.0, root_translation: None });
        }
        let h = BASE + 0x10;
        let i32_at = |o: usize| Reader::at(buf, h + o).i32();
        let mut t = Reader::at(buf, h + 0x80);
        let frame_start = t.f32()?;
        let frame_end = t.f32()?;
        let fps = t.f32()?;
        let frame_return = t.f32()?;

        let root_off = i32_at(0x1C)?;
        let fwd_off = i32_at(0x20)? as usize;
        let fwd_count = i32_at(0x24)?.max(0) as usize;
        let inv_off = i32_at(0x28)? as usize;
        let inv_count = i32_at(0x2C)?.max(0) as usize;
        let keys_off = i32_at(0x30)? as usize;
        let times_off = i32_at(0x34)? as usize;
        let values_off = i32_at(0x38)? as usize;
        let tangents_off = i32_at(0x3C)? as usize;

        let f32_at = |off: usize, i: usize| Reader::at(buf, BASE + off + i * 4).f32();
        let curve = |id: i32| -> Result<Option<Curve>> {
            if id < 0 {
                return Ok(None);
            }
            let id = id as usize;
            let fc = if id < fwd_count {
                BASE + fwd_off + id * 6
            } else if id - fwd_count < inv_count {
                BASE + inv_off + (id - fwd_count) * 6
            } else {
                return Ok(None);
            };
            let mut c = Reader::at(buf, fc);
            let _joint = c.i16()?;
            let _channel = c.u8()?;
            let n = c.u8()? as usize;
            let start = c.i16()? as usize;
            let mut keys = Vec::with_capacity(n);
            for k in 0..n {
                let mut kr = Reader::at(buf, BASE + keys_off + (start + k) * 8);
                let tt = kr.u16()?;
                let vi = kr.i16()?.max(0) as usize;
                let li = kr.i16()?.max(0) as usize;
                let ri = kr.i16()?.max(0) as usize;
                let interp = (tt & 3) as u8;
                let ti = (tt >> 2) as usize;
                keys.push((f32_at(times_off, ti)?, f32_at(values_off, vi)?, f32_at(tangents_off, li)?, f32_at(tangents_off, ri)?, interp));
            }
            Ok(Some(Curve { keys }))
        };

        let root_translation = if root_off > 0 {
            let mut rr = Reader::at(buf, BASE + root_off as usize);
            rr.skip(0x20)?; // scale, rotation
            let base = [rr.f32()?, rr.f32()?, rr.f32()?];
            rr.skip(4)?;
            let ids = (0..9).map(|_| rr.i32()).collect::<Result<Vec<_>>>()?;
            let ch = |axis: usize| -> Result<Channel> {
                Ok(match curve(ids[6 + axis])? {
                    Some(c) => Channel::Animated(c),
                    None => Channel::Const(base[axis]),
                })
            };
            Some([ch(0)?, ch(1)?, ch(2)?])
        } else {
            None
        };

        Ok(MotionInfo { kind, frame_start, frame_end, fps, frame_return, root_translation })
    }
}

/// One animation of a moveset.
#[derive(Clone, Debug, PartialEq)]
pub struct Anb {
    pub motion: Option<MotionInfo>,
    pub triggers: MotionTriggers,
    /// The full keyframed motion (for posing a skeleton). `None` for raw
    /// motions or ANBs without motion data.
    pub pose: Option<crate::anim::Interpolated>,
}

impl Anb {
    pub fn parse(buf: &[u8]) -> Result<Anb> {
        let bar = Bar::parse(buf)?;
        let mut anb = Anb { motion: None, triggers: MotionTriggers::default(), pose: None };
        for e in &bar.entries {
            match e.kind {
                BAR_TYPE_MOTION if anb.motion.is_none() && !e.data.is_empty() => {
                    anb.motion = MotionInfo::parse(e.data).ok();
                    anb.pose = crate::anim::Interpolated::parse(e.data).ok().flatten();
                }
                BAR_TYPE_MOTION_TRIGGERS if !e.data.is_empty() => anb.triggers = MotionTriggers::parse(e.data)?,
                _ => {}
            }
        }
        Ok(anb)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MsetSlot {
    pub index: usize,
    /// BAR tag, e.g. "A320". Not used by the game for lookup.
    pub name: String,
    /// `None` for dummy slots.
    pub anb: Option<Anb>,
    /// A "capture" slot (BAR type 20): motions another object plays when
    /// this one captures it, by tag ("0010" Sora's body, "0011"/"0012" his
    /// weapons; the first non-empty entry of each tag).
    pub nested: Vec<(String, Anb)>,
}

/// BAR entry type of a nested motion set (capture slot).
pub const BAR_TYPE_NESTED_MSET: u16 = 20;

impl MsetSlot {
    /// The capture motion for `tag`, if this is a capture slot.
    pub fn capture(&self, tag: &str) -> Option<&Anb> {
        self.nested.iter().find(|(t, _)| t == tag).map(|(_, a)| a)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mset {
    /// 0 default, 1 player, 2 raw.
    pub kind: i32,
    pub slots: Vec<MsetSlot>,
}

impl Mset {
    pub fn parse(buf: &[u8]) -> Result<Mset> {
        let bar = Bar::parse(buf)?;
        let mut slots = Vec::with_capacity(bar.entries.len());
        for (index, e) in bar.entries.iter().enumerate() {
            let anb = if e.kind == BAR_TYPE_ANB && !e.data.is_empty() { Some(Anb::parse(e.data)?) } else { None };
            let mut nested: Vec<(String, Anb)> = Vec::new();
            if e.kind == BAR_TYPE_NESTED_MSET && !e.data.is_empty() {
                if let Ok(inner) = Bar::parse(e.data) {
                    for n in inner.entries.iter().filter(|n| n.kind == BAR_TYPE_ANB && !n.data.is_empty()) {
                        if !nested.iter().any(|(t, _)| *t == n.name) {
                            if let Ok(a) = Anb::parse(n.data) {
                                nested.push((n.name.clone(), a));
                            }
                        }
                    }
                }
            }
            slots.push(MsetSlot { index, name: e.name.clone(), anb, nested });
        }
        Ok(Mset { kind: bar.motionset, slots })
    }

    /// Resolves a motion id the way the game does for player movesets,
    /// including the documented dummy fallback order.
    pub fn slot_for(&self, motion_id: usize, battle: bool, weapon: bool) -> Option<&MsetSlot> {
        if self.kind != 1 {
            return self.slots.get(motion_id).filter(|s| s.anb.is_some());
        }
        let rel = match (battle, weapon) {
            (true, true) => 0,
            (true, false) => 1,
            (false, false) => 2,
            (false, true) => 3,
        };
        let order: [usize; 4] = match rel {
            3 => [3, 2, 0, 1],
            2 => [2, 3, 1, 0],
            1 => [1, 0, 2, 3],
            _ => [0, 1, 3, 2],
        };
        order.iter().map(|r| motion_id * 4 + r).find_map(|i| self.slots.get(i).filter(|s| s.anb.is_some()))
    }
}

/// Collision shape attached to a model bone (MDLX BAR entry type 23).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectCollision {
    pub group: u8,
    pub parts: u8,
    pub argument: i16,
    /// 2 = hit (hurtbox), 6 = attack, 13 = guard, ... (OpenKH table).
    pub kind: u8,
    /// 0 ellipsoid, 1 column, 2 cube, 3 sphere.
    pub shape: u8,
    /// Model bone index; 16384 = none.
    pub bone: i16,
    pub position: [i16; 3],
    pub position_height: i16,
    pub radius: i16,
    pub height: i16,
}

pub const BAR_TYPE_MODEL_COLLISION: u16 = 23;

/// The collision table's default enabled-group mask (bit g = group g), or
/// group 0 only when there is no table.
pub fn model_collision_groups(mdlx: &[u8]) -> Result<u32> {
    let bar = Bar::parse(mdlx)?;
    let Some(e) = bar.entries.iter().find(|e| e.kind == BAR_TYPE_MODEL_COLLISION) else { return Ok(1) };
    let mut r = Reader::new(e.data);
    let _count = r.u32()?;
    r.u32()
}

/// Reads the collision list from an MDLX (a BAR).
pub fn parse_model_collision(mdlx: &[u8]) -> Result<Vec<ObjectCollision>> {
    let bar = Bar::parse(mdlx)?;
    let Some(e) = bar.entries.iter().find(|e| e.kind == BAR_TYPE_MODEL_COLLISION) else {
        return Ok(Vec::new());
    };
    let mut r = Reader::new(e.data);
    let count = r.u32()? as usize;
    r.skip(4 + 56)?;
    (0..count)
        .map(|_| {
            Ok(ObjectCollision {
                group: r.u8()?,
                parts: r.u8()?,
                argument: r.i16()?,
                kind: r.u8()?,
                shape: r.u8()?,
                bone: r.i16()?,
                position: [r.i16()?, r.i16()?, r.i16()?],
                position_height: r.i16()?,
                radius: r.i16()?,
                height: r.i16()?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_eval() {
        let c = Curve { keys: vec![(0.0, 0.0, 0.0, 0.0, 1), (10.0, 100.0, 0.0, 0.0, 2), (20.0, 100.0, 0.0, 0.0, 2)] };
        assert_eq!(c.eval(-5.0), 0.0);
        assert_eq!(c.eval(5.0), 50.0);
        assert_eq!(c.eval(15.0), 100.0); // flat hermite segment
        assert_eq!(c.eval(99.0), 100.0);
        // hermite with zero tangents is smoothstep
        let h = Curve { keys: vec![(0.0, 0.0, 0.0, 0.0, 2), (10.0, 10.0, 0.0, 0.0, 2)] };
        assert!((h.eval(5.0) - 5.0).abs() < 1e-5);
        assert!((h.eval(2.5) - 1.5625).abs() < 1e-4);
    }

    #[test]
    fn parses_triggers() {
        // 1 range trigger (frames 3..9, attack hitbox, atkp 7, group 2),
        // 1 frame trigger at offset 14 (frame 5, id 17, param 1).
        let mut b = vec![1u8, 1];
        b.extend_from_slice(&14i16.to_le_bytes());
        for v in [3i16, 9] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&[10, 2]);
        for v in [7i16, 2] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&5i16.to_le_bytes());
        b.extend_from_slice(&[17, 1]);
        b.extend_from_slice(&1i16.to_le_bytes());
        let t = MotionTriggers::parse(&b).unwrap();
        assert_eq!(t.ranges, vec![RangeTrigger { start: 3, end: 9, id: 10, params: vec![7, 2] }]);
        assert_eq!(t.frames, vec![FrameTrigger { frame: 5, id: 17, params: vec![1] }]);
    }
}
