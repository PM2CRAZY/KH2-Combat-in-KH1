//! Everything loaded once: the PAX elements, the DPDs they play, the shared
//! `texcommon.dpd`, and every texture decoded to RGBA8.
//!
//! ## Textures
//!
//! Shapes and models name their texture with a PS2 GS `TEX0` register
//! (base block `TBP`, buffer width, size, palette block `CBP`). We rebuild
//! the image the GS would sample (`docs/kh2-effects.md` §4, §9):
//!
//! - `TBP < 0x3000` reads pixels from `texcommon.dpd`, else from the
//!   effect's own DPD; `CBP < 0x3E00` likewise for the palette.
//! - The DPD textures are laid out in a virtual GS memory: each is uploaded
//!   into the buffer starting at its `tbp` (its own buffer width) at its
//!   (x, y), one 128×64 page (8-bit) per 32 blocks; the `TEX0` rectangle is
//!   read back through its own base and width. Small textures share pages
//!   side by side (texcommon packs four into the page at 0x2C40); the
//!   `TEX0`'s palette is applied to the whole rectangle, as on the GS.
//! - Palettes are the texture whose `cbp` equals `TEX0.CBP` plus the
//!   sprite's palette offset; 256-colour palettes are in GS CSM1 order.
//! - Alpha 0x80 is opaque and is doubled to 0xFF.
//!
//! Every `TEX0` used by any shape or model in the package is decoded at
//! load and gets a stable [`TextureId`] (its index in
//! [`Library::textures`]). The PC remaster swaps in HD images for some
//! textures; those are not read here, so these are the PS2 originals.

use std::collections::HashMap;

use kh2_formats::bar::Bar;
use kh2_formats::dpd::{Dpd, Tex0};
use kh2_formats::pax::{Pax, PaxElement};

/// Index into [`Library::textures`].
pub type TextureId = u32;

/// A decoded texture: RGBA8, rows top-down, `width * height * 4` bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Errors while loading.
#[derive(Debug)]
pub enum LoadError {
    /// Neither a PAX nor a BAR holding one.
    NoPax,
    Format(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::NoPax => write!(f, "no PAX found"),
            LoadError::Format(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<kh2_formats::Error> for LoadError {
    fn from(e: kh2_formats::Error) -> Self {
        LoadError::Format(e.to_string())
    }
}

/// Where a decoded texture comes from (the GS `TEX0` fields it was built
/// for), e.g. to match it with a remastered replacement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureSource {
    /// Pixels from the effect's own DPD (else `texcommon.dpd`).
    pub local_pixels: bool,
    /// Palette from the effect's own DPD (else `texcommon.dpd`).
    pub local_palette: bool,
    pub tbp: u16,
    pub tbw: u8,
    pub psm: u8,
    pub tw: u8,
    pub th: u8,
    pub cbp: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct TexKey {
    /// DPD index for local pixels (None = texcommon).
    pix: Option<usize>,
    /// DPD index for the local palette (None = texcommon).
    pal: Option<usize>,
    tbp: u16,
    tbw: u8,
    psm: u8,
    tw: u8,
    th: u8,
    cbp: u16,
}

/// The loaded effect package.
pub struct Library {
    pub(crate) elements: Vec<PaxElement>,
    pub(crate) dpds: Vec<Dpd>,
    /// Effect number → (DPD index, particle data index).
    pub(crate) effects: HashMap<u32, (usize, usize)>,
    common: Option<Dpd>,
    textures: Vec<Texture>,
    sources: Vec<TexKey>,
    tex_ids: HashMap<TexKey, TextureId>,
    warnings: Vec<String>,
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap()))
}

/// The PAX inside `buf`: the buffer itself, or the named (else first) PAX
/// entry of a BAR.
fn find_pax<'a>(buf: &'a [u8], name: Option<&str>) -> Option<&'a [u8]> {
    if buf.starts_with(b"PAX_") {
        return Some(buf);
    }
    let bar = Bar::parse(buf).ok()?;
    let mut paxes = bar.entries.iter().filter(|e| e.data.starts_with(b"PAX_"));
    match name {
        Some(n) => paxes.find(|e| e.name == n).map(|e| e.data),
        None => paxes.next().map(|e| e.data),
    }
}

