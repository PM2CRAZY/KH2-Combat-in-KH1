//! KH2's enemy HP gauge's rules (`dk::GAUGE_ENEMY`, Steam KH2, Checked (decomp)), pure, for KH2's
//! gauge drawn by KH1's own enemy HP bar draw (`win/enemyhp`, tm45; notes `docs/kh1-kh2-enemy-hp.md`).
//!
//! - **The values** (the object's `+0x4B08` the HP shown, `+0x4B0C` the max, `+0x4B10` the HP
//!   before the last hit, `+0x4B14` the HP now): a new target (`0x140183520`, from the target's
//!   gauge `0x1403DD440`) sets them all to its HP (at most the max); a heal (`0x140183220`, from KH2's
//!   HP change `0x1403D2C50` / `0x1403D2EB0` with a gain) sets them all to the new HP; a hit
//!   (`0x1401832A0`, the same with a loss) keeps the HP before it as "before", the new HP (at least
//!   0) as shown and now, and restarts the damage piece.
//! - **200 HP a bar**: the bars past the first are KH2's pips (`(max - 1) / 200`, at most 16 drawn).
//! - **Each frame** (`0x140183640`): the top bar is `max - pips × 200` long, 200 once the HP shown
//!   is below the top bar (or past it); the HP shown and "before" each within their own bar (200
//!   for "before" when it is on another bar than the HP); 1 KH2 unit per HP: each bar piece is
//!   scaled by its length / 100 and placed its length left of the gauge's anchor.
//! - Integer division as KH2's (toward zero: `(0 - 1) / 200` is 0).

/// HP in one bar (`0x140183640`, `0x1401832A0`, ...: 200).
pub const PER_BAR: i32 = 200;
/// KH2's pips (16 pairs of sprites, `0x14016C3A0`).
pub const PIPS: usize = 16;

/// What a value change asks the pieces for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pip {
    /// Left as it plays.
    Keep,
    /// The full pip again from its start (group 44).
    Full,
    /// The pip lost (group 46).
    Lost,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gauge {
    pub max: i32,
    /// `+0x4B08`.
    pub shown: i32,
    /// `+0x4B10`.
    pub before: i32,
    /// `+0x4B14`.
    pub now: i32,
}

/// The frame's layout (`0x140183640`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    /// Pips (bars past the top one), `+0x4B18`.
    pub pips: i32,
    /// The top bar's length (`+0x4B1C`): the frame (39) and its left end (40).
    pub frame: i32,
    /// "Before" within its bar (`+0x4B20`): the damage piece (41).
    pub before: i32,
    /// The HP shown within its bar (`+0x4B24`): the fill (43).
    pub fill: i32,
    /// The piece 42 shown (`+0x4B29`: the HP is on the top bar and there are pips).
    pub on_top: bool,
    /// Pips with their background (45) shown: `pips`, at most 16.
    pub backs: usize,
    /// Pips with their full piece (44 or 46, as last set) shown: "before"'s bar, at most 16 (none
    /// without pips).
    pub fulls: usize,
}

fn bar(v: i32) -> i32 {
    (v - 1) / PER_BAR
}

impl Gauge {
    /// A new target (`0x140183520`): the full pips restart for every bar under the HP's.
    pub fn show(hp: i32, max: i32) -> (Gauge, usize) {
        let v = hp.min(max);
        let g = Gauge { max, shown: v, before: v, now: v };
        (g, bar(v).clamp(0, PIPS as i32) as usize)
    }

    /// A heal (`0x140183220`): the same, the max kept.
    pub fn heal(&mut self, hp: i32) -> usize {
        let v = hp.min(self.max);
        self.shown = v;
        self.now = v;
        self.before = v;
        bar(v).clamp(0, PIPS as i32) as usize
    }

    /// A hit (`0x1401832A0`): the pips under "before"'s bar, full under the new HP's bar, lost from
    /// it up. The damage piece restarts (the caller's).
    pub fn hit(&mut self, hp: i32) -> [Pip; PIPS] {
        self.before = self.now;
        let v = hp.max(0);
        self.now = v;
        self.shown = v;
        let mut out = [Pip::Keep; PIPS];
        let (was, is) = (bar(self.before), bar(v));
        for (i, p) in out.iter_mut().enumerate().take(was.clamp(0, PIPS as i32) as usize) {
            *p = if (i as i32) < is { Pip::Full } else { Pip::Lost };
        }
        out
    }

    pub fn layout(&self) -> Layout {
        let pips = bar(self.max);
        let mut frame = self.max - pips * PER_BAR;
        let cb = bar(self.shown);
        let fill = self.shown - cb * PER_BAR;
        let pb = bar(self.before);
        let mut before = self.before - pb * PER_BAR;
        if frame < fill {
            frame = PER_BAR;
        }
        if pips != cb {
            frame = PER_BAR;
        }
        if pb != cb {
            before = PER_BAR;
        }
        Layout {
            pips,
            frame,
            before,
            fill,
            on_top: cb == pips && pips != 0,
            backs: pips.clamp(0, PIPS as i32) as usize,
            fulls: if pips != 0 { pb.clamp(0, PIPS as i32) as usize } else { 0 },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_bar() {
        let (g, full) = Gauge::show(120, 150);
        assert_eq!(full, 0);
        let l = g.layout();
        assert_eq!((l.pips, l.frame, l.fill, l.before, l.on_top, l.backs, l.fulls), (0, 150, 120, 120, false, 0, 0));
    }

    #[test]
    fn three_bars_and_a_hit_across_one() {
        let (mut g, full) = Gauge::show(530, 530);
        assert_eq!(full, 2);
        let l = g.layout();
        assert_eq!((l.pips, l.frame, l.fill, l.on_top, l.backs, l.fulls), (2, 130, 130, true, 2, 2));
        let pips = g.hit(380);
        assert_eq!(&pips[..3], &[Pip::Full, Pip::Lost, Pip::Keep]);
        let l = g.layout();
        // (Now on the second bar: the frame is a whole bar, "before" was on another bar.)
        assert_eq!((l.frame, l.fill, l.before, l.on_top, l.fulls), (200, 180, 200, false, 2));
        let pips = g.hit(300);
        assert_eq!(&pips[..2], &[Pip::Full, Pip::Keep]);
        assert_eq!(g.layout().before, 180);
    }

    #[test]
    fn zero_and_heal() {
        let (mut g, _) = Gauge::show(10, 10);
        g.hit(-5);
        let l = g.layout();
        assert_eq!((g.now, l.fill, l.before), (0, 0, 10));
        assert_eq!(g.heal(400), 0);
        assert_eq!((g.shown, g.before, g.now), (10, 10, 10));
    }
}
