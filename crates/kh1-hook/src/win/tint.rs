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
//! The pixels: built once into the mod folder (`tintcache`, `kh1-kh2-cache\form-tint-<look>-1c<n>.bin`)
//! from the HD texture file KH1 keeps in memory (`+0x68`, a DDS), else from the PKG (the model's
//! remastered assets, `Kh1Install::read_remastered`), and loaded from there afterwards; all of it
//! on a thread the first time a form needs a slot and look, and dropped once its texture is made
//! (the tester, ram-use m1 answer 2 (a) and the assets built once). Each recoloured texture is
//! made once per slot and look and kept for later sources (a model loaded again), not made again
//! for each.
//!
//! Anti Form (the owner, main s272..s292: "make sora look like shadow sora in this form (the
//! miniboss from neverland)", then "copying the exact textures of KH1 anti-sora onto sora's anti
//! form model instead of replacing the model", and "the eyes ... solid yellow, the same yellow as
//! in anti-sora's model"): KH1's own Anti-Sora (`xa_pp_3020.mdls`, read from the player's
//! install) is Sora's PS2 mesh with his own textures, so his colours are carried onto Sora's HD
//! textures by the surface (`kh1_data::texmap`, `bake_anti`), the shorts' pockets (`-1c2`) and
//! the face's eye and mouth frame sheets (`-2c0`, `-2c1`, which KH1 pastes onto the face each
//! draw) too, with solid yellow eyes. Built once into the mod folder like the other looks, and
//! swapped in the same way. If it can't be made, Sora's textures are darkened instead
//! (`Op::Shadow`, main's fallback) and the log says why.

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
/// Where a swapped texture is found on Sora's model.
#[derive(Clone, Copy)]
enum Tex {
    /// A mesh-table entry's source texture.
    Mesh(usize),
    /// A texture-animation frame sheet, by the end of its name (`mdls2c0.`: the eyes): KH1
    /// pastes a frame of it onto the face's per-object copy each draw (`docs/kh1-tint.md`).
    Frames(&'static [u8]),
}

/// The textures that can be swapped, in the order `ops` numbers them (`k`), with their asset in
/// the PKG: mesh table entries 0 and 1 (`-1c0`, the collar; `-1c1`, the clothes), 3 (`-1c3`, the
/// shoes; the user: "soras shoes should turn from yellow to blue in this form"; every texel of it
/// is on a leg or foot bone, `docs/kh1-tint.md`), and for Anti also entry 2 (`-1c2`, the shorts'
/// pockets) and the face's eye and mouth frames (`-2c0`, `-2c1`).
const TEXTURES: [(Tex, &str); 6] = [
    (Tex::Mesh(0), "-1c0"),
    (Tex::Mesh(1), "-1c1"),
    (Tex::Mesh(3), "-1c3"),
    (Tex::Mesh(2), "-1c2"),
    (Tex::Frames(b"mdls2c0."), "-2c0"),
    (Tex::Frames(b"mdls2c1."), "-2c1"),
];
const NTEX: usize = TEXTURES.len();
/// KH1's Anti-Sora (the dark Sora of Neverland's ship): Sora's PS2 mesh on his own textures
/// (`docs/kh1-tint.md`). Main found him in `kh1_fifth` (Checked (main's docs)).
const ANTI_MODEL: &str = "xa_pp_3020.mdls";
/// A face frame sheet's frame height (Sora's eyes, `-2c0`: 320×128 frames; the mouth, `-2c1`;
/// main's reading of his files, Checked (main's docs)).
const FRAME_H: u32 = 128;
/// How far (mean per channel, 0..255) a sheet's first frame may differ from the face where it is
/// pasted and still count as found (main's value; Sora's about 6).
const FRAME_MATCH: f32 = 24.0;
/// How far (mean per channel) one of Anti-Sora's HD textures may be from his PS2 one and still
/// count as the same texture drawn larger (Sora's: 1.1, `kh1_data` tests); past it his PS2 one is
/// used.
const HD_MATCH: f32 = 12.0;
/// Anti's fallback (main's `Op::Shadow`): every texel near black at a blue-violet hue, its
/// brightness `lo + (hi - lo) × v`, so the folds still faintly show.
const SHADOW_HUE: f32 = 260.0;
const SHADOW_SAT: f32 = 0.5;
const SHADOW_LO: f32 = 0.03;
const SHADOW_HI: f32 = 0.13;
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
/// (t1029) The collar's red strip on `-1c0` (u0, v0, u1, v1): main's region (main `tint.rs`
/// `COLLAR`: the texture's one large red run, about 3460 texels at u 0..0.018, v 0.35..0.69).
/// `-1c0`'s other red, an oval on the shoe piece at u 0.40..0.49, v 0.63..0.70, is outside it, so
/// Master's shoes keep their colours. Assumption: main's plot, not plotted again here.
const COLLAR: [f32; 4] = [0.0, 0.34, 0.025, 0.70];
/// (t1029) Master's yellow (main's spec, main `tint.rs`: its `LIMIT_BELT_*` values, which main's
/// Master look paints with): hue 45, saturation 0.85, brightness from 0.55 up (`Op::Paint`), so a
/// dark red or blue comes out a bright yellow near the shoes', its shading kept. (Main's note: a
/// plain hue shift of the dark red and blue read olive.)
const MASTER_YELLOW_HUE: f32 = 45.0;
const MASTER_YELLOW_SAT: f32 = 0.85;
const MASTER_YELLOW_LO: f32 = 0.55;
/// (t1041) Final's white (main's spec, main `tint.rs` `WHITE_LO` / `WHITE_HI`): a recoloured
/// texel's brightness `0..1` mapped to `0.5..0.97` with no saturation (`Op::Grey`), so the folds
/// still show on the white.
const FINAL_WHITE_LO: f32 = 0.5;
const FINAL_WHITE_HI: f32 = 0.97;

/// A form's look (`form`): Wisdom's blue, Valor's red, Limit's black (t974), Master's yellow
/// (t1029), Final's white (t1041), Anti's KH1 Anti-Sora (testing-merge).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Look {
    Wisdom,
    Valor,
    /// Limit Form (the owner, round 12 answer 3 (b), main's s293 / s296: "black jumpsuit and
    /// jacket, black hood lining, yellow belt, black shoes"; KH2's own Limit Form is KH1 Sora's
    /// outfit, so this is the owner's look, not KH2's).
    Limit,
    /// Master Form (t1029; the owner, §24.2 answer 4: main's yellow spec, "jumpsuit, collar and
    /// hood lining yellow, belt black"; main's s3xx words: "I want sora's jumpsuit to be yellow,
    /// and the inside of sora's jacket's hood be yellow as well instead of blue. I want soras belt
    /// to be black"). KH2's own Master Form wears its own outfit (`P_EX100_TRIF`), so on KH1's
    /// model this is the owner's look, as Limit's.
    Master,
    /// Final Form (t1041; the owner's carried answer: main's white spec, "outfit white including
    /// the shoes, belt black; the jacket's black trim stays"; main's words: "make sora's outfit
    /// White, including his shoes. turn his belt black though"). KH2's own Final Form wears its
    /// own outfit (`P_EX100_ULTF`), so on KH1's model this is the owner's look, as Limit's and
    /// Master's.
    Final,
    /// Anti Form (main s290 / s292): KH1 Anti-Sora's textures carried onto Sora's, solid yellow
    /// eyes (`bake_anti`). KH2's own Anti Form is a model of its own (`P_EX100_HTLF`); on KH1's
    /// model this is the owner's look.
    Anti,
}

