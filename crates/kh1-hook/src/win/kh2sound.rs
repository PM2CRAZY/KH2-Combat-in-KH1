//! KH2's own sound effects for Sora's moves, played in KH1 (the user: Magnet Burst's and
//! Explosion's exact sounds) (`docs/kh2-sound.md`).
//!
//! KH2 starts an effect element with motion trigger 1 (`EventKind::Effect`); each element of
//! Sora's effect package (`obj/P_EX100.a.us`, PAX "p_ex") names a sound, `bank * 1000 +
//! index` (Explosion: 999038 at its start, 999042 at frame 34; Magnet Burst: 169 at frame 10,
//! 999021 at frame 60). Once KH2 Sora is loaded, a thread reads those from the user's own KH2
//! install (`se/seNNN.win32.scd`, `kh2_formats::scd`), decodes them and keeps them; nothing is
//! copied anywhere. They play through Windows' `waveOut`, a device handle per sound, closed
//! when it is done. Ini: `kh2_sounds` (`magnet_explosion` the default, `all`, `off`) and
//! `kh2_volume`.

use super::log;
use super::sys::{waveOutClose, waveOutOpen, waveOutPrepareHeader, waveOutUnprepareHeader, waveOutWrite, WaveFormatEx, WaveHdr, WAVE_MAPPER, WHDR_DONE};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Explosion and Magnet Burst.
pub const DEFAULT_MOTIONS: [u16; 2] = [166, 197];

struct Sound {
    rate: u32,
    channels: u16,
    data: Arc<Vec<i16>>,
}

/// (motion, element) → its sound, once loaded.
static LOADED: Mutex<Option<HashMap<i16, Sound>>> = Mutex::new(None);

struct Voice {
    handle: usize,
    hdr: Box<WaveHdr>,
    _data: Arc<Vec<i16>>,
}

// The header and buffer live in the voice until it is closed.
unsafe impl Send for Voice {}

static VOICES: Mutex<Vec<Voice>> = Mutex::new(Vec::new());

fn allowed(motion: u16) -> bool {
    match &super::settings::get().kh2_sounds {
        None => false,
        Some(None) => true,
        Some(Some(v)) => v.contains(&motion),
    }
}

