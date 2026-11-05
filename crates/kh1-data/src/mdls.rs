//! KH1 character models (`.mdls`): the skeleton (KH2's motions are retargeted onto it), and the
//! PS2 meshes and textures (`parse_faces`, `parse_textures`: Anti Form's look carries KH1
//! Anti-Sora's textures onto Sora's by them, `texmap`; KH1 draws its own meshes).
//!
//! Layout, worked out from the user's `xa_ex_0010.mdls` and the executable's loader
//! (`0x1401D5AD0`, which relocates a `MOBJ`; `0x1401DCF90` walks the tagged chunks):
//! - the file: `count u32`, then `count` section offsets; section 0 is the model object.
//! - a section is a run of chunks `{tag u32, size u32, data}`, the next at `+size+8`: `MOBJ`
//!   (model), `MENV`, `TEXA`, `BONE`, `MMTN`, `URSL`.
//! - `MOBJ`: offsets from its start at word 2 (the texture table, 16-byte entries, count =
//!   word 3 >> 4), 4 (the textures' pixels, word 5 bytes), 6 (their colour tables, word 7
//!   bytes), 8 (the model header), 10, 12.
//! - The model header (`MOBJ + word 8`): `bone_count u32`, an offset (from the header) to the
//!   bone table, another offset, `mesh_count u32`, then `mesh_count` 16-byte mesh entries.
//! - A bone is 0x30 bytes: `scale[3] f32, index u32, rotation[3] f32 (radians, KH2's Euler
//!   order), pad u32, translation[3] f32, links u32`, where `links` packs the parent (bits
//!   0-9), the next sibling (bits 10-19) and the first child (bits 20-29), 0x3FF meaning none.
//!   This is KH2's MDLX bone (`kh2_formats::anim::Bone`) packed tighter, so the KH2 tools pose
//!   it unchanged.
//! - A mesh entry (16 bytes): `pass u32` (1: drawn in the second pass), `texture u32` (twice),
//!   and the offset (from the model header) of its first packet.
//! - A mesh is a run of packets, each a PS2 DMA tag (quadword count in the low 16 bits, id in
//!   bits 28..30; the next packet `16 × (count + 1)` bytes on; id 7 ends the mesh) whose data,
//!   from `+0x10`, is chunks: tag 0, 0x80 bytes, putting bone `+4` in matrix slot `+8`; tag 1,
//!   `size` at `+4`, `count` at `+8`, a GIF tag at `+0x10`, then `count` 48-byte vertices from
//!   `+0x20` making one triangle strip; tag 0x8000 ends the packet. A vertex: normal (3 floats)
//!   and its matrix slot (u32), position in its bone's space (4 floats, w 1), UV (2 floats), 1.0,
//!   0. Checked (files): Sora's 9 meshes give 2828 triangles, as his remastered `-c0.cvbl`, and
//!   posed in the bind pose they lie on it (`texmap`'s tests).
//! - Textures (Checked (files), Sora's: seven 128×128 entries, `w u16, h u16` at `+4`, whose
//!   bytes at one a texel fill word 5 exactly, and seven 256-colour tables of 4 bytes fill word
//!   7): 8-bit indices, rows from the top, into a colour table of R, G, B, A (A 0..0x80) in the
//!   PS2's CSM1 order (within each 32, entries 8..15 and 16..23 swapped). Decoded so, Sora's
//!   texture 0 matches his HD `-100` within 1.1 of 255 (the test below). Assumption: every KH1
//!   character model's textures are 8-bit this way (anything else is refused).

use crate::image::Image;
use crate::mesh::{Corner, Face};
use glam::{Vec2, Vec3};
use kh2_formats::anim::Bone;
use kh2_formats::{Error, Result};

const MOBJ: u32 = u32::from_le_bytes(*b"MOBJ");
const NONE: u32 = 0x3FF;
const BONE_SIZE: usize = 0x30;

fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    b.get(at..at + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap())).ok_or_else(|| Error::Format(format!("mdls: read past the end at {at:#x}")))
}

