//! How KH1's retargeted thumbs lie against KH2's: the angle between the two thumbs' directions
//! (base → tip, model space) over KH2 motions. `<mdls> <KH2 dir> <motions...>`
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[1])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let r = kh1_data::retarget::Retarget::new(&kh1, &rig.skeleton);
    for ms in &a[2..] {
        let motion: u16 = ms.parse().unwrap();
        let Some(m) = rig.motions.get(&motion) else { continue };
        let n = kh2_anim::length_frames(m) as usize;
        let mut worst = [0.0f32; 2];
        for f in 0..n.max(1) {
            let t = kh2_anim::key_time(m, f as f32, false);
            let kh2 = kh2_anim::evaluate(&rig.skeleton, m, t, kh2_anim::Options::default()).bones;
            let p = r.pose(&kh2);
            let d = |v: &[glam::Mat4], a: usize, b: usize| (v[b].w_axis - v[a].w_axis).truncate().normalize_or_zero();
            for (k, (t1, t2)) in [(103usize, 181usize), (207, 89)].into_iter().enumerate() {
                let ang = d(&p, t1, t1 + 4).angle_between(d(&kh2, t2, t2 + 3)).to_degrees();
                worst[k] = worst[k].max(ang);
            }
        }
        println!("{motion}: worst thumb angle KH1 vs KH2: right {:.0} deg, left {:.0} deg", worst[0], worst[1]);
    }
}
