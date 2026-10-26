//! Valor's left Keyblade's hits: its own KH1 attack object (t680; its own module now,
//! `docs/kh1-drive-forms-native.md` §7.2).
//!
//! KH2 keeps one attack per (group, shape-source object) (Steam KH2 `0x14040D810` ->
//! `0x1403DAFE0` -> `0x1403D29B0` -> `0x1403D25D0`, a new attack `0x14040F1C0`; Checked (decomp)),
//! so the two Keyblades always run separate attacks. The left Keyblade's hit windows (weapon 2)
//! play on a KH1 attack group of their own (`kh2slots::left_channel`); for each play of a Valor
//! slot with such windows, `new_play` makes one KH1 attack object of Sora's with KH1's constructor
//! `0x1402B2DC0` (bone `0x30`, motion-tied), with the left Keyblade's own weapon row bonuses as
//! KH1's row maker `0x1402B3270` adds the right one's; `step` re-arms it at each new KH2 attack
//! (`rehit::new_attack`) and puts the open window's data on it. KH1's collision pass collides it,
//! its contact lands the hit.
//!
//! `bodyhits` owns the pass's entry hook (`0x1402B3800`), the pool helpers and our data copies;
//! this module is called from it (`entries`, `atkp`, `hit`, `new_play`) and from `takeover`
//! (`step`).

use super::addr::Rva;
use super::bodyhits::{rd, wr};
use super::{log, pose};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

/// KH1's Keyblade bone id (data `+0x10`): the object is bone-tied (position kind 1, activity 2,
/// tied to the motion and its attack group, `0x1402B2DC0`), as KH1's own swing object is.
const BLADE_BONE: u32 = 0x30;
/// KH1 Sora's grip bone (the weapon's root under the hand, 280) as a raw bone point id.
const GRIP_POINT: u32 = 0x4000 | 280;
/// KH2's body shape kinds (`kh2_data::sora::BoneShape::kind`): attack and guard.
const ATTACK_KIND: u8 = 6;
const GUARD_KIND: u8 = 13;
/// KH1's attack constructor (Steam), its free hit bit, its object free and its bone point (Epic
/// RVAs, mapped).
const MAKE_STEAM: usize = 0x2B2DC0;
const FREE_BIT_RVA: Rva = Rva(0x2B15D0);
const FREE_ATTACK_RVA: Rva = Rva(0x2B13D0);
const BONE_POINT_RVA: Rva = Rva(0x29C990);

/// The live object's address (0: none), for the per-object checks without the lock.
static OBJ: AtomicUsize = AtomicUsize::new(0);
static STATE: Mutex<Option<Left>> = Mutex::new(None);
static LOGGED: AtomicU32 = AtomicU32::new(0);

fn note(s: impl FnOnce() -> String) {
    if super::log_on(super::LogCat::Combat) && LOGGED.fetch_add(1, Ordering::Relaxed) < 80 {
        log!(Combat, "{}", s());
    }
}

/// Valor's left Keyblade's KH1 attack object for the motion playing: one object for all its hit
/// windows (weapon 2), tied to their KH1 attack group (`kh2slots::left_channel`), re-armed at each
/// new KH2 attack (`step`), as KH1's own Keyblade object is for the right one.
struct Left {
    obj: usize,
    /// Its data now (the open window's).
    data: usize,
    motion: u16,
    windows: Vec<LeftWin>,
    /// The window open at the last step, and whether one was.
    cur: Option<usize>,
    was_open: bool,
    /// The left Keyblade's weapon bonuses on the priority (`0x1402B3270`'s): the row's `+0x33`.
    priority_bonus: i32,
    /// KH2's sweep of its shapes between frames (`combat_core::AttackVolume`, one per attack), the
    /// game frame it was last run on and that frame's shapes (the pass may build it twice).
    vol: combat_core::AttackVolume,
    vol_frame: u64,
    shapes: Vec<combat_core::Shape>,
}

