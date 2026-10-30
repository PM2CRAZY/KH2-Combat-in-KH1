//! KH2's MP costs scaled onto KH1's MP (the tester, thread kh2-magic-and-reflect-native, answer
//! 6 (b): "scale KH2's costs onto KH1's MP"). Base Sora's KH2 spells only; Drive Forms keep
//! their own (1 MP a spell).
//!
//! - KH2's costs (03system `cmd`, Checked (files)): Fire 12, Blizzard 15, Thunder 18 at every
//!   level (commands 49 / 119 / 120, 51 / 121 / 122, 50 / 123 / 124), Cure 255 = all of it (52 /
//!   125 / 126), Reflect 10 (177..179), out of KH2 Sora's MP of 100 (00battle `plrp`, every row of
//!   character 1, Checked (files)).
//! - KH1's MP (Checked (decomp), Steam): Sora's stats `+0x44` whole bars of `+0x48`, and the
//!   charge part `+0xBE` (u16), which fills toward the next bar: bar `n + 1` comes at
//!   `(n + 1) × 30` (`0x1402A4CB0`). KH1's payment `0x1402A5B60 (actor, cost)`: from 100 up
//!   `cost / 100` whole bars; below 100 `cost` charge units, and when they run out the top bar
//!   breaks into its own worth (bar `n` gives `n × 30`). KH1's test `0x1402A4C10` counts
//!   `Σ k × 30 + charge` for costs below 100. So KH1's MP is a pool of units in which bar `k` is
//!   worth `k × 30`, and the charge part is a share of the next bar.
//! - The scale: a spell costs `KH2 cost / 100` of Sora's KH1 maximum, in bars, taken from the
//!   top down (the charge part first, at the next bar's worth, then each bar at its own), paid in
//!   units through KH1's own payment in steps of at most 30 (the smallest bar's worth, so each
//!   step breaks at most one bar, as KH1's payment does). Cure takes everything and needs
//!   at least one whole bar (KH2: all of the MP; the bar is the tester's rule, below).
//! - **KH2's cost kinds in KH1's engine** (the tester, thread kh2-magic-and-reflect-native m6:
//!   "we need to give kh1's menu and mp the ability to allow costs like kh2 cure ... this needs to
//!   be a feature in the engine ... so that kh2 cure can call for it. as design choice lets make
//!   sure that you have at least 1 bar of blue mp before you can cast kh2 cure"). KH1's MP check
//!   `0x1402A4C10 (actor, cost)` and payment `0x1402A5B60 (actor, cost)` (Steam, Checked
//!   (decomp)) know two kinds of cost: under 100 charge units, from 100 up whole bars × 100. They
//!   are hooked to know two more, tagged in the cost's high bits (KH1's own costs are all under
//!   0x10000, so no KH1 cost is read as one): `KIND_ALL | bars` (every unit of MP, needing at
//!   least `bars` whole bars: KH2's Cure) and `KIND_SHARE | percent` (a share of the maximum,
//!   this module's scale: KH2's other spells). Anything that asks KH1's own check or payment with
//!   such a cost gets it: the usable check `0x14028EFD0 (actor, spell)` (the Magic menu's greying
//!   in its fill `0x1402944C0`, the shortcuts, the command step's casts `0x1402A3600` /
//!   `0x1402A3DB0` / `0x1402A4150`), hooked to read the KH2 kind of a spell whose Config row is
//!   KH2 for base Sora, and our KH2 casts' payment (`pay`).
//! - KH2's own rule (Steam KH2, Checked (decomp)): the command's state `0x140401D60` (magic, case
//!   2) is 5 (greyed) while MP Charge runs, else, with MP under the cost, 2, which is still usable
//!   (its callers take any state under 4, `0x1403A9EA0`), unless a per-command flag
//!   (`0x1403C5940`, the bit table `0x1409ACFF4`, not identified: Assumption) greys it; the payment
//!   `0x1403D3CF0` going below 0 starts MP Charge with MP 0. So in KH2 Cure (cost 255) can be cast
//!   with any MP, even under one point, and so can every other spell with less MP than its cost
//!   (it empties the pool and starts the charge). Ours differs by the tester's choice: Cure needs
//!   a whole bar; the other spells still need their whole cost (asked); no MP Charge yet (its own
//!   thread).

use super::pose;
use std::ptr::read_unaligned as rd;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// KH2 Sora's maximum MP (00battle `plrp`, Checked (files)).
const KH2_MAX_MP: f32 = 100.0;
/// A charge unit's worth per bar index (`0x1402A4CB0`, Checked (decomp)).
const UNITS_PER_INDEX: i32 = 30;

