//! KH2's own visual effects for Sora's moves, drawn in KH1 (the user: Magnet Burst's and
//! Explosion's real effects). The effect package is Sora's (`obj/P_EX100.a.us`, PAX "p_ex",
//! with the shared `effect/texcommon.dpd` textures), read from the user's KH2 install once KH2
//! Sora is loaded; `kh2_fx` plays its elements the way KH2's particle machine does
//! (`docs/kh2-effects.md`) and `fx12` draws them in KH1's Direct3D 12 frame.
//!
//! A move's frame trigger 1 (`EventKind::Effect`) starts its element at Sora's bone; elements
//! flagged to end with their motion are ended when KH2's motion changes. Effects step once per
//! KH2 controller frame (KH1's game frame: they stop when the game stops) and are drawn after
//! KH1's world-model pass (`0x1401EADD0`, hooked here).
//!
//! Bones: KH2 Sora's pose (KH2 model space) and the model → world it was drawn with
//! (`pose::kh2_pose`). The effect world is KH2's (y down), as KH1's is; a bone's world matrix
//! is model → world × the bone, the object root's is model → world turned half round about x (element
//! offsets at the root are in KH2's y-down object space), the weapon points are the Keyblade's
//! tip (its collision point 0 in the bind pose, carried by hand bone 178).
//!
//! Ini: `kh2_effects` (`magnet_explosion` default, `all`, `off`), `fx_test = on` starts Magnet
//! Burst's sphere at Sora every 4 seconds (to check the renderer in game).
//!
//! Explosion (elements 44..48) is KH1's own on Steam (`kh2dpx`, retired here): `event` hands it
//! over and this player gets it only while KH1's DPX isn't built, if KH2's programs couldn't be
//! plugged into KH1's program machine, or on another build (`kh2dpx::event`).


use super::addr::Rva;
/// Explosion, Vicinity Break, Magnet Burst and Aerial Finish (the user: those three first;
/// Vicinity Break's was asked for when it went unnoticed without it, Aerial Finish's (motion
/// 194, Sora's combo row 29, ability 100) when it was missing), then Guard Break (165: elements
/// 31, 30, 32), Finishing Leap (167: 33, 34) and Aerial Recovery (40: element 0), the user
/// finding them without theirs.
pub const DEFAULT_MOTIONS: [u16; 7] = [166, 170, 197, 194, 165, 167, 40];
use super::{fx12, hook, log};
use character_core::EventKind;
use glam::{Mat4, Vec3};
use kh2_fx::{Camera, EffectHandle, Effects, Frame, Library};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const WORLD_PASS_RVA: Rva = Rva(0x1EADD0);
const WORLD_PASS_PROLOGUE: [u8; 15] = [0x48, 0x8B, 0xC4, 0x48, 0x89, 0x58, 0x08, 0x48, 0x89, 0x70, 0x10, 0x48, 0x89, 0x78, 0x18];
/// KH2 element flag: ended when the owner's motion changes.
const ENDS_WITH_MOTION: u32 = 0x20;
/// The Keyblade's tip in KH2 Sora's bind pose (model space, W_EX010's collision point 0), and
/// the hand bone that carries it.
const TIP_BIND: Vec3 = Vec3::new(-59.7, 108.0, 75.5);
const HAND: usize = 178;

static ORIG: AtomicUsize = AtomicUsize::new(0);
static BASE: AtomicUsize = AtomicUsize::new(0);

struct State {
    fx: Effects,
    /// Running effects: handle, the motion that started it, whether it ends with that motion.
    running: Vec<(EffectHandle, u16, bool)>,
    /// Valor Form's elements started (`form_event`): handle, the motion, ends with it.
    form_running: Vec<(u32, u16, bool)>,
    motion: u16,
    tip_local: Vec3,
    frame: Frame,
    test_at: Option<std::time::Instant>,
    started: u32,
    /// Each level's spell file's own effect package (`reflect`: `magic/REFLECT_<n>.mag`, by
    /// level − 1), played beside Sora's; their textures follow his in the renderer's texture
    /// list (each from its `tex_base`).
    spell: Vec<Option<SpellFx>>,
    /// Whether the effects were hidden for a pause or menu last time (for the log).
    hidden: bool,
    /// The Keyblade's own package (`obj/W_EX010.a.us`): its flash as it vanishes (`weapon_vanish`).
    weapon: Option<SpellFx>,
}