/// One left-Keyblade hit window: its ATKP, its weapon-model collision group, KH1 time span, our
/// attack data for it.
#[derive(Clone, Copy)]
struct LeftWin {
    attack: u16,
    group: u8,
    start: u32,
    end: Option<u32>,
    data: usize,
}

impl LeftWin {
    fn contains(&self, f: u32) -> bool {
        f >= self.start && self.end.map_or(true, |e| f < e)
    }
}

/// Our object is still the one we made (in use, our data).
unsafe fn still_ours(l: &Left) -> bool {
    rd::<u32>(l.obj) & 0x800 != 0 && pose::ptr(rd::<u32>(l.obj + 0x5C)) == l.data
}

fn attack_kind(k: u8) -> bool {
    k == ATTACK_KIND || k == GUARD_KIND
}

/// The KH2 weapon rig whose shapes the left Keyblade hits with (Valor's second weapon; both of
/// KH2's weapons are `W_EX010.mdlx`, so the same shapes).
fn left_rig_shapes() -> Option<&'static [kh2_data::sora::BoneShape]> {
    let rig = &pose::kh2()?.valor.as_ref()?.rig;
    Some(rig.weapons.get(1).or_else(|| rig.weapons.first())?.shapes.as_slice())
}

/// A new play of motion `m` in our slot (`bodyhits::frame`): the last play's object freed, and
/// in Valor (on KH1's own states, two Keyblades) a new one for this play's left windows.
pub unsafe fn new_play(actor: usize, m: u16, req: u32, air: bool, clip: &character_core::Clip) {
    let Ok(mut g) = STATE.try_lock() else { return };
    if let Some(l) = g.take() {
        if still_ours(&l) {
            let free: unsafe extern "system" fn(usize) = std::mem::transmute(super::bodyhits::base() + FREE_ATTACK_RVA);
            free(l.obj);
        }
    }
    OBJ.store(0, Ordering::Relaxed);
    if super::form::native_now() && super::form::two_blades() {
        *g = make(actor, m, req, air, clip);
        OBJ.store(g.as_ref().map_or(0, |l| l.obj), Ordering::Relaxed);
    }
}