/// How dark a blackened texel is: its brightness scaled to this (its shading kept).
const BLACK_V: f32 = 0.2;

/// One recolouring.
#[derive(Clone, Copy)]
enum Op {
    /// Saturated texels near hue `from` (within `width`), at least `min_v` bright, to hue `to`,
    /// same saturation and brightness; within `rect` if given.
    Shift { from: f32, width: f32, to: f32, min_v: f32, rect: Option<[f32; 4]> },
    /// Light, unsaturated texels (white, cream, grey) given hue `to` at saturation `sat`, their
    /// brightness kept; within `rect`.
    Colourize { to: f32, sat: f32, rect: [f32; 4] },
    /// Texels near hue `(from, width)` (saturated, at least `min_v` bright), or with `None` the
    /// light unsaturated ones (as `Colourize` takes), made near black: no saturation, their
    /// brightness × `BLACK_V` (t974, Limit's look); within `rect` if given.
    Black { hue: Option<(f32, f32)>, min_v: f32, rect: Option<[f32; 4]> },
    /// (t1029, Master's look; main's op of the same name) Saturated texels near hue `from`
    /// (within `width`, as `Shift`) given hue `to` at saturation `sat`, brightness
    /// `lo + (hi - lo) × v` (a dark colour made a bright one, its shading kept); within `rect` if
    /// given.
    Paint { from: f32, width: f32, to: f32, sat: f32, lo: f32, hi: f32, rect: Option<[f32; 4]> },
    /// (t1041, Final's look; main's op of the same name) Saturated texels near hue `from` (within
    /// `width`, at least `min_v` bright, as `Shift`) made grey: no saturation, brightness
    /// `lo + (hi - lo) × v` (white with its shading kept); within `rect` if given.
    Grey { from: f32, width: f32, min_v: f32, lo: f32, hi: f32, rect: Option<[f32; 4]> },
    /// (Anti's fallback, main's op of the same name) Every texel near black (`SHADOW_*`); then
    /// Anti's solid eyes where Sora's are, if `eyes`.
    Shadow { eyes: Option<Eyes> },
}

/// Where an `Op::Shadow` puts Anti's solid eyes.
#[derive(Clone, Copy)]
enum Eyes {
    /// The face's eyes (`texmap::SORA_FACE_EYES`).
    Face,
    /// Every frame (`FRAME_H` rows) of the eye frame sheet (`-2c0`).
    Frames,
}

