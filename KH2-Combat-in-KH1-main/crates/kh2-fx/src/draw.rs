//! Turning a player's state into triangles for one camera
//! (`docs/kh2-effects.md` §7.3, §7.4, §8.5, §10, §11).
//!
//! Draw programs are visited in the game's order: emitter by emitter, the
//! draw programs of each in list order, the instances of each in group
//! order. Matrices are built in view space as the game builds them (so
//! billboards face the camera) and the result is taken back to world
//! space with the inverse view.

use glam::{Mat4, Vec3};
use kh2_formats::dpd::{Key, Model, Program, ShapeFrame};

use crate::camera::Camera;
use crate::library::Library;
use crate::output::{Blend, DrawState, Frame, Vertex};
use crate::math;
use crate::player::{k, mime_vertices, DKind, Instance, Player, ProgState};

/// Colour bytes (0x80 = 1.0) times a vertex colour, in 0x80 units,
/// clamped like the PC's vertex path.
fn modulate(v: [u8; 4], c: [u8; 4]) -> [f32; 4] {
    let f = |a: u8, b: u8| (a as f32 * b as f32 / 128.0).min(255.0) / 128.0;
    [f(v[0], c[0]), f(v[1], c[1]), f(v[2], c[2]), f(v[3], c[3])]
}

fn w(p: &Program, i: usize) -> i32 {
    p.work.get(i).copied().unwrap_or(0)
}

struct Ctx<'a> {
    lib: &'a Library,
    dpd: usize,
    view: Mat4,
    inv: Mat4,
    root: Mat4,
    vr: Mat4,
    rs: Vec3,
    /// The upright camera-facing frame of DrawMatrixWood, in view space.
    wood: Mat4,
}

/// The game's upright billboard frame (`docs/kh2-effects.md` §7.3): y is
/// the root's y axis in view space, normalised; x is perpendicular to it
/// in the screen plane, `(y.y, -y.x, 0)` normalised; z = x × y; origin is
/// the root's view-space origin.
pub(crate) fn wood_frame(vr: &Mat4) -> Mat4 {
    let y = vr.y_axis.truncate().try_normalize().unwrap_or(Vec3::Y);
    let x = Vec3::new(y.y, -y.x, 0.0).try_normalize().unwrap_or(Vec3::X);
    let z = x.cross(y).try_normalize().unwrap_or(Vec3::Z);
    Mat4::from_cols(x.extend(0.0), y.extend(0.0), z.extend(0.0), vr.w_axis)
}

impl Ctx<'_> {
    /// The instance's draw matrix `D`, in view space.
    fn d(&self, inst: &Instance) -> Option<Mat4> {
        match inst.d {
            DKind::None => None,
            DKind::Full => Some(self.vr * inst.m),
            DKind::Front => Some(self.front(inst.m)),
            DKind::Wood => {
                let m = inst.m;
                Some(self.wood * Mat4::from_cols(m.x_axis * self.rs.x, m.y_axis * self.rs.y, m.z_axis * self.rs.z, m.w_axis))
            }
            DKind::World(root) => Some(self.view * root * inst.m),
        }
    }

    fn front(&self, m: Mat4) -> Mat4 {
        Mat4::from_cols(m.x_axis * self.rs.x, m.y_axis * self.rs.y, m.z_axis * self.rs.z, self.vr.transform_point3(m.w_axis.truncate()).extend(1.0))
    }

    /// The Z-corrected matrix a KeZCrct(Shp) program writes at `offset`
    /// for this instance, in view space.
    fn z(&self, def: &[Program], inst: &Instance, offset: i32) -> Option<Mat4> {
        let (pi, prog) = def.iter().enumerate().find(|(_, p)| matches!(p.kind, k::KE_Z_CRCT | k::KE_Z_CRCT_SHP) && w(p, 0) == offset)?;
        let key = inst.key(prog, pi)?;
        let off = Vec3::from(key.vec3(4));
        let ks = Vec3::from(key.vec3(16));
        let mode = key.u8(28);
        let m = inst.m;
        if prog.kind == k::KE_Z_CRCT_SHP {
            let t = m.w_axis.truncate() * ks;
            let t = match mode {
                0 => self.vr.transform_point3(t + off),
                1 => self.vr.transform_point3(t) + off,
                2 => self.view.transform_point3(self.root.transform_point3(t) + self.rs * off),
                _ => self.vr.transform_point3(t),
            };
            let mut z = self.front(m);
            z.w_axis = t.extend(1.0);
            Some(z)
        } else {
            let mut l = Mat4::from_cols(m.x_axis * ks.x, m.y_axis * ks.y, m.z_axis * ks.z, m.w_axis);
            Some(match mode {
                0 => {
                    l.w_axis += off.extend(0.0);
                    self.vr * l
                }
                2 => {
                    let mut r = self.root * l;
                    r.w_axis += (self.rs * off).extend(0.0);
                    self.view * r
                }
                _ => {
                    let mut r = self.vr * l;
                    r.w_axis += off.extend(0.0);
                    r
                }
            })
        }
    }

    fn world(&self, view_pos: Vec3) -> [f32; 3] {
        self.inv.transform_point3(view_pos).to_array()
    }
}

