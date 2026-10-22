//! KH2's Drive gauge on KH1's HUD (the user: "Draw KH2's Drive gauge on KH1's HUD near Sora's
//! HP/MP gauge", then, with a mockup: "Can you bring in the actual drive gauge HUD from KH2?
//! ideally it would be positioned around here"). KH1's pieces drawn to KH2's look: the orange
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
//! **Only while the player has Sora in the field** (the user: "make sure the drive gauge is only
//! showing when the character is actively controlling sora. Dont show it on the world map, in the
//! menu, in the gummi ship routes, during cutscenes, etc"). KH1 runs this task outside the field
//! too: the Gummi side scene sets the HUD up as well (`0x14020BAB0` calls the HUD setup
//! `0x140264C50`, as the field's `0x14016E5B0` does), with no gauge slot bound, which the s323
//! gauge drew at full alpha. `visibility` gives the reason it isn't drawn, if any:
//! - KH1's game mode (`0x1423405C0`, stepped by `0x14016E850`): 5 is the field (set by the
//!   field's setup `0x14016E5B0`); 6 / 7 the Gummi side scene (`0x14020BAB0`: the world map, the
//!   flights), 8 / 9 its other setup (`0x14020B310`), 0..3 and 13..15 the title and the way back
//!   to it, the rest the loads between (10, 11, 12, 17, 18 set the field up again);
//! - KH1's pause machine (`0x14232E9B4`, `kh2fx::pause_menu_up`: the pause menu and the menus
//!   opened the same way);
//! - no physics frame of Sora's for 8 HUD frames (`takeover::GAME_FRAME`: any other menu or load
//!   that stops the field);
//! - his HUD slot not bound (`0x1428123E0`);
//! - KH1's event bit (above: cut at once, as KH1's own gauge is);
//! - an event running (`0x14298D3A0`: cutscenes, and talks with their text boxes) or KH1's
//!   script having him (`takeover::script_has_sora`: a scene, a door, a room's exit or entry walk;
//!   not his own actions, hits, knock-downs or KH1's limits);
//! - his HP 0 (the game over);
//! - KH1's own gauge faded out (its alpha, as before).
//!
//! It fades in and out in `FADE_S` (multiplied with KH1's alpha), cut at once where KH1's gauge
//! is (the event bit) or there is no field. KH1's MP HP label is hidden, and the black edge drawn
//! under the bars' ends, only while the gauge is drawn (past half its fade). Each change of reason
//! is logged once ("Drive gauge HUD: hidden (...)" / "shown").
//!
//! **KH2's own art** (`kh2hud`, `drive_gauge_art = kh2`): KH2's gauge from its field HUD file,
//! its animations as KH2 plays them; KH1's pieces above only when that can't be had.
//!
//! **Where**: left of Sora's portrait, where KH2 puts its gauge against its own portrait: KH2's
//! gauge centre at the middle of the largest of Sora's gauge sprites as KH1 draws them (his
//! record `0x142812400`: its place from `0x14026B820`, which already holds the wide screen's
//! shift, plus the rectangle of each sprite of the two lists KH1's gauge draws put there), the
//! DRIVE bar level with KH1's MP HP label (a wide, low sprite left of the portrait), which is
//! hidden (`hide_hp_label`: its record's size 0 while KH1 draws). `drive_gauge = dx, dy` moves
//! it (HUD units: 384 is the screen's height), `drive_gauge_scale` sizes it, `drive_gauge_debug
//! = on` draws a bright magenta block over it. KH1's pieces go under the portrait's box instead.

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
/// KH1's game mode (i32; 5 the field). See the module's notes.
const GAME_MODE_RVA: Rva = Rva(0x23405C0);
const FIELD_MODE: i32 = 5;
/// Nonzero while an event runs (cutscenes, talks): `takeover`'s and `camera`'s.
const EVENT_RVA: Rva = Rva(0x298D3A0);
/// KH1's party slot 0 (Sora's actor).
const PARTY_RVA: Rva = Rva(0x2D37C80);
/// HUD frames with no physics frame of Sora's before the field counts as stopped.
const STALL_FRAMES: u32 = 8;
/// The gauge's fade in or out, in seconds.
const FADE_S: f32 = 0.2;

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
/// KH1 HUD units per KH2 HUD unit: both fill the screen's height, KH1's 384 high, KH2's 448 (its
/// PS2 frame), times 1.34 (the user's mockup of the right size, measured against s321's); `drive_gauge_scale` multiplies it.
const KH2_UNIT: f32 = 384.0 / 448.0 * 1.34;
/// The middle of KH2's DRIVE bar below its gauge centre, in KH2 units (its bar spans 2 to 25).
const KH2_BAR_MIDDLE: f32 = 13.5;

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

