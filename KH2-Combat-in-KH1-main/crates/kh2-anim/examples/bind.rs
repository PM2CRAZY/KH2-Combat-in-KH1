//! Research helper: `bind <mdlx> [x y z]` prints bind-pose bone positions
//! (nearest to a point first, if given).
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let skel = kh2_formats::anim::parse_skeleton(&std::fs::read(&a[1]).unwrap()).unwrap();
    let pose = kh2_anim::Pose::bind(&skel);
    let mut v: Vec<(usize, glam::Vec3)> = (0..skel.len()).map(|i| (i, pose.bone_position(i))).collect();
    if a.len() >= 5 {
        let p = glam::Vec3::new(a[2].parse().unwrap(), a[3].parse().unwrap(), a[4].parse().unwrap());
        v.sort_by(|x, y| x.1.distance(p).total_cmp(&y.1.distance(p)));
        v.truncate(8);
    }
    for (i, p) in v { println!("{i:>3} parent {:>4} {:>8.2} {:>8.2} {:>8.2}", skel[i].parent, p.x, p.y, p.z); }
}
