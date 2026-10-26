//! Valor Form's Keyblade in KH1's Equipment page (`docs/kh1-equipment.md`, section 4): Sora's
//! slots get a row under his weapon, "V: <Keyblade>", the Keyblade Valor uses (`equip::
//! valor_keyblade`, kept at `save + equip::VALOR_OFF`). Picking it opens KH1's own weapon list,
//! narrowed to the Keyblades Valor may take; the pick is stored and Sora's own weapon is left
//! as it was.
//!
//! - The slots window's update (its definition's first qword, `0x1405491A0`) is ours: it gives
//!   Sora's window one more row (KH1 makes it accessories + 1) and runs KH1's.
//! - KH1 reads the item of slot row `n >= 1` as `[r12 + n + 0x1C]` (`r12` = Sora's record):
//!   `r12` is set at `0x140303C15`, now a jump to a stub that points it at a shadow of ours for
//!   Sora (row 1: the Valor Keyblade, rows 2..: his accessories).
//! - Confirm's row test (`0x140303CA4`: row 0 the weapon list, else an accessory list) goes
//!   through a stub: on Sora's row 1 it takes the weapon list's way and marks a Valor pick (or
//!   buzzes when no Keyblade is free); rows 2.. are accessories one lower (fixed after KH1's
//!   update).
//! - The weapon list's fill (`0x140304E50`) keeps only the free Keyblades during a Valor pick;
//!   the list window (`0x140549140`) keeps KH1's live preview (the hovered Keyblade loaded into
//!   Sora's hand, `0x1402845A0`, which also writes his weapon byte), and its confirm is undone
//!   for Sora and stored for Valor. KH1 puts his own weapon back itself once the list closes:
//!   the list's close handler (`0x1403033D0`) opens a window (`0x140302AF0`) that loads the
//!   chosen weapon again when the one loaded differs, and writes his weapon byte.
//! - While the preview may have changed Sora's weapon byte (`PICK_WEAPON`), the Valor slot isn't
//!   re-decided and the free Keyblades are counted with his own weapon.
//! - The row's name (`call 0x1402E6570` at `0x140304F74`, in KH1's row drawing) is our "V: "
//!   label while the slots window or the page header draws the Valor Keyblade.
//! - The help line (`0x140549080`) shows the Valor Keyblade's help on its row and the right
//!   accessory's on the others.

use super::addr::Rva;
use super::{equip, hook, log};
use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering};

// Window definitions (their first qword is the update function).
const SLOTS_DEF: Rva = Rva(0x5491A0);
const SLOTS_UPDATE: Rva = Rva(0x303BE0);
const MEMBERS_DEF: Rva = Rva(0x5491D0);
const MEMBERS_UPDATE: Rva = Rva(0x304100);
const HELP_DEF: Rva = Rva(0x549080);
const HELP_UPDATE: Rva = Rva(0x303B30);
const LIST_DEF: Rva = Rva(0x549140);
const LIST_UPDATE: Rva = Rva(0x303090);
const PAGE_DEF: Rva = Rva(0x549200);
const PAGE_UPDATE: Rva = Rva(0x304360);

/// `imul r12, rsi, 0x74; add r12, rdx` in the slots update, and where it goes on.
const R12_SITE: Rva = Rva(0x303C15);
const R12_BYTES: [u8; 7] = [0x4C, 0x6B, 0xE6, 0x74, 0x4C, 0x03, 0xE2];
const R12_BACK: Rva = Rva(0x303C1C);
/// `test ebx, ebx; jne <accessory>` (the confirmed row), the weapon way after it, the accessory
/// way, and the update's drawing part (where a refused confirm goes).
const ROW_SITE: Rva = Rva(0x303CA4);
const WEAPON_WAY: Rva = Rva(0x303CAC);
const ACCESSORY_WAY: Rva = Rva(0x303E4E);
const DRAW_PART: Rva = Rva(0x303F87);
/// KH1's menu sound (4: refused).
const SOUND_FN: Rva = Rva(0x177F60);
/// The row name call in `0x140304F30`, and KH1's item name function.
const NAME_CALL: Rva = Rva(0x304F74);
const NAME_FN: Rva = Rva(0x2E6570);
/// The weapon list's fill and its first bytes (three `mov [rsp+..], reg`).
const FILL_FN: Rva = Rva(0x304E50);
const FILL_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x48, 0x89, 0x7C, 0x24, 0x18];
/// The pause menu's party stats refresh.
const REFRESH_FN: Rva = Rva(0x2E5A80);

