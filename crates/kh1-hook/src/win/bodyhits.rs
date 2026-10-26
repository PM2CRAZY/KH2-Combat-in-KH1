//! KH2's hit windows on Sora's own body as KH1 attack objects (testing branch; the tester, thread
//! "What is still missing for native integration into KH1", m5 answer "(b)": the body hits of
//! Vicinity Break, Explosion and Magnet Burst into KH1's own collision pass). `docs/kh1-anim-slots.md`
//! §7.z.8.
//!
//! Before: `takeover::kh2_hits` posed KH2's rig where KH1 has him, swept KH2's shapes against KH1's
//! enemies itself and landed each hit through KH1's contact (our own sweep). Now each body window is
//! a KH1 attack object of Sora's, made with KH1's own constructor, tied to his motion and to a KH1
//! attack group of its own, and KH1's collision pass collides it with every body itself.
//!
//! KH1's collision pass (Steam, Checked (decomp)):
//! - The pass task `0x1402B4460` (priority 0x13881) first runs `0x1402B47A0` over the attack pool
//!   (`0x142DB4710`, 64 objects of 0xC0): an object tied to a motion (activity `(flags >> 2) & 3`
//!   == 2) is live only while its owner plays the request it took (`+0x58`, captured on its first
//!   pass from the owner's `+0x164`; another request marks it dead, `0x200000`, and it is freed) and
//!   the owner's motion events hold its attack group's bit (`+0x1CC` bit `0x10000 << group`, the
//!   group from the object's flags bits 4..5, from its data's `+0x34`). A live object goes to
//!   `0x1402B3800 (object, entries, count) -> count`, which puts its entries (0x40 bytes each:
//!   `+0` team, `+8` owner, `+0x10` hit bit, `+0x18` the object, `+0x20` shape type, `+0x24`
//!   radius, `+0x28` half height, `+0x30` centre) into the frame's list and tests each against the
//!   entries already there (`0x1402B3BE0 (entry, entries, count)`: attack against attack, KH1's
//!   clash). Then every hittable actor's body parts are put in and tested against the attacks
//!   (`0x1402B3BE0` again): an attack entry meeting a body part (`0x1402B5EF0`) whose actor doesn't
//!   hold the attack's hit bit (`+0x3E0`) makes the contact `0x1402B4060`, the hit record and its
//!   reaction (KH1's own path, `probe::hit` and `kh2damage` as for every swing of Sora's).
//! - Shape types of `0x1402B5EF0`: 1 an ellipsoid (radius across, half height up; two of them meet
//!   when the gap divided by the summed sizes is inside the unit sphere), 2 a column, 3 a box. KH2's
//!   overlap (`combat_core::Shape`) is the same test: KH2's ellipsoid and sphere (shapes 0 and 3) are
//!   KH1's 1, its column (1) KH1's 2; KH2's cube (2) never hits in KH2 and is left out.
//! - KH1's own builder makes a bone-tied object a row of spheres from one point on a bone to
//!   another (`0x1402B3D50`); KH2's body shapes are ellipsoids on his root (Explosion's group 19:
//!   lift 120, 350 across, 180 high; Vicinity Break's 21: 60, 450, 150; Magnet Burst's 22: 80, 750,
//!   450 and 23: 90, 130, 160) and a sphere of 400 on bone 67 (Magnet Burst's last hit, group 3)
//!   (`P_EX100.mdlx`, Checked (files)). So `0x1402B3800` is hooked: for an object of ours it puts in
//!   KH2's shape (the ellipsoid's own half height) and KH2's sweep between frames (copies one radius
//!   apart while the gap left is over 1.5 radii, none on the window's first frame,
//!   `combat_core::sweep_fill`), and tests them with KH1's own `0x1402B3BE0`; any other object goes
//!   to KH1's builder. The data's clash word has bit 16 ("spent"), so KH1 runs no wall test for them
//!   (`0x1402B3800`'s last part; KH2 runs none for his body).
//! - Where: root shapes at his position (`+0x10`), the offset turned by his yaw (`+0x3C`, as
//!   `kh2_hits` took it for KH2's facing) and raised by the lift (KH1's world is y down); bone shapes
//!   on KH1's bone that has KH2's bone's frame in the bind poses (KH2 67 → KH1 70,
//!   `docs/kh2-effects.md`, Checked (files)), placed through KH1's bone point (`0x14029EB20`, the
//!   raw bone id `0x4000 | n`) and its drawn matrix (`+0x1B4`) for the offset.
//! - KH2's re-hit interval (ATKP `+0x26`, 60 Hz frames: Explosion's 911 every 8): KH1's own re-arm
//!   (`0x1402B3380`, attack `+0x50`) runs one clock for the whole object and takes the bit off
//!   every actor at once; KH2's runs per target from its hit (`HitRecord::mark` / `tick`). So the
//!   bit is taken off each actor `interval` frames after its own hit (`hit`, `rearm`), and KH1's own
//!   re-arm is left off (`+0x50` 0).
//!
//! The objects: one per shape of each body window, made when our slot of the move starts playing
//! (`frame`, from `kh2slots::frame`), through KH1's row object maker `0x1402B3270 (actor, hit bit,
//! data)` (the one KH1's attack rows use: Sora's effect source for the spark, the "connected" flag
//! `+0x50` of his combat info, which KH1's Attack step reads), with a hit bit of its own
//! (`0x1402B3760`), its row's effect and sound bytes (`+0xA8`, flag `0x400000`, as KH1's row start
//! sets them), its request taken at once (`+0x58`). Their attack data: a copy of Sora's KH1 swing
//! row's (finisher row's for a KH2 blow; ground or air by his KH1 state), as `kh2_hits` landed them,
//! with KH2's reaction on it (KH1's launch, kind 5, for a blow; a flinch, 3, for a flinch), its
//! attack group (`+0x34`), its bone `0x8003` (KH1's body centre: KH1's own placements of the object,
//! its creation and first match, read it), and the Recoil row's clash rules (`kh1rows::kh2_clash`).
//! `kh1rows::kh2_atkp` gives the window's ATKP for it, so KH2's power, Magic scaling, launch and
//! knock-back come in as for KH2's other moves (`kh2damage`, `combat::scale_record`, `knockback`).
//! KH1 frees them itself when the motion's request changes.
//!
//! The Magnet pull (ATKP reaction 5, Magnet Burst's 1900) stays ours, as before (KH1 has no such
//! reaction; `combat::Strikes::native_pull` from `kh2_hits`), now for the enemies KH1's pass caught.
//!
//! Steam only (new addresses); elsewhere `kh2_hits` sweeps as before. Cost: one hook on the pass's
//! per-attack builder (a lookup in a 64-entry table per live attack object); a few entries per live
//! window; at most one data copy per (move, window, air) for the session.

