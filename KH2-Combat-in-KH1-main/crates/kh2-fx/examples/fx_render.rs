//! Render KH2 effects to PNG frames with a small software rasteriser, to
//! eyeball `kh2-fx` against the game.
//!
//! ```text
//! cargo run --release -p kh2-fx --example fx_render -- <dir> <out dir> [explosion|magnet|all]
//!     [--frames N] [--dist D] [--size PX] [--seed S] [--side] [--bg LIFT]
//!     [--flip-z] [--follow-root]
//! ```
//!
//! `--flip-z` maps Sora's y-up model space to the y-down effect world by a
//! half turn about x (y and z negated) instead of the default mirror (y
//! negated). `--follow-root` puts the object root (0x4000) under bone 0's
//! ground position, so effects bound to it follow the motion's lunge. Both
//! are viewing aids for effects offset from the object root (Guard Break's
//! element 32 sits 235 units along -z); neither is settled.
//!
//! `<dir>` holds the user's own extracted files: `P_EX100.a.us` (Sora's
//! BAR with the "p_ex" PAX) or `fx/p_ex.pax`, `fx/texcommon.dpd`, and
//! optionally `P_EX100.mdlx` + `P_EX100.mset` to pose Sora with the real
//! motion (else fixed anchor points). Writes `<scene>_NNN.png` per game
//! frame, `<scene>_sheet.png` (every 4th frame) and `textures.png`.
//! Nothing is written into the repository.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use glam::{Mat4, Vec2, Vec3, Vec4};
use kh2_fx::{Camera, Effects, Frame, Library};

// ---------------------------------------------------------------- PNG

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *t = c;
    }
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c = table[((c ^ b as u32) & 255) as usize] ^ (c >> 8);
    }
    c ^ 0xffff_ffff
}

fn write_png(path: &Path, w: usize, h: usize, rgb: &[u8]) {
    let mut raw = Vec::with_capacity((w * 3 + 1) * h);
    for y in 0..h {
        raw.push(0);
        raw.extend_from_slice(&rgb[y * w * 3..(y + 1) * w * 3]);
    }
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&raw).unwrap();
    let idat = z.finish().unwrap();
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut c = kind.to_vec();
        c.extend_from_slice(data);
        out.extend_from_slice(&c);
        out.extend_from_slice(&crc32(&c).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &idat);
    chunk(b"IEND", &[]);
    std::fs::write(path, out).unwrap();
}

// ---------------------------------------------------------- rasteriser

struct Image {
    w: usize,
    h: usize,
    rgb: Vec<[f32; 3]>,
    depth: Vec<f32>,
}

impl Image {
    /// A vertical gradient; `bg` lifts it (subtractive effects only show
    /// on a lit background).
    fn new(w: usize, h: usize, bg: f32) -> Image {
        let mut rgb = vec![[0.0; 3]; w * h];
        for y in 0..h {
            let t = y as f32 / h as f32;
            for x in 0..w {
                rgb[y * w + x] = [0.06 + 0.04 * t + bg, 0.07 + 0.04 * t + bg, 0.10 + 0.05 * t + bg];
            }
        }
        Image { w, h, rgb, depth: vec![f32::INFINITY; w * h] }
    }

    fn bytes(&self) -> Vec<u8> {
        self.rgb.iter().flat_map(|c| c.map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8)).collect()
    }
}

struct View {
    cam: Camera,
    focal: f32,
}

impl View {
    fn project(&self, v: Vec3, img: &Image) -> Vec2 {
        Vec2::new(img.w as f32 * 0.5 + self.focal * v.x / v.z, img.h as f32 * 0.5 + self.focal * v.y / v.z)
    }
}

#[derive(Clone, Copy)]
struct RV {
    v: Vec3,
    uv: Vec2,
    col: Vec4,
}

fn lerp_rv(a: RV, b: RV, t: f32) -> RV {
    RV { v: a.v.lerp(b.v, t), uv: a.uv.lerp(b.uv, t), col: a.col.lerp(b.col, t) }
}

const NEAR: f32 = 5.0;

