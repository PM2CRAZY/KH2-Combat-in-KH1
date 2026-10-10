//! KH2FM's camera, from plain-language notes on the game's camera code
//! (see `docs/camera-model.md`).
//!
//! - **Free follow.** The camera looks at a point 170 above the feet. The eye
//!   sits `dist` away horizontally at yaw `yaw`. Its height follows a parabola
//!   of the distance, so zooming in also lowers the camera:
//!   `eye height above the look point = k (dist − dmin)² − 150`.
//!   While Sora runs with the right stick idle, the camera swings behind him
//!   at about 1° per frame.
//! - **Lock-on.** The camera looks 30% of the way from Sora's chest to the
//!   target, and keeps its eye around Sora's chest. It doesn't rotate while
//!   both are on screen. When one leaves the screen, the camera turns
//!   (10° per frame at most) to stand behind Sora facing the target.
//! - **Both modes.** The eye is pulled in front of walls. The look point and
//!   the eye then follow their goals with the game's spring: about 16% and 9%
//!   of the gap per 60 Hz frame.
//!
//! The game's y axis points down; this crate is **y-up**. Units are game
//! units; `dt` is in 60 Hz frames.

use glam::{Vec2, Vec3};
use std::f32::consts::{PI, TAU};

/// One row of the game's per-area camera table (row 0 is normal field and
/// battle areas).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// Horizontal field of view on a 4:3 frame (radians).
    pub fov_h: f32,
    pub dist_default: f32,
    /// Lock-on comfort distance (also the lock-on reset distance).
    pub dist_alt: f32,
    pub dist_min: f32,
    pub dist_max: f32,
    /// Height curve coefficient.
    pub k: f32,
}

impl Params {
    /// Row 0: normal field and battle areas.
    pub const FIELD: Params = Params { fov_h: 1.5, dist_default: 420.0, dist_alt: 380.0, dist_min: 250.0, dist_max: 500.0, k: 0.008 };
    /// Rows 1–3: large rooms.
    pub const WIDE: Params = Params { fov_h: 1.047_197_6, dist_default: 600.0, dist_alt: 520.0, dist_min: 400.0, dist_max: 700.0, k: 0.005 };

    /// Vertical field of view for a renderer (the game's frame is 4:3).
    pub fn fov_v(&self) -> f32 {
        2.0 * ((self.fov_h * 0.5).tan() * 0.75).atan()
    }
}

/// Look point height above the feet (free mode) and Sora's "chest".
pub const AT_UP: f32 = 170.0;
/// Eye height offset at the minimum distance: 150 below the look point.
pub const EYE_DOWN: f32 = 150.0;
/// Yaw change limit toward the goal yaw (10° per frame).
pub const YAW_LIMIT: f32 = 0.174_532_9;
/// Right-stick yaw speed at full tilt (3° per frame).
pub const STICK_YAW: f32 = 0.052_359_9;
/// Right-stick zoom speed at full tilt (units per frame).
pub const STICK_ZOOM: f32 = 8.0;
/// Swing-behind base rate (0.25° per frame) and its run-speed term.
pub const RECENTER: f32 = 0.004_363_323;
pub const RECENTER_RUN: f32 = 0.000_174_53;
/// Lock-on: the look point goes this far from Sora's chest to the target.
pub const LOCK_WEIGHT: f32 = 0.3;
/// Lock-on: the target-side offset when one of them leaves the screen.
pub const LOCK_SIDE: f32 = 0.349_066;
/// Lock-on: Sora's screen-test point (100 above the feet).
pub const LOCK_SORA_UP: f32 = 100.0;
/// Lock-on: beyond this from Sora and steeply above, the target is ignored.
pub const LOCK_FAR: f32 = 300.0;
/// The eye stops this far in front of a wall it was pulled in by.
pub const WALL_GAP: f32 = 5.0;

/// Screen-test flags (the game's 512×416 frame with 32/8-pixel margins).
pub mod screen {
    pub const TOP: u8 = 1;
    pub const BOTTOM: u8 = 2;
    pub const LEFT: u8 = 4;
    pub const RIGHT: u8 = 8;
    pub const NEAR: u8 = 0x10;
}

/// The player, as the camera sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Subject {
    pub feet: Vec3,
    /// Facing yaw: forward is (sin, 0, cos).
    pub facing: f32,
    /// The current form's run speed (Sora 8; faster forms swing quicker).
    pub run_speed: f32,
}

