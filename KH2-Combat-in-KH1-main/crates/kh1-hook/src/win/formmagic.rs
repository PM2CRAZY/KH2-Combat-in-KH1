//! Wisdom Form's KH2 scripts: its shots and its spells (`form`).
//!
//! - Shots: Wisdom's motions carry frame trigger 1 (`EventKind::Effect(n)`) on their shot frames;
//!   each runs the form model's script (`wisd`, BAR entry 3 of `P_EX100_MAGF.mdlx`) as KH2 does
//!   (`Magic::object_effect`), with the form's effect archive (`kh2fx` package 4). Its attack
//!   (ATKP 859) hits KH1's enemies through the spell hit records (`Strikes::spell_strike`); a
//!   hit is passed back (the script ends the shot). With nothing locked on, a shot goes for the
//!   nearest enemy in front. The arrival's triggers (motion 53) start the change's light; its
//!   looping parts are ended when the arrival does (KH2's form change ends them; left alone they
//!   stayed on Sora, the user), and everything of the form's is put out on a revert.
//! - Spells (the user: "lets use the wisdom form version of these spells": Fire, Blizzard,
//!   Thunder, Cure): chosen from KH1's menu (or a shortcut) in the form, while KH2 has Sora,
//!   they are KH2's: KH2's cast motion for the spell (MAGC, the form's own animations), KH1's
//!   cost and level (Fire / Fira / Firaga ...), KH2's script (`magic/FIRE_<level>.mag` ...)
//!   at Wisdom's magic sub (level × 2 − 2, + 1 for the combo finisher, + 10), its effects
//!   (`kh2fx` packages from 5, `spell_package`) and sounds, its attacks through the spell hit
//!   records. Cure's heal (a script's direct application to Sora): ours, a share of KH1's max
//!   HP by the ATKP row's power (`CURE_SHARE`), as KH1 has no KH2 Magic stat to scale it by.
//! - Final Form casts the same way (the user: "lets try to incorporate our rules, logic, and
//!   process from wisdom forms' magic, and bring it to final form, with final form's
//!   animations"): `form::casting_form`. The same MAGC rows and spell files (KH2 has none of
//!   Final's own), its moveset's motions under their ids (`P_EX100_ULTF`'s E36x / E37x), at its
//!   magic sub (+ 30, `form::Kind::magic_sub`). No shots: Final's frame triggers 1 are its own
//!   elements (`kh2fx::form_event`).
//! - Master Form casts the same way too (the user: "lets take the rules and procedures we've done
//!   for wisdom form"): its own casts under the MAGC ids (`P_EX100_TRIF`'s D370..D374, one clip
//!   for the ground, the finisher and the air, both its Keyblades' hits in them), at its sub (+ 20).

use super::reflect::{flip, world_now, KhWorld, SORA};
use super::{combat, log, pose};
use character_core::{CastMotions, Character, Output};
use glam::{Mat4, Quat, Vec3};
use kh2_formats::battle::{Atkp, Battle};
use kh2_magic::{Event, Magic, SpellFile};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

pub const SPELL_NAMES: [&str; 4] = ["Fire", "Blizzard", "Thunder", "Cure"];

/// Base Sora casts KH2's magic for a spell cast now, on the ground or in the air (the move
/// styles' Magic / Aerial Magic set to KH2, `styles`; not in a Drive Form, which keep their own).
pub fn base_kh2(air: bool) -> bool {
    use super::styles::{kh2, Kind};
    !super::form::in_form() && kh2(if air { Kind::AirMagic } else { Kind::Magic })
}

/// The KH1 spell KH1 casts now (Gravity, Stop, Aero) is drawn with base Sora's KH2 cast motion
/// on its clock, as Wisdom's KH1 spells are (`takeover::command_started`, `pose::follow`): the
/// Magic styles at KH2 in base form (the user: "KH1 spell, KH2 animation").
pub static BASE_KH1_CAST: AtomicBool = AtomicBool::new(false);

