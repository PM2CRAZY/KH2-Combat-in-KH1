//! Valor Form's second Keyblade in Sora's left hand (`docs/kh1-weapon-second.md`).
//!
//! KH1 gives a party member one weapon: its `.wpn` model is skinned to the last 13 bones of his
//! skeleton (280..292, the grip under the right hand and the chain), and KH1's model queue
//! (`0x1401D1F10`) queues it with the body, copying the bone matrices at that moment. So:
//!
//! 1. **Load** the Valor Keyblade's `.wpn` once, with KH1's own loader (`0x140289A80`) into a
//!    buffer of ours, and set its model up with KH1's setup (`0x1401D45F0`) in the callback.
//!    The loader's file record carries the remastered assets, so the HD mesh is hooked up as
//!    for KH1's own Keyblade. It stays loaded for the session; a different Keyblade chosen
//!    later is loaded in a new buffer and the old one released (KH1's own order: the model,
//!    then the file record with its textures, with the `.wpn` release handler kept out of it).
//! 2. **Pose**: `pose` hands over the left hand's 13 weapon matrices each frame (`note_pose`),
//!    from KH2's own left Keyblade (the Valor rig's second weapon, `kh2_left` from `takeover`).
//!    A weapon that isn't a Keyblade (Valor's default, the Dream Sword, item 0x52) is held
//!    rigidly as the right hand holds one (`dream_left`): KH1's own hold of it put on the left
//!    grip, or the Keyblade's grip with the bones as in KH1's bind.
//! 3. **Draw**: after KH1 queues Sora with his Keyblade out, queue him a second time with every
//!    body mesh hidden, our model as the weapon and the left-hand matrices put in bones
//!    280..292 for that call only (put back at once).
//!
//! 4. **Trail**: KH1's own Keyblade trail, with the left Keyblade's own look: the effect data
//!    from its own `.wpn` (the file's second section, set up as KH1 sets up its weapon's,
//!    `0x1401DDF20`; the right one's if it has none), and the trail id KH1 was last asked for this
//!    swing. It runs on the frames KH2's left weapon has its trail (`W_EX010_BTLF_L`'s ranges).
//!    - KH1's trail is an effect that follows a bone of its owner actor each frame
//!      (`0x1402AC780`), read through the owner's model instance's world matrices. A point along
//!      a bone is taken from its parent's world matrix and the bone's local offset
//!      (`0x1401D41C0`): for the grip, the right hand's.
//!    - Its owner here is a copy of Sora's actor. That copy's instance is a copy of his, with its
//!      own matrices: every one of his moved by the turn that takes his right grip to the left
//!      one, so whichever bones the trail reads (the hand, the grip, the blade) are the left
//!      Keyblade's (the user: "the second keyblades' trail doesnt follow the second keyblade").
//!
//! Everything is off with `valor_left_blade = off` (nothing installed). Every step is logged a
//! few times.

use super::addr::Rva;
use super::log;
use glam::Mat4;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// KH1's model queue `(instance, view struct, draw params, actor)`.
const QUEUE_RVA: Rva = Rva(0x1D1F10);
/// `push rbp; push rbx; push rsi; push rdi` (both builds).
const QUEUE_PROLOGUE: [u8; 5] = [0x40, 0x55, 0x53, 0x56, 0x57];
/// The file loader `(name, buffer, callback(size, id, buffer), delay)`: queued on KH1's loader
/// task; the callback runs on the game thread once the file is in.
const LOAD_RVA: Rva = Rva(0x289A80);
/// A `jmp` to the model setup `0x1401D5AD0 (container)` (relocation, the HD table).
const SETUP_RVA: Rva = Rva(0x1D45F0);
/// The model release `(container)` (KH1's `.wpn` release handler calls it).
const RELEASE_MODEL_RVA: Rva = Rva(0x1D49D0);
/// A weapon's effect data set up (offsets to handles, its parts made) and released, as KH1's
/// `.wpn` handler and its release do (`0x140285120` / `0x1402851E0`).
const EFFECTS_SETUP_RVA: Rva = Rva(0x1DDF20);
const EFFECTS_RELEASE_RVA: Rva = Rva(0x1DED70);
/// The file record holding an address `(addr, unlink)`, and dropping one `(record, textures)`.
const FIND_RECORD_RVA: Rva = Rva(0xE2B10);
const DROP_RECORD_RVA: Rva = Rva(0xE2DF0);
/// KH1's `.wpn` release handler and the place in the file-type table that points at it (in
/// writable data): replaced by `wpn_release`, which keeps KH1's handler off our buffers (it
/// looks a buffer up among the three party slots and would use garbage for ours).
const WPN_RELEASE_RVA: Rva = Rva(0x2851E0);
const WPN_RELEASE_SLOT_RVA: Rva = Rva(0x4DAF48);
/// `btltbl.bin` and its item table pointer; each member's weapon table offset (Sora's first).
const ITEM_TABLE_PTR_RVA: Rva = Rva(0x2D23738);
const BTLTBL_RVA: Rva = Rva(0x2D23740);
const WEAPON_TABLE_OFFSETS_RVA: Rva = Rva(0x528D60);

/// KH1's trail request `(actor, id)`, hooked to learn the swing's trail id. The trail effect
/// maker `(owner, effect data, id) -> effect`. An effect's end (`jmp 0x1401EA310`). Pointer →
/// handle.
const TRAIL_START_RVA: Rva = Rva(0x2C0160);
const TRAIL_START_PROLOGUE: [u8; 5] = [0x48, 0x89, 0x5C, 0x24, 0x08];
const TRAIL_MAKE_RVA: Rva = Rva(0x2AC100);
const EFFECT_KILL_RVA: Rva = Rva(0x1DE890);
const TO_HANDLE_RVA: Rva = Rva(0x38B080);
/// Actor fields: its size, its weapon object and model instance handles, and its state bits
/// (`+0x374`: bit 0 alive, bit 1 gone; `0x14028FE60`).
const ACTOR_SIZE: usize = 0x4B0;
const ACTOR_WEAPON: usize = 0x14C;
const ACTOR_INSTANCE: usize = 0x15C;
const ACTOR_STATE: usize = 0x374;
/// The actor's flags word holding the "trail wanted" bit (`combat::TRAIL_FLAG`).
const ACTOR_TRAIL_FLAGS: usize = 0x1CC;
const TRAIL_FLAG: u32 = 0x80_0000;
/// A model instance's size, and the weapon object's effect data (the `.wpn`'s second section).
const INSTANCE_SIZE: usize = 0x200;
const OBJ_EFFECTS: usize = 0x18;
/// An effect: its handle to the "end" byte, its state (`& 3 == 1` running; bit 0x20 followed),
/// and its handle to the block whose first word is its owner.
const FX_END: usize = 0x10;
const FX_STATE: usize = 0x14;
const FX_OWNER: usize = 0x18;
/// Our owner block: the actor copy, the instance copy, the matrices.
const FAKE_SIZE: usize = 0x8000;
/// Its weapon object (a copy of Sora's, `0x140284D40` builds his: 0x60 bytes, `+0x18` the
/// weapon's effect data, `+0x20` its row in `btltbl.bin`'s weapon table, whose `+0x24` / `+0x28`
/// are the blade's range along bone 0x30, `0x14029C990`): the left Keyblade's own row, so its
/// trail spans its own blade, not the right one's (the user: "valor forms 2nd keyblade trail is
/// not aligned with the keyblades length").
const FAKE_WEAPON: usize = 0x700;
const WEAPON_OBJ_SIZE: usize = 0x60;
const OBJ_ROW: usize = 0x20;
/// A weapon row's blade range (start, end) along bone 0x30, and the rows' size.
const ROW_BLADE: usize = 0x24;
const WEAPON_ROW_SIZE: usize = 0x58;
const FAKE_INSTANCE: usize = 0x800;
const FAKE_MATS: usize = 0x1000;
/// Trails kept at once (one swinging, the others fading).
const MAX_TRAILS: usize = 4;

/// A file record's buffer, size, and its file-type row (-1: no handler is called on release).
const REC_BUFFER: usize = 0x658;
const REC_ROW: usize = 0x64C;
/// The instance's world matrices (handle).
const INST_WORLD: usize = 0x1B4;
/// The draw params: the weapon model, the hidden-mesh mask; their size.
const P_WEAPON: usize = 0x18;
const P_MASK: usize = 0x38;
const P_SIZE: usize = 0x60;
/// The weapon bones: the first, how many, and Sora's bone count.
const GRIP: usize = 280;
const WEAPON_BONES: usize = 13;
const SORA_BONES: usize = 293;
/// KH1 Sora's hands (`pose::HANDS`).
const RIGHT_HAND: usize = 101;
const LEFT_HAND: usize = 206;
/// KH2's Keyblade bone the blade is on (`W_EX010.mdlx`: its mesh and its hit shapes are on bone 2;
/// bones 0 and 1 place it, 3.. are the keychain). Final Form's weapon motions turn the blade on
/// this bone, not on the root: the glide's (205..207) spins it six times a loop about its z, the
/// idle rolls it once a loop (the user: "sora's keyblades dont spin properly during final form
/// glide"; we followed the root, which the glide holds still beside him).
pub const KH2_BLADE: usize = 2;
/// A KH2 weapon's blade bone (its root if the pose is short).
fn kh2_blade(weapon: &[Mat4]) -> Option<Mat4> {
    weapon.get(KH2_BLADE).or(weapon.first()).copied()
}
/// Our load buffer (Sora's own weapon slot is 0x40000; twice that to be safe).
const BUF_SIZE: usize = 0x80000;
const MENV: u32 = u32::from_le_bytes(*b"MENV");
const HD_MARK: u32 = 0x9696_9696;
/// A load that hasn't answered by then is given up (KH1 never calls back on a failure).
const LOAD_TIMEOUT_S: f32 = 15.0;

