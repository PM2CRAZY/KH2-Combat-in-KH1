//! The 60 Hz character state machine.
//!
//! The host calls [`Character::step`] once per 60 Hz frame with the input
//! and a little world information, moves the body by
//! [`Output::displacement`] with its own collision, and reports
//! `grounded` back on the next frame. Hitboxes come out as data; the host
//! places shapes and resolves hits (e.g. with `combat-core`).

use std::collections::HashMap;

use crate::clip::{Clip, EventKind, WindowKind};
use crate::combo::{select, ComboRow, Situation, TargetGeom};
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
    pub const GLIDE_START: u16 = 205;
    pub const GLIDE: u16 = 206;
    pub const GLIDE_TURN: u16 = 207;
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
    pub group: u8,
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
    pub locked: Option<u32>,
    pub combo_count: u8,
    pub events: Vec<EventKind>,
}

#[derive(Clone, Debug, PartialEq)]
enum ActionKind {
    Jump { high: bool },
    Land,
    DodgeRoll,
    QuickRun,
    AerialDodge,
    Guard,
    Attack { row: ComboRow, air: bool },
}

#[derive(Clone, Debug)]
struct Action {
    kind: ActionKind,
    motion: u16,
    frame: u32,
    swing: u64,
    /// Horizontal speed driven by the action (quick run).
    speed: f32,
}

#[derive(Clone, Debug)]
enum Mode {
    Free,
    Glide { frame: u32, speed: f32 },
    Action(Action),
}

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
    mode: Mode,
    swing_counter: u64,
    lock: Option<u32>,
    air_dodged: bool,
    ground_hits: u8,
    air_hits: u8,
    finishers: u8,
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