fn f32_at(b: &[u8], at: usize) -> Result<f32> {
    Ok(f32::from_bits(u32_at(b, at)?))
}

/// Section 0's `MOBJ` (its offset in the file).
fn mobj(mdls: &[u8]) -> Result<usize> {
    let count = u32_at(mdls, 0)? as usize;
    if count == 0 || count > 64 {
        return Err(Error::Format(format!("mdls: {count} sections")));
    }
    let mobj = u32_at(mdls, 4)? as usize;
    if u32_at(mdls, mobj)? != MOBJ {
        return Err(Error::Format("mdls: section 0 is not a MOBJ".into()));
    }
    Ok(mobj)
}

/// The skeleton of a KH1 `.mdls`, as KH2 bones (parents before children, as stored).
pub fn parse_skeleton(mdls: &[u8]) -> Result<Vec<Bone>> {
    let mobj = mobj(mdls)?;
    let header = mobj + u32_at(mdls, mobj + 8 * 4)? as usize;
    let bone_count = u32_at(mdls, header)? as usize;
    let table = header + u32_at(mdls, header + 4)? as usize;
    parse_bones(mdls, table, bone_count)
}

/// `count` bones of the 0x30-byte table at `table` in `bytes` (a whole `.mdls`, or the table
/// itself read from the game's memory with `table` 0).
pub fn parse_bones(mdls: &[u8], table: usize, bone_count: usize) -> Result<Vec<Bone>> {
    if bone_count == 0 || bone_count > 1024 {
        return Err(Error::Format(format!("mdls: {bone_count} bones")));
    }
    let link = |v: u32, shift: u32| -> i16 {
        let x = (v >> shift) & NONE;
        if x == NONE { -1 } else { x as i16 }
    };
    (0..bone_count)
        .map(|i| {
            let at = table + i * BONE_SIZE;
            let f3 = |o: usize| -> Result<[f32; 3]> { Ok([f32_at(mdls, at + o)?, f32_at(mdls, at + o + 4)?, f32_at(mdls, at + o + 8)?]) };
            let index = u32_at(mdls, at + 0x0C)?;
            if index as usize != i {
                return Err(Error::Format(format!("mdls: bone {i} says it is {index}")));
            }
            let links = u32_at(mdls, at + 0x2C)?;
            Ok(Bone {
                index: i as i16,
                sibling: link(links, 10),
                parent: link(links, 0),
                child: link(links, 20),
                flags: (links >> 30) as i32,
                scale: f3(0)?,
                length: 0.0,
                rotation: f3(0x10)?,
                translation: f3(0x20)?,
            })
        })
        .collect()
}

fn bad<T>(m: String) -> Result<T> {
    Err(Error::Format(format!("mdls: {m}")))
}

