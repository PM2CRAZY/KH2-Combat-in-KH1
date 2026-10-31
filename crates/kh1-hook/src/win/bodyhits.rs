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
//! The Magnet catch (ATKP reaction 5, Magnet Burst's 1900): an enemy KH1's pass hits with such an
//! object goes into KH1's movement state of ours for it (`magnet`, t862), which KH1's physics step
//! steps toward the object's place (`magnet_centre`) while its window is live.
//!
//! (t680) Valor's left Keyblade's hit windows (weapon 2) are its own KH1 attack object: module
//! `lefthits` (`docs/kh1-drive-forms-native.md` §7.2).
//!
//! Steam only (new addresses); elsewhere `kh2_hits` sweeps as before. Cost: one hook on the pass's
//! per-attack builder (a lookup in a 64-entry table per live attack object); a few entries per live
//! window; at most one data copy per (move, window, air) at a time.
//!
//! (t1046) **Crits only on a move's last blow** (the owner's answer 1 (b)). KH1 rolls its critical
//! from the attack's chance, which its constructor takes from the data (`+0x94` = data `+0x18` ×
//! 0.05 × stats `+0xD8`, `+0x98` = data `+0x1C`; `0x1402B2DC0`, the roll in `0x1402BFA80`, Checked
//! (decomp)), and only its finisher data has a chance (Checked (files) `btltbl.bin`). Before, every
//! KH2 blow window took the finisher row's data, so each hit of Explosion's 911 rolled. Now only the
//! clip's last blow window on his body (`kh1rows::last_blow`: Explosion's 912, Magnet Burst's 1903,
//! Final's 154's 1737), and only in a move playing on a finisher request (`kh1rows::finisher_request`,
//! the owner's answer (c)), takes the finisher row's; every other blow window takes the swing row's data with
//! the blow's kind 5 kept (`combat::scale_record` still gives KH2's arc; its spark and sound bytes
//! the blow's, as before). A re-hit of the last window on the same target rolls no more
//! (`rehit::again`). A KH2 spell's knocking hit on the finisher's data (no KH1 spell data of its own)
//! has its chance and bonus cleared: KH1's spells never roll (all nine of KH1's spell entries have
//! 0, Checked (files)) and which hit of a spell script is its last can't be told from one attack.
//!
//! (t1045) **Our data copies are reused** once nothing of KH1's uses them (`data_slot`, `reclaim`).
//! One page holds them (`DATA_PAGE` / `DATA_SLOT`: 64), shared since t1037 with `lefthits`' blade
//! and guard objects, so Final's ~40 attacks per blade filled it and later copies were refused
//! ("no room for another attack data copy"): Explosion, Vicinity Break, Magnet Burst and new Final
//! hits got no objects until a fresh start (the owner's report; fix "(a)"). Now, when a copy is
//! needed for a key not on the page and the page is full, the copies nothing uses are freed (their
//! keys dropped) and reused. In use, from KH1's own memory (Steam KH1, Checked (decomp)):
//! - every object of KH1's attack pool (`0x142DB4710`, 64 of 0xC0) with its in-use bit `+0`
//!   `0x800` (the constructor `0x1402B2DC0` takes the first without it; KH1's pool walks
//!   `0x1402B34B0` / `0x1402B3380` test it), its data handle `+0x5C` (`0x1402B2DC0` writes it;
//!   one KH1 marked dead, `0x200000`, keeps `0x800` until it is freed, so it counts);
//! - every hit record (KH1's record maker `0x1402BF630`: 64 records of 0x40 at `0x142DE5E10`, 128
//!   at `0x142DE6E10`, its spare one at `0x142DE5DD0`; in use `+0` bit 1, cleared for all by
//!   `0x1402BF560`) by the attack's data handle it keeps (`+0x2C`, the object's `+0x5C`), so a hit
//!   KH1 hasn't finished with keeps its data; the pool addresses checked in the exe once
//!   (`0x1402BF560`'s two `lea`s, Checked (exe)), else nothing is reused (as before);
//! - what `lefthits` holds for its live objects (every window's copy of a blade's play, which its
//!   step puts on the object later; `lefthits::held`), and a copy handed out this game frame or the
//!   last (one being made now: `lefthits::make` takes all its windows' copies before its object).
//! Before reusing, entries of ours whose object KH1 freed are forgotten (here and in `lefthits`),
//! so none takes a reused copy on a reused pool object for its own. **Assumption**: nothing else of
//! KH1's keeps an attack's data past its object and its hit records. Nothing runs per frame: the
//! scan (64 objects, 193 records) runs only when the page is full and a new key comes.

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
/// (t1045) KH1's hit records (Steam, `0x1402BF630`, Checked (decomp)): 64 and 128 of 0x40 bytes,
/// the spare one, in use `+0` bit 1, the attack's data handle `+0x2C`.
const RECORDS_A_STEAM: usize = 0x2DE5E10;
const RECORDS_B_STEAM: usize = 0x2DE6E10;
const RECORD_SPARE_STEAM: usize = 0x2DE5DD0;
const RECORDS_A_COUNT: usize = 64;
const RECORDS_B_COUNT: usize = 128;
const RECORD: usize = 0x40;
/// KH1's record reset `0x1402BF560`'s two `lea rax, [records]; mov ecx, count` (Steam, Checked
/// (exe)): at `+0xA` the first pool's (64), at `+0x2E` the second's (128).
const RECORDS_LEA_A: (usize, [u8; 12]) = (0x2BF56A, [0x48, 0x8D, 0x05, 0x9F, 0x68, 0xB2, 0x02, 0xB9, 0x40, 0x00, 0x00, 0x00]);
const RECORDS_LEA_B: (usize, [u8; 12]) = (0x2BF58E, [0x48, 0x8D, 0x05, 0x7B, 0x78, 0xB2, 0x02, 0xB9, 0x80, 0x00, 0x00, 0x00]);
/// A freed copy's key (no key of ours has every bit).
const FREE_KEY: u64 = u64::MAX;
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
/// (t1045) KH1's hit record pools are where we read them (0 unknown, 1 yes, 2 no), and the
/// reclaim / "no room" lines logged (their own count: they come late in a session).
static RECORDS_OK: AtomicU32 = AtomicU32::new(0);
static PAGE_LOGGED: AtomicU32 = AtomicU32::new(0);

fn note(s: impl FnOnce() -> String) {
    if !super::log_on(super::LogCat::Combat) {
        return;
    }
    if super::log_on(super::LogCat::Combat) && LOGGED.fetch_add(1, Ordering::Relaxed) < 80 {
        log!(Combat, "{}", s());
    }
}

pub(super) unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
pub(super) unsafe fn wr<T: Copy>(p: usize, v: T) {
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
    /// (t1053) The clip's last blow on his body in a finisher (`data_for`'s `finisher`): a KH2 blow
    /// from it stays KH1's knock-back or launch; any other blow goes to KH1's flinch
    /// (`combat::scale_record`).
    last: bool,
}

struct State {
    live: [Option<Live>; POOL_COUNT],
    /// A KH2 spell's attack on one of our objects (`spell_place`), by pool index.
    spells: Vec<(usize, SpellObj)>,
    /// Each enemy's last hit group of a KH2 spell and the hits in a row from it (KH2's hit record:
    /// the ATKP row by that count, `HitRecord::admit`).
    counts: Vec<(usize, i64, u32)>,
    /// Hits KH1's pass made with spell objects since `spell_results` last asked: (key, ATKP,
    /// actor, damage).
    results: Vec<(u64, u16, usize, Option<u32>)>,
    /// The row a spell hit is being made with now (`before_hit` → `atkp`): (attack, row).
    current: Option<(usize, kh2_formats::battle::Atkp)>,
    /// Our data copies on the page: their key (`body_key`, `spell_key`, `lefthits`') → address.
    /// (t1045) Freed (`FREE_KEY`) and reused only when no KH1 attack object, hit record (which
    /// keeps an attack's data after the attack) or live object of `lefthits` uses them (`reclaim`).
    data: Vec<DataCopy>,
    page: usize,
    /// The move playing: (motion, KH1 time, request).
    playing: Option<(u16, f32, u32)>,
}

/// (t1045) One of our data copies on the page: its key (`FREE_KEY`: free), its address, the game
/// frame it was last handed out.
struct DataCopy {
    key: u64,
    addr: usize,
    used: u64,
}

