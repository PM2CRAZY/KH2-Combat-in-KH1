//! KH2's player model (`player_model = kh2`, the Config menu's Player Model row): Sora drawn
//! with KH2's own model, his Drive Forms with their KH2 models, and his Keyblade with KH2's model
//! of the same Keyblade where KH2 has one (`docs/kh1-kh2-model.md`).
//!
//! - **Load** (a thread at start): `obj/P_EX100.mdlx` and the forms' (`_BTLF`, `_MAGF`, `_ULTF`,
//!   `_HTLF`)
//!   from the KH2 install (the same files `pose` reads), parsed by `kh2_formats::mdlx`; and which
//!   model each KH2 Keyblade has: `03system.bin`'s `item` (its weapon number) and `went` (base
//!   Sora's weapon objects by number), `00objentry.bin` (the object's model), named by
//!   `msg/us/sys.bar`. A Keyblade's model is loaded the first time KH1's Keyblade of the same
//!   name is held (another thread); until it is in, KH1's own is drawn.
//! - **Pose**: `pose` hands over each frame's KH2 pose (body and Keyblade, KH2 model space) with
//!   KH1's placement and the two skeletons' size ratio; `leftblade` the left Keyblade's (Valor,
//!   Final).
//! - **Hide**: KH1's model queue (`leftblade`'s hook on `0x1401D1F10`) asks `hide` for Sora's
//!   draw: every body mesh masked off (the draw params' mesh mask), and his Keyblade too when
//!   ours replaces it. KH1 still poses him (hitboxes, effects, the trail and clanks use those
//!   bones), only his draw is held back.
//! - **Draw**: from the world-model pass (`kh2fx`), the triangles skinned on the CPU and drawn by
//!   `fx12::draw_mesh` (depth-tested and depth-writing, cut out at half alpha, KH1's camera).
//!
//! When KH1 animates him (its own moves, KH1-style moves set in the Config menu, scripted walks),
//! `pose` carries KH1's pose onto KH2's skeleton (`kh1_data::inverse`) and hands that over
//! instead, with no Keyblade pose: KH1's own Keyblade is drawn in his hand then. Any frame
//! without a fresh pose (pause menus, a copy of Sora elsewhere) draws KH1's model as before.

use super::addr::Rva;
use super::form::Kind;
use super::log;
use glam::Mat4;
use kh2_formats::mdlx;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

/// The draw params: the weapon model, the hidden-mesh mask (`leftblade`'s).
const P_WEAPON: usize = 0x18;
const P_MASK: usize = 0x38;
/// The save (Sora's weapon at `+0x36`).
const SAVE_PTR_RVA: Rva = Rva(0x28695A0);
/// A pose older than this isn't drawn (KH1 has him).
const FRESH_S: f32 = 0.1;

/// One loaded model, its textures keyed for `fx12`'s model texture buffer.
struct Loaded {
    name: String,
    model: mdlx::Model,
    key: u64,
}

impl Loaded {
    fn new(name: &str, bytes: &[u8]) -> Result<Loaded, String> {
        let model = mdlx::parse(bytes).map_err(|e| format!("{name}: {e}"))?;
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        name.hash(&mut h);
        Ok(Loaded { name: name.to_string(), model, key: h.finish() & !0xFF })
    }
}

/// A Keyblade's KH2 model: loading, loaded, or none.
enum Weapon {
    Loading,
    Ready(Arc<Loaded>),
    Failed,
}

struct Library {
    install: kh2_data::Install,
    source: kh2_data::Source,
    /// Base Sora, then Valor, Wisdom, Final, Anti, Limit, Master.
    bodies: [Option<Arc<Loaded>>; 7],
    /// KH2's Keyblades: name (lower case) → model name.
    keyblades: Vec<(String, String)>,
    /// By model name.
    weapons: HashMap<String, Weapon>,
}

static LIB: OnceLock<Result<Mutex<Library>, String>> = OnceLock::new();
static BASE: AtomicUsize = AtomicUsize::new(0);

/// This frame's pose, from `pose`.
struct Frame {
    body: Vec<Mat4>,
    weapon: Vec<Mat4>,
    to_world: Mat4,
    ratio: f32,
    at: Instant,
}

