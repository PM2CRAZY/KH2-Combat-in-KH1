//! Sora's per-frame trace (the user: "have the log keep track of kh1s soras position, kh2s soras
//! position, the current anim state, etc... anything that would help you debug soras current
//! actions and physics without me having to show you a video").
//!
//! Every one of Sora's physics frames is one row: who has him (KH1's states or KH2's controller),
//! where he is and how far he moved, what KH2 asked to move him by, KH1's actor (state, motion,
//! height above the floor, collision flags, velocity, jump arc), KH2's side (state, motion and
//! frame, in the air, on top of something, facing, combo, lock-on), the pad, and the log lines
//! written during that frame. The last `trace_seconds` (the ini's, 10 s by default) of rows stay in memory and are written
//! to `kh1-kh2-traces\` next to the game as a CSV:
//! - on Select + Triangle (the user's pick; Select is KH1's first-person view, so while Triangle
//!   is held KH1 doesn't see Select: `hides_select`),
//! - on their own when something looks wrong (`anomaly`): a jump in his position in one frame,
//!   under the floor, KH1 and KH2 trading him back and forth, a NaN. Those are written
//!   `AUTO_DELAY` frames later, so what followed is in the file too.
//!
//! Each row also has the camera (`camera::seen`, the `cam_*` and `kh2cam_*` columns): which camera
//! placed it (KH2's or KH1's, and why not KH2's), the eye and look point, their motion, the eye's
//! distance and height from Sora, KH1's yaw / pitch / distance / field of view / blend, and KH2's
//! camera's own yaw, goal yaw, distance and the sticks it read.
//!
//! Each row also has the Config menu's KH1 / KH2 rows as they were on that frame (`config_now`:
//! Camera Style, Physics, the move styles) and whose physics numbers were moving him then
//! (`physics_now`: KH2's in a Drive Form whatever the Physics row says).
//!
//! `trace` in `kh1-kh2.ini`: `on` (the default: memory and dumps), `always` (every row also
//! written to `kh1-kh2.trace.csv` as it goes), `off` (nothing kept).
//!
//! (t354) What a row holds is chosen by category (`cat`, `ITEMS`, `GROUPS`; the debug menu's
//! Trace Recorder page, `debugmenu`, and the ini's `trace_*` keys): a category that is off is
//! neither gathered (its producer skips its work: the camera's record, the draw note, the feet
//! hook, KH2's side, the hit-effect lines) nor written (its columns leave the CSV's header). The
//! frame's identity (frame .. event) is always there. Each row keeps the categories it was made
//! with, so a dump spanning a change writes the current columns, empty where a row lacks them.
//! The mode, seconds and categories are kept live here (`refresh`, `set_*`): the menu's change
//! shows at once, the ini is written beside it and read back when it reloads.
//!
//! Positions are KH2's way up (`up` = -KH1's y). `moved` is where he ended the frame less where
//! he started it; `asked` is KH2's displacement (and a platform's carry) over the frame's 60 Hz
//! steps, empty on KH1's frames: the two differ where KH1's mover (walls, slopes, floors) had its say.

use super::log;
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::io::Write as _;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

/// How much is kept for a dump when the ini doesn't say (`trace_seconds`).
pub const DEFAULT_SECONDS: u32 = 10;
/// The longest clip the ini can ask for.
pub const MAX_SECONDS: u32 = 120;
/// At most this many rows a second kept (120 game frames a second).
const ROWS_PER_SECOND: usize = 120;

/// How much is kept for a dump, in ms (`trace_seconds` in the ini, or the debug menu's).
fn keep_ms() -> u64 {
    seconds() as u64 * 1000
}

/// The trace's categories (bits of `Row::cats` and of the live mask).
pub mod cat {
    pub const MOVEMENT: u32 = 1 << 0;
    pub const KH1_STATE: u32 = 1 << 1;
    pub const KH2_STATE: u32 = 1 << 2;
    pub const FORM: u32 = 1 << 3;
    pub const DRAW: u32 = 1 << 4;
    pub const FEET: u32 = 1 << 5;
    pub const KH1_CAMERA: u32 = 1 << 6;
    pub const KH2_CAMERA: u32 = 1 << 7;
    pub const INPUT: u32 = 1 << 8;
    pub const CONFIG: u32 = 1 << 9;
    pub const LOG: u32 = 1 << 10;
    pub const HIT_FX: u32 = 1 << 11;
    pub const SOUNDS: u32 = 1 << 12;
    pub const ALL: u32 = (1 << 13) - 1;
    /// The events column's sources.
    pub const EVENTS: u32 = LOG | HIT_FX | SOUNDS;
}

/// One category: its bit, its ini key and its name in the debug menu.
pub struct Item {
    pub bit: u32,
    pub key: &'static str,
    pub name: &'static str,
}

