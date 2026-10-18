//! A ninth Config menu row, "Camera Style: KH1 / KH2" (the user: switch between KH1's and KH2's
//! camera in the game's Config menu). (A tenth, Gummi Routes, went with the Gummi skip: the
//! user, keep the mod modular.)
//! `docs/kh1-config-menu.md` has KH1's side.
//!
//! - The Config list window's definition (`0x140547090`) gets 9 rows instead of 8.
//! - Its update (`0x1402F7DD0`) is wrapped: on our row, left / right / confirm switch the camera
//!   (and are hidden from KH1's update, whose left / right on a ninth row would step the
//!   difficulty); after it, our row is drawn with KH1's own panel calls. While it runs the rows
//!   are 25 apart instead of 28, so nine fit above the help box.
//! - The help line (`0x1402F7D60`) reads its text id from a table by row; for our row it is
//!   pointed at our id for the call.
//! - The menu's text lookup (`0x1402CE7C0`, sysmsg ids) is replaced: ids from `OUR_IDS` are our
//!   strings, the rest are looked up as KH1 does.
//!
//! The choice is kept in `kh1-kh2.ini` (`camera = kh1 / kh2`), not the save.

use super::addr::Rva;
use super::{camera, hook, log};
use std::sync::atomic::{AtomicUsize, Ordering};

const TEXT_RVA: Rva = Rva(0x2CE7C0);
/// Its first instruction (`mov rax, [rip -> TEXT_TABLE_RVA]`; Epic: 48 8B 05 89 CE B4 02).
fn text_prologue() -> [u8; 7] {
    super::addr::mov_rax_rip(TEXT_RVA, TEXT_TABLE_RVA)
}
const TEXT_TABLE_RVA: Rva = Rva(0x2E1B650);
const TEXT_BASE_RVA: Rva = Rva(0x2E1B648);
const UPDATE_RVA: Rva = Rva(0x2F7DD0);
const UPDATE_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18];
const HELP_RVA: Rva = Rva(0x2F7D60);
const HELP_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x40];
/// The Config window definition's row count and visible rows (u16).
const ROWS_RVA: Rva = Rva(0x5470B6);
const VISIBLE_RVA: Rva = Rva(0x5470B8);
/// Per-row help ids (8 u32); the ninth reads the next table's first entry (row 0's max value).
const HELP_IDS_RVA: Rva = Rva(0x5470C0);
const MAX0_RVA: Rva = Rva(0x5470E0);
/// Value panel styles by value: selected row, other rows (left, right).
const STYLES_SELECTED_RVA: Rva = Rva(0x547130);
const STYLES_OTHER_RVA: Rva = Rva(0x547140);
/// The row under the cursor (the help window reads it).
const ROW_RVA: Rva = Rva(0x2E998F8);
/// Row height and gap (shared by every menu).
const ROW_H_RVA: Rva = Rva(0x546050);
const ROW_GAP_RVA: Rva = Rva(0x546058);
/// Menu pad: `+8` pressed this frame, `+0x10` pressed with repeat.
const PAD_PTR_RVA: Rva = Rva(0x2E96320);
const CONFIRM_MASK_RVA: Rva = Rva(0x546390);
const LEFT: u32 = 0x8000_0080;
const RIGHT: u32 = 0x2000_0020;
const FOCUS_RVA: Rva = Rva(0x2E2E40);
const SOUND_RVA: Rva = Rva(0x177F60);
const PANEL_RVA: Rva = Rva(0x2D5590);
const DECOR_RVA: Rva = Rva(0x2F0740);

/// Our rows: 8 the camera. Each: label, left value, right value, help.
const FIRST_ROW: i32 = 8;
const ROWS: [[u32; 4]; 1] = [[0x7F00_0001, 0x7F00_0002, 0x7F00_0003, 0x7F00_0004]];
/// The camera row's help line (with the version under it).
const HELP_ID: u32 = 0x7F00_0004;
const STRINGS: [(u32, &str); 4] = [
    (0x7F00_0001, "Camera Style"),
    (0x7F00_0002, "KH1"),
    (0x7F00_0003, "KH2"),
    (0x7F00_0004, "Choose KH1's camera or KH2's field camera."),
];

