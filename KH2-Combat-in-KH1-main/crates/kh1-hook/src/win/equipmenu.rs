//! Valor's, Master's and Final Form's Keyblades in KH1's Equipment page (`docs/kh1-equipment.md`,
//! section 4): Sora's slots get a row under his weapon for each of the three forms unlocked
//! (`form::unlocked`), in KH2's order: "V: <Keyblade>" (`equip::valor_keyblade`, kept at `save +
//! equip::VALOR_OFF`), "M: <Keyblade>" (`equip::master_keyblade`, `save + equip::MASTER_OFF`),
//! then "F: <Keyblade>" (`equip::final_keyblade`, kept at `save + equip::FINAL_OFF`; the user:
//! "add a section in the equipment menu for the second keyblade in final form"). Picking one
//! opens KH1's own weapon list, narrowed to the Keyblades the form may take (owned, a copy not
//! equipped and not another slot's); the pick is stored in that form's slot and Sora's own weapon
//! is left as it was.
//!
//! - Valor's row always offers the Dream Sword (`equip::VALOR_DEFAULT`, Valor's default, the
//!   user: "whenever the player gets valor form, i want the Dream Sword to be in the second
//!   keyblade slot by default"), first in its list beside the free Keyblades, though Sora doesn't
//!   own it (KH1's fill lists only what he owns, so it is put in: `fill`); picking it holds no
//!   copy. Master's and Final's rows don't offer it.
//! - A slot's Keyblade is in use like an equipped one (the user: "When you put a keyblade in any
//!   of the drive equipment slots ... it doesn't remove the keyblade from the list"): it leaves
//!   the other slot's list and Sora's own weapon list (`fill_sora`) until the slot changes, and
//!   KH1's free count (`0x1402E6330`, hooked: `free_count`) takes it off for the shops.
//! - The slots window's update (its definition's first qword, `0x1405491A0`) is ours: it gives
//!   Sora's window one more row per form row (KH1 makes it accessories + 1) and runs KH1's.
//! - KH1 reads the item of slot row `n >= 1` as `[r12 + n + 0x1C]` (`r12` = Sora's record):
//!   `r12` is set at `0x140303C15`, now a jump to a stub that points it at a shadow of ours for
//!   Sora (rows 1..=forms: the forms' Keyblades, the rows after: his accessories).
//! - Confirm's row test (`0x140303CA4`: row 0 the weapon list, else an accessory list) goes
//!   through a stub: on Sora's form rows it takes the weapon list's way and marks a pick with the
//!   row (or buzzes when no Keyblade is free); the rows after are accessories `forms` lower
//!   (fixed after KH1's update).
//! - The weapon list's fill (`0x140304E50`) keeps only the free Keyblades during a pick; the list
//!   window (`0x140549140`) keeps KH1's live preview (the hovered Keyblade loaded into Sora's
//!   hand, `0x1402845A0`, which also writes his weapon byte), and its confirm is undone for Sora
//!   and stored for the form. KH1 puts his own weapon back itself once the list closes: the
//!   list's close handler (`0x1403033D0`) opens a window (`0x140302AF0`) that loads the chosen
//!   weapon again when the one loaded differs, and writes his weapon byte.
//! - While the preview may have changed Sora's weapon byte (`PICK_WEAPON`), the slots aren't
//!   re-decided and the free Keyblades are counted with his own weapon.
//! - The row's name (`call 0x1402E6570` at `0x140304F74`, in KH1's row drawing `0x140304F30`) is
//!   our "V: " / "F: " label while the slots window or the page header draws a form's Keyblade.
//!   The two rows may hold the same Keyblade, so the slots' label goes by the row being drawn
//!   (the slots loop's `rbx`, saved by the row drawing's prologue, passed on by a stub).
//! - The help line (`0x140549080`) shows a form row's Keyblade's help and the right accessory's
//!   on the others.

use super::addr::Rva;
use super::{equip, hook, log};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering};

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
/// The weapon compare box (Strength and MP, now -> with the hovered weapon).
const BOX_DEF: Rva = Rva(0x548FC0);
const BOX_UPDATE: Rva = Rva(0x302C90);
/// The compare box's figures "now": Strength and max MP (ints, from the menu's stats copy as
/// the list opens, `0x140303DFD`).
const BOX_STRENGTH: usize = 0x4C;
const BOX_MAX_MP: usize = 0x50;

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
/// KH1's row drawing (a row's name and icon), and its prologue: `rbx` saved at its entry
/// `rsp + 8`, then three pushes and `sub rsp, 0x50`, so at its name call (our stub's entry) the
/// caller's `rbx` (the slots loop's row) is at `rsp + 0x78`.
const ROW_DRAW_FN: Rva = Rva(0x304F30);
const ROW_DRAW_PROLOGUE: [u8; 24] = [
    0x48, 0x89, 0x5C, 0x24, 0x08, // mov [rsp+8], rbx
    0x48, 0x89, 0x6C, 0x24, 0x10, // mov [rsp+0x10], rbp
    0x48, 0x89, 0x74, 0x24, 0x18, // mov [rsp+0x18], rsi
    0x57, 0x41, 0x56, 0x41, 0x57, // push rdi; push r14; push r15
    0x48, 0x83, 0xEC, 0x50, // sub rsp, 0x50
];
const ROW_RBX_AT: u8 = 0x78;
/// The weapon list's fill and its first bytes (three `mov [rsp+..], reg`).
const FILL_FN: Rva = Rva(0x304E50);
const FILL_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x48, 0x89, 0x7C, 0x24, 0x18];
/// The pause menu's party stats refresh.
const REFRESH_FN: Rva = Rva(0x2E5A80);
/// KH1's free count of an item (`0x1402E6330 (id)`, a leaf) and its first bytes (`mov
/// [rsp+8], rbx; mov [rsp+0x10], rdi`; the same in the Steam exe at `0x1402E8770`).
const FREE_FN: Rva = Rva(0x2E6330);
const FREE_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x7C, 0x24, 0x10];

