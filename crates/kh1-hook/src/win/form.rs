//! Wisdom Form (the user: "lets add wisdom form", from the second command page's Drive, costing
//! MP, Sora recoloured blue). Chosen from `drive`'s list, it plays out as KH2's Drive does
//! (`kh2-bevy`'s `drive.rs`, from the exe):
//! - Sora's Drive motion (52, invulnerable) with base Sora's moveset; when it ends, the swap:
//!   the form's moveset, motions and combo rows (`P_EX100_MAGF`, moveset 3), his facing and
//!   lock kept, and its arrival (53). Its abilities: Sora's support abilities, not his action
//!   or growth ones, and the form's own growth ability, Quick Run, at the form's level;
//! - the form lasts `(level + 3) × 600` frames of battle (KH2's gauge, which stands still
//!   out of battle); then, or on the Revert command (the same Drive entry), back to Sora with
//!   the revert motion (4), on the ground. Knocked out: back at once;
//! - its shots: a motion's frame trigger 1 (`EventKind::Effect(n)`) runs the form model's
//!   script (`wisd`, BAR entry 3) as KH2 does (`Magic::object_effect`), with the form's effect
//!   archive (`kh2fx` package 4); its attack (ATKP 859) hits KH1's enemies through the same
//!   hit records as Sora's spells (`Strikes::spell_strike`). With nothing locked on, a shot
//!   goes for the nearest enemy in front, as KH2's auto-target does;
//! - spells in the form: KH2's magic sub + 10 (Reflect, ours).
//! The cost is MP (`COST`, KH1's own payment), not a Drive gauge, which KH1 has none of.

use super::reflect::{flip, world_now, KhWorld, SORA};
use super::{combat, log, pose};
use character_core::{Abilities, Character, EventKind, Output};
use glam::{Mat4, Quat, Vec3};
use kh2_formats::battle::{Atkp, Battle};
use kh2_magic::{Event, Magic, SpellFile};
use kh2_rig::RigAnim;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

pub const DRIVE_MOTION: u16 = 52;
pub const ARRIVE_MOTION: u16 = 53;
pub const REVERT_MOTION: u16 = 4;
/// The longest Sora's Drive motion runs before the swap (KH2: its 160 frames).
const DRIVE_FRAMES: u32 = 160;
/// MP to change (KH1's payment takes hundredths of a bar from 100 up).
pub const COST_MP: i32 = 3;
const COST: i32 = COST_MP * 100;
/// The form's level (KH1 has no form experience): Lv 1, 40 s of battle.
const LEVEL: u32 = 1;
/// KH2's magic sub offset in Wisdom Form.
const MAGIC_SUB: i32 = 10;
/// Wisdom's shots' hit-record keys (apart from Reflect's and the swings').
const SHOT_KEY: u64 = 1 << 59;
/// The farthest an unlocked shot looks for an enemy, and how far off his facing.
const AUTO_RANGE: f32 = 1500.0;
const AUTO_CONE: f32 = 0.5;

/// A request: 1 Wisdom, 2 revert (the menu), 3 knocked out (`hurt`).
static REQUEST: AtomicU8 = AtomicU8::new(0);
/// In a form (the menu's entry reads "Revert"), or changing into one.
static IN_FORM: AtomicBool = AtomicBool::new(false);
static CHANGING: AtomicBool = AtomicBool::new(false);

/// Sora went down (`hurt`): out of the form at once, when KH2 next has him.
pub fn knocked_out() {
    if in_form() || CHANGING.load(Ordering::Relaxed) {
        REQUEST.store(3, Ordering::Relaxed);
    }
}

pub fn in_form() -> bool {
    IN_FORM.load(Ordering::Relaxed)
}

pub fn busy() -> bool {
    CHANGING.load(Ordering::Relaxed) || REQUEST.load(Ordering::Relaxed) != 0
}

