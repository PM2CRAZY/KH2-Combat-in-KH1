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
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::OnceLock;

/// The forms (command record, word index, name).
pub const FORMS: [(u8, u16, &str); 1] = [(0x7E, 0x1FE, "Wisdom Form")];
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

static BASE: AtomicUsize = AtomicUsize::new(0);
/// The second page is up.
static PAGE2: AtomicBool = AtomicBool::new(false);
static ON: AtomicBool = AtomicBool::new(false);
static NAMES: OnceLock<(usize, Vec<usize>, usize)> = OnceLock::new();

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

/// Before KH1's command step (`shortcuts::step`, or ours when that isn't hooked): D-pad left at
/// the top level flips the page.
pub unsafe fn pre_step() {
    let base = base();
    if !on() || base == 0 {
        return;
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

/// Whether form `k` can be chosen now (`form`: its MP, or Revert in the form).
unsafe fn usable(actor: usize, _k: usize) -> bool {
    super::form::usable(actor)
}

/// After KH1's battle-menu fill (and Reflect's entry): page 2's Drive row and list.
pub unsafe fn fill(actor: usize, _flags: usize) {
    let base = base();
    if !on() || base == 0 {
        return;
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
    // The summon list as the forms.
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
    if index == SUMMON as i32 && PAGE2.load(Ordering::Relaxed) {
        return Some(*drive);
    }
    // In the form its entry is KH2's Revert.
    FORMS.iter().position(|f| f.1 as i32 == index).map(|k| if super::form::in_form() { *revert } else { forms[k] })
}

/// A form chosen (`combat::command`: magic with argument `FORM_ARG + k`): `form` changes him
/// (or reverts, in the form).
pub unsafe fn chosen(actor: usize, k: usize) {
    let name = FORMS.get(k).map_or("?", |f| f.2);
    match super::form::choose(actor) {
        Ok(what) => log(&format!("Drive: {what} chosen")),
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
    let f: unsafe extern "system" fn() -> u64 = std::mem::transmute(ORIG_STEP.load(Ordering::Acquire));
    f()
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
    ON.store(true, Ordering::Release);
    static N: AtomicU32 = AtomicU32::new(0);
    N.fetch_add(1, Ordering::Relaxed);
    log(&format!("Drive: D-pad left at the battle menu's top flips to page 2 (Attack, Magic, Items, Drive: {})", FORMS.iter().map(|f| f.2).collect::<Vec<_>>().join(", ")));
}
