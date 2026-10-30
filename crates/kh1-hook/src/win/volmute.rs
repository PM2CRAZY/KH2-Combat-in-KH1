//! KH1's volume sliders: Volume Mute (the owner, t915: "when audio is set to the lowest setting it currently doesn't
//! mute the game, can you make it so that the lowest setting on any of the audio sliders completely
//! mutes that track?"). `docs/kh1-volume.md`.
//!
//! KH1 keeps its four volume sliders as steps 1..10 in its settings block (Steam `0x1404D8614`,
//! fields `+0x14` master, `+0x16` bus 1, `+0x18` sound effects, `+0x1A` voices) and turns a step into
//! a volume through a table of eleven floats in `.rdata` (Steam `0x1403D8B60`: 0.0, 0.1 .. 1.0), read
//! by its four slider setters (`0x140116A70` master, `0x140116870` bus 1, `0x140116DB0` effects
//! buses 2 / 3 and 4..8, `0x140116F60` voices bus 3 / 2); step 1, the lowest the menu allows (the
//! setters take 1..10 only), is 0.1. Checked (decomp + exe).
//!
//! Two more places read the steps without the table, as step / 10.0: KH1's fade (`0x1400DDD10`)
//! puts buses 1..3 back to their sliders when a fade-in ends, and a stream's start (`0x14011CEA0`,
//! Assumption: the movie player's audio) takes the master step. Checked (decomp + exe).
//!
//! On (`volume_mute = on`, the default): the table's step 1 becomes 0.0, and those four step / 10.0
//! reads become reads of the same table (`movzx ecx, word [rax + field]; lea rax, [table];
//! movss xmm, [rax + rcx * 4]`, the same 19 bytes; `rax` and `rdx` are dead there, Checked (exe)).
//! So the lowest step is silence everywhere KH1 sets a volume, and steps 2..10 are what they were.
//! Data and code KH1 runs itself: nothing of ours runs per frame. Off: nothing is patched (vanilla).
//! Read once at start (a change to the ini needs a restart).
//!
//! t916, buses 5 and 6 after a fade (the owner, "(b) go ahead": Olympus's wind came back after a
//! Trinity with the effects slider at its lowest). KH1's effects setter sets buses 2 (or 3) and 4..8
//! to the effects step, but KH1's fade sets buses 5 and 6 to the fade's own level (its two calls of
//! the bus volume set, `0x1400547F0`, at `0x1400DDEA9` and `0x1400DDED3`, Checked (decomp + exe)):
//! after every fade-in they play at full whatever the effects slider says (vanilla too). With
//! `effects_after_fade = slider` (the default) those two calls go through `bus_after_fade`: a fade-in
//! (level above 0) sets them to the effects step's volume from KH1's own table instead, over the same
//! fade time; a fade-out (0) is left as it is. Only runs when KH1 fades (room changes, events), never
//! per frame. `vanilla`: not redirected.

use super::addr::Build;
use super::log;

/// The step-to-volume table (eleven floats, then one more 0.0) and its step 1 (0.1).
const TABLE_STEAM: usize = 0x3D8B60;
const STEP1_WAS: [u8; 4] = [0xCD, 0xCC, 0xCC, 0x3D];
const STEP1_NOW: [u8; 4] = [0x00, 0x00, 0x00, 0x00];

/// The step / 10.0 reads: where each starts, the settings field it reads, the register it fills
/// (6 = `xmm6`, 1 = `xmm1`) and the 19 bytes it must have (Checked (exe), Steam).
const SITES: [(usize, u8, u8, [u8; 19], &str); 4] = [
    // KH1's fade, bus 1 (`movsx ecx, word [rax + 0x16]; movd xmm6, ecx; cvtdq2ps xmm6, xmm6;
    // divss xmm6, [10.0]`).
    (0xDDD5B, 0x16, 6, [0x0F, 0xBF, 0x48, 0x16, 0x66, 0x0F, 0x6E, 0xF1, 0x0F, 0x5B, 0xF6, 0xF3, 0x0F, 0x5E, 0x35, 0xE6, 0x1B, 0x35, 0x00], "fade, bus 1"),
    // KH1's fade, bus 2 (the effects step).
    (0xDDD9C, 0x18, 6, [0x0F, 0xBF, 0x48, 0x18, 0x66, 0x0F, 0x6E, 0xF1, 0x0F, 0x5B, 0xF6, 0xF3, 0x0F, 0x5E, 0x35, 0xA5, 0x1B, 0x35, 0x00], "fade, bus 2"),
    // KH1's fade, bus 3 (the voice step).
    (0xDDDDD, 0x1A, 6, [0x0F, 0xBF, 0x48, 0x1A, 0x66, 0x0F, 0x6E, 0xF1, 0x0F, 0x5B, 0xF6, 0xF3, 0x0F, 0x5E, 0x35, 0x64, 0x1B, 0x35, 0x00], "fade, bus 3"),
    // A stream's start, the master step (`... xmm1 ...`).
    (0x11CF04, 0x14, 1, [0x0F, 0xBF, 0x48, 0x14, 0x66, 0x0F, 0x6E, 0xC9, 0x0F, 0x5B, 0xC9, 0xF3, 0x0F, 0x5E, 0x0D, 0x3D, 0x2A, 0x31, 0x00], "stream start, master"),
];