/// KH2's magic sub offset now (`reflect`).
pub fn magic_sub() -> i32 {
    if in_form() {
        MAGIC_SUB
    } else {
        0
    }
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn mp(sora: usize) -> i32 {
    let stats = pose::ptr(rd::<u32>(sora + 0x6C));
    if stats < 0x10000 {
        return 0;
    }
    rd::<i32>(stats + 0x44)
}

unsafe fn hp(sora: usize) -> i32 {
    let stats = pose::ptr(rd::<u32>(sora + 0x6C));
    if stats < 0x10000 {
        return 1;
    }
    rd::<i32>(stats + 0x3C)
}

/// Whether the Drive entry can be chosen now (`drive`'s menu): reverting in the form, else MP
/// for it and KH2 having Sora, its data loaded.
pub unsafe fn usable(sora: usize) -> bool {
    if busy() {
        return false;
    }
    if in_form() {
        return true;
    }
    pose::kh2().is_some_and(|k| k.wisdom.is_some()) && super::takeover::kh2_has() && sora > 0x10000 && mp(sora) >= COST_MP
}

/// Chosen from the Drive list (`drive::chosen`). Returns why not, if refused.
pub unsafe fn choose(sora: usize) -> Result<&'static str, String> {
    if busy() {
        return Err("a change is under way".into());
    }
    if in_form() {
        REQUEST.store(2, Ordering::Relaxed);
        return Ok("Revert");
    }
    if !pose::kh2().is_some_and(|k| k.wisdom.is_some()) {
        return Err("Wisdom Form's files didn't load".into());
    }
    if !super::takeover::kh2_has() {
        return Err("KH1 has Sora".into());
    }
    let have = mp(sora);
    if have < COST_MP {
        return Err(format!("MP {have}, it costs {COST_MP}"));
    }
    REQUEST.store(1, Ordering::Relaxed);
    Ok("Wisdom Form")
}

/// The Driver's form state (`takeover`).
#[derive(Default)]
pub struct Drive {
    /// Changing: Sora's Drive motion playing, its frames.
    change: Option<u32>,
    /// In the form: battle frames left, Sora's own abilities (back on revert), the form's.
    form: Option<InForm>,
    /// A revert asked for, waiting for the ground.
    revert: bool,
}

struct InForm {
    left: u32,
    sora: Abilities,
    own: Abilities,
    logged_s: u32,
}

/// The form's controller data while in it, else Sora's.
pub fn character<'a>(d: &Drive, kh2: &'a pose::Kh2) -> &'a Character {
    match (&d.form, &kh2.wisdom) {
        (Some(_), Some(f)) => &f.character,
        _ => &kh2.character,
    }
}

pub fn battle<'a>(d: &Drive, kh2: &'a pose::Kh2) -> &'a Arc<Battle> {
    match (&d.form, &kh2.wisdom) {
        (Some(_), Some(f)) => &f.battle,
        _ => &kh2.battle,
    }
}

/// The form's abilities (KH2's, exe): Sora's support abilities carry over, his action and
/// growth ones don't; the form's own growth ability at its level; every combo row of the form's
/// table unlocked.
fn form_abilities(sora: &Abilities, form: &Character) -> Abilities {
    let mut a = Abilities {
        guard: form.moveset.clips.contains_key(&character_core::motion::GUARD),
        aerial_recovery: false,
        combo_plus: sora.combo_plus,
        air_combo_plus: sora.air_combo_plus,
        finishing_plus: sora.finishing_plus,
        negative_combo: sora.negative_combo,
        combo_master: sora.combo_master,
        one_swing_on_miss: sora.one_swing_on_miss,
        high_jump: 0,
        quick_run: LEVEL as u8,
        aerial_dodge: 0,
        glide: 0,
        dodge_roll: 0,
        ..sora.clone()
    };
    a.equipped = form.moveset.combo_rows.iter().map(|r| r.ability).filter(|&x| x != 0).collect();
    a
}

/// Replaces the controller (keeps facing, lock-on, the glide button).
fn swap(ch: &mut Character, anim: &mut RigAnim, data: &Character, rig: &Arc<kh2_data::sora::Rig>, abilities: Abilities) {
    let (facing, lock, glide) = (ch.facing(), ch.lock(), ch.glide_button);
    let mut c = data.clone();
    c.abilities = abilities;
    c.glide_button = glide;
    c.air_magic_falls = ch.air_magic_falls;
    c.set_facing(facing);
    c.set_lock(lock);
    *ch = c;
    let field = anim.field;
    *anim = RigAnim::new(rig.clone());
    anim.queued_blend = 14.0;
    anim.field = field;
}

