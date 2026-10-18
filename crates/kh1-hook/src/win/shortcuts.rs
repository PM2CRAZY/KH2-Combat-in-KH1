//! KH2's four shortcuts in KH1 (the user: four slots like KH2, KH1's own shortcut menu extended
//! so it fits in). `docs/kh1-shortcuts.md` has KH1's side.
//!
//! - The fourth spell is kept in the save at `+0x847` (the byte after KH1's three at `+0x844`,
//!   padding before the u16 at `+0x848`), as the spell + 1 (0 = none, so old saves start empty).
//!   KH1's get / set (`0x140282F20` / `0x140283640`) are replaced so slot 3 reads and writes it.
//! - The battle shortcut menu (`0x140529718`, a command list of type 5) gets four entries: its
//!   fill (`0x140292330`) is followed by ours for the fourth, and L1 + the cancel button (KH1's
//!   logical 6) runs it from the command step (`0x140291660`) at the point where KH1 runs its own
//!   three (L1 + Triangle / Square / confirm).
//! - Its drawing (`0x140279700`, case 5) is moved up one row (22 px) so the four rows end where
//!   KH1's three did, lined up with the command menu; its panel is stretched by a row (cut in
//!   two, a one-texel band between), the row loop runs four times and the fourth row gets the
//!   cancel button's icon (text icon 0x42; 0x41 is confirm, both follow the button swap).
//! - Customize's shortcut list (window `0x140548E40`, drawn by `0x140300510`; the overview
//!   `0x1403010E0`) gets a fourth row, labelled with KH1's third label with its icon swapped.

use super::addr::Rva;
use super::{config, hook, log};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

const SAVE_PTR_RVA: Rva = Rva(0x28695A0);
const SLOTS_OFF: usize = 0x844;
const SLOT4_OFF: usize = 0x847;
const FLAGS_OFF: usize = 0x874;

/// The battle shortcut menu: `+4` / `+8` disabled bits, `+0x10` count, `+0x14` command ids.
const MENU_RVA: Rva = Rva(0x529718);
/// The battle magic menu's entries, count and disabled bits (filled by the same function).
const MAGIC_ENTRIES_RVA: Rva = Rva(0x52964C);
const MAGIC_COUNT_RVA: Rva = Rva(0x529648);
const MAGIC_DISABLED_RVA: Rva = Rva(0x52963C);
/// Set when any shortcut is set (KH1 switches to the shortcut menu on L1 only then).
const HAS_SHORTCUTS_RVA: Rva = Rva(0x2D5445C);
/// The command menu chain's root, and the command picked this frame.
const ROOT_RVA: Rva = Rva(0x2D543F0);
const PICKED_RVA: Rva = Rva(0x2D543E8);
/// Logical buttons pressed this frame (6: cancel).
const PRESSED_RVA: Rva = Rva(0x298D4A8);
const CANCEL: u32 = 6;
/// No command (an empty shortcut).
const EMPTY: u8 = 0x42;

const COMMAND_RECORD_RVA: Rva = Rva(0x28D7D0);
const MAGIC_LEVEL_RVA: Rva = Rva(0x28D0D0);
const RUN_ENTRY_RVA: Rva = Rva(0x2912F0);

const GET_RVA: Rva = Rva(0x282F20);
/// `mov rax, [rip -> SAVE_PTR_RVA]` (Epic: 48 8B 05 79 66 5E 02).
fn get_prologue() -> [u8; 7] {
    super::addr::mov_rax_rip(GET_RVA, SAVE_PTR_RVA)
}
const SET_RVA: Rva = Rva(0x283640);
/// `mov rax, [rip -> SAVE_PTR_RVA]` (Epic: 48 8B 05 59 5F 5E 02).
fn set_prologue() -> [u8; 7] {
    super::addr::mov_rax_rip(SET_RVA, SAVE_PTR_RVA)
}
const LEARN_RVA: Rva = Rva(0x2830F0);
const LEARN_PROLOGUE: [u8; 6] = [0x48, 0x63, 0xC1, 0x44, 0x8B, 0xD2];
pub(super) const FILL_RVA: Rva = Rva(0x292330);
pub(super) const FILL_PROLOGUE: [u8; 12] = [0x4C, 0x8B, 0xDC, 0x41, 0x57, 0x48, 0x81, 0xEC, 0xC0, 0x00, 0x00, 0x00];
const STEP_RVA: Rva = Rva(0x291660);
const STEP_PROLOGUE: [u8; 13] = [0x40, 0x53, 0x55, 0x41, 0x54, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x20];

