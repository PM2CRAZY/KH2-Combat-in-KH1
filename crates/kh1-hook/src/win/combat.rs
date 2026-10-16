//! Stage 10: KH2's combo on KH1's enemies.
//!
//! From Ghidra (the user's Epic exe) and the stage 9 probe:
//! - The Attack command (`0x1402A5380`, command type 1 in `0x142D54440`) picks a row of the
//!   actor's attack table and runs it with `0x1402A4760` `(actor, row, dir)`; `0x1402A17F0`
//!   reads the row (`0x142D5D4B0[object id] + row * 0x14`: the motion request at +0, the attack
//!   data at `0x142D23740 + [+4]`, sound/effect bytes at +0xC..+0xE), sets the motion and makes
//!   the attack object (`0x1402B10E0` `(actor, hit bit from 0x1402B15D0, data)`).
//! - Each frame `0x1402B22D0` tests the live attack objects against every hittable actor's
//!   body parts (`0x14028FEE0` says which actors; a part's shape from `0x1402B3CB0`: type,
//!   radius, half height, centre), and a contact runs `0x1402B1ED0` `(attack entry, body
//!   entry)`: the hit record through `0x1402BD180` (damage `0x1402BD8D0`), its push direction,
//!   the hit effect and the Keyblade's hit sound. `0x1402B13D0` frees an attack object.
//!
//! While KH2 has Sora, a regular Attack command is not run: it is KH2's attack button (`press`).
//! KH2's own hit detection (its swept attack shapes and hit records, the way the playground does
//! it) runs against KH1's body parts, and each hit KH2 lets through is landed with
//! KH1's own contact code, through an attack object made from Sora's own row (ground or air,
//! swing or finisher, as KH2's attack is), so HP, the enemy's reaction, drops, EXP, effects and
//! sounds are KH1's. The damage is scaled by the KH2 attack's power (`POWER`, applied to the
//! hit record in `probe::hit`).

use super::{hook, log, pose, takeover};
use character_core::{Character, Output, TargetInfo};
use combat_core::{first_overlap, AttackVolume, HitRecord, Shape};
use kh2_formats::battle::Battle;
use kh2_rig::RigAnim;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

const EXEC_RVA: usize = 0x2A4760;
const EXEC_PROLOGUE: [u8; 11] = [0x4C, 0x8B, 0xDC, 0x57, 0x48, 0x81, 0xEC, 0x90, 0x00, 0x00, 0x00];
const COMMAND_RVA: usize = 0x2D54440;
const PARTY_RVA: usize = 0x2D37C80;
const ROWS_RVA: usize = 0x2D5D4B0;
const ATTACK_DATA_RVA: usize = 0x2D23740;
const NEXT_ACTOR_RVA: usize = 0x28F990;
const HITTABLE_RVA: usize = 0x28FEE0;
const PART_SHAPE_RVA: usize = 0x2B3CB0;
const NEW_ATTACK_RVA: usize = 0x2B10E0;
const FREE_BIT_RVA: usize = 0x2B15D0;
const LAND_RVA: usize = 0x2B1ED0;
const FREE_ATTACK_RVA: usize = 0x2B13D0;
/// Enemies farther than this from Sora are not looked at.
const NEAR: f32 = 1500.0;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_EXEC: AtomicUsize = AtomicUsize::new(0);
static ORIG_COMMAND: AtomicUsize = AtomicUsize::new(0);
/// KH1's command step `(actor, dir)`: runs the command at `0x142D54440` (type byte first).
const COMMAND_STEP_RVA: usize = 0x2A5380;
const COMMAND_STEP_PROLOGUE: [u8; 11] = [0x40, 0x55, 0x56, 0x57, 0x48, 0x81, 0xEC, 0x60, 0x01, 0x00, 0x00];
const STATE_RVA: usize = 0x2AF7D0;
/// The Attack command was given since KH2's last frame.
static PRESSED: AtomicBool = AtomicBool::new(false);
/// While KH2's hit is landed: its power in percent (0 otherwise).
pub static POWER: AtomicU32 = AtomicU32::new(0);
static LOGGED: AtomicU32 = AtomicU32::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn note(s: String) {
    if LOGGED.fetch_add(1, Ordering::Relaxed) < 300 {
        log(&s);
    }
}

type ExecFn = unsafe extern "system" fn(usize, i32, usize);

unsafe extern "system" fn exec(actor: usize, row: i32, dir: usize) {
    let base = BASE.load(Ordering::Relaxed);
    let sora = rd::<u64>(base + PARTY_RVA) as usize;
    let command = rd::<u8>(base + COMMAND_RVA);
    if actor == sora && command == 1 && takeover::ON.load(Ordering::Relaxed) && takeover::kh2_has() {
        PRESSED.store(true, Ordering::Relaxed);
        note(format!("Attack command (KH1 row {row}, motion {}): KH2's attack button", row_info(actor, row).map_or(-1, |r| r.motion as i32)));
        return;
    }
    let f: ExecFn = std::mem::transmute(ORIG_EXEC.load(Ordering::Acquire));
    f(actor, row, dir)
}

/// Command types that don't move Sora (`0x1402A1470`): the party's (`+0x98`'s), the menu's.
const NOT_SORAS: [u8; 21] = [0x1D, 0x1E, 0x46, 0x51, 0x54, 0x64, 0x4B, 0x4D, 0x4E, 0x52, 0x57, 0x58, 0x59, 0x5A, 0x5B, 0x5E, 0x5F, 0x60, 0x4F, 0x5C, 0x5D];

