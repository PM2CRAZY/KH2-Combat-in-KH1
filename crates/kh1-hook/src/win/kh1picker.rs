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
//! Combo shortens the length KH1's picker reads (`shorten`). Flight at the Aerial Base Combo KH2:
//! KH2's air pick on KH1's flight rows (`combomods::flight_pick`). Every other case: KH1's row as
//! it picked it. Steam only; the prologues are checked.
//!
//! The finisher walk (the tester, m21: "do 1", one walk in KH1's picker with KH1's rule set): at
//! a finisher KH1's own ability finishers are walked with KH2's (`combomods::finisher_walk`), so
//! while KH1's picker runs its ability bits for them are held back (`HELD_GROUND` / `HELD_AIR`)
//! and the walk marks them by KH1's own rules in their new places. Blitz's bit stays: KH1's own
//! test for it (row 6: its finisher marked and nothing beside him, row 11's count 0) runs inside
//! KH1's picker, which returns 6 when it passes. So does Hurricane Blast's (row 15: a target, no
//! more than 100 above him; the tester, m22 answer 3 (b)). KH1's 30% chance on Stun Impact, Gravity
//! Break and Zantetsuken is the ini's `kh1_ability_chance`, the Config menu's Tweaks row (`chance`).
//!
//! At the combo style KH2 (the tester, m23: "I can't do any kh1 finisher on kh2 combat settings")
//! the same hooks run KH1's picker with its ability finisher bits held back and KH2's combo length
//! on the length it reads, and when KH2's pick at this press is a finisher the same walk takes
//! KH1's own ability finishers in their KH2 places (`combomods::kh2_finisher_walk`); otherwise KH2's
//! pick stands, and the picker returns the KH1 row that plays it (`combomods::kh2_pick_in_picker`;
//! m24: "(a)", KH2's whole pick made in KH1's picker), which the row runner runs as it is.

use super::addr::Build;
use super::sys::GetModuleHandleA;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

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
/// Stun Impact, Gravity Break, Zantetsuken on the ground (KH1's rows 7, 3, 4, 5; Zantetsuken's
/// 30% roll is KH1's third, after the other two, so the walk rolls it the same way, with KH1
/// Ability Chance on like the other two: the tester, after m25). Blitz (3, row 6) stays KH1's own, and so does Hurricane Blast in
/// the air (25, row 15: its KH1 rule, a target no more than 100 above him, m22 answer 3 (b)):
/// KH1's picker returns their row when its own test passes, and the walk takes that as their rule.
/// (Holding Zantetsuken back also keeps KH1's row 8 marked, which Blitz's test needs.)
pub const HELD_GROUND: u32 = 1 << 26 | 1 << 27 | 1 << 28 | 1 << 29;
pub const HELD_AIR: u32 = 0;
/// Blitz (bit 3) and Hurricane Blast (25), KH1's two ability finishers its picker tests itself:
/// held back too in Valor Form (`at_kh2`), where none of KH1's own moves come out.
const ALL_FINISHER_BITS: u32 = 1 << 3 | 1 << 25;
/// KH1's random number in 0..1 (`0x14031BA30`: three `rand` bytes x 2^-24; its picker's 30% rolls
/// use it), `mov [rsp+8], rbx; push rdi; sub rsp, 0x20` (Checked (exe)).
const RANDOM_STEAM: usize = 0x31BA30;
const RANDOM_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];
static RANDOM: AtomicUsize = AtomicUsize::new(0);
/// The Tweaks row "KH1 Ability Chance" (0 not read yet, 1 off, 2 on).
static CHANCE: AtomicU8 = AtomicU8::new(0);

/// KH1's 30% chance on Stun Impact, Gravity Break and Zantetsuken (the tester, m21: "a toggle in the config
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
    super::log!(Combat, "Config menu: KH1 Ability Chance {}", if on { "On" } else { "Off" });
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

/// KH1's air picker's plain swing (flight row 21 once the Attack step adds 3).
const FLIGHT_SWING: i32 = 18;

