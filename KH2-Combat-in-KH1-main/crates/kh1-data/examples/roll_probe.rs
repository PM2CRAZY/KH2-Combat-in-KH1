//! Retarget check: in KH1's result, does each limb bone's rest-forward (+z) stay with where its
//! foot points, and is the hip line level with KH2's? Prints per motion frame.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[1]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[0])).unwrap();
    let rig = sora.rig.unwrap();
    let r = kh1_data::retarget::Retarget::new(&kh1, &rig.skeleton);
    let bind = r.bind();
    println!("{} KH1 bones turned by measured parts", r.mapped());
    for mo in [0u16, 1, 2] {
        let m = rig.motions.get(&mo).unwrap();
        for f in [0.0f32, 10.0, 20.0] {
            let p2 = kh2_anim::evaluate(&rig.skeleton, m, kh2_anim::key_time(m, f, true), kh2_anim::Options::default()).bones;
            let p = r.pose(&p2);
            let turn = |i: usize| p[i].to_scale_rotation_translation().1 * bind[i].to_scale_rotation_translation().1.inverse();
            let pos = |i: usize| p[i].w_axis.truncate();
            let mut line = format!("motion {mo} f{f:2}");
            for (name, bones, heel, toe) in [("+x", [42usize, 43, 44, 45, 47, 48], 48usize, 49usize), ("-x", [16, 17, 18, 19, 21, 22], 22, 23)] {
                let foot = (pos(toe) - pos(heel)).normalize();
                let s: Vec<String> = bones.iter().map(|&b| format!("{:3.0}", (turn(b) * glam::Vec3::Z).angle_between(foot).to_degrees())).collect();
                line += &format!(" | {name} leg {}", s.join(" "));
            }
            // Belt / shorts bones (children of the pelvis and thighs): how far they turn from the pelvis.
            let rel = |b: usize| (turn(3).inverse() * turn(b)).angle_between(glam::Quat::IDENTITY).to_degrees();
            line += &format!(" | thighs vs pelvis {:3.0} {:3.0} | chest vs pelvis {:3.0} | toes y {:.1} {:.1}", rel(43), rel(17), rel(78), pos(49).y, pos(23).y);
            println!("{line}");
        }
    }
}
