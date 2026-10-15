//! DPD particle packages: the effect programs, textures, sprite shapes and
//! small models that a PAX effect plays.
//!
//! Container layout and the program type names are from OpenKH
//! (`OpenKh.Kh2/Dpd.cs`, `Dpd.ParticleData.cs`, `DpdPData.cs`,
//! `Dpd.Texture.cs`, `Dpd.Shape.cs`, `DpdModel.cs`, `DpdVsf.cs`; Apache-2.0,
//! Copyright OpenKH Team and Contributors), rewritten in Rust. Field meanings
//! beyond OpenKH's (key frames, work offsets, shape sequences, model
//! primitive kinds, texture lookup) come from our own reading of the PC
//! executable; `docs/kh2-effects.md` explains how the game runs these.
//!
//! ```text
//! DPD:  0x96 | 5 lists (count i32 + offsets): particle data, textures,
//!       shapes, models, vertex sets | 4 x i32 VRAM bounds
//! particle data: 0x110-byte header, then a program file:
//!       end frame | flags (bit 0 loops) | type list | emitter list | emitters
//! emitter: next | cv offsets | 2 x editor ints | start | total | loop a | loop b
//!       | work size | priority i16 | program count i16 | programs x 16 bytes
//! program: type | key size u16 | key count u16 | keys | work offsets
//! ```
//!
//! Frames inside programs are fixed point, 4096 = one effect tick. The
//! effect runs one tick per two 60 Hz game frames.

use crate::reader::Reader;
use crate::{bad, Error, Result};

pub const MAGIC: u32 = 0x96;
/// One effect tick in key and emitter frame values.
pub const TICK: i32 = 0x1000;
/// 60 Hz game frames per effect tick.
pub const GAME_FRAMES_PER_TICK: f32 = 2.0;
/// `total` / loop frame value meaning "never".
pub const NEVER: i32 = 0x7000_0000;
/// `start` value of emitters that only another program spawns.
pub const SPAWNED: i32 = 0x7fff_ffff;
/// Word that follows the last key of every program.
pub const KEY_END: u32 = 0xffff_f000;

fn oob(what: &str) -> Error {
    Error::Format(format!("DPD: {what} out of range"))
}

fn offsets(r: &mut Reader, what: &str) -> Result<Vec<usize>> {
    let n = r.i32()?;
    if !(0..=4096).contains(&n) {
        return bad(format!("DPD: implausible {what} count {n}"));
    }
    (0..n).map(|_| Ok(r.u32()? as usize)).collect()
}

fn i16x3(r: &mut Reader) -> Result<[i16; 3]> {
    Ok([r.i16()?, r.i16()?, r.i16()?])
}

/// A whole DPD.
#[derive(Clone, Debug, PartialEq)]
pub struct Dpd {
    pub particles: Vec<ParticleData>,
    pub textures: Vec<Texture>,
    pub shapes: Vec<Shape>,
    pub models: Vec<Model>,
    pub vertex_sets: Vec<VertexSet>,
    /// Texture and CLUT VRAM ranges (editor data).
    pub vram: [i32; 4],
}

impl Dpd {
    pub fn parse(buf: &[u8]) -> Result<Dpd> {
        let mut r = Reader::new(buf);
        if r.u32()? != MAGIC {
            return bad("not a DPD");
        }
        let pd = offsets(&mut r, "particle data")?;
        let tex = offsets(&mut r, "texture")?;
        let shp = offsets(&mut r, "shape")?;
        let mdl = offsets(&mut r, "model")?;
        let vsf = offsets(&mut r, "vertex set")?;
        let vram = [r.i32()?, r.i32()?, r.i32()?, r.i32()?];
        let at = |o: usize, what: &str| buf.get(o..).ok_or_else(|| oob(what));
        Ok(Dpd {
            particles: pd.iter().map(|&o| ParticleData::parse(at(o, "particle data")?)).collect::<Result<_>>()?,
            textures: tex.iter().map(|&o| Texture::parse(at(o, "texture")?)).collect::<Result<_>>()?,
            shapes: shp.iter().map(|&o| Shape::parse(at(o, "shape")?)).collect::<Result<_>>()?,
            models: mdl.iter().map(|&o| Model::parse(at(o, "model")?)).collect::<Result<_>>()?,
            vertex_sets: vsf.iter().map(|&o| VertexSet::parse(at(o, "vertex set")?)).collect::<Result<_>>()?,
            vram,
        })
    }

    /// Programs name shapes by the number in the shape's own header.
    pub fn shape(&self, number: i32) -> Option<&Shape> {
        self.shapes.iter().find(|s| s.number as i32 == number)
    }

    /// Programs name models by the number in the model's own header.
    pub fn model(&self, number: i32) -> Option<&Model> {
        self.models.iter().find(|m| m.number as i32 == number)
    }

    pub fn vertex_set(&self, number: i32) -> Option<&VertexSet> {
        self.vertex_sets.iter().find(|v| v.number == number)
    }

    /// The texture whose pixels start at GS block address `tbp` (file
    /// address space, see [`Tex0`]).
    pub fn texture_at(&self, tbp: u16) -> Option<&Texture> {
        self.textures.iter().find(|t| t.tbp == tbp)
    }

    /// The texture whose palette is stored at GS block address `cbp`.
    pub fn palette_at(&self, cbp: u16) -> Option<&Texture> {
        self.textures.iter().find(|t| t.cbp == cbp)
    }
}

/// Where effect `number` of a PAX file lives: its DPD bytes and the index
/// of its particle data inside that DPD.
pub fn effect_source(pax: &[u8], number: u32) -> Result<Option<(&[u8], usize)>> {
    let dpx = Reader::at(pax, 12).u32()? as usize;
    let d = pax.get(dpx..).ok_or_else(|| oob("DPX"))?;
    let n = Reader::at(d, 12).i32()?;
    if !(0..=4096).contains(&n) {
        return bad("DPX: implausible effect count");
    }
    for i in 0..n as usize {
        let mut e = Reader::at(d, 16 + i * 32);
        let (off, index, num) = (e.u32()? as usize, e.u32()? as usize, e.u32()?);
        if num == number {
            return Ok(Some((d.get(off..).ok_or_else(|| oob("DPD"))?, index)));
        }
    }
    Ok(None)
}

