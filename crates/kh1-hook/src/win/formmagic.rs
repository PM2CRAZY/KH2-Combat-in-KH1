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

use super::reflect::{flip, world_now, KhWorld, SORA};
use super::{combat, log, pose};
use character_core::{CastMotions, Character, Output};
use glam::{Mat4, Quat, Vec3};
use kh2_formats::battle::{Atkp, Battle};
use kh2_magic::{Event, Magic, SpellFile};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

pub const SPELL_NAMES: [&str; 4] = ["Fire", "Blizzard", "Thunder", "Cure"];

/// Base Sora casts KH2's magic for a spell cast now, on the ground or in the air (the move
/// styles' Magic / Aerial Magic set to KH2, `styles`; not in a Drive Form, which keep their own).
pub fn base_kh2(air: bool) -> bool {
    use super::styles::{kh2, Kind};
    !super::form::in_form() && kh2(if air { Kind::AirMagic } else { Kind::Magic })
}

/// Base Sora's Cure at the Magic styles KH2 is KH2's (the Config menu's Cure row, `styles::Kind::
/// Cure`); else KH1's own Cure on KH2's Cure cast motion, as Gravity / Stop / Aero.
pub fn kh2_cure() -> bool {
    super::styles::kh2(super::styles::Kind::Cure)
}

/// Base Sora casts KH2's `spell` (Fire 0 .. Cure 3) with the Magic styles at KH2 (`base_kh2`):
/// Fire, Blizzard and Thunder, and Cure when the Cure row is KH2's.
pub fn kh2_spell(spell: u8) -> bool {
    spell <= 2 || spell == 3 && kh2_cure()
}

/// The KH1 spell KH1 casts now (Gravity, Stop, Aero) is drawn with base Sora's KH2 cast motion
/// on its clock, as Wisdom's KH1 spells are (`takeover::command_started`, `pose::follow`): the
/// Magic styles at KH2 in base form (the user: "KH1 spell, KH2 animation").
pub static BASE_KH1_CAST: AtomicBool = AtomicBool::new(false);

/// The KH1 spell `spell` (Gravity 4, Stop 5, Aero 6) is cast KH1's whole way even with the Magic
/// styles at KH2 (the debug menu's KH1 Spells page, the ini's `kh1_cast_*`; the tester: "kh2
/// effects took over some of the native magic effects ... can we have a toggle in the debug
/// menu?"): its own cast motion, its clock and its own effects, as with the styles at KH1. KH2
/// has no Gravity, Stop or Aero; what KH2 brings to them is its cast motion (Thunder's for
/// Gravity, Cure's for Stop and Aero, `kh1_spell_motion`), on whose clock KH1's motion is cut.
pub fn kh1_cast(spell: u8) -> bool {
    if !(4..=6).contains(&spell) {
        return false;
    }
    let i = (spell - 4) as usize;
    match KH1_CAST_SET[i].load(Ordering::Relaxed) {
        0 => super::settings::get().kh1_casts[i],
        v => v == 2,
    }
}

/// The debug menu's choice this session (0: the ini's, 1: KH2's cast motion, 2: KH1's whole cast).
static KH1_CAST_SET: [std::sync::atomic::AtomicU8; 3] = [const { std::sync::atomic::AtomicU8::new(0) }; 3];

/// The debug menu's KH1 Spells row: `spell` (4..6) cast KH1's whole way or not, kept in the ini.
pub fn set_kh1_cast(spell: u8, on: bool) {
    if !(4..=6).contains(&spell) {
        return;
    }
    let i = (spell - 4) as usize;
    KH1_CAST_SET[i].store(if on { 2 } else { 1 }, Ordering::Relaxed);
    super::settings::save(super::settings::KH1_CAST_KEYS[i], if on { "on" } else { "off" });
    log(&format!("debug menu: KH1 spell {spell} casts {}", if on { "KH1's whole way" } else { "on KH2's cast motion" }));
}

