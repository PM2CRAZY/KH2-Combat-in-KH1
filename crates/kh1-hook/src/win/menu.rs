//! Phase 7: KH2's abilities from KH1's menu (`crate::growth`), by the ini's `abilities`
//! (`settings`): `test` (the default for now: everything at MAX, as before), `kh1` (KH1's menu
//! and Sora's level), `all` (everything, every combo modifier too).
//!
//! KH2's own abilities go into KH1's menu (`growth::MENU`, `docs/abilities.md`). KH1's ability
//! table (`btltbl.bin`'s, loaded by `0x14028D840`; its address at `0x142D37748`, read by
//! `0x14028D760(id)` = table + ((id & 0x7F) - 1) * 12, no bound check) has 65 entries of 12
//! bytes: +0 AP, +1 kind, +2 menu order (u16), +4 the name's handle, +8 the help text's handle
//! (the remaster's `AbilityName.bin` / `AbilityHelp.bin` strings, made handles with
//! `0x14038B080(ptr)`). Final Mix itself added Combo Master (65) by writing its entry there.
//! Here the table is copied into a longer one (every id up to `growth::MAX_ID`) with our
//! entries made from a KH1 template entry, KH2's AP, and our own text (KH1's text codes: 0 ends,
//! 1 space, 0x21.. digits, 0x2B.. A-Z, 0x45.. a-z), and `0x142D37748` points at the copy (again
//! whenever KH1 reloads the table). KH1's own 65 keep KH1's entries untouched (the split,
//! testing: KH1's names, help and AP). Sora learns ours by level, boss or KH1 namesake: added
//! to his list, off until equipped. In the other modes they are taken back out of his lists,
//! so a save stays KH1's own.
//!
//! KH1 counts an ability (`0x140282A70 (id, member)`) by scanning the list for the id, and its
//! AP sums walk the 48 bytes (`0x1402E6220`, `0x1402E62D0`: `cmp 0x30`), so ours live in Sora's
//! own 48-byte list; nothing in KH1 indexes an array by ability id.

use super::addr::Rva;
use super::log;
use crate::growth::{self, Kh1Sora, Mode};
use character_core::Character;

const SAVE_PTR_RVA: Rva = Rva(0x28695A0);
const SAVE_RVA: Rva = Rva(0x2DE9D60);
const TABLE_PTR_RVA: Rva = Rva(0x2D37748);
const TO_HANDLE_RVA: Rva = Rva(0x38B080);
const TABLE_LOAD_RVA: Rva = Rva(0x28D840);
const TABLE_LOAD_PROLOGUE: [u8; 9] = [0x40, 0x53, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x20];
/// KH1's own entries.
const KH1_ENTRIES: usize = 65;
/// The worlds' story progress bytes in the save block (`0x142DEB264`, 12 bytes).
const PROGRESS_OFF: usize = 0x1504;
/// Our Aerial Dodge's id (`growth::MENU`).
const AERIAL_DODGE: u8 = growth::ours::AERIAL_DODGE;
/// Our table's entries: every id a list byte can hold.
const ENTRIES: usize = growth::MAX_ID as usize;

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

/// Quick Run's menu name buffer (KH1 text codes) and its size.
static QR_NAME: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
const QR_NAME_LEN: usize = 32;

/// Our Aerial Dodge's (like Quick Run's). (KH1's own High Jump, Glide, Superglide and Dodge
/// Roll keep KH1's names since the split; their KH2 growth shows in the "Ability Up" boxes.)
static AD_NAME: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn write_name(slot: &std::sync::atomic::AtomicUsize, name: String) {
    let p = slot.load(std::sync::atomic::Ordering::Relaxed);
    if p == 0 {
        return;
    }
    let mut b = encode(&name);
    b.truncate(QR_NAME_LEN - 1);
    b.push(0);
    unsafe { std::ptr::copy_nonoverlapping(b.as_ptr(), p as *mut u8, b.len()) };
}

/// Writes Aerial Dodge's level into its menu name.
pub fn set_aerial_dodge_name(level: u8) {
    write_name(&AD_NAME, growth::aerial_dodge_name(level));
}

/// KH2's own Glide's (t323, `growth::ours::GLIDE`): "Glide LV2", "Glide MAX".
static GL_NAME: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn set_glide_name(level: u8) {
    write_name(&GL_NAME, growth::glide_name(level));
}

