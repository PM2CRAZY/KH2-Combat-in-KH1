//! KH2's Drive gauge on KH1's HUD (the user: "Draw KH2's Drive gauge on KH1's HUD near Sora's
//! HP/MP gauge", then, with a mockup: "Can you bring in the actual drive gauge HUD from KH2?
//! ideally it would be positioned around here"). KH2's look, under Sora's HP circle: the orange
//! DRIVE bar (its label at the left, the current bar's points filling it), the white MAX plate
//! over it when the gauge is full, and the yellow slanted cap at the right end with the number
//! of full bars. In a form it is the form gauge: the bar in gold, draining, and KH2's number (the
//! full bars left, the one draining not counted).
//!
//! **Drawn with KH1's own HUD primitives, inside Sora's HP/MP gauge task** (`0x14026B990`, hooked
//! here; the s318 gauge drew in the command-menu task `0x140279700`, which KH1 runs *before* this
//! one: both are priority 170001 and the HUD setup `0x140264C50` registers the command menu
//! first, so KH1 drew its HP/MP gauge over it). Each piece is a quad of KH1's sprite path: a copy
//! of a Drive list row's panel record (the summon list's layout `0x14285A260`, its texture the
//! shared HUD texture), its corners and colours set here (`0x14027B300` builds the quad, the
//! corners are moved to make the slanted plates, `0x140274340` draws it at once through KH1's
//! immediate draw); the words and the number with KH1's text draw (`0x1402CFB20`, as the Drive
//! list's title). It follows Sora's gauge: drawn only when KH1 drew it (not in events: KH1's
//! event bit `0x142867CD8` & 8), with its alpha (`+0xA9C` of his gauge record), and the HUD hide
//! (`hud`, Page Down) empties it with the rest of that task's draws.
//!
//! **Where**: under Sora's gauge as KH1 draws it: the bounding box of his gauge's sprites (his
//! record `0x142812400`: its place from `0x14026B820`, which already holds the wide screen's
//! shift, plus the rectangle of each sprite of the two lists KH1's gauge draws put there), the
//! right end a little past its right edge and the bar's top just over its bottom. `drive_gauge = dx, dy` in
//! the ini moves it from there (HUD units: 384 is the screen's height); `drive_gauge_debug = on`
//! draws a bright magenta block over the whole gauge, to find it.
//!
//! KH2's own art (its HUD 2D files) isn't used: KH1's sprite path can only sample a texture KH1
//! itself registered (its texture map, keyed by the PS2 texture address, filled from its own
//! files), so KH2's images would need a texture made in KH1's renderer first.

use super::addr::Rva;
use super::{hook, log};
use std::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::OnceLock;

/// KH1's HP/MP gauge task (`hud::TASKS`' third).
const TASK_RVA: Rva = Rva(0x26B990);
/// `mov [rsp+0x18], rbx; push rbp; push rsi; push rdi`.
const TASK_PROLOGUE: [u8; 8] = [0x48, 0x89, 0x5C, 0x24, 0x18, 0x55, 0x56, 0x57];
/// Sora's gauge record (slot 0; slot n at `+ n × 0xDF0`) and its bound flag.
const SORA_GAUGE_RVA: Rva = Rva(0x2812400);
const SORA_BOUND_RVA: Rva = Rva(0x28123E0);
/// A gauge record's place `(record, &pivot, &place)` (the wide screen's shift included).
const GAUGE_PLACE_RVA: Rva = Rva(0x26B820);
/// A sprite record's rectangle `(record, &[x, y, w, h])`.
const RECT_RVA: Rva = Rva(0x27C2C0);
/// KH1's flags: bit 3 set in events (Sora's gauge isn't drawn then).
const FLAGS_RVA: Rva = Rva(0x2867CD8);
/// The wide screen's extra HUD units each side (KH1 adds it to the right side's pieces).
const WIDE_RVA: Rva = Rva(0x232E6D0);
/// The list draw's set-up (`0x140277F40`): the HUD's draw state block and its two calls.
const STATE_RVA: Rva = Rva(0x2867450);
const STATE_INIT_RVA: Rva = Rva(0x275460);
const STATE_SEND_RVA: Rva = Rva(0x27B020);
/// A sprite record into a quad `(record, quad, texture)`, the quad KH1's sprite draws use, and the
/// quad's draw `(quad, 1, packet) -> packet`.
const BUILD_RVA: Rva = Rva(0x27B300);
const QUAD_RVA: Rva = Rva(0x2867360);
const SUBMIT_RVA: Rva = Rva(0x274340);
/// The packet pointer and the buffer the list draw starts it at.
const PACKET_RVA: Rva = Rva(0x2853218);
const PACKET_BUF_RVA: Rva = Rva(0x2EE9D10);
/// The sprite draw's alpha gate (nothing is drawn while it is 0) and its two colour scales.
const GATE_RVA: Rva = Rva(0x528820);
const SCALE_A_RVA: Rva = Rva(0x2867440);
const SCALE_B_RVA: Rva = Rva(0x2867444);
/// The summon list's layout (`drive`'s): `+0` its texture, `+0x28` its row panels (0xC0 each).
const LAYOUT_RVA: Rva = Rva(0x285A260);
/// Text: `(packet, 1, rgba, x, y, text) -> packet`, then run from the start (`drive`'s title).
const TEXT_RVA: Rva = Rva(0x2CFB20);
const TEXT_RUN_RVA: Rva = Rva(0x3028F0);
const FONT_H_RVA: Rva = Rva(0x54603C);
const TEXT_WIDTH_RVA: Rva = Rva(0x2CEC60);

