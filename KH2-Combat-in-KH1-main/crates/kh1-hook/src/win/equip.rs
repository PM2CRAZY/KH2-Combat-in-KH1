//! Read-only views of Sora's equipment and KH1's inventory, for Valor's, Master's and Final Form's
//! second Keyblades (`docs/kh1-equipment.md` has KH1's side). Nothing here writes to the game.
//! (The user: "When you put a keyblade in any of the drive equipment slots ... it doesn't remove
//! the keyblade from the list, which makes you able to have the same secondary keyblade for
//! multiple forms".)
//!
//! - KH1's party records are `0x74` bytes from the save block's start (member 0 is Sora); a
//!   record's weapon is its byte `+0x36` (a KH1 item id, 0 for none), its accessory count
//!   `+0x1C` and its accessories from `+0x1D`.
//! - The inventory is one count per item id at `save + 0x499 + id` (ids 1..=255). It counts
//!   every copy Sora owns: equipping a weapon or an accessory leaves it unchanged (KH1's "free"
//!   count, `0x1402E6330`, takes the copies equipped across the ten members off it).
//! - Valor's, Final's and Master's slots (`VALOR_OFF`, `FINAL_OFF`, `MASTER_OFF`) hold a copy
//!   each, as equipping does (`form_held`): the copy is taken off the free count, ours (`free`)
//!   and KH1's (hooked, `equipmenu::free_count`), off Sora's weapon list and off the other slots'
//!   lists. Valor's slot keeps its copy first, then Final's, then Master's.
//! - KH1's item table (`btltbl.bin`'s, at `*0x142D23738`): 255 records of `0x14` bytes, by
//!   id - 1; `+0` the name's word number, `+2` the kind (0 an item, 1 an accessory, 2 + member a
//!   weapon of that member: 2 is Sora's), `+6` the weapon's row in its member's weapon table
//!   (1-based), `+0xC` the help text's handle.
//! - Sora's weapons that aren't Keyblades: `0x52..=0x55` (Dream Sword / Shield / Rod and the
//!   wooden sword; KH1 keeps the weapon row locked while one is held, `0x1402E6C60`).
//! - Valor's default second weapon is the Dream Sword (`VALOR_DEFAULT`, the user: "whenever the
//!   player gets valor form, i want the Dream Sword to be in the second keyblade slot by
//!   default"): an empty or invalid Valor slot gets it, it needs no copy owned and holds none
//!   (as Final's and Master's Kingdom Key fallbacks), and the Valor row always offers it beside
//!   the free Keyblades (`valor_candidates`).
//!
//! Used by `equipmenu` (the Valor and Final rows in the Equipment page).

#![allow(dead_code)]

use super::addr::Rva;

/// The pointer to KH1's item table (inside the loaded `btltbl.bin`).
const ITEM_TABLE_PTR_RVA: Rva = Rva(0x2D23738);
/// The loaded `btltbl.bin` itself (weapon tables are offsets into it).
const BTLTBL_RVA: Rva = Rva(0x2D23740);
/// Each member's weapon table, as an offset into `btltbl.bin` (10 ints; Sora's first).
const WEAPON_TABLE_OFFSETS_RVA: Rva = Rva(0x528D60);
/// KH1's word list: 512 text handles (the item names among them).
const WORDS_RVA: Rva = Rva(0x2D23998);
/// The text handles' pages (`0x14038B230`: `pages[h >> 25 & 0x3F] | (h & 0x1FF_FFFF)`).
const TEXT_PAGES_RVA: Rva = Rva(0x2EE4730);

const MEMBER_SIZE: usize = 0x74;
const MEMBERS: usize = 10;
const ACC_COUNT_OFF: usize = 0x1C;
const ACC_OFF: usize = 0x1D;
const WEAPON_OFF: usize = 0x36;
const STOCK_OFF: usize = 0x499;
const ITEM_SIZE: usize = 0x14;
const ITEMS: usize = 255;
const WEAPON_ROW_SIZE: usize = 0x58;
/// Sora's weapon kind in the item table.
const SORA_WEAPON: u8 = 2;
/// Sora's weapons that aren't Keyblades.
const NOT_KEYBLADES: std::ops::RangeInclusive<u8> = 0x52..=0x55;
/// The Dream Sword (KH1 item 0x52: Sora's weapon kind, weapon row 2, `xw_dh_5000`; the Dive to
/// the Heart's sword, not a Keyblade by `is_keyblade`).
pub const DREAM_SWORD: u8 = 0x52;
/// Valor's second weapon when its slot holds none it may take (empty: a save that never had
/// Valor, or the first unlock): the Dream Sword. Sora needn't own it and it holds no copy.
pub const VALOR_DEFAULT: u8 = DREAM_SWORD;

