//! Research helper: `events <mset> <id>...` lists slots with frame triggers of
//! the given ids.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mset = kh2_formats::motion::Mset::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    let ids: Vec<u8> = a[2..].iter().map(|s| s.parse().unwrap()).collect();
    for s in &mset.slots {
        let Some(anb) = &s.anb else { continue };
        for f in &anb.triggers.frames {
            if ids.contains(&f.id) {
                println!("motion {} ({}) frame {} id {} params {:?}", s.index / 4, s.name, f.frame, f.id, f.params);
            }
        }
    }
}