/// KH1's colours run 0..0x80 for the texture's own (0x80 = as it is, up to 0xFF = twice).
const BLACK: [i32; 4] = [0, 0, 0, 0x80];
const EMPTY: [i32; 4] = [0x38, 0x1C, 0x08, 0x80];
const DRIVE_TOP: [i32; 4] = [0xFF, 0xB8, 0x30, 0x80];
const DRIVE_BOTTOM: [i32; 4] = [0xF8, 0x70, 0x00, 0x80];
const FORM_TOP: [i32; 4] = [0xFF, 0xF0, 0x90, 0x80];
const FORM_BOTTOM: [i32; 4] = [0xE8, 0xB8, 0x30, 0x80];
const WHITE: [i32; 4] = [0xFF, 0xFF, 0xFF, 0x80];
const CAP_TOP: [i32; 4] = [0xFF, 0xF8, 0x40, 0x80];
const CAP_BOTTOM: [i32; 4] = [0xFF, 0xC0, 0x00, 0x80];
const MAGENTA: [i32; 4] = [0xFF, 0x00, 0xFF, 0x80];
/// Text colours (bytes; alpha 0x80 = opaque).
const LABEL_TEXT: [u8; 4] = [0xFF, 0x98, 0x20, 0x80];
const MAX_TEXT: [u8; 4] = [0x10, 0x10, 0x10, 0x80];
const NUMBER_TEXT: [u8; 4] = [0xFF, 0x78, 0x00, 0x80];
const OUTLINE: [u8; 4] = [0x00, 0x00, 0x00, 0x80];

/// The gauge's shape in HUD units, from its right end `R` and its bar's top `T`: the bar 12 high;
/// the cap `CAP_W` wide from `T - 10` to `T + 14`, leaning by `SLANT`; the MAX plate over the bar
/// left of the cap, `T - 13` to `T + 1`.
const BAR_H: i32 = 12;
const CAP_W: i32 = 24;
const SLANT: i32 = 6;
const PLATE_W: i32 = 38;
/// The gauge's width when Sora's gauge can't be measured, and the limits on the measured one.
const WIDTH: i32 = 130;
const MIN_W: i32 = 100;
const MAX_W: i32 = 170;
/// From Sora's gauge's box: the right end this far past its right edge, the bar's top this far
/// above its bottom.
const PAST_RIGHT: i32 = 4;
const OVERLAP: i32 = 2;
/// The HUD's height in its units (the bottom edge of the screen).
const HUD_H: i32 = 384;

static ORIG_TASK: AtomicUsize = AtomicUsize::new(0);
static RECORD: OnceLock<usize> = OnceLock::new();
/// "0".."9", "MAX", "DRIVE" in KH1's text codes.
static WORDS: OnceLock<Vec<usize>> = OnceLock::new();
/// Frames drawn, for the debug line every few seconds, and the last box logged.
static FRAMES: AtomicU32 = AtomicU32::new(0);
static QUADS: AtomicU32 = AtomicU32::new(0);
static LAST_BOX: [AtomicI32; 4] = [const { AtomicI32::new(i32::MIN) }; 4];
static SKIPPED: AtomicU32 = AtomicU32::new(0);
/// The frame of the last debug line (a moved gauge is logged at most once a second).
static LAST_LOG: AtomicU32 = AtomicU32::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}
fn base() -> usize {
    unsafe { super::sys::GetModuleHandleA(std::ptr::null()) as usize }
}

