//! Phase 7: KH2's abilities from KH1's menu (`crate::growth`), by the ini's `abilities`
//! (`settings`): `test` (the default for now: everything at MAX, as before), `kh1` (KH1's menu
//! and Sora's level), `all` (everything, every combo modifier too).
//!
//! KH2's own abilities go into KH1's menu (`growth::MENU`). KH1's ability table (`btltbl.bin`'s,
//! loaded by `0x14028D840`; its address at `0x142D37748`, read by `0x14028D760(id)` =
//! table + (id - 1) * 12) has 65 entries of 12 bytes: +0 AP, +1 kind, +2 menu order (u16),
//! +4 the name's handle, +8 the help text's handle (the remaster's `AbilityName.bin` /
//! `AbilityHelp.bin` strings, made handles with `0x14038B080(ptr)`). Final Mix itself added
//! Combo Master (65) by writing its entry there. Here the table is copied into a longer one
//! with entries 66 up made from a KH1 template entry, KH2's AP, and our own text (KH1's text
//! codes: 0 ends, 1 space, 0x21.. digits, 0x2B.. A-Z, 0x45.. a-z), and `0x142D37748` points at
//! the copy (again whenever KH1 reloads the table). Sora learns them at their level: added to
//! his lists, Quick Run and Aerial Dodge on (shared abilities), the rest off until equipped.
//! In the other modes they are taken back out of his lists, so a save stays KH1's own.

use super::log;
use crate::growth::{self, Kh1Sora, Mode};
use character_core::Character;

const SAVE_PTR_RVA: usize = 0x28695A0;
const SAVE_RVA: usize = 0x2DE9D60;
const TABLE_PTR_RVA: usize = 0x2D37748;
const TO_HANDLE_RVA: usize = 0x38B080;
const TABLE_LOAD_RVA: usize = 0x28D840;
const TABLE_LOAD_PROLOGUE: [u8; 9] = [0x40, 0x53, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x20];
/// KH1's own entries.
const KH1_ENTRIES: usize = 65;
const ENTRIES: usize = 80;

static TABLE: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
/// Our abilities' text handles (name, help), by `MENU` order.
static TEXT: std::sync::OnceLock<Vec<(u32, u32)>> = std::sync::OnceLock::new();
static ORIG_LOAD: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// KH1's text codes for a plain string (letters, digits and some punctuation; others become a
/// space).
pub fn encode(text: &str) -> Vec<u8> {
    let mut out: Vec<u8> = text
        .chars()
        .map(|c| match c {
            'A'..='Z' => 0x2B + (c as u8 - b'A'),
            'a'..='z' => 0x45 + (c as u8 - b'a'),
            '0'..='9' => 0x21 + (c as u8 - b'0'),
            '-' => 0x6E,
            '.' => 0x68,
            ',' => 0x69,
            '\'' => 0x71,
            '!' => 0x5F,
            '?' => 0x60,
            '(' => 0x74,
            ')' => 0x75,
            '/' => 0x66,
            ':' => 0x6B,
            '+' => 0x63,
            '&' => 0x61,
            '\n' => 0x02,
            _ => 0x01,
        })
        .collect();
    out.push(0);
    out
}

/// The other way, for the log (codes it doesn't know as `{xx}`).
fn decode(p: usize) -> String {
    let mut s = String::new();
    for i in 0..64 {
        let b = unsafe { rd::<u8>(p + i) };
        match b {
            0 => break,
            0x01 => s.push(' '),
            0x21..=0x2A => s.push((b'0' + b - 0x21) as char),
            0x2B..=0x44 => s.push((b'A' + b - 0x2B) as char),
            0x45..=0x5E => s.push((b'a' + b - 0x45) as char),
            _ => s.push_str(&format!("{{{b:02x}}}")),
        }
    }
    s
}

