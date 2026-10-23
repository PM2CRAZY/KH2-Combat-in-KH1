//! Prints a 2D file's sequences: `cargo run -p kh2-formats --example seqd_info -- <file.2dd>
//! [entry] [group frame...]` (an extracted `.2dd` from the user's own install; e.g.
//! `field2d/us/zz0field.2dd`, entry `gaug`, groups `23 30 36 300`).

use kh2_formats::bar::Bar;
use kh2_formats::seqd::Sequence;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: seqd_info <file.2dd> [entry] [group frame...]");
        return;
    };
    let data = std::fs::read(path).expect("read");
    let bar = Bar::parse(&data).expect("BAR");
    for e in &bar.entries {
        println!("{:>4} {:<4} {} bytes", e.kind, e.name, e.data.len());
    }
    let Some(name) = args.get(2) else { return };
    let Some(e) = bar.entries.iter().find(|e| e.kind == 25 && &e.name == name) else {
        eprintln!("no sequence {name}");
        return;
    };
    let s = Sequence::parse(e.data).expect("SEQD");
    println!("{name}: {} sprites, {} parts, {} part groups, {} animations, {} animation groups", s.sprites.len(), s.parts.len(), s.part_groups.len(), s.animations.len(), s.groups.len());
    for pair in args[3..].chunks(2) {
        let (g, f): (usize, i32) = (pair[0].parse().unwrap(), pair.get(1).map_or(0, |v| v.parse().unwrap()));
        let fr = s.frame(g, f);
        let mut q = Vec::new();
        s.quads(g, fr, &mut q);
        println!("group {g} at {f} (frame {fr}, length {}, attach {:?}, origin {:?}):", s.length(g), s.groups[g].attach, s.origin(g, fr));
        for q in q {
            println!("  {:?} uv {:?} colour {:?}", q.corners.map(|c| [c[0].round(), c[1].round()]), q.uv, q.colours[0].map(|c| c.round()));
        }
    }
}
