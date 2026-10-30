//! KH2's 2D sequence player (the `SEQD` entries of KH2's `.2dd` files), as KH2's own player runs it,
//! for KH2's HUD pieces drawn by KH1 (the Drive gauge, t939; `docs/kh1-drive-forms-native.md` §14).
//! Pure: it gives the quads (corners, texture rectangles, colours, blend) a frame shows; `win/gaugehud`
//! hands them to KH1's own sprite draw.
//!
//! Steam KH2, Checked (decomp) and (exe) (the decomp drops the float arguments, read from the
//! assembly):
//! - **The file** (the player's set-up `0x140192EB0`): header `"SEQD"`, a version, then count and
//!   offset of the sprites (0x2C: `+4` u0 v0 u1 v1 texels, `+0x14` / `+0x18` a texture scroll, `+0x1C`
//!   four corner colours top left, top right, bottom left, bottom right, `0xAABBGGRR`, 0x80 = 1), the
//!   parts (0x14: left top right bottom, the sprite), the part groups (4: first part, count), the
//!   animations (0x90) and the animation groups (0x24: first animation, count, `+4` u16 plays, `+8`
//!   loop start, `+0xC` loop end, `+0x10` / `+0x14` the attach point).
//! - **A group** (`0x140192050`): its length is the latest animation end (less one where flag 0x8000
//!   makes the end exclusive), cut at the loop end when that is set; the player's frame runs from 0
//!   one per 60 Hz frame (`0x140192130`, the clock `0x140717484` = 1.0, Checked (exe)) and past the
//!   length goes back to the loop start; with `plays` set it stops on the last frame after that many
//!   runs. A new group starts at frame 0 (`0x140193250`, only if it is another group).
//! - **An animation** (`0x140192250`), shown while start <= frame <= end (end exclusive with 0x8000):
//!   t = (frame - start) / (end - start), eased unless flag 1 by KH2's cubic `t^3 + 3t(1-t)^2 c1 +
//!   3t^2(1-t) c2` (c1 `+0x64`, c2 `+0x6C`). Each key pair is interpolated with t: the place `+0x10` /
//!   `+0x18` (ints; held at the first key when every animation of the group has flags 0x1010, the
//!   group bit `0x140191E80` sets), a bounce unless 0x10 (sine, cosine with 0x8, of count `+0x80` /
//!   `+0x82` × t × pi, times the amplitude `+0x70` / `+0x78`), the pivot `+0x20` / `+0x28` (0 with
//!   0x4000), the rotations x y z `+0x30 / +0x38 / +0x40` (none with 0x20; matrix `0x140196400`), the
//!   scale `+0x48` and x / y scales `+0x50 / +0x58` (none with 0x40; `0x140196570`), the colour
//!   `+0x88` / `+0x8C` per byte (the first only with 0x80; with 0x400 the sprites' own colours), and
//!   the blend `+0x84` (0 normal, 1 additive, 2 subtractive: KH2's ALPHA 0x44 / 0x48 / 0x42).
//! - **A part** (`0x1401969A0`; no sprite of the gauge's scrolls, so not `0x1401970A0`): corners
//!   ((rect + pivot) × 16, scaled, rotated, truncated) + (the player's place + the animation's) × 16;
//!   texture coordinates texel × 16 + 8; each corner's colour the sprite's times the animation's
//!   (`0x14018FD20`: c × p >> 7 unless p is 0x80, at most 255).
//! - The player keeps where its last part group was drawn (`+0xB0`, `+0xB4`): pieces placed on it
//!   (KH2's parented pieces) are placed from there.

