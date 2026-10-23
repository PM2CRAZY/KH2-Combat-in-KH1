//! Where our time goes on the game's thread (the user: "there is a slight jitter in the VFX, the
//! tiniest bit of lag"): each section's average and longest over 5 s, logged when one of them
//! took over 2 ms (40 lines at most).

use std::sync::Mutex;
use std::time::Instant;

pub const STEP: usize = 0;
pub const ANIM: usize = 1;
pub const STRIKES: usize = 2;
pub const EVENTS: usize = 3;
pub const FX_TICK: usize = 4;
pub const FORM: usize = 5;
pub const FRAME: usize = 6;
pub const POSE: usize = 7;
const NAMES: [&str; 8] = ["KH2 step", "animation", "hits", "sound/effect events", "effects step", "form", "Sora's whole frame", "pose (drawing)"];

struct Acc {
    since: Option<Instant>,
    sum: [f32; 8],
    max: [f32; 8],
    n: [u32; 8],
    logged: u32,
}

static ACC: Mutex<Acc> = Mutex::new(Acc { since: None, sum: [0.0; 8], max: [0.0; 8], n: [0; 8], logged: 0 });

/// `section` took since `t0`.
pub fn add(section: usize, t0: Instant) {
    let ms = t0.elapsed().as_secs_f32() * 1000.0;
    let Ok(mut a) = ACC.try_lock() else { return };
    a.sum[section] += ms;
    a.max[section] = a.max[section].max(ms);
    a.n[section] += 1;
    let since = *a.since.get_or_insert_with(Instant::now);
    if since.elapsed().as_secs_f32() >= 5.0 {
        if a.max.iter().any(|&m| m > 2.0) && a.logged < 40 {
            a.logged += 1;
            let parts: Vec<String> = (0..8).filter(|&i| a.n[i] > 0).map(|i| format!("{} avg {:.2} max {:.1}", NAMES[i], a.sum[i] / a.n[i] as f32, a.max[i])).collect();
            super::log(&format!("time (ms, last 5 s): {}", parts.join("; ")));
        }
        a.sum = [0.0; 8];
        a.max = [0.0; 8];
        a.n = [0; 8];
        a.since = Some(Instant::now());
    }
}
