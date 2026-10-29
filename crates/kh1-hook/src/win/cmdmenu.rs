//! KH2's command menu in KH1's (the user: "gut the system and make sure that its 1:1 and any
//! features that are different between the two ... are in the command menu config";
//! `docs/kh1-kh2-command-menu.md` has both games' menus and every difference).
//!
//! Everything here runs on KH1's own battle menu (its menu structs, its chain, its fill, its
//! navigation and its run): nothing of ours draws or steps a menu. The Config menu's Command Menu
//! list switches each part (KH1 = vanilla KH1):
//!
//! - **Drive / Summon** (`drive_page`, t751/t752): KH2 puts Drive on page 1 (Attack, Magic, Items,
//!   Drive) and Summon on page 2 (Attack, Summon, Party, Limit), Checked (decomp: Steam KH2
//!   `0x1403F29E0` builds menu 0, `0x1403F3860` menu 6). The row can swap them: Summon on page 1
//!   (KH1's own row 4), Drive on page 2.
//! - **D-Pad Left** (`dpad_left`, t751/t752): KH2's d-pad left at the top level flips the page and the
//!   cursor keeps its row (`0x1403F2470`, Checked (decomp)); KH1's moves the cursor to Attack and
//!   there is no page 2 (`0x1402931B0`, Checked (decomp)).
//! - **Cursor Memory** (`cursor_memory`, t754): KH2 sends every open list's cursor and the top
//!   cursor back to row 0 once a command runs (`0x1403B21D0`), keeps a cursor on its command when a
//!   list changes (`0x1403B11E0`), and cancel resets only the top level's cursor (Checked (decomp)).
//!   KH1 keeps its lists' cursors after a command, follows rows, and cancel resets the parent's
//!   cursor at any depth (`0x140293480`, `0x1402931B0`, Checked (decomp)).
//! - **Command Lock** (`command_lock`, t755): KH2 takes a chosen command only when Sora can act on
//!   it (`0x1403B6950` / `0x1403BA070`); before that the press does nothing and the list stays open.
//!   Its rows grey only for availability (MP, items, form), never for what Sora is doing (Checked
//!   (decomp)). KH1 greys and blocks spells and items by Sora's movement state (`0x140294220` /
//!   `0x1402944C0`, the state table's bits 0x20 / 0x40) and takes any unblocked command.
//!
//! Page 2's Party and Limit rows are KH1's empty command (0x42) for now, as KH2 shows them while
//! they're not available (the user: "(a) we'll add them later").
//!
//! Steam only (`docs/engineering-rules.md`): the hooks here check the Steam build and the first
//! bytes, and stay out otherwise (KH1's menu as it is).

use super::addr::Build;
use super::{hook, log};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

/// KH1's battle menu structs (Steam; 0x38 bytes each: `+4` greyed bits, `+8` blocked bits, `+0xC`
/// cursor, `+0x10` count, `+0x14` command ids, `+0x34` the entry whose list is open below).
const ROOT_STEAM: usize = 0x528900;
const MAGIC_STEAM: usize = 0x528938;
const ITEMS_STEAM: usize = 0x528970;
const SUMMON_STEAM: usize = 0x5289A8;
/// The menu chain (eight slots, the first the top level) and the pointer to its current slot.
const CHAIN_STEAM: usize = 0x2D539F0;
const LEVEL_STEAM: usize = 0x2D53A50;
/// KH1's battle buttons (`+4` held, `+8` pressed, logical bits) and the confirm button's logical
/// bits (the button swap applied).
const BUTTONS_STEAM: usize = 0x298CA90;
const CONFIRM_MASK_STEAM: usize = 0x5456A0;
/// Sora's actor (KH1's first party slot) and KH1's movement state table (0x40 bytes a state).
const SORA_STEAM: usize = 0x2D37280;
const STATES_STEAM: usize = 0x2DB3840;
/// KH1's top-level fill (`0x140294220`, run by the command step before navigation): Attack, the
/// battle lists (`0x1402944C0`), the Items row, the party targets, the Triangle command.
const TOP_FILL_STEAM: usize = 0x294220;
/// `mov [rsp+0x10], rbx; mov [rsp+0x18], rbp`.
const TOP_FILL_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x6C, 0x24, 0x18];
/// KH1's can-act test (`0x140297870 (actor)`).
const CAN_ACT_STEAM: usize = 0x297870;
/// KH1's system sound (`0x14017A040 (id)`: 1 the cursor's move).
const SOUND_STEAM: usize = 0x17A040;

