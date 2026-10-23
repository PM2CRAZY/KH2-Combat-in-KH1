//! # kh1-data
//!
//! Kingdom Hearts Final Mix's files, read from the user's own PC install (KH 1.5+2.5 ReMIX).
//! The packages (`Image/<lang>/kh1_first..fifth.hed/.pkg`) use the same format and cipher as
//! KH2's (`kh2_formats::egs`); the key table comes from the KH1 executable. Files are found by
//! the MD5 of their original name (`xa_ex_0010.mdls`, `btltbl.bin`, ...). Nothing from the game
//! is stored in this repository.
//!
//! A mod manager (OpenKH's) keeps the originals of the packages it patched in `BackupImage/`;
//! [`Source::Original`] reads those first, so a randomizer's edits don't leak into our tables.

pub mod inverse;
pub mod jiggle;
pub mod mdls;
pub mod mset;
pub mod retarget;
pub mod secondary;
pub mod texmap;

use kh2_formats::egs::{Archive, KeyTable};
use kh2_formats::{Error, Result};
use std::path::{Path, PathBuf};

/// The five KH1 packages, in the order searched.
pub const ARCHIVES: [&str; 5] = ["kh1_first", "kh1_second", "kh1_third", "kh1_fourth", "kh1_fifth"];
pub const EXECUTABLE: &str = "KINGDOM HEARTS FINAL MIX.exe";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// What the game loads now (mods included).
    Live,
    /// The unmodded package where a mod manager kept one (`BackupImage/`), else the live one.
    Original,
}

pub struct Kh1Install {
    pub root: PathBuf,
    pub image_dir: PathBuf,
}

impl Kh1Install {
    /// The install folder (the one holding `KINGDOM HEARTS FINAL MIX.exe`) or the executable.
    pub fn locate(path: impl AsRef<Path>) -> Result<Kh1Install> {
        let p = path.as_ref();
        let root = if p.is_file() { p.parent().unwrap_or(Path::new(".")).to_path_buf() } else { p.to_path_buf() };
        if !root.join(EXECUTABLE).is_file() {
            return Err(Error::Format(format!("{EXECUTABLE} not found in {}", root.display())));
        }
        let image_dir = ["en", "dt", "jp"]
            .iter()
            .map(|l| root.join("Image").join(l))
            .find(|d| d.join("kh1_first.hed").is_file())
            .ok_or_else(|| Error::Format(format!("kh1_first.hed not found under {}", root.join("Image").display())))?;
        Ok(Kh1Install { root, image_dir })
    }

    pub fn key_table(&self) -> Result<KeyTable> {
        KeyTable::from_executable(self.root.join(EXECUTABLE))
    }

    /// The `.hed` of one package, the backup copy first for [`Source::Original`].
    fn hed(&self, source: Source, part: &str) -> PathBuf {
        let live = self.image_dir.join(format!("{part}.hed"));
        if source == Source::Original {
            let backup = self.root.join("BackupImage").join(format!("{part}.hed"));
            if backup.is_file() && backup.with_extension("pkg").is_file() {
                return backup;
            }
        }
        live
    }

    /// Reads one of a file's remastered (HD) assets (`asset`: the start of its name, e.g.
    /// `-1c1` for a model's second HD texture), with its full name.
    pub fn read_remastered(&self, source: Source, name: &str, asset: &str) -> Result<(String, Vec<u8>)> {
        let keys = self.key_table()?;
        for part in ARCHIVES {
            let hed = self.hed(source, part);
            if !hed.is_file() {
                continue;
            }
            let archive = Archive::open(&hed)?;
            if archive.find(name).is_some() {
                return archive.read_remastered(&keys, name, asset);
            }
        }
        Err(Error::NotFound(name.to_string()))
    }

    /// The packages opened once, with the key table built once, for several reads. A package
    /// whose index doesn't read is left out (the user's `kh1_fifth.hed` has an entry with a
    /// negative length, which `Archive::open` refuses).
    pub fn files(&self, source: Source) -> Result<Files> {
        let archives: Vec<Archive> = ARCHIVES.iter().map(|part| self.hed(source, part)).filter(|h| h.is_file()).filter_map(|h| Archive::open(h).ok()).collect();
        if archives.is_empty() {
            return Err(Error::Format(format!("no KH1 package read under {}", self.image_dir.display())));
        }
        Ok(Files { keys: self.key_table()?, archives })
    }

    /// Reads a file by its original name from whichever package holds it.
    pub fn read(&self, source: Source, name: &str) -> Result<Vec<u8>> {
        let keys = self.key_table()?;
        for part in ARCHIVES {
            let hed = self.hed(source, part);
            if !hed.is_file() {
                continue;
            }
            let archive = Archive::open(&hed)?;
            if archive.find(name).is_some() {
                return archive.read(&keys, name);
            }
        }
        Err(Error::NotFound(name.to_string()))
    }
}

/// KH1's packages, opened (`Kh1Install::files`).
pub struct Files {
    keys: KeyTable,
    archives: Vec<Archive>,
}

impl Files {
    fn holding(&self, name: &str) -> Result<&Archive> {
        self.archives.iter().find(|a| a.find(name).is_some()).ok_or_else(|| Error::NotFound(name.to_string()))
    }

    /// A file by its original name.
    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        self.holding(name)?.read(&self.keys, name)
    }

    /// One of a file's remastered (HD) assets (`Kh1Install::read_remastered`).
    pub fn read_remastered(&self, name: &str, asset: &str) -> Result<(String, Vec<u8>)> {
        self.holding(name)?.read_remastered(&self.keys, name, asset)
    }
}
