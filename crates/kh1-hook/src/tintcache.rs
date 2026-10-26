//! The Drive Form recolour cache (the tester, ram-use / what-is-still-missing: the assets the mod
//! needs built once from the player's own files into the mod folder, rebuilt only when those
//! change). `docs/kh1-asset-cache.md` §6, `docs/kh1-tint.md`.
//!
//! `win::tint` recolours Sora's HD textures for a form's look (Wisdom's blue, Valor's red): one
//! texture per slot and look. Each result (its B8G8R8A8 pixels, ready for KH1's texture creator) is
//! kept as its own file in `kh1-kh2-cache` beside the DLL, in the slot cache's file layout
//! (`slotcache`: a manifest, one block, a check hash at the end, written to a temporary file and
//! renamed). The manifest names what the pixels were made from:
//! - the format here (`TINT_FORMAT`), the mod's version, the game build;
//! - the look and the slot, the source texture's name and size;
//! - the source DDS's content hash, and where it came from: KH1's copy in memory, or the PKG with
//!   the PKG entry's identity (package, entry offset and lengths, the `.pkg`'s size and time).
//!
//! A file is used when its fixed lines match, and either the source in hand has the same content
//! hash (KH1's copy in memory) or, with no copy in memory, it was made from the PKG and the PKG
//! entry is the same (so the PKG isn't read at all). Anything else (missing, damaged, changed) is
//! rebuilt and the file written again. No ini key changes the recolour (`drive` only switches it
//! on), so none is in the manifest.
//!
//! Pure Rust (no Windows calls), unit tested on any host.

use crate::slotcache::{self, Block, Manifest};
use std::path::{Path, PathBuf};

/// Bump when the recolour's output changes for the same inputs without the mod's version changing.
pub const TINT_FORMAT: u32 = 1;

/// What one recoloured texture is: the look, the slot (the remastered mesh table's texture slot,
/// the asset `-1c<slot>.dds`), the source texture's name and size, the game build.
#[derive(Clone, Debug)]
pub struct Desc {
    pub look: String,
    pub slot: usize,
    pub name: String,
    pub w: u32,
    pub h: u32,
    pub build: String,
}

impl Desc {
    /// The file's name in the cache folder: one per look and slot (`form-tint-wisdom-1c1.bin`).
    pub fn file_name(&self) -> String {
        format!("form-tint-{}-1c{}.bin", self.look.to_ascii_lowercase(), self.slot)
    }

    /// The pixel data's length (B8G8R8A8).
    pub fn len(&self) -> usize {
        self.w as usize * self.h as usize * 4
    }

    fn fixed(&self) -> Manifest {
        let mut m = Manifest::default();
        m.set("tint format", TINT_FORMAT.to_string());
        m.set("mod version", crate::VERSION.to_string());
        m.set("game build", self.build.clone());
        m.set("look", self.look.clone());
        m.set("slot", self.slot.to_string());
        m.set("source name", self.name.clone());
        m.set("source size", format!("{}x{}", self.w, self.h));
        m
    }
}

/// How the source in hand is known: by its contents (KH1's copy in memory, hashed), or, with no
/// copy in memory, by the PKG entry it would be read from (`pkg_entry`'s text).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    Content(u128),
    Pkg(String),
}

/// Where the pixels written were made from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    Memory,
    Pkg(String),
}

/// The source DDS's hash: its first 128 + w·h·4 bytes (the header and the one level read), so
/// KH1's copy in memory and the PKG's file give the same hash for the same texture.
pub fn source_hash(dds: &[u8], d: &Desc) -> u128 {
    slotcache::hash(&dds[..dds.len().min(128 + d.len())])
}

/// Why a cached file wasn't used.
#[derive(Debug, PartialEq, Eq)]
pub enum Miss {
    Missing,
    Damaged(String),
    Changed(Vec<String>),
}

impl std::fmt::Display for Miss {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Miss::Missing => write!(f, "none yet"),
            Miss::Damaged(e) => write!(f, "unusable ({e})"),
            Miss::Changed(d) => write!(f, "changed: {}", d.join(", ")),
        }
    }
}

fn manifest(d: &Desc, source: u128, origin: &Origin) -> Manifest {
    let mut m = d.fixed();
    m.set("source hash", format!("{source:032x}"));
    match origin {
        Origin::Memory => m.set("source from", "KH1's copy in memory"),
        Origin::Pkg(id) => {
            m.set("source from", "pkg");
            m.set("pkg entry", id.clone());
        }
    }
    m
}

