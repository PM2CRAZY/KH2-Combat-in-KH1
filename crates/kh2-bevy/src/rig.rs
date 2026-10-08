//! Sora as an asset-free mannequin: his real skeleton (bones only) posed by
//! his real motions every frame, both read from the user's install. The
//! visible shapes are our own capsules between joints; no mesh or texture
//! is loaded.

use std::sync::Arc;

use bevy::prelude::*;
use character_core::{motion, Character, Output, StateName};
use kh2_anim::{evaluate, key_time, Options};
use kh2_data::sora::{BoneSphere, Rig};
use kh2_formats::anim::Interpolated;

/// Frames over which a new motion is blended in from the previous pose
/// (approximation; the game uses per-move blend times).
const BLEND_FRAMES: f32 = 8.0;

#[derive(Clone, Copy, PartialEq)]
struct Track {
    motion: u16,
    frame: f32,
    looping: bool,
    /// Action clips: subtract the clip's root motion (the controller has
    /// already moved the character by it).
    root: bool,
}

pub struct RigAnim {
    pub rig: Arc<Rig>,
    track: Option<Track>,
    /// Model-space bone matrices (Sora), and the Keyblade's bones in Sora's
    /// model space.
    pub bones: Vec<Mat4>,
    pub weapon: Vec<Mat4>,
    /// Model space → world (KH2 units).
    pub to_world: Mat4,
    /// Object root (feet, facing; no root-motion offset) → world.
    pub object: Mat4,
    blend_from: Option<(Vec<(Vec3, Quat, Vec3)>, Vec<(Vec3, Quat, Vec3)>)>,
    blend_t: f32,
}

fn decompose(v: &[Mat4]) -> Vec<(Vec3, Quat, Vec3)> {
    v.iter()
        .map(|m| {
            let (s, r, t) = m.to_scale_rotation_translation();
            (s, r, t)
        })
        .collect()
}

fn blend(from: &[(Vec3, Quat, Vec3)], to: &mut [Mat4], w: f32) {
    for (m, f) in to.iter_mut().zip(from) {
        let (s, r, t) = m.to_scale_rotation_translation();
        *m = Mat4::from_scale_rotation_translation(f.0.lerp(s, w), f.1.slerp(r, w), f.2.lerp(t, w));
    }
}

impl RigAnim {
    pub fn new(rig: Arc<Rig>) -> RigAnim {
        let bones = kh2_anim::Pose::bind(&rig.skeleton).bones;
        let weapon = kh2_anim::Pose::bind(&rig.weapon_skeleton).bones;
        RigAnim { rig, track: None, bones, weapon, to_world: Mat4::IDENTITY, object: Mat4::IDENTITY, blend_from: None, blend_t: 1.0 }
    }

    fn pick(&self, out: &Output) -> Track {
        let prev = self.track;
        let free = |m: u16, looping: bool| {
            let frame = match prev {
                Some(p) if p.motion == m => p.frame + 1.0,
                _ => 0.0,
            };
            Track { motion: m, frame, looping, root: false }
        };
        match out.state {
            StateName::Idle | StateName::Walk | StateName::Run => free(out.motion, true),
            StateName::Glide => free(out.motion, true),
            StateName::Air => {
                // Keep playing the jump while rising, then the fall loop.
                let rising = out.displacement[1] > 0.05;
                match prev {
                    Some(p) if rising && (p.motion == motion::JUMP || p.motion == motion::HIGH_JUMP || p.motion == motion::AERIAL_DODGE) => {
                        Track { frame: p.frame + 1.0, looping: false, root: false, ..p }
                    }
                    _ => free(motion::FALL, true),
                }
            }
            _ => Track { motion: out.motion, frame: out.clip_frame as f32 + 1.0, looping: false, root: true },
        }
    }

