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
use crate::combo::{select, ComboRow, Situation, TargetGeom, FLAG_JUMP_ADJUST};
use crate::params::{Abilities, AttackParams, ComboRules, GrowthLevel, MoveParams};

/// Motion ids used by the controller (KH2 player moveset numbering).
pub mod motion {
    pub const IDLE: u16 = 0;
    pub const WALK: u16 = 1;
    pub const RUN: u16 = 2;
    pub const JUMP: u16 = 3;
    pub const FALL: u16 = 4;
    pub const LAND: u16 = 5;
    pub const GUARD: u16 = 42;
    pub const HIGH_JUMP: u16 = 201;
    pub const AERIAL_DODGE: u16 = 202;
    pub const QUICK_RUN: u16 = 203;
    pub const QUICK_RUN_END: u16 = 204;
    pub const GLIDE: u16 = 205;
    pub const GLIDE_TURN_A: u16 = 206;
    pub const GLIDE_TURN_B: u16 = 207;
    pub const GLIDE_END: u16 = 208;
    pub const DODGE_ROLL: u16 = 209;
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

#[derive(Clone, Copy, Debug)]
pub struct Env<'a> {
    /// Character feet position.
    pub pos: [f32; 3],
    pub grounded: bool,
    pub height_above_ground: f32,
    pub targets: &'a [TargetInfo],
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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActiveHitbox {
    /// Unique per swing and window; feed to a hit tracker.
    pub swing: u64,
    pub attack: u16,
    /// Collision group of the shapes to use.
    pub group: u8,
    /// The group belongs to the weapon model (else the character model).
    pub weapon: bool,
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
    pub locked: Option<u32>,
    /// KH2 combo counter (1 on the first swing; only advances on swings
    /// that follow a connecting one).
    pub combo_count: u8,
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
const HOLD_MIN_FRAMES: f32 = 6.0;
/// ...and more than this many frames before the apex.
const HOLD_APEX_MARGIN: f32 = 5.0;

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
    pub abilities: Abilities,
    pub combo_rules: ComboRules,
    facing: f32,
    vel_h: [f32; 2],
    vel_y: f32,
    airborne: bool,
    arc: Option<Arc>,
    tick: u64,
    mode: Mode,
    swing_counter: u64,
    lock: Option<u32>,
    /// Set when a jump starts with Aerial Dodge owned; cleared on use.
    air_dodge_ready: bool,
    /// Jump button still held since the jump started / released early.
    jump_held: bool,
    jump_released: bool,
    count: u8,
    finishers: u8,
    /// The current swing has connected.
    hit: bool,
    buffered_attack: bool,
}

const TAU: f32 = std::f32::consts::TAU;

fn wrap(a: f32) -> f32 {
    let mut a = (a + std::f32::consts::PI) % TAU;
    if a < 0.0 {
        a += TAU;
    }
    a - std::f32::consts::PI
}

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
            abilities: Abilities::default(),
            combo_rules: ComboRules::default(),
            facing: 0.0,
            vel_h: [0.0; 2],
            vel_y: 0.0,
            airborne: false,
            arc: None,
            tick: 0,
            mode: Mode::Free,
            swing_counter: 0,
            lock: None,
            air_dodge_ready: false,
            count: 0,
            finishers: 0,
            hit: false,
            jump_held: false,
            jump_released: false,
            buffered_attack: false,
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