/// Depth shift of a Z matrix against `D` along view z.
fn z_offset(d: &Mat4, z: Option<Mat4>) -> f32 {
    z.map(|z| z.w_axis.z - d.w_axis.z).unwrap_or(0.0)
}

/// Depth handling of a sprite draw.
#[derive(Clone, Copy)]
struct SpriteDepth {
    test: bool,
    offset: f32,
    sorted: bool,
}

/// Draw the sprites of one shape frame with a sprite-plane → view matrix.
fn shape_frame(cx: &Ctx, out: &mut Frame, fr: &ShapeFrame, m: &Mat4, colour: [u8; 4], depth: SpriteDepth) {
    let SpriteDepth { test, offset: depth_offset, sorted } = depth;
    let sort = sorted.then_some(m.w_axis.z);
    for sp in &fr.sprites {
        let st = DrawState {
            texture: cx.lib.texture_for(cx.dpd, fr.tex0, sp.clut_offset),
            blend: Blend { alpha: sp.blend, fix: sp.blend_fix },
            depth_test: test,
            depth_write: false,
            depth_offset,
            sorted: sort,
        };
        out.begin(st);
        let col = modulate(sp.rgba, colour);
        let p = sp.positions();
        let [u0, v0, u1, v1] = sp.uv();
        let corner = |i: usize, uv: [f32; 2]| Vertex { pos: cx.world(m.transform_point3(Vec3::new(p[i][0], p[i][1], 0.0))), uv, color: col };
        // Corners are TL, TR, BL, BR; drawn as TL, TR, BR, BL.
        out.quad([corner(0, [u0, v0]), corner(1, [u1, v0]), corner(3, [u1, v1]), corner(2, [u0, v1])]);
    }
}

/// Model draw settings from a DrawMdl key.
struct MdlDraw {
    blend: Option<u16>,
    bits: u8,
    scroll: (f32, f32),
    z: Option<Mat4>,
}