/// The proposed place of the Valor Keyblade's id in the save: `+0x488`, the four bytes between
/// the ten party records (`0x74 * 10 = 0x488`) and Sora's growth bytes `+0x48C / +0x48D`, which
/// nothing in KH1 reads or writes (see the doc). Any value read there must be checked
/// (`valor_keyblade`), as old saves may hold anything.
pub const VALOR_OFF: usize = 0x488;
/// Final Form's Keyblade's id in the save: `+0x489`, the byte after Valor's (same four spare
/// bytes; the doc's section 5). Checked on every read as Valor's (`final_keyblade`).
pub const FINAL_OFF: usize = 0x489;
/// Final Form's Keyblade when its slot holds none Sora may take: the Kingdom Key (item 0x51),
/// as Final always had before it got a slot.
pub const FINAL_DEFAULT: u8 = 0x51;
/// Master Form's Keyblade's id in the save: `+0x48B`, the last of the four spare bytes (the doc's
/// section 5: no access by KH1 at all). The testing branch's ability-split mark is a u16 at
/// `+0x48A`, so it overlaps this byte; its value there (0x4B) is no Keyblade, so the checked read
/// falls back. Checked on every read as Final's (`master_keyblade`).
pub const MASTER_OFF: usize = 0x48B;
/// Master Form's Keyblade when its slot holds none Sora may take: the Kingdom Key, as Final's.
pub const MASTER_DEFAULT: u8 = 0x51;

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn save(base: usize) -> Option<usize> {
    super::menu::save_block(base)
}

/// KH1's item record for `id` (1..=255).
unsafe fn item(base: usize, id: u8) -> Option<usize> {
    if id == 0 {
        return None;
    }
    let table = rd::<u64>(base + ITEM_TABLE_PTR_RVA) as usize;
    let rec = table + (id as usize - 1) * ITEM_SIZE;
    (table >= 0x10000 && super::crash::readable(table) && super::crash::readable(rec + ITEM_SIZE - 1)).then_some(rec)
}

/// The item's kind (0 item, 1 accessory, 2.. a member's weapon: 2 Sora's).
pub unsafe fn kind(base: usize, id: u8) -> Option<u8> {
    item(base, id).map(|r| rd::<u8>(r + 2))
}

/// One of Sora's Keyblades (his weapons, less the dream weapons and the wooden sword).
pub unsafe fn is_keyblade(base: usize, id: u8) -> bool {
    !NOT_KEYBLADES.contains(&id) && kind(base, id) == Some(SORA_WEAPON)
}

/// Sora's equipped weapon (KH1 item id, save `+0x36`), if the save reads.
pub unsafe fn sora_weapon(base: usize) -> Option<u8> {
    save(base).map(|s| rd::<u8>(s + WEAPON_OFF)).filter(|w| *w != 0)
}

/// How many of `id` the inventory holds (equipped copies included).
pub unsafe fn owned(base: usize, id: u8) -> u8 {
    match save(base) {
        Some(s) if id != 0 => rd::<u8>(s + STOCK_OFF + id as usize),
        _ => 0,
    }
}

/// How many of `id` are equipped across KH1's ten members (weapons and accessories).
pub unsafe fn equipped_count(base: usize, id: u8) -> u32 {
    let Some(s) = save(base) else { return 0 };
    let mut n = 0;
    for m in 0..MEMBERS {
        let r = s + m * MEMBER_SIZE;
        n += (rd::<u8>(r + WEAPON_OFF) == id) as u32;
        let acc = (rd::<u8>(r + ACC_COUNT_OFF) as usize).min(8);
        n += (0..acc).filter(|i| rd::<u8>(r + ACC_OFF + i) == id).count() as u32;
    }
    n
}

