//! Scratch: KH2 walk / run ground speed (planted heel's travel against bone 0), as the hook
//! measures it.
use glam::{Mat4, Vec3};
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[0])).unwrap();
    let rig = sora.rig.unwrap();
    println!("walk_speed {} run_speed {}", sora.character.params.walk_speed, sora.character.params.run_speed);
    for (name, clip) in [("walk", rig.motions.get(&1)), ("run", rig.motions.get(&2)), ("field walk", rig.field_motions.get(&1)), ("field run", rig.field_motions.get(&2))] {
        let Some(clip) = clip else { continue };
        let len = kh2_anim::length_frames(clip).max(2.0) as usize;
        let poses: Vec<Vec<Mat4>> = (0..=len).map(|f| kh2_anim::evaluate(&rig.skeleton, clip, kh2_anim::key_time(clip, f as f32, true), kh2_anim::Options::default()).bones).collect();
        let at = |p: &Vec<Mat4>, b: usize| p[b].w_axis.truncate();
        let (mut sum, mut n) = (0.0, 0);
        for foot in [16usize, 45] {
            let low = poses.iter().map(|p| at(p, foot).y).fold(f32::MAX, f32::min);
            for w in poses.windows(2) {
                if at(&w[0], foot).y < low + 4.0 && at(&w[1], foot).y < low + 4.0 {
                    let d = (at(&w[1], foot) - at(&w[1], 0)) - (at(&w[0], foot) - at(&w[0], 0));
                    sum += Vec3::new(d.x, 0.0, d.z).length();
                    n += 1;
                }
            }
        }
        let root = at(poses.last().unwrap(), 0) - at(&poses[0], 0);
        println!("{name}: {len} frames, {n} planted frames, ground {:.0} units/s, root travel over the clip {root:.1?}", if n > 0 { sum / n as f32 * 60.0 } else { 0.0 });
    }
}
