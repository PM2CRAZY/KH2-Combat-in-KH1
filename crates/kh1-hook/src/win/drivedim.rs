//! KH2's dim during a Drive change on KH1's own lights and fog (row 21, t964). The numbers:
//! `crate::drivedim`; notes: `docs/kh1-drive-forms-native.md` §21.
//!
//! Steam KH1, Checked (decomp) and (exe):
//! - **The light.** KH1's light list (31 slots of 0x40 at `0x142D5B030`: +0 type, +4 flags, +8 id,
//!   +0x30 an RGBA float colour). `0x14029D880 (type, flags, id)` takes a slot; `0x14029E0D0 (light,
//!   colour, ticks, when done)` fades its colour with KH1's own task (`0x14029E710`), which calls
//!   "when done" with the light; KH1's `0x14029DC20 (light)` frees it, as KH1's own field dim does
//!   (`0x1401CD1E0`). Every type-1 colour is multiplied into the map's ambient and each actor's
//!   light each tick (`0x14029E790`, `0x14029E5C0`). Ours: type 1, flags 8 (freed by KH1 at a room
//!   change, `0x14029D8D0`), id `OUR_ID`; faded to 0.375 over 12 ticks when the change starts, back
//!   to 1.0 over 15 ticks and freed by KH1 when the form arrives (or the change ends any other way).
//! - **The fog.** KH1's fog block `0x14233FE50` (+0 four distances, +0x10 the colour as three ints,
//!   +0x20 a block kept) and its fog fade, the one its event scripts start (`0x1401B0880`; the fade
//!   back to the room's `0x1401B0A00`): start `0x142382A60..`, target `0x142382A90..`, length
//!   `0x142382A50` in ticks (a script's frames × 2), time `0x142382A54`, on while `0x142382594` bit
//!   1; stepped by KH1's `0x1401B3840` from the field's script task `0x1401A2320` (Assumption: it
//!   runs in battle as in the field). Ours writes that block as the script's op does: the colour ×
//!   48 / 128 over 12 ticks, back over 15. Left alone when a script's fog fade is running.
//! - **The blur** (t965, answer (a)): KH1's own feedback blur, drawn each frame by KH1's
//!   `0x14017B190 (mode)` → `0x140185C40` from its draw loop (`0x140170FF0`) while the mode
//!   `0x14233FBE8` is 1 or 4: the last frame (`0x140108300`, prevFrameTex, copied every frame end by
//!   `0x1401030C0`) as a strip over the new one, blended `(Cs − Cd)·As + Cd` (ALPHA 0x44), colour
//!   `0x14233FDE0..DEC` (R, G, B, A; 0x80 = 1.0), offset `0x14233FE00` / `FE04` (PS2 pixels), spin to
//!   `0x14233FDF8` radians over `0x14233FDFC` frames. KH1's own event uses it at alpha 0x70
//!   (`0x14023F900`). Ours: mode 1, colour (0x80, 0x80, 0x80, 0x70), offset (2, 2), no spin, 30
//!   frames into the change (KH2's wait), its alpha down over 20 frames once the form arrives, then
//!   mode 0. Left alone when one of KH1's events has its own blur on (the mode not 0).
//! - The dim: twice a change (start, end). The blur: a few writes on the form's own frame
//!   (`form::native_frame`) while it waits or fades. Only with Drive on and on Steam.

use super::log;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::drivedim as rules;

const NEW_LIGHT_RVA: usize = 0x29D880;
const NEW_LIGHT_BYTES: [u8; 6] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20];
const FADE_RVA: usize = 0x29E0D0;
const FADE_BYTES: [u8; 10] = [0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18];
const FREE_RVA: usize = 0x29DC20;
const FREE_BYTES: [u8; 7] = [0xC7, 0x01, 0x00, 0x00, 0x00, 0x00, 0xC3];
const LIGHT_AMBIENT: i32 = 1;
const LIGHT_FREED_ON_ROOM: i32 = 8;
/// Our light's id ("K2").
const OUR_ID: i32 = 0x4B32;

const FOG_RVA: usize = 0x233FE50;
const FOG_FADE_RVA: usize = 0x2382A50;
const SCENE_FLAGS_RVA: usize = 0x2382594;
const FOG_FADE_ON: u32 = 2;
const BLUR_MODE_RVA: usize = 0x233FBE8;
const BLUR_COLOUR_RVA: usize = 0x233FDE0;
const BLUR_SPIN_RVA: usize = 0x233FDF8;
const BLUR_SPIN_FRAMES_RVA: usize = 0x233FDFC;
const BLUR_OFFSET_RVA: usize = 0x233FE00;
/// KH1's blur mode that draws the frame back once (4 also draws a plain copy first).
const BLUR_ON: i32 = 1;
const WORLD_RVA: usize = 0x233FE84;
const ROOM_RVA: usize = 0x233FE8C;

