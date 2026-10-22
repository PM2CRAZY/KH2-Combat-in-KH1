//! KH2's Drive in KH1's battle command menu (the user: "press left on the dpad to have
//! essentially a second command menu (like how it is in KH2), and have a drive menu at the
//! bottom of the second command menu"; for now Wisdom Form).
//!
//! KH1's top-level command menu (menu 0, `0x140529600`, 0x38-byte menu structs) is rebuilt every
//! frame by `0x140292090`: Attack, Magic, Items, and a fourth row that is not one of its entries:
//! drawn (`0x1402769D0`) and run (the step `0x140291660`) from the magic list's last entry when
//! that is Summon (command 6, whose record opens the summon list, menu 3 `0x1405296A8`). So the
//! second page is that same row turned into Drive (`docs/kh1-drive.md`):
//! - D-pad left (KH1's logical `0x80`, pressed `0x14298D4A8`) at the top level flips the page
//!   (`pre_step`, before KH1's step, which would otherwise move the cursor to Attack);
//! - on page 2, after KH1's fill (`fill`): the magic list ends in Summon (appended when KH1 had
//!   none to append), its availability is Drive's, and menu 3 holds our form commands instead
//!   of the summons; command 6's name (word 6) reads "Drive" (`word`);
//! - a form's command record (`0x7E`.., a spell-type record with an argument KH1 has no spell
//!   for) arrives at `combat::command` as magic, argument `0x10 + form`, and comes to `chosen`.
//! KH1 rebuilds all of it each frame, so page 1 is KH1's own again as soon as the page flips.

use super::addr::Rva;
use super::{hook, log};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::OnceLock;

/// The forms (command record, word index, name).
/// (In `form::Kind::ALL`'s order, KH2's: Valor, then Wisdom, then Final. Wisdom kept its record and
/// word; Valor takes the next free ones, record 0x7F and word 0x1FD; Final record 0x80 and word
/// 0x1FC (the user: "put it into the drive menu"; btltbl's records 0x7E.. are empty and none of
/// its records names a word above 0x173).)
/// (Limit after Wisdom, KH2's order: record 0x81 and word 0x1FB, the next free ones. The records'
/// arguments follow the list's order, so Final's is 0x13 now.)
/// (Master after Limit, before Final, KH2's order: record 0x87 and word 0x1F5, the next free ones
/// after Limit Form's limits (records 0x82..0x86, words 0x1FA..0x1F6). Final's argument is 0x14
/// now.)
pub const FORMS: [(u8, u16, &str); 5] =
    [(0x7F, 0x1FD, "Valor Form"), (0x7E, 0x1FE, "Wisdom Form"), (0x81, 0x1FB, "Limit Form"), (0x87, 0x1F5, "Master Form"), (0x80, 0x1FC, "Final Form")];
/// The record argument of form `k` (KH1's spells are 0..6, Reflect 7).
pub const FORM_ARG: u8 = 0x10;
/// Summon's command (its record opens the summon list) and the summon-is-out one.
const SUMMON: u8 = 6;
const SUMMON_OUT: u8 = 0x4F;

const MENU_RVA: Rva = Rva(0x529600);
const MAGIC_MENU_RVA: Rva = Rva(0x529638);
const SUMMON_MENU_RVA: Rva = Rva(0x5296A8);
const ROOT_RVA: Rva = Rva(0x2D543F0);
const LEVEL_RVA: Rva = Rva(0x2D54450);
const PRESSED_RVA: Rva = Rva(0x298D4A8);
const SUMMON_OUT_RVA: Rva = Rva(0x298D1A0);
const RECORD_RVA: Rva = Rva(0x28D7D0);
const SOUND_RVA: Rva = Rva(0x177F60);
const DPAD_LEFT: u32 = 0x80;
/// KH1's battle button state (`+4` held, `+8` pressed, `+0x10` / `+0x14` repeats) and the
/// confirm button's logical bits (the button swap applied).
const BUTTONS_RVA: Rva = Rva(0x298D4A0);
const CONFIRM_MASK_RVA: Rva = Rva(0x546390);

