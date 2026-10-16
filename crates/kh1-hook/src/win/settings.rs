//! `kh1-kh2.ini` next to the game, read once when the game starts (written with the defaults
//! the first time; keys it lacks keep their defaults). Each newer, untested-in-game feature has
//! a switch here, so it can be turned off without a new build.

use super::log;
use crate::growth::Mode;
use std::sync::OnceLock;

const INI: &str = "kh1-kh2.ini";
const DEFAULT_INI: &str = "; kh1-kh2 settings (read when the game starts)\r\n\
;\r\n\
; abilities: test = every KH2 movement ability at MAX (testing), kh1 = KH2's abilities from\r\n\
; KH1's Ability menu and Sora's level, all = everything and every combo modifier\r\n\
abilities = test\r\n\
; hurt: kh2 = Sora takes hits KH2's way (dodge-roll invulnerability, KH2 guard, KH2\r\n\
; reactions); kh1 = KH1's own hit reactions\r\n\
hurt = kh2\r\n\
; carry: on = moving platforms carry Sora and KH1's pushes move him while KH2 has him\r\n\
carry = on\r\n\
; ceilings: on = ceilings stop KH2's jumps (KH1's collision), off = they don't\r\n\
ceilings = on\r\n\
; grabs: on = KH1's ledge grabs, vine / rope grabs and swimming take Sora from KH2's jumps\r\n\
grabs = on\r\n\
; commands: on = KH1's commands other than Attack (magic, items, specials, summons) take\r\n\
; Sora from KH2 until KH1 has him standing again (in the air: KH1's fall from there)\r\n\
commands = on\r\n\
; swing_fx: on = KH2's swings play KH1's Keyblade trail, swing effect and Sora's voice\r\n\
swing_fx = on\r\n\
; kh2_sounds: KH2's own sound effects for these moves, from your KH2 install:\r\n\
; magnet_explosion (Magnet Burst and Explosion), all (every move), off\r\n\
kh2_sounds = magnet_explosion\r\n\
kh2_volume = 1.0\r\n\
; kh2_voice: on = Sora's KH2 battle voice on those moves instead of KH1's; off = KH1's voice\r\n\
kh2_voice = off\r\n\
; bonus_ap: on = (abilities = kh1) Sora's max AP grows by the AP of each KH2 ability he learns\r\n\
bonus_ap = on\r\n\
; auto_aim: off = swings turn toward only KH1's target (the R1 lock or the small cursor);\r\n\
; on = KH2's own pick, the nearest enemy in reach\r\n\
auto_aim = off\r\n\
; wall_clink: on = a swing that meets a wall clinks off it and bounces Sora back; off = it passes\r\n\
wall_clink = off\r\n\
; camera: kh2 = KH2's field camera (follows behind, right stick turns and zooms, R3 resets);\r\n\
; kh1 = KH1's own. The Config menu's camera row switches it in game too.\r\n\
camera = kh2\r\n\
; camera_scale: KH2's camera distances and heights times this (1.0: KH2's own)\r\n\
camera_scale = 1.0\r\n\
; config_row: on = the Config menu's Camera Style row; off = no row (the ini alone switches)\r\n\
config_row = on\r\n\
; kh2_effects: KH2's own visual effects for these moves, from your KH2 install, drawn in\r\n\
; KH1's frame: magnet_explosion (Magnet Burst and Explosion), all (every move), off\r\n\
kh2_effects = magnet_explosion\r\n\
; fx_test: on = start Magnet Burst's effect at Sora every 4 seconds (checks the renderer)\r\n\
fx_test = off\r\n\
; flip_weapons: Sora's weapons (KH1 item ids) KH2's grip holds the wrong way round, turned half\r\n\
; round in his hand (0x53: the Dream Shield); none = off\r\n\
flip_weapons = 0x53\r\n";

#[derive(Debug)]
pub struct Settings {
    pub abilities: Mode,
    pub hurt: bool,
    pub carry: bool,
    pub ceilings: bool,
    pub grabs: bool,
    pub commands: bool,
    pub swing_fx: bool,
    /// Which motions play KH2's own sounds: `None` off, `Some(None)` all.
    pub kh2_sounds: Option<Option<Vec<u16>>>,
    pub kh2_volume: f32,
    pub kh2_voice: bool,
    pub bonus_ap: bool,
    pub auto_aim: bool,
    pub wall_clink: bool,
    pub kh2_camera: bool,
    pub camera_scale: f32,
    pub config_row: bool,
    /// Which motions show KH2's own effects: `None` off, `Some(None)` all.
    pub kh2_effects: Option<Option<Vec<u16>>>,
    pub fx_test: bool,
    pub flip_weapons: Vec<u8>,
}