/// A DrawMdl program's draw settings from its key and work, and the work
/// offset of its Z matrix when the key asks for one.
fn mdl_params(prog: &Program, key: &Key, wk: &crate::player::Work) -> (MdlDraw, Option<i32>) {
    let s = w(prog, 2);
    match prog.kind {
        k::DRAW_MDL3 => (MdlDraw { blend: None, bits: key.u8(8), scroll: (0.0, 0.0), z: None }, (key.u8(12) != 0).then(|| w(prog, 2))),
        // DrawMdl3 with the blend from the key (@8, the GS ALPHA byte and
        // FIX, always used) and its other bytes two later.
        k::DRAW_MDL_SEMI3 => (MdlDraw { blend: Some(key.u16(8)), bits: key.u8(10), scroll: (0.0, 0.0), z: None }, (key.u8(14) != 0).then(|| w(prog, 2))),
        k::DRAW_MDL_TS3 => (MdlDraw { blend: None, bits: key.u8(32), scroll: (wk.f32(s), wk.f32(s + 12)), z: None }, (key.u8(36) != 0).then(|| w(prog, 3))),
        // DrawMdlLoop: LoopZ's draw without the Z matrix.
        k::DRAW_MDL_LOOP => (MdlDraw { blend: (key.i8(32) != -1).then(|| key.u16(32)), bits: key.u8(34), scroll: (wk.f32(s + 4), wk.f32(s + 16)), z: None }, None),
        _ => (
            MdlDraw { blend: (key.i8(32) != -1).then(|| key.u16(32)), bits: key.u8(34), scroll: (wk.f32(s + 4), wk.f32(s + 16)), z: None },
            (key.u8(38) != 0).then(|| w(prog, 3)),
        ),
    }
}

fn model(cx: &Ctx, out: &mut Frame, mdl: &Model, verts: &[[i16; 3]], d: &Mat4, colour: [u8; 4], o: MdlDraw) {
    let immediate = o.bits & 2 != 0;
    let per_tri = !immediate && o.bits & 1 != 0;
    let blend = match o.blend {
        Some(b) => Blend { alpha: b as u8, fix: (b >> 8) as u8 },
        None => Blend { alpha: mdl.blend, fix: mdl.blend_fix },
    };
    // OMD bit 0 clear: depth write; bit 1 clear: no depth test (flag 0x10).
    let test = mdl.flags & 2 != 0;
    let write = mdl.flags & 1 == 0 && !per_tri && test;
    let depth_offset = z_offset(d, o.z);
    let sort = (!immediate).then_some(d.w_axis.z);
    for g in &mdl.groups {
        let tex = if g.textured() { cx.lib.texture_for(cx.dpd, g.tex0, 0) } else { None };
        out.begin(DrawState { texture: tex, blend, depth_test: test, depth_write: write, depth_offset, sorted: sort });
        // Scroll in texels, as the game stores it: (u * 65536) >> (log2 size + 4), in 1/4096.
        let du = (((o.scroll.0 * 65536.0) as i32) >> (g.tex0.tw() as i32 + 4)) as f32 / 4096.0;
        let dv = (((o.scroll.1 * 65536.0) as i32) >> (g.tex0.th() as i32 + 4)) as f32 / 4096.0;
        for p in &g.prims {
            let vert = |i: usize| {
                let v = verts.get(p.vertices[i] as usize).or_else(|| mdl.vertices.get(p.vertices[i] as usize)).copied().unwrap_or([0; 3]);
                let uv = p.st.get(i).map(|s| [s[0] as f32 / 4096.0 + du, s[1] as f32 / 4096.0 + dv]).unwrap_or([0.0; 2]);
                Vertex {
                    pos: cx.world(d.transform_point3(Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32))),
                    uv,
                    color: modulate(p.rgba[i], colour),
                }
            };
            if p.vertices.len() >= 4 {
                out.tri([vert(0), vert(1), vert(2)]);
                out.tri([vert(1), vert(3), vert(2)]);
            } else if p.vertices.len() == 3 {
                out.tri([vert(0), vert(1), vert(2)]);
            }
        }
    }
}