/// Reflect's barriers as KH1 guard objects (`combat::Strikes::barrier_guard`, t796): each one's
/// shapes this frame (KH1 world centre, KH1 type, radius, half height), its shape count and the
/// game frame they are from. KH1's entry builder (`entries`) puts them in as KH2's own ellipsoids,
/// so KH1's pass meets enemy attacks with the barrier's real shape; a barrier not given shapes this
/// frame or the last puts in none (it meets nothing).
struct GuardShapes {
    obj: usize,
    shapes: [([f32; 3], u32, f32, f32); SPELL_SHAPES],
    n: usize,
    seen: u64,
}

static GUARDS: Mutex<Vec<GuardShapes>> = Mutex::new(Vec::new());

/// KH2's guard windows on his body as KH1 guard objects placed by KH1's own pass (t909; the owner:
/// "do all 3"): each object, the KH2 body shape it stands on (placed from KH1's state each pass, as
/// the body hits' are: his root, facing and KH1's bone, `centre`) and last frame's centre for KH2's
/// sweep. `combat::Strikes::guard_step` makes and frees them with the window; nothing of ours poses
/// KH2's rig for them.
/// (t911) Also KH2's Keyblade guard shapes on KH1's grip (`grip_guard_keep`): `true` holds the
/// object's result word at "clashed" each pass, as the Keyblade guards of clash type 3 need (t861).
static BODY_GUARDS: Mutex<Vec<(usize, BodyShape, Option<([f32; 3], u64)>, bool)>> = Mutex::new(Vec::new());

/// KH1 Sora's grip bone (the Keyblade's, `kh1_data::retarget::GRIP_BONE`).
const GRIP_BONE: u32 = 280;

/// One of KH2's Keyblade guard shapes on KH1's grip (Guard Shape KH2, t911): its KH1 shape type,
/// size and its centre in the grip's own axes (KH1's drawn matrix of bone 280), measured once from
/// KH2's shape on KH2's posed rig (`grip_local`) and carried by KH1's grip after that.
#[derive(Clone, Copy, Debug)]
pub struct GripShape {
    pub kh1_type: u32,
    pub radius: f32,
    pub height: f32,
    pub offset: [f32; 3],
}

/// KH1's bone `b` of `owner` now: its place (KH1's bone point, `0x14029EB20`'s raw `0x4000 | b`) and
/// the axes of its drawn matrix (`+0x15C` → `+0x1B4`, row vectors), KH1 world.
unsafe fn kh1_bone_frame(owner: usize, b: u32) -> Option<([f32; 3], [[f32; 3]; 3])> {
    let base = BASE.load(Ordering::Relaxed);
    let point: unsafe extern "system" fn(usize, u32, f32, *mut [f32; 4]) = std::mem::transmute(base + BONE_POINT_RVA);
    let mut out = [0.0f32; 4];
    point(owner, 0x4000 | b, 1.0, &mut out);
    let inst = pose::ptr(rd::<u32>(owner + 0x15C));
    let mats = if inst > 0x10000 { pose::ptr(rd::<u32>(inst + 0x1B4)) } else { 0 };
    if mats < 0x10000 {
        return None;
    }
    let m = rd::<[f32; 16]>(mats + b as usize * 0x40);
    let axes = [[m[0], m[1], m[2]], [m[4], m[5], m[6]], [m[8], m[9], m[10]]];
    let p = [out[0], out[1], out[2]];
    (p.iter().all(|v| v.is_finite()) && axes.iter().flatten().all(|v| v.is_finite())).then_some((p, axes))
}

/// KH2's Keyblade shape `s` (KH2 world, y up, on KH2's rig posed at KH1's time) as a shape on KH1's
/// grip now (`GripShape`): its centre in the grip's axes. `None`: a KH2 cube (never met in KH2), or
/// no grip matrix. **Assumption**: the KH2 shape keeps its place on the blade, so the grip carries
/// it as the blade turns and moves (KH2's Keyblade has no bone of its own moving under the grip).
pub unsafe fn grip_local(owner: usize, s: &combat_core::Shape) -> Option<GripShape> {
    use combat_core::ShapeKind;
    let kh1_type = match s.kind {
        ShapeKind::Ellipsoid | ShapeKind::Sphere => 1u32,
        ShapeKind::Column => 2,
        ShapeKind::Cube => return None,
    };
    let (p, axes) = kh1_bone_frame(owner, GRIP_BONE)?;
    let d = [s.center[0] - p[0], -s.center[1] - p[1], s.center[2] - p[2]];
    let mut offset = [0.0f32; 3];
    for (k, a) in axes.iter().enumerate() {
        let l2 = a[0] * a[0] + a[1] * a[1] + a[2] * a[2];
        if !(l2 > 1e-8) {
            return None;
        }
        offset[k] = (d[0] * a[0] + d[1] * a[1] + d[2] * a[2]) / l2;
    }
    Some(GripShape { kh1_type, radius: s.radius, height: s.height, offset })
}

/// The Keyblade guard object `obj` stands on `g` on KH1's grip (`grip_local`), placed by KH1's own
/// pass from the grip each pass (`body_guard_entries`), its result word held at "clashed".
pub fn grip_guard_keep(obj: usize, g: &GripShape) -> bool {
    if !ON.load(Ordering::Relaxed) || !TEST_OK.load(Ordering::Relaxed) {
        return false;
    }
    let shape = BodyShape { kh1_type: g.kh1_type, radius: g.radius, height: g.height, bone: Some(GRIP_BONE), offset: g.offset, lift: 0.0 };
    let Ok(mut l) = BODY_GUARDS.try_lock() else { return false };
    match l.iter_mut().find(|x| x.0 == obj) {
        Some(x) => {
            x.1 = shape;
            x.3 = true;
        }
        None => l.push((obj, shape, None, true)),
    }
    true
}

/// The guard object `obj` stands on KH2's body shape `s` (`BODY_GUARDS`): false where the shape can't
/// be placed by KH1's state (a KH2 cube, never met in KH2; a KH2 bone with no KH1 bone of the same
/// frame), for the caller's own placing.
pub fn body_guard_keep(obj: usize, s: &kh2_data::sora::BoneShape) -> bool {
    let kh1_type = match s.shape {
        0 | 3 => 1u32,
        1 => 2,
        _ => return false,
    };
    let bone = match s.bone {
        None => None,
        Some(b) => match BONES.iter().find(|x| x.0 == b) {
            Some(&(_, k)) => Some(k),
            None => return false,
        },
    };
    if !ON.load(Ordering::Relaxed) || !TEST_OK.load(Ordering::Relaxed) {
        return false;
    }
    let shape = BodyShape { kh1_type, radius: s.radius, height: s.height, bone, offset: s.offset, lift: s.lift };
    let Ok(mut g) = BODY_GUARDS.try_lock() else { return false };
    match g.iter_mut().find(|x| x.0 == obj) {
        Some(x) => x.1 = shape,
        None => g.push((obj, shape, None, false)),
    }
    true
}

/// KH2's body shape `s` can be placed from KH1's state (`body_guard_keep`): not a KH2 cube, on his
/// root or on a KH2 bone with a KH1 bone of the same frame (`BONES`).
pub fn body_shape_ok(s: &kh2_data::sora::BoneShape) -> bool {
    s.shape != 2 && s.bone.map_or(true, |b| BONES.iter().any(|x| x.0 == b))
}

/// A body guard object freed (`combat::Strikes::guard_step`).
pub fn body_guard_forget(obj: usize) {
    if let Ok(mut g) = BODY_GUARDS.try_lock() {
        g.retain(|x| x.0 != obj);
    }
}

