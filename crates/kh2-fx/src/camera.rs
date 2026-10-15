//! The camera the frame is built for.
//!
//! Effects are simulated without a camera; only drawing needs one, because
//! most sprites are billboards (they face the camera) and sorted draws go
//! by view depth. The view space is the game's (`docs/kh2-effects.md` §6):
//! **x right, y down, z forward** (into the screen), in KH2 world units.

use glam::{Mat4, Vec3};

/// World → view transform in the game's view convention.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    view: Mat4,
    inv: Mat4,
}

impl Default for Camera {
    fn default() -> Self {
        Camera::from_view(Mat4::IDENTITY)
    }
}

impl Camera {
    /// From a world → view matrix whose view space is x right, y down,
    /// z forward (KH2's own).
    pub fn from_view(view: Mat4) -> Camera {
        Camera { view, inv: view.inverse() }
    }

    /// From a Direct3D-style left-handed view matrix (x right, **y up**,
    /// z forward). The world side must still be KH2's (y down); if the
    /// host's world differs, fold that conversion into `view` first.
    pub fn from_view_y_up(view: Mat4) -> Camera {
        Camera::from_view(Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0)) * view)
    }

    /// The game's look-at: z toward the target, x = forward × up,
    /// y = forward × x. With KH2's up `(0, -1, 0)` this gives y down on
    /// screen.
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Camera {
        let z = (target - eye).normalize_or_zero();
        let x = z.cross(up).normalize_or_zero();
        let y = z.cross(x);
        let rot = Mat4::from_cols(x.extend(0.0), y.extend(0.0), z.extend(0.0), Vec3::ZERO.extend(1.0)).transpose();
        Camera::from_view(rot * Mat4::from_translation(-eye))
    }

    /// World → view.
    pub fn view(&self) -> Mat4 {
        self.view
    }

    /// View → world.
    pub fn view_inverse(&self) -> Mat4 {
        self.inv
    }

    /// Camera position in world space.
    pub fn eye(&self) -> Vec3 {
        self.inv.w_axis.truncate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_at_axes() {
        // Looking along +z with KH2's up (0, -1, 0): x right, y down.
        let c = Camera::look_at(Vec3::new(0.0, -100.0, -300.0), Vec3::new(0.0, -100.0, 0.0), Vec3::NEG_Y);
        let p = c.view().transform_point3(Vec3::new(10.0, -90.0, 0.0));
        assert!((p - Vec3::new(10.0, 10.0, 300.0)).length() < 1e-3, "{p}");
        assert!((c.eye() - Vec3::new(0.0, -100.0, -300.0)).length() < 1e-3);
    }

    #[test]
    fn y_up_view_flips() {
        let c = Camera::from_view_y_up(Mat4::IDENTITY);
        assert_eq!(c.view().transform_point3(Vec3::Y), Vec3::NEG_Y);
    }
}