type TaskFn = unsafe extern "system" fn(usize) -> usize;

/// KH1's HP/MP gauge task, then the Drive gauge over it.
unsafe extern "system" fn task(node: usize) -> usize {
    let f: TaskFn = std::mem::transmute(ORIG_TASK.load(Ordering::Acquire));
    let r = f(node);
    let _ = std::panic::catch_unwind(|| draw());
    r
}

/// Where KH1 draws Sora's gauge this frame: the box `[x0, y0, x1, y1]` of its sprites, in HUD
/// units. `None` when his slot isn't bound or the records aren't as expected.
unsafe fn sora_box(base: usize) -> Option<[i32; 4]> {
    let g = base + SORA_GAUGE_RVA;
    if rd::<u32>(base + SORA_BOUND_RVA) == 0 || rd::<u32>(g) > 4 {
        return None;
    }
    let place: unsafe extern "system" fn(usize, *mut [i32; 2], *mut [i32; 2]) = std::mem::transmute(base + GAUGE_PLACE_RVA);
    let rect: unsafe extern "system" fn(usize, *mut [i32; 4]) = std::mem::transmute(base + RECT_RVA);
    let (mut pivot, mut at) = ([0i32; 2], [0i32; 2]);
    place(g, &mut pivot, &mut at);
    let mut b = [i32::MAX, i32::MAX, i32::MIN, i32::MIN];
    // (The two lists KH1's gauge draws put at that place: `+0x50`, `+0x70` of them, by
    // `0x140267020`; `+0x10`, `+0x48` of them, by `0x14026AD20`.)
    let lists = [(g + 0x50, rd::<i32>(g + 0x70).clamp(0, 4)), (g + 0x10, rd::<i32>(g + 0x48).clamp(0, 7))];
    for (list, n) in lists {
        for k in 0..n as usize {
            let rec = rd::<u64>(list + 8 * k) as usize;
            if rec < 0x10000 || !super::crash::readable(rec + 0xBF) {
                continue;
            }
            let mut r = [0i32; 4];
            rect(rec, &mut r);
            if r[2] <= 0 || r[3] <= 0 || r[2] > 400 || r[3] > 400 {
                continue;
            }
            b = [b[0].min(at[0] + r[0]), b[1].min(at[1] + r[1]), b[2].max(at[0] + r[0] + r[2]), b[3].max(at[1] + r[1] + r[3])];
        }
    }
    (b[0] < b[2] && (-300..900).contains(&b[0]) && (-300..900).contains(&b[2]) && (0..HUD_H + 40).contains(&b[3])).then_some(b)
}

/// One quad: the panel record copied (the middle of its picture, so a flat block of the colour),
/// its corners `[top-left, bottom-left, top-right, bottom-right]` and their colours, drawn now.
unsafe fn quad(base: usize, template: usize, texture: usize, at: [(i32, i32); 4], colour: [[i32; 4]; 4], alpha: f32) {
    let rec = *RECORD.get_or_init(|| {
        let b = Box::leak(vec![0u8; 0xC0 + 16].into_boxed_slice()).as_ptr() as usize;
        (b + 15) & !15
    });
    std::ptr::copy_nonoverlapping(template as *const u8, rec as *mut u8, 0xC0);
    wr(rec, [0i32, 0, 1, 1]);
    // The picture's inner half both ways (the panel's body, not its rim).
    let uv = rd::<[i32; 4]>(rec + 0x10);
    let (du, dv) = ((uv[2] - uv[0]) / 4, (uv[3] - uv[1]) / 4);
    wr(rec + 0x10, [uv[0] + du, uv[1] + dv, uv[2] - du, uv[3] - dv]);
    // A still record (no animated rectangle or colour), one colour per corner.
    wr(rec + 0x60, (rd::<u32>(rec + 0x60) & !(0x10 | 0x80 | 0x08)) | 0x20);
    for (v, c) in colour.iter().enumerate() {
        wr(rec + 0x20 + v * 0x10, *c);
    }
    let q = base + QUAD_RVA;
    let build: unsafe extern "system" fn(usize, usize, usize) = std::mem::transmute(base + BUILD_RVA);
    build(rec, q, texture);
    // Corners: KH1's quad keeps them at `+0xA0 + v × 0x10` as (x + 0x700, y + 0x740) × 16.
    for (v, (x, y)) in at.iter().enumerate() {
        wr(q + 0xA0 + v * 0x10, (x + 0x700) << 4);
        wr(q + 0xA4 + v * 0x10, (y + 0x740) << 4);
    }
    for v in 0..4 {
        let a = q + 0x6C + v * 0x10;
        wr(a, (rd::<i32>(a) as f32 * alpha) as i32);
    }
    if rd::<f32>(base + GATE_RVA) == 0.0 {
        return;
    }
    let submit: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + SUBMIT_RVA);
    wr(base + PACKET_RVA, submit(q, 1, rd::<u64>(base + PACKET_RVA) as usize) as u64);
    QUADS.fetch_add(1, Ordering::Relaxed);
}