fn sample(t: &kh2_fx::Texture, uv: Vec2) -> Vec4 {
    let (w, h) = (t.width as i32, t.height as i32);
    let x = uv.x * w as f32 - 0.5;
    let y = uv.y * h as f32 - 0.5;
    let (x0, y0) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let px = |x: i32, y: i32| {
        let (x, y) = (x.rem_euclid(w) as usize, y.rem_euclid(h) as usize);
        let o = (y * w as usize + x) * 4;
        Vec4::new(t.rgba[o] as f32, t.rgba[o + 1] as f32, t.rgba[o + 2] as f32, t.rgba[o + 3] as f32) / 255.0
    };
    let a = px(x0, y0).lerp(px(x0 + 1, y0), fx);
    let b = px(x0, y0 + 1).lerp(px(x0 + 1, y0 + 1), fx);
    a.lerp(b, fy)
}

fn draw_frame(img: &mut Image, view: &View, lib: &Library, frame: &Frame) {
    let vm = view.cam.view();
    for b in &frame.batches {
        let tex = b.texture.and_then(|t| lib.texture(t));
        for tri in frame.batch_vertices(b).chunks(3) {
            let mut poly: Vec<RV> = tri
                .iter()
                .map(|v| RV { v: vm.transform_point3(Vec3::from(v.pos)), uv: Vec2::from(v.uv), col: Vec4::from(v.color) })
                .collect();
            // Clip against the near plane.
            let mut clipped = Vec::with_capacity(4);
            for i in 0..poly.len() {
                let (a, c) = (poly[i], poly[(i + 1) % poly.len()]);
                if a.v.z >= NEAR {
                    clipped.push(a);
                }
                if (a.v.z >= NEAR) != (c.v.z >= NEAR) {
                    clipped.push(lerp_rv(a, c, (NEAR - a.v.z) / (c.v.z - a.v.z)));
                }
            }
            poly = clipped;
            for i in 1..poly.len().saturating_sub(1) {
                raster(img, view, [poly[0], poly[i], poly[i + 1]], tex, b);
            }
        }
    }
}

fn raster(img: &mut Image, view: &View, t: [RV; 3], tex: Option<&kh2_fx::Texture>, b: &kh2_fx::Batch) {
    let p = t.map(|r| view.project(r.v, img));
    let area = (p[1] - p[0]).perp_dot(p[2] - p[0]);
    if area.abs() < 1e-6 {
        return;
    }
    let minx = p.iter().map(|q| q.x).fold(f32::INFINITY, f32::min).floor().max(0.0) as i32;
    let maxx = p.iter().map(|q| q.x).fold(f32::NEG_INFINITY, f32::max).ceil().min(img.w as f32 - 1.0) as i32;
    let miny = p.iter().map(|q| q.y).fold(f32::INFINITY, f32::min).floor().max(0.0) as i32;
    let maxy = p.iter().map(|q| q.y).fold(f32::NEG_INFINITY, f32::max).ceil().min(img.h as f32 - 1.0) as i32;
    if minx > maxx || miny > maxy {
        return;
    }
    let iz = t.map(|r| 1.0 / r.v.z);
    for y in miny..=maxy {
        for x in minx..=maxx {
            let q = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let w0 = (p[2] - p[1]).perp_dot(q - p[1]) / area;
            let w1 = (p[0] - p[2]).perp_dot(q - p[2]) / area;
            let w2 = 1.0 - w0 - w1;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let izq = w0 * iz[0] + w1 * iz[1] + w2 * iz[2];
            let z = 1.0 / izq;
            let k = [w0 * iz[0] * z, w1 * iz[1] * z, w2 * iz[2] * z];
            let o = y as usize * img.w + x as usize;
            let depth = z + b.depth_offset;
            if b.depth_test && depth >= img.depth[o] {
                continue;
            }
            let uv = t[0].uv * k[0] + t[1].uv * k[1] + t[2].uv * k[2];
            let col = t[0].col * k[0] + t[1].col * k[1] + t[2].col * k[2];
            let tc = tex.map(|t| sample(t, uv)).unwrap_or(Vec4::ONE);
            let src = tc * col;
            if src.w <= 0.0 && tex.is_some() {
                continue;
            }
            let out = b.blend.apply(src.to_array(), img.rgb[o]);
            img.rgb[o] = out.map(|v| v.clamp(0.0, 1.0));
            if b.depth_write {
                img.depth[o] = depth;
            }
        }
    }
}

