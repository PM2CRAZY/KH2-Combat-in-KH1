//! KH2's Drive gauge and form gauge (rules from plain-language notes on the
//! executable; see `docs/drive-model.md`).
//!
//! - **Drive gauge.** Bars of 100 points, up to `max` bars (9 at most).
//!   Sora's hits add the attack's drive value (×0.75 on normal enemies,
//!   ×2 while MP charges), truncated.
//! - **Form gauge.** Entering a form sets it to `(form level + 3) × 600`
//!   frames. It drains 1 per frame in battle (0.5 outside), and each hit
//!   Sora lands drains `15 × drive` (×0.2 on normal enemies).
//! - **Anti Form.** A counter rises with every drive that isn't Final:
//!   0% Anti below 5, 10% from 5, 25% from 10. Final lowers it by 10,
//!   Anti by 6.

/// Form ids (the game's).
pub mod form {
    pub const NONE: u8 = 0;
    pub const VALOR: u8 = 1;
    pub const WISDOM: u8 = 2;
    pub const LIMIT: u8 = 3;
    pub const MASTER: u8 = 4;
    pub const FINAL: u8 = 5;
    pub const ANTI: u8 = 6;
}

/// `sstm` numbers the gauge uses (03system pref).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DriveParams {
    /// Drive from hits on normal enemies (0.75).
    pub enemy_rate: f32,
    /// Drive multiplier while MP charges (2.0).
    pub mp_charge_rate: f32,
    /// Form-gauge frames per bar (600).
    pub bar_frames: f32,
    /// Form-gauge drain outside battle (0.5).
    pub relax_rate: f32,
    /// Form-gauge drain per drive point of a hit (15).
    pub hit_drain: f32,
    /// ...times this on normal enemies (0.2).
    pub hit_drain_enemy: f32,
    /// Drive orbs and the like refill a form gauge at this rate (12).
    pub orb_rate: f32,
    /// Anti Form's gauge level (5): its gauge is `level − 2 + 3` bars.
    pub anti_level: u8,
    /// Anti counter decreases: after Anti (6), after Final (10).
    pub anti_sub: u32,
    pub final_sub: u32,
    /// Damage Sora takes in Anti Form (×1.5).
    pub anti_damage: f32,
}

impl Default for DriveParams {
    fn default() -> Self {
        DriveParams {
            enemy_rate: 0.75,
            mp_charge_rate: 2.0,
            bar_frames: 600.0,
            relax_rate: 0.5,
            hit_drain: 15.0,
            hit_drain_enemy: 0.2,
            orb_rate: 12.0,
            anti_level: 5,
            anti_sub: 6,
            final_sub: 10,
            anti_damage: 1.5,
        }
    }
}

/// The form gauge while in a form.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FormGauge {
    pub form: u8,
    /// Frames left / at the start.
    pub left: f32,
    pub max: f32,
}