/// Whose spells these are, for the log.
fn who() -> &'static str {
    match super::form::casting_form() {
        Some(super::form::Kind::Wisdom) => "Wisdom",
        Some(super::form::Kind::Master) => "Master",
        Some(super::form::Kind::Final) => "Final",
        _ => "KH2 magic",
    }
}
const FILE_NAMES: [&str; 4] = ["FIRE", "BLIZZARD", "THUNDER", "CURE"];
/// `kh2fx` packages (1-based, its `level` argument): 1..3 Reflect, 4 Wisdom's archive, then
/// each spell's three levels.
const FIRST_SPELL_PACKAGE: u8 = super::kh2fx::WISDOM + 1;
pub const LAST_PACKAGE: u8 = FIRST_SPELL_PACKAGE + 4 * 3 - 1;
/// Hit-record keys of the form's attacks (apart from Reflect's and the swings').
const KEY: u64 = 1 << 59;
/// The farthest an unlocked shot looks for an enemy, and how far off his facing.
const AUTO_RANGE: f32 = 1500.0;
const AUTO_CONE: f32 = 0.5;
/// The arrival's light on his body (Wisdom's elements 1..12, the form's colour, 11 / 12 looping)
/// stays this many frames, whatever he does meanwhile (the user: "when sora first enters his drive
/// form whatever animation he's in there should be an effect"; ours: KH2's end isn't traced).
const ENVELOPE_FRAMES: u32 = 90;
/// The form's own aura (Wisdom's element 100, looping: the sparkles at his feet and the trails
/// round him), started by the arrival and kept until the revert (the user: "wisdom is missing
/// its effects around sora"; s192 had ended it with the arrival).
const AURA: u16 = 100;
/// How long a chosen spell waits for KH2's move to allow a cast.
const WAIT_S: f32 = 5.0;
/// Cure's heal: the ATKP row's power / this, of KH1's max HP (ours).
const CURE_SHARE: f32 = 3000.0;

pub fn spell_package(spell: u8, level: u8) -> u8 {
    FIRST_SPELL_PACKAGE + spell * 3 + (level - 1)
}

pub fn package_spell(package: u8) -> Option<(u8, u8)> {
    (FIRST_SPELL_PACKAGE..=LAST_PACKAGE).contains(&package).then(|| ((package - FIRST_SPELL_PACKAGE) / 3, (package - FIRST_SPELL_PACKAGE) % 3 + 1))
}

fn magc(spell: u8, level: u8) -> Option<&'static kh2_formats::battle::Magc> {
    pose::kh2()?.battle.magc.iter().find(|m| m.id == spell && m.level == level && m.world == 0)
}

/// A spell's file from the user's KH2 install (read once).
pub fn spell_file(spell: u8, level: u8) -> Option<Arc<Vec<u8>>> {
    static FILES: [OnceLock<Option<Arc<Vec<u8>>>>; 12] = [const { OnceLock::new() }; 12];
    if spell > 3 || !(1..=3).contains(&level) {
        return None;
    }
    FILES[(spell * 3 + level - 1) as usize]
        .get_or_init(|| {
            let name = magc(spell, level).map_or(format!("magic/{}_{level}.mag", FILE_NAMES[spell as usize]), |m| m.filename.clone());
            let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
            let r = (|| -> Result<Vec<u8>, String> {
                let install = kh2_data::Install::locate(&exe).map_err(|e| e.to_string())?;
                let src = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
                install.read_any(src, &name).map_err(|e| e.to_string())
            })();
            match r {
                Ok(b) => Some(Arc::new(b)),
                Err(e) => {
                    log(&format!("KH2 spells: {name} not read ({e})"));
                    None
                }
            }
        })
        .clone()
}