/// The shared list for the Glide style now (t323, `growth::glide_swap`): KH2's own Glide in place
/// of KH1's Glide and Superglide at KH2, KH1's back at KH1 (`kh2`). Written back if it changed.
unsafe fn glide_style_list(save: usize, now: &Kh1Sora, kh2: bool) -> bool {
    let (list, lines) = growth::glide_swap(&now.shared, kh2, &now.progress);
    if list == now.shared {
        return false;
    }
    for (k, b) in list.iter().enumerate().take(10) {
        std::ptr::write((save + 0x599 + k) as *mut u8, *b);
    }
    for line in &lines {
        log(&format!("abilities: {line}"));
    }
    true
}

/// The Glide style switched in the Config menu (`config::set`, KH1's menu update, the game's
/// thread): the shared list swapped there and then, not at the next second of Sora's physics
/// frames (`Growth::update` runs only from them, so while the pause menu is up KH1's Abilities
/// screen still showed KH1's Glide and Superglide and no KH2 Glide: the tester, glide thread m6).
/// KH2's Glide's menu name gets its level at once too. Only with `abilities = kh1` (the other
/// modes keep KH1's lists, `Growth::update`).
pub unsafe fn glide_style_now(base: usize) {
    if base == 0 || super::settings::get().abilities != Mode::Kh1 {
        return;
    }
    let Some(save) = save_block(base) else { return };
    let Some(now) = Growth::kh1(base) else { return };
    let kh2 = super::styles::kh2(super::styles::Kind::Glide);
    let changed = glide_style_list(save, &now, kh2);
    if let Some(after) = Growth::kh1(base) {
        if kh2 && growth::has(&after.shared, growth::ours::GLIDE) {
            set_glide_name(growth::kh2_glide_level(after.level, &after.progress));
        }
        log(&format!(
            "abilities: Glide style {} set in the Config menu: shared list {} now ({:?})",
            if kh2 { "KH2" } else { "KH1" },
            if changed { "swapped" } else { "already right" },
            after.shared.iter().take_while(|b| **b != 0).collect::<Vec<_>>()
        ));
    }
}

