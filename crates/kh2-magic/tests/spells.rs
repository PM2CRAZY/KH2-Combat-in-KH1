//! Runs every spell script from extracted files against a small fake
//! world. Research aid and smoke test: skipped unless the files exist
//! (`local/re/FIRE_1.mag`, ... and `03system.bin`, extracted from your own
//! install with `kh2-combat extract`).

use std::path::PathBuf;

use glam::{Mat4, Vec3};
use kh2_magic::{Event, Magic, ObjId, SpellFile, World};

struct Fake {
    frame: u32,
    release_at: u32,
    motion_end: u32,
    seed: u32,
}

const SORA: ObjId = 1;
const FOE: ObjId = 2;

impl World for Fake {
    fn exists(&self, _: ObjId) -> bool {
        true
    }
    fn pos(&self, o: ObjId) -> Vec3 {
        if o == FOE {
            Vec3::new(0.0, 0.0, 300.0)
        } else {
            Vec3::ZERO
        }
    }
    fn facing(&self, _: ObjId) -> f32 {
        0.0
    }
    fn motion(&self, o: ObjId) -> u16 {
        if o == SORA && self.frame < self.motion_end {
            56
        } else {
            0
        }
    }
    fn airborne(&self, _: ObjId) -> bool {
        false
    }
    fn jumping(&self, _: ObjId) -> bool {
        false
    }
    fn bone(&self, o: ObjId, _code: u16) -> Option<Mat4> {
        Some(Mat4::from_translation(self.pos(o)))
    }
    fn target_point(&self, o: ObjId) -> Vec3 {
        self.pos(o) + Vec3::Y * 60.0
    }
    fn target_of(&self, o: ObjId) -> Option<ObjId> {
        (o == SORA).then_some(FOE)
    }
    fn party(&self, n: u32) -> Option<ObjId> {
        (n == 0).then_some(SORA)
    }
    fn has_ability(&self, _: ObjId, _: u32) -> bool {
        false
    }
    fn ground(&self, from: Vec3, depth: f32) -> Option<Vec3> {
        (from.y >= 0.0 && from.y - depth <= 0.0).then(|| Vec3::new(from.x, 0.0, from.z))
    }
    fn bg_hit(&self, _: Vec3, to: Vec3) -> bool {
        to.y < 0.0
    }
    fn hitback(&mut self, _: ObjId, _: Vec3, _: f32) {}
    fn random(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.seed >> 8) as f32 / (1u32 << 24) as f32
    }
}

fn dir() -> PathBuf {
    std::env::var_os("KH2_RE_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../local/re"))
}

#[test]
fn every_spell_runs() {
    let d = dir();
    let Ok(system) = std::fs::read(d.join("03system.bin")) else {
        eprintln!("skipped: no extracted files in {}", d.display());
        return;
    };
    let pref = kh2_formats::pref::Pref::from_03system(&system).unwrap();
    let only = std::env::var("SPELL").ok();
    for (element, name) in ["FIRE", "BLIZZARD", "THUNDER", "CURE", "MAGNET", "REFLECT"].into_iter().enumerate() {
        if only.as_deref().is_some_and(|o| o != name) {
            continue;
        }
        for level in 1..=3 {
            let file = format!("{name}_{level}.mag");
            let Ok(bytes) = std::fs::read(d.join(&file)) else { continue };
            let mut magic = Magic::new();
            let spell = magic.add_spell(SpellFile::parse(&file, &bytes).unwrap());
            let mut w = Fake { frame: 0, release_at: 22, motion_end: 60, seed: 1 };
            let sub = (level as i32) * 2 - 2;
            magic.cast(spell, SORA, element as u8, sub, pref.magi.get(sub).unwrap(), &mut w).unwrap();
            let mut log = Vec::new();
            let mut max_attacks = 0;
            let mut atk_ids = std::collections::BTreeSet::new();
            let mut strikes = 0;
            for f in 1..400 {
                w.frame = f;
                if f == w.release_at {
                    assert!(magic.release(SORA, &mut w), "{file}: release");
                }
                for e in magic.update(&mut w).into_iter().chain(magic.take_events()) {
                    match e {
                        Event::Error(s) => panic!("{file}: {s}"),
                        Event::Missing(n) => log.push(format!("missing {n}")),
                        Event::Strike { .. } => strikes += 1,
                    }
                }
                let live: Vec<_> = magic.attacks.iter().filter(|a| !a.freed && a.pos.is_some()).collect();
                max_attacks = max_attacks.max(live.len());
                for a in &live {
                    atk_ids.insert((a.atk, a.sub));
                }
                if magic.casts.is_empty() && magic.attacks.iter().all(|a| a.freed) {
                    log.push(format!("finished at frame {f}"));
                    break;
                }
            }
            eprintln!("{file}: attacks {atk_ids:?} (max {max_attacks} at once), strikes {strikes}, {log:?}");
            assert!(log.iter().any(|l| l.starts_with("finished")), "{file} never finished: {log:?}");
        }
    }
}

