//! Research helper: `vbrt <00battle.bin>` prints KH2's pad vibration patterns by trigger id.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let b = kh2_formats::battle::Battle::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    for v in &b.vibration {
        println!("id {}: channel 0 {:?}, channel 1 {:?}", v.id, v.channels[0], v.channels[1]);
    }
}
