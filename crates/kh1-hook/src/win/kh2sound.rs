//! KH2's own sound effects for Sora's moves, played in KH1 (the user: Magnet Burst's and
//! Explosion's exact sounds) (`docs/kh2-sound.md`).
//!
//! A move's KH2 sounds come from its frame triggers (OpenKH's table): 1 starts an effect
//! element of Sora's effect package (`obj/P_EX100.a.us`, PAX "p_ex"), which names a sound
//! (Explosion: 999038 at its start, 999042 at frame 34; Magnet Burst: 169 at frame 10, 999021
//! at frame 60); 8, on the Keyblade's motion set, plays a sound of the weapon's bank (both:
//! 500042 / 500044 at frame 18); 14 plays a battle voice (`00battle` VTBL id → a choice of
//! clips in `voice/us/battle/*_sora.win32.scd`: Explosion 6 at 0, Magnet Burst 7 at 0 and 6
//! at 50). Sound ids name the bank's id records, not its streams (`kh2_formats::scd`). Once
//! KH2 Sora is loaded, a thread reads them from the user's own KH2 install, decodes them
//! (Microsoft ADPCM, Ogg Vorbis for voices) and keeps them; nothing is copied anywhere. They play through Windows' `waveOut`, a device handle per sound, closed
//! when it is done. Ini: `kh2_sounds` (`magnet_explosion` the default, `all`, `off`) and
//! `kh2_volume`. The default is every KH2 ability move (`DEFAULT_MOTIONS`).

use super::log;
use super::sys::{waveOutClose, waveOutOpen, waveOutPrepareHeader, waveOutUnprepareHeader, waveOutWrite, WaveFormatEx, WaveHdr, WAVE_MAPPER, WHDR_DONE};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// KH2's ability moves (the user: their exact sounds, as Explosion's and Magnet Burst's): Upper
/// Slash 161, Slapshot 162, Dodge Slash 163, Slide Dash 164, Guard Break 165, Explosion 166,
/// Finishing Leap 167, Flash Step 169, Vicinity Break 170, Counterguard 171, Retaliating Slash
/// 172, Aerial Sweep 191, Aerial Spiral 192, Horizontal Slash 193, Aerial Finish 194, Aerial
/// Dive 196, Magnet Burst 197; Aerial Recovery 40 (the user: it had no sound) and the guard 173
/// (its swish). (The plain combo's swings keep KH1's.)
pub const DEFAULT_MOTIONS: [u16; 19] = [161, 162, 163, 164, 165, 166, 167, 169, 170, 171, 172, 191, 192, 193, 194, 196, 197, 40, 173];

struct Sound {
    rate: u32,
    channels: u16,
    data: Arc<Vec<i16>>,
}

/// What's loaded: effect element → sound, sound id → sound, voice id → its choices.
#[derive(Default)]
struct Loaded {
    elements: HashMap<i16, Sound>,
    sounds: HashMap<u32, Sound>,
    voices: HashMap<i16, Vec<Sound>>,
    /// A spell file's effect elements' sounds (`reflect`: REFLECT_<n>.mag's PAX), by (level,
    /// element).
    spell_elements: HashMap<(u8, u16), Sound>,
}

static LOADED: Mutex<Option<Loaded>> = Mutex::new(None);

