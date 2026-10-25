//! Behaviour tests on flat ground with a made-up moveset (not KH2 data).

use character_core::*;

struct World {
    ch: Character,
    pos: [f32; 3],
    targets: Vec<TargetInfo>,
    ledges: Vec<Ledge>,
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
        World { ch, pos: [0.0; 3], targets: Vec::new(), ledges: Vec::new(), prev: [false; 4] }
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
        let env = Env { pos: self.pos, grounded: self.pos[1] <= 0.0, height_above_ground: self.pos[1].max(0.0), targets: &self.targets, ledges: &self.ledges };
        let out = self.ch.step(&input, &env);
        for i in 0..3 {
            self.pos[i] += out.displacement[i];
        }
        if self.pos[1] < 0.0 {
            self.pos[1] = 0.0;
        }
        out
    }

    fn outs_free(&mut self) -> bool {
        matches!(self.step(IDLE).state, StateName::Idle | StateName::Land)
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
            quick_run_stop_brake: 0.5,
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
    c.events.push(Event { frame: 10, kind: EventKind::QuickRunCheck });
    c
}

fn attack_clip(motion: u16, len: u32, hit: (u32, u32), attack: u16, combo_at: u32) -> Clip {
    let mut c = Clip::plain(motion, len);
    c.windows.push(Window { start: hit.0, end: Some(hit.1), kind: WindowKind::Hitbox { attack, group: 1, combo: None, weapon: 1 } });
    c.windows.push(Window { start: combo_at, end: None, kind: WindowKind::Combo });
    c
}

fn row(id: u8, motion: u16, finisher: bool, air: bool) -> ComboRow {
    ComboRow {
        id, kind: 3, sub: -1, combo_offset: 0,
        flag: if finisher { FLAG_FINISHER } else { 0 } | if air { FLAG_AIR } else { 0 },
        motion, next_motion: 0, jump: if air { 20.0 } else { 0.0 }, jump_min: 0.0, jump_max: 0.0,
        speed_min: 0.0, speed_max: 0.0, near: 0.0, far: 0.0, low: 0.0, high: 0.0, inner_min: -2.0,
        inner_max: 2.0, blend_time: 0.0, distance_adjust: 0.0, ability: 0, score: 1,
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
    // Finishers first: the game takes the first valid row in table order.
    ms.combo_rows = vec![row(1, 110, true, false), row(0, 100, false, false), row(3, 120, true, true), row(2, 120, false, true)];
    let mut ch = Character::new(ms, MoveParams::default(), growth(), AttackParams::default());
    ch.abilities = Abilities::max_growth();
    ch
}

const IDLE: Pad = Pad { stick: [0.0, 0.0], jump: false, attack: false, action: false, lock: false };

impl Pad {
    /// Attack held only on even frames (a mash), when `attack` is set.
    fn with_attack(self, f: usize) -> Pad {
        Pad { attack: self.attack && f % 2 == 0, ..self }
    }
}

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
        w.step(Pad { jump: true, ..IDLE });
        apex = apex.max(w.pos[1]);
    }
    // High Jump MAX (level 4) = 400 in this fixture. Sampled every
    // 2-frame tick, so the apex is within one step of the true height.
    assert!(apex <= 400.0 + 1e-3 && apex > 399.0, "apex {apex}");
    assert_eq!(w.pos[1], 0.0);
}

#[test]
fn landing_keeps_running_with_the_stick_and_stops_dead_without() {
    // Holding the stick: landing hands over to walk/run at once.
    let mut w = World::new(sora());
    w.ch.abilities.high_jump = 0;
    w.run(1, Pad { jump: true, stick: [0.0, 1.0], ..IDLE });
    let outs: Vec<_> = (0..200).map(|_| w.step(Pad { stick: [0.0, 1.0], ..IDLE })).collect();
    let land: Vec<&Output> = outs.iter().filter(|o| o.state == StateName::Land).collect();
    // KH2 holds him in the landing for a moment (the user's KH2 video), then he runs.
    assert_eq!(land.len(), 14, "landing frames with stick held");
    assert!(land.iter().all(|o| o.displacement[0] == 0.0 && o.displacement[2] == 0.0), "still while landing");
    assert!(matches!(outs.last().unwrap().state, StateName::Run | StateName::Walk));

    // Neutral stick: the land motion plays and Sora does not slide.
    let mut w = World::new(sora());
    w.ch.abilities.high_jump = 0;
    w.run(1, Pad { jump: true, stick: [0.0, 1.0], ..IDLE });
    let mut landed = None;
    for f in 0..200 {
        let before = w.pos;
        let o = w.step(if landed.is_none() && f < 10 { Pad { stick: [0.0, 1.0], ..IDLE } } else { IDLE });
        if o.state == StateName::Land {
            landed.get_or_insert(f);
            assert!((w.pos[0] - before[0]).abs() < 1e-4 && (w.pos[2] - before[2]).abs() < 1e-4, "slid while landing");
        }
    }
    assert!(landed.is_some());
}

#[test]
fn dodge_roll_uses_root_motion_and_level_iframes() {
    let mut w = World::new(sora());
    w.ch.abilities.dodge_roll = 2; // 20 i-frames in this fixture
    let outs: Vec<_> = (0..40).map(|f| w.step(if f < 3 { Pad { stick: [1.0, 0.0], action: true, ..IDLE } } else { IDLE })).collect();
    assert_eq!(outs[0].state, StateName::DodgeRoll);
    let inv = outs.iter().take_while(|o| o.invulnerable).count();
    assert_eq!(inv, 20);
    assert!((w.pos[0] - 400.0).abs() < 1e-3, "rolled along +x by the clip's root motion: {:?}", w.pos);

    // Holding the stick: the roll's recovery is walked out of from its fall-check event.
    let mut w = World::new(sora());
    w.ch.abilities.dodge_roll = 2;
    let outs: Vec<_> = (0..40).map(|f| w.step(Pad { stick: [1.0, 0.0], action: f < 3, ..IDLE })).collect();
    let rolled = outs.iter().take_while(|o| o.state == StateName::DodgeRoll).count();
    assert!(rolled < 40, "still rolling with the stick held after the fall check: {rolled} frames");
}

fn mash(w: &mut World, connect: bool) -> (Vec<u16>, Vec<(usize, u16)>) {
    let mut motions = Vec::new();
    let mut hits = Vec::new();
    for f in 0..200 {
        let o = w.step(Pad { attack: f % 2 == 0, ..IDLE });
        if o.state == StateName::Attack && o.clip_frame == 0 {
            motions.push(o.motion);
        }
        if connect && !o.hitboxes.is_empty() {
            w.ch.register_hit();
        }
        for h in o.hitboxes {
            hits.push((f, h.attack));
        }
    }
    (motions, hits)
}

#[test]
fn host_swing_playing_is_passed_over() {
    // Two ground swings (rows 0, 4) and the finisher: with the host's own swing of row 0's motion
    // playing, the selector walks on to the next valid row, as for its own swing (exe).
    let mut ch = sora();
    let mut second = row(4, 101, false, false);
    second.sub = 4;
    ch.moveset.combo_rows.push(second);
    let env = Env { pos: [0.0; 3], grounded: true, height_above_ground: 0.0, targets: &[], ledges: &[] };
    let from = Some(ComboState { count: 1, finishers: 1, hit: true, finisher_next: false });
    let first = ch.would_take_after(from, None, false, false, [0.0; 2], &env).unwrap();
    assert_eq!(first.motion, 100);
    let next = ch.would_take_after(from, Some(100), false, false, [0.0; 2], &env).unwrap();
    assert_eq!(next.motion, 101);
    // Not playing anything of the host's: the same as `would_take`.
    assert_eq!(ch.would_take(from, false, false, [0.0; 2], &env).unwrap().motion, 100);
}

#[test]
fn host_trial_gives_the_rows_hop() {
    // A ground row that hops (KH2's rows 0 / 1): the trial gives its launch, the row's own height
    // and speed with no target; a plain swing gives none.
    let mut ch = sora();
    let env = Env { pos: [0.0; 3], grounded: true, height_above_ground: 0.0, targets: &[], ledges: &[] };
    let (r, l) = ch.would_take_launch(None, None, false, false, [0.0; 2], &env).unwrap();
    assert_eq!(r.motion, 100);
    assert!(l.is_none());
    let mut hop = row(4, 101, false, false);
    (hop.jump, hop.jump_min, hop.jump_max, hop.speed_max) = (20.0, 20.0, 20.0, 5.0);
    ch.moveset.combo_rows.insert(0, hop);
    let (r, l) = ch.would_take_launch(None, None, false, false, [0.0; 2], &env).unwrap();
    assert_eq!(r.motion, 101);
    let l = l.unwrap();
    assert_eq!(l.height, 20.0);
    assert!((l.velocity[0].hypot(l.velocity[1]) - 5.0).abs() < 1e-4);
    assert_eq!(l.gravity, ch.params.gravity);
    // Nothing of the controller changed.
    assert!(!ch.airborne());
}

#[test]
fn a_rows_hop_at_a_target_is_kh2s() {
    // KH2's rule (Steam KH2 `0x1403D78D0`): from his own target shape's centre (70 up, radius 55)
    // to the target's, the height bounded by the row's jump window, the gap less both radii plus
    // `distance_adjust` over KH2's rise time for that height (its own gravity constant).
    let mut ch = sora();
    ch.attack.target_lift = 70.0;
    ch.attack.target_radius = 55.0;
    let mut hop = row(4, 101, false, false);
    (hop.jump, hop.jump_min, hop.jump_max, hop.speed_min, hop.speed_max, hop.distance_adjust) = (17.0, 17.0, 67.0, -8.0, 18.0, 85.0);
    ch.moveset.combo_rows.insert(0, hop);
    ch.params.gravity = 0.3;
    let targets = [TargetInfo { id: 7, pos: [0.0, 100.0, 200.0], radius: 30.0, height: 100.0 }];
    ch.set_lock(Some(7));
    let env = Env { pos: [0.0; 3], grounded: true, height_above_ground: 0.0, targets: &targets, ledges: &[] };
    let (_, l) = ch.would_take_launch(None, None, false, false, [0.0; 2], &env).unwrap();
    let l = l.unwrap();
    assert_eq!(l.height, 30.0);
    let up = (60.0f32 / character_core::KH2_GRAVITY).sqrt();
    let s = 200.0 / up;
    assert!((l.velocity[1] - s).abs() < 1e-3 && l.velocity[0].abs() < 1e-4, "{:?} vs {s}", l.velocity);
    // A target below his target shape's centre: the jump window's low end; close in, the gap
    // less both radii plus `distance_adjust` (not held at 0 first).
    let targets = [TargetInfo { id: 7, pos: [0.0, 20.0, 10.0], radius: 30.0, height: 100.0 }];
    let env = Env { targets: &targets, ..env };
    let (_, l) = ch.would_take_launch(None, None, false, false, [0.0; 2], &env).unwrap();
    let l = l.unwrap();
    assert_eq!(l.height, 17.0);
    let up = (34.0f32 / character_core::KH2_GRAVITY).sqrt();
    assert!((l.velocity[1] - (10.0 - 85.0 + 85.0) / up).abs() < 1e-3, "{:?}", l.velocity);
}

#[test]
fn whiffed_swings_do_not_advance_the_combo() {
    let mut w = World::new(sora());
    let (motions, _) = mash(&mut w, false);
    assert!(motions.len() >= 4);
    assert!(motions.iter().all(|&m| m == 100), "{motions:?}");
}

