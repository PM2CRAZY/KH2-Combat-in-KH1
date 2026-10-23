//! Our Config menu rows (the user: switch between KH1's and KH2's camera in the game's Config
//! menu; then "toggle what sora uses for his ground and aerial moves"): Camera Style, Ground
//! Combo, Aerial Combo, Magic, Aerial Magic, each KH1 / KH2. (A Gummi Routes row went with the
//! Gummi skip: the user, keep the mod modular.)
//! `docs/kh1-config-menu.md` has KH1's side.
//!
//! - The Config list window's definition (`0x140547090`) gets 13 rows instead of 8.
//! - KH1 draws its rows at fixed places (`0x47` + row × 28, no scrolling), and 13 don't fit
//!   above the help box, so the list has two pages: KH1's 8 rows, and ours (rows 8..12) drawn
//!   in their places when the cursor is on one of ours. KH1's own drawing is then held back for
//!   the call: every row piece it draws is one panel call (`0x1402D5590`, hooked), skipped.
//! - Its update (`0x1402F7DD0`) is wrapped: on our rows, left / right / confirm switch the
//!   value (and are hidden from KH1's update, whose left / right there would step the
//!   difficulty); after it, our page is drawn with KH1's own panel calls.
//! - The help line (`0x1402F7D60`) reads its text id from a table by row; for our rows it is
//!   pointed at our id for the call.
//! - The menu's text lookup (`0x1402CE7C0`, sysmsg ids) is replaced: ids from `OUR_IDS` are our
//!   strings, the rest are looked up as KH1 does.
//!
//! The choices are kept in `kh1-kh2.ini` (`camera`, `ground_combo`, `air_combo`, `magic`,
//! `air_magic` = kh1 / kh2), not the save.

use super::addr::Rva;
use super::styles::Kind;
use super::{camera, hook, log, styles};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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
/// Up / down (the d-pad's and the stick's bits, as left / right).
const UP: u32 = 0x1000_0010;
const DOWN: u32 = 0x4000_0040;
const FOCUS_RVA: Rva = Rva(0x2E2E40);
const SOUND_RVA: Rva = Rva(0x177F60);
const PANEL_RVA: Rva = Rva(0x2D5590);
/// `push rbx; sub rsp, 0x50`.
const PANEL_PROLOGUE: [u8; 6] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x50];
/// The list cursor's row (`(win+0xFE + 8 + (win+0xFC & 0xF)) >> 4`).
const CURSOR_ROW_RVA: Rva = Rva(0x2ED030);
const DECOR_RVA: Rva = Rva(0x2F0740);

/// Our rows: 8 the camera, 9..16 the move styles (8 a page: rows 8..15, 16..) (`styles::Kind::EVERY` in order). Each:
/// label, left value, right value, help.
const FIRST_ROW: i32 = 8;
const ROWS: [[u32; 4]; 10] = [
    [0x7F00_0001, 0x7F00_0002, 0x7F00_0003, 0x7F00_0004],
    [0x7F00_0101, 0x7F00_0002, 0x7F00_0003, 0x7F00_0102],
    [0x7F00_0103, 0x7F00_0002, 0x7F00_0003, 0x7F00_0104],
    [0x7F00_0105, 0x7F00_0002, 0x7F00_0003, 0x7F00_0106],
    [0x7F00_0107, 0x7F00_0002, 0x7F00_0003, 0x7F00_0108],
    [0x7F00_0109, 0x7F00_0002, 0x7F00_0003, 0x7F00_010A],
    [0x7F00_010B, 0x7F00_0002, 0x7F00_0003, 0x7F00_010C],
    [0x7F00_010D, 0x7F00_0002, 0x7F00_0003, 0x7F00_010E],
    [0x7F00_0115, 0x7F00_0002, 0x7F00_0003, 0x7F00_0116],
    // (Player Model, off for now, s259, the user: "comment it out for now":)
    // [0x7F00_0117, 0x7F00_0002, 0x7F00_0003, 0x7F00_0118],
    [0x7F00_010F, 0x7F00_0002, 0x7F00_0003, 0x7F00_0110],
    // (Gummi Routes, off for now, the user: "comment it out" (the skip):)
    // [0x7F00_0111, 0x7F00_0113, 0x7F00_0114, 0x7F00_0112],
];
/// The camera row's help line (with the version under it).
const HELP_ID: u32 = 0x7F00_0004;
const STRINGS: [(u32, &str); 28] = [
    (0x7F00_0001, "Camera Style"),
    (0x7F00_0002, "KH1"),
    (0x7F00_0003, "KH2"),
    (0x7F00_0004, "Choose KH1's camera or KH2's field camera."),
    (0x7F00_0101, "Base Combo"),
    (0x7F00_0102, "Choose KH1's or KH2's ground combo for Sora.\nDrive Forms keep their own."),
    (0x7F00_0103, "Aerial Base Combo"),
    (0x7F00_0104, "Choose KH1's or KH2's aerial combo for Sora.\nDrive Forms keep their own."),
    (0x7F00_0105, "Magic"),
    (0x7F00_0106, "Choose KH1's or KH2's magic for Sora on the ground.\nDrive Forms keep their own."),
    (0x7F00_0107, "Aerial Magic"),
    (0x7F00_0108, "Choose KH1's or KH2's magic for Sora in the air.\nDrive Forms keep their own."),
    (0x7F00_0109, "Jump"),
    (0x7F00_010A, "Choose KH1's or KH2's jump for Sora.\nDrive Forms keep their own."),
    (0x7F00_010B, "Glide"),
    (0x7F00_010C, "Choose KH1's or KH2's glide for Sora.\nDrive Forms keep their own."),
    (0x7F00_010D, "Dodge Roll"),
    (0x7F00_010E, "Choose KH1's or KH2's dodge roll for Sora.\nDrive Forms keep their own."),
    (0x7F00_0115, "Misc Animations"),
    (0x7F00_0116, "Choose KH1's or KH2's animations for standing, walking,\nrunning and items."),
    (0x7F00_0117, "Player Model"),
    (0x7F00_0118, "Draw Sora with KH1's model or KH2's.\nDrive Forms and Keyblades follow."),
    (0x7F00_010F, "Battle Quotes"),
    (0x7F00_0110, "Choose KH1's or KH2's battle quotes for Sora."),
    (0x7F00_0111, "Gummi Routes"),
    (0x7F00_0112, "Fly the Gummi Ship between worlds, or skip the flights."),
    (0x7F00_0113, "Vanilla"),
    (0x7F00_0114, "Skip"),
];