use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprite {
    pub uv: [i32; 4],
    pub colours: [u32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    pub rect: [i32; 4],
    pub sprite: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anim {
    pub flags: u32,
    pub parts: usize,
    pub start: i32,
    pub end: i32,
    pub x: [i32; 2],
    pub y: [i32; 2],
    pub px: [i32; 2],
    pub py: [i32; 2],
    pub rot: [[f32; 2]; 3],
    pub scale: [f32; 2],
    pub sx: [f32; 2],
    pub sy: [f32; 2],
    pub c1: f32,
    pub c2: f32,
    pub bx: [f32; 2],
    pub by: [f32; 2],
    pub bnx: u16,
    pub bny: u16,
    pub blend: u32,
    pub colour: [u32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Group {
    pub first: usize,
    pub count: usize,
    pub plays: u16,
    pub loop_start: i32,
    pub loop_end: i32,
    pub attach: [i32; 2],
    /// Every animation has flags 0x1010: the place is held at its first key (`0x140191E80`).
    pub fixed: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Seq {
    pub sprites: Vec<Sprite>,
    pub parts: Vec<Part>,
    pub part_groups: Vec<(usize, usize)>,
    pub anims: Vec<Anim>,
    pub groups: Vec<Group>,
}

fn u16_at(b: &[u8], o: usize) -> Result<u16, String> {
    b.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).ok_or_else(|| format!("SEQD cut short at {o:#x}"))
}
fn u32_at(b: &[u8], o: usize) -> Result<u32, String> {
    b.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])).ok_or_else(|| format!("SEQD cut short at {o:#x}"))
}
fn i32_at(b: &[u8], o: usize) -> Result<i32, String> {
    u32_at(b, o).map(|v| v as i32)
}
fn f32_at(b: &[u8], o: usize) -> Result<f32, String> {
    u32_at(b, o).map(f32::from_bits)
}

impl Seq {
    pub fn parse(b: &[u8]) -> Result<Seq, String> {
        if b.len() < 0x30 || &b[..4] != b"SEQD" {
            return Err("not a SEQD".into());
        }
        let mut t = [(0usize, 0usize); 5];
        for (k, e) in t.iter_mut().enumerate() {
            let (n, o) = (u32_at(b, 8 + k * 8)? as usize, u32_at(b, 12 + k * 8)? as usize);
            if n > 0x10000 {
                return Err("implausible SEQD table".into());
            }
            *e = (n, o);
        }
        let mut s = Seq::default();
        for i in 0..t[0].0 {
            let o = t[0].1 + i * 0x2C;
            s.sprites.push(Sprite {
                uv: [i32_at(b, o + 4)?, i32_at(b, o + 8)?, i32_at(b, o + 0xC)?, i32_at(b, o + 0x10)?],
                colours: [u32_at(b, o + 0x1C)?, u32_at(b, o + 0x20)?, u32_at(b, o + 0x24)?, u32_at(b, o + 0x28)?],
            });
        }
        for i in 0..t[1].0 {
            let o = t[1].1 + i * 0x14;
            s.parts.push(Part { rect: [i32_at(b, o)?, i32_at(b, o + 4)?, i32_at(b, o + 8)?, i32_at(b, o + 0xC)?], sprite: u32_at(b, o + 0x10)? as usize });
        }
        for i in 0..t[2].0 {
            let o = t[2].1 + i * 4;
            s.part_groups.push((u16_at(b, o)? as usize, u16_at(b, o + 2)? as usize));
        }
        for i in 0..t[3].0 {
            let o = t[3].1 + i * 0x90;
            let f2 = |at: usize| -> Result<[f32; 2], String> { Ok([f32_at(b, o + at)?, f32_at(b, o + at + 4)?]) };
            let i2 = |at: usize| -> Result<[i32; 2], String> { Ok([i32_at(b, o + at)?, i32_at(b, o + at + 4)?]) };
            s.anims.push(Anim {
                flags: u32_at(b, o)?,
                parts: u32_at(b, o + 4)? as usize,
                start: i32_at(b, o + 8)?,
                end: i32_at(b, o + 0xC)?,
                x: i2(0x10)?,
                y: i2(0x18)?,
                px: i2(0x20)?,
                py: i2(0x28)?,
                rot: [f2(0x30)?, f2(0x38)?, f2(0x40)?],
                scale: f2(0x48)?,
                sx: f2(0x50)?,
                sy: f2(0x58)?,
                c1: f32_at(b, o + 0x64)?,
                c2: f32_at(b, o + 0x6C)?,
                bx: f2(0x70)?,
                by: f2(0x78)?,
                bnx: u16_at(b, o + 0x80)?,
                bny: u16_at(b, o + 0x82)?,
                blend: u32_at(b, o + 0x84)?,
                colour: [u32_at(b, o + 0x88)?, u32_at(b, o + 0x8C)?],
            });
        }
        for i in 0..t[4].0 {
            let o = t[4].1 + i * 0x24;
            let (first, count) = (u16_at(b, o)? as usize, u16_at(b, o + 2)? as usize);
            let anims = s.anims.get(first..first + count).ok_or("SEQD group past its animations")?;
            s.groups.push(Group {
                first,
                count,
                plays: u16_at(b, o + 4)?,
                loop_start: i32_at(b, o + 8)?,
                loop_end: i32_at(b, o + 0xC)?,
                attach: [i32_at(b, o + 0x10)?, i32_at(b, o + 0x14)?],
                fixed: count > 0 && anims.iter().all(|a| a.flags & 0x1010 == 0x1010),
            });
        }
        let ok = s.parts.iter().all(|p| p.sprite < s.sprites.len())
            && s.part_groups.iter().all(|&(a, n)| a + n <= s.parts.len())
            && s.anims.iter().all(|a| a.parts < s.part_groups.len());
        if !ok {
            return Err("SEQD index out of range".into());
        }
        Ok(s)
    }

    /// A group's last frame (`0x140192050`).
    pub fn length(&self, g: usize) -> i32 {
        let Some(gr) = self.groups.get(g) else { return 0 };
        let mut len = 0i32;
        for a in &self.anims[gr.first..gr.first + gr.count] {
            if len < a.end {
                len = if a.flags & 0x8000 != 0 { a.end - 1 } else { a.end };
            }
        }
        if gr.loop_end != 0 && gr.loop_end < len {
            len = gr.loop_end;
        }
        len
    }
}

/// One quad: corners top left, top right, bottom left, bottom right (KH2's order) in 1/16 KH2
/// units, texture coordinates (texel × 16 + 8) per corner, colours per corner (`0xAABBGGRR`, 0x80 =
/// 1), and the blend (0 normal, 1 additive, 2 subtractive).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub at: [[i32; 2]; 4],
    pub uv: [[i32; 2]; 4],
    pub colours: [u32; 4],
    pub blend: u32,
}