/// Camera controls for one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Controls {
    /// Left stick, x right / y forward, magnitude 0..1.
    pub move_stick: Vec2,
    /// Right stick, x right / y up, magnitude 0..1. Right turns the view
    /// right, up zooms in.
    pub look_stick: Vec2,
    /// Camera reset (behind Sora, default distance).
    pub reset: bool,
    /// Automatic swing-behind allowed (the Camera "Auto" option).
    pub auto: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Camera {
    pub params: Params,
    /// Direction from the look point to the eye: eye = at + dist·(sin, 0, cos).
    pub yaw: f32,
    pub goal_yaw: f32,
    pub dist: f32,
    /// Smoothed look point and eye (what is rendered).
    pub at: Vec3,
    pub eye: Vec3,
    at_goal: Vec3,
    eye_goal: Vec3,
    prev_at_goal: Vec3,
    prev_eye_goal: Vec3,
    last_feet: Option<Vec3>,
    /// Lock-on framing is active.
    pub locked: bool,
}

/// `approach_angle` (exe): turn `cur` toward `goal` by at most `step`.
pub fn approach_angle(cur: f32, goal: f32, step: f32) -> f32 {
    let d = wrap(goal - cur).clamp(-step, step);
    wrap(cur + d)
}

/// Wraps an angle into [−π, π).
pub fn wrap(a: f32) -> f32 {
    (a + PI).rem_euclid(TAU) - PI
}

/// The game's camera spring: moves `cur` toward `goal`, closing `2.5 r` of
/// the gap per frame, with a small term for the goal's own motion and a
/// dead band of about `band` units.
pub fn spring(cur: Vec3, goal: Vec3, prev_goal: Vec3, r: f32, band: f32) -> Vec3 {
    let v = cur - goal;
    let len = v.length();
    if len == 0.0 {
        return cur;
    }
    let n = v / len;
    let s = r * (0.5 * v.dot((prev_goal - goal) * r) / len + (band - len) * 2.5);
    cur + n * s
}

impl Camera {
    /// A camera behind the subject at the default distance.
    pub fn new(params: Params, s: &Subject) -> Camera {
        let yaw = wrap(s.facing + PI);
        let mut c = Camera {
            params,
            yaw,
            goal_yaw: yaw,
            dist: params.dist_default,
            at: Vec3::ZERO,
            eye: Vec3::ZERO,
            at_goal: Vec3::ZERO,
            eye_goal: Vec3::ZERO,
            prev_at_goal: Vec3::ZERO,
            prev_eye_goal: Vec3::ZERO,
            last_feet: None,
            locked: false,
        };
        let at = s.feet + Vec3::Y * AT_UP;
        c.at = at;
        c.eye = c.place(at);
        c.at_goal = c.at;
        c.eye_goal = c.eye;
        c.prev_at_goal = c.at;
        c.prev_eye_goal = c.eye;
        c
    }

    /// Turns the camera to `yaw` at once (no swing): the eye and its goals are placed there.
    /// For a host handing over from another camera that was looking that way.
    pub fn snap_yaw(&mut self, yaw: f32) {
        self.yaw = wrap(yaw);
        self.goal_yaw = self.yaw;
        self.eye = self.place(self.at);
        self.eye_goal = self.eye;
        self.prev_eye_goal = self.eye;
    }

    /// Horizontal direction the camera looks along (from eye to look
    /// point), as a yaw: forward is (sin, 0, cos).
    pub fn view_yaw(&self) -> f32 {
        let d = self.at - self.eye;
        d.x.atan2(d.z)
    }

    /// The eye for anchor `p` at the current yaw and distance.
    fn place(&self, p: Vec3) -> Vec3 {
        let k = self.params.k * (self.dist - self.params.dist_min).powi(2) - EYE_DOWN;
        p + Vec3::new(self.yaw.sin() * self.dist, k, self.yaw.cos() * self.dist)
    }

    /// Screen test of `p` against the current (smoothed) view: flags from
    /// [`screen`], 0 when comfortably on screen.
    pub fn screen_test(&self, p: Vec3) -> u8 {
        let f = (self.at - self.eye).normalize_or_zero();
        let right = f.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(f);
        let d = p - self.eye;
        let z = d.dot(f);
        if z < 1.0 {
            return screen::NEAR;
        }
        let th = (self.params.fov_h * 0.5).tan();
        let x = d.dot(right) / (z * th);
        let y = d.dot(up) / (z * th * 0.75);
        // 512×416 with 32 px side and 8 px top/bottom margins.
        let (mx, my) = (1.0 - 32.0 / 256.0, 1.0 - 8.0 / 208.0);
        let mut flags = 0;
        if x < -mx {
            flags |= screen::LEFT;
        }
        if x > mx {
            flags |= screen::RIGHT;
        }
        if y > my {
            flags |= screen::TOP;
        }
        if y < -my {
            flags |= screen::BOTTOM;
        }
        flags
    }

