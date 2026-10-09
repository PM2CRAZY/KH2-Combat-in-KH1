//! Moving a character against level geometry the way KH2FM does it.
//!
//! The rules come from plain-language notes on the game's background
//! collision solver (see `docs/movement-model.md`). In short:
//! - The character is **one sphere** (Sora: radius 90, centre 115 above the
//!   feet). It is swept through the move in sub-steps and pushed out of
//!   polygons: faces first, then edges and corners.
//! - Characters whose shape is "map" type add a **lower sphere** that only
//!   meets wall-type polygons, so low walls still stop them.
//! - A **floor ray** then finds the ground under the result and snaps the
//!   feet onto it. A grounded character keeps its footing on floors at most
//!   `h − hh − 1` (24) below; anything lower starts a fall.
//!
//! Everything else follows from the geometry: step-up height is `h − r` (25),
//! floor-type slopes steeper than `acos(r / h)` (38.5°) push the sphere and
//! slide, and a rise that the sphere can't make counts as a ceiling.
//!
//! The game's y axis points down; this crate is **y-up**. Floors have
//! normals with positive y.

use glam::Vec3;

/// Polygon attribute bits (the low nibble is a surface type).
pub mod attr {
    /// Walls: meet the main sphere and the lower sphere.
    pub const WALL: u32 = 0;
    /// Floors: only the main sphere meets them.
    pub const FLOOR: u32 = 1;
    /// Can't be landed on from the air (still ground while walking).
    pub const NO_LANDING: u32 = 2;
    /// Ignored by every query.
    pub const IGNORED: u32 = 4;
    /// The player and party pass through it.
    pub const PLAYER_PASS: u32 = 0x10;
    /// Only the floor ray sees it; the sphere sweep doesn't.
    pub const RAY_ONLY: u32 = 0x10_0000;
}

/// Skin added to the radius on every push (exe).
pub const SKIN: f32 = 0.008;
/// Point-in-polygon tolerance (exe).
pub const INSIDE_TOLERANCE: f32 = 0.01;
/// Push-out passes per sub-step (exe).
pub const MAX_PASSES: usize = 6;
/// More pushes than this in one pass (grounded) means stuck (exe).
pub const STUCK_PUSHES: i32 = 9;
/// Largest extra ray-start height taken from the vertical move (exe).
pub const RAY_LIFT_MAX: f32 = 100.0;
/// Ray length below the feet, in centre heights (exe).
pub const RAY_DEPTH: f32 = 3.0;
/// Below the plane by more than this counts as landed (exe).
pub const LAND_EPSILON: f32 = 0.001;
/// A rise reaching less than this fraction of the desired one means the
/// head hit a ceiling (03system sstm `ceiling_stop`).
pub const CEILING_STOP: f32 = 0.49;

/// A convex polygon (3 or 4 corners), counter-clockwise seen from its front.
#[derive(Clone, Debug, PartialEq)]
pub struct Poly {
    verts: [Vec3; 4],
    count: usize,
    pub normal: Vec3,
    /// Plane offset: `dist(p) = normal · p + d`.
    pub d: f32,
    pub attr: u32,
    pub min: Vec3,
    pub max: Vec3,
}

impl Poly {
    /// Builds a polygon from 3 or 4 corners. The front is the side from
    /// which they run counter-clockwise.
    pub fn new(corners: &[Vec3], attr: u32) -> Poly {
        assert!(corners.len() == 3 || corners.len() == 4, "polygons have 3 or 4 corners");
        let mut verts = [Vec3::ZERO; 4];
        verts[..corners.len()].copy_from_slice(corners);
        let mut n = Vec3::ZERO;
        // Newell's method: robust for quads that are slightly bent.
        for i in 0..corners.len() {
            let a = corners[i];
            let b = corners[(i + 1) % corners.len()];
            n += Vec3::new((a.y - b.y) * (a.z + b.z), (a.z - b.z) * (a.x + b.x), (a.x - b.x) * (a.y + b.y));
        }
        let normal = n.normalize_or_zero();
        let d = -normal.dot(corners[0]);
        let (mut min, mut max) = (corners[0], corners[0]);
        for c in corners {
            min = min.min(*c);
            max = max.max(*c);
        }
        Poly { verts, count: corners.len(), normal, d, attr, min, max }
    }

