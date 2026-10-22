//! Shared states: KH1's own state machine owns base Sora; KH2 brings its animations and its
//! physics to the moves both games have, and borrows him only for the moves KH1 doesn't have.
//!
//! The user's rewrite (2026-10-20, testing branch): "kh2 replaces the base animations that share
//! logic (idle walk, run) with kh2s animations ... but then reinjects kh1s soras animations back
//! into the game. We need to have a clean rewrite". The rule, per move style (`styles`, every one
//! KH1 by default):
//!
//! - **A shared move** (idle / walk / run / item use: Misc Animations; jump, high jump, fall,
//!   landing: Jump; the ground combo's swings and finisher: Base Combo; the glide: Glide) always
//!   runs in KH1's own states, with KH1's own rules (its motion slots, its transitions, ledge
//!   grabs, landing, its combo's rows, hit objects and damage). At KH1 nothing of ours touches it.
//!   At KH2, KH1's slot is drawn with KH2's motion for the same thing (`kh2_look`,
//!   `pose::follow`), "KH1 should treat KH2's animations as its own". Whose numbers move him is
//!   the physics setting's, not the style's (`physics`: with it at KH2, KH2's run speed, jump arc
//!   and swing travel on every move; at KH1, KH1's own). The user (after s261): "currently don't share animations but should: sora base
//!   attack 2, kh2 base attack 2 / kh1 base finisher, kh2 base finishers / glide".
//! - **The glide** is not shared any more (t321, the user: "after incorporating kh2 glide as a
//!   shared ability it stopped it from being 1:1 with kh2's glide"; "not have glide be affected by
//!   the physics setting"): at Glide KH1 it is KH1's own glide state with KH1's own numbers, at
//!   Glide KH2 a KH2 borrow (`GLIDE`), KH2's controller's own glide (its speed, fall, turn,
//!   banking, hover and release) on KH2's glide numbers. The physics setting doesn't touch it.
//! - **A KH2-only move** (Aerial Dodge, KH2's glide at Glide KH2, KH2's Square moves, KH2's Quick
//!   Run out of KH1's roll, the aerial combo at KH2, KH2's spells and Drives for now) is KH2's controller's: it borrows
//!   Sora out of KH1's states for it (`borrow`), and gives him back to KH1's ground or air state
//!   the moment the move is over (`give_back`). So with Jump at KH1, Aerial Dodge is still his
//!   double jump. With physics KH1 KH2's controller runs these on KH1's numbers
//!   (`physics::kh2_controller`).
//! - **A KH1-only move** (KH1's specials, summons) is KH1's, as unmodded.
//! - Drive Forms keep KH2's controller all through (their own movesets), as before.
//!
//! `takeover` keeps the hand-off mechanics (what KH1's fields are set to either way); this
//! module says when. `docs/shared-states.md`.

use super::styles::{self, Kind};
use super::takeover;
use character_core::StateName;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

/// Base Sora is KH1's own states' (the ini's `shared`, not in a Drive Form).
pub fn on() -> bool {
    super::settings::get().shared && !super::form::in_form()
}

/// The move style a KH1 motion slot of Sora's shows (`pose`'s slot numbers: the user's play
/// logs): 0..7 field and battle idle / walk / run; 8 / 9 jump and rise, 12 / 13 high jump and rise, 10
/// fall, 11 landing; 98..101 the ground combo's swings (98 the first and third, 99 the second,
/// 101 the finisher), 102..104 the air combo's; 75..78 KH1's glide (its pre-step `0x1402B06F0`
/// asks for requests 0x71 / 0x72 gliding slow / fast, 0x73 / 0x74 banking into a turn, Superglide
/// 0x76..0x79; KH1's request table maps both sets to slots 75..78, `docs/kh1-anim-slots.md`. The
/// s261 log's "set motion slot 113 / 115" are those request ids, and `pose::slot_of` read 75 / 77
/// meanwhile. 113..116 are attack motions, requests 0xD7..0xDA).
pub fn slot_kind(slot: usize) -> Option<Kind> {
    Some(match slot {
        0..=7 => Kind::MiscAnims,
        8..=13 => Kind::Jump,
        98..=101 => Kind::GroundCombo,
        102..=104 => Kind::AirCombo,
        GLIDE_SLOTS_START..=GLIDE_SLOTS_END => Kind::Glide,
        _ => return None,
    })
}

/// KH1's glide slots (`slot_kind`).
pub const GLIDE_SLOTS_START: usize = 75;
pub const GLIDE_SLOTS_END: usize = 78;

/// The Attack command runs KH1's own row where he is (KH1's command step, `combat::exec`): the
/// ground combo is a shared move (KH1's states at either style, KH2's swings and lunges drawn on
/// them at KH2: `pose::follow`, `physics`); the air combo is KH1's at KH1 and a KH2 borrow at KH2,
/// unless `kh1_slots` has built KH2's air swings as slots (181 / 182 / 183 on KH1's 102..104):
/// then it is KH1's air state at either style, KH2's swings played in its motion table.
pub fn kh1_combo(air: bool) -> bool {
    !air || styles::kh1_now(Kind::AirCombo) || super::kh2slots::covers(Kind::AirCombo)
}

/// KH2's motion is drawn over this KH1 slot: its style is KH2's (base Sora; a Drive Form draws
/// the form's motions over every slot it has one for, as before).
pub fn kh2_look(slot: Option<usize>) -> bool {
    if super::form::in_form() {
        return true;
    }
    slot.and_then(slot_kind).is_some_and(styles::kh2)
}

/// The hand-off now running is ours: KH1 has base Sora in its own states.
static MODE: AtomicBool = AtomicBool::new(false);
/// KH2's controller has him for one of its moves (`borrow` .. `give_back`).
static KH2_MOVE: AtomicBool = AtomicBool::new(false);

/// KH1 has base Sora in its own states now.
pub fn active() -> bool {
    MODE.load(Ordering::Relaxed) && takeover::handing_off()
}

/// KH2 has him for one of its own moves.
pub fn kh2_move() -> bool {
    KH2_MOVE.load(Ordering::Relaxed)
}

/// Any hand-off (`takeover::hand_to_kh1`) ends the mode; ours sets it after.
pub fn clear() {
    MODE.store(false, Ordering::Relaxed);
}

/// KH1's states have him from now (after `takeover::hand_to_kh1`).
pub fn started() {
    MODE.store(true, Ordering::Relaxed);
    KH2_MOVE.store(false, Ordering::Relaxed);
    BORROW.store(NONE, Ordering::Relaxed);
}

/// KH2's controller took him for a move.
pub fn lent() {
    MODE.store(false, Ordering::Relaxed);
    KH2_MOVE.store(true, Ordering::Relaxed);
    WANT_KH2.store(false, Ordering::Relaxed);
}