static BASE: AtomicUsize = AtomicUsize::new(0);
/// The second page is up.
static PAGE2: AtomicBool = AtomicBool::new(false);
static ON: AtomicBool = AtomicBool::new(false);
static NAMES: OnceLock<(usize, Vec<usize>, usize)> = OnceLock::new();
/// A form or Revert was just chosen: confirm is KH1's again only once let go (the user: still
/// holding it after a revert opened the Drive list again).
static SWALLOW_CONFIRM: AtomicBool = AtomicBool::new(false);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}
fn base() -> usize {
    BASE.load(Ordering::Relaxed)
}

pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// The second page is up (the summon list is the Drive list).
pub fn page2() -> bool {
    on() && PAGE2.load(Ordering::Relaxed)
}

/// The command step is hooked here (`shortcuts` didn't).
pub fn step_hooked() -> bool {
    ORIG_STEP.load(Ordering::Acquire) != 0
}

/// The menu run is the Drive row (`combat::run_entry`: the magic list, its last entry, on page 2).
pub unsafe fn is_drive_row(menu: usize, id: u8) -> bool {
    on() && PAGE2.load(Ordering::Relaxed) && menu == base() + MAGIC_MENU_RVA && id == SUMMON
}

/// In the form the Drive row is Revert itself (the user: "selecting return should cause the
/// drive form to return instead of opening a submenu"): taken only on a frame Sora can change
/// (`form::accept_now`), else nothing, kept for no later (the same as a spell). `true`: handled
/// (KH1 opens no list).
pub unsafe fn revert_row(actor: usize) -> bool {
    if !super::form::in_form() {
        return false;
    }
    // Anti Form: no Revert (the user: "I dont want sora to be able to revert out of it"); the row
    // is greyed (`fill`), and refused with KH1's "can't" if run all the same.
    if super::form::in_anti() {
        let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base() + SOUND_RVA);
        sound(4);
        static A: AtomicU32 = AtomicU32::new(0);
        if A.fetch_add(1, Ordering::Relaxed) % 30 == 0 {
            log("Revert refused: Anti Form has none");
        }
        return true;
    }
    if !super::form::accept_now() {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) % 30 == 0 {
            log("Revert not taken: Sora's move isn't at its end yet (no queue)");
        }
        return true;
    }
    chosen(actor, 0);
    // The cursor back on Attack (the user: "when pressing return ... it needs to return to the
    // 1st slot (attack)"): the top menu's cursor (`+0xC`; KH1's confirm on the fourth row
    // checks it is 3).
    if !super::form::in_form() || super::form::busy() {
        wr(base() + MENU_RVA + 0xC, 0i32);
    }
    true
}

/// Before KH1's command step (`shortcuts::step`, or ours when that isn't hooked): D-pad left at
/// the top level flips the page.
pub unsafe fn pre_step() {
    let base = base();
    if !on() || base == 0 {
        return;
    }
    if SWALLOW_CONFIRM.load(Ordering::Relaxed) {
        let confirm = rd::<u32>(base + CONFIRM_MASK_RVA);
        let b = base + BUTTONS_RVA;
        if confirm == 0 || rd::<u32>(b + 4) & confirm == 0 {
            SWALLOW_CONFIRM.store(false, Ordering::Relaxed);
        } else {
            for off in [8usize, 0x10, 0x14] {
                wr(b + off, rd::<u32>(b + off) & !confirm);
            }
        }
    }
    let pressed = base + PRESSED_RVA;
    if rd::<u32>(pressed) & DPAD_LEFT == 0 {
        return;
    }
    // The top level of the battle menu (not the L1 shortcut menu, not a submenu), no summon out.
    let root = rd::<u64>(base + ROOT_RVA) as usize;
    let level = rd::<u64>(base + LEVEL_RVA) as usize;
    if root != base + MENU_RVA || level != base + ROOT_RVA || rd::<u32>(base + SUMMON_OUT_RVA) & 8 != 0 {
        return;
    }
    wr(pressed, rd::<u32>(pressed) & !DPAD_LEFT);
    let now = !PAGE2.load(Ordering::Relaxed);
    PAGE2.store(now, Ordering::Relaxed);
    // The summon list's cursor back to its top (the list changes under it).
    wr(base + SUMMON_MENU_RVA + 0xC, 0i32);
    let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
    sound(1);
    log(&format!("command menu: page {}", if now { "2 (Attack, Magic, Items, Drive)" } else { "1" }));
}

