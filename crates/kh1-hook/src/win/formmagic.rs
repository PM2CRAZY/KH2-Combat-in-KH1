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
//!   records. Cure's heal (a script's direct application to each party member in its circle): ours,
//!   a share of KH1's max HP by the ATKP row's power (`CURE_SHARE`), scaled by Sora's max MP as
//!   KH1 has no KH2 Magic stat (`combat::magic_factor`, t828).

use super::reflect::{flip, world_now, KhWorld, SORA};
use super::{combat, log, pose};
use character_core::{CastMotions, Character, Output};
use glam::{Mat4, Quat, Vec3};
use kh2_formats::battle::{Atkp, Battle};
use kh2_magic::{Event, Magic, SpellFile, World};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

pub const SPELL_NAMES: [&str; 4] = ["Fire", "Blizzard", "Thunder", "Cure"];

/// The Config menu's row for what `spell` (Fire 0 .. Cure 3) base Sora casts (`styles`; the
/// tester, thread kh2-effects-in-kh1s-own-effect-system m19: "each magic should have its own
/// settings of which magic you can choose ... fire: kh1 | kh2 and that would change what gets
/// casted"), on the ground and in the air alike (the Magic / Aerial Magic rows are gone).
fn spell_row(spell: u8) -> Option<super::styles::Kind> {
    use super::styles::Kind;
    match spell {
        0 => Some(Kind::Fire),
        1 => Some(Kind::Blizzard),
        2 => Some(Kind::Thunder),
        3 => Some(Kind::Cure),
        _ => None,
    }
}

/// The row for `spell`'s cast animation (Blizzard 1 .. Aero 6; the tester, m19: "you can also
/// change the animation of magic with the exception being reflect, and fire"). Fire has none (its
/// animation is its spell's), nor Reflect (always KH2's).
fn anim_row(spell: u8) -> Option<super::styles::Kind> {
    use super::styles::Kind;
    match spell {
        1 => Some(Kind::BlizzardAnim),
        2 => Some(Kind::ThunderAnim),
        3 => Some(Kind::CureAnim),
        4 => Some(Kind::GravityAnim),
        5 => Some(Kind::StopAnim),
        6 => Some(Kind::AeroAnim),
        _ => None,
    }
}

/// `spell` (Fire 0 .. Cure 3) is set to KH2's on its row (whether or not he is in a Drive Form).
pub fn kh2_spell(spell: u8) -> bool {
    spell_row(spell).is_some_and(super::styles::setting)
}

/// Base Sora casts KH2's `spell` now (its row KH2's, not in a Drive Form, which keep their own).
pub fn base_kh2_spell(spell: u8) -> bool {
    !super::form::in_form() && kh2_spell(spell)
}

/// Wisdom Form on KH1's own states casts KH2's `spell` (Fire 0 .. Cure 3) now: all four are KH2's
/// in Wisdom (KH2's Wisdom casts KH2's Fire, Blizzard, Thunder and Cure, its own cast motions; a
/// Drive Form ignores the Config rows, the tester, what-is-still-missing after m13), on KH1's own
/// cast state as base Sora's KH2 spells are (`native`), at Wisdom's magic sub (`form::magic_sub`).
pub fn wisdom_kh2_spell(spell: u8) -> bool {
    spell <= 3 && super::form::native_form() == Some(super::form::Kind::Wisdom)
}

/// Base Sora's cast of `spell` (Fire 0 .. Aero 6) plays KH2's cast motion now: by the spell's
/// animation row, KH1's spell or KH2's (the tester, kh2-effects-in-kh1s-own-effect-system m20:
/// "if a kh2 spell is casted on a kh1 animation then it should cast on the frame that the kh1
/// animation casts it and vice versa": a KH2 spell on KH1's animation releases at KH1's cast
/// frame, `KH1_CAST_FRAME`); Fire has no row, its animation its spell's. Not in a Drive Form.
pub fn anim_kh2(spell: u8) -> bool {
    // (A Drive Form on KH1's own states: its own cast motions for every spell, KH2's all through,
    // whatever the rows: KH1's Gravity, Stop and Aero on them as for base Sora at KH2.)
    if super::form::native_form().is_some() {
        return true;
    }
    !super::form::in_form() && anim_row(spell).map_or(spell == 0 && kh2_spell(0), super::styles::setting)
}

/// The frame KH1's own spell script casts its spell, in 60 Hz frames from its start (KH1's
/// scripts, `xm_<spell>0N.mag`, every level, ground and air alike, **Checked (files)**: each waits
/// on KH1's frame scale, script call 5 `0x1402B98E0` reading `0x14233FBDC`, then checks the cast
/// request is still playing (call 51, `+0x164`) and makes its shot / pays its MP (calls 20, 47)):
/// Blizzard 28, Thunder 30, Cure 36, Gravity 28 (its parameter block's `+0x1C`), Stop 4 + 24,
/// Aero 30 (sooner when its cast motion ends). Fire has none: it waits for its first motion
/// (0x36 / 0x83, KH1's slot 54 / 79: 28 frames) to end. `docs/kh1-magic-native.md` §4j.
pub const KH1_CAST_FRAME: [f32; 7] = [28.0, 28.0, 30.0, 36.0, 28.0, 28.0, 30.0];

/// KH1's frame scale (Steam `0x14233FBDC`: 1.0 a frame at 60 fps, 2.0 at 30; `frameadv`).
const FRAME_SCALE_STEAM: usize = 0x233FBDC;

pub(super) unsafe fn frame_scale() -> f32 {
    let s: f32 = rd(super::sys::GetModuleHandleA(std::ptr::null()) as usize + FRAME_SCALE_STEAM);
    if s.is_finite() && s > 0.0 && s < 4.0 { s } else { 1.0 }
}

/// The KH1 spell KH1 casts now (Gravity, Stop, Aero) is drawn with base Sora's KH2 cast motion
/// on its clock, as Wisdom's KH1 spells are (`takeover::command_started`, `pose::follow`): its
/// animation row at KH2 (the user: "KH1 spell, KH2 animation").
pub static BASE_KH1_CAST: AtomicBool = AtomicBool::new(false);

/// The KH1 spell `spell` (Gravity 4, Stop 5, Aero 6) is cast KH1's whole way (the debug menu's
/// KH1 Spells page, which now shows the Config menu's animation rows for them): its animation row
/// at KH1. KH2 has no Gravity, Stop or Aero; what KH2 brings to them is its cast motion (Thunder's
/// for Gravity, Cure's for Stop and Aero, `kh1_spell_motion`).
pub fn kh1_cast(spell: u8) -> bool {
    (4..=6).contains(&spell) && !anim_row(spell).is_some_and(super::styles::setting)
}

/// The debug menu's KH1 Spells row: `spell` (4..6) cast KH1's whole way or not: its animation row
/// (kept in the ini, as the Config menu keeps it).
pub fn set_kh1_cast(spell: u8, on: bool) {
    if let Some(k) = anim_row(spell).filter(|_| (4..=6).contains(&spell)) {
        super::styles::set(k, !on);
    }
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
/// Cure's heal: the ATKP row's power / this, of KH1's max HP (ours), times Sora's max MP factor
/// (`heal`, t828).
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
                    log!(Magic, "KH2 spells: {name} not read ({e})");
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
    // (Also for base Sora's KH2 magic, `base_kh2_spell`, which the Config menu switches in game.)
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
    /// On KH1's own cast animation (its animation row at KH1): KH1's cast request and the frame
    /// KH1's script would cast at (`KH1_CAST_FRAME`).
    kh1_anim: Option<(u32, f32)>,
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
    /// A cast on KH1's own animation (`Fresh::kh1_anim`): KH1's cast request, the 60 Hz frames
    /// since the script started, the frame it is released at, and whether KH1 has applied the
    /// request yet (it plays from KH1's next animation step).
    kh1_clock: Option<(u32, f32, f32, bool)>,
    fired: u32,
    hits: u32,
    /// The world as the scripts saw it at our last step, for KH1's script machine stepping a cast
    /// (`kh1_step`, `kh2script`).
    world: Option<KhWorld>,
    actor: usize,
}

static RT: Mutex<Option<Runtime>> = Mutex::new(None);
/// A chosen spell, when, and the KH2 move it was chosen in (it waits only while that plays).
static WAITING: Mutex<Option<(u8, Instant, u32)>> = Mutex::new(None);
static NO_WISD: AtomicBool = AtomicBool::new(false);