/// What KH2 borrows him for out of KH1's states, read off the buttons in KH1's control step
/// (`control`) and acted on in its physics step (`takeover`).
pub const NONE: u8 = 0;
/// Square: KH2's dodge roll / guard / Quick Run on the ground (Dodge Roll at KH2).
pub const SQUARE: u8 = 1;
/// The jump button in the air: Aerial Dodge.
pub const AIR_JUMP: u8 = 2;
/// Square held through KH1's own roll (Dodge Roll at KH1): KH2's Quick Run out of it, when Sora
/// has Quick Run (it has no style of its own: KH2's move, by its ability).
pub const QUICK_RUN: u8 = 3;
/// Square in the air with the Glide style at KH2: KH2's own glide (t321). Its button is Square, as
/// the KH2 rewrite binds it (t327, `kh2-bevy`: "Glide on the action button ... jump still Aerial
/// Dodges"), whether the Aerial Dodge is used or not; the jump in the air is the dodge only.
pub const GLIDE: u8 = 4;
static BORROW: AtomicU8 = AtomicU8::new(NONE);
/// Square held since it started KH1's roll (`QUICK_RUN`), and how long KH1's roll motion has
/// played in that hold (frames at 60 Hz, f32 bits; 0 not yet seen).
static ROLL_HELD: AtomicBool = AtomicBool::new(false);
static ROLL_FRAMES: AtomicU32 = AtomicU32::new(0);
/// A command of KH2's chosen while KH1 has him (`combat::command`: a KH2 spell, Reflect, a Drive):
/// KH2 takes him for it, then runs it (`combat::resume_command`).
static WANT_KH2: AtomicBool = AtomicBool::new(false);
/// Aerial Dodge used in this air time (KH2 allows one; back on the ground, again).
static AIR_DODGED: AtomicBool = AtomicBool::new(false);
/// The jump button has been let go since the Aerial Dodge's press (a glide needs a fresh press).
static DODGE_JUMP_UP: AtomicBool = AtomicBool::new(true);
/// Buttons on KH1's last control step, for fresh presses.
static LAST: AtomicU32 = AtomicU32::new(0);

pub fn want_kh2() {
    WANT_KH2.store(true, Ordering::Relaxed);
}

pub fn take_want_kh2() -> bool {
    WANT_KH2.swap(false, Ordering::Relaxed)
}

/// This frame's borrow, taken.
pub fn take_borrow() -> u8 {
    BORROW.swap(NONE, Ordering::Relaxed)
}

/// What KH1's control step sees of the buttons while KH1 has base Sora, and what it should not
/// see (`kh1_hides`: KH1's own bits to take out of its button state for its step).
pub struct Seen {
    /// KH1's movement state (0 ground, 1 jump, 2 fall, 0x15 glide...).
    pub state: i32,
    /// KH1's can-act test.
    pub can_act: bool,
    /// Going up this frame (KH1's jump still rising).
    pub rising: bool,
    pub jump: bool,
    /// Square on the pad (not KH1's action bit its held jump sets in the air).
    pub square: bool,
    /// KH2's Aerial Dodge is equipped.
    pub air_dodge: bool,
    /// KH1's Superglide is equipped: its glide is on Square (Glide alone: the jump button).
    pub superglide: bool,
    /// KH2's glide would start now (the Glide style is KH2's, KH2's Glide is equipped and he is
    /// above its start height).
    pub kh2_glide: bool,
    /// KH1's motion is in its combo window (a swing of KH1's own combo can chain now).
    pub combo_window: bool,
    /// KH1 has him knocked away in the air, far enough in for KH2's Aerial Recovery
    /// (`combomods::launched`).
    pub launched: bool,
    /// KH2 has something for Square on the ground: a dodge roll, Guard or Quick Run (KH1's
    /// menu: Dodge Roll, Guard, Quick Run equipped).
    pub kh2_square: bool,
    /// KH2's Quick Run is equipped.
    pub quick_run: bool,
    /// KH1's roll motion (`styleroll::ROLL_MOTION`) is playing.
    pub kh1_rolling: bool,
    /// KH1 has its own roll (its Dodge Roll equipped: bit `0x20` of `0x142D5F610`).
    pub kh1_roll: bool,
    /// The stick is pushed (past KH1's roll's 0.15).
    pub moving: bool,
    /// Where in KH2's roll its Quick Run check comes (frames).
    pub quick_run_check: f32,
    /// KH1's frame step (1.0 at 60 fps).
    pub dt: f32,
}

/// KH1's button bits for its jump (6) and its Square action (`0x100000`).
pub const KH1_JUMP_BITS: u32 = 0x6;
pub const KH1_ACTION_BIT: u32 = 0x10_0000;

/// What the hide rules last saw (the trace log): the dodge was used this fall, the dodge's jump
/// press has been let go since.
pub fn dodge_flags() -> (bool, bool) {
    (AIR_DODGED.load(Ordering::Relaxed), DODGE_JUMP_UP.load(Ordering::Relaxed))
}

/// The Aerial Dodge was used in this air time by KH2's controller in another of its moves (its
/// glide's borrow, t327): KH1's air doesn't borrow him for a second one.
pub fn mark_air_dodged() {
    AIR_DODGED.store(true, Ordering::Relaxed);
}