/// What each look does to each slot (`SLOTS` order); none: that slot isn't swapped.
fn ops(look: Look, k: usize) -> Vec<Op> {
    let shift = |from, width, to| Op::Shift { from, width, to, min_v: 0.0, rect: None };
    let shoes = |to| Op::Shift { from: YELLOW_HUE, width: YELLOW_WIDTH, to, min_v: YELLOW_MIN_V, rect: None };
    let paint_yellow = |from, width, rect| Op::Paint { from, width, to: MASTER_YELLOW_HUE, sat: MASTER_YELLOW_SAT, lo: MASTER_YELLOW_LO, hi: 1.0, rect };
    let white = |from, width, min_v| Op::Grey { from, width, min_v, lo: FINAL_WHITE_LO, hi: FINAL_WHITE_HI, rect: None };
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
        // Limit (the owner's black outfit): the collar's and the jumpsuit's reds black; the
        // jacket's sleeves (the light strip, `SLEEVES`) black; the hood's lining black; the belt's
        // blue yellow; the shoes' yellow black. Assumption: the jacket outside the sleeves' strip
        // keeps its colours (no mask of the jacket's other islands here).
        (Look::Limit, 0) => vec![Op::Black { hue: Some((RED, RED_WIDTH)), min_v: 0.0, rect: None }],
        (Look::Limit, 1) => {
            let mut v = vec![Op::Black { hue: Some((RED, RED_WIDTH)), min_v: 0.0, rect: None }];
            v.extend(SLEEVES.map(|rect| Op::Black { hue: None, min_v: 0.0, rect: Some(rect) }));
            v.push(Op::Black { hue: Some((BLUE_HUE, BLUE_WIDTH)), min_v: 0.0, rect: Some(HOOD_LINING) });
            v.push(Op::Shift { from: BLUE_HUE, width: BLUE_WIDTH, to: YELLOW_HUE, min_v: 0.0, rect: Some(BELT) });
            v
        }
        (Look::Limit, 2) => vec![Op::Black { hue: Some((YELLOW_HUE, YELLOW_WIDTH)), min_v: YELLOW_MIN_V, rect: None }],
        // Master (t1029, main's spec): the belt's blue black first (so nothing after touches
        // it; Limit's black, as Limit's hood lining); then the hood lining's blue and the reds
        // (the jumpsuit on `-1c1`, the collar's strip on `-1c0`) one bright yellow (`Op::Paint`).
        // The jacket, its blue trim outside the lining, the sleeves and the shoes stay.
        (Look::Master, 0) => vec![paint_yellow(RED, RED_WIDTH, Some(COLLAR))],
        (Look::Master, 1) => vec![
            Op::Black { hue: Some((BLUE_HUE, BLUE_WIDTH)), min_v: 0.0, rect: Some(BELT) },
            paint_yellow(BLUE_HUE, BLUE_WIDTH, Some(HOOD_LINING)),
            paint_yellow(RED, RED_WIDTH, None),
        ],
        // Final (t1041, main's spec, main `tint.rs` `Look::Final`): the belt's blue black first
        // (so the white pass leaves it; Limit's black, as Master's belt); then the reds (the
        // jumpsuit on `-1c1`, the collar and the shoe piece's red oval on `-1c0`), the blues (the
        // hood lining and the jacket's blue trim) and the shoes' yellow (with their red and blue)
        // white, their shading kept (`Op::Grey`). The jacket's black trim, its cream, the skin and
        // the rest stay (none is a saturated red, blue or bright yellow).
        (Look::Final, 0) => vec![white(RED, RED_WIDTH, 0.0)],
        (Look::Final, 1) => vec![
            Op::Black { hue: Some((BLUE_HUE, BLUE_WIDTH)), min_v: 0.0, rect: Some(BELT) },
            white(RED, RED_WIDTH, 0.0),
            white(BLUE_HUE, BLUE_WIDTH, 0.0),
        ],
        (Look::Final, 2) => vec![white(YELLOW_HUE, YELLOW_WIDTH, YELLOW_MIN_V), white(RED, RED_WIDTH, 0.0), white(BLUE_HUE, BLUE_WIDTH, 0.0)],
        // Anti: only when KH1 Anti-Sora's textures couldn't be carried over (`bake_anti`; the
        // log says why): all of him near black, with the solid eyes (main's fallback).
        (Look::Anti, _) => vec![Op::Shadow {
            eyes: match TEXTURES[k].1 {
                "-1c0" => Some(Eyes::Face),
                "-2c0" => Some(Eyes::Frames),
                _ => None,
            },
        }],
        _ => Vec::new(),
    }
}

/// Whether `look` swaps texture `k`.
fn wears(look: Look, k: usize) -> bool {
    !ops(look, k).is_empty()
}

static BASE: AtomicUsize = AtomicUsize::new(0);

/// A recoloured texture: width, height, B8G8R8A8 rows.
struct Pixels {
    w: u32,
    h: u32,
    bgra: Vec<u8>,
}

/// A recoloured texture for KH1's creator: width, height, B8G8R8A8 rows (the cache file's buffer,
/// or the recolour's own: one copy either way).
struct Ready {
    w: u32,
    h: u32,
    bgra: crate::slotcache::Block,
}

/// The work for one slot and look, on a thread (`start`): the cache file checked, else the source
/// recoloured and the file written. Its result is taken by the game thread (`ours_for`) to make the
/// GPU texture, then dropped.
enum Job {
    Working,
    Done(Ready),
    Failed,
}

