//! MDLX meshes and textures: enough to draw a KH2 character or weapon
//! model (Sora, his Drive Forms, his Keyblades) with its own skeleton.
//!
//! An MDLX is a BAR holding a model (type 4) and its textures (type 7).
//!
//! **Model.** After 0x90 reserved bytes comes a header (bone count and
//! offset, group count) and one 0x20-byte descriptor per group: attributes,
//! texture index, the offset of the group's DMA chain and its tag count.
//! Each packet of the chain is a reference to VIF code that fills VU1
//! memory, then one reference per bone matrix (the tag's address field is
//! the bone index, its second VIF word the VU address the matrix goes to),
//! then an MSCNT. We run the VIF code (UNPACK with masks and the fill
//! row) and read the VU layout it builds: a header, then index entries
//! (`u, v` in 1/4096 units, vertex index, strip flag) and vertices in the
//! local space of their bone, grouped by bone with a count per matrix.
//!
//! **Textures.** PS2 GS uploads: a list of image transfers (32-bit,
//! swizzled into GS memory) and, per texture, a `TEX0` register saying how
//! to read it back (8-bit indexed with a 256-colour 32-bit palette). We
//! replay the transfers into a 4 MiB copy of GS memory and read each
//! texture through the 8-bit page layout.
//!
//! The container and header layouts follow OpenKH's docs
//! (`docs/kh2/file/type/mdlx.md`, Apache-2.0); the GS memory layout is the
//! PS2's own (block and column arrangement as documented by the PS2
//! emulation community). The reading code here is our own, checked
//! against the user's files.

use crate::anim::{parse_skeleton, Bone, BAR_TYPE_MODEL};
use crate::bar::Bar;
use crate::reader::Reader;
use crate::{bad, Result};

/// BAR entry type of the texture container inside an MDLX.
pub const BAR_TYPE_TEXTURES: u16 = 7;
const RESERVED: usize = 0x90;

/// One corner of a triangle, in the local space of `bone`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub bone: u16,
    /// Texture coordinates, origin at the top left.
    pub uv: [f32; 2],
}

/// The triangles that share one texture.
#[derive(Clone, Debug, Default)]
pub struct Group {
    /// The descriptor's first word. Bit 0 marks a translucent group
    /// (Keyblade shine), drawn blended after the opaque ones.
    pub attributes: u32,
    pub texture: usize,
    pub triangles: Vec<[Vertex; 3]>,
}

impl Group {
    pub fn translucent(&self) -> bool {
        self.attributes & 1 != 0
    }
}

/// An RGBA8 image (alpha 255 = opaque).
#[derive(Clone, Debug, Default)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub bones: Vec<Bone>,
    pub groups: Vec<Group>,
    pub textures: Vec<Texture>,
}

impl Model {
    pub fn triangle_count(&self) -> usize {
        self.groups.iter().map(|g| g.triangles.len()).sum()
    }
}

/// Reads the first model of an MDLX with its skeleton and textures.
pub fn parse(mdlx: &[u8]) -> Result<Model> {
    let bar = Bar::parse(mdlx)?;
    let Some(model) = bar.entries.iter().find(|e| e.kind == BAR_TYPE_MODEL) else {
        return bad("MDLX has no model entry");
    };
    let bones = parse_skeleton(mdlx)?;
    let groups = parse_groups(model.data, bones.len())?;
    let textures = match bar.entries.iter().find(|e| e.kind == BAR_TYPE_TEXTURES) {
        Some(t) => parse_textures(t.data)?,
        None => Vec::new(),
    };
    for g in &groups {
        if g.texture >= textures.len() && !textures.is_empty() {
            return bad(format!("group uses texture {} of {}", g.texture, textures.len()));
        }
    }
    Ok(Model { bones, groups, textures })
}