use super::addr::{Build, Rva};
use super::{log, pose};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

/// KH1's per-attack entry builder of the collision pass (Steam): `push rbp; push rsi; push rdi;
/// sub rsp, 0x80` (Checked (exe)).
const ENTRIES_STEAM: usize = 0x2B3800;
const ENTRIES_PROLOGUE: [u8; 11] = [0x40, 0x55, 0x56, 0x57, 0x48, 0x81, 0xEC, 0x80, 0x00, 0x00, 0x00];
/// KH1's entry test `(entry, entries, count) -> contacts` (Steam), its first bytes checked before
/// the first call (Checked (exe)).
const TEST_STEAM: usize = 0x2B3BE0;
const TEST_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x6C, 0x24, 0x18];
/// KH1's attack pool (Steam), 64 objects of 0xC0.
const POOL_STEAM: usize = 0x2DB4710;
const POOL_COUNT: usize = 64;
const OBJ: usize = 0xC0;
/// KH1's row object maker, its free hit bit, its object free and its bone point (Epic RVAs, mapped).
const NEW_ATTACK_RVA: Rva = Rva(0x2B10E0);
const FREE_BIT_RVA: Rva = Rva(0x2B15D0);
const FREE_ATTACK_RVA: Rva = Rva(0x2B13D0);
const BONE_POINT_RVA: Rva = Rva(0x29C990);
/// KH1's battle data block (our data copies sit within an offset's reach, as `kh1rows`').
const BLOCK_RVA: Rva = Rva(0x2D23740);
/// An attack data entry and our copies' spacing.
const DATA: usize = 0x70;
const DATA_SLOT: usize = 0x80;
const DATA_PAGE: usize = 0x2000;
/// KH1's body centre point id (`0x14029EB20`'s 0x8003: the first body part's centre).
const BODY_BONE: u32 = 0x8003;
/// The most entries KH1's list takes before it writes over its last (`0x1402B3D50`).
const LIST_CAP: i32 = 0xFD;
/// KH2's body shape kinds (`kh2_data::sora::BoneShape::kind`): attack and guard.
const ATTACK_KIND: u8 = 6;
const GUARD_KIND: u8 = 13;
/// KH2 bones on Sora's body shapes and KH1's bone with the same bind frame (`docs/kh2-effects.md`,
/// Checked (files)).
const BONES: [(usize, u32); 1] = [(67, 70)];

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG: AtomicUsize = AtomicUsize::new(0);
static ON: AtomicBool = AtomicBool::new(false);
/// Sora's KH1 rows are readable (the template rows of our data, `kh1rows::kh1_row`).
static READY: AtomicBool = AtomicBool::new(false);
static TEST_OK: AtomicBool = AtomicBool::new(false);
static LOGGED: AtomicU32 = AtomicU32::new(0);

