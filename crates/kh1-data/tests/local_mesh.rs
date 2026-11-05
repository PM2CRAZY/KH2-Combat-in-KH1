//! KH1 Sora's PS2 mesh and textures (`mdls::parse_faces`, `parse_textures`) and his remastered
//! mesh (`mesh::cvbl_faces`), read from your own files, skipped without them: `xa_ex_0010.mdls`
//! (`KH1_RE_DIR`, else `local/re/kh1` or `local/re`) and his remastered assets as exported
//! (`-c0.cvbl`, `-100.png`, `-1c0..-1c3.png`; `KH1_HDTEX_DIR`, else `local/kh1-hdtex`). And the
//! surface transfer (`texmap`): his PS2 textures carried onto his remastered layout come out
//! close to his own HD textures.

use glam::Vec3;
use kh1_data::image::Image;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn mdls() -> Option<Vec<u8>> {
    let dirs = match std::env::var_os("KH1_RE_DIR") {
        Some(d) => vec![PathBuf::from(d)],
        None => vec![root().join("local/re/kh1"), root().join("local/re")],
    };
    let r = dirs.iter().find_map(|d| std::fs::read(d.join("xa_ex_0010.mdls")).ok());
    if r.is_none() {
        eprintln!("skipped: no xa_ex_0010.mdls in {dirs:?}");
    }
    r
}

pub fn hdtex(name: &str) -> Option<Vec<u8>> {
    let d = std::env::var_os("KH1_HDTEX_DIR").map(PathBuf::from).unwrap_or_else(|| root().join("local/kh1-hdtex"));
    let r = std::fs::read(d.join(name)).ok();
    if r.is_none() {
        eprintln!("skipped: no {name} in {}", d.display());
    }
    r
}

/// A PNG as exported (8-bit RGBA) as B8G8R8A8.
pub fn png(bytes: &[u8]) -> Image {
    let mut d = png::Decoder::new(std::io::Cursor::new(bytes));
    d.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut r = d.read_info().unwrap();
    let mut buf = vec![0; r.output_buffer_size().unwrap()];
    let info = r.next_frame(&mut buf).unwrap();
    assert_eq!((info.color_type, info.bit_depth), (png::ColorType::Rgba, png::BitDepth::Eight));
    let bgra = buf[..info.buffer_size()].chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();
    Image { w: info.width, h: info.height, bgra }
}

#[test]
fn soras_ps2_mesh_and_textures() {
    let Some(m) = mdls() else { return };
    let faces = kh1_data::mdls::parse_faces(&m).unwrap();
    // As his remastered mesh: 2828 triangles; 9 meshes on textures 0..6.
    assert_eq!(faces.len(), 2828);
    let mut slots: Vec<(u32, u32)> = faces.iter().map(|f| (f.mesh, f.slot)).collect();
    slots.dedup();
    assert_eq!(slots, [(0, 0), (1, 1), (2, 2), (3, 3), (4, 3), (5, 4), (6, 5), (7, 5), (8, 6)]);
    let bones = kh1_data::mdls::parse_skeleton(&m).unwrap().len();
    assert!(faces.iter().all(|f| f.corners.iter().all(|c| c.bone < bones && c.uv.x >= -0.01 && c.uv.x <= 1.01)));
    let tex = kh1_data::mdls::parse_textures(&m).unwrap();
    assert_eq!(tex.len(), 7);
    assert!(tex.iter().all(|t| (t.w, t.h) == (128, 128)));
    let Some(hd) = hdtex("-100.png") else { return };
    // His HD `-100` is his PS2 texture 0 drawn at 512²: within a little of it.
    let d = tex[0].difference_from_larger(&png(&hd)).unwrap();
    eprintln!("PS2 texture 0 against -100.png: {d:.2} of 255");
    assert!(d < 2.5, "{d}");
}

#[test]
fn soras_remastered_mesh() {
    let Some(c) = hdtex("-c0.cvbl") else { return };
    let faces = kh1_data::mesh::cvbl_faces(&c).unwrap();
    assert_eq!(faces.len(), 2828);
    let mut slots: Vec<(u32, u32)> = faces.iter().map(|f| (f.mesh, f.slot)).collect();
    slots.dedup();
    assert_eq!(slots, [(0, 0), (1, 1), (2, 2), (3, 3), (4, 1), (5, 1), (6, 1), (7, 1), (8, 1)]);
}

/// Both of Sora's meshes in the bind pose: the PS2 one (with its textures) and the remastered.
fn both() -> Option<(Vec<kh1_data::texmap::Tri>, Vec<Image>, Vec<kh1_data::texmap::Tri>)> {
    let (m, c) = (mdls()?, hdtex("-c0.cvbl")?);
    let bones = kh1_data::texmap::bind_pose(&m).unwrap();
    let ps2 = kh1_data::texmap::posed(&kh1_data::mdls::parse_faces(&m).unwrap(), &bones).unwrap();
    let hd = kh1_data::texmap::posed(&kh1_data::mesh::cvbl_faces(&c).unwrap(), &bones).unwrap();
    Some((ps2, kh1_data::mdls::parse_textures(&m).unwrap(), hd))
}

#[test]
fn the_two_meshes_lie_on_each_other_facing_alike() {
    let Some((ps2, _, hd)) = both() else { return };
    let key = |p: Vec3| ((p.x * 100.0).round() as i64, (p.y * 100.0).round() as i64, (p.z * 100.0).round() as i64);
    let tkey = |t: &kh1_data::texmap::Tri| {
        let mut k = t.p.map(key);
        k.sort();
        k
    };
    let twins: std::collections::HashMap<_, Vec3> = ps2.iter().map(|t| (tkey(t), t.n)).collect();
    let (mut same, mut away) = (0, 0);
    for t in &hd {
        if let Some(n) = twins.get(&tkey(t)) {
            if n.dot(t.n) > 0.0 {
                same += 1;
            } else {
                away += 1;
            }
        }
    }
    eprintln!("remastered triangles with a PS2 twin: {same} facing alike, {away} the other way, of {}", hd.len());
    assert!(same > 2000 && away < same / 20, "{same} {away}");
}

