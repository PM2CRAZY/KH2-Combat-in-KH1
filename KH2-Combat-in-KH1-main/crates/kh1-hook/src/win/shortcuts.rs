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
//!
//! **Reflect** (`reflect`, spell 7, which KH1 has no data for) can be put in any shortcut (with
//! `shortcuts = 3` too). KH1's own slot bytes never hold it, so nothing of KH1's reads spell 7 from
//! them (KH1's fill would index its spell table by it): a slot holding Reflect is KH1's "none"
//! (`0xFF`) with its bit set in `+0x847` (bits 4..6 for slots 0..2; the fourth's spell + 1 is
//! 8). After KH1's fill such an entry is our command 0x7D, usable as the Magic menu's Reflect, so
//! L1 + its button gives KH1's command step the same command as the menu and `combat::command`
//! hands it to `reflect::chosen`. KH1's learn (`0x1402830F0`, a new spell into the first empty
//! slot) sees those slots as taken. Customize: the spell list (`0x142E9A9F0`, 8 ints, KH1's 7
//! spells and "none") gets Reflect before "none", so it is moved to a buffer of ours (the four
//! instructions that address it repointed), its rows pitched closer when there are 9, its help
//! line ours (`reflect::HELP_ID`), and the spell names (`0x14028D100 (spell)`, 0 past 6) answer
//! 7 with Reflect's level name.

use super::addr::Rva;
use super::{config, hook, log};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

const SAVE_PTR_RVA: Rva = Rva(0x28695A0);
const SLOTS_OFF: usize = 0x844;
/// Low nibble: the fourth shortcut's spell + 1 (0 none, 8 Reflect); bits 4..6: KH1's slots 0..2
/// hold Reflect (only while their byte is KH1's none, `0xFF`).
const SLOT4_OFF: usize = 0x847;
const FLAGS_OFF: usize = 0x874;
/// Reflect's spell number (KH1's are 0..6).
const REFLECT: i32 = super::reflect::SPELL as i32;
const NONE: u8 = 0xFF;

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
pub(super) const STEP_RVA: Rva = Rva(0x291660);
pub(super) const STEP_PROLOGUE: [u8; 13] = [0x40, 0x53, 0x55, 0x41, 0x54, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x20];

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

// Customize's spell list (Reflect in it).
/// The list (8 ints: the spells learned, then 7 for "none", always last) and its count; the
/// cursor in it.
const PICK_LIST_RVA: Rva = Rva(0x2E9A9F0);
const PICK_COUNT_RVA: Rva = Rva(0x2E9A9E4);
/// Its fills (`0x1402D8AA0` calls the first as the pause menu opens; the second has the same
/// body) and their first bytes (`mov [rsp+8], rbx; push rdi; sub rsp, 0x20`).
const PICK_FILLS: [Rva; 2] = [Rva(0x301D60), Rva(0x301E50)];
const PICK_FILL_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];
/// Every instruction that addresses the list: `lea reg, [rip -> PICK_LIST_RVA]` (help line,
/// list window, the two fills).
const PICK_LEAS: [(Rva, [u8; 3]); 4] = [(Rva(0x30029A), [0x48, 0x8D, 0x0D]), (Rva(0x300322), [0x48, 0x8D, 0x35]), (Rva(0x301D86), [0x48, 0x8D, 0x3D]), (Rva(0x301E5C), [0x48, 0x8D, 0x3D])];
/// The list window's update (it draws the rows) and its first bytes.
const PICK_UPDATE_RVA: Rva = Rva(0x300300);
const PICK_UPDATE_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18];
/// The help line's `call 0x14028D0D0` (the level; its help id is list entry × 3 + 0xD4 + it).
const HELP_LEVEL_CALL: Rva = Rva(0x30028E);
/// A spell's name (`0x14028D100 (spell)`: KH1's word for its command, 0 past spell 6) and the
/// calls to it (list rows, the chosen one, the shortcut list, the overview).
const SPELL_NAME_RVA: Rva = Rva(0x28D100);
const SPELL_NAME_CALLS: [Rva; 4] = [Rva(0x3003DD), Rva(0x30042B), Rva(0x3006F0), Rva(0x301177)];
/// Menu rows' height and gap (every menu's), and the pitch our 9-row list uses.
const ROW_H_RVA: Rva = Rva(0x546050);
const ROW_GAP_RVA: Rva = Rva(0x546058);
const PICK_PITCH: i32 = 25;
/// Our copy of the list (16 ints), near the executable.
static PICK_BUF: AtomicUsize = AtomicUsize::new(0);
static ORIG_PICK_FILLS: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
static ORIG_PICK_UPDATE: AtomicUsize = AtomicUsize::new(0);

