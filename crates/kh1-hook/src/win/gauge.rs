//! KH2's Drive gauge for Sora (the user: "lets incorporate the drive gauge"). The rules are
//! KH2's own (`character_core::drive::Drive`, from the executable; `docs/drive-model.md`, "The
//! Drive gauge in KH1"); the tables are `crate::drivegauge`.
//!
//! - **What it is.** Bars of 100 points up to a maximum of 3, +1 for each of six fights won
//!   (`drivegauge::max_bars`, from the story flags), shown once the first Drive Form is open
//!   (`drivegauge::shown`). The bars and points live in KH1's save block
//!   (`drivegauge::BARS_OFF` / `POINTS_OFF`): every change is written there at once, and every use
//!   reads them back, so a save keeps the gauge and a load brings its own.
//! - **Filling.** Sora's hits (`hit`, from KH1's hit record `probe::hit`: KH2's ATKP drive value
//!   for a KH2 hit, `drivegauge::KH1_PHYSICAL` / `KH1_SPELL` for KH1's), damage he takes (`frame`,
//!   KH2's Damage Drive rule) and Drive orbs (`orb`, `prize`).
//! - **In a form** the gauge is the form's countdown (`enter`, `frame`, `ran_out`): `(level + 3) ×
//!   600` frames, the level the form's growth ability's (`form::form_level`), 1 a frame in battle
//!   and 0.5 outside, less per hit, more per orb (Anti: its own 6 bars, orbs drain it).
//!   The bars left after the cost wait under it; there is no refill on the way out.
//! - **The log**: one summary line a second while it changes, never one per hit.

use super::log;
use crate::drivegauge as dg;
use character_core::drive::{form as dform, Drive, FormGauge};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::Mutex;

/// KH1's battle flag (bit 0; `takeover`'s `BATTLE_RVA`).
const BATTLE_RVA: super::addr::Rva = super::addr::Rva(0x2867CD8);

static STATE: Mutex<Option<Drive>> = Mutex::new(None);
/// The maximum now (0: not read yet), and whether the gauge shows (a Drive Form is open).
static MAX: AtomicU8 = AtomicU8::new(0);
static SHOWN: AtomicBool = AtomicBool::new(false);
/// The level of the form entered last (`enter`; the backstop in `frame` starts its gauge again).
static LEVEL: AtomicU8 = AtomicU8::new(1);
/// The form gauge ran out this frame or earlier (taken by `form::drain`).
static RAN_OUT: AtomicBool = AtomicBool::new(false);
/// Sora's actor and HP last frame (damage taken), and his max HP then.
static LAST_ACTOR: AtomicUsize = AtomicUsize::new(0);
static LAST_HP: AtomicI32 = AtomicI32::new(-1);
static LAST_MAX_HP: AtomicI32 = AtomicI32::new(-1);
/// What changed it since the last summary line: points from hits (and their count), from damage,
/// from orbs (and their count); in a form, frames drained by time, by hits, refilled by orbs.
static HIT_POINTS: AtomicI32 = AtomicI32::new(0);
static HITS: AtomicU32 = AtomicU32::new(0);
static DAMAGE_POINTS: AtomicI32 = AtomicI32::new(0);
static ORB_POINTS: AtomicI32 = AtomicI32::new(0);
static ORBS: AtomicU32 = AtomicU32::new(0);
static FRAMES: AtomicU32 = AtomicU32::new(0);
/// The gauge at the last summary (bars << 8 | points, or the form gauge's frames left), and the
/// frame counter.
static LOGGED: AtomicU32 = AtomicU32::new(u32::MAX);
static TICKS: AtomicU32 = AtomicU32::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn base() -> usize {
    unsafe { super::sys::GetModuleHandleA(std::ptr::null()) as usize }
}

