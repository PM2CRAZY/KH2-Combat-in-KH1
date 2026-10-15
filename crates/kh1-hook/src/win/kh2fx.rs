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
//! is model → world × the bone, the object root's is model → world with y flipped (element
//! offsets at the root are in KH2's y-down object space), the weapon points are the Keyblade's
//! tip (its collision point 0 in the bind pose, carried by hand bone 178).
//!
//! Ini: `kh2_effects` (`magnet_explosion` default, `all`, `off`), `fx_test = on` starts Magnet
//! Burst's sphere at Sora every 4 seconds (to check the renderer in game).

use super::{fx12, hook, log};
use character_core::EventKind;
use glam::{Mat4, Vec3};
use kh2_fx::{Camera, EffectHandle, Effects, Frame, Library};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const WORLD_PASS_RVA: usize = 0x1EADD0;
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
            Ok(lib) => {
                let bind = kh2_anim::Pose::bind(&kh2.rig.skeleton);
                let tip_local = bind.bones.get(HAND).map(|h| h.inverse().transform_point3(TIP_BIND)).unwrap_or(Vec3::ZERO);
                let textures: Vec<(u32, u32, Vec<u8>)> = lib.textures().iter().map(|t| (t.width, t.height, t.rgba.clone())).collect();
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
                    *g = Some(State { fx, running: Vec::new(), motion: u16::MAX, tip_local, frame: Frame::new(), test_at: None, started: 0 });
                }
            }
            Err(e) => log(&format!("KH2 effects: not loaded ({e})")),
        }
    });
}

fn load() -> Result<Library, String> {
    let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
    let install = kh2_data::Install::locate(&exe).map_err(|e| e.to_string())?;
    let src = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
    let pack = install.read_any(src, "obj/P_EX100.a.us").map_err(|e| e.to_string())?;
    let common = install.read_any(src, "effect/texcommon.dpd").map_err(|e| format!("texcommon.dpd: {e}"))?;
    Library::load_named(&pack, Some("p_ex"), Some(&common)).map_err(|e| format!("{e:?}"))
}

/// The world matrix of an element's bone code (KH2 world, y down).
fn bone(code: u16, tip_local: Vec3) -> Option<Mat4> {
    super::pose::kh2_pose(|pose, w| {
        let w = *w;
        match code {
            0x4000 => Some(w * Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0))),
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
        // Without KH2's controller ticking (KH1 has Sora), step here.
        if !super::takeover::kh2_has() {
            step(s, None);
        }
    }
    if s.fx.running_count() == 0 {
        return;
    }
    let base = BASE.load(Ordering::Relaxed);
    let Some((view, proj)) = (unsafe { fx12::camera(base) }) else { return };
    let cam = Camera::from_view(view);
    s.fx.build(&cam, &mut s.frame);
    unsafe { fx12::draw(base, &s.frame, view, proj) };
}
