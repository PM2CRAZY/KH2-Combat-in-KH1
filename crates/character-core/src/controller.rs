//! The 60 Hz character state machine.
//!
//! The host calls [`Character::step`] once per 60 Hz frame with the input
//! and a little world information, moves the body by
//! [`Output::displacement`] with its own collision, and reports
//! `grounded` back on the next frame. Hitboxes come out as data; the host
//! places shapes, resolves hits (e.g. with `combat-core`) and calls
//! [`Character::register_hit`] when the current swing connects.
//!
//! Like KH2, input and action logic run every frame but velocities are
//! recomputed on a 30 Hz physics tick (every second frame) and held in
//! between. See `docs/movement-model.md`.

use std::collections::HashMap;

use crate::clip::{Clip, EventKind, WindowKind};
use crate::combo::{condition, select, Choice, ComboRow, Situation, TargetGeom, FLAG_JUMP_ADJUST};
use crate::params::{Abilities, AttackParams, ComboRules, GrowthLevel, MoveParams};

/// Motion ids used by the controller (KH2 player moveset numbering).
pub mod motion {
    pub const IDLE: u16 = 0;
    pub const WALK: u16 = 1;
    pub const RUN: u16 = 2;
    pub const JUMP: u16 = 3;
    pub const FALL: u16 = 4;
    pub const LAND: u16 = 5;
    /// The guard's repel: an attack met his guard (pushed back, Counterguard's window).
    pub const GUARD: u16 = 42;
    /// Guard pressed (combo row 11's motion, A322: guard window frames 6..37, the blade's trail;
    /// exe: the action button standing tries row 11). Motion 42 is the block, not the press (the
    /// user: our guard was the animation he plays when his guard is hit).
    pub const GUARD_PRESS: u16 = 173;
    pub const HIGH_JUMP: u16 = 201;
    pub const AERIAL_DODGE: u16 = 202;
    pub const QUICK_RUN: u16 = 203;
    pub const QUICK_RUN_END: u16 = 204;
    pub const GLIDE: u16 = 205;
    pub const GLIDE_TURN_A: u16 = 206;
    pub const GLIDE_TURN_B: u16 = 207;
    /// Gliding in place (ACTION_GLIDEHOVER's motion, exe `0x140422B20`).
    pub const GLIDE_HOVER: u16 = 208;
    pub const DODGE_ROLL: u16 = 209;
    pub const HANG: u16 = 7;
    /// Repelled by a wall (or a guard) on the ground / in the air.
    pub const REPELLED: u16 = 41;
    pub const REPELLED_AIR: u16 = 43;
    pub const CLIMB: u16 = 8;
    /// Aerial Recovery's flip (exe).
    pub const AIR_RECOVERY: u16 = 40;
}

/// Everything the controller needs to know about a character's moves.
#[derive(Clone, Debug, Default)]
pub struct Moveset {
    pub clips: HashMap<u16, Clip>,
    pub combo_rows: Vec<ComboRow>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Button {
    /// Went down this frame.
    pub pressed: bool,
    pub held: bool,
}

impl Button {
    pub fn from_held(prev: bool, now: bool) -> Button {
        Button { pressed: now && !prev, held: now }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    /// Desired direction on the ground plane in world space (x, z), with
    /// length 0..=1 as stick tilt.
    pub stick: [f32; 2],
    pub jump: Button,
    pub attack: Button,
    /// KH2's Square / "action" button: guard, dodge roll, quick run, aerial dodge.
    pub action: Button,
    pub lock: Button,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetInfo {
    pub id: u32,
    /// Feet position.
    pub pos: [f32; 3],
    /// Collision radius and height (used by the combo table's range checks).
    pub radius: f32,
    pub height: f32,
}

/// A grabbable edge: the top edge of a wall, from `a` to `b`, with `out`
/// the horizontal unit normal pointing away from the wall.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ledge {
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub out: [f32; 2],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Env<'a> {
    /// Character feet position.
    pub pos: [f32; 3],
    pub grounded: bool,
    pub height_above_ground: f32,
    pub targets: &'a [TargetInfo],
    /// Ledges near the character (the host may pass all of them).
    pub ledges: &'a [Ledge],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateName {
    Idle,
    Walk,
    Run,
    Air,
    Jump,
    Land,
    Glide,
    DodgeRoll,
    QuickRun,
    AerialDodge,
    Guard,
    Attack,
    Hang,
    Climb,
    Hurt,
    Blown,
    Magic,
    /// A motion the host asked for (Drive, form arrival, reaction command).
    Scripted,
}

/// The combo slot a spell took (`Character::spell_slot`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellSlot {
    /// A step of the combo (its first, or one before its last).
    Step,
    /// A finisher slot; `more`: Finishing Plus leaves another finisher after it.
    Finisher { more: bool },
}

/// The combo carried over a move of the host's own (`Character::combo_state`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComboState {
    pub count: u8,
    pub finishers: u8,
    pub hit: bool,
    /// The next step is the combo's next finisher.
    pub finisher_next: bool,
}

/// A spell's cast motions (MAGC): ground, air, and the combo-finisher one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CastMotions {
    pub ground: u16,
    pub air: u16,
    pub finisher: Option<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActiveHitbox {
    /// Unique per swing and window; feed to a hit tracker.
    pub swing: u64,
    pub attack: u16,
    /// Collision group of the shapes to use.
    pub group: u8,
    /// The group belongs to the weapon model (else the character model).
    /// 0: the character's own shapes; 1, 2, ...: its weapons (right hand
    /// first).
    pub weapon: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Output {
    pub displacement: [f32; 3],
    /// Yaw in radians; forward = (sin yaw, cos yaw) on (x, z).
    pub facing: f32,
    pub state: StateName,
    /// Motion id being played (for debug/animation), if any.
    pub motion: u16,
    pub clip_frame: u32,
    pub hitboxes: Vec<ActiveHitbox>,
    pub invulnerable: bool,
    pub guarding: bool,
    /// Live guard windows; their shapes come from the collision group.
    pub guard_boxes: Vec<ActiveHitbox>,
    /// The motion's Keyblade-trail window is open (the guard's swish, the swings').
    pub weapon_trail: bool,
    pub locked: Option<u32>,
    /// KH2 combo counter (1 on the first swing; only advances on swings
    /// that follow a connecting one).
    pub combo_count: u8,
    /// A combo is going and has steps left (a spell now would be one of them: `combo_live`).
    pub combo_live: bool,
    pub events: Vec<EventKind>,
}

/// KH2's jump curve. Rising: `y = h * sin(pi/2 * sin(pi/2 * t/T))` with
/// `T = sqrt(2h/g)`; after the apex, `y = h - g/2 * (t - T)^2`. `t` is in
/// 60 Hz frames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arc {
    pub height: f32,
    pub t: f32,
    pub uptime: f32,
    /// Releasing jump early cuts this arc short (button jumps and Aerial
    /// Dodge; not attack hops).
    pub holdable: bool,
}

impl Arc {
    pub fn new(height: f32, gravity: f32) -> Arc {
        let h = height.max(0.0);
        Arc { height: h, t: 0.0, uptime: uptime(h, gravity), holdable: false }
    }

    /// Falling from rest (walked off a ledge, glide ended, ...).
    pub fn fall() -> Arc {
        Arc { height: 0.0, t: 0.0, uptime: 0.0, holdable: false }
    }

    pub fn y(&self, t: f32, gravity: f32) -> f32 {
        if t < self.uptime {
            let half_pi = std::f32::consts::FRAC_PI_2;
            self.height * (half_pi * (half_pi * t / self.uptime).sin()).sin()
        } else {
            let d = t - self.uptime;
            self.height - 0.5 * gravity * d * d
        }
    }

    pub fn rising(&self) -> bool {
        self.t < self.uptime
    }

    /// Advances by `dt` frames and returns the height change, with the
    /// downward step limited to `fall_max` per frame.
    pub fn advance(&mut self, dt: f32, gravity: f32, fall_max: f32) -> f32 {
        let dy = self.y(self.t + dt, gravity) - self.y(self.t, gravity);
        self.t += dt;
        dy.max(-fall_max * dt)
    }
}

/// When jump is released early, KH2 replaces the rest of the rise with a hop
/// this high from the current height: g x 0.5 x 5 x 5 (checked in the exe).
pub fn release_hop(gravity: f32) -> f32 {
    gravity * 0.5 * 5.0 * 5.0
}
/// The cut only happens after this many frames of the rise...
pub const HOLD_MIN_FRAMES: f32 = 6.0;
/// ...and more than this many frames before the apex.
pub const HOLD_APEX_MARGIN: f32 = 5.0;

