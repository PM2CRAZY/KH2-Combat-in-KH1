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
        // Decide by the motion, not the state: on an action's last frame the
        // controller already reports a free state but still names the
        // action's motion.
        let free_motion = matches!(out.motion, motion::IDLE | motion::WALK | motion::RUN | motion::FALL);
        match out.state {
            StateName::Idle | StateName::Walk | StateName::Run if free_motion => free(out.motion, true),
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

        // Root motion already moved the character; take the clip's current
        // root offset out of the pose so everything is in object space (this
        // keeps blends between clips from jumping).
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
        let unroot = Mat4::from_translation(-Vec3::from(root));
        let t = key_time(m, track.frame, track.looping);
        let pose = evaluate(&rig.skeleton, m, t, Options::default());
        let mut bones: Vec<Mat4> = pose.bones.iter().map(|b| unroot * *b).collect();
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
        self.object = Mat4::from_rotation_translation(Quat::from_rotation_y(out.facing), Vec3::from(pos));
        self.to_world = self.object;
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

    /// World position (KH2 units) of an anchor, if its bone exists.
    pub fn anchor(&self, part: Part, a: Anchor) -> Option<Vec3> {
        let bones = match part {
            Part::Body => &self.bones,
            Part::Weapon => &self.weapon,
        };
        let p = match a {
            Anchor::Bone(i) => bones.get(i)?.w_axis.truncate(),
            Anchor::Lerp(i, j, t) => bones.get(i)?.w_axis.truncate().lerp(bones.get(j)?.w_axis.truncate(), t),
            Anchor::Offset(i, v) => bones.get(i)?.transform_point3(v),
        };
        Some(self.to_world.transform_point3(p))
    }
}

/// Which skeleton a mannequin piece follows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
    Body,
    Weapon,
}

/// A point on the skeleton.
#[derive(Clone, Copy, Debug)]
pub enum Anchor {
    Bone(usize),
    /// Between two bones (0 = first, 1 = second).
    Lerp(usize, usize, f32),
    /// An offset in a bone's own space.
    Offset(usize, Vec3),
}

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Capsule(Anchor, Anchor, f32),
    Sphere(Anchor, f32),
}

/// Palette slots (our own styling).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Paint {
    Jacket,
    Pants,
    Skin,
    Shoe,
    Steel,
    Gold,
    Grip,
}

impl Paint {
    pub const ALL: [Paint; 7] = [Paint::Jacket, Paint::Pants, Paint::Skin, Paint::Shoe, Paint::Steel, Paint::Gold, Paint::Grip];

    pub fn material(self) -> StandardMaterial {
        let (c, rough, metal) = match self {
            Paint::Jacket => (Color::srgb(0.16, 0.17, 0.22), 0.7, 0.0),
            Paint::Pants => (Color::srgb(0.24, 0.32, 0.52), 0.75, 0.0),
            Paint::Skin => (Color::srgb(0.93, 0.88, 0.82), 0.55, 0.0),
            Paint::Shoe => (Color::srgb(0.82, 0.24, 0.22), 0.5, 0.0),
            Paint::Steel => (Color::srgb(0.82, 0.84, 0.9), 0.25, 0.8),
            Paint::Gold => (Color::srgb(0.95, 0.75, 0.3), 0.3, 0.9),
            Paint::Grip => (Color::srgb(0.2, 0.2, 0.25), 0.6, 0.2),
        };
        StandardMaterial { base_color: c, perceptual_roughness: rough, metallic: metal, ..default() }
    }
}

/// One mannequin piece, posed every frame.
#[derive(Component, Clone, Copy, Debug)]
pub struct Piece {
    pub part: Part,
    pub shape: Shape,
    pub paint: Paint,
}

