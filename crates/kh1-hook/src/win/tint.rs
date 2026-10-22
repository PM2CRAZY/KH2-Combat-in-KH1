//! Sora's clothes recoloured in a Drive Form (the user: Wisdom Form, "recolor KH1 Sora blue";
//! s233: Wisdom's sleeves blue too, and Valor red: "his shoes red, his sleeves red, his belt red,
//! the inside part of his hoodie red"). Each look recolours its parts of the textures (`ops`):
//! a hue shift (red or yellow or blue to another hue) or a colourize (light grey and cream given
//! a hue), some only within a region of the texture (`SLEEVES`, `BELT`, `HOOD_LINING`), or
//! within a part found by the mesh's triangles (`Mask`: Limit Form's jacket).
//!
//! The PC draws the remastered Sora (`docs/kh1-tint.md`): 9 meshes on 4 HD textures, the
//! jumpsuit, jacket, hood, zipper and crown all on the second (`-1c1.dds`, slot 1), and a strip
//! of red collar on the first (`-1c0.dds`, face, skin and hair). A shader tint per mesh can't
//! make only the red blue (the mesh shader scales and offsets each channel on its own), so the
//! texture itself is swapped: a copy of each with its red recoloured to blue (hue within ~12° of
//! red and saturated → hue 225°, same saturation and brightness; skin, the black and white
//! jacket, the blue trim stay), made once as a game texture (`0x1400F9F80`, KH1's own creator,
//! from the game thread), and written into the source texture's GPU-texture slot (`+0x70`) while
//! in the form, KH1's put back after. Everything that samples the source follows the pointer
//! (the pause menu's Sora too).
//!
//! The pixels: the HD texture file KH1 keeps in memory (`+0x68`, a DDS), else read from the PKG
//! (the model's remastered assets, `Kh1Install::read_remastered`), recoloured in the background.
//!
//! Anti Form (the user: "make sora look like shadow sora in this form (the miniboss from
//! neverland)", then "lets see if you can try copying the exact textures of KH1 anti-sora onto
//! sora's anti form model instead of replacing the model"): KH1's own Anti-Sora textures
//! (`xa_pp_3020.mdls`, read from the user's install at start) carried onto Sora's remastered UV
//! layout by the surface (`kh1_data::texmap`: the two meshes share their geometry), the shorts'
//! pockets (`-1c2`) and the face's animated eye and mouth frames (`-2c0`, `-2c1`, pasted onto the
//! face each frame) included, with solid yellow eyes (Anti-Sora's face has none). If that can't
//! be made, every texture of Sora's darkened instead (`Op::Shadow`: near-black with a faint
//! blue-violet sheen, its shading kept), and the log says why.