    pub fn corners(&self) -> &[Vec3] {
        &self.verts[..self.count]
    }

    pub fn surface(&self) -> u32 {
        self.attr & 0xF
    }

    pub fn dist(&self, p: Vec3) -> f32 {
        self.normal.dot(p) + self.d
    }

    /// The game's point-in-polygon test, for a point on (or near) the plane.
    pub fn contains(&self, p: Vec3) -> bool {
        let c = self.corners();
        (0..c.len()).all(|i| {
            let a = c[i];
            let b = c[(i + 1) % c.len()];
            (p - a).cross(b - a).dot(self.normal) <= INSIDE_TOLERANCE
        })
    }

    fn overlaps(&self, lo: Vec3, hi: Vec3) -> bool {
        self.min.cmple(hi).all() && self.max.cmpge(lo).all()
    }
}

/// The level: a soup of polygons.
#[derive(Clone, Debug, Default)]
pub struct Map {
    pub polys: Vec<Poly>,
}

/// A ray or segment hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub point: Vec3,
    pub poly: usize,
    /// Fraction along the segment.
    pub t: f32,
}

impl Map {
    pub fn new() -> Map {
        Map::default()
    }

    pub fn add(&mut self, corners: &[Vec3], attr: u32) -> usize {
        self.polys.push(Poly::new(corners, attr));
        self.polys.len() - 1
    }

    /// A solid box: floor-type top, wall-type sides and, when it floats, a
    /// wall-type underside (a ceiling).
    pub fn add_box(&mut self, min: Vec3, max: Vec3) {
        self.add_box_open(min, max, 0);
    }

    /// [`add_box`](Self::add_box) without some sides, where another shape
    /// covers them (a ramp leading up to it): bit 0 −z, 1 +z, 2 −x, 3 +x.
    pub fn add_box_open(&mut self, min: Vec3, max: Vec3, open: u8) {
        let (a, b) = (min, max);
        let c = |x: f32, y: f32, z: f32| Vec3::new(x, y, z);
        // Top (+y).
        self.add(&[c(a.x, b.y, a.z), c(a.x, b.y, b.z), c(b.x, b.y, b.z), c(b.x, b.y, a.z)], attr::FLOOR);
        // Sides.
        let sides = [
            [c(a.x, b.y, a.z), c(b.x, b.y, a.z), c(b.x, a.y, a.z), c(a.x, a.y, a.z)], // -z
            [c(b.x, b.y, b.z), c(a.x, b.y, b.z), c(a.x, a.y, b.z), c(b.x, a.y, b.z)], // +z
            [c(a.x, b.y, b.z), c(a.x, b.y, a.z), c(a.x, a.y, a.z), c(a.x, a.y, b.z)], // -x
            [c(b.x, b.y, a.z), c(b.x, b.y, b.z), c(b.x, a.y, b.z), c(b.x, a.y, a.z)], // +x
        ];
        for (i, s) in sides.iter().enumerate() {
            if open & (1 << i) == 0 {
                self.add(s, attr::WALL);
            }
        }
        if a.y > 0.0 {
            self.add(&[c(b.x, a.y, a.z), c(b.x, a.y, b.z), c(a.x, a.y, b.z), c(a.x, a.y, a.z)], attr::WALL);
        }
    }

    fn usable(p: &Poly, mask: u32) -> bool {
        p.surface() != attr::IGNORED && p.attr & mask == 0
    }