/// One particle data: the 0x110-byte header plus the program file.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleData {
    /// Local transform applied under the effect root: translation,
    /// angles (65536 = full turn, applied X then Y then Z) and scale.
    pub offset: [f32; 3],
    pub angle: [i32; 3],
    pub scale: [f32; 3],
    /// Bit masks over the DPD's texture / shape / model / vertex-set lists
    /// (bit `i % 8` of byte `i / 8`): what this effect needs resident.
    pub use_texture: [u8; 16],
    pub use_shape: [u8; 16],
    pub use_model: [u8; 16],
    pub use_vertex_set: [u8; 16],
    pub work_size: i32,
    pub packet_size: i32,
    /// Skip updates while the effect is off screen.
    pub view_clip_stop: bool,
    /// Multiply the root by the owner's character scale.
    pub chr_scale: bool,
    pub bind_pos_only: bool,
    pub non_ambient: bool,
    pub se: u16,
    pub se_volume: u16,
    pub se_wait: f32,
    pub bone: i32,
    pub wait: f32,
    /// Ticks (x4096) the effect lasts unless it loops.
    pub end: i32,
    /// Runs until ended from outside, then until its objects are gone.
    pub looping: bool,
    /// The distinct program types used (for per-type setup hooks).
    pub kinds: Vec<u32>,
    pub emitters: Vec<EmitterDef>,
}

pub fn mask_has(mask: &[u8; 16], i: usize) -> bool {
    i < 128 && mask[i / 8] >> (i % 8) & 1 != 0
}

impl ParticleData {
    pub fn parse(buf: &[u8]) -> Result<ParticleData> {
        let mut r = Reader::at(buf, 0x80);
        let offset = [r.f32()?, r.f32()?, r.f32()?];
        r.skip(4)?;
        let angle = [r.i32()?, r.i32()?, r.i32()?];
        r.skip(4)?;
        let scale = [r.f32()?, r.f32()?, r.f32()?];
        r.skip(4)?;
        let mut m = || -> Result<[u8; 16]> { Ok(r.bytes(16)?.try_into().unwrap()) };
        let (use_texture, use_shape, use_model, use_vertex_set) = (m()?, m()?, m()?, m()?);
        let work_size = r.i32()?;
        let packet_size = r.i32()?;
        let (view_clip_stop, chr_scale, bind_pos_only, non_ambient) = (r.u8()? != 0, r.u8()? != 0, r.u8()? != 0, r.u8()? != 0);
        let se = r.u16()?;
        let se_volume = r.u16()?;
        let se_wait = r.f32()?;
        let bone = r.i32()?;
        let wait = r.f32()?;
        let file = buf.get(0x110..).ok_or_else(|| oob("program file"))?;
        let mut h = Reader::new(file);
        let end = h.i32()?;
        let looping = h.i32()? & 1 != 0;
        let kinds_at = h.u32()? as usize;
        let list_at = h.u32()? as usize;
        let mut k = Reader::at(file, kinds_at);
        let nk = k.i32()?;
        if !(0..=1024).contains(&nk) {
            return bad("DPD: implausible program type count");
        }
        let kinds = (0..nk).map(|_| k.u32()).collect::<Result<_>>()?;
        let mut l = Reader::at(file, list_at);
        let ne = l.i32()?;
        if !(0..=1024).contains(&ne) {
            return bad("DPD: implausible emitter count");
        }
        let mut emitters = Vec::with_capacity(ne as usize);
        for _ in 0..ne {
            emitters.push(EmitterDef::parse(file, l.u32()? as usize)?);
        }
        Ok(ParticleData {
            offset, angle, scale, use_texture, use_shape, use_model, use_vertex_set, work_size, packet_size,
            view_clip_stop, chr_scale, bind_pos_only, non_ambient, se, se_volume, se_wait, bone, wait, end, looping, kinds, emitters,
        })
    }
}

/// One emitter ("pObj" definition): a list of programs run on each of its
/// instances.
#[derive(Clone, Debug, PartialEq)]
pub struct EmitterDef {
    /// Effect frame (x4096) at which one instance is spawned;
    /// [`SPAWNED`] if only another program spawns it.
    pub start: i32,
    /// Instance lifetime (x4096); [`NEVER`] = until the effect ends.
    pub total: i32,
    /// When an instance reaches `loop_end` it restarts and fast-forwards to
    /// `loop_start` (both below [`NEVER`] to be active).
    pub loop_start: i32,
    pub loop_end: i32,
    /// Bytes of the instance block: 0xA0 header + program work. The
    /// per-program key cursors follow it.
    pub work_size: i32,
    /// Instances are kept sorted by this (lower first).
    pub priority: i16,
    pub programs: Vec<Program>,
}

impl EmitterDef {
    fn parse(file: &[u8], at: usize) -> Result<EmitterDef> {
        let mut r = Reader::at(file, at);
        let _next = r.u32()?;
        let cv = r.u32()? as usize;
        let _flag = r.u32()?;
        let _status = r.u32()?;
        let (start, total, loop_start, loop_end) = (r.i32()?, r.i32()?, r.i32()?, r.i32()?);
        let work_size = r.i32()?;
        let priority = r.i16()?;
        let n = r.i16()?;
        if !(0..=256).contains(&n) {
            return bad("DPD: implausible program count");
        }
        let mut heads = Vec::with_capacity(n as usize);
        for _ in 0..n {
            heads.push((r.u32()?, r.u16()?, r.u16()?, r.u32()? as usize, r.u32()? as usize));
        }
        let mut programs = Vec::with_capacity(heads.len());
        for (i, &(kind, size, count, keys_at, work_at)) in heads.iter().enumerate() {
            let mut kr = Reader::at(file, keys_at);
            let mut keys = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let data = kr.bytes(size as usize)?.to_vec();
                if data.len() < 4 {
                    return bad("DPD: key smaller than its frame");
                }
                keys.push(Key { data });
            }
            let work_end = heads.get(i + 1).map(|h| h.3).unwrap_or(cv);
            let words = work_end.saturating_sub(work_at) / 4;
            let mut wr = Reader::at(file, work_at);
            let work = (0..words.min(64)).map(|_| wr.i32()).collect::<Result<_>>()?;
            programs.push(Program { kind, key_size: size, keys, work });
        }
        Ok(EmitterDef { start, total, loop_start, loop_end, work_size, priority, programs })
    }

    /// Tick at which the effect spawns this emitter itself, if it does.
    pub fn start_tick(&self) -> Option<i32> {
        (self.start < NEVER).then_some(self.start / TICK)
    }

    pub fn total_ticks(&self) -> Option<i32> {
        (self.total < NEVER).then_some(self.total / TICK)
    }

    pub fn looping(&self) -> Option<(i32, i32)> {
        (self.loop_start < NEVER && self.loop_end < NEVER).then_some((self.loop_start / TICK, self.loop_end / TICK))
    }
}

