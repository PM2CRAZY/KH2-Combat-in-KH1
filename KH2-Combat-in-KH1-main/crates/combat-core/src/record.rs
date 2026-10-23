//! Per-defender hit bookkeeping, modelled on KH2's.
//!
//! Every live attack has a small id. A defender remembers which ids have
//! already hit it, so one attack hits once, until either the attack ends
//! or its re-hit interval runs out. It also counts consecutive hits from
//! the same attack **group**: the attack's row is picked by that count
//! (the "level" byte of an ATKP row is this count, not the attacker's
//! level), which is how a spell or combo hits weaker or harder on later
//! hits. Checked against the game's executable; see `docs/combat-model.md`.

use std::collections::HashSet;

/// A defender's record of recent hits.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HitRecord {
    /// Attacks (by swing id) that have hit and may not hit again yet.
    hit: HashSet<u64>,
    /// The group of the last admitted hit, and how many hits in a row came
    /// from it (0 for the first).
    group: Option<i64>,
    count: u32,
    /// While positive, only the last group may hit (set to 30 on hits for
    /// defenders that use it; the game does this for some objects).
    timer: f32,
    /// Up to four pending re-hits: the attack and the frames left.
    rehit: [Option<(u64, f32)>; 4],
}

/// The result of [`HitRecord::admit`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Admit {
    /// The hit counts (deals damage and a reaction). When false, the game
    /// still registers the contact but without effect.
    pub effective: bool,
    /// Which row to use: hits so far in a row from this group (0-based).
    pub count: u32,
}

impl HitRecord {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether this attack already hit and is waiting (the game's first
    /// check, done before anything else).
    pub fn has_hit(&self, swing: u64) -> bool {
        self.hit.contains(&swing)
    }

    /// Registers a contact from attack `swing` in `group`. `heals` attacks
    /// (Cure kinds) are always effective. `uses_timer` defenders refuse
    /// other groups for 30 frames after a hit.
    ///
    /// Call [`HitRecord::has_hit`] first; then [`HitRecord::mark`] with the
    /// chosen row's interval.
    pub fn admit(&mut self, group: i64, heals: bool, uses_timer: bool) -> Admit {
        let same = self.group == Some(group);
        let effective = heals || same || group == 0xff || self.timer <= 0.0;
        if effective {
            if same {
                self.count += 1;
            } else {
                self.group = Some(group);
                self.count = 0;
            }
            if uses_timer {
                self.timer = 30.0;
            }
        }
        Admit { effective, count: self.count }
    }

    /// Marks the attack as having hit. With a non-zero `interval` (frames)
    /// it may hit again once that runs out (if a re-hit slot is free).
    pub fn mark(&mut self, swing: u64, interval: u16) {
        if interval != 0 {
            if let Some(slot) = self.rehit.iter_mut().find(|s| s.is_none()) {
                *slot = Some((swing, interval as f32));
            }
        }
        self.hit.insert(swing);
    }

    /// The attack ended: it no longer counts as having hit.
    pub fn forget(&mut self, swing: u64) {
        self.hit.remove(&swing);
    }

    /// One frame. `frozen` stops the group timer (the game does while the
    /// defender plays a hit-reaction motion).
    pub fn tick(&mut self, frametime: f32, frozen: bool) {
        if self.timer > 0.0 && !frozen {
            self.timer -= frametime;
        }
        for slot in self.rehit.iter_mut() {
            if let Some((swing, left)) = slot {
                *left -= frametime;
                if *left <= 0.0 {
                    self.hit.remove(swing);
                    *slot = None;
                }
            }
        }
    }

    /// Hits in a row from the current group (0-based).
    pub fn count(&self) -> u32 {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_group_counts_up_and_new_group_resets() {
        let mut r = HitRecord::new();
        assert_eq!(r.admit(7, false, false), Admit { effective: true, count: 0 });
        r.mark(1, 0);
        assert!(r.has_hit(1));
        assert_eq!(r.admit(7, false, false).count, 1);
        assert_eq!(r.admit(9, false, false).count, 0);
    }

    #[test]
    fn interval_allows_a_rehit() {
        // Fire: interval 18.
        let mut r = HitRecord::new();
        r.admit(1, false, false);
        r.mark(5, 18);
        for _ in 0..17 {
            r.tick(1.0, false);
        }
        assert!(r.has_hit(5));
        r.tick(1.0, false);
        assert!(!r.has_hit(5));
        assert_eq!(r.admit(1, false, false).count, 1);
    }

    #[test]
    fn timer_blocks_other_groups() {
        let mut r = HitRecord::new();
        r.admit(1, false, true);
        assert!(!r.admit(2, false, true).effective);
        assert!(r.admit(1, false, true).effective);
        assert!(r.admit(2, true, true).effective, "heals always land");
        for _ in 0..30 {
            r.tick(1.0, true);
        }
        assert!(!r.admit(3, false, true).effective, "frozen while reacting");
        for _ in 0..30 {
            r.tick(1.0, false);
        }
        assert!(r.admit(3, false, true).effective);
    }
}