impl Library {
    /// Load from a PAX (or a BAR holding one, e.g. Sora's `P_EX100.a.us`)
    /// and, for effects that use shared textures, `effect/texcommon.dpd`.
    pub fn load(pax_or_bar: &[u8], texcommon: Option<&[u8]>) -> Result<Library, LoadError> {
        Library::load_named(pax_or_bar, None, texcommon)
    }

    /// Like [`Library::load`], picking the BAR entry by name (`"p_ex"`).
    pub fn load_named(pax_or_bar: &[u8], name: Option<&str>, texcommon: Option<&[u8]>) -> Result<Library, LoadError> {
        let pax_buf = find_pax(pax_or_bar, name).ok_or(LoadError::NoPax)?;
        let pax = Pax::parse(pax_buf)?;
        // DPX: count at +12, 32-byte records (DPD offset, data index, number).
        let dpx_at = u32_at(pax_buf, 12).ok_or(LoadError::NoPax)? as usize;
        let dpx = pax_buf.get(dpx_at..).ok_or_else(|| LoadError::Format("DPX out of range".into()))?;
        let n = u32_at(dpx, 12).unwrap_or(0) as usize;
        if n > 4096 {
            return Err(LoadError::Format("implausible DPX count".into()));
        }
        let mut dpd_offsets: Vec<usize> = Vec::new();
        let mut effects = HashMap::new();
        for i in 0..n {
            let r = 16 + i * 32;
            let (Some(off), Some(index), Some(number)) = (u32_at(dpx, r), u32_at(dpx, r + 4), u32_at(dpx, r + 8)) else {
                break;
            };
            let off = off as usize;
            let k = match dpd_offsets.iter().position(|&o| o == off) {
                Some(k) => k,
                None => {
                    dpd_offsets.push(off);
                    dpd_offsets.len() - 1
                }
            };
            effects.entry(number).or_insert((k, index as usize));
        }
        // A DPD that does not parse is left empty (its effects cannot start)
        // so the rest of the package still loads.
        let mut dpds = Vec::with_capacity(dpd_offsets.len());
        let mut warnings = Vec::new();
        for &off in &dpd_offsets {
            match dpx.get(off..).ok_or_else(|| "out of range".to_string()).and_then(|b| Dpd::parse(b).map_err(|e| e.to_string())) {
                Ok(d) => dpds.push(d),
                Err(e) => {
                    warnings.push(format!("DPD at DPX+{off:#x}: {e}"));
                    dpds.push(Dpd { particles: vec![], textures: vec![], shapes: vec![], models: vec![], vertex_sets: vec![], vram: [0; 4] });
                }
            }
        }
        effects.retain(|_, (k, i)| *i < dpds[*k].particles.len());
        let common = match texcommon {
            Some(b) => Some(Dpd::parse(b)?),
            None => None,
        };
        let mut lib = Library { elements: pax.elements, dpds, effects, common, textures: Vec::new(), sources: Vec::new(), tex_ids: HashMap::new(), warnings };
        lib.decode_all();
        Ok(lib)
    }

    /// The PAX elements (what motions start by id).
    pub fn elements(&self) -> &[PaxElement] {
        &self.elements
    }

    /// The first element with this id (the one the game starts).
    pub fn element(&self, id: u16) -> Option<&PaxElement> {
        self.elements.iter().find(|e| e.id == id)
    }

    /// Every decoded texture; a [`TextureId`] indexes this.
    pub fn textures(&self) -> &[Texture] {
        &self.textures
    }

    pub fn texture(&self, id: TextureId) -> Option<&Texture> {
        self.textures.get(id as usize)
    }

    /// The `TEX0` a texture was decoded for.
    pub fn texture_source(&self, id: TextureId) -> Option<TextureSource> {
        self.sources.get(id as usize).map(|k| TextureSource {
            local_pixels: k.pix.is_some(),
            local_palette: k.pal.is_some(),
            tbp: k.tbp,
            tbw: k.tbw,
            psm: k.psm,
            tw: k.tw,
            th: k.th,
            cbp: k.cbp,
        })
    }

    /// Problems met while loading that did not stop it (e.g. a DPD that
    /// failed to parse; its effects cannot start).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Whether `texcommon.dpd` was given.
    pub fn has_common(&self) -> bool {
        self.common.is_some()
    }