static POSE: Mutex<Option<Frame>> = Mutex::new(None);
/// KH2's left Keyblade (Valor, Final), from `leftblade`, and when.
static LEFT: Mutex<Option<(Vec<Mat4>, Instant)>> = Mutex::new(None);
/// What became of each queue of a Sora (the field's, the pause menu's copy): bumped per queue,
/// whether KH1's meshes were held back, and the draw prepared for ours then. The world pass
/// mirrors the latest: ours is drawn on every pass whose Sora was held back, and only then.
struct Track {
    seq: u64,
    /// When it last changed.
    at: Option<Instant>,
    hidden: bool,
    prepared: Option<Arc<Prepared>>,
    /// The world pass's view of it: the last seq seen and what it decided.
    seen: u64,
    drew: bool,
}

/// Everything one draw needs, captured when KH1's Sora was held back.
struct Prepared {
    parts: Vec<(Arc<Loaded>, Vec<Mat4>)>,
    place: Mat4,
}

static MAIN: Mutex<Track> = Mutex::new(Track { seq: 0, at: None, hidden: false, prepared: None, seen: 0, drew: false });
static MENU: Mutex<Track> = Mutex::new(Track { seq: 0, at: None, hidden: false, prepared: None, seen: 0, drew: false });
/// The pause menu's copy of Sora: its instance and its pose on KH2's skeleton (KH1 world
/// space, KH2 units), from `pose`, and when.
static MENU_POSE: Mutex<Option<(usize, Vec<Mat4>, f32, Instant)>> = Mutex::new(None);
/// KH1's left Keyblade's draw (`leftblade`) is ours this frame.
static LEFT_OURS: AtomicBool = AtomicBool::new(false);

/// Counters for the log (`report`): Sora's queues, held back, refused (by reason), world
/// passes, ours drawn, held back but not drawn, the menu's copy, KH1's interpolated replays.
#[derive(Default)]
struct Stats {
    queues: u64,
    hidden: u64,
    refused: std::collections::BTreeMap<&'static str, u64>,
    passes: u64,
    drawn: u64,
    missed: u64,
    menu_hidden: u64,
    menu_drawn: u64,
    frame_buffers: std::collections::BTreeMap<i32, u64>,
    replays: u64,
}
static STATS: Mutex<Option<(Stats, Instant)>> = Mutex::new(None);
const REPORT_S: f32 = 20.0;
/// A world pass with no queue of that Sora since the last one repeats the last decision only
/// this soon after it.
const STALE_S: f32 = 0.05;

fn stat(f: impl FnOnce(&mut Stats)) {
    if let Ok(mut g) = STATS.lock() {
        let (s, _) = g.get_or_insert_with(|| (Stats::default(), Instant::now()));
        f(s);
    }
}

/// Logs the counters every `REPORT_S` seconds while the KH2 model is on.
fn report() {
    let Ok(mut g) = STATS.lock() else { return };
    let Some((s, t)) = g.as_mut() else { return };
    if t.elapsed().as_secs_f32() < REPORT_S {
        return;
    }
    if s.queues + s.passes + s.menu_hidden > 0 {
        log(&format!(
            "KH2 model, last {REPORT_S:.0}s: Sora queued {}, held back {} (refused {:?}); world passes {}, ours drawn {}, held back but not drawn {}; pause menu copy held back {}, drawn {}; frame buffers {:?}; KH1 interpolated replays {}",
            s.queues, s.hidden, s.refused, s.passes, s.drawn, s.missed, s.menu_hidden, s.menu_drawn, s.frame_buffers, s.replays
        ));
    }
    *s = Stats::default();
    *t = Instant::now();
}

/// KH1's replay of a recorded frame with an interpolated camera (`docs/kh1-render.md` §3.2):
/// `mov rax, rsp; mov [rax+18h], rbx; mov [rax+20h], rsi; push rdi`. Hooked to count, and to
/// keep KH1's model while it replays (our draws are recorded straight into KH1's list, so a
/// replayed frame lacks them).
const REPLAY_RVA: Rva = Rva(0x102740);
const REPLAY_PROLOGUE: [u8; 12] = [0x48, 0x8B, 0xC4, 0x48, 0x89, 0x58, 0x18, 0x48, 0x89, 0x70, 0x20, 0x57];
static ORIG_REPLAY: AtomicUsize = AtomicUsize::new(0);

