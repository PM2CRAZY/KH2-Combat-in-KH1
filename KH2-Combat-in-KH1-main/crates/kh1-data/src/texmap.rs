//! Moving a texture from one model's UV layout to another's, by the surface: each texel of the
//! target texture is a point on the target model (its triangle there, in the bind pose), and takes
//! the colour the source model's texture has at the nearest point of the source model's surface.
//!
//! Used for Anti Form (`kh1-hook`'s `tint`): KH1's Anti-Sora (`xa_pp_3020.mdls`, the dark copy of
//! Sora from Neverland) is Sora's PS2 mesh on seven 512² textures in the PS2 layout, while the PC
//! draws Sora with his remastered mesh on four HD atlases of another layout. Same skeleton, so
//! both meshes posed in the bind pose lie on each other, and Anti-Sora's colours can be carried
//! onto Sora's HD textures (`docs/kh1-tint.md`).
//!
//! The remastered meshes (`-c0.cvbl`, a model's remastered asset; layout from the user's files,
//! see `docs/kh1-tint.md`): `count` at `+4`, a skip at `+8` (`0x1xxxx`: skip `xxxx`), then 16-byte
//! entries (`+4` texture slot, `+0xC` the packet list). A packet list has 16 bytes of header, then
//! chunks `{tag u32, size u32 (whole chunk)}`: tag `0x11` puts bone `+8` in matrix slot `+0xC`,
//! tag 1 holds `count` (`+8`) 48-byte vertices from `+0x20`, tag `0x8000` ends. A vertex: normal
//! (3 floats), flags (low 16 bits the matrix slot, high 16: `0x1000` starts a strip, 1 or 2 makes
//! a triangle with the two before), UV (2 floats), two more words, position in its bone's space
//! (4 floats).
//!
//! Nothing here holds game data; it works on the user's own files at run time.

use glam::{Mat4, Vec2, Vec3};

/// A B8G8R8A8 image.
#[derive(Clone, Debug)]
pub struct Image {
    pub w: u32,
    pub h: u32,
    pub bgra: Vec<u8>,
}

impl Image {
    /// An uncompressed 32-bit DDS's first level as B8G8R8A8 (whatever its masks).
    pub fn from_dds(dds: &[u8]) -> Option<Image> {
        if dds.len() < 128 || &dds[..4] != b"DDS " {
            return None;
        }
        let u = |o: usize| u32::from_le_bytes(dds[o..o + 4].try_into().unwrap());
        let (h, w) = (u(12), u(16));
        let (flags, bits) = (u(80), u(88));
        let masks = [u(92), u(96), u(100), u(104)];
        // RGB (0x40), 32 bits, no four-cc (0x4).
        if flags & 0x40 == 0 || flags & 0x4 != 0 || bits != 32 || w == 0 || h == 0 || w > 8192 || h > 8192 {
            return None;
        }
        let n = (w * h) as usize;
        let data = dds.get(128..128 + n * 4)?;
        let chan = |v: u32, m: u32| if m == 0 { 255 } else { ((v & m) >> m.trailing_zeros()) as u8 };
        let mut bgra = Vec::with_capacity(n * 4);
        for px in data.chunks_exact(4) {
            let v = u32::from_le_bytes(px.try_into().unwrap());
            bgra.extend_from_slice(&[chan(v, masks[2]), chan(v, masks[1]), chan(v, masks[0]), chan(v, masks[3])]);
        }
        Some(Image { w, h, bgra })
    }

    /// Bilinear, wrapping; B, G, R as 0..255 floats.
    fn sample(&self, uv: Vec2) -> [f32; 3] {
        let (w, h) = (self.w as i64, self.h as i64);
        let x = uv.x.rem_euclid(1.0) * self.w as f32 - 0.5;
        let y = uv.y.rem_euclid(1.0) * self.h as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let px = |xi: i64, yi: i64| {
            let i = ((yi.rem_euclid(h) * w + xi.rem_euclid(w)) * 4) as usize;
            [self.bgra[i] as f32, self.bgra[i + 1] as f32, self.bgra[i + 2] as f32]
        };
        let (x0, y0) = (x0 as i64, y0 as i64);
        let (a, b, c, d) = (px(x0, y0), px(x0 + 1, y0), px(x0, y0 + 1), px(x0 + 1, y0 + 1));
        let mut out = [0.0; 3];
        for k in 0..3 {
            let top = a[k] + (b[k] - a[k]) * fx;
            let bot = c[k] + (d[k] - c[k]) * fx;
            out[k] = top + (bot - top) * fy;
        }
        out
    }
}

/// A triangle of a posed model: world positions, UVs, its texture slot, its facing (unit normal
/// by the strip's winding).
#[derive(Clone, Copy, Debug)]
pub struct Tri {
    pub p: [Vec3; 3],
    pub uv: [Vec2; 3],
    pub slot: u32,
    pub n: Vec3,
}

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}
fn f32_at(b: &[u8], o: usize) -> Option<f32> {
    u32_at(b, o).map(f32::from_bits)
}
fn vec3_at(b: &[u8], o: usize) -> Option<Vec3> {
    Some(Vec3::new(f32_at(b, o)?, f32_at(b, o + 4)?, f32_at(b, o + 8)?))
}

/// A model's bind pose (one model-space matrix per bone) from its `.mdls`.
pub fn bind_pose(mdls: &[u8]) -> Result<Vec<Mat4>, String> {
    let skeleton = crate::mdls::parse_skeleton(mdls).map_err(|e| e.to_string())?;
    Ok(kh2_anim::Pose::bind(&skeleton).bones)
}

