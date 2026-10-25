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
//! style KH1 where he is, KH2's ability rows join its choice (`combomods::picker`), and Negative
//! Combo shortens the length KH1's picker reads (`shorten`). Flight and every other case: KH1's
//! row as it picked it. Steam only; the prologues are checked.
//!
//! The finisher walk (the tester, m21: "do 1", one walk in KH1's picker with KH1's rule set): at
//! a finisher KH1's own ability finishers are walked with KH2's (`combomods::finisher_walk`), so
//! while KH1's picker runs its ability bits for them are held back (`HELD_GROUND` / `HELD_AIR`)
//! and the walk marks them by KH1's own rules in their new places. Blitz's bit stays: KH1's own
//! test for it (row 6: its finisher marked and nothing beside him, row 11's count 0) runs inside
//! KH1's picker, which returns 6 when it passes. KH1's 30% chance on Stun Impact and Gravity
//! Break is the ini's `kh1_ability_chance`, the Config menu's Tweaks row (`chance`).

use super::addr::Build;
use super::sys::GetModuleHandleA;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

const GROUND_STEAM: usize = 0x2A6EE0;
/// `mov r11, rsp; push rbp; push rsi; push rdi; lea rbp, [r11 - 0xB8]` (Checked (exe)).
const GROUND_PROLOGUE: [u8; 13] = [0x4C, 0x8B, 0xDC, 0x55, 0x56, 0x57, 0x49, 0x8D, 0xAB, 0x48, 0xFF, 0xFF, 0xFF];
const AIR_STEAM: usize = 0x2A6C70;
/// `push rbx; push rbp; push rsi; push rdi; push r12; push r14; push r15` (Checked (exe)).
const AIR_PROLOGUE: [u8; 11] = [0x40, 0x53, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x56, 0x41, 0x57];

/// Sora's ability bits (Steam `0x142D5EC10`; set from his equipped abilities by `0x1402A6380`,
/// Checked (decomp): bit 3 Blitz (20), 25 Hurricane Blast (55), 26 Ripple Drive (56), 27 Stun
/// Impact (57), 28 Gravity Break (58), 29 Zantetsuken (59)).
const ABILITY_BITS_STEAM: usize = 0x2D5EC10;
/// Held back while KH1's picker runs at the combo style KH1 (the walk marks them): Ripple Drive,
/// Stun Impact, Gravity Break on the ground (KH1's rows 7, 3, 4); Hurricane Blast in the air
/// (row 15). Zantetsuken (29, row 5) and Blitz (3, row 6) stay KH1's own.
pub const HELD_GROUND: u32 = 1 << 26 | 1 << 27 | 1 << 28;
pub const HELD_AIR: u32 = 1 << 25;
/// KH1's random number in 0..1 (`0x14031BA30`: three `rand` bytes x 2^-24; its picker's 30% rolls
/// use it), `mov [rsp+8], rbx; push rdi; sub rsp, 0x20` (Checked (exe)).
const RANDOM_STEAM: usize = 0x31BA30;
const RANDOM_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];
static RANDOM: AtomicUsize = AtomicUsize::new(0);
/// The Tweaks row "KH1 Ability Chance" (0 not read yet, 1 off, 2 on).
static CHANCE: AtomicU8 = AtomicU8::new(0);

/// KH1's 30% chance on Stun Impact and Gravity Break (the tester, m21: "a toggle in the config
/// for the 30 percent chance to exist, if it does then if the 30 percent chance roll fails then
/// it picks a similar kh2 move"). On: each rolls as in KH1, and a failed roll goes on down the
/// walk. Off: they pass whenever their checks do, as KH2's rows.
pub fn chance() -> bool {
    match CHANCE.load(Ordering::Relaxed) {
        0 => {
            let c = super::settings::get().kh1_ability_chance;
            CHANCE.store(1 + c as u8, Ordering::Relaxed);
            c
        }
        v => v == 2,
    }
}

/// The Config row (Tweaks > KH1 Ability Chance): live, kept in the ini.
pub fn set_chance(on: bool) {
    CHANCE.store(1 + on as u8, Ordering::Relaxed);
    super::settings::save("kh1_ability_chance", if on { "on" } else { "off" });
    super::log(&format!("Config menu: KH1 Ability Chance {}", if on { "On" } else { "Off" }));
}

