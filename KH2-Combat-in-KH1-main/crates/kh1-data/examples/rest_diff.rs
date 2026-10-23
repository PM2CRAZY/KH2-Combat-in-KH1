//! KH1 bones no KH2 part turns: how far each one's local turn in a KH1 motion (default slot 1,
//! the battle idle, frame 0) is from its bind local, with parent and position.
//! `cargo run -p kh1-data --example rest_diff -- <xa_ex_0010.mdls> <xa_ex_0010.mset> <KH2 dir> [slot] [frame]`
use kh1_data::retarget::Retarget;
use glam::Quat;

fn local_rot(b: &kh2_formats::anim::Bone) -> Quat {
    let r = b.rotation;
    Quat::from_rotation_z(r[2]) * Quat::from_rotation_y(r[1]) * Quat::from_rotation_x(r[0])
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let motions = kh1_data::mset::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[2])).unwrap();
    let rig = sora.rig.expect("KH2 rig");
    let slot: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let frame: f32 = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let r = Retarget::new(&kh1, &rig.skeleton);
    let m = motions[slot].as_ref().expect("slot");
    let posed = m.apply(&kh1, frame);
    let bind = kh2_anim::Pose::bind(&kh1).bones;
    for i in 0..kh1.len() {
        if r.is_mapped(i) {
            continue;
        }
        let angle = local_rot(&kh1[i]).angle_between(local_rot(&posed[i])).to_degrees();
        let dt = (glam::Vec3::from(kh1[i].translation) - glam::Vec3::from(posed[i].translation)).length();
        if angle > 10.0 || dt > 1.0 {
            println!("{i:3} parent {:3} at {:.0?}: {angle:.0} deg, moved {dt:.1}", kh1[i].parent, bind[i].w_axis.truncate());
        }
    }
}