/// Points KH1 at our longer ability table (if it isn't already).
pub unsafe fn ensure_table(base: usize) {
    let cur = rd::<u64>(base + TABLE_PTR_RVA) as usize;
    let ours = *TABLE.get_or_init(|| Box::leak(vec![0u8; ENTRIES * 12].into_boxed_slice()).as_mut_ptr() as usize);
    if cur == ours || cur < 0x10000 || !super::crash::readable(cur) || !super::crash::readable(cur + KH1_ENTRIES * 12 - 1) {
        return;
    }
    let to_handle: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(base + TO_HANDLE_RVA);
    let text = TEXT.get_or_init(|| {
        growth::MENU
            .iter()
            .map(|ab| {
                let name = Box::leak(encode(ab.name).into_boxed_slice()).as_ptr() as usize;
                let help = Box::leak(encode(ab.help).into_boxed_slice()).as_ptr() as usize;
                (to_handle(name), to_handle(help))
            })
            .collect()
    });
    std::ptr::copy_nonoverlapping(cur as *const u8, ours as *mut u8, KH1_ENTRIES * 12);
    for (k, ab) in growth::MENU.iter().enumerate() {
        let e = ours + (ab.id as usize - 1) * 12;
        let t = cur + (ab.template as usize - 1) * 12;
        std::ptr::copy_nonoverlapping(t as *const u8, e as *mut u8, 12);
        std::ptr::write(e as *mut u8, ab.ap);
        std::ptr::write_unaligned((e + 2) as *mut u16, rd::<u16>(t + 2).wrapping_add(1));
        std::ptr::write_unaligned((e + 4) as *mut u32, text[k].0);
        std::ptr::write_unaligned((e + 8) as *mut u32, text[k].1);
    }
    std::ptr::write_unaligned((base + TABLE_PTR_RVA) as *mut u64, ours as u64);
    static LOGGED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if LOGGED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 6 {
        let sample = |id: usize| {
            let e = ours + (id - 1) * 12;
            format!("{id}: AP {} kind {} order {} \"{}\" / \"{}\"", rd::<u8>(e), rd::<u8>(e + 1), rd::<u16>(e + 2), decode(super::pose::ptr(rd::<u32>(e + 4))), decode(super::pose::ptr(rd::<u32>(e + 8))))
        };
        log(&format!("KH1's ability table {cur:#x} -> ours {ours:#x} ({} KH2 abilities added); {} | {} | {} | {}", growth::MENU.len(), sample(6), sample(3), sample(65), sample(68)));
    }
}

unsafe extern "system" fn table_load() {
    let f: unsafe extern "system" fn() = std::mem::transmute(ORIG_LOAD.load(std::sync::atomic::Ordering::Acquire));
    f();
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    ensure_table(base);
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    super::hook::install(base + TABLE_LOAD_RVA, &TABLE_LOAD_PROLOGUE, table_load as *const () as usize, &ORIG_LOAD, "ability table load");
}

/// Sora's save block, if it is there.
unsafe fn save_block(base: usize) -> Option<usize> {
    let mut save = rd::<u64>(base + SAVE_PTR_RVA) as usize;
    if save < 0x10000 {
        save = base + SAVE_RVA;
    }
    (super::crash::readable(save) && super::crash::readable(save + 0x5A3)).then_some(save)
}

/// Adds `id` (with `bits`) at the first free place of a list of `len`; false if it is full.
unsafe fn add(list: usize, len: usize, byte: u8) -> bool {
    for i in 0..len {
        if rd::<u8>(list + i) == 0 {
            std::ptr::write((list + i) as *mut u8, byte);
            return true;
        }
    }
    false
}

/// Notes a KH1 ability given up for one of ours in `kh1-kh2-removed.txt` next to the game (so
/// it can be given back by hand).
fn remember(line: &str) {
    use std::io::Write;
    let path = format!("{}\\kh1-kh2-removed.txt", super::exe_dir());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}