/// Every category, in the debug menu's order.
pub const ITEMS: [Item; 13] = [
    Item { bit: cat::MOVEMENT, key: "trace_movement", name: "Movement & Physics" },
    Item { bit: cat::KH1_STATE, key: "trace_kh1_state", name: "KH1 State" },
    Item { bit: cat::KH2_STATE, key: "trace_kh2_state", name: "KH2 State" },
    Item { bit: cat::FORM, key: "trace_form", name: "Form" },
    Item { bit: cat::DRAW, key: "trace_draw", name: "Animation Draw" },
    Item { bit: cat::FEET, key: "trace_feet", name: "Feet / IK" },
    Item { bit: cat::KH1_CAMERA, key: "trace_kh1_camera", name: "KH1 Camera" },
    Item { bit: cat::KH2_CAMERA, key: "trace_kh2_camera", name: "KH2 Camera" },
    Item { bit: cat::INPUT, key: "trace_input", name: "Input" },
    Item { bit: cat::CONFIG, key: "trace_config", name: "Config" },
    Item { bit: cat::LOG, key: "trace_log", name: "Log Lines" },
    Item { bit: cat::HIT_FX, key: "trace_hit_fx", name: "Hit Effects" },
    Item { bit: cat::SOUNDS, key: "trace_sounds", name: "Sounds" },
];

/// The debug menu's groups of categories (indices into `ITEMS`).
pub struct Group {
    pub name: &'static str,
    pub items: &'static [usize],
}

pub const GROUPS: [Group; 6] = [
    Group { name: "KH1 Actor", items: &[0, 1] },
    Group { name: "KH2 Controller", items: &[2, 3] },
    Group { name: "Pose", items: &[4, 5] },
    Group { name: "Camera", items: &[6, 7] },
    Group { name: "Input & Config", items: &[8, 9] },
    Group { name: "Events", items: &[10, 11, 12] },
];

/// The live settings (from the ini, or the debug menu's since): the mode (0 off, 1 on, 2
/// always), the seconds, the categories, the sound log; and the settings they were read from.
static LIVE_MODE: AtomicU32 = AtomicU32::new(1);
static LIVE_SECONDS: AtomicU32 = AtomicU32::new(DEFAULT_SECONDS);
static LIVE_CATS: AtomicU32 = AtomicU32::new(cat::ALL);
static LIVE_SOUND: AtomicBool = AtomicBool::new(true);
/// (t371) The seconds count game frames (`game_frames`), not real time.
static LIVE_GAME_FRAMES: AtomicBool = AtomicBool::new(false);
static LIVE_FROM: AtomicUsize = AtomicUsize::new(0);

/// Reads the ini's values again when the settings were reloaded (the ini saved: by the menu,
/// which set them here already, or by hand).
pub fn refresh() {
    let s = super::settings::get();
    let id = s as *const _ as usize;
    if LIVE_FROM.swap(id, Ordering::Relaxed) != id {
        LIVE_MODE.store(match s.trace { Mode::Off => 0, Mode::On => 1, Mode::Always => 2 }, Ordering::Relaxed);
        LIVE_SECONDS.store(s.trace_seconds, Ordering::Relaxed);
        LIVE_CATS.store(s.trace_cats, Ordering::Relaxed);
        LIVE_SOUND.store(s.sound_log, Ordering::Relaxed);
        LIVE_GAME_FRAMES.store(s.trace_game_frames, Ordering::Relaxed);
    }
}

/// The trace's mode now.
pub fn mode() -> Mode {
    match LIVE_MODE.load(Ordering::Relaxed) {
        0 => Mode::Off,
        2 => Mode::Always,
        _ => Mode::On,
    }
}

/// How many seconds a dump holds now.
pub fn seconds() -> u32 {
    LIVE_SECONDS.load(Ordering::Relaxed).clamp(1, MAX_SECONDS)
}

/// The categories recorded now.
pub fn cats() -> u32 {
    LIVE_CATS.load(Ordering::Relaxed)
}

/// KH1's sounds go to the log now (`soundlog`).
pub fn sound_log() -> bool {
    LIVE_SOUND.load(Ordering::Relaxed)
}

/// (t371) The clip's seconds count game frames (the ini's `trace_game_frames`, the Trace Recorder
/// page's Game Frames row): `seconds` x 60 of the 60 Hz frames KH1 has stepped (`Row::ticks`, 2 a
/// frame at 30 fps), so a frame-advanced clip (`frameadv`) holds as much play as one at full
/// speed. Off: `seconds` of real time (`Row::wall_ms`).
pub fn game_frames() -> bool {
    LIVE_GAME_FRAMES.load(Ordering::Relaxed)
}

/// Whether any of `bits` is being recorded (the trace on and the category on): producers ask
/// before gathering.
pub fn wants(bits: u32) -> bool {
    LIVE_MODE.load(Ordering::Relaxed) != 0 && LIVE_CATS.load(Ordering::Relaxed) & bits != 0
}

/// The debug menu's changes: live at once, and kept in the ini.
pub fn set_mode(m: Mode) {
    LIVE_MODE.store(match m { Mode::Off => 0, Mode::On => 1, Mode::Always => 2 }, Ordering::Relaxed);
    ENABLED.store(m != Mode::Off, Ordering::Relaxed);
    super::settings::save("trace", match m { Mode::Off => "off", Mode::On => "on", Mode::Always => "always" });
    log(&format!("debug menu: trace {m:?}"));
}

