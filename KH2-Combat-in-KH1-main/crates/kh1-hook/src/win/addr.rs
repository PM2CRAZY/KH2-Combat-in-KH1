//! Game addresses for both PC releases of KINGDOM HEARTS FINAL MIX (1.5+2.5): the Epic Games
//! Store build and the Steam build. They are the same game compiled the same day, but each links
//! its own store's code, so functions and globals sit at different places. Every address in this
//! mod is written as the Epic build's RVA in an `Rva`; `base + rva` gives the address in the build
//! that is running (`STEAM` holds the Steam RVA of each one).
//!
//! The Steam table was made by matching the two executables function by function (14,827 of
//! 15,824 functions identical once relative offsets are set aside) and pairing the globals their
//! RIP-relative operands point at; each code address was then checked instruction by
//! instruction, with every reference it makes mapping to the same place.

use std::sync::atomic::{AtomicU8, Ordering};

/// An address in the game, as its RVA in the Epic Games Store build.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rva(pub usize);

impl Rva {
    /// This address's RVA in the running build.
    pub fn get(self) -> usize {
        match Build::current() {
            Build::Steam => steam(self.0),
            _ => self.0,
        }
    }
}

impl std::ops::Add<Rva> for usize {
    type Output = usize;
    fn add(self, r: Rva) -> usize {
        self + r.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Build {
    Unknown,
    Epic,
    Steam,
}

static BUILD: AtomicU8 = AtomicU8::new(0);

impl Build {
    pub fn current() -> Build {
        match BUILD.load(Ordering::Relaxed) {
            1 => Build::Epic,
            2 => Build::Steam,
            _ => Build::Unknown,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Build::Epic => "Epic Games Store",
            Build::Steam => "Steam",
            Build::Unknown => "unknown",
        }
    }
}

/// The two supported builds' PE link timestamps.
const EPIC_STAMP: u32 = 1_721_648_200;
const STEAM_STAMP: u32 = 1_721_645_006;

/// Works out which build is running (its link timestamp, then a few functions' first bytes at
/// their places in that build). Unknown builds are left alone.
pub unsafe fn detect(base: usize) -> Build {
    let rd32 = |at: usize| std::ptr::read_unaligned(at as *const u32);
    let pe = base + rd32(base + 0x3C) as usize;
    let stamp = rd32(pe + 8);
    let build = match stamp {
        EPIC_STAMP => Build::Epic,
        STEAM_STAMP => Build::Steam,
        _ => Build::Unknown,
    };
    BUILD.store(match build { Build::Epic => 1, Build::Steam => 2, Build::Unknown => 0 }, Ordering::Relaxed);
    if build == Build::Unknown {
        return build;
    }
    // KH1's actor physics step, player control step and reward message line: the same first bytes
    // in both builds.
    let signs: [(Rva, &[u8]); 3] = [
        (Rva(0x298230), &[0x40, 0x55, 0x53, 0x41, 0x56, 0x48, 0x8B, 0xEC, 0x48, 0x83, 0xEC, 0x50]),
        (Rva(0x2A4090), &[0x48, 0x89, 0x5C, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x40]),
        (Rva(0x1B6990), &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x20]),
    ];
    let ok = signs.iter().all(|(rva, bytes)| {
        let at = base + *rva;
        super::crash::readable(at + bytes.len()) && std::slice::from_raw_parts(at as *const u8, bytes.len()) == *bytes
    });
    if !ok {
        BUILD.store(0, Ordering::Relaxed);
        return Build::Unknown;
    }
    build
}

/// The link timestamp of the running build (for the log).
pub unsafe fn stamp(base: usize) -> u32 {
    let rd32 = |at: usize| std::ptr::read_unaligned(at as *const u32);
    rd32(base + rd32(base + 0x3C) as usize + 8)
}

fn steam(epic: usize) -> usize {
    match STEAM.binary_search_by_key(&(epic as u32), |p| p.0) {
        Ok(i) => STEAM[i].1 as usize,
        Err(_) => {
            // Every `Rva` in the source is in the table (a test checks it): this is a bug.
            super::log(&format!("no Steam address for Epic RVA {epic:#x}"));
            epic
        }
    }
}

/// Epic RVA -> Steam RVA, sorted by the Epic RVA.
pub(crate) static STEAM: &[(u32, u32)] = &include!("steam_map.in");

/// `mov rax, [rip + disp]` at `at` reading `target`, as the running build has it (the first
/// instruction of a few functions we replace outright; its offset differs between builds).
pub fn mov_rax_rip(at: Rva, target: Rva) -> [u8; 7] {
    let disp = (target.get() as i64 - (at.get() as i64 + 7)) as i32;
    let d = disp.to_le_bytes();
    [0x48, 0x8B, 0x05, d[0], d[1], d[2], d[3]]
}
