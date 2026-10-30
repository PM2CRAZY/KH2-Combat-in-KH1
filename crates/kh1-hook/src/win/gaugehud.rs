//! KH2's Drive gauge drawn on KH1's HUD by KH1's own sprite draw (t939; the owner, the main-branch
//! port's round 3, "go ahead"; notes `docs/kh1-drive-forms-native.md` §14).
//!
//! **Where it runs** (Steam KH1, Checked (decomp)): KH1's HP/MP gauge task `0x14026DB00` maps its HUD
//! image (`0x14010DC50`) and draws each bound party slot's gauge body with `0x14026CF90`, then the
//! portrait (`0x14027F470`). That body is hooked: after KH1's own body for Sora's slot (record
//! `0x142811A00`, kind 0), the gauge is drawn there, so KH1's own rules decide when it shows (the
//! event bit skips the task's slot loop, an unbound slot isn't drawn, the slot's alpha `+0xA9C` fades
//! it: §14.2.5) and the portrait is drawn over its end as in KH2.
//!
//! **How**: KH2's pieces played by KH2's own sequence player (`crate::seqplay`) from the `gaug`
//! sequence, each quad built by KH1's sprite-to-quad `0x14027D470` from a sprite record of ours and
//! submitted by KH1's `0x1402764B0` into KH1's HUD packet (`0x142821BC0`), as KH1's `0x14026CE90`
//! does; corners, texture coordinates (KH2's texels × 16 + 8 of the PS2 picture, as KH1's own) and
//! colours set from KH2's quad. The texture: ours (`kh2hud`, t938), mapped each frame as KH1 maps its
//! own HUD image, set with KH1's `0x140386660` and bound (`0x14010D720` / `0x140108910`) as the body
//! does, KH1's HUD texture set back after. KH2's blends (normal, additive, subtractive) through KH1's
//! ALPHA register write (`0x140277160` / `0x140277130`, KH1's 0x44 / 0x48 / 0x42), normal after.
//!
//! **What plays** (KH2's player gauge `0x1401816C0` / `0x14017F9B0`, Checked (decomp) and (exe) for
//! the tables): Drive: the bar group 23 (KH2's slide-in 21 left out: the owner, round 3 answer 3), the
//! fill 25, the number `0x14071B108`[bars] = 28 + bars, MAX 20 once the bars reach the maximum; in a
//! form on the Drive gauge: bar 51, fill 49, number `0x14071B130`[n] = 54 + n, n = ticks left / ticks
//! per bar (one less when full, `0x140180490`). The fill, MAX and the number are placed on the bar
//! (KH2 parents them to it), the number at the bar group's attach point; the fill under KH2's mask
//! (`seqplay::outside`): its angle `0x14071B158` (0.0040143 rad, Checked (exe)) × points, × 200 when
//! full; in a form 23° / ticks per bar × ticks into the bar. KH2's clock: 1 a 60 Hz frame; KH1's
//! frame scale `0x14233FBDC` (60 Hz frames since the last pass, 0 on a held frame) steps it.
//!
//! **When** (the owner, round 3 answers 5, 7): Drive on, Form Cost on the Drive gauge, and a form
//! owned (`formown::owned`); else not drawn and KH1's MP HP label is back.
//!
//! **Where and how big** (the owner, round 3 answer 2 (b): main's measured place): KH2's gauge centre
//! (the bar's parent, KH2's HP ring) at Sora's portrait's middle + 6 across, and down at KH1's MP HP
//! label's middle less KH2's bar middle (13.5 KH2 units) plus 0.3 of the portrait's radius + 1.3;
//! 384 / 448 × 1.34 KH1 HUD units per KH2 unit. The portrait and the label are KH1's own sprites of
//! Sora's gauge (lists `+0x50` / `+0x10`, rectangles by `0x14027E430` at the place `0x14026D990`
//! gives), found once a second (Assumption: the label is the widest low sprite left of the portrait,
//! main's rule; logged).
//!
//! **KH1's MP HP label** (answer 4 (a)): hidden while the gauge shows, as data: its sprites' size set
//! to 0 when the gauge starts showing, put back when it stops (KH1 draws a 0-size sprite as nothing).

