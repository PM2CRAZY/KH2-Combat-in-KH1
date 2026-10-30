//! KH2's Drive orb art in KH1's prize file, built once from the player's own KH1 and KH2 files into
//! the mod folder (the owner, the main-branch port's round 4, row 3, answer 3 (a) and "(a)": KH2's
//! Drive orb art now). Notes: `docs/kh1-drive-forms-native.md` §15.4–15.5.
//!
//! - **The source** (Checked (files)): KH1's `prize.dpx` (one entry, one DPD at `0xA0`: 4 texture
//!   records, 4 shapes, the prize program's 10 entries with 7..9 unused) and its HD images
//!   (`-320.dds`, `-1740.dds`, `-3b60.dds`, keyed by the offset of each TBP's first record's pixels
//!   in the file); KH2's `obj/PRIZE.mdlx` (a BAR whose `priz` entry is the PAX), whose prize program
//!   draws kinds 7 / 8 (the small and big Drive orbs) with shape 3 on texture TBP 0x3000 / CBP
//!   0x3E00, and its HD images (keyed the same way, `+0x24` of each asset's header).
//! - **The build** (`kh2_formats::kh1dpx::prize_with_kh2`): KH1's file with KH2's shape and texture
//!   record appended, entries 7 / 8 set to KH2's (slot, colour, scale); KH1's HD images under their
//!   new keys, KH2's under its record's.
//! - **The cache** (`kh2-prize-orbs.bin` in `kh1-kh2-cache`, the slot cache's file layout): a manifest
//!   (this format, the mod's version, the game build, both source files' package entries) and the
//!   blocks: the DPX (key 0), then each HD image (key `1 << 32 | its key in the DPX`).
//!
//! Pure Rust (no Windows calls), unit tested on any host; the game side is `win::driveorbs`.

use crate::slotcache::{self, Block, Manifest};
use std::path::{Path, PathBuf};

/// Bump when the cache's output changes for the same inputs without the mod's version changing.
pub const FORMAT: u32 = 1;
pub const FILE_NAME: &str = "kh2-prize-orbs.bin";
/// KH1's prize file and KH2's (their names in the packages).
pub const KH1_FILE: &str = "prize.dpx";
pub const KH2_FILE: &str = "obj/PRIZE.mdlx";
/// The prize kinds drawn as KH2's: the small and big Drive orbs (KH2's kinds, KH1's new types).
pub const KINDS: [usize; 2] = [7, 8];
const HD_KEY: u128 = 1 << 32;

/// What the cache is made from.
#[derive(Clone, Debug)]
pub struct Desc {
    pub build: String,
    /// `prize.dpx`'s package entry (`kh1_data::Kh1Install::stamp`), which holds its HD images too.
    pub kh1: String,
    /// `obj/PRIZE.mdlx`'s (`kh2_data::Install::stamp_any`).
    pub kh2: String,
}

impl Desc {
    fn manifest(&self) -> Manifest {
        let mut m = Manifest::default();
        m.set("prize art format", FORMAT.to_string());
        m.set("mod version", crate::VERSION.to_string());
        m.set("game build", self.build.clone());
        m.set("prize.dpx entry", self.kh1.clone());
        m.set("obj/PRIZE.mdlx entry", self.kh2.clone());
        m
    }
}

/// The art: the DPX (KH1's layout, not yet relocated) and its HD images by key (the offset of the
/// record's pixels in the DPX).
pub struct Art {
    pub dpx: Block,
    pub hd: Vec<(u32, Block)>,
}

/// What `build` makes, and what it did (for the log).
pub struct Built {
    pub dpx: Vec<u8>,
    pub hd: Vec<(u32, Vec<u8>)>,
    pub report: String,
}