pub fn set_seconds(n: u32) {
    let n = n.clamp(1, MAX_SECONDS);
    LIVE_SECONDS.store(n, Ordering::Relaxed);
    super::settings::save("trace_seconds", &n.to_string());
}

pub fn set_sound_log(on: bool) {
    LIVE_SOUND.store(on, Ordering::Relaxed);
    super::settings::save("sound_log", if on { "on" } else { "off" });
    log(&format!("debug menu: sound log {}", if on { "on" } else { "off" }));
}

pub fn set_game_frames(on: bool) {
    if LIVE_GAME_FRAMES.swap(on, Ordering::Relaxed) != on {
        super::settings::save("trace_game_frames", if on { "on" } else { "off" });
        log(&format!("trace: seconds count {}", if on { "game frames" } else { "real time" }));
    }
}

/// Turns the categories in `bits` on or off (each item's ini key written).
pub fn set_cats(bits: u32, on: bool) {
    let now = if on { cats() | bits } else { cats() & !bits };
    LIVE_CATS.store(now, Ordering::Relaxed);
    for it in ITEMS.iter().filter(|it| it.bit & bits != 0) {
        super::settings::save(it.key, if on { "on" } else { "off" });
    }
    log(&format!("debug menu: trace categories {now:#06x}"));
}
/// An anomaly's dump waits this many game frames (what followed it goes in too).
const AUTO_DELAY: u32 = 60;
/// The same anomaly dumps again only after this long.
const AUTO_COOLDOWN_MS: u64 = 10_000;
/// At most this many dumps in a session (an anomaly that never stops can't fill the disk).
const MAX_DUMPS: u32 = 40;
/// A move this long in one 60 Hz step is a jump in his position (KH2's run is ~8 a step, its
/// fastest moves ~30).
const JUMP_PER_STEP: f32 = 150.0;
/// This far under the floor (KH1's height above it, `+0x3D4`) for `UNDER_FRAMES` frames in a row.
const UNDER_BY: f32 = 30.0;
const UNDER_FRAMES: u32 = 3;
/// This many changes of who has him within `FLIP_WINDOW` rows.
const FLIPS: usize = 8;
const FLIP_WINDOW: usize = 60;
/// Frames further apart than this (ms) are a pause: no jump is read across them.
const GAP_MS: u64 = 250;

/// The pad's own buttons (PS2 order, `probe::pad`): Select and Triangle.
pub const RAW_SELECT: u16 = 0x0001;
pub const RAW_TRIANGLE: u16 = 0x1000;
/// Select in KH1's merged button state (`camera::SELECT`, the keyboard's too).
pub const KH1_SELECT: u32 = 0x8000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Off,
    On,
    Always,
}

impl Mode {
    pub fn parse(v: Option<&str>) -> Mode {
        match v {
            Some("off" | "0" | "no" | "false") => Mode::Off,
            Some("always" | "stream" | "all") => Mode::Always,
            _ => Mode::On,
        }
    }
}

/// KH2's side of a row (its controller's state after the frame).
#[derive(Clone, Default)]
pub struct Kh2 {
    pub state: String,
    pub motion: u16,
    pub frame: u32,
    pub air: bool,
    pub grounded: bool,
    pub on_top: Option<f32>,
    pub facing: f32,
    pub combo: u8,
    pub lock: Option<u32>,
    pub invulnerable: bool,
    pub guarding: bool,
}

/// One of Sora's frames.
#[derive(Clone, Default)]
pub struct Row {
    pub frame: u64,
    pub ticks: u64,
    pub steps: u32,
    pub ms: u64,
    /// (t371) Real time (ms), set by `frame`: the clip's window when it counts real time. (`ms`
    /// is the mod's clock, which stops while Frame Advance holds the game, `frameadv::clock_ms`.)
    pub wall_ms: u64,
    /// "KH2", "KH1" or "handoff" (`takeover::FRAME_KIND`).
    pub owner: &'static str,
    /// KH1's own states with KH2's moves sharing them (`shared::active`).
    pub shared: bool,
    pub world: u32,
    pub room: u32,
    pub event: bool,
    /// The Config menu's rows, bit per `CONFIG` column (set: KH2), filled by `frame`.
    pub config: u32,
    pub pos: [f32; 3],
    pub moved: [f32; 3],
    pub asked: Option<[f32; 3]>,
    pub above: f32,
    pub floor_up: f32,
    pub collision: u32,
    pub stand_on: u32,
    pub platform: bool,
    pub yaw: f32,
    pub kh1_state: i32,
    pub kh1_motion: u32,
    pub flags: u32,
    pub combo_window: u32,
    pub velocity: [f32; 4],
    pub arc: [f32; 4],
    pub push: [f32; 3],
    pub kh2: Option<Kh2>,
    pub form: Option<String>,
    pub buttons: u16,
    pub kh1_buttons: u32,
    /// Right x, right y, left x, left y (0x80 centred).
    pub sticks: [u8; 4],
    /// What `pose` drew this frame (`pose::drawn`), filled by `frame`.
    pub draw: super::pose::Drawn,
    /// The camera as its last follow / lock-on call left it, filled by `frame`.
    pub cam: super::camera::Seen,
    /// KH1's own skeleton pass's result for his feet (`pose::take_feet`), filled by `frame`.
    pub feet: super::pose::FeetSeen,
    pub events: String,
    /// The categories this row was made with (`cat`), filled by `frame`.
    pub cats: u32,
}