struct SpellFx {
    fx: Effects,
    tex_base: u32,
    frame: Frame,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn allowed(motion: u16) -> bool {
    match &super::settings::get().kh2_effects {
        None => false,
        Some(None) => true,
        Some(Some(v)) => v.contains(&motion),
    }
}

/// Hooks the world pass and loads the effects in the background once KH2 Sora is there.
pub unsafe fn start() {
    let set = super::settings::get();
    if set.kh2_effects.is_none() && !set.fx_test {
        log("KH2 effects: off (kh2_effects = off)");
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + WORLD_PASS_RVA, &WORLD_PASS_PROLOGUE, world_pass as *const () as usize, &ORIG, "world-model pass");
    std::thread::spawn(|| {
        let t = std::time::Instant::now();
        let kh2 = loop {
            if let Some(k) = super::pose::kh2() {
                break k;
            }
            if t.elapsed().as_secs() > 300 {
                log("KH2 effects: KH2 Sora never loaded");
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        };
        match load() {
            Ok((lib, common)) => {
                let bind = kh2_anim::Pose::bind(&kh2.rig.skeleton);
                let tip_local = bind.bones.get(HAND).map(|h| h.inverse().transform_point3(TIP_BIND)).unwrap_or(Vec3::ZERO);
                let mut textures: Vec<(u32, u32, Vec<u8>)> = lib.textures().iter().map(|t| (t.width, t.height, t.rgba.clone())).collect();
                // Reflect's own packages (the user: its VFX), one per level, their textures after
                // Sora's.
                // (And Wisdom Form's, as package 4: its shots, `form`.)
                // (And Wisdom Form's spells, Fire / Blizzard / Thunder / Cure by level, as packages
                // `form::spell_package`.)
                // (The spells' packages without the forms too: base Sora's KH2 magic, `styles`.)
                let last = if super::settings::get().drive { FINAL } else { super::form::LAST_PACKAGE };
                let spell: Vec<Option<SpellFx>> = (1..=last)
                    .map(|level| {
                        let lib = if level == WISDOM {
                            wisdom_library(kh2, &common)
                        } else if level == VALOR {
                            valor_library(kh2, &common)
                        } else if level == FINAL {
                            form_library(kh2.final_form.as_ref(), "Final Form", &common)
                        } else if level > WISDOM {
                            form_spell_library(level, &common)
                        } else {
                            spell_library(level, &common)
                        };
                        lib.map(|l| {
                            let tex_base = textures.len() as u32;
                            textures.extend(l.textures().iter().map(|t| (t.width, t.height, t.rgba.clone())));
                            let mut fx = Effects::new(Arc::new(l));
                            fx.seed(0x5EF1 + level as u32);
                            SpellFx { fx, tex_base, frame: Frame::new() }
                        })
                    })
                    .collect();
                log(&format!(
                    "KH2 effects: {} elements, {} textures loaded in {:.1} s{}",
                    lib.elements().len(),
                    textures.len(),
                    t.elapsed().as_secs_f32(),
                    if lib.warnings().is_empty() { String::new() } else { format!("; {} warnings, first: {}", lib.warnings().len(), lib.warnings()[0]) }
                ));
                for id in [44u16, 45, 46, 47, 48, 51, 52] {
                    let skipped = lib.unsupported_programs(id);
                    if !skipped.is_empty() {
                        log(&format!("KH2 effects: element {id} skips {skipped:?}"));
                    }
                }
                // The Keyblade's package (KH2's Kingdom Key, W_EX010), its textures after the rest.
                let weapon = weapon_library(&common).map(|l| {
                    let tex_base = textures.len() as u32;
                    textures.extend(l.textures().iter().map(|t| (t.width, t.height, t.rgba.clone())));
                    let mut fx = Effects::new(Arc::new(l));
                    fx.seed(0x3EA0);
                    SpellFx { fx, tex_base, frame: Frame::new() }
                });
                fx12::set_textures(textures);
                let lib = Arc::new(lib);
                let mut fx = Effects::new(lib);
                fx.seed(0x2D1F);
                if let Ok(mut g) = STATE.lock() {
                    *g = Some(State { fx, running: Vec::new(), form_running: Vec::new(), motion: u16::MAX, tip_local, frame: Frame::new(), test_at: None, started: 0, spell, hidden: false, weapon });
                }
            }
            Err(e) => log(&format!("KH2 effects: not loaded ({e})")),
        }
    });
}

fn load() -> Result<(Library, Vec<u8>), String> {
    let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
    let install = kh2_data::Install::locate(&exe).map_err(|e| e.to_string())?;
    let src = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
    let pack = install.read_any(src, "obj/P_EX100.a.us").map_err(|e| e.to_string())?;
    let common = install.read_any(src, "effect/texcommon.dpd").map_err(|e| format!("texcommon.dpd: {e}"))?;
    let lib = Library::load_named(&pack, Some("p_ex"), Some(&common)).map_err(|e| format!("{e:?}"))?;
    Ok((lib, common))
}

/// The Keyblade's effect package (`obj/W_EX010.a.us`).
fn weapon_library(common: &[u8]) -> Option<Library> {
    let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
    let r = (|| -> Result<Library, String> {
        let install = kh2_data::Install::locate(&exe).map_err(|e| e.to_string())?;
        let src = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
        let pack = install.read_any(src, "obj/W_EX010.a.us").map_err(|e| e.to_string())?;
        Library::load(&pack, Some(common)).map_err(|e| format!("{e:?}"))
    })();
    match r {
        Ok(l) => {
            log(&format!("KH2 effects: the Keyblade's package, {} elements", l.elements().len()));
            Some(l)
        }
        Err(e) => {
            log(&format!("KH2 effects: the Keyblade's package not loaded ({e})"));
            None
        }
    }
}

/// The Keyblade's vanish flash: element 0 of its package (on its blade, bone 2: a flash that
/// swells and fades over 13 ticks with a turning glint; Data, KH2's own call for it not traced),
/// started where the blade is as KH2 puts it away (the user: "when a keyblade disappears in any
/// animation there isn't any visual effects from kh2 for a dissapearing keyblade").
pub const WEAPON_VANISH: u16 = 0;

/// The Keyblade's bones as the weapon package's elements name them: any code is the blade, half
/// way from the hand to the tip, turned as the hand.
fn blade(tip_local: Vec3) -> Option<Mat4> {
    super::pose::kh2_pose(|pose, w| pose.get(HAND).map(|h| *w * *h * Mat4::from_translation(tip_local * 0.5))).flatten()
}

pub fn weapon_vanish() {
    if super::settings::get().kh2_effects.is_none() {
        return;
    }
    let Ok(mut g) = STATE.lock() else { return };
    // (The effects port) KH1's own effect system plays it once its DPX is built (`kh2dpx`).
    if super::kh2dpx::weapon_vanish(g.as_ref().map_or(NO_MOTION, |s| s.motion)) {
        return;
    }
    let Some(s) = g.as_mut() else { return };
    let tip = s.tip_local;
    let Some(wp) = s.weapon.as_mut() else { return };
    if wp.fx.start(WEAPON_VANISH, &mut |_| blade(tip)).is_some() {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 10 {
            log("KH2 effect: the Keyblade's vanish");
        }
    }
}

/// Wisdom Form's effect package (`P_EX100_MAGF.a.us`, PAX "p_ex"), played as spell package 4.
pub const WISDOM: u8 = 4;

fn wisdom_library(kh2: &super::pose::Kh2, common: &[u8]) -> Option<Library> {
    let archive = kh2.wisdom.as_ref()?.archive.as_ref()?;
    match Library::load_named(archive, Some("p_ex"), Some(common)) {
        Ok(l) => {
            log(&format!("KH2 effects: Wisdom Form's package, {} elements, {} textures", l.elements().len(), l.textures().len()));
            Some(l)
        }
        Err(e) => {
            log(&format!("KH2 effects: Wisdom Form's package not loaded ({e:?})"));
            None
        }
    }
}

/// Valor Form's effect package (`P_EX100_BTLF.a.us`), after Wisdom's spells' (stage 211): its
/// motions' frame trigger 1 starts its own elements, not Sora's (`form_event`).
pub const VALOR: u8 = super::form::LAST_PACKAGE + 1;

/// Final Form's (`P_EX100_ULTF.a.us`), after Valor's (stage 231).
pub const FINAL: u8 = VALOR + 1;

fn valor_library(kh2: &super::pose::Kh2, common: &[u8]) -> Option<Library> {
    form_library(kh2.valor.as_ref(), "Valor Form", common)
}

fn form_library(form: Option<&super::pose::Form>, name: &str, common: &[u8]) -> Option<Library> {
    let archive = form?.archive.as_ref()?;
    let l = Library::load_named(archive, Some("p_ex"), Some(common)).or_else(|_| Library::load(archive, Some(common)));
    match l {
        Ok(l) => {
            log(&format!("KH2 effects: {name}'s package, {} elements, {} textures", l.elements().len(), l.textures().len()));
            Some(l)
        }
        Err(e) => {
            log(&format!("KH2 effects: {name}'s package not loaded ({e:?})"));
            None
        }
    }
}

/// A frame trigger 1 of Valor's motion (`takeover`, `form::takes`): its element from Valor's
/// package, on Sora; one that ends with its motion is ended when the motion changes (`step`).
pub fn form_event(motion: u16, el: i16) {
    if el < 0 || super::settings::get().kh2_effects.is_none() {
        return;
    }
    let Ok(mut g) = STATE.lock() else { return };
    let Some(s) = g.as_mut() else { return };
    let tip = s.tip_local;
    // (Each element's anchor logged once: which bone, which weapon point.)
    {
        static SEEN: Mutex<Vec<(u8, i16)>> = Mutex::new(Vec::new());
        let pkg = if super::form::in_final() { FINAL } else { VALOR };
        if let (Ok(mut seen), Some(sp)) = (SEEN.lock(), s.spell.get(pkg as usize - 1).and_then(|p| p.as_ref())) {
            if !seen.contains(&(pkg, el)) && seen.len() < 64 {
                seen.push((pkg, el));
                if let Some(e) = sp.fx.library().element(el as u16) {
                    log(&format!(
                        "KH2 effect: form package {pkg} element {el} (motion {motion}): anchor {:#06x}, flags {:#x}, at {:.0?}, turned {:.2?}",
                        e.bone, e.flags, e.translation, e.rotation
                    ));
                }
            }
        }
    }
    // (Final Form's own package in Final, else Valor's.)
    let pkg = if super::form::in_final() { FINAL } else { VALOR };
    let Some(sp) = s.spell.get_mut(pkg as usize - 1).and_then(|p| p.as_mut()) else { return };
    let ends = sp.fx.library().element(el as u16).is_some_and(|e| e.flags & ENDS_WITH_MOTION != 0);
    match sp.fx.start(el as u16, &mut |c| form_bone(c, tip)) {
        Some(h) => {
            unsafe { super::kh2sound::spell_element(pkg, el as u16) };
            s.form_running.push((spell_handle(pkg, h), motion, ends));
            s.started += 1;
            if s.started <= 40 {
                log(&format!("KH2 effect: form package {pkg} element {el} of motion {motion} started"));
            }
        }
        None => {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 10 {
                log(&format!("KH2 effect: element {el} not in Valor's package"));
            }
        }
    }
}

/// A Wisdom Form spell's package (its `.mag` file's PAX), `form::spell_package` numbering.
fn form_spell_library(package: u8, common: &[u8]) -> Option<Library> {
    let (spell, level) = super::form::package_spell(package)?;
    let bytes = super::form::spell_file(spell, level)?;
    match Library::load(&bytes, Some(common)) {
        Ok(l) => Some(l),
        Err(e) => {
            log(&format!("KH2 effects: Wisdom {} level {level}'s package not loaded ({e:?})", super::form::SPELL_NAMES[spell as usize]));
            None
        }
    }
}

/// Sora's object replaced (a form change, `form`): his effects that end with their owner end
/// (the Drive motion's glows on his limbs loop until then; the user: the glow stayed on him).
pub fn owner_gone() {
    super::kh2dpx::owner_gone();
    let Ok(mut g) = STATE.lock() else { return };
    let Some(s) = g.as_mut() else { return };
    let mut n = s.fx.owner_gone();
    for sp in s.spell.iter_mut().flatten() {
        n += sp.fx.owner_gone();
    }
    if n > 0 {
        log(&format!("KH2 effects: {n} ended with Sora's object (a form change)"));
    }
}

/// Puts a spell effect's root where the script has it (an unbound one it moves: Wisdom's shots),
/// a KH2-world (y down) matrix.
pub fn spell_place(h: u32, at: Mat4) {
    with_spell(h, |fx, e| fx.set_anchor(e, at));
}

/// A Reflect level's effect package (the PAX in its spell file), if the spell is on and its file
/// read.
fn spell_library(level: u8, common: &[u8]) -> Option<Library> {
    if !super::settings::get().reflect {
        return None;
    }
    let Some(bytes) = super::reflect::spell_file(level) else {
        log(&format!("KH2 effects: Reflect level {level}'s spell file not read: no effects for it"));
        return None;
    };
    match Library::load(&bytes, Some(common)) {
        Ok(l) => {
            let skipped: Vec<_> = [0u16, 1, 4, 12, 13, 14].iter().map(|&id| (id, l.unsupported_programs(id))).filter(|(_, v)| !v.is_empty()).collect();
            log(&format!(
                "KH2 effects: Reflect level {level}'s package, {} elements, {} textures{}",
                l.elements().len(),
                l.textures().len(),
                if skipped.is_empty() { String::new() } else { format!("; skipped programs {skipped:?}") }
            ));
            Some(l)
        }
        Err(e) => {
            log(&format!("KH2 effects: Reflect level {level}'s package not loaded ({e:?})"));
            None
        }
    }
}

/// A spell effect's handle for `reflect`: the level's package (bits 30..31: level − 1) and its
/// handle there.
/// A spell package's effect: the package (`level` − 1) in the top 5 bits.
fn spell_handle(level: u8, h: EffectHandle) -> u32 {
    ((level.saturating_sub(1) as u32 & 31) << 27) | (h.0 & 0x07FF_FFFF)
}

/// Starts element `el` of a Reflect level's package on Sora (its bone code as for his own
/// effects), for `reflect` (the spell script's `pax_start_bind`). Returns its handle.
pub fn spell_start(level: u8, el: u16) -> Option<u32> {
    if super::settings::get().kh2_effects.is_none() {
        return None;
    }
    let Ok(mut g) = STATE.lock() else { return None };
    let s = g.as_mut()?;
    let tip = s.tip_local;
    let sp = s.spell.get_mut((level as usize).wrapping_sub(1))?.as_mut()?;
    let h = sp.fx.start(el, &mut |c| bone(c, tip))?;
    Some(spell_handle(level, h))
}

/// A Reflect level's package is loaded (for the log).
pub fn spell_ready(level: u8) -> bool {
    STATE.lock().ok().is_some_and(|g| g.as_ref().is_some_and(|s| s.spell.get((level as usize).wrapping_sub(1)).is_some_and(|p| p.is_some())))
}

/// What the spell script did to an effect it started: ended its loop (`end`), killed it, or set
/// its fade (`alpha`).
pub fn spell_end(h: u32) {
    with_spell(h, |fx, e| fx.end(e));
}

pub fn spell_kill(h: u32) {
    with_spell(h, |fx, e| fx.kill(e));
}

pub fn spell_alpha(h: u32, alpha: f32) {
    with_spell(h, |fx, e| fx.set_alpha(e, alpha));
}

pub fn spell_running(h: u32) -> bool {
    with_spell(h, |fx, e| fx.is_running(e)).unwrap_or(false)
}

fn with_spell<R>(h: u32, f: impl FnOnce(&mut Effects, EffectHandle) -> R) -> Option<R> {
    let mut g = STATE.lock().ok()?;
    let sp = g.as_mut()?.spell.get_mut((h >> 27) as usize)?.as_mut()?;
    Some(f(&mut sp.fx, EffectHandle(h & 0x07FF_FFFF)))
}

/// KH1's pause machine (`0x14017D030`): 0 while the game runs; 1..6 from the pause button (or a
/// menu opened the same way: kinds `0x14232E984`, 0 the pause menu `0x1402D3450`) until the
/// game runs again. While it is up KH1 runs only its pause tasks (task mask `0x142867D74` = 1),
/// and the menu's own task (`0x1402D3040`, priority 159999) draws the menu's windows before the
/// world-model pass, then repoints the camera (`0x142340ED8` / `0x142340EE0`) for the party
/// models it shows there: our pass, drawn after the world's, landed on top of the menu.
const PAUSE_STATE_RVA: Rva = Rva(0x232E9B4);

/// The game is paused or a full-screen menu holds it: KH1's pause machine is up, or Sora has had
/// no physics frame for a while (`takeover::paused`: any other menu that stops the field).
pub(super) fn paused(base: usize) -> Option<String> {
    let state = unsafe { std::ptr::read_unaligned((base + PAUSE_STATE_RVA) as *const i32) };
    if state != 0 {
        return Some(format!("KH1's pause / menu state {state}"));
    }
    if super::takeover::paused() {
        return Some("no physics frame of Sora's for over 120 ms".into());
    }
    None
}

/// The world matrix of an element's bone code (KH2 world, y down).
fn bone(code: u16, tip_local: Vec3) -> Option<Mat4> {
    super::pose::kh2_pose(|pose, w| {
        let w = *w;
        match code {
            // KH2's object space is the model's turned half round about x (y down, and forward
            // along -z: Guard Break's spiral sits 235 along -z, at the end of the thrust), not
            // mirrored in y alone, which put it behind him (the user: the effect sat at the middle
            // of the Keyblade).
            0x4000 => Some(w * Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0))),
            0x4001 => pose.first().map(|b| Mat4::from_translation(w.transform_point3(b.w_axis.truncate()))),
            0x4003..=0x4008 => pose.get(HAND).map(|h| w * *h * Mat4::from_translation(tip_local)),
            c => pose.get(c as usize).map(|b| w * *b),
        }
    })
    .flatten()
}