/// Whether our wrapper on KH1's ground / air picker is in (`at_kh2` makes KH2's pick there, so the
/// row runner doesn't ask KH2's walk again, `combomods::attack`).
pub fn hooked(air: bool) -> bool {
    if air { AIR_IN.load(Ordering::Relaxed) } else { GROUND_IN.load(Ordering::Relaxed) }
}

static GROUND_IN: AtomicBool = AtomicBool::new(false);
static AIR_IN: AtomicBool = AtomicBool::new(false);

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

/// Base Sora with KH2's combo style where he is (as `ours`, the style KH2): KH1's own ability
/// finishers join KH2's finisher walk (`combomods::kh2_finisher_walk`).
unsafe fn ours_kh2(actor: usize, air: bool) -> bool {
    use super::styles::{kh2, Kind};
    actor >= 0x10000
        && actor == super::takeover::sora_actor()
        && super::takeover::ON.load(Ordering::Relaxed)
        && super::shared::active()
        // (Valor Form on KH1's own states: KH2's pick from Valor's table whatever the styles; a
        // Drive Form is KH2's all through, `form::native_now`.)
        && (super::form::native_now() || !super::form::in_form() && kh2(if air { Kind::AirCombo } else { Kind::GroundCombo }))
}

/// KH1's picker at the combo style KH2 (the tester, combo row selection m23: "I can't do any kh1
/// finisher on kh2 combat settings"; m24: "(a)", KH2's whole pick in KH1's picker): KH2's pick at
/// this press first (`combomods::kh2_press`, the only time KH2's selector runs for it), then KH1's
/// picker with KH2's combo length in the length it reads (`sync_length`, t891) and its
/// ability finisher bits held back (`held`), then the walk (`combomods::kh2_finisher_walk`), and,
/// where KH2's pick stands, the KH1 row that plays it (`combomods::kh2_pick_in_picker`).
unsafe fn at_kh2(actor: usize, air: bool, held: u32, dir: usize, flight: bool, f: impl FnOnce() -> i32) -> i32 {
    use super::combomods::Ask;
    // (KH2's combo length in KH1's stats, kept there, t891: KH2's walk and KH1's picker both read
    // it.)
    sync_length(actor, air);
    let press = match super::combomods::kh2_press(actor, air) {
        Ask::Took(p) => Some(p),
        // KH2's walk takes no row (the tester, what-is-still-missing m14 answer 8: KH1's own combo
        // logic refusing as KH2's does, for base Sora at the KH2 setting and the Drive Forms
        // alike): KH1's picker itself answers "no attack" (`combomods::NO_ROW`), so its Attack
        // step runs no row, instead of KH1's own row playing under KH2's setting (base Sora) or a
        // second check at the row runner (t685, the Drive Forms). Not in flight (its step adds 3).
        Ask::Refused if !flight && super::combomods::refuse_in_picker(actor, air) => return super::combomods::NO_ROW,
        Ask::Refused | Ask::Unasked => None,
    };
    if press.is_none() {
        // (Not asked, or a refusal the row runner overrides: KH1's picker as it is; the row runner
        // is told KH2 takes nothing.)
        let row = f();
        return super::combomods::kh2_pick_in_picker(actor, row, dir, air, None);
    }
    // (In Valor Form, `form::native_now`, none of KH1's own ability finishers: KH2's Valor has its
    // own table only. Every one of KH1's bits held back for its picker's call, no walk.)
    let valor = super::form::native_now();
    let (row, bits) = holding(if valor { held | HELD_GROUND | ALL_FINISHER_BITS } else { held }, f);
    if !valor {
        if let Some(n) = super::combomods::kh2_finisher_walk(actor, row, air, bits, press.as_ref()) {
            return n;
        }
    }
    super::combomods::kh2_pick_in_picker(actor, row, dir, air, press)
}

unsafe extern "system" fn ground(actor: usize, target: usize, dir: usize) -> u64 {
    let f: GroundFn = std::mem::transmute(ORIG_GROUND.load(Ordering::Acquire));
    if rd::<i32>(actor + 0x70) == 0 && ours_kh2(actor, false) {
        return at_kh2(actor, false, HELD_GROUND, dir, false, || f(actor, target, dir) as i32) as u32 as u64;
    }
    if rd::<i32>(actor + 0x70) != 0 || !ours(actor, false) {
        return f(actor, target, dir);
    }
    sync_length(actor, false);
    let saved = shorten(actor, false);
    let (row, bits) = holding(HELD_GROUND, || f(actor, target, dir));
    put_back(saved);
    super::combomods::picker(actor, row as i32, dir, false, bits) as u32 as u64
}