// The Equipment page's state.
const STATE: Rva = Rva(0x2E9AA40); // 0 members, 1 slots, 2 a list open, 3 changed
const SLOT: Rva = Rva(0x2E9AA44); // the party slot chosen
const RECORD: Rva = Rva(0x2E9AA50); // its record + 4 (u64)
const ACC_INDEX: Rva = Rva(0x2E9AA4C); // the accessory slot being set
const ROW_PICKED: Rva = Rva(0x2E9AA58);
const CURRENT: Rva = Rva(0x2E9AA5C); // the item being replaced (header, compare box)
const NEW_ITEM: Rva = Rva(0x2E9AA60);
const CURSOR_SLOT: Rva = Rva(0x2E9AA64); // the members' cursor (the slot shown)
const SCROLL: Rva = Rva(0x2E9AA68); // the slots' first visible row
const CURSOR: Rva = Rva(0x2E9AA6C); // the slots' cursor row (visible)
const LIST_SCROLL: Rva = Rva(0x2E9AA70);
const LIST_CURSOR: Rva = Rva(0x2E9AA74);
const LOADED: Rva = Rva(0x2E9AA80); // per slot: the weapon model loaded
const CHOSEN: Rva = Rva(0x2E9AA90); // per slot: the weapon chosen
#[allow(dead_code)]
const PREVIEW: Rva = Rva(0x2E9AAA8); // 1: put the hovered weapon in his hand (KH1's preview steps)
const LIST: Rva = Rva(0x2E9AAB0); // ints
const LIST_COUNTS: Rva = Rva(0x2E9AEB0); // ints
const LIST_LEN: Rva = Rva(0x2E9B2B0);
const ROW_H: Rva = Rva(0x546050);
const ROW_GAP: Rva = Rva(0x546058);

const SAVE_PARTY: usize = 0x48E;
const ACC_COUNT: usize = 0x1C;
const ACC_FIRST: usize = 0x1D;
const WEAPON: usize = 0x36;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_SLOTS: AtomicUsize = AtomicUsize::new(0);
static ORIG_MEMBERS: AtomicUsize = AtomicUsize::new(0);
static ORIG_HELP: AtomicUsize = AtomicUsize::new(0);
static ORIG_LIST: AtomicUsize = AtomicUsize::new(0);
static ORIG_PAGE: AtomicUsize = AtomicUsize::new(0);
static ORIG_FILL: AtomicUsize = AtomicUsize::new(0);
/// Our memory near the game: the stubs, their cells and the shadow record.
static MEM: AtomicUsize = AtomicUsize::new(0);
/// While our slots / page wrappers run: label the Valor Keyblade's name.
static LABEL_ON: AtomicU8 = AtomicU8::new(0);
/// The id the label was made for, and the label (KH1 text codes).
static LABEL_ID: AtomicU8 = AtomicU8::new(0);
static LABEL_BUF: AtomicUsize = AtomicUsize::new(0);
const LABEL_LEN: usize = 64;
/// The last Valor Keyblade logged / Sora rows count logged (logs on change only).
static LOGGED_ID: AtomicU32 = AtomicU32::new(u32::MAX);
static LOGGED_ROWS: AtomicU32 = AtomicU32::new(u32::MAX);
/// Sora's own weapon while a Valor pick's preview may have put another in his weapon byte
/// (0: none): kept from the list's opening until his byte holds it again with the list closed.
static PICK_WEAPON: AtomicU32 = AtomicU32::new(0);
/// The hovered Keyblade last logged during a Valor pick, and how many were logged.
static LOGGED_HOVER: AtomicU32 = AtomicU32::new(u32::MAX);
static HOVER_LOGS: AtomicU32 = AtomicU32::new(0);

