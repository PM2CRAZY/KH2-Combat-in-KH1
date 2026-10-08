//! Research helper: `dump <mdlx> <mset> <slot> <game_frame> [fk]` prints the
//! posed joint positions as JSON (bones then helpers) for plotting.
use kh2_formats::anim::parse_skeleton;
use kh2_formats::motion::Mset;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let skel = parse_skeleton(&std::fs::read(&a[1]).unwrap()).unwrap();
    let mset = Mset::parse(&std::fs::read(&a[2]).unwrap()).unwrap();
    let slot: usize = a[3].parse().unwrap();
    let frame: f32 = a[4].parse().unwrap();
    let fk = a.get(5).is_some_and(|s| s == "fk");
    let m = mset.slots[slot].anb.as_ref().unwrap().pose.as_ref().unwrap();
    let t = kh2_anim::key_time(m, frame, true);
    let pose = kh2_anim::evaluate(&skel, m, t, kh2_anim::Options { constraints: !fk });
    let pts = |v: &[glam::Mat4]| v.iter().map(|w| format!("[{:.2},{:.2},{:.2}]", w.w_axis.x, w.w_axis.y, w.w_axis.z)).collect::<Vec<_>>().join(",");
    let parents: Vec<String> = skel.iter().map(|b| b.parent.to_string()).collect();
    let hp: Vec<String> = m.ik_helpers.iter().map(|b| b.parent.to_string()).collect();
    println!("{{\"bones\":[{}],\"parents\":[{}],\"helpers\":[{}],\"hparents\":[{}],\"bc\":{}}}", pts(&pose.bones), parents.join(","), pts(&pose.helpers), hp.join(","), m.bone_count);
}