/// The PS2 meshes' triangles (each corner in its bone's space), mesh by mesh; a face's `slot` is
/// its mesh's texture (`parse_textures`' index). A strip's facing alternates, its first
/// triangle's `+1`, or `-1` when the chunk's `+0xC` is 1 (Checked (files): so the triangles
/// with a twin in Sora's remastered mesh face as it does, `tests/local_mesh.rs`).
pub fn parse_faces(mdls: &[u8]) -> Result<Vec<Face>> {
    let mobj = mobj(mdls)?;
    let header = mobj + u32_at(mdls, mobj + 8 * 4)? as usize;
    let mesh_count = u32_at(mdls, header + 12)? as usize;
    if mesh_count == 0 || mesh_count > 64 {
        return bad(format!("{mesh_count} meshes"));
    }
    let mut faces = Vec::new();
    for m in 0..mesh_count {
        let e = header + 16 + 16 * m;
        let slot = u32_at(mdls, e + 4)?;
        let mut o = header + u32_at(mdls, e + 12)? as usize;
        let mut matrix = [0usize; 256];
        for packets in 0.. {
            if packets > 4096 {
                return bad(format!("mesh {m}: no end packet"));
            }
            let tag = u32_at(mdls, o)?;
            let (qwc, id) = ((tag & 0xFFFF) as usize, tag >> 28 & 7);
            if id != 1 && id != 7 {
                return bad(format!("mesh {m}: a packet tag {tag:#x} at {o:#x}"));
            }
            let end = o + 16 * (qwc + 1);
            let mut p = o + 0x10;
            loop {
                if p >= end {
                    return bad(format!("mesh {m}: the packet at {o:#x} has no end chunk"));
                }
                match u32_at(mdls, p)? {
                    0 => {
                        let (bone, s) = (u32_at(mdls, p + 4)? as usize, u32_at(mdls, p + 8)? as usize);
                        if s < matrix.len() {
                            matrix[s] = bone;
                        }
                        p += 0x80;
                    }
                    1 => {
                        let (size, n) = (u32_at(mdls, p + 4)? as usize, u32_at(mdls, p + 8)? as usize);
                        if 0x20 + 48 * n > size {
                            return bad(format!("mesh {m}: {n} vertices in a chunk of {size:#x}"));
                        }
                        // (`+0xC` 1: the strip starts the other way round.)
                        let first = if u32_at(mdls, p + 12)? & 1 == 1 { -1.0 } else { 1.0 };
                        let mut strip: Vec<Corner> = Vec::with_capacity(n);
                        for k in 0..n {
                            let v = p + 0x20 + 48 * k;
                            let s = u32_at(mdls, v + 12)? as usize;
                            strip.push(Corner {
                                bone: matrix.get(s).copied().unwrap_or(0),
                                pos: Vec3::new(f32_at(mdls, v + 16)?, f32_at(mdls, v + 20)?, f32_at(mdls, v + 24)?),
                                uv: Vec2::new(f32_at(mdls, v + 32)?, f32_at(mdls, v + 36)?),
                            });
                        }
                        for k in 2..n {
                            let facing = if k % 2 == 0 { first } else { -first };
                            faces.push(Face { mesh: m as u32, slot, corners: [strip[k - 2], strip[k - 1], strip[k]], facing });
                        }
                        p += size;
                    }
                    0x8000 => break,
                    t => return bad(format!("mesh {m}: a chunk of tag {t:#x} at {p:#x}")),
                }
            }
            if id == 7 {
                break;
            }
            o = end;
        }
    }
    Ok(faces)
}