/// Copies of `id` owned and not in use: not equipped by a member (KH1's own count,
/// `0x1402E6330`) and not held by Valor's or Final's slot (`form_held`; KH1's count is hooked
/// to take those off too, `equipmenu::free_count`).
pub unsafe fn free(base: usize, id: u8) -> u32 {
    (owned(base, id) as u32).saturating_sub(equipped_count(base, id) + form_held(base, id))
}

/// The raw byte of a form's slot (`VALOR_OFF`, `FINAL_OFF`), 0 if the save doesn't read.
unsafe fn slot_byte(base: usize, off: usize) -> u8 {
    save(base).map(|s| rd::<u8>(s + off)).unwrap_or(0)
}

/// `id` may be Valor's: a Keyblade with a copy no member has equipped. Valor's slot comes first:
/// Final's doesn't count against it (an old save with both slots on one copy: Valor keeps it).
pub unsafe fn valor_may_hold(base: usize, id: u8) -> bool {
    is_keyblade(base, id) && owned(base, id) as u32 > equipped_count(base, id)
}

/// `id` may be Valor's second weapon: the Dream Sword (`VALOR_DEFAULT`, always: no copy needed),
/// or a Keyblade it may hold (`valor_may_hold`).
pub unsafe fn valor_may_use(base: usize, id: u8) -> bool {
    id == VALOR_DEFAULT || valor_may_hold(base, id)
}

/// `id` may be Final's: a Keyblade with a copy that neither a member nor Valor's slot holds.
pub unsafe fn final_may_hold(base: usize, id: u8) -> bool {
    let valor = (valor_held(base) == id) as u32;
    is_keyblade(base, id) && owned(base, id) as u32 > equipped_count(base, id) + valor
}

/// The Keyblade a copy of which Valor's slot holds (0: none): the slot's, while Valor is unlocked
/// (`form::unlocked`; a slot whose row isn't shown takes nothing from Sora) and it may be Valor's.
/// The Dream Sword (`VALOR_DEFAULT`) holds no copy.
pub unsafe fn valor_held(base: usize) -> u8 {
    let id = slot_byte(base, VALOR_OFF);
    if id != 0 && super::form::unlocked(super::form::Kind::Valor) && valor_may_hold(base, id) {
        id
    } else {
        0
    }
}

/// The Keyblade a copy of which Final's slot holds (0: none), as Valor's. Its Kingdom Key
/// fallback (an empty slot, `final_keyblade`) holds no copy.
pub unsafe fn final_held(base: usize) -> u8 {
    let id = slot_byte(base, FINAL_OFF);
    if id != 0 && super::form::unlocked(super::form::Kind::Final) && final_may_hold(base, id) {
        id
    } else {
        0
    }
}

/// `id` may be Master's: a Keyblade with a copy that no member, not Valor's slot and not Final's
/// holds (Master's slot, the newest, comes last: older saves keep their Valor and Final copies).
pub unsafe fn master_may_hold(base: usize, id: u8) -> bool {
    let others = (valor_held(base) == id) as u32 + (final_held(base) == id) as u32;
    is_keyblade(base, id) && owned(base, id) as u32 > equipped_count(base, id) + others
}

/// The Keyblade a copy of which Master's slot holds (0: none), as Final's. Its Kingdom Key
/// fallback (an empty slot, `master_keyblade`) holds no copy.
pub unsafe fn master_held(base: usize) -> u8 {
    let id = slot_byte(base, MASTER_OFF);
    if id != 0 && super::form::unlocked(super::form::Kind::Master) && master_may_hold(base, id) {
        id
    } else {
        0
    }
}

/// How many copies of `id` the form slots hold (0..=3; one each at most).
pub unsafe fn form_held(base: usize, id: u8) -> u32 {
    if id == 0 {
        return 0;
    }
    // (Cheap first: only an id in a slot byte can be held.)
    let (v, f, m) = (slot_byte(base, VALOR_OFF), slot_byte(base, FINAL_OFF), slot_byte(base, MASTER_OFF));
    if v != id && f != id && m != id {
        return 0;
    }
    (v == id && valor_held(base) == id) as u32 + (f == id && final_held(base) == id) as u32 + (m == id && master_held(base) == id) as u32
}

