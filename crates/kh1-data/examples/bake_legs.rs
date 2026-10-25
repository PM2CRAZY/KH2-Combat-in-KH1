//! KH1's own leg rig on baked KH2 motions (`kh1_data::legs`, `docs/kh1-anim-baking.md` §8): which
//! of KH1 Sora's motions carry the rig, and for KH2 motions baked on KH1 templates, how close
//! KH1's pass (its leg IK on flat ground, `legs::build`) comes to the retargeted pose, with the
//! legs on KH1's IK and as plain FK.
//! `cargo run -p kh1-data --example bake_legs -- <xa_ex_0010.mdls> <xa_ex_0010.mset> <KH2 dir> [kh2:template ...]`
//! (files extracted from your own installs; nothing is kept).
use kh1_data::{bake, legs, mset, slot};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let file = std::fs::read(&a[1]).unwrap();
    let motions = mset::parse(&file).unwrap();
    // Every slot: the legs found in its joint words.
    let mut without = Vec::new();
    for s in 0..motions.len() {
        let Ok(b) = slot::motion_bytes(&file, s) else { continue };
        let u = |at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
        let (w, hc, ha) = (u(0x14) as usize, u(0x2C) as usize, u(0x30) as usize);
        let words: Vec<u32> = (0..kh1.len()).map(|i| u(w + i * 4)).collect();
        let parents: Vec<i32> = (0..hc).map(|k| u(ha + k * 40 + 0xC) as i32).collect();
        let l = legs::find(&words, &kh1, &parents);
        if l.len() != 2 {
            without.push((s, l.len()));
        }
    }
    println!("{} slots, with fewer or more than two KH1 legs: {without:?}", motions.len());

    let Some(dir) = a.get(2) else { return };
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(dir)).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let mut r = kh1_data::retarget::Retarget::new(&kh1, &rig.skeleton);
    if let Some(Some(m)) = motions.get(1) {
        r.set_rest(&m.apply(&kh1, 0.0));
    }
    let pairs: Vec<(u16, usize)> = if a.len() > 3 {
        a[3..].iter().map(|p| { let (x, y) = p.split_once(':').unwrap(); (x.parse().unwrap(), y.parse().unwrap()) }).collect()
    } else {
        vec![(1, 2), (2, 5), (151, 98), (152, 99), (154, 101), (181, 104), (4, 10), (3, 8), (201, 12), (9, 58), (56, 55)]
    };
    for (motion, template) in pairs {
        let Some(m) = rig.motions.get(&motion) else { println!("KH2 {motion}: none"); continue };
        let frames = (kh2_anim::length_frames(m).round() as u32).clamp(1, 250);
        let poses: Vec<Vec<glam::Mat4>> = (0..=frames).map(|f| r.pose(&kh2_anim::evaluate(&rig.skeleton, m, kh2_anim::key_time(m, f as f32, false), kh2_anim::Options::default()).bones)).collect();
        let t = slot::motion_bytes(&file, template).unwrap();
        for on in [true, false] {
            match bake::baked(&t, frames, &[], &kh1, &poses, &[], on) {
                Ok((block, rep)) => println!(
                    "KH2 {motion} on slot {template} ({frames} frames), legs {}: {} bytes, {} KH1 legs ({} helper keys), within {:.3} (bone {}); hip turned up to {:.1} deg, thigh / shin {:.2} deg",
                    if on { "KH1 IK" } else { "FK" },
                    block.len(),
                    rep.kh1_legs,
                    rep.leg_keys,
                    rep.max_error,
                    rep.worst_bone,
                    rep.leg_hip_turn.to_degrees(),
                    rep.leg_turn.to_degrees()
                ),
                Err(e) => println!("KH2 {motion} on slot {template}: {e}"),
            }
        }
    }
}