// Drawing (`0x140279700`, case 5).
const PANEL_FN_RVA: Rva = Rva(0x278880);
const ICON_FN_RVA: Rva = Rva(0x277E50);
const PANEL_CALLS: [Rva; 2] = [Rva(0x279E66), Rva(0x279E90)];
const ICON_X_CALL: Rva = Rva(0x279EE7);
/// `mov r9d, 0x11a` before the panels and icons (the menu's y).
const Y_MOVS: [Rva; 5] = [Rva(0x279E2C), Rva(0x279E81), Rva(0x279E95), Rva(0x279EB2), Rva(0x279ECF)];
/// `add r8d, 0x11a` (the rows' y).
const Y_ADD: Rva = Rva(0x279F32);
/// `lea ebp, [rdi - 0x11]` (3 rows, rdi = 0x14).
const ROWS_LEA: Rva = Rva(0x279F09);
/// `mov dword [0x1428531f0], 0x11a` (the menu's y, kept for later).
const Y_STORE: Rva = Rva(0x279F73);
const MENU_Y: i32 = 0x11A;
const ROW: i32 = 0x16;
/// The quad the panel calls build (`0x142867360`) and the draw list cursor.
const QUAD_RVA: Rva = Rva(0x2867360);
const LIST_RVA: Rva = Rva(0x2853218);
const ALPHA_RVA: Rva = Rva(0x2867444);
const DRAW_ON_RVA: Rva = Rva(0x528820);
const BUILD_RVA: Rva = Rva(0x27B300);
const SUBMIT_RVA: Rva = Rva(0x274340);

// Customize.
const WINDOW_ROWS_RVA: Rva = Rva(0x548E66);
const WINDOW_VISIBLE_RVA: Rva = Rva(0x548E68);
const LIST_LOOP: Rva = Rva(0x30075D);
const OVERVIEW_LOOP: Rva = Rva(0x3011DF);
const LIST_LABELS_RVA: Rva = Rva(0x548D40);
const OVERVIEW_LABELS_RVA: Rva = Rva(0x548F50);
/// KH1's label for the third shortcut (confirm).
const LABEL3: u32 = 0xD0;
pub const ID_LABEL4: u32 = 0x7F00_0005;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ON: AtomicBool = AtomicBool::new(false);
static ORIG_GET: AtomicUsize = AtomicUsize::new(0);
static ORIG_SET: AtomicUsize = AtomicUsize::new(0);
static ORIG_LEARN: AtomicUsize = AtomicUsize::new(0);
static ORIG_FILL: AtomicUsize = AtomicUsize::new(0);
/// Our fill hook is in (`reflect` adds its entry from it; else it hooks the fill itself).
static FILL_HOOKED: AtomicBool = AtomicBool::new(false);

pub fn fill_hooked() -> bool {
    FILL_HOOKED.load(Ordering::Acquire)
}
static ORIG_STEP: AtomicUsize = AtomicUsize::new(0);
static LABEL: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn base() -> usize {
    BASE.load(Ordering::Relaxed)
}

