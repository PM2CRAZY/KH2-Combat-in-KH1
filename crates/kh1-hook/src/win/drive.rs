//! KH2's Drive in KH1's battle command menu (the user: "press left on the dpad to have
//! essentially a second command menu (like how it is in KH2), and have a drive menu at the
//! bottom of the second command menu"; for now Wisdom Form).
//!
//! Which page holds Drive, and the page flip itself, are `cmdmenu`'s (t752, the Command Menu rows:
//! KH2's Drive is on page 1, its Summon on page 2; `docs/kh1-kh2-command-menu.md`). Here: Drive's
//! row and list on whichever page shows it (`cmdmenu::drive_shown`).
//!
//! KH1's top-level command menu (menu 0, `0x140529600`, 0x38-byte menu structs) is rebuilt every
//! frame by `0x140292090`: Attack, Magic, Items, and a fourth row that is not one of its entries:
//! drawn (`0x1402769D0`) and run (the step `0x140291660`) from the magic list's last entry when
//! that is Summon (command 6, whose record opens the summon list, menu 3 `0x1405296A8`). Drive is
//! that command 6 named "Drive", with menu 3 holding the forms (`docs/kh1-drive.md`):
//! - Drive on page 1 (`cmdmenu::drive_in_row4`): after KH1's fill (`fill`) the magic list ends in
//!   Summon (appended when KH1 had none to append), its availability is Drive's: KH1's own row 4;
//! - Drive on page 2 (`cmdmenu::drive_in_page2`): command 6 is page 2's second row
//!   (`cmdmenu::page2_rows`), opening the same menu 3;
//! - menu 3 holds our form commands instead of the summons; command 6's name (word 6) reads
//!   "Drive" (`word`);
//! - a form's command record (`0x7E`.., a spell-type record with an argument KH1 has no spell
//!   for) arrives at `combat::command` as magic, argument `0x10 + form`, and comes to `chosen`.
//! KH1 rebuilds all of it each frame, so the other page is KH1's own again as soon as it flips.

use super::addr::Rva;
use super::{hook, log};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::OnceLock;

/// The forms (command record, word index, name).
/// (In `form::Kind::ALL`'s order, KH2's: Valor, then Wisdom. Wisdom kept its record and word; Valor
/// takes the next free ones, record 0x7F and word 0x1FD.)
pub const FORMS: [(u8, u16, &str); 2] = [(0x7F, 0x1FD, "Valor Form"), (0x7E, 0x1FE, "Wisdom Form")];
/// The record argument of form `k` (KH1's spells are 0..6, Reflect 7).
pub const FORM_ARG: u8 = 0x10;
/// Summon's command (its record opens the summon list) and the summon-is-out one.
const SUMMON: u8 = 6;
const SUMMON_OUT: u8 = 0x4F;

const MENU_RVA: Rva = Rva(0x529600);
const MAGIC_MENU_RVA: Rva = Rva(0x529638);
const SUMMON_MENU_RVA: Rva = Rva(0x5296A8);
const SUMMON_OUT_RVA: Rva = Rva(0x298D1A0);
const RECORD_RVA: Rva = Rva(0x28D7D0);
const SOUND_RVA: Rva = Rva(0x177F60);
/// KH1's battle button state (`+4` held, `+8` pressed, `+0x10` / `+0x14` repeats) and the
/// confirm button's logical bits (the button swap applied).
const BUTTONS_RVA: Rva = Rva(0x298D4A0);
const CONFIRM_MASK_RVA: Rva = Rva(0x546390);

static BASE: AtomicUsize = AtomicUsize::new(0);
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

/// The menu run is the Drive row (`combat::run_entry`): the magic list's last entry with Drive in
/// row 4, the top level's second entry with Drive on page 2.
pub unsafe fn is_drive_row(menu: usize, id: u8) -> bool {
    on() && id == SUMMON && (menu == base() + MAGIC_MENU_RVA && super::cmdmenu::drive_in_row4() || menu == base() + MENU_RVA && super::cmdmenu::drive_in_page2())
}

/// A form can be chosen now (page 2's Drive row's availability, `cmdmenu::page2_rows`).
pub unsafe fn any_usable(actor: usize) -> bool {
    on() && (0..FORMS.len()).any(|k| usable(actor, k))
}

