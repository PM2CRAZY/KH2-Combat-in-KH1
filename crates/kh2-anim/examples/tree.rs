//! Research helper: `tree <mdlx>` prints the bone hierarchy with bind-pose
//! positions, segment lengths and descendant counts.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let skel = kh2_formats::anim::parse_skeleton(&std::fs::read(&a[1]).unwrap()).unwrap();
    let pose = kh2_anim::Pose::bind(&skel);
    let n = skel.len();
    let mut desc = vec![0usize; n];
    for i in (0..n).rev() {
        let p = skel[i].parent;
        if p >= 0 { desc[p as usize] += desc[i] + 1; }
    }
    for (i, b) in skel.iter().enumerate() {
        let p = pose.bone_position(i);
        let len = if b.parent >= 0 { p.distance(pose.bone_position(b.parent as usize)) } else { 0.0 };
        println!("{i:>3} p{:>4} len {len:>6.1} desc {:>3} at ({:>6.1},{:>6.1},{:>6.1}) w {:.1}", b.parent, desc[i], p.x, p.y, p.z, b.length);
    }
}