/// The table read that replaces a site at `rva` (19 bytes: `movzx ecx, word [rax + field]`, 4;
/// `lea rax, [rip + table]`, 7; `movss xmm, [rax + rcx * 4]`, 5; a 3-byte nop). `movzx` keeps the
/// index in 0..65535, inside `.rdata` past the table, so even a step out of range reads, as vanilla's
/// divide did.
fn table_read(rva: usize, field: u8, xmm: u8) -> [u8; 19] {
    let disp = (TABLE_STEAM as i64 - (rva as i64 + 4 + 7)) as i32;
    let d = disp.to_le_bytes();
    let modrm = (xmm << 3) | 0x04;
    [0x0F, 0xB7, 0x48, field, 0x48, 0x8D, 0x05, d[0], d[1], d[2], d[3], 0xF3, 0x0F, 0x10, modrm, 0x88, 0x0F, 0x1F, 0x00]
}

pub unsafe fn install() {
    install_mute();
    install_fade();
}

/// KH1's bus volume set (`0x1400547F0 (bus handle, volume, time)`, Checked (decomp)) and the two calls
/// of it in KH1's fade that set buses 5 and 6 (`call rel32`, Checked (exe)).
const BUS_SET_STEAM: usize = 0x547F0;
const FADE_BUS5_CALL_STEAM: usize = 0xDDEA9;
const FADE_BUS6_CALL_STEAM: usize = 0xDDED3;
/// KH1's settings block (where its sliders' steps are), as KH1's own getter returns it (`0x14012AF20`:
/// `lea rax, [rip + rel]; ret`, Checked (exe)), and the effects step in it.
const SETTINGS_GET_STEAM: usize = 0x12AF20;
const SETTINGS_STEAM: usize = 0x4D8614;
const EFFECTS_STEP: usize = 0x18;

static BUS_SET: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static BASE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// The volume a fade gives buses 5 and 6: a fade-in's level becomes the effects step's volume from
/// KH1's table (with Volume Mute on, step 1 is 0.0 there); a fade-out (0 or less) stays. A step out of
/// 1..10 keeps the fade's level (as vanilla).
fn after_fade(level: f32, step: i16, table: &[f32; 11]) -> f32 {
    if level <= 0.0 || !(1..=10).contains(&step) {
        return level;
    }
    table[step as usize]
}

unsafe extern "system" fn bus_after_fade(bus: usize, level: f32, time: u32) {
    use std::sync::atomic::Ordering;
    let base = BASE.load(Ordering::Relaxed);
    let step = std::ptr::read_unaligned((base + SETTINGS_STEAM + EFFECTS_STEP) as *const i16);
    let table = std::ptr::read_unaligned((base + TABLE_STEAM) as *const [f32; 11]);
    let set: unsafe extern "system" fn(usize, f32, u32) = std::mem::transmute(BUS_SET.load(Ordering::Relaxed));
    set(bus, after_fade(level, step, &table), time);
}