const SUMMON_CMD: u8 = 6;
const SUMMON_OUT: u8 = 0x4F;
/// KH1's empty command (the top level's rows before the fill gives them a command).
const NONE: u8 = 0x42;
/// Logical buttons: d-pad left, and the cancel bit (KH1's cancel is `6`: jump `4` and cancel `2`).
const DPAD_LEFT: u32 = 0x80;
const CANCEL_BIT: u32 = 2;
/// The movement states KH1 casts and uses items in (`0x1402A3600`: 0, 6, 0x18 on the ground; 2, 8
/// in the air, Checked (decomp)). Command Lock KH2 lifts KH1's state greying only in these: KH1's
/// own states (ladders, swimming, the Jungle Slider...) keep KH1's rule.
const BATTLE_STATES: [i32; 5] = [0, 2, 6, 8, 0x18];
/// The state table's "may cast" and "may use items" bits (`0x1402944C0`, `0x140294220`).
const STATE_MAY_CAST: u8 = 0x20;
const STATE_MAY_ITEM: u8 = 0x40;

static BASE: AtomicUsize = AtomicUsize::new(0);
/// Our hooks are in (Steam).
static ON: AtomicBool = AtomicBool::new(false);
/// The settings (the Command Menu rows).
static DRIVE_PAGE2: AtomicBool = AtomicBool::new(false);
static DPAD_KH2: AtomicBool = AtomicBool::new(true);
static MEMORY_KH2: AtomicBool = AtomicBool::new(true);
static LOCK_KH2: AtomicBool = AtomicBool::new(true);
/// Page 2 is up.
static PAGE2: AtomicBool = AtomicBool::new(false);
static ORIG_TOP_FILL: AtomicUsize = AtomicUsize::new(0);
/// L2 + R3 held this frame (`camera::l2_r3`, from the pad records), and last frame.
pub static R3_CONFIRM: AtomicBool = AtomicBool::new(false);
static R3_BEFORE: AtomicBool = AtomicBool::new(false);
/// Cursor Memory KH2: each list's command under its cursor (magic, items, summons / forms), `-1`
/// none.
static LAST_ID: [AtomicI32; 3] = [AtomicI32::new(-1), AtomicI32::new(-1), AtomicI32::new(-1)];
/// Cursor Memory KH2: a cancel pressed two levels deep (the level slot, the parent list, its
/// cursor), to keep the parent's cursor where KH1 sets it to 0.
static BACK_OUT: Mutex<Option<(usize, usize, i32)>> = Mutex::new(None);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}
fn base() -> usize {
    BASE.load(Ordering::Relaxed)
}
fn on() -> bool {
    ON.load(Ordering::Relaxed) && base() != 0
}

// ---------------------------------------------------------------------------------------------
// Settings (the Config menu's Command Menu list, kept in the ini).

/// Drive on page 2 (Summon on page 1): `drive_page = 2`. KH2's is Drive on page 1.
pub fn drive_page2() -> bool {
    DRIVE_PAGE2.load(Ordering::Relaxed)
}
pub fn set_drive_page2(v: bool) {
    DRIVE_PAGE2.store(v, Ordering::Relaxed);
    super::settings::save("drive_page", if v { "2" } else { "1" });
    log!(Menus, "Config menu: Drive on page {} (Summon on page {})", if v { 2 } else { 1 }, if v { 1 } else { 2 });
}
/// D-pad left at the top level flips the page (KH2) or moves the cursor to Attack (KH1).
pub fn dpad_kh2() -> bool {
    DPAD_KH2.load(Ordering::Relaxed)
}
pub fn set_dpad_kh2(v: bool) {
    DPAD_KH2.store(v, Ordering::Relaxed);
    if !v {
        PAGE2.store(false, Ordering::Relaxed);
    }
    super::settings::save("dpad_left", if v { "kh2" } else { "kh1" });
    log!(Menus, "Config menu: D-Pad Left {}", if v { "KH2 (flips the page)" } else { "KH1 (cursor to Attack, no page 2)" });
}
pub fn memory_kh2() -> bool {
    MEMORY_KH2.load(Ordering::Relaxed)
}
pub fn set_memory_kh2(v: bool) {
    MEMORY_KH2.store(v, Ordering::Relaxed);
    for l in &LAST_ID {
        l.store(-1, Ordering::Relaxed);
    }
    super::settings::save("cursor_memory", if v { "kh2" } else { "kh1" });
    log!(Menus, "Config menu: Cursor Memory {}", if v { "KH2" } else { "KH1" });
}
pub fn lock_kh2() -> bool {
    LOCK_KH2.load(Ordering::Relaxed)
}
pub fn set_lock_kh2(v: bool) {
    LOCK_KH2.store(v, Ordering::Relaxed);
    super::settings::save("command_lock", if v { "kh2" } else { "kh1" });
    log!(Menus, "Config menu: Command Lock {}", if v { "KH2" } else { "KH1" });
}