/// Loads, in the background once KH2 Sora is there, the sounds of the effect elements the
/// chosen motions start.
pub fn start() {
    if super::settings::get().kh2_sounds.is_none() {
        log("KH2 sounds: off (kh2_sounds = off)");
        return;
    }
    std::thread::spawn(|| {
        let t = std::time::Instant::now();
        let kh2 = loop {
            if let Some(k) = super::pose::kh2() {
                break k;
            }
            if t.elapsed().as_secs() > 300 {
                log("KH2 sounds: KH2 Sora never loaded");
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        };
        let elements: Vec<(u16, Vec<i16>)> = kh2
            .character
            .moveset
            .clips
            .iter()
            .filter(|(m, _)| allowed(**m))
            .map(|(m, c)| (*m, c.events.iter().filter_map(|e| if let character_core::EventKind::Effect(n) = e.kind { Some(n) } else { None }).collect()))
            .collect();
        match load(&elements) {
            Ok(map) => {
                log(&format!("KH2 sounds: {} loaded in {:.1} s", map.len(), t.elapsed().as_secs_f32()));
                if let Ok(mut g) = LOADED.lock() {
                    *g = Some(map);
                }
            }
            Err(e) => log(&format!("KH2 sounds: not loaded ({e})")),
        }
    });
}

fn load(elements: &[(u16, Vec<i16>)]) -> Result<HashMap<i16, Sound>, String> {
    let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
    let install = kh2_data::Install::locate(&exe).map_err(|e| e.to_string())?;
    let src = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
    let pack = install.read_any(src, "obj/P_EX100.a.us").map_err(|e| e.to_string())?;
    let bar = kh2_formats::bar::Bar::parse(&pack).map_err(|e| e.to_string())?;
    let pax = bar.entries.iter().find(|e| e.data.starts_with(b"PAX_")).ok_or("no effect package")?;
    let pax = kh2_formats::pax::Pax::parse(pax.data).map_err(|e| e.to_string())?;
    let volume = super::settings::get().kh2_volume.clamp(0.0, 4.0);
    let mut by_bank: HashMap<u32, Vec<(i16, usize, u32)>> = HashMap::new();
    for &el in elements.iter().flat_map(|(_, e)| e.iter()) {
        let Some(e) = pax.element(el as u16) else { continue };
        if e.sound < 0 {
            continue;
        }
        let (bank, index) = kh2_formats::scd::split_id(e.sound as u32);
        let list = by_bank.entry(bank).or_default();
        if !list.iter().any(|x| x.0 == el) {
            list.push((el, index, e.sound as u32));
        }
    }
    let mut out = HashMap::new();
    for (bank, list) in by_bank {
        let bytes = match install.read_any(src, &kh2_formats::scd::bank_file(bank)) {
            Ok(b) => b,
            Err(e) => {
                log(&format!("KH2 sounds: bank {bank}: {e}"));
                continue;
            }
        };
        let scd = kh2_formats::scd::Scd::parse(&bytes).map_err(|e| e.to_string())?;
        for (el, index, id) in list {
            match scd.sound(index) {
                Ok(p) => {
                    let data: Vec<i16> = p.samples.iter().map(|&x| (x as f32 * volume).clamp(-32768.0, 32767.0) as i16).collect();
                    log(&format!("KH2 sound {id} (element {el}): {} Hz, {} ch, {:.2} s", p.rate, p.channels, data.len() as f32 / p.channels.max(1) as f32 / p.rate.max(1) as f32));
                    out.insert(el, Sound { rate: p.rate, channels: p.channels, data: Arc::new(data) });
                }
                Err(e) => log(&format!("KH2 sound {id} (element {el}): {e}")),
            }
        }
    }
    Ok(out)
}

/// Sora's `motion` started effect element `element`: its KH2 sound, if it has one.
pub unsafe fn element(motion: u16, element: i16) {
    if !allowed(motion) {
        return;
    }
    let Ok(g) = LOADED.try_lock() else { return };
    let Some(snd) = g.as_ref().and_then(|m| m.get(&element)) else { return };
    play(snd);
}

unsafe fn play(s: &Sound) {
    let block = s.channels * 2;
    let fmt = WaveFormatEx { format_tag: 1, channels: s.channels, samples_per_sec: s.rate, avg_bytes_per_sec: s.rate * block as u32, block_align: block, bits_per_sample: 16, cb_size: 0 };
    let mut handle = 0usize;
    if waveOutOpen(&mut handle, WAVE_MAPPER, &fmt, 0, 0, 0) != 0 {
        return;
    }
    let data = s.data.clone();
    let mut hdr = Box::new(WaveHdr { data: data.as_ptr() as *mut u8, buffer_length: (data.len() * 2) as u32, bytes_recorded: 0, user: 0, flags: 0, loops: 0, next: std::ptr::null_mut(), reserved: 0 });
    let size = std::mem::size_of::<WaveHdr>() as u32;
    if waveOutPrepareHeader(handle, &mut *hdr, size) != 0 || waveOutWrite(handle, &mut *hdr, size) != 0 {
        waveOutClose(handle);
        return;
    }
    if let Ok(mut v) = VOICES.lock() {
        v.push(Voice { handle, hdr, _data: data });
    }
}

/// Every frame: closes the sounds that finished.
pub unsafe fn tick() {
    let Ok(mut v) = VOICES.try_lock() else { return };
    let size = std::mem::size_of::<WaveHdr>() as u32;
    v.retain_mut(|voice| {
        let done = std::ptr::read_volatile(&voice.hdr.flags) & WHDR_DONE != 0;
        if done {
            waveOutUnprepareHeader(voice.handle, &mut *voice.hdr, size);
            waveOutClose(voice.handle);
        }
        !done
    });
}
