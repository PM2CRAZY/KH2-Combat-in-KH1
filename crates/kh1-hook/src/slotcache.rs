//! The motion slot cache (the tester, what-is-still-missing m7 answer 2 (a): "save the KH2 motion
//! slots we bake at every start into the mod folder, built from your own KH1 and KH2 files and
//! rebuilt only when those change"). `docs/kh1-asset-cache.md`.
//!
//! The baked KH1-format motion blocks `kh2slots::prepare` makes (each KH2 motion baked into a
//! KH1 motion, `kh1_data::bake`) are kept in one file next to the DLL. The file starts with a
//! manifest: everything the bake depends on (the bake format, the mod's version, the game build,
//! a hash of every KH1 and KH2 file read for it, the ini keys it reads). When the manifest on disk
//! matches the one worked out at this start, its blocks are used in place of baking; otherwise
//! (or for any block it doesn't have) the block is baked, and the file is written again.
//!
//! Each block also carries its own key: a hash of what that one bake reads (the KH1 template
//! motion's bytes, the KH2 motion, its length, its events, its travel), so a block is only ever
//! used for exactly the bake it came from.
//!
//! Pure Rust (no Windows calls) so it is unit tested on any host; the game side is in
//! `win::kh2slots` / `win::pose`.

use std::collections::HashMap;
use std::ops::{Deref, Range};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Bump when the bake's output changes for the same inputs without the mod's version changing
/// (the version is in the manifest too, so every release rebuilds once anyway).
pub const BAKE_FORMAT: u32 = 1;

const MAGIC: &[u8; 8] = b"KH1KH2SC";
/// The file's own layout (this module's), not the bake's.
const FILE_FORMAT: u32 = 1;
/// The cache file's name in its folder.
pub const FILE_NAME: &str = "kh2-motion-slots.bin";

/// FNV-1a, 128 bits (a content hash for change detection, not for security).
#[derive(Clone, Copy)]
pub struct Hasher(u128);

impl Default for Hasher {
    fn default() -> Self {
        Hasher(0x6c62_272e_07bb_0142_62b8_2175_6295_c58d)
    }
}

impl Hasher {
    const PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013B;

    /// Bytes as they are, without the length first (for a hash fed piece by piece).
    pub fn raw(&mut self, b: &[u8]) {
        for x in b {
            self.0 ^= *x as u128;
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    pub fn bytes(&mut self, b: &[u8]) -> &mut Self {
        // (The length first, so ("ab", "c") and ("a", "bc") differ.)
        for x in (b.len() as u64).to_le_bytes().iter().chain(b) {
            self.0 ^= *x as u128;
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
        self
    }

    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.bytes(&v.to_le_bytes())
    }

    pub fn str(&mut self, s: &str) -> &mut Self {
        self.bytes(s.as_bytes())
    }

    pub fn finish(&self) -> u128 {
        self.0
    }
}

pub fn hash(b: &[u8]) -> u128 {
    Hasher::default().bytes(b).finish()
}

/// One baked block, without a copy of its own: either baked this start (its own buffer) or a part
/// of the cache file read at start (that file's one buffer, shared by every block taken from it).
/// The cache's list of blocks to write back and `kh2slots`' build share the same buffers, so the
/// baked motions are in memory once while loading, and not at all once `kh2slots::place` has copied
/// them where KH1 plays them from and dropped its own (the tester, ram-use m1 answer 2 (a)).
#[derive(Clone)]
pub struct Block {
    buf: Arc<Vec<u8>>,
    range: Range<usize>,
}

impl Block {
    pub fn new(v: Vec<u8>) -> Block {
        let range = 0..v.len();
        Block { buf: Arc::new(v), range }
    }
}

impl Block {
    /// A part of this block (`range` within it), sharing its buffer (no copy).
    pub fn part(&self, range: Range<usize>) -> Block {
        assert!(range.start <= range.end && range.end <= self.range.len());
        Block { buf: self.buf.clone(), range: self.range.start + range.start..self.range.start + range.end }
    }
}

impl Deref for Block {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.buf[self.range.clone()]
    }
}

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Block({} bytes)", self.len())
    }
}

/// Everything the bake depends on, as `name = value` lines (readable in the file, logged on a
/// mismatch). Order-independent: lines are sorted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    lines: Vec<(String, String)>,
}