// Layout of `MEM`.
const STUB_R12: usize = 0x00;
const STUB_ROW: usize = 0x40;
/// u64: the shadow record's address while Sora's slots show the Valor row, else 0.
const CELL_SHADOW: usize = 0xA0;
/// u64: 0 no Valor row, 1 a Valor row with a Keyblade, 2 a Valor row with none.
const CELL_ROW: usize = 0xA8;
/// u8: set by the row stub when Sora's row 1 is confirmed (a Valor pick is under way).
const CELL_PICK: usize = 0xB0;
const SHADOW: usize = 0xC0;
const MEM_SIZE: usize = 0x100;

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}
fn base() -> usize {
    BASE.load(Ordering::Relaxed)
}
fn mem() -> usize {
    MEM.load(Ordering::Acquire)
}
unsafe fn g<T: Copy>(r: Rva) -> T {
    rd::<T>(base() + r)
}
unsafe fn s<T: Copy>(r: Rva, v: T) {
    wr(base() + r, v)
}

/// An absolute `jmp [rip+0]; dq to`.
fn jmp_abs(out: &mut Vec<u8>, to: usize) {
    out.extend_from_slice(&[0xFF, 0x25, 0, 0, 0, 0]);
    out.extend_from_slice(&(to as u64).to_le_bytes());
}

fn rel32(from_end: usize, to: usize) -> [u8; 4] {
    ((to as i64 - from_end as i64) as i32).to_le_bytes()
}

/// The `r12` stub at `at`: KH1's two instructions, then for member 0 (`esi`) the shadow when
/// the cell holds one; back to `back`.
pub(crate) fn stub_r12(at: usize, cell: usize, back: usize) -> Vec<u8> {
    let mut b = R12_BYTES.to_vec();
    b.extend_from_slice(&[0x85, 0xF6, 0x75, 0x0E]); // test esi, esi; jnz +14
    b.extend_from_slice(&[0x48, 0x8B, 0x05]); // mov rax, [rip + cell]
    let end = at + b.len() + 4;
    b.extend_from_slice(&rel32(end, cell));
    b.extend_from_slice(&[0x48, 0x85, 0xC0, 0x4C, 0x0F, 0x45, 0xE0]); // test rax, rax; cmovnz r12, rax
    jmp_abs(&mut b, back);
    b
}

