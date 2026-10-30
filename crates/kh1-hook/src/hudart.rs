//! KH2's Drive gauge art, built once from the player's own KH2 files into the mod folder (the owner,
//! the main-branch port's round 3: "go ahead", answer 1 (a) KH2's art only; the tester's rule "the
//! game's assets are built once, kept in the mod folder"). Notes: `docs/kh1-drive-forms-native.md`
//! §14, `docs/kh1-asset-cache.md`.
//!
//! - **The source** (Checked (files)): KH2's field HUD `field2d/<lang>/zz0field.2dd` is a BAR of
//!   images (type 24) and sequences (type 25) `fiel`, `gaug`, `mapj`, `miss`; the Drive gauge is the
//!   `gaug` sequence (SEQD: its sprites, parts, groups and animations) on the second image, whose HD
//!   picture is the second remastered PNG (`US_zz0field_2dd1.png`, 1024 × 512, 8-bit RGBA). The
//!   image entry (IMGD) gives the PS2 picture's size at `+0x1C` (u16 width, u16 height: 512 × 256
//!   for `gaug`, Checked (files)), the size the sequence's texture rectangles are in; KH1's HUD
//!   textures are made with their PS2 size too (`0x140274060` from the TIM2's, Checked (decomp)).
//! - **The cache** (`kh2-hud-gauge.bin` in `kh1-kh2-cache`, the slot cache's file layout): a manifest
//!   (this format, the mod's version, the game build, the source's PKG entry and the picture's size)
//!   and two blocks, the `gaug` SEQD as it is and the picture as B8G8R8A8 pixels, ready for KH1's
//!   texture (the PNG is never decoded again).
//! - The PNG is read by our own decoder (`png_rgba`: zlib inflate and the five row filters, 8-bit
//!   RGB / RGBA only, no interlace), so nothing new is linked in.
//!
//! Pure Rust (no Windows calls), unit tested on any host.

use crate::slotcache::{self, Block, Manifest};
use std::path::{Path, PathBuf};

/// Bump when the cache's output changes for the same inputs without the mod's version changing.
pub const ART_FORMAT: u32 = 2;
pub const FILE_NAME: &str = "kh2-hud-gauge.bin";
/// The BAR entry types.
const BAR_IMAGE: u16 = 24;
const BAR_SEQD: u16 = 25;

/// A BAR's entries: (type, name, bytes).
pub fn bar_entries(b: &[u8]) -> Result<Vec<(u16, String, &[u8])>, String> {
    if b.len() < 16 || &b[..3] != b"BAR" {
        return Err("not a BAR".into());
    }
    let n = u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize;
    let mut out = Vec::new();
    for i in 0..n {
        let e = 16 + i * 16;
        if e + 16 > b.len() {
            return Err("BAR entry table cut short".into());
        }
        let ty = u16::from_le_bytes([b[e], b[e + 1]]);
        let name = String::from_utf8_lossy(&b[e + 4..e + 8]).trim_end_matches('\0').to_string();
        let off = u32::from_le_bytes(b[e + 8..e + 12].try_into().unwrap()) as usize;
        let len = u32::from_le_bytes(b[e + 12..e + 16].try_into().unwrap()) as usize;
        let data = b.get(off..off + len).ok_or_else(|| format!("BAR entry {i} out of the file"))?;
        out.push((ty, name, data));
    }
    Ok(out)
}

/// The `gaug` image's PS2 size (its IMGD's `+0x1C`: width, height).
pub fn image_size(bar: &[u8]) -> Result<(u16, u16), String> {
    let entries = bar_entries(bar)?;
    let img = entries.iter().find(|e| e.0 == BAR_IMAGE && e.1 == "gaug").ok_or("no gaug image")?.2;
    if img.len() < 0x20 || &img[..4] != b"IMGD" {
        return Err("the gaug image isn't an IMGD".into());
    }
    let (w, h) = (u16::from_le_bytes([img[0x1C], img[0x1D]]), u16::from_le_bytes([img[0x1E], img[0x1F]]));
    if w == 0 || h == 0 {
        return Err("the gaug image has no size".into());
    }
    Ok((w, h))
}

/// The `gaug` sequence's bytes and its image's index among the BAR's images (the remastered PNGs'
/// order), or why not.
pub fn gauge_sequence(bar: &[u8]) -> Result<(&[u8], usize), String> {
    let entries = bar_entries(bar)?;
    let seqd = entries.iter().find(|e| e.0 == BAR_SEQD && e.1 == "gaug").ok_or("no gaug sequence")?.2;
    // (Each sequence's image is the image entry of the same name.)
    let image = entries.iter().filter(|e| e.0 == BAR_IMAGE).position(|e| e.1 == "gaug").ok_or("no gaug image")?;
    Ok((seqd, image))
}

