//! A non-player character: plays its own clips (idle, walk, attacks) on
//! command, and reacts to hits with the game's generic reaction
//! ([`crate::reaction`]). What to do and when is the host's choice (KH2
//! drives enemies with per-enemy scripts, which this crate doesn't run).

use crate::reaction::{self, Blow, Hit, ReactionParams, Response};
use crate::{ActiveHitbox, Arc, Clip, WindowKind, KH2_GRAVITY};

/// Motion ids every KH2 enemy moveset uses for these.
pub mod motion {
    pub const IDLE: u16 = 0;
    pub const WALK: u16 = 1;
    pub const FALL: u16 = 4;
}

/// What the host wants this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NpcCommand {
    /// Walk this way (unit vector on x, z), turning toward it.
    pub walk: Option<[f32; 2]>,
    /// Turn toward this direction without moving.
    pub face: Option<[f32; 2]>,
    /// Start this motion as an action (ignored while busy).
    pub action: Option<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcState {
    Idle,
    Walk,
    /// Playing a commanded action.
    Action,
    Hurt,
    /// Launched; `Down` once landed and getting up.
    Blown,
    Down,
    Dead,
    /// Pulled into a Magnet.
    Suction,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NpcOutput {
    pub displacement: [f32; 3],
    pub facing: f32,
    pub state: NpcState,
    pub motion: u16,
    pub clip_frame: u32,
    /// The motion loops (idle, walk).
    pub looping: bool,
    /// The clip's root motion moved the character (subtract it when posing).
    pub root_motion: bool,
    pub hitboxes: Vec<ActiveHitbox>,
    pub guard_boxes: Vec<ActiveHitbox>,
    /// Hitboxes are off (collision-off window).
    pub invulnerable: bool,
}

#[derive(Clone, Debug)]
enum Mode {
    Idle,
    Walk,
    Clip { state: NpcState, motion: u16, frame: u32, next: Option<u16>, swing: u64 },
    /// `step`: this tick's per-frame displacement (physics runs every
    /// second frame).
    Blow { motion: u16, frame: u32, blow: Blow, step: [f32; 3] },
    Dead { motion: u16, frame: u32 },
    /// Caught by a Magnet: orbit radius and swirl speed (exe, reaction 5).
    Suction { motion: u16, frame: u32, radius: f32, swirl: f32 },
}

#[derive(Clone, Debug)]
pub struct Npc {
    pub clips: std::collections::HashMap<u16, Clip>,
    pub reaction: ReactionParams,
    /// Yaw change per frame when turning (radians).
    pub turn_speed: f32,
    facing: f32,
    mode: Mode,
    frame: u32,
    swing: u64,
    push: Option<([f32; 2], f32)>,
    hop: Option<(Arc, [f32; 2])>,
    airborne: bool,
    fall_vel: f32,
    tick: u64,
    /// While caught by a Magnet: the vector from the body centre to the
    /// magnet's centre this frame (the host updates it; `None` lets go).
    pub suction_to: Option<[f32; 3]>,
}

fn yaw_of(d: [f32; 2]) -> f32 {
    d[0].atan2(d[1])
}

fn wrap(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut a = (a + std::f32::consts::PI) % tau;
    if a < 0.0 {
        a += tau;
    }
    a - std::f32::consts::PI
}

impl Npc {
    pub fn new(clips: std::collections::HashMap<u16, Clip>, reaction: ReactionParams) -> Npc {
        Npc {
            clips,
            reaction,
            turn_speed: 0.2,
            facing: 0.0,
            mode: Mode::Idle,
            frame: 0,
            swing: 0,
            push: None,
            hop: None,
            airborne: false,
            fall_vel: 0.0,
            tick: 0,
            suction_to: None,
        }
    }

    pub fn facing(&self) -> f32 {
        self.facing
    }

    pub fn set_facing(&mut self, yaw: f32) {
        self.facing = wrap(yaw);
    }

    pub fn forward(&self) -> [f32; 2] {
        [self.facing.sin(), self.facing.cos()]
    }

    /// Busy: hurt, launched, down, dead or mid-action.
    pub fn busy(&self) -> bool {
        !matches!(self.mode, Mode::Idle | Mode::Walk)
    }

    pub fn is_dead(&self) -> bool {
        matches!(self.mode, Mode::Dead { .. })
    }

    fn start(&mut self, state: NpcState, motion: u16, next: Option<u16>) {
        self.swing += 1;
        self.mode = Mode::Clip { state, motion, frame: 0, next, swing: self.swing };
    }

    /// Applies the game's generic reaction to a hit.
    pub fn take_hit(&mut self, hit: &Hit) {
        if self.is_dead() {
            return;
        }
        let clips = &self.clips;
        let r = reaction::respond(hit, self.forward(), self.airborne, |m| clips.contains_key(&m), &self.reaction);
        self.apply(r, hit.dir);
    }

    fn apply(&mut self, r: Response, dir: [f32; 2]) {
        match r {
            Response::None => {}
            Response::Push { distance } => self.push = Some((dir, distance * 0.1)),
            Response::Flinch { motion, push } => {
                self.start(NpcState::Hurt, motion, None);
                if let Some(d) = push {
                    self.push = Some((dir, d * 0.1));
                }
            }
            Response::AirFlinch { motion, face, hop_speed } => {
                self.facing = yaw_of(face);
                self.start(NpcState::Hurt, motion, Some(motion::FALL));
                self.hop = Some((Arc::new(50.0, KH2_GRAVITY), [dir[0] * hop_speed, dir[1] * hop_speed]));
                self.airborne = true;
            }
            Response::Blow { motion, face, vel_h, up } => {
                self.facing = yaw_of(face);
                self.push = None;
                self.hop = None;
                self.airborne = true;
                self.mode = Mode::Blow { motion, frame: 0, blow: Blow::new(vel_h, up, &self.reaction), step: [0.0; 3] };
            }
        }
    }

    /// Repelled by a guard or a clashing attack: play `motion` (if the
    /// moveset has it) and get pushed `distance` along `dir`.
    pub fn repel(&mut self, motion: u16, distance: f32, dir: [f32; 2]) {
        if self.is_dead() {
            return;
        }
        if self.clips.contains_key(&motion) {
            self.start(NpcState::Hurt, motion, None);
        }
        self.push = Some((dir, distance * 0.1));
    }

    /// Caught by a Magnet (ATKP reaction 5). The game picks the orbit
    /// radius at random between the attack's two knockback values and
    /// takes the swirl speed from its next value, plays motion 20 (21 when
    /// hit from behind) and pulls every frame (see [`Npc::step`]).
    pub fn suction(&mut self, from_front: bool, radius: f32, swirl: f32) {
        if self.is_dead() {
            return;
        }
        let motion = if from_front { 20 } else { 21 };
        self.push = None;
        self.hop = None;
        self.airborne = true;
        self.mode = Mode::Suction { motion, frame: 0, radius, swirl };
    }

    /// Dies with `motion` (if present; otherwise holds the current pose).
    pub fn die(&mut self, motion: u16) {
        self.mode = Mode::Dead { motion, frame: 0 };
    }

    /// Advances one 60 Hz frame. `grounded`: the host's ground check.
    pub fn step(&mut self, cmd: &NpcCommand, grounded: bool) -> NpcOutput {
        let mut out = NpcOutput {
            displacement: [0.0; 3],
            facing: self.facing,
            state: NpcState::Idle,
            motion: motion::IDLE,
            clip_frame: 0,
            looping: true,
            root_motion: false,
            hitboxes: Vec::new(),
            guard_boxes: Vec::new(),
            invulnerable: false,
        };
        let physics_tick = self.tick % 2 == 0;
        self.tick += 1;
        let was_air = self.airborne;
        let rising = self.hop.as_ref().map_or(false, |h| h.0.rising());
        self.airborne = !grounded || rising || matches!(self.mode, Mode::Blow { frame: 0, .. });
        let landed = was_air && !self.airborne;

        if !self.busy() {
            if let Some(m) = cmd.action.filter(|m| self.clips.contains_key(m)) {
                self.start(NpcState::Action, m, None);
            }
        }

        let turn = |from: f32, to: [f32; 2], rate: f32| {
            let d = wrap(yaw_of(to) - from);
            if d.abs() <= rate { yaw_of(to) } else { wrap(from + rate * d.signum()) }
        };

        match self.mode.clone() {
            Mode::Idle | Mode::Walk => {
                if let Some(f) = cmd.face.or(cmd.walk) {
                    self.facing = turn(self.facing, f, self.turn_speed);
                }
                if cmd.walk.is_some() && self.clips.contains_key(&motion::WALK) {
                    if !matches!(self.mode, Mode::Walk) {
                        self.frame = 0;
                    }
                    self.mode = Mode::Walk;
                    let clip = &self.clips[&motion::WALK];
                    let len = clip.length.max(1);
                    let d = clip.root_delta(self.frame % len);
                    out.displacement = self.rotate(d);
                    out.displacement[1] = 0.0;
                    out.state = NpcState::Walk;
                    out.motion = motion::WALK;
                    out.clip_frame = self.frame % len;
                } else {
                    if !matches!(self.mode, Mode::Idle) {
                        self.frame = 0;
                    }
                    self.mode = Mode::Idle;
                    out.motion = motion::IDLE;
                    out.clip_frame = self.frame;
                }
                self.frame += 1;
            }
            Mode::Clip { state, motion, frame, next, swing } => {
                let clip = self.clips.get(&motion).cloned().unwrap_or_else(|| Clip::plain(motion, 1));
                out.state = state;
                out.motion = motion;
                out.clip_frame = frame;
                out.looping = false;
                out.root_motion = true;
                for (wi, w) in clip.windows.iter().enumerate() {
                    if !w.contains(frame) {
                        continue;
                    }
                    match w.kind {
                        WindowKind::Hitbox { attack, group, weapon, .. } => {
                            out.hitboxes.push(ActiveHitbox { swing: (swing << 8) | wi as u64, attack, group, weapon })
                        }
                        WindowKind::Guard { attack, group, weapon } => {
                            out.guard_boxes.push(ActiveHitbox { swing: (swing << 8) | wi as u64, attack, group, weapon })
                        }
                        WindowKind::Invulnerable => out.invulnerable = true,
                        _ => {}
                    }
                }
                let d = clip.root_delta(frame);
                out.displacement = self.rotate(d);
                if frame + 1 >= clip.length {
                    match next {
                        Some(m) if self.airborne => self.start(state, m, None),
                        _ => {
                            self.mode = Mode::Idle;
                            self.frame = 0;
                        }
                    }
                } else {
                    self.mode = Mode::Clip { state, motion, frame: frame + 1, next, swing };
                }
                if landed && next.is_some() {
                    self.mode = Mode::Idle;
                    self.frame = 0;
                }
            }
            Mode::Blow { motion, frame, mut blow, mut step } => {
                out.state = NpcState::Blown;
                out.motion = motion;
                out.clip_frame = frame;
                out.looping = false;
                if frame > 0 && grounded && blow.vel_y <= 0.0 {
                    // Landing: the motion after the launch (getting up).
                    self.airborne = false;
                    self.start(NpcState::Down, motion + 1, None);
                } else {
                    if physics_tick || frame == 0 {
                        let d = blow.tick(2.0, &self.reaction);
                        step = [d[0] / 2.0, d[1] / 2.0, d[2] / 2.0];
                    }
                    self.fall_vel = 0.0;
                    out.displacement = step;
                    self.mode = Mode::Blow { motion, frame: frame + 1, blow, step };
                    out.facing = self.facing;
                    return out;
                }
            }
            Mode::Suction { motion, frame, radius, swirl } => {
                out.state = NpcState::Suction;
                out.motion = motion;
                out.clip_frame = frame;
                out.looping = true;
                match self.suction_to {
                    Some(d) => {
                        // Toward the orbit radius at up to 20 a frame, plus a
                        // sideways swirl (exe).
                        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                        let n = if len > 1e-4 { [d[0] / len, d[1] / len, d[2] / len] } else { [0.0; 3] };
                        let radial = ((len - radius) * 0.1).clamp(-20.0, 20.0);
                        // Turned a quarter about the vertical, flattened.
                        let t = [n[2], 0.0, -n[0]];
                        let tl = (t[0] * t[0] + t[2] * t[2]).sqrt();
                        let t = if tl > 1e-4 { [t[0] / tl, 0.0, t[2] / tl] } else { [0.0; 3] };
                        out.displacement = [n[0] * radial + t[0] * swirl, n[1] * radial, n[2] * radial + t[2] * swirl];
                        if frame == 0 {
                            let f = if motion == 20 { [n[0], n[2]] } else { [-n[0], -n[2]] };
                            if f[0] != 0.0 || f[1] != 0.0 {
                                self.facing = yaw_of(f);
                            }
                        }
                        self.mode = Mode::Suction { motion, frame: frame + 1, radius, swirl };
                        self.fall_vel = 0.0;
                        out.facing = self.facing;
                        return out;
                    }
                    None => {
                        // Let go: drop as from a launch with no speed.
                        self.mode = Mode::Blow { motion: crate::reaction::motion::BLOW_FRONT, frame: 1, blow: Blow::new([0.0; 2], 0.0, &self.reaction), step: [0.0; 3] };
                        out.state = NpcState::Blown;
                    }
                }
            }
            Mode::Dead { motion, frame } => {
                out.state = NpcState::Dead;
                out.motion = motion;
                out.clip_frame = frame;
                out.looping = false;
                let len = self.clips.get(&motion).map_or(1, |c| c.length);
                self.mode = Mode::Dead { motion, frame: (frame + 1).min(len) };
            }
        }

        // Hit-back push (exe: 0.1 x distance per frame, x0.9 each frame,
        // dropped below 1).
        if let Some((dir, v)) = self.push {
            out.displacement[0] += dir[0] * v;
            out.displacement[2] += dir[1] * v;
            let next = v * 0.9;
            self.push = if next < 1.0 { None } else { Some((dir, next)) };
        }
        // Air-hit hop.
        if let Some((mut arc, v)) = self.hop {
            let dy = arc.advance(1.0, KH2_GRAVITY, self.reaction.fall_max);
            out.displacement[0] += v[0];
            out.displacement[1] += dy;
            out.displacement[2] += v[1];
            self.hop = if grounded && !arc.rising() { None } else { Some((arc, v)) };
        } else if self.airborne {
            // Plain fall.
            self.fall_vel = (self.fall_vel + KH2_GRAVITY).min(self.reaction.fall_max);
            out.displacement[1] -= self.fall_vel;
        } else {
            self.fall_vel = 0.0;
        }
        out.facing = self.facing;
        out
    }

    fn rotate(&self, d: [f32; 3]) -> [f32; 3] {
        let f = self.forward();
        let right = [f[1], -f[0]];
        [d[0] * right[0] + d[2] * f[0], d[1], d[0] * right[1] + d[2] * f[1]]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn npc() -> Npc {
        let mut clips = HashMap::new();
        let mut walk = Clip::plain(motion::WALK, 40);
        walk.root = (0..=40).map(|f| [0.0, 0.0, f as f32 * 5.0]).collect();
        clips.insert(motion::WALK, walk);
        clips.insert(motion::IDLE, Clip::plain(motion::IDLE, 50));
        for m in [10u16, 12, 14, 15, 16, 17, 18, 19] {
            clips.insert(m, Clip::plain(m, 30));
        }
        let mut atk = Clip::plain(201, 40);
        atk.windows.push(crate::Window { start: 10, end: Some(20), kind: WindowKind::Hitbox { attack: 5, group: 1, combo: None, weapon: 0 } });
        clips.insert(201, atk);
        Npc::new(clips, ReactionParams::default())
    }

    #[test]
    fn walks_by_root_motion_and_attacks_with_windows() {
        let mut n = npc();
        let o = n.step(&NpcCommand { walk: Some([0.0, 1.0]), ..Default::default() }, true);
        assert_eq!(o.displacement, [0.0, 0.0, 5.0]);
        let outs: Vec<_> = (0..40).map(|_| n.step(&NpcCommand { action: Some(201), ..Default::default() }, true)).collect();
        assert_eq!(outs.iter().filter(|o| !o.hitboxes.is_empty()).count(), 10);
        assert!(!n.busy());
    }

    #[test]
    fn small_hit_flinches_and_slides_back() {
        let mut n = npc();
        n.take_hit(&Hit { reaction: 3, knockback: (125, 0), dir: [0.0, -1.0], part: 0 });
        let outs: Vec<_> = (0..30).map(|_| n.step(&NpcCommand::default(), true)).collect();
        assert_eq!(outs[0].motion, 10);
        let moved: f32 = outs.iter().map(|o| o.displacement[2]).sum();
        assert!(moved < -100.0 && moved > -125.0, "pushed back about 110: {moved}");
        assert!(!n.busy());
    }

    #[test]
    fn launch_flies_then_lands_into_the_down_motion() {
        let mut n = npc();
        n.take_hit(&Hit { reaction: 7, knockback: (24, 40), dir: [0.0, -1.0], part: 0 });
        let mut y = 0.0f32;
        let mut z = 0.0f32;
        let mut states = Vec::new();
        for _ in 0..200 {
            let o = n.step(&NpcCommand::default(), y <= 0.0);
            y = (y + o.displacement[1]).max(0.0);
            z += o.displacement[2];
            states.push((o.state, o.motion));
        }
        assert_eq!(states[0], (NpcState::Blown, 16));
        assert!(states.iter().any(|s| *s == (NpcState::Down, 17)), "landed into 17");
        assert!(z < -200.0, "flew back: {z}");
    }

    #[test]
    fn magnet_pulls_into_an_orbit_then_drops() {
        let mut n = npc();
        n.suction(true, 100.0, -6.0);
        // Centre 300 ahead and 200 up; follow the pull.
        let c = [0.0f32, 200.0, 300.0];
        let mut p = [0.0f32; 3];
        for _ in 0..120 {
            n.suction_to = Some([c[0] - p[0], c[1] - p[1], c[2] - p[2]]);
            let o = n.step(&NpcCommand::default(), false);
            assert_eq!(o.state, NpcState::Suction);
            for i in 0..3 {
                p[i] += o.displacement[i];
            }
        }
        let d = ((c[0] - p[0]).powi(2) + (c[1] - p[1]).powi(2) + (c[2] - p[2]).powi(2)).sqrt();
        assert!((d - 100.0).abs() < 20.0, "orbiting near the radius: {d}");
        n.suction_to = None;
        let o = n.step(&NpcCommand::default(), false);
        assert_eq!(o.state, NpcState::Blown);
    }
}
