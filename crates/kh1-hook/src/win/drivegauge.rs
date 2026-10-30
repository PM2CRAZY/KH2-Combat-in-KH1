//! KH2's Drive gauge kept in Sora's KH1 save (the owner, the main-branch port's round 2, "go
//! ahead"; answer 9 (a): main's bytes, so a main save keeps its gauge). The rules are
//! `crate::drivegauge`'s (KH2's, Steam, Checked (decomp)); notes in `docs/kh1-drive-forms-native.md`
//! §13.
//!
//! - **Save bytes**: the bars at `save + 0x37` (bit 7 "written"; unwritten reads as a full gauge),
//!   the points at `save + 0xAB`. They are the spare byte after the weapon in Sora's and Donald's
//!   party records (`+0x36`, `+0x74 + 0x36`), part of KH1's save field list and saved with it; no
//!   function of the Steam exe that loads the save's pointer (`0x142868BA0`) touches `+0x37` or
//!   `+0xAB`, and the byte accesses at those displacements elsewhere are other records' (Checked
//!   (exe), an objdump scan, 2026-10-29). Assumption: nothing reaches them through a pointer handed
//!   in from elsewhere (a static scan can't see that). Testing used neither (Checked (code)).
//! - **The maximum**: 9 with Progression > Drive Gauge Growth off (the owner, round 2 answer 2 (b);
//!   KH2 raises it only by bonus levels, `0x1403EDF90`, which KH1 has none of yet: "we'll add bonus
//!   levels later"); on (t935, answer 3 (a)): the owner's six fights, 3 + one for each won
//!   (`crate::storyforms::gauge_fights`), at most 9. A full gauge stays full as it rises.

use super::log;
use crate::drivegauge::{self, Gauge};
use std::sync::atomic::{AtomicU8, Ordering};

/// Progression > Drive Gauge Growth: 0 not read yet, 1 off, 2 on.
static GROWTH: AtomicU8 = AtomicU8::new(0);

/// Progression > Drive Gauge Growth (the ini's `gauge_growth`, off by default).
pub fn growth() -> bool {
    match GROWTH.load(Ordering::Relaxed) {
        0 => {
            let on = super::settings::get().gauge_growth;
            GROWTH.store(1 + on as u8, Ordering::Relaxed);
            on
        }
        v => v == 2,
    }
}

/// Switches Drive Gauge Growth (the Config menu), kept in the ini.
pub fn set_growth(on: bool) {
    GROWTH.store(1 + on as u8, Ordering::Relaxed);
    super::settings::save("gauge_growth", if on { "on" } else { "off" });
    log!(Forms, "Config menu: Drive Gauge Growth {}", if on { "On" } else { "Off" });
}

pub const BARS_OFF: usize = 0x37;
pub const POINTS_OFF: usize = 0x74 + 0x37;

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn save() -> Option<usize> {
    let base = super::takeover::game_base();
    if base == 0 {
        return None;
    }
    super::menu::save_block(base).filter(|s| super::crash::readable(s + POINTS_OFF))
}

/// The gauge's maximum now.
pub fn max() -> u8 {
    if !growth() {
        return drivegauge::MAX_BARS;
    }
    unsafe {
        let Some(s) = save().filter(|s| super::crash::readable(s + crate::storyforms::SPAN_END - 1)) else { return drivegauge::START_BARS };
        let fights = crate::storyforms::gauge_fights(std::slice::from_raw_parts(s as *const u8, crate::storyforms::SPAN_END));
        (drivegauge::START_BARS + fights).min(drivegauge::MAX_BARS)
    }
}

/// The gauge now (`None`: no save).
pub unsafe fn get() -> Option<Gauge> {
    let s = save()?;
    Some(drivegauge::unpack(rd::<u8>(s + BARS_OFF), rd::<u8>(s + POINTS_OFF), max()))
}

unsafe fn put(g: &Gauge) {
    let Some(s) = save() else { return };
    let (b, p) = drivegauge::pack(g);
    std::ptr::write_unaligned((s + BARS_OFF) as *mut u8, b);
    std::ptr::write_unaligned((s + POINTS_OFF) as *mut u8, p);
}

/// Full bars now.
pub unsafe fn bars() -> u8 {
    get().map_or(0, |g| g.bars)
}

/// KH2's add of `n` points (`0x1403C0AD0`).
pub unsafe fn add(n: i32, why: &str) {
    let Some(mut g) = get() else { return };
    let before = g;
    g.add(n);
    if g != before {
        put(&g);
        log!(Forms, "Drive gauge {n:+} ({why}): {}.{:02} -> {}.{:02} of {}", before.bars, before.points.min(99), g.bars, g.points.min(99), g.max);
    }
}

/// KH2's pay of `cost` bars (`0x1403C2780`). False if short.
pub unsafe fn pay(cost: u8) -> bool {
    let Some(mut g) = get() else { return false };
    if g.bars < cost {
        return false;
    }
    let before = g;
    g.pay(cost);
    put(&g);
    log!(Forms, "Drive gauge: {cost} bars paid ({} -> {} of {})", before.bars, g.bars, g.max);
    true
}

/// Every bar full (KH2's refill `0x1403C2020`).
pub unsafe fn refill(why: &str) {
    let Some(g) = get() else { return };
    let full = Gauge::full(g.max);
    if g != full {
        put(&full);
        log!(Forms, "Drive gauge refilled ({why}): {} -> {} of {}", g.bars, full.bars, full.max);
    }
}

/// KH1's contact made a hit record for Sora's `attack` on `target` (`probe::hit`, t932): KH2's add
/// for a hit (the attack owner's hit callbacks `0x1403A7480` / `0x1403A8800` with the ATKP row,
/// then `0x1403A9CF0`, Checked (decomp)): only on an enemy (KH2: class bit `+0x6C8` 0x10; KH1: an
/// actor of kind 3, `+6`), the ATKP's drive value (byte `+0x29`, `Atkp::drive_drain`) for KH2's
/// moves (`kh1rows::kh2_atkp`), KH1's own attack data's `+0x2C` for KH1's moves (the value KH1's
/// contact `0x1402BF330` charges MP with, Checked (decomp); the owner, round 2 answer 6 (a)).
/// × 0.75 on a normal enemy (answer 7: KH1's boss signal isn't found yet, so every KH1 enemy is
/// normal for now, Assumption). Out of a form only: in a form the hit drains the form gauge
/// instead (`form::hit_drain`).
pub unsafe fn hit(attack: usize, target: usize) {
    if attack < 0x10000 || target < 0x10000 || rd::<u8>(target + 6) != 3 {
        return;
    }
    let value = match super::kh1rows::kh2_atkp(attack) {
        Some(a) => a.drive_drain,
        None => {
            let data = super::pose::ptr(rd::<u32>(attack + 0x5C));
            if data < 0x10000 {
                return;
            }
            rd::<i32>(data + 0x2C).clamp(0, 255) as u8
        }
    };
    if value == 0 {
        return;
    }
    if super::form::in_form() || super::form::busy() {
        super::form::hit_drain(value, true);
        return;
    }
    let n = drivegauge::hit_points(value, true);
    if n > 0 {
        add(n, "a hit");
    }
}