/// Phase 8: KH1's own commands stay KH1's. KH1's command step (`0x1402A5380`, from its think
/// step `0x1402A4090`) runs a chosen command: Attack (type 1) becomes KH2's attack (`exec`);
/// any other that moves Sora (magic is type 2, spell in bits 8..23; items, specials,
/// summons...) is KH1's from here (`takeover::hand_to_kh1`). KH1 picks the command's ground
/// or air version by its movement state, which stays the ground's while KH2 has him: in KH2's
/// air, Sora is first put in KH1's jump state as falling from where he is (the fields KH1's
/// own ceiling bonk `0x1402AD260` sets: rise time `+0x3B8` 0, time `+0x3B0` 0, height `+0x3BC`
/// 0, base `+0x3B4` his height), so the air version plays and KH1 lands him.
unsafe extern "system" fn command(actor: usize, dir: usize) {
    let base = BASE.load(Ordering::Relaxed);
    let f: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_COMMAND.load(Ordering::Acquire));
    let sora = rd::<u64>(base + PARTY_RVA) as usize;
    let kind = rd::<u8>(base + COMMAND_RVA);
    if actor != sora || matches!(kind, 0 | 1) || NOT_SORAS.contains(&kind) || !super::settings::get().commands || !takeover::ON.load(Ordering::Relaxed) || !takeover::kh2_has() {
        return f(actor, dir);
    }
    trail_off(actor);
    let air = takeover::kh2_airborne();
    let before = (rd::<i32>(actor + 0x70), rd::<u32>(actor + 0x164));
    if air {
        let set_state: unsafe extern "system" fn(usize, u32) -> u32 = std::mem::transmute(base + STATE_RVA);
        let y = rd::<f32>(actor + 0x14);
        set_state(actor, 2);
        wr(actor + 0x3B0, 0.0f32);
        wr(actor + 0x3B8, 0.0f32);
        wr(actor + 0x3BC, 0.0f32);
        wr(actor + 0x3B4, y);
        wr(actor + 0x3C0, [0.0f32; 4]);
        wr(actor + 0x3D0, 0u32);
        wr(actor, rd::<u32>(actor) & !0x1_0000);
    }
    // What KH1 doing something with him changes: his state, motion request, the script-held
    // flag 0x1000000 (spells and items set it, the user's log), his motion slot.
    // ... and above all any motion KH1 sets on him (`probe`'s set-motion hooks count them: the
    // air cast's `0x14029D7F0(actor, 0x83 + spell)` changed none of the others, the user's log).
    let look = || (rd::<i32>(actor + 0x70), rd::<u32>(actor + 0x164), rd::<u32>(actor) & 0x100_0000, pose::sora_slot(), super::probe::SORA_MOTIONS.load(Ordering::Relaxed));
    let set = look();
    let full = rd::<u32>(base + COMMAND_RVA);
    f(actor, dir);
    // Only if KH1 did something with him (a new state or motion request); else as before.
    let after = look();
    if after != set {
        takeover::hand_to_kh1(&format!("command {kind:#x} ({full:08x}){}, KH1 motion {:#x}", if air { " in the air" } else { "" }, super::probe::SORA_LAST_SLOT.load(Ordering::Relaxed)));
    } else {
        if air {
            wr(actor + 0x70, before.0);
        }
        note(format!("command {kind:#x} ({full:08x}){}: KH1 did nothing with Sora, KH2 keeps him", if air { " in the air" } else { "" }));
    }
}

/// The Attack command since the last call (KH2's attack button press).
pub fn take_press() -> bool {
    PRESSED.swap(false, Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug)]
struct Row {
    motion: i16,
    data: i32,
    /// Bytes +0xC..+0xE (the attack object's +0xA8..+0xAA: effect and sound).
    fx: [u8; 3],
    /// What the row's start plays (`0x1402A17F0`): +2 the weapon trail (`0x1402C0160`), +3 an
    /// effect on Sora (`0x1402ABB60`), +0xF a voice (`0x14028E060`); -1 none.
    trail: i8,
    effect: i8,
    voice: i8,
}

unsafe fn row_info(actor: usize, row: i32) -> Option<Row> {
    let base = BASE.load(Ordering::Relaxed);
    let obj = pose::ptr(rd::<u32>(actor + 0x130));
    if obj < 0x10000 || !(0..0x80).contains(&row) {
        return None;
    }
    let table = rd::<u64>(base + ROWS_RVA + rd::<u16>(obj + 0x4C) as usize * 8) as usize;
    if table < 0x10000 {
        return None;
    }
    let r = table + row as usize * 0x14;
    Some(Row { motion: rd(r), data: rd(r + 4), fx: rd(r + 0xC), trail: rd(r + 2), effect: rd(r + 3), voice: rd(r + 0xF) })
}

/// Sora's own rows to land KH2's hits with: [ground swing, ground finisher, air swing, air
/// finisher]: KH1's combo motions 200 / 202 on the ground, 204 / 205 in the air (the Attack
/// command's own picks in the user's log; 203's first row carries a much stronger attack).
unsafe fn sora_rows(sora: usize) -> Option<[Row; 4]> {
    let mut rows: Vec<Row> = Vec::new();
    let mut all = Vec::new();
    for i in 0..0x40 {
        let Some(r) = row_info(sora, i) else { break };
        if r.motion >= 0 {
            all.push(format!("{i}:{}/{:#x}/{:02x?}", r.motion, r.data, r.fx));
            rows.push(r);
        }
    }
    log(&format!("Sora's KH1 attack rows (row:motion/data/fx): {}", all.join(" ")));
    // The attack data the rows use (first words: strength source, element, reaction, ...).
    let base = BASE.load(Ordering::Relaxed);
    let mut seen = HashSet::new();
    let datas: Vec<String> = rows
        .iter()
        .filter(|r| seen.insert(r.data))
        .map(|r| format!("{:#x}: [{}]", r.data, (0..12).map(|k| format!("{:x}", rd::<u32>(base + ATTACK_DATA_RVA + r.data as usize + k * 4))).collect::<Vec<_>>().join(" ")))
        .collect();
    log(&format!("their attack data: {}", datas.join(" | ")));
    let first = |m: i16| rows.iter().find(|r| r.motion == m).copied();
    // The attack data's third word is KH1's reaction kind (into the attack's `+0x30` low bits;
    // 3 a flinch, 5 the finishers' knock-back, the user's stage 26 log). KH1's combo finishers
    // are motions 203 (ground) and 206 (air), both with a kind-5 data (0x4C58, +4 strength);
    // a finisher lands with the first kind-5 row of that motion.
    let kind = |r: &Row| rd::<u32>(base + ATTACK_DATA_RVA + r.data as usize + 8);
    let finisher = |m: i16| rows.iter().find(|r| r.motion == m && kind(r) == 5).copied().or_else(|| rows.iter().find(|r| (200..=206).contains(&r.motion) && kind(r) == 5).copied());
    let ground = first(200).or_else(|| first(201))?;
    let air = first(204).or(Some(ground))?;
    Some([ground, finisher(203).unwrap_or(ground), air, finisher(206).unwrap_or(air)])
}