/// The Drive list's form as `story`'s (`None`: Anti Form).
fn story_form(kind: super::form::Kind) -> Option<crate::story::Form> {
    use super::form::Kind;
    use crate::story::Form;
    match kind {
        Kind::Valor => Some(Form::Valor),
        Kind::Wisdom => Some(Form::Wisdom),
        Kind::Limit => Some(Form::Limit),
        Kind::Master => Some(Form::Master),
        Kind::Final => Some(Form::Final),
        Kind::Anti => None,
    }
}

/// KH2's form id for ours.
fn kh2_form(kind: super::form::Kind) -> u8 {
    use super::form::Kind;
    match kind {
        Kind::Valor => dform::VALOR,
        Kind::Wisdom => dform::WISDOM,
        Kind::Limit => dform::LIMIT,
        Kind::Master => dform::MASTER,
        Kind::Final => dform::FINAL,
        Kind::Anti => dform::ANTI,
    }
}

/// A Drive's cost in bars (`drivegauge::cost`).
pub fn cost(kind: super::form::Kind) -> u8 {
    dg::cost(story_form(kind))
}

/// The maximum from the save's story flags (`drivegauge::MAX_TABLE`) and whether the gauge shows
/// (a Drive Form open, `drivegauge::shown`), each logged when it changes.
unsafe fn read_max(save: usize) -> u8 {
    let Some(flags) = super::menu::story_flags_at(save) else { return MAX.load(Ordering::Relaxed).max(dg::BASE_BARS) };
    let choice = rd::<u8>(save + crate::story::DIVE_CHOICE);
    let max = dg::max_bars(&flags);
    let shown = dg::shown(|f| crate::story::unlocked(f, choice, &flags));
    let was_shown = SHOWN.swap(shown, Ordering::Relaxed);
    if MAX.swap(max, Ordering::Relaxed) != max {
        let won = dg::raised_by(&flags);
        log(&format!("Drive gauge: maximum {max} bars ({} base{}{})", dg::BASE_BARS, if won.is_empty() { "" } else { ", " }, won.join(", ")));
    }
    if was_shown != shown {
        log(if shown { "Drive gauge: shown (a Drive Form is open)" } else { "Drive gauge: hidden until the first Drive Form opens" });
    }
    max
}

/// Runs `f` on the gauge, its bars and points as the save has them (and its maximum as the story
/// has it), and writes the bars and points back. `None` without a save block.
fn with<R>(f: impl FnOnce(&mut Drive) -> R) -> Option<R> {
    unsafe {
        let save = super::menu::save_block(base())?;
        let max = match MAX.load(Ordering::Relaxed) {
            0 => read_max(save),
            m => m,
        };
        let mut g = STATE.lock().ok()?;
        let d = g.get_or_insert_with(|| Drive::new(max));
        d.max = max;
        let (have_b, have_p) = (rd::<u8>(save + dg::BARS_OFF), rd::<u8>(save + dg::POINTS_OFF));
        let (b, p) = dg::from_save(have_b, have_p, max);
        if have_b & dg::SET != 0 && ((have_b & !dg::SET) > dg::MAX_BARS || have_p > 100) {
            static ONCE: AtomicBool = AtomicBool::new(false);
            if !ONCE.swap(true, Ordering::Relaxed) {
                log(&format!("Drive gauge: the save's bytes ({have_b}, {have_p}) aren't a gauge: empty"));
            }
        }
        d.bars = b;
        d.points = p;
        let r = f(d);
        // (Nothing written before the first form opens: the gauge is full when it first shows,
        // at the maximum it has then.)
        if SHOWN.load(Ordering::Relaxed) && (d.bars | dg::SET, d.points) != (have_b, have_p) {
            wr(save + dg::BARS_OFF, d.bars | dg::SET);
            wr(save + dg::POINTS_OFF, d.points);
        }
        Some(r)
    }
}

/// What the HUD draws (`gaugehud`).
#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub bars: u8,
    pub points: u8,
    pub max: u8,
    pub form: Option<FormGauge>,
    /// Whether it is drawn at all (a Drive Form is open, `drivegauge::shown`).
    pub shown: bool,
}