/// Every Keyblade Sora owns (equipped one included), by id.
pub unsafe fn owned_keyblades(base: usize) -> Vec<u8> {
    (1..=ITEMS as u8).filter(|&id| owned(base, id) > 0 && is_keyblade(base, id)).collect()
}

/// What Valor's slot may take: the Dream Sword first (`VALOR_DEFAULT`, always, owned or not: so
/// the player can go back to it), then the Keyblades owned with a copy that no member has
/// equipped and neither Final's nor Master's slot holds (so never Sora's own weapon or another
/// slot's unless he owns another copy); Valor's own current one included.
pub unsafe fn valor_candidates(base: usize) -> Vec<u8> {
    let (fin, master) = (final_held(base), master_held(base));
    let mut v = vec![VALOR_DEFAULT];
    v.extend(owned_keyblades(base).into_iter().filter(|&id| owned(base, id) as u32 > equipped_count(base, id) + (fin == id) as u32 + (master == id) as u32));
    v
}

/// The Keyblades Final's slot may take: as Valor's, with Valor's and Master's slots' copies taken
/// off; Final's own current one included.
pub unsafe fn final_candidates(base: usize) -> Vec<u8> {
    let (valor, master) = (valor_held(base), master_held(base));
    owned_keyblades(base).into_iter().filter(|&id| owned(base, id) as u32 > equipped_count(base, id) + (valor == id) as u32 + (master == id) as u32).collect()
}

/// The Keyblades Master's slot may take: as Final's, with Valor's and Final's slots' copies taken
/// off; Master's own current one included.
pub unsafe fn master_candidates(base: usize) -> Vec<u8> {
    let (valor, fin) = (valor_held(base), final_held(base));
    owned_keyblades(base).into_iter().filter(|&id| owned(base, id) as u32 > equipped_count(base, id) + (valor == id) as u32 + (fin == id) as u32).collect()
}

/// The Strength a Sora weapon adds (its weapon-table row's `+0x30`; KH1's `0x1402A2FA0`).
pub unsafe fn weapon_strength(base: usize, id: u8) -> Option<i8> {
    weapon_row(base, id).map(|r| rd::<i8>(r + 0x30))
}

/// The max MP a Sora weapon adds (its row's `+0x38`).
pub unsafe fn weapon_mp(base: usize, id: u8) -> Option<i8> {
    weapon_row(base, id).map(|r| rd::<i8>(r + 0x38))
}

/// Sora's weapon-table row for a weapon of his (`0x140284960 (0, id)`).
unsafe fn weapon_row(base: usize, id: u8) -> Option<usize> {
    let rec = item(base, id)?;
    if rd::<u8>(rec + 2) != SORA_WEAPON {
        return None;
    }
    let row = rd::<i16>(rec + 6);
    if row <= 0 {
        return None;
    }
    let table = base + BTLTBL_RVA + rd::<i32>(base + WEAPON_TABLE_OFFSETS_RVA) as usize;
    let at = table + (row as usize - 1) * WEAPON_ROW_SIZE;
    super::crash::readable(at + WEAPON_ROW_SIZE - 1).then_some(at)
}

/// The id stored for Valor (`VALOR_OFF`), if Valor may still use it (`valor_may_use`: the Dream
/// Sword, or a Keyblade with its copy, kept before Final's slot); else the Dream Sword
/// (`VALOR_DEFAULT`; `valor::keyblade` then writes it in the slot). Valor is never refused for
/// want of a second weapon (it took the strongest free Keyblade before, and none refused Valor).
pub unsafe fn valor_keyblade(base: usize) -> u8 {
    let stored = slot_byte(base, VALOR_OFF);
    if stored != 0 && valor_may_use(base, stored) {
        stored
    } else {
        VALOR_DEFAULT
    }
}

/// Valor's slot as it is stored (kept by `valor::keyblade`), the Dream Sword when it holds neither
/// a Keyblade nor the Dream Sword: for the model, the stats and the menu during a pick, as
/// `final_stored`.
pub unsafe fn valor_stored(base: usize) -> u8 {
    let stored = slot_byte(base, VALOR_OFF);
    if stored == VALOR_DEFAULT || is_keyblade(base, stored) {
        stored
    } else {
        VALOR_DEFAULT
    }
}