/// The columns, by category (`cat`; `0` = always there), in the CSV's order.
const COLUMNS: [(u32, &str); 12] = [
    (0, "frame,ticks,steps,ms,owner,shared,world,room,event"),
    (cat::CONFIG, "cfg_camera,cfg_physics,physics_now,cfg_ground_combo,cfg_air_combo,cfg_magic,cfg_air_magic,cfg_jump,cfg_glide,\
cfg_dodge_roll,cfg_misc_anims,cfg_player_model,cfg_battle_quotes,cfg_cure,cfg_hit_reactions,cfg_recoil,\
cfg_fire,cfg_blizzard,cfg_thunder,cfg_anim_blizzard,cfg_anim_thunder,cfg_anim_cure,cfg_anim_gravity,cfg_anim_stop,cfg_anim_aero"),
    (cat::MOVEMENT, "pos_x,pos_up,pos_z,moved_x,moved_up,moved_z,asked_x,asked_up,asked_z,\
above_floor,floor_up,collision,stand_on,platform,yaw,\
kh1_vel_x,kh1_vel_y,kh1_vel_z,kh1_vel_w,kh1_arc_0,kh1_arc_1,kh1_arc_2,kh1_arc_3,push_x,push_y,push_z"),
    (cat::KH1_STATE, "kh1_state,kh1_motion,actor_flags,combo_window"),
    (cat::KH2_STATE, "kh2_state,kh2_motion,kh2_frame,kh2_air,kh2_grounded,kh2_on_top,kh2_facing,kh2_combo,kh2_lock,kh2_invulnerable,kh2_guarding"),
    (cat::FORM, "form"),
    (cat::INPUT, "buttons,kh1_buttons,stick_rx,stick_ry,stick_lx,stick_ly"),
    (cat::DRAW, "draw_route,draw_slot,draw_play,draw_kh2_motion,draw_clip_frame,draw_blend,draw_restart,draw_age_ms"),
    (cat::KH1_CAMERA, "cam_who,cam_via,cam_why,cam_lag,cam_mode_now,cam_blend_now,cam_entry,\
cam_eye_x,cam_eye_up,cam_eye_z,cam_at_x,cam_at_up,cam_at_z,cam_eye_step,cam_at_step,\
cam_horiz,cam_eye_height,cam_look_height,cam_yaw,cam_pitch,cam_dist,cam_fov,cam_blend_left,cam_blend_time"),
    (cat::KH2_CAMERA, "kh2cam_yaw,kh2cam_goal_yaw,kh2cam_dist,kh2cam_look_x,kh2cam_look_y,kh2cam_move_x,kh2cam_move_y,kh2cam_reset,kh2cam_auto,kh2cam_hold,kh2cam_mouse_x,kh2cam_mouse_y,kh2cam_kbm,kh2cam_lock_dist"),
    (cat::FEET, "ik_ground_on,ik_lifted,ik_flags,foot_r_up,foot_l_up,pose_path"),
    (cat::EVENTS, "events"),
];

/// The CSV's header for the categories in `mask`.
fn header(mask: u32) -> String {
    COLUMNS.iter().filter(|(c, _)| *c == 0 || mask & c != 0).map(|(_, h)| *h).collect::<Vec<_>>().join(",")
}

/// Columns in `Row::config` (as in `HEADER`): Camera Style, Physics, the physics in use now, then
/// the move styles in `styles::Kind::EVERY`'s order.
const CONFIG: usize = 3 + super::styles::Kind::EVERY.len();

/// The Config menu's KH1 / KH2 rows now (bit set: KH2's), `CONFIG`'s order.
fn config_now() -> u32 {
    use super::shared::physics::{self, Rule};
    let mut bits = [super::camera::KH2.load(Ordering::Relaxed), physics::rule() == Rule::Kh2, physics::source() == Rule::Kh2].to_vec();
    bits.extend(super::styles::Kind::EVERY.iter().map(|k| super::styles::kh2(*k)));
    bits.iter().enumerate().fold(0, |m, (i, b)| m | (*b as u32) << i)
}