impl Manifest {
    pub fn new() -> Manifest {
        let mut m = Manifest::default();
        m.set("bake format", BAKE_FORMAT.to_string());
        m.set("mod version", crate::VERSION.to_string());
        m
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into().replace(['\n', '\r', '='], " ");
        let value = value.into().replace(['\n', '\r'], " ");
        match self.lines.iter_mut().find(|(k, _)| *k == key) {
            Some(l) => l.1 = value,
            None => self.lines.push((key, value)),
        }
        self.lines.sort();
    }

    /// An input file: its name and its contents' hash and size.
    pub fn file(&mut self, name: &str, bytes: &[u8]) {
        self.set(format!("file {name}"), format!("{:032x} {}", hash(bytes), bytes.len()));
    }

    pub fn text(&self) -> String {
        self.lines.iter().map(|(k, v)| format!("{k} = {v}\n")).collect()
    }

    /// A manifest from its text (`text`'s form), for other caches in the same folder
    /// (`soundcache`, `kh2dpx`).
    pub fn from_text(text: &str) -> Manifest {
        Manifest::parse(text)
    }

    fn parse(text: &str) -> Manifest {
        let mut m = Manifest::default();
        for l in text.lines() {
            if let Some((k, v)) = l.split_once(" = ") {
                m.lines.push((k.to_string(), v.to_string()));
            }
        }
        m.lines.sort();
        m
    }

    /// The keys whose values differ (or are missing on one side), for the log.
    pub fn differences(&self, other: &Manifest) -> Vec<String> {
        let mut out = Vec::new();
        for (k, v) in &self.lines {
            match other.lines.iter().find(|(k2, _)| k2 == k) {
                Some((_, v2)) if v2 == v => {}
                Some(_) => out.push(format!("{k} changed")),
                None => out.push(format!("{k} new")),
            }
        }
        for (k, _) in &other.lines {
            if !self.lines.iter().any(|(k2, _)| k2 == k) {
                out.push(format!("{k} gone"));
            }
        }
        out
    }
}

/// The cache for one start: the blocks read from disk (when the manifest matched), the blocks
/// baked this start, and whether the file needs writing.
#[derive(Default)]
pub struct Cache {
    manifest: Manifest,
    /// The blocks of the file on disk (parts of its one buffer), by key.
    loaded: HashMap<u128, Block>,
    /// Every block this start used (cached or baked), in order, to write back (shared with the
    /// caller's, not copies).
    used: Vec<(u128, Block)>,
    /// A block was baked (not found), or the file on disk didn't match: write it again.
    dirty: bool,
    pub hits: usize,
    pub misses: usize,
    /// Asks for a bake already made this start (same key).
    pub shared: usize,
}

/// Why the file on disk wasn't used.
#[derive(Debug, PartialEq, Eq)]
pub enum Miss {
    /// No file.
    Missing,
    /// Not ours, cut short, or its check failed.
    Damaged(String),
    /// Its manifest differs (the keys that differ).
    Changed(Vec<String>),
}

impl Cache {
    /// The cache for `manifest`, from `bytes` (the file's contents, if any; kept as the one buffer
    /// its blocks are taken from, no copy per block). The `Miss` says why nothing on disk could be
    /// used (`None`: the file matched).
    pub fn open(manifest: Manifest, bytes: Option<Vec<u8>>) -> (Cache, Option<Miss>) {
        let mut c = Cache { manifest, ..Cache::default() };
        let miss = match bytes {
            None => Some(Miss::Missing),
            Some(b) => match parse(&b) {
                Err(e) => Some(Miss::Damaged(e)),
                Ok((m, blocks)) => {
                    if m == c.manifest {
                        let buf = Arc::new(b);
                        c.loaded = blocks.into_iter().map(|(k, range)| (k, Block { buf: buf.clone(), range })).collect();
                        None
                    } else {
                        Some(Miss::Changed(c.manifest.differences(&m)))
                    }
                }
            },
        };
        c.dirty = miss.is_some();
        (c, miss)
    }