/// A row's value: right (true) or left.
fn value(_row: i32) -> bool {
    camera::KH2.load(Ordering::Relaxed)
}

fn toggle(row: i32) {
    let v = !value(row);
    camera::KH2.store(v, Ordering::Relaxed);
    super::settings::save("camera", if v { "kh2" } else { "kh1" });
    log(&format!("Config menu: camera style {}", if v { "KH2" } else { "KH1" }));
}

fn ours(row: i32) -> bool {
    (FIRST_ROW..FIRST_ROW + ROWS.len() as i32).contains(&row)
}

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_UPDATE: AtomicUsize = AtomicUsize::new(0);
static ORIG_HELP: AtomicUsize = AtomicUsize::new(0);
static ORIG_TEXT: AtomicUsize = AtomicUsize::new(0);
static TEXT: std::sync::OnceLock<Vec<(u32, usize)>> = std::sync::OnceLock::new();

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    TEXT.get_or_init(|| {
        STRINGS
            .iter()
            .map(|(id, s)| {
                // The camera row's help carries the mod's version on its second line (the user:
                // a version shown in the Config menu, to tell which build a PC runs).
                let s = if *id == HELP_ID { format!("{s}\nKH2 Combat version {}", crate::VERSION) } else { s.to_string() };
                (*id, Box::leak(super::menu::encode(&s).into_boxed_slice()).as_ptr() as usize)
            })
            .collect()
    });
    // The text lookup carries our strings for the shortcuts too (`shortcuts.rs`).
    if hook::install(base + TEXT_RVA, &text_prologue(), text as *const () as usize, &ORIG_TEXT, "menu text lookup").is_none() {
        return;
    }
    TEXT_HOOKED.store(true, Ordering::Release);
    if !super::settings::get().config_row {
        log("Config menu: no rows of ours (config_row = off)");
        return;
    }
    if rd::<u16>(base + ROWS_RVA) != 8 || rd::<u16>(base + VISIBLE_RVA) != 8 {
        log(&format!("Config menu: unexpected row counts {} / {}: no rows of ours", rd::<u16>(base + ROWS_RVA), rd::<u16>(base + VISIBLE_RVA)));
        return;
    }
    if hook::install(base + HELP_RVA, &HELP_PROLOGUE, help as *const () as usize, &ORIG_HELP, "Config help line").is_none() {
        return;
    }
    if hook::install(base + UPDATE_RVA, &UPDATE_PROLOGUE, update as *const () as usize, &ORIG_UPDATE, "Config menu").is_none() {
        return;
    }
    let mut old = 0u32;
    super::sys::VirtualProtect((base + ROWS_RVA) as *mut _, 4, 0x04, &mut old);
    let n = 8 + ROWS.len() as u16;
    wr(base + ROWS_RVA, n);
    wr(base + VISIBLE_RVA, n);
    log("Config menu: Camera Style row added");
}

/// KH1's sysmsg lookup (its own instructions, re-done here: its first one is RIP-relative, so
/// the original can't be called), with our ids on top.
unsafe extern "system" fn text(id: u32) -> usize {
    if let Some(t) = TEXT.get().and_then(|t| t.iter().find(|(i, _)| *i == id)) {
        return t.1;
    }
    if id == super::shortcuts::ID_LABEL4 {
        return super::shortcuts::label4(|i| kh1_text(i));
    }
    if let Some(p) = super::reflect::text(id) {
        return p;
    }
    kh1_text(id)
}

/// Whether our text lookup is in (our ids can be used).
pub static TEXT_HOOKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// KH1's own sysmsg text for `id`.
pub unsafe fn kh1_text(id: u32) -> usize {
    let base = BASE.load(Ordering::Relaxed);
    let table = rd::<u64>(base + TEXT_TABLE_RVA) as usize;
    let at = rd::<u64>(base + TEXT_BASE_RVA) as usize;
    let entry = table.wrapping_add(id.wrapping_sub(1) as usize * 2);
    // An id past the table (one of ours read from a table KH1 has only three of) reads as
    // empty instead of wild memory.
    if id == 0 || id >= 0x10000 || (id > 0x2000 && !super::crash::readable(entry)) {
        static EMPTY: [u8; 1] = [0];
        return EMPTY.as_ptr() as usize;
    }
    at.wrapping_add(rd::<u16>(entry) as usize)
}