/// In the form the Drive row is Revert itself (the user: "selecting return should cause the
/// drive form to return instead of opening a submenu"): taken only on a frame Sora can change
/// (`form::accept_now`), else nothing, kept for no later (the same as a spell). `true`: handled
/// (KH1 opens no list).
pub unsafe fn revert_row(actor: usize) -> bool {
    if !super::form::in_form() {
        return false;
    }
    if !super::form::accept_now() {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) % 30 == 0 {
            log!(Forms, "Revert not taken: Sora's move isn't at its end yet (no queue)");
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

/// Before KH1's command step (`shortcuts::step`, or ours when that isn't hooked): a form's confirm
/// press kept from KH1 until let go. (The page flip is `cmdmenu::pre_step`'s, t752.)
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
                    log!(Forms, "Drive: command record {id:#x} is in use ({have:02x?}): no Drive");
                }
                return false;
            }
            wr(r, want);
        }
    }
    true
}

/// Whether form `k` can be chosen now (`form`: its MP, or Revert in the form).
unsafe fn usable(actor: usize, k: usize) -> bool {
    super::form::Kind::ALL.get(k).is_some_and(|&kind| super::form::usable(actor, kind))
}

/// After KH1's battle-menu fill (and Reflect's entry): Drive's row 4 (Drive on page 1) and its
/// list (whichever page shows Drive).
pub unsafe fn fill(actor: usize, _flags: usize) {
    // (KH1's Counterattack command in Attack's place during KH2's guard repel, whatever the
    // Drive setting: `combomods::counter_entry`.)
    super::combomods::counter_entry(actor);
    let base = base();
    if !on() || base == 0 {
        return;
    }
    // In Valor Form the top's Magic row greyed and blocked, on both pages (the user: "when in
    // valor form magic command menu should be unselectable"). KH1 keeps the same bit (1) set
    // when Sora has no spell, so the list doesn't open; Drive's row 4 is drawn from the magic
    // list's own bit (`drive_row4`), page 2's from the forms (`cmdmenu`), so Revert stays usable.
    if super::form::in_valor() {
        let top = base + MENU_RVA;
        if rd::<u8>(top + 0x14 + 1) == 2 {
            for off in [4usize, 8] {
                wr(top + off, rd::<u32>(top + off) | 2);
            }
        }
    }
    if !super::cmdmenu::drive_shown() || !ensure_records(base) {
        return;
    }
    // (Page 2's Drive row is `cmdmenu::page2_rows`, a top-level entry: only the list here.)
    if super::cmdmenu::drive_in_row4() && !drive_row4(base, actor) {
        return;
    }
    forms_list(base, actor);
}

/// Drive as KH1's row 4: the magic list's hidden last entry (Summon's command, named "Drive").
/// `false`: no row 4 now (a summon out keeps KH1's own row 4, or the list is full).
unsafe fn drive_row4(base: usize, actor: usize) -> bool {
    if rd::<u32>(base + SUMMON_OUT_RVA) & 8 != 0 {
        return false;
    }
    let magic = base + MAGIC_MENU_RVA;
    let n = rd::<i32>(magic + 0x10);
    if !(0..=15).contains(&n) {
        return false;
    }
    let last = (n > 0).then(|| rd::<u8>(magic + 0x14 + n as usize - 1));
    if last == Some(SUMMON_OUT) {
        return false;
    }
    // The magic list ends in Summon, which row 4 shows (KH1 appends it once Sora has a summon).
    let n = if last == Some(SUMMON) {
        n
    } else {
        if n >= 15 || (n >= 8 && !super::reflect::eight_rows_pub(base)) {
            return false;
        }
        wr(magic + 0x14 + n as usize, SUMMON);
        wr(magic + 0x10, n + 1);
        n + 1
    };
    let any = (0..FORMS.len()).any(|k| usable(actor, k));
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
    true
}

/// Menu 3 (KH1's summon list) as the forms.
unsafe fn forms_list(base: usize, actor: usize) {
    let list = base + SUMMON_MENU_RVA;
    let mut grey = 0u32;
    for (k, &(id, _, _)) in FORMS.iter().enumerate() {
        wr(list + 0x14 + k, id);
        if !usable(actor, k) {
            grey |= 1 << k;
        }
    }
    wr(list + 0x10, FORMS.len() as i32);
    wr(list + 4, grey);
    wr(list + 8, grey);
    if rd::<i32>(list + 0xC) >= FORMS.len() as i32 {
        wr(list + 0xC, 0i32);
    }
}

/// A command's name by its word index: "Drive" for Summon's (6) on page 2, the forms' own.
/// `None`: KH1's (or Reflect's) word.
pub fn word(index: i32) -> Option<usize> {
    if !on() {
        return None;
    }
    let (drive, forms, revert) = NAMES.get()?;
    if index == SUMMON as i32 && super::cmdmenu::drive_shown() {
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
            log!(Forms, "Drive: {what} chosen");
        }
        Err(why) => {
            log!(Forms, "Drive: {name} refused ({why})");
            let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base() + SOUND_RVA);
            sound(4);
        }
    }
}