/// One program call on an emitter.
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    /// Index into the game's program table, see [`program_name`].
    pub kind: u32,
    pub key_size: u16,
    /// Sorted by frame. Each starts with its frame (x4096).
    pub keys: Vec<Key>,
    /// Byte offsets into the instance's work area (which starts at +0xA0 of
    /// the instance) of the values this program reads and writes, in the
    /// order the program defines.
    pub work: Vec<i32>,
}

impl Program {
    pub fn name(&self) -> Option<&'static str> {
        program_name(self.kind)
    }
}

/// A key: the frame word followed by the program's parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    pub data: Vec<u8>,
}

impl Key {
    pub fn frame(&self) -> i32 {
        i32::from_le_bytes(self.data[0..4].try_into().unwrap())
    }
    pub fn tick(&self) -> i32 {
        self.frame() / TICK
    }
    pub fn u8(&self, at: usize) -> u8 {
        self.data.get(at).copied().unwrap_or(0)
    }
    pub fn i8(&self, at: usize) -> i8 {
        self.u8(at) as i8
    }
    pub fn u16(&self, at: usize) -> u16 {
        u16::from_le_bytes([self.u8(at), self.u8(at + 1)])
    }
    pub fn i16(&self, at: usize) -> i16 {
        self.u16(at) as i16
    }
    pub fn i32(&self, at: usize) -> i32 {
        i32::from_le_bytes([self.u8(at), self.u8(at + 1), self.u8(at + 2), self.u8(at + 3)])
    }
    pub fn f32(&self, at: usize) -> f32 {
        f32::from_bits(self.i32(at) as u32)
    }
    pub fn vec3(&self, at: usize) -> [f32; 3] {
        [self.f32(at), self.f32(at + 4), self.f32(at + 8)]
    }
}

/// An indexed PS2 texture with its palette.
#[derive(Clone, Debug, PartialEq)]
pub struct Texture {
    /// GS block address of the pixels and of the palette, as the shapes and
    /// models in the same package reference them.
    pub tbp: u16,
    pub cbp: u16,
    pub buffer_width: i16,
    /// GS pixel format: 0x13 = 8-bit indexed, 0x14 = 4-bit indexed.
    pub psm: i16,
    /// Upload position inside the buffer that starts at `tbp` (GS
    /// DSAX/DSAY): small textures share a page side by side.
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    /// One index per pixel (4-bit textures are unpacked), rows top-down.
    pub indices: Vec<u8>,
    /// RGBA, alpha 0x80 = opaque. Stored in GS CLUT order (see [`Texture::color`]).
    pub palette: Vec<[u8; 4]>,
}

impl Texture {
    pub fn parse(buf: &[u8]) -> Result<Texture> {
        let mut r = Reader::new(buf);
        let tbp = r.u16()?;
        let cbp = r.u16()?;
        let buffer_width = r.i16()?;
        let psm = r.i16()?;
        let (x, y) = (r.i16()?, r.i16()?);
        let (width, height) = (r.u16()?, r.u16()?);
        r.skip(16)?;
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return bad("DPD: implausible texture size");
        }
        let n = width as usize * height as usize;
        let (indices, colors) = if psm == 0x14 {
            let packed = r.bytes(n.div_ceil(2))?;
            (packed.iter().flat_map(|b| [b & 15, b >> 4]).take(n).collect(), 16)
        } else {
            (r.bytes(n)?.to_vec(), 256)
        };
        let palette = (0..colors).map(|_| Ok(r.bytes(4)?.try_into().unwrap())).collect::<Result<_>>()?;
        Ok(Texture { tbp, cbp, buffer_width, psm, x, y, width, height, indices, palette })
    }

    /// The palette entry for pixel index `i`. 256-colour palettes are stored
    /// in the GS "CSM1" order, which swaps bits 3 and 4 of the index.
    pub fn color(&self, i: u8) -> [u8; 4] {
        let k = if self.palette.len() == 256 { (i & 0xe7) | (i & 0x08) << 1 | (i & 0x10) >> 1 } else { i & 15 };
        self.palette[k as usize]
    }

    /// Decoded pixels, RGBA8 with alpha scaled so 0x80 becomes 0xFF.
    pub fn rgba(&self) -> Vec<u8> {
        self.indices
            .iter()
            .flat_map(|&i| {
                let [r, g, b, a] = self.color(i);
                [r, g, b, (a as u16 * 2).min(255) as u8]
            })
            .collect()
    }
}

/// The GS TEX0 register stored with shape frames and model primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tex0(pub u64);

impl Tex0 {
    /// Texture base, in 256-byte blocks. Values below 0x3000 address the
    /// shared `effect/texcommon.dpd`; 0x3000 and up the effect's own DPD.
    pub fn tbp(self) -> u16 {
        (self.0 & 0x3fff) as u16
    }
    /// Buffer width in 64-pixel units.
    pub fn tbw(self) -> u8 {
        (self.0 >> 14 & 0x3f) as u8
    }
    pub fn psm(self) -> u8 {
        (self.0 >> 20 & 0x3f) as u8
    }
    /// Texture size, log2.
    pub fn tw(self) -> u8 {
        (self.0 >> 26 & 15) as u8
    }
    pub fn th(self) -> u8 {
        (self.0 >> 30 & 15) as u8
    }
    /// Texture alpha used (1) or ignored (0).
    pub fn tcc(self) -> bool {
        self.0 >> 34 & 1 != 0
    }
    /// 0 = modulate, 1 = decal, 2 = highlight, 3 = highlight2.
    pub fn tfx(self) -> u8 {
        (self.0 >> 35 & 3) as u8
    }
    /// Palette base, in 256-byte blocks.
    pub fn cbp(self) -> u16 {
        (self.0 >> 37 & 0x3fff) as u16
    }
    pub fn csa(self) -> u8 {
        (self.0 >> 56 & 31) as u8
    }
}

/// A sprite shape: an animation of sprite frames.
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    /// The number programs use to name this shape.
    pub number: i16,
    pub sequence: Vec<ShapeStep>,
}