/// Whose spells these are, for the log.
fn who() -> &'static str {
    if super::form::in_wisdom() {
        "Wisdom"
    } else {
        "KH2 magic"
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
/// `Runtime::casting`'s mark for a cast motion seen playing, and how many frames one may wait
/// for it (ours).
const SEEN: u8 = u8::MAX;
const NOT_SEEN_FRAMES: u8 = 4;
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

/// A spell's file in KH2's install (its MAGC row's, else `magic/<SPELL>_<n>.mag`).
pub fn spell_path(spell: u8, level: u8) -> String {
    magc(spell, level).map_or(format!("magic/{}_{level}.mag", FILE_NAMES[spell as usize]), |m| m.filename.clone())
}

/// A spell's file from the user's KH2 install (read once).
pub fn spell_file(spell: u8, level: u8) -> Option<Arc<Vec<u8>>> {
    static FILES: [OnceLock<Option<Arc<Vec<u8>>>>; 12] = [const { OnceLock::new() }; 12];
    if spell > 3 || !(1..=3).contains(&level) {
        return None;
    }
    FILES[(spell * 3 + level - 1) as usize]
        .get_or_init(|| {
            let name = spell_path(spell, level);
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
    /// The cast motion waiting for its magic-cast event (the release), and the frames it hasn't
    /// been seen playing yet (`SEEN` once it has: a cast in KH1's cast state starts its motion at
    /// KH1's next animation step).
    casting: Option<(u16, u8)>,
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
    if !super::takeover::kh2_has() {
        return false;
    }
    let wisdom = spell <= 2 && super::form::in_wisdom() && super::settings::get().drive;
    let base = kh2_spell(spell) && base_kh2(super::takeover::kh2_airborne());
    if !(wisdom || base) {
        return false;
    }
    let level = super::reflect::kh1_spell_level(spell);
    if spell_file(spell, level).is_none() || magc(spell, level).is_none() {
        return false;
    }
    let cost = super::reflect::kh1_spell_cost(spell);
    if !affordable(actor, spell, cost) {
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

/// Something of the scripts' still runs or plays (`takeover::native_spells` runs their frame).
pub fn busy() -> bool {
    RT.try_lock().is_ok_and(|g| g.as_ref().is_some_and(|rt| !rt.queued.is_empty() || rt.fresh.is_some() || rt.casting.is_some() || !rt.magic.casts.is_empty() || !rt.magic.effects.is_empty() || !rt.fx.is_empty()))
}

// ------------------------------------------------- KH2's spells in KH1's own cast state

/// KH1's spell start (Steam `0x14028F140 (actor, spell, target)`, Checked (decomp) and (exe)):
/// KH1's command step calls it once it has asked for the cast's request (and queued what follows
/// it); it starts the spell's `.mag` script (KH1's script machine, a process of the list at
/// `0x142D22D08`, `0x14028F5E0`), which makes the spell's attack objects (syscall 23,
/// `0x1402B2DC0`) and pays its MP (syscall 47, `0x1402A5B60` with the attack data's `+0x30`).
/// `docs/kh1-magic-native.md`. Its prologue: `mov [rsp+8], rbx; mov [rsp+10h], rsi; push rdi;
/// sub rsp, 40h`.
const SPELL_START_STEAM: usize = 0x28F140;
const SPELL_START_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x40];
static ORIG_SPELL_START: AtomicUsize = AtomicUsize::new(0);
static NATIVE_HOOKED: AtomicBool = AtomicBool::new(false);
/// KH1's motion queue on an actor: its count, then entries of 0x10 (the request first).
const QUEUE_COUNT: usize = 0x178;
const QUEUE: usize = 0x17C;
const QUEUE_ENTRY: usize = 0x10;
/// Fire's second part on the ground and in the air (`kh2slots::CASTS`), queued by KH1 after its
/// first (0x36 / 0x83).
const FIRE_SECOND: [(u32, u32); 2] = [(0x36, 0x3D), (0x83, 0x8A)];

/// The hook (Steam only: the address is Steam's).
pub unsafe fn install() {
    if super::addr::Build::current() != super::addr::Build::Steam {
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    if super::hook::install(base + SPELL_START_STEAM, &SPELL_START_PROLOGUE, spell_start as *const () as usize, &ORIG_SPELL_START, "KH1's spell start (KH2 magic in KH1's cast state)").is_some() {
        NATIVE_HOOKED.store(true, Ordering::Relaxed);
    }
}

/// KH2's `spell` (Fire 0 .. Cure 3) is cast in KH1's own cast state now (`air`: KH1's air cast):
/// base Sora in KH1's own states (`shared`), the Magic / Aerial Magic style KH2's, KH1's spell
/// start hooked, our slot of KH2's cast motion built for KH1's request (`kh2slots::CASTS`), and
/// the spell's KH2 file and MAGC row there. Else KH2's controller borrows him for it, as before.
pub unsafe fn native(spell: u8, air: bool) -> bool {
    if !kh2_spell(spell) || !NATIVE_HOOKED.load(Ordering::Relaxed) || !base_kh2(air) || !super::shared::active() {
        return false;
    }
    let request = if air { 0x83 } else { 0x36 } + spell as usize;
    let level = super::reflect::kh1_spell_level(spell);
    super::kh2slots::cast_motion(request).is_some() && spell_file(spell, level).is_some() && magc(spell, level).is_some()
}

/// Before KH1's command step casts KH2's `spell` natively (`combat::command`): its ground cast
/// requests (and Fire's second part, which KH1 plays for a Fire out of a swing) pointed at KH2's
/// finisher cast when this cast is the combo's finisher by KH2's rule (`Character::cast` /
/// `spell_slot`: a spell takes the combo's next step, the finisher when that step reaches the
/// combo's length), on KH1's own combo: live while the motion playing has KH1's combo window
/// (`+0x1CC` bit 22, which KH1's Attack step tests, `0x1402A7510`), the step counted on from
/// KH1's count (combat info `+0x41`) when the last swing connected (`+0x50`) or Combo Master is
/// on, as KH1's Attack step counts and KH2's `swing_counts`, against KH1's ground combo length
/// (stats `+0xD4`, `combomods`). In the air KH2 has no finisher cast.
pub unsafe fn pick_cast(actor: usize, spell: u8, air: bool) {
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    let stats = pose::ptr(rd::<u32>(actor + 0x6C));
    // The cast's step of the combo (KH2's `spell_slot`: in the air every step counts, KH2's
    // `swing_counts`), kept for its start (`kh1_spell_start`, the tester's answer 1 (a)).
    let step = (info >= 0x10000 && stats >= 0x10000).then(|| {
        let live = rd::<u32>(actor + 0x1CC) >> 22 & 1 != 0;
        let count = rd::<u8>(info + 0x41) as u32;
        let max = rd::<u8>(stats + if air { 0xD5 } else { 0xD4 }).max(1) as u32;
        let counts = air || rd::<u32>(info + 0x50) != 0 || super::takeover::with_abilities(|a| a.combo_master) == Some(true);
        let step = if !live { 1 } else if counts { count + 1 } else { count.max(1) };
        (step.min(255) as u8, step >= max)
    });
    CAST_STEP.store(step.map_or(0, |(s, last)| s as u32 | (last as u32) << 8 | 1 << 16), Ordering::Relaxed);
    // (KH2 has no finisher cast in the air: the air cast's own motion, the step still the last.)
    let finisher = !air && step.is_some_and(|s| s.1);
    let ground = 0x36 + spell as usize;
    let m = super::kh2slots::pick_cast(ground, finisher);
    if spell == 0 {
        super::kh2slots::pick_cast(0x3D, finisher);
    }
    if finisher {
        log(&format!("KH2 magic {}: the combo's finisher, KH2's finisher cast {m:?}", SPELL_NAMES[spell as usize]));
    }
}

/// The step `pick_cast` worked out for the native cast KH1's command step is about to start:
/// bits 0..7 the step, bit 8 the combo's last, bit 16 set.
static CAST_STEP: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// A native cast takes its step of KH1's combo (the tester's answer 1 (a), KH2's `spell_slot` /
/// `take_spell_slot`): KH1's count (combat info `+0x41`) at the cast's step and its swing
/// "connected" (`+0x50`: the step after a spell counts on from it, as after a connected swing in
/// KH2), so KH1's Attack step in the cast's combo window (`kh2slots::cast_follow`) counts on from
/// it (`0x1402A7510`: one more inside the window when `+0x50` is set, Checked (decomp)). The
/// combo's last step (a finisher cast, or the step at the length in the air) ends it: the count
/// set to 0, so an Attack counted on there starts at 1 (a new combo, as KH2's after its finisher).
unsafe fn take_step(actor: usize) {
    let v = CAST_STEP.swap(0, Ordering::Relaxed);
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    if v & 1 << 16 == 0 || info < 0x10000 {
        return;
    }
    let (step, last) = ((v & 0xFF) as u8, v >> 8 & 1 != 0);
    wr(info + 0x41, if last { 0u8 } else { step });
    wr(info + 0x50, 1u32);
    log(&format!("KH2 magic: the cast takes step {step} of KH1's combo{}", if last { " (its last: the combo ends)" } else { "" }));
}

/// KH1's control step with base Sora in its own states (`takeover::shared_control`, before it
/// reads whether he can act): a fresh jump or Square in our cast slot's cut window
/// (`kh2slots::cast_cut_open`, KH2's range id 14) ends the cast's action flag (`+0` bit 2, as
/// KH1's own move ends do), so KH1's jump, or the Square move KH1 or a borrow of KH2's has for it,
/// starts this step; the stick alone doesn't (KH2's cast takes only presses there). Whether it
/// did.
pub unsafe fn cast_cut(actor: usize, jump: bool, square: bool) -> bool {
    static LAST: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
    let was = LAST.swap(jump as u8 | (square as u8) << 1, Ordering::Relaxed);
    let fresh = (jump && was & 1 == 0) || (square && was & 2 == 0);
    if !fresh || rd::<u32>(actor) & 4 == 0 || !super::kh2slots::cast_cut_open(actor) {
        return false;
    }
    wr(actor, rd::<u32>(actor) & !4);
    log(&format!("KH2 cast cut by {} in its reaction-command window (KH2 motion {:?})", if jump { "the jump" } else { "Square" }, super::kh2slots::playing_motion(actor)));
    true
}

unsafe extern "system" fn spell_start(actor: usize, spell: i32, target: usize) -> u64 {
    if (0..=3).contains(&spell) && kh1_spell_start(actor, spell as u8) {
        return 0;
    }
    let f: unsafe extern "system" fn(usize, i32, usize) -> u64 = std::mem::transmute(ORIG_SPELL_START.load(Ordering::Acquire));
    f(actor, spell, target)
}

/// KH1's command step starting `spell` for base Sora, the cast request asked for: with KH2's
/// slot on that request (`native`), KH2's spell instead of KH1's script: its MP (KH1's cost and
/// payment, as KH2's borrowed casts pay it, `before_step`), KH2's MAGC voice,
/// KH2's script started (trigger 20) on the next KH1 frame and released (21) at the cast motion's
/// magic-cast event at KH1's time (`takeover::native_spells`). Fire's second part, which KH1
/// queues after its first, is taken off the queue (KH2's one motion is the whole cast). Whether
/// it was taken (else KH1's own script runs).
unsafe fn kh1_spell_start(actor: usize, spell: u8) -> bool {
    if actor < 0x10000 || actor != super::takeover::sora_actor() || super::form::in_form() || !super::shared::active() {
        return false;
    }
    let count = rd::<u32>(actor + QUEUE_COUNT);
    if count == 0 || count > 8 {
        return false;
    }
    let request = rd::<u32>(actor + QUEUE) & 0xFFFF;
    let Some(&(own, _, _, air)) = super::kh2slots::CASTS.iter().find(|c| c.2 == request as usize) else { return false };
    if !native(spell, air) {
        return false;
    }
    // (Its pick, `pick_cast`: KH2's finisher cast, else the request's own.)
    let Some(motion) = super::kh2slots::cast_now(request as usize) else { return false };
    let level = super::reflect::kh1_spell_level(spell);
    let Some(m) = magc(spell, level) else { return false };
    let finisher = motion != own && m.finish_motion >= 0 && motion == m.finish_motion as u16;
    // (KH1's command step judged the cast usable with KH1's own check, `0x14028EFD0`, at KH2's
    // cost, `combat::command`.) KH2's cost scaled onto KH1's MP, through KH1's payment (`mpcost`).
    let cost = super::mpcost::pay(actor, spell).unwrap_or(0);
    let mut dropped = None;
    if let Some(&(_, second)) = FIRE_SECOND.iter().find(|f| f.0 == request) {
        if count >= 2 && rd::<u32>(actor + QUEUE + QUEUE_ENTRY) & 0xFFFF == second {
            for k in 1..count as usize - 1 {
                let e: [u8; QUEUE_ENTRY] = rd(actor + QUEUE + (k + 1) * QUEUE_ENTRY);
                wr(actor + QUEUE + k * QUEUE_ENTRY, e);
            }
            wr(actor + QUEUE_COUNT, count - 1);
            dropped = Some(second);
        }
    }
    if let Ok(mut g) = RT.lock() {
        runtime(&mut g).fresh = Some(Fresh { spell, level, finisher, motion });
    }
    take_step(actor);
    let v = if finisher { m.voice_finisher } else { m.voice };
    if v != 255 {
        super::kh2sound::voice(v as i16);
    }
    log(&format!(
        "KH2 magic {} (level {level}) in KH1's cast state: request {request:#x}, KH2 motion {motion}{}, cost {cost} units{}",
        SPELL_NAMES[spell as usize],
        if air { " (air)" } else if finisher { " (the combo's finisher)" } else { "" },
        dropped.map_or(String::new(), |r| format!(", KH1's second part {r:#x} taken off its queue"))
    ));
    true
}

/// Whether Sora can pay for `spell` now: in a Drive Form KH1's 1 MP (`cost`); base Sora KH2's
/// cost scaled onto KH1's MP (`mpcost`), magic allowed as KH1 judges it.
unsafe fn affordable(actor: usize, spell: u8, cost: i32) -> bool {
    if super::form::in_form() {
        return super::reflect::can_pay(actor, cost);
    }
    super::reflect::can_pay(actor, 1) && super::mpcost::units(actor, spell).is_some()
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
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
    if !affordable(actor, spell, cost) {
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
    if super::form::in_form() {
        super::reflect::pay_mp(actor, cost);
    } else {
        super::mpcost::pay(actor, spell);
    }
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
pub unsafe fn shots_step(actor: usize, out: &Output, airborne: bool, strikes: &mut combat::Strikes, battle: &Battle, lock: Option<u32>) {
    let Ok(mut g) = RT.try_lock() else { return };
    let Some(rt) = g.as_mut() else { return };
    if rt.queued.is_empty() && rt.fresh.is_none() && rt.casting.is_none() && rt.magic.casts.is_empty() && rt.magic.effects.is_empty() && rt.fx.is_empty() {
        return;
    }
    let mut world = world_now(actor, out, airborne, strikes, lock, 0x51A7_0000 ^ rt.fired.wrapping_mul(2_654_435_761));
    if world.target.is_none() {
        world.target = auto_target(&world, out.facing);
    }
    // A spell cast this frame (trigger 20), its release (21) at the motion's magic-cast event.
    if let Some(f) = rt.fresh.take() {
        if let Some(file) = spell_index(rt, f.spell, f.level) {
            // (Wisdom's subs from 10; base Sora's from 0, `form::magic_sub`.)
            let sub = f.level as i32 * 2 - 2 + f.finisher as i32 + super::form::magic_sub();
            let magi = pose::kh2().and_then(|k| k.magi.get(sub).map(|r| r.to_vec())).unwrap_or_else(|| vec![0; kh2_formats::pref::MagiTable::RECORD]);
            match rt.magic.cast(file, SORA, f.spell, sub, &magi, &mut world) {
                Ok(_) => rt.casting = Some((f.motion, 0)),
                Err(e) => log(&format!("{} {}'s script didn't start ({e})", who(), SPELL_NAMES[f.spell as usize])),
            }
        }
    }
    if let Some((m, waited)) = rt.casting {
        let fired = out.motion == m && out.events.iter().any(|e| matches!(e, character_core::EventKind::MagicCast(x) if *x as u16 == out.motion));
        let waited = if out.motion == m { SEEN } else { waited.saturating_add(1).min(SEEN - 1) };
        rt.casting = Some((m, waited));
        // (In KH1's cast state the motion starts at KH1's next animation step: not cut before it
        // has played, unless it never does.)
        if fired || (out.motion != m && (waited == SEEN || waited > NOT_SEEN_FRAMES)) {
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
                    log(&format!("Wisdom's scripts used {n} (not modelled)"));
                }
            }
            Event::Error(e) => log(&format!("Wisdom's script error: {e}")),
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
            }
            if rt.hits <= 60 {
                log(&format!("{}: ATKP {atk} hit {foe:#x} -> damage {dealt:?}", who()));
            }
        }
    }
    for a in rt.magic.freed().filter(|a| a.owns_id) {
        strikes.spell_forget(a.id | KEY);
    }
}