/// The form's effect elements' sounds (`kh2sound`): Wisdom's archive and the spells', by package.
pub fn element_sounds(kh2: &pose::Kh2) -> Vec<(u8, u16, i32)> {
    let mut out = Vec::new();
    let mut add = |package: u8, bytes: &[u8]| {
        if let Some(pax) = SpellFile::bar_entry(bytes, 18).and_then(|p| kh2_formats::pax::Pax::parse(&p).ok()) {
            out.extend(pax.elements.iter().map(|e| (package, e.id, e.sound)));
        }
    };
    if let Some(a) = kh2.wisdom.as_ref().and_then(|f| f.archive.as_ref()) {
        add(super::kh2fx::WISDOM, a);
    }
    // (Valor's effect elements' sounds: `kh2fx::form_event`.)
    if let Some(a) = kh2.valor.as_ref().and_then(|f| f.archive.as_ref()) {
        add(super::kh2fx::VALOR, a);
    }
    if let Some(a) = kh2.final_form.as_ref().and_then(|f| f.archive.as_ref()) {
        add(super::kh2fx::FINAL, a);
    }
    // (Anti Form's, the secret code's.)
    if let Some(a) = kh2.anti.as_ref().and_then(|f| f.archive.as_ref()) {
        add(super::kh2fx::ANTI, a);
    }
    // (Limit Form's.)
    if let Some(a) = kh2.limit.as_ref().and_then(|f| f.archive.as_ref()) {
        add(super::kh2fx::LIMIT, a);
    }
    // (Master Form's.)
    if let Some(a) = kh2.master.as_ref().and_then(|f| f.archive.as_ref()) {
        add(super::kh2fx::MASTER, a);
    }
    // (Also for base Sora's KH2 magic, `base_kh2`, which the Config menu switches in game.)
    {
        for spell in 0..4u8 {
            for level in 1..=3u8 {
                if let Some(b) = spell_file(spell, level) {
                    add(spell_package(spell, level), &b);
                }
            }
        }
    }
    out
}

struct Fresh {
    spell: u8,
    level: u8,
    finisher: bool,
    motion: u16,
}

struct Runtime {
    magic: Magic,
    wisd: Option<usize>,
    /// Each (spell, level)'s script in `magic`, once parsed (`None`: not usable).
    files: HashMap<(u8, u8), Option<usize>>,
    /// A script's effect package (`kh2fx`), by its index in `magic`.
    package: HashMap<usize, u8>,
    /// Script effect id → our drawn effect, and whether its end was passed on.
    fx: HashMap<u64, (Option<u32>, bool)>,
    queued: Vec<i16>,
    /// The arrival's effects and frames since it (their loops end `ENVELOPE_FRAMES` later; the
    /// form's aura, `AURA`, stays for the form).
    arrival: Vec<u64>,
    arrival_age: u32,
    fresh: Option<Fresh>,
    casting: Option<u16>,
    fired: u32,
    hits: u32,
}

static RT: Mutex<Option<Runtime>> = Mutex::new(None);
/// A chosen spell, when, and the KH2 move it was chosen in (it waits only while that plays).
static WAITING: Mutex<Option<(u8, Instant, u32)>> = Mutex::new(None);
static NO_WISD: AtomicBool = AtomicBool::new(false);

fn runtime(g: &mut Option<Runtime>) -> &mut Runtime {
    g.get_or_insert_with(|| {
        let mut magic = Magic::new();
        let mut package = HashMap::new();
        let wisd = pose::kh2().and_then(|k| k.wisdom.as_ref()).and_then(|f| {
            let made = (|| {
                let script = f.script.as_ref().ok_or("no script in its model")?;
                let pax = f.archive.as_ref().and_then(|a| SpellFile::bar_entry(a, 18)).ok_or("no effects in its archive")?;
                SpellFile::from_parts("P_EX100_MAGF", script, &pax)
            })();
            match made {
                Ok(file) => {
                    let i = magic.add_spell(file);
                    package.insert(i, super::kh2fx::WISDOM);
                    log("Wisdom Form's script ready (its shots)");
                    Some(i)
                }
                Err(e) => {
                    if !NO_WISD.swap(true, Ordering::Relaxed) {
                        log(&format!("Wisdom Form fires no shots: {e}"));
                    }
                    None
                }
            }
        });
        Runtime { magic, wisd, files: HashMap::new(), package, fx: HashMap::new(), queued: Vec::new(), arrival: Vec::new(), arrival_age: 0, fresh: None, casting: None, fired: 0, hits: 0 }
    })
}

