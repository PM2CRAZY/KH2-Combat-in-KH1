//! Sora's clothes recoloured in a Drive Form (the user: Wisdom Form, "recolor KH1 Sora blue";
//! s233: Wisdom's sleeves blue too, and Valor red: "his shoes red, his sleeves red, his belt red,
//! the inside part of his hoodie red"). Each look recolours its parts of the textures (`ops`):
//! a hue shift (red or yellow or blue to another hue) or a colourize (light grey and cream given
//! a hue), some only within a region of the texture (`SLEEVES`, `BELT`, `HOOD_LINING`).
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
//! (the model's remastered assets, `Kh1Install::read_remastered`) on a thread the first time a
//! form needs them, and dropped once its texture is made (the tester, ram-use m1 answer 2 (a)).
//! Each recoloured texture is made once per slot and look and kept for later sources (a model
//! loaded again), not made again for each.

use super::addr::Rva;
use super::{log, pose};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

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
/// The slots recoloured (mesh table entries 0 and 1: `-1c0`, the collar; `-1c1`, the clothes;
/// 3: `-1c3`, the shoes (the user: "soras shoes should turn from yellow to blue in this form";
/// every texel of it is on a leg or foot bone, `docs/kh1-tint.md`).
const SLOTS: [usize; 3] = [0, 1, 3];
const YELLOW_HUE: f32 = 48.0;
/// (Wide enough for the orange where the yellow shades off; only bright texels, so the dark
/// brown beside the shoes stays.)
const YELLOW_WIDTH: f32 = 26.0;
const YELLOW_MIN_V: f32 = 0.42;
/// The blue of the belt and of the hood's lining (hue, half width).
const BLUE_HUE: f32 = 225.0;
const BLUE_WIDTH: f32 = 22.0;
const MODEL: &str = "xa_ex_0010.mdls";
/// Hue shift: how wide the colour moved is (degrees, soft edge after), the saturation it takes
/// from (soft edge after).
const RED_WIDTH: f32 = 12.0;
const RED_SOFT: f32 = 6.0;
const SAT_FROM: f32 = 0.55;
const SAT_SOFT: f32 = 0.15;
const BLUE: f32 = 225.0;
const RED: f32 = 0.0;

/// Regions of `-1c1` (u0, v0, u1, v1; texture fractions), found by plotting the remastered mesh
/// 1's vertices by bone on the texture (`docs/kh1-tint.md`): the jacket's sleeves (the cream
/// strip both arms' bones use; the zipper rail beside it below 0.62 left out), the belt (the
/// blue band of the top strip, meshes 4..8) and the hood's lining (the blue band of the hood).
const SLEEVES: [[f32; 4]; 2] = [[0.035, 0.415, 0.236, 0.85], [0.236, 0.415, 0.215 + 0.025, 0.62]];
const BELT: [f32; 4] = [0.035, 0.07, 0.57, 0.14];
const HOOD_LINING: [f32; 4] = [0.755, 0.72, 1.0, 1.0];

/// A form's look (`form`): Wisdom's blue, Valor's red.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Look {
    Wisdom,
    Valor,
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
}

/// What each look does to each slot (`SLOTS` order); none: that slot isn't swapped.
fn ops(look: Look, k: usize) -> Vec<Op> {
    let shift = |from, width, to| Op::Shift { from, width, to, min_v: 0.0, rect: None };
    let shoes = |to| Op::Shift { from: YELLOW_HUE, width: YELLOW_WIDTH, to, min_v: YELLOW_MIN_V, rect: None };
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
        _ => Vec::new(),
    }
}

static BASE: AtomicUsize = AtomicUsize::new(0);

/// A recoloured texture: width, height, B8G8R8A8 rows.
struct Pixels {
    w: u32,
    h: u32,
    bgra: Vec<u8>,
}

/// A slot's original pixels from the PKG: read only when a form needs them and KH1's own copy in
/// memory doesn't fit (`pkg_pixels`), and dropped once used (about 4 MB a 1024x1024 slot).
enum FromPkg {
    Idle,
    Reading,
    Ready(Pixels),
    Failed,
}

static FROM_PKG: Mutex<[FromPkg; 3]> = Mutex::new([FromPkg::Idle, FromPkg::Idle, FromPkg::Idle]);

/// Slot `k`'s PKG pixels if read (taken: the caller keeps them only as long as it needs them);
/// else a read started on a thread (`None` meanwhile; the caller asks again next frame). `Err`:
/// they can't be read.
fn pkg_pixels(k: usize) -> Result<Option<Pixels>, ()> {
    let Ok(mut g) = FROM_PKG.lock() else { return Ok(None) };
    match std::mem::replace(&mut g[k], FromPkg::Idle) {
        FromPkg::Ready(p) => Ok(Some(p)),
        FromPkg::Failed => {
            g[k] = FromPkg::Failed;
            Err(())
        }
        FromPkg::Reading => {
            g[k] = FromPkg::Reading;
            Ok(None)
        }
        FromPkg::Idle => {
            g[k] = FromPkg::Reading;
            drop(g);
            std::thread::spawn(move || {
                let r = read_pkg(k);
                if let Ok(mut g) = FROM_PKG.lock() {
                    g[k] = match r {
                        Some(p) => FromPkg::Ready(p),
                        None => FromPkg::Failed,
                    };
                }
            });
            Ok(None)
        }
    }
}

