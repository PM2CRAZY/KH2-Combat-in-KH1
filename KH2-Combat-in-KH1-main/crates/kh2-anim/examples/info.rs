//! Research helper: `info <mset> <slot>` prints a motion's structure.
use kh2_formats::motion::Mset;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mset = Mset::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    let slot: usize = a[2].parse().unwrap();
    let m = mset.slots[slot].anb.as_ref().unwrap().pose.as_ref().unwrap();
    println!("bones {} total {} frames {}..{} fps {} ret {}", m.bone_count, m.total_joints, m.frame_start, m.frame_end, m.fps, m.frame_return);
    println!("initial pose {:?}", m.initial_pose);
    for c in &m.forward { println!("fwd joint {} ch {} keys {} pre {:?} post {:?} v0 {:?}", c.joint, c.channel, c.keys.len(), c.pre, c.post, c.keys.first().map(|k| k.value)); }
    for c in &m.inverse { println!("inv helper {} ch {} keys {}", c.joint, c.channel, c.keys.len()); }
    for h in &m.ik_helpers { println!("helper {} parent {} t {:?} r {:?}", h.index, h.parent, h.translation, h.rotation); }
    println!("joints {:?}", m.joints.iter().map(|j| (j.joint, j.flags)).collect::<Vec<_>>());
    for c in &m.constraints { println!("constraint {:?}", c); }
    println!("ext {:?} expr {}", m.external_effectors, m.expressions.len());
}