unsafe fn install_fade() {
    use std::sync::atomic::Ordering;
    if !super::settings::get().fade_follows_effects {
        log!(Sound, "Effects after fade: vanilla (effects_after_fade = vanilla): buses 5 and 6 take the fade's level");
        return;
    }
    if Build::current() != Build::Steam {
        log!(Sound, "Effects after fade: Steam only, not redirected");
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    // The settings getter must name the block read here (`lea rax, [rip + rel]; ret`).
    let get = base + SETTINGS_GET_STEAM;
    let head = std::slice::from_raw_parts(get as *const u8, 3);
    let names = head == [0x48, 0x8D, 0x05]
        && *((get + 7) as *const u8) == 0xC3
        && (get as i64 + 7 + std::ptr::read_unaligned((get + 3) as *const i32) as i64) as usize == base + SETTINGS_STEAM;
    if !names {
        log!(Sound, "Effects after fade: KH1's settings getter isn't the one expected, not redirected");
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    BUS_SET.store(base + BUS_SET_STEAM, Ordering::Relaxed);
    let ours = bus_after_fade as *const () as usize;
    let five = super::hook::redirect_call(base + FADE_BUS5_CALL_STEAM, base + BUS_SET_STEAM, ours, "fade, bus 5");
    let six = five && super::hook::redirect_call(base + FADE_BUS6_CALL_STEAM, base + BUS_SET_STEAM, ours, "fade, bus 6");
    if five && six {
        log!(Sound, "Effects after fade: slider (effects_after_fade = slider): buses 5 and 6 follow the effects slider after a fade-in");
    } else {
        log!(Sound, "Effects after fade: redirect failed (bus 5 {five}, bus 6 {six})");
    }
}

unsafe fn install_mute() {
    if !super::settings::get().volume_mute {
        log!(Sound, "Volume Mute: off (volume_mute = off): KH1's lowest volume step stays 0.1");
        return;
    }
    if Build::current() != Build::Steam {
        log!(Sound, "Volume Mute: Steam only, not patched");
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    // All or nothing: every site is checked before anything is written, so a half-patched mix of
    // step 1 = 0.0 in the setters and 0.1 after a fade can't happen.
    let table_ok = std::slice::from_raw_parts((base + TABLE_STEAM + 4) as *const u8, 4) == STEP1_WAS;
    let sites_ok = SITES.iter().all(|(rva, _, _, want, _)| std::slice::from_raw_parts((base + rva) as *const u8, want.len()) == want);
    if !table_ok || !sites_ok {
        log!(Sound, "Volume Mute: unexpected bytes in KH1's volume table or fade (table {table_ok}, reads {sites_ok}), not patched");
        return;
    }
    for (rva, field, xmm, want, what) in SITES {
        if !super::hook::patch(base + rva, &want, &table_read(rva, field, xmm), what) {
            return;
        }
    }
    if !super::hook::patch(base + TABLE_STEAM + 4, &STEP1_WAS, &STEP1_NOW, "volume table, step 1") {
        return;
    }
    log!(Sound, "Volume Mute: on (volume_mute = on): the lowest step of every volume slider is silence");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn after_fade_levels() {
        let mut t = [0.0f32; 11];
        for (i, v) in t.iter_mut().enumerate() {
            *v = i as f32 / 10.0;
        }
        t[1] = 0.0;
        assert_eq!(after_fade(1.0, 1, &t), 0.0);
        assert_eq!(after_fade(1.0, 5, &t), 0.5);
        assert_eq!(after_fade(1.0, 10, &t), 1.0);
        assert_eq!(after_fade(0.0, 5, &t), 0.0);
        assert_eq!(after_fade(1.0, 0, &t), 1.0);
        assert_eq!(after_fade(1.0, 11, &t), 1.0);
    }

    #[test]
    fn step1_bytes() {
        assert_eq!(STEP1_WAS, 0.1f32.to_bits().to_le_bytes());
        assert_eq!(STEP1_NOW, 0.0f32.to_bits().to_le_bytes());
    }

    /// The replacement at the fade's bus 1 read is the bytes checked against a disassembler.
    #[test]
    fn table_read_bytes() {
        assert_eq!(
            table_read(0xDDD5B, 0x16, 6),
            [0x0F, 0xB7, 0x48, 0x16, 0x48, 0x8D, 0x05, 0xFA, 0xAD, 0x2F, 0x00, 0xF3, 0x0F, 0x10, 0x34, 0x88, 0x0F, 0x1F, 0x00]
        );
        assert_eq!(
            table_read(0x11CF04, 0x14, 1),
            [0x0F, 0xB7, 0x48, 0x14, 0x48, 0x8D, 0x05, 0x51, 0xBC, 0x2B, 0x00, 0xF3, 0x0F, 0x10, 0x0C, 0x88, 0x0F, 0x1F, 0x00]
        );
    }
}
