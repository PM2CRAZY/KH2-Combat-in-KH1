//! The effect package cache (the tester, ram-use thread m3: "do all three", the assets built once
//! into the mod folder; `docs/kh1-asset-cache.md` §5).
//!
//! Each KH2 effect package made into a KH1 DPX (`win::kh2dpx::build`: Sora's `p_ex100`, the
//! Keyblade's `w_ex010`, the forms', the spells') is saved once as its own file in the mod's
//! `kh1-kh2-cache` folder, beside the motion slot cache: the DPX's bytes, its MD5, what the game
//! side needs to enter and play it (its layers, program types, KH2 program places, lookup words and
//! per-palette DPDs, as an opaque `meta` the game side writes and reads), and the HD images its
//! texture records take (its own, each once; KH2's common ones by TBP, which stay read from
//! `texcommon.dpd` at every start). The file starts with a manifest (`slotcache::Manifest`: the
//! format, the mod's version, the game build, the package's source file and everything the build
//! is given); a file whose manifest differs, or that is damaged, is built again and rewritten.
//!
//! Loading keeps one copy: the DPX is read straight into the memory it is played from (the caller's
//! `alloc`), and the images into one buffer that each image is a part of (`slotcache::Block`),
//! freed once the package is entered in KH1's loaded files.
//!
//! Pure Rust (no Windows calls), unit tested on any host.

use crate::slotcache::{Block, Manifest, Miss};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Bump when the DPX build's output changes for the same inputs without the mod's version changing
/// (the version is in the manifest too, so every release rebuilds once anyway).
pub const BUILD_FORMAT: u32 = 1;
const MAGIC: &[u8; 8] = b"KH1KH2FX";
/// The file's own layout (this module's).
const FILE_FORMAT: u32 = 1;
/// The longest piece a file may claim (a damaged length can't ask for more memory than this).
const MAX_PIECE: usize = 64 << 20;

/// The file of the package labelled `label` (`kh2dpx`'s labels: `p_ex100`, `fire_3_mag`, ...).
pub fn file_name(label: &str) -> String {
    let clean: String = label.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
    format!("kh2-effects-{clean}.bin")
}

/// A manifest with this cache's format and the mod's version (the game side adds the rest).
pub fn manifest() -> Manifest {
    let mut m = Manifest::default();
    m.set("dpx build format", BUILD_FORMAT.to_string());
    m.set("mod version", crate::VERSION.to_string());
    m
}

/// A fast 128-bit content hash (two 64-bit lanes over 8-byte words), for change checks of files of
/// several MB (not for security). About a word per nanosecond, where `slotcache::Hasher` takes a
/// byte at a time.
pub fn fast_hash(b: &[u8]) -> u128 {
    const K1: u64 = 0x9E37_79B9_7F4A_7C15;
    const K2: u64 = 0xC2B2_AE3D_27D4_EB4F;
    let mut a: u64 = 0x243F_6A88_85A3_08D3 ^ b.len() as u64;
    let mut c: u64 = 0x1319_8A2E_0370_7344 ^ (b.len() as u64).rotate_left(32);
    let mut words = b.chunks_exact(8);
    for w in &mut words {
        let w = u64::from_le_bytes(w.try_into().unwrap());
        a = (a ^ w).wrapping_mul(K1).rotate_left(29);
        c = (c ^ w.rotate_left(23)).wrapping_mul(K2).rotate_left(31);
    }
    let mut tail = [0u8; 8];
    let rest = words.remainder();
    tail[..rest.len()].copy_from_slice(rest);
    let w = u64::from_le_bytes(tail) ^ ((rest.len() as u64) << 56);
    a = (a ^ w).wrapping_mul(K1).rotate_left(29);
    c = (c ^ w.rotate_left(23)).wrapping_mul(K2).rotate_left(31);
    let mix = |mut x: u64| {
        x ^= x >> 30;
        x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^ (x >> 31)
    };
    let (a2, c2) = (mix(a ^ c.rotate_left(17)), mix(c ^ a.rotate_left(41)));
    ((a2 as u128) << 64) | c2 as u128
}

/// The check over a file's pieces, in order.
#[derive(Default)]
struct Check(u128);

impl Check {
    fn piece(&mut self, b: &[u8]) {
        let h = fast_hash(b);
        self.0 = fast_hash(&[self.0.to_le_bytes(), h.to_le_bytes()].concat());
    }
}

/// An HD texture record's image: one of the package's own (its index in the images), or KH2's
/// common image of a source TBP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HdRef {
    Own(u32),
    Common(i32),
}

