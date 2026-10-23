fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mset = kh2_formats::motion::Mset::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    let want: Vec<usize> = a[2..].iter().map(|s| s.parse().unwrap()).collect();
    for s in &mset.slots {
        if !want.contains(&(s.index / 4)) { continue; }
        let Some(anb) = &s.anb else { continue };
        let Some(m) = anb.pose.as_ref() else { continue };
        println!("motion {} slot {} ({}) frames {}..{} fps {} ret {}", s.index / 4, s.index, s.name, m.frame_start, m.frame_end, m.fps, m.frame_return);
        for r in &anb.triggers.ranges { println!("   range id {} {}..{} {:?}", r.id, r.start, r.end, r.params); }
        for f in &anb.triggers.frames { println!("   frame id {} at {} {:?}", f.id, f.frame, f.params); }
    }
}
