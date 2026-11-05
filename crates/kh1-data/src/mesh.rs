//! A model's meshes as triangles, each corner in its bone's space with its UV: KH1's remastered
//! meshes (`-c0.cvbl`, a model's remastered asset, the mesh the PC draws: [`cvbl_faces`]) and the
//! PS2 meshes inside a `.mdls` (`mdls::parse_faces`). Posed by the model's skeleton
//! (`kh2_anim::Pose::bind`), both lie on the same surface when the geometry is the same (Sora's
//! remastered mesh and his PS2 one; KH1's Anti-Sora is Sora's PS2 mesh), which `texmap` uses to
//! carry a texture from one UV layout to the other.
//!
//! The `-c0.cvbl` layout (Checked (files): Sora's `xa_ex_0010.mdls` `-c0.cvbl`, its 9 meshes and
//! 2828 triangles, `docs/kh1-tint.md`): `count` at `+4`, a skip at `+8` (`0x1xxxx`: skip `xxxx`),
//! then 16-byte entries (`+4` the texture slot, `+0xC` the packet list) from `0x10 × (1 + 2 ×
//! skip)`. A packet list has 16 bytes of header, then chunks `{tag u32, size u32 (whole chunk)}`:
//! tag `0x11` puts bone `+8` in matrix slot `+0xC`, tag 1 holds `count` (`+8`) 48-byte vertices
//! from `+0x20`, tag `0x8000` ends. A vertex: normal (3 floats), flags (low 16 bits the matrix
//! slot; high 16: `0x1000` starts a strip, 1 or 2 makes a triangle with the two before, by its
//! winding), UV (2 floats), two more words, position in its bone's space (4 floats).
//!
//! Nothing here holds game data; it reads the player's own files.

use glam::{Vec2, Vec3};
use kh2_formats::{Error, Result};

/// One corner of a triangle: its bone, its position in that bone's space, its UV.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner {
    pub bone: usize,
    pub pos: Vec3,
    pub uv: Vec2,
}

/// A triangle of a model: its mesh (the mesh table's entry), the texture slot it samples, its
/// three corners, and its facing by the strip's winding (+1 or -1: the side the strip's order
/// makes it face, the same in two models built alike).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    pub mesh: u32,
    pub slot: u32,
    pub corners: [Corner; 3],
    pub facing: f32,
}

pub(crate) fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap())).ok_or_else(|| Error::Format(format!("read past the end at {o:#x}")))
}

pub(crate) fn f32_at(b: &[u8], o: usize) -> Result<f32> {
    Ok(f32::from_bits(u32_at(b, o)?))
}

pub(crate) fn vec3_at(b: &[u8], o: usize) -> Result<Vec3> {
    Ok(Vec3::new(f32_at(b, o)?, f32_at(b, o + 4)?, f32_at(b, o + 8)?))
}

fn bad<T>(m: String) -> Result<T> {
    Err(Error::Format(format!("cvbl: {m}")))
}

