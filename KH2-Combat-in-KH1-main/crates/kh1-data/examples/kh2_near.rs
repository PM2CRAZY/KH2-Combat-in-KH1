//! KH2 bones in a bind-pose box (model space): `<KH2 dir> <ymin> <ymax> <zmax>`.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[0])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let (y0, y1, z1): (f32, f32, f32) = (a[1].parse().unwrap(), a[2].parse().unwrap(), a[3].parse().unwrap());
    let bind = kh2_anim::Pose::bind(&rig.skeleton).bones;
    for (i, m) in bind.iter().enumerate() {
        let p = m.w_axis.truncate();
        if p.y >= y0 && p.y <= y1 && p.z <= z1 {
            println!("{i:3} parent {:3} at {p:.1?}", rig.skeleton[i].parent);
        }
    }
}
