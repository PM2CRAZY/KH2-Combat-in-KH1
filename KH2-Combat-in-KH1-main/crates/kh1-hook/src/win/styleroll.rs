//! KH1's own dodge roll for base Sora (`styles`: Dodge Roll set to KH1; the user: "add an option
//! for switching between dodge roll"). Drive Forms keep KH2's (`styles::kh1_now`).
//! `docs/kh1-move-styles.md`.
//!
//! KH1's roll (from its per-frame button handler `0x1402A59A0`, on Square in its ground state 0
//! when not in deep water `0x1402BF530`): with the Dodge Roll ability (bit `0x20` of
//! `0x142F5F610`; `0x10` is Guard, and with both the stick's tilt against the stats' threshold
//! picks between them) it faces him along the stick (`0x140295E20 (actor, dir)` gives the angle,
//! `0x140295C80 (actor, angle)` sets it) and starts the roll (`0x1402C7CB0`: the actor's rolling
//! flag `0x100000`, motion `0xDC`, then `0x14029D8A0 (0x40000000)`). Its untouchable frames are
//! its motion's events (flag `0x40`, `docs/kh1-intangible.md`).
//!
//! Here: Square with the stick, on the ground, KH2 having him free (idle, walk, run, landing),
//! runs that roll instead of KH2's (`takeover::style_roll_start`), and KH1 has him through it
//! (motion `0xDC` playing): no hand-back on the stick meanwhile. An Attack KH1's command step
//! takes during the roll (KH1's own window for it) goes by the Base Combo style: KH1's combo
//! from there, or KH2 takes him then with the press for its combo. After the roll, KH2 takes him
//! back as after KH1's other moves (stick, jump, Square, Attack). Square standing still is KH2's
//! guard as before; a command chosen in the roll (magic, an item, a Drive) waits for it to end and
//! is KH2's then (by the Magic styles); without KH1's Dodge Roll ability the press is KH2's (its roll or Quick Run).
//! KH2's roll-into-Quick-Run (Square held through the roll) is KH2's roll's, so not out of KH1's.

use super::styles::{self, Kind};
use std::sync::atomic::{AtomicBool, Ordering};

/// The hand-off now running is KH1's roll's.
static ON: AtomicBool = AtomicBool::new(false);
/// KH1 took an Attack during its roll and the Base Combo is KH2's: KH2 takes him with it.
static ATTACK: AtomicBool = AtomicBool::new(false);

/// A command chosen during the roll: KH2's, once the roll is over.
static WANT_KH2: AtomicBool = AtomicBool::new(false);

/// KH1's roll motion (its rolling flag, `0x100000` on the actor, isn't read: nothing seen clears it).
pub const ROLL_MOTION: u32 = 0xDC;

/// The dodge roll is KH1's for base Sora now.
pub fn kh1_roll() -> bool {
    styles::kh1_now(Kind::DodgeRoll)
}

/// KH1 has him for its roll now.
pub fn active() -> bool {
    super::takeover::handing_off() && !super::takeover::FROM_COMMAND.load(Ordering::Relaxed) && ON.load(Ordering::Relaxed)
}

/// Any hand-off to KH1 (`takeover::hand_to_kh1`) ends it; ours sets it after.
pub fn clear() {
    ON.store(false, Ordering::Relaxed);
    ATTACK.store(false, Ordering::Relaxed);
    WANT_KH2.store(false, Ordering::Relaxed);
}

/// KH1 took him for its roll (after `hand_to_kh1`).
pub fn started() {
    ON.store(true, Ordering::Relaxed);
    ATTACK.store(false, Ordering::Relaxed);
    WANT_KH2.store(false, Ordering::Relaxed);
}

/// A command chosen during the roll (`combat::command`): KH2 runs it once it has him.
pub fn want_kh2() {
    WANT_KH2.store(true, Ordering::Relaxed);
}

pub fn wanted() -> bool {
    WANT_KH2.load(Ordering::Relaxed)
}

/// An Attack KH1 took in its roll, for KH2's combo (`combat::exec`).
pub fn attack() {
    ATTACK.store(true, Ordering::Relaxed);
}

/// Taken by `takeover`'s hand-back.
pub fn take_attack() -> bool {
    ATTACK.swap(false, Ordering::Relaxed)
}