/// Plain lines for the ground grid and the skeleton.
fn line(img: &mut Image, view: &View, a: Vec3, b: Vec3, col: [f32; 3]) {
    let vm = view.cam.view();
    let (mut va, mut vb) = (vm.transform_point3(a), vm.transform_point3(b));
    if va.z < NEAR && vb.z < NEAR {
        return;
    }
    if va.z < NEAR {
        va = va.lerp(vb, (NEAR - va.z) / (vb.z - va.z));
    }
    if vb.z < NEAR {
        vb = vb.lerp(va, (NEAR - vb.z) / (va.z - vb.z));
    }
    let (pa, pb) = (view.project(va, img), view.project(vb, img));
    let n = (pb - pa).length().ceil().max(1.0) as usize;
    for i in 0..=n {
        let q = pa.lerp(pb, i as f32 / n as f32);
        if q.x >= 0.0 && q.y >= 0.0 && (q.x as usize) < img.w && (q.y as usize) < img.h {
            let o = q.y as usize * img.w + q.x as usize;
            img.rgb[o] = col;
        }
    }
}

// --------------------------------------------------------------- scene

/// Sora posed by his real motion, if the files are there.
struct Sora {
    skel: Vec<kh2_formats::anim::Bone>,
    motions: kh2_formats::motion::Mset,
    tip_local: Vec3,
}

/// KH2 model space is y-up; the effect world is y-down.
fn to_world(flip_z: bool) -> Mat4 {
    Mat4::from_scale(Vec3::new(1.0, -1.0, if flip_z { -1.0 } else { 1.0 }))
}

/// How the example maps Sora into the effect world.
#[derive(Clone, Copy)]
struct Mapping {
    flip_z: bool,
    follow_root: bool,
}

impl Sora {
    fn load(dir: &Path) -> Option<Sora> {
        let mdlx = std::fs::read(dir.join("P_EX100.mdlx")).ok()?;
        let skel = kh2_formats::anim::parse_skeleton(&mdlx).ok()?;
        let motions = kh2_formats::motion::Mset::parse(&std::fs::read(dir.join("P_EX100.mset")).ok()?).ok()?;
        // The Keyblade tip (its collision point 0 in W_EX010's bind pose)
        // in the space of the hand bone 178 it hangs from.
        let bind = kh2_anim::Pose::bind(&skel);
        let tip_local = bind.bones.get(178).map(|h| h.inverse().transform_point3(Vec3::new(-59.7, 108.0, 75.5))).unwrap_or(Vec3::ZERO);
        Some(Sora { skel, motions, tip_local })
    }

    fn pose(&self, motion: usize, frame: f32) -> Option<kh2_anim::Pose> {
        let slot = self.motions.slot_for(motion, true, true)?;
        let m = slot.anb.as_ref()?.pose.as_ref()?;
        let t = kh2_anim::key_time(m, frame, false);
        Some(kh2_anim::evaluate(&self.skel, m, t, Default::default()))
    }
}

struct Anchors {
    pose: Option<kh2_anim::Pose>,
    tip_local: Vec3,
    t: f32,
    map: Mapping,
}

impl Anchors {
    fn bone(&self, code: u16) -> Option<Mat4> {
        let w = to_world(self.map.flip_z);
        match (code, &self.pose) {
            (0x4000, Some(p)) if self.map.follow_root => {
                let r = w.transform_point3(p.bone_position(0));
                Some(Mat4::from_translation(Vec3::new(r.x, 0.0, r.z)))
            }
            (0x4000, _) => Some(Mat4::IDENTITY),
            (0x4001, Some(p)) => Some(Mat4::from_translation(w.transform_point3(p.bone_position(0)))),
            (0x4003..=0x4008, Some(p)) => {
                let h = *p.bones.get(178)?;
                Some(w * h * Mat4::from_translation(self.tip_local))
            }
            (c, Some(p)) => p.bones.get(c as usize).map(|b| w * *b),
            // Without a skeleton: a circling weapon tip, hands at chest height.
            (0x4003..=0x4008, None) => {
                let a = self.t * 0.25;
                Some(Mat4::from_translation(Vec3::new(70.0 * a.cos(), -100.0, 70.0 * a.sin())))
            }
            (c, None) => Some(Mat4::from_translation(Vec3::new(if c % 2 == 0 { 25.0 } else { -25.0 }, -95.0, 10.0))),
        }
    }
}

struct Scene {
    name: &'static str,
    motion: usize,
    /// (motion frame, element id)
    triggers: &'static [(u32, u16)],
    frames: u32,
}