/// The left Keyblade's object for a new play of motion `m`: one KH1 attack object of Sora's,
/// made with KH1's attack constructor (`0x1402B2DC0`) on our copy of his KH1 swing row's data
/// (finisher row's for a KH2 blow), bone `0x30`, on the clip's left attack group
/// (`kh2slots::left_channel`), with the left Keyblade's own weapon bonuses on it as KH1's row
/// maker puts the right one's (`0x1402B3270`), its effect data as the spark's source and his
/// "connected" word.
unsafe fn make(actor: usize, m: u16, req: u32, air: bool, clip: &character_core::Clip) -> Option<Left> {
    if !super::bodyhits::make_ok() {
        return None;
    }
    let Some(ch) = super::kh2slots::left_channel(clip) else {
        if clip.windows.iter().any(|w| matches!(w.kind, character_core::WindowKind::Hitbox { weapon, .. } if weapon >= 2)) {
            note(|| format!("left Keyblade hits: motion {m} has no free KH1 attack group: left out"));
        }
        return None;
    };
    let (kh2, shapes) = (pose::kh2()?, left_rig_shapes()?);
    let base = super::bodyhits::base();
    let mut windows = Vec::new();
    for (idx, w) in clip.windows.iter().enumerate() {
        let character_core::WindowKind::Hitbox { attack, group, weapon, .. } = w.kind else { continue };
        if weapon < 2 {
            continue;
        }
        let Some(a) = kh2.battle.attack_for_level(attack, 0, 0).cloned() else { continue };
        let r = shapes.iter().find(|s| s.group == group && attack_kind(s.kind)).map_or(40.0, |s| s.radius);
        let Some(data) = super::bodyhits::left_data(m, idx, air, ch, &a, r, BLADE_BONE) else { continue };
        windows.push(LeftWin { attack, group, start: w.start, end: w.end, data });
    }
    let first = windows.first().copied()?;
    let make: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + MAKE_STEAM);
    let free_bit: unsafe extern "system" fn() -> u32 = std::mem::transmute(base + FREE_BIT_RVA);
    // The left Keyblade's weapon object (`leftblade`'s owner copy: the left Keyblade's row and
    // effect data), else Sora's own.
    let owner = super::leftblade::left_owner();
    let wobj = pose::ptr(rd::<u32>(owner.unwrap_or(actor) + 0x14C));
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    let bit = free_bit();
    if bit == 0 {
        note(|| format!("left Keyblade hits: no free hit bit (motion {m})"));
        return None;
    }
    let obj = make(actor, bit, first.data);
    if super::bodyhits::pool_index(obj).is_none() {
        note(|| format!("left Keyblade hits: KH1's constructor gave {obj:#x} (pool full?)"));
        return None;
    }
    // `0x1402B3270`'s weapon part, with the left Keyblade's row (Checked (decomp)).
    let mut priority_bonus = 0;
    if wobj > 0x10000 {
        let row = rd::<u64>(wobj + 0x20) as usize;
        if rd::<u32>(obj + 0x20) == BLADE_BONE && row > 0x10000 {
            wr(obj + 0xC, rd::<f32>(actor + 0x40) * rd::<f32>(row + 0x20) + rd::<f32>(obj + 0xC));
            wr(obj + 0x24, rd::<f32>(obj + 0x24) * rd::<f32>(row + 0x2C));
            wr(obj + 0x28, rd::<f32>(obj + 0x28) * rd::<f32>(row + 0x2C));
            priority_bonus = rd::<u8>(row + 0x33) as i32;
            wr(obj + 0x68, (rd::<i32>(obj + 0x68) + priority_bonus).clamp(1, 99));
            wr(obj + 0x94, rd::<u8>(row + 0x31) as f32 * 0.05 * rd::<f32>(obj + 0x94));
            wr(obj + 0x98, rd::<i32>(obj + 0x98) + rd::<i8>(row + 0x32) as i32);
        }
        wr(obj + 8, pose::handle(rd::<u64>(wobj + 0x18) as usize));
    }
    // His "connected" word (KH1's Attack step reads it; a hit sets it), not cleared: not a new swing.
    if info > 0x10000 {
        wr(obj + 0x60, (info + 0x50) as u64);
    }
    if let Some(fx) = super::bodyhits::template_fx(air, false) {
        wr(obj, rd::<u32>(obj) | 0x40_0000);
        wr(obj + 0xA8, fx);
    }
    wr(obj + 0x58, req);
    let list: Vec<String> = windows.iter().map(|w| format!("ATKP {} group {} at {}..{:?}", w.attack, w.group, w.start, w.end)).collect();
    note(|| format!("left Keyblade hits: motion {m} (request {req:#x}, {}): KH1 attack object {obj:#x} on KH1 group {ch}{}: {}", if air { "air" } else { "ground" }, if owner.is_some() { "" } else { " (left Keyblade not drawn: placed on the right one)" }, list.join("; ")));
    Some(Left { obj, data: first.data, motion: m, windows, cur: None, was_open: false, priority_bonus, vol: combat_core::AttackVolume::new(), vol_frame: u64::MAX, shapes: Vec::new() })
}