static INSTALLED: AtomicBool = AtomicBool::new(false);
static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_QUEUE: AtomicUsize = AtomicUsize::new(0);
static IN_SECOND: AtomicBool = AtomicBool::new(false);
/// Valor's form is out (`show` / `hide`).
static SHOWN: AtomicBool = AtomicBool::new(false);
/// The mesh model drawn in the left hand (0: none ready).
static MESH: AtomicUsize = AtomicUsize::new(0);
/// The left Keyblade's effect data (0: none; the right one's used).
static EFFECTS: AtomicUsize = AtomicUsize::new(0);
/// The buffer of the load under way (0: none).
static PENDING_BUF: AtomicUsize = AtomicUsize::new(0);
/// The left weapon's KH1 item, as last shown (0: none): a dream weapon is held its own way.
static LEFT_ITEM: AtomicU8 = AtomicU8::new(0);

struct Loaded {
    item: u8,
    buf: usize,
    container: usize,
    mesh: usize,
    /// Its effect data (the trail's look), 0 if the file has none.
    effects: usize,
}

struct State {
    loaded: Option<Loaded>,
    /// The load under way: item, buffer, since.
    pending: Option<(u8, usize, Instant)>,
    /// Items whose load failed this session (not asked again).
    failed: Vec<u8>,
    /// Every buffer we ever gave KH1's loader (never freed: a late answer may still land).
    bufs: Vec<usize>,
}

static STATE: Mutex<State> = Mutex::new(State { loaded: None, pending: None, failed: Vec::new(), bufs: Vec::new() });
/// KH2's left Keyblade this frame (KH2 model space), from `takeover`.
/// Its last few steps, each with the body's root bone of the same step (`note_pose` takes the one
/// whose body it is drawing, the user: the second Keyblade jittered as Valor fell in place).
static KH2_LEFT: Mutex<std::collections::VecDeque<(u64, Vec<Mat4>, Instant)>> = Mutex::new(std::collections::VecDeque::new());

/// KH2's left Keyblade's root (KH2 model space) of the last step, if recent (`kh2fx`'s weapon
/// anchors for Valor's and Final's effects).
pub fn kh2_left_root() -> Option<Mat4> {
    let g = KH2_LEFT.try_lock().ok()?;
    g.back().filter(|(_, _, t)| t.elapsed().as_secs_f32() < 0.5).and_then(|(_, b, _)| b.first().copied())
}

/// KH2's left Keyblade's blade bone (`KH2_BLADE`, KH2 model space) of the last step, if recent.
pub fn kh2_left_blade() -> Option<Mat4> {
    let g = KH2_LEFT.try_lock().ok()?;
    g.back().filter(|(_, _, t)| t.elapsed().as_secs_f32() < 0.5).and_then(|(_, b, _)| kh2_blade(b))
}

/// Final Form's two floating Keyblades as drawn (KH1's world): KH2's blade bone (`KH2_BLADE`) of
/// each, where KH1's floated Keyblade is (`final_float`, `note_pose`: KH1's hand, KH2's float and
/// spin from it), right then left, and when. KH2's weapon anchors are that bone
/// (`kh2fx::form_bone`): Final's elements on them sit along the blade and turn with it (the user:
/// "some of the particles effects in final form's combo aren't 1:1").
static FINAL_BLADES: Mutex<[Option<(Mat4, Instant)>; 2]> = Mutex::new([None, None]);

/// Final Form's Keyblade `second` (0 right, 1 left) as drawn: its KH2 blade bone in the world
/// (KH2 world, y down, as `kh2fx`'s bones), if drawn in the last quarter second.
pub fn final_blade(second: bool) -> Option<Mat4> {
    let g = FINAL_BLADES.try_lock().ok()?;
    g[second as usize].filter(|(_, t)| t.elapsed().as_secs_f32() < 0.25).map(|(m, _)| m)
}

fn note_final_blade(second: bool, world: Mat4) {
    if !world.is_finite() {
        return;
    }
    if let Ok(mut g) = FINAL_BLADES.try_lock() {
        g[second as usize] = Some((world, Instant::now()));
    }
}

/// A body pose's signature (its bones' places, bit for bit).
fn body_sig(body: &[Mat4]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for m in body {
        for v in m.w_axis.to_array() {
            v.to_bits().hash(&mut h);
        }
    }
    h.finish()
}
/// Sora's model-to-world as last posed (the left Keyblade's bones are kept in model space and put
/// in the world with the frame's own placement: a pose a frame old no longer lags behind him).
static TO_WORLD: Mutex<Option<Mat4>> = Mutex::new(None);
/// The left hand's weapon bones in KH1's world, and the game frame they were made on.
static LEFT: Mutex<Option<([Mat4; WEAPON_BONES], u64)>> = Mutex::new(None);

/// The left hand's weapon bones in KH1's world now (`LEFT`, model space, by this frame's
/// placement), if posed within 2 game frames.
fn left_world() -> Option<[Mat4; WEAPON_BONES]> {
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    let m = LEFT.try_lock().ok().and_then(|g| g.as_ref().filter(|(_, at)| frame.saturating_sub(*at) <= 2).map(|(m, _)| *m))?;
    let w = TO_WORLD.try_lock().ok().and_then(|g| *g)?;
    Some(m.map(|b| w * b))
}
static ORIG_TRAIL_START: AtomicUsize = AtomicUsize::new(0);
static TRAIL_HOOKED: AtomicBool = AtomicBool::new(false);
/// The trail id KH1 was last asked for Sora (`u32::MAX`: none yet).
static TRAIL_ID: AtomicU32 = AtomicU32::new(u32::MAX);
/// Our owner block (0: not made), the instance copy's and matrices' handles; whether it was
/// filled this Valor.
static FAKE: AtomicUsize = AtomicUsize::new(0);
static FAKE_INSTANCE_H: AtomicU32 = AtomicU32::new(0);
static FAKE_WEAPON_H: AtomicU32 = AtomicU32::new(0);
/// The left Keyblade's weapon row (0: unknown; the right one's is used).
static LEFT_ROW: AtomicUsize = AtomicUsize::new(0);
static FAKE_MATS_H: AtomicU32 = AtomicU32::new(0);
static FAKE_FILLED: AtomicBool = AtomicBool::new(false);
/// The left weapon's trail window is open (last KH2 step).
static LEFT_OPEN: AtomicBool = AtomicBool::new(false);
/// KH2's left weapon's trail ranges by motion id (each two-Keyblade form's own left weapon motion
/// set, range 12), read once, by the form's moveset (`TRAIL_MOVESETS`). Valor's alone were read
/// before, and Final's left trail opened and closed on Valor's frames for its motion ids (its
/// 154 trails 11..124 of 280 frames, Valor's 37..95; its 183 four windows over 360 frames, Valor's
/// one, 19..29), as Master's would have.
type Windows = std::collections::HashMap<u16, Vec<(u32, Option<u32>)>>;
static TRAIL_WINDOWS: std::sync::OnceLock<std::collections::HashMap<usize, Windows>> = std::sync::OnceLock::new();
/// The two-Keyblade forms' movesets (03system `wmst` row `moveset × 2 + 1` is the left weapon's)
/// and their left sets' names if `wmst` can't be read.
const TRAIL_MOVESETS: [(usize, &str); 3] = [(2, "W_EX010_BTLF_L"), (4, "W_EX010_TRIF_L"), (5, "W_EX010_ULTF_L")];

/// The current form's moveset (its left trail windows), if it has two Keyblades.
fn form_moveset() -> Option<usize> {
    match super::form::current()? {
        super::form::Kind::Valor => Some(2),
        super::form::Kind::Master => Some(4),
        super::form::Kind::Final => Some(5),
        _ => None,
    }
}

struct Trail {
    effect: usize,
    fading: bool,
    logged: u32,
}
/// Our trails, and whether the left weapon's trail window was open last step.
static TRAILS: Mutex<(Vec<Trail>, bool)> = Mutex::new((Vec::new(), false));

/// KH2's bind pose of the Valor skeleton (for the hand offsets), made once.
static KH2_BIND: Mutex<Option<Vec<Mat4>>> = Mutex::new(None);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

fn readable(at: usize, len: usize) -> bool {
    at >= 0x10000 && unsafe { super::crash::readable(at) && super::crash::readable(at + len.max(1) - 1) }
}

/// Logs `line` the first `max` times its counter is bumped.
fn log_n(counter: &AtomicU32, max: u32, line: impl FnOnce() -> String) {
    if counter.fetch_add(1, Ordering::Relaxed) < max {
        log(&format!("left Keyblade: {}", line()));
    }
}