use super::addr::Rva;
use super::{log, pose};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// The render state (per thread) and KH1's texture creator (`(R, &D3D12_RESOURCE_DESC, pixels,
/// name, 0)` → GPU texture object, synchronous).
const STATE_RVA: Rva = Rva(0xFD9C0);
const CREATE_RVA: Rva = Rva(0xF9F80);
/// `Axa::EntryTex`: its magic, the DDS in memory, the GPU texture, the size.
const TEX_MAGIC: u32 = 0x02F0_2EB5;
const TEX_DDS: usize = 0x68;
const TEX_GPU: usize = 0x70;
const TEX_W: usize = 0x7C;
const TEX_H: usize = 0x7E;
/// Where a swapped texture is found on Sora's model.
#[derive(Clone, Copy)]
enum Tex {
    /// A mesh-table entry's source texture.
    Mesh(usize),
    /// A texture-animation frame sheet (its name ends `mdls<tag>.<ext>`): KH1 pastes a frame of
    /// it onto the face's copy each draw (`docs/kh1-tint.md`).
    Frames(&'static [u8]),
}

/// The textures that can be swapped, in the order `ops` numbers them (`k`), with their PKG
/// asset: mesh table entries 0 and 1 (`-1c0`, the collar; `-1c1`, the clothes), 3 (`-1c3`, the
/// shoes; the user: "soras shoes should turn from yellow to blue in this form"; every texel of
/// it is on a leg or foot bone, `docs/kh1-tint.md`), and for Anti also entry 2 (`-1c2`, the
/// shorts' pockets) and the face's eye and mouth frames (`-2c0`, `-2c1`).
const TEXTURES: [(Tex, &str); 6] = [
    (Tex::Mesh(0), "-1c0"),
    (Tex::Mesh(1), "-1c1"),
    (Tex::Mesh(3), "-1c3"),
    (Tex::Mesh(2), "-1c2"),
    (Tex::Frames(b"mdls2c0."), "-2c0"),
    (Tex::Frames(b"mdls2c1."), "-2c1"),
];
const NTEX: usize = TEXTURES.len();
const YELLOW_HUE: f32 = 48.0;
/// (Wide enough for the orange where the yellow shades off; only bright texels, so the dark
/// brown beside the shoes stays.)
const YELLOW_WIDTH: f32 = 26.0;
const YELLOW_MIN_V: f32 = 0.42;
/// The blue of the belt and of the hood's lining (hue, half width).
const BLUE_HUE: f32 = 225.0;
const BLUE_WIDTH: f32 = 22.0;
const MODEL: &str = "xa_ex_0010.mdls";
/// KH1's Anti-Sora (the dark Sora of Neverland's ship): Sora's PS2 mesh, the same geometry as the
/// remastered one the PC draws for Sora, on seven 512² textures of the PS2 layout (`-1c0..-1c6`;
/// `docs/kh1-tint.md`).
const ANTI_MODEL: &str = "xa_pp_3020.mdls";
const ANTI_SLOTS: u32 = 7;
/// A face frame sheet's frame height (the eyes, `-2c0`: 22 frames of 320×128; the mouth, `-2c1`).
const FRAME_H: u32 = 128;
/// How far (mean per channel, 0..255) a sheet's first frame may differ from the face where it is
/// pasted and still count as found (Sora's: about 6).
const FRAME_MATCH: f32 = 24.0;
/// Hue shift: how wide the colour moved is (degrees, soft edge after), the saturation it takes
/// from (soft edge after).
const RED_WIDTH: f32 = 12.0;
const RED_SOFT: f32 = 6.0;
const SAT_FROM: f32 = 0.55;
const SAT_SOFT: f32 = 0.15;
const BLUE: f32 = 225.0;
const RED: f32 = 0.0;
/// Final's white (a colour's brightness 0..1 to this range, so its folds still show) and black.
const WHITE_LO: f32 = 0.5;
const WHITE_HI: f32 = 0.97;
const BLACK_LO: f32 = 0.04;
const BLACK_HI: f32 = 0.2;
/// Limit Form's yellow belt: hue, saturation, and the brightness its dark blue's 0..1 maps from
/// (to 1.0).
const LIMIT_BELT_HUE: f32 = 45.0;
const LIMIT_BELT_SAT: f32 = 0.85;
const LIMIT_BELT_LO: f32 = 0.55;
/// Anti's Shadow Sora: brightness 0..1 to this range at this hue and saturation (pitch black,
/// the folds still faintly there, a blue-violet sheen).
const SHADOW_HUE: f32 = 260.0;
const SHADOW_SAT: f32 = 0.5;
const SHADOW_LO: f32 = 0.03;
const SHADOW_HI: f32 = 0.13;
/// Anti's eyes: solid yellow blobs (the user: "i need the eyes to be solid yellow, and have it be
/// the same yellow as in anti-sora's model"), one flat colour (`texmap::ANTI_EYE_RGB`) filling an
/// ellipse fitted to Sora's eyeball, on `-1c0`'s eyes and on every `-2c0` frame (a shut frame
/// stays shut, so they still blink). Anti-Sora's own textures have no eyes (his face is blank,
/// `docs/kh1-tint.md`), so they are added on his textures and on the fallback alike.
const ANTI_EYES: bool = true;

/// Where an `Op::Shadow` puts Anti's solid eyes.
#[derive(Clone, Copy)]
enum Eyes {
    /// The face's eyes, in this region (u0, v0, u1, v1 fractions; `texmap::SORA_FACE_EYES`).
    Face([f32; 4]),
    /// Every frame (`FRAME_H` rows) of the eye frame sheet (`-2c0`).
    Frames,
}

/// Regions of `-1c1` (u0, v0, u1, v1; texture fractions), found by plotting the remastered mesh
/// 1's vertices by bone on the texture (`docs/kh1-tint.md`): the jacket's sleeves (the cream
/// strip both arms' bones use; the zipper rail beside it below 0.62 left out), the belt (the
/// blue band of the top strip, meshes 4..8) and the hood's lining (the blue band of the hood).
const SLEEVES: [[f32; 4]; 2] = [[0.035, 0.415, 0.236, 0.85], [0.236, 0.415, 0.215 + 0.025, 0.62]];
const BELT: [f32; 4] = [0.035, 0.07, 0.57, 0.14];
const HOOD_LINING: [f32; 4] = [0.755, 0.72, 1.0, 1.0];
/// The collar's red strip on `-1c0` (the texture's one large red run, 3460 texels at u 0..0.018,
/// v 0.35..0.69). The other on it, a red oval on the shoe piece at u 0.40..0.49, v 0.63..0.70, is
/// left out: Master keeps the shoes as they are.
const COLLAR: [f32; 4] = [0.0, 0.34, 0.025, 0.70];
/// The jacket on `-1c1` (`Mask::Jacket`): the UV islands of the remastered mesh (`-c0.cvbl`, mesh
/// 1) holding these points (u, v), each less a region if given: the sleeves, the vest less its
/// open zipper (the teeth and tape along its front edges, painted on the vest's own triangles),
/// the hood (outside and lining), the sleeves' inside. Its cream has the hue and saturation of
/// the silver chain and zipper, so the jacket is found by its triangles, not by colour
/// (`docs/kh1-tint.md`).
const JACKET: [([f32; 2], Option<[f32; 4]>); 4] = [([0.12, 0.65], None), ([0.45, 0.88], Some([0.2, 0.6, 0.257, 1.0])), ([0.92, 0.75], None), ([0.737, 0.69], None)];

/// Sora's remastered mesh's triangles on its textures (`-c0.cvbl`, read at start; `None`: not
/// read, so the jacket can't be found).
static UV_TRIS: OnceLock<Option<Vec<kh1_data::texmap::UvTri>>> = OnceLock::new();

/// A form's look (`form`): Wisdom's blue, Valor's red, Final's white, Limit's black and yellow,
/// Master's yellow and black, Anti's Shadow Sora.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Look {
    Wisdom,
    Valor,
    Final,
    /// Limit Form: KH1 Sora's outfit with the jumpsuit, the jacket and the shoes black, the belt
    /// yellow.
    Limit,
    /// Master Form: the jumpsuit, the collar and the hood's lining yellow, the belt black.
    Master,
    /// (`form` maps Anti Form here; until it does, nothing names it.)
    Anti,
}