/// Emit every draw of one player.
pub(crate) fn build(lib: &Library, p: &Player, cam: &Camera, out: &mut Frame) {
    let view = cam.view();
    let vr = view * p.fx.root;
    let cx = Ctx { lib, dpd: p.dpd, view, inv: cam.view_inverse(), root: p.fx.root, vr, rs: p.fx.rs, wood: wood_frame(&vr) };
    let data = p.data(lib);
    let dpd = &lib.dpds[p.dpd];
    for (e, def) in data.emitters.iter().enumerate() {
        for (pi, prog) in def.programs.iter().enumerate() {
            for inst in p.group(e) {
                if inst.spawned {
                    continue;
                }
                let Some(key) = inst.key(prog, pi) else { continue };
                match prog.kind {
                    k::DRAW_SHAPE | k::DRAW_SHAPE_X => {
                        let (Some(d), Some(sh)) = (cx.d(inst), dpd.shape(key.i32(4))) else { continue };
                        let Some(step) = sh.sequence.get(inst.work.u16(w(prog, 0) + 4) as usize) else { continue };
                        let colour = inst.work.rgba(w(prog, 1));
                        let (sorted, test, z) = if prog.kind == k::DRAW_SHAPE_X {
                            (key.u8(15) == 0, key.u8(16) != 1, if key.u8(14) != 0 { cx.z(&def.programs, inst, w(prog, 3)) } else { None })
                        } else {
                            (key.u8(12) == 0, key.u8(13) != 1, None)
                        };
                        shape_frame(&cx, out, &step.frame, &d, colour, SpriteDepth { test, offset: z_offset(&d, z), sorted });
                    }
                    k::DRAW_MDL3 | k::DRAW_MDL_SEMI3 | k::DRAW_MDL_TS3 | k::DRAW_MDL_LOOP | k::DRAW_MDL_LOOP_Z => {
                        let (Some(d), Some(mdl)) = (cx.d(inst), dpd.model(key.i32(4))) else { continue };
                        let wk = &inst.work;
                        let colour = wk.rgba(w(prog, 0));
                        if colour[3] == 0 || key.i32(4) == 0xffff {
                            continue;
                        }
                        let (o, zat) = mdl_params(prog, key, wk);
                        let z = zat.and_then(|at| cx.z(&def.programs, inst, at));
                        let morphed = mime_vertices(dpd, &def.programs, inst);
                        model(&cx, out, mdl, morphed.as_deref().unwrap_or(&mdl.vertices), &d, colour, MdlDraw { z, ..o });
                    }
                    k::KE_SHP_TAIL2X | k::KE_SHP_TAIL3X => tail(&cx, out, &def.programs, inst, pi, prog, key),
                    k::MEGA_BIRTH => megabirth(&cx, out, inst, pi, prog, key),
                    k::SHIPOLY => shipoly(&cx, out, inst, pi, prog, key),
                    _ => {}
                }
            }
        }
    }
}

