//! KH1's shorts and coat under KH2's poses, as KH2 draws its own: the shorts rigid on KH1's
//! thigh (their bind relation), with nothing of KH1's own animation reaching them
//! (`retarget::KH1_OWN` is empty); the coat rigid on KH1's chest, as KH2's jacket is on its torso. Needs files from your own installs, skipped without them: KH1's
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
fn kh1_s_coat_moves_with_the_chest_under_kh2_s_poses() {
    let Some(f) = files() else { return };
    let mut r = Retarget::new(&f.kh1, &f.kh2);
    r.set_rest(&f.kh1_motions[1].as_ref().unwrap().apply(&f.kh1, 0.0));
    let parents: Vec<i16> = f.kh1.iter().map(|b| b.parent).collect();
    let b1 = kh2_anim::Pose::bind(&f.kh1).bones;
    // The coat: the back centre 72-75 and the eight panels 248-279, against the chest's 78.
    let coat: Vec<usize> = (72..=75).chain(248..=279).collect();
    let chest = 78;
    for motion in [0usize, 2, 3, 151, 154, 181, 201] {
        let m = f.kh2_motions.slot_for(motion, true, true).and_then(|s| s.anb.as_ref()).and_then(|a| a.pose.clone()).expect("KH2 motion");
        let frames = kh2_anim::length_frames(&m).round() as usize;
        for fr in 0..=frames {
            let body = kh2_anim::evaluate(&f.kh2, &m, kh2_anim::key_time(&m, fr as f32, false), kh2_anim::Options::default()).bones;
            let pose = r.pose(&body);
            assert!(pose.iter().all(|m| m.is_finite()), "motion {motion} frame {fr}: NaN");
            for &b in &coat {
                let rel = (pose[chest].inverse() * pose[b]).to_scale_rotation_translation();
                let bind = (b1[chest].inverse() * b1[b]).to_scale_rotation_translation();
                let off = rel.1.angle_between(bind.1).to_degrees();
                assert!(off < 0.2, "motion {motion} frame {fr}: coat bone {b} turned {off}° against the chest");
                let d = rel.2.distance(bind.2);
                assert!(d < 0.05, "motion {motion} frame {fr}: coat bone {b} moved {d} against the chest");
                let p = parents[b] as usize;
                let len = at(&pose[b]).distance(at(&pose[p]));
                let len0 = at(&b1[b]).distance(at(&b1[p]));
                assert!((len - len0).abs() < 0.01, "motion {motion} frame {fr}: coat bone {b} length {len} vs {len0}");
            }
        }
    }
}
