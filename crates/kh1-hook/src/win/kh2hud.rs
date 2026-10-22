//! KH2's own Drive gauge HUD, from the user's KH2 install (the user: "Can you bring in the actual
//! drive gauge HUD from KH2?", then "use ghidra if you have to on KH2 to get the exact graphics,
//! the animation while the gauge is charging, when its being drained, and all the animations
//! that the hud element has"). `docs/drive-model.md`, "KH2's Drive gauge HUD".
//!
//! **The art**: `field2d/<lang>/zz0field.2dd`, KH2's field HUD (the file KH2 loads for its field
//! HUD, `field2d/%s/%s0field.2dd` with the world's prefix; `zz` is its default), its sequence
//! `gaug` (`kh2_formats::seqd`) and that sequence's image, the PKG's remastered PNG beside it
//! (1024 × 512 for the PS2's 512 × 256). Read and decoded on a thread at start; nothing of it is
//! kept on disk or in the mod.
//!
//! **What plays when** (KH2's own choices, read in its HUD code; the numbers are the sequence's
//! animation groups): the bar appears with group 21 (the DRIVE bar sliding up into place), then
//! stays as group 23; the fill is group 25, shown to the current bar's points (KH2 sweeps a mask
//! over it; here it is cut at the share); the number of full bars is group 28 + n (each digit
//! pops in: turned, scaled and faded from nothing, then sparkles); a bar gained plays group 22 over
//! the bar (the fill flashing white and streaks); a full gauge shows group 20 (MAX, flashing in
//! and cycling its colours). In a form: the FORM bar (group 50 sliding in, then 51 pulsing), its
//! fill (group 49, pulsing) drains with the form gauge, the number is 54 + the bars left; changing
//! between the two plays group 130 (a ring of sparkles) on Sora's gauge.
//!
//! **Drawn with KH1's own sprite path** (the HP/MP gauge task, as `gaugehud`): each quad is built
//! from a Drive list row's record as `gaugehud`'s are, then its corners, texture coordinates and
//! colours are set from KH2's sequence, and for the draw the shared HUD texture's entry in KH1's
//! texture map (`0x14010D860`, by the quad's TEX0) points at our texture (its GPU texture `+0x70`,
//! put back at once): KH1 binds the entry's texture when it draws the quad.

use super::addr::Rva;
use super::log;
use kh2_formats::seqd::{Quad, Sequence};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

/// KH1's texture map: `(&TEX0) -> entry` (the entry: magic `+0x10`, GPU texture `+0x70`, its PS2
/// size `+0x94`, `+0x96`).
const LOOKUP_RVA: Rva = Rva(0x10D860);
const TEX_MAGIC: u32 = 0x02F0_2EB5;
/// A sprite record into a quad `(record, quad, texture)`, the quad, its draw `(quad, 1, packet)`.
const BUILD_RVA: Rva = Rva(0x27B300);
const QUAD_RVA: Rva = Rva(0x2867360);
const SUBMIT_RVA: Rva = Rva(0x274340);
const PACKET_RVA: Rva = Rva(0x2853218);
const GATE_RVA: Rva = Rva(0x528820);

/// KH2's animation groups for the gauge (from its HUD code: `docs/drive-model.md`).
const BAR_APPEAR: usize = 21;
const BAR: usize = 23;
const FILL: usize = 25;
const MAX: usize = 20;
const GAINED: usize = 22;
const NUMBER: usize = 28;
const FORM_APPEAR: usize = 50;
const FORM_BAR: usize = 51;
const FORM_FILL: usize = 49;
const FORM_NUMBER: usize = 54;
const MODE_CHANGE: usize = 130;
/// The languages KH2's field HUD comes in, tried in turn.
const LANGS: [&str; 7] = ["us", "uk", "fr", "gr", "it", "sp", "jp"];

pub struct Art {
    seq: Sequence,
    /// The image, BGRA, its size, and the PS2 image's size the sequence's texels count in.
    bgra: Vec<u8>,
    w: u32,
    h: u32,
    ps2: (f32, f32),
}

static ART: OnceLock<Result<Art, String>> = OnceLock::new();
/// Our texture in KH1's renderer (0: not made yet), and a failed attempt.
static GPU: AtomicU64 = AtomicU64::new(0);
static GPU_FAILED: AtomicBool = AtomicBool::new(false);

