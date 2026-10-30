//! Which Drive Forms Sora owns, kept in KH1's save (the owner, the main-branch port, round 1:
//! "go ahead"; `docs/main-port-inventory.md` round 1, `docs/kh1-drive-forms-native.md` §12).
//!
//! - **KH2** (Checked (decomp), Steam KH2): owning a form is owning its form item. The form's item
//!   is `0x1403C3EF0(form)` (a table at `0x1405C5D08`), owned is `0x1403C4720`. The Drive submenu
//!   (`0x1403F3110`) lists only owned forms, and the form level cap is 2 + the forms owned among
//!   Valor, Wisdom, Limit, Master and Final (`0x1403E8380`, `0x1403E88B0`).
//! - **KH1** has no forms, so the owned forms are one byte of Sora's save: `save + 0x881`, the
//!   first of the seven bytes after KH1's bool at `+0x880` (no Steam instruction reads or writes
//!   `+0x881..+0x887`, Checked (exe), `formexp.rs`; nothing else of ours uses `+0x881`, Checked
//!   (code)). It travels with the save as the form levels at `+0x882..` do. Bits:
//!   `formlevel::owned` (KH2's form ids - 1) for the forms given in this save, bit 7 "set up".
//! - **Set up once a save** (`check`, from `menu::Growth::update`'s once-a-second pass): a save
//!   whose byte isn't set up gets bit 7, and Final's bit if its form record is set (it has been
//!   entered), with no box.
//! - **What Sora owns** (`owned`): with Progression > Story Unlocks off (the default, t926; the
//!   owner's answer 1 (a)) Valor and Wisdom, the forms testing has always given, and Final once
//!   entered; on, the forms given in the save: the owner's table (`crate::storyforms`) and Final
//!   once entered.
//! - **The table** (Story Unlocks on, `check`): the forms its fights open are given in the save.
//!   The first pass on a save (bit 6 not set yet) or right after the row is switched on gives them
//!   silently (the owner's answer 6 (a)); a form given later is a gain during play (logged; its box
//!   comes with the "New Drive Form" version). Forms not built yet (Limit, Master) are stored and
//!   wait (answer 4 (a)).

use super::{form::Kind, log};
use crate::formlevel::{self, owned};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// The owned forms' byte in Sora's save.
pub const OWNED_OFF: usize = 0x881;
/// The byte's bit 6: the table has been read for this save once (its forms then given silently).
const STORY_READ: u8 = 0x40;

/// Progression > Story Unlocks: 0 not read yet, 1 off, 2 on.
static STORY: AtomicU8 = AtomicU8::new(0);
/// Story Unlocks was just switched on: the next pass gives the table's forms silently.
static QUIET: AtomicBool = AtomicBool::new(false);

/// Progression > Story Unlocks (the ini's `story_unlocks`, off by default).
pub fn story_unlocks() -> bool {
    match STORY.load(Ordering::Relaxed) {
        0 => {
            let on = super::settings::get().story_unlocks;
            STORY.store(1 + on as u8, Ordering::Relaxed);
            on
        }
        v => v == 2,
    }
}

