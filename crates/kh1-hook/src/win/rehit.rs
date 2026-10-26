//! KH2's hit record on the KH1 attack objects of Sora's KH2 moves (testing branch, t-rehit2; the
//! tester, combo row selection after m33: "if other moves are multi hits in kh2 and don't have it
//! right now (aerial finish) then yes apply it ... see if we are not writing two codes for the same
//! thing. check it behind kh2 c decomp"). `docs/kh1-rehit.md`.
//!
//! KH2's rule (Steam KH2, Checked (decomp)):
//! - Each frame a motion's attack range (range id 10) is open, the range handler (`0x14040D810`)
//!   asserts its ATKP for its collision group (`0x1403DAFE0` → the object's `+0x98` call →
//!   `0x1403D25D0`, the object's 8 attack slots at `+0xC10`, one per group and owner). The same
//!   ATKP on the group's slot keeps that attack (and what it has hit); another ATKP ends it
//!   (`0x1403D0EB0`) and makes a new attack with a new id (`0x14040F1C0`), which may hit
//!   everything again. An attack whose group is no longer asserted ends in its own update
//!   (`0x14040F610`).
//! - Within one attack, a target it hit may be hit again only after the hit ATKP row's `interval`
//!   (60 Hz frames; KH2's per-target re-hit slots, `combat_core::HitRecord::mark` / `tick`, from
//!   the OG rewrite: Checked (code)).
//!
//! On KH1's side each of KH2's attacks is a KH1 attack object of Sora's, and KH1's hit-once is the
//! attack's bit (`+0x54`) in each actor's `+0x3E0` (`0x1402BD480`, `docs/kh1-rehit.md`):
//! - KH2's windows on his body are one KH1 object each (`bodyhits`), so a new KH2 attack is a new
//!   KH1 object already.
//! - KH2's windows on the Keyblade all play on the one attack object KH1's row makes for the whole
//!   motion: a new KH2 attack there (`kh2slots::keyblade_rearm`) re-arms that object with KH1's own
//!   re-arm `0x1402B3380` (`new_attack`).
//! - KH2's per-target interval, for both kinds: each hit is noted with its row's interval (`on_hit`,
//!   from `probe::hit`) and the target's bit is taken off once it has run (`tick`, each of Sora's
//!   frames, before KH1's collision pass). KH1's own interval (`+0x50` / `+0xA4`, one clock for
//!   every target at once) stays off, as KH2's runs per target.
//!
//! Before, `bodyhits` kept its own per-target interval for the body windows (Explosion's 911 every
//! 8 frames), and the Keyblade windows had none: Aerial Sweep (909, 10), Aerial Spiral (1131, 12),
//! Aerial Dive (1882, 8) and Retaliating Slash (1582, 10) hit once.

use super::log;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// A hit waiting for its row's re-hit interval: the attack object, its hit bit then, the target,
/// `takeover::TICKS` at the hit, the interval (60 Hz frames).
#[derive(Clone, Copy)]
struct Pending {
    attack: usize,
    bit: u32,
    target: usize,
    at: u64,
    interval: u64,
}

static PENDING: Mutex<Vec<Pending>> = Mutex::new(Vec::new());
/// KH2 keeps four re-hit slots per target (`HitRecord`); kept loosely here, a bound for the list.
const MOST: usize = 64;

fn note(f: impl FnOnce() -> String) {
    static N: AtomicU32 = AtomicU32::new(0);
    if super::log_on(super::LogCat::Combat) && N.fetch_add(1, Ordering::Relaxed) < 60 {
        log!(Combat, "{}", f());
    }
}

/// `probe::hit`: Sora's `attack` just hit `target`. With its KH2 row's interval (the window's ATKP,
/// `kh1rows::kh2_atkp`: a body window's or a spell's through `bodyhits`, a Keyblade window's by
/// KH1's time in our slot), the target may be hit by it again once that has run (`tick`).
pub unsafe fn on_hit(attack: usize, target: usize) {
    if attack < 0x10000 || target < 0x10000 {
        return;
    }
    let Some(a) = super::kh1rows::kh2_atkp(attack) else { return };
    let interval = a.interval as u64;
    if interval == 0 {
        return;
    }
    let bit = rd::<u32>(attack + 0x54);
    if bit == 0 {
        return;
    }
    let at = super::takeover::TICKS.load(Ordering::Relaxed);
    let Ok(mut g) = PENDING.lock() else { return };
    g.retain(|p| !(p.attack == attack && p.target == target));
    if g.len() >= MOST {
        g.remove(0);
    }
    g.push(Pending { attack, bit, target, at, interval });
    drop(g);
    note(|| format!("KH2 re-hit: ATKP {} on {target:#x} again after {interval} frames", a.id));
}

/// Each of Sora's frames (`takeover::physics`, before KH1's collision pass): a target whose re-hit
/// interval has run loses the attack's bit (the attack still the one that hit: in use, same bit).
pub unsafe fn tick() {
    let Ok(mut g) = PENDING.lock() else { return };
    if g.is_empty() {
        return;
    }
    let now = super::takeover::TICKS.load(Ordering::Relaxed);
    g.retain(|p| {
        if now.saturating_sub(p.at) < p.interval {
            return true;
        }
        if rd::<u32>(p.attack) & 0x800 != 0 && rd::<u32>(p.attack + 0x54) == p.bit {
            wr(p.target + 0x3E0, rd::<u32>(p.target + 0x3E0) & !p.bit);
        }
        false
    });
}

/// KH1's re-arm of an attack (`0x1402B3380 (attack)`, Steam; Checked (decomp)): with its interval
/// `+0x50` (a float, > 0) reached by its clock `+0xA4`, it takes the attack's hit bit (`+0x54`) off
/// every actor's hit-once bits (`+0x3E0`, actor iterator `0x140291B20`) and off every attack's
/// met mask in the pool (`+0x9C`, pool `0x142DB4710`, 0xC0 each, in use `0x800`), so it can hit
/// again. KH1 runs it each frame for motion-tied attacks (`0x1402B47A0`).
const REARM_STEAM: usize = 0x2B3380;

/// A new KH2 attack on KH1's attack object `attack` (KH2's next attack of the group,
/// `kh2slots::keyblade_rearm`): KH1's own re-arm, its interval made due for the one call and put
/// back; the last attack's waiting re-hits dropped (the new one starts with no hits).
pub unsafe fn new_attack(attack: usize) {
    if super::addr::Build::current() != super::addr::Build::Steam || attack < 0x10000 {
        return;
    }
    if let Ok(mut g) = PENDING.lock() {
        g.retain(|p| p.attack != attack);
    }
    let rearm: unsafe extern "system" fn(usize) = std::mem::transmute(super::takeover::game_base() + REARM_STEAM);
    let (interval, clock) = (rd::<f32>(attack + 0x50), rd::<f32>(attack + 0xA4));
    wr(attack + 0x50, f32::MIN_POSITIVE);
    wr(attack + 0xA4, 1.0f32);
    rearm(attack);
    wr(attack + 0x50, interval);
    wr(attack + 0xA4, clock);
}
