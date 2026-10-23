//! Limit Form's limits from its model's own script (`limi`, BAR entry 3 of
//! `P_EX100_KH1F.mdlx`) against a fake Sora whose motions come from the form's
//! moveset (`P_EX100_KH1F.mset`: lengths, script ranges, frame triggers; its combo rows from
//! `00battle.bin`). Skipped unless the files are extracted (`local/re`, or `KH2_RE_DIR`;
//! Infinity's shots also need the form's effects, `P_EX100_KH1F.a.us`).

use std::collections::HashMap;
use std::path::PathBuf;

use glam::{Mat3, Mat4, Vec3};
use kh2_formats::motion::Mset;
use kh2_magic::{Event, Magic, ObjId, SpellFile, World};

const SORA: ObjId = 1;
const FOE: ObjId = 2;
/// Sora's Keyblade as an object (Strike Raid throws it), and a second enemy (Infinity's shots
/// search for targets).
const WEAPON: ObjId = 3;
const FOE2: ObjId = 4;
/// Where the fake's Keyblade is held: in front of Sora, at hand height.
const HAND: Vec3 = Vec3::new(0.0, 100.0, 40.0);
/// The fake enemies' bodies: a sphere this big round their aim point (KH1's are 30..80 round).
const BODY: f32 = 80.0;
/// `00objentry` id of `P_EX100_KH1F` (the script checks for it).
const LIMIT_FORM_ENTRY: u32 = 0x95D;

/// A motion: its length (60 Hz frames) and its ranges (id, start, end, first parameter).
struct Motion {
    len: u32,
    ranges: Vec<(u8, u32, Option<u32>, i16)>,
    /// Frame triggers: id, frame, params.
    frames: Vec<(u8, u32, Vec<i16>)>,
}

struct Fake {
    motions: HashMap<u16, Motion>,
    motion: u16,
    frame: u32,
    /// Pushed motions, in order (0: idle, the chain's end).
    queued: Vec<u16>,
    facing: f32,
    offers: Vec<u16>,
    started: Vec<u16>,
    /// Combo row → its motion (PTYA set 0x42), and the rows the script played.
    rows: HashMap<u16, u16>,
    played_rows: Vec<u16>,
    /// Pushes the script asked for (direction, distance).
    hitbacks: Vec<(Vec3, f32)>,
    rng: u32,
}

impl Fake {
    fn new(mset: &Mset) -> Fake {
        let mut motions = HashMap::new();
        for m in 0..=255u16 {
            let Some(anb) = mset.slot_for(m as usize, true, true).and_then(|s| s.anb.as_ref()) else { continue };
            let Some(info) = &anb.motion else { continue };
            let len = ((info.frame_end - info.frame_start) / info.fps * 60.0).round().max(1.0) as u32;
            let ranges = anb.triggers.ranges.iter().map(|r| (r.id, r.start.max(0) as u32, (r.end >= 0).then_some(r.end as u32), r.params.first().copied().unwrap_or(0))).collect();
            let frames = anb.triggers.frames.iter().map(|f| (f.id, f.frame.max(0) as u32, f.params.clone())).collect();
            motions.insert(m, Motion { len, ranges, frames });
        }
        Fake { motions, motion: 0, frame: 0, queued: Vec::new(), facing: 0.0, offers: Vec::new(), started: Vec::new(), rows: HashMap::new(), played_rows: Vec::new(), hitbacks: Vec::new(), rng: 1 }
    }

    /// The motion's frame triggers `id` on this frame (their params).
    fn frame_triggers(&self, id: u8) -> Vec<Vec<i16>> {
        let f = self.frame;
        self.motions.get(&self.motion).map_or(Vec::new(), |m| m.frames.iter().filter(|t| t.0 == id && t.1 == f).map(|t| t.2.clone()).collect())
    }

    /// A command the motion itself offers now (range 20: the engine's reaction command).
    fn motion_offer(&self) -> Option<u16> {
        let f = self.frame;
        self.motions.get(&self.motion)?.ranges.iter().find(|&&(id, s, e, _)| id == 20 && f >= s && e.map_or(true, |e| f < e)).map(|r| r.3 as u16)
    }

    fn len(&self) -> u32 {
        self.motions.get(&self.motion).map_or(1, |m| m.len)
    }

    /// One frame of the motion (after the scripts ran, as the host's controller steps).
    fn advance(&mut self) {
        self.frame += 1;
        if self.frame >= self.len() {
            self.frame = 0;
            self.motion = if self.queued.is_empty() { 0 } else { self.queued.remove(0) };
        }
    }
}