/// The triangles of a remastered mesh (`-c0.cvbl`), each vertex placed by its bone's matrix in
/// `bones` (the bind pose: `kh2_anim::Pose::bind` of the model's skeleton).
pub fn cvbl_triangles(cvbl: &[u8], bones: &[Mat4]) -> Result<Vec<Tri>, String> {
    let mut tris = Vec::new();
    cvbl_strips(cvbl, |_, slot, v, flags| {
        let place = |(bone, local, _): (usize, Vec3, Vec2)| -> Result<Vec3, String> {
            Ok(bones.get(bone).ok_or_else(|| format!("cvbl: bone {bone} not in the skeleton ({})", bones.len()))?.transform_point3(local))
        };
        let p = [place(v[0])?, place(v[1])?, place(v[2])?];
        let face = (p[1] - p[0]).cross(p[2] - p[0]);
        if face.length_squared() < 1e-12 {
            return Ok(());
        }
        // Facing by the strip's winding (flag 1 or 2): the meshes' own normals don't follow
        // it, but two models sharing geometry wind alike, so the two sides of a double-sided
        // part stay apart.
        let n = face.normalize() * if flags >> 16 & 3 == 1 { 1.0 } else { -1.0 };
        tris.push(Tri { p, uv: [v[0].2, v[1].2, v[2].2], slot, n });
        Ok(())
    })?;
    Ok(tris)
}

/// A remastered mesh's triangle on its texture: the mesh (its entry in the `-c0.cvbl` table), its
/// texture slot, its UVs.
#[derive(Clone, Copy, Debug)]
pub struct UvTri {
    pub mesh: u32,
    pub slot: u32,
    pub uv: [Vec2; 3],
}

/// The triangles of a remastered mesh (`-c0.cvbl`) on its textures only (no skeleton needed).
pub fn cvbl_uv_triangles(cvbl: &[u8]) -> Result<Vec<UvTri>, String> {
    let mut tris = Vec::new();
    cvbl_strips(cvbl, |mesh, slot, v, _| {
        tris.push(UvTri { mesh: mesh as u32, slot, uv: [v[0].2, v[1].2, v[2].2] });
        Ok(())
    })?;
    Ok(tris)
}