/// The move style a row switches (`None`: the camera row).
/// The Gummi Routes row (our last; Vanilla | Skip).
fn gummi_row(row: i32) -> bool {
    // (Its row is commented out for now: none is the Gummi row.)
    let _ = row;
    false
}

fn style(row: i32) -> Option<Kind> {
    ROW_KINDS.get(index(row)).copied().flatten()
}

/// Each row's move style (`ROWS` in order; `None`: the camera and Gummi rows).
const ROW_KINDS: [Option<Kind>; 10] = [
    None,
    Some(Kind::GroundCombo),
    Some(Kind::AirCombo),
    Some(Kind::Magic),
    Some(Kind::AirMagic),
    Some(Kind::Jump),
    Some(Kind::Glide),
    Some(Kind::DodgeRoll),
    Some(Kind::MiscAnims),
    // Some(Kind::PlayerModel),
    Some(Kind::BattleQuotes),
    // None, (Gummi Routes, commented out)
];

/// A row's value: right (KH2, true) or left (KH1).
fn value(row: i32) -> bool {
    if gummi_row(row) {
        return super::gummi::SKIP.load(Ordering::Relaxed);
    }
    match style(row) {
        Some(k) => styles::kh2(k),
        None => camera::KH2.load(Ordering::Relaxed),
    }
}

fn toggle(row: i32) {
    let v = !value(row);
    if gummi_row(row) {
        super::gummi::set(v);
        return;
    }
    match style(row) {
        Some(k) => styles::set(k, v),
        None => {
            camera::KH2.store(v, Ordering::Relaxed);
            super::settings::save("camera", if v { "kh2" } else { "kh1" });
            log(&format!("Config menu: camera style {}", if v { "KH2" } else { "KH1" }));
        }
    }
}

fn ours(row: i32) -> bool {
    (FIRST_ROW..FIRST_ROW + (ROWS.len() as i32).min(SLOTS)).contains(&row)
}

/// KH1's list takes 16 rows at most (s242's 17th wrote past the window's row arrays over a
/// function pointer: the user's crash entering the Config menu). Ours are its rows 8..15, and
/// when we have more than 8 that page scrolls (`SCROLL`): Down on its last row brings the next
/// one up, Up on its first takes it back.
const SLOTS: i32 = 8;
const MAX_SCROLL: i32 = if ROWS.len() as i32 > SLOTS { ROWS.len() as i32 - SLOTS } else { 0 };
static SCROLL: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