pub fn snapshot() -> Option<Snapshot> {
    with(|d| Snapshot { bars: d.bars, points: d.points, max: d.max, form: d.gauge, shown: SHOWN.load(Ordering::Relaxed) })
}

/// Whether the gauge pays for form `kind` now (KH2: a Drive is greyed when the gauge is short; ours,
/// Anti Form wants `drivegauge::ANTI_NEEDS` bars).
pub fn can_pay(kind: super::form::Kind) -> bool {
    let need = if kind == super::form::Kind::Anti { dg::ANTI_NEEDS } else { cost(kind) };
    with(|d| d.gauge.is_none() && d.bars >= need).unwrap_or(false)
}

/// The gauge as text (refusals and the log): "2 bars + 40 of 5".
pub fn describe() -> String {
    with(|d| match d.gauge {
        Some(g) => format!("in a form, {:.0} of {:.0} frames", g.left, g.max),
        None => format!("{} bars + {} of {}", d.bars, if d.bars >= d.max { 0 } else { d.points }, d.max),
    })
    .unwrap_or_else(|| "no save".into())
}

/// Pays form `kind`'s cost (KH2's `SpendDrive`: a part-filled bar survives unless the gauge was
/// full or short). Returns the gauge before and after, for the log.
pub fn pay(kind: super::form::Kind) -> (String, String) {
    let before = describe();
    with(|d| d.spend(cost(kind)));
    (before, describe())
}

/// The form has arrived: its gauge, `(level + 3) × 600` frames with `level` the form's
/// (`form::form_level`, 1..=4; Anti its own 6 bars). Returns its length.
pub fn enter(kind: super::form::Kind, level: u8) -> f32 {
    LEVEL.store(level, Ordering::Relaxed);
    RAN_OUT.store(false, Ordering::Relaxed);
    LOGGED.store(u32::MAX, Ordering::Relaxed);
    with(|d| {
        d.enter(kh2_form(kind), level);
        d.gauge.map_or(0.0, |g| g.max)
    })
    .unwrap_or(0.0)
}

/// The form has ended (any way): the gauge is the Drive gauge again, as the cost left it.
pub fn revert() {
    RAN_OUT.store(false, Ordering::Relaxed);
    LOGGED.store(u32::MAX, Ordering::Relaxed);
    with(|d| d.revert());
}

/// The form gauge has run out (`form::drain` asks; the revert is the form's).
pub fn ran_out() -> bool {
    RAN_OUT.load(Ordering::Relaxed)
}

/// Whether the gauge is shown now (a Drive Form open, `drivegauge::shown`), read from the save.
pub fn form_open() -> bool {
    unsafe {
        let Some(save) = super::menu::save_block(base()) else { return false };
        read_max(save);
    }
    SHOWN.load(Ordering::Relaxed)
}

/// The bars as text, in a form too (the Drive Recovery log): "2 bars + 40 of 5".
pub fn describe_bars() -> String {
    with(|d| {
        format!(
            "{} bars + {} of {}{}",
            d.bars,
            if d.bars >= d.max { 0 } else { d.points },
            d.max,
            if d.gauge.is_some() { " (waiting under the form's gauge)" } else { "" }
        )
    })
    .unwrap_or_else(|| "no save".into())
}

/// A Drive Recovery can be used (`driveitems::usable`, KH2's test: the gauge shown and not full;
/// in a form, the bars under the form's gauge).
pub fn recovery_usable() -> bool {
    form_open() && with(|d| crate::driveitems::usable(true, d.bars, d.max)).unwrap_or(false)
}

/// A Drive Recovery used: `points` added through the gauge's add (KH2's: the bars, in a form the
/// bars under the form's gauge). Returns the gauge before and after, for the log.
pub fn recover(points: i32) -> (String, String) {
    let before = describe_bars();
    form_open();
    with(|d| d.add_points(points));
    LOGGED.store(u32::MAX, Ordering::Relaxed);
    (before, describe_bars())
}

