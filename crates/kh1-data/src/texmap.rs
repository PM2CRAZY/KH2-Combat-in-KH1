//! Carrying a texture from one model's UV layout to another's, by the surface (Anti Form's look:
//! KH1 Anti-Sora's textures onto Sora's HD textures, `kh1-hook`'s `tint`, `docs/kh1-tint.md`).
//!
//! Each texel of the target's texture lies on one of the target's triangles (rasterised in UV
//! space, edge texels up to one texel outside), so it is a point on the target's surface in the
//! bind pose. The nearest point of the source's surface facing the same way gives the source's UV
//! there, and the source's texture is sampled (bilinear). Texels no triangle covers take their
//! neighbours' colour, then the mean; the target's alpha is kept (Sora's hair cut-outs belong to
//! his layout). KH1's Anti-Sora (`xa_pp_3020.mdls`) is Sora's PS2 mesh (the same skeleton and
//! geometry), so the two surfaces lie on each other and each texel takes Anti-Sora's colour from
//! the same spot (`docs/kh1-tint.md`).
//!
//! Also here: Anti's solid yellow eyes (the owner, main s292) and the face's animated frame sheets
//! (KH1 pastes an eye and a mouth frame onto the face each draw) rebuilt from the new face.
//!
//! Nothing here holds game data; it works on the player's own files at run time.

use crate::image::Image;
use crate::mesh::Face;
use glam::{Mat4, Vec2, Vec3};

/// A triangle placed in the bind pose: its corners, UVs, texture slot and facing (a unit normal
/// by the strip's winding, so the two sides of a double-sided part stay apart).
#[derive(Clone, Copy, Debug)]
pub struct Tri {
    pub p: [Vec3; 3],
    pub uv: [Vec2; 3],
    pub slot: u32,
    pub n: Vec3,
}

/// A `.mdls`'s bind pose: one model-space matrix per bone.
pub fn bind_pose(mdls: &[u8]) -> kh2_formats::Result<Vec<Mat4>> {
    Ok(kh2_anim::Pose::bind(&crate::mdls::parse_skeleton(mdls)?).bones)
}

/// `faces` placed by `bones` (`bind_pose`); degenerate ones left out. `Err` names a bone the
/// skeleton lacks.
pub fn posed(faces: &[Face], bones: &[Mat4]) -> Result<Vec<Tri>, String> {
    let mut out = Vec::with_capacity(faces.len());
    for f in faces {
        let mut p = [Vec3::ZERO; 3];
        for (k, c) in f.corners.iter().enumerate() {
            p[k] = bones.get(c.bone).ok_or_else(|| format!("bone {} not in the skeleton ({})", c.bone, bones.len()))?.transform_point3(c.pos);
        }
        let face = (p[1] - p[0]).cross(p[2] - p[0]);
        if face.length_squared() < 1e-12 {
            continue;
        }
        out.push(Tri { p, uv: f.corners.map(|c| c.uv), slot: f.slot, n: face.normalize() * f.facing });
    }
    Ok(out)
}

/// The point of triangle `t` nearest `p` and its barycentric weights (Ericson, "Real-Time
/// Collision Detection", 5.1.5).
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
    let d = 1.0 / (va + vb + vc);
    let (v, w) = (vb * d, vc * d);
    (a + ab * v + ac * w, [1.0 - v - w, v, w])
}

/// The source: its triangles (those whose slot has a texture), bucketed in a grid.
pub struct Source<'a> {
    tris: &'a [Tri],
    textures: &'a [Option<Image>],
    lo: Vec3,
    cell: f32,
    dims: [usize; 3],
    cells: Vec<Vec<u32>>,
}