/// Our records for the forms (a self-cast spell record, as Reflect's), written over empty ones.
unsafe fn ensure_records(base: usize) -> bool {
    let record: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base + RECORD_RVA);
    for (k, &(id, word, _)) in FORMS.iter().enumerate() {
        let r = record(id as u32);
        if r < 0x10000 {
            return false;
        }
        let want: [u8; 16] = [0x02, 0x00, 0xFF, 0x00, word as u8, (word >> 8) as u8, 0x02, FORM_ARG + k as u8, 0x00, 0xFF, 0x01, 0x02, 0, 0, 0, 0];
        let have = rd::<[u8; 16]>(r);
        if have != want {
            if have != [0u8; 16] && have[4..8] != want[4..8] {
                static ONCE: AtomicBool = AtomicBool::new(false);
                if !ONCE.swap(true, Ordering::Relaxed) {
                    log(&format!("Drive: command record {id:#x} is in use ({have:02x?}): no Drive"));
                }
                return false;
            }
            wr(r, want);
        }
    }
    true
}

/// Whether form `k` can be chosen now (`form::usable`: its Drive bars, `gauge::can_pay`, its
/// partners and the rest; or Revert in the form). A form the gauge is short of is greyed.
unsafe fn usable(actor: usize, k: usize) -> bool {
    super::form::Kind::ALL.get(k).is_some_and(|&kind| super::form::usable(actor, kind))
}

/// After KH1's battle-menu fill (and Reflect's entry): page 2's Drive row and list.
pub unsafe fn fill(actor: usize, _flags: usize) {
    let base = base();
    if !on() || base == 0 {
        return;
    }
    // In Valor Form the top's Magic row greyed and blocked, on both pages (the user: "when in
    // valor form magic command menu should be unselectable"). KH1 keeps the same bit (1) set
    // when Sora has no spell, so the list doesn't open; page 2's Drive row is drawn from the
    // magic list's own bit (`fill` below), so Revert stays usable.
    // (Limit Form: KH2's Limit can't cast either; its Limit commands take Magic's place there,
    // `limits::fill`. With that menu off for now, `limits::KH2_LIMITS`, Magic is greyed here first
    // as in Valor, and `limits::fill` opens it again as the Limit menu of KH1's limits; if that
    // can't be set up, it stays greyed.)
    if super::form::in_valor() || super::form::in_limit() && !super::limits::KH2_LIMITS {
        let top = base + MENU_RVA;
        if rd::<u8>(top + 0x14 + 1) == 2 {
            for off in [4usize, 8] {
                wr(top + off, rd::<u32>(top + off) | 2);
            }
        }
    }
    super::limits::fill(actor);
    // In Anti Form, Magic and Items greyed and blocked too, and Summon (the magic list's last
    // entry, the fourth row on page 1): KH2's Anti uses none of them, as Valor has no magic.
    // Page 2's Drive row is greyed below (no form `usable`: no Revert in Anti).
    if super::form::in_anti() {
        let top = base + MENU_RVA;
        for (i, id) in [(1usize, 2u8), (2, 3)] {
            if rd::<u8>(top + 0x14 + i) == id {
                for off in [4usize, 8] {
                    wr(top + off, rd::<u32>(top + off) | 1 << i);
                }
            }
        }
        let magic = base + MAGIC_MENU_RVA;
        let n = rd::<i32>(magic + 0x10);
        if (1..=15).contains(&n) && matches!(rd::<u8>(magic + 0x14 + n as usize - 1), SUMMON | SUMMON_OUT) {
            for off in [4usize, 8] {
                wr(magic + off, rd::<u32>(magic + off) | 1 << (n - 1));
            }
        }
    }
    if !PAGE2.load(Ordering::Relaxed) || rd::<u32>(base + SUMMON_OUT_RVA) & 8 != 0 || !ensure_records(base) {
        return;
    }
    let magic = base + MAGIC_MENU_RVA;
    let n = rd::<i32>(magic + 0x10);
    if !(0..=15).contains(&n) {
        return;
    }
    let last = (n > 0).then(|| rd::<u8>(magic + 0x14 + n as usize - 1));
    if last == Some(SUMMON_OUT) {
        return;
    }
    // The magic list ends in Summon, which row 4 shows (KH1 appends it once Sora has a summon).
    let n = if last == Some(SUMMON) {
        n
    } else {
        if n >= 15 || (n >= 8 && !super::reflect::eight_rows_pub(base)) {
            return;
        }
        wr(magic + 0x14 + n as usize, SUMMON);
        wr(magic + 0x10, n + 1);
        n + 1
    };
    // (Only the forms Sora has unlocked, `form::unlocked`; in a form, its Revert rows.)
    let shown: Vec<usize> = (0..FORMS.len()).filter(|&k| super::form::in_form() || super::form::Kind::ALL.get(k).is_some_and(|&kind| super::form::unlocked(kind))).collect();
    let any = shown.iter().any(|&k| usable(actor, k));
    let bit = 1u32 << (n - 1);
    for off in [4usize, 8] {
        let v = rd::<u32>(magic + off);
        wr(magic + off, if any { v & !bit } else { v | bit });
    }
    // Row 4 is drawn as the magic list's last entry only while the top's Magic entry is there:
    // with no spell yet, it is put there, kept unusable.
    let top = base + MENU_RVA;
    if rd::<u8>(top + 0x14 + 1) != 2 {
        wr(top + 0x14 + 1, 2u8);
        for off in [4usize, 8] {
            wr(top + off, rd::<u32>(top + off) | 2);
        }
    }
    // The summon list as the forms.
    let list = base + SUMMON_MENU_RVA;
    let mut grey = 0u32;
    for (row, &k) in shown.iter().enumerate() {
        wr(list + 0x14 + row, FORMS[k].0);
        if !usable(actor, k) {
            grey |= 1 << row;
        }
    }
    wr(list + 0x10, shown.len() as i32);
    wr(list + 4, grey);
    wr(list + 8, grey);
    if rd::<i32>(list + 0xC) >= shown.len() as i32 {
        wr(list + 0xC, 0i32);
    }
}

