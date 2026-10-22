//! KH2's prize art: the HP, MP and Drive orbs and the munny, `obj/PRIZE.mdlx` (`docs/kh1-prizes.md`,
//! "KH2's prize art"). Read from the user's own install; nothing of it is kept in the repository.
//!
//! The file is a BAR with one PAX (`priz`) whose one effect runs a single program,
//! `pppRyjMegaBirthPrize`, the same program KH1 draws its prizes with. Its key is the program's
//! data: per prize type (KH2's types 0..9) an entry of 12 bytes from `+4` (a shape number,
//! `0xFFFF` none; a colour, `0x80` = 1.0; a float scale), and the shape animation speed at
//! `+0x7C`. The shapes are DPD sprite animations ([`crate::dpd::Shape`]) on 8-bit textures that
//! share 128 × 64 pages, each with its own palette; the package's remastered images (`-0.dds`,
//! `-1.dds`) are those pages, palettes applied, at twice the size.
//!
//! In the user's file: types 0 / 1 HP small and big (shape 1, a green orb with a white glow), 2..4
//! munny (shape 0), 5 / 6 MP (shape 2, the bubble), 7 / 8 the Drive orb small and big (shape 3 at
//! 0.9 and 1.7: a gold orb, normally blended, pulsing from half to one and a half brightness, under
//! a grey swirl ring added in orange that turns a full circle over its 24 steps).

use crate::bar::Bar;
use crate::dpd::{animation_frames, effect_source, shape_animation, Dpd, Tex0};
use crate::pax::Pax;
use crate::{bad, Result};

/// The file KH2 draws its prizes from (its object `PRIZE`, `00objentry`).
pub const FILE: &str = "obj/PRIZE.mdlx";
/// The program the prize effect runs.
pub const PROGRAM: &str = "pppRyjMegaBirthPrize";
/// Entries in the program's data (one per prize type).
pub const ENTRIES: usize = 10;
/// KH2's prize types.
pub const MP_SMALL: usize = 5;
pub const MP_BIG: usize = 6;
pub const DRIVE_SMALL: usize = 7;
pub const DRIVE_BIG: usize = 8;
/// The program data's length (the frame word, the entries, the speed, two flag bytes and padding).
pub const DATA_LEN: usize = 0x84;

/// One prize type's look.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry {
    /// The shape drawn (`None`: nothing).
    pub shape: Option<i32>,
    /// `0x80` = 1.0 per channel.
    pub rgba: [u8; 4],
    pub scale: f32,
}

/// One shape, as the effect runtime uses it.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeArt {
    pub number: i32,
    /// The raw animation (see [`shape_animation`]).
    pub animation: Vec<u8>,
    /// Its frames' offsets in `animation`.
    pub frames: Vec<usize>,
    /// The first frame's texture register (every frame of KH2's prize shapes has the same).
    pub tex0: Tex0,
    /// How far the first frame's sprites reach from the centre (shape units, the largest
    /// corner coordinate).
    pub extent: f32,
    pub steps: usize,
}

/// A texture page as the shapes address it: `1 << tw` × `1 << th` texels at a GS block address.
#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub tbp: u16,
    /// Which of the package's remastered images it is (the distinct pages in texture order).
    pub index: usize,
    /// The PS2 size the shapes' texture coordinates count in.
    pub width: u32,
    pub height: u32,
    /// The PS2 page itself: every texture on it with its own palette, RGBA (alpha `0x80` scaled to
    /// 255), `width × height`.
    pub rgba: Vec<u8>,
}

/// KH2's prize effect, read.
#[derive(Clone, Debug)]
pub struct PrizeArt {
    pub entries: [Entry; ENTRIES],
    /// The shape animation speed (added each run; a step lasts its wait).
    pub speed: i32,
    /// The program's data, as the runtime gets it.
    pub data: Vec<u8>,
    dpd_bytes: Vec<u8>,
    dpd: Dpd,
}

/// The entries of the program's data (`data` from its frame word).
pub fn parse_entries(data: &[u8]) -> Result<([Entry; ENTRIES], i32)> {
    if data.len() < 0x80 {
        return bad("prize program data too short");
    }
    let rd = |at: usize| [data[at], data[at + 1], data[at + 2], data[at + 3]];
    let entries = std::array::from_fn(|t| {
        let e = 4 + t * 12;
        let shape = i32::from_le_bytes(rd(e));
        Entry { shape: (shape != 0xFFFF).then_some(shape), rgba: rd(e + 4), scale: f32::from_le_bytes(rd(e + 8)) }
    });
    Ok((entries, i32::from_le_bytes(rd(0x7C))))
}