/// The command step when `shortcuts` hasn't hooked it.
static ORIG_STEP: AtomicUsize = AtomicUsize::new(0);
unsafe extern "system" fn step() -> u64 {
    pre_step();
    super::cmdmenu::pre_step();
    let f: unsafe extern "system" fn() -> u64 = std::mem::transmute(ORIG_STEP.load(Ordering::Acquire));
    let r = f();
    super::cmdmenu::post_step();
    r
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
/// Where the title sits in the header tab's rectangle (`0x1402CFB20` adds its own 0x10, 2, as for
/// a row's name in its panel).
const TITLE_DX: i32 = 0;
const TITLE_DY: i32 = 0;
static TITLE_TEXT: OnceLock<usize> = OnceLock::new();
static ORIG_LIST_DRAW: AtomicUsize = AtomicUsize::new(0);
/// The header tab's stand-in while the Drive list draws: its record with no size.
static BLANK_HEADER: OnceLock<usize> = OnceLock::new();

/// The Drive list is the summon list's contents now.
unsafe fn drive_list_shown(base: usize, menu: usize) -> bool {
    super::cmdmenu::drive_shown()
        && menu == base + SUMMON_MENU_RVA
        && rd::<u32>(base + SUMMON_OUT_RVA) & 8 == 0
        && rd::<u8>(menu + 0x14) == FORMS[0].0
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
            log!(Forms, "Drive list: summon layout not as expected (header {header:#x}, rows {rows:#x}, {n}): drawn as KH1's");
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
    // The title, as `0x1402787B0` draws a row's name: white, the list's alpha.
    if alpha != 0.0 {
        if let Some(&text) = TITLE_TEXT.get() {
            let a = (alpha * rd::<f32>(base + TEXT_ALPHA_RVA)).clamp(0.0, 255.0) as u8;
            let colour = u32::from_le_bytes([0x80, 0x80, 0x80, a]);
            let lx = x.wrapping_add(rd::<i32>(base + LIST_X_RVA)) + rect[0] + TITLE_DX;
            let ly = y + rect[1] + TITLE_DY;
            let tx = (lx.wrapping_add(0x710).wrapping_shl(4) >> 4) - 0x700;
            let ty = (ly.wrapping_add(0x742).wrapping_shl(4) >> 4) - 0x740;
            let draw: unsafe extern "system" fn(usize, i32, u32, i32, i32, usize) -> usize = std::mem::transmute(base + TEXT_RVA);
            let run: unsafe extern "system" fn(usize) = std::mem::transmute(base + TEXT_RUN_RVA);
            let start = rd::<u64>(base + PACKET_RVA) as usize;
            if start > 0x10000 {
                let end = draw(start, 1, colour, tx, ty, text);
                wr(base + PACKET_RVA, end as u64);
                run(start);
            }
        }
    }
    static LOGGED: AtomicBool = AtomicBool::new(false);
    if !LOGGED.swap(true, Ordering::Relaxed) {
        log!(Forms,
            "Drive list: title \"{TITLE}\" at header rect {rect:?} + list ({}, {y}); header tab hidden; {} panels whitened, {animated} animated (kept); colours before: {:?}",
            x.wrapping_add(rd::<i32>(base + LIST_X_RVA)),
            saved.len(),
            saved.iter().take(4).map(|(_, c)| [c[0], c[1], c[2], c[3]]).collect::<Vec<_>>()
        );
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
        log!(Forms, "Drive: off (drive = off)");
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
        log!(Forms, "Drive: the command step wasn't as expected: no second page");
        return;
    }
    if !super::shortcuts::fill_hooked()
        && !super::reflect::fill_hooked()
        && hook::install(base + super::shortcuts::FILL_RVA, &super::shortcuts::FILL_PROLOGUE, fill_hook as *const () as usize, &ORIG_FILL, "battle menus fill (Drive)").is_none()
    {
        log!(Forms, "Drive: the battle menu fill wasn't as expected: no second page");
        return;
    }
    if !super::reflect::word_hooked() && !hook::redirect_call(base + super::reflect::NAME_CALL_RVA, base + super::reflect::WORD_RVA, word_hook as *const () as usize, "command name word (Drive)") {
        log!(Forms, "Drive: the command name call wasn't as expected: no second page");
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
        log!(Forms, "Drive: the list draw wasn't as expected: the Drive list keeps KH1's summon title");
    }
    ON.store(true, Ordering::Release);
    static N: AtomicU32 = AtomicU32::new(0);
    N.fetch_add(1, Ordering::Relaxed);
    log!(Forms, "Drive: the Drive row and list ({}), on the page the Command Menu rows put it (`cmdmenu`)", FORMS.iter().map(|f| f.2).collect::<Vec<_>>().join(", "));
}
