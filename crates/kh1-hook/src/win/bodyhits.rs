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
//!   bit is taken off each actor `interval` frames after its own hit, and KH1's own re-arm is left
//!   off (`+0x50` 0). (t-rehit2) That is `rehit`'s now, shared with the Keyblade windows; this
//!   module kept its own copy of it before.
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
/// KH1's attack constructor `(actor, hit bit, data) -> object` (Steam), which KH1's spell scripts
/// call for their shots (through syscall `0x1402B9870`): unlike the row maker `0x1402B3270` it
/// reads no field it doesn't write for a free-standing object (the row maker tests the bone at
/// `+0x20`, which the constructor leaves as the pool slot's last user had it for a free-standing
/// one, and would add the Keyblade's bonuses on a 0x30). First bytes checked (Checked (exe)).
const MAKE_STEAM: usize = 0x2B2DC0;
const MAKE_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];
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
static MAKE_OK: AtomicBool = AtomicBool::new(false);
static LOGGED: AtomicU32 = AtomicU32::new(0);

fn note(s: impl FnOnce() -> String) {
    if !super::log_on(super::LogCat::Combat) {
        return;
    }
    if super::log_on(super::LogCat::Combat) && LOGGED.fetch_add(1, Ordering::Relaxed) < 80 {
        log!(Combat, "{}", s());
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
    /// The Magnet catch (orbit radius from the ATKP's knock-back values, swirl), reaction 5.
    magnet: Option<(i16, i16, i16)>,
    /// Last game frame's centre (KH1 world) and that frame's number.
    prev: Option<([f32; 3], u64)>,
    /// Enemies its Magnet caught (actor, orbit radius, swirl).
    caught: [(usize, f32, f32); 8],
}

struct State {
    live: [Option<Live>; POOL_COUNT],
    /// A KH2 spell's attack on one of our objects (`spell_attacks`), by pool index.
    spells: Vec<(usize, SpellObj)>,
    /// Each enemy's last hit group of a KH2 spell and the hits in a row from it (KH2's hit record:
    /// the ATKP row by that count, `HitRecord::admit`).
    counts: Vec<(usize, i64, u32)>,
    /// Hits KH1's pass made with spell objects since `spell_attacks` last asked: (key, ATKP,
    /// actor, damage).
    results: Vec<(u64, u16, usize, Option<u32>)>,
    /// The row a spell hit is being made with now (`before_hit` → `atkp`): (attack, row).
    current: Option<(usize, kh2_formats::battle::Atkp)>,
    /// Our data copies: their key (`body_key`, `spell_key`) → address. Never freed: KH1's hit
    /// records keep an attack's data after the attack (`docs/kh1-attack-elements.md` §4).
    data: Vec<(u64, usize)>,
    page: usize,
    /// The move playing: (motion, KH1 time, request).
    playing: Option<(u16, f32, u32)>,
}

static STATE: Mutex<State> = Mutex::new(State { live: [None; POOL_COUNT], spells: Vec::new(), counts: Vec::new(), results: Vec::new(), current: None, data: Vec::new(), page: 0, playing: None });

/// A KH2 spell attack on a KH1 object of ours: its key in the spell runtime, ATKP rows by hits in
/// a row (0.., the last repeated), its hit group, its shapes this frame (KH1 world centre, KH1 type,
/// radius, half height) and the game frame they are from, and whether its data is a KH1 spell's
/// own (its stat source is then KH1's: max MP, not KH2's Magic again).
struct SpellObj {
    key: u64,
    atk: u16,
    rows: Vec<kh2_formats::battle::Atkp>,
    group: i64,
    shapes: [([f32; 3], u32, f32, f32); SPELL_SHAPES],
    n: usize,
    seen: u64,
    kh1_spell: bool,
}

/// Shapes a spell attack tests at most (`combat_core::MAX_SWEPT_SHAPES` plus its own).
const SPELL_SHAPES: usize = 33;
/// ATKP rows kept per spell attack (hits in a row; the last one repeats).
const SPELL_ROWS: u8 = 16;
/// KH1's clash type that meets no attack or guard but hits bodies (`kh1rows::PIERCE_TYPE`): spell
/// attacks met nothing on our own sweep before; kept so.
const NO_CLASH_TYPE: u32 = 6;

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
        log!(Combat, "KH2 body hits: KH1's entry test has unexpected bytes {have:02x?}: KH2's own sweep stays");
        return;
    }
    TEST_OK.store(true, Ordering::Relaxed);
    let have = std::slice::from_raw_parts((base + MAKE_STEAM) as *const u8, MAKE_BYTES.len());
    MAKE_OK.store(have == MAKE_BYTES, Ordering::Relaxed);
    if have != MAKE_BYTES {
        log!(Combat, "KH2 spell hits: KH1's attack constructor has unexpected bytes {have:02x?}: spells keep our own sweep");
    }
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
        log!(Combat, "KH2 body hits: through KH1's collision pass (KH1 attack objects with KH2's shapes)");
    }
    let Some(kh2) = pose::kh2() else { return };
    let Ok(mut g) = STATE.try_lock() else { return };
    // (Ours KH1 freed: forgotten.)
    for e in g.live.iter_mut() {
        if e.is_some_and(|l| !still_ours(&l)) {
            *e = None;
        }
    }
    let live_now: Vec<bool> = g.live.iter().map(|e| e.is_some()).collect();
    g.spells.retain(|(i, _)| live_now[*i]);
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
    let spell_slots: Vec<usize> = g.spells.iter().map(|(i, _)| *i).collect();
    for (i, e) in g.live.iter_mut().enumerate() {
        if spell_slots.contains(&i) {
            continue;
        }
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
                        note(|| format!("KH2 body hits: KH2 bone {b} (ATKP {}) has no KH1 bone: left out", w.attack));
                        continue;
                    }
                },
            };
            let shape = BodyShape { kh1_type, radius: s.radius, height: s.height, bone, offset: s.offset, lift: s.lift };
            let Some(data) = data_for(&mut g, base, m, w.index, air, si, w.channel, &a, s.radius) else { continue };
            let fx = template_fx(air, super::kh1rows::throws(&a));
            let bit = free_bit();
            if bit == 0 {
                note(|| format!("KH2 body hits: no free hit bit for ATKP {} (motion {m})", w.attack));
                continue;
            }
            // (KH1's maker clears his combat info's "connected" flag for a new swing: this isn't one.)
            let connected = if info > 0x10000 { rd::<u32>(info + 0x50) } else { 0 };
            let obj = new_attack(actor, bit, data);
            if info > 0x10000 {
                wr(info + 0x50, rd::<u32>(info + 0x50) | connected);
            }
            let Some(i) = pool_index(obj) else {
                note(|| format!("KH2 body hits: KH1's maker gave {obj:#x} for ATKP {} (pool full?)", w.attack));
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
            g.live[i] = Some(Live { obj, data, motion: m, atkp: w.attack, shape, magnet, prev: None, caught: [(0, 0.0, 0.0); 8] });
            made.push(format!("ATKP {} group {} on KH1 group {} ({:?} r {} h {} lift {}, interval {})", w.attack, w.group, w.channel, bone, s.radius, s.height, s.lift, a.interval));
        }
    }
    if !made.is_empty() {
        note(|| format!("KH2 body hits: motion {m} (request {req:#x}, {}): KH1 attack objects {}", if air { "air" } else { "ground" }, made.join("; ")));
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
    let key = (motion as u64) << 32 | (window as u64 & 0xFF) << 16 | (air as u64) << 8 | (shape as u64 & 0xFF);
    let d = data_slot(g, base, key)?;
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

/// Our data copy for `key` (made the first time; its address).
unsafe fn data_slot(g: &mut State, base: usize, key: u64) -> Option<usize> {
    if let Some(e) = g.data.iter().find(|e| e.0 == key) {
        return Some(e.1);
    }
    if g.page == 0 {
        g.page = super::kh1rows::near(base + BLOCK_RVA, DATA_PAGE).unwrap_or(usize::MAX);
        note(|| format!("KH2 body / spell hits: attack data copies at {:#x}", g.page));
    }
    if g.page == usize::MAX || (g.data.len() + 1) * DATA_SLOT > DATA_PAGE {
        note(|| "KH2 body / spell hits: no room for another attack data copy".to_string());
        return None;
    }
    let d = g.page + g.data.len() * DATA_SLOT;
    g.data.push((key, d));
    Some(d)
}

/// A KH2 spell's attacks this frame (`formmagic`, `reflect`, in place of `combat::Strikes::
/// spell_strike`): each a free-standing KH1 attack object of ours (made the first time its key is
/// seen, with KH1's attack constructor, as KH1's spell scripts make theirs), its shapes this frame for KH1's pass (`entries`), which collides it with
/// every body itself. Returns the hits KH1's pass made with them since the last call: (key, ATKP,
/// actor, damage). The objects: free-standing (data bone < 0: KH1's attribute `0x400`, as KH1's own
/// spell shots), active always (activity 3), no KH1 re-arm; the data a copy of the KH1 spell's own
/// entry (`kh1_data`: its element and stat source) or of Sora's ground swing row's (his finisher's
/// for a hit that knocks away, as `spell_strike` took them), the clash type that meets nothing (the
/// old sweep met no attack or guard either), the KH2 spark group in `+0x28` with the spell
/// package's effect list on the object (`kh2dpx::hit_list`, as `combat::land_hit` set it per hit).
pub fn spells_native() -> bool {
    native() && MAKE_OK.load(Ordering::Relaxed)
}

pub unsafe fn spell_attacks(sora: usize, attacks: &[super::combat::SpellAttack], row: &dyn Fn(u16, u8, u8) -> Option<kh2_formats::battle::Atkp>) -> Vec<(u64, u16, usize, Option<u32>)> {
    use combat_core::ShapeKind;
    let base = BASE.load(Ordering::Relaxed);
    let Ok(mut g) = STATE.try_lock() else { return Vec::new() };
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    let make: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + MAKE_STEAM);
    let free_bit: unsafe extern "system" fn() -> u32 = std::mem::transmute(base + FREE_BIT_RVA);
    for a in attacks {
        // (Ours KH1 freed meanwhile: made again.)
        if let Some(p) = g.spells.iter().position(|(i, sp)| sp.key == a.key && g.live[*i].is_some_and(|l| still_ours(&l))) {
            let sp = &mut g.spells[p].1;
            let mut n = 0;
            for s in a.shapes.iter() {
                let t = match s.kind {
                    ShapeKind::Ellipsoid | ShapeKind::Sphere => 1u32,
                    ShapeKind::Column => 2,
                    ShapeKind::Cube => continue,
                };
                if n < SPELL_SHAPES {
                    sp.shapes[n] = ([s.center[0], -s.center[1], s.center[2]], t, s.radius, s.height);
                    n += 1;
                }
            }
            sp.n = n;
            sp.seen = frame;
            continue;
        }
        g.spells.retain(|(_, sp)| sp.key != a.key);
        let rows: Vec<kh2_formats::battle::Atkp> = (0..SPELL_ROWS).map_while(|c| row(a.atk, a.sub, c)).collect();
        let Some(first) = rows.first().cloned() else { continue };
        let k1 = if first.knockback_1 == i16::MAX { 0 } else { first.knockback_1 };
        let knock = character_core::reaction::category(first.enemy_reaction) == 4 || k1 >= 100;
        // The spark: the spell package's effect list and KH2's group for the row.
        let spark = a.package.and_then(|p| {
            let group = super::combat::kh2_group(super::combat::Kh2Spark { pkg: super::kh2dpx::Pkg::Spell(p), effect_on_hit: first.effect_on_hit, random_effect: first.random_effect });
            let (list, _) = super::kh2dpx::hit_list(super::kh2dpx::Pkg::Spell(p), group)?;
            let h = pose::handle(list);
            (h != 0).then_some((h, group))
        });
        let src = match a.kh1_data {
            Some(off) => (base + BLOCK_RVA).wrapping_add(off as isize as usize),
            None => match super::kh1rows::kh1_row(template_row(false, knock)) {
                Some((d, _)) => d,
                None => continue,
            },
        };
        let key = 1u64 << 63 | (a.atk as u64) << 32 | (a.sub as u64) << 24 | ((spark.map_or(-1, |s| s.1) + 1) as u64 & 0xFF) << 16 | (a.kh1_data.map_or(0xFFFF, |o| (o as u64 >> 3) & 0xFFFF)) | (knock as u64) << 40;
        let Some(d) = data_slot(&mut g, base, key) else { continue };
        std::ptr::copy_nonoverlapping(src as *const u8, d as *mut u8, DATA);
        wr(d + 0x10, -1i32);
        let r0 = a.shapes.first().map_or(10.0, |s| s.radius);
        wr(d + 0x5C, r0.round() as i32);
        wr(d + 0x38, (rd::<u32>(d + 0x38) & !7) | NO_CLASH_TYPE | 0x1_0000);
        if let Some((_, group)) = spark {
            wr(d + 0x28, group);
        }
        let bit = free_bit();
        if bit == 0 {
            note(|| format!("KH2 spell hits: no free hit bit for ATKP {}", a.atk));
            continue;
        }
        let obj = make(sora, bit, d);
        let Some(i) = pool_index(obj) else {
            note(|| format!("KH2 spell hits: KH1's constructor gave {obj:#x} for ATKP {} (pool full?)", a.atk));
            continue;
        };
        // What the row maker adds for any data (`0x1402B3270`, Checked (decomp)), as the old sweep's
        // objects had it: Sora's effect source (`+8`, his combat info's `+0x18`) and his "connected"
        // word (`+0x60` → combat info `+0x50`), which a hit sets.
        let info = pose::ptr(rd::<u32>(sora + 0x14C));
        if info > 0x10000 {
            wr(obj + 8, pose::handle(rd::<u64>(info + 0x18) as usize));
            wr(obj + 0x60, (info + 0x50) as u64);
        }
        // Active from now (activity 3), its place written by us (position kind 0).
        wr(obj, (rd::<u32>(obj) & !0xF) | 0xC);
        if let Some((h, _)) = spark {
            wr(obj + 8, h);
        }
        let dummy = BodyShape { kh1_type: 1, radius: 0.0, height: 0.0, bone: None, offset: [0.0; 3], lift: 0.0 };
        g.live[i] = Some(Live { obj, data: d, motion: 0, atkp: a.atk, shape: dummy, magnet: None, prev: None, caught: [(0, 0.0, 0.0); 8] });
        let mut sp = SpellObj { key: a.key, atk: a.atk, rows, group: a.group, shapes: [([0.0; 3], 1, 0.0, 0.0); SPELL_SHAPES], n: 0, seen: frame, kh1_spell: a.kh1_data.is_some() };
        for s in a.shapes.iter() {
            let t = match s.kind {
                ShapeKind::Ellipsoid | ShapeKind::Sphere => 1u32,
                ShapeKind::Column => 2,
                ShapeKind::Cube => continue,
            };
            if sp.n < SPELL_SHAPES {
                sp.shapes[sp.n] = ([s.center[0], -s.center[1], s.center[2]], t, s.radius, s.height);
                sp.n += 1;
            }
        }
        g.spells.push((i, sp));
        note(|| format!("KH2 spell hits: ATKP {} sub {} on KH1 attack object {obj:#x} (data {d:#x}{}, {} shape(s), spark {:?})", a.atk, a.sub, if a.kh1_data.is_some() { ", KH1 spell's" } else { "" }, a.shapes.len(), spark.map(|s| s.1)));
    }
    std::mem::take(&mut g.results)
}

