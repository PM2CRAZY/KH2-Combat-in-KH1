//! KH2's Drive gauge on KH1's HUD (the user: "Draw KH2's Drive gauge on KH1's HUD near Sora's
//! HP/MP gauge"): KH2's segmented bar, one segment per bar of the maximum, the full ones lit and
//! the current one filled by its points, and the number of full bars beside it ("MAX" when the
//! gauge is full), as KH2 shows them. In a form it is the form gauge, draining, its segments the
//! form gauge's bars and its number KH2's (the full bars left, the current one not counted).
//!
//! Drawn with KH1's own HUD primitives inside KH1's command-menu HUD task (`0x140279700`, hooked
//! here): its sprite draw (`0x140278880 (record, texture, x, y, alpha, 0, 0)`) with a copy of a
//! list row's panel record (the summon list's layout, `0x14285A260`, which is always loaded on the
//! field), recoloured and sized for each segment, and its text draw (`0x1402CFB20`, as the Drive
//! list's title), after the same set-up its list draw makes (`0x140277F40`: `0x140275460` and
//! `0x14027B020` on `0x142867450`, the packet at `0x142EE9D10`). So it fades with the command menu
//! (its alpha `0x14285320C`: gone in events, KH1's own HUD fades) and the HUD hide (`hud`, Page
//! Down) empties it with the rest of that task's draws. Where: `drive_gauge = x, y` in the ini
//! (the gauge's right end and top, in KH1's HUD units: 512 × 384 for the 4:3 middle, the wide
//! screen's sides beyond 0 and 512), under Sora's HP and MP by default.

use super::addr::Rva;
use super::{hook, log};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::OnceLock;

/// KH1's command-menu HUD task (`hud::TASKS`' last).
const TASK_RVA: Rva = Rva(0x279700);
/// `push rbx; push rbp; push rdi; push r12; push r15`.
const TASK_PROLOGUE: [u8; 8] = [0x40, 0x53, 0x55, 0x57, 0x41, 0x54, 0x41, 0x57];
/// The list draw's set-up (`0x140277F40`): the HUD's draw state block and its two calls.
const STATE_RVA: Rva = Rva(0x2867450);
const STATE_INIT_RVA: Rva = Rva(0x275460);
const STATE_SEND_RVA: Rva = Rva(0x27B020);
/// The sprite draw.
const SPRITE_RVA: Rva = Rva(0x278880);
/// The packet pointer and the buffer the list draw starts it at.
const PACKET_RVA: Rva = Rva(0x2853218);
const PACKET_BUF_RVA: Rva = Rva(0x2EE9D10);
/// The sprite draw's alpha gate (nothing is drawn while it is 0) and its two colour scales.
const GATE_RVA: Rva = Rva(0x528820);
const SCALE_A_RVA: Rva = Rva(0x2867440);
const SCALE_B_RVA: Rva = Rva(0x2867444);
/// The command menu's fade (0..1).
const MENU_ALPHA_RVA: Rva = Rva(0x285320C);
/// The summon list's layout (`drive`'s): `+0` its texture, `+0x28` its row panels (0xC0 each).
const LAYOUT_RVA: Rva = Rva(0x285A260);
/// Text: `(packet, 1, rgba, x, y, text) -> packet`, then run from the start (`drive`'s title).
const TEXT_RVA: Rva = Rva(0x2CFB20);
const TEXT_RUN_RVA: Rva = Rva(0x3028F0);
const FONT_H_RVA: Rva = Rva(0x54603C);
const TEXT_WIDTH_RVA: Rva = Rva(0x2CEC60);

/// A segment, in HUD units, and the gap between two.
const SEG_W: i32 = 12;
const SEG_H: i32 = 7;
const GAP: i32 = 2;
/// Room for the number at the right end.
const NUMBER_W: i32 = 22;
/// KH1's colours run 0..0x80 (0x80 full).
const EMPTY: [i32; 4] = [0x14, 0x12, 0x10, 0x50];
/// KH2's Drive gauge segments: yellow-orange; the form gauge's: a paler gold.
const DRIVE: [i32; 4] = [0x80, 0x5A, 0x0C, 0x80];
const FORM: [i32; 4] = [0x80, 0x76, 0x3C, 0x80];
const NUMBER: [u8; 4] = [0xFF, 0xD8, 0x50, 0x80];