unsafe fn save() -> usize {
    rd::<u64>(base() + SAVE_PTR_RVA) as usize
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    if !super::settings::get().four_shortcuts {
        log("shortcuts: KH1's three (shortcuts = 3)");
        return;
    }
    // The slots first: everything else reads the fourth through them.
    if hook::install(base + GET_RVA, &get_prologue(), get as *const () as usize, &ORIG_GET, "shortcut get").is_none()
        || hook::install(base + SET_RVA, &set_prologue(), set as *const () as usize, &ORIG_SET, "shortcut set").is_none()
    {
        log("shortcuts: KH1's three (the slot functions weren't as expected)");
        return;
    }
    hook::install(base + LEARN_RVA, &LEARN_PROLOGUE, learn as *const () as usize, &ORIG_LEARN, "magic learned");
    if hook::install(base + FILL_RVA, &FILL_PROLOGUE, fill as *const () as usize, &ORIG_FILL, "battle menus fill").is_none() {
        return;
    }
    FILL_HOOKED.store(true, Ordering::Release);
    if hook::install(base + STEP_RVA, &STEP_PROLOGUE, step as *const () as usize, &ORIG_STEP, "command step").is_none() {
        return;
    }
    ON.store(true, Ordering::Release);
    // The battle menu: four entries (the fourth empty until filled).
    let menu = base + MENU_RVA;
    let mut old = 0u32;
    super::sys::VirtualProtect(menu as *mut _, 0x38, 0x04, &mut old);
    wr(menu + 0x14 + 3, EMPTY);
    wr(menu + 4, rd::<u32>(menu + 4) | 8);
    wr(menu + 8, rd::<u32>(menu + 8) | 8);
    wr(menu + 0x10, 4i32);
    install_draw(base);
    install_customize(base);
    let s = save();
    if s > 0x10000 {
        log(&format!("shortcuts: four (save bytes {:02x?})", std::slice::from_raw_parts((s + SLOTS_OFF) as *const u8, 4)));
    } else {
        log("shortcuts: four");
    }
}

unsafe fn install_draw(base: usize) {
    let y = (MENU_Y as u32).to_le_bytes();
    let up = ((MENU_Y - ROW) as u32).to_le_bytes();
    let mut ok = true;
    for at in Y_MOVS {
        ok &= hook::patch(base + at, &[0x41, 0xB9, y[0], y[1], y[2], y[3]], &[0x41, 0xB9, up[0], up[1], up[2], up[3]], "shortcut menu y");
    }
    ok &= hook::patch(base + Y_ADD, &[0x41, 0x81, 0xC0, y[0], y[1], y[2], y[3]], &[0x41, 0x81, 0xC0, up[0], up[1], up[2], up[3]], "shortcut rows y");
    ok &= hook::patch(base + Y_STORE + 6, &y, &up, "shortcut menu y kept");
    ok &= hook::patch(base + ROWS_LEA, &[0x8D, 0x6F, 0xEF], &[0x8D, 0x6F, 0xF0], "shortcut rows");
    for at in PANEL_CALLS {
        ok &= hook::redirect_call(base + at, base + PANEL_FN_RVA, panel as *const () as usize, "shortcut panel");
    }
    ok &= hook::redirect_call(base + ICON_X_CALL, base + ICON_FN_RVA, icon as *const () as usize, "shortcut icons");
    log(if ok { "shortcut menu: four rows" } else { "shortcut menu: some drawing patches failed (see above)" });
}

unsafe fn install_customize(base: usize) {
    if !config::TEXT_HOOKED.load(Ordering::Acquire) {
        log("Customize: three shortcut rows (no text lookup for the fourth's label)");
        return;
    }
    let id = ID_LABEL4.to_le_bytes();
    let ok = hook::patch(base + LIST_LABELS_RVA + 12, &[0, 0, 0, 0], &id, "Customize shortcut label")
        && hook::patch(base + OVERVIEW_LABELS_RVA + 12, &[0, 0, 0, 0], &id, "Customize overview label")
        && hook::patch(base + WINDOW_ROWS_RVA, &[3, 0, 3, 0], &[4, 0, 4, 0], "Customize shortcut rows")
        && hook::patch(base + LIST_LOOP, &[0x83, 0xFD, 0x03], &[0x83, 0xFD, 0x04], "Customize shortcut list")
        && hook::patch(base + OVERVIEW_LOOP, &[0x83, 0xFF, 0x03], &[0x83, 0xFF, 0x04], "Customize overview");
    debug_assert_eq!(WINDOW_VISIBLE_RVA.0, WINDOW_ROWS_RVA.0 + 2);
    log(if ok { "Customize: four shortcut rows" } else { "Customize: shortcut rows only partly patched (see above)" });
}