static SETTINGS: OnceLock<Settings> = OnceLock::new();

/// The ini's `key = value`, if set.
fn setting(text: &str, key: &str) -> Option<String> {
    text.lines().filter_map(|l| l.split(';').next()).filter_map(|l| l.split_once('=')).find(|(k, _)| k.trim().eq_ignore_ascii_case(key)).map(|(_, v)| v.trim().to_ascii_lowercase())
}

fn on(text: &str, key: &str, on_word: &str, default: bool) -> bool {
    match setting(text, key) {
        Some(v) => v == on_word || v == "on" || v == "true" || v == "1" || v == "yes",
        None => default,
    }
}

pub fn get() -> &'static Settings {
    SETTINGS.get_or_init(|| {
        let path = format!("{}\\{INI}", super::exe_dir());
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => {
                let _ = std::fs::write(&path, DEFAULT_INI);
                DEFAULT_INI.to_string()
            }
        };
        let s = Settings {
            abilities: setting(&text, "abilities").and_then(|v| Mode::parse(&v)).unwrap_or(Mode::Test),
            hurt: on(&text, "hurt", "kh2", true),
            carry: on(&text, "carry", "on", true),
            ceilings: on(&text, "ceilings", "on", true),
            grabs: on(&text, "grabs", "on", true),
            commands: on(&text, "commands", "on", true),
            swing_fx: on(&text, "swing_fx", "on", true),
            kh2_sounds: match setting(&text, "kh2_sounds").as_deref() {
                Some("off") | Some("0") | Some("no") => None,
                Some("all") => Some(None),
                _ => Some(Some(super::kh2sound::DEFAULT_MOTIONS.to_vec())),
            },
            kh2_volume: setting(&text, "kh2_volume").and_then(|v| v.parse().ok()).unwrap_or(1.0),
            kh2_voice: on(&text, "kh2_voice", "on", false),
            bonus_ap: on(&text, "bonus_ap", "on", true),
            auto_aim: on(&text, "auto_aim", "on", false),
            wall_clink: on(&text, "wall_clink", "on", false),
            kh2_camera: on(&text, "camera", "kh2", true),
            config_row: on(&text, "config_row", "on", true),
            camera_scale: setting(&text, "camera_scale").and_then(|v| v.parse::<f32>().ok()).filter(|v| (0.3..3.0).contains(v)).unwrap_or(1.0),
            kh2_effects: match setting(&text, "kh2_effects").as_deref() {
                Some("off") | Some("0") | Some("no") => None,
                Some("all") => Some(None),
                _ => Some(Some(super::kh2sound::DEFAULT_MOTIONS.to_vec())),
            },
            fx_test: on(&text, "fx_test", "on", false),
            flip_weapons: match setting(&text, "flip_weapons") {
                None => vec![0x53],
                Some(v) => v
                    .split([',', ' '])
                    .filter_map(|t| {
                        let t = t.trim();
                        t.strip_prefix("0x").map_or_else(|| t.parse().ok(), |h| u8::from_str_radix(h, 16).ok())
                    })
                    .collect(),
            },
        };
        log(&format!("settings ({path}): {s:?}"));
        s
    })
}

/// Keeps the Config menu's camera choice in the ini (`camera = kh1 / kh2`): that line changed,
/// or added, the rest as it was.
pub fn save_camera(kh2: bool) {
    let path = format!("{}\\{INI}", super::exe_dir());
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let value = if kh2 { "kh2" } else { "kh1" };
    let mut found = false;
    let mut out: Vec<String> = text
        .lines()
        .map(|l| {
            let t = l.trim_start();
            if !t.starts_with(';') && t.split('=').next().is_some_and(|k| k.trim().eq_ignore_ascii_case("camera")) {
                found = true;
                format!("camera = {value}")
            } else {
                l.to_string()
            }
        })
        .collect();
    if !found {
        out.push(format!("camera = {value}"));
    }
    let _ = std::fs::write(&path, out.join("\r\n") + "\r\n");
}