/// The art from KH1's `prize.dpx` (`kh1`), its HD image of a key (`kh1_hd`, `None` when it has
/// none), KH2's `obj/PRIZE.mdlx` (`mdlx`) and its HD images (`kh2_hd`: each one's `+0x24` key and
/// bytes). A KH1 record whose TBP has no HD image (KH1's fourth, which shares the first's) gets
/// none, as in KH1's own file; a KH2 record without one is an error.
pub fn build(kh1: &[u8], kh1_hd: impl Fn(u32) -> Option<Vec<u8>>, mdlx: &[u8], kh2_hd: &[(u32, Vec<u8>)]) -> Result<Built, String> {
    let entries = crate::hudart::bar_entries(mdlx)?;
    let pax = entries.iter().find(|e| e.1 == "priz" && e.2.starts_with(b"PAX_")).ok_or("no priz PAX in obj/PRIZE.mdlx")?.2;
    let pax_at = pax.as_ptr() as usize - mdlx.as_ptr() as usize;
    let r = kh2_formats::kh1dpx::prize_with_kh2(kh1, pax, &KINDS).map_err(|e| e.to_string())?;
    let mut hd = Vec::new();
    let mut report = vec![r.report.clone()];
    for &(old, new) in &r.kh1_keys {
        if let Some(d) = kh1_hd(old) {
            report.push(format!("KH1 {old:#x} -> {new:#x} ({} bytes)", d.len()));
            hd.push((new, d));
        }
    }
    for (new, keys) in &r.kh2_keys {
        let found = keys.iter().find_map(|&k| kh2_hd.iter().find(|a| a.0 & 0xFF_FFFF == (pax_at as u32 + k)));
        let Some((key, data)) = found else {
            return Err(format!("no KH2 HD image for the prize texture at {:#x}", pax_at as u32 + keys[0]));
        };
        report.push(format!("KH2 {key:#x} -> {new:#x} ({} bytes)", data.len()));
        hd.push((*new, data.clone()));
    }
    Ok(Built { dpx: r.bytes, hd, report: report.join("; ") })
}

/// Writes the art into `dir`.
pub fn store(dir: &Path, d: &Desc, b: &Built) -> std::io::Result<PathBuf> {
    let mut blocks: Vec<(u128, &[u8])> = vec![(0, &b.dpx[..])];
    blocks.extend(b.hd.iter().map(|(k, v)| (HD_KEY | *k as u128, &v[..])));
    slotcache::write_named_blocks(dir, FILE_NAME, &d.manifest(), &blocks)
}

/// The art in `dir` if it was made from these sources (`Err`: missing, damaged or changed; the
/// reason as text).
pub fn load(dir: &Path, d: &Desc) -> Result<Art, String> {
    let b = std::fs::read(dir.join(FILE_NAME)).map_err(|_| "none yet".to_string())?;
    let (m, blocks) = slotcache::decode_shared(b)?;
    let differ = d.manifest().differences(&m);
    if !differ.is_empty() {
        return Err(format!("changed: {}", differ.join(", ")));
    }
    let mut it = blocks.into_iter();
    let Some((0, dpx)) = it.next() else { return Err("damaged blocks".into()) };
    let mut hd = Vec::new();
    for (k, v) in it {
        if k >> 32 != 1 || k as u32 as usize >= dpx.len() {
            return Err("damaged blocks".into());
        }
        hd.push((k as u32, v));
    }
    Ok(Art { dpx, hd })
}

/// `Built` as the art (no file).
pub fn art_of(b: Built) -> Art {
    Art { dpx: Block::new(b.dpx), hd: b.hd.into_iter().map(|(k, v)| (k, Block::new(v))).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desc() -> Desc {
        Desc { build: "Steam".into(), kh1: "kh1_first @1 2/3".into(), kh2: "kh2_first @4 5/6".into() }
    }

    #[test]
    fn the_cache_keeps_the_dpx_and_its_images() {
        let dir = std::env::temp_dir().join(format!("kh1-prizeart-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let b = Built { dpx: vec![7; 0x6000], hd: vec![(0x320, vec![1; 40]), (0x5ba0, vec![2; 50])], report: String::new() };
        assert!(load(&dir, &desc()).is_err());
        store(&dir, &desc(), &b).unwrap();
        let a = load(&dir, &desc()).unwrap();
        assert_eq!(&a.dpx[..], &b.dpx[..]);
        assert_eq!(a.hd.iter().map(|(k, v)| (*k, v.to_vec())).collect::<Vec<_>>(), b.hd);
        // A changed source is a miss.
        let mut other = desc();
        other.kh2 = "kh2_first @4 5/7".into();
        assert!(load(&dir, &other).err().is_some_and(|e| e.starts_with("changed")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