/// The model queue's hook is in (KH2's player model, `model::hide`, uses it too).
static QUEUE_HOOKED: AtomicBool = AtomicBool::new(false);

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    // KH1's .wpn release handler, pointed at ours (data: the pointer itself is checked).
    let slot = base + WPN_RELEASE_SLOT_RVA;
    let want = ((base + WPN_RELEASE_RVA) as u64).to_le_bytes();
    let ours = (wpn_release as *const () as usize as u64).to_le_bytes();
    let left = super::settings::get().valor_left_blade;
    let patched = if !left {
        log("left Keyblade: off (valor_left_blade = off)");
        false
    } else if !super::crash::readable(slot) || !super::hook::patch(slot, &want, &ours, "the .wpn release handler (left Keyblade)") {
        log("left Keyblade: KH1's .wpn release handler not where expected: not installed");
        false
    } else {
        true
    };
    // The queue's hook goes in either way: KH2's player model holds back KH1's Sora through it.
    if super::hook::install(base + QUEUE_RVA, &QUEUE_PROLOGUE, queue as *const () as usize, &ORIG_QUEUE, "model queue (left Keyblade, KH2 model)").is_none() {
        // (Put KH1's handler back: nothing of ours will be loaded.)
        if patched {
            super::hook::patch(slot, &ours, &want, "the .wpn release handler (back)");
        }
        return;
    }
    QUEUE_HOOKED.store(true, Ordering::Relaxed);
    if !patched {
        return;
    }
    INSTALLED.store(true, Ordering::Relaxed);
    // The trail (fails alone: the Keyblade is still drawn).
    if super::hook::install(base + TRAIL_START_RVA, &TRAIL_START_PROLOGUE, trail_start as *const () as usize, &ORIG_TRAIL_START, "Keyblade trail request (left Keyblade)").is_some() {
        TRAIL_HOOKED.store(true, Ordering::Relaxed);
        std::thread::spawn(load_trail_windows);
    } else {
        log("left Keyblade: trail off (KH1's trail request not where expected)");
    }
}

// ---------------------------------------------------------------------------------------------
// Loading

/// A Sora weapon's row (KH1 item id) in `btltbl.bin`'s weapon table: its `.wpn` stem first, its
/// blade's range at `ROW_BLADE` (what his weapon object points at, `0x140284D40`).
unsafe fn weapon_row(base: usize, item: u8) -> Option<usize> {
    if item == 0 {
        return None;
    }
    let table = rd::<u64>(base + ITEM_TABLE_PTR_RVA) as usize;
    let rec = table.checked_add((item as usize - 1) * 0x14)?;
    if !readable(rec, 0x14) || rd::<u8>(rec + 2) != 2 {
        return None;
    }
    let row = rd::<i16>(rec + 6);
    if row <= 0 {
        return None;
    }
    let off = rd::<i32>(base + WEAPON_TABLE_OFFSETS_RVA);
    if off <= 0 {
        return None;
    }
    let at = (base + BTLTBL_RVA).checked_add(off as usize)? + (row as usize - 1) * WEAPON_ROW_SIZE;
    readable(at, WEAPON_ROW_SIZE).then_some(at)
}

/// The `.wpn` file name of a Sora weapon (KH1 item id), from `btltbl.bin`'s weapon table.
unsafe fn file_name(base: usize, item: u8) -> Option<String> {
    let at = weapon_row(base, item)?;
    let stem: Vec<u8> = std::slice::from_raw_parts(at as *const u8, 16).iter().copied().take_while(|&c| c != 0).collect();
    if stem.is_empty() || !stem.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_') {
        return None;
    }
    Some(format!("{}.wpn", String::from_utf8_lossy(&stem)))
}

/// Valor began (`valor::started`), its Keyblade `item`: drawn once its model is in; loaded now
/// if it isn't (game thread).
pub unsafe fn show(item: Option<u8>) {
    if !INSTALLED.load(Ordering::Relaxed) {
        return;
    }
    SHOWN.store(true, Ordering::Relaxed);
    LEFT_ITEM.store(item.unwrap_or(0), Ordering::Relaxed);
    let Some(item) = item else {
        MESH.store(0, Ordering::Relaxed);
        return;
    };
    let base = BASE.load(Ordering::Relaxed);
    LEFT_ROW.store(weapon_row(base, item).unwrap_or(0), Ordering::Relaxed);
    let Ok(mut st) = STATE.try_lock() else { return };
    // A load that never answered: given up (its buffer is kept, never reused).
    if let Some((it, buf, since)) = st.pending {
        if since.elapsed().as_secs_f32() > LOAD_TIMEOUT_S {
            log(&format!("left Keyblade: the load of {it:#x} never answered ({:.0} s): given up", since.elapsed().as_secs_f32()));
            st.pending = None;
            st.failed.push(it);
            if PENDING_BUF.load(Ordering::Relaxed) == buf {
                PENDING_BUF.store(0, Ordering::Relaxed);
            }
        }
    }
    if let Some(l) = st.loaded.as_ref() {
        if l.item == item {
            MESH.store(l.mesh, Ordering::Relaxed);
            EFFECTS.store(l.effects, Ordering::Relaxed);
            return;
        }
    }
    if st.pending.is_some_and(|p| p.0 == item) || st.failed.contains(&item) {
        return;
    }
    if st.pending.is_some() {
        // (Another Keyblade's load is under way: asked again at the next Valor.)
        return;
    }
    // Another Keyblade was loaded: released (not drawn from now on).
    if let Some(old) = st.loaded.take() {
        MESH.store(0, Ordering::Relaxed);
        EFFECTS.store(0, Ordering::Relaxed);
        end_trails();
        release(&old);
    }
    let Some(name) = file_name(base, item) else {
        log(&format!("left Keyblade: no weapon file for item {item:#x}"));
        st.failed.push(item);
        return;
    };
    let buf = super::sys::VirtualAlloc(std::ptr::null_mut(), BUF_SIZE, 0x3000, 0x04) as usize;
    if buf == 0 {
        log("left Keyblade: no memory for the model");
        st.failed.push(item);
        return;
    }
    st.bufs.push(buf);
    st.pending = Some((item, buf, Instant::now()));
    PENDING_BUF.store(buf, Ordering::Relaxed);
    drop(st);
    let mut cname = name.clone().into_bytes();
    cname.push(0);
    let load: unsafe extern "system" fn(*const u8, usize, usize, u32) -> u32 = std::mem::transmute(base + LOAD_RVA);
    let id = load(cname.as_ptr(), buf, loaded as *const () as usize, 0);
    log(&format!("left Keyblade: load of {name} (item {item:#x}) requested into {buf:#x} (request {id})"));
}

/// Back from Valor (`valor::ended`): not drawn; the model stays loaded.
pub fn hide() {
    SHOWN.store(false, Ordering::Relaxed);
}

/// KH1's loader is done with our file (game thread, KH1's loader task).
unsafe extern "system" fn loaded(size: i32, id: u32, buf: usize) {
    static N: AtomicU32 = AtomicU32::new(0);
    log_n(&N, 20, || format!("load answered: request {id}, {size} bytes into {buf:#x}"));
    if buf == 0 || PENDING_BUF.load(Ordering::Relaxed) != buf {
        log_n(&N, 20, || "(not the load we wait for: ignored)".into());
        return;
    }
    PENDING_BUF.store(0, Ordering::Relaxed);
    let Ok(mut st) = STATE.try_lock() else {
        log("left Keyblade: busy when the load answered: model not used");
        return;
    };
    let Some((item, _, _)) = st.pending.take() else { return };
    match set_up(size, buf) {
        Ok((container, mesh, effects)) => {
            st.loaded = Some(Loaded { item, buf, container, mesh, effects });
            if SHOWN.load(Ordering::Relaxed) {
                MESH.store(mesh, Ordering::Relaxed);
                EFFECTS.store(effects, Ordering::Relaxed);
            }
        }
        Err(e) => {
            log(&format!("left Keyblade: item {item:#x}'s model not used: {e}"));
            st.failed.push(item);
        }
    }
}

/// The `.wpn` in `buf`: its model container set up by KH1; returns it, its mesh model and its
/// effect data (set up too; 0 if none).
unsafe fn set_up(size: i32, buf: usize) -> Result<(usize, usize, usize), String> {
    if size <= 0x10 || size as usize > BUF_SIZE {
        return Err(format!("size {size}"));
    }
    let size = size as usize;
    let (o4, o8, oc) = (rd::<u32>(buf + 4) as usize, rd::<u32>(buf + 8) as usize, rd::<u32>(buf + 0xC) as usize);
    // (`+0xC` may run past the bytes loaded: the user's Divine Rose said 0x1F700 of 0x1F690, its
    // end padding. KH1's own `.wpn` handler `0x140285120` reads only `+4` and `+8`.)
    let _ = oc;
    if !(o4 <= o8 && o4 < size && o8 % 4 == 0 && o8 + 0x30 <= size) {
        return Err(format!("header {o4:#x} {o8:#x} {oc:#x} (size {size:#x})"));
    }
    let container = buf + o8;
    let magic = rd::<u32>(container);
    if magic != MENV {
        return Err(format!("container magic {magic:#010x}, not MENV"));
    }
    let setup: unsafe extern "system" fn(usize) = std::mem::transmute(BASE.load(Ordering::Relaxed) + SETUP_RVA);
    setup(container);
    let mesh = super::pose::ptr(rd::<u32>(container + 0x20));
    if !readable(mesh, 0x20) {
        return Err(format!("no mesh model after setup ({mesh:#x})"));
    }
    let bones = rd::<i16>(mesh);
    if bones < 1 || bones as usize > WEAPON_BONES {
        return Err(format!("{bones} bones (at most {WEAPON_BONES} fit Sora's weapon bones)"));
    }
    let hd = rd::<u32>(mesh + 0x10) == HD_MARK && rd::<u32>(mesh + 0x14) != 0;
    log(&format!(
        "left Keyblade: model set up: container {container:#x}, mesh {mesh:#x}, {bones} bones, {} meshes, {}",
        rd::<i32>(mesh + 0xC),
        if hd { "remastered (HD) table hooked up" } else { "no remastered table (PS2 mesh)" }
    ));
    // Its effect data (`+4` up to the model), as KH1's handler sets up its weapon's: the trail's
    // look is the Keyblade's own (the user: "it doesnt play keyblade specific trails for the
    // second keyblade").
    let effects = if o8 > o4 && o4 >= 0x10 && readable(buf + o4, 0x14) {
        let at = buf + o4;
        let n = rd::<i32>(at + 0xC);
        if (0..=256).contains(&n) && o4 + 0x10 + n as usize * 0x20 <= o8 {
            let setup: unsafe extern "system" fn(usize) = std::mem::transmute(BASE.load(Ordering::Relaxed) + EFFECTS_SETUP_RVA);
            setup(at);
            log(&format!("left Keyblade: its effect data set up at {at:#x} ({n} entries)"));
            at
        } else {
            log(&format!("left Keyblade: effect data at {o4:#x} looks wrong ({n} entries): the right Keyblade's trail look used"));
            0
        }
    } else {
        0
    };
    Ok((container, mesh, effects))
}