/// Set from the ini on the first frame (`note` can't read the settings: their first read logs).
static ENABLED: AtomicBool = AtomicBool::new(false);
static RING: Mutex<VecDeque<Row>> = Mutex::new(VecDeque::new());
/// Log lines written since the last row.
static EVENTS: Mutex<String> = Mutex::new(String::new());
/// KH2's asked move this frame (x, up, z), added over its steps.
static ASKED: Mutex<Option<[f32; 3]>> = Mutex::new(None);
static COMBO_PREV: AtomicBool = AtomicBool::new(false);
static UNDER: AtomicU32 = AtomicU32::new(0);
/// An anomaly's dump to come: frames left (0 = none) and its reason.
static PENDING: Mutex<Option<(u32, String)>> = Mutex::new(None);
static DUMPS: AtomicU32 = AtomicU32::new(0);
/// Each anomaly's last dump (ms): jump, under, flip, nan.
static LAST_AUTO: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
/// The stream file (`trace = always`) and the categories its header was written with.
static STREAM: Mutex<Option<(std::fs::File, u32)>> = Mutex::new(None);

/// A log line, kept for this frame's row (when log lines are recorded).
pub fn note(line: &str) {
    note_as(cat::LOG, line);
}

/// A line of category `c` (`cat::LOG`, `HIT_FX` or `SOUNDS`), kept for this frame's row.
pub fn note_as(c: u32, line: &str) {
    if !ENABLED.load(Ordering::Relaxed) || LIVE_CATS.load(Ordering::Relaxed) & c == 0 {
        return;
    }
    if let Ok(mut e) = EVENTS.try_lock() {
        if e.len() < 2000 {
            if !e.is_empty() {
                e.push_str(" | ");
            }
            e.push_str(line);
        }
    }
}

/// KH2's move of one 60 Hz step as it goes to KH1's mover: its displacement (up) and the carry
/// (KH1's y-down).
pub fn asked(displacement: [f32; 3], carry: [f32; 3]) {
    if !ENABLED.load(Ordering::Relaxed) || !wants(cat::MOVEMENT) {
        return;
    }
    if let Ok(mut a) = ASKED.lock() {
        let s = a.get_or_insert([0.0; 3]);
        s[0] += displacement[0] + carry[0];
        s[1] += displacement[1] - carry[1];
        s[2] += displacement[2] + carry[2];
    }
}

/// This frame's asked move (cleared).
pub fn take_asked() -> Option<[f32; 3]> {
    ASKED.lock().ok().and_then(|mut a| a.take())
}

/// The trace has run this session (no settings read: for the hooks, `pose::note_feet`).
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// The trace is on (the ini, or the debug menu).
pub fn on() -> bool {
    refresh();
    let on = mode() != Mode::Off;
    ENABLED.store(on, Ordering::Relaxed);
    on
}

/// Triangle held: KH1 doesn't see Select (its first-person view), so Select + Triangle dumps the
/// trace without the camera changing.
pub fn hides_select(buttons: u16) -> bool {
    ENABLED.load(Ordering::Relaxed) && buttons & RAW_TRIANGLE != 0
}

/// Sora's frame is over: its row kept, anomalies looked for, Select + Triangle answered.
pub fn frame(mut row: Row) {
    let mode = mode();
    if mode == Mode::Off {
        return;
    }
    ENABLED.store(true, Ordering::Relaxed);
    row.wall_ms = super::frameadv::wall_ms();
    let cats = cats();
    row.cats = cats;
    if cats & cat::CONFIG != 0 {
        row.config = config_now();
    }
    if cats & cat::DRAW != 0 {
        row.draw = super::pose::drawn();
    }
    if cats & (cat::KH1_CAMERA | cat::KH2_CAMERA) != 0 {
        row.cam = super::camera::seen();
    }
    // (Taken every frame so the counts don't pile up while the category is off.)
    let feet = super::pose::take_feet();
    if cats & cat::FEET != 0 {
        row.feet = feet;
    }
    if let Ok(mut e) = EVENTS.lock() {
        row.events = std::mem::take(&mut *e);
    }
    let Ok(mut ring) = RING.lock() else { return };
    let anomaly = anomaly(&ring, &row);
    if mode == Mode::Always {
        stream(&row);
    }
    let now = row.ms;
    let select = row.buttons & RAW_SELECT != 0 || row.kh1_buttons & KH1_SELECT != 0;
    let combo = select && row.buttons & RAW_TRIANGLE != 0;
    // (t371) The clip's window in game frames or in real time (`game_frames`).
    let game = game_frames();
    let at = |r: &Row| if game { r.ticks * 1000 / 60 } else { r.wall_ms };
    let newest = at(&row);
    ring.push_back(row);
    let (keep, max_rows) = (keep_ms(), seconds() as usize * ROWS_PER_SECOND);
    while ring.len() > max_rows || ring.front().is_some_and(|r| newest.saturating_sub(at(r)) > keep) {
        ring.pop_front();
    }
    if combo && !COMBO_PREV.swap(true, Ordering::Relaxed) {
        // (The pause menu's click when the file is written, its "can't" when it isn't.)
        click(dump(&ring, "select_triangle"));
    } else if !combo {
        COMBO_PREV.store(false, Ordering::Relaxed);
    }
    if let Some((i, why)) = anomaly {
        let last = &LAST_AUTO[i];
        let free = last.load(Ordering::Relaxed) == 0 || now.saturating_sub(last.load(Ordering::Relaxed)) > AUTO_COOLDOWN_MS;
        if let Ok(mut p) = PENDING.lock() {
            if free && p.is_none() {
                last.store(now.max(1), Ordering::Relaxed);
                log(&format!("trace: {why}; the last {}s go to a file in {} frames", keep / 1000, AUTO_DELAY));
                *p = Some((AUTO_DELAY, why));
            }
        }
    }
    let due = match PENDING.lock() {
        Ok(mut p) => match p.as_mut() {
            Some((left, _)) if *left > 1 => {
                *left -= 1;
                None
            }
            Some(_) => p.take().map(|(_, why)| why),
            None => None,
        },
        Err(_) => None,
    };
    if let Some(why) = due {
        let tag: String = why.split(':').next().unwrap_or("auto").chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        dump(&ring, &tag);
    }
}