#[test]
fn ground_combo_buffers_and_ends_with_finisher() {
    let mut w = World::new(sora());
    let (motions, hits) = mash(&mut w, true);
    // Base combo of 3: two normal hits then the finisher.
    assert_eq!(motions[..3], [100, 100, 110]);
    // The second hit starts exactly when the combo window opens (frame 20).
    let second = hits.iter().find(|h| h.1 == 1 && h.0 >= 20).unwrap();
    assert_eq!(second.0, 20 + 10);
}

#[test]
fn glide_needs_height_and_held_button() {
    let mut w = World::new(sora());
    w.ch.abilities.aerial_dodge = 0;
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
fn glide_on_the_action_button() {
    let mut w = World::new(sora());
    w.ch.glide_button = GlideButton::Action;
    w.run(1, Pad { jump: true, ..IDLE });
    w.run(10, IDLE);
    // Jump in mid-air still Aerial Dodges...
    let o = w.step(Pad { jump: true, ..IDLE });
    assert_eq!(o.state, StateName::AerialDodge);
    w.run(20, IDLE);
    // ...and with the dodge spent, it no longer glides.
    let o = w.step(Pad { jump: true, ..IDLE });
    assert_ne!(o.state, StateName::Glide);
    w.run(1, IDLE);
    // The action button glides, held.
    let o = w.step(Pad { action: true, ..IDLE });
    assert_eq!(o.state, StateName::Glide);
    let outs = w.run(10, Pad { action: true, stick: [0.0, 1.0], ..IDLE });
    assert!(outs.iter().all(|o| o.state == StateName::Glide));
    // Holding jump doesn't keep it going.
    let o = w.step(Pad { jump: true, ..IDLE });
    assert_ne!(o.state, StateName::Glide);
}

#[test]
fn action_button_mid_dodge_glides() {
    let mut w = World::new(sora());
    w.ch.glide_button = GlideButton::Action;
    w.run(1, Pad { jump: true, ..IDLE });
    w.run(12, IDLE);
    let o = w.step(Pad { jump: true, ..IDLE });
    assert_eq!(o.state, StateName::AerialDodge);
    w.run(3, IDLE);
    let o = w.step(Pad { action: true, ..IDLE });
    assert_eq!(o.state, StateName::Glide);
}

#[test]
fn aerial_dodge_once_per_airtime() {
    let mut w = World::new(sora());
    w.run(1, Pad { jump: true, ..IDLE });
    w.run(10, IDLE);
    // KH2: Aerial Dodge is the jump button in mid-air.
    let o = w.step(Pad { jump: true, ..IDLE });
    assert_eq!(o.state, StateName::AerialDodge);
    w.run(20, IDLE);
    // Second press this airtime: glide instead (high enough), never a second dodge.
    let o = w.step(Pad { jump: true, ..IDLE });
    assert_ne!(o.state, StateName::AerialDodge);
    assert_eq!(o.state, StateName::Glide);
    // Walking off a ledge (no jump) never allows the dodge.
    let mut w = World::new(sora());
    w.ch.abilities.glide = 0;
    w.pos[1] = 300.0;
    w.run(3, IDLE);
    let o = w.step(Pad { jump: true, ..IDLE });
    assert_ne!(o.state, StateName::AerialDodge);
}

#[test]
fn lock_on_picks_nearest_and_releases() {
    let mut w = World::new(sora());
    let t = |id, z| TargetInfo { id, pos: [0.0, 0.0, z], radius: 30.0, height: 100.0 };
    w.targets = vec![t(7, 500.0), t(9, 300.0)];
    let o = w.step(Pad { lock: true, ..IDLE });
    assert_eq!(o.locked, Some(9));
    w.targets[1].pos = [0.0, 0.0, 5000.0];
    let o = w.step(IDLE);
    assert_eq!(o.locked, None);
}

fn apex_with_hold(frames: usize) -> f32 {
    let mut w = World::new(sora());
    let mut apex = 0.0f32;
    for f in 0..160 {
        w.step(Pad { jump: f < frames, ..IDLE });
        apex = apex.max(w.pos[1]);
    }
    apex
}

#[test]
fn jump_height_depends_on_hold() {
    let tap = apex_with_hold(1);
    let mid = apex_with_hold(20);
    let full = apex_with_hold(200);
    assert!(tap < mid && mid < full, "tap {tap} mid {mid} full {full}");
    assert!((full - 400.0).abs() < 1.0);
    assert!(tap < full * 0.5, "tap {tap}");
}

#[test]
fn quick_run_ends_on_release_and_roll_chains_into_it() {
    // Tap: the dash stops early.
    let mut w = World::new(sora());
    w.ch.abilities.dodge_roll = 0;
    let outs: Vec<_> = (0..40).map(|f| w.step(Pad { stick: [1.0, 0.0], action: f < 2, ..IDLE })).collect();
    let tap = w.pos[0];
    assert_eq!(outs[0].state, StateName::QuickRun);
    let mut w = World::new(sora());
    w.ch.abilities.dodge_roll = 0;
    w.run(40, Pad { stick: [1.0, 0.0], action: true, ..IDLE });
    assert!(w.pos[0] > tap + 50.0, "held {} vs tap {tap}", w.pos[0]);
}

/// KH2's Quick Run (an air slide) ends in the landing, not straight in idle (the user).
#[test]
fn quick_run_ends_in_the_landing() {
    let mut ch = sora();
    ch.abilities.dodge_roll = 0;
    ch.moveset.clips.insert(motion::LAND, Clip::plain(motion::LAND, 20));
    let mut w = World::new(ch);
    let outs: Vec<_> = (0..60).map(|f| w.step(Pad { stick: [1.0, 0.0], action: f < 2, ..IDLE })).collect();
    let qr = outs.iter().rposition(|o| o.state == StateName::QuickRun).expect("no Quick Run");
    assert_eq!(outs[qr + 1].state, StateName::Land, "{:?}", outs.iter().map(|o| o.state).collect::<Vec<_>>());
}

#[test]
fn holding_through_the_roll_turns_it_into_quick_run() {
    let mut w = World::new(sora());
    let outs = w.run(20, Pad { stick: [1.0, 0.0], action: true, ..IDLE });
    assert_eq!(outs[5].state, StateName::DodgeRoll);
    assert_eq!(outs[12].state, StateName::QuickRun);
}

fn hanger() -> World {
    let mut ch = sora();
    ch.abilities.high_jump = 0;
    ch.params.hang = HangParams { height: 150.0, margin: 210.0, range_h: 30.0, range_l: 60.0, range_xz: 20.0, body_radius: 90.0 };
    ch.moveset.clips.insert(motion::HANG, Clip::plain(motion::HANG, 60));
    let mut climb = Clip::plain(motion::CLIMB, 30);
    climb.root = (0..=30).map(|f| [0.0, (f as f32 * 20.0).min(200.0), f as f32 * 110.0 / 30.0]).collect();
    ch.moveset.clips.insert(motion::CLIMB, climb);
    let mut w = World::new(ch);
    // A wall whose top is 280 up, 100 ahead (+z); too high to jump onto.
    w.ledges.push(Ledge { a: [-500.0, 280.0, 100.0], b: [500.0, 280.0, 100.0], out: [0.0, -1.0] });
    w
}

#[test]
fn grabs_a_ledge_after_the_apex_and_hangs() {
    let mut w = hanger();
    // Mostly sideways: grabs (stick toward the wall at all) but doesn't climb.
    let side = Pad { stick: [0.3, 0.05], ..IDLE };
    let mut hung = false;
    for _ in 0..120 {
        if w.step(Pad { jump: true, ..side }).state == StateName::Hang {
            hung = true;
            break;
        }
    }
    assert!(hung, "never grabbed");
    assert!((w.pos[1] - 130.0).abs() < 1e-3 && (w.pos[2] - 10.0).abs() < 1e-3, "hangs 150 below, 90 out: {:?}", w.pos);
    let x = w.pos[0];
    w.run(20, IDLE);
    assert!((w.pos[1] - 130.0).abs() < 1e-3 && w.pos[0] == x, "holds still");
    // Push toward the wall: climb by the clip's root motion.
    let outs = w.run(31, Pad { stick: [0.0, 1.0], ..IDLE });
    assert_eq!(outs[0].state, StateName::Climb);
    assert!((w.pos[2] - 120.0).abs() < 1.0 && (w.pos[1] - 330.0).abs() < 1.0, "climbed onto the top: {:?}", w.pos);
}

#[test]
fn holding_toward_the_wall_grabs_then_a_fresh_push_climbs() {
    let mut w = hanger();
    let toward = Pad { stick: [0.0, 0.2], ..IDLE };
    let outs: Vec<_> = (0..120).map(|_| w.step(Pad { jump: true, ..toward })).collect();
    assert!(outs.iter().any(|o| o.state == StateName::Hang), "grabbed");
    assert!(outs.iter().all(|o| o.state != StateName::Climb), "still holding: hangs, doesn't climb");
    w.run(3, IDLE);
    let o = w.step(Pad { stick: [0.0, 1.0], ..IDLE });
    assert_eq!(o.state, StateName::Climb);
}

#[test]
fn jump_also_climbs_from_a_hang() {
    let mut w = hanger();
    let toward = Pad { stick: [0.0, 0.2], ..IDLE };
    for _ in 0..120 {
        if w.step(Pad { jump: true, ..toward }).state == StateName::Hang {
            break;
        }
    }
    w.step(toward);
    assert_eq!(w.step(Pad { jump: true, ..toward }).state, StateName::Climb);
}

#[test]
fn aerial_recovery_rights_sora_mid_launch() {
    let mut ch = sora();
    hurt_clips(&mut ch);
    ch.moveset.clips.insert(motion::AERIAL_DODGE, Clip::plain(motion::AERIAL_DODGE, 20));
    ch.abilities.aerial_recovery = true;
    let mut w = World::new(ch);
    w.step(IDLE);
    w.ch.take_hit(&reaction::Hit { reaction: 7, knockback: (24, 40), dir: [0.0, -1.0], part: 0 });
    // Not before the launch's 16th frame (exe).
    w.run(5, IDLE);
    let o = w.step(Pad { jump: true, ..IDLE });
    assert_ne!(o.state, StateName::Hurt);
    w.run(11, IDLE);
    let z = w.pos[2];
    let o = w.step(Pad { jump: true, ..IDLE });
    assert_eq!(o.state, StateName::Hurt);
    w.run(90, IDLE);
    assert!((w.pos[2] - z).abs() < 30.0, "stopped flying back: {} -> {}", z, w.pos[2]);
    assert!(w.outs_free());
}
#[test]
fn no_grab_when_the_ledge_is_too_close_to_the_ground() {
    let mut w = hanger();
    w.ledges[0].a[1] = 200.0;
    w.ledges[0].b[1] = 200.0;
    let toward = Pad { stick: [0.0, 0.2], ..IDLE };
    let outs: Vec<_> = (0..120).map(|f| w.step(Pad { jump: f < 60, ..toward })).collect();
    assert!(outs.iter().all(|o| o.state != StateName::Hang));
}

fn hurt_clips(ch: &mut Character) {
    for m in [10u16, 11, 12, 13, 14, 15, 16, 17, 18, 19] {
        let mut c = Clip::plain(m, 30);
        c.windows.push(Window { start: 8, end: None, kind: WindowKind::AllowReactionCommand });
        // (KH2's free state, range id 2, late in the flinch: 50 of 70 in Sora's 10..13.)
        c.windows.push(Window { start: 20, end: None, kind: WindowKind::Free });
        ch.moveset.clips.insert(m, c);
    }
}

#[test]
fn no_jump_or_roll_out_of_a_swing_before_its_end() {
    // KH2: a swing's combo window takes the next Attack, not a jump or the action button; those
    // come only in a free state (range id 0 / 1 / 2), which Sora's swings don't have, so at the
    // swing's end (the tester: "you can jump and dodge roll out of moves during its end frames").
    let mut w = World::new(sora());
    w.step(Pad { attack: true, ..IDLE });
    assert_eq!(w.step(IDLE).state, StateName::Attack);
    w.run(22, IDLE);
    assert_eq!(w.step(Pad { jump: true, ..IDLE }).state, StateName::Attack);
    assert_eq!(w.step(Pad { action: true, stick: [1.0, 0.0], ..IDLE }).state, StateName::Attack);
    // The swing's own free state, if a clip has one, lets them through.
    let mut ch = sora();
    ch.moveset.clips.get_mut(&100).unwrap().windows.push(Window { start: 30, end: None, kind: WindowKind::Free });
    let mut w = World::new(ch);
    w.step(Pad { attack: true, ..IDLE });
    w.run(31, IDLE);
    assert_eq!(w.step(Pad { jump: true, ..IDLE }).state, StateName::Jump);
}

#[test]
fn sora_flinches_and_is_pushed_back_by_a_small_hit() {
    let mut ch = sora();
    hurt_clips(&mut ch);
    let mut w = World::new(ch);
    w.step(IDLE);
    // Facing +z, hit from the front (push along -z), knockback 125.
    w.ch.take_hit(&reaction::Hit { reaction: 3, knockback: (125, 0), dir: [0.0, -1.0], part: 0 });
    let outs = w.run(30, IDLE);
    assert_eq!(outs[0].state, StateName::Hurt);
    assert_eq!(outs[0].motion, 10);
    assert!(w.pos[2] < -100.0 && w.pos[2] > -125.0, "{:?}", w.pos);
    // A jump cuts the flinch only from its free state (KH2's range id 2), not from the
    // reaction-command window (frame 8); an Attack doesn't (the user: no attacking out of a
    // damage reaction).
    let mut w2 = World::new({ let mut c = sora(); hurt_clips(&mut c); c });
    w2.step(IDLE);
    w2.ch.take_hit(&reaction::Hit { reaction: 3, knockback: (125, 0), dir: [0.0, -1.0], part: 0 });
    w2.run(8, IDLE);
    let o = w2.step(Pad { jump: true, ..IDLE });
    assert_eq!(o.state, StateName::Hurt);
    w2.run(12, IDLE);
    let o = w2.step(Pad { attack: true, ..IDLE });
    assert_eq!(o.state, StateName::Hurt);
    let o = w2.step(Pad { jump: true, ..IDLE });
    assert_ne!(o.state, StateName::Hurt);
}

#[test]
fn sora_is_launched_lands_and_gets_up() {
    let mut ch = sora();
    hurt_clips(&mut ch);
    let mut w = World::new(ch);
    w.step(IDLE);
    w.ch.take_hit(&reaction::Hit { reaction: 7, knockback: (24, 40), dir: [0.0, -1.0], part: 0 });
    let outs = w.run(120, IDLE);
    assert_eq!(outs[0].state, StateName::Blown);
    assert!(outs.iter().any(|o| o.state == StateName::Blown && w.pos[1] >= 0.0));
    let up = outs.iter().position(|o| o.motion == 17).expect("got up with the motion after the launch");
    assert!(up > 10);
    assert!(w.pos[2] < -200.0, "{:?}", w.pos);
}

/// `air_arc` says where on its arc he is (the host writes it into its own physics state): its
/// time, a frame short after a physics tick's first frame, gives his height every frame.
#[test]
fn air_arc_matches_where_he_is() {
    let mut w = World::new(sora());
    w.ch.abilities.high_jump = 0;
    w.step(Pad { jump: true, ..IDLE });
    let mut seen = 0;
    for _ in 0..80 {
        w.step(Pad { jump: true, ..IDLE });
        if let Some((a, first)) = w.ch.air_arc() {
            let g = w.ch.params.gravity;
            if a.rising() {
                // (Mid-tick, a frame short of the arc's time, he has gone half the tick's rise in a
                // straight line.)
                let at = if first { (a.y(a.t, g) + a.y((a.t - 2.0).max(0.0), g)) / 2.0 } else { a.y(a.t, g) };
                assert!((at - w.pos[1]).abs() < 1e-3, "t {}: arc {at} at {}", a.t, w.pos[1]);
                seen += 1;
            }
        }
    }
    assert!(seen > 10, "{seen} rising frames");
    assert!(w.ch.air_arc().is_none(), "landed: no arc");
}

#[test]
fn carried_into_the_air_falls_on_at_its_speed() {
    // Handed over mid-fall (off a bar): moving 6 sideways and 3 down per frame.
    let mut w = World::new(sora());
    w.pos = [0.0, 300.0, 0.0];
    w.ch.carry_into_air([6.0, 0.0], -3.0);
    let o = w.step(IDLE);
    assert_eq!(o.state, StateName::Air);
    assert!(o.displacement[0] > 4.0, "keeps going sideways: {:?}", o.displacement);
    assert!(o.displacement[1] < -2.5, "falls on, not from rest: {:?}", o.displacement);
    let mut falls = 0;
    while w.pos[1] > 0.0 && falls < 300 {
        w.step(IDLE);
        falls += 1;
    }
    assert!(falls < 300 && w.pos[1] == 0.0, "lands");
}

#[test]
fn jump_out_of_the_landing_hold() {
    let mut w = World::new(sora());
    w.ch.abilities.high_jump = 0;
    w.run(1, Pad { jump: true, ..IDLE });
    let mut f = 0;
    while w.step(IDLE).state != StateName::Land {
        f += 1;
        assert!(f < 300, "never landed");
    }
    // Two frames into the landing (before its reaction window): jump.
    w.step(Pad { stick: [0.0, 1.0], ..IDLE });
    let o = w.step(Pad { jump: true, stick: [0.0, 1.0], ..IDLE });
    assert_eq!(o.state, StateName::Jump, "jumped out of the landing");
}

/// Horizontal Slash (row 38, ability 99) stands for one swing of an air combo: mashing Square
/// in the air takes it only while the combo has swings left before its finisher, so it can't
/// be spammed forever (it hops, so before Sora could stay up for good, the user).
#[test]
fn horizontal_slash_is_part_of_the_air_combo() {
    let mut ch = sora();
    ch.moveset.clips.insert(193, attack_clip(193, 30, (5, 10), 5, 15));
    let mut slash = row(38, 193, false, true);
    slash.kind = 0;
    slash.ability = 99;
    slash.jump = 14.0;
    ch.moveset.combo_rows.push(slash);
    ch.abilities.equipped.push(99);
    let mut w = World::new(ch);
    w.run(3, Pad { jump: true, ..IDLE });
    w.run(12, IDLE);
    // One air swing, then nothing but Square.
    let mut motions = Vec::new();
    for f in 0..400 {
        let o = w.step(Pad { attack: f < 2, action: f >= 2 && f % 2 == 0, ..IDLE });
        if o.state == StateName::Attack && o.clip_frame == 0 {
            motions.push(o.motion);
        }
    }
    let slashes = motions.iter().filter(|&&m| m == 193).count();
    assert!(slashes >= 1, "Horizontal Slash never came out: {motions:?}");
    let max = w.ch.combo_rules.air_base as usize;
    assert!(slashes <= max - 1, "{slashes} Horizontal Slashes in a combo of {max}: {motions:?}");
}

/// An air cast (`air_magic_falls`, as Final Form's air Reflect) falls as it plays (s206 reverts
/// s204's hold, the user: "we do want it to fall like kh1"), and ends on touching the ground in
/// KH2's landing, not standing in the rest of the cast (the user).
#[test]
fn falling_air_cast_lands_into_the_landing() {
    let mut ch = sora();
    let mut cast = Clip::plain(73, 200);
    cast.windows.push(Window { start: 0, end: Some(20), kind: WindowKind::NoGravity });
    ch.moveset.clips.insert(73, cast);
    ch.air_magic_falls = true;
    let mut w = World::new(ch);
    w.run(3, Pad { jump: true, ..IDLE });
    w.run(12, IDLE);
    assert!(w.pos[1] > 50.0);
    assert_eq!(w.ch.cast(CastMotions { ground: 71, finisher: Some(72), air: 73 }), Some(73));
    let y = w.pos[1];
    w.run(18, IDLE);
    assert!(w.pos[1] < y, "held in the cast's no-gravity frames: {} -> {}", y, w.pos[1]);
    let outs = w.run(150, IDLE);
    let landed = outs.iter().position(|o| o.state == StateName::Land).expect("never landed");
    assert!(outs[..landed].iter().all(|o| o.motion == 73), "left the cast before landing");
    assert!(w.pos[1] <= 0.0);
}

/// No spell during KH2's landing (the user: "reflect can also be used during the landing
/// animation, we need to disable this"): refused there, nothing kept for after, and allowed once
/// it is over.
#[test]
fn no_cast_during_the_landing() {
    // Jump, and stop on the landing's first frame.
    let mut w = World::new(sora());
    w.run(3, Pad { jump: true, ..IDLE });
    let mut on_land = false;
    for _ in 0..200 {
        let o = w.run(1, IDLE);
        if o[0].state == StateName::Land {
            on_land = true;
            break;
        }
    }
    assert!(on_land);
    assert!(!w.ch.can_cast() && !w.ch.magic_later(), "a spell in the landing");
    assert_eq!(w.ch.cast(CastMotions { ground: 71, finisher: Some(72), air: 73 }), None);
    let outs = w.run(60, IDLE);
    assert!(outs.iter().any(|o| o.state != StateName::Land));
    assert!(w.ch.can_cast(), "no spell after the landing");
}

/// An air cast with a share of gravity (KH1's slow sink, the user: "falls too fast"): the cast
/// stops the fall, he sinks slower than a plain fall from the same height, and the fall after the
/// cast goes on from the speed reached (no jump in speed).
#[test]
fn air_cast_sinks_at_its_share_of_gravity() {
    let setup = |share: f32| {
        let mut ch = sora();
        let mut cast = Clip::plain(73, 40);
        cast.windows.push(Window { start: 0, end: Some(38), kind: WindowKind::NoGravity });
        ch.moveset.clips.insert(73, cast);
        ch.air_magic_falls = true;
        ch.air_magic_gravity = share;
        let mut w = World::new(ch);
        w.run(3, Pad { jump: true, ..IDLE });
        w.run(30, IDLE);
        w
    };
    let mut full = setup(1.0);
    let mut slow = setup(0.3);
    let h0 = slow.pos[1];
    assert!((full.pos[1] - h0).abs() < 1e-3 && h0 > 50.0);
    assert!(full.ch.cast(CastMotions { ground: 71, finisher: Some(72), air: 73 }).is_some());
    assert!(slow.ch.cast(CastMotions { ground: 71, finisher: Some(72), air: 73 }).is_some());
    let a = full.run(8, IDLE);
    let b = slow.run(8, IDLE);
    assert!(a.iter().all(|o| o.motion == 73) && b.iter().all(|o| o.motion == 73));
    let (fell_full, fell_slow) = (h0 - full.pos[1], h0 - slow.pos[1]);
    assert!(fell_slow > 0.0, "didn't sink");
    assert!(fell_slow < fell_full * 0.5, "sank {fell_slow} against {fell_full}");
    // Through the end of the cast into the fall: no sudden jump in fall speed.
    let outs = slow.run(30, IDLE);
    let steps: Vec<f32> = outs.iter().map(|o| -o.displacement[1]).collect();
    for p in steps.windows(2) {
        assert!(p[1] - p[0] < 6.0, "fall speed jumped: {steps:?}");
    }
}

/// An air cast at full gravity (KH1's air spells, the user: Reflect "resets soras downward
/// velocity for one frame ... then after have it fall to the ground with soras normal gravity"):
/// falling fast, the cast stops his fall, then he falls from rest as a plain fall.
#[test]
fn air_cast_stops_the_fall_one_frame_then_falls() {
    let mut ch = sora();
    let mut cast = Clip::plain(73, 60);
    cast.windows.push(Window { start: 0, end: Some(50), kind: WindowKind::NoGravity });
    ch.moveset.clips.insert(73, cast);
    ch.air_magic_falls = true;
    ch.air_magic_gravity = 1.0;
    let mut w = World::new(ch);
    w.run(3, Pad { jump: true, ..IDLE });
    w.run(40, IDLE);
    let before = w.run(1, IDLE)[0].displacement[1];
    assert!(before < -1.0, "not falling yet: {before}");
    assert!(w.ch.cast(CastMotions { ground: 71, finisher: Some(72), air: 73 }).is_some());
    let outs = w.run(6, IDLE);
    assert!(outs.iter().all(|o| o.motion == 73));
    let d: Vec<f32> = outs.iter().map(|o| -o.displacement[1]).collect();
    // (The fall's arc from rest: its first step is a fraction of a unit.)
    assert!(d[0] >= 0.0 && d[0] < 1.0 && d[0] < before.abs() * 0.2, "not stopped: {d:?}");
    for p in d[1..].windows(2) {
        assert!(p[1] >= p[0], "full gravity speeds him up: {d:?}");
    }
    assert!(d[5] > d[1], "full gravity speeds him up: {d:?}");
}

/// Gliding in place is KH2's hover (motion 208): a glide started with the stick neutral hovers
/// in place, the stick takes it into the glide, and a glide let go hovers again once stopped.
#[test]
fn glide_in_place_hovers() {
    let mut w = World::new(sora());
    w.ch.abilities.aerial_dodge = 0;
    w.run(1, Pad { jump: true, ..IDLE });
    w.run(12, IDLE);
    let outs = w.run(10, Pad { jump: true, ..IDLE });
    assert!(outs.iter().all(|o| o.state == StateName::Glide && o.motion == motion::GLIDE_HOVER), "{:?}", outs.iter().map(|o| o.motion).collect::<Vec<_>>());
    assert!(outs.iter().all(|o| o.displacement[0].abs() + o.displacement[2].abs() < 1e-3), "hovering moved");
    let outs = w.run(20, Pad { jump: true, stick: [0.0, 1.0], ..IDLE });
    assert!(outs.iter().all(|o| o.motion != motion::GLIDE_HOVER));
    assert!(outs.last().unwrap().displacement[2].abs() > 5.0);
    // KH2's glide never goes back to the hover (Steam exe: only a neutral start hovers), and its
    // speed doesn't follow the stick: let go, he flies on at glide speed.
    let outs = w.run(80, Pad { jump: true, ..IDLE });
    assert!(outs.iter().all(|o| o.motion != motion::GLIDE_HOVER));
    assert!(outs.last().unwrap().displacement[2].abs() > 5.0, "slowed with the stick let go");
}

/// KH2's stored launch speed (Steam exe `+0xE0`): an Aerial Dodge into a neutral glide hovers,
/// and tilting the stick puts the dodge's speed back; letting go of the glide keeps the speed.
#[test]
fn hover_keeps_the_dodge_speed() {
    let first = |dodge: bool| {
        let mut w = World::new(sora());
        w.ch.glide_button = GlideButton::Action;
        w.run(1, Pad { jump: true, ..IDLE });
        w.run(10, IDLE);
        if dodge {
            let o = w.step(Pad { jump: true, stick: [0.0, 1.0], ..IDLE });
            assert_eq!(o.state, StateName::AerialDodge);
            w.run(3, Pad { stick: [0.0, 1.0], ..IDLE });
        }
        // Square with the stick neutral: the hover, slowing to a stop.
        let outs = w.run(60, Pad { action: true, ..IDLE });
        assert!(outs.iter().all(|o| o.motion == motion::GLIDE_HOVER), "{:?}", outs.iter().map(|o| o.motion).collect::<Vec<_>>());
        assert!(outs.last().unwrap().displacement[2].abs() < 0.5, "still drifting: {}", outs.last().unwrap().displacement[2]);
        // Tilted: the glide, from the stored speed.
        let o = w.run(2, Pad { action: true, stick: [0.0, 1.0], ..IDLE });
        let glide = o.last().unwrap().displacement[2].abs();
        // Let go of the glide: the fall keeps its speed.
        let o = w.step(Pad { stick: [0.0, 1.0], ..IDLE });
        assert_ne!(o.state, StateName::Glide);
        (glide, w.ch.state_name())
    };
    let (plain, _) = first(false);
    let (dodged, _) = first(true);
    assert!(dodged > plain + 1.0, "dodge's speed not kept: {dodged} vs {plain}");
}

/// Aerial Dodge only moves along the stick: neutral, it goes straight up (the user: KH2's
/// doesn't push Sora forward unless tilted).
#[test]
fn neutral_aerial_dodge_goes_straight_up() {
    let mut w = World::new(sora());
    w.run(1, Pad { jump: true, ..IDLE });
    w.run(10, IDLE);
    let x0 = (w.pos[0], w.pos[2]);
    let o = w.step(Pad { jump: true, ..IDLE });
    assert_eq!(o.state, StateName::AerialDodge);
    w.run(20, IDLE);
    assert!((w.pos[0] - x0.0).abs() + (w.pos[2] - x0.1).abs() < 1e-3, "moved {:?}", (w.pos[0] - x0.0, w.pos[2] - x0.1));
    let mut w = World::new(sora());
    w.run(1, Pad { jump: true, ..IDLE });
    w.run(10, IDLE);
    let z0 = w.pos[2];
    w.step(Pad { jump: true, stick: [0.0, 1.0], ..IDLE });
    w.run(10, Pad { stick: [0.0, 1.0], ..IDLE });
    assert!(w.pos[2] - z0 > 20.0);
}

/// A spell cast mid-combo inserts itself as a step of it (the user: as a modifier does): swing,
/// spell in the swing's combo window, an Attack pressed during the spell, and that swing is the
/// combo's finisher. Back in idle the combo is over (the user: "the intended behavior is for
/// sora to have his combo reset if he returns to idle"): a spell after the swing has ended
/// starts a new combo, and one played to its end leaves none to go on with.
#[test]
fn spell_mid_combo_is_a_step_of_it() {
    let mut ch = sora();
    // (A cast with a combo window, as Blizzard's: the Attack goes on with the combo there.)
    let mut c71 = Clip::plain(71, 30);
    c71.windows.push(Window { start: 20, end: None, kind: WindowKind::Combo });
    ch.moveset.clips.insert(71, c71);
    let mut w = World::new(ch);
    w.step(Pad { attack: true, ..IDLE });
    let mut cast = None;
    for _ in 0..45 {
        let o = w.step(IDLE);
        if !o.hitboxes.is_empty() {
            w.ch.register_hit();
        }
        if cast.is_none() && w.ch.can_cast() {
            cast = w.ch.cast(CastMotions { ground: 71, finisher: None, air: 73 });
        }
    }
    assert_eq!(cast, Some(71), "cast from the swing's window");
    // (Mashed: a press counts only once the cast's end lets it in.)
    let outs: Vec<_> = (0..40).map(|i| w.step(Pad { attack: i % 2 == 0, ..IDLE })).collect();
    let next: Vec<u16> = outs.iter().filter(|o| o.state == StateName::Attack && o.clip_frame == 0).map(|o| o.motion).collect();
    assert_eq!(next.first(), Some(&110), "the swing after the spell should be the finisher: {next:?}");

    // A spell played to its end on the ground: idle, the combo over.
    let mut ch = sora();
    ch.moveset.clips.insert(71, Clip::plain(71, 30));
    let mut w = World::new(ch);
    w.step(Pad { attack: true, ..IDLE });
    for _ in 0..12 {
        let o = w.step(IDLE);
        if !o.hitboxes.is_empty() {
            w.ch.register_hit();
        }
    }
    for _ in 0..30 {
        if w.ch.can_cast() {
            w.ch.cast(CastMotions { ground: 71, finisher: None, air: 73 });
            break;
        }
        w.step(IDLE);
    }
    w.run(60, IDLE);
    assert_eq!(w.step(Pad { attack: true, ..IDLE }).motion, 100, "a new combo");

    // A spell after the swing has ended (idle): a new combo's first step.
    let mut ch = sora();
    ch.moveset.clips.insert(71, Clip::plain(71, 30));
    let mut w = World::new(ch);
    w.step(Pad { attack: true, ..IDLE });
    w.run(60, IDLE);
    w.ch.cast(CastMotions { ground: 71, finisher: None, air: 73 });
    w.run(40, IDLE);
    assert_eq!(w.step(Pad { attack: true, ..IDLE }).motion, 100);
}

/// A spell chosen during the combo's finisher (no next finisher) can't come in that move, nor
/// later: `magic_later` says so, and the host drops the choice (the user: "regardless of if a
/// spell is inputted during a finisher it should not be allowed to execute").
#[test]
fn no_magic_buffered_through_a_finisher() {
    let mut w = World::new(sora());
    let (motions, _) = mash(&mut w, true);
    assert!(motions.contains(&110));
    let mut w = World::new(sora());
    let mut seen = false;
    for f in 0..200 {
        let o = w.step(Pad { attack: f % 2 == 0, ..IDLE });
        if !o.hitboxes.is_empty() {
            w.ch.register_hit();
        }
        if o.state == StateName::Attack && o.motion == 110 {
            seen = true;
            assert!(!w.ch.magic_later(), "frame {} of the finisher", o.clip_frame);
            break;
        }
    }
    assert!(seen);
    // In a plain swing before its window: later in the move, yes.
    let mut w = World::new(sora());
    w.step(Pad { attack: true, ..IDLE });
    w.step(IDLE);
    assert!(!w.ch.can_cast() && w.ch.magic_later());
}

/// A spell chosen mid-swing is cast as soon as the swing would take its next swing (its combo
/// window), not only once it has ended (the user: "too much a pause before magic gets cast").
#[test]
fn spell_cast_from_the_swing_combo_window() {
    let mut ch = sora();
    ch.moveset.clips.insert(71, Clip::plain(71, 30));
    let mut w = World::new(ch);
    w.step(Pad { attack: true, ..IDLE });
    let mut opened = None;
    for f in 1..40 {
        w.step(IDLE);
        if w.ch.can_cast() {
            opened = Some(f);
            break;
        }
    }
    // The test swing (motion 100, 40 frames) opens its combo window at frame 20.
    assert!(opened.is_some_and(|f| f <= 22), "could cast only at {opened:?}");
}

// ---- Magic in the combo string (spec: magic combo logic) ----

/// The test moveset with cast motions (71 ground, 72 the finisher cast, 73 air), a finisher
/// swing with a finisher window, Combo Master (every swing counts) and `fp` Finishing Plus.
fn magic_sora(fp: u8) -> Character {
    let mut ch = sora();
    let mut fin = attack_clip(110, 50, (12, 20), 3, 999);
    fin.windows.retain(|w| !matches!(w.kind, WindowKind::Combo));
    fin.windows.push(Window { start: 30, end: Some(45), kind: WindowKind::Finisher });
    ch.moveset.clips.insert(110, fin);
    let mut c71 = Clip::plain(71, 40);
    c71.windows.push(Window { start: 20, end: None, kind: WindowKind::AllowReactionCommand });
    // (An Attack follows a cast only in its combo window, range id 11, as Blizzard's 59 from 36:
    // `attack_out_of_a_cast_only_in_its_combo_window`.)
    c71.windows.push(Window { start: 20, end: None, kind: WindowKind::Combo });
    c71.windows.push(Window { start: 25, end: Some(35), kind: WindowKind::ComboMagic });
    ch.moveset.clips.insert(71, c71);
    let mut c72 = Clip::plain(72, 50);
    c72.windows.push(Window { start: 20, end: None, kind: WindowKind::AllowReactionCommand });
    c72.windows.push(Window { start: 30, end: Some(45), kind: WindowKind::Finisher });
    ch.moveset.clips.insert(72, c72);
    ch.moveset.clips.insert(73, Clip::plain(73, 40));
    ch.abilities.combo_master = true;
    ch.abilities.finishing_plus = fp;
    ch
}

const CAST: CastMotions = CastMotions { ground: 71, finisher: Some(72), air: 73 };

/// Steps until `pred` holds (at most `n` frames), pressing Attack every other frame if `mash`.
fn until(w: &mut World, n: usize, mash: bool, pred: impl Fn(&Output, &Character) -> bool) -> Option<Output> {
    for f in 0..n {
        let o = w.step(Pad { attack: mash && f % 2 == 0, ..IDLE });
        if pred(&o, &w.ch) {
            return Some(o);
        }
    }
    None
}

/// Swing, then the next swing in its combo window, until `swings` swings have started; returns
/// in the last one's combo / finisher window (`at` frames in).
fn swings(w: &mut World, swings: usize, at: u32) -> Output {
    let mut started = 0;
    let mut last = None;
    for f in 0..400 {
        let o = w.step(Pad { attack: started < swings && f % 2 == 0, ..IDLE });
        if o.state == StateName::Attack && o.clip_frame == 0 {
            started += 1;
        }
        if started == swings && o.state == StateName::Attack && o.clip_frame >= at {
            return o;
        }
        last = Some(o);
    }
    panic!("never got {swings} swings: last {:?}", last.map(|o| (o.state, o.motion)));
}

#[test]
fn reflect_cannot_follow_the_last_finisher() {
    // No Finishing Plus: swing, swing, finisher; in the finisher's window no spell.
    let mut w = World::new(magic_sora(0));
    let o = swings(&mut w, 3, 32);
    assert_eq!(o.motion, 110);
    assert!(!w.ch.can_cast(), "a spell after the combo's last finisher");
    assert_eq!(w.ch.cast(CAST), None);
    // Its animation over, a new combo may start, with a spell too (its first step).
    until(&mut w, 60, false, |o, _| o.state != StateName::Attack).unwrap();
    assert_eq!(w.ch.cast(CAST), Some(71));
}

#[test]
fn finishing_reflect_as_the_first_finisher_then_a_finisher() {
    // Case A: swing, swing, finishing Reflect, finisher.
    let mut w = World::new(magic_sora(1));
    swings(&mut w, 2, 21);
    assert_eq!(w.ch.cast(CAST), Some(72), "the combo's last step is the finishing cast");
    // Attack pressed early: nothing until its finisher window (frame 30).
    for _ in 0..14 {
        let o = w.step(Pad { attack: true, ..IDLE });
        assert_eq!(o.motion, 72);
        w.step(IDLE);
    }
    let o = until(&mut w, 30, true, |o, _| o.state == StateName::Attack).expect("no second finisher");
    assert_eq!(o.motion, 110, "the swing after a finishing Reflect is the next finisher");
    // That was the last finisher: no spell, no swing until it has ended.
    let o = until(&mut w, 40, true, |o, _| o.clip_frame >= 32).unwrap();
    assert_eq!(o.motion, 110);
    assert!(!w.ch.can_cast());
    let o = until(&mut w, 30, true, |o, _| o.clip_frame == 0 || o.state != StateName::Attack).unwrap();
    assert!(!(o.state == StateName::Attack && o.motion == 110 && o.clip_frame == 0), "a third finisher");
}

#[test]
fn finishing_reflect_twice_then_the_combo_ends_after_it() {
    // Case B / C: swing, swing, finishing Reflect, finishing Reflect; locked to its end.
    let mut w = World::new(magic_sora(1));
    swings(&mut w, 2, 21);
    assert_eq!(w.ch.cast(CAST), Some(72));
    until(&mut w, 40, false, |o, _| o.clip_frame >= 31).unwrap();
    assert_eq!(w.ch.cast(CAST), Some(72), "the second finisher, a finishing Reflect");
    // Nothing interrupts it: Attack, a spell, jump, Square.
    for f in 1..49 {
        let o = w.step(Pad { attack: f % 2 == 0, jump: f % 7 == 0, action: f % 5 == 0, ..IDLE });
        assert_eq!((o.state, o.motion), (StateName::Magic, 72), "frame {f}");
        assert!(!w.ch.can_cast(), "frame {f}");
    }
    // Ended: a new combo from its first swing.
    let o = until(&mut w, 20, true, |o, _| o.state == StateName::Attack).unwrap();
    assert_eq!(o.motion, 100);
}

#[test]
fn finishing_reflect_as_the_second_finisher() {
    // Case D: swing, swing, finisher, finishing Reflect.
    let mut w = World::new(magic_sora(1));
    let o = swings(&mut w, 3, 31);
    assert_eq!(o.motion, 110);
    assert_eq!(w.ch.cast(CAST), Some(72), "Finishing Plus: a finishing Reflect after the first finisher");
    for f in 1..49 {
        let o = w.step(Pad { attack: f % 2 == 0, ..IDLE });
        assert_eq!(o.motion, 72, "frame {f}");
    }
    assert_eq!(w.ch.cast(CAST), None, "still in it");
}

#[test]
fn reflect_mid_combo_is_one_step() {
    // Swing, Reflect, finisher (the combo of 3): magic takes one slot, none skipped.
    let mut w = World::new(magic_sora(0));
    swings(&mut w, 1, 21);
    assert_eq!(w.ch.cast(CAST), Some(71));
    let o = until(&mut w, 60, true, |o, _| o.state == StateName::Attack).unwrap();
    assert_eq!(o.motion, 110);
    // Reflect first: Reflect, swing, finisher.
    let mut w = World::new(magic_sora(0));
    assert_eq!(w.ch.cast(CAST), Some(71));
    let o = until(&mut w, 60, true, |o, _| o.state == StateName::Attack).unwrap();
    assert_eq!(o.motion, 100);
    let o = until(&mut w, 60, true, |o, _| o.state == StateName::Attack && o.clip_frame == 0 && o.motion != 100).unwrap();
    assert_eq!(o.motion, 110);
    // Reflect into Reflect (its combo-magic window), then the finisher.
    let mut w = World::new(magic_sora(0));
    assert_eq!(w.ch.cast(CAST), Some(71));
    until(&mut w, 40, false, |o, _| o.clip_frame >= 26).unwrap();
    assert_eq!(w.ch.cast(CAST), Some(71), "second spell, second step");
    let o = until(&mut w, 60, true, |o, _| o.state == StateName::Attack).unwrap();
    assert_eq!(o.motion, 110);
}

/// An Attack follows a cast only in the cast's combo window (range id 11), as Steam KH2's Attack
/// check `0x1403D6CA0` (combo window, finisher window or a free state; the reaction-command
/// window, range id 14, is for reaction commands): without one (Fire, Thunder, Cure, Reflect,
/// the air casts) mashing does nothing until the cast has ended, and on the ground the swing
/// after it starts a new combo (the free state); with one (Blizzard's from 36) the Attack there
/// goes on with the combo.
#[test]
fn attack_out_of_a_cast_only_in_its_combo_window() {
    for combo_window in [false, true] {
        let mut ch = sora();
        let mut c71 = Clip::plain(71, 40);
        c71.windows.push(Window { start: 20, end: None, kind: WindowKind::AllowReactionCommand });
        if combo_window {
            c71.windows.push(Window { start: 30, end: None, kind: WindowKind::Combo });
        }
        ch.moveset.clips.insert(71, c71);
        ch.abilities.combo_master = true;
        let mut w = World::new(ch);
        w.step(Pad { attack: true, ..IDLE });
        let mut cast = None;
        for _ in 0..45 {
            w.step(IDLE);
            if cast.is_none() && w.ch.can_cast() {
                cast = w.ch.cast(CastMotions { ground: 71, finisher: None, air: 73 });
                break;
            }
        }
        assert_eq!(cast, Some(71));
        let mut first = None;
        for i in 0..60 {
            let o = w.step(Pad { attack: i % 2 == 0, ..IDLE });
            if o.state == StateName::Attack && o.clip_frame == 0 {
                first = Some((i, o.motion));
                break;
            }
        }
        let (at, motion) = first.expect("a swing after the cast");
        if combo_window {
            assert!((29..32).contains(&at), "taken in the combo window: {at}");
            assert_ne!(motion, 100, "the combo goes on");
        } else {
            assert!(at >= 39, "not before the cast's end: {at}");
            assert_eq!(motion, 100, "a new combo from the free state");
        }
    }
}

/// KH1's spells (`magic_step` at the hand-off, `combo_state` / `set_combo_state` around it).
fn kh1_spell(w: &mut World) -> Option<SpellSlot> {
    let slot = w.ch.magic_step();
    let c = w.ch.combo_state();
    // The host's own cast plays; KH2's controller starts over after it, the combo carried.
    let mut fresh = magic_sora(w.ch.abilities.finishing_plus);
    if slot.is_some_and(|s| s != SpellSlot::Finisher { more: false }) {
        fresh.set_combo_state(c);
    }
    w.ch = fresh;
    slot
}

#[test]
fn kh1_spells_take_one_slot_each() {
    // Magic, swing, finisher.
    let mut w = World::new(magic_sora(0));
    assert_eq!(kh1_spell(&mut w), Some(SpellSlot::Step));
    let o = until(&mut w, 10, true, |o, _| o.state == StateName::Attack).unwrap();
    assert_eq!(o.motion, 100);
    let o = until(&mut w, 60, true, |o, _| o.state == StateName::Attack && o.clip_frame == 0 && o.motion != 100).unwrap();
    assert_eq!(o.motion, 110, "magic, swing, then the finisher: none skipped");
    // Swing, magic, finisher.
    let mut w = World::new(magic_sora(0));
    swings(&mut w, 1, 21);
    assert_eq!(kh1_spell(&mut w), Some(SpellSlot::Step));
    let o = until(&mut w, 10, true, |o, _| o.state == StateName::Attack).unwrap();
    assert_eq!(o.motion, 110);
    // Magic, magic, finisher.
    let mut w = World::new(magic_sora(0));
    assert_eq!(kh1_spell(&mut w), Some(SpellSlot::Step));
    assert_eq!(kh1_spell(&mut w), Some(SpellSlot::Step));
    let o = until(&mut w, 10, true, |o, _| o.state == StateName::Attack).unwrap();
    assert_eq!(o.motion, 110);
    // Swing, magic, magic on the last slot: the combo ends with it.
    let mut w = World::new(magic_sora(0));
    swings(&mut w, 1, 21);
    assert_eq!(kh1_spell(&mut w), Some(SpellSlot::Step));
    assert_eq!(kh1_spell(&mut w), Some(SpellSlot::Finisher { more: false }));
    let o = until(&mut w, 10, true, |o, _| o.state == StateName::Attack).unwrap();
    assert_eq!(o.motion, 100, "a new combo after the last step's spell");
    // With Finishing Plus the spell on the last slot is the first finisher: a finisher next.
    let mut w = World::new(magic_sora(1));
    swings(&mut w, 2, 21);
    assert_eq!(kh1_spell(&mut w), Some(SpellSlot::Finisher { more: true }));
    let o = until(&mut w, 10, true, |o, _| o.state == StateName::Attack).unwrap();
    assert_eq!(o.motion, 110);
    // ... and nothing after that one's window.
    until(&mut w, 40, false, |o, _| o.clip_frame >= 32).unwrap();
    assert!(!w.ch.can_cast());
}

#[test]
fn air_attack_lands_into_the_landing() {
    let mut w = World::new(magic_sora(0));
    w.run(3, Pad { jump: true, ..IDLE });
    w.run(10, IDLE);
    let o = until(&mut w, 20, true, |o, _| o.state == StateName::Attack).unwrap();
    assert_eq!(o.motion, 120);
    // The ground comes up mid-swing (the log: "Attack 181 at 85 above the floor -> Run").
    w.step(IDLE);
    w.pos[1] = 0.5;
    let outs: Vec<_> = (0..200).map(|_| w.step(IDLE)).collect();
    let i = outs.iter().position(|o| o.state != StateName::Attack && o.state != StateName::Air).unwrap();
    assert_eq!(outs[i].state, StateName::Land, "after an air attack: {:?}", outs[i].state);
}

#[test]
fn aerial_dodge_after_rolling_off_a_ledge() {
    let mut w = World::new(magic_sora(0));
    w.step(Pad { action: true, stick: [0.0, 1.0], ..IDLE });
    w.run(5, Pad { stick: [0.0, 1.0], ..IDLE });
    // The floor drops away under the roll; the roll plays out into the fall.
    w.pos[1] = 5000.0;
    let o = until(&mut w, 80, false, |o, _| o.state == StateName::Air).expect("no fall after the roll");
    assert!(w.ch.airborne(), "{:?}", o.state);
    let o = w.step(Pad { jump: true, ..IDLE });
    assert_eq!(o.state, StateName::AerialDodge);
}


#[test]
fn no_finishing_reflect_after_a_missed_finisher() {
    // No Combo Master: the finisher whiffs, so neither a swing nor a spell follows it.
    let mut ch = magic_sora(1);
    ch.abilities.combo_master = false;
    let mut w = World::new(ch);
    // Two connecting swings, then the finisher, which misses.
    let mut started = 0;
    for f in 0..400 {
        let o = w.step(Pad { attack: started < 3 && f % 2 == 0, ..IDLE });
        if o.state == StateName::Attack && o.clip_frame == 0 {
            started += 1;
        }
        if !o.hitboxes.is_empty() && o.motion != 110 {
            w.ch.register_hit();
        }
        if started == 3 && o.motion == 110 && o.clip_frame >= 32 {
            break;
        }
    }
    assert_eq!(w.ch.state_name(), StateName::Attack);
    assert!(!w.ch.can_cast(), "a spell after a missed finisher");
    assert_eq!(w.ch.cast(CAST), None);
}

#[test]
fn air_spell_combo_ends_on_landing() {
    // Air combo, an air spell on its last step (Finishing Plus leaving another), falling to the
    // ground (Reflect's air fall): on the ground the next swing starts a new combo.
    let mut ch = magic_sora(1);
    ch.air_magic_falls = true;
    let mut w = World::new(ch);
    w.run(3, Pad { jump: true, ..IDLE });
    w.run(10, IDLE);
    let mut started = 0;
    for f in 0..200 {
        let o = w.step(Pad { attack: started < 2 && f % 2 == 0, ..IDLE });
        if o.state == StateName::Attack && o.clip_frame == 0 {
            started += 1;
        }
        if started == 2 && o.state == StateName::Attack && o.clip_frame >= 16 {
            break;
        }
    }
    assert_eq!(w.ch.cast(CAST), Some(73));
    until(&mut w, 200, false, |o, _| o.state == StateName::Land).expect("never landed");
    w.run(30, IDLE);
    let o = until(&mut w, 20, true, |o, _| o.state == StateName::Attack).unwrap();
    assert_eq!(o.motion, 100, "the air combo's finisher carried onto the ground");
}

/// Wisdom Form's ground inertia (prty acceleration 0.85, brake 0.97): he slides to a stop, in
/// his idle, after letting go of the stick (the user: "KH2 sora slides when he stops in wisdom
/// form").
#[test]
fn ground_inertia_slides_to_a_stop() {
    let mut ch = sora();
    ch.params.acceleration = 0.85;
    ch.params.brake = 0.97;
    let mut w = World::new(ch);
    w.run(60, Pad { stick: [0.0, 1.0], ..IDLE });
    let z0 = w.pos[2];
    let outs = w.run(30, IDLE);
    assert!(w.pos[2] - z0 > 40.0, "slid {}", w.pos[2] - z0);
    assert!(outs.iter().all(|o| o.state == StateName::Idle), "idle while sliding");
    w.run(400, IDLE);
    let z1 = w.pos[2];
    w.run(10, IDLE);
    assert_eq!(w.pos[2], z1, "stopped");
    // Without the numbers (Sora): stops dead.
    let mut w = World::new(sora());
    w.run(60, Pad { stick: [0.0, 1.0], ..IDLE });
    let z0 = w.pos[2];
    w.run(5, IDLE);
    assert!(w.pos[2] - z0 < 8.5);
}

/// An attack motion's allow-movement range lets him move with the stick (Wisdom's shots; the
/// user: "sora is able to move/slide mid combo in KH2"); outside one he stands.
#[test]
fn attack_moves_in_its_allow_movement_range() {
    let mut ch = sora();
    let mut clip = attack_clip(100, 40, (10, 16), 1, 20);
    clip.windows.push(Window { start: 0, end: Some(30), kind: WindowKind::AllowMovement });
    ch.moveset.clips.insert(100, clip);
    let mut w = World::new(ch);
    w.step(Pad { attack: true, ..IDLE });
    let x0 = w.pos[0];
    let outs = w.run(12, Pad { stick: [1.0, 0.0], ..IDLE });
    assert!(outs.iter().all(|o| o.state == StateName::Attack));
    assert!(w.pos[0] - x0 > 50.0, "moved {}", w.pos[0] - x0);
    // Sora's own swing (no range): he stays put.
    let mut w = World::new(sora());
    w.step(Pad { attack: true, ..IDLE });
    let x0 = w.pos[0];
    let outs = w.run(12, Pad { stick: [1.0, 0.0], ..IDLE });
    assert!(outs.iter().all(|o| o.state == StateName::Attack));
    assert!((w.pos[0] - x0).abs() < 1e-3);
}

/// Wisdom Form's air shots (an allow-movement range over the whole motion, a no-gravity range
/// in it): he steers with the stick and falls as he shoots (the user: "wisdom forms aerial
/// combat allows you to move in the air while the attack plays, and sora also has gravity").
#[test]
fn moving_air_attack_steers_and_falls() {
    let mut ch = sora();
    let mut clip = attack_clip(120, 30, (5, 10), 4, 15);
    clip.windows.push(Window { start: 0, end: None, kind: WindowKind::AllowMovement });
    clip.windows.push(Window { start: 0, end: Some(25), kind: WindowKind::NoGravity });
    ch.moveset.clips.insert(120, clip);
    let mut w = World::new(ch);
    w.step(Pad { jump: true, ..IDLE });
    w.run(14, Pad { jump: true, ..IDLE });
    let h0 = w.pos[1];
    assert!(h0 > 30.0, "up {h0}");
    let x0 = w.pos[0];
    w.step(Pad { attack: true, ..IDLE });
    let outs = w.run(10, Pad { stick: [1.0, 0.0], ..IDLE });
    assert!(outs.iter().all(|o| o.state == StateName::Attack), "{:?}", outs.iter().map(|o| o.state).collect::<Vec<_>>());
    assert!(w.pos[0] - x0 > 10.0, "steered {}", w.pos[0] - x0);
    // Falling: his height goes down over the shots (no hover).
    let h1 = w.pos[1];
    let more = w.run(10, Pad { stick: [1.0, 0.0], ..IDLE });
    assert!(w.pos[1] < h1 || more.iter().any(|o| o.state != StateName::Attack), "fell from {h1} to {}", w.pos[1]);
}

/// The combo keeps one direction (the user: "in kh2 sora will actually stay locked in one
/// direction during a combo until ... he ends his combo ... [or] if the orange or blue reticle
/// is on, his next attack will snap him towards the direction of the enemy ... not mid
/// attack"): the stick aims only the combo's first swing; with a target, each swing starts
/// facing it and doesn't turn during the swing.
#[test]
fn combo_keeps_its_direction() {
    let mut ch = sora();
    ch.set_combo_direction_lock(true);
    ch.set_face_lock(true);
    let mut w = World::new(ch);
    // First swing toward +x.
    let o = w.step(Pad { attack: true, stick: [1.0, 0.0], ..IDLE });
    assert_eq!(o.state, StateName::Attack);
    let first = o.facing;
    assert!((first - std::f32::consts::FRAC_PI_2).abs() < 1e-3, "{first}");
    // The rest of the combo with the stick held the other way: same direction.
    let mut facings = Vec::new();
    for f in 0..60 {
        let o = w.step(Pad { attack: f % 2 == 0, stick: [-1.0, 0.0], ..IDLE });
        if !o.hitboxes.is_empty() {
            w.ch.register_hit();
        }
        if o.state == StateName::Attack {
            facings.push(o.facing);
        }
    }
    assert!(facings.len() > 30);
    assert!(facings.iter().all(|f| (f - first).abs() < 1e-3), "{facings:?}");
    // Back in idle: the next combo turns to the stick again.
    w.run(80, IDLE);
    let o = w.step(Pad { attack: true, stick: [-1.0, 0.0], ..IDLE });
    assert!((o.facing + std::f32::consts::FRAC_PI_2).abs() < 1e-3, "{}", o.facing);

    // Locked on a target behind him: the next swing starts facing it; it doesn't turn mid-swing.
    let mut ch = sora();
    ch.set_combo_direction_lock(true);
    ch.set_face_lock(true);
    let mut w = World::new(ch);
    w.targets.push(TargetInfo { id: 7, pos: [0.0, 0.0, -100.0], radius: 30.0, height: 100.0 });
    w.ch.set_lock(Some(7));
    let o = w.step(Pad { attack: true, ..IDLE });
    assert!((o.facing.abs() - std::f32::consts::PI).abs() < 1e-3, "{}", o.facing);

    // With a reticle on a target, the swing's own turn window turns him toward it mid-swing
    // (the user: only then); with none, the same window doesn't.
    let turn_swing = |locked: bool| {
        let mut ch = sora();
        let mut clip = attack_clip(100, 40, (10, 16), 1, 20);
        clip.windows.push(Window { start: 0, end: Some(20), kind: WindowKind::TurnToLock(0.2) });
        ch.moveset.clips.insert(100, clip);
        ch.set_combo_direction_lock(true);
        ch.set_face_lock(true);
        ch.set_auto_target(false);
        let mut w = World::new(ch);
        w.step(Pad { attack: true, ..IDLE });
        // A target appears off to his side after the swing has started.
        w.targets.push(TargetInfo { id: 9, pos: [200.0, 0.0, 0.0], radius: 30.0, height: 100.0 });
        if locked {
            w.ch.set_lock(Some(9));
        }
        let outs = w.run(10, IDLE);
        outs.last().unwrap().facing
    };
    assert!(turn_swing(true) > 0.5, "turned toward the reticle");
    assert!(turn_swing(false).abs() < 1e-3, "no reticle, no turn");
}

/// A finisher motion with no finisher range (Wisdom Form's finisher shot, its finishing casts):
/// with Finishing Plus the next finisher, a swing or a spell, follows from its last frames (the
/// user: Wisdom's finishing magic couldn't take the second finisher slot).
#[test]
fn second_finisher_after_a_finisher_without_a_window() {
    let run = |spell: bool| {
        let mut ch = sora();
        ch.abilities.finishing_plus = 1;
        ch.moveset.clips.insert(71, Clip::plain(71, 30));
        let mut w = World::new(ch);
        let mut seq = Vec::new();
        let mut cast = false;
        for f in 0..160 {
            let finishers = seq.iter().filter(|m| **m == 110).count();
            let o = w.step(Pad { attack: !spell || finishers == 0, ..IDLE }.with_attack(f));
            if !o.hitboxes.is_empty() {
                w.ch.register_hit();
            }
            if (o.state == StateName::Attack || o.state == StateName::Magic) && o.clip_frame == 0 {
                seq.push(o.motion);
            }
            if spell && finishers == 1 && !cast && w.ch.can_cast() && w.ch.in_finisher() {
                cast = w.ch.cast(CastMotions { ground: 71, finisher: Some(71), air: 73 }).is_some();
            }
        }
        (seq, cast)
    };
    let (seq, _) = run(false);
    assert!(seq.windows(2).any(|p| p == [110, 110]), "two finisher swings in a row: {seq:?}");
    let (seq, cast) = run(true);
    assert!(cast && seq.windows(2).any(|p| p == [110, 71]), "a spell as the second finisher: {seq:?}");
}

/// Wisdom Form in the air and landing (the user: "sora should not loose his velcoity when doing
/// an air move in wisdom form", "his landing animation should preserve his momentum"): a moving
/// air attack keeps the speed he jumped with, and with ground inertia the landing slides on.
#[test]
fn wisdom_keeps_momentum_in_the_air_and_on_landing() {
    let mut ch = sora();
    ch.params.acceleration = 0.85;
    ch.params.brake = 0.97;
    let mut clip = attack_clip(120, 30, (5, 10), 4, 15);
    clip.windows.push(Window { start: 0, end: None, kind: WindowKind::AllowMovement });
    ch.moveset.clips.insert(120, clip);
    let mut w = World::new(ch);
    // Run, jump forward.
    w.run(40, Pad { stick: [0.0, 1.0], ..IDLE });
    w.step(Pad { jump: true, stick: [0.0, 1.0], ..IDLE });
    w.run(8, Pad { jump: true, stick: [0.0, 1.0], ..IDLE });
    // Air attack with the stick let go: still travelling forward.
    let z0 = w.pos[2];
    let o = w.step(Pad { attack: true, ..IDLE });
    assert_eq!(o.state, StateName::Attack);
    w.run(4, IDLE);
    assert!(w.pos[2] - z0 > 15.0, "kept going {}", w.pos[2] - z0);
    // Down and landing: he slides on through the landing.
    let mut landed_at = None;
    for _ in 0..120 {
        let o = w.step(IDLE);
        if o.state == StateName::Land && landed_at.is_none() {
            landed_at = Some(w.pos[2]);
        }
    }
    if let Some(z) = landed_at {
        assert!(w.pos[2] - z > 5.0, "slid {}", w.pos[2] - z);
    }
}

/// Guard turns him toward the target with a reticle on it (the lock).
#[test]
fn guard_faces_the_locked_target() {
    let mut ch = sora();
    ch.moveset.clips.insert(motion::GUARD, Clip::plain(motion::GUARD, 30));
    let mut w = World::new(ch);
    w.targets.push(TargetInfo { id: 5, pos: [-150.0, 0.0, 0.0], radius: 30.0, height: 100.0 });
    w.ch.set_lock(Some(5));
    let o = w.step(Pad { action: true, ..IDLE });
    assert_eq!(o.state, StateName::Guard);
    assert!((o.facing + std::f32::consts::FRAC_PI_2).abs() < 1e-3, "{}", o.facing);
}

/// No attack buffer (the user: "sora can still queue an attack even if hes not on the available
/// frame ... We don't want a buffer type system"): an Attack pressed before the swing's combo
/// window is dropped, and one inside it starts the next swing.
#[test]
fn an_early_attack_press_is_not_kept() {
    let mut w = World::new(sora());
    w.step(Pad { attack: true, ..IDLE });
    w.step(IDLE);
    w.step(Pad { attack: true, ..IDLE });
    let outs = w.run(80, IDLE);
    let swings = outs.iter().filter(|o| o.state == StateName::Attack && o.clip_frame == 0).count();
    assert_eq!(swings, 0, "an Attack pressed before the window came out later");
    let mut w = World::new(sora());
    w.step(Pad { attack: true, ..IDLE });
    let mut next = false;
    for i in 0..80 {
        let open = w.ch.can_take_command();
        let o = w.step(Pad { attack: open && i % 2 == 0, ..IDLE });
        if i > 2 && o.state == StateName::Attack && o.clip_frame == 0 {
            next = true;
            break;
        }
    }
    assert!(next, "a press inside the window started no swing");
}

/// A Drive in the air holds Sora where he is (`play_held`; the user: "sora inputs a drive in
/// the air and should stay frozen in that position"), and a form swap carries his movement
/// (`carry_movement`: a revert on the ground doesn't halt him, one in the air falls on).
#[test]
fn drive_in_the_air_holds_and_swaps_carry_movement() {
    let mut ch = sora();
    ch.moveset.clips.insert(52, Clip::plain(52, 160));
    let mut w = World::new(ch);
    w.run(1, Pad { jump: true, ..IDLE });
    w.run(12, IDLE);
    assert!(w.ch.can_change_form(), "free in the air");
    assert!(w.ch.play_held(52, true));
    let y = w.pos[1];
    w.run(40, IDLE);
    assert!((w.pos[1] - y).abs() < 1e-3, "moved during an air Drive: {} -> {}", y, w.pos[1]);
    assert!(!w.ch.can_change_form(), "a Drive playing takes no other change");
    let old = w.ch.clone();
    let mut c = sora();
    c.carry_movement(&old);
    w.ch = c;
    let outs = w.run(120, IDLE);
    assert!(outs.iter().any(|o| o.state == StateName::Land) || w.pos[1] <= 0.0, "didn't fall after the swap");

    let mut w = World::new(sora());
    w.run(30, Pad { stick: [0.0, 1.0], ..IDLE });
    let speed = w.run(1, Pad { stick: [0.0, 1.0], ..IDLE })[0].displacement[2];
    let old = w.ch.clone();
    let mut c = sora();
    c.carry_movement(&old);
    w.ch = c;
    let o = w.step(Pad { stick: [0.0, 1.0], ..IDLE });
    assert!(o.displacement[2] > speed * 0.5, "the swap halted him: {} then {}", speed, o.displacement[2]);
}

/// An air cast held as Final Form's air Reflect (`hold_air_cast`; the user: base and Wisdom air
/// Reflect "copy the physics of final forms reflect"): stopped where he is, no fall through the
/// hold even with the cast's own window later, then a plain fall.
#[test]
fn air_reflect_holds_as_final_forms() {
    let mut ch = sora();
    let mut cast = Clip::plain(73, 84);
    cast.windows.push(Window { start: 16, end: Some(74), kind: WindowKind::NoGravity });
    ch.moveset.clips.insert(73, cast);
    ch.air_magic_falls = true;
    ch.air_magic_gravity = 0.3;
    let mut w = World::new(ch);
    w.run(3, Pad { jump: true, stick: [0.0, 1.0], ..IDLE });
    w.run(12, Pad { stick: [0.0, 1.0], ..IDLE });
    assert_eq!(w.ch.cast(CastMotions { ground: 71, finisher: Some(72), air: 73 }), Some(73));
    w.ch.hold_air_cast(54);
    let (y, z) = (w.pos[1], w.pos[2]);
    w.run(50, Pad { stick: [0.0, 1.0], ..IDLE });
    assert!((w.pos[1] - y).abs() < 1e-3, "moved up/down in the hold: {} -> {}", y, w.pos[1]);
    assert!((w.pos[2] - z).abs() < 1e-3, "drifted in the hold");
    w.run(20, IDLE);
    assert!(w.pos[1] < y - 1.0, "didn't fall after the hold");
}

/// A moveset whose guard row is an attack (Valor Form's rising slash, jump 90): Square starts it,
/// standing or walking, and it launches him (the KH1 port's user).
#[test]
fn guard_row_attack_launches_from_square() {
    for stick in [[0.0, 0.0], [0.0, 0.4]] {
        let mut ch = sora();
        let mut clip = Clip::plain(181, 48);
        clip.windows.push(Window { start: 5, end: Some(15), kind: WindowKind::Hitbox { attack: 228, group: 2, combo: None, weapon: 1 } });
        ch.moveset.clips.insert(181, clip);
        ch.moveset.combo_rows.retain(|r| r.id != 11);
        ch.moveset.combo_rows.push(ComboRow { kind: 0, jump: 90.0, ..row(11, 181, false, false) });
        let mut w = World::new(ch);
        w.run(5, Pad { stick, ..IDLE });
        let o = w.step(Pad { action: true, stick, ..IDLE });
        assert_eq!(o.state, StateName::Attack, "stick {stick:?}");
        assert_eq!(o.motion, 181);
        w.run(20, IDLE);
        assert!(w.pos[1] > 30.0, "didn't launch: y {}", w.pos[1]);
    }
}

/// A wall met by a swing in a moveset without a repelled motion (Final Form's): pushed back, the
/// swing goes on to its end (it went back to its frame 0, met the wall again, for ever: the KH1
/// port's user, "it soft locks you").
#[test]
fn wall_hit_without_a_repelled_motion_doesnt_loop() {
    let mut ch = sora();
    ch.moveset.clips.remove(&41);
    ch.moveset.clips.remove(&43);
    let mut w = World::new(ch);
    let first = w.run(1, Pad { attack: true, ..IDLE })[0].clone();
    assert_eq!(first.state, StateName::Attack);
    let mut frames = Vec::new();
    for _ in 0..200 {
        w.ch.register_wall_hit();
        let o = w.run(1, IDLE)[0].clone();
        if o.state != StateName::Attack {
            break;
        }
        frames.push(o.clip_frame);
    }
    assert!(frames.len() < 200, "never left the swing");
    assert!(frames.windows(2).all(|p| p[1] >= p[0]), "the swing went back: {frames:?}");
}

/// A host's own combo with KH2's abilities on top (`would_take`, the KH1 port's KH1 combo
/// style): what KH2's combo would take at the host's count, without changing the controller.
#[test]
fn would_take_reads_kh2_ability_moves_over_a_host_combo() {
    let mut ch = sora();
    ch.moveset.clips.insert(130, attack_clip(130, 40, (10, 16), 6, 20));
    ch.moveset.clips.insert(140, attack_clip(140, 40, (10, 16), 7, 20));
    // An ability's step (as Dodge Slash), before the plain swing in table order.
    let mut step = row(4, 130, false, false);
    step.ability = 91;
    ch.moveset.combo_rows.insert(1, step);
    // A Square move (as Upper Slash): reached only by the action button.
    let mut upper = row(12, 140, false, false);
    upper.kind = 0;
    upper.ability = 18;
    ch.moveset.combo_rows.push(upper);
    let env = Env { pos: [0.0; 3], grounded: true, height_above_ground: 0.0, targets: &[], ledges: &[] };
    let at = |count: u8, next_finisher: bool| Some(ComboState { count, finishers: 1, hit: true, finisher_next: next_finisher });
    let pick = |ch: &mut Character, from: Option<ComboState>, square: bool| ch.would_take(from, false, square, [0.0; 2], &env).map(|r| r.id);
    // Not equipped: the plain swing, then the finisher at the combo's end.
    assert_eq!(pick(&mut ch, at(1, false), false), Some(0));
    assert_eq!(pick(&mut ch, at(2, false), false), Some(1));
    assert_eq!(pick(&mut ch, at(1, false), true), None);
    ch.abilities.equipped.extend([91, 18]);
    assert_eq!(pick(&mut ch, at(1, false), false), Some(4));
    assert_eq!(pick(&mut ch, at(1, false), true), Some(12));
    // A Square step only while the combo has a swing left before its finisher.
    assert_eq!(pick(&mut ch, at(2, false), true), None);
    // Negative Combo: the finisher a swing sooner.
    ch.abilities.negative_combo = 1;
    assert_eq!(pick(&mut ch, at(1, false), false), Some(1));
    ch.abilities.negative_combo = 0;
    // Finishing Plus: another finisher after the first.
    // (Without it, a press after the finisher is a new combo's first swing.)
    ch.abilities.finishing_plus = 0;
    assert_eq!(pick(&mut ch, at(3, true), false), Some(4));
    ch.abilities.finishing_plus = 1;
    assert_eq!(pick(&mut ch, at(3, true), false), Some(1));
    // Nothing of the controller changed.
    assert_eq!(ch.combo_count(), 0);
    assert_eq!(ch.state_name(), StateName::Idle);
    assert!(ch.moveset.clips.contains_key(&130));
}

/// KH2's ability rows inside a host's own picker (`ability_row`, the KH1 port's Base Combo KH1):
/// the host's count picks the kind, KH2's own range checks pick the row, KH2's length plays no
/// part, and nothing of the controller changes.
#[test]
fn ability_row_joins_a_host_picker_at_its_count() {
    let mut ch = sora();
    ch.moveset.clips.insert(130, attack_clip(130, 40, (10, 16), 6, 20));
    ch.moveset.clips.insert(150, attack_clip(150, 40, (10, 16), 8, 20));
    // An ability swing that wants the primary target 0..200 away (kind 1, as Slide Dash's).
    let mut step = row(4, 130, false, false);
    (step.ability, step.kind, step.far, step.low, step.high) = (91, 1, 200.0, 500.0, -500.0);
    // An ability finisher with no range (kind 3, as Explosion's all-foes row checks none here).
    let mut fin = row(5, 150, true, false);
    fin.ability = 94;
    ch.moveset.combo_rows.extend([step, fin]);
    let any = |_: &ComboRow| true;
    let near = [TargetInfo { id: 7, pos: [0.0, 0.0, 100.0], radius: 10.0, height: 100.0 }];
    let far = [TargetInfo { id: 7, pos: [0.0, 0.0, 900.0], radius: 10.0, height: 100.0 }];
    fn env(t: &[TargetInfo]) -> Env<'_> {
        Env { pos: [0.0; 3], grounded: true, height_above_ground: 0.0, targets: t, ledges: &[] }
    }
    // Not equipped: nothing.
    assert!(ch.ability_row(false, false, None, &any, [0.0; 2], &env(&near)).is_none());
    ch.abilities.equipped.extend([91, 94]);
    // A swing: the ability swing in range, none out of range (the host's own row then).
    assert_eq!(ch.ability_row(false, false, None, &any, [0.0; 2], &env(&near)).map(|r| r.0.id), Some(4));
    assert!(ch.ability_row(false, false, None, &any, [0.0; 2], &env(&far)).is_none());
    // A finisher: the ability finisher, whatever KH2's own length (Combo Plus) says.
    ch.abilities.combo_plus = 3;
    assert_eq!(ch.ability_row(true, false, None, &any, [0.0; 2], &env(&far)).map(|r| r.0.id), Some(5));
    // Its motion playing: passed over (KH2's anti-repeat); one the host can't play: passed over.
    assert!(ch.ability_row(true, false, Some(150), &any, [0.0; 2], &env(&far)).is_none());
    assert!(ch.ability_row(true, false, None, &|r: &ComboRow| r.id != 5, [0.0; 2], &env(&far)).is_none());
    // Not in the air.
    assert!(ch.ability_row(true, true, None, &any, [0.0; 2], &env(&far)).is_none());
    assert_eq!(ch.combo_count(), 0);
    assert_eq!(ch.state_name(), StateName::Idle);
}