/// Texels of a `w`×`h` texture (row-major, `true`: in) covered by the UV islands of `tris` on
/// `slot` that hold one of `seeds` (UV points; an island: triangles of one mesh joined by shared
/// UV vertices), a texel past their edges included (for bilinear filtering). Returns the mask and
/// how many islands the seeds found.
pub fn island_mask(tris: &[UvTri], slot: u32, w: u32, h: u32, seeds: &[[f32; 2]]) -> (Vec<bool>, usize) {
    let on: Vec<&UvTri> = tris.iter().filter(|t| t.slot == slot).collect();
    let mut parent: Vec<usize> = (0..on.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    // (UVs within 1/4096 count as shared.)
    let mut first = std::collections::HashMap::new();
    for (i, t) in on.iter().enumerate() {
        for uv in t.uv {
            let j = *first.entry((t.mesh, (uv.x * 4096.0).round() as i32, (uv.y * 4096.0).round() as i32)).or_insert(i);
            let (a, b) = (root(&mut parent, i), root(&mut parent, j));
            parent[a] = b;
        }
    }
    // Signed distances (in `scale` units) of `p` inside each edge of `t`, or `None` if degenerate.
    let inside = |t: &UvTri, p: Vec2, scale: Vec2| -> Option<f32> {
        let q = t.uv.map(|uv| uv * scale);
        let area = (q[1] - q[0]).perp_dot(q[2] - q[0]);
        if area.abs() < 1e-9 {
            return None;
        }
        let p = p * scale;
        Some((0..3).map(|k| area.signum() * (q[(k + 1) % 3] - q[k]).perp_dot(p - q[k]) / (q[(k + 1) % 3] - q[k]).length().max(1e-6)).fold(f32::MAX, f32::min))
    };
    let mut picked: Vec<usize> = Vec::new();
    for s in seeds {
        for i in 0..on.len() {
            if inside(on[i], Vec2::new(s[0], s[1]), Vec2::ONE).is_some_and(|d| d >= 0.0) {
                let r = root(&mut parent, i);
                if !picked.contains(&r) {
                    picked.push(r);
                }
            }
        }
    }
    let (wu, hu) = (w as usize, h as usize);
    let mut mask = vec![false; wu * hu];
    let scale = Vec2::new(w as f32, h as f32);
    for i in 0..on.len() {
        if !picked.contains(&root(&mut parent, i)) {
            continue;
        }
        let t = on[i];
        let q = t.uv.map(|uv| uv * scale);
        let (lo, hi) = (q[0].min(q[1]).min(q[2]), q[0].max(q[1]).max(q[2]));
        let (x0, x1) = ((lo.x - 1.0).floor().max(0.0) as usize, ((hi.x + 1.0).ceil().max(0.0) as usize).min(wu));
        let (y0, y1) = ((lo.y - 1.0).floor().max(0.0) as usize, ((hi.y + 1.0).ceil().max(0.0) as usize).min(hu));
        for y in y0..y1 {
            for x in x0..x1 {
                let c = Vec2::new((x as f32 + 0.5) / scale.x, (y as f32 + 0.5) / scale.y);
                if inside(t, c, scale).is_some_and(|d| d >= -1.0) {
                    mask[y * wu + x] = true;
                }
            }
        }
    }
    (mask, picked.len())
}

/// Walks a remastered mesh's (`-c0.cvbl`) triangle strips: per triangle, its mesh, texture slot,
/// three vertices (bone, position in the bone's space, UV) and the last vertex's flags.
fn cvbl_strips(cvbl: &[u8], mut emit: impl FnMut(usize, u32, [(usize, Vec3, Vec2); 3], u32) -> Result<(), String>) -> Result<(), String> {
    let count = u32_at(cvbl, 4).ok_or("cvbl: too short")? as usize;
    let w = u32_at(cvbl, 8).ok_or("cvbl: too short")?;
    let skip = if w & 0x1_0000 != 0 { (w & 0xFFFF) as usize } else { 0 };
    if count == 0 || count > 64 || skip > 16 {
        return Err(format!("cvbl: {count} meshes, skip {skip}"));
    }
    for m in 0..count {
        let e = 0x10 * (1 + skip * 2 + m);
        let slot = u32_at(cvbl, e + 4).ok_or("cvbl: entry past the end")?;
        let list = u32_at(cvbl, e + 0xC).ok_or("cvbl: entry past the end")? as usize;
        let mut matrix = [0usize; 256];
        let mut o = list + 0x10;
        loop {
            let tag = u32_at(cvbl, o).ok_or_else(|| format!("cvbl: mesh {m} runs past the end"))?;
            let size = u32_at(cvbl, o + 4).ok_or("cvbl: chunk past the end")? as usize;
            match tag {
                0x8000 => break,
                0x11 => {
                    let bone = u32_at(cvbl, o + 8).ok_or("cvbl: chunk past the end")? as usize;
                    let s = u32_at(cvbl, o + 0xC).ok_or("cvbl: chunk past the end")? as usize;
                    if s < matrix.len() {
                        matrix[s] = bone;
                    }
                }
                1 => {
                    let n = u32_at(cvbl, o + 8).ok_or("cvbl: chunk past the end")? as usize;
                    if 0x20 + 48 * n > size {
                        return Err(format!("cvbl: mesh {m}: {n} vertices in a chunk of {size:#x}"));
                    }
                    let mut strip: Vec<(usize, Vec3, Vec2)> = Vec::with_capacity(3);
                    for k in 0..n {
                        let v = o + 0x20 + 48 * k;
                        let flags = u32_at(cvbl, v + 12).ok_or("cvbl: vertex past the end")?;
                        let bone = matrix.get((flags & 0xFFFF) as usize).copied().unwrap_or(0);
                        let local = vec3_at(cvbl, v + 32).ok_or("cvbl: vertex past the end")?;
                        let uv = Vec2::new(f32_at(cvbl, v + 16).unwrap_or(0.0), f32_at(cvbl, v + 20).unwrap_or(0.0));
                        if strip.len() == 3 {
                            strip.remove(0);
                        }
                        strip.push((bone, local, uv));
                        if flags >> 16 & 0x1000 == 0 && strip.len() == 3 {
                            emit(m, slot, [strip[0], strip[1], strip[2]], flags)?;
                        }
                    }
                }
                _ => {}
            }
            if size < 8 {
                return Err(format!("cvbl: mesh {m}: a chunk of size {size:#x}"));
            }
            o += size;
        }
    }
    Ok(())
}

/// The point of triangle `t` nearest `p`, as barycentric weights (Ericson, "Real-Time Collision
/// Detection", 5.1.5).
fn closest(p: Vec3, t: &[Vec3; 3]) -> (Vec3, [f32; 3]) {
    let (a, b, c) = (t[0], t[1], t[2]);
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return (a, [1.0, 0.0, 0.0]);
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return (b, [0.0, 1.0, 0.0]);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (a + ab * v, [1.0 - v, v, 0.0]);
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return (c, [0.0, 0.0, 1.0]);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (a + ac * w, [1.0 - w, 0.0, w]);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (b + (c - b) * w, [0.0, 1.0 - w, w]);
    }
    let denom = 1.0 / (va + vb + vc);
    let (v, w) = (vb * denom, vc * denom);
    (a + ab * v + ac * w, [1.0 - v - w, v, w])
}

/// The source surface, bucketed in a grid for nearest-triangle queries.
pub struct Source<'a> {
    tris: &'a [Tri],
    textures: &'a [Option<Image>],
    lo: Vec3,
    cell: f32,
    dims: [usize; 3],
    cells: Vec<Vec<u32>>,
}

impl<'a> Source<'a> {
    /// `textures[slot]`: the image each triangle's slot samples.
    pub fn new(tris: &'a [Tri], textures: &'a [Option<Image>]) -> Source<'a> {
        let tris_ok = |t: &&Tri| textures.get(t.slot as usize).is_some_and(|x| x.is_some());
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for t in tris.iter().filter(tris_ok) {
            for p in t.p {
                lo = lo.min(p);
                hi = hi.max(p);
            }
        }
        if lo.x > hi.x {
            return Source { tris, textures, lo: Vec3::ZERO, cell: 1.0, dims: [0; 3], cells: Vec::new() };
        }
        let ext = (hi - lo).max_element().max(1e-3);
        let cell = ext / 48.0;
        let dims = [0, 1, 2].map(|k| (((hi[k] - lo[k]) / cell) as usize + 1).min(64));
        let mut cells = vec![Vec::new(); dims[0] * dims[1] * dims[2]];
        let mut s = Source { tris, textures, lo, cell, dims, cells: Vec::new() };
        for (i, t) in tris.iter().enumerate().filter(|(_, t)| tris_ok(t)) {
            let (a, b) = (t.p[0].min(t.p[1]).min(t.p[2]), t.p[0].max(t.p[1]).max(t.p[2]));
            let (ca, cb) = (s.cell_of(a), s.cell_of(b));
            for z in ca[2]..=cb[2] {
                for y in ca[1]..=cb[1] {
                    for x in ca[0]..=cb[0] {
                        cells[(z * dims[1] + y) * dims[0] + x].push(i as u32);
                    }
                }
            }
        }
        s.cells = cells;
        s
    }

    fn cell_of(&self, p: Vec3) -> [usize; 3] {
        [0, 1, 2].map(|k| (((p[k] - self.lo[k]) / self.cell).floor().max(0.0) as usize).min(self.dims[k].saturating_sub(1)))
    }

    /// Triangles in the cells overlapping the box `a..b`.
    fn near(&self, a: Vec3, b: Vec3, seen: &mut [u32], stamp: u32, out: &mut Vec<u32>) {
        out.clear();
        if self.cells.is_empty() {
            return;
        }
        let (ca, cb) = (self.cell_of(a), self.cell_of(b));
        for z in ca[2]..=cb[2] {
            for y in ca[1]..=cb[1] {
                for x in ca[0]..=cb[0] {
                    for &i in &self.cells[(z * self.dims[1] + y) * self.dims[0] + x] {
                        if seen[i as usize] != stamp {
                            seen[i as usize] = stamp;
                            out.push(i);
                        }
                    }
                }
            }
        }
    }
}