/// (t371) Select + Triangle while Frame Advance holds the game (`frameadv`): no row is made then
/// (rows come with Sora's physics frames), so the dump is asked for from here, once a frame, with
/// the pad as pressed. The same press isn't taken twice (`COMBO_PREV`, shared with `frame`).
pub fn frozen_combo(buttons: u16) {
    if mode() == Mode::Off {
        return;
    }
    let combo = buttons & RAW_SELECT != 0 && buttons & RAW_TRIANGLE != 0;
    if !combo {
        COMBO_PREV.store(false, Ordering::Relaxed);
        return;
    }
    if !COMBO_PREV.swap(true, Ordering::Relaxed) {
        if let Ok(ring) = RING.lock() {
            click(dump(&ring, "select_triangle"));
        }
    }
}

/// Something in this row that looks wrong next to the last ones: (which, what).
fn anomaly(ring: &VecDeque<Row>, row: &Row) -> Option<(usize, String)> {
    let finite = row.pos.iter().chain(row.moved.iter()).chain(row.velocity.iter()).all(|v| v.is_finite());
    if !finite {
        return Some((3, format!("nan: a NaN in his position or velocity (pos {:.1?}, velocity {:.1?})", row.pos, row.velocity)));
    }
    let prev = ring.back()?;
    let same_place = prev.world == row.world && prev.room == row.room && !row.event && !prev.event;
    let steady = row.ms.saturating_sub(prev.ms) <= GAP_MS && row.frame == prev.frame + 1;
    let dist = (row.moved[0].powi(2) + row.moved[1].powi(2) + row.moved[2].powi(2)).sqrt();
    if same_place && steady && dist > JUMP_PER_STEP * row.steps.max(1) as f32 {
        return Some((0, format!("jump: he moved {dist:.0} in one frame ({} steps; {} has him; moved {:.1?}, asked {:.1?})", row.steps, row.owner, row.moved, row.asked)));
    }
    if row.above < -UNDER_BY && !row.event {
        let n = UNDER.fetch_add(1, Ordering::Relaxed) + 1;
        if n == UNDER_FRAMES {
            return Some((1, format!("under: {:.0} under the floor for {n} frames ({} has him, pos {:.1?})", -row.above, row.owner, row.pos)));
        }
    } else {
        UNDER.store(0, Ordering::Relaxed);
    }
    let mut flips = 0;
    let mut last = row.owner;
    for r in ring.iter().rev().take(FLIP_WINDOW) {
        if r.owner != last {
            flips += 1;
            last = r.owner;
        }
    }
    if flips >= FLIPS {
        return Some((2, format!("flip: KH1 and KH2 traded him {flips} times in {FLIP_WINDOW} frames")));
    }
    None
}

/// KH1's menu sounds (`0x140177F60 (n)`: 1 the cursor's move, 2 the confirm, 4 the "can't").
const SOUND_RVA: super::addr::Rva = super::addr::Rva(0x177F60);

/// The pause menu's confirm sound (a trace recorded) or its "can't" (none could be).
fn click(written: bool) {
    unsafe {
        let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
        let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
        sound(if written { 2 } else { 4 });
    }
}

/// Writes the ring to a CSV; whether it was written.
fn dump(ring: &VecDeque<Row>, why: &str) -> bool {
    let n = DUMPS.fetch_add(1, Ordering::Relaxed);
    if n >= MAX_DUMPS {
        if n == MAX_DUMPS {
            log(&format!("trace: {MAX_DUMPS} dumps this session; no more until the game restarts"));
        }
        return false;
    }
    let dir = format!("{}\\kh1-kh2-traces", super::exe_dir());
    let _ = std::fs::create_dir_all(&dir);
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let last = ring.back().map_or(0, |r| r.frame);
    let path = format!("{dir}\\trace_{secs}_f{last}_{why}.csv");
    let mask = cats();
    let head = header(mask);
    let mut text = String::with_capacity(ring.len() * 400 + head.len() + 64);
    text.push_str(&head);
    text.push_str("\r\n");
    for r in ring.iter() {
        line(&mut text, r, mask);
    }
    match std::fs::write(&path, text) {
        Ok(()) => {
            log(&format!("trace: {} frames ({:.1}s) written to {path} ({why})", ring.len(), ring.back().zip(ring.front()).map_or(0.0, |(b, f)| b.ms.saturating_sub(f.ms) as f32 / 1000.0)));
            true
        }
        Err(e) => {
            log(&format!("trace: couldn't write {path}: {e}"));
            false
        }
    }
}