/// KH1's HP/MP gauge task, then the Drive gauge over it. KH1's "MP HP" label (under the gauge)
/// is left out of KH1's draw (its sprite record's size 0 meanwhile, put back after).
unsafe extern "system" fn task(node: usize) -> usize {
    let f: TaskFn = std::mem::transmute(ORIG_TASK.load(Ordering::Acquire));
    let base = base();
    // Whether (and how faded) the gauge is drawn this frame.
    let fade = std::panic::catch_unwind(|| step_fade(base)).unwrap_or(0.0);
    let l = std::panic::catch_unwind(|| measure(base)).ok().flatten();
    // Both halves of the label ("MP" and "HP": the user, 'the "HP" text is still there, remove
    // it').
    // (Only while the gauge shows: before the first Drive Form, and wherever the gauge isn't
    // drawn, KH1's label stays.)
    let label_hidden = fade >= 0.5 && super::settings::get().hide_hp_label;
    let labels: Vec<(usize, [i32; 4])> = if label_hidden { l.as_ref().map(|l| l.labels.clone()).unwrap_or_default() } else { Vec::new() };
    let hidden: Vec<(usize, [i32; 2])> = labels
        .iter()
        .filter_map(|&(rec, _)| {
            // (A record with an animated rectangle keeps its own: not hidden.)
            if rd::<u32>(rec + 0x60) & 0x14 == 0x14 {
                return None;
            }
            let keep = rd::<[i32; 2]>(rec + 8);
            wr(rec + 8, [0i32, 0]);
            Some((rec, keep))
        })
        .collect();
    *LAYOUT.lock().unwrap_or_else(|e| e.into_inner()) = l;
    let r = f(node);
    for (rec, keep) in hidden {
        wr(rec + 8, keep);
    }
    if fade > 0.0 {
        let _ = std::panic::catch_unwind(|| draw(fade, label_hidden));
    }
    r
}

/// The gauge's fade (0 hidden .. 1 shown) and the reason last logged.
static FADE: std::sync::Mutex<(f32, Option<std::time::Instant>)> = std::sync::Mutex::new((0.0, None));
static REASON: std::sync::Mutex<Option<Option<String>>> = std::sync::Mutex::new(None);
static REASONS_LOGGED: AtomicU32 = AtomicU32::new(0);
/// Sora's physics frame last seen, and the HUD frames since it changed.
static LAST_GAME_FRAME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static STALLED: AtomicU32 = AtomicU32::new(0);

/// This frame's fade: towards 1 while Sora is the player's in the field, towards 0 (or 0 at once,
/// `visibility`'s `cut`) otherwise; a change of reason logged.
unsafe fn step_fade(base: usize) -> f32 {
    let (reason, cut) = match visibility(base) {
        Some((r, cut)) => (Some(r), cut),
        None => (None, false),
    };
    {
        let mut last = REASON.lock().unwrap_or_else(|e| e.into_inner());
        if last.as_ref() != Some(&reason) {
            // (Logged once per change, 400 at most.)
            if REASONS_LOGGED.fetch_add(1, Ordering::Relaxed) < 400 {
                log(&match &reason {
                    Some(r) => format!("Drive gauge HUD: hidden ({r})"),
                    None => "Drive gauge HUD: shown".into(),
                });
            }
            *last = Some(reason.clone());
        }
    }
    let mut g = FADE.lock().unwrap_or_else(|e| e.into_inner());
    let now = std::time::Instant::now();
    // (The time since the last HUD frame, at most 50 ms: a pause stops this task.)
    let dt = g.1.map_or(0.0, |t| now.duration_since(t).as_secs_f32()).min(0.05);
    g.1 = Some(now);
    g.0 = match reason {
        None => (g.0 + dt / FADE_S).min(1.0),
        Some(_) if cut => 0.0,
        Some(_) => (g.0 - dt / FADE_S).max(0.0),
    };
    g.0
}