// ---------------------------------------------------------------------------------------------
// Pages.

/// Page 2 is up (only with D-Pad Left KH2 and the Drive hooks in: `drive::on`).
pub fn page2() -> bool {
    on() && dpad_kh2() && super::drive::on() && PAGE2.load(Ordering::Relaxed)
}

/// The shown page holds Drive (page 1 with Drive on page 1, page 2 with Drive on page 2).
pub fn drive_shown() -> bool {
    super::drive::on() && page2() == drive_page2()
}

/// Drive is KH1's fourth top row now (the magic list's hidden last entry, KH1's own way of
/// making row 4: `docs/kh1-drive.md`).
pub fn drive_in_row4() -> bool {
    drive_shown() && !page2()
}

/// Drive is page 2's second row (the top level's entry 1).
pub fn drive_in_page2() -> bool {
    drive_shown() && page2()
}

/// The top level (not the L1 shortcut menu, no list open).
unsafe fn at_top(base: usize) -> bool {
    rd::<u64>(base + LEVEL_STEAM) as usize == base + CHAIN_STEAM && rd::<u64>(base + CHAIN_STEAM) as usize == base + ROOT_STEAM
}

/// Before KH1's command step (`shortcuts::step`, or `drive`'s own hook): L2 + R3 as confirm, the
/// page flip, and a cancel's back-out remembered.
pub unsafe fn pre_step() {
    let base = base();
    if !on() {
        return;
    }
    let b = base + BUTTONS_STEAM;
    // L2 + R3: KH2's confirm (Steam KH2's field button table `0x1405C34F0`: raw L2 + R3 is logical
    // bit 33, which the menu reads as confirm with bit 2, `0x1403B11E0`; Checked (exe + decomp)).
    let r3 = R3_CONFIRM.swap(false, Ordering::Relaxed);
    if r3 && !R3_BEFORE.load(Ordering::Relaxed) {
        let confirm = rd::<u32>(base + CONFIRM_MASK_STEAM);
        if confirm != 0 {
            wr(b + 8, rd::<u32>(b + 8) | confirm);
            wr(b + 4, rd::<u32>(b + 4) | confirm);
            log!(Menus, "command menu: L2 + R3, confirm");
        }
    }
    R3_BEFORE.store(r3, Ordering::Relaxed);
    let pressed = rd::<u32>(b + 8);
    // D-pad left at the top level: KH2's page flip (KH1's would move the cursor to Attack). The
    // cursor keeps its row (`0x1403F2470`: the new page's cursor set to the old one's row).
    if pressed & DPAD_LEFT != 0 && dpad_kh2() && super::drive::on() && at_top(base) {
        wr(b + 8, pressed & !DPAD_LEFT);
        let now = !PAGE2.load(Ordering::Relaxed);
        PAGE2.store(now, Ordering::Relaxed);
        // (The list under row 2 / row 4 changes contents with the page.)
        wr(base + SUMMON_STEAM + 0xC, 0i32);
        LAST_ID[2].store(-1, Ordering::Relaxed);
        // KH2's page flip sound (KH2 system sound 0x14, `0x1401DD170`; KH1 has none for it), through
        // KH1's engine (`kh2sound::page_flip`); KH1's cursor sound until KH2's sounds are loaded.
        if !super::kh2sound::page_flip() {
            let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_STEAM);
            sound(1);
        }
        log!(Menus, "command menu: page {}", if now { "2" } else { "1" });
    }
    // Cancel two or more levels deep: KH1 sets the parent's cursor to 0, KH2 only the top level's.
    if memory_kh2() && pressed & CANCEL_BIT != 0 {
        let level = rd::<u64>(base + LEVEL_STEAM) as usize;
        let chain = base + CHAIN_STEAM;
        let depth = level.wrapping_sub(chain) / 8;
        if level >= chain && depth >= 2 && depth < 8 {
            let parent = rd::<u64>(level - 8) as usize;
            if parent > 0x10000 {
                *BACK_OUT.lock().unwrap_or_else(|e| e.into_inner()) = Some((level, parent, rd::<i32>(parent + 0xC)));
            }
        }
    }
}

