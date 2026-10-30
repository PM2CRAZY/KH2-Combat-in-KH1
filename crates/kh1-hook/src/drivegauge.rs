//! KH2's Drive gauge, for KH1 (the owner, the main-branch port's round 2: "go ahead"; notes in
//! `docs/kh1-drive-forms-native.md` §13 and `docs/main-port-inventory.md` round 2). Pure rules,
//! kept in Sora's KH1 save by `win/drivegauge.rs`. Steam KH2, Checked (decomp) unless marked:
//!
//! - **The gauge**: full bars, the points (0..100) toward the next bar, a maximum (status `+0x1B0`
//!   points, `+0x1B1` bars, `+0x1B2` maximum). A new game starts full, 3 of 3 (`0x1403A0E70`).
//! - **Adding points** (`0x1403C0AD0 (status, n)`): points over 99 carry into a bar while under the
//!   maximum; at the maximum the points hold at 100. A loss below 0 breaks a bar (points + 100) if
//!   there is one, else the points stay at 0.
//! - **Paying** (`0x1403C2780 (status, cost)`): bars less the cost, floored at 0; the points are
//!   cleared when the gauge was full (bars at the maximum) or short of the cost.
//! - **The form gauge** (entering, `0x1403C17C0`): (form level + 3) bars of `TICKS_PER_BAR` ticks
//!   (sstm `+0x68`, 600.0, Checked (files)); it drains one tick a 60 Hz frame in battle and
//!   `OUT_OF_BATTLE` (sstm `+0x6C`, 0.5, Checked (files)) outside (the player step `0x1403A92C0`); a
//!   hit Sora lands takes the ATKP's drive value × `HIT_DRAIN` (sstm `+0x74`, 15.0) ticks off it,
//!   × `NORMAL_DRAIN` (sstm `+0x64`, 0.2) on a normal enemy (Checked (files) for the numbers).
//! - **A hit's add out of a form**: the drive value, × `NORMAL_ADD` (sstm `+0x60`, 0.75) on a
//!   normal enemy, × MP Charge's factor while MP charges (KH1 has no MP Charge) × the world's factor
//!   (1.0 on KH1: Assumption, KH2's default not found), truncated (`0x1403A9CF0`).
//! - **Costs** (KH2's 03system `cmd` rows, byte `+0x1C`, Checked (files)): Valor 3, Wisdom 3, Limit
//!   4, Master 4, Final 5, Anti 9.

/// Ticks a form gauge bar lasts (sstm `+0x68`).
pub const TICKS_PER_BAR: f32 = 600.0;
/// The drain's share out of battle (sstm `+0x6C`).
pub const OUT_OF_BATTLE: f32 = 0.5;
/// Ticks a drive point of a hit takes off the form gauge (sstm `+0x74`).
pub const HIT_DRAIN: f32 = 15.0;
/// That on a normal enemy (sstm `+0x64`).
pub const NORMAL_DRAIN: f32 = 0.2;
/// A hit's add on a normal enemy (sstm `+0x60`).
pub const NORMAL_ADD: f32 = 0.75;
/// The most bars (KH2's bonus levels stop at 9, `0x1403EDF90`).
pub const MAX_BARS: u8 = 9;
/// A new game's gauge (`0x1403A0E70`).
pub const START_BARS: u8 = 3;