/// One control step of KH1's with base Sora in its states: the borrow it starts (kept for the
/// physics step), and KH1's bits it doesn't get this step.
pub fn control(s: &Seen) -> u32 {
    let was = LAST.swap((s.jump as u32) | (s.square as u32) << 1, Ordering::Relaxed);
    let jump_fresh = s.jump && was & 1 == 0;
    let square_fresh = s.square && was & 2 == 0;
    let air = matches!(s.state, 1 | 2);
    if s.state == 0 {
        AIR_DODGED.store(false, Ordering::Relaxed);
    }
    let on_square = super::settings::get().glide_on_square || s.superglide;
    if !s.jump {
        DODGE_JUMP_UP.store(true, Ordering::Relaxed);
    }
    let roll_kh2 = styles::kh2(Kind::DodgeRoll);
    // KH2's ability moves out of KH1's own moves whatever the styles (`combomods`): a Square move
    // in KH1's combo window, Aerial Recovery / Retaliating Slash while KH1 has him knocked away.
    // The press is KH2's: KH1 sees none of it.
    if let Some(hide) = super::combomods::control(s, jump_fresh, square_fresh) {
        return hide;
    }
    // KH1's roll held into KH2's Quick Run (the user: "kh1 dodge roll doesn't go into quick run
    // when holding square"): Square pressed for KH1's roll and held since, KH1's roll motion seen,
    // then KH2 takes him for its Quick Run once KH1 lets him act at or past KH2's own Quick Run
    // check in its roll (or when KH1's roll is over).
    if !s.square {
        ROLL_HELD.store(false, Ordering::Relaxed);
        ROLL_FRAMES.store(0, Ordering::Relaxed);
    } else if s.state == 0 && square_fresh && !roll_kh2 {
        ROLL_HELD.store(true, Ordering::Relaxed);
        ROLL_FRAMES.store(0, Ordering::Relaxed);
    }
    let roll_frames = f32::from_bits(ROLL_FRAMES.load(Ordering::Relaxed));
    let (roll_frames, roll_over) = if !ROLL_HELD.load(Ordering::Relaxed) {
        (0.0, false)
    } else if s.kh1_rolling {
        let n = roll_frames + s.dt.clamp(0.0, 4.0);
        ROLL_FRAMES.store(n.to_bits(), Ordering::Relaxed);
        (n, false)
    } else {
        (roll_frames, roll_frames > 0.0)
    };
    let quick_run = s.state == 0 && s.square && s.quick_run && roll_frames > 0.0 && (roll_over || roll_frames >= s.quick_run_check);
    let mut borrow = NONE;
    if s.can_act {
        if quick_run {
            ROLL_HELD.store(false, Ordering::Relaxed);
            ROLL_FRAMES.store(0, Ordering::Relaxed);
            borrow = QUICK_RUN;
        } else if s.state == 0 && square_fresh && !roll_kh2 && !s.kh1_roll && s.quick_run && s.moving {
            // (Dodge Roll at KH1 without KH1's roll equipped: Square while moving is KH2's Quick
            // Run, which has no style of its own, when it is equipped.)
            borrow = SQUARE;
        } else if s.state == 0 && square_fresh && roll_kh2 {
            // (Nothing of KH2's for Square without those abilities: KH2 would only take him and
            // give him back two frames later, s261's "kh2 dodge roll is unusable" with Dodge
            // Roll, Guard and Quick Run all taken off in KH1's menu. KH1's own roll needs its
            // Dodge Roll equipped too, its bit `0x20` of `0x142D5F610`, set in its control step
            // from `0x140282A70 (22)`.)
            if s.kh2_square {
                borrow = SQUARE;
            } else {
                static N: AtomicU32 = AtomicU32::new(0);
                if N.fetch_add(1, Ordering::Relaxed) < 10 {
                    super::log("Square with the Dodge Roll style KH2: KH2 has no dodge roll, guard or Quick Run (none of Dodge Roll, Guard, Quick Run is equipped in KH1's menu), so it doesn't take him");
                }
            }
        } else if air && jump_fresh && s.air_dodge && !AIR_DODGED.load(Ordering::Relaxed) {
            AIR_DODGED.store(true, Ordering::Relaxed);
            DODGE_JUMP_UP.store(false, Ordering::Relaxed);
            borrow = AIR_JUMP;
        } else if air && s.kh2_glide && square_fresh {
            // KH2's glide (Glide at KH2, t321): KH2's controller glides him, on Square as in the KH2
            // rewrite (t327): up or down, before or after the Aerial Dodge.
            borrow = GLIDE;
        }
    }
    if borrow != NONE {
        BORROW.store(borrow, Ordering::Relaxed);
        // (The press is KH2's: KH1 sees none of the button this step.)
        return if matches!(borrow, SQUARE | QUICK_RUN) { KH1_ACTION_BIT } else { KH1_JUMP_BITS | KH1_ACTION_BIT };
    }
    let mut hide = 0;
    // On the ground: Square is KH2's with the Dodge Roll style at KH2 (KH2's roll, guard and
    // Quick Run); KH1's own roll and guard with it at KH1.
    if s.state == 0 && roll_kh2 {
        hide |= KH1_ACTION_BIT;
    }
    // In the air past the rise: KH1's glide comes from its held jump (and the action bit its held
    // jump sets). The glide is a shared move: KH1's glide state at either style (KH2's glide
    // motions and numbers on it at KH2, `pose::follow`, `physics`). With the ini's `glide = jump`
    // KH1's own held jump glides him; on Square (`glide = square`) KH1's glide is started from
    // Square (`takeover`), so its jump isn't seen there (the user: "Sora is still able to do a
    // normal kh1 glide by pressing the jump button"). Rising, the held jump is KH1's own, for its
    // height.
    if air && !s.rising && on_square {
        hide |= KH1_JUMP_BITS | KH1_ACTION_BIT;
    }
    // With the Aerial Dodge equipped, KH1's glide (its held jump) comes only after the dodge, and on
    // a fresh press: until the dodge is used, and while the jump pressed for it is still held, KH1
    // sees no jump (t318 traces: KH1 asked for its glide on the dodge's first frame). Without the
    // dodge equipped, KH1's glide is as it was.
    // (t327: KH1's glide's rule only, at Glide KH1; KH2's glide has KH2's own rules.)
    if air && !s.rising && s.air_dodge && !styles::kh2(Kind::Glide) && (!AIR_DODGED.load(Ordering::Relaxed) || !DODGE_JUMP_UP.load(Ordering::Relaxed)) {
        hide |= KH1_JUMP_BITS | KH1_ACTION_BIT;
    }
    // Glide at KH2 (t321): KH1's own glide never, past the rise: the glide is KH2's (`GLIDE`).
    if air && !s.rising && styles::kh2(Kind::Glide) {
        hide |= KH1_JUMP_BITS | KH1_ACTION_BIT;
    }
    hide
}

/// KH2 gives him back to KH1's states: on the ground (KH1's ground state), or falling (KH1's fall
/// state).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Back {
    Ground,
    Air,
    /// KH2's air move touched down (its `Land`): KH1's fall state on the floor, so KH1 lands him
    /// itself (its landing slot, its hold, its can-act rules) instead of KH2 playing the landing.
    Land,
}

/// Frames KH2's controller has been in a state KH1 shares.
static SHARED_FRAMES: AtomicU32 = AtomicU32::new(0);
/// An air combo ended this long ago (frames since its last swing): KH1's fall from there (KH2
/// keeps a combo going until landing otherwise).
const AIR_COMBO_DONE: u32 = 24;

/// After KH2's step during one of its moves: whether its move is over and where he goes back to.
/// `combo`: KH2's combo going and frames since its last swing; `waiting`: a command or press of
/// KH2's still to run.
pub fn give_back(state: StateName, grounded: bool, falling: bool, combo: (bool, u32), waiting: bool) -> Option<Back> {
    // KH2's air move has touched down: KH1 lands him (t307: "kh1 needs to handle landing kh2
    // attacks with its own internal animation state instead of giving it back"; t306 let KH2's own
    // `Land` hold play out and gave him back at idle). At once: no wait for a second frame. A
    // command or press of KH2's still to run keeps KH2's landing, as before.
    if state == StateName::Land && grounded && !waiting {
        SHARED_FRAMES.store(0, Ordering::Relaxed);
        return Some(Back::Land);
    }
    let back = match state {
        StateName::Idle | StateName::Walk | StateName::Run if grounded && !combo.0 => Some(Back::Ground),
        StateName::Air if !grounded && falling && (!combo.0 || combo.1 >= AIR_COMBO_DONE) => Some(Back::Air),
        _ => None,
    };
    let back = back.filter(|_| !waiting);
    let n = if back.is_some() { SHARED_FRAMES.fetch_add(1, Ordering::Relaxed) + 1 } else { SHARED_FRAMES.store(0, Ordering::Relaxed); 0 };
    // (Two frames in it: a move's own one-frame pass through idle doesn't end it.)
    back.filter(|_| n >= 2).inspect(|_| SHARED_FRAMES.store(0, Ordering::Relaxed))
}

/// Whose physics move base Sora: the physics setting (`kh1phys::Rule`, the ini's `physics = kh1 |
/// kh2`, the Config menu's Physics row; the move styles choose the animations only). The user:
/// "if its kh1 then all the animations (vanilla, and our kh2 animations ...) will run with kh1s
/// physics (with exceptions to drive forms) if its kh2 then all the animations (including vanilla
/// animations) will run with kh2s physics instead". Drive Forms are KH2's either way.
///
/// **KH2 on KH1's own states.** KH1's state table (`0x142DB4240`, 0x40 bytes a state: `+0x18`
/// pre-step `(actor, input)`, `+0x20` step `(actor, &displacement)`, `+0x28` transition) is
/// writable: the steps below are wrapped there for Sora, KH1's own step running first. With
/// physics KH2, whatever the move's style:
///
/// - Walk and run: the ground step's displacement (`0x1402AF6C0`: the move direction on the floor
///   × `+0x2C`, which is KH1's walk 2.0 or run 8.0 from the stats' `+4` / `+8`, by the stick past
///   0.8) is scaled to KH2's walk or run speed. KH1's choice of walk or run (and so its animation)
///   is untouched.
/// - Jump, High Jump, falls: KH1's jump (its takeoff, state 1's pre-step `0x1402ADC10`, then its
///   rise and fall in state 2's step `0x1402AD510`) follows KH2's arc: KH2's jump or High Jump
///   height and gravity, its early-release hop (after 6 frames of the rise, more than 5 before the
///   apex), its terminal fall speed, its air steering. KH1's own arc fields (`+0x3B0` air time,
///   `+0x3B4` start height, `+0x3B8` time to the apex, `+0x3BC` the height) and air speed `+0x3C0`
///   are written from it every frame, so KH1's apex animation, its air-move gating and its own air
///   moves go on from KH2's motion; KH1's own early cut (flag `0x40000`) is kept off. Landing
///   stays KH1's (its collision).
/// - KH1's own air moves (its air attacks, casts, a blow's launch: no KH2 numbers of their own):
///   their own lunge, speed and hop height, on KH2's gravity, fall and drift (`air_step`).
/// - The ground combo's swings (slots 98..101, its action flag 4 up; KH1's or KH2's animation):
///   moved by the root travel of KH2's swing for that slot (`pose::swing_motion`), stretched over
///   KH1's swing, along his facing, in place of KH1's own travel. KH1's hit objects, timing and
///   damage stay.
/// - Not the glide (t321): KH1's glide state (0x15) is KH1's own with its own numbers at either
///   physics setting, and at Glide KH2 the glide is KH2's controller's (a borrow, `GLIDE`).
/// - KH1's other ground actions (its roll, an item's use, a ground cast, its specials) keep their
///   own travel: no KH2 numbers for them.
///
/// **KH1 on KH2's controller.** With physics KH1 none of the above runs (KH1's states are KH1's),
/// and KH2's controller takes KH1's numbers while it has base Sora (`kh2_controller`).
///
/// KH1's own physics fields are where momentum lives, whoever moves him (`docs/kh1-physics.md`,
/// `crate::kh1phys`): KH2's numbers on KH1's states are written into them (`air_step` keeps KH1's
/// arc on KH2's), and while KH2's controller has him each of its steps is written into them
/// (`mirror`), so KH1's air state takes him from there at KH2's speed (`kh1_takes_air`) and KH2
/// takes him from KH1's at KH1's (`kh1_velocity`).
pub mod physics {
    use super::super::addr::Rva;
    use crate::kh1phys::{self, Motion};
    use character_core::{Arc, Character, Output};
    use std::sync::atomic::{AtomicU32, AtomicU8, AtomicUsize, Ordering};
    use std::sync::Mutex;

