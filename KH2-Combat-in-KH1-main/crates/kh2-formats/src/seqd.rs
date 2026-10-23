//! KH2's 2D sequences (`SEQD`, the `.2dd` files' entries of type 25): the HUD's animated pieces.
//!
//! The format's outline (a sequence is sprites, groups of sprites, animations of a group and
//! groups of animations; animations translate, rotate, scale and colour their group between two
//! keys) is from OpenKH's documentation of the 2D animation system (`docs/kh2/file/type/2ld.md`,
//! Apache-2.0, OpenKH Team and Contributors). The byte layout below and the meaning of each field
//! were worked out here from the user's own `field2d/us/zz0field.2dd` and KH2's HUD code that
//! plays it (`docs/drive-model.md`, "KH2's Drive gauge HUD"); no OpenKH code is used.
//!
//! Layout (all little-endian):
//! - header (0x30): `"SEQD"`, a version, then five `count, offset` pairs: sprites, parts,
//!   part groups, animations, animation groups.
//! - sprite (0x2C): `0`, `u0 v0 u1 v1` (texels of the sequence's image), two zeros, four corner
//!   colours (top left, top right, bottom left, bottom right; `0xAABBGGRR`, 0x80 = 1).
//! - part (0x14): the quad `left top right bottom` and its sprite.
//! - part group (4): first part (u16), count (u16).
//! - animation (0x90): flags, part group, first and last frame, the two keys (start, end) of:
//!   translation A (`+0x10` x, `+0x18` y) and B (`+0x20` x, `+0x28` y, added to A), four unknown
//!   ints, rotation (radians, `+0x40`), scale (`+0x48`), x and y scale (`+0x50`, `+0x58`), an
//!   easing curve (`+0x60`, a cubic Bézier's two control points, all zero for linear), the pivot
//!   (`+0x70` x, `+0x78` y), two unknown words, and the colour (`+0x88`).
//! - animation group (0x24): first animation (u16), count (u16), flags (bit 0: plays once),
//!   loop start and loop end (frames), and an attach point other pieces are placed at (`+0x10`).

use crate::reader::Reader;
use crate::{bad, Result};

