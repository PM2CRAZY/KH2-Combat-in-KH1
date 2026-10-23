//! The PAX element layer and the public runtime (`docs/kh2-effects.md` §2,
//! §5.5): starting elements at a bone, binding, start waits, the 30 Hz
//! effect clock, fading and ending.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat3, Mat4, Vec3};
use kh2_formats::pax::PaxElement;

use crate::camera::Camera;
use crate::draw;
use crate::library::Library;
use crate::math;
use crate::output::Frame;
use crate::player::{supported, Player};
use crate::rng::Rng;

/// Identifies a started effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectHandle(pub u32);

/// Element flags (`docs/kh2-effects.md` §2).
pub mod flags {
    pub const BIND: u32 = 0x01;
    pub const BIND_UNTIL_START: u32 = 0x02;
    pub const BIND_POSITION_ONLY: u32 = 0x04;
    pub const END_ON_MOTION_CHANGE: u32 = 0x20;
    pub const END_FADE: u32 = 0x40;
    pub const END_LOOP: u32 = 0x80;
    pub const BONE_SCALE: u32 = 0x100;
    pub const END_WITH_OWNER: u32 = 0x200;
    pub const WEAPON_VISIBILITY: u32 = 0x800;
    /// Placed in **camera space** every frame: the element's translation and rotation are
    /// taken relative to the camera, not to its bone (a screen-locked flash or tint).
    pub const CAMERA: u32 = 0x20000;
}

/// Game frames (60 Hz) per effect tick.
const TICK_FRAMES: f32 = 2.0;

struct Running {
    handle: u32,
    element: PaxElement,
    player: Player,
    wait: f32,
    acc: f32,
    alpha: f32,
    fade_rate: f32,
    fading: bool,
    started: bool,
    pending_advance: bool,
    visible: bool,
    pos: Vec3,
    rot: Mat3,
    scale: Vec3,
    /// Placed by the host (`set_anchor`): this matrix stands for the bone.
    anchor: Option<Mat4>,
}

/// Plays PAX elements.
///
/// Per game frame: [`Effects::tick`] once with the frame delta (1.0 per
/// 60 Hz frame) and the current bone matrices, then [`Effects::build`]
/// with the camera to get the triangles to draw.
pub struct Effects {
    lib: Arc<Library>,
    running: Vec<Running>,
    rng: Rng,
    next: u32,
    skipped: HashMap<u32, usize>,
    /// The camera view of the last [`Effects::build`], for the programs
    /// that read the camera in their calc (RyjMatrixNoRot, KeDMat).
    view: Cell<Mat4>,
}

/// The effect root from an element and its bone matrix: translation,
/// rotation (orthonormal) and scale.
pub fn element_transform(el: &PaxElement, bone: Mat4) -> (Vec3, Mat3, Vec3) {
    let t = Vec3::from(el.translation);
    let r = math::euler_xyz(Vec3::from(el.rotation));
    let mut scale = Vec3::from(el.scale);
    if el.flags & flags::BONE_SCALE != 0 {
        scale *= math::axis_lengths(&bone);
    }
    if el.flags & flags::BIND_POSITION_ONLY != 0 {
        return (bone.transform_point3(t), r, scale);
    }
    let (pos, rot) = rigid(bone * Mat4::from_translation(t) * Mat4::from_mat3(r));
    (pos, rot, scale)
}

/// Translation and orthonormal rotation of `m` (the game re-derives Euler angles from it).
fn rigid(m: Mat4) -> (Vec3, Mat3) {
    let x = m.x_axis.truncate().normalize_or_zero();
    let y = (m.y_axis.truncate() - x * x.dot(m.y_axis.truncate())).normalize_or_zero();
    let z = x.cross(y);
    let z = if z.dot(m.z_axis.truncate()) < 0.0 { -z } else { z };
    (m.w_axis.truncate(), Mat3::from_cols(x, y, z))
}

