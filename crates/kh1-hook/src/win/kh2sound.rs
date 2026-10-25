//! KH2's own sound effects for Sora's moves, played in KH1 (the user: Magnet Burst's and
//! Explosion's exact sounds) (`docs/kh2-sound.md`).
//!
//! A move's KH2 sounds come from its frame triggers (OpenKH's table): 1 starts an effect
//! element of Sora's effect package (`obj/P_EX100.a.us`, PAX "p_ex"), which names a sound
//! (Explosion: 999038 at its start, 999042 at frame 34; Magnet Burst: 169 at frame 10, 999021
//! at frame 60); 8, on the Keyblade's motion set, plays a sound of the weapon's bank (both:
//! 500042 / 500044 at frame 18); 14 plays a battle voice (`00battle` VTBL id → a choice of
//! clips in `voice/us/battle/*_sora.win32.scd`: Explosion 6 at 0, Magnet Burst 7 at 0 and 6
//! at 50). Sound ids name the bank's id records, not its streams (`kh2_formats::scd`).
//!
//! Every one of them plays through KH1's own sound engine, nothing else (the tester, "gut the
//! old system and transfer everything"; `docs/kh1-sound-native.md`): once KH2 Sora is loaded, a
//! thread reads the banks from the user's own KH2 install, cuts each sound into a one-sound bank
//! in KH1's file form (`kh2_formats::scd_cut`, still compressed: KH1's engine decodes it as it
//! plays, as it does its own) and hands it to `kh1bank`, which keeps it in KH1's bank list;
//! effects go through KH1's effect queue at Sora, battle quotes through KH1's voice channel.
//! Nothing is decoded on our side and no sound of ours has a player of its own: what KH1's
//! engine can't take (off Steam, the queue full) isn't heard, as KH1 drops its own. Ini:
//! `kh2_sounds` (`abilities` the default, `all`, `off`) and `kh2_volume` (in the cut banks'
//! records, as KH1's own volumes are).

use super::log;
use std::collections::HashMap;
use std::sync::Mutex;

/// KH2's ability moves (the user: their exact sounds, as Explosion's and Magnet Burst's): Upper
/// Slash 161, Slapshot 162, Dodge Slash 163, Slide Dash 164, Guard Break 165, Explosion 166,
/// Finishing Leap 167, Flash Step 169, Vicinity Break 170, Counterguard 171, Retaliating Slash
/// 172, Aerial Sweep 191, Aerial Spiral 192, Horizontal Slash 193, Aerial Finish 194, Aerial
/// Dive 196, Magnet Burst 197; Aerial Recovery 40 (the user: it had no sound) and the guard 173
/// (its swish), Aerial Dodge 202 (its whoosh, sound 999019 at frame 1; the user: "aerial dodge is
/// missing its sfx"). (The plain combo's swings keep KH1's.)
pub const DEFAULT_MOTIONS: [u16; 20] = [161, 162, 163, 164, 165, 166, 167, 169, 170, 171, 172, 191, 192, 193, 194, 196, 197, 40, 173, 202];

/// A KH2 sound as KH1 knows it: its one-sound bank's file number in KH1's engine (`kh1bank`)
/// and its length (ms; `None` when it can't be told without decoding: a Vorbis effect).
struct Sound {
    kh1: i32,
    ms: Option<u32>,
}

/// What's loaded: effect element → sound, sound id → sound, voice id → its choices.
#[derive(Default)]
struct Loaded {
    elements: HashMap<i16, Sound>,
    sounds: HashMap<u32, Sound>,
    voices: HashMap<i16, VoiceSet>,
    /// A spell file's effect elements' sounds (`reflect`: REFLECT_<n>.mag's PAX), by (level,
    /// element).
    spell_elements: HashMap<(u8, u16), Sound>,
}

/// A KH2 battle voice (`00battle`'s VTBL row for Sora): its priority, its five (clip, chance)
/// choices, and the clips of KH2's voice bank it can pick that play something.
struct VoiceSet {
    priority: i32,
    choices: Vec<(i8, i8)>,
    clips: Vec<u8>,
}