/// A form's element anchor (Valor's, Final's: two Keyblades). KH2's weapon anchors
/// (`0x1403B43D0`): 0x4003 / 0x4004 the tip of weapon 0 / 1 (an odd code the first, an even the
/// second), 0x4005 / 0x4006 its root, 0x4007 / 0x4008 halfway (× 0.5, `0x14062399C`); `bone`
/// sent every weapon code to the right hand's tip. Here the point is that Keyblade's (its root
/// from KH2's weapon bones of the step, the tip at the right one's tip-from-root, the same model),
/// turned with Sora's facing, as the object root (the user: "valor forms grounded finisher
/// effect should spawn in front of the keyblades and its direction its sent should be the same
/// as where sora is facing"). Other codes as `bone`.
fn form_bone(code: u16, tip_local: Vec3) -> Option<Mat4> {
    if !(0x4003..=0x4008).contains(&code) {
        return bone(code, tip_local);
    }
    let second = (code - 1) & 1 == 1;
    let along = match code {
        0x4003 | 0x4004 => 1.0,
        0x4005 | 0x4006 => 0.0,
        _ => 0.5,
    };
    let _ = tip_local;
    let right_root = {
        let g = super::takeover::POSE.try_lock().ok()?;
        *g.as_ref()?.1.first()?
    };
    // The tip in a Keyblade root's frame: `TIP_BIND` is W_EX010's collision point in the weapon
    // skeleton's own bind (its root there at (-59.7, 108, -2.5): the tip 78 along it), the same
    // model in both hands.
    static TIP_IN_ROOT: std::sync::OnceLock<Option<Vec3>> = std::sync::OnceLock::new();
    let tip_in_root = (*TIP_IN_ROOT.get_or_init(|| {
        let w = super::pose::kh2()?.rig.weapons.first()?;
        let root = *kh2_anim::Pose::bind(&w.skeleton).bones.first()?;
        let t = root.inverse().transform_point3(TIP_BIND);
        log(&format!("KH2 effects: the Keyblade's tip {:.1?} from its root (weapon anchors)", t));
        Some(t)
    }))?;
    let root = if second { super::leftblade::kh2_left_root()? } else { right_root };
    let point_model = root.transform_point3(tip_in_root * along);
    super::pose::kh2_pose(|_, w| {
        // The object root's turn (Sora's facing, KH2's object axes as for 0x4000) at the point.
        let obj = *w * Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0));
        let mut m = obj;
        m.w_axis = w.transform_point3(point_model).extend(1.0);
        m
    })
}

