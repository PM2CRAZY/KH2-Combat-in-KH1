//! How a character reacts to being hit, and how attacks clash.
//!
//! Read from KH2 Final Mix's executable (the generic reaction shared by the
//! player and enemies, and the attack-versus-attack table); see
//! `docs/combat-model.md`, "Hit reactions". Only the rules are written here.
//! Directions are horizontal unit vectors on (x, z); heights are y-up.

/// Numbers the reactions read (KH2 `sstm`). [`Default`] holds the values
/// found in KH2 Final Mix; `kh2-data` fills them from the install.
#[derive(Clone, Debug, PartialEq)]
pub struct ReactionParams {
    /// Hit-back distance when the attack gives none.
    pub hit_back: f32,
    /// Hit-back distance for flinch reactions when the attack gives none.
    pub hit_back_small: f32,
    /// Launch defaults: vertical speed and horizontal speed (per frame).
    pub blow_up: f32,
    pub blow_speed: f32,
    /// Hit-back distance per unit of launch speed.
    pub blow_to_hit_back: f32,
    /// While launched, per physics tick: horizontal speed x this, but not
    /// below `blow_min_xz` x the starting speed; rising speed x `blow_brake_up`.
    pub blow_brake_xz: f32,
    pub blow_min_xz: f32,
    pub blow_brake_up: f32,
    /// Hop speed per unit of hit-back distance for hits taken in the air.
    pub hit_back_to_jump: f32,
    /// Gravity added per physics tick while launched, and the fall cap.
    pub gravity: f32,
    pub fall_max: f32,
}

impl Default for ReactionParams {
    fn default() -> Self {
        ReactionParams {
            hit_back: 200.0,
            hit_back_small: 200.0,
            blow_up: 24.0,
            blow_speed: 40.0,
            blow_to_hit_back: 8.0,
            blow_brake_xz: 0.8,
            blow_min_xz: 0.1,
            blow_brake_up: 0.8,
            hit_back_to_jump: 0.048,
            gravity: crate::KH2_GRAVITY,
            fall_max: 16.0,
        }
    }
}

/// Motion ids the reactions play (the same numbers for Sora and enemies).
pub mod motion {
    /// Ground flinch: hit from the front, front (alternate), from behind,
    /// from behind (alternate).
    pub const FLINCH: [u16; 4] = [10, 11, 12, 13];
    /// Air flinch: from the front, from behind.
    pub const AIR_FLINCH: [u16; 2] = [14, 15];
    /// Launched: knocked backward (hit from the front) / forward. The motion
    /// after each (+1) plays on landing.
    pub const BLOW_FRONT: u16 = 16;
    pub const BLOW_BACK: u16 = 18;
    /// Reaction type 0xe: from the front / from behind.
    pub const SPECIAL: [u16; 2] = [23, 24];
    /// Pushed back by a guard (party members).
    pub const GUARD_REPEL: u16 = 42;
    /// Repelled by a wall or another attack: ground / air.
    pub const REPELLED: u16 = 41;
    pub const REPELLED_AIR: u16 = 43;
}

/// What an ATKP reaction byte means: the game folds the 19 ids into these
/// groups (exe).
pub fn category(reaction: u8) -> u8 {
    match reaction {
        1 | 0xc => 1,
        3 | 6 | 8 | 10 | 0xd | 0x10 | 0x12 => 3,
        4 | 7 | 9 | 0xb | 0xf | 0x11 => 4,
        r => r,
    }
}

/// One hit, as the defender sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// ATKP reaction id.
    pub reaction: u8,
    /// ATKP knockback values (32767 = none for the first).
    pub knockback: (i16, i16),
    /// Push direction: the attacker's facing.
    pub dir: [f32; 2],
    /// Which way to flinch on an alternate-capable body part (0 = default).
    pub part: u8,
}

/// Hit-back distance for a hit (exe): the attack's first knockback value,
/// else the table default; for launches, the launch speed x
/// `blow_to_hit_back`; halved for reactions 0xc and 0xd.
pub fn hit_back_distance(hit: &Hit, p: &ReactionParams) -> f32 {
    let (k1, k2) = hit.knockback;
    let cat = category(hit.reaction);
    let mut d = if cat == 4 {
        let speed = if k1 != 0 || k2 != 0 { k2 as f32 } else { p.blow_speed };
        speed.abs() * p.blow_to_hit_back
    } else {
        let base = if cat == 3 { p.hit_back_small } else { p.hit_back };
        if k1 != i16::MAX { k1 as f32 } else { base }
    };
    if matches!(hit.reaction, 0xc | 0xd) {
        d *= 0.5;
    }
    d
}