/// The row stub at `at` (entered with `ebx` = the confirmed row): row 0 the weapon way; row 1
/// with the row cell 1: the weapon way with the pick cell set; with 2: sound 4, then the
/// drawing part; else the accessory way.
pub(crate) fn stub_row(at: usize, row_cell: usize, pick_cell: usize, weapon: usize, accessory: usize, draw: usize, sound: usize) -> Vec<u8> {
    let mut b = vec![
        0x85, 0xDB, // 0: test ebx, ebx
        0x74, 30, // 2: jz weapon (34)
        0x83, 0xFB, 0x01, // 4: cmp ebx, 1
        0x75, 39, // 7: jne accessory (48)
        0x48, 0x8B, 0x05, // 9: mov rax, [rip + row_cell]
    ];
    b.extend_from_slice(&rel32(at + 16, row_cell));
    b.extend_from_slice(&[0x48, 0x85, 0xC0]); // 16: test rax, rax
    b.extend_from_slice(&[0x74, 27]); // 19: jz accessory (48)
    b.extend_from_slice(&[0x48, 0x83, 0xF8, 0x02]); // 21: cmp rax, 2
    b.extend_from_slice(&[0x74, 35]); // 25: je refused (62)
    b.extend_from_slice(&[0xC6, 0x05]); // 27: mov byte [rip + pick_cell], 1
    b.extend_from_slice(&rel32(at + 34, pick_cell));
    b.push(1);
    debug_assert_eq!(b.len(), 34);
    jmp_abs(&mut b, weapon); // 34
    jmp_abs(&mut b, accessory); // 48
    b.extend_from_slice(&[0xB9, 4, 0, 0, 0]); // 62: mov ecx, 4
    b.extend_from_slice(&[0x48, 0xB8]); // mov rax, sound
    b.extend_from_slice(&(sound as u64).to_le_bytes());
    b.extend_from_slice(&[0xFF, 0xD0]); // call rax
    jmp_abs(&mut b, draw);
    b
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    // Everything checked before anything changes.
    let defs = [(SLOTS_DEF, SLOTS_UPDATE), (MEMBERS_DEF, MEMBERS_UPDATE), (HELP_DEF, HELP_UPDATE), (LIST_DEF, LIST_UPDATE), (PAGE_DEF, PAGE_UPDATE)];
    for (def, f) in defs {
        let have = rd::<u64>(base + def) as usize;
        if have != base + f {
            log!(Menus, "Equipment: window definition {:#x} holds {have:#x}, not {:#x}: no Valor row", base + def, base + f);
            return;
        }
    }
    let row_site = base + ROW_SITE;
    let row_old = {
        let mut b = vec![0x85, 0xDB, 0x0F, 0x85];
        b.extend_from_slice(&rel32(row_site + 8, base + ACCESSORY_WAY));
        b
    };
    let checks: [(usize, &[u8], &str); 3] =
        [(base + R12_SITE, &R12_BYTES, "slot record"), (row_site, &row_old, "slot row test"), (base + FILL_FN, &FILL_PROLOGUE, "weapon list fill")];
    for (at, want, what) in checks {
        let have = std::slice::from_raw_parts(at as *const u8, want.len());
        if have != want {
            log!(Menus, "Equipment: {what} at {at:#x} is {have:02x?}, not {want:02x?}: no Valor row");
            return;
        }
    }
    let call = base + NAME_CALL;
    if rd::<u8>(call) != 0xE8 || (call as i64 + 5 + rd::<i32>(call + 1) as i64) as usize != base + NAME_FN {
        log!(Menus, "Equipment: no call to the item name at {call:#x}: no Valor row");
        return;
    }
    let Some(m) = hook::alloc_near(base + R12_SITE, MEM_SIZE) else {
        log!(Menus, "Equipment: no memory near the game: no Valor row");
        return;
    };
    std::ptr::write_bytes(m as *mut u8, 0, MEM_SIZE);
    let a = stub_r12(m + STUB_R12, m + CELL_SHADOW, base + R12_BACK);
    std::ptr::copy_nonoverlapping(a.as_ptr(), (m + STUB_R12) as *mut u8, a.len());
    let b = stub_row(m + STUB_ROW, m + CELL_ROW, m + CELL_PICK, base + WEAPON_WAY, base + ACCESSORY_WAY, base + DRAW_PART, base + SOUND_FN);
    std::ptr::copy_nonoverlapping(b.as_ptr(), (m + STUB_ROW) as *mut u8, b.len());
    MEM.store(m, Ordering::Release);
    let label = vec![0u8; LABEL_LEN].into_boxed_slice();
    LABEL_BUF.store(Box::leak(label).as_mut_ptr() as usize, Ordering::Release);

    // Wrappers first (they only add to KH1's), then the code patches.
    ORIG_SLOTS.store(base + SLOTS_UPDATE, Ordering::Release);
    ORIG_MEMBERS.store(base + MEMBERS_UPDATE, Ordering::Release);
    ORIG_HELP.store(base + HELP_UPDATE, Ordering::Release);
    ORIG_LIST.store(base + LIST_UPDATE, Ordering::Release);
    ORIG_PAGE.store(base + PAGE_UPDATE, Ordering::Release);
    if hook::install(base + FILL_FN, &FILL_PROLOGUE, fill as *const () as usize, &ORIG_FILL, "Equipment weapon list fill").is_none() {
        return;
    }
    let ours: [(Rva, Rva, usize); 5] = [
        (SLOTS_DEF, SLOTS_UPDATE, slots_update as *const () as usize),
        (MEMBERS_DEF, MEMBERS_UPDATE, members_update as *const () as usize),
        (HELP_DEF, HELP_UPDATE, help_update as *const () as usize),
        (LIST_DEF, LIST_UPDATE, list_update as *const () as usize),
        (PAGE_DEF, PAGE_UPDATE, page_update as *const () as usize),
    ];
    let mut ok = true;
    for (def, f, w) in ours {
        ok &= hook::patch(base + def, &((base + f) as u64).to_le_bytes(), &(w as u64).to_le_bytes(), "Equipment window update");
    }
    ok &= hook::redirect_call(call, base + NAME_FN, item_name as *const () as usize, "Equipment row name");
    // The two jumps last: until they are in, KH1's code runs as it was.
    let mut j = vec![0xE9];
    j.extend_from_slice(&rel32(base + R12_SITE + 5, m + STUB_R12));
    j.extend_from_slice(&[0x90, 0x90]);
    ok &= hook::patch(base + R12_SITE, &R12_BYTES, &j, "Equipment slot record");
    let mut j = vec![0xE9];
    j.extend_from_slice(&rel32(row_site + 5, m + STUB_ROW));
    j.extend_from_slice(&[0x90, 0x90, 0x90]);
    ok &= hook::patch(row_site, &row_old, &j, "Equipment slot row test");
    log!(Menus, "Equipment: Valor row for Sora{} (stubs at {m:#x})", if ok { "" } else { ", some patches failed (see above)" });
}

