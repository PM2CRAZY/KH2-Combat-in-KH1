//! A small PNG reader for the remastered (HD) images the PC packages carry beside the PS2 data
//! (the 2D files' `IMGD` images come as 8-bit RGBA PNGs). Only what those use: 8-bit RGBA or
//! RGB, not interlaced. Our own code, written from the PNG specification.

use crate::{bad, Result};
use std::io::Read;

pub struct Image {
    pub width: u32,
    pub height: u32,
    /// `width × height × 4` bytes, R G B A.
    pub rgba: Vec<u8>,
}

fn be32(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i16 + b as i16 - c as i16;
    let (pa, pb, pc) = ((p - a as i16).abs(), (p - b as i16).abs(), (p - c as i16).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

pub fn decode(png: &[u8]) -> Result<Image> {
    if png.len() < 8 || &png[..8] != b"\x89PNG\r\n\x1a\n" {
        return bad("not a PNG");
    }
    let (mut width, mut height, mut channels) = (0u32, 0u32, 0usize);
    let mut idat = Vec::new();
    let mut p = 8;
    while p + 8 <= png.len() {
        let n = be32(&png[p..]) as usize;
        let kind = &png[p + 4..p + 8];
        let Some(data) = png.get(p + 8..p + 8 + n) else { return bad("PNG chunk out of bounds") };
        match kind {
            b"IHDR" => {
                if n < 13 {
                    return bad("short IHDR");
                }
                width = be32(data);
                height = be32(&data[4..]);
                let (depth, colour, interlace) = (data[8], data[9], data[12]);
                channels = match (depth, colour, interlace) {
                    (8, 6, 0) => 4,
                    (8, 2, 0) => 3,
                    _ => return bad(format!("PNG not read here (depth {depth}, colour type {colour}, interlace {interlace})")),
                };
            }
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => break,
            _ => {}
        }
        p += 12 + n;
    }
    if channels == 0 || width == 0 || height == 0 || width > 8192 || height > 8192 {
        return bad("PNG without a usable header");
    }
    let stride = width as usize * channels;
    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    flate2::read::ZlibDecoder::new(&idat[..]).read_to_end(&mut raw)?;
    if raw.len() < (stride + 1) * height as usize {
        return bad("PNG data too short");
    }
    let mut out = vec![0u8; stride * height as usize];
    for y in 0..height as usize {
        let filter = raw[y * (stride + 1)];
        let line = &raw[y * (stride + 1) + 1..(y + 1) * (stride + 1)];
        for x in 0..stride {
            let a = if x >= channels { out[y * stride + x - channels] } else { 0 };
            let b = if y > 0 { out[(y - 1) * stride + x] } else { 0 };
            let c = if x >= channels && y > 0 { out[(y - 1) * stride + x - channels] } else { 0 };
            out[y * stride + x] = line[x].wrapping_add(match filter {
                0 => 0,
                1 => a,
                2 => b,
                3 => ((a as u16 + b as u16) / 2) as u8,
                4 => paeth(a, b, c),
                f => return bad(format!("PNG filter {f}")),
            });
        }
    }
    let rgba = if channels == 4 { out } else { out.chunks_exact(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect() };
    Ok(Image { width, height, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        out.extend_from_slice(&[0; 4]); // (the CRC isn't checked)
    }

    #[test]
    fn decodes_filtered_rgba() {
        // 2×2: row 0 with Sub, row 1 with Up.
        let rows: [&[u8]; 2] = [&[1, 10, 20, 30, 255, 5, 5, 5, 0], &[2, 1, 1, 1, 0, 2, 2, 2, 0]];
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        for r in rows {
            z.write_all(r).unwrap();
        }
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&2u32.to_be_bytes());
        ihdr.extend_from_slice(&2u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        chunk(&mut png, b"IHDR", &ihdr);
        chunk(&mut png, b"IDAT", &z.finish().unwrap());
        chunk(&mut png, b"IEND", &[]);
        let im = decode(&png).unwrap();
        assert_eq!((im.width, im.height), (2, 2));
        assert_eq!(&im.rgba[..8], &[10, 20, 30, 255, 15, 25, 35, 255]);
        assert_eq!(&im.rgba[8..], &[11, 21, 31, 255, 17, 27, 37, 255]);
    }
}