/// Releases a loaded model as KH1 releases a `.wpn` (its model, then its file record with the
/// textures, deferred by KH1), with the record's handler row cleared so KH1's handler isn't
/// called on it. The buffer is kept.
unsafe fn release(old: &Loaded) {
    let base = BASE.load(Ordering::Relaxed);
    let find: unsafe extern "system" fn(usize, i32) -> usize = std::mem::transmute(base + FIND_RECORD_RVA);
    let record = find(old.buf, 0);
    if record != 0 && readable(record, REC_BUFFER + 8) && rd::<u64>(record + REC_BUFFER) as usize == old.buf {
        std::ptr::write_unaligned((record + REC_ROW) as *mut i32, -1);
        if old.effects != 0 {
            let free_fx: unsafe extern "system" fn(usize) = std::mem::transmute(base + EFFECTS_RELEASE_RVA);
            free_fx(old.effects);
        }
        let free_model: unsafe extern "system" fn(usize) = std::mem::transmute(base + RELEASE_MODEL_RVA);
        free_model(old.container);
        let drop_record: unsafe extern "system" fn(usize, u8) = std::mem::transmute(base + DROP_RECORD_RVA);
        drop_record(record, 1);
        log(&format!("left Keyblade: item {:#x}'s model released (record {record:#x})", old.item));
    } else {
        log(&format!("left Keyblade: item {:#x}'s file record not found ({record:#x}): left loaded", old.item));
    }
}

/// KH1's `.wpn` release handler (`record`), but never for a buffer of ours (it would look our
/// buffer up among the three party slots, find none and release garbage).
unsafe extern "system" fn wpn_release(record: usize) -> u64 {
    let buf = if readable(record, REC_BUFFER + 8) { rd::<u64>(record + REC_BUFFER) as usize } else { 0 };
    let ours = STATE.try_lock().map(|s| s.bufs.contains(&buf)).unwrap_or(false) || buf == PENDING_BUF.load(Ordering::Relaxed);
    if ours {
        static N: AtomicU32 = AtomicU32::new(0);
        log_n(&N, 10, || format!("KH1 released our file record {record:#x} (buffer {buf:#x}): not drawn from now on"));
        MESH.store(0, Ordering::Relaxed);
        EFFECTS.store(0, Ordering::Relaxed);
        end_trails();
        if let Ok(mut st) = STATE.try_lock() {
            if st.loaded.as_ref().is_some_and(|l| l.buf == buf) {
                st.loaded = None;
            }
        }
        return 0;
    }
    let f: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(BASE.load(Ordering::Relaxed) + WPN_RELEASE_RVA);
    f(record)
}

// ---------------------------------------------------------------------------------------------
// Posing

/// `takeover`, each KH2 step. Gets:
/// - KH2's left Keyblade this frame (the rig's second weapon), if there is one;
/// - the clip playing (`motion`, `frame`);
/// - whether any of the clip's trail windows is open (`trail`, used if the left weapon's own
///   ranges couldn't be read);
/// - whether he is in a move (`acting`: not standing, walking or running). Master's floating
///   Keyblade has a trail window all through its idle, walk and run (`W_EX010_TRIF_L` 0..2) as
///   KH2 draws its float; KH1's swing trail there would hang on him all the time, so the left
///   trail is for moves only, as the right one is (its attack windows).
pub fn kh2_left(bones: Option<&[Mat4]>, body: &[Mat4], motion: u16, frame: u32, trail: bool, acting: bool) {
    super::model::note_left(bones);
    if !INSTALLED.load(Ordering::Relaxed) {
        return;
    }
    let valor = super::form::two_blades() && SHOWN.load(Ordering::Relaxed) && MESH.load(Ordering::Relaxed) != 0;
    if valor {
        if let (Ok(mut g), Some(b)) = (KH2_LEFT.try_lock(), bones.filter(|b| !b.is_empty())) {
            g.push_back((body_sig(body), b.to_vec(), Instant::now()));
            while g.len() > 6 {
                g.pop_front();
            }
        }
    }
    unsafe { trail_step(valor, motion, frame, trail, acting) };
}

/// Base Sora's Keyblade root relative to his KH2 right hand bone in his idle (held), once.
static HELD: std::sync::OnceLock<Option<Mat4>> = std::sync::OnceLock::new();

/// Final Form's right Keyblade (`pose`, before the weapon bones are drawn): KH1's weapon bones
/// (`kh1`, model space) moved from his hand to where KH2's own weapon motion floats it (the
/// user: Final Form, its Keyblades floating as KH2's). The turn from KH2's held blade (base
/// Sora's idle, at his hand; its blade bone, `KH2_BLADE`) to KH1's grip as drawn is kept, and put
/// on Final's blade now, its spin included; the left one then follows by the same turn
/// (`note_pose`). `kh2_body`, `kh2_right`: KH2's body and right Keyblade of the drawn frame (model
/// space); `ratio`: KH1's size against KH2's; `to_world` KH1's placement (the blade as drawn is
/// noted for the effects on KH2's weapon anchors, `final_blade`).
pub fn final_float(kh1: &mut [Mat4], kh1_bind: &[Mat4], kh2_body: &[Mat4], kh2_right: &[Mat4], ratio: f32, to_world: &Mat4) {
    static N: AtomicU32 = AtomicU32::new(0);
    if !super::form::in_final() || kh1.len() < SORA_BONES || kh1_bind.len() < SORA_BONES || kh2_right.is_empty() {
        return;
    }
    let Some(kh2) = super::pose::kh2() else { return };
    let Some(form) = kh2.final_form.as_ref() else { return };
    let Some(sr) = form.rig.weapons.first().and_then(|w| w.bone) else { return };
    let held = *HELD.get_or_init(|| {
        let w = kh2.rig.weapons.first()?;
        let m = w.motions.get(&character_core::motion::IDLE).or_else(|| w.motions.get(&0))?;
        let blade = kh2_blade(&kh2_anim::evaluate(&w.skeleton, m, 0.0, kh2_anim::Options::default()).bones)?;
        log(&format!("Final Form: base Sora's held Keyblade blade at {:.1?} from his hand bone", blade.w_axis.truncate()));
        Some(blade)
    });
    let Some(held) = held else { return };
    if kh2_body.len() <= sr {
        return;
    }
    let b2 = kh2_anim::Pose::bind(&form.rig.skeleton).bones;
    if b2.len() <= sr {
        return;
    }
    let k = if ratio.is_finite() && ratio > 0.1 { ratio } else { 1.0 };
    let offset = kh1_bind[RIGHT_HAND].inverse() * scaled(b2[sr], k);
    let held_root = kh1[RIGHT_HAND] * offset * scaled(held, k);
    let turn = held_root.inverse() * kh1[GRIP];
    let Some(right) = kh2_blade(kh2_right) else { return };
    let now_root = kh1[RIGHT_HAND] * offset * scaled(kh2_body[sr].inverse() * right, k);
    let grip = now_root * turn;
    if !grip.is_finite() {
        return;
    }
    note_final_blade(false, *to_world * unit(now_root));
    let inv = kh1[GRIP].inverse();
    let moved: Vec<Mat4> = (0..WEAPON_BONES).map(|i| grip * inv * kh1[GRIP + i]).collect();
    kh1[GRIP..GRIP + WEAPON_BONES].copy_from_slice(&moved);
    log_n(&N, 3, || format!("Final Form: right Keyblade floated {:.0} units from his hand", (grip.w_axis - kh1[RIGHT_HAND].w_axis).truncate().length()));
}

/// KH2's Keyblade blade bone (`KH2_BLADE`) where KH1's drawn grip holds it, in KH1's model space:
/// base Sora's held blade (his idle, relative to his hand bone, `HELD`) put on KH1's right hand by
/// the bind poses' relation, as `final_float` does. KH1 holds its Keyblade rigidly in the hand,
/// so this is the blade as drawn (Limit Form's weapon anchors, `limits`). `ratio`: KH1's size
/// against KH2's.
pub fn held_blade(kh1: &[Mat4], kh1_bind: &[Mat4], ratio: f32) -> Option<Mat4> {
    if kh1.len() < SORA_BONES || kh1_bind.len() < SORA_BONES {
        return None;
    }
    let kh2 = super::pose::kh2()?;
    let w = kh2.rig.weapons.first()?;
    let sr = w.bone?;
    let held = (*HELD.get_or_init(|| {
        let m = w.motions.get(&character_core::motion::IDLE).or_else(|| w.motions.get(&0))?;
        let blade = kh2_blade(&kh2_anim::evaluate(&w.skeleton, m, 0.0, kh2_anim::Options::default()).bones)?;
        log(&format!("Final Form: base Sora's held Keyblade blade at {:.1?} from his hand bone", blade.w_axis.truncate()));
        Some(blade)
    }))?;
    static BIND: std::sync::OnceLock<Option<Mat4>> = std::sync::OnceLock::new();
    let hand = (*BIND.get_or_init(|| kh2_anim::Pose::bind(&kh2.rig.skeleton).bones.get(sr).copied()))?;
    let k = if ratio.is_finite() && ratio > 0.1 { ratio } else { 1.0 };
    let offset = kh1_bind[RIGHT_HAND].inverse() * scaled(hand, k);
    let m = kh1[RIGHT_HAND] * offset * scaled(held, k);
    m.is_finite().then_some(m)
}

