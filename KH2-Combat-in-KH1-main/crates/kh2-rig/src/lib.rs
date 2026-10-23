//! # kh2-rig
//!
//! Plays Sora's (or an enemy's) real motions on his real skeleton from a character
//! controller's output, the way the game does: which motion, the blends between them, the
//! weapons following their hand bones, root motion taken out of the pose. Also places the rig's
//! collision shapes (attack, guard, hurt) in the world.
//!
//! No engine here: the playground draws a mannequin on these bones, and the KH1 hook puts
//! them on KH1's Sora.

use std::collections::HashMap;
use std::sync::Arc;

use character_core::npc::{NpcOutput, NpcState};
use character_core::{motion, ActiveHitbox, Character, Clip, Output, StateName};
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
        (motion::GLIDE..=motion::GLIDE_HOVER).contains(&m)
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
    /// Playing the motion's out-of-combat (field) variant.
    field: bool,
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
    /// Out of combat: idle, walk and run use their field variants and the
    /// weapons are put away while Sora just moves (the host decides when;
    /// see [`RigAnim::weapons_out`]).
    pub field: bool,
    /// Blend (60 Hz frames) into the motion queued after a finished action. The game uses none
    /// (`blend::QUEUED`); a host can soften the snap.
    pub queued_blend: f32,
    /// Motions shown as another, looping (`(motion, shown)`): a form whose clip for a motion is
    /// only Sora's own (Valor's landing is his, without the Keyblade) shows its stance instead.
    pub stand_in: Vec<(u16, u16)>,
}