/// What the cache is made from.
#[derive(Clone, Debug)]
pub struct Desc {
    pub build: String,
    /// `zz0field.2dd`'s PKG entry (`kh2_data::Install::stamp_any`: archive, entry offset and lengths,
    /// the `.pkg`'s size and time), which holds its HD pictures too: a start needn't read it.
    pub source: String,
}

impl Desc {
    fn manifest(&self, w: u32, h: u32, ps2: (u16, u16)) -> Manifest {
        let mut m = Manifest::default();
        m.set("hud art format", ART_FORMAT.to_string());
        m.set("mod version", crate::VERSION.to_string());
        m.set("game build", self.build.clone());
        m.set("zz0field.2dd entry", self.source.clone());
        m.set("picture size", format!("{w}x{h}"));
        m.set("ps2 size", format!("{}x{}", ps2.0, ps2.1));
        m
    }
}

/// The art: the `gaug` SEQD and the picture (B8G8R8A8, `w` × `h`), and the PS2 picture's size
/// (`ps2`, what the sequence's texture rectangles are in).
pub struct Art {
    pub seqd: Block,
    pub pixels: Block,
    pub w: u32,
    pub h: u32,
    pub ps2: (u16, u16),
}

/// What `build` makes.
pub struct Built {
    pub seqd: Vec<u8>,
    pub pixels: Vec<u8>,
    pub w: u32,
    pub h: u32,
    pub ps2: (u16, u16),
}

/// Builds the art from the BAR and the PNG.
pub fn build(bar: &[u8], png: &[u8]) -> Result<Built, String> {
    let (seqd, _) = gauge_sequence(bar)?;
    if seqd.len() < 4 || &seqd[..4] != b"SEQD" {
        return Err("the gaug entry isn't a SEQD".into());
    }
    let ps2 = image_size(bar)?;
    let (w, h, mut px) = png_rgba(png)?;
    rgba_to_bgra(&mut px);
    Ok(Built { seqd: seqd.to_vec(), pixels: px, w, h, ps2 })
}

/// Writes the art into `dir`.
pub fn store(dir: &Path, d: &Desc, b: &Built) -> std::io::Result<PathBuf> {
    slotcache::write_named_blocks(dir, FILE_NAME, &d.manifest(b.w, b.h, b.ps2), &[(0u128, &b.seqd[..]), (1u128, &b.pixels[..])])
}

