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
//! Positions are KH2's way up (`up` = -KH1's y). `moved` is where he ended the frame less where
//! he started it; `asked` is KH2's displacement (and a platform's carry) over the frame's 60 Hz
//! steps, empty on KH1's frames: the two differ where KH1's mover (walls, slopes, floors) had its say.

use super::log;
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::io::Write as _;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;

/// How much is kept for a dump when the ini doesn't say (`trace_seconds`).
pub const DEFAULT_SECONDS: u32 = 10;
/// The longest clip the ini can ask for.
pub const MAX_SECONDS: u32 = 120;
/// At most this many rows a second kept (120 game frames a second).
const ROWS_PER_SECOND: usize = 120;

/// How much is kept for a dump, in ms (`trace_seconds` in the ini).
fn keep_ms() -> u64 {
    super::settings::get().trace_seconds as u64 * 1000
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
    /// "KH2", "KH1" or "handoff" (`takeover::FRAME_KIND`).
    pub owner: &'static str,
    /// KH1's own states with KH2's moves sharing them (`shared::active`).
    pub shared: bool,
    pub world: u32,
    pub room: u32,
    pub event: bool,
    /// The Config menu's rows, bit per `CONFIG` column (set: KH2), filled by `frame`.
    pub config: u16,
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
    pub events: String,
}

const HEADER: &str = "frame,ticks,steps,ms,owner,shared,world,room,event,\
cfg_camera,cfg_physics,physics_now,cfg_ground_combo,cfg_air_combo,cfg_magic,cfg_air_magic,cfg_jump,cfg_glide,\
cfg_dodge_roll,cfg_misc_anims,cfg_player_model,cfg_battle_quotes,\
pos_x,pos_up,pos_z,moved_x,moved_up,moved_z,asked_x,asked_up,asked_z,\
above_floor,floor_up,collision,stand_on,platform,yaw,\
kh1_state,kh1_motion,actor_flags,combo_window,\
kh1_vel_x,kh1_vel_y,kh1_vel_z,kh1_vel_w,kh1_arc_0,kh1_arc_1,kh1_arc_2,kh1_arc_3,push_x,push_y,push_z,\
kh2_state,kh2_motion,kh2_frame,kh2_air,kh2_grounded,kh2_on_top,kh2_facing,kh2_combo,kh2_lock,kh2_invulnerable,kh2_guarding,form,\
buttons,kh1_buttons,stick_rx,stick_ry,stick_lx,stick_ly,\
draw_route,draw_slot,draw_play,draw_kh2_motion,draw_clip_frame,draw_blend,draw_restart,draw_age_ms,\
cam_who,cam_via,cam_why,cam_lag,cam_mode_now,cam_blend_now,cam_entry,\
cam_eye_x,cam_eye_up,cam_eye_z,cam_at_x,cam_at_up,cam_at_z,cam_eye_step,cam_at_step,\
cam_horiz,cam_eye_height,cam_look_height,cam_yaw,cam_pitch,cam_dist,cam_fov,cam_blend_left,cam_blend_time,\
kh2cam_yaw,kh2cam_goal_yaw,kh2cam_dist,kh2cam_look_x,kh2cam_look_y,kh2cam_move_x,kh2cam_move_y,kh2cam_reset,kh2cam_auto,kh2cam_lock_dist,events";

/// Columns in `Row::config` (as in `HEADER`): Camera Style, Physics, the physics in use now, then
/// the move styles in `styles::Kind::EVERY`'s order.
const CONFIG: usize = 3 + super::styles::Kind::EVERY.len();

