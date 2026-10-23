//! The random numbers the effect programs draw.
//!
//! The game uses the C runtime's `rand` (the Microsoft linear congruential
//! generator) and builds a uniform `[0, 1]` value from the low byte of
//! three calls (`docs/kh2-effects.md` §5.7). We keep our own generator so
//! a replay with the same seed gives the same particles.

/// Microsoft C runtime `rand` plus the effect system's `rand01`.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u32,
}

impl Default for Rng {
    fn default() -> Self {
        Rng::new(1)
    }
}

impl Rng {
    /// `srand(seed)`. The C runtime starts at 1.
    pub fn new(seed: u32) -> Rng {
        Rng { state: seed }
    }

    /// One `rand()`: 15 bits.
    pub fn rand(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(214_013).wrapping_add(2_531_011);
        (self.state >> 16) & 0x7fff
    }

    /// Uniform in `[0, 1]`: three bytes, the first call's byte highest.
    pub fn rand01(&mut self) -> f32 {
        let a = self.rand() & 255;
        let b = self.rand() & 255;
        let c = self.rand() & 255;
        (a << 16 | b << 8 | c) as f32 / 16_777_215.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msvc_sequence() {
        // srand(1): the first values of the Microsoft C runtime's rand().
        let mut r = Rng::new(1);
        let v: Vec<u32> = (0..5).map(|_| r.rand()).collect();
        assert_eq!(v, vec![41, 18467, 6334, 26500, 19169]);
    }

    #[test]
    fn rand01_range() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            let x = r.rand01();
            assert!((0.0..=1.0).contains(&x));
        }
    }
}