/// One enemy's body part: KH1's entry for its contact code and the shape in KH2 space.
struct Part {
    entry: [u8; 0x40],
    shape: Shape,
}

struct Body {
    actor: usize,
    parts: Vec<Part>,
    target: TargetInfo,
}

/// KH1's hittable actors on other teams than Sora's, near him.
unsafe fn bodies(sora: usize) -> Vec<Body> {
    let base = BASE.load(Ordering::Relaxed);
    let next: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(base + NEXT_ACTOR_RVA);
    let hittable: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(base + HITTABLE_RVA);
    let part_shape: unsafe extern "system" fn(usize, usize, *mut u32) = std::mem::transmute(base + PART_SHAPE_RVA);
    let team = |a: usize| (rd::<u32>(a + 0x374) >> 5) & 3;
    let ours = team(sora);
    let at = rd::<[f32; 3]>(sora + 0x10);
    let mut out = Vec::new();
    let mut a = next(0);
    let mut n = 0;
    while a != 0 && n < 256 {
        n += 1;
        let this = a;
        a = next(a);
        if this == sora || team(this) == ours || hittable(this) & 0xFF == 0 {
            continue;
        }
        let p = rd::<[f32; 3]>(this + 0x10);
        let (dx, dz) = (p[0] - at[0], p[2] - at[2]);
        if dx * dx + dz * dz > NEAR * NEAR {
            continue;
        }
        let col = pose::ptr(rd::<u32>(this + 0x134));
        if col < 0x10000 {
            continue;
        }
        let count = rd::<u8>(col + 6) as usize;
        let list = pose::ptr(rd::<u32>(col + 0x18));
        if list < 0x10000 {
            continue;
        }
        let off = rd::<u32>(this + 0x3A8);
        let feet = [p[0], -p[1], p[2]];
        let (mut radius, mut top) = (0.0f32, feet[1]);
        let mut parts = Vec::new();
        for i in 0..count.min(32) {
            if off & (1 << i) != 0 {
                continue;
            }
            let part = list + i * 0x14;
            let mut entry = [0u8; 0x40];
            let e = entry.as_mut_ptr() as usize;
            wr(e, team(this));
            wr(e + 8, this as u64);
            wr(e + 0x10, rd::<u8>(part) as u32);
            wr(e + 0x20, 1u32);
            part_shape(this, part, (e + 0x20) as *mut u32);
            let kind = rd::<u32>(e + 0x20);
            let (r, h) = (rd::<f32>(e + 0x24), rd::<f32>(e + 0x28));
            let c = rd::<[f32; 3]>(e + 0x30);
            if !(r > 0.0 && r < 2000.0 && h >= 0.0 && h < 2000.0 && c.iter().all(|v| v.is_finite())) {
                continue;
            }
            let centre = [c[0], -c[1], c[2]];
            let shape = if kind == 1 { Shape::sphere(centre, r) } else { Shape::column(centre, r, h.max(r * 0.25)) };
            radius = radius.max(r);
            top = top.max(centre[1] + h);
            parts.push(Part { entry, shape });
        }
        if parts.is_empty() {
            continue;
        }
        let target = TargetInfo { id: this as u32, pos: feet, radius, height: (top - feet[1]).max(radius) };
        out.push(Body { actor: this, parts, target });
    }
    out
}

/// KH2's hit detection and hit records for Sora's swings.
#[derive(Default)]
pub struct Strikes {
    volumes: HashMap<(u64, u8), AttackVolume>,
    records: HashMap<usize, HitRecord>,
    live: HashSet<u64>,
    rows: Option<[Row; 4]>,
    rows_tried: bool,
    bodies: Vec<Body>,
    guards: Vec<Guard>,
    /// Swings a guard repelled: they hit nothing more.
    repelled: HashSet<u64>,
    /// KH1's swing rows by motion (200..206), for `swing_start`, and the model they are for.
    swing_rows: Option<(u16, HashMap<i16, Row>)>,
    /// Frames since a trail was asked for.
    trail_asked: Option<u32>,
    /// Enemies a Magnet (ATKP reaction 5: Magnet Burst) caught: the swing, orbit radius and
    /// swirl. Pulled every frame while the swing lasts.
    pulled: HashMap<usize, (u64, f32, f32)>,
    /// (swing, shot) pairs already met (once a swing).
    met_shots: HashSet<(u64, usize)>,
    shots: Vec<Shot>,
}

/// An enemy's shot Sora's swings can knock back (Wakka's ball): a free-standing KH1 attack
/// object of the attack pool (attribute `+0x30` bit `0x400`), `docs/kh1-reflect.md`.
struct Shot {
    obj: usize,
    /// Its swept path (previous and current centre) and radius, KH2 y-up.
    shapes: Vec<Shape>,
    owner: usize,
}