/// `entries` for a body guard object (`BODY_GUARDS`): `Some(new count)` when `obj` is one. Its shape
/// from KH1's state now and KH2's sweep back to last frame's (as the body hits'). A pure guard of
/// clash type 4: KH1's pass never meets it with a body.
unsafe fn body_guard_entries(obj: usize, list: usize, count: i32) -> Option<i32> {
    let mut g = BODY_GUARDS.try_lock().ok()?;
    let e = g.iter_mut().find(|x| x.0 == obj)?;
    if !TEST_OK.load(Ordering::Relaxed) || list < 0x10000 {
        return Some(count);
    }
    let owner = pose::ptr(rd::<u32>(obj + 0x34));
    if owner < 0x10000 {
        return Some(count);
    }
    let Some(c) = centre(owner, &e.1) else { return Some(count) };
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    let shape = e.1;
    let hold = e.3;
    let r = shape.radius;
    let mut centres: [[f32; 3]; SPELL_SHAPES] = [[0.0; 3]; SPELL_SHAPES];
    let mut n = 1usize;
    centres[0] = c;
    if let Some((p, pf)) = e.2 {
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
    if e.2.map_or(true, |(_, pf)| pf != frame) {
        e.2 = Some((c, frame));
    }
    drop(g);
    let c4 = [c[0], c[1], c[2], 1.0f32];
    wr(obj + 0x10, c4);
    wr(obj + 0x40, c4);
    if hold {
        // (A Keyblade guard of clash type 3: its result word held at "clashed" so it never strikes
        // a body, as `guard_entries` holds the Keyblade's and the barrier's, t813 / t861.)
        wr(obj + 4, 0x10u32);
        wr(obj, rd::<u32>(obj) & !0x0010_0080);
    }
    let bit = rd::<u32>(obj + 0x54);
    let mut count = count;
    for q in &centres[..n] {
        count = put_entry(obj, owner, bit, list, count, shape.kh1_type, shape.radius, shape.height, *q);
    }
    Some(count)
}

/// A barrier's guard object `obj` stands on `shapes` (KH2 y-up) this frame.
pub fn guard_shapes(obj: usize, shapes: &[combat_core::Shape]) {
    use combat_core::ShapeKind;
    let Ok(mut g) = GUARDS.try_lock() else { return };
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    let mut gs = GuardShapes { obj, shapes: [([0.0; 3], 1, 0.0, 0.0); SPELL_SHAPES], n: 0, seen: frame };
    for s in shapes {
        let t = match s.kind {
            ShapeKind::Ellipsoid | ShapeKind::Sphere => 1u32,
            ShapeKind::Column => 2,
            ShapeKind::Cube => continue,
        };
        if gs.n < SPELL_SHAPES {
            gs.shapes[gs.n] = ([s.center[0], -s.center[1], s.center[2]], t, s.radius, s.height);
            gs.n += 1;
        }
    }
    match g.iter_mut().find(|x| x.obj == obj) {
        Some(x) => *x = gs,
        None => g.push(gs),
    }
}

/// A barrier's guard object freed (`combat::Strikes::barrier_guards_keep`).
pub fn guard_forget(obj: usize) {
    if let Ok(mut g) = GUARDS.try_lock() {
        g.retain(|x| x.obj != obj);
    }
}

/// `entries` for a barrier's guard object: `Some(new count)` when `obj` is one.
unsafe fn guard_entries(obj: usize, list: usize, count: i32) -> Option<i32> {
    let g = GUARDS.try_lock().ok()?;
    let gs = g.iter().find(|x| x.obj == obj)?;
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    let (shapes, n, seen) = (gs.shapes, gs.n, gs.seen);
    drop(g);
    if n == 0 || frame.saturating_sub(seen) > 1 || !TEST_OK.load(Ordering::Relaxed) || list < 0x10000 {
        return Some(count);
    }
    let owner = pose::ptr(rd::<u32>(obj + 0x34));
    if owner < 0x10000 {
        return Some(count);
    }
    let bit = rd::<u32>(obj + 0x54);
    let c = shapes[0].0;
    let c4 = [c[0], c[1], c[2], 1.0f32];
    wr(obj + 0x10, c4);
    wr(obj + 0x40, c4);
    // (t813) Its result word held at "clashed" (0x10): KH1's attack pass clears it just before
    // building the entries (`0x1402B47A0`), and KH1's entry test skips a body against an attack
    // whose result has 0x30 (`0x1402B3BE0`), so the barrier's clash type 3 never strikes a body.
    // Attack-against-attack it still meets (that path reads no result without the one-hit bit
    // 0x80), and KH1's half for it (`0x1402B4BC0`) does nothing to an attack already "clashed":
    // the barrier stands. A half that marks it spent (0x100000) is undone here each pass.
    wr(obj + 4, 0x10u32);
    wr(obj, rd::<u32>(obj) & !0x0010_0080);
    let first = list + count.max(0) as usize * 0x40;
    let mut count = count;
    let start = count;
    for &(q, t, r, h) in &shapes[..n] {
        count = put_entry(obj, owner, bit, list, count, t, r, h, q);
    }
    // (t814) The enemy attacks KH1's pairing table lets nothing meet (types 6 and 7): met here
    // against the barrier's entries with KH1's own overlap test and KH1's meeting.
    let mine = (count - start).max(0) as usize;
    if mine > 0 {
        unmet_types(owner, first, mine);
    }
    Some(count)
}

/// KH1's overlap of two entries' shapes (Steam `0x1402B5EF0 (shape, shape) -> bool`, the test
/// `0x1402B3BE0` calls on `entry + 0x20`; first bytes checked, Checked (exe)).
const OVERLAP_STEAM: usize = 0x2B5EF0;
const OVERLAP_BYTES: [u8; 8] = [0x48, 0x89, 0x5C, 0x24, 0x18, 0x55, 0x56, 0x57];
/// KH1's attack meeting (Steam `0x1402B4A20`; `recoil`'s hook on it runs the barrier's rule first).
const MEET_STEAM: usize = 0x2B4A20;

/// (t814) The tester's owner: "let it meet any guard types it can't". KH1's pairing table
/// (`0x140528CB8`, Checked (exe)) has rows of 0 for clash types 6 and 7, so KH1's test
/// (`0x1402B3BE0`) never meets an attack of those types with any other: KH1's own guards don't stop
/// them either. KH2's barrier meets every enemy attack (`0x1403D2140`, Checked (decomp) KH2). So
/// each live enemy attack of type 6 or 7 in KH1's pool is tested against the barrier's entries this
/// pass with KH1's own overlap test, and one that touches goes to KH1's meeting (`0x1402B4A20`),
/// where the barrier's rule takes it (`combat::barrier_meeting`: a shot stopped, melee to KH1's
/// halves, counted, the contact effect), once per barrier and attack.
unsafe fn unmet_types(sora: usize, entries: usize, n: usize) {
    let base = BASE.load(Ordering::Relaxed);
    if std::slice::from_raw_parts((base + OVERLAP_STEAM) as *const u8, OVERLAP_BYTES.len()) != OVERLAP_BYTES {
        return;
    }
    let overlap: unsafe extern "system" fn(usize, usize) -> u8 = std::mem::transmute(base + OVERLAP_STEAM);
    let meet: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + MEET_STEAM);
    let team = |a: usize| (rd::<u32>(a + 0x374) >> 5) & 3;
    let ours = team(sora);
    for k in 0..POOL_COUNT {
        let g = base + POOL_STEAM + k * OBJ;
        let flags = rd::<u32>(g);
        // In use, active, not freed or turned back, of a type KH1 meets with nothing.
        if flags & 0x800 == 0 || flags & 0xC == 0 || flags & (0x0020_0000 | 0x0100_0000) != 0 || (flags >> 17) & 7 < 6 {
            continue;
        }
        let owner = pose::ptr(rd::<u32>(g + 0x34));
        if owner < 0x10000 || owner == sora || team(owner) == ours {
            continue;
        }
        let r = rd::<f32>(g + 0xC);
        let c = rd::<[f32; 3]>(g + 0x10);
        if !(r > 0.0 && r < 500.0) || !c.iter().all(|v| v.is_finite()) {
            continue;
        }
        // Its entry as KH1's builder lays one out (a sphere at its place).
        let mut e = [0u8; 0x40];
        let p = e.as_mut_ptr() as usize;
        wr(p, (flags >> 12) & 3);
        wr(p + 8, owner as u64);
        wr(p + 0x10, rd::<u32>(g + 0x54));
        wr(p + 0x18, g as u64);
        wr(p + 0x20, 1u32);
        wr(p + 0x24, r);
        wr(p + 0x28, r);
        wr(p + 0x30, [c[0], c[1], c[2], 1.0f32]);
        for i in 0..n {
            let mine = entries + i * 0x40;
            if overlap(mine + 0x20, p + 0x20) != 0 {
                meet(mine, p);
                note(|| format!("Reflect barrier: enemy attack {g:#x} of clash type {} (one KH1's table meets with nothing) met it", (flags >> 17) & 7));
                break;
            }
        }
    }
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
    /// Made and moved by its script's own calls (`spell_place`): kept until freed, its sweep only
    /// on the frame after a move.
    by_calls: bool,
    /// It has hit something (`spell_hit`).
    hit: bool,
    /// (t1056) Its KH2 blows stay blows (`combat::SpellAttack::send_off`, `last_blow`).
    send_off: bool,
}

/// Frames a spell attack placed by its script's calls stays live without being moved (5 s at 60).
const BY_CALLS_MAX: u64 = 300;
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

