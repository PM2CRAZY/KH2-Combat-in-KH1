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
//! (Microsoft ADPCM, Ogg Vorbis for voices) and keeps them; nothing is copied anywhere. They play through Windows' `waveOut`: one device, opened once on a
//! thread of ours that mixes every sound playing into it (`mixer`). (It was a device per sound,
//! opened and closed on the game's thread, which stalled it each time, the user: "the new
//! abilities ... lag when i use them. The base combo doesn't lag ... the drive forms lag too",
//! the moves with KH2 sounds.) Ini: `kh2_sounds` (`magnet_explosion` the default, `all`, `off`) and
//! `kh2_volume`. The default is every KH2 ability move (`DEFAULT_MOTIONS`).

use super::log;
use super::sys::{waveOutOpen, waveOutPrepareHeader, waveOutWrite, WaveFormatEx, WaveHdr, CALLBACK_EVENT, WAVE_MAPPER, WHDR_DONE};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// KH2's ability moves (the user: their exact sounds, as Explosion's and Magnet Burst's): Upper
/// Slash 161, Slapshot 162, Dodge Slash 163, Slide Dash 164, Guard Break 165, Explosion 166,
/// Finishing Leap 167, Flash Step 169, Vicinity Break 170, Counterguard 171, Retaliating Slash
/// 172, Aerial Sweep 191, Aerial Spiral 192, Horizontal Slash 193, Aerial Finish 194, Aerial
/// Dive 196, Magnet Burst 197; Aerial Recovery 40 (the user: it had no sound) and the guard 173
/// (its swish), Aerial Dodge 202 (its whoosh, sound 999019 at frame 1; the user: "aerial dodge is
/// missing its sfx"). (The plain combo's swings keep KH1's.)
pub const DEFAULT_MOTIONS: [u16; 20] = [161, 162, 163, 164, 165, 166, 167, 169, 170, 171, 172, 191, 192, 193, 194, 196, 197, 40, 173, 202];

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

/// A sound asked for, for the mixer: its samples, rate, channels and gain.
struct Voice {
    data: Arc<Vec<i16>>,
    rate: u32,
    channels: u16,
    gain: f32,
    /// Where it is, in its own frames.
    pos: f64,
}

/// Sounds asked for since the mixer last looked.
static QUEUE: Mutex<Vec<Voice>> = Mutex::new(Vec::new());
/// The mixer's thread runs (started on the first sound).
static MIXER: std::sync::Once = std::sync::Once::new();
/// The mixer's output: 48 kHz stereo, in blocks of 20 ms, four queued. The mixer wakes on
/// Windows' event as each block is done (s274 polled with a 2 ms sleep, which Windows rounds up to
/// its 15.6 ms tick: sounds started late by a varying amount, "kind of jittery"). s276's 5 ms
/// blocks were shorter than Windows' own mixing period, which ran dry between them: "the sound
/// sounds like its crackling".
const OUT_RATE: u32 = 48_000;
const BLOCK_FRAMES: usize = 960;
const BLOCKS: usize = 4;