/// One recolouring.
#[derive(Clone, Copy)]
enum Op {
    /// Saturated texels near hue `from` (within `width`), at least `min_v` bright, to hue `to`,
    /// same saturation and brightness; within `rect` if given.
    Shift { from: f32, width: f32, to: f32, min_v: f32, rect: Option<[f32; 4]> },
    /// Light, unsaturated texels (white, cream, grey) given hue `to` at saturation `sat`, their
    /// brightness kept; within `rect`.
    Colourize { to: f32, sat: f32, rect: [f32; 4] },
    /// Saturated texels near hue `from` (as `Shift`) made grey: brightness `lo + (hi - lo) × v`
    /// (white with its shading kept, or black); within `rect` if given.
    Grey { from: f32, width: f32, min_v: f32, lo: f32, hi: f32, rect: Option<[f32; 4]> },
    /// Saturated texels near hue `from` (as `Shift`) given hue `to` at saturation `sat`,
    /// brightness `lo + (hi - lo) × v` (a dark colour made a bright one, its shading kept);
    /// within `rect` if given.
    Paint { from: f32, width: f32, to: f32, sat: f32, lo: f32, hi: f32, rect: Option<[f32; 4]> },
    /// Every texel given hue `to` at saturation `sat`, brightness `lo + (hi - lo) × v` (Shadow
    /// Sora's black); then, if `eyes` is given, Anti's solid yellow eyes there.
    Shadow { to: f32, sat: f32, lo: f32, hi: f32, eyes: Option<Eyes> },
    /// Every texel within `mask`, whatever its colour, made grey: brightness `lo + (hi - lo) × v`,
    /// never brighter than it was (so a black stays black).
    Darken { lo: f32, hi: f32, mask: Mask },
}

/// A part of a texture found by the mesh (`mask`).
#[derive(Clone, Copy, Debug)]
enum Mask {
    /// The jacket on `-1c1` (`JACKET`).
    Jacket,
}

/// `m`'s texels on a `w`×`h` texture (row-major), or `None` (the mesh not read, or the part not
/// found on it).
fn mask(m: Mask, w: u32, h: u32) -> Option<Vec<bool>> {
    let tris = UV_TRIS.get()?.as_ref()?;
    match m {
        Mask::Jacket => {
            let (wu, hu) = (w as usize, h as usize);
            let mut all = vec![false; wu * hu];
            let mut found = 0;
            for (seed, less) in JACKET {
                let (mut v, n) = kh1_data::texmap::island_mask(tris, 1, w, h, &[seed]);
                found += n.min(1);
                if let Some([u0, v0, u1, v1]) = less {
                    let (x0, x1) = (((u0 * w as f32) as usize).min(wu), ((u1 * w as f32).ceil() as usize).min(wu));
                    let (y0, y1) = (((v0 * h as f32) as usize).min(hu), ((v1 * h as f32).ceil() as usize).min(hu));
                    for y in y0..y1 {
                        v[y * wu + x0..y * wu + x1.max(x0)].fill(false);
                    }
                }
                all.iter_mut().zip(v).for_each(|(a, b)| *a |= b);
            }
            if found != JACKET.len() {
                log(&format!("form tint: {found} of the jacket's {} parts found on the mesh", JACKET.len()));
                if found == 0 {
                    return None;
                }
            }
            Some(all)
        }
    }
}

