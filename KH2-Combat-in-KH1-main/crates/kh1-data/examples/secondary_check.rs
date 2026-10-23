//! KH1 Sora's hair, hood and keychain following KH2's own (`kh1_data::secondary`), checked on
//! several KH2 motions: no NaN, KH1's bone lengths kept, the parts moving over time, each link
//! along the direction its KH2 match gives; with `[out dir]`, writes each checked frame's bones
//! as JSON lines (KH1's posed bones, then KH2's, for a stick-figure plot).
//! `cargo run -p kh1-data --example secondary_check -- <xa_ex_0010.mdls> <xa_ex_0010.mset> <dir with KH2 files> [out.jsonl]`
//! (files extracted from your own installs; nothing is kept).

use glam::{Mat4, Vec3};
use kh1_data::retarget::{Retarget, GRIP_BONE};
use kh1_data::secondary::{Parts, Secondary};

fn at(m: &Mat4) -> Vec3 {
    m.w_axis.truncate()
}

/// The hook's `keyblade_teeth_up`: the grip and everything under it half round about its x.
fn teeth_up(pose: &mut [Mat4], parents: &[i16]) {
    let turn = pose[GRIP_BONE] * Mat4::from_rotation_x(std::f32::consts::PI) * pose[GRIP_BONE].inverse();
    for i in GRIP_BONE..pose.len() {
        let mut p = i as i16;
        while p >= 0 && p as usize >= GRIP_BONE {
            if p as usize == GRIP_BONE {
                pose[i] = turn * pose[i];
                break;
            }
            p = parents[p as usize];
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let kh1_motions = kh1_data::mset::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[2])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let wr = &rig.weapons[0];
    let mut r = Retarget::new(&kh1, &rig.skeleton);
    r.set_rest(&kh1_motions[1].as_ref().unwrap().apply(&kh1, 0.0));
    r.set_rest_open(&kh1_motions[0].as_ref().unwrap().apply(&kh1, 0.0));
    let s = Secondary::new(&kh1, &rig.skeleton, Some(&wr.skeleton));
    let parents: Vec<i16> = kh1.iter().map(|b| b.parent).collect();
    let b1 = kh2_anim::Pose::bind(&kh1).bones;
    let b2 = kh2_anim::Pose::bind(&rig.skeleton).bones;
    println!("hair (KH1 spike bone @ root → KH2 spike bone @ root):");
    for (l, (other, share)) in s.hair().iter().zip(s.hair_mix()) {
        println!("  {} @ {:.1?} → {} @ {:.1?} ({:.0}% of {} @ {:.1?})", l.kh1, at(&b1[l.kh1]), l.kh2, at(&b2[l.kh2]), share * 100.0, other, at(&b2[*other]));
    }
    println!("hood: {:?}", s.hood().iter().map(|l| (l.kh1, l.kh2)).collect::<Vec<_>>());
    println!("chain: {:?}", s.chain().iter().map(|l| (l.kh1, l.kh2)).collect::<Vec<_>>());
    let wb = kh2_anim::Pose::bind(&wr.skeleton).bones;
    let worst_bind = s.chain().iter().map(|l| at(&b1[l.kh1]).distance(at(&wb[l.kh2]))).fold(0.0f32, f32::max);
    println!("chain binds: the farthest pair {worst_bind:.2} apart");
    let mut out = a.get(3).map(|p| std::fs::File::create(p).unwrap());
    let mut failures = 0;
    for motion in [0u16, 2, 3, 151, 154, 181, 201] {
        let Some(m) = rig.motions.get(&motion) else {
            println!("motion {motion}: none");
            continue;
        };
        let wm = wr.motions.get(&motion).or_else(|| wr.motions.get(&0));
        let n = kh2_anim::length_frames(m).max(1.0) as usize;
        // Per part: the most a link turned (relative to its reference bone) from the first frame,
        // the worst angle off the expected direction, the worst length change, NaN.
        let (mut moved, mut off, mut stretch, mut nan, mut clamped) = ([0f32; 3], [0f32; 3], 0f32, false, 0);
        // How near the hood's joints came to the head (the spike roots' middle) and the spine.
        let (mut near_head, mut near_spine) = (f32::MAX, f32::MAX);
        let mut first: Option<Vec<Vec3>> = None;
        let frames: Vec<usize> = (0..n).step_by(2).collect();
        for &f in &frames {
            let t = kh2_anim::key_time(m, f as f32, false);
            let body = kh2_anim::evaluate(&rig.skeleton, m, t, kh2_anim::Options::default()).bones;
            let hand = body[wr.bone.unwrap()];
            let weapon: Vec<Mat4> = match wm {
                Some(wm) => kh2_anim::evaluate(&wr.skeleton, wm, kh2_anim::key_time(wm, f as f32, false), kh2_anim::Options::default()).bones,
                None => wb.clone(),
            }
            .iter()
            .map(|w| hand * *w)
            .collect();
            let mut pose = r.pose(&body);
            teeth_up(&mut pose, &parents);
            let before = pose.clone();
            let rep = s.apply(&mut pose, &body, Some(&weapon), Parts::ALL);
            clamped += rep.clamped;
            nan |= pose.iter().any(|m| !m.is_finite());
            let head = pose[151] * b1[151].inverse();
            let roots: Vec<Vec3> = s.hair().iter().map(|l| at(&b1[l.kh1])).collect();
            let centre = head.transform_point3(roots.iter().copied().sum::<Vec3>() / roots.len() as f32);
            for l in s.hood() {
                let j = at(&pose[l.tip]);
                near_head = near_head.min(j.distance(centre));
                let (a, b) = (at(&pose[71]), at(&pose[78]));
                let t = ((j - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
                near_spine = near_spine.min(j.distance(a + (b - a) * t));
            }
            for i in 1..pose.len() {
                if let Some(p) = s.parent(i) {
                    let (l0, l1) = (at(&before[i]).distance(at(&before[p])), at(&pose[i]).distance(at(&pose[p])));
                    stretch = stretch.max((l1 - l0).abs());
                }
            }
            let groups = |b: usize| if s.hair().iter().any(|l| l.kh1 == b) { 0 } else if s.hood().iter().any(|l| l.kh1 == b) { 1 } else { 2 };
            for (l, want) in s.expected(&before, &body, Some(&weapon)) {
                let have = (at(&pose[l.tip]) - at(&pose[l.kh1])).normalize();
                let g = groups(l.kh1);
                off[g] = off[g].max(have.angle_between(want).to_degrees());
            }
            // Link directions in their reference bone's frame (head, neck base, hand).
            let rel: Vec<Vec3> = s
                .hair()
                .iter()
                .map(|l| (l, 151))
                .chain(s.hood().iter().map(|l| (l, 78)))
                .chain(s.chain().iter().map(|l| (l, 101)))
                .map(|(l, refb)| pose[refb].inverse().transform_vector3(at(&pose[l.tip]) - at(&pose[l.kh1])).normalize())
                .collect();
            if let Some(f0) = &first {
                let (h, d) = (s.hair().len(), s.hood().len());
                for (k, (a, b)) in f0.iter().zip(&rel).enumerate() {
                    let g = if k < h { 0 } else if k < h + d { 1 } else { 2 };
                    moved[g] = moved[g].max(a.angle_between(*b).to_degrees());
                }
            } else {
                first = Some(rel);
            }
            if let Some(fh) = out.as_mut() {
                use std::io::Write;
                if [0, frames.len() / 2].contains(&frames.iter().position(|x| *x == f).unwrap()) {
                    let p1: Vec<[f32; 3]> = pose.iter().map(|m| at(m).to_array()).collect();
                    let p2: Vec<[f32; 3]> = body.iter().chain(&weapon).map(|m| at(m).to_array()).collect();
                    let pb: Vec<[f32; 3]> = before.iter().map(|m| at(m).to_array()).collect();
                    writeln!(fh, "{{\"motion\":{motion},\"frame\":{f},\"kh1\":{p1:?},\"before\":{pb:?},\"kh2\":{p2:?}}}").unwrap();
                }
            }
        }
        let ok = !nan && stretch < 0.01 && moved.iter().all(|m| *m > 3.0);
        if !ok {
            failures += 1;
        }
        println!(
            "motion {motion:3} ({n} frames): NaN {nan}, worst length change {stretch:.4}, moved (hair / hood / chain) {:.0}° / {:.0}° / {:.0}°, off expected {:.1}° / {:.1}° / {:.1}°, held back {clamped}, hood nearest the head {near_head:.1} / the spine {near_spine:.1}{}",
            moved[0], moved[1], moved[2], off[0], off[1], off[2], if ok { "" } else { "  <- FAIL" }
        );
    }
    if let Some(fh) = out.as_mut() {
        use std::io::Write;
        writeln!(fh, "{{\"parents1\":{:?},\"parents2\":{:?},\"parents_w\":{:?}}}", parents, rig.skeleton.iter().map(|b| b.parent).collect::<Vec<_>>(), wr.skeleton.iter().map(|b| b.parent).collect::<Vec<_>>()).unwrap();
    }
    println!("{failures} motions failed");
    // Every motion, every third frame: NaN, lengths, how often a part was held back, and the
    // widest a spike / the hood's first link turned from KH1's bind relative to the head / neck.
    let mut ids: Vec<u16> = rig.motions.keys().copied().collect();
    ids.sort();
    let (mut frames, mut nan, mut stretch, mut clamped) = (0usize, 0usize, 0f32, 0usize);
    let mut widest: Vec<(f32, u16, usize, usize)> = Vec::new();
    for &id in &ids {
        let m = &rig.motions[&id];
        let wm = wr.motions.get(&id).or_else(|| wr.motions.get(&0));
        for f in (0..kh2_anim::length_frames(m).max(1.0) as usize).step_by(3) {
            let body = kh2_anim::evaluate(&rig.skeleton, m, kh2_anim::key_time(m, f as f32, false), kh2_anim::Options::default()).bones;
            let hand = body[wr.bone.unwrap()];
            let weapon: Vec<Mat4> = match wm {
                Some(wm) => kh2_anim::evaluate(&wr.skeleton, wm, kh2_anim::key_time(wm, f as f32, false), kh2_anim::Options::default()).bones,
                None => wb.clone(),
            }
            .iter()
            .map(|w| hand * *w)
            .collect();
            let mut pose = r.pose(&body);
            teeth_up(&mut pose, &parents);
            let before = pose.clone();
            let rep = s.apply(&mut pose, &body, Some(&weapon), Parts::ALL);
            frames += 1;
            clamped += rep.clamped;
            nan += pose.iter().any(|m| !m.is_finite()) as usize;
            for i in 1..pose.len() {
                if let Some(p) = s.parent(i) {
                    stretch = stretch.max((at(&pose[i]).distance(at(&pose[p])) - at(&before[i]).distance(at(&before[p]))).abs());
                }
            }
            for l in s.hair().iter().chain(&s.hood()[..1]) {
                let refb = if l.kh1 < 100 { 78 } else { 151 };
                let now = pose[refb].inverse().transform_vector3(at(&pose[l.tip]) - at(&pose[l.kh1])).normalize();
                let bind = b1[refb].inverse().transform_vector3(at(&b1[l.tip]) - at(&b1[l.kh1])).normalize();
                widest.push((now.angle_between(bind).to_degrees(), id, f, l.kh1));
            }
        }
    }
    widest.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("all {} motions, {frames} frames: {nan} with NaN, worst length change {stretch:.4}, {clamped} parts held back", ids.len());
    println!("widest turns from KH1's bind (degrees, motion, frame, KH1 bone): {:?}", widest.iter().take(8).map(|w| (w.0.round(), w.1, w.2, w.3)).collect::<Vec<_>>());
    let over = widest.iter().filter(|w| w.0 > 120.0).count();
    println!("{over} of {} spike/hood samples turned over 120°", widest.len());
}