/// When KH1 last replayed a frame (ms since `CLOCK`, 0 never).
static LAST_REPLAY_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static CLOCK: OnceLock<Instant> = OnceLock::new();

fn now_ms() -> u64 {
    CLOCK.get_or_init(Instant::now).elapsed().as_millis() as u64 + 1
}

/// KH1 replayed a frame within the last second: a replayed frame would show him with KH1's
/// meshes held back and ours missing, so KH1's model is drawn meanwhile.
fn replaying() -> bool {
    let t = LAST_REPLAY_MS.load(Ordering::Relaxed);
    t != 0 && now_ms().saturating_sub(t) < 1000
}

unsafe extern "system" fn replay(r: usize, n: i32, t: f32) {
    note_replay();
    if LAST_REPLAY_MS.swap(now_ms(), Ordering::Relaxed) == 0 {
        log("KH2 model: KH1 replays frames with an interpolated camera here; KH1's model is drawn while it does");
    }
    let f: unsafe extern "system" fn(usize, i32, f32) = std::mem::transmute(ORIG_REPLAY.load(Ordering::Acquire));
    f(r, n, t)
}

/// The replay counter's hook (diagnostics; nothing changes if it fails).
pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    super::hook::install(base + REPLAY_RVA, &REPLAY_PROLOGUE, replay as *const () as usize, &ORIG_REPLAY, "interpolated replay (KH2 model's counters)");
}

/// The Config row / ini choice.
pub fn on() -> bool {
    // (Off for now, s259, the user: "comment it out for now" (its row too); the ini's
    // `player_model` is ignored meanwhile.)
    // super::styles::kh2(super::styles::Kind::PlayerModel)
    false
}

/// Starts loading the models (a thread).
pub fn load() {
    let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    BASE.store(base, Ordering::Relaxed);
    std::thread::spawn(|| {
        let t = Instant::now();
        let r = (|| -> Result<Library, String> {
            let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
            let install = kh2_data::Install::locate(&exe).map_err(|e| format!("KH2 install: {e}"))?;
            let source = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
            let read = |name: &str| install.read_any(source, name).map_err(|e| format!("{name}: {e}"));
            let mut bodies: [Option<Arc<Loaded>>; 7] = [None, None, None, None, None, None, None];
            for (i, name) in ["P_EX100", "P_EX100_BTLF", "P_EX100_MAGF", "P_EX100_ULTF", "P_EX100_HTLF", "P_EX100_KH1F", "P_EX100_TRIF"].iter().enumerate() {
                match read(&format!("obj/{name}.mdlx")).and_then(|b| Loaded::new(name, &b)) {
                    Ok(l) => {
                        log(&format!(
                            "KH2 model: {name}: {} bones, {} triangles in {} groups, {} textures",
                            l.model.bones.len(),
                            l.model.triangle_count(),
                            l.model.groups.len(),
                            l.model.textures.len()
                        ));
                        bodies[i] = Some(Arc::new(l));
                    }
                    Err(e) => log(&format!("KH2 model: {e}")),
                }
            }
            if bodies[0].is_none() {
                return Err("base Sora's model did not load".into());
            }
            let keyblades = keyblade_models(&read).unwrap_or_else(|e| {
                log(&format!("KH2 model: Keyblade models not found ({e}); KH1's Keyblades kept"));
                Vec::new()
            });
            log(&format!("KH2 model: {} KH2 Keyblades named ({}), {source:?} files, in {:.1}s", keyblades.len(), keyblades.iter().map(|(n, m)| format!("{n} {m}")).collect::<Vec<_>>().join(", "), t.elapsed().as_secs_f32()));
            Ok(Library { install, source, bodies, keyblades, weapons: HashMap::new() })
        })();
        if let Err(e) = &r {
            log(&format!("KH2 model: off ({e})"));
        }
        let _ = LIB.set(r.map(Mutex::new));
    });
}

