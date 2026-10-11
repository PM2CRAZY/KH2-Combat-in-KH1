//! The test arena, built from polygons and run through the game's map
//! collision (`map-collision`). It has:
//! - flat floor with walls at the edge
//! - stairs (20-unit risers)
//! - a gentle 20° ramp and a steep 45° one that slides
//! - a floating slab to bonk jumps on
//! - the ledges and tower for High Jump, hanging and Glide
//!
//! What is drawn is exactly what collides.

use std::sync::OnceLock;

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use map_collision::{attr, Map};

use crate::SCALE;

/// Half the floor's side.
const EDGE: f32 = 3000.0;

/// Ledge sides of a box's top: bit 0 −z, 1 +z, 2 −x, 3 +x.
const ALL: u8 = 0b1111;

enum Solid {
    /// min, max, open sides (see `Map::add_box_open`), ledge sides.
    Box(Vec3, Vec3, u8, u8),
    /// A ramp over x0..x1 rising along +z from z0 (floor) to z1 (top).
    Ramp { x0: f32, x1: f32, z0: f32, z1: f32, top: f32 },
}

fn solids() -> Vec<Solid> {
    let v = Vec3::new;
    let mut s = Vec::new();
    // Stairs: 8 steps of 20 with 100-deep treads, rising toward -z; the
    // last one is the landing (160).
    for i in 1..=8 {
        let near = -300.0 - 100.0 * (i - 1) as f32;
        let far = if i == 8 { -1400.0 } else { near - 100.0 };
        let open = if i == 8 { 0 } else { 0b0001 };
        let ledges = if i == 8 { 0b1101 } else { 0 };
        s.push(Solid::Box(v(-1250.0, 0.0, far), v(-1000.0, 20.0 * i as f32, near), open, ledges));
    }
    // Gentle ramp (20°) up to a 160 platform.
    let top = 160.0;
    let len = top / 20f32.to_radians().tan();
    s.push(Solid::Ramp { x0: -1250.0, x1: -1000.0, z0: 100.0, z1: 100.0 + len, top });
    s.push(Solid::Box(v(-1250.0, 0.0, 100.0 + len), v(-1000.0, top, 900.0), 0b0001, 0b1110));
    // Steep ramp (45°): too steep to stand on.
    s.push(Solid::Ramp { x0: -1750.0, x1: -1500.0, z0: 100.0, z1: 350.0, top: 250.0 });
    s.push(Solid::Box(v(-1750.0, 0.0, 350.0), v(-1500.0, 250.0, 600.0), 0b0001, 0b1110));
    // Floating slab, underside at 320: a jump (head 205 above the feet)
    // bonks on it.
    s.push(Solid::Box(v(1400.0, 320.0, 400.0), v(1800.0, 360.0, 800.0), 0, ALL));
    // Ledge reachable with a normal jump (185).
    s.push(Solid::Box(v(800.0, 0.0, -900.0), v(1200.0, 150.0, -500.0), 0, ALL));
    // Ledge that needs High Jump.
    s.push(Solid::Box(v(800.0, 0.0, -200.0), v(1200.0, 300.0, 200.0), 0, ALL));
    // Tall tower for gliding off (needs High Jump MAX).
    s.push(Solid::Box(v(-300.0, 0.0, 1300.0), v(100.0, 550.0, 1700.0), 0, ALL));
    s
}

pub struct Arena {
    pub map: Map,
    ledges: Vec<character_core::Ledge>,
    /// Polygons from this index on are drawn (the floor and the edge walls
    /// come first).
    first_drawn: usize,
}

impl Arena {
    /// The arena (built once).
    pub fn get() -> &'static Arena {
        static ARENA: OnceLock<Arena> = OnceLock::new();
        ARENA.get_or_init(Arena::build)
    }

    fn build() -> Arena {
        let v = Vec3::new;
        let mut map = Map::new();
        map.add(&[v(-EDGE, 0.0, -EDGE), v(-EDGE, 0.0, EDGE), v(EDGE, 0.0, EDGE), v(EDGE, 0.0, -EDGE)], attr::FLOOR);
        let h = 2000.0;
        for (a, b) in [(v(-EDGE, 0.0, -EDGE), v(EDGE, 0.0, -EDGE)), (v(EDGE, 0.0, -EDGE), v(EDGE, 0.0, EDGE)), (v(EDGE, 0.0, EDGE), v(-EDGE, 0.0, EDGE)), (v(-EDGE, 0.0, EDGE), v(-EDGE, 0.0, -EDGE))] {
            // Facing inward (counter-clockwise seen from inside the arena). These were wound the
            // other way, facing out: they never held Sora in, and within his radius of one the
            // solver pushed him out through it.
            map.add(&[a, b, b + Vec3::Y * h, a + Vec3::Y * h], attr::WALL);
        }
        let first_drawn = map.polys.len();
        let mut ledges = Vec::new();
        for s in solids() {
            match s {
                Solid::Box(lo, hi, open, l) => {
                    map.add_box_open(lo, hi, open);
                    let y = hi.y;
                    let (x0, z0, x1, z1) = (lo.x, lo.z, hi.x, hi.z);
                    let edges = [
                        ([x0, y, z0], [x1, y, z0], [0.0, -1.0]),
                        ([x0, y, z1], [x1, y, z1], [0.0, 1.0]),
                        ([x0, y, z0], [x0, y, z1], [-1.0, 0.0]),
                        ([x1, y, z0], [x1, y, z1], [1.0, 0.0]),
                    ];
                    for (i, (a, b, out)) in edges.into_iter().enumerate() {
                        if l & (1 << i) != 0 {
                            ledges.push(character_core::Ledge { a, b, out });
                        }
                    }
                }
                Solid::Ramp { x0, x1, z0, z1, top } => {
                    map.add(&[v(x0, 0.0, z0), v(x0, top, z1), v(x1, top, z1), v(x1, 0.0, z0)], attr::FLOOR);
                    map.add(&[v(x0, 0.0, z0), v(x0, 0.0, z1), v(x0, top, z1)], attr::WALL);
                    map.add(&[v(x1, 0.0, z0), v(x1, top, z1), v(x1, 0.0, z1)], attr::WALL);
                }
            }
        }
        Arena { map, ledges, first_drawn }
    }

    /// Height of the highest floor under `p` not above its feet (0 when
    /// there is none).
    pub fn ground_below(&self, p: [f32; 3]) -> f32 {
        self.map.ground(Vec3::from(p), 1.0, attr::PLAYER_PASS).map_or(0.0, |h| h.point.y)
    }

    /// First polygon the segment `a`→`b` crosses from its front.
    pub fn raycast(&self, a: Vec3, b: Vec3) -> Option<Vec3> {
        self.map.segment(a, b, 0).map(|h| h.point)
    }

    /// Grabbable box-top edges.
    pub fn ledges(&self) -> &[character_core::Ledge] {
        &self.ledges
    }
}