fn parse_groups(d: &[u8], bone_count: usize) -> Result<Vec<Group>> {
    let base = RESERVED;
    let count = Reader::at(d, base + 0x1C).u16()? as usize;
    let mut out = Vec::with_capacity(count);
    for g in 0..count {
        let mut r = Reader::at(d, base + 0x20 + g * 0x20);
        let attributes = r.u32()?;
        let texture = r.u32()? as usize;
        let _matrix_count = r.u32()?;
        let _unknown = r.u32()?;
        let dma = r.u32()? as usize;
        let _matrix_list = r.u32()?;
        let tags = r.u32()? as usize;
        if tags > 0x10000 {
            return bad(format!("implausible DMA tag count {tags}"));
        }
        let mut group = Group { attributes, texture, triangles: Vec::new() };
        let tag = |i: usize| -> Result<DmaTag> { DmaTag::read(d, base + dma + i * 16) };
        let mut i = 0;
        while i < tags {
            let t = tag(i)?;
            i += 1;
            if !(t.id == 3 && t.vif[0] == 0) {
                continue;
            }
            let start = base + t.addr as usize;
            let Some(code) = d.get(start..start + t.qwc as usize * 16) else {
                return bad("VIF packet past the end of the model");
            };
            let mut matrices = Vec::new();
            while i < tags {
                let m = tag(i)?;
                if m.id != 3 || m.vif[0] == 0 {
                    break;
                }
                matrices.push(((m.vif[1] & 0x3ff) as u32, m.addr));
                i += 1;
            }
            let mut vu = vec![[0u32; 4]; 1024];
            run_vif(code, &mut vu)?;
            read_packet(&vu, &matrices, bone_count, &mut group.triangles)?;
        }
        out.push(group);
    }
    Ok(out)
}

struct DmaTag {
    qwc: u16,
    id: u8,
    addr: u32,
    vif: [u32; 2],
}

impl DmaTag {
    fn read(d: &[u8], at: usize) -> Result<DmaTag> {
        let mut r = Reader::at(d, at);
        let qwc = r.u16()?;
        let param = r.u16()?;
        let addr = r.u32()?;
        let vif = [r.u32()?, r.u32()?];
        Ok(DmaTag { qwc, id: ((param >> 12) & 7) as u8, addr, vif })
    }
}

/// Runs the VIF1 commands a model uses into a copy of VU1 data memory
/// (1024 quadwords).
fn run_vif(code: &[u8], vu: &mut [[u32; 4]]) -> Result<()> {
    let word = |at: usize| -> Option<u32> { code.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())) };
    let mut i = 0;
    let mut mask = 0u32;
    let mut row = [0u32; 4];
    while let Some(c) = word(i) {
        i += 4;
        let imm = c & 0xffff;
        let num = (c >> 16) & 0xff;
        let cmd = (c >> 24) & 0x7f;
        match cmd {
            // NOP, STCYCL, OFFSET, BASE, ITOP, STMOD, MARK, FLUSHE, FLUSH,
            // FLUSHA, MSCAL, MSCALF, MSCNT
            0x00..=0x07 | 0x10 | 0x11 | 0x13 | 0x14 | 0x15 | 0x17 => {}
            0x20 => {
                mask = word(i).ok_or_else(|| crate::Error::Format("STMASK cut short".into()))?;
                i += 4;
            }
            0x30 | 0x31 => {
                let mut v = [0u32; 4];
                for (k, x) in v.iter_mut().enumerate() {
                    *x = word(i + k * 4).ok_or_else(|| crate::Error::Format("STROW cut short".into()))?;
                }
                if cmd == 0x30 {
                    row = v;
                }
                i += 16;
            }
            _ if cmd & 0x60 == 0x60 => {
                let vn = ((cmd >> 2) & 3) as usize;
                let vl = (cmd & 3) as usize;
                let masked = cmd & 0x10 != 0;
                let n = if num == 0 { 256 } else { num as usize };
                let addr = (imm & 0x3ff) as usize;
                let unsigned = imm & 0x4000 != 0;
                let comps = vn + 1;
                let bits = [32usize, 16, 8, 5][vl];
                if bits == 5 {
                    return bad("V4-5 unpack is not used by models");
                }
                let size = (n * comps * bits).div_ceil(32) * 4;
                let Some(data) = code.get(i..i + size) else {
                    return bad("UNPACK data cut short");
                };
                i += size;
                for k in 0..n {
                    let mut v = [0u32; 4];
                    for (c, x) in v.iter_mut().enumerate().take(comps) {
                        let at = (k * comps + c) * bits / 8;
                        *x = match bits {
                            32 => u32::from_le_bytes(data[at..at + 4].try_into().unwrap()),
                            16 => {
                                let h = u16::from_le_bytes([data[at], data[at + 1]]);
                                if unsigned { h as u32 } else { h as i16 as i32 as u32 }
                            }
                            _ => {
                                let b = data[at];
                                if unsigned { b as u32 } else { b as i8 as i32 as u32 }
                            }
                        };
                    }
                    // Short formats fill the rest: S -> xxxx, V2 -> xyxy;
                    // V3 leaves w alone.
                    let lanes: [Option<u32>; 4] = match comps {
                        1 => [Some(v[0]); 4],
                        2 => [Some(v[0]), Some(v[1]), Some(v[0]), Some(v[1])],
                        3 => [Some(v[0]), Some(v[1]), Some(v[2]), None],
                        _ => v.map(Some),
                    };
                    let Some(q) = vu.get_mut(addr + k) else {
                        return bad("UNPACK past the end of VU memory");
                    };
                    for c in 0..4 {
                        let m = if masked { (mask >> (c * 2)) & 3 } else { 0 };
                        match (m, lanes[c]) {
                            (0, Some(x)) => q[c] = x,
                            (1, _) => q[c] = row[c],
                            _ => {}
                        }
                    }
                }
            }
            _ => return bad(format!("unknown VIF command {cmd:#x}")),
        }
    }
    Ok(())
}

