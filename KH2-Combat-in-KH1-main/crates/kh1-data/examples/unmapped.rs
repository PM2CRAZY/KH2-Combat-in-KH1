//! KH1 bones no KH2 part turns, with parent, bind position and child count.
//! `cargo run -p kh1-data --example unmapped -- <xa_ex_0010.mdls> <KH2 dir>`
use kh1_data::retarget::Retarget;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[1])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let r = Retarget::new(&kh1, &rig.skeleton);
    let bind = kh2_anim::Pose::bind(&kh1).bones;
    for i in 0..kh1.len() {
        let kids = kh1.iter().filter(|b| b.parent == i as i16).count();
        println!("{i:3} {} parent {:3} at {:.0?} kids {kids} flags {}", if r.is_mapped(i) { "MAP" } else { "   " }, kh1[i].parent, bind[i].w_axis.truncate(), kh1[i].flags);
    }
}
