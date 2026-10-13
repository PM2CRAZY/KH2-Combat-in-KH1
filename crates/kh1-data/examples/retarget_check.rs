//! KH2 Sora's motions on KH1 Sora's skeleton: prints key joints per frame and writes the
//! posed bones as JSON lines (for a plot).
//! `cargo run -p kh1-data --example retarget_check -- <xa_ex_0010.mdls> <dir with KH2 files> <motion> [frames] [out.jsonl]`
//! (files extracted from your own installs; nothing is kept).

use kh1_data::retarget::Retarget;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[1])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let motion: u16 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let frames: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let r = Retarget::new(&kh1, &rig.skeleton);
    println!("{} of {} KH1 bones mapped", r.mapped(), kh1.len());
    let m = rig.motions.get(&motion).expect("motion");
    let mut out = a.get(4).map(|p| std::fs::File::create(p).unwrap());
    // `KH2=1`: write KH2's own pose instead (to compare).
    let raw = std::env::var("KH2").is_ok();
    let parents: Vec<i16> = if raw { rig.skeleton.iter().map(|b| b.parent).collect() } else { kh1.iter().map(|b| b.parent).collect() };
    if let Some(f) = out.as_mut() {
        use std::io::Write;
        writeln!(f, "{}", serde_like(&parents.iter().map(|&p| p as f32).collect::<Vec<_>>())).unwrap();
    }
    for f in 0..frames {
        let t = kh2_anim::key_time(m, f as f32, true);
        let kh2 = kh2_anim::evaluate(&rig.skeleton, m, t, kh2_anim::Options::default()).bones;
        let kh1p = r.pose(&kh2);
        let p = |i: usize| kh1p[i].w_axis.truncate();
        let q = |i: usize| kh2[i].w_axis.truncate();
        let bad = kh1p.iter().any(|m| !m.is_finite());
        if f % 10 == 0 || f + 1 == frames {
            println!(
                "f{f:3} nan {bad} | KH1 hips {:.1?} head {:.1?} toes {:.1?} {:.1?} hands {:.1?} {:.1?} | KH2 head {:.1?} hands {:.1?} {:.1?}",
                p(0), p(151), p(23), p(49), p(206), p(101), q(141), q(86), q(178)
            );
        }
        if let Some(fh) = out.as_mut() {
            use std::io::Write;
            let v: Vec<f32> = if raw { &kh2 } else { &kh1p }.iter().flat_map(|m| m.w_axis.truncate().to_array()).collect();
            writeln!(fh, "{}", serde_like(&v)).unwrap();
        }
    }
}

fn serde_like(v: &[f32]) -> String {
    format!("[{}]", v.iter().map(|x| format!("{x:.2}")).collect::<Vec<_>>().join(","))
}
