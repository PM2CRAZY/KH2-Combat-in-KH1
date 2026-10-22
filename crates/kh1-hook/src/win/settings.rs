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
; damage_scale: times the damage of Sora's KH2 attacks (1.0 = KH1's hit x KH2's power)\r\n\
damage_scale = 1.0\r\n\
; magic_scale: times KH2's Magic-based attacks on top (Explosion, Magnet Burst; they also\r\n\
;   grow with Sora's max MP, +10% a point)\r\n\
magic_scale = 1.0\r\n\
; glide: square = Square in the air glides (as KH1's Superglide); jump = hold jump in the air\r\n\
glide = square\r\n\
; air_cast_fall: how fast Sora falls while Reflect plays in the air, a share of KH1's own\r\n\
;   gravity (0 = he hangs, 1 = KH1's); his fall is stopped as the cast starts (physics = kh1,\r\n\
;   and Valor / Wisdom; with physics = kh2 base Sora's air casts are KH2's own)\r\n\
air_cast_fall = 1.0\r\n\
; kh2_voice: on = Sora's KH2 battle voice on those moves instead of KH1's; off = KH1's voice\r\n\
; (battle_quotes, below, takes its place once set)\r\n\
kh2_voice = off\r\n\
; bonus_ap: on = (abilities = kh1) Sora's max AP grows by the AP of each KH2 ability he learns\r\n\
bonus_ap = on\r\n\
; auto_aim: off = swings turn toward only KH1's target (the R1 lock or the small cursor);\r\n\
; on = KH2's own pick, the nearest enemy in reach\r\n\
auto_aim = off\r\n\
; wall_clink: on = a swing that meets a wall clinks off it and bounces Sora back; off = it passes\r\n\
wall_clink = on\r\n\
; camera: kh2 = KH2's field camera (follows behind, right stick turns and zooms, R3 resets);\r\n\
; kh1 = KH1's own. The Config menu's camera row switches it in game too.\r\n\
camera = kh2\r\n\
; camera_scale: KH2's camera distances and heights times this; 0.89 = KH1 Sora's size against\r\n\
; KH2 Sora's (hips 78.9 / 88.5), so he is framed exactly as in KH2\r\n\
camera_scale = 0.89\r\n\
; shared: on = KH1's own states run base Sora (idle, walk, run, jump, fall, landing), with\r\n\
;   KH2's animations where a move style is KH2 (and KH2's physics with physics = kh2); KH2\r\n\
;   borrows him only for its own moves; off = KH2's controller has him whenever he stands,\r\n\
;   walks or runs (older builds)\r\n\
shared = on\r\n\
; kh1_slots: on = KH2's moves that have one play as new slots in KH1's own motion table (KH1's\r\n\
;   clock, events, hit windows and blends; for now KH2's ground combo, 151 / 152 / 154, and air combo 183 / 184 / 181, at Base\r\n\
;   / Aerial Base Combo KH2); a list of KH2 motions (e.g. 152) = only those; off = KH2's motions drawn over\r\n\
;   KH1's (as before)\r\n\
kh1_slots = on\r\n\
; camera_lower: KH2's camera (look point and eye together) this much lower (0 = as KH2)\r\n\
camera_lower = 0\r\n\
; camera_calls: kh2 = (camera = kh2) KH2's camera runs as KH1's own camera mode: KH1's moves,\r\n\
;   its ledge / climbing / pole cameras, its lock-on switches and its door walks keep KH2's\r\n\
;   camera and field of view (events and cutscenes stay KH1's); kh1 = they hand the view to\r\n\
;   KH1's camera (older builds)\r\n\
camera_calls = kh2\r\n\
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
; reflect: on = KH2's Reflect in the Magic menu and the shortcuts once Sora has Aero (KH2's\r\n\
; cast, barrier, counter, effects and sounds, from your KH2 install; Reflera / Reflega with\r\n\
; Aerora / Aeroga; 1 MP); off = KH1's spells only\r\n\
reflect = on\r\n\
; keyblade_turn: on = every Keyblade held with its teeth up, as KH2 holds it (KH1's grip held\r\n\
; them to the ground under KH2's moves); off = as before\r\n\
keyblade_turn = on\r\n\
; hair_physics: on = Sora's hair, hood and keychain move during KH2's moves; off = rigid\r\n\
hair_physics = on\r\n\
; kh2_hair: on = (hair_physics = on) they move as KH2 animates its own hair, hood and keychain in\r\n\
; each move; off = the older swing and sway (a breeze and gravity) instead\r\n\
kh2_hair = on\r\n\
; drive: on = D-pad left at the battle menu's top flips to a second page (Attack, Magic, Items,\r\n\
; Drive), Drive listing KH2's forms; off = KH1's menu only\r\n\
drive = on\r\n\
; valor_left_blade: on = in Valor Form, its Keyblade in Sora's left hand (KH1's own model of it);\r\n\
; off = nothing of it is installed (turn it off if it misbehaves)\r\n\
valor_left_blade = on\r\n\
; physics: kh1 or kh2, whose physics move base Sora in every move (run speeds, jump arc,\r\n\
;   gravity, air drift, attack travel), whatever the move styles below (they choose the\r\n\
;   animations); Drive Forms always KH2's. The Config menu's Physics row switches it in game\r\n\
physics = kh2\r\n\
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
; trace: on = the last trace_seconds of Sora's frames (position, KH1's and KH2's states, the pad)\r\n\
; are kept and written to the kh1-kh2-traces folder on Select + Triangle, or on their own when\r\n\
; something looks wrong; always = every frame is also written to kh1-kh2.trace.csv; off = none\r\n\
trace = on\r\n\
; trace_seconds: how many seconds before the dump the trace clip holds (1 to 120, default 10)\r\n\
trace_seconds = 10\r\n";

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
    pub wall_clink: bool,
    pub kh2_camera: bool,
    pub camera_scale: f32,
    pub camera_lower: f32,
    /// KH1's gameplay camera calls drive KH2's camera (`camera_calls = kh2`, `camera.rs`).
    pub camera_calls_kh2: bool,
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
    /// Hair, hood and keychain move under KH2's moves (`hair_physics = on`; an older ini's
    /// `physics = on / off`); off, they keep KH1's rest pose.
    pub physics: bool,
    /// Whose physics move base Sora (`physics = kh1 | kh2`, KH2's by default;
    /// `shared::physics::rule`).
    pub physics_rule: crate::kh1phys::Rule,
    /// They follow KH2's own animation of them (`kh2_hair = on`, `kh1_data::secondary`) rather
    /// than the swing (`kh1_data::jiggle`).
    pub kh2_hair: bool,
    /// The second command page with Drive (`drive = on`).
    pub drive: bool,
    /// Valor's Keyblade drawn in his left hand (`valor_left_blade = on`, `leftblade`).
    pub valor_left_blade: bool,
    /// KH2's (true) or KH1's moves per `styles::Kind` (ground combo, air combo, magic, air magic, jump, glide, dodge roll, misc animations, player model, battle quotes).
    pub styles: [bool; 10],
    /// Skip the Gummi Ship's flights (`gummi = skip`, `gummi.rs`).
    pub gummi_skip: bool,
    /// KH1's own states own base Sora (`shared = on`, `shared.rs`): KH2's animations on KH1's
    /// states per move style (its physics by `physics_rule`), KH2's controller only for KH2's own
    /// moves. Off: the old
    /// way (KH2's controller has him whenever KH1 shows him standing, walking or running).
    pub shared: bool,
    /// KH2's moves as new slots in KH1 Sora's motion table (`kh2slots.rs`): `None` off (the
    /// default), `Some(None)` every move that has one (`on`), else only these KH2 motions.
    pub kh1_slots: Option<Option<Vec<u16>>>,
    pub trace: super::trace::Mode,
    /// How many seconds of Sora's frames a trace dump holds (`trace_seconds`, 1..=120, 10 by default).
    pub trace_seconds: u32,
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
            damage_scale: setting(&text, "damage_scale").and_then(|v| v.parse::<f32>().ok()).filter(|v| (0.1..=10.0).contains(v)).unwrap_or(1.0),
            // (Glide on Square held off, t319: a glide is the jump button after the Aerial Dodge. The
            // `glide = square` setting and the Square glide code stay, behind this flag, for later.)
            // glide_on_square: on(&text, "glide", "square", true),
            glide_on_square: false,
            air_cast_fall: setting(&text, "air_cast_fall").and_then(|v| v.parse::<f32>().ok()).filter(|v| (0.0..=1.0).contains(v)).unwrap_or(AIR_CAST_FALL),
            magic_scale: setting(&text, "magic_scale").and_then(|v| v.parse::<f32>().ok()).filter(|v| (0.1..=10.0).contains(v)).unwrap_or(1.0),
            kh2_voice: on(&text, "kh2_voice", "on", false),
            bonus_ap: on(&text, "bonus_ap", "on", true),
            auto_aim: on(&text, "auto_aim", "on", false),
            wall_clink: on(&text, "wall_clink", "on", true),
            kh2_camera: on(&text, "camera", "kh2", true),
            config_row: on(&text, "config_row", "on", true),
            four_shortcuts: on(&text, "shortcuts", "4", true),
            camera_lower: setting(&text, "camera_lower").and_then(|v| v.parse::<f32>().ok()).filter(|v| (-200.0..200.0).contains(v)).unwrap_or(0.0),
            camera_scale: setting(&text, "camera_scale").and_then(|v| v.parse::<f32>().ok()).filter(|v| (0.3..3.0).contains(v)).unwrap_or(0.89),
            camera_calls_kh2: on(&text, "camera_calls", "kh2", true),
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
            physics: hair_physics(&text),
            physics_rule: crate::kh1phys::Rule::parse(setting(&text, "physics").as_deref()),
            kh2_hair: on(&text, "kh2_hair", "on", true),
            drive: on(&text, "drive", "on", true),
            valor_left_blade: on(&text, "valor_left_blade", "on", true),
            styles: [on(&text, "ground_combo", "kh2", false), on(&text, "air_combo", "kh2", false), on(&text, "magic", "kh2", false), on(&text, "air_magic", "kh2", false), on(&text, "jump_style", "kh2", false), on(&text, "glide_style", "kh2", false), on(&text, "roll_style", "kh2", false), on(&text, "misc_anims", "kh2", false), on(&text, "player_model", "kh2", false), on(&text, "battle_quotes", "kh2", on(&text, "kh2_voice", "on", false))],
            gummi_skip: on(&text, "gummi", "skip", false),
            shared: on(&text, "shared", "on", true),
            kh1_slots: match setting(&text, "kh1_slots").as_deref() {
                Some("off") | Some("0") | Some("no") => None,
                None | Some("on") | Some("all") | Some("yes") => Some(None),
                Some(v) => Some(Some(v.split([',', ' ']).filter_map(|t| t.trim().parse().ok()).collect())),
            },
            trace: super::trace::Mode::parse(setting(&text, "trace").as_deref()),
            trace_seconds: setting(&text, "trace_seconds").and_then(|v| v.trim_end_matches('s').trim().parse::<f32>().ok()).map_or(super::trace::DEFAULT_SECONDS, |v| (v.round() as u32).clamp(1, super::trace::MAX_SECONDS)),
        };
        log(&format!("settings ({path}): {s:?}"));
        s
    })
}

/// The hair's switch: `hair_physics`, or an older ini's `physics = on / off` (before `physics`
/// became the physics setting, `kh1 | kh2`, which leaves the hair on).
fn hair_physics(text: &str) -> bool {
    match setting(text, "hair_physics") {
        Some(_) => on(text, "hair_physics", "on", true),
        None => !matches!(setting(text, "physics").as_deref(), Some("off" | "0" | "no" | "false")),
    }
}

/// Keeps the physics setting in the ini (the Config menu's Physics row). An older ini whose
/// `physics` line is the hair's switch gets that as `hair_physics` first, so it isn't lost.
pub fn save_physics(r: crate::kh1phys::Rule) {
    let path = format!("{}\\{INI}", super::exe_dir());
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    if setting(&text, "hair_physics").is_none() {
        save("hair_physics", if hair_physics(&text) { "on" } else { "off" });
    }
    save("physics", r.key());
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