const SCENES: &[Scene] = &[
    Scene { name: "explosion", motion: 166, triggers: &[(0, 44), (0, 45), (0, 46), (0, 47), (34, 48)], frames: 100 },
    Scene { name: "magnet", motion: 197, triggers: &[(10, 51), (60, 52)], frames: 130 },
    Scene { name: "guard_break", motion: 165, triggers: &[(24, 31), (28, 30), (30, 32)], frames: 80 },
    Scene { name: "finishing_leap", motion: 167, triggers: &[(2, 33), (28, 34)], frames: 80 },
    Scene { name: "aerial_recovery", motion: 40, triggers: &[(0, 0)], frames: 40 },
    Scene { name: "aerial_finish", motion: 194, triggers: &[(0, 35), (0, 36), (0, 37), (0, 38)], frames: 80 },
];

#[allow(clippy::too_many_arguments)]
fn run(lib: &Arc<Library>, sora: Option<&Sora>, scene: &Scene, out: &Path, frames: u32, dist: f32, size: usize, seed: u32, side: bool, bg: f32, map: Mapping) {
    let mut fx = Effects::new(lib.clone());
    fx.seed(seed);
    // A game-like camera: about head height plus a bit, looking slightly down.
    let target = Vec3::new(0.0, -95.0, 0.0);
    let eye = if side { Vec3::new(dist, -200.0, 0.0) } else { Vec3::new(dist * 0.34, -200.0, dist * 0.92) };
    let view = View { cam: Camera::look_at(eye, target, Vec3::NEG_Y), focal: size as f32 * 0.5 / (35f32.to_radians()).tan() };
    let mut sheet: Vec<Vec<u8>> = Vec::new();
    let mut frame = Frame::new();
    let mut peak = 0usize;
    for f in 0..frames {
        let anchors = Anchors { pose: sora.and_then(|s| s.pose(scene.motion, f as f32)), tip_local: sora.map(|s| s.tip_local).unwrap_or(Vec3::ZERO), t: f as f32, map };
        let mut bones = |c: u16| anchors.bone(c);
        // (This frame's camera before starting and ticking: camera-space elements, flag 0x20000.)
        fx.set_camera(&view.cam);
        for &(at, id) in scene.triggers {
            if at == f && fx.start(id, &mut bones).is_none() {
                eprintln!("element {id} not found");
            }
        }
        fx.tick(1.0, &mut bones);
        fx.build(&view.cam, &mut frame);
        peak = peak.max(frame.triangle_count());
        let mut img = Image::new(size, size, bg);
        for i in -5..=5 {
            let s = i as f32 * 50.0;
            line(&mut img, &view, Vec3::new(s, 0.0, -250.0), Vec3::new(s, 0.0, 250.0), [0.16, 0.17, 0.2]);
            line(&mut img, &view, Vec3::new(-250.0, 0.0, s), Vec3::new(250.0, 0.0, s), [0.16, 0.17, 0.2]);
        }
        if let (Some(s), Some(p)) = (sora, &anchors.pose) {
            let w = to_world(map.flip_z);
            for (i, b) in s.skel.iter().enumerate() {
                if b.parent >= 0 && (b.parent as usize) < p.bones.len() && i < p.bones.len() {
                    line(&mut img, &view, w.transform_point3(p.bone_position(b.parent as usize)), w.transform_point3(p.bone_position(i)), [0.35, 0.35, 0.4]);
                }
            }
            if let Some(t) = anchors.bone(0x4003) {
                let h = w.transform_point3(p.bone_position(178));
                line(&mut img, &view, h, t.w_axis.truncate(), [0.5, 0.5, 0.3]);
            }
        }
        draw_frame(&mut img, &view, lib, &frame);
        let bytes = img.bytes();
        write_png(&out.join(format!("{}_{:03}.png", scene.name, f)), size, size, &bytes);
        if f % 4 == 0 {
            sheet.push(bytes);
        }
    }
    // Contact sheet: every 4th frame, half size, 6 per row.
    let th = size / 2;
    let cols = 6;
    let rows = sheet.len().div_ceil(cols);
    let (sw, sh) = (cols * th, rows * th);
    let mut s = vec![0u8; sw * sh * 3];
    for (i, img) in sheet.iter().enumerate() {
        let (ox, oy) = ((i % cols) * th, (i / cols) * th);
        for y in 0..th {
            for x in 0..th {
                for c in 0..3 {
                    let mut acc = 0u32;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        acc += img[((y * 2 + dy) * size + x * 2 + dx) * 3 + c] as u32;
                    }
                    s[((oy + y) * sw + ox + x) * 3 + c] = (acc / 4) as u8;
                }
            }
        }
    }
    write_png(&out.join(format!("{}_sheet.png", scene.name)), sw, sh, &s);
    println!("{}: {} frames, peak {} triangles, skipped programs {:?}", scene.name, frames, peak, fx.skipped_programs());
}