/// How a transfer went.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    /// Target triangles on this slot, texels they cover, texels filled from neighbours.
    pub triangles: usize,
    pub covered: usize,
    pub filled: usize,
    /// The mean and largest distance from a target texel's point to the source surface.
    pub mean_gap: f32,
    pub max_gap: f32,
}

/// `base` (the target's own texture for `slot`: its size and alpha are kept) recoloured from the
/// source surface: each texel covered by one of `target`'s triangles on `slot` takes the colour of
/// the nearest source point facing the same way (a back-facing one counts as further); the rest
/// take their covered neighbours' colour.
pub fn transfer(target: &[Tri], slot: u32, base: &Image, src: &Source) -> (Image, Stats) {
    let (w, h) = (base.w as usize, base.h as usize);
    let mut out = base.clone();
    // Per texel: how far outside its triangle (texels; < 0 not yet written).
    let mut edge = vec![f32::MAX; w * h];
    let mut seen = vec![0u32; src.tris.len()];
    let mut cand = Vec::new();
    let mut st = Stats::default();
    let mut gaps = 0f64;
    let facing = src.cell;
    for (ti, t) in target.iter().enumerate().filter(|(_, t)| t.slot == slot) {
        st.triangles += 1;
        let px = t.uv.map(|uv| Vec2::new(uv.x * w as f32, uv.y * h as f32));
        let area = (px[1] - px[0]).perp_dot(px[2] - px[0]);
        if area.abs() < 1e-6 {
            continue;
        }
        let x0 = (px[0].x.min(px[1].x).min(px[2].x) - 1.0).floor().max(0.0) as usize;
        let x1 = ((px[0].x.max(px[1].x).max(px[2].x) + 1.0).ceil() as usize).min(w);
        let y0 = (px[0].y.min(px[1].y).min(px[2].y) - 1.0).floor().max(0.0) as usize;
        let y1 = ((px[0].y.max(px[1].y).max(px[2].y) + 1.0).ceil() as usize).min(h);
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        // Source triangles near this one, the search widened until the nearest is inside it.
        let (lo, hi) = (t.p[0].min(t.p[1]).min(t.p[2]), t.p[0].max(t.p[1]).max(t.p[2]));
        let mut r = src.cell;
        let texels: Vec<(usize, f32, Vec3)> = (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .filter_map(|(x, y)| {
                let c = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                // Signed distances (texels) inside each edge.
                let s = area.signum();
                let d = [0, 1, 2].map(|k| {
                    let (a, b) = (px[k], px[(k + 1) % 3]);
                    s * (b - a).perp_dot(c - a) / (b - a).length().max(1e-6)
                });
                let out_by = -d[0].min(d[1]).min(d[2]);
                if out_by > 1.0 {
                    return None;
                }
                let i = y * w + x;
                if out_by >= edge[i] {
                    return None;
                }
                // Barycentric of the centre (clamped onto the triangle for edge texels).
                let l1 = (c - px[0]).perp_dot(px[2] - px[0]) / area;
                let l2 = (px[1] - px[0]).perp_dot(c - px[0]) / area;
                let (l1, l2) = (l1.clamp(0.0, 1.0), l2.clamp(0.0, 1.0));
                let sum = (l1 + l2).max(1.0);
                let (l1, l2) = (l1 / sum, l2 / sum);
                let p = t.p[0] * (1.0 - l1 - l2) + t.p[1] * l1 + t.p[2] * l2;
                Some((i, out_by, p))
            })
            .collect();
        if texels.is_empty() {
            continue;
        }
        loop {
            src.near(lo - Vec3::splat(r), hi + Vec3::splat(r), &mut seen, ti as u32 + 1, &mut cand);
            let mut worst = 0f32;
            let mut found: Vec<(usize, f32, [f32; 3], f32)> = Vec::with_capacity(texels.len());
            for &(i, out_by, p) in &texels {
                let mut best: Option<(f32, f32, u32, [f32; 3])> = None;
                for &ci in &cand {
                    let s = &src.tris[ci as usize];
                    let (q, bary) = closest(p, &s.p);
                    let d = (q - p).length();
                    let score = d + facing * (1.0 - s.n.dot(t.n)).max(0.0);
                    if best.map_or(true, |b| score < b.0) {
                        best = Some((score, d, ci, bary));
                    }
                }
                let Some((_, d, ci, bary)) = best else {
                    worst = f32::MAX;
                    break;
                };
                worst = worst.max(d);
                let s = &src.tris[ci as usize];
                let uv = s.uv[0] * bary[0] + s.uv[1] * bary[1] + s.uv[2] * bary[2];
                let col = src.textures[s.slot as usize].as_ref().map_or([0.0; 3], |img| img.sample(uv));
                found.push((i, out_by, col, d));
            }
            let all = r >= src.cell * src.dims.iter().copied().max().unwrap_or(1) as f32;
            if worst <= r || all {
                for (i, out_by, col, d) in found {
                    if edge[i] == f32::MAX {
                        st.covered += 1;
                        gaps += d as f64;
                        st.max_gap = st.max_gap.max(d);
                    }
                    edge[i] = out_by;
                    out.bgra[i * 4..i * 4 + 3].copy_from_slice(&col.map(|v| (v + 0.5).clamp(0.0, 255.0) as u8));
                }
                break;
            }
            r = if worst == f32::MAX { r * 2.0 } else { worst.max(r * 1.5) };
        }
    }
    st.mean_gap = if st.covered > 0 { (gaps / st.covered as f64) as f32 } else { 0.0 };
    // Uncovered texels: their covered neighbours' colour, spreading out (bilinear filtering and
    // mips reach a little past the UV islands); what is left, the mean.
    let mut done: Vec<bool> = edge.iter().map(|&e| e != f32::MAX).collect();
    for _ in 0..16 {
        let mut next = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if done[i] {
                    continue;
                }
                let (mut sum, mut n) = ([0u32; 3], 0);
                for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if done[j] {
                        for k in 0..3 {
                            sum[k] += out.bgra[j * 4 + k] as u32;
                        }
                        n += 1;
                    }
                }
                if n > 0 {
                    next.push((i, sum.map(|s| (s / n) as u8)));
                }
            }
        }
        if next.is_empty() {
            break;
        }
        for (i, c) in next {
            out.bgra[i * 4..i * 4 + 3].copy_from_slice(&c);
            done[i] = true;
            st.filled += 1;
        }
    }
    let covered: Vec<usize> = (0..w * h).filter(|&i| done[i]).collect();
    let mean = if covered.is_empty() {
        [0u8; 3]
    } else {
        [0, 1, 2].map(|k| (covered.iter().map(|&i| out.bgra[i * 4 + k] as u64).sum::<u64>() / covered.len() as u64) as u8)
    };
    for i in (0..w * h).filter(|&i| !done[i]) {
        out.bgra[i * 4..i * 4 + 3].copy_from_slice(&mean);
    }
    (out, st)
}