/// Why the gauge isn't drawn this frame, and whether it goes at once (`true`) or fades out;
/// `None`: Sora is the player's in the field.
unsafe fn visibility(base: usize) -> Option<(String, bool)> {
    // Sora's physics frames (the field running): counted here per HUD frame.
    let gf = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    let stalled = if LAST_GAME_FRAME.swap(gf, Ordering::Relaxed) != gf {
        STALLED.store(0, Ordering::Relaxed);
        0
    } else {
        STALLED.fetch_add(1, Ordering::Relaxed).saturating_add(1)
    };
    let mode = rd::<i32>(base + GAME_MODE_RVA);
    if mode != FIELD_MODE {
        let what = match mode {
            6..=9 => "world map / Gummi Ship",
            0..=3 | 13..=15 => "title screen",
            _ => "loading",
        };
        return Some((format!("{what}: KH1 game mode {mode}, not the field"), true));
    }
    if super::kh2fx::pause_menu_up(base) {
        return Some(("menu: KH1's pause machine is up".into(), true));
    }
    let sora = rd::<u64>(base + PARTY_RVA) as usize;
    if sora < 0x10000 || !super::crash::readable(sora + 0x448) {
        return Some(("no Sora in the field".into(), true));
    }
    if gf == 0 || stalled >= STALL_FRAMES {
        // (Faded, not cut: a moment of KH1's own that skips his frames isn't a load.)
        return Some(("the field is stopped: no physics frame of Sora's (a menu or a load)".into(), gf == 0));
    }
    if rd::<u32>(base + SORA_BOUND_RVA) == 0 {
        return Some(("Sora's HP/MP gauge isn't up".into(), true));
    }
    if rd::<u32>(base + FLAGS_RVA) & 8 != 0 {
        return Some(("cutscene: KH1's event bit".into(), true));
    }
    if rd::<u32>(base + EVENT_RVA) != 0 {
        return Some(("event or dialogue".into(), false));
    }
    // (`takeover` keeps it on Sora's frames while it runs; without it, the same reads here.)
    let script = if super::takeover::ON.load(Ordering::Relaxed) {
        super::takeover::script_has_sora()
    } else {
        super::pose::ptr(rd::<u32>(sora + 0x440)) != 0 || rd::<u32>(sora + 0x444) & 1 != 0
    };
    if script {
        return Some(("scene: KH1's script has Sora".into(), false));
    }
    let stats = super::pose::ptr(rd::<u32>(sora + 0x6C));
    if stats > 0x10000 && super::crash::readable(stats + 0x44) && rd::<i32>(stats + 0x3C) <= 0 {
        return Some(("game over: Sora's HP 0".into(), false));
    }
    if !super::gauge::snapshot().is_some_and(|s| s.shown) {
        return Some(("no Drive Form opened yet".into(), true));
    }
    let alpha = rd::<f32>(base + SORA_GAUGE_RVA + 0xA9C);
    if !(0.01..=1.5).contains(&alpha) {
        return Some(("KH1's own gauge faded out".into(), true));
    }
    None
}

/// Sora's gauge as KH1 draws it this frame (`measure`).
#[derive(Clone, Debug)]
struct Layout {
    /// Every sprite's record and box `[x0, y0, x1, y1]`.
    sprites: Vec<(usize, [i32; 4])>,
    /// Their bounding box.
    all: [i32; 4],
    /// The largest (the portrait's), its middle the gauge's.
    portrait: [i32; 4],
    /// KH1's "MP HP" label: a wide, low sprite left of the portrait's middle, level with it (the
    /// largest; the gauge is placed from it).
    label: Option<(usize, [i32; 4])>,
    /// Every sprite of that label ("MP" and "HP" are two).
    labels: Vec<(usize, [i32; 4])>,
}

static LAYOUT: std::sync::Mutex<Option<Layout>> = std::sync::Mutex::new(None);
/// The label last logged (so a change is logged once).
static LABEL_LOGGED: AtomicUsize = AtomicUsize::new(usize::MAX);

