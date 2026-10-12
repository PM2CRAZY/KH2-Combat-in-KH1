//! KH1 Sora's skeleton next to KH2 Sora's, in their bind poses (for the retarget map).
//! `cargo run -p kh1-data --example skeletons -- <xa_ex_0010.mdls> <P_EX100.mdlx>`
//! (files extracted from your own installs; nothing is kept).

use kh2_anim::Pose;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&std::fs::read(&a[1]).unwrap()).unwrap();
    let p1 = Pose::bind(&kh1);
    let p2 = Pose::bind(&kh2);
    println!("KH1 {} bones, KH2 {} bones", kh1.len(), kh2.len());
    let n = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(80);
    for i in 0..n.min(kh1.len().max(kh2.len())) {
        let f = |p: &Pose, sk: &[kh2_formats::anim::Bone]| {
            if i < sk.len() {
                let v = p.bone_position(i);
                format!("{:4} {:7.1} {:7.1} {:7.1}", sk[i].parent, v.x, v.y, v.z)
            } else {
                String::from("-")
            }
        };
        println!("{i:3} | KH1 {} | KH2 {}", f(&p1, &kh1), f(&p2, &kh2));
    }
}
