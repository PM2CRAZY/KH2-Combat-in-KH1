//! What a frame build produces: world-space triangles in batches that
//! share a texture and draw state, in the game's draw order.
//!
//! ## Colour
//!
//! Every vertex colour is final: the sprite or model vertex colour, the
//! instance colour from the effect's colour programs, the particle colour
//! offsets and the element fade are already multiplied in. Units are the
//! PS2 Graphics Synthesizer's: **1.0 = 0x80**, so values up to about 2.0
//! occur. The renderer computes
//!
//! ```text
//! src.rgb = texture.rgb * vertex.rgb          (texture decoded to 0..1)
//! src.a   = texture.a   * vertex.a            (texture alpha 0x80 -> 1.0)
//! ```
//!
//! and blends `src` with the framebuffer by [`Blend`]: the GS equation
//! `Cv = (A - B) * C + D`, where the source alpha `src.a` is the `C` term
//! (1.0 = full). Additive (0x48) is `dst + src.rgb * src.a`, alpha blend
//! (0x44) `lerp(dst, src.rgb, src.a)`, subtractive (0x42)
//! `dst - src.rgb * src.a`. A shader that outputs `src.rgb * src.a` and
//! blends with ONE (or uses [`Blend::d3d`]) keeps the brightness of
//! `src.a > 1`, which a UNORM target would otherwise clamp.
//!
//! Untextured triangles (`texture == None`) use `src = vertex`. Sample
//! textures with wrapping (models scroll their coordinates past 1).
//!
//! How the sprite colours are combined follows the PC build's own CPU
//! vertex path for particles (`clamp(sprite * instance / 128, 0, 255)`
//! in 0x80 units); models and ribbons are scaled the same way.
//!
//! ## Depth
//!
//! Effects test depth (pass if nearer) unless `depth_test` is false, and
//! almost never write it. `depth_offset` is the game's "Z correction": the
//! depth used for testing is that of the vertex moved by this many units
//! along the **view z axis** (negative = toward the camera, typically -30
//! to -80), while its screen position stays where it is. A renderer can
//! implement it by transforming `view_pos + (0, 0, depth_offset)` for the
//! depth only, or approximate it with a depth bias.

use crate::library::TextureId;

/// One vertex in world space (KH2 units and axes, y down).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Vertex {
    pub pos: [f32; 3],
    /// Normalised texture coordinates of the batch texture; may go
    /// outside 0..1 (wrap).
    pub uv: [f32; 2],
    /// Final RGBA, 1.0 = 0x80 (see the module docs).
    pub color: [f32; 4],
}

impl Vertex {
    /// The colour as bytes with **0x80 = 1.0** (the PS2 convention), for
    /// renderers with 8-bit vertex colours: multiply by 255/128 (about 2)
    /// in the shader to get [`Vertex::color`] back.
    pub fn color_bytes(&self) -> [u8; 4] {
        self.color.map(|c| (c * 128.0).round().clamp(0.0, 255.0) as u8)
    }
}

/// GS alpha blending: `Cv = (A - B) * C + D` with the ALPHA register's low
/// byte (two bits per term, A first) and its fixed alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Blend {
    pub alpha: u8,
    pub fix: u8,
}

/// The common blend equations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMode {
    /// 0x48: `dst + src * As`.
    Additive,
    /// 0x44: `lerp(dst, src, As)`.
    Alpha,
    /// 0x42: `dst - src * As`.
    Subtract,
    /// 0x46: `dst * (1 - As)`.
    Darken,
    /// Anything else; use [`Blend::apply`] or [`Blend::d3d`].
    Other,
}

/// One input of the blend equation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendInput {
    Source,
    Dest,
    Zero,
}

/// The blend factor (the `C` term).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendCoef {
    SourceAlpha,
    DestAlpha,
    /// `fix / 128`.
    Fixed,
}

/// Blend factors in Direct3D terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum D3dFactor {
    Zero,
    One,
    SrcAlpha,
    InvSrcAlpha,
    DestAlpha,
    InvDestAlpha,
    /// The blend-factor constant; set it to `fix / 128`.
    BlendFactor,
    InvBlendFactor,
}

/// Blend operations in Direct3D terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum D3dOp {
    /// `src * Fs + dst * Fd`
    Add,
    /// `src * Fs - dst * Fd`
    Subtract,
    /// `dst * Fd - src * Fs`
    RevSubtract,
}

impl Blend {
    pub const ADDITIVE: Blend = Blend { alpha: 0x48, fix: 0 };
    pub const ALPHA: Blend = Blend { alpha: 0x44, fix: 0 };
    pub const SUBTRACT: Blend = Blend { alpha: 0x42, fix: 0 };

    fn input(bits: u8) -> BlendInput {
        match bits & 3 {
            0 => BlendInput::Source,
            1 => BlendInput::Dest,
            _ => BlendInput::Zero,
        }
    }

