//! Phase 7: KH2's abilities from KH1's menu (`crate::growth`), by the ini's `abilities`
//! (`settings`): `test` (the default for now: everything at MAX, as before), `kh1` (KH1's menu
//! and Sora's level), `all` (everything, every combo modifier too).

use super::log;
use crate::growth::{self, Kh1Sora, Mode};
use character_core::Character;

const SAVE_PTR_RVA: usize = 0x28695A0;
const SAVE_RVA: usize = 0x2DE9D60;
pub struct Growth {
    mode: Mode,
    frames: u32,
    seen: Option<Kh1Sora>,
    applied: bool,
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

impl Growth {
    pub fn new() -> Growth {
        Growth { mode: super::settings::get().abilities, frames: 0, seen: None, applied: false }
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