/// The Keyblade's sound bank: frame trigger 8's bank 500 is the equipped weapon's own
/// (`0x1403DA0D0` in KH2: ids 500000..500999 take the weapon's bank); the Kingdom Key's is 500.
const WEAPON_BANK: u32 = 500;
/// Sora's battle voices (KH2 loads a world's set; Twilight Town's here).
const VOICE_FILE: &str = "voice/us/battle/tt0_sora.win32.scd";

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
        let mut events: Vec<character_core::EventKind> = kh2
            .character
            .moveset
            .clips
            .iter()
            .filter(|(m, _)| allowed(**m) || super::form::own_motion(**m))
            .flat_map(|(_, c)| c.events.iter().map(|e| e.kind))
            .collect();
        // Reflect (the user: its sounds): its cast voices (MAGC, played by KH2's cast code, not a
        // motion trigger; only with `kh2_voice`) and its effect elements' sounds.
        let mut spell: Vec<(u8, u16, i32)> = Vec::new();
        if super::settings::get().reflect {
            events.extend(super::reflect::cast_voices(kh2).into_iter().map(character_core::EventKind::Voice));
            spell = super::reflect::element_sounds();
        }
        // Wisdom Form's (its shots', `form`).
        spell.extend(super::form::element_sounds(kh2));
        match load(&events, &spell) {
            Ok(map) => {
                log(&format!(
                    "KH2 sounds: {} effect, {} other, {} voices, {} of Reflect's loaded in {:.1} s",
                    map.elements.len(),
                    map.sounds.len(),
                    map.voices.len(),
                    map.spell_elements.len(),
                    t.elapsed().as_secs_f32()
                ));
                if let Ok(mut g) = LOADED.lock() {
                    *g = Some(map);
                }
            }
            Err(e) => log(&format!("KH2 sounds: not loaded ({e})")),
        }
    });
}

fn load(events: &[character_core::EventKind], spell: &[(u8, u16, i32)]) -> Result<Loaded, String> {
    use character_core::EventKind;
    let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
    let install = kh2_data::Install::locate(&exe).map_err(|e| e.to_string())?;
    let src = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
    let pack = install.read_any(src, "obj/P_EX100.a.us").map_err(|e| e.to_string())?;
    let bar = kh2_formats::bar::Bar::parse(&pack).map_err(|e| e.to_string())?;
    let pax = bar.entries.iter().find(|e| e.data.starts_with(b"PAX_")).ok_or("no effect package")?;
    let pax = kh2_formats::pax::Pax::parse(pax.data).map_err(|e| e.to_string())?;
    let volume = super::settings::get().kh2_volume.clamp(0.0, 4.0);
    // Every sound id wanted, and who wants it.
    enum Want {
        Element(i16),
        Sound(u32),
        Spell(u8, u16),
    }
    let mut wants: Vec<(u32, Want)> = Vec::new();
    let mut voice_ids: Vec<i16> = Vec::new();
    for e in events {
        match *e {
            EventKind::Effect(el) => match pax.element(el as u16) {
                Some(p) if p.sound >= 0 => wants.push((p.sound as u32, Want::Element(el))),
                _ => log(&format!("KH2 sounds: effect element {el} has no sound")),
            },
            EventKind::Sound(id) => {
                let id = if (500_000..501_000).contains(&id) { id - 500_000 + WEAPON_BANK * 1000 } else { id };
                wants.push((id, Want::Sound(id)));
            }
            // (The user: KH1's voice, not KH2's, unless `kh2_voice = on`.)
            EventKind::Voice(v) if v >= 0 && !voice_ids.contains(&v) && super::settings::get().kh2_voice => voice_ids.push(v),
            _ => {}
        }
    }
    for &(level, el, sound) in spell {
        if sound >= 0 && !wants.iter().any(|(_, w)| matches!(w, Want::Spell(l, e) if *l == level && *e == el)) {
            wants.push((sound as u32, Want::Spell(level, el)));
        }
    }
    let mut out = Loaded::default();
    let decode = |p: kh2_formats::scd::Pcm| -> Sound {
        let data: Vec<i16> = p.samples.iter().map(|&x| (x as f32 * volume * p.volume).clamp(-32768.0, 32767.0) as i16).collect();
        Sound { rate: p.rate, channels: p.channels, data: Arc::new(data) }
    };
    let mut banks: HashMap<u32, Vec<u8>> = HashMap::new();
    for (id, want) in wants {
        let (bank, index) = kh2_formats::scd::split_id(id);
        if !banks.contains_key(&bank) {
            match install.read_any(src, &kh2_formats::scd::bank_file(bank)) {
                Ok(b) => {
                    banks.insert(bank, b);
                }
                Err(e) => {
                    log(&format!("KH2 sounds: bank {bank}: {e}"));
                    continue;
                }
            }
        }
        let scd = kh2_formats::scd::Scd::parse(&banks[&bank]).map_err(|e| e.to_string())?;
        match scd.sound_id(index) {
            Ok(p) => {
                let of = match want {
                    Want::Element(el) => format!(" (effect element {el})"),
                    Want::Sound(_) => String::new(),
                    Want::Spell(level, el) => format!(" (Reflect level {level}'s element {el})"),
                };
                log(&format!("KH2 sound {id}{of}: {} Hz, {} ch, {:.2} s, volume {:.2}", p.rate, p.channels, p.samples.len() as f32 / p.channels.max(1) as f32 / p.rate.max(1) as f32, p.volume));
                let snd = decode(p);
                match want {
                    Want::Element(el) => {
                        out.elements.insert(el, snd);
                    }
                    Want::Sound(s) => {
                        out.sounds.insert(s, snd);
                    }
                    Want::Spell(level, el) => {
                        out.spell_elements.insert((level, el), snd);
                    }
                }
            }
            Err(e) => log(&format!("KH2 sound {id}: {e}")),
        }
    }
    // Voices: `00battle`'s VTBL (character 1 = Sora: id, priority, five (voice, chance)).
    if !voice_ids.is_empty() {
        let battle = install.read_any(src, "00battle.bin").map_err(|e| e.to_string())?;
        let voice = install.read_any(src, VOICE_FILE).map_err(|e| e.to_string())?;
        let vscd = kh2_formats::scd::Scd::parse(&voice).map_err(|e| e.to_string())?;
        let table = vtbl(&battle);
        for v in voice_ids {
            let Some(choices) = table.iter().find(|(c, id, _)| *c == 1 && *id as i16 == v).map(|e| e.2.clone()) else { continue };
            let clips: Vec<Sound> = choices.iter().filter_map(|&c| vscd.sound_id(c as usize).ok()).map(decode).collect();
            log(&format!("KH2 voice {v}: clips {choices:?} ({} decoded)", clips.len()));
            if !clips.is_empty() {
                out.voices.insert(v, clips);
            }
        }
    }
    Ok(out)
}