#[test]
fn ability_rows_by_id_and_range_checks_for_a_host_walk() {
    let mut ch = sora();
    ch.moveset.clips.insert(150, attack_clip(150, 40, (10, 16), 8, 20));
    ch.moveset.clips.insert(151, attack_clip(151, 40, (10, 16), 8, 20));
    // A group finisher (kind 2, any target 0..300 off the aim, as Explosion's row 26) and a
    // no-range one (kind 3, as Guard Break's row 27).
    let mut group = row(26, 150, true, false);
    (group.ability, group.kind, group.far, group.low, group.high, group.inner_min, group.inner_max) = (94, 2, 300.0, 500.0, -500.0, -1.0, 0.92);
    let mut single = row(27, 151, true, false);
    (single.ability, single.kind) = (93, 3);
    ch.moveset.combo_rows.extend([group, single]);
    let any = |_: &ComboRow| true;
    let one = [TargetInfo { id: 7, pos: [0.0, 0.0, 100.0], radius: 10.0, height: 100.0 }];
    let two = [TargetInfo { id: 7, pos: [0.0, 0.0, 100.0], radius: 10.0, height: 100.0 }, TargetInfo { id: 8, pos: [150.0, 0.0, 0.0], radius: 10.0, height: 100.0 }];
    fn env(t: &[TargetInfo]) -> Env<'_> {
        Env { pos: [0.0; 3], grounded: true, height_above_ground: 0.0, targets: t, ledges: &[] }
    }
    // The range checks alone, whatever is equipped: the group row needs a second target off the
    // line to the primary one; the no-range row always passes.
    ch.set_auto_target(true);
    assert!(!ch.row_in_range(26, &env(&one)));
    assert!(ch.row_in_range(26, &env(&two)));
    assert!(ch.row_in_range(27, &env(&one)));
    assert!(!ch.row_in_range(99, &env(&one)));
    // By id: not equipped, nothing; equipped, its own checks.
    assert!(ch.ability_row_id(27, true, false, None, &any, [0.0; 2], &env(&one)).is_none());
    ch.abilities.equipped.extend([93, 94]);
    assert_eq!(ch.ability_row_id(27, true, false, None, &any, [0.0; 2], &env(&one)).map(|r| r.0.id), Some(27));
    assert!(ch.ability_row_id(26, true, false, None, &any, [0.0; 2], &env(&one)).is_none());
    assert_eq!(ch.ability_row_id(26, true, false, None, &any, [0.0; 2], &env(&two)).map(|r| r.0.id), Some(26));
    // A swing press doesn't take a finisher row; its motion playing doesn't either.
    assert!(ch.ability_row_id(27, false, false, None, &any, [0.0; 2], &env(&one)).is_none());
    assert!(ch.ability_row_id(27, true, false, Some(151), &any, [0.0; 2], &env(&one)).is_none());
    // Locked on the one ahead, the group row still sees the other (KH2's kind-2 pass counts every
    // enemy, the lock playing no part), and still needs it.
    ch.set_lock(Some(7));
    assert!(ch.row_in_range(26, &env(&two)));
    assert!(!ch.row_in_range(26, &env(&one)));
    assert_eq!(ch.ability_row_id(26, true, false, None, &any, [0.0; 2], &env(&two)).map(|r| r.0.id), Some(26));
    assert_eq!(ch.state_name(), StateName::Idle);
}