/// The Config menu's KH1 / KH2 rows now (bit set: KH2's), `CONFIG`'s order.
fn config_now() -> u16 {
    use super::shared::physics::{self, Rule};
    let mut bits = [super::camera::KH2.load(Ordering::Relaxed), physics::rule() == Rule::Kh2, physics::source() == Rule::Kh2].to_vec();
    bits.extend(super::styles::Kind::EVERY.iter().map(|k| super::styles::kh2(*k)));
    bits.iter().enumerate().fold(0, |m, (i, b)| m | (*b as u16) << i)
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
static STREAM: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// A log line, kept for this frame's row.
pub fn note(line: &str) {
    if !ENABLED.load(Ordering::Relaxed) {
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
    if !ENABLED.load(Ordering::Relaxed) {
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

/// The trace is on (the ini).
pub fn on() -> bool {
    let on = super::settings::get().trace != Mode::Off;
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
    let mode = super::settings::get().trace;
    if mode == Mode::Off {
        return;
    }
    ENABLED.store(true, Ordering::Relaxed);
    row.config = config_now();
    row.draw = super::pose::drawn();
    row.cam = super::camera::seen();
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
    ring.push_back(row);
    let (keep, max_rows) = (keep_ms(), super::settings::get().trace_seconds as usize * ROWS_PER_SECOND);
    while ring.len() > max_rows || ring.front().is_some_and(|r| now.saturating_sub(r.ms) > keep) {
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
    let mut text = String::with_capacity(ring.len() * 400 + HEADER.len() + 64);
    text.push_str(HEADER);
    text.push_str("\r\n");
    for r in ring.iter() {
        line(&mut text, r);
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
    if s.is_none() {
        let path = format!("{}\\kh1-kh2.trace.csv", super::exe_dir());
        match std::fs::File::create(&path) {
            Ok(mut f) => {
                let _ = write!(f, "{HEADER}\r\n");
                log(&format!("trace: every frame also goes to {path} (trace = always)"));
                *s = Some(f);
            }
            Err(e) => {
                log(&format!("trace: couldn't open {path}: {e}"));
                return;
            }
        }
    }
    if let Some(f) = s.as_mut() {
        let mut text = String::with_capacity(512);
        line(&mut text, row);
        let _ = f.write_all(text.as_bytes());
    }
}

/// A row as a CSV line.
fn line(t: &mut String, r: &Row) {
    let b = |v: bool| if v { 1 } else { 0 };
    let _ = write!(t, "{},{},{},{},{},{},{:#x},{:#x},{},", r.frame, r.ticks, r.steps, r.ms, r.owner, b(r.shared), r.world, r.room, b(r.event));
    for i in 0..CONFIG {
        t.push_str(if r.config & (1 << i) != 0 { "KH2," } else { "KH1," });
    }
    let _ = write!(t, "{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},", r.pos[0], r.pos[1], r.pos[2], r.moved[0], r.moved[1], r.moved[2]);
    match r.asked {
        Some(a) => {
            let _ = write!(t, "{:.2},{:.2},{:.2},", a[0], a[1], a[2]);
        }
        None => t.push_str(",,,"),
    }
    let _ = write!(t, "{:.2},{:.2},{:#x},{:#x},{},{:.3},", r.above, r.floor_up, r.collision, r.stand_on, b(r.platform), r.yaw);
    let _ = write!(t, "{},{:#x},{:#010x},{:#x},", r.kh1_state, r.kh1_motion, r.flags, r.combo_window);
    for v in r.velocity.iter().chain(r.arc.iter()).chain(r.push.iter()) {
        let _ = write!(t, "{v:.3},");
    }
    match &r.kh2 {
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
    let _ = write!(t, "{},", r.form.as_deref().unwrap_or(""));
    let _ = write!(t, "{:#06x},{:#010x},{},{},{},{},", r.buttons, r.kh1_buttons, r.sticks[0], r.sticks[1], r.sticks[2], r.sticks[3]);
    {
        let d = &r.draw;
        let age = d.at.map_or(String::new(), |_| d.age_ms.to_string());
        let slot = d.slot.map_or(String::new(), |n| n.to_string());
        if d.route == "follow" {
            let _ = write!(t, "{},{},\"{}\",{},{:.1},{:.2},{},{},", d.route, slot, d.play, d.motion, d.frames, d.blend, b(d.restart), age);
        } else {
            let _ = write!(t, "{},{},,,,,,{},", d.route, slot, age);
        }
    }
    camera(t, r);
    if !r.events.is_empty() {
        t.push('"');
        t.push_str(&r.events.replace('"', "\"\"").replace(['\r', '\n'], " "));
        t.push('"');
    }
    t.push_str("\r\n");
}

/// The camera's columns (the `cam_*` ones are KH1's camera as the last follow / lock-on call left
/// it, KH1's units with `up` = minus its y; `kh2cam_*` is KH2's camera's own numbers when it
/// placed it, with the distance in KH2's units, KH1's divided by `camera_scale`).
/// `cam_lag` is how many game frames before this row's the camera last ran (Sora's physics
/// runs before the camera in a frame, so 1 is normal; more: KH1 has the camera in a mode that is
/// neither follow nor lock-on, `cam_mode_now`, such as an event, an area camera or first person).
/// `cam_horiz` / `cam_eye_height` / `cam_look_height`: the eye's distance along the ground from
/// Sora's feet, and the eye's and look point's height above them, at the camera's own frame.
fn camera(t: &mut String, r: &Row) {
    let c = &r.cam;
    if c.who.is_empty() {
        t.push_str(&",".repeat(34));
        return;
    }
    let lag = r.frame as i64 - c.frame as i64;
    let _ = write!(t, "{},{},{},{},{},{:.1},{:#x},", c.who, c.via, c.why, lag, c.mode_now, c.blend_now, c.entry);
    let _ = write!(t, "{:.1},{:.1},{:.1},{:.1},{:.1},{:.1},{:.2},{:.2},", c.eye[0], c.eye[1], c.eye[2], c.at[0], c.at[1], c.at[2], c.eye_step, c.at_step);
    let horiz = ((c.eye[0] - c.feet[0]).powi(2) + (c.eye[2] - c.feet[2]).powi(2)).sqrt();
    let _ = write!(t, "{:.1},{:.1},{:.1},", horiz, c.eye[1] - c.feet[1], c.at[1] - c.feet[1]);
    let _ = write!(t, "{:.3},{:.3},{:.1},{:.1},{:.1},{:.1},", c.yaw, c.pitch, c.dist, c.fov, c.blend_left, c.blend_time);
    match &c.own {
        Some(o) => {
            let _ = write!(t, "{:.3},{:.3},{:.1},{:.2},{:.2},{:.2},{:.2},{},{},", o.yaw, o.goal_yaw, o.dist, o.look[0], o.look[1], o.mv[0], o.mv[1], o.reset as u8, o.auto as u8);
            match o.lock_dist {
                Some(d) => {
                    let _ = write!(t, "{d:.1},");
                }
                None => t.push(','),
            }
        }
        None => t.push_str(",,,,,,,,,,"),
    }
}