static BASE: AtomicUsize = AtomicUsize::new(0);
/// The fourth shortcut is on (`shortcuts = 4`).
static ON: AtomicBool = AtomicBool::new(false);
/// Reflect can be put in shortcuts (`reflect = on`).
static REFL: AtomicBool = AtomicBool::new(false);
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

/// The command step is hooked here (`shortcuts = 4`): `drive` runs its page flip in it.
pub fn step_hooked() -> bool {
    ORIG_STEP.load(Ordering::Acquire) != 0
}
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
    let (four, reflect) = (super::settings::get().four_shortcuts, super::settings::get().reflect);
    if !four && !reflect {
        log("shortcuts: KH1's three (shortcuts = 3)");
        return;
    }
    // The slots first: everything else reads the fourth and Reflect through them.
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
    if reflect {
        REFL.store(true, Ordering::Release);
        install_reflect(base);
    }
    if !four {
        log("shortcuts: KH1's three (shortcuts = 3), Reflect can go in them");
        return;
    }
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

/// `lea reg, [rip -> target]` at `at` (7 bytes; `op` its first three) as the running build has it.
fn lea_rip(op: [u8; 3], at: usize, target: usize) -> Option<[u8; 7]> {
    let d = i32::try_from(target as i64 - (at as i64 + 7)).ok()?.to_le_bytes();
    Some([op[0], op[1], op[2], d[0], d[1], d[2], d[3]])
}

/// Reflect in Customize: its own spell list (room for Reflect), names, help line and row pitch.
unsafe fn install_reflect(base: usize) {
    if !config::TEXT_HOOKED.load(Ordering::Acquire) {
        log("Customize: no Reflect in the spell list (no text lookup for its help)");
        return;
    }
    // Every instruction that addresses KH1's list, checked before any is changed.
    let list = base + PICK_LIST_RVA;
    let ok = PICK_LEAS.iter().all(|(at, op)| {
        let at = base + *at;
        lea_rip(*op, at, list).is_some_and(|want| std::slice::from_raw_parts(at as *const u8, 7) == want)
    }) && PICK_FILLS.iter().all(|f| std::slice::from_raw_parts((base + *f) as *const u8, PICK_FILL_PROLOGUE.len()) == PICK_FILL_PROLOGUE)
        && std::slice::from_raw_parts((base + PICK_UPDATE_RVA) as *const u8, PICK_UPDATE_PROLOGUE.len()) == PICK_UPDATE_PROLOGUE;
    if !ok {
        log("Customize: KH1's spell list code wasn't as expected: no Reflect in it (battle shortcuts that hold it still work)");
        return;
    }
    let Some(buf) = hook::alloc_near(base + PICK_LEAS[0].0, 0x1000) else {
        log("Customize: no memory near the game for the spell list: no Reflect in it");
        return;
    };
    // KH1's list and count as they are (empty before the pause menu first opens).
    std::ptr::copy_nonoverlapping(list as *const u8, buf as *mut u8, 8 * 4);
    PICK_BUF.store(buf, Ordering::Release);
    // The fills (they write the list through the repointed `lea`), the window and the help line.
    let fills: [usize; 2] = [pick_fill0 as *const () as usize, pick_fill1 as *const () as usize];
    for (k, f) in PICK_FILLS.iter().enumerate() {
        if hook::install(base + *f, &PICK_FILL_PROLOGUE, fills[k], &ORIG_PICK_FILLS[k], "Customize spell list fill").is_none() {
            return;
        }
    }
    let mut all = true;
    for (at, op) in PICK_LEAS {
        let at = base + at;
        let (Some(old), Some(new)) = (lea_rip(op, at, list), lea_rip(op, at, buf)) else {
            all = false;
            continue;
        };
        all &= hook::patch(at, &old, &new, "Customize spell list address");
    }
    all &= hook::install(base + PICK_UPDATE_RVA, &PICK_UPDATE_PROLOGUE, pick_update as *const () as usize, &ORIG_PICK_UPDATE, "Customize spell list window").is_some();
    all &= hook::redirect_call(base + HELP_LEVEL_CALL, base + MAGIC_LEVEL_RVA, help_level as *const () as usize, "Customize spell help");
    for at in SPELL_NAME_CALLS {
        all &= hook::redirect_call(base + at, base + SPELL_NAME_RVA, spell_name as *const () as usize, "Customize spell name");
    }
    log(&format!("Customize: Reflect can go in the shortcuts{} (spell list moved to {buf:#x})", if all { "" } else { ", some patches failed (see above)" }));
}