static JOBS: Mutex<Vec<((usize, Look), Job)>> = Mutex::new(Vec::new());

/// Slot `k`'s job for `look`: `None` when none was started.
fn take_job(k: usize, look: Look) -> Option<Job> {
    let mut g = JOBS.lock().ok()?;
    let i = g.iter().position(|(key, _)| *key == (k, look))?;
    match &g[i].1 {
        Job::Working => Some(Job::Working),
        Job::Failed => Some(Job::Failed),
        Job::Done(_) => Some(g.swap_remove(i).1),
    }
}

fn set_job(k: usize, look: Look, j: Job) {
    if let Ok(mut g) = JOBS.lock() {
        match g.iter_mut().find(|(key, _)| *key == (k, look)) {
            Some(e) => e.1 = j,
            None => g.push(((k, look), j)),
        }
    }
}

fn look_name(look: Look) -> &'static str {
    match look {
        Look::Wisdom => "Wisdom",
        Look::Valor => "Valor",
        Look::Limit => "Limit",
        Look::Master => "Master",
        Look::Final => "Final",
        Look::Anti => "Anti",
    }
}

/// The cache's folder: beside the DLL (the mod's folder), `kh1-kh2-cache` (as the slot cache's).
fn cache_dir() -> std::path::PathBuf {
    std::path::Path::new(&super::dll_dir()).join("kh1-kh2-cache")
}

/// KH1's live PKG entry for Sora's model, as text (package, entry offset and lengths, the `.pkg`'s
/// size and time): what a recolour made from the PKG is known by, so a later start needn't read it.
fn pkg_entry(install: &kh1_data::Kh1Install) -> Option<String> {
    for part in kh1_data::ARCHIVES {
        let hed = install.image_dir.join(format!("{part}.hed"));
        if !hed.is_file() {
            continue;
        }
        let archive = kh2_formats::egs::Archive::open_cached(&hed).ok()?;
        if let Some(e) = archive.find(MODEL) {
            let meta = std::fs::metadata(&archive.pkg_path).ok()?;
            let time = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
            return Some(format!("{part} {:#x} {} {}; pkg {} bytes, time {time}", e.offset, e.stored_len, e.real_len, meta.len()));
        }
    }
    None
}

/// Texture `k`'s HD file (the DDS) from the PKG (the job's thread).
fn read_pkg(install: &kh1_data::Kh1Install, k: usize) -> Option<Vec<u8>> {
    let asset = format!("{}.dds", TEXTURES[k].1);
    match install.read_remastered(kh1_data::Source::Live, MODEL, &asset) {
        Ok((name, bytes)) => {
            log!(Animation, "form tint: {MODEL} {name} read from the PKG, on first need");
            Some(bytes)
        }
        Err(e) => {
            log!(Animation, "form tint: {MODEL} {asset} not read from the PKG ({e})");
            None
        }
    }
}

/// Starts slot `k`'s job for `look` on a thread: `mem` is KH1's copy of the source DDS (copied off
/// the game's memory), if it fits.
fn start(k: usize, look: Look, name: String, w: u32, h: u32, mem: Option<Vec<u8>>) {
    set_job(k, look, Job::Working);
    std::thread::spawn(move || {
        let t = std::time::Instant::now();
        let d = crate::tintcache::Desc { look: look_name(look).into(), asset: TEXTURES[k].1.trim_start_matches('-').into(), name, w, h, build: format!("{:?}", super::addr::Build::current()) };
        let Some((ready, store)) = build(k, look, &d, mem, t) else {
            set_job(k, look, Job::Failed);
            return;
        };
        // (Handed to the game thread first; the file written after, from the same buffer.)
        let bgra = ready.bgra.clone();
        set_job(k, look, Job::Done(ready));
        if let Some((source, origin)) = store {
            let dir = cache_dir();
            match crate::tintcache::store(&dir, &d, source, &origin, &bgra) {
                Ok(p) => log!(Loading, "form tint cache: written ({})", p.display()),
                Err(e) => log!(Loading, "form tint cache: {} not written ({e}); made again next time", d.file_name()),
            }
        }
    });
}