fn runtime(g: &mut Option<Runtime>) -> &mut Runtime {
    g.get_or_insert_with(|| {
        let mut magic = Magic::new();
        // (KH1's script machine runs the spell scripts, `kh2script`, t798.)
        magic.native = super::kh2script::machine(super::kh2script::Kind::Form);
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
                    log!(Magic, "Wisdom Form's script ready (its shots)");
                    Some(i)
                }
                Err(e) => {
                    if !NO_WISD.swap(true, Ordering::Relaxed) {
                        log!(Magic, "Wisdom Form fires no shots: {e}");
                    }
                    None
                }
            }
        });
        Runtime { magic, wisd, files: HashMap::new(), package, fx: HashMap::new(), queued: Vec::new(), arrival: Vec::new(), arrival_age: 0, fresh: None, casting: None, kh1_clock: None, fired: 0, hits: 0, world: None, actor: 0 }
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
            log!(Magic, "KH2 spells: {name} not usable ({e})");
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
    rt.kh1_clock = None;
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
/// KH2's when the spell's row is KH2's (`base_kh2_spell`). Returns
/// whether it was taken (else KH1 casts its own).
pub unsafe fn spell_chosen(actor: usize, spell: u8) -> bool {
    // (Wisdom: Fire, Blizzard, Thunder; Cure is KH1's own on the form's Cure motion, as Gravity,
    // Stop and Aero are: the user, "wisdom form currently casts kh2 cure, it needs to be kh1 cure
    // with kh2s cures animation". Base Sora's KH2 magic: Cure too, KH2's.)
    if !super::takeover::kh2_has() {
        return false;
    }
    let wisdom = spell <= 2 && super::form::in_wisdom() && super::settings::get().drive;
    let base = base_kh2_spell(spell);
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
        log!(Magic, "{} {} refused: can't pay {cost}", who(), SPELL_NAMES[spell as usize]);
        return true;
    }
    if let Ok(mut w) = WAITING.lock() {
        *w = Some((spell, Instant::now(), super::takeover::KH2_MOTION.load(Ordering::Relaxed)));
    }
    log!(Magic, "{} {} chosen (level {level}, cost {cost})", who(), SPELL_NAMES[spell as usize]);
    true
}

/// A spell is waiting for its window (`takeover`).
pub fn waiting() -> bool {
    WAITING.try_lock().map_or(false, |w| w.is_some())
}

/// Something of the scripts' still runs or plays (`takeover::native_spells` runs their frame).
pub fn busy() -> bool {
    RT.try_lock().is_ok_and(|g| g.as_ref().is_some_and(|rt| !rt.queued.is_empty() || rt.fresh.is_some() || rt.casting.is_some() || rt.kh1_clock.is_some() || !rt.magic.casts.is_empty() || !rt.magic.effects.is_empty() || !rt.fx.is_empty()))
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
    // KH1's script call 5 (the frame scale its scripts count their waits in), ours in KH1's own
    // script-call table, for KH1's spells on KH2's cast animation (`script_step`).
    let entry = base + SCRIPT_CALLS_STEAM + 5 * SCRIPT_CALL_SIZE;
    let was = ((base + SCRIPT_STEP_STEAM) as u64).to_le_bytes();
    let ours = (script_step as *const () as usize as u64).to_le_bytes();
    if super::hook::patch(entry, &was, &ours, "KH1's script call 5 (KH1's spells timed to KH2's cast animation)") {
        log!(Magic, "KH1's script call 5 at {entry:#x} is ours (KH1's spells on KH2's cast animation cast on its cast frame)");
    }
}

/// KH1's script-call table (Steam `0x140529180`, 16 bytes a call: the function, then its argument
/// count with bit 30 "returns a value"; Checked (exe)) and call 5's function (`0x1402B98E0`: writes
/// KH1's frame scale `0x14233FBDC` into its one argument, `movss xmm0, [rip+..]; movss [rcx],
/// xmm0; ret`, Checked (exe)); the process the script machine is running (`0x142DB7B48`, set by
/// `0x1402BCEC0` / `0x1402BD540` around each run of `0x1402CC620`, Checked (decomp)).
const SCRIPT_CALLS_STEAM: usize = 0x529180;
const SCRIPT_CALL_SIZE: usize = 0x10;
const SCRIPT_STEP_STEAM: usize = 0x2B98E0;
const SCRIPT_RUNNING_STEAM: usize = 0x2DB7B48;

/// A KH1 spell cast on KH2's cast animation (the tester, m20, "vice versa"): its script process,
/// its clock's rate (KH1's cast frame over KH2's), the 60 Hz frames run, the frames it runs for
/// (KH2's cast frame) and the last game frame counted.
struct Timed {
    process: usize,
    rate: f32,
    run: f32,
    upto: f32,
    frame: u64,
}

static TIMED: Mutex<Option<Timed>> = Mutex::new(None);

/// KH1's script call 5 (ours, `install`): KH1's frame scale, as KH1's; for the process of a KH1
/// spell cast on KH2's cast animation (`time_kh1_spell`), times its rate until KH2's cast frame,
/// so KH1's script's own wait (`KH1_CAST_FRAME`) ends on the frame KH2's animation casts. KH1's
/// scripts count every wait in this call (`docs/kh1-magic-native.md` §4j); nothing else of the
/// script changes.
unsafe extern "system" fn script_step(out: *mut f32) {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let f: unsafe extern "system" fn(*mut f32) = std::mem::transmute(base + SCRIPT_STEP_STEAM);
    f(out);
    if out.is_null() {
        return;
    }
    let Ok(mut g) = TIMED.try_lock() else { return };
    let Some(t) = g.as_mut() else { return };
    if rd::<usize>(base + SCRIPT_RUNNING_STEAM) != t.process {
        return;
    }
    let step = *out;
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    if frame != t.frame {
        if t.frame != 0 {
            t.run += step;
        }
        t.frame = frame;
    }
    if t.run >= t.upto {
        log!(Magic, "KH1's spell on KH2's cast animation: its script's clock back to KH1's after {:.0} frames", t.run);
        *g = None;
        return;
    }
    *out = step * t.rate;
}

/// After KH1's spell start for one of KH1's own spells (Blizzard .. Aero) on our slot of KH2's
/// cast motion (its animation row at KH2, `anim_kh2`; `kh2slots::redirect` points its request
/// there): its script's clock runs at KH1's cast frame over KH2's (the clip's magic-cast event,
/// frame trigger 17) until KH2's cast frame, so it casts there (the tester, m20: "and vice
/// versa"). `started`: what KH1's spell start returned (the task's `+0x18`; its process at `+0x20`,
/// `0x14028F5E0`, Checked (decomp)). `request`: the cast's request, queued first.
unsafe fn time_kh1_spell(actor: usize, spell: u8, request: u32, started: u64) {
    if !(1..=6).contains(&spell) || started < 0x10000 || !anim_kh2(spell) || kh2_spell(spell) && spell <= 3 {
        return;
    }
    // (The cast picked for the request, `kh2slots::cast_now`: Gravity's Thunder finisher cast 63
    // when it is the combo's finisher.)
    let Some(motion) = super::kh2slots::cast_now(request as usize) else { return };
    let Some(clip) = pose::kh2().and_then(|k| k.character.moveset.clips.get(&motion)) else { return };
    let casts = |want: bool| clip.events.iter().find(|e| matches!(e.kind, character_core::EventKind::MagicCast(x) if !want || x as u16 == motion)).map(|e| e.frame as f32);
    let Some(upto) = casts(true).or_else(|| casts(false)).filter(|f| *f > 0.0) else { return };
    let process = rd::<usize>(started as usize + 0x20);
    if process < 0x10000 {
        return;
    }
    let own = KH1_CAST_FRAME[spell as usize];
    // (A hair over KH1's frame over KH2's, so the count reaches 0 on KH2's frame and not a frame
    // later by rounding.)
    let rate = own / upto * 1.0001;
    if let Ok(mut g) = TIMED.lock() {
        *g = Some(Timed { process, rate, run: 0.0, upto, frame: 0 });
    }
    let _ = actor;
    log!(Magic, "KH1's spell {spell} on KH2's cast motion {motion} (request {request:#x}): KH1's script casts on KH2's cast frame {upto:.0} (its own {own:.0}; its clock x{rate:.3} until then)");
}