// The Equipment page's state.
const STATE: Rva = Rva(0x2E9AA40); // 0 members, 1 slots, 2 a list open, 3 changed
const SLOT: Rva = Rva(0x2E9AA44); // the party slot chosen
const MEMBER: Rva = Rva(0x2E9AA48); // its member (0 Sora; the weapon list's kind)
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
static ORIG_BOX: AtomicUsize = AtomicUsize::new(0);
/// While KH1's compare box update runs during a pick (its stats call is the what-if).
static COMPARING: AtomicBool = AtomicBool::new(false);
static ORIG_FILL: AtomicUsize = AtomicUsize::new(0);
static ORIG_FREE: AtomicUsize = AtomicUsize::new(0);
/// Our memory near the game: the stubs, their cells and the shadow record.
static MEM: AtomicUsize = AtomicUsize::new(0);
/// While our wrappers run: label the forms' Keyblades' names (`LABEL_SLOTS`: the slots, by the
/// row drawn; `LABEL_PAGE`: the page header).
static LABEL_ON: AtomicU8 = AtomicU8::new(0);
const LABEL_SLOTS: u8 = 1;
const LABEL_PAGE: u8 = 2;
/// Per form (Valor, Final, Master): the id its label was made for, and the label (KH1 text codes;
/// one buffer each, as all the rows may be drawn in one frame).
static LABEL_ID: [AtomicU8; FORMS] = [AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0)];
static LABEL_BUF: [AtomicUsize; FORMS] = [AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0)];
const LABEL_LEN: usize = 64;
/// The last form rows logged / Sora rows count logged (logs on change only).
static LOGGED_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
static LOGGED_ROWS: AtomicU32 = AtomicU32::new(u32::MAX);
/// The form of Sora's rows 1..=3 (`Form` as u8, 0 none), set as the slots are prepared.
static ROW_FORM: [AtomicU8; FORMS] = [AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0)];
/// The forms with a row (and their rows at most).
const FORMS: usize = 3;
/// Sora's own weapon while a pick's preview may have put another in his weapon byte (0: none):
/// kept from the list's opening until his byte holds it again with the list closed.
static PICK_WEAPON: AtomicU32 = AtomicU32::new(0);
/// The hovered Keyblade last logged during a pick, and how many were logged.
static LOGGED_HOVER: AtomicU32 = AtomicU32::new(u32::MAX);
static HOVER_LOGS: AtomicU32 = AtomicU32::new(0);

// Layout of `MEM`.
const STUB_R12: usize = 0x00;
const STUB_ROW: usize = 0x40;
const STUB_NAME: usize = 0xC0;
/// u64: the shadow record's address while Sora's slots show form rows, else 0.
const CELL_SHADOW: usize = 0xE0;
/// u8 each, for Sora's rows 1..=3: 0 not a form row, 1 a form row with Keyblades to choose
/// from, 2 a form row with none (confirm refused).
const CELL_ROWS: usize = 0xE8;
/// u8: the form row confirmed (1..=3; a pick is under way), set by the row stub; else 0.
const CELL_PICK: usize = 0xF0;
const SHADOW: usize = 0x100;
const MEM_SIZE: usize = 0x140;

/// A form with a row (and a slot) of its own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Form {
    Valor = 1,
    Final = 2,
    Master = 3,
}

impl Form {
    fn from(b: u8) -> Option<Form> {
        match b {
            1 => Some(Form::Valor),
            2 => Some(Form::Final),
            3 => Some(Form::Master),
            _ => None,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Form::Valor => "Valor",
            Form::Final => "Final",
            Form::Master => "Master",
        }
    }
    /// Its slot in the save.
    fn off(self) -> usize {
        match self {
            Form::Valor => equip::VALOR_OFF,
            Form::Final => equip::FINAL_OFF,
            Form::Master => equip::MASTER_OFF,
        }
    }
    /// Its row's label before the Keyblade's name (short: the row is `0xA6` px wide).
    fn prefix(self) -> &'static str {
        match self {
            Form::Valor => "V: ",
            Form::Final => "F: ",
            Form::Master => "M: ",
        }
    }
    fn index(self) -> usize {
        self as usize - 1
    }
}

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