    /// The block for `key`, baking it with `bake` when the cache doesn't have it. A bake that
    /// fails is not cached (the caller falls back as before).
    pub fn get_or_bake<E>(&mut self, key: u128, bake: impl FnOnce() -> Result<Vec<u8>, E>) -> Result<Block, E> {
        // (The same bake asked for again this start (a KH2 finisher on two of our requests, with
        // the same template, length and events): the block already made, not baked again nor kept
        // twice. Before, the file's copy was taken by the first ask and every later one was baked
        // again at every start.)
        if let Some((_, b)) = self.used.iter().find(|(k, _)| *k == key) {
            self.shared += 1;
            return Ok(b.clone());
        }
        if let Some(b) = self.loaded.remove(&key) {
            self.hits += 1;
            self.used.push((key, b.clone()));
            return Ok(b);
        }
        let b = Block::new(bake()?);
        self.misses += 1;
        self.dirty = true;
        self.used.push((key, b.clone()));
        Ok(b)
    }

    /// Blocks on disk this start didn't ask for (a setting changed which motions are built): the
    /// file is written again without them.
    pub fn needs_write(&self) -> bool {
        self.dirty || !self.loaded.is_empty()
    }

    /// The file's contents for this start.
    pub fn encode(&self) -> Vec<u8> {
        encode(&self.manifest, &self.used)
    }

    /// Writes the file for this start into `dir` (as `write`), streamed from the blocks in memory
    /// rather than put together in one more buffer first.
    pub fn write(&self, dir: &Path) -> std::io::Result<PathBuf> {
        write_with(dir, |f| encode_to(&self.manifest, &self.used, f))
    }
}

/// The file: `MAGIC`, `FILE_FORMAT` (u32), the manifest's length (u32) and text, the block count
/// (u32), each block's key (u128) length (u32) and bytes, then the hash of everything before it.
pub fn encode<B: Deref<Target = [u8]>>(manifest: &Manifest, blocks: &[(u128, B)]) -> Vec<u8> {
    let mut out = Vec::new();
    encode_to(manifest, blocks, &mut out).expect("writing to a Vec");
    out
}

/// `encode`, written out piece by piece to `w`, the check hashed as it goes (the same bytes).
fn encode_to<B: Deref<Target = [u8]>>(manifest: &Manifest, blocks: &[(u128, B)], w: &mut impl std::io::Write) -> std::io::Result<()> {
    let text = manifest.text();
    let body_len = 8 + 4 + 4 + text.len() + 4 + blocks.iter().map(|b| 20 + b.1.len()).sum::<usize>();
    // (`hash(body)`: the body's length first, as `Hasher::bytes` does, then its bytes.)
    let mut h = Hasher::default();
    h.raw(&(body_len as u64).to_le_bytes());
    let mut put = |b: &[u8]| -> std::io::Result<()> {
        h.raw(b);
        w.write_all(b)
    };
    put(MAGIC)?;
    put(&FILE_FORMAT.to_le_bytes())?;
    put(&(text.len() as u32).to_le_bytes())?;
    put(text.as_bytes())?;
    put(&(blocks.len() as u32).to_le_bytes())?;
    for (k, b) in blocks {
        put(&k.to_le_bytes())?;
        put(&(b.len() as u32).to_le_bytes())?;
        put(b)?;
    }
    let check = h.finish();
    w.write_all(&check.to_le_bytes())
}

/// The file's manifest and its blocks (copied out; `parse` gives where they are instead).
#[allow(clippy::type_complexity)]
pub fn decode(b: &[u8]) -> Result<(Manifest, Vec<(u128, Vec<u8>)>), String> {
    let (m, blocks) = parse(b)?;
    Ok((m, blocks.into_iter().map(|(k, r)| (k, b[r].to_vec())).collect()))
}