/// A KH2 controller event (from `takeover`, each frame KH2 has Sora).
pub fn event(motion: u16, kind: EventKind) {
    let EventKind::Effect(el) = kind else { return };
    if el < 0 || !(allowed(motion) || super::form::own_motion(motion)) {
        return;
    }
    // (t368) Elements KH1's own effect system plays (`kh2dpx`).
    if super::kh2dpx::event(motion, el) {
        return;
    }
    let Ok(mut g) = STATE.lock() else { return };
    let Some(s) = g.as_mut() else { return };
    let tip = s.tip_local;
    let ends = s.fx.library().element(el as u16).is_some_and(|e| e.flags & ENDS_WITH_MOTION != 0);
    match s.fx.start(el as u16, &mut |c| bone(c, tip)) {
        Some(h) => {
            s.running.push((h, motion, ends));
            s.started += 1;
            if s.started <= 20 {
                log(&format!("KH2 effect: element {el} of motion {motion} started"));
            }
        }
        None => log(&format!("KH2 effect: element {el} not in Sora's package")),
    }
}

/// `tick`'s motion while KH1 has Sora: no KH2 motion (effects bound to one end).
pub const NO_MOTION: u16 = u16::MAX - 1;

/// One game frame of KH2's effects: KH2's motion now, and the frame's delta in 60 Hz frames (1.0
/// at 60 fps, 2.0 at 30, like the game's own frame delta; the effect clock adds it up).
pub fn tick(motion: u16, delta: f32) {
    // (Step 2) The elements KH1 plays: their endings and fades (`kh2dpx`).
    super::kh2dpx::tick(motion, delta);
    let Ok(mut g) = STATE.lock() else { return };
    let Some(s) = g.as_mut() else { return };
    step(s, Some(motion), delta);
}