/// Our row (index into `ROWS`) shown on KH1's row `row` (8..15).
fn index(row: i32) -> usize {
    (row - FIRST_ROW + SCROLL.load(Ordering::Relaxed)) as usize
}
fn place(row: i32) -> i32 {
    row - FIRST_ROW
}

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_UPDATE: AtomicUsize = AtomicUsize::new(0);
static ORIG_HELP: AtomicUsize = AtomicUsize::new(0);
static ORIG_TEXT: AtomicUsize = AtomicUsize::new(0);
static ORIG_PANEL: AtomicUsize = AtomicUsize::new(0);
/// KH1's Config update runs while our page shows: its own row pieces aren't drawn.
static HIDE_KH1: AtomicBool = AtomicBool::new(false);
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
    // (Before the update's hook: without it our page would show over KH1's rows.)
    if hook::install(base + PANEL_RVA, &PANEL_PROLOGUE, panel as *const () as usize, &ORIG_PANEL, "menu panel (Config pages)").is_none() {
        return;
    }
    if hook::install(base + UPDATE_RVA, &UPDATE_PROLOGUE, update as *const () as usize, &ORIG_UPDATE, "Config menu").is_none() {
        return;
    }
    let mut old = 0u32;
    super::sys::VirtualProtect((base + ROWS_RVA) as *mut _, 4, 0x04, &mut old);
    let n = 8 + (ROWS.len() as i32).min(SLOTS) as u16;
    wr(base + ROWS_RVA, n);
    wr(base + VISIBLE_RVA, n);
    log("Config menu: our rows added (Camera Style and the move styles, on pages of 8 after KH1's)");
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
    // Rows 8..12 read their help ids from just past the table (the max-value table's entries
    // 0..4, read only by the Config update's left / right, never during this call): ours for the
    // call, put back right after.
    // (The help id of the row scrolled into this place.)
    debug_assert_eq!(base + HELP_IDS_RVA + 8 * 4, base + MAX0_RVA);
    let slot = row;
    let at = base + HELP_IDS_RVA + slot as usize * 4;
    let keep = rd::<u32>(at);
    wr(at, ROWS[index(row)][3]);
    f(win);
    wr(at, keep);
}

/// KH1's panel draw (a label or a value of a menu row): held back while our page shows.
unsafe extern "system" fn panel(win: usize, style: i32, a: i32, b: i32, x: i32, y: i32, w: i32, h: i32, id: u32) {
    if HIDE_KH1.load(Ordering::Relaxed) {
        return;
    }
    let f: unsafe extern "system" fn(usize, i32, i32, i32, i32, i32, i32, i32, u32) = std::mem::transmute(ORIG_PANEL.load(Ordering::Acquire));
    f(win, style, a, b, x, y, w, h, id)
}

unsafe extern "system" fn update(win: usize) {
    let base = BASE.load(Ordering::Relaxed);
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_UPDATE.load(Ordering::Acquire));
    let focus: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + FOCUS_RVA);
    let cursor: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + CURSOR_ROW_RVA);
    let pad = rd::<u64>(base + PAD_PTR_RVA) as usize;
    let mut kept: Option<(u32, u32)> = None;
    // The row before this frame's cursor move: the page shown (KH1's and ours alike, so they
    // never both draw; a page change shows a frame late).
    let row = cursor(win);
    if pad > 0x10000 && ours(row) && focus(win) != 0 {
        let confirm = rd::<u32>(base + CONFIRM_MASK_RVA);
        let (pressed, repeat) = (rd::<u32>(pad + 8), rd::<u32>(pad + 0x10));
        if pressed & (LEFT | RIGHT | confirm) != 0 {
            toggle(row);
            let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
            sound(1);
        }
        // Our page scrolls when it has more rows than places: Down on its last place, Up on its
        // first, move the rows instead of KH1's cursor.
        let mut swallow = LEFT | RIGHT | confirm;
        let scroll = SCROLL.load(Ordering::Relaxed);
        if row == FIRST_ROW + SLOTS - 1 && repeat & DOWN != 0 && scroll < MAX_SCROLL {
            SCROLL.store(scroll + 1, Ordering::Relaxed);
            swallow |= DOWN;
            let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
            sound(0);
        } else if row == FIRST_ROW && repeat & UP != 0 && scroll > 0 {
            SCROLL.store(scroll - 1, Ordering::Relaxed);
            swallow |= UP;
            let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
            sound(0);
        }
        // KH1's update must not see them on our rows (its left / right would step the difficulty).
        kept = Some((pressed, repeat));
        wr(pad + 8, pressed & !swallow);
        wr(pad + 0x10, repeat & !swallow);
    }
    let our_page = ours(row);
    HIDE_KH1.store(our_page, Ordering::Relaxed);
    f(win);
    HIDE_KH1.store(false, Ordering::Relaxed);
    if our_page {
        for k in 0..(ROWS.len() as i32).min(SLOTS) {
            draw(base, win, FIRST_ROW + k);
        }
    }
    if let Some((p, r)) = kept {
        wr(pad + 8, p);
        wr(pad + 0x10, r);
    }
}

/// One of our rows, drawn as KH1 draws rows 0..5 (in the place of KH1's row `row - 8`): the
/// label, then the two choice panels.
unsafe fn draw(base: usize, win: usize, row: i32) {
    let panel: unsafe extern "system" fn(usize, i32, i32, i32, i32, i32, i32, i32, u32) = std::mem::transmute(base + PANEL_RVA);
    let decor: unsafe extern "system" fn(usize, i32, i32, i32) = std::mem::transmute(base + DECOR_RVA);
    let ids = ROWS[index(row)];
    let h = rd::<i32>(base + ROW_H_RVA);
    let y = 0x47 + place(row) * (h + rd::<i32>(base + ROW_GAP_RVA));
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