/// The enemies' shots near Sora that a swing may meet.
unsafe fn shots(sora: usize) -> Vec<Shot> {
    let base = BASE.load(Ordering::Relaxed);
    let team = |a: usize| (rd::<u32>(a + 0x374) >> 5) & 3;
    let ours = team(sora);
    let at = rd::<[f32; 3]>(sora + 0x10);
    let mut out = Vec::new();
    for k in 0..ATTACK_SLOTS {
        let g = base + ATTACK_POOL_RVA + k * 0xC0;
        let flags = rd::<u32>(g);
        // In use and active; not switched off, freed, or already turned back.
        if flags & 0x800 == 0 || flags & 0xC == 0 || flags & (0x400 | 0x0020_0000 | 0x0100_0000) != 0 {
            continue;
        }
        // (Every live enemy attack: KH1's collision pass meets each pair of attacks, and the
        // meeting itself decides what happens: a shot turned back, a clash, nothing. Guards,
        // priority 100, have their own path.)
        let prio = rd::<i32>(g + 0x68);
        if prio == GUARD_PRIORITY || prio < 0 {
            continue;
        }
        // KH1's pass (`0x1402B1A50`) leaves out attacks with word bit 0x400, and those with
        // bit 0x80 that already met something (`+4`).
        if flags & 0x400 != 0 || (flags & 0x80 != 0 && rd::<u32>(g + 4) != 0) {
            continue;
        }
        let owner = pose::ptr(rd::<u32>(g + 0x34));
        if owner == sora || (owner > 0x10000 && team(owner) == ours) {
            continue;
        }
        // Its carrier (`+0x90`) switched off (`+0x88` bit 1): only some types stay (`0x1402B2610`).
        let t = flags & 0xE0000;
        let carrier = pose::ptr(rd::<u32>(g + 0x90));
        if carrier > 0x10000 && rd::<u8>(carrier + 0x88) & 1 != 0 && rd::<u32>(g + 0x30) & 0x400 == 0 && (((t.wrapping_sub(0x20000)) & 0xfff9_ffff) != 0 || t == 0x60000) {
            continue;
        }
        // Tied to a motion (activity 2): live only while the owner plays it and within its hit
        // window (`0x1402B2610`: owner `+0x1CC` bit `0x10000 << group`), as KH1's pass has it
        // (the user: endless clinks against the enemies' resting attack shapes).
        if (flags >> 2) & 3 == 2 {
            if owner < 0x10000 {
                continue;
            }
            let req = rd::<u32>(g + 0x58);
            let window = 0x10000u32 << ((flags >> 4) & 3);
            if req == 0xFFF || req != rd::<u32>(owner + 0x164) || rd::<u32>(owner + 0x160) & 0x100 != 0 || rd::<u32>(owner + 0x1CC) & window == 0 {
                continue;
            }
        }
        let c = rd::<[f32; 3]>(g + 0x10);
        let prev = rd::<[f32; 3]>(g + 0x70);
        let r = rd::<f32>(g + 0xC);
        if !(r > 0.0 && r < 500.0) || !c.iter().chain(prev.iter()).all(|v| v.is_finite()) {
            continue;
        }
        let (dx, dz) = (c[0] - at[0], c[2] - at[2]);
        if dx * dx + dz * dz > NEAR * NEAR {
            continue;
        }
        let up = |p: [f32; 3]| [p[0], -p[1], p[2]];
        let mid = [(c[0] + prev[0]) * 0.5, (c[1] + prev[1]) * 0.5, (c[2] + prev[2]) * 0.5];
        let shapes = vec![Shape::sphere(up(c), r), Shape::sphere(up(mid), r), Shape::sphere(up(prev), r)];
        // What it is, once a slot and kind (to tell shots apart in the log).
        static SEEN: std::sync::Mutex<Vec<(usize, u32, u32)>> = std::sync::Mutex::new(Vec::new());
        if let Ok(mut seen) = SEEN.lock() {
            let key = (g, flags & 0xFFFF_F000, rd::<u32>(g + 0x30));
            if seen.len() < 200 && !seen.contains(&key) {
                seen.push(key);
                log(&format!(
                    "enemy attack near Sora: slot {k} flags {flags:08x} +4 {:08x} +0x30 {:08x} owner {owner:#x} priority {prio} radius {r:.0} at {c:.0?} (was {prev:.0?})",
                    rd::<u32>(g + 4),
                    rd::<u32>(g + 0x30)
                ));
            }
        }
        out.push(Shot { obj: g, shapes, owner });
    }
    out
}

/// A swing meets a shot as KH1's own swing would (`0x1402B2890`, the attack-vs-attack clash,
/// both ways): KH1 turns the shot back (team swapped, Sora its owner, flag `0x1000000`).
/// Returns whether it was turned back.
unsafe fn meet_shot(sora: usize, shot: &Shot, at: &Shape, row: Row) -> bool {
    let base = BASE.load(Ordering::Relaxed);
    let free_bit: unsafe extern "system" fn() -> u32 = std::mem::transmute(base + FREE_BIT_RVA);
    let new_attack: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + NEW_ATTACK_RVA);
    let free: unsafe extern "system" fn(usize) = std::mem::transmute(base + FREE_ATTACK_RVA);
    let clash: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + CLASH_RVA);
    let obj = new_attack(sora, free_bit(), base + ATTACK_DATA_RVA + row.data as usize);
    if obj < 0x10000 {
        return false;
    }
    // KH1's pairing table (`0x1405299B8`, by type `flags >> 17 & 7`): either way round.
    let ty = |o: usize| (rd::<u32>(o) >> 17) & 7;
    let table = |a: u32, b: u32| rd::<u8>(base + PAIRS_RVA + a as usize) & (1u8 << b) != 0;
    let (ts, to) = (ty(obj), ty(shot.obj));
    if !table(ts, to) && !table(to, ts) {
        free(obj);
        return false;
    }
    let c = [at.center[0], -at.center[1], at.center[2], 1.0f32];
    wr(obj + 0x10, c);
    wr(obj + 0x40, c);
    wr(obj + 0x70, c);
    wr(obj + 0xC, at.radius);
    let entry = |o: usize, owner: usize, centre: [f32; 4], r: f32| {
        let mut e = [0u8; 0x40];
        let p = e.as_mut_ptr() as usize;
        wr(p, (rd::<u32>(o) >> 12) & 3);
        wr(p + 8, owner as u64);
        wr(p + 0x10, rd::<u32>(o + 0x54));
        wr(p + 0x18, o as u64);
        wr(p + 0x20, 1u32);
        wr(p + 0x24, r);
        wr(p + 0x28, r);
        wr(p + 0x30, centre);
        e
    };
    let sc = rd::<[f32; 3]>(shot.obj + 0x10);
    let swing = entry(obj, sora, c, at.radius);
    let them = entry(shot.obj, shot.owner, [sc[0], sc[1], sc[2], 1.0], rd::<f32>(shot.obj + 0xC));
    clash(swing.as_ptr() as usize, them.as_ptr() as usize);
    free(obj);
    rd::<u32>(shot.obj) & 0x0100_0000 != 0
}