    pub fn a(&self) -> BlendInput {
        Blend::input(self.alpha)
    }
    pub fn b(&self) -> BlendInput {
        Blend::input(self.alpha >> 2)
    }
    pub fn c(&self) -> BlendCoef {
        match self.alpha >> 4 & 3 {
            0 => BlendCoef::SourceAlpha,
            1 => BlendCoef::DestAlpha,
            _ => BlendCoef::Fixed,
        }
    }
    pub fn d(&self) -> BlendInput {
        Blend::input(self.alpha >> 6)
    }

    pub fn mode(&self) -> BlendMode {
        match self.alpha {
            0x48 => BlendMode::Additive,
            0x44 => BlendMode::Alpha,
            0x42 => BlendMode::Subtract,
            0x46 => BlendMode::Darken,
            _ => BlendMode::Other,
        }
    }

    /// Reference blend: `src` is the shaded source (rgb and alpha, alpha
    /// 1.0 = 0x80), `dst` the framebuffer colour; destination alpha is
    /// taken as 1. Returns the unclamped result.
    pub fn apply(&self, src: [f32; 4], dst: [f32; 3]) -> [f32; 3] {
        let c = match self.c() {
            BlendCoef::SourceAlpha => src[3],
            BlendCoef::DestAlpha => 1.0,
            BlendCoef::Fixed => self.fix as f32 / 128.0,
        };
        let pick = |i: BlendInput, k: usize| match i {
            BlendInput::Source => src[k],
            BlendInput::Dest => dst[k],
            BlendInput::Zero => 0.0,
        };
        let mut out = [0.0; 3];
        for (k, o) in out.iter_mut().enumerate() {
            *o = (pick(self.a(), k) - pick(self.b(), k)) * c + pick(self.d(), k);
        }
        out
    }

    /// The same equation as Direct3D blend state, when it can be expressed
    /// (every value the effects here use can). Factors that are the GS
    /// alpha are the shader's output alpha, so a UNORM render target clamps
    /// them to 1.
    pub fn d3d(&self) -> Option<(D3dFactor, D3dFactor, D3dOp)> {
        // Coefficients of src and dst as (constant, multiple of C).
        let mut s = (0i32, 0i32);
        let mut d = (0i32, 0i32);
        let mut add = |i: BlendInput, k: (i32, i32)| match i {
            BlendInput::Source => s = (s.0 + k.0, s.1 + k.1),
            BlendInput::Dest => d = (d.0 + k.0, d.1 + k.1),
            BlendInput::Zero => {}
        };
        add(self.a(), (0, 1));
        add(self.b(), (0, -1));
        add(self.d(), (1, 0));
        let (c, ic) = match self.c() {
            BlendCoef::SourceAlpha => (D3dFactor::SrcAlpha, D3dFactor::InvSrcAlpha),
            BlendCoef::DestAlpha => (D3dFactor::DestAlpha, D3dFactor::InvDestAlpha),
            BlendCoef::Fixed => (D3dFactor::BlendFactor, D3dFactor::InvBlendFactor),
        };
        // Each coefficient as (factor, sign).
        let factor = |k: (i32, i32)| -> Option<(D3dFactor, i32)> {
            Some(match k {
                (0, 0) => (D3dFactor::Zero, 1),
                (1, 0) => (D3dFactor::One, 1),
                (-1, 0) => (D3dFactor::One, -1),
                (0, 1) => (c, 1),
                (0, -1) => (c, -1),
                (1, -1) => (ic, 1),
                (-1, 1) => (ic, -1),
                _ => return None,
            })
        };
        let (fs, ss) = factor(s)?;
        let (fd, sd) = factor(d)?;
        match (ss, sd) {
            (1, 1) => Some((fs, fd, D3dOp::Add)),
            (-1, 1) => Some((fs, fd, D3dOp::RevSubtract)),
            (1, -1) => Some((fs, fd, D3dOp::Subtract)),
            _ => None,
        }
    }
}

/// A run of triangles sharing texture and draw state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Batch {
    pub texture: Option<TextureId>,
    pub blend: Blend,
    pub depth_test: bool,
    pub depth_write: bool,
    /// View-space z shift for the depth test (see the module docs).
    pub depth_offset: f32,
    /// First vertex in [`Frame::vertices`]; `count` vertices follow
    /// (a triangle list, `count` is a multiple of 3).
    pub first: u32,
    pub count: u32,
    /// `Some(view depth)` for the game's depth-sorted draws. They are
    /// already placed after the immediate draws, farthest first.
    pub sorted: Option<f32>,
}

impl Batch {
    fn same_state(&self, o: &Batch) -> bool {
        self.texture == o.texture
            && self.blend == o.blend
            && self.depth_test == o.depth_test
            && self.depth_write == o.depth_write
            && self.depth_offset == o.depth_offset
            && self.sorted.is_none()
            && o.sorted.is_none()
    }
}

/// Draw state for the triangles about to be pushed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DrawState {
    pub texture: Option<TextureId>,
    pub blend: Blend,
    pub depth_test: bool,
    pub depth_write: bool,
    pub depth_offset: f32,
    pub sorted: Option<f32>,
}

/// The triangles of one frame.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub vertices: Vec<Vertex>,
    pub batches: Vec<Batch>,
}