/// The cached pixels for `d` known by `key`, from `bytes` (the file's contents, if any), as a part
/// of that one buffer (no copy).
pub fn check(d: &Desc, key: &Key, bytes: Option<Vec<u8>>) -> Result<Block, Miss> {
    let b = bytes.ok_or(Miss::Missing)?;
    let (m, mut blocks) = slotcache::decode_shared(b).map_err(Miss::Damaged)?;
    let fixed = d.fixed();
    let mut differ: Vec<String> = fixed.differences(&m).into_iter().filter(|x| !x.ends_with(" gone")).collect();
    let field = |k: &str| m.text().lines().find_map(|l| l.split_once(" = ").filter(|(a, _)| *a == k).map(|(_, v)| v.to_string()));
    match key {
        Key::Content(h) => {
            if field("source hash").as_deref() != Some(&format!("{h:032x}")) {
                differ.push("source hash changed".into());
            }
        }
        Key::Pkg(id) => {
            if field("source from").as_deref() != Some("pkg") {
                differ.push("made from KH1's copy in memory, which isn't there now".into());
            } else if field("pkg entry").as_deref() != Some(id.as_str()) {
                differ.push("pkg entry changed".into());
            }
        }
    }
    if !differ.is_empty() {
        return Err(Miss::Changed(differ));
    }
    if blocks.len() != 1 || blocks[0].1.len() != d.len() {
        return Err(Miss::Damaged(format!("{} block(s), not one of {} bytes", blocks.len(), d.len())));
    }
    Ok(blocks.pop().unwrap().1)
}

/// `check` on the file in `dir`.
pub fn load(dir: &Path, d: &Desc, key: &Key) -> Result<Block, Miss> {
    check(d, key, std::fs::read(dir.join(d.file_name())).ok())
}

/// The pixels for `d` (made from a source with hash `source`, from `origin`) written into `dir`.
pub fn store(dir: &Path, d: &Desc, source: u128, origin: &Origin, pixels: &[u8]) -> std::io::Result<PathBuf> {
    slotcache::write_named_blocks(dir, &d.file_name(), &manifest(d, source, origin), &[(0u128, pixels)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desc() -> Desc {
        Desc { look: "Wisdom".into(), slot: 1, name: "xa_ex_0010mdls1c1".into(), w: 2, h: 2, build: "Steam".into() }
    }

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kh1kh2-tintcache-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn content_key_round_trip() {
        let d = desc();
        let dir = dir("content");
        let px = vec![7u8; d.len()];
        assert_eq!(load(&dir, &d, &Key::Content(5)).unwrap_err(), Miss::Missing);
        store(&dir, &d, 5, &Origin::Memory, &px).unwrap();
        assert_eq!(&*load(&dir, &d, &Key::Content(5)).unwrap(), &px[..]);
        // Another source: rebuilt.
        assert!(matches!(load(&dir, &d, &Key::Content(6)), Err(Miss::Changed(_))));
        // Made from memory: no use with only the PKG to go by.
        assert!(matches!(load(&dir, &d, &Key::Pkg("a".into())), Err(Miss::Changed(_))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pkg_key_and_content_key() {
        let d = desc();
        let dir = dir("pkg");
        let px = vec![1u8; d.len()];
        store(&dir, &d, 9, &Origin::Pkg("first 0x10 1 2".into()), &px).unwrap();
        assert!(load(&dir, &d, &Key::Pkg("first 0x10 1 2".into())).is_ok());
        assert!(matches!(load(&dir, &d, &Key::Pkg("first 0x20 1 2".into())), Err(Miss::Changed(_))));
        // The same texture in memory later: its hash matches the PKG's.
        assert!(load(&dir, &d, &Key::Content(9)).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn version_size_and_damage() {
        let d = desc();
        let dir = dir("damage");
        store(&dir, &d, 1, &Origin::Memory, &vec![0u8; d.len()]).unwrap();
        let mut other = d.clone();
        other.look = "Valor".into();
        // (Another look is another file.)
        assert_eq!(load(&dir, &other, &Key::Content(1)).unwrap_err(), Miss::Missing);
        let mut big = d.clone();
        big.w = 4;
        big.look = "Wisdom".into();
        assert!(matches!(load(&dir, &big, &Key::Content(1)), Err(Miss::Changed(_))));
        let path = dir.join(d.file_name());
        let mut b = std::fs::read(&path).unwrap();
        let n = b.len();
        b[n - 20] ^= 0xFF;
        std::fs::write(&path, &b).unwrap();
        assert!(matches!(load(&dir, &d, &Key::Content(1)), Err(Miss::Damaged(_))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hash_ignores_trailing_bytes() {
        let d = desc();
        let a = vec![3u8; 128 + d.len()];
        let mut b = a.clone();
        b.extend_from_slice(&[9, 9, 9]);
        assert_eq!(source_hash(&a, &d), source_hash(&b, &d));
    }
}
