//! In KH1's own motions, how a bone turns between two others: for each frame, the blend t of
//! slerp(a, b, t) closest to the bone's model-space turn from bind, and its residual.
//! `<mdls> <mset> <bone> <a> <b> <slots...>`
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let motions = kh1_data::mset::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    let (bone, ba, bb): (usize, usize, usize) = (a[2].parse().unwrap(), a[3].parse().unwrap(), a[4].parse().unwrap());
    let bind = kh2_anim::Pose::bind(&kh1).bones;
    let rot = |m: &glam::Mat4| m.to_scale_rotation_translation().1;
    for ss in &a[5..] {
        let slot: usize = ss.parse().unwrap();
        let Some(m) = motions.get(slot).cloned().flatten() else { continue };
        let mut ts = vec![];
        for f in (0..m.frames).step_by(4) {
            let p = kh2_anim::Pose::bind(&m.apply(&kh1, f as f32)).bones;
            let turn = |i: usize| rot(&p[i]) * rot(&bind[i]).inverse();
            let (q, qa, qb) = (turn(bone), turn(ba), turn(bb));
            let mut best = (0.0f32, f32::MAX);
            for k in 0..=20 {
                let t = k as f32 / 20.0;
                let e = q.angle_between(qa.slerp(qb, t)).to_degrees();
                if e < best.1 { best = (t, e); }
            }
            ts.push(format!("{:.2}/{:.0}", best.0, best.1));
        }
        println!("slot {slot}: t/residual deg per 4 frames: {}", ts.join(" "));
    }
}