    /// First front face the segment `a`→`b` crosses.
    pub fn segment(&self, a: Vec3, b: Vec3, mask: u32) -> Option<Hit> {
        let d = b - a;
        let (lo, hi) = (a.min(b), a.max(b));
        let mut best: Option<Hit> = None;
        for (i, p) in self.polys.iter().enumerate() {
            if !Self::usable(p, mask) || !p.overlaps(lo, hi) {
                continue;
            }
            let dn = p.normal.dot(d);
            if dn >= 0.0 {
                continue;
            }
            let t = -p.dist(a) / dn;
            if !(0.0..=1.0).contains(&t) || best.is_some_and(|h| h.t <= t) {
                continue;
            }
            let q = a + d * t;
            if p.contains(q) {
                best = Some(Hit { point: q, poly: i, t });
            }
        }
        best
    }

    /// Height of the highest floor under `p` that is no higher than
    /// `p.y + up`, or `None`.
    pub fn ground(&self, p: Vec3, up: f32, mask: u32) -> Option<Hit> {
        self.segment(p + Vec3::Y * up, p - Vec3::Y * 100_000.0, mask)
            .filter(|h| h.point.y <= p.y + up)
    }
}

/// The shape a character moves with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    /// Sphere radius `r`.
    pub radius: f32,
    /// The shape's "height" field `hh` (sets the step-down reach and the
    /// lower sphere's air offsets).
    pub half_height: f32,
    /// Sphere centre above the feet, `h`.
    pub center: f32,
    /// Adds the lower sphere against walls (shape type "map").
    pub lower_sphere: bool,
    /// Attribute bits this body passes through ([`attr::PLAYER_PASS`] for
    /// the player).
    pub pass: u32,
    /// Extra step-down reach (30 on moving platforms in the game, else 0).
    pub snap: f32,
}

impl Body {
    /// Sora's type-4 shape from P_EX100.mdlx (radius 90, height 90, centre
    /// 115): a body built from those three numbers.
    pub fn player(radius: f32, half_height: f32, center: f32) -> Body {
        Body { radius, half_height, center, lower_sphere: true, pass: attr::PLAYER_PASS, snap: 0.0 }
    }

    /// How far below the start a floor may be and still hold the feet.
    pub fn step_down(&self) -> f32 {
        self.center - self.half_height - 1.0 + self.snap
    }
}

/// The result of one move.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moved {
    /// New feet position.
    pub pos: Vec3,
    /// Standing on ground (grounded rules), or touched down (air rules).
    pub ground: bool,
    /// The floor found under the result, if any.
    pub floor: Option<Hit>,
    /// The sphere was pushed by geometry at least once.
    pub wall: bool,
}

impl Moved {
    /// The ground's normal (up when there is none).
    pub fn ground_normal(&self, map: &Map) -> Vec3 {
        self.floor.map_or(Vec3::Y, |h| map.polys[h.poly].normal)
    }
}

/// Lays a grounded move onto the ground plane keeping its length (exe): the
/// along-slope speed equals the flat speed, and a vertical part turns into
/// along-slope motion.
pub fn along_ground(mv: Vec3, normal: Vec3) -> Vec3 {
    let len = mv.length();
    let a = normal.cross(mv).normalize_or_zero();
    let b = a.cross(normal).normalize_or_zero();
    b * len
}

/// True when a rise that reached `actual` of a desired `desired` (both
/// y-up) hit a ceiling (exe: less than 49%).
pub fn hit_ceiling(desired: f32, actual: f32) -> bool {
    desired > 0.0 && actual / desired < CEILING_STOP
}

enum Face {
    None,
    Pushed(Vec3),
    Edge,
}

impl Body {
    fn face(p: &Poly, c: Vec3, r: f32) -> Face {
        let dist = p.dist(c);
        if !(0.0..=r).contains(&dist) {
            return Face::None;
        }
        let q = c - p.normal * dist;
        if p.contains(q) {
            Face::Pushed(q + p.normal * (r + SKIN))
        } else {
            Face::Edge
        }
    }