static ORIG_TASK: AtomicUsize = AtomicUsize::new(0);
static RECORD: OnceLock<usize> = OnceLock::new();
/// "0".."9" and "MAX" in KH1's text codes.
static WORDS: OnceLock<Vec<usize>> = OnceLock::new();
static LOGGED: AtomicU32 = AtomicU32::new(0);

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

/// KH1's command-menu task, then the gauge.
unsafe extern "system" fn task(node: usize) -> usize {
    let f: TaskFn = std::mem::transmute(ORIG_TASK.load(Ordering::Acquire));
    let r = f(node);
    draw();
    r
}

/// One segment: the panel record copied, its rectangle `w × h` and its colour, drawn at `(x, y)`;
/// `solid`: from the panel's middle texel only (its `+0x10` texture rectangle shrunk to it), a
/// plain block of the colour, else the whole panel (its frame) squeezed into it.
#[allow(clippy::too_many_arguments)]
unsafe fn segment(base: usize, template: usize, texture: usize, x: i32, y: i32, w: i32, h: i32, colour: [i32; 4], alpha: f32, solid: bool) {
    if w <= 0 || h <= 0 {
        return;
    }
    let rec = *RECORD.get_or_init(|| {
        let b = Box::leak(vec![0u8; 0xC0 + 16].into_boxed_slice()).as_ptr() as usize;
        (b + 15) & !15
    });
    std::ptr::copy_nonoverlapping(template as *const u8, rec as *mut u8, 0xC0);
    wr(rec, [0i32, 0, w, h]);
    if solid {
        let uv = rd::<[i32; 4]>(rec + 0x10);
        let (cu, cv) = ((uv[0] + uv[2]) / 2, (uv[1] + uv[3]) / 2);
        wr(rec + 0x10, [cu, cv, cu, cv]);
    }
    // A still record (no animated rectangle or colour), one colour per corner.
    wr(rec + 0x60, (rd::<u32>(rec + 0x60) & !(0x10 | 0x80 | 0x08)) | 0x20);
    for v in 0..4 {
        wr(rec + 0x20 + v * 0x10, colour);
    }
    let draw: unsafe extern "system" fn(usize, usize, i32, i32, f32, i32, i32) = std::mem::transmute(base + SPRITE_RVA);
    draw(rec, texture, x, y, alpha, 0, 0);
}

/// A text at `(x, y)` (its middle at y + 7), right-aligned to `x` if `right`.
unsafe fn text(base: usize, word: usize, x: i32, y: i32, right: bool, colour: [u8; 4], alpha: f32) {
    let height = rd::<i32>(base + FONT_H_RVA);
    let x = if right && (8..=32).contains(&height) {
        let measure: unsafe extern "system" fn(usize, i32) -> i32 = std::mem::transmute(base + TEXT_WIDTH_RVA);
        x - measure(word, height)
    } else {
        x
    };
    let start = rd::<u64>(base + PACKET_RVA) as usize;
    if start < 0x10000 {
        return;
    }
    let a = (colour[3] as f32 * alpha).clamp(0.0, 255.0) as u8;
    let put: unsafe extern "system" fn(usize, i32, u32, i32, i32, usize) -> usize = std::mem::transmute(base + TEXT_RVA);
    let run: unsafe extern "system" fn(usize) = std::mem::transmute(base + TEXT_RUN_RVA);
    let end = put(start, 1, u32::from_le_bytes([colour[0], colour[1], colour[2], a]), x, y, word);
    wr(base + PACKET_RVA, end as u64);
    run(start);
}