/// The effect root of a camera-space element (flag 0x20000, `docs/kh2-effects.md` §2): the
/// camera's frame (the inverse of `view`, made rigid) times the element's own
/// `T(translation) · R(rotation)`; scale is the element scale. The bone and the other
/// placement flags are not read. The game does this every frame (`0x1402c4240`).
pub fn camera_transform(el: &PaxElement, view: Mat4) -> (Vec3, Mat3, Vec3) {
    let (eye, cam) = rigid(view.inverse());
    let mut frame = Mat4::from_mat3(cam);
    frame.w_axis = eye.extend(1.0);
    let (pos, rot) = rigid(frame * Mat4::from_translation(Vec3::from(el.translation)) * Mat4::from_mat3(math::euler_xyz(Vec3::from(el.rotation))));
    (pos, rot, Vec3::from(el.scale))
}

/// Where an element's effect goes this frame, from its bone (or the camera, flag 0x20000).
fn place(el: &PaxElement, bone: Mat4, view: Mat4) -> (Vec3, Mat3, Vec3) {
    if el.flags & flags::CAMERA != 0 {
        camera_transform(el, view)
    } else {
        element_transform(el, bone)
    }
}

impl Effects {
    pub fn new(lib: Arc<Library>) -> Effects {
        Effects { lib, running: Vec::new(), rng: Rng::default(), next: 1, skipped: HashMap::new(), view: Cell::new(Mat4::IDENTITY) }
    }

    pub fn library(&self) -> &Arc<Library> {
        &self.lib
    }

    /// Reseed the particle random numbers (the C runtime's `srand`).
    pub fn seed(&mut self, seed: u32) {
        self.rng = Rng::new(seed);
    }

    /// Start the first element with this id. `bones` maps the element's
    /// bone code (a bone index, or 0x4000 object root, 0x4001 body
    /// centre, 0x4003+ weapon points) to that bone's world matrix (KH2
    /// world, y down; column vectors). An unknown bone starts at the
    /// origin. Returns `None` if the package has no such element.
    pub fn start(&mut self, element_id: u16, bones: &mut dyn FnMut(u16) -> Option<Mat4>) -> Option<EffectHandle> {
        let el = self.lib.element(element_id)?.clone();
        let (dpd, pd) = self.lib.effect_data(el.effect as u32)?;
        let mut player = Player::new(&self.lib, dpd, pd);
        for e in &self.lib.dpds[dpd].particles[pd].emitters {
            for p in &e.programs {
                if !supported(p.kind) {
                    *self.skipped.entry(p.kind).or_insert(0) += 1;
                }
            }
        }
        let bone = if el.flags & flags::CAMERA != 0 { Mat4::IDENTITY } else { bones(el.bone).unwrap_or(Mat4::IDENTITY) };
        let (pos, rot, scale) = place(&el, bone, self.view.get());
        player.set_root(&self.lib, pos, rot, scale);
        let handle = self.next;
        self.next = self.next.wrapping_add(1).max(1);
        let wait = el.start_wait + if el.flags & flags::BIND != 0 { 0.1 } else { 0.0 };
        self.running.push(Running {
            handle,
            element: el,
            player,
            wait,
            acc: 0.0,
            alpha: 1.0,
            fade_rate: 0.0,
            fading: false,
            started: false,
            pending_advance: false,
            visible: false,
            pos,
            rot,
            scale,
            anchor: None,
        });
        Some(EffectHandle(handle))
    }

    /// End an effect the way its element says: flag 0x80 stops its loops
    /// (live instances finish), flag 0x40 fades it out over the element's
    /// fade-out frames (0 = at once); otherwise it plays to its end.
    pub fn end(&mut self, h: EffectHandle) {
        let Some(i) = self.running.iter().position(|r| r.handle == h.0) else { return };
        let r = &mut self.running[i];
        let f = r.element.flags;
        if f & flags::END_LOOP != 0 {
            r.player.ended = true;
        } else if f & flags::END_FADE != 0 {
            let n = r.element.fadeout_frames as f32;
            if n > 0.0 {
                r.fading = true;
                r.fade_rate = -(r.alpha / n);
            } else {
                self.running.remove(i);
            }
        }
    }

