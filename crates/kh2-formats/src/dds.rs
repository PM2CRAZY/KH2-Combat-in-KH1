//! A small DDS reader for the remastered (HD) textures the PC packages carry beside the PS2 data
//! (KH2's models and effects come as DXT5 DDS files; some as plain 32-bit). Only what those use:
//! the first mip level of a 2D texture in BC1 (DXT1), BC2 (DXT3), BC3 (DXT5) or uncompressed
//! 32-bit with channel masks. Our own code, written from the DDS and BC format descriptions
//! (Microsoft's "Programming Guide for DDS" and "Texture Block Compression in Direct3D 11").

use crate::png::Image;
use crate::{bad, Result};

fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// A 5:6:5 colour to 8-bit channels.
fn rgb565(c: u16) -> [u8; 3] {
    let (r, g, b) = ((c >> 11) & 31, (c >> 5) & 63, c & 31);
    [((r << 3) | (r >> 2)) as u8, ((g << 2) | (g >> 4)) as u8, ((b << 3) | (b >> 2)) as u8]
}

/// The 16 colours of a BC1 colour block (`b`, 8 bytes), RGBA. `four` forces the four-colour
/// mode (BC2 and BC3 always use it); otherwise `c0 <= c1` selects three colours and transparent.
fn colour_block(b: &[u8], four: bool) -> [[u8; 4]; 16] {
    let (c0, c1) = (u16::from_le_bytes([b[0], b[1]]), u16::from_le_bytes([b[2], b[3]]));
    let (p0, p1) = (rgb565(c0), rgb565(c1));
    let mix = |a: u8, b: u8, wa: u16, wb: u16| ((a as u16 * wa + b as u16 * wb) / (wa + wb)) as u8;
    let mut pal = [[p0[0], p0[1], p0[2], 255], [p1[0], p1[1], p1[2], 255], [0; 4], [0; 4]];
    if four || c0 > c1 {
        pal[2] = [mix(p0[0], p1[0], 2, 1), mix(p0[1], p1[1], 2, 1), mix(p0[2], p1[2], 2, 1), 255];
        pal[3] = [mix(p0[0], p1[0], 1, 2), mix(p0[1], p1[1], 1, 2), mix(p0[2], p1[2], 1, 2), 255];
    } else {
        pal[2] = [mix(p0[0], p1[0], 1, 1), mix(p0[1], p1[1], 1, 1), mix(p0[2], p1[2], 1, 1), 255];
        pal[3] = [0, 0, 0, 0];
    }
    let bits = le32(b, 4);
    std::array::from_fn(|i| pal[(bits >> (2 * i) & 3) as usize])
}

/// The 16 alphas of a BC3 alpha block (8 bytes).
fn alpha_block(b: &[u8]) -> [u8; 16] {
    let (a0, a1) = (b[0] as u16, b[1] as u16);
    let mut pal = [0u16; 8];
    pal[0] = a0;
    pal[1] = a1;
    if a0 > a1 {
        for i in 1..7 {
            pal[i + 1] = ((7 - i as u16) * a0 + i as u16 * a1) / 7;
        }
    } else {
        for i in 1..5 {
            pal[i + 1] = ((5 - i as u16) * a0 + i as u16 * a1) / 5;
        }
        pal[6] = 0;
        pal[7] = 255;
    }
    let bits = b[2..8].iter().rev().fold(0u64, |acc, &x| acc << 8 | x as u64);
    std::array::from_fn(|i| pal[(bits >> (3 * i) & 7) as usize] as u8)
}

/// The 16 alphas of a BC2 alpha block (8 bytes, 4 bits each).
fn explicit_alpha(b: &[u8]) -> [u8; 16] {
    std::array::from_fn(|i| {
        let n = (b[i / 2] >> (4 * (i % 2))) & 15;
        n * 17
    })
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Bc1,
    Bc2,
    Bc3,
    /// Uncompressed 32-bit: the R, G, B, A masks.
    Raw([u32; 4]),
}

