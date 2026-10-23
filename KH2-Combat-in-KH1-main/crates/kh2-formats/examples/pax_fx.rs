//! Print the effect programs behind PAX elements.
//!
//! `cargo run -q -p kh2-formats --example pax_fx -- <file.a.us|file.mdlx|file.pax> [element id ...]`
//!
//! Reads the user's own extracted file; prints structure only.

use kh2_formats::bar::Bar;
use kh2_formats::dpd::{effect_source, mask_has, Dpd, TICK};
use kh2_formats::pax::Pax;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: pax_fx <file> [element id ...]");
    let ids: Vec<u16> = args.map(|a| a.parse().expect("element id")).collect();
    let buf = std::fs::read(&path).expect("read file");
    let pax_buf: Vec<u8> = if buf.starts_with(b"PAX_") {
        buf.clone()
    } else {
        let bar = Bar::parse(&buf).expect("BAR");
        bar.entries.iter().find(|e| e.data.starts_with(b"PAX_")).expect("no PAX entry").data.to_vec()
    };
    let pax = Pax::parse(&pax_buf).expect("PAX");
    for el in &pax.elements {
        if !ids.is_empty() && !ids.contains(&el.id) {
            continue;
        }
        println!(
            "element {} -> effect {} bone {:#x} flags {:#x} wait {} fade {} t {:?} r {:?} s {:?}",
            el.id, el.effect, el.bone, el.flags, el.start_wait, el.fadeout_frames, el.translation, el.rotation, el.scale
        );
        let Some((dpd_buf, index)) = effect_source(&pax_buf, el.effect as u32).expect("DPX") else {
            println!("  (no such effect)");
            continue;
        };
        let dpd = match Dpd::parse(dpd_buf) {
            Ok(d) => d,
            Err(e) => {
                println!("  DPD error: {e}");
                continue;
            }
        };
        let pd = &dpd.particles[index];
        println!(
            "  particle data {index}: {} ticks{} | {} textures, {} shapes, {} models in DPD",
            pd.end / TICK,
            if pd.looping { " (loops)" } else { "" },
            dpd.textures.len(),
            dpd.shapes.len(),
            dpd.models.len()
        );
        let used: Vec<usize> = (0..dpd.textures.len()).filter(|&i| mask_has(&pd.use_texture, i)).collect();
        for i in used {
            let t = &dpd.textures[i];
            println!("    texture {i}: {}x{} psm {:#x} tbp {:#x} cbp {:#x}", t.width, t.height, t.psm, t.tbp, t.cbp);
        }
        for (k, e) in pd.emitters.iter().enumerate() {
            let start = e.start_tick().map(|t| t.to_string()).unwrap_or_else(|| "child".into());
            let total = e.total_ticks().map(|t| t.to_string()).unwrap_or_else(|| "endless".into());
            println!("  emitter {k}: start {start} life {total} work {:#x} prio {}", e.work_size, e.priority);
            for p in &e.programs {
                let keys: Vec<i32> = p.keys.iter().map(|k| k.tick()).collect();
                println!("    {:<28} keys at {:?} work {:?}", p.name().unwrap_or("?"), keys, p.work);
            }
        }
    }
}
