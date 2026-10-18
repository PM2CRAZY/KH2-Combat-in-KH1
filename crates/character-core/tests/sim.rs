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