fn stream(row: &Row) {
    let Ok(mut s) = STREAM.lock() else { return };
    // (The categories changed: the file starts again with the new header.)
    if s.as_ref().is_some_and(|(_, m)| *m != row.cats) {
        *s = None;
    }
    if s.is_none() {
        let path = format!("{}\\kh1-kh2.trace.csv", super::exe_dir());
        match std::fs::File::create(&path) {
            Ok(mut f) => {
                let _ = write!(f, "{}\r\n", header(row.cats));
                log(&format!("trace: every frame also goes to {path} (trace = always)"));
                *s = Some((f, row.cats));
            }
            Err(e) => {
                log(&format!("trace: couldn't open {path}: {e}"));
                return;
            }
        }
    }
    if let Some((f, mask)) = s.as_mut() {
        let mut text = String::with_capacity(512);
        line(&mut text, row, *mask);
        let _ = f.write_all(text.as_bytes());
    }
}

/// A row as a CSV line, with the columns of the categories in `mask` (empty where the row was
/// made without one of them).
fn line(t: &mut String, r: &Row, mask: u32) {
    let b = |v: bool| if v { 1 } else { 0 };
    // Whether a category's columns are written, and whether this row has them.
    let col = |c: u32| mask & c != 0;
    let has = |c: u32| r.cats & c != 0;
    let _ = write!(t, "{},{},{},{},{},{},{:#x},{:#x},{},", r.frame, r.ticks, r.steps, r.ms, r.owner, b(r.shared), r.world, r.room, b(r.event));
    if col(cat::CONFIG) {
        for i in 0..CONFIG {
            t.push_str(if !has(cat::CONFIG) { "," } else if r.config & (1 << i) != 0 { "KH2," } else { "KH1," });
        }
    }
    if col(cat::MOVEMENT) {
        if has(cat::MOVEMENT) {
            let _ = write!(t, "{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},", r.pos[0], r.pos[1], r.pos[2], r.moved[0], r.moved[1], r.moved[2]);
            match r.asked {
                Some(a) => {
                    let _ = write!(t, "{:.2},{:.2},{:.2},", a[0], a[1], a[2]);
                }
                None => t.push_str(",,,"),
            }
            let _ = write!(t, "{:.2},{:.2},{:#x},{:#x},{},{:.3},", r.above, r.floor_up, r.collision, r.stand_on, b(r.platform), r.yaw);
            for v in r.velocity.iter().chain(r.arc.iter()).chain(r.push.iter()) {
                let _ = write!(t, "{v:.3},");
            }
        } else {
            t.push_str(&",".repeat(26));
        }
    }
    if col(cat::KH1_STATE) {
        if has(cat::KH1_STATE) {
            let _ = write!(t, "{},{:#x},{:#010x},{:#x},", r.kh1_state, r.kh1_motion, r.flags, r.combo_window);
        } else {
            t.push_str(",,,,");
        }
    }
    if col(cat::KH2_STATE) {
        match r.kh2.as_ref().filter(|_| has(cat::KH2_STATE)) {
            Some(k) => {
                let _ = write!(t, "{},{},{},{},{},", k.state, k.motion, k.frame, b(k.air), b(k.grounded));
                match k.on_top {
                    Some(v) => {
                        let _ = write!(t, "{v:.1},");
                    }
                    None => t.push(','),
                }
                let _ = write!(t, "{:.3},{},", k.facing, k.combo);
                match k.lock {
                    Some(v) => {
                        let _ = write!(t, "{v:#x},");
                    }
                    None => t.push(','),
                }
                let _ = write!(t, "{},{},", b(k.invulnerable), b(k.guarding));
            }
            None => t.push_str(",,,,,,,,,,,"),
        }
    }
    if col(cat::FORM) {
        let _ = write!(t, "{},", r.form.as_deref().filter(|_| has(cat::FORM)).unwrap_or(""));
    }
    if col(cat::INPUT) {
        if has(cat::INPUT) {
            let _ = write!(t, "{:#06x},{:#010x},{},{},{},{},", r.buttons, r.kh1_buttons, r.sticks[0], r.sticks[1], r.sticks[2], r.sticks[3]);
        } else {
            t.push_str(",,,,,,");
        }
    }
    if col(cat::DRAW) {
        let d = &r.draw;
        if !has(cat::DRAW) {
            t.push_str(",,,,,,,,");
        } else {
            let age = d.at.map_or(String::new(), |_| d.age_ms.to_string());
            let slot = d.slot.map_or(String::new(), |n| n.to_string());
            if d.route == "follow" || d.route == "baked" {
                let _ = write!(t, "{},{},\"{}\",{},{:.1},{:.2},{},{},", d.route, slot, d.play, d.motion, d.frames, d.blend, b(d.restart), age);
            } else {
                let _ = write!(t, "{},{},,,,,,{},", d.route, slot, age);
            }
        }
    }
    if col(cat::KH1_CAMERA) {
        camera(t, r, has(cat::KH1_CAMERA));
    }
    if col(cat::KH2_CAMERA) {
        kh2_camera(t, r, has(cat::KH2_CAMERA));
    }
    if col(cat::FEET) {
        feet(t, r, has(cat::FEET));
    }
    if col(cat::EVENTS) {
        if !r.events.is_empty() {
            t.push('"');
            t.push_str(&r.events.replace('"', "\"\"").replace(['\r', '\n'], " "));
            t.push('"');
        }
    } else if t.ends_with(',') {
        // (The last column written had its comma: no events column after it.)
        t.pop();
    }
    t.push_str("\r\n");
}

