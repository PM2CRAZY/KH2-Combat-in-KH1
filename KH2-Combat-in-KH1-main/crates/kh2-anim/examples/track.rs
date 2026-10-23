//! Research helper: `track <mdlx> <mset> <slot> <joint>` prints, per 4 game
//! frames, the root-motion z and a joint's world position.
use kh2_formats::anim::parse_skeleton;
use kh2_formats::motion::Mset;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let skel = parse_skeleton(&std::fs::read(&a[1]).unwrap()).unwrap();
    let mset = Mset::parse(&std::fs::read(&a[2]).unwrap()).unwrap();
    let slot: usize = a[3].parse().unwrap();
    let joint: usize = a[4].parse().unwrap();
    let anb = mset.slots[slot].anb.as_ref().unwrap();
    let m = anb.pose.as_ref().unwrap();
    let info = anb.motion.as_ref().unwrap();
    let len = kh2_anim::length_frames(m);
    let mut f = 0.0;
    while f <= len {
        let t = kh2_anim::key_time(m, f, false);
        let p = kh2_anim::evaluate(&skel, m, t, Default::default());
        let r = info.root_at(t);
        println!("f{f:>4} root {:>7.1} {:>7.1} {:>7.1}  joint {:?}", r[0], r[1], r[2], p.bone_position(joint));
        f += 4.0;
    }
}
