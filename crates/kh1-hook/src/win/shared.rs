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
//!   Glide KH2 KH2's glide (its start, speed, fall, turn, banking, hover and release) run in
//!   KH1's glide state (`glide`, the tester's kh2-glide-in-kh1s-glide-state; no borrow). The
//!   physics setting doesn't touch it.
//! - **A KH2-only move** (Aerial Dodge, KH2's Square moves, KH2's Quick
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

/// Base Sora is KH1's own states' (the ini's `shared`, not in a Drive Form on KH2's controller; Valor Form runs on
/// them too, `form::native_now`).
pub fn on() -> bool {
    super::settings::get().shared && !super::form::driver_form()
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
    if super::form::driver_form() {
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
// (4 was KH2's glide's borrow, t321..: KH2's glide runs in KH1's glide state now, `glide`.)
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
    // Valor Form on KH1's own states (`form::native_now`): KH2's Valor has no dodge roll, guard,
    // Quick Run, glide or Aerial Dodge (its growth ability is High Jump), so KH1 sees no Square,
    // and no jump in the air past the rise (KH1's own glide). Square is Valor's own Square moves
    // (`combomods::valor_square`: its row 11 free on the ground, rows 12 / 38 in a combo), on
    // KH1's rows of ours.
    if super::form::native_now() {
        let _ = super::combomods::valor_square(s, square_fresh);
        return KH1_ACTION_BIT | if air && !s.rising { KH1_JUMP_BITS } else { 0 };
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
    // (KH2's roll on KH1's roll, `combomods::roll_press`: KH1 doesn't let him act until KH2's
    // fall-check frame, but KH2 turns the held roll into Quick Run at its own check frame before
    // that, as the OG rewrite's DodgeRoll action does: that one press goes through.)
    let kh2_roll = quick_run && roll_kh2 && unsafe { super::kh2slots::playing_motion(super::takeover::sora_actor()) } == Some(character_core::motion::DODGE_ROLL);
    // KH1's own roll (Dodge Roll KH1) held into KH2's Quick Run at the same point as KH2's roll
    // (the tester, quick-run-native: "waits until dodge roll finishes to go into quick run ...
    // make sure it behaves the same as kh2 dodge roll"): KH2's roll turns into Quick Run at its
    // clip's Quick Run check (motion 209's trigger 34, frame 10; the OG rewrite's DodgeRoll action),
    // counted from the roll's first frame on the same counter as KH2's roll on KH1's roll above.
    // KH1 doesn't let him act until its roll motion is over (30 frames in the tester's trace), so
    // that check let KH2's Quick Run in only then; now it cuts KH1's roll at KH2's frame.
    let kh1_roll_cut = quick_run && !roll_kh2 && s.kh1_rolling && roll_frames >= s.quick_run_check;
    if s.can_act || kh2_roll || kh1_roll_cut {
        if quick_run {
            ROLL_HELD.store(false, Ordering::Relaxed);
            ROLL_FRAMES.store(0, Ordering::Relaxed);
            // KH2's Quick Run on KH1's state of its own (the tester, quick-run-native: "do your first
            // purposed route"), along his facing as KH2's roll turns into it (`quickrun::start`, no
            // borrow). Else the borrow.
            if unsafe { super::quickrun::start(super::takeover::sora_actor(), [0.0, 0.0], "Square held through KH1's roll") } {
                return KH1_ACTION_BIT;
            }
            borrow = QUICK_RUN;
        } else if s.state == 0 && square_fresh && !roll_kh2 && !s.kh1_roll && s.quick_run && s.moving {
            // (Dodge Roll at KH1 without KH1's roll equipped: Square while moving is KH2's Quick
            // Run, which has no style of its own, when it is equipped.) Natively along the stick
            // (`quickrun::start`), else the borrow.
            if unsafe { quick_run_on_stick("Square while moving (KH1's Dodge Roll not equipped)") } {
                return KH1_ACTION_BIT;
            }
            borrow = SQUARE;
        } else if s.state == 0 && square_fresh && roll_kh2 && super::combomods::guard_press() {
            // KH2's guard on KH1's states (stick at rest, Guard equipped): our request for KH2's
            // guard press, no borrow (`combomods::guard_press`). The press is KH2's.
            return KH1_ACTION_BIT;
        } else if s.state == 0 && square_fresh && roll_kh2 && super::combomods::roll_press() {
            // KH2's dodge roll on KH1's own roll (stick pushed, KH2's Dodge Roll equipped): KH1's
            // roll start with our slot of KH2's 209 on its request, no borrow
            // (`combomods::roll_press`). The press is KH2's. Square held through it turns into
            // KH2's Quick Run at KH2's check frame (below).
            ROLL_HELD.store(true, Ordering::Relaxed);
            ROLL_FRAMES.store(0, Ordering::Relaxed);
            return KH1_ACTION_BIT;
        } else if s.state == 0 && square_fresh && roll_kh2 {
            // (Nothing of KH2's for Square without those abilities: KH2 would only take him and
            // give him back two frames later, s261's "kh2 dodge roll is unusable" with Dodge
            // Roll, Guard and Quick Run all taken off in KH1's menu. KH1's own roll needs its
            // Dodge Roll equipped too, its bit `0x20` of `0x142D5F610`, set in its control step
            // from `0x140282A70 (22)`.)
            if s.kh2_square {
                // KH2's action button while moving with no dodge roll of KH2's but Quick Run (KH2's
                // order, `0x1403A75A0`: the guard with the stick at rest, else the dodge roll, else
                // Quick Run): its Quick Run natively (`quickrun::start`), else the borrow.
                let kh2_runs = s.moving && super::takeover::with_abilities(|a| a.dodge_roll == 0 && a.quick_run > 0) == Some(true);
                if kh2_runs && unsafe { quick_run_on_stick("Square while moving (the Dodge Roll style KH2, no dodge roll of KH2's)") } {
                    return KH1_ACTION_BIT;
                }
                borrow = SQUARE;
            } else {
                static N: AtomicU32 = AtomicU32::new(0);
                if super::log_on(super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 10 {
                    super::log!(Takeover, "Square with the Dodge Roll style KH2: KH2 has no dodge roll, guard or Quick Run (none of Dodge Roll, Guard, Quick Run is equipped in KH1's menu), so it doesn't take him");
                }
            }
        } else if air && jump_fresh && s.air_dodge && !AIR_DODGED.load(Ordering::Relaxed) {
            AIR_DODGED.store(true, Ordering::Relaxed);
            DODGE_JUMP_UP.store(false, Ordering::Relaxed);
            // KH2's Aerial Dodge on KH1's air state (the tester, aerial-dodge-native): our request
            // of KH2's motion 202 with KH2's hop, no borrow (`combomods::aerial_dodge`). The press
            // is KH2's: KH1 sees none of it (no glide, no Neverland flight on it). Else the borrow.
            if unsafe { super::combomods::aerial_dodge(super::takeover::sora_actor()) } {
                return KH1_JUMP_BITS | KH1_ACTION_BIT;
            }
            borrow = AIR_JUMP;
        } else if s.state == super::glide::STATE && jump_fresh && s.air_dodge && !AIR_DODGED.load(Ordering::Relaxed) && unsafe { super::glide::running(super::takeover::sora_actor()) } {
            // The Aerial Dodge out of KH2's glide (KH2's glide action's shared air slot
            // `0x1403CB9F0`): the glide ends into the fall, then the dodge on KH1's air state
            // (`combomods::aerial_dodge`). The press is KH2's.
            AIR_DODGED.store(true, Ordering::Relaxed);
            DODGE_JUMP_UP.store(false, Ordering::Relaxed);
            unsafe {
                let actor = super::takeover::sora_actor();
                super::glide::end(actor, "the jump: the Aerial Dodge out of the glide");
                super::combomods::aerial_dodge(actor);
            }
            return KH1_JUMP_BITS | KH1_ACTION_BIT;
        } else if air && s.kh2_glide && square_fresh {
            // KH2's glide in KH1's glide state (the tester, kh2-glide-in-kh1s-glide-state: "go
            // ahead"), on Square as in the KH2 rewrite (t327): up or down, before or after the
            // Aerial Dodge; no borrow (`glide::start`). The press is KH2's.
            if unsafe { super::glide::start(super::takeover::sora_actor(), "Square in the air") } {
                return KH1_JUMP_BITS | KH1_ACTION_BIT;
            }
        }
    }
    // On Steam no borrow of KH2's controller is left for these (the tester, what-is-still-missing
    // m5: "(b)", base Sora never on KH2's controller): KH2's dodge roll, guard and Quick Run run on
    // KH1's roll, guard row and Quick Run state, its Aerial Dodge on KH1's air state; when none of
    // those can take the press here (KH1's deep water for the roll, a slot not built), the press is
    // KH1's as unmodded, as t630 left KH1 its Square and jump where no KH1 row plays KH2's move.
    if borrow != NONE && super::addr::Build::current() == super::addr::Build::Steam {
        static N: AtomicU32 = AtomicU32::new(0);
        if super::log_on(super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 20 {
            super::log!(Takeover,
                "{}: no KH1 state of ours could take it here (KH1 state {}): the press is KH1's (no KH2-controller borrow)",
                match borrow {
                    SQUARE => "Square (KH2's dodge roll / guard / Quick Run)",
                    AIR_JUMP => "the jump in the air (KH2's Aerial Dodge)",
                    _ => "Square held through KH1's roll (KH2's Quick Run)",
                },
                s.state
            );
        }
        borrow = NONE;
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
    // Glide at KH2 (t321): KH1's own glide never, past the rise: the glide is KH2's (`glide`).
    if air && !s.rising && styles::kh2(Kind::Glide) {
        hide |= KH1_JUMP_BITS | KH1_ACTION_BIT;
    }
    // In KH2's glide (KH1's glide state at Glide KH2): its buttons are KH2's (Square held glides,
    // read off the pad by `glide`; the jump is the Aerial Dodge or nothing), none of KH1's.
    if s.state == super::glide::STATE && styles::kh2(Kind::Glide) {
        hide |= KH1_JUMP_BITS | KH1_ACTION_BIT;
    }
    hide
}

/// KH2's Quick Run natively along the stick (`quickrun::start`).
unsafe fn quick_run_on_stick(why: &str) -> bool {
    let actor = super::takeover::sora_actor();
    if actor < 0x10000 {
        return false;
    }
    super::quickrun::start(actor, super::takeover::stick_now(actor), why)
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
/// - (Air casts follow the Air Physics row, not this one: `physics::cast_rule`, the tester's m19.)
/// - KH1's own air moves (its air attacks, casts, a blow's launch: no KH2 numbers of their own):
///   their own lunge, speed and hop height, on KH2's gravity, fall and drift (`air_step`).
/// - The ground combo's swings (slots 98..101, its action flag 4 up; KH1's or KH2's animation):
///   moved by the root travel of KH2's swing for that slot (`pose::swing_motion`), stretched over
///   KH1's swing, along his facing, in place of KH1's own travel. KH1's hit objects, timing and
///   damage stay.
/// - Not the glide (t321): KH1's glide state (0x15) is KH1's own with its own numbers at either
///   physics setting, and at Glide KH2 it runs KH2's glide (`glide`), whatever the physics setting.
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
    use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
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
                super::super::log!(Takeover, "physics: {} for every move of base Sora's (the ini's `physics`; Drive Forms KH2's)", r.key().to_uppercase());
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
        super::super::log!(Takeover, "Config menu: physics {} (was {}): {}", r.key().to_uppercase(), was.key().to_uppercase(), match r {
            Rule::Kh1 => "KH1's numbers for every move of base Sora's, KH2's borrowed moves too (KH2's controller given KH1's gravity, fall, jump, speeds and air steering)",
            Rule::Kh2 => "KH2's numbers for every move of base Sora's, KH1's own moves on its states too",
        });
    }

    /// Whose physics numbers move base Sora now, whatever the move and its style (the styles choose
    /// the animations only): the setting, and KH2's in a Drive Form (its own moveset all through).
    /// The one switch point: KH1's own states take KH2's numbers where this is KH2's (the wrappers
    /// below), KH2's controller takes KH1's where it is KH1's (`kh2_controller`).
    pub fn source() -> Rule {
        rule().source(super::super::form::in_form() || super::super::form::native_running())
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

    /// KH2's hold (a no-gravity range) was on at Sora's last air step: KH2's `+0x120` bit
    /// 0x800000 ("held at the last think", set by `0x1403B4980`), for the fall's restart when
    /// the hold ends (`kh2_fall_at`).
    static HELD_LAST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    /// KH2's fall started over at `down` per frame (downward, y-down positive) from the height he
    /// is at: `0x1403B4780`, Checked (decomp) KH2 Steam: the arc's time `+0xF0` = speed / g, its
    /// rise time `+0xF4` and height `+0xFC` 0, its base `+0xF8` his height less g t² / 2.
    fn kh2_fall_at(down: f32, gravity: f32) -> Arc {
        let t = if gravity > 0.0 && down.is_finite() && down > 0.0 { down / gravity } else { 0.0 };
        Arc { t, ..Arc::fall() }
    }

    /// Not in KH1's air state (or KH2 has him): no arc of ours.
    pub fn off_air() {
        if let Ok(mut a) = AIR.try_lock() {
            *a = None;
        }
        HELD_LAST.store(false, Ordering::Relaxed);
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
        super::active() && source() == Rule::Kh2 && !super::super::form::driver_form()
    }

    /// The Air Physics row (Config > Combat > Magic; `styles::Kind::AirPhysics`): whose physics move
    /// base Sora while he casts in the air, every spell, in place of the Physics row (the tester,
    /// thread kh2-effects-in-kh1s-own-effect-system m19: "change air magic to air physics kh1 |
    /// kh2 so now you can cast kh2 fire with kh1 air physics like how we did for reflect. remove
    /// anything related to the main physics setting that has magic in it and put it in this new
    /// air setting"). Drive Forms are KH2's (their own casts), as for the Physics row.
    pub fn cast_rule() -> Rule {
        if super::super::form::in_form() || super::super::styles::setting(super::super::styles::Kind::AirPhysics) {
            Rule::Kh2
        } else {
            Rule::Kh1
        }
    }

    /// KH2's air-cast physics apply to KH1's air state now (`cast_rule`, base Sora on KH1's states).
    fn cast_kh2() -> bool {
        super::active() && cast_rule() == Rule::Kh2 && !super::super::form::driver_form()
    }

    /// KH1's request ids of its air casts (`kh2slots::CASTS`: `0x83 + spell`, Fire's second part
    /// 0x8A; Checked (files) `xa_ex_0010.mset`), whatever plays on them.
    const AIR_CAST_REQUESTS: std::ops::RangeInclusive<u32> = 0x83..=0x8A;

    /// Base Sora is casting in KH1's air state now: its action flag (`+0` bit 2) with KH1's air
    /// cast request applied (`+0x164`'s low half, `0x14029DAB0`; KH1's own cast motion or our slot
    /// of KH2's on it), or a cast slot of ours (Reflect's requests, `kh2slots::cast_playing`).
    unsafe fn casting_in_air(actor: usize) -> bool {
        rd::<i32>(actor + 0x70) == 2 && rd::<u32>(actor) & 4 != 0 && (AIR_CAST_REQUESTS.contains(&(rd::<u32>(actor + 0x164) & 0xFFFF)) || super::super::kh2slots::cast_playing(actor))
    }

    /// KH1's own air cast animation (KH1's slot on KH1's air cast request, no slot of ours) at Air
    /// Physics KH2: the request, the 60 Hz frames since it began playing and the game frame last
    /// counted (`kh1_cast_hold`).
    static KH1_CAST: Mutex<Option<(u32, f32, u64)>> = Mutex::new(None);

    /// Air Physics KH2 with KH1's own air cast animation (the tester, kh2-effects-in-kh1s-own-
    /// effect-system m20 answer 2: "(b) hold him like kh2, the player expects it to be like kh2"):
    /// KH2's hover for that cast, timed from the cast's start: the no-gravity range (id 3) of KH2's
    /// air cast clip for the spell (`kh2slots::CASTS`: Fire 58 16..74, Blizzard 61 16..66, Thunder
    /// 64 / Cure 67 16..58; Gravity on Thunder's, Stop and Aero on Cure's; Checked (files)
    /// `P_EX100.mset`) at the frames KH1's cast motion has played, as for our slots (§4h; the fall
    /// after it starts over from rest, KH2's `0x1403B4780`, §4n). KH1's air cast motions outlast those ranges (slots 80 / 81: 78 / 68
    /// frames, Checked (files) `xa_ex_0010.mset`), so the hold ends inside KH1's cast as KH2's
    /// does. Nothing at Air Physics KH1 (KH1's own air cast has no hover).
    unsafe fn kh1_cast_hold(actor: usize, dt: f32) -> bool {
        use character_core::WindowKind as W;
        let Ok(mut g) = KH1_CAST.try_lock() else { return false };
        if !cast_kh2() || !casting_in_air(actor) || super::super::kh2slots::cast_playing(actor) {
            *g = None;
            return false;
        }
        let request = rd::<u32>(actor + 0x164) & 0xFFFF;
        let frame = game_frame();
        let t = match *g {
            Some((r, t, f)) if r == request && f == frame => t,
            Some((r, t, f)) if r == request && f + 1 == frame => t + dt,
            _ => 0.0,
        };
        *g = Some((request, t, frame));
        let Some(&(motion, ..)) = super::super::kh2slots::CASTS.iter().find(|c| c.2 == request as usize && c.3) else { return false };
        let Some(clip) = super::super::pose::kh2().and_then(|k| k.character.moveset.clips.get(&motion)) else { return false };
        let moving = clip.windows.iter().any(|w| matches!(w.kind, W::AllowMovement) && w.start == 0 && w.end.is_none());
        let hold = !moving && clip.has(t as u32, |k| matches!(k, W::NoGravity));
        static N: AtomicU32 = AtomicU32::new(0);
        if hold && N.load(Ordering::Relaxed) < 20 && clip.first(|k| matches!(k, W::NoGravity)).is_some_and(|s| (t as u32) == s) {
            N.fetch_add(1, Ordering::Relaxed);
            super::super::log!(Takeover, "Air Physics KH2 on KH1's air cast animation (request {request:#x}): KH2's hover of motion {motion} from frame {t:.0}");
        }
        hold
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

    /// KH2's magic jump (sstm `+0x48` `magic_jump`, 40.0, Checked (files) `03system.bin`): the hop
    /// KH2's cast start gives an airborne caster (`0x1403C5170` → the action's slot 4
    /// `0x140409B00` → `0x1403CBC30 (actor, 40)`, Checked (decomp) KH2 Steam). (sstm `+0xD0`, 30.0,
    /// is for actor `+0x6C8` bit 7 with `+0xDE0 == 7`, not base Sora.)
    const MAGIC_JUMP: f32 = 40.0;

    /// The velocity a KH2 air cast keeps (`cast_stops`), for after KH1's command step
    /// (`cast_after_command`): the game frame and KH1's `+0x3C0` before KH1's air cast halved it.
    static CAST_KEEP: Mutex<Option<(u64, [f32; 4])>> = Mutex::new(None);

    /// A KH2 cast starting on KH1's cast state in the air (`formmagic::kh1_spell_start`,
    /// `reflect::cast_native`), physics KH2, base Sora: KH2's air cast start (the tester, thread
    /// kh2-magic-and-reflect-native, m5 plan item 3; Drive Forms as they are). KH2's
    /// `0x1403C5170` (Checked (decomp) KH2 Steam, `docs/kh1-magic-native.md` §4e): a hop of
    /// `MAGIC_JUMP` from where he is (`0x1403CBC30` with h ≥ 0: the arc's time 0, its height h, its
    /// rise time `sqrt(2h / g)`, 14 frames for 40), the launch speed 0 so the sideways velocity is
    /// **kept**; then ACTION_JUMP's step drifts it × 0.95 a tick (`air_step`'s `air_drag` in a
    /// move) and the cast motion's no-gravity range holds him (`kh2slots::no_gravity`). KH1's own
    /// air cast (`0x1402A3600`: a 30 hop, half the sideways speed, Checked (decomp)) is replaced:
    /// KH2's arc is `air_step`'s from here, and the velocity before KH1's halving is put back
    /// after KH1's command step (`cast_after_command`). (t487's stop, sideways 0 and a fall from
    /// rest, is gone: it was not KH2's.) Must be called before KH1's command step halves `+0x3C0`.
    pub unsafe fn cast_stops(actor: usize) {
        // (The Air Physics row's, not the Physics row's: `cast_rule`.)
        if !cast_kh2() || !is_sora(actor) || rd::<i32>(actor + 0x70) != 2 {
            return;
        }
        let Some(n) = numbers() else { return };
        let y: f32 = rd(actor + 0x14);
        let v: [f32; 4] = rd(actor + VEL);
        if !y.is_finite() || !v.iter().all(|x| x.is_finite()) {
            return;
        }
        let was = kh1_motion(actor);
        let arc = Arc::new(MAGIC_JUMP, n.gravity);
        if let Ok(mut g) = AIR.try_lock() {
            *g = Some(Air { arc, last_y: y, asked_up: 0.0, written: None });
        }
        set_kh1_motion(actor, &Motion { h: [v[0], v[2]], arc: kh1phys::Arc::from_kh2(&arc, y, n.gravity, n.fall_max) }, 1.0);
        if let Ok(mut k) = CAST_KEEP.lock() {
            *k = Some((game_frame(), v));
        }
        super::super::log!(Takeover, "KH2's air cast start (Air Physics KH2): a hop of {MAGIC_JUMP:.0} ({:.1} frames up) from where he is (KH1's arc rising {}), sideways {:.1}/frame kept", arc.uptime, was.arc.up_speed() > 0.0, v[0].hypot(v[2]));
    }

    /// After KH1's command step ran a KH2 air cast (`combat::command`, `reflect::cast_native`):
    /// KH1's air cast halved `+0x3C0` and wrote its own 30 hop into its arc fields after the spell
    /// start; KH2's kept velocity and KH2's arc (`cast_stops`) go back on them.
    pub unsafe fn cast_after_command(actor: usize) {
        let Some((at, v)) = CAST_KEEP.try_lock().ok().and_then(|mut k| k.take()) else { return };
        if at != game_frame() || !is_sora(actor) || rd::<i32>(actor + 0x70) != 2 {
            return;
        }
        let Some(n) = numbers() else { return };
        let y: f32 = rd(actor + 0x14);
        let arc = AIR.try_lock().ok().and_then(|g| g.as_ref().map(|a| a.arc));
        let Some(arc) = arc.filter(|_| y.is_finite()) else { return };
        set_kh1_motion(actor, &Motion { h: [v[0], v[2]], arc: kh1phys::Arc::from_kh2(&arc, y, n.gravity, n.fall_max) }, 1.0);
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
            super::super::log!(Takeover,
                "KH1's air state takes him with KH2's momentum{}: sideways {:.1}/frame, up {up:.1}/frame (KH1's arc: time {:.1}, apex at {:.1}, {:.0} higher)",
                if fresh { "" } else { " (no KH2 step written lately: from rest, KH2's sideways speed)" },
                h[0].hypot(h[1]),
                m.arc.t,
                m.arc.up_t,
                m.arc.rise_left()
            );
        }
        m
    }

    /// KH2's hop of a combo row (`combomods`: a ground row with `jump`, KH1's air swing row run
    /// for it out of KH1's air state just entered, or an air row's own hop on KH1's air swing,
    /// `combat::exec`): KH1's arc fields and air speed
    /// set to KH2's, after the row's start set its own lunge and hop (`0x1402A68F0`, Steam: the
    /// air rows 15..18 give `+0x3C0` 8 / 12 / 4 along the direction, and the row's own hop).
    /// `height`: KH2's rise (`Launch`), `vel`: its sideways velocity (x / z, a 60 Hz frame).
    /// Physics KH2: KH2's arc for that height on KH2's gravity, which `air_step` then carries on
    /// (as `takeoff` does for KH2's jump); physics KH1: KH1's rise of that height
    /// (`kh1phys::Arc::hop`, KH1's gravity), the row's height and speed being the move's own
    /// (`docs/kh1-physics.md` §7).
    pub unsafe fn kh2_hop(actor: usize, height: f32, vel: [f32; 2]) {
        kh2_hop_as(actor, height, vel, false);
    }

    /// `kh2_hop` for a rise the jump button holds (KH2's Aerial Dodge: its `+0x120` bit `0x10000`,
    /// which the air actions' shared enter `0x1403CBD30` turns into the held jump `0x20000`;
    /// `0x1403D6FC0` marks its release, `0x1403CB680` cuts the rise there; Checked (decomp)):
    /// physics KH2, its early release is `air_step`'s, as for KH2's jump; physics KH1, no release:
    /// KH1's cut is closed on it and it always rises fully (`kh1_held_jump`).
    pub unsafe fn kh2_hop_held(actor: usize, height: f32, vel: [f32; 2]) {
        kh2_hop_as(actor, height, vel, true);
    }

    unsafe fn kh2_hop_as(actor: usize, height: f32, vel: [f32; 2], holdable: bool) {
        let y: f32 = rd(actor + 0x14);
        if !is_sora(actor) || !y.is_finite() || !height.is_finite() || rd::<i32>(actor + 0x70) != 2 {
            return;
        }
        let vel = kh1phys::carried(vel);
        let kh2_numbers = if kh2() { numbers() } else { None };
        let how = match kh2_numbers {
            Some(n) => {
                let mut arc = Arc::new(height, n.gravity);
                arc.holdable = holdable;
                let kh1 = kh1phys::Arc::from_kh2(&arc, y, n.gravity, n.fall_max);
                set_kh1_motion(actor, &Motion { h: vel, arc: kh1 }, dt());
                if let Ok(mut a) = AIR.lock() {
                    *a = Some(Air { arc, last_y: y, asked_up: 0.0, written: Some((y, kh1)) });
                }
                format!("KH2's arc, {:.1} frames up", arc.uptime)
            }
            None => {
                set_kh1_motion(actor, &Motion { h: vel, arc: kh1phys::Arc::hop(y, height) }, dt());
                off_air();
                if holdable {
                    // (KH1's early-release cut closed on it once KH1's control step this frame is
                    // done, `kh1_held_jump`: the full rise.)
                    HELD_JUMP_AT.store(game_frame().max(1), Ordering::Relaxed);
                }
                "KH1's gravity (physics KH1)".to_string()
            }
        };
        static N: AtomicU32 = AtomicU32::new(0);
        if super::super::log_on(super::super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 40 {
            super::super::log!(Takeover, "KH2's hop on KH1's air state: {height:.0} high, {:.1}/frame sideways, {how}", vel[0].hypot(vel[1]));
        }
    }

    /// The game frame a held hop (`kh2_hop_held`) was started at physics KH1, 0 none.
    static HELD_JUMP_AT: AtomicU64 = AtomicU64::new(0);

    /// After KH1's control step (`takeover::shared_control`): a held hop started in it at physics
    /// KH1 (KH2's Aerial Dodge, `combomods::aerial_dodge`) has KH1's early-release cut closed
    /// (`+0` flag `0x40000` off), so it always rises fully (the tester's choice, aerial-dodge-native
    /// m3 "(b)": no release at physics KH1). KH1's button handler (`0x1402A7B30`, from the control
    /// step `0x1402A6220`, Steam, Checked (decomp)) runs its variable jump only while `0x40000` is
    /// on (with `0x20000` off and the air time at 6 or more, the cut to `0x1402AF8F0 (actor, 5.0)`;
    /// past 5 frames before the apex, or out of state 2, it clears `0x40000` itself), and no other
    /// KH1 code reads that bit of the actor's `+0` (Checked (decomp): the other `0xE0000` tests are
    /// on attack objects). The dodge kept the flags of the jump before it, and with that jump let
    /// go once (`0x40000` on, `0x20000` off) KH1 cut every dodge 6 frames in (traces f80604 /
    /// f80807). t545 set `0x60000` instead (KH1's own takeoff's, `0x1402AFDA0`), giving the dodge
    /// KH1's release; closed now. The fall comes at the apex (`combomods::dodge_apex`). Physics KH2
    /// is not this path (`air_step` keeps KH1's cut off and runs KH2's own release). Set after the
    /// step, as KH1 runs the cut inside it.
    pub unsafe fn kh1_held_jump(actor: usize) {
        let at = HELD_JUMP_AT.swap(0, Ordering::Relaxed);
        if at == 0 || at != game_frame() || !is_sora(actor) || rd::<i32>(actor + 0x70) != 2 {
            return;
        }
        wr(actor, rd::<u32>(actor) & !CUT_OPEN);
        static N: AtomicU32 = AtomicU32::new(0);
        if super::super::log_on(super::super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 20 {
            super::super::log!(Takeover, "held hop at physics KH1: KH1's early-release cut closed (flag 0x40000 off), the full rise");
        }
    }

    /// KH2's hop out of KH1's ground combo (`combat::exec`, KH1's state just set to 2): the game
    /// frame and KH1's air velocity (`+0x3C0`) as `kh2_hop` left it.
    static GROUND_HOP: Mutex<Option<(u64, [f32; 4])>> = Mutex::new(None);

    /// After `kh2_hop` for a hop out of KH1's ground combo (`combat::exec`): KH1's control step
    /// goes on, after the command, with the state step and then its carry into the air on the
    /// frame its state changed (Steam `0x1402A3300`, Checked (decomp): with flag `0x400` and state
    /// 2, `+0x3C0` = the move direction `+0x20` times its speed `+0x2C`, which the air step
    /// `0x1402AFC20` has just copied from the stick). Standing, that is 0, and KH2's sideways speed
    /// was gone (the tester's t409 traces f3990..f5413: every ledge hop rose and fell in place, the
    /// velocity 0 from its first frame, where the log had 8/frame). Kept for `ground_hop_carry`.
    pub unsafe fn kh2_hop_from_ground(actor: usize) {
        if !is_sora(actor) || rd::<i32>(actor + 0x70) != 2 {
            return;
        }
        if let Ok(mut g) = GROUND_HOP.lock() {
            *g = Some((game_frame(), rd(actor + VEL)));
        }
    }

    /// Sora's physics step (`takeover::physics`), before KH1's: on the frame after
    /// `kh2_hop_from_ground`, KH2's hop's velocity goes back on KH1's air fields in place of KH1's
    /// carry from the stick, so KH1's physics step moves him by it as KH2's hop (`0x1403CBB00`:
    /// the velocity the flat direction times the speed) does from its first frame.
    pub unsafe fn ground_hop_carry(actor: usize) {
        let Some((at, vel)) = GROUND_HOP.lock().ok().and_then(|mut g| g.take()) else { return };
        if game_frame().saturating_sub(at) != 1 || !is_sora(actor) || rd::<i32>(actor + 0x70) != 2 {
            return;
        }
        let was: [f32; 4] = rd(actor + VEL);
        wr(actor + VEL, vel);
        static N: AtomicU32 = AtomicU32::new(0);
        if super::super::log_on(super::super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 40 {
            super::super::log!(Takeover,
                "KH2's hop out of KH1's ground combo: its {:.1}/frame sideways put back after KH1's carry into the air (which left {:.1})",
                vel[0].hypot(vel[2]),
                was[0].hypot(was[2])
            );
        }
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
        if super::super::log_on(super::super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 40 {
            super::super::log!(Takeover,
                "KH1's air attack (motion {motion:#x}) out of KH2's move: {}",
                if set { format!("its own lunge {s:.1}/frame given (KH1 left it {had:.1}; {speed:.1} at its start, frame {t:.0})") } else { format!("it has {had:.1}/frame already, left as it is") }
            );
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
            super::super::log!(Takeover,
                "KH1's air attack out of KH2's motion: its own lunge {:.1}/frame and hop {:.0}; KH2's speed {:.1}/frame and rise {:.0} left; kept {:.1}/frame and {}",
                s(now.h),
                now.arc.rise_left(),
                s(before.h),
                before.arc.rise_left(),
                s(kept.h),
                if kept.arc == now.arc { "KH1's hop" } else { "KH2's rise" }
            );
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
            if super::super::log_on(super::super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 20 {
                super::super::log!(Takeover,
                    "KH2's controller on {} numbers: gravity {:.3}, fall {:.0}, jump {:.0}, walk {:.1}, run {:.1} (past {:.2} tilt), air {}",
                    if stats.is_some() { "KH1's" } else { "its own (KH2's)" },
                    p.gravity,
                    p.fall_max,
                    p.jump_height,
                    p.walk_speed,
                    p.run_speed,
                    p.run_threshold,
                    if p.air_accel > 0.0 { format!("steering +{:.3} x speed a frame, drag {:.3} a tick", p.air_accel, p.air_drag) } else { format!("keep {:.3}, drag {:.3} a tick", p.air_keep, p.air_drag) }
                );
                if let Some(s) = &stats {
                    if s.air_accel.is_none() || s.air_drag.is_none() || s.jump.is_none() {
                        super::super::log!(Takeover, "(KH1's stats not all read: {s:?}; those stay KH2's)");
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
                super::super::log!(Takeover, "shared states: KH1's state step at {slot:#x} wrapped ({now:#x})");
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
    /// KH1's ground combo (physics KH2, either swing animation), fed to KH1's own step.
    unsafe extern "system" fn ground_step(actor: usize, disp: *mut [f32; 4]) -> usize {
        let f: StepFn = std::mem::transmute(ORIG_GROUND.load(Ordering::Acquire));
        // KH1's ground swing: KH2's travel for it written into KH1's own move vector and speed
        // before KH1's step, which moves him by them (`0x1402B1850`: the move vector `+0x20` laid
        // on the ground's normal `+0xC0`, times the speed `+0x2C`, times the frame), so slopes and
        // KH1's own reading of those fields see KH2's lunge (t363; it used to replace the step's
        // result afterwards).
        let swing = (is_sora(actor) && rd::<i32>(actor + 0x70) == 0 && !disp.is_null() && rd::<u32>(actor) & 4 != 0)
            .then(super::super::pose::sora_slot)
            .flatten()
            .filter(|s| (98..=101).contains(s));
        if let Some(slot) = swing {
            let r = swing_step(actor, slot, disp, f);
            // KH2's edge stop in our slot's motion (either physics: it is the motion's own).
            edge_stop(actor, disp);
            return r;
        }
        let r = f(actor, disp);
        if !is_sora(actor) || rd::<i32>(actor + 0x70) != 0 || disp.is_null() {
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

    /// KH2's edge stop factor: its ground mover sweeps the step lengthened by this times his
    /// collision radius (`+0x6F0`, Sora's 90) for the floor (`0x1403B9090`: `DAT_14074B060`, 0.2,
    /// Checked (exe)).
    const EDGE_REACH: f32 = 0.2;
    /// KH1's travel flag on the actor (`+0x160` bit 4) and the frame's travel it adds after the
    /// state's step (`+0x200`, the physics step `0x14029A3C0`, Checked (decomp)).
    const TRAVEL_ON: u32 = 4;
    const TRAVEL: usize = 0x200;

    /// KH2's edge stop on KH1's ground swing (the tester: "it should cancel his animation like
    /// kh1, ... look at steam kh2 ghidra decomp"; Steam KH2, Checked (decomp)): while KH2's motion
    /// has range id 29 open (`kh2slots::edge_stop`: its range handler `0x14040D810` sets the motion
    /// state word `+0x18C` bit 23), KH2's ground mover (`0x1403B9090`, its test `0x1403BAB20`) sweeps
    /// the horizontal step lengthened by 0.2 of his radius for the floor (`0x1403BA750`, the same
    /// no-floor rule as the line ahead) and, with none, keeps only the step's vertical part: he
    /// stays on the ledge and the swing goes on. Outside it he steps off and KH2 cuts the swing into
    /// its fall (below), as KH1 does. Here: this frame's step (KH1's step's result and the motion's
    /// travel KH1 adds after it, `+0x200`), on KH1's collision (`combomods::floor_gone`); with no
    /// floor, both lose their horizontal part.
    unsafe fn edge_stop(actor: usize, disp: *mut [f32; 4]) {
        if disp.is_null() || !super::super::kh2slots::edge_stop(actor) {
            return;
        }
        let travel = rd::<u32>(actor + 0x160) & TRAVEL_ON != 0;
        let t: [f32; 4] = if travel { rd(actor + TRAVEL) } else { [0.0; 4] };
        let d = *disp;
        let step = [d[0] + t[0], d[2] + t[2]];
        let len = step[0].hypot(step[1]);
        if !(len > 1e-3) || !len.is_finite() {
            return;
        }
        let Some(radius) = super::super::pose::kh2().and_then(|k| kh2_data::sora::bg_shape(&k.rig.body_shapes).map(|s| s.radius)) else { return };
        if super::super::combomods::floor_gone(actor, step, len + EDGE_REACH * radius).is_none() {
            return;
        }
        let d = &mut *disp;
        d[0] = 0.0;
        d[2] = 0.0;
        if travel {
            wr(actor + TRAVEL, [0.0f32, t[1], 0.0, t[3]]);
        }
        static N: AtomicU32 = AtomicU32::new(0);
        if super::super::log_on(super::super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 40 {
            super::super::log!(Takeover,
                "KH2's edge stop (KH2 motion {:?}): no floor {:.0} ahead, his {len:.1} step this frame kept on the ledge",
                super::super::kh2slots::playing_motion(actor),
                len + EDGE_REACH * radius
            );
        }
    }

    /// KH1's ground swing's step (state 0, our wrapper): KH2's swing travel with physics KH2.
    unsafe fn swing_step(actor: usize, slot: usize, disp: *mut [f32; 4], f: StepFn) -> usize {
        if !kh2() {
            return f(actor, disp);
        }
        // (Ours baked with KH2's root travel in its H10 (t365, `kh2slots::moves_itself`):
        // KH1's own root travel moves him (`0x14029A3C0` adds `+0x200` after this step) and
        // keeps the model on him; feeding KH2's travel here too moved him twice, and KH1
        // pulled the model back by the template's travel: the t364 trace, the model trailing
        // the screen's centre, then jumping to it at the swing's end.)
        if super::super::kh2slots::moves_itself(actor) {
            if let Ok(mut g) = SWING.try_lock() {
                *g = None;
            }
            return f(actor, disp);
        }
        // (KH1's own move vector and speed put back after its step: the next frame, with the
        // stick let go, KH1's input filter for the state (`0x1402A5F10`) takes the move vector
        // as the input, and the ground pre-step (`0x1402CC1A0`, `0x140297FB0`) turns his
        // desired facing (`+0x150`) toward it; KH1's turn (`0x1402A3300`, π/16 a frame) then
        // turned him toward KH2's travel, sideways in its finisher: the t363 trace.)
        let kept: [f32; 4] = rd(actor + 0x20);
        let fed = swing_travel(actor, slot);
        let r = f(actor, disp);
        if fed {
            wr(actor + 0x20, kept);
        }
        r
    }

    /// KH1's ground swing (its slot `slot`) moved as KH2's swing for it moves him: KH2's clip's
    /// root travel (`Clip::root`, +Z forward, +X right), stretched over KH1's swing as its motion
    /// is (`pose::follow`), along his facing, as KH1's move vector (`+0x20`, flat) and speed
    /// (`+0x2C`, units a 60 Hz frame) for KH1's own step. KH1's own travel this frame (its swing's
    /// step forward, its dash at a target) gives way: one owner state (KH1's swing, its hit
    /// objects and timing), KH2's physics on it.
    unsafe fn swing_travel(actor: usize, slot: usize) -> bool {
        // (Our slot playing for it, KH2's row selection's pick among them: its motion's travel.)
        let ours = super::super::kh2slots::playing().filter(|s| super::super::kh2slots::stands_for(*s) == Some(slot)).and_then(super::super::kh2slots::kh2_motion);
        let Some(motion) = ours.or_else(|| super::super::pose::swing_motion(slot)) else { return false };
        let Some(clip) = super::super::pose::kh2_moves().and_then(|c| c.moveset.clips.get(&motion)) else { return false };
        let kh1_len = super::super::pose::slot_frames(slot).unwrap_or(0) as f32;
        if clip.root.len() < 2 || kh1_len < 1.0 {
            return false;
        }
        let Ok(mut g) = SWING.try_lock() else { return false };
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
            return false;
        }
        // (KH2's world turned by his facing as the controller does, `fo.root`; KH1's x / z are
        // KH2's.)
        let fw = [yaw.sin(), yaw.cos()];
        let right = [fw[1], -fw[0]];
        let (x, z) = (local[0] * right[0] + local[1] * fw[0], local[0] * right[1] + local[1] * fw[1]);
        let len = (x * x + z * z).sqrt();
        if len > 1e-5 && dt > 1e-4 {
            wr(actor + 0x20, [x / len, 0.0f32, z / len]);
            wr(actor + 0x2C, len / dt);
        } else {
            wr(actor + 0x2C, 0.0f32);
        }
        true
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
            super::super::log!(Takeover,
                "KH1's {} with KH2's arc: {h:.0} high, {:.1} frames up (KH1's own: {:.0}; its air acceleration {:.3}, drag {:.3})",
                if high { "High Jump" } else { "jump" },
                arc.uptime,
                s(0x10),
                s(0x14),
                s(0x18)
            );
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
    ///
    /// KH2's no-gravity range on our slot playing (`kh2slots::no_gravity`: KH2's ability moves'
    /// motions, Aerial Dive's frames 46..54), either physics: his rise or fall held where it is,
    /// at physics KH2 the arc's time going on (KH2's ACTION_JUMP step, `0x1403CB360`) and the fall
    /// started over from his actual speed when the hold ends (`0x1403B4780`), at physics KH1
    /// not (the arc kept and gone on with after it; never for a cast slot there, KH1's own cast has
    /// no hold), the sideways speed KH1's (or KH2's) air drift as before.
    unsafe extern "system" fn air_step(actor: usize, disp: *mut [f32; 4]) -> usize {
        let f: StepFn = std::mem::transmute(ORIG_AIR.load(Ordering::Acquire));
        let sora = is_sora(actor);
        let v0: [f32; 4] = if sora { rd(actor + 0x3C0) } else { [0.0; 4] };
        // Whose physics move him this frame: the Air Physics row's while he casts in the air
        // (`cast_rule`, the tester's m19), else the Physics row's.
        let casting = sora && casting_in_air(actor);
        let phys = if casting { cast_kh2() } else { kh2() };
        // (A cast at Air Physics KH1 under physics KH2: no arc of ours through it, so KH2's arc
        // after it goes on from KH1's, not from the one before the cast.)
        if casting && !phys && kh2() {
            off_air();
        }
        // (Physics KH1: none for a cast slot, as KH1's own air cast has no hover, `0x1402AF6A0`;
        // the tester, m4: "see how casting is done in the air physics wise to get the same for kh1".)
        let step_dt: f32 = rd(base() + DT_RVA);
        let step_dt = if step_dt.is_finite() && step_dt > 0.0 && step_dt < 4.0 { step_dt } else { 1.0 };
        let air_ok = sora && rd::<i32>(actor + 0x70) == 2 && !disp.is_null() && rd::<u32>(actor) & AIR_HELD == 0;
        // (KH1's own air cast animation at Air Physics KH2: KH2's hover for it, `kh1_cast_hold`.)
        let kh1_hold = sora && kh1_cast_hold(actor, step_dt);
        // (A rising row's first frames, before its own no-gravity range: `kh2slots::rise_start`.)
        let rise = if air_ok { super::super::kh2slots::rise_start(actor, step_dt) } else { None };
        let held = air_ok && (kh1_hold || rise.is_some() || super::super::kh2slots::no_gravity(actor) && (!super::super::kh2slots::cast_playing(actor) || cast_kh2()));
        // (Whether the hold was on at his last air step: KH2's fall restart when it ends, below.)
        let was_held = sora && HELD_LAST.swap(held, Ordering::Relaxed);
        if held {
            static N: AtomicU32 = AtomicU32::new(0);
            if super::super::log_on(super::super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 20 {
                super::super::log!(Takeover, "KH2's no-gravity range on KH1's air state (KH2 motion {:?}): his height held ({})", super::super::kh2slots::playing_motion(actor), if phys { "KH2's arc" } else { "KH1's arc" });
            }
        }
        let kh1_held = if held && !phys { Some(kh1_motion(actor)) } else { None };
        // (KH2's root travel where its clip has no gravity, `kh2slots::air_root`: Finishing Leap.)
        let root_dt: f32 = rd(base() + DT_RVA);
        let root = if held { super::super::kh2slots::air_root(actor, if root_dt.is_finite() && root_dt > 0.0 && root_dt < 4.0 { root_dt } else { 1.0 }).or(rise) } else { None };
        let ours = sora && rd::<i32>(actor + 0x70) == 2 && !disp.is_null() && phys;
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
        // An enemy KH2 knocked into the air: KH2's flight in KH1's step (`knockback`).
        if !sora && !disp.is_null() {
            super::super::knockback::air_step(actor, disp);
        }
        // (Physics KH1: KH1's step undone up and down, its arc's time and start put back.)
        if let Some(b) = kh1_held.filter(|_| rd::<i32>(actor + 0x70) == 2) {
            wr(actor + ARC_T, b.arc.t);
            wr(actor + ARC_Y0, b.arc.y0);
            wr(actor + VEL_Y, 0.0f32);
            (*disp)[1] = 0.0;
            if let Some(d) = root {
                add_root(actor, disp, d);
            }
        }
        if !sora || rd::<i32>(actor + 0x70) != 2 || disp.is_null() || !phys {
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
        // KH1's own move in the air (an air attack, a cast, a blow), flag 4. Not KH2's Aerial Dodge
        // on our request (`combomods::aerial_dodge`, asked with the flag): KH2 steers it with the
        // stick all through (its controller's `air_control` with steering on), as a jump.
        let dodge = super::super::kh2slots::playing_motion(actor) == Some(character_core::motion::AERIAL_DODGE);
        let action = flags & 4 != 0 && !dodge;
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
                    if super::super::log_on(super::super::LogCat::Takeover) && KEPT.fetch_add(1, Ordering::Relaxed) < 5 {
                        super::super::log!(Takeover, "KH1 rewrote its arc in a plain jump's rise: KH2's arc kept (its early release still open)");
                    }
                } else {
                    a.arc = kh1phys::kh1_arc_on_kh2(&b.arc, n.gravity);
                    a.asked_up = 0.0;
                    static N: AtomicU32 = AtomicU32::new(0);
                    if super::super::log_on(super::super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 20 {
                        super::super::log!(Takeover, "KH1's own air move{} set its arc: on KH2's numbers from here ({})", if action { "" } else { " (no action flag)" }, if a.arc.rising() { format!("a rise of {:.0} over {:.1} frames", a.arc.height - a.arc.y(a.arc.t, n.gravity), a.arc.uptime - a.arc.t) } else { format!("falling at {:.1}/frame", n.gravity * a.arc.t) });
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
            if super::super::log_on(super::super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 20 {
                super::super::log!(Takeover, "KH2's arc on KH1's air state from KH1's own: {}", if arc.rising() { format!("rising, {:.0} to go", arc.height - arc.y(arc.t, n.gravity)) } else { format!("falling at {:.1}/frame", n.gravity * arc.t) });
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
                // (KH2's release swaps the jump's motion for its fall at once, before its hop:
                // `0x1403CB680` -> `0x1403CB610`, Checked (decomp); KH1's fall request on our slot.)
                super::super::kh2slots::fall_now(actor, "the jump let go");
                // (KH2's Aerial Dodge let go: its fall now, as KH2's release does, `combomods`.)
                if dodge {
                    super::super::combomods::dodge_fall(actor, "the jump let go");
                }
            } else if air.arc.t >= air.arc.uptime - character_core::HOLD_APEX_MARGIN {
                air.arc.holdable = false;
            }
        }
        // KH2's hold (the physics KH2 path: casts, and KH2's moves with the same hold, Aerial
        // Finish, Magnet Burst, Aerial Dive, Finishing Leap). ACTION_JUMP's step `0x1403CB360`
        // (Checked (decomp) KH2 Steam) adds the step to the arc's time `+0xF0` before its hold
        // test (motion flag 0x20, range id 3, or 0x40, range 24) and moves him by 0 while held.
        // But on the frame the hold ends, before that step, SORA's think starts his fall over:
        // vtable slot +0x28 `0x140404B60` -> `0x1403A9250` -> `0x1403D6870` -> `0x1403DAD10` ->
        // `0x1403B4780` finds the hold gone with `+0x120` bit 0x800000 (held at the last think,
        // `0x1403B4980` from slot +0x10) and sets the arc to a fall at his actual vertical speed
        // (`+0xA3C`, 0 after a hold): time = speed / g, rise time and height 0, base where he is.
        // So he drops from rest at KH2's gravity (Checked (decomp); Checked (video): the tester's
        // kh2.mp4, an air Reflega falling from rest at its hold's end, landing ~31 frames later).
        // t621 and before ran the clock on through the hold and dropped him at the 16 cap at once.
        if was_held && !held {
            let down = if dt > 0.0 { (y - air.last_y) / dt } else { 0.0 };
            air.arc = kh2_fall_at(down.max(0.0), n.gravity);
            static N: AtomicU32 = AtomicU32::new(0);
            if super::super::log_on(super::super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 20 {
                super::super::log!(Takeover, "KH2's hold ended on KH1's air state (KH2 motion {:?}): his fall starts over at {:.1}/frame (KH2's 0x1403B4780)", super::super::kh2slots::playing_motion(actor), down.max(0.0));
            }
        }
        let up = if held {
            let _ = air.arc.advance(dt, n.gravity, n.fall_max);
            0.0
        } else {
            air.arc.advance(dt, n.gravity, n.fall_max)
        };
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
        if let Some(rt) = root {
            add_root(actor, disp, rt);
        }
        r
    }

    /// KH2's root travel (`kh2slots::air_root`, KH2's y up) on KH1's air move this frame (y down),
    /// logged for the first few.
    unsafe fn add_root(actor: usize, disp: *mut [f32; 4], d: [f32; 3]) {
        let m = &mut *disp;
        m[0] += d[0];
        m[1] -= d[1];
        m[2] += d[2];
        static N: AtomicU32 = AtomicU32::new(0);
        if super::super::log_on(super::super::LogCat::Takeover) && N.fetch_add(1, Ordering::Relaxed) < 20 {
            super::super::log!(Takeover, "KH2's root travel with no gravity on KH1's air state (KH2 motion {:?}): {:.1} up, {:.1} along", super::super::kh2slots::playing_motion(actor), d[1], d[0].hypot(d[2]));
        }
    }
}