/// Before KH2's step: the menu's request, the change under way, the form's end.
pub unsafe fn before_step(d: &mut Drive, ch: &mut Character, anim: &mut RigAnim, kh2: &pose::Kh2, actor: usize) {
    // Knocked out in the form: Sora again at once (KH2: a KO reverts).
    let knocked = REQUEST.load(Ordering::Relaxed) == 3 && REQUEST.swap(0, Ordering::Relaxed) == 3;
    if (d.form.is_some() || d.change.is_some()) && (knocked || hp(actor) <= 0) {
        revert(d, ch, anim, kh2, false, "knocked out");
        return;
    }
    match REQUEST.swap(0, Ordering::Relaxed) {
        1 if d.form.is_none() && d.change.is_none() => {
            if ch.airborne() || !ch.can_take_command() {
                // (KH2 takes the Drive command in the air too; here it waits for the ground.)
                log(&format!("Drive: Wisdom Form waits (airborne {}, busy {})", ch.airborne(), !ch.can_take_command()));
                REQUEST.store(1, Ordering::Relaxed);
                return;
            }
            if mp(actor) < COST_MP {
                log(&format!("Drive: MP {} short of {COST_MP} by now", mp(actor)));
                return;
            }
            if !ch.play(DRIVE_MOTION, true) {
                log("Drive: Sora has no Drive motion (52)");
                return;
            }
            let before = mp(actor);
            super::reflect::pay_mp(actor, COST);
            d.change = Some(0);
            CHANGING.store(true, Ordering::Relaxed);
            log(&format!("Drive: Wisdom Form (MP {before} -> {}), Sora's Drive motion", mp(actor)));
        }
        2 if d.form.is_some() => d.revert = true,
        _ => {}
    }
    if let Some(n) = d.change {
        let n = n + 1;
        d.change = Some(n);
        if !ch.scripted() || n >= DRIVE_FRAMES {
            d.change = None;
            arrive(d, ch, anim, kh2);
        }
        return;
    }
    if d.revert && !ch.airborne() && ch.can_take_command() {
        d.revert = false;
        revert(d, ch, anim, kh2, true, "the Revert command");
    }
}

unsafe fn arrive(d: &mut Drive, ch: &mut Character, anim: &mut RigAnim, kh2: &pose::Kh2) {
    CHANGING.store(false, Ordering::Relaxed);
    let Some(f) = &kh2.wisdom else { return };
    let sora = ch.abilities.clone();
    let own = form_abilities(&sora, &f.character);
    swap(ch, anim, &f.character, &f.rig, own.clone());
    ch.play(ARRIVE_MOTION, true);
    let left = (LEVEL + 3) * 600;
    d.form = Some(InForm { left, sora, own, logged_s: u32::MAX });
    IN_FORM.store(true, Ordering::Relaxed);
    log(&format!("Wisdom Form (Lv{LEVEL}): {:.0} s of battle, Quick Run LV{LEVEL}", left as f32 / 60.0));
}

unsafe fn revert(d: &mut Drive, ch: &mut Character, anim: &mut RigAnim, kh2: &pose::Kh2, motion: bool, why: &str) {
    CHANGING.store(false, Ordering::Relaxed);
    d.change = None;
    d.revert = false;
    let Some(f) = d.form.take() else { return };
    IN_FORM.store(false, Ordering::Relaxed);
    swap(ch, anim, &kh2.character, &kh2.rig, f.sora);
    if motion {
        ch.play(REVERT_MOTION, true);
    }
    end_shots();
    log(&format!("Revert ({why})"));
}

/// After KH1's menu set Sora's abilities (`menu::Growth`): in the form, they are his own for
/// after it, and the form's stay.
pub fn abilities(d: &mut Drive, ch: &mut Character) {
    if let Some(f) = d.form.as_mut() {
        if ch.abilities != f.own {
            f.sora = ch.abilities.clone();
            // (Its own growth ability and rows stay.)
            let mut own = f.own.clone();
            own.combo_plus = f.sora.combo_plus;
            own.air_combo_plus = f.sora.air_combo_plus;
            own.finishing_plus = f.sora.finishing_plus;
            own.negative_combo = f.sora.negative_combo;
            own.combo_master = f.sora.combo_master;
            f.own = own.clone();
            ch.abilities = own;
        }
    }
}

/// After KH2's step: the look (`tint`), the gauge (battle frames), its end.
pub unsafe fn after_step(d: &mut Drive, actor: usize, battle: bool) {
    super::tint::refresh(actor, d.form.is_some());
    let Some(f) = d.form.as_mut() else { return };
    if battle {
        f.left = f.left.saturating_sub(1);
    }
    let s = f.left.div_ceil(60);
    if s % 10 == 0 && s != f.logged_s {
        f.logged_s = s;
        log(&format!("Wisdom Form: {s} s left"));
    }
    if f.left == 0 && !d.revert {
        d.revert = true;
        log("Wisdom Form: the gauge is empty");
    }
}