/// Switches Story Unlocks (the Config menu), kept in the ini.
pub fn set_story_unlocks(on: bool) {
    STORY.store(1 + on as u8, Ordering::Relaxed);
    if on {
        QUIET.store(true, Ordering::Relaxed);
    }
    super::settings::save("story_unlocks", if on { "on" } else { "off" });
    log!(Forms, "Config menu: Story Unlocks {}", if on { "On" } else { "Off" });
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

/// KH2's form id of `kind`.
pub fn form_id(kind: Kind) -> u8 {
    match kind {
        Kind::Valor => formlevel::form::VALOR,
        Kind::Wisdom => formlevel::form::WISDOM,
        Kind::Limit => formlevel::form::LIMIT,
        Kind::Final => formlevel::form::FINAL,
    }
}

unsafe fn save() -> Option<usize> {
    let base = super::takeover::game_base();
    if base == 0 {
        return None;
    }
    super::menu::save_block(base).filter(|s| super::crash::readable(s + OWNED_OFF))
}

/// The save's byte, or `None` before it is set up (or with no save).
pub unsafe fn bits() -> Option<u8> {
    let s = save()?;
    let b = rd::<u8>(s + OWNED_OFF);
    (b & owned::SET_UP != 0).then_some(b)
}

/// The forms testing has always given (Valor and Wisdom).
fn always() -> u8 {
    owned::bit(formlevel::form::VALOR) | owned::bit(formlevel::form::WISDOM)
}

/// The forms Sora owns now (`formlevel::owned` bits, without bits 6 and 7). Story Unlocks off:
/// Valor and Wisdom, and Final once entered; on: the forms given in the save.
pub unsafe fn owned() -> u8 {
    let given = bits().unwrap_or(0) & 0x3F;
    if story_unlocks() {
        given
    } else {
        always() | (given & owned::bit(formlevel::form::FINAL))
    }
}

/// Sora owns KH2 form `form` (1..6).
pub unsafe fn owns(form: u8) -> bool {
    owned() & owned::bit(form) != 0
}

/// Gives Sora KH2 form `form`. True if he didn't own it before (a gain).
pub unsafe fn give(form: u8) -> bool {
    let Some(s) = save() else { return false };
    let b = rd::<u8>(s + OWNED_OFF);
    if b & owned::SET_UP == 0 || owned::bit(form) == 0 || b & owned::bit(form) != 0 {
        return false;
    }
    std::ptr::write_unaligned((s + OWNED_OFF) as *mut u8, b | owned::bit(form));
    log!(Forms, "form {form} owned now (save +0x{OWNED_OFF:X}: {:#04x})", b | owned::bit(form));
    true
}

/// The forms counted for the level cap (`formlevel::owned::for_cap`).
pub unsafe fn for_cap() -> u8 {
    owned::for_cap(owned())
}

/// Once a second with the save (`menu::Growth::update`): a save not set up yet is set up, with
/// Final if its record is set, silently; with Story Unlocks on, the table's forms are given.
pub unsafe fn check(save: usize) {
    if !super::crash::readable(save + OWNED_OFF) {
        return;
    }
    let b = rd::<u8>(save + OWNED_OFF);
    if b & owned::SET_UP != 0 {
        if story_unlocks() {
            story(save, b);
        }
        return;
    }
    let mut n = owned::SET_UP;
    if super::formexp::has_record(Kind::Final) {
        n |= owned::bit(formlevel::form::FINAL);
    }
    std::ptr::write_unaligned((save + OWNED_OFF) as *mut u8, n);
    log!(Forms, "owned forms set up for this save (save +0x{OWNED_OFF:X} was {b:#04x}, now {n:#04x}{})", if n & owned::bit(formlevel::form::FINAL) != 0 { ": Final" } else { "" });
}

/// The table's forms (Story Unlocks on): those not given yet are given; silently on the save's first
/// read or right after the row was switched on, else as gains during play.
unsafe fn story(save: usize, b: u8) {
    use crate::storyforms::{Flags, SPAN_END};
    if !super::crash::readable(save + SPAN_END - 1) {
        return;
    }
    let Some(f) = Flags::read(std::slice::from_raw_parts(save as *const u8, SPAN_END)) else { return };
    let quiet = QUIET.swap(false, Ordering::Relaxed) || b & STORY_READ == 0;
    let new = f.forms() & !b & 0x3F;
    if new == 0 && b & STORY_READ != 0 {
        return;
    }
    let n = b | new | STORY_READ;
    std::ptr::write_unaligned((save + OWNED_OFF) as *mut u8, n);
    if new & owned::bit(formlevel::form::VALOR) != 0 {
        valor_slot();
    }
    if new != 0 {
        log!(Forms, "Story Unlocks: forms {new:#04x} given{} ({}; save +0x{OWNED_OFF:X} now {n:#04x})", if quiet { " silently (the save's first read)" } else { " (a gain during play)" }, f.describe());
        if !quiet {
            gained(new);
        }
    }
}

/// Forms gained during play (bits): KH1's prompt box "New Drive Form: <name>" for each that is in
/// the Drive list (`form::Kind::ALL`; the owner, main s308: "the game should tell you when you get a
/// drive form"; t929). Forms not built yet (Limit, Master; Final on hold) get theirs when they are
/// built (answer 4 (a)); KH2 itself shows a gained form only as a line in its bonus window
/// (`0x1403EDCA0`, Checked (decomp)).
fn gained(bits: u8) {
    for kind in Kind::ALL {
        if bits & owned::bit(form_id(kind)) != 0 {
            super::menu::queue_prompt("New Drive Form", kind.name());
            log!(Forms, "New Drive Form: {}", kind.name());
        }
    }
}

/// Valor gained (t930, answer 5 (a)): KH2 puts the first owned Keyblade into an empty Keyblade slot
/// of Valor, Master or Final when the form's item is added (`0x1403C3FE0`: item-table order, kind 2
/// records, Checked (decomp)). Here Valor's slot (`equip::VALOR_OFF`, save `+0x488`), if empty,
/// gets the first Keyblade with a copy not equipped (`equip::valor_candidates`, lowest id first);
/// none: it stays empty, and Valor is refused without a spare Keyblade as before (testing's
/// two-blade moveset). KH2 takes the Keyblade out of the inventory; KH1's free count already leaves
/// it to Valor.
unsafe fn valor_slot() {
    let base = super::takeover::game_base();
    let Some(s) = super::menu::save_block(base) else { return };
    let at = s + super::equip::VALOR_OFF;
    if !super::crash::readable(at) || rd::<u8>(at) != 0 {
        return;
    }
    match super::equip::valor_candidates(base).first() {
        Some(&id) => {
            std::ptr::write_unaligned(at as *mut u8, id);
            log!(Forms, "Valor Form gained: its Keyblade slot gets item {id:#04x} (the first spare Keyblade)");
        }
        None => log!(Forms, "Valor Form gained: no spare Keyblade for its slot (it stays empty)"),
    }
}