#[repr(C, align(16))]
struct Colour([f32; 4]);
static DIMMED: Colour = Colour([rules::DIM, rules::DIM, rules::DIM, 1.0]);
static FULL: Colour = Colour([1.0, 1.0, 1.0, 1.0]);

/// Our light while the change runs.
static LIGHT: AtomicUsize = AtomicUsize::new(0);
/// The fog colour we dimmed from, and the room it was in (world << 32 | room).
static FOG_WAS: Mutex<Option<[i32; 3]>> = Mutex::new(None);
static FOG_ROOM: AtomicU64 = AtomicU64::new(0);
static ON: AtomicBool = AtomicBool::new(false);
/// The blur: 0 off, 1 waiting (the change's first 30 frames), 2 on, 3 fading.
static BLUR: AtomicU8 = AtomicU8::new(0);
/// Frames waited, or fade frames left.
static BLUR_N: AtomicU32 = AtomicU32::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn on() -> bool {
    super::settings::get().drive && super::addr::Build::current() == super::addr::Build::Steam
}

unsafe fn calls_ok(base: usize) -> bool {
    super::hook::prologue_is(base + NEW_LIGHT_RVA, &NEW_LIGHT_BYTES) && super::hook::prologue_is(base + FADE_RVA, &FADE_BYTES) && super::hook::prologue_is(base + FREE_RVA, &FREE_BYTES)
}

unsafe fn room(base: usize) -> u64 {
    (rd::<u32>(base + WORLD_RVA) as u64) << 32 | rd::<u32>(base + ROOM_RVA) as u64
}

/// The light is still ours (KH1 frees flag-8 lights at a room change).
unsafe fn ours(light: usize) -> bool {
    light >= 0x10000 && rd::<i32>(light) == LIGHT_AMBIENT && rd::<i32>(light + 4) == LIGHT_FREED_ON_ROOM && rd::<i32>(light + 8) == OUR_ID
}

/// KH1's fog fade from the fog now to `colour` over `ticks` (as KH1's script op `0x1401B0880`
/// writes it: distances and the kept block unchanged).
unsafe fn fog_fade(base: usize, colour: [i32; 3], ticks: f32) {
    let fog = base + FOG_RVA;
    let fade = base + FOG_FADE_RVA;
    let dist: [u8; 16] = rd(fog);
    let now: [u8; 16] = rd(fog + 0x10);
    let kept: [u8; 16] = rd(fog + 0x20);
    // (Start: +0x10 the distances, +0x20 the colour, +0x30 the kept block; target +0x40 / +0x50 /
    // +0x60.)
    wr(fade + 0x10, dist);
    wr(fade + 0x20, now);
    wr(fade + 0x30, kept);
    wr(fade + 0x40, dist);
    wr(fade + 0x50, now);
    wr(fade + 0x50, colour);
    wr(fade + 0x60, kept);
    wr(fade + 4, 0.0f32);
    wr(fade, ticks);
    let flags = base + SCENE_FLAGS_RVA;
    wr(flags, rd::<u32>(flags) | FOG_FADE_ON);
}

/// The change started (`form::native_start`): KH1's light and fog dimmed over 12 ticks.
pub unsafe fn start() {
    let base = super::takeover::game_base();
    if !on() || base == 0 || ON.swap(true, Ordering::Relaxed) {
        return;
    }
    if !calls_ok(base) {
        log!(Forms, "Drive dim: KH1's light calls aren't as expected; no dim");
        ON.store(false, Ordering::Relaxed);
        return;
    }
    let new_light: unsafe extern "system" fn(i32, i32, i32) -> usize = std::mem::transmute(base + NEW_LIGHT_RVA);
    let fade: unsafe extern "system" fn(usize, usize, f32, usize) = std::mem::transmute(base + FADE_RVA);
    let light = new_light(LIGHT_AMBIENT, LIGHT_FREED_ON_ROOM, OUR_ID);
    if light >= 0x10000 {
        fade(light, &DIMMED as *const Colour as usize, rules::FRAMES_IN, 0);
        LIGHT.store(light, Ordering::Relaxed);
    }
    // The fog, unless one of KH1's scripts is fading it.
    let busy = rd::<u32>(base + SCENE_FLAGS_RVA) & FOG_FADE_ON != 0;
    let was: [i32; 3] = rd(base + FOG_RVA + 0x10);
    if !busy {
        fog_fade(base, was.map(rules::fog), rules::FRAMES_IN);
        if let Ok(mut g) = FOG_WAS.lock() {
            *g = Some(was);
        }
        FOG_ROOM.store(room(base), Ordering::Relaxed);
    }
    // The blur after KH2's 30 frames, unless one of KH1's events has its own on.
    let event_blur = rd::<i32>(base + BLUR_MODE_RVA) != 0;
    BLUR.store(if event_blur { 0 } else { 1 }, Ordering::Relaxed);
    BLUR_N.store(0, Ordering::Relaxed);
    log!(Forms, "Drive dim: KH1's light {} and fog {} dimmed to 48 / 128 over 12 ticks", if light >= 0x10000 { "slot taken" } else { "(no free slot)" }, if busy { "left (a script's fade runs)" } else { "colour" });
}