impl FormGauge {
    /// Bars shown on the HUD.
    pub fn bars(&self, p: &DriveParams) -> f32 {
        self.left / p.bar_frames
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Drive {
    pub params: DriveParams,
    /// Full bars and the points in the current one (0..=99; 100 = full).
    pub bars: u8,
    pub points: u8,
    pub max: u8,
    /// Some while in a form.
    pub gauge: Option<FormGauge>,
    /// The Anti counter (kept in the Anti save record).
    pub anti_counter: u32,
    /// Drive Boost copies (×(1 + 0.2 n) while MP charges) and Form Boost
    /// copies (form gauge drains ×0.8 each).
    pub drive_boost: u8,
    pub form_boost: u8,
}

impl Drive {
    pub fn new(max: u8) -> Drive {
        Drive { params: DriveParams::default(), bars: 0, points: 0, max: max.min(9), gauge: None, anti_counter: 0, drive_boost: 0, form_boost: 0 }
    }

    pub fn full(&self) -> bool {
        self.bars >= self.max
    }

    /// Adds drive points (exe): 100 per bar; a full gauge holds at
    /// `points = 100`. Negative amounts take from the current bar first.
    pub fn add_points(&mut self, n: i32) {
        if n < 0 {
            let v = self.points as i32 + n;
            if v < 0 && self.bars > 0 {
                self.bars -= 1;
                self.points = (self.points as i32 + 100 + n).clamp(0, 99) as u8;
            } else {
                self.points = v.max(0) as u8;
            }
            return;
        }
        let mut v = self.points as i32 + n;
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

    /// Pays a drive's cost (exe): a part-filled bar survives unless the
    /// gauge was full or short.
    pub fn spend(&mut self, cost: u8) {
        if self.bars >= self.max || self.bars < cost {
            self.points = 0;
        }
        self.bars = self.bars.saturating_sub(cost);
    }

    pub fn can_afford(&self, cost: u8) -> bool {
        self.gauge.is_none() && self.bars >= cost
    }

    /// One of Sora's hits landed with ATKP drive value `d`. `enemy` is a
    /// normal enemy (not a boss).
    pub fn on_hit(&mut self, d: u8, enemy: bool, mp_charging: bool) {
        if d == 0 {
            return;
        }
        let p = self.params;
        match &mut self.gauge {
            None => {
                let mut x = d as f32;
                if enemy {
                    x *= p.enemy_rate;
                }
                if mp_charging {
                    x *= p.mp_charge_rate * (1.0 + 0.2 * self.drive_boost as f32);
                }
                self.add_points(x as i32);
            }
            Some(g) => {
                let mut x = d as f32 * p.hit_drain;
                if enemy {
                    x *= p.hit_drain_enemy;
                }
                g.left = (g.left - x).clamp(0.0, g.max);
            }
        }
    }

    /// Drive orb picked up (1 small, 3 big). In a form it refills the form
    /// gauge (×12), except Anti's, which it drains.
    pub fn orb(&mut self, value: u8) {
        let p = self.params;
        match &mut self.gauge {
            None => self.add_points(value as i32),
            Some(g) => {
                let x = value as f32 * p.orb_rate * if g.form == form::ANTI { -1.0 } else { 1.0 };
                g.left = (g.left + x).clamp(0.0, g.max);
            }
        }
    }

    /// The Anti roll for a requested form (exe). `roll` is 0..100. Anti
    /// only happens in battle.
    pub fn anti_roll(&mut self, requested: u8, battle: bool, roll: u32) -> u8 {
        if requested == form::FINAL {
            self.anti_counter = self.anti_counter.saturating_sub(self.params.final_sub);
            return requested;
        }
        let rate = self.anti_rate();
        self.anti_counter += 1;
        if roll < rate && battle {
            self.anti_counter = self.anti_counter.saturating_sub(self.params.anti_sub);
            return form::ANTI;
        }
        requested
    }

    /// Current Anti chance in percent (fmlv Anti rows: 10/5 → 25%/10%).
    pub fn anti_rate(&self) -> u32 {
        match self.anti_counter {
            c if c >= 10 => 25,
            c if c >= 5 => 10,
            _ => 0,
        }
    }

    /// Starts the form gauge (form level 1..=7).
    pub fn enter(&mut self, f: u8, level: u8) {
        let level = if f == form::ANTI { self.params.anti_level.saturating_sub(2) } else { level.max(1) };
        let max = (level as f32 + 3.0) * self.params.bar_frames;
        self.gauge = Some(FormGauge { form: f, left: max, max });
    }

    /// One frame in a form. Returns true when the gauge has run out.
    pub fn tick(&mut self, battle: bool) -> bool {
        let p = self.params;
        let boost = 0.8f32.powi(self.form_boost as i32);
        match &mut self.gauge {
            Some(g) => {
                let x = if battle { 1.0 } else { p.relax_rate } * boost;
                g.left = (g.left - x).max(0.0);
                g.left <= 0.0
            }
            None => false,
        }
    }

    /// Fills the gauge (a testing aid, not a game rule): every Drive bar
    /// out of a form, the form gauge in one.
    pub fn refill(&mut self) {
        match &mut self.gauge {
            None => {
                self.bars = self.max;
                self.points = 100;
            }
            Some(g) => g.left = g.max,
        }
    }

    pub fn revert(&mut self) {
        self.gauge = None;
    }

    pub fn form(&self) -> u8 {
        self.gauge.map_or(form::NONE, |g| g.form)
    }
}

/// Form levels and EXP (00battle fmlv).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct FormLevel {
    pub level: u8,
    pub exp: u32,
}

impl FormLevel {
    /// Adds EXP with the level table `to_next[level − 1]` (EXP from each
    /// level to the next) up to `cap`. Returns the levels gained.
    pub fn add(&mut self, n: u32, to_next: &[u32], cap: u8) -> u8 {
        let mut gained = 0;
        self.exp += n;
        while self.level < cap {
            let Some(&need) = to_next.get(self.level.max(1) as usize - 1) else { break };
            if need == 0 || self.exp < need {
                break;
            }
            self.exp -= need;
            self.level += 1;
            gained += 1;
        }
        if self.level >= cap {
            self.exp = 0;
        }
        gained
    }
}

/// The growth ability level a form has at a form level (fmlv: Lv1 → 1,
/// Lv3 → 2, Lv5 → 3, Lv7 → 4 for every form).
pub fn growth_at(level: u8) -> u8 {
    match level {
        0 => 0,
        1..=2 => 1,
        3..=4 => 2,
        5..=6 => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_fill_bars_and_cap() {
        let mut d = Drive::new(3);
        d.add_points(250);
        assert_eq!((d.bars, d.points), (2, 50));
        d.add_points(80);
        assert_eq!((d.bars, d.points), (3, 100), "full holds at 100");
        d.add_points(-30);
        assert_eq!((d.bars, d.points), (3, 70));
    }

    #[test]
    fn refill_fills_the_drive_or_the_form_gauge() {
        let mut d = Drive::new(5);
        d.add_points(140);
        d.refill();
        assert!(d.full() && d.points == 100);
        d.spend(3);
        d.enter(form::VALOR, 1);
        for _ in 0..500 {
            d.tick(true);
        }
        d.refill();
        let g = d.gauge.unwrap();
        assert_eq!(g.left, g.max);
    }

    #[test]
    fn spending_keeps_a_part_bar_unless_full() {
        let mut d = Drive::new(5);
        d.add_points(340);
        d.spend(3);
        assert_eq!((d.bars, d.points), (0, 40));
        let mut d = Drive::new(3);
        d.add_points(1000);
        d.spend(3);
        assert_eq!((d.bars, d.points), (0, 0));
    }

    #[test]
    fn hits_add_drive_truncated() {
        let mut d = Drive::new(5);
        d.on_hit(1, true, false);
        assert_eq!(d.points, 0, "0.75 truncates to 0");
        d.on_hit(4, true, false);
        assert_eq!(d.points, 3);
        d.on_hit(4, true, true);
        assert_eq!(d.points, 9, "MP charge doubles");
        d.on_hit(4, false, false);
        assert_eq!(d.points, 13, "bosses give the full value");
    }

    #[test]
    fn form_gauge_lasts_level_plus_three_bars_and_drains() {
        let mut d = Drive::new(9);
        d.enter(form::VALOR, 1);
        assert_eq!(d.gauge.unwrap().max, 2400.0);
        for _ in 0..100 {
            d.tick(true);
        }
        assert_eq!(d.gauge.unwrap().left, 2300.0);
        d.tick(false);
        assert_eq!(d.gauge.unwrap().left, 2299.5);
        d.on_hit(4, true, false);
        assert!((d.gauge.unwrap().left - (2299.5 - 12.0)).abs() < 1e-3);
        d.enter(form::ANTI, 7);
        assert_eq!(d.gauge.unwrap().max, 3600.0, "Anti: 6 bars");
        d.orb(1);
        assert_eq!(d.gauge.unwrap().left, 3600.0 - 12.0, "orbs drain Anti");
    }

    #[test]
    fn anti_chance_follows_the_counter() {
        let mut d = Drive::new(9);
        for _ in 0..5 {
            assert_eq!(d.anti_roll(form::VALOR, true, 0), form::VALOR);
        }
        assert_eq!(d.anti_rate(), 10);
        assert_eq!(d.anti_roll(form::VALOR, false, 0), form::VALOR, "never outside battle");
        assert_eq!(d.anti_roll(form::VALOR, true, 9), form::ANTI);
        assert_eq!(d.anti_counter, 1);
        d.anti_counter = 12;
        assert_eq!(d.anti_roll(form::FINAL, true, 0), form::FINAL);
        assert_eq!(d.anti_counter, 2);
    }

    #[test]
    fn levels_follow_the_table() {
        let valor = [80, 160, 280, 448, 560, 672];
        let mut l = FormLevel { level: 1, exp: 0 };
        assert_eq!(l.add(239, &valor, 7), 1);
        assert_eq!((l.level, l.exp), (2, 159));
        assert_eq!(l.add(10_000, &valor, 7), 5);
        assert_eq!((l.level, l.exp), (7, 0));
        assert_eq!(growth_at(1), 1);
        assert_eq!(growth_at(5), 3);
        assert_eq!(growth_at(7), 4);
    }
}