/// Fills the gauge (a testing aid): every bar, or the form gauge in a form.
#[allow(dead_code)]
pub fn refill() {
    with(|d| d.refill());
    log(&format!("Drive gauge refilled: {}", describe()));
}

/// KH1's foe test (`0x14028FE30 (a, b)`): `b` isn't marked not-a-foe (`0x200`), and has a team
/// (`+0x374` bits 5-6) other than `a`'s.
unsafe fn foe(a: usize, b: usize) -> bool {
    if b < 0x10000 || !super::crash::readable(b + 0x378) {
        return false;
    }
    let team = |x: usize| (rd::<u32>(x + 0x374) >> 5) & 3;
    rd::<u32>(b) & 0x200 == 0 && team(b) != 0 && team(b) != team(a)
}

/// One of Sora's hits landed on `target` (KH1's hit record was made, `probe::hit`), with KH2's
/// ATKP drive value `d` (KH1's own hits: `drivegauge::KH1_PHYSICAL` / `KH1_SPELL`). KH2's rule:
/// out of a form, `d × 0.75` on a normal enemy, truncated; in a form, the form gauge loses `15 ×
/// d` frames (× 0.2 on a normal enemy). Only foes count (not a partner Cure reaches).
pub unsafe fn hit(sora: usize, target: usize, d: u8) {
    if d == 0 || !foe(sora, target) {
        return;
    }
    // (KH1 has no boss mark this reads: every foe counts as KH2's normal enemy, its ×0.75 and ×0.2.)
    with(|g| {
        let (b, p, left) = (g.bars as i32, g.points as i32, g.gauge.map(|f| f.left));
        g.on_hit(d, true, false);
        match (left, g.gauge) {
            (Some(l), Some(f)) => {
                HIT_POINTS.fetch_add((f.left - l).round() as i32, Ordering::Relaxed);
            }
            _ => {
                HIT_POINTS.fetch_add((g.bars as i32 * 100 + g.points as i32) - (b * 100 + p), Ordering::Relaxed);
            }
        }
        HITS.fetch_add(1, Ordering::Relaxed);
    });
}

/// A Drive orb picked up (`prize`): `value` points (KH2's small 1, big 3), or in a form `value ×
/// 12` frames of the form gauge (Anti's lose them).
pub fn orb(value: u8) {
    with(|g| {
        let (b, p, left) = (g.bars as i32, g.points as i32, g.gauge.map(|f| f.left));
        g.orb(value);
        let gained = match (left, g.gauge) {
            (Some(l), Some(f)) => (f.left - l).round() as i32,
            _ => (g.bars as i32 * 100 + g.points as i32) - (b * 100 + p),
        };
        ORB_POINTS.fetch_add(gained, Ordering::Relaxed);
        ORBS.fetch_add(1, Ordering::Relaxed);
    });
}