/// One step of a shape animation.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeStep {
    /// How long the step shows, in animation units (the drawing program
    /// adds its speed, usually 512, each tick).
    pub wait: u16,
    /// Set on the last step: the animation restarts instead of holding.
    pub loops: bool,
    pub frame: ShapeFrame,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeFrame {
    /// Quads with four free corners (true) or axis-aligned rectangles.
    pub free_quads: bool,
    pub tex0: Tex0,
    pub sprites: Vec<Sprite>,
}

/// One textured quad of a shape frame. Positions are 12.4 fixed point in
/// the shape's local plane; texture coordinates are normalised, 4096 = the
/// full texture size given by the frame's TEX0 (`1 << tw`, `1 << th`).
#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    /// Top-left, top-right, bottom-left, bottom-right.
    pub corners: [[i16; 2]; 4],
    pub s: i16,
    pub t: i16,
    pub w: i16,
    pub h: i16,
    /// Vertex colour, 0x80 = 1.0.
    pub rgba: [u8; 4],
    /// Low byte of the GS ALPHA register: the blend equation
    /// (`(A - B) * C + D`, two bits each, A first).
    pub blend: u8,
    /// The ALPHA register's fixed alpha.
    pub blend_fix: u8,
    pub code: u8,
    /// Added to the TEX0 palette base when drawn.
    pub clut_offset: u8,
}

impl Sprite {
    /// Corner positions in shape units.
    pub fn positions(&self) -> [[f32; 2]; 4] {
        self.corners.map(|[x, y]| [x as f32 / 16.0, y as f32 / 16.0])
    }
    /// Texture rectangle `(u0, v0, u1, v1)` in 0..1 of the TEX0 texture
    /// size; the top-left corner takes `(u0, v0)` and the bottom-right
    /// `(u1, v1)`.
    pub fn uv(&self) -> [f32; 4] {
        let f = |v: i16| v as f32 / 4096.0;
        [f(self.s), f(self.t), f(self.s) + f(self.w), f(self.t) + f(self.h)]
    }
}

impl Shape {
    pub fn parse(buf: &[u8]) -> Result<Shape> {
        let number = Reader::new(buf).i16()?;
        let anm = buf.get(16..).ok_or_else(|| oob("shape"))?;
        let steps = Reader::at(anm, 6).i16()?;
        if !(0..=1024).contains(&steps) {
            return bad("DPD: implausible shape step count");
        }
        let mut sequence = Vec::with_capacity(steps as usize);
        for i in 0..steps as usize {
            let mut s = Reader::at(anm, 16 + i * 8);
            let at = s.i16()? as usize;
            let wait = s.u16()?;
            let loops = s.u8()? & 0x80 != 0;
            let mut f = Reader::at(anm, at);
            let flag = f.i16()?;
            let n = f.i16()?;
            let attr_at = at + f.i32()? as usize;
            let tex0 = Tex0(Reader::at(anm, at + 16).i64()? as u64);
            let free_quads = flag & 8 != 0;
            if !(0..=1024).contains(&n) {
                return bad("DPD: implausible sprite count");
            }
            let mut sr = Reader::at(anm, at + 32);
            let mut ar = Reader::at(anm, attr_at);
            let mut sprites = Vec::with_capacity(n as usize);
            for _ in 0..n {
                let corners = if free_quads {
                    [[sr.i16()?, sr.i16()?], [sr.i16()?, sr.i16()?], [sr.i16()?, sr.i16()?], [sr.i16()?, sr.i16()?]]
                } else {
                    let (x0, y0, x3, y3) = (sr.i16()?, sr.i16()?, sr.i16()?, sr.i16()?);
                    [[x0, y0], [x3, y0], [x0, y3], [x3, y3]]
                };
                let (s, t, w, h) = (sr.i16()?, sr.i16()?, sr.i16()?, sr.i16()?);
                let a = ar.bytes(8)?;
                sprites.push(Sprite {
                    corners, s, t, w, h,
                    rgba: [a[0], a[1], a[2], a[3]],
                    blend: a[4], blend_fix: a[5], code: a[6], clut_offset: a[7],
                });
            }
            sequence.push(ShapeStep { wait, loops, frame: ShapeFrame { free_quads, tex0, sprites } });
        }
        Ok(Shape { number, sequence })
    }
}

/// A small effect model (an "OMD" mesh).
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    /// The number programs use to name this model.
    pub number: i16,
    /// Bit 0 clear: the model writes depth. Bit 1 clear: drawn with the
    /// alternate cull mode (see the docs).
    pub flags: i32,
    /// Low byte of the GS ALPHA register used unless the program overrides it.
    pub blend: u8,
    pub blend_fix: u8,
    pub groups: Vec<PrimGroup>,
    /// Positions in model units (no fixed-point scale).
    pub vertices: Vec<[i16; 3]>,
    pub normals: Vec<[i16; 3]>,
}

/// A run of primitives sharing one kind and texture.
#[derive(Clone, Debug, PartialEq)]
pub struct PrimGroup {
    /// 0 G3, 1 G3L, 2 GT3, 3 GT3L (triangles); 4 G4, 5 G4L, 6 GT4, 7 GT4L
    /// (quads, drawn as a 4-vertex strip). T = textured, L = has normals.
    pub kind: u8,
    pub flag: u8,
    pub tex0: Tex0,
    pub prims: Vec<Prim>,
}

impl PrimGroup {
    pub fn is_quad(&self) -> bool {
        self.kind >= 4
    }
    pub fn textured(&self) -> bool {
        matches!(self.kind, 2 | 3 | 6 | 7)
    }
    fn size(kind: u8) -> Option<usize> {
        [0x14, 0x1c, 0x20, 0x28, 0x18, 0x20, 0x28, 0x30].get(kind as usize).copied()
    }
}

/// One triangle or quad. Texture coordinates are normalised, 4096 = the
/// full texture size given by the group's TEX0.
#[derive(Clone, Debug, PartialEq)]
pub struct Prim {
    pub vertices: Vec<u16>,
    /// Per-vertex colour, 0x80 = 1.0.
    pub rgba: Vec<[u8; 4]>,
    pub st: Vec<[u16; 2]>,
    pub normals: Vec<u16>,
}