/// What is saved of a built package.
pub struct Save<'a> {
    pub manifest: &'a Manifest,
    pub md5: [u8; 16],
    pub meta: &'a [u8],
    /// Each HD texture record: its pixels' offset in the DPX and its image.
    pub hd: &'a [(u32, HdRef)],
    pub images: &'a [&'a [u8]],
    pub dpx: &'a [u8],
}

/// A package read from its file: the DPX in the caller's memory (`dpx`, its first `dpx_len` bytes),
/// the images parts of one buffer.
pub struct Loaded<P> {
    pub md5: [u8; 16],
    pub meta: Vec<u8>,
    pub hd: Vec<(u32, HdRef)>,
    pub images: Vec<Block>,
    pub dpx: P,
    pub dpx_len: usize,
}

/// Writes `s` as `dir` / `name`: to a temporary file first, then renamed over the old one (no half
/// file under the real name; the temporary removed on failure).
pub fn write(dir: &Path, name: &str, s: &Save) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(name);
    let tmp = dir.join(format!("{name}.{}.tmp", std::process::id()));
    let r = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        {
            let mut w = std::io::BufWriter::new(&mut f);
            let mut check = Check::default();
            let mut put = |b: &[u8]| -> std::io::Result<()> {
                check.piece(b);
                w.write_all(&(b.len() as u32).to_le_bytes())?;
                w.write_all(b)
            };
            put(MAGIC)?;
            put(&FILE_FORMAT.to_le_bytes())?;
            put(s.manifest.text().as_bytes())?;
            put(s.meta)?;
            put(&s.md5)?;
            let mut hd = Vec::with_capacity(s.hd.len() * 9);
            for (offset, r) in s.hd {
                hd.extend_from_slice(&offset.to_le_bytes());
                match r {
                    HdRef::Own(i) => {
                        hd.push(0);
                        hd.extend_from_slice(&i.to_le_bytes());
                    }
                    HdRef::Common(t) => {
                        hd.push(1);
                        hd.extend_from_slice(&t.to_le_bytes());
                    }
                }
            }
            put(&hd)?;
            let lens: Vec<u8> = s.images.iter().flat_map(|i| (i.len() as u32).to_le_bytes()).collect();
            put(&lens)?;
            for i in s.images {
                put(i)?;
            }
            put(s.dpx)?;
            let c = check.0;
            w.write_all(&c.to_le_bytes())?;
            w.flush()?;
        }
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, &path)
    })();
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r.map(|_| path)
}

