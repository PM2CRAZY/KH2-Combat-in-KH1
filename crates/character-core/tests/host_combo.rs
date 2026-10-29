//! `HostPress` (KH2's press on a host's own state) against `Character`'s own press on the same
//! inputs: the same row every time (a made-up moveset, not KH2 data).

use character_core::*;

fn row(id: u8, kind: u8, sub: i8, flag: u32, motion: u16, near: f32, far: f32, ability: u16) -> ComboRow {
    ComboRow {
        id, kind, sub, combo_offset: 0, flag, motion, next_motion: 0, jump: 0.0, jump_min: 0.0, jump_max: 0.0,
        speed_min: 0.0, speed_max: 0.0, near, far, low: 100.0, high: -300.0, inner_min: -2.0, inner_max: 2.0,
        blend_time: 0.0, distance_adjust: 0.0, ability, score: 0,
    }
}

fn rows() -> Vec<ComboRow> {
    let mut leap = row(37, 0, -1, FLAG_FINISHER, 167, 0.0, 0.0, 95);
    leap.combo_offset = 1;
    let mut narrow = row(26, 2, -1, FLAG_FINISHER, 166, 0.0, 300.0, 33);
    narrow.inner_min = 0.5;
    vec![
        narrow,
        row(2, 2, 3, FLAG_FINISHER, 154, 0.0, 180.0, 0),
        row(27, 3, -1, FLAG_FINISHER, 165, 0.0, 0.0, 93),
        row(3, 3, 3, FLAG_FINISHER, 155, 0.0, 0.0, 0),
        row(14, 1, -1, 0, 160, 200.0, 500.0, 13),
        row(4, 1, 5, 0, 151, 0.0, 150.0, 0),
        row(5, 3, 6, 0, 150, 0.0, 0.0, 0),
        row(6, 3, 5, 0, 152, 0.0, 0.0, 0),
        row(8, 3, 8, FLAG_FINISHER | FLAG_AIR, 183, 0.0, 0.0, 0),
        row(9, 3, 10, FLAG_AIR, 180, 0.0, 0.0, 0),
        row(10, 3, 9, FLAG_AIR, 181, 0.0, 0.0, 0),
        row(49, 2, -1, FLAG_FINISHER | FLAG_AIR, 195, 0.0, 650.0, 34),
        leap,
    ]
}

fn character(rows: Vec<ComboRow>, equipped: Vec<u16>, finishing_plus: u8) -> Character {
    let mut ms = Moveset::default();
    for r in &rows {
        ms.clips.insert(r.motion, Clip::plain(r.motion, 40));
    }
    ms.combo_rows = rows;
    let mut ch = Character::new(ms, MoveParams::default(), Vec::new(), AttackParams::default());
    ch.abilities.equipped = equipped;
    ch.abilities.finishing_plus = finishing_plus;
    ch
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn f(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (self.next() % 10_000) as f32 / 10_000.0 * (hi - lo)
    }
}