/// Movement motions that keep the weapons put away out of combat. Attacks,
/// guard, magic, reactions and the rest bring them out.
pub fn moves_without_weapon(m: u16) -> bool {
    matches!(
        m,
        motion::IDLE
            | motion::WALK
            | motion::RUN
            | motion::JUMP
            | motion::FALL
            | motion::LAND
            | motion::HIGH_JUMP
            | motion::AERIAL_DODGE
            | motion::QUICK_RUN
            | motion::QUICK_RUN_END
            | motion::DODGE_ROLL
            | motion::HANG
            | motion::CLIMB
    ) || blend::is_glide(m)
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
        RigAnim { rig, track: None, bones, weapons, to_world: Mat4::IDENTITY, object: Mat4::IDENTITY, blend_from: None, blend_t: 1.0, blend_len: blend::DEFAULT, field: false, queued_blend: blend::QUEUED, stand_in: Vec::new() }
    }

    /// The keyframes a track plays: the field variant when it asks for one.
    fn motion_for<'a>(rig: &'a Rig, m: u16, field: bool) -> Option<&'a Arc<Interpolated>> {
        field.then(|| rig.field_motions.get(&m)).flatten().or_else(|| rig.motions.get(&m))
    }

    /// The motion playing, whether its field slot is (one of its own), its frame and length (60
    /// Hz frames): for the host's footsteps (`Rig::footsteps`).
    pub fn playing(&self) -> Option<(u16, bool, f32, f32)> {
        let t = self.track?;
        let field = t.field && self.rig.field_motions.contains_key(&t.motion);
        let m = Self::motion_for(&self.rig, t.motion, t.field)?;
        Some((t.motion, field, t.frame, kh2_anim::length_frames(m)))
    }

    /// Whether to draw the weapons: always in battle; out of combat only
    /// while something other than plain movement plays.
    pub fn weapons_out(&self) -> bool {
        !self.field || self.track.map_or(false, |t| !moves_without_weapon(t.motion))
    }

    fn pick(&self, out: &Output) -> Track {
        let t = self.pick_motion(out);
        match self.stand_in.iter().find(|(m, _)| *m == t.motion) {
            Some(&(_, shown)) => {
                let frame = match self.track {
                    Some(p) if p.motion == shown && !p.field => p.frame + 1.0,
                    _ => 0.0,
                };
                Track { motion: shown, frame, looping: true, field: false, root: false }
            }
            None => t,
        }
    }

    fn pick_motion(&self, out: &Output) -> Track {
        let prev = self.track;
        let field_of = |m: u16| self.field && self.rig.field_motions.contains_key(&m);
        let free = |m: u16, looping: bool| {
            let field = field_of(m);
            let frame = match prev {
                Some(p) if p.motion == m && p.field == field => p.frame + 1.0,
                _ => 0.0,
            };
            Track { motion: m, frame, looping, field, root: false }
        };
        // Decide by the motion, not the state: on an action's last frame the
        // controller already reports a free state but still names the
        // action's motion.
        let free_motion = matches!(out.motion, motion::IDLE | motion::WALK | motion::RUN | motion::FALL);
        match out.state {
            StateName::Idle | StateName::Walk | StateName::Run if free_motion => free(out.motion, true),
            // The action's last frame, reported in a free state: a host that softens the snap
            // into the next motion starts it now rather than holding the action's end a frame.
            StateName::Idle | StateName::Walk | StateName::Run if self.queued_blend > blend::MIN => {
                free(match out.state { StateName::Walk => motion::WALK, StateName::Run => motion::RUN, _ => motion::IDLE }, true)
            }
            StateName::Glide => free(out.motion, true),
            StateName::Air => {
                // Keep playing the jump while rising, then the fall loop.
                let rising = out.displacement[1] > 0.05;
                // Only while the clip lasts: past its end it would hold its last frame, frozen,
                // for as long as the rise goes on (an aerial dodge's end).
                let left = |p: &Track| Self::motion_for(&self.rig, p.motion, p.field).is_some_and(|m| p.frame + 1.0 < kh2_anim::length_frames(m));
                // The High Jump's flip comes about its apex: it plays to its end however the
                // rise goes (a short hop included), so a High Jump always flips.
                match prev {
                    Some(p) if p.motion == motion::HIGH_JUMP && left(&p) => Track { frame: p.frame + 1.0, looping: false, root: false, ..p },
                    Some(p) if rising && (p.motion == motion::JUMP || p.motion == motion::AERIAL_DODGE) && left(&p) => {
                        Track { frame: p.frame + 1.0, looping: false, root: false, ..p }
                    }
                    _ => free(motion::FALL, true),
                }
            }
            _ => Track { motion: out.motion, frame: out.clip_frame as f32 + 1.0, looping: false, field: field_of(out.motion), root: true },
        }
    }

    /// Advances one 60 Hz frame from the controller's output.
    pub fn update(&mut self, out: &Output, character: &Character, pos: [f32; 3]) {
        let mut track = self.pick(out);
        let rig = self.rig.clone();
        let Some(m) = Self::motion_for(&rig, track.motion, track.field).or_else(|| self.track.and_then(|t| Self::motion_for(&rig, t.motion, t.field))) else { return };
        if !rig.motions.contains_key(&track.motion) {
            track = self.track.unwrap_or(track);
        }
        // Switching between the battle and field variants blends like a new
        // motion.
        let restarted = match self.track {
            Some(p) => p.motion != track.motion || p.field != track.field || track.frame < p.frame,
            None => false,
        };
        if let (true, Some(prev)) = (restarted, self.track) {
            let mut frames = blend::frames(character, prev.motion, track.motion);
            if frames <= blend::QUEUED && blend::is_free(track.motion) && !blend::is_free(prev.motion) {
                frames = frames.max(self.queued_blend);
            }
            // Walk <-> run keeps the gait's phase (the game scales the old
            // motion's progress, plus the blend, onto the new one's length).
            let gait = |m: u16| m == motion::WALK || m == motion::RUN;
            if gait(prev.motion) && gait(track.motion) && prev.motion != track.motion {
                if let Some(pm) = Self::motion_for(&rig, prev.motion, prev.field) {
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
                let idle_field = self.field && rig.field_motions.contains_key(&motion::IDLE);
                let idle = Track { motion: motion::IDLE, frame: 0.0, looping: true, field: idle_field, root: false };
                self.blend_from = match (via_idle, Self::motion_for(&rig, motion::IDLE, idle_field)) {
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
            Track { motion: out.motion, frame, looping: true, field: false, root: false }
        } else {
            Track { motion: out.motion, frame: out.clip_frame as f32 + 1.0, looping: false, field: false, root: out.root_motion }
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

/// Marks the swing ids [`MotionHits`] hands out (the controller's are small counters).
pub const MOTION_SWING: u64 = 1 << 62;

/// The weapons' attack windows of the motion a rig shows, for moves the controller doesn't run as
/// actions (jumping on after the jump's push, falling, gliding). KH2's weapon motions carry their
/// own attack triggers, live whatever the player's action: Final Form's Keyblades hurt through
/// its jump (`W_EX010_ULTF_R/_L` motion 3: ATKP 1273/1274 frames 2..12, then 1286/1287), fall
/// (4: 1275/1276, frames 2..20) and glide (205..207: 1281/1282 all through, re-hitting every 30
/// frames). Guard windows (the glide's hover, 208) are left out.
#[derive(Clone, Debug, Default)]
pub struct MotionHits {
    /// Windows open last frame: (attack, group, weapon) and their swing id.
    open: Vec<((u16, u8, u8), u64)>,
    runs: u64,
}

impl MotionHits {
    /// This frame's, from the clip `anim` shows at its frame (its field variant has none of its
    /// own). `on`: the host's say whether the controller leaves the motion's windows to us (else
    /// everything closes).
    pub fn step(&mut self, anim: &RigAnim, character: &Character, on: bool) -> Vec<ActiveHitbox> {
        let playing = anim.playing().filter(|p| on && !p.1);
        let clip = playing.and_then(|p| character.moveset.clips.get(&p.0));
        self.step_at(clip, playing.map_or(0.0, |p| p.2))
    }

    /// As [`MotionHits::step`], from a clip and its frame (`None`: nothing open). A window open
    /// from one frame to the next keeps its swing id, the same attack across motions too (the
    /// glide and its banked turns share 1281/1282), so a foe is hit once a window, or again
    /// after its row's re-hit interval; one that closes and opens again is a new swing.
    pub fn step_at(&mut self, clip: Option<&Clip>, frame: f32) -> Vec<ActiveHitbox> {
        let f = frame.max(0.0) as u32;
        let mut now: Vec<((u16, u8, u8), u64)> = Vec::new();
        for w in clip.map_or(&[][..], |c| c.windows.as_slice()).iter().filter(|w| w.contains(f)) {
            let character_core::WindowKind::Hitbox { attack, group, weapon, .. } = w.kind else { continue };
            let key = (attack, group, weapon);
            if weapon == 0 || now.iter().any(|(k, _)| *k == key) {
                continue;
            }
            let id = match self.open.iter().find(|(k, _)| *k == key) {
                Some(&(_, id)) => id,
                None => {
                    self.runs += 1;
                    MOTION_SWING | self.runs
                }
            };
            now.push((key, id));
        }
        self.open = now;
        self.open.iter().map(|&((attack, group, weapon), swing)| ActiveHitbox { swing, attack, group, weapon }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use character_core::{Window, WindowKind};

    fn clip(motion: u16, windows: &[(u32, Option<u32>, u16, u8)]) -> Clip {
        let windows = windows.iter().map(|&(start, end, attack, weapon)| Window { start, end, kind: WindowKind::Hitbox { attack, group: 2, combo: None, weapon } }).collect();
        Clip { motion, length: 60, root: Vec::new(), windows, events: Vec::new() }
    }

    #[test]
    fn motion_hits_keep_their_swing_while_open() {
        let glide = clip(205, &[(0, None, 1281, 1), (0, None, 1282, 2)]);
        let turn = clip(206, &[(0, None, 1281, 1), (0, None, 1282, 2)]);
        let jump = clip(3, &[(2, Some(12), 1273, 1), (12, None, 1286, 1), (0, None, 9, 0)]);
        let mut h = MotionHits::default();
        let a = h.step_at(Some(&glide), 0.0);
        assert_eq!(a.len(), 2);
        assert!(a.iter().all(|x| x.swing & MOTION_SWING != 0));
        // The banked turn: the same attacks, the same swings.
        let b = h.step_at(Some(&turn), 0.0);
        assert_eq!(a, b);
        // Closed, then open again: new swings.
        assert!(h.step_at(None, 0.0).is_empty());
        let c = h.step_at(Some(&glide), 5.0);
        assert!(c.iter().all(|x| !a.iter().any(|y| y.swing == x.swing)));
        // The jump: its first window, then the next; the body's own (weapon 0) left out.
        assert!(h.step_at(Some(&jump), 1.0).is_empty());
        let j = h.step_at(Some(&jump), 2.0);
        assert_eq!(j.iter().map(|x| x.attack).collect::<Vec<_>>(), vec![1273]);
        assert_eq!(h.step_at(Some(&jump), 11.0), j);
        assert_eq!(h.step_at(Some(&jump), 12.0).iter().map(|x| x.attack).collect::<Vec<_>>(), vec![1286]);
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

