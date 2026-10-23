//! Feet on uneven ground (the user: Sora's feet went through the stairs). KH1's ground-IK
//! skeleton pass (`0x140392710`) bends its own legs onto the floor under each foot, but our
//! pose replaces KH1's, and KH2 has no such step. So, while KH2 has Sora on the ground:
//! - the floor under each sole is found with KH1's own floor ray (`0x1401910E0`, the query
//!   the ground-IK pass makes: start point, a length, the hit at `+0x20` and a hit id at `+0x30`,
//!   -1 for none);
//! - the body drops when a planted foot's floor is lower than his (a step down), and each leg
//!   bends (two-bone IK: thigh, shin; the foot only moves, keeping its flat sole) to put a
//!   planted sole on its floor, or to lift a raised foot out of a step it would cut into.
//! Changes are eased over a few frames, and fade out when KH2 lets go or he leaves the ground.

use super::addr::Rva;
use glam::{Mat4, Quat, Vec3};
use std::sync::Mutex;

const FLOOR_RAY_RVA: Rva = Rva(0x1910E0);
/// KH1 Sora, per side: thigh, knee, ankle, sole (the bone on the ground in the bind).
pub const LEGS: [[usize; 4]; 2] = [[17, 18, 19, 22], [43, 44, 45, 48]];
/// A sole this close to the model's ground (KH1 units) counts as planted.
const PLANTED: f32 = 4.0;
/// How far the floor is looked for above and below a sole, and the most a body drops.
const REACH: f32 = 40.0;
const MAX_DROP: f32 = 30.0;
/// Share of the remaining change made each frame.
const EASE: f32 = 0.35;

struct State {
    drop: f32,
    lift: [f32; 2],
}

static STATE: Mutex<State> = Mutex::new(State { drop: 0.0, lift: [0.0; 2] });

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

/// The floor height (KH1 world, y-down) under a world point, if any within `REACH`.
unsafe fn floor_under(base: usize, at: Vec3) -> Option<f32> {
    let mut q = [0u8; 0x48];
    let p = q.as_mut_ptr() as usize;
    std::ptr::write_unaligned(p as *mut [f32; 3], [at.x, at.y - REACH, at.z]);
    std::ptr::write_unaligned((p + 0xC) as *mut f32, 1.0);
    std::ptr::write_unaligned((p + 0x14) as *mut f32, REACH * 2.0);
    std::ptr::write_unaligned((p + 0x1C) as *mut f32, 1.0);
    std::ptr::write_unaligned((p + 0x30) as *mut i16, -1);
    let ray: unsafe extern "system" fn(usize, u32) = std::mem::transmute(base + FLOOR_RAY_RVA);
    ray(p, 0);
    if rd::<i16>(p + 0x30) == -1 {
        return None;
    }
    let y = rd::<f32>(p + 0x24);
    y.is_finite().then_some(y)
}

/// Whether bone `i` is `root` or below it.
fn under(parents: &[i16], mut i: usize, root: usize) -> bool {
    for _ in 0..64 {
        if i == root {
            return true;
        }
        match usize::try_from(parents[i]) {
            Ok(p) if p < parents.len() => i = p,
            _ => return false,
        }
    }
    false
}

/// Puts the model-space `pose` (drawn with `to_world`) on the floor. `active`: KH2 has Sora
/// and he is on the ground (else any change fades out).
pub unsafe fn place(base: usize, pose: &mut [Mat4], parents: &[i16], to_world: &Mat4, active: bool) {
    let Ok(mut st) = STATE.try_lock() else { return };
    if pose.len() < 49 || parents.len() < pose.len() {
        return;
    }
    // Model up (KH1's world is y-down).
    let up = to_world.inverse().transform_vector3(Vec3::NEG_Y).normalize_or_zero();
    if up == Vec3::ZERO {
        return;
    }
    let mut want_drop = 0.0f32;
    let mut want = [0.0f32; 2];
    if active {
        let mut delta = [None; 2];
        for (k, leg) in LEGS.iter().enumerate() {
            let sole = to_world.transform_point3(pose[leg[3]].w_axis.truncate());
            if let Some(y) = floor_under(base, sole) {
                // Up from the sole to its floor (y-down world).
                let d = sole.y - y;
                if d.abs() < REACH {
                    let planted = pose[leg[3]].w_axis.truncate().dot(up) < PLANTED;
                    delta[k] = Some((d, planted));
                }
            }
        }
        want_drop = delta.iter().flatten().filter(|(_, p)| *p).map(|(d, _)| *d).fold(0.0f32, f32::min).max(-MAX_DROP);
        for k in 0..2 {
            if let Some((d, planted)) = delta[k] {
                let w = d - want_drop;
                want[k] = if planted { w } else { w.max(0.0) };
            }
        }
    }
    st.drop += (want_drop - st.drop) * EASE;
    for k in 0..2 {
        st.lift[k] += (want[k] - st.lift[k]) * EASE;
    }
    if st.drop.abs() < 0.05 && st.lift.iter().all(|l| l.abs() < 0.05) {
        return;
    }
    let drop = Mat4::from_translation(up * st.drop);
    for m in pose.iter_mut() {
        *m = drop * *m;
    }
    for (k, leg) in LEGS.iter().enumerate() {
        if st.lift[k].abs() < 0.05 {
            continue;
        }
        let [thigh, knee, ankle, _] = *leg;
        let (h, kn, a) = (pose[thigh].w_axis.truncate(), pose[knee].w_axis.truncate(), pose[ankle].w_axis.truncate());
        let target = a + up * st.lift[k];
        let (l1, l2) = ((kn - h).length(), (a - kn).length());
        if l1 < 1e-3 || l2 < 1e-3 {
            continue;
        }
        let span = target - h;
        let d = span.length().clamp((l1 - l2).abs() + 1e-3, l1 + l2 - 1e-3);
        let dir = span.normalize_or_zero();
        if dir == Vec3::ZERO {
            continue;
        }
        // Keep the knee on its side of the new hip-ankle line.
        let mut pole = (kn - h) - dir * (kn - h).dot(dir);
        if pole.length_squared() < 1e-4 {
            pole = to_world.inverse().transform_vector3(Vec3::Z) - dir * dir.z;
        }
        let pole = pole.normalize_or_zero();
        let along = (l1 * l1 + d * d - l2 * l2) / (2.0 * d);
        let side = (l1 * l1 - along * along).max(0.0).sqrt();
        let knee2 = h + dir * along + pole * side;
        let r1 = Quat::from_rotation_arc((kn - h) / l1, (knee2 - h).normalize_or_zero());
        let m1 = Mat4::from_translation(h) * Mat4::from_quat(r1) * Mat4::from_translation(-h);
        let shin = (r1 * (a - kn)).normalize_or_zero();
        let r2 = Quat::from_rotation_arc(shin, (h + dir * d - knee2).normalize_or_zero());
        let m2 = Mat4::from_translation(knee2) * Mat4::from_quat(r2) * Mat4::from_translation(-knee2) * m1;
        let m3 = Mat4::from_translation(h + dir * d - a);
        for i in 0..pose.len() {
            if under(parents, i, ankle) {
                pose[i] = m3 * pose[i];
            } else if under(parents, i, knee) {
                pose[i] = m2 * pose[i];
            } else if under(parents, i, thigh) {
                pose[i] = m1 * pose[i];
            }
        }
    }
}