#[test]
fn host_press_matches_the_characters_press() {
    let mut rng = Rng(7);
    let all_rows = rows();
    let playing_choices = [None, Some(150u16), Some(151), Some(155), Some(154), Some(180), Some(183)];
    let mut differ = 0;
    let mut seen = std::collections::BTreeSet::new();
    for case in 0..4000 {
        let equipped: Vec<u16> = [13u16, 33, 34, 93, 95].into_iter().filter(|_| rng.next() % 2 == 0).collect();
        let plus = (rng.next() % 3) as u8;
        let mut ch = character(all_rows.clone(), equipped.clone(), plus);
        ch.set_auto_target(false);
        let n = rng.next() % 4;
        let targets: Vec<TargetInfo> = (0..n)
            .map(|i| TargetInfo { id: 10 + i, pos: [rng.f(-600.0, 600.0), rng.f(-50.0, 200.0), rng.f(-600.0, 600.0)], radius: rng.f(10.0, 80.0), height: rng.f(10.0, 120.0) })
            .collect();
        let lock = if n > 0 && rng.next() % 3 != 0 { Some(targets[(rng.next() % n) as usize].id) } else { None };
        ch.set_lock(lock);
        let facing = rng.f(-3.1, 3.1);
        ch.set_facing(facing);
        let air = rng.next() % 3 == 0;
        let pos = [rng.f(-50.0, 50.0), if air { 120.0 } else { 0.0 }, rng.f(-50.0, 50.0)];
        let env = Env { pos, grounded: !air, height_above_ground: pos[1], targets: &targets, ledges: &[] };
        let playing = playing_choices[(rng.next() as usize) % playing_choices.len()];
        let len = ch.combo_length(air);
        let max_finishers = ch.combo_rules.finisher_base + plus;
        let host: Vec<HostTarget> = targets.iter().map(|t| HostTarget { id: t.id, pos: t.pos, radius: t.radius, height: t.height }).collect();
        let primary = lock.and_then(|id| host.iter().find(|t| t.id == id).copied());
        let has = |m: u16| all_rows.iter().any(|r| r.motion == m);
        let p = HostPress {
            rows: &all_rows,
            has_motion: &has,
            air,
            pos,
            facing,
            primary,
            targets: &host,
            equipped: &equipped,
            jump_height: ch.params.jump_height,
            max_count: len,
            max_finishers,
            berserk: false,
            drop_ahead: false,
            playing,
        };
        // A press in the combo at each count.
        let count = 1 + (rng.next() % (len as u32 + 1)) as u8;
        let from = ComboState { count: count - 1, finishers: 1, hit: true, finisher_next: false };
        let theirs = ch.would_take_launch(Some(from), playing, air, false, [0.0; 2], &env).map(|(r, _)| r.id);
        let ours = p.press(count).map(|r| r.id);
        seen.insert(ours);
        if theirs != ours {
            differ += 1;
            eprintln!("case {case}: press at {count}: character {theirs:?}, host {ours:?}");
        }
        // A press in a finisher's window.
        let fin = 1 + (rng.next() % 3) as u8;
        let from = ComboState { count, finishers: fin, hit: true, finisher_next: true };
        let theirs = ch.would_take_launch(Some(from), playing, air, false, [0.0; 2], &env).filter(|(r, _)| r.is_finisher()).map(|(r, _)| r.id);
        let ours = p.next_finisher(count, fin).map(|r| r.id);
        if theirs != ours {
            differ += 1;
            eprintln!("case {case}: next finisher at {count} / {fin}: character {theirs:?}, host {ours:?}");
        }
        // The extra candidates of a host's own picker.
        for finisher in [false, true] {
            let theirs = ch.ability_row(finisher, air, playing, &|_| true, [0.0; 2], &env).map(|(r, _)| r.id);
            let ours = p.ability_row(finisher, &|_| true).map(|r| r.id);
            if theirs != ours {
                differ += 1;
                eprintln!("case {case}: ability row ({finisher}): character {theirs:?}, host {ours:?}");
            }
        }
        for id in [26u8, 2, 27, 42, 49, 7, 29, 50] {
            assert_eq!(ch.row_in_range(id, &env), p.row_in_range(id), "case {case}: row {id}'s range");
        }
        let theirs = ch.normal_finisher(air, playing, [0.0; 2], &env).map(|(r, _)| r.id);
        let ours = p.normal_finisher().map(|r| r.id);
        if theirs != ours {
            differ += 1;
            eprintln!("case {case}: normal finisher: character {theirs:?}, host {ours:?}");
        }
    }
    assert_eq!(differ, 0);
    // (The cases reach many rows, refusals included.)
    assert!(seen.len() >= 8, "{seen:?}");
}

#[test]
fn no_clip_is_no_attack() {
    let rows = rows();
    let none = |m: u16| m != 150;
    let p = HostPress {
        rows: &rows,
        has_motion: &none,
        air: false,
        pos: [0.0; 3],
        facing: 0.0,
        primary: None,
        targets: &[],
        equipped: &[],
        jump_height: 0.0,
        max_count: 3,
        max_finishers: 1,
        berserk: false,
        drop_ahead: false,
        playing: None,
    };
    assert_eq!(p.press(1), None);
    assert!(!p.finisher_left(1));
}

