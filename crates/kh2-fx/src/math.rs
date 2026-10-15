//! Angle and rotation helpers in the effect system's conventions.
//!
//! The game keeps matrices in row-vector form (`v' = v·M`, translation in
//! the fourth row). That memory layout is exactly glam's column-major
//! `Mat4` used with column vectors, so a game row is a glam column and the
//! game's "A applied after B" product is `A * B` here.
//!
//! Program angles are integers with 65536 = one turn, looked up in a
//! 4096-entry sine table (`angle >> 4`), so we quantise the same way.

use glam::{Mat3, Mat4, Vec3};
use std::f32::consts::TAU;

/// Sine of an integer angle (65536 = 360°) through the game's 4096-entry table.
pub fn sin_i(a: i32) -> f32 {
    let i = (a >> 4) & 0xfff;
    (i as f32 * TAU / 4096.0).sin()
}

/// Cosine of an integer angle: the sine table a quarter turn on.
pub fn cos_i(a: i32) -> f32 {
    sin_i(a.wrapping_add(0x4000))
}

fn rx(s: f32, c: f32) -> Mat3 {
    Mat3::from_cols(Vec3::X, Vec3::new(0.0, c, s), Vec3::new(0.0, -s, c))
}

fn ry(s: f32, c: f32) -> Mat3 {
    Mat3::from_cols(Vec3::new(c, 0.0, -s), Vec3::Y, Vec3::new(s, 0.0, c))
}

fn rz(s: f32, c: f32) -> Mat3 {
    Mat3::from_cols(Vec3::new(c, s, 0.0), Vec3::new(-s, c, 0.0), Vec3::Z)
}

/// Rotation about x by an integer angle.
pub fn rot_x_i(a: i32) -> Mat3 {
    rx(sin_i(a), cos_i(a))
}
pub fn rot_y_i(a: i32) -> Mat3 {
    ry(sin_i(a), cos_i(a))
}
pub fn rot_z_i(a: i32) -> Mat3 {
    rz(sin_i(a), cos_i(a))
}

pub fn rot_x(a: f32) -> Mat3 {
    rx(a.sin(), a.cos())
}
pub fn rot_y(a: f32) -> Mat3 {
    ry(a.sin(), a.cos())
}
pub fn rot_z(a: f32) -> Mat3 {
    rz(a.sin(), a.cos())
}

/// Integer angles applied X, then Y, then Z (pppMatrixXYZ, the particle
/// data header).
pub fn euler_xyz_i(a: [i32; 3]) -> Mat3 {
    rot_z_i(a[2]) * rot_y_i(a[1]) * rot_x_i(a[0])
}

/// Y, then X, then Z (pppMatrixYXZ).
pub fn euler_yxz_i(a: [i32; 3]) -> Mat3 {
    rot_z_i(a[2]) * rot_x_i(a[0]) * rot_y_i(a[1])
}

/// Z, then X, then Y (pppMatrixZXY).
pub fn euler_zxy_i(a: [i32; 3]) -> Mat3 {
    rot_y_i(a[1]) * rot_x_i(a[0]) * rot_z_i(a[2])
}

/// Radians applied X, then Y, then Z (PAX element rotations, effect root).
pub fn euler_xyz(a: Vec3) -> Mat3 {
    rot_z(a.z) * rot_y(a.y) * rot_x(a.x)
}

/// Rotation, per-axis scale and translation: `T · R · S` (the game's
/// "rows of R scaled by the scale vector, translation = position").
pub fn trs(t: Vec3, r: Mat3, s: Vec3) -> Mat4 {
    Mat4::from_cols(
        (r.x_axis * s.x).extend(0.0),
        (r.y_axis * s.y).extend(0.0),
        (r.z_axis * s.z).extend(0.0),
        t.extend(1.0),
    )
}

/// Lengths of a matrix's three axes (the game's `rs`).
pub fn axis_lengths(m: &Mat4) -> Vec3 {
    Vec3::new(m.x_axis.truncate().length(), m.y_axis.truncate().length(), m.z_axis.truncate().length())
}

/// Normalise, leaving a zero vector as it is (the game's normalise returns
/// zero for a zero input).
pub fn norm_or_zero(v: Vec3) -> Vec3 {
    let l = v.length();
    if l > 0.0 {
        v / l
    } else {
        Vec3::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn table_angles() {
        assert!((sin_i(0x4000) - 1.0).abs() < 1e-6);
        assert!((cos_i(0) - 1.0).abs() < 1e-6);
        assert!(sin_i(0x8000).abs() < 1e-5);
        // Negative angles wrap like the table index does.
        assert!((sin_i(-0x4000) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn rotation_conventions() {
        // Rx maps (y, z) -> (y c - z s, y s + z c).
        assert!(close(rot_x_i(0x4000) * Vec3::Y, Vec3::Z));
        assert!(close(rot_y_i(0x4000) * Vec3::Z, Vec3::X));
        assert!(close(rot_z_i(0x4000) * Vec3::X, Vec3::Y));
        // X is applied first: a quarter turn about x then about z.
        let m = euler_xyz_i([0x4000, 0, 0x4000]);
        assert!(close(m * Vec3::Y, Vec3::Z));
        assert!(close(m * Vec3::X, Vec3::Y));
        assert!(close(euler_xyz(Vec3::new(std::f32::consts::FRAC_PI_2, 0.0, 0.0)) * Vec3::Y, Vec3::Z));
    }

    #[test]
    fn trs_scales_axes() {
        let m = trs(Vec3::new(1.0, 2.0, 3.0), Mat3::IDENTITY, Vec3::new(2.0, 3.0, 4.0));
        assert!(close(m.transform_point3(Vec3::ONE), Vec3::new(3.0, 5.0, 7.0)));
        assert!(close(axis_lengths(&m), Vec3::new(2.0, 3.0, 4.0)));
    }
}