/// The job's work: the cached file if it matches, else the recolour (with what to write).
#[allow(clippy::type_complexity)]
fn build(k: usize, look: Look, d: &crate::tintcache::Desc, mem: Option<Vec<u8>>, t: std::time::Instant) -> Option<(Ready, Option<(u128, crate::tintcache::Origin)>)> {
    use crate::tintcache::{Key, Origin};
    let install = match kh1_data::Kh1Install::locate(super::exe_dir()) {
        Ok(i) => Some(i),
        Err(e) => {
            log!(Animation, "form tint: KH1's files not found ({e})");
            None
        }
    };
    // Anti: KH1 Anti-Sora's textures carried over (`bake_anti`), known by both models' PKG
    // entries; the recolour below only as the fallback.
    if look == Look::Anti {
        if let Some(r) = install.as_ref().and_then(|i| build_anti(k, d, i, t)) {
            return Some(r);
        }
    }
    let key = match &mem {
        Some(dds) => Some(Key::Content(crate::tintcache::source_hash(dds, d))),
        None => install.as_ref().and_then(pkg_entry).map(Key::Pkg),
    };
    // (Anti's fallback isn't cached: KH1 Anti-Sora's textures are tried again next start.)
    let key = key.filter(|_| look != Look::Anti);
    let dir = cache_dir();
    if let Some(key) = &key {
        match crate::tintcache::load(&dir, d, key) {
            Ok(bgra) => {
                log!(Loading, "form tint cache: {} ({look:?}, {}) from {} in {:.0} ms", d.name, d.file_name(), dir.display(), t.elapsed().as_secs_f32() * 1000.0);
                return Some((Ready { w: d.w, h: d.h, bgra }, None));
            }
            Err(why) => log!(Loading, "form tint cache: {} ({look:?}) made now ({why})", d.file_name()),
        }
    }
    // Not cached: KH1's copy, else the PKG's.
    let (dds, origin) = match mem {
        Some(m) => (m, Origin::Memory),
        None => {
            let dds = read_pkg(install.as_ref()?, k)?;
            let origin = match key {
                Some(Key::Pkg(id)) => Origin::Pkg(id),
                // (No PKG entry known: made, not cached.)
                _ => Origin::Pkg(String::new()),
            };
            (dds, origin)
        }
    };
    let source = crate::tintcache::source_hash(&dds, d);
    let what = match origin {
        Origin::Memory => format!("{} (KH1's copy in memory)", d.name),
        Origin::Pkg(_) => format!("{} (the PKG's)", d.name),
    };
    let Some(p) = dds_bgra(&dds) else {
        log!(Animation, "form tint: {what} isn't a plain 32-bit DDS");
        return None;
    };
    drop(dds);
    if p.w != d.w || p.h != d.h {
        log!(Animation, "form tint: {what} is {}x{}, not {}x{}", p.w, p.h, d.w, d.h);
        return None;
    }
    let p = recoloured(p, &what, k, look);
    let store = match &origin {
        Origin::Pkg(id) if id.is_empty() => None,
        _ if look == Look::Anti => None,
        _ => Some((source, origin)),
    };
    log!(Loading, "form tint cache: {} ({look:?}) recoloured in {:.0} ms", d.name, t.elapsed().as_secs_f32() * 1000.0);
    Some((Ready { w: p.w, h: p.h, bgra: crate::slotcache::Block::new(p.bgra) }, store))
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
    swapped: [Option<(usize, u64, Look)>; NTEX],
    /// Our GPU texture for a slot and a look, made once and used again for every later source of
    /// the same name and size (Sora's model loaded again by an area change: the same file).
    /// Before, it was made per source address, so each area change made new ones and the old were
    /// never released.
    ours: Option<HashMap<(usize, Look), Made>>,
    /// Those we couldn't make (not tried again): slot, look.
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
        Op::Black { rect, .. } => rect,
        Op::Paint { rect, .. } => rect,
        Op::Grey { rect, .. } => rect,
        Op::Shadow { .. } => None,
    };
    // (Sora's own eye drawing, for where Anti's solid eyes go.)
    let eyes_from = match op {
        Op::Shadow { eyes: Some(e) } => Some((e, kh1_data::image::Image { w: p.w, h: p.h, bgra: p.bgra.clone() })),
        _ => None,
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
                Op::Black { hue: Some((from, width)), min_v, .. } => {
                    let dh = (hue - from).rem_euclid(360.0);
                    let d = dh.min(360.0 - dh);
                    let k = ((width - d) / RED_SOFT).clamp(0.0, 1.0) * ((s - SAT_FROM) / SAT_SOFT).clamp(0.0, 1.0) * if min_v > 0.0 { ((v - min_v) / 0.08 + 0.5).clamp(0.0, 1.0) } else { 1.0 };
                    (k, rgb(0.0, 0.0, v * BLACK_V))
                }
                Op::Black { hue: None, .. } => {
                    let k = ((0.3 - s) / 0.1).clamp(0.0, 1.0) * ((v - 0.3) / 0.15).clamp(0.0, 1.0);
                    (k, rgb(0.0, 0.0, v * BLACK_V))
                }
                Op::Paint { from, width, to, sat, lo, hi, .. } => {
                    let dh = (hue - from).rem_euclid(360.0);
                    let d = dh.min(360.0 - dh);
                    let k = ((width - d) / RED_SOFT).clamp(0.0, 1.0) * ((s - SAT_FROM) / SAT_SOFT).clamp(0.0, 1.0);
                    (k, rgb(to, sat, lo + (hi - lo) * v))
                }
                Op::Grey { from, width, min_v, lo, hi, .. } => {
                    let dh = (hue - from).rem_euclid(360.0);
                    let d = dh.min(360.0 - dh);
                    let k = ((width - d) / RED_SOFT).clamp(0.0, 1.0) * ((s - SAT_FROM) / SAT_SOFT).clamp(0.0, 1.0) * if min_v > 0.0 { ((v - min_v) / 0.08 + 0.5).clamp(0.0, 1.0) } else { 1.0 };
                    let l = lo + (hi - lo) * v;
                    (k, (l, l, l))
                }
                Op::Shadow { .. } => (1.0, rgb(SHADOW_HUE, SHADOW_SAT, SHADOW_LO + (SHADOW_HI - SHADOW_LO) * v)),
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
        let mut img = kh1_data::image::Image { w: p.w, h: p.h, bgra: std::mem::take(&mut p.bgra) };
        solid_eyes(&mut img, &orig, eyes);
        p.bgra = img.bgra;
    }
    changed
}