pub(super) fn pool_index(obj: usize) -> Option<usize> {
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
    // (t1050) Body windows KH1's pass stopped building: their log summed up.
    window_flush_due();
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
    let playing = if super::shared::active() { super::kh2slots::now_playing(actor) } else { None };
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
                super::rehit::forget(l.obj);
            }
        }
    }
    let Some(clip) = super::pose::kh2_moves().and_then(|c| c.moveset.clips.get(&m)) else { return };
    let air = matches!(rd::<i32>(actor + 0x70), 1 | 2);
    // (t680) Valor's left Keyblade's hit windows: a KH1 attack object of their own (`lefthits`,
    // which takes our data copies through `data_copy`, so without this lock).
    drop(g);
    super::lefthits::new_play(actor, m, req, air, clip);
    let Ok(mut g) = STATE.try_lock() else { return };
    let windows = super::kh2slots::body_windows(clip);
    if windows.is_empty() {
        return;
    }
    // (t1037) The shapes of the model playing: a Drive Form's own on KH1's own states (Final's 154:
    // group 2 a sphere of 150 on bone 67, group 19 an ellipsoid 120 up, 350 across, 180 high,
    // `P_EX100_ULTF.mdlx`, Checked (files)), else base Sora's.
    let rig = super::form::native_form().and_then(|k| k.data(kh2)).map_or(&kh2.rig, |f| &f.rig);
    let new_attack: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + NEW_ATTACK_RVA);
    let free_bit: unsafe extern "system" fn() -> u32 = std::mem::transmute(base + FREE_BIT_RVA);
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    let mut made = Vec::new();
    // (t1046) The clip's last blow on his body, in a finisher (`kh1rows::finisher_request`): the
    // one window on the finisher's data.
    let last = if super::kh1rows::finisher_request(req) { super::kh1rows::last_blow(clip, super::kh1rows::Carrier::Body) } else { None };
    for w in &windows {
        let Some(a) = kh2.battle.attack_for_level(w.attack, 0, 0).cloned() else { continue };
        let shapes: Vec<&kh2_data::sora::BoneShape> = rig.body_shapes.iter().filter(|s| s.group == w.group && (s.kind == ATTACK_KIND || s.kind == GUARD_KIND)).collect();
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
            let finisher = super::kh1rows::throws(&a) && last == Some(w.start);
            let Some(data) = data_for(&mut g, base, m, w.index, air, si, w.channel, &a, finisher, s.radius, BODY_BONE) else { continue };
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
            g.live[i] = Some(Live { obj, data, motion: m, atkp: w.attack, shape, magnet, prev: None, last: finisher });
            made.push(format!(
                "ATKP {} group {} on KH1 group {} ({:?} r {} h {} lift {}, interval {}{})",
                w.attack,
                w.group,
                w.channel,
                bone,
                s.radius,
                s.height,
                s.lift,
                a.interval,
                if finisher { ", the last blow: KH1's finisher data" } else { "" }
            ));
        }
    }
    if !made.is_empty() {
        note(|| format!("KH2 body hits: motion {m} (request {req:#x}, {}): KH1 attack objects {}", if air { "air" } else { "ground" }, made.join("; ")));
    }
}

/// KH1's row of Sora's that KH2's hits on his body are landed with (as `combat::Strikes::strike_as`
/// takes it: swing, or finisher for a KH2 blow; ground or air by his KH1 state).
pub(super) fn template_row(air: bool, throws: bool) -> usize {
    use super::kh1rows::Kind;
    match (air, throws) {
        (false, false) => Kind::Swing,
        (false, true) => Kind::Finisher,
        (true, false) => Kind::AirSwing,
        (true, true) => Kind::AirFinisher,
    }
    .template_row()
}

pub(super) unsafe fn template_fx(air: bool, throws: bool) -> Option<[u8; 3]> {
    super::kh1rows::kh1_row(template_row(air, throws)).map(|r| r.1)
}

/// Our attack data for one body shape of a window (made the first time, refreshed each time: the
/// Recoil row's clash rules can change). (t1046) `finisher`: the clip's last blow on his body
/// (`kh1rows::last_blow`), on KH1's finisher row's data (its critical chance and bonus); any other
/// window on the swing row's, a blow keeping kind 5.
#[allow(clippy::too_many_arguments)]
unsafe fn data_for(g: &mut State, base: usize, motion: u16, window: usize, air: bool, shape: usize, channel: u8, a: &kh2_formats::battle::Atkp, finisher: bool, radius: f32, bone: u32) -> Option<usize> {
    let key = (motion as u64) << 32 | (window as u64 & 0xFF) << 16 | (air as u64) << 8 | (shape as u64 & 0xFF);
    let d = data_slot(g, base, key)?;
    let throws = super::kh1rows::throws(a);
    let (src, _) = super::kh1rows::kh1_row(template_row(air, throws && finisher))?;
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
    wr(d + 0x10, bone);
    wr(d + 0x34, (rd::<u32>(d + 0x34) & !3) | channel as u32);
    wr(d + 0x54, 0.0f32);
    wr(d + 0x58, 0.0f32);
    wr(d + 0x5C, radius.round() as i32);
    // A KH2 Magic-based window (Explosion's, Magnet Burst's): KH1's own magic stat on the data
    // (`kh1rows::kh1_magic`, Ripple Drive's), not the swing's Strength.
    if a.stat_source == 1 {
        super::kh1rows::kh1_magic_row(d);
    }
    let (t1, p1) = (rd::<u32>(d + 0x38) & 7, rd::<i32>(d + 0x3C));
    let (t, p) = if super::recoil::kh2() { super::kh1rows::kh2_clash(Some(a), t1, p1) } else { (t1, p1) };
    // (Bit 16: "spent", no wall test.)
    wr(d + 0x38, (rd::<u32>(d + 0x38) & !7) | t | 0x1_0000);
    wr(d + 0x3C, p);
    Some(d)
}

/// (`lefthits`) Our attack data copy for `key` (`data_slot`, made the first time; its address).
pub(super) unsafe fn data_copy(key: u64) -> Option<usize> {
    let base = BASE.load(Ordering::Relaxed);
    let mut g = STATE.try_lock().ok()?;
    data_slot(&mut g, base, key)
}

/// (`lefthits`) KH1's game base as this module read it, and whether KH1's attack constructor is
/// the one we know (its first bytes).
pub(super) fn base() -> usize {
    BASE.load(Ordering::Relaxed)
}
pub(super) fn make_ok() -> bool {
    MAKE_OK.load(Ordering::Relaxed)
}

/// Our data copy for `key` (made the first time; its address). (t1045) A new key takes a freed
/// copy, else a new one while the page has room, else the copies nothing uses are freed first
/// (`reclaim`).
unsafe fn data_slot(g: &mut State, base: usize, key: u64) -> Option<usize> {
    let now = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    if let Some(e) = g.data.iter_mut().find(|e| e.key == key) {
        e.used = now;
        return Some(e.addr);
    }
    if g.page == 0 {
        g.page = super::kh1rows::near(base + BLOCK_RVA, DATA_PAGE).unwrap_or(usize::MAX);
        note(|| format!("KH2 body / spell hits: attack data copies at {:#x}", g.page));
    }
    if g.page == usize::MAX {
        note(|| "KH2 body / spell hits: no room for another attack data copy".to_string());
        return None;
    }
    if let Some(d) = take_free(g, key, now) {
        return Some(d);
    }
    if (g.data.len() + 1) * DATA_SLOT <= DATA_PAGE {
        let d = g.page + g.data.len() * DATA_SLOT;
        g.data.push(DataCopy { key, addr: d, used: now });
        return Some(d);
    }
    if reclaim(g, base, now) > 0 {
        if let Some(d) = take_free(g, key, now) {
            return Some(d);
        }
    }
    if super::log_on(super::LogCat::Combat) && PAGE_LOGGED.fetch_add(1, Ordering::Relaxed) < 40 {
        log!(Combat, "KH2 body / spell hits: no room for another attack data copy (all {} in use by KH1's attack objects or hit records)", g.data.len());
    }
    None
}

/// (t1045) A freed copy for `key`, if there is one.
fn take_free(g: &mut State, key: u64, now: u64) -> Option<usize> {
    let e = g.data.iter_mut().find(|e| e.key == FREE_KEY)?;
    e.key = key;
    e.used = now;
    Some(e.addr)
}

/// (t1045) KH1's hit record pools are the ones we read (`RECORDS_LEA_A` / `_B`, once).
unsafe fn records_ok(base: usize) -> bool {
    match RECORDS_OK.load(Ordering::Relaxed) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    let same = |(rva, bytes): (usize, [u8; 12])| (0..bytes.len()).all(|i| rd::<u8>(base + rva + i) == bytes[i]);
    let ok = same(RECORDS_LEA_A) && same(RECORDS_LEA_B);
    RECORDS_OK.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
    if !ok {
        log!(Combat, "KH2 body / spell hits: KH1's hit records aren't where we read them: our attack data copies aren't reused");
    }
    ok
}

