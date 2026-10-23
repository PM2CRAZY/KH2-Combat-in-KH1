//! KH2's pad sticks, as the game builds them each frame (Steam `0x14039c180`; the dead zone is
//! set in `0x14039b7f0`). KH1's PC pad step builds its sticks the same way (Steam
//! `0x14028b4f0`, dead zone 60 set in `0x14028afc0`). `docs/kh2-pad.md`.
//!
//! Each axis is the pad byte minus 127.5. A dead zone of 60 is taken off toward the centre
//! (an axis within 60 of the centre is 0) and the rest divided by 67.5. The pair is then
//! normalized: the pad keeps the unit direction and, beside it, the length clamped to 1 (the
//! tilt). Pad y is positive pulled down.

/// The per-axis dead zone, in pad steps of 127.5.
pub const DEAD: f32 = 60.0;

/// One axis from its pad byte, dead zone off, -1..1 (the corner can exceed 1 before the
/// tilt is clamped).
pub fn axis(b: u8) -> f32 {
    let v = b as f32 - 127.5;
    let v = if v > 0.0 { (v - DEAD).max(0.0) } else { (v + DEAD).min(0.0) };
    v / (127.5 - DEAD)
}

/// A stick from its two pad bytes: (unit direction, tilt 0..1). Idle: ([0, 0], 0).
pub fn stick(x: u8, y: u8) -> ([f32; 2], f32) {
    let v = [axis(x), axis(y)];
    let len = v[0].hypot(v[1]);
    if len == 0.0 {
        return ([0.0; 2], 0.0);
    }
    ([v[0] / len, v[1] / len], len.min(1.0))
}

/// The stick as one vector: its unit direction times its tilt (length at most 1).
pub fn stick_vec(x: u8, y: u8) -> [f32; 2] {
    let (d, t) = stick(x, y);
    [d[0] * t, d[1] * t]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_inner_sixty_of_each_axis_is_dead() {
        assert_eq!(stick(128, 128), ([0.0; 2], 0.0));
        assert_eq!(stick(127, 128), ([0.0; 2], 0.0));
        // 127.5 + 60 = 187.5: 187 is still dead, 188 just past it.
        assert_eq!(axis(187), 0.0);
        assert!(axis(188) > 0.0 && axis(188) < 0.01);
        assert_eq!(axis(68), 0.0);
        assert!(axis(67) < 0.0);
        assert_eq!(axis(255), 1.0);
        assert_eq!(axis(0), -1.0);
    }

    #[test]
    fn the_pair_is_a_unit_direction_and_a_clamped_tilt() {
        // Full right: (1, 0), tilt 1.
        assert_eq!(stick(255, 128), ([1.0, 0.0], 1.0));
        // Pulled down (toward the camera): y positive.
        assert_eq!(stick(128, 255), ([0.0, 1.0], 1.0));
        // The corner: both axes 1, so the direction is diagonal and the tilt clamps to 1.
        let (d, t) = stick(255, 0);
        assert!((d[0] - 0.70711).abs() < 1e-4 && (d[1] + 0.70711).abs() < 1e-4);
        assert_eq!(t, 1.0);
        // A small x within the dead zone is dropped: the direction snaps to the axis.
        assert_eq!(stick(170, 255).0, [0.0, 1.0]);
        // Half way past the dead zone: tilt 0.5.
        let half = (127.5 + 60.0 + 33.75) as u8;
        assert!((stick(half, 128).1 - 0.4963).abs() < 1e-3);
        let v = stick_vec(255, 0);
        assert!((v[0].hypot(v[1]) - 1.0).abs() < 1e-6);
    }
}