    /// The object the effects are bound to is gone (KH2: a form change replaces the player
    /// object): elements flagged to end with their owner (0x200) end, faded over their fade-out
    /// frames or at once. Returns how many.
    pub fn owner_gone(&mut self) -> usize {
        let before = self.running.len();
        let mut n = 0;
        self.running.retain_mut(|r| {
            if r.element.flags & flags::END_WITH_OWNER == 0 {
                return true;
            }
            n += 1;
            let frames = r.element.fadeout_frames as f32;
            if frames > 0.0 && !r.fading {
                r.fading = true;
                r.fade_rate = -(r.alpha / frames);
                true
            } else {
                false
            }
        });
        let _ = before;
        n
    }

    /// Remove an effect now.
    pub fn kill(&mut self, h: EffectHandle) {
        self.running.retain(|r| r.handle != h.0);
    }

    /// Remove every effect.
    pub fn clear(&mut self) {
        self.running.clear();
    }

    pub fn is_running(&self, h: EffectHandle) -> bool {
        self.running.iter().any(|r| r.handle == h.0)
    }

    pub fn running_count(&self) -> usize {
        self.running.len()
    }

    /// Place an effect: `bone` (KH2 world, y down) stands for its bone from now on, followed
    /// every tick whatever its bind flags (a script moving an unbound effect, a shot).
    pub fn set_anchor(&mut self, h: EffectHandle, bone: Mat4) {
        if let Some(r) = self.running.iter_mut().find(|r| r.handle == h.0) {
            r.anchor = Some(bone);
            (r.pos, r.rot, r.scale) = place(&r.element, bone, self.view.get());
        }
    }

    /// Give the programs that read the camera in their calc
    /// (RyjMatrixNoRot, KeDMat, KeDMatFr) this frame's camera before
    /// [`Effects::tick`]. Without it they use the camera of the last
    /// [`Effects::build`], one frame old.
    pub fn set_camera(&mut self, camera: &Camera) {
        self.view.set(camera.view());
    }

    /// Multiply an effect's alpha (e.g. hide weapon effects, flag 0x800).
    pub fn set_alpha(&mut self, h: EffectHandle, alpha: f32) {
        if let Some(r) = self.running.iter_mut().find(|r| r.handle == h.0) {
            r.alpha = alpha;
        }
    }

    /// Program types met in started effects that this crate skips, with
    /// how many program records used them.
    pub fn skipped_programs(&self) -> Vec<(u32, &'static str, usize)> {
        let mut v: Vec<_> = self.skipped.iter().map(|(&k, &n)| (k, kh2_formats::dpd::program_name(k).unwrap_or("?"), n)).collect();
        v.sort();
        v
    }

    /// Advance by `game_frames` 60 Hz frames (1.0 per frame at 60 fps).
    /// Bound effects re-read their bone through `bones`.
    ///
    /// The effect clock: each effect adds the delta to an accumulator and
    /// runs one 30 Hz tick per 2.0. A tick is computed on the first of its
    /// two frames and shown on both; particle bursts step on the second,
    /// and the frames advance after it, as in the game.
    pub fn tick(&mut self, game_frames: f32, bones: &mut dyn FnMut(u16) -> Option<Mat4>) {
        let lib = self.lib.clone();
        let rng = &mut self.rng;
        let view = self.view.get();
        let dt = game_frames.max(0.0);
        self.running.retain_mut(|r| {
            r.visible = false;
            if r.pending_advance {
                r.pending_advance = false;
                r.player.advance(&lib, rng);
                if r.player.finished {
                    return false;
                }
            }
            let f = r.element.flags;
            let follow = f & flags::BIND != 0 && (f & flags::BIND_UNTIL_START == 0 || !r.started);
            if f & flags::CAMERA != 0 {
                // In place of any binding: re-placed in front of the camera every frame.
                (r.pos, r.rot, r.scale) = camera_transform(&r.element, view);
            } else if let Some(a) = r.anchor {
                (r.pos, r.rot, r.scale) = element_transform(&r.element, a);
            } else if follow {
                if let Some(b) = bones(r.element.bone) {
                    (r.pos, r.rot, r.scale) = element_transform(&r.element, b);
                }
            }
            if r.fading {
                r.alpha += r.fade_rate * dt;
                if r.alpha <= 0.0 {
                    return false;
                }
            }
            if r.wait > 0.0 {
                r.wait -= dt;
                if r.wait > 0.0 {
                    return true;
                }
            }
            r.started = true;
            r.player.set_root(&lib, r.pos, r.rot, r.scale);
            r.player.fx.colour = [1.0, 1.0, 1.0, r.alpha];
            r.player.fx.view = view;
            r.acc += dt;
            let repeat = r.acc < TICK_FRAMES;
            if !repeat {
                r.acc -= TICK_FRAMES;
                while r.acc >= TICK_FRAMES {
                    r.player.calc_pass(&lib, rng);
                    r.player.draw_pass(&lib, rng, false, false);
                    r.player.advance(&lib, rng);
                    r.acc -= TICK_FRAMES;
                    if r.player.finished {
                        return false;
                    }
                }
            }
            r.player.calc_pass(&lib, rng);
            r.player.draw_pass(&lib, rng, repeat, true);
            r.pending_advance = !repeat;
            r.visible = true;
            true
        });
    }

