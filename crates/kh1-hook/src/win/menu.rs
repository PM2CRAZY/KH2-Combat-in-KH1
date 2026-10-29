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
//! testing: KH1's names, help and AP). KH1's own progression is vanilla (the tester, what-is-
//! still-missing after m17: "make it all vanilla, just give the player all the custom abilities
//! we added"): Sora simply has every one of ours, off until equipped. They live in the KH2 list
//! (after m18, answer 1: "give kh2's its own ability list right next to sora, should be titled
//! "KH2""; `kh2list`, Steam), a list of their own that KH1's Abilities menu shows as a row "KH2"
//! after Sora's and equips with Sora's AP, so Sora's own 48 are KH1's alone. In the other modes
//! they are taken back out of his lists, so a save stays KH1's own.
//!
//! KH1 counts an ability (`0x140282A70 (id, member)`) by scanning the list for the id, and its
//! AP sums walk the 48 bytes (Steam `0x1402E8660`, `0x1402E8710`: `cmp 0x30`), which `kh2list`
//! hooks to add the KH2 list's AP; nothing in KH1 indexes an array by ability id.

use super::addr::Rva;
use super::log;
use crate::growth::{self, Kh1Sora, Mode};

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

/// An ability's name in plain text, from the ability table KH1 reads (`0x142D37748`, entry
/// `(id & 0x7F) - 1`, its name handle at `+4`), for the In-game Scene Skip's notice (t785).
pub unsafe fn ability_name(base: usize, id: u8) -> Option<String> {
    let n = (id & 0x7F) as usize;
    if n == 0 || n > ENTRIES {
        return None;
    }
    let table = rd::<u64>(base + TABLE_PTR_RVA) as usize;
    let e = table.checked_add((n - 1) * 12)?;
    if table < 0x10000 || !super::crash::readable(e) || !super::crash::readable(e + 11) {
        return None;
    }
    let p = super::pose::ptr(rd::<u32>(e + 4));
    if p < 0x10000 || !super::crash::readable(p) {
        return None;
    }
    let s = decode(p);
    (!s.is_empty()).then_some(s)
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
        log!(Menus, "abilities: {line}");
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
            set_glide_name(after.kh2_glide_now());
        }
        log!(Menus,
            "abilities: Glide style {} set in the Config menu: shared list {} now ({:?})",
            if kh2 { "KH2" } else { "KH1" },
            if changed { "swapped" } else { "already right" },
            after.shared.iter().take_while(|b| **b != 0).collect::<Vec<_>>()
        );
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
        log!(Menus,
            "KH1's ability table {cur:#x} -> ours {ours:#x} ({} KH2 abilities added, ids {}..={}); {} | {} | {} | {} | {}",
            growth::MENU.len(),
            growth::FIRST_ID,
            growth::MENU.iter().map(|a| a.id).max().unwrap_or(0),
            sample(6),
            sample(growth::kh1::HURRICANE_BLAST as usize),
            sample(65),
            sample(68),
            sample(growth::ours::EXPLOSION as usize)
        );
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

/// KH1's party stats, with Valor's second Keyblade and the debug menu's locks on top.
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
    // KH2's combo length in Sora's own stats where it is his (`kh1picker::after_stats`, t896).
    super::kh1picker::after_stats(out);
    // (No extra AP for the KH2 abilities any more, nor for the form levels' abilities: the
    // tester, what-is-still-missing after m17, "remove extra ap for now". KH1's own AP only.)
}

// (KH1's reward-line hook, `0x1401B6990`, which hid Zantetsuken's "learned" line, is gone: the
// tester, combo row selection m22 answer 3, "bring back zantetsuken".)

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    super::hook::install(base + TABLE_LOAD_RVA, &TABLE_LOAD_PROLOGUE, table_load as *const () as usize, &ORIG_LOAD, "ability table load");
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
    log!(Menus, "prompt shown: {title} / {line1} / {line2} (box {index}, KH1's count was {n})");
    true
}

/// Boxes asked for from elsewhere (`formexp`'s form level-ups), shown as the "New Ability" ones
/// are (`Growth::update`).
static QUEUED: std::sync::Mutex<Vec<(String, String, String)>> = std::sync::Mutex::new(Vec::new());