/// "WxH" as numbers.
fn size_of(m: &Manifest, key: &str) -> Option<(u32, u32)> {
    let v = m.text().lines().find_map(|l| l.strip_prefix(key).and_then(|r| r.strip_prefix(" = ")).map(str::to_string))?;
    let (a, b) = v.split_once('x')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

/// The art in `dir` if it was made from these sources (`None`: missing, damaged or changed; the
/// reason as text).
pub fn load(dir: &Path, d: &Desc) -> Result<Art, String> {
    let b = std::fs::read(dir.join(FILE_NAME)).map_err(|_| "none yet".to_string())?;
    let (m, mut blocks) = slotcache::decode_shared(b)?;
    let (w, h) = size_of(&m, "picture size").ok_or("no picture size")?;
    let ps2 = size_of(&m, "ps2 size").filter(|s| s.0 > 0 && s.1 > 0 && s.0 < 0x10000 && s.1 < 0x10000).ok_or("no ps2 size")?;
    let ps2 = (ps2.0 as u16, ps2.1 as u16);
    let differ = d.manifest(w, h, ps2).differences(&m);
    if !differ.is_empty() {
        return Err(format!("changed: {}", differ.join(", ")));
    }
    if blocks.len() != 2 || blocks[1].1.len() != w as usize * h as usize * 4 {
        return Err("damaged blocks".into());
    }
    let pixels = blocks.pop().unwrap().1;
    let seqd = blocks.pop().unwrap().1;
    Ok(Art { seqd, pixels, w, h, ps2 })
}

/// A DDS header (128 bytes) for `w` × `h` B8G8R8A8 pixels, one level: the uncompressed layout
/// KH1's DDS loader takes as `DXGI_FORMAT_B8G8R8A8_UNORM` (0x57; pixel format size 0x20, flags 0x41,
/// 32 bits, masks R 0xFF0000, G 0xFF00, B 0xFF, A 0xFF000000; the data at +0x80; no mip levels
/// without flag 0x20000; Steam `0x1400FA160`, the entry texture's maker, Checked
/// (decomp)).
pub fn dds_header(w: u32, h: u32) -> [u8; 128] {
    let mut d = [0u32; 32];
    d[0] = 0x2053_4444;
    d[1] = 124;
    // (Caps, height, width, pitch, pixel format.)
    d[2] = 0x1 | 0x2 | 0x4 | 0x8 | 0x1000;
    d[3] = h;
    d[4] = w;
    d[5] = w * 4;
    d[7] = 1;
    d[0x13] = 0x20;
    d[0x14] = 0x41;
    d[0x16] = 32;
    d[0x17] = 0x00FF_0000;
    d[0x18] = 0x0000_FF00;
    d[0x19] = 0x0000_00FF;
    d[0x1A] = 0xFF00_0000;
    d[0x1B] = 0x1000;
    let mut out = [0u8; 128];
    for (i, v) in d.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    out
}

fn rgba_to_bgra(px: &mut [u8]) {
    for p in px.chunks_exact_mut(4) {
        p.swap(0, 2);
    }
}

// ------------------------------------------------------------------------------------ the PNG

/// An 8-bit RGB or RGBA PNG (no interlace) as RGBA pixels: (width, height, pixels).
pub fn png_rgba(b: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if b.len() < 8 || b[..8] != [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return Err("not a PNG".into());
    }
    let (mut w, mut h, mut kind) = (0u32, 0u32, 0u8);
    let mut idat = Vec::new();
    let mut at = 8;
    while at + 8 <= b.len() {
        let len = u32::from_be_bytes(b[at..at + 4].try_into().unwrap()) as usize;
        let ty = &b[at + 4..at + 8];
        let data = b.get(at + 8..at + 8 + len).ok_or("PNG chunk cut short")?;
        match ty {
            b"IHDR" => {
                if data.len() < 13 {
                    return Err("short IHDR".into());
                }
                w = u32::from_be_bytes(data[0..4].try_into().unwrap());
                h = u32::from_be_bytes(data[4..8].try_into().unwrap());
                let (depth, interlace) = (data[8], data[12]);
                kind = data[9];
                if depth != 8 || interlace != 0 || !(kind == 2 || kind == 6) {
                    return Err(format!("PNG kind {kind}, depth {depth}, interlace {interlace}: only 8-bit RGB / RGBA"));
                }
            }
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => break,
            _ => {}
        }
        at += 12 + len;
    }
    if w == 0 || h == 0 {
        return Err("no IHDR".into());
    }
    let raw = zlib_inflate(&idat)?;
    let bpp = if kind == 6 { 4 } else { 3 };
    let stride = w as usize * bpp;
    if raw.len() < (stride + 1) * h as usize {
        return Err("PNG data cut short".into());
    }
    let mut cur = vec![0u8; stride];
    let mut prev = vec![0u8; stride];
    let mut out = Vec::with_capacity(w as usize * h as usize * 4);
    for y in 0..h as usize {
        let row = &raw[y * (stride + 1)..(y + 1) * (stride + 1)];
        let f = row[0];
        for i in 0..stride {
            let x = row[1 + i];
            let a = if i >= bpp { cur[i - bpp] as i32 } else { 0 };
            let up = prev[i] as i32;
            let c = if i >= bpp { prev[i - bpp] as i32 } else { 0 };
            let v = match f {
                0 => x as i32,
                1 => x as i32 + a,
                2 => x as i32 + up,
                3 => x as i32 + (a + up) / 2,
                4 => {
                    let p = a + up - c;
                    let (pa, pb, pc) = ((p - a).abs(), (p - up).abs(), (p - c).abs());
                    x as i32 + if pa <= pb && pa <= pc { a } else if pb <= pc { up } else { c }
                }
                _ => return Err(format!("PNG filter {f}")),
            };
            cur[i] = v as u8;
        }
        for px in cur.chunks_exact(bpp) {
            out.extend_from_slice(&px[..3]);
            out.push(if bpp == 4 { px[3] } else { 255 });
        }
        std::mem::swap(&mut cur, &mut prev);
    }
    Ok((w, h, out))
}

/// zlib's inflate (RFC 1950 / 1951).
pub fn zlib_inflate(b: &[u8]) -> Result<Vec<u8>, String> {
    if b.len() < 2 || b[0] & 0x0F != 8 || (u16::from(b[0]) << 8 | u16::from(b[1])) % 31 != 0 {
        return Err("not zlib data".into());
    }
    inflate(&b[2..])
}

struct Bits<'a> {
    b: &'a [u8],
    at: usize,
    bit: u32,
    n: u32,
}