    pub use crate::kh1phys::Rule;

    /// The physics setting (the ini's `physics = kh1 | kh2`, KH2's without it; the Config menu's
    /// Physics row): 0 not read yet, else 1 + `Rule`.
    static RULE: AtomicU8 = AtomicU8::new(0);

    fn store(r: Rule) {
        RULE.store(1 + (r == Rule::Kh2) as u8, Ordering::Relaxed);
    }

    /// The physics setting now.
    pub fn rule() -> Rule {
        match RULE.load(Ordering::Relaxed) {
            0 => {
                let r = super::super::settings::get().physics_rule;
                store(r);
                super::super::log(&format!("physics: {} for every move of base Sora's (the ini's `physics`; Drive Forms KH2's)", r.key().to_uppercase()));
                r
            }
            1 => Rule::Kh1,
            _ => Rule::Kh2,
        }
    }

    /// Switches it (the Config menu's Physics row): live, kept in the ini.
    pub fn set_rule(r: Rule) {
        let was = rule();
        store(r);
        super::super::settings::save_physics(r);
        super::super::log(&format!("Config menu: physics {} (was {}): {}", r.key().to_uppercase(), was.key().to_uppercase(), match r {
            Rule::Kh1 => "KH1's numbers for every move of base Sora's, KH2's borrowed moves too (KH2's controller given KH1's gravity, fall, jump, speeds and air steering)",
            Rule::Kh2 => "KH2's numbers for every move of base Sora's, KH1's own moves on its states too",
        }));
    }

    /// Whose physics numbers move base Sora now, whatever the move and its style (the styles choose
    /// the animations only): the setting, and KH2's in a Drive Form (its own moveset all through).
    /// The one switch point: KH1's own states take KH2's numbers where this is KH2's (the wrappers
    /// below), KH2's controller takes KH1's where it is KH1's (`kh2_controller`).
    pub fn source() -> Rule {
        rule().source(super::super::form::in_form())
    }

    const TABLE_RVA: Rva = Rva(0x2DB4240);
    /// KH1's frame scale (1.0 at 60 fps).
    const DT_RVA: Rva = Rva(0x23405DC);
    const PARTY_RVA: Rva = Rva(0x2D37C80);
    const ENTRY: usize = 0x40;
    const PRE_STEP: usize = 0x18;
    const STEP: usize = 0x20;
    /// KH1's arc's start height, its time to the apex and its height (negative: KH1 is y-down).
    const ARC_T: usize = 0x3B0;
    const ARC_Y0: usize = 0x3B4;
    const ARC_UP_T: usize = 0x3B8;
    const ARC_H: usize = 0x3BC;
    /// KH1's early-release cut is still open (its button handler, `0x1402A59A0`).
    const CUT_OPEN: u32 = 0x4_0000;

    static ORIG_GROUND: AtomicUsize = AtomicUsize::new(0);
    static ORIG_AIR: AtomicUsize = AtomicUsize::new(0);
    static ORIG_TAKEOFF: AtomicUsize = AtomicUsize::new(0);
    /// KH1's ground swing now (its slot, and KH1 frames into it: KH1's swings alternate slots,
    /// 98 / 99, so a new slot is a new swing), for KH2's swing travel.
    static SWING: Mutex<Option<(usize, f32)>> = Mutex::new(None);

    /// KH2's movement numbers with Sora's abilities now (`takeover::refresh_kh2_numbers`).
    #[derive(Clone, Copy, Debug)]
    pub struct Numbers {
        pub walk: f32,
        pub run: f32,
        pub jump: f32,
        pub high_jump: f32,
        pub gravity: f32,
        pub fall_max: f32,
        pub air_keep: f32,
        pub air_drag: f32,
    }
    static NUMBERS: Mutex<Option<Numbers>> = Mutex::new(None);

    pub fn set_numbers(n: Numbers) {
        if let Ok(mut g) = NUMBERS.try_lock() {
            *g = Some(n);
        }
    }

    fn numbers() -> Option<Numbers> {
        NUMBERS.try_lock().ok().and_then(|g| *g).filter(|n| n.gravity > 0.0)
    }

    /// KH2's arc for KH1's jump or fall now, the height KH1 reached on its last step and the rise
    /// asked for then (a ceiling: less than half of it happened), and the height KH1's arc fields
    /// were written for (`air_step`: moved to where he is, if a floor, a ceiling or a push had him
    /// elsewhere, before KH1's own step reads them).
    struct Air {
        arc: Arc,
        last_y: f32,
        asked_up: f32,
        written: Option<(f32, kh1phys::Arc)>,
    }
    static AIR: Mutex<Option<Air>> = Mutex::new(None);

    /// Not in KH1's air state (or KH2 has him): no arc of ours.
    pub fn off_air() {
        if let Ok(mut a) = AIR.try_lock() {
            *a = None;
        }
    }

    unsafe fn rd<T: Copy>(p: usize) -> T {
        std::ptr::read_unaligned(p as *const T)
    }

    unsafe fn wr<T: Copy>(p: usize, v: T) {
        std::ptr::write_unaligned(p as *mut T, v)
    }

    fn base() -> usize {
        super::super::takeover::game_base()
    }

    unsafe fn is_sora(actor: usize) -> bool {
        actor > 0x10000 && actor == rd::<u64>(base() + PARTY_RVA) as usize
    }

    /// KH2's physics apply to KH1's states now (KH1's own states have base Sora, the physics
    /// setting is KH2's).
    fn kh2() -> bool {
        super::active() && source() == Rule::Kh2 && !super::super::form::in_form()
    }

    /// KH1's sideways velocity (`+0x3C0`: x, y, z, w = its length, per 60 Hz frame) and its
    /// vertical step this frame (`+0x3C4`).
    const VEL: usize = 0x3C0;
    const VEL_Y: usize = 0x3C4;
    /// KH1's flag that holds an actor still in its air state (its arc started over each frame).
    const AIR_HELD: u32 = 0x1000;

