//! KH1 Sora's motions: per slot, frames, looping, root travel, and posed bone positions at a
//! few frames (JSON lines, for a contact sheet).
//! `cargo run -p kh1-data --example kh1_motions -- <xa_ex_0010.mdls> <xa_ex_0010.mset> <out.jsonl>`
use std::io::Write;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let skel = kh1_data::mdls::parse_skeleton(&std::fs::read(&a[0]).unwrap()).unwrap();
    let motions = kh1_data::mset::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    let mut out = std::fs::File::create(&a[2]).unwrap();
    writeln!(out, "{:?}", skel.iter().map(|b| b.parent).collect::<Vec<_>>()).unwrap();
    for (i, m) in motions.iter().enumerate() {
        let Some(m) = m else { println!("{i:3}: empty"); writeln!(out, "null").unwrap(); continue };
        let loops = m.channels.iter().filter(|c| c.after == 2).count();
        let root = |f: f32| kh2_anim::Pose::bind(&m.apply(&skel, f)).bone_position(0);
        let (r0, r1) = (root(0.0), root(m.frames as f32));
        println!("{i:3}: off {:#8x} {:4} frames {:.0} fps, {} channels ({} repeat), root {:.0?} -> {:.0?}", m.offset, m.frames, m.fps, m.channels.len(), loops, r0, r1);
        let frames: Vec<f32> = (0..4).map(|k| m.frames as f32 * k as f32 / 4.0).collect();
        let poses: Vec<Vec<f32>> = frames.iter().map(|&f| kh2_anim::Pose::bind(&m.apply(&skel, f)).bones.iter().flat_map(|b| b.w_axis.truncate().to_array()).map(|v| (v * 10.0).round() / 10.0).collect()).collect();
        writeln!(out, "{:?}", poses).unwrap();
    }
}
