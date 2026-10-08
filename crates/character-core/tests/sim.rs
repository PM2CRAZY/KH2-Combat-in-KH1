//! Behaviour tests on flat ground with a made-up moveset (not KH2 data).

use character_core::*;

struct World {
    ch: Character,
    pos: [f32; 3],
    targets: Vec<TargetInfo>,
    prev: [bool; 4],
}

#[derive(Default, Clone, Copy)]
struct Pad {
    stick: [f32; 2],
    jump: bool,
    attack: bool,
    action: bool,
    lock: bool,
}

impl World {
    fn new(ch: Character) -> World {
        World { ch, pos: [0.0; 3], targets: Vec::new(), prev: [false; 4] }
    }

    fn step(&mut self, pad: Pad) -> Output {
        let input = Input {
            stick: pad.stick,
            jump: Button::from_held(self.prev[0], pad.jump),
            attack: Button::from_held(self.prev[1], pad.attack),
            action: Button::from_held(self.prev[2], pad.action),
            lock: Button::from_held(self.prev[3], pad.lock),
        };
        self.prev = [pad.jump, pad.attack, pad.action, pad.lock];
        let env = Env { pos: self.pos, grounded: self.pos[1] <= 0.0, height_above_ground: self.pos[1].max(0.0), targets: &self.targets };
        let out = self.ch.step(&input, &env);
        for i in 0..3 {
            self.pos[i] += out.displacement[i];
        }
        if self.pos[1] < 0.0 {
            self.pos[1] = 0.0;
        }
        out
    }

    fn run(&mut self, frames: usize, pad: Pad) -> Vec<Output> {
        (0..frames).map(|_| self.step(pad)).collect()
    }
}

fn growth() -> Vec<GrowthLevel> {
    (1..=4)
        .map(|l| GrowthLevel {
            high_jump_height: 200.0 + l as f32 * 50.0,
            air_dodge_height: 100.0,
            air_dodge_speed: 10.0,
            quick_run_time: 10.0,
            quick_run_speed: 30.0,
            quick_run_brake: 0.9,
            quick_run_stop_brake: 0.9,
            quick_run_star_time: 5.0,
            glide_speed: 16.0,
            glide_fall_ratio: 0.015,
            glide_fall_height: 0.0,
            glide_fall_max: 3.0,
            glide_acceleration: 0.9,
            glide_start_height: 60.0,
            glide_end_height: 10.0,
            glide_turn_speed: 0.05,
            dodge_roll_star_time: 10.0 * l as f32,
        })
        .collect()
}

fn roll_clip() -> Clip {
    let len = 40;
    let mut c = Clip::plain(motion::DODGE_ROLL, len);
    c.root = (0..=len).map(|f| [0.0, 0.0, f as f32 * 10.0]).collect();
    c.events.push(Event { frame: 30, kind: EventKind::FallCheck });
    c
}

fn attack_clip(motion: u16, len: u32, hit: (u32, u32), attack: u16, combo_at: u32) -> Clip {
    let mut c = Clip::plain(motion, len);
    c.windows.push(Window { start: hit.0, end: Some(hit.1), kind: WindowKind::Hitbox { attack, group: 1, combo: None } });
    c.windows.push(Window { start: combo_at, end: None, kind: WindowKind::Combo });
    c
}

fn row(id: u8, motion: u16, finisher: bool, air: bool) -> ComboRow {
    ComboRow {
        id, kind: 3, sub: -1, flag: if finisher { FLAG_FINISHER } else { 0 }, motion, next_motion: 0,
        jump: if air { 20.0 } else { 0.0 }, speed_min: 0.0, speed_max: 0.0, near: 0.0, far: 0.0,
        low: 0.0, high: 0.0, ability: 0, score: 1,
    }
}

fn sora() -> Character {
    let mut ms = Moveset::default();
    let mut jump = Clip::plain(motion::JUMP, 10);
    jump.events.push(Event { frame: 4, kind: EventKind::Jump });
    ms.clips.insert(motion::JUMP, jump);
    let mut land = Clip::plain(motion::LAND, 20);
    land.windows.push(Window { start: 8, end: None, kind: WindowKind::AllowReactionCommand });
    ms.clips.insert(motion::LAND, land);
    ms.clips.insert(motion::DODGE_ROLL, roll_clip());
    ms.clips.insert(100, attack_clip(100, 40, (10, 16), 1, 20));
    ms.clips.insert(101, attack_clip(101, 40, (10, 16), 2, 20));
    ms.clips.insert(110, attack_clip(110, 50, (12, 20), 3, 60));
    ms.clips.insert(120, attack_clip(120, 30, (5, 10), 4, 15));
    ms.combo_rows = vec![row(0, 100, false, false), row(1, 110, true, false), row(2, 120, false, true), row(3, 120, true, true)];
    let mut ch = Character::new(ms, MoveParams::default(), growth(), AttackParams::default());
    ch.abilities = Abilities::max_growth();
    ch
}

const IDLE: Pad = Pad { stick: [0.0, 0.0], jump: false, attack: false, action: false, lock: false };