impl Model {
    pub fn parse(buf: &[u8]) -> Result<Model> {
        let number = Reader::new(buf).i16()?;
        let omd = buf.get(16..).ok_or_else(|| oob("model"))?;
        let mut r = Reader::new(omd);
        let flags = r.i32()?;
        let (prims_at, verts_at, norms_at) = (r.u32()? as usize, r.u32()? as usize, r.u32()? as usize);
        let _pcount = r.i16()?;
        let vcount = r.i16()?;
        let ncount = r.i16()?;
        let blend = r.u8()?;
        let blend_fix = r.u8()?;
        let mut groups = Vec::new();
        let mut at = prims_at;
        loop {
            let mut g = Reader::at(omd, at);
            let flag = g.u8()?;
            let kind = g.u8()?;
            if kind == 0xff {
                break;
            }
            let size = PrimGroup::size(kind).ok_or_else(|| Error::Format(format!("DPD: unknown primitive kind {kind}")))?;
            let count = g.i16()?;
            if !(0..=8192).contains(&count) || groups.len() > 256 {
                return bad("DPD: implausible primitive count");
            }
            let tex0 = Tex0(Reader::at(omd, at + 8).i64()? as u64);
            let mut prims = Vec::with_capacity(count as usize);
            for i in 0..count as usize {
                let mut p = Reader::at(omd, at + 16 + i * size);
                let n = if kind >= 4 { 4 } else { 3 };
                let rgba = (0..n).map(|_| Ok(p.bytes(4)?.try_into().unwrap())).collect::<Result<Vec<[u8; 4]>>>()?;
                let vertices = (0..n).map(|_| p.u16()).collect::<Result<Vec<_>>>()?;
                if n == 3 {
                    p.skip(2)?;
                }
                let st = if matches!(kind, 2 | 3 | 6 | 7) { (0..n).map(|_| Ok([p.u16()?, p.u16()?])).collect::<Result<_>>()? } else { Vec::new() };
                let normals = if kind & 1 == 1 { (0..n).map(|_| p.u16()).collect::<Result<_>>()? } else { Vec::new() };
                prims.push(Prim { vertices, rgba, st, normals });
            }
            groups.push(PrimGroup { kind, flag, tex0, prims });
            at += 16 + count as usize * size;
        }
        let read3 = |at: usize, n: i16| -> Result<Vec<[i16; 3]>> {
            let mut v = Reader::at(omd, at);
            (0..n.max(0)).map(|_| i16x3(&mut v)).collect()
        };
        let vertices = read3(verts_at, vcount)?;
        let normals = if norms_at != 0 { read3(norms_at, ncount).unwrap_or_default() } else { Vec::new() };
        Ok(Model { number, flags, blend, blend_fix, groups, vertices, normals })
    }
}

/// A list of model vertices that programs spawn things at.
#[derive(Clone, Debug, PartialEq)]
pub struct VertexSet {
    pub number: i32,
    pub model: i32,
    pub indices: Vec<u16>,
}

impl VertexSet {
    pub fn parse(buf: &[u8]) -> Result<VertexSet> {
        let mut r = Reader::new(buf);
        let number = r.i32()?;
        let model = r.i32()?;
        let n = r.i32()?;
        if !(0..=65536).contains(&n) {
            return bad("DPD: implausible vertex set size");
        }
        let indices = (0..n).map(|_| r.u16()).collect::<Result<_>>()?;
        Ok(VertexSet { number, model, indices })
    }
}

/// Name of program type `kind` (the game's own names; OpenKH lists the
/// same table in `DpdPData.cs`).
pub fn program_name(kind: u32) -> Option<&'static str> {
    kind.checked_sub(1).and_then(|i| PROGRAM_NAMES.get(i as usize)).copied()
}