impl Bits<'_> {
    fn need(&mut self, k: u32) -> Result<(), String> {
        while self.n < k {
            let byte = *self.b.get(self.at).ok_or("deflate data cut short")?;
            self.at += 1;
            self.bit |= (byte as u32) << self.n;
            self.n += 8;
        }
        Ok(())
    }
    fn get(&mut self, k: u32) -> Result<u32, String> {
        if k == 0 {
            return Ok(0);
        }
        self.need(k)?;
        let v = self.bit & ((1u32 << k) - 1);
        self.bit >>= k;
        self.n -= k;
        Ok(v)
    }
}

/// A canonical Huffman code: counts per length and the symbols in order.
struct Huff {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl Huff {
    fn new(lengths: &[u8]) -> Huff {
        let mut counts = [0u16; 16];
        for &l in lengths {
            counts[l as usize] += 1;
        }
        counts[0] = 0;
        let mut offs = [0u16; 16];
        for i in 1..16 {
            offs[i] = offs[i - 1] + counts[i - 1];
        }
        let mut symbols = vec![0u16; lengths.len()];
        for (s, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbols[offs[l as usize] as usize] = s as u16;
                offs[l as usize] += 1;
            }
        }
        Huff { counts, symbols }
    }
    fn decode(&self, bits: &mut Bits) -> Result<u16, String> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..16 {
            code |= bits.get(1)? as i32;
            let count = self.counts[len] as i32;
            if code - count < first {
                return self.symbols.get((index + code - first) as usize).copied().ok_or_else(|| "bad Huffman code".to_string());
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err("bad Huffman code".into())
    }
}

const LEN_BASE: [u16; 29] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LEN_EXTRA: [u8; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DIST_BASE: [u16; 30] = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577];
const DIST_EXTRA: [u8; 30] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];

/// Raw deflate.
pub fn inflate(b: &[u8]) -> Result<Vec<u8>, String> {
    let mut bits = Bits { b, at: 0, bit: 0, n: 0 };
    let mut out: Vec<u8> = Vec::new();
    loop {
        let last = bits.get(1)?;
        match bits.get(2)? {
            0 => {
                bits.bit = 0;
                bits.n = 0;
                let at = bits.at;
                let hdr = b.get(at..at + 4).ok_or("stored block cut short")?;
                let len = u16::from_le_bytes([hdr[0], hdr[1]]) as usize;
                out.extend_from_slice(b.get(at + 4..at + 4 + len).ok_or("stored block cut short")?);
                bits.at = at + 4 + len;
            }
            1 => {
                let mut l = [0u8; 288];
                l[..144].fill(8);
                l[144..256].fill(9);
                l[256..280].fill(7);
                l[280..].fill(8);
                block(&mut bits, &mut out, &Huff::new(&l), &Huff::new(&[5u8; 30]))?;
            }
            2 => {
                let hlit = bits.get(5)? as usize + 257;
                let hdist = bits.get(5)? as usize + 1;
                let hclen = bits.get(4)? as usize + 4;
                const ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
                let mut cl = [0u8; 19];
                for &o in ORDER.iter().take(hclen) {
                    cl[o] = bits.get(3)? as u8;
                }
                let clh = Huff::new(&cl);
                let mut lens = vec![0u8; hlit + hdist];
                let mut i = 0;
                while i < lens.len() {
                    let s = clh.decode(&mut bits)?;
                    match s {
                        0..=15 => {
                            lens[i] = s as u8;
                            i += 1;
                        }
                        16 => {
                            let prev = *lens.get(i.wrapping_sub(1)).ok_or("repeat with nothing before")?;
                            for _ in 0..3 + bits.get(2)? {
                                *lens.get_mut(i).ok_or("lengths overrun")? = prev;
                                i += 1;
                            }
                        }
                        17 => i += 3 + bits.get(3)? as usize,
                        _ => i += 11 + bits.get(7)? as usize,
                    }
                }
                if i > lens.len() {
                    return Err("lengths overrun".into());
                }
                block(&mut bits, &mut out, &Huff::new(&lens[..hlit]), &Huff::new(&lens[hlit..]))?;
            }
            _ => return Err("bad deflate block".into()),
        }
        if last == 1 {
            return Ok(out);
        }
    }
}