/// Where the map's walls stop a blade moving `from` → `to` (KH1 world, y down): KH1's
/// background ray (`0x140191520`, the one its attacks use), and the hit face's normal
/// (`0x14018FC60`) to tell walls from floors and ceilings.
unsafe fn wall_between(from: [f32; 3], to: [f32; 3]) -> Option<[f32; 3]> {
    let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if !(0.5..400.0).contains(&len) || !from.iter().chain(to.iter()).all(|v| v.is_finite()) {
        return None;
    }
    let base = BASE.load(Ordering::Relaxed);
    let ray: unsafe extern "system" fn(*mut [u8; 0x40], u32) = std::mem::transmute(base + RAY_RVA);
    let normal: unsafe extern "system" fn(i32, i32, *mut [f32; 4]) = std::mem::transmute(base + FACE_NORMAL_RVA);
    let mut r = [0u8; 0x40];
    let p = r.as_mut_ptr() as usize;
    wr(p, [from[0], from[1], from[2], 1.0f32]);
    wr(p + 0x10, [d[0], d[1], d[2], 0.0f32]);
    ray(&mut r, 1);
    let face = rd::<i16>(p + 0x30) as i32;
    let owner = rd::<i32>(p + 0x34);
    // The map's own faces only (`-1`): objects (crates, barrels) take the swing as a hit.
    if face < 0 || owner != -1 {
        return None;
    }
    let mut n = [0.0f32; 4];
    normal(owner, face, &mut n);
    let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if !(nl > 1e-3 && nl.is_finite()) || (n[1] / nl).abs() > 0.6 {
        return None;
    }
    let at = rd::<[f32; 3]>(p + 0x20);
    at.iter().all(|v| v.is_finite()).then_some(at)
}

/// An enemy's guard: a KH1 attack object of priority 100 (`0x1402B2A30`: such an object
/// repels whatever attack meets it), e.g. the Large Body's belly, shields. Directional ones
/// (type bits `0xE0000` of 1 or 3) only guard an arc in front of their owner (`0x1402B24B0`).
struct Guard {
    shape: Shape,
    /// KH1 world centre.
    at: [f32; 3],
    owner: usize,
}

const ATTACK_POOL_RVA: usize = 0x2DB5110;
const ATTACK_SLOTS: usize = 0x40;
const GUARD_PRIORITY: i32 = 100;
const CLANG_RVA: usize = 0x2AC4C0;
/// KH1's attack-vs-attack meeting (`0x1402B2890 (entry, entry)`).
const CLASH_RVA: usize = 0x2B2890;
/// KH1's attack pairing table (8 bytes, a bit per type that may meet).
const PAIRS_RVA: usize = 0x5299B8;
/// KH1's attack-vs-background ray (`0x1402B1870 (attack, from, to)`): spark, sound, and a
/// clash record on the owner when it meets a wall.
#[allow(dead_code)]
const WALL_RVA: usize = 0x2B1870;
/// KH1's background ray (`0x140191520 (ray, mode)`): ray from `+0`, direction `+0x10`; out:
/// the point `+0x20`, the face `+0x30` (i16, -1 none), its object `+0x34` (-1 the map).
const RAY_RVA: usize = 0x191520;
/// A face's normal (`0x14018FC60 (object, face, out)`).
const FACE_NORMAL_RVA: usize = 0x18FC60;
const CLANG_SOURCE_RVA: usize = 0x2D67400;

/// The live enemy guards near Sora.
unsafe fn guards(sora: usize) -> Vec<Guard> {
    let base = BASE.load(Ordering::Relaxed);
    let hittable: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(base + HITTABLE_RVA);
    let team = |a: usize| (rd::<u32>(a + 0x374) >> 5) & 3;
    let ours = team(sora);
    let at = rd::<[f32; 3]>(sora + 0x10);
    let mut out = Vec::new();
    for k in 0..ATTACK_SLOTS {
        let g = base + ATTACK_POOL_RVA + k * 0xC0;
        let flags = rd::<u32>(g);
        // In use, active, not being freed or switched off.
        if flags & 0x800 == 0 || flags & 0xC == 0 || flags & 0x0120_0000 != 0 {
            continue;
        }
        if rd::<i32>(g + 0x68) != GUARD_PRIORITY {
            continue;
        }
        let owner = pose::ptr(rd::<u32>(g + 0x34));
        if owner < 0x10000 || owner == sora || team(owner) == ours || hittable(owner) & 0xFF == 0 {
            continue;
        }
        // Tied to a motion (activity type 2): only while the owner plays it.
        if (flags >> 2) & 3 == 2 {
            let req = rd::<u32>(g + 0x58);
            if req != 0xFFF && req != rd::<u32>(owner + 0x164) {
                continue;
            }
        }
        let c = rd::<[f32; 3]>(g + 0x10);
        let r = rd::<f32>(g + 0xC);
        if !(r > 0.0 && r < 1000.0) || !c.iter().all(|v| v.is_finite()) {
            continue;
        }
        let (dx, dz) = (c[0] - at[0], c[2] - at[2]);
        if dx * dx + dz * dz > NEAR * NEAR {
            continue;
        }
        // Directional: Sora must be within the arc (cosine at the data's `+0x64`) around the
        // owner's facing turned by the data's `+0x60`.
        if (flags & 0xE0000).wrapping_sub(0x20000) & 0xFFF9_FFFF == 0 {
            let data = pose::ptr(rd::<u32>(g + 0x5C));
            if data > 0x10000 {
                let f = rd::<[f32; 3]>(owner + 0x30);
                let turn = rd::<f32>(data + 0x60);
                let (s, co) = (turn.sin(), turn.cos());
                let fx = f[0] * co + f[2] * s;
                let fz = -f[0] * s + f[2] * co;
                let (o, me) = (rd::<[f32; 3]>(owner + 0x50), rd::<[f32; 3]>(sora + 0x50));
                let (dx, dz) = (me[0] - o[0], me[2] - o[2]);
                let (lf, ld) = ((fx * fx + fz * fz).sqrt(), (dx * dx + dz * dz).sqrt());
                if lf > 1e-4 && ld > 1e-4 && (fx * dx + fz * dz) / (lf * ld) < rd::<f32>(data + 0x64) {
                    continue;
                }
            }
        }
        static SEEN: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());
        if let Ok(mut v) = SEEN.try_lock() {
            if v.len() < 40 && !v.contains(&owner) {
                v.push(owner);
                let obj = pose::ptr(rd::<u32>(owner + 0x130));
                let id = if obj > 0x10000 { rd::<u16>(obj + 0x4C) } else { 0xFFFF };
                log(&format!("guard seen: actor {owner:#x} (object {id}), radius {r:.0}, flags {flags:08x}"));
            }
        }
        out.push(Guard { shape: Shape::sphere([c[0], -c[1], c[2]], r), at: c, owner });
    }
    out
}