/// KH2's `spell` (Fire 0 .. Cure 3) is cast in KH1's own cast state now (`air`: KH1's air cast):
/// base Sora in KH1's own states (`shared`), the Magic / Aerial Magic style KH2's, KH1's spell
/// start hooked, our slot of KH2's cast motion built for KH1's request (`kh2slots::CASTS`), and
/// the spell's KH2 file and MAGC row there. Else KH2's controller borrows him for it, as before.
pub unsafe fn native(spell: u8, air: bool) -> bool {
    if !(base_kh2_spell(spell) || wisdom_kh2_spell(spell)) || !NATIVE_HOOKED.load(Ordering::Relaxed) || !super::shared::active() {
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
    let step = note_step(actor, air);
    // (KH2 has no finisher cast in the air: the air cast's own motion, the step still the last.)
    let finisher = !air && step.is_some_and(|s| s.1);
    let ground = 0x36 + spell as usize;
    let m = super::kh2slots::pick_cast(ground, finisher);
    if spell == 0 {
        super::kh2slots::pick_cast(0x3D, finisher);
    }
    if finisher {
        log!(Magic, "KH2 magic {}: the combo's finisher, KH2's finisher cast {m:?}", SPELL_NAMES[spell as usize]);
    }
}

/// The step of KH1's combo a cast starting now takes (KH2's `spell_slot`, `pick_cast`) and
/// whether it is the combo's last, kept for the cast's start (`take_step`). Also for Reflect on
/// KH1's cast state (`reflect::cast_native`).
pub unsafe fn note_step(actor: usize, air: bool) -> Option<(u8, bool)> {
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    let stats = pose::ptr(rd::<u32>(actor + 0x6C));
    // The cast's step of the combo (KH2's `spell_slot`: in the air every step counts, KH2's
    // `swing_counts`), kept for its start (`kh1_spell_start`, the tester's answer 1 (a)).
    // (Taken in Stop-as-Magnet's Finishing Plus window: KH2's `+0xD75` one more, `0x1403D6CA0`,
    // though the cast it comes out of was no finisher.)
    let magnet = magnet_window(actor) == Some(true);
    let step = (info >= 0x10000 && stats >= 0x10000).then(|| {
        let live = rd::<u32>(actor + 0x1CC) >> 22 & 1 != 0;
        let count = rd::<u8>(info + 0x41) as u32;
        let max = rd::<u8>(stats + if air { 0xD5 } else { 0xD4 }).max(1) as u32;
        // (A new combo; or Magnet's window out of a mid-combo Stop. At the length Stop is noted as
        // the combo's finisher and its window counts as a finisher's, `CHAINED`.)
        if !live {
            super::combomods::new_combo();
        } else if magnet && count < max {
            super::combomods::spend_finisher_window();
        }
        // (KH2's magic command is checked with the window test on whatever hit, `0x1403D6FC0` →
        // `0x1403D6CA0 (actor, 1)`: in a combo or finisher window a cast always counts on; only
        // the Attack command needs the last move to have connected, `take_step`. Checked (decomp).)
        let counts = true;
        let step = if !live { 1 } else if counts { count + 1 } else { count.max(1) };
        // (Past the length: taken in a finisher's Finishing Plus window, KH2's `+0xD75` one more,
        // `0x1403D6CA0`.)
        CHAINED.store(live && counts && count >= max, Ordering::Relaxed);
        (step.min(255) as u8, step >= max)
    });
    if step.is_none() {
        CHAINED.store(false, Ordering::Relaxed);
    }
    CAST_STEP.store(step.map_or(0, |(s, last)| s as u32 | (last as u32) << 8 | 1 << 16), Ordering::Relaxed);
    step
}

/// Whether the cast KH1's command step is starting takes the combo's last step (`note_step`'s
/// `CAST_STEP`, read before `take_step` consumes it); `None` when no step was noted for it. t747: the
/// spell voice's finisher (`kh2sound::kh1_spell_group`, `spell_line`), so Stop, which KH2's combo
/// plays as Magnet with no finisher cast, says its line on the combo's last step too.
pub fn cast_last() -> Option<bool> {
    let v = CAST_STEP.load(Ordering::Relaxed);
    (v & 1 << 16 != 0).then_some(v >> 8 & 1 != 0)
}

/// The cast `note_step` worked out comes out of a finisher's Finishing Plus window (KH1's count
/// already at the combo's length or past it).
static CHAINED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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
/// Except a ground finisher cast with Finishing Plus's next finisher left
/// (`combomods::finisher_left`; the tester, after m10, question 2 (a)): its count stays at its
/// step, so KH1's Attack step in its Finishing Plus window (range id 50,
/// `kh2slots::finisher_window`) counts on past the length, and the cast is noted as a finisher of
/// the combo (`combomods::cast_step`), so the next finisher is KH2's (or KH1's, at the combo style
/// KH1) as after a finisher swing. `request`: the KH1 request the cast plays on; `finisher`: it
/// plays KH2's finisher cast (Cure has none).
pub(super) unsafe fn take_step(actor: usize, request: u32, finisher: bool, air: bool) {
    let v = CAST_STEP.swap(0, Ordering::Relaxed);
    let chained = CHAINED.swap(false, Ordering::Relaxed);
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    if v & 1 << 16 == 0 || info < 0x10000 {
        return;
    }
    let (step, last) = ((v & 0xFF) as u8, v >> 8 & 1 != 0);
    let finisher = finisher && last && !air;
    // (An air cast taken in an air finisher's Finishing Plus window (the count already at the air
    // length, `CHAINED`): KH2's `0x1403D6CA0` adds one to the finisher count `+0xD75` there for the
    // magic command as for the Attack, whatever the cast plays (Checked (decomp)); noted as the next
    // finisher of the combo (`combomods::cast_step` chained), though KH2 has no air finisher cast
    // and so nothing follows it (its air cast has no window). The tester, what-is-still-missing
    // m17 question 7: "add it".)
    let air_chained = air && chained && last;
    super::combomods::cast_step(step, request & 0xFFFF, finisher || air_chained, chained, air);
    let plus = finisher && super::combomods::finisher_left();
    wr(info + 0x41, if last && !plus { 0u8 } else { step });
    // (KH2 clears its "connected" flag, `+0x120` bit 0x2000000, as the cast is taken
    // (`0x1403D6CA0`), and its hit routine `0x1403D12C0` sets it again on the owner when one of the
    // spell's attacks connects; the Attack command counts on in a window only with it set, or with
    // Combo Master (`0x1403D6FC0`'s Attack case; Checked (decomp)). KH1's Attack step reads its
    // swing's "connected" `+0x50` the same way (`0x1402A7510`, Checked (decomp)): cleared here,
    // set by the spell's hit, `spell_hit`. The tester, combo row selection m30 answer 1 (a).)
    wr(info + 0x50, 0u32);
    SPELL_HITS.store(true, Ordering::Relaxed);
    log!(Magic,
        "KH2 magic: the cast takes step {step} of KH1's combo{}",
        if plus {
            if chained { " (Finishing Plus's next finisher; another left)" } else { " (its finisher; Finishing Plus leaves another)" }
        } else if air_chained {
            " (in the air finisher's Finishing Plus window: counted as the next finisher; the air combo ends)"
        } else if last {
            " (its last: the combo ends)"
        } else {
            ""
        }
    );
}

/// A cast took its step (`take_step`) and its hits now count as its "connected".
static SPELL_HITS: AtomicBool = AtomicBool::new(false);

/// One of Sora's free-standing attack objects (a spell's: KH1's own script's or a KH2 spell's on
/// KH1's pool, `+0x30` bit 0x400) made a hit record (`probe::hit`): while the cast that took its
/// step of the combo plays (KH1's ground cast requests 0x36..0x3D, or one of our cast slots), its
/// "connected" (`+0x50`) is set, so KH1's Attack step counts on from the cast in its window, as
/// KH2's hit routine sets the owner's flag for a spell's hit (`0x1403D12C0`, Checked (decomp)).
pub unsafe fn spell_hit(actor: usize, attack: usize) {
    if !SPELL_HITS.load(Ordering::Relaxed) || actor < 0x10000 || rd::<u32>(attack + 0x30) & 0x400 == 0 {
        return;
    }
    let request = rd::<u32>(actor + 0x164) & 0xFFFF;
    if !((0x36..=0x3D).contains(&request) || KH1_AIR_REQUESTS.contains(&request) || super::kh2slots::cast_playing(actor)) {
        SPELL_HITS.store(false, Ordering::Relaxed);
        return;
    }
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    if info < 0x10000 || rd::<u32>(info + 0x50) != 0 {
        return;
    }
    wr(info + 0x50, 1u32);
    log!(Magic, "KH2 magic: the cast's spell connected (request {request:#x}): an Attack in its window goes on with the combo");
}

/// KH1's control step with base Sora in its own states (`takeover::shared_control`, before it
/// reads whether he can act): a fresh jump or Square in our cast slot's free state
/// (`kh2slots::cast_cut_open`, KH2's range id 0 / 1 / 2; none of Sora's casts has one, so in
/// practice never: KH2 takes neither during a cast) ends the cast's action flag (`+0` bit 2, as
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
    log!(Magic, "KH2 cast cut by {} in its free state (KH2 motion {:?})", if jump { "the jump" } else { "Square" }, super::kh2slots::playing_motion(actor));
    true
}

