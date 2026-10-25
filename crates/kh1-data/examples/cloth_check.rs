//! The test bake's hood, coat and shorts (`kh1_data::cloth`) on KH2's walk and run, battle and
//! field: how far each part swings against its carrier, how deep its joints go into the body
//! (the hood off the spine and head, the coat outside the shorts, the shorts off their thigh),
//! and the largest second difference of a joint's place against its carrier (pops), before the
//! cloth pass (as the bake was) and after. `docs/kh1-secondary-motion.md`, "Hood, coat and shorts
//! from KH2's hidden keys (test bake)".
//! `cargo run -p kh1-data --example cloth_check -- <xa_ex_0010.mdls> <xa_ex_0010.mset> <dir with KH2 files> [motion ...]`
//! (files extracted from your own installs; nothing is kept).

use glam::{Mat4, Quat, Vec3};
use kh1_data::cloth::{ClothBake, Parts};
use kh1_data::retarget::Retarget;
use kh1_data::secondary::Secondary;

fn at(m: &Mat4) -> Vec3 {
    m.w_axis.truncate()
}
fn rot(m: &Mat4) -> Quat {
    m.to_scale_rotation_translation().1.normalize()
}
fn seg(p: Vec3, a: Vec3, b: Vec3) -> (f32, f32) {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    ((p - (a + ab * t)).length(), t)
}

