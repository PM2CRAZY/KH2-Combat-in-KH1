//! The Frame Advance info panel (Mr Bugs, "Missing sleeves animation" m19: "in the debug menu
//! when you frame advance it tells you the animation state your in followed by the animationcount
//! ... a separate window pops up with the info", (a) "drawn on the game screen in a corner", (a)
//! "the frame within the animation"). While Frame Advance is on, a small box in the screen's top
//! right corner, apart from the debug menu's box (drawn whether the menu is open or not, in the
//! same overlay pass, `debugmenu::draw_end`). (t763) Also drawn without Frame Advance when the
//! debug menu's Anim Display row is on (`debugmenu::anim_display`), titled "ANIMATION":
//!
//! - the animation drawn: KH2's motion (battle or field) on KH1's slot, or KH1's own;
//! - how it is drawn: baked into KH1's motion (KH1 plays it), live (our pose over KH1's), KH2's
//!   controller's pose, or KH1's own animation;
//! - the frame within it, of its length ("Animation: field run, frame 12 / 40"), and how many times it has looped
//!   since it began;
//! - the trace's frame number (the trace's `frame` column, `takeover::GAME_FRAME`), so a stretch can
//!   be named as "trace frames X to Y".
//!
//! The frame within the animation is the clip's own time (KH1's motion time for our slots, which
//! KH1 plays one frame a 60 Hz frame; our clock for KH2's controller's), floored; a loop is counted
//! when it comes round (Assumption: KH1 wraps a looping motion's time at its length; a clip time
//! past the length is also taken round).

use std::sync::Mutex;

/// The animation drawn, as the panel shows it.
#[derive(Clone, Debug, Default)]
pub struct Anim {
    /// "baked", "live", "kh2_controller" or "kh1_own".
    pub route: &'static str,
    /// KH2's motion (None: KH1's own animation or KH2's controller's pose).
    pub motion: Option<u16>,
    pub field: bool,
    /// KH1's slot it stands for (ours reported as KH1's), and ours.
    pub kh1_slot: Option<usize>,
    pub slot: Option<usize>,
    /// The frame within the animation and its length (0: unknown), loops since it began.
    pub frame: u32,
    pub len: u32,
    pub loops: u32,
}

struct Clock {
    key: (&'static str, Option<usize>, u16),
    last: f32,
    loops: u32,
}
static CLOCK: Mutex<Option<Clock>> = Mutex::new(None);

/// A KH2 motion's name for the panel.
pub fn motion_name(m: u16) -> String {
    match m {
        0 => "idle".into(),
        1 => "walk".into(),
        2 => "run".into(),
        3 => "jump".into(),
        4 => "fall".into(),
        _ => format!("motion {m}"),
    }
}

/// The animation drawn last (`pose::drawn`), with its frame and loop count.
pub fn anim() -> Anim {
    let d = super::pose::drawn();
    let route = match d.route {
        "baked" => "baked",
        "follow" => "live",
        "kh2_controller" => "kh2_controller",
        _ => "kh1_own",
    };
    let field = d.slot.is_some_and(super::kh2slots::is_field) || d.play.contains("field: true");
    let motion = matches!(route, "baked" | "live").then_some(d.motion);
    let kh1_slot = d.slot.map(|n| super::kh2slots::stands_for(n).unwrap_or(n));
    let len = motion
        .and_then(|m| {
            let k = super::pose::kh2()?;
            let clip = if field { k.rig.field_motions.get(&m) } else { None }.or_else(|| k.rig.motions.get(&m))?;
            Some(kh2_anim::length_frames(clip).round().max(1.0) as u32)
        })
        .or_else(|| (route == "kh1_own").then(|| d.slot.and_then(super::pose::slot_frames)).flatten())
        .unwrap_or(0);
    let mut a = Anim { route, motion, field, kh1_slot, slot: d.slot, len, ..Default::default() };
    let t = if d.frames.is_finite() && d.frames >= 0.0 { d.frames } else { 0.0 };
    let key = (route, d.slot, d.motion);
    let Ok(mut c) = CLOCK.try_lock() else { return a };
    let restart = d.restart || c.as_ref().map_or(true, |c| c.key != key);
    let clock = c.get_or_insert(Clock { key, last: t, loops: 0 });
    if restart {
        *clock = Clock { key, last: t, loops: 0 };
    } else if len > 1 && t + len as f32 * 0.5 < clock.last {
        // (Came round.)
        clock.loops += 1;
    }
    clock.last = t;
    let (within, past) = if len > 0 { ((t as u32) % len, t as u32 / len) } else { (t as u32, 0) };
    a.frame = within;
    a.loops = clock.loops + past;
    a
}

/// The panel's lines.
pub fn lines() -> Vec<String> {
    let a = anim();
    let mut v = Vec::with_capacity(6);
    v.push(if super::frameadv::on() { "FRAME ADVANCE" } else { "ANIMATION" }.to_string());
    let what = match a.motion {
        Some(m) => format!("KH2 {} {}", if a.field { "field" } else { "battle" }, motion_name(m)),
        None if a.route == "kh2_controller" => "KH2 controller pose".to_string(),
        None => "KH1's own animation".to_string(),
    };
    v.push(match a.kh1_slot {
        Some(s) => format!("{what}  (KH1 slot {s}{})", a.slot.filter(|o| *o != s).map_or(String::new(), |o| format!(", ours {o}"))),
        None => what,
    });
    v.push(format!("Drawn: {}", match a.route {
        "baked" => "baked (KH1 plays it)",
        "live" => "live (our pose)",
        "kh2_controller" => "KH2 controller",
        _ => "KH1's own",
    }));
    let name = a.motion.map_or("anim".to_string(), |m| format!("{} {}", if a.field { "field" } else { "battle" }, motion_name(m)));
    v.push(if a.len > 0 { format!("Animation: {name}, frame {} / {}, loop {}", a.frame, a.len, a.loops) } else { format!("Animation: frame {}, loop {}", a.frame, a.loops) });
    v.push(format!("Trace frame: {}", super::takeover::GAME_FRAME.load(std::sync::atomic::Ordering::Relaxed)));
    v
}