fn hop_rows() -> Vec<ComboRow> {
    let mut rs = rows();
    // Hops: a ground jump (rows 0 / 1 style), an air swing hop, Retaliating Slash, the Square moves.
    let mut j = row(1, 3, -1, 0, 182, 0.0, 0.0, 0);
    (j.jump, j.jump_min, j.jump_max, j.speed_min, j.speed_max, j.distance_adjust) = (21.0, 17.0, 67.0, -8.0, 18.0, 85.0);
    rs.insert(0, j);
    for r in rs.iter_mut().filter(|r| r.id == 9 || r.id == 10) {
        (r.jump, r.jump_min, r.jump_max, r.speed_min, r.speed_max) = (14.0, 10.0, 40.0, 0.0, 12.0);
        r.flag |= FLAG_JUMP_ADJUST;
    }
    let mut ret = row(40, 0, -1, FLAG_AIR, 172, 0.0, 0.0, 101);
    (ret.jump, ret.jump_min, ret.jump_max, ret.speed_max) = (21.0, 21.0, 121.0, 10.0);
    rs.push(ret);
    rs.push(row(39, 0, -1, 0, 171, 0.0, 0.0, 96));
    rs.push(row(12, 0, -1, 0, 161, 0.0, 0.0, 18));
    let mut hs = row(38, 0, -1, FLAG_AIR, 193, 0.0, 0.0, 99);
    (hs.jump, hs.jump_min, hs.jump_max, hs.speed_max) = (14.0, 14.0, 14.0, 6.0);
    rs.push(hs);
    rs
}

fn close(a: Option<Launch>, b: Option<Launch>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => (a.height - b.height).abs() < 1e-3 && (a.velocity[0] - b.velocity[0]).abs() < 1e-3 && (a.velocity[1] - b.velocity[1]).abs() < 1e-3 && a.gravity == b.gravity,
        _ => false,
    }
}

