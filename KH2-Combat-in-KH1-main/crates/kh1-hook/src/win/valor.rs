//! Valor Form's second Keyblade (the user: "add the slot in menu for valor form, and
//! automatically put a keyblade that isnt equipped into the slot. If no keyblade is eligible to
//! be in that slot, dont let players go into valor form"), and its default, the Dream Sword (the
//! user: "whenever the player gets valor form, i want the Dream Sword to be in the second
//! keyblade slot by default").
//!
//! - The slot: an item id kept in the save at `equip::VALOR_OFF` (`docs/kh1-equipment.md`).
//! - Eligible: the Dream Sword (`equip::VALOR_DEFAULT`, item 0x52: needn't be owned, holds no
//!   copy), or a Keyblade Sora owns with a copy not equipped (`equip::valor_candidates`). The
//!   slot's pick is kept while it stays eligible; an empty slot (Valor just unlocked, or an old
//!   save that never had one) or an invalid one gets the Dream Sword, written in the slot
//!   (`equip::valor_keyblade`). So Valor is never refused for want of a second weapon (before,
//!   the strongest free Keyblade was put in, and with none Valor couldn't be chosen).
//! - In the form, its swings' second-hand hitboxes are KH2's (the rig's second weapon); its model
//!   in his left hand: `started` / `ended`.
//! - Final Form has a slot of its own (the user: "add a section in the equipment menu for the
//!   second keyblade in final form"): `equip::FINAL_OFF`, the same Keyblades to choose from, but
//!   the Kingdom Key when it holds none of them (`final_keyblade`), so Final is never refused.
//! - Master Form too (`equip::MASTER_OFF`, `master_keyblade`), as Final's: the Kingdom Key when it
//!   holds none Sora may give.
//! - Each slot holds its copy as equipping does (`equip::form_held`): one copy can't be in two
//!   slots, nor in a slot and Sora's hand. Valor's slot keeps its copy first, then Final's, then
//!   Master's; Final's and Master's Kingdom Key fallbacks hold none (their slots are emptied),
//!   nor does Valor's Dream Sword (kept in its slot: it is Valor's default and a pick of its own).

use super::log;
use std::sync::atomic::{AtomicU32, Ordering};

/// The weapon in Valor's slot, as last decided (0 none), for the log.
static CHOSEN: AtomicU32 = AtomicU32::new(0);

/// The weapon Valor uses now (its item id: a Keyblade picked in the menu, or the Dream Sword),
/// the slot filled or fixed as it goes: an empty or invalid slot gets the Dream Sword (only then:
/// a Keyblade picked stays). `None` only when the save doesn't read.
pub unsafe fn keyblade() -> Option<u8> {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let save = super::menu::save_block(base)?;
    let slot = (save + super::equip::VALOR_OFF) as *mut u8;
    let stored = std::ptr::read_unaligned(slot);
    // (While a form row's pick may have the hovered Keyblade in Sora's weapon byte, the copies
    // can't be counted: the slot as stored, nothing written, as Final's.)
    let picking = super::equipmenu::pick_own().is_some();
    let want = if picking { super::equip::valor_stored(base) } else { super::equip::valor_keyblade(base) };
    if !picking && stored != want {
        std::ptr::write_unaligned(slot, want);
        let name = |k: u8| super::equip::item_name(base, k).unwrap_or_else(|| format!("{k:#x}"));
        let why = if stored == 0 {
            "the slot was empty (Valor just unlocked, or a save from before the Dream Sword default)".to_string()
        } else if super::equip::is_keyblade(base, stored) {
            format!("{} ({stored:#x}) has no copy free for it any more", name(stored))
        } else {
            format!("the slot held {stored:#x}, not a Keyblade (an old save)")
        };
        log(&format!("Valor's second weapon: {why}: {} ({want:#x}) written in the slot", name(want)));
    }
    // While he is Valor, the slot's weapon in his hand, changed or not (the Equipment row, the
    // user: "when switching the second keyblade out while in valor form it doesnt swap
    // properly"; `leftblade::show` does nothing more when it's the one loaded, and asks again
    // while another load is still under way).
    if super::form::in_valor() {
        super::leftblade::show(Some(want));
    }
    let prev = CHOSEN.swap(want as u32, Ordering::Relaxed);
    if prev != want as u32 {
        let name = super::equip::item_name(base, want).unwrap_or_else(|| "?".into());
        let how = if want == super::equip::VALOR_DEFAULT {
            "the default, no copy held"
        } else if prev == 0 && stored == want {
            "kept from the save, its copy held"
        } else {
            "its copy held"
        };
        log(&format!(
            "Valor's second weapon: {name} ({want:#x}; {how}; Sora's {:?}, Keyblades owned {:02x?})",
            super::equip::sora_weapon(base),
            super::equip::owned_keyblades(base)
        ));
    }
    Some(want)
}

/// The Keyblade in Final's slot, as last decided (0 not yet), for the log.
static FINAL_CHOSEN: AtomicU32 = AtomicU32::new(0);

