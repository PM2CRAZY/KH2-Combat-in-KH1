//! `kh1-hook`: the KH2 combat rewrite inside Kingdom Hearts Final Mix (PC, KH 1.5+2.5 ReMIX).
//!
//! `KINGDOM HEARTS FINAL MIX.exe` loads `dinput8.dll` from its own folder before the system
//! one, so this library is built as that DLL (64-bit). The folder is shared with KH2, Birth by
//! Sleep and Re:Chain of Memories, which may load it too: it only forwards
//! `DirectInput8Create` to the real system DLL there, and only acts inside KH1.
//!
//! Stage 1 (now): load, log, prove it runs alongside OpenKH's Panacea and LuaBackend.

#![allow(clippy::missing_safety_doc)]

pub mod growth;
pub mod kh1phys;

/// The mod's version as the user sees it (the Config menu's Camera Style help line, the log):
/// the stage of the latest `AGENTS.md` entry, bumped with every change to the source (a test
/// checks). `t` for the testing branch (the user, from stage 300; `s` before it).
pub const VERSION: &str = "t512";

#[cfg(all(windows, target_arch = "x86_64"))]
mod win;

#[cfg(all(windows, target_arch = "x86_64"))]
pub use win::*;
