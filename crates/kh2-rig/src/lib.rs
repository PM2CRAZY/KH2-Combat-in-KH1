//! # kh2-rig
//!
//! Plays Sora's (or an enemy's) real motions on his real skeleton from a character
//! controller's output, the way the game does: which motion, the blends between them, the
//! weapons following their hand bones, root motion taken out of the pose. Also places the rig's
//! collision shapes (attack, guard, hurt) in the world.
//!
//! No engine here: the playground draws a mannequin on these bones, and a host can put
//! them on its own character.

use std::collections::HashMap;
use std::sync::Arc;

use character_core::npc::{NpcOutput, NpcState};
use character_core::{motion, Character, Clip, Output, StateName};
use combat_core::{Shape as CShape, ShapeKind};
use glam::{Mat4, Quat, Vec3};
use kh2_anim::{evaluate, key_time, Options};
use kh2_data::sora::{BoneShape, Rig};
use kh2_formats::anim::Interpolated;

/// Collision types (MDLX collision entries; OpenKH's names).
pub mod kind {
    pub const BG: u8 = 0;
    pub const OBJ: u8 = 1;
    pub const HIT: u8 = 2;
    pub const TARGET: u8 = 3;
    pub const BG_PLAYER: u8 = 4;
    pub const ATTACK: u8 = 6;
    pub const GUARD: u8 = 13;
}

/// Motion blend lengths the executable passes when it starts a motion
/// (60 Hz frames). See docs/animation.md, "Blending".
pub mod blend {
    use character_core::{motion, Character};

    /// Idle/walk/run changes, and the default when an action requests a
    /// motion (jump, roll, guard, the fall after a jump, ...).
    pub const DEFAULT: f32 = 8.0;
    /// Gliding, and the general "start falling" helper.
    pub const GLIDE: f32 = 16.0;
    /// The motion queued to follow a finished action plays at once.
    pub const QUEUED: f32 = 0.0;
    /// Blends of 2 frames or less are skipped.
    pub const MIN: f32 = 2.0;

    pub fn is_glide(m: u16) -> bool {
        (motion::GLIDE..=motion::GLIDE_END).contains(&m)
    }

    pub fn is_free(m: u16) -> bool {
        matches!(m, motion::IDLE | motion::WALK | motion::RUN | motion::FALL) || is_glide(m)
    }