/// A spell attack ended (`formmagic`, `reflect`): its object freed.
pub unsafe fn spell_forget(key: u64) {
    let Ok(mut g) = STATE.try_lock() else { return };
    let Some(p) = g.spells.iter().position(|(_, sp)| sp.key == key) else { return };
    let (i, _) = g.spells.remove(p);
    if let Some(l) = g.live[i].take() {
        if still_ours(&l) {
            let free: unsafe extern "system" fn(usize) = std::mem::transmute(BASE.load(Ordering::Relaxed) + FREE_ATTACK_RVA);
            free(l.obj);
        }
    }
}

/// The window's ATKP for an attack object of ours (`kh1rows::kh2_atkp`).
pub unsafe fn atkp(attack: usize) -> Option<kh2_formats::battle::Atkp> {
    if !native() {
        return None;
    }
    let i = pool_index(attack)?;
    let g = STATE.try_lock().ok()?;
    let l = g.live[i].filter(|l| l.obj == attack && still_ours(l))?;
    // A spell's: the row by the hits in a row KH2's hit record counts (`before_hit`), its stat
    // source left to KH1's spell data where the data is a KH1 spell's (max MP, as KH1's spells).
    if let Some((_, sp)) = g.spells.iter().find(|(k, _)| *k == i) {
        let mut a = match &g.current {
            Some((at, a)) if *at == attack => a.clone(),
            _ => sp.rows.first()?.clone(),
        };
        if sp.kh1_spell {
            a.stat_source = 0xFF;
        }
        return Some(a);
    }
    let id = l.atkp;
    drop(g);
    pose::kh2()?.battle.attack_for_level(id, 0, 0).cloned()
}