/// Reads `path` for `manifest`: the package when the file is there, undamaged and made for exactly
/// this manifest, else why not (`Miss`; any read error counts as damaged). `alloc(n)` gives the
/// memory the DPX's `n` bytes are read into (`None`: no memory, a miss; dropped on any later
/// failure).
pub fn load<P: AsMut<[u8]>>(path: &Path, manifest: &Manifest, alloc: impl FnOnce(usize) -> Option<P>) -> Result<Loaded<P>, Miss> {
    let f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(Miss::Missing),
        Err(e) => return Err(Miss::Damaged(e.to_string())),
    };
    let mut r = std::io::BufReader::new(f);
    let mut check = Check::default();
    let damaged = |e: String| Miss::Damaged(e);
    let len = |r: &mut std::io::BufReader<std::fs::File>| -> Result<usize, Miss> {
        let mut n = [0u8; 4];
        r.read_exact(&mut n).map_err(|_| damaged("cut short".into()))?;
        let n = u32::from_le_bytes(n) as usize;
        if n > MAX_PIECE {
            return Err(damaged("a piece too long".into()));
        }
        Ok(n)
    };
    let take = |r: &mut std::io::BufReader<std::fs::File>, check: &mut Check| -> Result<Vec<u8>, Miss> {
        let n = len(r)?;
        let mut v = vec![0u8; n];
        r.read_exact(&mut v).map_err(|_| damaged("cut short".into()))?;
        check.piece(&v);
        Ok(v)
    };
    if take(&mut r, &mut check)? != MAGIC {
        return Err(damaged("not an effect cache".into()));
    }
    let format = take(&mut r, &mut check)?;
    if format != FILE_FORMAT.to_le_bytes() {
        return Err(damaged("another file format".into()));
    }
    let text = take(&mut r, &mut check)?;
    let on_disk = Manifest::from_text(std::str::from_utf8(&text).map_err(|_| damaged("manifest not text".into()))?);
    if on_disk != *manifest {
        return Err(Miss::Changed(manifest.differences(&on_disk)));
    }
    let meta = take(&mut r, &mut check)?;
    let md5: [u8; 16] = take(&mut r, &mut check)?.try_into().map_err(|_| damaged("no MD5".into()))?;
    let hd_bytes = take(&mut r, &mut check)?;
    if hd_bytes.len() % 9 != 0 {
        return Err(damaged("HD list".into()));
    }
    let lens: Vec<usize> = take(&mut r, &mut check)?.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap()) as usize).collect();
    let mut hd = Vec::with_capacity(hd_bytes.len() / 9);
    for e in hd_bytes.chunks_exact(9) {
        let offset = u32::from_le_bytes(e[0..4].try_into().unwrap());
        let v = u32::from_le_bytes(e[5..9].try_into().unwrap());
        hd.push((
            offset,
            match e[4] {
                0 if (v as usize) < lens.len() => HdRef::Own(v),
                1 => HdRef::Common(v as i32),
                _ => return Err(damaged("HD list".into())),
            },
        ));
    }
    // The images in one buffer, each a part of it.
    let total: usize = lens.iter().sum();
    if total > MAX_PIECE {
        return Err(damaged("images too long".into()));
    }
    let mut all = vec![0u8; total];
    let mut at = 0;
    let mut ranges = Vec::with_capacity(lens.len());
    for &want in &lens {
        let n = len(&mut r)?;
        if n != want {
            return Err(damaged("image length".into()));
        }
        r.read_exact(&mut all[at..at + n]).map_err(|_| damaged("cut short".into()))?;
        check.piece(&all[at..at + n]);
        ranges.push(at..at + n);
        at += n;
    }
    let buf = Block::new(all);
    let images = ranges.into_iter().map(|rg| buf.part(rg)).collect();
    drop(buf);
    // The DPX, straight into its memory.
    let n = len(&mut r)?;
    let mut dpx = alloc(n).ok_or_else(|| damaged("no memory for its DPX".into()))?;
    {
        let m = dpx.as_mut();
        if m.len() < n {
            return Err(damaged("no memory for its DPX".into()));
        }
        r.read_exact(&mut m[..n]).map_err(|_| damaged("cut short".into()))?;
        check.piece(&m[..n]);
    }
    let mut c = [0u8; 16];
    r.read_exact(&mut c).map_err(|_| damaged("cut short".into()))?;
    if u128::from_le_bytes(c) != check.0 {
        return Err(damaged("check failed (cut short or damaged)".into()));
    }
    let mut extra = [0u8; 1];
    if r.read(&mut extra).map_err(|e| damaged(e.to_string()))? != 0 {
        return Err(damaged("bytes after its end".into()));
    }
    Ok(Loaded { md5, meta, hd, images, dpx, dpx_len: n })
}

/// A little-endian writer and reader for the game side's `meta`.
#[derive(Default)]
pub struct Out(pub Vec<u8>);

impl Out {
    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.0.push(v);
        self
    }
    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn i32(&mut self, v: i32) -> &mut Self {
        self.u32(v as u32)
    }
    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn f32(&mut self, v: f32) -> &mut Self {
        self.u32(v.to_bits())
    }
    pub fn f32s(&mut self, v: &[f32]) -> &mut Self {
        for x in v {
            self.f32(*x);
        }
        self
    }
}