    /// The DPDs of the package (in DPX order of first use).
    pub fn dpds(&self) -> &[Dpd] {
        &self.dpds
    }

    /// (DPD index, particle data index) of DPX effect `number`.
    pub fn effect_data(&self, number: u32) -> Option<(usize, usize)> {
        self.effects.get(&number).copied()
    }

    /// Program types used by element `id` that this crate does not run,
    /// with how many program records use each.
    pub fn unsupported_programs(&self, id: u16) -> Vec<(u32, &'static str, usize)> {
        let mut out: Vec<(u32, &'static str, usize)> = Vec::new();
        let Some(el) = self.element(id) else { return out };
        let Some((d, p)) = self.effect_data(el.effect as u32) else { return out };
        for e in &self.dpds[d].particles[p].emitters {
            for pr in &e.programs {
                if !crate::player::supported(pr.kind) {
                    match out.iter_mut().find(|o| o.0 == pr.kind) {
                        Some(o) => o.2 += 1,
                        None => out.push((pr.kind, pr.name().unwrap_or("?"), 1)),
                    }
                }
            }
        }
        out
    }

    fn key(&self, dpd: usize, tex0: Tex0, clut: u8) -> TexKey {
        let cbp = tex0.cbp().wrapping_add(clut as u16) & 0x3fff;
        TexKey {
            pix: (tex0.tbp() >= 0x3000).then_some(dpd),
            pal: (cbp >= 0x3e00).then_some(dpd),
            tbp: tex0.tbp(),
            tbw: tex0.tbw(),
            psm: tex0.psm(),
            tw: tex0.tw().min(10),
            th: tex0.th().min(10),
            cbp,
        }
    }

    /// The texture a shape frame or model group of DPD `dpd` samples.
    /// `clut` is the sprite's palette offset (0 for models).
    pub fn texture_for(&self, dpd: usize, tex0: Tex0, clut: u8) -> Option<TextureId> {
        if tex0.0 == 0 {
            return None;
        }
        self.tex_ids.get(&self.key(dpd, tex0, clut)).copied()
    }

    fn decode_all(&mut self) {
        let mut wanted: Vec<TexKey> = Vec::new();
        for (d, dpd) in self.dpds.iter().enumerate() {
            for s in &dpd.shapes {
                for st in &s.sequence {
                    for sp in &st.frame.sprites {
                        wanted.push(self.key(d, st.frame.tex0, sp.clut_offset));
                    }
                }
            }
            for m in &dpd.models {
                for g in &m.groups {
                    if g.tex0.0 != 0 {
                        wanted.push(self.key(d, g.tex0, 0));
                    }
                }
            }
        }
        for k in wanted {
            if self.tex_ids.contains_key(&k) {
                continue;
            }
            let t = self.decode(&k);
            self.tex_ids.insert(k, self.textures.len() as TextureId);
            self.textures.push(t);
            self.sources.push(k);
        }
    }

    fn source(&self, local: Option<usize>) -> Option<&Dpd> {
        match local {
            Some(d) => self.dpds.get(d),
            None => self.common.as_ref(),
        }
    }

    fn decode(&self, k: &TexKey) -> Texture {
        let (w, h) = (1usize << k.tw, 1usize << k.th);
        let mut rgba = vec![0u8; w * h * 4];
        let (Some(pix), Some(pal)) = (self.source(k.pix), self.source(k.pal)) else {
            return Texture { width: w as u32, height: h as u32, rgba };
        };
        // Page geometry: 8-bit pages are 128x64, 4-bit 128x128. A texture
        // is uploaded into the buffer that starts at its `tbp` (width
        // `buffer_width` x 64) at its (x, y); the TEX0 reads its own buffer.
        let (pw, ph) = if k.psm == 0x14 { (128usize, 128usize) } else { (128, 64) };
        let per_row = |bw: usize| (bw * 64 / pw).max(1);
        let page_of = |tbp: u16| tbp as usize / 32;
        let mut pages: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, t) in pix.textures.iter().enumerate() {
            if (t.psm == 0x14) != (k.psm == 0x14) {
                continue;
            }
            let pr = per_row(t.buffer_width.max(1) as usize);
            let (x, y) = (t.x.max(0) as usize, t.y.max(0) as usize);
            for py in y / ph..=(y + t.height as usize - 1) / ph {
                for px in x / pw..=(x + t.width as usize - 1) / pw {
                    pages.entry(page_of(t.tbp) + py * pr + px).or_default().push(i);
                }
            }
        }
        let palette_tex = pal
            .textures
            .iter()
            .find(|t| t.cbp == k.cbp && t.tbp == k.tbp)
            .or_else(|| pal.textures.iter().find(|t| t.cbp == k.cbp));
        let key_pr = per_row(k.tbw.max(1) as usize);
        let base = page_of(k.tbp);
        // Index of buffer pixel (lx, ly) of `page` in texture `i`, if it holds it.
        let texel = |i: usize, page: usize, lx: usize, ly: usize| -> Option<u8> {
            let t = &pix.textures[i];
            let pr = per_row(t.buffer_width.max(1) as usize);
            let rel = page.checked_sub(page_of(t.tbp))?;
            let (bx, by) = ((rel % pr) * pw + lx, (rel / pr) * ph + ly);
            let (u, v) = (bx.checked_sub(t.x.max(0) as usize)?, by.checked_sub(t.y.max(0) as usize)?);
            (u < t.width as usize && v < t.height as usize).then(|| t.indices[v * t.width as usize + u])
        };
        for y in 0..h {
            for x in 0..w {
                let page = base + (y / ph) * key_pr + x / pw;
                let Some(cands) = pages.get(&page) else { continue };
                let (lx, ly) = (x % pw, y % ph);
                // Overlapping uploads: prefer the texture the palette belongs to.
                let hit = |f: &dyn Fn(&kh2_formats::dpd::Texture) -> bool| {
                    cands.iter().filter(|&&i| f(&pix.textures[i])).find_map(|&i| texel(i, page, lx, ly))
                };
                let Some(idx) = hit(&|t| t.cbp == k.cbp).or_else(|| hit(&|t| t.tbp == k.tbp)).or_else(|| hit(&|_| true)) else {
                    continue;
                };
                let col = match palette_tex {
                    Some(p) => p.color(idx),
                    None => [idx, idx, idx, 0x80],
                };
                let o = (y * w + x) * 4;
                rgba[o..o + 4].copy_from_slice(&[col[0], col[1], col[2], (col[3] as u16 * 2).min(255) as u8]);
            }
        }
        Texture { width: w as u32, height: h as u32, rgba }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn put(b: &mut Vec<u8>, at: usize, bytes: &[u8]) {
        if b.len() < at + bytes.len() {
            b.resize(at + bytes.len(), 0);
        }
        b[at..at + bytes.len()].copy_from_slice(bytes);
    }

    /// (tbp, cbp, width, height, buffer width, upload x, fill index,
    /// palette colour for that index).
    pub(crate) type TestTexture = (u16, u16, u16, u16, i16, i16, u8, [u8; 4]);

    /// A DPD with textures only.
    pub(crate) fn texture_dpd(texs: &[TestTexture]) -> Vec<u8> {
        // Header: magic, five lists (count + offsets), four VRAM words.
        let head = 4 + 4 + (4 + texs.len() * 4) + 3 * 4 + 16;
        let mut at = (head + 15) & !15;
        let mut out = Vec::new();
        put(&mut out, 0, &0x96u32.to_le_bytes());
        let mut hdr = Vec::new();
        hdr.extend_from_slice(&0i32.to_le_bytes());
        hdr.extend_from_slice(&(texs.len() as i32).to_le_bytes());
        let mut body = Vec::new();
        for &(tbp, cbp, w, h, bw, x, fill, col) in texs {
            let mut t = vec![0u8; 0x20];
            t[0..2].copy_from_slice(&tbp.to_le_bytes());
            t[2..4].copy_from_slice(&cbp.to_le_bytes());
            t[4..6].copy_from_slice(&bw.to_le_bytes());
            t[6..8].copy_from_slice(&0x13i16.to_le_bytes());
            t[8..10].copy_from_slice(&x.to_le_bytes());
            t[12..14].copy_from_slice(&w.to_le_bytes());
            t[14..16].copy_from_slice(&h.to_le_bytes());
            t.extend(vec![fill; w as usize * h as usize]);
            let mut pal = vec![0u8; 1024];
            // CSM1: index i is stored at i with bits 3 and 4 swapped.
            let k = (fill & 0xe7) | (fill & 0x08) << 1 | (fill & 0x10) >> 1;
            pal[k as usize * 4..k as usize * 4 + 4].copy_from_slice(&col);
            t.extend(pal);
            hdr.extend_from_slice(&(at as u32).to_le_bytes());
            let len = t.len();
            body.push((at, t));
            at = (at + len + 15) & !15;
        }
        for _ in 0..3 {
            hdr.extend_from_slice(&0i32.to_le_bytes());
        }
        put(&mut out, 4, &hdr);
        put(&mut out, 4 + hdr.len(), &[0u8; 16]);
        for (o, t) in body {
            put(&mut out, o, &t);
        }
        out
    }

    /// A library of the given DPDs only (no elements, no textures).
    pub(crate) fn lib_with_dpds(dpds: Vec<Dpd>) -> Library {
        Library { elements: vec![], dpds, effects: HashMap::new(), common: None, textures: vec![], sources: vec![], tex_ids: HashMap::new(), warnings: vec![] }
    }

    fn lib_with_common(dpd: Dpd) -> Library {
        Library { elements: vec![], dpds: vec![], effects: HashMap::new(), common: Some(dpd), textures: vec![], sources: vec![], tex_ids: HashMap::new(), warnings: vec![] }
    }

    #[test]
    fn decode_packed_page() {
        // Two 64x64 textures uploaded side by side into one page (x 0 and
        // 64), each with its own palette. A 128x64 TEX0 with the second
        // palette shows the second texture on its right half.
        let common = texture_dpd(&[(0x2c60, 0x2e04, 64, 64, 2, 0, 3, [1, 2, 3, 0x80]), (0x2c60, 0x2e18, 64, 64, 2, 64, 5, [200, 100, 50, 0x80])]);
        let lib = lib_with_common(Dpd::parse(&common).unwrap());
        let tex0 = Tex0(0x2c60 | 2 << 14 | 0x13 << 20 | 7 << 26 | 6 << 30 | 1 << 34 | 0x2e18 << 37);
        let t = lib.decode(&lib.key(0, tex0, 0));
        let at = |x: usize, y: usize| &t.rgba[(y * 128 + x) * 4..(y * 128 + x) * 4 + 4];
        assert_eq!(at(100, 10), &[200, 100, 50, 255]);
        // The left half is the other texture, read through this palette.
        assert_eq!(at(10, 10), &[0, 0, 0, 0]);
    }

    #[test]
    fn decode_atlas_from_pages() {
        // Two 128x64 pages side by side in a 256-wide buffer, each with its
        // own fill colour, read as one 256x64 texture.
        let common = texture_dpd(&[
            (0x2a00, 0x2e00, 128, 64, 2, 0, 7, [10, 20, 30, 0x80]),
            (0x2a20, 0x2e04, 128, 64, 2, 0, 9, [40, 50, 60, 0x40]),
        ]);
        let dpd = Dpd::parse(&common).unwrap();
        let lib = lib_with_common(dpd);
        // TEX0: tbp 0x2a00, tbw 4 (256 px), psm 0x13, 256x64, cbp 0x2e00.
        let tex0 = Tex0(0x2a00 | 4 << 14 | 0x13 << 20 | 8 << 26 | 6 << 30 | 1 << 34 | 0x2e00 << 37);
        let t = lib.decode(&lib.key(0, tex0, 0));
        assert_eq!((t.width, t.height), (256, 64));
        assert_eq!(&t.rgba[0..4], &[10, 20, 30, 255]);
        // The right page uses the same palette (the TEX0's), where index 9
        // is black and transparent.
        let o = (10 * 256 + 200) * 4;
        assert_eq!(&t.rgba[o..o + 4], &[0, 0, 0, 0]);
        // With the second palette (clut offset 4 blocks) it is coloured.
        let t2 = lib.decode(&lib.key(0, tex0, 4));
        assert_eq!(&t2.rgba[o..o + 4], &[40, 50, 60, 128]);
    }
}