/// Each of Sora's frames (`takeover`'s physics step): the maximum, the form gauge's drain, damage
/// taken, and the summary line.
pub unsafe fn frame(actor: usize) {
    let base = base();
    let t = TICKS.fetch_add(1, Ordering::Relaxed);
    let Some(save) = super::menu::save_block(base) else { return };
    // (The story flags every half second: the maximum follows a form opening, or a save loaded.)
    if t % 30 == 0 {
        read_max(save);
    }
    let battle = rd::<u32>(base + BATTLE_RVA) & 1 != 0;
    // (A backstop: the form gauge goes with the form, whatever ended or started it.)
    let has_gauge = with(|d| d.gauge.is_some()).unwrap_or(false);
    if has_gauge && !super::form::in_form() && !super::form::busy() {
        log("Drive gauge: a form gauge with no form: cleared");
        revert();
    } else if !has_gauge && super::form::in_form() {
        if let Some(kind) = super::form::current() {
            let level = LEVEL.load(Ordering::Relaxed);
            log(&format!("Drive gauge: {} with no form gauge: started (level {level})", kind.name()));
            enter(kind, level);
        }
    }
    // The form gauge drains (KH2: every frame of the form, half as fast out of battle).
    let empty = with(|d| d.gauge.is_some() && d.tick(battle)).unwrap_or(false);
    if super::form::in_form() {
        FRAMES.fetch_add(1, Ordering::Relaxed);
    }
    if empty && !RAN_OUT.swap(true, Ordering::Relaxed) {
        log(&format!("{}: the form gauge ran out", super::form::current().map_or("The form", |k| k.name())));
    }
    // Damage taken (KH2's HP-change rule): his HP down from last frame, the same actor and max HP.
    let stats = super::pose::ptr(rd::<u32>(actor + 0x6C));
    if stats > 0x10000 && super::crash::readable(stats + 0x44) {
        let (hp, max_hp) = (rd::<i32>(stats + 0x3C), rd::<i32>(stats + 0x40));
        let same = LAST_ACTOR.swap(actor, Ordering::Relaxed) == actor && LAST_MAX_HP.swap(max_hp, Ordering::Relaxed) == max_hp;
        let last = LAST_HP.swap(hp, Ordering::Relaxed);
        if same && last > hp && hp >= 0 {
            let n = dg::damage_points(last - hp, max_hp);
            // (Out of a form only: KH2's damage rule runs in its normal gauge mode.)
            if n > 0 && with(|d| {
                if d.gauge.is_some() {
                    return false;
                }
                d.add_points(n);
                true
            }) == Some(true)
            {
                DAMAGE_POINTS.fetch_add(n, Ordering::Relaxed);
            }
        }
    }
    // The summary, once a second while it changes.
    if t % 60 == 0 {
        summary();
    }
}

fn summary() {
    let Some(s) = snapshot() else { return };
    let key = match s.form {
        Some(g) => 0x8000_0000 | g.left as u32,
        None => (s.bars as u32) << 8 | s.points as u32 | (s.max as u32) << 16,
    };
    let (hp, hits, dp, op, orbs, frames) = (
        HIT_POINTS.swap(0, Ordering::Relaxed),
        HITS.swap(0, Ordering::Relaxed),
        DAMAGE_POINTS.swap(0, Ordering::Relaxed),
        ORB_POINTS.swap(0, Ordering::Relaxed),
        ORBS.swap(0, Ordering::Relaxed),
        FRAMES.swap(0, Ordering::Relaxed),
    );
    if LOGGED.swap(key, Ordering::Relaxed) == key {
        return;
    }
    let mut why = Vec::new();
    if hits > 0 {
        why.push(format!("{hp:+} from {hits} hit{}", if hits == 1 { "" } else { "s" }));
    }
    if dp != 0 {
        why.push(format!("{dp:+} from damage"));
    }
    if orbs > 0 {
        why.push(format!("{op:+} from {orbs} Drive orb{}", if orbs == 1 { "" } else { "s" }));
    }
    match s.form {
        Some(g) => {
            if frames > 0 {
                why.insert(0, format!("{frames} frames of the form"));
            }
            log(&format!(
                "{}'s gauge: {:.0} of {:.0} frames ({:.1} bars){}",
                super::form::current().map_or("The form", |k| k.name()),
                g.left,
                g.max,
                g.bars(&Default::default()),
                if why.is_empty() { String::new() } else { format!("; the last second: {}", why.join(", ")) }
            ));
        }
        None => log(&format!(
            "Drive gauge: {} bars + {} of {}{}",
            s.bars,
            if s.bars >= s.max { 0 } else { s.points },
            s.max,
            if why.is_empty() { String::new() } else { format!("; the last second: {}", why.join(", ")) }
        )),
    }
}