/// Reads KH2's gauge art (a thread at start).
pub fn load() {
    std::thread::spawn(|| {
        let r = load_art();
        match &r {
            Ok(a) => log(&format!(
                "KH2 Drive gauge HUD: field HUD sequence 'gaug' ({} animation groups) and its {}x{} image (PS2 {}x{}) read from the KH2 install",
                a.seq.groups.len(),
                a.w,
                a.h,
                a.ps2.0,
                a.ps2.1
            )),
            Err(e) => log(&format!("KH2 Drive gauge HUD: not read ({e}); the gauge is drawn with KH1's pieces instead")),
        }
        let _ = ART.set(r);
    });
}

fn load_art() -> Result<Art, String> {
    let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
    let install = kh2_data::Install::locate(&exe).map_err(|e| e.to_string())?;
    let src = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
    let (name, data) = LANGS
        .iter()
        .map(|l| format!("field2d/{l}/zz0field.2dd"))
        .find_map(|n| install.read_any(src, &n).ok().map(|d| (n, d)))
        .ok_or("no field2d/<lang>/zz0field.2dd")?;
    let bar = kh2_formats::bar::Bar::parse(&data).map_err(|e| format!("{name}: {e}"))?;
    let seq = bar.entries.iter().find(|e| e.kind == 25 && e.name == "gaug").ok_or(format!("{name}: no sequence 'gaug'"))?;
    let seq = Sequence::parse(seq.data).map_err(|e| format!("{name} gaug: {e}"))?;
    // Its image: the BAR's image entries are the remastered PNGs' order.
    let images: Vec<_> = bar.entries.iter().filter(|e| e.kind == 24).collect();
    let k = images.iter().position(|e| e.name == "gaug").ok_or(format!("{name}: no image 'gaug'"))?;
    let imgd = images[k].data;
    if imgd.len() < 0x20 || &imgd[..4] != b"IMGD" {
        return Err(format!("{name}: image 'gaug' isn't an IMGD"));
    }
    let ps2 = (u16::from_le_bytes([imgd[0x1C], imgd[0x1D]]) as f32, u16::from_le_bytes([imgd[0x1E], imgd[0x1F]]) as f32);
    let assets = install.read_remastered_any(src, &name).map_err(|e| format!("{name} HD images: {e}"))?;
    let (asset, png) = assets.get(k).ok_or(format!("{name}: {} HD images, wanted the {}th", assets.len(), k + 1))?;
    let im = kh2_formats::png::decode(png).map_err(|e| format!("{asset}: {e}"))?;
    let bgra = im.rgba.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();
    Ok(Art { seq, bgra, w: im.width, h: im.height, ps2 })
}

/// An animation group playing: which, and the frames since it started.
#[derive(Clone, Copy)]
struct Play {
    g: usize,
    t: i32,
}

impl Play {
    fn new(g: usize) -> Play {
        Play { g, t: 0 }
    }
}