/// Each frame KH1 has Sora (`takeover::physics`, beside `kh2slots::keyblade_rearm`, before KH1's
/// collision pass): the left window open at KH1's time in our slot. A new KH2 attack there (its
/// ATKP or group differs from the last window's, or a window opens after a frame with none:
/// Steam KH2 `0x1403D25D0` keeps an attack per (group, weapon object), Checked (decomp)) re-arms
/// the object with KH1's own re-arm (`rehit::new_attack`) and starts KH2's sweep afresh; each
/// window's data (its reaction, clash rules) goes on the object.
pub unsafe fn step(actor: usize) {
    if !super::bodyhits::native() || actor < 0x10000 || OBJ.load(Ordering::Relaxed) == 0 {
        return;
    }
    let Ok(mut g) = STATE.try_lock() else { return };
    if !g.as_ref().is_some_and(|l| still_ours(l)) {
        *g = None;
        OBJ.store(0, Ordering::Relaxed);
        return;
    }
    let Some((m, _, t)) = super::kh2slots::playing_clip(actor) else { return };
    let Some(l) = g.as_mut() else { return };
    if m != l.motion {
        return;
    }
    let f = t as u32;
    let Some(o) = l.windows.iter().position(|w| w.contains(f)) else {
        l.was_open = false;
        return;
    };
    let w = l.windows[o];
    let new = match l.cur {
        Some(c) => !l.was_open || l.windows[c].attack != w.attack || l.windows[c].group != w.group,
        None => false,
    };
    let switch = l.cur != Some(o);
    l.cur = Some(o);
    l.was_open = true;
    if new {
        l.vol = combat_core::AttackVolume::new();
        l.vol_frame = u64::MAX;
    }
    let obj = l.obj;
    if switch {
        wr(obj + 0x5C, pose::handle(w.data));
        wr(obj + 0x68, (rd::<i32>(w.data + 0x3C) + l.priority_bonus).clamp(1, 99));
        l.data = w.data;
    }
    drop(g);
    if new {
        super::rehit::new_attack(obj);
        static N: AtomicU32 = AtomicU32::new(0);
        if super::log_on(super::LogCat::Combat) && N.fetch_add(1, Ordering::Relaxed) < 40 {
            log!(Combat, "left Keyblade hits: KH2's next attack (motion {m}, ATKP {} group {}, frame {t:.0}): its KH1 object re-armed", w.attack, w.group);
        }
    }
}

/// The window's ATKP for our object (`bodyhits::atkp`, `kh1rows::kh2_atkp`): the window open at
/// the last step, else the first.
pub unsafe fn atkp(attack: usize) -> Option<kh2_formats::battle::Atkp> {
    if attack == 0 || OBJ.load(Ordering::Relaxed) != attack {
        return None;
    }
    let g = STATE.try_lock().ok()?;
    let l = g.as_ref().filter(|l| l.obj == attack && still_ours(l))?;
    let id = l.windows.get(l.cur.unwrap_or(0))?.attack;
    drop(g);
    pose::kh2()?.battle.attack_for_level(id, 0, 0).cloned()
}

/// `bodyhits::hit`: KH1's contact made a hit with our object (logged).
pub unsafe fn hit(attack: usize, target: usize) {
    if attack == 0 || OBJ.load(Ordering::Relaxed) != attack {
        return;
    }
    let Ok(g) = STATE.try_lock() else { return };
    let Some(l) = g.as_ref() else { return };
    let (atk, motion) = (l.windows.get(l.cur.unwrap_or(0)).map_or(0, |w| w.attack), l.motion);
    drop(g);
    note(|| format!("KH2 left Keyblade hit: ATKP {atk} (motion {motion}) from KH1's pass on actor {target:#x}"));
}