/// The save block.
unsafe fn save() -> Option<usize> {
    super::menu::save_block(base())
}

/// Sora's record is the one shown in the slots (`slot`: a party slot 0..3).
unsafe fn sora_in(save: usize, slot: i32) -> bool {
    (0..4).contains(&slot) && rd::<u8>(save + SAVE_PARTY + slot as usize) == 0
}

/// During a Valor pick whose preview has put another Keyblade in Sora's weapon byte: (his own
/// weapon, the hovered one). For the stats shown then (`valorstats::add`).
pub unsafe fn valor_preview() -> Option<(u8, u8, u8)> {
    let own = PICK_WEAPON.load(Ordering::Relaxed) as u8;
    if own == 0 {
        return None;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let save = super::menu::save_block(base)?;
    let now = rd::<u8>(save + WEAPON);
    // (Sora's own, the one KH1's preview has in his byte (a hover behind, or still his own), and
    // the one under the cursor: s223 took the byte for the hovered one and did nothing when it
    // was his own, the stats then his with his own Keyblade alone, the user: "the strength cap
    // is at 70 again".)
    let hovered = LOGGED_HOVER.load(Ordering::Relaxed);
    let hovered = if (1..=255).contains(&hovered) { hovered as u8 } else { now };
    (now != 0).then_some((own, now, hovered))
}

/// Sora's own weapon while a Valor pick's preview may hold another in his weapon byte; the
/// guard ends once the list is closed and his byte is his own again (KH1's restore).
unsafe fn pick_guard(save: usize) -> Option<u8> {
    let w = PICK_WEAPON.load(Ordering::Relaxed);
    if w == 0 {
        return None;
    }
    if g::<i32>(STATE) != 2 && rd::<u8>(save + WEAPON) as u32 == w {
        PICK_WEAPON.store(0, Ordering::Relaxed);
        log!(Menus, "Equipment: Sora's own weapon {w:#04x} back after the Valor pick");
        return None;
    }
    Some(w as u8)
}

/// `f` with Sora's own weapon in his weapon byte (the preview's put back after).
unsafe fn with_real_weapon<R>(save: usize, f: impl FnOnce() -> R) -> R {
    let Some(w) = pick_guard(save) else { return f() };
    let now = rd::<u8>(save + WEAPON);
    wr(save + WEAPON, w);
    let r = f();
    wr(save + WEAPON, now);
    r
}

/// The Valor Keyblade (0 for none): `valor::keyblade`, which keeps `save + VALOR_OFF` up to
/// date (the slot refilled when its Keyblade stops being free); during a pick, the stored one
/// as it is (the preview's Keyblade in Sora's byte would look equipped).
unsafe fn valor_id(save: usize) -> u8 {
    let id = if pick_guard(save).is_some() {
        rd::<u8>(save + equip::VALOR_OFF)
    } else {
        super::valor::keyblade().unwrap_or(0)
    };
    if LOGGED_ID.swap(id as u32, Ordering::Relaxed) != id as u32 {
        log!(Menus, "Equipment: Valor row shows {id:#04x} (candidates {:02x?})", equip::valor_candidates(base()));
    }
    id
}

/// Sora's slots window with the Valor row: `accessories + 2` rows (KH1 makes `+ 1`), visible
/// rows and height as KH1 works them out (8 at most).
unsafe fn fix_rows(win: usize, acc: u16) {
    if win < 0x10000 || !super::crash::readable(win + 0x100) {
        return;
    }
    let rows = rd::<u16>(win + 0xF8);
    if rows != acc + 1 {
        return;
    }
    let want = acc + 2;
    let vis = want.min(8);
    let (h, gap) = (g::<i32>(ROW_H) as i16, g::<i32>(ROW_GAP) as i16);
    wr(win + 0xF8, want);
    wr(win + 0xFA, vis);
    wr(win + 0xE6, ((h + gap) * vis as i16 - gap) * 16);
    if LOGGED_ROWS.swap(want as u32, Ordering::Relaxed) != want as u32 {
        log!(Menus, "Equipment: Sora's slots {} rows (weapon, Valor, {acc} accessories)", want);
    }
}

/// Before KH1's slots update: rows, the shadow record and the row cell for this frame.
unsafe fn prepare(save: usize, win: usize) -> bool {
    let m = mem();
    let slot = g::<i32>(CURSOR_SLOT);
    if m == 0 || !sora_in(save, slot) {
        if m != 0 {
            wr(m + CELL_SHADOW, 0u64);
            wr(m + CELL_ROW, 0u64);
        }
        return false;
    }
    let acc = rd::<u8>(save + ACC_COUNT).min(8);
    if win != 0 {
        fix_rows(win, acc as u16);
    }
    let id = valor_id(save);
    let sh = m + SHADOW;
    std::ptr::write_bytes(sh as *mut u8, 0, 0x40);
    wr(sh + ACC_FIRST, id);
    for i in 0..acc as usize {
        wr(sh + ACC_FIRST + 1 + i, rd::<u8>(save + ACC_FIRST + i));
    }
    wr(m + CELL_SHADOW, sh as u64);
    wr(m + CELL_ROW, if id != 0 { 1u64 } else { 2 });
    true
}

/// After a KH1 update that may have opened a list from Sora's slots: an accessory row is one
/// lower than KH1 took it; a Valor pick replaces Sora's weapon as the item being replaced.
unsafe fn after(save: usize) {
    let m = mem();
    if g::<i32>(STATE) != 2 {
        if rd::<u8>(m + CELL_PICK) != 0 {
            wr(m + CELL_PICK, 0u8);
        }
        pick_guard(save);
        return;
    }
    if rd::<u8>(m + CELL_PICK) != 0 {
        let id = rd::<u8>(m + SHADOW + ACC_FIRST);
        if g::<i32>(CURRENT) != id as i32 {
            s(CURRENT, id as i32);
            log!(Menus, "Equipment: Valor row picked (Keyblade {id:#04x}): KH1's weapon list for Valor");
        }
        return;
    }
    let row = g::<i32>(ROW_PICKED);
    if row >= 2 && g::<i32>(ACC_INDEX) == row - 1 {
        let k = row - 2;
        s(ACC_INDEX, k);
        s(CURRENT, rd::<u8>(save + ACC_FIRST + k as usize) as i32);
        log!(Menus, "Equipment: Sora's row {row} is accessory slot {k}");
    }
}

unsafe extern "system" fn slots_update(win: usize) {
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_SLOTS.load(Ordering::Acquire));
    let Some(save) = save() else { return f(win) };
    if !prepare(save, win) {
        return f(win);
    }
    LABEL_ON.store(1, Ordering::Relaxed);
    f(win);
    LABEL_ON.store(0, Ordering::Relaxed);
    after(save);
}