/// Decodes the first mip level of a DDS file to RGBA.
pub fn decode(dds: &[u8]) -> Result<Image> {
    if dds.len() < 128 || &dds[..4] != b"DDS " || le32(dds, 4) != 124 {
        return bad("not a DDS");
    }
    let (height, width) = (le32(dds, 12), le32(dds, 16));
    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return bad("DDS: implausible size");
    }
    let flags = le32(dds, 80);
    let fourcc = &dds[84..88];
    let mut start = 128;
    let kind = if flags & 4 != 0 {
        match fourcc {
            b"DXT1" => Kind::Bc1,
            b"DXT2" | b"DXT3" => Kind::Bc2,
            b"DXT4" | b"DXT5" => Kind::Bc3,
            b"DX10" => {
                if dds.len() < 148 {
                    return bad("DDS: short DX10 header");
                }
                start = 148;
                match le32(dds, 128) {
                    71 | 72 => Kind::Bc1,
                    74 | 75 => Kind::Bc2,
                    77 | 78 => Kind::Bc3,
                    28 | 29 => Kind::Raw([0xFF, 0xFF00, 0xFF_0000, 0xFF00_0000]),
                    87 | 91 => Kind::Raw([0xFF_0000, 0xFF00, 0xFF, 0xFF00_0000]),
                    f => return bad(format!("DDS: DXGI format {f} not read")),
                }
            }
            f => return bad(format!("DDS: format {:?} not read", String::from_utf8_lossy(f))),
        }
    } else if flags & 0x40 != 0 && le32(dds, 88) == 32 {
        let a = if flags & 1 != 0 { le32(dds, 104) } else { 0 };
        Kind::Raw([le32(dds, 92), le32(dds, 96), le32(dds, 100), a])
    } else {
        return bad("DDS: neither block-compressed nor 32-bit");
    };
    let (w, h) = (width as usize, height as usize);
    let mut rgba = vec![0u8; w * h * 4];
    match kind {
        Kind::Raw(masks) => {
            let need = start + w * h * 4;
            if dds.len() < need {
                return bad("DDS: pixels cut short");
            }
            let chan = |px: u32, m: u32| -> u8 {
                if m == 0 {
                    return 255;
                }
                let v = (px & m) >> m.trailing_zeros();
                let bits = (m >> m.trailing_zeros()).count_ones();
                if bits >= 8 {
                    (v >> (bits - 8)) as u8
                } else {
                    (v * 255 / ((1 << bits) - 1)) as u8
                }
            };
            for (i, px) in dds[start..need].chunks_exact(4).enumerate() {
                let p = u32::from_le_bytes([px[0], px[1], px[2], px[3]]);
                rgba[i * 4..i * 4 + 4].copy_from_slice(&[chan(p, masks[0]), chan(p, masks[1]), chan(p, masks[2]), chan(p, masks[3])]);
            }
        }
        _ => {
            let block = if kind == Kind::Bc1 { 8 } else { 16 };
            let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
            let need = start + bw * bh * block;
            if dds.len() < need {
                return bad("DDS: blocks cut short");
            }
            for by in 0..bh {
                for bx in 0..bw {
                    let b = &dds[start + (by * bw + bx) * block..][..block];
                    let (colours, alphas) = match kind {
                        Kind::Bc1 => (colour_block(b, false), None),
                        Kind::Bc2 => (colour_block(&b[8..], true), Some(explicit_alpha(b))),
                        _ => (colour_block(&b[8..], true), Some(alpha_block(b))),
                    };
                    for i in 0..16 {
                        let (x, y) = (bx * 4 + i % 4, by * 4 + i / 4);
                        if x >= w || y >= h {
                            continue;
                        }
                        let mut c = colours[i];
                        if let Some(a) = alphas {
                            c[3] = a[i];
                        }
                        rgba[(y * w + x) * 4..][..4].copy_from_slice(&c);
                    }
                }
            }
        }
    }
    Ok(Image { width, height, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(w: u32, h: u32, fourcc: &[u8; 4]) -> Vec<u8> {
        let mut d = vec![0u8; 128];
        d[..4].copy_from_slice(b"DDS ");
        d[4..8].copy_from_slice(&124u32.to_le_bytes());
        d[12..16].copy_from_slice(&h.to_le_bytes());
        d[16..20].copy_from_slice(&w.to_le_bytes());
        d[76..80].copy_from_slice(&32u32.to_le_bytes());
        d[80..84].copy_from_slice(&4u32.to_le_bytes());
        d[84..88].copy_from_slice(fourcc);
        d
    }

    #[test]
    fn bc3_block() {
        let mut d = header(4, 4, b"DXT5");
        // Alpha: 255 and 0, eight-alpha mode; texel 0 index 0 (255), texel 1 index 1 (0),
        // texel 2 index 2 (6/7 of 255 = 218), the rest index 7 (1/7 = 36).
        let idx: [u64; 16] = [0, 1, 2, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7];
        let bits = idx.iter().enumerate().fold(0u64, |acc, (i, &v)| acc | v << (3 * i));
        d.extend_from_slice(&[255, 0]);
        d.extend_from_slice(&bits.to_le_bytes()[..6]);
        // Colour: pure red and pure blue; texel 0 red, 1 blue, 2 two-thirds red, rest blue.
        d.extend_from_slice(&0xF800u16.to_le_bytes());
        d.extend_from_slice(&0x001Fu16.to_le_bytes());
        let cidx: [u32; 16] = [0, 1, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
        let cbits = cidx.iter().enumerate().fold(0u32, |acc, (i, &v)| acc | v << (2 * i));
        d.extend_from_slice(&cbits.to_le_bytes());
        let im = decode(&d).unwrap();
        assert_eq!((im.width, im.height), (4, 4));
        assert_eq!(&im.rgba[0..4], &[255, 0, 0, 255]);
        assert_eq!(&im.rgba[4..8], &[0, 0, 255, 0]);
        assert_eq!(&im.rgba[8..12], &[170, 0, 85, 218]);
        assert_eq!(&im.rgba[12..16], &[0, 0, 255, 36]);
    }

    #[test]
    fn bc1_transparent() {
        let mut d = header(4, 4, b"DXT1");
        // c0 <= c1: three colours and transparent; every texel index 3.
        d.extend_from_slice(&0x0000u16.to_le_bytes());
        d.extend_from_slice(&0xFFFFu16.to_le_bytes());
        d.extend_from_slice(&u32::MAX.to_le_bytes());
        let im = decode(&d).unwrap();
        assert!(im.rgba.chunks(4).all(|p| p == [0, 0, 0, 0]));
    }

    #[test]
    fn raw_bgra() {
        let mut d = header(1, 1, b"\0\0\0\0");
        d[80..84].copy_from_slice(&0x41u32.to_le_bytes());
        d[88..92].copy_from_slice(&32u32.to_le_bytes());
        for (at, m) in [(92, 0xFF_0000u32), (96, 0xFF00), (100, 0xFF), (104, 0xFF00_0000)] {
            d[at..at + 4].copy_from_slice(&m.to_le_bytes());
        }
        d.extend_from_slice(&[1, 2, 3, 4]);
        let im = decode(&d).unwrap();
        assert_eq!(im.rgba, vec![3, 2, 1, 4]);
    }

    #[test]
    fn rejects_other_files() {
        assert!(decode(b"PNG").is_err());
        assert!(decode(&header(4, 4, b"ATI2")).is_err());
    }
}