/// KH2's Keyblades by name: (name in lower case, model).
fn keyblade_models(read: &dyn Fn(&str) -> Result<Vec<u8>, String>) -> Result<Vec<(String, String)>, String> {
    let system = read("03system.bin")?;
    let items = kh2_formats::system::parse_items(&system).map_err(|e| e.to_string())?;
    let objects = kh2_formats::objentry::parse_objentry(&read("00objentry.bin")?).map_err(|e| e.to_string())?;
    let msg = kh2_formats::msg::Messages::parse(&read("msg/us/sys.bar")?).map_err(|e| e.to_string())?;
    let bar = kh2_formats::bar::Bar::parse(&system).map_err(|e| e.to_string())?;
    let went = bar.get("went").ok_or("no went table")?.data;
    let word = |i: usize| went.get(i * 4..i * 4 + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    // Base Sora's weapon set (moveset 1): its list's offset (in words), then objects by number.
    let list = word(1).ok_or("went too short")? as usize;
    let mut out = Vec::new();
    for it in items.items.iter().filter(|i| i.kind == kh2_formats::system::item_type::KEYBLADE) {
        let Some(name) = msg.text(it.name as u32).filter(|n| !n.is_empty()) else { continue };
        let Some(obj) = word(list + it.number as usize).filter(|o| *o != 0) else { continue };
        let Some(model) = objects.iter().find(|o| o.id == obj).map(|o| o.model.clone()).filter(|m| m.starts_with("W_")) else { continue };
        if !out.iter().any(|(n, _): &(String, String)| *n == name.to_lowercase()) {
            out.push((name.to_lowercase(), model));
        }
    }
    Ok(out)
}

/// The body model for the form he is in (base Sora's if the form's didn't load).
fn body(lib: &Library) -> Option<Arc<Loaded>> {
    let i = match super::form::current() {
        Some(Kind::Valor) => 1,
        Some(Kind::Wisdom) => 2,
        Some(Kind::Final) => 3,
        Some(Kind::Anti) => 4,
        Some(Kind::Limit) => 5,
        Some(Kind::Master) => 6,
        None => 0,
    };
    lib.bodies[i].clone().or_else(|| lib.bodies[0].clone())
}

/// KH2's model of KH1's Keyblade `item`, if KH2 has one by that name and it is in (asked for
/// the first time: loaded on a thread).
unsafe fn weapon_model(lib: &mut Library, item: u8) -> Option<Arc<Loaded>> {
    static NAMES: Mutex<Vec<(u8, Option<String>)>> = Mutex::new(Vec::new());
    let base = BASE.load(Ordering::Relaxed);
    let model = {
        let mut names = NAMES.lock().ok()?;
        match names.iter().find(|(i, _)| *i == item) {
            Some((_, m)) => m.clone(),
            None => {
                let name = super::equip::item_name(base, item)?;
                let m = lib.keyblades.iter().find(|(n, _)| *n == name.to_lowercase()).map(|(_, m)| m.clone());
                log(&format!("KH2 model: KH1's {name} ({item:#x}): {}", m.as_deref().map_or("no KH2 Keyblade of that name, KH1's drawn".to_string(), |m| format!("KH2's {m}"))));
                names.push((item, m.clone()));
                m
            }
        }
    }?;
    match lib.weapons.get(&model) {
        Some(Weapon::Ready(l)) => return Some(l.clone()),
        Some(_) => return None,
        None => {}
    }
    lib.weapons.insert(model.clone(), Weapon::Loading);
    let (install, source) = (lib.install.clone(), lib.source);
    std::thread::spawn(move || {
        let r = install.read_any(source, &format!("obj/{model}.mdlx")).map_err(|e| format!("{model}: {e}")).and_then(|b| Loaded::new(&model, &b));
        let w = match r {
            Ok(l) => {
                log(&format!("KH2 model: {model}: {} bones, {} triangles, {} textures", l.model.bones.len(), l.model.triangle_count(), l.model.textures.len()));
                Weapon::Ready(Arc::new(l))
            }
            Err(e) => {
                log(&format!("KH2 model: {e}; KH1's Keyblade drawn"));
                Weapon::Failed
            }
        };
        if let Some(Ok(lib)) = LIB.get() {
            if let Ok(mut g) = lib.lock() {
                g.weapons.insert(model, w);
            }
        }
    });
    None
}

/// Sora's Keyblade (KH1 item id), if the save reads.
unsafe fn held_weapon(base: usize) -> Option<u8> {
    let save = std::ptr::read_unaligned((base + SAVE_PTR_RVA) as *const u64) as usize;
    (save >= 0x10000 && super::crash::readable(save + 0x36)).then(|| std::ptr::read_unaligned((save + 0x36) as *const u8))
}

/// The left weapon's KH1 item (Valor's, Master's, Final's), if he holds two.
unsafe fn left_weapon(base: usize) -> Option<u8> {
    match super::form::current()? {
        Kind::Final => Some(super::equip::final_stored(base)),
        Kind::Master => Some(super::equip::master_stored(base)),
        // (The Dream Sword by default: no KH2 Keyblade of its name, so KH1's is drawn.)
        Kind::Valor => Some(super::equip::valor_stored(base)),
        Kind::Wisdom | Kind::Limit | Kind::Anti => None,
    }
}

/// `pose`, each frame KH2 poses him: the body and the right Keyblade (KH2 model space), KH1's
/// placement, and KH1's size against KH2's.
pub fn note_pose(body: &[Mat4], weapon: &[Mat4], to_world: &Mat4, ratio: f32) {
    if !on() {
        return;
    }
    if let Ok(mut g) = POSE.try_lock() {
        match g.as_mut() {
            Some(f) if f.body.len() == body.len() && f.weapon.len() == weapon.len() => {
                f.body.copy_from_slice(body);
                f.weapon.copy_from_slice(weapon);
                f.to_world = *to_world;
                f.ratio = ratio;
                f.at = Instant::now();
            }
            _ => *g = Some(Frame { body: body.to_vec(), weapon: weapon.to_vec(), to_world: *to_world, ratio, at: Instant::now() }),
        }
    }
}

/// `leftblade`, each KH2 step: KH2's left Keyblade (KH2 model space), if the form has one.
pub fn note_left(bones: Option<&[Mat4]>) {
    if !on() {
        return;
    }
    if let (Ok(mut g), Some(b)) = (LEFT.try_lock(), bones.filter(|b| !b.is_empty())) {
        *g = Some((b.to_vec(), Instant::now()));
    }
}

/// Draw params (KH1's are 0x60 bytes; 16-aligned as KH1's own).
#[repr(C, align(16))]
pub struct Params(pub [u8; 0x100]);

/// What `leftblade::queue` draws Sora with instead of KH1's params: his meshes masked off;
/// `collapse`: his Keyblade's bones shrunk into the grip for the call (KH2's is drawn in its
/// place; the weapon stays queued, so its trail and effects go on as KH1 has them).
pub struct Hide {
    pub params: Params,
    pub collapse: bool,
}

fn refuse(why: &'static str, track: &Mutex<Track>) -> Option<Hide> {
    stat(|s| *s.refused.entry(why).or_default() += 1);
    if let Ok(mut t) = track.lock() {
        t.seq += 1;
        t.at = Some(Instant::now());
        t.hidden = false;
        t.prepared = None;
    }
    None
}

fn masked(params: usize, collapse: bool) -> Hide {
    let mut p = Params([0u8; 0x100]);
    unsafe {
        std::ptr::copy_nonoverlapping(params as *const u8, p.0.as_mut_ptr(), 0x60);
        std::ptr::write_unaligned(p.0.as_mut_ptr().add(P_MASK) as *mut u32, 0xFFFF_FFFF);
    }
    Hide { params: p, collapse }
}

/// KH1's model queue, for the field Sora's draw (`leftblade::queue`): the params to draw him with
/// instead, or `None` to draw him as KH1 does. Ours is prepared here, for the world pass.
pub unsafe fn hide(params: usize) -> Option<Hide> {
    stat(|s| s.queues += 1);
    if !on() {
        return refuse("off", &MAIN);
    }
    if !super::fx12::usable() {
        return refuse("renderer off", &MAIN);
    }
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 || super::kh2fx::paused(base).is_some() {
        return refuse("paused", &MAIN);
    }
    if super::pose::menu_sora_within(150) {
        return refuse("pause menu's Sora", &MAIN);
    }
    if replaying() {
        return refuse("interpolated frames", &MAIN);
    }
    let Some(Ok(lib)) = LIB.get() else { return refuse("not loaded", &MAIN) };
    let Ok(mut lib) = lib.lock() else { return refuse("not loaded", &MAIN) };
    let Some(body_model) = body(&lib) else { return refuse("not loaded", &MAIN) };
    let Ok(pose) = POSE.lock() else { return refuse("no pose", &MAIN) };
    let Some(f) = pose.as_ref().filter(|f| f.at.elapsed().as_secs_f32() < FRESH_S) else { return refuse("no pose", &MAIN) };
    let has_weapon = std::ptr::read_unaligned((params + P_WEAPON) as *const u64) != 0;
    let k = if f.ratio.is_finite() && f.ratio > 0.1 { f.ratio } else { 1.0 };
    let mut parts = vec![(body_model, f.body.clone())];
    let mut ours = 0u32;
    if has_weapon && !f.weapon.is_empty() {
        if let Some(w) = held_weapon(base).and_then(|w| weapon_model(&mut lib, w)) {
            parts.push((w, f.weapon.clone()));
            ours |= 1;
        }
    }
    let left = LEFT.lock().ok().and_then(|g| g.as_ref().filter(|(_, t)| t.elapsed().as_secs_f32() < FRESH_S).map(|(b, _)| b.clone()));
    if let (true, Some(b), true) = (has_weapon, left, super::form::two_blades()) {
        if let Some(w) = left_weapon(base).and_then(|w| weapon_model(&mut lib, w)) {
            parts.push((w, b));
            ours |= 2;
        }
    }
    let prepared = Prepared { parts, place: f.to_world * Mat4::from_scale(glam::Vec3::splat(k)) };
    drop(pose);
    drop(lib);
    LEFT_OURS.store(ours & 2 != 0, Ordering::Relaxed);
    if let Ok(mut t) = MAIN.lock() {
        t.seq += 1;
        t.at = Some(Instant::now());
        t.hidden = true;
        t.prepared = Some(Arc::new(prepared));
    }
    stat(|s| s.hidden += 1);
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 3 {
        log(&format!("KH2 model: KH1's Sora held back (Keyblades ours {ours:#b})"));
    }
    // (Anti Form holds no Keyblade: KH1's is shrunk away though none of ours replaces it.)
    Some(masked(params, ours & 1 != 0 || super::form::in_anti()))
}

/// `pose`: the pause menu's copy of Sora (`instance`), KH1's menu animation on KH2's skeleton
/// (KH1 world space, KH2 units) and KH1's size against KH2's.
pub fn note_menu(instance: usize, kh2: Vec<Mat4>, ratio: f32) {
    if let Ok(mut g) = MENU_POSE.lock() {
        *g = Some((instance, kh2, ratio, Instant::now()));
    }
}

/// The pause menu's copy of Sora, if `instance` is it.
pub fn is_menu_copy(instance: usize) -> bool {
    on() && MENU_POSE.lock().ok().is_some_and(|g| g.as_ref().is_some_and(|(i, _, _, t)| *i == instance && t.elapsed().as_secs_f32() < 0.5))
}

/// KH1's model queue, for the pause menu's copy of Sora: masked off, ours drawn in its place
/// (its pose and placement from KH1's own, `note_menu`; the menu's camera in the world pass).
pub unsafe fn hide_menu(instance: usize, params: usize) -> Option<Hide> {
    if !on() || !super::fx12::usable() {
        return refuse("off", &MENU);
    }
    if replaying() {
        return refuse("interpolated frames", &MENU);
    }
    let Some(Ok(lib)) = LIB.get() else { return refuse("not loaded", &MENU) };
    let Some(body_model) = lib.lock().ok().and_then(|l| body(&l)) else { return refuse("not loaded", &MENU) };
    let Some((bones, k)) = MENU_POSE.lock().ok().and_then(|g| g.as_ref().filter(|(i, _, _, t)| *i == instance && t.elapsed().as_secs_f32() < FRESH_S).map(|(_, b, k, _)| (b.clone(), *k))) else {
        return refuse("menu: no pose", &MENU);
    };
    let k = if k.is_finite() && k > 0.1 { k } else { 1.0 };
    if let Ok(mut t) = MENU.lock() {
        t.seq += 1;
        t.at = Some(Instant::now());
        t.hidden = true;
        t.prepared = Some(Arc::new(Prepared { parts: vec![(body_model, bones)], place: Mat4::from_scale(glam::Vec3::splat(k)) }));
    }
    stat(|s| s.menu_hidden += 1);
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 3 {
        log(&format!("KH2 model: the pause menu's Sora ({instance:#x}) held back"));
    }
    Some(masked(params, false))
}

/// Whether KH1's left Keyblade (`leftblade::second`) gives way to ours this frame.
pub fn left_is_ours() -> bool {
    on() && MAIN.lock().ok().is_some_and(|t| t.hidden) && LEFT_OURS.load(Ordering::Relaxed)
}

/// KH1's interpolated replay (`0x140102740`): counted for the log.
fn note_replay() {
    if on() {
        stat(|s| s.replays += 1);
    }
}

/// The world pass (`kh2fx`): ours drawn wherever KH1's Sora was held back.
pub fn draw() {
    if !on() {
        return;
    }
    stat(|s| s.passes += 1);
    let base = BASE.load(Ordering::Relaxed);
    for (track, menu) in [(&MAIN, false), (&MENU, true)] {
        // The latest queue decides; with no queue since the last pass, as the last pass did.
        let prepared = {
            let Ok(mut t) = track.lock() else { continue };
            if t.seq != t.seen {
                t.seen = t.seq;
                t.drew = t.hidden;
            } else if t.at.is_none_or(|a| a.elapsed().as_secs_f32() > STALE_S) {
                // (No Sora queued for a while: the menu closed, or he isn't drawn.)
                t.drew = false;
            }
            if !t.drew {
                continue;
            }
            t.prepared.clone()
        };
        let ok = prepared.is_some_and(|p| unsafe { draw_prepared(base, &p) });
        stat(|s| match (ok, menu) {
            (true, false) => s.drawn += 1,
            (true, true) => s.menu_drawn += 1,
            (false, _) => s.missed += 1,
        });
        if ok {
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) < 3 {
                log(&format!("KH2 model: drew {}{}", if menu { "the pause menu's Sora " } else { "" }, prepared_names(track)));
            }
        }
    }
    report();
}

