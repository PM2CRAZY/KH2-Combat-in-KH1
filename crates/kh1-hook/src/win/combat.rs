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
    if actor == sora && command == 1 && takeover::ON.load(Ordering::Relaxed) && takeover::active() {
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
    if actor != sora || matches!(kind, 0 | 1) || NOT_SORAS.contains(&kind) || !super::settings::get().commands || !takeover::ON.load(Ordering::Relaxed) || !takeover::active() {
        return f(actor, dir);
    }
    let air = takeover::kh2_airborne();
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
    let full = rd::<u32>(base + COMMAND_RVA);
    f(actor, dir);
    takeover::hand_to_kh1(&format!("command {kind:#x} ({full:08x}){}", if air { " in the air" } else { "" }));
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
    Some(Row { motion: rd(r), data: rd(r + 4), fx: rd(r + 0xC) })
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

impl Strikes {
    /// The enemies this frame, as KH2's targets (before KH2's step).
    pub unsafe fn look(&mut self, sora: usize) -> Vec<TargetInfo> {
        self.bodies = bodies(sora);
        self.guards = guards(sora);
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
        let air = character.airborne();
        for h in &out.hitboxes {
            let shapes = anim.attack_shapes(h.group, h.weapon);
            let swept = self.volumes.entry((h.swing, h.weapon)).or_default().update(&shapes);
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
            let guard_shapes: Vec<Shape> = self.guards.iter().map(|g| g.shape).collect();
            if let Some((_, gi)) = first_overlap(&swept, &guard_shapes) {
                self.repelled.insert(h.swing);
                let bounced = character.register_wall_hit();
                clang(self.guards[gi].at);
                note(format!("KH2 attack {} repelled by the guard of actor {:#x} (bounced: {bounced})", h.attack, self.guards[gi].owner));
                continue;
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
                note(format!(
                    "KH2 attack {} (power {}, reaction {}) hit actor {:#x} part {ti} at {:?}: landed with KH1 row motion {} -> {dealt:?}",
                    h.attack, row.power, row.enemy_reaction, b.actor, swept[ai].center, kh1.motion
                ));
            }
        }
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