/// What to do with a hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Response {
    /// No reaction (the damage still applies).
    None,
    /// Pushed back only.
    Push { distance: f32 },
    /// Play a flinch motion, then idle; optionally pushed back.
    Flinch { motion: u16, push: Option<f32> },
    /// In the air: face along `face`, play `motion`, then fall, while
    /// hopping 50 up and drifting along the push at `hop_speed`.
    AirFlinch { motion: u16, face: [f32; 2], hop_speed: f32 },
    /// Launched: face along `face`, play `motion` (+1 on landing), with the
    /// starting velocity `vel_h` (per frame) and `up` (per frame).
    Blow { motion: u16, face: [f32; 2], vel_h: [f32; 2], up: f32 },
}

fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

/// The game's generic reaction (shared by Sora and enemies). `facing` is
/// the defender's forward vector; `has` tells whether a motion exists (the
/// game falls back to the other variant when one is missing).
pub fn respond(hit: &Hit, facing: [f32; 2], airborne: bool, has: impl Fn(u16) -> bool, p: &ReactionParams) -> Response {
    let front = dot(facing, hit.dir) < 0.0;
    let pick = |a: u16, b: u16, first: bool| {
        let m = if first { a } else { b };
        if has(m) { m } else if first { b } else { a }
    };
    let flinch = || {
        let base = if front { 0 } else { 2 };
        let alt = !matches!(hit.part, 0 | 2 | 3 | 4 | 5) as usize;
        let i = base + alt;
        let m = motion::FLINCH[i];
        if has(m) { m } else { motion::FLINCH[i ^ 1] }
    };
    match category(hit.reaction) {
        1 => Response::Push { distance: hit_back_distance(hit, p) },
        2 => Response::Flinch { motion: flinch(), push: None },
        3 => {
            let d = hit_back_distance(hit, p);
            if airborne {
                let face = if front { [-hit.dir[0], -hit.dir[1]] } else { hit.dir };
                Response::AirFlinch {
                    motion: pick(motion::AIR_FLINCH[0], motion::AIR_FLINCH[1], front),
                    face,
                    hop_speed: d * p.hit_back_to_jump,
                }
            } else {
                Response::Flinch { motion: flinch(), push: Some(d) }
            }
        }
        4 => {
            let (k1, k2) = hit.knockback;
            let (up, speed) = if k1 == 0 && k2 == 0 { (p.blow_up, p.blow_speed) } else { (k1 as f32, k2 as f32) };
            let face = if front { [-hit.dir[0], -hit.dir[1]] } else { hit.dir };
            let m = if front { motion::BLOW_FRONT } else { motion::BLOW_BACK };
            Response::Blow { motion: m, face, vel_h: [hit.dir[0] * speed, hit.dir[1] * speed], up }
        }
        0xe => Response::Flinch { motion: pick(motion::SPECIAL[1], motion::SPECIAL[0], !front), push: Some(hit_back_distance(hit, p)) },
        _ => Response::None,
    }
}

/// A launch in progress (the game's blow action).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blow {
    pub vel_h: [f32; 2],
    /// Vertical speed, y-up, per frame.
    pub vel_y: f32,
    min_speed: f32,
}

impl Blow {
    pub fn new(vel_h: [f32; 2], up: f32, p: &ReactionParams) -> Blow {
        let s = (vel_h[0] * vel_h[0] + vel_h[1] * vel_h[1]).sqrt();
        Blow { vel_h, vel_y: up, min_speed: s * p.blow_min_xz }
    }

    /// One physics tick of `dt` frames: brakes, then returns the
    /// displacement for the tick.
    pub fn tick(&mut self, dt: f32, p: &ReactionParams) -> [f32; 3] {
        let s = (self.vel_h[0] * self.vel_h[0] + self.vel_h[1] * self.vel_h[1]).sqrt();
        if s > 1e-6 {
            let k = (s * p.blow_brake_xz).max(self.min_speed) / s;
            self.vel_h = [self.vel_h[0] * k, self.vel_h[1] * k];
        }
        if self.vel_y > 0.0 {
            self.vel_y *= p.blow_brake_up;
        }
        self.vel_y = (self.vel_y - p.gravity).max(-p.fall_max);
        [self.vel_h[0] * dt, self.vel_y * dt, self.vel_h[1] * dt]
    }
}

/// How two overlapping bodies share the push that separates them (exe):
/// each moves in inverse proportion to its weight (objentry weight; Sora
/// and the Shadow are both 100). Weight 0 means immovable. Returns the
/// fractions of the overlap `a` and `b` each take.
pub fn push_shares(weight_a: f32, weight_b: f32) -> (f32, f32) {
    match (weight_a > 0.0, weight_b > 0.0) {
        (true, true) => (weight_b / (weight_a + weight_b), weight_a / (weight_a + weight_b)),
        (false, true) => (0.0, 1.0),
        (true, false) => (1.0, 0.0),
        (false, false) => (0.0, 0.0),
    }
}