fn texture_sheet(lib: &Library, out: &Path) {
    let cell = 256usize;
    let n = lib.textures().len();
    let cols = 6;
    let rows = n.div_ceil(cols).max(1);
    let (w, h) = (cols * cell, rows * cell);
    let mut rgb = vec![0u8; w * h * 3];
    for (i, t) in lib.textures().iter().enumerate() {
        let (ox, oy) = ((i % cols) * cell, (i / cols) * cell);
        for y in 0..cell {
            for x in 0..cell {
                let (tx, ty) = (x * t.width as usize / cell, y * t.height as usize / cell);
                let o = (ty * t.width as usize + tx) * 4;
                let a = t.rgba[o + 3] as u32;
                let checker = if (x / 16 + y / 16) % 2 == 0 { 40 } else { 70 };
                for c in 0..3 {
                    let v = (t.rgba[o + c] as u32 * a + checker * (255 - a)) / 255;
                    rgb[((oy + y) * w + ox + x) * 3 + c] = v as u8;
                }
            }
        }
    }
    write_png(&out.join("textures.png"), w, h, &rgb);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("usage: fx_render <dir> <out dir> [explosion|magnet|all] [--frames N] [--dist D] [--size PX] [--seed S] [--side] [--bg LIFT] [--flip-z] [--follow-root]");
        std::process::exit(1);
    }
    let dir = PathBuf::from(&args[0]);
    let out = PathBuf::from(&args[1]);
    std::fs::create_dir_all(&out).unwrap();
    let which = args.get(2).filter(|a| !a.starts_with("--")).cloned().unwrap_or_else(|| "all".into());
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).and_then(|v| v.parse::<f32>().ok());
    let side = args.iter().any(|a| a == "--side");
    let map = Mapping { flip_z: args.iter().any(|a| a == "--flip-z"), follow_root: args.iter().any(|a| a == "--follow-root") };
    let dist = opt("--dist").unwrap_or(300.0);
    let size = opt("--size").unwrap_or(480.0) as usize;
    let seed = opt("--seed").unwrap_or(1.0) as u32;
    let bg = opt("--bg").unwrap_or(0.0);
    let pax = std::fs::read(dir.join("P_EX100.a.us")).or_else(|_| std::fs::read(dir.join("fx/p_ex.pax"))).expect("P_EX100.a.us or fx/p_ex.pax");
    let common = std::fs::read(dir.join("fx/texcommon.dpd")).or_else(|_| std::fs::read(dir.join("texcommon.dpd"))).ok();
    let lib = Arc::new(Library::load_named(&pax, Some("p_ex"), common.as_deref()).expect("load"));
    println!("{} textures, texcommon: {}", lib.textures().len(), lib.has_common());
    for id in [44u16, 45, 46, 47, 48, 51, 52] {
        let u = lib.unsupported_programs(id);
        if !u.is_empty() {
            println!("element {id}: unsupported {u:?}");
        }
    }
    for (i, t) in lib.textures().iter().enumerate() {
        let used = t.rgba.chunks(4).filter(|p| p[3] != 0 || p[0] != 0).count();
        println!("texture {i}: {}x{} {:x?} ({used} non-empty pixels)", t.width, t.height, lib.texture_source(i as u32));
    }
    texture_sheet(&lib, &out);
    let sora = Sora::load(&dir);
    if sora.is_none() {
        println!("no P_EX100.mdlx/.mset: using fixed anchors");
    }
    for s in SCENES {
        if which == "all" || which == s.name {
            let frames = opt("--frames").map(|f| f as u32).unwrap_or(s.frames);
            run(&lib, sora.as_ref(), s, &out, frames, dist, size, seed, side, bg, map);
        }
    }
}