struct State {
    form: bool,
    bar: Play,
    fill: Play,
    number: Play,
    value: i32,
    max: Option<Play>,
    /// One-off effects: the group, and whether it goes with the bar (else Sora's gauge).
    effects: Vec<(Play, bool)>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// What the gauge shows this frame.
pub struct View {
    /// The current bar's share (1 when full).
    pub frac: f32,
    /// The digit shown (full bars, or in a form KH2's count of the bars left).
    pub number: i32,
    pub max: bool,
    pub form: bool,
}

/// Steps KH2's animations one frame for `v`.
fn step(seq: &Sequence, v: &View) -> State {
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let n = v.number.clamp(0, 9);
    let s = g.get_or_insert_with(|| State {
        form: v.form,
        bar: Play::new(if v.form { FORM_APPEAR } else { BAR_APPEAR }),
        fill: Play::new(if v.form { FORM_FILL } else { FILL }),
        number: Play::new(if v.form { FORM_NUMBER } else { NUMBER } + n as usize),
        value: n,
        max: None,
        effects: Vec::new(),
    });
    if s.form != v.form {
        // Into or out of a form: the other bar slides in, KH2's sparkle ring on the gauge.
        s.form = v.form;
        s.bar = Play::new(if v.form { FORM_APPEAR } else { BAR_APPEAR });
        s.fill = Play::new(if v.form { FORM_FILL } else { FILL });
        s.number = Play::new(if v.form { FORM_NUMBER } else { NUMBER } + n as usize);
        s.value = n;
        s.max = None;
        s.effects.push((Play::new(MODE_CHANGE), false));
    }
    if n != s.value {
        // A new digit pops in; a bar gained (not in a form) flashes the bar.
        if !v.form && n > s.value {
            s.effects.push((Play::new(GAINED), true));
        }
        s.value = n;
        s.number = Play::new(if v.form { FORM_NUMBER } else { NUMBER } + n as usize);
    }
    match (v.max && !v.form, s.max.is_some()) {
        (true, false) => s.max = Some(Play::new(MAX)),
        (false, true) => s.max = None,
        _ => {}
    }
    // The bar's slide-in done: its steady group.
    if (s.bar.g == BAR_APPEAR || s.bar.g == FORM_APPEAR) && seq.finished(s.bar.g, s.bar.t) {
        s.bar = Play::new(if s.form { FORM_BAR } else { BAR });
    }
    let out = State { form: s.form, bar: s.bar, fill: s.fill, number: s.number, value: s.value, max: s.max, effects: s.effects.clone() };
    for p in [&mut s.bar, &mut s.fill, &mut s.number].into_iter().chain(s.max.iter_mut()) {
        p.t = p.t.saturating_add(1).min(1 << 24);
    }
    for (p, _) in s.effects.iter_mut() {
        p.t += 1;
    }
    s.effects.retain(|(p, _)| !seq.finished(p.g, p.t));
    out
}

/// Whether KH2's art is ready (read; the texture is made on the first draw).
pub fn ready() -> bool {
    matches!(ART.get(), Some(Ok(_))) && !GPU_FAILED.load(Ordering::Relaxed)
}

/// Draws KH2's gauge for `v` with KH2's gauge centre at `(ox, oy)` in KH1's HUD units, `k` KH1
/// units to KH2's, faded by `alpha`, through KH1's sprite path (its draw state already set up by
/// `gaugehud`): `template` a sprite record to build quads from, `texture` the texture descriptor
/// whose entry is pointed at ours meanwhile. Returns the quads drawn, `None` if KH2's art isn't
/// ready.
#[allow(clippy::too_many_arguments)]
pub unsafe fn draw(base: usize, v: &View, ox: f32, oy: f32, k: f32, alpha: f32, template: usize, texture: usize, rec: usize) -> Option<(u32, [f32; 4])> {
    let Some(Ok(art)) = ART.get() else { return None };
    if GPU_FAILED.load(Ordering::Relaxed) {
        return None;
    }
    let mut gpu = GPU.load(Ordering::Relaxed);
    if gpu == 0 {
        match super::tint::make_texture(art.w, art.h, &art.bgra, b"kh2combat_kh2_drive_gauge\0") {
            Some(t) => {
                GPU.store(t, Ordering::Relaxed);
                gpu = t;
                log(&format!("KH2 Drive gauge HUD: its image made in KH1's renderer ({}x{}, texture {t:#x})", art.w, art.h));
            }
            None => {
                GPU_FAILED.store(true, Ordering::Relaxed);
                log("KH2 Drive gauge HUD: KH1 made no texture for KH2's image; the gauge is drawn with KH1's pieces instead");
                return None;
            }
        }
    }
    let seq = &art.seq;
    let s = step(seq, v);
    // The quads, in KH2's units around its gauge centre: the bar, its fill (cut at the share),
    // MAX, the number, the effects. The fill, MAX, the number and the bar's effects go with the
    // bar (its translation; the number at its attach point), as KH2 parents them.
    let bar_f = seq.frame(s.bar.g, s.bar.t);
    let steady = s.bar.g == BAR || s.bar.g == FORM_BAR;
    let o = seq.origin(s.bar.g, bar_f);
    let attach = seq.groups.get(s.bar.g).map_or([0, 0], |g| g.attach);
    let mut list: Vec<(Quad, [f32; 2], Option<f32>)> = Vec::new();
    let push = |g: usize, t: i32, at: [f32; 2], cut: Option<f32>, list: &mut Vec<(Quad, [f32; 2], Option<f32>)>| {
        let mut q = Vec::new();
        seq.quads(g, seq.frame(g, t), &mut q);
        list.extend(q.into_iter().map(|q| (q, at, cut)));
    };
    push(s.bar.g, s.bar.t, [0.0, 0.0], None, &mut list);
    if steady {
        push(s.fill.g, s.fill.t, o, Some(v.frac.clamp(0.0, 1.0)), &mut list);
        if let Some(m) = s.max {
            push(m.g, m.t, o, None, &mut list);
        }
        push(s.number.g, s.number.t, [o[0] + attach[0] as f32, o[1] + attach[1] as f32], None, &mut list);
    }
    for (p, with_bar) in &s.effects {
        push(p.g, p.t, if *with_bar { o } else { [0.0, 0.0] }, None, &mut list);
    }
    // KH1's texture entry for the quads' TEX0, pointed at ours while they draw.
    let q = base + QUAD_RVA;
    let build: unsafe extern "system" fn(usize, usize, usize) = std::mem::transmute(base + BUILD_RVA);
    let lookup: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(base + LOOKUP_RVA);
    let submit: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + SUBMIT_RVA);
    std::ptr::copy_nonoverlapping(template as *const u8, rec as *mut u8, 0xC0);
    wr(rec + 0x60, (rd::<u32>(rec + 0x60) & !(0x10 | 0x80 | 0x08)) | 0x20);
    build(rec, q, texture);
    let entry = lookup(q + 0x10);
    if entry < 0x10000 || !super::crash::readable(entry + 0xA8) || rd::<u32>(entry + 0x10) != TEX_MAGIC {
        static ONCE: AtomicBool = AtomicBool::new(false);
        if !ONCE.swap(true, Ordering::Relaxed) {
            log(&format!("KH2 Drive gauge HUD: no texture entry for the HUD texture's TEX0 ({entry:#x}); the gauge is drawn with KH1's pieces instead"));
        }
        GPU_FAILED.store(true, Ordering::Relaxed);
        return None;
    }
    // (The quad's texture coordinates count in the entry's PS2 size; KH2's in its PS2 image's.)
    let (pw, ph) = (rd::<u16>(entry + 0x94) as f32, rd::<u16>(entry + 0x96) as f32);
    if pw < 1.0 || ph < 1.0 {
        GPU_FAILED.store(true, Ordering::Relaxed);
        return None;
    }
    let (su, sv) = (pw / art.ps2.0 * 16.0, ph / art.ps2.1 * 16.0);
    let mut drawn = 0;
    let mut bounds = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    let gate = rd::<f32>(base + GATE_RVA) != 0.0;
    for (quad, at, cut) in list {
        let mut c = quad.corners;
        let mut uv = quad.uv.map(|x| x as f32);
        // The fill cut at the share: its right edge moved in, its texture with it.
        if let Some(frac) = cut {
            let (x0, x1) = (c[0][0].min(c[2][0]), c[0][0].max(c[2][0]));
            let edge = x0 + (x1 - x0) * frac;
            if edge <= x0 + 0.01 {
                continue;
            }
            if edge < x1 {
                let r = (edge - x0) / (x1 - x0);
                uv[2] = uv[0] + (uv[2] - uv[0]) * r;
                c[2][0] = edge;
                c[3][0] = edge;
            }
        }
        std::ptr::copy_nonoverlapping(template as *const u8, rec as *mut u8, 0xC0);
        wr(rec + 0x60, (rd::<u32>(rec + 0x60) & !(0x10 | 0x80 | 0x08)) | 0x20);
        for (i, col) in quad.colours.iter().enumerate() {
            let a = (col[3] * alpha).clamp(0.0, 255.0);
            wr(rec + 0x20 + i * 0x10, [col[0].clamp(0.0, 255.0) as i32, col[1].clamp(0.0, 255.0) as i32, col[2].clamp(0.0, 255.0) as i32, a as i32]);
        }
        build(rec, q, texture);
        // Corners (TL, BL, TR, BR) as KH1's quad keeps them: (x + 0x700, y + 0x740) × 16.
        for (i, p) in c.iter().enumerate() {
            let (x, y) = (ox + (p[0] + at[0]) * k, oy + (p[1] + at[1]) * k);
            bounds = [bounds[0].min(x), bounds[1].min(y), bounds[2].max(x), bounds[3].max(y)];
            wr(q + 0xA0 + i * 0x10, ((x + 0x700 as f32) * 16.0).round() as i32);
            wr(q + 0xA4 + i * 0x10, ((y + 0x740 as f32) * 16.0).round() as i32);
        }
        // Texture coordinates per corner: TL (u0, v0), BL (u0, v1), TR (u1, v0), BR (u1, v1).
        for (i, (u, v)) in [(uv[0], uv[1]), (uv[0], uv[3]), (uv[2], uv[1]), (uv[2], uv[3])].into_iter().enumerate() {
            wr(q + 0x20 + i * 0x10, (u * su).round() as i32);
            wr(q + 0x24 + i * 0x10, (v * sv).round() as i32);
        }
        if !gate {
            continue;
        }
        let keep = rd::<u64>(entry + 0x70);
        wr(entry + 0x70, gpu);
        wr(base + PACKET_RVA, submit(q, 1, rd::<u64>(base + PACKET_RVA) as usize) as u64);
        wr(entry + 0x70, keep);
        drawn += 1;
    }
    Some((drawn, bounds))
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}