/// A spell's script in the runtime (parsed on first use).
fn spell_index(rt: &mut Runtime, spell: u8, level: u8) -> Option<usize> {
    if let Some(i) = rt.files.get(&(spell, level)) {
        return *i;
    }
    let name = format!("{}_{level}.mag", FILE_NAMES[spell as usize]);
    let i = spell_file(spell, level).and_then(|b| match SpellFile::parse(&name, &b) {
        Ok(f) => {
            let i = rt.magic.add_spell(f);
            rt.package.insert(i, spell_package(spell, level));
            Some(i)
        }
        Err(e) => {
            log(&format!("KH2 spells: {name} not usable ({e})"));
            None
        }
    });
    rt.files.insert((spell, level), i);
    i
}

/// A frame trigger 1 of the form's motion this frame (`takeover`): a shot (or the arrival's).
pub fn effect(n: i16) {
    if let Ok(mut g) = RT.lock() {
        runtime(&mut g).queued.push(n);
    }
}

/// Out of the form: its effects put out, its casts ended.
pub fn end_all() {
    if let Ok(mut w) = WAITING.lock() {
        *w = None;
    }
    let Ok(mut g) = RT.lock() else { return };
    let Some(rt) = g.as_mut() else { return };
    rt.queued.clear();
    rt.arrival.clear();
    rt.fresh = None;
    rt.casting = None;
    rt.magic.cancel(SORA);
    for e in rt.magic.effects.iter_mut() {
        e.kill();
    }
    for h in rt.fx.values().filter_map(|v| v.0) {
        super::kh2fx::spell_kill(h);
    }
    rt.fx.clear();
}

/// A KH1 spell chosen (`combat::command`, magic `spell`): in the form, KH2's; in base form,
/// KH2's when the Magic style (ground or air, as he is now) is KH2's (`base_kh2`). Returns
/// whether it was taken (else KH1 casts its own).
pub unsafe fn spell_chosen(actor: usize, spell: u8) -> bool {
    // (Wisdom: Fire, Blizzard, Thunder; Cure is KH1's own on the form's Cure motion, as Gravity,
    // Stop and Aero are: the user, "wisdom form currently casts kh2 cure, it needs to be kh1 cure
    // with kh2s cures animation". Base Sora's KH2 magic: Cure too, KH2's.)
    // (Final Form as Wisdom: `form::casting_form`.)
    if !super::takeover::kh2_has() {
        return false;
    }
    let form = spell <= 2 && super::form::casting_form().is_some() && super::settings::get().drive;
    let base = spell <= 3 && base_kh2(super::takeover::kh2_airborne());
    if !(form || base) {
        return false;
    }
    let level = super::reflect::kh1_spell_level(spell);
    if spell_file(spell, level).is_none() || magc(spell, level).is_none() {
        return false;
    }
    let cost = super::reflect::kh1_spell_cost(spell);
    if !super::reflect::can_pay(actor, cost) {
        super::reflect::menu_sound(4);
        log(&format!("{} {} refused: can't pay {cost}", who(), SPELL_NAMES[spell as usize]));
        return true;
    }
    if let Ok(mut w) = WAITING.lock() {
        *w = Some((spell, Instant::now(), super::takeover::KH2_MOTION.load(Ordering::Relaxed)));
    }
    log(&format!("{} {} chosen (level {level}, cost {cost})", who(), SPELL_NAMES[spell as usize]));
    true
}

/// A spell is waiting for its window (`takeover`).
pub fn waiting() -> bool {
    WAITING.try_lock().map_or(false, |w| w.is_some())
}