/// KH1's weapon bones (the grip and its chain, `kh1`, model space) moved together so the KH2
/// blade bone they stand for (as `held_blade` relates them) is at `blade` (KH1's model space):
/// a Keyblade out of the hand (Limit Form's Strike Raid, `limits`). Returns whether they moved.
pub fn float_grip(kh1: &mut [Mat4], kh1_bind: &[Mat4], blade: Mat4, ratio: f32) -> bool {
    let Some(held) = held_blade(kh1, kh1_bind, ratio) else { return false };
    let turn = held.inverse() * kh1[GRIP];
    let grip = blade * turn;
    if !grip.is_finite() {
        return false;
    }
    let inv = kh1[GRIP].inverse();
    let moved: Vec<Mat4> = (0..WEAPON_BONES).map(|i| grip * inv * kh1[GRIP + i]).collect();
    kh1[GRIP..GRIP + WEAPON_BONES].copy_from_slice(&moved);
    true
}

/// A matrix's turn and place without any scale (its axes made unit and square to each other).
pub fn unit(m: Mat4) -> Mat4 {
    let x = m.x_axis.truncate().normalize_or_zero();
    let y = (m.y_axis.truncate() - x * x.dot(m.y_axis.truncate())).normalize_or_zero();
    let z = x.cross(y);
    let z = if z.dot(m.z_axis.truncate()) < 0.0 { -z } else { z };
    Mat4::from_cols(x.extend(0.0), y.extend(0.0), z.extend(0.0), m.w_axis)
}

/// A matrix with its translation times `k` (KH2 units to KH1's).
pub fn scaled(m: Mat4, k: f32) -> Mat4 {
    let mut m = m;
    m.w_axis = (m.w_axis.truncate() * k).extend(1.0);
    m
}

