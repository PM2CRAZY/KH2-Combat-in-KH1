//! Soft Reset, the Config menu's QoL row (the user: "you can quick reset the game with l1 l2 select
//! and start pushed together", then "the combo does not work in the pc version so restore it": the
//! PS2's six-button reset, L1 + L2 + R1 + R2 + Start + Select). `docs/kh1-softreset.md`.
//!
//! KH1's pad code still tests for that combo (mask `0x0F09` on its pad table, Steam `0x14012B3A7` and
//! `0x14012BE03`, Checked (decomp + exe)) and raises a reset flag for the PS2 platform layer, which
//! this build does nothing useful with (Checked (user): the combo does nothing on PC).
//!
//! On: the combo held for a second (`HOLD_MS`, read from the mod's own pad read, `probe::pad`, so no
//! new hook and nothing per frame beyond one flag check) puts KH1's own game mode back to its boot
//! state: its mode step (`0x140170870`, a switch on the mode at `0x14233FBC0`) in mode 1 counts a
//! timer down and, below zero, runs KH1's own full reset (`0x140170630`: tasks, scene, sound and
//! fade state back to the start, mode 2), which loads the title screen as at start-up. The mod
//! writes only the mode, the mode-change countdown and that timer; KH1 steps the reset itself.
//! Unsaved progress is lost, as with a console reset; Continue loads the save. Off: nothing.
//!
//! Not checked in game: what is left behind by a world that was running (Assumption: KH1's reset,
//! written for boot, clears it; the PS2's overlay loads are no-ops in this build, Checked (decomp)).

use super::addr::Build;
use super::log;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

/// On (`soft_reset = on`).
static ON: AtomicBool = AtomicBool::new(false);
/// KH1's mode step is the one this was written for (checked once).
static VERIFIED: AtomicBool = AtomicBool::new(false);
static CHECKED: AtomicBool = AtomicBool::new(false);
/// When the combo was first held (ms since the first look, plus 1; 0 = not held), and the reset for
/// this hold is done.
static HELD_SINCE: AtomicU64 = AtomicU64::new(0);
static DONE: AtomicBool = AtomicBool::new(false);

/// The PS2 pad word's Select, Start, L2, R2, L1, R1 (KH1's own mask `0x0F09`, which the mod's pad read
/// shares: Select `0x0001`, L1 `0x0400`).
const COMBO: u16 = 0x0F09;
/// Held this long, in milliseconds, so a slip of the buttons doesn't reset.
const HOLD_MS: u64 = 1000;

/// KH1's mode step (Steam; `sub rsp, 0x28; movsxd rax, [mode]`, then `cmp eax, 0x12`), the mode, the
/// mode-change countdown and mode 1's timer (Checked (decomp + exe)).
const MODE_STEP_STEAM: usize = 0x170870;
const MODE_STEP_PROLOGUE: [u8; 7] = [0x48, 0x83, 0xEC, 0x28, 0x48, 0x63, 0x05];
const MODE_STEAM: usize = 0x233FBC0;
const COUNTDOWN_STEAM: usize = 0x22EC0AC;
const TIMER_STEAM: usize = 0x22EC0A8;
/// KH1's full reset (`push rbx; sub rsp, 0x20`), only looked at.
const RESET_STEAM: usize = 0x170630;
const RESET_PROLOGUE: [u8; 6] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20];
/// The modes: 0 and 1 are start-up, 2 and 3 the title screen; from 4 on a game is running.
const FIRST_GAME_MODE: i32 = 4;
const LAST_MODE: i32 = 0x12;

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// Soft Reset is on.
pub fn enabled() -> bool {
    ON.load(Ordering::Relaxed)
}

pub unsafe fn install() {
    let on = super::settings::get().soft_reset;
    ON.store(on, Ordering::Relaxed);
    if on {
        verify();
    }
    log!(Menus, "Soft Reset: {}", if on { "on (soft_reset = on): L1 + L2 + R1 + R2 + Start + Select held a second" } else { "off (soft_reset = off)" });
}

/// The Config menu's Soft Reset row, kept in the ini.
pub fn set(on: bool) {
    ON.store(on, Ordering::Relaxed);
    if on {
        unsafe { verify() };
    }
    super::settings::save("soft_reset", if on { "on" } else { "off" });
    log!(Menus, "Config menu: Soft Reset {}", if on { "on" } else { "off" });
}

/// Steam only: KH1's mode step and reset are the code this was read from (the first bytes, and the
/// mode the step reads is the address used here). Nothing is changed.
unsafe fn verify() {
    if CHECKED.swap(true, Ordering::Relaxed) {
        return;
    }
    if Build::current() != Build::Steam {
        log!(Menus, "Soft Reset: Steam only (KH1's mode step is where the Steam build has it)");
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let step = std::slice::from_raw_parts((base + MODE_STEP_STEAM) as *const u8, MODE_STEP_PROLOGUE.len());
    let reset = std::slice::from_raw_parts((base + RESET_STEAM) as *const u8, RESET_PROLOGUE.len());
    if step != MODE_STEP_PROLOGUE || reset != RESET_PROLOGUE {
        log!(Menus, "Soft Reset: unexpected bytes at KH1's mode step {step:02x?} / reset {reset:02x?}, not used");
        return;
    }
    // (The step's `movsxd rax, [rip + rel]` names the mode: it must be the address used here.)
    let rel = rd::<i32>(base + MODE_STEP_STEAM + MODE_STEP_PROLOGUE.len());
    let mode = (base + MODE_STEP_STEAM + MODE_STEP_PROLOGUE.len() + 4).wrapping_add(rel as isize as usize);
    if mode != base + MODE_STEAM {
        log!(Menus, "Soft Reset: KH1's mode is at {:#x}, not where expected, not used", mode.wrapping_sub(base));
        return;
    }
    VERIFIED.store(true, Ordering::Relaxed);
}

/// Each pad read (`probe::pad`, KH1's records merged): the combo held for `HOLD_MS` resets, once
/// for each hold.
pub fn pad(buttons: u16) {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    if buttons & COMBO != COMBO {
        HELD_SINCE.store(0, Ordering::Relaxed);
        DONE.store(false, Ordering::Relaxed);
        return;
    }
    let now = ms() + 1;
    let since = HELD_SINCE.load(Ordering::Relaxed);
    if since == 0 {
        HELD_SINCE.store(now, Ordering::Relaxed);
    } else if now - since >= HOLD_MS && !DONE.swap(true, Ordering::Relaxed) {
        unsafe { reset() };
    }
}

/// KH1's mode back to the start-up step that runs its own reset (see the header).
unsafe fn reset() {
    if !VERIFIED.load(Ordering::Relaxed) {
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let mode = rd::<i32>(base + MODE_STEAM);
    if !(FIRST_GAME_MODE..=LAST_MODE).contains(&mode) {
        log!(Menus, "Soft Reset: not in a game (KH1's mode {mode}), nothing to do");
        return;
    }
    // (Mode 1 counts its timer down and runs the reset below zero: the timer is put there, a pending
    // mode change dropped.)
    wr(base + COUNTDOWN_STEAM, 0i32);
    wr(base + TIMER_STEAM, -1i32);
    wr(base + MODE_STEAM, 1i32);
    log!(Menus, "Soft Reset: KH1's mode {mode} -> 1: its reset to the title screen runs on the next step");
}