/// After KH1's command step: a back-out's parent cursor kept (Cursor Memory KH2), each list's
/// command under its cursor noted.
pub unsafe fn post_step() {
    let base = base();
    if !on() {
        return;
    }
    if let Some((level, parent, cursor)) = BACK_OUT.lock().unwrap_or_else(|e| e.into_inner()).take() {
        if rd::<u64>(base + LEVEL_STEAM) as usize == level - 8 && rd::<u64>(level - 8) as usize == parent && rd::<i32>(parent + 0xC) == 0 && cursor != 0 {
            wr(parent + 0xC, cursor);
            log!(Menus, "command menu: back out, the list's cursor kept on row {cursor} (Cursor Memory KH2)");
        }
    }
    if memory_kh2() {
        for (k, m) in [MAGIC_STEAM, ITEMS_STEAM, SUMMON_STEAM].into_iter().enumerate() {
            let m = base + m;
            let (n, c) = (rd::<i32>(m + 0x10), rd::<i32>(m + 0xC));
            let id = if (0..16).contains(&n) && (0..n).contains(&c) { rd::<u8>(m + 0x14 + c as usize) as i32 } else { -1 };
            LAST_ID[k].store(id, Ordering::Relaxed);
        }
    }
}

/// KH1's top-level fill, with Command Lock KH2's state rule around it, then page 2's rows and the
/// cursors kept on their commands.
unsafe extern "system" fn top_fill() {
    let f: unsafe extern "system" fn() = std::mem::transmute(ORIG_TOP_FILL.load(Ordering::Acquire));
    let base = base();
    let sora = rd::<u64>(base + SORA_STEAM) as usize;
    // Command Lock KH2: KH1 greys and blocks spells and items by the movement state's bits; KH2 by
    // availability only. The fill reads the state's bits while it runs: in the battle states they
    // read "may cast, may use items" for its run, and KH1's own availability (MP, the items held,
    // range, the save's flags) is all that greys. Put back at once.
    let mut lifted: Option<(usize, u8)> = None;
    if lock_kh2() && sora > 0x10000 {
        let state = rd::<i32>(sora + 0x70);
        if BATTLE_STATES.contains(&state) {
            let rec = base + STATES_STEAM + state as usize * 0x40;
            let was = rd::<u8>(rec);
            wr(rec, was | STATE_MAY_CAST | STATE_MAY_ITEM);
            lifted = Some((rec, was));
        }
    }
    f();
    if let Some((rec, was)) = lifted {
        wr(rec, was);
    }
    if sora > 0x10000 {
        page2_rows(base, sora);
        follow_commands(base);
    }
}

/// Page 2's rows over KH1's top level (Steam KH2 `0x1403F3860`, Checked (decomp)): Attack (KH1's
/// own), Summon (or Drive with Drive on page 2), Party, Limit. Party and Limit aren't in yet: KH1's
/// empty command, blocked. Row 4 is KH1's own row: KH1 draws and runs it as the magic list's last
/// entry only while row 2 is Magic, so it is empty here too.
unsafe fn page2_rows(base: usize, sora: usize) {
    if !page2() {
        return;
    }
    let top = base + ROOT_STEAM;
    let magic = base + MAGIC_STEAM;
    let (id, grey, block) = if drive_in_page2() {
        let any = super::drive::any_usable(sora);
        (SUMMON_CMD, !any, !any)
    } else {
        // Summon as KH1 shows it in row 4 (the magic list's last entry, Summon or the summon out).
        let n = rd::<i32>(magic + 0x10);
        let last = (1..=16).contains(&n).then(|| rd::<u8>(magic + 0x14 + n as usize - 1));
        match last {
            Some(c) if c == SUMMON_CMD || c == SUMMON_OUT => {
                let bit = 1u32 << (n - 1);
                (c, rd::<u32>(magic + 4) & bit != 0, rd::<u32>(magic + 8) & bit != 0)
            }
            _ => (NONE, true, true),
        }
    };
    wr(top + 0x14 + 1, id);
    wr(top + 0x14 + 2, NONE);
    let set = |off: usize, bit: u32, on: bool| {
        let v = rd::<u32>(top + off);
        wr(top + off, if on { v | bit } else { v & !bit });
    };
    set(4, 2, grey);
    set(8, 2, block);
    set(4, 4, true);
    set(8, 4, true);
}

