//! Research helper: `extent <mdlx> <mset> <slot> <frame>` prints the lowest and
//! highest bone y and the hip position.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let skel = kh2_formats::anim::parse_skeleton(&std::fs::read(&a[1]).unwrap()).unwrap();
    let mset = kh2_formats::motion::Mset::parse(&std::fs::read(&a[2]).unwrap()).unwrap();
    let m = mset.slots[a[3].parse::<usize>().unwrap()].anb.as_ref().unwrap().pose.as_ref().unwrap();
    let t = kh2_anim::key_time(m, a[4].parse().unwrap(), false);
    let p = kh2_anim::evaluate(&skel, m, t, Default::default());
    let ys: Vec<f32> = (0..m.bone_count).map(|i| p.bone_position(i).y).collect();
    let lo = ys.iter().cloned().fold(f32::MAX, f32::min);
    let hi = ys.iter().cloned().fold(f32::MIN, f32::max);
    println!("min y {lo:.1} max y {hi:.1} hips {:?}", p.bone_position(0));
}