    /// KH1's motion fields on an actor (`kh1phys::Motion`).
    pub unsafe fn kh1_motion(actor: usize) -> Motion {
        let v: [f32; 4] = rd(actor + VEL);
        let arc = kh1phys::Arc { t: rd(actor + ARC_T), y0: rd(actor + ARC_Y0), up_t: rd(actor + ARC_UP_T), h: rd(actor + ARC_H) };
        Motion { h: [v[0], v[2]], arc }
    }

    /// Writes them: `dt` the frame scale its vertical step `+0x3C4` is for (KH1 writes the frame's).
    pub unsafe fn set_kh1_motion(actor: usize, m: &Motion, dt: f32) {
        let a = m.arc;
        if ![a.t, a.y0, a.up_t, a.h, m.h[0], m.h[1]].iter().all(|x| x.is_finite()) {
            return;
        }
        wr(actor + VEL, [m.h[0], 0.0f32, m.h[1], m.h[0].hypot(m.h[1])]);
        wr(actor + VEL_Y, -a.up_speed() * dt);
        wr(actor + ARC_T, a.t);
        wr(actor + ARC_Y0, a.y0);
        wr(actor + ARC_UP_T, a.up_t);
        wr(actor + ARC_H, a.h);
    }

    /// KH2's motion last written into KH1's fields (`mirror`), KH1's height then and the game
    /// frame (`takeover::GAME_FRAME`).
    static LAST: Mutex<Option<(Motion, f32, u64)>> = Mutex::new(None);

    fn game_frame() -> u64 {
        super::super::takeover::GAME_FRAME.load(Ordering::Relaxed)
    }

    /// After each of KH2's controller's steps while it has him (`takeover`, every 60 Hz step at 30
    /// fps too): its motion written into KH1's own fields. Sideways: the step's travel (a 60 Hz
    /// step's, so its velocity, an attack's root motion included); up: KH2's arc going on from
    /// where he is (`kh1phys::from_kh2_step`), or nothing on the ground. KH1's physics step doesn't
    /// run on KH2's frames (`takeover` has moved him), so nothing is applied twice; the fields are
    /// what KH1's air state goes on from when it takes him (`kh1_takes_air`).
    pub unsafe fn mirror(actor: usize, ch: &Character, out: &Output, grounded: bool) {
        let y: f32 = rd(actor + 0x14);
        if !y.is_finite() {
            return;
        }
        let p = &ch.params;
        let m = if grounded {
            Motion { h: kh1phys::carried([out.displacement[0], out.displacement[2]]), arc: kh1phys::Arc::rest(y) }
        } else {
            kh1phys::from_kh2_step(ch.air_arc(), out.displacement, y, p.gravity, p.fall_max)
        };
        set_kh1_motion(actor, &m, 1.0);
        if let Ok(mut l) = LAST.try_lock() {
            *l = Some((m, y, game_frame()));
        }
    }

    /// KH1's air state takes him from KH2's controller in the air (KH1's air command, KH2's move
    /// given back as a fall; KH1's state already set to 2): KH1's fields with KH2's momentum, the
    /// last `mirror` moved to where he is. (Before, the arc was zeroed here: KH1 started him
    /// falling from rest, the user's "no momentum during his kh1 move".) Returns that motion.
    pub unsafe fn kh1_takes_air(actor: usize) -> Motion {
        let y: f32 = rd(actor + 0x14);
        let last = LAST.try_lock().ok().and_then(|l| *l).filter(|(_, _, f)| game_frame().saturating_sub(*f) <= 2);
        let fresh = last.is_some();
        let m = match last {
            Some((mut m, then, _)) => {
                m.arc.y0 += y - then;
                m
            }
            None => Motion { h: kh1phys::carried(super::super::takeover::kh2_sideways()), arc: kh1phys::Arc::rest(y) },
        };
        set_kh1_motion(actor, &m, dt());
        off_air();
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 40 {
            let (h, up) = m.velocity();
            super::super::log(&format!(
                "KH1's air state takes him with KH2's momentum{}: sideways {:.1}/frame, up {up:.1}/frame (KH1's arc: time {:.1}, apex at {:.1}, {:.0} higher)",
                if fresh { "" } else { " (no KH2 step written lately: from rest, KH2's sideways speed)" },
                h[0].hypot(h[1]),
                m.arc.t,
                m.arc.up_t,
                m.arc.rise_left()
            ));
        }
        m
    }

    /// The last game frame KH2's arc on KH1's air state was past its apex (`air_step`), 0 none.
    static ARC_FALL_FRAME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    /// KH2's arc on KH1's air state is falling this frame (or was last frame): KH2's jump clip has
    /// handed over to its fall loop, as KH2's own does (`kh2_rig`: the jump plays while rising).
    pub fn kh2_arc_falling() -> bool {
        let f = ARC_FALL_FRAME.load(Ordering::Relaxed);
        f != 0 && game_frame().saturating_sub(f) <= 1
    }

    /// KH1's aerial combo swings' own lunge (their starting sideways speed, units a 60 Hz frame,
    /// along his facing), as KH1's own combo starts them: the same in every room, with or without
    /// a target or the stick (the user's traces t299: 7.80, 3.90, 11.70, each slowed by
    /// `LUNGE_DECAY` a frame).
    const LUNGES: [(u32, f32); 3] = [(0xCC, 7.8), (0xCD, 3.9), (0xCE, 11.7)];
    const LUNGE_DECAY: f32 = 0.975;
    /// The game frame a KH1 air attack was started out of KH2's move (`combat::command`), 0 none.
    static LUNGE_FROM: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    /// A KH1 air attack was just started out of KH2's air move: its lunge to be seeded once it
    /// plays (`seed_kh1_lunge`).
    pub fn kh1_air_attack_from_kh2() {
        LUNGE_FROM.store(game_frame().max(1), Ordering::Relaxed);
    }

    /// After KH1's physics frame (`takeover::physics`): a KH1 air attack started out of KH2's air
    /// move went on with no sideways speed at all (the user: "this bug also appears any time a kh2
    /// air attack transitions into kh1 attack"; the traces: KH1's start left `+0x3C0` at 0, the
    /// finisher only rose and fell where it began, where KH1's own combo sends it ~280 forward).
    /// On its first frame of a KH1 aerial swing it gets that swing's own lunge along his facing, as
    /// far into its slowing as the swing is (`ARC_T`), so it travels as in KH1's own combo.
    pub unsafe fn seed_kh1_lunge(actor: usize) {
        let from = LUNGE_FROM.load(Ordering::Relaxed);
        if from == 0 || !is_sora(actor) {
            return;
        }
        if game_frame().saturating_sub(from) > 4 || rd::<i32>(actor + 0x70) != 2 {
            LUNGE_FROM.store(0, Ordering::Relaxed);
            return;
        }
        let motion: u32 = rd(actor + 0x164);
        let Some(&(_, speed)) = LUNGES.iter().find(|(m, _)| *m == motion) else { return };
        LUNGE_FROM.store(0, Ordering::Relaxed);
        let v: [f32; 4] = rd(actor + VEL);
        let had = v[0].hypot(v[2]);
        let t: f32 = rd(actor + ARC_T);
        let t = if t.is_finite() { t } else { 1.0 };
        let s = speed * LUNGE_DECAY.powf((t - 1.0).clamp(0.0, 30.0));
        let set = had < 0.5;
        if set {
            let yaw: f32 = rd(actor + 0x3C);
            if !yaw.is_finite() {
                return;
            }
            wr(actor + VEL, [yaw.sin() * s, v[1], yaw.cos() * s, s]);
        }
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 40 {
            super::super::log(&format!(
                "KH1's air attack (motion {motion:#x}) out of KH2's move: {}",
                if set { format!("its own lunge {s:.1}/frame given (KH1 left it {had:.1}; {speed:.1} at its start, frame {t:.0})") } else { format!("it has {had:.1}/frame already, left as it is") }
            ));
        }
    }

