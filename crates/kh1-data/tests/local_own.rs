//! KH1's own parts (`retarget::KH1_OWN`) taking KH1's own animation of a move under KH2's
//! poses (`Retarget::set_own`). Needs files from your own installs, skipped without them: KH1's
//! `xa_ex_0010.mdls` and `xa_ex_0010.mset` (`KH1_MDLS` / `KH1_MSET`, or `local/re/`) and KH2's
//! `P_EX100.mdlx` / `P_EX100.mset` (`KH2_RE_DIR`, or `local/re`).

use glam::{Mat4, Vec3};
use kh1_data::retarget::{own_relation, Retarget, KH1_OWN};
use std::path::PathBuf;

struct Files {
    kh1: Vec<kh2_formats::anim::Bone>,
    kh1_motions: Vec<Option<kh1_data::mset::Motion>>,
    kh2: Vec<kh2_formats::anim::Bone>,
    kh2_motions: kh2_formats::motion::Mset,
}

fn files() -> Option<Files> {
    let re = std::env::var_os("KH2_RE_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../local/re"));
    let mdls = std::env::var_os("KH1_MDLS").map(PathBuf::from).unwrap_or_else(|| re.join("xa_ex_0010.mdls"));
    let mset = std::env::var_os("KH1_MSET").map(PathBuf::from).unwrap_or_else(|| re.join("xa_ex_0010.mset"));
    let read = |p: PathBuf| std::fs::read(&p).ok();
    let (Some(a), Some(b), Some(c), Some(d)) = (read(mdls), read(mset), read(re.join("P_EX100.mdlx")), read(re.join("P_EX100.mset"))) else {
        eprintln!("skipped: no KH1 or KH2 files in {}", re.display());
        return None;
    };
    Some(Files {
        kh1: kh1_data::mdls::parse_skeleton(&a).ok()?,
        kh1_motions: kh1_data::mset::parse(&b).ok()?,
        kh2: kh2_formats::anim::parse_skeleton(&c).ok()?,
        kh2_motions: kh2_formats::motion::Mset::parse(&d).ok()?,
    })
}

fn at(m: &Mat4) -> Vec3 {
    m.w_axis.truncate()
}

#[test]
fn kh1_s_own_parts_follow_kh1_s_motion_on_kh2_s() {
    let Some(f) = files() else { return };
    let mut r = Retarget::new(&f.kh1, &f.kh2);
    r.set_rest(&f.kh1_motions[1].as_ref().unwrap().apply(&f.kh1, 0.0));
    let parents: Vec<i16> = f.kh1.iter().map(|b| b.parent).collect();
    // KH2's run (2) on KH1's run (slot 5), KH2's first swing (151) on KH1's (98).
    for (motion, slot) in [(2usize, 5usize), (151, 98)] {
        let m = f.kh2_motions.slot_for(motion, true, true).and_then(|s| s.anb.as_ref()).and_then(|a| a.pose.clone()).expect("KH2 motion");
        let t = f.kh1_motions[slot].as_ref().expect("KH1 motion");
        let frames = kh2_anim::length_frames(&m).round() as usize;
        let mut moved = 0f32;
        let mut first: Option<Vec<Mat4>> = None;
        for fr in 0..=frames {
            let tm = kh2_anim::Pose::bind(&t.apply(&f.kh1, fr as f32 * t.frames as f32 / frames as f32)).bones;
            let want: Vec<Option<(Mat4, Vec3)>> = (0..f.kh1.len()).map(|b| own_relation(&tm[(parents[b].max(0)) as usize], &tm[b])).collect();
            r.set_own(|b| want[b].map(|w| w.0));
            let body = kh2_anim::evaluate(&f.kh2, &m, kh2_anim::key_time(&m, fr as f32, false), kh2_anim::Options::default()).bones;
            let pose = r.pose(&body);
            assert!(pose.iter().all(|m| m.is_finite()), "motion {motion} frame {fr}: NaN");
            // Every own part as KH1's motion has it against its parent, but where the cloth was
            // swung off a thigh (the belt chain, the hem: `keep_clear` turns a chain about its
            // root, so its first joint turns against the root; the rest within float error).
            for &b in KH1_OWN.iter() {
                let have = own_relation(&pose[parents[b] as usize], &pose[b]).unwrap().0;
                let off = have.to_scale_rotation_translation().1.angle_between(want[b].unwrap().0.to_scale_rotation_translation().1).to_degrees();
                let first_joint = matches!(b, 11 | 73) || (249..=278).contains(&b) && (b - 249) % 4 == 0;
                let cloth = matches!(b, 11..=14 | 73 | 74 | 249..=278);
                assert!(first_joint || off < if cloth { 0.2 } else { 0.1 }, "motion {motion} frame {fr}: bone {b} {off}° off KH1's");
                // (Its distance from its parent as KH1's motion has it: KH1 keys some panels'.)
                let len = at(&pose[b]).distance(at(&pose[parents[b] as usize]));
                let kh1 = at(&want[b].unwrap().0).length();
                assert!((len - kh1).abs() < 0.01, "motion {motion} frame {fr}: bone {b} length {len} vs {kh1}");
            }
            match &first {
                Some(f0) => {
                    for &b in &[12usize, 28, 250] {
                        let p = parents[b] as usize;
                        let d0 = f0[p].inverse() * f0[b];
                        let d = pose[p].inverse() * pose[b];
                        moved = moved.max(d0.to_scale_rotation_translation().1.angle_between(d.to_scale_rotation_translation().1).to_degrees());
                    }
                }
                None => first = Some(pose.clone()),
            }
        }
        // Not rigid: KH1's own animation moves them.
        assert!(moved > 3.0, "motion {motion}: moved {moved}°");
    }
    // Cleared: back to the bind's relation.
    r.set_own(|_| None);
    let body = kh2_anim::Pose::bind(&f.kh2).bones;
    let pose = r.pose(&body);
    let b1 = kh2_anim::Pose::bind(&f.kh1).bones;
    for &b in &[12usize, 28, 87] {
        let p = parents[b] as usize;
        let rel = (pose[p].inverse() * pose[b]).to_scale_rotation_translation().1;
        let bind = (b1[p].inverse() * b1[b]).to_scale_rotation_translation().1;
        assert!(rel.angle_between(bind) < 2e-3, "bone {b}: {}", rel.angle_between(bind));
    }
}
