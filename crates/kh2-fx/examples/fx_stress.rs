//! Play every element of PAX packages headlessly and report triangle
//! counts and the program types `kh2-fx` skips (a smoke test on files
//! other than Sora's).
//!
//! `cargo run --release -p kh2-fx --example fx_stress -- <texcommon.dpd> <file.pax|file.bar|.mag|.a.us> ...`

use std::sync::Arc;

use glam::{Mat4, Vec3};
use kh2_fx::{Camera, Effects, Frame, Library};

fn main() {
    let mut args = std::env::args().skip(1);
    let common = std::fs::read(args.next().expect("texcommon.dpd")).ok();
    let cam = Camera::look_at(Vec3::new(100.0, -200.0, 300.0), Vec3::new(0.0, -90.0, 0.0), Vec3::NEG_Y);
    for path in args {
        let bytes = std::fs::read(&path).expect("read");
        let lib = match Library::load(&bytes, common.as_deref()) {
            Ok(l) => Arc::new(l),
            Err(e) => {
                println!("{path}: {e}");
                continue;
            }
        };
        for w in lib.warnings() {
            println!("{path}: warning {w}");
        }
        let ids: Vec<u16> = {
            let mut v: Vec<u16> = lib.elements().iter().map(|e| e.id).collect();
            v.dedup();
            v
        };
        let mut fx = Effects::new(lib.clone());
        let mut frame = Frame::new();
        let mut peak = 0;
        let mut started = 0;
        let mut bones = |code: u16| Some(Mat4::from_translation(Vec3::new((code % 7) as f32 * 10.0, -100.0, 0.0)));
        for &id in &ids {
            if fx.start(id, &mut bones).is_some() {
                started += 1;
            }
        }
        for f in 0..600 {
            fx.tick(1.0, &mut bones);
            fx.build(&cam, &mut frame);
            peak = peak.max(frame.triangle_count());
            if f == 300 {
                fx.clear();
                for &id in &ids {
                    if let Some(h) = fx.start(id, &mut bones) {
                        fx.end(h);
                    }
                }
            }
        }
        for v in &frame.vertices {
            assert!(v.pos.iter().all(|c| c.is_finite()), "non-finite vertex in {path}");
        }
        println!(
            "{path}: {} elements, {started} started, {} textures, peak {peak} triangles, still running {}, skipped {:?}",
            ids.len(),
            lib.textures().len(),
            fx.running_count(),
            fx.skipped_programs()
        );
    }
}