/// Wisdom Form's shots: frame trigger 1 [25] runs the form model's script
/// (`wisd`) with its effect archive.
#[test]
fn wisdom_fires_a_homing_shot() {
    let d = dir();
    let (Ok(mdlx), Ok(archive)) = (std::fs::read(d.join("P_EX100_MAGF.mdlx")), std::fs::read(d.join("P_EX100_MAGF.a.us"))) else {
        eprintln!("skipped: no Wisdom files in {}", d.display());
        return;
    };
    let script = SpellFile::bar_entry(&mdlx, 3).expect("wisd script");
    let pax = SpellFile::bar_entry(&archive, 18).expect("effects");
    let mut magic = Magic::new();
    let file = magic.add_spell(SpellFile::from_parts("wisdomform", &script, &pax).unwrap());
    let mut w = Fake { frame: 0, release_at: 0, motion_end: 0, seed: 1 };
    magic.object_effect(file, SORA, 25, &mut w).unwrap();
    let mut path = Vec::new();
    for f in 1..80 {
        w.frame = f;
        for e in magic.update(&mut w) {
            if let Event::Error(s) = e {
                panic!("{s}");
            }
        }
        if let Some(a) = magic.attacks.iter().find(|a| !a.freed && a.pos.is_some()) {
            path.push((a.atk, a.sub, a.radius, a.pos.unwrap()));
        }
        if magic.casts.is_empty() {
            break;
        }
    }
    eprintln!("{} positions; first {:?}; last {:?}", path.len(), path.first(), path.last());
    assert!(path.len() > 10, "the shot flew: {path:?}");
    assert!(path.iter().all(|p| p.0 == 859 && p.1 == 0 && p.2 == 50.0));
    // It heads for the target in front (+z) and goes faster.
    let (a, b) = (path[1].3, path[path.len() - 1].3);
    assert!(b.z > a.z + 300.0, "{a} -> {b}");
}

/// Reflect: the barrier (ATKP 573, kind reflect) stops attacks; each one
/// counted (`attack_get_reflect_count`) makes the counter fire after it.
#[test]
fn reflect_counters_what_it_blocked() {
    let d = dir();
    let Ok(system) = std::fs::read(d.join("03system.bin")) else {
        eprintln!("skipped: no extracted files in {}", d.display());
        return;
    };
    let pref = kh2_formats::pref::Pref::from_03system(&system).unwrap();
    // Reflect, Reflera, Reflega: each level's own file, at its sub (level × 2 − 2).
    for level in 1..=3 {
        let file = format!("REFLECT_{level}.mag");
        let Ok(bytes) = std::fs::read(d.join(&file)) else { continue };
        let sub = level * 2 - 2;
        reflect_level(&file, &bytes, sub, pref.magi.get(sub).unwrap());
    }
}

