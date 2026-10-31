//! Read-only views of Sora's equipment and KH1's inventory, for Valor Form's second Keyblade
//! (`docs/kh1-equipment.md` has KH1's side). Nothing here writes to the game.
//!
//! - KH1's party records are `0x74` bytes from the save block's start (member 0 is Sora); a
//!   record's weapon is its byte `+0x36` (a KH1 item id, 0 for none), its accessory count
//!   `+0x1C` and its accessories from `+0x1D`.
//! - The inventory is one count per item id at `save + 0x499 + id` (ids 1..=255). It counts
//!   every copy Sora owns: equipping a weapon or an accessory leaves it unchanged (KH1's "free"
//!   count, `0x1402E6330`, takes the copies equipped across the ten members off it).
//! - KH1's item table (`btltbl.bin`'s, at `*0x142D23738`): 255 records of `0x14` bytes, by
//!   id - 1; `+0` the name's word number, `+2` the kind (0 an item, 1 an accessory, 2 + member a
//!   weapon of that member: 2 is Sora's), `+6` the weapon's row in its member's weapon table
//!   (1-based), `+0xC` the help text's handle.
//! - Sora's weapons that aren't Keyblades: `0x52..=0x55` (Dream Sword / Shield / Rod and the
//!   wooden sword; KH1 keeps the weapon row locked while one is held, `0x1402E6C60`).
//!
//! Used by `equipmenu` (the Valor row in the Equipment page).

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

/// The proposed place of the Valor Keyblade's id in the save: `+0x488`, the four bytes between
/// the ten party records (`0x74 * 10 = 0x488`) and Sora's growth bytes `+0x48C / +0x48D`, which
/// nothing in KH1 reads or writes (see the doc). Any value read there must be checked
/// (`valor_keyblade`), as old saves may hold anything.
pub const VALOR_OFF: usize = 0x488;

/// Master Form's Keyblade id in the save (t1025, the owner's answer 1 after research round 2):
/// `+0x489`, the byte after Valor's. Checked (exe), `docs/kh1-equipment.md` §5: no displacement,
/// immediate or absolute address `0x489..0x48B` in `.text`, and new game (`0x140283150`) writes
/// nothing at `0x488..0x48B`; nothing of ours used it before (Checked (code); the layout mark is
/// the u16 at `+0x48A`). An old save may hold anything there: a value that isn't a Keyblade with a
/// copy free is an empty slot (`master_holds`), and `valor::master_slot` clears it.
pub const MASTER_OFF: usize = 0x489;

/// Final Form's Keyblade id in the save (t1038, the owner's answer 1 (a) after Final's research
/// round 2): `+0x894`, the byte after Master's level u16 (`formexp::MASTER_LEVEL_OFF`, `+0x892..`)
/// in the span `+0x888..+0xC00` no KH1 field list describes (`formexp`'s Limit notes). Checked
/// (exe): no `.text` instruction names `0x142DE9BF4` (save `0x142DE9360` + `0x894`) by address,
/// and the three `0x894` displacements are other objects: `0x140041980` and `0x140100170` a stack
/// frame (`rbp` from `rsp`), `0x14039DB44` an object of floats whose same run also writes `+0x880` /
/// `+0x888` / `+0x890`. Checked (code): nothing of ours used `+0x894` (Master's level is the u16
/// `+0x892..+0x893`). Assumption: nothing reaches it through a computed pointer. An old save may
/// hold anything there: a value that isn't a Keyblade with a copy free is an empty slot
/// (`final_holds`), and `valor::final_slot` clears it.
pub const FINAL_OFF: usize = 0x894;

/// The save byte of a form's Keyblade slot (t1025; Final's since t1038): KH2 gives forms 1 / 4 / 5
/// one (`0x1402FC860`).
pub fn slot_off(kind: super::form::Kind) -> Option<usize> {
    match kind {
        super::form::Kind::Valor => Some(VALOR_OFF),
        super::form::Kind::Master => Some(MASTER_OFF),
        super::form::Kind::Final => Some(FINAL_OFF),
        _ => None,
    }
}

/// The forms with a Keyblade slot, in the order their slots keep a shared copy (t1038): Valor
/// first, then Master, then Final.
pub const SLOT_FORMS: [super::form::Kind; 3] = [super::form::Kind::Valor, super::form::Kind::Master, super::form::Kind::Final];

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

/// Copies of `id` owned and not equipped (KH1's `0x1402E6330`).
pub unsafe fn free(base: usize, id: u8) -> u32 {
    (owned(base, id) as u32).saturating_sub(equipped_count(base, id))
}

/// Every Keyblade Sora owns (equipped one included), by id.
pub unsafe fn owned_keyblades(base: usize) -> Vec<u8> {
    (1..=ITEMS as u8).filter(|&id| owned(base, id) > 0 && is_keyblade(base, id)).collect()
}

/// The Keyblades Valor could take: owned, and a copy not equipped (so never Sora's own
/// weapon unless he owns two), less the copies Master's slot (t1025) and Final's (t1038) hold.
pub unsafe fn valor_candidates(base: usize) -> Vec<u8> {
    candidates(base, super::form::Kind::Valor)
}