/// Initial upward speed that peaks at `height` under this controller's
/// per-frame integration (gravity is subtracted before the move each frame):
/// apex = v0^2 / 2g + v0 / 2 - ... solved exactly for the discrete sum.
fn jump_velocity(height: f32, gravity: f32) -> f32 {
    let g = gravity.max(1e-4);
    let h = height.max(0.0);
    // Sum of (v0 - k g) for k = 1..n while positive ~= v0^2 / (2g) - v0 / 2.
    g / 2.0 + (g * g / 4.0 + 2.0 * g * h).sqrt()
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
            mode: Mode::Free,
            swing_counter: 0,
            lock: None,
            air_dodged: false,
            ground_hits: 0,
            air_hits: 0,
            finishers: 0,
            buffered_attack: false,
        }
    }

    pub fn facing(&self) -> f32 {
        self.facing
    }

    pub fn set_facing(&mut self, yaw: f32) {
        self.facing = wrap(yaw);
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

    fn max_hits(&self, air: bool) -> u8 {
        let a = &self.abilities;
        let (base, plus) = if air {
            (self.combo_rules.air_base, a.air_combo_plus)
        } else {
            (self.combo_rules.ground_base, a.combo_plus)
        };
        (base + plus).saturating_sub(a.negative_combo).max(1)
    }

    fn target_pos(&self, env: &Env) -> Option<[f32; 3]> {
        let pick = |id: u32| env.targets.iter().find(|t| t.id == id).map(|t| t.pos);
        if let Some(id) = self.lock {
            return pick(id);
        }
        // Auto-target: nearest within the longest combo-row reach or the
        // auto lock-on range, whichever is larger (hypothesis).
        let reach = self
            .moveset
            .combo_rows
            .iter()
            .map(|r| r.far)
            .fold(self.attack.auto_lock_on_range, f32::max);
        env.targets
            .iter()
            .map(|t| (t, dist_h(env.pos, t.pos)))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(t, _)| t.pos)
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
        self.mode = Mode::Action(Action { kind, motion, frame: 0, swing: self.swing_counter, speed: 0.0 });
    }

    /// Tries to start an attack. Returns false if no combo row fits.
    fn try_attack(&mut self, input: &Input, env: &Env) -> bool {
        let air = self.airborne;
        let hits = if air { self.air_hits } else { self.ground_hits };
        let max = self.max_hits(air);
        let chaining_finisher = self.finishers > 0;
        if hits >= max && !chaining_finisher {
            return false;
        }
        let finisher = chaining_finisher || hits + 1 >= max;
        let target = self.target_pos(env).map(|t| TargetGeom {
            distance: dist_h(env.pos, t),
            dy: t[1] - (env.pos[1] + self.attack.attack_y_offset),
        });
        let sit = Situation { air, finisher, target, equipped: &self.abilities.equipped };
        let Some(row) = select(&self.moveset.combo_rows, &sit).cloned() else { return false };
        if self.clip(row.motion).is_none() {
            return false;
        }
        // No target: face the stick direction immediately.
        if target.is_none() && len2(input.stick) > 0.1 {
            self.facing = yaw_of(input.stick);
        }
        if air {
            self.air_hits += 1;
            if row.jump > 0.0 {
                self.vel_y = jump_velocity(row.jump, self.params.gravity);
            }
        } else {
            self.ground_hits += 1;
        }
        if finisher {
            self.finishers += 1;
        }
        self.vel_h = [0.0; 2];
        self.buffered_attack = false;
        let motion = row.motion;
        self.start_action(ActionKind::Attack { row, air }, motion);
        true
    }

    fn try_action_button(&mut self, input: &Input) -> bool {
        let moving = len2(input.stick) > 0.1;
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
        } else if a.aerial_dodge > 0 && !self.air_dodged {
            let g = self.growth(a.aerial_dodge).cloned().unwrap_or_default();
            self.air_dodged = true;
            self.vel_y = jump_velocity(g.air_dodge_height, self.params.gravity);
            if moving {
                self.facing = yaw_of(input.stick);
            }
            let f = self.forward();
            let s = if moving { g.air_dodge_speed } else { 0.0 };
            self.vel_h = [f[0] * s, f[1] * s];
            self.start_action(ActionKind::AerialDodge, motion::AERIAL_DODGE);
            return true;
        }
        if a.quick_run > 0 && moving {
            let g = self.growth(a.quick_run).cloned().unwrap_or_default();
            self.facing = yaw_of(input.stick);
            self.start_action(ActionKind::QuickRun, motion::QUICK_RUN);
            if let Mode::Action(act) = &mut self.mode {
                act.speed = g.quick_run_speed;
            }
            return true;
        }
        false
    }

    fn try_jump(&mut self, input: &Input, env: &Env) -> bool {
        if !self.airborne {
            let high = self.abilities.high_jump > 0;
            let m = if high && self.clip(motion::HIGH_JUMP).is_some() { motion::HIGH_JUMP } else { motion::JUMP };
            self.start_action(ActionKind::Jump { high }, m);
            return true;
        }
        let lv = self.abilities.glide;
        if let Some(g) = self.growth(lv).cloned() {
            if env.height_above_ground >= g.glide_start_height && input.jump.held {
                let speed = len2(self.vel_h);
                self.vel_y = self.vel_y.min(0.0);
                self.mode = Mode::Glide { frame: 0, speed };
                return true;
            }
        }
        false
    }

    /// Ground/air movement from the stick, KH2 style: the character turns
    /// at a capped rate and always moves along its facing.
    fn free_move(&mut self, input: &Input) {
        let m = len2(input.stick).min(1.0);
        let p = &self.params;
        let target_speed = if m >= p.run_threshold {
            p.run_speed
        } else if m > 0.1 {
            p.walk_speed
        } else {
            0.0
        };
        if m > 0.1 {
            self.facing = turn_toward(self.facing, yaw_of(input.stick), p.turn_speed);
        }
        let f = self.forward();
        let want = [f[0] * target_speed, f[1] * target_speed];
        if p.acceleration > 0.0 && target_speed > 0.0 {
            let a = p.acceleration;
            self.vel_h = [self.vel_h[0] * a + want[0] * (1.0 - a), self.vel_h[1] * a + want[1] * (1.0 - a)];
        } else if p.brake > 0.0 && target_speed == 0.0 {
            self.vel_h = [self.vel_h[0] * p.brake, self.vel_h[1] * p.brake];
            if len2(self.vel_h) < 0.05 {
                self.vel_h = [0.0; 2];
            }
        } else {
            self.vel_h = want;
        }
    }

    fn apply_gravity(&mut self, no_gravity: bool) {
        if !self.airborne {
            self.vel_y = self.vel_y.max(0.0);
            return;
        }
        if !no_gravity {
            self.vel_y = (self.vel_y - self.params.gravity).max(-self.params.fall_max);
        }
    }

    fn reset_combo(&mut self) {
        self.ground_hits = 0;
        self.air_hits = 0;
        self.finishers = 0;
        self.buffered_attack = false;
    }

    /// Advances one 60 Hz frame.
    pub fn step(&mut self, input: &Input, env: &Env) -> Output {
        self.update_lock(input, env);
        let mut events = Vec::new();
        let was_airborne = self.airborne;
        let rising = self.vel_y > 0.0;
        self.airborne = !env.grounded || rising;

        // Landing.
        if was_airborne && !self.airborne {
            self.air_dodged = false;
            self.air_hits = 0;
            match &self.mode {
                Mode::Free => {
                    self.reset_combo();
                    if self.clip(motion::LAND).is_some() {
                        self.start_action(ActionKind::Land, motion::LAND);
                    }
                }
                Mode::Glide { .. } => {
                    self.mode = Mode::Free;
                }
                Mode::Action(a) => {
                    // Air attacks and air dodges end on touchdown.
                    if matches!(a.kind, ActionKind::Attack { air: true, .. } | ActionKind::AerialDodge) {
                        self.mode = Mode::Free;
                        self.reset_combo();
                    }
                }
            }
        }
        // Walked off a ledge mid-ground-action: keep going, gravity applies.

        let target = self.target_pos(env);
        let mut root = [0.0f32; 3];
        let mut hitboxes = Vec::new();
        let mut invulnerable = false;
        let mut guarding = false;
        let mut no_gravity = false;
        let mut motion_id;
        let mut clip_frame = 0;

        match std::mem::replace(&mut self.mode, Mode::Free) {
            Mode::Free => {
                self.mode = Mode::Free;
                let acted = (input.attack.pressed && self.try_attack(input, env))
                    || (input.jump.pressed && self.try_jump(input, env))
                    || (input.action.pressed && self.try_action_button(input));
                if !acted {
                    self.free_move(input);
                }
                motion_id = if self.airborne { motion::FALL } else if len2(self.vel_h) > self.params.walk_speed { motion::RUN } else if len2(self.vel_h) > 0.0 { motion::WALK } else { motion::IDLE };
                if !matches!(self.mode, Mode::Free) {
                    // Started something this frame: run its frame 0 now.
                    return self.step_started(input, env, events);
                }
            }
            Mode::Glide { frame, speed } => {
                let g = self.growth(self.abilities.glide).cloned().unwrap_or_default();
                if !input.jump.held || env.height_above_ground < g.glide_end_height || !self.airborne {
                    self.mode = Mode::Free;
                    self.free_move(input);
                    motion_id = motion::GLIDE_END;
                } else {
                    if len2(input.stick) > 0.1 {
                        self.facing = turn_toward(self.facing, yaw_of(input.stick), g.glide_turn_speed);
                    }
                    let a = g.glide_acceleration;
                    let speed = speed * a + g.glide_speed * (1.0 - a);
                    let f = self.forward();
                    self.vel_h = [f[0] * speed, f[1] * speed];
                    // Descent accelerates by fall_ratio x speed per frame,
                    // capped at glide_fall_max (hypothesis, see docs).
                    self.vel_y = (self.vel_y - g.glide_fall_ratio * speed).max(-g.glide_fall_max);
                    no_gravity = true;
                    self.mode = Mode::Glide { frame: frame + 1, speed };
                    motion_id = if frame < 10 { motion::GLIDE_START } else { motion::GLIDE };
                    clip_frame = frame;
                }
            }
            Mode::Action(mut act) => {
                motion_id = act.motion;
                clip_frame = act.frame;
                let clip = self.clip(act.motion).cloned().unwrap_or_else(|| Clip::plain(act.motion, 1));
                let f = act.frame;
                let mut finished = f >= clip.length;
                let mut cancel_allowed = clip.has(f, |k| matches!(k, WindowKind::Combo | WindowKind::Finisher));

                // Windows.
                for (wi, w) in clip.windows.iter().enumerate() {
                    if !w.contains(f) {
                        continue;
                    }
                    match w.kind {
                        WindowKind::Hitbox { attack, group, .. } => hitboxes.push(ActiveHitbox {
                            swing: (act.swing << 8) | wi as u64,
                            attack,
                            group,
                        }),
                        WindowKind::TurnToLock(rate) | WindowKind::TurnToTarget(rate) => {
                            if let Some(t) = target {
                                let d = [t[0] - env.pos[0], t[2] - env.pos[2]];
                                if len2(d) > 1.0 {
                                    self.facing = turn_toward(self.facing, yaw_of(d), rate);
                                }
                            }
                        }
                        WindowKind::Invulnerable => invulnerable = true,
                        WindowKind::NoGravity => no_gravity = true,
                        WindowKind::Guard => guarding = true,
                        _ => {}
                    }
                }
                for e in clip.events.iter().filter(|e| e.frame == f) {
                    events.push(e.kind);
                }

                match &act.kind {
                    ActionKind::Jump { high } => {
                        // Keep running during the takeoff frames.
                        self.free_move(input);
                        let jump_frame = clip.events.iter().find(|e| e.kind == EventKind::Jump).map_or(0, |e| e.frame);
                        if f >= jump_frame {
                            let h = if *high {
                                self.growth(self.abilities.high_jump).map_or(self.params.jump_height, |g| g.high_jump_height)
                            } else {
                                self.params.jump_height
                            };
                            self.vel_y = jump_velocity(h, self.params.gravity);
                            self.airborne = true;
                            finished = true;
                        }
                    }
                    ActionKind::Land => {
                        // Momentum is kept; inputs are accepted once the
                        // reaction-command window opens (frame 8 for Sora).
                        let open = clip.first(|k| matches!(k, WindowKind::AllowReactionCommand)).unwrap_or(0);
                        if f >= open {
                            cancel_allowed = true;
                            if len2(input.stick) > 0.1 {
                                finished = true;
                            }
                        }
                    }
                    ActionKind::DodgeRoll => {
                        let g = self.growth(self.abilities.dodge_roll).cloned().unwrap_or_default();
                        if (f as f32) < g.dodge_roll_star_time {
                            invulnerable = true;
                        }
                        // Other actions may interrupt from the fall-check
                        // event (frame 34 for Sora) (hypothesis).
                        let free_at = clip.events.iter().find(|e| e.kind == EventKind::FallCheck).map_or(clip.length, |e| e.frame);
                        if f >= free_at {
                            cancel_allowed = true;
                        }
                    }
                    ActionKind::QuickRun => {
                        let g = self.growth(self.abilities.quick_run).cloned().unwrap_or_default();
                        if (f as f32) < g.quick_run_star_time {
                            invulnerable = true;
                        }
                        if (f as f32) >= g.quick_run_time {
                            act.speed *= g.quick_run_brake;
                        }
                        let fw = self.forward();
                        self.vel_h = [fw[0] * act.speed, fw[1] * act.speed];
                        no_gravity = true;
                        finished = act.speed <= self.params.run_speed && (f as f32) >= g.quick_run_time;
                    }
                    ActionKind::AerialDodge => {}
                    ActionKind::Guard => {
                        self.vel_h = [0.0; 2];
                    }
                    ActionKind::Attack { row, .. } => {
                        if input.attack.pressed {
                            self.buffered_attack = true;
                        }
                        // Approach assist before the first hitbox opens
                        // (hypothesis using plyr attack v0 values).
                        let first_hit = clip.first(|k| matches!(k, WindowKind::Hitbox { .. })).unwrap_or(0);
                        self.vel_h = [0.0; 2];
                        if let Some(t) = target {
                            let d = dist_h(env.pos, t);
                            let gap = d - self.attack.contact_distance;
                            if f < first_hit && gap > 0.0 && row.needs_target() {
                                let v0 = if row.is_finisher() {
                                    self.attack.finish_v0
                                } else if self.ground_hits + self.air_hits <= 1 {
                                    self.attack.first_v0
                                } else {
                                    self.attack.combo_v0
                                };
                                let fw = self.forward();
                                let s = v0.min(gap);
                                self.vel_h = [fw[0] * s, fw[1] * s];
                            }
                        }
                        let combo_open = clip.has(f, |k| matches!(k, WindowKind::Combo));
                        let finisher_open = clip.has(f, |k| matches!(k, WindowKind::Finisher))
                            && self.finishers <= self.abilities.finishing_plus;
                        if self.buffered_attack && (combo_open || finisher_open) {
                            if finisher_open {
                                // Keep the finisher chain going.
                            } else {
                                self.finishers = 0;
                            }
                            if self.try_attack(input, env) {
                                return self.step_started(input, env, events);
                            }
                            self.buffered_attack = false;
                        }
                    }
                }

                // Root motion (rotated into world space).
                let rd = clip.root_delta(f);
                let fw = self.forward();
                let right = [fw[1], -fw[0]];
                root = [rd[0] * right[0] + rd[2] * fw[0], rd[1], rd[0] * right[1] + rd[2] * fw[1]];

                // Cancels into other actions.
                if !finished && cancel_allowed {
                    if input.jump.pressed || input.action.pressed || (input.attack.pressed && !matches!(act.kind, ActionKind::Attack { .. })) {
                        finished = true;
                    }
                }

                act.frame += 1;
                if finished {
                    let was_attack = matches!(act.kind, ActionKind::Attack { .. });
                    self.mode = Mode::Free;
                    if was_attack && f >= clip.length {
                        self.reset_combo();
                    }
                    // Handle the input that cancelled this action now.
                    if cancel_allowed || f >= clip.length {
                        let acted = (input.attack.pressed && self.try_attack(input, env))
                            || (input.jump.pressed && self.try_jump(input, env))
                            || (input.action.pressed && self.try_action_button(input));
                        if acted {
                            return self.step_started(input, env, events);
                        }
                    }
                } else {
                    self.mode = Mode::Action(act);
                }
            }
        }

        self.apply_gravity(no_gravity);
        if let Mode::Free = self.mode {
            if self.airborne {
                motion_id = motion::FALL;
            }
        }
        let state = self.state_name();
        Output {
            displacement: [self.vel_h[0] + root[0], self.vel_y + root[1], self.vel_h[1] + root[2]],
            facing: self.facing,
            state,
            motion: motion_id,
            clip_frame,
            hitboxes,
            invulnerable,
            guarding,
            locked: self.lock,
            combo_count: self.ground_hits.max(self.air_hits),
            events,
        }
    }

    /// Runs frame 0 of an action that was just started.
    fn step_started(&mut self, input: &Input, env: &Env, mut events: Vec<EventKind>) -> Output {
        let mut out = self.step_action_only(input, env);
        events.append(&mut out.events);
        out.events = events;
        out
    }

    fn step_action_only(&mut self, input: &Input, env: &Env) -> Output {
        // Same as `step` but without landing detection and lock update, so
        // a freshly started action gets its frame 0 this frame.
        let saved_airborne = self.airborne;
        let release = |b: Button| Button { pressed: false, held: b.held };
        let quiet = Input {
            attack: release(input.attack),
            jump: release(input.jump),
            action: release(input.action),
            ..*input
        };
        let fake_env = Env { grounded: !saved_airborne, ..*env };
        let lock = self.lock;
        let out = self.step(&Input { lock: Button::default(), ..quiet }, &fake_env);
        self.lock = lock;
        out
    }

    pub fn state_name(&self) -> StateName {
        match &self.mode {
            Mode::Free => {
                if self.airborne {
                    StateName::Air
                } else if len2(self.vel_h) > self.params.walk_speed {
                    StateName::Run
                } else if len2(self.vel_h) > 0.0 {
                    StateName::Walk
                } else {
                    StateName::Idle
                }
            }
            Mode::Glide { .. } => StateName::Glide,
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
