//! Sora's no-damage time after a hit, while KH2 has him (`win/hurt.rs`, `docs/kh1-rehit.md`).
//!
//! Both games give it, and the same length:
//! - KH1 (its exe): every one of the player's reactions sets the actor's flag `0x80` and a
//!   30-frame timer (`0x140295DA0`); the actor task counts the timer down once the reaction's
//!   action is over, and clears the flag at 0. While it is set KH1's damage step takes no HP
//!   (`0x1402BCD30`) and its hit pass plays no reaction but a clash. So an attack lands once per
//!   reaction: the rest of that frame's records, every later part of the same attack (a slam's
//!   shockwave, a pillar re-armed on its interval) and every other attack deal nothing until 30
//!   frames after he is back on his feet.
//! - KH2 (its exe, `combat_core::HitRecord`): a hit sets the player's group timer to 30, and the
//!   timer holds while he plays a reaction motion (10..=39: flinch, blow, landing, getting up).
//!   While it runs only the last hit's group lands (a combo of one attacker; KH1's attacks have
//!   no such groups, and KH1's rule is the one they were made for).
//!
//! KH2 plays its own reactions in KH1's place, so KH1 never set its flag: this keeps that time
//! instead, from KH2's motions. Pure (no game memory), so it is tested here.

/// Frames of no damage after the reaction ends (KH1's timer, KH2's group timer).
pub const AFTER: u64 = 30;

/// KH2's reaction motions, during which the no-damage time holds (Sora's flinches, air
/// flinches, blows, their landings and getting up).
pub fn reaction_motion(motion: u32) -> bool {
    (10..=39).contains(&motion)
}

/// What a hit record says about the attack it came from (KH1's record, `0x1402BD480`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Swing {
    /// The attack's owner (record `+4`, the attack's `+0x34`: the actor that made it).
    pub from: u32,
    /// The attack data entry (record `+0x2C`).
    pub data: u32,
    /// The attack's own bit (record `+0x28`): one attack object.
    pub bit: u32,
}

/// How a hit that comes during the no-damage time relates to the one that landed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repeat {
    /// The same attack object again (re-armed on its interval, or another frame of it).
    SameAttack,
    /// The same attacker's same attack data from another attack object (another part).
    SamePart,
    /// The same attacker, another attack (a shockwave after a slam).
    SameAttacker,
    /// Someone else.
    Other,
}

/// The no-damage time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mercy {
    /// The last frame of the hit or of its reaction, while the time runs.
    held: Option<u64>,
    /// The hit that started it.
    swing: Option<Swing>,
}

impl Mercy {
    pub const fn new() -> Self {
        Self { held: None, swing: None }
    }

    /// One look at KH2's motion at `frame` (`u32::MAX` when KH1 has him: no KH2 reaction).
    pub fn see(&mut self, frame: u64, motion: u32) {
        if let Some(h) = self.held {
            if reaction_motion(motion) {
                self.held = Some(h.max(frame));
            } else if frame.saturating_sub(h) >= AFTER {
                self.held = None;
            }
        }
    }

    /// Whether a hit at `frame` deals nothing.
    pub fn active(&self, frame: u64) -> bool {
        self.held.is_some_and(|h| frame.saturating_sub(h) < AFTER)
    }

    /// A hit landed at `frame` (its damage taken, its reaction played).
    pub fn landed(&mut self, frame: u64, swing: Swing) {
        self.held = Some(frame);
        self.swing = Some(swing);
    }

    /// Frames since the reaction ended (0 while it plays).
    pub fn since(&self, frame: u64) -> u64 {
        self.held.map_or(0, |h| frame.saturating_sub(h))
    }