/// A box, its two top corners one colour and its two bottom corners another, the top edge moved
/// right by `lean`.
#[allow(clippy::too_many_arguments)]
unsafe fn block(base: usize, t: usize, tex: usize, x0: i32, y0: i32, x1: i32, y1: i32, lean: i32, top: [i32; 4], bottom: [i32; 4], alpha: f32) {
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    quad(base, t, tex, [(x0 + lean, y0), (x0, y1), (x1 + lean, y0), (x1, y1)], [top, bottom, top, bottom], alpha);
}

/// A word's width in HUD units.
unsafe fn width(base: usize, word: usize) -> i32 {
    let height = rd::<i32>(base + FONT_H_RVA);
    if !(8..=32).contains(&height) {
        return 0;
    }
    let measure: unsafe extern "system" fn(usize, i32) -> i32 = std::mem::transmute(base + TEXT_WIDTH_RVA);
    measure(word, height)
}

/// A word with its left end at `x` and its middle at `y`, in `colour`, ringed in black if
/// `outline`.
unsafe fn text(base: usize, word: usize, x: i32, y: i32, colour: [u8; 4], outline: bool, alpha: f32) {
    let start = rd::<u64>(base + PACKET_RVA) as usize;
    if start < 0x10000 {
        return;
    }
    let put: unsafe extern "system" fn(usize, i32, u32, i32, i32, usize) -> usize = std::mem::transmute(base + TEXT_RVA);
    let run: unsafe extern "system" fn(usize) = std::mem::transmute(base + TEXT_RUN_RVA);
    let rgba = |c: [u8; 4]| u32::from_le_bytes([c[0], c[1], c[2], (c[3] as f32 * alpha).clamp(0.0, 255.0) as u8]);
    // (KH1's text call puts the word's middle at its y + 7.)
    let y = y - 7;
    let mut end = start;
    if outline {
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1), (-1, 1), (1, -1)] {
            end = put(end, 1, rgba(OUTLINE), x + dx, y + dy, word);
        }
    }
    end = put(end, 1, rgba(colour), x, y, word);
    wr(base + PACKET_RVA, end as u64);
    run(start);
}

