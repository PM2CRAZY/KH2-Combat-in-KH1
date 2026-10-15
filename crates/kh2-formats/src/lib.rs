//! # kh2-formats
//!
//! Readers for the Kingdom Hearts II Final Mix PC (Epic Games Store) data
//! files, used to pull combat tables out of a copy of the game the user owns.
//!
//! - [`egs`]: the `kh2_*.hed` / `kh2_*.pkg` archive pairs
//! - [`bar`]: the BAR container format used by most KH2 files
//! - [`battle`]: typed tables inside `00battle.bin`
//! - [`motion`]: movesets (MSET/ANB), motion timing, motion triggers, model collision
//! - [`pref`]: movement parameters from `03system.bin`
//! - [`pax`], [`dpd`]: effect packages (elements, emitter programs, textures, sprites, models)
//!
//! Format knowledge comes from the OpenKH project (Apache-2.0). Parts of
//! this crate are ports of OpenKH's C# readers; see `NOTICE` and
//! `CREDITS.md` at the repository root. No game data is embedded here. The
//! one piece of the archive cipher that comes from the game (a 256-byte
//! substitution table) is read from the user's own executable at runtime;
//! this crate only stores its SHA-256 to recognise it.

pub mod anim;
pub mod bar;
pub mod dpd;
pub mod battle;
pub mod egs;
pub mod motion;
pub mod msg;
pub mod pax;
pub mod scd;
pub mod objentry;
pub mod pref;
pub mod system;
mod reader;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Format(String),
    #[error("file not found in archive: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn bad<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Format(msg.into()))
}