/// `pose`, after Sora's KH1 bones are drawn from KH2's pose: the left hand's weapon bones.
///
/// - `kh1`: KH1's bones as drawn (KH1 model space), `kh1_bind` KH1's bind; `kh2_body` and
///   `kh2_right` KH2's body and right Keyblade (KH2 model space); `to_world` KH1's placement;
///   `ratio` KH1's size against KH2's (`Retarget::hip_ratio`).
/// - KH2's two Keyblades are one model on two hand bones (`sklt`): each weapon root relative to
///   its hand bone is known for both. Each KH2 hand bone is put on KH1's hand by the bind poses'
///   relation (both T-poses facing +z, same axes). The turn from KH2's right weapon root to KH1's
///   grip (280, as `pose` drew it: KH1's grip, `flip_weapons`, teeth up) is then the same for the
///   left, and the chain keeps the right one's shape.
/// - Final Form's floating left Keyblade is put on KH1's right hand instead, by KH2's right hand
///   bone, as the right one is: the two keep KH2's spacing.
pub fn note_pose(kh1: &[Mat4], kh1_bind: &[Mat4], kh2_body: &[Mat4], kh2_right: &[Mat4], to_world: &Mat4, ratio: f32) {
    if !INSTALLED.load(Ordering::Relaxed) || !SHOWN.load(Ordering::Relaxed) || !super::form::two_blades() || MESH.load(Ordering::Relaxed) == 0 {
        return;
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if let Ok(mut w) = TO_WORLD.try_lock() {
        *w = Some(*to_world);
    }
    // Which KH2 weapon bone the two are matched by: the root (Valor's, held), or in Final and
    // Master Form the blade, whose turn its weapon motions carry (Final's right one was floated by
    // it, `final_float`; Master's right one is held, 6..9 units from the hand in every motion).
    let floats = super::form::floating_blade();
    let master = super::form::in_master();
    let pick = |w: &[Mat4]| if floats { kh2_blade(w) } else { w.first().copied() };
    // KH2's left Keyblade of the step whose body this is (else the latest).
    let Some(left) = KH2_LEFT.try_lock().ok().and_then(|g| {
        let fresh = |t: &Instant| t.elapsed().as_secs_f32() < 0.15;
        let sig = body_sig(kh2_body);
        g.iter().rev().find(|(r, _, t)| *r == sig && fresh(t)).or_else(|| g.back().filter(|(_, _, t)| fresh(t))).and_then(|(_, b, _)| pick(b))
    }) else { return };
    let Some(right) = pick(kh2_right) else { return };
    // (The form's own rig: the same skeleton and hand bones in every form, 178 and 86.)
    let Some(rig) = super::pose::kh2().and_then(|k| super::form::current().and_then(|f| f.data(k)).or(k.valor.as_ref())).map(|f| f.rig.clone()) else { return };
    let (Some(sr), Some(sl)) = (rig.weapons.first().and_then(|w| w.bone), rig.weapons.get(1).and_then(|w| w.bone)) else {
        log_n(&N, 5, || "the form's rig has no second weapon bone".into());
        return;
    };
    if kh1.len() < SORA_BONES || kh1_bind.len() < SORA_BONES || kh2_body.len() <= sr.max(sl) || kh2_right.is_empty() {
        return;
    }
    let Ok(mut bind) = KH2_BIND.try_lock() else { return };
    let b2 = bind.get_or_insert_with(|| kh2_anim::Pose::bind(&rig.skeleton).bones);
    if b2.len() <= sr.max(sl) {
        return;
    }
    let k = if ratio.is_finite() && ratio > 0.1 { ratio } else { 1.0 };
    // KH2's hand bone in KH1's hand frame (bind poses).
    let offset = |kh1_hand: usize, kh2_hand: usize| kh1_bind[kh1_hand].inverse() * scaled(b2[kh2_hand], k);
    // Weapon root relative to its KH2 hand bone now.
    let rel = |kh2_hand: usize, root: Mat4| scaled(kh2_body[kh2_hand].inverse() * root, k);
    let right_root = kh1[RIGHT_HAND] * offset(RIGHT_HAND, sr) * rel(sr, right);
    let turn = right_root.inverse() * kh1[GRIP];
    // Final Form's float is put where KH2 has it from the right Keyblade's hand, as the right one
    // is (`final_float`), not from KH1's left hand: so the two keep KH2's own spacing and the left
    // is as steady as the right (the user: "when sora hovers on the ground, the second keyblade
    // follows him but it jitters a little. also, when gliding and turning, the second keyblade
    // goes to the same position as the first keyblade"). KH1's hands are rebuilt from KH2's
    // bones, not copied, so each turns a little differently from KH2's; a Keyblade 40 to 120
    // units from its hand swings by that, and the left hand's wobble showed as a jitter. In a
    // banked glide turn (206, 207) KH2 puts both on one side of him, about 35 units
    // apart, the left one 115 units from its hand across his body: the two hands' differences
    // there came to the 35 units, and the left landed on the right.
    //
    // Master Form's second Keyblade floats by his left hand, "held by magic" (KH2's
    // `W_EX010_TRIF_L`: 22..123 units from the left hand while it floats, 8 when he holds it in
    // the jump, the fall, the landing, the run and Aerial Dodge; the right one is always in his
    // hand). Floating, it hangs from the right hand as Final's does (s282: the left hand's wobble
    // over a long float); held, from the left hand, so it stays in his grip; in between, the two
    // blended by how far KH2 has it from its left hand (`MASTER_HELD`, `MASTER_FLOAT`).
    let via_right = || kh1[RIGHT_HAND] * offset(RIGHT_HAND, sr) * rel(sr, left);
    let via_left = || kh1[LEFT_HAND] * offset(LEFT_HAND, sl) * rel(sl, left);
    let (hand, left_blade) = if super::form::in_final() {
        (RIGHT_HAND, via_right())
    } else if master {
        let d = (kh2_body[sl].inverse() * left).w_axis.truncate().length();
        let w = ((d - MASTER_HELD) / (MASTER_FLOAT - MASTER_HELD)).clamp(0.0, 1.0);
        let w = w * w * (3.0 - 2.0 * w);
        let m = if w <= 0.0 {
            via_left()
        } else if w >= 1.0 {
            via_right()
        } else {
            blend(via_left(), via_right(), w)
        };
        (if w < 0.5 { LEFT_HAND } else { RIGHT_HAND }, m)
    } else {
        (LEFT_HAND, via_left())
    };
    let grip = left_blade * turn;
    let from_hand = (grip.w_axis - kh1[hand].w_axis).truncate().length();
    // (Final's and Master's float away from his hand.)
    let reach = if floats { 400.0 } else { 40.0 };
    if !grip.is_finite() || from_hand > reach {
        log_n(&N, 5, || format!("left grip {from_hand:.0} units from the hand it hangs from: not drawn this frame"));
        return;
    }
    let inv_grip = kh1[GRIP].inverse();
    let mut out = [Mat4::IDENTITY; WEAPON_BONES];
    if let Some(held) = dream_left(kh1, kh1_bind, grip) {
        out = held;
    } else {
        for (i, m) in out.iter_mut().enumerate() {
            *m = grip * inv_grip * kh1[GRIP + i];
        }
    }
    log_n(&N, 3, || format!("left grip posed, {from_hand:.1} units from the {} hand", if hand == RIGHT_HAND { "right" } else { "left" }));
    // (Final's and Master's blade bones as drawn, for the effects on their weapon anchors; Master's
    // right one is KH2's on KH1's right hand, where KH1's held grip is.)
    if floats {
        note_final_blade(true, *to_world * unit(left_blade));
    }
    if master {
        note_final_blade(false, *to_world * unit(right_root));
    }
    if let Ok(mut g) = LEFT.try_lock() {
        *g = Some((out, super::takeover::GAME_FRAME.load(Ordering::Relaxed)));
    }
    // The trail's owner from this pose at once: Sora's world matrices were just written from it
    // (`pose`), and KH1's own trail reads his from there. Filled only at the left Keyblade's draw
    // (as before), the owner could be a frame behind his own whenever KH1's trail follower ran
    // between the pose and the draw, so a fast floating blade's ribbon trailed it.
    if TRAIL_HOOKED.load(Ordering::Relaxed) {
        // (Safe here: the pose pass runs on the game thread, as the draw does.)
        unsafe {
            fill_owner();
        }
    }
}

/// KH1 items held rigidly, not as a Keyblade (the dream weapons and the wooden sword,
/// `pose::dream_weapon`); of them only the Dream Sword can be in Valor's left hand.
const DREAM_WEAPONS: std::ops::RangeInclusive<u8> = 0x52..=0x55;

/// The left hand's weapon bones (KH1 model space) when the left weapon isn't a Keyblade (Valor's
/// default, the Dream Sword), from `grip`, where the right Keyblade's grip is put in the left hand.
/// The right hand holds such a weapon rigidly, the way KH1 holds it (`pose::dream_hold`: its grip
/// from the hand and every weapon bone from the grip, noted from KH1's own pose; no Keyblade
/// half-turn, no keychain swing): the left hand the same way.
/// - KH1's hold noted this session (Sora held it in the Dive to the Heart): the turn from the
///   right Keyblade's grip as drawn to where that hold puts the weapon in the right hand is put on
///   the left grip, and the bones follow the hold.
/// - Else (the usual case: the Dive is long past): the Keyblade's grip, every bone after it as in
///   KH1's bind (the mesh as it was made, nothing swinging it apart). The Dream Sword's weapon row
///   gives the trail's blade range along the same bone with the same sign as a Keyblade's (-4.3..0
///   against the Kingdom Key's -3.7..0), so its blade should lie where a Keyblade's does (its roll
///   against KH1's own Dive hold is unchecked: the log names the source).
fn dream_left(kh1: &[Mat4], kh1_bind: &[Mat4], grip: Mat4) -> Option<[Mat4; WEAPON_BONES]> {
    let item = LEFT_ITEM.load(Ordering::Relaxed);
    if !DREAM_WEAPONS.contains(&item) || kh1.len() < GRIP + WEAPON_BONES || kh1_bind.len() < GRIP + WEAPON_BONES {
        return None;
    }
    let mut out = [Mat4::IDENTITY; WEAPON_BONES];
    let (how, source) = match super::pose::dream_hold_of(item).filter(|r| r.len() >= WEAPON_BONES) {
        Some(rel) => {
            let g = grip * (kh1[GRIP].inverse() * kh1[RIGHT_HAND] * rel[0]);
            out[0] = g;
            for i in 1..WEAPON_BONES {
                out[i] = g * rel[i];
            }
            (1, "KH1's own hold of it (noted this session)")
        }
        None => {
            let inv = kh1_bind[GRIP].inverse();
            out[0] = grip;
            for i in 1..WEAPON_BONES {
                out[i] = grip * inv * kh1_bind[GRIP + i];
            }
            (2, "the Keyblade's grip, its bones as in KH1's bind (no hold of KH1's noted this session)")
        }
    };
    if !out.iter().all(|m| m.is_finite()) {
        return None;
    }
    static LOGGED: AtomicU32 = AtomicU32::new(0);
    let key = (item as u32) << 8 | how;
    if LOGGED.swap(key, Ordering::Relaxed) != key {
        log(&format!("left Keyblade: item {item:#x} isn't a Keyblade: held rigidly, by {source}"));
    }
    Some(out)
}

/// Master Form's second Keyblade: KH2's blade this close to its left hand or closer is held there
/// (8 in its held motions), this far or farther floats (22 at its nearest float). KH2 units.
const MASTER_HELD: f32 = 12.0;
const MASTER_FLOAT: f32 = 22.0;

/// `a` toward `b` by `t`: place, turn and scale each (the hands' frames can be scaled). Two
/// frames of opposite handedness (a mirrored hand) aren't blended: the nearer is taken.
fn blend(a: Mat4, b: Mat4, t: f32) -> Mat4 {
    let det = |m: &Mat4| glam::Mat3::from_mat4(*m).determinant();
    if det(&a) * det(&b) <= 0.0 {
        return if t < 0.5 { a } else { b };
    }
    let (sa, ra, pa) = a.to_scale_rotation_translation();
    let (sb, rb, pb) = b.to_scale_rotation_translation();
    let m = Mat4::from_scale_rotation_translation(sa.lerp(sb, t), ra.slerp(rb, t), pa.lerp(pb, t));
    if m.is_finite() {
        m
    } else if t < 0.5 {
        a
    } else {
        b
    }
}

// ---------------------------------------------------------------------------------------------
// Drawing

type Queue = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;

unsafe extern "system" fn queue(instance: usize, view: usize, params: usize, actor: usize) -> usize {
    let f: Queue = std::mem::transmute(ORIG_QUEUE.load(Ordering::Acquire));
    // KH2's player model: Sora's own draw (the field's, or the pause menu's copy) with his
    // meshes held back, and his Keyblade shrunk into his grip when KH2's replaces it.
    let first = !IN_SECOND.load(Ordering::Relaxed) && readable(params, P_SIZE);
    let ours = if first && actor != 0 && actor == super::takeover::sora_actor() && instance == super::pose::SORA_INSTANCE.load(Ordering::Relaxed) {
        super::model::hide(params)
    } else if first && super::model::is_menu_copy(instance) {
        super::model::hide_menu(instance, params)
    } else {
        None
    };
    // (Anti Form holds no Keyblade, as KH2's: KH1's shrunk away on his field draw, KH1's model too.)
    let anti = first && actor != 0 && actor == super::takeover::sora_actor() && instance == super::pose::SORA_INSTANCE.load(Ordering::Relaxed) && super::form::in_anti();
    let collapsed = (ours.as_ref().is_some_and(|h| h.collapse) || anti).then(|| collapse_weapon(instance)).flatten();
    let r = f(instance, view, ours.as_ref().map_or(params, |h| h.params.0.as_ptr() as usize), actor);
    if let Some((at, saved)) = collapsed {
        std::ptr::copy_nonoverlapping(saved.as_ptr(), at as *mut u8, saved.len());
    }
    if !IN_SECOND.load(Ordering::Relaxed) && INSTALLED.load(Ordering::Relaxed) {
        second(f, instance, view, params, actor);
    }
    r
}

/// KH1's Keyblade drawn as a speck in the grip for one queue call (KH2's is drawn in its place):
/// every weapon bone shrunk to a thousandth about the grip. It stays queued, so whatever KH1
/// does with a drawn weapon (its trail, its effects) goes on. Returns where the bones are and
/// their saved bytes, to put back right after the call.
unsafe fn collapse_weapon(instance: usize) -> Option<(usize, [u8; WEAPON_BONES * 0x40])> {
    if !readable(instance + INST_WORLD, 4) {
        return None;
    }
    let world = super::pose::ptr(rd::<u32>(instance + INST_WORLD));
    if !readable(world, SORA_BONES * 0x40) {
        return None;
    }
    let at = world + GRIP * 0x40;
    let mut saved = [0u8; WEAPON_BONES * 0x40];
    std::ptr::copy_nonoverlapping(at as *const u8, saved.as_mut_ptr(), saved.len());
    let grip = Mat4::from_cols_array(&rd::<[f32; 16]>(at));
    let shrink = Mat4::from_translation(grip.w_axis.truncate()) * Mat4::from_scale(glam::Vec3::splat(1e-3)) * Mat4::from_translation(-grip.w_axis.truncate());
    for i in 0..WEAPON_BONES {
        let m = shrink * Mat4::from_cols_array(&rd::<[f32; 16]>(at + i * 0x40));
        if m.is_finite() {
            std::ptr::write_unaligned((at + i * 0x40) as *mut [f32; 16], m.to_cols_array());
        }
    }
    Some((at, saved))
}

/// The left Keyblade: Sora queued again, body hidden, our model in the left hand's bones.
unsafe fn second(f: Queue, instance: usize, view: usize, params: usize, actor: usize) {
    let mesh = MESH.load(Ordering::Relaxed);
    if mesh == 0 || !SHOWN.load(Ordering::Relaxed) || !super::form::two_blades() || actor == 0 || actor != super::takeover::sora_actor() {
        return;
    }
    // (KH2's model of the same Keyblade is drawn in its place.)
    if super::model::left_is_ours() {
        return;
    }
    if instance != super::pose::SORA_INSTANCE.load(Ordering::Relaxed) || !readable(params, P_SIZE) {
        return;
    }
    // KH1 drew his own Keyblade (out, and its model there).
    if rd::<u64>(params + P_WEAPON) == 0 {
        return;
    }
    let Some(left) = left_world() else { return };
    if !readable(instance + INST_WORLD, 4) {
        return;
    }
    let world = super::pose::ptr(rd::<u32>(instance + INST_WORLD));
    if !readable(world, SORA_BONES * 0x40) {
        return;
    }
    let at = world + GRIP * 0x40;
    let mut saved = [0u8; WEAPON_BONES * 0x40];
    std::ptr::copy_nonoverlapping(at as *const u8, saved.as_mut_ptr(), saved.len());
    for (i, m) in left.iter().enumerate() {
        std::ptr::write_unaligned((at + i * 0x40) as *mut [f32; 16], m.to_cols_array());
    }
    // KH1's own params for this draw, but every body mesh hidden and our model as the weapon.
    #[repr(C, align(16))]
    struct Params([u8; 0x100]);
    let mut p = Params([0; 0x100]);
    std::ptr::copy_nonoverlapping(params as *const u8, p.0.as_mut_ptr(), P_SIZE);
    std::ptr::write_unaligned(p.0.as_mut_ptr().add(P_MASK) as *mut u32, 0xFFFF_FFFF);
    std::ptr::write_unaligned(p.0.as_mut_ptr().add(P_WEAPON) as *mut u64, mesh as u64);
    IN_SECOND.store(true, Ordering::Relaxed);
    f(instance, view, p.0.as_ptr() as usize, actor);
    IN_SECOND.store(false, Ordering::Relaxed);
    std::ptr::copy_nonoverlapping(saved.as_ptr(), at as *mut u8, saved.len());
    // The trail's owner from this very frame's matrices (KH1's own trail reads his as drawn; a
    // step later ours ran a frame behind).
    if TRAIL_HOOKED.load(Ordering::Relaxed) {
        fill_owner();
    }
    static N: AtomicU32 = AtomicU32::new(0);
    log_n(&N, 3, || format!("drawn in the left hand (mesh {mesh:#x}, grip at {:.0?})", left[0].w_axis.truncate()));
}

// ---------------------------------------------------------------------------------------------
// Trail

/// KH1's trail request: the id is noted for Sora (KH1's own swings, and `combat`'s for KH2's).
unsafe extern "system" fn trail_start(actor: usize, id: u32) {
    if actor != 0 && actor == super::takeover::sora_actor() {
        TRAIL_ID.store(id, Ordering::Relaxed);
    }
    let f: unsafe extern "system" fn(usize, u32) = std::mem::transmute(ORIG_TRAIL_START.load(Ordering::Acquire));
    f(actor, id);
}

/// KH2's left weapons' trail ranges, from the KH2 install (as `pose::load_kh2` finds it), for each
/// two-Keyblade form.
fn load_trail_windows() {
    let r = (|| -> Result<std::collections::HashMap<usize, Windows>, String> {
        let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
        let install = kh2_data::Install::locate(&exe).map_err(|e| format!("KH2 install: {e}"))?;
        let source = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
        let wmst = install.read_any(source, "03system.bin").ok().and_then(|b| kh2_formats::objentry::parse_wmst(&b).ok());
        let mut all = std::collections::HashMap::new();
        for (moveset, fallback) in TRAIL_MOVESETS {
            // The form's left weapon's motion set name in 03system's `wmst`.
            let name = wmst.as_ref().and_then(|n| n.get(moveset * 2 + 1).filter(|s| !s.is_empty()).cloned()).unwrap_or_else(|| fallback.into());
            let mset = match install.read_any(source, &format!("obj/{name}.mset")).map_err(|e| e.to_string()).and_then(|b| kh2_formats::motion::Mset::parse(&b).map_err(|e| e.to_string())) {
                Ok(m) => m,
                Err(e) => {
                    log(&format!("left Keyblade: trail windows of {name} not read ({e}); KH2's trail windows of either hand used there"));
                    continue;
                }
            };
            let mut out = Windows::new();
            for m in 0..=255u16 {
                let Some(anb) = mset.slot_for(m as usize, true, true).and_then(|s| s.anb.as_ref()) else { continue };
                let w: Vec<(u32, Option<u32>)> = anb
                    .triggers
                    .ranges
                    .iter()
                    .filter(|r| r.id == kh2_formats::motion::range_id::WEAPON_TRAIL)
                    .map(|r| (r.start.max(0) as u32, (r.end >= 0).then_some(r.end as u32)))
                    .collect();
                if !w.is_empty() {
                    out.insert(m, w);
                }
            }
            log(&format!("left Keyblade: trail windows of {name} (moveset {moveset}) for {} motions", out.len()));
            all.insert(moveset, out);
        }
        Ok(all)
    })();
    match r {
        Ok(w) => {
            let _ = TRAIL_WINDOWS.set(w);
        }
        Err(e) => log(&format!("left Keyblade: trail windows not read ({e}); KH2's trail windows of either hand used")),
    }
}

/// The left grip as a proper turn with the right grip's scale, where `left` is: KH1 makes an
/// effect's angles from its matrix's rows, normalized (`0x1402A6D60`), which reads a mirrored or
/// sheared matrix as a wrong turn (the user: the left trail floated off the blade). Mirrored, the
/// axis across the blade the least (the blade: the right grip toward its last bone, `tip`) is
/// the one turned round, so the blade keeps its way.
fn proper_like(left: Mat4, right: Mat4, tip: Mat4) -> Mat4 {
    let m = glam::Mat3::from_mat4(left);
    let cols = [m.x_axis, m.y_axis, m.z_axis];
    let d = right.inverse().transform_point3(tip.w_axis.truncate());
    let d = [d.x.abs(), d.y.abs(), d.z.abs()];
    let a = (0..3).max_by(|&i, &j| d[i].total_cmp(&d[j])).unwrap_or(0);
    // Of the other two, the one along the blade more is kept; the last is made from them.
    let (b, c) = {
        let o: Vec<usize> = (0..3).filter(|&i| i != a).collect();
        if d[o[0]] >= d[o[1]] { (o[0], o[1]) } else { (o[1], o[0]) }
    };
    let ca = cols[a].normalize_or_zero();
    let cb = (cols[b] - ca * ca.dot(cols[b])).normalize_or_zero();
    let cyclic = matches!((a, b, c), (0, 1, 2) | (1, 2, 0) | (2, 0, 1));
    let cc = if cyclic { ca.cross(cb) } else { cb.cross(ca) };
    let r = glam::Mat3::from_mat4(right);
    let scale = [r.x_axis.length(), r.y_axis.length(), r.z_axis.length()];
    let mut out = [glam::Vec3::ZERO; 3];
    out[a] = ca * scale[a];
    out[b] = cb * scale[b];
    out[c] = cc * scale[c];
    Mat4::from_cols(out[0].extend(0.0), out[1].extend(0.0), out[2].extend(0.0), left.w_axis)
}

/// Each KH2 step: our trail owner, a copy of Sora's actor and model instance whose world
/// matrices are his as KH1 last drew him (the ones its own trail reads, his right one), each
/// moved by the turn from his right grip to the left one as drawn (`LEFT`, the same frame): the
/// left Keyblade's bones exactly, and whichever bone its trail reads beside them (the grip's
/// parent, his hand) where it sits for the left. (s220 made them from our pose's matrices at
/// draw time: the user saw the trail well off the blade.)
unsafe fn fill_owner() -> bool {
    static N: AtomicU32 = AtomicU32::new(0);
    let base = BASE.load(Ordering::Relaxed);
    let sora = super::takeover::sora_actor();
    let inst = super::pose::SORA_INSTANCE.load(Ordering::Relaxed);
    if !readable(sora, ACTOR_SIZE) || !readable(inst, INSTANCE_SIZE) {
        return false;
    }
    let world = super::pose::ptr(rd::<u32>(inst + INST_WORLD));
    if !readable(world, SORA_BONES * 0x40) {
        return false;
    }
    let Some(left) = left_world() else { return false };
    let mat = |i: usize| Mat4::from_cols_array(&rd::<[f32; 16]>(world + i * 0x40));
    let right = mat(GRIP);
    let grip = proper_like(left[0], right, mat(GRIP + WEAPON_BONES - 1));
    let to_left = grip * right.inverse();
    if !to_left.is_finite() {
        return false;
    }
    let mut fake = FAKE.load(Ordering::Relaxed);
    if fake == 0 {
        fake = super::sys::VirtualAlloc(std::ptr::null_mut(), FAKE_SIZE, 0x3000, 0x04) as usize;
        if fake == 0 {
            log_n(&N, 3, || "trail: no memory for its owner".into());
            return false;
        }
        let to_handle: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(base + TO_HANDLE_RVA);
        let (ih, mh, wh) = (to_handle(fake + FAKE_INSTANCE), to_handle(fake + FAKE_MATS), to_handle(fake + FAKE_WEAPON));
        if super::pose::ptr(ih) != fake + FAKE_INSTANCE || super::pose::ptr(mh) != fake + FAKE_MATS || super::pose::ptr(wh) != fake + FAKE_WEAPON {
            log_n(&N, 3, || format!("trail: handles of its owner don't resolve ({ih:#x} {mh:#x} {wh:#x}): no trail"));
            TRAIL_HOOKED.store(false, Ordering::Relaxed);
            return false;
        }
        FAKE_INSTANCE_H.store(ih, Ordering::Relaxed);
        FAKE_WEAPON_H.store(wh, Ordering::Relaxed);
        FAKE_MATS_H.store(mh, Ordering::Relaxed);
        FAKE.store(fake, Ordering::Relaxed);
        log_n(&N, 3, || format!("trail: its owner made at {fake:#x}"));
    }
    std::ptr::copy_nonoverlapping(sora as *const u8, fake as *mut u8, ACTOR_SIZE);
    std::ptr::copy_nonoverlapping(inst as *const u8, (fake + FAKE_INSTANCE) as *mut u8, INSTANCE_SIZE);
    std::ptr::write_unaligned((fake + ACTOR_INSTANCE) as *mut u32, FAKE_INSTANCE_H.load(Ordering::Relaxed));
    std::ptr::write_unaligned((fake + FAKE_INSTANCE + INST_WORLD) as *mut u32, FAKE_MATS_H.load(Ordering::Relaxed));
    // Its weapon object: Sora's, with the left Keyblade's row (its blade's range) and effect data.
    let obj = super::pose::ptr(rd::<u32>(sora + ACTOR_WEAPON));
    let row = LEFT_ROW.load(Ordering::Relaxed);
    if row != 0 && readable(obj, WEAPON_OBJ_SIZE) {
        std::ptr::copy_nonoverlapping(obj as *const u8, (fake + FAKE_WEAPON) as *mut u8, WEAPON_OBJ_SIZE);
        std::ptr::write_unaligned((fake + FAKE_WEAPON + OBJ_ROW) as *mut u64, row as u64);
        let fx = EFFECTS.load(Ordering::Relaxed);
        if fx != 0 {
            std::ptr::write_unaligned((fake + FAKE_WEAPON + OBJ_EFFECTS) as *mut u64, fx as u64);
        }
        std::ptr::write_unaligned((fake + ACTOR_WEAPON) as *mut u32, FAKE_WEAPON_H.load(Ordering::Relaxed));
    }
    // (The weapon bones too: a rigid copy of his right hand's Keyblade, so the trail is KH1's
    // right one exactly, on the left grip.)
    for i in 0..SORA_BONES {
        let m = to_left * mat(i);
        std::ptr::write_unaligned((fake + FAKE_MATS + i * 0x40) as *mut [f32; 16], m.to_cols_array());
    }
    // Its trail flag (`+0x1CC` 0x800000, which KH1 keeps set while a trail is wanted) follows the
    // left window, not the right one's (in case the effect reads it from its owner).
    let flags = rd::<u32>(fake + ACTOR_TRAIL_FLAGS);
    let flags = if LEFT_OPEN.load(Ordering::Relaxed) { flags | TRAIL_FLAG } else { flags & !TRAIL_FLAG };
    std::ptr::write_unaligned((fake + ACTOR_TRAIL_FLAGS) as *mut u32, flags);
    FAKE_FILLED.store(true, Ordering::Relaxed);
    // (For the log: how far the left grip is from the right, and from his own position.)
    static M: AtomicU32 = AtomicU32::new(0);
    log_n(&M, 4, || {
        let p = rd::<[f32; 3]>(sora + 0x10);
        let d3 = |m: Mat4| glam::Mat3::from_mat4(m).determinant();
        format!(
            "trail owner filled: right grip at {:.0?} (det {:.2}), left grip at {:.0?} (det {:.2}, as used {:.2}), his position {:.0?}",
            right.w_axis.truncate(),
            d3(right),
            left[0].w_axis.truncate(),
            d3(left[0]),
            d3(grip),
            p
        )
    });
    true
}

/// A live trail's numbers for the log: its follow block (`+0x30` of its follower task is KH1's;
/// here the effect's own position, `*effect` → its point) against the left grip.
unsafe fn trail_report(effect: usize) -> String {
    let pos = super::pose::ptr(rd::<u32>(effect));
    let at = if readable(pos, 12) { format!("{:.0?}", rd::<[f32; 3]>(pos)) } else { "?".into() };
    let grip = left_world().map(|m| m[0].w_axis.truncate()).map_or("?".into(), |v| format!("{v:.0?}"));
    let tip = left_world().map(|m| m[WEAPON_BONES - 1].w_axis.truncate()).map_or("?".into(), |v| format!("{v:.0?}"));
    format!("effect at {at}, left grip {grip}, left last bone {tip}, state {:#x}", rd::<u32>(effect + FX_STATE))
}

/// Whether the left weapon's trail is on at this frame of this motion (the current form's own
/// left weapon's windows).
fn left_window(motion: u16, frame: u32, either: bool) -> bool {
    match TRAIL_WINDOWS.get().and_then(|all| all.get(&form_moveset()?)) {
        Some(w) => w.get(&motion).is_some_and(|v| v.iter().any(|&(s, e)| frame >= s && e.map_or(true, |e| frame < e))),
        None => either,
    }
}

/// Our trail effect is still running, and still ours (the effect pool reuses entries).
unsafe fn trail_alive(effect: usize, fake: usize) -> bool {
    if !readable(effect, 0x3C) || rd::<u32>(effect + FX_STATE) & 3 != 1 {
        return false;
    }
    let owner = super::pose::ptr(rd::<u32>(effect + FX_OWNER));
    readable(owner, 8) && rd::<u64>(owner) as usize == fake
}

/// Each KH2 step: our trails started, faded and ended as KH1 does its own (`0x1402BFD70`):
/// - made as the left weapon's trail window opens;
/// - told to end (its end byte) as it closes;
/// - ended at once out of Valor.
unsafe fn trail_step(valor: bool, motion: u16, frame: u32, either: bool, acting: bool) {
    static N: AtomicU32 = AtomicU32::new(0);
    if !TRAIL_HOOKED.load(Ordering::Relaxed) {
        return;
    }
    let base = BASE.load(Ordering::Relaxed);
    let fake = FAKE.load(Ordering::Relaxed);
    let Ok(mut g) = TRAILS.try_lock() else { return };
    let (trails, was_open) = &mut *g;
    let open = valor && acting && left_window(motion, frame, either);

    // Those that ended, or aren't ours any more, are forgotten; out of Valor, ended.
    static R: AtomicU32 = AtomicU32::new(0);
    trails.retain_mut(|t| {
        if t.logged < 6 && R.fetch_add(1, Ordering::Relaxed) < 60 {
            t.logged += 1;
            log(&format!("left Keyblade: trail {:#x} step {}: {}", t.effect, t.logged, trail_report(t.effect)));
        }
        if !trail_alive(t.effect, fake) {
            log_n(&N, 12, || format!("trail {:#x} ended", t.effect));
            return false;
        }
        if !valor {
            let kill: unsafe extern "system" fn(usize) = std::mem::transmute(base + EFFECT_KILL_RVA);
            kill(t.effect);
            log_n(&N, 12, || format!("trail {:#x} ended (out of Valor)", t.effect));
            return false;
        }
        if !open {
            t.fading = true;
        }
        if t.fading {
            let end = super::pose::ptr(rd::<u32>(t.effect + FX_END));
            if readable(end, 1) {
                std::ptr::write_volatile(end as *mut u8, 1);
            }
        }
        true
    });
    if !valor {
        FAKE_FILLED.store(false, Ordering::Relaxed);
    }
    LEFT_OPEN.store(open, Ordering::Relaxed);
    let opened = open && !*was_open;
    *was_open = open;
    if !opened {
        return;
    }
    // The window opened: a new trail, as KH1 makes the right one (`0x1402BFD70`).
    let id = TRAIL_ID.load(Ordering::Relaxed);
    let sora = super::takeover::sora_actor();
    let obj = if readable(sora + ACTOR_WEAPON, 4) { super::pose::ptr(rd::<u32>(sora + ACTOR_WEAPON)) } else { 0 };
    let right = if readable(obj + OBJ_EFFECTS, 8) { rd::<u64>(obj + OBJ_EFFECTS) as usize } else { 0 };
    let own = EFFECTS.load(Ordering::Relaxed);
    let why = if id == u32::MAX {
        Some("no trail id asked of KH1 yet")
    } else if (own == 0 || !readable(own, 0x10)) && (right == 0 || !readable(right, 0x10)) {
        Some("neither Keyblade has effect data")
    } else if fake == 0 || !FAKE_FILLED.load(Ordering::Relaxed) {
        Some("its owner isn't ready")
    } else if rd::<u32>(fake + ACTOR_STATE) & 3 != 1 {
        Some("Sora isn't in the field")
    } else if trails.len() >= MAX_TRAILS {
        Some("too many trails")
    } else {
        None
    };
    if let Some(why) = why {
        log_n(&N, 12, || format!("trail not made at motion {motion} frame {frame}: {why}"));
        return;
    }
    let make: unsafe extern "system" fn(usize, usize, u32) -> usize = std::mem::transmute(base + TRAIL_MAKE_RVA);
    // The left Keyblade's own look; the right one's if its data has no such trail.
    let mut effect = 0;
    let mut whose = "the left Keyblade's own";
    if own != 0 && readable(own, 0x10) {
        effect = make(fake, own, id & !0x4000_0000);
    }
    if (effect == 0 || !readable(effect, 0x3C)) && right != 0 && readable(right, 0x10) {
        whose = "the right Keyblade's";
        effect = make(fake, right, id & !0x4000_0000);
    }
    if effect == 0 || !readable(effect, 0x3C) {
        log_n(&N, 12, || format!("trail not made at motion {motion} frame {frame}: KH1 made no effect (id {id:#x})"));
        return;
    }
    // (KH1 clears this bit on its own trail too, `0x1402BFD70`.)
    std::ptr::write_unaligned((effect + FX_STATE) as *mut u32, rd::<u32>(effect + FX_STATE) & !0x20);
    trails.push(Trail { effect, fading: false, logged: 0 });
    // Each Keyblade's blade range along bone 0x30 (its weapon row's), the left one's on our owner.
    let blade = |o: usize| -> String {
        let row = if readable(o + OBJ_ROW, 8) { rd::<u64>(o + OBJ_ROW) as usize } else { 0 };
        if readable(row + ROW_BLADE, 8) {
            format!("{:.2}..{:.2}", rd::<f32>(row + ROW_BLADE + 4), rd::<f32>(row + ROW_BLADE))
        } else {
            "?".into()
        }
    };
    let left_obj = super::pose::ptr(rd::<u32>(fake + ACTOR_WEAPON));
    log_n(&N, 12, || format!("trail {effect:#x} made at motion {motion} frame {frame} (id {id:#x}, {whose} look; blade right {}, left {})", blade(obj), blade(left_obj)));
}

/// Our trails ended at once (their effect data is about to go: another Keyblade, or released).
fn end_trails() {
    let base = BASE.load(Ordering::Relaxed);
    if let Ok(mut g) = TRAILS.try_lock() {
        let fake = FAKE.load(Ordering::Relaxed);
        for t in g.0.drain(..) {
            unsafe {
                if trail_alive(t.effect, fake) {
                    let kill: unsafe extern "system" fn(usize) = std::mem::transmute(base + EFFECT_KILL_RVA);
                    kill(t.effect);
                }
            }
        }
    }
}