/// The PS2 textures, in the texture table's order, as B8G8R8A8 (their alpha 0..0x80 made
/// 0..0xFF).
pub fn parse_textures(mdls: &[u8]) -> Result<Vec<Image>> {
    let mobj = mobj(mdls)?;
    let word = |i: usize| u32_at(mdls, mobj + 4 * i).map(|v| v as usize);
    let (table, count) = (mobj + word(2)?, word(3)? >> 4);
    let (pixels, pixels_len, cluts, cluts_len) = (mobj + word(4)?, word(5)?, mobj + word(6)?, word(7)?);
    if count == 0 || count > 64 {
        return bad(format!("{count} textures"));
    }
    let sizes: Vec<(usize, usize)> = (0..count).map(|i| Ok(((u32_at(mdls, table + 16 * i + 4)? & 0xFFFF) as usize, (u32_at(mdls, table + 16 * i + 4)? >> 16) as usize))).collect::<Result<_>>()?;
    let texels: usize = sizes.iter().map(|(w, h)| w * h).sum();
    if texels != pixels_len || count * 0x400 != cluts_len || sizes.iter().any(|&(w, h)| w == 0 || h == 0 || w > 1024 || h > 1024) {
        return bad(format!("textures {sizes:?} ({texels} texels) don't fill {pixels_len:#x} bytes at one a texel and {cluts_len:#x} of 256-colour tables"));
    }
    if mdls.len() < pixels + pixels_len || mdls.len() < cluts + cluts_len {
        return bad("textures past the end".into());
    }
    let mut at = pixels;
    let mut out = Vec::with_capacity(count);
    for (i, &(w, h)) in sizes.iter().enumerate() {
        let clut = &mdls[cluts + 0x400 * i..cluts + 0x400 * (i + 1)];
        // CSM1: within each 32 entries, 8..15 and 16..23 swapped.
        let colour = |idx: u8| {
            let j = idx as usize;
            let j = match j & 31 {
                8..=15 => j + 8,
                16..=23 => j - 8,
                _ => j,
            };
            let c = &clut[j * 4..j * 4 + 4];
            [c[2], c[1], c[0], ((c[3] as u32 * 255 + 64) / 128).min(255) as u8]
        };
        let bgra = mdls[at..at + w * h].iter().flat_map(|&ix| colour(ix)).collect();
        out.push(Image { w: w as u32, h: h as u32, bgra });
        at += w * h;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny `.mdls`: one section, a MOBJ with one 2×1 texture and one mesh of one packet
    /// (bone 3 in matrix slot 1, a strip of four vertices on it).
    fn tiny() -> Vec<u8> {
        let mut b = vec![0u8; 0x1000];
        let put = |b: &mut Vec<u8>, o: usize, v: u32| b[o..o + 4].copy_from_slice(&v.to_le_bytes());
        put(&mut b, 0, 1);
        put(&mut b, 4, 0x10);
        let m = 0x10;
        b[m..m + 4].copy_from_slice(b"MOBJ");
        // Texture table at +0x600 (one entry), pixels at +0x700 (2 bytes), colours at +0x800.
        put(&mut b, m + 8, 0x600);
        put(&mut b, m + 12, 0x10);
        put(&mut b, m + 16, 0x700);
        put(&mut b, m + 20, 2);
        put(&mut b, m + 24, 0x800);
        put(&mut b, m + 28, 0x400);
        put(&mut b, m + 32, 0x40);
        put(&mut b, m + 0x600 + 4, 1 << 16 | 2);
        b[m + 0x700] = 8;
        b[m + 0x701] = 1;
        // Index 8 is stored at 16 (CSM1); index 1 at 1.
        b[m + 0x800 + 16 * 4..m + 0x800 + 16 * 4 + 4].copy_from_slice(&[10, 20, 30, 0x80]);
        b[m + 0x800 + 4..m + 0x800 + 8].copy_from_slice(&[1, 2, 3, 0x40]);
        // Model header at +0x40: one mesh on texture 0, first packet at +0x100 from it.
        let h = m + 0x40;
        put(&mut b, h + 12, 1);
        put(&mut b, h + 16 + 12, 0x100);
        let mut p = h + 0x100;
        let start = p;
        p += 0x10;
        put(&mut b, p + 4, 3);
        put(&mut b, p + 8, 1);
        p += 0x80;
        put(&mut b, p, 1);
        put(&mut b, p + 4, 0x20 + 48 * 4);
        put(&mut b, p + 8, 4);
        for k in 0..4 {
            let v = p + 0x20 + 48 * k;
            put(&mut b, v + 12, 1);
            put(&mut b, v + 16, (k as f32).to_bits());
            put(&mut b, v + 36, 0.5f32.to_bits());
        }
        p += 0x20 + 48 * 4;
        put(&mut b, p, 0x8000);
        let qwc = (p + 0x10 - start - 0x10) / 16;
        put(&mut b, start, 0x7000_0000 | qwc as u32);
        b
    }

    #[test]
    fn ps2_meshes_and_textures_are_read() {
        let b = tiny();
        let f = parse_faces(&b).unwrap();
        assert_eq!(f.len(), 2);
        assert!(f.iter().all(|t| t.slot == 0 && t.corners.iter().all(|c| c.bone == 3 && c.uv.y == 0.5)));
        assert_eq!((f[0].facing, f[1].facing), (1.0, -1.0));
        assert_eq!(f[1].corners.map(|c| c.pos.x), [1.0, 2.0, 3.0]);
        let t = parse_textures(&b).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].bgra, vec![30, 20, 10, 255, 3, 2, 1, 128]);
    }
}
