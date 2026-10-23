//! KH1's own jump and glide for base Sora (`styles`: Jump / Glide set to KH1; the user: "can we
//! also add a setting for kh1 jump or kh2 jump and ... kh1 or kh2 glide"). Drive Forms keep
//! KH2's (`styles::kh1_now`). `docs/kh1-move-styles.md`.
//!
//! - Jump at KH1: KH2's fresh jump press on the ground, free, runs KH1's own ground jump press
//!   handler instead (`0x1402CA3F0`, ground state 0's entry `+0x38` in KH1's state table: its
//!   ground test `0x1402A0D30`, then the jump start `0x1402AD860`: jump or high jump motion, state
//!   1, air time 0), and KH1 has him (`takeover::hand_to_kh1`, mode `JUMP`) all through the jump:
//!   its rise, fall, ledge grabs and landing. KH2 takes him back in the air only for one of its
//!   own moves there (an Attack with the Aerial Base Combo at KH2, Square: Aerial Dodge / KH2's
//!   glide, the jump press for KH2's glide on the jump button, any command: magic, items), or on
//!   the ground once he is free as after KH1's other moves.
//! - Glide at KH1: in KH2's jump, a fresh jump press while falling puts him in KH1's air state 2
//!   (as an air command does: the air-time fields, KH2's sideways speed) and starts KH1's own
//!   glide, Superglide when he has it (`0x1402B05C0`, ability 4 by `0x140282A70 (4, 0xB)`), else
//!   Glide (`0x1402B0530`, ability 3); KH1 has him (mode `GLIDE`) until the glide ends: back in
//!   its fall, KH2's fall; on the ground, KH2's landing. KH2's own glide is off meanwhile. In KH1's
//!   jump, KH1's glide is its own (the jump press is left to KH1's control step).

use super::styles::{self, Kind};
use super::takeover;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

pub const JUMP: u8 = 1;
pub const GLIDE: u8 = 2;

/// The hand-off now running is KH1's jump's (`JUMP`) or glide's (`GLIDE`), 0 none.
static MODE: AtomicU8 = AtomicU8::new(0);
/// Something of KH2's was asked for during it (a command): KH2 takes him back.
static WANT_KH2: AtomicBool = AtomicBool::new(false);
/// Square and the jump button on the hand-off's last frame (fresh presses only).
static LAST_SQUARE: AtomicBool = AtomicBool::new(true);
static LAST_JUMP: AtomicBool = AtomicBool::new(true);

/// Jump is KH1's for base Sora now.
pub fn kh1_jump() -> bool {
    styles::kh1_now(Kind::Jump)
}

/// Glide is KH1's for base Sora now.
pub fn kh1_glide() -> bool {
    styles::kh1_now(Kind::Glide)
}

/// KH1 has him for its jump or glide now: the mode, or 0.
pub fn mode() -> u8 {
    if takeover::handing_off() && !takeover::FROM_COMMAND.load(Ordering::Relaxed) {
        MODE.load(Ordering::Relaxed)
    } else {
        0
    }
}

pub fn active() -> bool {
    mode() != 0
}

/// KH1's jump hand-off with KH1's glide: the jump button is KH1's (its glide) in its control step.
pub fn kh1_keeps_jump() -> bool {
    // (No more: KH1's glide is on Square now, started by `takeover`, and the jump press in KH1's
    // air is KH2's Aerial Dodge, the user.)
    false
}

/// Any hand-off to KH1 (`takeover::hand_to_kh1`) ends the mode; ours sets it after.
pub fn clear() {
    MODE.store(0, Ordering::Relaxed);
    WANT_KH2.store(false, Ordering::Relaxed);
}

/// KH1 took him for its jump or glide (after `hand_to_kh1`); the buttons are held as now.
pub fn started(mode: u8, square: bool, jump: bool) {
    MODE.store(mode, Ordering::Relaxed);
    WANT_KH2.store(false, Ordering::Relaxed);
    LAST_SQUARE.store(square, Ordering::Relaxed);
    LAST_JUMP.store(jump, Ordering::Relaxed);
}

/// A command chosen during KH1's jump or glide: KH2's to run once it has him (`combat::command`).
pub fn want_kh2() {
    WANT_KH2.store(true, Ordering::Relaxed);
}

/// A command waits for KH2 (kept until the next hand-off of ours).
pub fn wanted() -> bool {
    WANT_KH2.load(Ordering::Relaxed)
}

/// This frame's Square and jump presses during the hand-off (fresh: not held from before).
pub fn presses(square: bool, jump: bool) -> (bool, bool) {
    let was = (LAST_SQUARE.swap(square, Ordering::Relaxed), LAST_JUMP.swap(jump, Ordering::Relaxed));
    (square && !was.0, jump && !was.1)
}
