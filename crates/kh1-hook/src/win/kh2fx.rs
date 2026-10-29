//! KH2's own visual effects for Sora (the user: Magnet Burst's and Explosion's real effects, then
//! every one), played by KH1's own effect system (`kh2dpx`: KH2's effect packages made into KH1
//! effect files at run time, started with KH1's own calls; `docs/kh2-effects.md` §16-18).
//!
//! This is the front the game's events call: a move's frame trigger 1 (`event`), a form's
//! (`form_event`), Revert's flashes (`form`), the Keyblade's vanish flash (`weapon_vanish`), the
//! spell scripts' effects (`spell_*`, `reflect` and `formmagic`), and each of Sora's frames
//! (`tick`: the packages built and freed, the endings and fades) and his object going
//! (`owner_gone`).
//!
//! The old effect player (KH2's particle machine in Rust, its own instances and D3D12 draw after
//! KH1's world-model pass, t300s..t430) is gone (the tester: "the older player is to be
//! deprecated"): everything goes through KH1. An element KH1 can't play (its package not built,
//! or left out when it was built: logged with the reason) doesn't play.
//!
//! Ini: `kh2_effects` (`magnet_explosion` default, `all`, `off`).

/// Explosion, Vicinity Break, Magnet Burst and Aerial Finish (the user: those three first;
/// Vicinity Break's was asked for when it went unnoticed without it, Aerial Finish's (motion
/// 194, Sora's combo row 29, ability 100) when it was missing), then Guard Break (165: elements
/// 31, 30, 32), Finishing Leap (167: 33, 34) and Aerial Recovery (40: element 0), the user
/// finding them without theirs; Quick Run (203: its start effect above LV1, `quickrun`, the
/// tester's quick-run-native, 1:1 with KH2).
pub const DEFAULT_MOTIONS: [u16; 8] = [166, 170, 197, 194, 165, 167, 40, 203];
use character_core::EventKind;
use glam::Mat4;

pub(super) fn allowed(motion: u16) -> bool {
    match &super::settings::get().kh2_effects {
        None => false,
        Some(None) => true,
        Some(Some(v)) => v.contains(&motion),
    }
}

/// The Keyblade's vanish flash: element 0 of its package (`obj/W_EX010.a.us`: a flash that swells
/// and fades over 13 ticks with a turning glint; KH2's own call for it not traced), started where
/// the blade is as KH2 puts it away (the user: "when a keyblade disappears in any animation there
/// isn't any visual effects from kh2 for a dissapearing keyblade").
pub const WEAPON_VANISH: u16 = 0;

pub fn weapon_vanish() {
    if super::settings::get().kh2_effects.is_none() {
        return;
    }
    if super::kh2dpx::weapon_vanish() {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        if N.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 10 {
            super::log!(Effects, "KH2 effect: the Keyblade's vanish");
        }
    }
}

/// The spell packages' numbering (`kh2dpx::Pkg::Spell`): Reflect's levels 1..3, Wisdom Form's
/// archive (`P_EX100_MAGF.a.us`, PAX "p_ex": its shots) 4, then Wisdom's and base Sora's spells,
/// Fire / Blizzard / Thunder / Cure by level (`form::spell_package`).
pub const WISDOM: u8 = 4;

/// Valor Form's package number for its elements' sounds (`kh2sound`, `formmagic`), after the
/// spells': its motions' frame trigger 1 starts its own elements, not Sora's (`form_event`).
pub const VALOR: u8 = super::form::LAST_PACKAGE + 1;

/// Final Form's (`P_EX100_ULTF.a.us`), after Valor's.
pub const FINAL: u8 = VALOR + 1;

/// A frame trigger 1 of Valor's or Final's motion (`takeover`, `form::takes`): its element from
/// that form's package, on Sora; one that ends with its motion ends when the motion changes.
pub fn form_event(motion: u16, el: i16) {
    if el < 0 || super::settings::get().kh2_effects.is_none() {
        return;
    }
    if super::kh2dpx::form_event(super::form::in_final(), motion, el) {
        unsafe { super::kh2sound::spell_element(if super::form::in_final() { FINAL } else { VALOR }, el as u16) };
    }
}

/// Sora's object replaced (a form change, `form`): his effects that end with their owner end
/// (the Drive motion's glows on his limbs loop until then; the user: the glow stayed on him).
pub fn owner_gone() {
    super::kh2dpx::owner_gone();
}

/// Puts a spell effect's root where the script has it (an unbound one it moves: Wisdom's shots,
/// Reflect's contact flash), a KH2-world (y down) matrix.
pub fn spell_place(h: u32, at: Mat4) {
    super::kh2dpx::spell_place(h, at);
}

/// Starts element `el` of spell package `level` on Sora (its bone code as for his own effects),
/// for the spell scripts (`pax_start_bind`). Returns its handle.
pub fn spell_start(level: u8, el: u16) -> Option<u32> {
    if super::settings::get().kh2_effects.is_none() {
        return None;
    }
    match super::kh2dpx::spell_start(level, el) {
        super::kh2dpx::SpellStart::Kh1(h) => Some(h),
        super::kh2dpx::SpellStart::NotStarted | super::kh2dpx::SpellStart::NotMine => None,
    }
}

/// A spell package is built and entered in KH1 (for the log).
pub fn spell_ready(level: u8) -> bool {
    super::kh2dpx::ready(super::kh2dpx::Pkg::Spell(level))
}

/// What the spell script did to an effect it started: ended its loop (`end`), killed it, or set
/// its fade (`alpha`).
pub fn spell_end(h: u32) {
    super::kh2dpx::spell_end(h);
}

pub fn spell_kill(h: u32) {
    super::kh2dpx::spell_kill(h);
}

pub fn spell_alpha(h: u32, alpha: f32) {
    super::kh2dpx::spell_alpha(h, alpha);
}

/// The script fades its effect out over `frames` (60 Hz frames): run on KH1's instance by our
/// effect tick (t826).
pub fn spell_fade(h: u32, frames: f32) {
    super::kh2dpx::spell_fade(h, frames);
}

pub fn spell_running(h: u32) -> bool {
    super::kh2dpx::spell_running(h)
}

/// A KH2 controller event (from `takeover`, each frame KH2 has Sora; `kh2slots` while KH1 plays
/// one of KH2's motions; `form` for Revert's flashes).
pub fn event(motion: u16, kind: EventKind) {
    let EventKind::Effect(el) = kind else { return };
    if el < 0 || !(allowed(motion) || super::form::own_motion(motion)) {
        return;
    }
    super::kh2dpx::event(motion, el);
}

/// `tick`'s motion while KH1 has Sora: no KH2 motion (effects bound to one end).
pub const NO_MOTION: u16 = u16::MAX - 1;

/// One game frame of KH2's effects: KH2's motion now, and the frame's delta in 60 Hz frames (1.0
/// at 60 fps, 2.0 at 30, like the game's own frame delta): the packages wanted now built and
/// entered, the elements that end with their motion ended, fades stepped (`kh2dpx`).
pub fn tick(motion: u16, delta: f32) {
    super::kh2dpx::tick(motion, delta);
}