pub fn spawn_arena(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(2.0 * EDGE * SCALE, 2.0 * EDGE * SCALE))),
        MeshMaterial3d(mats.add(Color::srgb(0.32, 0.36, 0.3))),
        Transform::default(),
    ));
    // Grid lines every 1 m (100 units) for judging distances.
    let n = (EDGE / 100.0) as i32;
    let line = meshes.add(Cuboid::new(2.0 * EDGE * SCALE, 0.002, 0.01));
    let mat = mats.add(Color::srgb(0.38, 0.42, 0.36));
    for i in -n..=n {
        commands.spawn((Mesh3d(line.clone()), MeshMaterial3d(mat.clone()), Transform::from_xyz(0.0, 0.001, i as f32)));
        commands.spawn((
            Mesh3d(line.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(i as f32, 0.001, 0.0).with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        ));
    }
    // Every collision polygon from the props, flat shaded: floor-type
    // surfaces light, walls darker, steep floors reddish.
    let arena = Arena::get();
    let (mut pos, mut nor, mut col, mut idx) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for p in &arena.map.polys[arena.first_drawn..] {
        let base = pos.len() as u32;
        let c = if p.surface() == attr::FLOOR {
            if p.normal.y < (90.0f32 / 115.0) {
                [0.66, 0.45, 0.42, 1.0]
            } else {
                [0.6, 0.58, 0.66, 1.0]
            }
        } else {
            [0.42, 0.4, 0.48, 1.0]
        };
        for v in p.corners() {
            pos.push((*v * SCALE).to_array());
            nor.push(p.normal.to_array());
            col.push(c);
        }
        for k in 1..p.corners().len() as u32 - 1 {
            // Counter-clockwise from the front, as Bevy expects.
            idx.extend([base, base + k, base + k + 1]);
        }
    }
    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nor)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_indices(Indices::U32(idx));
    commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(mats.add(StandardMaterial { base_color: Color::WHITE, perceptual_roughness: 0.9, ..default() })), Transform::default()));
}

/// A mover's map-collision body from its model's BG shape (type 0 or 4).
/// Type 4 ("map") adds the lower sphere against walls (exe). Without a
/// model, Sora's numbers.
pub fn body_of(shape: Option<&kh2_data::sora::BoneShape>, player: bool) -> map_collision::Body {
    let pass = if player { attr::PLAYER_PASS } else { 0 };
    match shape {
        Some(s) => map_collision::Body { radius: s.radius, half_height: s.height, center: s.lift, lower_sphere: s.kind != 0, pass, snap: 0.0 },
        None => map_collision::Body { pass, ..map_collision::Body::player(90.0, 90.0, 115.0) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The edge walls face the middle of the arena and hold Sora in.
    #[test]
    fn edge_walls_face_inward_and_hold() {
        let arena = Arena::build();
        for p in &arena.map.polys[1..arena.first_drawn] {
            let centre = p.corners().iter().copied().sum::<Vec3>() / p.corners().len() as f32;
            assert!(p.normal.dot(-centre.with_y(0.0)) > 0.0, "edge wall at {centre} faces out ({})", p.normal);
        }
        let body = body_of(None, true);
        for dir in [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z] {
            let mut pos = dir * 2700.0 + dir.cross(Vec3::Y) * 300.0;
            for _ in 0..60 {
                let m = body.solve(&arena.map, pos, dir * 26.0, false);
                pos = m.pos;
            }
            let reach = pos.dot(dir);
            assert!(reach <= EDGE - 89.0 && reach > EDGE - 95.0, "running {dir}: held at {reach}");
            assert!(pos.y.abs() < 1e-3, "still on the floor: {pos}");
        }
    }
}
