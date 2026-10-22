//! Checks a 2D file's art as the KH1 mod reads it: `cargo run -p kh2-formats --example hud_art
//! -- <kh2_x.hed> <KH2 exe> [name] [entry]` (default `field2d/us/zz0field.2dd`, `gaug`): the
//! sequence, its image's PS2 size and its remastered PNG, decoded.

use kh2_formats::bar::Bar;
use kh2_formats::egs::{Archive, KeyTable};
use kh2_formats::seqd::Sequence;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: hud_art <kh2_x.hed> <KH2 exe> [name] [entry]");
        return;
    }
    let name = args.get(3).map_or("field2d/us/zz0field.2dd", |s| s.as_str());
    let entry = args.get(4).map_or("gaug", |s| s.as_str());
    let keys = KeyTable::from_executable(&args[2]).expect("key table");
    let archive = Archive::open(&args[1]).expect("archive");
    let data = archive.read(&keys, name).expect("read");
    let bar = Bar::parse(&data).expect("BAR");
    let seq = bar.entries.iter().find(|e| e.kind == 25 && e.name == entry).expect("sequence");
    let seq = Sequence::parse(seq.data).expect("SEQD");
    let images: Vec<_> = bar.entries.iter().filter(|e| e.kind == 24).collect();
    let k = images.iter().position(|e| e.name == entry).expect("image");
    let imgd = images[k].data;
    let ps2 = (u16::from_le_bytes([imgd[0x1C], imgd[0x1D]]), u16::from_le_bytes([imgd[0x1E], imgd[0x1F]]));
    let assets = archive.read_remastered_all(&keys, name).expect("assets");
    for (n, d) in &assets {
        println!("asset {n}: {} bytes", d.len());
    }
    let (asset, png) = &assets[k];
    let im = kh2_formats::png::decode(png).expect("png");
    let opaque = im.rgba.chunks_exact(4).filter(|p| p[3] > 0).count();
    println!(
        "{name} {entry}: {} animation groups; image {k} ({asset}) {}x{} (PS2 {}x{}), {opaque} texels not clear",
        seq.groups.len(),
        im.width,
        im.height,
        ps2.0,
        ps2.1
    );
}
