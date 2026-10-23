//! Runs against model files extracted into a local folder (never
//! committed): `KH2_LOCAL_DATA`, else `local/re` at the repository root.
//! Skips when the files are absent.

use kh2_formats::mdlx;

fn file(name: &str) -> Option<Vec<u8>> {
    let dir = std::env::var_os("KH2_LOCAL_DATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/re"));
    std::fs::read(dir.join(name)).ok()
}

#[test]
fn sora_mesh_textures_and_skeleton() {
    let Some(d) = file("P_EX100.mdlx") else { return };
    let m = mdlx::parse(&d).unwrap();
    assert_eq!(m.bones.len(), 256);
    assert_eq!(m.groups.len(), 3);
    assert_eq!(m.triangle_count(), 3065);
    assert_eq!(m.textures.len(), 3);
    assert_eq!((m.textures[0].width, m.textures[0].height), (256, 256));
    assert_eq!((m.textures[2].width, m.textures[2].height), (128, 64));
    for g in &m.groups {
        assert!(g.texture < m.textures.len());
        for t in &g.triangles {
            for v in t {
                assert!((v.bone as usize) < m.bones.len());
                assert!(v.uv.iter().all(|c| (-0.01..=1.01).contains(c)), "{:?}", v.uv);
                assert!(v.position.iter().all(|c| c.is_finite() && c.abs() < 500.0));
            }
        }
    }
    // Textures are mostly opaque and not one flat colour.
    for t in &m.textures {
        assert_eq!(t.rgba.len(), (t.width * t.height * 4) as usize);
        let opaque = t.rgba.chunks(4).filter(|p| p[3] == 255).count();
        assert!(opaque * 4 > t.rgba.len() / 4, "{opaque} of {}", t.rgba.len() / 4);
        let first = &t.rgba[..4];
        assert!(t.rgba.chunks(4).any(|p| p != first));
    }
}

#[test]
fn forms_and_keyblade_parse() {
    for (name, groups) in [("P_EX100_ULTF.mdlx", 3), ("P_EX100_HTLF.mdlx", 3), ("P_EX100_TRIF.mdlx", 3), ("W_EX010.mdlx", 2), ("M_EX010.mdlx", 4)] {
        let Some(d) = file(name) else { continue };
        let m = mdlx::parse(&d).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(m.groups.len(), groups, "{name}");
        assert!(m.triangle_count() > 100, "{name}");
    }
    if let Some(d) = file("W_EX010.mdlx") {
        let m = mdlx::parse(&d).unwrap();
        assert_eq!(m.bones.len(), 12);
        assert!(!m.groups[0].translucent() && m.groups[1].translucent());
    }
}