    /// One of KH1's own air moves just started from `kh1_takes_air`'s motion `before` (KH1's
    /// command step ran it): an air attack's start sets its own sideways lunge and hop in place of
    /// what was there (`0x1402A4760`); it keeps the faster speed and the higher rise
    /// (`kh1phys::keep_momentum`). A spell keeps KH1's own rule (half the sideways speed,
    /// `0x1402A5380`) and the carried arc, which KH1's cast leaves alone.
    pub unsafe fn kh1_air_attack_started(actor: usize, before: &Motion, keep_rise: bool) {
        if rd::<i32>(actor + 0x70) != 2 {
            return;
        }
        let now = kh1_motion(actor);
        if now == *before {
            // (No attack started: KH2's own took the press, or KH1 refused it.)
            return;
        }
        // Out of KH2's jump arc on KH1's air state the swing's own hop replaces the jump's rise, as
        // KH2's own air swing does (the t312 trace: a swing at the jump's start kept ~235 of rise and
        // sent him to 214 up); the faster sideways speed is still kept. Out of any KH2 air state
        // (t332: the jump's take-off and rise, the Aerial Dodge, an air swing or cast) it is the same:
        // `keep_rise` is true for no caller now, kept for a grounded KH2 attack later.
        let kept = {
            let k = kh1phys::keep_momentum(before, &now);
            if keep_rise { k } else { Motion { h: k.h, arc: now.arc } }
        };
        if kept != now {
            set_kh1_motion(actor, &kept, dt());
        }
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 40 {
            let s = |v: [f32; 2]| v[0].hypot(v[1]);
            super::super::log(&format!(
                "KH1's air attack out of KH2's motion: its own lunge {:.1}/frame and hop {:.0}; KH2's speed {:.1}/frame and rise {:.0} left; kept {:.1}/frame and {}",
                s(now.h),
                now.arc.rise_left(),
                s(before.h),
                before.arc.rise_left(),
                s(kept.h),
                if kept.arc == now.arc { "KH1's hop" } else { "KH2's rise" }
            ));
        }
    }

    /// KH2's arc has KH1's air state now (physics KH2, `air_step`): its motion as KH1's fields hold
    /// it, for a KH1 air attack started from it (`kh1_air_attack_started`, `combat::command`).
    pub unsafe fn kh2_air_motion(actor: usize) -> Option<Motion> {
        let ours = is_sora(actor) && rd::<i32>(actor + 0x70) == 2 && kh2() && AIR.try_lock().is_ok_and(|g| g.is_some());
        ours.then(|| kh1_motion(actor))
    }

    /// His velocity in KH1's own air state, per 60 Hz frame (sideways, up), for KH2's controller
    /// taking him from KH1's states: KH1's fields, not his travel over the game frame (two 60 Hz
    /// frames' worth at 30 fps). None outside state 2 (KH1's other states keep none there).
    pub unsafe fn kh1_velocity(actor: usize) -> Option<([f32; 2], f32)> {
        if rd::<i32>(actor + 0x70) != 2 || rd::<u32>(actor) & AIR_HELD != 0 {
            return None;
        }
        let (h, up) = kh1_motion(actor).velocity();
        (h.iter().all(|x| x.is_finite()) && up.is_finite()).then(|| (kh1phys::carried(h), up.clamp(-30.0, 30.0)))
    }

    /// KH1's own movement numbers on base Sora's stats block (`+0x6C`, `kh1phys::Stats`).
    unsafe fn kh1_stats(actor: usize) -> kh1phys::Stats {
        let stats = super::super::pose::ptr(rd::<u32>(actor + 0x6C));
        if stats < 0x10000 {
            return kh1phys::Stats::default();
        }
        let s = |o: usize| rd::<f32>(stats + o);
        kh1phys::Stats::read(s(4), s(8), s(0x10), s(0x14), s(0x18))
    }

    /// KH2's controller's numbers for base Sora's next step (each frame it has him, `takeover`):
    /// with physics KH1 and no Drive Form, KH1's (`kh1phys::kh1_params`, `kh1_growth`: KH1's
    /// gravity and fall, its jump for the jump and High Jump, its walk and run, its air steering
    /// and drag, from his stats now); else the moveset's own (`template`: KH2 Sora's, or the
    /// form's). So KH2's borrowed moves (Aerial Dodge, KH2's air combo, its roll, Quick Run, its
    /// spells) and the `kh1_slots` moves it runs move on KH1's numbers; what they bring themselves
    /// (Aerial Dodge's height and speed, Quick Run's speed and brake, root motion, an attack row's
    /// hop and lunge) stays theirs, under KH1's gravity and fall (`docs/kh1-physics.md` §7).
    pub unsafe fn kh2_controller(actor: usize, ch: &mut Character, template: &Character) {
        let stats = (source() == Rule::Kh1).then(|| kh1_stats(actor));
        let p = match &stats {
            Some(s) => kh1phys::kh1_params(&template.params, s),
            None => template.params.clone(),
        };
        if ch.params != p {
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 20 {
                super::super::log(&format!(
                    "KH2's controller on {} numbers: gravity {:.3}, fall {:.0}, jump {:.0}, walk {:.1}, run {:.1} (past {:.2} tilt), air {}",
                    if stats.is_some() { "KH1's" } else { "its own (KH2's)" },
                    p.gravity,
                    p.fall_max,
                    p.jump_height,
                    p.walk_speed,
                    p.run_speed,
                    p.run_threshold,
                    if p.air_accel > 0.0 { format!("steering +{:.3} x speed a frame, drag {:.3} a tick", p.air_accel, p.air_drag) } else { format!("keep {:.3}, drag {:.3} a tick", p.air_keep, p.air_drag) }
                ));
                if let Some(s) = &stats {
                    if s.air_accel.is_none() || s.air_drag.is_none() || s.jump.is_none() {
                        super::super::log(&format!("(KH1's stats not all read: {s:?}; those stay KH2's)"));
                    }
                }
            }
            ch.params = p;
        }
        let jump = stats.and_then(|s| s.jump);
        for (l, t) in ch.growth.iter_mut().zip(&template.growth) {
            l.high_jump_height = jump.unwrap_or(t.high_jump_height);
        }
    }

    /// Puts our wrappers in KH1's state table (each Sora physics frame: KH1 fills the table itself
    /// when it sets its states up, which would drop them).
    pub unsafe fn install(base: usize) {
        let table = base + TABLE_RVA;
        // (KH1's glide state, 0x15, is no longer wrapped, t321: KH2's numbers on it at physics KH2
        // are gone; the glide follows its style only.)
        let slots: [(usize, &AtomicUsize, usize); 3] = [
            (table + STEP, &ORIG_GROUND, ground_step as *const () as usize),
            (table + 2 * ENTRY + STEP, &ORIG_AIR, air_step as *const () as usize),
            (table + ENTRY + PRE_STEP, &ORIG_TAKEOFF, takeoff as *const () as usize),
        ];
        for (slot, orig, ours) in slots {
            let now: usize = rd(slot);
            if now != ours && now > 0x10000 {
                orig.store(now, Ordering::Release);
                wr(slot, ours);
                super::super::log(&format!("shared states: KH1's state step at {slot:#x} wrapped ({now:#x})"));
            }
        }
    }

    type StepFn = unsafe extern "system" fn(usize, *mut [f32; 4]) -> usize;