/// The Keyblades a form's slot could take (t1025; three-way since t1038): owned, with a copy free
/// of the members and of the other forms' slots (each copy another slot holds taken off). KH2
/// (Checked (decomp), `docs/kh1-drive-forms-native.md` §22.5): a slotted Keyblade is out of the
/// stock, so it is gone from the other forms' lists. With the other slots empty a list is what it
/// was.
pub unsafe fn candidates(base: usize, kind: super::form::Kind) -> Vec<u8> {
    if slot_off(kind).is_none() {
        return Vec::new();
    }
    let others: Vec<u8> = SLOT_FORMS.into_iter().filter(|&k| k != kind).filter_map(|k| slot_holds(base, k)).collect();
    owned_keyblades(base).into_iter().filter(|&id| free(base, id) > others.iter().filter(|&&o| o == id).count() as u32).collect()
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

/// The Keyblade Valor's slot holds a copy of (t966): the id stored (`VALOR_OFF`) while it is a
/// Keyblade Sora owns with a copy no member has equipped. KH2 moves a slotted Keyblade out of its
/// stock (`0x1403C3F00` / `0x1403C49C0` around the form record's `+0` write, `0x1402FC7C0`);
/// KH1 counts the copies equipped when it reads (`0x1402E8770`), so `valorslot` takes this one off
/// there (t967).
pub unsafe fn valor_holds(base: usize) -> Option<u8> {
    let stored = save(base).map(|s| rd::<u8>(s + VALOR_OFF)).unwrap_or(0);
    (stored != 0 && is_keyblade(base, stored) && free(base, stored) > 0).then_some(stored)
}

/// The Keyblade Valor uses (t966, round 11 answer 1 (a)): the slot's; with the slot empty, Sora's
/// own (KH2's weapon resolver `0x1403DFEB0`, case 0x23B: the form's weapon, else `0x140405190(0)`,
/// Sora's). `None` only when Sora holds no Keyblade (a dream weapon or the wooden sword).
pub unsafe fn valor_keyblade(base: usize) -> Option<u8> {
    valor_holds(base).or_else(|| sora_own(base))
}

/// Sora's own weapon when it is a Keyblade (an empty form slot's Keyblade).
unsafe fn sora_own(base: usize) -> Option<u8> {
    sora_weapon(base).filter(|&w| is_keyblade(base, w))
}

/// The Keyblade Master's slot holds a copy of (t1025): the id stored (`MASTER_OFF`) while it is a
/// Keyblade Sora owns with a copy free of the members and of Valor's slot (Valor's slot keeps its
/// copy first when both name the same Keyblade, so neither depends on the other twice).
pub unsafe fn master_holds(base: usize) -> Option<u8> {
    let stored = save(base).map(|s| rd::<u8>(s + MASTER_OFF)).unwrap_or(0);
    let taken = (valor_holds(base) == Some(stored)) as u32;
    (stored != 0 && is_keyblade(base, stored) && free(base, stored) > taken).then_some(stored)
}

/// The Keyblade Master uses (t1025): the slot's; with the slot empty, Sora's own (KH2's resolver
/// `0x1403DFEB0`, case 0x23B, as for Valor; the owner's round 11 answer 1 (a)).
pub unsafe fn master_keyblade(base: usize) -> Option<u8> {
    master_holds(base).or_else(|| sora_own(base))
}

/// The Keyblade Final's slot holds a copy of (t1038): the id stored (`FINAL_OFF`) while it is a
/// Keyblade Sora owns with a copy free of the members and of Valor's and Master's slots (theirs
/// keep their copies first, as Valor's does before Master's).
pub unsafe fn final_holds(base: usize) -> Option<u8> {
    let stored = save(base).map(|s| rd::<u8>(s + FINAL_OFF)).unwrap_or(0);
    let taken = (valor_holds(base) == Some(stored)) as u32 + (master_holds(base) == Some(stored)) as u32;
    (stored != 0 && is_keyblade(base, stored) && free(base, stored) > taken).then_some(stored)
}

/// The Keyblade Final uses in its left hand (t1038): the slot's; with the slot empty, Sora's own
/// (KH2's resolver `0x1403DFEB0`, case 0x23B, as for Valor and Master; the owner's round 11 answer
/// 1 (a), carried for Final). Hand 0 is Sora's own in every form (`0x1403D59C0`, Checked (decomp)),
/// so an empty F: is Sora's own Keyblade in both hands.
pub unsafe fn final_keyblade(base: usize) -> Option<u8> {
    final_holds(base).or_else(|| sora_own(base))
}

/// The copy a form's slot holds (t1025; Final t1038).
pub unsafe fn slot_holds(base: usize, kind: super::form::Kind) -> Option<u8> {
    match kind {
        super::form::Kind::Valor => valor_holds(base),
        super::form::Kind::Master => master_holds(base),
        super::form::Kind::Final => final_holds(base),
        _ => None,
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