    /// Blend length for switching from motion `from` to `to`.
    pub fn frames(character: &Character, from: u16, to: u16) -> f32 {
        if let Some(row) = character.moveset.combo_rows.iter().find(|r| r.motion == to) {
            // Attacks: PTYA blend_time (0 for all of Sora's rows).
            return row.blend_time;
        }
        if is_glide(to) {
            return GLIDE;
        }
        if is_free(to) && !is_free(from) {
            // A jump hands over to the fall itself (default blend); other
            // actions queue their follow-up motion, which starts at once.
            let jumpish = matches!(from, motion::JUMP | motion::HIGH_JUMP | motion::AERIAL_DODGE);
            return if jumpish { DEFAULT } else { QUEUED };
        }
        DEFAULT
    }
}

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
    /// Each weapon's bones (right hand first).
    pub weapons: Vec<Vec<Mat4>>,
    /// Model space → world (KH2 units).
    pub to_world: Mat4,
    /// Object root (feet, facing; no root-motion offset) → world.
    pub object: Mat4,
    blend_from: Option<(Vec<(Vec3, Quat, Vec3)>, Vec<Vec<(Vec3, Quat, Vec3)>>)>,
    blend_t: f32,
    blend_len: f32,
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
        let weapons = rig.weapons.iter().map(|w| kh2_anim::Pose::bind(&w.skeleton).bones).collect();
        RigAnim { rig, track: None, bones, weapons, to_world: Mat4::IDENTITY, object: Mat4::IDENTITY, blend_from: None, blend_t: 1.0, blend_len: blend::DEFAULT }
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
        if let (true, Some(prev)) = (restarted, self.track) {
            let frames = blend::frames(character, prev.motion, track.motion);
            // Walk <-> run keeps the gait's phase (the game scales the old
            // motion's progress, plus the blend, onto the new one's length).
            let gait = |m: u16| m == motion::WALK || m == motion::RUN;
            if gait(prev.motion) && gait(track.motion) && prev.motion != track.motion {
                if let Some(pm) = rig.motions.get(&prev.motion) {
                    let (pl, nl) = (kh2_anim::length_frames(pm), kh2_anim::length_frames(m));
                    let pos = key_time(pm, prev.frame, true) - pm.frame_start * 60.0 / pm.fps.max(1e-3);
                    if pl > 0.0 && nl > 0.0 {
                        track.frame = (nl * (1.0 + pos + frames) / pl) % nl;
                    }
                }
            }
            // An action's queued follow-up is idle, which starts at once; the
            // idle state then blends into walk/run. So blend from idle's
            // first pose rather than the action's last.
            let via_idle = !blend::is_free(prev.motion) && gait(track.motion);
            let frames = if via_idle { blend::DEFAULT } else { frames };
            if frames > blend::MIN {
                let idle = Track { motion: motion::IDLE, frame: 0.0, looping: true, root: false };
                self.blend_from = match (via_idle, rig.motions.get(&motion::IDLE)) {
                    (true, Some(im)) => {
                        let b = evaluate(&rig.skeleton, im, key_time(im, 0.0, true), Options::default()).bones;
                        let w = self.weapon_poses(&rig, idle, &b);
                        Some((decompose(&b), w.iter().map(|w| decompose(w)).collect()))
                    }
                    _ => Some((decompose(&self.bones), self.weapons.iter().map(|w| decompose(w)).collect())),
                };
                // The game's blend clock starts at 2 and the weight is
                // clock / length, ending once it reaches the length.
                self.blend_t = blend::MIN;
                self.blend_len = frames;
            } else {
                self.blend_from = None;
            }
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
        let mut weapons = self.weapon_poses(&rig, track, &bones);
        if let Some((fb, fw)) = &self.blend_from {
            self.blend_t += 1.0;
            let w = (self.blend_t / self.blend_len).min(1.0);
            blend(fb, &mut bones, w);
            for (to, from) in weapons.iter_mut().zip(fw) {
                blend(from, to, w);
            }
            if w >= 1.0 {
                self.blend_from = None;
            }
        }
        self.bones = bones;
        self.weapons = weapons;
        self.object = Mat4::from_rotation_translation(Quat::from_rotation_y(out.facing), Vec3::from(pos));
        self.to_world = self.object;
    }

    /// Advances one frame for an enemy from its controller's output. Loops
    /// (idle, walk) run freely; actions follow the controller's frame.
    /// Blends: reactions start at once (the generic reaction passes 0);
    /// everything else uses the default 8 (the enemy scripts choose theirs).
    pub fn update_npc(&mut self, out: &NpcOutput, clips: &HashMap<u16, Clip>, pos: [f32; 3]) {
        let rig = self.rig.clone();
        let prev = self.track;
        let mut track = if out.looping {
            let frame = match prev {
                Some(p) if p.motion == out.motion => p.frame + 1.0,
                _ => 0.0,
            };
            Track { motion: out.motion, frame, looping: true, root: false }
        } else {
            Track { motion: out.motion, frame: out.clip_frame as f32 + 1.0, looping: false, root: out.root_motion }
        };
        let Some(m) = rig.motions.get(&track.motion).or_else(|| prev.and_then(|t| rig.motions.get(&t.motion))) else { return };
        if !rig.motions.contains_key(&track.motion) {
            track = prev.unwrap_or(track);
        }
        if let Some(p) = prev {
            if p.motion != track.motion || track.frame < p.frame {
                let instant = matches!(out.state, NpcState::Hurt | NpcState::Blown | NpcState::Down | NpcState::Dead);
                if instant {
                    self.blend_from = None;
                } else {
                    self.blend_from = Some((decompose(&self.bones), Vec::new()));
                    self.blend_t = blend::MIN;
                    self.blend_len = blend::DEFAULT;
                }
            }
        }
        self.track = Some(track);
        let root = if track.root {
            clips.get(&track.motion).and_then(|c| c.root.get((track.frame as usize).min(c.root.len().saturating_sub(1)))).copied().unwrap_or([0.0; 3])
        } else {
            [0.0; 3]
        };
        let unroot = Mat4::from_translation(-Vec3::from(root));
        let pose = evaluate(&rig.skeleton, m, key_time(m, track.frame, track.looping), Options::default());
        let mut bones: Vec<Mat4> = pose.bones.iter().map(|b| unroot * *b).collect();
        if let Some((fb, _)) = &self.blend_from {
            self.blend_t += 1.0;
            let w = (self.blend_t / self.blend_len).min(1.0);
            blend(fb, &mut bones, w);
            if w >= 1.0 {
                self.blend_from = None;
            }
        }
        self.bones = bones;
        self.object = Mat4::from_rotation_translation(Quat::from_rotation_y(out.facing), Vec3::from(pos));
        self.to_world = self.object;
    }

    fn weapon_poses(&self, rig: &Rig, track: Track, bones: &[Mat4]) -> Vec<Vec<Mat4>> {
        rig.weapons
            .iter()
            .map(|wr| {
                let Some(hand) = wr.bone.and_then(|b| bones.get(b)).copied() else { return Vec::new() };
                let m: Option<&Arc<Interpolated>> = wr.motions.get(&track.motion).or_else(|| wr.motions.get(&0));
                let local = match m {
                    Some(m) => evaluate(&wr.skeleton, m, key_time(m, track.frame, track.looping), Options::default()).bones,
                    None => kh2_anim::Pose::bind(&wr.skeleton).bones,
                };
                local.iter().map(|w| hand * *w).collect()
            })
            .collect()
    }

    /// Bones of the body (`weapon` 0) or of weapon `weapon − 1`.
    fn bones_of(&self, weapon: u8) -> &[Mat4] {
        match weapon {
            0 => &self.bones,
            w => self.weapons.get(w as usize - 1).map_or(&[], |v| v.as_slice()),
        }
    }

    /// A collision shape placed in the world (KH2 units, y-up): the offset
    /// through its bone (or the object root), then raised by its lift.
    /// `weapon` 0 is the body, 1.. the weapons.
    pub fn world_shape(&self, s: &BoneShape, weapon: u8) -> Option<CShape> {
        let kind = ShapeKind::from_u8(s.shape)?;
        let off = Vec3::from(s.offset);
        let bones = self.bones_of(weapon);
        let c = match s.bone {
            Some(b) => self.to_world.transform_point3(bones.get(b)?.transform_point3(off)),
            None => self.object.transform_point3(off),
        } + Vec3::Y * s.lift;
        Some(CShape::new(kind, c.into(), s.radius, s.height))
    }

    /// Every shape of one collision type, with what it is on (0 the body,
    /// 1.. a weapon).
    pub fn shapes_of_kind(&self, kind: u8) -> Vec<(CShape, u8)> {
        let body = self.rig.body_shapes.iter().map(|s| (s, 0u8));
        let weapons = self.rig.weapons.iter().enumerate().flat_map(|(i, w)| w.shapes.iter().map(move |s| (s, i as u8 + 1)));
        body.chain(weapons).filter(|(s, _)| s.kind == kind).filter_map(|(s, w)| Some((self.world_shape(s, w)?, w))).collect()
    }

    /// The shapes an attack (or guard) window switches on: attack- and
    /// guard-type shapes of its group, on the Keyblade or on Sora, in data
    /// order (the game's order, which the sweep relies on).
    pub fn attack_shapes(&self, group: u8, weapon: u8) -> Vec<CShape> {
        let list = match weapon {
            0 => &self.rig.body_shapes,
            w => match self.rig.weapons.get(w as usize - 1) {
                Some(r) => &r.shapes,
                None => return Vec::new(),
            },
        };
        list.iter()
            .filter(|s| s.group == group && (s.kind == kind::ATTACK || s.kind == kind::GUARD))
            .filter_map(|s| self.world_shape(s, weapon))
            .collect()
    }

    /// Sora's hit (hurt) shapes.
    pub fn hurt_shapes(&self) -> Vec<CShape> {
        self.shapes_of_kind(kind::HIT).into_iter().filter(|(_, w)| *w == 0).map(|(s, _)| s).collect()
    }

    /// Bone joints as line segments (parent → child), world space.
    pub fn skeleton_lines(&self) -> Vec<(Vec3, Vec3, Part)> {
        let mut out = Vec::new();
        let parts = std::iter::once((Part::Body, &self.rig.skeleton, &self.bones))
            .chain(self.rig.weapons.iter().zip(&self.weapons).enumerate().map(|(i, (w, b))| (Part::Weapon(i as u8), &w.skeleton, b)));
        for (part, skel, bones) in parts {
            for (i, b) in skel.iter().enumerate() {
                let (Some(m), Some(p)) = (bones.get(i), usize::try_from(b.parent).ok().and_then(|p| bones.get(p))) else { continue };
                out.push((self.to_world.transform_point3(p.w_axis.truncate()), self.to_world.transform_point3(m.w_axis.truncate()), part));
            }
        }
        out
    }

    /// World position (KH2 units) of an anchor, if its bone exists.
    pub fn anchor(&self, part: Part, a: Anchor) -> Option<Vec3> {
        let bones: &[Mat4] = match part {
            Part::Body => &self.bones,
            Part::Weapon(i) => self.weapons.get(i as usize)?,
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
    /// A weapon, by index (right hand first).
    Weapon(u8),
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

