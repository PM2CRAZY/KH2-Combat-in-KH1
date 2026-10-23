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

/// A row that jumps Sora to a foe in the air, shaped like KH2's rows 0 / 33 / 48: a ground row
/// (flags 2 | 8), height window -110..-250 (y-down) less the target's size, the primary target
/// dead ahead, first in the table as there.
fn jump_to_foe_row() -> ComboRow {
    ComboRow {
        id: 30, kind: 1, sub: -1, combo_offset: 0, flag: FLAG_JUMP_ADJUST | FLAG_LOW_SUB, motion: 120, next_motion: 4,
        jump: 17.0, jump_min: 17.0, jump_max: 67.0, speed_min: 0.0, speed_max: 0.0, near: 0.0, far: 260.0,
        low: -110.0, high: -250.0, inner_min: 0.99996, inner_max: 1.0, blend_time: 0.0, distance_adjust: 0.0,
        ability: 0, score: 1,
    }
}

/// Mashes Attack with every swing connecting: the swings started, and whether he left the ground.
fn mash_at(target: TargetInfo) -> (Vec<u16>, bool) {
    let mut ch = sora();
    ch.moveset.combo_rows.insert(0, jump_to_foe_row());
    let mut w = World::new(ch);
    w.targets = vec![target];
    let (mut motions, mut rose) = (Vec::new(), false);
    for f in 0..200 {
        let o = w.step(Pad { attack: f % 2 == 0, ..IDLE });
        if o.state == StateName::Attack && o.clip_frame == 0 {
            motions.push(o.motion);
        }
        if !o.hitboxes.is_empty() {
            w.ch.register_hit();
        }
        rose |= w.pos[1] > 0.0 || w.ch.airborne();
    }
    (motions, rose)
}

#[test]
fn a_tall_grounded_target_gets_the_ground_combo() {
    // Hercules-like: standing on Sora's floor, 220 tall, a step ahead. Measured by its body (the
    // middle of its height span, half the span its size) it is no foe in the air: the ground
    // combo all through, its second swing included.
    let (motions, rose) = mash_at(TargetInfo::body(7, 0.0, 100.0, 0.0, 220.0, 40.0));
    assert_eq!(motions[..3], [100, 100, 110], "{motions:?}");
    assert!(!rose, "the ground combo jumped him: {motions:?}");

    // A foe flying above him (its body 150..200 up): the jump to it, then the air combo.
    let (motions, rose) = mash_at(TargetInfo::body(7, 0.0, 100.0, 150.0, 200.0, 40.0));
    assert_eq!(motions.first(), Some(&120), "{motions:?}");
    assert!(rose);

    // The same tall foe measured from a point high on its body with only that point's size (KH1's
    // lock point, as before): read as a foe in the air, Sora jumped to it.
    let (motions, rose) = mash_at(TargetInfo { id: 7, pos: [0.0, 170.0, 100.0], radius: 40.0, height: 30.0 });
    assert_eq!(motions.first(), Some(&120), "{motions:?}");
    assert!(rose);
}

#[test]
fn a_blocked_glide_is_noted_not_started() {
    let mut w = World::new(sora());
    w.ch.abilities.aerial_dodge = 0;
    w.ch.glide_blocked = true;
    w.run(1, Pad { jump: true, ..IDLE });
    w.run(12, IDLE);
    assert!(w.pos[1] > 60.0);
    let o = w.step(Pad { jump: true, ..IDLE });
    assert_ne!(o.state, StateName::Glide);
    assert!(w.ch.take_glide_refused(), "the press is noted for the host");
    assert!(!w.ch.take_glide_refused(), "once");
    // A glide already playing ends in the fall once blocked.
    w.ch.glide_blocked = false;
    let mut w = World::new(sora());
    w.ch.abilities.aerial_dodge = 0;
    w.run(1, Pad { jump: true, ..IDLE });
    w.run(12, IDLE);
    assert_eq!(w.step(Pad { jump: true, ..IDLE }).state, StateName::Glide);
    w.ch.glide_blocked = true;
    let o = w.step(Pad { jump: true, ..IDLE });
    assert_eq!(o.state, StateName::Air);
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
        ch.moveset.clips.insert(m, c);
    }
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
    // From frame 8 a jump cuts the flinch; an Attack doesn't (the user: no attacking out of a
    // damage reaction).
    let mut w2 = World::new({ let mut c = sora(); hurt_clips(&mut c); c });
    w2.step(IDLE);
    w2.ch.take_hit(&reaction::Hit { reaction: 3, knockback: (125, 0), dir: [0.0, -1.0], part: 0 });
    w2.run(8, IDLE);
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

