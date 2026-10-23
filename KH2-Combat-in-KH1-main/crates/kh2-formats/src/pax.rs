//! PAX effect packages (the effects inside `.mag`, `.mdlx` and other BARs).
//!
//! Layouts per OpenKH `OpenKh.Kh2/Pax.cs`, `Dpx.cs`, `Dpd.cs`,
//! `Dpd.ParticleData.cs` and `DpdPData.cs` (Apache-2.0). Only what the
//! game logic needs is read: each element (which effect, where it is bound
//! and how) and each effect's emitter timing (how long it lives, whether
//! it loops). Programs, textures, shapes and models are read by [`crate::dpd`].
//!
//! ```text
//! PAX:  "PAX_" | debug info offset | element count | DPX offset
//!       element x 80 bytes
//! DPX:  version | 0 | 0 | effect count
//!       effect x 32: DPD offset | particle data index | effect number | ...
//!       DPD count | DPD offsets
//! DPD:  0x96 | particle data list | textures | shapes | models | vsf
//! particle data: 272-byte header, then a list of emitters whose headers
//!       hold start frame, total frames and a loop range (x 4096)
//! ```

use crate::reader::Reader;
use crate::{bad, Result};

/// Element flags (OpenKH names).
pub mod flags {
    pub const BIND: u32 = 0x01;
    pub const BIND_ONLY_START: u32 = 0x02;
    pub const BIND_ONLY_POS: u32 = 0x04;
    pub const DESTROY_WHEN_MOTION_CHANGE: u32 = 0x20;
    pub const DESTROY_FADEOUT: u32 = 0x40;
    pub const DESTROY_LOOPEND: u32 = 0x80;
    pub const DESTROY_WHEN_OBJECT_LEAVES: u32 = 0x200;
}

/// One element: an effect placed relative to an object's bone.
#[derive(Clone, Debug, PartialEq)]
pub struct PaxElement {
    /// Which DPX effect plays.
    pub effect: u16,
    /// What scripts start it by.
    pub id: u16,
    pub group: u8,
    pub fadeout_frames: u8,
    /// Bone index, or a special anchor (0x4000 object root, 0x4001 body
    /// centre, 0x4003/0x4004 weapon tips, ...).
    pub bone: u16,
    pub flags: u32,
    pub start_wait: f32,
    pub sound: i32,
    /// In the bone's space, game axes (y points down).
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}

/// One emitter's timing, in effect ticks. The effect system advances one
/// tick per two 60 Hz game frames (see `docs/kh2-effects.md`), so a value
/// of 27 lasts 54 game frames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Emitter {
    /// `None`: starts when the effect's loop is ended.
    pub start: Option<f32>,
    /// `None`: runs until the loop is ended.
    pub total: Option<f32>,
    /// Plays `[a, b)` over and over until the loop is ended.
    pub looping: Option<(f32, f32)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaxEffect {
    pub number: u32,
    pub emitters: Vec<Emitter>,
}

impl PaxEffect {
    /// Whether the effect plays until something ends it.
    pub fn loops(&self) -> bool {
        self.emitters.iter().any(|e| e.looping.is_some() || (e.total.is_none() && e.start.is_some()))
    }

