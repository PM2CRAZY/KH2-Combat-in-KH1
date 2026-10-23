//! pppEiDrawShipoly (type 241): a ribbon trail through the instance's
//! recent positions (`docs/kh2-effects.md` §7.4).
//!
//! Every tick the program records the instance position (or, in two-point
//! mode, two points `±width/2` along the instance's y axis) into a ring of
//! `length` entries. The history can expand away from its centre, drift,
//! or be pulled toward a point. At draw time a natural cubic spline is run
//! through the history (newest first) and sampled `subdivisions + 1` times
//! in view space; a strip `2 × (key 0x13 + 1)` vertices wide is laid along
//! it, textured with the first texture of a DPD model.
//!
//! Key fields used (offsets from the key start): @4 model, @9 mode (0 two
//! points, 1 single point with a screen-space width, 2+ single point with
//! a smoothed width direction), @10 freeze, @11 length, @12 width, @16
//! taper, @17 space, @18 subdivisions, @19 strip columns, @20 no depth
//! test, @21 blend, @22 fade across, @23/@24 fade along, @28 expand, @29
//! drift, @30 attract, @0x2C.. their key vectors, @0x8C attract range,
//! @0x90 length-based u, @0x94/@0x98 scroll, @0x9C.. texture region.

use glam::{Mat4, Vec3, Vec4};
use kh2_formats::dpd::Key;

use crate::player::EffectView;
use crate::rng::Rng;

#[derive(Clone, Debug)]
pub(crate) struct Shipoly {
    len: usize,
    head: usize,
    count: usize,
    two_point: bool,
    buf: Vec<Vec4>,
    m_copy: Mat4,
    expand: f32,
    expand_vel: f32,
    expand_acc: f32,
    drift: Vec4,
    drift_vel: Vec4,
    drift_acc: Vec4,
    attract: Vec4,
    attract_vel: Vec4,
    attract_acc: Vec4,
    strength: f32,
    strength_vel: f32,
    strength_acc: f32,
    /// Texture scroll, wrapped to 0..1.
    pub scroll_u: f32,
    pub scroll_v: f32,
}

fn kv(key: &Key, at: usize) -> Vec4 {
    Vec3::from(key.vec3(at)).extend(0.0)
}

impl Shipoly {
    pub fn new(key: &Key, rng: &mut Rng) -> Shipoly {
        let len = (key.u8(11) as usize).clamp(4, 128);
        let two_point = key.u8(9) == 0;
        let (mut su, mut sv) = (0.0, 0.0);
        match key.u8(0x9f) {
            1 => su = rng.rand01(),
            2 => sv = rng.rand01(),
            3 => {
                su = rng.rand01();
                sv = rng.rand01();
            }
            _ => {}
        }
        Shipoly {
            len,
            head: 0,
            count: 0,
            two_point,
            buf: vec![Vec4::ZERO; if two_point { len * 2 } else { len }],
            m_copy: Mat4::IDENTITY,
            expand: 0.0,
            expand_vel: 0.0,
            expand_acc: 0.0,
            drift: Vec4::ZERO,
            drift_vel: Vec4::ZERO,
            drift_acc: Vec4::ZERO,
            attract: Vec4::ZERO,
            attract_vel: Vec4::ZERO,
            attract_acc: Vec4::ZERO,
            strength: 0.0,
            strength_vel: 0.0,
            strength_acc: 0.0,
            scroll_u: su,
            scroll_v: sv,
        }
    }

    /// Recorded points over the ring length.
    pub fn fill_ratio(&self) -> f32 {
        self.count as f32 / self.len as f32
    }

    pub fn reset(&mut self) {
        self.head = 0;
        self.count = 0;
        self.expand_vel = 0.0;
        self.expand_acc = 0.0;
        self.drift_vel = Vec4::ZERO;
        self.drift_acc = Vec4::ZERO;
        self.attract_vel = Vec4::ZERO;
        self.attract_acc = Vec4::ZERO;
        self.strength_vel = 0.0;
        self.strength_acc = 0.0;
    }

    fn idx(&self, back: usize) -> usize {
        (self.head + self.len * 2 - back % self.len) % self.len
    }

