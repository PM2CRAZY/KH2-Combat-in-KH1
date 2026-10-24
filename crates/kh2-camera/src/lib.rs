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

/// The lock-on goal yaw's side turn for the screen flags that started it (KH2's rule, Steam
/// `0x140176ec0`): off the game's screen to the left, `+LOCK_SIDE`; else off to the right,
/// `-LOCK_SIDE`; off only the top / bottom (or too near), none. The target's flags are used,
/// Sora's only when the target's are 0 (the caller's `off`).
///
/// KH2's flags are for its own screen (its world right-handed with +y down, like KH1's). This
/// crate's [`Camera::screen_test`] assumes a right-handed y-up world, where right is
/// `forward × up`. Fed KH1's world with only y negated (`kh1-hook`), that frame is mirrored:
/// the crate's `LEFT` is the game's right, and yaws (atan2 of x, z) are KH2's own, so KH2's
/// "left → +20°" is the crate's `RIGHT → +20°`. In a true right-handed y-up frame (a half turn
/// about x from KH2's world) yaws come out as π − KH2's and left stays left, which gives the
/// same: crate `LEFT → −20°`. So one rule holds in both: `RIGHT` is KH2's bit 4.
pub fn lock_side(off: u8) -> f32 {
    if off & screen::RIGHT != 0 {
        LOCK_SIDE
    } else if off & screen::LEFT != 0 {
        -LOCK_SIDE
    } else {
        0.0
    }
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
    /// Left stick, x right / y forward, magnitude 0..1: its direction is the pad's unit
    /// direction, its length the pad's tilt (KH2's pad stick, dead zone applied by the host).
    pub move_stick: Vec2,
    /// Right stick, x right / y up, magnitude 0..1. Right turns the view
    /// right, up zooms in.
    pub look_stick: Vec2,
    /// Camera reset (behind Sora, default distance).
    pub reset: bool,
    /// Automatic swing-behind allowed (the Camera "Auto" option).
    pub auto: bool,
    /// KH2's camera hold button is held (bit 32 of the held word; the tester: R2). With `auto`
    /// it stops the swing behind; without, the camera swings behind only while it is held
    /// (Steam KH2 `0x140178ae0`, read by the follow update `0x140165110`).
    pub hold: bool,
    /// KH2 PC's mouse: its motion this frame (x, y, each within ±1), when it moved (Steam KH2
    /// `0x1404e51f0`). The follow camera then turns at `max(|10 x|, 1) · 3°` and zooms at
    /// `max(|10 y|, 1)` times its usual speed (`0x140166100`). The host also puts the mouse on
    /// `look_stick` (KH2's pad does, `0x14039c180`).
    pub mouse: Option<Vec2>,
    /// Keyboard / mouse mode (KH2 `0x142b44a88`): the look point and eye follow at 0.4 (+ 0.0018
    /// per run speed past 8), not 0.07 (+ 0.0006) (`0x140165700`).
    pub kbm: bool,
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
    /// First-person view's pitch (radians, positive looks up).
    pub fp_pitch: f32,
}