/// Writes Quick Run's level into its menu name ("Quick Run LV2", "Quick Run MAX").
pub fn set_quick_run_name(level: u8) {
    let p = QR_NAME.load(std::sync::atomic::Ordering::Relaxed);
    if p == 0 {
        return;
    }
    let mut b = encode(&growth::quick_run_name(level));
    b.truncate(QR_NAME_LEN - 1);
    b.push(0);
    unsafe { std::ptr::copy_nonoverlapping(b.as_ptr(), p as *mut u8, b.len()) };
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
            .map(|ab| (ab.name, ab.help))
            .map(|(n, h)| {
                let name = if n == "Quick Run" {
                    // Its name shows its level, rewritten in place as it grows (the user: the
                    // menu should say which level, without duplicate copies).
                    let buf = Box::leak(vec![0u8; QR_NAME_LEN].into_boxed_slice()).as_mut_ptr() as usize;
                    QR_NAME.store(buf, std::sync::atomic::Ordering::Relaxed);
                    set_quick_run_name(1);
                    buf
                } else if n == "Aerial Dodge" {
                    let buf = Box::leak(vec![0u8; QR_NAME_LEN].into_boxed_slice()).as_mut_ptr() as usize;
                    AD_NAME.store(buf, std::sync::atomic::Ordering::Relaxed);
                    set_aerial_dodge_name(1);
                    buf
                } else if n == "Glide" {
                    let buf = Box::leak(vec![0u8; QR_NAME_LEN].into_boxed_slice()).as_mut_ptr() as usize;
                    GL_NAME.store(buf, std::sync::atomic::Ordering::Relaxed);
                    set_glide_name(1);
                    buf
                } else {
                    Box::leak(encode(n).into_boxed_slice()).as_ptr() as usize
                };
                let help = Box::leak(encode(h).into_boxed_slice()).as_ptr() as usize;
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
    // (KH1's own entries stay as KH1 has them: since the split no KH1 id is renamed.)
    std::ptr::write_unaligned((base + TABLE_PTR_RVA) as *mut u64, ours as u64);
    static LOGGED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if LOGGED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 6 {
        let sample = |id: usize| {
            let e = ours + (id - 1) * 12;
            format!("{id}: AP {} kind {} order {} \"{}\" / \"{}\"", rd::<u8>(e), rd::<u8>(e + 1), rd::<u16>(e + 2), decode(super::pose::ptr(rd::<u32>(e + 4))), decode(super::pose::ptr(rd::<u32>(e + 8))))
        };
        log(&format!(
            "KH1's ability table {cur:#x} -> ours {ours:#x} ({} KH2 abilities added, ids {}..={}); {} | {} | {} | {} | {}",
            growth::MENU.len(),
            growth::FIRST_ID,
            growth::MENU.iter().map(|a| a.id).max().unwrap_or(0),
            sample(6),
            sample(growth::kh1::HURRICANE_BLAST as usize),
            sample(65),
            sample(68),
            sample(growth::ours::EXPLOSION as usize)
        ));
    }
}

unsafe extern "system" fn table_load() {
    let f: unsafe extern "system" fn() = std::mem::transmute(ORIG_LOAD.load(std::sync::atomic::Ordering::Acquire));
    f();
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    ensure_table(base);
}

/// KH1's party stats (`0x1402A2FA0` `(member, stats)`, into `0x142E94BC0 + member * 0x100`):
/// max AP `+0xE4` = the save's AP byte (member `+9`) plus the accessories', at most 100.
const STATS_RVA: Rva = Rva(0x2A2FA0);
const STATS_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x6C, 0x24, 0x18, 0x57, 0x41, 0x56, 0x41, 0x57];
static ORIG_STATS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Sora's max AP gets the AP of each KH2 ability he has learned (`growth::bonus_ap`); the save
/// keeps KH1's own.
unsafe extern "system" fn stats(member: i32, out: usize) {
    stats_inner(member, out);
    // The debug menu's locked stats over everything (`debugmenu::lock_stats`).
    super::debugmenu::lock_stats(member, out);
}

unsafe fn stats_inner(member: i32, out: usize) {
    let f: unsafe extern "system" fn(i32, usize) = std::mem::transmute(ORIG_STATS.load(std::sync::atomic::Ordering::Acquire));
    f(member, out);
    // Valor's second Keyblade's Strength and MP on Sora's (`valorstats`).
    super::valorstats::add(member, out);
    let set = super::settings::get();
    if member != 0 || out < 0x10000 || !set.bonus_ap || set.abilities != growth::Mode::Kh1 {
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let Some(sora) = Growth::kh1(base) else { return };
    let bonus = growth::bonus_ap(&sora);
    let ap = rd::<i32>(out + 0xE4);
    let now = (ap + bonus as i32).min(100);
    std::ptr::write_unaligned((out + 0xE4) as *mut i32, now);
    static LAST: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);
    if LAST.swap(now, std::sync::atomic::Ordering::Relaxed) != now {
        log(&format!("Sora's AP: {ap} (KH1's) + {bonus} (KH2 abilities learned) = {now}"));
    }
}

/// KH1's event command that adds a reward's line to a message window (`0x1401B6990(event)`:
/// the reward id at the event stack's top `+0x198 + sp * 4`, its record `0x14028D810(id)`, a
/// u16: low 4 bits the kind, 1 an ability, its id the high byte, the party member bits 4..7;
/// it pops 2 and returns 2). Abilities are added by the command `0x1401AF120` from the same
/// record (`0x140282CA0(id, member)`).
const REWARD_LINE_RVA: Rva = Rva(0x1B6990);
const REWARD_LINE_PROLOGUE: [u8; 6] = [0x40, 0x57, 0x48, 0x83, 0xEC, 0x20];
const REWARD_RECORD_RVA: Rva = Rva(0x28D810);
static ORIG_REWARD_LINE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// No "learned" line for Zantetsuken (it's taken out again at once, the user): the command is
/// popped as KH1 does when it shows nothing. (Before the split the mod gave some KH1 abilities
/// early and hid KH1's own later line for them; KH1's abilities come at KH1's own points now.)
unsafe extern "system" fn reward_line(ev: usize) -> u64 {
    let f: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(ORIG_REWARD_LINE.load(std::sync::atomic::Ordering::Acquire));
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    if super::settings::get().abilities == growth::Mode::Kh1 && ev > 0x10000 {
        let sp = rd::<i32>(ev + 0x194);
        if (0..64).contains(&sp) {
            let id = rd::<u32>(ev + 0x198 + sp as usize * 4);
            let rec: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base + REWARD_RECORD_RVA);
            let r = rec(id);
            if r > 0x10000 {
                let v = rd::<u16>(r);
                let (kind, member, ability) = (v & 0xF, (v >> 4) & 0xF, (v >> 8) as u8);
                if kind == 1 && member == 0 && ability == growth::kh1::ZANTETSUKEN {
                    std::ptr::write_unaligned((ev + 0x194) as *mut i32, sp - 2);
                    log(&format!("no \"learned\" line for {}: the mod takes it out", growth::name(ability)));
                    return 2;
                }
            }
        }
    }
    f(ev)
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    super::hook::install(base + TABLE_LOAD_RVA, &TABLE_LOAD_PROLOGUE, table_load as *const () as usize, &ORIG_LOAD, "ability table load");
    super::hook::install(base + REWARD_LINE_RVA, &REWARD_LINE_PROLOGUE, reward_line as *const () as usize, &ORIG_REWARD_LINE, "reward message line");
    super::hook::install(base + STATS_RVA, &STATS_PROLOGUE, stats as *const () as usize, &ORIG_STATS, "party stats");
}

