//! `kh1-kh2.ini` next to the game, read once when the game starts (written with the defaults
//! the first time; keys it lacks keep their defaults). Each newer, untested-in-game feature has
//! a switch here, so it can be turned off without a new build.

use super::log;
use crate::growth::Mode;
use std::sync::OnceLock;

const INI: &str = "kh1-kh2.ini";
const DEFAULT_INI: &str = "; kh1-kh2 settings (read when the game starts)\r\n\
;\r\n\
; abilities: kh1 = KH2's abilities from KH1's Ability menu, Sora's level and bosses (normal\r\n\
; play); remove = take the KH2 abilities back out of your save (load it, save once, then\r\n\
; uninstall); test = every KH2 movement ability at MAX; all = everything, every modifier\r\n\
abilities = kh1\r\n\
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
; abilities (every KH2 ability move: Upper Slash, Explosion, Aerial Finish...), all (every\r\n\
; move, the plain combo too), off\r\n\
kh2_sounds = abilities\r\n\
kh2_volume = 1.0\r\n\
; music_mute: lowest = the game's lowest Music volume step is silence (KH1's own is still\r\n\
; quiet music); on = no music at any step; off = KH1's own steps. Effects and voices as set\r\n\
music_mute = lowest\r\n\
; damage_scale: times the damage of Sora's KH2 attacks (1.0 = KH1's hit x KH2's power)\r\n\
damage_scale = 1.0\r\n\
; magic_scale: times KH2's Magic-based attacks on top (Explosion, Magnet Burst; they also\r\n\
;   grow with Sora's max MP, +10% a point)\r\n\
magic_scale = 1.0\r\n\
; glide: square = Square in the air glides (as KH1's Superglide); jump = hold jump in the air\r\n\
glide = square\r\n\
; air_cast_fall: how fast Sora falls while Reflect plays in the air, a share of KH1's own\r\n\
;   gravity (0 = he hangs, 1 = KH1's); his fall is stopped as the cast starts\r\n\
air_cast_fall = 1.0\r\n\
; kh2_voice: on = Sora's KH2 battle voice on those moves instead of KH1's; off = KH1's voice\r\n\
; (battle_quotes, below, takes its place once set)\r\n\
kh2_voice = off\r\n\
; bonus_ap: on = (abilities = kh1) Sora's max AP grows by the AP of each KH2 ability he learns\r\n\
bonus_ap = on\r\n\
; auto_aim: off = swings turn toward only KH1's target (the R1 lock or the small cursor);\r\n\
; on = KH2's own pick, the nearest enemy in reach\r\n\
auto_aim = off\r\n\
; walls: what a swing does when it meets a wall. kh1 = as KH1's own swings: the Keyblade's\r\n\
; wall spark and sound where it touches, and the swing goes on; clink = it clinks off the wall\r\n\
; and bounces Sora back (an enemy's guard does that either way); pass = nothing at all\r\n\
walls = kh1\r\n\
; camera: kh2 = KH2's field camera (follows behind, right stick turns and zooms, R3 resets);\r\n\
; kh1 = KH1's own. The Config menu's camera row switches it in game too.\r\n\
camera = kh2\r\n\
; camera_scale: KH2's camera distances and heights times this; 0.89 = KH1 Sora's size against\r\n\
; KH2 Sora's (hips 78.9 / 88.5), so he is framed exactly as in KH2\r\n\
camera_scale = 0.89\r\n\
; camera_lower: KH2's camera (look point and eye together) this much lower (0 = as KH2)\r\n\
camera_lower = 0\r\n\
; config_row: on = the Config menu's Camera Style row; off = no row (the ini alone switches)\r\n\
config_row = on\r\n\
; shortcuts: 4 = KH2's four shortcuts (L1 + the cancel button is the fourth; set it in\r\n\
; Customize); 3 = KH1's three\r\n\
shortcuts = 4\r\n\
; kh2_effects: KH2's own visual effects for these moves, from your KH2 install, drawn in\r\n\
; KH1's frame: magnet_explosion (Magnet Burst, Explosion, Vicinity Break, Aerial Finish, Guard\r\n\
;   Break, Finishing Leap, Aerial Recovery), all (every move), off\r\n\
kh2_effects = magnet_explosion\r\n\
; flip_weapons: Sora's weapons (KH1 item ids) KH2's grip holds the wrong way round, turned half\r\n\
; round in his hand (0x53: the Dream Shield); none = off\r\n\
flip_weapons = 0x53\r\n\
; reflect: on = KH2's Reflect in the Magic menu and the shortcuts once Sora has found it (KH2's\r\n\
; cast, barrier, counter, effects and sounds, from your KH2 install; found after Pot Centipede,\r\n\
; the Hercules Cup and Ice Titan: Reflect, then Reflera, then Reflega; 1 MP); off = KH1's\r\n\
; spells only\r\n\
reflect = on\r\n\
; keyblade_turn: on = every Keyblade held with its teeth up, as KH2 holds it (KH1's grip held\r\n\
; them to the ground under KH2's moves); off = as before\r\n\
keyblade_turn = on\r\n\
; physics: on = Sora's hair, hood and keychain move during KH2's moves; off = rigid\r\n\
physics = on\r\n\
; kh2_hair: on = (physics = on) they move as KH2 animates its own hair, hood and keychain in\r\n\
; each move; off = the older swing and sway (a breeze and gravity) instead\r\n\
kh2_hair = on\r\n\
; drive: on = D-pad left at the battle menu's top flips to a second page (Attack, Magic, Items,\r\n\
; Drive), Drive listing KH2's forms; off = KH1's menu only\r\n\
drive = on\r\n\
; valor_left_blade: on = in Valor Form, its Keyblade in Sora's left hand (KH1's own model of it);\r\n\
; off = nothing of it is installed (turn it off if it misbehaves)\r\n\
valor_left_blade = on\r\n\
; ground_combo / air_combo / magic / air_magic: kh1 or kh2, which game's moves base Sora uses\r\n\
; for each (Drive Forms keep their own); the Config menu switches them in game too\r\n\
ground_combo = kh2\r\n\
air_combo = kh2\r\n\
magic = kh1\r\n\
air_magic = kh1\r\n\
; jump_style / glide_style: kh1 or kh2, KH1's or KH2's jump and glide for base Sora\r\n\
jump_style = kh2\r\n\
glide_style = kh2\r\n\
; roll_style: kh1 or kh2, KH1's or KH2's dodge roll for base Sora\r\n\
roll_style = kh2\r\n\
; misc_anims: kh1 or kh2, KH1's or KH2's animations for base Sora standing, walking, running\r\n\
; and using an item (KH2 still moves him; KH1 runs the item either way)\r\n\
misc_anims = kh2\r\n\
; player_model: kh1 or kh2, draw Sora with KH1's model or KH2's (his KH2 Drive Form models and\r\n\
; KH2's Keyblades where KH2 has the same one)\r\n\
player_model = kh1\r\n\
; battle_quotes: kh1 or kh2, Sora's KH1 or KH2 battle quotes (the Config menu's row; without\r\n\
; this line kh2_voice decides)\r\n\
battle_quotes = kh1\r\n\
; gummi: vanilla = fly the Gummi Ship between worlds; skip = land in the chosen world at once\r\n\
; (the Config menu's Gummi Routes row switches it in game too)\r\n\
gummi = vanilla\r\n\
; hud_toggle_key: the key that hides the whole HUD and shows it again (pagedown, pageup, home,\r\n\
; end, delete, f1..f12, a letter, or a key code such as 0x22); off = no key\r\n\
hud_toggle_key = pagedown\r\n\
; drive_gauge: moves KH2's Drive gauge from its place under Sora's HP circle: right, down, in\r\n\
; KH1's HUD units (384 = the screen's height; negative = left, up)\r\n\
drive_gauge = 0, 0\r\n\
; drive_gauge_debug: on = a bright magenta block over the Drive gauge, to find it on screen\r\n\
drive_gauge_debug = off\r\n";

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
    /// When the music is silent (`music_mute`, `music.rs`).
    pub music_mute: super::music::MusicMute,
    /// Times the damage of Sora's KH2 hits (KH1's damage x KH2's power x this).
    pub damage_scale: f32,
    /// Times KH2's Magic-based hits on top (Explosion, Magnet Burst), which also grow with MP.
    pub magic_scale: f32,
    /// Glide on Square (`glide = square`, default) or held jump (`glide = jump`).
    pub glide_on_square: bool,
    /// Gravity's share while a KH2 cast (Reflect) plays in the air, after its fall speed was
    /// stopped (`air_cast_fall`, 0..1; KH1's spells sink slowly).
    pub air_cast_fall: f32,
    /// (Read for `battle_quotes`'s default; `styles::Kind::BattleQuotes` is what plays.)
    #[allow(dead_code)]
    pub kh2_voice: bool,
    pub bonus_ap: bool,
    pub auto_aim: bool,
    /// What a swing does at a wall (`walls`).
    pub walls: Walls,
    pub kh2_camera: bool,
    pub camera_scale: f32,
    pub camera_lower: f32,
    pub config_row: bool,
    /// KH2's fourth shortcut (`shortcuts = 4`).
    pub four_shortcuts: bool,
    /// Which motions show KH2's own effects: `None` off, `Some(None)` all.
    pub kh2_effects: Option<Option<Vec<u16>>>,
    pub fx_test: bool,
    pub flip_weapons: Vec<u8>,
    /// KH2's Reflect in KH1's magic menu (`reflect = on`).
    pub reflect: bool,
    /// Keyblades turned teeth up in his hand (`keyblade_turn = on`, `pose::keyblade_teeth_up`).
    pub keyblade_turn: bool,
    /// Hair, hood and keychain move under KH2's moves (`physics = on`); off, they keep KH1's rest
    /// pose.
    pub physics: bool,
    /// They follow KH2's own animation of them (`kh2_hair = on`, `kh1_data::secondary`) rather
    /// than the swing (`kh1_data::jiggle`).
    pub kh2_hair: bool,
    /// The second command page with Drive (`drive = on`).
    pub drive: bool,
    /// Valor's Keyblade drawn in his left hand (`valor_left_blade = on`, `leftblade`).
    pub valor_left_blade: bool,
    /// KH2's (true) or KH1's moves per `styles::Kind` (ground combo, air combo, magic, air magic, jump, glide, dodge roll, misc animations, player model, battle quotes).
    pub styles: [bool; 10],
    /// Skip the Gummi Ship's flights (`gummi = skip`, `gummi.rs`; commented out for now).
    #[allow(dead_code)]
    pub gummi_skip: bool,
    /// The key that hides and shows the HUD (`hud_toggle_key`, `hud.rs`); `None`: no key.
    pub hud_toggle_key: Option<u8>,
    /// How far the Drive gauge is moved from its place under Sora's HP circle (`drive_gauge = dx,
    /// dy` in KH1's HUD units, `gaugehud`).
    pub drive_gauge: (i32, i32),
    /// A bright block over the Drive gauge, to find it (`drive_gauge_debug = on`).
    pub drive_gauge_debug: bool,
}

/// s318's `drive_gauge` default (its right end and top then, not a move): read as no move.
const OLD_DRIVE_GAUGE: (i32, i32) = (548, 352);

/// What a swing does when it meets a wall (`walls`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Walls {
    /// As KH1's own swings: the Keyblade's wall spark and sound, and the swing goes on.
    Kh1,
    /// It clinks off and Sora bounces back (the old `wall_clink = on`).
    Clink,
    /// Nothing.
    Pass,
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
            abilities: setting(&text, "abilities").and_then(|v| Mode::parse(&v)).unwrap_or(Mode::Kh1),
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
            music_mute: setting(&text, "music_mute").map_or(super::music::MusicMute::Lowest, |v| super::music::MusicMute::parse(&v)),
            damage_scale: setting(&text, "damage_scale").and_then(|v| v.parse::<f32>().ok()).filter(|v| (0.1..=10.0).contains(v)).unwrap_or(1.0),
            glide_on_square: on(&text, "glide", "square", true),
            air_cast_fall: setting(&text, "air_cast_fall").and_then(|v| v.parse::<f32>().ok()).filter(|v| (0.0..=1.0).contains(v)).unwrap_or(AIR_CAST_FALL),
            magic_scale: setting(&text, "magic_scale").and_then(|v| v.parse::<f32>().ok()).filter(|v| (0.1..=10.0).contains(v)).unwrap_or(1.0),
            kh2_voice: on(&text, "kh2_voice", "on", false),
            bonus_ap: on(&text, "bonus_ap", "on", true),
            auto_aim: on(&text, "auto_aim", "on", false),
            walls: match setting(&text, "walls").as_deref() {
                Some("clink") => Walls::Clink,
                Some("pass") | Some("off") => Walls::Pass,
                _ => Walls::Kh1,
            },
            kh2_camera: on(&text, "camera", "kh2", true),
            config_row: on(&text, "config_row", "on", true),
            four_shortcuts: on(&text, "shortcuts", "4", true),
            camera_lower: setting(&text, "camera_lower").and_then(|v| v.parse::<f32>().ok()).filter(|v| (-200.0..200.0).contains(v)).unwrap_or(0.0),
            camera_scale: setting(&text, "camera_scale").and_then(|v| v.parse::<f32>().ok()).filter(|v| (0.3..3.0).contains(v)).unwrap_or(0.89),
            kh2_effects: match setting(&text, "kh2_effects").as_deref() {
                Some("off") | Some("0") | Some("no") => None,
                Some("all") => Some(None),
                _ => Some(Some(super::kh2fx::DEFAULT_MOTIONS.to_vec())),
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
            reflect: on(&text, "reflect", "on", true),
            keyblade_turn: on(&text, "keyblade_turn", "on", true),
            physics: on(&text, "physics", "on", true),
            kh2_hair: on(&text, "kh2_hair", "on", true),
            drive: on(&text, "drive", "on", true),
            valor_left_blade: on(&text, "valor_left_blade", "on", true),
            styles: [on(&text, "ground_combo", "kh2", true), on(&text, "air_combo", "kh2", true), on(&text, "magic", "kh2", false), on(&text, "air_magic", "kh2", false), on(&text, "jump_style", "kh2", true), on(&text, "glide_style", "kh2", true), on(&text, "roll_style", "kh2", true), on(&text, "misc_anims", "kh2", true), on(&text, "player_model", "kh2", false), on(&text, "battle_quotes", "kh2", on(&text, "kh2_voice", "on", false))],
            gummi_skip: on(&text, "gummi", "skip", false),
            hud_toggle_key: match setting(&text, "hud_toggle_key").map(|v| (crate::keys::parse(&v), v)) {
                None => Some(crate::keys::PAGE_DOWN),
                Some((Some(crate::keys::KeySetting::Key(k)), _)) => Some(k),
                Some((Some(crate::keys::KeySetting::Off), _)) => None,
                Some((None, v)) => {
                    log(&format!("hud_toggle_key = {v}: not a key this knows; Page Down instead"));
                    Some(crate::keys::PAGE_DOWN)
                }
            },
            drive_gauge: match setting(&text, "drive_gauge").and_then(|v| {
                let mut it = v.split([',', ' ']).filter(|t| !t.is_empty()).map(|t| t.parse::<i32>().ok());
                Some((it.next()??, it.next()??))
            }) {
                Some(OLD_DRIVE_GAUGE) => {
                    log("drive_gauge = 548, 352 is s318's place, not a move: no move (it is a move from under Sora's HP circle now)");
                    (0, 0)
                }
                Some((x, y)) if (-300..=300).contains(&x) && (-300..=300).contains(&y) => (x, y),
                Some((x, y)) => {
                    log(&format!("drive_gauge = {x}, {y}: a move of at most 300 each way; no move"));
                    (0, 0)
                }
                None => (0, 0),
            },
            drive_gauge_debug: on(&text, "drive_gauge_debug", "on", false),
        };
        log(&format!("settings ({path}): {s:?}"));
        s
    })
}

/// Keeps a Config menu choice in the ini (`key = value`): that line changed, or added, the rest
/// as it was.
pub fn save(key: &str, value: &str) {
    let path = format!("{}\\{INI}", super::exe_dir());
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut found = false;
    let mut out: Vec<String> = text
        .lines()
        .map(|l| {
            let t = l.trim_start();
            if !t.starts_with(';') && t.split('=').next().is_some_and(|k| k.trim().eq_ignore_ascii_case(key)) {
                found = true;
                format!("{key} = {value}")
            } else {
                l.to_string()
            }
        })
        .collect();
    if !found {
        out.push(format!("{key} = {value}"));
    }
    let _ = std::fs::write(&path, out.join("\r\n") + "\r\n");
}

/// The default `air_cast_fall`: full gravity (the user: Reflect should stop his fall for one
/// frame, "then after have it fall to the ground with soras normal gravity physics. Right now
/// sora just slowly falls to the ground which is not accurate to how kh1 magic works").
pub const AIR_CAST_FALL: f32 = 1.0;