fn prepared_names(track: &Mutex<Track>) -> String {
    track.lock().ok().and_then(|t| t.prepared.as_ref().map(|p| p.parts.iter().map(|(l, _)| l.name.as_str()).collect::<Vec<_>>().join(", "))).unwrap_or_default()
}

/// Skins and draws a prepared model (KH1's camera of this pass).
unsafe fn draw_prepared(base: usize, p: &Prepared) -> bool {
    let mut vertices: Vec<kh2_fx::Vertex> = Vec::with_capacity(12_000);
    let mut batches = Vec::new();
    let mut textures = Vec::new();
    for (l, bones) in &p.parts {
        let first_tex = textures.len();
        for (i, t) in l.model.textures.iter().enumerate() {
            textures.push(super::fx12::MeshTexture { key: l.key | i as u64, width: t.width, height: t.height, rgba: &t.rgba });
        }
        let mut runs: Vec<(usize, usize)> = Vec::new(); // (group, first vertex)
        kh2_anim::skin::skin(&l.model, bones, p.place, |g, c, uv| {
            if runs.last().is_none_or(|r| r.0 != g) {
                runs.push((g, vertices.len()));
            }
            for k in 0..3 {
                vertices.push(kh2_fx::Vertex { pos: c[k].to_array(), uv: uv[k], color: [1.0; 4] });
            }
        });
        for (i, &(g, first)) in runs.iter().enumerate() {
            let end = runs.get(i + 1).map_or(vertices.len(), |r| r.1);
            let group = &l.model.groups[g];
            batches.push(super::fx12::MeshBatch { first: first as u32, count: (end - first) as u32, texture: first_tex + group.texture, translucent: group.translucent() });
        }
    }
    let Some((view, proj)) = super::fx12::camera(base) else { return false };
    match super::fx12::draw_mesh(base, &vertices, &batches, &textures, view, proj) {
        Some(fbi) => {
            stat(|s| *s.frame_buffers.entry(fbi).or_default() += 1);
            true
        }
        None => false,
    }
}