impl VoiceSet {
    /// KH2's pick (Steam KH2 `0x1403CDB60`): a roll of 0..99 from its random number (x 0x10DCD a
    /// draw, / 0x28F5C29); each choice with a chance takes the roll down by it and is picked when
    /// the roll goes below 0; the choices without a chance are the pool, one picked evenly if
    /// none was. A negative clip is none (-3, a partner's line, isn't Sora's: left out).
    fn pick(&self) -> Option<u8> {
        static RNG: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let draw = || -> u32 {
            let mut r = RNG.load(std::sync::atomic::Ordering::Relaxed);
            if r == 0 {
                r = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos() | 1);
            }
            r = r.wrapping_mul(0x10DCD);
            RNG.store(r, std::sync::atomic::Ordering::Relaxed);
            r
        };
        let mut roll = (draw() / 0x28F_5C29) as i32;
        let mut pool: Vec<i8> = Vec::new();
        let mut chosen = None;
        for &(clip, chance) in &self.choices {
            if clip < 0 {
                continue;
            }
            if chance < 1 {
                pool.push(clip);
            } else {
                roll -= chance as i32;
                if roll < 0 {
                    chosen = Some(clip);
                    break;
                }
            }
        }
        let clip = match chosen {
            Some(c) => c,
            None if !pool.is_empty() => {
                let n = pool.len() as u64;
                pool[(draw() as u64 / ((n - 1 + 0x1_0000_0000) / n)).min(n - 1) as usize]
            }
            None => return None,
        };
        self.clips.iter().copied().find(|c| *c as i8 == clip)
    }

    /// Sora says it: through KH1's voice channel at the row's priority (`kh1bank::voice`; KH1's
    /// rule plays it only over a line of lower standing).
    unsafe fn say(&self) -> Option<u8> {
        let clip = self.pick()?;
        if !super::kh1bank::voice(clip, self.priority.max(1)) {
            dropped(|| format!("quote clip {clip}: KH1's voice path can't take it now"));
        }
        Some(clip)
    }
}

static LOADED: Mutex<Option<Loaded>> = Mutex::new(None);

/// KH1's sound-effect bus (its slider's, `0x1400DDD10`).
const EFFECTS_BUS: u8 = 2;