/// A piece's player (KH2's sprite player's part of it: `+0x140` state, `+0x148` frame, `+0x14C`
/// end, `+0x184` plays left, `+0x188` loop start, `+0xB0` / `+0xB4` where it last drew).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Player {
    pub group: Option<usize>,
    /// 0 none, 1 started, 2 running, 3 stopped.
    pub state: u8,
    pub frame: f32,
    end: f32,
    plays_left: u32,
    loop_start: i32,
    /// Where its last part group was drawn (KH2 units).
    pub origin: [i32; 2],
}

impl Default for Player {
    fn default() -> Player {
        Player { group: None, state: 0, frame: 0.0, end: 0.0, plays_left: 0, loop_start: 0, origin: [0, 0] }
    }
}

impl Player {
    /// Group `g`, from its start if it is another one (`0x140193250`).
    pub fn set(&mut self, seq: &Seq, g: usize) {
        if self.group != Some(g) {
            self.restart(seq, g);
        }
    }

    /// Group `g` from its start (`0x140193280`).
    pub fn restart(&mut self, seq: &Seq, g: usize) {
        let Some(gr) = seq.groups.get(g) else {
            self.group = None;
            self.state = 0;
            return;
        };
        self.group = Some(g);
        self.plays_left = gr.plays as u32;
        self.loop_start = gr.loop_start;
        let len = seq.length(g) as f32;
        if len < self.frame {
            self.frame = len;
        }
        self.end = len + 1.0;
        self.state = 1;
        self.frame = 0.0;
    }

    pub fn hide(&mut self) {
        self.group = None;
        self.state = 0;
    }