unsafe extern "system" fn spell_start(actor: usize, spell: i32, target: usize) -> u64 {
    if (0..=3).contains(&spell) && kh1_spell_start(actor, spell as u8) {
        return 0;
    }
    // KH1's own spell in the air, base Sora: the Air Physics row's start too (KH2's 40 hop and
    // speed kept at KH2, `shared::physics::cast_stops`, put back over KH1's halving after the
    // command step, `cast_after_command`; nothing at KH1: KH1's own 30 hop and half speed), so a
    // KH1 spell and a KH2 one move him alike (the tester, m19: "air physics kh1 | kh2").
    if actor > 0x10000 && actor == super::takeover::sora_actor() && !super::form::in_form() && super::shared::active() && rd::<i32>(actor + 0x70) == 2 {
        super::shared::physics::cast_stops(actor);
    }
    // (KH1's own spell on our slot of KH2's cast motion: timed to KH2's cast frame,
    // `time_kh1_spell`.)
    let sora = actor > 0x10000 && actor == super::takeover::sora_actor() && !super::form::in_form() && super::shared::active();
    // (Wisdom Form on KH1's own states, whatever the Magic Combo row says since t757: KH1's own
    // Gravity, Stop and Aero take their step too, `kh1_air_cast_step` / `ground_role`.)
    let wisdom = actor > 0x10000 && actor == super::takeover::sora_actor() && wisdom_combo();
    let count = if sora || wisdom { rd::<u32>(actor + QUEUE_COUNT) } else { 0 };
    let request = if (1..=8).contains(&count) { rd::<u32>(actor + QUEUE) & 0xFFFF } else { 0 };
    let f: unsafe extern "system" fn(usize, i32, usize) -> u64 = std::mem::transmute(ORIG_SPELL_START.load(Ordering::Acquire));
    let r = f(actor, spell, target);
    if sora && (1..=6).contains(&spell) && super::kh2slots::CASTS.iter().any(|c| c.2 == request as usize) {
        time_kh1_spell(actor, spell as u8, request, r);
    }
    // (Magic Combo KH2: KH1's own grounded cast takes its step of the combo, as KH2's casts do,
    // `kh1_cast_step`; its step was noted at the command, `combat::command`.)
    if sora && kh1_cast_step(spell) && KH1_WINDOW_REQUESTS.contains(&request) {
        take_step(actor, request, false, false);
    }
    // (Gravity, Stop and Aero at Magic Combo KH1 with their animation rows at KH2: the combo's
    // finisher, `finisher_spell`, t644 / t651.)
    if (sora || wisdom) && finisher_spell(request) {
        take_step(actor, request, true, false);
    }
    // (At Magic Combo KH2: KH2 Thunder's, Magnet's and Cure's roles, `ground_role`; their step
    // noted at the command, `combat::command`. A finisher only as KH2's: Thunder's finisher cast
    // 63 picked at the length (on KH2's animation), Magnet's cast at the length (its own clip is a
    // finisher's, range id 50 only), Cure never.)
    if sora || wisdom {
        if let Some(role) = ground_role(request) {
            let finisher = match role {
                Role::Thunder => anim_kh2(4) && super::kh2slots::cast_now(request as usize) == Some(THUNDER_FINISHER),
                Role::Magnet => true,
                Role::Cure => false,
            };
            take_step(actor, request, finisher, false);
        }
    }
    // (KH1's own air cast at Magic Combo KH2: its step of the air combo, as KH2's air casts take
    // one, `kh1_air_cast_step`; noted at the command, `combat::command`.)
    if (sora || wisdom) && kh1_air_cast_step(spell) && KH1_AIR_REQUESTS.contains(&request) {
        take_step(actor, request, false, true);
    }
    r
}

/// KH1's air cast requests: `0x83 + spell` (Fire 0 .. Aero 6) and Fire's second part 0x8A (KH1's
/// request table, `xa_ex_0010.mset`, Checked (files); the same as `kh2slots::CASTS`' air rows).
const KH1_AIR_REQUESTS: std::ops::RangeInclusive<u32> = 0x83..=0x8A;

/// Wisdom Form on KH1's own states: KH1's own spells there (Gravity, Stop, Aero; Fire .. Cure are
/// KH2's, `wisdom_kh2_spell`, and take their step already) take a step of the combo as base Sora's
/// do at Magic Combo KH2, and take KH2's roles (`roles_on`).
///
/// Since t757 whatever the Magic Combo row says (the tester, drive-forms after m2: "drive forms
/// magic shouldn't be affected by the magic combo setting"): a Drive Form is KH2's, and KH2's
/// magic command counts every cast as a step of the combo (`0x1403D6CA0`, no setting, Checked
/// (decomp)), so the form always casts by KH2's rules. The row covers base Sora only. Before
/// t757 (m16 answer 8) Wisdom followed the row.
pub unsafe fn wisdom_combo() -> bool {
    super::form::native_form() == Some(super::form::Kind::Wisdom) && super::shared::active()
}

/// The Magic Combo row at KH2 (the tester, what-is-still-missing m16 answer 8: "if kh2's own air
/// magic system takes a combo step then yes do it for all forms of magic"): KH1's own air cast of
/// `spell` (Fire 0 .. Aero 6, KH1's script; KH1's air state 2 or 8) takes the next step of the
/// combo, as KH2's air cast does. **Checked (decomp)**, Steam KH2: the magic command
/// (`0x1403D6FC0`, row kind 2) always asks `0x1403D6CA0 (p, 1)`, which has no ground or air test:
/// in a combo window (`+0x18C` bit 10) the count `+0xD74` one more, in a Finishing Plus window
/// (`+0x194` bit 5) the count and the finisher count `+0xD75` one more, free (`+0x18C & 0x1C`) a new
/// combo (`0x101`); the cast start `0x1403A9DD0` then asks `0x1403D7780 (p, 0)` whether the count
/// has reached the combo's length, the air one (`[+0x5C0] + 0x201`) while airborne
/// (`0x1403BA9A0`), the ground one (`+0x200`) else. So KH2's air cast is a step of the air combo
/// on the same counter as the swings. Ours: `note_step (air)` at the command (against KH1's air
/// combo length, stats `+0xD5`), `take_step (air)` at KH1's spell start (KH1's count `+0x41` at the
/// step, 0 when it is the air combo's last; "connected" `+0x50` cleared, set by the spell's hit).
/// No combo window is added on KH1's air cast: KH2's air casts have none (§4c.1, Checked (files)).
/// Base Sora (not a KH2 spell, which takes its step already, `pick_cast`), and Wisdom's Gravity,
/// Stop and Aero (`wisdom_combo`).
pub unsafe fn kh1_air_cast_step(spell: i32) -> bool {
    if !(0..=6).contains(&spell) || !super::shared::active() {
        return false;
    }
    // (Wisdom whatever the Magic Combo row says, t757: `wisdom_combo`.)
    if wisdom_combo() {
        return (4..=6).contains(&spell);
    }
    super::styles::kh2(super::styles::Kind::MagicCombo) && !super::form::in_form()
}

/// KH1's ground Gravity (0x3A), Stop (0x3B) and Aero (0x3C) cast requests (KH1's request table,
/// Checked (files)).
pub const FINISHER_SPELL_REQUESTS: [u32; 3] = [0x3A, 0x3B, 0x3C];