/// The fourth label: KH1's third with the confirm icon swapped for cancel's (or its 3 for a 4).
pub unsafe fn label4(kh1: impl Fn(u32) -> usize) -> usize {
    *LABEL.get_or_init(|| {
        let p = kh1(LABEL3);
        // KH1's text codes take operands (`0x1402CFEA0`), and an operand can be 0.
        let mut b = Vec::new();
        let mut i = 0;
        while i < 64 {
            let c = rd::<u8>(p + i);
            if c == 0 || c == 0x10 {
                break;
            }
            let n = match c {
                0x0D | 0x0E | 0x11..=0x14 => 3,
                0x0B => 4,
                0x08 => 5,
                0x07 | 0x0C | 0x0F | 0x15..=0x1F => 2,
                _ => 1,
            };
            for k in 0..n {
                b.push(rd::<u8>(p + i + k));
            }
            if c == 0x0F && b.last() == Some(&0x41) {
                *b.last_mut().unwrap() = 0x42;
            }
            i += n;
        }
        if !b.windows(2).any(|w| w == [0x0F, 0x42]) {
            b.extend_from_slice(&[0x0F, 0x42]);
        }
        log(&format!("Customize: fourth shortcut label from {LABEL3:#x}: {b:02x?}"));
        b.push(0);
        Box::leak(b.into_boxed_slice()).as_ptr() as usize
    })
}

/// A shortcut's spell (0..6) or -1.
unsafe extern "system" fn get(slot: i32) -> i32 {
    let s = save();
    if slot == 3 {
        let v = rd::<u8>(s + SLOT4_OFF);
        return if v == 0 || v > 7 { -1 } else { v as i32 - 1 };
    }
    rd::<i8>(s + SLOTS_OFF + slot as usize) as i32
}

unsafe extern "system" fn set(slot: i32, spell: u8) {
    let s = save();
    if slot == 3 {
        wr(s + SLOT4_OFF, if spell >= 7 { 0u8 } else { spell + 1 });
        log(&format!("fourth shortcut: {}", if spell >= 7 { "none".to_string() } else { format!("spell {spell}") }));
        return;
    }
    wr(s + SLOTS_OFF + slot as usize, spell);
}

/// A spell learned: KH1 puts it in the first empty of its three; with those full, the fourth.
unsafe extern "system" fn learn(member: i32, spell: u32) {
    let f: unsafe extern "system" fn(i32, u32) = std::mem::transmute(ORIG_LEARN.load(Ordering::Acquire));
    f(member, spell);
    if member != 0 || spell >= 7 {
        return;
    }
    if (0..4).any(|k| get(k) == spell as i32) || get(3) != -1 {
        return;
    }
    set(3, spell as u8);
}

/// KH1's battle menus, then the fourth shortcut (as KH1 fills its three).
unsafe extern "system" fn fill(a: usize, b: usize) {
    let f: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_FILL.load(Ordering::Acquire));
    f(a, b);
    // Reflect's entry in the magic list first (the fourth shortcut reads that list).
    super::reflect::magic_menu(a, b);
    if !ON.load(Ordering::Acquire) {
        return;
    }
    let base = base();
    let s = save();
    if s < 0x10000 || rd::<u8>(s + FLAGS_OFF) & 9 != 0 || rd::<i32>(base + MAGIC_COUNT_RVA) <= 0 {
        return;
    }
    let menu = base + MENU_RVA;
    let spell = get(3);
    let (entry, usable) = if (0..7).contains(&spell) {
        let level: unsafe extern "system" fn(u32) -> i8 = std::mem::transmute(base + MAGIC_LEVEL_RVA);
        let cmd = (level(spell as u32) as i32 + (spell + 2) * 3) as u8;
        // Usable as the magic menu has it (KH1's own three use the same test).
        let n = rd::<i32>(base + MAGIC_COUNT_RVA).clamp(0, 32) as usize;
        let off = rd::<u32>(base + MAGIC_DISABLED_RVA);
        let usable = (0..n).any(|k| rd::<u8>(base + MAGIC_ENTRIES_RVA + k) == cmd && off >> k & 1 == 0);
        wr(base + HAS_SHORTCUTS_RVA, 1u32);
        (cmd, usable)
    } else {
        (EMPTY, false)
    };
    wr(menu + 0x14 + 3, entry);
    for at in [menu + 4, menu + 8] {
        let v = rd::<u32>(at);
        wr(at, if usable { v & !8 } else { v | 8 });
    }
    static LOGGED: AtomicU32 = AtomicU32::new(u32::MAX);
    let key = entry as u32 | (usable as u32) << 8;
    if LOGGED.swap(key, Ordering::Relaxed) != key {
        log(&format!("fourth shortcut in battle: command {entry:#x}, usable {usable}"));
    }
}