/// The members' window sets its slots child's rows as its cursor moves (`0x1403041A5`): fixed
/// right after for Sora.
unsafe extern "system" fn members_update(win: usize) {
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_MEMBERS.load(Ordering::Acquire));
    f(win);
    let Some(save) = save() else { return };
    if !sora_in(save, g::<i32>(CURSOR_SLOT)) || !super::crash::readable(win + 0x28) {
        return;
    }
    let c = rd::<u64>(win + 0x20) as usize;
    if c < 0x10000 || !super::crash::readable(c + 0x28) {
        return;
    }
    let slots = rd::<u64>(c + 0x20) as usize;
    fix_rows(slots, rd::<u8>(save + ACC_COUNT).min(8) as u16);
}

/// The page: its header names the item being replaced (labelled when it is Valor's), and its
/// rebuild of an open list (state 2) takes the accessory row as KH1 counts it.
unsafe extern "system" fn page_update(win: usize) {
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_PAGE.load(Ordering::Acquire));
    let Some(save) = save() else { return f(win) };
    let m = mem();
    let sora = m != 0 && sora_in(save, g::<i32>(SLOT));
    LABEL_ON.store(sora as u8, Ordering::Relaxed);
    f(win);
    LABEL_ON.store(0, Ordering::Relaxed);
    if sora && rd::<u64>(m + CELL_ROW) != 0 {
        after(save);
    }
}

