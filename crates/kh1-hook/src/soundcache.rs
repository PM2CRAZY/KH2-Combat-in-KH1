//! The sound cache (the tester, ram-use m1 / what-is-still-missing: "do all three ... go ahead",
//! the assets built once into the mod folder). `docs/kh1-asset-cache.md` §4.
//!
//! The KH2 sounds the mod plays through KH1's engine (`win::kh2sound`, `win::kh1bank`) are each a
//! one-sound bank cut out of one of KH2's sound banks (`kh2_formats::scd_cut`), plus Sora's battle
//! voice bank as it is. Cutting them means reading KH2's effect package (to know which sound each
//! effect element names), three whole sound banks (about 28 MB, decompressed from KH2's packages),
//! `00battle.bin` and the voice bank at every start. This keeps what the cutting made in one file
//! next to the DLL (`kh1-kh2-cache\kh2-sounds.bin`, beside the motion slot cache), so later starts
//! read only that file (about the size of the sounds themselves).
//!
//! The file starts with a manifest (`slotcache::Manifest`'s text): what was asked for (the cache's
//! format, the mod's version, `kh2_volume`, a hash of the effect elements, sound ids, spell
//! elements and voices wanted) and, for every KH2 file the cutting read, a stamp of where it is in
//! KH2's packages (`kh2_data::Install::stamp_any`: archive, index entry, the `.pkg`'s size and
//! time), so a changed file is noticed without reading it. Anything different: the file is
//! ignored and cut again. The file ends with a hash of everything before it (a damaged or
//! cut-short file is cut again); it is written to a temporary file and renamed.
//!
//! Memory: the file is read piece by piece straight into each sound's own buffer (no whole-file
//! buffer and no copy of each sound): those buffers are what `kh1bank` takes over, so each sound
//! is in memory once, as before.
//!
//! Pure Rust (no Windows calls), unit tested on any host.

use crate::slotcache::{Hasher, Manifest};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Bump when what the cutting makes changes for the same inputs without the mod's version
/// changing (the version is in the manifest too).
pub const FORMAT: u32 = 1;
const MAGIC: &[u8; 8] = b"KH1KH2SN";
/// The cache file's name, in the same folder as the motion slot cache.
pub const FILE_NAME: &str = "kh2-sounds.bin";
/// A manifest key naming a file the cutting read (its value is the file's stamp).
pub const FILE_KEY: &str = "file ";

/// Who wants a sound: an effect element of Sora's package, a sound id (a frame trigger 8), or a
/// spell package's element (level or package number, element).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Want {
    Element(i16),
    Sound(u32),
    Spell(u8, u16),
}

/// One sound cut out of KH2 bank `bank` (its sound `index`), ready for KH1's engine (`bytes`, as
/// `kh1bank::add` takes it), its length, and what the log says about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cut {
    pub bank: u32,
    pub index: u32,
    pub ms: Option<u32>,
    pub info: String,
    pub bytes: Vec<u8>,
}

/// A KH2 battle voice of Sora's: its id, priority, (clip, chance) choices and playable clips.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Voice {
    pub id: i16,
    pub priority: u8,
    pub choices: Vec<(i8, i8)>,
    pub clips: Vec<u8>,
}

/// Everything the cutting made.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sounds {
    pub cuts: Vec<Cut>,
    /// Each want and the cut it plays (an index into `cuts`).
    pub wants: Vec<(Want, u32)>,
    pub voices: Vec<Voice>,
    /// Sora's whole battle voice bank (for KH1's voice list), when any voice plays.
    pub voice_bank: Option<Vec<u8>>,
}

/// Why the file wasn't used.
#[derive(Debug, PartialEq, Eq)]
pub enum Miss {
    Missing,
    Damaged(String),
    Changed(Vec<String>),
}

/// A manifest for what is asked: the cache's format and the mod's version (more lines are added
/// by the caller: settings, the wants' hash, then the files read).
pub fn manifest() -> Manifest {
    let mut m = Manifest::default();
    m.set("sound cache format", FORMAT.to_string());
    m.set("mod version", crate::VERSION.to_string());
    m
}

