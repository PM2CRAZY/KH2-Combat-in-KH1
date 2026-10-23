//! Shoes against KH2's feet: per motion, the worst angle between KH1's foot "up" (sole bone
//! 22 / 48 → ankle 19 / 45) and KH2's (heel 16 / 45 → ankle 13 / 42), and between the toe
//! directions. `<mdls> <KH2 dir> <motions...>`
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
        let mut worst = [0.0f32; 4];
        for f in 0..n.max(1) {
            let t = kh2_anim::key_time(m, f as f32, false);
            let kh2 = kh2_anim::evaluate(&rig.skeleton, m, t, kh2_anim::Options::default()).bones;
            let p = r.pose(&kh2);
            let d = |v: &[glam::Mat4], a: usize, b: usize| (v[b].w_axis - v[a].w_axis).truncate().normalize_or_zero();
            // KH1 sides: -x (22,19 / toe 23) with KH2 +29 side (45,42 / 46); +x (48,45 / 49) with KH2 (16,13 / 17).
            for (k, (s1, a1, t1, h2, a2, t2)) in [(22usize, 19usize, 23usize, 45usize, 42usize, 46usize), (48, 45, 49, 16, 13, 17)].into_iter().enumerate() {
                let up = d(&p, s1, a1).angle_between(d(&kh2, h2, a2)).to_degrees();
                let toe = d(&p, s1, t1).angle_between(d(&kh2, h2, t2)).to_degrees();
                worst[k * 2] = worst[k * 2].max(up);
                worst[k * 2 + 1] = worst[k * 2 + 1].max(toe);
            }
        }
        println!("{motion}: worst up / toe angle: -x {:.0} / {:.0}, +x {:.0} / {:.0}", worst[0], worst[1], worst[2], worst[3]);
    }
}