fn note(s: String) {
    if LOGGED.fetch_add(1, Ordering::Relaxed) < 80 {
        log(&s);
    }
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// One KH2 body shape, as KH1 tests it.
#[derive(Clone, Copy, Debug)]
struct BodyShape {
    /// KH1's shape type (1 ellipsoid / sphere, 2 column).
    kh1_type: u32,
    radius: f32,
    height: f32,
    /// KH1's bone (`None`: his root).
    bone: Option<u32>,
    offset: [f32; 3],
    lift: f32,
}

/// One of our objects in the pool.
#[derive(Clone, Copy, Debug)]
struct Live {
    obj: usize,
    data: usize,
    motion: u16,
    atkp: u16,
    shape: BodyShape,
    /// KH2's re-hit interval (60 Hz frames; 0 once).
    interval: u64,
    /// The Magnet catch (orbit radius from the ATKP's knock-back values, swirl), reaction 5.
    magnet: Option<(i16, i16, i16)>,
    /// Last game frame's centre (KH1 world) and that frame's number.
    prev: Option<([f32; 3], u64)>,
    /// Hits: (actor, `takeover::TICKS` at the hit); 0 actor free.
    hits: [(usize, u64); 8],
    /// Enemies its Magnet caught (actor, orbit radius, swirl).
    caught: [(usize, f32, f32); 8],
}

struct State {
    live: [Option<Live>; POOL_COUNT],
    /// Our data copies: (motion, window index, air, shape index) → address.
    data: Vec<((u16, usize, bool, usize), usize)>,
    page: usize,
    /// The move playing: (motion, KH1 time, request).
    playing: Option<(u16, f32, u32)>,
}

static STATE: Mutex<State> = Mutex::new(State { live: [None; POOL_COUNT], data: Vec::new(), page: 0, playing: None });

/// KH2's body hits run through KH1's pass (Steam, the hook in, Sora's KH1 rows read).
pub fn native() -> bool {
    ON.load(Ordering::Relaxed) && READY.load(Ordering::Relaxed)
}

pub unsafe fn install() {
    if Build::current() != Build::Steam {
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    let have = std::slice::from_raw_parts((base + TEST_STEAM) as *const u8, TEST_BYTES.len());
    if have != TEST_BYTES {
        log(&format!("KH2 body hits: KH1's entry test has unexpected bytes {have:02x?}: KH2's own sweep stays"));
        return;
    }
    TEST_OK.store(true, Ordering::Relaxed);
    if super::hook::install(base + ENTRIES_STEAM, &ENTRIES_PROLOGUE, entries as *const () as usize, &ORIG, "KH1's attack entries (KH2's body hits)").is_some() {
        ON.store(true, Ordering::Relaxed);
    }
}

fn pool_index(obj: usize) -> Option<usize> {
    let pool = BASE.load(Ordering::Relaxed) + POOL_STEAM;
    let i = obj.checked_sub(pool)?;
    (i % OBJ == 0 && i / OBJ < POOL_COUNT).then_some(i / OBJ)
}

/// Our object is still the one we made (in use, our data).
unsafe fn still_ours(l: &Live) -> bool {
    rd::<u32>(l.obj) & 0x800 != 0 && pose::ptr(rd::<u32>(l.obj + 0x5C)) == l.data
}

/// Each frame base Sora is posed (`kh2slots::frame`): when our slot of a move with hit windows on
/// his body starts (a new motion, its time going back, or a new request), our objects for them.
pub unsafe fn frame(actor: usize) {
    if !ON.load(Ordering::Relaxed) || actor < 0x10000 {
        return;
    }
    if !READY.load(Ordering::Relaxed) {
        if super::kh1rows::kh1_row(template_row(false, false)).is_none() {
            return;
        }
        READY.store(true, Ordering::Relaxed);
        log("KH2 body hits: through KH1's collision pass (KH1 attack objects with KH2's shapes)");
    }
    let Some(kh2) = pose::kh2() else { return };
    let Ok(mut g) = STATE.try_lock() else { return };
    // (Ours KH1 freed: forgotten.)
    for e in g.live.iter_mut() {
        if e.is_some_and(|l| !still_ours(&l)) {
            *e = None;
        }
    }
    let playing = if super::shared::active() && !super::form::in_form() { super::kh2slots::now_playing(actor) } else { None };
    let req = rd::<u32>(actor + 0x164);
    let Some((m, t)) = playing else {
        g.playing = None;
        return;
    };
    let new = match g.playing {
        Some((pm, pt, pr)) => pm != m || t < pt || pr != req,
        None => true,
    };
    g.playing = Some((m, t, req));
    if !new {
        return;
    }
    // A new play: the last one's objects go (KH2: a new swing, its hits counted afresh).
    let base = BASE.load(Ordering::Relaxed);
    let free: unsafe extern "system" fn(usize) = std::mem::transmute(base + FREE_ATTACK_RVA);
    for e in g.live.iter_mut() {
        if let Some(l) = e.take() {
            if still_ours(&l) {
                free(l.obj);
            }
        }
    }
    let Some(clip) = kh2.character.moveset.clips.get(&m) else { return };
    let windows = super::kh2slots::body_windows(clip);
    if windows.is_empty() {
        return;
    }
    let air = matches!(rd::<i32>(actor + 0x70), 1 | 2);
    let new_attack: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + NEW_ATTACK_RVA);
    let free_bit: unsafe extern "system" fn() -> u32 = std::mem::transmute(base + FREE_BIT_RVA);
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    let mut made = Vec::new();
    for w in &windows {
        let Some(a) = kh2.battle.attack_for_level(w.attack, 0, 0).cloned() else { continue };
        let shapes: Vec<&kh2_data::sora::BoneShape> = kh2.rig.body_shapes.iter().filter(|s| s.group == w.group && (s.kind == ATTACK_KIND || s.kind == GUARD_KIND)).collect();
        for (si, s) in shapes.iter().enumerate() {
            let kh1_type = match s.shape {
                0 | 3 => 1u32,
                1 => 2,
                // (KH2's cube never hits, `combat_core::Shape::overlaps`.)
                _ => continue,
            };
            let bone = match s.bone {
                None => None,
                Some(b) => match BONES.iter().find(|x| x.0 == b) {
                    Some(&(_, k)) => Some(k),
                    None => {
                        note(format!("KH2 body hits: KH2 bone {b} (ATKP {}) has no KH1 bone: left out", w.attack));
                        continue;
                    }
                },
            };
            let shape = BodyShape { kh1_type, radius: s.radius, height: s.height, bone, offset: s.offset, lift: s.lift };
            let Some(data) = data_for(&mut g, base, m, w.index, air, si, w.channel, &a, s.radius) else { continue };
            let fx = template_fx(air, super::kh1rows::throws(&a));
            let bit = free_bit();
            if bit == 0 {
                note(format!("KH2 body hits: no free hit bit for ATKP {} (motion {m})", w.attack));
                continue;
            }
            // (KH1's maker clears his combat info's "connected" flag for a new swing: this isn't one.)
            let connected = if info > 0x10000 { rd::<u32>(info + 0x50) } else { 0 };
            let obj = new_attack(actor, bit, data);
            if info > 0x10000 {
                wr(info + 0x50, rd::<u32>(info + 0x50) | connected);
            }
            let Some(i) = pool_index(obj) else {
                note(format!("KH2 body hits: KH1's maker gave {obj:#x} for ATKP {} (pool full?)", w.attack));
                continue;
            };
            if let Some(fx) = fx {
                wr(obj, rd::<u32>(obj) | 0x40_0000);
                wr(obj + 0xA8, fx);
            }
            // Tied to the request playing from now (KH1 takes it on its first pass otherwise, a frame
            // late for a window from frame 0).
            wr(obj + 0x58, req);
            let magnet = (character_core::reaction::category(a.enemy_reaction) == 5).then_some((a.knockback_1, a.knockback_2, a.unknown_16));
            g.live[i] = Some(Live { obj, data, motion: m, atkp: w.attack, shape, interval: a.interval as u64, magnet, prev: None, hits: [(0, 0); 8], caught: [(0, 0.0, 0.0); 8] });
            made.push(format!("ATKP {} group {} on KH1 group {} ({:?} r {} h {} lift {}, interval {})", w.attack, w.group, w.channel, bone, s.radius, s.height, s.lift, a.interval));
        }
    }
    if !made.is_empty() {
        note(format!("KH2 body hits: motion {m} (request {req:#x}, {}): KH1 attack objects {}", if air { "air" } else { "ground" }, made.join("; ")));
    }
}

/// KH1's row of Sora's that KH2's hits on his body are landed with (as `combat::Strikes::strike_as`
/// takes it: swing, or finisher for a KH2 blow; ground or air by his KH1 state).
fn template_row(air: bool, throws: bool) -> usize {
    use super::kh1rows::Kind;
    match (air, throws) {
        (false, false) => Kind::Swing,
        (false, true) => Kind::Finisher,
        (true, false) => Kind::AirSwing,
        (true, true) => Kind::AirFinisher,
    }
    .template_row()
}

unsafe fn template_fx(air: bool, throws: bool) -> Option<[u8; 3]> {
    super::kh1rows::kh1_row(template_row(air, throws)).map(|r| r.1)
}

/// Our attack data for one body shape of a window (made the first time, refreshed each time: the
/// Recoil row's clash rules can change).
#[allow(clippy::too_many_arguments)]
unsafe fn data_for(g: &mut State, base: usize, motion: u16, window: usize, air: bool, shape: usize, channel: u8, a: &kh2_formats::battle::Atkp, radius: f32) -> Option<usize> {
    let key = (motion, window, air, shape);
    let d = match g.data.iter().find(|e| e.0 == key) {
        Some(e) => e.1,
        None => {
            if g.page == 0 {
                g.page = super::kh1rows::near(base + BLOCK_RVA, DATA_PAGE).unwrap_or(usize::MAX);
                note(format!("KH2 body hits: attack data copies at {:#x}", g.page));
            }
            if g.page == usize::MAX || (g.data.len() + 1) * DATA_SLOT > DATA_PAGE {
                return None;
            }
            let d = g.page + g.data.len() * DATA_SLOT;
            g.data.push((key, d));
            d
        }
    };
    let throws = super::kh1rows::throws(a);
    let (src, _) = super::kh1rows::kh1_row(template_row(air, throws))?;
    std::ptr::copy_nonoverlapping(src as *const u8, d as *mut u8, DATA);
    // KH2's reaction on it, as our rows have (`kh1rows::make`).
    let kind = if throws {
        5u32
    } else if character_core::reaction::category(a.enemy_reaction) == 3 {
        3
    } else {
        rd::<u32>(d + 0xC)
    };
    wr(d + 8, kind);
    wr(d + 0xC, kind);
    wr(d + 0x10, BODY_BONE);
    wr(d + 0x34, (rd::<u32>(d + 0x34) & !3) | channel as u32);
    wr(d + 0x54, 0.0f32);
    wr(d + 0x58, 0.0f32);
    wr(d + 0x5C, radius.round() as i32);
    let (t1, p1) = (rd::<u32>(d + 0x38) & 7, rd::<i32>(d + 0x3C));
    let (t, p) = if super::recoil::kh2() { super::kh1rows::kh2_clash(Some(a), t1, p1) } else { (t1, p1) };
    // (Bit 16: "spent", no wall test.)
    wr(d + 0x38, (rd::<u32>(d + 0x38) & !7) | t | 0x1_0000);
    wr(d + 0x3C, p);
    Some(d)
}

/// The window's ATKP for an attack object of ours (`kh1rows::kh2_atkp`).
pub unsafe fn atkp(attack: usize) -> Option<kh2_formats::battle::Atkp> {
    if !native() {
        return None;
    }
    let i = pool_index(attack)?;
    let id = STATE.try_lock().ok()?.live[i].filter(|l| l.obj == attack && still_ours(l))?.atkp;
    pose::kh2()?.battle.attack_for_level(id, 0, 0).cloned()
}

/// `probe::hit`: KH1's contact made a hit record with `attack` on `target`: its time (KH2's re-hit
/// interval) and a Magnet's catch.
pub unsafe fn hit(attack: usize, target: usize) {
    if !native() || target < 0x10000 {
        return;
    }
    let Some(i) = pool_index(attack) else { return };
    let Ok(mut g) = STATE.try_lock() else { return };
    let Some(l) = g.live[i].as_mut().filter(|l| l.obj == attack && still_ours(l)) else { return };
    let now = super::takeover::TICKS.load(Ordering::Relaxed);
    if l.interval > 0 {
        if let Some(h) = l.hits.iter_mut().find(|h| h.0 == target || h.0 == 0) {
            *h = (target, now);
        }
    }
    if let Some((k1, k2, swirl)) = l.magnet {
        if !l.caught.iter().any(|c| c.0 == target) {
            // KH2 picks the orbit radius between the two knock-back values (as `strike_as`).
            let (a, c) = (k1 as f32, k2 as f32);
            let t = ((target >> 4) % 97) as f32 / 96.0;
            if let Some(slot) = l.caught.iter_mut().find(|c| c.0 == 0) {
                *slot = (target, a + (c - a) * t, swirl as f32);
            }
        }
    }
    note(format!("KH2 body hit: ATKP {} (motion {}) from KH1's pass on actor {target:#x}", l.atkp, l.motion));
}

/// Enemies a live Magnet window of ours holds: (actor, its centre (KH2 y-up), orbit radius, swirl)
/// (`takeover::kh2_hits` → `combat::Strikes::native_pull`).
pub fn catches() -> Vec<(usize, [f32; 3], f32, f32)> {
    let mut out = Vec::new();
    if !native() {
        return out;
    }
    let Ok(g) = STATE.try_lock() else { return out };
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    for l in g.live.iter().flatten() {
        // (Live this frame or the last: the pass ran for it.)
        let Some((c, f)) = l.prev else { continue };
        if frame.saturating_sub(f) > 1 || unsafe { !still_ours(l) } {
            continue;
        }
        for &(a, r, s) in l.caught.iter().filter(|c| c.0 != 0) {
            out.push((a, [c[0], -c[1], c[2]], r, s));
        }
    }
    out
}

/// The shape's centre now (KH1 world, y down).
unsafe fn centre(owner: usize, s: &BodyShape) -> Option<[f32; 3]> {
    let base = BASE.load(Ordering::Relaxed);
    let c = match s.bone {
        None => {
            let p = rd::<[f32; 3]>(owner + 0x10);
            let yaw = rd::<f32>(owner + 0x3C);
            let (sn, cs) = yaw.sin_cos();
            // KH2's object frame: its offset turned by the facing about up (glam's rotation about y).
            let [ox, oy, oz] = s.offset;
            [p[0] + ox * cs + oz * sn, p[1] - (oy + s.lift), p[2] - ox * sn + oz * cs]
        }
        Some(b) => {
            let point: unsafe extern "system" fn(usize, u32, f32, *mut [f32; 4]) = std::mem::transmute(base + BONE_POINT_RVA);
            let mut out = [0.0f32; 4];
            point(owner, 0x4000 | b, 1.0, &mut out);
            let mut c = [out[0], out[1], out[2]];
            if s.offset != [0.0; 3] {
                // KH1's drawn matrix of that bone (row vectors), its axes for the offset.
                let inst = pose::ptr(rd::<u32>(owner + 0x15C));
                let mats = if inst > 0x10000 { pose::ptr(rd::<u32>(inst + 0x1B4)) } else { 0 };
                if mats > 0x10000 {
                    let m = rd::<[f32; 16]>(mats + b as usize * 0x40);
                    for k in 0..3 {
                        c[k] += s.offset[0] * m[k] + s.offset[1] * m[4 + k] + s.offset[2] * m[8 + k];
                    }
                }
            }
            c[1] -= s.lift;
            c
        }
    };
    c.iter().all(|v| v.is_finite()).then_some(c)
}

type EntriesF = unsafe extern "system" fn(usize, usize, i32) -> i32;
type TestF = unsafe extern "system" fn(usize, usize, i32) -> i32;

/// KH1's `0x1402B3800 (object, entries, count) -> count`: ours get KH2's shape and sweep.
unsafe extern "system" fn entries(obj: usize, list: usize, count: i32) -> i32 {
    let f: EntriesF = std::mem::transmute(ORIG.load(Ordering::Acquire));
    let Some(i) = pool_index(obj) else { return f(obj, list, count) };
    let Ok(mut g) = STATE.try_lock() else { return f(obj, list, count) };
    let Some(l) = g.live[i].as_mut().filter(|l| l.obj == obj && still_ours(l)) else {
        drop(g);
        return f(obj, list, count);
    };
    if !TEST_OK.load(Ordering::Relaxed) || list < 0x10000 {
        return count;
    }
    let owner = pose::ptr(rd::<u32>(obj + 0x34));
    let Some(c) = (if owner > 0x10000 { centre(owner, &l.shape) } else { None }) else { return count };
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    // KH2's re-hit interval: each target's bit off `interval` frames after its own hit.
    let bit = rd::<u32>(obj + 0x54);
    let now = super::takeover::TICKS.load(Ordering::Relaxed);
    for h in l.hits.iter_mut().filter(|h| h.0 != 0) {
        if now.saturating_sub(h.1) >= l.interval {
            wr(h.0 + 0x3E0, rd::<u32>(h.0 + 0x3E0) & !bit);
            *h = (0, 0);
        }
    }
    // The object's own place (KH1's spark, its knock-back direction, its clash).
    let c4 = [c[0], c[1], c[2], 1.0f32];
    wr(obj + 0x10, c4);
    wr(obj + 0x40, c4);
    // This frame's shape, then KH2's sweep back toward last frame's (none on the window's first).
    let r = l.shape.radius;
    let mut centres: [[f32; 3]; 33] = [[0.0; 3]; 33];
    let mut n = 1usize;
    centres[0] = c;
    if let Some((p, pf)) = l.prev {
        if frame.saturating_sub(pf) <= 1 && pf != frame && r > 0.0 {
            let d = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
            let mut len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if len > 0.0 {
                let step = [d[0] / len * r, d[1] / len * r, d[2] / len * r];
                let mut q = c;
                while r * 1.5 < len && n < centres.len() {
                    q = [q[0] + step[0], q[1] + step[1], q[2] + step[2]];
                    centres[n] = q;
                    n += 1;
                    len -= r;
                }
            }
        }
    }
    if l.prev.map_or(true, |(_, pf)| pf != frame) {
        l.prev = Some((c, frame));
    }
    let shape = l.shape;
    drop(g);
    let test: TestF = std::mem::transmute(BASE.load(Ordering::Relaxed) + TEST_STEAM);
    let team = (rd::<u32>(obj) >> 12) & 3;
    let mut count = count;
    for q in &centres[..n] {
        let e = list + count.max(0) as usize * 0x40;
        std::ptr::write_bytes(e as *mut u8, 0, 0x40);
        wr(e, team);
        wr(e + 8, owner as u64);
        wr(e + 0x10, bit);
        wr(e + 0x18, obj as u64);
        wr(e + 0x20, shape.kh1_type);
        wr(e + 0x24, shape.radius);
        wr(e + 0x28, shape.height);
        wr(e + 0x30, [q[0], q[1], q[2], 1.0f32]);
        test(e, list, count);
        if count < LIST_CAP {
            count += 1;
        }
    }
    count
}