/// Turns one packet's VU memory into triangles.
fn read_packet(vu: &[[u32; 4]], matrices: &[(u32, u32)], bone_count: usize, out: &mut Vec<[Vertex; 3]>) -> Result<()> {
    let q = |a: usize| -> Result<[u32; 4]> { vu.get(a).copied().ok_or_else(|| crate::Error::Format("VU address out of range".into())) };
    let [kind, ..] = q(0)?;
    if kind != 1 {
        return bad(format!("unsupported vertex packet type {kind}"));
    }
    let [index_count, index_at, counts_at, matrix_at] = q(1)?.map(|x| x as usize);
    let [vertex_count, vertex_at, _, matrix_count] = q(3)?.map(|x| x as usize);
    // Which bone each vertex belongs to: `counts` vertices per matrix slot.
    let mut bone_of = Vec::with_capacity(vertex_count);
    for k in 0..matrix_count {
        let n = q(counts_at + k / 4)?[k % 4] as usize;
        let slot = (matrix_at + 4 * k) as u32;
        let Some(&(_, bone)) = matrices.iter().find(|(a, _)| *a == slot) else {
            return bad(format!("no matrix for VU slot {slot:#x}"));
        };
        if bone as usize >= bone_count {
            return bad(format!("bone {bone} of {bone_count}"));
        }
        bone_of.extend(std::iter::repeat(bone as u16).take(n));
    }
    if bone_of.len() != vertex_count {
        return bad(format!("{} vertices for {} matrix entries", vertex_count, bone_of.len()));
    }
    let mut ring: Vec<Vertex> = Vec::with_capacity(3);
    for k in 0..index_count {
        let [u, v, index, flag] = q(index_at + k)?;
        let index = index as usize;
        if index >= vertex_count {
            return bad(format!("vertex {index} of {vertex_count}"));
        }
        let p = q(vertex_at + index)?;
        let vert = Vertex {
            position: [f32::from_bits(p[0]), f32::from_bits(p[1]), f32::from_bits(p[2])],
            bone: bone_of[index],
            uv: [u as i32 as f32 / 4096.0, v as i32 as f32 / 4096.0],
        };
        if ring.len() == 3 {
            ring.remove(0);
        }
        ring.push(vert);
        // 0x10 only feeds the strip; 0x20 and 0x30 draw with the two
        // windings (0x00 draws as 0x20).
        if flag & 0x30 != 0x10 && ring.len() == 3 {
            out.push(if flag & 0x30 == 0x30 { [ring[0], ring[2], ring[1]] } else { [ring[0], ring[1], ring[2]] });
        }
    }
    Ok(())
}

// ---- textures ----

const GS_BYTES: usize = 4 << 20;

const BLOCK32: [[usize; 8]; 4] =
    [[0, 1, 4, 5, 16, 17, 20, 21], [2, 3, 6, 7, 18, 19, 22, 23], [8, 9, 12, 13, 24, 25, 28, 29], [10, 11, 14, 15, 26, 27, 30, 31]];