/// Program types 1..=242.
pub const PROGRAM_NAMES: [&str; 242] = [
    "pppDummyFunc",
    "pppKeThRes",
    "pppKeThRes8",
    "pppKeThRes8x4",
    "pppKeThRes8x128",
    "pppKeThRes16",
    "pppKeThRes16x4",
    "pppKeThRes16x64",
    "pppKeThRes24",
    "pppKeThRes24x4",
    "pppKeThRes32",
    "pppKeThRes32x4",
    "pppKeThRes32x32",
    "pppKeThRes40",
    "pppKeThRes40x4",
    "pppKeThRes48",
    "pppKeThRes48x4",
    "pppKeThRes64",
    "pppKeThRes64x4",
    "pppKeThRes64x16",
    "pppKeThRes128",
    "pppKeThRes128x4",
    "pppKeThRes128x8",
    "pppKeThRes255",
    "pppKeThRes255x4",
    "pppKeHitBall",
    "pppKeGrvTgt",
    "pppAccele",
    "pppAngAccele",
    "pppSclAccele",
    "pppColAccele",
    "pppEiWfacc",
    "pppKeHmgEff",
    "pppKeGrvEff",
    "pppKeHitChkPxB",
    "pppMove",
    "pppAngMove",
    "pppSclMove",
    "pppColMove",
    "pppPoint",
    "pppAngle",
    "pppScale",
    "pppColor",
    "pppPObjPoint",
    "pppVertexAttend",
    "pppEiWindFun",
    "pppKeMvYpEff",
    "pppKeDrct",
    "pppVtMime",
    "pppKeTkFade",
    "pppRandFloat",
    "pppRandUpFloat",
    "pppRandDownFloat",
    "pppRandChar",
    "pppRandUpChar",
    "pppRandDownChar",
    "pppRandShort",
    "pppRandUpShort",
    "pppRandDownShort",
    "pppRandInt",
    "pppRandUpInt",
    "pppRandDownInt",
    "pppRandFV",
    "pppRandUpFV",
    "pppRandDownFV",
    "pppRandIV",
    "pppRandUpIV",
    "pppRandDownIV",
    "pppRandCV",
    "pppRandUpCV",
    "pppRandDownCV",
    "pppRandHCV",
    "pppRandUpHCV",
    "pppRandDownHCV",
    "pppSRandFV",
    "pppSRandUpFV",
    "pppSRandDownFV",
    "pppSRandCV",
    "pppSRandUpCV",
    "pppSRandDownCV",
    "pppSRandHCV",
    "pppSRandUpHCV",
    "pppSRandDownHCV",
    "pppSMatrix",
    "pppKeMatSN",
    "pppMatrix",
    "pppMatrixXYZ",
    "pppMatrixXZY",
    "pppMatrixYZX",
    "pppMatrixYXZ",
    "pppMatrixZXY",
    "pppMatrixZYX",
    "pppMatrixLoc",
    "pppMatrixScl",
    "pppMatrixFront",
    "pppRyjMatrixNoRot",
    "pppKeMatPht",
    "pppRyjMatrixWorld",
    "pppRyjMatrixWorldXYZ",
    "pppRyjMatrixWorldXZY",
    "pppRyjMatrixWorldYXZ",
    "pppRyjMatrixWorldYZX",
    "pppRyjMatrixWorldZXY",
    "pppRyjMatrixWorldZYX",
    "pppParMatrix",
    "pppKeParMatR",
    "pppChrSclXYZMatrix",
    "pppChrSclXZMatrix",
    "pppChrSclYMatrix",
    "pppChrYSclXYZMatrix",
    "pppChrXSclXYZMatrix",
    "pppDrawMatrix",
    "pppDrawMatrixFront",
    "pppDrawMatrixNoRot",
    "pppDrawMatrixWood",
    "pppKeDMat",
    "pppKeDMatFr",
    "pppKeDMatPht",
    "pppKeDMatPhtFr",
    "pppRyjDrawMatrixWorld",
    "pppRyjDrawMatrixWorldFront",
    "pppRyjDrawMatrixWorldNoRot",
    "pppRyjDrawMatrixWorldWood",
    "pppKeZCrct",
    "pppKeZCrctShp",
    "pppKeThTp",
    "pppKeThTp2",
    "pppKeThSft",
    "pppKeTh",
    "pppDrawMdlBS",
    "pppDrawMdl",
    "pppDrawMdlSemi",
    "pppDrawMdlTs",
    "pppDrawMdl2",
    "pppDrawMdlSemi2",
    "pppDrawMdlTs2",
    "pppDrawMdl3",
    "pppDrawMdlSemi3",
    "pppDrawMdlTs3",
    "pppDrawMdlSea",
    "pppDrawShape",
    "pppDrawShapeX",
    "pppDrawShapeField",
    "pppKeMdlBmp",
    "pppKeMdlDtt",
    "pppKeMdlTfd",
    "pppKeMdlTfdUv",
    "pppKeMdlTfd2",
    "pppKeMdlTfdUv2",
    "pppKeMdlTfd3",
    "pppKeMdlTfdUv3",
    "pppKeShpTail",
    "pppKeShpTailX",
    "pppKeShpTail2",
    "pppKeShpTail2X",
    "pppKeShpTail3",
    "pppKeShpTail3X",
    "pppKeShpTailPht",
    "pppKeShpTailLc",
    "pppKeShpDtt",
    "pppRyjDrawShipoly",
    "pppDrawHook",
    "pppSDMatrix",
    "pppSCMatrix",
    "pppWMatrix",
    "pppWMatrixXYZ",
    "pppWMatrixXZY",
    "pppWMatrixYZX",
    "pppWMatrixYXZ",
    "pppWMatrixZXY",
    "pppWMatrixZYX",
    "pppPointAp",
    "pppPointRAp",
    "pppVertexAp",
    "pppSegmentAp",
    "pppFaceAp",
    "pppVertexApLc",
    "pppVertexApAt",
    "pppKeBornRnd",
    "pppKeBornRnd2",
    "pppKeBornRnd3",
    "pppKeBornRnd4",
    "pppKeBornRnd5",
    "pppKeBornRnd6",
    "pppKeBornPtCmpl",
    "pppKeHitBorn",
    "pppKeThHitBorn",
    "pppKeLnsLpSft",
    "pppKeLnsLp",
    "pppKeLnsArnd",
    "pppKeLnsClm",
    "pppKeLnsCrn",
    "pppKeLnsFls",
    "pppKeAcmSolid",
    "pppKeHitChk",
    "pppKeThLz",
    "pppKeThCp",
    "pppKeThCpSft",
    "pppKeAccSpdSv",
    "pppRyjDrawShipolyBone",
    "pppRyjMegaBirth",
    "pppRyjMngFlag",
    "pppRyjMegaBirthPrize",
    "pppRyjMegaPlace",
    "pppRyjMegaPlaceShape",
    "pppRyjMegaPlaceModel",
    "pppRyjMegaPlaceLamp",
    "pppRyjMegaBirthModel",
    "pppRyjDrawKekoto",
    "pppMoveLoop",
    "pppAngMoveLoop",
    "pppSclMoveLoop",
    "pppPointLoop",
    "pppAngleLoop",
    "pppScaleLoop",
    "pppMatrixLoop",
    "pppDrawMatrixLoop",
    "pppDrawMdlLoop",
    "pppDrawMdlLoopZ",
    "pppRyjLight",
    "pppKeLnsLpT",
    "pppKeLnsArndT",
    "pppKeLnsClmT",
    "pppKeLnsCrnT",
    "pppKeLnsFlsT",
    "pppRyjDrawMatrixWorldDtt",
    "pppRyjDrawMatrixWorldDttFr",
    "pppRyjMegaBirthFilter",
    "pppRyjMegaBirthModelFilter",
    "pppMatrixLoopXYZ",
    "pppMatrixLoopXZY",
    "pppMatrixLoopYXZ",
    "pppMatrixLoopYZX",
    "pppMatrixLoopZXY",
    "pppMatrixLoopZYX",
    "pppVertexApLcLoop",
    "pppSnoScaleAll",
    "pppSnoMdlDttNorm",
    "pppRyjMegaBirthUserCtrl",
    "pppRyjMegaBirthUserCtrl2",
    "pppEiDrawShipoly",
    "pppRyjMegaBirthUserCtrl3",
];