/// Anti's solid yellow eyes on `p` where Sora's are in `orig` (the same size). Returns how many
/// eyes (the face) or open frames (the eye sheet) got them.
fn solid_eyes(p: &mut kh1_data::image::Image, orig: &kh1_data::image::Image, eyes: Eyes) -> usize {
    use kh1_data::texmap;
    match eyes {
        Eyes::Face => texmap::solid_eyes(p, orig, texmap::face_eyes_area(orig.w, orig.h), texmap::ANTI_EYE_RGB),
        Eyes::Frames => texmap::solid_eye_frames(p, orig, FRAME_H, texmap::ANTI_EYE_RGB),
    }
}

/// KH1 Anti-Sora's textures carried onto Sora's (`bake_anti`), per `TEXTURES` (`None`: that one
/// not made, or already taken by its job); `Err`: why not.
type Baked = Result<Vec<Option<kh1_data::image::Image>>, String>;

/// The carry-over, made once a session and only when a cached file doesn't serve (the first Anti
/// job makes it, the others wait on the lock; each takes its texture out, so the memory goes
/// with the last).
static ANTI: Mutex<Option<Baked>> = Mutex::new(None);

/// What Anti's textures are made from, as text (both models' PKG entries: `Kh1Install::stamp`),
/// so a later start uses the cached files without reading either.
fn anti_id(install: &kh1_data::Kh1Install) -> String {
    let src = kh1_data::Source::Live;
    format!("{MODEL}: {}; {ANTI_MODEL}: {}", install.stamp(src, MODEL), install.stamp(src, ANTI_MODEL))
}

/// Anti's texture `k` (the job's thread): the cached file if it matches both models' PKG entries,
/// else made (`bake_anti`) and written. `None`: not made (the fallback recolour then).
#[allow(clippy::type_complexity)]
fn build_anti(k: usize, d: &crate::tintcache::Desc, install: &kh1_data::Kh1Install, t: std::time::Instant) -> Option<(Ready, Option<(u128, crate::tintcache::Origin)>)> {
    use crate::tintcache::{Key, Origin};
    let id = anti_id(install);
    let dir = cache_dir();
    match crate::tintcache::load(&dir, d, &Key::Pkg(id.clone())) {
        Ok(bgra) => {
            log!(Loading, "form tint cache: {} (Anti, {}) from {} in {:.0} ms", d.name, d.file_name(), dir.display(), t.elapsed().as_secs_f32() * 1000.0);
            return Some((Ready { w: d.w, h: d.h, bgra }, None));
        }
        Err(why) => log!(Loading, "form tint cache: {} (Anti) made now ({why})", d.file_name()),
    }
    let img = {
        let mut g = ANTI.lock().ok()?;
        let baked = g.get_or_insert_with(|| {
            let r = bake_anti(install);
            if let Err(e) = &r {
                log!(Animation, "form tint (Anti): KH1 Anti-Sora's textures not made, Sora's darkened instead ({e})");
            }
            r
        });
        let img = baked.as_mut().ok()?.get_mut(k)?.take();
        // (All taken: dropped; one needed again later, its cached file missing, is made again.)
        if baked.as_ref().is_ok_and(|v| v.iter().all(|x| x.is_none())) {
            *g = None;
        }
        img
    };
    let Some(img) = img else {
        log!(Animation, "form tint (Anti): no Anti-Sora texture for {} ({}), darkened instead", d.name, TEXTURES[k].1);
        return None;
    };
    if (img.w, img.h) != (d.w, d.h) {
        log!(Animation, "form tint (Anti): {} is {}x{} in the game, Anti-Sora's made for {}x{}; darkened instead", d.name, d.w, d.h, img.w, img.h);
        return None;
    }
    let source = crate::slotcache::hash(id.as_bytes());
    Some((Ready { w: img.w, h: img.h, bgra: crate::slotcache::Block::new(img.bgra) }, Some((source, Origin::Pkg(id)))))
}