    fn manual(&mut self, c: &Controls, dt: f32) -> bool {
        let p = self.params;
        let r = c.look_stick;
        if r.x != 0.0 {
            // Right turns the view right: the eye swings the other way.
            self.yaw = wrap(self.yaw - r.x * STICK_YAW * dt);
            self.goal_yaw = self.yaw;
        }
        if r.y != 0.0 {
            self.dist = (self.dist - r.y * STICK_ZOOM * dt).clamp(p.dist_min, p.dist_max);
        }
        r != Vec2::ZERO
    }

    /// One frame. `lock` is the lock-on target's camera point, if locked.
    /// `wall` casts a ray against the map and returns the hit point and the
    /// surface normal.
    pub fn update(&mut self, s: &Subject, c: &Controls, lock: Option<Vec3>, dt: f32, wall: &dyn Fn(Vec3, Vec3) -> Option<(Vec3, Vec3)>) {
        let p = self.params;
        let chest = s.feet + Vec3::Y * AT_UP;
        let moved = self.last_feet.is_some_and(|l| (l.x, l.z) != (s.feet.x, s.feet.z));
        self.last_feet = Some(s.feet);
        let behind = wrap(s.facing + PI);
        let at_goal;
        match lock {
            None => {
                self.locked = false;
                if c.reset {
                    self.goal_yaw = behind;
                    self.dist = p.dist_default;
                    if c.move_stick != Vec2::ZERO {
                        // A hard cut when moving.
                        self.yaw = behind;
                    }
                } else {
                    let turned = self.manual(c, dt);
                    let lmag = c.move_stick.length().min(1.0);
                    // Stick y in the game is positive when pulled back.
                    let dir = c.move_stick.normalize_or_zero();
                    if c.auto && moved && lmag > 0.0 && !turned && -dir.y < 0.9 {
                        let step = 2.0 * (dir.x.abs() + 1.0) * (dir.y.abs() + 1.0) * lmag * (RECENTER + (s.run_speed - 8.0) * RECENTER_RUN) * dt;
                        self.yaw = approach_angle(self.yaw, behind, step.max(0.0));
                        self.goal_yaw = self.yaw;
                        let gap = p.dist_default - self.dist;
                        if gap.abs() >= 1.0 {
                            self.dist = (self.dist + gap.signum() * 0.5 * dt).clamp(p.dist_min, p.dist_max);
                        }
                    }
                }
                at_goal = chest;
            }
            Some(t) => {
                self.locked = true;
                let a = chest + (t - chest) * LOCK_WEIGHT;
                let q = s.feet + Vec3::Y * LOCK_SORA_UP;
                let st = self.screen_test(t);
                let sq = self.screen_test(q);
                let bottom = |f: u8| f & (screen::TOP | screen::BOTTOM) == screen::BOTTOM;
                if bottom(st) {
                    self.dist -= 2.0 * dt;
                }
                if bottom(sq) {
                    self.dist -= 2.0 * dt;
                }
                self.dist = self.dist.clamp(p.dist_min, p.dist_max);
                let qa = q - a;
                if qa.length() <= LOCK_FAR || qa.normalize_or_zero().y >= -0.5 {
                    if !bottom(st) && !bottom(sq) {
                        let gap = p.dist_alt - self.dist;
                        self.dist = (self.dist + gap.clamp(-dt, dt)).clamp(p.dist_min, p.dist_max);
                    }
                    let off = if st != 0 { st } else { sq };
                    if off != 0 {
                        // Behind Sora on the line from the look point through
                        // him; a target escaping sideways pulls 20° further.
                        let base = (s.feet.x - self.at_goal.x).atan2(s.feet.z - self.at_goal.z);
                        let side = if off & (screen::LEFT | screen::RIGHT) != 0 { LOCK_SIDE * wrap(base - self.yaw).signum() } else { 0.0 };
                        self.goal_yaw = wrap(base + side);
                    }
                    at_goal = a;
                } else {
                    at_goal = chest;
                }
                if c.reset {
                    self.goal_yaw = (s.feet.x - self.at_goal.x).atan2(s.feet.z - self.at_goal.z);
                    self.dist = p.dist_alt;
                } else {
                    self.manual(c, dt);
                }
            }
        }
        self.yaw = approach_angle(self.yaw, self.goal_yaw, YAW_LIMIT * dt);
        // The eye is placed from Sora's chest in both modes.
        let mut eye_goal = self.place(chest);
        if let Some((hit, n)) = wall(chest, eye_goal) {
            eye_goal = hit + n * WALL_GAP;
        }
        self.prev_at_goal = self.at_goal;
        self.prev_eye_goal = self.eye_goal;
        self.at_goal = at_goal;
        self.eye_goal = eye_goal;
        // The game's rates are per 30 Hz frame, halved at 60 Hz.
        let rate = (0.07 + (s.run_speed - 8.0) * 0.0006) * 0.5;
        let steps = dt.max(0.0);
        let (mut at, mut eye) = (self.at, self.eye);
        let mut left = steps;
        while left > 0.0 {
            let k = left.min(1.0);
            at = at.lerp(spring(at, self.at_goal, self.prev_at_goal, rate + 0.03, 1.0), k);
            eye = eye.lerp(spring(eye, self.eye_goal, self.prev_eye_goal, rate, 1.0), k);
            left -= 1.0;
        }
        if let Some((hit, n)) = wall(chest, eye) {
            eye = hit + n * WALL_GAP;
        }
        self.at = at;
        self.eye = eye;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: Vec3, _: Vec3) -> Option<(Vec3, Vec3)> {
        None
    }

