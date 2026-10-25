//! The cloth physics (`kh1_data::clothsim`) offline, the same code the game runs live: KH2's
//! motion (default its field run, 2) retargeted onto KH1 Sora as the bake draws it (root travel
//! out, KH2's own hair / hood / sleeves / necklace), moved straight ahead at `speed` a 60 Hz frame,
//! one KH2 frame a step, for `loops` loops; then each part's swing on the last loop against KH1's
//! own animation of that part in its matching move (`kh1_slot`, default 5, KH1's field run).
//! `docs/kh1-secondary-motion.md`, "Cloth physics".
//!
//! `cargo run -p kh1-data --example cloth_sim -- <xa_ex_0010.mdls> <xa_ex_0010.mset> <dir with KH2 files> [fit] [motion=2] [field=1] [kh1_slot=5] [loops=8] [speed=8]`
//! (files extracted from your own installs; nothing is kept). `fit`: searches each part's
//! stiffness, damping and weight for the swing nearest KH1's.

use glam::{Mat4, Quat, Vec3};
use kh1_data::clothsim::{ClothSim, Params, Part, Settings};

fn at(m: &Mat4) -> Vec3 {
    m.w_axis.truncate()
}
fn rot(m: &Mat4) -> Quat {
    m.to_scale_rotation_translation().1.normalize()
}

/// Per part: the mean over its links of the link's swing about its own mean direction in its
/// carrier's frame (rms, degrees), and the largest change of a link's direction in one frame.
fn swing(frames: &[Vec<Mat4>], links: &[(Part, usize, usize, usize)]) -> [(f32, f32); 4] {
    let mut out = [(0.0f32, 0.0f32); 4];
    let mut count = [0usize; 4];
    for &(part, carrier, b, tip) in links {
        let dirs: Vec<Vec3> = frames.iter().map(|p| rot(&p[carrier]).inverse() * (at(&p[tip]) - at(&p[b])).normalize_or_zero()).collect();
        let mean = dirs.iter().sum::<Vec3>().normalize_or_zero();
        let rms = (dirs.iter().map(|d| d.angle_between(mean).to_degrees().powi(2)).sum::<f32>() / dirs.len().max(1) as f32).sqrt();
        let step = dirs.windows(2).map(|w| w[0].angle_between(w[1]).to_degrees()).fold(0.0, f32::max);
        let i = part as usize;
        out[i].0 += rms;
        out[i].1 = out[i].1.max(step);
        count[i] += 1;
    }
    for i in 0..4 {
        out[i].0 /= count[i].max(1) as f32;
    }
    out
}

struct Run {
    /// The last loop's poses after the physics.
    last: Vec<Vec<Mat4>>,
    /// The largest difference of a tip between the last two loops at the same frame (the loop
    /// has settled into a cycle when small).
    seam: f32,
}