    /// One step of `dt` 60 Hz frames (`0x140192130`).
    pub fn step(&mut self, dt: f32) {
        match self.state {
            1 => {
                self.state = 2;
                self.frame = 0.0;
            }
            2 => {
                self.frame += dt;
                if self.end <= self.frame {
                    self.frame = self.loop_start as f32;
                    if self.plays_left != 0 {
                        self.plays_left -= 1;
                        if self.plays_left == 0 {
                            self.state = 3;
                            self.frame = self.end - 1.0;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// The frame shown (`+0x104`).
    pub fn shown(&self) -> f32 {
        (self.frame as i32) as f32
    }

    /// The quads this frame with the player placed at `pos` (KH2 units), appended to `out`
    /// (`0x140192250` + `0x1401969A0`).
    pub fn draw(&mut self, seq: &Seq, pos: [i32; 2], out: &mut Vec<Quad>) {
        let (Some(g), true) = (self.group, self.state != 0) else { return };
        let Some(gr) = seq.groups.get(g).copied() else { return };
        let f = self.shown();
        for a in &seq.anims[gr.first..gr.first + gr.count] {
            let (s, e) = (a.start as f32, a.end as f32);
            if f < s || f > e || (f == e && a.flags & 0x8000 != 0) {
                continue;
            }
            let span = e - s;
            let mut t = if span == 0.0 { 1.0 } else { (f - s) / span };
            if a.flags & 1 == 0 {
                t = ease(t, a.c1, a.c2);
            }
            let lerp = |k: [f32; 2]| (k[1] - k[0]) * t + k[0];
            let ilerp = |k: [i32; 2]| ((k[1] - k[0]) as f32 * t) as i32 + k[0];
            let rot = (a.flags & 0x20 == 0).then(|| matrix(lerp(a.rot[0]), lerp(a.rot[1]), lerp(a.rot[2])));
            let scale = (a.flags & 0x40 == 0).then(|| {
                let s = lerp(a.scale);
                (s * lerp(a.sx), s * lerp(a.sy))
            });
            let colour = if a.flags & 0x400 != 0 {
                None
            } else if a.flags & 0x80 != 0 {
                Some(a.colour[0])
            } else {
                let mut c = 0u32;
                for sh in [0u32, 8, 16, 24] {
                    let (b0, b1) = ((a.colour[0] >> sh & 0xFF) as i32, (a.colour[1] >> sh & 0xFF) as i32);
                    c |= ((((b1 - b0) as f32 * t) as i32 + b0) as u32 & 0xFF) << sh;
                }
                Some(c)
            };
            let (mut x, mut y);
            if gr.fixed {
                x = a.x[0];
                y = a.y[0];
            } else {
                x = ilerp(a.x);
                y = ilerp(a.y);
                if a.flags & 0x10 == 0 {
                    let tp = t * PI;
                    let (ax, ay) = (a.bnx as f32 * tp, a.bny as f32 * tp);
                    let (wx, wy) = if a.flags & 8 == 0 { (ax.sin(), ay.sin()) } else { (ax.cos(), ay.cos()) };
                    x += (wx * lerp(a.bx)) as i32;
                    y += (wy * lerp(a.by)) as i32;
                }
            }
            let (pvx, pvy) = if a.flags & 0x4000 != 0 { (0, 0) } else { (ilerp(a.px), ilerp(a.py)) };
            let (ox, oy) = (pos[0] + x, pos[1] + y);
            self.origin = [ox, oy];
            let (first, n) = seq.part_groups[a.parts];
            for p in &seq.parts[first..first + n] {
                let sp = &seq.sprites[p.sprite];
                let mut c = [
                    (p.rect[0] * 16 + pvx * 16) as f32,
                    (p.rect[1] * 16 + pvy * 16) as f32,
                    (p.rect[2] * 16 + pvx * 16) as f32,
                    (p.rect[3] * 16 + pvy * 16) as f32,
                ];
                if let Some((kx, ky)) = scale {
                    c = [c[0] * kx, c[1] * ky, c[2] * kx, c[3] * ky];
                }
                let [x0, y0, x1, y1] = c;
                let corners = match rot {
                    Some(m) => {
                        let (a0, b0, c0, d0) = (m[0] * x0, m[2] * y0, m[1] * x0, m[3] * y0);
                        let (a1, b1, c1, d1) = (m[0] * x1, m[2] * y1, m[1] * x1, m[3] * y1);
                        [[(b0 + a0) as i32, (d0 + c0) as i32], [(a1 + b0) as i32, (c1 + d0) as i32], [(b1 + a0) as i32, (d1 + c0) as i32], [(b1 + a1) as i32, (d1 + c1) as i32]]
                    }
                    None => [[x0 as i32, y0 as i32], [x1 as i32, y0 as i32], [x0 as i32, y1 as i32], [x1 as i32, y1 as i32]],
                };
                let at = corners.map(|[cx, cy]| [cx + ox * 16, cy + oy * 16]);
                let [u0, v0, u1, v1] = sp.uv.map(|v| v * 16 + 8);
                let colours = match colour {
                    Some(pc) => sp.colours.map(|sc| times(sc, pc)),
                    None => sp.colours,
                };
                out.push(Quad { at, uv: [[u0, v0], [u1, v0], [u0, v1], [u1, v1]], colours, blend: a.blend });
            }
        }
    }
}

/// KH2's easing (the assembly's order of operations, `0x140192467`).
fn ease(t: f32, c1: f32, c2: f32) -> f32 {
    let mut t2 = t * t;
    let mut u = 1.0 - t;
    let mut k1 = 1.0 - c1;
    let mut uu = u * u;
    let mut a = uu;
    uu *= u;
    a *= 3.0;
    u *= 3.0;
    a *= t;
    u *= t2;
    t2 *= t;
    k1 *= a;
    let k2 = (1.0 - c2) * u;
    t2 *= 0.0;
    k1 += uu;
    k1 += k2;
    k1 += t2;
    1.0 - k1
}

/// KH2's rotation (`0x140196400`): `[m34, m38, m3C, m40]`, x' = m34 x + m3C y, y' = m38 x + m40 y.
fn matrix(rx: f32, ry: f32, rz: f32) -> [f32; 4] {
    let (sx, cx) = (rx.sin(), rx.cos());
    let (sy, cy) = (ry.sin(), ry.cos());
    let (mut sz, mut cz) = (rz.sin(), rz.cos());
    if cz == 1.0 {
        sz = 0.0;
    } else if sz == 1.0 {
        cz = 0.0;
    }
    [cz * cy, -(sz * cy), sy * sx * cz + sz * cx, cz * cx - sy * sx * sz]
}

/// A colour times another per channel (`0x14018FD20`: c × p >> 7 unless p is 0x80, at most 255).
pub fn times(c: u32, p: u32) -> u32 {
    let mut out = 0;
    for sh in [0u32, 8, 16, 24] {
        let (a, b) = (c >> sh & 0xFF, p >> sh & 0xFF);
        let v = if b == 0x80 { a } else { (a * b >> 7).min(255) };
        out |= v << sh;
    }
    out
}

// ---------------------------------------------------------------------------- the fill's mask

/// A point with texture coordinates and a colour, for the cut fill.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vert {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
    pub c: [f32; 4],
}

/// KH2's fill mask (the player's gauge `0x1401816C0`, the mask quad `0x14016D600`): a box of `w` ×
/// `h` (50 × 120) centred at `off` (`+0x114` / `+0x118`, (−25, −60)) from `pivot` (`+0xC` / `+0xE`),
/// turned by `angle` about the pivot (KH2's `0x140141320`: x' = x cos − y sin, y' = x sin + y cos). It writes only depth (its GS TEST: alpha test never, Z only on
/// fail, Z test always), so the fill drawn under it doesn't show where it lies. Returns its corners
/// (KH2 units).
pub fn mask_corners(pivot: [f32; 2], angle: f32, w: f32, h: f32, off: [f32; 2]) -> [[f32; 2]; 4] {
    let (s, c) = (angle.sin(), angle.cos());
    let (hw, hh) = (w / 2.0, h / 2.0);
    let local = [[off[0] - hw, off[1] - hh], [off[0] + hw, off[1] - hh], [off[0] + hw, off[1] + hh], [off[0] - hw, off[1] + hh]];
    local.map(|[x, y]| [pivot[0] + x * c - y * s, pivot[1] + x * s + y * c])
}

/// The quad's area outside the convex polygon `mask` (its corners in order), as triangles, each with
/// its texture coordinates and colours from the quad's (affine over the quad: exact for the axis
/// aligned and sheared quads KH2's fills are).
pub fn outside(q: &Quad, mask: &[[f32; 2]; 4]) -> Vec<[Vert; 3]> {
    let p = |i: usize| [q.at[i][0] as f32 / 16.0, q.at[i][1] as f32 / 16.0];
    let (o, ex, ey) = (p(0), sub(p(1), p(0)), sub(p(2), p(0)));
    let det = ex[0] * ey[1] - ex[1] * ey[0];
    if det.abs() < 1e-6 {
        return Vec::new();
    }
    let rgba = |c: u32| [(c & 0xFF) as f32, (c >> 8 & 0xFF) as f32, (c >> 16 & 0xFF) as f32, (c >> 24) as f32];
    let cols = q.colours.map(rgba);
    let vert = |pt: [f32; 2]| {
        let d = sub(pt, o);
        let a = (d[0] * ey[1] - d[1] * ey[0]) / det;
        let b = (ex[0] * d[1] - ex[1] * d[0]) / det;
        let l = |k: usize| q.uv[0][k] as f32 + a * (q.uv[1][k] - q.uv[0][k]) as f32 + b * (q.uv[2][k] - q.uv[0][k]) as f32;
        let (u, v) = (l(0), l(1));
        let mut c = [0f32; 4];
        for k in 0..4 {
            let top = cols[0][k] + a * (cols[1][k] - cols[0][k]);
            let bottom = cols[2][k] + a * (cols[3][k] - cols[2][k]);
            c[k] = top + b * (bottom - top);
        }
        Vert { x: pt[0], y: pt[1], u, v, c }
    };
    let quad = vec![p(0), p(1), p(3), p(2)];
    // (The mask's winding, so "inside" is the same side for every edge.)
    let area: f32 = (0..4).map(|i| mask[i][0] * mask[(i + 1) % 4][1] - mask[(i + 1) % 4][0] * mask[i][1]).sum();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let mut rest = quad;
    let mut pieces = Vec::new();
    for i in 0..4 {
        let (a, b) = (mask[i], mask[(i + 1) % 4]);
        let side = |pt: [f32; 2]| sign * ((b[0] - a[0]) * (pt[1] - a[1]) - (b[1] - a[1]) * (pt[0] - a[0]));
        let out = clip(&rest, |pt| -side(pt));
        if out.len() >= 3 {
            pieces.push(out);
        }
        rest = clip(&rest, side);
        if rest.len() < 3 {
            break;
        }
    }
    let mut tris = Vec::new();
    for poly in pieces {
        for k in 1..poly.len() - 1 {
            tris.push([vert(poly[0]), vert(poly[k]), vert(poly[k + 1])]);
        }
    }
    tris
}

fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

/// The polygon's part where `d(point) >= 0` (Sutherland–Hodgman, one edge).
fn clip(poly: &[[f32; 2]], d: impl Fn([f32; 2]) -> f32) -> Vec<[f32; 2]> {
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (da, db) = (d(a), d(b));
        if da >= 0.0 {
            out.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            let t = da / (da - db);
            out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seq_bytes() -> Vec<u8> {
        // One sprite, one part, one part group, two animations, one group.
        let mut b = vec![0u8; 0x30];
        b[..4].copy_from_slice(b"SEQD");
        let tables = [(1u32, 0x30u32), (1, 0x5C), (1, 0x70), (2, 0x74), (1, 0x194)];
        for (k, (n, o)) in tables.iter().enumerate() {
            b[8 + k * 8..12 + k * 8].copy_from_slice(&n.to_le_bytes());
            b[12 + k * 8..16 + k * 8].copy_from_slice(&o.to_le_bytes());
        }
        let mut spr = vec![0u8; 0x2C];
        for (i, v) in [1i32, 36, 39, 59].iter().enumerate() {
            spr[4 + i * 4..8 + i * 4].copy_from_slice(&v.to_le_bytes());
        }
        for i in 0..4 {
            spr[0x1C + i * 4..0x20 + i * 4].copy_from_slice(&0x8040_8080u32.to_le_bytes());
        }
        b.extend(spr);
        let mut part = vec![0u8; 0x14];
        for (i, v) in [-39i32, -22, -1, 1, 0].iter().enumerate() {
            part[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        b.extend(part);
        b.extend([0u8, 0, 1, 0]);
        let anim = |flags: u32, st: i32, en: i32, x: [i32; 2], col: [u32; 2]| {
            let mut a = vec![0u8; 0x90];
            a[0..4].copy_from_slice(&flags.to_le_bytes());
            a[8..12].copy_from_slice(&st.to_le_bytes());
            a[12..16].copy_from_slice(&en.to_le_bytes());
            a[0x10..0x14].copy_from_slice(&x[0].to_le_bytes());
            a[0x14..0x18].copy_from_slice(&x[1].to_le_bytes());
            a[0x18..0x1C].copy_from_slice(&24i32.to_le_bytes());
            a[0x1C..0x20].copy_from_slice(&24i32.to_le_bytes());
            for (o, v) in [(0x48, 1.0f32), (0x4C, 1.0), (0x50, 1.0), (0x54, 1.0), (0x58, 1.0), (0x5C, 1.0)] {
                a[o..o + 4].copy_from_slice(&v.to_bits().to_le_bytes());
            }
            a[0x88..0x8C].copy_from_slice(&col[0].to_le_bytes());
            a[0x8C..0x90].copy_from_slice(&col[1].to_le_bytes());
            a
        };
        // (Linear, no bounce, no rotation: flags 1 | 0x10 | 0x20.)
        b.extend(anim(0x31, 0, 10, [-79, -69], [0x8080_8080, 0x8080_8080]));
        b.extend(anim(0x31 | 0x8000, 10, 30, [-69, -69], [0x80F0_F0F0, 0x8080_8080]));
        let mut g = vec![0u8; 0x24];
        g[2..4].copy_from_slice(&2u16.to_le_bytes());
        g[4..6].copy_from_slice(&1u16.to_le_bytes());
        g[8..12].copy_from_slice(&5i32.to_le_bytes());
        g[0x10..0x14].copy_from_slice(&52i32.to_le_bytes());
        b.extend(g);
        b
    }

    #[test]
    fn parse_and_length() {
        let s = Seq::parse(&seq_bytes()).unwrap();
        assert_eq!(s.groups[0].attach, [52, 0]);
        // (Ends 10 and 30, the second exclusive: 29.)
        assert_eq!(s.length(0), 29);
        assert!(!s.groups[0].fixed);
    }

    #[test]
    fn plays_once_and_stops_on_its_last_frame() {
        let s = Seq::parse(&seq_bytes()).unwrap();
        let mut p = Player::default();
        p.set(&s, 0);
        p.step(1.0);
        assert_eq!(p.shown(), 0.0);
        for _ in 0..40 {
            p.step(1.0);
        }
        assert_eq!((p.state, p.shown()), (3, 29.0));
    }

    #[test]
    fn quads_move_with_the_keys() {
        let s = Seq::parse(&seq_bytes()).unwrap();
        let mut p = Player::default();
        p.set(&s, 0);
        p.step(1.0);
        for _ in 0..5 {
            p.step(1.0);
        }
        let mut q = Vec::new();
        p.draw(&s, [0, 0], &mut q);
        // (Frame 5: x = -79 + 10 * 0.5 = -74; the part's left -39.)
        assert_eq!(q.len(), 1);
        assert_eq!(q[0].at[0], [(-39 - 74) * 16, (-22 + 24) * 16]);
        assert_eq!(q[0].uv[0], [1 * 16 + 8, 36 * 16 + 8]);
        assert_eq!(p.origin, [-74, 24]);
        assert_eq!(q[0].colours[0], 0x8040_8080);
    }

    #[test]
    fn colour_times() {
        assert_eq!(times(0x8040_80FF, 0x8080_8080), 0x8040_80FF);
        assert_eq!(times(0x0000_00FF, 0x0000_0040), 0x0000_007F);
        assert_eq!(times(0x0000_00FF, 0x0000_00FF), 0x0000_00FF);
    }

    #[test]
    fn ease_ends() {
        assert!((ease(0.0, 0.0, 1.0) - 0.0).abs() < 1e-6);
        assert!((ease(1.0, 0.0, 1.0) - 1.0).abs() < 1e-6);
        // (c1 = c2 = 0: t^3.)
        assert!((ease(0.5, 0.0, 0.0) - 0.125).abs() < 1e-6);
    }

    #[test]
    fn the_mask_sweeps_the_fill() {
        // (The Drive fill relative to its bar: x -1..48, y -22..1; the mask's pivot (50, 100).)
        let q = Quad { at: [[-16, -352], [768, -352], [-16, 16], [768, 16]], uv: [[0, 0], [784, 0], [0, 368], [784, 368]], colours: [0x8080_8080; 4], blend: 0 };
        let area = |t: &[[Vert; 3]]| t.iter().map(|t| ((t[1].x - t[0].x) * (t[2].y - t[0].y) - (t[2].x - t[0].x) * (t[1].y - t[0].y)).abs() / 2.0).sum::<f32>();
        let at = |a: f32| area(&outside(&q, &mask_corners([50.0, 100.0], a, 50.0, 120.0, [-25.0, -60.0])));
        let full = 49.0 * 23.0;
        // Empty but the edge KH2 leaves (the column left of the mask and the rows above it).
        assert!(at(0.0) < 0.12 * full);
        assert!(at(0.2) > 0.3 * full && at(0.2) < 0.7 * full);
        assert!((at(0.8) - full).abs() < 1.0);
        // The texture coordinates follow the points.
        let t = outside(&q, &mask_corners([50.0, 100.0], 0.8, 50.0, 120.0, [-25.0, -60.0]));
        for v in t.iter().flatten() {
            assert!((v.u - (v.x + 1.0) * 16.0).abs() < 0.5 && (v.v - (v.y + 22.0) * 16.0).abs() < 0.5);
        }
    }
}
