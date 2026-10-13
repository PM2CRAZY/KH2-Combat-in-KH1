//! Checks the retargeted pose for degenerate bone matrices (zero or flipped axes) over KH2
//! motions. `cargo run -p kh1-data --example hand_check -- <xa_ex_0010.mdls> <KH2 dir> <motions...>`
use kh1_data::retarget::Retarget;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[1])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let r = Retarget::new(&kh1, &rig.skeleton);
    for ms in &a[2..] {
        let motion: u16 = ms.parse().unwrap();
        let Some(m) = rig.motions.get(&motion) else { println!("{motion}: none"); continue };
        let n = kh2_anim::length_frames(m) as usize;
        let mut worst: Vec<(f32, usize, usize)> = Vec::new();
        for f in 0..n.max(1) {
            let t = kh2_anim::key_time(m, f as f32, false);
            let kh2 = kh2_anim::evaluate(&rig.skeleton, m, t, kh2_anim::Options::default()).bones;
            let p = r.pose(&kh2);
            for (i, b) in p.iter().enumerate() {
                let d = glam::Mat3::from_mat4(*b).determinant();
                if !(d > 0.2) {
                    worst.push((d, i, f));
                }
            }
        }
        worst.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Less));
        let bones: std::collections::BTreeSet<usize> = worst.iter().map(|w| w.1).collect();
        println!("{motion}: {n} frames, {} bad (bone, frame) pairs, bones {:?}, worst {:?}", worst.len(), bones, worst.first());
    }
}
