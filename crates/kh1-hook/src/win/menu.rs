//! Phase 7: KH2's abilities from KH1's menu (`crate::growth`), by the ini's `abilities`:
//! `test` (the default for now: everything at MAX, as before), `kh1` (KH1's menu and Sora's
//! level), `all` (everything, every combo modifier too). `kh1-kh2.ini` sits next to the game
//! and is written with the default the first time.

use super::log;
use crate::growth::{self, Kh1Sora, Mode};
use character_core::Character;

const SAVE_PTR_RVA: usize = 0x28695A0;
const SAVE_RVA: usize = 0x2DE9D60;
const INI: &str = "kh1-kh2.ini";
const DEFAULT_INI: &str = "; kh1-kh2 settings (read when the game starts)\n\
; abilities: test = every KH2 movement ability at MAX (testing), kh1 = KH2's abilities from\n\
; KH1's Ability menu and Sora's level, all = everything and every combo modifier\n\
abilities = test\n";

pub struct Growth {
    mode: Mode,
    frames: u32,
    seen: Option<Kh1Sora>,
    applied: bool,
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

/// The ini's `key = value`, if set.
fn setting(text: &str, key: &str) -> Option<String> {
    text.lines().filter_map(|l| l.split(';').next()).filter_map(|l| l.split_once('=')).find(|(k, _)| k.trim().eq_ignore_ascii_case(key)).map(|(_, v)| v.trim().to_string())
}

impl Growth {
    pub fn new() -> Growth {
        let path = format!("{}\\{INI}", super::exe_dir());
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => {
                let _ = std::fs::write(&path, DEFAULT_INI);
                DEFAULT_INI.to_string()
            }
        };
        let mode = setting(&text, "abilities").and_then(|v| Mode::parse(&v)).unwrap_or(Mode::Test);
        log(&format!("abilities: {mode:?} ({path})"));
        Growth { mode, frames: 0, seen: None, applied: false }
    }

    /// Sora's side of KH1's save block, if it is there.
    unsafe fn kh1(base: usize) -> Option<Kh1Sora> {
        let mut save = rd::<u64>(base + SAVE_PTR_RVA) as usize;
        if save < 0x10000 {
            save = base + SAVE_RVA;
        }
        if !super::crash::readable(save) || !super::crash::readable(save + 0x5A3) {
            return None;
        }
        let level = rd::<u8>(save + 4);
        let abilities = std::slice::from_raw_parts((save + 0x44) as *const u8, 48).to_vec();
        let shared = std::slice::from_raw_parts((save + 0x599) as *const u8, 10).to_vec();
        Some(Kh1Sora { level, abilities, shared })
    }

    /// Every second: KH1's menu and level, and Sora's abilities from them (`kh1`), or the
    /// mode's fixed set once.
    pub unsafe fn update(&mut self, base: usize, ch: &mut Character) {
        self.frames = self.frames.wrapping_add(1);
        if !self.applied {
            self.applied = true;
            match self.mode {
                Mode::Test => ch.abilities = growth::test(),
                Mode::All => ch.abilities = growth::all(),
                Mode::Kh1 => self.frames = 59,
            }
        }
        if self.frames % 60 != 0 {
            return;
        }
        let Some(now) = Self::kh1(base) else { return };
        if self.seen.as_ref() == Some(&now) {
            return;
        }
        let from_kh1 = now.kh2();
        let list = |l: &[u8]| l.iter().take_while(|b| **b != 0).map(|b| if b & 0x80 != 0 { format!("({})", b & 0x7F) } else { b.to_string() }).collect::<Vec<_>>().join(" ");
        log(&format!("KH1 Sora: level {}, abilities [{}], shared [{}] (in brackets: not equipped)", now.level, list(&now.abilities), list(&now.shared)));
        log(&format!("KH1's menu gives KH2: {}", growth::describe(&from_kh1)));
        if self.mode == Mode::Kh1 {
            ch.abilities = from_kh1;
        }
        self.seen = Some(now);
    }
}
