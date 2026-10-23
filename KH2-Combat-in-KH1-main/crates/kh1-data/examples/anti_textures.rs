//! Carries KH1 Anti-Sora's textures onto Sora's remastered UV layout (`kh1_data::texmap`), from
//! the user's own install (or files extracted from it), and writes the results as raw BGRA
//! (`.bgra`, width and height in the name) to look at. Nothing is kept in the repository.
//!
//! `cargo run --release -p kh1-data --example anti_textures -- <KH1 install dir> <out dir>`, or
//! `... -- <xa_ex_0010.mdls> <its remastered dir> <xa_pp_3020.mdls> <its remastered dir> <out dir>`

use glam::Vec3;
use kh1_data::texmap::{self, Image};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const SORA: &str = "xa_ex_0010.mdls";
const ANTI: &str = "xa_pp_3020.mdls";

fn read(p: &Path) -> Vec<u8> {
    std::fs::read(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn bounds(t: &[texmap::Tri]) -> (Vec3, Vec3) {
    t.iter().flat_map(|t| t.p).fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(a, b), p| (a.min(p), b.max(p)))
}

/// Where the files come from: the install's packages, or extracted files (the model, its HD
/// assets' folder) per model.
enum From {
    Install(kh1_data::Files),
    Dirs([(PathBuf, PathBuf); 2]),
}

impl From {
    fn model(&self, name: &str) -> Vec<u8> {
        match self {
            From::Install(f) => f.read(name).unwrap(),
            From::Dirs(d) => read(&d[(name == ANTI) as usize].0),
        }
    }

    fn asset(&self, name: &str, asset: &str) -> Option<Vec<u8>> {
        match self {
            From::Install(f) => f.read_remastered(name, asset).ok().map(|(n, b)| {
                println!("{name} {n}: {} bytes", b.len());
                b
            }),
            From::Dirs(d) => {
                let ext = if asset == "-c0" { "cvbl" } else { "dds" };
                std::fs::read(d[(name == ANTI) as usize].1.join(format!("{asset}.{ext}"))).ok()
            }
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (from, out) = match &a[..] {
        [install, out] => {
            let i = kh1_data::Kh1Install::locate(install).unwrap();
            (From::Install(i.files(kh1_data::Source::Live).unwrap()), Path::new(out))
        }
        [sora_mdls, sora_dir, anti_mdls, anti_dir, out] => (From::Dirs([(sora_mdls.into(), sora_dir.into()), (anti_mdls.into(), anti_dir.into())]), Path::new(out)),
        _ => {
            eprintln!("usage: anti_textures <KH1 install dir> <out dir>");
            eprintln!("   or: anti_textures <sora mdls> <sora remastered dir> <anti mdls> <anti remastered dir> <out dir>");
            std::process::exit(2);
        }
    };
    let mesh = |model: &str| {
        let b = texmap::bind_pose(&from.model(model)).unwrap();
        println!("{model}: {} bones", b.len());
        texmap::cvbl_triangles(&from.asset(model, "-c0").expect("no -c0.cvbl"), &b).unwrap()
    };
    let (sora, anti) = (mesh(SORA), mesh(ANTI));
    println!("Sora: {} triangles, bounds {:?}", sora.len(), bounds(&sora));
    println!("Anti-Sora: {} triangles, bounds {:?}", anti.len(), bounds(&anti));
    let textures: Vec<Option<Image>> = (0..7).map(|s| from.asset(ANTI, &format!("-1c{s}")).and_then(|b| Image::from_dds(&b))).collect();
    exact_report(&sora, &anti);
    let src = texmap::Source::new(&anti, &textures);
    std::fs::create_dir_all(out).unwrap();
    let save = |name: &str, img: &Image| std::fs::write(out.join(format!("{name}_{}x{}.bgra", img.w, img.h)), &img.bgra).unwrap();
    let mut face = None;
    for slot in 0..4u32 {
        let base = Image::from_dds(&from.asset(SORA, &format!("-1c{slot}")).unwrap()).unwrap();
        let t = std::time::Instant::now();
        let (img, st) = texmap::transfer(&sora, slot, &base, &src);
        println!("-1c{slot}: {st:?} in {:?}", t.elapsed());
        save(&format!("anti-1c{slot}"), &img);
        if slot == 0 {
            face = Some((base, img));
        }
    }
    let (orig, dark) = face.unwrap();
    for sheet in ["-2c0", "-2c1"] {
        let s = Image::from_dds(&from.asset(SORA, sheet).unwrap()).unwrap();
        let found = texmap::find_frame(&orig, &s, 128);
        println!("{sheet}: first frame at {found:?}");
        if let Some((x, y, _)) = found {
            let mut img = texmap::frames_from(&dark, x, y, &s, 128);
            if sheet == "-2c0" {
                let open = texmap::solid_eye_frames(&mut img, &s, 128, texmap::ANTI_EYE_RGB);
                println!("{sheet}: solid eyes on {open} of {} frames (the rest shut)", s.h / 128);
            }
            save(&format!("anti{sheet}"), &img);
        }
    }
    // The face with Anti Form's solid eyes, as the mod makes it (`tint::bake_anti`).
    let mut eyes = dark.clone();
    let [u0, v0, u1, v1] = texmap::SORA_FACE_EYES;
    let (w, h) = (eyes.w as f32, eyes.h as f32);
    let area = [(u0 * w) as u32, (v0 * h) as u32, ((u1 - u0) * w).ceil() as u32, ((v1 - v0) * h).ceil() as u32];
    let n = texmap::solid_eyes(&mut eyes, &orig, area, texmap::ANTI_EYE_RGB);
    println!("-1c0: {n} solid eyes in {area:?}");
    save("anti-1c0-eyes", &eyes);
}

/// How many of Sora's triangles have a twin (the same three corners) in Anti-Sora's mesh, and
/// which textures they pair.
fn exact_report(sora: &[texmap::Tri], anti: &[texmap::Tri]) {
    let key = |p: Vec3| ((p.x * 100.0).round() as i64, (p.y * 100.0).round() as i64, (p.z * 100.0).round() as i64);
    let tkey = |t: &texmap::Tri| {
        let mut k = t.p.map(key);
        k.sort();
        k
    };
    let mut m: HashMap<_, Vec<usize>> = HashMap::new();
    for (i, t) in anti.iter().enumerate() {
        m.entry(tkey(t)).or_default().push(i);
    }
    let (mut hit, mut flip, mut multi) = (0, 0, 0);
    let mut by = HashMap::new();
    for t in sora {
        if let Some(v) = m.get(&tkey(t)) {
            hit += 1;
            multi += (v.len() > 1) as usize;
            flip += (anti[v[0]].n.dot(t.n) < 0.0) as usize;
            *by.entry((t.slot, anti[v[0]].slot)).or_insert(0) += 1;
        }
    }
    println!("exact: {hit} of {} Sora triangles have an Anti-Sora twin ({flip} facing the other way, {multi} with more than one)", sora.len());
    let mut by: Vec<_> = by.into_iter().collect();
    by.sort();
    println!("textures paired (Sora's slot, Anti-Sora's): {by:?}");
}