    /// Advances one 60 Hz frame from the controller's output.
    pub fn update(&mut self, out: &Output, character: &Character, pos: [f32; 3]) {
        let mut track = self.pick(out);
        let rig = self.rig.clone();
        let Some(m) = rig.motions.get(&track.motion).or_else(|| self.track.and_then(|t| rig.motions.get(&t.motion))) else { return };
        if !rig.motions.contains_key(&track.motion) {
            track = self.track.unwrap_or(track);
        }
        let restarted = match self.track {
            Some(p) => p.motion != track.motion || track.frame < p.frame,
            None => false,
        };
        if restarted {
            self.blend_from = Some((decompose(&self.bones), decompose(&self.weapon)));
            self.blend_t = 0.0;
        }
        self.track = Some(track);

        let t = key_time(m, track.frame, track.looping);
        let pose = evaluate(&rig.skeleton, m, t, Options::default());
        let mut bones = pose.bones;
        let mut weapon = self.weapon_pose(&rig, track, &bones);
        if let Some((fb, fw)) = &self.blend_from {
            self.blend_t += 1.0;
            let w = (self.blend_t / BLEND_FRAMES).min(1.0);
            blend(fb, &mut bones, w);
            blend(fw, &mut weapon, w);
            if w >= 1.0 {
                self.blend_from = None;
            }
        }
        self.bones = bones;
        self.weapon = weapon;

        let root = if track.root {
            character
                .moveset
                .clips
                .get(&track.motion)
                .and_then(|c| c.root.get((track.frame as usize).min(c.root.len().saturating_sub(1))))
                .copied()
                .unwrap_or([0.0; 3])
        } else {
            [0.0; 3]
        };
        self.object = Mat4::from_rotation_translation(Quat::from_rotation_y(out.facing), Vec3::from(pos));
        self.to_world = self.object * Mat4::from_translation(-Vec3::from(root));
    }

    fn weapon_pose(&self, rig: &Rig, track: Track, bones: &[Mat4]) -> Vec<Mat4> {
        let Some(hand) = rig.weapon_bone.and_then(|b| bones.get(b)).copied() else { return Vec::new() };
        let m: Option<&Arc<Interpolated>> = rig.weapon_motions.get(&track.motion).or_else(|| rig.weapon_motions.get(&0));
        let local = match m {
            Some(m) => evaluate(&rig.weapon_skeleton, m, key_time(m, track.frame, track.looping), Options::default()).bones,
            None => kh2_anim::Pose::bind(&rig.weapon_skeleton).bones,
        };
        local.iter().map(|w| hand * *w).collect()
    }

    /// World-space (KH2 units) sphere centres and radii for an attack group.
    pub fn attack_spheres(&self, group: u8, weapon: bool) -> Vec<(Vec3, f32)> {
        let list = if weapon { &self.rig.weapon_attacks } else { &self.rig.body_attacks };
        list.iter().filter(|s| s.group == group).map(|s| (self.sphere_center(s, weapon), s.radius)).collect()
    }

    pub fn hurt_spheres(&self) -> Vec<(Vec3, f32)> {
        self.rig.hurt.iter().map(|s| (self.sphere_center(s, false), s.radius)).collect()
    }

    fn sphere_center(&self, s: &BoneSphere, weapon: bool) -> Vec3 {
        let off = Vec3::from(s.offset);
        let bones = if weapon { &self.weapon } else { &self.bones };
        match s.bone.and_then(|b| bones.get(b)) {
            Some(m) => self.to_world.transform_point3(m.transform_point3(off)),
            None => self.object.transform_point3(off),
        }
    }

    pub fn bone_world(&self, i: usize) -> Vec3 {
        self.to_world.transform_point3(self.bones[i].w_axis.truncate())
    }

    pub fn weapon_world(&self, i: usize) -> Vec3 {
        self.to_world.transform_point3(self.weapon[i].w_axis.truncate())
    }
}

/// Which rig a mannequin segment belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Body,
    Weapon,
}

/// One visible capsule between a joint and its parent.
#[derive(Component)]
pub struct Segment {
    pub part: Part,
    pub child: usize,
    pub parent: usize,
    pub radius: f32,
}

/// Segments worth drawing: joints whose bind-pose distance from their
/// parent is visible. Thickness grows with length (thin fingers and hair,
/// thicker limbs), which is our own styling.
pub fn segments(skeleton: &[kh2_formats::anim::Bone], part: Part) -> Vec<Segment> {
    let bind = kh2_anim::Pose::bind(skeleton);
    let mut out = Vec::new();
    for (i, b) in skeleton.iter().enumerate() {
        if b.parent < 0 {
            continue;
        }
        let p = b.parent as usize;
        let len = bind.bone_position(i).distance(bind.bone_position(p));
        if len < 1.0 {
            continue;
        }
        let radius = match part {
            Part::Body => (len * 0.2).clamp(0.8, 5.5),
            Part::Weapon => (len * 0.25).clamp(1.0, 3.0),
        };
        out.push(Segment { part, child: i, parent: p, radius });
    }
    out
}