/// KH1's Customize spell list (now ours) and its count.
unsafe fn pick_list() -> (usize, usize) {
    let buf = PICK_BUF.load(Ordering::Acquire);
    (buf, rd::<i32>(base() + PICK_COUNT_RVA).clamp(0, 15) as usize)
}

/// After KH1's fill (spells learned, then 7 for "none"): Reflect before "none" when Sora has it.
unsafe fn pick_fill_after() {
    let (buf, n) = pick_list();
    if buf == 0 || n == 0 || n >= 15 || !super::reflect::offered() {
        return;
    }
    let last = rd::<i32>(buf + (n - 1) * 4);
    wr(buf + (n - 1) * 4, REFLECT);
    wr(buf + n * 4, last);
    wr(base() + PICK_COUNT_RVA, n as i32 + 1);
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 5 {
        log(&format!("Customize: spell list {:?} (Reflect = 7 before none)", (0..=n).map(|k| rd::<i32>(buf + k * 4)).collect::<Vec<_>>()));
    }
}

unsafe extern "system" fn pick_fill0() {
    let f: unsafe extern "system" fn() = std::mem::transmute(ORIG_PICK_FILLS[0].load(Ordering::Acquire));
    f();
    pick_fill_after();
}

unsafe extern "system" fn pick_fill1() {
    let f: unsafe extern "system" fn() = std::mem::transmute(ORIG_PICK_FILLS[1].load(Ordering::Acquire));
    f();
    pick_fill_after();
}

/// The spell list window's update: with 9 rows (7 spells, Reflect, none) its rows are pitched
/// closer during the call, so the last clears the help box under it (as `config`'s nine rows).
unsafe extern "system" fn pick_update(win: usize) {
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_PICK_UPDATE.load(Ordering::Acquire));
    let base = base();
    if pick_list().1 <= 8 {
        return f(win);
    }
    let (h, gap) = (rd::<i32>(base + ROW_H_RVA), rd::<i32>(base + ROW_GAP_RVA));
    wr(base + ROW_H_RVA, PICK_PITCH.min(h));
    wr(base + ROW_GAP_RVA, 0i32);
    f(win);
    wr(base + ROW_H_RVA, h);
    wr(base + ROW_GAP_RVA, gap);
}

/// The help line's level call (KH1 passes the cursor; the help id is entry × 3 + 0xD4 + this):
/// on Reflect's row, what makes the id `reflect::HELP_ID`.
unsafe extern "system" fn help_level(cursor: i32) -> i32 {
    let (buf, n) = pick_list();
    if buf != 0 && cursor >= 0 && (cursor as usize) + 1 < n && rd::<i32>(buf + cursor as usize * 4) == REFLECT {
        return (super::reflect::HELP_ID as i32).wrapping_sub(REFLECT * 3 + 0xD4);
    }
    let f: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(base() + MAGIC_LEVEL_RVA);
    f(cursor)
}

/// A spell's name for Customize: Reflect's level name for 7 (0, KH1's "none", when it isn't
/// offered); KH1's for the rest.
unsafe extern "system" fn spell_name(spell: u32) -> usize {
    if spell == REFLECT as u32 {
        return super::reflect::name_now().unwrap_or(0);
    }
    let f: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base() + SPELL_NAME_RVA);
    f(spell)
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

/// A shortcut's spell (0..6, 7 Reflect) or -1.
unsafe extern "system" fn get(slot: i32) -> i32 {
    let s = save();
    let ours = rd::<u8>(s + SLOT4_OFF);
    if slot == 3 {
        let v = ours & 0xF;
        return if !ON.load(Ordering::Relaxed) || v == 0 || v > 8 || (v == 8 && !REFL.load(Ordering::Relaxed)) { -1 } else { v as i32 - 1 };
    }
    if !(0..3).contains(&slot) {
        return -1;
    }
    let b = rd::<u8>(s + SLOTS_OFF + slot as usize);
    if b == NONE && ours >> (4 + slot) & 1 != 0 && REFL.load(Ordering::Relaxed) {
        return REFLECT;
    }
    b as i8 as i32
}

