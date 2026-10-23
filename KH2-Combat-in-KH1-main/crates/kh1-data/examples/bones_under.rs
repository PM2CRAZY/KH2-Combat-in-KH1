//! Lists the KH1 bones under one bone, with bind positions and distance from it.
//! `cargo run -p kh1-data --example bones_under -- <xa_ex_0010.mdls> <KH2 dir> <bone>`
use kh1_data::retarget::Retarget;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[1])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let root: usize = a[2].parse().unwrap();
    let r = Retarget::new(&kh1, &rig.skeleton);
    let bind = r.bind();
    for i in 0..kh1.len() {
        let mut p = i as i16;
        let mut under = false;
        while p >= 0 {
            if p as usize == root {
                under = true;
                break;
            }
            p = kh1[p as usize].parent;
        }
        if under {
            let (s, q, _) = bind[i].to_scale_rotation_translation();
            println!("{i:3} parent {:3} at {:.1?} {:.1} from {root}, scale {s:.2?} rot {q:.2?}; local {:?}", kh1[i].parent, bind[i].w_axis.truncate(), (bind[i].w_axis - bind[root].w_axis).length(), kh1[i]);
        }
    }
}
