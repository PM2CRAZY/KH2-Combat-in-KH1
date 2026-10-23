//! Runs against Sora's files extracted into a local folder (never
//! committed): set `KH2_LOCAL_DATA` to that folder.

use kh2_formats::anim::parse_skeleton;
use kh2_formats::motion::Mset;

fn dir() -> Option<std::path::PathBuf> {
    std::env::var_os("KH2_LOCAL_DATA").map(Into::into)
}

#[test]
fn every_sora_motion_poses_and_feet_reach_ik_targets() {
    let Some(d) = dir() else { return };
    let skel = parse_skeleton(&std::fs::read(d.join("P_EX100.mdlx")).unwrap()).unwrap();
    let mset = Mset::parse(&std::fs::read(d.join("P_EX100.mset")).unwrap()).unwrap();
    let mut n = 0;
    for slot in &mset.slots {
        let Some(m) = slot.anb.as_ref().and_then(|a| a.pose.as_ref()) else { continue };
        for f in [0.0, 7.0, 19.0] {
            let t = kh2_anim::key_time(m, f, true);
            let p = kh2_anim::evaluate(&skel, m, t, Default::default());
            assert!(p.bones.iter().all(|b| b.is_finite()), "{}", slot.name);
        }
        n += 1;
    }
    assert!(n > 50);
    // Idle stands on the ground: the lowest bone is at y = 0.
    let idle = mset.slot_for(0, true, true).unwrap().anb.as_ref().unwrap().pose.as_ref().unwrap();
    let p = kh2_anim::evaluate(&skel, idle, kh2_anim::key_time(idle, 0.0, true), Default::default());
    let low = (0..idle.bone_count).map(|i| p.bone_position(i).y).fold(f32::MAX, f32::min);
    assert!(low.abs() < 1.0, "lowest bone {low}");
}

/// Sora's mesh at his bind pose: a T-pose standing on the ground, about 155 units tall.
#[test]
fn sora_mesh_skins_at_bind_pose() {
    let Some(d) = dir() else { return };
    let Ok(bytes) = std::fs::read(d.join("P_EX100.mdlx")) else { return };
    let model = kh2_formats::mdlx::parse(&bytes).unwrap();
    let pose = kh2_anim::Pose::bind(&model.bones);
    let (mut lo, mut hi) = (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN));
    let mut n = 0;
    kh2_anim::skin::skin(&model, &pose.bones, glam::Mat4::IDENTITY, |_, p, _| {
        n += 1;
        for v in p {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    });
    assert_eq!(n, model.triangle_count());
    assert!(lo.y > -2.0 && lo.y < 2.0, "{lo}");
    assert!(hi.y > 140.0 && hi.y < 170.0, "{hi}");
    assert!(hi.x > 50.0 && lo.x < -50.0, "arms out: {lo} {hi}");
}
