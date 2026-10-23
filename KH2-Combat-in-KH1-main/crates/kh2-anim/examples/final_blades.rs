//! Research helper: `final_blades <body.mdlx> <body.mset> <W_EX010.mdlx> <right.mset> <left.mset>
//! <motion> [right bone] [left bone]` prints, every 4 frames of a motion, where each weapon's
//! blade bone (2) is in the body's model space (each weapon's clip on its hand bone, as
//! `kh2_rig::RigAnim` places it) and how far apart the two are.
use kh2_formats::anim::parse_skeleton;
use kh2_formats::motion::Mset;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let body = parse_skeleton(&std::fs::read(&a[1]).unwrap()).unwrap();
    let bm = Mset::parse(&std::fs::read(&a[2]).unwrap()).unwrap();
    let ws = parse_skeleton(&std::fs::read(&a[3]).unwrap()).unwrap();
    let rm = Mset::parse(&std::fs::read(&a[4]).unwrap()).unwrap();
    let lm = Mset::parse(&std::fs::read(&a[5]).unwrap()).unwrap();
    let motion: usize = a[6].parse().unwrap();
    let rb: usize = a.get(7).map_or(178, |s| s.parse().unwrap());
    let lb: usize = a.get(8).map_or(86, |s| s.parse().unwrap());
    let clip = |m: &Mset| m.slot_for(motion, true, true).and_then(|s| s.anb.as_ref()).and_then(|a| a.pose.clone());
    let (Some(b), r, l) = (clip(&bm), clip(&rm), clip(&lm)) else { panic!("no body clip") };
    println!("right clip {} left clip {}", r.is_some(), l.is_some());
    let len = kh2_anim::length_frames(&b);
    let mut f = 0.0;
    while f <= len {
        let p = kh2_anim::evaluate(&body, &b, kh2_anim::key_time(&b, f, true), Default::default()).bones;
        let at = |w: &Option<kh2_formats::anim::Interpolated>, hand: usize| {
            let local = match w {
                Some(w) => kh2_anim::evaluate(&ws, w, kh2_anim::key_time(w, f, true), Default::default()).bones,
                None => kh2_anim::Pose::bind(&ws).bones,
            };
            (p[hand] * local[2]).w_axis.truncate()
        };
        let (pr, pl) = (at(&r, rb), at(&l, lb));
        println!(
            "f{f:>4} right {pr:>7.1?} left {pl:>7.1?} apart {:>6.1} (hands {:>7.1?} {:>7.1?})",
            pr.distance(pl),
            p[rb].w_axis.truncate(),
            p[lb].w_axis.truncate()
        );
        f += 4.0;
    }
}
