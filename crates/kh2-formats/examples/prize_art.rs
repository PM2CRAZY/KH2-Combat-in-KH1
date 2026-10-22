//! Prints KH2's prize art as the KH1 mod reads it (`docs/kh1-prizes.md`): `cargo run -p kh2-formats
//! --example prize_art -- <obj/PRIZE.mdlx> [its remastered page .dds]`. Reads the user's own
//! extracted file; prints structure only.

use kh2_formats::prize::{PrizeArt, DRIVE_BIG, DRIVE_SMALL, ENTRIES};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: prize_art <obj/PRIZE.mdlx> [page.dds]");
        return;
    };
    let art = PrizeArt::parse(&std::fs::read(path).expect("read")).expect("prize art");
    println!("speed {}", art.speed);
    for t in 0..ENTRIES {
        let e = art.entries[t];
        println!("type {t}: shape {:?} rgba {:02x?} scale {}", e.shape, e.rgba, e.scale);
    }
    for t in [DRIVE_SMALL, DRIVE_BIG] {
        let Some(n) = art.entries[t].shape else { continue };
        let s = art.shape(n).expect("shape");
        let page = art.page(s.tex0).expect("page");
        println!(
            "type {t}: shape {n}, {} steps, {} frames, {} bytes, reach {}, tbp {:#x} cbp {:#x}; page {} ({}x{}, {} texels not clear)",
            s.steps,
            s.frames.len(),
            s.animation.len(),
            s.extent,
            s.tex0.tbp(),
            s.tex0.cbp(),
            page.index,
            page.width,
            page.height,
            page.rgba.chunks(4).filter(|p| p[3] > 0).count()
        );
    }
    if let Some(dds) = args.get(2) {
        let im = kh2_formats::dds::decode(&std::fs::read(dds).expect("read dds")).expect("dds");
        println!("{dds}: {}x{}, {} texels not clear", im.width, im.height, im.rgba.chunks(4).filter(|p| p[3] > 0).count());
    }
}