/// Where a frame sheet's first frame (`frame_h` rows) sits in `face` (1:1), and how close it is
/// (mean difference per channel, 0..255): a texture-animation sheet pasted onto the face each
/// draw (Sora's eyes, `-2c0`, 320×128 frames, and mouth, `-2c1`).
pub fn find_frame(face: &Image, sheet: &Image, frame_h: u32) -> Option<(u32, u32, f32)> {
    let (fw, fh) = (sheet.w as usize, frame_h as usize);
    let (w, h) = (face.w as usize, face.h as usize);
    if fw > w || fh > h || fh == 0 || sheet.h < frame_h {
        return None;
    }
    let diff = |x: usize, y: usize, step: usize| {
        let mut sum = 0u64;
        let mut n = 0u64;
        for fy in (0..fh).step_by(step) {
            let (a, b) = (((y + fy) * w + x) * 4, fy * fw * 4);
            for fx in (0..fw).step_by(step) {
                for k in 0..3 {
                    sum += (face.bgra[a + fx * 4 + k] as i32 - sheet.bgra[b + fx * 4 + k] as i32).unsigned_abs() as u64;
                }
                n += 3;
            }
        }
        sum as f32 / n.max(1) as f32
    };
    // Coarse (every 4th texel, steps of 4), then the neighbourhood texel by texel.
    let mut best = (f32::MAX, 0, 0);
    for y in (0..=h - fh).step_by(4) {
        for x in (0..=w - fw).step_by(4) {
            let d = diff(x, y, 4);
            if d < best.0 {
                best = (d, x, y);
            }
        }
    }
    let (cx, cy) = (best.1, best.2);
    let mut fine = (f32::MAX, cx, cy);
    for y in cy.saturating_sub(4)..=(cy + 4).min(h - fh) {
        for x in cx.saturating_sub(4)..=(cx + 4).min(w - fw) {
            let d = diff(x, y, 1);
            if d < fine.0 {
                fine = (d, x, y);
            }
        }
    }
    Some((fine.1 as u32, fine.2 as u32, fine.0))
}

/// A frame sheet like `sheet` whose every frame is `face`'s rectangle at (`x`, `y`) (the sheet's
/// alpha kept).
pub fn frames_from(face: &Image, x: u32, y: u32, sheet: &Image, frame_h: u32) -> Image {
    let mut out = sheet.clone();
    let (fw, w) = (sheet.w as usize, face.w as usize);
    for row in 0..sheet.h as usize {
        let fy = y as usize + row % frame_h as usize;
        if fy >= face.h as usize {
            continue;
        }
        for col in 0..fw.min(w.saturating_sub(x as usize)) {
            let (a, b) = ((fy * w + x as usize + col) * 4, (row * fw + col) * 4);
            out.bgra[b..b + 3].copy_from_slice(&face.bgra[a..a + 3]);
        }
    }
    out
}