/// KH2's spell whose part in the combo KH1's Gravity, Stop or Aero takes at Magic Combo KH2 (the
/// tester, what-is-still-missing after m17: "make soras base and any form that can do drive form
/// magic this: aero is the same as kh2 cure, gravity is the same as thunder, stop is a finisher
/// like magnet").
///
/// **Checked (decomp)**, Steam KH2: the cast start `0x1403A9DD0` asks `0x1403D7780 (p, 0)` whether
/// the count has reached the combo's length (no other per-spell rule) and hands that to
/// `0x1403C5170 (p, spell, at_length)`, which on the ground plays the MAGC row's finisher motion
/// (`+0x2C`) when at the length and the row has one, else its ground motion (`+0x28`); in the air
/// always its air motion (`+0x30`). The count itself is the magic command's (`0x1403D6CA0`, one
/// rule for every spell). So a spell's part in the combo is its MAGC motions and the windows on
/// them. **Checked (files)**, `00battle.bin` MAGC (every level) and `P_EX100.mset` /
/// `P_EX100_MAGF.mset`:
/// - Thunder: 62 / finisher 63 / air 64. 62 has its combo window (range id 35 naming 62, from 42 of
///   its 68 frames at 60 per second); 63 is the finisher cast (range id 50, 42..78; no combo window).
/// - Cure: 65 / none / 67. 65 is Thunder's 62 animation (A364) with its ranges copied, so range id
///   35 names 62, not 65: no combo window, no finisher window; at the length it is the last step and
///   the combo ends.
/// - Magnet: 68 / none / 70. 68 is Thunder's finisher animation (A365) with its ranges: only the
///   Finishing Plus window (range id 50, 42..78) and no combo window. Its step is a plain one (the
///   count one more, the finisher count `+0xD75` untouched), so mid-combo nothing follows it but
///   Finishing Plus's window (`+0xD75`, 1 from a new combo, under 1 + Finishing Plus), and what
///   follows there spends one Finishing Plus (`combomods::spend_finisher_window`).
/// - Every air motion (64, 67, 70) is the same clip (A372 / Wisdom's C372) with no combo or
///   finisher window: a plain step of the air combo, as `kh1_air_cast_step` takes it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Thunder,
    Magnet,
    Cure,
}

/// KH2's Thunder finisher cast (MAGC, Checked (files)).
pub const THUNDER_FINISHER: u16 = 63;

/// The ground requests and the role each takes (Gravity Thunder's, Stop Magnet's, Aero Cure's).
const ROLE_REQUESTS: [(u32, Role); 3] = [(0x3A, Role::Thunder), (0x3B, Role::Magnet), (0x3C, Role::Cure)];

/// KH2's roles for KH1's Gravity, Stop and Aero apply now: base Sora on KH1's own states with the
/// Magic Combo row at KH2, or Wisdom on them whatever the row says (`wisdom_combo`, t757); Valor
/// has no magic and the forms on KH2's controller are left as they are.
pub unsafe fn roles_on() -> bool {
    if !super::shared::active() {
        return false;
    }
    if super::form::in_form() {
        return wisdom_combo();
    }
    super::styles::kh2(super::styles::Kind::MagicCombo)
}

/// The KH2 role (`Role`) of KH1's ground cast request `request` now (`roles_on`), if it is Gravity's,
/// Stop's or Aero's.
pub unsafe fn ground_role(request: u32) -> Option<Role> {
    if !roles_on() {
        return None;
    }
    ROLE_REQUESTS.iter().find(|r| r.0 == request).map(|r| r.1)
}

/// Stop as Magnet (`Role::Magnet`) playing on the ground on `actor`: whether its Finishing Plus
/// window is open now (KH2's range id 50 on Magnet's 68, 42..78, with Finishing Plus's next
/// finisher left, `combomods::finisher_left`). On our slot of 68 (its animation row at KH2) from the
/// clip itself (`kh2slots::cast_finisher_open`); on KH1's own animation at the same share of KH1's
/// motion (an **Assumption**, as every window placed on KH1's own casts). `None`: Stop as Magnet
/// isn't playing.
pub unsafe fn magnet_window(actor: usize) -> Option<bool> {
    let request = rd::<u32>(actor + 0x164) & 0xFFFF;
    if rd::<i32>(actor + 0x70) != 0 || ground_role(request) != Some(Role::Magnet) {
        return None;
    }
    if super::kh2slots::cast_playing(actor) {
        return Some(super::kh2slots::cast_finisher_open(actor));
    }
    if !super::combomods::finisher_left() {
        return Some(false);
    }
    let Some((a, b)) = super::kh2slots::finisher_share(MAGNET_GROUND) else { return Some(false) };
    let (t, len) = (rd::<f32>(actor + 0x16C), rd::<f32>(actor + 0x280));
    Some(t >= 0.0 && len > 0.0 && t / len >= a && t / len < b)
}

/// KH2's Magnet ground cast (MAGC, Checked (files)).
const MAGNET_GROUND: u16 = 68;

/// Gravity, Stop or Aero (Aero since t651, the tester, m30: "also forgot to add aero to the list";
/// on KH2's animation each plays KH2's Thunder finisher cast 63, `kh2slots::CASTS`) cast on the
/// ground counts as the combo's finisher (the tester, combo row
/// selection after m29: "for gravity and stop, there's a phenomenon in kh2 where those magics
/// count as a finisher so the combo row skips straight to the finisher, if you have a finishing
/// plus equipped you can combo into another finisher ... that would be applied for kh1 gravity
/// and stop as well if the config is set to can combo"): on KH2's cast animation (their
/// animation row at KH2: KH2's Thunder cast 62 for Gravity, Cure's 65 for Stop) always, on KH1's
/// own with the Magic Combo row at KH2. Base Sora on KH1's states; in a Drive Form only Wisdom's on
/// KH1's own states, with the Magic Combo row at KH2 (`wisdom_combo`).
///
/// Since the tester's "aero is the same as kh2 cure, gravity is the same as thunder, stop is a
/// finisher like magnet" (after m17), only at Magic Combo KH1: at KH2 the three take KH2's roles
/// (`ground_role`). Wisdom never at KH1 (as before t716).
pub unsafe fn finisher_spell(request: u32) -> bool {
    let Some(i) = FINISHER_SPELL_REQUESTS.iter().position(|&r| r == request) else { return false };
    let spell = 4 + i as u8;
    if super::form::in_form() || super::styles::kh2(super::styles::Kind::MagicCombo) {
        return false;
    }
    super::shared::active() && anim_kh2(spell)
}

/// The finisher step of Gravity or Stop (`finisher_spell`), worked out at the command
/// (`combat::command`), in place of `note_step`'s: the combo's length (KH1's `+0xD4`), so the
/// combo goes straight to its finisher, or one past the count when taken in a finisher's Finishing
/// Plus window (KH1's count already at the length or past it: `note_step`'s chained case). The
/// step, taken at the spell's start (`take_step` with `finisher`): `combomods::cast_step` notes it
/// as the combo's finisher, KH1's count is 0 after it (the combo ends) unless Finishing Plus leaves
/// another finisher (`combomods::finisher_left`), which then follows in its Finishing Plus window
/// (`finisher_spell_window`).
pub unsafe fn note_finisher_step(actor: usize) -> Option<u8> {
    let (step, _) = note_step(actor, false)?;
    let stats = pose::ptr(rd::<u32>(actor + 0x6C));
    let max = if stats >= 0x10000 { rd::<u8>(stats + 0xD4).max(1) } else { step };
    let step = step.max(max);
    CAST_STEP.store(step as u32 | 1 << 8 | 1 << 16, Ordering::Relaxed);
    Some(step)
}

/// Gravity or Stop playing on the ground as the combo's finisher (`finisher_spell`; KH1's
/// request `+0x164`): whether an Attack or a spell may follow now. Only in KH2's Finishing Plus
/// window, placed at the same share of the motion as on KH2's own finisher cast of the cast
/// animation it plays on (Gravity: Thunder's finisher cast 63, Stop: Magnet's 68 (MAGC; not Cure's), both 42..78,
/// `kh2slots::finisher_share`; an **Assumption**: KH2 has no Gravity or Stop to read it from),
/// while Finishing Plus leaves another finisher (`combomods::finisher_left`, KH2's count). `None`:
/// neither plays.
pub unsafe fn finisher_spell_window(actor: usize) -> Option<bool> {
    let request = rd::<u32>(actor + 0x164) & 0xFFFF;
    if rd::<i32>(actor + 0x70) != 0 || !finisher_spell(request) {
        return None;
    }
    if !super::combomods::finisher_left() {
        return Some(false);
    }
    // (All three on KH2's Thunder finisher cast's window, the animation they play at KH2, t651.)
    let from = 63;
    let Some((a, b)) = super::kh2slots::finisher_share(from) else { return Some(false) };
    let (t, len) = (rd::<f32>(actor + 0x16C), rd::<f32>(actor + 0x280));
    Some(t >= 0.0 && len > 0.0 && t / len >= a && t / len < b)
}