/// `00battle.bin`'s VTBL entries: (character, id, the voice clips it picks from).
fn vtbl(bar: &[u8]) -> Vec<(u8, u8, Vec<i8>)> {
    let Ok(b) = kh2_formats::bar::Bar::parse(bar) else { return Vec::new() };
    let Some(v) = b.entries.iter().find(|e| e.name.trim_end_matches('\0') == "vtbl") else { return Vec::new() };
    let d = v.data;
    let n = d.get(4..8).map_or(0, |x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as usize);
    (0..n)
        .filter_map(|k| d.get(8 + k * 14..8 + k * 14 + 14))
        .map(|e| (e[0], e[1], (0..5).map(|j| e[4 + 2 * j] as i8).filter(|x| *x >= 0).collect()))
        .collect()
}

/// Whether KH2's own sounds stand in for KH1's (its swing voice) on `motion`.
pub fn covers(motion: u16) -> bool {
    super::settings::get().kh2_voice && allowed(motion) && LOADED.try_lock().map_or(false, |g| g.is_some())
}

/// Sora's `motion` reached a KH2 event: its sound, if it has one.
pub unsafe fn event(motion: u16, kind: character_core::EventKind) {
    use character_core::EventKind;
    if !(allowed(motion) || super::form::own_motion(motion)) {
        return;
    }
    let Ok(g) = LOADED.try_lock() else { return };
    let Some(l) = g.as_ref() else { return };
    match kind {
        EventKind::Effect(el) => {
            if let Some(s) = l.elements.get(&el) {
                play(s, Channel::Effects);
            }
        }
        EventKind::Sound(id) => {
            let id = if (500_000..501_000).contains(&id) { id - 500_000 + WEAPON_BANK * 1000 } else { id };
            if let Some(s) = l.sounds.get(&id) {
                play(s, Channel::Effects);
            }
        }
        EventKind::Voice(v) => {
            if let Some(c) = l.voices.get(&v) {
                static PICK: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                let n = PICK.fetch_add(7, std::sync::atomic::Ordering::Relaxed) as usize;
                play(&c[n % c.len()], Channel::Voice);
            }
        }
        _ => {}
    }
}