    /// KH1's frame step now (1.0 at 60 fps).
    pub fn dt() -> f32 {
        let dt: f32 = unsafe { rd(base() + DT_RVA) };
        if dt.is_finite() && dt > 0.0 && dt < 4.0 {
            dt
        } else {
            1.0
        }
    }

    /// State 0's step: KH2's walk or run speed (physics KH2); KH2's swing travel in
    /// KH1's ground combo (physics KH2, either swing animation).
    unsafe extern "system" fn ground_step(actor: usize, disp: *mut [f32; 4]) -> usize {
        let f: StepFn = std::mem::transmute(ORIG_GROUND.load(Ordering::Acquire));
        let r = f(actor, disp);
        if !is_sora(actor) || rd::<i32>(actor + 0x70) != 0 || disp.is_null() {
            return r;
        }
        let slot = super::super::pose::sora_slot();
        if matches!(slot, Some(98..=101)) && rd::<u32>(actor) & 4 != 0 {
            if kh2() {
                swing_travel(actor, &mut *disp, slot.unwrap_or(0));
            }
            return r;
        }
        if let Ok(mut s) = SWING.try_lock() {
            *s = None;
        }
        // (KH1's walk and run only: not one of its actions, flag 4, an item's use.)
        if !kh2() || rd::<u32>(actor) & 4 != 0 {
            return r;
        }
        let Some(n) = numbers() else { return r };
        let speed: f32 = rd(actor + 0x2C);
        let stats = super::super::pose::ptr(rd::<u32>(actor + 0x6C));
        if speed <= 0.01 || stats < 0x10000 {
            return r;
        }
        let kh1_run: f32 = rd(stats + 8);
        let kh2 = if speed >= kh1_run - 0.01 { n.run } else { n.walk };
        let k = kh2 / speed;
        if k.is_finite() && k > 0.0 {
            let d = &mut *disp;
            d[0] *= k;
            d[2] *= k;
        }
        r
    }

    /// KH1's ground swing (its slot `slot`) moved as KH2's swing for it moves him: KH2's clip's
    /// root travel (`Clip::root`, +Z forward, +X right), stretched over KH1's swing as its motion
    /// is (`pose::follow`), along his facing. KH1's own travel this frame (its swing's step
    /// forward, its dash at a target) is replaced: one owner state (KH1's swing, its hit objects
    /// and timing), KH2's physics on it.
    unsafe fn swing_travel(actor: usize, d: &mut [f32; 4], slot: usize) {
        let Some(kh2) = super::super::pose::kh2() else { return };
        let Some(motion) = super::super::pose::swing_motion(slot) else { return };
        let Some(clip) = kh2.character.moveset.clips.get(&motion) else { return };
        let kh1_len = super::super::pose::slot_frames(slot).unwrap_or(0) as f32;
        if clip.root.len() < 2 || kh1_len < 1.0 {
            return;
        }
        let Ok(mut g) = SWING.try_lock() else { return };
        let s = match *g {
            Some((s, t)) if s == slot => (s, t),
            _ => (slot, 0.0),
        };
        let dt = dt();
        let k = clip.length.max(1) as f32 / kh1_len;
        let at = |t: f32| -> [f32; 3] {
            let u = (t * k).clamp(0.0, (clip.root.len() - 1) as f32);
            let i = (u as usize).min(clip.root.len() - 2);
            let w = u - i as f32;
            let (a, b) = (clip.root[i], clip.root[i + 1]);
            [a[0] + (b[0] - a[0]) * w, a[1] + (b[1] - a[1]) * w, a[2] + (b[2] - a[2]) * w]
        };
        let (a, b) = (at(s.1), at(s.1 + dt));
        *g = Some((s.0, s.1 + dt));
        let local = [b[0] - a[0], b[2] - a[2]];
        let yaw: f32 = rd(actor + 0x3C);
        if !yaw.is_finite() || !local.iter().all(|x| x.is_finite()) {
            return;
        }
        // (KH2's world turned by his facing as the controller does, `fo.root`; KH1's x / z are
        // KH2's.)
        let fw = [yaw.sin(), yaw.cos()];
        let right = [fw[1], -fw[0]];
        d[0] = local[0] * right[0] + local[1] * fw[0];
        d[2] = local[0] * right[1] + local[1] * fw[1];
    }

    type PreFn = unsafe extern "system" fn(usize, usize) -> usize;

    /// State 1's pre-step (KH1's takeoff): when it launches him (state 2), KH2's jump arc.
    unsafe extern "system" fn takeoff(actor: usize, input: usize) -> usize {
        let f: PreFn = std::mem::transmute(ORIG_TAKEOFF.load(Ordering::Acquire));
        let r = f(actor, input);
        if !is_sora(actor) || rd::<i32>(actor + 0x70) != 2 || !kh2() {
            return r;
        }
        let Some(n) = numbers() else { return r };
        // (KH1's High Jump: its landing-motion base 5, `0x1402AD860`.)
        let high = rd::<u32>(actor + 0x3D0) == 5;
        let h = if high { n.high_jump } else { n.jump };
        let mut arc = Arc::new(h, n.gravity);
        arc.holdable = true;
        wr(actor + ARC_UP_T, arc.uptime);
        wr(actor + ARC_H, -h);
        // (KH1's takeoff set its air time 0 and start height here: with these, KH1's arc is KH2's.)
        if let Ok(mut a) = AIR.lock() {
            *a = Some(Air { arc, last_y: rd(actor + 0x14), asked_up: 0.0, written: None });
        }
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 20 {
            let stats = super::super::pose::ptr(rd::<u32>(actor + 0x6C));
            // (KH1's own air numbers too, its stats `+0x10` jump height, `+0x14` air acceleration,
            // `+0x18` air drag a 30 Hz frame: `docs/kh1-physics.md`.)
            let s = |o: usize| if stats > 0x10000 { rd::<f32>(stats + o) } else { 0.0 };
            super::super::log(&format!(
                "KH1's {} with KH2's arc: {h:.0} high, {:.1} frames up (KH1's own: {:.0}; its air acceleration {:.3}, drag {:.3})",
                if high { "High Jump" } else { "jump" },
                arc.uptime,
                s(0x10),
                s(0x14),
                s(0x18)
            ));
        }
        r
    }