/// A command's name by its word index: "Drive" for Summon's (6) on page 2, the forms' own.
/// `None`: KH1's (or Reflect's) word.
pub fn word(index: i32) -> Option<usize> {
    if !on() {
        return None;
    }
    // (Limit Form's "Limit", its limits and the follow-up on Triangle first.)
    if let Some(p) = super::limits::word(index) {
        return Some(p);
    }
    let (drive, forms, revert) = NAMES.get()?;
    if index == SUMMON as i32 && PAGE2.load(Ordering::Relaxed) {
        // (In the form the row is KH2's Revert.)
        return Some(if super::form::in_form() { *revert } else { *drive });
    }
    // In the form its entry is KH2's Revert.
    FORMS.iter().position(|f| f.1 as i32 == index).map(|k| if super::form::in_form() { *revert } else { forms[k] })
}

/// A form chosen (`combat::command`: magic with argument `FORM_ARG + k`): `form` changes him
/// (or reverts, in the form).
pub unsafe fn chosen(actor: usize, k: usize) {
    let name = FORMS.get(k).map_or("?", |f| f.2);
    let Some(&kind) = super::form::Kind::ALL.get(k) else { return };
    match super::form::choose(actor, kind) {
        Ok(what) => {
            SWALLOW_CONFIRM.store(true, Ordering::Relaxed);
            log(&format!("Drive: {what} chosen"));
        }
        Err(why) => {
            log(&format!("Drive: {name} refused ({why})"));
            let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base() + SOUND_RVA);
            sound(4);
        }
    }
}

/// The command step when `shortcuts` hasn't hooked it.
static ORIG_STEP: AtomicUsize = AtomicUsize::new(0);
unsafe extern "system" fn step() -> u64 {
    pre_step();
    super::menujump::pre_step();
    let f: unsafe extern "system" fn() -> u64 = std::mem::transmute(ORIG_STEP.load(Ordering::Acquire));
    f()
}