/// The change ended (`arrived`: the form is on; else a revert / KO cut it, or the form ended): back
/// over 15 ticks, the light freed by KH1 when done.
pub unsafe fn end(arrived: bool) {
    let base = super::takeover::game_base();
    if base == 0 {
        return;
    }
    // The blur: out over 20 frames once the form is on; off at once if the change was cut short.
    match BLUR.load(Ordering::Relaxed) {
        2 if arrived => {
            BLUR.store(3, Ordering::Relaxed);
            BLUR_N.store(rules::BLUR_OUT, Ordering::Relaxed);
        }
        1 => BLUR.store(0, Ordering::Relaxed),
        2 | 3 => blur_off(base),
        _ => {}
    }
    if !ON.swap(false, Ordering::Relaxed) {
        return;
    }
    let light = LIGHT.swap(0, Ordering::Relaxed);
    if ours(light) {
        let fade: unsafe extern "system" fn(usize, usize, f32, usize) = std::mem::transmute(base + FADE_RVA);
        fade(light, &FULL as *const Colour as usize, rules::FRAMES_OUT, base + FREE_RVA);
    }
    let was = FOG_WAS.lock().ok().and_then(|mut g| g.take());
    if let Some(was) = was {
        // (A new room since: its own fog colour, which KH1 keeps at `0x14233FE30`.)
        let to = if room(base) == FOG_ROOM.load(Ordering::Relaxed) { was } else { rd(base + FOG_RVA - 0x20) };
        fog_fade(base, to, rules::FRAMES_OUT);
    }
    log!(Forms, "Drive dim: back over 15 ticks");
}

/// KH1's blur set as KH2's (mode 1: the frame drawn back once).
unsafe fn blur_on(base: usize) {
    wr(base + BLUR_COLOUR_RVA, [0x80i32, 0x80, 0x80, rules::BLUR_ALPHA]);
    wr(base + BLUR_SPIN_RVA, 0.0f32);
    wr(base + BLUR_SPIN_FRAMES_RVA, 1i32);
    wr(base + BLUR_OFFSET_RVA, [rules::BLUR_OFFSET, rules::BLUR_OFFSET]);
    wr(base + BLUR_MODE_RVA, BLUR_ON);
    BLUR.store(2, Ordering::Relaxed);
    log!(Forms, "Drive blur: KH1's feedback blur on (alpha 0x70, offset 2, 2)");
}

/// KH1's blur off, if it is still ours (an event may have taken it since).
unsafe fn blur_off(base: usize) {
    BLUR.store(0, Ordering::Relaxed);
    if rd::<i32>(base + BLUR_MODE_RVA) == BLUR_ON {
        wr(base + BLUR_MODE_RVA, 0i32);
    }
}

/// The form's frame (`form::native_frame`, `steps` 60 Hz frames): the blur's wait and fade.
pub unsafe fn frame(steps: u32) {
    let state = BLUR.load(Ordering::Relaxed);
    if state != 1 && state != 3 {
        return;
    }
    let base = super::takeover::game_base();
    if base == 0 {
        return;
    }
    let n = BLUR_N.load(Ordering::Relaxed);
    if state == 1 {
        let n = n + steps.max(1);
        BLUR_N.store(n, Ordering::Relaxed);
        if n >= rules::BLUR_AFTER {
            blur_on(base);
        }
        return;
    }
    // Fading: an event that took the blur over keeps it.
    if rd::<i32>(base + BLUR_MODE_RVA) != BLUR_ON {
        BLUR.store(0, Ordering::Relaxed);
        return;
    }
    let left = n.saturating_sub(steps.max(1));
    BLUR_N.store(left, Ordering::Relaxed);
    wr(base + BLUR_COLOUR_RVA + 0xC, rules::blur_alpha(left));
    if left == 0 {
        blur_off(base);
        log!(Forms, "Drive blur: off");
    }
}
