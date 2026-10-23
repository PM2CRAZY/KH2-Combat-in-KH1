//! Effect instances: where a PAX element is and whether it is still
//! playing. Particles are not simulated; the position, binding and
//! lifetime are (they drive the spells' logic).

use glam::{Mat4, Vec3};
use kh2_formats::pax::{flags, PaxEffect, PaxElement};

use crate::{flip, ObjId, World};

#[derive(Clone, Debug)]
pub struct Effect {
    pub id: u64,
    /// Index of the spell file it came from.
    pub spell: usize,
    pub element: PaxElement,
    pub timing: PaxEffect,
    /// Bound object, if any.
    pub owner: Option<ObjId>,
    /// Frames since it started (after its start wait).
    pub age: f32,
    pub wait: f32,
    pub ended_at: Option<f32>,
    /// Frames left, and the fade's length.
    pub fade: Option<(f32, f32)>,
    pub killed: bool,
    /// Frames a dead effect is kept around.
    pub linger: f32,
    /// World position (our axes) this frame.
    pub pos: Vec3,
    /// Set by a script: the effect stays there instead of following.
    pub placed: Option<Vec3>,
    pub dir: Vec3,
    pub scale: Vec3,
    start_motion: Option<u16>,
    start_pos: Option<Vec3>,
}

impl Effect {
    pub fn new(id: u64, spell: usize, element: PaxElement, timing: PaxEffect, owner: Option<ObjId>, world: &dyn World) -> Effect {
        let bound = owner.is_some() && element.flags & flags::BIND != 0;
        // The game adds a tenth of a frame to bound effects' waits.
        let wait = element.start_wait + if bound { 0.1 } else { 0.0 };
        let scale = Vec3::from(element.scale);
        let mut e = Effect {
            id,
            spell,
            timing,
            owner: owner.filter(|_| bound),
            age: 0.0,
            wait,
            ended_at: None,
            fade: None,
            killed: false,
            linger: 120.0,
            pos: Vec3::ZERO,
            placed: None,
            dir: Vec3::Z,
            scale: if scale == Vec3::ZERO { Vec3::ONE } else { scale },
            start_motion: owner.map(|o| world.motion(o)),
            start_pos: None,
            element,
        };
        e.pos = e.compute_pos(world);
        if e.element.flags & flags::BIND_ONLY_START != 0 {
            e.start_pos = Some(e.pos);
        }
        e
    }

    /// Where the element sits: its translation (game axes, y down) in the
    /// bound bone's space, or in world space when unbound.
    fn compute_pos(&self, world: &dyn World) -> Vec3 {
        if let Some(p) = self.placed {
            return p;
        }
        if let Some(p) = self.start_pos {
            return p;
        }
        let t = flip(Vec3::from(self.element.translation));
        match self.owner.and_then(|o| world.bone(o, self.element.bone)) {
            Some(m) => {
                if self.element.flags & flags::BIND_ONLY_POS != 0 {
                    m.w_axis.truncate() + t
                } else {
                    m.transform_point3(t)
                }
            }
            None => t,
        }
    }

    pub fn alive(&self) -> bool {
        if self.killed {
            return false;
        }
        if let Some((left, _)) = self.fade {
            if left <= 0.0 {
                return false;
            }
        }
        self.age < self.timing.length(self.ended_at)
    }

    /// The particle system still exists (the game keeps it until the
    /// effect dies).
    pub fn active(&self) -> bool {
        self.alive()
    }

    /// Fade multiplier for drawing.
    pub fn alpha(&self) -> f32 {
        match self.fade {
            Some((left, total)) if total > 0.0 => (left / total).clamp(0.0, 1.0),
            _ => 1.0,
        }
    }

    pub fn loop_end(&mut self) {
        if self.ended_at.is_none() {
            self.ended_at = Some(self.age);
        }
    }

    pub fn kill(&mut self) {
        self.killed = true;
    }

    pub fn fadeout(&mut self, frames: f32) {
        if frames > 0.0 {
            self.fade = Some((frames, frames));
        } else {
            self.kill();
        }
    }

    pub fn tick(&mut self, frametime: f32, world: &dyn World) {
        if self.wait > 0.0 {
            self.wait -= frametime;
        } else {
            self.age += frametime;
        }
        if let Some((left, _)) = self.fade.as_mut() {
            *left -= frametime;
        }
        if let Some(o) = self.owner {
            if !world.exists(o) && self.element.flags & flags::DESTROY_WHEN_OBJECT_LEAVES != 0 {
                self.kill();
            }
            if self.element.flags & flags::DESTROY_WHEN_MOTION_CHANGE != 0 && self.start_motion.is_some_and(|m| m != world.motion(o)) {
                self.kill();
            }
        }
        self.pos = self.compute_pos(world);
    }

    /// The bone's world matrix (for drawing orientation).
    pub fn frame(&self, world: &dyn World) -> Mat4 {
        self.owner.and_then(|o| world.bone(o, self.element.bone)).unwrap_or(Mat4::IDENTITY)
    }
}