fn block(bits: &mut Bits, out: &mut Vec<u8>, lit: &Huff, dist: &Huff) -> Result<(), String> {
    loop {
        let s = lit.decode(bits)? as usize;
        match s {
            0..=255 => out.push(s as u8),
            256 => return Ok(()),
            _ => {
                let k = s - 257;
                let len = *LEN_BASE.get(k).ok_or("bad length")? as usize + bits.get(LEN_EXTRA[k] as u32)? as usize;
                let d = dist.decode(bits)? as usize;
                let back = *DIST_BASE.get(d).ok_or("bad distance")? as usize + bits.get(DIST_EXTRA[d] as u32)? as usize;
                if back > out.len() {
                    return Err("distance before the start".into());
                }
                let from = out.len() - back;
                for j in 0..len {
                    let v = out[from + j];
                    out.push(v);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2×1 RGBA PNG (filter 1, red then green), made with Python's zlib for these tests.
    #[rustfmt::skip]
    const TEST_PNG: [u8; 74] = [137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 1, 8, 6, 0, 0, 0, 244, 34, 127, 138, 0, 0, 0, 17, 73, 68, 65, 84, 120, 156, 99, 252, 207, 192, 240, 159, 241, 63, 3, 3, 0, 16, 6, 3, 0, 56, 183, 54, 200, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130];

    #[test]
    fn inflate_stored_and_fixed() {
        // zlib of b"hello hello hello" (fixed Huffman), and a stored block.
        let z = [120, 218, 203, 72, 205, 201, 201, 87, 200, 64, 144, 0, 58, 46, 6, 125];
        assert_eq!(zlib_inflate(&z).unwrap(), b"hello hello hello");
        let stored = [0x01, 0x03, 0x00, 0xfc, 0xff, b'a', b'b', b'c'];
        assert_eq!(inflate(&stored).unwrap(), b"abc");
    }

    #[test]
    fn png_two_by_one() {
        // A 2×1 RGBA PNG (filter 1), red then green, made by Python's zlib.
        let png = &TEST_PNG;
        let (w, h, px) = png_rgba(png).unwrap();
        assert_eq!((w, h), (2, 1));
        assert_eq!(px, vec![255, 0, 0, 255, 0, 255, 0, 255]);
    }

    #[test]
    fn dds_header_is_kh1s_bgra() {
        let h = dds_header(1024, 512);
        let w = |i: usize| u32::from_le_bytes(h[i * 4..i * 4 + 4].try_into().unwrap());
        assert_eq!((w(0), w(1), w(3), w(4)), (0x2053_4444, 124, 512, 1024));
        assert_eq!((w(0x13), w(0x14), w(0x15), w(0x16)), (0x20, 0x41, 0, 32));
        assert_eq!((w(0x17), w(0x18), w(0x19), w(0x1A)), (0xFF_0000, 0xFF00, 0xFF, 0xFF00_0000));
        assert_eq!(w(2) & 0x20000, 0);
    }

    #[test]
    fn bar_and_cache_round_trip() {
        let mut bar = b"BAR\x01".to_vec();
        bar.extend_from_slice(&2u32.to_le_bytes());
        bar.extend_from_slice(&[0; 8]);
        let seqd = b"SEQD\0\0\0\0".to_vec();
        let mut img = vec![9u8; 0x20];
        img[..4].copy_from_slice(b"IMGD");
        img[0x1C..0x20].copy_from_slice(&[0, 2, 0, 1]);
        let off0 = 16 + 32;
        for (ty, name, off, len) in [(24u16, *b"gaug", off0, img.len()), (25u16, *b"gaug", off0 + img.len(), seqd.len())] {
            bar.extend_from_slice(&ty.to_le_bytes());
            bar.extend_from_slice(&[0, 0]);
            bar.extend_from_slice(&name);
            bar.extend_from_slice(&(off as u32).to_le_bytes());
            bar.extend_from_slice(&(len as u32).to_le_bytes());
        }
        bar.extend_from_slice(&img);
        bar.extend_from_slice(&seqd);
        let (s, i) = gauge_sequence(&bar).unwrap();
        assert_eq!((s, i), (&seqd[..], 0));
        let b = build(&bar, &TEST_PNG).unwrap();
        assert_eq!(b.pixels, vec![0, 0, 255, 255, 0, 255, 0, 255]);
        assert_eq!(b.ps2, (512, 256));
        let dir = std::env::temp_dir().join(format!("kh1kh2-hudart-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let d = Desc { build: "Steam".into(), source: "kh2_first @10 1/2 pkg 3 4".into() };
        assert!(load(&dir, &d).is_err());
        store(&dir, &d, &b).unwrap();
        let a = load(&dir, &d).unwrap();
        assert_eq!((&*a.seqd, &*a.pixels, a.w, a.h, a.ps2), (&b.seqd[..], &b.pixels[..], 2, 1, (512, 256)));
        assert!(load(&dir, &Desc { source: "kh2_first @20 1/2 pkg 3 4".into(), ..d }).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