fn step(s: &mut State, motion: Option<u16>, delta: f32) {
    if let Some(m) = motion {
        if m != s.motion {
            for (h, from, ends) in &s.running {
                if *ends && *from != m {
                    s.fx.end(*h);
                }
            }
            for (h, from, ends) in &s.form_running {
                if *ends && *from != m {
                    if let Some(sp) = s.spell.get_mut((*h >> 27) as usize).and_then(|p| p.as_mut()) {
                        sp.fx.end(EffectHandle(h & 0x07FF_FFFF));
                    }
                }
            }
            s.motion = m;
        }
    }
    let tip = s.tip_local;
    // This frame's camera for the programs that face it while ticking (kh2-fx: camera-facing
    // matrices; else they used the last frame built's, a frame late).
    if let Some((view, _)) = unsafe { fx12::camera(BASE.load(Ordering::Relaxed)) } {
        let cam = Camera::from_view(view);
        s.fx.set_camera(&cam);
        for sp in s.spell.iter_mut().flatten() {
            sp.fx.set_camera(&cam);
        }
    }
    s.fx.tick(delta, &mut |c| bone(c, tip));
    for sp in s.spell.iter_mut().flatten() {
        sp.fx.tick(delta, &mut |c| bone(c, tip));
    }
    if let Some(wp) = s.weapon.as_mut() {
        if let Some((view, _)) = unsafe { fx12::camera(BASE.load(Ordering::Relaxed)) } {
            wp.fx.set_camera(&Camera::from_view(view));
        }
        wp.fx.tick(delta, &mut |_| blade(tip));
    }
    let fx = &s.fx;
    s.running.retain(|(h, _, _)| fx.is_running(*h));
    let spell = &s.spell;
    s.form_running.retain(|(h, _, _)| spell.get((*h >> 27) as usize).and_then(|p| p.as_ref()).is_some_and(|sp| sp.fx.is_running(EffectHandle(h & 0x07FF_FFFF))));
}