/// Anti Form's eyes (`docs/kh1-tint.md`): one flat yellow, as R, G, B. KH1's Anti-Sora has two
/// solid glowing yellow eyes, but nothing in his model file holds them (his geometry and textures
/// are Sora's mesh on grey shading; the only yellow in the file is one of his effects, which
/// couldn't be tied to the eyes), so this is the centre of his eyes as the user's reference
/// picture of him shows them (the brightest texels of both eyes: 253, 230, 95).
pub const ANTI_EYE_RGB: [u8; 3] = [253, 230, 95];
/// Where Sora's eyes are on his face texture (`-1c0`; u0, v0, u1, v1 fractions; both eyes with
/// their lids, all on the head bone, found by plotting mesh 0's vertices by bone).
pub const SORA_FACE_EYES: [f32; 4] = [0.01, 0.115, 0.30, 0.195];
/// How much bigger than Sora's eye opening Anti's blob is drawn (a rounder, fuller shape).
const BLOB_GROW: f32 = 1.12;
/// An eye showing fewer texels of iris and white than this is shut (a blink frame): no blob.
const BLOB_MIN: f32 = 12.0;

fn hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let mx = r.max(g).max(b);
    let c = mx - r.min(g).min(b);
    if c <= 1e-6 || mx <= 0.0 {
        return (0.0, 0.0, mx);
    }
    let h = if mx == r {
        ((g - b) / c).rem_euclid(6.0)
    } else if mx == g {
        (b - r) / c + 2.0
    } else {
        (r - g) / c + 4.0
    } * 60.0;
    (h, c / mx, mx)
}

/// How much a texel (B, G, R) of Sora's eye drawing is the eyeball: the iris (blue, any
/// brightness, so the pupil too) or the white, 0..1. His skin, lids and lashes are 0.
pub fn eye_weight(bgr: &[u8]) -> f32 {
    let (hue, s, v) = hsv(bgr[2] as f32 / 255.0, bgr[1] as f32 / 255.0, bgr[0] as f32 / 255.0);
    let dh = (hue - 220.0).rem_euclid(360.0);
    let iris = ((40.0 - dh.min(360.0 - dh)) / 10.0).clamp(0.0, 1.0) * ((s - 0.25) / 0.1).clamp(0.0, 1.0);
    let white = ((0.25 - s) / 0.1).clamp(0.0, 1.0) * ((v - 0.55) / 0.1).clamp(0.0, 1.0);
    iris.max(white)
}

