//! pppRyjMegaBirth (type 201): sprite particle bursts
//! (`docs/kh2-effects.md` §8).
//!
//! The key (220 bytes) describes how many particle slots there are, how
//! often and how many are born, where they start (cone, sphere, box or the
//! vertices of a model) and how their speed, drift, scale, spin and colour
//! evolve. Every live particle is a camera-facing copy of a shape.

use glam::{Mat4, Vec3};
use kh2_formats::dpd::Key;
use std::f32::consts::PI;

use crate::library::Library;
use crate::math::{self, norm_or_zero};
use crate::player::{anim_step, EffectView};
use crate::rng::Rng;

/// One particle slot (the game's 0x60-byte record).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Particle {
    pub pos: Vec3,
    pub dir: Vec3,
    pub anim_time: u16,
    pub anim_step: u16,
    /// The step drawn.
    pub anim_drawn: u16,
    /// Ticks left; 0 = dead slot, 0xFFFF = endless.
    pub life: u16,
    /// Random colour offsets (signed bytes).
    pub col: [i8; 4],
    pub age: u8,
    /// Spin angle and velocity, 65536 = one turn.
    pub angle: i16,
    pub spin: i16,
    pub spin_rnd: i16,
    pub scale: [f32; 2],
    pub scale_vel: [f32; 2],
    pub scale_rnd: [f32; 2],
    pub speed: f32,
    pub drift: f32,
    /// Subtracted from the alpha (fade in / out).
    pub alpha_sub: f32,
    /// Grow-in / shrink-out size factor.
    pub size: f32,
    pub fade_in: u8,
    pub fade_out: u8,
    pub grow: u8,
    pub shrink: u8,
}

/// The program's state (allocated on its first run).
#[derive(Clone, Debug)]
pub(crate) struct MegaBirth {
    /// Base matrix, refreshed every tick: the effect root, or only its
    /// position and axis lengths for the "position only" emission types.
    pub base: Mat4,
    pub drift_dir: Vec3,
    pub slots: Vec<Particle>,
    /// Matrix remembered at birth (matrix modes 1-3).
    pub birth: Option<Vec<Mat4>>,
    /// Per-particle colour offset and velocity (key 0xC9).
    pub colvel: Option<Vec<[f32; 8]>>,
    pub counter: u16,
    pub vcursor: u16,
}

impl MegaBirth {
    pub fn new(key: &Key) -> MegaBirth {
        let n = key.u16(0x1c) as usize;
        let mode = key.u8(0xcc);
        MegaBirth {
            base: Mat4::IDENTITY,
            drift_dir: norm_or_zero(Vec3::from(key.vec3(0x98))),
            slots: vec![Particle::default(); n],
            birth: (1..=3).contains(&mode).then(|| vec![Mat4::ZERO; n]),
            colvel: (key.u8(0xc9) != 0).then(|| vec![[0.0; 8]; n]),
            counter: 0,
            vcursor: 0,
        }
    }