/// Attack-versus-attack: which ATKP kinds stop each other (exe table;
/// kinds 0 normal, 1 pierce armor, 2 guard, 3 special guard, 4 special,
/// 5 cure, 6 full cure).
pub fn clashes(a: u8, b: u8) -> bool {
    const T: [[bool; 7]; 7] = [
        [true, false, true, true, false, false, false],
        [false, false, false, true, false, false, false],
        [true, false, false, false, false, false, false],
        [true, true, false, false, false, false, false],
        [false; 7],
        [false; 7],
        [false; 7],
    ];
    (a as usize) < 7 && (b as usize) < 7 && T[a as usize][b as usize]
}

/// An attack's angle limit for clashing (exe): with a reflect range of `r`
/// degrees (0 = any), the other attack's push direction must point against
/// the owner's facing (turned by `angle` x 2 degrees) by more than
/// `cos(r)`.
pub fn clash_angle_ok(range_deg: u8, angle: i8, owner_facing_yaw: f32, other_dir: [f32; 2]) -> bool {
    if range_deg == 0 {
        return true;
    }
    let yaw = owner_facing_yaw + (angle as f32 * 2.0).to_radians();
    let f = [yaw.sin(), yaw.cos()];
    dot(other_dir, f) < -(range_deg as f32).to_radians().cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(reaction: u8, k: (i16, i16), dir: [f32; 2]) -> Hit {
        Hit { reaction, knockback: k, dir, part: 0 }
    }

    #[test]
    fn categories() {
        assert_eq!(category(3), 3);
        assert_eq!(category(7), 4);
        assert_eq!(category(0xc), 1);
        assert_eq!(category(2), 2);
    }

    #[test]
    fn hit_back_uses_knockback_or_defaults() {
        let p = ReactionParams::default();
        assert_eq!(hit_back_distance(&hit(3, (125, 0), [0.0, 1.0]), &p), 125.0);
        assert_eq!(hit_back_distance(&hit(3, (i16::MAX, 0), [0.0, 1.0]), &p), 200.0);
        assert_eq!(hit_back_distance(&hit(7, (3, 45), [0.0, 1.0]), &p), 360.0);
        assert_eq!(hit_back_distance(&hit(0xd, (100, 0), [0.0, 1.0]), &p), 50.0);
    }

    #[test]
    fn front_and_back_flinches() {
        let p = ReactionParams::default();
        let all = |_| true;
        // Defender faces +z; attacker pushes along -z: hit from the front.
        assert_eq!(respond(&hit(3, (125, 0), [0.0, -1.0]), [0.0, 1.0], false, all, &p), Response::Flinch { motion: 10, push: Some(125.0) });
        assert_eq!(respond(&hit(3, (125, 0), [0.0, 1.0]), [0.0, 1.0], false, all, &p), Response::Flinch { motion: 12, push: Some(125.0) });
        // Missing motion: the other variant.
        assert_eq!(respond(&hit(3, (125, 0), [0.0, 1.0]), [0.0, 1.0], false, |m| m != 12, &p), Response::Flinch { motion: 13, push: Some(125.0) });
    }

    #[test]
    fn launch_faces_the_attacker_and_uses_knockback_as_speed() {
        let p = ReactionParams::default();
        match respond(&hit(7, (3, 45), [0.0, -1.0]), [0.0, 1.0], false, |_| true, &p) {
            Response::Blow { motion, face, vel_h, up } => {
                assert_eq!(motion, 16);
                assert_eq!(face, [-0.0, 1.0]);
                assert_eq!(vel_h, [0.0, -45.0]);
                assert_eq!(up, 3.0);
            }
            r => panic!("{r:?}"),
        }
    }

    #[test]
    fn blow_brakes_to_the_floor_speed() {
        let p = ReactionParams::default();
        let mut b = Blow::new([45.0, 0.0], 3.0, &p);
        let mut x = 0.0;
        for _ in 0..40 {
            x += b.tick(2.0, &p)[0];
        }
        assert!((b.vel_h[0] - 4.5).abs() < 1e-3, "floored at 10% of the start: {}", b.vel_h[0]);
        assert!(x > 300.0);
    }

    #[test]
    fn heavier_bodies_move_less() {
        assert_eq!(push_shares(100.0, 100.0), (0.5, 0.5));
        assert_eq!(push_shares(100.0, 300.0), (0.75, 0.25));
        assert_eq!(push_shares(100.0, 0.0), (1.0, 0.0));
    }

    #[test]
    fn clash_table() {
        assert!(clashes(0, 0));
        assert!(clashes(2, 0) && clashes(0, 2));
        assert!(!clashes(2, 1), "a guard doesn't stop pierce-armor attacks");
        assert!(clashes(3, 1));
    }
}
