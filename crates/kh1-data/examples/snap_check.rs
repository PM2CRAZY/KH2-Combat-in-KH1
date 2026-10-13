//! Frame-to-frame turn of chosen KH1 bones over KH2 motions (spots snaps):
//! `<mdls> <KH2 dir> <bones comma list> <motions...>`.
use kh1_data::retarget::Retarget;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[1])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let bones: Vec<usize> = a[2].split(',').map(|s| s.parse().unwrap()).collect();
    let r = Retarget::new(&kh1, &rig.skeleton);
    for ms in &a[3..] {
        let motion: u16 = ms.parse().unwrap();
        let Some(m) = rig.motions.get(&motion) else { println!("{motion}: none"); continue };
        let n = kh2_anim::length_frames(m) as usize;
        let mut prev: Option<Vec<glam::Mat4>> = None;
        let mut worst = vec![(0.0f32, 0usize); bones.len()];
        for f in 0..n.max(1) * 2 {
            let t = kh2_anim::key_time(m, f as f32 * 0.5, false);
            let kh2 = kh2_anim::evaluate(&rig.skeleton, m, t, kh2_anim::Options::default()).bones;
            let p = r.pose(&kh2);
            if let Some(q) = &prev {
                for (k, &b) in bones.iter().enumerate() {
                    // How far a point 10 units out along each axis moves, less the bone's own move
                    // (a turn of the bone's frame, as the skin sees it).
                    let off = glam::Vec3::splat(10.0);
                    let da = p[b].transform_point3(off) - p[b].w_axis.truncate();
                    let db = q[b].transform_point3(off) - q[b].w_axis.truncate();
                    let ang = da.angle_between(db).to_degrees();
                    if ang > worst[k].0 { worst[k] = (ang, f / 2); }
                }
            }
            prev = Some(p);
        }
        println!("{motion} ({n} frames): worst half-frame turn {:?}", bones.iter().zip(&worst).map(|(b, w)| format!("{b}: {:.0} deg @f{}", w.0, w.1)).collect::<Vec<_>>());
    }
}
