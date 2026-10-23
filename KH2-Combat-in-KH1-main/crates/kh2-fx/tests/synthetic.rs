//! End-to-end checks on a small effect package built in memory (no game
//! files): one emitter that places a 32-unit billboard sprite, coloured
//! by a Color program, for three effect ticks.

use std::sync::Arc;

use glam::{Mat4, Vec3};
use kh2_fx::{Blend, Camera, Effects, Frame, Library};

fn put(b: &mut Vec<u8>, at: usize, bytes: &[u8]) {
    if b.len() < at + bytes.len() {
        b.resize(at + bytes.len(), 0);
    }
    b[at..at + bytes.len()].copy_from_slice(bytes);
}
fn w32(b: &mut Vec<u8>, at: usize, v: u32) {
    put(b, at, &v.to_le_bytes());
}
fn wf(b: &mut Vec<u8>, at: usize, v: f32) {
    put(b, at, &v.to_le_bytes());
}

const TICK: u32 = 0x1000;

/// (kind, key size, keys (each without its frame word: frame, data), work offsets)
type Prog = (u32, u16, Vec<(u32, Vec<u8>)>, Vec<i32>);

fn f3(x: f32, y: f32, z: f32) -> Vec<u8> {
    [x, y, z].iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// A particle data: header + program file with one emitter.
fn particle_data(total_ticks: u32, end_ticks: u32, progs: &[Prog]) -> Vec<u8> {
    let mut b = vec![0u8; 0x110];
    for i in 0..3 {
        wf(&mut b, 0xa0 + i * 4, 1.0);
    }
    let f = 0x110;
    // Program file header; type list and emitter list are placed at the end.
    w32(&mut b, f, end_ticks * TICK);
    w32(&mut b, f + 4, 0);
    let e = 0x10;
    let n = progs.len();
    let rec = e + 0x28;
    let mut at = rec + n * 16;
    let mut layout = Vec::new();
    for (_, size, keys, work) in progs {
        let keys_at = at;
        at += keys.len() * *size as usize + 4;
        let work_at = at;
        at += work.len() * 4;
        layout.push((keys_at, work_at));
    }
    let cv = at;
    at += 16;
    let kinds_at = at;
    at += 4 + 4 * n;
    let list_at = at;
    // Emitter header.
    let p = |o: usize| f + o;
    w32(&mut b, p(e), 0);
    w32(&mut b, p(e + 4), cv as u32);
    w32(&mut b, p(e + 16), 0);
    w32(&mut b, p(e + 20), total_ticks * TICK);
    w32(&mut b, p(e + 24), 0x7000_0000);
    w32(&mut b, p(e + 28), 0x7000_0000);
    // Room for a trail's ring (KeShpTail3X needs 0x1FB bytes).
    w32(&mut b, p(e + 32), 0xa0 + 0x400);
    put(&mut b, p(e + 36), &0i16.to_le_bytes());
    put(&mut b, p(e + 38), &(n as i16).to_le_bytes());
    for (i, (kind, size, keys, work)) in progs.iter().enumerate() {
        let r = rec + i * 16;
        let (keys_at, work_at) = layout[i];
        w32(&mut b, p(r), *kind);
        put(&mut b, p(r + 4), &size.to_le_bytes());
        put(&mut b, p(r + 6), &(keys.len() as u16).to_le_bytes());
        w32(&mut b, p(r + 8), keys_at as u32);
        w32(&mut b, p(r + 12), work_at as u32);
        for (k, (frame, data)) in keys.iter().enumerate() {
            let ka = keys_at + k * *size as usize;
            w32(&mut b, p(ka), frame * TICK);
            put(&mut b, p(ka + 4), data);
            if b.len() < p(ka + *size as usize) {
                b.resize(p(ka + *size as usize), 0);
            }
        }
        w32(&mut b, p(keys_at + keys.len() * *size as usize), 0xffff_f000);
        for (j, o) in work.iter().enumerate() {
            put(&mut b, p(work_at + j * 4), &o.to_le_bytes());
        }
    }
    w32(&mut b, p(kinds_at), n as u32);
    for (i, (kind, ..)) in progs.iter().enumerate() {
        w32(&mut b, p(kinds_at + 4 + i * 4), *kind);
    }
    w32(&mut b, p(list_at), 1);
    w32(&mut b, p(list_at + 4), e as u32);
    w32(&mut b, f + 8, kinds_at as u32);
    w32(&mut b, f + 12, list_at as u32);
    b
}

/// A 4x4 texture at TBP 0x3000 / CBP 0x3E00, every pixel index 1 = red.
fn texture() -> Vec<u8> {
    let mut t = vec![0u8; 0x20];
    t[0..2].copy_from_slice(&0x3000u16.to_le_bytes());
    t[2..4].copy_from_slice(&0x3e00u16.to_le_bytes());
    t[4..6].copy_from_slice(&2i16.to_le_bytes());
    t[6..8].copy_from_slice(&0x13i16.to_le_bytes());
    t[12..14].copy_from_slice(&4u16.to_le_bytes());
    t[14..16].copy_from_slice(&4u16.to_le_bytes());
    t.extend(vec![1u8; 16]);
    let mut pal = vec![0u8; 1024];
    pal[4..8].copy_from_slice(&[255, 0, 0, 0x80]);
    t.extend(pal);
    t
}

/// Shape 0: one step, one 32x32 rectangle sprite over the whole texture.
fn shape() -> Vec<u8> {
    let mut b = vec![0u8; 16];
    let a = 16;
    put(&mut b, a + 4, &1i16.to_le_bytes());
    put(&mut b, a + 6, &1i16.to_le_bytes());
    put(&mut b, a + 16, &0x20i16.to_le_bytes());
    put(&mut b, a + 18, &512u16.to_le_bytes());
    put(&mut b, a + 20, &[0x80]);
    let fr = a + 0x20;
    put(&mut b, fr, &0i16.to_le_bytes());
    put(&mut b, fr + 2, &1i16.to_le_bytes());
    w32(&mut b, fr + 4, 0x30);
    let tex0: u64 = 0x3000 | 2 << 14 | 0x13 << 20 | 2 << 26 | 2 << 30 | 1 << 34 | 0x3e00 << 37;
    put(&mut b, fr + 16, &tex0.to_le_bytes());
    for (i, v) in [-256i16, -256, 256, 256, 0, 0, 4096, 4096].iter().enumerate() {
        put(&mut b, fr + 0x20 + i * 2, &v.to_le_bytes());
    }
    put(&mut b, fr + 0x30, &[0x80, 0x80, 0x80, 0x80, 0x48, 0, 0, 0]);
    b
}

fn dpd(pd: Vec<u8>) -> Vec<u8> {
    let parts = [pd, texture(), shape()];
    let mut b = Vec::new();
    w32(&mut b, 0, 0x96);
    // Lists: particle data, textures, shapes, then empty models and vertex sets.
    let head = 4 + 3 * 8 + 2 * 4 + 16;
    let mut at = (head + 15) & !15;
    let mut o = 4;
    for p in &parts {
        w32(&mut b, o, 1);
        w32(&mut b, o + 4, at as u32);
        o += 8;
        put(&mut b, at, p);
        at = (at + p.len() + 15) & !15;
    }
    w32(&mut b, o, 0);
    w32(&mut b, o + 4, 0);
    b
}

/// A PAX with element `id` playing effect 0.
fn pax(id: u16, flags: u32, wait: f32, fadeout: u8, dpd: Vec<u8>) -> Vec<u8> {
    let mut b = Vec::new();
    put(&mut b, 0, b"PAX_");
    w32(&mut b, 8, 1);
    let el = 16;
    put(&mut b, el + 2, &id.to_le_bytes());
    b.resize(el + 6, 0);
    b[el + 5] = fadeout;
    put(&mut b, el + 6, &0x4000u16.to_le_bytes());
    w32(&mut b, el + 16, flags);
    wf(&mut b, el + 20, wait);
    for i in 0..3 {
        wf(&mut b, el + 52 + i * 4, 1.0);
    }
    let dpx = 16 + 80;
    w32(&mut b, 12, dpx as u32);
    w32(&mut b, dpx, 130);
    w32(&mut b, dpx + 12, 1);
    w32(&mut b, dpx + 16, 0x40);
    w32(&mut b, dpx + 20, 0);
    w32(&mut b, dpx + 24, 0);
    put(&mut b, dpx + 0x40, &dpd);
    b
}

fn sprite_effect(total: u32, end: u32) -> Vec<u8> {
    sprite_effect_with(total, end, (113, vec![48]))
}

/// The sprite effect with another draw-matrix program in place of
/// DrawMatrixFront (work bytes 96..160 are free for it).
fn sprite_effect_with(total: u32, end: u32, (matrix, work): (u32, Vec<i32>)) -> Vec<u8> {
    let progs: Vec<Prog> = vec![
        (40, 16, vec![(0, f3(10.0, 0.0, 0.0))], vec![0]),
        (42, 16, vec![(0, f3(2.0, 2.0, 2.0))], vec![16]),
        // Colour 0x4000 = 0x80 in each channel; tick 1 adds -0x2000 alpha.
        (43, 12, vec![(0, [0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40].to_vec()), (1, [0, 0, 0, 0, 0, 0, 0x00, 0xe0].to_vec())], vec![28]),
        (94, 4, vec![(0, vec![])], vec![0, 16]),
        (matrix, 4, vec![(0, vec![])], work),
        (142, 20, vec![(0, [0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0].to_vec())], vec![64, 28, 48, 0]),
    ];
    dpd(particle_data(total, end, &progs))
}

fn setup(flags: u32, wait: f32, fadeout: u8) -> (Effects, Camera) {
    let lib = Library::load(&pax(7, flags, wait, fadeout, sprite_effect(3, 4)), None).expect("synthetic package");
    assert_eq!(lib.textures().len(), 1);
    let fx = Effects::new(Arc::new(lib));
    let cam = Camera::look_at(Vec3::new(10.0, 0.0, -100.0), Vec3::new(10.0, 0.0, 0.0), Vec3::NEG_Y);
    (fx, cam)
}

fn frames(fx: &mut Effects, cam: &Camera, n: usize) -> Vec<Frame> {
    let mut bones = |_: u16| Some(Mat4::IDENTITY);
    (0..n)
        .map(|_| {
            fx.tick(1.0, &mut bones);
            let mut f = Frame::new();
            fx.build(cam, &mut f);
            f
        })
        .collect()
}

#[test]
fn texture_decodes() {
    let lib = Library::load(&pax(7, 0, 0.0, 0, sprite_effect(3, 4)), None).unwrap();
    let t = &lib.textures()[0];
    assert_eq!((t.width, t.height), (4, 4));
    assert_eq!(&t.rgba[0..4], &[255, 0, 0, 255]);
    assert!(lib.unsupported_programs(7).is_empty());
}

#[test]
fn billboard_sprite_lifetime_and_colour() {
    let (mut fx, cam) = setup(0, 0.0, 0);
    let mut bones = |_: u16| Some(Mat4::IDENTITY);
    assert!(fx.start(7, &mut bones).is_some());
    assert!(fx.start(8, &mut bones).is_none());
    let fr = frames(&mut fx, &cam, 10);
    // Three ticks of two frames each, then the instance has expired.
    let tris: Vec<usize> = fr.iter().map(|f| f.triangle_count()).collect();
    assert_eq!(tris, vec![2, 2, 2, 2, 2, 2, 0, 0, 0, 0]);
    let f = &fr[0];
    assert_eq!(f.batches.len(), 1);
    let b = &f.batches[0];
    assert_eq!(b.blend, Blend::ADDITIVE);
    assert!(b.depth_test && !b.depth_write);
    assert_eq!(b.depth_offset, 0.0);
    let vs = f.batch_vertices(b);
    // Centred at (10, 0, 0), 2 x 16 units each way, facing the camera.
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for v in vs {
        lo = lo.min(Vec3::from(v.pos));
        hi = hi.max(Vec3::from(v.pos));
        assert_eq!(v.color, [1.0, 1.0, 1.0, 1.0]);
    }
    assert!((lo - Vec3::new(-22.0, -32.0, 0.0)).length() < 1e-3, "{lo}");
    assert!((hi - Vec3::new(42.0, 32.0, 0.0)).length() < 1e-3, "{hi}");
    // The texture's top-left corner sits at the top of the screen (y down).
    let top_left = vs.iter().find(|v| v.uv == [0.0, 0.0]).unwrap();
    assert!(top_left.pos[1] < 0.0 && top_left.pos[0] < 10.0);
    // Tick 1's colour key halves the alpha.
    assert_eq!(fr[2].vertices[0].color[3], 0.5);
    // The effect ends at its end frame (4 ticks).
    assert_eq!(fx.running_count(), 0);
}

#[test]
fn billboard_faces_any_camera() {
    let (mut fx, _) = setup(0, 0.0, 0);
    let cam = Camera::look_at(Vec3::new(200.0, -50.0, 30.0), Vec3::ZERO, Vec3::NEG_Y);
    let mut bones = |_: u16| Some(Mat4::IDENTITY);
    fx.start(7, &mut bones);
    let fr = frames(&mut fx, &cam, 1);
    let view = cam.view();
    let zs: Vec<f32> = fr[0].vertices.iter().map(|v| view.transform_point3(Vec3::from(v.pos)).z).collect();
    for z in &zs {
        assert!((z - zs[0]).abs() < 1e-3);
    }
}

#[test]
fn start_wait_delays_drawing() {
    let (mut fx, cam) = setup(0, 4.0, 0);
    let mut bones = |_: u16| Some(Mat4::IDENTITY);
    fx.start(7, &mut bones);
    let tris: Vec<usize> = frames(&mut fx, &cam, 5).iter().map(|f| f.triangle_count()).collect();
    assert_eq!(tris, vec![0, 0, 0, 2, 2]);
}

#[test]
fn bound_effect_follows_bone_and_fades_on_end() {
    // Bind (0x01) and fade-out end (0x40) over 4 frames.
    let (mut fx, cam) = setup(0x41, 0.0, 4);
    let mut x = 0.0f32;
    let h = {
        let mut bones = |_: u16| Some(Mat4::from_translation(Vec3::new(x, 0.0, 0.0)));
        fx.start(7, &mut bones).unwrap()
    };
    let mut f = Frame::new();
    for i in 0..2 {
        x = 100.0 * i as f32;
        let mut bones = |_: u16| Some(Mat4::from_translation(Vec3::new(x, 0.0, 0.0)));
        fx.tick(1.0, &mut bones);
        fx.build(&cam, &mut f);
    }
    // Moved with the bone: centre at 100 + 10.
    let cx: f32 = f.vertices.iter().map(|v| v.pos[0]).sum::<f32>() / f.vertices.len() as f32;
    assert!((cx - 110.0).abs() < 1e-3, "{cx}");
    fx.end(h);
    assert!(fx.is_running(h));
    let mut bones = |_: u16| Some(Mat4::IDENTITY);
    for _ in 0..4 {
        fx.tick(1.0, &mut bones);
    }
    assert!(!fx.is_running(h));
}

#[test]
fn world_matrix_stays_where_it_was_spawned() {
    // RyjDrawMatrixWorld saves the root at spawn in its w1 (64 bytes).
    let lib = Library::load(&pax(7, 0x01, 0.0, 0, sprite_effect_with(3, 4, (120, vec![96, 96]))), None).unwrap();
    assert!(lib.unsupported_programs(7).is_empty());
    let mut fx = Effects::new(Arc::new(lib));
    let cam = Camera::look_at(Vec3::new(10.0, 0.0, -100.0), Vec3::new(10.0, 0.0, 0.0), Vec3::NEG_Y);
    let mut x = 0.0f32;
    {
        let mut bones = |_: u16| Some(Mat4::from_translation(Vec3::new(x, 0.0, 0.0)));
        fx.start(7, &mut bones).unwrap();
    }
    let mut f = Frame::new();
    for i in 0..4 {
        x = 100.0 * i as f32;
        let mut bones = |_: u16| Some(Mat4::from_translation(Vec3::new(x, 0.0, 0.0)));
        fx.tick(1.0, &mut bones);
        fx.build(&cam, &mut f);
    }
    // The bound root moved to 300, the sprite stayed at 0 + 10.
    let cx: f32 = f.vertices.iter().map(|v| v.pos[0]).sum::<f32>() / f.vertices.len() as f32;
    assert!((cx - 10.0).abs() < 1e-3, "{cx}");
}

#[test]
fn wood_billboard_stays_upright() {
    let lib = Library::load(&pax(7, 0, 0.0, 0, sprite_effect_with(3, 4, (115, vec![96]))), None).unwrap();
    let mut fx = Effects::new(Arc::new(lib));
    // Looking down at the sprite from above and in front.
    let cam = Camera::look_at(Vec3::new(10.0, -100.0, -100.0), Vec3::new(10.0, 0.0, 0.0), Vec3::NEG_Y);
    let mut bones = |_: u16| Some(Mat4::IDENTITY);
    fx.start(7, &mut bones);
    let fr = frames(&mut fx, &cam, 1);
    let vs = &fr[0].vertices;
    assert_eq!(vs.len(), 6);
    // Turned toward the camera about the root's y only: still in the z = 0
    // plane, 2 x 16 units each way around (10, 0, 0).
    for v in vs {
        assert!(v.pos[2].abs() < 1e-3, "{:?}", v.pos);
        assert!((v.pos[0] - 10.0).abs() > 31.9 && (v.pos[1]).abs() > 31.9, "{:?}", v.pos);
    }
}

/// A shot moving 10 units along +x per tick (Move, Point, Scale, Color,
/// MatrixScl) with a trail program drawing shape 0 behind it.
fn trail_effect(trail: (u32, u16, Vec<u8>)) -> Vec<u8> {
    let (kind, size, key) = trail;
    let progs: Vec<Prog> = vec![
        (36, 16, vec![(0, f3(10.0, 0.0, 0.0))], vec![0, 32]),
        (40, 16, vec![(0, f3(0.0, 0.0, 0.0))], vec![0]),
        (42, 16, vec![(0, f3(1.0, 1.0, 1.0))], vec![16]),
        (43, 12, vec![(0, [0x00, 0x40, 0x00, 0x40, 0x00, 0x40, 0x00, 0x40].to_vec())], vec![48]),
        (94, 4, vec![(0, vec![])], vec![0, 16]),
        (kind, size, vec![(0, key)], vec![256, 48, 0, 0]),
    ];
    dpd(particle_data(20, 20, &progs))
}

/// Key bytes (without the frame word) with fields at their key offsets.
fn key_bytes(len: usize, fields: &[(usize, &[u8])]) -> Vec<u8> {
    let mut k = vec![0u8; len - 4];
    for (at, b) in fields {
        k[at - 4..at - 4 + b.len()].copy_from_slice(b);
    }
    k
}

/// Centres of the drawn quads, in draw order.
fn quad_centres(f: &Frame) -> Vec<Vec3> {
    f.vertices.chunks(6).map(|q| q.iter().map(|v| Vec3::from(v.pos)).sum::<Vec3>() / 6.0).collect()
}

fn run_trail(trail: (u32, u16, Vec<u8>), frames: usize) -> Frame {
    let lib = Library::load(&pax(7, 0, 0.0, 0, trail_effect(trail)), None).unwrap();
    assert!(lib.unsupported_programs(7).is_empty());
    let mut fx = Effects::new(Arc::new(lib));
    let cam = Camera::look_at(Vec3::new(20.0, 0.0, -200.0), Vec3::new(20.0, 0.0, 0.0), Vec3::NEG_Y);
    let mut bones = |_: u16| Some(Mat4::IDENTITY);
    fx.start(7, &mut bones).unwrap();
    let mut f = Frame::new();
    for _ in 0..frames {
        fx.tick(1.0, &mut bones);
        fx.build(&cam, &mut f);
    }
    f
}

#[test]
fn shape_trail_3x_follows_the_shot() {
    // Shape 0, size 1 → 1, spacing 10, 4 sprites from the head, immediate,
    // head and tail colours 0x80 (0x4000 in 8.8) in every channel.
    let colour: Vec<u8> = [0x4000i16; 4].iter().flat_map(|c| c.to_le_bytes()).collect();
    let key = key_bytes(
        0x58,
        &[(0xc, &1f32.to_le_bytes()), (0x10, &1f32.to_le_bytes()), (0x18, &10f32.to_le_bytes()), (0x1c, &[4, 1]), (0x20, &colour), (0x28, &colour), (0x54, &[1])],
    );
    // Five ticks: the shot has been at x = 10, 20, 30, 40, 50.
    let f = run_trail((157, 0x58, key), 10);
    let c = quad_centres(&f);
    assert_eq!(c.len(), 4, "{c:?}");
    for (q, x) in c.iter().zip([50.0, 40.0, 30.0, 20.0]) {
        assert!((*q - Vec3::new(x, 0.0, 0.0)).length() < 1e-3, "{c:?}");
    }
    // 32-unit sprites (shape 0 at size 1), full colour.
    let xs: Vec<f32> = f.vertices[..6].iter().map(|v| v.pos[0]).collect();
    assert!((xs.iter().cloned().fold(f32::MIN, f32::max) - xs.iter().cloned().fold(f32::MAX, f32::min) - 32.0).abs() < 1e-3);
    assert_eq!(f.vertices[0].color, [1.0; 4]);
}

#[test]
fn shape_trail_2x_shrinks_and_fades() {
    // Size 2 → 1 over 3 sprites, spacing 10 (shrinking with the size: 7.5,
    // 5), head colour 0x80 fading to alpha 0 at the tail.
    let key = key_bytes(
        0x2c,
        &[
            (0xc, &2f32.to_le_bytes()),
            (0x10, &1f32.to_le_bytes()),
            (0x14, &[0x80, 0x80, 0x80, 0x80]),
            (0x18, &[0x80, 0x80, 0x80, 0]),
            (0x1c, &10f32.to_le_bytes()),
            (0x20, &3u16.to_le_bytes()),
            (0x22, &[1]),
            (0x27, &[1]),
        ],
    );
    let f = run_trail((155, 0x2c, key), 10);
    let c = quad_centres(&f);
    assert_eq!(c.len(), 3, "{c:?}");
    // Head at 50; the second sprite at the first spacing (10) back, the
    // third the second spacing (7.5) further.
    for (q, x) in c.iter().zip([50.0, 40.0, 32.5]) {
        assert!((q.x - x).abs() < 1e-3, "{c:?}");
    }
    let alpha: Vec<f32> = f.vertices.chunks(6).map(|q| q[0].color[3]).collect();
    assert_eq!(alpha, vec![1.0, 0.5, 0.0]);
    let width = |q: &[kh2_fx::Vertex]| q.iter().map(|v| v.pos[0]).fold(f32::MIN, f32::max) - q.iter().map(|v| v.pos[0]).fold(f32::MAX, f32::min);
    let w: Vec<f32> = f.vertices.chunks(6).map(width).collect();
    assert!((w[0] - 64.0).abs() < 1e-3 && (w[1] - 48.0).abs() < 1e-3 && (w[2] - 32.0).abs() < 1e-3, "{w:?}");
}

/// The spell packages the mod plays use no program this crate skips.
/// Needs the user's own files in `local/re/` (or the folder in
/// `KH2_FX_DIR`); skipped when they are missing.
#[test]
fn spell_packages_fully_supported() {
    let dir = std::env::var("KH2_FX_DIR").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/re"));
    let mut files: Vec<(String, Option<&str>)> = Vec::new();
    for s in ["FIRE", "BLIZZARD", "THUNDER", "CURE", "REFLECT"] {
        for n in 1..=3 {
            files.push((format!("{s}_{n}.mag"), None));
        }
    }
    files.push(("P_EX100_MAGF.a.us".into(), Some("p_ex")));
    files.push(("P_EX100.a.us".into(), Some("p_ex")));
    let common = std::fs::read(dir.join("fx/texcommon.dpd")).ok();
    let mut checked = 0;
    for (f, name) in &files {
        let Ok(bytes) = std::fs::read(dir.join(f)) else { continue };
        let lib = Library::load_named(&bytes, *name, common.as_deref()).unwrap_or_else(|e| panic!("{f}: {e}"));
        for el in lib.elements() {
            let u = lib.unsupported_programs(el.id);
            assert!(u.is_empty(), "{f} element {}: {u:?}", el.id);
        }
        checked += 1;
    }
    if checked == 0 {
        eprintln!("spell_packages_fully_supported: no files in {}, skipped", dir.display());
    }
}