    /// Nearest point of the polygon's outline within `r` of `c`: edges
    /// first, then corners.
    fn edge(p: &Poly, c: Vec3, r: f32) -> Option<Vec3> {
        let k = p.corners();
        let mut best: Option<(f32, Vec3)> = None;
        for i in 0..k.len() {
            let (a, b) = (k[i], k[(i + 1) % k.len()]);
            let e = b - a;
            let l = e.length();
            if l < 1e-6 {
                continue;
            }
            let s = (c - a).dot(e) / l;
            if s < -SKIN || s > l + SKIN {
                continue;
            }
            let q = a + e * (s / l);
            let d = q.distance(c);
            if d <= r && best.map_or(true, |(bd, _)| d < bd) {
                best = Some((d, q));
            }
        }
        if best.is_none() {
            for v in k {
                let d = v.distance(c);
                if d <= r && best.map_or(true, |(bd, _)| d < bd) {
                    best = Some((d, *v));
                }
            }
        }
        best.map(|(_, q)| q)
    }

    fn push_from(c: Vec3, q: Vec3, r: f32) -> Vec3 {
        let d = c - q;
        let n = if d.length_squared() > 1e-12 { d.normalize() } else { Vec3::Y };
        q + n * (r + SKIN)
    }

    /// One push-out pass. Returns the number of pushes, or -1 when stuck.
    fn pass(&self, map: &Map, cands: &[usize], c: &mut Vec3, air: bool) -> i32 {
        let r = self.radius;
        let mut pushes = 0;
        let mut edges = Vec::new();
        let lo_face = if air { self.center - 0.5 * self.half_height } else { r };
        for &i in cands {
            let p = &map.polys[i];
            if p.attr & attr::RAY_ONLY != 0 {
                continue;
            }
            let walls = self.lower_sphere && p.surface() != attr::FLOOR;
            if walls {
                let cl = *c - Vec3::Y * lo_face;
                if let Face::Pushed(n) = Self::face(p, cl, r) {
                    *c = n + Vec3::Y * lo_face;
                    pushes += 1;
                }
            }
            match Self::face(p, *c, r) {
                Face::Pushed(n) => {
                    *c = n;
                    pushes += 1;
                }
                Face::Edge => edges.push(i),
                Face::None => {}
            }
            if !air && pushes > STUCK_PUSHES {
                return -1;
            }
        }
        let lo_edge = self.center - 1.02 * self.half_height;
        for i in edges {
            let p = &map.polys[i];
            if air && self.lower_sphere && p.surface() != attr::FLOOR {
                let cl = *c - Vec3::Y * lo_edge;
                if let Some(q) = Self::edge(p, cl, r) {
                    *c = Self::push_from(cl, q, r) + Vec3::Y * lo_edge;
                    pushes += 1;
                }
            }
            if let Some(q) = Self::edge(p, *c, r) {
                *c = Self::push_from(*c, q, r);
                pushes += 1;
            }
            if !air && pushes > STUCK_PUSHES {
                return -1;
            }
        }
        pushes
    }

    /// Sweeps the sphere along `mv` from feet position `start`; returns the
    /// new feet position and whether anything pushed it.
    pub fn sweep(&self, map: &Map, start: Vec3, mv: Vec3, air: bool) -> (Vec3, bool) {
        let r = self.radius;
        let c0 = start + Vec3::Y * self.center;
        let c1 = c0 + mv;
        let pad = Vec3::splat(1.5 * r);
        let (lo, hi) = (c0.min(c1) - pad, c0.max(c1) + pad);
        let cands: Vec<usize> = map
            .polys
            .iter()
            .enumerate()
            .filter(|(_, p)| Map::usable(p, self.pass) && p.overlaps(lo, hi))
            .map(|(i, _)| i)
            .collect();
        let n = (mv.length() / r).floor() as usize + 1;
        let step = mv / n as f32;
        let mut c = c0;
        let mut wall = false;
        for _ in 0..n {
            let prev = c;
            c += step;
            let near: Vec<usize> = cands.iter().copied().filter(|&i| map.polys[i].overlaps(c - pad, c + pad)).collect();
            for _ in 0..MAX_PASSES {
                let k = self.pass(map, &near, &mut c, air);
                if k < 0 {
                    c = prev;
                    wall = true;
                    break;
                }
                if k == 0 {
                    break;
                }
                wall = true;
            }
        }
        // Anti-tunnelling: the centre's path must not cross a front face.
        if c != c0 && map.segment(c0, c, self.pass).is_some() {
            return (start, false);
        }
        (c - Vec3::Y * self.center, wall)
    }