/// Cursor Memory KH2: a list rebuilt with its cursor's command elsewhere has the cursor follow it
/// (Steam KH2 `0x1403B11E0` / `0x1403B1140`, Checked (decomp)); a command gone leaves the cursor's
/// row (row 0 past the end).
unsafe fn follow_commands(base: usize) {
    if !memory_kh2() {
        return;
    }
    for (k, m) in [MAGIC_STEAM, ITEMS_STEAM, SUMMON_STEAM].into_iter().enumerate() {
        let id = LAST_ID[k].load(Ordering::Relaxed);
        if id < 0 {
            continue;
        }
        let m = base + m;
        let (n, c) = (rd::<i32>(m + 0x10), rd::<i32>(m + 0xC));
        if !(0..16).contains(&n) || (0..n).contains(&c) && rd::<u8>(m + 0x14 + c as usize) as i32 == id {
            continue;
        }
        if let Some(p) = (0..n).find(|&p| rd::<u8>(m + 0x14 + p as usize) as i32 == id) {
            wr(m + 0xC, p);
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 20 {
                log!(Menus, "command menu: list {k}'s cursor follows command {id:#x} to row {p} (Cursor Memory KH2)");
            }
        } else if !(0..n.max(1)).contains(&c) {
            wr(m + 0xC, 0i32);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The run (`combat::run_entry`, KH1's `0x140293480 (menu, entry)`).

/// The open lists when a run starts (the chain's slots past the top), for `command_taken`.
pub unsafe fn open_lists() -> [usize; 7] {
    let mut out = [0usize; 7];
    let base = base();
    if !on() {
        return out;
    }
    let chain = base + CHAIN_STEAM;
    let level = rd::<u64>(base + LEVEL_STEAM) as usize;
    if level < chain || level >= chain + 64 {
        return out;
    }
    for (k, o) in out.iter_mut().enumerate() {
        let slot = chain + 8 * (k + 1);
        if slot > level {
            break;
        }
        *o = rd::<u64>(slot) as usize;
    }
    out
}

/// A command was taken (KH1's run returned 1; the chain is back at the top): Cursor Memory KH2 sends
/// every list that was open, and the top level, back to row 0 (Steam KH2 `0x1403B21D0`). KH1 keeps
/// the lists' cursors (and sets the top's to 0 itself, for commands without its "run on held" flag).
pub unsafe fn command_taken(open: &[usize; 7]) {
    if !on() || !memory_kh2() {
        return;
    }
    let base = base();
    for &m in open.iter().filter(|&&m| m > 0x10000) {
        wr(m + 0xC, 0i32);
    }
    wr(base + ROOT_STEAM + 0xC, 0i32);
    for l in &LAST_ID {
        l.store(-1, Ordering::Relaxed);
    }
}

/// Command Lock KH2: the command can't be taken yet (Steam KH2 `0x1403BA070`: Sora's action done or
/// in a window that takes a command). Natively: KH1's can-act, or KH1's combo window, which our
/// slots carry from KH2's combo and Finishing Plus windows (`kh2slots`, as `form::native_can_change`).
/// Assumption: KH1's can-act covers KH2's free states (idle, walk, run, jump, fall, a landing past
/// its lock), as `form` assumes for the Drive.
pub unsafe fn locked(actor: usize) -> bool {
    if !on() || !lock_kh2() || actor < 0x10000 {
        return false;
    }
    let can: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base() + CAN_ACT_STEAM);
    can(actor) == 0 && rd::<u32>(actor + 0x1CC) & super::combomods::KH1_COMBO_WINDOW == 0
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    let s = super::settings::get();
    DRIVE_PAGE2.store(s.drive_page2, Ordering::Relaxed);
    DPAD_KH2.store(s.dpad_left_kh2, Ordering::Relaxed);
    MEMORY_KH2.store(s.cursor_memory_kh2, Ordering::Relaxed);
    LOCK_KH2.store(s.command_lock_kh2, Ordering::Relaxed);
    if Build::current() != Build::Steam {
        log!(Menus, "command menu: KH1's (the Command Menu rows are Steam only for now)");
        return;
    }
    if hook::install(base + TOP_FILL_STEAM, &TOP_FILL_PROLOGUE, top_fill as *const () as usize, &ORIG_TOP_FILL, "command menu top fill").is_none() {
        log!(Menus, "command menu: the top fill wasn't as expected: KH1's menu");
        return;
    }
    ON.store(true, Ordering::Release);
    log!(Menus,
        "command menu: Drive on page {}, D-Pad Left {}, Cursor Memory {}, Command Lock {}",
        if drive_page2() { 2 } else { 1 },
        if dpad_kh2() { "KH2" } else { "KH1" },
        if memory_kh2() { "KH2" } else { "KH1" },
        if lock_kh2() { "KH2" } else { "KH1" }
    );
}