    /// State 2's step (KH1's rise and fall, and KH1's own air moves in it): KH2's arc and air
    /// drift (physics KH2), written into KH1's own arc and air speed each frame, so that KH1's own
    /// moves start from KH2's motion and KH2's arc picks up from KH1's when it has him again.
    ///
    /// KH1's own air moves (flag 4: its air attacks, its casts, a blow's launch) have no KH2
    /// numbers: what they set up themselves stays theirs (the attack's lunge, its sideways speed
    /// `+0x3C0`; the cast's halved speed; their hop's height), and KH2's nearest numbers move it
    /// (`docs/kh1-physics.md` §7): the hop as KH2's hop of that height (`kh1phys::kh1_arc_on_kh2`),
    /// KH2's gravity and fall cap, and KH2's air drift without the stick (`air_drag`, as KH2's own
    /// air attacks outside an allow-movement range). KH2's early release doesn't cut them.
    unsafe extern "system" fn air_step(actor: usize, disp: *mut [f32; 4]) -> usize {
        let f: StepFn = std::mem::transmute(ORIG_AIR.load(Ordering::Acquire));
        let sora = is_sora(actor);
        let v0: [f32; 4] = if sora { rd(actor + 0x3C0) } else { [0.0; 4] };
        let ours = sora && rd::<i32>(actor + 0x70) == 2 && !disp.is_null() && kh2();
        // KH1 set its fields itself since our last write (an air attack's hop from its command
        // step, a blow's launch): KH2's arc goes on from KH1's from here.
        let mut kh1_set = false;
        if ours {
            // KH1's arc written last frame for a height a floor, a ceiling or a push didn't leave
            // him at: moved to where he is (the same speed from here).
            if let Ok(mut g) = AIR.try_lock() {
                if let Some(a) = g.as_mut() {
                    // (Only while they are still ours: KH1's own air attack this frame, started in its
                    // command step, has set its hop there from where he is.)
                    if let Some((then, arc)) = a.written.take() {
                        let y: f32 = rd(actor + 0x14);
                        if kh1_motion(actor).arc != arc {
                            kh1_set = true;
                        } else if y.is_finite() && (y - then).abs() > 1e-3 {
                            wr(actor + ARC_Y0, arc.y0 + (y - then));
                        }
                    }
                }
            }
        }
        // (KH1's own arc before its step: where KH2's goes on from if KH1 moved him last.)
        let before = if ours { Some(kh1_motion(actor)) } else { None };
        let r = f(actor, disp);
        if !sora || rd::<i32>(actor + 0x70) != 2 || disp.is_null() || !kh2() {
            return r;
        }
        let Some(n) = numbers() else { return r };
        let Ok(mut g) = AIR.try_lock() else { return r };
        let flags = rd::<u32>(actor);
        // KH1 holding him in its air state (flag 0x1000: its arc started over each frame, no move):
        // KH1's.
        if flags & AIR_HELD != 0 {
            *g = None;
            return r;
        }
        // KH1's own move in the air (an air attack, a cast, a blow), flag 4.
        let action = flags & 4 != 0;
        let dt: f32 = rd(base() + DT_RVA);
        let dt = if dt.is_finite() && dt > 0.0 && dt < 4.0 { dt } else { 1.0 };
        let y: f32 = rd(actor + 0x14);
        if kh1_set {
            if let (Some(a), Some(b)) = (g.as_mut(), before) {
                // Not during a plain jump's rise of ours (holdable, no action flag): there KH1's
                // jump rewrites its own arc fields between our writes (every other frame, the logs:
                // "KH1's own air move (no action flag) set its arc" at f241, 243, 245, ... of one
                // High Jump), and taking that as one of KH1's moves started the rise over each
                // time, so KH2's early release never came (it needs 6 frames of the rise, and the
                // arc taken from KH1's lost `holdable`): the user's trace t302, the jump went to
                // its full 235 with the button let go 9 frames in.
                if !action && a.arc.holdable && a.arc.rising() {
                    static KEPT: AtomicU32 = AtomicU32::new(0);
                    if KEPT.fetch_add(1, Ordering::Relaxed) < 5 {
                        super::super::log("KH1 rewrote its arc in a plain jump's rise: KH2's arc kept (its early release still open)");
                    }
                } else {
                    a.arc = kh1phys::kh1_arc_on_kh2(&b.arc, n.gravity);
                    a.asked_up = 0.0;
                    static N: AtomicU32 = AtomicU32::new(0);
                    if N.fetch_add(1, Ordering::Relaxed) < 20 {
                        super::super::log(&format!("KH1's own air move{} set its arc: on KH2's numbers from here ({})", if action { "" } else { " (no action flag)" }, if a.arc.rising() { format!("a rise of {:.0} over {:.1} frames", a.arc.height - a.arc.y(a.arc.t, n.gravity), a.arc.uptime - a.arc.t) } else { format!("falling at {:.1}/frame", n.gravity * a.arc.t) }));
                    }
                }
            }
        }
        // A fall KH1 started itself (off a ledge, out of a ladder) or one of KH1's own air moves
        // out of something KH2's arc wasn't on: KH2's arc from KH1's (its rise exactly, its fall
        // at its speed, a fresh hop as KH2's hop), not from rest.
        let air = g.get_or_insert_with(|| {
            let arc = before.map_or_else(Arc::fall, |m| kh1phys::kh1_arc_on_kh2(&m.arc, n.gravity));
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 20 {
                super::super::log(&format!("KH2's arc on KH1's air state from KH1's own: {}", if arc.rising() { format!("rising, {:.0} to go", arc.height - arc.y(arc.t, n.gravity)) } else { format!("falling at {:.1}/frame", n.gravity * arc.t) }));
            }
            Air { arc, last_y: y, asked_up: 0.0, written: None }
        });
        if action {
            // (KH2's early release is the jump's: not in a move.)
            air.arc.holdable = false;
        }
        // A ceiling stopped the rise (less than half of it happened): falling from here.
        if air.asked_up > 0.5 && (air.last_y - y) < air.asked_up * 0.49 {
            air.arc = Arc::fall();
        }
        // KH2's early release (its own rule; KH1's cut kept off).
        wr(actor, rd::<u32>(actor) & !CUT_OPEN);
        if air.arc.holdable && air.arc.rising() {
            let held = super::super::takeover::buttons() & super::super::takeover::jump_button() != 0;
            if !held && air.arc.t > character_core::HOLD_MIN_FRAMES && air.arc.t < air.arc.uptime - character_core::HOLD_APEX_MARGIN {
                let hop = character_core::release_hop(n.gravity);
                air.arc = Arc::new(hop, n.gravity);
            } else if air.arc.t >= air.arc.uptime - character_core::HOLD_APEX_MARGIN {
                air.arc.holdable = false;
            }
        }
        let up = air.arc.advance(dt, n.gravity, n.fall_max);
        // Past the arc's apex (a released jump's hop included): KH2's fall loop shows (`pose`).
        if !air.arc.rising() {
            ARC_FALL_FRAME.store(game_frame().max(1), Ordering::Relaxed);
        }
        let d = &mut *disp;
        d[1] = -up;
        air.last_y = y;
        air.asked_up = up;
        // KH1's arc is KH2's from here (its rise the same curve; past KH2's apex, KH1's fall at
        // KH2's speed, so its fall animation, its air-move gates and its grabs see KH2's apex).
        let kh1_arc = kh1phys::Arc::from_kh2(&air.arc, y + d[1], n.gravity, n.fall_max);
        air.written = Some((y + d[1], kh1_arc));
        // Steering: KH2's (`air_keep` toward the stick at KH2's walk or run speed, `air_drag`
        // without it, or in one of KH1's own moves), from the speed before KH1's own steering this
        // frame. (Both are per KH2 physics tick, two 60 Hz frames: `dt / 2` of them a game frame,
        // one at 30 fps. Earlier builds took them per 60 Hz frame: twice KH2's.)
        let per = |k: f32| if dt == 2.0 { k } else { k.powf(dt / 2.0) };
        let (air_keep, air_drag) = (per(n.air_keep), per(n.air_drag));
        let speed: f32 = rd(actor + 0x2C);
        let dir: [f32; 3] = rd(actor + 0x20);
        let stats = super::super::pose::ptr(rd::<u32>(actor + 0x6C));
        let kh1_run: f32 = if stats > 0x10000 { rd(stats + 8) } else { 8.0 };
        let mut v = [v0[0], v0[2]];
        if !v.iter().all(|x| x.is_finite()) {
            v = [0.0; 2];
        }
        if !action && speed > 0.01 && dir.iter().all(|x| x.is_finite()) {
            let s = if speed >= kh1_run - 0.01 { n.run } else { n.walk };
            let t = [dir[0] * s, dir[2] * s];
            v = [v[0] * air_keep + t[0] * (1.0 - air_keep), v[1] * air_keep + t[1] * (1.0 - air_keep)];
        } else {
            v = [v[0] * air_drag, v[1] * air_drag];
        }
        set_kh1_motion(actor, &Motion { h: v, arc: kh1_arc }, dt);
        // (The frame's own drop, as KH1 writes it.)
        wr(actor + VEL_Y, -up);
        d[0] = v[0] * dt;
        d[2] = v[1] * dt;
        r
    }
}