unsafe extern "system" fn world_pass() {
    let f: unsafe extern "system" fn() = std::mem::transmute(ORIG.load(Ordering::Acquire));
    f();
    let _ = std::panic::catch_unwind(|| {
        // KH2's player model first (the effects then blend over it).
        super::model::draw();
        draw()
    });
}

fn draw() {
    let Ok(mut g) = STATE.try_lock() else { return };
    let Some(s) = g.as_mut() else { return };
    if super::settings::get().fx_test {
        let due = s.test_at.is_none_or(|t| t.elapsed().as_secs_f32() > 4.0);
        if due && super::pose::kh2_pose(|_, _| ()).is_some() {
            s.test_at = Some(std::time::Instant::now());
            let tip = s.tip_local;
            if let Some(h) = s.fx.start(51, &mut |c| bone(c, tip)) {
                s.running.push((h, u16::MAX, false));
                log("KH2 effect: test (element 51)");
            }
        }
        // (KH1's frames step the effects too: `takeover::physics`.)
    }
    if s.fx.running_count() == 0 && s.spell.iter().flatten().all(|sp| sp.fx.running_count() == 0) && s.weapon.as_ref().is_none_or(|w| w.fx.running_count() == 0) {
        return;
    }
    let base = BASE.load(Ordering::Relaxed);
    // Never over KH1's pause menu or another full-screen menu (the user: Reflect's barrier showed
    // through the pause menu). KH1 draws the menu's windows before this pass, so while it is up
    // the effects aren't drawn at all (they are frozen meanwhile: they step on Sora's frames).
    let why = paused(base);
    if why.is_some() != s.hidden {
        s.hidden = why.is_some();
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 40 {
            log(&match &why {
                Some(w) => format!("KH2 effects: hidden ({w})"),
                None => "KH2 effects: shown again (the game runs)".to_string(),
            });
        }
    }
    if why.is_some() {
        return;
    }
    // The pause menu draws its own Sora in another world pass, with its own camera: the effects
    // once, in the field's (the user: a copy of the effect in the pause menu).
    if super::pose::menu_sora_within(150) {
        return;
    }
    let Some((view, proj)) = (unsafe { fx12::camera(base) }) else { return };
    let cam = Camera::from_view(view);
    s.fx.build(&cam, &mut s.frame);
    // The spell's triangles after Sora's, its texture ids moved past his.
    for sp in s.spell.iter_mut().flatten().chain(s.weapon.as_mut()) {
        if sp.fx.running_count() > 0 {
            sp.fx.build(&cam, &mut sp.frame);
            let first = s.frame.vertices.len() as u32;
            s.frame.vertices.extend_from_slice(&sp.frame.vertices);
            for b in &sp.frame.batches {
                let mut b = *b;
                b.first += first;
                b.texture = b.texture.map(|t| t + sp.tex_base);
                s.frame.batches.push(b);
            }
        }
    }
    unsafe { fx12::draw(base, &s.frame, view, proj) };
}
