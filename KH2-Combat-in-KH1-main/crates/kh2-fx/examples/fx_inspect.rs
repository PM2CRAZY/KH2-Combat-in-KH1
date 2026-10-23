//! Print the raw key bytes and resources behind PAX elements (research aid).
//!
//! `cargo run -p kh2-fx --example fx_inspect -- <p_ex.pax> <texcommon.dpd> [element ids]`

use kh2_formats::dpd::{effect_source, program_name, Dpd};
use kh2_formats::pax::Pax;

fn main() {
    let mut args = std::env::args().skip(1);
    let pax_path = args.next().expect("pax");
    let common_path = args.next().expect("texcommon");
    let ids: Vec<u16> = args.map(|a| a.parse().unwrap()).collect();
    let pax_buf = std::fs::read(pax_path).unwrap();
    let common = Dpd::parse(&std::fs::read(common_path).unwrap()).unwrap();
    println!("texcommon: {} textures, {} shapes, {} models", common.textures.len(), common.shapes.len(), common.models.len());
    for (i, t) in common.textures.iter().enumerate() {
        println!("  common tex {i}: {}x{} psm {:#x} tbp {:#x} cbp {:#x} bw {}", t.width, t.height, t.psm, t.tbp, t.cbp, t.buffer_width);
    }
    let pax = Pax::parse(&pax_buf).unwrap();
    for el in &pax.elements {
        if !ids.contains(&el.id) {
            continue;
        }
        let (dpd_buf, index) = effect_source(&pax_buf, el.effect as u32).unwrap().unwrap();
        let dpd = Dpd::parse(dpd_buf).unwrap();
        println!("element {} effect {} data {}", el.id, el.effect, index);
        if el.id == ids[0] {
            for (i, t) in dpd.textures.iter().enumerate() {
                println!("  tex {i}: {}x{} psm {:#x} tbp {:#x} cbp {:#x} bw {}", t.width, t.height, t.psm, t.tbp, t.cbp, t.buffer_width);
            }
            for s in &dpd.shapes {
                let f = &s.sequence.first().map(|st| st.frame.tex0);
                println!("  shape {} steps {} tex0 {:?}", s.number, s.sequence.len(), f.map(|t| (format!("{:#x}", t.tbp()), t.tbw(), t.psm(), t.tw(), t.th(), format!("{:#x}", t.cbp()), t.tcc(), t.tfx())));
                for st in &s.sequence {
                    println!("     wait {} loop {} sprites {} {:?}", st.wait, st.loops, st.frame.sprites.len(), st.frame.sprites.iter().map(|sp| (sp.corners, sp.s, sp.t, sp.w, sp.h, sp.rgba, sp.blend, sp.clut_offset, sp.code)).collect::<Vec<_>>());
                }
            }
            for m in &dpd.models {
                println!("  model {} flags {:#x} blend {:#x} fix {} verts {} groups {:?}", m.number, m.flags, m.blend, m.blend_fix, m.vertices.len(),
                    m.groups.iter().map(|g| (g.kind, g.prims.len(), format!("{:#x}", g.tex0.tbp()), g.tex0.tbw(), g.tex0.tw(), g.tex0.th(), format!("{:#x}", g.tex0.cbp()))).collect::<Vec<_>>());
            }
            for v in &dpd.vertex_sets {
                let m = dpd.model(v.model);
                let pts: Vec<[i16; 3]> = v.indices.iter().take(8).filter_map(|&i| m.and_then(|m| m.vertices.get(i as usize)).copied()).collect();
                println!("  vset {} model {} n {} {:?}", v.number, v.model, v.indices.len(), pts);
            }
        }
        let pd = &dpd.particles[index];
        println!("  header offset {:?} angle {:?} scale {:?} chr {} end {} loop {}", pd.offset, pd.angle, pd.scale, pd.chr_scale, pd.end, pd.looping);
        for (k, e) in pd.emitters.iter().enumerate() {
            println!("  emitter {k}: start {:#x} total {:#x} loop {:#x}/{:#x}", e.start, e.total, e.loop_start, e.loop_end);
            for p in &e.programs {
                let name = program_name(p.kind).unwrap_or("?");
                let interesting = !matches!(p.kind, 28..=30 | 36..=38 | 40..=42 | 84 | 87 | 90 | 91 | 94 | 105 | 112 | 113 | 217);
                if interesting {
                    for key in &p.keys {
                        let hex: Vec<String> = key.data.chunks(4).map(|c| c.iter().map(|b| format!("{b:02x}")).collect::<String>()).collect();
                        println!("    {name} {:?} @{}: {}", p.work, key.tick(), hex.join(" "));
                    }
                } else {
                    let vals: Vec<String> = p.keys.iter().map(|k| {
                        let f: Vec<String> = (1..k.data.len() / 4).map(|i| {
                            let v = k.f32(i * 4);
                            if v.is_finite() && (v == 0.0 || v.abs() > 1e-6 && v.abs() < 1e7) { format!("{v}") } else { format!("i{}", k.i32(i * 4)) }
                        }).collect();
                        format!("@{}[{}]", k.tick(), f.join(","))
                    }).collect();
                    println!("    {name} {:?} {}", p.work, vals.join(" "));
                }
            }
        }
    }
}