/// The row stub at `at` (entered with `ebx` = the confirmed row): row 0 the weapon way; rows 1
/// to 3 by their cell (`rows`, `rows + 1`, `rows + 2`): 1 the weapon way with the pick cell set
/// to the row, 2 sound 4 then the drawing part, 0 (and rows past 3) the accessory way. Only `rax`
/// and the flags change, as before.
pub(crate) fn stub_row(at: usize, rows: usize, pick_cell: usize, weapon: usize, accessory: usize, draw: usize, sound: usize) -> Vec<u8> {
    let mut b = vec![
        0x85, 0xDB, // 0: test ebx, ebx
        0x74, 55, // 2: jz weapon (59)
        0x83, 0xFB, 0x03, // 4: cmp ebx, 3
        0x77, 64, // 7: ja accessory (73; unsigned, so a negative row too)
        0x83, 0xFB, 0x01, // 9: cmp ebx, 1
        0x75, 9, // 12: jne row2 (23)
        0x0F, 0xB6, 0x05, // 14: movzx eax, byte [rip + rows]
    ];
    b.extend_from_slice(&rel32(at + 21, rows));
    b.extend_from_slice(&[0xEB, 21]); // 21: jmp status (44)
    b.extend_from_slice(&[0x83, 0xFB, 0x02]); // 23: cmp ebx, 2
    b.extend_from_slice(&[0x75, 9]); // 26: jne row3 (37)
    b.extend_from_slice(&[0x0F, 0xB6, 0x05]); // 28: movzx eax, byte [rip + rows + 1]
    b.extend_from_slice(&rel32(at + 35, rows + 1));
    b.extend_from_slice(&[0xEB, 7]); // 35: jmp status (44)
    b.extend_from_slice(&[0x0F, 0xB6, 0x05]); // 37: movzx eax, byte [rip + rows + 2]
    b.extend_from_slice(&rel32(at + 44, rows + 2));
    b.extend_from_slice(&[0x85, 0xC0]); // 44: test eax, eax
    b.extend_from_slice(&[0x74, 25]); // 46: jz accessory (73)
    b.extend_from_slice(&[0x83, 0xF8, 0x02]); // 48: cmp eax, 2
    b.extend_from_slice(&[0x74, 34]); // 51: je refused (87)
    b.extend_from_slice(&[0x88, 0x1D]); // 53: mov byte [rip + pick_cell], bl
    b.extend_from_slice(&rel32(at + 59, pick_cell));
    debug_assert_eq!(b.len(), 59);
    jmp_abs(&mut b, weapon); // 59
    jmp_abs(&mut b, accessory); // 73
    debug_assert_eq!(b.len(), 87);
    b.extend_from_slice(&[0xB9, 4, 0, 0, 0]); // 73: mov ecx, 4
    b.extend_from_slice(&[0x48, 0xB8]); // mov rax, sound
    b.extend_from_slice(&(sound as u64).to_le_bytes());
    b.extend_from_slice(&[0xFF, 0xD0]); // call rax
    jmp_abs(&mut b, draw);
    b
}

/// The name stub: the row drawing's caller's `rbx` (at `rsp + rbx_at`; with no offset known,
/// -1) as the second argument, then our `item_name` (the item in `ecx` as KH1 passed it).
pub(crate) fn stub_name(rbx_at: Option<u8>, to: usize) -> Vec<u8> {
    let mut b = match rbx_at {
        Some(o) => vec![0x48, 0x8B, 0x54, 0x24, o], // mov rdx, [rsp + o]
        None => vec![0x48, 0xC7, 0xC2, 0xFF, 0xFF, 0xFF, 0xFF], // mov rdx, -1
    };
    jmp_abs(&mut b, to);
    b
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    // KH1's free count takes the form slots' copies off (on its own: the slots hold their copies
    // with or without the rows).
    hook::install(base + FREE_FN, &FREE_PROLOGUE, free_count as *const () as usize, &ORIG_FREE, "KH1 free item count");
    // Everything checked before anything changes.
    let defs = [(SLOTS_DEF, SLOTS_UPDATE), (MEMBERS_DEF, MEMBERS_UPDATE), (HELP_DEF, HELP_UPDATE), (LIST_DEF, LIST_UPDATE), (PAGE_DEF, PAGE_UPDATE), (BOX_DEF, BOX_UPDATE)];
    for (def, f) in defs {
        let have = rd::<u64>(base + def) as usize;
        if have != base + f {
            log(&format!("Equipment: window definition {:#x} holds {have:#x}, not {:#x}: no form rows", base + def, base + f));
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
            log(&format!("Equipment: {what} at {at:#x} is {have:02x?}, not {want:02x?}: no form rows"));
            return;
        }
    }
    let call = base + NAME_CALL;
    if rd::<u8>(call) != 0xE8 || (call as i64 + 5 + rd::<i32>(call + 1) as i64) as usize != base + NAME_FN {
        log(&format!("Equipment: no call to the item name at {call:#x}: no form rows"));
        return;
    }
    let Some(m) = hook::alloc_near(base + R12_SITE, MEM_SIZE) else {
        log("Equipment: no memory near the game: no form rows");
        return;
    };
    std::ptr::write_bytes(m as *mut u8, 0, MEM_SIZE);
    let a = stub_r12(m + STUB_R12, m + CELL_SHADOW, base + R12_BACK);
    std::ptr::copy_nonoverlapping(a.as_ptr(), (m + STUB_R12) as *mut u8, a.len());
    let b = stub_row(m + STUB_ROW, m + CELL_ROWS, m + CELL_PICK, base + WEAPON_WAY, base + ACCESSORY_WAY, base + DRAW_PART, base + SOUND_FN);
    debug_assert!(STUB_ROW + b.len() <= STUB_NAME);
    std::ptr::copy_nonoverlapping(b.as_ptr(), (m + STUB_ROW) as *mut u8, b.len());
    // (The row drawing's prologue as expected: the slots' label goes by the row; else by the
    // first form row holding the item.)
    let prologue = std::slice::from_raw_parts((base + ROW_DRAW_FN) as *const u8, ROW_DRAW_PROLOGUE.len()) == ROW_DRAW_PROLOGUE;
    if !prologue {
        log(&format!("Equipment: row drawing at {:#x} isn't as expected: the slots' labels go by the item only", base + ROW_DRAW_FN));
    }
    let c = stub_name(prologue.then_some(ROW_RBX_AT), item_name as *const () as usize);
    debug_assert!(STUB_NAME + c.len() <= CELL_SHADOW);
    std::ptr::copy_nonoverlapping(c.as_ptr(), (m + STUB_NAME) as *mut u8, c.len());
    MEM.store(m, Ordering::Release);
    for buf in &LABEL_BUF {
        let label = vec![0u8; LABEL_LEN].into_boxed_slice();
        buf.store(Box::leak(label).as_mut_ptr() as usize, Ordering::Release);
    }

    // Wrappers first (they only add to KH1's), then the code patches.
    ORIG_SLOTS.store(base + SLOTS_UPDATE, Ordering::Release);
    ORIG_MEMBERS.store(base + MEMBERS_UPDATE, Ordering::Release);
    ORIG_HELP.store(base + HELP_UPDATE, Ordering::Release);
    ORIG_LIST.store(base + LIST_UPDATE, Ordering::Release);
    ORIG_PAGE.store(base + PAGE_UPDATE, Ordering::Release);
    ORIG_BOX.store(base + BOX_UPDATE, Ordering::Release);
    if hook::install(base + FILL_FN, &FILL_PROLOGUE, fill as *const () as usize, &ORIG_FILL, "Equipment weapon list fill").is_none() {
        return;
    }
    let ours: [(Rva, Rva, usize); 6] = [
        (SLOTS_DEF, SLOTS_UPDATE, slots_update as *const () as usize),
        (MEMBERS_DEF, MEMBERS_UPDATE, members_update as *const () as usize),
        (HELP_DEF, HELP_UPDATE, help_update as *const () as usize),
        (LIST_DEF, LIST_UPDATE, list_update as *const () as usize),
        (PAGE_DEF, PAGE_UPDATE, page_update as *const () as usize),
        (BOX_DEF, BOX_UPDATE, box_update as *const () as usize),
    ];
    let mut ok = true;
    for (def, f, w) in ours {
        ok &= hook::patch(base + def, &((base + f) as u64).to_le_bytes(), &(w as u64).to_le_bytes(), "Equipment window update");
    }
    ok &= hook::redirect_call(call, base + NAME_FN, m + STUB_NAME, "Equipment row name");
    // The two jumps last: until they are in, KH1's code runs as it was.
    let mut j = vec![0xE9];
    j.extend_from_slice(&rel32(base + R12_SITE + 5, m + STUB_R12));
    j.extend_from_slice(&[0x90, 0x90]);
    ok &= hook::patch(base + R12_SITE, &R12_BYTES, &j, "Equipment slot record");
    let mut j = vec![0xE9];
    j.extend_from_slice(&rel32(row_site + 5, m + STUB_ROW));
    j.extend_from_slice(&[0x90, 0x90, 0x90]);
    ok &= hook::patch(row_site, &row_old, &j, "Equipment slot row test");
    log(&format!("Equipment: Valor, Master and Final rows for Sora{} (stubs at {m:#x})", if ok { "" } else { ", some patches failed (see above)" }));
}