unsafe fn draw() {
    let base = base();
    let alpha = rd::<f32>(base + MENU_ALPHA_RVA);
    if !(0.01..=1.5).contains(&alpha) {
        return;
    }
    let Some(s) = super::gauge::snapshot() else { return };
    let layout = base + LAYOUT_RVA;
    let texture = rd::<u64>(layout) as usize;
    let rows = rd::<u64>(layout + 0x28) as usize;
    if texture < 0x10000 || rows < 0x10000 || !super::crash::readable(texture) || !super::crash::readable(rows + 0xBF) {
        return;
    }
    // (An animated panel's colours aren't the record's own: skip, as `drive`'s whitening does.)
    let flags = rd::<u32>(rows + 0x60);
    if flags & 0xA == 0xA || flags & 0xC0 == 0xC0 {
        if LOGGED.fetch_add(1, Ordering::Relaxed) == 0 {
            log(&format!("Drive gauge HUD: the panel record is animated (flags {flags:#x}): not drawn"));
        }
        return;
    }
    // KH2's segments: the Drive gauge's bars, or the form gauge's in a form.
    let (count, full, part, colour, number) = match s.form {
        Some(g) => {
            let bars = (g.max / 600.0).round().max(1.0) as i32;
            let left = g.left.max(0.0);
            let full = (left / 600.0).floor() as i32;
            let part = ((left - full as f32 * 600.0) / 600.0 * SEG_W as f32).round() as i32;
            // (KH2's number: the full bars left, the one draining not counted.)
            (bars, full.min(bars), part, FORM, if left >= g.max { (bars - 1).max(0) } else { full })
        }
        None => {
            let max = s.max.max(1) as i32;
            let full = (s.bars as i32).min(max);
            let part = if full >= max { 0 } else { (s.points as i32 * SEG_W + 50) / 100 };
            (max, full, part, DRIVE, if full >= max { -1 } else { full })
        }
    };
    let (right, top) = super::settings::get().drive_gauge;
    let bar_right = right - NUMBER_W;
    let left = bar_right - count * (SEG_W + GAP) + GAP;
    // The list draw's set-up, its globals put back after.
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
    wr(base + SCALE_A_RVA, 1.0f32);
    wr(base + SCALE_B_RVA, 1.0f32);
    // Each segment: its frame (the panel squeezed in, dark), then its fill one unit in.
    for k in 0..count {
        let x = left + k * (SEG_W + GAP);
        segment(base, rows, texture, x, top, SEG_W, SEG_H, EMPTY, alpha, false);
        let fill = if k < full { SEG_W } else if k == full { part.min(SEG_W) } else { 0 };
        if fill > 0 {
            segment(base, rows, texture, x + 1, top + 1, (fill - 2).max(1), SEG_H - 2, colour, alpha, true);
        }
    }
    let words = WORDS.get_or_init(|| {
        let mut v: Vec<usize> = (0..10).map(|d| Box::leak(super::menu::encode(&d.to_string()).into_boxed_slice()).as_ptr() as usize).collect();
        v.push(Box::leak(super::menu::encode("MAX").into_boxed_slice()).as_ptr() as usize);
        v
    });
    let word = if number < 0 { words[10] } else { words[(number as usize).min(9)] };
    // (Its middle on the bar's: the text call's middle is at y + 7.)
    text(base, word, right, top + SEG_H / 2 - 7, true, NUMBER, alpha);
    wr(base + PACKET_RVA, saved.0);
    wr(base + GATE_RVA, saved.1);
    wr(base + SCALE_A_RVA, saved.2);
    wr(base + SCALE_B_RVA, saved.3);
    if LOGGED.fetch_add(1, Ordering::Relaxed) == 0 {
        log(&format!(
            "Drive gauge HUD: {count} segments at ({left}, {top}) to ({bar_right}, {}), number at {right}; panel record {rows:#x} flags {flags:#x} rect {:?} colour {:?}; texture {texture:#x}",
            top + SEG_H,
            rd::<[i32; 4]>(rows),
            rd::<[i32; 4]>(rows + 0x20)
        ));
    }
}

pub unsafe fn install() {
    if !super::settings::get().drive {
        return;
    }
    let base = base();
    if hook::install(base + TASK_RVA, &TASK_PROLOGUE, task as *const () as usize, &ORIG_TASK, "command menu task (Drive gauge HUD)").is_none() {
        log("Drive gauge HUD: KH1's command menu task wasn't as expected: no gauge on the HUD");
        return;
    }
    let (x, y) = super::settings::get().drive_gauge;
    log(&format!("Drive gauge HUD: drawn with the command menu, right end {x}, top {y} (drive_gauge in the ini)"));
}