/// `bodyhits`' hook on KH1's `0x1402B3800 (object, entries, count)`: for our object, its entries
/// (KH2's weapon shapes along the left Keyblade, `left_shapes`), tested with KH1's own test.
/// `None` for any other object.
pub unsafe fn entries(obj: usize, list: usize, count: i32) -> Option<i32> {
    if obj == 0 || OBJ.load(Ordering::Relaxed) != obj {
        return None;
    }
    let Ok(mut g) = STATE.try_lock() else { return Some(count) };
    if !g.as_ref().is_some_and(|l| l.obj == obj && still_ours(l)) {
        return None;
    }
    if !super::bodyhits::test_ok() || list < 0x10000 {
        return Some(count);
    }
    let owner = pose::ptr(rd::<u32>(obj + 0x34));
    if owner < 0x10000 {
        return Some(count);
    }
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    let bit = rd::<u32>(obj + 0x54);
    let Some(shapes) = left_shapes(g.as_mut()?, owner, frame) else { return Some(count) };
    drop(g);
    let (Some(a), Some(b)) = (shapes.first(), shapes.last()) else { return Some(count) };
    // The object's own place, as KH1's blade object has it (`0x1402B3800`, position kind 1):
    // `+0x10` the blade's first point, `+0x40` its last (the spark, knock-back, clash).
    wr(obj + 0x10, [a.center[0], a.center[1], a.center[2], 1.0f32]);
    wr(obj + 0x40, [b.center[0], b.center[1], b.center[2], 1.0f32]);
    let mut count = count;
    for s in &shapes {
        let t = if s.kind == combat_core::ShapeKind::Column { 2 } else { 1 };
        count = super::bodyhits::put_entry(obj, owner, bit, list, count, t, s.radius, s.height, s.center);
    }
    Some(count)
}

/// The left Keyblade's shapes this frame (KH1 world), with KH2's sweep: the open window's group of
/// KH2's weapon-model shapes (`W_EX010.mdlx`, spheres along its bone 2's -x from the grip, Checked
/// (files)), laid along the left Keyblade as drawn: from KH1's grip bone (280) toward its blade's
/// tip (KH1's blade point `0x30`, t 1, through its own row's blade range), both read with KH1's
/// bone point (`0x14029EB20`) on `leftblade`'s owner copy (Sora's matrices moved onto the left
/// Keyblade). KH2's distances and radii as they are (KH1 units, as `bodyhits`' body shapes).
unsafe fn left_shapes(l: &mut Left, sora: usize, frame: u64) -> Option<Vec<combat_core::Shape>> {
    use combat_core::{Shape, ShapeKind};
    if l.vol_frame == frame {
        return Some(l.shapes.clone());
    }
    let group = l.windows.get(l.cur.unwrap_or(0))?.group;
    let owner = super::leftblade::left_owner().unwrap_or_else(|| {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 3 {
            log!(Combat, "left Keyblade hits: the left Keyblade isn't drawn: its windows on the right Keyblade");
        }
        sora
    });
    let point: unsafe extern "system" fn(usize, u32, f32, *mut [f32; 4]) = std::mem::transmute(super::bodyhits::base() + BONE_POINT_RVA);
    let (mut grip, mut tip) = ([0.0f32; 4], [0.0f32; 4]);
    point(owner, GRIP_POINT, 1.0, &mut grip);
    point(owner, BLADE_BONE, 1.0, &mut tip);
    let d = [tip[0] - grip[0], tip[1] - grip[1], tip[2] - grip[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if !(len > 1.0) || !len.is_finite() {
        return None;
    }
    let dir = [d[0] / len, d[1] / len, d[2] / len];
    let mut now = Vec::new();
    for s in left_rig_shapes()?.iter().filter(|s| s.group == group && attack_kind(s.kind)) {
        let kind = match ShapeKind::from_u8(s.shape) {
            Some(ShapeKind::Cube) | None => continue,
            Some(k) => k,
        };
        // (KH2's weapon shapes lie on its bone 2's x axis, the blade along -x; their y / z offsets
        // and lifts are all 0, Checked (files).)
        let along = -s.offset[0];
        now.push(Shape::new(kind, [grip[0] + dir[0] * along, grip[1] + dir[1] * along, grip[2] + dir[2] * along], s.radius, s.height));
    }
    if now.is_empty() {
        return None;
    }
    l.shapes = l.vol.update(&now);
    l.vol_frame = frame;
    Some(l.shapes.clone())
}