/// KH1's command step, then the fourth shortcut: on the shortcut menu (L1 held), at the point
/// where KH1 checks its three (no command picked), L1 + cancel runs entry 3.
unsafe extern "system" fn step() -> u64 {
    let f: unsafe extern "system" fn() -> u64 = std::mem::transmute(ORIG_STEP.load(Ordering::Acquire));
    let base = base();
    // On the shortcut menu (L1 held since last frame), cancel is the fourth shortcut, not KH1's
    // jump (its jump check comes first and ends the step), so KH1 doesn't see it.
    let pressed = base + PRESSED_RVA;
    let want = rd::<u64>(base + ROOT_RVA) as usize == base + MENU_RVA && rd::<u32>(pressed) & CANCEL == CANCEL;
    if want {
        wr(pressed, rd::<u32>(pressed) & !CANCEL);
    }
    let r = f();
    if !want || rd::<u64>(base + ROOT_RVA) as usize != base + MENU_RVA {
        return r;
    }
    let record: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base + COMMAND_RECORD_RVA);
    if rd::<u64>(base + PICKED_RVA) as usize != record(0x43) {
        return r;
    }
    let run: unsafe extern "system" fn(usize, u32) -> u64 = std::mem::transmute(base + RUN_ENTRY_RVA);
    let done = run(base + MENU_RVA, 3);
    log(&format!("fourth shortcut: L1 + cancel ({})", if done != 0 { "cast" } else { "not usable" }));
    r
}