/// Program type numbers documented in `docs/kh2-effects.md`.
pub mod kind {
    pub const ANG_ACCELE: u32 = 29;
    pub const SCL_ACCELE: u32 = 30;
    pub const COL_ACCELE: u32 = 31;
    pub const ANG_MOVE: u32 = 37;
    pub const SCL_MOVE: u32 = 38;
    pub const COL_MOVE: u32 = 39;
    pub const POINT: u32 = 40;
    pub const ANGLE: u32 = 41;
    pub const SCALE: u32 = 42;
    pub const COLOR: u32 = 43;
    pub const RAND_FLOAT: u32 = 51;
    pub const RAND_IV: u32 = 66;
    pub const S_MATRIX: u32 = 84;
    pub const MATRIX_XYZ: u32 = 87;
    pub const MATRIX_YXZ: u32 = 90;
    pub const MATRIX_ZXY: u32 = 91;
    pub const MATRIX_SCL: u32 = 94;
    pub const PAR_MATRIX: u32 = 105;
    pub const DRAW_MATRIX: u32 = 112;
    pub const DRAW_MATRIX_FRONT: u32 = 113;
    pub const KE_Z_CRCT: u32 = 124;
    pub const KE_Z_CRCT_SHP: u32 = 125;
    pub const DRAW_MDL3: u32 = 137;
    pub const DRAW_MDL_TS3: u32 = 139;
    pub const DRAW_SHAPE: u32 = 141;
    pub const DRAW_SHAPE_X: u32 = 142;
    pub const VERTEX_AP_LC: u32 = 177;
    pub const RYJ_MEGA_BIRTH: u32 = 201;
    pub const DRAW_MATRIX_LOOP: u32 = 217;
    pub const DRAW_MDL_LOOP_Z: u32 = 219;
    pub const EI_DRAW_SHIPOLY: u32 = 241;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(b: &mut Vec<u8>, at: usize, bytes: &[u8]) {
        if b.len() < at + bytes.len() {
            b.resize(at + bytes.len(), 0);
        }
        b[at..at + bytes.len()].copy_from_slice(bytes);
    }
    fn w32(b: &mut Vec<u8>, at: usize, v: i32) {
        put(b, at, &v.to_le_bytes());
    }
    fn w16(b: &mut Vec<u8>, at: usize, v: i16) {
        put(b, at, &v.to_le_bytes());
    }
    fn wf(b: &mut Vec<u8>, at: usize, v: f32) {
        put(b, at, &v.to_le_bytes());
    }

    fn particle_data() -> Vec<u8> {
        let mut b = vec![0u8; 0x110];
        wf(&mut b, 0x80, 1.0);
        wf(&mut b, 0x84, 2.0);
        wf(&mut b, 0x88, 3.0);
        w32(&mut b, 0x98, 0x4000);
        wf(&mut b, 0xa0, 1.0);
        wf(&mut b, 0xa4, 1.0);
        wf(&mut b, 0xa8, 1.0);
        b[0xc0] = 0b10; // uses shape 1
        b[0xf9] = 1;
        let f = 0x110;
        w32(&mut b, f, 30 * TICK);
        w32(&mut b, f + 4, 0);
        w32(&mut b, f + 8, 0xb4);
        w32(&mut b, f + 12, 0xbc);
        // emitter at 0x10
        let e = f + 0x10;
        w32(&mut b, e + 4, 0x70);
        w32(&mut b, e + 16, 17 * TICK);
        w32(&mut b, e + 20, 6 * TICK);
        w32(&mut b, e + 24, NEVER);
        w32(&mut b, e + 28, NEVER);
        w32(&mut b, e + 32, 0xa0 + 16);
        w16(&mut b, e + 36, 2);
        w16(&mut b, e + 38, 1);
        w32(&mut b, e + 40, kind::SCALE as i32);
        w16(&mut b, e + 44, 16);
        w16(&mut b, e + 46, 2);
        w32(&mut b, e + 48, 0x48);
        w32(&mut b, e + 52, 0x6c);
        // keys at 0x48, terminator, work offsets
        let k = f + 0x48;
        w32(&mut b, k, 0);
        wf(&mut b, k + 4, 0.5);
        wf(&mut b, k + 8, 0.25);
        wf(&mut b, k + 12, 2.0);
        w32(&mut b, k + 16, 5 * TICK);
        wf(&mut b, k + 20, -0.5);
        put(&mut b, f + 0x68, &KEY_END.to_le_bytes());
        w32(&mut b, f + 0x6c, 80);
        // cv offsets 0x70..0xb0, end marker, kinds, emitter list
        w32(&mut b, f + 0xb4, 1);
        w32(&mut b, f + 0xb8, kind::SCALE as i32);
        w32(&mut b, f + 0xbc, 1);
        w32(&mut b, f + 0xc0, 0x10);
        b
    }

    fn texture() -> Vec<u8> {
        let mut b = vec![0u8; 0x20 + 16 + 1024];
        w16(&mut b, 0, 0x3000);
        w16(&mut b, 2, 0x3e00);
        w16(&mut b, 4, 2);
        w16(&mut b, 6, 0x13);
        w16(&mut b, 12, 16);
        w16(&mut b, 14, 1);
        b[0x20] = 8;
        b[0x21] = 16;
        b[0x22] = 3;
        put(&mut b, 0x30 + 16 * 4, &[1, 2, 3, 0x80]);
        put(&mut b, 0x30 + 8 * 4, &[9, 9, 9, 0x40]);
        put(&mut b, 0x30 + 3 * 4, &[7, 7, 7, 0x20]);
        b
    }

    fn shape() -> Vec<u8> {
        let mut b = Vec::new();
        w16(&mut b, 0, 5);
        let a = 16;
        w16(&mut b, a + 4, 1);
        w16(&mut b, a + 6, 1);
        w16(&mut b, a + 16, 0x20);
        w16(&mut b, a + 18, 512);
        b.resize(a + 21, 0);
        b[a + 20] = 0x80;
        let fr = a + 0x20;
        w16(&mut b, fr, 8);
        w16(&mut b, fr + 2, 1);
        w32(&mut b, fr + 4, 0x38);
        let tex0: u64 = 0x2a00 | 4 << 14 | 0x13 << 20 | 8 << 26 | 9 << 30 | 1 << 34 | 0x2e34 << 37;
        put(&mut b, fr + 16, &tex0.to_le_bytes());
        let c: [i16; 12] = [-160, -32, 160, -32, -160, 48, 160, 48, 528, 16, 480, 232];
        for (i, v) in c.iter().enumerate() {
            w16(&mut b, fr + 0x20 + i * 2, *v);
        }
        put(&mut b, fr + 0x38, &[0x0c, 0x18, 0x2f, 0x80, 0x48, 0, 0x43, 0]);
        b
    }