/// The slots' help line: KH1 takes row 0 as the weapon and row n as accessory n - 1 (from the
/// visible cursor only). For Sora: row 1 is the Valor Keyblade (shown through accessory 0 for
/// the call), row n >= 2 accessory n - 2.
unsafe extern "system" fn help_update(win: usize) {
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_HELP.load(Ordering::Acquire));
    let m = mem();
    let Some(save) = save() else { return f(win) };
    if m == 0 || rd::<u64>(m + CELL_ROW) == 0 || g::<u64>(RECORD) as usize != save + 4 || g::<i32>(STATE) != 1 {
        return f(win);
    }
    let (scroll, cursor) = (g::<i32>(SCROLL), g::<i32>(CURSOR));
    let row = scroll + cursor;
    if row == 0 {
        return f(win);
    }
    s(SCROLL, 0i32);
    if row == 1 {
        let acc0 = rd::<u8>(save + ACC_FIRST);
        wr(save + ACC_FIRST, rd::<u8>(m + SHADOW + ACC_FIRST));
        s(CURSOR, 1i32);
        f(win);
        wr(save + ACC_FIRST, acc0);
    } else {
        s(CURSOR, row - 1);
        f(win);
    }
    s(SCROLL, scroll);
    s(CURSOR, cursor);
}

/// KH1's weapon list fill; during a Valor pick only the Keyblades Valor may take stay, the
/// cursor on the current one.
unsafe extern "system" fn fill() {
    let f: unsafe extern "system" fn() = std::mem::transmute(ORIG_FILL.load(Ordering::Acquire));
    f();
    let m = mem();
    if m == 0 || rd::<u8>(m + CELL_PICK) == 0 {
        return;
    }
    // (Sora's own weapon noted as the list opens, before any preview; a rebuild of the open
    // list by the page counts with it too.)
    if let Some(save) = save() {
        if PICK_WEAPON.load(Ordering::Relaxed) == 0 {
            PICK_WEAPON.store(rd::<u8>(save + WEAPON) as u32, Ordering::Relaxed);
            HOVER_LOGS.store(0, Ordering::Relaxed);
            LOGGED_HOVER.store(u32::MAX, Ordering::Relaxed);
        }
    }
    let keep = match save() {
        Some(save) => with_real_weapon(save, || equip::valor_candidates(base())),
        None => equip::valor_candidates(base()),
    };
    let current = rd::<u8>(m + SHADOW + ACC_FIRST) as i32;
    let n = g::<i32>(LIST_LEN).clamp(0, 256) as usize;
    let (list, counts) = (base() + LIST, base() + LIST_COUNTS);
    let before: Vec<i32> = (0..n).map(|i| rd::<i32>(list + i * 4)).collect();
    let mut k = 0;
    for i in 0..n {
        let id = rd::<i32>(list + i * 4);
        if (1..=255).contains(&id) && keep.contains(&(id as u8)) {
            let c = rd::<i32>(counts + i * 4);
            wr(list + k * 4, id);
            wr(counts + k * 4, c);
            k += 1;
        }
    }
    s(LIST_LEN, k as i32);
    let at = (0..k).find(|&i| rd::<i32>(list + i * 4) == current).unwrap_or(0) as i32;
    let (top, cur) = if (k as i32) < 7 { (0, at) } else { let t = at.min(k as i32 - 7); (t, at - t) };
    s(LIST_SCROLL, top);
    s(LIST_CURSOR, cur);
    log!(Menus, "Equipment: weapon list for Valor: {before:02x?} -> {:02x?}", (0..k).map(|i| rd::<i32>(list + i * 4)).collect::<Vec<_>>());
}