/// KH1's grounded cast requests the Magic Combo row gives KH2 Fire's combo window
/// (`kh1_cast_window`): Fire's first shot 0x36 and its second part / repeated shot 0x3D, Blizzard
/// 0x37, Thunder 0x38, Cure 0x39, Aero 0x3C (KH1's request table, `xa_ex_0010.mset`, Checked
/// (files); Gravity 0x3A and Stop 0x3B are left out).
const KH1_WINDOW_REQUESTS: [u32; 5] = [0x36, 0x3D, 0x37, 0x38, 0x39];

/// The Magic Combo row at KH2 (`styles::Kind::MagicCombo`; the tester, combo row selection after
/// m29: "make a config for magic that allows you to combo out of grounded magic like kh2"): a
/// grounded cast KH1 casts itself (its own spell `spell`, 0 Fire .. 6 Aero; not Gravity 4 or
/// Stop 5) takes its step of the combo as KH2's casts do (`note_step` at the command,
/// `take_step` at the spell's start: KH1's count `+0x41` at the step, "connected" `+0x50` set).
/// Base Sora on KH1's states, not in a Drive Form.
pub unsafe fn kh1_cast_step(spell: i32) -> bool {
    matches!(spell, 0..=3) && super::styles::kh2(super::styles::Kind::MagicCombo) && !super::form::in_form() && super::shared::active()
}

/// Each frame Sora is posed (`kh2slots::frame`), the Magic Combo row at KH2: KH1's own cast
/// animation playing on the ground (`KH1_WINDOW_REQUESTS`, KH1's own slot, not one of ours,
/// whose KH2 cast motions carry KH2's own window, `kh2slots::combo_kind`) gets KH2 Fire's combo
/// window: KH1's combo window this frame (the instance's `+0x14` bit 22, as
/// `combomods::kh1_finisher_window` opens it) from the same share of KH1's motion as Fire 56's
/// window starts in KH2's (60 of its frames, `kh2slots::combo_share`), to its end. KH1's command
/// menu then takes an Attack or a spell there (`0x1402A2EC0`), and KH1's Attack step counts on
/// from the cast's step (`0x1402A7510`, Checked (decomp)). KH1's quick Fire (Fire again while
/// 0x3D plays) is KH1's own rule and still re-fires. KH2's Fire for every KH1 spell is the
/// tester's rule ("for kh1 magics logic you just apply kh2s fire combo logic on the first shot and
/// the repeated shot animation"); placing it by share is an **Assumption** (KH1's casts have no
/// window of their own), and so is that KH1's spell script, a task of its own, casts whatever
/// motion follows.
pub unsafe fn kh1_cast_window(actor: usize) {
    if actor < 0x10000 || actor != super::takeover::sora_actor() {
        return;
    }
    // (Gravity and Stop as the combo's finisher: their Finishing Plus window, `finisher_spell_window`.)
    if finisher_spell_window(actor) == Some(true) {
        let inst = pose::ptr(rd::<u32>(actor + 0x15C));
        if inst >= 0x10000 {
            wr(inst + 0x14, rd::<u32>(inst + 0x14) | super::combomods::KH1_COMBO_WINDOW);
            static F: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            if super::log_on(super::LogCat::Magic) && F.fetch_add(1, Ordering::Relaxed) < 20 {
                log!(Magic, "Gravity / Stop as the combo's finisher: Finishing Plus window (request {:#x}, time {:.1}): KH1's combo window", rd::<u32>(actor + 0x164) & 0xFFFF, rd::<f32>(actor + 0x16C));
            }
        }
        return;
    }
    // (Stop as Magnet on KH1's own animation: Magnet's Finishing Plus window only, `magnet_window`;
    // on our slot of 68 `kh2slots::finisher_window` opens it from the clip.)
    if !super::kh2slots::cast_playing(actor) && magnet_window(actor) == Some(true) {
        let inst = pose::ptr(rd::<u32>(actor + 0x15C));
        if inst >= 0x10000 {
            wr(inst + 0x14, rd::<u32>(inst + 0x14) | super::combomods::KH1_COMBO_WINDOW);
            static M: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            if super::log_on(super::LogCat::Magic) && M.fetch_add(1, Ordering::Relaxed) < 20 {
                log!(Magic, "Stop as KH2's Magnet: Magnet's Finishing Plus window on KH1's cast (time {:.1}): KH1's combo window", rd::<f32>(actor + 0x16C));
            }
        }
        return;
    }
    if !kh1_cast_step(0) || rd::<i32>(actor + 0x70) != 0 {
        return;
    }
    let request = rd::<u32>(actor + 0x164) & 0xFFFF;
    // (Gravity as Thunder and Aero as Cure, `ground_role`, on KH1's own animation: what KH1's own
    // Thunder and Cure get there, the tester's §4c.3 rule.)
    let role = matches!(ground_role(request), Some(Role::Thunder | Role::Cure));
    if !(KH1_WINDOW_REQUESTS.contains(&request) || role) || super::kh2slots::cast_playing(actor) {
        return;
    }
    let Some(share) = super::kh2slots::combo_share(56) else { return };
    let (t, len) = (rd::<f32>(actor + 0x16C), rd::<f32>(actor + 0x280));
    if !(t >= 0.0 && len > 0.0) || t / len < share {
        return;
    }
    let inst = pose::ptr(rd::<u32>(actor + 0x15C));
    if inst < 0x10000 {
        return;
    }
    wr(inst + 0x14, rd::<u32>(inst + 0x14) | super::combomods::KH1_COMBO_WINDOW);
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if super::log_on(super::LogCat::Magic) && N.fetch_add(1, Ordering::Relaxed) < 20 {
        log!(Magic, "Magic Combo: KH2 Fire's combo window on KH1's cast (request {request:#x}, time {t:.1} of {len:.1}, from {:.0}%)", share * 100.0);
    }
}

/// KH1's command step starting `spell` for base Sora, the cast request asked for: with KH2's
/// slot on that request (`native`), KH2's spell instead of KH1's script: its MP (KH1's cost and
/// payment, as KH2's borrowed casts pay it, `before_step`), KH2's MAGC voice,
/// KH2's script started (trigger 20) on the next KH1 frame and released (21) at the cast motion's
/// magic-cast event at KH1's time (`takeover::native_spells`). Fire's second part, which KH1
/// queues after its first, is taken off the queue (KH2's one motion is the whole cast). Whether
/// it was taken (else KH1's own script runs).
unsafe fn kh1_spell_start(actor: usize, spell: u8) -> bool {
    // (Wisdom on KH1's own states too, `wisdom_kh2_spell`; forms on KH2's controller cast theirs
    // from its step.)
    if actor < 0x10000 || actor != super::takeover::sora_actor() || super::form::driver_form() || !super::shared::active() {
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
    // (On KH1's own cast animation, its animation row at KH1: KH2's script released at the frame
    // KH1's own script casts, `KH1_CAST_FRAME`; it still sees Sora in KH2's cast motion until then,
    // as KH2 sets it, `shots_step`.)
    let kh1_anim = (!anim_kh2(spell)).then(|| (request, KH1_CAST_FRAME[spell as usize]));
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
        runtime(&mut g).fresh = Some(Fresh { spell, level, finisher, motion, kh1_anim });
    }
    take_step(actor, request, finisher, air);
    // (KH2's air cast start, physics KH2: `shared::physics::cast_stops`; its velocity put back
    // after KH1's command step halves it, `cast_after_command` from `combat::command`.)
    if air {
        super::shared::physics::cast_stops(actor);
    }
    let v = if finisher { m.voice_finisher } else { m.voice };
    if v != 255 {
        super::kh2sound::voice(v as i16);
    }
    kh1_cast_voice(actor, spell, finisher);
    log!(Magic,
        "KH2 magic {} (level {level}) in KH1's cast state: request {request:#x}, KH2 motion {motion}{}{}, cost {cost} units{}",
        SPELL_NAMES[spell as usize],
        if air { " (air)" } else if finisher { " (the combo's finisher)" } else { "" },
        kh1_anim.map_or(String::new(), |(_, f)| format!(", on KH1's cast animation: released at KH1's cast frame {f:.0}")),
        dropped.map_or(String::new(), |r| format!(", KH1's second part {r:#x} taken off its queue"))
    );
    true
}