    fn model() -> Vec<u8> {
        let mut b = Vec::new();
        w16(&mut b, 0, 17);
        let o = 16;
        w32(&mut b, o, 3);
        w32(&mut b, o + 4, 0x20);
        w32(&mut b, o + 8, 0x20 + 16 + 0x28 + 16);
        w32(&mut b, o + 12, 0);
        w16(&mut b, o + 16, 1);
        w16(&mut b, o + 18, 4);
        b.resize(o + 0x20, 0);
        b[o + 22] = 0x48;
        b[o + 23] = 0;
        let g = o + 0x20;
        b.resize(g + 2, 0);
        b[g + 1] = 6;
        w16(&mut b, g + 2, 1);
        put(&mut b, g + 8, &(0x2c80u64 | 7 << 26 | 7 << 30).to_le_bytes());
        let p = g + 16;
        put(&mut b, p, &[0x80; 4]);
        put(&mut b, p + 4, &[0, 0, 0, 0x80]);
        for (i, v) in [4u16, 3, 1, 0].iter().enumerate() {
            put(&mut b, p + 16 + i * 2, &v.to_le_bytes());
        }
        for (i, v) in [4096u16, 4096, 0, 4096, 4096, 0, 0, 0].iter().enumerate() {
            put(&mut b, p + 24 + i * 2, &v.to_le_bytes());
        }
        let end = p + 0x28;
        b.resize(end + 16, 0);
        b[end + 1] = 0xff;
        let v = end + 16;
        for (i, x) in [412i16, 412, 0, 0, 412, 0, -412, 412, 0, 412, 0, 0].iter().enumerate() {
            w16(&mut b, v + i * 2, *x);
        }
        b
    }

    fn dpd() -> Vec<u8> {
        let parts = [particle_data(), texture(), shape(), model(), {
            let mut v = Vec::new();
            w32(&mut v, 0, 3);
            w32(&mut v, 4, 17);
            w32(&mut v, 8, 2);
            w16(&mut v, 12, 1);
            w16(&mut v, 14, 2);
            v
        }];
        let mut b = Vec::new();
        w32(&mut b, 0, MAGIC as i32);
        let head = 4 + 5 * 8 + 16;
        let mut at = (head + 15) & !15;
        for (i, p) in parts.iter().enumerate() {
            w32(&mut b, 4 + i * 8, 1);
            w32(&mut b, 8 + i * 8, at as i32);
            put(&mut b, at, p);
            at = (at + p.len() + 15) & !15;
        }
        b
    }

    #[test]
    fn parses_programs() {
        let d = Dpd::parse(&dpd()).unwrap();
        let p = &d.particles[0];
        assert_eq!(p.offset, [1.0, 2.0, 3.0]);
        assert_eq!(p.angle[2], 0x4000);
        assert!(p.chr_scale && !p.looping);
        assert!(mask_has(&p.use_shape, 1) && !mask_has(&p.use_shape, 0));
        assert_eq!(p.end, 30 * TICK);
        assert_eq!(p.kinds, vec![kind::SCALE]);
        let e = &p.emitters[0];
        assert_eq!((e.start_tick(), e.total_ticks(), e.looping()), (Some(17), Some(6), None));
        assert_eq!(e.priority, 2);
        let prog = &e.programs[0];
        assert_eq!(prog.name(), Some("pppScale"));
        assert_eq!(prog.work, vec![80]);
        assert_eq!(prog.keys.len(), 2);
        assert_eq!(prog.keys[0].vec3(4), [0.5, 0.25, 2.0]);
        assert_eq!(prog.keys[1].tick(), 5);
        assert_eq!(prog.keys[1].f32(4), -0.5);
    }

    #[test]
    fn decodes_texture() {
        let d = Dpd::parse(&dpd()).unwrap();
        let t = d.texture_at(0x3000).unwrap();
        assert!(d.palette_at(0x3e00).is_some());
        let px = t.rgba();
        assert_eq!(px.len(), 16 * 4);
        // Index 8 reads palette entry 16 and vice versa (CSM1 order).
        assert_eq!(&px[0..4], &[1, 2, 3, 255]);
        assert_eq!(&px[4..8], &[9, 9, 9, 128]);
        assert_eq!(&px[8..12], &[7, 7, 7, 64]);
    }

    #[test]
    fn parses_shape_and_model() {
        let d = Dpd::parse(&dpd()).unwrap();
        let s = d.shape(5).unwrap();
        let step = &s.sequence[0];
        assert_eq!((step.wait, step.loops), (512, true));
        let f = &step.frame;
        assert!(f.free_quads);
        assert_eq!((f.tex0.tbp(), f.tex0.tbw(), f.tex0.psm(), f.tex0.tw(), f.tex0.cbp()), (0x2a00, 4, 0x13, 8, 0x2e34));
        assert!(f.tex0.tcc());
        let sp = &f.sprites[0];
        assert_eq!(sp.positions()[0], [-10.0, -2.0]);
        assert_eq!(sp.positions()[3], [10.0, 3.0]);
        assert_eq!(sp.uv(), [528.0 / 4096.0, 16.0 / 4096.0, 1008.0 / 4096.0, 248.0 / 4096.0]);
        assert_eq!((sp.rgba, sp.blend), ([0x0c, 0x18, 0x2f, 0x80], 0x48));

        let m = d.model(17).unwrap();
        assert_eq!((m.flags, m.blend), (3, 0x48));
        let g = &m.groups[0];
        assert!(g.is_quad() && g.textured());
        assert_eq!(g.tex0.tbp(), 0x2c80);
        assert_eq!(g.prims[0].vertices, vec![4, 3, 1, 0]);
        assert_eq!(g.prims[0].st[1], [0, 4096]);
        assert_eq!(g.prims[0].rgba[1], [0, 0, 0, 0x80]);
        assert_eq!(m.vertices.len(), 4);
        assert_eq!(m.vertices[2], [-412, 412, 0]);

        let v = d.vertex_set(3).unwrap();
        assert_eq!((v.model, v.indices.clone()), (17, vec![1, 2]));
    }

    #[test]
    fn finds_effect_source() {
        let mut pax = Vec::new();
        put(&mut pax, 0, b"PAX_");
        w32(&mut pax, 12, 16);
        let dpx = 16;
        w32(&mut pax, dpx, 130);
        w32(&mut pax, dpx + 12, 1);
        w32(&mut pax, dpx + 16, 0x40);
        w32(&mut pax, dpx + 20, 3);
        w32(&mut pax, dpx + 24, 7);
        w32(&mut pax, dpx + 0x40, MAGIC as i32);
        let (d, i) = effect_source(&pax, 7).unwrap().unwrap();
        assert_eq!((d[0], i), (0x96, 3));
        assert!(effect_source(&pax, 8).unwrap().is_none());
    }

    #[test]
    fn names() {
        assert_eq!(program_name(1), Some("pppDummyFunc"));
        assert_eq!(program_name(kind::RYJ_MEGA_BIRTH), Some("pppRyjMegaBirth"));
        assert_eq!(program_name(242), Some("pppRyjMegaBirthUserCtrl3"));
        assert_eq!(program_name(0), None);
        assert_eq!(program_name(243), None);
    }
}