/// The weapon list during a Valor pick: KH1's live preview shows the hovered Keyblade in
/// Sora's hand (KH1 loads his own back once the list closes); a confirmed Keyblade goes to
/// Valor, Sora keeping his own.
unsafe extern "system" fn list_update(win: usize) {
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_LIST.load(Ordering::Acquire));
    let m = mem();
    let save = save();
    let (Some(save), true) = (save, m != 0 && rd::<u8>(m + CELL_PICK) != 0) else { return f(win) };
    let slot = g::<i32>(SLOT).clamp(0, 3) as usize;
    let (loaded_at, chosen_at) = (base() + LOADED + slot * 4, base() + CHOSEN + slot * 4);
    let chosen = rd::<i32>(chosen_at);
    let weapon = match pick_guard(save) {
        Some(w) => w,
        None => rd::<u8>(save + WEAPON),
    };
    f(win);
    // What the list shows (logged as the cursor moves, a few times a pick).
    let n = g::<i32>(LIST_LEN);
    let at = g::<i32>(LIST_SCROLL) + g::<i32>(LIST_CURSOR);
    let hovered = if (0..n.min(256)).contains(&at) { rd::<i32>(base() + LIST + at as usize * 4) } else { 0 };
    if LOGGED_HOVER.swap(hovered as u32, Ordering::Relaxed) != hovered as u32 && HOVER_LOGS.fetch_add(1, Ordering::Relaxed) < 24 {
        let name = if (1..=255).contains(&hovered) { equip::item_name(base(), hovered as u8).unwrap_or_default() } else { String::new() };
        log!(Menus,
            "Equipment: Valor pick hovers {hovered:#04x} {name} (row {at} of {n}; preview loads {:#04x}, Sora's byte {:#04x}, his own {weapon:#04x})",
            rd::<i32>(loaded_at),
            rd::<u8>(save + WEAPON)
        );
    }
    if g::<i32>(STATE) == 3 {
        let picked = g::<i32>(NEW_ITEM);
        // Sora keeps his weapon: chosen and his byte as they were. The model loaded (the
        // preview's) is left for KH1's restore to swap back as the list closes.
        wr(chosen_at, chosen);
        wr(save + WEAPON, weapon);
        let refresh: unsafe extern "system" fn() = std::mem::transmute(base() + REFRESH_FN);
        refresh();
        wr(m + CELL_PICK, 0u8);
        if (1..=255).contains(&picked) && equip::valor_candidates(base()).contains(&(picked as u8)) {
            wr(save + equip::VALOR_OFF, picked as u8);
            let name = equip::item_name(base(), picked as u8).unwrap_or_default();
            log!(Menus,
                "Equipment: Valor's Keyblade stored: {picked:#04x} {name} (Sora keeps {weapon:#04x}; model loaded {:#04x}, put back by KH1)",
                rd::<i32>(loaded_at)
            );
        } else {
            log!(Menus, "Equipment: Valor pick {picked:#x} isn't a free Keyblade: not stored (Sora keeps {weapon:#04x})");
        }
        pick_guard(save);
    }
}

/// The name of a row's item (`0x140304F30`): "V: " and the Keyblade's name on the Valor row
/// (while the slots or the page header draw for Sora), KH1's otherwise.
unsafe extern "system" fn item_name(item: u32) -> usize {
    let f: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base() + NAME_FN);
    let kh1 = f(item);
    let m = mem();
    if LABEL_ON.load(Ordering::Relaxed) == 0 || m == 0 || kh1 == 0 || item == 0 || item > 255 {
        return kh1;
    }
    let id = rd::<u8>(m + SHADOW + ACC_FIRST);
    if id as u32 != item || rd::<u64>(m + CELL_ROW) != 1 {
        return kh1;
    }
    let buf = LABEL_BUF.load(Ordering::Acquire);
    if buf == 0 {
        return kh1;
    }
    if LABEL_ID.load(Ordering::Relaxed) != id {
        let mut b = super::menu::encode("V: ");
        b.pop();
        for i in 0..40 {
            let c = rd::<u8>(kh1 + i);
            if c == 0 {
                break;
            }
            b.push(c);
        }
        b.truncate(LABEL_LEN - 1);
        b.push(0);
        std::ptr::copy_nonoverlapping(b.as_ptr(), buf as *mut u8, b.len());
        LABEL_ID.store(id, Ordering::Relaxed);
    }
    buf
}