/// pppKeShpTail2X / 3X: one camera-facing sprite per trail point
/// (`crate::tail`). Each sprite's axes are the instance's `M` axes times
/// the root scale lengths and the sprite size, used as they are in view
/// space (like DrawMatrixFront); its position is the trail point through
/// `V·Root`, or `V` for points recorded in world space. 3X can also turn
/// each sprite by its random value and shows it at its own animation
/// phase; 2X draws the one step it animates.
fn tail(cx: &Ctx, out: &mut Frame, progs: &[Program], inst: &Instance, pi: usize, prog: &Program, key: &Key) {
    if key.i32(4) == 0xffff {
        return;
    }
    let Some(sh) = cx.lib.dpds[cx.dpd].shape(key.i32(4)) else { return };
    let o = w(prog, 0);
    let colour = inst.work.rgba(w(prog, 1));
    let tail3 = prog.kind == k::KE_SHP_TAIL3X;
    // Key bytes: Z matrix, world space, immediate draw, no depth test.
    let (z_at, world, sort, no_test) = if tail3 { (0x52, 0x53, 0x54, 0x55) } else { (0x25, 0x26, 0x27, 0x28) };
    let space = if key.u8(world) != 0 { cx.view } else { cx.vr };
    // The instance's Z matrix serves every sprite: the depth shift it
    // gives the instance itself.
    let z = (key.u8(z_at) != 0).then(|| cx.z(progs, inst, w(prog, 3))).flatten();
    let d = cx.d(inst).unwrap_or_else(|| Mat4::from_translation(cx.vr.transform_point3(inst.m.w_axis.truncate())));
    let depth = SpriteDepth { test: key.u8(no_test) != 1, offset: z_offset(&d, z), sorted: key.u8(sort) == 0 };
    let m = inst.m;
    let (ax, ay, az) = (m.x_axis * cx.rs.x, m.y_axis * cx.rs.y, m.z_axis * cx.rs.z);
    let at = |p: Vec3| space.transform_point3(p).extend(1.0);
    if tail3 {
        let time = match inst.state[pi] {
            ProgState::TailTime(t) => t,
            _ => inst.work.i32(o + crate::tail::TAIL3_TIME) as u32,
        };
        let spin = key.i16(0x1e) != 0;
        for s in crate::tail::tail3_sprites(&inst.work, o, key, colour) {
            let (mut x, mut y) = (ax * s.sprite.size, ay * s.sprite.size);
            if spin {
                let (sn, cs) = (math::sin_i(s.seed as i32), math::cos_i(s.seed as i32));
                (x, y) = (x * cs + y * sn, y * cs - x * sn);
            }
            let Some(step) = sh.sequence.get(crate::tail::tail3_step(sh, time, s.seed)) else { continue };
            let mm = Mat4::from_cols(x, y, glam::Vec4::ZERO, at(s.sprite.pos));
            shape_frame(cx, out, &step.frame, &mm, s.sprite.colour, depth);
        }
    } else {
        let Some(step) = sh.sequence.get(inst.work.u16(o + 6) as usize) else { return };
        for s in crate::tail::tail2_sprites(&inst.work, o, key, colour) {
            let mm = Mat4::from_cols(ax * s.size, ay * s.size, az * s.size, at(s.pos));
            shape_frame(cx, out, &step.frame, &mm, s.colour, depth);
        }
    }
}

fn megabirth(cx: &Ctx, out: &mut Frame, inst: &Instance, pi: usize, prog: &Program, key: &Key) {
    let ProgState::MegaBirth(mb) = &inst.state[pi] else { return };
    if key.i32(4) == 0xffff {
        return;
    }
    let Some(sh) = cx.lib.dpds[cx.dpd].shape(key.i32(4)) else { return };
    let colour = inst.work.rgba(w(prog, 1));
    let sorted = key.u8(15) == 0;
    let test = key.u8(0xcd) != 1;
    let zoff = if key.u8(14) != 0 { key.f32(0x18) } else { 0.0 };
    for i in 0..mb.slots.len() {
        if mb.slots[i].life == 0 {
            continue;
        }
        let Some(m) = mb.view_matrix(i, key, cx.view, inst.m, cx.rs) else { continue };
        let Some(step) = sh.sequence.get(mb.slots[i].anim_drawn as usize) else { continue };
        shape_frame(cx, out, &step.frame, &m, mb.colour(i, colour), SpriteDepth { test, offset: zoff, sorted });
    }
}