/// The tag bits of a KH2 cost kind in a KH1 cost (`encode`); KH1's own costs never have them.
const KIND_MASK: i32 = 0x7F00_0000;
/// Every unit of MP; the low byte the whole bars needed first.
const KIND_ALL: i32 = 0x4000_0000;
/// A share of the maximum MP; the low byte the percent (`units_of`).
const KIND_SHARE: i32 = 0x2000_0000;
/// The whole (blue) bars Cure needs (the tester's design rule; KH2 needs none, see above).
pub const ALL_MIN_BARS: i32 = 1;

/// A KH2 cost: a share of the pool, or all of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cost {
    Part(u8),
    All,
}

/// KH2's cost of a spell (KH1's numbering: Fire 0, Blizzard 1, Thunder 2, Cure 3; Reflect 7).
pub fn kh2_cost(spell: u8) -> Option<Cost> {
    Some(match spell {
        0 => Cost::Part(12),
        1 => Cost::Part(15),
        2 => Cost::Part(18),
        3 => Cost::All,
        7 => Cost::Part(10),
        _ => return None,
    })
}

/// Sora's MP now: (bars, maximum, charge units).
unsafe fn mp(sora: usize) -> Option<(i32, i32, i32)> {
    if sora < 0x10000 {
        return None;
    }
    let stats = pose::ptr(rd((sora + 0x6C) as *const u32));
    if stats < 0x10000 {
        return None;
    }
    Some((rd((stats + 0x44) as *const i32), rd((stats + 0x48) as *const i32), rd((stats + 0xBE) as *const u16) as i32))
}

/// The units a cost takes from (bars `n`, maximum `max`, charge `y`), or None if it can't be paid.
pub fn units_of(cost: Cost, n: i32, max: i32, y: i32) -> Option<i32> {
    if n < 0 || max <= 0 {
        return None;
    }
    let total = (1..=n).map(|k| k * UNITS_PER_INDEX).sum::<i32>() + y.max(0);
    let mut left = match cost {
        Cost::All => return (n >= ALL_MIN_BARS && total > 0).then_some(total),
        Cost::Part(c) => c as f32 / KH2_MAX_MP * max as f32,
    };
    let mut u = 0.0f32;
    // The charge part, at the next bar's worth.
    let next = ((n + 1) * UNITS_PER_INDEX) as f32;
    let take = left.min(y.max(0) as f32 / next);
    u += take * next;
    left -= take;
    for k in (1..=n).rev() {
        if left <= 1e-4 {
            break;
        }
        let take = left.min(1.0);
        u += take * (k * UNITS_PER_INDEX) as f32;
        left -= take;
    }
    if left > 1e-4 {
        return None;
    }
    Some((u.ceil() as i32).clamp(1, total.max(1)))
}

/// The units `spell`'s KH2 cost takes from Sora now, if he can pay it.
pub unsafe fn units(sora: usize, spell: u8) -> Option<i32> {
    let (n, max, y) = mp(sora)?;
    units_of(kh2_cost(spell)?, n, max, y)
}

/// Pays `units` through KH1's own payment, at most 30 at a time.
pub unsafe fn pay_units(sora: usize, units: i32) {
    let mut left = units.max(0);
    while left > 0 {
        let step = left.min(UNITS_PER_INDEX);
        super::reflect::pay_mp(sora, step);
        left -= step;
    }
}

/// Pays `spell`'s KH2 cost if Sora can; returns the units paid. Through KH1's own payment with
/// the KH2 kind (`encode`), which the hooked payment takes (`pay_hook`); before the hook is in,
/// the units in steps.
pub unsafe fn pay(sora: usize, spell: u8) -> Option<i32> {
    let u = units(sora, spell)?;
    let before = mp(sora);
    let cost = kh2_cost(spell)?;
    if HOOKED.load(Ordering::Relaxed) {
        super::reflect::pay_mp(sora, encode(cost));
    } else {
        pay_units(sora, u);
    }
    super::log!(Magic, "KH2 MP cost of spell {spell} ({cost:?}): {u} of KH1's units paid, MP {before:?} -> {:?}", mp(sora));
    Some(u)
}