/// Frames for a jump of `height` to reach its apex.
pub fn uptime(height: f32, gravity: f32) -> f32 {
    if height <= 0.0 {
        0.0
    } else {
        (2.0 * height / gravity.max(1e-6)).sqrt()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ActionKind {
    Jump { high: bool },
    Land,
    DodgeRoll,
    QuickRun,
    AerialDodge,
    Guard,
    Attack { row: ComboRow },
    /// Hanging from a ledge: feet at `at`, wall normal `out`. `armed` once
    /// the stick has stopped pushing toward the wall after the grab.
    Hang { at: [f32; 3], out: [f32; 2], armed: bool },
    Climb,
    /// Taking a hit (flinch, air flinch, getting up, guard repel).
    Hurt,
    /// Launched by a hit; `step` is the per-frame displacement of the
    /// current physics tick.
    Blow { blow: crate::reaction::Blow },
    /// Casting a spell (the motion's magic-cast event releases it).
    Magic,
    /// A motion the host asked for (Drive, form arrival, revert): plays to
    /// the end, takes no input.
    /// `hold`: held where he is in the air (no gravity, no drift), a Drive started mid-air.
    Scripted { invulnerable: bool, hold: bool },
}

#[derive(Clone, Debug)]
struct Action {
    kind: ActionKind,
    motion: u16,
    frame: u32,
    swing: u64,
    /// Quick Run: frames left before braking hard, and the current brake.
    timer: f32,
    brake: f32,
    /// Ground attack lunge speed (until the first hitbox opens).
    lunge: f32,
}

#[derive(Clone, Debug)]
enum Mode {
    Free,
    Glide,
    Action(Action),
}

/// Wall recoil: push-back distance on the ground, hop height in the air
/// (exe constant 50).
const WALL_HOP: f32 = 50.0;
/// Frames into a launch before Aerial Recovery / Retaliating Slash can be used (exe: 16).
const RECOVERY_AFTER: u32 = 16;
/// Quick Run's fewest invulnerable frames (KH2 gives LV1 none).
pub const QUICK_RUN_STAR_MIN: f32 = 6.0;

/// Aerial Dodge's invulnerable frames before its level's (2 more per level).
pub const AIR_DODGE_STAR: f32 = 8.0;

/// Aerial Recovery's hop height and speed along the stick (`plyr.blow_recovery_h` / `_v`).
const AIR_RECOVERY_HOP: f32 = 100.0;
const AIR_RECOVERY_SPEED: f32 = 10.0;
/// Combo rows only reached from a state (PTYA kind 0), and the abilities they need.
/// Counterguard: attack during the guard's repel (motion 42), as the exe's attack-press
/// handler does (`current motion == 0x2A` → row 0x27). Retaliating Slash: attack while
/// launched (ours; the exe checks it in a separate routine whose row id comes in a register).
const COUNTERGUARD_ROW: u8 = 39;
/// The guard's combo row (exe: tried by the action button while standing).
const GUARD_ROW: u8 = 11;
/// KH2's Square moves in a combo, in the order tried: (PTYA row, ability, in the air).
/// Finishing Leap (row 37, ability 95), Upper Slash (12, 18), Horizontal Slash (38, 99).
const SQUARE_MOVES: [(u8, u16, bool); 3] = [(37, 95, false), (12, 18, false), (38, 99, true)];
const RETALIATING_SLASH_ROW: u8 = 40;
/// Hit-back starting speed per unit of distance (exe: 0.1).
const HIT_BACK_SPEED: f32 = 0.1;

/// Frames a landing holds Sora where he touched down before the stick moves him again (the
/// user's KH2 video: touchdown, then the landing crouch for about 16 frames at 60 fps before he
/// runs off; it was 0, KH1 had none). Buttons still work from the motion's reaction window.
const LAND_HOLD: u32 = 14;

/// Frames after a ground swing or a spell ends in which the combo still goes on
/// (`combo_live`): just the hand-over (a host's move handing back, an input taken the next
/// frame). Back in idle the combo is over (the user: "the intended behavior is for sora to have
/// his combo reset if he returns to idle"; it was 60 frames, for spells picked from KH1's menu).
const COMBO_GRACE: u32 = 2;

/// A finisher's window for the next finisher (Finishing Plus): its motion's finisher range, or,
/// for a motion with none (Wisdom Form's finisher shot, its finishing casts), its last frames
/// (the next finisher, swing or spell, follows it straight on; the user: Wisdom's finishing
/// magic couldn't take Finishing Plus's second finisher).
fn finisher_window(clip: &Clip, f: u32) -> bool {
    if clip.windows.iter().any(|w| matches!(w.kind, WindowKind::Finisher)) {
        clip.has(f, |k| matches!(k, WindowKind::Finisher))
    } else {
        f + FINISHER_END >= clip.length
    }
}

/// The last frames of a finisher motion without a finisher range that take the next finisher.
const FINISHER_END: u32 = 6;

/// Physics tick length in frames: KH2 recomputes velocities at 30 Hz.
const DT: f32 = 2.0;
const DEAD_ZONE: f32 = 0.1;

/// One controllable character.
#[derive(Clone, Debug)]
pub struct Character {
    pub moveset: Moveset,
    pub params: MoveParams,
    /// Growth ability levels 1..=4 at index 0..=3.
    pub growth: Vec<GrowthLevel>,
    pub attack: AttackParams,
    /// How hits push and launch this character.
    pub reaction: crate::reaction::ReactionParams,
    pub abilities: Abilities,
    pub combo_rules: ComboRules,
    /// Which button starts and holds Glide in mid-air.
    pub glide_button: GlideButton,
    /// An air cast that reaches the ground while it plays ends there in the landing (as Final
    /// Form's air Reflect lands). It still holds in its motion's no-gravity window first
    /// (KH2's), and falls after it.
    pub air_magic_falls: bool,
    /// With `air_magic_falls`, gravity's share while an air cast plays (1 = full). KH1's air
    /// spells stop his fall when cast and then sink slowly while the spell comes out; KH2's
    /// full gravity from rest dropped Reflect too fast (the user).
    pub air_magic_gravity: f32,
    /// With `air_magic_falls`, the fastest an air cast falls (units per frame; `None`: the
    /// character's own `fall_max`). A host can give KH1's (its air spells fall at KH1's gravity
    /// and speed: the KH1 port).
    pub air_magic_fall_max: Option<f32>,
    /// Square in a combo (its combo or finisher window) goes into this finisher row, on the ground
    /// and in the air (`(ground, air)` row ids), if set: a host's form (the KH1 port's Valor, the
    /// user: "valor forms ability to go into a finisher by pressing square during a combo").
    pub square_finishers: Option<(u8, u8)>,
    /// The air cast playing holds as Final Form's air Reflect does (`hold_air_cast`): frames of
    /// no gravity from its start, no drift; then a plain fall.
    cast_hold: Option<u32>,
    facing: f32,
    vel_h: [f32; 2],
    /// The ground speed the stick asks for (with inertia, `vel_h` eases toward it): what walk,
    /// run or idle shows.
    steer_speed: f32,
    vel_y: f32,
    airborne: bool,
    arc: Option<Arc>,
    tick: u64,
    mode: Mode,
    swing_counter: u64,
    lock: Option<u32>,
    /// A kind-0 combo row the next attack must be (Counterguard, Retaliating Slash).
    forced_row: Option<u8>,
    /// Each attack starts facing the locked-on target (a host's choice; off = the game's turn
    /// windows only).
    face_lock: bool,
    /// The combo keeps one direction (a host's choice, the KH1 port's user: "in kh2 sora will
    /// actually stay locked in one direction during a combo"): no turning during a swing by the
    /// stick, and by the motions' turn windows only toward a target (a reticle on it); a swing
    /// that follows one in the combo keeps its facing unless a target is there to snap to at its
    /// start; the stick only aims a combo's first swing (after idle or a fall).
    combo_direction_lock: bool,
    /// No lock: aim at the nearest target (the game's auto-target). Off: aim only at the lock;
    /// the rows that look at every enemy still see them all (`set_auto_target`).
    auto_target: bool,
    /// Set when a jump starts with Aerial Dodge owned; cleared on use.
    air_dodge_ready: bool,
    /// Glide banking shown (0 level, 1 or -1 turning), held with some hysteresis.
    glide_bank: i8,
    /// Gliding in place (KH2's ACTION_GLIDEHOVER, motion 208): `None` until the glide's first
    /// frame decides it (the stick neutral then: hover).
    glide_hover: Option<bool>,
    /// Jump button still held since the jump started / released early.
    jump_held: bool,
    jump_released: bool,
    count: u8,
    finishers: u8,
    /// The get-up motion after landing from a launch, while it plays (Hyper Healing).
    get_up: Option<u16>,
    /// The current swing has connected.
    hit: bool,
    /// The next swing goes on with the combo (`set_combo_state`: after a spell).
    combo_carry: bool,
    /// Frames since an attack last played (0 while one does): a spell cast this soon after a
    /// swing on the ground still comes mid-combo (`combo_live`).
    since_swing: u32,
    /// The Magic action playing was cast as a finisher of the combo (`spell_slot`).
    cast_finisher: bool,
    /// The combo's next step is a finisher (a spell took a finisher slot and Finishing Plus
    /// leaves another): `try_attack` takes it as the next finisher.
    carry_finisher: bool,
    /// Frames since a dodge roll or Quick Run last played (an aerial dodge in the fall off a
    /// ledge it rolled from).
    since_roll: u32,
    /// Hang point to move onto this frame (a ledge was just grabbed).
    snap: Option<[f32; 3]>,
    /// Hit-back push: direction and speed (units/frame), decaying x0.9 per
    /// frame and dropped below 1 (exe).
    push: Option<([f32; 2], f32)>,
    /// Frames left without air commands after a ceiling bonk (exe).
    ceiling_lock: f32,
}

/// The mid-air button that glides (a host binding choice).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GlideButton {
    /// Jump again in mid-air, once Aerial Dodge is used up or not owned
    /// (the exe's rule).
    #[default]
    Jump,
    /// The action button (guard/roll; Square on a PlayStation pad). Jump in
    /// mid-air then only Aerial Dodges.
    Action,
}

const TAU: f32 = std::f32::consts::TAU;

fn wrap(a: f32) -> f32 {
    let mut a = (a + std::f32::consts::PI) % TAU;
    if a < 0.0 {
        a += TAU;
    }
    a - std::f32::consts::PI
}

/// Glide banking: start banking with this much turn still to make (radians, 12 degrees, or two
/// frames of turning if that is more) and level out below 4 degrees.
const GLIDE_BANK_IN: f32 = 0.21;
const GLIDE_BANK_OUT: f32 = 0.07;
/// Horizontal speed (per frame) under which a glide with the stick let go turns into the hover.
const GLIDE_HOVER_SPEED: f32 = 2.0;
/// The hover's horizontal speed each physics tick (exe `0x140422930`, `0x1405866E0`).
const GLIDE_HOVER_DRAG: f32 = 0.9;

fn turn_toward(from: f32, to: f32, rate: f32) -> f32 {
    let d = wrap(to - from);
    if d.abs() <= rate {
        to
    } else {
        wrap(from + rate * d.signum())
    }
}

fn yaw_of(dir: [f32; 2]) -> f32 {
    dir[0].atan2(dir[1])
}

fn len2(v: [f32; 2]) -> f32 {
    (v[0] * v[0] + v[1] * v[1]).sqrt()
}

fn norm(v: [f32; 2]) -> [f32; 2] {
    let l = len2(v);
    if l > 1e-6 { [v[0] / l, v[1] / l] } else { [0.0; 2] }
}

fn scale(v: [f32; 2], s: f32) -> [f32; 2] {
    [v[0] * s, v[1] * s]
}

#[derive(Default)]
struct FrameOut {
    root: [f32; 3],
    hitboxes: Vec<ActiveHitbox>,
    invulnerable: bool,
    guarding: bool,
    guard_boxes: Vec<ActiveHitbox>,
    weapon_trail: bool,
    no_gravity: bool,
    motion: u16,
    clip_frame: u32,
    events: Vec<EventKind>,
}

impl Character {
    pub fn new(moveset: Moveset, params: MoveParams, growth: Vec<GrowthLevel>, attack: AttackParams) -> Character {
        Character {
            moveset,
            params,
            growth,
            attack,
            reaction: crate::reaction::ReactionParams::default(),
            abilities: Abilities::default(),
            combo_rules: ComboRules::default(),
            glide_button: GlideButton::default(),
            air_magic_falls: false,
            air_magic_gravity: 1.0,
            air_magic_fall_max: None,
            square_finishers: None,
            cast_hold: None,
            facing: 0.0,
            vel_h: [0.0; 2],
            steer_speed: 0.0,
            vel_y: 0.0,
            airborne: false,
            arc: None,
            tick: 0,
            mode: Mode::Free,
            swing_counter: 0,
            lock: None,
            forced_row: None,
            face_lock: false,
            combo_direction_lock: false,
            auto_target: true,
            air_dodge_ready: false,
            glide_bank: 0,
            glide_hover: None,
            count: 0,
            finishers: 0,
            get_up: None,
            hit: false,
            combo_carry: false,
            since_swing: u32::MAX,
            cast_finisher: false,
            carry_finisher: false,
            since_roll: u32::MAX,
            jump_held: false,
            jump_released: false,
            snap: None,
            push: None,
            ceiling_lock: 0.0,
        }
    }

    pub fn facing(&self) -> f32 {
        self.facing
    }

    pub fn set_facing(&mut self, yaw: f32) {
        self.facing = wrap(yaw);
    }

    /// Tell the controller the current swing connected. In KH2 only a swing
    /// that follows a connecting swing advances the combo.
    pub fn register_hit(&mut self) {
        self.hit = true;
    }

    /// Tell the controller its weapon struck a wall (the attack's ray hit
    /// the background). As in the game: the swing counts as connected (the
    /// combo goes on), and Sora is repelled. On the ground he plays the
    /// repelled motion and is pushed back 50 units' worth (5/frame decaying
    /// by 0.9); in the air he hops 50 up while drifting back.
    /// Returns false if not attacking.
    pub fn register_wall_hit(&mut self) -> bool {
        let Mode::Action(act) = &mut self.mode else { return false };
        if !matches!(act.kind, ActionKind::Attack { .. }) || matches!(act.motion, motion::REPELLED | motion::REPELLED_AIR) {
            return false;
        }
        self.hit = true;
        let back = [-self.facing.sin(), -self.facing.cos()];
        // (Only with a repelled motion to play: without one (Final Form's moveset has none) the
        // swing went back to its frame 0, met the wall again, and so on for ever, the KH1 port's
        // user: "trying to do the ground combo repeats the same moves over and over ... and it
        // soft locks you". Then he is only pushed back and the swing goes on.)
        let air_clip = self.airborne && self.moveset.clips.contains_key(&motion::REPELLED_AIR);
        let ground_clip = !self.airborne && self.moveset.clips.contains_key(&motion::REPELLED);
        if !air_clip && !ground_clip {
            self.push = Some((back, WALL_HOP * HIT_BACK_SPEED));
            return true;
        }
        act.frame = 0;
        act.lunge = 0.0;
        if air_clip {
            act.motion = motion::REPELLED_AIR;
            self.arc = Some(Arc::new(WALL_HOP, self.params.gravity));
            self.vel_h = scale(back, WALL_HOP * self.params.hit_back_to_jump);
            self.vel_y = 0.0;
        } else {
            act.motion = motion::REPELLED;
            self.vel_h = [0.0; 2];
            self.push = Some((back, WALL_HOP * HIT_BACK_SPEED));
        }
        true
    }

    /// Whether a spell could be cast right now (spec: magic follows the combo's own rules):
    /// - free (a new combo, or the step after one of a going combo), or landing;
    /// - in a swing's combo window, when it would take its next swing (Sora's swings have no
    ///   magic window of their own; s180, the user: "too much a pause before magic");
    /// - in a finisher (a swing's or a spell's) only as the next finisher, with Finishing Plus
    ///   leaving one (its finisher window): after the combo's last finisher nothing new starts
    ///   until its animation has ended (the user: Reflect followed a finisher as an action of
    ///   its own);
    /// - in a spell's own "combo magic" window (a spell into a spell).
    pub fn can_cast(&self) -> bool {
        if self.ceiling_lock > 0.0 {
            return false;
        }
        match &self.mode {
            Mode::Free => !(self.combo_carry && self.carry_finisher && self.finishers >= self.max_finishers()),
            Mode::Action(a) => self.cast_window(a, a.frame.saturating_sub(1)),
            Mode::Glide => false,
        }
    }

    /// Whether the action playing lets a spell in at frame `f` (`can_cast`).
    fn cast_window(&self, a: &Action, f: u32) -> bool {
        let has = |p: &dyn Fn(&WindowKind) -> bool| self.clip(a.motion).map_or(false, |c| c.has(f, |k| p(k)));
        // (As a swing's next finisher: only after one that counts, `try_attack`.)
        let next_finisher = || self.finishers < self.max_finishers() && self.swing_counts() && self.clip(a.motion).is_some_and(|c| finisher_window(c, f));
        match &a.kind {
            ActionKind::Attack { row } if row.is_finisher() => next_finisher(),
            ActionKind::Attack { .. } => has(&|k| matches!(k, WindowKind::ComboMagic | WindowKind::Combo)),
            ActionKind::Magic if self.cast_finisher => next_finisher(),
            ActionKind::Magic => has(&|k| matches!(k, WindowKind::ComboMagic)),
            // Not in the landing (the user: "reflect can also be used during the landing
            // animation, we need to disable this"); once it is over.
            _ => false,
        }
    }

    /// Whether a spell chosen now can come out in the move playing, now or at a later window of
    /// it (KH2's buffer: into the move's next combo window). Not in a finisher with no next
    /// finisher to cast as (the user: a spell chosen during a finisher came out after it,
    /// however long after; "there shouldn't be any buffer system like that for magic"), nor in a
    /// move with no window for it: the choice is dropped, and only once he is back in idle (or
    /// in a window) does a spell come.
    pub fn magic_later(&self) -> bool {
        if self.can_cast() {
            return true;
        }
        match &self.mode {
            Mode::Action(a) => {
                let len = self.clip(a.motion).map_or(0, |c| c.length);
                (a.frame..len).any(|f| self.cast_window(a, f))
            }
            _ => false,
        }
    }

    /// The combo slot a spell cast now takes (spec: each spell is one step of the combo string,
    /// under the same rules as a swing): `None` when it can't take one (after the combo's last
    /// finisher). Doesn't change anything; `take_spell_slot` does.
    fn spell_slot(&self) -> Option<(SpellSlot, u8, u8)> {
        let max = self.max_count(self.airborne);
        let more = |finishers: u8| finishers < self.max_finishers();
        if self.finisher_now() {
            // The next finisher of the combo (Finishing Plus), as a finisher swing's attack
            // would be (`try_attack`'s finisher branch).
            // (Only after a finisher that counts, as `try_attack` takes the next finisher swing.)
            if self.finishers >= self.max_finishers() || !self.swing_counts() {
                return None;
            }
            let (c, fin) = (self.count.saturating_add(1), self.finishers + 1);
            return Some((SpellSlot::Finisher { more: more(fin) }, c, fin));
        }
        if self.combo_live() {
            // The combo's next step; it counts as a swing would (a connected one, Combo Master,
            // in the air), and on its last step it is the combo's (first) finisher.
            let c = if self.swing_counts() { self.count.saturating_add(1) } else { self.count.max(1) };
            let slot = if c >= max { SpellSlot::Finisher { more: more(self.finishers) } } else { SpellSlot::Step };
            return Some((slot, c, self.finishers.max(1)));
        }
        // No combo going: the spell is a new combo's first step.
        let slot = if 1 >= max { SpellSlot::Finisher { more: more(1) } } else { SpellSlot::Step };
        Some((slot, 1, 1))
    }

    /// Takes the slot `spell_slot` gave: the counters move, and the combo goes on after the
    /// spell unless it was the combo's last finisher (the next step then a finisher too while
    /// Finishing Plus leaves one).
    fn take_spell_slot(&mut self, (slot, count, finishers): (SpellSlot, u8, u8)) {
        self.count = count;
        self.finishers = finishers;
        // The step after a spell counts on from it (as after a connected swing).
        self.hit = true;
        self.combo_carry = !matches!(slot, SpellSlot::Finisher { more: false });
        self.carry_finisher = matches!(slot, SpellSlot::Finisher { more: true });
        self.cast_finisher = matches!(slot, SpellSlot::Finisher { .. });
        self.since_swing = 0;
    }

    /// Whether the move playing now is a finisher of the combo (a finisher swing or a spell cast
    /// as one), or one just ended whose combo goes on to the next finisher (`carry_finisher`).
    fn finisher_now(&self) -> bool {
        match &self.mode {
            Mode::Action(Action { kind: ActionKind::Attack { row }, .. }) => row.is_finisher(),
            Mode::Action(Action { kind: ActionKind::Magic, .. }) => self.cast_finisher,
            _ => self.combo_carry && self.carry_finisher,
        }
    }

    /// Starts a cast: the air motion when airborne, the finisher motion when it takes a
    /// finisher slot of the combo (`spell_slot`), else the ground one. Returns the motion
    /// started, or `None` if casting isn't possible now. The spell itself is released by the
    /// motion's magic-cast event (`EventKind::MagicCast` with the motion's own id).
    pub fn cast(&mut self, m: CastMotions) -> Option<u16> {
        if !self.can_cast() {
            return None;
        }
        let slot = self.spell_slot()?;
        let motion = if self.airborne {
            m.air
        } else if matches!(slot.0, SpellSlot::Finisher { .. }) {
            m.finisher.unwrap_or(m.ground)
        } else {
            m.ground
        };
        let clip = self.clip(motion)?;
        // A cast that moves (Wisdom Form's, an allow-movement range) keeps his speed and his
        // fall (the user: no velocity lost in Wisdom's air moves).
        let moving = clip.windows.iter().any(|w| matches!(w.kind, WindowKind::AllowMovement));
        self.take_spell_slot(slot);
        if !moving {
            self.vel_h = [0.0; 2];
            if self.airborne {
                self.vel_y = 0.0;
                self.arc = None;
            }
        }
        self.start_action(ActionKind::Magic, motion);
        Some(motion)
    }

    /// The air cast just started (`cast`) moves as Final Form's air Reflect (the KH1 port's user:
    /// "make it so that base air reflect and wisdom air reflect copy the physics of final forms
    /// reflect but keep their respected animations"): stopped where he is, no gravity for
    /// `frames` from its start (Final's D373: no-gravity range 0..54), no steering, then a plain
    /// fall (no `air_magic_gravity` sink). Nothing on the ground.
    pub fn hold_air_cast(&mut self, frames: u32) {
        if !self.airborne || !matches!(self.mode, Mode::Action(Action { kind: ActionKind::Magic, .. })) {
            return;
        }
        self.cast_hold = Some(frames);
        self.vel_h = [0.0; 2];
        self.vel_y = 0.0;
        self.arc = None;
    }

    /// Takes a hit: the game's generic reaction (flinch, air flinch,
    /// launch, push), as for enemies. Interrupts whatever was going on.
    pub fn take_hit(&mut self, hit: &crate::reaction::Hit) {
        use crate::reaction::Response;
        let clips = &self.moveset.clips;
        let r = crate::reaction::respond(hit, self.forward(), self.airborne, |m| clips.contains_key(&m), &self.reaction);
        match r {
            Response::None => {}
            Response::Push { distance } => self.hit_back(hit.dir, distance),
            Response::Flinch { motion, push } => {
                self.reset_combo();
                self.arc = None;
                self.vel_h = [0.0; 2];
                self.start_action(ActionKind::Hurt, motion);
                if let Some(d) = push {
                    self.hit_back(hit.dir, d);
                }
            }
            Response::AirFlinch { motion, face, hop_speed } => {
                self.reset_combo();
                self.facing = yaw_of(face);
                self.arc = Some(Arc::new(WALL_HOP, self.params.gravity));
                self.airborne = true;
                self.vel_h = scale(hit.dir, hop_speed);
                self.vel_y = 0.0;
                self.start_action(ActionKind::Hurt, motion);
            }
            Response::Blow { motion, face, vel_h, up } => {
                self.reset_combo();
                self.facing = yaw_of(face);
                self.arc = None;
                self.push = None;
                self.airborne = true;
                self.vel_h = [0.0; 2];
                self.vel_y = 0.0;
                let blow = crate::reaction::Blow::new(vel_h, up, &self.reaction);
                self.start_action(ActionKind::Blow { blow }, motion);
            }
        }
    }

    /// An attack of Sora's met another attack or a guard (the game's party
    /// clash reaction). `mine_self` is Sora's attack's ATKP "refact self",
    /// `other_other` the other attack's "refact other". 2 on either side:
    /// nothing happens. Otherwise, on the ground with either nonzero: guard
    /// repel (face `toward`, play the guard motion, pushed back 200). Else
    /// the wall-hit recoil.
    pub fn clash(&mut self, mine_self: u8, other_other: u8, toward: [f32; 2]) {
        self.hit = true;
        if mine_self == 2 || other_other == 2 {
            return;
        }
        if !self.airborne && (mine_self != 0 || other_other != 0) && self.clip(motion::GUARD).is_some() {
            self.facing = yaw_of(toward);
            self.vel_h = [0.0; 2];
            self.start_action(ActionKind::Hurt, motion::GUARD);
            self.hit_back([-toward[0], -toward[1]], 200.0);
            return;
        }
        if !self.register_wall_hit() {
            self.hit_back([-toward[0], -toward[1]], WALL_HOP);
        }
    }

    /// Starts a hit-back push of `distance` along `dir` (x, z): initial
    /// speed distance x 0.1, decaying x0.9 per frame (exe).
    pub fn hit_back(&mut self, dir: [f32; 2], distance: f32) {
        self.push = Some((norm(dir), distance * HIT_BACK_SPEED));
    }

    /// Already falling when the host hands him over (out of a move of the host's own: a swing
    /// off a bar): in the air with that speed, per frame (`vel_y` up; negative falls), so the
    /// fall goes on as it was instead of starting from rest.
    pub fn carry_into_air(&mut self, vel_h: [f32; 2], vel_y: f32) {
        self.mode = Mode::Free;
        self.airborne = true;
        self.vel_h = vel_h;
        let g = self.params.gravity;
        // Rising (taken over mid-jump): the rest of the rise, as high as that speed carries.
        if vel_y > 0.0 && g > 0.0 {
            self.arc = Some(Arc::new(vel_y * vel_y / (2.0 * g), g));
            self.vel_y = vel_y;
            return;
        }
        let mut arc = Arc::fall();
        if g > 0.0 {
            arc.t = (-vel_y).max(0.0) / g;
        }
        self.arc = Some(arc);
        self.vel_y = vel_y.min(0.0);
    }

    /// The Aerial Dodge is still there for this fall (a jump press in the air would be the dodge,
    /// not a glide).
    pub fn air_dodge_available(&self) -> bool {
        self.air_dodge_ready
    }

    /// The Aerial Dodge is there for this fall (taken over from the host's own jump).
    pub fn allow_air_dodge(&mut self) {
        self.air_dodge_ready = self.abilities.aerial_dodge > 0;
    }

    /// On the ground out of a move of the host's own that slid him (a Drive Form's cast of a KH1
    /// spell): free, moving at `vel_h` (per frame) so his momentum goes on from there (the user:
    /// "we don't want sora to have his momentum halted"), eased by the ground inertia toward the
    /// stick as usual.
    pub fn carry_on_ground(&mut self, vel_h: [f32; 2]) {
        self.mode = Mode::Free;
        self.airborne = false;
        self.arc = None;
        self.vel_y = 0.0;
        self.vel_h = vel_h;
        self.steer_speed = len2(vel_h);
    }

    /// Just touched down out of a move of the host's own (a spell cast in the air): standing,
    /// in the landing (its short hold, a jump out of it at any frame).
    pub fn land_now(&mut self) {
        self.airborne = true;
        self.mode = Mode::Free;
        self.land();
        self.airborne = false;
        self.vel_h = [0.0; 2];
    }

    /// Where the combo stands, to carry it over a move of the host's own (a spell of KH1's: the
    /// combo goes on after it).
    pub fn combo_state(&self) -> ComboState {
        ComboState { count: self.count, finishers: self.finishers, hit: self.hit, finisher_next: self.carry_finisher }
    }

    /// The apex heights of a jump and of a High Jump with the abilities now.
    pub fn jump_heights(&self) -> (f32, f32) {
        let n = self.params.jump_height;
        (n, self.growth(self.abilities.high_jump).map_or(n, |g| g.high_jump_height))
    }

    /// A combo is going (`combo_live`), and frames since its last swing started: a host that
    /// lends the character to the controller only for its moves keeps it through a combo.
    pub fn combo_going(&self) -> (bool, u32) {
        (self.combo_live(), self.since_swing)
    }

    /// A combo is going (the user: a spell cast mid-combo inserts itself in it, as a modifier
    /// does): a swing playing, or one just ended (in the air until landing, on the ground for
    /// `COMBO_GRACE` frames: picking the spell from KH1's menu takes a moment), and its last
    /// swing not yet reached.
    fn combo_live(&self) -> bool {
        let swinging = matches!(&self.mode, Mode::Action(Action { kind: ActionKind::Attack { .. } | ActionKind::Magic, .. }));
        let recent = self.count > 0 && (self.airborne || self.since_swing <= COMBO_GRACE);
        (swinging || recent) && self.count < self.max_count(self.airborne)
    }

    /// Whether the step taken now counts on the combo's counter (as `try_attack`: a connected
    /// swing, Combo Master, or in the air).
    fn swing_counts(&self) -> bool {
        self.hit || self.abilities.combo_master || self.airborne
    }

    /// A spell of the host's own (KH1's) is cast now: it takes its combo slot as KH2's casts do
    /// (`cast`, `spell_slot`). Returns the slot it took, `None` if it couldn't take one.
    pub fn magic_step(&mut self) -> Option<SpellSlot> {
        let slot = self.spell_slot()?;
        self.take_spell_slot(slot);
        Some(slot.0)
    }

    /// Back from a move of the host's own (`combo_state` before it): the combo goes on, its next
    /// step counting on from there (a finisher if `finisher_next`).
    pub fn set_combo_state(&mut self, c: ComboState) {
        self.count = c.count;
        self.finishers = c.finishers;
        self.hit = c.hit;
        self.combo_carry = c.count > 0;
        self.carry_finisher = c.finisher_next;
        self.since_swing = 0;
    }

    /// In the air this frame (the host picks the map's air rules with it).
    pub fn airborne(&self) -> bool {
        self.airborne
    }

    /// The jump arc in the air now (none on the ground), and whether the last step was the first
    /// frame of a physics tick. The arc moves a 2-frame tick at a time and he moves half of it
    /// each frame, so after a tick's first frame the arc's time is one frame ahead of him. A host
    /// that keeps his momentum in its own physics state continues the arc from here.
    pub fn air_arc(&self) -> Option<(Arc, bool)> {
        self.arc.filter(|_| self.airborne).map(|a| (a, self.tick % 2 == 1))
    }

    /// The map stopped a rise: less than 49% of the desired climb happened
    /// (exe). A jump or launch more than 6 frames in drops from rest at the
    /// current height, loses its hold, and air commands are refused for
    /// `ceiling_lock` frames or until landing. Earlier in the rise the
    /// ceiling only blocks.
    pub fn hit_ceiling(&mut self) {
        let mut bonked = false;
        if let Some(a) = self.arc {
            if a.rising() && a.t > HOLD_MIN_FRAMES {
                self.arc = Some(Arc::fall());
                self.jump_released = false;
                self.jump_held = false;
                bonked = true;
            }
        }
        if let Mode::Action(Action { kind: ActionKind::Blow { blow }, frame, .. }) = &mut self.mode {
            if blow.vel_y > 0.0 && *frame as f32 > HOLD_MIN_FRAMES {
                blow.vel_y = 0.0;
                self.vel_y = 0.0;
                bonked = true;
            }
        }
        if bonked {
            self.vel_y = self.vel_y.min(0.0);
            self.ceiling_lock = self.params.ceiling_lock;
        }
    }

    /// Plays `motion` to its end with no control (the host's choice: a
    /// Drive, a form's arrival, a revert). Returns false if the motion
    /// doesn't exist.
    pub fn play(&mut self, motion: u16, invulnerable: bool) -> bool {
        if self.clip(motion).is_none() {
            return false;
        }
        self.reset_combo();
        self.vel_h = [0.0; 2];
        if !self.airborne {
            self.arc = None;
        }
        self.start_action(ActionKind::Scripted { invulnerable, hold: false }, motion);
        true
    }

    /// As [`play`](Self::play), and in the air held where he is while it plays (KH2's Drive
    /// taken mid-air: Sora stays frozen in that position, the user); on the ground the same as
    /// `play`. When it ends he falls from there.
    pub fn play_held(&mut self, motion: u16, invulnerable: bool) -> bool {
        if !self.play(motion, invulnerable) {
            return false;
        }
        if self.airborne {
            self.vel_y = 0.0;
            self.arc = None;
            if let Mode::Action(a) = &mut self.mode {
                a.kind = ActionKind::Scripted { invulnerable, hold: true };
            }
        }
        true
    }

    /// Whether a Drive or a Revert is taken now (the KH1 port's user: only on "end frames", no
    /// buffer): free (idle, walking, running, jumping, falling), landing, or at the end of what
    /// plays: an attack or a cast once its combo / finisher / reaction-command window is open
    /// (the move is out, the spell cast) or in its last frames. Never while hurt, launched,
    /// hanging or in a scripted motion.
    pub fn can_change_form(&self) -> bool {
        match &self.mode {
            Mode::Free | Mode::Glide => true,
            Mode::Action(a) => match a.kind {
                ActionKind::Land | ActionKind::Jump { .. } => true,
                ActionKind::Hurt | ActionKind::Blow { .. } | ActionKind::Scripted { .. } | ActionKind::Hang { .. } | ActionKind::Climb => false,
                _ => self.clip(a.motion).is_some_and(|c| {
                    let f = a.frame.saturating_sub(1);
                    f + FINISHER_END >= c.length || c.has(f, |k| matches!(k, WindowKind::Combo | WindowKind::Finisher | WindowKind::AllowReactionCommand))
                }),
            },
        }
    }

    /// A form swap (the host replaced the controller with another's data): his movement carries
    /// over from `old` (speed, the stick's speed, his fall or rise, in the air or not), so a
    /// revert doesn't halt him and one in the air goes on falling (the user). What `old` was
    /// playing is not carried: he is free (idle, walk or run by the stick; in the air, falling).
    pub fn carry_movement(&mut self, old: &Character) {
        self.vel_h = old.vel_h;
        self.steer_speed = old.steer_speed;
        self.vel_y = old.vel_y;
        self.airborne = old.airborne;
        self.arc = old.arc;
        self.tick = old.tick;
        self.jump_held = old.jump_held;
        self.jump_released = old.jump_released;
        self.mode = Mode::Free;
        if self.airborne && self.arc.is_none() {
            let mut arc = Arc::fall();
            if self.params.gravity > 0.0 {
                arc.t = (-self.vel_y).max(0.0) / self.params.gravity;
            }
            self.arc = Some(arc);
        }
    }

    /// Can a command (a reaction command) start now (exe, simplified):
    /// free movement, landing, or an action in a combo or "allow reaction
    /// command" window; not while hurt, launched, or in a scripted motion,
    /// nor during a ceiling lock.
    pub fn can_take_command(&self) -> bool {
        if self.ceiling_lock > 0.0 {
            return false;
        }
        match &self.mode {
            Mode::Free | Mode::Glide => true,
            Mode::Action(a) => match a.kind {
                ActionKind::Land => true,
                ActionKind::Hurt | ActionKind::Blow { .. } | ActionKind::Scripted { .. } | ActionKind::Hang { .. } | ActionKind::Climb => false,
                _ => self.clip(a.motion).is_some_and(|c| {
                    let f = a.frame.saturating_sub(1);
                    c.has(f, |k| matches!(k, WindowKind::Combo | WindowKind::Finisher | WindowKind::AllowReactionCommand))
                }),
            },
        }
    }

    /// Whether the moveset has a clip for `motion`.
    pub fn clip_exists(&self, motion: u16) -> bool {
        self.clip(motion).is_some()
    }

    /// Playing a motion from [`play`](Self::play).
    pub fn scripted(&self) -> bool {
        matches!(&self.mode, Mode::Action(Action { kind: ActionKind::Scripted { .. }, .. }))
    }

    /// The locked-on target.
    pub fn lock(&self) -> Option<u32> {
        self.lock
    }

    pub fn set_lock(&mut self, id: Option<u32>) {
        self.lock = id;
    }

    /// Whether each attack starts facing the locked-on target, wherever it is (our addition for
    /// hosts whose lock-on should aim every swing; the game only turns in its turn windows).
    pub fn set_face_lock(&mut self, on: bool) {
        self.face_lock = on;
    }

    /// See `combo_direction_lock`.
    pub fn set_combo_direction_lock(&mut self, on: bool) {
        self.combo_direction_lock = on;
    }

    /// Whether, with no lock, swings aim at the nearest target (the game's way, default) or at
    /// none. Either way every target in `Env::targets` counts for the combo rows that look at
    /// all enemies (Explosion's, Vicinity Break's, Magnet Burst's).
    pub fn set_auto_target(&mut self, on: bool) {
        self.auto_target = on;
    }

    pub fn combo_count(&self) -> u8 {
        self.count
    }

    /// The attack playing is a combo finisher.
    pub fn in_finisher(&self) -> bool {
        matches!(&self.mode, Mode::Action(Action { kind: ActionKind::Attack { row }, .. }) if row.is_finisher())
    }

    /// A host running a combo of its own with this character's abilities kept on top of it (the
    /// KH1 port's KH1 combo style): the combo row this controller would take for a press now.
    /// `from`: where the host's combo stands before the press (`set_combo_state`'s), the
    /// character free there, in the air or not (`air`); `None`: this controller's own combo as it
    /// is now. `square`: the action button, KH2's Square moves (Finishing Leap, Upper Slash,
    /// Horizontal Slash, a form's Square finisher), instead of Attack. Nothing of `self` changes.
    pub fn would_take(&mut self, from: Option<ComboState>, air: bool, square: bool, stick: [f32; 2], env: &Env) -> Option<ComboRow> {
        // (The moveset isn't cloned: the trial borrows it and gives it back.)
        let moveset = std::mem::take(&mut self.moveset);
        let mut t = self.clone();
        t.moveset = moveset;
        if let Some(c) = from {
            t.mode = Mode::Free;
            t.airborne = air;
            t.arc = if air { Some(Arc::fall()) } else { None };
            t.set_combo_state(c);
        }
        let press = Button { pressed: true, held: true };
        let input = if square { Input { stick, action: press, ..Input::default() } } else { Input { stick, attack: press, ..Input::default() } };
        let took = if square {
            let mut rows: Vec<u8> = Vec::new();
            if let Some((ground, air_row)) = t.square_finishers {
                rows.push(if t.airborne { air_row } else { ground });
            }
            rows.extend(SQUARE_MOVES.iter().filter(|m| m.2 == t.airborne && t.abilities.equipped.contains(&m.1)).map(|m| m.0));
            let mode = t.mode.clone();
            rows.into_iter().any(|row| {
                t.mode = mode.clone();
                t.forced_row = Some(row);
                t.try_attack(&input, env)
            })
        } else {
            t.try_attack(&input, env)
        };
        let row = match &t.mode {
            Mode::Action(Action { kind: ActionKind::Attack { row }, .. }) if took => Some(row.clone()),
            _ => None,
        };
        self.moveset = std::mem::take(&mut t.moveset);
        row
    }

    /// The combo's length now (swings to its finisher, with Combo Plus, Air Combo Plus and
    /// Negative Combo), on the ground or in the air.
    pub fn combo_length(&self, air: bool) -> u8 {
        self.max_count(air)
    }

    /// The next attack is this combo row (a host's borrow for one of KH2's ability moves:
    /// `would_take`'s row), whatever the selector would pick; cleared if it can't be used.
    pub fn force_row(&mut self, id: u8) {
        self.forced_row = Some(id);
    }

    /// Aerial Recovery now (a host's borrow out of its own knock-back, in the air): the
    /// recovery's hop and motion, as a jump while launched gives it. False without the ability
    /// or out of the air.
    pub fn air_recovery(&mut self, stick: [f32; 2]) -> bool {
        if !self.abilities.aerial_recovery || !self.airborne {
            return false;
        }
        self.start_air_recovery(stick);
        true
    }

    /// The Retaliating Slash row (Square while launched), for a host's borrow (`force_row`).
    pub fn retaliating_slash_row(&self) -> Option<u8> {
        let r = self.moveset.combo_rows.iter().find(|r| r.id == RETALIATING_SLASH_ROW)?;
        (r.ability == 0 || self.abilities.equipped.contains(&r.ability)).then_some(r.id)
    }

    fn start_air_recovery(&mut self, stick: [f32; 2]) {
        if len2(stick) > DEAD_ZONE {
            self.facing = yaw_of(stick);
            self.vel_h = scale(norm(stick), AIR_RECOVERY_SPEED);
        } else {
            self.vel_h = [0.0; 2];
        }
        self.vel_y = 0.0;
        self.arc = Some(Arc::new(AIR_RECOVERY_HOP, self.params.gravity));
        self.air_dodge_ready = false;
        let m = if self.clip(motion::AIR_RECOVERY).is_some() { motion::AIR_RECOVERY } else { motion::AERIAL_DODGE };
        self.start_action(ActionKind::Hurt, m);
    }

    /// The growth numbers for an ability level (1 = LV1; 0 has none), for a host that runs a
    /// move of its own with this character's numbers (KH1's glide with KH2's).
    pub fn growth_level(&self, level: u8) -> Option<&GrowthLevel> {
        self.growth(level)
    }

    /// Starts a Quick Run straight ahead, now, if it is owned and he is on the ground (a host's
    /// own dodge roll held into KH2's Quick Run). Whether it started.
    pub fn quick_run_now(&mut self) -> bool {
        if self.airborne || self.abilities.quick_run == 0 {
            return false;
        }
        let dir = self.forward();
        self.start_quick_run(dir)
    }

    fn growth(&self, level: u8) -> Option<&GrowthLevel> {
        if level == 0 {
            None
        } else {
            self.growth.get((level as usize - 1).min(self.growth.len().saturating_sub(1)))
        }
    }

    fn forward(&self) -> [f32; 2] {
        [self.facing.sin(), self.facing.cos()]
    }

    fn clip(&self, m: u16) -> Option<&Clip> {
        self.moveset.clips.get(&m)
    }

    fn max_count(&self, air: bool) -> u8 {
        let a = &self.abilities;
        let (base, plus) = if air {
            (self.combo_rules.air_base, a.air_combo_plus)
        } else {
            (self.combo_rules.ground_base, a.combo_plus)
        };
        (base + plus).saturating_sub(a.negative_combo).max(1)
    }

    fn max_finishers(&self) -> u8 {
        self.combo_rules.finisher_base + self.abilities.finishing_plus
    }

    fn primary_target<'e>(&self, env: &'e Env) -> Option<&'e TargetInfo> {
        if let Some(id) = self.lock {
            return env.targets.iter().find(|t| t.id == id);
        }
        if !self.auto_target {
            return None;
        }
        // Auto-target: nearest within the longest combo-row reach or the
        // auto lock-on range, whichever is larger (not yet checked).
        let reach = self.moveset.combo_rows.iter().map(|r| r.far).fold(self.attack.auto_lock_on_range, f32::max);
        env.targets
            .iter()
            .map(|t| (t, dist_h(env.pos, t.pos)))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(t, _)| t)
    }

    /// A target as the combo table sees it; `aim` is the selector's facing (exe `0x140401BB0`:
    /// the direction to the primary target when there is one, else Sora's facing).
    fn geom(&self, env: &Env, t: &TargetInfo, aim: [f32; 2]) -> TargetGeom {
        let d = [t.pos[0] - env.pos[0], t.pos[2] - env.pos[2]];
        let n = norm(d);
        TargetGeom {
            id: t.id,
            distance: len2(d),
            dy: t.pos[1] - env.pos[1],
            radius: t.radius,
            height: t.height,
            facing_dot: if len2(d) > 1e-3 { (n[0] * aim[0] + n[1] * aim[1]).min(1.0) } else { 1.0 },
        }
    }

    fn update_lock(&mut self, input: &Input, env: &Env) {
        if let Some(id) = self.lock {
            let gone = env
                .targets
                .iter()
                .find(|t| t.id == id)
                .map_or(true, |t| dist3(env.pos, t.pos) > self.attack.lock_on_release_range);
            if gone || input.lock.pressed {
                self.lock = None;
                return;
            }
        }
        if input.lock.pressed && self.lock.is_none() {
            self.lock = env
                .targets
                .iter()
                .map(|t| (t.id, dist3(env.pos, t.pos)))
                .filter(|(_, d)| *d <= self.attack.lock_on_range)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(id, _)| id);
        }
    }

    /// Switches the lock-on to the next target around the character (exe):
    /// among the other targets in lock-on range, the one with the smallest
    /// turn from the current target in direction `dir` (+1 toward larger
    /// yaw, −1 toward smaller). Returns true when the target changed.
    pub fn switch_lock(&mut self, dir: f32, pos: [f32; 3], targets: &[TargetInfo]) -> bool {
        let Some(cur) = self.lock.and_then(|id| targets.iter().find(|t| t.id == id)) else { return false };
        let yaw = |t: &TargetInfo| (t.pos[0] - pos[0]).atan2(t.pos[2] - pos[2]);
        let y0 = yaw(cur);
        let next = targets
            .iter()
            .filter(|t| t.id != cur.id && dist3(pos, t.pos) <= self.attack.lock_on_range)
            .map(|t| (t.id, wrap(yaw(t) - y0) * dir))
            .filter(|(_, d)| *d > 0.0)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match next {
            Some((id, _)) => {
                self.lock = Some(id);
                true
            }
            None => false,
        }
    }

    fn start_action(&mut self, kind: ActionKind, motion: u16) {
        self.get_up = None;
        self.cast_hold = None;
        self.swing_counter += 1;
        self.mode = Mode::Action(Action { kind, motion, frame: 0, swing: self.swing_counter, timer: 0.0, brake: 1.0, lunge: 0.0 });
    }

    fn start_arc(&mut self, height: f32) {
        self.arc = Some(Arc::new(height, self.params.gravity));
        self.airborne = true;
    }

    /// A jump whose height depends on how long the button is held.
    fn start_held_arc(&mut self, height: f32) {
        self.start_arc(height);
        if let Some(a) = &mut self.arc {
            a.holdable = true;
        }
    }

    /// KH2's variable jump height: an early release (after 6 frames of rise,
    /// more than 5 before the apex) swaps the rest of the rise for a small hop.
    fn check_jump_release(&mut self) {
        let Some(a) = self.arc else { return };
        if !a.holdable {
            return;
        }
        if self.jump_released && a.t > HOLD_MIN_FRAMES && a.t < a.uptime - HOLD_APEX_MARGIN {
            self.jump_released = false;
            self.start_arc(release_hop(self.params.gravity));
        } else if a.t >= a.uptime - HOLD_APEX_MARGIN {
            if let Some(a) = &mut self.arc {
                a.holdable = false;
            }
        }
    }

    fn start_quick_run(&mut self, dir: [f32; 2]) -> bool {
        let Some(g) = self.growth(self.abilities.quick_run).cloned() else { return false };
        self.facing = yaw_of(dir);
        self.vel_h = scale(norm(dir), g.quick_run_speed);
        self.vel_y = 0.0;
        self.arc = None;
        self.start_action(ActionKind::QuickRun, motion::QUICK_RUN);
        if let Mode::Action(act) = &mut self.mode {
            act.timer = g.quick_run_time;
            act.brake = g.quick_run_brake;
        }
        true
    }

    /// Tries to start an attack. Returns false if no combo row fits.
    fn try_attack(&mut self, input: &Input, env: &Env) -> bool {
        let (current_motion, in_finisher) = match &self.mode {
            Mode::Action(Action { kind: ActionKind::Attack { row }, motion, .. }) => (Some(*motion), row.is_finisher()),
            // A spell cast as a finisher is the finisher playing (`spell_slot`).
            Mode::Action(Action { kind: ActionKind::Magic, motion, .. }) if self.cast_finisher => (Some(*motion), true),
            _ => (None, false),
        };
        let in_attack = current_motion.is_some();
        let carry = std::mem::take(&mut self.combo_carry);
        let carry_finisher = std::mem::take(&mut self.carry_finisher);
        // After a spell that took a finisher slot with Finishing Plus leaving another: the next
        // finisher.
        let in_finisher = in_finisher || (carry && carry_finisher);
        if in_attack && self.abilities.one_swing_on_miss && !self.abilities.combo_master && !self.hit {
            (self.combo_carry, self.carry_finisher) = (carry, carry_finisher);
            return false;
        }
        // Ground swings only advance the combo when the previous one
        // connected (read from the exe). In the air, whiffed swings advance
        // it too, so an air combo always ends in its finisher (matches the
        // game; the exe path for this hasn't been traced yet).
        let hit = self.hit || self.abilities.combo_master || (self.airborne && in_attack);
        let saved = (self.count, self.finishers);
        // The counter moves when the next swing starts.
        if in_finisher {
            if hit && self.finishers < self.max_finishers() {
                self.count = self.count.saturating_add(1);
                self.finishers += 1;
            } else if in_attack {
                // The combo's last finisher: nothing new until its animation has ended (spec).
                (self.combo_carry, self.carry_finisher) = (carry, carry_finisher);
                return false;
            } else {
                self.count = 1;
                self.finishers = 1;
            }
        } else if (in_attack || carry) && hit {
            self.count = self.count.saturating_add(1);
        } else if !in_attack && !carry {
            self.count = 1;
            self.finishers = 1;
        }

        // Locked on (when the host asks for it): the swing faces the target, near or far,
        // before the row is picked (rows that want the target dead ahead then see it there).
        let to_lock = if self.face_lock {
            self.lock
                .and_then(|id| env.targets.iter().find(|t| t.id == id))
                .map(|t| [t.pos[0] - env.pos[0], t.pos[2] - env.pos[2]])
                .filter(|d| len2(*d) > 1e-3)
        } else {
            None
        };
        if let Some(d) = to_lock {
            self.facing = yaw_of(d);
        }
        let air = self.airborne;
        // The selector faces the primary target (exe `0x140401BB0`), so its own rows (kind 1,
        // facing window 0.99996..1) always face it and the all-target rows (kind 2) are
        // measured from it. (Before, Sora's own facing was used: kind-1 rows such as Aerial
        // Sweep, Aerial Dive, Slide Dash and the jump to an airborne foe almost never passed.)
        let primary_info = self.primary_target(env);
        let aim = primary_info
            .map(|t| [t.pos[0] - env.pos[0], t.pos[2] - env.pos[2]])
            .filter(|d| len2(*d) > 1e-3)
            .map(norm)
            .unwrap_or_else(|| self.forward());
        // Locked on, the locked target is the only one the rows are measured against (the user:
        // locked onto one ahead with another beside him, Sora took the base finisher and lunged
        // at the one beside him instead of the locked one's Aerial Finish).
        let locked = self.lock.filter(|id| env.targets.iter().any(|t| t.id == *id));
        let all: Vec<TargetGeom> = env.targets.iter().filter(|t| locked.is_none_or(|id| t.id == id)).map(|t| self.geom(env, t, aim)).collect();
        let primary = primary_info.map(|t| TargetGeom { facing_dot: 1.0, ..self.geom(env, t, aim) });
        let sit = Situation {
            air,
            count: self.count,
            finishers: self.finishers,
            max_count: self.max_count(air),
            current_motion: current_motion.unwrap_or(u16::MAX),
            primary,
            all: &all,
            equipped: &self.abilities.equipped,
            jump_height: self.params.jump_height,
        };
        // A forced row (a state's own move: Counterguard, Retaliating Slash, the Square moves)
        // that can't be used is no attack at all; it never falls back to the ordinary combo (the
        // user: Sora could attack while knocked into the air).
        let wanted = self.forced_row.take();
        // A Square move that isn't a finisher (Upper Slash, Horizontal Slash) stands for one
        // swing of the combo, so it is only there while the combo has a swing left before its
        // finisher; at the end the press does nothing and the Attack button's finisher is next.
        // (It was always there: Horizontal Slash hops, so Sora could spam it forever in the air,
        // the user.)
        // (A form's Square finisher, `square_finishers`, is a finisher whatever its row id.)
        let form_finisher = wanted.is_some_and(|id| self.square_finishers.is_some_and(|(g, a)| id == g || id == a));
        let square = SQUARE_MOVES.iter().any(|m| Some(m.0) == wanted) && !form_finisher;
        let forced = wanted
            .and_then(|id| self.moveset.combo_rows.iter().find(|r| r.id == id))
            .filter(|r| condition(r, &sit))
            .filter(|r| !square || r.is_finisher() || sit.count < sit.max_count);
        let picked = match (wanted, forced) {
            (_, Some(r)) => Some(Choice { row: r, target: sit.primary }),
            (Some(_), None) => None,
            (None, None) => select(&self.moveset.combo_rows, &sit),
        };
        let Some(choice) = picked else {
            (self.count, self.finishers) = saved;
            return false;
        };
        let row = choice.row.clone();
        let target = choice.target;
        if self.clip(row.motion).is_none() {
            (self.count, self.finishers) = saved;
            return false;
        }
        // No target: face the stick direction immediately (with the combo's direction locked,
        // only for a combo's first swing; one that follows keeps the combo's direction).
        let follows = in_attack || carry;
        if to_lock.is_none() && target.is_none() && len2(input.stick) > DEAD_ZONE && !(self.combo_direction_lock && follows) {
            self.facing = yaw_of(input.stick);
        }
        // A target there: the swing starts facing it (and holds that through the swing).
        if self.combo_direction_lock && to_lock.is_none() {
            if let Some(t) = target.and_then(|t| env.targets.iter().find(|x| x.id == t.id)) {
                let d = [t.pos[0] - env.pos[0], t.pos[2] - env.pos[2]];
                if len2(d) > 1e-3 {
                    self.facing = yaw_of(d);
                }
            }
        }
        let dir = match target.and_then(|t| env.targets.iter().find(|x| x.id == t.id)) {
            Some(t) => norm([t.pos[0] - env.pos[0], t.pos[2] - env.pos[2]]),
            None => self.forward(),
        };
        let mut lunge = 0.0;
        if row.is_jump() {
            if !self.airborne {
                // Launching into the air restarts the count.
                self.count = 1;
                self.finishers = 1;
            }
            let (height, speed) = match target {
                Some(t) => {
                    let extra = if row.flag & FLAG_JUMP_ADJUST != 0 { self.params.jump_height } else { 0.0 };
                    let h = t.dy.clamp(row.jump_min, (row.jump_max + extra).max(row.jump_min));
                    let up = uptime(h, self.params.gravity).max(1.0);
                    let s = ((t.distance - t.radius).max(0.0) / up).clamp(row.speed_min, row.speed_max.max(row.speed_min));
                    (h, s)
                }
                None => (row.jump, row.speed_max),
            };
            self.start_arc(height);
            // A moving air attack (an allow-movement range: Wisdom Form's air shot) keeps the
            // speed he has (the user: "sora should not loose his velocity when doing an air move
            // in wisdom form"); the rest go at the row's speed.
            let moving = self.clip(row.motion).is_some_and(|c| c.windows.iter().any(|w| matches!(w.kind, WindowKind::AllowMovement)));
            if !(moving && self.airborne) {
                self.vel_h = scale(dir, speed);
            }
            self.vel_y = 0.0;
        } else {
            self.vel_h = [0.0; 2];
            if let Some(t) = target {
                if row.speed_max > 0.0 {
                    let gap = (t.distance - t.radius).max(0.0);
                    lunge = ((gap + row.distance_adjust) * self.attack.distance_to_speed).clamp(row.speed_min, row.speed_max);
                }
            }
        }
        self.hit = false;
        let motion = row.motion;
        self.start_action(ActionKind::Attack { row }, motion);
        if let Mode::Action(a) = &mut self.mode {
            a.lunge = lunge;
        }
        true
    }

    fn try_action_button(&mut self, input: &Input, env: &Env) -> bool {
        let moving = len2(input.stick) > DEAD_ZONE;
        let a = self.abilities.clone();
        if self.airborne && self.glide_button == GlideButton::Action {
            return self.try_glide(env);
        }
        if !self.airborne {
            // A moveset whose guard row (11) is an attack, not a guard (Valor Form's: a rising
            // double slash that launches him, jump 90; KH2's Valor has no guard): Square, standing
            // or moving, is that attack (the KH1 port's user: "using sora's square move in valor
            // doesnt launch him", "he also should be able to use it while walking").
            let attack_row = self.moveset.combo_rows.iter().find(|r| r.id == GUARD_ROW).filter(|r| {
                self.clip(r.motion).is_some_and(|c| {
                    !c.windows.iter().any(|w| matches!(w.kind, WindowKind::Guard { .. })) && c.windows.iter().any(|w| matches!(w.kind, WindowKind::Hitbox { .. }))
                })
            });
            if attack_row.is_some() {
                self.forced_row = Some(GUARD_ROW);
                if self.try_attack(input, env) {
                    return true;
                }
                self.forced_row = None;
                return false;
            }
            if !moving {
                // Row 11's motion (the moveset's own), else the press's, else the repel's.
                let press = self.moveset.combo_rows.iter().find(|r| r.id == GUARD_ROW).map(|r| r.motion).filter(|m| self.clip(*m).is_some());
                let m = press.or_else(|| self.clip(motion::GUARD_PRESS).map(|_| motion::GUARD_PRESS)).or_else(|| self.clip(motion::GUARD).map(|_| motion::GUARD));
                if a.guard {
                    if let Some(m) = m {
                        self.vel_h = [0.0; 2];
                        // Facing the target with a reticle on it (the lock), as KH2's guard does
                        // (the user: guard didn't turn him toward it).
                        if let Some(t) = self.lock.and_then(|id| env.targets.iter().find(|t| t.id == id)) {
                            let d = [t.pos[0] - env.pos[0], t.pos[2] - env.pos[2]];
                            if len2(d) > 1e-3 {
                                self.facing = yaw_of(d);
                            }
                        }
                        self.start_action(ActionKind::Guard, m);
                        return true;
                    }
                }
                return false;
            }
            if a.dodge_roll > 0 && self.clip(motion::DODGE_ROLL).is_some() {
                self.facing = yaw_of(input.stick);
                self.vel_h = [0.0; 2];
                self.start_action(ActionKind::DodgeRoll, motion::DODGE_ROLL);
                return true;
            }
        }
        // Quick Run is ground-only (the game checks airborne first).
        if a.quick_run > 0 && moving && !self.airborne {
            return self.start_quick_run(input.stick);
        }
        false
    }

    /// Glide, when it is owned and Sora is high enough.
    fn try_glide(&mut self, env: &Env) -> bool {
        if let Some(g) = self.growth(self.abilities.glide) {
            if env.height_above_ground > g.glide_start_height {
                self.arc = None;
                self.vel_y = 0.0;
                self.mode = Mode::Glide;
                self.glide_hover = None;
                return true;
            }
        }
        false
    }

    /// The button that holds Glide.
    fn glide_held(&self, input: &Input) -> bool {
        match self.glide_button {
            GlideButton::Jump => input.jump.held,
            GlideButton::Action => input.action.held,
        }
    }

    /// Jump button. On the ground: jump. In the air: Aerial Dodge if this
    /// airtime still allows it, otherwise Glide (when Glide is on jump).
    fn try_jump(&mut self, input: &Input, env: &Env) -> bool {
        if self.airborne && self.abilities.aerial_dodge > 0 && self.air_dodge_ready {
            let g = self.growth(self.abilities.aerial_dodge).cloned().unwrap_or_default();
            self.air_dodge_ready = false;
            let moving = len2(input.stick) > DEAD_ZONE;
            let dir = norm(input.stick);
            if moving {
                self.facing = yaw_of(input.stick);
            }
            self.start_held_arc(g.air_dodge_height);
            self.jump_held = input.jump.held;
            self.jump_released = !input.jump.held;
            // Only along the stick: with it neutral the dodge goes straight up (the user: KH2's
            // Aerial Dodge doesn't push Sora forward unless the stick is tilted).
            self.vel_h = if moving { scale(dir, g.air_dodge_speed) } else { [0.0; 2] };
            self.vel_y = 0.0;
            self.start_action(ActionKind::AerialDodge, motion::AERIAL_DODGE);
            return true;
        }
        if !self.airborne {
            let high = self.abilities.high_jump > 0;
            let m = if high && self.clip(motion::HIGH_JUMP).is_some() { motion::HIGH_JUMP } else { motion::JUMP };
            self.jump_held = input.jump.held;
            self.jump_released = !input.jump.held;
            self.start_action(ActionKind::Jump { high }, m);
            return true;
        }
        self.glide_button == GlideButton::Jump && self.try_glide(env)
    }

    /// KH2's ledge grab, checked every frame of a jump or fall once the
    /// arc is 6 frames past its apex (15 frames into a plain fall), with the
    /// stick held (read from the exe; the stick-toward-wall part is ours).
    fn try_hang(&mut self, input: &Input, env: &Env) -> bool {
        let h = self.params.hang.clone();
        if h.margin == 0.0 || len2(input.stick) <= DEAD_ZONE || self.clip(motion::HANG).is_none() {
            return false;
        }
        let Some(arc) = self.arc else { return false };
        let after = if arc.uptime > 0.0 { arc.uptime + 6.0 } else { 15.0 };
        if arc.t <= after {
            return false;
        }
        let p = [env.pos[0], env.pos[1] + h.height, env.pos[2]];
        let reach = h.range_xz + h.body_radius;
        let ground = env.pos[1] - env.height_above_ground;
        let stick = norm(input.stick);
        let mut best: Option<([f32; 3], [f32; 2], f32)> = None;
        for l in env.ledges {
            // Pushing toward the wall.
            if stick[0] * -l.out[0] + stick[1] * -l.out[1] <= 0.0 {
                continue;
            }
            let q = closest_on_segment(l.a, l.b, p);
            if q[1] < p[1] - h.range_l || q[1] > p[1] + h.range_h {
                continue;
            }
            if q[1] - ground < h.margin {
                continue;
            }
            let d = dist_h(p, q);
            if d > reach || best.map_or(false, |b| b.2 <= d) {
                continue;
            }
            best = Some((q, l.out, d));
        }
        let Some((q, out, _)) = best else { return false };
        let at = [q[0] + out[0] * h.body_radius, q[1] - h.height, q[2] + out[1] * h.body_radius];
        self.facing = yaw_of([-out[0], -out[1]]);
        self.arc = None;
        self.vel_h = [0.0; 2];
        self.vel_y = 0.0;
        self.reset_combo();
        self.snap = Some(at);
        self.start_action(ActionKind::Hang { at, out, armed: false }, motion::HANG);
        true
    }

    fn reset_combo(&mut self) {
        self.count = 0;
        self.finishers = 0;
        self.combo_carry = false;
        self.carry_finisher = false;
    }

    fn free_motion(&self) -> u16 {
        let p = &self.params;
        let speed = self.shown_speed();
        if self.airborne {
            motion::FALL
        } else if speed >= (p.walk_speed + p.run_speed) * 0.5 {
            motion::RUN
        } else if speed > 0.0 {
            motion::WALK
        } else {
            motion::IDLE
        }
    }

    fn land(&mut self) {
        self.arc = None;
        self.vel_y = 0.0;
        self.air_dodge_ready = false;
        let land = self.clip(motion::LAND).is_some();
        match &self.mode {
            Mode::Free | Mode::Glide => {
                self.mode = Mode::Free;
                self.reset_combo();
                if land {
                    self.start_action(ActionKind::Land, motion::LAND);
                }
            }
            Mode::Action(a) => match &a.kind {
                ActionKind::AerialDodge | ActionKind::QuickRun | ActionKind::Jump { .. } => {
                    self.mode = Mode::Free;
                    if land {
                        self.start_action(ActionKind::Land, motion::LAND);
                    }
                }
                ActionKind::Attack { row } if row.is_jump() || row.is_air() => {
                    self.mode = Mode::Free;
                    self.reset_combo();
                    // KH2's landing, not straight to idle (spec 3.1: aerial attacks went to idle).
                    if land {
                        self.start_action(ActionKind::Land, motion::LAND);
                    }
                }
                ActionKind::Blow { .. } => {
                    // Landing from a launch: the motion after it (down and up).
                    let m = a.motion + 1;
                    self.vel_h = [0.0; 2];
                    if self.clip(m).is_some() {
                        self.start_action(ActionKind::Hurt, m);
                        self.get_up = Some(m);
                    } else {
                        self.mode = Mode::Free;
                    }
                }
                ActionKind::Hurt => {
                    self.vel_h = [0.0; 2];
                    self.mode = Mode::Free;
                    if land {
                        self.start_action(ActionKind::Land, motion::LAND);
                    }
                }
                // An air cast falling as it plays (`air_magic_falls`) ends on the ground in KH2's
                // landing, as Final Form's air Reflect lands (the user). The spell itself is out
                // by then (released by the cast motion's own event).
                ActionKind::Magic if self.air_magic_falls => {
                    self.vel_h = [0.0; 2];
                    self.mode = Mode::Free;
                    // The air combo ends on the ground, as an aerial swing's does.
                    self.reset_combo();
                    if land {
                        self.start_action(ActionKind::Land, motion::LAND);
                    }
                }
                _ => {}
            },
        }
    }

    /// Advances one 60 Hz frame.
    pub fn step(&mut self, input: &Input, env: &Env) -> Output {
        self.since_swing = if matches!(&self.mode, Mode::Action(Action { kind: ActionKind::Attack { .. } | ActionKind::Magic, .. })) { 0 } else { self.since_swing.saturating_add(1) };
        self.since_roll = if matches!(&self.mode, Mode::Action(Action { kind: ActionKind::DodgeRoll | ActionKind::QuickRun, .. })) { 0 } else { self.since_roll.saturating_add(1) };
        // A combo carried past a step (a spell's) lapses on the ground after `COMBO_GRACE` with
        // nothing new (spec: the combo string is one).
        if matches!(self.mode, Mode::Free) && self.combo_carry && !self.airborne && self.since_swing > COMBO_GRACE {
            self.reset_combo();
        }
        self.update_lock(input, env);
        let was_airborne = self.airborne;
        let rising = self.arc.map_or(false, |a| a.rising())
            || matches!(&self.mode, Mode::Action(Action { kind: ActionKind::Blow { blow }, frame, .. }) if blow.vel_y > 0.0 || *frame == 0);
        self.airborne = !env.grounded || rising;
        if was_airborne && !self.airborne {
            self.land();
        }
        // Rolled off a ledge (a dodge roll or Quick Run going from the ground into the air): the
        // fall has its Aerial Dodge, as a jump's does (spec 3.3; walking off a ledge still has
        // none, KH2's rule).
        if !was_airborne && self.airborne && self.since_roll <= 2 && self.abilities.aerial_dodge > 0 {
            self.air_dodge_ready = true;
        }
        if self.airborne {
            self.ceiling_lock = (self.ceiling_lock - 1.0).max(0.0);
        } else {
            self.ceiling_lock = 0.0;
        }

        if self.jump_held && !input.jump.held {
            self.jump_held = false;
            self.jump_released = true;
        }
        if self.airborne && matches!(self.mode, Mode::Free | Mode::Action(Action { kind: ActionKind::AerialDodge, .. })) {
            self.check_jump_release();
        }

        let mut fo = FrameOut::default();
        let release = |b: Button| Button { pressed: false, held: b.held };
        let mut inp = *input;
        for _ in 0..4 {
            if !self.control(&inp, env, &mut fo) {
                break;
            }
            // Something started: run its frame 0 now, without re-reading presses.
            let events = std::mem::take(&mut fo.events);
            fo = FrameOut { events, ..FrameOut::default() };
            inp = Input { attack: release(input.attack), jump: release(input.jump), action: release(input.action), ..*input };
        }

        if let Some((dir, v)) = self.push {
            fo.root[0] += dir[0] * v;
            fo.root[2] += dir[1] * v;
            let next = v * 0.9;
            self.push = if next < 1.0 { None } else { Some((dir, next)) };
        }
        if let Some(at) = self.snap.take() {
            for i in 0..3 {
                fo.root[i] += at[i] - env.pos[i];
            }
        }
        if self.tick % 2 == 0 {
            self.physics(input, env, fo.no_gravity);
        }
        self.tick += 1;

        Output {
            displacement: [self.vel_h[0] + fo.root[0], self.vel_y + fo.root[1], self.vel_h[1] + fo.root[2]],
            facing: self.facing,
            state: self.state_name(),
            motion: fo.motion,
            clip_frame: fo.clip_frame,
            hitboxes: fo.hitboxes,
            invulnerable: fo.invulnerable,
            guarding: fo.guarding,
            guard_boxes: fo.guard_boxes,
            weapon_trail: fo.weapon_trail,
            locked: self.lock,
            combo_count: self.count,
            combo_live: self.combo_live(),
            events: fo.events,
        }
    }

    /// Per-frame control. Returns true when a new action or mode started
    /// and its first frame still has to run.
    fn control(&mut self, input: &Input, env: &Env, fo: &mut FrameOut) -> bool {
        let moving = len2(input.stick) > DEAD_ZONE;
        match std::mem::replace(&mut self.mode, Mode::Free) {
            Mode::Free => {
                if self.ceiling_lock <= 0.0
                    && ((input.attack.pressed && self.try_attack(input, env))
                    || (input.jump.pressed && self.try_jump(input, env))
                        || (input.action.pressed && self.try_action_button(input, env)))
                {
                    return true;
                }
                if self.airborne && self.try_hang(input, env) {
                    return true;
                }
                if moving {
                    self.facing = turn_toward(self.facing, yaw_of(input.stick), self.params.turn_speed);
                }
                fo.motion = self.free_motion();
                false
            }
            Mode::Glide => {
                let g = self.growth(self.abilities.glide).cloned().unwrap_or_default();
                if !self.glide_held(input) || env.height_above_ground < g.glide_end_height || !self.airborne {
                    self.mode = Mode::Free;
                    self.arc = Some(Arc::fall());
                    fo.motion = motion::FALL;
                    return false;
                }
                // Bank into a turn only while there is a real turn left to make, and come out of it
                // only once it is nearly made (ours: steering by a camera-relative stick leaves a
                // hair of turn either way every frame, and banking on its sign flipped the banked
                // motions back and forth).
                let mut d = 0.0;
                if moving {
                    let want = yaw_of(input.stick);
                    d = wrap(want - self.facing);
                    self.facing = turn_toward(self.facing, want, g.glide_turn_speed);
                }
                let (enter, leave) = (GLIDE_BANK_IN.max(g.glide_turn_speed * 2.0), GLIDE_BANK_OUT);
                self.glide_bank = match self.glide_bank {
                    0 if d > enter => 1,
                    0 if d < -enter => -1,
                    1 if d < leave => if d < -enter { -1 } else { 0 },
                    -1 if d > -leave => if d > enter { 1 } else { 0 },
                    b => b,
                };
                // Gliding in place (exe `0x1403A8F44`: a glide started with the stick neutral is
                // ACTION_GLIDEHOVER, motion 208, and the stick takes it into the glide, `0x140422B20`).
                // Ours: a glide whose stick is let go hovers too once it has slowed to a stop
                // (our glide slows with the stick; the exe's glide has no way back).
                let hover = *self.glide_hover.get_or_insert(!moving);
                let hover = if hover { !moving } else { !moving && len2(self.vel_h) < GLIDE_HOVER_SPEED };
                self.glide_hover = Some(hover);
                if hover {
                    self.glide_bank = 0;
                }
                let turn = self.glide_bank;
                fo.motion = match turn {
                    _ if hover => motion::GLIDE_HOVER,
                    0 => motion::GLIDE,
                    1 => motion::GLIDE_TURN_A,
                    _ => motion::GLIDE_TURN_B,
                };
                fo.no_gravity = true;
                self.mode = Mode::Glide;
                false
            }
            Mode::Action(act) => self.run_action(act, input, env, fo),
        }
    }

    fn run_action(&mut self, mut act: Action, input: &Input, env: &Env, fo: &mut FrameOut) -> bool {
        fo.motion = act.motion;
        fo.clip_frame = act.frame;
        let clip = self.clip(act.motion).cloned().unwrap_or_else(|| Clip::plain(act.motion, 1));
        let f = act.frame;
        let mut finished = f >= clip.length;
        // A finisher's window lets the next finisher in only while Finishing Plus leaves one: the
        // combo's last finisher plays to its end before anything new (spec).
        let finisher_more = self.finishers < self.max_finishers();
        let mut cancel_allowed = clip.has(f, |k| matches!(k, WindowKind::Combo) || matches!(k, WindowKind::Finisher) && finisher_more) && self.ceiling_lock <= 0.0;
        let target = self.primary_target(env).map(|t| t.pos);

        for (wi, w) in clip.windows.iter().enumerate() {
            if !w.contains(f) {
                continue;
            }
            match w.kind {
                WindowKind::Hitbox { attack, group, weapon, .. } => {
                    fo.hitboxes.push(ActiveHitbox { swing: (act.swing << 8) | wi as u64, attack, group, weapon })
                }
                // (With the combo's direction locked, no turning during a swing unless a target's
                // reticle is on, the user: "change the direction he's facing mid move only if ...
                // an entity having a blue or orange reticle on it". Then the motion's own turn
                // windows home in on it, as KH2's do.)
                WindowKind::TurnToLock(_) | WindowKind::TurnToTarget(_) if self.combo_direction_lock && target.is_none() && matches!(act.kind, ActionKind::Attack { .. }) => {}
                WindowKind::TurnToLock(rate) | WindowKind::TurnToTarget(rate) => {
                    if let Some(t) = target {
                        let d = [t[0] - env.pos[0], t[2] - env.pos[2]];
                        if len2(d) > 1.0 {
                            self.facing = turn_toward(self.facing, yaw_of(d), rate);
                        }
                    }
                }
                WindowKind::Invulnerable => fo.invulnerable = true,
                // (A host may have air casts fall as they play: KH1's do, the user wanted Reflect
                // in the air to as well.)
                // (Nor in a moving attack, Wisdom Form's air shots: he falls as he shoots, the user,
                // "sora also has gravity".)
                // (Air casts fall through their no-gravity frames from rest, as s174..s177 had it,
                // the user's pick in s229: "roll back the physics of reflect to version s177";
                // `air_magic_falls` ends a cast reaching the ground in KH2's landing.)
                // (An air attack moving all through, Wisdom's shots, falls through its no-gravity frames; one
                // moving only part of the way holds there, Final Form's: the KH1 port's user, its air
                // combos "end after two hits".)
                WindowKind::NoGravity if self.cast_hold.is_none() && !(self.air_magic_falls && matches!(act.kind, ActionKind::Magic)) && !(matches!(act.kind, ActionKind::Attack { .. }) && clip.windows.iter().any(|w| matches!(w.kind, WindowKind::AllowMovement) && w.start == 0 && w.end.is_none())) => fo.no_gravity = true,
                WindowKind::WeaponTrail => fo.weapon_trail = true,
                WindowKind::Guard { attack, group, weapon } => {
                    fo.guarding = true;
                    fo.guard_boxes.push(ActiveHitbox { swing: (act.swing << 8) | wi as u64, attack, group, weapon });
                }
                _ => {}
            }
        }
        for e in clip.events.iter().filter(|e| e.frame == f) {
            fo.events.push(e.kind);
        }
        // (An air cast held as Final Form's Reflect: no gravity for its hold.)
        if let (Some(n), ActionKind::Magic) = (self.cast_hold, &act.kind) {
            if f < n && self.airborne {
                fo.no_gravity = true;
            }
        }
        // Moving in an allow-movement range with nothing to aim at: he turns with the stick.
        if target.is_none() && !self.combo_direction_lock && len2(input.stick) > DEAD_ZONE && matches!(act.kind, ActionKind::Attack { .. } | ActionKind::Magic) && clip.has(f, |k| matches!(k, WindowKind::AllowMovement)) {
            self.facing = turn_toward(self.facing, yaw_of(input.stick), self.params.turn_speed);
        }

        match &act.kind {
            ActionKind::Jump { high } => {
                if len2(input.stick) > DEAD_ZONE {
                    self.facing = turn_toward(self.facing, yaw_of(input.stick), self.params.turn_speed);
                }
                let jump_frame = clip.events.iter().find(|e| e.kind == EventKind::Jump).map_or(0, |e| e.frame);
                if f >= jump_frame {
                    let h = if *high {
                        self.growth(self.abilities.high_jump).map_or(self.params.jump_height, |g| g.high_jump_height)
                    } else {
                        self.params.jump_height
                    };
                    self.start_held_arc(h);
                    self.air_dodge_ready = self.abilities.aerial_dodge > 0;
                    finished = true;
                }
            }
            ActionKind::Scripted { invulnerable, hold } => {
                if *invulnerable {
                    fo.invulnerable = true;
                }
                if *hold {
                    fo.no_gravity = true;
                }
            }
            ActionKind::Land => {
                // Landing hands control to the idle action at once (exe):
                // the land motion only plays while the stick is neutral, and
                // moving switches straight to walk/run. Buttons are accepted
                // once the reaction-command window opens (frame 8 for Sora).
                if len2(input.stick) > DEAD_ZONE && f >= LAND_HOLD {
                    finished = true;
                }
                let open = clip.first(|k| matches!(k, WindowKind::AllowReactionCommand)).unwrap_or(0);
                // Jump at any moment of it (the user: the hold is right, but jumping out of it
                // should work).
                if f >= open || input.jump.pressed {
                    cancel_allowed = true;
                }
            }
            ActionKind::DodgeRoll => {
                let g = self.growth(self.abilities.dodge_roll).cloned().unwrap_or_default();
                if (f as f32) < g.dodge_roll_star_time {
                    fo.invulnerable = true;
                }
                // Other actions may interrupt from the fall-check event
                // (frame 34 for Sora) (not yet checked).
                let free_at = clip.events.iter().find(|e| e.kind == EventKind::FallCheck).map_or(clip.length, |e| e.frame);
                if f >= free_at {
                    cancel_allowed = true;
                    // The stick walks out of the roll's recovery there too, as it does out of a
                    // landing (the user: too much of a freeze after the roll).
                    if len2(input.stick) > DEAD_ZONE {
                        finished = true;
                    }
                }
                // The roll's "quick run check" event: still holding the
                // action button there turns the roll into a Quick Run along
                // the roll direction (checked in the exe).
                if clip.event_at(f, |k| *k == EventKind::QuickRunCheck) && input.action.held && self.abilities.quick_run > 0 {
                    let dir = self.forward();
                    if self.start_quick_run(dir) {
                        return true;
                    }
                }
            }
            ActionKind::QuickRun => {
                let g = self.growth(self.abilities.quick_run).cloned().unwrap_or_default();
                // KH2's (fmab: 0 / 8 / 12 / 16 frames by level); LV1 gets 6 too (the user: Quick
                // Run should always have some).
                if (f as f32) < g.quick_run_star_time.max(QUICK_RUN_STAR_MIN) {
                    fo.invulnerable = true;
                }
                fo.no_gravity = true;
                if act.motion == motion::QUICK_RUN {
                    finished = false;
                    // Releasing the button ends the dash right away.
                    act.timer = if input.action.held { act.timer - 1.0 } else { 0.0 };
                    if act.timer <= 0.0 {
                        act.brake = g.quick_run_stop_brake;
                        act.motion = motion::QUICK_RUN_END;
                        act.frame = 0;
                        self.mode = Mode::Action(act);
                        return false;
                    }
                } else {
                    let end_len = self.clip(motion::QUICK_RUN_END).map_or(0, |c| c.length);
                    finished = f >= end_len && (end_len > 0 || len2(self.vel_h) < 0.5);
                }
            }
            ActionKind::AerialDodge => {
                // No invulnerability (KH2's dodge has none; its clip's Guard window guards it all
                // through, the KH1 port's user: "I only want a guard hitbox").
                // The glide button mid-dodge: straight into Glide.
                if self.glide_button == GlideButton::Action && input.action.pressed {
                    let me = std::mem::replace(&mut self.mode, Mode::Free);
                    if self.try_glide(env) {
                        return true;
                    }
                    self.mode = me;
                }
                if self.glide_button == GlideButton::Jump && input.jump.pressed {
                    let me = std::mem::replace(&mut self.mode, Mode::Free);
                    if self.try_jump(input, env) {
                        return true;
                    }
                    self.mode = me;
                }
            }
            ActionKind::Guard => {}
            ActionKind::Hang { at, out, armed } => {
                // Hold on the hang point (the grab moved us there; the hang
                // motion loops).
                let _ = at;
                fo.no_gravity = true;
                let toward = input.stick[0] * -out[0] + input.stick[1] * -out[1];
                act.frame += 1;
                // In the game any push toward the wall climbs, so holding the
                // stick grabs and climbs in one go. Here the push has to be a
                // fresh one (let go of the stick first), or press jump, so you
                // can hang and then choose (our change, by request).
                let armed = *armed || toward <= DEAD_ZONE;
                let (at, out) = (*at, *out);
                act.kind = ActionKind::Hang { at, out, armed };
                if ((armed && toward > DEAD_ZONE) || input.jump.pressed) && self.clip(motion::CLIMB).is_some() {
                    // Push toward the wall: climb up (exe).
                    self.start_action(ActionKind::Climb, motion::CLIMB);
                    return true;
                }
                if toward < -DEAD_ZONE {
                    // Pull away: let go (ours; the game's release input
                    // isn't traced yet).
                    self.mode = Mode::Free;
                    self.arc = Some(Arc::fall());
                    return false;
                }
                let len = clip.length.max(1);
                fo.clip_frame = f % len;
                self.mode = Mode::Action(act);
                return false;
            }
            ActionKind::Climb => fo.no_gravity = true,
            ActionKind::Magic if self.cast_finisher => {
                // A spell cast as a finisher (spec: finishing Reflect is a finisher of the combo):
                // nothing interrupts it, no new combo, no spell, no jump or roll, until it ends;
                // only the combo's next finisher (Finishing Plus leaving one), an Attack in its
                // finisher window (or a spell, `can_cast`).
                cancel_allowed = false;
                // (A press counts only on a frame the window is open: no buffer, the user: "only
                // inputs to be accepted when its in a acceptable window".)
                let next = finisher_more && finisher_window(&clip, f);
                if input.attack.pressed && next {
                    self.mode = Mode::Action(act.clone());
                    if self.try_attack(input, env) {
                        return true;
                    }
                            }
                if finished {
                    // Its animation over, the combo is (a new one may start).
                    self.reset_combo();
                }
            }
            ActionKind::Magic => {
                // Other actions once the reaction-command window opens.
                let open = clip.first(|k| matches!(k, WindowKind::AllowReactionCommand)).unwrap_or(clip.length);
                if f >= open {
                    cancel_allowed = true;
                }
                // An Attack once the window opens goes on with the combo (`combo_carry`, set when
                // the cast came mid-combo); pressed before, it is not kept (no buffer, the user).
                if input.attack.pressed && (f >= open || f + 1 >= clip.length) {
                    self.mode = Mode::Action(act.clone());
                    if self.try_attack(input, env) {
                        return true;
                    }
                            }
                if finished && !self.airborne {
                    // Played to its end on the ground: back in idle, the combo is over (the
                    // user). (In the air it goes on until the landing, as a swing's does.)
                    self.reset_combo();
                }
            }
            ActionKind::Hurt => {
                // Aerial Recovery's own motion (exe, `docs/aerial-recovery.md`): invulnerable to
                // frame 16, control back at frame 24, then the fall.
                if act.motion == motion::AIR_RECOVERY {
                    // (Its i-frames are its clip's own Invulnerable window, 0..16, frame for frame,
                    // KH2's: the KH1 port's user, "i frames and guard hitbox timing should be 1:1".)
                    if f >= 24 {
                        finished = true;
                    }
                }
                // Counterguard: attack during the guard's repel (exe).
                if act.motion == motion::GUARD && input.attack.pressed && !self.airborne {
                    self.forced_row = Some(COUNTERGUARD_ROW);
                    if self.try_attack(input, env) {
                        return false;
                    }
                    self.forced_row = None;
                }
                // Buttons are taken once the reaction-command window opens
                // (frame 8 for Sora's damage motions); our reading.
                let open = clip.first(|k| matches!(k, WindowKind::AllowReactionCommand)).unwrap_or(clip.length);
                if f >= open {
                    cancel_allowed = true;
                }
                if self.airborne && f >= clip.length {
                    // Air flinch done: keep falling.
                    finished = true;
                }
                // Hyper Healing (ours): the get-up after a launch ends halfway.
                if self.abilities.hyper_healing && self.get_up == Some(act.motion) && f >= clip.length / 2 {
                    finished = true;
                }
            }
            ActionKind::Blow { .. } => {
                // Aerial Recovery (ours: the exe's input for it isn't traced):
                // jump while launched rights Sora with a small hop and the
                // aerial-dodge flip, and control comes back when it ends.
                // As the exe (`0x1403D6030`, `docs/aerial-recovery.md`): from the launch's 16th
                // frame, jump: motion 40, a 100 hop, 10/frame along the stick (straight up with
                // it neutral), the launch's own speed dropped; no Aerial Dodge after it.
                let recovery_after = RECOVERY_AFTER;
                if self.abilities.aerial_recovery && input.jump.pressed && self.airborne && f >= recovery_after {
                    self.start_air_recovery(input.stick);
                    return false;
                }
                // Retaliating Slash (see RETALIATING_SLASH_ROW): Square while launched, from the
                // same frame (exe).
                if input.action.pressed && self.airborne && f >= recovery_after {
                    self.forced_row = Some(RETALIATING_SLASH_ROW);
                    let (vh, vy, arc) = (self.vel_h, self.vel_y, self.arc);
                    self.vel_h = [0.0; 2];
                    self.vel_y = 0.0;
                    self.arc = Some(Arc::fall());
                    if self.try_attack(input, env) {
                        return false;
                    }
                    (self.vel_h, self.vel_y, self.arc) = (vh, vy, arc);
                    self.forced_row = None;
                }
                fo.no_gravity = true;
                // Hold the last frame until landing.
                finished = false;
                if f + 1 >= clip.length {
                    act.frame = f;
                }
            }
            ActionKind::Attack { .. } => {
                // The next swing only for a press inside the combo / finisher window: one before it
                // is dropped (the user: an attack queued early took the window from a spell; "We
                // don't want a buffer type system").
                let combo_open = clip.has(f, |k| matches!(k, WindowKind::Combo));
                let is_finisher = matches!(&act.kind, ActionKind::Attack { row } if row.is_finisher());
                let finisher_open = (clip.has(f, |k| matches!(k, WindowKind::Finisher)) || is_finisher && finisher_window(&clip, f)) && self.finishers < self.max_finishers();
                if input.attack.pressed && (combo_open || finisher_open) {
                    self.mode = Mode::Action(act.clone());
                    if self.try_attack(input, env) {
                        return true;
                    }
                            }
                // The action button (Square) in a combo: KH2's Square moves, whose PTYA rows are
                // kind 0 (reached only this way; the user: Upper Slash and Horizontal Slash never
                // came out). KH2's help: Finishing Leap "at the end of a combo with Square",
                // Upper Slash "knocks a target into the air during a combo", Horizontal Slash
                // "during a midair combo with Square". The first whose ability is equipped and
                // whose row's conditions pass (a finisher's: the combo's end) is taken.
                if input.action.pressed && (combo_open || finisher_open) {
                    if let Some((ground, air)) = self.square_finishers {
                        let row = if self.airborne { air } else { ground };
                        if self.moveset.combo_rows.iter().any(|r| r.id == row) {
                            self.mode = Mode::Action(act.clone());
                            self.forced_row = Some(row);
                            if self.try_attack(input, env) {
                                return true;
                            }
                            self.forced_row = None;
                        }
                    }
                    for (row, ability, air) in SQUARE_MOVES {
                        if air != self.airborne || !self.abilities.equipped.contains(&ability) {
                            continue;
                        }
                        self.mode = Mode::Action(act.clone());
                        self.forced_row = Some(row);
                        if self.try_attack(input, env) {
                            return true;
                        }
                        self.forced_row = None;
                    }
                }
            }
        }

        // Root motion (rotated into world space).
        let rd = clip.root_delta(f);
        let fw = self.forward();
        let right = [fw[1], -fw[0]];
        fo.root = [rd[0] * right[0] + rd[2] * fw[0], rd[1], rd[0] * right[1] + rd[2] * fw[1]];

        // (A damage reaction takes no Attack before it ends, the user: "sora is able to cancel his
        // taking damage animations into attack"; a jump or the action button still may.)
        let attack_cuts = !matches!(act.kind, ActionKind::Attack { .. } | ActionKind::Hurt);
        if !finished && cancel_allowed && (input.jump.pressed || input.action.pressed || (input.attack.pressed && attack_cuts)) {
            finished = true;
        }

        act.frame += 1;
        if !finished {
            self.mode = Mode::Action(act);
            return false;
        }
        self.mode = Mode::Free;
        if self.airborne && self.arc.is_none() {
            self.arc = Some(Arc::fall());
        }
        // KH2's Quick Run is an air slide: it ends in the landing (the user: "Sora does not enter
        // his landing animation after doing a quick run"), its short hold, a jump at any frame.
        if matches!(act.kind, ActionKind::QuickRun) && !self.airborne && self.clip(motion::LAND).is_some() && !(input.attack.pressed || input.jump.pressed || input.action.pressed) {
            self.start_action(ActionKind::Land, motion::LAND);
            return false;
        }
        if cancel_allowed || f >= clip.length {
            if (input.attack.pressed && (attack_cuts || f >= clip.length || !matches!(act.kind, ActionKind::Hurt)) && self.try_attack(input, env))
                || (input.jump.pressed && self.try_jump(input, env))
                || (input.action.pressed && self.try_action_button(input, env))
            {
                return true;
            }
        }
        false
    }

    /// The 30 Hz physics tick: recomputes velocities (per frame) for this
    /// frame and the next.
    fn physics(&mut self, input: &Input, env: &Env, no_gravity: bool) {
        let p = self.params.clone();
        let m = len2(input.stick).min(1.0);
        let moving = m > DEAD_ZONE;
        let stick_dir = norm(input.stick);
        let ground_speed = if !moving { 0.0 } else if m >= p.run_threshold { p.run_speed } else { p.walk_speed };
        self.steer_speed = ground_speed;
        // Ground inertia (KH2's idle-action move, exe): eased toward the stick, braked to a stop.
        let ground = |v: [f32; 2]| -> [f32; 2] {
            let target = scale(stick_dir, ground_speed);
            if p.acceleration <= 0.0 {
                target
            } else if moving {
                [target[0] * (1.0 - p.acceleration) + v[0] * p.acceleration, target[1] * (1.0 - p.acceleration) + v[1] * p.acceleration]
            } else {
                let b = scale(v, p.brake);
                if len2(b) < 0.05 { [0.0; 2] } else { b }
            }
        };
        let steer = |v: [f32; 2], target: [f32; 2]| [v[0] * p.air_keep + target[0] * (1.0 - p.air_keep), v[1] * p.air_keep + target[1] * (1.0 - p.air_keep)];
        // KH1's way (`air_accel`): added to, its length kept to the larger of the old and the run
        // speed (DT 60 Hz frames of it a tick).
        let add = |v: [f32; 2]| -> [f32; 2] {
            let a = p.air_accel * ground_speed * DT;
            let w = [v[0] + stick_dir[0] * a, v[1] + stick_dir[1] * a];
            let cap = len2(v).max(p.run_speed);
            if len2(w) > cap { scale(norm(w), cap) } else { w }
        };
        let air_control = |c: &Self, stick: bool| -> [f32; 2] {
            if stick && moving && p.air_accel > 0.0 {
                add(c.vel_h)
            } else if stick && moving {
                // (s183's momentum keeping under the stick was reverted in s193, the user.)
                steer(c.vel_h, scale(stick_dir, ground_speed))
            } else {
                scale(c.vel_h, p.air_drag)
            }
        };

        let mut gravity = !no_gravity;
        let mode = self.mode.clone();
        match &mode {
            Mode::Free => {
                self.vel_h = if self.airborne { air_control(self, true) } else { ground(self.vel_h) };
            }
            Mode::Glide => {
                let g = self.growth(self.abilities.glide).cloned().unwrap_or_default();
                let a = g.glide_acceleration;
                // Its speed follows the stick: let go and he glides in place, hovering (the user:
                // "in KH2 you're able to glide in place").
                let tilt = if moving { m } else { 0.0 };
                let speed = (1.0 - a) * g.glide_speed * tilt + a * len2(self.vel_h);
                self.vel_h = if self.glide_hover == Some(true) { scale(self.vel_h, GLIDE_HOVER_DRAG) } else { scale(self.forward(), speed) };
                let fall = ((env.height_above_ground - g.glide_fall_height).max(0.0) * g.glide_fall_ratio).min(g.glide_fall_max);
                self.vel_y = -fall;
                gravity = false;
            }
            Mode::Action(act) => match &act.kind {
                ActionKind::Jump { .. } => {
                    self.vel_h = if self.airborne { air_control(self, true) } else { scale(stick_dir, ground_speed) };
                }
                // Idle-action movement: velocity is the stick, no momentum
                // carried from the air.
                // (With ground inertia, Wisdom Form's, the landing keeps his momentum and slides
                // it off, the user; else KH2's hold.)
                ActionKind::Land => self.vel_h = if act.frame < LAND_HOLD && p.acceleration <= 0.0 { [0.0; 2] } else { ground(self.vel_h) },
                ActionKind::Hang { .. } | ActionKind::Climb => {
                    self.vel_h = [0.0; 2];
                    self.vel_y = 0.0;
                    gravity = false;
                }
                ActionKind::Hurt => {
                    if !self.airborne {
                        self.vel_h = [0.0; 2];
                    }
                }
                ActionKind::Magic => {
                    let steer = self.clip_has(act.motion, act.frame, |k| matches!(k, WindowKind::AllowMovement)) && self.cast_hold.is_none();
                    self.vel_h = if self.airborne && self.cast_hold.is_some_and(|n| act.frame < n) {
                        // (Held in place, Final Form's air Reflect.)
                        [0.0; 2]
                    } else if self.airborne && steer {
                        // (Steered as in a jump, Wisdom Form's air casts.)
                        air_control(self, true)
                    } else if self.airborne {
                        scale(self.vel_h, p.air_drag)
                    } else if steer {
                        // (Wisdom Form's casts move: their motions' allow-movement range.)
                        ground(self.vel_h)
                    } else if p.acceleration > 0.0 {
                        scale(self.vel_h, p.brake)
                    } else {
                        [0.0; 2]
                    }
                }
                ActionKind::Blow { blow } => {
                    let mut b = *blow;
                    let d = b.tick(DT, &self.reaction);
                    self.vel_h = [d[0] / DT, d[2] / DT];
                    self.vel_y = d[1] / DT;
                    gravity = false;
                    if let Mode::Action(a) = &mut self.mode {
                        a.kind = ActionKind::Blow { blow: b };
                    }
                }
                ActionKind::DodgeRoll | ActionKind::Guard => self.vel_h = [0.0; 2],
                ActionKind::Scripted { hold: true, .. } => self.vel_h = [0.0; 2],
                ActionKind::Scripted { .. } => self.vel_h = if self.airborne { scale(self.vel_h, p.air_drag) } else { [0.0; 2] },
                ActionKind::QuickRun => {
                    let b = act.brake;
                    self.vel_h = scale(self.vel_h, b);
                    self.vel_y = 0.0;
                    gravity = false;
                }
                ActionKind::AerialDodge => self.vel_h = air_control(self, true),
                ActionKind::Attack { row } => {
                    if row.is_jump() || self.airborne {
                        // (Steered by the stick in an allow-movement range: Wisdom Form's air
                        // shots, the user.)
                        let steer = self.clip_has(act.motion, act.frame, |k| matches!(k, WindowKind::AllowMovement));
                        self.vel_h = air_control(self, steer);
                    } else {
                        let before_hit = {
                            let clip = self.moveset.clips.get(&act.motion);
                            let first = clip.and_then(|c| c.first(|k| matches!(k, WindowKind::Hitbox { .. }))).unwrap_or(0);
                            act.frame <= first
                        };
                        // Moving while attacking in the motion's allow-movement range (Wisdom Form's
                        // shots; the user: "sora is able to move/slide mid combo in KH2"); with
                        // inertia, sliding on after it.
                        self.vel_h = if act.lunge > 0.0 && before_hit {
                            scale(self.forward(), act.lunge)
                        } else if self.clip_has(act.motion, act.frame, |k| matches!(k, WindowKind::AllowMovement)) {
                            ground(self.vel_h)
                        } else if p.acceleration > 0.0 {
                            let b = scale(self.vel_h, p.brake);
                            if len2(b) < 0.05 { [0.0; 2] } else { b }
                        } else {
                            [0.0; 2]
                        };
                    }
                }
            },
        }

        if !self.airborne {
            self.vel_y = 0.0;
        } else if gravity && self.air_magic_falls && ((self.air_magic_gravity - 1.0).abs() > 1e-4 || self.air_magic_fall_max.is_some()) && self.cast_hold.is_none() && matches!(self.mode, Mode::Action(Action { kind: ActionKind::Magic, .. })) {
            // An air cast sinking at a share of gravity (KH1's spells). The arc is kept at the
            // speed reached, so the fall after the cast goes on from it.
            let share = self.air_magic_gravity.max(0.0);
            self.vel_y = (self.vel_y.min(0.0) - p.gravity * share * DT).max(-p.fall_max);
            let mut arc = Arc::fall();
            if p.gravity > 0.0 {
                arc.t = -self.vel_y / p.gravity;
            }
            self.arc = Some(arc);
        } else if gravity {
            let arc = self.arc.get_or_insert_with(Arc::fall);
            self.vel_y = arc.advance(DT, p.gravity, p.fall_max) / DT;
        } else if !matches!(self.mode, Mode::Glide | Mode::Action(Action { kind: ActionKind::Blow { .. }, .. })) {
            self.vel_y = 0.0;
        }
    }

    /// The speed walk / run / idle go by: with ground inertia, what the stick asks (he slides
    /// to a stop in his idle, as Wisdom Form does), else his speed.
    fn shown_speed(&self) -> f32 {
        if self.params.acceleration > 0.0 && !self.airborne {
            self.steer_speed
        } else {
            len2(self.vel_h)
        }
    }

    fn clip_has(&self, motion: u16, frame: u32, f: impl Fn(&WindowKind) -> bool) -> bool {
        self.moveset.clips.get(&motion).is_some_and(|c| c.has(frame, |k| f(k)))
    }

    pub fn state_name(&self) -> StateName {
        match &self.mode {
            Mode::Free => {
                if self.airborne {
                    StateName::Air
                } else if self.shown_speed() >= (self.params.walk_speed + self.params.run_speed) * 0.5 {
                    StateName::Run
                } else if self.shown_speed() > 0.0 {
                    StateName::Walk
                } else {
                    StateName::Idle
                }
            }
            Mode::Glide => StateName::Glide,
            Mode::Action(a) => match a.kind {
                ActionKind::Jump { .. } => StateName::Jump,
                ActionKind::Land => StateName::Land,
                ActionKind::DodgeRoll => StateName::DodgeRoll,
                ActionKind::QuickRun => StateName::QuickRun,
                ActionKind::AerialDodge => StateName::AerialDodge,
                ActionKind::Guard => StateName::Guard,
                ActionKind::Attack { .. } => StateName::Attack,
                ActionKind::Hang { .. } => StateName::Hang,
                ActionKind::Climb => StateName::Climb,
                ActionKind::Hurt => StateName::Hurt,
                ActionKind::Blow { .. } => StateName::Blown,
                ActionKind::Magic => StateName::Magic,
                ActionKind::Scripted { .. } => StateName::Scripted,
            },
        }
    }
}

fn closest_on_segment(a: [f32; 3], b: [f32; 3], p: [f32; 3]) -> [f32; 3] {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let l2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    let t = if l2 > 0.0 { (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1] + (p[2] - a[2]) * ab[2]) / l2).clamp(0.0, 1.0) } else { 0.0 };
    [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t]
}

fn dist_h(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_peaks_at_height_and_is_continuous() {
        let g = crate::KH2_GRAVITY;
        let a = Arc::new(185.0, g);
        assert!((a.uptime - (2.0 * 185.0 / g).sqrt()).abs() < 1e-4);
        assert!((a.y(a.uptime, g) - 185.0).abs() < 1e-3);
        assert!((a.y(a.uptime - 0.01, g) - a.y(a.uptime + 0.01, g)).abs() < 0.01);
        let mut b = a;
        let mut y = 0.0;
        for _ in 0..200 {
            y += b.advance(2.0, g, 16.0);
            if y < -1000.0 {
                break;
            }
        }
        assert!(b.advance(2.0, g, 16.0) >= -32.0 - 1e-4, "fall step capped");
    }
}