// ---------------------------------------------------------------- the shots

struct Shots {
    magic: Magic,
    file: usize,
    /// Script effect id → our drawn effect, and whether its end was passed on.
    fx: HashMap<u64, (Option<u32>, bool)>,
    queued: Vec<i16>,
    fired: u32,
    hits: u32,
}

static SHOTS: Mutex<Option<Shots>> = Mutex::new(None);
static NO_SHOTS: AtomicBool = AtomicBool::new(false);

fn shots(g: &mut Option<Shots>) -> Option<&mut Shots> {
    if g.is_none() && !NO_SHOTS.load(Ordering::Relaxed) {
        let f = pose::kh2()?.wisdom.as_ref()?;
        let made = (|| {
            let script = f.script.as_ref().ok_or("no script in its model")?;
            let pax = f.archive.as_ref().and_then(|a| SpellFile::bar_entry(a, 18)).ok_or("no effects in its archive")?;
            SpellFile::from_parts("P_EX100_MAGF", script, &pax)
        })();
        match made {
            Ok(file) => {
                let mut magic = Magic::new();
                let file = magic.add_spell(file);
                *g = Some(Shots { magic, file, fx: HashMap::new(), queued: Vec::new(), fired: 0, hits: 0 });
                log("Wisdom Form's script ready (its shots)");
            }
            Err(e) => {
                NO_SHOTS.store(true, Ordering::Relaxed);
                log(&format!("Wisdom Form fires no shots: {e}"));
            }
        }
    }
    g.as_mut()
}

/// Wisdom's effect elements' sounds (`kh2sound`, as spell package `kh2fx::WISDOM`).
pub fn element_sounds(kh2: &pose::Kh2) -> Vec<(u8, u16, i32)> {
    let Some(pax) = kh2.wisdom.as_ref().and_then(|f| f.archive.as_ref()).and_then(|a| SpellFile::bar_entry(a, 18)).and_then(|p| kh2_formats::pax::Pax::parse(&p).ok()) else {
        return Vec::new();
    };
    pax.elements.iter().map(|e| (super::kh2fx::WISDOM, e.id, e.sound)).collect()
}

/// A frame trigger 1 of the form's motion this frame (`takeover`): a shot.
pub fn effect(n: i16) {
    if let Ok(mut g) = SHOTS.lock() {
        if let Some(s) = shots(&mut g) {
            s.queued.push(n);
        }
    }
}

fn end_shots() {
    if let Ok(mut g) = SHOTS.lock() {
        if let Some(s) = g.as_mut() {
            s.queued.clear();
        }
    }
}

fn row(battle: &Battle, atk: u16, sub: u8, count: u8) -> Option<&Atkp> {
    battle.attack_for_level(atk, sub, count)
}

