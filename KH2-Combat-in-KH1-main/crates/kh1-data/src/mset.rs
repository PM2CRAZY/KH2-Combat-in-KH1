//! KH1 motion sets (`.mset`): the motion table and plain forward evaluation of a motion's
//! keyed channels, enough to pose a motion on its skeleton (for telling motions apart, and
//! for knowing which one the game is playing).
//!
//! Layout, from the user's `xa_ex_0010.mset` and the executable's skeleton pass
//! (`0x140391E20`, which reads the motion header fields below, and `0x14038F9D0`, which
//! evaluates one channel):
//! - the file: `count u32`, then section offsets; section 0 is `MMTN`, a motion table:
//!   `"MMTN"`, size, ..., and at `+0x40` motion offsets (from `+0x40`), ended by `0xFFFFFFFF`.
//! - a motion (offsets from its own start): `+0x04` frame count u32, `+0x08` frames per second
//!   f32, `+0x10` bone count, `+0x18` channel count, `+0x1C` channels, `+0x20` / `+0x24` a
//!   second channel list (IK helpers; not evaluated here), `+0x28` keys, `+0x2C` / `+0x30` the
//!   helpers.
//! - a channel (6 bytes): `bone u16`, `kind u8` (low nibble: 1-3 scale x/y/z, 4-6 rotation,
//!   7-9 translation; bits 4-5 / 6-7 before / after the keys: 2 repeats), `count u8`,
//!   `first key u16`.
//! - a key (16 bytes): `time << 8 | interpolation` as i32 (time in 1/256 frames; interpolation
//!   0 holds the value, 1 is linear, else cubic Hermite), value, out tangent, in tangent (per
//!   frame). Between keys `k` and `k+1`: Hermite with `k`'s value and its tangent at `+0xC`,
//!   `k+1`'s value and its tangent at `+0x8`, tangents scaled by the gap.

use kh2_formats::anim::Bone;
use kh2_formats::{Error, Result};

const MMTN: u32 = u32::from_le_bytes(*b"MMTN");

fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    b.get(at..at + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap())).ok_or_else(|| Error::Format(format!("mset: read past the end at {at:#x}")))
}

#[derive(Clone, Copy, Debug)]
pub struct Key {
    pub time: f32,
    pub interpolation: u8,
    pub value: f32,
    pub tangent_out: f32,
    pub tangent_in: f32,
}

#[derive(Clone, Debug)]
pub struct Channel {
    pub bone: u16,
    /// 1-3 scale, 4-6 rotation, 7-9 translation (x, y, z).
    pub kind: u8,
    pub before: u8,
    pub after: u8,
    pub keys: Vec<Key>,
}

#[derive(Clone, Debug)]
pub struct Motion {
    /// Offset of the motion in the motion table section (what the game's pointer is relative
    /// to, once the section is loaded).
    pub offset: usize,
    pub frames: u32,
    pub fps: f32,
    pub bones: u32,
    pub channels: Vec<Channel>,
    /// The IK helpers' channel count (`+0x20`) and helper count (`+0x2C`), not evaluated.
    pub helper_channels: u32,
    pub helpers: u32,
}

/// The motions of a `.mset`, in table order (`None` where a slot is empty or unreadable).
pub fn parse(mset: &[u8]) -> Result<Vec<Option<Motion>>> {
    let count = u32_at(mset, 0)? as usize;
    if count == 0 || count > 64 {
        return Err(Error::Format(format!("mset: {count} sections")));
    }
    let sec = u32_at(mset, 4)? as usize;
    if u32_at(mset, sec)? != MMTN {
        return Err(Error::Format("mset: section 0 is not MMTN".into()));
    }
    let table = sec + 0x40;
    let mut out = Vec::new();
    for i in 0..4096 {
        let off = u32_at(mset, table + i * 4)?;
        if off == 0xFFFF_FFFF {
            break;
        }
        out.push(motion(mset, table + off as usize).ok().map(|mut m| {
            m.offset = off as usize + 0x40;
            m
        }));
    }
    Ok(out)
}