/// KH1's stats function for `member` into `out` as KH1 alone fills it (unhooked: no Valor
/// Keyblade, no KH2 AP), for `valorstats`; false if it can't be run.
pub unsafe fn kh1_stats(member: i32, out: usize) -> bool {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    if out < 0x10000 || save_block(base).is_none() {
        return false;
    }
    let orig = ORIG_STATS.load(std::sync::atomic::Ordering::Acquire);
    let at = if orig != 0 { orig } else { base + STATS_RVA };
    let f: unsafe extern "system" fn(i32, usize) = std::mem::transmute(at);
    f(member, out);
    true
}

/// KH1's level-up prompt boxes (`0x14283BD90`, Sora's first, `0x3A20` apart; the box count
/// per party member at 16 bytes before) and their text (`0x142DC3068`: titles `0x20` apart,
/// lines from `+0x70`); the layout as KH1's own box updater (`0x140270DA0`) reads it. Shows one
/// box in Sora's place; false while that box is in use (a level-up's own, say).
const PROMPT_BOX_RVA: Rva = Rva(0x283BD90);
#[allow(dead_code)]
const PROMPT_TEXT_RVA: Rva = Rva(0x2DC3068);
/// Boxes per party member (`0x3A20` / `0xBA0`) and their size.
const PROMPT_BOXES: usize = 5;
const PROMPT_BOX_SIZE: usize = 0xBA0;
const PROMPT_BOX_COLOUR_RVA: Rva = Rva(0x528710);
const PROMPT_TEXT_COLOUR_RVA: Rva = Rva(0x528750);

unsafe fn prompt(base: usize, title: &str, line1: &str, line2: &str, append: bool) -> bool {
    let first = base + PROMPT_BOX_RVA;
    let count_at = first - 0x10;
    if !super::crash::readable(count_at) || !super::crash::readable(first + PROMPT_BOX_SIZE * PROMPT_BOXES) {
        return false;
    }
    let showing = rd::<i32>(first) != 0;
    let n = rd::<i32>(count_at);
    // Beside KH1's own (a level-up's) in Sora's column (the user: show both at once), or alone.
    let index = match (showing, append) {
        (true, true) if (1..PROMPT_BOXES as i32).contains(&n) => n as usize,
        (false, false) => 0,
        _ => return false,
    };
    let bx = first + index * PROMPT_BOX_SIZE;
    // Our own text (KH1's prompt text area holds the level-up's).
    let keep = |s: &str| -> usize {
        let mut b = encode(s);
        b.truncate(0x1F);
        b.push(0);
        Box::leak(b.into_boxed_slice()).as_ptr() as usize
    };
    let w = |at: usize, v: u64| std::ptr::write_unaligned(at as *mut u64, v);
    let wi = |at: usize, v: i32| std::ptr::write_unaligned(at as *mut i32, v);
    w(bx + 0x30, keep(title) as u64);
    wi(bx + 0x18, if line2.is_empty() { 1 } else { 2 });
    w(bx + 0x20, keep(line1) as u64);
    if !line2.is_empty() {
        w(bx + 0x28, keep(line2) as u64);
    }
    wi(bx + 0x0C, 0);
    std::ptr::write_unaligned((bx + 0xB80) as *mut f32, 1.0);
    w(bx + 0xB88, (base + PROMPT_BOX_COLOUR_RVA) as u64);
    w(bx + 0xB90, (base + PROMPT_TEXT_COLOUR_RVA) as u64);
    wi(bx, 1);
    wi(count_at, index as i32 + 1);
    log(&format!("prompt shown: {title} / {line1} / {line2} (box {index}, KH1's count was {n})"));
    true
}

