//! KH1 Sora's hair, hood and keychain on KH2's real motions (`kh1_data::secondary`). Needs
//! files from your own installs, skipped without them: KH1's `xa_ex_0010.mdls` (`KH1_MDLS`, or
//! `local/re/xa_ex_0010.mdls`) and KH2's Sora files in `local/re` (or `KH2_RE_DIR`).

use glam::{Mat4, Vec3};
use kh1_data::secondary::{Parts, Secondary};
use std::path::PathBuf;

fn files() -> Option<(Vec<kh2_formats::anim::Bone>, kh2_data::sora::SoraData)> {
    let re = std::env::var_os("KH2_RE_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../local/re"));
    let mdls = std::env::var_os("KH1_MDLS").map(PathBuf::from).unwrap_or_else(|| re.join("xa_ex_0010.mdls"));
    let (Ok(m), true) = (std::fs::read(&mdls), re.join("P_EX100.mdlx").is_file()) else {
        eprintln!("skipped: no {} or no KH2 files in {}", mdls.display(), re.display());
        return None;
    };
    Some((kh1_data::mdls::parse_skeleton(&m).ok()?, kh2_data::sora::load_sora_dir(&re).ok()?))
}

fn at(m: &Mat4) -> Vec3 {
    m.w_axis.truncate()
}

#[test]
fn kh1_parts_follow_kh2_motions() {
    let Some((kh1, sora)) = files() else { return };
    let rig = sora.rig.expect("KH2 rig");
    let wr = &rig.weapons[0];
    let r = kh1_data::retarget::Retarget::new(&kh1, &rig.skeleton);
    let s = Secondary::new(&kh1, &rig.skeleton, Some(&wr.skeleton));
    // Fourteen KH1 spikes, the hood's four joints, the nine chain links; the two chains bind
    // at the same places.
    assert_eq!((s.hair().len(), s.hood().len(), s.chain().len()), (14, 4, 9));
    let (b1, wb) = (kh2_anim::Pose::bind(&kh1).bones, kh2_anim::Pose::bind(&wr.skeleton).bones);
    for l in s.chain() {
        assert!(at(&b1[l.kh1]).distance(at(&wb[l.kh2])) < 1.0, "{l:?}");
    }
    for motion in [0u16, 2, 151, 181] {
        let m = &rig.motions[&motion];
        let wm = wr.motions.get(&motion).or_else(|| wr.motions.get(&0)).unwrap();
        let mut first: Option<Vec<Vec3>> = None;
        let mut moved = [0f32; 3];
        for f in (0..kh2_anim::length_frames(m) as usize).step_by(4) {
            let body = kh2_anim::evaluate(&rig.skeleton, m, kh2_anim::key_time(m, f as f32, false), kh2_anim::Options::default()).bones;
            let hand = body[wr.bone.unwrap()];
            let weapon: Vec<Mat4> = kh2_anim::evaluate(&wr.skeleton, wm, kh2_anim::key_time(wm, f as f32, false), kh2_anim::Options::default()).bones.iter().map(|w| hand * *w).collect();
            let mut pose = r.pose(&body);
            let before = pose.clone();
            let rep = s.apply(&mut pose, &body, Some(&weapon), Parts::ALL);
            assert_eq!(rep.parts, Parts::ALL);
            assert!(pose.iter().all(|m| m.is_finite()), "motion {motion} frame {f}: NaN");
            for i in 1..pose.len() {
                if let Some(p) = s.parent(i) {
                    let d = at(&pose[i]).distance(at(&pose[p])) - at(&before[i]).distance(at(&before[p]));
                    assert!(d.abs() < 0.01, "motion {motion} frame {f}: bone {i} length changed by {d}");
                }
            }
            // Every link along the direction its KH2 match gives: the chain always, the hair and
            // hood unless held back off the head or the back.
            for (l, want) in s.expected(&before, &body, Some(&weapon)) {
                if rep.clamped == 0 || s.chain().contains(&l) {
                    let have = (at(&pose[l.tip]) - at(&pose[l.kh1])).normalize();
                    assert!(have.angle_between(want).to_degrees() < 2.0, "motion {motion} frame {f}: {l:?}");
                }
            }
            let rel: Vec<Vec3> = [(s.hair(), 151), (s.hood(), 78), (s.chain(), 101)]
                .iter()
                .flat_map(|(ls, refb)| ls.iter().map(move |l| (l, *refb)))
                .map(|(l, refb)| pose[refb].inverse().transform_vector3(at(&pose[l.tip]) - at(&pose[l.kh1])).normalize())
                .collect();
            match &first {
                Some(f0) => {
                    let (h, d) = (s.hair().len(), s.hood().len());
                    for (k, (a, b)) in f0.iter().zip(&rel).enumerate() {
                        let g = if k < h { 0 } else if k < h + d { 1 } else { 2 };
                        moved[g] = moved[g].max(a.angle_between(*b).to_degrees());
                    }
                }
                None => first = Some(rel),
            }
        }
        // Not rigid: each part moves within the motion.
        assert!(moved.iter().all(|m| *m > 3.0), "motion {motion}: moved {moved:?}");
    }
}
