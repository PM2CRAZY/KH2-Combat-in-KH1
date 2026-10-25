//! The test bake's hood, coat and shorts (`kh1_data::cloth`) on KH2's walk and run (battle and
//! field), as `Baker::poses` makes them: nothing not finite, no bone length changed by the pass,
//! the loop still closing, every joint kept out of the body (within 2 units), and the hood and
//! shorts moving now. Needs files from your own installs, skipped without them (as `local_own`):
//! KH1's `xa_ex_0010.mdls` / `.mset` (`KH1_MDLS` / `KH1_MSET`, or `local/re/`) and KH2's files in
//! `KH2_RE_DIR` (or `local/re`) as `kh2_data::sora::load_sora_dir` reads them.

use glam::{Mat4, Quat, Vec3};
use kh1_data::cloth::{ClothBake, Parts};
use kh1_data::retarget::Retarget;
use std::path::PathBuf;

fn at(m: &Mat4) -> Vec3 {
    m.w_axis.truncate()
}

#[test]
fn hood_coat_and_shorts_move_and_stay_out_of_the_body() {
    let re = std::env::var_os("KH2_RE_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../local/re"));
    let mdls = std::env::var_os("KH1_MDLS").map(PathBuf::from).unwrap_or_else(|| re.join("xa_ex_0010.mdls"));
    let mset = std::env::var_os("KH1_MSET").map(PathBuf::from).unwrap_or_else(|| re.join("xa_ex_0010.mset"));
    let (Ok(a), Ok(b), Ok(sora)) = (std::fs::read(&mdls), std::fs::read(&mset), kh2_data::sora::load_sora_dir(&re)) else {
        eprintln!("skipped: no KH1 or KH2 files in {}", re.display());
        return;
    };
    let kh1 = kh1_data::mdls::parse_skeleton(&a).unwrap();
    let kh1_motions = kh1_data::mset::parse(&b).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let mut r = Retarget::new(&kh1, &rig.skeleton);
    r.set_rest(&kh1_motions[1].as_ref().unwrap().apply(&kh1, 0.0));
    let s = kh1_data::secondary::Secondary::new(&kh1, &rig.skeleton, None);
    let c = ClothBake::new(&kh1, &rig.skeleton);
    let rot = |m: &Mat4| m.to_scale_rotation_translation().1.normalize();
    for field in [false, true] {
        for id in [1u16, 2] {
            let m = if field { rig.field_motions.get(&id) } else { rig.motions.get(&id) }.expect("KH2 walk / run");
            let n = kh2_anim::length_frames(m).round() as usize;
            let mut bodies = Vec::new();
            let mut poses = Vec::new();
            for f in 0..=n {
                let body = kh2_anim::evaluate(&rig.skeleton, m, kh2_anim::key_time(m, f as f32, false), kh2_anim::Options::default()).bones;
                let mut p = r.pose(&body);
                s.apply(&mut p, &body, None, kh1_data::secondary::Parts { hair: true, hood: true, chain: false, sleeves: true, necklace: true });
                poses.push(p);
                bodies.push(body);
            }
            let before = poses.clone();
            let rep = c.bake(&mut poses, &bodies, Some(n), Parts::ALL);
            assert_eq!(rep.chains, 18, "KH2 {id} field {field}: {rep:?}");
            assert!(rep.worst_inside < 2.0, "KH2 {id} field {field}: {rep:?}");
            for (p, q) in poses.iter().zip(&before) {
                assert!(p.iter().all(|m| m.is_finite()));
                for (i, bone) in kh1.iter().enumerate() {
                    if bone.parent >= 0 {
                        let pp = bone.parent as usize;
                        let d = (at(&p[i]).distance(at(&p[pp])) - at(&q[i]).distance(at(&q[pp]))).abs();
                        assert!(d < 0.01, "KH2 {id} field {field}: bone {i} length changed by {d}");
                    }
                }
            }
            let seam = poses[0].iter().zip(&poses[n]).map(|(x, y)| at(x).distance(at(y))).fold(0.0f32, f32::max);
            let seam0 = before[0].iter().zip(&before[n]).map(|(x, y)| at(x).distance(at(y))).fold(0.0f32, f32::max);
            assert!(seam <= seam0 + 0.05, "KH2 {id} field {field}: loop seam {seam0} -> {seam}");
            // The hood (80 against the neck base 78) and a shorts panel (57 against the +x thigh 43)
            // move within the motion now.
            for (j, carrier) in [(80usize, 78usize), (57, 43)] {
                let rel: Vec<Quat> = poses.iter().map(|p| rot(&p[carrier]).inverse() * rot(&p[j])).collect();
                let swing = rel.iter().map(|q| q.angle_between(rel[0])).fold(0.0f32, f32::max).to_degrees();
                assert!(swing > 5.0, "KH2 {id} field {field}: bone {j} swings only {swing:.1} degrees");
            }
        }
    }
}