const COLUMN32: [[usize; 8]; 8] = [
    [0, 1, 4, 5, 8, 9, 12, 13],
    [2, 3, 6, 7, 10, 11, 14, 15],
    [16, 17, 20, 21, 24, 25, 28, 29],
    [18, 19, 22, 23, 26, 27, 30, 31],
    [32, 33, 36, 37, 40, 41, 44, 45],
    [34, 35, 38, 39, 42, 43, 46, 47],
    [48, 49, 52, 53, 56, 57, 60, 61],
    [50, 51, 54, 55, 58, 59, 62, 63],
];

struct Gs {
    mem: Vec<u8>,
}

impl Gs {
    fn addr32(bp: usize, bw: usize, x: usize, y: usize) -> usize {
        let page = (x >> 6) + (y >> 5) * bw;
        let block = (bp + page * 32 + BLOCK32[(y >> 3) & 3][(x >> 3) & 7]) & 0x3fff;
        (block * 64 + COLUMN32[y & 7][x & 7]) * 4
    }

    fn addr8(bp: usize, bw: usize, x: usize, y: usize) -> usize {
        let page = (x >> 7) + (y >> 6) * (bw >> 1).max(1);
        let block = (bp + page * 32 + BLOCK32[(y >> 4) & 3][(x >> 4) & 7]) & 0x3fff;
        block * 256 + COLUMN8[y & 15][x & 15]
    }

    fn write32(&mut self, bp: usize, bw: usize, w: usize, h: usize, data: &[u8]) {
        for (i, px) in data.chunks_exact(4).take(w * h).enumerate() {
            let a = Gs::addr32(bp, bw, i % w, i / w);
            self.mem[a..a + 4].copy_from_slice(px);
        }
    }
}

/// Byte offset inside a 256-byte block of each pixel of a 16x16 8-bit
/// block (PSMT8). Derived from how a 32-bit upload of known bytes lands
/// in GS memory and checked against the decoded textures (rows 2/3 and
/// 10/11 swap their 4-pixel groups).
const COLUMN8: [[usize; 16]; 16] = [
    [0, 4, 16, 20, 32, 36, 48, 52, 2, 6, 18, 22, 34, 38, 50, 54],
    [8, 12, 24, 28, 40, 44, 56, 60, 10, 14, 26, 30, 42, 46, 58, 62],
    [33, 37, 49, 53, 1, 5, 17, 21, 35, 39, 51, 55, 3, 7, 19, 23],
    [41, 45, 57, 61, 9, 13, 25, 29, 43, 47, 59, 63, 11, 15, 27, 31],
    [96, 100, 112, 116, 64, 68, 80, 84, 98, 102, 114, 118, 66, 70, 82, 86],
    [104, 108, 120, 124, 72, 76, 88, 92, 106, 110, 122, 126, 74, 78, 90, 94],
    [65, 69, 81, 85, 97, 101, 113, 117, 67, 71, 83, 87, 99, 103, 115, 119],
    [73, 77, 89, 93, 105, 109, 121, 125, 75, 79, 91, 95, 107, 111, 123, 127],
    [128, 132, 144, 148, 160, 164, 176, 180, 130, 134, 146, 150, 162, 166, 178, 182],
    [136, 140, 152, 156, 168, 172, 184, 188, 138, 142, 154, 158, 170, 174, 186, 190],
    [161, 165, 177, 181, 129, 133, 145, 149, 163, 167, 179, 183, 131, 135, 147, 151],
    [169, 173, 185, 189, 137, 141, 153, 157, 171, 175, 187, 191, 139, 143, 155, 159],
    [224, 228, 240, 244, 192, 196, 208, 212, 226, 230, 242, 246, 194, 198, 210, 214],
    [232, 236, 248, 252, 200, 204, 216, 220, 234, 238, 250, 254, 202, 206, 218, 222],
    [193, 197, 209, 213, 225, 229, 241, 245, 195, 199, 211, 215, 227, 231, 243, 247],
    [201, 205, 217, 221, 233, 237, 249, 253, 203, 207, 219, 223, 235, 239, 251, 255],
];