/// Sora's eyes within the rectangle `area` (x, y, w, h texels) of `orig` drawn on `out` (the same
/// size) as two solid blobs of `rgb`: each half of the area (one eye each) gets an ellipse fitted
/// to its eyeball (the iris and white texels' spread, `eye_weight`), a little enlarged, filled
/// flat and opaque (alpha kept; one texel of edge blended for smoothness), clipped to that half.
/// Nothing of Sora's eye drawing (pupil, highlight, outline) is kept. A shut eye (a blink frame)
/// gets none. Returns how many eyes were drawn (0..2).
pub fn solid_eyes(out: &mut Image, orig: &Image, area: [u32; 4], rgb: [u8; 3]) -> usize {
    if (out.w, out.h) != (orig.w, orig.h) {
        return 0;
    }
    let (w, h) = (orig.w as usize, orig.h as usize);
    let [ax, ay, aw, ah] = area.map(|v| v as usize);
    let (x_end, y_end) = ((ax + aw).min(w), (ay + ah).min(h));
    let mid = ax + aw / 2;
    let mut drawn = 0;
    for (x0, x1) in [(ax, mid.min(x_end)), (mid, x_end)] {
        // The eyeball's weighted moments.
        let (mut n, mut sx, mut sy) = (0f64, 0f64, 0f64);
        let mut pts = Vec::new();
        for y in ay..y_end {
            for x in x0..x1 {
                let i = (y * w + x) * 4;
                let e = eye_weight(&orig.bgra[i..i + 3]);
                if e > 0.3 {
                    pts.push((x as f64 + 0.5, y as f64 + 0.5, e as f64));
                    n += e as f64;
                    sx += e as f64 * (x as f64 + 0.5);
                    sy += e as f64 * (y as f64 + 0.5);
                }
            }
        }
        if n < BLOB_MIN as f64 {
            continue;
        }
        let (cx, cy) = (sx / n, sy / n);
        let (mut cxx, mut cyy, mut cxy) = (0f64, 0f64, 0f64);
        for &(x, y, e) in &pts {
            cxx += e * (x - cx) * (x - cx);
            cyy += e * (y - cy) * (y - cy);
            cxy += e * (x - cx) * (y - cy);
        }
        let (cxx, cyy, cxy) = (cxx / n, cyy / n, cxy / n);
        // Principal axes; a filled ellipse of semi-axis a has variance a²/4 along it.
        let half = (cxx - cyy) / 2.0;
        let root = (half * half + cxy * cxy).sqrt();
        let (l1, l2) = ((cxx + cyy) / 2.0 + root, ((cxx + cyy) / 2.0 - root).max(0.0));
        let angle = 0.5 * (2.0 * cxy).atan2(cxx - cyy);
        let (ca, sa) = (angle.cos(), angle.sin());
        let a = (2.0 * l1.sqrt() * BLOB_GROW as f64).max(1.0);
        let b = (2.0 * l2.sqrt() * BLOB_GROW as f64).max(1.0);
        drawn += 1;
        for y in ay..y_end {
            for x in x0..x1 {
                let (dx, dy) = (x as f64 + 0.5 - cx, y as f64 + 0.5 - cy);
                let (u, v) = (dx * ca + dy * sa, -dx * sa + dy * ca);
                let d = ((u / a).powi(2) + (v / b).powi(2)).sqrt();
                // Texels inside the edge: about (1 - d) × the ellipse's radius that way.
                let r = if d > 1e-9 { (u * u + v * v).sqrt() / d } else { a };
                let k = ((1.0 - d) * r + 0.5).clamp(0.0, 1.0) as f32;
                if k <= 0.0 {
                    continue;
                }
                let i = (y * w + x) * 4;
                for (c, &to) in [rgb[2], rgb[1], rgb[0]].iter().enumerate() {
                    let from = out.bgra[i + c] as f32;
                    out.bgra[i + c] = (from + (to as f32 - from) * k + 0.5).clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
    drawn
}

/// `solid_eyes` on every frame (`frame_h` rows each, the sheet's whole width) of an eye frame
/// sheet (Sora's `-2c0`). Returns how many frames have their eyes open (any blob drawn).
pub fn solid_eye_frames(out: &mut Image, orig: &Image, frame_h: u32, rgb: [u8; 3]) -> usize {
    if frame_h == 0 {
        return 0;
    }
    (0..orig.h / frame_h).filter(|f| solid_eyes(out, orig, [0, f * frame_h, orig.w, frame_h], rgb) > 0).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, c: [u8; 4]) -> Image {
        Image { w, h, bgra: c.repeat((w * h) as usize) }
    }

    fn quad(z: f32, slot: u32, uv0: Vec2, uv1: Vec2) -> [Tri; 2] {
        let p = [Vec3::new(0.0, 0.0, z), Vec3::new(10.0, 0.0, z), Vec3::new(10.0, 10.0, z), Vec3::new(0.0, 10.0, z)];
        let uv = [uv0, Vec2::new(uv1.x, uv0.y), uv1, Vec2::new(uv0.x, uv1.y)];
        let n = Vec3::Z;
        [Tri { p: [p[0], p[1], p[2]], uv: [uv[0], uv[1], uv[2]], slot, n }, Tri { p: [p[0], p[2], p[3]], uv: [uv[0], uv[2], uv[3]], slot, n }]
    }

    #[test]
    fn colours_follow_the_surface() {
        // Source: an 8×1 texture, left half red, right half blue, on a 10×10 square.
        let mut tex = solid(8, 1, [0, 0, 255, 255]);
        for x in 4..8 {
            tex.bgra[x * 4..x * 4 + 4].copy_from_slice(&[255, 0, 0, 255]);
        }
        let src_tris = quad(0.0, 0, Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
        let textures = [Some(tex)];
        let src = Source::new(&src_tris, &textures);
        // Target: the same square a little in front, mapped mirrored onto the top half of a 16×16.
        let dst = quad(0.2, 3, Vec2::new(1.0, 0.0), Vec2::new(0.0, 0.5));
        let base = solid(16, 16, [9, 9, 9, 77]);
        let (out, st) = transfer(&dst, 3, &base, &src);
        assert_eq!(st.triangles, 2);
        assert!(st.covered >= 16 * 8, "{st:?}");
        assert!((st.mean_gap - 0.2).abs() < 1e-3);
        let px = |x: usize, y: usize| &out.bgra[(y * 16 + x) * 4..(y * 16 + x) * 4 + 4];
        // Mirrored: the texture's left (u small) is the surface's right.
        assert!(px(1, 3)[0] > 200 && px(1, 3)[2] < 50, "{:?}", px(1, 3));
        assert!(px(14, 3)[2] > 200 && px(14, 3)[0] < 50, "{:?}", px(14, 3));
        // Alpha kept; the uncovered half filled.
        assert_eq!(px(5, 12)[3], 77);
        assert_ne!(px(5, 12)[..3], [9, 9, 9]);
    }

    #[test]
    fn islands_are_found_by_a_point() {
        let tri = |mesh, uv: [[f32; 2]; 3]| UvTri { mesh, slot: 1, uv: uv.map(|p| Vec2::new(p[0], p[1])) };
        // On a 16×16: a square of two triangles sharing a diagonal (one island), another square
        // beside it (another), and on slot 0 a triangle over the first (left alone).
        let tris = [
            tri(0, [[0.0, 0.0], [0.25, 0.0], [0.25, 0.25]]),
            tri(0, [[0.0, 0.0], [0.25, 0.25], [0.0, 0.25]]),
            tri(0, [[0.5, 0.5], [1.0, 0.5], [1.0, 1.0]]),
            tri(0, [[0.5, 0.5], [1.0, 1.0], [0.5, 1.0]]),
            UvTri { slot: 0, ..tri(0, [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]) },
        ];
        // A point in the first square's upper triangle takes the lower one too.
        let (m, n) = island_mask(&tris, 1, 16, 16, &[[0.2, 0.05]]);
        assert_eq!(n, 1);
        let at = |x: usize, y: usize| m[y * 16 + x];
        assert!(at(0, 3) && at(3, 0) && at(1, 2));
        // A texel past the edge, not two; the other square not.
        assert!(at(4, 2) && !at(5, 2));
        assert!(!at(12, 12) && !at(8, 8));
        // Both, by a point in each; a point on nothing finds nothing.
        let (m, n) = island_mask(&tris, 1, 16, 16, &[[0.1, 0.2], [0.9, 0.6]]);
        assert_eq!(n, 2);
        assert!(m[12 * 16 + 12] && m[2 * 16 + 2]);
        assert_eq!(island_mask(&tris, 1, 16, 16, &[[0.4, 0.1]]).1, 0);
    }

    #[test]
    fn back_facing_counts_as_further() {
        let tex_a = solid(1, 1, [0, 255, 0, 255]);
        let tex_b = solid(1, 1, [255, 0, 255, 255]);
        let mut src_tris = quad(0.0, 0, Vec2::ZERO, Vec2::ONE).to_vec();
        // Closer, but facing away.
        let mut back = quad(0.15, 1, Vec2::ZERO, Vec2::ONE);
        for t in &mut back {
            t.n = -Vec3::Z;
        }
        src_tris.extend(back);
        let textures = [Some(tex_a), Some(tex_b)];
        let src = Source::new(&src_tris, &textures);
        let dst = quad(0.2, 0, Vec2::ZERO, Vec2::ONE);
        let (out, _) = transfer(&dst, 0, &solid(4, 4, [0, 0, 0, 255]), &src);
        assert_eq!(&out.bgra[0..3], &[0, 255, 0]);
    }

    #[test]
    fn frames_are_found_and_rebuilt() {
        let mut face = solid(32, 32, [10, 20, 30, 255]);
        let mut sheet = solid(8, 8, [0, 0, 0, 200]);
        for y in 0..4 {
            for x in 0..8 {
                let v = (x * 30 + y * 7) as u8;
                let f = ((y + 12) * 32 + x + 5) * 4;
                face.bgra[f..f + 3].copy_from_slice(&[v, v, v]);
                let s = (y * 8 + x) * 4;
                sheet.bgra[s..s + 3].copy_from_slice(&[v, v, v]);
            }
        }
        let (x, y, d) = find_frame(&face, &sheet, 4).unwrap();
        assert_eq!((x, y), (5, 12));
        assert!(d < 0.01);
        let dark = solid(32, 32, [1, 2, 3, 255]);
        let f = frames_from(&dark, x, y, &sheet, 4);
        assert!(f.bgra.chunks_exact(4).all(|p| p == [1, 2, 3, 200]));
    }

    #[test]
    fn eyes_become_solid_blobs_and_shut_frames_stay_shut() {
        // A sheet of two 40×16 frames on skin: the first has two blue eyes (with a dark pupil and
        // a white highlight), the second is shut (skin and a dark lid line only).
        let skin = [90, 140, 220, 255];
        let mut orig = solid(40, 32, skin);
        let put = |img: &mut Image, x: usize, y: usize, c: [u8; 4]| img.bgra[(y * 40 + x) * 4..(y * 40 + x) * 4 + 4].copy_from_slice(&c);
        for (cx, cy) in [(10.0f32, 8.0f32), (30.0, 8.0)] {
            for y in 0..16 {
                for x in 0..40 {
                    let (dx, dy) = ((x as f32 + 0.5 - cx) / 6.0, (y as f32 + 0.5 - cy) / 4.0);
                    if dx * dx + dy * dy <= 1.0 {
                        put(&mut orig, x, y, [200, 90, 30, 255]);
                    }
                }
            }
            put(&mut orig, cx as usize, cy as usize, [40, 10, 5, 255]);
            put(&mut orig, cx as usize + 2, cy as usize - 1, [250, 250, 250, 255]);
        }
        for x in 4..36 {
            put(&mut orig, x, 24, [20, 30, 40, 255]);
        }
        let mut out = solid(40, 32, [5, 5, 5, 200]);
        let yellow = [250, 220, 90];
        assert_eq!(solid_eye_frames(&mut out, &orig, 16, yellow), 1);
        let px = |x: usize, y: usize| out.bgra[(y * 40 + x) * 4..(y * 40 + x) * 4 + 4].to_vec();
        // Flat yellow over the pupil and the highlight alike, alpha kept.
        for (x, y) in [(10, 8), (12, 7), (30, 8), (32, 7), (8, 9)] {
            assert_eq!(px(x, y), vec![90, 220, 250, 200], "({x}, {y})");
        }
        // Between the eyes and in the shut frame: untouched.
        assert_eq!(px(20, 8), vec![5, 5, 5, 200]);
        assert!((16..32).all(|y| (0..40).all(|x| px(x, y) == vec![5, 5, 5, 200])));
    }

    #[test]
    fn strips_make_triangles() {
        // One mesh, slot 2: bone 1 in matrix slot 0, four vertices as one strip (two triangles).
        let mut b = vec![0u8; 0x200];
        let put = |b: &mut Vec<u8>, o: usize, v: u32| b[o..o + 4].copy_from_slice(&v.to_le_bytes());
        put(&mut b, 4, 1);
        put(&mut b, 0x14, 2);
        put(&mut b, 0x1C, 0x20);
        let mut o = 0x30;
        put(&mut b, o, 0x11);
        put(&mut b, o + 4, 0x10);
        put(&mut b, o + 8, 1);
        o += 0x10;
        put(&mut b, o, 1);
        put(&mut b, o + 4, 0x20 + 48 * 4);
        put(&mut b, o + 8, 4);
        let flags = [0x1000_0000u32, 0x1000_0000, 0x1_0000, 0x2_0000];
        let pos = [[0.0f32, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        for k in 0..4 {
            let v = o + 0x20 + 48 * k;
            put(&mut b, v + 8, 1f32.to_bits());
            put(&mut b, v + 12, flags[k]);
            put(&mut b, v + 32, pos[k][0].to_bits());
            put(&mut b, v + 36, pos[k][1].to_bits());
        }
        o += 0x20 + 48 * 4;
        put(&mut b, o, 0x8000);
        put(&mut b, o + 4, 0x10);
        let bones = [Mat4::IDENTITY, Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0))];
        let tris = cvbl_triangles(&b, &bones).unwrap();
        assert_eq!(tris.len(), 2);
        assert!(tris.iter().all(|t| t.slot == 2 && t.n.z > 0.99));
        assert_eq!(tris[0].p[0], Vec3::new(5.0, 0.0, 0.0));
    }
}
