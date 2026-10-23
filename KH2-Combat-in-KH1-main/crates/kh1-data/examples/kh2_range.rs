//! KH2 bind positions of a bone range: `<KH2 dir> <from> <to>`.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[0])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let (f, t): (usize, usize) = (a[1].parse().unwrap(), a[2].parse().unwrap());
    let bind = kh2_anim::Pose::bind(&rig.skeleton).bones;
    for i in f..=t.min(bind.len() - 1) {
        println!("{i:3} parent {:3} at {:.1?}", rig.skeleton[i].parent, bind[i].w_axis.truncate());
    }
}