/// The save block.
unsafe fn save() -> Option<usize> {
    super::menu::save_block(base())
}

/// Sora's record is the one shown in the slots (`slot`: a party slot 0..3).
unsafe fn sora_in(save: usize, slot: i32) -> bool {
    (0..4).contains(&slot) && rd::<u8>(save + SAVE_PARTY + slot as usize) == 0
}

/// Sora's own weapon while a pick's preview may hold another in his weapon byte (from the list's
/// opening until his byte is his own again with the list closed), for the stats
/// (`valorstats::add`).
pub fn pick_own() -> Option<u8> {
    let own = PICK_WEAPON.load(Ordering::Relaxed) as u8;
    (own != 0).then_some(own)
}

/// KH1's compare box is working out its figures for a pick: a stats call now is its what-if,
/// the weapon in Sora's byte the hovered one (`0x1402E69A0`).
pub fn comparing() -> bool {
    COMPARING.load(Ordering::Relaxed)
}

/// Sora's own weapon while a pick's preview may hold another in his weapon byte; the guard ends
/// once the list is closed and his byte is his own again (KH1's restore).
unsafe fn pick_guard(save: usize) -> Option<u8> {
    let w = PICK_WEAPON.load(Ordering::Relaxed);
    if w == 0 {
        return None;
    }
    if g::<i32>(STATE) != 2 && rd::<u8>(save + WEAPON) as u32 == w {
        PICK_WEAPON.store(0, Ordering::Relaxed);
        log(&format!("Equipment: Sora's own weapon {w:#04x} back after the pick"));
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

/// The form rows under Sora's weapon (rows 1..=3): Valor's, Master's then Final's (KH2's order,
/// as the Drive list), each once its form is unlocked (`form::unlocked`).
unsafe fn form_rows() -> Vec<Form> {
    use super::form::{unlocked, Kind};
    let mut v = Vec::with_capacity(FORMS);
    if unlocked(Kind::Valor) {
        v.push(Form::Valor);
    }
    if unlocked(Kind::Master) {
        v.push(Form::Master);
    }
    if unlocked(Kind::Final) {
        v.push(Form::Final);
    }
    v
}

/// How many form rows Sora's slots have this frame (as `prepare` left the row cells; 0: KH1's
/// rows as they are).
unsafe fn rows_shown() -> usize {
    let m = mem();
    if m == 0 {
        return 0;
    }
    (0..FORMS).filter(|i| rd::<u8>(m + CELL_ROWS + i) != 0).count()
}

/// The form a pick is for (the form row confirmed, `CELL_PICK`), if one is under way.
unsafe fn pick_form() -> Option<Form> {
    let m = mem();
    if m == 0 {
        return None;
    }
    match rd::<u8>(m + CELL_PICK) {
        r @ 1..=3 => Form::from(ROW_FORM[r as usize - 1].load(Ordering::Relaxed)),
        _ => None,
    }
}

/// The Keyblade on Sora's form row `row` (1..=3) as prepared (0 for none).
unsafe fn row_id(row: usize) -> u8 {
    rd::<u8>(mem() + SHADOW + ACC_FIRST + row - 1)
}

/// A form's Keyblade for its row (0 for none). Valor: `valor::keyblade`, which keeps its slot up
/// to date (the Dream Sword when it holds none Valor may use); Final: `valor::final_keyblade`
/// (the Kingdom Key when its slot holds none Sora may take). During a pick, the slots as stored
/// (the preview's Keyblade in Sora's byte would look equipped).
unsafe fn form_id(save: usize, form: Form) -> u8 {
    let guarded = pick_guard(save).is_some();
    match (form, guarded) {
        (Form::Valor, true) => equip::valor_stored(base()),
        (Form::Valor, false) => super::valor::keyblade().unwrap_or(0),
        (Form::Final, true) => equip::final_stored(base()),
        (Form::Final, false) => super::valor::final_keyblade(),
        (Form::Master, true) => equip::master_stored(base()),
        (Form::Master, false) => super::valor::master_keyblade(),
    }
}

/// Sora's slots window with the form rows: `accessories + 1 + forms` rows (KH1 makes `+ 1`),
/// visible rows and height as KH1 works them out (8 at most).
unsafe fn fix_rows(win: usize, acc: u16, forms: u16) {
    if forms == 0 || win < 0x10000 || !super::crash::readable(win + 0x100) {
        return;
    }
    let rows = rd::<u16>(win + 0xF8);
    if rows != acc + 1 {
        return;
    }
    let want = acc + 1 + forms;
    let vis = want.min(8);
    let (h, gap) = (g::<i32>(ROW_H) as i16, g::<i32>(ROW_GAP) as i16);
    wr(win + 0xF8, want);
    wr(win + 0xFA, vis);
    wr(win + 0xE6, ((h + gap) * vis as i16 - gap) * 16);
    if LOGGED_ROWS.swap(want as u32, Ordering::Relaxed) != want as u32 {
        log(&format!("Equipment: Sora's slots {want} rows (weapon, {forms} form rows, {acc} accessories)"));
    }
}

/// Before KH1's slots update: rows, the shadow record and the row cells for this frame.
unsafe fn prepare(save: usize, win: usize) -> bool {
    let m = mem();
    let slot = g::<i32>(CURSOR_SLOT);
    let forms = if m != 0 && sora_in(save, slot) { form_rows() } else { Vec::new() };
    if forms.is_empty() {
        if m != 0 {
            wr(m + CELL_SHADOW, 0u64);
            wr(m + CELL_ROWS, 0u32);
        }
        return false;
    }
    let acc = rd::<u8>(save + ACC_COUNT).min(8) as usize;
    if win != 0 {
        fix_rows(win, acc as u16, forms.len() as u16);
    }
    let sh = m + SHADOW;
    std::ptr::write_bytes(sh as *mut u8, 0, 0x40);
    let mut key = 0u64;
    for i in 0..FORMS {
        let (form, id) = match forms.get(i) {
            Some(&f) => (f as u8, form_id(save, f)),
            None => (0, 0),
        };
        ROW_FORM[i].store(form, Ordering::Relaxed);
        // Valor's row holds the Dream Sword at least, and always offers it; Final's and Master's
        // hold the Kingdom Key at least, but offer a choice only when one is free. (A Keyblade
        // to choose from: owned, not equipped, not another slot's copy, counted with his own
        // weapon; the row's own one counts.)
        let free = match Form::from(form) {
            Some(f) => with_real_weapon(save, || !candidates(f).is_empty()),
            None => false,
        };
        let cell: u8 = match form {
            0 => 0,
            _ if id != 0 && free => 1,
            _ => 2,
        };
        wr(m + CELL_ROWS + i, cell);
        wr(sh + ACC_FIRST + i, id);
        key |= ((form as u64) << 8 | id as u64) << (16 * i);
    }
    for i in 0..acc {
        wr(sh + ACC_FIRST + forms.len() + i, rd::<u8>(save + ACC_FIRST + i));
    }
    wr(m + CELL_SHADOW, sh as u64);
    if LOGGED_ID.swap(key, Ordering::Relaxed) != key {
        let rows: Vec<String> =
            forms.iter().enumerate().map(|(i, &f)| format!("{} {:#04x} (candidates {:02x?})", f.name(), row_id(i + 1), with_real_weapon(save, || candidates(f)))).collect();
        log(&format!("Equipment: form rows [{}]", rows.join(", ")));
    }
    true
}

/// After a KH1 update that may have opened a list from Sora's slots: an accessory row is
/// `forms` lower than KH1 took it; a pick replaces Sora's weapon as the item being replaced by
/// the form's Keyblade.
unsafe fn after(save: usize) {
    let m = mem();
    if g::<i32>(STATE) != 2 {
        if rd::<u8>(m + CELL_PICK) != 0 {
            wr(m + CELL_PICK, 0u8);
        }
        pick_guard(save);
        return;
    }
    let pick = rd::<u8>(m + CELL_PICK);
    if (1..=FORMS as u8).contains(&pick) {
        let id = row_id(pick as usize);
        if g::<i32>(CURRENT) != id as i32 {
            s(CURRENT, id as i32);
            let name = pick_form().map_or("?", |f| f.name());
            log(&format!("Equipment: {name} row picked (Keyblade {id:#04x}): KH1's weapon list for {name}"));
        }
        return;
    }
    let forms = rows_shown() as i32;
    let row = g::<i32>(ROW_PICKED);
    if forms > 0 && row > forms && g::<i32>(ACC_INDEX) == row - 1 {
        let k = row - 1 - forms;
        s(ACC_INDEX, k);
        s(CURRENT, rd::<u8>(save + ACC_FIRST + k as usize) as i32);
        log(&format!("Equipment: Sora's row {row} is accessory slot {k}"));
    }
}

unsafe extern "system" fn slots_update(win: usize) {
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_SLOTS.load(Ordering::Acquire));
    let Some(save) = save() else { return f(win) };
    if !prepare(save, win) {
        return f(win);
    }
    LABEL_ON.store(LABEL_SLOTS, Ordering::Relaxed);
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
    fix_rows(slots, rd::<u8>(save + ACC_COUNT).min(8) as u16, form_rows().len() as u16);
}

/// The page: its header names the item being replaced (labelled when it is a form's), and its
/// rebuild of an open list (state 2) takes the accessory row as KH1 counts it.
unsafe extern "system" fn page_update(win: usize) {
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_PAGE.load(Ordering::Acquire));
    let Some(save) = save() else { return f(win) };
    let m = mem();
    let sora = m != 0 && sora_in(save, g::<i32>(SLOT));
    LABEL_ON.store(if sora { LABEL_PAGE } else { 0 }, Ordering::Relaxed);
    f(win);
    LABEL_ON.store(0, Ordering::Relaxed);
    if sora && rows_shown() != 0 {
        after(save);
    }
}

