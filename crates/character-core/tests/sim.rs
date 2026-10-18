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
    // Buttons work again from frame 8.
    let mut w2 = World::new({ let mut c = sora(); hurt_clips(&mut c); c });
    w2.step(IDLE);
    w2.ch.take_hit(&reaction::Hit { reaction: 3, knockback: (125, 0), dir: [0.0, -1.0], part: 0 });
    w2.run(8, IDLE);
    let o = w2.step(Pad { attack: true, ..IDLE });
    assert_eq!(o.state, StateName::Attack);
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

/// An air cast that falls as it plays (`air_magic_falls`, as Final Form's air Reflect) ends on
/// touching the ground in KH2's landing, not standing in the rest of the cast (the user).
#[test]
fn falling_air_cast_lands_into_the_landing() {
    let mut ch = sora();
    let mut cast = Clip::plain(73, 200);
    cast.windows.push(Window { start: 0, end: Some(190), kind: WindowKind::NoGravity });
    ch.moveset.clips.insert(73, cast);
    ch.air_magic_falls = true;
    let mut w = World::new(ch);
    w.run(3, Pad { jump: true, ..IDLE });
    w.run(12, IDLE);
    assert!(w.pos[1] > 50.0);
    assert_eq!(w.ch.cast(CastMotions { ground: 71, finisher: Some(72), air: 73 }), Some(73));
    let outs = w.run(150, IDLE);
    let landed = outs.iter().position(|o| o.state == StateName::Land).expect("never landed");
    assert!(outs[..landed].iter().all(|o| o.motion == 73), "left the cast before landing");
    assert!(w.pos[1] <= 0.0);
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

/// A spell cast mid-combo inserts itself as a step of it (the user: as a modifier does), also
/// just after a swing has ended (picking it from the menu takes a moment): swing, spell, then
/// the next swing is the combo's finisher.
#[test]
fn spell_mid_combo_is_a_step_of_it() {
    let mut ch = sora();
    ch.moveset.clips.insert(71, Clip::plain(71, 30));
    let mut w = World::new(ch);
    w.step(Pad { attack: true, ..IDLE });
    for _ in 0..45 {
        let o = w.step(IDLE);
        if !o.hitboxes.is_empty() {
            w.ch.register_hit();
        }
    }
    w.run(10, IDLE);
    assert_eq!(w.ch.cast(CastMotions { ground: 71, finisher: None, air: 73 }), Some(71));
    w.run(40, IDLE);
    let o = w.step(Pad { attack: true, ..IDLE });
    assert_eq!(o.motion, 110, "the swing after the spell should be the finisher");
    // Long after a swing, a spell starts no combo step.
    let mut ch = sora();
    ch.moveset.clips.insert(71, Clip::plain(71, 30));
    let mut w = World::new(ch);
    w.step(Pad { attack: true, ..IDLE });
    w.run(200, IDLE);
    w.ch.cast(CastMotions { ground: 71, finisher: None, air: 73 });
    w.run(40, IDLE);
    assert_eq!(w.step(Pad { attack: true, ..IDLE }).motion, 100);
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
fn dodge_momentum_kept_with_the_stick() {
    // KH2's Aerial Dodge MAX speed; the fall after it, the stick held forward then sideways.
    let mut ch = magic_sora(0);
    for g in &mut ch.growth {
        g.air_dodge_speed = 36.0;
    }
    let speed_after = |stick_late: [f32; 2]| {
        let mut w = World::new(ch.clone());
        w.run(2, Pad { jump: true, ..IDLE });
        w.run(10, IDLE);
        w.step(Pad { jump: true, stick: [0.0, 1.0], ..IDLE });
        w.run(4, IDLE);
        let mut d = [0.0; 3];
        for _ in 0..10 {
            d = w.step(Pad { stick: stick_late, ..IDLE }).displacement;
        }
        (d[0] * d[0] + d[2] * d[2]).sqrt()
    };
    let let_go = speed_after([0.0, 0.0]);
    let forward = speed_after([0.0, 1.0]);
    let sideways = speed_after([1.0, 0.0]);
    let back = speed_after([0.0, -1.0]);
    assert!(forward >= let_go * 0.99, "forward {forward} vs let go {let_go}");
    assert!(sideways >= let_go * 0.99, "sideways {sideways} vs let go {let_go}");
    assert!(back < let_go * 0.8, "pushing back brakes: {back} vs {let_go}");
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