/// Decodes an MDLX texture container (BAR type 7).
pub fn parse_textures(tim: &[u8]) -> Result<Vec<Texture>> {
    let mut h = Reader::at(tim, 0);
    let mut head = [0u32; 9];
    for x in &mut head {
        *x = h.u32()?;
    }
    let [_, _, count, _, map_at, transfers_at, regs_at, _, _] = head.map(|x| x as usize);
    if count > 256 {
        return bad(format!("implausible texture count {count}"));
    }
    let mut gs = Gs { mem: vec![0; GS_BYTES] };
    // Image transfers: 0x90 bytes each (DMA tag, GIF tag, BITBLTBUF,
    // TRXPOS, TRXREG, TRXDIR, IMAGE tag, data reference, end).
    let mut at = transfers_at;
    while at + 0x90 <= regs_at {
        let q = |o: usize| -> Result<u64> { Reader::at(tim, at + o).i64().map(|x| x as u64) };
        if q(0x28)? != 0x50 || q(0x48)? != 0x52 {
            break;
        }
        let bitblt = q(0x20)?;
        let trxreg = q(0x40)?;
        let data_ref = q(0x70)?;
        let dbp = ((bitblt >> 32) & 0x3fff) as usize;
        let dbw = ((bitblt >> 48) & 0x3f) as usize;
        let dpsm = (bitblt >> 56) & 0x3f;
        let w = (trxreg & 0xfff) as usize;
        let hgt = ((trxreg >> 32) & 0xfff) as usize;
        let src = (data_ref >> 32) as usize;
        let qwc = (data_ref & 0xffff) as usize;
        if dpsm != 0 || qwc * 16 != w * hgt * 4 {
            return bad(format!("unsupported texture upload (psm {dpsm}, {w}x{hgt}, {qwc} qw)"));
        }
        // A palette upload can run past the end of the container (the PS2
        // read on into whatever followed); only its start is ever used.
        let Some(data) = tim.get(src..(src + qwc * 16).min(tim.len())) else {
            return bad("texture data past the end");
        };
        gs.write32(dbp, dbw, w, hgt, data);
        at += 0x90;
    }
    // TEX0 registers, in order.
    let mut tex0s = Vec::new();
    let mut at = regs_at;
    while at + 16 <= tim.len() && tex0s.len() < 256 {
        let mut r = Reader::at(tim, at);
        let value = r.i64()? as u64;
        let reg = r.i64()? as u64;
        if reg == 0x06 {
            tex0s.push(value);
        }
        at += 16;
        if tex0s.len() >= count && reg == 0x08 {
            break;
        }
    }
    let map = tim.get(map_at..map_at + count).ok_or_else(|| crate::Error::Format("texture map cut short".into()))?;
    map.iter().map(|&m| decode_tex0(&gs, *tex0s.get(m as usize).ok_or_else(|| crate::Error::Format(format!("texture map entry {m}")))?)).collect()
}

fn decode_tex0(gs: &Gs, tex0: u64) -> Result<Texture> {
    let f = |shift: u32, bits: u32| ((tex0 >> shift) & ((1 << bits) - 1)) as usize;
    let (tbp, tbw, psm) = (f(0, 14), f(14, 6), f(20, 6));
    let (w, h) = (1usize << f(26, 4), 1usize << f(30, 4));
    let (cbp, cpsm, csm) = (f(37, 14), f(51, 4), f(55, 1));
    if psm != 19 || cpsm != 0 || csm != 0 || w > 1024 || h > 1024 {
        return bad(format!("unsupported texture format (psm {psm}, cpsm {cpsm}, csm {csm}, {w}x{h})"));
    }
    // 256 colours as a 16x16 CT32 block; CSM1 swaps index bits 3 and 4.
    let mut palette = [[0u8; 4]; 256];
    for (i, c) in palette.iter_mut().enumerate() {
        let j = (i & !0x18) | ((i & 8) << 1) | ((i & 16) >> 1);
        let a = Gs::addr32(cbp, 1, j & 15, j >> 4);
        let p = &gs.mem[a..a + 4];
        *c = [p[0], p[1], p[2], (p[3] as u16 * 2).min(255) as u8];
    }
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            rgba.extend_from_slice(&palette[gs.mem[Gs::addr8(tbp, tbw, x, y)] as usize]);
        }
    }
    Ok(Texture { width: w as u32, height: h as u32, rgba })
}