/// The Keyblade Final uses now (its item id): the slot's if Sora may give it (a copy no member
/// and not Valor's slot holds), else the Kingdom Key (`equip::final_keyblade`). The slot is
/// emptied for the Kingdom Key fallback, so the fallback holds no copy (`equip::final_held`);
/// a Kingdom Key picked in the menu stays and holds one.
pub unsafe fn final_keyblade() -> u8 {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    // (While a form row's pick may have the hovered Keyblade in Sora's weapon byte, the copies
    // can't be counted: the slot as stored, nothing written.)
    let picking = super::equipmenu::pick_own().is_some();
    let k = if picking { super::equip::final_stored(base) } else { super::equip::final_keyblade(base) };
    if let (Some(save), false) = (super::menu::save_block(base), picking) {
        let slot = (save + super::equip::FINAL_OFF) as *mut u8;
        let stored = std::ptr::read_unaligned(slot);
        if stored != 0 && !super::equip::final_may_hold(base, stored) {
            // (An old save with both slots on Sora's one free copy: Valor keeps it.)
            let why = if super::equip::valor_held(base) == stored {
                "its copy is Valor's (Valor's slot keeps it)"
            } else {
                "no copy of it is free"
            };
            let name = super::equip::item_name(base, stored).unwrap_or_else(|| format!("{stored:#x}"));
            log(&format!("Final's Keyblade: {name} ({stored:#x}) {why}: Final falls back to the Kingdom Key, slot emptied"));
            std::ptr::write_unaligned(slot, 0);
        }
    }
    // (In Final, a change made in the menu reaches his left hand, as Valor's.)
    if super::form::in_final() {
        super::leftblade::show(Some(k));
    }
    if FINAL_CHOSEN.swap(k as u32, Ordering::Relaxed) != k as u32 {
        let name = super::equip::item_name(base, k).unwrap_or_else(|| format!("{k:#x}"));
        log(&format!("Final's Keyblade: {name} ({k:#x}; Sora's {:?})", super::equip::sora_weapon(base)));
    }
    k
}

/// The Keyblade in Master's slot, as last decided (0 not yet), for the log.
static MASTER_CHOSEN: AtomicU32 = AtomicU32::new(0);

/// The Keyblade Master uses now (its item id), as Final's (`final_keyblade`): the slot's if Sora
/// may give it (a copy no member, not Valor's slot and not Final's holds), else the Kingdom Key,
/// the slot emptied so the fallback holds no copy.
pub unsafe fn master_keyblade() -> u8 {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    // (During a form row's pick the hovered Keyblade may be in Sora's weapon byte: as stored.)
    let picking = super::equipmenu::pick_own().is_some();
    let k = if picking { super::equip::master_stored(base) } else { super::equip::master_keyblade(base) };
    if let (Some(save), false) = (super::menu::save_block(base), picking) {
        let slot = (save + super::equip::MASTER_OFF) as *mut u8;
        let stored = std::ptr::read_unaligned(slot);
        // (A byte that is no Keyblade at all is left as it is, only read past: the testing branch
        // keeps a mark over this byte, `equip::MASTER_OFF`.)
        if stored != 0 && super::equip::is_keyblade(base, stored) && !super::equip::master_may_hold(base, stored) {
            let why = if super::equip::valor_held(base) == stored {
                "its copy is Valor's"
            } else if super::equip::final_held(base) == stored {
                "its copy is Final's"
            } else {
                "no copy of it is free"
            };
            let name = super::equip::item_name(base, stored).unwrap_or_else(|| format!("{stored:#x}"));
            log(&format!("Master's Keyblade: {name} ({stored:#x}) {why}: Master falls back to the Kingdom Key, slot emptied"));
            std::ptr::write_unaligned(slot, 0);
        }
    }
    // (In Master, a change made in the menu reaches his second Keyblade, as Valor's.)
    if super::form::in_master() {
        super::leftblade::show(Some(k));
    }
    if MASTER_CHOSEN.swap(k as u32, Ordering::Relaxed) != k as u32 {
        let name = super::equip::item_name(base, k).unwrap_or_else(|| format!("{k:#x}"));
        log(&format!("Master's Keyblade: {name} ({k:#x}; Sora's {:?})", super::equip::sora_weapon(base)));
    }
    k
}

/// Sora became Valor (or Master, or Final): his second Keyblade comes out.
pub unsafe fn started() {
    if super::form::in_final() {
        let k = final_keyblade();
        log(&format!("Final Form: second Keyblade {k:#x}"));
        super::leftblade::show(Some(k));
        return;
    }
    if super::form::in_master() {
        let k = master_keyblade();
        log(&format!("Master Form: second Keyblade {k:#x}"));
        super::leftblade::show(Some(k));
        return;
    }
    let k = keyblade();
    log(&format!("Valor Form: second weapon {:?}", k.map(|k| format!("{k:#x}"))));
    super::leftblade::show(k);
}

/// Back from Valor.
pub unsafe fn ended() {
    super::leftblade::hide();
    // Sora's own Strength and max MP again (`valorstats`).
    super::valorstats::step(super::takeover::sora_actor());
}
