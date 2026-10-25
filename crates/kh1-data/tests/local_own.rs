//! KH1's shorts and coat under KH2's poses, as KH2 draws its own: the shorts rigid on KH1's
//! thigh (their bind relation), with nothing of KH1's own animation reaching them
//! (`retarget::KH1_OWN` is empty); the coat turning as KH2's coat ring does against KH2's chest (`retarget::coat_kh2`). Needs files from your own installs, skipped without them: KH1's
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
fn kh1_s_shorts_hang_on_the_thigh_under_kh2_s_poses() {
    let Some(f) = files() else { return };
    assert!(KH1_OWN.is_empty(), "nothing of KH1's own animation goes on KH2's poses");
    let mut r = Retarget::new(&f.kh1, &f.kh2);
    r.set_rest(&f.kh1_motions[1].as_ref().unwrap().apply(&f.kh1, 0.0));
    let parents: Vec<i16> = f.kh1.iter().map(|b| b.parent).collect();
    let b1 = kh2_anim::Pose::bind(&f.kh1).bones;
    // The shorts' panels: four a leg (root, two joints, tip), on KH1's thighs 17 / 43.
    let shorts: Vec<usize> = (26..=41).chain(52..=67).collect();
    // KH2's run (2) on KH1's run (slot 5), KH2's first swing (151) on KH1's (98): KH1's own
    // motion offered to `set_own` changes nothing.
    for (motion, slot) in [(2usize, 5usize), (151, 98)] {
        let m = f.kh2_motions.slot_for(motion, true, true).and_then(|s| s.anb.as_ref()).and_then(|a| a.pose.clone()).expect("KH2 motion");
        let t = f.kh1_motions[slot].as_ref().expect("KH1 motion");
        let frames = kh2_anim::length_frames(&m).round() as usize;
        for fr in 0..=frames {
            let tm = kh2_anim::Pose::bind(&t.apply(&f.kh1, fr as f32 * t.frames as f32 / frames as f32)).bones;
            let want: Vec<Option<(Mat4, Vec3)>> = (0..f.kh1.len()).map(|b| own_relation(&tm[(parents[b].max(0)) as usize], &tm[b])).collect();
            r.set_own(|b| want[b].map(|w| w.0));
            let body = kh2_anim::evaluate(&f.kh2, &m, kh2_anim::key_time(&m, fr as f32, false), kh2_anim::Options::default()).bones;
            let pose = r.pose(&body);
            assert!(pose.iter().all(|m| m.is_finite()), "motion {motion} frame {fr}: NaN");
            for &b in &shorts {
                let p = parents[b] as usize;
                let rel = (pose[p].inverse() * pose[b]).to_scale_rotation_translation().1;
                let bind = (b1[p].inverse() * b1[b]).to_scale_rotation_translation().1;
                let off = rel.angle_between(bind).to_degrees();
                assert!(off < 0.1, "motion {motion} frame {fr}: shorts bone {b} {off}° off its bind relation");
                let len = at(&pose[b]).distance(at(&pose[p]));
                let len0 = at(&b1[b]).distance(at(&b1[p]));
                assert!((len - len0).abs() < 0.01, "motion {motion} frame {fr}: shorts bone {b} length {len} vs {len0}");
            }
        }
    }
}

#[test]
fn kh1_s_coat_turns_as_kh2_s_coat_ring_under_kh2_s_poses() {
    let Some(f) = files() else { return };
    let mut r = Retarget::new(&f.kh1, &f.kh2);
    r.set_rest(&f.kh1_motions[1].as_ref().unwrap().apply(&f.kh1, 0.0));
    let parents: Vec<i16> = f.kh1.iter().map(|b| b.parent).collect();
    let b1 = kh2_anim::Pose::bind(&f.kh1).bones;
    let b2 = kh2_anim::Pose::bind(&f.kh2).bones;
    // The coat: the back centre 72-75 and the eight panels 248-279 (root, root + 1, two joints),
    // and the KH2 coat ring leaf each follows (under KH2's chest 68).
    let roots = [72usize, 248, 252, 256, 260, 264, 268, 272, 276];
    let ring = [70usize, 129, 131, 133, 135, 221, 223, 225, 227];
    let (chest, chest2) = (78, 68);
    let rot = |m: &Mat4| m.to_scale_rotation_translation().1.normalize();
    for motion in [0usize, 1, 2, 3, 151, 154, 181, 201] {
        let m = f.kh2_motions.slot_for(motion, true, true).and_then(|s| s.anb.as_ref()).and_then(|a| a.pose.clone()).expect("KH2 motion");
        let frames = kh2_anim::length_frames(&m).round() as usize;
        let mut first: Vec<Option<glam::Quat>> = vec![None; roots.len()];
        let mut swing = vec![0.0f32; roots.len()];
        for fr in 0..=frames {
            let body = kh2_anim::evaluate(&f.kh2, &m, kh2_anim::key_time(&m, fr as f32, false), kh2_anim::Options::default()).bones;
            let pose = r.pose(&body);
            assert!(pose.iter().all(|m| m.is_finite()), "motion {motion} frame {fr}: NaN");
            let c1 = rot(&pose[chest]) * rot(&b1[chest]).inverse();
            let c2 = rot(&body[chest2]) * rot(&b2[chest2]).inverse();
            for (k, &root) in roots.iter().enumerate() {
                // The root stays on the chest (KH2's ring roots are under its chest).
                let rel = (pose[chest].inverse() * pose[root]).to_scale_rotation_translation();
                let bind = (b1[chest].inverse() * b1[root]).to_scale_rotation_translation();
                assert!(rel.2.distance(bind.2) < 0.05 && rel.1.angle_between(bind.1).to_degrees() < 0.2, "motion {motion} frame {fr}: coat root {root} moved against the chest");
                // The joint turns from its bind against the chest as KH2's ring leaf does.
                let t1 = c1.inverse() * rot(&pose[root + 1]) * rot(&b1[root + 1]).inverse();
                let t2 = c2.inverse() * rot(&body[ring[k]]) * rot(&b2[ring[k]]).inverse();
                let off = t1.angle_between(t2).to_degrees();
                assert!(off < 0.3, "motion {motion} frame {fr}: coat joint {} {off}° off KH2's {}", root + 1, ring[k]);
                let q0 = *first[k].get_or_insert(t1);
                swing[k] = swing[k].max(t1.angle_between(q0).to_degrees());
                // The joints below hang from it as in the bind (KH2's panel is one bone).
                for b in [root + 2, root + 3] {
                    let p = parents[b] as usize;
                    let rel = (pose[p].inverse() * pose[b]).to_scale_rotation_translation().1;
                    let bind = (b1[p].inverse() * b1[b]).to_scale_rotation_translation().1;
                    assert!(rel.angle_between(bind).to_degrees() < 0.2, "motion {motion} frame {fr}: coat bone {b} off its bind relation");
                }
                for b in root + 1..=root + 3 {
                    let p = parents[b] as usize;
                    let len = at(&pose[b]).distance(at(&pose[p]));
                    let len0 = at(&b1[b]).distance(at(&b1[p]));
                    assert!((len - len0).abs() < 0.01, "motion {motion} frame {fr}: coat bone {b} length {len} vs {len0}");
                }
            }
        }
        // It moves: in the run and the swings every panel swings against the chest.
        if [2usize, 151, 181].contains(&motion) {
            for (k, &root) in roots.iter().enumerate() {
                assert!(swing[k] > 5.0, "motion {motion}: coat chain {root} swings only {}°", swing[k]);
            }
        }
    }
}
