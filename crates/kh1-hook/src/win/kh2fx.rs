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
                let spell: Vec<Option<SpellFx>> = (1..=3u8)
                    .map(|level| {
                        spell_library(level, &common).map(|l| {
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
                fx12::set_textures(textures);
                let lib = Arc::new(lib);
                let mut fx = Effects::new(lib);
                fx.seed(0x2D1F);
                if let Ok(mut g) = STATE.lock() {
                    *g = Some(State { fx, running: Vec::new(), motion: u16::MAX, tip_local, frame: Frame::new(), test_at: None, started: 0, spell, hidden: false });
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
fn spell_handle(level: u8, h: EffectHandle) -> u32 {
    ((level.saturating_sub(1) as u32 & 3) << 30) | (h.0 & 0x3FFF_FFFF)
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
    let sp = g.as_mut()?.spell.get_mut((h >> 30) as usize)?.as_mut()?;
    Some(f(&mut sp.fx, EffectHandle(h & 0x3FFF_FFFF)))
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
fn paused(base: usize) -> Option<String> {
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

/// A KH2 controller event (from `takeover`, each frame KH2 has Sora).
pub fn event(motion: u16, kind: EventKind) {
    let EventKind::Effect(el) = kind else { return };
    if el < 0 || !allowed(motion) {
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

/// One game frame of KH2's controller: KH2's motion now.
/// `tick`'s motion while KH1 has Sora: no KH2 motion (effects bound to one end).
pub const NO_MOTION: u16 = u16::MAX - 1;

pub fn tick(motion: u16) {
    let Ok(mut g) = STATE.lock() else { return };
    let Some(s) = g.as_mut() else { return };
    step(s, Some(motion));
}

fn step(s: &mut State, motion: Option<u16>) {
    if let Some(m) = motion {
        if m != s.motion {
            for (h, from, ends) in &s.running {
                if *ends && *from != m {
                    s.fx.end(*h);
                }
            }
            s.motion = m;
        }
    }
    let tip = s.tip_local;
    s.fx.tick(1.0, &mut |c| bone(c, tip));
    for sp in s.spell.iter_mut().flatten() {
        sp.fx.tick(1.0, &mut |c| bone(c, tip));
    }
    let fx = &s.fx;
    s.running.retain(|(h, _, _)| fx.is_running(*h));
}

unsafe extern "system" fn world_pass() {
    let f: unsafe extern "system" fn() = std::mem::transmute(ORIG.load(Ordering::Acquire));
    f();
    let _ = std::panic::catch_unwind(|| draw());
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
    if s.fx.running_count() == 0 && s.spell.iter().flatten().all(|sp| sp.fx.running_count() == 0) {
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
    for sp in s.spell.iter_mut().flatten() {
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
