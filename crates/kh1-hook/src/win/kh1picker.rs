//! KH1's own combo row pickers, patched with KH2's ability rows (testing branch; the tester,
//! combo row selection m19: "we need to modify kh1s so that it can choose explosion and other moves
//! during its kh1 combo", "patch the extra functions that kh2 uses in kh1"). `docs/kh1-anim-slots.md`
//! §7.z.
//!
//! KH1's Attack step (Steam `0x1402A7510`, Checked (decomp)) counts the press (the combat info's
//! `+0x41`) and asks its picker for the row: the ground picker `0x1402A6EE0 (actor, target, dir)`
//! in state 0, the air picker `0x1402A6C70 (actor, target, dir)` in states 2 and 8 (flight). Each
//! marks its candidate rows in a flag array by its own rules (the finisher at the stats' `+0xD4`
//! / `+0xD5`, its ability rows by its ability bits `0x142D5EC10` and chances, the target's
//! distance and height) and walks it in table order, the first marked row whose request isn't the
//! one playing winning. Ours wrap both: KH1's picker runs whole, then, with base Sora at the combo
//! style KH1 where he is, KH2's ability rows join its choice (`combomods::picker`). Flight and
//! every other case: KH1's row as it picked it. Steam only; the prologues are checked.

use super::addr::Build;
use super::sys::GetModuleHandleA;
use std::sync::atomic::{AtomicUsize, Ordering};

const GROUND_STEAM: usize = 0x2A6EE0;
/// `mov r11, rsp; push rbp; push rsi; push rdi; lea rbp, [r11 - 0xB8]` (Checked (exe)).
const GROUND_PROLOGUE: [u8; 13] = [0x4C, 0x8B, 0xDC, 0x55, 0x56, 0x57, 0x49, 0x8D, 0xAB, 0x48, 0xFF, 0xFF, 0xFF];
const AIR_STEAM: usize = 0x2A6C70;
/// `push rbx; push rbp; push rsi; push rdi; push r12; push r14; push r15` (Checked (exe)).
const AIR_PROLOGUE: [u8; 11] = [0x40, 0x53, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x56, 0x41, 0x57];

static ORIG_GROUND: AtomicUsize = AtomicUsize::new(0);
static ORIG_AIR: AtomicUsize = AtomicUsize::new(0);

type GroundFn = unsafe extern "system" fn(usize, usize, usize) -> u64;
type AirFn = unsafe extern "system" fn(usize, usize, usize) -> i32;

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

/// Base Sora with KH1's own combo style where he is (KH1's own states, not a Drive Form, the
/// takeover on): KH2's ability rows join KH1's picker.
unsafe fn ours(actor: usize, air: bool) -> bool {
    use super::styles::{kh2, Kind};
    actor >= 0x10000
        && actor == super::takeover::sora_actor()
        && super::takeover::ON.load(Ordering::Relaxed)
        && super::shared::active()
        && !super::form::in_form()
        && !kh2(if air { Kind::AirCombo } else { Kind::GroundCombo })
}

unsafe extern "system" fn ground(actor: usize, target: usize, dir: usize) -> u64 {
    let f: GroundFn = std::mem::transmute(ORIG_GROUND.load(Ordering::Acquire));
    let row = f(actor, target, dir);
    if rd::<i32>(actor + 0x70) != 0 || !ours(actor, false) {
        return row;
    }
    super::combomods::picker(actor, row as i32, dir, false) as u32 as u64
}

unsafe extern "system" fn air(actor: usize, target: usize, dir: usize) -> i32 {
    let f: AirFn = std::mem::transmute(ORIG_AIR.load(Ordering::Acquire));
    let row = f(actor, target, dir);
    // (State 8, flight, runs KH1's picker too: KH1's own there.)
    if rd::<i32>(actor + 0x70) != 2 || !ours(actor, true) {
        return row;
    }
    super::combomods::picker(actor, row, dir, true)
}

pub unsafe fn install() {
    if Build::current() != Build::Steam {
        return;
    }
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    super::hook::install(base + GROUND_STEAM, &GROUND_PROLOGUE, ground as *const () as usize, &ORIG_GROUND, "KH1's ground combo row picker (KH2's ability rows)");
    super::hook::install(base + AIR_STEAM, &AIR_PROLOGUE, air as *const () as usize, &ORIG_AIR, "KH1's air combo row picker (KH2's ability rows)");
}