unsafe fn draw() {
    let base = base();
    let settings = super::settings::get();
    // Only with Sora's gauge: not in events (KH1 draws none then), and with its alpha.
    if rd::<u32>(base + FLAGS_RVA) & 8 != 0 {
        return;
    }
    let g = base + SORA_GAUGE_RVA;
    let alpha = if rd::<u32>(base + SORA_BOUND_RVA) != 0 { rd::<f32>(g + 0xA9C) } else { 1.0 };
    if !(0.01..=1.5).contains(&alpha) {
        SKIPPED.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let alpha = alpha.min(1.0);
    let Some(s) = super::gauge::snapshot() else { return };
    let layout = base + LAYOUT_RVA;
    let texture = rd::<u64>(layout) as usize;
    let rows = rd::<u64>(layout + 0x28) as usize;
    if texture < 0x10000 || rows < 0x10000 || !super::crash::readable(texture + 0x20) || !super::crash::readable(rows + 0xBF) {
        if SKIPPED.fetch_add(1, Ordering::Relaxed) == 0 {
            log(&format!("Drive gauge HUD: the Drive list's panel isn't loaded (texture {texture:#x}, rows {rows:#x}): not drawn"));
        }
        return;
    }
    // What it shows: the current bar's share (1 when full), the number, MAX.
    let (frac, number, max, form) = match s.form {
        Some(f) => {
            let left = f.left.max(0.0);
            let full = (left / 600.0).floor();
            let frac = if left >= f.max { 1.0 } else { (left - full * 600.0) / 600.0 };
            let bars = (f.max / 600.0).round().max(1.0) as i32;
            let number = if left >= f.max { (bars - 1).max(0) } else { full as i32 };
            (frac, number, false, true)
        }
        None => {
            let full = s.bars >= s.max.max(1);
            (if full { 1.0 } else { s.points.min(100) as f32 / 100.0 }, s.bars as i32, full, false)
        }
    };
    // Where: under Sora's gauge as KH1 draws it.
    let wide = rd::<i32>(base + WIDE_RVA).clamp(0, 400);
    let measured = sora_box(base);
    let (r0, t0, w) = match measured {
        Some(b) => (b[2] + PAST_RIGHT, b[3] - OVERLAP, (b[2] - b[0]).clamp(MIN_W, MAX_W)),
        // (KH1's pivot for Sora's gauge, `0x140527FB0`, plus the wide screen's shift.)
        None => (443 + wide + 48, 316 + 40, WIDTH),
    };
    let (dx, dy) = settings.drive_gauge;
    let r = (r0 + dx).min(512 + wide - 2);
    let t = (t0 + dy).min(HUD_H - BAR_H - 4);
    let left = r - w;
    // KH1's list draw's set-up, its globals put back after.
    let saved = (rd::<u64>(base + PACKET_RVA), rd::<f32>(base + GATE_RVA), rd::<f32>(base + SCALE_A_RVA), rd::<f32>(base + SCALE_B_RVA));
    let state = base + STATE_RVA;
    let init: unsafe extern "system" fn(usize) = std::mem::transmute(base + STATE_INIT_RVA);
    let send: unsafe extern "system" fn(usize) = std::mem::transmute(base + STATE_SEND_RVA);
    init(state);
    wr(state + 0xE0, 5u64);
    wr(state + 0x60, 0x30003u64);
    send(state);
    wr(base + PACKET_RVA, (base + PACKET_BUF_RVA) as u64);
    wr(base + GATE_RVA, alpha);
    wr(base + SCALE_A_RVA, alpha);
    wr(base + SCALE_B_RVA, alpha);
    let counted = super::hud::count_start();
    let q0 = QUADS.load(Ordering::Relaxed);
    let words = WORDS.get_or_init(|| {
        let mut v: Vec<usize> = (0..10).map(|d| Box::leak(super::menu::encode(&d.to_string()).into_boxed_slice()).as_ptr() as usize).collect();
        for w in ["MAX", "DRIVE"] {
            v.push(Box::leak(super::menu::encode(w).into_boxed_slice()).as_ptr() as usize);
        }
        v
    });
    let (tx, rw) = (texture, rows);
    // The bar: a black band, the label's box at its left, the empty part, then the fill.
    let label_w = (width(base, words[11]) + 8).clamp(30, 60);
    let bar_r = r - CAP_W + SLANT;
    block(base, rw, tx, left - 1, t - 1, bar_r, t + BAR_H + 1, 0, BLACK, BLACK, alpha);
    let (fx0, fx1) = (left + label_w, bar_r - 1);
    block(base, rw, tx, fx0, t + 1, fx1, t + BAR_H - 1, 0, EMPTY, EMPTY, alpha);
    let fill = ((fx1 - fx0) as f32 * frac.clamp(0.0, 1.0)).round() as i32;
    let (top, bottom) = if form { (FORM_TOP, FORM_BOTTOM) } else { (DRIVE_TOP, DRIVE_BOTTOM) };
    block(base, rw, tx, fx0, t + 1, fx0 + fill, t + BAR_H - 1, 0, top, bottom, alpha);
    // MAX: the white plate over the bar, left of the cap.
    if max {
        let (px1, px0) = (r - CAP_W - 2, r - CAP_W - 2 - PLATE_W);
        block(base, rw, tx, px0 - 1, t - 14, px1 + 2, t + 2, 4, BLACK, BLACK, alpha);
        block(base, rw, tx, px0, t - 13, px1, t + 1, 4, WHITE, WHITE, alpha);
    }
    // The cap: yellow, leaning, ringed in black.
    let (cx0, cy0, cy1) = (r - CAP_W, t - 10, t + BAR_H + 2);
    block(base, rw, tx, cx0 - 2, cy0 - 2, r + 1, cy1 + 2, SLANT, BLACK, BLACK, alpha);
    block(base, rw, tx, cx0, cy0, r - 1, cy1, SLANT, CAP_TOP, CAP_BOTTOM, alpha);
    // The words: DRIVE in its box, MAX on its plate, the number in the cap.
    text(base, words[11], left + 4, t + BAR_H / 2, LABEL_TEXT, false, alpha);
    if max {
        let px0 = r - CAP_W - 2 - PLATE_W;
        let mw = width(base, words[10]);
        text(base, words[10], px0 + 2 + (PLATE_W - mw) / 2, t - 6, MAX_TEXT, false, alpha);
    }
    let digit = words[number.clamp(0, 9) as usize];
    let nw = width(base, digit);
    text(base, digit, cx0 + SLANT / 2 + (CAP_W - nw) / 2, (cy0 + cy1) / 2, NUMBER_TEXT, true, alpha);
    // The finder: a bright block over the whole gauge.
    if settings.drive_gauge_debug {
        block(base, rw, tx, left - 2, t - 16, r + 4, t + BAR_H + 6, 0, MAGENTA, MAGENTA, 1.0);
    }
    let (asked, drawn) = super::hud::count_end();
    wr(base + PACKET_RVA, saved.0);
    wr(base + GATE_RVA, saved.1);
    wr(base + SCALE_A_RVA, saved.2);
    wr(base + SCALE_B_RVA, saved.3);
    // The proof, every few seconds: where, how big, the alpha, what KH1 drew.
    let n = FRAMES.fetch_add(1, Ordering::Relaxed);
    let bx = [left, t - 13, r, t + BAR_H + 4];
    let moved = (0..4).any(|i| LAST_BOX[i].load(Ordering::Relaxed) != bx[i]);
    let last = LAST_LOG.load(Ordering::Relaxed);
    if n % 180 == 0 || (moved && n.wrapping_sub(last) >= 60) {
        LAST_LOG.store(n, Ordering::Relaxed);
        for (i, v) in bx.iter().enumerate() {
            LAST_BOX[i].store(*v, Ordering::Relaxed);
        }
        let quads = QUADS.load(Ordering::Relaxed).wrapping_sub(q0);
        let from = match measured {
            Some(b) => format!("Sora's gauge drawn at [{}, {}, {}, {}]", b[0], b[1], b[2], b[3]),
            None => format!("Sora's gauge not measured: KH1's pivot (443, 316) + wide {wide}"),
        };
        log(&format!(
            "Drive gauge HUD (frame {n}): box [{}, {}, {}, {}] in HUD units ({from}; ini offset {dx}, {dy}), alpha {alpha:.2}, {quads} quads + {} words; KH1's immediate draw got {} vertices, drew {}{}; {}{}{}",
            bx[0],
            bx[1],
            bx[2],
            bx[3],
            if max { 3 } else { 2 },
            if counted { asked.to_string() } else { "?".into() },
            if counted { drawn.to_string() } else { "?".into() },
            if counted && drawn == 0 && asked > 0 { " (the HUD is hidden)" } else { "" },
            if form { format!("form gauge {:.0}% of a bar, {number} left", frac * 100.0) } else { format!("{} bars + {:.0}%{}", number, frac * 100.0, if max { " (MAX)" } else { "" }) },
            match SKIPPED.swap(0, Ordering::Relaxed) {
                0 => String::new(),
                k => format!("; {k} frames not drawn (Sora's gauge faded out)"),
            },
            if n == 0 { format!("; panel record {rows:#x} flags {:#x} uv {:?} colour {:?}, texture {texture:#x}", rd::<u32>(rows + 0x60), rd::<[i32; 4]>(rows + 0x10), rd::<[i32; 4]>(rows + 0x20)) } else { String::new() }
        ));
    }
}

pub unsafe fn install() {
    if !super::settings::get().drive {
        return;
    }
    let base = base();
    if hook::install(base + TASK_RVA, &TASK_PROLOGUE, task as *const () as usize, &ORIG_TASK, "HP/MP gauge task (Drive gauge HUD)").is_none() {
        log("Drive gauge HUD: KH1's HP/MP gauge task wasn't as expected: no gauge on the HUD");
        return;
    }
    let s = super::settings::get();
    log(&format!(
        "Drive gauge HUD: drawn after Sora's HP/MP gauge, under it (offset {}, {} from there){}",
        s.drive_gauge.0,
        s.drive_gauge.1,
        if s.drive_gauge_debug { "; drive_gauge_debug: a magenta block over it" } else { "" }
    ));
}
