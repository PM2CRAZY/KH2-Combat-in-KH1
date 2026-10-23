//! Closest approach of chosen KH1 bones to a limb segment (a bone pair) over KH2 motions:
//! `<mdls> <KH2 dir> <bones a,b,..> <seg_from> <seg_to> <motions...>` (spots cloth cutting
//! into a thigh).
fn seg_dist(p: glam::Vec3, a: glam::Vec3, b: glam::Vec3) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[1])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let bones: Vec<usize> = a[2].split(',').map(|s| s.parse().unwrap()).collect();
    let (s0, s1): (usize, usize) = (a[3].parse().unwrap(), a[4].parse().unwrap());
    let r = kh1_data::retarget::Retarget::new(&kh1, &rig.skeleton);
    let bind = r.bind();
    let at = |v: &[glam::Mat4], i: usize| v[i].w_axis.truncate();
    let b0: Vec<f32> = bones.iter().map(|&b| seg_dist(at(&bind, b), at(&bind, s0), at(&bind, s1))).collect();
    println!("bind distances {:?}", b0.iter().map(|d| format!("{d:.1}")).collect::<Vec<_>>());
    for ms in &a[5..] {
        let motion: u16 = ms.parse().unwrap();
        let Some(m) = rig.motions.get(&motion) else { continue };
        let n = kh2_anim::length_frames(m) as usize;
        let mut low = vec![(f32::MAX, 0usize); bones.len()];
        for f in 0..n.max(1) {
            let t = kh2_anim::key_time(m, f as f32, false);
            let kh2 = kh2_anim::evaluate(&rig.skeleton, m, t, kh2_anim::Options::default()).bones;
            let p = r.pose(&kh2);
            for (k, &b) in bones.iter().enumerate() {
                let d = seg_dist(at(&p, b), at(&p, s0), at(&p, s1));
                if d < low[k].0 { low[k] = (d, f); }
            }
        }
        println!("{motion}: closest {:?}", bones.iter().zip(&low).map(|(b, l)| format!("{b}: {:.1} @f{}", l.0, l.1)).collect::<Vec<_>>());
    }
}