/// KH1's clash spark and sound where a swing is repelled (`0x1402B2890`'s).
pub unsafe fn clang(at: [f32; 3]) {
    let base = BASE.load(Ordering::Relaxed);
    let source = rd::<u64>(base + CLANG_SOURCE_RVA) as usize;
    if source < 0x10000 {
        return;
    }
    let f: unsafe extern "system" fn(usize, u32, u32, *const [f32; 4]) -> usize = std::mem::transmute(base + CLANG_RVA);
    let p = [at[0], at[1], at[2], 1.0];
    f(source, 10, 20000, &p);
}

const TRAIL_RVA: usize = 0x2C0160;
const EFFECT_RVA: usize = 0x2ABB60;
const VOICE_RVA: usize = 0x28E060;
/// Actor `+0x1CC` bit: the weapon trail is drawn (`0x1402BFD70`; KH1's attack motions' triggers
/// set it over their swing).
const TRAIL_FLAG: u32 = 0x80_0000;
/// The trail flag is ours (set for a KH2 swing).
static TRAIL_ON: AtomicBool = AtomicBool::new(false);

/// Clears the trail flag if we set it (KH1 has Sora again, whatever the reason).
pub unsafe fn trail_off(sora: usize) {
    if TRAIL_ON.swap(false, Ordering::Relaxed) {
        wr(sora + 0x1CC, rd::<u32>(sora + 0x1CC) & !TRAIL_FLAG);
    }
}

impl Strikes {
    /// Phase 9: a KH2 swing starts with what KH1's matching combo row plays when its motion
    /// starts (`0x1402A17F0`): the Keyblade trail asked for, the swing's effect and Sora's
    /// voice. `count` is KH2's combo count (1 on the first swing).
    pub unsafe fn swing_start(&mut self, sora: usize, air: bool, count: u8, finisher: bool, kh1_voice: bool) {
        let obj = pose::ptr(rd::<u32>(sora + 0x130));
        let model = if obj > 0x10000 { rd::<u16>(obj + 0x4C) } else { u16::MAX };
        if self.swing_rows.as_ref().map(|r| r.0) != Some(model) {
            let mut m = HashMap::new();
            for i in 0..0x40 {
                let Some(r) = row_info(sora, i) else { break };
                if (200..=206).contains(&r.motion) {
                    m.entry(r.motion).or_insert(r);
                }
            }
            log(&format!("KH1 swing rows (motion: trail / effect / voice): {}", {
                let mut v: Vec<_> = m.values().map(|r| format!("{}: {} / {} / {}", r.motion, r.trail, r.effect, r.voice)).collect();
                v.sort();
                v.join(", ")
            }));
            if m.is_empty() {
                return;
            }
            self.swing_rows = Some((model, m));
        }
        let n = count.max(1) as i16 - 1;
        let motion = match (air, finisher) {
            (false, false) => 200 + n.min(2),
            (false, true) => 203,
            (true, false) => 204 + n.min(1),
            (true, true) => 206,
        };
        let Some(r) = self.swing_rows.as_ref().and_then(|m| m.1.get(&motion)).copied() else { return };
        let base = BASE.load(Ordering::Relaxed);
        let cs = pose::ptr(rd::<u32>(sora + 0x14C));
        let source = if cs > 0x10000 { rd::<u64>(cs + 0x18) as usize } else { 0 };
        if source > 0x10000 {
            if r.trail >= 0 {
                let f: unsafe extern "system" fn(usize, u32) = std::mem::transmute(base + TRAIL_RVA);
                f(sora, r.trail as u32);
                self.trail_asked = Some(0);
            }
            if r.effect >= 0 {
                let f: unsafe extern "system" fn(usize, i32, usize, u32) -> usize = std::mem::transmute(base + EFFECT_RVA);
                f(source, r.effect as i32, sora, 0x30);
            }
        }
        if r.voice >= 0 && kh1_voice {
            let f: unsafe extern "system" fn(usize, i32, usize) = std::mem::transmute(base + VOICE_RVA);
            f(sora, r.voice as i32, 0);
        }
    }