    /// The calc part: refresh the base matrix, then (not on the repeat
    /// frame) count, birth dead slots and step live ones.
    #[allow(clippy::too_many_arguments)]
    pub fn calc(&mut self, lib: &Library, dpd: usize, rng: &mut Rng, fx: &EffectView, key: &Key, m: Mat4, colour: [u8; 4], repeat: bool) {
        let typ = key.u8(0x26);
        self.base = if matches!(typ, 1 | 3 | 5 | 7 | 9) {
            Mat4::from_cols(
                glam::Vec4::X * fx.rs.x,
                glam::Vec4::Y * fx.rs.y,
                glam::Vec4::Z * fx.rs.z,
                fx.root.w_axis.truncate().extend(1.0),
            )
        } else {
            fx.root
        };
        if repeat || key.i32(4) == 0xffff {
            return;
        }
        self.counter = self.counter.wrapping_add(1);
        let shape = lib.dpds[dpd].shape(key.i32(4));
        let mut born = 0u16;
        for i in 0..self.slots.len() {
            if self.slots[i].life == 0 {
                if key.u16(0x20) <= self.counter && born < key.u16(0x1e) {
                    self.birth(lib, dpd, rng, key, i, m, colour[3]);
                    born += 1;
                }
            } else {
                self.update(key, i, colour[3]);
                if let Some(sh) = shape {
                    let p = &mut self.slots[i];
                    let (t, s, d) = anim_step(sh, p.anim_time, p.anim_step, p.anim_drawn, key.i16(8));
                    p.anim_time = t;
                    p.anim_step = s;
                    p.anim_drawn = d;
                }
            }
        }
        if born > 0 {
            self.counter = 0;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn birth(&mut self, lib: &Library, dpd: usize, rng: &mut Rng, key: &Key, i: usize, m: Mat4, alpha: u8) {
        let cone = key.u8(0x27) as f32;
        let mut p = Particle::default();
        if let Some(b) = &mut self.birth {
            b[i] = Mat4::ZERO;
        }
        if let Some(c) = &mut self.colvel {
            c[i] = [0.0; 8];
        }
        let typ = key.u8(0x26);
        let mode = key.u8(0xc8);
        let axis = Vec3::from(key.vec3(0xbc));
        // Direction: a random rotation within the cone (degrees) of the base
        // direction; the rotation applies Z, then Y, then X.
        if typ < 8 {
            let mut a = [0.0f32; 3];
            for v in a.iter_mut() {
                *v = (rng.rand01() * (cone + cone) - cone) * PI / 180.0;
            }
            if typ == 2 || typ == 3 {
                a[0] = 0.0;
                a[1] = 0.0;
            }
            let r = math::rot_x(a[0]) * math::rot_y(a[1]) * math::rot_z(a[2]);
            p.dir = norm_or_zero(r * Vec3::from(key.vec3(0x8c)) * axis);
        }
        let radius = key.f32(0xb8);
        if !(4..=9).contains(&typ) {
            if radius != 0.0 {
                let r = match mode {
                    1 => {
                        rng.rand01();
                        rng.rand01() * radius
                    }
                    2 => {
                        let a = rng.rand01();
                        let b = rng.rand01();
                        a * radius * b
                    }
                    3 => {
                        let a = rng.rand01();
                        let b = rng.rand01();
                        radius - radius * a * b * 0.7
                    }
                    4 => {
                        let a = rng.rand01();
                        let b = rng.rand01();
                        let c = rng.rand01();
                        let d = rng.rand01();
                        a * radius * b * c * d
                    }
                    5 => {
                        let a = rng.rand01();
                        let b = rng.rand01();
                        let c = rng.rand01();
                        radius - radius * a * b * c * 0.5
                    }
                    _ => radius,
                };
                p.pos = p.dir * r;
            }
        } else if typ < 6 {
            if radius != 0.0 {
                let half = radius * 0.5;
                if mode == 1 {
                    rng.rand01();
                }
                let mut v = [0.0f32; 3];
                for c in v.iter_mut() {
                    *c = match mode {
                        2 => {
                            let a = rng.rand01();
                            let b = rng.rand01();
                            a * radius * b
                        }
                        3 => {
                            let a = rng.rand01();
                            let b = rng.rand01();
                            radius - radius * a * b * 0.7
                        }
                        4 => {
                            let a = rng.rand01();
                            let b = rng.rand01();
                            let c = rng.rand01();
                            let d = rng.rand01();
                            a * radius * b * c * d
                        }
                        5 => {
                            let a = rng.rand01();
                            let b = rng.rand01();
                            let c = rng.rand01();
                            radius - radius * a * b * c * 0.5
                        }
                        _ => rng.rand01() * radius,
                    } - half;
                }
                p.pos = Vec3::from(v) * axis;
            }
        } else if key.i16(0xd0) >= 0 {
            // Types 6-9: a vertex of the vertex set's model.
            let d = &lib.dpds[dpd];
            if let Some(vs) = d.vertex_set(key.i16(0xd0) as i32) {
                let n = vs.indices.len();
                let u = match mode {
                    1 => {
                        rng.rand01();
                        Some(rng.rand01())
                    }
                    2 => {
                        let a = rng.rand01();
                        let b = rng.rand01();
                        let c = rng.rand01();
                        Some(c * b * a)
                    }
                    3 => {
                        let a = rng.rand01();
                        let b = rng.rand01();
                        let c = rng.rand01();
                        Some(1.0 - c * b * a)
                    }
                    4 => {
                        let a = rng.rand01();
                        let b = rng.rand01();
                        let c = rng.rand01();
                        let dd = rng.rand01();
                        Some(dd * b * a * c)
                    }
                    5 => {
                        let a = rng.rand01();
                        let b = rng.rand01();
                        let c = rng.rand01();
                        let dd = rng.rand01();
                        let e = rng.rand01();
                        Some(1.0 - e * b * a * c * dd)
                    }
                    _ => None,
                };
                if n as u16 <= self.vcursor {
                    self.vcursor = 0;
                }
                let idx = match u {
                    Some(u) => (n as f32 * u) as usize,
                    None => {
                        let c = self.vcursor as usize;
                        self.vcursor += 1;
                        c
                    }
                };
                let v = d
                    .model(vs.model)
                    .and_then(|m| vs.indices.get(idx).and_then(|&k| m.vertices.get(k as usize)))
                    .copied()
                    .unwrap_or([0; 3]);
                p.pos = Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32) * axis;
                if typ >= 8 {
                    p.dir = norm_or_zero(p.pos);
                }
            }
        }
        // Colour offsets: asymmetric and wrapping, as the game computes them.
        for (c, o) in p.col.iter_mut().enumerate() {
            let b = key.u8(0x48 + c);
            *o = (rng.rand01() * (b as u32 * 2) as f32 - (b >> 1) as f32) as i32 as i8;
        }
        if key.u8(0x24) != 0 {
            p.alpha_sub = alpha as f32;
            p.fade_in = key.u8(0x24);
        }
        p.fade_out = key.u8(0x25);
        p.size = 1.0;
        if key.u8(0xd3) != 0 {
            p.size = 0.0;
            p.grow = key.u8(0xd3);
        }
        p.shrink = key.u8(0xd4);
        let deg = |d: f32| (d * 32768.0 / 180.0) as i32 as i16;
        p.angle = deg(key.f32(0x7c));
        p.spin = deg(key.f32(0x80));
        let sf = key.u8(0xcb);
        if sf != 0 {
            let s = deg(rng.rand01() * key.f32(0x88));
            p.spin_rnd = s;
            if sf & 3 == 3 {
                if rng.rand01() > 0.5 {
                    p.spin_rnd = s.wrapping_neg();
                }
            } else if sf & 2 != 0 {
                p.spin_rnd = s.wrapping_neg();
            }
        }
        if sf & 4 != 0 {
            p.angle = p.angle.wrapping_add(p.spin_rnd);
        }
        if sf & 8 != 0 {
            p.spin = p.spin.wrapping_add(p.spin_rnd);
        }
        p.scale = [key.f32(0x4c), key.f32(0x50)];
        p.scale_vel = [key.f32(0x58), key.f32(0x5c)];
        let cf = key.u8(0xca);
        if cf != 0 {
            let rx = key.f32(0x70) * rng.rand01();
            p.scale_rnd[0] = rx;
            if cf & 0x20 == 0 {
                p.scale_rnd[1] = rng.rand01() * key.f32(0x74);
                if cf & 3 == 3 {
                    if rng.rand01() > 0.5 {
                        p.scale_rnd[0] = -p.scale_rnd[0];
                    }
                    if rng.rand01() > 0.5 {
                        p.scale_rnd[1] = -p.scale_rnd[1];
                    }
                } else if cf & 2 != 0 {
                    p.scale_rnd = [-p.scale_rnd[0], -p.scale_rnd[1]];
                }
            } else {
                p.scale_rnd[1] = rx;
                if cf & 3 == 3 {
                    if rng.rand01() > 0.5 {
                        p.scale_rnd = [-rx, -rx];
                    }
                } else if cf & 2 != 0 {
                    p.scale_rnd = [-rx, -rx];
                }
            }
        }
        if cf & 4 != 0 {
            p.scale[0] += p.scale_rnd[0];
            p.scale[1] += p.scale_rnd[1];
        }
        if cf & 8 != 0 {
            p.scale_vel[0] += p.scale_rnd[0];
            p.scale_vel[1] += p.scale_rnd[1];
        }
        // Random mirroring of the sprite.
        match key.u8(0xd2) {
            1 => {
                if rng.rand01() > 0.5 {
                    p.scale[0] = -p.scale[0];
                }
            }
            2 => {
                if rng.rand01() > 0.5 {
                    p.scale[1] = -p.scale[1];
                }
            }
            3 => {
                if rng.rand01() > 0.5 {
                    p.scale[0] = -p.scale[0];
                }
                if rng.rand01() > 0.5 {
                    p.scale[1] = -p.scale[1];
                }
            }
            _ => {}
        }
        p.speed = key.f32(0xa4);
        p.drift = key.f32(0xb0);
        let r = key.f32(0xac);
        if r != 0.0 {
            p.speed += (r + r) * rng.rand01() - r;
        }
        let r = key.f32(0xd8);
        if r != 0.0 {
            p.drift += (r + r) * rng.rand01() - r;
        }
        p.life = match key.u16(0x22) {
            0 => 0xffff,
            l => l,
        };
        p.age = 0;
        if let Some(b) = &mut self.birth {
            b[i] = match key.u8(0xcc) {
                1 => self.base,
                2 => m,
                _ => self.base * m,
            };
        }
        if let Some(c) = &mut self.colvel {
            for k in 0..4 {
                c[i][4 + k] = key.f32(0x28 + k * 4);
            }
        }
        self.slots[i] = p;
    }

    fn update(&mut self, key: &Key, i: usize, inst_alpha: u8) {
        let mut alpha = inst_alpha as i32;
        if let Some(cv) = &mut self.colvel {
            let c = &mut cv[i];
            for k in 0..4 {
                c[k] += c[4 + k];
                c[4 + k] += key.f32(0x38 + k * 4);
            }
            alpha = (c[3] as i32 + inst_alpha as i32).min(255);
        }
        let drift_dir = self.drift_dir;
        let p = &mut self.slots[i];
        p.angle = p.angle.wrapping_add(p.spin);
        let sa = if key.u8(0xcb) & 0x10 != 0 { p.spin_rnd as f32 + key.f32(0x84) } else { key.f32(0x84) };
        p.spin = (p.spin as f32 + sa) as i32 as i16;
        let sv0 = p.scale_vel[0];
        p.scale[0] += sv0;
        p.scale[1] += p.scale_vel[1];
        if key.u8(0xca) & 0x10 == 0 {
            p.scale_vel[0] = sv0 + key.f32(0x64);
            p.scale_vel[1] += key.f32(0x68);
        } else {
            p.scale_vel[0] = p.scale_rnd[0] + key.f32(0x64) + sv0;
            p.scale_vel[1] += p.scale_rnd[1] + key.f32(0x68);
        }
        let mut speed = p.speed + key.f32(0xa8);
        if key.u8(0xce) == 0 {
            let (s0, acc) = (key.f32(0xa4), key.f32(0xa8));
            // The speed stops at zero instead of changing sign.
            let flipped = (s0 > 0.0 && acc < 0.0 && speed < 0.0) || (s0 < 0.0 && acc > 0.0 && speed > 0.0);
            if flipped {
                speed = 0.0;
            }
        }
        p.speed = speed;
        p.drift += key.f32(0xb4);
        p.pos += p.dir * speed;
        p.pos += drift_dir * p.drift;
        if key.u16(0x22) != 0 {
            p.life = p.life.wrapping_sub(1);
        }
        p.age = p.age.wrapping_add(1);
        if p.fade_in != 0 && p.age <= p.fade_in {
            p.alpha_sub -= alpha as f32 / p.fade_in as f32;
        }
        let fo = key.u8(0x25);
        if p.fade_out != 0 && p.life <= p.fade_out as u16 && fo != 0 {
            p.alpha_sub += alpha as f32 / fo as f32;
        }
        if p.grow != 0 && p.age <= p.grow {
            p.size = p.age as f32 / p.grow as f32;
        }
        if p.shrink != 0 && p.life <= p.shrink as u16 {
            p.size = p.life as f32 / p.shrink as f32;
        }
    }

    /// Particle colour bytes for drawing: instance colour plus the random
    /// offsets (and velocity arrays), alpha minus the fade; rgb 0..255,
    /// alpha 0..127.
    pub fn colour(&self, i: usize, inst: [u8; 4]) -> [u8; 4] {
        let p = &self.slots[i];
        let mut c = [
            p.col[0] as i32 + inst[0] as i32,
            p.col[1] as i32 + inst[1] as i32,
            p.col[2] as i32 + inst[2] as i32,
            (p.col[3] as f32 + inst[3] as f32 - p.alpha_sub) as i32,
        ];
        if let Some(cv) = &self.colvel {
            for (k, v) in c.iter_mut().enumerate() {
                *v += cv[i][k] as i32;
            }
        }
        [c[0].clamp(0, 255) as u8, c[1].clamp(0, 255) as u8, c[2].clamp(0, 255) as u8, c[3].clamp(0, 127) as u8]
    }

    /// The particle's matrix in view space: the sprite plane rotated by
    /// the spin, scaled by `rs · scale · size`, at the particle's view
    /// position for the key's matrix mode.
    pub fn view_matrix(&self, i: usize, key: &Key, view: Mat4, m: Mat4, rs: Vec3) -> Option<Mat4> {
        let p = &self.slots[i];
        let mut sx = rs.x * p.scale[0];
        let mut sy = rs.y * p.scale[1];
        if p.size < 1.0 {
            sx *= p.size;
            sy *= p.size;
        }
        let s = Mat4::from_scale(Vec3::new(sx, sy, sx));
        let (sn, cs) = (math::sin_i(p.angle as u16 as i32), math::cos_i(p.angle as i32));
        let r = Mat4::from_cols(
            glam::Vec4::new(cs, sn, 0.0, 0.0),
            glam::Vec4::new(-sn, cs, 0.0, 0.0),
            glam::Vec4::Z,
            glam::Vec4::W,
        );
        // Rotate in the sprite plane first, then scale.
        let mut out = if p.angle != 0 { s * r } else { s };
        let place = match key.u8(0xcc) {
            0 => view * self.base * m,
            1 | 3 => view * self.birth.as_ref()?[i] * m,
            2 => view * self.base * self.birth.as_ref()?[i],
            _ => return None,
        };
        out.w_axis = place.transform_point3(p.pos).extend(1.0);
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(bytes: &[(usize, &[u8])]) -> Key {
        let mut data = vec![0u8; 220];
        for (at, b) in bytes {
            data[*at..*at + b.len()].copy_from_slice(b);
        }
        Key { data }
    }

    #[test]
    fn update_moves_and_fades() {
        let k = key(&[
            (0x1c, &4u16.to_le_bytes()),
            (0x22, &10u16.to_le_bytes()),
            (0x25, &[2]),
            (0xa8, &(-1.0f32).to_le_bytes()),
            (0xa4, &3.0f32.to_le_bytes()),
        ]);
        let mut mb = MegaBirth::new(&k);
        mb.slots[0] = Particle { dir: Vec3::Y, speed: 3.0, life: 5, fade_out: 2, size: 1.0, ..Default::default() };
        mb.update(&k, 0, 100);
        assert_eq!(mb.slots[0].pos, Vec3::new(0.0, 2.0, 0.0));
        assert_eq!((mb.slots[0].life, mb.slots[0].alpha_sub), (4, 0.0));
        mb.update(&k, 0, 100);
        mb.update(&k, 0, 100);
        // Speed 3 - 1 - 1 - 1 reaches 0 and stays there instead of reversing.
        mb.update(&k, 0, 100);
        assert_eq!(mb.slots[0].speed, 0.0);
        assert_eq!(mb.slots[0].pos, Vec3::new(0.0, 3.0, 0.0));
        // The last two ticks of life fade 100 / 2 each.
        assert_eq!(mb.slots[0].alpha_sub, 100.0);
        assert_eq!(mb.colour(0, [128, 128, 128, 100]), [128, 128, 128, 0]);
    }

    #[test]
    fn view_matrix_modes() {
        let k = key(&[]);
        let mut mb = MegaBirth::new(&k);
        mb.base = Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0));
        let mut mb2 = mb.clone();
        mb2.slots = vec![Particle { pos: Vec3::new(0.0, 5.0, 0.0), scale: [2.0, 3.0], size: 1.0, life: 1, ..Default::default() }];
        let v = mb2.view_matrix(0, &k, Mat4::IDENTITY, Mat4::IDENTITY, Vec3::ONE).unwrap();
        assert_eq!(v.w_axis.truncate(), Vec3::new(10.0, 5.0, 0.0));
        assert_eq!(v.x_axis.x, 2.0);
        assert_eq!(v.y_axis.y, 3.0);
    }
}