fn reflect_level(file: &str, bytes: &[u8], sub: i32, magi: &[u8]) {
    for blocks in [0u8, 1, 3] {
        let mut magic = Magic::new();
        let spell = magic.add_spell(SpellFile::parse(file, bytes).unwrap());
        let mut w = Fake { frame: 0, release_at: 0, motion_end: 60, seed: 1 };
        magic.cast(spell, SORA, 5, sub, magi, &mut w).unwrap();
        assert!(magic.release(SORA, &mut w));
        // (frame, atk, radius, y) of each attack's first frame placed, and every counter frame.
        let mut seen: Vec<(u32, u16, f32, f32)> = Vec::new();
        let mut effects = std::collections::BTreeSet::new();
        let mut barrier_frames = 0;
        for f in 1..200 {
            w.frame = f;
            for e in magic.update(&mut w) {
                if let Event::Error(s) = e {
                    panic!("{s}");
                }
            }
            for e in magic.effects.iter().filter(|e| e.alive()) {
                effects.insert((e.element.id, e.element.bone, e.element.sound));
            }
            if f == 5 {
                if let Some(a) = magic.attacks.iter_mut().find(|a| a.atk == 573 && !a.freed) {
                    a.reflect_count = blocks;
                }
            }
            for a in magic.attacks.iter().filter(|a| !a.freed && a.pos.is_some() && a.radius > 0.0) {
                if a.atk == 573 {
                    barrier_frames += 1;
                    if barrier_frames > 1 {
                        continue;
                    }
                }
                seen.push((f, a.atk, a.radius, a.pos.unwrap().y));
            }
            if magic.casts.is_empty() {
                break;
            }
        }
        let counters: Vec<_> = seen.iter().filter(|s| s.1 != 573).collect();
        eprintln!("{file} sub {sub}, {blocks} blocked: barrier {barrier_frames} frames from {:?}, counters {counters:?}, effects (id, bone, sound) {effects:?}", seen.first());
        assert!(barrier_frames > 10);
        assert_eq!(blocks == 0, counters.is_empty(), "{counters:?}");
    }
}

/// Reflect held (`Magic::hold`, the host stretching the barrier while hits keep coming): the
/// barrier and its effect stay for the whole hold, the script picks up where it stopped, and
/// the counter still fires after.
#[test]
fn reflect_barrier_held() {
    let d = dir();
    let (Ok(system), Ok(bytes)) = (std::fs::read(d.join("03system.bin")), std::fs::read(d.join("REFLECT_1.mag"))) else {
        eprintln!("skipped: no extracted files in {}", d.display());
        return;
    };
    let pref = kh2_formats::pref::Pref::from_03system(&system).unwrap();
    let mut magic = Magic::new();
    let spell = magic.add_spell(SpellFile::parse("REFLECT_1.mag", &bytes).unwrap());
    let mut w = Fake { frame: 0, release_at: 0, motion_end: 60, seed: 1 };
    magic.cast(spell, SORA, 5, 0, pref.magi.get(0).unwrap(), &mut w).unwrap();
    assert!(magic.release(SORA, &mut w));
    let (hold_from, hold_to) = (24u32, 124u32);
    let (mut barrier, mut effect0, mut counter_from) = (Vec::new(), Vec::new(), None);
    for f in 1..300 {
        w.frame = f;
        magic.hold(SORA, (hold_from..hold_to).contains(&f));
        for e in magic.update(&mut w) {
            if let Event::Error(s) = e {
                panic!("{s}");
            }
        }
        if f == 5 {
            if let Some(a) = magic.attacks.iter_mut().find(|a| a.atk == 573 && !a.freed) {
                a.reflect_count = 2;
            }
        }
        if magic.attacks.iter().any(|a| a.atk == 573 && !a.freed) {
            barrier.push(f);
        }
        if magic.effects.iter().any(|e| e.element.id == 0 && e.alive()) {
            effect0.push(f);
        }
        if counter_from.is_none() && magic.attacks.iter().any(|a| a.atk == 574 && !a.freed && a.pos.is_some()) {
            counter_from = Some(f);
        }
        if magic.casts.is_empty() {
            break;
        }
    }
    let span = |v: &Vec<u32>| (v.first().copied(), v.last().copied());
    eprintln!("held {hold_from}..{hold_to}: barrier {:?}, effect 0 {:?}, counter from {counter_from:?}", span(&barrier), span(&effect0));
    assert!(barrier.last().is_some_and(|l| *l >= hold_to), "barrier through the hold: {:?}", span(&barrier));
    assert!(effect0.last().is_some_and(|l| *l >= hold_to - 1), "barrier effect through the hold: {:?}", span(&effect0));
    assert!(counter_from.is_some_and(|c| c > hold_to), "counter after the hold: {counter_from:?}");
}