/// The slots' help line: KH1 takes row 0 as the weapon and row n as accessory n - 1 (from the
/// visible cursor only). For Sora: rows 1..=forms are the forms' Keyblades (shown through
/// accessory 0 for the call), row n after them accessory n - 1 - forms.
unsafe extern "system" fn help_update(win: usize) {
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_HELP.load(Ordering::Acquire));
    let m = mem();
    let Some(save) = save() else { return f(win) };
    let forms = rows_shown() as i32;
    if m == 0 || forms == 0 || g::<u64>(RECORD) as usize != save + 4 || g::<i32>(STATE) != 1 {
        return f(win);
    }
    let (scroll, cursor) = (g::<i32>(SCROLL), g::<i32>(CURSOR));
    let row = scroll + cursor;
    if row <= 0 {
        return f(win);
    }
    s(SCROLL, 0i32);
    if row <= forms {
        let acc0 = rd::<u8>(save + ACC_FIRST);
        wr(save + ACC_FIRST, row_id(row as usize));
        s(CURSOR, 1i32);
        f(win);
        wr(save + ACC_FIRST, acc0);
    } else {
        s(CURSOR, row - forms);
        f(win);
    }
    s(SCROLL, scroll);
    s(CURSOR, cursor);
}

/// KH1's free count (`0x1402E6330 (id)`: owned less the copies the ten members have equipped),
/// less the copies Valor's and Final's slots hold (`equip::form_held`), so KH1 counts them as in
/// use too. Its callers (Epic exe, read statically): the shop's sell list and its counts
/// (`0x1402FF380`: items with a free copy and a sell price, `0x1402FF472` / `0x1402FF4D8`), the
/// shop windows' "free" figures and the sell quantity's limit (`0x1402FD20C`, `0x1402FD353`,
/// `0x1402FDC31`, `0x1402FFB12`), and the accessory list's fill (`0x140304D9F` / `0x140304E08`,
/// accessories only, so no change there). KH1's weapon list doesn't use it (`fill` does that).
unsafe extern "system" fn free_count(id: i32) -> i32 {
    let f: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(ORIG_FREE.load(Ordering::Acquire));
    let n = f(id);
    if n <= 0 || !(1..=255).contains(&id) {
        return n;
    }
    let held = equip::form_held(base(), id as u8) as i32;
    if held == 0 {
        return n;
    }
    static LOGS: AtomicU32 = AtomicU32::new(0);
    static LAST: AtomicU32 = AtomicU32::new(u32::MAX);
    let key = (id as u32) << 16 | (n as u32 & 0xFF) << 8 | held as u32;
    if LAST.swap(key, Ordering::Relaxed) != key && LOGS.fetch_add(1, Ordering::Relaxed) < 20 {
        log(&format!("Equipment: KH1's free count of {id:#04x}: {n} less {held} held by the form slots = {}", (n - held).max(0)));
    }
    (n - held).max(0)
}

