//! KH1 Sora's PS2 mesh and textures (`mdls::parse_faces`, `parse_textures`) and his remastered
//! mesh (`mesh::cvbl_faces`), read from your own files, skipped without them: `xa_ex_0010.mdls`
//! (`KH1_RE_DIR`, else `local/re/kh1` or `local/re`) and his remastered assets as exported
//! (`-c0.cvbl`, `-100.png`; `KH1_HDTEX_DIR`, else `local/kh1-hdtex`).

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