/// What each look does to each texture (`TEXTURES` order); none: that one isn't swapped.
fn ops(look: Look, k: usize) -> Vec<Op> {
    let shift = |from, width, to| Op::Shift { from, width, to, min_v: 0.0, rect: None };
    let shoes = |to| Op::Shift { from: YELLOW_HUE, width: YELLOW_WIDTH, to, min_v: YELLOW_MIN_V, rect: None };
    let white = |from, width, min_v| Op::Grey { from, width, min_v, lo: WHITE_LO, hi: WHITE_HI, rect: None };
    let black = |from, width, min_v| Op::Grey { from, width, min_v, lo: BLACK_LO, hi: BLACK_HI, rect: None };
    // (Master's yellow: Limit's belt's, the bright yellow near the shoes'.)
    let paint_yellow = |from, width, rect| Op::Paint { from, width, to: LIMIT_BELT_HUE, sat: LIMIT_BELT_SAT, lo: LIMIT_BELT_LO, hi: 1.0, rect };
    match (look, k) {
        // Wisdom (the user: "recolor KH1 Sora blue"): the reds and the shoes' yellow blue, and
        // the sleeves blue (the user, s232: "can you make his sleeves blue when in wisdom?").
        (Look::Wisdom, 0) => vec![shift(0.0, RED_WIDTH, BLUE)],
        (Look::Wisdom, 1) => {
            let mut v = vec![shift(0.0, RED_WIDTH, BLUE)];
            v.extend(SLEEVES.map(|rect| Op::Colourize { to: BLUE, sat: 0.7, rect }));
            v
        }
        (Look::Wisdom, 2) => vec![shoes(BLUE)],
        // Valor (the user: "make his shoes red, his sleeves red, his belt red, the inside part of
        // his hoodie red"); his jumpsuit is red already.
        (Look::Valor, 1) => {
            let mut v: Vec<Op> = SLEEVES.map(|rect| Op::Colourize { to: RED, sat: 0.75, rect }).to_vec();
            v.push(Op::Shift { from: BLUE_HUE, width: BLUE_WIDTH, to: RED, min_v: 0.0, rect: Some(BELT) });
            v.push(Op::Shift { from: BLUE_HUE, width: BLUE_WIDTH, to: RED, min_v: 0.0, rect: Some(HOOD_LINING) });
            v
        }
        (Look::Valor, 2) => vec![shoes(RED)],
        // Final (the user: "make sora's outfit White, including his shoes. turn his belt black
        // though"): the reds (jumpsuit, collar), the blues (hood lining, trim) and the shoes'
        // yellow white, shaded as before; the belt's blue black first (so the white pass leaves
        // it). The jacket's black trim stays black.
        (Look::Final, 0) => vec![white(RED, RED_WIDTH, 0.0)],
        (Look::Final, 1) => vec![
            Op::Grey { from: BLUE_HUE, width: BLUE_WIDTH, min_v: 0.0, lo: BLACK_LO, hi: BLACK_HI, rect: Some(BELT) },
            white(RED, RED_WIDTH, 0.0),
            white(BLUE_HUE, BLUE_WIDTH, 0.0),
        ],
        (Look::Final, 2) => vec![white(YELLOW_HUE, YELLOW_WIDTH, YELLOW_MIN_V), white(RED, RED_WIDTH, 0.0), white(BLUE_HUE, BLUE_WIDTH, 0.0)],
        // Limit (the user: "change sora's outfit in limit form to have his jumpsuit be black,
        // his belt yellow, and his shoes black"): the reds (jumpsuit, collar) black, shaded as
        // Final's belt; the belt's dark blue a bright yellow near the shoes' (first, so nothing
        // after touches it; a plain hue shift left it olive);
        // the shoes' yellow, red and blue black. Then (the user: "make limit forms jacket fully
        // black as well including the inside of his hood") the whole jacket black, shaded the
        // same: its cream, blue trim and the hood's blue lining; its zipper, the chain, the crown
        // and the belt stay.
        (Look::Limit, 0) => vec![black(RED, RED_WIDTH, 0.0)],
        (Look::Limit, 1) => vec![
            Op::Paint { from: BLUE_HUE, width: BLUE_WIDTH, to: LIMIT_BELT_HUE, sat: LIMIT_BELT_SAT, lo: LIMIT_BELT_LO, hi: 1.0, rect: Some(BELT) },
            black(RED, RED_WIDTH, 0.0),
            Op::Darken { lo: BLACK_LO, hi: BLACK_HI, mask: Mask::Jacket },
        ],
        (Look::Limit, 2) => vec![black(YELLOW_HUE, YELLOW_WIDTH, YELLOW_MIN_V), black(RED, RED_WIDTH, 0.0), black(BLUE_HUE, BLUE_WIDTH, 0.0)],
        // Master (the user: "I want sora's jumpsuit to be yellow, and the inside of sora's jacket's
        // hood be yellow as well instead of blue. I want soras belt to be black"): the belt's blue
        // black first, as Final's (so nothing after touches it); the hood lining's blue and the
        // reds (jumpsuit, collar) one bright yellow, Limit's belt yellow near the shoes' (`Op::Paint`:
        // a plain hue shift of the dark red and blue reads olive), their shading kept. The jacket's
        // blue trim outside the lining, the shoes and the rest stay (on `-1c0` only the collar's
        // strip: its other red is on the shoe piece).
        (Look::Master, 0) => vec![paint_yellow(RED, RED_WIDTH, Some(COLLAR))],
        (Look::Master, 1) => vec![
            Op::Grey { from: BLUE_HUE, width: BLUE_WIDTH, min_v: 0.0, lo: BLACK_LO, hi: BLACK_HI, rect: Some(BELT) },
            paint_yellow(BLUE_HUE, BLUE_WIDTH, Some(HOOD_LINING)),
            paint_yellow(RED, RED_WIDTH, None),
        ],
        // Anti (the user: "make sora look like shadow sora in this form (the miniboss from
        // neverland)"): all of him black, the face's eye frames too (else KH1 would paste
        // Sora's blue eyes onto the black face).
        (Look::Anti, _) => {
            let eyes = match TEXTURES[k].1 {
                "-1c0" => Some(Eyes::Face(kh1_data::texmap::SORA_FACE_EYES)),
                "-2c0" => Some(Eyes::Frames),
                _ => None,
            };
            vec![Op::Shadow { to: SHADOW_HUE, sat: SHADOW_SAT, lo: SHADOW_LO, hi: SHADOW_HI, eyes: eyes.filter(|_| ANTI_EYES) }]
        }
        _ => Vec::new(),
    }
}

static BASE: AtomicUsize = AtomicUsize::new(0);

/// A texture: width, height, B8G8R8A8 rows.
type Pixels = kh1_data::texmap::Image;

/// The textures' original pixels from the PKG (`None`: not read).
static FROM_PKG: [OnceLock<Option<Arc<Pixels>>>; NTEX] = [const { OnceLock::new() }; NTEX];

/// Anti-Sora's textures on Sora's layout (per `TEXTURES`; `None`: that one not made), made at
/// start; `Err`: why not (Anti Form then falls back to `Op::Shadow`).
type Baked = [Option<Arc<Pixels>>; NTEX];
static ANTI: OnceLock<Result<Baked, String>> = OnceLock::new();

struct Swap {
    /// Per texture: the source texture swapped (its address), KH1's GPU texture to put back,
    /// and the look it wears.
    swapped: [Option<(usize, u64, Look)>; NTEX],
    /// Our GPU texture for a source texture and a look (made once each).
    ours: Option<HashMap<(usize, Look), u64>>,
    /// Those we couldn't make (not tried again).
    failed: Vec<(usize, Look)>,
}

static SWAP: Mutex<Swap> = Mutex::new(Swap { swapped: [None; NTEX], ours: None, failed: Vec::new() });

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let c = mx - mn;
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

fn rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let hh = h.rem_euclid(360.0) / 60.0;
    let i = hh.floor() as i32 % 6;
    let f = hh - hh.floor();
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