/// Slot `k`'s HD texture from the PKG (the loader thread).
fn read_pkg(k: usize) -> Option<Pixels> {
    let install = match kh1_data::Kh1Install::locate(super::exe_dir()) {
        Ok(i) => i,
        Err(e) => {
            log(&format!("form tint: KH1's files not found ({e})"));
            return None;
        }
    };
    let asset = format!("-1c{}.dds", SLOTS[k]);
    match install.read_remastered(kh1_data::Source::Live, MODEL, &asset) {
        Ok((name, bytes)) => match dds_bgra(&bytes) {
            Some(p) => {
                log(&format!("form tint: {MODEL} {name} read from the PKG ({}x{}), on first need", p.w, p.h));
                Some(p)
            }
            None => {
                log(&format!("form tint: {name} isn't a plain 32-bit DDS"));
                None
            }
        },
        Err(e) => {
            log(&format!("form tint: {MODEL} {asset} not read from the PKG ({e})"));
            None
        }
    }
}

/// One of our recoloured GPU textures: the source it was made from (name, size), and the texture.
struct Made {
    name: String,
    w: u32,
    h: u32,
    texture: u64,
}

struct Swap {
    /// Per slot: the source texture swapped (its address), KH1's GPU texture to put back, and
    /// the look it wears.
    swapped: [Option<(usize, u64, Look)>; 3],
    /// Our GPU texture for a slot and a look, made once and used again for every later source of
    /// the same name and size (Sora's model loaded again by an area change: the same file).
    /// Before, it was made per source address, so each area change made new ones and the old were
    /// never released.
    ours: Option<HashMap<(usize, Look), Made>>,
    /// Those we couldn't make (not tried again): slot, look.
    failed: Vec<(usize, Look)>,
}

static SWAP: Mutex<Swap> = Mutex::new(Swap { swapped: [None, None, None], ours: None, failed: Vec::new() });

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
    };
    let [u0, v0, u1, v1] = rect.unwrap_or([0.0, 0.0, 1.0, 1.0]);
    let (x0, x1) = ((u0 * w as f32) as usize, ((u1 * w as f32).ceil() as usize).min(w));
    let (y0, y1) = ((v0 * h as f32) as usize, ((v1 * h as f32).ceil() as usize).min(h));
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
                Op::Colourize { to, sat, .. } => {
                    // Light and grey: not the black trim, not already coloured.
                    let k = ((0.3 - s) / 0.1).clamp(0.0, 1.0) * ((v - 0.3) / 0.15).clamp(0.0, 1.0);
                    // (A little darker: the cream is near white, and a pale tint read as washed out.)
                    (k, rgb(to, sat, v * 0.85))
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
    changed
}