    /// KH2's Magnet: each caught enemy is drawn toward the pulling hitbox's centre, to its
    /// orbit radius at up to 20 a frame and swirled sideways (the exe's suction step, as
    /// `character_core::npc`), through KH1's own per-frame push (`+0x120`, which its physics
    /// step adds to the actor's move and clears). Let go when the swing ends.
    unsafe fn pull(&mut self, out: &Output, anim: &RigAnim, battle: &Battle) {
        if self.pulled.is_empty() {
            return;
        }
        let mut centres: HashMap<u64, [f32; 3]> = HashMap::new();
        for h in &out.hitboxes {
            let Some(a) = battle.attack_for_level(h.attack, 0, 0) else { continue };
            if character_core::reaction::category(a.enemy_reaction) != 5 {
                continue;
            }
            if let Some(c) = anim.attack_shapes(h.group, h.weapon).first() {
                centres.entry(h.swing).or_insert(c.center);
            }
        }
        let alive: HashMap<usize, f32> = self.bodies.iter().map(|b| (b.actor, b.target.height)).collect();
        self.pulled.retain(|a, (swing, _, _)| centres.contains_key(swing) && alive.contains_key(a));
        for (&actor, &(swing, radius, swirl)) in &self.pulled {
            let c = centres[&swing];
            let p = rd::<[f32; 3]>(actor + 0x10);
            // KH2's frame (y up), from the body's middle.
            let body = [p[0], -p[1] + alive[&actor] * 0.5, p[2]];
            let d = [c[0] - body[0], c[1] - body[1], c[2] - body[2]];
            let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if len < 1e-3 {
                continue;
            }
            let n = [d[0] / len, d[1] / len, d[2] / len];
            let radial = ((len - radius) * 0.1).clamp(-20.0, 20.0);
            let tl = (n[2] * n[2] + n[0] * n[0]).sqrt();
            let t = if tl > 1e-4 { [n[2] / tl, -n[0] / tl] } else { [0.0, 0.0] };
            let m = [n[0] * radial + t[0] * swirl, n[1] * radial, n[2] * radial + t[1] * swirl];
            let push = rd::<[f32; 3]>(actor + 0x120);
            wr(actor + 0x120, [push[0] + m[0], push[1] - m[1], push[2] + m[2]]);
        }
        static SEEN: AtomicU32 = AtomicU32::new(0);
        if SEEN.fetch_add(1, Ordering::Relaxed) % 60 == 0 {
            note(format!("Magnet: {} enemies held around {} centre(s)", self.pulled.len(), centres.len()));
        }
    }

    /// The trail's flag over KH2's swing (its hit windows); only ever cleared if we set it.
    pub unsafe fn trail(&mut self, sora: usize, swinging: bool) {
        let flags = rd::<u32>(sora + 0x1CC);
        if swinging {
            wr(sora + 0x1CC, flags | TRAIL_FLAG);
            TRAIL_ON.store(true, Ordering::Relaxed);
        } else {
            trail_off(sora);
        }
        // Whether the trail came (logged for the first few).
        if let Some(n) = self.trail_asked {
            let cs = pose::ptr(rd::<u32>(sora + 0x14C));
            let made = cs > 0x10000 && rd::<u64>(cs + 0x28) != 0;
            if made || n >= 30 {
                static SEEN: AtomicU32 = AtomicU32::new(0);
                if SEEN.fetch_add(1, Ordering::Relaxed) < 12 {
                    log(&format!("Keyblade trail {} ({n} frames after asking, trail flag {})", if made { "drawn" } else { "not drawn" }, flags & TRAIL_FLAG != 0));
                }
                self.trail_asked = None;
            } else {
                self.trail_asked = Some(n + 1);
            }
        }
    }

    /// The enemies this frame, as KH2's targets (before KH2's step).
    pub unsafe fn look(&mut self, sora: usize) -> Vec<TargetInfo> {
        self.bodies = bodies(sora);
        self.guards = guards(sora);
        self.shots = shots(sora);
        self.bodies.iter().map(|b| b.target).collect()
    }

    /// After KH2's step and pose: its live attack shapes against the enemies' bodies.
    pub unsafe fn strike(&mut self, sora: usize, out: &Output, anim: &RigAnim, character: &mut Character, battle: &Battle) {
        if !self.rows_tried {
            self.rows_tried = true;
            self.rows = sora_rows(sora);
            if self.rows.is_none() {
                log("Sora's KH1 combo rows not found: KH2's hits cannot be landed");
            }
        }
        let keys: HashSet<(u64, u8)> = out.hitboxes.iter().map(|h| (h.swing, h.weapon)).collect();
        self.volumes.retain(|k, _| keys.contains(k));
        self.met_shots.retain(|(sw, _)| keys.iter().any(|k| k.0 == *sw));
        let air = character.airborne();
        for h in &out.hitboxes {
            let shapes = anim.attack_shapes(h.group, h.weapon);
            let vol = self.volumes.entry((h.swing, h.weapon)).or_default();
            let before: Vec<[f32; 3]> = vol.previous().to_vec();
            let swept = vol.update(&shapes);
            if swept.is_empty() {
                continue;
            }
            let Some(first) = battle.attack_for_level(h.attack, 0, 0).cloned() else { continue };
            let group = if first.combo_group != 0 { first.combo_group as i64 } else { h.swing as i64 };
            // An enemy's guard repels the swing before it reaches a body (KH2's repel, as off a
            // wall; KH1's clash spark and sound).
            if self.repelled.contains(&h.swing) {
                continue;
            }
            // A wall in the blade's way since last frame: KH1's background ray, as its own swings
            // have it (`0x1402B1870`), and KH2's repel off it (the user: swings went through
            // walls). Walls only: a floor or ceiling the blade dips into doesn't repel.
            if let Some(at) = before.iter().zip(shapes.iter()).find_map(|(p, s)| wall_between([p[0], -p[1], p[2]], [s.center[0], -s.center[1], s.center[2]])) {
                self.repelled.insert(h.swing);
                let bounced = character.register_wall_hit();
                clang(at);
                note(format!("KH2 attack {} struck a wall at {at:.0?} (bounced: {bounced})", h.attack));
                continue;
            }
            let guard_shapes: Vec<Shape> = self.guards.iter().map(|g| g.shape).collect();
            if let Some((_, gi)) = first_overlap(&swept, &guard_shapes) {
                self.repelled.insert(h.swing);
                let bounced = character.register_wall_hit();
                clang(self.guards[gi].at);
                note(format!("KH2 attack {} repelled by the guard of actor {:#x} (bounced: {bounced})", h.attack, self.guards[gi].owner));
                continue;
            }
            // An enemy's shot met: knocked back as KH1's swing would (the user: Wakka's ball).
            if let Some(rows) = self.rows {
                for shot in &self.shots {
                    if self.met_shots.contains(&(h.swing, shot.obj)) {
                        continue;
                    }
                    let Some((ai, _)) = first_overlap(&swept, &shot.shapes) else { continue };
                    self.met_shots.insert((h.swing, shot.obj));
                    let back = meet_shot(sora, shot, &swept[ai], rows[(air as usize) * 2]);
                    if back {
                        character.register_hit();
                    }
                    static MET: AtomicU32 = AtomicU32::new(0);
                    if MET.fetch_add(1, Ordering::Relaxed) < 200 {
                        log(&format!("KH2 swing met enemy attack {:#x} of {:#x} (flags now {:08x}): turned back {back}", shot.obj, shot.owner, rd::<u32>(shot.obj)));
                    }
                }
            }
            for b in &self.bodies {
                let rec = self.records.entry(b.actor).or_default();
                if rec.has_hit(h.swing) {
                    continue;
                }
                let targets: Vec<Shape> = b.parts.iter().map(|p| p.shape).collect();
                let Some((ai, ti)) = first_overlap(&swept, &targets) else { continue };
                let adm = rec.admit(group, false, false);
                let row = battle.attack_for_level(h.attack, 0, adm.count.min(255) as u8).cloned().unwrap_or(first.clone());
                rec.mark(h.swing, row.interval);
                if !adm.effective {
                    continue;
                }
                character.register_hit();
                let finisher = character_core::reaction::category(row.enemy_reaction) == 4;
                let Some(rows) = self.rows else { continue };
                let kh1 = rows[(air as usize) * 2 + finisher as usize];
                let dealt = land(sora, &b.parts[ti], &swept[ai], kh1, row.power);
                // A Magnet's catch (the user: Magnet Burst didn't pull them together): KH2 picks
                // the orbit radius between the two knockback values, the swirl from the next.
                if character_core::reaction::category(row.enemy_reaction) == 5 {
                    let (a, c) = (row.knockback_1 as f32, row.knockback_2 as f32);
                    let t = ((b.actor >> 4) % 97) as f32 / 96.0;
                    self.pulled.insert(b.actor, (h.swing, a + (c - a) * t, row.unknown_16 as f32));
                }
                note(format!(
                    "KH2 attack {} (power {}, reaction {}) hit actor {:#x} part {ti} at {:?}: landed with KH1 row motion {} -> {dealt:?}",
                    h.attack, row.power, row.enemy_reaction, b.actor, swept[ai].center, kh1.motion
                ));
            }
        }
        self.pull(out, anim, battle);
        let now: HashSet<u64> = out.hitboxes.iter().map(|h| h.swing).collect();
        self.repelled.retain(|s| now.contains(s));
        for ended in self.live.difference(&now) {
            for r in self.records.values_mut() {
                r.forget(*ended);
            }
        }
        for r in self.records.values_mut() {
            r.tick(1.0, false);
        }
        self.live = now;
        let alive: HashSet<usize> = self.bodies.iter().map(|b| b.actor).collect();
        self.records.retain(|a, _| alive.contains(a));
    }
}