/// The file's manifest and where each block is in it.
#[allow(clippy::type_complexity)]
fn parse(b: &[u8]) -> Result<(Manifest, Vec<(u128, Range<usize>)>), String> {
    if b.len() < MAGIC.len() + 16 + 12 || &b[..8] != MAGIC {
        return Err("not a slot cache".into());
    }
    let (body, tail) = b.split_at(b.len() - 16);
    if hash(body) != u128::from_le_bytes(tail.try_into().unwrap()) {
        return Err("check failed (cut short or damaged)".into());
    }
    let at = std::cell::Cell::new(8usize);
    let take_at = |n: usize| -> Result<Range<usize>, String> {
        let from = at.get();
        body.get(from..from + n).ok_or("cut short")?;
        at.set(from + n);
        Ok(from..from + n)
    };
    let take = |n: usize| -> Result<&[u8], String> { take_at(n).map(|r| &body[r]) };
    let format = u32::from_le_bytes(take(4)?.try_into().unwrap());
    if format != FILE_FORMAT {
        return Err(format!("file format {format}"));
    }
    let len = u32::from_le_bytes(take(4)?.try_into().unwrap()) as usize;
    let text = std::str::from_utf8(take(len)?).map_err(|_| "manifest not text")?;
    let manifest = Manifest::parse(text);
    let count = u32::from_le_bytes(take(4)?.try_into().unwrap()) as usize;
    let mut blocks = Vec::with_capacity(count.min(4096));
    for _ in 0..count {
        let key = u128::from_le_bytes(take(16)?.try_into().unwrap());
        let n = u32::from_le_bytes(take(4)?.try_into().unwrap()) as usize;
        blocks.push((key, take_at(n)?));
    }
    Ok((manifest, blocks))
}

/// Reads the file at `dir` / `FILE_NAME` (`None` when there is none or it can't be read).
pub fn read(dir: &Path) -> Option<Vec<u8>> {
    std::fs::read(dir.join(FILE_NAME)).ok()
}

/// Writes `bytes` as `dir` / `FILE_NAME`: to a temporary file first, then renamed over the old one
/// (replacing it), so a crash or a full disk mid-write never leaves a half file under the real
/// name. The temporary is removed on failure.
pub fn write(dir: &Path, bytes: &[u8]) -> std::io::Result<PathBuf> {
    write_with(dir, |f| f.write_all(bytes))
}

use std::io::Write;

fn write_with(dir: &Path, fill: impl FnOnce(&mut std::io::BufWriter<&mut std::fs::File>) -> std::io::Result<()>) -> std::io::Result<PathBuf> {
    write_named(dir, FILE_NAME, fill)
}