fn run(sim: &mut ClothSim, input: &[Vec<Mat4>], set: &Settings, loops: usize, speed: f32) -> Run {
    sim.reset();
    let n = input.len();
    let links = sim.links();
    let mut prev: Vec<Vec<Mat4>> = Vec::new();
    let mut last = Vec::new();
    for l in 0..loops {
        last.clear();
        for (f, p) in input.iter().enumerate() {
            let step = (l * n + f) as f32;
            let to_world = Mat4::from_translation(Vec3::new(0.0, 0.0, speed * step));
            let mut pose = p.clone();
            sim.step(&mut pose, &to_world, if l == 0 && f == 0 { 0 } else { 1 }, set);
            last.push(pose);
        }
        if l + 1 < loops {
            prev = last.clone();
        }
    }
    let seam = last.iter().zip(&prev).flat_map(|(a, b)| links.iter().map(move |&(_, _, _, t)| at(&a[t]).distance(at(&b[t])))).fold(0.0, f32::max);
    Run { last, seam }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let fit = a.iter().any(|s| s == "fit");
    let num: Vec<f32> = a[3..].iter().filter(|s| *s != "fit").filter_map(|s| s.parse().ok()).collect();
    let motion = num.first().map_or(2, |v| *v as u16);
    let field = num.get(1).map_or(true, |v| *v != 0.0);
    let kh1_slot = num.get(2).map_or(5, |v| *v as usize);
    let loops = num.get(3).map_or(8, |v| *v as usize);
    let speed = num.get(4).copied().unwrap_or(8.0);
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let kh1_motions = kh1_data::mset::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[2])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let mut r = kh1_data::retarget::Retarget::new(&kh1, &rig.skeleton);
    r.set_rest(&kh1_motions[1].as_ref().unwrap().apply(&kh1, 0.0));
    r.set_rest_open(&kh1_motions[0].as_ref().unwrap().apply(&kh1, 0.0));
    let sec = kh1_data::secondary::Secondary::new(&kh1, &rig.skeleton, rig.weapons.first().map(|w| w.skeleton.as_slice()));
    let clip = if field { rig.field_motions.get(&motion) } else { rig.motions.get(&motion) }.or_else(|| rig.motions.get(&motion)).expect("motion");
    let len = kh2_anim::length_frames(clip).round().max(1.0) as usize;
    let roots = sora.character.moveset.clips.get(&motion).map(|c| c.root.clone());
    // KH2's motion as the bake draws it, one frame a step (the loop's last frame is its first).
    let input: Vec<Vec<Mat4>> = (0..len)
        .map(|f| {
            let mut body = kh2_anim::evaluate(&rig.skeleton, clip, kh2_anim::key_time(clip, f as f32, true), kh2_anim::Options::default()).bones;
            if let Some(root) = roots.as_ref().and_then(|r| r.get(f.min(r.len().saturating_sub(1)))) {
                let un = Mat4::from_translation(-Vec3::from(*root));
                for m in body.iter_mut() {
                    *m = un * *m;
                }
            }
            let mut p = r.pose(&body);
            sec.apply(&mut p, &body, None, kh1_data::secondary::Parts { hair: true, hood: true, chain: false, sleeves: true, necklace: true });
            p
        })
        .collect();
    let rest = kh1_motions.get(kh1_slot).and_then(|m| m.as_ref());
    let mut sim = ClothSim::new(&kh1, rest);
    let links = sim.links();
    println!("KH2 motion {motion}{} ({len} frames), {} chains, rest from KH1 slot {kh1_slot}, {loops} loops at {speed} a frame", if field { " (field)" } else { "" }, sim.chains());
    // KH1's own: its matching move, FK (each part's keys against its carrier are exact).
    let kh1_own: Vec<Vec<Mat4>> = rest.map_or(Vec::new(), |m| (0..m.frames).map(|f| kh2_anim::Pose::bind(&m.apply(&kh1, f as f32)).bones).collect());
    let target = swing(&kh1_own, &links);
    let before = swing(&input, &links);
    let mut report = |set: &Settings| {
        let r = run(&mut sim, &input, set, loops, speed);
        (swing(&r.last, &links), r.seam)
    };
    let mut set = Settings::TUNED;
    if fit {
        for (pi, part) in Part::ALL.iter().enumerate() {
            let mut best: Option<(f32, Params)> = None;
            for &k in &[0.02f32, 0.04, 0.06, 0.08, 0.1, 0.15, 0.2, 0.3, 0.45] {
                for &d in &[0.05f32, 0.1, 0.15, 0.2, 0.3, 0.45] {
                    for &w in &[0.25f32, 0.5, 0.75, 1.0] {
                        let mut s = set;
                        s.parts[pi] = Params { stiffness: k, damping: d, weight: w, swing: set.parts[pi].swing };
                        let (sw, seam) = report(&s);
                        let (got, step) = sw[pi];
                        // As near KH1's swing as can be, no choppier than KH1's own, settled.
                        let cost = (got - target[pi].0).powi(2) + (step - target[pi].1).max(0.0).powi(2) * 0.5 + seam.max(0.0) * 0.1;
                        if best.as_ref().map_or(true, |b| cost < b.0) {
                            best = Some((cost, s.parts[pi]));
                        }
                    }
                }
            }
            if let Some((c, p)) = best {
                set.parts[pi] = p;
                println!("fit {}: {p:?} (cost {c:.2})", part.name());
            }
        }
    }
    let (after, seam) = report(&set);
    println!("settings id {}", set.id());
    println!("{:<11} {:>22} {:>22} {:>22}", "part", "KH1's own (rms / step)", "before physics", "with physics");
    for (i, p) in Part::ALL.iter().enumerate() {
        println!("{:<11} {:>12.1} / {:>6.1} {:>14.1} / {:>5.1} {:>14.1} / {:>5.1}", p.name(), target[i].0, target[i].1, before[i].0, before[i].1, after[i].0, after[i].1);
    }
    println!("loop seam (largest tip difference between the last two loops): {seam:.3}");
}