#[test]
fn soras_ps2_textures_onto_his_hd_layout_come_out_as_his_hd_textures() {
    let Some((ps2, tex, hd)) = both() else { return };
    let textures: Vec<Option<Image>> = tex.into_iter().map(Some).collect();
    let src = kh1_data::texmap::Source::new(&ps2, &textures);
    for slot in 0..4u32 {
        let Some(png_bytes) = hdtex(&format!("-1c{slot}.png")) else { return };
        let own = png(&png_bytes);
        let t = std::time::Instant::now();
        let (out, st) = kh1_data::texmap::transfer(&hd, slot, &own, &src);
        // The mean difference over the covered texels (opaque ones: the hair cards' cut-outs
        // aside), per channel.
        let mask: Vec<bool> = {
            let mut m = vec![false; (own.w * own.h) as usize];
            let scale = glam::Vec2::new(own.w as f32, own.h as f32);
            for tri in hd.iter().filter(|t| t.slot == slot) {
                let q = tri.uv.map(|uv| uv * scale);
                let area = (q[1] - q[0]).perp_dot(q[2] - q[0]);
                if area.abs() < 1e-6 {
                    continue;
                }
                let (lo, hi) = (q[0].min(q[1]).min(q[2]), q[0].max(q[1]).max(q[2]));
                for y in (lo.y.floor().max(0.0) as u32)..(hi.y.ceil() as u32).min(own.h) {
                    for x in (lo.x.floor().max(0.0) as u32)..(hi.x.ceil() as u32).min(own.w) {
                        let c = glam::Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                        if (0..3).all(|k| area.signum() * (q[(k + 1) % 3] - q[k]).perp_dot(c - q[k]) >= 0.0) {
                            m[(y * own.w + x) as usize] = true;
                        }
                    }
                }
            }
            m
        };
        let (mut sum, mut n) = (0f64, 0u64);
        for (i, _) in mask.iter().enumerate().filter(|(i, &m)| m && own.bgra[i * 4 + 3] > 128) {
            for k in 0..3 {
                sum += (out.bgra[i * 4 + k] as f64 - own.bgra[i * 4 + k] as f64).abs();
            }
            n += 3;
        }
        let d = sum / n.max(1) as f64;
        // Against: his own texture's difference from its mean colour over the same texels (what
        // a transfer that carried nothing would come to).
        let used: Vec<usize> = (0..mask.len()).filter(|&i| mask[i] && own.bgra[i * 4 + 3] > 128).collect();
        let mean = [0, 1, 2].map(|k| used.iter().map(|&i| own.bgra[i * 4 + k] as f64).sum::<f64>() / used.len().max(1) as f64);
        let flat = used.iter().map(|&i| (0..3).map(|k| (own.bgra[i * 4 + k] as f64 - mean[k]).abs()).sum::<f64>()).sum::<f64>() / (3 * used.len().max(1)) as f64;
        eprintln!("-1c{slot}: {st:?} in {:?}; mean difference from his own over {} texels: {d:.1} of 255 (his own from its mean colour: {flat:.1})", t.elapsed(), n / 3);
        if let Some(dir) = std::env::var_os("KH1_TEXMAP_OUT") {
            let f = std::fs::File::create(PathBuf::from(dir).join(format!("ps2-onto-1c{slot}.png"))).unwrap();
            let mut e = png::Encoder::new(f, out.w, out.h);
            e.set_color(png::ColorType::Rgba);
            let rgba: Vec<u8> = out.bgra.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();
            e.write_header().unwrap().write_image_data(&rgba).unwrap();
        }
        assert!(st.covered > 0 && st.mean_gap < 0.01 && st.max_gap < 0.2 && d < flat / 2.0, "-1c{slot}: {st:?}, {d} (flat {flat})");
    }
}

#[test]
fn both_eyes_are_found_on_his_face() {
    let Some(b) = hdtex("-1c0.png") else { return };
    let face = png(&b);
    let mut out = face.clone();
    let area = kh1_data::texmap::face_eyes_area(face.w, face.h);
    assert_eq!(kh1_data::texmap::solid_eyes(&mut out, &face, area, kh1_data::texmap::ANTI_EYE_RGB), 2);
    // The yellow stays inside the area.
    let [x0, y0, w, h] = area;
    let yellow = |i: usize| out.bgra[i * 4..i * 4 + 3] == [95, 230, 253];
    let n = (0..(face.w * face.h) as usize).filter(|&i| yellow(i)).count();
    let inside = (y0..y0 + h).flat_map(|y| (x0..x0 + w).map(move |x| (y * face.w + x) as usize)).filter(|&i| yellow(i)).count();
    eprintln!("solid eyes: {n} texels, area {area:?}");
    assert!(n > 500 && n == inside);
    if let Some(dir) = std::env::var_os("KH1_TEXMAP_OUT") {
        let f = std::fs::File::create(PathBuf::from(dir).join("eyes-1c0.png")).unwrap();
        let mut e = png::Encoder::new(f, out.w, out.h);
        e.set_color(png::ColorType::Rgba);
        let rgba: Vec<u8> = out.bgra.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();
        e.write_header().unwrap().write_image_data(&rgba).unwrap();
    }
}