/// (t1045) The page is full and a new key came: every copy no KH1 attack object (in use, `+0`
/// `0x800`: its data `+0x5C`), no KH1 hit record in use (`+0` bit 1: its data `+0x2C`, and the
/// spare record's) and no live object of `lefthits` (`lefthits::held`) uses, and not handed out
/// this game frame or the last, is freed (its key dropped). Ours KH1 freed are forgotten first (as
/// `frame` does), so none of them takes a reused copy on a reused pool object for its own. How
/// many were freed.
unsafe fn reclaim(g: &mut State, base: usize, now: u64) -> usize {
    if !records_ok(base) {
        return 0;
    }
    for e in g.live.iter_mut() {
        if e.is_some_and(|l| !still_ours(&l)) {
            *e = None;
        }
    }
    let live_now: Vec<bool> = g.live.iter().map(|e| e.is_some()).collect();
    g.spells.retain(|(i, _)| live_now[*i]);
    let mut held: Vec<usize> = Vec::with_capacity(POOL_COUNT + 16);
    let pool = base + POOL_STEAM;
    for i in 0..POOL_COUNT {
        let o = pool + i * OBJ;
        if rd::<u32>(o) & 0x800 != 0 {
            held.push(pose::ptr(rd::<u32>(o + 0x5C)));
        }
    }
    for (start, n) in [(RECORDS_A_STEAM, RECORDS_A_COUNT), (RECORDS_B_STEAM, RECORDS_B_COUNT)] {
        for i in 0..n {
            let r = base + start + i * RECORD;
            if rd::<u32>(r) & 1 != 0 {
                held.push(pose::ptr(rd::<u32>(r + 0x2C)));
            }
        }
    }
    held.push(pose::ptr(rd::<u32>(base + RECORD_SPARE_STEAM + 0x2C)));
    super::lefthits::held(&mut held);
    let mut n = 0;
    for e in g.data.iter_mut() {
        if e.key != FREE_KEY && e.used + 1 < now && !held.contains(&e.addr) {
            e.key = FREE_KEY;
            n += 1;
        }
    }
    if super::log_on(super::LogCat::Combat) && PAGE_LOGGED.fetch_add(1, Ordering::Relaxed) < 40 {
        log!(Combat, "KH2 body / spell hits: attack data page full ({} copies): {n} no KH1 attack object or hit record uses freed for reuse", g.data.len());
    }
    n
}

/// A KH2 spell's attacks (`formmagic`, `reflect`, in place of `combat::Strikes::spell_strike`):
/// each a free-standing KH1 attack object of ours (made the first time its key is seen, with KH1's
/// attack constructor, as KH1's spell scripts make theirs; since t819 by the script's own calls,
/// `spell_place`), its shapes for KH1's pass (`entries`), which collides it with every body
/// itself; the hits it made: `spell_results`. The objects: free-standing (data bone < 0: KH1's attribute `0x400`, as KH1's own
/// spell shots), active always (activity 3), no KH1 re-arm; the data a copy of the KH1 spell's own
/// entry (`kh1_data`: its element and stat source) or of Sora's ground swing row's (his finisher's
/// for a hit that knocks away, as `spell_strike` took them), the clash type that meets nothing (the
/// old sweep met no attack or guard either), the KH2 spark group in `+0x28` with the spell
/// package's effect list on the object (`kh2dpx::hit_list`, as `combat::land_hit` set it per hit).
pub fn spells_native() -> bool {
    native() && MAKE_OK.load(Ordering::Relaxed)
}

/// The hits KH1's pass made since the last call with the spell attack objects whose keys carry
/// `mark` (a runtime's own key bit): (key, ATKP, actor, damage).
pub fn spell_results(mark: u64) -> Vec<(u64, u16, usize, Option<u32>)> {
    let Ok(mut g) = STATE.try_lock() else { return Vec::new() };
    let (mine, mut rest): (Vec<_>, Vec<_>) = std::mem::take(&mut g.results).into_iter().partition(|r| r.0 & mark == mark);
    // (Another runtime's, kept for it; not without end, if it no longer takes them.)
    let over = rest.len().saturating_sub(64);
    rest.drain(..over);
    g.results = rest;
    mine
}

/// A spell attack its script made, sized or moved just now (`reflect::KhWorld::attack_set`, t819):
/// its KH1 attack object made or moved at once, as KH1's own spell scripts make and move theirs.
/// It stays where the script last put it until the script moves or frees it (`spell_forget`).
pub unsafe fn spell_place(sora: usize, attack: &super::combat::SpellAttack, row: &dyn Fn(u16, u8, u8) -> Option<kh2_formats::battle::Atkp>) {
    place(sora, std::slice::from_ref(attack), row, true);
}

/// Whether the spell attack `key`'s object has hit something (KH2's `trap_attack_is_hit`, t819).
/// (t1047) `attack` is one of our KH2 spell objects (`kh2damage::share_of` leaves them out).
pub fn spell_object(attack: usize) -> bool {
    let Some(i) = pool_index(attack) else { return false };
    STATE.try_lock().is_ok_and(|g| g.spells.iter().any(|(k, _)| *k == i) && g.live[i].is_some_and(|l| l.obj == attack))
}

pub fn spell_hit(key: u64) -> bool {
    STATE.try_lock().is_ok_and(|g| g.spells.iter().any(|(_, sp)| sp.key == key && sp.hit))
}

/// The spell attacks' objects made or moved; `by_calls`: by the script's own calls (kept until freed)
/// rather than by our step each frame (gone a frame after it stops coming).
unsafe fn place(sora: usize, attacks: &[super::combat::SpellAttack], row: &dyn Fn(u16, u8, u8) -> Option<kh2_formats::battle::Atkp>, by_calls: bool) {
    use combat_core::ShapeKind;
    let base = BASE.load(Ordering::Relaxed);
    let Ok(mut g) = STATE.try_lock() else { return };
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
            sp.by_calls = by_calls;
            continue;
        }
        g.spells.retain(|(_, sp)| sp.key != a.key);
        let rows: Vec<kh2_formats::battle::Atkp> = (0..SPELL_ROWS).map_while(|c| row(a.atk, a.sub, c)).collect();
        let Some(first) = rows.first().cloned() else { continue };
        let k1 = if first.knockback_1 == i16::MAX { 0 } else { first.knockback_1 };
        let knock = character_core::reaction::category(first.enemy_reaction) == 4 || k1 >= 100;
        // The spark: the spell package's effect list and KH2's group for the row.
        let spark = a.package.and_then(|p| {
            let group = super::combat::kh2_group(super::combat::Kh2Spark { pkg: super::kh2dpx::pkg_of(p), effect_on_hit: first.effect_on_hit, random_effect: first.random_effect });
            let (list, _) = super::kh2dpx::hit_list(super::kh2dpx::pkg_of(p), group)?;
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
        // A Magic-based KH2 attack on the swing's data (no KH1 spell of its own): KH1's own magic
        // stat (`kh1rows::kh1_magic`, Ripple Drive's). A KH1 spell's data keeps its own (t920: its
        // power points and max MP, KH1's own magic formula; `kh2damage` counts KH2's power
        // against KH2's normal cast).
        if a.kh1_data.is_none() && first.stat_source == 1 {
            super::kh1rows::kh1_magic_row(d);
        }
        // (t1046) A knocking hit on the finisher's data: no critical chance or bonus (KH1's spells
        // never roll; KH2 has no critical).
        if a.kh1_data.is_none() && knock {
            wr(d + 0x18, 0i32);
            wr(d + 0x1C, 0i32);
        }
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
        g.live[i] = Some(Live { obj, data: d, motion: 0, atkp: a.atk, shape: dummy, magnet: None, prev: None, last: false });
        let mut sp = SpellObj { key: a.key, atk: a.atk, rows, group: a.group, shapes: [([0.0; 3], 1, 0.0, 0.0); SPELL_SHAPES], n: 0, seen: frame, kh1_spell: a.kh1_data.is_some(), by_calls, hit: false, send_off: a.send_off };
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
        note(|| format!("KH2 spell hits: ATKP {} sub {} on KH1 attack object {obj:#x} (data {d:#x}{}, {} shape(s), spark {:?}{})", a.atk, a.sub, if a.kh1_data.is_some() { ", KH1 spell's" } else { "" }, a.shapes.len(), spark.map(|s| s.1), if by_calls { ", by the script's calls" } else { "" }));
    }
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
            super::rehit::forget(l.obj);
        }
    }
}

