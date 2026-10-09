//! A tiny test arena made of boxes: flat floor plus ledges and pillars to
//! try High Jump, Glide and Aerial Dodge. Collision is axis-aligned boxes
//! against a vertical cylinder; enough for movement testing, not a physics
//! engine.

use bevy::prelude::*;

use crate::SCALE;

/// (min x, min z, max x, max z, top y) in KH2 units.
pub type BoxDef = (f32, f32, f32, f32, f32);

pub const ARENA_BOXES: &[BoxDef] = &[
    // Steps: 30, 60, 90 units.
    (-1200.0, -900.0, -1000.0, -700.0, 30.0),
    (-1200.0, -700.0, -1000.0, -500.0, 60.0),
    (-1200.0, -500.0, -1000.0, -300.0, 90.0),
    // Ledge reachable with a normal jump (185).
    (800.0, -900.0, 1200.0, -500.0, 150.0),
    // Ledge that needs High Jump.
    (800.0, -200.0, 1200.0, 200.0, 300.0),
    // Tall tower for gliding off (needs High Jump MAX).
    (-300.0, 1300.0, 100.0, 1700.0, 550.0),
];

pub struct Arena<'a> {
    boxes: &'a [BoxDef],
}

impl<'a> Arena<'a> {
    pub fn new(boxes: &'a [BoxDef]) -> Self {
        Arena { boxes }
    }

    fn overlaps(b: &BoxDef, p: [f32; 3], r: f32) -> bool {
        let cx = p[0].clamp(b.0, b.2);
        let cz = p[2].clamp(b.1, b.3);
        (p[0] - cx).powi(2) + (p[2] - cz).powi(2) < r * r
    }

    /// Highest surface under the cylinder that is not above its feet by
    /// more than a small step.
    pub fn ground_below(&self, p: [f32; 3], r: f32) -> f32 {
        self.ground_crossed(p[1], p, r)
    }

    /// Like [`ground_below`](Self::ground_below), for a move that started at
    /// height `from_y`: a box top the feet passed through this frame still
    /// counts, so a fast fall lands on it instead of sinking in. The same
    /// radius as [`push_out`](Self::push_out) is used, so there is no strip
    /// at an edge where Sora is neither supported nor pushed away.
    pub fn ground_crossed(&self, from_y: f32, p: [f32; 3], r: f32) -> f32 {
        let top = from_y.max(p[1]) + 1.0;
        let mut g = 0.0f32;
        for b in self.boxes {
            if b.4 <= top && Self::overlaps(b, p, r) {
                g = g.max(b.4);
            }
        }
        g
    }

    /// First point where the segment `a`→`b` enters a box or goes below
    /// the floor (y = 0), if any.
    pub fn raycast(&self, a: Vec3, b: Vec3) -> Option<Vec3> {
        let d = b - a;
        let mut best: Option<f32> = None;
        if a.y >= 0.0 && b.y < 0.0 {
            best = Some(a.y / (a.y - b.y));
        }
        for bx in self.boxes {
            let lo = Vec3::new(bx.0, 0.0, bx.1);
            let hi = Vec3::new(bx.2, bx.4, bx.3);
            let (mut t0, mut t1) = (0.0f32, 1.0f32);
            let mut ok = true;
            for i in 0..3 {
                if d[i].abs() < 1e-6 {
                    if a[i] < lo[i] || a[i] > hi[i] {
                        ok = false;
                        break;
                    }
                } else {
                    let (mut ta, mut tb) = ((lo[i] - a[i]) / d[i], (hi[i] - a[i]) / d[i]);
                    if ta > tb {
                        std::mem::swap(&mut ta, &mut tb);
                    }
                    t0 = t0.max(ta);
                    t1 = t1.min(tb);
                    if t0 > t1 {
                        ok = false;
                        break;
                    }
                }
            }
            if ok && best.map_or(true, |b| t0 < b) {
                best = Some(t0);
            }
        }
        best.map(|t| a + d * t)
    }

    /// Every box's top edges as grabbable ledges.
    pub fn ledges(&self) -> Vec<character_core::Ledge> {
        let mut v = Vec::new();
        for b in self.boxes {
            let y = b.4;
            let (x0, z0, x1, z1) = (b.0, b.1, b.2, b.3);
            v.push(character_core::Ledge { a: [x0, y, z0], b: [x1, y, z0], out: [0.0, -1.0] });
            v.push(character_core::Ledge { a: [x0, y, z1], b: [x1, y, z1], out: [0.0, 1.0] });
            v.push(character_core::Ledge { a: [x0, y, z0], b: [x0, y, z1], out: [-1.0, 0.0] });
            v.push(character_core::Ledge { a: [x1, y, z0], b: [x1, y, z1], out: [1.0, 0.0] });
        }
        v
    }

    /// Pushes the cylinder out of box sides it walked into. Boxes whose top
    /// is within `step` of the feet are stepped onto instead.
    pub fn push_out(&self, mut p: [f32; 3], r: f32, step: f32) -> [f32; 3] {
        for b in self.boxes {
            if b.4 <= p[1] + step || !Self::overlaps(b, p, r) {
                continue;
            }
            let cx = p[0].clamp(b.0, b.2);
            let cz = p[2].clamp(b.1, b.3);
            let (dx, dz) = (p[0] - cx, p[2] - cz);
            let d = (dx * dx + dz * dz).sqrt();
            if d > 1e-3 {
                p[0] = cx + dx / d * r;
                p[2] = cz + dz / d * r;
            } else {
                // Center inside: leave by the nearest face.
                let opts = [(p[0] - b.0, 0), (b.2 - p[0], 1), (p[2] - b.1, 2), (b.3 - p[2], 3)];
                let (_, side) = opts.iter().copied().min_by(|a, b| a.0.total_cmp(&b.0)).unwrap();
                match side {
                    0 => p[0] = b.0 - r,
                    1 => p[0] = b.2 + r,
                    2 => p[2] = b.1 - r,
                    _ => p[2] = b.3 + r,
                }
            }
        }
        p
    }
}

pub fn spawn_arena(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(60.0, 60.0))),
        MeshMaterial3d(mats.add(Color::srgb(0.32, 0.36, 0.3))),
        Transform::default(),
    ));
    // Grid lines every 1 m (100 units) for judging distances.
    let line = meshes.add(Cuboid::new(60.0, 0.002, 0.01));
    let mat = mats.add(Color::srgb(0.38, 0.42, 0.36));
    for i in -30..=30 {
        commands.spawn((Mesh3d(line.clone()), MeshMaterial3d(mat.clone()), Transform::from_xyz(0.0, 0.001, i as f32)));
        commands.spawn((
            Mesh3d(line.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(i as f32, 0.001, 0.0).with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        ));
    }
    let box_mat = mats.add(Color::srgb(0.5, 0.48, 0.55));
    for b in ARENA_BOXES {
        let size = Vec3::new(b.2 - b.0, b.4, b.3 - b.1) * SCALE;
        let center = Vec3::new((b.0 + b.2) * 0.5, b.4 * 0.5, (b.1 + b.3) * 0.5) * SCALE;
        commands.spawn((Mesh3d(meshes.add(Cuboid::from_size(size))), MeshMaterial3d(box_mat.clone()), Transform::from_translation(center)));
    }
}