/// The triangles of a remastered mesh file (`-c0.cvbl`), mesh by mesh.
pub fn cvbl_faces(cvbl: &[u8]) -> Result<Vec<Face>> {
    let count = u32_at(cvbl, 4)? as usize;
    let w = u32_at(cvbl, 8)?;
    let skip = if w & 0x1_0000 != 0 { (w & 0xFFFF) as usize } else { 0 };
    if count == 0 || count > 64 || skip > 16 {
        return bad(format!("{count} meshes, skip {skip}"));
    }
    let mut faces = Vec::new();
    for m in 0..count {
        let e = 0x10 * (1 + skip * 2 + m);
        let slot = u32_at(cvbl, e + 4)?;
        let list = u32_at(cvbl, e + 0xC)? as usize;
        let mut matrix = [0usize; 256];
        let mut o = list + 0x10;
        loop {
            let tag = u32_at(cvbl, o)?;
            let size = u32_at(cvbl, o + 4)? as usize;
            match tag {
                0x8000 => break,
                0x11 => {
                    let bone = u32_at(cvbl, o + 8)? as usize;
                    let s = u32_at(cvbl, o + 0xC)? as usize;
                    if s < matrix.len() {
                        matrix[s] = bone;
                    }
                }
                1 => {
                    let n = u32_at(cvbl, o + 8)? as usize;
                    if 0x20 + 48 * n > size {
                        return bad(format!("mesh {m}: {n} vertices in a chunk of {size:#x}"));
                    }
                    let mut strip: Vec<Corner> = Vec::with_capacity(3);
                    for k in 0..n {
                        let v = o + 0x20 + 48 * k;
                        let flags = u32_at(cvbl, v + 12)?;
                        let corner = Corner {
                            bone: matrix.get((flags & 0xFFFF) as usize).copied().unwrap_or(0),
                            pos: vec3_at(cvbl, v + 32)?,
                            uv: Vec2::new(f32_at(cvbl, v + 16)?, f32_at(cvbl, v + 20)?),
                        };
                        if strip.len() == 3 {
                            strip.remove(0);
                        }
                        strip.push(corner);
                        if flags >> 16 & 0x1000 == 0 && strip.len() == 3 {
                            let facing = if flags >> 16 & 3 == 1 { 1.0 } else { -1.0 };
                            faces.push(Face { mesh: m as u32, slot, corners: [strip[0], strip[1], strip[2]], facing });
                        }
                    }
                }
                _ => {}
            }
            if size < 8 {
                return bad(format!("mesh {m}: a chunk of size {size:#x}"));
            }
            o += size;
        }
    }
    Ok(faces)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_make_triangles() {
        // One mesh, slot 2: bone 1 in matrix slot 0, four vertices as one strip (two triangles).
        let mut b = vec![0u8; 0x200];
        let put = |b: &mut Vec<u8>, o: usize, v: u32| b[o..o + 4].copy_from_slice(&v.to_le_bytes());
        put(&mut b, 4, 1);
        put(&mut b, 0x14, 2);
        put(&mut b, 0x1C, 0x20);
        let mut o = 0x30;
        put(&mut b, o, 0x11);
        put(&mut b, o + 4, 0x10);
        put(&mut b, o + 8, 1);
        o += 0x10;
        put(&mut b, o, 1);
        put(&mut b, o + 4, 0x20 + 48 * 4);
        put(&mut b, o + 8, 4);
        let flags = [0x1000_0000u32, 0x1000_0000, 0x1_0000, 0x2_0000];
        let pos = [[0.0f32, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        for k in 0..4 {
            let v = o + 0x20 + 48 * k;
            put(&mut b, v + 12, flags[k]);
            put(&mut b, v + 16, (k as f32 / 4.0).to_bits());
            put(&mut b, v + 32, pos[k][0].to_bits());
            put(&mut b, v + 36, pos[k][1].to_bits());
        }
        o += 0x20 + 48 * 4;
        put(&mut b, o, 0x8000);
        put(&mut b, o + 4, 0x10);
        let f = cvbl_faces(&b).unwrap();
        assert_eq!(f.len(), 2);
        assert!(f.iter().all(|t| t.slot == 2 && t.mesh == 0 && t.corners.iter().all(|c| c.bone == 1)));
        assert_eq!((f[0].facing, f[1].facing), (1.0, -1.0));
        assert_eq!(f[1].corners[2].pos, Vec3::new(1.0, 1.0, 0.0));
        assert_eq!(f[1].corners[0].uv, Vec2::new(0.25, 0.0));
    }

    #[test]
    fn a_short_file_is_refused() {
        assert!(cvbl_faces(&[0u8; 6]).is_err());
        let mut b = vec![0u8; 0x20];
        b[4] = 100;
        assert!(cvbl_faces(&b).is_err());
    }
}