/// First person (Steam KH2 `0x140165e30`): the look point is this far ahead of the eye.
pub const FP_AHEAD: f32 = 350.0;
/// First person: pitch speed at full tilt, and its limits up / down (radians). KH2's exe data,
/// read from the tester's Steam KH2 exe (the function `0x140165e30` reads them there, Checked
/// (exe)): the rate `0x140718bf4` = 0.0523599 (3° a frame), the limits `0x140718bf8` = −1.2217305
/// and `0x140718bfc` = 1.2217305 (70° each way). KH1's own are 3° a frame, 80° up, 60° down
/// (Steam KH1 `0x1401E2390`).
pub const FP_PITCH_RATE: f32 = 0.052_359_88;
pub const FP_PITCH_UP: f32 = 1.221_730_5;
pub const FP_PITCH_DOWN: f32 = 1.221_730_5;

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
            fp_pitch: 0.0,
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

    /// The right stick's turn and zoom (Steam KH2 follow `0x140165110`, lock-on `0x140176c70`,
    /// zoom `0x1401654d0` / `0x140176bc0`). KH2 tests the stick's direction bits (an axis past
    /// its dead zone, `0x140178ab0` / `0x140178b10` turn, `0x140178b50` / `0x140178b80` zoom) and
    /// moves by the stick's whole tilt, not by that axis: diagonally the camera turns and zooms
    /// at full speed together. Returns whether the stick is tilted at all.
    /// (`speed`: the turn speed at tilt 1 and the zoom factor, `speeds`.) The tilt is the stick's
    /// length, not held to 1: KH2's pad scales it by the mouse sensitivity in a mouse frame.
    fn manual(&mut self, c: &Controls, speed: (f32, f32), dt: f32) -> bool {
        self.turn(c, speed.0, dt);
        self.zoom(c, speed.1, dt);
        c.look_stick != Vec2::ZERO
    }

    /// The follow camera's turn speed and zoom factor this frame (Steam KH2 `0x140166100`): 3° a
    /// frame (the exe's `0x140718bf0`, Checked (exe) through the OG rewrite) and 1; in a frame
    /// the mouse moved, `max(|10 x|, 1) · 3°` and `max(|10 y|, 1)`. The lock-on camera has
    /// neither (`0x140176c70`, `0x140176bc0`): `(STICK_YAW, 1)`.
    pub fn speeds(c: &Controls) -> (f32, f32) {
        match c.mouse {
            Some(m) => ((m.x * 10.0).abs().max(1.0) * 3.0f32.to_radians(), (m.y * 10.0).abs().max(1.0)),
            None => (STICK_YAW, 1.0),
        }
    }

    fn turn(&mut self, c: &Controls, speed: f32, dt: f32) {
        let r = c.look_stick;
        if r.x != 0.0 {
            // Right turns the view right: the eye swings the other way.
            self.yaw = wrap(self.yaw - r.x.signum() * r.length() * speed * dt);
            self.goal_yaw = self.yaw;
        }
    }

    fn zoom(&mut self, c: &Controls, factor: f32, dt: f32) {
        let p = self.params;
        let r = c.look_stick;
        if r.y != 0.0 {
            self.dist = (self.dist - r.y.signum() * r.length() * STICK_ZOOM * factor * dt).clamp(p.dist_min, p.dist_max);
        }
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
        // A hard cut this frame: the rendered points jump to their goals.
        let mut cut = false;
        match lock {
            None => {
                self.locked = false;
                if c.reset {
                    self.goal_yaw = behind;
                    self.dist = p.dist_default;
                    // KH2's reset also puts the eye on its last goal (`0x140165110`: the eye
                    // `+0x18` from the eye goal `+0x64`), so only the turn eases.
                    self.eye = self.eye_goal;
                    if c.move_stick != Vec2::ZERO {
                        // A hard cut when moving: KH2 (`0x140165110`, the reset with the left
                        // stick tilted) places the camera with its cut flag (`0x140164520`), which
                        // sets the yaw to its goal and the look point and eye to theirs
                        // (`0x140164d70`), with no spring.
                        self.yaw = behind;
                        cut = true;
                    }
                    // (KH2 zooms after its reset branch too, `0x140165110`.)
                    self.zoom(c, Self::speeds(c).1, dt);
                } else {
                    let turned = self.manual(c, Self::speeds(c), dt);
                    // KH2's pad keeps the left stick as a unit direction and a tilt (Steam
                    // 0x14039c180 normalizes the dead-zoned axes in place, 0x140142080, and
                    // keeps the length, at most 1, beside them). The follow update
                    // (0x140165110) tests and scales by the unit direction: no swing while it
                    // points within acos 0.9 (25.8°) of straight back, at any tilt. Its pad y
                    // is positive pulled back; here y is forward, so -y < 0.9.
                    let lmag = c.move_stick.length();
                    let d = c.move_stick.normalize_or_zero();
                    // The swing flag (`+0x42`): cleared by the hold button with Auto, set only
                    // by it with Manual (`0x140178ae0`: bit 32, inverted unless Auto).
                    let swing = c.auto != c.hold;
                    if swing && moved && lmag > 0.0 && !turned && -d.y < 0.9 {
                        let step = 2.0 * (d.x.abs() + 1.0) * (d.y.abs() + 1.0) * lmag * (RECENTER + (s.run_speed - 8.0) * RECENTER_RUN) * dt;
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
                        // him, turned 20° by the screen edge it left by.
                        let base = (s.feet.x - self.at_goal.x).atan2(s.feet.z - self.at_goal.z);
                        self.goal_yaw = wrap(base + lock_side(off));
                    }
                    at_goal = a;
                } else {
                    at_goal = chest;
                }
                // KH2's lock-on (`0x140176c70`) turns and zooms first, then a reset sets the goal
                // and the distance.
                self.manual(c, (STICK_YAW, 1.0), dt);
                if c.reset {
                    self.goal_yaw = (s.feet.x - self.at_goal.x).atan2(s.feet.z - self.at_goal.z);
                    self.dist = p.dist_alt;
                }
            }
        }
        self.yaw = approach_angle(self.yaw, self.goal_yaw, YAW_LIMIT * dt);
        // The eye is placed from Sora's chest in both modes.
        let mut eye_goal = self.place(chest);
        if let Some((hit, n)) = wall(chest, eye_goal) {
            eye_goal = hit + n * WALL_GAP;
        }
        let (at, mut eye) = self.settle(at_goal, eye_goal, s.run_speed, c.kbm, cut, dt);
        if let Some((hit, n)) = wall(chest, eye) {
            eye = hit + n * WALL_GAP;
        }
        self.at = at;
        self.eye = eye;
    }

    /// The rendered look point and eye after this frame's spring toward their goals (KH2's
    /// `0x140165700`, run in every mode), or the goals themselves on a cut. Sets the goals.
    /// `kbm`: keyboard / mouse mode, KH2's tighter follow.
    fn settle(&mut self, at_goal: Vec3, eye_goal: Vec3, run_speed: f32, kbm: bool, cut: bool, dt: f32) -> (Vec3, Vec3) {
        self.prev_at_goal = self.at_goal;
        self.prev_eye_goal = self.eye_goal;
        self.at_goal = at_goal;
        self.eye_goal = eye_goal;
        // KH2 springs once a frame (`0x140165700`): 0.07 + (run - 8) · 0.0006, or in keyboard /
        // mouse mode 0.4 + (run - 8) · 0.0018, halved when the frame step is under 2 (60 fps) and
        // whole from 2 up (30 fps, or slower). The look point's rate is 0.03 more.
        let (r0, r1) = if kbm { (0.4, 0.0018) } else { (0.07, 0.0006) };
        let rate = (r0 + (run_speed - 8.0) * r1) * if dt < 2.0 { 0.5 } else { 1.0 };
        if cut {
            self.prev_at_goal = self.at_goal;
            self.prev_eye_goal = self.eye_goal;
            return (self.at_goal, self.eye_goal);
        }
        if dt <= 0.0 {
            // (No frame passed: held.)
            return (self.at, self.eye);
        }
        let at = spring(self.at, self.at_goal, self.prev_at_goal, rate + 0.03, 1.0);
        let eye = spring(self.eye, self.eye_goal, self.prev_eye_goal, rate, 1.0);
        (at, eye)
    }

    /// Into KH2's first-person view (Steam KH2 follow `0x140165110`, Select where the player may:
    /// the mode set to 1, the pitch to 0). The host fades the player out (KH2: 30 frames).
    pub fn enter_first_person(&mut self) {
        self.fp_pitch = 0.0;
        self.locked = false;
    }

    /// One frame of KH2's first-person view (Steam KH2 `0x140165e30`). The eye's goal is the
    /// player's position raised by `eye_height` (his `prty` `subjective_offset`, Sora 150; KH2
    /// `0x1403b5c50`), the look point's 350 ahead of it along his facing, turned up by the pitch.
    /// The right stick's up / down turns the pitch by its tilt (`c.look_stick.y`, y up), within
    /// limits; a reset (`c.reset`, R3) puts it back level. The camera's own yaw is kept behind him
    /// (`0x140164e90`), so leaving puts the follow camera there. The rendered points spring to
    /// the goals as in every mode, so the view eases into and out of his head.
    pub fn update_first_person(&mut self, s: &Subject, eye_height: f32, c: &Controls, dt: f32) {
        if c.reset {
            self.fp_pitch = 0.0;
        }
        if c.look_stick.y != 0.0 {
            // (KH2 tests the stick's up / down bits and turns by the tilt times the axis of its
            // unit direction: the vector's y.)
            self.fp_pitch = wrap(self.fp_pitch + c.look_stick.y * FP_PITCH_RATE * dt);
        }
        self.fp_pitch = self.fp_pitch.clamp(-FP_PITCH_DOWN, FP_PITCH_UP);
        self.locked = false;
        let behind = wrap(s.facing + PI);
        self.yaw = behind;
        self.goal_yaw = behind;
        self.last_feet = Some(s.feet);
        let eye_goal = s.feet + Vec3::Y * eye_height;
        let (sp, cp) = self.fp_pitch.sin_cos();
        let ahead = Vec3::new(s.facing.sin() * cp, sp, s.facing.cos() * cp);
        let (at, eye) = self.settle(eye_goal + ahead * FP_AHEAD, eye_goal, s.run_speed, c.kbm, false, dt);
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

    /// The default position a host resets to (KH1's camera resets, `kh1-hook`): behind Sora
    /// whichever way he faces, at the default distance, the look point above his feet.
    #[test]
    fn a_new_camera_is_behind_sora_at_the_default_distance() {
        let feet = Vec3::new(300.0, 40.0, -900.0);
        let facing = 2.2_f32;
        let c = Camera::new(Params::FIELD, &sora(feet, facing));
        assert_eq!(c.dist, Params::FIELD.dist_default);
        assert!((c.at - (feet + Vec3::Y * AT_UP)).length() < 1e-3);
        // Sora looks along (sin, cos) of his facing; the eye is the other way.
        let back = Vec2::new(c.eye.x - feet.x, c.eye.z - feet.z);
        let ahead = Vec2::new(facing.sin(), facing.cos());
        assert!((back.length() - 420.0).abs() < 0.01);
        assert!((back.normalize().dot(ahead) + 1.0).abs() < 1e-4, "{back}");
        assert!((c.eye.y - feet.y - 251.2).abs() < 0.01);
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

    /// KH2 PC's mouse (`0x140166100`): the follow camera turns at `max(|10 x|, 1) · 3°` and zooms
    /// at `max(|10 y|, 1)` times; the lock-on camera keeps its own speeds.
    #[test]
    fn the_mouse_speeds_up_the_follow_camera() {
        let mut c = Camera::new(Params::FIELD, &sora(Vec3::ZERO, 0.0));
        let y0 = c.yaw;
        let ctl = Controls { look_stick: Vec2::new(1.0, 0.0), mouse: Some(Vec2::new(0.3, 0.0)), ..Default::default() };
        c.update(&sora(Vec3::ZERO, 0.0), &ctl, None, 1.0, &none);
        assert!((wrap(c.yaw - y0).abs().to_degrees() - 9.0).abs() < 1e-3);
        // A slow mouse: at least 3° a frame times the tilt.
        let mut c = Camera::new(Params::FIELD, &sora(Vec3::ZERO, 0.0));
        let ctl = Controls { look_stick: Vec2::new(0.5, 0.0), mouse: Some(Vec2::new(0.02, 0.0)), ..Default::default() };
        c.update(&sora(Vec3::ZERO, 0.0), &ctl, None, 1.0, &none);
        assert!((wrap(c.yaw - y0).abs().to_degrees() - 1.5).abs() < 1e-3);
        // Zoom: 8 · 4 a frame for y 0.4.
        let mut c = Camera::new(Params::FIELD, &sora(Vec3::ZERO, 0.0));
        let ctl = Controls { look_stick: Vec2::new(0.0, 1.0), mouse: Some(Vec2::new(0.0, 0.4)), ..Default::default() };
        c.update(&sora(Vec3::ZERO, 0.0), &ctl, None, 1.0, &none);
        assert!((c.dist - (420.0 - 32.0)).abs() < 1e-3);
        // Locked on: 3° whatever the mouse.
        let mut c = Camera::new(Params::FIELD, &sora(Vec3::ZERO, 0.0));
        let ctl = Controls { look_stick: Vec2::new(1.0, 0.0), mouse: Some(Vec2::new(0.3, 0.0)), ..Default::default() };
        let g = c.goal_yaw;
        c.update(&sora(Vec3::ZERO, 0.0), &ctl, Some(Vec3::new(0.0, 170.0, 300.0)), 1.0, &none);
        assert!((wrap(c.goal_yaw - g).abs().to_degrees() - 3.0).abs() < 0.2, "{}", wrap(c.goal_yaw - g).to_degrees());
    }

    /// Keyboard / mouse mode: the eye closes about 2.5 · 0.2 = 50% of its gap a frame, not 9%.
    #[test]
    fn keyboard_mouse_mode_follows_tighter() {
        let gap = |kbm: bool| {
            let mut c = Camera::new(Params::FIELD, &sora(Vec3::ZERO, 0.0));
            let ctl = Controls { kbm, ..Default::default() };
            let before = c.eye;
            c.update(&sora(Vec3::new(100.0, 0.0, 0.0), 0.0), &ctl, None, 1.0, &none);
            let goal = before + Vec3::new(100.0, 0.0, 0.0);
            (c.eye - goal).length() / 100.0
        };
        assert!(gap(true) < 0.55 && gap(true) > 0.4, "{}", gap(true));
        assert!(gap(false) > 0.85, "{}", gap(false));
    }

    /// At 30 fps (step 2) KH2 springs once at the whole rate (`0x140165700`): the eye closes
    /// 2.5 · 0.4 = all of its gap in keyboard / mouse mode, and 2.5 · 0.07 = 17.5% with a pad
    /// (less the spring's small term for the goal's own motion).
    #[test]
    fn thirty_fps_springs_once_at_the_whole_rate() {
        let gap = |kbm: bool| {
            let mut c = Camera::new(Params::FIELD, &sora(Vec3::ZERO, 0.0));
            let ctl = Controls { kbm, ..Default::default() };
            let before = c.eye;
            c.update(&sora(Vec3::new(100.0, 0.0, 0.0), 0.0), &ctl, None, 2.0, &none);
            let goal = before + Vec3::new(100.0, 0.0, 0.0);
            (c.eye - goal).length() / 100.0
        };
        assert!((gap(true) - 0.09).abs() < 0.02, "{}", gap(true));
        assert!((gap(false) - 0.825).abs() < 0.02, "{}", gap(false));
    }

    /// The hold button (R2): with Auto it stops the swing behind, with Manual only it swings
    /// (Steam KH2 `0x140178ae0`).
    #[test]
    fn the_hold_button_flips_the_swing() {
        let run = |auto: bool, hold: bool| {
            let mut c = Camera::new(Params::FIELD, &sora(Vec3::ZERO, 0.0));
            let ctl = Controls { move_stick: Vec2::new(1.0, 0.0), auto, hold, ..Default::default() };
            let y0 = c.yaw;
            let mut feet = Vec3::ZERO;
            for _ in 0..10 {
                feet.x -= 8.0;
                c.update(&sora(feet, -std::f32::consts::FRAC_PI_2), &ctl, None, 1.0, &none);
            }
            wrap(c.yaw - y0).abs().to_degrees()
        };
        assert!((run(true, false) - 9.0).abs() < 0.05);
        assert!(run(true, true) < 1e-4);
        assert!(run(false, false) < 1e-4);
        assert!((run(false, true) - 9.0).abs() < 0.05);
    }

    /// Runs Sora toward the camera for 10 frames with `stick` held (y forward) and returns how
    /// far the camera turned, in degrees. His facing follows the stick through the camera, as
    /// the host's movement does.
    fn turn_running_back(stick: Vec2) -> f32 {
        let mut c = Camera::new(Params::FIELD, &sora(Vec3::ZERO, 0.0));
        let ctl = Controls { move_stick: stick, auto: true, ..Default::default() };
        let y0 = c.yaw;
        let mut feet = Vec3::ZERO;
        for _ in 0..10 {
            // Camera-relative: forward is away from the eye, so the stick's angle is added to
            // the view yaw.
            let facing = wrap(c.view_yaw() + stick.x.atan2(stick.y));
            feet += Vec3::new(facing.sin(), 0.0, facing.cos()) * 8.0;
            c.update(&sora(feet, facing), &ctl, None, 1.0, &none);
        }
        wrap(c.yaw - y0).abs().to_degrees()
    }

    /// The tester's trace f571370: the stick pulled straight back must not swing the camera
    /// (KH2 Steam 0x140165110: no swing while the pad's unit direction has y 0.9 or more pulled
    /// back). The direction, not the tilt, decides: a part tilt straight back doesn't swing either.
    #[test]
    fn running_straight_back_does_not_swing() {
        assert!(turn_running_back(Vec2::new(0.0, -1.0)) < 1e-4);
        assert!(turn_running_back(Vec2::new(0.0, -0.5)) < 1e-4);
        assert!(turn_running_back(Vec2::new(0.3, -0.95)) < 1e-4);
        // 20° off straight back: cos 20° = 0.94, still "straight back".
        let a = 20f32.to_radians();
        assert!(turn_running_back(Vec2::new(a.sin(), -a.cos()) * 0.6) < 1e-4);
    }

    /// More than 25.8° off straight back (diagonally back) still swings, at the rate of the
    /// unit direction's components times the tilt.
    #[test]
    fn diagonal_back_still_swings() {
        let r = 0.5_f32.sqrt();
        // 2 (|x| + 1)(|y| + 1) · tilt · 0.25° per frame, over 9 frames (the first has no "moved").
        let diag = 2.0 * (r + 1.0) * (r + 1.0) * 0.25 * 9.0;
        assert!((turn_running_back(Vec2::new(r, -r)) - diag).abs() < 0.05);
        assert!((turn_running_back(Vec2::new(r, -r) * 0.5) - diag * 0.5).abs() < 0.05);
        let a = 30f32.to_radians();
        let thirty = 2.0 * (a.sin() + 1.0) * (a.cos() + 1.0) * 0.25 * 9.0;
        assert!((turn_running_back(Vec2::new(a.sin(), -a.cos())) - thirty).abs() < 0.05);
    }

    /// KH2 turns and zooms by the right stick's whole tilt once its axis is past the dead zone:
    /// a full diagonal turns 3° and zooms 8 a frame, not 3° · 0.71.
    #[test]
    fn the_right_stick_turns_and_zooms_by_its_whole_tilt() {
        let s = sora(Vec3::ZERO, 0.0);
        let r = 0.5_f32.sqrt();
        let mut c = Camera::new(Params::FIELD, &s);
        let (y0, d0) = (c.yaw, c.dist);
        let ctl = Controls { look_stick: Vec2::new(r, r), ..Default::default() };
        c.update(&s, &ctl, None, 1.0, &none);
        assert!((wrap(y0 - c.yaw) - STICK_YAW).abs() < 1e-5, "{}", wrap(y0 - c.yaw));
        assert!((d0 - c.dist - STICK_ZOOM).abs() < 1e-4);
        // Half tilt straight right: half speed, no zoom.
        let mut c = Camera::new(Params::FIELD, &s);
        let ctl = Controls { look_stick: Vec2::new(0.5, 0.0), ..Default::default() };
        c.update(&s, &ctl, None, 1.0, &none);
        assert!((wrap(y0 - c.yaw) - STICK_YAW * 0.5).abs() < 1e-5);
        assert_eq!(c.dist, d0);
    }

    /// The reset with the left stick held is a cut: the camera is behind Sora at once, with no
    /// spring (KH2 `0x140164520` / `0x140164d70`).
    #[test]
    fn a_reset_while_moving_cuts() {
        let mut c = Camera::new(Params::FIELD, &sora(Vec3::ZERO, 0.0));
        let s = sora(Vec3::new(8.0, 0.0, 0.0), 1.0);
        let ctl = Controls { reset: true, move_stick: Vec2::new(0.0, 1.0), ..Default::default() };
        c.update(&s, &ctl, None, 1.0, &none);
        let want = Camera::new(Params::FIELD, &s);
        assert!((c.eye - want.eye).length() < 1e-3, "{} {}", c.eye, want.eye);
        assert!((c.at - want.at).length() < 1e-3);
        // Without the stick, the yaw turns at 10° a frame and the spring eases.
        let mut c = Camera::new(Params::FIELD, &sora(Vec3::ZERO, 0.0));
        let ctl = Controls { reset: true, ..Default::default() };
        c.update(&s, &ctl, None, 1.0, &none);
        assert!((c.eye - want.eye).length() > 10.0);
    }

    /// KH2's first person (`0x140165e30`): the eye eases to the player's position plus his
    /// first-person height, looking 350 ahead along his facing; the right stick's up / down
    /// pitches within the limits, a reset levels it, and the camera's yaw stays behind him.
    #[test]
    fn first_person_looks_ahead_from_the_head() {
        let feet = Vec3::new(100.0, 0.0, 50.0);
        let facing = 0.7_f32;
        let s = sora(feet, facing);
        let mut c = Camera::new(Params::FIELD, &s);
        c.enter_first_person();
        let still = Controls::default();
        c.update_first_person(&s, 150.0, &still, 1.0);
        // The first frame only eases toward the head.
        assert!((c.eye - (feet + Vec3::Y * 150.0)).length() > 50.0);
        for _ in 0..300 {
            c.update_first_person(&s, 150.0, &still, 1.0);
        }
        assert!((c.eye - (feet + Vec3::Y * 150.0)).length() < 2.0, "{}", c.eye);
        let ahead = (c.at - c.eye).normalize();
        assert!((ahead - Vec3::new(facing.sin(), 0.0, facing.cos())).length() < 1e-2, "{ahead}");
        assert!((wrap(c.yaw - (facing + PI))).abs() < 1e-5);
        // Stick up: pitch up by 3° a frame at full tilt, to the limit.
        let up = Controls { look_stick: Vec2::new(0.0, 1.0), ..Default::default() };
        c.update_first_person(&s, 150.0, &up, 1.0);
        assert!((c.fp_pitch - FP_PITCH_RATE).abs() < 1e-6);
        for _ in 0..100 {
            c.update_first_person(&s, 150.0, &up, 1.0);
        }
        assert_eq!(c.fp_pitch, FP_PITCH_UP);
        let down = Controls { look_stick: Vec2::new(0.0, -0.5), ..Default::default() };
        for _ in 0..200 {
            c.update_first_person(&s, 150.0, &down, 1.0);
        }
        assert_eq!(c.fp_pitch, -FP_PITCH_DOWN);
        let reset = Controls { reset: true, ..Default::default() };
        c.update_first_person(&s, 150.0, &reset, 1.0);
        assert_eq!(c.fp_pitch, 0.0);
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
    fn lock_on_side_turn_follows_the_edge_left_by() {
        // KH2's rule (`0x140176ec0`): its left (the crate's RIGHT) +20°, else its right −20°,
        // top / bottom alone none; KH2's left wins.
        assert_eq!(lock_side(screen::RIGHT), LOCK_SIDE);
        assert_eq!(lock_side(screen::LEFT), -LOCK_SIDE);
        assert_eq!(lock_side(screen::LEFT | screen::RIGHT), LOCK_SIDE);
        assert_eq!(lock_side(screen::TOP), 0.0);
        assert_eq!(lock_side(screen::BOTTOM | screen::RIGHT), LOCK_SIDE);
        assert_eq!(lock_side(0), 0.0);
        let s = sora(Vec3::ZERO, 0.0);
        for (x, flag, side) in [(2000.0, screen::LEFT, -LOCK_SIDE), (-2000.0, screen::RIGHT, LOCK_SIDE)] {
            let mut c = Camera::new(Params::FIELD, &s);
            // Looking along +z: the crate's right is −x.
            let t = Vec3::new(x, 100.0, 400.0);
            assert_eq!(c.screen_test(t) & (screen::LEFT | screen::RIGHT), flag);
            let at = c.at_goal;
            let base = (s.feet.x - at.x).atan2(s.feet.z - at.z);
            c.update(&s, &Controls::default(), Some(t), 1.0, &none);
            assert!(wrap(c.goal_yaw - (base + side)).abs() < 1e-4, "{} vs {}", c.goal_yaw, base + side);
        }
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