    /// How `s` relates to the hit that landed.
    pub fn repeat(&self, s: Swing) -> Repeat {
        match self.swing {
            Some(l) if l.from == s.from && l.bit == s.bit && l.data == s.data => Repeat::SameAttack,
            Some(l) if l.from == s.from && l.data == s.data => Repeat::SamePart,
            Some(l) if l.from == s.from => Repeat::SameAttacker,
            _ => Repeat::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: u32 = u32::MAX;
    const SLAM: Swing = Swing { from: 7, data: 0x100, bit: 1 };

    fn run(m: &mut Mercy, frames: std::ops::Range<u64>, motion: u32) {
        for f in frames {
            m.see(f, motion);
        }
    }

    #[test]
    fn nothing_without_a_hit() {
        let mut m = Mercy::new();
        run(&mut m, 0..50, 10);
        assert!(!m.active(50));
    }

    #[test]
    fn the_same_frame_lands_once() {
        let mut m = Mercy::new();
        m.landed(100, SLAM);
        assert!(m.active(100), "the frame's other records deal nothing");
    }

    #[test]
    fn held_through_the_reaction_then_30_frames() {
        // A blow at 0: launched (16) to frame 40, landing and getting up (17) to 120, idle.
        let mut m = Mercy::new();
        m.landed(0, SLAM);
        m.see(0, 0); // (KH2's motion still the one before the hit, this frame)
        run(&mut m, 1..41, 16);
        run(&mut m, 41..121, 17);
        assert!(m.active(120));
        run(&mut m, 121..150, 0);
        assert!(m.active(149), "29 frames after he is up");
        m.see(150, 0);
        assert!(!m.active(150), "30 frames after");
        assert!(!m.active(400));
    }

    #[test]
    fn a_short_flinch() {
        let mut m = Mercy::new();
        m.landed(10, SLAM);
        run(&mut m, 11..40, 10);
        run(&mut m, 40..69, 0);
        assert!(m.active(68));
        m.see(69, 0);
        assert!(!m.active(69));
    }

    #[test]
    fn kh1_having_him_counts_down() {
        let mut m = Mercy::new();
        m.landed(0, SLAM);
        run(&mut m, 1..30, NONE);
        assert!(m.active(29));
        assert!(!m.active(30));
    }

    #[test]
    fn missed_frames_do_not_stretch_it() {
        // The hurt pass not seen for a while: the time is still from the last reaction frame.
        let mut m = Mercy::new();
        m.landed(0, SLAM);
        run(&mut m, 1..20, 10);
        assert!(!m.active(19 + AFTER));
    }

    #[test]
    fn a_new_hit_after_it_starts_it_again() {
        let mut m = Mercy::new();
        m.landed(0, SLAM);
        run(&mut m, 1..100, 0);
        assert!(!m.active(100));
        let other = Swing { from: 9, data: 0x200, bit: 4 };
        m.landed(100, other);
        assert!(m.active(100));
        assert_eq!(m.repeat(SLAM), Repeat::Other);
    }

    #[test]
    fn repeats_are_told_apart() {
        let mut m = Mercy::new();
        m.landed(0, SLAM);
        assert_eq!(m.repeat(SLAM), Repeat::SameAttack);
        assert_eq!(m.repeat(Swing { bit: 2, ..SLAM }), Repeat::SamePart);
        assert_eq!(m.repeat(Swing { bit: 3, data: 0x170, ..SLAM }), Repeat::SameAttacker);
        assert_eq!(m.repeat(Swing { from: 8, ..SLAM }), Repeat::Other);
    }

    #[test]
    fn a_slam_and_its_shockwave() {
        // The report: a slam whose shockwave (other attack objects of the same attacker, and the
        // same one re-armed) meets him on many frames. One lands; nothing else until he is up
        // and 30 frames have gone; then another enemy's hit lands.
        let mut m = Mercy::new();
        let mut landed = vec![];
        let mut motion = 0u32;
        for f in 0..200u64 {
            m.see(f, motion);
            let hits: Vec<Swing> = match f {
                0 => vec![SLAM, Swing { bit: 2, data: 0x170, ..SLAM }],
                1..=40 => vec![Swing { bit: 2 + (f as u32 % 3), data: 0x170, ..SLAM }],
                60 | 80 => vec![SLAM],
                170 => vec![Swing { from: 9, data: 0x200, bit: 5 }],
                _ => vec![],
            };
            for h in hits {
                if !m.active(f) {
                    m.landed(f, h);
                    landed.push((f, h.from));
                }
            }
            // KH2: blown from frame 1 to 40, getting up to 130, then idle.
            motion = match f {
                0..=39 => 16,
                40..=129 => 17,
                _ => 0,
            };
        }
        assert_eq!(landed, vec![(0, 7), (170, 9)]);
    }
}