/// Takes our abilities back out of a list (closing the gaps).
unsafe fn strip(list: usize, len: usize) -> usize {
    let bytes: Vec<u8> = (0..len).map(|i| rd::<u8>(list + i)).collect();
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(len);
    let ours = |b: u8| growth::MENU.iter().any(|ab| ab.id == b & 0x7F);
    let kept: Vec<u8> = bytes[..end].iter().copied().filter(|b| !ours(*b)).collect();
    let removed = end - kept.len();
    if removed > 0 {
        for i in 0..end {
            std::ptr::write((list + i) as *mut u8, kept.get(i).copied().unwrap_or(0));
        }
    }
    removed
}

pub struct Growth {
    mode: Mode,
    frames: u32,
    seen: Option<Kh1Sora>,
    applied: bool,
    /// Abilities already reported as not fitting (once each).
    no_room: Vec<u8>,
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

impl Growth {
    pub fn new() -> Growth {
        Growth { mode: super::settings::get().abilities, frames: 0, seen: None, applied: false, no_room: Vec::new() }
    }

    /// Sora's side of KH1's save block, if it is there.
    unsafe fn kh1(base: usize) -> Option<Kh1Sora> {
        let save = save_block(base)?;
        let level = rd::<u8>(save + 4);
        let abilities = std::slice::from_raw_parts((save + 0x44) as *const u8, 48).to_vec();
        let shared = std::slice::from_raw_parts((save + 0x599) as *const u8, 10).to_vec();
        Some(Kh1Sora { level, abilities, shared })
    }

    /// Every second: KH1's menu and level, and Sora's abilities from them (`kh1`), or the
    /// mode's fixed set once.
    pub unsafe fn update(&mut self, base: usize, ch: &mut Character) {
        self.frames = self.frames.wrapping_add(1);
        ensure_table(base);
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
        let Some(mut now) = Self::kh1(base) else { return };
        // Sora learns KH2's abilities at their levels (`kh1`), or has them taken back out.
        if let Some(save) = save_block(base) {
            if self.mode == Mode::Kh1 {
                for ab in now.grants() {
                    let (list, len, byte) = match ab.list {
                        growth::List::Shared => (save + 0x599, 10, ab.id),
                        growth::List::Sora => (save + 0x44, 48, ab.id | 0x80),
                    };
                    let mut done = add(list, len, byte);
                    // KH2's abilities first (the user): a KH1 ability that's off makes room.
                    if !done && ab.list == growth::List::Sora {
                        let bytes: Vec<u8> = (0..len).map(|i| rd::<u8>(list + i)).collect();
                        if let Some(i) = growth::make_room(&bytes) {
                            let gone = bytes[i];
                            let mut kept = bytes.clone();
                            kept.remove(i);
                            kept.push(0);
                            for (k, b) in kept.iter().enumerate() {
                                std::ptr::write((list + k) as *mut u8, *b);
                            }
                            done = add(list, len, byte);
                            let line = format!("Sora's list was full: {} (off) made room for {}", growth::name(gone), ab.name);
                            log(&line);
                            remember(&format!("level {}: removed {} (id {}) for {}", now.level, growth::name(gone), gone & 0x7F, ab.name));
                        }
                    }
                    if done {
                        log(&format!("Sora learns {} (level {}, {} AP)", ab.name, ab.level, ab.ap));
                    } else if !self.no_room.contains(&ab.id) {
                        self.no_room.push(ab.id);
                        log(&format!("Sora can't learn {} (level {}): his ability list is full (48, KH1's most)", ab.name, ab.level));
                    }
                }
            } else {
                let n = strip(save + 0x44, 48) + strip(save + 0x599, 10);
                if n > 0 {
                    log(&format!("took {n} KH2 abilities back out of Sora's lists (abilities = {:?})", self.mode));
                }
            }
            if let Some(again) = Self::kh1(base) {
                now = again;
            }
        }
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