use super::log;
use crate::seqplay::{self, Player, Quad, Seq};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const BODY_RVA: usize = 0x26CF90;
const BODY_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x60];
const SORA_RVA: usize = 0x2811A00;
const PLACE_RVA: usize = 0x26D990;
const RECT_RVA: usize = 0x27E430;
const BUILD_RVA: usize = 0x27D470;
const SUBMIT_RVA: usize = 0x2764B0;
const REG_RVA: usize = 0x277160;
const GIFTAG_RVA: usize = 0x277130;
const SEND_TEX0_RVA: usize = 0x386660;
const LOOKUP_RVA: usize = 0x10D720;
const BIND_RVA: usize = 0x108910;
const MAP_RVA: usize = 0x10DC50;
const PACKET_RVA: usize = 0x2821BC0;
const FRAME_SCALE_RVA: usize = 0x233FBDC;
/// The calls' first bytes (Checked (exe)).
const CALLS: [(usize, &[u8]); 10] = [
    (PLACE_RVA, &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40]),
    (RECT_RVA, &[0x8B, 0x41, 0x60, 0x83, 0xE0, 0x14, 0x3C, 0x14]),
    (BUILD_RVA, &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x4D, 0x8B, 0x40, 0x18]),
    (SUBMIT_RVA, &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x45, 0x33, 0xD2, 0x49, 0x8B, 0xD8]),
    (REG_RVA, &[0x49, 0x89, 0x10, 0x49, 0x8D, 0x40, 0x10, 0x49, 0x89, 0x48, 0x08, 0xC3]),
    (GIFTAG_RVA, &[0x48, 0x2B, 0xD1, 0x48, 0xC7, 0x41, 0x08, 0x0E]),
    (SEND_TEX0_RVA, &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0x01]),
    (LOOKUP_RVA, &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF]),
    (BIND_RVA, &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9]),
    (MAP_RVA, &[0x48, 0x8B, 0xC4, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x70]),
];
/// KH1's ALPHA register (`0x42`) values for KH2's blends 0, 1, 2 (`0x140192250`: 0x44 / 0x48 /
/// 0x42), with KH1's FIX 0x80 as its own writes have it (`0x14026CF90`).
const ALPHA: [u64; 3] = [0x80_0000_0044, 0x80_0000_0048, 0x80_0000_0042];

/// KH2's groups (§14.1.3, Checked (decomp) and (exe)).
const BAR: usize = 23;
const FILL: usize = 25;
const MAX: usize = 20;
const NUMBER: usize = 28;
const FORM_BAR: usize = 51;
const FORM_FILL: usize = 49;
const FORM_NUMBER: usize = 54;
/// The group whose attach point the number is placed at (KH2 reads it at the HUD's making, with the
/// bar on its slide-in group 21, `0x14017F9B0`).
const NUMBER_ATTACH: usize = 21;
/// KH2's fill mask (`0x14017E580` / `0x1401816C0`): 50 × 120, centred (−25, −60) from its pivot,
/// the pivot (50, 100) from the bar.
const MASK_SIZE: [f32; 2] = [50.0, 120.0];
const MASK_OFF: [f32; 2] = [-25.0, -60.0];
const MASK_PIVOT: [i32; 2] = [50, 100];
const ANGLE_PER_POINT: f32 = 0.004_014_257_3;
/// KH1 HUD units per KH2 unit (main s322, the owner's measured size) and the place's terms.
const KH2_UNIT: f32 = 384.0 / 448.0 * 1.34;
const KH2_BAR_MIDDLE: f32 = 13.5;

static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static BASE: AtomicUsize = AtomicUsize::new(0);

