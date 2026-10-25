//! The coat (KH1's jacket hem) as `Retarget::pose` makes it over KH2's motions, now turning as
//! KH2's coat ring (`retarget::coat_kh2`; docs/kh1-secondary-motion.md "The coat follows KH2's
//! coat ring"). `<mdls> <KH2 dir> <motions, comma separated | all> [q]`: per motion and chain, how
//! far its tip stands off the hips' up axis against the bind (mean [min, max]) and how far it
//! swings against the chest within the motion (s, degrees); then, over the motions given, how
//! often a tip turns more than 60 / 90 degrees from its bind direction against the chest. `q`
//! prints the totals only.
use glam::{Mat4, Quat, Vec3};
fn rot(m: &Mat4) -> Quat { m.to_scale_rotation_translation().1.normalize() }
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let kh1 = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[1])).unwrap();
    let rig = sora.rig.expect("rig");
    let r = kh1_data::retarget::Retarget::new(&kh1, &rig.skeleton);
    let kb = r.bind();
    let at = |v: &[Mat4], i: usize| v[i].w_axis.truncate();
    let roots = [72usize, 248, 252, 256, 260, 264, 268, 272, 276];
    let mut only: Vec<u16> = rig.motions.keys().copied().collect(); only.sort();
    if a[2] != "all" { only = a[2].split(',').map(|x| x.parse().unwrap()).collect(); }
    let mut per = std::collections::BTreeMap::new();
    let (mut tot, mut over60, mut over90, mut worst) = (0usize, 0usize, 0usize, (0.0f32, 0u16, 0usize, 0usize));
    for id in only {
        let m = &rig.motions[&id];
        let n = kh2_anim::length_frames(m) as usize;
        let mut line = format!("{id:3}:");
        for &r0 in &roots {
            let t = r0 + 3;
            let (mut omin, mut omax, mut osum) = (f32::MAX, f32::MIN, 0.0);
            let mut dirs = vec![];
            for f in 0..n.max(1) {
                let kh2 = kh2_anim::evaluate(&rig.skeleton, m, kh2_anim::key_time(m, f as f32, false), kh2_anim::Options::default()).bones;
                let p = r.pose(&kh2);
                let hips = rot(&p[3]) * rot(&kb[3]).inverse();
                let l = hips.inverse() * (at(&p, t) - at(&p, 3));
                let lb = at(&kb, t) - at(&kb, 3);
                let rad = |v: Vec3| (v.x * v.x + v.z * v.z).sqrt();
                let o = rad(l) - rad(lb);
                omin = omin.min(o); omax = omax.max(o); osum += o;
                let chest = rot(&p[78]) * rot(&kb[78]).inverse();
                let d = (chest.inverse() * (at(&p, t) - at(&p, r0 + 1))).normalize();
                let db = (at(&kb, t) - at(&kb, r0 + 1)).normalize();
                let ang = d.angle_between(db).to_degrees();
                tot += 1; if ang > 60.0 { over60 += 1; } if ang > 90.0 { over90 += 1; *per.entry(id).or_insert(0usize) += 1; }
                if ang > worst.0 { worst = (ang, id, f, r0); }
                dirs.push(d);
            }
            let mean = dirs.iter().fold(Vec3::ZERO, |s, d| s + *d).normalize();
            let sw = dirs.iter().map(|d| d.angle_between(mean).to_degrees()).fold(0.0f32, f32::max);
            line += &format!(" {r0}:{:+.1}[{:+.0},{:+.0}]s{sw:.0}", osum / n.max(1) as f32, omin, omax);
        }
        if a.len() < 4 { println!("{line}"); }
    }
    println!("motions with >90: {:?}", per);
    println!("tip off its bind direction (chest frame): {tot} chain-frames, >60 {:.2}%, >90 {:.2}%, worst {:.0} (motion {} f{} chain {})", 100.0 * over60 as f32 / tot as f32, 100.0 * over90 as f32 / tot as f32, worst.0, worst.1, worst.2, worst.3);
}
