//! A texture as B8G8R8A8 pixels: read from an uncompressed 32-bit DDS (a model's remastered
//! texture, `-1c<n>.dds`), or from a PS2 texture of a `.mdls` (`mdls::parse_textures`), and
//! sampled by UV (`texmap`).

/// A B8G8R8A8 image, rows from the top.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub w: u32,
    pub h: u32,
    pub bgra: Vec<u8>,
}

impl Image {
    /// An uncompressed 32-bit DDS's first level as B8G8R8A8 (whatever its masks), as
    /// `win::tint`'s `dds_bgra` reads one.
    pub fn from_dds(dds: &[u8]) -> Option<Image> {
        if dds.len() < 128 || &dds[..4] != b"DDS " {
            return None;
        }
        let u = |o: usize| u32::from_le_bytes(dds[o..o + 4].try_into().unwrap());
        let (h, w) = (u(12), u(16));
        let (flags, bits) = (u(80), u(88));
        let masks = [u(92), u(96), u(100), u(104)];
        // RGB (0x40), 32 bits, no four-cc (0x4).
        if flags & 0x40 == 0 || flags & 0x4 != 0 || bits != 32 || w == 0 || h == 0 || w > 8192 || h > 8192 {
            return None;
        }
        let n = (w * h) as usize;
        let data = dds.get(128..128 + n * 4)?;
        let chan = |v: u32, m: u32| if m == 0 { 255 } else { ((v & m) >> m.trailing_zeros()) as u8 };
        let mut bgra = Vec::with_capacity(n * 4);
        for px in data.chunks_exact(4) {
            let v = u32::from_le_bytes(px.try_into().unwrap());
            bgra.extend_from_slice(&[chan(v, masks[2]), chan(v, masks[1]), chan(v, masks[0]), chan(v, masks[3])]);
        }
        Some(Image { w, h, bgra })
    }

    /// Bilinear, wrapping; B, G, R as 0..255 floats.
    pub fn sample(&self, u: f32, v: f32) -> [f32; 3] {
        let (w, h) = (self.w as i64, self.h as i64);
        let x = u.rem_euclid(1.0) * self.w as f32 - 0.5;
        let y = v.rem_euclid(1.0) * self.h as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let px = |xi: i64, yi: i64| {
            let i = ((yi.rem_euclid(h) * w + xi.rem_euclid(w)) * 4) as usize;
            [self.bgra[i] as f32, self.bgra[i + 1] as f32, self.bgra[i + 2] as f32]
        };
        let (x0, y0) = (x0 as i64, y0 as i64);
        let (a, b, c, d) = (px(x0, y0), px(x0 + 1, y0), px(x0, y0 + 1), px(x0 + 1, y0 + 1));
        let mut out = [0.0; 3];
        for k in 0..3 {
            let top = a[k] + (b[k] - a[k]) * fx;
            let bot = c[k] + (d[k] - c[k]) * fx;
            out[k] = top + (bot - top) * fy;
        }
        out
    }

    /// The mean difference per colour channel (0..255) from `other` scaled to this one's size
    /// (each of this image's texels against the mean of the block of `other` it covers; `other` a
    /// whole multiple of this size). `None` if the sizes don't divide.
    pub fn difference_from_larger(&self, other: &Image) -> Option<f32> {
        if self.w == 0 || self.h == 0 || other.w % self.w != 0 || other.h % self.h != 0 {
            return None;
        }
        let (sx, sy) = ((other.w / self.w) as usize, (other.h / self.h) as usize);
        let (w, ow) = (self.w as usize, other.w as usize);
        let mut sum = 0f64;
        for y in 0..self.h as usize {
            for x in 0..w {
                for k in 0..3 {
                    let mut m = 0u32;
                    for dy in 0..sy {
                        for dx in 0..sx {
                            m += other.bgra[((y * sy + dy) * ow + x * sx + dx) * 4 + k] as u32;
                        }
                    }
                    let m = m as f64 / (sx * sy) as f64;
                    sum += (self.bgra[(y * w + x) * 4 + k] as f64 - m).abs();
                }
            }
        }
        Some((sum / (w * self.h as usize * 3) as f64) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dds_is_read_whatever_its_masks() {
        // A 2×1 A8B8G8R8 DDS (red in the low byte).
        let mut d = vec![0u8; 128 + 8];
        d[..4].copy_from_slice(b"DDS ");
        let put = |d: &mut Vec<u8>, o: usize, v: u32| d[o..o + 4].copy_from_slice(&v.to_le_bytes());
        put(&mut d, 12, 1);
        put(&mut d, 16, 2);
        put(&mut d, 80, 0x41);
        put(&mut d, 88, 32);
        put(&mut d, 92, 0xFF);
        put(&mut d, 96, 0xFF00);
        put(&mut d, 100, 0xFF_0000);
        put(&mut d, 104, 0xFF00_0000);
        put(&mut d, 128, 0x8040_2010);
        put(&mut d, 132, 0xFF00_00FF);
        let img = Image::from_dds(&d).unwrap();
        assert_eq!((img.w, img.h), (2, 1));
        assert_eq!(img.bgra, vec![0x40, 0x20, 0x10, 0x80, 0, 0, 0xFF, 0xFF]);
        // Bilinear between them, at the middle.
        let s = img.sample(0.5, 0.5);
        assert!((s[2] - (0x10 as f32 + 0xFF as f32) / 2.0).abs() < 0.01, "{s:?}");
    }

    #[test]
    fn differences_against_a_larger_copy() {
        let small = Image { w: 1, h: 1, bgra: vec![10, 20, 30, 255] };
        let big = Image { w: 2, h: 2, bgra: [[0, 20, 40, 255], [20, 20, 20, 255], [10, 20, 30, 0], [10, 20, 30, 0]].concat() };
        assert_eq!(small.difference_from_larger(&big), Some(0.0));
        assert_eq!(big.difference_from_larger(&small), None);
    }
}