/// A KH2 spell's voice with Battle Quotes KH1 (t744; the user: "when kh1 sora does any kh2 magic we
/// need to make him grunt like kh2 sora until he gets to his finisher, then he plays one of his
/// normal magic lines"): KH2's rule (MAGC: voice 7, a grunt, on every cast; the spell's own line on
/// the finisher cast, Checked (files)) with KH1's lines, through KH1's own group play: a swing grunt
/// (group 4) on a cast, the spell's group (`spell + 7`: Fire 7, Blizzard 8, Thunder 9, as KH1's
/// command step picks it, Checked (decomp)) on the combo's finisher. Fire, Blizzard and Thunder only:
/// KH2 casts Cure with no finisher, so Cure keeps KH1's own line (the command step's, `combat`).
unsafe fn kh1_cast_voice(actor: usize, spell: u8, finisher: bool) {
    if super::kh2sound::kh2_voice() || spell > 2 {
        return;
    }
    let group = if finisher { spell as i32 + 7 } else { super::kh1voice::SWING_GROUP };
    super::kh1voice::play(actor, group);
    log!(Magic, "KH2 magic {} with Battle Quotes KH1: KH1's voice group {group} ({})", SPELL_NAMES[spell as usize], if finisher { "the finisher's spell line" } else { "a grunt" });
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
            log!(Magic, "{} {}: dropped, the move it was chosen in (motion {motion}) ended without a window for it", who(), SPELL_NAMES[spell as usize]);
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
        log!(Magic, "{} {}: no cast motion ({motions:?})", who(), SPELL_NAMES[spell as usize]);
        return;
    };
    if super::form::in_form() {
        super::reflect::pay_mp(actor, cost);
    } else {
        super::mpcost::pay(actor, spell);
    }
    let finisher = Some(motion) == motions.finisher && motions.finisher != Some(motions.ground);
    if let Ok(mut g) = RT.lock() {
        runtime(&mut g).fresh = Some(Fresh { spell, level, finisher, motion, kh1_anim: None });
    }
    let v = if finisher { m.voice_finisher } else { m.voice };
    if v != 255 {
        super::kh2sound::voice(v as i16);
    }
    kh1_cast_voice(actor, spell, finisher);
    log!(Magic, "{} {} (level {level}) cast: motion {motion}{}", who(), SPELL_NAMES[spell as usize], if finisher { " (combo finisher)" } else { "" });
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
    let at = w.pos(me.id);
    let ahead = Vec3::new(facing.sin(), 0.0, facing.cos());
    w.objs
        .iter()
        .filter(|o| o.id >= super::reflect::FOE0)
        .filter_map(|o| {
            let d = w.target_point(o.id) - at;
            let flat = Vec3::new(d.x, 0.0, d.z);
            let dist = flat.length();
            (dist < AUTO_RANGE && flat.normalize_or_zero().dot(ahead) > AUTO_CONE).then_some((o.id, dist))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|x| x.0)
}

/// An unbound effect's root where the script has it (y up, our axes) → KH2's world (y down),
/// turned along its direction.
pub(super) fn place(pos: Vec3, dir: Vec3) -> Mat4 {
    let r = Quat::from_rotation_arc(Vec3::Z, dir.try_normalize().unwrap_or(Vec3::Z));
    flip() * Mat4::from_rotation_translation(r, pos) * flip()
}

/// KH1's own HP add (Steam `0x1402A4920 (actor, amount)`, Checked (decomp): the stats block
/// `+0x6C`'s HP `+0x3C` raised up to its max `+0x40`, KH1's own heal modifier for its status
/// applied, the party's HUD copy updated), and its first bytes (Checked (exe)).
const ADD_HP_STEAM: usize = 0x2A4920;
const ADD_HP_BYTES: [u8; 8] = [0x48, 0x89, 0x74, 0x24, 0x10, 0x48, 0x89, 0x7C];

/// A party member (Sora, Donald, Goofy: their stats blocks are laid out alike, Checked (decomp):
/// KH1's save point heals all three through `+0x6C` `+0x3C` / `+0x40`) healed by a script
/// (Cure): KH1's own HP add (t822; before, written by us, Sora only).
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
    // (Scaled by Sora's max MP, t828: the owner, "tie it to the amount of max mp you have, in kh1
    // thats how magic is scaled (assumption)". KH2's heal is power × the caster's Magic / 100
    // (kind 6); KH1 has no Magic stat, so the stand-in the mod's other Magic-based hits use:
    // +10% a max MP point, at most 30, times the ini's `magic_scale`, `combat::magic_factor`.)
    let add = ((max as f32 * power as f32 / CURE_SHARE * combat::magic_factor()).ceil() as i32).max(1);
    let at = super::takeover::game_base() + ADD_HP_STEAM;
    if super::addr::Build::current() == super::addr::Build::Steam && std::slice::from_raw_parts(at as *const u8, 8) == ADD_HP_BYTES {
        let f: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(at);
        f(actor, add);
    } else {
        std::ptr::write_unaligned((stats + 0x3C) as *mut i32, (hp + add).min(max));
    }
    Some((hp, std::ptr::read_unaligned((stats + 0x3C) as *const i32)))
}

/// A spell attack of Sora's that deals damage, as KH1 strikes with it: Fire, Blizzard and Thunder
/// with KH1's own data for the spell at its level (its element), Fire at KH1 Sora's size (the
/// tester's m20 answer 3, "(a)", as its effects since t561 and Reflect's barrier since t518).
pub(super) unsafe fn spell_attack(actor: usize, a: &kh2_magic::MagicAttack, battle: &Battle, package: &HashMap<usize, u8>) -> Option<combat::SpellAttack> {
    if a.freed || a.owner != SORA {
        return None;
    }
    let pkg = package.get(&a.spell).copied().unwrap_or(super::kh2fx::WISDOM);
    let kh1_data = package.get(&a.spell).and_then(|&p| package_spell(p)).filter(|(spell, _)| *spell <= 2).and_then(|(spell, level)| combat::kh1_spell_data(spell, level));
    let first = row(battle, a.atk, a.sub, 0)?;
    if !combat_core::AttackKind::from_id(first.kind).deals_damage() {
        return None;
    }
    let shapes = a.shapes();
    let fire = package.get(&a.spell).and_then(|&p| package_spell(p)).is_some_and(|(spell, _)| spell == 0);
    let shapes = if fire { super::reflect::kh1_sized_shapes(actor, shapes) } else { shapes };
    (!shapes.is_empty()).then(|| combat::SpellAttack { kh1_data, package: Some(pkg), key: attack_key(a), group: if first.combo_group != 0 { first.combo_group as i64 } else { a.group }, atk: a.atk, sub: a.sub, shapes })
}

/// A spell attack's key for KH1's objects and our hit records.
pub(super) fn attack_key(a: &kh2_magic::MagicAttack) -> u64 {
    a.id | KEY
}

/// KH1's script machine steps one of these casts on its KH1 process, or one of its threads on a KH1
/// child process (`kh2script`, t798 / t799): its script (or that thread) runs once on KH1's actors
/// as they are now (`reflect::KhWorld`, t817). Returns whether its process goes on.
pub unsafe fn kh1_step(id: u64, ctx: kh2_magic::kh2_bdx::CtxId) -> bool {
    // (Our own step has the runtime: the script waits a frame.)
    let Ok(mut g) = RT.try_lock() else { return true };
    let Some(rt) = g.as_mut() else { return false };
    let Some(mut world) = rt.world.take() else { return true };
    rt.magic.frametime = frame_scale();
    rt.magic.frame = Some(super::takeover::GAME_FRAME.load(Ordering::Relaxed));
    let goes = rt.magic.step_external(id, ctx, &mut world);
    rt.world = Some(world);
    goes
}

