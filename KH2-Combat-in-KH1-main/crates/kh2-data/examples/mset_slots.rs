//! Research helper: `mset_slots <mset> [lo] [hi]` lists each slot's motion, name, frames and its
//! weapon-trail ranges.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mset = kh2_formats::motion::Mset::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    let lo: usize = a.get(2).map_or(0, |s| s.parse().unwrap());
    let hi: usize = a.get(3).map_or(9999, |s| s.parse().unwrap());
    for s in &mset.slots {
        let m = s.index / 4;
        if m < lo || m > hi { continue; }
        let Some(anb) = &s.anb else { continue };
        let trail: Vec<String> = anb.triggers.ranges.iter().filter(|r| r.id == kh2_formats::motion::range_id::WEAPON_TRAIL).map(|r| format!("{}..{}", r.start, r.end)).collect();
        let ranges: Vec<String> = anb.triggers.ranges.iter().map(|r| format!("{}:{}..{}{:?}", r.id, r.start, r.end, r.params)).collect();
        let frames: Vec<String> = anb.triggers.frames.iter().map(|f| format!("{}@{}{:?}", f.id, f.frame, f.params)).collect();
        let len = anb.pose.as_ref().map_or(0.0, |p| p.frame_end);
        println!("m{m:>3} slot {:>4} {:>6} end {len:>6} trail [{}] ranges [{}] frames [{}]", s.index, s.name, trail.join(","), ranges.join(" "), frames.join(" "));
    }
}
