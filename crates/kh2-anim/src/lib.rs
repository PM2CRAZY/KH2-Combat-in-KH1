//! # kh2-anim
//!
//! Poses a KH2 skeleton from an interpolated motion read from the player's
//! own install, following the order of operations of KH2 Final Mix's motion
//! evaluator (described in plain language in `docs/animation.md`):
//!
//! 1. bind pose, then the motion's initial pose, then F-curves (bones and
//!    IK helpers) sampled at the 60 Hz key time
//! 2. joints in the motion's joint-table order: expressions, constraints
//!    (position, orientation, aim, scale, limits), one- and two-bone IK
//!
//! Only skeletons are involved: no meshes or textures. Units are model units
//! (y up, the character faces +z).

mod solve;

pub use solve::{euler_from_matrix, ik_solve};

use glam::{EulerRot, Mat4, Quat, Vec3};
use kh2_formats::anim::{Bone, Interpolated};

/// Scale, rotation (Euler, radians, applied X then Y then Z) and
/// translation of one joint relative to its parent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Srt {
    pub scale: Vec3,
    pub rotation: Vec3,
    pub translation: Vec3,
}

impl Srt {
    pub fn from_bone(b: &Bone) -> Srt {
        Srt { scale: b.scale.into(), rotation: b.rotation.into(), translation: b.translation.into() }
    }

    pub fn quat(&self) -> Quat {
        euler_quat(self.rotation)
    }

    /// Rotation and translation only (scale is carried separately, as the
    /// game does).
    pub fn rt_matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.quat(), self.translation)
    }

    pub fn set(&mut self, channel: u8, v: f32) {
        match channel {
            0 => self.scale.x = v,
            1 => self.scale.y = v,
            2 => self.scale.z = v,
            3 => self.rotation.x = v,
            4 => self.rotation.y = v,
            5 => self.rotation.z = v,
            6 => self.translation.x = v,
            7 => self.translation.y = v,
            8 => self.translation.z = v,
            _ => {}
        }
    }
}

/// Euler XYZ (X applied first) as a quaternion.
pub fn euler_quat(r: Vec3) -> Quat {
    Quat::from_euler(EulerRot::ZYX, r.z, r.y, r.x)
}

/// A posed skeleton: one model-space matrix per model bone (scale applied),
/// plus the motion's IK helpers.
#[derive(Clone, Debug, Default)]
pub struct Pose {
    pub bones: Vec<Mat4>,
    pub helpers: Vec<Mat4>,
}

impl Pose {
    pub fn bone_position(&self, i: usize) -> Vec3 {
        self.bones.get(i).map_or(Vec3::ZERO, |m| m.w_axis.truncate())
    }

    /// The bind pose of a skeleton (no motion).
    pub fn bind(skeleton: &[Bone]) -> Pose {
        let mut unscaled = vec![Mat4::IDENTITY; skeleton.len()];
        let mut scale = vec![Vec3::ONE; skeleton.len()];
        let mut bones = vec![Mat4::IDENTITY; skeleton.len()];
        for (i, b) in skeleton.iter().enumerate() {
            let s = Srt::from_bone(b);
            let p = parent_index(b.parent, i, skeleton.len());
            scale[i] = p.map_or(s.scale, |p| scale[p] * s.scale);
            unscaled[i] = p.map_or(s.rt_matrix(), |p| unscaled[p] * s.rt_matrix());
            let pos = p.map_or(s.translation, |p| bones[p].transform_point3(s.translation));
            bones[i] = scaled(unscaled[i], scale[i], pos);
        }
        Pose { bones, helpers: Vec::new() }
    }
}

pub(crate) fn parent_index(parent: i16, child: usize, len: usize) -> Option<usize> {
    (parent >= 0 && (parent as usize) < len && parent as usize != child).then_some(parent as usize)
}

/// The game's "scaled world" matrix: the unscaled rotation with each axis
/// multiplied by the accumulated scale, and an explicit position.
pub(crate) fn scaled(unscaled: Mat4, s: Vec3, pos: Vec3) -> Mat4 {
    Mat4::from_cols(unscaled.x_axis * s.x, unscaled.y_axis * s.y, unscaled.z_axis * s.z, pos.extend(1.0))
}

/// Evaluation settings.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Apply expressions, constraints and IK (otherwise plain FK).
    pub constraints: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { constraints: true }
    }
}

/// Poses `skeleton` with `motion` at key time `t` (60 Hz frames; see
/// [`key_time`]).
pub fn evaluate(skeleton: &[Bone], motion: &Interpolated, t: f32, opts: Options) -> Pose {
    let mut s = solve::State::new(skeleton, motion, t);
    s.run(opts.constraints);
    let bc = s.bc;
    let mut bones = vec![Mat4::IDENTITY; skeleton.len()];
    bones[..bc].copy_from_slice(&s.world[..bc]);
    // Model bones the motion doesn't address keep their bind offsets.
    let mut unscaled = s.unscaled[..bc].to_vec();
    unscaled.resize(skeleton.len(), Mat4::IDENTITY);
    let mut scale = s.wscale[..bc].to_vec();
    scale.resize(skeleton.len(), Vec3::ONE);
    for i in bc..skeleton.len() {
        let b = &skeleton[i];
        let srt = Srt::from_bone(b);
        let p = parent_index(b.parent, i, skeleton.len());
        scale[i] = p.map_or(srt.scale, |p| scale[p] * srt.scale);
        unscaled[i] = p.map_or(srt.rt_matrix(), |p| unscaled[p] * srt.rt_matrix());
        let pos = p.map_or(srt.translation, |p| bones[p].transform_point3(srt.translation));
        bones[i] = scaled(unscaled[i], scale[i], pos);
    }
    Pose { bones, helpers: s.world[bc..].to_vec() }
}

/// Converts a clip-relative time into the motion's key time.
///
/// `game_frames` counts 60 Hz frames since the clip started. The result is
/// on the key timeline (also 60 Hz), offset by `frame_start`. With
/// `looping`, playback wraps from `frame_end` back to `frame_return`.
pub fn key_time(motion: &Interpolated, game_frames: f32, looping: bool) -> f32 {
    let k = 60.0 / motion.fps.max(1e-3);
    let start = motion.frame_start * k;
    let end = motion.frame_end * k;
    let t = start + game_frames;
    if t <= end {
        return t;
    }
    if !looping {
        return end;
    }
    let ret = (motion.frame_return * k).clamp(start, end);
    let span = end - ret;
    if span <= 0.0 {
        end
    } else {
        ret + (t - end) % span
    }
}

/// Clip length in 60 Hz frames.
pub fn length_frames(motion: &Interpolated) -> f32 {
    (motion.frame_end - motion.frame_start).max(0.0) * 60.0 / motion.fps.max(1e-3)
}