#[test]
fn runs_at_run_speed_and_turns_at_capped_rate() {
    let mut w = World::new(sora());
    w.run(30, Pad { stick: [0.0, 1.0], ..IDLE });
    let z0 = w.pos[2];
    w.run(10, Pad { stick: [0.0, 1.0], ..IDLE });
    assert!((w.pos[2] - z0 - 80.0).abs() < 1e-3, "run 8/frame");
    // 180-degree turn takes ceil(pi / 0.25) = 13 frames.
    let outs = w.run(13, Pad { stick: [0.0, -1.0], ..IDLE });
    assert!((outs[11].facing.abs() - std::f32::consts::PI).abs() > 0.01);
    assert!((outs[12].facing.abs() - std::f32::consts::PI).abs() < 1e-4);
}

#[test]
fn jump_has_takeoff_frames_and_reaches_height() {
    let mut w = World::new(sora());
    let outs = w.run(4, Pad { jump: true, ..IDLE });
    assert!(outs[..4].iter().all(|o| o.state == StateName::Jump));
    assert_eq!(w.pos[1], 0.0, "still on the ground during takeoff");
    let mut apex = 0.0f32;
    for _ in 0..120 {
        w.step(IDLE);
        apex = apex.max(w.pos[1]);
    }
    // High Jump MAX (level 4) = 400 in this fixture.
    assert!((apex - 400.0).abs() < 15.0, "apex {apex}");
    assert_eq!(w.pos[1], 0.0);
}

#[test]
fn landing_recovery_then_free() {
    let mut w = World::new(sora());
    w.ch.abilities.high_jump = 0;
    w.run(1, Pad { jump: true, ..IDLE });
    let mut landed_at = None;
    for f in 0..200 {
        let o = w.step(Pad { stick: [0.0, 1.0], ..IDLE });
        if o.state == StateName::Land && landed_at.is_none() {
            landed_at = Some(f);
        }
        if let Some(l) = landed_at {
            if o.state != StateName::Land {
                assert_eq!(f - l, 8, "8 recovery frames, then the frame-8 window lets the stick through");
                return;
            }
        }
    }
    panic!("never landed / recovered");
}

#[test]
fn dodge_roll_uses_root_motion_and_level_iframes() {
    let mut w = World::new(sora());
    w.ch.abilities.dodge_roll = 2; // 20 i-frames in this fixture
    let outs = w.run(40, Pad { stick: [1.0, 0.0], action: true, ..IDLE });
    assert_eq!(outs[0].state, StateName::DodgeRoll);
    let inv = outs.iter().take_while(|o| o.invulnerable).count();
    assert_eq!(inv, 20);
    assert!((w.pos[0] - 400.0).abs() < 1e-3, "rolled along +x by the clip's root motion: {:?}", w.pos);
}

#[test]
fn ground_combo_buffers_and_ends_with_finisher() {
    let mut w = World::new(sora());
    let mut motions = Vec::new();
    let mut hits = Vec::new();
    for f in 0..200 {
        let mash = f % 2 == 0;
        let o = w.step(Pad { attack: mash, ..IDLE });
        if o.state == StateName::Attack && o.clip_frame == 0 {
            motions.push(o.motion);
        }
        for h in o.hitboxes {
            hits.push((f, h.attack));
        }
    }
    // Base combo of 3: two normal hits then the finisher.
    assert_eq!(motions[..3], [100, 100, 110]);
    // The second hit starts exactly when the combo window opens (frame 20).
    let second = hits.iter().find(|h| h.1 == 1 && h.0 >= 20).unwrap();
    assert_eq!(second.0, 20 + 10);
}

#[test]
fn glide_needs_height_and_held_button() {
    let mut w = World::new(sora());
    w.run(1, Pad { jump: true, ..IDLE });
    w.run(12, IDLE);
    assert!(w.pos[1] > 60.0);
    let o = w.step(Pad { jump: true, ..IDLE });
    assert_eq!(o.state, StateName::Glide);
    let outs = w.run(30, Pad { jump: true, stick: [0.0, 1.0], ..IDLE });
    assert!(outs.iter().all(|o| o.displacement[1] >= -3.0 - 1e-4), "glide descent capped");
    let o = w.step(Pad { jump: false, ..IDLE });
    assert_ne!(o.state, StateName::Glide);
}

#[test]
fn aerial_dodge_once_per_airtime() {
    let mut w = World::new(sora());
    w.run(1, Pad { jump: true, ..IDLE });
    w.run(10, IDLE);
    let o = w.step(Pad { action: true, ..IDLE });
    assert_eq!(o.state, StateName::AerialDodge);
    w.run(3, IDLE);
    let o = w.step(Pad { action: true, ..IDLE });
    assert_ne!(o.state, StateName::AerialDodge);
}

#[test]
fn lock_on_picks_nearest_and_releases() {
    let mut w = World::new(sora());
    w.targets = vec![TargetInfo { id: 7, pos: [0.0, 0.0, 500.0] }, TargetInfo { id: 9, pos: [0.0, 0.0, 300.0] }];
    let o = w.step(Pad { lock: true, ..IDLE });
    assert_eq!(o.locked, Some(9));
    w.targets[1].pos = [0.0, 0.0, 5000.0];
    let o = w.step(IDLE);
    assert_eq!(o.locked, None);
}