/// After KH2's step and the swings: the form's scripts' frame (casts, shots, effects, hits).
pub unsafe fn shots_step(actor: usize, out: &Output, airborne: bool, strikes: &mut combat::Strikes, battle: &Arc<Battle>, lock: Option<u32>) {
    let Ok(mut g) = RT.try_lock() else { return };
    let Some(rt) = g.as_mut() else { return };
    if rt.queued.is_empty() && rt.fresh.is_none() && rt.casting.is_none() && rt.kh1_clock.is_none() && rt.magic.casts.is_empty() && rt.magic.effects.is_empty() && rt.fx.is_empty() {
        return;
    }
    let mut world = world_now(actor, out, airborne, strikes, lock, 0x51A7_0000 ^ rt.fired.wrapping_mul(2_654_435_761));
    // (KH1's script machine steps the casts on its processes: the effects age once a game frame,
    // before the frame's first script, whichever of the two runs first, `kh2_magic::Magic::frame`.)
    rt.magic.frame = rt.magic.native.is_some().then(|| super::takeover::GAME_FRAME.load(Ordering::Relaxed));
    // (KH1's frame time, as KH2's scripts run on their game's, t817.)
    rt.magic.frametime = frame_scale();
    // (Its attacks made and moved as KH1 attack objects by the scripts' calls, t819.)
    world.sink = Some(super::reflect::Sink { battle: battle.clone(), kind: super::reflect::SinkKind::Form(rt.package.clone()) });
    if world.locked().is_none() {
        world.auto = auto_target(&world, out.facing);
    }
    // (A cast in KH1's cast state: KH1 queues its motion and plays it from its next animation
    // step, so for a frame or two KH1's slot is still the one before, or at a time below 0. KH2's
    // spell scripts end a cast at once when the caster isn't in a cast motion on its first run
    // (FIRE_3 / BLIZZARD_3 / THUNDER_3 run with the caster's motion other than 56 on their first
    // frame: no release, no attack; Checked (files)); KH2 sets the cast motion before the script
    // first runs. So until KH1 plays it, the scripts see Sora in the cast motion. The tester:
    // "kh2s magic doesn't get casted the first time you input said magic"; a second cast in the
    // combo worked, the motion already playing.)
    // (On KH1's own cast animation KH2's cast motion never plays: the scripts see it until their
    // release, `kh1_clock`.)
    let starting = rt.fresh.as_ref().map(|f| f.motion).or(rt.casting.filter(|c| c.1 != SEEN || rt.kh1_clock.is_some()).map(|c| c.0));
    if let (Some(m), Some(me)) = (starting, world.objs.first_mut()) {
        if out.motion != m {
            me.motion = m;
        }
    }
    // A spell cast this frame (trigger 20), its release (21) at the motion's magic-cast event.
    if let Some(f) = rt.fresh.take() {
        if let Some(file) = spell_index(rt, f.spell, f.level) {
            // (Wisdom's subs from 10; base Sora's from 0, `form::magic_sub`.)
            let sub = f.level as i32 * 2 - 2 + f.finisher as i32 + super::form::magic_sub();
            let magi = pose::kh2().and_then(|k| k.magi.get(sub).map(|r| r.to_vec())).unwrap_or_else(|| vec![0; kh2_formats::pref::MagiTable::RECORD]);
            match rt.magic.cast(file, SORA, f.spell, sub, &magi, &mut world) {
                Ok(_) => {
                    rt.casting = Some((f.motion, 0));
                    rt.kh1_clock = f.kh1_anim.map(|(request, at)| (request, 0.0, at, false));
                }
                Err(e) => log!(Magic, "{} {}'s script didn't start ({e})", who(), SPELL_NAMES[f.spell as usize]),
            }
        }
    }
    if let (Some((m, _)), Some((request, t, at, seen))) = (rt.casting, rt.kh1_clock) {
        // KH2's spell on KH1's own cast animation (the tester's m20): released at the frame KH1's
        // script casts (`KH1_CAST_FRAME`, 60 Hz frames from its start, KH1's frame scale a game
        // frame); KH1's cast request no longer playing once it has (`+0x164`: a hit, a cut), or
        // never applied, releases it then, paid for as below.
        let now = rd::<u32>(actor + 0x164) & 0xFFFF;
        let seen = seen || now == request;
        let cut = if seen { now != request || rd::<u32>(actor) & 4 == 0 } else { t > NOT_SEEN_FRAMES as f32 };
        if t >= at || cut {
            rt.casting = None;
            rt.kh1_clock = None;
            rt.magic.release(SORA, &mut world);
            log!(Magic, "KH2 magic on KH1's cast animation: released at frame {t:.0} ({})", if t >= at { format!("KH1's cast frame {at:.0}") } else { format!("its cast cut, request now {now:#x}") });
        } else {
            rt.kh1_clock = Some((request, t + frame_scale(), at, seen));
            let _ = m;
        }
    } else if let Some((m, waited)) = rt.casting {
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
                            log!(Magic, "Wisdom shot (trigger {n}, motion {}), target {:?}", out.motion, world.target_of(SORA));
                        }
                    }
                }
                Err(e) => log!(Magic, "Wisdom's script didn't take trigger {n}: {e}"),
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
                if super::log_on(super::LogCat::Magic) && N.fetch_add(1, Ordering::Relaxed) < 20 {
                    log!(Magic, "Wisdom's scripts used {n} (not modelled)");
                }
            }
            Event::Error(e) => log!(Magic, "Wisdom's script error: {e}"),
            // KH2's Cure: its script strikes each party member standing in its circle (KH2's own
            // test: party slots 0..3, alive, not a summon, flat distance under the magi record's
            // radius; ATKP 708 on the caster, 709 on the others), each healed (t822: Donald and
            // Goofy are KH1's party actors in the scripts' world).
            Event::Strike { attack, target } if target == SORA || (super::reflect::PARTY0..super::reflect::FOE0).contains(&target) => {
                let Some(a) = rt.magic.attack_mut(attack) else { continue };
                let (atk, sub) = (a.atk, a.sub);
                let Some(r) = row(battle, atk, sub, 0) else { continue };
                let Some(who_actor) = world.actor_of(target) else { continue };
                if matches!(combat_core::AttackKind::from_id(r.kind), combat_core::AttackKind::Cure | combat_core::AttackKind::CureStrong) {
                    if let Some((from, to)) = heal(who_actor, r.power) {
                        log!(Magic, "{} Cure on {}: ATKP {atk} sub {sub} (power {}): HP {from} -> {to}", who(), if target == SORA { "Sora".to_string() } else { format!("party slot {}", target - super::reflect::PARTY0) }, r.power);
                    }
                }
            }
            Event::Strike { .. } => {}
        }
    }
    // (Kept for KH1's script machine, which steps the casts on its own processes, `kh1_step`.)
    rt.world = Some(world);
    rt.actor = actor;
    // Effects: bound ones follow Sora's bones; unbound ones go where the script has them.
    let mut seen = Vec::new();
    for e in &rt.magic.effects {
        seen.push(e.id);
        let package = rt.package.get(&e.spell).copied().unwrap_or(super::kh2fx::WISDOM);
        let placed = e.owner.is_none().then(|| place(e.pos, e.dir));
        match rt.fx.get(&e.id) {
            // (Started by its script's call, t820, `reflect::Sink::effect_start`: its instance, or
            // 0 for none drawn.)
            None if e.host.is_some() => {
                rt.fx.insert(e.id, (e.host.filter(|h| *h != 0), false));
            }
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
                // (Its script's calls place, end and fade it, t826; the rest still from here.)
                let by_call = e.host == Some(h);
                if let (Some(at), false) = (placed, by_call) {
                    super::kh2fx::spell_place(h, at);
                }
                if e.killed {
                    super::kh2fx::spell_kill(h);
                } else if e.fade.is_some() {
                    if !(by_call && e.host_fading) {
                        super::kh2fx::spell_alpha(h, e.alpha());
                    }
                    if !e.alive() {
                        super::kh2fx::spell_kill(h);
                    }
                } else if (e.ended_at.is_some() || !e.alive()) && !ended {
                    if !(by_call && e.host_ended) {
                        super::kh2fx::spell_end(h);
                    }
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
    // (KH1 attack objects of ours that KH1's own collision pass collides, `bodyhits`, made and moved
    // by the scripts' own calls, t819, `reflect::KhWorld::attack_set`: the hits it made since; else
    // our own sweep, as before.)
    let hits = if super::bodyhits::spells_native() {
        super::bodyhits::spell_results(KEY)
    } else {
        let attacks: Vec<combat::SpellAttack> = rt.magic.attacks.iter().filter_map(|a| spell_attack(actor, a, battle, &rt.package)).collect();
        if attacks.is_empty() {
            Vec::new()
        } else {
            strikes.spell_strike(actor, &attacks, &|atk, sub, count| row(battle, atk, sub, count).cloned())
        }
    };
    for (key, atk, foe, dealt) in hits {
        rt.hits += 1;
        if let Some(a) = rt.magic.attacks.iter_mut().find(|a| a.id | KEY == key) {
            a.hit = true;
        }
        if rt.hits <= 60 {
            log!(Magic, "{}: ATKP {atk} hit {foe:#x} -> damage {dealt:?}", who());
        }
    }
    for a in rt.magic.freed().filter(|a| a.owns_id) {
        strikes.spell_forget(a.id | KEY);
        super::bodyhits::spell_forget(a.id | KEY);
    }
}