/// KH1's random number (0..1), as its picker draws it; `None` if the function isn't the
/// expected code here.
pub unsafe fn random() -> Option<f32> {
    let at = RANDOM.load(Ordering::Relaxed);
    if at == 0 {
        return None;
    }
    let f: unsafe extern "system" fn() -> f32 = std::mem::transmute(at);
    Some(f()).filter(|r| r.is_finite())
}

/// KH1's picker `f` run with `held` of its ability bits held back, put back after. The row it
/// took and the bits as KH1 had them.
unsafe fn holding<R>(held: u32, f: impl FnOnce() -> R) -> (R, u32) {
    let at = GetModuleHandleA(std::ptr::null()) as usize + ABILITY_BITS_STEAM;
    let bits = rd::<u32>(at);
    std::ptr::write_unaligned(at as *mut u32, bits & !held);
    let r = f();
    std::ptr::write_unaligned(at as *mut u32, bits);
    (r, bits)
}

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

/// Negative Combo on KH1's own length for its picker's call (`combomods::kh1_length`): the stats'
/// length byte (`+0xD4` ground, `+0xD5` air; the pickers' own finisher test, Checked (decomp))
/// lowered while KH1's picker runs and put back after, so KH1 itself marks its finisher a swing
/// sooner. Nothing else reads it in that call. The byte's address and value, to put back.
unsafe fn shorten(actor: usize, air: bool) -> Option<(usize, u8)> {
    let stats = super::pose::ptr(rd::<u32>(actor + 0x6C));
    if stats < 0x10000 {
        return None;
    }
    let at = stats + if air { 0xD5 } else { 0xD4 };
    let was = rd::<u8>(at);
    let now = super::combomods::kh1_length(was);
    if now == was {
        return None;
    }
    std::ptr::write_unaligned(at as *mut u8, now);
    Some((at, was))
}

unsafe fn put_back(saved: Option<(usize, u8)>) {
    if let Some((at, was)) = saved {
        std::ptr::write_unaligned(at as *mut u8, was);
    }
}

unsafe extern "system" fn ground(actor: usize, target: usize, dir: usize) -> u64 {
    let f: GroundFn = std::mem::transmute(ORIG_GROUND.load(Ordering::Acquire));
    if rd::<i32>(actor + 0x70) != 0 || !ours(actor, false) {
        return f(actor, target, dir);
    }
    let saved = shorten(actor, false);
    let (row, bits) = holding(HELD_GROUND, || f(actor, target, dir));
    put_back(saved);
    super::combomods::picker(actor, row as i32, dir, false, bits) as u32 as u64
}

unsafe extern "system" fn air(actor: usize, target: usize, dir: usize) -> i32 {
    let f: AirFn = std::mem::transmute(ORIG_AIR.load(Ordering::Acquire));
    // (State 8, flight, runs KH1's picker too: KH1's own there.)
    if rd::<i32>(actor + 0x70) != 2 || !ours(actor, true) {
        return f(actor, target, dir);
    }
    let saved = shorten(actor, true);
    let (row, bits) = holding(HELD_AIR, || f(actor, target, dir));
    put_back(saved);
    super::combomods::picker(actor, row, dir, true, bits)
}

pub unsafe fn install() {
    if Build::current() != Build::Steam {
        return;
    }
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    if rd::<[u8; 10]>(base + RANDOM_STEAM) == RANDOM_PROLOGUE {
        RANDOM.store(base + RANDOM_STEAM, Ordering::Relaxed);
    } else {
        super::log("KH1's random number (0x14031BA30) isn't the expected code here: KH1 Ability Chance rolls never pass");
    }
    super::hook::install(base + GROUND_STEAM, &GROUND_PROLOGUE, ground as *const () as usize, &ORIG_GROUND, "KH1's ground combo row picker (KH2's ability rows)");
    super::hook::install(base + AIR_STEAM, &AIR_PROLOGUE, air as *const () as usize, &ORIG_AIR, "KH1's air combo row picker (KH2's ability rows)");
}