/// Before KH2's step: a chosen spell's cast, as soon as KH2's move allows one.
pub unsafe fn before_step(actor: usize, ch: &mut Character) {
    let Ok(mut w) = WAITING.try_lock() else { return };
    let Some((spell, when, motion)) = *w else { return };
    if !ch.can_cast() {
        // (No buffer, s249: dropped at once; it waited for the move's next window:)
        // if !super::takeover::same_move(motion) || !ch.magic_later() || when.elapsed().as_secs_f32() > WAIT_S {
        let _ = (when, WAIT_S, super::takeover::same_move(motion) && ch.magic_later());
        {
            *w = None;
            log(&format!("{} {}: dropped, the move it was chosen in (motion {motion}) ended without a window for it", who(), SPELL_NAMES[spell as usize]));
        }
        return;
    }
    *w = None;
    let level = super::reflect::kh1_spell_level(spell);
    let cost = super::reflect::kh1_spell_cost(spell);
    if !super::reflect::can_pay(actor, cost) {
        super::reflect::menu_sound(4);
        return;
    }
    let Some(m) = magc(spell, level) else { return };
    let pick = |v: i16| (v >= 0).then_some(v as u16);
    let motions = CastMotions { ground: pick(m.ground_motion).unwrap_or(56), air: pick(m.air_motion).unwrap_or(58), finisher: pick(m.finish_motion) };
    let Some(motion) = ch.cast(motions) else {
        log(&format!("{} {}: no cast motion ({motions:?})", who(), SPELL_NAMES[spell as usize]));
        return;
    };
    super::reflect::pay_mp(actor, cost);
    let finisher = Some(motion) == motions.finisher && motions.finisher != Some(motions.ground);
    if let Ok(mut g) = RT.lock() {
        runtime(&mut g).fresh = Some(Fresh { spell, level, finisher, motion });
    }
    let v = if finisher { m.voice_finisher } else { m.voice };
    if v != 255 {
        super::kh2sound::voice(v as i16);
    }
    log(&format!("{} {} (level {level}) cast: motion {motion}{}", who(), SPELL_NAMES[spell as usize], if finisher { " (combo finisher)" } else { "" }));
}

/// The form's own cast motion for a KH1 spell it has no KH2 version of (Gravity, Stop, Aero):
/// Thunder's (MAGC) at that level, ground or air (the user: "the same logic as thunder").
/// (MAGC gives a motion id; the form's moveset its clip: Wisdom's C36x / C37x, Final's E36x /
/// E37x, base Sora's A36x / A37x.)
pub fn kh1_spell_motion(spell: u8, level: u8, air: bool) -> u16 {
    // Gravity on Thunder's motion; Cure, Stop and Aero on Cure's (`pose::follow` shows the same).
    let thunder = spell == 4;
    let m = magc(if thunder { 2 } else { 3 }, level.clamp(1, 3));
    let pick = |v: i16| (v >= 0).then_some(v as u16);
    if air {
        m.and_then(|m| pick(m.air_motion)).unwrap_or(if thunder { 64 } else { 67 })
    } else {
        m.and_then(|m| pick(m.ground_motion)).unwrap_or(if thunder { 62 } else { 65 })
    }
}

fn row(battle: &Battle, atk: u16, sub: u8, count: u8) -> Option<&Atkp> {
    battle.attack_for_level(atk, sub, count)
}