/// The mixer (its own thread): one `waveOut` device for every KH2 sound, kept open; each block,
/// as Windows hands it back, refilled with the sounds playing (resampled to 48 kHz, each at its
/// gain, clipped) and queued again.
fn mixer() {
    let fmt = WaveFormatEx { format_tag: 1, channels: 2, samples_per_sec: OUT_RATE, avg_bytes_per_sec: OUT_RATE * 4, block_align: 4, bits_per_sample: 16, cb_size: 0 };
    let mut handle = 0usize;
    let event = unsafe { super::sys::CreateEventW(std::ptr::null_mut(), 0, 0, std::ptr::null()) };
    unsafe {
        // (THREAD_PRIORITY_TIME_CRITICAL: the blocks must not wait on the game's threads.)
        super::sys::SetThreadPriority(super::sys::GetCurrentThread(), 15);
    }
    let r = unsafe { waveOutOpen(&mut handle, WAVE_MAPPER, &fmt, event as usize, 0, if event.is_null() { 0 } else { CALLBACK_EVENT }) };
    if r != 0 {
        log(&format!("KH2 sound: Windows refused to open the sound device (waveOutOpen {r}): no KH2 sounds"));
        return;
    }
    log("KH2 sound: mixer running (one device, 48 kHz stereo, 20 ms blocks, woken by its event)");
    let size = std::mem::size_of::<WaveHdr>() as u32;
    let mut bufs: Vec<Vec<i16>> = (0..BLOCKS).map(|_| vec![0i16; BLOCK_FRAMES * 2]).collect();
    let mut hdrs: Vec<Box<WaveHdr>> = bufs
        .iter_mut()
        .map(|b| Box::new(WaveHdr { data: b.as_mut_ptr() as *mut u8, buffer_length: (b.len() * 2) as u32, bytes_recorded: 0, user: 0, flags: 0, loops: 0, next: std::ptr::null_mut(), reserved: 0 }))
        .collect();
    for h in hdrs.iter_mut() {
        unsafe {
            waveOutPrepareHeader(handle, &mut **h, size);
        }
        // (Ready to fill.)
        h.flags |= WHDR_DONE;
    }
    let mut playing: Vec<Voice> = Vec::new();
    let mut mix = vec![0f32; BLOCK_FRAMES * 2];
    loop {
        let mut wrote = false;
        for i in 0..BLOCKS {
            let done = unsafe { std::ptr::read_volatile(&hdrs[i].flags) } & WHDR_DONE != 0;
            if !done {
                continue;
            }
            if let Ok(mut q) = QUEUE.lock() {
                playing.append(&mut q);
            }
            mix.iter_mut().for_each(|x| *x = 0.0);
            for v in playing.iter_mut() {
                let ch = v.channels.max(1) as usize;
                let frames = v.data.len() / ch;
                let step = v.rate as f64 / OUT_RATE as f64;
                for f in 0..BLOCK_FRAMES {
                    let p = v.pos as usize;
                    if p + 1 >= frames {
                        v.pos = frames as f64;
                        break;
                    }
                    let t = (v.pos - p as f64) as f32;
                    let at = |k: usize, c: usize| v.data[k * ch + c.min(ch - 1)] as f32;
                    let l = at(p, 0) * (1.0 - t) + at(p + 1, 0) * t;
                    let r = at(p, 1) * (1.0 - t) + at(p + 1, 1) * t;
                    mix[f * 2] += l * v.gain;
                    mix[f * 2 + 1] += r * v.gain;
                    v.pos += step;
                }
            }
            playing.retain(|v| (v.pos as usize) + 1 < v.data.len() / v.channels.max(1) as usize);
            for (o, m) in bufs[i].iter_mut().zip(&mix) {
                *o = m.clamp(-32768.0, 32767.0) as i16;
            }
            hdrs[i].flags &= !WHDR_DONE;
            unsafe {
                waveOutWrite(handle, &mut *hdrs[i], size);
            }
            wrote = true;
        }
        if !wrote {
            if event.is_null() {
                std::thread::sleep(std::time::Duration::from_millis(1));
            } else {
                unsafe {
                    super::sys::WaitForSingleObject(event, 20);
                }
            }
        }
    }
}

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
    // (With `kh2_sounds = off` Sora's moves keep KH1's sounds, but what KH1 has no sound of its own
    // for, Reflect and the Drive Forms, still gets KH2's: the user, "the reflect sound FX arent
    // working for some people".)
    let moves = super::settings::get().kh2_sounds.is_some();
    if !moves {
        log("KH2 sounds: Sora's moves keep KH1's (kh2_sounds = off); Reflect's and the forms' still load");
    }
    std::thread::spawn(move || {
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
            .filter(|(m, _)| moves && allowed(**m) || super::form::own_motion(**m))
            .flat_map(|(_, c)| c.events.iter().map(|e| e.kind))
            .collect();
        // Every move's battle voice (the Battle Quotes row switches them in game, on every move:
        // the user, "when switching to KH2 battle quotes, there are no KH2 battle quotes": only
        // the `kh2_sounds` list's moves had any).
        events.extend(kh2.character.moveset.clips.values().flat_map(|c| c.events.iter().map(|e| e.kind)).filter(|k| matches!(k, character_core::EventKind::Voice(_))));
        // Valor Form's own: its Keyblades' swings (frame trigger 8 on its weapon motion sets) and
        // its voices; its effect elements' sounds come with the forms' below (the user: "make
        // sure also that valor form has its proper sound FX").
        // (And Final Form's, and Anti Form's: its claws' swings and its voice, the user: "I want
        // 1:1 combat parity".)
        // (And Limit Form's: its Keyblade's swings and its voice.)
        for v in [kh2.valor.as_ref(), kh2.final_form.as_ref(), kh2.anti.as_ref(), kh2.limit.as_ref(), kh2.master.as_ref()].into_iter().flatten() {
            events.extend(v.character.moveset.clips.values().flat_map(|c| c.events.iter().map(|e| e.kind)).filter(|k| matches!(k, character_core::EventKind::Sound(_) | character_core::EventKind::Voice(_))));
        }
        // Reflect (the user: its sounds): its cast voices (MAGC, played by KH2's cast code, not a
        // motion trigger; only with `kh2_voice`) and its effect elements' sounds.
        let mut spell: Vec<(u8, u16, i32)> = Vec::new();
        if super::settings::get().reflect {
            events.extend(super::reflect::cast_voices(kh2).into_iter().map(character_core::EventKind::Voice));
            events.extend(REFLECT_VOICES.map(character_core::EventKind::Voice));
            spell = super::reflect::element_sounds();
        }
        // Wisdom Form's (its shots', `form`), and the revert's (Sora's elements, `form::revert_fx`).
        spell.extend(super::form::element_sounds(kh2));
        events.extend(super::form::REVERT_ELEMENTS.map(character_core::EventKind::Effect));
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
    // (Sora's effect package names his elements' sounds; without it the rest still loads.)
    let pack = install.read_any(src, "obj/P_EX100.a.us");
    if let Err(e) = &pack {
        log(&format!("KH2 sounds: Sora's effect package not read ({e}): his elements have no sounds"));
    }
    let pack = pack.unwrap_or_default();
    let bar = kh2_formats::bar::Bar::parse(&pack).ok();
    let pax = bar.as_ref().and_then(|b| b.entries.iter().find(|e| e.data.starts_with(b"PAX_"))).and_then(|p| kh2_formats::pax::Pax::parse(p.data).ok());
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
            EventKind::Effect(el) => match pax.as_ref().and_then(|p| p.element(el as u16)) {
                Some(p) if p.sound >= 0 => wants.push((p.sound as u32, Want::Element(el))),
                _ => log(&format!("KH2 sounds: effect element {el} has no sound")),
            },
            EventKind::Sound(id) => {
                let id = if (500_000..501_000).contains(&id) { id - 500_000 + WEAPON_BANK * 1000 } else { id };
                wants.push((id, Want::Sound(id)));
            }
            // (The user: KH1's voice, not KH2's, unless `kh2_voice = on`.)
            // (Reflect's, `REFLECT_VOICES`, whatever `kh2_voice` says.)
            EventKind::Voice(v) if v >= 0 && !voice_ids.contains(&v) => voice_ids.push(v),
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
        // (A bank that doesn't parse costs its own sounds, not all of them.)
        let scd = match kh2_formats::scd::Scd::parse(&banks[&bank]) {
            Ok(s) => s,
            Err(e) => {
                log(&format!("KH2 sounds: bank {bank} not readable ({e})"));
                continue;
            }
        };
        match scd.sound_id(index) {
            Ok(p) => {
                let of = match want {
                    Want::Element(el) => format!(" (effect element {el})"),
                    Want::Sound(_) => String::new(),
                    Want::Spell(level, el) => format!(" (package {level}'s element {el})"),
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
    // (Every move with KH2's quotes on: KH1's swing grunts don't mix with KH2's lines.)
    let _ = motion;
    kh2_voice() && LOADED.try_lock().map_or(false, |g| g.is_some())
}

/// Sora's `motion` reached a KH2 event: its sound, if it has one.
pub unsafe fn event(motion: u16, kind: character_core::EventKind) {
    use character_core::EventKind;
    // (Valor's swings and voices are its own, whatever `kh2_sounds` lists: `start`.)
    let valor = (super::form::in_valor() || super::form::in_limit() || super::form::in_master() || super::form::in_final() || super::form::in_anti()) && matches!(kind, EventKind::Sound(_) | EventKind::Voice(_));
    // (Battle quotes on every move, `kh2_voice`.)
    let quote = matches!(kind, EventKind::Voice(_)) && kh2_voice();
    if !(allowed(motion) || super::form::own_motion(motion) || valor || quote) {
        return;
    }
    let Ok(g) = LOADED.try_lock() else { return };
    let Some(l) = g.as_ref() else { return };
    match kind {
        EventKind::Effect(el) => {
            if let Some(s) = l.elements.get(&el) {
                play(s, Channel::Effects);
                note_play(|| format!("Sora's element {el} (motion {motion})"));
            }
        }
        EventKind::Sound(id) => {
            let id = if (500_000..501_000).contains(&id) { id - 500_000 + WEAPON_BANK * 1000 } else { id };
            match l.sounds.get(&id) {
                Some(s) => {
                    play(s, Channel::Effects);
                    note_play(|| format!("sound {id} (motion {motion}{})", if valor { ", Valor" } else { "" }));
                }
                None => note_play(|| format!("sound {id} (motion {motion}) not loaded")),
            }
        }
        // (A motion's voice only with `kh2_voice`: Reflect's voices load without it.)
        EventKind::Voice(v) if kh2_voice() => {
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
    let Ok(g) = LOADED.try_lock() else { return };
    match g.as_ref().and_then(|l| l.spell_elements.get(&(level, el))) {
        Some(s) => {
            play(s, Channel::Effects);
            note_play(|| format!("package {level}'s element {el}"));
        }
        None if g.is_none() => note_play(|| format!("package {level}'s element {el}: KH2 sounds not loaded (yet)")),
        None => {}
    }
}

/// The first plays, for the log (the user: "the sounds are still not there"; a sound that loads
/// but isn't heard: was it asked for, what did Windows say).
fn note_play(what: impl FnOnce() -> String) {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if N.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 40 {
        log(&format!("KH2 sound played: {} (volume {:.2})", what(), game_volume(Channel::Effects)));
    }
}

/// Reflect's finisher line (KH2's voice 25, clip 31), loaded whatever `kh2_voice` says
/// (`reflect_voice`).
pub const REFLECT_FINISHER_VOICE: i16 = 25;
/// (Its casts' grunts are KH1's, `kh1voice`; only the finisher's line is KH2's.)
pub const REFLECT_VOICES: [i16; 1] = [REFLECT_FINISHER_VOICE];

/// Reflect's cast voice `v` (`reflect`), whatever `kh2_voice` says: one of its clips at random.
pub unsafe fn reflect_voice(v: i16) {
    let Ok(g) = LOADED.try_lock() else { return };
    match g.as_ref().and_then(|l| l.voices.get(&v)) {
        Some(c) if !c.is_empty() => {
            let n = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_nanos()) / 1000) as usize % c.len();
            play(&c[n], Channel::Voice);
            note_play(|| format!("Reflect voice {v} (clip {n} of {})", c.len()));
        }
        _ => note_play(|| format!("Reflect voice {v} not loaded")),
    }
}

/// Sora's battle voice `v` (a spell's cast voice, `reflect`), when KH2's voices are on.
pub unsafe fn voice(v: i16) {
    if !kh2_voice() {
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
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        if N.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 5 {
            log("KH2 sound: muted (the game's volume for it is 0)");
        }
        return;
    }
    // (Handed to the mixer: nothing here waits on Windows' sound device.)
    MIXER.call_once(|| {
        std::thread::spawn(mixer);
    });
    if let Ok(mut q) = QUEUE.lock() {
        q.push(Voice { data: s.data.clone(), rate: s.rate.max(1), channels: s.channels.max(1), gain, pos: 0.0 });
    }
}

/// Every frame (no work now: the mixer closes nothing; kept for its callers).
pub unsafe fn tick() {}

/// KH2's battle quotes for Sora (the Battle Quotes row / ini `battle_quotes`; `kh2_voice` without
/// it). Every motion's voices are loaded, so the row switches them in game.
pub fn kh2_voice() -> bool {
    super::styles::kh2(super::styles::Kind::BattleQuotes)
}