/// An unlocked shot's target: the nearest enemy within `AUTO_RANGE` in front of him.
fn auto_target(w: &KhWorld, facing: f32) -> Option<u32> {
    let me = w.objs.first()?;
    let ahead = Vec3::new(facing.sin(), 0.0, facing.cos());
    w.objs
        .iter()
        .skip(1)
        .filter_map(|o| {
            let d = o.target - me.pos;
            let flat = Vec3::new(d.x, 0.0, d.z);
            let dist = flat.length();
            (dist < AUTO_RANGE && flat.normalize_or_zero().dot(ahead) > AUTO_CONE).then_some((o.id, dist))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|x| x.0)
}

/// After KH2's step and the swings: the shots' frame (script, effects, hits).
pub unsafe fn shots_step(actor: usize, out: &Output, airborne: bool, strikes: &mut combat::Strikes, battle: &Battle, lock: Option<u32>) {
    let Ok(mut g) = SHOTS.try_lock() else { return };
    let Some(s) = g.as_mut() else { return };
    if s.queued.is_empty() && s.magic.casts.is_empty() && s.magic.effects.is_empty() && s.fx.is_empty() {
        return;
    }
    let mut world = world_now(actor, out, airborne, strikes, lock, 0x51A7_0000 ^ s.fired.wrapping_mul(2_654_435_761));
    if world.target.is_none() {
        world.target = auto_target(&world, out.facing);
    }
    for n in std::mem::take(&mut s.queued) {
        match s.magic.object_effect(s.file, SORA, n, &mut world) {
            Ok(_) => {
                s.fired += 1;
                if s.fired <= 30 {
                    log(&format!("Wisdom shot (trigger {n}, motion {}), target {:?}", out.motion, world.target));
                }
            }
            Err(e) => log(&format!("Wisdom's script didn't take trigger {n}: {e}")),
        }
    }
    for e in s.magic.update(&mut world).into_iter().chain(s.magic.take_events()) {
        match e {
            Event::Missing(n) => {
                static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                if N.fetch_add(1, Ordering::Relaxed) < 10 {
                    log(&format!("Wisdom's script used {n} (not modelled)"));
                }
            }
            Event::Error(e) => log(&format!("Wisdom's script error: {e}")),
            Event::Strike { .. } => {}
        }
    }
    // Effects: bound ones (the muzzle flash) follow Sora's bones; the shot's own the script moves.
    let mut seen = Vec::new();
    for e in &s.magic.effects {
        seen.push(e.id);
        let placed = e.owner.is_none().then(|| place(e.pos, e.dir));
        match s.fx.get(&e.id) {
            None => {
                let h = if e.alive() { super::kh2fx::spell_start(super::kh2fx::WISDOM, e.element.id) } else { None };
                if let (Some(h), Some(at)) = (h, placed) {
                    super::kh2fx::spell_place(h, at);
                }
                if e.alive() {
                    super::kh2sound::spell_element(super::kh2fx::WISDOM, e.element.id);
                }
                s.fx.insert(e.id, (h, false));
            }
            Some((Some(h), ended)) => {
                let (h, ended) = (*h, *ended);
                if let Some(at) = placed {
                    super::kh2fx::spell_place(h, at);
                }
                if e.killed {
                    super::kh2fx::spell_kill(h);
                } else if e.fade.is_some() {
                    super::kh2fx::spell_alpha(h, e.alpha());
                    if !e.alive() {
                        super::kh2fx::spell_kill(h);
                    }
                } else if e.ended_at.is_some() && !ended {
                    super::kh2fx::spell_end(h);
                    s.fx.insert(e.id, (Some(h), true));
                } else if !e.alive() && !ended {
                    super::kh2fx::spell_end(h);
                    s.fx.insert(e.id, (Some(h), true));
                }
            }
            Some((None, _)) => {}
        }
    }
    s.fx.retain(|id, (h, _)| seen.contains(id) || h.is_some_and(super::kh2fx::spell_running));
    // The shots' attacks against KH1's enemies; one that hits is told so (the script ends it).
    let attacks: Vec<combat::SpellAttack> = s
        .magic
        .attacks
        .iter()
        .filter(|a| !a.freed && a.owner == SORA)
        .filter_map(|a| {
            let first = row(battle, a.atk, a.sub, 0)?;
            let shapes = a.shapes();
            (!shapes.is_empty()).then(|| combat::SpellAttack { key: a.id | SHOT_KEY, group: if first.combo_group != 0 { first.combo_group as i64 } else { a.group }, atk: a.atk, sub: a.sub, shapes })
        })
        .collect();
    if !attacks.is_empty() {
        let hits = strikes.spell_strike(actor, &attacks, &|atk, sub, count| row(battle, atk, sub, count).cloned());
        for (key, atk, foe, dealt) in hits {
            s.hits += 1;
            if let Some(a) = s.magic.attacks.iter_mut().find(|a| a.id | SHOT_KEY == key) {
                a.hit = true;
            }
            if s.hits <= 40 {
                log(&format!("Wisdom shot: ATKP {atk} hit {foe:#x} -> damage {dealt:?}"));
            }
        }
    }
    for a in s.magic.freed().filter(|a| a.owns_id) {
        strikes.spell_forget(a.id | SHOT_KEY);
    }
}

/// An unbound effect's root where the script has it (y up, our axes) → KH2's world (y down),
/// turned along its direction.
fn place(pos: Vec3, dir: Vec3) -> Mat4 {
    let r = Quat::from_rotation_arc(Vec3::Z, dir.try_normalize().unwrap_or(Vec3::Z));
    flip() * Mat4::from_rotation_translation(r, pos) * flip()
}

/// Sora's Drive and revert motions: their effects and sounds play whatever the ini's lists
/// (`kh2fx`, `kh2sound`).
pub fn own_motion(motion: u16) -> bool {
    matches!(motion, DRIVE_MOTION | REVERT_MOTION)
}

/// Whether this frame's event goes to the form (its shots) instead of Sora's effects.
pub fn takes(d: &Drive, e: &EventKind) -> bool {
    d.form.is_some() && matches!(e, EventKind::Effect(_))
}