#[test]
fn host_hops_forced_rows_and_square_moves_match_the_characters() {
    let mut rng = Rng(11);
    let all_rows = hop_rows();
    let mut differ = 0;
    let mut hops = 0;
    for case in 0..4000 {
        let equipped: Vec<u16> = [13u16, 33, 34, 93, 95, 101, 96, 18, 99].into_iter().filter(|_| rng.next() % 3 != 0).collect();
        let plus = (rng.next() % 3) as u8;
        let mut ch = character(all_rows.clone(), equipped.clone(), plus);
        ch.set_auto_target(false);
        ch.attack.target_lift = 70.0;
        ch.attack.target_radius = 55.0;
        ch.params.gravity = 0.4;
        ch.params.jump_height = 120.0;
        let n = rng.next() % 4;
        let targets: Vec<TargetInfo> = (0..n)
            .map(|i| TargetInfo { id: 10 + i, pos: [rng.f(-600.0, 600.0), rng.f(-50.0, 300.0), rng.f(-600.0, 600.0)], radius: rng.f(10.0, 80.0), height: rng.f(10.0, 120.0) })
            .collect();
        let lock = if n > 0 && rng.next() % 3 != 0 { Some(targets[(rng.next() % n) as usize].id) } else { None };
        ch.set_lock(lock);
        ch.set_face_lock(true);
        ch.set_combo_direction_lock(true);
        let facing = rng.f(-3.1, 3.1);
        ch.set_facing(facing);
        let air = rng.next() % 2 == 0;
        let pos = [rng.f(-50.0, 50.0), if air { 120.0 } else { 0.0 }, rng.f(-50.0, 50.0)];
        let env = Env { pos, grounded: !air, height_above_ground: pos[1], targets: &targets, ledges: &[] };
        let stick = if rng.next() % 2 == 0 { [rng.f(-1.0, 1.0), rng.f(-1.0, 1.0)] } else { [0.0, 0.0] };
        let len = ch.combo_length(air);
        let host: Vec<HostTarget> = targets.iter().map(|t| HostTarget { id: t.id, pos: t.pos, radius: t.radius, height: t.height }).collect();
        let primary = lock.and_then(|id| host.iter().find(|t| t.id == id).copied());
        let has = |m: u16| all_rows.iter().any(|r| r.motion == m);
        let p = HostPress {
            rows: &all_rows,
            has_motion: &has,
            air,
            pos,
            facing,
            primary,
            targets: &host,
            equipped: &equipped,
            jump_height: ch.params.jump_height,
            max_count: len,
            max_finishers: ch.combo_rules.finisher_base + plus,
            berserk: false,
            drop_ahead: false,
            playing: None,
        };
        let hop_in = |first: bool| HopIn { target_lift: 70.0, target_radius: 55.0, gravity: 0.4, stick, first, moving: false, vel_h: [0.0; 2] };
        // A press in the combo: its row and hop.
        let count = 1 + (rng.next() % (len as u32 + 1)) as u8;
        let from = ComboState { count: count - 1, finishers: 1, hit: true, finisher_next: false };
        let theirs = ch.would_take_launch(Some(from), None, air, false, stick, &env);
        let ours = p.press(count).map(|r| {
            let l = p.hop(&r, &hop_in(count - 1 == 0));
            (r, l)
        });
        match (&theirs, &ours) {
            (Some((a, la)), Some((b, lb))) if a.id == b.id && close(*la, *lb) => hops += la.is_some() as u32,
            (None, None) => {}
            _ => {
                differ += 1;
                eprintln!("case {case}: press {count}: character {:?}, host {:?}", theirs.as_ref().map(|t| (t.0.id, t.1)), ours.as_ref().map(|t| (t.0.id, t.1)));
            }
        }
        // Forced rows (Retaliating Slash, Counterguard, the guard press's row).
        for id in [40u8, 39, 1, 12] {
            let square = id == 12;
            let theirs = ch.would_take_row(id, Some(ComboState { count: 0, finishers: 1, hit: false, finisher_next: false }), air, stick, &env);
            let ours = p.forced(id, square).map(|r| {
                let l = p.hop(&r, &hop_in(true));
                (r, l)
            });
            let same = match (&theirs, &ours) {
                (Some((a, la)), Some((b, lb))) => a.id == b.id && close(*la, *lb),
                (None, None) => true,
                _ => false,
            };
            if !same {
                differ += 1;
                eprintln!("case {case}: forced {id}: character {:?}, host {:?}", theirs.as_ref().map(|t| (t.0.id, t.1)), ours.as_ref().map(|t| (t.0.id, t.1)));
            }
        }
        // The Square moves.
        let (combo, finisher, free) = (rng.next() % 2 == 0, rng.next() % 3 == 0, rng.next() % 2 == 0);
        let fin = 1 + (rng.next() % 2) as u8;
        let hit = rng.next() % 3 != 0;
        let sc = (rng.next() % (len as u32 + 1)) as u8;
        let skip: &[u8] = if rng.next() % 4 == 0 { &[0x25] } else { &[] };
        let from = ComboState { count: sc, finishers: fin, hit, finisher_next: false };
        let theirs = ch.would_take_square_exe(from, combo, finisher, free, skip, air, stick, &env);
        let ours = p.square(sc, fin, hit, combo, finisher, free, skip).map(|(r, fresh)| {
            let l = p.hop(&r, &hop_in(fresh || sc == 0));
            (r, l)
        });
        let same = match (&theirs, &ours) {
            (Some((a, la)), Some((b, lb))) => a.id == b.id && close(*la, *lb),
            (None, None) => true,
            _ => false,
        };
        if !same {
            differ += 1;
            eprintln!("case {case}: square ({sc}, {fin}, hit {hit}, combo {combo}, finisher {finisher}, free {free}): character {:?}, host {:?}", theirs.as_ref().map(|t| (t.0.id, t.1)), ours.as_ref().map(|t| (t.0.id, t.1)));
        }
    }
    assert_eq!(differ, 0);
    assert!(hops > 100, "{hops} hops compared");
}
