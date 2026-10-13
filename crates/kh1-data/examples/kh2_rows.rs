//! Scratch: KH2 Sora's combo rows (PTYA) and the motions they play.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&a[0])).unwrap();
    let rig = sora.rig.unwrap();
    for r in &sora.character.moveset.combo_rows {
        let len = rig.motions.get(&r.motion).map_or(0.0, |m| kh2_anim::length_frames(m));
        println!("{:3} kind {} sub {:3} off {:2} flag {:08x} motion {:3} ({:4.0}f) next {:3} jump {:5.1} ability {:4} near {:6.1} far {:6.1}", r.id, r.kind, r.sub, r.combo_offset, r.flag, r.motion, len, r.next_motion, r.jump, r.ability, r.near, r.far);
    }
    for m in [3u16, 4, 5, 205, 206, 207, 208] {
        println!("motion {m}: {:?} field {:?}", rig.motions.get(&m).map(|x| kh2_anim::length_frames(x)), rig.field_motions.get(&m).map(|x| kh2_anim::length_frames(x)));
    }
    let mut f: Vec<_> = rig.field_motions.keys().collect(); f.sort(); println!("field variants: {f:?}");
}