/// Damage of the last landed hit (set by `probe::hit` from the record).
pub static LAST_DAMAGE: AtomicU32 = AtomicU32::new(0);

/// Lands one hit with KH1's contact code, through a KH1 attack object made from Sora's row.
/// Returns the damage written to the hit record, if one was made.
unsafe fn land(sora: usize, part: &Part, at: &Shape, row: Row, power: u16) -> Option<u32> {
    let base = BASE.load(Ordering::Relaxed);
    let free_bit: unsafe extern "system" fn() -> u32 = std::mem::transmute(base + FREE_BIT_RVA);
    let new_attack: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + NEW_ATTACK_RVA);
    let contact: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + LAND_RVA);
    let free: unsafe extern "system" fn(usize) = std::mem::transmute(base + FREE_ATTACK_RVA);
    let bit = free_bit();
    let obj = new_attack(sora, bit, base + ATTACK_DATA_RVA + row.data as usize);
    if obj < 0x10000 {
        return None;
    }
    // As the Attack command's row does (`0x1402A17F0`): its effect and sound bytes.
    wr(obj, rd::<u32>(obj) | 0x400000);
    wr(obj + 0xA8, row.fx);
    // KH2's hit records decide what may hit again, not KH1's hit-once bit.
    let saved = rd::<u32>(obj + 0x54);
    wr(obj + 0x54, 0u32);
    let c = [at.center[0], -at.center[1], at.center[2], 1.0f32];
    wr(obj + 0x10, c);
    let mut entry = [0u8; 0x40];
    let e = entry.as_mut_ptr() as usize;
    wr(e, (rd::<u32>(obj) >> 12) & 3);
    wr(e + 8, sora as u64);
    wr(e + 0x18, obj as u64);
    wr(e + 0x20, 1u32);
    wr(e + 0x24, at.radius);
    wr(e + 0x28, at.radius);
    wr(e + 0x30, c);
    LAST_DAMAGE.store(u32::MAX, Ordering::Relaxed);
    POWER.store(power.max(1) as u32, Ordering::Relaxed);
    contact(e, part.entry.as_ptr() as usize);
    POWER.store(0, Ordering::Relaxed);
    wr(obj + 0x54, saved);
    free(obj);
    let d = LAST_DAMAGE.load(Ordering::Relaxed);
    (d != u32::MAX).then_some(d)
}

/// `probe::hit`: a hit record was just made; while KH2's hit is landed, its damage is KH1's
/// scaled by KH2's power.
pub unsafe fn scale_record(rec: usize) {
    let p = POWER.load(Ordering::Relaxed);
    if p == 0 || rec < 0x10000 {
        return;
    }
    let d = rd::<i32>(rec + 8);
    let scaled = ((d as i64 * p as i64) / 100).max(1) as i32;
    wr(rec + 8, scaled);
    LAST_DAMAGE.store(scaled as u32, Ordering::Relaxed);
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + EXEC_RVA, &EXEC_PROLOGUE, exec as *const () as usize, &ORIG_EXEC, "execute attack row");
    hook::install(base + COMMAND_STEP_RVA, &COMMAND_STEP_PROLOGUE, command as *const () as usize, &ORIG_COMMAND, "command step");
}