impl Frame {
    pub fn new() -> Frame {
        Frame::default()
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.batches.clear();
    }

    /// Number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.vertices.len() / 3
    }

    /// Triangles of each batch in draw order.
    pub fn batch_vertices(&self, b: &Batch) -> &[Vertex] {
        &self.vertices[b.first as usize..(b.first + b.count) as usize]
    }

    /// Start a new draw (sorted draws never merge with others).
    pub(crate) fn begin(&mut self, st: DrawState) {
        let b = Batch {
            texture: st.texture,
            blend: st.blend,
            depth_test: st.depth_test,
            depth_write: st.depth_write,
            depth_offset: st.depth_offset,
            first: self.vertices.len() as u32,
            count: 0,
            sorted: st.sorted,
        };
        if let Some(last) = self.batches.last() {
            if last.count == 0 {
                self.batches.pop();
            } else if last.same_state(&b) {
                return;
            }
        }
        self.batches.push(b);
    }

    pub(crate) fn tri(&mut self, v: [Vertex; 3]) {
        self.vertices.extend_from_slice(&v);
        if let Some(b) = self.batches.last_mut() {
            b.count += 3;
        }
    }

    /// Two triangles: corners in the order top-left, top-right,
    /// bottom-right, bottom-left.
    pub(crate) fn quad(&mut self, v: [Vertex; 4]) {
        self.tri([v[0], v[1], v[2]]);
        self.tri([v[0], v[2], v[3]]);
    }

    /// Put sorted draws after the immediate ones, farthest first (the
    /// game's ordering table; its exact position among the immediate
    /// draws is inferred).
    pub(crate) fn finish(&mut self) {
        self.batches.retain(|b| b.count > 0);
        let (mut imm, mut sorted): (Vec<Batch>, Vec<Batch>) = self.batches.iter().partition(|b| b.sorted.is_none());
        sorted.sort_by(|a, b| b.sorted.unwrap_or(0.0).total_cmp(&a.sorted.unwrap_or(0.0)));
        imm.extend(sorted);
        self.batches = imm;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_bytes() {
        let v = Vertex { color: [1.0, 0.5, 255.0 / 128.0, 3.0], ..Default::default() };
        assert_eq!(v.color_bytes(), [128, 64, 255, 255]);
    }

    #[test]
    fn blend_equations() {
        let src = [1.0, 0.5, 0.25, 0.5];
        let dst = [0.2, 0.2, 0.2];
        let add = Blend::ADDITIVE.apply(src, dst);
        assert_eq!(add, [0.7, 0.45, 0.325]);
        let al = Blend::ALPHA.apply(src, dst);
        assert!((al[0] - 0.6).abs() < 1e-6);
        let sub = Blend::SUBTRACT.apply(src, dst);
        assert!((sub[0] + 0.3).abs() < 1e-6);
        assert_eq!(Blend::ADDITIVE.mode(), BlendMode::Additive);
    }

    #[test]
    fn d3d_factors() {
        assert_eq!(Blend::ADDITIVE.d3d(), Some((D3dFactor::SrcAlpha, D3dFactor::One, D3dOp::Add)));
        assert_eq!(Blend::ALPHA.d3d(), Some((D3dFactor::SrcAlpha, D3dFactor::InvSrcAlpha, D3dOp::Add)));
        assert_eq!(Blend::SUBTRACT.d3d(), Some((D3dFactor::SrcAlpha, D3dFactor::One, D3dOp::RevSubtract)));
        assert_eq!(Blend { alpha: 0x46, fix: 0 }.d3d(), Some((D3dFactor::Zero, D3dFactor::InvSrcAlpha, D3dOp::Add)));
        // Fixed alpha: (Cs - 0) * FIX + Cd.
        assert_eq!(Blend { alpha: 0x68, fix: 0x40 }.d3d(), Some((D3dFactor::BlendFactor, D3dFactor::One, D3dOp::Add)));
    }

    #[test]
    fn batching_and_sorting() {
        let mut f = Frame::new();
        let st = DrawState { texture: Some(0), blend: Blend::ADDITIVE, depth_test: true, depth_write: false, depth_offset: 0.0, sorted: None };
        f.begin(st);
        f.quad([Vertex::default(); 4]);
        f.begin(st);
        f.quad([Vertex::default(); 4]);
        f.begin(DrawState { sorted: Some(10.0), ..st });
        f.tri([Vertex::default(); 3]);
        f.begin(DrawState { sorted: Some(50.0), ..st });
        f.tri([Vertex::default(); 3]);
        f.begin(DrawState { texture: Some(1), ..st });
        f.tri([Vertex::default(); 3]);
        f.finish();
        assert_eq!(f.batches.len(), 4);
        assert_eq!(f.batches[0].count, 12);
        assert_eq!(f.batches[1].texture, Some(1));
        assert_eq!(f.batches[2].sorted, Some(50.0));
        assert_eq!(f.batches[3].sorted, Some(10.0));
        assert_eq!(f.triangle_count(), 7);
    }
}
