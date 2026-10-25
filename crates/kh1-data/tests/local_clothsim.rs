//! The cloth physics (`kh1_data::clothsim`) on KH1 Sora's own skeleton: a still pose stays in its
//! rest, a steady move moves nothing, a sudden start swings the parts and they settle again, a
//! warp starts over, bone lengths are kept, and no step on a held frame. Needs KH1's
//! `xa_ex_0010.mdls` / `.mset` from your own install (`KH1_MDLS` / `KH1_MSET`, or `local/re/`);
//! skipped without them.

use glam::{Mat4, Vec3};
use kh1_data::clothsim::{ClothSim, Settings};
use std::path::PathBuf;

fn at(m: &Mat4) -> Vec3 {
    m.w_axis.truncate()
}

fn files() -> Option<(Vec<kh2_formats::anim::Bone>, Vec<Option<kh1_data::mset::Motion>>)> {
    let re = std::env::var_os("KH2_RE_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../local/re"));
    let mdls = std::env::var_os("KH1_MDLS").map(PathBuf::from).unwrap_or_else(|| re.join("xa_ex_0010.mdls"));
    let mset = std::env::var_os("KH1_MSET").map(PathBuf::from).unwrap_or_else(|| re.join("xa_ex_0010.mset"));
    let (Ok(a), Ok(b)) = (std::fs::read(&mdls), std::fs::read(&mset)) else {
        eprintln!("skipped: no KH1 files at {}", mdls.display());
        return None;
    };
    Some((kh1_data::mdls::parse_skeleton(&a).ok()?, kh1_data::mset::parse(&b).ok()?))
}

#[test]
fn cloth_physics_rests_swings_and_settles() {
    let Some((kh1, motions)) = files() else { return };
    let mut sim = ClothSim::new(&kh1, motions.get(5).and_then(|m| m.as_ref()));
    assert_eq!(sim.chains(), 19, "9 coat chains, 8 shorts panels, the belt chain, the belt front");
    let bind = kh2_anim::Pose::bind(&kh1).bones;
    let links = sim.links();
    let set = Settings::TUNED;
    let lengths = |p: &[Mat4]| links.iter().map(|&(_, _, b, t)| at(&p[b]).distance(at(&p[t]))).collect::<Vec<f32>>();
    // In the rest (the first frame) and still: no swing.
    let mut first = bind.clone();
    sim.step(&mut first, &Mat4::IDENTITY, 0, &set);
    let rest_len = lengths(&first);
    // (Each tip's place against the model from one frame to the next.)
    let tips = |p: &[Mat4]| links.iter().map(|&(_, _, _, t)| at(&p[t])).collect::<Vec<Vec3>>();
    let moved = |a: &[Vec3], b: &[Vec3]| a.iter().zip(b).map(|(x, y)| x.distance(*y)).fold(0.0f32, f32::max);
    let mut p = bind.clone();
    let mut was = tips(&first);
    // (Its first steps push what starts inside the body out.)
    for i in 0..300 {
        p = bind.clone();
        sim.step(&mut p, &Mat4::IDENTITY, 1, &set);
        let now = tips(&p);
        assert!(i < 240 || moved(&now, &was) < 0.01, "still at {i}: {}", moved(&now, &was));
        was = now;
    }
    // A steady move: nothing (no acceleration after its first step).
    for f in 1..300 {
        p = bind.clone();
        sim.step(&mut p, &Mat4::from_translation(Vec3::new(0.0, 0.0, 8.0 * f as f32)), 1, &set);
        let now = tips(&p);
        if f > 240 {
            assert!(moved(&now, &was) < 0.05, "steady at {f}: {}", moved(&now, &was));
        }
        was = now;
    }
    let steady = was.clone();
    // A sudden stop: the parts swing, then settle where they were.
    let stop = Mat4::from_translation(Vec3::new(0.0, 0.0, 8.0 * 299.0));
    let mut most: f32 = 0.0;
    for _ in 0..20 {
        p = bind.clone();
        sim.step(&mut p, &stop, 1, &set);
        most = most.max(moved(&tips(&p), &steady));
    }
    assert!(most > 1.0, "a stop swings them: {most}");
    for (a, b) in lengths(&p).iter().zip(&rest_len) {
        assert!((a - b).abs() < 0.05, "length kept: {a} vs {b}");
    }
    for _ in 0..900 {
        p = bind.clone();
        sim.step(&mut p, &stop, 1, &set);
    }
    let left = moved(&tips(&p), &steady);
    assert!(left < 0.5, "settled: {left}");
    // A held frame: no step, the same as before.
    let mut q = bind.clone();
    sim.step(&mut q, &stop, 0, &set);
    for (a, b) in q.iter().zip(&p) {
        assert!(at(a).distance(at(b)) < 1e-3);
    }
    // A warp: started over.
    p = bind.clone();
    assert!(sim.step(&mut p, &Mat4::from_translation(Vec3::new(5000.0, 0.0, 0.0)), 1, &set).restarted);
}