/// Reads `dir` / `FILE_NAME` when its manifest is `request` plus, for every file it lists, that
/// file's stamp now (`stamp(name)`). Only the manifest is read when it doesn't match.
pub fn open(dir: &Path, request: &Manifest, stamp: impl Fn(&str) -> String) -> Result<Sounds, Miss> {
    let path = dir.join(FILE_NAME);
    let Ok(f) = std::fs::File::open(&path) else { return Err(Miss::Missing) };
    let len = f.metadata().map_or(0, |m| m.len());
    let mut r = Reader { r: std::io::BufReader::new(f), h: Hasher::default(), left: len.saturating_sub(16) };
    let damaged = |e: std::io::Error| Miss::Damaged(e.to_string());
    let mut magic = [0u8; 8];
    r.exact(&mut magic).map_err(damaged)?;
    if &magic != MAGIC {
        return Err(Miss::Damaged("not a sound cache".into()));
    }
    let format = r.u32().map_err(damaged)?;
    if format != FORMAT {
        return Err(Miss::Damaged(format!("file format {format}")));
    }
    let text = r.bytes().map_err(damaged)?;
    let stored = Manifest::from_text(&String::from_utf8_lossy(&text));
    let mut now = request.clone();
    for line in String::from_utf8_lossy(&text).lines() {
        if let Some((k, _)) = line.split_once(" = ") {
            if let Some(name) = k.strip_prefix(FILE_KEY) {
                now.set(k, stamp(name));
            }
        }
    }
    if now != stored {
        return Err(Miss::Changed(now.differences(&stored)));
    }
    let sounds = read_payload(&mut r).map_err(damaged)?;
    let check = r.h.finish();
    let mut tail = [0u8; 16];
    r.r.read_exact(&mut tail).map_err(damaged)?;
    if u128::from_le_bytes(tail) != check || r.r.read(&mut [0u8; 1]).map_or(true, |n| n != 0) {
        return Err(Miss::Damaged("check failed (cut short or damaged)".into()));
    }
    Ok(sounds)
}

fn read_payload(r: &mut Reader<impl Read>) -> std::io::Result<Sounds> {
    let mut s = Sounds::default();
    for _ in 0..r.u32()? {
        let bank = r.u32()?;
        let index = r.u32()?;
        let ms = r.u32()?;
        let info = String::from_utf8_lossy(&r.bytes()?).into_owned();
        let bytes = r.bytes()?;
        s.cuts.push(Cut { bank, index, ms: (ms != u32::MAX).then_some(ms), info, bytes });
    }
    for _ in 0..r.u32()? {
        let tag = r.u8()?;
        let a = r.u32()?;
        let b = r.u32()?;
        let cut = r.u32()?;
        let want = match tag {
            0 => Want::Element(a as i16),
            1 => Want::Sound(a),
            2 => Want::Spell(a as u8, b as u16),
            _ => return Err(bad("want kind")),
        };
        if cut as usize >= s.cuts.len() {
            return Err(bad("want's sound"));
        }
        s.wants.push((want, cut));
    }
    for _ in 0..r.u32()? {
        let id = r.u32()? as i16;
        let priority = r.u8()?;
        let choices = r.bytes()?.chunks_exact(2).map(|c| (c[0] as i8, c[1] as i8)).collect();
        let clips = r.bytes()?;
        s.voices.push(Voice { id, priority, choices, clips });
    }
    if r.u8()? != 0 {
        s.voice_bank = Some(r.bytes()?);
    }
    Ok(s)
}

fn bad(what: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, format!("bad {what}"))
}

/// A reader hashing what it reads, refusing a length longer than the file has left (a damaged
/// length is never allocated).
struct Reader<R> {
    r: R,
    h: Hasher,
    left: u64,
}

impl<R: Read> Reader<R> {
    fn exact(&mut self, b: &mut [u8]) -> std::io::Result<()> {
        if b.len() as u64 > self.left {
            return Err(bad("length (cut short)"));
        }
        self.r.read_exact(b)?;
        self.left -= b.len() as u64;
        self.h.raw(b);
        Ok(())
    }
    fn u8(&mut self) -> std::io::Result<u8> {
        let mut b = [0u8; 1];
        self.exact(&mut b)?;
        Ok(b[0])
    }
    fn u32(&mut self) -> std::io::Result<u32> {
        let mut b = [0u8; 4];
        self.exact(&mut b)?;
        Ok(u32::from_le_bytes(b))
    }
    /// A length-prefixed run of bytes, in a buffer of its own.
    fn bytes(&mut self) -> std::io::Result<Vec<u8>> {
        let n = self.u32()? as u64;
        if n > self.left {
            return Err(bad("length (cut short)"));
        }
        let mut v = vec![0u8; n as usize];
        self.exact(&mut v)?;
        Ok(v)
    }
}

/// A writer hashing what it writes.
struct Writer<'a, W: Write> {
    w: &'a mut W,
    h: Hasher,
}

impl<W: Write> Writer<'_, W> {
    fn put(&mut self, b: &[u8]) -> std::io::Result<()> {
        self.h.raw(b);
        self.w.write_all(b)
    }
    fn u32(&mut self, v: u32) -> std::io::Result<()> {
        self.put(&v.to_le_bytes())
    }
    fn bytes(&mut self, b: &[u8]) -> std::io::Result<()> {
        self.u32(u32::try_from(b.len()).map_err(|_| bad("size"))?)?;
        self.put(b)
    }
}