    /// Frames after start (or after the loop is ended, for looping
    /// effects) until every emitter is done.
    pub fn length(&self, ended_at: Option<f32>) -> f32 {
        let mut end: f32 = 0.0;
        for e in &self.emitters {
            let start = match (e.start, ended_at) {
                (Some(s), _) => s,
                (None, Some(t)) => t,
                (None, None) => continue,
            };
            let stop = match (e.total, e.looping, ended_at) {
                (Some(t), None, _) => start + t,
                // A loop plays to its end once released.
                (Some(t), Some((_, b)), Some(at)) => at.max(start) + (t - b).max(0.0),
                (None, _, Some(at)) => at.max(start),
                _ => f32::INFINITY,
            };
            end = end.max(stop);
        }
        end
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pax {
    pub elements: Vec<PaxElement>,
    pub effects: Vec<PaxEffect>,
}

const FIXED: f32 = 4096.0;
const NEVER: i32 = 0x7000_0000;
const AT_END: i32 = 0x7fff_ffff;

impl Pax {
    pub fn element(&self, id: u16) -> Option<&PaxElement> {
        self.elements.iter().find(|e| e.id == id)
    }

    pub fn effect(&self, number: u16) -> Option<&PaxEffect> {
        self.effects.iter().find(|e| e.number == number as u32)
    }

    pub fn parse(buf: &[u8]) -> Result<Pax> {
        let mut r = Reader::new(buf);
        if r.u32()? != 0x5f58_4150 {
            return bad("not a PAX file");
        }
        let _debug = r.i32()?;
        let n = r.i32()?;
        let dpx = r.i32()?;
        if !(0..=4096).contains(&n) {
            return bad("PAX: implausible element count");
        }
        let mut elements = Vec::with_capacity(n as usize);
        for i in 0..n as usize {
            let mut e = Reader::at(buf, 16 + i * 80);
            let effect = e.u16()?;
            let id = e.u16()?;
            let group = e.u8()?;
            let fadeout_frames = e.u8()?;
            let bone = e.u16()?;
            let _category = e.u32()? as u64 | (e.u32()? as u64) << 32;
            let flags = e.u32()?;
            let start_wait = e.f32()?;
            let sound = e.i32()?;
            let mut v3 = || -> Result<[f32; 3]> { Ok([e.f32()?, e.f32()?, e.f32()?]) };
            let translation = v3()?;
            let rotation = v3()?;
            let scale = v3()?;
            elements.push(PaxElement { effect, id, group, fadeout_frames, bone, flags, start_wait, sound, translation, rotation, scale });
        }
        let effects = if dpx > 0 && (dpx as usize) < buf.len() { parse_dpx(&buf[dpx as usize..])? } else { Vec::new() };
        Ok(Pax { elements, effects })
    }
}

fn parse_dpx(dpx: &[u8]) -> Result<Vec<PaxEffect>> {
    let mut r = Reader::new(dpx);
    r.skip(12)?;
    let n = r.i32()?;
    if !(0..=4096).contains(&n) {
        return bad("DPX: implausible effect count");
    }
    let mut out = Vec::new();
    for i in 0..n as usize {
        let mut e = Reader::at(dpx, 16 + i * 32);
        let dpd = e.u32()? as usize;
        let data_index = e.u32()? as usize;
        let number = e.u32()?;
        let emitters = dpd_emitters(dpx.get(dpd..).unwrap_or(&[]), data_index).unwrap_or_default();
        out.push(PaxEffect { number, emitters });
    }
    Ok(out)
}

fn dpd_emitters(dpd: &[u8], index: usize) -> Result<Vec<Emitter>> {
    let mut r = Reader::new(dpd);
    if r.u32()? != 0x96 {
        return bad("not a DPD");
    }
    let count = r.i32()? as usize;
    if index >= count {
        return bad("DPD: particle data index out of range");
    }
    let mut o = Reader::at(dpd, 8 + index * 4);
    let pd = o.u32()? as usize;
    // 272-byte particle header, then the emitter file.
    let file = dpd.get(pd + 272..).ok_or_else(|| crate::Error::Format("DPD: particle data out of range".into()))?;
    let mut h = Reader::at(file, 12);
    let list = h.u32()? as usize;
    let mut l = Reader::at(file, list);
    let n = l.i32()?;
    if !(0..=1024).contains(&n) {
        return bad("DPD: implausible emitter count");
    }
    let mut out = Vec::new();
    for _ in 0..n {
        let at = l.u32()? as usize;
        let mut e = Reader::at(file, at + 16);
        let (start, total, l0, l1) = (e.i32()?, e.i32()?, e.i32()?, e.i32()?);
        out.push(Emitter {
            start: (start != AT_END).then_some(start as f32 / FIXED),
            total: (total != NEVER).then_some(total as f32 / FIXED),
            looping: (l0 != NEVER && l1 != NEVER && l0 >= 0 && l1 > l0).then_some((l0 as f32 / FIXED, l1 as f32 / FIXED)),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths() {
        // A one-shot effect, a looping one and one with an end phase.
        let once = PaxEffect { number: 0, emitters: vec![Emitter { start: Some(0.0), total: Some(27.0), looping: None }, Emitter { start: Some(3.0), total: Some(10.0), looping: None }] };
        assert!(!once.loops());
        assert_eq!(once.length(None), 27.0);
        let lp = PaxEffect { number: 1, emitters: vec![Emitter { start: Some(0.0), total: Some(9.0), looping: Some((3.0, 4.0)) }] };
        assert!(lp.loops());
        assert_eq!(lp.length(None), f32::INFINITY);
        assert_eq!(lp.length(Some(20.0)), 25.0);
        let tail = PaxEffect { number: 2, emitters: vec![Emitter { start: Some(0.0), total: None, looping: None }, Emitter { start: None, total: Some(27.0), looping: None }] };
        assert!(tail.loops());
        assert_eq!(tail.length(Some(10.0)), 37.0);
    }
}