/// `op` on `p`'s pixels, in place. Returns how many changed.
fn apply(p: &mut Pixels, op: Op) -> usize {
    let (w, h) = (p.w as usize, p.h as usize);
    let rect = match op {
        Op::Shift { rect, .. } => rect,
        Op::Colourize { rect, .. } => Some(rect),
        Op::Grey { rect, .. } => rect,
        Op::Paint { rect, .. } => rect,
        Op::Shadow { .. } | Op::Darken { .. } => None,
    };
    let within = match op {
        Op::Darken { mask: m, .. } => match mask(m, p.w, p.h) {
            Some(v) => Some(v),
            None => {
                log(&format!("form tint: {m:?} not found on the mesh, left as it is"));
                return 0;
            }
        },
        _ => None,
    };
    let [u0, v0, u1, v1] = rect.unwrap_or([0.0, 0.0, 1.0, 1.0]);
    let (x0, x1) = ((u0 * w as f32) as usize, ((u1 * w as f32).ceil() as usize).min(w));
    let (y0, y1) = ((v0 * h as f32) as usize, ((v1 * h as f32).ceil() as usize).min(h));
    // Sora's own eye drawing, for where Anti's solid eyes go.
    let eyes_from = match op {
        Op::Shadow { eyes: Some(e), .. } => Some((e, p.clone())),
        _ => None,
    };
    let mut changed = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = (y * w + x) * 4;
            let px = &mut p.bgra[i..i + 4];
            let (b, g, r) = (px[0] as f32 / 255.0, px[1] as f32 / 255.0, px[2] as f32 / 255.0);
            let (hue, s, v) = hsv(r, g, b);
            let (k, (nr, ng, nb)) = match op {
                Op::Shift { from, width, to, min_v, .. } => {
                    let dh = (hue - from).rem_euclid(360.0);
                    let d = dh.min(360.0 - dh);
                    let k = ((width - d) / RED_SOFT).clamp(0.0, 1.0) * ((s - SAT_FROM) / SAT_SOFT).clamp(0.0, 1.0) * if min_v > 0.0 { ((v - min_v) / 0.08 + 0.5).clamp(0.0, 1.0) } else { 1.0 };
                    (k, rgb(to, s, v))
                }
                Op::Grey { from, width, min_v, lo, hi, .. } => {
                    let dh = (hue - from).rem_euclid(360.0);
                    let d = dh.min(360.0 - dh);
                    let k = ((width - d) / RED_SOFT).clamp(0.0, 1.0) * ((s - SAT_FROM) / SAT_SOFT).clamp(0.0, 1.0) * if min_v > 0.0 { ((v - min_v) / 0.08 + 0.5).clamp(0.0, 1.0) } else { 1.0 };
                    let l = lo + (hi - lo) * v;
                    (k, (l, l, l))
                }
                Op::Paint { from, width, to, sat, lo, hi, .. } => {
                    let dh = (hue - from).rem_euclid(360.0);
                    let d = dh.min(360.0 - dh);
                    let k = ((width - d) / RED_SOFT).clamp(0.0, 1.0) * ((s - SAT_FROM) / SAT_SOFT).clamp(0.0, 1.0);
                    (k, rgb(to, sat, lo + (hi - lo) * v))
                }
                Op::Colourize { to, sat, .. } => {
                    // Light and grey: not the black trim, not already coloured.
                    let k = ((0.3 - s) / 0.1).clamp(0.0, 1.0) * ((v - 0.3) / 0.15).clamp(0.0, 1.0);
                    // (A little darker: the cream is near white, and a pale tint read as washed out.)
                    (k, rgb(to, sat, v * 0.85))
                }
                Op::Shadow { to, sat, lo, hi, .. } => (1.0, rgb(to, sat, lo + (hi - lo) * v)),
                Op::Darken { lo, hi, .. } => {
                    let k = if within.as_ref().is_some_and(|m| m[y * w + x]) { 1.0 } else { 0.0 };
                    let l = (lo + (hi - lo) * v).min(v);
                    (k, (l, l, l))
                }
            };
            if k <= 0.0 {
                continue;
            }
            changed += 1;
            let mix = |a: f32, z: f32| ((a * (1.0 - k) + z * k) * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
            px[0] = mix(b, nb);
            px[1] = mix(g, ng);
            px[2] = mix(r, nr);
        }
    }
    if let Some((eyes, orig)) = eyes_from {
        solid_eyes(p, &orig, eyes);
    }
    changed
}

/// Anti's solid yellow eyes on `p` where Sora's are in `orig` (the same size; `ANTI_EYES`).
/// Returns how many eyes (on the face) or open frames (on the sheet) got them.
fn solid_eyes(p: &mut Pixels, orig: &Pixels, eyes: Eyes) -> usize {
    use kh1_data::texmap;
    match eyes {
        Eyes::Face([u0, v0, u1, v1]) => {
            let (w, h) = (orig.w as f32, orig.h as f32);
            let area = [(u0 * w) as u32, (v0 * h) as u32, ((u1 - u0) * w).ceil() as u32, ((v1 - v0) * h).ceil() as u32];
            texmap::solid_eyes(p, orig, area, texmap::ANTI_EYE_RGB)
        }
        Eyes::Frames => texmap::solid_eye_frames(p, orig, FRAME_H, texmap::ANTI_EYE_RGB),
    }
}

