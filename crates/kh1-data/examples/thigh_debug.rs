//! KH2 hip / knee / ankle positions and the KH1 thigh bone (17) around a frame.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[1])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let motion: u16 = a[2].parse().unwrap();
    let f0: f32 = a[3].parse().unwrap();
    let r = kh1_data::retarget::Retarget::new(&kh1, &rig.skeleton);
    let m = rig.motions.get(&motion).unwrap();
    for k in 0..8 {
        let f = f0 + k as f32 * 0.5;
        let t = kh2_anim::key_time(m, f, false);
        let kh2 = kh2_anim::evaluate(&rig.skeleton, m, t, kh2_anim::Options::default()).bones;
        let p = |i: usize| kh2[i].w_axis.truncate();
        let o = r.pose(&kh2);
        let pt = r.part_turns(&kh2);
        let ax = |q: Option<glam::Quat>| q.map(|q| (q * glam::Vec3::Z, q * glam::Vec3::Y));
        println!("   parts 3 (thigh +x) {:.2?} 18 (thigh -x) {:.2?} 19 (shin -x) {:.2?}", ax(pt[3]), ax(pt[18]), ax(pt[19]));
        println!("f{f:4.1} hipL {:.1?} kneeL {:.1?} ankleL {:.1?} hipR {:.1?} | KH1 17 x {:.2?} z {:.2?}", p(40), p(41), p(42), p(11), o[17].x_axis.truncate().normalize(), o[17].z_axis.truncate().normalize());
    }
}