fn motion(b: &[u8], at: usize) -> Result<Motion> {
    let frames = u32_at(b, at + 4)?;
    let fps = f32::from_bits(u32_at(b, at + 8)?);
    let bones = u32_at(b, at + 0x10)?;
    let n = u32_at(b, at + 0x18)? as usize;
    let chans = at + u32_at(b, at + 0x1C)? as usize;
    let keys = at + u32_at(b, at + 0x28)? as usize;
    if n > 4096 || !(1.0..=240.0).contains(&fps) {
        return Err(Error::Format("mset: not a motion".into()));
    }
    let mut channels = Vec::with_capacity(n);
    for c in 0..n {
        let e = chans + c * 6;
        let s = b.get(e..e + 6).ok_or_else(|| Error::Format("mset: channel past the end".into()))?;
        let bone = u16::from_le_bytes([s[0], s[1]]);
        let kind = s[2];
        let count = s[3] as usize;
        let first = u16::from_le_bytes([s[4], s[5]]) as usize;
        let mut ks = Vec::with_capacity(count);
        for k in first..first + count {
            let p = keys + k * 16;
            let head = u32_at(b, p)? as i32;
            ks.push(Key {
                time: (head >> 8) as f32 / 256.0,
                interpolation: (head & 0xFF) as u8,
                value: f32::from_bits(u32_at(b, p + 4)?),
                tangent_out: f32::from_bits(u32_at(b, p + 8)?),
                tangent_in: f32::from_bits(u32_at(b, p + 12)?),
            });
        }
        channels.push(Channel { bone, kind: kind & 0xF, before: (kind >> 4) & 3, after: kind >> 6, keys: ks });
    }
    Ok(Motion { offset: 0, frames, fps, bones, channels, helper_channels: u32_at(b, at + 0x20)?, helpers: u32_at(b, at + 0x2C)? })
}

impl Channel {
    /// The value at `frame` (repeating past the ends when the channel says so, else held).
    pub fn sample(&self, frame: f32) -> Option<f32> {
        let (first, last) = (self.keys.first()?, self.keys.last()?);
        if self.keys.len() == 1 {
            return Some(first.value);
        }
        let span = last.time - first.time;
        let mut t = frame;
        if t < first.time {
            if self.before == 2 && span > 0.0 {
                t = last.time - (first.time - t) % span;
            } else {
                return Some(first.value);
            }
        }
        if t >= last.time {
            if self.after == 2 && span > 0.0 {
                t = first.time + (t - first.time) % span;
            } else {
                return Some(last.value);
            }
        }
        let i = self.keys.partition_point(|k| k.time <= t).saturating_sub(1).min(self.keys.len() - 2);
        let (a, b) = (self.keys[i], self.keys[i + 1]);
        let gap = b.time - a.time;
        if gap <= 0.0 || a.interpolation == 0 {
            return Some(a.value);
        }
        let x = t - a.time;
        if a.interpolation == 1 {
            return Some(a.value + (b.value - a.value) * x / gap);
        }
        let s = x / gap;
        let (s2, s3) = (s * s, s * s * s);
        let h00 = 2.0 * s3 - 3.0 * s2 + 1.0;
        let h01 = 3.0 * s2 - 2.0 * s3;
        let h10 = s3 - 2.0 * s2 + s;
        let h11 = s3 - s2;
        Some(h00 * a.value + h01 * b.value + gap * (h10 * a.tangent_in + h11 * b.tangent_out))
    }
}

impl Motion {
    /// The skeleton with this motion's channels applied at `frame` (plain FK: the IK helper
    /// list is not evaluated).
    pub fn apply(&self, skeleton: &[Bone], frame: f32) -> Vec<Bone> {
        let mut s = skeleton.to_vec();
        for c in &self.channels {
            let (Some(b), Some(v)) = (s.get_mut(c.bone as usize), c.sample(frame)) else { continue };
            match c.kind {
                1..=3 => b.scale[(c.kind - 1) as usize] = v,
                4..=6 => b.rotation[(c.kind - 4) as usize] = v,
                7..=9 => b.translation[(c.kind - 7) as usize] = v,
                _ => {}
            }
        }
        s
    }
}
