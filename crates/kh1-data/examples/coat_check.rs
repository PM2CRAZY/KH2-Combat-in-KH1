//! The coat (KH1's jacket hem, carried by the chest as KH2's jacket) against the thighs and hips
//! over every KH2 motion, as it is and under candidate rules (research, `docs/kh1-secondary-motion.md`
//! "The coat against the thighs").
//! `<mdls> <KH2 dir> kh1 <xa_ex_0010.mset>`: KH1's own motions, how far its coat stands off its hips.
//! `<mdls> <KH2 dir> waist`: how KH2's waist bones (65 / 66 / 67) turn between the retarget's
//! pelvis and chest parts.
//! `<mdls> <KH2 dir> <letters> [clear w0 w1 w2 lean/thigh margin soft lead0 lead1 lead2 fade lstart step]`:
//! the coat as `Retarget::pose` makes it, then each lettered rule: w waist blend (KH2 66), h height
//! blend chest -> hips (+ front panels' thigh share), T thigh lead (swing share), B carry blend
//! (chest / hips places; b: the front panels' hips place a share toward their thigh's), L lift by the thigh's swing about the panel's hinge, Y the hips' twist, K carried by KH1's spine base 71 (its own parent), Q (with h) the
//! lower turn is 71's,
//! W hanging (yaw, `lean` of the lean), p hinge search push, s radial push out of the shorts
//! (C flat top, F clearance frozen per frame, R ramp at the hip end, G only about the panel's
//! hinge, S tangent while deep, H hips push, E eased), J per-motion lines, D / X / P traces.
//! Measures: joints nearer a thigh than in the bind, inside the shorts (radius 6.4 at the hip to
//! 11.2 from mid-thigh, KH1's panel bones in the bind), nearer the hips' up axis, distance from
//! the chest-rigid coat, and the second difference against the chest (pops) next to the knees'.
use glam::{Mat4, Quat, Vec3};
fn rot(m: &Mat4) -> Quat { m.to_scale_rotation_translation().1.normalize() }
fn seg(p: Vec3, a: Vec3, b: Vec3) -> (f32, Vec3) {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    let n = a + ab * t;
    ((p - n).length(), n)
}
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[1])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let r = kh1_data::retarget::Retarget::new(&kh1, &rig.skeleton);
    let mut ids: Vec<u16> = rig.motions.keys().copied().collect();
    ids.sort();
    if a.get(2).map(|s| s.as_str()) == Some("waist") {
        let bind2 = kh2_anim::Pose::bind(&rig.skeleton).bones;
        for bone in [65usize, 66, 67] {
            let mut ts = vec![];
            let mut res = vec![];
            for &id in &ids {
                let m = &rig.motions[&id];
                let n = kh2_anim::length_frames(m) as usize;
                for f in (0..n.max(1)).step_by(2) {
                    let kh2 = kh2_anim::evaluate(&rig.skeleton, m, kh2_anim::key_time(m, f as f32, false), kh2_anim::Options::default()).bones;
                    let pt = r.part_turns(&kh2);
                    let (Some(pp), Some(pc)) = (pt[0], pt[1]) else { continue };
                    let q = rot(&kh2[bone]) * rot(&bind2[bone]).inverse();
                    let mut best = (0.0f32, f32::MAX);
                    for k in -10..=30 { let t = k as f32 / 20.0; let e = q.angle_between(pp.slerp(pc, t)).to_degrees(); if e < best.1 { best = (t, e); } }
                    ts.push(best.0); res.push(best.1);
                }
            }
            ts.sort_by(|a, b| a.partial_cmp(b).unwrap()); res.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let q = |v: &Vec<f32>, p: f32| v[((v.len() - 1) as f32 * p) as usize];
            println!("KH2 {bone}: t median {:.2} (p10 {:.2} p90 {:.2}); residual median {:.1} p95 {:.1} max {:.1} deg, {} frames", q(&ts, 0.5), q(&ts, 0.1), q(&ts, 0.9), q(&res, 0.5), q(&res, 0.95), q(&res, 1.0), ts.len());
        }
        return;
    }
    if a.get(2).map(|s| s.as_str()) == Some("kh1") {
        // KH1's own motions (plain FK, `mset::Motion::apply`): how far its coat's lower joints
        // stand off the hips' up axis against the bind, in the hips' (3) frame, as KH1's own
        // animators move it.
        let motions = kh1_data::mset::parse(&std::fs::read(&a[3]).unwrap()).unwrap();
        let kb = kh2_anim::Pose::bind(&kh1).bones;
        let at = |v: &[Mat4], i: usize| v[i].w_axis.truncate();
        let roots = [72usize, 248, 252, 256, 260, 264, 268, 272, 276];
        let joints: Vec<usize> = roots.iter().flat_map(|&r| (r + 2)..(r + 4)).collect();
        let rr = |v: &[Mat4], j: usize| { let l = (rot(&v[3]) * rot(&kb[3]).inverse()).inverse() * (at(v, j) - at(v, 3)); (l.x * l.x + l.z * l.z).sqrt() };
        let mut outs = vec![];
        let mut ins = vec![];
        let mut per: Vec<(usize, f32)> = vec![];
        for (slot, m) in motions.iter().enumerate() {
            let Some(m) = m else { continue };
            let mut mx = 0.0f32;
            for f in (0..m.frames.max(1)).step_by(2) {
                let p = kh2_anim::Pose::bind(&m.apply(&kh1, f as f32)).bones;
                for &j in &joints {
                    let d = rr(&p, j) - rr(&kb, j);
                    if d > 0.0 { outs.push(d); mx = mx.max(d); } else { ins.push(-d); }
                }
            }
            per.push((slot, mx));
        }
        outs.sort_by(|a, b| a.partial_cmp(b).unwrap()); ins.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = (outs.len() + ins.len()) as f32;
        let q = |v: &Vec<f32>, p: f32| if v.is_empty() { 0.0 } else { v[((v.len() - 1) as f32 * p) as usize] };
        println!("KH1's own coat off its hips' axis (lower joints, every 2nd frame): out >2 {:.1}%, >5 {:.1}%, out p95 {:.1} p99 {:.1} max {:.1}; in >2 {:.1}%, in max {:.1}",
            100.0 * outs.iter().filter(|&&d| d > 2.0).count() as f32 / n, 100.0 * outs.iter().filter(|&&d| d > 5.0).count() as f32 / n, q(&outs, 0.95), q(&outs, 0.99), q(&outs, 1.0),
            100.0 * ins.iter().filter(|&&d| d > 2.0).count() as f32 / n, q(&ins, 1.0));
        per.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        println!("slots standing out most: {:?}", per.iter().take(12).map(|(s, d)| format!("{s}:{d:.1}")).collect::<Vec<_>>());
        return;
    }
    let bind = r.bind();
    let bind2 = kh2_anim::Pose::bind(&rig.skeleton).bones;
    let parent: Vec<Option<usize>> = kh1.iter().map(|b| (b.parent >= 0).then_some(b.parent as usize)).collect();
    let at = |v: &[Mat4], i: usize| v[i].w_axis.truncate();
    let roots = [72usize, 248, 252, 256, 260, 264, 268, 272, 276];
    let joints: Vec<usize> = roots.iter().flat_map(|&r| (r + 1)..(r + 4)).collect();
    let measured = |j: usize| !roots.contains(&(j - 1));
    let thighs = [(43usize, 44usize), (17, 18)];
    let bd: Vec<f32> = joints.iter().map(|&j| thighs.iter().map(|&(h, k)| seg(at(&bind, j), at(&bind, h), at(&bind, k)).0).fold(f32::MAX, f32::min)).collect();
    let variant = a.get(2).cloned().unwrap_or_default();
    let clear: f32 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(11.0);
    let tw: [f32; 3] = [a.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0), a.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.5), a.get(6).and_then(|s| s.parse().ok()).unwrap_or(1.0)];
    // Rebuild a chain: joint k (root+1..root+3) turned by slerp(chest, waist, w[k]).
    let waist = |p: &mut Vec<Mat4>, kh2: &[Mat4]| {
        let chest = rot(&p[78]) * rot(&bind[78]).inverse();
        let w66 = rot(&kh2[66]) * rot(&bind2[66]).inverse();
        for &r0 in &roots {
            for (k, j) in ((r0 + 1)..(r0 + 4)).enumerate() {
                let q = parent[j].unwrap();
                let tq = rot(&p[q]) * rot(&bind[q]).inverse();
                let pos = at(p, q) + tq * (at(&bind, j) - at(&bind, q));
                let t = chest.slerp(w66, tw[k]);
                p[j] = Mat4::from_rotation_translation((t * rot(&bind[j])).normalize(), pos);
            }
        }
    };
    // Height blend: joint k of each chain turns slerp(chest, lower, tw[k]); lower = the pelvis,
    // the front panels a share toward their thigh (front .25, side-front .15, side-back .05).
    let share: [f32; 9] = [0.0, 0.25, 0.15, 0.05, 0.0, 0.25, 0.15, 0.05, 0.0];
    let side: [usize; 9] = [0, 43, 43, 43, 43, 17, 17, 17, 17];
    let thigh_k: f32 = a.get(7).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let hblend = |p: &mut Vec<Mat4>| {
        let tr = |p: &Vec<Mat4>, i: usize| rot(&p[i]) * rot(&bind[i]).inverse();
        let chest = tr(p, 78);
        let pel = tr(p, 3);
        for (ci, &r0) in roots.iter().enumerate() {
            let pel = if variant.contains('Q') { tr(p, 71) } else { pel };
            let low = if side[ci] == 0 { pel } else { pel.slerp(tr(p, side[ci]), share[ci] * thigh_k) };
            for (k, j) in ((r0 + 1)..(r0 + 4)).enumerate() {
                let q = parent[j].unwrap();
                let tq = rot(&p[q]) * rot(&bind[q]).inverse();
                let pos = at(p, q) + tq * (at(&bind, j) - at(&bind, q));
                let t = chest.slerp(low, tw[k]);
                p[j] = Mat4::from_rotation_translation((t * rot(&bind[j])).normalize(), pos);
            }
        }
    };
    // Thigh lead: each panel chain swings about its root by a share of its thigh's turn
    // against the chest (front most).
    let lead: [f32; 3] = [a.get(10).and_then(|s| s.parse().ok()).unwrap_or(0.3), a.get(11).and_then(|s| s.parse().ok()).unwrap_or(0.15), a.get(12).and_then(|s| s.parse().ok()).unwrap_or(0.05)];
    let tlead = |p: &mut Vec<Mat4>| {
        let tr = |p: &Vec<Mat4>, i: usize| rot(&p[i]) * rot(&bind[i]).inverse();
        let chest = tr(p, 78);
        for (ci, &r0) in roots.iter().enumerate() {
            if side[ci] == 0 { continue; }
            let k = (ci - 1) % 4;
            if k == 3 { continue; }
            let th = side[ci];
            let (now, was) = ((at(p, th + 1) - at(p, th)).normalize(), chest * (at(&bind, th + 1) - at(&bind, th)).normalize());
            let delta = Quat::from_rotation_arc(was, now);
            let q = Quat::IDENTITY.slerp(delta, lead[k]);
            let root = at(p, r0);
            let m = Mat4::from_translation(root) * Mat4::from_quat(q) * Mat4::from_translation(-root);
            for j in (r0 + 1)..(r0 + 4) { p[j] = m * p[j]; }
        }
    };
    // Lift: each panel with a thigh beside it flaps out about its level hinge by a share of how
    // far that thigh has swung out (forward for the front ones) about the same hinge from the
    // bind, in the chest's frame. One smooth function of the thigh's direction, no collision.
    let lshare: [f32; 3] = [a.get(10).and_then(|s| s.parse().ok()).unwrap_or(0.6), a.get(11).and_then(|s| s.parse().ok()).unwrap_or(0.35), a.get(12).and_then(|s| s.parse().ok()).unwrap_or(0.1)];
    let lstart: f32 = a.get(14).and_then(|s| s.parse::<f32>().ok()).unwrap_or(20.0).to_radians();
    let lift_l = |p: &mut Vec<Mat4>| {
        let chest = rot(&p[78]) * rot(&bind[78]).inverse();
        for (ci, &r0) in roots.iter().enumerate() {
            if side[ci] == 0 { continue; }
            let k = (ci - 1) % 4;
            if k == 3 { continue; }
            let o = at(&bind, r0) - at(&bind, 71);
            let out = Vec3::new(o.x, 0.0, o.z).normalize();
            let ax_b = Vec3::Y.cross(out).normalize();
            let th = side[ci];
            let now = chest.inverse() * (at(p, th + 1) - at(p, th)).normalize();
            let was = (at(&bind, th + 1) - at(&bind, th)).normalize();
            let (n2, w2) = (now - ax_b * now.dot(ax_b), was - ax_b * was.dot(ax_b));
            let wgt = (n2.length() / 0.5).clamp(0.0, 1.0);
            let (n2, w2) = (n2.normalize_or_zero(), w2.normalize_or_zero());
            if n2 == Vec3::ZERO { continue; }
            let phi = w2.cross(n2).dot(ax_b).atan2(w2.dot(n2));
            // Only outward: the hinge's positive turn swings the panel out (ax = up x out).
            let x = phi - lstart;
            let soft_r = 10f32.to_radians();
            let pos = if x >= soft_r { x } else if x <= -soft_r { 0.0 } else { (x + soft_r) * (x + soft_r) / (4.0 * soft_r) };
            let theta = lshare[k] * pos * wgt;
            if theta == 0.0 { continue; }
            let root = at(p, r0);
            let m = Mat4::from_translation(root) * Mat4::from_quat(Quat::from_axis_angle(chest * ax_b, theta)) * Mat4::from_translation(-root);
            for j in (r0 + 1)..(r0 + 4) { p[j] = m * p[j]; }
        }
    };
    // Hips' twist: the lower joints turn about the chest's up axis by a share of the hips' turn
    // about it against the chest (swing-twist), tw[k] per joint, each about its parent.
    let ytwist = |p: &mut Vec<Mat4>| {
        let chest = rot(&p[78]) * rot(&bind[78]).inverse();
        let pel = rot(&p[3]) * rot(&bind[3]).inverse();
        let rel = (chest.inverse() * pel).normalize();
        let tq = Quat::from_xyzw(0.0, rel.y, 0.0, rel.w);
        let tq = if tq.length_squared() < 1e-8 { Quat::IDENTITY } else { tq.normalize() };
        let up = chest * Vec3::Y;
        let (axis, ang) = tq.to_axis_angle();
        let ang = if axis.y < 0.0 { -ang } else { ang };
        let ang = if ang > std::f32::consts::PI { ang - 2.0 * std::f32::consts::PI } else if ang < -std::f32::consts::PI { ang + 2.0 * std::f32::consts::PI } else { ang };
        for &r0 in &roots {
            let mut done = 0.0;
            for (k, j) in ((r0 + 1)..(r0 + 4)).enumerate() {
                let want = tw[k] * ang;
                let d = want - done;
                done = want;
                if d == 0.0 { continue; }
                let piv = at(p, parent[j].unwrap());
                let m = Mat4::from_translation(piv) * Mat4::from_quat(Quat::from_axis_angle(up, d)) * Mat4::from_translation(-piv);
                for b in j..(r0 + 4) { p[b] = m * p[b]; }
            }
        }
    };
    // Hang: the lower joints keep the chest's yaw but only `lean_k` of its lean, so they hang
    // (as the hood did), per joint tw[k] of the way from the chest's turn to the hanging one.
    let lean_k: f32 = a.get(7).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let whang = |p: &mut Vec<Mat4>| {
        let chest = rot(&p[78]) * rot(&bind[78]).inverse();
        let yaw = { let t = Quat::from_xyzw(0.0, chest.y, 0.0, chest.w); if t.length_squared() < 1e-8 { Quat::IDENTITY } else { t.normalize() } };
        let hang = yaw.slerp(chest, lean_k);
        for &r0 in &roots {
            for (k, j) in ((r0 + 1)..(r0 + 4)).enumerate() {
                let q = parent[j].unwrap();
                let tq = rot(&p[q]) * rot(&bind[q]).inverse();
                let pos = at(p, q) + tq * (at(&bind, j) - at(&bind, q));
                let t = chest.slerp(hang, tw[k]);
                p[j] = Mat4::from_rotation_translation((t * rot(&bind[j])).normalize(), pos);
            }
        }
    };
    // Carry blend: each lower joint's place is a blend of where the chest and the hips carry it
    // (as in the bind), by tw[k]; the segments above it re-aimed (lengths kept).
    let cblend = |p: &mut Vec<Mat4>| {
        let cc = if variant.contains('K') { p[71] * bind[71].inverse() } else { p[78] * bind[78].inverse() };
        let cp = p[3] * bind[3].inverse();
        for &r0 in &roots {
            for (k, j) in ((r0 + 1)..(r0 + 4)).enumerate() {
                if k == 0 || tw[k] == 0.0 { continue; }
                let pj = parent[j].unwrap();
                let piv = at(p, pj);
                let ci = roots.iter().position(|&x| x == r0).unwrap();
                let low = if variant.contains('b') && side[ci] != 0 {
                    let ct = p[side[ci]] * bind[side[ci]].inverse();
                    cp.transform_point3(at(&bind, j)).lerp(ct.transform_point3(at(&bind, j)), share[ci] * thigh_k)
                } else { cp.transform_point3(at(&bind, j)) };
                let want = cc.transform_point3(at(&bind, j)).lerp(low, tw[k]);
                let (from, to) = ((at(p, j) - piv).normalize_or_zero(), (want - piv).normalize_or_zero());
                if from == Vec3::ZERO || to == Vec3::ZERO { continue; }
                let m = Mat4::from_translation(piv) * Mat4::from_quat(Quat::from_rotation_arc(from, to)) * Mat4::from_translation(-piv);
                for b in pj..(r0 + 4) { if b != r0 { p[b] = m * p[b]; } }
            }
        }
    };
    // The soft push: each lower joint (root+2, root+3) pushed out of the shorts round each
    // thigh (radius by the place along it) and out of the hips (its bind radius about the
    // hips' up axis), the segment above re-aimed at it (lengths kept).
    let shorts_r = |t: f32| if t < 0.5 { 6.4 + (11.2 - 6.4) * t / 0.5 } else { 11.2 };
    let margin: f32 = a.get(8).and_then(|s| s.parse().ok()).unwrap_or(1.5);
    let fade: f32 = a.get(13).and_then(|s| s.parse().ok()).unwrap_or(4.0);
    let soft: f32 = a.get(9).and_then(|s| s.parse().ok()).unwrap_or(1.5);
    let lift = |d: f32, c: f32| -> f32 { let x = d - c; if x >= soft { d } else if x <= -soft { c } else { c + (x + soft) * (x + soft) / (4.0 * soft) } };
    let hips_r: Vec<f32> = joints.iter().map(|&j| { let l = at(&bind, j) - at(&bind, 3); (l.x * l.x + l.z * l.z).sqrt() }).collect();
    let use_hips = variant.contains('H');
    let spush = |p: &mut Vec<Mat4>| {
        let mut frozen: std::collections::HashMap<(usize, usize), f32> = std::collections::HashMap::new();
        for &r0 in &roots {
            for j in [r0 + 2, r0 + 2, r0 + 2, r0 + 2, r0 + 2, r0 + 2, r0 + 3, r0 + 3, r0 + 3, r0 + 3, r0 + 3, r0 + 3] {
                let idx = joints.iter().position(|&x| x == j).unwrap();
                let pj = parent[j].unwrap();
                let piv = at(p, pj);
                let mut n = at(p, j);
                for &(h, kn) in &thighs {
                    let (ha, kb) = (at(p, h), at(p, kn));
                    let ab = kb - ha;
                    let t_raw = (n - ha).dot(ab) / ab.length_squared();
                    let t = t_raw.clamp(if variant.contains('C') { f32::MIN } else { 0.0 }, 1.0);
                    let near = ha + ab * t;
                    let d = (n - near).length();
                    // Flat-topped tube: above the hip joint the clearance fades out over `fade`.
                    let fade_k = if variant.contains('C') { (1.0 + t_raw * ab.length() / fade).clamp(0.0, 1.0) } else { 1.0 };
                    let ti = if h == 43 { 0 } else { 1 };
                    let key = (idx, ti);
                    let tf = *frozen.entry(key).or_insert(t_raw);
                    let fade_k = if variant.contains('R') { let x = (tf * ab.length() / fade).clamp(0.0, 1.0); x * x * (3.0 - 2.0 * x) } else if variant.contains('F') { (1.0 + tf * ab.length() / fade).clamp(0.0, 1.0) } else { fade_k };
                    let c = (shorts_r(if variant.contains('F') { tf.clamp(0.0, 1.0) } else { t.max(0.0) }) + margin).min(bd[idx]) * fade_k;
                    if c <= 0.0 { continue; }
                    if variant.contains('X') && d < c + soft { println!("      push j{j} thigh {h} t {t_raw:.2} d {d:.2} c {c:.1} dir {:.2?}", ((n - near) / d.max(1e-6))); }
                    if variant.contains('S') {
                        // Pushed along the radial direction while shallow, along the way the
                        // panel swings out (the hinge's tangent, square to the thigh) while deep.
                        if d >= c + soft { continue; }
                        let axis_d = ab.normalize_or_zero();
                        let root = at(p, r0);
                        let out = { let o = at(p, r0) - at(p, 71); let o = Vec3::new(o.x, 0.0, o.z).normalize_or_zero(); o };
                        let hinge_ax = Vec3::Y.cross(out).normalize_or_zero();
                        let tang = hinge_ax.cross(n - root);
                        let tang = (tang - axis_d * tang.dot(axis_d)).normalize_or_zero();
                        let radial = if d > 1e-4 { (n - near) / d } else { tang };
                        let w = (d / c).clamp(0.0, 1.0);
                        let dir = (radial * w + tang * (1.0 - w)).normalize_or_zero();
                        if dir == Vec3::ZERO { continue; }
                        // Move along dir until the distance from the axis line is the lifted one.
                        let v = n - near;
                        let target = lift(d, c);
                        let b = v.dot(dir);
                        let g = b * b - v.length_squared() + target * target;
                        let sft = (-b + g.max(0.0).sqrt()).max(0.0);
                        n += dir * sft;
                        continue;
                    }
                    if d < 1e-4 { continue; }
                    let d2 = lift(d, c);
                    n = near + (n - near) * (d2 / d);
                }
                if use_hips && at(&bind, j).y < 92.0 {
                    let turn3 = rot(&p[3]) * rot(&bind[3]).inverse();
                    let l = turn3.inverse() * (n - at(p, 3));
                    let rr = (l.x * l.x + l.z * l.z).sqrt();
                    if rr > 1e-4 {
                        let r2 = lift(rr, hips_r[idx] - 0.5);
                        n = at(p, 3) + turn3 * Vec3::new(l.x * r2 / rr, l.y, l.z * r2 / rr);
                    }
                }
                let (from, to) = ((at(p, j) - piv).normalize_or_zero(), (n - piv).normalize_or_zero());
                if from == Vec3::ZERO || to == Vec3::ZERO { continue; }
                let qr = if variant.contains('G') {
                    // Only the swing about the panel's own level hinge (its out-and-up flap).
                    let ci = roots.iter().position(|&x| x == r0).unwrap();
                    let o = at(&bind, r0) - at(&bind, 71);
                    let _ = ci;
                    let ax = (rot(&p[78]) * rot(&bind[78]).inverse()) * Vec3::Y.cross(Vec3::new(o.x, 0.0, o.z).normalize()).normalize();
                    let (f2, t2) = ((from - ax * from.dot(ax)).normalize_or_zero(), (to - ax * to.dot(ax)).normalize_or_zero());
                    if f2 == Vec3::ZERO || t2 == Vec3::ZERO { continue; }
                    let ang = f2.cross(t2).dot(ax).atan2(f2.dot(t2));
                    Quat::from_axis_angle(ax, ang)
                } else { Quat::from_rotation_arc(from, to) };
                let m = Mat4::from_translation(piv) * Mat4::from_quat(qr) * Mat4::from_translation(-piv);
                // The segment's own bone (the parent) and everything under it turn.
                for b in pj..(r0 + 4) { if b != r0 { p[b] = m * p[b]; } }
            }
        }
    };
    let pen = |p: &Vec<Mat4>, j: usize| -> f32 {
        let mut worst = 0.0f32;
        for &(h, kn) in &thighs {
            let (ha, kb) = (at(p, h), at(p, kn));
            let ab = kb - ha;
            let t = (at(p, j) - ha).dot(ab) / ab.length_squared();
            if !(0.0..=1.0).contains(&t) { continue; }
            let d = (at(p, j) - (ha + ab * t)).length();
            worst = worst.max(shorts_r(t) - d);
        }
        worst
    };
    // The hinge push: each chain swings outward about a level axis at its root, the least angle
    // that keeps every joint `clear` (or its bind distance) off both thighs.
    let hinge: Vec<Vec3> = roots.iter().map(|&r0| {
        let o = at(&bind, r0) - at(&bind, 71);
        Vec3::Y.cross(Vec3::new(o.x, 0.0, o.z).normalize()).normalize()
    }).collect();
    let push = |p: &mut Vec<Mat4>| {
        let chest = rot(&p[78]) * rot(&bind[78]).inverse();
        for (ci, &r0) in roots.iter().enumerate() {
            let root = at(p, r0);
            let axis = chest * hinge[ci];
            let g = |th: f32| -> f32 {
                let q = Quat::from_axis_angle(axis, th);
                let mut worst = f32::MAX;
                for j in (r0 + 1)..(r0 + 4) {
                    let idx = joints.iter().position(|&x| x == j).unwrap();
                    let c = bd[idx].min(clear);
                    let n = root + q * (at(p, j) - root);
                    for &(h, kn) in &thighs { worst = worst.min(seg(n, at(p, h), at(p, kn)).0 - c); }
                }
                worst
            };
            if g(0.0) >= 0.0 { continue; }
            let step = 2f32.to_radians();
            let mut th = None;
            let mut prev = g(0.0);
            let mut best = (prev, 0.0f32);
            for k in 1..=50 {
                let t = k as f32 * step;
                let v = g(t);
                if v > best.0 { best = (v, t); }
                if v >= 0.0 { th = Some(t - step * v / (v - prev).max(1e-6)); break; }
                prev = v;
            }
            let th = th.unwrap_or(best.1);
            let m = Mat4::from_translation(root) * Mat4::from_quat(Quat::from_axis_angle(axis, th)) * Mat4::from_translation(-root);
            for j in (r0 + 1)..(r0 + 4) { p[j] = m * p[j]; }
        }
    };
    // KH1's own parent: every chain carried by the spine base 71 (its parent in KH1's skeleton,
    // turned 0.67 of the way from the pelvis part to the chest part) as in the bind.
    let by71 = |p: &mut Vec<Mat4>| {
        let carry = p[71] * bind[71].inverse();
        for &r0 in &roots { for j in r0..(r0 + 4) { p[j] = carry * bind[j]; } }
    };
    let bpel = bind[3].inverse();
    let _ = bpel;
    let br: Vec<f32> = joints.iter().map(|&j| { let l = at(&bind, j) - at(&bind, 3); (l.x * l.x + l.z * l.z).sqrt() }).collect();
    let mut waist_in = 0usize;
    let (mut waist_out, mut waist_out5, mut low_n) = (0usize, 0usize, 0usize);
    let mut outs: Vec<f32> = vec![];
    let mut off71 = (0.0f64, 0.0f32);
    println!("variant '{variant}' clear {clear} waist weights {tw:?}");
    let (mut frames, mut deficit2, mut under9) = (0usize, 0usize, 0usize);
    let mut nearest = (f32::MAX, 0u16, 0.0f32, 0usize);
    let mut snap = (0.0f32, 0u16, 0.0f32, 0usize);
    let mut off_kh2 = (0.0f64, 0.0f32);
    let mut deepest = (0.0f32, 0u16, 0.0f32, 0usize);
    let mut worst_m: Vec<(u16, f32)> = vec![];
    let mut steps: Vec<f32> = vec![];
    let mut knee_acc: Vec<f32> = vec![];
    for &id in &ids {
        let m = &rig.motions[&id];
        let n = kh2_anim::length_frames(m) as usize;
        let mut low = f32::MAX;
        let mut prev: Option<Vec<Vec3>> = None;
        let mut prev2: Option<Vec<Vec3>> = None;
        let mut ease: std::collections::HashMap<(usize, usize), Quat> = std::collections::HashMap::new();
        let (mut m_coat, mut m_knee) = (0.0f32, 0.0f32);
        let (mut m_pen, mut m_hipj) = (0.0f32, f32::MAX);
        let mut mo: std::collections::BTreeMap<usize, (f32, usize)> = Default::default();
        for h in 0..(n.max(1) * 2) {
            let f = h as f32 * 0.5;
            let kh2 = kh2_anim::evaluate(&rig.skeleton, m, kh2_anim::key_time(m, f, false), kh2_anim::Options::default()).bones;
            let mut p = r.pose(&kh2);
            if variant.contains('X') { println!("   {id} f{f}"); }
            let rigid = p.clone();
            if variant.contains('K') { by71(&mut p); }
            if variant.contains('w') { waist(&mut p, &kh2); }
            if variant.contains('h') { hblend(&mut p); }
            if variant.contains('T') { tlead(&mut p); }
            if variant.contains('B') { cblend(&mut p); }
            if variant.contains('L') { lift_l(&mut p); }
            if variant.contains('Y') { ytwist(&mut p); }
            if variant.contains('W') { whang(&mut p); }
            if variant.contains('p') { push(&mut p); }
            if variant.contains('s') {
                let before = p.clone();
                spush(&mut p);
                if variant.contains('E') {
                    let step: f32 = a.get(15).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.15);
                    let lim = |prev: Option<Quat>, want: Quat| -> Quat { match prev { Some(pq) => { let an = pq.angle_between(want); if an > step { pq.slerp(want, step / an).normalize() } else { want } } None => want } };
                    for &r0 in &roots {
                        let root = at(&before, r0);
                        let (b2, b3, p2, p3) = (at(&before, r0 + 2), at(&before, r0 + 3), at(&p, r0 + 2), at(&p, r0 + 3));
                        let q2 = Quat::from_rotation_arc((b2 - root).normalize(), (p2 - root).normalize());
                        let q3 = Quat::from_rotation_arc((q2 * (b3 - b2)).normalize(), (p3 - p2).normalize());
                        let e2 = lim(ease.get(&(r0, 2)).copied(), q2);
                        let e3 = lim(ease.get(&(r0, 3)).copied(), q3);
                        ease.insert((r0, 2), e2);
                        ease.insert((r0, 3), e3);
                        let m2 = Mat4::from_translation(root) * Mat4::from_quat(e2) * Mat4::from_translation(-root);
                        for j in (r0 + 1)..(r0 + 4) { p[j] = m2 * before[j]; }
                        let piv = at(&p, r0 + 2);
                        let m3 = Mat4::from_translation(piv) * Mat4::from_quat(e3) * Mat4::from_translation(-piv);
                        p[r0 + 2] = m3 * p[r0 + 2];
                        p[r0 + 3] = m3 * p[r0 + 3];
                    }
                }
            }
            frames += 1;
            let chest_inv = p[78].inverse();
            let local: Vec<Vec3> = joints.iter().map(|&j| chest_inv.transform_point3(at(&p, j))).collect();
            let mut local = local;
            local.push(chest_inv.transform_point3(at(&p, 44)));
            local.push(chest_inv.transform_point3(at(&p, 18)));
            if let (Some(pr), Some(pr2)) = (&prev, &prev2) {
                for k in 0..joints.len() { if !measured(joints[k]) { continue; } let s = (local[k] - 2.0 * pr[k] + pr2[k]).length(); steps.push(s); m_coat = m_coat.max(s); if variant.contains('D') && s > 6.0 { let rl = chest_inv.transform_point3(at(&rigid, joints[k])); println!("    {id} f{f} joint {} jerk {s:.1} pos {:.1?} prev {:.1?} prev2 {:.1?} rigid {:.1?} d+x {:.1} d-x {:.1}", joints[k], local[k], pr[k], pr2[k], rl, seg(at(&rigid, joints[k]), at(&p,43), at(&p,44)).0, seg(at(&rigid, joints[k]), at(&p,17), at(&p,18)).0); } if s > snap.0 { snap = (s, id, f, joints[k]); } }
                for k in joints.len()..local.len() { let s = (local[k] - 2.0 * pr[k] + pr2[k]).length(); knee_acc.push(s); m_knee = m_knee.max(s); }
            }
            prev2 = prev.take();
            prev = Some(local);
            let pel_inv = p[3].inverse();
            for (k, &j) in joints.iter().enumerate() {
                if !measured(j) { continue; }
                let _ = &pel_inv;
                let l = (rot(&p[3]) * rot(&bind[3]).inverse()).inverse() * (at(&p, j) - at(&p, 3));
                let rr = (l.x * l.x + l.z * l.z).sqrt();
                if at(&bind, j).y < 92.0 && rr < br[k] - 2.0 { waist_in += 1; }
                if at(&bind, j).y < 92.0 && variant.contains('O') { let ci = roots.iter().position(|&x| x == j - (j - 72) % 4).unwrap_or(0); let e = mo.entry(ci).or_insert((0.0f32, 0usize)); e.0 += rr - br[k]; e.1 += 1; }
                if at(&bind, j).y < 92.0 { let d = rr - br[k]; if d > 2.0 { waist_out += 1; } if d > 5.0 { waist_out5 += 1; } if d > 0.0 { outs.push(d); } low_n += 1; }
                let d71 = (at(&p, j) - (p[71] * bind[71].inverse()).transform_point3(at(&bind, j))).length(); off71.0 += d71 as f64; off71.1 = off71.1.max(d71);
                let dev = (at(&p, j) - at(&rigid, j)).length();
                off_kh2.0 += dev as f64; off_kh2.1 = off_kh2.1.max(dev);
                let d = thighs.iter().map(|&(hh, kn)| seg(at(&p, j), at(&p, hh), at(&p, kn)).0).fold(f32::MAX, f32::min);
                if d < bd[k] - 2.0 { deficit2 += 1; }
                let pe = pen(&p, j);
                if pe > 0.0 { under9 += 1; }
                if pe > deepest.0 { deepest = (pe, id, f, j); }
                if variant.contains('P') && pe > 2.0 {
                    let chest = rot(&p[78]) * rot(&bind[78]).inverse();
                    for &(hh, kn) in &thighs { let ab = at(&p, kn) - at(&p, hh); let t = (at(&p, j) - at(&p, hh)).dot(ab) / ab.length_squared(); let dl = chest.inverse() * ab.normalize(); if (0.0..=1.0).contains(&t) { let d = (at(&p, j) - (at(&p, hh) + ab * t)).length(); if d < 6.5 { println!("     pen {id} f{f} j{j} thigh {hh} t {t:.2} d {d:.1} thigh dir in chest frame {dl:.2?}"); } } }
                }
                m_pen = m_pen.max(pe);
                for hj in [43usize, 17] { m_hipj = m_hipj.min((at(&p, j) - at(&p, hj)).length()); }
                low = low.min(d);
                if d < nearest.0 { nearest = (d, id, f, j); }
            }
        }
        worst_m.push((id, low));
        if variant.contains('O') { println!("  motion {id}: mean off the hips' axis per chain (back, +x front, side-front, side-back, back, -x front, ...): {:?}", mo.values().map(|(a, n)| format!("{:.1}", a / *n as f32)).collect::<Vec<_>>()); }
        if variant.contains('J') { println!("  motion {id}: coat jerk max {:.1}, knee jerk max {:.1} pen {:.1} hipjoint {:.1}", m_coat, m_knee, m_pen, m_hipj); }
    }
    worst_m.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    steps.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let jf = (frames * joints.iter().filter(|&&j| measured(j)).count()) as f32;
    println!("{frames} half-frames; joint-frames >2 nearer than bind {:.1}%, inside the shorts {:.2}% (deepest {:.1}, motion {} f{} joint {}); nearest {:.1} (motion {} f{} joint {})", 100.0 * deficit2 as f32 / jf, 100.0 * under9 as f32 / jf, deepest.0, deepest.1, deepest.2, deepest.3, nearest.0, nearest.1, nearest.2, nearest.3);
    println!("joint-frames >2 nearer the hips' axis than in the bind (into the belt / shorts top): {:.1}%", 100.0 * waist_in as f32 / jf);
    outs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let qo = |p: f32| if outs.is_empty() { 0.0 } else { outs[((outs.len() - 1) as f32 * p) as usize] };
    println!("standing off the hips' axis (lower joints): out >2 {:.1}%, >5 {:.1}%, out p95 {:.1} p99 {:.1} max {:.1}", 100.0 * waist_out as f32 / low_n.max(1) as f32, 100.0 * waist_out5 as f32 / low_n.max(1) as f32, qo(0.95), qo(0.99), qo(1.0));
    println!("off where KH1's spine base 71 carries it: mean {:.2} max {:.1}", off71.0 / jf as f64, off71.1);
    println!("off the chest-rigid coat: mean {:.2} max {:.1}; second difference against the chest: p99 {:.2} p99.9 {:.2} max {:.1} (motion {} f{} joint {})", off_kh2.0 / jf as f64, off_kh2.1, steps[(steps.len() as f32 * 0.99) as usize], steps[(steps.len() as f32 * 0.999) as usize], snap.0, snap.1, snap.2, snap.3);
    knee_acc.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("the knees' own second difference against the chest: p99 {:.2} p99.9 {:.2} max {:.1}", knee_acc[(knee_acc.len() as f32 * 0.99) as usize], knee_acc[(knee_acc.len() as f32 * 0.999) as usize], knee_acc[knee_acc.len() - 1]);
    println!("worst motions: {:?}", worst_m.iter().take(10).map(|(i, d)| format!("{i}:{d:.1}")).collect::<Vec<_>>());
}