/// Shows a box in KH1's prompt style (title, one line) when KH1's box is free.
pub fn queue_prompt(title: &str, line: &str) {
    if let Ok(mut q) = QUEUED.lock() {
        if q.len() < 16 {
            q.push((title.to_string(), line.to_string(), String::new()));
        }
    }
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

/// Writes Sora's two lists back (48 and 10 bytes).
unsafe fn write_lists(save: usize, abilities: &[u8], shared: &[u8]) {
    for (k, b) in abilities.iter().enumerate().take(48) {
        std::ptr::write((save + 0x44 + k) as *mut u8, *b);
    }
    for (k, b) in shared.iter().enumerate().take(10) {
        std::ptr::write((save + 0x599 + k) as *mut u8, *b);
    }
}

/// Places our abilities leave free in Sora's own list on a build without the KH2 list
/// (`kh2list::available`: not Steam, or its hooks didn't go in), as t718 did; they never take a
/// KH1 ability's place there.
const FALLBACK_RESERVE: usize = 2;

/// Free places in Sora's own list now.
unsafe fn free_now(save: usize) -> usize {
    let bytes: Vec<u8> = (0..48).map(|i| rd::<u8>(save + 0x44 + i)).collect();
    growth::free(&bytes)
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
    /// Our abilities moved over from Sora's own list this run (no new box for them).
    yielded: Vec<u8>,
    /// "New ability" boxes waiting for KH1's prompt box to be free: (title, line, line).
    prompts: std::collections::VecDeque<(String, String, String)>,
    /// Frames KH1's prompt box has been free.
    idle: u32,
    /// Quick Run's, High Jump's and KH2 Glide's levels last seen (None: not looked yet).
    qr_level: Option<u8>,
    hj_level: Option<u8>,
    gl_level: Option<u8>,
    /// The abilities last given before the debug menu's growth overrides (`debugmenu::
    /// growth_overrides`), and the overrides' count when they were applied.
    given: Option<character_core::Abilities>,
    ov_gen: u32,
    /// Quick Run's, Glide's and Aerial Dodge's levels last written into their menu names.
    names: [Option<u8>; 3],
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

/// Sora's KH2 abilities read from KH1's own lists now (t893; the owner: "go ahead with whats
/// open"): his ability list (save `+0x44`), the shared list (`+0x599`) and the KH2 list, as the
/// game holds them this instant, through the same table of ours that tells which KH2 ability
/// each entry is (`growth::Kh1Sora::kh2`; KH1 has no such numbers of its own), with the debug
/// menu's growth overrides on top; the `abilities` setting's fixed sets for its test modes. Read
/// at the moment of use (`combomods::abilities_now`) instead of from the copy our KH2 controller
/// object refreshes once a second (`Growth::update`).
pub unsafe fn kh1_abilities_now(base: usize) -> Option<character_core::Abilities> {
    let a = match super::settings::get().abilities {
        Mode::Test => growth::test(),
        Mode::All => growth::all(),
        Mode::Kh1 => Growth::kh1(base)?.kh2(),
    };
    Some(growth::with_growth_override(&a, &super::debugmenu::growth_overrides().0))
}

/// The menu's ability work (`Growth`), its own since t900 (it was part of our KH2 controller
/// object, `takeover`'s driver): each frame of Sora's on KH1's states (`takeover`).
static GROWTH: std::sync::Mutex<Option<Growth>> = std::sync::Mutex::new(None);

/// One frame of the ability menu's own work (`Growth::update`).
pub unsafe fn growth_frame(base: usize) {
    if let Ok(mut g) = GROWTH.try_lock() {
        g.get_or_insert_with(Growth::new).update(base);
    }
}

impl Growth {
    pub fn new() -> Growth {
        Growth { mode: super::settings::get().abilities, frames: 0, seen: None, applied: false, no_room: Vec::new(), yielded: Vec::new(), prompts: Default::default(), idle: 0, qr_level: None, hj_level: None, gl_level: None, given: None, ov_gen: 0, names: [None; 3] }
    }

    /// Quick Run's (0), Glide's (1) or Aerial Dodge's (2) menu name at level `l` (his own), or at
    /// the debug menu's forced level (`debugmenu::growth_override`; Off keeps his own name).
    fn name(&mut self, which: usize, l: u8) {
        let row = [super::debugmenu::GROWTH_QUICK_RUN, super::debugmenu::GROWTH_GLIDE, super::debugmenu::GROWTH_AERIAL_DODGE][which];
        let l = super::debugmenu::growth_override(row).filter(|&f| f > 0).unwrap_or(l);
        if l == 0 || self.names[which] == Some(l) {
            return;
        }
        self.names[which] = Some(l);
        match which {
            0 => set_quick_run_name(l),
            1 => set_glide_name(l),
            _ => set_aerial_dodge_name(l),
        }
    }

    /// Sora's side, with the KH2 list read whatever the save's mark (while moving it over).
    unsafe fn kh1_now(base: usize, use_list: bool) -> Option<Kh1Sora> {
        let mut s = Self::kh1(base)?;
        if use_list {
            if let Some(k) = super::kh2list::read(base) {
                s.kh2 = k;
            }
        }
        Some(s)
    }

    /// Writes a moved-over save's lists: Sora's own and the shared, and the KH2 list (or, on a
    /// build without it, its entries into Sora's own list where there's room).
    unsafe fn store(&mut self, base: usize, save: usize, m: &growth::Migrated, use_list: bool) {
        write_lists(save, &m.abilities, &m.shared);
        if use_list {
            super::kh2list::write(base, &m.kh2);
        } else {
            for b in m.kh2.iter().take_while(|b| **b != 0) {
                if !growth::has(&m.abilities, b & 0x7F) && !(free_now(save) > FALLBACK_RESERVE && add(save + 0x44, 48, *b)) {
                    log!(Menus, "abilities: no room for {} in Sora's list", growth::name(*b));
                }
            }
        }
    }

    /// Moves ours out of Sora's own 48 into the KH2 list (`growth::move_to_kh2_list`).
    unsafe fn to_kh2_list(&mut self, base: usize, save: usize, now: &mut Kh1Sora) {
        let m = growth::move_to_kh2_list(now);
        self.store(base, save, &m, true);
        log!(Menus, "abilities: {} of ours moved from Sora's own list to the KH2 list", m.lines.len());
        for line in m.lines.iter().chain(&m.removed) {
            log!(Menus, "abilities: {line}");
        }
        self.yielded.extend(m.kh2.iter().take_while(|b| **b != 0).map(|b| b & 0x7F));
        if let Some(again) = Self::kh1_now(base, true) {
            *now = again;
        }
    }

    /// Gives one of ours (`byte`: its id, bit 7 off): into the KH2 list, or Sora's own on a build
    /// without it; false with no room.
    unsafe fn give_ours(&mut self, base: usize, save: usize, byte: u8, on_list: bool) -> bool {
        if on_list {
            let Some(mut l) = super::kh2list::read(base) else { return false };
            let Some(i) = l.iter().position(|b| *b == 0) else { return false };
            l[i] = byte;
            super::kh2list::write(base, &l);
            true
        } else {
            free_now(save) > FALLBACK_RESERVE && add(save + 0x44, 48, byte)
        }
    }

    /// Sora's side of KH1's save block, if it is there.
    unsafe fn kh1(base: usize) -> Option<Kh1Sora> {
        let save = save_block(base)?;
        let level = rd::<u8>(save + 4);
        let abilities = std::slice::from_raw_parts((save + 0x44) as *const u8, 48).to_vec();
        let shared = std::slice::from_raw_parts((save + 0x599) as *const u8, 10).to_vec();
        let progress = std::slice::from_raw_parts((save + PROGRESS_OFF) as *const u8, 12).to_vec();
        // (The KH2 list once the save is on it; before, its bytes are nobody's.)
        let on_list = rd::<u16>(save + growth::LAYOUT_OFF) == growth::LAYOUT_KH2_LIST;
        let kh2 = if on_list { super::kh2list::read(base) } else { None }.unwrap_or_else(|| vec![0; growth::KH2_LIST_LEN]);
        Some(Kh1Sora { level, abilities, shared, kh2, progress, form_growth: super::formexp::base_growth() })
    }

    /// Gives Sora `a`, with the debug menu's growth overrides on it (the quick-run-native thread,
    /// m6: `debugmenu::growth_overrides`, all Normal without the debug tools).
    fn give(&mut self, a: character_core::Abilities) {
        let (ov, gen) = super::debugmenu::growth_overrides();
        if ov.iter().any(|o| o.is_some()) {
            log!(Debug, "debug growth overrides on Sora's abilities: {}", growth::describe(&growth::with_growth_override(&a, &ov)));
        }
        self.given = Some(a);
        self.ov_gen = gen;
    }

    /// Every second: KH1's menu and level, and Sora's abilities from them (`kh1`), or the
    /// mode's fixed set once.
    /// (t900: no KH2 controller object to give them to: what reads them reads KH1's lists itself,
    /// `kh1_abilities_now`; this is the menu's own work on those lists.)
    pub unsafe fn update(&mut self, base: usize) {
        self.frames = self.frames.wrapping_add(1);
        ensure_table(base);
        if !self.applied {
            self.applied = true;
            match self.mode {
                Mode::Test => self.give(growth::test()),
                Mode::All => self.give(growth::all()),
                Mode::Kh1 => self.frames = 59,
            }
            if self.mode != Mode::Kh1 {
                super::styles::note_abilities(&self.given.clone().unwrap_or_default());
            }
        }
        // A debug menu growth override changed: applied at once to what was last given.
        if super::debugmenu::growth_overrides().1 != self.ov_gen {
            if let Some(a) = self.given.clone() {
                self.give(a);
            }
        }
        if let Ok(mut q) = QUEUED.try_lock() {
            self.prompts.extend(q.drain(..));
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
                // A save the mod's older builds changed is moved over once (the save's mark,
                // `growth::LAYOUT_OFF`): the split (KH1's modifiers stood for KH2's before testing
                // s261, `growth::migrate`), ours into the KH2 list (after m18, answer 1,
                // `growth::move_to_kh2_list`), then KH1's vanilla progression put back (the tester,
                // what-is-still-missing after m17: "make it all vanilla ... existing saves get
                // updated", `growth::restore_vanilla`).
                let use_list = super::kh2list::available();
                let mark = rd::<u16>(save + growth::LAYOUT_OFF);
                if use_list && mark != growth::LAYOUT_KH2_LIST {
                    // (The KH2 list starts empty: its bytes were nobody's before.)
                    super::kh2list::write(base, &[0; growth::KH2_LIST_LEN]);
                    now.kh2 = vec![0; growth::KH2_LIST_LEN];
                }
                if mark != growth::LAYOUT_VANILLA && mark != growth::LAYOUT_KH2_LIST {
                    let touched = mark == growth::LAYOUT_SPLIT || growth::old_layout(&now) || growth::MENU.iter().any(|ab| growth::has(&now.abilities, ab.id) || growth::has(&now.shared, ab.id));
                    if mark != growth::LAYOUT_SPLIT && growth::old_layout(&now) {
                        let m = growth::migrate(&now);
                        self.store(base, save, &m, use_list);
                        log!(Menus, "abilities: a save from before the split, moved over ({} changes)", m.lines.len());
                        for line in &m.lines {
                            log!(Menus, "abilities: {line}");
                        }
                        if let Some(again) = Self::kh1_now(base, use_list) {
                            now = again;
                        }
                    }
                    // (Ours out of Sora's 48 first, so KH1's given back have their places.)
                    if use_list {
                        self.to_kh2_list(base, save, &mut now);
                    }
                    if touched {
                        let path = format!("{}\\kh1-kh2-removed.txt", super::exe_dir());
                        let text = std::fs::read_to_string(path).unwrap_or_default();
                        let m = growth::restore_vanilla(&now, &text);
                        self.store(base, save, &m, use_list);
                        log!(Menus, "abilities: back on KH1's vanilla progression ({} given back)", m.lines.len());
                        for line in m.lines.iter().chain(&m.removed) {
                            log!(Menus, "abilities: {line}");
                        }
                    } else {
                        log!(Menus, "abilities: a save the mod hadn't changed: KH1's vanilla progression as it is");
                    }
                    std::ptr::write_unaligned((save + growth::LAYOUT_OFF) as *mut u16, growth::LAYOUT_VANILLA);
                    if let Some(again) = Self::kh1_now(base, use_list) {
                        now = again;
                    }
                }
                // The KH2 list (after m18, answer 1): ours out of Sora's own 48, as they were, once.
                if use_list && rd::<u16>(save + growth::LAYOUT_OFF) != growth::LAYOUT_KH2_LIST {
                    self.to_kh2_list(base, save, &mut now);
                    std::ptr::write_unaligned((save + growth::LAYOUT_OFF) as *mut u16, growth::LAYOUT_KH2_LIST);
                    log!(Menus, "abilities: the KH2 abilities have their own list now (KH1's Abilities menu, row \"KH2\"); Sora's 48 are KH1's alone");
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
                let on_list = use_list && rd::<u16>(save + growth::LAYOUT_OFF) == growth::LAYOUT_KH2_LIST;
                // Quick Run and Aerial Dodge moved from the shared list into theirs (an older
                // build had them there); one that was on stays on.
                for ab in growth::MENU.iter().filter(|a| a.list == growth::List::Kh2) {
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
                    let byte = if on { ab.id } else { ab.id | 0x80 };
                    let moved = now.has_ours(ab.id) || self.give_ours(base, save, byte, on_list);
                    log!(Menus, "{} moved from the shared abilities to {} ({}){}", ab.name, if on_list { "the KH2 list" } else { "Sora's" }, if on { "on" } else { "off" }, if moved { "" } else { ": no room" });
                    if let Some(again) = Self::kh1(base) {
                        now = again;
                    }
                }
                // Every KH2 ability of ours is simply Sora's (the tester, after m17: "just give the
                // player all the custom abilities we added"), off until equipped, in the KH2 list
                // (KH1's own 48 untouched). One box for several at once.
                let mut learned: Vec<&'static str> = Vec::new();
                for ab in now.grants() {
                    // (Not one the debug menu took out this session.)
                    if super::debugmenu::ability_forced(ab.id) == Some(false) {
                        continue;
                    }
                    if self.give_ours(base, save, ab.id | 0x80, on_list) {
                        log!(Menus, "Sora is given {} (id {}, {} AP, off, {})", ab.name, ab.id, ab.ap, if on_list { "KH2 list" } else { "his own list" });
                        if !self.yielded.contains(&ab.id) {
                            learned.push(ab.name);
                        }
                    } else if !self.no_room.contains(&ab.id) {
                        self.no_room.push(ab.id);
                        if on_list {
                            log!(Menus, "no room for {} in the KH2 list (48)", ab.name);
                        } else {
                            log!(Menus, "no room for {} in Sora's list beside KH1's own (48, {FALLBACK_RESERVE} kept free for KH1; the KH2 list isn't in on this build)", ab.name);
                        }
                    }
                }
                match learned.len() {
                    0 => {}
                    1 | 2 => {
                        for n in learned {
                            self.prompts.push_back(("New Ability".to_string(), n.to_string(), String::new()));
                        }
                    }
                    n => self.prompts.push_back(("New Abilities".to_string(), format!("{n} KH2 Abilities"), String::new())),
                }
                if let Some(again) = Self::kh1(base) {
                    now = again;
                }
                // The fmlv rows' abilities (KH2's own form level-up rule, `0x1403C3F00`,
                // `formexp::rewards_due`): KH1's own counterpart in his list, off until equipped,
                // with its box. A list full of KH1's own stops that form's until there's room. One
                // without a counterpart yet is noted and passed.
                let mut stopped: Vec<super::form::Kind> = Vec::new();
                for (kind, level, kh2_id) in super::formexp::rewards_due() {
                    if stopped.contains(&kind) {
                        continue;
                    }
                    let what = growth::kh2_reward_name(kh2_id);
                    let Some(id) = growth::kh1_reward(kh2_id) else {
                        log!(Menus, "{} LV{level}: KH2 gives Sora {what} ({kh2_id}), which has no KH1 counterpart yet: not given", kind.name());
                        super::formexp::rewarded(kind, level);
                        continue;
                    };
                    let list = save + 0x44;
                    if add(list, 48, id | 0x80) {
                        log!(Menus, "{} LV{level}: Sora learns {} (id {id}, KH2's {what}), off", kind.name(), growth::name(id));
                        self.prompts.push_back(("New Ability".to_string(), growth::name(id), String::new()));
                        super::formexp::rewarded(kind, level);
                    } else {
                        stopped.push(kind);
                        if !self.no_room.contains(&id) {
                            self.no_room.push(id);
                            log!(Menus, "{} LV{level}: Sora can't learn {}: his ability list is full (48, KH1's most)", kind.name(), growth::name(id));
                        }
                    }
                }
            } else {
                // (KH1's Glide and Superglide back first: KH2's Glide stood for them, t323.)
                glide_style_list(save, &now, false);
                let n = strip(save + 0x44, 48) + strip(save + 0x599, 10);
                if n > 0 {
                    log!(Menus, "took {n} KH2 abilities back out of Sora's lists (abilities = {:?})", self.mode);
                }
            }
            if let Some(again) = Self::kh1(base) {
                now = again;
            }
        }
        // Quick Run's level: its menu name, and a box each time it grows (the user).
        if self.mode == Mode::Kh1 {
            let l = if now.has_ours(growth::QUICK_RUN_ID) { now.quick_run_now() } else { 0 };
            self.name(0, l);
            if self.qr_level != Some(l) {
                if let Some(old) = self.qr_level {
                    if old > 0 && l > old {
                        log!(Menus, "Quick Run grows: {}", growth::quick_run_name(l));
                        self.prompts.push_back(("Ability Up".to_string(), growth::quick_run_name(l), String::new()));
                    }
                }
                self.qr_level = Some(l);
            }
            // KH1's High Jump and Glide keep KH1's names (the split); the box says how far KH2's
            // own grows with them (only from the Drive Forms' levels now).
            let l = if growth::has(&now.shared, growth::kh1::HIGH_JUMP) { now.high_jump_now() } else { 0 };
            if self.hj_level != Some(l) {
                if let Some(old) = self.hj_level {
                    if old > 0 && l > old {
                        log!(Menus, "High Jump grows: {}", growth::high_jump_name(l));
                        self.prompts.push_back(("Ability Up".to_string(), growth::high_jump_name(l), String::new()));
                    }
                }
                self.hj_level = Some(l);
            }
            // KH2's own Glide (t323, Glide style KH2): its menu name and a box as it grows (only
            // from Final's level since m18 answer 3 (b)). At KH1 KH1's Glide and Superglide are vanilla: no boxes of ours.
            let l = if growth::has(&now.shared, growth::ours::GLIDE) { now.kh2_glide_now() } else { 0 };
            self.name(1, l);
            if self.gl_level != Some(l) {
                if let Some(old) = self.gl_level {
                    if old > 0 && l > old {
                        log!(Menus, "Glide grows: {}", growth::glide_name(l));
                        self.prompts.push_back(("Ability Up".to_string(), growth::glide_name(l), String::new()));
                    }
                }
                self.gl_level = Some(l);
            }
            // Aerial Dodge stays LV1 (KH2 grows it with Master Form, not on KH1's states here).
            let l = u8::from(now.has_ours(AERIAL_DODGE));
            self.name(2, l);
        }
        if self.seen.as_ref() == Some(&now) {
            return;
        }
        let from_kh1 = now.kh2();
        let list = |l: &[u8]| l.iter().take_while(|b| **b != 0).map(|b| if b & 0x80 != 0 { format!("({})", b & 0x7F) } else { b.to_string() }).collect::<Vec<_>>().join(" ");
        log!(Menus, "KH1 Sora: level {}, abilities [{}], shared [{}], KH2 list [{}] (in brackets: not equipped)", now.level, list(&now.abilities), list(&now.shared), list(&now.kh2));
        log!(Menus, "KH1's menu gives KH2: {}", growth::describe(&from_kh1));
        // The table's hooks (`growth::Side::Hook`): on, but no meaning on the other side yet.
        let names = |ids: Vec<u8>| ids.iter().map(|id| growth::name(*id)).collect::<Vec<_>>().join(", ");
        let (k1, k2) = (now.kh1_hooks(), now.kh2_hooks());
        if !k1.is_empty() || !k2.is_empty() {
            log!(Menus, "abilities on with nothing on the other side yet: KH1's [{}] while KH2 has him; KH2's [{}] while KH1's states have him", names(k1), names(k2));
        }
        if self.mode == Mode::Kh1 {
            // (A combo ability only KH2 has makes the combo KH2's: `styles::kh2`.)
            super::styles::note_abilities(&from_kh1);
            self.give(from_kh1);
        }
        self.seen = Some(now);
    }
}