/// A KH2 limit's cost, `percent` of KH2's 100 (Sonic Rave 60, t979): paid as a spell's share when
/// Sora has it, else every unit he has (KH2: `0x1403D88E0` leaves a limit usable with MP under
/// its cost, state 2, and its payment `0x1403D3CF0` going below 0 sets MP to 0, Steam KH2, Checked
/// (decomp); the owner's round 12 answer 4 (a): "give it the ability to use limit even if its
/// below its requirement", MP Charge not built yet). None with no MP at all (KH2 has none only
/// while MP Charge runs, which greys the limit). Returns the units paid.
pub unsafe fn pay_limit(sora: usize, percent: u8) -> Option<i32> {
    let (n, max, y) = mp(sora)?;
    let before = (n, max, y);
    let cost = Cost::Part(percent);
    let paid = match units_of(cost, n, max, y) {
        Some(u) => {
            if HOOKED.load(Ordering::Relaxed) {
                super::reflect::pay_mp(sora, encode(cost));
            } else {
                pay_units(sora, u);
            }
            u
        }
        None => {
            let total = (1..=n.max(0)).map(|k| k * UNITS_PER_INDEX).sum::<i32>() + y.max(0);
            if total <= 0 {
                return None;
            }
            pay_units(sora, total);
            total
        }
    };
    super::log!(Magic, "KH2 limit cost {percent} of 100: {paid} of KH1's units paid, MP {before:?} -> {:?}", mp(sora));
    Some(paid)
}

/// A KH2 cost as a KH1 cost of its kind.
pub fn encode(cost: Cost) -> i32 {
    match cost {
        Cost::All => KIND_ALL | ALL_MIN_BARS,
        Cost::Part(p) => KIND_SHARE | p as i32,
    }
}

/// The KH2 kind in a KH1 cost, if it is one (and the bars Cure's kind needs).
pub fn decode(raw: i32) -> Option<(Cost, i32)> {
    match raw & KIND_MASK {
        KIND_ALL => Some((Cost::All, raw & 0xFF)),
        KIND_SHARE => Some((Cost::Part((raw & 0xFF) as u8), 0)),
        _ => None,
    }
}

// ---------------------------------------------------------------- in KH1's engine

/// KH1's MP check, payment and spell usable check (Steam; prologues Checked (exe):
/// `mov [rsp+10h], rbp; mov [rsp+18h], rsi; push rdi; sub rsp, 20h`; `push r14; sub rsp, 30h;
/// mov qword [rsp+20h], -2`; `mov [rsp+8], rbx; mov [rsp+10h], rsi; push rdi; sub rsp, 20h`).
const CHECK_STEAM: usize = 0x2A4C10;
const CHECK_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x20];
const PAY_STEAM: usize = 0x2A5B60;
const PAY_PROLOGUE: [u8; 15] = [0x41, 0x56, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF];
const USABLE_STEAM: usize = 0x28EFD0;
const USABLE_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20];
/// KH1's spell slots (0x38 bytes each; `+0x18` the spell's data, whose `+0x30` is its cost),
/// Steam, as the usable check reads them (Checked (decomp)).
const SPELLS_STEAM: usize = 0x2D22B80;
static ORIG_CHECK: AtomicUsize = AtomicUsize::new(0);
static ORIG_PAY: AtomicUsize = AtomicUsize::new(0);
static ORIG_USABLE: AtomicUsize = AtomicUsize::new(0);
/// The check and the payment both take the KH2 kinds.
static HOOKED: AtomicBool = AtomicBool::new(false);
static BASE: AtomicUsize = AtomicUsize::new(0);

/// The KH2 kinds are in KH1's engine (both hooks in).
pub fn hooked() -> bool {
    HOOKED.load(Ordering::Relaxed)
}

/// KH1's check with a KH2 kind: whether the actor can pay it now.
unsafe extern "system" fn check_hook(actor: usize, cost: i32) -> u32 {
    if let Some((kind, _)) = decode(cost) {
        let Some((n, max, y)) = mp(actor) else { return 0 };
        return units_of(kind, n, max, y).is_some() as u32;
    }
    let f: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(ORIG_CHECK.load(Ordering::Acquire));
    f(actor, cost)
}

/// KH1's payment with a KH2 kind: the units it takes, through KH1's own payment (so its rules,
/// the free-MP debug flag, the save mirror `+0xC8`, stay KH1's). Cure's kind: the charge part in
/// steps under 100 (no bar breaks: each step is within what is left), then every bar as one cost
/// of bars × 100, again while KH1's MP change leaves one (its half-change rule, `0x1402A4A00`).
unsafe extern "system" fn pay_hook(actor: usize, cost: i32) {
    let orig: unsafe extern "system" fn(usize, i32) = std::mem::transmute(ORIG_PAY.load(Ordering::Acquire));
    let Some((kind, _)) = decode(cost) else { return orig(actor, cost) };
    let Some((n, max, y)) = mp(actor) else { return };
    let Some(u) = units_of(kind, n, max, y) else { return };
    match kind {
        Cost::All => {
            let mut left = y.max(0);
            while left > 0 {
                let step = left.min(99);
                orig(actor, step);
                left -= step;
            }
            for _ in 0..4 {
                let Some((n, _, _)) = mp(actor) else { break };
                if n <= 0 {
                    break;
                }
                orig(actor, n * 100);
            }
        }
        Cost::Part(_) => {
            let mut left = u;
            while left > 0 {
                let step = left.min(UNITS_PER_INDEX);
                orig(actor, step);
                left -= step;
            }
        }
    }
}