/// The Keyblade's sound bank: frame trigger 8's bank 500 is the equipped weapon's own
/// (`0x1403DA0D0` in KH2: ids 500000..500999 take the weapon's bank); the Kingdom Key's is 500.
const WEAPON_BANK: u32 = 500;
/// Sora's battle voices (KH2 loads a world's set; Twilight Town's here).
const VOICE_FILE: &str = "voice/us/battle/tt0_sora.win32.scd";

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
        // (And Final Form's.)
        for v in [kh2.valor.as_ref(), kh2.final_form.as_ref()].into_iter().flatten() {
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
    // Each sound as a one-sound bank for KH1's own engine (`kh1bank`), at the id's own volume (in
    // the bank's record) times `kh2_volume`. One bank a sound: a sound several moves use is cut
    // once. Nothing is decoded: KH1's engine plays the cut bank's compressed audio as it is.
    let mut made: HashMap<(u32, usize), Option<Sound>> = HashMap::new();
    let mut native = |key: u32, bank: &[u8], index: usize, what: String| -> Option<Sound> {
        if let Some(s) = made.get(&(key, index)) {
            return s.as_ref().map(|s| Sound { kh1: s.kh1, ms: s.ms });
        }
        let snd = match kh2_formats::scd_cut::cut(bank, index, (super::kh1bank::FIRST_NO / 1000) as u16) {
            Ok(mut c) => {
                let _ = kh2_formats::scd_cut::scale_volume(&mut c.bytes, volume);
                // (KH2's sound-effect slider sets buses 2 and 4..8 alike, KH2 `0x140138290`;
                // KH1's sets 2 only, `0x1400DDD10`, its 5 / 6 follow a scale. So a KH2 effect on
                // 4..8, the Keyblade's bank on 8 among them, goes on KH1's effects bus 2, where
                // KH1's slider reaches it as KH2's does.)
                let kh2_bus = c.bus;
                if (4..=8).contains(&kh2_bus) {
                    let _ = kh2_formats::scd_cut::set_bus(&mut c.bytes, EFFECTS_BUS);
                }
                let ms = kh2_formats::scd_cut::length_ms(&c.bytes);
                let (bus, tracks, size) = (if (4..=8).contains(&kh2_bus) { format!("{kh2_bus} -> {EFFECTS_BUS}") } else { kh2_bus.to_string() }, c.tracks, c.bytes.len());
                let no = super::kh1bank::add(c.bytes, what.clone());
                match no {
                    Some(no) => log(&format!("KH1 banks: {what} -> se{no} ({size} bytes, bus {bus}, {tracks} track(s), {})", ms.map_or("length unknown".to_string(), |m| format!("{m} ms")))),
                    None => log(&format!("KH1 banks: {what} not added (KH1's sound engine is used on the Steam build only)")),
                }
                no.map(|kh1| Sound { kh1, ms })
            }
            Err(e) => {
                log(&format!("KH1 banks: {what} not cut ({e})"));
                None
            }
        };
        let back = snd.as_ref().map(|s| Sound { kh1: s.kh1, ms: s.ms });
        made.insert((key, index), snd);
        back
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
        let of = match want {
            Want::Element(el) => format!(" (effect element {el})"),
            Want::Sound(_) => String::new(),
            Want::Spell(level, el) => format!(" (package {level}'s element {el})"),
        };
        let Some(snd) = native(bank, &banks[&bank], index, format!("KH2 sound {id}{of}")) else { continue };
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
    // (The whole KH2 banks were only read to cut the sounds out: freed here.)
    drop(banks);
    // Voices: `00battle`'s VTBL (character 1 = Sora: id, priority, five (voice, chance)).
    if !voice_ids.is_empty() {
        let battle = install.read_any(src, "00battle.bin").map_err(|e| e.to_string())?;
        let voice = install.read_any(src, VOICE_FILE).map_err(|e| e.to_string())?;
        let table = vtbl(&battle);
        for v in voice_ids {
            let Some((priority, choices)) = table.iter().find(|(c, id, _, _)| *c == 1 && *id as i16 == v).map(|e| (e.2, e.3.clone())) else { continue };
            // (A clip KH1's voice create could play: its sound record has audio, the same test
            // the cut makes.)
            let clips: Vec<u8> = choices.iter().filter(|(c, _)| *c >= 0).map(|&(c, _)| c as u8).filter(|&c| kh2_formats::scd_cut::cut(&voice, c as usize, 0).is_ok()).collect();
            log(&format!("KH2 voice {v}: priority {priority}, (clip, chance) {choices:?} ({} playable)", clips.len()));
            if !clips.is_empty() {
                out.voices.insert(v, VoiceSet { priority: priority as i32, choices, clips });
            }
        }
        // The whole bank for KH1's voice list (as KH1 keeps a character's voices: `kh1bank`).
        if !out.voices.is_empty() {
            super::kh1bank::set_voice_bank(voice);
        }
    }
    Ok(out)
}

/// `00battle.bin`'s VTBL entries (14 bytes, as KH2's pick `0x1403CDB60` reads them): (character,
/// id, priority, five (clip, chance)).
fn vtbl(bar: &[u8]) -> Vec<(u8, u8, u8, Vec<(i8, i8)>)> {
    let Ok(b) = kh2_formats::bar::Bar::parse(bar) else { return Vec::new() };
    let Some(v) = b.entries.iter().find(|e| e.name.trim_end_matches('\0') == "vtbl") else { return Vec::new() };
    let d = v.data;
    let n = d.get(4..8).map_or(0, |x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as usize);
    (0..n)
        .filter_map(|k| d.get(8 + k * 14..8 + k * 14 + 14))
        .map(|e| (e[0], e[1], e[2], (0..5).map(|j| (e[4 + 2 * j] as i8, e[5 + 2 * j] as i8)).filter(|x| x.0 != -1).collect()))
        .collect()
}

/// Whether KH2's own sounds stand in for KH1's (its swing voice) on `motion`.
pub fn covers(motion: u16) -> bool {
    // (Every move with KH2's quotes on: KH1's swing grunts don't mix with KH2's lines.)
    let _ = motion;
    kh2_voice() && LOADED.try_lock().map_or(false, |g| g.is_some())
}

/// The return address of KH1's voice-group call in its attack row start (Steam `0x1402A3980`:
/// the row's `+0xF` voice, `0x1402901F0 (actor, voice, 0)` at `0x1402A3AAF`, Checked (exe)).
const ROW_VOICE_RETURN_STEAM: usize = 0x2A3AB4;
/// Where the row start keeps the row it runs: Sora's combat state (the handle at `+0x14C`)
/// `+0x40` (byte), written just before the voice call. Checked (decomp).
const ROW_IN_STATE: usize = 0x40;

/// KH1's voice group `0x1402901F0` is called (`soundlog`'s hook, `from` the call's return RVA):
/// true to leave it out. With KH2's battle quotes on, the voice KH1's attack row start gives Sora
/// (its row's `+0xF`, the grunt of KH1's swing) isn't said on a row that plays a KH2 motion of
/// ours: that motion's own KH2 quote takes its place (its frame trigger 14, `event`), as in KH2,
/// where the swing has only its own line. Before, KH1's grunt took Sora's voice channel first, and
/// KH1's priority rule then kept every KH2 quote out (the t544 logs: "KH1 kept the line playing"
/// on every one). KH1's other voices of Sora (hurt lines, spells, scripts) are untouched.
pub unsafe fn holds_back(actor: usize, from: usize) -> bool {
    if from != ROW_VOICE_RETURN_STEAM || super::addr::Build::current() != super::addr::Build::Steam || !kh2_voice() {
        return false;
    }
    if actor < 0x10000 || actor != super::takeover::sora_actor() {
        return false;
    }
    // (KH2's quotes loaded: otherwise KH1's grunt stays.)
    if !LOADED.try_lock().map_or(false, |g| g.as_ref().map_or(false, |l| !l.voices.is_empty())) {
        return false;
    }
    let cs = super::pose::ptr(std::ptr::read_unaligned((actor + 0x14C) as *const u32));
    if cs < 0x10000 || !super::crash::readable(cs + ROW_IN_STATE) {
        return false;
    }
    let row = std::ptr::read_unaligned((cs + ROW_IN_STATE) as *const u8) as usize;
    let Some(request) = super::kh1rows::row_request(actor, row) else { return false };
    let held = super::kh2slots::plays_kh2(request as usize);
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if held && N.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 20 {
        log(&format!("KH2 quotes: KH1's swing voice of row {row} (request {request:#x}) left out; the KH2 motion's quote takes its place"));
    }
    held
}

/// Sora's `motion` reached a KH2 event: its sound, if it has one.
pub unsafe fn event(motion: u16, kind: character_core::EventKind) {
    use character_core::EventKind;
    // (Valor's swings and voices are its own, whatever `kh2_sounds` lists: `start`.)
    let valor = (super::form::in_valor() || super::form::in_final()) && matches!(kind, EventKind::Sound(_) | EventKind::Voice(_));
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
                play(s);
                note_play(|| format!("Sora's element {el} (motion {motion})"));
            }
        }
        EventKind::Sound(id) => {
            let id = if (500_000..501_000).contains(&id) { id - 500_000 + WEAPON_BANK * 1000 } else { id };
            match l.sounds.get(&id) {
                Some(s) => {
                    play(s);
                    note_play(|| format!("sound {id} (motion {motion}{})", if valor { ", Valor" } else { "" }));
                }
                None => note_play(|| format!("sound {id} (motion {motion}) not loaded")),
            }
        }
        // (A motion's voice only with `kh2_voice`: Reflect's voices load without it.)
        EventKind::Voice(v) if kh2_voice() => {
            if let Some(c) = l.voices.get(&v) {
                c.say();
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
            play(s);
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
        log(&format!("KH2 sound played: {}", what()));
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
    match g.as_ref().and_then(|l| l.voices.get(&v)).and_then(|c| c.say()) {
        Some(clip) => note_play(|| format!("Reflect voice {v} (clip {clip})")),
        None => note_play(|| format!("Reflect voice {v} not loaded")),
    }
}

/// Sora's battle voice `v` (a spell's cast voice, `reflect`), when KH2's voices are on.
pub unsafe fn voice(v: i16) {
    if !kh2_voice() {
        return;
    }
    let Ok(g) = LOADED.try_lock() else { return };
    if let Some(c) = g.as_ref().and_then(|l| l.voices.get(&v)) {
        c.say();
    }
}

/// Sora's hurt line `v` (`hitreact::hurt_line`, KH2's battle quotes): whether it was said.
pub unsafe fn hurt_voice(v: i16) -> bool {
    if !kh2_voice() {
        return false;
    }
    let Ok(g) = LOADED.try_lock() else { return false };
    g.as_ref().and_then(|l| l.voices.get(&v)).and_then(|c| c.say()).is_some()
}

/// Plays a KH2 sound effect at Sora through KH1's effect queue (`kh1bank::play`); when KH1 can't
/// take it now (its bank not back in KH1's list yet after a room load, the queue full), it isn't
/// heard, as KH1 drops its own sounds then (logged).
unsafe fn play(s: &Sound) {
    if !super::kh1bank::play(s.kh1, s.ms) {
        dropped(|| format!("se{}: KH1's effect queue or bank list can't take it now", s.kh1));
    }
}

/// A KH2 sound KH1's engine couldn't take, for the log (the first 20).
fn dropped(what: impl FnOnce() -> String) {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if N.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 20 {
        log(&format!("KH2 sound not played: {}", what()));
    }
}

/// KH2's battle quotes for Sora (the Battle Quotes row / ini `battle_quotes`; `kh2_voice` without
/// it). Every motion's voices are loaded, so the row switches them in game.
pub fn kh2_voice() -> bool {
    super::styles::kh2(super::styles::Kind::BattleQuotes)
}

