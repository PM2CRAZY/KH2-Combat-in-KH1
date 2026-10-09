//! Collision shapes and the overlap test KH2 uses between an attack and the
//! shapes it can hit.
//!
//! The rules were read from KH2 Final Mix's executable (see
//! `docs/combat-model.md`, "Hit detection"); only the behaviour is written
//! here. Coordinates are y-up and in whatever units the host uses (KH2's are
//! centimetre-like model units).
//!
//! Each shape is a centre plus two sizes:
//! - `radius`: the horizontal size
//! - `height`: the vertical half-size (a column reaches `height` above and
//!   below its centre; an ellipsoid's vertical semi-axis)
//!
//! Spheres use `radius` against other spheres; against an ellipsoid their
//! own `height` counts as the vertical radius (KH2's data stores spheres with
//! `height == radius`).

/// Shape kinds, numbered as in KH2's model collision data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShapeKind {
    Ellipsoid = 0,
    Column = 1,
    /// Present in the data, but KH2's overlap test never reports a hit for it.
    Cube = 2,
    Sphere = 3,
}

impl ShapeKind {
    pub fn from_u8(v: u8) -> Option<ShapeKind> {
        Some(match v {
            0 => ShapeKind::Ellipsoid,
            1 => ShapeKind::Column,
            2 => ShapeKind::Cube,
            3 => ShapeKind::Sphere,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub kind: ShapeKind,
    pub center: [f32; 3],
    pub radius: f32,
    pub height: f32,
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn horiz2(d: [f32; 3]) -> f32 {
    d[0] * d[0] + d[2] * d[2]
}

impl Shape {
    pub fn new(kind: ShapeKind, center: [f32; 3], radius: f32, height: f32) -> Shape {
        Shape { kind, center, radius, height }
    }

    pub fn sphere(center: [f32; 3], radius: f32) -> Shape {
        Shape::new(ShapeKind::Sphere, center, radius, radius)
    }

    pub fn column(center: [f32; 3], radius: f32, half_height: f32) -> Shape {
        Shape::new(ShapeKind::Column, center, radius, half_height)
    }

    /// KH2's overlap test. Symmetric; touching is not a hit (strict `<`).
    pub fn overlaps(&self, other: &Shape) -> bool {
        use ShapeKind::*;
        match (self.kind, other.kind) {
            (Cube, _) | (_, Cube) => false,
            (Sphere, Sphere) => {
                let d = sub(other.center, self.center);
                let r = self.radius + other.radius;
                d[0] * d[0] + d[1] * d[1] + d[2] * d[2] < r * r
            }
            (Column, Column) => {
                let d = sub(other.center, self.center);
                let r = self.radius + other.radius;
                d[1].abs() < self.height + other.height && horiz2(d) < r * r
            }
            (Column, _) => round_vs_column(other, self),
            (_, Column) => round_vs_column(self, other),
            // Ellipsoid with ellipsoid or sphere: scale the gap by the summed
            // sizes and compare with the unit sphere.
            _ => {
                let d = sub(other.center, self.center);
                let r = self.radius + other.radius;
                let h = self.height + other.height;
                let (x, y, z) = (d[0] / r, d[1] / h, d[2] / r);
                x * x + y * y + z * z < 1.0
            }
        }
    }
}

/// Ellipsoid or sphere `e` against column `c`: the ellipsoid's horizontal
/// radius at the height of the column's nearer end.
fn round_vs_column(e: &Shape, c: &Shape) -> bool {
    let d = sub(c.center, e.center);
    let dy = d[1].abs();
    if e.height + c.height <= dy {
        return false;
    }
    let r = if dy <= c.height {
        e.radius
    } else {
        let t = (dy - c.height) / e.height;
        (1.0 - t * t).abs().sqrt() * e.radius
    };
    let r = r + c.radius;
    horiz2(d) < r * r
}

/// Fills the gap a fast-moving attack shape leaves between frames.
///
/// KH2 remembers each attack shape's centre from the previous frame. If the
/// shape moved more than 1.5 × its radius, copies are added one radius
/// apart, stepping from the current centre back toward the previous one,
/// until the remaining gap is at most 1.5 × the radius. The first frame of
/// an attack has no previous centre and adds nothing.
pub fn sweep_fill(shape: &Shape, previous: [f32; 3]) -> Vec<Shape> {
    let mut out = Vec::new();
    let d = sub(previous, shape.center);
    let mut len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let r = shape.radius;
    if len == 0.0 || r <= 0.0 {
        return out;
    }
    let step = [d[0] / len * r, d[1] / len * r, d[2] / len * r];
    let mut c = shape.center;
    while r * 1.5 < len {
        c = [c[0] + step[0], c[1] + step[1], c[2] + step[2]];
        out.push(Shape { center: c, ..*shape });
        len -= r;
    }
    out
}

/// Most shapes one attack tracks between frames (KH2's limit).
pub const MAX_SWEPT_SHAPES: usize = 32;

/// One live attack's shapes across frames, with the in-between copies.
#[derive(Clone, Debug, Default)]
pub struct AttackVolume {
    previous: Vec<[f32; 3]>,
}

impl AttackVolume {
    pub fn new() -> AttackVolume {
        AttackVolume::default()
    }

    /// Centres remembered from the previous frame (empty on the first).
    pub fn previous(&self) -> &[[f32; 3]] {
        &self.previous
    }

    /// This frame's shapes plus their sweep copies; remembers the centres
    /// for the next frame. Pass the shapes in the same order every frame.
    pub fn update(&mut self, shapes: &[Shape]) -> Vec<Shape> {
        let mut out = Vec::with_capacity(shapes.len());
        let first = self.previous.is_empty();
        for (i, s) in shapes.iter().enumerate() {
            out.push(*s);
            if i >= MAX_SWEPT_SHAPES {
                continue;
            }
            if !first {
                if let Some(p) = self.previous.get(i) {
                    out.extend(sweep_fill(s, *p));
                }
            }
        }
        self.previous = shapes.iter().take(MAX_SWEPT_SHAPES).map(|s| s.center).collect();
        out
    }
}

/// The first of `targets` that any of `attack` overlaps, as KH2 reports it:
/// attack shapes are tried in order, and for each the target's shapes in
/// order. Returns (attack index, target index).
pub fn first_overlap(attack: &[Shape], targets: &[Shape]) -> Option<(usize, usize)> {
    attack.iter().enumerate().find_map(|(i, a)| targets.iter().position(|t| a.overlaps(t)).map(|j| (i, j)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spheres() {
        let a = Shape::sphere([0.0; 3], 10.0);
        assert!(a.overlaps(&Shape::sphere([19.9, 0.0, 0.0], 10.0)));
        assert!(!a.overlaps(&Shape::sphere([20.0, 0.0, 0.0], 10.0)));
        assert!(a.overlaps(&Shape::sphere([0.0, 19.0, 0.0], 10.0)));
    }

    #[test]
    fn columns_are_vertical_and_centred() {
        let a = Shape::column([0.0; 3], 10.0, 50.0);
        assert!(a.overlaps(&Shape::column([15.0, 90.0, 0.0], 10.0, 50.0)));
        assert!(!a.overlaps(&Shape::column([15.0, 100.0, 0.0], 10.0, 50.0)));
        assert!(!a.overlaps(&Shape::column([20.0, 0.0, 0.0], 10.0, 50.0)));
    }

    #[test]
    fn sphere_against_column_narrows_past_the_end() {
        let c = Shape::column([0.0; 3], 10.0, 50.0);
        // Beside the column's side: full radius.
        assert!(Shape::sphere([29.0, 40.0, 0.0], 20.0).overlaps(&c));
        // 10 above the top: radius shrinks to 20·sqrt(1 - 0.25) ≈ 17.3.
        assert!(Shape::sphere([27.0, 60.0, 0.0], 20.0).overlaps(&c));
        assert!(!Shape::sphere([28.0, 60.0, 0.0], 20.0).overlaps(&c));
        assert!(c.overlaps(&Shape::sphere([27.0, 60.0, 0.0], 20.0)));
        assert!(!Shape::sphere([0.0, 70.0, 0.0], 20.0).overlaps(&c));
    }

    #[test]
    fn ellipsoid_uses_summed_axes() {
        let e = Shape::new(ShapeKind::Ellipsoid, [0.0; 3], 10.0, 40.0);
        let s = Shape::sphere([0.0, 49.0, 0.0], 10.0);
        assert!(e.overlaps(&s));
        assert!(!e.overlaps(&Shape::sphere([0.0, 51.0, 0.0], 10.0)));
        assert!(!e.overlaps(&Shape::sphere([20.0, 0.0, 0.0], 10.0)));
    }

    #[test]
    fn cubes_never_hit() {
        let c = Shape::new(ShapeKind::Cube, [0.0; 3], 100.0, 100.0);
        assert!(!c.overlaps(&Shape::sphere([0.0; 3], 100.0)));
    }

    #[test]
    fn sweep_steps_one_radius_until_within_one_and_a_half() {
        let s = Shape::sphere([0.0; 3], 10.0);
        let fill = sweep_fill(&s, [50.0, 0.0, 0.0]);
        let xs: Vec<f32> = fill.iter().map(|f| f.center[0]).collect();
        assert_eq!(xs, vec![10.0, 20.0, 30.0, 40.0]);
        assert!(sweep_fill(&s, [15.0, 0.0, 0.0]).is_empty());
        assert_eq!(sweep_fill(&s, [16.0, 0.0, 0.0]).len(), 1);
    }

    #[test]
    fn volume_sweeps_from_the_second_frame() {
        let mut v = AttackVolume::new();
        assert_eq!(v.update(&[Shape::sphere([0.0; 3], 10.0)]).len(), 1);
        let out = v.update(&[Shape::sphere([100.0, 0.0, 0.0], 10.0)]);
        assert_eq!(out.len(), 1 + 9);
        // The gap is closed: a target between the two frames is hit.
        let target = [Shape::sphere([50.0, 0.0, 0.0], 1.0)];
        assert!(first_overlap(&out, &target).is_some());
    }
}