pub struct In<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> In<'a> {
    pub fn new(b: &'a [u8]) -> In<'a> {
        In { b, at: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let s = self.b.get(self.at..self.at + n).ok_or("meta cut short")?;
        self.at += n;
        Ok(s)
    }
    pub fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    pub fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn i32(&mut self) -> Result<i32, String> {
        Ok(self.u32()? as i32)
    }
    pub fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    pub fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_bits(self.u32()?))
    }
    pub fn f32x3(&mut self) -> Result<[f32; 3], String> {
        Ok([self.f32()?, self.f32()?, self.f32()?])
    }
    /// A count, sanity-checked against what is left (each item at least `min` bytes).
    pub fn count(&mut self, min: usize) -> Result<usize, String> {
        let n = self.u32()? as usize;
        if n.saturating_mul(min.max(1)) > self.b.len() - self.at {
            return Err("meta count too large".into());
        }
        Ok(n)
    }
    pub fn done(&self) -> Result<(), String> {
        if self.at == self.b.len() {
            Ok(())
        } else {
            Err("meta has bytes left".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dpxcache-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn sample(m: &Manifest) -> (Vec<u8>, Vec<(u32, HdRef)>, Vec<Vec<u8>>, Vec<u8>) {
        let _ = m;
        (vec![1, 2, 3, 4, 5], vec![(0x40, HdRef::Own(1)), (0x80, HdRef::Common(0x3000)), (0x120, HdRef::Own(0))], vec![vec![9; 100], vec![7; 33]], (0..5000u32).map(|x| x as u8).collect())
    }

    fn save(d: &Path, m: &Manifest) -> PathBuf {
        let (meta, hd, images, dpx) = sample(m);
        let refs: Vec<&[u8]> = images.iter().map(|i| &i[..]).collect();
        write(d, &file_name("p_ex100"), &Save { manifest: m, md5: [5; 16], meta: &meta, hd: &hd, images: &refs, dpx: &dpx }).unwrap()
    }

    #[test]
    fn round_trip() {
        let d = dir("rt");
        let mut m = manifest();
        m.set("package", "p_ex100");
        let p = save(&d, &m);
        assert_eq!(p.file_name().unwrap(), "kh2-effects-p_ex100.bin");
        let l = load(&p, &m, |n| Some(vec![0u8; n + 7])).ok().unwrap();
        let (meta, hd, images, dpx) = sample(&m);
        assert_eq!(l.meta, meta);
        assert_eq!(l.hd, hd);
        assert_eq!(l.md5, [5; 16]);
        assert_eq!(l.images.len(), 2);
        assert_eq!(&*l.images[0], &images[0][..]);
        assert_eq!(&*l.images[1], &images[1][..]);
        assert_eq!(l.dpx_len, dpx.len());
        assert_eq!(&l.dpx[..l.dpx_len], &dpx[..]);
        // (One buffer for every image.)
        assert_eq!(l.images[0].as_ptr() as usize + 100, l.images[1].as_ptr() as usize);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn missing_changed_damaged() {
        let d = dir("miss");
        let mut m = manifest();
        m.set("file obj/P_EX100.a.us", "aa 1");
        let p = d.join(file_name("p_ex100"));
        assert!(matches!(load(&p, &m, |n| Some(vec![0u8; n])), Err(Miss::Missing)));
        save(&d, &m);
        let mut m2 = m.clone();
        m2.set("file obj/P_EX100.a.us", "bb 1");
        match load(&p, &m2, |n| Some(vec![0u8; n])) {
            Err(Miss::Changed(d)) => assert_eq!(d, vec!["file obj/P_EX100.a.us changed".to_string()]),
            _ => panic!("expected a changed manifest"),
        }
        // A byte of the DPX flipped, then the file cut short: both damaged, and the memory given
        // back (dropped).
        let mut b = std::fs::read(&p).unwrap();
        let k = b.len() - 100;
        b[k] ^= 1;
        std::fs::write(&p, &b).unwrap();
        assert!(matches!(load(&p, &m, |n| Some(vec![0u8; n])), Err(Miss::Damaged(_))));
        std::fs::write(&p, &b[..b.len() / 2]).unwrap();
        assert!(matches!(load(&p, &m, |n| Some(vec![0u8; n])), Err(Miss::Damaged(_))));
        std::fs::write(&p, b"nonsense").unwrap();
        assert!(matches!(load(&p, &m, |n| Some(vec![0u8; n])), Err(Miss::Damaged(_))));
        // No memory for the DPX: a miss, not a panic.
        save(&d, &m);
        assert!(matches!(load(&p, &m, |_| None::<Vec<u8>>), Err(Miss::Damaged(_))));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn rewritten_in_place() {
        let d = dir("rw");
        let m = manifest();
        let p = save(&d, &m);
        save(&d, &m);
        assert!(load(&p, &m, |n| Some(vec![0u8; n])).is_ok());
        // (No temporary left.)
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn fast_hash_sees_every_byte() {
        let a: Vec<u8> = (0..1001u32).map(|x| (x * 7) as u8).collect();
        let h = fast_hash(&a);
        for i in [0, 7, 8, 500, 999, 1000] {
            let mut b = a.clone();
            b[i] ^= 0x10;
            assert_ne!(fast_hash(&b), h, "byte {i}");
        }
        assert_ne!(fast_hash(&a[..1000]), h);
        assert_ne!(fast_hash(&[]), fast_hash(&[0]));
    }

    #[test]
    fn meta_round_trip() {
        let mut o = Out::default();
        o.u8(3).u32(7).i32(-2).u64(1 << 40).f32(1.5).f32s(&[1.0, 2.0, 3.0]);
        let mut i = In::new(&o.0);
        assert_eq!((i.u8().unwrap(), i.u32().unwrap(), i.i32().unwrap(), i.u64().unwrap(), i.f32().unwrap()), (3, 7, -2, 1 << 40, 1.5));
        assert_eq!(i.f32x3().unwrap(), [1.0, 2.0, 3.0]);
        i.done().unwrap();
        assert!(i.u8().is_err());
    }
}