/// KH2's normal finisher for a host's next finisher (`normal_finisher`, the KH1 port's Finishing
/// Plus after KH1's own normal finisher): 154 at a group, else 155 (rows 2 / 3 in Sora's table; 40 /
/// 41 here, past the test moveset's own), the one playing
/// passed over for its `sub`.
#[test]
fn normal_finisher_by_kh2s_walk() {
    let mut ch = sora();
    ch.moveset.clips.insert(154, attack_clip(154, 40, (10, 16), 8, 20));
    ch.moveset.clips.insert(155, attack_clip(155, 40, (10, 16), 8, 20));
    let mut group = row(40, 154, true, false);
    (group.kind, group.far, group.low, group.high, group.inner_min, group.inner_max) = (2, 180.0, 500.0, -500.0, -1.0, 0.92);
    let mut last = row(41, 155, true, false);
    last.sub = 40;
    ch.moveset.combo_rows.retain(|r| !r.is_finisher() && r.id != 40 && r.id != 41);
    ch.moveset.combo_rows.extend([group, last]);
    let one = [TargetInfo { id: 7, pos: [0.0, 0.0, 100.0], radius: 10.0, height: 100.0 }];
    let two = [TargetInfo { id: 7, pos: [0.0, 0.0, 100.0], radius: 10.0, height: 100.0 }, TargetInfo { id: 8, pos: [120.0, 0.0, 0.0], radius: 10.0, height: 100.0 }];
    fn env(t: &[TargetInfo]) -> Env<'_> {
        Env { pos: [0.0; 3], grounded: true, height_above_ground: 0.0, targets: t, ledges: &[] }
    }
    ch.set_auto_target(true);
    assert_eq!(ch.normal_finisher(false, None, [0.0; 2], &env(&one)).map(|r| r.0.id), Some(41));
    assert!(ch.row_in_range(40, &env(&two)));
    assert_eq!(ch.normal_finisher(false, None, [0.0; 2], &env(&two)).map(|r| r.0.id), Some(40));
    // 155 playing: its sub, 154, whatever its range.
    assert_eq!(ch.normal_finisher(false, Some(155), [0.0; 2], &env(&one)).map(|r| r.0.id), Some(40));
    // None in the air here.
    assert!(ch.normal_finisher(true, None, [0.0; 2], &env(&one)).is_none());
    assert_eq!(ch.state_name(), StateName::Idle);
}