/// KH1's list draw (`0x140277F40 (kind, menu, _, _, layout, x, y, alpha)`, called by
/// `0x140279700` per open list: kind 3 the summon list with its layout `0x14285A260`); it sets
/// the text packet pointer, then draws through `0x1402769D0`: the layout's background panels
/// (`+0x18`, `+0x50`), its header tab (`+0x20`, a sprite record whose texture carries the word
/// "SUMMON"), its row panels (`+0x28`, `+0x10` of them, 0xC0 bytes each; `+0x58` a second
/// set or none) and the rows' names.
const LIST_DRAW_RVA: Rva = Rva(0x277F40);
/// `mov [rsp+8], rbx; mov [rsp+0x10], rbp` (both executables).
const LIST_DRAW_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];
const SUMMON_LAYOUT_RVA: Rva = Rva(0x285A260);
/// Added to the list's x by `0x140277F40`.
const LIST_X_RVA: Rva = Rva(0x232E6CC);
/// A sprite record's rectangle (`0x14027C2C0 (record, &[x0, y0, x1, y1])`, its animated one too).
const RECT_RVA: Rva = Rva(0x27C2C0);
/// The text packet: `0x1402CFB20 (packet, 1, rgba, x, y, text) -> packet`, then
/// `0x1403028F0 (start)` runs it when the list is shown (`0x1402787B0`, a row's name, does the same).
const TEXT_RVA: Rva = Rva(0x2CFB20);
const TEXT_RUN_RVA: Rva = Rva(0x3028F0);
const PACKET_RVA: Rva = Rva(0x2853218);
/// `0x1402787B0`'s alpha scale (the list's alpha times this).
const TEXT_ALPHA_RVA: Rva = Rva(0x3B31F8);
/// The Drive list's title (the user: "the text in the sub menu says "SUMMON" when it needs to say
/// "DRIVES" lets change the color of that sub menu as well to be white").
const TITLE: &str = "DRIVES";
/// Nudges for the title, in screen pixels, after it is centred on the header word's rectangle.
/// (That word, the header record `+0x20`, is a 65x8 sprite KH1 draws at the list's x / y plus its
/// rectangle `[x, y, w, h]` (`0x140278880`; vertices `(x + 0x700) << 4`, `(y + 0x740) << 4`), the
/// tab behind it is the background pair; the rows start at y + 0x14, 0x16 apart.)
const TITLE_DX: i32 = 0;
const TITLE_DY: i32 = 0;
/// The font's height (14; 18 in some languages) and a string's width in pixels at a height
/// (`0x1402CEC60 (text, height)`, what `0x1402CFEA0` centres and right-aligns with).
const FONT_H_RVA: Rva = Rva(0x54603C);
const TEXT_WIDTH_RVA: Rva = Rva(0x2CEC60);
/// The row count the title was last logged for.
static TITLE_LOGGED_ROWS: AtomicI32 = AtomicI32::new(-1);
static TITLE_TEXT: OnceLock<usize> = OnceLock::new();
/// The costs' digits, and how far in from a row's right end they sit.
static COST_TEXT: OnceLock<Vec<usize>> = OnceLock::new();
const COST_DX: i32 = 5;
static ORIG_LIST_DRAW: AtomicUsize = AtomicUsize::new(0);
/// The header tab's stand-in while the Drive list draws: its record with no size.
static BLANK_HEADER: OnceLock<usize> = OnceLock::new();

/// The Drive list is the summon list's contents now.
unsafe fn drive_list_shown(base: usize, menu: usize) -> bool {
    PAGE2.load(Ordering::Relaxed)
        && menu == base + SUMMON_MENU_RVA
        && rd::<u32>(base + SUMMON_OUT_RVA) & 8 == 0
        // (Its first row is any form's: only the unlocked ones are listed, so it isn't always
        // Valor's.)
        && FORMS.iter().any(|f| f.0 == rd::<u8>(menu + 0x14))
}

/// A sprite record's colour without its hue: each vertex's red, green and blue as their
/// brightest (`0x14027C660`: `+0x20` r, g, b, a ints, four vertices at 0x10 apart when `+0x60`
/// has 0x20, else the first for all; with 0xA or 0xC0 there the colour is animated, not these).
/// The old colour is pushed to `saved` to be put back. `false`: animated, left as it is.
unsafe fn whiten(rec: usize, saved: &mut Vec<(usize, [i32; 16])>) -> bool {
    if rec < 0x10000 {
        return true;
    }
    let flags = rd::<u32>(rec + 0x60);
    if flags & 0xA == 0xA || flags & 0xC0 == 0xC0 {
        return false;
    }
    let old = rd::<[i32; 16]>(rec + 0x20);
    saved.push((rec, old));
    let n = if flags & 0x20 != 0 { 4 } else { 1 };
    for v in 0..n {
        let p = rec + 0x20 + v * 0x10;
        let m = rd::<i32>(p).max(rd::<i32>(p + 4)).max(rd::<i32>(p + 8));
        for c in 0..3 {
            wr(p + c * 4, m);
        }
    }
    true
}

