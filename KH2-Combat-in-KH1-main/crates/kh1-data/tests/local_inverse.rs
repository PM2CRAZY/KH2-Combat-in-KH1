//! KH1 poses on KH2's skeleton (`kh1_data::inverse`): a KH2 motion taken to KH1's skeleton
//! (`retarget`) and back lands its joints near where KH2 had them. Needs files from your own
//! installs, skipped without them (as `local_secondary.rs`).

use glam::{Mat4, Vec3};
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
fn kh2_motions_survive_the_round_trip() {
    let Some((kh1, sora)) = files() else { return };
    let rig = sora.rig.expect("KH2 rig");
    let r = kh1_data::retarget::Retarget::new(&kh1, &rig.skeleton);
    let weapon = kh1_data::retarget::GRIP_BONE;
    let inv = kh1_data::inverse::Inverse::new(&kh1, &rig.skeleton, &r, |i| i < weapon && r.follows_one_part(i));
    eprintln!("{} of {} KH2 bones turn with a measured part (ratio {:.3})", inv.mapped(), rig.skeleton.len(), r.hip_ratio());
    // Hips, knees, ankles, shoulders, elbows, wrists (both sides), the head.
    let joints = [11usize, 12, 13, 40, 41, 42, 76, 79, 80, 168, 171, 172, 141];
    let mut worst: f32 = 0.0;
    for motion in [0u16, 2, 151, 181] {
        let m = &rig.motions[&motion];
        for f in [0.0f32, 9.0, 21.0] {
            let p = kh2_anim::evaluate(&rig.skeleton, m, kh2_anim::key_time(m, f, true), Default::default()).bones;
            let back = inv.pose(&r.pose(&p));
            // Relative to the hips (KH2 bone 0): KH1's bone lengths differ, the directions don't.
            let e = joints.iter().map(|&j| ((at(&back[j]) - at(&back[0])) - (at(&p[j]) - at(&p[0]))).length()).fold(0.0f32, f32::max);
            let each: Vec<String> = joints.iter().map(|&j| format!("{j}:{:.0}", ((at(&back[j]) - at(&back[0])) - (at(&p[j]) - at(&p[0]))).length())).collect();
            eprintln!("motion {motion} frame {f}: worst joint {e:.1} units ({})", each.join(" "));
            worst = worst.max(e);
            assert!(back.iter().all(|m| m.is_finite()));
        }
    }
    // KH2 Sora is ~155 units tall: within ~8 of KH2's own places reads as the same pose (the
    // spine's middle follows the pelvis or the chest, not a blend).
    assert!(worst < 8.0, "worst joint {worst:.1} units off");
}