impl PrizeArt {
    /// Reads `obj/PRIZE.mdlx` (`bar`, the file's bytes).
    pub fn parse(bar: &[u8]) -> Result<PrizeArt> {
        let bar = Bar::parse(bar)?;
        let pax = bar.entries.iter().find(|e| e.data.starts_with(b"PAX_")).ok_or(crate::Error::Format("no PAX in the BAR".into()))?.data;
        let p = Pax::parse(pax)?;
        let effect = p.elements.first().map_or(0, |e| e.effect as u32);
        let (dpd_buf, index) = effect_source(pax, effect)?.ok_or(crate::Error::Format(format!("PAX: no effect {effect}")))?;
        let dpd = Dpd::parse(dpd_buf)?;
        let pd = dpd.particles.get(index).ok_or(crate::Error::Format("DPD: no particle data".into()))?;
        let prog = pd
            .emitters
            .iter()
            .flat_map(|e| e.programs.iter())
            .find(|p| p.name() == Some(PROGRAM))
            .ok_or(crate::Error::Format(format!("no {PROGRAM} program")))?;
        let key = prog.keys.first().ok_or(crate::Error::Format(format!("{PROGRAM}: no key")))?;
        let (entries, speed) = parse_entries(&key.data)?;
        Ok(PrizeArt { entries, speed, data: key.data.clone(), dpd_bytes: dpd_buf.to_vec(), dpd })
    }

    /// Shape `number`, raw, with its first frame's texture register and reach.
    pub fn shape(&self, number: i32) -> Result<ShapeArt> {
        let animation = shape_animation(&self.dpd_bytes, number)?.ok_or(crate::Error::Format(format!("no shape {number}")))?;
        let frames = animation_frames(&animation)?;
        let s = self.dpd.shape(number).ok_or(crate::Error::Format(format!("no shape {number}")))?;
        let first = &s.sequence.first().ok_or(crate::Error::Format(format!("shape {number} has no steps")))?.frame;
        let extent = first.sprites.iter().flat_map(|sp| sp.positions()).flat_map(|[x, y]| [x.abs(), y.abs()]).fold(0.0f32, f32::max);
        Ok(ShapeArt { number, animation, frames, tex0: first.tex0, extent, steps: s.sequence.len() })
    }

    /// The page a shape's texture register addresses, drawn from the PS2 textures.
    pub fn page(&self, tex0: Tex0) -> Option<Page> {
        let mut tbps: Vec<u16> = Vec::new();
        for t in &self.dpd.textures {
            if !tbps.contains(&t.tbp) {
                tbps.push(t.tbp);
            }
        }
        let index = tbps.iter().position(|&t| t == tex0.tbp())?;
        let (w, h) = (1u32 << tex0.tw().min(11), 1u32 << tex0.th().min(11));
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for t in self.dpd.textures.iter().filter(|t| t.tbp == tex0.tbp()) {
            let px = t.rgba();
            for y in 0..t.height as i32 {
                for x in 0..t.width as i32 {
                    let (px_x, px_y) = (t.x as i32 + x, t.y as i32 + y);
                    if px_x < 0 || px_y < 0 || px_x >= w as i32 || px_y >= h as i32 {
                        continue;
                    }
                    let from = ((y * t.width as i32 + x) * 4) as usize;
                    let to = ((px_y * w as i32 + px_x) * 4) as usize;
                    rgba[to..to + 4].copy_from_slice(&px[from..from + 4]);
                }
            }
        }
        Some(Page { tbp: tex0.tbp(), index, width: w, height: h, rgba })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_entries() {
        let mut d = vec![0u8; DATA_LEN];
        for t in 0..ENTRIES {
            let e = 4 + t * 12;
            let shape: i32 = if t == 9 { 0xFFFF } else { t as i32 / 2 };
            d[e..e + 4].copy_from_slice(&shape.to_le_bytes());
            d[e + 4..e + 8].copy_from_slice(&[0x80, 0x80, 0x80, 0x80]);
            d[e + 8..e + 12].copy_from_slice(&(0.5 + t as f32 * 0.1).to_le_bytes());
        }
        d[0x7C..0x80].copy_from_slice(&512i32.to_le_bytes());
        let (e, speed) = parse_entries(&d).unwrap();
        assert_eq!(speed, 512);
        assert_eq!(e[DRIVE_SMALL].shape, Some(3));
        assert_eq!(e[9].shape, None);
        assert!((e[DRIVE_BIG].scale - 1.3).abs() < 1e-6);
        assert!(parse_entries(&d[..0x40]).is_err());
    }
}