/// Sets a shortcut (Customize): a spell 0..6, 7 Reflect, else none.
unsafe extern "system" fn set(slot: i32, spell: u8) {
    let s = save();
    let ours = rd::<u8>(s + SLOT4_OFF);
    let name = |spell: u8| match spell {
        0..=6 => format!("spell {spell}"),
        7 => "Reflect".to_string(),
        _ => "none".to_string(),
    };
    if slot == 3 {
        wr(s + SLOT4_OFF, ours & 0xF0 | if spell <= REFLECT as u8 { spell + 1 } else { 0 });
        log(&format!("fourth shortcut: {}", name(spell)));
        return;
    }
    if !(0..3).contains(&slot) {
        return;
    }
    let bit = 1u8 << (4 + slot);
    if spell == REFLECT as u8 {
        // KH1's byte stays its none: nothing of KH1's ever reads spell 7 from it.
        wr(s + SLOTS_OFF + slot as usize, NONE);
        wr(s + SLOT4_OFF, ours | bit);
    } else {
        wr(s + SLOTS_OFF + slot as usize, spell);
        wr(s + SLOT4_OFF, ours & !bit);
    }
    if spell == REFLECT as u8 || ours & bit != 0 {
        log(&format!("shortcut {}: {}", slot + 1, name(spell)));
    }
}

/// A spell learned: KH1 puts it in the first empty of its three (a slot holding Reflect is not
/// empty); with those full, the fourth.
unsafe extern "system" fn learn(member: i32, spell: u32) {
    let f: unsafe extern "system" fn(i32, u32) = std::mem::transmute(ORIG_LEARN.load(Ordering::Acquire));
    // Reflect's slots read as taken for KH1's learn (a placeholder, put back after).
    const TAKEN: u8 = 0xFE;
    let s = save();
    let mut held = [false; 3];
    if s > 0x10000 {
        for (k, h) in held.iter_mut().enumerate() {
            if get(k as i32) == REFLECT {
                wr(s + SLOTS_OFF + k, TAKEN);
                *h = true;
            }
        }
    }
    f(member, spell);
    for (k, h) in held.iter().enumerate() {
        if *h && rd::<u8>(s + SLOTS_OFF + k) == TAKEN {
            wr(s + SLOTS_OFF + k, NONE);
        }
    }
    if !ON.load(Ordering::Relaxed) || member != 0 || spell >= 7 {
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
    // Reflect's entry in the magic list first (the fourth shortcut reads that list), then the
    // second page's Drive row (`drive`).
    super::reflect::magic_menu(a, b);
    super::drive::fill(a, b);
    // In Limit Form the shortcuts are its limits (`limits::fill_kh1`, from `drive::fill`): Sora's
    // own (Reflect, the fourth) come back with KH1's next fill out of the form.
    if super::limits::shortcuts_ours() {
        return;
    }
    let base = base();
    let s = save();
    // (KH1 fills its shortcuts only with magic on here and a spell learned.)
    if s < 0x10000 || rd::<u8>(s + FLAGS_OFF) & 9 != 0 || rd::<i32>(base + MAGIC_COUNT_RVA) <= 0 {
        return;
    }
    let menu = base + MENU_RVA;
    // Reflect in KH1's three: KH1 saw its none there (entry 0x42, disabled); ours instead.
    let reflect = if REFL.load(Ordering::Relaxed) { super::reflect::shortcut_state(a, b) } else { None };
    let mut reflect_slots = 0u32;
    for k in 0..3usize {
        if get(k as i32) != REFLECT {
            continue;
        }
        let Some(usable) = reflect else { continue };
        reflect_slots |= 1 << k;
        wr(menu + 0x14 + k, super::reflect::COMMAND);
        for at in [menu + 4, menu + 8] {
            let v = rd::<u32>(at);
            wr(at, if usable { v & !(1 << k) } else { v | 1 << k });
        }
        wr(base + HAS_SHORTCUTS_RVA, 1u32);
    }
    if reflect_slots != 0 {
        static LOGGED: AtomicU32 = AtomicU32::new(u32::MAX);
        let key = reflect_slots | (reflect == Some(true)) as u32 * 8;
        if LOGGED.swap(key, Ordering::Relaxed) != key {
            log(&format!("Reflect in shortcut(s) {:?} in battle: command {:#x}, usable {}", (0..3).filter(|k| reflect_slots >> k & 1 != 0).map(|k| k + 1).collect::<Vec<_>>(), super::reflect::COMMAND, reflect == Some(true)));
        }
    }
    if !ON.load(Ordering::Acquire) {
        return;
    }
    let spell = get(3);
    let (entry, usable) = if spell == REFLECT && reflect.is_some() {
        wr(base + HAS_SHORTCUTS_RVA, 1u32);
        (super::reflect::COMMAND, reflect == Some(true))
    } else if (0..7).contains(&spell) {
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
    // The second page (D-pad left, `drive`).
    super::drive::pre_step();
    // A jump / cancel press backing out of a submenu is the menu's, not KH2's jump (`menujump`).
    super::menujump::pre_step();
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
