//! Research helper: `blade <W_EX010.mdlx>` prints the Keyblade's bind-pose
//! bone positions and where collision offsets on bone 2 land.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&a[1]).unwrap();
    let skel = kh2_formats::anim::parse_skeleton(&bytes).unwrap();
    let pose = kh2_anim::Pose::bind(&skel);
    for i in 0..skel.len() { println!("bone {i} {:?}", pose.bone_position(i)); }
    for c in kh2_formats::motion::parse_model_collision(&bytes).unwrap() {
        if c.bone >= 0 && (c.bone as usize) < skel.len() {
            let p = pose.bones[c.bone as usize].transform_point3(glam::Vec3::new(c.position[0] as f32, c.position[1] as f32, c.position[2] as f32));
            println!("group {} kind {} r {} -> {:?}", c.group, c.kind, c.radius, p);
        }
    }
}