pub const MAGIC: u32 = 0x4451_4553; // "SEQD"

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprite {
    /// `u0 v0 u1 v1` in the image's texels (u0 > u1: mirrored).
    pub uv: [i32; 4],
    /// Top left, top right, bottom left, bottom right; `0xAABBGGRR`, 0x80 = 1.
    pub colours: [u32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    /// `left top right bottom` around the animation's origin.
    pub rect: [i32; 4],
    pub sprite: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Animation {
    pub flags: u32,
    pub group: usize,
    pub start: i32,
    pub end: i32,
    pub translate_a: [[i32; 2]; 2],
    pub translate_b: [[i32; 2]; 2],
    pub rotation: [f32; 2],
    pub scale: [f32; 2],
    pub scale_x: [f32; 2],
    pub scale_y: [f32; 2],
    pub curve: [f32; 4],
    pub pivot: [[f32; 2]; 2],
    pub colour: [u32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationGroup {
    pub first: usize,
    pub count: usize,
    pub flags: u32,
    pub loop_start: i32,
    pub loop_end: i32,
    /// Where pieces attached to this one go (relative to its translation).
    pub attach: [i32; 2],
}

#[derive(Clone, Debug, Default)]
pub struct Sequence {
    pub sprites: Vec<Sprite>,
    pub parts: Vec<Part>,
    pub part_groups: Vec<(usize, usize)>,
    pub animations: Vec<Animation>,
    pub groups: Vec<AnimationGroup>,
}

/// One quad to draw: corners top left, bottom left, top right, bottom right; the sprite's `uv`;
/// each corner's colour (r, g, b, a with 128 = 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub corners: [[f32; 2]; 4],
    pub uv: [i32; 4],
    pub colours: [[f32; 4]; 4],
}

fn rgba(c: u32) -> [f32; 4] {
    [(c & 0xFF) as f32, (c >> 8 & 0xFF) as f32, (c >> 16 & 0xFF) as f32, (c >> 24) as f32]
}

/// A cubic Bézier easing through (0, 0), `(x1, y1)`, `(x2, y2)`, (1, 1): y at x = `t`.
fn ease(t: f32, c: [f32; 4]) -> f32 {
    if c == [0.0; 4] {
        return t;
    }
    let bez = |s: f32, p1: f32, p2: f32| 3.0 * (1.0 - s) * (1.0 - s) * s * p1 + 3.0 * (1.0 - s) * s * s * p2 + s * s * s;
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..24 {
        let s = (lo + hi) / 2.0;
        if bez(s, c[0], c[2]) < t {
            lo = s;
        } else {
            hi = s;
        }
    }
    bez((lo + hi) / 2.0, c[1], c[3])
}

impl Sequence {
    pub fn parse(buf: &[u8]) -> Result<Sequence> {
        let mut r = Reader::new(buf);
        if r.u32()? != MAGIC {
            return bad("not a SEQD sequence");
        }
        r.skip(4)?;
        let mut table = [(0usize, 0usize); 5];
        for t in table.iter_mut() {
            let (n, o) = (r.i32()?, r.i32()?);
            if !(0..=0x10000).contains(&n) || o < 0 {
                return bad("implausible SEQD table");
            }
            *t = (n as usize, o as usize);
        }
        let at = |k: usize, i: usize, size: usize| Reader::at(buf, table[k].1 + i * size);
        let mut s = Sequence::default();
        for i in 0..table[0].0 {
            let mut r = at(0, i, 0x2C);
            r.skip(4)?;
            let uv = [r.i32()?, r.i32()?, r.i32()?, r.i32()?];
            r.skip(8)?;
            let colours = [r.u32()?, r.u32()?, r.u32()?, r.u32()?];
            s.sprites.push(Sprite { uv, colours });
        }
        for i in 0..table[1].0 {
            let mut r = at(1, i, 0x14);
            let rect = [r.i32()?, r.i32()?, r.i32()?, r.i32()?];
            let sprite = r.i32()?.max(0) as usize;
            s.parts.push(Part { rect, sprite });
        }
        for i in 0..table[2].0 {
            let mut r = at(2, i, 4);
            s.part_groups.push((r.u16()? as usize, r.u16()? as usize));
        }
        for i in 0..table[3].0 {
            let mut r = at(3, i, 0x90);
            let flags = r.u32()?;
            let group = r.i32()?.max(0) as usize;
            let (start, end) = (r.i32()?, r.i32()?);
            let mut ints = [0i32; 8];
            for v in ints.iter_mut() {
                *v = r.i32()?;
            }
            r.skip(16)?;
            let mut f = [0f32; 16];
            for v in f.iter_mut() {
                *v = r.f32()?;
            }
            r.skip(8)?;
            let colour = [r.u32()?, r.u32()?];
            s.animations.push(Animation {
                flags,
                group,
                start,
                end,
                translate_a: [[ints[0], ints[1]], [ints[2], ints[3]]],
                translate_b: [[ints[4], ints[5]], [ints[6], ints[7]]],
                rotation: [f[0], f[1]],
                scale: [f[2], f[3]],
                scale_x: [f[4], f[5]],
                scale_y: [f[6], f[7]],
                curve: [f[8], f[9], f[10], f[11]],
                pivot: [[f[12], f[13]], [f[14], f[15]]],
                colour,
            });
        }
        for i in 0..table[4].0 {
            let mut r = at(4, i, 0x24);
            let (first, count) = (r.u16()? as usize, r.u16()? as usize);
            let flags = r.u32()?;
            let (loop_start, loop_end) = (r.i32()?, r.i32()?);
            let attach = [r.i32()?, r.i32()?];
            s.groups.push(AnimationGroup { first, count, flags, loop_start, loop_end, attach });
        }
        // Every index points inside its table.
        let ok = s.parts.iter().all(|p| p.sprite < s.sprites.len())
            && s.part_groups.iter().all(|&(a, n)| a + n <= s.parts.len())
            && s.animations.iter().all(|a| a.group < s.part_groups.len())
            && s.groups.iter().all(|g| g.first + g.count <= s.animations.len());
        if !ok {
            return bad("SEQD index out of range");
        }
        Ok(s)
    }

    /// The last frame of animation group `g` (its animations' latest end).
    pub fn length(&self, g: usize) -> i32 {
        self.groups.get(g).map_or(0, |gr| self.animations[gr.first..gr.first + gr.count].iter().map(|a| a.end).max().unwrap_or(0))
    }

    /// Whether group `g`, `t` frames after it started, has finished (it plays once and is past
    /// its end).
    pub fn finished(&self, g: usize, t: i32) -> bool {
        self.groups.get(g).is_none_or(|gr| gr.flags & 1 != 0 && t > self.length(g))
    }

    /// The frame group `g` shows `t` frames after it started: past its end it loops from its loop
    /// start to its loop end (or to its end), which holds it when they are the same frame.
    pub fn frame(&self, g: usize, t: i32) -> i32 {
        let Some(gr) = self.groups.get(g) else { return t };
        let end = self.length(g);
        if t <= end {
            return t;
        }
        if gr.flags & 1 != 0 {
            return end;
        }
        let (ls, le) = (gr.loop_start.clamp(0, end), if gr.loop_end > gr.loop_start { gr.loop_end } else { end });
        if le <= ls {
            return ls;
        }
        ls + (t - ls) % (le - ls)
    }

    /// The translation (A + B) of group `g`'s first animation active at frame `f` (where pieces
    /// attached to it go, with its attach point).
    pub fn origin(&self, g: usize, f: i32) -> [f32; 2] {
        let Some(gr) = self.groups.get(g) else { return [0.0; 2] };
        for a in &self.animations[gr.first..gr.first + gr.count] {
            if a.start <= f && f <= a.end {
                let t = ease(if a.end == a.start { 0.0 } else { (f - a.start) as f32 / (a.end - a.start) as f32 }, a.curve);
                let l = |k: [i32; 2]| k[0] as f32 + (k[1] - k[0]) as f32 * t;
                return [l(a.translate_a[0]) + l(a.translate_b[0]), l(a.translate_a[1]) + l(a.translate_b[1])];
            }
        }
        [0.0; 2]
    }

    /// The quads group `g` draws at frame `f` (from `frame`), appended to `out`.
    pub fn quads(&self, g: usize, f: i32, out: &mut Vec<Quad>) {
        let Some(gr) = self.groups.get(g) else { return };
        for a in &self.animations[gr.first..gr.first + gr.count] {
            if f < a.start || f > a.end {
                continue;
            }
            let t = ease(if a.end == a.start { 0.0 } else { (f - a.start) as f32 / (a.end - a.start) as f32 }, a.curve);
            let lf = |k: [f32; 2]| k[0] + (k[1] - k[0]) * t;
            let li = |k: [i32; 2]| k[0] as f32 + (k[1] - k[0]) as f32 * t;
            let (tx, ty) = (li(a.translate_a[0]) + li(a.translate_b[0]), li(a.translate_a[1]) + li(a.translate_b[1]));
            let rot = lf(a.rotation);
            let sc = lf(a.scale);
            let (sx, sy) = (lf(a.scale_x) * sc, lf(a.scale_y) * sc);
            let (px, py) = (lf(a.pivot[0]), lf(a.pivot[1]));
            let (c0, c1) = (rgba(a.colour[0]), rgba(a.colour[1]));
            let ac: [f32; 4] = std::array::from_fn(|i| c0[i] + (c1[i] - c0[i]) * t);
            let (sin, cos) = rot.sin_cos();
            let (first, n) = self.part_groups[a.group];
            for p in &self.parts[first..first + n] {
                let sp = self.sprites[p.sprite];
                let [l, tp, r, b] = p.rect;
                let corners = [(l, tp), (l, b), (r, tp), (r, b)].map(|(x, y)| {
                    let (x, y) = ((x as f32 - px) * sx, (y as f32 - py) * sy);
                    [x * cos - y * sin + px + tx, x * sin + y * cos + py + ty]
                });
                // (The sprite's corners are TL, TR, BL, BR; the quad's TL, BL, TR, BR.)
                let sc = sp.colours.map(rgba);
                let colours = [sc[0], sc[2], sc[1], sc[3]].map(|c| std::array::from_fn(|i| c[i] * ac[i] / 128.0));
                out.push(Quad { corners, uv: sp.uv, colours });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(b: &mut Vec<u8>, at: usize, v: &[u8]) {
        if b.len() < at + v.len() {
            b.resize(at + v.len(), 0);
        }
        b[at..at + v.len()].copy_from_slice(v);
    }

    /// One sprite, one part, one group, one animation sliding 10 to the right over 10 frames
    /// and fading in, in one looping animation group.
    fn synthetic() -> Vec<u8> {
        let mut b = vec![0u8; 0x30];
        put(&mut b, 0, b"SEQD");
        let le = |v: i32| v.to_le_bytes();
        let offs = [0x30, 0x60, 0x80, 0x90, 0x130];
        for (k, o) in offs.iter().enumerate() {
            put(&mut b, 8 + k * 8, &le(1));
            put(&mut b, 12 + k * 8, &le(*o));
        }
        // sprite
        for (i, v) in [0, 10, 20, 30, 40, 0, 0].iter().enumerate() {
            put(&mut b, 0x30 + i * 4, &le(*v));
        }
        for i in 0..4 {
            put(&mut b, 0x30 + 0x1C + i * 4, &0x8080_8080u32.to_le_bytes());
        }
        // part
        for (i, v) in [-5, -5, 5, 5, 0].iter().enumerate() {
            put(&mut b, 0x60 + i * 4, &le(*v));
        }
        // part group
        put(&mut b, 0x80, &[0, 0, 1, 0]);
        // animation
        let a = 0x90;
        put(&mut b, a + 4, &le(0));
        put(&mut b, a + 8, &le(0));
        put(&mut b, a + 0xC, &le(10));
        put(&mut b, a + 0x10, &le(0));
        put(&mut b, a + 0x14, &le(10));
        for (o, v) in [(0x48, 1.0f32), (0x4C, 1.0), (0x50, 1.0), (0x54, 1.0), (0x58, 1.0), (0x5C, 1.0)] {
            put(&mut b, a + o, &v.to_le_bytes());
        }
        put(&mut b, a + 0x88, &0x0080_8080u32.to_le_bytes());
        put(&mut b, a + 0x8C, &0x8080_8080u32.to_le_bytes());
        // animation group: loops 0..10
        put(&mut b, 0x130, &[0, 0, 1, 0]);
        put(&mut b, 0x130 + 0x10, &le(7));
        put(&mut b, 0x130 + 0x14, &le(-2));
        b.resize(0x130 + 0x24, 0);
        b
    }

    #[test]
    fn parses_and_animates() {
        let s = Sequence::parse(&synthetic()).unwrap();
        assert_eq!(s.sprites[0].uv, [10, 20, 30, 40]);
        assert_eq!(s.groups[0].attach, [7, -2]);
        assert_eq!(s.length(0), 10);
        let mut q = Vec::new();
        s.quads(0, 5, &mut q);
        assert_eq!(q.len(), 1);
        assert_eq!(q[0].corners[0], [0.0, -5.0]);
        assert_eq!(q[0].corners[3], [10.0, 5.0]);
        assert!((q[0].colours[0][3] - 64.0).abs() < 0.01);
        // Past the end it holds at the loop start (0) to its end (10): frame 13 is frame 3.
        assert_eq!(s.frame(0, 13), 3);
        assert!(!s.finished(0, 13));
        assert_eq!(s.origin(0, 10), [10.0, 0.0]);
    }

    #[test]
    fn rejects_bad_indices() {
        let mut b = synthetic();
        b[0x60 + 16] = 9; // the part's sprite
        assert!(Sequence::parse(&b).is_err());
    }
}