/// KH1 Anti-Sora's textures carried onto Sora's (`kh1_data::texmap`, `docs/kh1-tint.md`), from
/// the PKG: Sora's skeleton, his remastered mesh and his own HD textures (each the base, its size
/// and alpha kept); Anti-Sora's PS2 mesh and his textures (his HD ones where they are his PS2
/// ones drawn larger, else the PS2 ones). The face gets Anti's solid eyes, and the eye and mouth
/// sheets are made of the new face where KH1 pastes them (Anti-Sora has neither), the eyes' with
/// the solid eyes on each open frame.
fn bake_anti(install: &kh1_data::Kh1Install) -> Baked {
    use kh1_data::image::Image;
    use kh1_data::texmap;
    let started = std::time::Instant::now();
    let src = kh1_data::Source::Live;
    let read = |name: &str| install.read(src, name).map_err(|e| format!("{name}: {e}"));
    let asset = |name: &str, a: &str| install.read_remastered(src, name, a).map(|(_, b)| b).map_err(|e| format!("{name} {a}: {e}"));
    // Sora: his remastered mesh in his bind pose.
    let sora_mdls = read(MODEL)?;
    let sora_bones = texmap::bind_pose(&sora_mdls).map_err(|e| format!("{MODEL}: {e}"))?;
    drop(sora_mdls);
    let target = texmap::posed(&kh1_data::mesh::cvbl_faces(&asset(MODEL, "-c0.cvbl")?).map_err(|e| format!("{MODEL} -c0.cvbl: {e}"))?, &sora_bones).map_err(|e| format!("{MODEL}: {e}"))?;
    // Anti-Sora: his PS2 mesh in his bind pose, his textures.
    let anti = read(ANTI_MODEL)?;
    let bones = texmap::bind_pose(&anti).map_err(|e| format!("{ANTI_MODEL}: {e}"))?;
    let tris = texmap::posed(&kh1_data::mdls::parse_faces(&anti).map_err(|e| format!("{ANTI_MODEL}: {e}"))?, &bones).map_err(|e| format!("{ANTI_MODEL}: {e}"))?;
    let ps2 = kh1_data::mdls::parse_textures(&anti).map_err(|e| format!("{ANTI_MODEL}: {e}"))?;
    drop(anti);
    let textures: Vec<Option<Image>> = ps2
        .into_iter()
        .enumerate()
        .map(|(n, p)| {
            let hd = asset(ANTI_MODEL, &format!("-1c{n}")).ok().and_then(|b| Image::from_dds(&b));
            match hd.as_ref().and_then(|h| p.difference_from_larger(h).map(|d| (h, d))) {
                Some((h, d)) if d <= HD_MATCH => {
                    log!(Animation, "form tint (Anti): {ANTI_MODEL} texture {n}: his HD -1c{n} ({}x{}; {d:.1} from his PS2 one)", h.w, h.h);
                    hd
                }
                found => {
                    log!(Animation, "form tint (Anti): {ANTI_MODEL} texture {n}: his PS2 one ({}x{}; HD -1c{n}: {:?})", p.w, p.h, found.map(|(h, d)| (h.w, h.h, d)));
                    Some(p)
                }
            }
        })
        .collect();
    let source = texmap::Source::new(&tris, &textures);
    // Sora's own HD textures, the bases (from the PKG: the same files KH1 loads).
    let own: Vec<Option<Image>> = TEXTURES.iter().map(|(_, a)| asset(MODEL, &format!("{a}.dds")).ok().and_then(|b| Image::from_dds(&b))).collect();
    let mut out: Vec<Option<Image>> = vec![None; NTEX];
    for (k, (tex, a)) in TEXTURES.iter().enumerate() {
        let Tex::Mesh(_) = tex else { continue };
        let Some(slot) = a.strip_prefix("-1c").and_then(|s| s.parse::<u32>().ok()) else { continue };
        let Some(base) = &own[k] else {
            log!(Animation, "form tint (Anti): Sora's {a} not read from the PKG, so Anti-Sora's isn't carried onto it");
            continue;
        };
        let (img, st) = texmap::transfer(&target, slot, base, &source);
        log!(
            Animation,
            "form tint (Anti): Anti-Sora onto Sora's {a} ({}x{}): {} triangles, {} texels covered, {} filled beside, gap to his surface {:.4} mean, {:.3} max",
            img.w, img.h, st.triangles, st.covered, st.filled, st.mean_gap, st.max_gap
        );
        if st.triangles > 0 {
            out[k] = Some(img);
        }
    }
    // The eye and mouth sheets: the new face where KH1 pastes each (before the face's own eyes).
    let face = TEXTURES.iter().position(|t| t.1 == "-1c0").unwrap_or(0);
    for (k, (tex, a)) in TEXTURES.iter().enumerate() {
        let Tex::Frames(_) = tex else { continue };
        let (Some(sheet), Some(orig), Some(new)) = (&own[k], &own[face], &out[face]) else { continue };
        match texmap::find_frame(orig, sheet, FRAME_H) {
            Some((x, y, d)) if d <= FRAME_MATCH => {
                let mut img = texmap::frames_from(new, x, y, sheet, FRAME_H);
                let open = if *a == "-2c0" { solid_eyes(&mut img, sheet, Eyes::Frames) } else { 0 };
                log!(Animation, "form tint (Anti): {a} frames made from the face at ({x}, {y}) (difference {d:.1}), solid eyes on {open} of {} frames", sheet.h / FRAME_H);
                out[k] = Some(img);
            }
            found => log!(Animation, "form tint (Anti): {a}'s first frame not found on the face ({found:?})"),
        }
    }
    if let (Some(img), Some(orig)) = (out[face].as_mut(), &own[face]) {
        let n = solid_eyes(img, orig, Eyes::Face);
        log!(Animation, "form tint (Anti): {n} solid eyes on the face");
    }
    if out.iter().all(|x| x.is_none()) {
        return Err("none of Sora's textures could take Anti-Sora's".into());
    }
    log!(Animation, "form tint (Anti): KH1 Anti-Sora's textures ready in {:.1} s", started.elapsed().as_secs_f32());
    Ok(out)
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
    log!(Animation, "form tint ({look:?}): {what}, {}x{}, texels recoloured per step {counts:?}", p.w, p.h);
    p
}