    /// Record this tick (the program's calc; runs once per tick).
    pub fn record(&mut self, key: &Key, frame: i32, m: Mat4, fx: &EffectView) {
        let on_key = key.frame() == frame;
        let frozen = key.u8(10) != 0;
        let world = key.u8(0x11) != 0;
        if !frozen {
            self.m_copy = m;
            if self.two_point {
                let w = key.f32(12);
                let mut a = self.m_copy * Vec4::new(0.0, w * 0.5, 0.0, 1.0);
                let mut b = self.m_copy * Vec4::new(0.0, w * -0.5, 0.0, 1.0);
                if world {
                    a = fx.root * a;
                    b = fx.root * b;
                }
                self.buf[self.head * 2] = a;
                self.buf[self.head * 2 + 1] = b;
            } else {
                let mut p = self.m_copy.w_axis;
                if world {
                    p = fx.root * p;
                }
                if key.u8(0x1c) != 0 {
                    p.w = self.expand;
                }
                self.buf[self.head] = p;
            }
            self.count = (self.count + 1).min(self.len);
        }
        if !self.two_point && key.u8(0x1c) != 0 {
            // Each point moves away from the centre of the history at the
            // speed it was recorded with.
            let n = self.count.max(1);
            let avg = (0..self.count).map(|i| self.buf[self.idx(i)]).fold(Vec4::ZERO, |a, b| a + b) / n as f32;
            for i in 1..self.count.saturating_sub(2) {
                let k = self.idx(i);
                let p = self.buf[k];
                let d = (p - avg).truncate().normalize_or_zero() * p.w;
                self.buf[k] = (p.truncate() + d).extend(p.w);
            }
            self.expand_vel += self.expand_acc;
            self.expand += self.expand_vel;
            if on_key {
                self.expand += key.f32(0x2c);
                self.expand_vel += key.f32(0x30);
                self.expand_acc += key.f32(0x34);
            }
        }
        if !self.two_point && key.u8(0x1d) != 0 {
            for i in 1..self.count {
                let k = self.idx(i);
                self.buf[k] += self.drift;
            }
            self.drift_vel += self.drift_acc;
            self.drift += self.drift_vel;
            if on_key {
                self.drift += kv(key, 0x38);
                self.drift_vel += kv(key, 0x44);
                self.drift_acc += kv(key, 0x50);
            }
        }
        if !self.two_point && key.u8(0x1e) != 0 {
            let range = key.f32(0x8c);
            for i in 1..self.count {
                let k = self.idx(i);
                let target = if key.u8(0x1e) == 2 { self.attract } else { self.m_copy * self.attract.truncate().extend(1.0) };
                let d = target - self.buf[k];
                let f = if range != 0.0 { (d.truncate().length() / range).clamp(0.0, 1.0) } else { 1.0 };
                let d = (d * ((1.0 - f) * self.strength)).truncate().extend(0.0);
                self.buf[k] += d;
            }
            self.attract_vel += self.attract_acc;
            self.attract += self.attract_vel;
            self.strength_vel += self.strength_acc;
            self.strength += self.strength_vel;
            if on_key {
                self.attract += kv(key, 0x5c);
                self.attract_vel += kv(key, 0x68);
                self.attract_acc += kv(key, 0x74);
                self.strength += key.f32(0x80);
                self.strength_vel += key.f32(0x84);
                self.strength_acc += key.f32(0x88);
            }
        }
        if !frozen {
            self.head = (self.head + 1) % self.len;
        }
    }

    /// Texture scroll after a drawn tick.
    pub fn scroll(&mut self, key: &Key) {
        let wrap = |v: f32| {
            let mut v = v;
            while v > 1.0 {
                v -= 1.0;
            }
            while v < 0.0 {
                v += 1.0;
            }
            v
        };
        self.scroll_v = wrap(self.scroll_v + key.f32(0x98));
        self.scroll_u = wrap(self.scroll_u + key.f32(0x94));
    }

    /// The two edges of the ribbon in view space, `subdivisions + 1`
    /// samples each, newest first; `w` carries the sample's length weight.
    /// `space` maps the recorded points to view space; `half_width` is the
    /// width the single-point modes use (key width × instance scale x ×
    /// root scale x).
    pub fn edges(&self, key: &Key, space: Mat4, half_width: f32) -> Option<(Vec<Vec4>, Vec<Vec4>)> {
        if self.count < 3 {
            return None;
        }
        let sub = key.u8(0x12) as usize;
        if sub == 0 {
            return None;
        }
        let n = self.count;
        let at = |i: usize| i as f32 * (n - 1) as f32 / sub as f32;
        if self.two_point {
            let side = |s: usize| -> Vec<Vec4> {
                let pts: Vec<Vec4> = (0..n).map(|j| space * self.buf[self.idx(j + 1) * 2 + s].truncate().extend(1.0)).collect();
                let y2 = spline(&pts);
                (0..=sub).map(|i| eval(&pts, &y2, at(i)).truncate().extend(1.0)).collect()
            };
            return Some(self.taper(key, side(0), side(1)));
        }
        let pts: Vec<Vec4> = (0..n).map(|j| self.buf[self.idx(j + 1)].truncate().extend(1.0)).collect();
        let y2 = spline(&pts);
        let mut s: Vec<Vec4> = (0..=sub).map(|i| (space * eval(&pts, &y2, at(i)).truncate().extend(1.0)).truncate().extend(1.0)).collect();
        if key.f32(0x90) > 0.0 {
            for i in 0..sub {
                s[i].w = (s[i] - s[i + 1]).truncate().length();
            }
            s[sub].w = 0.0;
        }
        let mut a = Vec::with_capacity(s.len());
        let mut b = Vec::with_capacity(s.len());
        let mut prev = Vec3::ZERO;
        for i in 0..=sub {
            let cur = s[i];
            let d = if i != sub { cur - s[i + 1] } else { s[i - 1] - cur };
            let p = if key.u8(9) == 1 {
                let d = d.truncate().normalize_or_zero() * half_width;
                Vec3::new(-d.y, d.x, 0.0)
            } else {
                // Smoothed: the sum of this and the previous direction.
                let d = if i == sub { Vec3::ZERO } else { Vec3::new(d.x, d.y, 0.0).normalize_or_zero() * half_width };
                let sum = prev + d;
                prev = d;
                Vec3::new(-sum.y, sum.x, 0.0)
            };
            a.push((cur.truncate() + p).truncate().extend(cur.z).extend(cur.w));
            b.push((cur.truncate() - p).truncate().extend(cur.z).extend(cur.w));
        }
        Some(self.taper(key, a, b))
    }