/// KH2's cost in bars of form `form` (KH2's form ids: 1 Valor .. 6 Anti).
pub fn cost(form: u8) -> u8 {
    match form {
        1 | 2 => 3,
        3 | 4 => 4,
        5 => 5,
        6 => 9,
        _ => 0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gauge {
    pub bars: u8,
    pub points: u8,
    pub max: u8,
}

impl Gauge {
    pub fn full(max: u8) -> Gauge {
        Gauge { bars: max, points: 100, max }
    }

    /// KH2's add (`0x1403C0AD0`).
    pub fn add(&mut self, n: i32) {
        let v = self.points as i32 + n;
        if n < 0 {
            if v < 0 {
                if self.bars > 0 {
                    self.bars -= 1;
                    self.points = (v + 100).clamp(0, 99) as u8;
                } else {
                    self.points = 0;
                }
                return;
            }
            self.points = v as u8;
            return;
        }
        let mut v = v;
        while v > 99 {
            if self.bars >= self.max {
                self.points = 100;
                return;
            }
            self.bars += 1;
            if self.bars >= self.max {
                self.points = 100;
                return;
            }
            v -= 100;
        }
        self.points = v as u8;
    }

    /// KH2's pay (`0x1403C2780`).
    pub fn pay(&mut self, cost: u8) {
        if self.bars == self.max || self.bars < cost {
            self.points = 0;
        }
        self.bars = self.bars.saturating_sub(cost);
    }

    /// A new maximum: bars and points kept within it (KH2: a full gauge stays full as the maximum
    /// rises, `0x1403EDF90`, Checked (decomp)).
    pub fn with_max(mut self, max: u8) -> Gauge {
        let max = max.clamp(1, MAX_BARS);
        let was_full = self.bars >= self.max;
        self.max = max;
        if was_full || self.bars >= max {
            self.bars = max;
            self.points = 100;
        }
        self
    }
}

/// A hit's drive points out of a form (`0x1403A9CF0`, mode 1): KH1 has no MP Charge, the world's
/// factor 1.0.
pub fn hit_points(value: u8, normal: bool) -> i32 {
    let v = value as f32 * if normal { NORMAL_ADD } else { 1.0 };
    v as i32
}

/// The form gauge's length at form level `level` (`0x1403C17C0`).
pub fn form_ticks(level: u8) -> f32 {
    (level as f32 + 3.0) * TICKS_PER_BAR
}

/// One 60 Hz frame's drain (`0x1403A92C0`).
pub fn drain(battle: bool) -> f32 {
    if battle {
        1.0
    } else {
        OUT_OF_BATTLE
    }
}

/// A hit's loss of form gauge ticks in a form (the hit callbacks, mode 2).
pub fn hit_drain(value: u8, normal: bool) -> f32 {
    value as f32 * HIT_DRAIN * if normal { NORMAL_DRAIN } else { 1.0 }
}

/// The save's bars byte: bit 7 "written", the bars below. Unwritten reads as a full gauge (KH2's
/// new game starts full; the owner, main s323: "By default, the drive gauge should be full"). Points
/// 100 are a full gauge whatever the maximum is now (KH2: a full gauge stays full as the maximum
/// rises, `0x1403EDF90`).
pub fn unpack(bars_byte: u8, points: u8, max: u8) -> Gauge {
    if bars_byte & 0x80 == 0 || points >= 100 {
        return Gauge::full(max);
    }
    let bars = (bars_byte & 0x7F).min(max);
    let points = if bars >= max { 100 } else { points.min(99) };
    Gauge { bars, points, max }
}

pub fn pack(g: &Gauge) -> (u8, u8) {
    (0x80 | g.bars.min(MAX_BARS), g.points.min(100))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_carries_and_holds_at_the_max() {
        let mut g = Gauge { bars: 1, points: 90, max: 3 };
        g.add(20);
        assert_eq!((g.bars, g.points), (2, 10));
        g.add(250);
        assert_eq!((g.bars, g.points), (3, 100));
        g.add(5);
        assert_eq!((g.bars, g.points), (3, 100));
    }

    #[test]
    fn a_loss_breaks_a_bar() {
        let mut g = Gauge { bars: 2, points: 10, max: 3 };
        g.add(-30);
        assert_eq!((g.bars, g.points), (1, 80));
        let mut g = Gauge { bars: 0, points: 10, max: 3 };
        g.add(-30);
        assert_eq!((g.bars, g.points), (0, 0));
    }

    #[test]
    fn pay_clears_points_when_full_or_short() {
        let mut g = Gauge::full(5);
        g.pay(3);
        assert_eq!((g.bars, g.points), (2, 0));
        let mut g = Gauge { bars: 4, points: 50, max: 9 };
        g.pay(3);
        assert_eq!((g.bars, g.points), (1, 50));
        let mut g = Gauge { bars: 4, points: 50, max: 9 };
        g.pay(9);
        assert_eq!((g.bars, g.points), (0, 0));
    }

    #[test]
    fn save_bytes_and_the_rules() {
        assert_eq!(unpack(0, 0, 9), Gauge::full(9));
        let g = Gauge { bars: 2, points: 40, max: 9 };
        let (b, p) = pack(&g);
        assert_eq!(unpack(b, p, 9), g);
        assert_eq!(hit_points(3, true), 2);
        assert_eq!(hit_points(3, false), 3);
        assert_eq!(form_ticks(1), 2400.0);
        assert_eq!(hit_drain(3, true), 9.0);
        assert_eq!(cost(1), 3);
        assert_eq!(cost(5), 5);
        assert_eq!(Gauge::full(3).with_max(4), Gauge::full(4));
        let (b, p) = pack(&Gauge::full(3));
        assert_eq!(unpack(b, p, 4), Gauge::full(4));
        assert_eq!(unpack(0x80 | 5, 20, 4), Gauge { bars: 4, points: 100, max: 4 });
        assert_eq!(Gauge { bars: 1, points: 5, max: 3 }.with_max(4), Gauge { bars: 1, points: 5, max: 4 });
    }
}