/// The file's contents for `manifest` (the request plus the files read, with their stamps) and
/// `sounds`, written piece by piece to `w`.
pub fn encode_to(manifest: &Manifest, sounds: &Sounds, w: &mut impl Write) -> std::io::Result<()> {
    let mut o = Writer { w, h: Hasher::default() };
    o.put(MAGIC)?;
    o.u32(FORMAT)?;
    o.bytes(manifest.text().as_bytes())?;
    o.u32(sounds.cuts.len() as u32)?;
    for c in &sounds.cuts {
        o.u32(c.bank)?;
        o.u32(c.index)?;
        o.u32(c.ms.unwrap_or(u32::MAX))?;
        o.bytes(c.info.as_bytes())?;
        o.bytes(&c.bytes)?;
    }
    o.u32(sounds.wants.len() as u32)?;
    for (w, cut) in &sounds.wants {
        let (tag, a, b) = match *w {
            Want::Element(e) => (0u8, e as u16 as u32, 0),
            Want::Sound(s) => (1, s, 0),
            Want::Spell(l, e) => (2, l as u32, e as u32),
        };
        o.put(&[tag])?;
        o.u32(a)?;
        o.u32(b)?;
        o.u32(*cut)?;
    }
    o.u32(sounds.voices.len() as u32)?;
    for v in &sounds.voices {
        o.u32(v.id as u16 as u32)?;
        o.put(&[v.priority])?;
        o.bytes(&v.choices.iter().flat_map(|&(c, p)| [c as u8, p as u8]).collect::<Vec<u8>>())?;
        o.bytes(&v.clips)?;
    }
    match &sounds.voice_bank {
        Some(b) => {
            o.put(&[1])?;
            o.bytes(b)?;
        }
        None => o.put(&[0])?,
    }
    let check = o.h.finish();
    o.w.write_all(&check.to_le_bytes())
}

/// Writes the cache into `dir` (atomically: `slotcache::write_named`).
pub fn write(dir: &Path, manifest: &Manifest, sounds: &Sounds) -> std::io::Result<PathBuf> {
    crate::slotcache::write_named(dir, FILE_NAME, |w| encode_to(manifest, sounds, w))
}

/// A hash of what is asked for (the want list's text), for the manifest.
pub fn wants_hash(text: &str) -> String {
    format!("{:032x}", Hasher::default().str(text).finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kh-soundcache-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn sample() -> Sounds {
        Sounds {
            cuts: vec![
                Cut { bank: 999, index: 38, ms: Some(1200), info: "bus 2".into(), bytes: vec![1, 2, 3, 4] },
                Cut { bank: 500, index: 42, ms: None, info: String::new(), bytes: vec![9; 40] },
            ],
            wants: vec![(Want::Element(-3), 0), (Want::Sound(500042), 1), (Want::Spell(2, 11), 0)],
            voices: vec![Voice { id: 6, priority: 3, choices: vec![(1, 50), (-1, 0)], clips: vec![1] }],
            voice_bank: Some(vec![7; 100]),
        }
    }

    fn request() -> Manifest {
        let mut m = manifest();
        m.set("ini kh2_volume", "1");
        m
    }

    fn with_files(stamp: &str) -> Manifest {
        let mut m = request();
        m.set(format!("{FILE_KEY}se/se999.win32.scd"), stamp);
        m.set(format!("{FILE_KEY}00battle.bin"), "b");
        m
    }

    #[test]
    fn round_trip() {
        let d = dir("rt");
        assert_eq!(open(&d, &request(), |_| String::new()), Err(Miss::Missing));
        write(&d, &with_files("a"), &sample()).unwrap();
        let got = open(&d, &request(), |n| if n == "00battle.bin" { "b".into() } else { "a".into() }).unwrap();
        assert_eq!(got, sample());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_changed_file_or_setting_is_cut_again() {
        let d = dir("changed");
        write(&d, &with_files("a"), &sample()).unwrap();
        let stamp = |n: &str| if n == "00battle.bin" { "b".to_string() } else { "a2".to_string() };
        assert_eq!(open(&d, &request(), stamp), Err(Miss::Changed(vec!["file se/se999.win32.scd changed".into()])));
        let mut other = request();
        other.set("ini kh2_volume", "0.5");
        assert!(matches!(open(&d, &other, |n| if n == "00battle.bin" { "b".into() } else { "a".into() }), Err(Miss::Changed(_))));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_damaged_file_is_cut_again() {
        let d = dir("damaged");
        let p = write(&d, &with_files("a"), &sample()).unwrap();
        let stamp = |n: &str| if n == "00battle.bin" { "b".to_string() } else { "a".to_string() };
        let mut b = std::fs::read(&p).unwrap();
        let n = b.len();
        b[n - 30] ^= 0xFF;
        std::fs::write(&p, &b).unwrap();
        assert!(matches!(open(&d, &request(), stamp), Err(Miss::Damaged(_))));
        std::fs::write(&p, &b[..n / 2]).unwrap();
        assert!(matches!(open(&d, &request(), stamp), Err(Miss::Damaged(_))));
        let _ = std::fs::remove_dir_all(&d);
    }
}