/// The Keyblades a form's slot may take (`equip::valor_candidates` / `final_candidates` /
/// `master_candidates`: not equipped, not another slot's copy; its own current one included).
unsafe fn candidates(form: Form) -> Vec<u8> {
    match form {
        Form::Valor => equip::valor_candidates(base()),
        Form::Final => equip::final_candidates(base()),
        Form::Master => equip::master_candidates(base()),
    }
}

/// The copies of `id` a list shows as Sora's to take: owned less those the form slots hold, his
/// own slot's (`form`, during its pick) left in. The Dream Sword in Valor's list: 1 (it needs
/// none owned; KH1 shows no count for 1).
unsafe fn shown_count(id: u8, form: Option<Form>) -> u32 {
    let b = base();
    if form == Some(Form::Valor) && id == equip::VALOR_DEFAULT {
        return 1;
    }
    let held = match form {
        None => equip::form_held(b, id),
        Some(Form::Valor) => (equip::final_held(b) == id) as u32 + (equip::master_held(b) == id) as u32,
        Some(Form::Final) => (equip::valor_held(b) == id) as u32 + (equip::master_held(b) == id) as u32,
        Some(Form::Master) => (equip::valor_held(b) == id) as u32 + (equip::final_held(b) == id) as u32,
    };
    (equip::owned(base(), id) as u32).saturating_sub(held)
}

