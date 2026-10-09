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