    fn sora(feet: Vec3, facing: f32) -> Subject {
        Subject { feet, facing, run_speed: 8.0 }
    }

    #[test]
    fn height_follows_the_distance_curve() {
        let mut c = Camera::new(Params::FIELD, &sora(Vec3::ZERO, 0.0));
        // Default 420: eye 251.2 above the feet, 420 behind.
        assert!((c.eye.y - 251.2).abs() < 0.01, "{}", c.eye);
        assert!((c.eye.z + 420.0).abs() < 0.01);
        c.dist = 250.0;
        assert!((c.place(Vec3::Y * AT_UP).y - 20.0).abs() < 1e-3);
        assert!((Params::FIELD.fov_v().to_degrees() - 69.87).abs() < 0.05);
    }

    #[test]
    fn running_sideways_swings_behind_about_one_degree_per_frame() {
        let mut c = Camera::new(Params::FIELD, &sora(Vec3::ZERO, 0.0));
        let ctl = Controls { move_stick: Vec2::new(1.0, 0.0), auto: true, ..Default::default() };
        let facing = -std::f32::consts::FRAC_PI_2;
        let y0 = c.yaw;
        let mut feet = Vec3::ZERO;
        for _ in 0..10 {
            feet.x -= 8.0;
            c.update(&sora(feet, facing), &ctl, None, 1.0, &none);
        }
        let turned = wrap(c.yaw - y0).abs().to_degrees();
        // The first frame has no previous position, so no "moved".
        assert!((turned - 9.0).abs() < 0.05, "{turned}");
    }

    #[test]
    fn spring_closes_sixteen_and_nine_percent() {
        let a = spring(Vec3::ZERO, Vec3::X * 100.0, Vec3::X * 100.0, 0.065, 1.0);
        assert!((a.x - 100.0 * 0.1625 + 0.1625).abs() < 0.01, "{a}");
        let e = spring(Vec3::ZERO, Vec3::X * 100.0, Vec3::X * 100.0, 0.035, 1.0);
        assert!((e.x - 8.75 + 0.0875).abs() < 0.01, "{e}");
    }

    #[test]
    fn lock_on_turns_to_bring_an_off_screen_target_in_and_stops() {
        let s = sora(Vec3::ZERO, 0.0);
        let mut c = Camera::new(Params::FIELD, &s);
        // Target directly behind the camera.
        let t = Vec3::new(0.0, 100.0, -900.0);
        assert_ne!(c.screen_test(t), 0);
        for _ in 0..120 {
            c.update(&s, &Controls::default(), Some(t), 1.0, &none);
        }
        assert_eq!(c.screen_test(t), 0, "target back on screen");
        assert_eq!(c.screen_test(Vec3::Y * LOCK_SORA_UP), 0, "Sora on screen");
        let yaw = c.yaw;
        for _ in 0..30 {
            c.update(&s, &Controls::default(), Some(t), 1.0, &none);
        }
        assert!((c.yaw - yaw).abs() < 1e-4, "no rotation while both are on screen");
        assert!((c.dist - 380.0).abs() < 1.0, "drifts to the lock distance: {}", c.dist);
    }

    #[test]
    fn walls_pull_the_eye_in() {
        let s = sora(Vec3::ZERO, 0.0);
        let mut c = Camera::new(Params::FIELD, &s);
        // A wall at z = -200 facing +z.
        let wall = |a: Vec3, b: Vec3| -> Option<(Vec3, Vec3)> {
            if a.z > -200.0 && b.z < -200.0 {
                let t = (a.z + 200.0) / (a.z - b.z);
                Some((a + (b - a) * t, Vec3::Z))
            } else {
                None
            }
        };
        for _ in 0..60 {
            c.update(&s, &Controls::default(), None, 1.0, &wall);
        }
        assert!(c.eye.z >= -195.1, "{}", c.eye);
    }
}