    /// The triangles of every visible effect for this camera, in the
    /// game's draw order (effects in start order). `out` is cleared first.
    pub fn build(&self, camera: &Camera, out: &mut Frame) {
        self.view.set(camera.view());
        out.clear();
        for r in &self.running {
            if r.visible {
                draw::build(&self.lib, &r.player, camera, out);
            }
        }
        out.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(flags: u32) -> PaxElement {
        PaxElement {
            effect: 0,
            id: 1,
            group: 0,
            fadeout_frames: 0,
            bone: 0x4000,
            flags,
            start_wait: 0.0,
            sound: 0,
            translation: [0.0, -10.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [2.0, 2.0, 2.0],
        }
    }

    #[test]
    fn transform_follows_bone() {
        let bone = Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0)) * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2) * Mat4::from_scale(Vec3::splat(3.0));
        let (p, r, s) = element_transform(&element(0), bone);
        // Translation (0, -10, 0) in bone space: rotated a quarter turn and scaled.
        assert!((p - Vec3::new(35.0, 0.0, 0.0)).length() < 1e-3, "{p}");
        assert!((r * Vec3::X - Vec3::Y).length() < 1e-5);
        assert_eq!(s, Vec3::splat(2.0));
        let (_, _, s) = element_transform(&element(flags::BONE_SCALE), bone);
        assert!((s - Vec3::splat(6.0)).length() < 1e-4);
        let (p, r, _) = element_transform(&element(flags::BIND_POSITION_ONLY), bone);
        assert!((p - Vec3::new(35.0, 0.0, 0.0)).length() < 1e-3);
        assert_eq!(r, Mat3::IDENTITY);
    }

    #[test]
    fn camera_element_sits_in_view_space() {
        // Limit Form's element 58: bone 0, flags 0x20001, 209 in front of the camera.
        let mut el = element(flags::CAMERA | flags::BIND);
        el.bone = 0;
        el.translation = [0.0, 11.0, 209.0];
        el.scale = [0.5; 3];
        let cam = Camera::look_at(Vec3::new(300.0, -200.0, 400.0), Vec3::new(0.0, -95.0, 0.0), Vec3::NEG_Y);
        // The bone is not read: whatever it is, the root is at (0, 11, 209) in view space,
        // turned with the camera.
        let bone = Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0));
        let (p, r, s) = place(&el, bone, cam.view());
        assert!((cam.view().transform_point3(p) - Vec3::new(0.0, 11.0, 209.0)).length() < 1e-3, "{p}");
        let vr = Mat3::from_mat4(cam.view()) * r;
        assert!((vr * Vec3::Z - Vec3::Z).length() < 1e-5 && (vr * Vec3::Y - Vec3::Y).length() < 1e-5);
        assert_eq!(s, Vec3::splat(0.5));
        // An ordinary element on the same bone stays with the bone.
        let (p, _, _) = place(&element(flags::BIND), bone, cam.view());
        assert!((p - Vec3::new(5.0, -10.0, 0.0)).length() < 1e-3);
    }
}