/// Anti-Sora's textures carried onto Sora's (`kh1_data::texmap`; `docs/kh1-tint.md`): Sora's
/// mesh-table textures recoloured texel by texel from the point of Anti-Sora's surface they lie
/// on (the same geometry), and the face's frame sheets made of the face's new pixels where KH1
/// pastes them. Needs Sora's own textures from the PKG (`FROM_PKG`) first.
fn bake_anti(files: &kh1_data::Files) -> Result<Baked, String> {
    use kh1_data::texmap;
    let started = std::time::Instant::now();
    let mesh = |model: &str| -> Result<Vec<texmap::Tri>, String> {
        let bind = texmap::bind_pose(&files.read(model).map_err(|e| format!("{model}: {e}"))?).map_err(|e| format!("{model}: {e}"))?;
        let (_, cvbl) = files.read_remastered(model, "-c0").map_err(|e| format!("{model} -c0.cvbl: {e}"))?;
        texmap::cvbl_triangles(&cvbl, &bind).map_err(|e| format!("{model}: {e}"))
    };
    let sora = mesh(MODEL)?;
    let anti = mesh(ANTI_MODEL)?;
    let textures: Vec<Option<Pixels>> = (0..ANTI_SLOTS).map(|s| files.read_remastered(ANTI_MODEL, &format!("-1c{s}")).ok().and_then(|(_, b)| Pixels::from_dds(&b))).collect();
    let missing: Vec<u32> = (0..ANTI_SLOTS).filter(|&s| textures[s as usize].is_none() && anti.iter().any(|t| t.slot == s)).collect();
    if !missing.is_empty() {
        return Err(format!("{ANTI_MODEL}: textures {missing:?} not read"));
    }
    let src = texmap::Source::new(&anti, &textures);
    let mut out: Baked = Default::default();
    let orig = |k: usize| FROM_PKG[k].get().cloned().flatten();
    // Sora's mesh-table textures, by their slot (`-1c<slot>`).
    for (k, (tex, asset)) in TEXTURES.iter().enumerate() {
        let (Tex::Mesh(_), Some(slot)) = (tex, asset.strip_prefix("-1c").and_then(|s| u32::from_str_radix(s, 16).ok())) else { continue };
        let Some(base) = orig(k) else {
            log(&format!("form tint (Anti): Sora's {asset} wasn't read from the PKG, so Anti-Sora's isn't carried onto it"));
            continue;
        };
        let (img, st) = texmap::transfer(&sora, slot, &base, &src);
        if st.triangles == 0 {
            log(&format!("form tint (Anti): no triangle of Sora's on {asset}"));
            continue;
        }
        log(&format!(
            "form tint (Anti): Anti-Sora's textures onto Sora's {asset} ({}x{}): {} triangles, {} texels covered, {} filled beside, gap to his surface {:.4} mean, {:.3} max",
            img.w, img.h, st.triangles, st.covered, st.filled, st.mean_gap, st.max_gap
        ));
        out[k] = Some(Arc::new(img));
    }
    // The eye and mouth frames: the new face where each is pasted (Anti-Sora has neither).
    let face = TEXTURES.iter().position(|t| t.1 == "-1c0");
    for (k, (tex, asset)) in TEXTURES.iter().enumerate() {
        let Tex::Frames(_) = tex else { continue };
        let (Some(sheet), Some(f)) = (orig(k), face) else { continue };
        let (Some(face_orig), Some(face_new)) = (orig(f), out[f].clone()) else { continue };
        match texmap::find_frame(&face_orig, &sheet, FRAME_H) {
            Some((x, y, d)) if d <= FRAME_MATCH => {
                let mut img = texmap::frames_from(&face_new, x, y, &sheet, FRAME_H);
                let open = if ANTI_EYES && *asset == "-2c0" { solid_eyes(&mut img, &sheet, Eyes::Frames) } else { 0 };
                log(&format!(
                    "form tint (Anti): {asset} frames made from the face at ({x}, {y}) (difference {d:.1}), solid eyes on {open} of {} frames",
                    sheet.h / FRAME_H
                ));
                out[k] = Some(Arc::new(img));
            }
            found => log(&format!("form tint (Anti): {asset}'s first frame not found on the face ({found:?})")),
        }
    }
    // The face's own eyes too (after the frames, which take the face without them).
    if let (true, Some(f)) = (ANTI_EYES, face) {
        if let (Some(mut img), Some(face_orig)) = (out[f].take(), orig(f)) {
            solid_eyes(Arc::make_mut(&mut img), &face_orig, Eyes::Face(texmap::SORA_FACE_EYES));
            out[f] = Some(img);
        }
    }
    if out.iter().all(|t| t.is_none()) {
        return Err("none of Sora's textures could take Anti-Sora's".into());
    }
    log(&format!("form tint (Anti): Anti-Sora's textures ready in {:.1} s", started.elapsed().as_secs_f32()));
    Ok(out)
}

fn recoloured(p: &Pixels, what: &str, k: usize, look: Look) -> Arc<Pixels> {
    let mut p = Pixels { w: p.w, h: p.h, bgra: p.bgra.clone() };
    let counts: Vec<usize> = ops(look, k).into_iter().map(|op| apply(&mut p, op)).collect();
    log(&format!("form tint ({look:?}): {what}, {}x{}, texels recoloured per step {counts:?}", p.w, p.h));
    Arc::new(p)
}

/// At start: the HD textures from the PKG, recoloured (for when KH1 no longer keeps the file in
/// memory).
pub fn install() {
    let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    BASE.store(base, Ordering::Relaxed);
    if !super::settings::get().drive {
        return;
    }
    std::thread::spawn(|| {
        let install = match kh1_data::Kh1Install::locate(super::exe_dir()) {
            Ok(i) => i,
            Err(e) => {
                log(&format!("Wisdom tint: KH1's files not found ({e})"));
                let _ = UV_TRIS.set(None);
                let _ = ANTI.set(Err(format!("KH1's files not found ({e})")));
                return;
            }
        };
        // The mesh's triangles on its textures (for the parts found by the mesh, `Mask`).
        let tris = install.read_remastered(kh1_data::Source::Live, MODEL, "-c0.cvbl").map_err(|e| e.to_string()).and_then(|(_, b)| kh1_data::texmap::cvbl_uv_triangles(&b));
        match &tris {
            Ok(t) => log(&format!("form tint: {MODEL} -c0.cvbl read from the PKG ({} triangles)", t.len())),
            Err(e) => log(&format!("form tint: {MODEL} -c0.cvbl not read from the PKG ({e}); Limit Form's jacket keeps its colours")),
        }
        let _ = UV_TRIS.set(tris.ok());
        for (k, (_, asset)) in TEXTURES.iter().enumerate() {
            let asset = format!("{asset}.dds");
            let p = match install.read_remastered(kh1_data::Source::Live, MODEL, &asset) {
                Ok((name, bytes)) => match Pixels::from_dds(&bytes) {
                    Some(p) => {
                        log(&format!("form tint: {MODEL} {name} read from the PKG ({}x{})", p.w, p.h));
                        Some(Arc::new(p))
                    }
                    None => {
                        log(&format!("Wisdom tint: {name} isn't a plain 32-bit DDS"));
                        None
                    }
                },
                Err(e) => {
                    log(&format!("Wisdom tint: {MODEL} {asset} not read from the PKG ({e})"));
                    None
                }
            };
            let _ = FROM_PKG[k].set(p);
        }
        let anti = install.files(kh1_data::Source::Live).map_err(|e| format!("KH1's packages: {e}")).and_then(|f| bake_anti(&f));
        if let Err(e) = &anti {
            log(&format!("form tint (Anti): Anti-Sora's textures not made, Anti Form darkens Sora's instead ({e})"));
        }
        let _ = ANTI.set(anti);
    });
}