/// Sora's save block, if it is there.
pub unsafe fn save_block(base: usize) -> Option<usize> {
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

/// KH1 abilities `kh1-kh2-removed.txt` says were given up that may no longer be (the user
/// keeps Encounter Plus): their ids.
fn given_back() -> Vec<u8> {
    let path = format!("{}\\kh1-kh2-removed.txt", super::exe_dir());
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    let mut ids = Vec::new();
    for line in text.lines() {
        let Some(i) = line.find("(id ") else { continue };
        let Some(id) = line[i + 4..].split(')').next().and_then(|n| n.trim().parse::<u8>().ok()) else { continue };
        if !growth::MAKE_ROOM.contains(&id) && id != growth::kh1::ZANTETSUKEN && !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

/// Takes our abilities back out of a list (closing the gaps).
unsafe fn strip(list: usize, len: usize) -> usize {
    let bytes: Vec<u8> = (0..len).map(|i| rd::<u8>(list + i)).collect();
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(len);
    // (Not one the debug menu gave this session.)
    let ours = |b: u8| growth::MENU.iter().any(|ab| ab.id == b & 0x7F) && super::debugmenu::ability_forced(b) != Some(true);
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
    /// Abilities given up before that no longer would be were given back (once a run).
    gave_back: bool,
    /// "New ability" boxes waiting for KH1's prompt box to be free: (title, line, line).
    prompts: std::collections::VecDeque<(String, String, String)>,
    /// Frames KH1's prompt box has been free.
    idle: u32,
    /// Quick Run's and Dodge Roll's levels last seen (None: not looked yet).
    qr_level: Option<u8>,
    dr_level: Option<u8>,
    hj_level: Option<u8>,
    ad_level: Option<u8>,
    gl_level: Option<u8>,
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

impl Growth {
    pub fn new() -> Growth {
        Growth { mode: super::settings::get().abilities, frames: 0, seen: None, applied: false, no_room: Vec::new(), gave_back: false, prompts: Default::default(), idle: 0, qr_level: None, dr_level: None, hj_level: None, ad_level: None, gl_level: None }
    }

    /// Sora's side of KH1's save block, if it is there.
    unsafe fn kh1(base: usize) -> Option<Kh1Sora> {
        let save = save_block(base)?;
        let level = rd::<u8>(save + 4);
        let abilities = std::slice::from_raw_parts((save + 0x44) as *const u8, 48).to_vec();
        let shared = std::slice::from_raw_parts((save + 0x599) as *const u8, 10).to_vec();
        let progress = std::slice::from_raw_parts((save + PROGRESS_OFF) as *const u8, 12).to_vec();
        Some(Kh1Sora { level, abilities, shared, progress })
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
            if self.mode != Mode::Kh1 {
                super::styles::note_abilities(&ch.abilities);
            }
        }
        // Beside the level-up's own box when it comes (the user: KH1 shows several at once): for
        // up to 3 s after learning, added to KH1's when it is up; then alone if the box is free.
        if !self.prompts.is_empty() {
            self.idle += 1;
            if let Some(p) = self.prompts.front().cloned() {
                if prompt(base, &p.0, &p.1, &p.2, true) || (self.idle >= 180 && prompt(base, &p.0, &p.1, &p.2, false)) {
                    self.prompts.pop_front();
                    self.idle = 0;
                }
            }
        } else {
            self.idle = 0;
        }
        if self.frames % 60 != 0 {
            return;
        }
        let Some(mut now) = Self::kh1(base) else { return };
        // Sora learns KH2's abilities at their levels (`kh1`), or has them taken back out.
        if let Some(save) = save_block(base) {
            if self.mode == Mode::Kh1 {
                // Given back once (the user: keep Encounter Plus), off, making room if needed.
                if !self.gave_back {
                    self.gave_back = true;
                    for id in given_back() {
                        if growth::has(&now.abilities, id) {
                            continue;
                        }
                        let list = save + 0x44;
                        let mut done = add(list, 48, id | 0x80);
                        if !done {
                            let bytes: Vec<u8> = (0..48).map(|i| rd::<u8>(list + i)).collect();
                            if let Some(i) = growth::make_room(&bytes) {
                                let gone = bytes[i];
                                let mut kept = bytes.clone();
                                kept.remove(i);
                                kept.push(0);
                                for (k, b) in kept.iter().enumerate() {
                                    std::ptr::write((list + k) as *mut u8, *b);
                                }
                                remember(&format!("level {}: removed {} (id {}) to give back {}", now.level, growth::name(gone), gone & 0x7F, growth::name(id)));
                                log(&format!("{} (off) made room to give back {}", growth::name(gone), growth::name(id)));
                                done = add(list, 48, id | 0x80);
                            }
                        }
                        log(&format!("{} given back to Sora{}", growth::name(id), if done { " (off)" } else { ": no room" }));
                    }
                    if let Some(again) = Self::kh1(base) {
                        now = again;
                    }
                }
                // The Glide style's glide (t323): KH2's own Glide alone at KH2, KH1's Glide and
                // Superglide (vanilla) at KH1.
                if glide_style_list(save, &now, super::styles::kh2(super::styles::Kind::Glide)) {
                    if let Some(again) = Self::kh1(base) {
                        now = again;
                    }
                }
                // One High Jump, one Glide (the user): spares go.
                let (kept, spare) = growth::one_only(&now.shared);
                if !spare.is_empty() {
                    for (k, b) in kept.iter().enumerate() {
                        std::ptr::write((save + 0x599 + k) as *mut u8, *b);
                    }
                    let names: Vec<String> = spare.iter().map(|id| growth::name(*id)).collect();
                    log(&format!("spare shared abilities taken out (one of each counts): {}", names.join(", ")));
                    remember(&format!("level {}: spare shared abilities taken out (one of each counts): {}", now.level, names.join(", ")));
                    now.shared = kept;
                }
                // (Superglide is KH1's own again since the split: KH2's Glide MAX while it is on.)
                // No Zantetsuken (the user): it goes from Sora's list wherever KH1 gives it, a slot
                // for KH2's abilities (a finished save's list was full).
                {
                    let list = save + 0x44;
                    let bytes: Vec<u8> = (0..48).map(|i| rd::<u8>(list + i)).collect();
                    // (Unless the debug menu gave it, `debugmenu::ability_forced`.)
                    if growth::has(&bytes, growth::kh1::ZANTETSUKEN) && super::debugmenu::ability_forced(growth::kh1::ZANTETSUKEN) != Some(true) {
                        let end = bytes.iter().position(|b| *b == 0).unwrap_or(48);
                        let mut kept: Vec<u8> = bytes[..end].iter().copied().filter(|b| b & 0x7F != growth::kh1::ZANTETSUKEN).collect();
                        let n = end - kept.len();
                        kept.resize(48, 0);
                        for (k, b) in kept.iter().enumerate() {
                            std::ptr::write((list + k) as *mut u8, *b);
                        }
                        log(&format!("Zantetsuken taken out of Sora's list ({n} cop{})", if n == 1 { "y" } else { "ies" }));
                        remember(&format!("level {}: Zantetsuken taken out (the mod has none)", now.level));
                        if let Some(again) = Self::kh1(base) {
                            now = again;
                        }
                    }
                }
                // The split (testing): a save from before it, where KH1's modifiers stood for
                // KH2's (Hurricane Blast was Explosion), is moved over once (`growth::migrate`),
                // before KH1's namesakes bring KH2's abilities (off) below. Marked in the save's
                // spare bytes (`growth::LAYOUT_OFF`) either way. (KH1's own later copies of its
                // modifiers are no longer taken out: they are KH1's abilities.)
                if rd::<u16>(save + growth::LAYOUT_OFF) != growth::LAYOUT_SPLIT {
                    if growth::old_layout(&now) {
                        let m = growth::migrate(&now);
                        for (k, b) in m.abilities.iter().enumerate().take(48) {
                            std::ptr::write((save + 0x44 + k) as *mut u8, *b);
                        }
                        for (k, b) in m.shared.iter().enumerate().take(10) {
                            std::ptr::write((save + 0x599 + k) as *mut u8, *b);
                        }
                        log(&format!("abilities: a save from before the split, moved over ({} changes)", m.lines.len() + m.removed.len()));
                        for line in &m.lines {
                            log(&format!("abilities: {line}"));
                            remember(&format!("level {}: split: {line}", now.level));
                        }
                        for line in &m.removed {
                            log(&format!("abilities: {line}"));
                            remember(&format!("level {}: {line}", now.level));
                        }
                    } else {
                        log("abilities: the save has the split ids (nothing from before it to move over)");
                    }
                    std::ptr::write_unaligned((save + growth::LAYOUT_OFF) as *mut u16, growth::LAYOUT_SPLIT);
                    if let Some(again) = Self::kh1(base) {
                        now = again;
                    }
                }
                // Quick Run and Aerial Dodge moved from the shared list into Sora's (the user: with
                // AP); one that was on stays on.
                for ab in growth::MENU.iter().filter(|a| a.list == growth::List::Sora) {
                    let shared = save + 0x599;
                    let bytes: Vec<u8> = (0..10).map(|i| rd::<u8>(shared + i)).collect();
                    let Some(i) = bytes.iter().take_while(|b| **b != 0).position(|b| b & 0x7F == ab.id) else { continue };
                    let on = bytes[i] & 0x80 == 0;
                    let mut kept = bytes.clone();
                    kept.remove(i);
                    kept.push(0);
                    for (k, b) in kept.iter().enumerate() {
                        std::ptr::write((shared + k) as *mut u8, *b);
                    }
                    let moved = growth::has(&now.abilities, ab.id) || add(save + 0x44, 48, if on { ab.id } else { ab.id | 0x80 });
                    log(&format!("{} moved from the shared abilities to Sora's ({}){}", ab.name, if on { "on" } else { "off" }, if moved { "" } else { ": his list is full, learned again when there's room" }));
                    if let Some(again) = Self::kh1(base) {
                        now = again;
                    }
                }
                // Aerial Dodge is Genie Jafar's (the user; Riku's before): taken back out until he
                // is beaten (given again then, with its box).
                if !growth::BOSS_GIFTS.iter().any(|g| g.ability == AERIAL_DODGE && growth::beaten(&now.progress, g)) && super::debugmenu::ability_forced(AERIAL_DODGE) != Some(true) {
                    let list = save + 0x44;
                    let bytes: Vec<u8> = (0..48).map(|i| rd::<u8>(list + i)).collect();
                    let end = bytes.iter().position(|b| *b == 0).unwrap_or(48);
                    if let Some(i) = bytes[..end].iter().position(|b| b & 0x7F == AERIAL_DODGE) {
                        let mut kept = bytes.clone();
                        kept.remove(i);
                        kept.push(0);
                        for (k, b) in kept.iter().enumerate() {
                            std::ptr::write((list + k) as *mut u8, *b);
                        }
                        log("Aerial Dodge taken back until Genie Jafar is beaten (Agrabah)");
                        if let Some(again) = Self::kh1(base) {
                            now = again;
                        }
                    }
                }
                // KH2's abilities by level, boss (the user's list) or KH1 namesake, off until
                // equipped, each with its box.
                for (why, ab) in now.grants() {
                    // (Not one the debug menu took out this session.)
                    if super::debugmenu::ability_forced(ab.id) == Some(false) {
                        continue;
                    }
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
                        log(&format!("{why}: Sora learns {} (id {}, {} AP)", ab.name, ab.id, ab.ap));
                        // The user: the game should say so.
                        // Its name only (the user: no description needed).
                        let shown = match ab.effect {
                            growth::Effect::QuickRun => growth::quick_run_name(growth::quick_run_level(now.level)),
                            growth::Effect::AerialDodge => growth::aerial_dodge_name(growth::aerial_dodge_level(now.level)),
                            _ => ab.name.to_string(),
                        };
                        self.prompts.push_back(("New Ability".to_string(), shown, String::new()));
                    } else if !self.no_room.contains(&ab.id) {
                        self.no_room.push(ab.id);
                        log(&format!("{why}, but Sora can't learn {}: his ability list is full (48, KH1's most)", ab.name));
                    }
                }
            } else {
                // (KH1's Glide and Superglide back first: KH2's Glide stood for them, t323.)
                glide_style_list(save, &now, false);
                let n = strip(save + 0x44, 48) + strip(save + 0x599, 10);
                if n > 0 {
                    log(&format!("took {n} KH2 abilities back out of Sora's lists (abilities = {:?})", self.mode));
                }
            }
            if let Some(again) = Self::kh1(base) {
                now = again;
            }
        }
        // Quick Run's level: its menu name, and a box each time it grows (the user).
        if self.mode == Mode::Kh1 {
            let l = if growth::has(&now.abilities, growth::QUICK_RUN_ID) { growth::quick_run_level(now.level).max(1) } else { 0 };
            if self.qr_level != Some(l) {
                if l > 0 {
                    set_quick_run_name(l);
                }
                if let Some(old) = self.qr_level {
                    if old > 0 && l > old {
                        log(&format!("Quick Run grows: {}", growth::quick_run_name(l)));
                        self.prompts.push_back(("Ability Up".to_string(), growth::quick_run_name(l), String::new()));
                    }
                }
                self.qr_level = Some(l);
            }
            // KH1's Dodge Roll, High Jump and Glide keep KH1's names (the split); the box says
            // how far KH2's own grows with them.
            let l = if growth::has(&now.abilities, growth::kh1::DODGE_ROLL) { growth::dodge_roll_level(now.level) } else { 0 };
            if self.dr_level != Some(l) {
                if let Some(old) = self.dr_level {
                    if old > 0 && l > old {
                        log(&format!("Dodge Roll grows: {}", growth::dodge_roll_name(l)));
                        self.prompts.push_back(("Ability Up".to_string(), growth::dodge_roll_name(l), String::new()));
                    }
                }
                self.dr_level = Some(l);
            }
            let l = if growth::has(&now.shared, growth::kh1::HIGH_JUMP) { growth::high_jump_level(now.level) } else { 0 };
            if self.hj_level != Some(l) {
                if let Some(old) = self.hj_level {
                    if old > 0 && l > old {
                        log(&format!("High Jump grows: {}", growth::high_jump_name(l)));
                        self.prompts.push_back(("Ability Up".to_string(), growth::high_jump_name(l), String::new()));
                    }
                }
                self.hj_level = Some(l);
            }
            // KH2's own Glide (t323, Glide style KH2): its menu name and a box as it grows, MAX
            // at Chernabog. At KH1 KH1's Glide and Superglide are vanilla: no boxes of ours.
            let l = if growth::has(&now.shared, growth::ours::GLIDE) { growth::kh2_glide_level(now.level, &now.progress) } else { 0 };
            if self.gl_level != Some(l) {
                if l > 0 {
                    set_glide_name(l);
                }
                if let Some(old) = self.gl_level {
                    if old > 0 && l > old {
                        log(&format!("Glide grows: {}", growth::glide_name(l)));
                        self.prompts.push_back(("Ability Up".to_string(), growth::glide_name(l), String::new()));
                    }
                }
                self.gl_level = Some(l);
            }
            let l = if growth::has(&now.abilities, AERIAL_DODGE) { growth::aerial_dodge_level(now.level) } else { 0 };
            if self.ad_level != Some(l) {
                if l > 0 {
                    set_aerial_dodge_name(l);
                }
                if let Some(old) = self.ad_level {
                    if old > 0 && l > old {
                        log(&format!("Aerial Dodge grows: {}", growth::aerial_dodge_name(l)));
                        self.prompts.push_back(("Ability Up".to_string(), growth::aerial_dodge_name(l), String::new()));
                    }
                }
                self.ad_level = Some(l);
            }
        }
        if self.seen.as_ref() == Some(&now) {
            return;
        }
        let from_kh1 = now.kh2();
        let list = |l: &[u8]| l.iter().take_while(|b| **b != 0).map(|b| if b & 0x80 != 0 { format!("({})", b & 0x7F) } else { b.to_string() }).collect::<Vec<_>>().join(" ");
        log(&format!("KH1 Sora: level {}, abilities [{}], shared [{}] (in brackets: not equipped)", now.level, list(&now.abilities), list(&now.shared)));
        log(&format!("KH1's menu gives KH2: {}", growth::describe(&from_kh1)));
        // The table's hooks (`growth::Side::Hook`): on, but no meaning on the other side yet.
        let names = |ids: Vec<u8>| ids.iter().map(|id| growth::name(*id)).collect::<Vec<_>>().join(", ");
        let (k1, k2) = (now.kh1_hooks(), now.kh2_hooks());
        if !k1.is_empty() || !k2.is_empty() {
            log(&format!("abilities on with nothing on the other side yet: KH1's [{}] while KH2 has him; KH2's [{}] while KH1's states have him", names(k1), names(k2)));
        }
        if self.mode == Mode::Kh1 {
            // (A combo ability only KH2 has makes the combo KH2's: `styles::kh2`.)
            super::styles::note_abilities(&from_kh1);
            ch.abilities = from_kh1;
        }
        self.seen = Some(now);
    }
}