/// Sora's own weapon list (row 0): his Keyblades whose every copy the form slots hold are taken
/// out (his own weapon always has a copy left: a slot holds one only beside the equipped ones),
/// and the counts are the copies left.
unsafe fn fill_sora() {
    let n = g::<i32>(LIST_LEN).clamp(0, 256) as usize;
    let (list, counts) = (base() + LIST, base() + LIST_COUNTS);
    let mut k = 0;
    let mut gone = Vec::new();
    for i in 0..n {
        let id = rd::<i32>(list + i * 4);
        let c = rd::<i32>(counts + i * 4);
        let (keep, c) = match u8::try_from(id) {
            Ok(b) if b != 0 && equip::is_keyblade(base(), b) => {
                let left = shown_count(b, None) as i32;
                (left > 0, left)
            }
            _ => (true, c),
        };
        if !keep {
            gone.push(id);
            continue;
        }
        wr(list + k * 4, id);
        wr(counts + k * 4, c);
        k += 1;
    }
    if k == n {
        return;
    }
    s(LIST_LEN, k as i32);
    log(&format!("Equipment: Sora's weapon list: {gone:02x?} held by the form slots, taken out ({k} left)"));
}

/// KH1's weapon list fill; during a pick only the Keyblades the form may take stay, the cursor
/// on the form's current one; for Sora's own weapon, without the copies the form slots hold.
unsafe extern "system" fn fill() {
    let f: unsafe extern "system" fn() = std::mem::transmute(ORIG_FILL.load(Ordering::Acquire));
    f();
    let m = mem();
    let pick = if m == 0 { 0 } else { rd::<u8>(m + CELL_PICK) };
    if !(1..=FORMS as u8).contains(&pick) {
        if g::<i32>(MEMBER) == 0 {
            fill_sora();
        }
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
    let Some(form) = pick_form() else { return };
    // (The copies counted with his own weapon in his byte, not the preview's.)
    let count = || {
        let keep = candidates(form);
        let shown: Vec<i32> = keep.iter().map(|&id| shown_count(id, Some(form)) as i32).collect();
        (keep, shown)
    };
    let (keep, shown) = match save() {
        Some(save) => with_real_weapon(save, count),
        None => count(),
    };
    let current = row_id(pick as usize) as i32;
    let n = g::<i32>(LIST_LEN).clamp(0, 256) as usize;
    let (list, counts) = (base() + LIST, base() + LIST_COUNTS);
    let before: Vec<i32> = (0..n).map(|i| rd::<i32>(list + i * 4)).collect();
    // (Valor's Dream Sword first, though KH1's fill lists only what he owns: `equip::valor_candidates`
    // offers it always.)
    let mut out: Vec<(i32, i32)> = Vec::with_capacity(n + 1);
    if let Some(j) = keep.iter().position(|&c| form == Form::Valor && c == equip::VALOR_DEFAULT) {
        out.push((equip::VALOR_DEFAULT as i32, shown[j]));
    }
    for i in 0..n {
        let id = rd::<i32>(list + i * 4);
        if out.iter().any(|(o, _)| *o == id) {
            continue;
        }
        if let Some(j) = keep.iter().position(|&c| c as i32 == id) {
            out.push((id, shown[j]));
        }
    }
    out.truncate(256);
    for (i, &(id, c)) in out.iter().enumerate() {
        wr(list + i * 4, id);
        wr(counts + i * 4, c);
    }
    let k = out.len();
    s(LIST_LEN, k as i32);
    let at = (0..k).find(|&i| rd::<i32>(list + i * 4) == current).unwrap_or(0) as i32;
    let (top, cur) = if (k as i32) < 7 { (0, at) } else { let t = at.min(k as i32 - 7); (t, at - t) };
    s(LIST_SCROLL, top);
    s(LIST_CURSOR, cur);
    log(&format!("Equipment: weapon list for {}: {before:02x?} -> {:02x?}", form.name(), (0..k).map(|i| rd::<i32>(list + i * 4)).collect::<Vec<_>>()));
}

/// The weapon list during a pick: KH1's live preview shows the hovered Keyblade in Sora's hand
/// (KH1 loads his own back once the list closes); a confirmed Keyblade goes to the form's slot,
/// Sora keeping his own.
unsafe extern "system" fn list_update(win: usize) {
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_LIST.load(Ordering::Acquire));
    let m = mem();
    let save = save();
    let (Some(save), true) = (save, m != 0 && rd::<u8>(m + CELL_PICK) != 0) else { return f(win) };
    let form = pick_form();
    let name = form.map_or("?", |f| f.name());
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
        let item = if (1..=255).contains(&hovered) { equip::item_name(base(), hovered as u8).unwrap_or_default() } else { String::new() };
        log(&format!(
            "Equipment: {name} pick hovers {hovered:#04x} {item} (row {at} of {n}; preview loads {:#04x}, Sora's byte {:#04x}, his own {weapon:#04x})",
            rd::<i32>(loaded_at),
            rd::<u8>(save + WEAPON)
        ));
    }
    if g::<i32>(STATE) == 3 {
        let picked = g::<i32>(NEW_ITEM);
        // Sora keeps his weapon: chosen and his byte as they were. The model loaded (the
        // preview's) is left for KH1's restore to swap back as the list closes.
        wr(chosen_at, chosen);
        wr(save + WEAPON, weapon);
        wr(m + CELL_PICK, 0u8);
        match form {
            Some(form) if (1..=255).contains(&picked) && candidates(form).contains(&(picked as u8)) => {
                let was = rd::<u8>(save + form.off());
                wr(save + form.off(), picked as u8);
                let item = equip::item_name(base(), picked as u8).unwrap_or_default();
                let held = if form == Form::Valor && picked as u8 == equip::VALOR_DEFAULT { "no copy held (Valor's default)" } else { "its copy held" };
                log(&format!(
                    "Equipment: {name}'s Keyblade stored: {picked:#04x} {item}, {held} ({was:#04x} given back; Sora keeps {weapon:#04x}; model loaded {:#04x}, put back by KH1)",
                    rd::<i32>(loaded_at)
                ));
            }
            _ => log(&format!("Equipment: {name} pick {picked:#x} isn't a free Keyblade: not stored (Sora keeps {weapon:#04x})")),
        }
        // (The stats refreshed once the pick is stored: in the form they count the new one.)
        let refresh: unsafe extern "system" fn() = std::mem::transmute(base() + REFRESH_FN);
        refresh();
        pick_guard(save);
    }
}