/// Sora's model's source texture for `TEXTURES[k]` (`docs/kh1-tint.md`), if it is an HD
/// texture.
unsafe fn source_texture(actor: usize, k: usize) -> Option<usize> {
    let p = remastered(actor)?;
    match TEXTURES[k].0 {
        Tex::Mesh(entry) => mesh_texture(p, entry),
        Tex::Frames(tag) => frames_texture(p, tag),
    }
}

/// The remastered model data behind Sora's model header `+0x14` (its mesh table at `+0`, its
/// texture-animation patches at `+0x20` / `+0x28`).
unsafe fn remastered(actor: usize) -> Option<usize> {
    let inst = pose::ptr(rd::<u32>(actor + 0x15C));
    if inst < 0x10000 || !super::crash::readable(inst + 8) {
        return None;
    }
    let slot = rd::<u64>(inst) as usize;
    if slot < 0x10000 || !super::crash::readable(slot + 8) {
        return None;
    }
    let header = rd::<u64>(slot) as usize;
    if header < 0x10000 || !super::crash::readable(header + 0x20) || rd::<u32>(header + 0x10) != 0x9696_9696 {
        return None;
    }
    let p = pose::ptr(rd::<u32>(header + 0x14));
    (p > 0x10000 && super::crash::readable(p + 8)).then_some(p)
}

/// Mesh-table entry `entry`'s source texture.
unsafe fn mesh_texture(p: usize, entry: usize) -> Option<usize> {
    // The remastered table at `+0`: its count at `+4`, entries after a skip given by `+8`.
    let t = rd::<u64>(p) as usize;
    if t < 0x10000 || !super::crash::readable(t + 0x10) {
        return None;
    }
    let w = rd::<u32>(t + 8);
    let skip = if w & 0x1_0000 != 0 { (w & 0xFFFF) as usize } else { 0 };
    let count = rd::<u32>(t + 4) as usize;
    if entry >= count || count > 32 {
        return None;
    }
    let e = t + 0x10 * (1 + skip * 2 + entry);
    if !super::crash::readable(e + 0x10) {
        return None;
    }
    // Entry `+4` → texture descriptor X; X `+8` → the source `Axa::EntryTex`.
    let x = pose::ptr(rd::<u32>(e + 4));
    if x < 0x10000 || !super::crash::readable(x + 0x10) {
        return None;
    }
    let src = pose::ptr(rd::<u32>(x + 8));
    if src < 0x10000 || !super::crash::readable(src + 0x98) || rd::<u32>(src + 0x10) != TEX_MAGIC {
        return None;
    }
    Some(src)
}

/// The texture-animation frame sheet named `…<tag>…` (`mdls2c0.`: the eyes): the model's patches
/// (`+0x20` count, `+0x28` 32-byte records; KH1's `0x1401DB370`), each naming the `EntryTex` it
/// pastes from at `+0x18`.
unsafe fn frames_texture(p: usize, tag: &[u8]) -> Option<usize> {
    if !super::crash::readable(p + 0x30) {
        return None;
    }
    let n = rd::<u32>(p + 0x20) as usize;
    let patches = rd::<u64>(p + 0x28) as usize;
    if n == 0 || n > 64 || patches < 0x10000 || !super::crash::readable(patches) || !super::crash::readable(patches + 0x20 * n - 1) {
        return None;
    }
    (0..n).map(|i| rd::<u64>(patches + 0x20 * i + 0x18) as usize).find(|&t| {
        t >= 0x10000 && super::crash::readable(t + 0x98) && rd::<u32>(t + 0x10) == TEX_MAGIC && rd::<[u8; 32]>(t + 0x14).windows(tag.len()).any(|w| w == tag)
    })
}

#[repr(C)]
struct Desc {
    dimension: u32,
    alignment: u64,
    width: u64,
    height: u32,
    depth: u16,
    mips: u16,
    format: u32,
    samples: u32,
    quality: u32,
    layout: u32,
    flags: u32,
}

