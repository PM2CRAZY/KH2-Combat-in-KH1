//! Checks the retargeted pose over KH2 motions: degenerate bone matrices, and how far each
//! bone below the right hand (101) strays from its bind distance to the hand.
//! `cargo run -p kh1-data --example hand_check -- <xa_ex_0010.mdls> <KH2 dir> <motions...>`
use kh1_data::retarget::Retarget;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[1])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let r = Retarget::new(&kh1, &rig.skeleton);
    let bind = r.bind();
    let below = |root: usize| -> Vec<usize> {
        (0..kh1.len()).filter(|&i| {
            let mut p = i as i16;
            while p >= 0 { if p as usize == root { return true; } p = kh1[p as usize].parent; }
            false
        }).collect()
    };
    let hand: Vec<usize> = below(101);
    println!("below 101: {hand:?}");
    for ms in &a[2..] {
        let motion: u16 = ms.parse().unwrap();
        let Some(m) = rig.motions.get(&motion) else { println!("{motion}: none"); continue };
        let n = kh2_anim::length_frames(m) as usize;
        let mut bad = 0;
        let mut worst = (0.0f32, 0usize, 0usize);
        let mut scale = (f32::MAX, f32::MIN);
        for f in 0..n.max(1) {
            let t = kh2_anim::key_time(m, f as f32, false);
            let kh2 = kh2_anim::evaluate(&rig.skeleton, m, t, kh2_anim::Options::default()).bones;
            let p = r.pose(&kh2);
            bad += p.iter().filter(|b| !(glam::Mat3::from_mat4(**b).determinant() > 0.2)).count();
            for &i in &hand {
                let d0 = (bind[i].w_axis - bind[101].w_axis).length();
                let d = (p[i].w_axis - p[101].w_axis).length();
                if (d - d0).abs() > worst.0 { worst = ((d - d0).abs(), i, f); }
                let det = glam::Mat3::from_mat4(p[i]).determinant();
                scale = (scale.0.min(det), scale.1.max(det));
            }
        }
        println!("{motion}: {n} frames, {bad} degenerate, hand chain worst stretch {:.1} (bone {}, frame {}), det range {:.2?}", worst.0, worst.1, worst.2, scale);
    }
}