/// `probe::hit`, before KH1's record builder runs for `attack` on `target`: for a spell object of
/// ours, KH2's hit record's count (hits in a row from its group; KH2's `HitRecord::admit`) picks
/// the ATKP row its damage and reactions take (`atkp`).
pub unsafe fn before_hit(attack: usize, target: usize) {
    if !native() || target < 0x10000 {
        return;
    }
    let Some(i) = pool_index(attack) else { return };
    let Ok(mut g) = STATE.try_lock() else { return };
    g.current = None;
    if !g.live[i].is_some_and(|l| l.obj == attack) {
        return;
    }
    let Some(group) = g.spells.iter().find(|(k, _)| *k == i).map(|(_, sp)| sp.group) else { return };
    let count = match g.counts.iter_mut().find(|c| c.0 == target) {
        Some(c) if c.1 == group => {
            c.2 += 1;
            c.2
        }
        Some(c) => {
            *c = (target, group, 0);
            0
        }
        None => {
            if g.counts.len() > 64 {
                g.counts.clear();
            }
            g.counts.push((target, group, 0));
            0
        }
    };
    let row = g.spells.iter().find(|(k, _)| *k == i).and_then(|(_, sp)| sp.rows.get((count as usize).min(sp.rows.len().saturating_sub(1))).cloned());
    g.current = row.map(|r| (attack, r));
}