/// `write`'s atomic write (a temporary file renamed over the old one) for any file `name` in
/// `dir`, filled by `fill` (other caches in the same folder: `soundcache`).
pub fn write_named(dir: &Path, name: &str, fill: impl FnOnce(&mut std::io::BufWriter<&mut std::fs::File>) -> std::io::Result<()>) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(name);
    let tmp = dir.join(format!("{name}.{}.tmp", std::process::id()));
    let r = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        let mut w = std::io::BufWriter::new(&mut f);
        fill(&mut w)?;
        w.flush()?;
        drop(w);
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, &path)
    })();
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r.map(|_| path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(v: &str) -> Manifest {
        let mut m = Manifest::new();
        m.set("game build", "steam");
        m.file("xa_ex_0010.mset", v.as_bytes());
        m
    }

    #[test]
    fn round_trip_and_hit() {
        let m = manifest("a");
        let (mut c, miss) = Cache::open(m.clone(), None);
        assert_eq!(miss, Some(Miss::Missing));
        let b = c.get_or_bake::<()>(7, || Ok(vec![1, 2, 3])).unwrap();
        assert_eq!(&*b, &[1, 2, 3][..]);
        assert_eq!((c.hits, c.misses), (0, 1));
        assert!(c.needs_write());
        let file = c.encode();

        let (mut c2, miss) = Cache::open(m, Some(file.clone()));
        assert_eq!(miss, None);
        let b = c2.get_or_bake::<()>(7, || panic!("baked again")).unwrap();
        assert_eq!(&*b, &[1, 2, 3][..]);
        assert_eq!((c2.hits, c2.misses), (1, 0));
        assert!(!c2.needs_write());
    }

    #[test]
    fn changed_input_rebakes() {
        let (mut c, _) = Cache::open(manifest("a"), None);
        c.get_or_bake::<()>(7, || Ok(vec![9])).unwrap();
        let file = c.encode();
        let (mut c2, miss) = Cache::open(manifest("b"), Some(file.clone()));
        assert_eq!(miss, Some(Miss::Changed(vec!["file xa_ex_0010.mset changed".into()])));
        let mut baked = false;
        c2.get_or_bake::<()>(7, || {
            baked = true;
            Ok(vec![8])
        })
        .unwrap();
        assert!(baked && c2.needs_write());
    }

    #[test]
    fn damaged_file_is_ignored() {
        let (mut c, _) = Cache::open(manifest("a"), None);
        c.get_or_bake::<()>(1, || Ok(vec![5; 100])).unwrap();
        let mut file = c.encode();
        let n = file.len();
        file[n / 2] ^= 0xFF;
        assert!(matches!(Cache::open(manifest("a"), Some(file.clone())).1, Some(Miss::Damaged(_))));
        assert!(matches!(Cache::open(manifest("a"), Some(file[..n - 5].to_vec())).1, Some(Miss::Damaged(_))));
        assert!(matches!(Cache::open(manifest("a"), Some(b"nonsense".to_vec())).1, Some(Miss::Damaged(_))));
    }

    #[test]
    fn unused_blocks_are_dropped() {
        let (mut c, _) = Cache::open(manifest("a"), None);
        c.get_or_bake::<()>(1, || Ok(vec![1])).unwrap();
        c.get_or_bake::<()>(2, || Ok(vec![2])).unwrap();
        let file = c.encode();
        let (mut c2, _) = Cache::open(manifest("a"), Some(file.clone()));
        c2.get_or_bake::<()>(1, || panic!()).unwrap();
        assert!(c2.needs_write());
        let (_, blocks) = decode(&c2.encode()).unwrap();
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn failed_bake_not_cached() {
        let (mut c, _) = Cache::open(manifest("a"), None);
        assert!(c.get_or_bake(3, || Err("no")).is_err());
        assert_eq!(decode(&c.encode()).unwrap().1.len(), 0);
    }

    #[test]
    fn same_bake_twice_is_one_block() {
        let (mut c, _) = Cache::open(manifest("a"), None);
        let a = c.get_or_bake::<()>(5, || Ok(vec![4; 10])).unwrap();
        let b = c.get_or_bake::<()>(5, || panic!("baked twice")).unwrap();
        assert!(Arc::ptr_eq(&a.buf, &b.buf));
        assert_eq!((c.hits, c.misses, c.shared), (0, 1, 1));
        let file = c.encode();
        assert_eq!(decode(&file).unwrap().1.len(), 1);
        // (From the file: one hit, the second ask shares it, nothing baked.)
        let (mut c2, _) = Cache::open(manifest("a"), Some(file));
        c2.get_or_bake::<()>(5, || panic!()).unwrap();
        c2.get_or_bake::<()>(5, || panic!()).unwrap();
        assert_eq!((c2.hits, c2.misses, c2.shared), (1, 0, 1));
        assert!(!c2.needs_write());
    }

    #[test]
    fn streamed_write_is_the_encoding() {
        let (mut c, _) = Cache::open(manifest("a"), None);
        c.get_or_bake::<()>(1, || Ok(vec![1, 2, 3])).unwrap();
        c.get_or_bake::<()>(2, || Ok(vec![7; 300])).unwrap();
        let mut streamed = Vec::new();
        encode_to(&c.manifest, &c.used, &mut streamed).unwrap();
        assert_eq!(streamed, c.encode());
        // (A block taken from the file shares the file's buffer.)
        let (mut c2, miss) = Cache::open(manifest("a"), Some(streamed));
        assert_eq!(miss, None);
        let a = c2.get_or_bake::<()>(1, || panic!()).unwrap();
        let b = c2.get_or_bake::<()>(2, || panic!()).unwrap();
        assert!(Arc::ptr_eq(&a.buf, &b.buf));
        assert_eq!((&*a, &*b), (&[1u8, 2, 3][..], &[7u8; 300][..]));
    }

    #[test]
    fn atomic_write() {
        let dir = std::env::temp_dir().join(format!("slotcache-test-{}", std::process::id()));
        let (mut c, _) = Cache::open(manifest("a"), None);
        c.get_or_bake::<()>(1, || Ok(vec![1, 2])).unwrap();
        write(&dir, &c.encode()).unwrap();
        c.write(&dir).unwrap();
        let back = read(&dir).unwrap();
        assert_eq!(Cache::open(manifest("a"), Some(back)).1, None);
        let left: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).filter(|e| e.file_name().to_string_lossy().ends_with(".tmp")).collect();
        assert!(left.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