struct Group {
    name: &'static str,
    carrier: Vec<usize>,
    joints: Vec<usize>,
    checked: Vec<usize>,
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let kh1_motions = kh1_data::mset::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[2])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let ids: Vec<u16> = if a.len() > 3 { a[3..].iter().map(|s| s.parse().unwrap()).collect() } else { vec![1, 2] };
    let mut r = Retarget::new(&kh1, &rig.skeleton);
    r.set_rest(&kh1_motions[1].as_ref().unwrap().apply(&kh1, 0.0));
    r.set_rest_open(&kh1_motions[0].as_ref().unwrap().apply(&kh1, 0.0));
    let s = Secondary::new(&kh1, &rig.skeleton, rig.weapons.first().map(|w| w.skeleton.as_slice()));
    let coat_rest = match std::env::var("COAT_REST").as_deref() { Ok("chest") => kh1_data::cloth::CoatRest::Chest, Ok("hips") => kh1_data::cloth::CoatRest::Hips, _ => kh1_data::cloth::CoatRest::Ring };
    let c = ClothBake::with_coat(&kh1, &rig.skeleton, coat_rest);
    println!("shorts drivers (KH1 joint: KH2 leaf share):");
    for (j, by) in c.shorts_drivers() {
        println!("  {j}: {}", by.iter().map(|(k, w)| format!("{k} {:.0}%", w * 100.0)).collect::<Vec<_>>().join(", "));
    }
    let b1 = kh2_anim::Pose::bind(&kh1).bones;
    let coat: Vec<usize> = [72usize, 248, 252, 256, 260, 264, 268, 272, 276].to_vec();
    let shorts: Vec<usize> = [26usize, 30, 34, 38, 52, 56, 60, 64].to_vec();
    let groups = [
        Group { name: "hood", carrier: vec![78; 4], joints: vec![80, 81, 82, 83], checked: vec![81, 82, 83, 84] },
        Group { name: "coat", carrier: vec![78; 9], joints: coat.iter().map(|r| r + 1).collect(), checked: coat.iter().flat_map(|r| [r + 2, r + 3]).collect() },
        Group { name: "shorts", carrier: shorts.iter().map(|&r| if r < 52 { 17 } else { 43 }).collect(), joints: shorts.iter().map(|r| r + 1).collect(), checked: shorts.iter().flat_map(|r| [r + 2, r + 3]).collect() },
    ];
    let thighs = [(17usize, 18usize), (43, 44)];
    let shorts_r = |t: f32| if t < 0.5 { 6.4 + (11.2 - 6.4) * t / 0.5 } else { 11.2 };
    let mut failures = 0;
    // CLOTH_DUMP=<file>: each frame's KH1 bone places before and after, as JSON lines (plots).
    let mut dump = std::env::var_os("CLOTH_DUMP").map(|p| std::fs::File::create(p).unwrap());
    if let Some(d) = dump.as_mut() {
        use std::io::Write;
        writeln!(d, "{{\"parents\":{:?}}}", kh1.iter().map(|b| b.parent).collect::<Vec<_>>()).unwrap();
    }
    for field in [false, true] {
        for &id in &ids {
            let Some(m) = (if field { rig.field_motions.get(&id) } else { rig.motions.get(&id) }) else { continue };
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
            let t0 = std::time::Instant::now();
            let rep = c.bake(&mut poses, &bodies, Some(n), Parts::ALL);
            let ms = t0.elapsed().as_secs_f32() * 1000.0;
            if let Some(d) = dump.as_mut() {
                use std::io::Write;
                for (f, (b, p)) in before.iter().zip(&poses).enumerate() {
                    let pts = |v: &[Mat4]| v.iter().map(|m| { let q = at(m); format!("[{:.2},{:.2},{:.2}]", q.x, q.y, q.z) }).collect::<Vec<_>>().join(",");
                    writeln!(d, "{{\"motion\":{id},\"field\":{field},\"frame\":{f},\"before\":[{}],\"after\":[{}]}}", pts(b), pts(p)).unwrap();
                }
            }
            println!("KH2 {id} ({}) {n} frames: {rep:?}, {ms:.0} ms", if field { "field" } else { "battle" });
            for g in &groups {
                let stat = |ps: &[Vec<Mat4>]| {
                    // Swing: each joint's turn against its carrier, range within the motion.
                    let mut swing = 0.0f32;
                    for (k, &j) in g.joints.iter().enumerate() {
                        let rel: Vec<Quat> = ps.iter().map(|p| rot(&p[g.carrier[k]]).inverse() * rot(&p[j])).collect();
                        for x in &rel {
                            for y in &rel {
                                swing = swing.max(x.angle_between(*y).to_degrees());
                            }
                        }
                    }
                    // Inside: worst depth.
                    let mut inside = 0.0f32;
                    for p in ps {
                        for &j in &g.checked {
                            let q = at(&p[j]);
                            let d = match g.name {
                                "hood" => 7.0 - seg(q, at(&p[71]), at(&p[78])).0,
                                "coat" => thighs.iter().map(|&(h, k)| { let (d, t) = seg(q, at(&p[h]), at(&p[k])); let bd = seg(at(&b1[j]), at(&b1[h]), at(&b1[k])).0; (shorts_r(t) + 1.0).min(bd) - d }).fold(f32::MIN, f32::max).max(seg(at(&b1[j]), at(&b1[3]), at(&b1[71])).0 - 5.0 - seg(q, at(&p[3]), at(&p[71])).0),
                                _ => { let (h, k) = if j < 52 { (17, 18) } else { (43, 44) }; let bd = seg(at(&b1[j]), at(&b1[h]), at(&b1[k])).0; bd * 0.95 - seg(q, at(&p[h]), at(&p[k])).0 }
                            };
                            inside = inside.max(d);
                        }
                    }
                    // Pops: second difference against the carrier (round the loop).
                    let mut pop = 0.0f32;
                    for (k, &j) in g.checked.iter().enumerate() {
                        let car = g.carrier[(k / (g.checked.len() / g.carrier.len()).max(1)).min(g.carrier.len() - 1)];
                        let loc: Vec<Vec3> = ps[..n].iter().map(|p| p[car].inverse().transform_point3(at(&p[j]))).collect();
                        for f in 0..n {
                            let (pv, nx) = (loc[(f + n - 1) % n], loc[(f + 1) % n]);
                            pop = pop.max((pv - 2.0 * loc[f] + nx).length());
                        }
                    }
                    (swing, inside, pop)
                };
                let (s0, i0, p0) = stat(&before);
                let (s1, i1, p1) = stat(&poses);
                println!("  {:6}: swing {s0:5.1} -> {s1:5.1} deg, deepest inside {i0:5.2} -> {i1:5.2}, pops {p0:5.2} -> {p1:5.2}", g.name);
            }
            // Shorts tips: how far out from their thigh's axis, against the bind.
            for &r0 in &shorts {
                let (h, k) = if r0 < 52 { (17, 18) } else { (43, 44) };
                let tip = r0 + 3;
                let bd = seg(at(&b1[tip]), at(&b1[h]), at(&b1[k])).0;
                let ds: Vec<f32> = poses.iter().map(|p| seg(at(&p[tip]), at(&p[h]), at(&p[k])).0).collect();
                let sw: f32 = poses.iter().map(|p| (rot(&p[h]).inverse() * rot(&p[r0 + 1])).angle_between(rot(&b1[h]).inverse() * rot(&b1[r0 + 1])).to_degrees()).fold(0.0, f32::max);
                println!("    panel {r0}: tip off thigh bind {bd:.1}, min {:.1} max {:.1}; turn from bind max {sw:.0}", ds.iter().cloned().fold(f32::MAX, f32::min), ds.iter().cloned().fold(0.0, f32::max));
            }
            for &r0 in &coat {
                let tip = r0 + 3;
                let sw: f32 = poses.iter().map(|p| (rot(&p[78]).inverse() * rot(&p[r0 + 1])).angle_between(rot(&before[0][78]).inverse() * rot(&before[0][r0 + 1])).to_degrees()).fold(0.0, f32::max);
                let pushes: f32 = poses.iter().zip(&before).map(|(p, q)| (rot(&p[r0 + 1]) * rot(&q[r0 + 1]).inverse()).angle_between(Quat::IDENTITY).to_degrees()).fold(0.0, f32::max);
                let _ = tip;
                println!("    coat {r0}: changed by the pass at most {pushes:.0} deg ({sw:.0} from frame 0 before)");
            }
            // Baked as the hook bakes it (on KH1's walk 2 / run 5 as the template): KH1's FK
            // from the block against the pose.
            let mset = std::fs::read(&a[1]).unwrap();
            let slot = if id == 1 { 2 } else { 5 };
            if let Ok(t) = kh1_data::slot::motion_bytes(&mset, slot) {
                for (name, ps) in [("before", &before), ("after", &poses)] {
                    match kh1_data::bake::baked(&t, n as u32, &[], &kh1, ps, &[], &[]) {
                        Ok((_, r)) => println!("  bake {name}: within {:.3} (worst bone {}), {} keyed channels, {} keys", r.max_error, r.worst_bone, r.keyed_channels, r.keys),
                        Err(e) => { println!("  bake {name}: {e}"); failures += 1; }
                    }
                }
            }
            // Lengths kept, finite.
            let len_err = |ps: &[Vec<Mat4>]| { let mut w = (0.0f32, 0usize); for p in ps { for (i, b) in kh1.iter().enumerate() { if b.parent >= 0 { let pp = b.parent as usize; let e = (at(&p[i]).distance(at(&p[pp])) - at(&b1[i]).distance(at(&b1[pp]))).abs(); if e > w.0 { w = (e, i); } } } } w };
            println!("  length change before {:?}", len_err(&before));
            let mut worst_len = 0.0f32;
            let changed = poses.iter().zip(&before).map(|(p, q)| (0..kh1.len()).filter(|&i| kh1[i].parent >= 0).map(|i| { let pp = kh1[i].parent as usize; ((at(&p[i]).distance(at(&p[pp]))) - at(&q[i]).distance(at(&q[pp]))).abs() }).fold(0.0f32, f32::max)).fold(0.0f32, f32::max);
            println!("  length change by the cloth pass {changed:.4}");
            for p in &poses {
                for (i, b) in kh1.iter().enumerate() {
                    if b.parent >= 0 {
                        let pp = b.parent as usize;
                        worst_len = worst_len.max((at(&p[i]).distance(at(&p[pp])) - at(&b1[i]).distance(at(&b1[pp]))).abs());
                    }
                }
                if !p.iter().all(|m| m.is_finite()) {
                    failures += 1;
                }
            }
            println!("  worst length change {worst_len:.4}");
            if changed > 0.01 {
                failures += 1;
            }
            // The loop closes: the last frame is the first.
            let seam = poses[0].iter().zip(&poses[n]).map(|(x, y)| at(x).distance(at(y))).fold(0.0f32, f32::max);
            let seam0 = before[0].iter().zip(&before[n]).map(|(x, y)| at(x).distance(at(y))).fold(0.0f32, f32::max);
            println!("  loop seam (frame 0 vs {n}): {seam0:.3} -> {seam:.3}");
        }
    }
    // KH1's own walk / run (plain FK of its own motions: its own coat and shorts against the same
    // measures; its IK helpers aren't evaluated, Assumption).
    for slot in [2usize, 3, 5, 6] {
        let Some(m) = kh1_motions.get(slot).and_then(|m| m.as_ref()) else { continue };
        let ps: Vec<Vec<Mat4>> = (0..m.frames.max(1)).map(|f| kh2_anim::Pose::bind(&m.apply(&kh1, f as f32)).bones).collect();
        let mut hips = 0.0f32;
        let mut thigh = 0.0f32;
        for p in &ps {
            for &r0 in &coat {
                for j in [r0 + 2, r0 + 3] {
                    let q = at(&p[j]);
                    hips = hips.max(seg(at(&b1[j]), at(&b1[3]), at(&b1[71])).0 - seg(q, at(&p[3]), at(&p[71])).0);
                    for &(h, k) in &thighs {
                        let (d, t) = seg(q, at(&p[h]), at(&p[k]));
                        let bd = seg(at(&b1[j]), at(&b1[h]), at(&b1[k])).0;
                        thigh = thigh.max((shorts_r(t) + 1.0).min(bd) - d);
                    }
                }
            }
        }
        println!("KH1's own slot {slot}: its coat's deepest nearer the hips' axis than the bind {hips:.1}, inside the shorts {thigh:.1}");
    }
    if failures > 0 {
        println!("{failures} failures");
        std::process::exit(1);
    }
}