/// An unlocked shot's target: the nearest enemy within `AUTO_RANGE` in front of him (also Limit
/// Form's limits' lock-on target with nothing locked, `limits`).
pub(super) fn auto_target(w: &KhWorld, facing: f32) -> Option<u32> {
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

/// An unbound effect's root where the script has it (y up, our axes) → KH2's world (y down),
/// turned along its direction.
fn place(pos: Vec3, dir: Vec3) -> Mat4 {
    let r = Quat::from_rotation_arc(Vec3::Z, dir.try_normalize().unwrap_or(Vec3::Z));
    flip() * Mat4::from_rotation_translation(r, pos) * flip()
}

/// Sora healed by a script (Cure): KH1's HP.
unsafe fn heal(actor: usize, power: u16) -> Option<(i32, i32)> {
    let stats = pose::ptr(std::ptr::read_unaligned((actor + 0x6C) as *const u32));
    if stats < 0x10000 {
        return None;
    }
    let hp = std::ptr::read_unaligned((stats + 0x3C) as *const i32);
    let max = std::ptr::read_unaligned((stats + 0x40) as *const i32);
    if hp <= 0 || max < hp || max <= 0 {
        return None;
    }
    let add = ((max as f32 * power as f32 / CURE_SHARE).ceil() as i32).max(1);
    let now = (hp + add).min(max);
    std::ptr::write_unaligned((stats + 0x3C) as *mut i32, now);
    Some((hp, now))
}

/// After KH2's step and the swings: the form's scripts' frame (casts, shots, effects, hits).
/// Returns whether one of the form's shots (its model script's attacks, not a spell's) landed on
/// an enemy this frame; the caller tells the controller, as a swing's own hit does
/// (`Character::register_hit`). Wisdom's swings have no hit windows: their shots are their hits
/// (s304, the user: without Combo Master "it just does the first hit over and over", as no
/// shot's hit reached the controller and every swing counted as a miss).
pub unsafe fn shots_step(actor: usize, out: &Output, airborne: bool, strikes: &mut combat::Strikes, battle: &Battle, lock: Option<u32>) -> bool {
    let Ok(mut g) = RT.try_lock() else { return false };
    let Some(rt) = g.as_mut() else { return false };
    if rt.queued.is_empty() && rt.fresh.is_none() && rt.casting.is_none() && rt.magic.casts.is_empty() && rt.magic.effects.is_empty() && rt.fx.is_empty() {
        return false;
    }
    let mut shot_hit = false;
    let mut world = world_now(actor, out, airborne, strikes, lock, 0x51A7_0000 ^ rt.fired.wrapping_mul(2_654_435_761));
    if world.target.is_none() {
        world.target = auto_target(&world, out.facing);
    }
    // A spell cast this frame (trigger 20), its release (21) at the motion's magic-cast event.
    if let Some(f) = rt.fresh.take() {
        if let Some(file) = spell_index(rt, f.spell, f.level) {
            // (Wisdom's subs from 10, Final's from 30; base Sora's from 0, `form::magic_sub`.)
            let sub = f.level as i32 * 2 - 2 + f.finisher as i32 + super::form::magic_sub();
            let magi = pose::kh2().and_then(|k| k.magi.get(sub).map(|r| r.to_vec())).unwrap_or_else(|| vec![0; kh2_formats::pref::MagiTable::RECORD]);
            match rt.magic.cast(file, SORA, f.spell, sub, &magi, &mut world) {
                Ok(_) => rt.casting = Some(f.motion),
                Err(e) => log(&format!("{} {}'s script didn't start ({e})", who(), SPELL_NAMES[f.spell as usize])),
            }
        }
    }
    if let Some(m) = rt.casting {
        let fired = out.motion == m && out.events.iter().any(|e| matches!(e, character_core::EventKind::MagicCast(x) if *x as u16 == out.motion));
        if fired || out.motion != m {
            rt.casting = None;
            // (Cut off before its release: released anyway, as the spell was paid for.)
            rt.magic.release(SORA, &mut world);
        }
    }
    // The shots (and the arrival's light).
    if let Some(file) = rt.wisd {
        for n in std::mem::take(&mut rt.queued) {
            let before = rt.magic.effects.iter().map(|e| e.id).max().unwrap_or(0);
            match rt.magic.object_effect(file, SORA, n, &mut world) {
                Ok(_) => {
                    if out.motion == super::form::ARRIVE_MOTION {
                        rt.arrival.extend(rt.magic.effects.iter().filter(|e| e.id > before && e.element.id != AURA).map(|e| e.id));
                        rt.arrival_age = 0;
                    } else {
                        rt.fired += 1;
                        if rt.fired <= 30 {
                            log(&format!("Wisdom shot (trigger {n}, motion {}), target {:?}", out.motion, world.target));
                        }
                    }
                }
                Err(e) => log(&format!("Wisdom's script didn't take trigger {n}: {e}")),
            }
        }
    } else {
        rt.queued.clear();
    }
    if !rt.arrival.is_empty() {
        rt.arrival_age += 1;
        if rt.arrival_age >= ENVELOPE_FRAMES {
            for e in rt.magic.effects.iter_mut().filter(|e| rt.arrival.contains(&e.id)) {
                e.loop_end();
            }
            rt.arrival.clear();
        }
    }
    for e in rt.magic.update(&mut world).into_iter().chain(rt.magic.take_events()) {
        match e {
            Event::Missing(n) => {
                static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                if N.fetch_add(1, Ordering::Relaxed) < 20 {
                    log(&format!("{}'s scripts used {n} (not modelled)", who()));
                }
            }
            Event::Error(e) => log(&format!("{}'s script error: {e}", who())),
            Event::Strike { attack, target } if target == SORA => {
                let Some(a) = rt.magic.attack_mut(attack) else { continue };
                let (atk, sub) = (a.atk, a.sub);
                let Some(r) = row(battle, atk, sub, 0) else { continue };
                if matches!(combat_core::AttackKind::from_id(r.kind), combat_core::AttackKind::Cure | combat_core::AttackKind::CureStrong) {
                    if let Some((from, to)) = heal(actor, r.power) {
                        log(&format!("{} Cure: ATKP {atk} sub {sub} (power {}): HP {from} -> {to}", who(), r.power));
                    }
                }
            }
            Event::Strike { .. } => {}
        }
    }
    // Effects: bound ones follow Sora's bones; unbound ones go where the script has them.
    let mut seen = Vec::new();
    for e in &rt.magic.effects {
        seen.push(e.id);
        let package = rt.package.get(&e.spell).copied().unwrap_or(super::kh2fx::WISDOM);
        let placed = e.owner.is_none().then(|| place(e.pos, e.dir));
        match rt.fx.get(&e.id) {
            None => {
                let h = if e.alive() { super::kh2fx::spell_start(package, e.element.id) } else { None };
                if let (Some(h), Some(at)) = (h, placed) {
                    super::kh2fx::spell_place(h, at);
                }
                if e.alive() {
                    super::kh2sound::spell_element(package, e.element.id);
                }
                rt.fx.insert(e.id, (h, false));
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
                } else if (e.ended_at.is_some() || !e.alive()) && !ended {
                    super::kh2fx::spell_end(h);
                    rt.fx.insert(e.id, (Some(h), true));
                }
            }
            Some((None, _)) => {}
        }
    }
    rt.fx.retain(|id, (h, _)| seen.contains(id) || h.is_some_and(super::kh2fx::spell_running));
    // The attacks against KH1's enemies; one that hits is told so.
    // (Fire, Blizzard and Thunder land with KH1's own data for the spell at its level: its
    // element; the shots stay the swings'.)
    let package = &rt.package;
    let attacks: Vec<combat::SpellAttack> = rt
        .magic
        .attacks
        .iter()
        .filter(|a| !a.freed && a.owner == SORA)
        .filter_map(|a| {
            let kh1_data = package.get(&a.spell).and_then(|&p| package_spell(p)).filter(|(spell, _)| *spell <= 2).and_then(|(spell, level)| combat::kh1_spell_data(spell, level));
            let first = row(battle, a.atk, a.sub, 0)?;
            if !combat_core::AttackKind::from_id(first.kind).deals_damage() {
                return None;
            }
            let shapes = a.shapes();
            (!shapes.is_empty()).then(|| combat::SpellAttack { kh1_data, key: a.id | KEY, group: if first.combo_group != 0 { first.combo_group as i64 } else { a.group }, atk: a.atk, sub: a.sub, shapes })
        })
        .collect();
    if !attacks.is_empty() {
        let hits = strikes.spell_strike(actor, &attacks, &|atk, sub, count| row(battle, atk, sub, count).cloned());
        for (key, atk, foe, dealt) in hits {
            rt.hits += 1;
            if let Some(a) = rt.magic.attacks.iter_mut().find(|a| a.id | KEY == key) {
                a.hit = true;
                // (A shot of the form's own script: the swing connected. A spell's hit isn't
                // needed: a cast already takes its combo step as a connected one.)
                shot_hit |= Some(a.spell) == rt.wisd;
            }
            if rt.hits <= 60 {
                log(&format!("{}: ATKP {atk} hit {foe:#x} -> damage {dealt:?}", who()));
            }
        }
    }
    for a in rt.magic.freed().filter(|a| a.owns_id) {
        strikes.spell_forget(a.id | KEY);
    }
    shot_hit
}
