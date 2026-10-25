//! The cloth physics live in the game (`kh1_data::clothsim`, the same code the offline runner
//! `kh1-data/examples/cloth_sim.rs` runs), on the one move being worked on (Mr Bugs, "Missing
//! sleeves animation": "physics only on the move, we'll tweak each animation until it looks
//! solid", "start with soras running animation without the keyblade"): KH2's field run, our slot
//! on KH1's field run (`kh2slots`, the first of the run's three). Every other move is drawn as
//! before. `docs/kh1-secondary-motion.md`, "Cloth physics (live, to tune before a bake)".
//!
//! - **Where.** `pose::check`, after KH1's skeleton pass, on Sora's pose in model space:
//!   - the slot baked (`kh1_bake`, the default): KH1 plays the bake itself and draws it; the coat,
//!     shorts, belt chain and belt-front bones (and what hangs under them) are read back from what
//!     KH1 drew (`+0x1B4`), moved by the physics, and written back; the rest stays KH1's drawing of
//!     the bake;
//!   - the slot drawn live (`kh1_bake` off): the physics runs on the pose we draw, before its
//!     write.
//! - **The rest** it pulls toward: KH1's own field run (slot 5, KH1 Sora's motion table),
//!   each part's relation to its parent averaged over its 36 frames.
//! - **Steps**: one per 60 Hz frame KH1 stepped since the last (`takeover::TICKS`), none while
//!   Frame Advance holds the game.
//! - **The numbers** (`settings`): KH1-tuned (`Settings::TUNED`, settings id 0), changed live on the
//!   debug menu's Cloth Physics page; every change is logged with its id.
//! - **Cost**: only on the run's frames; 19 chains of 1-4 links (79 bones), a few hundred matrix
//!   products and a handful of small allocations a frame (well under 0.1 ms).

use super::log;
use glam::Mat4;
use kh1_data::clothsim::{ClothSim, Params, Part, Settings};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

/// KH1's field run (slot 5): the rest.
pub const REST_SLOT: usize = 5;

/// The physics on the move (the debug menu's Physics row; on by default).
static ON: AtomicBool = AtomicBool::new(true);
static SETTINGS: Mutex<Settings> = Mutex::new(Settings::TUNED);

/// KH1's motion for the rest (set as KH1 Sora's motion table loads).
pub static REST: OnceLock<Option<kh1_data::mset::Motion>> = OnceLock::new();

struct Live {
    sim: ClothSim,
    /// `takeover::TICKS` at the last step.
    ticks: u64,
}
static LIVE: Mutex<Option<Live>> = Mutex::new(None);

/// The last frame the physics ran, for the trace and the panel.
#[derive(Clone, Copy, Debug, Default)]
pub struct Seen {
    pub active: bool,
    /// The game frame (`takeover::GAME_FRAME`) it ran on.
    pub frame: u64,
    pub baked: bool,
    pub report: kh1_data::clothsim::Report,
    pub settings: u32,
}
static SEEN: Mutex<Seen> = Mutex::new(Seen { active: false, frame: 0, baked: false, report: kh1_data::clothsim::Report { steps: 0, restarted: false, max_swing: 0.0, pushed: 0 }, settings: 0 });
/// Restarts so far (a warp, a long gap, the move begun again).
static RESTARTS: AtomicU64 = AtomicU64::new(0);

pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

pub fn set_on(v: bool) {
    if ON.swap(v, Ordering::Relaxed) != v {
        log(&format!("cloth physics: {} on KH2's field run", if v { "on" } else { "off" }));
    }
}

pub fn settings() -> Settings {
    SETTINGS.lock().map(|s| *s).unwrap_or_default()
}

/// The debug menu's change of one part's numbers (logged with the new id).
pub fn set_part(p: Part, v: Params) {
    let s = match SETTINGS.lock() {
        Ok(mut s) => {
            s.parts[p as usize] = v;
            *s
        }
        Err(_) => return,
    };
    log_settings(&s, &format!("{} changed", p.name()));
}

pub fn reset_settings() {
    if let Ok(mut s) = SETTINGS.lock() {
        *s = Settings::TUNED;
    }
    log_settings(&Settings::TUNED, "back to the tuned numbers");
}

fn log_settings(s: &Settings, why: &str) {
    let parts: Vec<String> = Part::ALL.iter().map(|p| {
        let v = s.get(*p);
        format!("{} stiffness {:.2} damping {:.2} weight {:.2} swing {:.0}", p.name(), v.stiffness, v.damping, v.weight, v.swing)
    }).collect();
    log(&format!("cloth physics settings id {} ({why}): {}", s.id(), parts.join("; ")));
}

/// What the physics did last (for the trace and the panel).
pub fn seen() -> Seen {
    SEEN.try_lock().map(|s| *s).unwrap_or_default()
}

pub fn restarts() -> u64 {
    RESTARTS.load(Ordering::Relaxed)
}

/// The move the physics runs on: our slot playing KH2's field run.
pub fn wants(slot: Option<usize>) -> bool {
    on() && slot.is_some_and(|n| super::kh2slots::kh2_motion(n) == Some(character_core::motion::RUN) && super::kh2slots::is_field(n))
}

/// A frame of the move: `pose` (KH1 model space, unscaled) moved by the physics. `baked`: the
/// pose was read back from KH1's own drawing of the bake (for the trace).
pub fn step(kh1: &[kh2_formats::anim::Bone], pose: &mut [Mat4], to_world: &Mat4, baked: bool) {
    let Ok(mut g) = LIVE.try_lock() else { return };
    let ticks = super::takeover::TICKS.load(Ordering::Relaxed);
    let live = g.get_or_insert_with(|| {
        let rest = REST.get().and_then(|m| m.as_ref());
        let sim = ClothSim::new(kh1, rest);
        log(&format!("cloth physics ready: {} chains, rest from KH1's slot {REST_SLOT} ({})", sim.chains(), if rest.is_some() { "read" } else { "missing: the bind" }));
        log_settings(&settings(), "start");
        Live { sim, ticks }
    });
    let set = settings();
    let steps = ticks.saturating_sub(live.ticks).min(u32::MAX as u64) as u32;
    live.ticks = ticks;
    // (Not on the run since the last frame of it, or the first: started over.)
    let was = seen();
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    if !was.active || frame.saturating_sub(was.frame) > 2 {
        live.sim.reset();
    }
    let report = live.sim.step(pose, to_world, steps, &set);
    if report.restarted {
        let n = RESTARTS.fetch_add(1, Ordering::Relaxed);
        if n < 40 {
            log(&format!("cloth physics: started (again) on KH2's field run ({})", if baked { "baked slot, KH1's drawing moved" } else { "drawn live" }));
        }
    }
    if let Ok(mut s) = SEEN.try_lock() {
        *s = Seen { active: true, frame, baked, report, settings: set.id() };
    }
}

/// A frame drawn without the physics.
pub fn idle() {
    if let Ok(mut s) = SEEN.try_lock() {
        s.active = false;
    }
}
