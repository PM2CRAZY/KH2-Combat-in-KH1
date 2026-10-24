//! The retarget's thigh and shin turns against the KH2 bones that carry those parts of KH2's
//! mesh (the shorts' legs 24 / 53, the knees 12 / 41), over every motion: the angle between
//! the turns, the twist about the limb, and the largest half-frame jump of each.
//! `<mdls> <KH2 dir>`
use glam::{Mat4, Quat, Vec3};
fn rot(m: &Mat4) -> Quat { m.to_scale_rotation_translation().1.normalize() }
fn twist(q: Quat, axis: Vec3) -> f32 {
    let p = Vec3::new(q.x, q.y, q.z).dot(axis);
    let t = Quat::from_xyzw(axis.x * p, axis.y * p, axis.z * p, q.w);
    let t = if t.length_squared() < 1e-9 { Quat::IDENTITY } else { t.normalize() };
    t.angle_between(Quat::IDENTITY).to_degrees()
}
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[1])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let r = kh1_data::retarget::Retarget::new(&kh1, &rig.skeleton);
    let b1 = r.bind();
    let b2 = kh2_anim::Pose::bind(&rig.skeleton).bones;
    let mut ids: Vec<u16> = rig.motions.keys().copied().collect();
    ids.sort();
    // (KH1 bone, its child, KH2 bone, name)
    let pairs = [(43usize, 44usize, 24usize, "+x thigh/24"), (17, 18, 53, "-x thigh/53"), (44, 45, 12, "+x shin/12"), (18, 19, 41, "-x shin/41"), (45, 49, 15, "+x foot/15"), (19, 23, 44, "-x foot/44")];
    for &(k1, c1, k2, name) in &pairs {
        let (mut ang, mut tw) = (vec![], vec![]);
        let (mut jump1, mut jump2) = ((0.0f32, 0u16, 0.0f32), (0.0f32, 0u16, 0.0f32));
        let mut per: Vec<(u16, f32)> = vec![];
        for &id in &ids {
            let m = &rig.motions[&id];
            let n = kh2_anim::length_frames(m) as usize;
            let mut prev: Option<(Quat, Quat)> = None;
            let mut worst = 0.0f32;
            for h in 0..n.max(1) * 2 {
                let f = h as f32 * 0.5;
                let kh2 = kh2_anim::evaluate(&rig.skeleton, m, kh2_anim::key_time(m, f, false), kh2_anim::Options::default()).bones;
                let p = r.pose(&kh2);
                let q1 = rot(&p[k1]) * rot(&b1[k1]).inverse();
                let q2 = rot(&kh2[k2]) * rot(&b2[k2]).inverse();
                let axis = (p[c1].w_axis.truncate() - p[k1].w_axis.truncate()).normalize_or_zero();
                let e = q1.angle_between(q2).to_degrees();
                ang.push(e);
                tw.push(twist((q1 * q2.inverse()).normalize(), axis));
                worst = worst.max(e);
                if let Some((p1, p2)) = prev {
                    let j1 = q1.angle_between(p1).to_degrees();
                    let j2 = q2.angle_between(p2).to_degrees();
                    if j1 > jump1.0 { jump1 = (j1, id, f); }
                    if j2 > jump2.0 { jump2 = (j2, id, f); }
                }
                prev = Some((q1, q2));
            }
            per.push((id, worst));
        }
        ang.sort_by(|a, b| a.partial_cmp(b).unwrap());
        tw.sort_by(|a, b| a.partial_cmp(b).unwrap());
        per.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let q = |v: &Vec<f32>, p: f32| v[((v.len() - 1) as f32 * p) as usize];
        println!("{name}: angle median {:.1} p95 {:.1} max {:.1}; twist about the limb median {:.1} p95 {:.1} max {:.1}; largest half-frame turn: ours {:.1} ({} f{}), KH2's {:.1} ({} f{})", q(&ang, 0.5), q(&ang, 0.95), q(&ang, 1.0), q(&tw, 0.5), q(&tw, 0.95), q(&tw, 1.0), jump1.0, jump1.1, jump1.2, jump2.0, jump2.1, jump2.2);
        println!("   worst motions: {:?}", per.iter().take(10).map(|(i, w)| format!("{i}:{w:.0}")).collect::<Vec<_>>());
    }
}