impl<'a> Source<'a> {
    /// `textures[slot]`: the image a triangle on `slot` samples (`None`: that slot's triangles are
    /// left out).
    pub fn new(tris: &'a [Tri], textures: &'a [Option<Image>]) -> Source<'a> {
        let usable = |t: &Tri| textures.get(t.slot as usize).is_some_and(|x| x.is_some());
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for t in tris.iter().filter(|t| usable(t)) {
            for p in t.p {
                lo = lo.min(p);
                hi = hi.max(p);
            }
        }
        let mut s = Source { tris, textures, lo, cell: 1.0, dims: [0; 3], cells: Vec::new() };
        if lo.x > hi.x {
            return s;
        }
        s.cell = (hi - lo).max_element().max(1e-3) / 48.0;
        s.dims = [0, 1, 2].map(|k| (((hi[k] - lo[k]) / s.cell) as usize + 1).min(64));
        let mut cells = vec![Vec::new(); s.dims.iter().product()];
        for (i, t) in tris.iter().enumerate().filter(|(_, t)| usable(t)) {
            let (a, b) = (s.cell_of(t.p[0].min(t.p[1]).min(t.p[2])), s.cell_of(t.p[0].max(t.p[1]).max(t.p[2])));
            for z in a[2]..=b[2] {
                for y in a[1]..=b[1] {
                    for x in a[0]..=b[0] {
                        cells[(z * s.dims[1] + y) * s.dims[0] + x].push(i as u32);
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

    /// The triangles in the cells the box `a..b` touches, once each (`seen` / `stamp`).
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

    /// The grid's whole extent (a search this wide sees every triangle).
    fn extent(&self) -> f32 {
        self.cell * self.dims.iter().copied().max().unwrap_or(1) as f32
    }
}

/// How a transfer went.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    /// The target's triangles on the slot, the texels they cover, the texels filled beside them.
    pub triangles: usize,
    pub covered: usize,
    pub filled: usize,
    /// The mean and the largest distance from a covered texel's point to the source's surface.
    pub mean_gap: f32,
    pub max_gap: f32,
}

/// `base` (the target's own texture for `slot`: its size and alpha kept) with each texel covered
/// by one of `target`'s triangles on `slot` coloured from `src` at the nearest point facing the
/// same way (one facing away counts as a grid cell further); the rest from their neighbours.
pub fn transfer(target: &[Tri], slot: u32, base: &Image, src: &Source) -> (Image, Stats) {
    let (w, h) = (base.w as usize, base.h as usize);
    let mut out = base.clone();
    // Per texel: how far outside its triangle it lies (texels); MAX: not covered.
    let mut edge = vec![f32::MAX; w * h];
    let mut seen = vec![0u32; src.tris.len()];
    let mut cand = Vec::new();
    let mut st = Stats::default();
    let mut gaps = 0f64;
    let away = src.cell;
    let scale = Vec2::new(w as f32, h as f32);
    for (ti, t) in target.iter().enumerate().filter(|(_, t)| t.slot == slot) {
        st.triangles += 1;
        let q = t.uv.map(|uv| uv * scale);
        let area = (q[1] - q[0]).perp_dot(q[2] - q[0]);
        if area.abs() < 1e-6 {
            continue;
        }
        let (lo, hi) = (q[0].min(q[1]).min(q[2]), q[0].max(q[1]).max(q[2]));
        let (x0, x1) = ((lo.x - 1.0).floor().max(0.0) as usize, ((hi.x + 1.0).ceil().max(0.0) as usize).min(w));
        let (y0, y1) = ((lo.y - 1.0).floor().max(0.0) as usize, ((hi.y + 1.0).ceil().max(0.0) as usize).min(h));
        // The texels this triangle colours: (index, how far outside, the surface point).
        let mut texels = Vec::new();
        for y in y0..y1 {
            for x in x0..x1 {
                let c = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let inside = (0..3).map(|k| area.signum() * (q[(k + 1) % 3] - q[k]).perp_dot(c - q[k]) / (q[(k + 1) % 3] - q[k]).length().max(1e-6)).fold(f32::MAX, f32::min);
                let out_by = (-inside).max(0.0);
                let i = y * w + x;
                if inside < -1.0 || out_by >= edge[i] {
                    continue;
                }
                // Barycentric of the centre, clamped onto the triangle for edge texels.
                let b1 = ((c - q[0]).perp_dot(q[2] - q[0]) / area).clamp(0.0, 1.0);
                let b2 = ((q[1] - q[0]).perp_dot(c - q[0]) / area).clamp(0.0, 1.0);
                let s = (b1 + b2).max(1.0);
                let (b1, b2) = (b1 / s, b2 / s);
                texels.push((i, out_by, t.p[0] * (1.0 - b1 - b2) + t.p[1] * b1 + t.p[2] * b2));
            }
        }
        if texels.is_empty() {
            continue;
        }
        // Source triangles near this one, the search widened until every texel's nearest is
        // inside it.
        let (plo, phi) = (t.p[0].min(t.p[1]).min(t.p[2]), t.p[0].max(t.p[1]).max(t.p[2]));
        let mut r = src.cell;
        loop {
            src.near(plo - Vec3::splat(r), phi + Vec3::splat(r), &mut seen, ti as u32 + 1, &mut cand);
            let mut worst = 0f32;
            let mut found = Vec::with_capacity(texels.len());
            for &(i, out_by, p) in &texels {
                let best = cand.iter().map(|&ci| {
                    let s = &src.tris[ci as usize];
                    let (at, bary) = closest(p, &s.p);
                    let d = (at - p).length();
                    (d + away * (1.0 - s.n.dot(t.n)).max(0.0), d, ci, bary)
                });
                let Some((_, d, ci, bary)) = best.min_by(|a, b| a.0.total_cmp(&b.0)) else {
                    worst = f32::MAX;
                    break;
                };
                worst = worst.max(d);
                let s = &src.tris[ci as usize];
                let uv = s.uv[0] * bary[0] + s.uv[1] * bary[1] + s.uv[2] * bary[2];
                let col = src.textures[s.slot as usize].as_ref().map_or([0.0; 3], |img| img.sample(uv.x, uv.y));
                found.push((i, out_by, col, d));
            }
            if worst <= r || r >= src.extent() {
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
    st.filled = fill_beside(&mut out, edge.iter().map(|&e| e != f32::MAX).collect());
    (out, st)
}

/// Texels not `done` take their done neighbours' mean colour, spreading out 16 texels (bilinear
/// filtering and mips reach past the UV islands); any left, the mean of all. Returns how many
/// were filled from neighbours.
fn fill_beside(img: &mut Image, mut done: Vec<bool>) -> usize {
    let (w, h) = (img.w as usize, img.h as usize);
    let mut filled = 0;
    for _ in 0..16 {
        let mut next = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if done[y * w + x] {
                    continue;
                }
                let (mut sum, mut n) = ([0u32; 3], 0);
                for (nx, ny) in [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)] {
                    if nx < w && ny < h && done[ny * w + nx] {
                        for k in 0..3 {
                            sum[k] += img.bgra[(ny * w + nx) * 4 + k] as u32;
                        }
                        n += 1;
                    }
                }
                if n > 0 {
                    next.push((y * w + x, sum.map(|s| (s / n) as u8)));
                }
            }
        }
        if next.is_empty() {
            break;
        }
        filled += next.len();
        for (i, c) in next {
            img.bgra[i * 4..i * 4 + 3].copy_from_slice(&c);
            done[i] = true;
        }
    }
    let have: Vec<usize> = (0..w * h).filter(|&i| done[i]).collect();
    if !have.is_empty() && have.len() < w * h {
        let mean = [0, 1, 2].map(|k| (have.iter().map(|&i| img.bgra[i * 4 + k] as u64).sum::<u64>() / have.len() as u64) as u8);
        for i in (0..w * h).filter(|&i| !done[i]) {
            img.bgra[i * 4..i * 4 + 3].copy_from_slice(&mean);
        }
    }
    filled
}

/// Where a frame sheet's first frame (`frame_h` rows, the sheet's width) is pasted on `face`
/// (1:1), and how close it is there (mean difference per channel, 0..255). KH1 pastes Sora's eye
/// frames (`-2c0`, 320×128 each) and mouth frames (`-2c1`) onto his face (`-1c0`) each draw.
pub fn find_frame(face: &Image, sheet: &Image, frame_h: u32) -> Option<(u32, u32, f32)> {
    let (fw, fh, w, h) = (sheet.w as usize, frame_h as usize, face.w as usize, face.h as usize);
    if fw > w || fh > h || fh == 0 || sheet.h < frame_h {
        return None;
    }
    let diff = |x: usize, y: usize, step: usize| {
        let (mut sum, mut n) = (0u64, 0u64);
        for fy in (0..fh).step_by(step) {
            for fx in (0..fw).step_by(step) {
                let (a, b) = (((y + fy) * w + x + fx) * 4, (fy * fw + fx) * 4);
                for k in 0..3 {
                    sum += (face.bgra[a + k] as i32 - sheet.bgra[b + k] as i32).unsigned_abs() as u64;
                }
                n += 3;
            }
        }
        sum as f32 / n.max(1) as f32
    };
    // Coarse (every 4th texel at steps of 4), then texel by texel around the best.
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

/// A frame sheet the size of `sheet` (its alpha kept) whose every frame is `face`'s rectangle at
/// (`x`, `y`): the face's own pixels where KH1 pastes a frame.
pub fn frames_from(face: &Image, x: u32, y: u32, sheet: &Image, frame_h: u32) -> Image {
    let mut out = sheet.clone();
    let (fw, w) = (sheet.w as usize, face.w as usize);
    for row in 0..sheet.h as usize {
        let fy = y as usize + row % frame_h.max(1) as usize;
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

/// Anti Form's eyes: one flat yellow (R, G, B). The owner (main s292): "i need the eyes to be
/// solid yellow, and have it be the same yellow as in anti-sora's model". Anti-Sora's model
/// holds no eye colour (main's study of his file, `docs/kh1-tint.md`: his textures are greys, no
/// vertex colour), so this is main's value, the brightest texels at the centre of both eyes in
/// the owner's reference picture of him. Checked (main's docs), not from the game's data.
pub const ANTI_EYE_RGB: [u8; 3] = [253, 230, 95];
/// Where Sora's eyes are on his face texture (`-1c0`; u0, v0, u1, v1 fractions; both eyes with
/// their lids, on the head bone). Main's plot of the remastered mesh 0 by bone (Checked (main's
/// docs); `texmap`'s local test finds both eyes there on the owner's `-1c0`).
pub const SORA_FACE_EYES: [f32; 4] = [0.01, 0.115, 0.30, 0.195];
/// How much larger than Sora's eye opening the solid eye is (a fuller shape; main's value).
const EYE_GROW: f64 = 1.12;
/// An eye showing less eyeball than this (texels of weight) is shut (a blink frame): left so.
const EYE_MIN: f64 = 12.0;

fn hsv(bgr: &[u8]) -> (f32, f32, f32) {
    let (r, g, b) = (bgr[2] as f32 / 255.0, bgr[1] as f32 / 255.0, bgr[0] as f32 / 255.0);
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

/// How much a texel (B, G, R) of Sora's eye drawing is eyeball, 0..1: the iris (blue, any
/// brightness, so the pupil too) or the white. His skin, lids and lashes are 0.
pub fn eye_weight(bgr: &[u8]) -> f32 {
    let (hue, s, v) = hsv(bgr);
    let dh = (hue - 220.0).rem_euclid(360.0);
    let iris = ((40.0 - dh.min(360.0 - dh)) / 10.0).clamp(0.0, 1.0) * ((s - 0.25) / 0.1).clamp(0.0, 1.0);
    let white = ((0.25 - s) / 0.1).clamp(0.0, 1.0) * ((v - 0.55) / 0.1).clamp(0.0, 1.0);
    iris.max(white)
}

/// Sora's eyes in the rectangle `area` (x, y, w, h texels) of `orig` drawn on `out` (the same
/// size) as two solid `rgb` eyes: in each half of the area (one eye) an ellipse fitted to the
/// eyeball (`eye_weight`'s texels: their weighted centre and spread; a filled ellipse of
/// semi-axis a has variance a²/4 along it), enlarged a little, filled flat (alpha kept, a texel
/// of edge blended). Nothing of Sora's eye drawing stays. A shut eye gets none. Returns how many
/// eyes were drawn (0..2).
pub fn solid_eyes(out: &mut Image, orig: &Image, area: [u32; 4], rgb: [u8; 3]) -> usize {
    if (out.w, out.h) != (orig.w, orig.h) {
        return 0;
    }
    let (w, h) = (orig.w as usize, orig.h as usize);
    let [ax, ay, aw, ah] = area.map(|v| v as usize);
    let (x_end, y_end) = ((ax + aw).min(w), (ay + ah).min(h));
    let mid = (ax + aw / 2).min(x_end);
    let mut drawn = 0;
    for (x0, x1) in [(ax, mid), (mid, x_end)] {
        let mut pts = Vec::new();
        for y in ay..y_end {
            for x in x0..x1 {
                let e = eye_weight(&orig.bgra[(y * w + x) * 4..(y * w + x) * 4 + 3]) as f64;
                if e > 0.3 {
                    pts.push((x as f64 + 0.5, y as f64 + 0.5, e));
                }
            }
        }
        let n: f64 = pts.iter().map(|p| p.2).sum();
        if n < EYE_MIN {
            continue;
        }
        let (cx, cy) = (pts.iter().map(|p| p.0 * p.2).sum::<f64>() / n, pts.iter().map(|p| p.1 * p.2).sum::<f64>() / n);
        let m = |f: &dyn Fn(f64, f64) -> f64| pts.iter().map(|&(x, y, e)| e * f(x - cx, y - cy)).sum::<f64>() / n;
        let (sxx, syy, sxy) = (m(&|x, _| x * x), m(&|_, y| y * y), m(&|x, y| x * y));
        let root = (((sxx - syy) / 2.0).powi(2) + sxy * sxy).sqrt();
        let (l1, l2) = ((sxx + syy) / 2.0 + root, ((sxx + syy) / 2.0 - root).max(0.0));
        let angle = 0.5 * (2.0 * sxy).atan2(sxx - syy);
        let (ca, sa) = (angle.cos(), angle.sin());
        let (a, b) = ((2.0 * l1.sqrt() * EYE_GROW).max(1.0), (2.0 * l2.sqrt() * EYE_GROW).max(1.0));
        drawn += 1;
        for y in ay..y_end {
            for x in x0..x1 {
                let (dx, dy) = (x as f64 + 0.5 - cx, y as f64 + 0.5 - cy);
                let (u, v) = (dx * ca + dy * sa, -dx * sa + dy * ca);
                let d = ((u / a).powi(2) + (v / b).powi(2)).sqrt();
                // About (1 - d) × the ellipse's radius that way: texels inside the edge.
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

/// `solid_eyes` on every frame (`frame_h` rows, the sheet's width) of an eye frame sheet (Sora's
/// `-2c0`), so the eyes still blink. Returns how many frames have their eyes open.
pub fn solid_eye_frames(out: &mut Image, orig: &Image, frame_h: u32, rgb: [u8; 3]) -> usize {
    if frame_h == 0 {
        return 0;
    }
    (0..orig.h / frame_h).filter(|f| solid_eyes(out, orig, [0, f * frame_h, orig.w, frame_h], rgb) > 0).count()
}

/// `SORA_FACE_EYES` as a texel rectangle (x, y, w, h) of a `w`×`h` face.
pub fn face_eyes_area(w: u32, h: u32) -> [u32; 4] {
    let [u0, v0, u1, v1] = SORA_FACE_EYES;
    let (w, h) = (w as f32, h as f32);
    [(u0 * w) as u32, (v0 * h) as u32, ((u1 - u0) * w).ceil() as u32, ((v1 - v0) * h).ceil() as u32]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, c: [u8; 4]) -> Image {
        Image { w, h, bgra: c.repeat((w * h) as usize) }
    }

    /// A 10×10 square at height `z` facing +z, its UVs `uv0..uv1`.
    fn square(z: f32, slot: u32, uv0: Vec2, uv1: Vec2) -> [Tri; 2] {
        let p = [Vec3::new(0.0, 0.0, z), Vec3::new(10.0, 0.0, z), Vec3::new(10.0, 10.0, z), Vec3::new(0.0, 10.0, z)];
        let uv = [uv0, Vec2::new(uv1.x, uv0.y), uv1, Vec2::new(uv0.x, uv1.y)];
        [Tri { p: [p[0], p[1], p[2]], uv: [uv[0], uv[1], uv[2]], slot, n: Vec3::Z }, Tri { p: [p[0], p[2], p[3]], uv: [uv[0], uv[2], uv[3]], slot, n: Vec3::Z }]
    }

    #[test]
    fn colours_follow_the_surface() {
        // Source: an 8×1 texture, red on the left half, blue on the right, on the square.
        let mut tex = solid(8, 1, [0, 0, 255, 255]);
        for x in 4..8 {
            tex.bgra[x * 4..x * 4 + 4].copy_from_slice(&[255, 0, 0, 255]);
        }
        let src_tris = square(0.0, 0, Vec2::ZERO, Vec2::ONE);
        let textures = [Some(tex)];
        let src = Source::new(&src_tris, &textures);
        // Target: the same square a little above, mirrored onto the top half of a 16×16.
        let dst = square(0.2, 3, Vec2::new(1.0, 0.0), Vec2::new(0.0, 0.5));
        let (out, st) = transfer(&dst, 3, &solid(16, 16, [9, 9, 9, 77]), &src);
        assert_eq!(st.triangles, 2);
        assert!(st.covered >= 16 * 8, "{st:?}");
        assert!((st.mean_gap - 0.2).abs() < 1e-3, "{st:?}");
        let px = |x: usize, y: usize| out.bgra[(y * 16 + x) * 4..(y * 16 + x) * 4 + 4].to_vec();
        // Mirrored: the texture's left is the surface's right.
        assert!(px(1, 3)[0] > 200 && px(1, 3)[2] < 50, "{:?}", px(1, 3));
        assert!(px(14, 3)[2] > 200 && px(14, 3)[0] < 50, "{:?}", px(14, 3));
        // Alpha kept; the uncovered half filled from beside.
        assert_eq!(px(5, 12)[3], 77);
        assert_ne!(px(5, 12)[..3], [9, 9, 9]);
    }

    #[test]
    fn facing_away_counts_as_further() {
        let mut src_tris = square(0.0, 0, Vec2::ZERO, Vec2::ONE).to_vec();
        // Nearer, but facing away.
        let mut back = square(0.15, 1, Vec2::ZERO, Vec2::ONE);
        for t in &mut back {
            t.n = -Vec3::Z;
        }
        src_tris.extend(back);
        let textures = [Some(solid(1, 1, [0, 255, 0, 255])), Some(solid(1, 1, [255, 0, 255, 255]))];
        let src = Source::new(&src_tris, &textures);
        let (out, _) = transfer(&square(0.2, 0, Vec2::ZERO, Vec2::ONE), 0, &solid(4, 4, [0, 0, 0, 255]), &src);
        assert_eq!(&out.bgra[0..3], &[0, 255, 0]);
    }

    #[test]
    fn faces_are_posed_by_their_bones() {
        use crate::mesh::Corner;
        let c = |x: f32, y: f32| Corner { bone: 1, pos: Vec3::new(x, y, 0.0), uv: Vec2::new(x, y) };
        let f = Face { mesh: 0, slot: 2, corners: [c(0.0, 0.0), c(1.0, 0.0), c(0.0, 1.0)], facing: -1.0 };
        let bones = [Mat4::IDENTITY, Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0))];
        let t = posed(&[f], &bones).unwrap();
        assert_eq!(t[0].p[0], Vec3::new(5.0, 0.0, 0.0));
        assert!(t[0].n.z < -0.99 && t[0].slot == 2);
        assert!(posed(&[f], &bones[..1]).is_err());
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
                sheet.bgra[(y * 8 + x) * 4..(y * 8 + x) * 4 + 3].copy_from_slice(&[v, v, v]);
            }
        }
        let (x, y, d) = find_frame(&face, &sheet, 4).unwrap();
        assert_eq!((x, y), (5, 12));
        assert!(d < 0.01);
        let f = frames_from(&solid(32, 32, [1, 2, 3, 255]), x, y, &sheet, 4);
        assert!(f.bgra.chunks_exact(4).all(|p| p == [1, 2, 3, 200]));
    }

    #[test]
    fn eyes_become_solid_and_shut_frames_stay_shut() {
        // Two 40×16 frames on skin: the first with two blue eyes (a dark pupil, a white
        // highlight), the second shut (a dark lid line only).
        let mut orig = solid(40, 32, [90, 140, 220, 255]);
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
        assert_eq!(solid_eye_frames(&mut out, &orig, 16, [250, 220, 90]), 1);
        let px = |x: usize, y: usize| out.bgra[(y * 40 + x) * 4..(y * 40 + x) * 4 + 4].to_vec();
        for (x, y) in [(10, 8), (12, 7), (30, 8), (32, 7), (8, 9)] {
            assert_eq!(px(x, y), vec![90, 220, 250, 200], "({x}, {y})");
        }
        assert_eq!(px(20, 8), vec![5, 5, 5, 200]);
        assert!((16..32).all(|y| (0..40).all(|x| px(x, y) == vec![5, 5, 5, 200])));
    }
}