unsafe extern "system" fn air(actor: usize, target: usize, dir: usize) -> i32 {
    let f: AirFn = std::mem::transmute(ORIG_AIR.load(Ordering::Acquire));
    // State 8, flight, runs KH1's picker too (the Attack step runs its row + 3): at the Aerial
    // Base Combo KH2, KH2's air pick made as in state 2, played on KH1's flight row of its kind
    // (`combomods::flight_pick`; the tester, what-is-still-missing m6 answer 2 (a)).
    if rd::<i32>(actor + 0x70) == 8 && ours_kh2(actor, true) {
        let mut own = FLIGHT_SWING;
        let run = at_kh2(actor, true, HELD_AIR, dir, true, || {
            own = f(actor, target, dir);
            own
        });
        return super::combomods::flight_pick(own, run);
    }
    if rd::<i32>(actor + 0x70) == 2 && ours_kh2(actor, true) {
        return at_kh2(actor, true, HELD_AIR, dir, false, || f(actor, target, dir));
    }
    if rd::<i32>(actor + 0x70) != 2 || !ours(actor, true) {
        return f(actor, target, dir);
    }
    sync_length(actor, true);
    let saved = shorten(actor, true);
    let (row, bits) = holding(HELD_AIR, || f(actor, target, dir));
    put_back(saved);
    super::combomods::picker(actor, row, dir, true, bits)
}

// KH1's stats builder (Steam `0x1402A5130 (character, stats)`, Checked (decomp)): it writes the
// combo's length into the stats, `+0xD4` (ground) = Combo Plus (ability 6) equipped + 3 and
// `+0xD5` (air) = Air Combo Plus (7) + 3, counted in the character's ability list by
// `0x140284BE0 (ability, character)`; called whenever KH1 rebuilds a party member's stats
// (`0x1402A5AE0`, the party loop at `0x140284C30`'s callers, the menus). KH1's pickers'
// finisher test reads those bytes (`0x1402A6EE0` / `0x1402A6C70`: `len <= count`). It is
// hooked once, by `menu`'s party stats hook (Valor's second Keyblade, the debug menu's locks),
// which calls `after_stats` after KH1's own (t896: t891 hooked it a second time here, and
// `menu`'s hook then refused it, finding our jump in its first bytes: the log, "hook party stats
// ... unexpected bytes, not hooked").

/// KH1's ability counter `0x140284BE0 (ability, character) -> copies equipped`: `mov r8, [rip+..]`
/// then `mov r9d, ecx; cmp edx, 9` (Checked (exe); the first instruction's offset not compared).
const COUNT_STEAM: usize = 0x284BE0;
const COUNT_HEAD: [u8; 3] = [0x4C, 0x8B, 0x05];
const COUNT_NEXT: [u8; 6] = [0x44, 0x8B, 0xC9, 0x83, 0xFA, 0x09];
static COUNT: AtomicUsize = AtomicUsize::new(0);
/// KH2's length is in the byte (ground, air): KH1's own goes back when it is no longer KH2's.
static KH2_LENGTH_IN: [AtomicBool; 2] = [AtomicBool::new(false), AtomicBool::new(false)];

/// After KH1's stats builder ran for a party member (`menu`'s party stats hook): KH2's length in
/// Sora's own stats where it is his (`sync_length`). Steam only.
pub unsafe fn after_stats(out: usize) {
    if Build::current() != Build::Steam {
        return;
    }
    let actor = super::takeover::sora_actor();
    if actor >= 0x10000 && out >= 0x10000 && super::pose::ptr(rd::<u32>(actor + 0x6C)) == out {
        // (KH1 wrote its own length just now: none of KH2's is in it.)
        KH2_LENGTH_IN[0].store(false, Ordering::Relaxed);
        KH2_LENGTH_IN[1].store(false, Ordering::Relaxed);
        sync_length(actor, false);
        sync_length(actor, true);
    }
}