impl World for Fake {
    fn exists(&self, _: ObjId) -> bool {
        true
    }
    fn pos(&self, o: ObjId) -> Vec3 {
        match o {
            FOE => Vec3::new(0.0, 0.0, 300.0),
            FOE2 => Vec3::new(250.0, 0.0, 450.0),
            WEAPON => HAND,
            _ => Vec3::ZERO,
        }
    }
    fn facing(&self, _: ObjId) -> f32 {
        self.facing
    }
    fn motion(&self, o: ObjId) -> u16 {
        if o == SORA {
            self.motion
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
        // (The Keyblade held upright, facing ahead: object axes x, y down, z back.)
        let turn = if o == WEAPON { Mat4::from_mat3(Mat3::from_diagonal(Vec3::new(1.0, -1.0, -1.0))) } else { Mat4::IDENTITY };
        Some(Mat4::from_translation(self.pos(o)) * turn)
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
    fn hitback(&mut self, o: ObjId, dir: Vec3, dist: f32) {
        if o == SORA {
            self.hitbacks.push((dir, dist));
        }
    }
    fn random(&mut self) -> f32 {
        // (A fixed sequence, so the runs repeat; a constant would aim Infinity's shots' wobble
        // the same way every frame.)
        self.rng = self.rng.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (self.rng >> 8) as f32 / (1u32 << 24) as f32
    }
    fn motion_start(&mut self, o: ObjId, m: u16, _blend: f32) {
        if o == SORA {
            self.motion = m;
            self.frame = 0;
            self.queued.clear();
            self.started.push(m);
        }
    }
    fn motion_push(&mut self, o: ObjId, m: u16, _blend: f32) {
        if o == SORA {
            self.queued.push(m);
        }
    }
    fn motion_ended(&self, _: ObjId) -> bool {
        self.frame + 1 >= self.len() && self.queued.is_empty()
    }
    fn motion_range(&self, _: ObjId, n: u8) -> bool {
        let f = self.frame;
        self.motions.get(&self.motion).is_some_and(|m| m.ranges.iter().any(|&(id, s, e, _)| id == 15 + n && f >= s && e.map_or(true, |e| f < e)))
    }
    fn face(&mut self, _: ObjId, dir: Vec3) {
        self.facing = dir.x.atan2(dir.z);
    }
    fn entry_id(&self, _: ObjId) -> u32 {
        LIMIT_FORM_ENTRY
    }
    fn offer_command(&mut self, _: ObjId, command: u16) {
        self.offers.push(command);
    }
    fn motion_trigger(&self, o: ObjId, n: u8) -> bool {
        o == SORA && !self.frame_triggers(8 + n).is_empty()
    }
    fn party_attack(&mut self, o: ObjId, row: u16) {
        self.played_rows.push(row);
        if let (SORA, Some(&m)) = (o, self.rows.get(&row)) {
            self.motion = m;
            self.frame = 0;
            self.queued.clear();
        }
    }
    fn weapon(&self, o: ObjId, n: u32) -> Option<ObjId> {
        (o == SORA && n == 0).then_some(WEAPON)
    }
    fn foes(&self) -> Vec<ObjId> {
        vec![FOE, FOE2]
    }
}

fn dir() -> PathBuf {
    std::env::var_os("KH2_RE_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../local/re"))
}

/// Sonic Rave (03system `cmd` 698): K400 (211) into K401 (212); Rave (699) offered in 212's
/// script range 1, each press turning him round into 212 again; after five, Finish (700) in its
/// range 2 instead, into K402 (213); then the limit ends with the motion.
#[test]
fn sonic_rave_runs_with_its_follow_ups() {
    let d = dir();
    let (Ok(mdlx), Ok(mset)) = (std::fs::read(d.join("P_EX100_KH1F.mdlx")), std::fs::read(d.join("P_EX100_KH1F.mset"))) else {
        eprintln!("skipped: no Limit Form files in {}", d.display());
        return;
    };
    let script = SpellFile::bar_entry(&mdlx, 3).expect("limi");
    let mut magic = Magic::new();
    let file = magic.add_spell(SpellFile::script_only("P_EX100_KH1F", &script).unwrap());
    let mset = Mset::parse(&mset).unwrap();
    let mut w = Fake::new(&mset);
    magic.limit(file, SORA, 698, &mut w).unwrap();
    let mut seen: Vec<u16> = Vec::new();
    let mut raves = 0;
    let mut finish = false;
    let mut offered = Vec::new();
    let mut flips = 0;
    let mut taken_at = 0;
    for f in 0..2000 {
        w.offers.clear();
        let facing = w.facing;
        for e in magic.update(&mut w) {
            match e {
                Event::Error(s) => panic!("frame {f}: {s}"),
                Event::Missing(n) => eprintln!("missing {n}"),
                Event::Strike { .. } => {}
            }
        }
        if (w.facing - facing).abs() > 3.0 {
            flips += 1;
        }
        for &c in &w.offers {
            offered.push((w.motion, w.frame, c));
        }
        // Each follow-up taken as soon as it is offered, once per motion start (the host's
        // rule: one press per window; the script starts the next motion on the next frame).
        if let Some(&c) = w.offers.first().filter(|_| w.started.len() > taken_at) {
            taken_at = w.started.len();
            assert!(magic.limit_command(SORA, c));
            if c == 699 {
                raves += 1;
            } else if c == 700 {
                finish = true;
            }
        }
        if seen.last() != Some(&w.motion) {
            seen.push(w.motion);
        }
        if !magic.limit_running(SORA) {
            eprintln!("Sonic Rave over at frame {f}: motions {seen:?}, started {:?}, {raves} Raves, Finish {finish}, flips {flips}", w.started);
            break;
        }
        w.advance();
    }
    assert!(!magic.limit_running(SORA), "the limit never ended: {seen:?}");
    assert_eq!(&w.started[..2], &[211, 212], "{:?}", w.started);
    assert_eq!(raves, 5, "offers {offered:?}");
    assert!(finish, "no Finish: offers {offered:?}");
    assert!(w.started.contains(&213));
    // The Raves in 212's range 1 (16..46), the Finish in its range 2 (the same frames).
    assert!(offered.iter().all(|&(m, f, _)| m == 212 && (16..46).contains(&f)), "{offered:?}");
    assert!(offered.iter().any(|&(_, f, _)| f == 16), "offered from the window's first frame: {offered:?}");
    assert_eq!(flips, 6, "each Rave and the Finish turn him round");
    assert_eq!(w.motion, 0);
}

/// Last Arcanum (`cmd` 701): K410 (221); Bash (702) offered by the motions themselves (range
/// 20), each press the next of K411..K415 (222..226); The End (703) from 226's, into K416 (227).
#[test]
fn last_arcanum_runs_its_combo() {
    let d = dir();
    let (Ok(mdlx), Ok(mset)) = (std::fs::read(d.join("P_EX100_KH1F.mdlx")), std::fs::read(d.join("P_EX100_KH1F.mset"))) else {
        eprintln!("skipped: no Limit Form files in {}", d.display());
        return;
    };
    let script = SpellFile::bar_entry(&mdlx, 3).expect("limi");
    let mut magic = Magic::new();
    let file = magic.add_spell(SpellFile::script_only("P_EX100_KH1F", &script).unwrap());
    let mut w = Fake::new(&Mset::parse(&mset).unwrap());
    magic.limit(file, SORA, 701, &mut w).unwrap();
    let mut taken = Vec::new();
    let mut taken_at = 0;
    for f in 0..3000 {
        w.offers.clear();
        for e in magic.update(&mut w) {
            if let Event::Error(s) = e {
                panic!("frame {f}: {s}");
            }
        }
        let offer = w.offers.first().copied().or_else(|| w.motion_offer());
        if let Some(c) = offer.filter(|_| w.started.len() > taken_at) {
            taken_at = w.started.len();
            assert!(magic.limit_command(SORA, c));
            taken.push(c);
        }
        if !magic.limit_running(SORA) {
            break;
        }
        w.advance();
    }
    eprintln!("Last Arcanum: started {:?}, took {taken:?}", w.started);
    assert!(!magic.limit_running(SORA));
    assert_eq!(w.started, vec![221, 222, 223, 224, 225, 226, 227]);
    assert_eq!(taken, vec![702, 702, 702, 702, 702, 703]);
}

/// The limit's script with the form's own effects when they are extracted (Infinity's shots fly
/// while their effect plays), else alone.
fn limit_script(d: &std::path::Path, mdlx: &[u8]) -> (SpellFile, bool) {
    let script = SpellFile::bar_entry(mdlx, 3).expect("limi");
    match std::fs::read(d.join("P_EX100_KH1F.a.us")).ok().and_then(|a| SpellFile::bar_entry(&a, 18)) {
        Some(pax) => (SpellFile::from_parts("P_EX100_KH1F", &script, &pax).unwrap(), true),
        None => (SpellFile::script_only("P_EX100_KH1F", &script).unwrap(), false),
    }
}

/// Strike Raid (`cmd` 704): K420 (231), then the first throw (232) on its own; each throw's frame
/// trigger 9 sends the Keyblade off on its own (a loose object of `Magic`'s: detached, spinning
/// on its motion 235, flying at the target as ATKP 1866 radius 160), the catch (234) puts it back
/// in his hand. Raid (705) is offered in 234's script range 1 while the count lasts (three), then
/// Judgment (706) in its range 2, whose throw (233) is ATKP 1867 radius 190.
#[test]
fn strike_raid_throws_the_keyblade() {
    let d = dir();
    let (Ok(mdlx), Ok(mset)) = (std::fs::read(d.join("P_EX100_KH1F.mdlx")), std::fs::read(d.join("P_EX100_KH1F.mset"))) else {
        eprintln!("skipped: no Limit Form files in {}", d.display());
        return;
    };
    let mut magic = Magic::new();
    let file = magic.add_spell(limit_script(&d, &mdlx).0);
    let mut w = Fake::new(&Mset::parse(&mset).unwrap());
    magic.limit(file, SORA, 704, &mut w).unwrap();
    let mut taken = Vec::new();
    let mut taken_at = 0;
    // Per throw: the attack's row and radius, the Keyblade's farthest from Sora, the attack's
    // nearest to the enemy's aim point, and the Keyblade's own motion.
    let mut throws: Vec<(u16, f32, f32, f32, u16)> = Vec::new();
    let mut loose_before = false;
    for f in 0..4000 {
        w.offers.clear();
        for e in magic.update(&mut w) {
            match e {
                Event::Error(s) => panic!("frame {f}: {s}"),
                Event::Missing(n) => eprintln!("missing {n}"),
                Event::Strike { .. } => {}
            }
        }
        let loose = magic.loose.get(&WEAPON).copied();
        if let Some(l) = loose {
            if !loose_before {
                throws.push((0, 0.0, 0.0, f32::MAX, l.motion));
            }
            let t = throws.last_mut().unwrap();
            if let Some(a) = magic.attacks.iter().find(|a| !a.freed && (a.atk == 1866 || a.atk == 1867)) {
                (t.0, t.1) = (a.atk, a.radius);
                if let Some(p) = a.pos {
                    t.3 = t.3.min(p.distance(w.target_point(FOE)));
                }
            }
            t.2 = t.2.max(l.pos.length());
            t.4 = l.motion;
            assert_eq!(l.holder, SORA);
            assert!(l.pos.is_finite() && l.rot.is_finite(), "frame {f}: {l:?}");
        }
        loose_before = loose.is_some();
        if let Some(&c) = w.offers.first().filter(|_| w.started.len() > taken_at) {
            taken_at = w.started.len();
            assert!(magic.limit_command(SORA, c));
            taken.push(c);
        }
        if !magic.limit_running(SORA) {
            break;
        }
        w.advance();
    }
    eprintln!("Strike Raid: started {:?}, took {taken:?}, throws (row, radius, farthest, nearest the enemy, motion) {throws:?}", w.started);
    assert!(!magic.limit_running(SORA));
    assert!(magic.loose.is_empty(), "the Keyblade is back in his hand");
    assert_eq!(w.started, vec![231, 232, 234, 232, 234, 232, 234, 232, 234, 233, 234]);
    assert_eq!(taken, vec![705, 705, 705, 706]);
    assert_eq!(throws.len(), 5, "{throws:?}");
    for (i, t) in throws.iter().enumerate() {
        let (atk, radius) = if i == 4 { (1867, 190.0) } else { (1866, 160.0) };
        assert_eq!((t.0, t.1), (atk, radius), "throw {i}");
        assert_eq!(t.4, 235, "the Keyblade spins on its own motion");
        assert!(t.2 > 300.0, "throw {i} flies off ({t:?})");
        assert!(t.3 < radius + BODY, "throw {i} reaches the enemy ({t:?})");
    }
}

/// Infinity (`cmd` 683, KH2 FM's Ragnarok): K430 (241), then combo rows 52..56 by
/// `trap_obj_party_attack` (242..246); 246 offers Shoot (684) by its range 20; Shoot pushes him
/// back 50 and plays 247, whose frame trigger 7 [100, 0] (the script's trigger 17) fires ten
/// shots (ATKP 1862, radius 80) from the Keyblade, three at the lock-on target and the rest at
/// enemies the script searches for; each ends when it hits.
#[test]
fn infinity_runs_its_combo_and_shots() {
    let d = dir();
    let (Ok(mdlx), Ok(mset), Ok(battle)) = (std::fs::read(d.join("P_EX100_KH1F.mdlx")), std::fs::read(d.join("P_EX100_KH1F.mset")), std::fs::read(d.join("00battle.bin"))) else {
        eprintln!("skipped: no Limit Form files in {}", d.display());
        return;
    };
    let mut magic = Magic::new();
    let (file, effects) = limit_script(&d, &mdlx);
    let file = magic.add_spell(file);
    let mut w = Fake::new(&Mset::parse(&mset).unwrap());
    let battle = kh2_formats::battle::Battle::parse(&battle).unwrap();
    w.rows = battle.ptya.set(0x42).expect("Limit Form's combo rows").iter().map(|r| (r.id as u16, r.motion)).collect();
    magic.limit(file, SORA, 683, &mut w).unwrap();
    let mut taken = Vec::new();
    let mut taken_at = 0;
    let mut seen: Vec<u16> = Vec::new();
    let mut shots_fired = 0;
    let mut most_shots = 0;
    let mut hits: Vec<ObjId> = Vec::new();
    let mut hit_keys: Vec<u64> = Vec::new();
    for f in 0..3000 {
        w.offers.clear();
        for e in magic.update(&mut w) {
            match e {
                Event::Error(s) => panic!("frame {f}: {s}"),
                Event::Missing(n) => eprintln!("missing {n}"),
                Event::Strike { .. } => {}
            }
        }
        // The shots' frame trigger (the host passes frame trigger 7 on as the script's 17).
        for params in w.frame_triggers(7) {
            let p: Vec<i32> = params.iter().map(|x| *x as i32).collect();
            assert!(magic.limit_motion_trigger(SORA, 17, &p));
            shots_fired += 1;
        }
        // The fake's hits: a shot on an enemy's body (`BODY` round its aim point).
        let live: Vec<(u64, Vec3, f32)> = magic.attacks.iter().filter(|a| !a.freed && a.atk == 1862).filter_map(|a| Some((a.key, a.pos?, a.radius))).collect();
        most_shots = most_shots.max(live.len());
        for (key, pos, radius) in live {
            for foe in [FOE, FOE2] {
                if !hit_keys.contains(&key) && pos.distance(w.target_point(foe)) < radius + BODY {
                    magic.attack_mut(key).unwrap().hit = true;
                    hit_keys.push(key);
                    hits.push(foe);
                }
            }
        }
        let offer = w.offers.first().copied().filter(|c| *c != 0).or_else(|| w.motion_offer());
        if let Some(c) = offer.filter(|_| w.started.len() > taken_at) {
            taken_at = w.started.len();
            assert!(magic.limit_command(SORA, c));
            taken.push(c);
        }
        if seen.last() != Some(&w.motion) {
            seen.push(w.motion);
        }
        if !magic.limit_running(SORA) {
            break;
        }
        w.advance();
    }
    eprintln!("Infinity: motions {seen:?}, started {:?}, rows {:?}, took {taken:?}, pushed {:?}, shots {most_shots} at once, hits {hits:?}", w.started, w.played_rows, w.hitbacks);
    assert!(!magic.limit_running(SORA));
    assert_eq!(w.played_rows, vec![52, 53, 54, 55, 56]);
    assert_eq!(&seen[1..8], &[241, 242, 243, 244, 245, 246, 247], "{seen:?}");
    assert_eq!(taken, vec![684]);
    assert_eq!(shots_fired, 1);
    assert_eq!(w.hitbacks.len(), 1);
    let (dir, dist) = w.hitbacks[0];
    assert_eq!(dist, 50.0);
    assert!((dir - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-4, "pushed back, away from his facing: {dir}");
    if !effects {
        eprintln!("(no P_EX100_KH1F.a.us: the shots' flight not checked)");
        return;
    }
    assert_eq!(most_shots, 10, "Shoot's ten shots");
    // (They home in spiralling: KH2's own aim, a random share of the way off to the side each
    // frame; most hit, the rest run out after 150..180 frames.)
    assert!(hits.len() >= 6, "most shots hit: {hits:?}");
    assert!(hits.contains(&FOE) && hits.contains(&FOE2), "the lock-on target and the one the others found: {hits:?}");
    assert!(magic.attacks.iter().all(|a| a.freed || a.atk != 1862), "every shot over");
}