/// An uncompressed 32-bit DDS's pixels as B8G8R8A8 (whatever its masks), with its size.
fn dds_bgra(dds: &[u8]) -> Option<Pixels> {
    if dds.len() < 128 || &dds[..4] != b"DDS " {
        return None;
    }
    let u = |o: usize| u32::from_le_bytes(dds[o..o + 4].try_into().unwrap());
    let (h, w) = (u(12), u(16));
    let (flags, bits) = (u(80), u(88));
    let masks = [u(92), u(96), u(100), u(104)];
    // RGB (0x40), 32 bits, no four-cc (0x4).
    if flags & 0x40 == 0 || flags & 0x4 != 0 || bits != 32 || w == 0 || h == 0 {
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
    Some(Pixels { w, h, bgra })
}

/// `p` recoloured for `look` (in place: the pixels are this one texture's, dropped after).
fn recoloured(mut p: Pixels, what: &str, k: usize, look: Look) -> Pixels {
    let counts: Vec<usize> = ops(look, k).into_iter().map(|op| apply(&mut p, op)).collect();
    log(&format!("form tint ({look:?}): {what}, {}x{}, texels recoloured per step {counts:?}", p.w, p.h));
    p
}

/// At start: only where the game is. Nothing is read now: the PKG's pixels are read the first
/// time a form needs them (`pkg_pixels`), when KH1's own copy doesn't fit.
pub fn install() {
    let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    BASE.store(base, Ordering::Relaxed);
}

/// Sora's model's source texture for mesh-table entry `entry` (`docs/kh1-tint.md`), if it is an
/// HD texture.
unsafe fn source_texture(actor: usize, entry: usize) -> Option<usize> {
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
    // The remastered table behind `+0x14`: its count at `+4`, entries after a skip given by `+8`.
    let p = pose::ptr(rd::<u32>(header + 0x14));
    let t = if p > 0x10000 && super::crash::readable(p + 8) { rd::<u64>(p) as usize } else { 0 };
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

/// Our recoloured GPU texture for slot `k`'s `src` in `look` (made on first use; the one made
/// before for this slot and look when `src` is the same texture, by name and size).
unsafe fn ours_for(s: &mut Swap, k: usize, src: usize, look: Look) -> Option<u64> {
    let (w, h) = (rd::<u16>(src + TEX_W) as u32, rd::<u16>(src + TEX_H) as u32);
    let name: String = (0..32).map(|i| rd::<u8>(src + 0x14 + i)).take_while(|&b| b != 0).map(|b| b as char).collect();
    let made = s.ours.as_ref().and_then(|o| o.get(&(k, look)));
    if let Some(m) = made.filter(|m| m.name == name && m.w == w && m.h == h) {
        return Some(m.texture);
    }
    if s.failed.contains(&(k, look)) {
        return None;
    }
    static OTHER: AtomicU32 = AtomicU32::new(0);
    if let Some(m) = made.filter(|_| OTHER.fetch_add(1, Ordering::Relaxed) < 10) {
        // (Another texture in this slot: not ours to reuse; ours made afresh.)
        log(&format!("form tint ({look:?}): slot {k} is now {name} ({w}x{h}), ours was made for {} ({}x{})", m.name, m.w, m.h));
    }
    // The pixels: the DDS KH1 keeps (the same file), else the PKG's.
    let dds = rd::<u64>(src + TEX_DDS) as usize;
    let mem = if dds > 0x10000 && super::crash::readable(dds + 128) && rd::<u32>(dds) == 0x2053_4444 {
        let (dh, dw) = (rd::<u32>(dds + 12), rd::<u32>(dds + 16));
        let len = 128 + (dw as usize) * (dh as usize) * 4;
        (dw == w && dh == h && super::crash::readable(dds + len - 1)).then(|| dds_bgra(std::slice::from_raw_parts(dds as *const u8, len))).flatten().map(|p| recoloured(p, &format!("{name} (KH1's copy in memory)"), k, look))
    } else {
        None
    };
    let pixels = match mem {
        Some(p) => Some(p),
        // (The PKG's, read on a thread now if not yet: nothing this frame, asked again next.)
        None => match pkg_pixels(k) {
            Ok(None) => return None,
            Ok(Some(p)) if p.w == w && p.h == h => Some(recoloured(p, &format!("{name} (the PKG's)"), k, look)),
            Ok(Some(_)) | Err(()) => None,
        },
    };
    let Some(p) = pixels else {
        log(&format!("form tint: no pixels for {name} ({w}x{h}; neither KH1's copy nor the PKG's fits)"));
        s.failed.push((k, look));
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
    };
    let t = create(r, &desc, p.bgra.as_ptr(), label.as_ptr(), 0);
    if t < 0x10000 {
        log(&format!("form tint: KH1 made no texture for {name} ({t:#x})"));
        s.failed.push((k, look));
        return None;
    }
    log(&format!("form tint ({look:?}): {name} ready ({w}x{h}, texture {t:#x}; KH1's {:#x}); its source pixels dropped", rd::<u64>(src + TEX_GPU)));
    // (A texture made before for this slot and look, for another texture now gone, isn't
    // released here: KH1's release for one isn't known (`docs/kh1-tint.md`).)
    s.ours.get_or_insert_with(HashMap::new).insert((k, look), Made { name, w, h, texture: t });
    Some(t)
}

/// Each KH2 frame (`form`, the game thread): in a form with a look, Sora's textures swapped for
/// its recoloured copies; out of it (revert, knocked out, another form), KH1's back. A model
/// loaded again (an area change) is a new source: ours are used again for it; only one that was
/// in the old source when it went is dropped (KH1 may have released it with that source).
pub unsafe fn refresh(actor: usize, look: Option<Look>) {
    if BASE.load(Ordering::Relaxed) == 0 || !super::settings::get().drive {
        return;
    }
    let Ok(mut s) = SWAP.try_lock() else { return };
    for (k, &entry) in SLOTS.iter().enumerate() {
        let src = source_texture(actor, entry);
        let want = look.filter(|l| !ops(*l, k).is_empty());
        if let Some((was, orig, l)) = s.swapped[k] {
            if want != Some(l) || src != Some(was) {
                let ours = s.ours.as_ref().and_then(|o| o.get(&(k, l))).map_or(0, |m| m.texture);
                if src == Some(was) && rd::<u64>(was + TEX_GPU) == ours {
                    wr(was + TEX_GPU, orig);
                } else if src != Some(was) {
                    // (The old source went while it held ours: KH1 may have released ours with
                    // it, so it isn't used again.)
                    if let Some(o) = s.ours.as_mut() {
                        o.remove(&(k, l));
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