/// KH1's usable check for a spell: for base Sora's Fire .. Cure with their Config row at KH2, the
/// spell data's cost reads the KH2 kind for the call (put back after), so KH1's own gates (the
/// spell learned and loaded, magic allowed) and its MP check decide, by KH2's cost.
unsafe extern "system" fn usable_hook(actor: usize, spell: i32) -> u64 {
    let orig: unsafe extern "system" fn(usize, i32) -> u64 = std::mem::transmute(ORIG_USABLE.load(Ordering::Acquire));
    let kh2 = HOOKED.load(Ordering::Relaxed)
        && actor > 0x10000
        && actor == super::takeover::sora_actor()
        && (0..=3).contains(&spell)
        && super::formmagic::base_kh2_spell(spell as u8);
    let Some(cost) = kh2.then(|| kh2_cost(spell as u8)).flatten() else { return orig(actor, spell) };
    let base = BASE.load(Ordering::Relaxed);
    let data = rd::<u64>((base + SPELLS_STEAM + spell as usize * 0x38 + 0x18) as *const u64) as usize;
    if data < 0x10000 || !super::crash::readable(data + 0x34) {
        return orig(actor, spell);
    }
    let was = rd::<i32>((data + 0x30) as *const i32);
    std::ptr::write_unaligned((data + 0x30) as *mut i32, encode(cost));
    let r = orig(actor, spell);
    std::ptr::write_unaligned((data + 0x30) as *mut i32, was);
    r
}

/// The three hooks (Steam only: the addresses are Steam's). The KH2 kinds count as in only with
/// both the check and the payment hooked.
pub unsafe fn install() {
    if super::addr::Build::current() != super::addr::Build::Steam {
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    let c = super::hook::install(base + CHECK_STEAM, &CHECK_PROLOGUE, check_hook as *const () as usize, &ORIG_CHECK, "KH1's MP check (KH2's cost kinds)").is_some();
    let p = super::hook::install(base + PAY_STEAM, &PAY_PROLOGUE, pay_hook as *const () as usize, &ORIG_PAY, "KH1's MP payment (KH2's cost kinds)").is_some();
    if !(c && p) {
        return;
    }
    HOOKED.store(true, Ordering::Relaxed);
    super::hook::install(base + USABLE_STEAM, &USABLE_PROLOGUE, usable_hook as *const () as usize, &ORIG_USABLE, "KH1's spell usable check (KH2's costs by the Magic rows)");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fire_on_two_bars() {
        // 12% of 2 bars = 0.24 bar, from the top bar (worth 60): 14.4 -> 15 units.
        assert_eq!(units_of(Cost::Part(12), 2, 2, 0), Some(15));
        // 12% of 3 bars = 0.36 bar: the charge toward bar 3 (30 of its 90, a third of a bar),
        // then 0.0267 of bar 2 (worth 60): 30 + 1.6 -> 32 units.
        assert_eq!(units_of(Cost::Part(12), 2, 3, 30), Some(32));
    }

    #[test]
    fn not_enough() {
        assert_eq!(units_of(Cost::Part(18), 0, 5, 10), None);
        assert_eq!(units_of(Cost::All, 0, 5, 0), None);
        // Cure needs a whole bar (the tester's rule), whatever the charge part holds.
        assert_eq!(units_of(Cost::All, 0, 5, 29), None);
    }

    #[test]
    fn kinds_round_trip() {
        assert_eq!(decode(encode(Cost::All)), Some((Cost::All, ALL_MIN_BARS)));
        assert_eq!(decode(encode(Cost::Part(12))), Some((Cost::Part(12), 0)));
        // KH1's own costs are no kind.
        for c in [0, 1, 30, 99, 100, 300, 0xFFFF] {
            assert_eq!(decode(c), None);
        }
    }

    #[test]
    fn cure_takes_everything() {
        assert_eq!(units_of(Cost::All, 3, 5, 7), Some(30 + 60 + 90 + 7));
    }
}