/// The shortcut menu's panels (`0x140278880`, re-done here): the tall one (header and three
/// rows) is drawn as a header and four rows, its gradient spread over them; the title as it is.
unsafe extern "system" fn panel(inst: usize, def: usize, x: i32, y: i32, scale: f32, p6: i32, p7: i32) {
    let base = base();
    let q = base + QUAD_RVA;
    let build: unsafe extern "system" fn(usize, usize, usize) = std::mem::transmute(base + BUILD_RVA);
    build(inst, q, def);
    let add = |off: usize, v: i32| wr(q + off, rd::<i32>(q + off).wrapping_add(v));
    add(0xA0, x * 16);
    add(0xB0, x * 16);
    add(0xA4, y * 16);
    add(0xC0, x * 16 + p7 * 16);
    add(0xD0, x * 16 + p7 * 16);
    add(0xB4, y * 16);
    add(0xC4, y * 16);
    add(0xD4, y * 16);
    add(0x40, p6 * 16);
    add(0x50, p6 * 16);
    let alpha = rd::<f32>(base + ALPHA_RVA);
    for v in 0..4 {
        let c = q + 0x60 + v * 0x10;
        for k in 0..3 {
            wr(c + k * 4, (rd::<i32>(c + k * 4) as f32 * scale) as i32);
        }
        wr(c + 12, (rd::<i32>(c + 12) as f32 * alpha) as i32);
    }
    if rd::<f32>(base + DRAW_ON_RVA) == 0.0 {
        return;
    }
    let submit: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + SUBMIT_RVA);
    let go = || wr(base + LIST_RVA, submit(q, 1, rd::<u64>(base + LIST_RVA) as usize) as u64);
    let (top, bottom) = (rd::<i32>(q + 0xA4), rd::<i32>(q + 0xB4));
    let e = ROW * 16;
    static SEEN: AtomicU32 = AtomicU32::new(0);
    if SEEN.fetch_add(1, Ordering::Relaxed) < 2 {
        log(&format!(
            "shortcut panel {def:#x}: x {} y {}..{} (uv {}..{} / {}..{}), menu y {y}",
            rd::<i32>(q + 0xA0) / 16,
            top / 16,
            bottom / 16,
            rd::<i32>(q + 0x20) / 16,
            rd::<i32>(q + 0x40) / 16,
            rd::<i32>(q + 0x24) / 16,
            rd::<i32>(q + 0x34) / 16
        ));
    }
    if bottom - top >= 48 * 16 {
        // Header, then four rows from KH1's three (22 px each, from the bottom up). Its
        // gradient spreads over the four: row k shows the texture's row 2k/3, a blend of the
        // two nearest rows (same shapes, so only the shading moves).
        let mut keep = [0u8; 0xE0];
        std::ptr::copy_nonoverlapping(q as *const u8, keep.as_mut_ptr(), keep.len());
        let (vt, vb) = (rd::<i32>(q + 0x24), rd::<i32>(q + 0x34));
        let ev = ((vb - vt) as i64 * e as i64 / (bottom - top) as i64) as i32;
        let (rows_y, rows_v) = (bottom - 3 * e, vb - 3 * ev);
        let piece = |y0: i32, y1: i32, v0: i32, v1: i32, alpha: f32| {
            std::ptr::copy_nonoverlapping(keep.as_ptr(), q as *mut u8, keep.len());
            for (off, v) in [(0xA4, y0), (0xC4, y0), (0xB4, y1), (0xD4, y1), (0x24, v0), (0x44, v0), (0x34, v1), (0x54, v1)] {
                wr(q + off, v);
            }
            if alpha < 1.0 {
                for k in 0..4 {
                    let c = q + 0x60 + k * 0x10 + 12;
                    wr(c, (rd::<i32>(c) as f32 * alpha) as i32);
                }
            }
            go();
        };
        piece(top, rows_y, vt, rows_v, 1.0);
        for k in 0..4 {
            let y = rows_y + k * e;
            let c = k as f32 * 2.0 / 3.0;
            let (a, f) = (c.floor() as i32, c.fract());
            piece(y, y + e, rows_v + a * ev, rows_v + (a + 1) * ev, 1.0);
            if f > 0.01 {
                piece(y, y + e, rows_v + (a + 1) * ev, rows_v + (a + 2) * ev, f);
            }
        }
    } else {
        go();
    }
}

/// The third row's icon (confirm), then the fourth's (cancel) a row lower.
unsafe extern "system" fn icon(inst: usize, glyph: u32, x: i32, y: i32, scale: f32) {
    let f: unsafe extern "system" fn(usize, u32, i32, i32, f32) = std::mem::transmute(base() + ICON_FN_RVA);
    f(inst, glyph, x, y, scale);
    f(inst, 0x42, x, y + ROW, scale);
}

/// L1 is held and opens KH1's shortcut menu (KH1 switches to it only when a shortcut is set,
/// `HAS_SHORTCUTS_RVA`): the face buttons are KH1's shortcuts then, not KH2's jump / action.
pub fn menu_on_l1(buttons: u16) -> bool {
    if buttons & super::takeover::L1 == 0 {
        return false;
    }
    let base = base();
    base != 0 && unsafe { rd::<u32>(base + HAS_SHORTCUTS_RVA) != 0 }
}

/// Where the command menu's current level is kept (`0x140291660`: `&ROOT_RVA` at the top level,
/// a deeper slot in a submenu: magic, items, summons...).
const LEVEL_PTR_RVA: Rva = Rva(0x2D54450);

/// A submenu of the command menu is open: Circle (KH1's cancel, `4`) backs out of it there, where
/// at the top level it is the jump (the menu step turns it into the jump flag `0x1000000`). The
/// user: with a KH2 move running, the magic / item / summon lists couldn't be left.
pub fn submenu_open() -> bool {
    let base = base();
    base != 0 && unsafe { rd::<u64>(base + LEVEL_PTR_RVA) as usize != base + ROOT_RVA }
}