/// Reflect cast into Reflect (the host's chain: the last cast is cancelled, its blocked hits
/// carried to the new barrier): the old barrier is gone at once, so the carried hits land on the
/// new one, which then counters for them (the user: the chain's burst never came; the carry
/// went to the old barrier, still standing until the next update).
#[test]
fn reflect_chain_counters_from_the_last_cast() {
    let d = dir();
    let (Ok(system), Ok(bytes)) = (std::fs::read(d.join("03system.bin")), std::fs::read(d.join("REFLECT_1.mag"))) else {
        eprintln!("skipped: no extracted files in {}", d.display());
        return;
    };
    let pref = kh2_formats::pref::Pref::from_03system(&system).unwrap();
    let magi = pref.magi.get(0).unwrap();
    let mut magic = Magic::new();
    let spell = magic.add_spell(SpellFile::parse("REFLECT_1.mag", &bytes).unwrap());
    let mut w = Fake { frame: 0, release_at: 0, motion_end: 60, seed: 1 };
    magic.cast(spell, SORA, 5, 0, magi, &mut w).unwrap();
    assert!(magic.release(SORA, &mut w));
    for f in 1..8 {
        w.frame = f;
        magic.update(&mut w);
        if f == 5 {
            magic.attacks.iter_mut().find(|a| a.atk == 573 && !a.freed).unwrap().reflect_count = 2;
        }
    }
    // Cast again, as `reflect::after_step` does.
    let carry = magic.attacks.iter().filter(|a| a.atk == 573 && !a.freed).map(|a| a.reflect_count).max().unwrap_or(0);
    assert_eq!(carry, 2);
    magic.cancel(SORA);
    assert!(magic.attacks.iter().all(|a| a.atk != 573 || a.freed), "the old barrier still stands after the cancel");
    w.frame = 0;
    magic.cast(spell, SORA, 5, 0, magi, &mut w).unwrap();
    assert!(magic.release(SORA, &mut w));
    let mut carried = false;
    let mut counters = 0;
    for f in 1..200 {
        w.frame = f;
        if !carried {
            if let Some(a) = magic.attacks.iter_mut().find(|a| a.atk == 573 && !a.freed) {
                a.reflect_count += carry;
                carried = true;
            }
        }
        magic.update(&mut w);
        counters += magic.attacks.iter().filter(|a| a.atk != 573 && !a.freed && a.pos.is_some()).count();
        if magic.casts.is_empty() {
            break;
        }
    }
    assert!(carried, "the new barrier never stood");
    assert!(counters > 0, "the last Reflect of the chain didn't counter");
}

/// Wisdom Form's spells: Fire, Blizzard, Thunder and Cure at Wisdom's magic sub (level × 2 − 2
/// + 10), as the KH1 port casts them in the form: each starts, puts out its attacks (Cure: a
/// direct application to Sora) and ends.
#[test]
fn wisdom_spells_run_at_their_sub() {
    let d = dir();
    let Ok(system) = std::fs::read(d.join("03system.bin")) else {
        eprintln!("skipped: no extracted files in {}", d.display());
        return;
    };
    let pref = kh2_formats::pref::Pref::from_03system(&system).unwrap();
    for (spell, name) in ["FIRE", "BLIZZARD", "THUNDER", "CURE"].iter().enumerate() {
        for level in 1..=3 {
            let file = format!("{name}_{level}.mag");
            let Ok(bytes) = std::fs::read(d.join(&file)) else { continue };
            let sub = level * 2 - 2 + 10;
            let mut magic = Magic::new();
            let s = magic.add_spell(SpellFile::parse(&file, &bytes).unwrap());
            let mut w = Fake { frame: 0, release_at: 0, motion_end: 40, seed: 1 };
            magic.cast(s, SORA, spell as u8, sub, pref.magi.get(sub).unwrap(), &mut w).unwrap();
            assert!(magic.release(SORA, &mut w));
            let (mut attacks, mut strikes, mut effects) = (std::collections::BTreeSet::new(), 0, 0);
            let mut done = false;
            for f in 1..600 {
                w.frame = f;
                for e in magic.update(&mut w) {
                    match e {
                        Event::Error(s) => panic!("{file}: {s}"),
                        Event::Strike { target, .. } if target == SORA => strikes += 1,
                        _ => {}
                    }
                }
                effects = effects.max(magic.effects.len());
                attacks.extend(magic.attacks.iter().filter(|a| a.pos.is_some()).map(|a| (a.atk, a.sub)));
                if magic.casts.is_empty() {
                    done = true;
                    break;
                }
            }
            eprintln!("{file} sub {sub}: attacks {attacks:?}, strikes on Sora {strikes}, effects {effects}");
            assert!(done, "{file} ended");
            assert!(!attacks.is_empty() || strikes > 0, "{file} did something");
            assert!(attacks.iter().all(|a| a.1 as i32 == sub || a.1 == 0), "{file}: at its sub");
        }
    }
}