/// Sora's bone roles (indices into `P_EX100`'s skeleton, found from the bind
/// pose's hierarchy; only the bones the mannequin needs).
mod sora_bones {
    pub const PELVIS: usize = 3;
    pub const SPINE: usize = 67;
    pub const CHEST: usize = 68;
    pub const NECK: usize = 138;
    pub const HEAD_TOP: usize = 141;
    /// (shoulder, elbow, wrist, hand, middle-finger tip)
    pub const ARMS: [[usize; 5]; 2] = [[76, 79, 80, 86, 99], [168, 171, 172, 178, 191]];
    /// (hip, knee, ankle, heel, toe)
    pub const LEGS: [[usize; 5]; 2] = [[11, 12, 13, 16, 17], [40, 41, 42, 45, 46]];
    pub const COUNT: usize = 256;
}

/// A simple, rounded mannequin on Sora's skeleton (our own design: torso,
/// head, limbs, mitten hands, big shoes) and a Keyblade-shaped prop on the
/// weapon skeleton. Falls back to one capsule per bone for other skeletons.
pub fn pieces(rig: &Rig) -> Vec<Piece> {
    use sora_bones::*;
    use Anchor::*;
    use Shape::*;
    let mut out = Vec::new();
    let mut add = |part, shape, paint| out.push(Piece { part, shape, paint });
    if rig.skeleton.len() == COUNT {
        let b = Part::Body;
        add(b, Sphere(Lerp(PELVIS, SPINE, 0.15), 10.5), Paint::Pants);
        add(b, Capsule(Lerp(PELVIS, SPINE, 0.35), Bone(CHEST), 10.0), Paint::Jacket);
        add(b, Capsule(Bone(CHEST), Bone(NECK), 3.6), Paint::Skin);
        add(b, Sphere(Lerp(NECK, HEAD_TOP, 0.45), 10.5), Paint::Skin);
        for [sh, el, wr, hand, tip] in ARMS {
            add(b, Sphere(Bone(sh), 5.2), Paint::Jacket);
            add(b, Capsule(Bone(sh), Bone(el), 4.0), Paint::Jacket);
            add(b, Capsule(Bone(el), Bone(wr), 3.5), Paint::Skin);
            add(b, Sphere(Lerp(hand, tip, 0.4), 4.6), Paint::Skin);
        }
        for [hip, knee, ankle, heel, toe] in LEGS {
            add(b, Capsule(Bone(hip), Bone(knee), 6.0), Paint::Pants);
            add(b, Capsule(Bone(knee), Bone(ankle), 4.8), Paint::Pants);
            add(b, Capsule(Lerp(heel, ankle, 0.35), Bone(toe), 5.6), Paint::Shoe);
        }
    } else {
        for s in segments(&rig.skeleton) {
            add(Part::Body, Capsule(Bone(s.0), Bone(s.1), s.2), Paint::Jacket);
        }
    }
    if rig.weapon_skeleton.len() >= 12 {
        // The blade is rigid on bone 2 and points along its -X (where the
        // attack spheres sit); bones 3..11 are the dangling keychain.
        let w = Part::Weapon;
        let along = |d: f32| Offset(2, Vec3::new(-d, 0.0, 0.0));
        add(w, Capsule(Bone(2), Bone(3), 1.7), Paint::Grip);
        add(w, Capsule(along(3.0), along(-3.0), 4.5), Paint::Gold);
        add(w, Capsule(along(4.0), along(70.0), 2.0), Paint::Steel);
        add(w, Sphere(along(66.0), 4.2), Paint::Steel);
        for i in 3..11 {
            add(w, Capsule(Bone(i), Bone(i + 1), 0.6), Paint::Gold);
        }
    }
    out
}

/// (child, parent, radius) for every visible bone segment.
fn segments(skeleton: &[kh2_formats::anim::Bone]) -> Vec<(usize, usize, f32)> {
    let bind = kh2_anim::Pose::bind(skeleton);
    let mut out = Vec::new();
    for (i, b) in skeleton.iter().enumerate() {
        if b.parent < 0 {
            continue;
        }
        let p = b.parent as usize;
        let len = bind.bone_position(i).distance(bind.bone_position(p));
        if len >= 1.0 {
            out.push((i, p, (len * 0.2).clamp(0.8, 5.5)));
        }
    }
    out
}