/// `probe::hit`: KH1's contact made a hit record with `attack` on `target`: a spell's result and a
/// Magnet's catch. (Its re-hit interval: `rehit::on_hit`, called before this while `current`
/// still holds a spell's row.)
pub unsafe fn hit(attack: usize, target: usize, rec: usize) {
    if !native() || target < 0x10000 {
        return;
    }
    let Some(i) = pool_index(attack) else { return };
    let Ok(mut g) = STATE.try_lock() else { return };
    g.current = None;
    let spell = g.spells.iter().find(|(k, _)| *k == i).map(|(_, sp)| (sp.key, sp.atk));
    if let Some((key, atk)) = spell {
        let dealt = (rec > 0x10000).then(|| rd::<i32>(rec + 8).max(0) as u32);
        if g.results.len() < 256 {
            g.results.push((key, atk, target, dealt));
        }
    }
    let Some(l) = g.live[i].as_mut().filter(|l| l.obj == attack && still_ours(l)) else { return };
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
    note(|| format!("KH2 {} hit: ATKP {} (motion {}) from KH1's pass on actor {target:#x}", if spell.is_some() { "spell" } else { "body" }, l.atkp, l.motion));
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

/// KH1's `0x1402B3800 (object, entries, count) -> count`: ours get KH2's shapes (a body window's
/// with KH2's sweep, a spell attack's as its script gave them).
unsafe extern "system" fn entries(obj: usize, list: usize, count: i32) -> i32 {
    let f: EntriesF = std::mem::transmute(ORIG.load(Ordering::Acquire));
    let Some(i) = pool_index(obj) else { return f(obj, list, count) };
    let Ok(mut g) = STATE.try_lock() else { return f(obj, list, count) };
    if !g.live[i].is_some_and(|l| l.obj == obj && still_ours(&l)) {
        drop(g);
        return f(obj, list, count);
    }
    if !TEST_OK.load(Ordering::Relaxed) || list < 0x10000 {
        return count;
    }
    let owner = pose::ptr(rd::<u32>(obj + 0x34));
    if owner < 0x10000 {
        return count;
    }
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    let bit = rd::<u32>(obj + 0x54);
    // (KH2's re-hit interval: `rehit`, for every KH2 attack of Sora's.)
    // A spell's attack: the shapes its script gave this frame (KH2's own sweep already in them).
    if let Some((_, sp)) = g.spells.iter().find(|(k, _)| *k == i) {
        if sp.n == 0 || frame.saturating_sub(sp.seen) > 1 {
            return count;
        }
        let (shapes, n) = (sp.shapes, sp.n);
        drop(g);
        let c = shapes[0].0;
        let c4 = [c[0], c[1], c[2], 1.0f32];
        wr(obj + 0x10, c4);
        wr(obj + 0x40, c4);
        let mut count = count;
        for &(q, t, r, h) in &shapes[..n] {
            count = put_entry(obj, owner, bit, list, count, t, r, h, q);
        }
        return count;
    }
    let Some(l) = g.live[i].as_mut() else { return count };
    let Some(c) = centre(owner, &l.shape) else { return count };
    // The object's own place (KH1's spark, its knock-back direction, its clash).
    let c4 = [c[0], c[1], c[2], 1.0f32];
    wr(obj + 0x10, c4);
    wr(obj + 0x40, c4);
    // This frame's shape, then KH2's sweep back toward last frame's (none on the window's first).
    let r = l.shape.radius;
    let mut centres: [[f32; 3]; SPELL_SHAPES] = [[0.0; 3]; SPELL_SHAPES];
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
    let mut count = count;
    for q in &centres[..n] {
        count = put_entry(obj, owner, bit, list, count, shape.kh1_type, shape.radius, shape.height, *q);
    }
    count
}

/// One entry of KH1's list for our object (`0x1402B3D50`'s layout), tested at once against the
/// entries already in (KH1's `0x1402B3BE0`). Returns the new count.
#[allow(clippy::too_many_arguments)]
unsafe fn put_entry(obj: usize, owner: usize, bit: u32, list: usize, count: i32, kind: u32, radius: f32, height: f32, at: [f32; 3]) -> i32 {
    let test: TestF = std::mem::transmute(BASE.load(Ordering::Relaxed) + TEST_STEAM);
    let e = list + count.max(0) as usize * 0x40;
    std::ptr::write_bytes(e as *mut u8, 0, 0x40);
    wr(e, (rd::<u32>(obj) >> 12) & 3);
    wr(e + 8, owner as u64);
    wr(e + 0x10, bit);
    wr(e + 0x18, obj as u64);
    wr(e + 0x20, kind);
    wr(e + 0x24, radius);
    wr(e + 0x28, height);
    wr(e + 0x30, [at[0], at[1], at[2], 1.0f32]);
    test(e, list, count);
    if count < LIST_CAP {
        count + 1
    } else {
        count
    }
}