/// A Reflect level's effect element `el` started (`reflect`): its sound, as Sora's elements'
/// (the game's effects and volume sliders).
pub unsafe fn spell_element(level: u8, el: u16) {
    if super::settings::get().kh2_sounds.is_none() {
        return;
    }
    let Ok(g) = LOADED.try_lock() else { return };
    if let Some(s) = g.as_ref().and_then(|l| l.spell_elements.get(&(level, el))) {
        play(s, Channel::Effects);
    }
}

/// Sora's battle voice `v` (a spell's cast voice, `reflect`), when KH2's voices are on.
pub unsafe fn voice(v: i16) {
    if !super::settings::get().kh2_voice {
        return;
    }
    let Ok(g) = LOADED.try_lock() else { return };
    if let Some(c) = g.as_ref().and_then(|l| l.voices.get(&v)) {
        static PICK: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(3);
        let n = PICK.fetch_add(7, std::sync::atomic::Ordering::Relaxed) as usize;
        play(&c[n % c.len()], Channel::Voice);
    }
}

/// KH1's global settings (`0x14012AC50` returns them): its volume sliders, 0..10 each:
/// master `+0x10` (the device's, `0x14011BE4A`), then channels 1..3 `+0x12`, `+0x14`, `+0x16`
/// (`0x1400DDD00` sets each channel to its slider / 10), in the options menu's order: music,
/// sound effects, voice. Steam's build keeps them 4 bytes later (`+0x14..+0x1A`).
const SETTINGS_RVA: super::addr::Rva = super::addr::Rva(0x12AC50);

#[derive(Clone, Copy)]
enum Channel {
    Effects,
    Voice,
}

/// The game's volume for KH2's sounds of a kind now: master × its channel (the user: KH2's
/// sounds ignored the game's volume sliders).
fn game_volume(c: Channel) -> f32 {
    let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    if base == 0 {
        return 1.0;
    }
    let get: unsafe extern "system" fn() -> usize = unsafe { std::mem::transmute(base + SETTINGS_RVA) };
    let s = unsafe { get() };
    if s < 0x10000 {
        return 1.0;
    }
    let shift = if super::addr::Build::current() == super::addr::Build::Steam { 4 } else { 0 };
    let rd = |off: usize| unsafe { std::ptr::read_unaligned((s + off + shift) as *const i16) } as f32;
    let master = rd(0x10);
    let ch = rd(match c {
        Channel::Effects => 0x14,
        Channel::Voice => 0x16,
    });
    ((master / 10.0).clamp(0.0, 1.0)) * ((ch / 10.0).clamp(0.0, 1.0))
}

unsafe fn play(s: &Sound, channel: Channel) {
    let gain = game_volume(channel);
    if gain <= 0.001 {
        return;
    }
    let block = s.channels * 2;
    let fmt = WaveFormatEx { format_tag: 1, channels: s.channels, samples_per_sec: s.rate, avg_bytes_per_sec: s.rate * block as u32, block_align: block, bits_per_sample: 16, cb_size: 0 };
    let mut handle = 0usize;
    if waveOutOpen(&mut handle, WAVE_MAPPER, &fmt, 0, 0, 0) != 0 {
        return;
    }
    let data = if gain < 0.999 { Arc::new(s.data.iter().map(|&x| (x as f32 * gain) as i16).collect::<Vec<i16>>()) } else { s.data.clone() };
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
