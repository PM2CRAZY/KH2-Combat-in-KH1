//! The retarget with a KH1 rest pose (slot 1 frame 0): for some KH2 motions, where the hood
//! tip (84), a shorts panel tip (29, 33, 37, 41) and the toes sit, with and without it, and any
//! degenerate matrices.
//! `cargo run -p kh1-data --example rest_check -- <mdls> <mset> <KH2 dir> <motions...>`
use kh1_data::retarget::Retarget;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let motions = kh1_data::mset::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[2])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let plain = Retarget::new(&kh1, &rig.skeleton);
    let mut rested = Retarget::new(&kh1, &rig.skeleton);
    rested.set_rest(&motions[1].as_ref().unwrap().apply(&kh1, 0.0));
    for ms in &a[3..] {
        let motion: u16 = ms.parse().unwrap();
        let Some(m) = rig.motions.get(&motion) else { continue };
        let n = kh2_anim::length_frames(m) as usize;
        for f in [0, n / 3, 2 * n / 3] {
            let t = kh2_anim::key_time(m, f as f32, false);
            let kh2 = kh2_anim::evaluate(&rig.skeleton, m, t, kh2_anim::Options::default()).bones;
            let (p, q) = (plain.pose(&kh2), rested.pose(&kh2));
            let rel = |v: &[glam::Mat4], i: usize, from: usize| (v[i].w_axis - v[from].w_axis).truncate();
            let seg = |v: &[glam::Mat4], n: usize| {
                let (a, b, p) = (v[71].w_axis.truncate(), v[78].w_axis.truncate(), v[n].w_axis.truncate());
                let ab = b - a;
                let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
                (p - (a + ab * t)).length()
            };
            println!("   hood joints off the spine (71-78): {:.1} {:.1} {:.1} {:.1}", seg(&q, 81), seg(&q, 82), seg(&q, 83), seg(&q, 84));
            let bad = q.iter().filter(|m| !(glam::Mat3::from_mat4(**m).determinant() > 0.2)).count();
            println!(
                "{motion} f{f}: hood tip from neck {:.0?} -> {:.0?}; front panel tip from hip {:.0?} -> {:.0?}; toe {:.0?} -> {:.0?}; bad {bad}",
                rel(&p, 84, 78), rel(&q, 84, 78), rel(&p, 29, 17), rel(&q, 29, 17), rel(&p, 25, 21), rel(&q, 25, 21)
            );
        }
    }
}
