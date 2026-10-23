//! A model's triangles at a pose (CPU skinning). KH2 models bind each
//! vertex to one bone, in that bone's space: a corner is its bone's
//! model-space matrix times its position.

use glam::{Mat4, Vec3};
use kh2_formats::mdlx::Model;

/// Calls `emit(group, corners, uvs)` for every triangle of `model` posed by
/// `bones` (model space, as [`crate::evaluate`] gives them) and then placed
/// by `place`. Triangles on a bone the pose lacks are skipped.
pub fn skin(model: &Model, bones: &[Mat4], place: Mat4, mut emit: impl FnMut(usize, [Vec3; 3], [[f32; 2]; 3])) {
    let at: Vec<Mat4> = bones.iter().map(|b| place * *b).collect();
    for (g, group) in model.groups.iter().enumerate() {
        'tri: for t in &group.triangles {
            let mut p = [Vec3::ZERO; 3];
            for (k, v) in t.iter().enumerate() {
                let Some(m) = at.get(v.bone as usize) else { continue 'tri };
                p[k] = m.transform_point3(Vec3::from(v.position));
            }
            emit(g, p, [t[0].uv, t[1].uv, t[2].uv]);
        }
    }
}