/// The window's ATKP for an attack object of ours (`kh1rows::kh2_atkp`).
pub unsafe fn atkp(attack: usize) -> Option<kh2_formats::battle::Atkp> {
    if !native() {
        return None;
    }
    // (Valor's left Keyblade's object: the window open on it.)
    if let Some(a) = super::lefthits::atkp(attack) {
        return Some(a);
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

/// (t1053) For an attack object of ours (`atkp`'s): whether a KH2 blow from it stays KH1's
/// knock-back or launch (`combat::scale_record`). A body window: the clip's last blow on his body in
/// a finisher (`Live::last`, t1046's rule). A blade of `lefthits`: its window's (`lefthits::last_blow`).
/// A KH2 spell: (t1056) by its cast (`SpellObj::send_off`): yes on the combo finisher's cast and on
/// Thunder's every cast, else no (t1057: Fire no on every cast, its finisher too), so a plain cast's
/// blows go to KH1's flinch (the owner's rule). Which
/// hit of a script is its last still can't be told from one attack (t1046), so every blow of a
/// finisher cast keeps its blow. `None` when `attack` isn't ours.
pub unsafe fn last_blow(attack: usize) -> Option<bool> {
    if !native() {
        return None;
    }
    if let Some(last) = super::lefthits::last_blow(attack) {
        return Some(last);
    }
    let i = pool_index(attack)?;
    let g = STATE.try_lock().ok()?;
    let l = g.live[i].filter(|l| l.obj == attack && still_ours(l))?;
    if let Some((_, sp)) = g.spells.iter().find(|(k, _)| *k == i) {
        return Some(sp.send_off);
    }
    Some(l.last)
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
    super::lefthits::hit(attack, target);
    let Some(i) = pool_index(attack) else { return };
    let Ok(mut g) = STATE.try_lock() else { return };
    g.current = None;
    let spell = g.spells.iter_mut().find(|(k, _)| *k == i).map(|(_, sp)| {
        sp.hit = true;
        (sp.key, sp.atk)
    });
    if let Some((key, atk)) = spell {
        let dealt = (rec > 0x10000).then(|| rd::<i32>(rec + 8).max(0) as u32);
        if g.results.len() < 256 {
            g.results.push((key, atk, target, dealt));
        }
    }
    let Some(l) = g.live[i].filter(|l| l.obj == attack && still_ours(l)) else { return };
    let (kind, atk) = if spell.is_some() {
        ("spell", l.atkp)
    } else {
        ("body", l.atkp)
    };
    let motion = l.motion;
    drop(g);
    // (t1050) The body window log: KH1 made contact in this window.
    if spell.is_none() {
        window_contact(attack, target);
    }
    note(|| format!("KH2 {kind} hit: ATKP {atk} (motion {motion}) from KH1's pass on actor {target:#x}"));
    // KH2's Magnet catch, on every hit (`0x14040FBF0`): KH1's state of ours (`magnet`).
    if let Some((k1, k2, swirl)) = l.magnet {
        super::magnet::catch(attack, target, rec, k1, k2, swirl);
    }
}

/// The place of a live Magnet object of ours (`magnet`'s step): KH1's attack object's own place
/// (`+0x10`, KH1 world, which `entries` writes as KH1's pass builds it), while KH1's pass built it
/// this frame or the last (its window open). `Err` when our table is busy.
pub fn magnet_centre(obj: usize) -> Result<Option<[f32; 3]>, ()> {
    if !native() {
        return Ok(None);
    }
    let Some(i) = pool_index(obj) else { return Ok(None) };
    let Ok(g) = STATE.try_lock() else { return Err(()) };
    let Some(l) = g.live[i].filter(|l| l.obj == obj && l.magnet.is_some()) else { return Ok(None) };
    drop(g);
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    let Some((_, f)) = l.prev else { return Ok(None) };
    // (Freed or reused by KH1 meanwhile, or its window shut: no Magnet.)
    if frame.saturating_sub(f) > 1 || unsafe { !still_ours(&l) } {
        return Ok(None);
    }
    let c = unsafe { rd::<[f32; 3]>(obj + 0x10) };
    Ok(c.iter().all(|v| v.is_finite()).then_some(c))
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
    // (Valor's left Keyblade's object: its bone-source actor brought up to date, then KH1's own
    // builder, as for the right Keyblade's, `lefthits`.)
    super::lefthits::before_build(obj);
    // (A Reflect barrier's guard object: KH2's barrier shapes, t796.)
    if let Some(n) = guard_entries(obj, list, count) {
        return n;
    }
    // (A body guard of KH2's, placed from KH1's state, t909.)
    if let Some(n) = body_guard_entries(obj, list, count) {
        return n;
    }
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
        let stale = frame.saturating_sub(sp.seen) > 1;
        // (One its script placed and never freed, its runtime gone meanwhile (a room change, a
        // reset): inert after `BY_CALLS_MAX` frames standing still.)
        if sp.n == 0 || (stale && !sp.by_calls) || frame.saturating_sub(sp.seen) > BY_CALLS_MAX {
            return count;
        }
        // (Not moved since the frame before last: where it stands, no sweep, as KH2's attack that
        // wasn't moved.)
        let (shapes, n) = (sp.shapes, if stale { 1 } else { sp.n });
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
    let (motion, atkp) = (l.motion, l.atkp);
    drop(g);
    let mut count = count;
    let start = count;
    for q in &centres[..n] {
        count = put_entry(obj, owner, bit, list, count, shape.kh1_type, shape.radius, shape.height, *q);
    }
    // (t1050) The body window log (Combat logging only): each enemy near against these entries.
    window_log(obj, owner, list + start.max(0) as usize * 0x40, (count - start).max(0) as usize, motion, atkp);
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

// ---------------------------------------------------------------------------------------------
// (t1050) The body window log (the owner's approval; log only, nothing of KH1's or ours changes).
// While a body window of ours is open (KH1's pass builds its object: Explosion's 911 / 912,
// Vicinity Break's, Magnet Burst's, Final's 154 body shapes), each enemy within `WLOG_REACH` of
// Sora is logged (Combat) once on the window's first frame it is seen and once when the window
// closes, with its frame of closest approach. What is read is what KH1's own pass reads for it
// (Steam, Checked (decomp)):
// - the actors: KH1's pool `0x142D372A0` (96 of 0x4B0), live with `+0x374` bit 1 set and bit 2
//   clear (`0x140291B20`, the pass's iterator); an enemy is of another team (`+0x374 >> 5 & 3`,
//   which KH1's entry test `0x1402B3BE0` compares with the attack entry's);
// - hittable (`0x140292070`, which the pass `0x1402B4460` asks before it puts an actor's parts in):
//   a part table (`[actor+0x134]`, count `+6` not 0), its byte `+6` not 2, its flags word
//   `& 0x8048` == `0x8000` (`0x40`: KH1's untouchable flag, its dodge roll's, `reflect`), and one
//   part not switched off by `+0x3A8`;
// - its parts: `[[actor+0x134]+0x18]`, 0x14 bytes each (`+0` hit id, `+1` type, `+2` bone code,
//   `+4` t, `+8` lift, `+0xC` radius, `+0x10` half height), part i switched off by `+0x3A8` bit i;
//   each placed by KH1's own placement `0x1402B5E50 (actor, part, shape)` (as the pass places them;
//   it only reads the actor and writes the shape we give it), first bytes checked;
// - the test: KH1's own overlap `0x1402B5EF0 (attack shape, body shape)` (first bytes checked, as
//   `unmet_types`) against every entry of ours this frame (this frame's shape and KH2's sweep), and
//   our normalised squared gap of that same formula (`gap`: below 1 meets);
// - why none met (`0x1402B3BE0`'s order): not hittable, the attack object refused for bodies (dead
//   `0x200000`, `0x400`, result `+4 & 0x30`, clash types 1 / 2 / 4, the one-hit bit `0x80` with a
//   result), the hit bit held (`actor+0x3E0 & object+0x54`), else the parts: a part switched off that
//   would meet, or outside the shape.
// Contact: our hit hook (`hit`, KH1's contact through `probe::hit`). Bounded: only while Combat
// logging is on, only in an open window (inside KH1's pass, `entries`), enemies within 800, at most
// `WLOG_ENEMIES` per window and `WLOG_PARTS` parts per line, `WLOG_LINES` lines a session. Nothing
// runs outside open windows but one atomic load in `frame`.

/// KH1's part placement `0x1402B5E50 (actor, part, shape)` (Steam; `mov [rsp+8], rbx; mov
/// [rsp+0x10], rsi; push rdi; sub rsp, 0x20`, Checked (exe)).
const PLACE_STEAM: usize = 0x2B5E50;
const PLACE_BYTES: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20];
/// KH1's actor pool (Steam, the pass's iterator `0x140291B20`): 96 of 0x4B0.
const ACTORS_STEAM: usize = 0x2D372A0;
const ACTOR_COUNT: usize = 96;
const ACTOR_SIZE: usize = 0x4B0;
const WLOG_REACH: f32 = 800.0;
const WLOG_PARTS: usize = 12;
const WLOG_ENEMIES: usize = 16;
const WLOG_WINDOWS: usize = 8;
const WLOG_LINES: u32 = 160;
/// KH1's placement and overlap are the ones we know (0 unknown, 1 yes, 2 no).
static PLACE_OK: AtomicU32 = AtomicU32::new(0);
static WLOG_PENDING: AtomicBool = AtomicBool::new(false);
static WLOG_LOGGED: AtomicU32 = AtomicU32::new(0);

struct WinEnemy {
    actor: usize,
    id: u32,
    /// The smallest gap of any of its parts this window, its frame and that frame's line.
    best: f32,
    best_frame: u64,
    best_line: String,
    best_why: &'static str,
    contact: Option<u64>,
}

struct WinLog {
    obj: usize,
    motion: u16,
    atkp: u16,
    open: u64,
    last: u64,
    enemies: Vec<WinEnemy>,
}

static WLOG: Mutex<Vec<WinLog>> = Mutex::new(Vec::new());

fn wlog_line(s: String) {
    if WLOG_LOGGED.fetch_add(1, Ordering::Relaxed) < WLOG_LINES {
        log!(Combat, "{}", s);
    }
}

/// KH1's placement and overlap checked (once).
unsafe fn place_ok(base: usize) -> bool {
    match PLACE_OK.load(Ordering::Relaxed) {
        1 => return true,
        2 => return false,
        _ => {}
    }
    let same = |at: usize, bytes: &[u8]| crash_ok(base + at, bytes.len()) && std::slice::from_raw_parts((base + at) as *const u8, bytes.len()) == bytes;
    let ok = same(PLACE_STEAM, &PLACE_BYTES) && same(OVERLAP_STEAM, &OVERLAP_BYTES);
    PLACE_OK.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
    if !ok {
        log!(Combat, "KH2 body window log: KH1's part placement or overlap has unexpected bytes: no window log");
    }
    ok
}

/// `n` bytes at `p` readable (`crash::readable` on its first and last byte).
unsafe fn crash_ok(p: usize, n: usize) -> bool {
    p > 0x10000 && super::crash::readable(p) && super::crash::readable(p + n.max(1) - 1)
}

/// KH1's entry test's formula as a normalised squared gap (below 1 meets; `0x1402B5EF0`, Checked
/// (decomp)): `a` / `b` (type, radius, half height, centre). Types 1 (ellipsoid) and 2 (column);
/// `None` for another (a box: KH1's own answer only).
fn gap(a: (u32, f32, f32, [f32; 3]), b: (u32, f32, f32, [f32; 3])) -> Option<f32> {
    let (lo, hi) = if b.0 < a.0 { (b, a) } else { (a, b) };
    let d = [hi.3[0] - lo.3[0], hi.3[1] - lo.3[1], hi.3[2] - lo.3[2]];
    let horiz = d[0] * d[0] + d[2] * d[2];
    let dy = d[1].abs();
    let sq = |v: f32| v * v;
    match (lo.0, hi.0) {
        (1, 1) => {
            let r = lo.1 + hi.1;
            let h = lo.2 + hi.2;
            (r > 0.0 && h > 0.0).then(|| horiz / sq(r) + sq(d[1] / h))
        }
        (1, 2) => {
            let (he, hc) = (lo.2, hi.2);
            if !(he > 0.0) {
                return None;
            }
            if dy >= he + hc {
                return Some(sq(dy / (he + hc)).max(1.0));
            }
            let re = if dy <= hc {
                lo.1
            } else {
                let t = (dy - hc) / he;
                (1.0 - t * t).abs().sqrt() * lo.1
            };
            let r = re + hi.1;
            (r > 0.0).then(|| horiz / sq(r))
        }
        (2, 2) => {
            let (r, h) = (lo.1 + hi.1, lo.2 + hi.2);
            (r > 0.0 && h > 0.0).then(|| (horiz / sq(r)).max(sq(dy / h)))
        }
        _ => None,
    }
}

/// The attack object's own refusal of bodies in KH1's entry test (`0x1402B3BE0`), if any.
unsafe fn attack_refuses(obj: usize) -> Option<String> {
    let flags = rd::<u32>(obj);
    let result = rd::<u32>(obj + 4);
    let t = (flags >> 17) & 7;
    let type_ok = ((flags & 0xE_0000).wrapping_sub(0x2_0000) & 0xFFF9_FFFF) != 0 || flags & 0xE_0000 == 0x6_0000;
    let why = if flags & 0x20_0000 != 0 {
        "marked dead (0x200000)"
    } else if flags & 0x400 != 0 {
        "flag 0x400"
    } else if result & 0x30 != 0 {
        "its result word already clashed (0x30)"
    } else if !type_ok {
        "its clash type meets no body"
    } else if flags & 0x80 != 0 && result != 0 {
        "one-hit and already used"
    } else {
        return None;
    };
    Some(format!("attack object refused for bodies: {why} (flags {flags:#x}, result {result:#x}, clash type {t})"))
}

/// The shape KH1's placement writes (an entry's `+0x20`..: type, radius, half height, centre `+0x10`;
/// 16-aligned as KH1's list entries are).
#[repr(C, align(16))]
struct PartShape([u8; 0x20]);

/// One part of an enemy this frame, placed by KH1.
#[derive(Clone, Copy)]
struct PartNow {
    hit_id: u8,
    kind: u32,
    bone: u16,
    centre: [f32; 3],
    radius: f32,
    half: f32,
    off: bool,
    gap: Option<f32>,
    meets: bool,
}

/// (t1050) `entries` built our body object `obj` this frame: its `n` entries at `first` in KH1's
/// list. Each enemy within reach measured against them, as KH1's pass will test it.
unsafe fn window_log(obj: usize, owner: usize, first: usize, n: usize, motion: u16, atkp: u16) {
    if !super::log_on(super::LogCat::Combat) || n == 0 || WLOG_LOGGED.load(Ordering::Relaxed) >= WLOG_LINES {
        return;
    }
    let base = BASE.load(Ordering::Relaxed);
    if !place_ok(base) || !crash_ok(first, n * 0x40) || !crash_ok(owner, 0x400) || !crash_ok(base + ACTORS_STEAM, ACTOR_COUNT * ACTOR_SIZE) {
        return;
    }
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    let Ok(mut logs) = WLOG.try_lock() else { return };
    // (A play of this object that closed meanwhile, or the oldest when full: summed up now.)
    let mut done: Vec<WinLog> = Vec::new();
    let mut k = 0;
    while k < logs.len() {
        if logs[k].obj == obj && logs[k].last + 1 < frame {
            done.push(logs.remove(k));
        } else {
            k += 1;
        }
    }
    if !logs.iter().any(|w| w.obj == obj) {
        if logs.len() >= WLOG_WINDOWS {
            done.push(logs.remove(0));
        }
        logs.push(WinLog { obj, motion, atkp, open: frame, last: frame, enemies: Vec::new() });
    }
    WLOG_PENDING.store(true, Ordering::Relaxed);
    let place: unsafe extern "system" fn(usize, usize, *mut PartShape) = std::mem::transmute(base + PLACE_STEAM);
    let overlap: unsafe extern "system" fn(usize, usize) -> u8 = std::mem::transmute(base + OVERLAP_STEAM);
    let feet = rd::<[f32; 3]>(owner + 0x10);
    let ours: Vec<(u32, f32, f32, [f32; 3])> = (0..n)
        .map(|i| {
            let e = first + i * 0x40;
            let c = rd::<[f32; 3]>(e + 0x30);
            (rd::<u32>(e + 0x20), rd::<f32>(e + 0x24), rd::<f32>(e + 0x28), c)
        })
        .collect();
    let above = |c: [f32; 3]| feet[1] - c[1];
    let out = |c: [f32; 3]| ((c[0] - feet[0]).powi(2) + (c[2] - feet[2]).powi(2)).sqrt();
    let w = logs.iter_mut().find(|w| w.obj == obj).unwrap();
    w.last = frame;
    if frame == w.open {
        let s = ours[0];
        let refused = attack_refuses(obj).map_or(String::new(), |r| format!("; {r}"));
        wlog_line(format!(
            "KH2 body window log: ATKP {atkp} (motion {motion}) object {obj:#x} opens: our shape type {} centre {:.0} above Sora's feet, {:.0} out, radius {:.0}, half height {:.0}, {n} entr{} (with KH2's sweep){refused}",
            s.0,
            above(s.3),
            out(s.3),
            s.1,
            s.2,
            if n == 1 { "y" } else { "ies" }
        ));
    }
    let team = |a: usize| (rd::<u32>(a + 0x374) >> 5) & 3;
    let our_team = (rd::<u32>(obj) >> 12) & 3;
    let bit = rd::<u32>(obj + 0x54);
    let refused = attack_refuses(obj).is_some();
    for ai in 0..ACTOR_COUNT {
        let a = base + ACTORS_STEAM + ai * ACTOR_SIZE;
        if a == owner {
            continue;
        }
        let live = rd::<u32>(a + 0x374);
        if live & 1 == 0 || live & 2 != 0 || team(a) == our_team {
            continue;
        }
        let p = rd::<[f32; 3]>(a + 0x10);
        let dist = ((p[0] - feet[0]).powi(2) + (p[1] - feet[1]).powi(2) + (p[2] - feet[2]).powi(2)).sqrt();
        if !(dist <= WLOG_REACH) {
            continue;
        }
        // KH1's hittable (`0x140292070`) and its parts.
        let table = pose::ptr(rd::<u32>(a + 0x134));
        let flags = rd::<u32>(a);
        let b6 = rd::<u8>(a + 6);
        let mask = rd::<u32>(a + 0x3A8);
        let count = if crash_ok(table, 0x1C) { rd::<u8>(table + 6) as usize } else { 0 };
        let parts = if count > 0 { pose::ptr(rd::<u32>(table + 0x18)) } else { 0 };
        let count = if crash_ok(parts, count * 0x14) { count } else { 0 };
        let on = |i: usize| (mask >> (i & 31)) & 1 == 0;
        let hittable = count > 0 && b6 != 2 && flags & 0x8048 == 0x8000 && (0..count).any(on);
        let mut now: Vec<PartNow> = Vec::with_capacity(count.min(32));
        for i in 0..count.min(32) {
            let part = parts + i * 0x14;
            let mut shape = PartShape([0u8; 0x20]);
            place(a, part, &mut shape);
            let sp = shape.0.as_ptr() as usize;
            let kind = rd::<u32>(sp);
            let (radius, half, centre) = (rd::<f32>(sp + 4), rd::<f32>(sp + 8), rd::<[f32; 3]>(sp + 0x10));
            let mut g: Option<f32> = None;
            let mut meets = false;
            for (k, s) in ours.iter().enumerate() {
                if let Some(v) = gap(*s, (kind, radius, half, centre)) {
                    g = Some(g.map_or(v, |x: f32| x.min(v)));
                }
                if overlap(first + k * 0x40 + 0x20, sp) != 0 {
                    meets = true;
                }
            }
            now.push(PartNow { hit_id: rd::<u8>(part), kind, bone: rd::<u16>(part + 2), centre, radius, half, off: !on(i), gap: g, meets });
        }
        let held = rd::<u32>(a + 0x3E0) & bit != 0;
        let why: &'static str = if !hittable {
            "not hittable"
        } else if refused {
            "the attack object refused for bodies"
        } else if held {
            "hit bit already held"
        } else if now.iter().any(|q| !q.off && q.meets) {
            "would meet"
        } else if now.iter().any(|q| q.off && q.meets) {
            "part off"
        } else {
            "outside the shape"
        };
        // This frame's closest part (smallest gap; KH1's "meets" as 0 where we have no gap of ours).
        let closest = now.iter().map(|q| q.gap.unwrap_or(if q.meets { 0.0 } else { f32::INFINITY })).fold(f32::INFINITY, f32::min);
        let id = super::probe::object_id(a);
        let state = rd::<i32>(a + 0x70);
        let line = || {
            let mut s = format!(
                "enemy object {id} ({a:#x}, state {state}, {dist:.0} from Sora; hittable {hittable}: flags {flags:#x} (0x8000 {}, untouchable 0x40 {}, 0x8 {}), byte +6 {b6}, parts {count}, off mask {mask:#x}; hit bit {})",
                flags & 0x8000 != 0,
                flags & 0x40 != 0,
                flags & 0x8 != 0,
                if held { "held" } else { "free" }
            );
            for (i, q) in now.iter().enumerate().take(WLOG_PARTS) {
                s.push_str(&format!(
                    "; part {i} (id {}, type {}, bone {:#x}): centre {:.0} above Sora's feet, {:.0} out, radius {:.0}, half height {:.0}, {}, gap {}, KH1 meets {}",
                    q.hit_id,
                    q.kind,
                    q.bone,
                    above(q.centre),
                    out(q.centre),
                    q.radius,
                    q.half,
                    if q.off { "off" } else { "on" },
                    q.gap.map_or("n/a".to_string(), |v| format!("{v:.2}")),
                    if q.meets { "yes" } else { "no" }
                ));
            }
            if now.len() > WLOG_PARTS {
                s.push_str(&format!("; {} more parts", now.len() - WLOG_PARTS));
            }
            s
        };
        let pos = match w.enemies.iter().position(|e| e.actor == a) {
            Some(i) => i,
            None => {
                if w.enemies.len() >= WLOG_ENEMIES {
                    continue;
                }
                let l = line();
                wlog_line(format!("KH2 body window log: ATKP {} (motion {}) first frame (+{}) of {l}; now: {why}", w.atkp, w.motion, frame - w.open));
                w.enemies.push(WinEnemy { actor: a, id, best: f32::INFINITY, best_frame: frame, best_line: String::new(), best_why: why, contact: None });
                w.enemies.len() - 1
            }
        };
        let e = &mut w.enemies[pos];
        if closest < e.best || e.best_line.is_empty() {
            e.best = closest;
            e.best_frame = frame;
            e.best_line = line();
            e.best_why = why;
        }
    }
    drop(logs);
    for w in done {
        window_summary(w);
    }
}

/// (t1050) KH1's pass made contact with our body object `attack` on `target` (`hit`).
fn window_contact(attack: usize, target: usize) {
    if !super::log_on(super::LogCat::Combat) {
        return;
    }
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    let Ok(mut logs) = WLOG.try_lock() else { return };
    let Some(w) = logs.iter_mut().find(|w| w.obj == attack) else { return };
    if let Some(e) = w.enemies.iter_mut().find(|e| e.actor == target) {
        if e.contact.is_none() {
            e.contact = Some(frame);
        }
    }
}

/// (t1050) Each frame (`frame`): the windows KH1's pass didn't build last frame are closed and
/// summed up. One atomic load while none is open.
fn window_flush_due() {
    if !WLOG_PENDING.load(Ordering::Relaxed) {
        return;
    }
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    let done: Vec<WinLog> = {
        let Ok(mut logs) = WLOG.try_lock() else { return };
        let (gone, keep): (Vec<WinLog>, Vec<WinLog>) = std::mem::take(&mut *logs).into_iter().partition(|w| w.last + 1 < frame);
        *logs = keep;
        if logs.is_empty() {
            WLOG_PENDING.store(false, Ordering::Relaxed);
        }
        gone
    };
    for w in done {
        window_summary(w);
    }
}

/// (t1050) One line per enemy of a closed window: contact, else why not, and its closest frame.
fn window_summary(w: WinLog) {
    if !super::log_on(super::LogCat::Combat) {
        return;
    }
    let frames = w.last - w.open + 1;
    for e in &w.enemies {
        let result = match e.contact {
            Some(f) => format!("KH1 made contact at +{}", f.saturating_sub(w.open)),
            None => format!(
                "no contact: {}",
                match e.best_why {
                    "would meet" => "KH1's test would meet at the closest frame but no contact came (cause not read)",
                    other => other,
                }
            ),
        };
        let best = if e.best.is_finite() { format!("{:.2}", e.best) } else { "none".to_string() };
        wlog_line(format!(
            "KH2 body window log: ATKP {} (motion {}) object {:#x} closed after {frames} frame(s): enemy object {}: {result}; closest at +{} (gap {best}): {}",
            w.atkp,
            w.motion,
            w.obj,
            e.id,
            e.best_frame.saturating_sub(w.open),
            e.best_line
        ));
    }
}