    pub fn combo_count(&self) -> u8 {
        self.count
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

    fn geom(&self, env: &Env, t: &TargetInfo) -> TargetGeom {
        let d = [t.pos[0] - env.pos[0], t.pos[2] - env.pos[2]];
        let n = norm(d);
        let f = self.forward();
        TargetGeom {
            id: t.id,
            distance: len2(d),
            dy: t.pos[1] - env.pos[1],
            radius: t.radius,
            height: t.height,
            facing_dot: if len2(d) > 1e-3 { n[0] * f[0] + n[1] * f[1] } else { 1.0 },
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

    fn start_action(&mut self, kind: ActionKind, motion: u16) {
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
            _ => (None, false),
        };
        let in_attack = current_motion.is_some();
        // Ground swings only advance the combo when the previous one
        // connected (read from the exe). In the air, whiffed swings advance
        // it too, so an air combo always ends in its finisher (matches the
        // game; the exe path for this hasn't been traced yet).
        let hit = self.hit || self.abilities.combo_master || (self.airborne && in_attack);
        let saved = (self.count, self.finishers);
        // The counter moves when the next swing starts.
        if in_finisher && hit && self.finishers < self.max_finishers() {
            self.count = self.count.saturating_add(1);
            self.finishers += 1;
        } else if in_attack && hit {
            self.count = self.count.saturating_add(1);
        } else if !in_attack {
            self.count = 1;
            self.finishers = 1;
        }

        let air = self.airborne;
        let all: Vec<TargetGeom> = env.targets.iter().map(|t| self.geom(env, t)).collect();
        let primary = self.primary_target(env).map(|t| self.geom(env, t));
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
        let Some(choice) = select(&self.moveset.combo_rows, &sit) else {
            (self.count, self.finishers) = saved;
            return false;
        };
        let row = choice.row.clone();
        let target = choice.target;
        if self.clip(row.motion).is_none() {
            (self.count, self.finishers) = saved;
            return false;
        }
        // No target: face the stick direction immediately.
        if target.is_none() && len2(input.stick) > DEAD_ZONE {
            self.facing = yaw_of(input.stick);
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
            self.vel_h = scale(dir, speed);
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
        self.buffered_attack = false;
        let motion = row.motion;
        self.start_action(ActionKind::Attack { row }, motion);
        if let Mode::Action(a) = &mut self.mode {
            a.lunge = lunge;
        }
        true
    }

    fn try_action_button(&mut self, input: &Input) -> bool {
        let moving = len2(input.stick) > DEAD_ZONE;
        let a = self.abilities.clone();
        if !self.airborne {
            if !moving {
                if a.guard && self.clip(motion::GUARD).is_some() {
                    self.vel_h = [0.0; 2];
                    self.start_action(ActionKind::Guard, motion::GUARD);
                    return true;
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

    /// Jump button. On the ground: jump. In the air: Aerial Dodge if this
    /// airtime still allows it, otherwise Glide.
    fn try_jump(&mut self, input: &Input, env: &Env) -> bool {
        if self.airborne && self.abilities.aerial_dodge > 0 && self.air_dodge_ready {
            let g = self.growth(self.abilities.aerial_dodge).cloned().unwrap_or_default();
            self.air_dodge_ready = false;
            let moving = len2(input.stick) > DEAD_ZONE;
            let dir = if moving { norm(input.stick) } else { self.forward() };
            if moving {
                self.facing = yaw_of(input.stick);
            }
            self.start_held_arc(g.air_dodge_height);
            self.jump_held = input.jump.held;
            self.jump_released = !input.jump.held;
            self.vel_h = scale(dir, g.air_dodge_speed);
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
        if let Some(g) = self.growth(self.abilities.glide) {
            if env.height_above_ground > g.glide_start_height {
                self.arc = None;
                self.vel_y = 0.0;
                self.mode = Mode::Glide;
                return true;
            }
        }
        false
    }

    fn reset_combo(&mut self) {
        self.count = 0;
        self.finishers = 0;
        self.buffered_attack = false;
    }

    fn free_motion(&self) -> u16 {
        let p = &self.params;
        let speed = len2(self.vel_h);
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
                ActionKind::AerialDodge => {
                    self.mode = Mode::Free;
                    if land {
                        self.start_action(ActionKind::Land, motion::LAND);
                    }
                }
                ActionKind::Attack { row } if row.is_jump() || row.is_air() => {
                    self.mode = Mode::Free;
                    self.reset_combo();
                }
                _ => {}
            },
        }
    }

    /// Advances one 60 Hz frame.
    pub fn step(&mut self, input: &Input, env: &Env) -> Output {
        self.update_lock(input, env);
        let was_airborne = self.airborne;
        let rising = self.arc.map_or(false, |a| a.rising());
        self.airborne = !env.grounded || rising;
        if was_airborne && !self.airborne {
            self.land();
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
            locked: self.lock,
            combo_count: self.count,
            events: fo.events,
        }
    }

    /// Per-frame control. Returns true when a new action or mode started
    /// and its first frame still has to run.
    fn control(&mut self, input: &Input, env: &Env, fo: &mut FrameOut) -> bool {
        let moving = len2(input.stick) > DEAD_ZONE;
        match std::mem::replace(&mut self.mode, Mode::Free) {
            Mode::Free => {
                if (input.attack.pressed && self.try_attack(input, env))
                    || (input.jump.pressed && self.try_jump(input, env))
                    || (input.action.pressed && self.try_action_button(input))
                {
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
                if !input.jump.held || env.height_above_ground < g.glide_end_height || !self.airborne {
                    self.mode = Mode::Free;
                    self.arc = Some(Arc::fall());
                    fo.motion = motion::FALL;
                    return false;
                }
                let mut turn = 0;
                if moving {
                    let want = yaw_of(input.stick);
                    let d = wrap(want - self.facing);
                    if d.abs() > 1e-3 {
                        turn = if d > 0.0 { 1 } else { -1 };
                    }
                    self.facing = turn_toward(self.facing, want, g.glide_turn_speed);
                }
                fo.motion = match turn {
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
        let mut cancel_allowed = clip.has(f, |k| matches!(k, WindowKind::Combo | WindowKind::Finisher));
        let target = self.primary_target(env).map(|t| t.pos);

        for (wi, w) in clip.windows.iter().enumerate() {
            if !w.contains(f) {
                continue;
            }
            match w.kind {
                WindowKind::Hitbox { attack, group, weapon, .. } => {
                    fo.hitboxes.push(ActiveHitbox { swing: (act.swing << 8) | wi as u64, attack, group, weapon })
                }
                WindowKind::TurnToLock(rate) | WindowKind::TurnToTarget(rate) => {
                    if let Some(t) = target {
                        let d = [t[0] - env.pos[0], t[2] - env.pos[2]];
                        if len2(d) > 1.0 {
                            self.facing = turn_toward(self.facing, yaw_of(d), rate);
                        }
                    }
                }
                WindowKind::Invulnerable => fo.invulnerable = true,
                WindowKind::NoGravity => fo.no_gravity = true,
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
            ActionKind::Land => {
                // Inputs are accepted once the reaction-command window opens
                // (frame 8 for Sora).
                let open = clip.first(|k| matches!(k, WindowKind::AllowReactionCommand)).unwrap_or(0);
                if f >= open {
                    cancel_allowed = true;
                    if len2(input.stick) > DEAD_ZONE {
                        finished = true;
                    }
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
                if (f as f32) < g.quick_run_star_time {
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
                // Jump again mid-dodge: straight into Glide.
                if input.jump.pressed {
                    let me = std::mem::replace(&mut self.mode, Mode::Free);
                    if self.try_jump(input, env) {
                        return true;
                    }
                    self.mode = me;
                }
            }
            ActionKind::Guard => {}
            ActionKind::Attack { .. } => {
                if input.attack.pressed {
                    self.buffered_attack = true;
                }
                let combo_open = clip.has(f, |k| matches!(k, WindowKind::Combo));
                let finisher_open =
                    clip.has(f, |k| matches!(k, WindowKind::Finisher)) && self.finishers < self.max_finishers();
                if self.buffered_attack && (combo_open || finisher_open) {
                    self.mode = Mode::Action(act.clone());
                    if self.try_attack(input, env) {
                        return true;
                    }
                    self.buffered_attack = false;
                }
            }
        }

        // Root motion (rotated into world space).
        let rd = clip.root_delta(f);
        let fw = self.forward();
        let right = [fw[1], -fw[0]];
        fo.root = [rd[0] * right[0] + rd[2] * fw[0], rd[1], rd[0] * right[1] + rd[2] * fw[1]];

        if !finished && cancel_allowed && (input.jump.pressed || input.action.pressed || (input.attack.pressed && !matches!(act.kind, ActionKind::Attack { .. }))) {
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
        if cancel_allowed || f >= clip.length {
            if (input.attack.pressed && self.try_attack(input, env))
                || (input.jump.pressed && self.try_jump(input, env))
                || (input.action.pressed && self.try_action_button(input))
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
        let steer = |v: [f32; 2], target: [f32; 2]| [v[0] * p.air_keep + target[0] * (1.0 - p.air_keep), v[1] * p.air_keep + target[1] * (1.0 - p.air_keep)];
        let air_control = |c: &Self, stick: bool| -> [f32; 2] {
            if stick && moving {
                steer(c.vel_h, scale(stick_dir, ground_speed))
            } else {
                scale(c.vel_h, p.air_drag)
            }
        };

        let mut gravity = !no_gravity;
        let mode = self.mode.clone();
        match &mode {
            Mode::Free => {
                self.vel_h = if self.airborne { air_control(self, true) } else { scale(stick_dir, ground_speed) };
            }
            Mode::Glide => {
                let g = self.growth(self.abilities.glide).cloned().unwrap_or_default();
                let a = g.glide_acceleration;
                let speed = (1.0 - a) * g.glide_speed + a * len2(self.vel_h);
                self.vel_h = scale(self.forward(), speed);
                let fall = ((env.height_above_ground - g.glide_fall_height).max(0.0) * g.glide_fall_ratio).min(g.glide_fall_max);
                self.vel_y = -fall;
                gravity = false;
            }
            Mode::Action(act) => match &act.kind {
                ActionKind::Jump { .. } => {
                    self.vel_h = if self.airborne { air_control(self, true) } else { scale(stick_dir, ground_speed) };
                }
                ActionKind::Land => {}
                ActionKind::DodgeRoll | ActionKind::Guard => self.vel_h = [0.0; 2],
                ActionKind::QuickRun => {
                    let b = act.brake;
                    self.vel_h = scale(self.vel_h, b);
                    self.vel_y = 0.0;
                    gravity = false;
                }
                ActionKind::AerialDodge => self.vel_h = air_control(self, true),
                ActionKind::Attack { row } => {
                    if row.is_jump() || self.airborne {
                        self.vel_h = air_control(self, false);
                    } else {
                        let before_hit = {
                            let clip = self.moveset.clips.get(&act.motion);
                            let first = clip.and_then(|c| c.first(|k| matches!(k, WindowKind::Hitbox { .. }))).unwrap_or(0);
                            act.frame <= first
                        };
                        self.vel_h = if act.lunge > 0.0 && before_hit { scale(self.forward(), act.lunge) } else { [0.0; 2] };
                    }
                }
            },
        }

        if !self.airborne {
            self.vel_y = 0.0;
        } else if gravity {
            let arc = self.arc.get_or_insert_with(Arc::fall);
            self.vel_y = arc.advance(DT, p.gravity, p.fall_max) / DT;
        } else if !matches!(self.mode, Mode::Glide) {
            self.vel_y = 0.0;
        }
    }

    pub fn state_name(&self) -> StateName {
        match &self.mode {
            Mode::Free => {
                if self.airborne {
                    StateName::Air
                } else if len2(self.vel_h) >= (self.params.walk_speed + self.params.run_speed) * 0.5 {
                    StateName::Run
                } else if len2(self.vel_h) > 0.0 {
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
            },
        }
    }
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