unsafe fn measure(base: usize) -> Option<Layout> {
    let sprites = sora_sprites(base)?;
    let all = sprites.iter().fold([i32::MAX, i32::MAX, i32::MIN, i32::MIN], |b, (_, r)| [b[0].min(r[0]), b[1].min(r[1]), b[2].max(r[2]), b[3].max(r[3])]);
    let area = |r: &[i32; 4]| (r[2] - r[0]) * (r[3] - r[1]);
    let portrait = sprites.iter().map(|(_, r)| *r).max_by_key(area)?;
    let (pcx, pw) = ((portrait[0] + portrait[2]) / 2, portrait[2] - portrait[0]);
    let labels: Vec<(usize, [i32; 4])> = sprites
        .iter()
        .filter(|(_, r)| {
            let (w, h) = (r[2] - r[0], r[3] - r[1]);
            *r != portrait && w * 10 >= h * 14 && h <= 24 && w <= 120 && (r[0] + r[2]) / 2 < pcx - pw / 4 && r[1] >= portrait[1] - 10 && r[3] <= portrait[3] + 10
        })
        .copied()
        .collect();
    let label = labels.iter().max_by_key(|(_, r)| area(r)).copied();
    let l = Layout { sprites, all, portrait, label, labels };
    let key = l.label.map_or(0, |(rec, _)| rec);
    if LABEL_LOGGED.swap(key, Ordering::Relaxed) != key {
        let hide = if super::settings::get().hide_hp_label { "hidden" } else { "left (hide_hp_label = off)" };
        log(&format!(
            "Drive gauge HUD: Sora's gauge's sprites {:?}; portrait {:?}; KH1's MP HP label {}",
            l.sprites.iter().map(|(_, r)| *r).collect::<Vec<_>>(),
            l.portrait,
            match l.label {
                Some((rec, r)) => format!("{r:?} (record {rec:#x}), all its pieces {:?}, {hide}", l.labels.iter().map(|(_, r)| *r).collect::<Vec<_>>()),
                None => "not found".into(),
            }
        ));
    }
    (all[0] < all[2] && (-300..900).contains(&all[0]) && (-300..900).contains(&all[2]) && (0..HUD_H + 40).contains(&all[3])).then_some(l)
}

/// Sora's gauge's sprites as KH1 draws them this frame: each record and its box `[x0, y0, x1,
/// y1]` in HUD units. `None` when his slot isn't bound or the records aren't as expected.
unsafe fn sora_sprites(base: usize) -> Option<Vec<(usize, [i32; 4])>> {
    let g = base + SORA_GAUGE_RVA;
    if rd::<u32>(base + SORA_BOUND_RVA) == 0 || rd::<u32>(g) > 4 {
        return None;
    }
    let place: unsafe extern "system" fn(usize, *mut [i32; 2], *mut [i32; 2]) = std::mem::transmute(base + GAUGE_PLACE_RVA);
    let rect: unsafe extern "system" fn(usize, *mut [i32; 4]) = std::mem::transmute(base + RECT_RVA);
    let (mut pivot, mut at) = ([0i32; 2], [0i32; 2]);
    place(g, &mut pivot, &mut at);
    let mut out = Vec::new();
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
            out.push((rec, [at[0] + r[0], at[1] + r[1], at[0] + r[0] + r[2], at[1] + r[1] + r[3]]));
        }
    }
    (!out.is_empty()).then_some(out)
}

/// Our copy of a sprite record (16-byte aligned) the quads are built from.
fn rec_buffer() -> usize {
    *RECORD.get_or_init(|| {
        let b = Box::leak(vec![0u8; 0xC0 + 16].into_boxed_slice()).as_ptr() as usize;
        (b + 15) & !15
    })
}