/// An air combo goes on through the fall between swings and ends with its finisher until the
/// landing (the KH1 port's player: Aerial Spiral, a jump row, lifted Sora on every press, and a
/// press after each swing had ended started the count again, so the air combo never ended while
/// a foe flew near).
#[test]
fn air_combo_ends_before_the_landing_however_slow_the_presses() {
    let mut ch = sora();
    ch.moveset.clips.insert(192, attack_clip(192, 30, (5, 10), 6, 15));
    let mut spiral = row(36, 192, false, true);
    spiral.jump = 120.0;
    // (Before the base air swing, as Aerial Spiral's row is in KH2's table.)
    ch.moveset.combo_rows.insert(3, spiral);
    let mut w = World::new(ch);
    w.run(3, Pad { jump: true, ..IDLE });
    w.run(12, IDLE);
    // A press every 34 frames: each one after the last swing (30 frames) is over, in the fall.
    let mut motions = Vec::new();
    let mut landed = None;
    for f in 0..1200 {
        let o = w.step(Pad { attack: f % 34 == 0, ..IDLE });
        if o.state == StateName::Attack && o.clip_frame == 0 {
            motions.push(o.motion);
        }
        if w.pos[1] <= 0.0 {
            landed = Some(f);
            break;
        }
    }
    assert!(landed.is_some(), "never landed: {} air swings {motions:?}", motions.len());
    let max = w.ch.combo_rules.air_base as usize + w.ch.abilities.air_combo_plus as usize;
    assert!(motions.len() <= max, "{} air swings in an air combo of {max}: {motions:?}", motions.len());
    assert_eq!(motions.last(), Some(&120), "the air combo ends in its finisher: {motions:?}");
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
    let outs = w.run(80, Pad { jump: true, ..IDLE });
    assert_eq!(outs.last().unwrap().motion, motion::GLIDE_HOVER);
    assert!(outs[0].motion != motion::GLIDE_HOVER, "hovered before slowing down");
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
    ch.moveset.clips.insert(71, Clip::plain(71, 30));
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

/// A moveset whose guard row is a move with no guard and no hit (Anti Form's Anti Glide, motion
/// 173): Square, standing or moving, plays it with its root motion and its invulnerable frames,
/// with no guard ability and nothing blocked (the KH1 port's user: "lets add anti-form's square
/// move"; s283: no guard in Anti).
#[test]
fn guard_row_move_plays_from_square_without_a_guard() {
    for stick in [[0.0, 0.0], [0.0, 0.4]] {
        let mut ch = sora();
        ch.abilities.guard = false;
        ch.abilities.dodge_roll = 0;
        let len = 60;
        let mut clip = Clip::plain(173, len);
        clip.root = (0..=len).map(|f| [0.0, 0.0, f as f32 * 10.0]).collect();
        clip.windows.push(Window { start: 0, end: Some(8), kind: WindowKind::Invulnerable });
        clip.windows.push(Window { start: 2, end: None, kind: WindowKind::NoGravity });
        ch.moveset.clips.insert(173, clip);
        ch.moveset.combo_rows.push(ComboRow { kind: 0, ..row(11, 173, false, false) });
        let mut w = World::new(ch);
        w.run(5, Pad { stick, ..IDLE });
        let z = w.pos[2];
        let o = w.step(Pad { action: true, stick, ..IDLE });
        assert_eq!(o.state, StateName::Attack, "stick {stick:?}");
        assert_eq!(o.motion, 173);
        assert!(o.invulnerable, "not invulnerable on its first frame");
        assert!(!o.guarding && o.guard_boxes.is_empty(), "it guards");
        let outs = w.run(20, IDLE);
        assert!(!outs.last().unwrap().invulnerable, "invulnerable past its window");
        assert!(w.pos[2] - z > 150.0, "didn't move by its root motion: {} -> {}", z, w.pos[2]);
    }
}

/// A form's Square swing (`square_swing`: Anti Form's Side Spin, row 12, ability 0): Square in a
/// ground combo's window goes into it with no ability equipped, only while a swing is left before
/// the finisher; without `square_swing` the row is never taken (Sora's Upper Slash rule).
#[test]
fn square_swing_in_a_ground_combo() {
    for set in [true, false] {
        let mut ch = sora();
        ch.abilities.combo_master = true;
        ch.moveset.clips.insert(161, attack_clip(161, 40, (9, 19), 542, 30));
        ch.moveset.combo_rows.push(ComboRow { kind: 0, ..row(12, 161, false, false) });
        if set {
            ch.square_swing = Some(12);
        }
        let mut w = World::new(ch);
        let mut motions = Vec::new();
        for f in 0..400 {
            // One swing, then Square only.
            let o = w.step(Pad { attack: f < 2, action: f >= 22 && f % 2 == 0, ..IDLE });
            if o.state == StateName::Attack && o.clip_frame == 0 {
                motions.push(o.motion);
            }
        }
        let spins = motions.iter().filter(|&&m| m == 161).count();
        if set {
            assert_eq!(motions.first(), Some(&100), "{motions:?}");
            assert!(spins >= 1, "Side Spin never came out: {motions:?}");
            assert!(spins <= w.ch.combo_rules.ground_base as usize - 1, "{spins} Side Spins in one combo: {motions:?}");
        } else {
            assert_eq!(spins, 0, "taken without square_swing: {motions:?}");
        }
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

/// A host's scripted chain (KH2's Limit Form scripts push the next motion): the queued motion
/// starts the frame the scripted one ends, with no free frame between; any other action
/// starting drops the queue.
#[test]
fn a_pushed_scripted_motion_follows_without_a_gap() {
    let mut ch = sora();
    ch.moveset.clips.insert(211, Clip::plain(211, 6));
    ch.moveset.clips.insert(212, Clip::plain(212, 8));
    let mut w = World::new(ch);
    assert!(!w.ch.push_scripted(Some(212)), "nothing scripted plays yet");
    assert!(w.ch.play(211, false));
    assert!(w.ch.push_scripted(Some(212)));
    let outs = w.run(16, IDLE);
    let motions: Vec<u16> = outs.iter().map(|o| o.motion).collect();
    assert_eq!(&motions[..7], &[211; 7], "{motions:?}");
    assert_eq!(&motions[7..15], &[212; 8], "{motions:?}");
    assert!(outs[7..15].iter().all(|o| o.state == StateName::Scripted));
    assert_ne!(outs[15].state, StateName::Scripted);
    // Dropped by another start.
    assert!(w.ch.play(211, false));
    assert!(w.ch.push_scripted(Some(212)));
    assert!(w.ch.play(211, false));
    assert_eq!(w.ch.scripted_next(), None);
}

/// A kind-2 row (checked against every target near) with a facing window and a lenient height
/// window, needing `ability`.
fn all_row(id: u8, motion: u16, finisher: bool, air: bool, far: f32, inner: (f32, f32), ability: u16) -> ComboRow {
    ComboRow { kind: 2, far, low: 500.0, high: -500.0, inner_min: inner.0, inner_max: inner.1, ability, ..row(id, motion, finisher, air) }
}

/// The test moveset with rows shaped as KH2's: Vicinity Break (ability 180, a ground swing on a
/// foe behind the one he aims at, within 3 m), and an air finisher of Magnet Burst (ability 179:
/// a foe off to the side within 6.5 m, or the aimed one alone 3..6.5 m away) ahead of the base
/// air finisher for a foe beside (no ability) and the lone one's.
fn surround_sora() -> Character {
    let mut ch = sora();
    for m in [170, 183, 184, 197] {
        ch.moveset.clips.insert(m, attack_clip(m, 30, (5, 10), 6, 15));
    }
    let mut magnet_far = row(50, 197, true, true);
    magnet_far.kind = 1;
    magnet_far.near = 300.0;
    magnet_far.far = 650.0;
    magnet_far.low = 500.0;
    magnet_far.high = -500.0;
    magnet_far.inner_min = 0.9999;
    magnet_far.inner_max = 1.0;
    magnet_far.ability = 179;
    ch.moveset.combo_rows = vec![
        row(1, 110, true, false),
        all_row(51, 170, false, false, 300.0, (-1.0, -0.17), 180),
        row(0, 100, false, false),
        all_row(49, 197, true, true, 650.0, (-1.0, 0.92), 179),
        all_row(7, 184, true, true, 260.0, (-0.7, 0.92), 0),
        magnet_far,
        row(8, 183, true, true),
        row(2, 120, false, true),
    ];
    ch.abilities.combo_master = true;
    ch
}

fn foe(id: u32, x: f32, z: f32) -> TargetInfo {
    TargetInfo { id, pos: [x, 0.0, z], radius: 30.0, height: 100.0 }
}

/// The attack motions a mashed ground combo (or, with `air`, an air combo after a jump) plays.
fn combo_motions(ch: Character, targets: Vec<TargetInfo>, lock: Option<u32>, air: bool) -> Vec<u16> {
    let mut w = World::new(ch);
    w.targets = targets;
    w.ch.set_lock(lock);
    if air {
        w.run(3, Pad { jump: true, ..IDLE });
        w.run(12, IDLE);
    }
    let mut motions = Vec::new();
    for f in 0..160 {
        let o = w.step(Pad { attack: true, ..IDLE }.with_attack(f));
        if o.state == StateName::Attack && o.clip_frame == 0 {
            motions.push(o.motion);
        }
    }
    motions
}

/// Vicinity Break (the player: never seen with it equipped): KH2 uses it when foes are around
/// him, one behind the one he aims at. With three around it comes, locked on (KH1's R1 lock or
/// its soft target, nearly always there) or not; with one, never, wherever it stands (with no
/// lock and no auto-target the rows still measure from the nearest, as KH2 does).
#[test]
fn vicinity_break_when_surrounded_only() {
    let mut ch = surround_sora();
    ch.abilities.equipped = vec![180];
    let around = vec![foe(1, 0.0, 100.0), foe(2, 30.0, -150.0), foe(3, 150.0, 0.0)];
    for lock in [Some(1), None] {
        let m = combo_motions(ch.clone(), around.clone(), lock, false);
        assert!(m.contains(&170), "lock {lock:?}, three around: {m:?}");
    }
    let mut no_aim = ch.clone();
    no_aim.set_auto_target(false);
    assert!(combo_motions(no_aim.clone(), around.clone(), None, false).contains(&170), "no auto-target");
    // One foe: ahead (locked), or behind him with nothing locked (aimed at or not).
    assert!(!combo_motions(ch.clone(), vec![foe(1, 0.0, 100.0)], Some(1), false).contains(&170));
    assert!(!combo_motions(ch.clone(), vec![foe(2, 30.0, -150.0)], None, false).contains(&170));
    assert!(!combo_motions(no_aim, vec![foe(2, 30.0, -150.0)], None, false).contains(&170));
    // Not equipped: never.
    let mut off = ch.clone();
    off.abilities.equipped.clear();
    assert!(!combo_motions(off, around.clone(), Some(1), false).contains(&170));
    // Only foes near count: the one behind 5 m away doesn't.
    assert!(!combo_motions(ch.clone(), vec![foe(1, 0.0, 100.0), foe(2, 30.0, -500.0)], Some(1), false).contains(&170));

    // The log line says why it wasn't taken, once per combo.
    let mut w = World::new(ch);
    w.targets = vec![foe(1, 0.0, 100.0)];
    w.ch.set_lock(Some(1));
    let mut notes = Vec::new();
    for f in 0..160 {
        w.step(Pad { attack: true, ..IDLE }.with_attack(f));
        while let Some(n) = w.ch.take_combo_note() {
            notes.push(n);
        }
    }
    assert!(!notes.is_empty() && notes[0].starts_with("Vicinity Break not taken"), "{notes:?}");
    assert!(notes[0].contains("deg from the aim"), "{notes:?}");
    assert!(notes.len() <= 2, "more than once a combo: {notes:?}");
}

/// Magnet Burst (the player: it fires against one enemy and not against groups, as if by chance):
/// KH2 has no chance on it; its air finisher row wants a foe off to the side of the one he aims
/// at, within 6.5 m, so a group brings it, locked on or not. Against one it comes only from its
/// other row, the aimed foe alone 3..6.5 m away (KH2's own); a lone foe close gets the plain
/// finisher.
#[test]
fn magnet_burst_with_foes_beside() {
    let mut ch = surround_sora();
    ch.abilities.equipped = vec![179];
    let group = vec![foe(1, 0.0, 100.0), foe(2, 250.0, 50.0), foe(3, -250.0, 50.0)];
    for lock in [Some(1), None] {
        let m = combo_motions(ch.clone(), group.clone(), lock, true);
        assert!(m.contains(&197), "lock {lock:?}, a group: {m:?}");
    }
    let m = combo_motions(ch.clone(), vec![foe(1, 0.0, 100.0)], Some(1), true);
    assert!(!m.contains(&197) && m.contains(&183), "one close: {m:?}");
    let m = combo_motions(ch.clone(), vec![foe(1, 0.0, 100.0)], None, true);
    assert!(!m.contains(&197), "one close, no lock: {m:?}");
    let m = combo_motions(ch.clone(), vec![foe(1, 0.0, 450.0)], Some(1), true);
    assert!(m.contains(&197), "one 4.5 m away: {m:?}");

    // The base finisher for a foe beside (no ability) is measured against the locked one alone
    // (the user, s206: the locked one's finisher, not the base one for the one beside).
    let mut base = ch.clone();
    base.abilities.equipped.clear();
    let m = combo_motions(base.clone(), group.clone(), Some(1), true);
    assert!(!m.contains(&184) && m.contains(&183), "locked: {m:?}");
    let m = combo_motions(base, group, None, true);
    assert!(m.contains(&184), "not locked: {m:?}");
}

/// Sora with Guard (its press motion), Dodge Roll and Quick Run as asked.
fn square_sora(guard: bool, roll: bool, quick_run: bool) -> Character {
    let mut ch = sora();
    ch.moveset.clips.insert(motion::GUARD_PRESS, Clip::plain(motion::GUARD_PRESS, 30));
    ch.abilities.guard = guard;
    ch.abilities.dodge_roll = if roll { 2 } else { 0 };
    ch.abilities.quick_run = if quick_run { 2 } else { 0 };
    ch
}

/// KH2's Square on the ground (exe `0x1403A75A0`), every combination of Guard, Dodge Roll and
/// Quick Run, standing and moving: standing, Guard; else Dodge Roll; else Quick Run; else Guard
/// (moving too). (The player: on keyboard, Guard or the roll always came out, never Quick Run.)
#[test]
fn square_priority_for_every_equipped_combination() {
    use StateName::*;
    for guard in [false, true] {
        for roll in [false, true] {
            for qr in [false, true] {
                for moving in [false, true] {
                    let want = match (moving, guard, roll, qr) {
                        (false, true, _, _) => Some(Guard),
                        (_, _, true, _) => Some(DodgeRoll),
                        (_, _, false, true) => Some(QuickRun),
                        (true, true, false, false) => Some(Guard),
                        _ => None,
                    };
                    let stick = if moving { [1.0, 0.0] } else { [0.0, 0.0] };
                    let mut w = World::new(square_sora(guard, roll, qr));
                    w.run(20, Pad { stick, ..IDLE });
                    let o = w.step(Pad { stick, action: true, ..IDLE });
                    let what = format!("guard {guard} roll {roll} quick run {qr} moving {moving}");
                    match want {
                        Some(s) => assert_eq!(o.state, s, "{what}"),
                        None => assert!(matches!(o.state, Idle | Walk | Run), "{what}: {:?}", o.state),
                    }
                }
            }
        }
    }
}

/// With Guard, Dodge Roll and Quick Run all equipped, Quick Run is still there: Square while
/// moving rolls, and the button held to the roll's quick-run check turns it into Quick Run.
#[test]
fn quick_run_out_of_the_roll_with_guard_equipped() {
    let mut w = World::new(square_sora(true, true, true));
    w.run(20, Pad { stick: [1.0, 0.0], ..IDLE });
    let outs = w.run(20, Pad { stick: [1.0, 0.0], action: true, ..IDLE });
    assert_eq!(outs[0].state, StateName::DodgeRoll);
    assert_eq!(outs[12].state, StateName::QuickRun);
    // Let go before the check: the roll stays a roll.
    let mut w = World::new(square_sora(true, true, true));
    w.run(20, Pad { stick: [1.0, 0.0], ..IDLE });
    let outs: Vec<_> = (0..20).map(|f| w.step(Pad { stick: [1.0, 0.0], action: f < 3, ..IDLE })).collect();
    assert!(outs.iter().all(|o| o.state != StateName::QuickRun));
}

/// Two quick presses: the second, made during the roll and held over its quick-run check, is no
/// hold of the first. The roll stays a roll (the player: a double tap went into Quick Run, which
/// KH2 doesn't do), and a press once the roll lets go of him is a new roll.
#[test]
fn a_double_tap_of_the_roll_is_no_quick_run() {
    let mut w = World::new(square_sora(true, true, true));
    w.run(20, Pad { stick: [1.0, 0.0], ..IDLE });
    let outs: Vec<_> = (0..30).map(|f| w.step(Pad { stick: [1.0, 0.0], action: f < 3 || f >= 5, ..IDLE })).collect();
    assert_eq!(outs[0].state, StateName::DodgeRoll);
    assert!(outs.iter().all(|o| o.state != StateName::QuickRun), "{:?}", outs.iter().map(|o| o.state).collect::<Vec<_>>());
}

/// Standing with no Guard and no roll (Wisdom Form's case), Quick Run dashes along his facing.
#[test]
fn quick_run_from_a_standstill_goes_along_the_facing() {
    let mut w = World::new(square_sora(false, false, true));
    w.run(20, Pad { stick: [0.0, 1.0], ..IDLE });
    w.run(30, IDLE);
    let z = w.pos[2];
    let o = w.step(Pad { action: true, ..IDLE });
    assert_eq!(o.state, StateName::QuickRun);
    w.run(8, Pad { action: true, ..IDLE });
    assert!(w.pos[2] - z > 50.0, "dashed {} along +z", w.pos[2] - z);
    assert!(w.pos[0].abs() < 1.0);
}

/// Quick Run's motions carry no free state and no combo window in KH2's data (only the
/// reaction-command range from frame 8), so the dash takes no Attack or jump; letting go of the
/// button ends it (its end motion), and the landing after it takes a jump.
#[test]
fn quick_run_ends_only_on_release() {
    let mut ch = square_sora(true, false, true);
    ch.moveset.clips.insert(motion::QUICK_RUN, Clip::plain(motion::QUICK_RUN, 60));
    ch.moveset.clips.insert(motion::QUICK_RUN_END, Clip::plain(motion::QUICK_RUN_END, 12));
    let mut w = World::new(ch);
    w.run(20, Pad { stick: [1.0, 0.0], ..IDLE });
    assert_eq!(w.step(Pad { stick: [1.0, 0.0], action: true, ..IDLE }).state, StateName::QuickRun);
    let outs: Vec<_> = (0..6).map(|f| w.step(Pad { stick: [1.0, 0.0], action: true, attack: f == 1, jump: f == 3, ..IDLE })).collect();
    assert!(outs.iter().all(|o| o.state == StateName::QuickRun && o.motion == motion::QUICK_RUN), "{:?}", outs.iter().map(|o| (o.state, o.motion)).collect::<Vec<_>>());
    let outs = w.run(2, Pad { stick: [1.0, 0.0], ..IDLE });
    assert_eq!((outs[1].state, outs[1].motion), (StateName::QuickRun, motion::QUICK_RUN_END));
    let outs = w.run(30, Pad { stick: [1.0, 0.0], ..IDLE });
    assert!(outs.iter().any(|o| o.state != StateName::QuickRun), "never left the Quick Run");
}

/// Wisdom Form's ground combo (s304, the user: in Wisdom "if a hit connects, it doesn't go into
/// the full combo, it just does the first hit over and over"). Its swings have no hit windows:
/// each fires shots (frame trigger 1 at frames 10, 20, 30 of motion 151) that land some frames
/// later, and the host tells the controller when one does (`register_hit`, as for a swing's own
/// hit). Without Combo Master (`one_swing_on_miss`): shots landing chain 151, 151 and the
/// finisher 154; none landing, each 151 plays out alone.
#[test]
fn wisdom_shots_landing_carry_the_combo() {
    let wisdom = || {
        let mut ms = Moveset::default();
        let mut swing = Clip::plain(151, 62);
        swing.windows.push(Window { start: 36, end: None, kind: WindowKind::Combo });
        for (frame, n) in [(10, 25), (20, 26), (30, 27)] {
            swing.events.push(Event { frame, kind: EventKind::Effect(n) });
        }
        ms.clips.insert(151, swing);
        let mut finisher = Clip::plain(154, 140);
        finisher.windows.push(Window { start: 92, end: Some(124), kind: WindowKind::Finisher });
        finisher.events.push(Event { frame: 16, kind: EventKind::Effect(28) });
        ms.clips.insert(154, finisher);
        // (Wisdom's PTYA rows 3 and 5 name themselves as their `sub`: 151 may follow 151.)
        ms.combo_rows = vec![ComboRow { sub: 3, ..row(3, 154, true, false) }, ComboRow { sub: 5, ..row(5, 151, false, false) }];
        let mut ch = Character::new(ms, MoveParams::default(), growth(), AttackParams::default());
        ch.abilities = Abilities { one_swing_on_miss: true, combo_master: false, ..Abilities::default() };
        ch
    };
    // Each shot lands `flight` frames after its trigger (or never).
    let run = |flight: Option<usize>| {
        let mut w = World::new(wisdom());
        let (mut seq, mut starts, mut landing) = (Vec::new(), Vec::new(), Vec::new());
        for f in 0..400 {
            let o = w.step(Pad { attack: true, ..IDLE }.with_attack(f));
            if o.state == StateName::Attack && o.clip_frame == 0 {
                seq.push(o.motion);
                starts.push(f);            }
            if let Some(t) = flight {
                landing.extend(o.events.iter().filter(|e| matches!(e, EventKind::Effect(_))).map(|_| f + t));
            }
            if landing.contains(&f) {
                w.ch.register_hit();
            }        }
        (seq, starts)
    };
    // Shots landing before the combo window opens (frame 36), and some landing after it (the
    // press that follows the hit chains: no buffer, so a mash).
    for t in [12, 30] {
        let (seq, _) = run(Some(t));
        assert_eq!(seq[..3], [151, 151, 154], "shots landing after {t} frames: {seq:?}");
    }
    let (seq, starts) = run(None);
    assert!(seq.len() >= 3 && seq.iter().all(|&m| m == 151), "{seq:?}");
    assert!(starts.windows(2).all(|p| p[1] - p[0] >= 62), "a missed swing was cut short: {starts:?}");
}