unsafe extern "system" fn list_draw(kind: i32, menu: usize, a3: i32, a4: i32, layout: usize, x: i32, y: i32, alpha: f32) -> u64 {
    let f: unsafe extern "system" fn(i32, usize, i32, i32, usize, i32, i32, f32) -> u64 = std::mem::transmute(ORIG_LIST_DRAW.load(Ordering::Acquire));
    let base = base();
    if kind != 3 || !on() || base == 0 || layout != base + SUMMON_LAYOUT_RVA || !drive_list_shown(base, menu) {
        return f(kind, menu, a3, a4, layout, x, y, alpha);
    }
    let header = rd::<u64>(layout + 0x20) as usize;
    let rows = rd::<u64>(layout + 0x28) as usize;
    let rows2 = rd::<u64>(layout + 0x58) as usize;
    let n = rd::<i32>(layout + 0x10);
    if header < 0x10000 || rows < 0x10000 || !(1..=16).contains(&n) {
        static BAD: AtomicBool = AtomicBool::new(false);
        if !BAD.swap(true, Ordering::Relaxed) {
            log(&format!("Drive list: summon layout not as expected (header {header:#x}, rows {rows:#x}, {n}): drawn as KH1's"));
        }
        return f(kind, menu, a3, a4, layout, x, y, alpha);
    }
    // The header's place, for the title.
    let rect_of: unsafe extern "system" fn(usize, *mut [i32; 4]) = std::mem::transmute(base + RECT_RVA);
    let mut rect = [0i32; 4];
    rect_of(header, &mut rect);
    // White: every panel's colour without its hue, for this draw only.
    let mut saved = Vec::with_capacity(2 * n as usize + 2);
    let mut animated = 0;
    for rec in [rd::<u64>(layout + 0x18) as usize, rd::<u64>(layout + 0x50) as usize] {
        animated += !whiten(rec, &mut saved) as u32;
    }
    for k in 0..n as usize {
        animated += !whiten(rows + k * 0xC0, &mut saved) as u32;
        if rows2 > 0x10000 {
            animated += !whiten(rows2 + k * 0xC0, &mut saved) as u32;
        }
    }
    // The tab with "SUMMON" on it: a copy with no size (drawn as nothing), its colour clear.
    let blank = *BLANK_HEADER.get_or_init(|| {
        let b = Box::leak(vec![0u8; 0xC0 + 16].into_boxed_slice()).as_ptr() as usize;
        (b + 15) & !15
    });
    std::ptr::copy_nonoverlapping(header as *const u8, blank as *mut u8, 0xC0);
    wr(blank, [0i32; 4]);
    wr(blank + 0x60, rd::<u32>(blank + 0x60) & !(0x10 | 0x80 | 0x08));
    for v in 0..4 {
        wr(blank + 0x2C + v * 0x10, 0i32);
    }
    wr(layout + 0x20, blank as u64);
    let r = f(kind, menu, a3, a4, layout, x, y, alpha);
    wr(layout + 0x20, header as u64);
    for (rec, old) in saved.iter().rev() {
        wr(rec + 0x20, *old);
    }
    // The title where KH1 draws the header word: centred on its rectangle, both ways. (Before, it
    // went where a row's name goes in its panel, the rectangle's corner + 0x10, 2: low, under the
    // first row's top, and right of where the word sits.)
    // KH1 draws the word at the list's x (+ [LIST_X]) / y plus its rectangle; `0x1402CFB20` puts
    // the text's top at y - height / 2 + 7 (in KH1's other languages than 0 and 7), so its middle
    // at y + 7, whatever the font's height.
    let lx = x.wrapping_add(rd::<i32>(base + LIST_X_RVA));
    let height = rd::<i32>(base + FONT_H_RVA);
    let text = TITLE_TEXT.get().copied().unwrap_or(0);
    let width = if text != 0 && (8..=32).contains(&height) {
        let measure: unsafe extern "system" fn(usize, i32) -> i32 = std::mem::transmute(base + TEXT_WIDTH_RVA);
        measure(text, height)
    } else {
        0
    };
    let tx = lx + rect[0] + (rect[2] - width) / 2 + TITLE_DX;
    let ty = y + rect[1] + rect[3] / 2 - 7 + TITLE_DY;
    // White as KH1's own available rows' names are (`0x1402787B0`'s brightness, a byte per row at
    // the menu's `+0x24`; 0x80 is the grey it was), with the list's alpha.
    let count = rd::<i32>(menu + 0x10).clamp(0, 16);
    let bright = (0..count as usize).map(|k| rd::<u8>(menu + 0x24 + k)).max().unwrap_or(0).max(0x80);
    if alpha != 0.0 && text != 0 {
        let a = (alpha * rd::<f32>(base + TEXT_ALPHA_RVA)).clamp(0.0, 255.0) as u8;
        let colour = u32::from_le_bytes([bright, bright, bright, a]);
        let draw: unsafe extern "system" fn(usize, i32, u32, i32, i32, usize) -> usize = std::mem::transmute(base + TEXT_RVA);
        let run: unsafe extern "system" fn(usize) = std::mem::transmute(base + TEXT_RUN_RVA);
        let start = rd::<u64>(base + PACKET_RVA) as usize;
        if start > 0x10000 {
            let end = draw(start, 1, colour, tx, ty, text);
            wr(base + PACKET_RVA, end as u64);
            run(start);
        }
    }
    // Each row's cost in Drive bars at its right end, as KH2's Drive list shows it (not in a form,
    // where the rows read Revert), in the row's own brightness (greyed with a row the gauge is
    // short of).
    if alpha != 0.0 && !super::form::in_form() {
        // (The row's right end: its second panel's, the list's right column, if it has one.)
        let mut r = [0i32; 4];
        rect_of(if rows2 > 0x10000 { rows2 } else { rows }, &mut r);
        let draw: unsafe extern "system" fn(usize, i32, u32, i32, i32, usize) -> usize = std::mem::transmute(base + TEXT_RVA);
        let run: unsafe extern "system" fn(usize) = std::mem::transmute(base + TEXT_RUN_RVA);
        let measure: unsafe extern "system" fn(usize, i32) -> i32 = std::mem::transmute(base + TEXT_WIDTH_RVA);
        let a = (alpha * rd::<f32>(base + TEXT_ALPHA_RVA)).clamp(0.0, 255.0) as u8;
        let costs = COST_TEXT.get_or_init(|| (0..10).map(|d| Box::leak(super::menu::encode(&d.to_string()).into_boxed_slice()).as_ptr() as usize).collect());
        for row in 0..count as usize {
            let id = rd::<u8>(menu + 0x14 + row);
            let Some(k) = FORMS.iter().position(|f| f.0 == id) else { continue };
            let Some(&kind) = super::form::Kind::ALL.get(k) else { continue };
            let word = costs[(super::gauge::cost(kind) as usize).min(9)];
            let w = if (8..=32).contains(&height) { measure(word, height) } else { 0 };
            let b = rd::<u8>(menu + 0x24 + row);
            let cx = lx + r[0] + r[2] - w - COST_DX;
            let cy = y + 0x14 + r[1] + row as i32 * 0x16 + r[3] / 2 - 7;
            let start = rd::<u64>(base + PACKET_RVA) as usize;
            if start > 0x10000 {
                let end = draw(start, 1, u32::from_le_bytes([b, b, b, a]), cx, cy, word);
                wr(base + PACKET_RVA, end as u64);
                run(start);
            }
        }
    }
    // Each time the row count changes (the list's y moves 0x16 a row): where everything went.
    if TITLE_LOGGED_ROWS.swap(count, Ordering::Relaxed) != count {
        let boxed = |rec: usize, dy: i32| -> String {
            if rec < 0x10000 {
                return "none".into();
            }
            let mut r = [0i32; 4];
            rect_of(rec, &mut r);
            format!("({}, {}) {}x{}", lx + r[0], y + dy + r[1], r[2], r[3])
        };
        log(&format!(
            "Drive list: menu count {count} ({} rows drawn), list at ({lx}, {y}); header word (hidden) rect {rect:?} = ({}, {}) {}x{}; tab / back panels {} and {}; first row panel {}; title \"{TITLE}\" {width}x{height} at ({tx}, {}) (text call ({tx}, {ty})), middle ({}, {}), brightness {bright:#x}; {} panels whitened, {animated} animated (kept); colours before: {:?}",
            count - 1,
            lx + rect[0],
            y + rect[1],
            rect[2],
            rect[3],
            boxed(rd::<u64>(layout + 0x18) as usize, 0),
            boxed(rd::<u64>(layout + 0x50) as usize, 0),
            boxed(rows, 0x14),
            ty - height / 2 + 7,
            tx + width / 2,
            ty + 7,
            saved.len(),
            saved.iter().take(4).map(|(_, c)| [c[0], c[1], c[2], c[3]]).collect::<Vec<_>>()
        ));
    }
    r
}