fn shipoly(cx: &Ctx, out: &mut Frame, inst: &Instance, pi: usize, prog: &Program, key: &Key) {
    let ProgState::Shipoly(sp) = &inst.state[pi] else { return };
    let dpd = &cx.lib.dpds[cx.dpd];
    let tex = dpd.model(key.i32(4)).and_then(|m| m.groups.first()).and_then(|g| cx.lib.texture_for(cx.dpd, g.tex0, 0));
    let space = if key.u8(0x11) == 0 { cx.vr } else { cx.view };
    let half = key.f32(12) * inst.work.f32(w(prog, 1)) * cx.rs.x;
    let Some((a, b)) = sp.edges(key, space, half) else { return };
    let sub = key.u8(0x12) as usize;
    let cols = key.u8(0x13) as usize + 1;
    if cols < 2 || sub == 0 {
        return;
    }
    let colour = inst.work.rgba(w(prog, 2));
    let blend = match key.u8(0x15) {
        0 => 0x44,
        1 => 0x48,
        2 => 0x42,
        3 => 0x41,
        _ => 0x46,
    };
    out.begin(DrawState {
        texture: tex,
        blend: Blend { alpha: blend, fix: 0x80 },
        depth_test: key.u8(0x14) == 0,
        depth_write: false,
        depth_offset: 0.0,
        sorted: None,
    });
    // Texture region and direction along the ribbon.
    let mut u_end: f32 = match key.u8(0x9c) {
        1 => 0.5,
        2 => 0.25,
        _ => 1.0,
    };
    let v_max: f32 = match key.u8(0x9d) {
        1 => 0.5,
        2 => 0.25,
        _ => 1.0,
    } - 0.01;
    let mut u_start = 0.0;
    match key.u8(0x9e) {
        1 => {
            u_end += 0.25;
            u_start = 0.25;
        }
        2 => {
            u_end += 0.5;
            u_start = 0.5;
        }
        3 => {
            u_end += 0.75;
            u_start = 0.75;
        }
        _ => {}
    }
    u_start += 0.01;
    u_end -= 0.01;
    let span = u_end - u_start;
    let span = if key.u8(0xa1) != 0 { span * sp.fill_ratio() } else { span };
    let (mut u, step) = match key.u8(0xa0) {
        0 | 2 => (u_start, span / sub as f32),
        1 | 3 => (u_end, -span / sub as f32),
        _ => (0.0, 0.0),
    };
    // Alpha along the ribbon (key 0x17): up over the first part, down after.
    let alpha = colour[3] as f32;
    let mut f = 1.0f32;
    let (mut up, mut down, mut falling) = (0.0f32, 0.0f32, false);
    if key.u8(0x17) != 0 {
        let fade = (sub as f32 * key.f32(0x18)) as i32 as f32;
        let rest = sub as f32 - fade;
        if fade > 0.0 {
            f = 0.0;
            up = 1.0 / fade;
        }
        if rest > 0.0 {
            down = 1.0 / rest;
        }
    }
    // The PC scales ribbon colours by 1/225 where other paths use 1/255.
    let scale = 255.0 / 225.0 / 128.0;
    let rgb = [colour[0] as f32 * scale, colour[1] as f32 * scale, colour[2] as f32 * scale];
    for i in 0..sub {
        let (mut a0, mut a1) = (alpha, alpha);
        if key.u8(0x17) != 0 {
            a0 = alpha * f;
            let nf = if !falling && f < 1.0 {
                f + up
            } else {
                falling = true;
                f - down
            };
            f = nf.clamp(0.0, 1.0);
            a1 = alpha * f;
        }
        // u at the far end of this segment, and where the next one starts.
        let (raw, next) = if key.f32(0x90) > 0.0 {
            let n = u + a[i].w / key.f32(0x90) * span;
            (n, if n > u_end { n - u_end + u_start } else { n })
        } else {
            (u + step, (u + step).min(u_end))
        };
        let (uc, un) = (u + sp.scroll_u, raw + sp.scroll_u);
        let mut across = 0.0f32;
        let mut delta = 2.0 / (cols - 1) as f32;
        let mut row = Vec::with_capacity(cols);
        for c in 0..cols {
            let t = c as f32 / (cols - 1) as f32;
            let v = v_max * t + 0.01 + sp.scroll_v;
            let (mut c0, mut c1) = (a0, a1);
            if key.u8(0x16) != 0 {
                c0 *= across;
                c1 *= across;
                let nx = across + delta;
                if nx < 1.0 {
                    across = nx.max(0.0);
                } else {
                    delta = -delta;
                    across = 1.0;
                }
            }
            let p0 = b[i].lerp(a[i], t).truncate();
            let p1 = b[i + 1].lerp(a[i + 1], t).truncate();
            let (uv0, uv1) = if key.u8(0xa0) > 1 { ([v, uc], [v, un]) } else { ([uc, v], [un, v]) };
            let col = |al: f32| [rgb[0], rgb[1], rgb[2], (al as i32) as u8 as f32 * scale];
            row.push((
                Vertex { pos: cx.world(p0), uv: uv0, color: col(c0) },
                Vertex { pos: cx.world(p1), uv: uv1, color: col(c1) },
            ));
        }
        for c in 0..cols - 1 {
            out.quad([row[c].0, row[c].1, row[c + 1].1, row[c + 1].0]);
        }
        u = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colour_modulation() {
        assert_eq!(modulate([0x80; 4], [0x80; 4]), [1.0; 4]);
        // Clamped at 255 in 0x80 units (just under 2.0).
        let c = modulate([255, 0, 64, 128], [255, 128, 128, 64]);
        assert!((c[0] - 255.0 / 128.0).abs() < 1e-6);
        assert_eq!(c[1], 0.0);
        assert_eq!(c[2], 0.5);
        assert_eq!(c[3], 0.5);
    }

    #[test]
    fn model_programs_key_layouts() {
        let key = |bytes: &[(usize, u8)]| {
            let mut data = vec![0u8; 44];
            for &(at, b) in bytes {
                data[at] = b;
            }
            Key { data }
        };
        let prog = |kind: u32| Program { kind, key_size: 44, keys: vec![], work: vec![0, 0, 16, 64] };
        let mut wk = crate::player::Work::new(128);
        for (i, v) in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0].iter().enumerate() {
            wk.set_f32(16 + i as i32 * 4, *v);
        }
        // DrawMdlSemi3: blend @8 (ALPHA byte, FIX) always, bits @10, Z @14 at w2.
        let (o, z) = mdl_params(&prog(k::DRAW_MDL_SEMI3), &key(&[(8, 0x48), (9, 0x40), (10, 2), (14, 1)]), &wk);
        assert_eq!((o.blend, o.bits, o.scroll, z), (Some(0x4048), 2, (0.0, 0.0), Some(16)));
        let (o, z) = mdl_params(&prog(k::DRAW_MDL_SEMI3), &key(&[]), &wk);
        assert_eq!((o.blend, z), (Some(0), None));
        // DrawMdlLoop: LoopZ's blend (0xFF = the model's), bits and scroll,
        // never a Z matrix even with LoopZ's byte set.
        let (o, z) = mdl_params(&prog(k::DRAW_MDL_LOOP), &key(&[(32, 0xff), (34, 1), (38, 1)]), &wk);
        assert_eq!((o.blend, o.bits, o.scroll, z), (None, 1, (2.0, 5.0), None));
        let (_, z) = mdl_params(&prog(k::DRAW_MDL_LOOP_Z), &key(&[(32, 0xff), (38, 1)]), &wk);
        assert_eq!(z, Some(64));
    }

    #[test]
    fn wood_frame_keeps_root_up_and_faces_screen() {
        // Root y tilted in the screen plane and toward the camera, scaled 2.
        let y = Vec3::new(0.6, 0.0, 0.8) * 2.0;
        let vr = Mat4::from_cols(Vec3::new(0.0, 0.0, 2.0).extend(0.0), y.extend(0.0), Vec3::new(1.6, 0.0, -1.2).extend(0.0), Vec3::new(5.0, 6.0, 7.0).extend(1.0));
        let f = wood_frame(&vr);
        let (fx, fy, fz) = (f.x_axis.truncate(), f.y_axis.truncate(), f.z_axis.truncate());
        assert!((fy - y.normalize()).length() < 1e-6);
        // x stays in the screen plane, perpendicular to y; z = x × y.
        assert_eq!(fx.z, 0.0);
        assert!(fx.dot(fy).abs() < 1e-6 && (fx.length() - 1.0).abs() < 1e-6);
        assert!((fz - fx.cross(fy)).length() < 1e-6);
        assert_eq!(f.w_axis, vr.w_axis);
        // An upright root (y down the screen) gives the plain screen frame.
        let up = wood_frame(&Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)));
        assert!((up.x_axis.truncate() - Vec3::X).length() < 1e-6 && (up.z_axis.truncate() - Vec3::Z).length() < 1e-6);
    }
}