/// KH1's own combo length for Sora (`0x1402A5130`'s rule: the copies of Combo Plus or Air Combo
/// Plus equipped, `0x140284BE0`, plus 3), if KH1's counter is the expected code.
unsafe fn kh1_own_length(actor: usize, air: bool) -> Option<u8> {
    let at = COUNT.load(Ordering::Relaxed);
    let obj = super::pose::ptr(rd::<u32>(actor + 0x130));
    if at == 0 || obj < 0x10000 {
        return None;
    }
    let count: unsafe extern "system" fn(u32, u32) -> i32 = std::mem::transmute(at);
    let n = count(if air { 7 } else { 6 }, rd::<u16>(obj + 0x4C) as u32);
    Some((n.clamp(0, 250) + 3) as u8)
}

/// KH2's combo length in Sora's own stats (the owner's plan, answer 2 (a), t891): base Sora
/// with the Base / Aerial Base Combo at KH2 where he is, or a Drive Form on KH1's own states,
/// has KH2's length (`kh2table`'s rule on his abilities: KH2's base, Combo Plus / Air Combo Plus,
/// Negative Combo) in the byte KH1's code reads (`+0xD4` / `+0xD5`), kept there: written after
/// KH1's own stats builder runs (`stats`) and at each Attack press in KH1's pickers (an ability or
/// a Config change between the two). Elsewhere KH1's own length goes back once (`kh1_own_length`)
/// where ours was in it.
pub unsafe fn sync_length(actor: usize, air: bool) {
    let stats = super::pose::ptr(rd::<u32>(actor + 0x6C));
    if stats < 0x10000 {
        return;
    }
    let at = stats + if air { 0xD5 } else { 0xD4 };
    let flag = &KH2_LENGTH_IN[air as usize];
    if ours_kh2(actor, air) {
        let Some(len) = super::kh2table::now().zip(super::combomods::abilities_now()).map(|(t, a)| t.length(&a, air)) else { return };
        if rd::<u8>(at) != len {
            std::ptr::write_unaligned(at as *mut u8, len);
        }
        flag.store(true, Ordering::Relaxed);
    } else if flag.swap(false, Ordering::Relaxed) {
        if let Some(len) = kh1_own_length(actor, air) {
            std::ptr::write_unaligned(at as *mut u8, len);
        }
    }
}

/// KH2's length is in Sora's stats byte for this press (`sync_length`).
pub fn kh2_length_in(air: bool) -> bool {
    KH2_LENGTH_IN[air as usize].load(Ordering::Relaxed)
}

pub unsafe fn install() {
    if Build::current() != Build::Steam {
        return;
    }
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    if rd::<[u8; 3]>(base + COUNT_STEAM) == COUNT_HEAD && rd::<[u8; 6]>(base + COUNT_STEAM + 7) == COUNT_NEXT {
        COUNT.store(base + COUNT_STEAM, Ordering::Relaxed);
    } else {
        super::log!(Combat, "KH1's ability counter (0x140284BE0) isn't the expected code here: KH1's own combo length isn't put back after KH2's");
    }
    if rd::<[u8; 10]>(base + RANDOM_STEAM) == RANDOM_PROLOGUE {
        RANDOM.store(base + RANDOM_STEAM, Ordering::Relaxed);
    } else {
        super::log!(Combat, "KH1's random number (0x14031BA30) isn't the expected code here: KH1 Ability Chance rolls never pass");
    }
    let g = super::hook::install(base + GROUND_STEAM, &GROUND_PROLOGUE, ground as *const () as usize, &ORIG_GROUND, "KH1's ground combo row picker (KH2's ability rows)");
    GROUND_IN.store(g.is_some(), Ordering::Relaxed);
    let a = super::hook::install(base + AIR_STEAM, &AIR_PROLOGUE, air as *const () as usize, &ORIG_AIR, "KH1's air combo row picker (KH2's ability rows)");
    AIR_IN.store(a.is_some(), Ordering::Relaxed);
}