unsafe extern "system" fn help(win: usize) {
    let base = BASE.load(Ordering::Relaxed);
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_HELP.load(Ordering::Acquire));
    let row = rd::<i32>(base + ROW_RVA);
    if !ours(row) {
        return f(win);
    }
    // Row 8 reads its help id from just past the table (the next table's first entry): ours
    // for the call.
    debug_assert_eq!(base + HELP_IDS_RVA + 8 * 4, base + MAX0_RVA);
    let at = base + HELP_IDS_RVA + row as usize * 4;
    let keep = rd::<u32>(at);
    wr(at, ROWS[(row - FIRST_ROW) as usize][3]);
    f(win);
    wr(at, keep);
}

unsafe extern "system" fn update(win: usize) {
    let base = BASE.load(Ordering::Relaxed);
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_UPDATE.load(Ordering::Acquire));
    let focus: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + FOCUS_RVA);
    let pad = rd::<u64>(base + PAD_PTR_RVA) as usize;
    let mut kept: Option<(u32, u32)> = None;
    let row = rd::<i32>(base + ROW_RVA);
    if pad > 0x10000 && ours(row) && focus(win) != 0 {
        let confirm = rd::<u32>(base + CONFIRM_MASK_RVA);
        let (pressed, repeat) = (rd::<u32>(pad + 8), rd::<u32>(pad + 0x10));
        if pressed & (LEFT | RIGHT | confirm) != 0 {
            toggle(row);
            let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
            sound(1);
        }
        // KH1's update must not see them on our rows (its left / right would step the difficulty).
        kept = Some((pressed, repeat));
        wr(pad + 8, pressed & !(LEFT | RIGHT | confirm));
        wr(pad + 0x10, repeat & !(LEFT | RIGHT | confirm));
    }
    // Nine rows 25 apart (instead of 28) during the call, so ours clears the help box.
    let (h, gap) = (rd::<i32>(base + ROW_H_RVA), rd::<i32>(base + ROW_GAP_RVA));
    wr(base + ROW_H_RVA, ROW_PITCH.min(h));
    wr(base + ROW_GAP_RVA, 0i32);
    f(win);
    for k in 0..ROWS.len() as i32 {
        draw(base, win, FIRST_ROW + k);
    }
    wr(base + ROW_H_RVA, h);
    wr(base + ROW_GAP_RVA, gap);
    if let Some((p, r)) = kept {
        wr(pad + 8, p);
        wr(pad + 0x10, r);
    }
}

/// Our rows' pitch while the Config list runs.
const ROW_PITCH: i32 = 25;

/// One of our rows, drawn as KH1 draws rows 0..5: the label, then the two choice panels.
unsafe fn draw(base: usize, win: usize, row: i32) {
    let panel: unsafe extern "system" fn(usize, i32, i32, i32, i32, i32, i32, i32, u32) = std::mem::transmute(base + PANEL_RVA);
    let decor: unsafe extern "system" fn(usize, i32, i32, i32) = std::mem::transmute(base + DECOR_RVA);
    let ids = ROWS[(row - FIRST_ROW) as usize];
    let h = rd::<i32>(base + ROW_H_RVA);
    let y = 0x47 + row * (h + rd::<i32>(base + ROW_GAP_RVA));
    let selected = rd::<i32>(base + ROW_RVA) == row;
    if selected {
        let y2 = h / 2 - 1 + y;
        decor(win, -0x40, y2, 0x4B);
        decor(win, 0x99, y2, 6);
        decor(win, 0x141, y2, 6);
        decor(win, 0x1E9, y2, 0x17);
    }
    panel(win, if selected { 0xB } else { 0 }, 2, 0, 0x0B, y, 0x8E, 0x1B, ids[0]);
    let v = value(row) as usize;
    let styles = base + if selected { STYLES_SELECTED_RVA } else { STYLES_OTHER_RVA };
    let (left, right) = (rd::<u32>(styles + v * 8) as i32, rd::<u32>(styles + v * 8 + 4) as i32);
    panel(win, left, 2, 2, 0x9F, y, 0xA2, 0xF0, ids[1]);
    panel(win, right, 2, 2, 0x147, y, 0xA2, 0x198, ids[2]);
}