    /// Taper: the newest end narrows to the middle line.
    fn taper(&self, key: &Key, mut a: Vec<Vec4>, mut b: Vec<Vec4>) -> (Vec<Vec4>, Vec<Vec4>) {
        if key.u8(0x10) != 0 {
            let sub = key.u8(0x12).max(1) as f32;
            let mut t = 1.0f32;
            for i in 0..a.len() {
                let mid = (a[i] + b[i]) * 0.5;
                a[i] = a[i].lerp(mid, t);
                b[i] = b[i].lerp(mid, t);
                t = (t - 1.0 / sub).max(0.0);
            }
        }
        (a, b)
    }
}

/// Second derivatives of a natural cubic spline through `p` (uniform
/// parameter).
fn spline(p: &[Vec4]) -> Vec<Vec4> {
    let n = p.len();
    let mut y2 = vec![Vec4::ZERO; n];
    let mut u = vec![Vec4::ZERO; n];
    for i in 1..n.saturating_sub(1) {
        let pp = y2[i - 1] * 0.5 + Vec4::splat(2.0);
        y2[i] = Vec4::splat(-0.5) / pp;
        let d = (p[i + 1] - p[i]) - (p[i] - p[i - 1]);
        u[i] = (d * 3.0 - u[i - 1] * 0.5) / pp;
    }
    for k in (0..n.saturating_sub(1)).rev() {
        y2[k] = y2[k] * y2[k + 1] + u[k];
    }
    y2
}

fn eval(p: &[Vec4], y2: &[Vec4], t: f32) -> Vec4 {
    let n = p.len();
    let i = (t.max(0.0) as usize).min(n - 2);
    let f = t - i as f32;
    let (a, b) = (1.0 - f, f);
    p[i] * a + p[i + 1] * b + (y2[i] * (a * a * a - a) + y2[i + 1] * (b * b * b - b)) / 6.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spline_passes_through_points() {
        let p: Vec<Vec4> = [0.0f32, 1.0, 4.0, 9.0, 16.0].iter().map(|&y| Vec4::new(0.0, y, 0.0, 1.0)).collect();
        let y2 = spline(&p);
        for (i, q) in p.iter().enumerate() {
            assert!((eval(&p, &y2, i as f32) - *q).length() < 1e-4);
        }
        // Between points it is smooth and monotone here.
        let mid = eval(&p, &y2, 2.5).y;
        assert!(mid > 4.0 && mid < 9.0);
    }

    fn key(bytes: &[(usize, &[u8])]) -> Key {
        let mut data = vec![0u8; 0xa4];
        for (at, b) in bytes {
            data[*at..*at + b.len()].copy_from_slice(b);
        }
        Key { data }
    }

    #[test]
    fn records_and_builds_edges() {
        let k = key(&[(9, &[1]), (11, &[6]), (12, &10.0f32.to_le_bytes()), (0x12, &[8])]);
        let mut rng = Rng::new(1);
        let mut s = Shipoly::new(&k, &mut rng);
        let fx = EffectView { root: Mat4::IDENTITY, rs: Vec3::ONE, colour: [1.0; 4], view: Mat4::IDENTITY };
        for t in 0..10 {
            let m = Mat4::from_translation(Vec3::new(t as f32 * 10.0, 0.0, 100.0));
            s.record(&k, t * 4096, m, &fx);
        }
        assert_eq!(s.count, 6);
        let (a, b) = s.edges(&k, Mat4::IDENTITY, 5.0).unwrap();
        assert_eq!(a.len(), 9);
        // Newest first: x = 90 down to 40, edges 5 above and below.
        assert!((a[0].x - 90.0).abs() < 1e-3 && (a[8].x - 40.0).abs() < 1e-3);
        assert!(((a[0].y - b[0].y).abs() - 10.0).abs() < 1e-3);
        assert!((a[4].z - 100.0).abs() < 1e-3);
    }
}