/// One quad: the panel record copied (the middle of its picture, so a flat block of the colour),
/// its corners `[top-left, bottom-left, top-right, bottom-right]` and their colours, drawn now.
unsafe fn quad(base: usize, template: usize, texture: usize, at: [(i32, i32); 4], colour: [[i32; 4]; 4], alpha: f32) {
    let rec = rec_buffer();
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

/// The gauge, faded by `fade` (`step_fade`; `visibility` has already asked for Sora's gauge, the
/// field and the player's control), and the bars' bottom edge if KH1's label was left out.
unsafe fn draw(fade: f32, label_hidden: bool) {
    let base = base();
    let settings = super::settings::get();
    // With Sora's gauge's alpha.
    let g = base + SORA_GAUGE_RVA;
    let alpha = rd::<f32>(g + 0xA9C);
    if !(0.01..=1.5).contains(&alpha) {
        SKIPPED.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let alpha = alpha.min(1.0) * fade.clamp(0.0, 1.0);
    let Some(s) = super::gauge::snapshot() else { return };
    // (Not before the first Drive Form opens: the user.)
    if !s.shown {
        return;
    }
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
            // (Its length is the form's level's, `drivegauge::form_frames`: 4 bars at level 1,
            // 7 at MAX.)
            let (frac, number) = crate::drivegauge::form_view(f.left, f.max);
            (frac, number, false, true)
        }
        None => {
            let full = s.bars >= s.max.max(1);
            (if full { 1.0 } else { s.points.min(100) as f32 / 100.0 }, s.bars as i32, full, false)
        }
    };
    // Where: under Sora's gauge as KH1 draws it.
    let wide = rd::<i32>(base + WIDE_RVA).clamp(0, 400);
    let layout = LAYOUT.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let measured = layout.as_ref().map(|l| l.all);
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
    // KH2's own gauge, from its files: KH2's gauge centre at the portrait's middle (KH2 puts its
    // bar left of and just under it), its bar level with KH1's MP HP label when that is found.
    let k = KH2_UNIT * settings.drive_gauge_scale;
    let kh2 = if settings.drive_gauge_kh2 && super::kh2hud::ready() {
        let (cx, cy) = match &layout {
            Some(l) => {
                let p = l.portrait;
                // The user's screenshots (s320 against their mockup): the bar's middle 0.3 of the
                // portrait's radius below the MP HP label's middle, its right end at the
                // portrait's lower left.
                // (1.3 more and 6 right: the user's mockup of the right place, "This should be the
                // right size and positioning of the gauge", measured against s321's screenshot.)
                let below = 0.3 * (p[2] - p[0]) as f32 / 2.0 + 1.3;
                let cy = match l.label {
                    Some((_, r)) => (r[1] + r[3]) as f32 / 2.0 - KH2_BAR_MIDDLE * k + below,
                    None => (p[1] + p[3]) as f32 / 2.0 + below,
                };
                ((p[0] + p[2]) as f32 / 2.0 + 6.0, cy)
            }
            None => ((443 + wide) as f32, 316.0),
        };
        let view = super::kh2hud::View { frac, number, max, form };
        super::kh2hud::draw(base, &view, cx + dx as f32, cy + dy as f32, k, alpha, rows, texture, rec_buffer()).map(|(n, b)| (n, b, cx, cy))
    } else {
        None
    };
    if let Some((_, b, _, _)) = kh2 {
        // The MP and HP bars' ends lose their bottom edge with the label hidden (the label's
        // black plate drew it): a black line under both ends, from the MP bar's end (0.57 of the
        // ring's quarter left of it, measured on the user's s323 screenshot) to the label's right
        // end, 2 units thick as KH1's outlines are (the user: "the outline of the MP and HP bars
        // look like they've been cut off").
        // (Only when the label was left out this frame: wherever the gauge isn't drawn, KH1's
        // label and its edge are back.)
        if label_hidden {
            if let Some(l) = layout.as_ref().filter(|l| !l.labels.is_empty()) {
                let p = l.portrait;
                let left = p[0] - ((p[2] - p[0]) as f32 * 0.57).round() as i32;
                let right = l.labels.iter().map(|(_, r)| r[2]).max().unwrap_or(p[0] + 10).max(p[0] + 4);
                block(base, rows, texture, left, p[1], right + 1, p[1] + 2, 0, BLACK, BLACK, alpha);
            }
        }
        if settings.drive_gauge_debug {
            let b = b.map(|v| v.round() as i32);
            block(base, rows, texture, b[0] - 2, b[1] - 2, b[2] + 2, b[3] + 2, 0, MAGENTA, MAGENTA, 1.0);
        }
    } else {
        draw_kh1(base, rows, texture, (left, t, r), (frac, number, max, form), alpha, settings.drive_gauge_debug);
    }
    let (asked, drawn) = super::hud::count_end();
    wr(base + PACKET_RVA, saved.0);
    wr(base + GATE_RVA, saved.1);
    wr(base + SCALE_A_RVA, saved.2);
    wr(base + SCALE_B_RVA, saved.3);
    // The proof, every few seconds: where, how big, the alpha, what KH1 drew.
    let n = FRAMES.fetch_add(1, Ordering::Relaxed);
    let bx = match kh2 {
        Some((_, b, _, _)) => b.map(|v| v.round() as i32),
        None => [left, t - 13, r, t + BAR_H + 4],
    };
    let moved = (0..4).any(|i| LAST_BOX[i].load(Ordering::Relaxed) != bx[i]);
    let last = LAST_LOG.load(Ordering::Relaxed);
    if n % 180 == 0 || (moved && n.wrapping_sub(last) >= 60) {
        LAST_LOG.store(n, Ordering::Relaxed);
        for (i, v) in bx.iter().enumerate() {
            LAST_BOX[i].store(*v, Ordering::Relaxed);
        }
        let quads = QUADS.load(Ordering::Relaxed).wrapping_sub(q0);
        let from = match (&kh2, measured) {
            (Some((_, _, cx, cy)), _) => format!(
                "KH2's art, {} quads, KH2's gauge centre at ({cx:.0}, {cy:.0}) from Sora's portrait {:?}{}, {k:.3} HUD units per KH2 unit",
                kh2.map_or(0, |x| x.0),
                layout.as_ref().map(|l| l.portrait),
                if layout.as_ref().is_some_and(|l| l.label.is_some()) { " and KH1's MP HP label" } else { "" }
            ),
            (None, Some(b)) => format!("KH1's pieces ({quads} quads + {} words), Sora's gauge drawn at [{}, {}, {}, {}]", if max { 3 } else { 2 }, b[0], b[1], b[2], b[3]),
            (None, None) => format!("KH1's pieces ({quads} quads + {} words), Sora's gauge not measured: KH1's pivot (443, 316) + wide {wide}", if max { 3 } else { 2 }),
        };
        log(&format!(
            "Drive gauge HUD (frame {n}): box [{}, {}, {}, {}] in HUD units ({from}; ini offset {dx}, {dy}), alpha {alpha:.2}; KH1's immediate draw got {} vertices, drew {}{}; {}{}{}",
            bx[0],
            bx[1],
            bx[2],
            bx[3],
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

/// s319's gauge, drawn with KH1's pieces (when KH2's art can't be had): the bar, its fill, MAX,
/// the cap and the number, from the right end `r`, the bar's top `t` and its left end.
unsafe fn draw_kh1(base: usize, rw: usize, tx: usize, (left, t, r): (i32, i32, i32), (frac, number, max, form): (f32, i32, bool, bool), alpha: f32, debug: bool) {
    let words = WORDS.get_or_init(|| {
        let mut v: Vec<usize> = (0..10).map(|d| Box::leak(super::menu::encode(&d.to_string()).into_boxed_slice()).as_ptr() as usize).collect();
        for w in ["MAX", "DRIVE"] {
            v.push(Box::leak(super::menu::encode(w).into_boxed_slice()).as_ptr() as usize);
        }
        v
    });
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
    if debug {
        block(base, rw, tx, left - 2, t - 16, r + 4, t + BAR_H + 6, 0, MAGENTA, MAGENTA, 1.0);
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
    if s.drive_gauge_kh2 {
        super::kh2hud::load();
    }
    log(&format!(
        "Drive gauge HUD: drawn after Sora's HP/MP gauge, left of his portrait ({}; moved {}, {}; scale {}){}{}",
        if s.drive_gauge_kh2 { "KH2's own art when it can be read" } else { "KH1's pieces (drive_gauge_art = kh1)" },
        s.drive_gauge.0,
        s.drive_gauge.1,
        s.drive_gauge_scale,
        if s.hide_hp_label { "; KH1's MP HP label hidden" } else { "" },
        if s.drive_gauge_debug { "; drive_gauge_debug: a magenta block over it" } else { "" }
    ));
}