    /// One frame's move: sweep, then the floor ray. `air` selects the air
    /// rules (no step-down reach; landing needs the feet to go below the
    /// floor). Grounded moves should already be laid on the ground with
    /// [`along_ground`].
    pub fn solve(&self, map: &Map, start: Vec3, mv: Vec3, air: bool) -> Moved {
        let (mut pos, wall) = self.sweep(map, start, mv, air);
        let h = self.center;
        let lift = mv.y.abs().min(RAY_LIFT_MAX);
        let from = pos + Vec3::Y * (h + lift);
        let to = pos - Vec3::Y * (RAY_DEPTH * h);
        let floor = map.segment(from, to, self.pass).filter(|f| map.polys[f.poly].normal.y > 0.0);
        let mut ground = false;
        if let Some(f) = floor {
            let p = &map.polys[f.poly];
            if air {
                if p.dist(pos) < -LAND_EPSILON {
                    pos.y = f.point.y;
                    ground = p.surface() != attr::NO_LANDING;
                }
            } else {
                let y0 = start.y + mv.y.min(0.0);
                if f.point.y >= y0 - self.step_down() {
                    pos.y = f.point.y;
                    ground = true;
                }
            }
        }
        Moved { pos, ground, floor, wall }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sora() -> Body {
        Body::player(90.0, 90.0, 115.0)
    }

    fn flat() -> Map {
        let mut m = Map::new();
        m.add(&[Vec3::new(-5000.0, 0.0, -5000.0), Vec3::new(-5000.0, 0.0, 5000.0), Vec3::new(5000.0, 0.0, 5000.0), Vec3::new(5000.0, 0.0, -5000.0)], attr::FLOOR);
        m
    }

    /// Walks along +z for `frames` at `speed`, the way the playground does.
    fn walk(map: &Map, body: &Body, mut pos: Vec3, speed: f32, frames: usize) -> Vec3 {
        let mut n = Vec3::Y;
        let mut air = false;
        for _ in 0..frames {
            let mv = if air { Vec3::new(0.0, -8.0, speed) } else { along_ground(Vec3::new(0.0, 0.0, speed), n) };
            let m = body.solve(map, pos, mv, air);
            pos = m.pos;
            air = !m.ground;
            n = m.ground_normal(map);
        }
        pos
    }

    #[test]
    fn floor_is_upward_and_contains() {
        let m = flat();
        assert!(m.polys[0].normal.y > 0.99);
        assert!(m.polys[0].contains(Vec3::new(10.0, 0.0, 10.0)));
        assert!(!m.polys[0].contains(Vec3::new(6000.0, 0.0, 10.0)));
        let mut b = Map::new();
        b.add_box(Vec3::new(0.0, 0.0, 0.0), Vec3::new(100.0, 50.0, 100.0));
        assert!(b.polys[1].normal.z < -0.99, "-z side faces -z");
        assert!(b.polys[4].normal.x > 0.99);
    }

    #[test]
    fn standing_still_stays_put() {
        let m = flat();
        let r = sora().solve(&m, Vec3::ZERO, Vec3::ZERO, false);
        assert!(r.ground && r.pos.distance(Vec3::ZERO) < 1e-4 && !r.wall);
    }

    #[test]
    fn steps_up_to_25_are_climbed_higher_ones_block() {
        for (riser, climbs) in [(20.0, true), (24.5, true), (25.5, false), (60.0, false)] {
            let mut m = flat();
            m.add_box(Vec3::new(-500.0, 0.0, 200.0), Vec3::new(500.0, riser, 800.0));
            let end = walk(&m, &sora(), Vec3::ZERO, 8.0, 80);
            if climbs {
                assert!((end.y - riser).abs() < 1e-3 && end.z > 300.0, "riser {riser}: {end}");
            } else {
                assert!(end.y.abs() < 1e-3 && end.z < 200.0, "riser {riser}: {end}");
            }
        }
    }

    #[test]
    fn stairs_of_20_are_walked_up() {
        // Treads must be deeper than the radius, or the lower sphere meets
        // the next riser while the feet are still on the step below.
        let mut m = flat();
        for i in 1..=8 {
            let z = 100.0 + 100.0 * (i - 1) as f32;
            m.add_box(Vec3::new(-300.0, 0.0, z), Vec3::new(300.0, 20.0 * i as f32, 2000.0));
        }
        let end = walk(&m, &sora(), Vec3::ZERO, 8.0, 150);
        assert!((end.y - 160.0).abs() < 1e-3, "{end}");
    }

    fn ramp(m: &mut Map, deg: f32, len: f32) -> f32 {
        let top = len * deg.to_radians().tan();
        let (a, b) = (Vec3::new(-300.0, 0.0, 200.0), Vec3::new(300.0, top, 200.0 + len));
        m.add(&[Vec3::new(a.x, 0.0, a.z), Vec3::new(a.x, top, b.z), Vec3::new(b.x, top, b.z), Vec3::new(b.x, 0.0, a.z)], attr::FLOOR);
        // Platform at the top (its ramp side is covered by the ramp).
        m.add_box_open(Vec3::new(-300.0, 0.0, b.z), Vec3::new(300.0, top, b.z + 1000.0), 1);
        top
    }

    #[test]
    fn gentle_ramp_is_walked_at_full_speed_along_the_slope() {
        let mut m = flat();
        let top = ramp(&mut m, 20.0, 400.0);
        let n = m.polys[1].normal;
        assert!(n.y > 0.9 && n.z < 0.0, "ramp faces up and back: {n}");
        // On the ramp the 3-D step keeps the flat length.
        let mid = Vec3::new(0.0, 300.0 * 20f32.to_radians().tan(), 500.0);
        let r = sora().solve(&m, mid, along_ground(Vec3::new(0.0, 0.0, 8.0), n), false);
        assert!(r.ground);
        assert!(((r.pos - mid).length() - 8.0).abs() < 0.05, "{}", (r.pos - mid).length());
        let end = walk(&m, &sora(), Vec3::ZERO, 8.0, 120);
        assert!((end.y - top).abs() < 1e-2 && end.z > 600.0, "{end}");
    }

    #[test]
    fn steep_floor_slope_slides_back() {
        let mut m = flat();
        ramp(&mut m, 45.0, 300.0);
        let n = m.polys[1].normal;
        // Standing on the slope: pushed down it.
        let on = Vec3::new(0.0, 100.0, 300.0);
        let r = sora().solve(&m, on, Vec3::ZERO, false);
        assert!(r.ground && r.pos.z < on.z - 1.0 && r.pos.y < on.y, "{} -> {}", on, r.pos);
        // Walking up at run speed never gets far.
        let end = walk(&m, &sora(), Vec3::ZERO, 8.0, 200);
        assert!(end.y < 50.0, "{end}");
        // The threshold, acos(90/115) = 38.5°: standing on 37° holds, on
        // 40° slides (running still climbs 40°; 45° beats run speed).
        let _ = n;
        for (deg, holds) in [(37.0, true), (40.0, false)] {
            let mut m = flat();
            ramp(&mut m, deg, 300.0);
            let on = Vec3::new(0.0, 150.0 * deg.to_radians().tan(), 350.0);
            let r = sora().solve(&m, on, Vec3::ZERO, false);
            assert!(r.ground);
            assert_eq!(r.pos.distance(on) < 1e-3, holds, "{deg}°: {on} -> {}", r.pos);
        }
    }

    #[test]
    fn walls_slide_and_hold_at_radius() {
        let mut m = flat();
        m.add_box(Vec3::new(-1000.0, 0.0, 200.0), Vec3::new(1000.0, 500.0, 400.0));
        let s = sora();
        let mut p = Vec3::ZERO;
        for _ in 0..60 {
            p = s.solve(&m, p, Vec3::new(4.0, 0.0, 8.0), false).pos;
        }
        assert!((p.z - (200.0 - 90.0 - SKIN)).abs() < 0.05, "held 90 from the wall: {p}");
        assert!(p.x > 200.0, "slid sideways: {p}");
    }

    #[test]
    fn walking_off_an_edge_steps_down_24_then_falls() {
        for (drop, holds) in [(20.0, true), (24.0, true), (30.0, false)] {
            let mut m = Map::new();
            m.add(&[Vec3::new(-500.0, 0.0, -500.0), Vec3::new(-500.0, 0.0, 0.0), Vec3::new(500.0, 0.0, 0.0), Vec3::new(500.0, 0.0, -500.0)], attr::FLOOR);
            m.add(&[Vec3::new(-500.0, -drop, 0.0), Vec3::new(-500.0, -drop, 500.0), Vec3::new(500.0, -drop, 500.0), Vec3::new(500.0, -drop, 0.0)], attr::FLOOR);
            let r = sora().solve(&m, Vec3::new(0.0, 0.0, -2.0), Vec3::new(0.0, 0.0, 8.0), false);
            assert_eq!(r.ground, holds, "drop {drop}");
            if holds {
                assert!((r.pos.y + drop).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn landing_needs_the_feet_below_the_floor() {
        let m = flat();
        let s = sora();
        let r = s.solve(&m, Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -3.0, 0.0), true);
        assert!(!r.ground && (r.pos.y - 2.0).abs() < 1e-4);
        let r = s.solve(&m, Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -8.0, 0.0), true);
        assert!(r.ground && r.pos.y.abs() < 1e-4);
    }

    #[test]
    fn ceiling_stops_the_rise() {
        let mut m = flat();
        m.add_box(Vec3::new(-500.0, 260.0, -500.0), Vec3::new(500.0, 300.0, 500.0));
        let s = sora();
        // Head top is 205 above the feet: at feet 50 it touches 255 < 260.
        let start = Vec3::new(0.0, 52.0, 0.0);
        let r = s.solve(&m, start, Vec3::new(0.0, 10.0, 0.0), true);
        let rise = r.pos.y - start.y;
        assert!((rise - 3.0).abs() < 0.02, "{rise}");
        assert!(hit_ceiling(10.0, rise));
        assert!(!hit_ceiling(10.0, 10.0));
        assert!(r.wall);
    }

    #[test]
    fn segment_finds_front_faces_only() {
        let mut m = flat();
        m.add_box(Vec3::new(0.0, 0.0, 0.0), Vec3::new(100.0, 100.0, 100.0));
        let h = m.segment(Vec3::new(-50.0, 50.0, 50.0), Vec3::new(200.0, 50.0, 50.0), 0).unwrap();
        assert!((h.point.x - 0.0).abs() < 1e-4);
        assert!(m.segment(Vec3::new(50.0, 50.0, 50.0), Vec3::new(50.0, 50.0, 150.0), 0).is_none(), "leaving from inside is not a hit");
        let g = m.ground(Vec3::new(50.0, 300.0, 50.0), 0.0, 0).unwrap();
        assert!((g.point.y - 100.0).abs() < 1e-4);
    }
}