/// The id stored for Final (`FINAL_OFF`), if it is a Keyblade Sora owns with a copy that no
/// member and not Valor's slot holds (`final_may_hold`); else the Kingdom Key (`FINAL_DEFAULT`),
/// which Final may always use, equipped or not, without holding a copy (`valor::final_keyblade`
/// then empties the slot). Final is never refused for want of a Keyblade.
pub unsafe fn final_keyblade(base: usize) -> u8 {
    let stored = slot_byte(base, FINAL_OFF);
    if stored != 0 && final_may_hold(base, stored) {
        stored
    } else {
        FINAL_DEFAULT
    }
}

/// Final's slot as it is stored (kept by `valor::final_keyblade`), the Kingdom Key when it holds
/// no Keyblade (an empty slot, an old save): for the model, the stats and the menu during a pick,
/// where the free copies can't be counted (the menu's preview puts the hovered one in Sora's
/// weapon byte).
pub unsafe fn final_stored(base: usize) -> u8 {
    let stored = save(base).map(|s| rd::<u8>(s + FINAL_OFF)).unwrap_or(0);
    if is_keyblade(base, stored) {
        stored
    } else {
        FINAL_DEFAULT
    }
}

/// The id stored for Master (`MASTER_OFF`), if it is a Keyblade Sora owns with a copy that no
/// member and neither Valor's nor Final's slot holds (`master_may_hold`); else the Kingdom Key
/// (`MASTER_DEFAULT`), as Final's (`valor::master_keyblade` then empties the slot). Master is
/// never refused for want of a Keyblade.
pub unsafe fn master_keyblade(base: usize) -> u8 {
    let stored = slot_byte(base, MASTER_OFF);
    if stored != 0 && master_may_hold(base, stored) {
        stored
    } else {
        MASTER_DEFAULT
    }
}

/// Master's slot as it is stored, the Kingdom Key when it holds no Keyblade: as `final_stored`.
pub unsafe fn master_stored(base: usize) -> u8 {
    let stored = save(base).map(|s| rd::<u8>(s + MASTER_OFF)).unwrap_or(0);
    if is_keyblade(base, stored) {
        stored
    } else {
        MASTER_DEFAULT
    }
}

/// KH1's text for a handle (`0x14038B0B0`), 0 for none.
unsafe fn text(base: usize, handle: u32) -> usize {
    if handle == 0 {
        return 0;
    }
    let h = handle & 0x7FFF_FFFF;
    let page = rd::<u64>(base + TEXT_PAGES_RVA + ((h >> 25) as usize) * 8) as usize;
    let p = page | (h & 0x1FF_FFFF) as usize;
    (page != 0 && super::crash::readable(p)).then_some(p).unwrap_or(0)
}

/// The item's name as KH1 text codes (KH1's `0x1402E6570`: the word `+0` of its record), 0 if
/// it doesn't read.
pub unsafe fn item_name_ptr(base: usize, id: u8) -> usize {
    let Some(rec) = item(base, id) else { return 0 };
    let word = rd::<u16>(rec) as usize;
    if word >= 512 {
        return 0;
    }
    text(base, rd::<u32>(base + WORDS_RVA + word * 4))
}

/// The item's name in plain text (KH1's codes for letters, digits and a few marks; others as
/// `{xx}`), for logs and our own menu strings.
pub unsafe fn item_name(base: usize, id: u8) -> Option<String> {
    let p = item_name_ptr(base, id);
    if p == 0 {
        return None;
    }
    let mut s = String::new();
    for i in 0..48 {
        if !super::crash::readable(p + i) {
            break;
        }
        let b = rd::<u8>(p + i);
        match b {
            0 => break,
            0x01 => s.push(' '),
            0x21..=0x2A => s.push((b'0' + b - 0x21) as char),
            0x2B..=0x44 => s.push((b'A' + b - 0x2B) as char),
            0x45..=0x5E => s.push((b'a' + b - 0x45) as char),
            0x6E => s.push('-'),
            0x68 => s.push('.'),
            0x71 => s.push('\''),
            _ => s.push_str(&format!("{{{b:02x}}}")),
        }
    }
    Some(s)
}