/// The battle-menu fill when neither `shortcuts` nor `reflect` hooked it.
static ORIG_FILL: AtomicUsize = AtomicUsize::new(0);
unsafe extern "system" fn fill_hook(a: usize, b: usize) {
    let f: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_FILL.load(Ordering::Acquire));
    f(a, b);
    fill(a, b);
}

/// KH1's word lookup when `reflect` didn't redirect it.
static ORIG_WORD: AtomicUsize = AtomicUsize::new(0);
unsafe extern "system" fn word_hook(index: i32) -> usize {
    if let Some(p) = word(index) {
        return p;
    }
    let f: unsafe extern "system" fn(i32) -> usize = std::mem::transmute(ORIG_WORD.load(Ordering::Acquire));
    f(index)
}

/// After `shortcuts::install` and `reflect::install`: the hooks either didn't put in place.
pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    if !super::settings::get().drive {
        log("Drive: off (drive = off)");
        return;
    }
    let _ = NAMES.set((
        Box::leak(super::menu::encode("Drive").into_boxed_slice()).as_ptr() as usize,
        FORMS.iter().map(|f| Box::leak(super::menu::encode(f.2).into_boxed_slice()).as_ptr() as usize).collect(),
        Box::leak(super::menu::encode("Revert").into_boxed_slice()).as_ptr() as usize,
    ));
    if !super::shortcuts::step_hooked()
        && hook::install(base + super::shortcuts::STEP_RVA, &super::shortcuts::STEP_PROLOGUE, step as *const () as usize, &ORIG_STEP, "command step (Drive)").is_none()
    {
        log("Drive: the command step wasn't as expected: no second page");
        return;
    }
    if !super::shortcuts::fill_hooked()
        && !super::reflect::fill_hooked()
        && hook::install(base + super::shortcuts::FILL_RVA, &super::shortcuts::FILL_PROLOGUE, fill_hook as *const () as usize, &ORIG_FILL, "battle menus fill (Drive)").is_none()
    {
        log("Drive: the battle menu fill wasn't as expected: no second page");
        return;
    }
    if !super::reflect::word_hooked() && !hook::redirect_call(base + super::reflect::NAME_CALL_RVA, base + super::reflect::WORD_RVA, word_hook as *const () as usize, "command name word (Drive)") {
        log("Drive: the command name call wasn't as expected: no second page");
        return;
    }
    if super::reflect::word_hooked() {
        // (Reflect's word asks ours first.)
    } else {
        ORIG_WORD.store(base + super::reflect::WORD_RVA, Ordering::Release);
    }
    // The Drive list's title and colour (only a look: without it the list reads "SUMMON").
    let _ = TITLE_TEXT.set(Box::leak(super::menu::encode(TITLE).into_boxed_slice()).as_ptr() as usize);
    if hook::install(base + LIST_DRAW_RVA, &LIST_DRAW_PROLOGUE, list_draw as *const () as usize, &ORIG_LIST_DRAW, "list draw (Drive title)").is_none() {
        log("Drive: the list draw wasn't as expected: the Drive list keeps KH1's summon title");
    }
    ON.store(true, Ordering::Release);
    static N: AtomicU32 = AtomicU32::new(0);
    N.fetch_add(1, Ordering::Relaxed);
    log(&format!("Drive: D-pad left at the battle menu's top flips to page 2 (Attack, Magic, Items, Drive: {})", FORMS.iter().map(|f| f.2).collect::<Vec<_>>().join(", ")));
}
