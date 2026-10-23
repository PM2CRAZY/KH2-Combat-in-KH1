//! The jump press that backs out of a command submenu is the menu's, not KH2's jump (the user:
//! "when the drives, summons, items, or magic submenus are open, pressing the jump button closes
//! the menus and jumps at the same time").
//!
//! KH1 has one button for both: its pad step (`0x140288F30`) turns Circle (Cross with the confirm
//! swap) into logical `6`, jump (`4`) and cancel (`2`) together, and the keyboard's bindings
//! only set pad buttons (`0x1400D7DE0`), so no binding can split them. Its command step
//! (`0x140291660`) reads the jump press only at the top level (`[0x142D54450]` the root); in a
//! submenu the same press backs out a level (`0x140291020`) and KH1 doesn't jump.
//!
//! KH1 runs that step as a task of priority 55000 and the actors (Sora's control step, then his
//! physics step, where KH2 reads the pad) at 80000, later in the same frame (its task list runs
//! in ascending order). So on the press frame the submenu was already closed when KH2 looked
//! (`shortcuts::submenu_open` false) and the press was fresh: KH2 jumped. Here the press is
//! noted before KH1's step runs and kept from KH2 (`takeover::buttons`) until it is let go: no
//! jump, Aerial Dodge, glide or other move on the jump button out of it, on the ground or in
//! the air.

use super::addr::Rva;
use super::{hook, log};
use crate::menujump::JumpHold;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

/// KH1's logical buttons pressed this frame (`+8` of its state `0x14298D4A0`).
const PRESSED_RVA: Rva = Rva(0x298D4A8);
/// KH1's jump (`4`) and cancel (`2`), one button.
const JUMP_CANCEL: u32 = 6;
/// The command menu's current level (a slot of its chain, holding the open menu's struct).
const LEVEL_PTR_RVA: Rva = Rva(0x2D54450);

static BASE: AtomicUsize = AtomicUsize::new(0);
static HOLD: Mutex<JumpHold> = Mutex::new(JumpHold::new());
static ORIG_STEP: AtomicUsize = AtomicUsize::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

/// The open menu, by its struct's type (`docs/kh1-drive.md`): 1 the magic list (Limit Form's
/// Limit menu), 2 items, 3 summons (page 2's Drive list), 4 a target list.
unsafe fn menu_name(base: usize) -> &'static str {
    let slot = rd::<u64>(base + LEVEL_PTR_RVA) as usize;
    let menu = if slot > 0x10000 { rd::<u64>(slot) as usize } else { 0 };
    let kind = if menu > 0x10000 { rd::<i32>(menu) } else { -1 };
    match kind {
        1 if super::form::in_limit() => "Limit menu",
        1 => "Magic menu",
        2 => "Items menu",
        3 if super::drive::page2() => "Drive menu",
        3 => "Summon menu",
        4 => "target list",
        _ => "submenu",
    }
}

/// Before KH1's command step (from `shortcuts::step`, `drive::step` or ours): a jump / cancel
/// press with a submenu open is the menu's.
pub unsafe fn pre_step() {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 {
        return;
    }
    let pressed = rd::<u32>(base + PRESSED_RVA) & JUMP_CANCEL != 0;
    let submenu = super::shortcuts::submenu_open();
    let held = super::takeover::jump_held_raw();
    // (Let go: the hold ends here too, every frame, so a release made while KH2 wasn't reading
    // the pad isn't missed.)
    let new = HOLD
        .lock()
        .map(|mut h| {
            h.filter(held);
            h.menu_step(submenu, pressed)
        })
        .unwrap_or(false);
    if new {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 300 {
            log(&format!("jump held back: it closed the {}", menu_name(base)));
        }
    }
}

/// The jump button as KH2 sees it (`takeover::buttons`): not while it is the press that backed
/// out of a submenu.
pub fn jump_down(down: bool) -> bool {
    HOLD.lock().map(|mut h| h.filter(down)).unwrap_or(down)
}

unsafe extern "system" fn step() -> u64 {
    pre_step();
    let f: unsafe extern "system" fn() -> u64 = std::mem::transmute(ORIG_STEP.load(Ordering::Acquire));
    f()
}

/// After `shortcuts` and `drive`: the command step hooked here only when neither did.
pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    if super::shortcuts::step_hooked() || super::drive::step_hooked() {
        return;
    }
    if hook::install(base + super::shortcuts::STEP_RVA, &super::shortcuts::STEP_PROLOGUE, step as *const () as usize, &ORIG_STEP, "command step (jump held back)").is_none() {
        log("menu jump: the command step wasn't as expected: a jump that closes a submenu may still jump");
    }
}