/// Steam only; installed whatever the Drive row says (the Config menu can switch it on later; the
/// draw asks every frame, `visible`).
pub unsafe fn install(base: usize) {
    if super::addr::Build::current() != super::addr::Build::Steam {
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    for (rva, bytes) in CALLS {
        if std::slice::from_raw_parts((base + rva) as *const u8, bytes.len()) != bytes {
            log!(General, "Drive gauge HUD: unexpected code at 0x{:X}; not drawn", 0x1_4000_0000usize + rva);
            return;
        }
    }
    super::hook::install(base + BODY_RVA, &BODY_BYTES, body as *const () as usize, &ORIGINAL, "KH1's gauge body (Drive gauge HUD)");
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

unsafe extern "system" fn body(record: usize) {
    let orig: unsafe extern "system" fn(usize) = std::mem::transmute(ORIGINAL.load(Ordering::Relaxed));
    let base = BASE.load(Ordering::Relaxed);
    let sora = base != 0 && record == base + SORA_RVA && rd::<i32>(record) == 0;
    let shown = sora && visible();
    if sora {
        label(base, record, shown);
    }
    orig(record);
    if shown {
        let _ = std::panic::catch_unwind(|| draw(base, record));
    }
}

/// Drive on, Form Cost on the gauge, a form owned, the art and its texture ready.
fn visible() -> bool {
    super::settings::get().drive && super::form::gauge_cost() && unsafe { super::formown::owned() } != 0 && super::kh2hud::texture().is_some()
}

// --------------------------------------------------------------------- the place, the label

#[derive(Default)]
struct Layout {
    /// Frames since the last measure.
    age: u32,
    portrait: Option<[i32; 4]>,
    /// KH1's MP HP label's sprites: record, its box, its size (put back when the gauge goes).
    label: Vec<(usize, [i32; 4], [i32; 2])>,
    hidden: bool,
    logged: bool,
}

static LAYOUT: Mutex<Option<Layout>> = Mutex::new(None);

/// Sora's gauge sprites as KH1 draws them (records and boxes), from the two lists his body draws
/// at his place.
unsafe fn sprites(base: usize, g: usize) -> Vec<(usize, [i32; 4])> {
    let place: unsafe extern "system" fn(usize, *mut [i32; 2], *mut [i32; 2]) = std::mem::transmute(base + PLACE_RVA);
    let rect: unsafe extern "system" fn(usize, *mut [i32; 4]) = std::mem::transmute(base + RECT_RVA);
    let (mut pivot, mut at) = ([0i32; 2], [0i32; 2]);
    place(g, &mut pivot, &mut at);
    let mut out = Vec::new();
    for (list, n) in [(g + 0x50, rd::<i32>(g + 0x70).clamp(0, 4)), (g + 0x10, rd::<i32>(g + 0x48).clamp(0, 7))] {
        for k in 0..n as usize {
            let rec = rd::<usize>(list + 8 * k);
            if rec < 0x10000 || !super::crash::readable(rec + 0xBF) {
                continue;
            }
            let mut r = [0i32; 4];
            rect(rec, &mut r);
            if r[2] <= 0 || r[3] <= 0 || r[2] > 400 || r[3] > 400 {
                continue;
            }
            out.push((rec, [at[0] + r[0], at[1] + r[1], at[0] + r[0] + r[2], at[1] + r[1] + r[3]]));
        }
    }
    out
}

/// Once a second: the portrait (the largest of Sora's gauge sprites) and, while the label is
/// shown, the label's sprites; the label hidden or put back with the gauge.
unsafe fn label(base: usize, g: usize, shown: bool) {
    let Ok(mut lock) = LAYOUT.lock() else { return };
    let l = lock.get_or_insert_with(Layout::default);
    l.age += 1;
    if l.portrait.is_none() || l.age >= 60 {
        l.age = 0;
        let s = sprites(base, g);
        let area = |r: &[i32; 4]| (r[2] - r[0]) * (r[3] - r[1]);
        l.portrait = s.iter().map(|(_, r)| *r).max_by_key(area);
        if let (Some(p), false) = (l.portrait, l.hidden) {
            let (pcx, pw) = ((p[0] + p[2]) / 2, p[2] - p[0]);
            l.label = s
                .iter()
                .filter(|(_, r)| {
                    let (w, h) = (r[2] - r[0], r[3] - r[1]);
                    *r != p && w * 10 >= h * 14 && h <= 24 && w <= 120 && (r[0] + r[2]) / 2 < pcx - pw / 4 && r[1] >= p[1] - 10 && r[3] <= p[3] + 10
                })
                .map(|&(rec, r)| (rec, r, [rd::<i32>(rec + 8), rd::<i32>(rec + 0xC)]))
                .collect();
            if !l.logged {
                l.logged = true;
                log!(Forms, "Drive gauge HUD: Sora's portrait {p:?}, KH1's MP HP label {:?} (of his sprites {:?})", l.label.iter().map(|x| x.1).collect::<Vec<_>>(), s.iter().map(|x| x.1).collect::<Vec<_>>());
            }
        }
        // (KH1 may set a size again: hidden again.)
        if l.hidden {
            for &(rec, _, _) in &l.label {
                if rd::<i32>(rec + 8) != 0 {
                    wr(rec + 8, 0i32);
                    wr(rec + 0xC, 0i32);
                }
            }
        }
    }
    if shown != l.hidden {
        l.hidden = shown;
        for &(rec, _, size) in &l.label {
            if rd::<u8>(rec + 0x60) & 0x14 == 0x14 {
                continue;
            }
            let v = if shown { [0, 0] } else { size };
            wr(rec + 8, v[0]);
            wr(rec + 0xC, v[1]);
        }
        log!(Forms, "Drive gauge HUD: {} ({} label sprites {})", if shown { "shown" } else { "not shown" }, l.label.len(), if shown { "hidden" } else { "back" });
    }
}

/// KH2's gauge centre and scale in KH1 HUD units.
fn centre() -> Option<(f32, f32, f32)> {
    let g = LAYOUT.lock().ok()?;
    let l = g.as_ref()?;
    let p = l.portrait?;
    let k = KH2_UNIT;
    let below = 0.3 * (p[2] - p[0]) as f32 / 2.0 + 1.3;
    let cy = match l.label.iter().max_by_key(|x| (x.1[2] - x.1[0]) * (x.1[3] - x.1[1])) {
        Some((_, r, _)) => (r[1] + r[3]) as f32 / 2.0 - KH2_BAR_MIDDLE * k + below,
        None => (p[1] + p[3]) as f32 / 2.0 + below,
    };
    Some(((p[0] + p[2]) as f32 / 2.0 + 6.0, cy, k))
}

// ------------------------------------------------------------------------------- KH2's pieces

#[derive(Default)]
struct Pieces {
    seq: Option<Seq>,
    form: Option<bool>,
    bar: Player,
    fill: Player,
    max: Player,
    number: Player,
    value: i32,
}

static PIECES: Mutex<Option<Pieces>> = Mutex::new(None);

/// What the gauge shows: in a form on the gauge (ticks left, full), or the Drive bars.
struct View {
    form: Option<(f32, f32)>,
    gauge: crate::drivegauge::Gauge,
}

unsafe fn view() -> Option<View> {
    let gauge = super::drivegauge::get()?;
    Some(View { form: if super::form::in_form() { super::form::form_gauge() } else { None }, gauge })
}

/// The quads this frame (KH2 units × 16 around KH2's gauge centre), the fill's in its own list.
fn step(p: &mut Pieces, v: &View, dt: f32) -> Option<(Vec<Quad>, Vec<[seqplay::Vert; 3]>, Vec<Quad>)> {
    let seq = p.seq.as_ref()?;
    let form = v.form.is_some();
    // Form numbers (`0x140180490`) and the mask's angle (`0x1401816C0`).
    let (number, angle) = match v.form {
        Some((left, full)) => {
            let bars = (full / crate::drivegauge::TICKS_PER_BAR).round().max(1.0) as i32;
            let per = (full / bars as f32) as i32;
            if per == 0 {
                (0, 0.0)
            } else {
                let n = left as i32 / per;
                let into = if n == bars { per } else { left as i32 % per };
                (if n == bars { bars - 1 } else { n }, (23.0 / per as f32) * 0.017_453_292 * into as f32)
            }
        }
        None => (v.gauge.bars as i32, if v.gauge.points >= 100 { ANGLE_PER_POINT * 200.0 } else { v.gauge.points as f32 * ANGLE_PER_POINT }),
    };
    let number = number.clamp(0, 9);
    if p.form != Some(form) {
        p.form = Some(form);
        p.bar.restart(seq, if form { FORM_BAR } else { BAR });
        p.fill.restart(seq, if form { FORM_FILL } else { FILL });
        p.number.restart(seq, if form { FORM_NUMBER } else { NUMBER } + number as usize);
        p.value = number;
        p.max.hide();
    }
    if number != p.value {
        p.value = number;
        p.number.restart(seq, if form { FORM_NUMBER } else { NUMBER } + number as usize);
    }
    let full = !form && v.gauge.bars >= v.gauge.max;
    match (full, p.max.group.is_some()) {
        (true, false) => p.max.restart(seq, MAX),
        (false, true) => p.max.hide(),
        _ => {}
    }
    for pl in [&mut p.bar, &mut p.fill, &mut p.max, &mut p.number] {
        pl.step(dt);
    }
    let mut under = Vec::new();
    p.bar.draw(seq, [0, 0], &mut under);
    let b = p.bar.origin;
    let mut fill = Vec::new();
    p.fill.draw(seq, b, &mut fill);
    let mask = seqplay::mask_corners([(b[0] + MASK_PIVOT[0]) as f32, (b[1] + MASK_PIVOT[1]) as f32], angle, MASK_SIZE[0], MASK_SIZE[1], MASK_OFF);
    let tris: Vec<[seqplay::Vert; 3]> = fill.iter().flat_map(|q| seqplay::outside(q, &mask)).collect();
    let mut over = Vec::new();
    p.max.draw(seq, b, &mut over);
    let at = seq.groups.get(NUMBER_ATTACH).map_or([0, 0], |g| g.attach);
    p.number.draw(seq, [b[0] + at[0], b[1] + at[1]], &mut over);
    Some((under, tris, over))
}

// ------------------------------------------------------------------------------- KH1's draw

/// Our sprite record, quad and texture descriptor (KH1 reads them; 16-byte aligned).
fn buffers() -> (usize, usize, usize) {
    static B: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    let b = *B.get_or_init(|| (Box::leak(vec![0u8; 0x400].into_boxed_slice()).as_ptr() as usize + 15) & !15);
    (b, b + 0x100, b + 0x200)
}

struct Kh1 {
    base: usize,
    alpha: f32,
    tex0: u64,
    blend: u32,
    ox: f32,
    oy: f32,
    k: f32,
}

impl Kh1 {
    unsafe fn packet(&self) -> usize {
        rd::<usize>(self.base + PACKET_RVA)
    }

    /// KH1's ALPHA register write, as its body writes it.
    unsafe fn blend(&mut self, b: u32) {
        if b == self.blend {
            return;
        }
        self.blend = b;
        let reg: unsafe extern "system" fn(u64, u64, usize) -> usize = std::mem::transmute(self.base + REG_RVA);
        let tag: unsafe extern "system" fn(usize, usize) = std::mem::transmute(self.base + GIFTAG_RVA);
        let start = self.packet();
        let end = reg(0x42, ALPHA[(b as usize).min(2)], start + 0x10);
        tag(start, end);
        wr(self.base + PACKET_RVA, end);
    }

    /// One quad: KH1's order top left, bottom left, top right, bottom right; positions in KH2
    /// units, texture coordinates × 16, colours r g b a (0x80 = 1).
    unsafe fn quad(&mut self, pos: [[f32; 2]; 4], uv: [[f32; 2]; 4], col: [[f32; 4]; 4], blend: u32) {
        self.blend(blend);
        let (rec, q, desc) = buffers();
        std::ptr::write_bytes(rec as *mut u8, 0, 0xC0);
        wr(rec, [0i32, 0, 1, 1]);
        wr(rec + 0x10, [0i32, 0, 1, 1]);
        wr(desc + 0x18, self.tex0);
        let build: unsafe extern "system" fn(usize, usize, usize) = std::mem::transmute(self.base + BUILD_RVA);
        build(rec, q, desc);
        // (Our picture has no palette: both TEX0 writes ours.)
        wr(q, self.tex0);
        wr(q + 0x10, self.tex0);
        for i in 0..4 {
            wr(q + 0x20 + i * 0x10, [uv[i][0].round() as i32, uv[i][1].round() as i32]);
            let c = col[i];
            wr(q + 0x60 + i * 0x10, [c[0].round() as i32, c[1].round() as i32, c[2].round() as i32, (c[3] * self.alpha).round() as i32]);
            let (x, y) = (self.ox + pos[i][0] * self.k, self.oy + pos[i][1] * self.k);
            wr(q + 0xA0 + i * 0x10, [((x + 0x700 as f32) * 16.0).round() as i32, ((y + 0x740 as f32) * 16.0).round() as i32]);
        }
        let submit: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(self.base + SUBMIT_RVA);
        let p = submit(q, 1, self.packet());
        wr(self.base + PACKET_RVA, p);
    }

    unsafe fn kh2(&mut self, q: &Quad) {
        let f = |i: usize| [q.at[i][0] as f32 / 16.0, q.at[i][1] as f32 / 16.0];
        let u = |i: usize| [q.uv[i][0] as f32, q.uv[i][1] as f32];
        let c = |i: usize| rgba(q.colours[i]);
        // (KH2's top left, top right, bottom left, bottom right to KH1's TL, BL, TR, BR.)
        self.quad([f(0), f(2), f(1), f(3)], [u(0), u(2), u(1), u(3)], [c(0), c(2), c(1), c(3)], q.blend);
    }

    unsafe fn tri(&mut self, t: &[seqplay::Vert; 3]) {
        let p = |v: &seqplay::Vert| [v.x, v.y];
        let u = |v: &seqplay::Vert| [v.u, v.v];
        self.quad([p(&t[0]), p(&t[1]), p(&t[2]), p(&t[2])], [u(&t[0]), u(&t[1]), u(&t[2]), u(&t[2])], [t[0].c, t[1].c, t[2].c, t[2].c], 0);
    }
}

fn rgba(c: u32) -> [f32; 4] {
    [(c & 0xFF) as f32, (c >> 8 & 0xFF) as f32, (c >> 16 & 0xFF) as f32, (c >> 24) as f32]
}

unsafe fn draw(base: usize, record: usize) {
    let Some((tex, tex0)) = super::kh2hud::texture() else { return };
    let Some((ox, oy, k)) = centre() else { return };
    let Some(v) = view() else { return };
    let Ok(mut g) = PIECES.try_lock() else { return };
    let p = g.get_or_insert_with(Pieces::default);
    if p.seq.is_none() {
        let Some(art) = super::kh2hud::art() else { return };
        match Seq::parse(&art.seqd) {
            Ok(s) => {
                log!(Forms, "Drive gauge HUD: KH2's gaug sequence read ({} groups)", s.groups.len());
                p.seq = Some(s);
            }
            Err(e) => {
                log!(Forms, "Drive gauge HUD: KH2's gaug sequence not read ({e})");
                return;
            }
        }
    }
    let dt = rd::<f32>(base + FRAME_SCALE_RVA).clamp(0.0, 5.0);
    let Some((under, tris, over)) = step(p, &v, dt) else { return };
    let alpha = rd::<f32>(record + 0xA9C).clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    // Our texture mapped and set, as KH1 maps and sets its HUD image for this body.
    let map: unsafe extern "system" fn(i32, u32, usize, u32, i32) = std::mem::transmute(base + MAP_RVA);
    map((tex0 & 0x3FFF) as i32, 0, tex, 0, 0);
    let send: unsafe extern "system" fn(*const u64, i32) = std::mem::transmute(base + SEND_TEX0_RVA);
    let lookup: unsafe extern "system" fn(*const u64) -> usize = std::mem::transmute(base + LOOKUP_RVA);
    let bind: unsafe extern "system" fn(usize) = std::mem::transmute(base + BIND_RVA);
    send(&tex0, 0);
    bind(lookup(&tex0));
    let mut kh1 = Kh1 { base, alpha, tex0, blend: 0, ox, oy, k };
    for q in &under {
        kh1.kh2(q);
    }
    for t in &tris {
        kh1.tri(t);
    }
    for q in &over {
        kh1.kh2(q);
    }
    kh1.blend(0);
    // KH1's HUD texture back, for what KH1 draws next.
    let desc = rd::<usize>(record + 0xE0);
    if desc >= 0x10000 {
        let own = rd::<u64>(desc + 0x18);
        send(&own, 0);
        bind(lookup(&own));
    }
}