/// The weapon compare box (`0x140302C90`) during a pick. KH1 draws Strength "now" -> with the
/// hovered weapon (a what-if, `0x1402E69A0`) and MP "now" -> "now" + the hovered weapon's - the
/// current one's (`0x142E9AA5C`, the form row's Keyblade here), its "now" figures taken from the
/// menu's stats copy as the list opens: Sora's stats as they are, without a second Keyblade out
/// of the forms (or with the other form's in it). For the pick they are the form's: "now" his own
/// Keyblade and the row's, the what-if his own and the hovered one (`valorstats::add` while
/// `COMPARING`), so the row's own Keyblade shows no change and the figures follow the cursor
/// however fast it moves (the preview's load, a hover behind, plays no part). Valor's and Final's
/// rows work the same way: the second Keyblade's Strength and MP add alike in both forms.
unsafe extern "system" fn box_update(win: usize) {
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_BOX.load(Ordering::Acquire));
    let m = mem();
    let own = PICK_WEAPON.load(Ordering::Relaxed) as u8;
    if m == 0 || rd::<u8>(m + CELL_PICK) == 0 || own == 0 || win < 0x10000 || !super::crash::readable(win + BOX_MAX_MP + 4) {
        return f(win);
    }
    let current = g::<i32>(CURRENT);
    let second = if (1..=255).contains(&current) { current as u8 } else { 0 };
    if let Some((s, mp)) = super::valorstats::with_second(own, second) {
        let was = (rd::<i32>(win + BOX_STRENGTH), rd::<i32>(win + BOX_MAX_MP));
        wr(win + BOX_STRENGTH, s);
        wr(win + BOX_MAX_MP, mp);
        if was != (s, mp) {
            let name = pick_form().map_or("?", |f| f.name());
            log(&format!("Equipment: {name} pick's compare box: now Strength {s}, max MP {mp} (his own {own:#04x}, {name}'s {second:#04x}; KH1's {} / {})", was.0, was.1));
        }
    }
    COMPARING.store(true, Ordering::Relaxed);
    f(win);
    COMPARING.store(false, Ordering::Relaxed);
}

/// The name of a row's item (`0x140304F30`, through the name stub: `row` is the caller's `rbx`,
/// the slots loop's row; -1 when the stub can't know it): "V: " / "F: " and the Keyblade's name
/// on a form row (while the slots or the page header draw for Sora), KH1's otherwise.
unsafe extern "system" fn item_name(item: u32, row: u64) -> usize {
    let f: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base() + NAME_FN);
    let kh1 = f(item);
    let m = mem();
    let on = LABEL_ON.load(Ordering::Relaxed);
    if on == 0 || m == 0 || kh1 == 0 || item == 0 || item > 255 {
        return kh1;
    }
    let forms = rows_shown();
    if forms == 0 {
        return kh1;
    }
    let holds = |r: usize| (1..=forms).contains(&r) && row_id(r) as u32 == item;
    let first = || (1..=forms).find(|&r| holds(r));
    // The row named: the slots' row being drawn; the page header's: the row being picked for,
    // the cursor's, or the first holding the item.
    let at = if on == LABEL_SLOTS {
        if row == u64::MAX {
            first()
        } else {
            usize::try_from(row).ok().filter(|&r| holds(r))
        }
    } else {
        let pick = rd::<u8>(m + CELL_PICK) as usize;
        let cursor = if g::<i32>(STATE) == 1 { (g::<i32>(SCROLL) + g::<i32>(CURSOR)).max(0) as usize } else { 0 };
        [pick, cursor].into_iter().find(|&r| holds(r)).or_else(first)
    };
    let Some(at) = at else { return kh1 };
    let Some(form) = Form::from(ROW_FORM[at - 1].load(Ordering::Relaxed)) else { return kh1 };
    let buf = LABEL_BUF[form.index()].load(Ordering::Acquire);
    if buf == 0 {
        return kh1;
    }
    let id = item as u8;
    if LABEL_ID[form.index()].load(Ordering::Relaxed) != id {
        let mut b = super::menu::encode(form.prefix());
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
        LABEL_ID[form.index()].store(id, Ordering::Relaxed);
    }
    buf
}