/// KH1's own result for his feet (`pose::FeetSeen`): `ik_ground_on` (`instance + 0x14` bits 0-1,
/// KH1's ground lift turned on), `ik_lifted` (bit `0x200000`, a foot target lifted onto the floor
/// that pass), `ik_flags` (the whole word), each ankle's height above `floor_up` (the floor under
/// his centre; on a slope, planted feet read uneven), and which of KH1's skeleton passes ran since
/// the last row (`pass` 0x140391E20, `ik` 0x140392710, with counts). Empty when KH1 didn't pose
/// him since the last row.
fn feet(t: &mut String, r: &Row, has: bool) {
    let f = &r.feet;
    if !f.seen || !has {
        t.push_str(",,,,,,");
        return;
    }
    let path = match f.passes {
        [0, 0] => String::new(),
        [p, 0] => format!("pass{}", if p > 1 { format!(" x{p}") } else { String::new() }),
        [0, k] => format!("ik{}", if k > 1 { format!(" x{k}") } else { String::new() }),
        [p, k] => format!("pass x{p} + ik x{k}"),
    };
    let _ = write!(
        t,
        "{},{},{:#010x},{:.2},{:.2},{},",
        f.flags & 3,
        (f.flags & 0x20_0000 != 0) as u8,
        f.flags,
        f.ankle_up[0] - r.floor_up,
        f.ankle_up[1] - r.floor_up,
        path
    );
}

/// The camera's columns (the `cam_*` ones are KH1's camera as the last follow / lock-on call left
/// it, KH1's units with `up` = minus its y; `kh2cam_*` is KH2's camera's own numbers when it
/// placed it, with the distance in KH2's units, KH1's divided by `camera_scale`).
/// `cam_lag` is how many game frames before this row's the camera last ran (Sora's physics
/// runs before the camera in a frame, so 1 is normal; more: KH1 has the camera in a mode that is
/// neither follow nor lock-on, `cam_mode_now`, such as an event, an area camera or first person).
/// `cam_horiz` / `cam_eye_height` / `cam_look_height`: the eye's distance along the ground from
/// Sora's feet, and the eye's and look point's height above them, at the camera's own frame.
fn camera(t: &mut String, r: &Row, has: bool) {
    let c = &r.cam;
    if c.who.is_empty() || !has {
        t.push_str(&",".repeat(24));
        return;
    }
    let lag = r.frame as i64 - c.frame as i64;
    let _ = write!(t, "{},{},{},{},{},{:.1},{:#x},", c.who, c.via, c.why, lag, c.mode_now, c.blend_now, c.entry);
    let _ = write!(t, "{:.1},{:.1},{:.1},{:.1},{:.1},{:.1},{:.2},{:.2},", c.eye[0], c.eye[1], c.eye[2], c.at[0], c.at[1], c.at[2], c.eye_step, c.at_step);
    let horiz = ((c.eye[0] - c.feet[0]).powi(2) + (c.eye[2] - c.feet[2]).powi(2)).sqrt();
    let _ = write!(t, "{:.1},{:.1},{:.1},", horiz, c.eye[1] - c.feet[1], c.at[1] - c.feet[1]);
    let _ = write!(t, "{:.3},{:.3},{:.1},{:.1},{:.1},{:.1},", c.yaw, c.pitch, c.dist, c.fov, c.blend_left, c.blend_time);
}

/// KH2's camera's own numbers when it placed the camera (`kh2cam_*`).
fn kh2_camera(t: &mut String, r: &Row, has: bool) {
    let c = &r.cam;
    match c.own.as_ref().filter(|_| has && !c.who.is_empty()) {
        Some(o) => {
            let _ = write!(t, "{:.3},{:.3},{:.1},{:.2},{:.2},{:.2},{:.2},{},{},", o.yaw, o.goal_yaw, o.dist, o.look[0], o.look[1], o.mv[0], o.mv[1], o.reset as u8, o.auto as u8);
            let _ = write!(t, "{},", o.hold as u8);
            match o.mouse {
                Some(m) => {
                    let _ = write!(t, "{:.3},{:.3},", m[0], m[1]);
                }
                None => t.push_str(",,"),
            }
            let _ = write!(t, "{},", o.kbm as u8);
            match o.lock_dist {
                Some(d) => {
                    let _ = write!(t, "{d:.1},");
                }
                None => t.push(','),
            }
        }
        None => t.push_str(",,,,,,,,,,,,,,"),
    }
}