/// At start: only where the game is. Nothing is read now: the PKG's pixels are read the first
/// time a form needs them (`pkg_pixels`), when KH1's own copy doesn't fit.
pub fn install() {
    let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    BASE.store(base, Ordering::Relaxed);
}

/// Sora's model's source texture for `TEXTURES[k]` (`docs/kh1-tint.md`), if it is an HD texture.
unsafe fn source_texture(actor: usize, k: usize) -> Option<usize> {
    match TEXTURES[k].0 {
        Tex::Mesh(entry) => mesh_texture(actor, entry),
        Tex::Frames(tag) => frames_texture(actor, tag),
    }
}

/// The remastered model data behind Sora's model header `+0x14` (`0x96969696` at `+0x10`): its
/// mesh table at `+0`, its texture-animation patches at `+0x20` / `+0x28`.
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
    (p > 0x10000 && super::crash::readable(p + 0x30)).then_some(p)
}

/// The texture-animation frame sheet whose name holds `tag` (`mdls2c0.`: the eyes): the model's
/// patches, a count at `+0x20` and 32-byte records from the pointer at `+0x28`, each record's
/// `+0x18` the `Axa::EntryTex` it pastes from. Checked (decomp): KH1's `0x1401DD4D0` indexes
/// the records so (`0x96969696` header, handle at `+0x14`, `+0x20` count, `+0x28` × 0x20), and
/// its caller (`0x1401D4070`) writes the record's `+0x18` texture's `+0x94`. The names: main's
/// reading (`…_mdls2c0.dds`, as KH1's own `xa_he_1000_mdls2c0.dds`), Assumption here.
unsafe fn frames_texture(actor: usize, tag: &[u8]) -> Option<usize> {
    let p = remastered(actor)?;
    let n = rd::<u32>(p + 0x20) as usize;
    let patches = rd::<u64>(p + 0x28) as usize;
    if n == 0 || n > 64 || patches < 0x10000 || !super::crash::readable(patches) || !super::crash::readable(patches + 0x20 * n - 1) {
        return None;
    }
    (0..n).map(|i| rd::<u64>(patches + 0x20 * i + 0x18) as usize).find(|&t| {
        t >= 0x10000 && super::crash::readable(t + 0x98) && rd::<u32>(t + 0x10) == TEX_MAGIC && rd::<[u8; 32]>(t + 0x14).windows(tag.len()).any(|w| w == tag)
    })
}

/// Mesh-table entry `entry`'s source texture.
unsafe fn mesh_texture(actor: usize, entry: usize) -> Option<usize> {
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
        log!(Animation, "form tint ({look:?}): slot {k} is now {name} ({w}x{h}), ours was made for {} ({}x{})", m.name, m.w, m.h);
    }
    // The pixels: from the cache, else recoloured from the DDS KH1 keeps (the same file) or the
    // PKG's, on a thread (`start`; KH1's colours meanwhile, asked again next frame).
    let p = match take_job(k, look) {
        None => {
            let dds = rd::<u64>(src + TEX_DDS) as usize;
            let len = 128 + (w as usize) * (h as usize) * 4;
            let fits = dds > 0x10000 && super::crash::readable(dds + 128) && rd::<u32>(dds) == 0x2053_4444 && rd::<u32>(dds + 12) == h && rd::<u32>(dds + 16) == w && super::crash::readable(dds + len - 1);
            // (Copied off KH1's memory here, on the game thread: KH1 may release it meanwhile.)
            let mem = fits.then(|| std::slice::from_raw_parts(dds as *const u8, len).to_vec());
            start(k, look, name, w, h, mem);
            return None;
        }
        Some(Job::Working) => return None,
        Some(Job::Done(p)) if p.w == w && p.h == h => p,
        Some(Job::Done(_)) | Some(Job::Failed) => {
            log!(Animation, "form tint: no pixels for {name} ({w}x{h}; neither KH1's copy nor the PKG's fits)");
            s.failed.push((k, look));
            return None;
        }
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
        Look::Limit => b"kh2combat_limit_black\0",
        Look::Master => b"kh2combat_master_yellow\0",
        Look::Final => b"kh2combat_final_white\0",
        Look::Anti => b"kh2combat_anti_sora\0",
    };
    let t = create(r, &desc, p.bgra.as_ptr(), label.as_ptr(), 0);
    if t < 0x10000 {
        log!(Animation, "form tint: KH1 made no texture for {name} ({t:#x})");
        s.failed.push((k, look));
        return None;
    }
    log!(Animation, "form tint ({look:?}): {name} ready ({w}x{h}, texture {t:#x}; KH1's {:#x}); its source pixels dropped", rd::<u64>(src + TEX_GPU));
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
    for k in 0..NTEX {
        let want = look.filter(|l| wears(*l, k));
        // (Nothing worn and nothing to put back: Sora's model isn't looked at.)
        if want.is_none() && s.swapped[k].is_none() {
            continue;
        }
        let src = source_texture(actor, k);
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
        log!(Animation, "form tint: still nothing swapped after 10 s in the form");
    }
}