/// A GPU texture made by KH1's own creator from `w × h` BGRA pixels (`kh2hud`'s KH2 HUD image),
/// on the game thread. `None` if KH1 made none.
pub(super) unsafe fn make_texture(w: u32, h: u32, bgra: &[u8], label: &'static [u8]) -> Option<u64> {
    if bgra.len() < (w * h * 4) as usize || label.last() != Some(&0) {
        return None;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let desc = Desc { dimension: 3, alignment: 0, width: w as u64, height: h, depth: 1, mips: 1, format: 0x57, samples: 1, quality: 0, layout: 0, flags: 0 };
    let state: unsafe extern "system" fn() -> usize = std::mem::transmute(base + STATE_RVA);
    let create: unsafe extern "system" fn(usize, *const Desc, *const u8, *const u8, u32) -> u64 = std::mem::transmute(base + CREATE_RVA);
    let r = state();
    if r < 0x10000 {
        return None;
    }
    let t = create(r, &desc, bgra.as_ptr(), label.as_ptr(), 0);
    (t >= 0x10000).then_some(t)
}

/// Our recoloured GPU texture for `src` in `look` (made on first use).
unsafe fn ours_for(s: &mut Swap, k: usize, src: usize, look: Look) -> Option<u64> {
    if let Some(&t) = s.ours.as_ref().and_then(|o| o.get(&(src, look))) {
        return Some(t);
    }
    if s.failed.contains(&(src, look)) {
        return None;
    }
    let (w, h) = (rd::<u16>(src + TEX_W) as u32, rd::<u16>(src + TEX_H) as u32);
    let name: String = (0..32).map(|i| rd::<u8>(src + 0x14 + i)).take_while(|&b| b != 0).map(|b| b as char).collect();
    // Anti: Anti-Sora's own, once made (at start; until then, nothing swapped yet), else the
    // Shadow recolour below.
    let baked = if look == Look::Anti {
        match ANTI.get() {
            None => return None,
            Some(Ok(t)) => {
                let p = t[k].clone().filter(|p| p.w == w && p.h == h);
                if p.is_none() {
                    log(&format!("form tint (Anti): no Anti-Sora texture for {name} ({w}x{h}), darkened instead"));
                }
                p
            }
            Some(Err(_)) => None,
        }
    } else {
        None
    };
    // A part found by the mesh: not until the mesh has been read (at start).
    if UV_TRIS.get().is_none() && ops(look, k).iter().any(|op| matches!(op, Op::Darken { .. })) {
        return None;
    }
    // The pixels: the DDS KH1 keeps (the same file), else the PKG's.
    let dds = rd::<u64>(src + TEX_DDS) as usize;
    let mem = if baked.is_some() {
        None
    } else if dds > 0x10000 && super::crash::readable(dds + 128) && rd::<u32>(dds) == 0x2053_4444 {
        let (dh, dw) = (rd::<u32>(dds + 12), rd::<u32>(dds + 16));
        let len = 128 + (dw as usize) * (dh as usize) * 4;
        (dw == w && dh == h && super::crash::readable(dds + len - 1)).then(|| Pixels::from_dds(std::slice::from_raw_parts(dds as *const u8, len))).flatten().map(|p| recoloured(&p, &format!("{name} (KH1's copy in memory)"), k, look))
    } else {
        None
    };
    let pixels = baked.or(mem).or_else(|| FROM_PKG[k].get().cloned().flatten().filter(|p| p.w == w && p.h == h).map(|p| recoloured(&p, &format!("{name} (the PKG's)"), k, look)));
    let Some(p) = pixels else {
        if FROM_PKG[k].get().is_some() {
            log(&format!("form tint: no pixels for {name} ({w}x{h}; neither KH1's copy nor the PKG's fits)"));
            s.failed.push((src, look));
        }
        return None;
    };
    let desc = Desc { dimension: 3, alignment: 0, width: w as u64, height: h, depth: 1, mips: 1, format: 0x57, samples: 1, quality: 0, layout: 0, flags: 0 };
    let base = BASE.load(Ordering::Relaxed);
    let state: unsafe extern "system" fn() -> usize = std::mem::transmute(base + STATE_RVA);
    let create: unsafe extern "system" fn(usize, *const Desc, *const u8, *const u8, u32) -> u64 = std::mem::transmute(base + CREATE_RVA);
    let r = state();
    if r < 0x10000 {
        return None;
    }
    let label: &[u8] = match look {
        Look::Wisdom => b"kh2combat_wisdom_blue\0",
        Look::Valor => b"kh2combat_valor_red\0",
        Look::Final => b"kh2combat_final_white\0",
        Look::Limit => b"kh2combat_limit_black\0",
        Look::Master => b"kh2combat_master_yellow\0",
        Look::Anti => b"kh2combat_anti_shadow\0",
    };
    let t = create(r, &desc, p.bgra.as_ptr(), label.as_ptr(), 0);
    if t < 0x10000 {
        log(&format!("form tint: KH1 made no texture for {name} ({t:#x})"));
        s.failed.push((src, look));
        return None;
    }
    log(&format!("form tint ({look:?}): {name} ready ({w}x{h}, texture {t:#x}; KH1's {:#x})", rd::<u64>(src + TEX_GPU)));
    s.ours.get_or_insert_with(HashMap::new).insert((src, look), t);
    Some(t)
}

/// Each KH2 frame (`form`, the game thread): in a form with a look, Sora's textures swapped for
/// its recoloured copies; out of it (revert, knocked out, another form), KH1's back. A model
/// loaded again (an area change) is a new source: ours for the old one are left to KH1's freeing.
pub unsafe fn refresh(actor: usize, look: Option<Look>) {
    if BASE.load(Ordering::Relaxed) == 0 || !super::settings::get().drive {
        return;
    }
    let Ok(mut s) = SWAP.try_lock() else { return };
    for k in 0..NTEX {
        let want = look.filter(|l| !ops(*l, k).is_empty());
        // (Not one this look swaps, nor swapped: nothing to look up.)
        if want.is_none() && s.swapped[k].is_none() {
            continue;
        }
        let src = source_texture(actor, k);
        if let Some((was, orig, l)) = s.swapped[k] {
            if want != Some(l) || src != Some(was) {
                let ours = s.ours.as_ref().and_then(|o| o.get(&(was, l))).copied().unwrap_or(0);
                if src == Some(was) && rd::<u64>(was + TEX_GPU) == ours {
                    wr(was + TEX_GPU, orig);
                } else if src != Some(was) {
                    if let Some(o) = s.ours.as_mut() {
                        o.retain(|(t, _), _| *t != was);
                    }
                }
                s.swapped[k] = None;
            }
        }
        if let (Some(l), None, Some(src)) = (want, s.swapped[k], src) {
            if let Some(t) = ours_for(&mut s, k, src, l) {
                let orig = rd::<u64>(src + TEX_GPU);
                wr(src + TEX_GPU, t);
                s.swapped[k] = Some((src, orig, l));
            }
        }
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if look.is_some() && s.swapped.iter().all(|x| x.is_none()) && N.fetch_add(1, Ordering::Relaxed) == 600 {
        log("form tint: still nothing swapped after 10 s in the form");
    }
}
