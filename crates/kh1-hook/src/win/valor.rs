//! Valor Form's second Keyblade (the user: "add the slot in menu for valor form, and
//! automatically put a keyblade that isnt equipped into the slot"; t966, the main-branch port's
//! round 11 answer 1 (a): an empty slot is Sora's own Keyblade, as KH2).
//!
//! - The slot: a Keyblade id kept in the save at `equip::VALOR_OFF` (`docs/kh1-equipment.md`),
//!   filled when Valor is gained (`formown::valor_slot`, KH2's first spare Keyblade).
//! - The slot's Keyblade is kept while Sora owns a copy no member has equipped
//!   (`equip::valor_holds`); otherwise the slot is emptied, and Valor uses Sora's own Keyblade
//!   (`equip::valor_keyblade`; KH2's `0x1403DFEB0`, case 0x23B), so Valor no longer needs a spare
//!   Keyblade. (Before t966: the free Keyblade with the most Strength, and no Valor with none.)
//! - In the form, its swings' second-hand hitboxes are KH2's (the rig's second weapon); its model
//!   in his left hand: `started` / `ended`.

use super::log;
use std::sync::atomic::{AtomicU32, Ordering};

/// The Keyblade in Valor's slot, as last decided (0 none), for the log.
static CHOSEN: AtomicU32 = AtomicU32::new(0);

/// The Keyblade Valor uses now (its item id): the slot's, else Sora's own; a slot whose Keyblade
/// has no copy left is emptied (KH2's empty slot). `None` when Sora holds no Keyblade.
pub unsafe fn keyblade() -> Option<u8> {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let save = super::menu::save_block(base)?;
    let slot = (save + super::equip::VALOR_OFF) as *mut u8;
    let held = super::equip::valor_holds(base).unwrap_or(0);
    if std::ptr::read_unaligned(slot) != held {
        std::ptr::write_unaligned(slot, held);
    }
    let pick = super::equip::valor_keyblade(base);
    let want = pick.unwrap_or(0);
    // While he is Valor, the slot's Keyblade in his hand, changed or not (the Equipment row, the
    // user: "when switching the second keyblade out while in valor form it doesnt swap
    // properly"; `leftblade::show` does nothing more when it's the one loaded, and asks again
    // while another load is still under way).
    if super::form::in_valor() {
        super::leftblade::show(pick);
    }
    if CHOSEN.swap(want as u32, Ordering::Relaxed) != want as u32 {
        let name = pick.and_then(|k| super::equip::item_name(base, k)).unwrap_or_else(|| "none".into());
        log!(Forms,
            "Valor's Keyblade: {name} ({want:#x}; Sora's {:?}, Keyblades owned {:02x?})",
            super::equip::sora_weapon(base),
            super::equip::owned_keyblades(base)
        );
    }
    pick
}

/// Final Form's second Keyblade: always the Kingdom Key (KH1 item 0x51), no slot (the user).
pub const FINAL_KEYBLADE: u8 = 0x51;

/// Master Form's second (floating) Keyblade (t1024; its M: slot since t1025): the slot's
/// (`equip::MASTER_OFF`, save `+0x489`), else Sora's own (KH2's rule, the owner's round 11 answer
/// 1 (a)). Read only. `None` when Sora holds no Keyblade.
pub unsafe fn master_keyblade() -> Option<u8> {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    super::equip::master_keyblade(base)
}

/// The Keyblade in Master's slot, as last decided (0 none), for the log (t1025).
static CHOSEN_MASTER: AtomicU32 = AtomicU32::new(0);

/// Master's Keyblade now (t1025, as `keyblade` for Valor): a slot whose Keyblade has no copy left
/// (or an old save's byte that isn't a Keyblade) is emptied, and in Master the Keyblade goes to his
/// left hand. Called from the Equipment row, Master's start and its 10-frame check.
pub unsafe fn master_slot() -> Option<u8> {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let save = super::menu::save_block(base)?;
    let slot = (save + super::equip::MASTER_OFF) as *mut u8;
    let held = super::equip::master_holds(base).unwrap_or(0);
    if std::ptr::read_unaligned(slot) != held {
        std::ptr::write_unaligned(slot, held);
    }
    let pick = super::equip::master_keyblade(base);
    let want = pick.unwrap_or(0);
    if super::form::in_master() {
        super::leftblade::show(pick);
    }
    if CHOSEN_MASTER.swap(want as u32, Ordering::Relaxed) != want as u32 {
        let name = pick.and_then(|k| super::equip::item_name(base, k)).unwrap_or_else(|| "none".into());
        log!(Forms, "Master's Keyblade: {name} ({want:#x}; slot {held:#x}, Sora's {:?})", super::equip::sora_weapon(base));
    }
    pick
}

/// The second Keyblade of the two-bladed form he is in (KH1 item id): Valor's slot's, Master's,
/// Final's (`model::left_weapon`, `valorstats::bonus`; no save write).
pub unsafe fn second_keyblade(base: usize) -> Option<u8> {
    match super::form::current()? {
        super::form::Kind::Final => Some(FINAL_KEYBLADE),
        super::form::Kind::Valor => super::equip::valor_keyblade(base),
        super::form::Kind::Master => master_keyblade(),
        super::form::Kind::Wisdom | super::form::Kind::Limit => None,
    }
}

/// Sora became Valor (or Final): his second Keyblade comes out.
pub unsafe fn started() {
    if super::form::in_final() {
        log!(Forms, "Final Form: second Keyblade the Kingdom Key (0x51)");
        super::leftblade::show(Some(FINAL_KEYBLADE));
        return;
    }
    if super::form::in_master() {
        // (t1025: its M: slot's, else Sora's own; `master_slot` shows it.)
        let k = master_slot();
        log!(Forms, "Master Form: second Keyblade {:?}", k.map(|k| format!("{k:#x}")));
        super::leftblade::show(k);
        return;
    }
    let k = keyblade();
    log!(Forms, "Valor Form: second Keyblade {:?}", k.map(|k| format!("{k:#x}")));
    super::leftblade::show(k);
}

/// Back from Valor.
pub unsafe fn ended() {
    super::leftblade::hide();
    // Sora's own Strength and max MP again (`valorstats`).
    super::valorstats::step(super::takeover::sora_actor());
}
