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
//! 2. **Pose**: on Valor's KH1 states, KH1's own pose task poses the holder actor (`holder`, one
//!    of KH1's actors attached to Sora's left palm) with the holder motion of Sora's move (the left
//!    hand's weapon bones baked from KH2's left Keyblade in the palm's space, `kh2slots::holder_one`;
//!    KH1's own slots get Valor's idle hold), through KH1's own blends (t831). Every two-Keyblade
//!    form (Valor, Master, Final) runs on KH1's states.
//! 3. **Draw**: on the forms' KH1 states (t832) KH1's own draw task draws the holder actor
//!    (`holder`), whose weapon object is the left Keyblade's KH1 object from KH1's own pool, every
//!    body mesh hidden.
//!
//! The holder actor is also the trail's owner (KH1's own step runs its trail step, t832) and the
//! bone source of the left hits and KH2's second-weapon effects (`left_owner`, t833). With no holder
//! actor (KH1's pool full) nothing of the left Keyblade shows. (t1035: the old controller's copy of
//! Sora's actor, its left draw and its trail, which served Final Form under KH2's controller, are
//! gone: that controller was removed, and Final runs on KH1's states since t1033.)
//!
//! 4. **Trail**: KH1's own trail system on the holder: each request KH1 gets for Sora
//!    (`0x1402C2310`) is asked for the holder too (its weapon object, so the left Keyblade's own
//!    effect data: the file's second section, set up as KH1 sets up its weapon's, `0x1401DDF20`;
//!    the right one's if it has none), and KH1's trail step (`0x1402C1F20`) runs on the holder
//!    after Sora's, making, fading and ending the trail by the holder's trail bit: the left
//!    weapon's trail windows baked into the holder motions as KH1's own 7.0 (`kh2slots`), read by
//!    KH1's own pass on the holder.
//!    - KH1's trail is an effect that follows a bone of its owner actor each frame
//!      (`0x1402AC780`), read through the owner's model instance's world matrices.
//!
//! Everything is off with `valor_left_blade = off` (nothing installed). Every step is logged a
//! few times.

use super::addr::Rva;
use super::log;
use glam::Mat4;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// KH1's model queue `(instance, view struct, draw params, actor)`.
const QUEUE_RVA: Rva = Rva(0x1D1F10);
/// `push rbp; push rbx; push rsi; push rdi` (both builds).
const QUEUE_PROLOGUE: [u8; 5] = [0x40, 0x55, 0x53, 0x56, 0x57];
/// KH1's party actor draw (Steam): `push rbp; push rbx; push rsi; push rdi` (Checked (exe)).
const ACTOR_DRAW_STEAM: usize = 0x2A1630;
const ACTOR_DRAW_PROLOGUE: [u8; 5] = [0x40, 0x55, 0x53, 0x56, 0x57];
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
/// KH1's trail step `(actor)` (Steam; `push rdi; sub rsp, 0x20`, Checked (exe)).
const TRAIL_STEP_STEAM: usize = 0x2C1F20;
const TRAIL_STEP_PROLOGUE: [u8; 6] = [0x40, 0x57, 0x48, 0x83, 0xEC, 0x20];
const EFFECT_KILL_RVA: Rva = Rva(0x1DE890);
/// Actor fields: its weapon object handle.
const ACTOR_WEAPON: usize = 0x14C;
/// The weapon object's effect data (the `.wpn`'s second section).
const OBJ_EFFECTS: usize = 0x18;
/// A weapon object's size and fields (KH1's, Steam `0x140286D60`, Checked (decomp)): `+0x00` the
/// model container's handle, `+0x04` its mesh model's (`0x1401D6760`), `+0x0C..+0x0F` its draw
/// colour (the draw-weapon step `0x1402A2E30` sets `ff ff ff 80`), `+0x18` the `.wpn`'s effect
/// data, `+0x20` its row in `btltbl.bin`'s weapon table (whose `+0x24` / `+0x28` are the blade's
/// range along bone 0x30, `0x14029EB20`), `+0x28` / `+0x30` / `+0x38` the trail (`0x1402C2310`,
/// `0x1402C1F20`), `+0x40` (u16) and `+0x48` cleared at creation.
const WEAPON_OBJ_SIZE: usize = 0x60;
const OBJ_CONTAINER: usize = 0x00;
const OBJ_MESH: usize = 0x04;
const OBJ_COLOUR: usize = 0x0C;
const OBJ_ROW: usize = 0x20;
const OBJ_TRAIL: usize = 0x28;
const OBJ_TRAIL_ID: usize = 0x38;
/// KH1's weapon object pool (Steam): nine objects of 0x60 bytes, their in-use bits (a u32; KH1's
/// area start `0x140286B40` clears them all), and the call that fills an object's mesh model from
/// its container (`0x1401D6760 (container, &obj+4, &out, &out)`; a `.wpn`'s container is an `MENV`,
/// for which it only copies the container's `+0x20`). Checked (decomp, exe).
const POOL_STEAM: usize = 0x296B1E0;
const POOL_BITS_STEAM: usize = 0x296B5D8;
const POOL_COUNT: usize = 9;
const OBJ_MODEL_STEAM: usize = 0x1D6760;
const OBJ_MODEL_PROLOGUE: [u8; 5] = [0x48, 0x89, 0x5C, 0x24, 0x08];
/// A weapon row's size.
const WEAPON_ROW_SIZE: usize = 0x58;

/// A file record's buffer, size, and its file-type row (-1: no handler is called on release).
const REC_BUFFER: usize = 0x658;
const REC_ROW: usize = 0x64C;
/// The instance's world matrices (handle).
const INST_WORLD: usize = 0x1B4;
/// The model queue's draw params' size.
const P_SIZE: usize = 0x60;
/// The weapon bones: the first, how many, and Sora's bone count.
const GRIP: usize = 280;
const WEAPON_BONES: usize = 13;
const SORA_BONES: usize = 293;
/// KH1 Sora's hands (`pose::HANDS`).
const RIGHT_HAND: usize = 101;
const LEFT_HAND: usize = 206;
/// Our load buffer (Sora's own weapon slot is 0x40000; twice that to be safe).
const BUF_SIZE: usize = 0x80000;
const MENV: u32 = u32::from_le_bytes(*b"MENV");
const HD_MARK: u32 = 0x9696_9696;
/// A load that hasn't answered by then is given up (KH1 never calls back on a failure).
const LOAD_TIMEOUT_S: f32 = 15.0;

static INSTALLED: AtomicBool = AtomicBool::new(false);
static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_QUEUE: AtomicUsize = AtomicUsize::new(0);
static ORIG_DRAW: AtomicUsize = AtomicUsize::new(0);
/// Valor's form is out (`show` / `hide`).
static SHOWN: AtomicBool = AtomicBool::new(false);
/// The mesh model drawn in the left hand (0: none ready).
static MESH: AtomicUsize = AtomicUsize::new(0);
/// The left Keyblade's effect data (0: none; the right one's used).
static EFFECTS: AtomicUsize = AtomicUsize::new(0);
/// The buffer of the load under way (0: none).
static PENDING_BUF: AtomicUsize = AtomicUsize::new(0);

struct Loaded {
    item: u8,
    buf: usize,
    container: usize,
    mesh: usize,
    /// Its effect data (the trail's look), 0 if the file has none.
    effects: usize,
    /// Its row in `btltbl.bin`'s weapon table (its weapon object's `+0x20`).
    row: usize,
}

/// `l` is the Keyblade drawn in the left hand from now on (its weapon object is made from it at
/// its first use, `left_object`).
fn make_current(l: &Loaded) {
    MESH.store(l.mesh, Ordering::Relaxed);
    EFFECTS.store(l.effects, Ordering::Relaxed);
    OBJ_ROW_NOW.store(l.row, Ordering::Relaxed);
    OBJ_FROM.store(l.container, Ordering::Relaxed);
}

/// No Keyblade in the left hand any more (its model is about to go): not drawn, and its weapon
/// object given back to KH1's pool.
unsafe fn clear_current() {
    MESH.store(0, Ordering::Relaxed);
    EFFECTS.store(0, Ordering::Relaxed);
    free_object();
    OBJ_FROM.store(0, Ordering::Relaxed);
    OBJ_ROW_NOW.store(0, Ordering::Relaxed);
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

static ORIG_TRAIL_START: AtomicUsize = AtomicUsize::new(0);
static TRAIL_HOOKED: AtomicBool = AtomicBool::new(false);
/// The trail id KH1 was last asked for Sora (`u32::MAX`: none yet).
static ORIG_TRAIL_STEP: AtomicUsize = AtomicUsize::new(0);
/// Inside our own call of KH1's trail request or step (for the holder).
static IN_TRAIL: AtomicBool = AtomicBool::new(false);
/// The left Keyblade's weapon object: one of KH1's own pool (0: none taken), and what it is made
/// of (the loaded model's container and weapon row; 0 while none is loaded).
static LEFT_OBJ: AtomicUsize = AtomicUsize::new(0);
static OBJ_FROM: AtomicUsize = AtomicUsize::new(0);
static OBJ_ROW_NOW: AtomicUsize = AtomicUsize::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

fn readable(at: usize, len: usize) -> bool {
    at >= 0x10000 && unsafe { super::crash::readable(at) && super::crash::readable(at + len.max(1) - 1) }
}

/// Logs `line` the first `max` times its counter is bumped.
fn log_n(counter: &AtomicU32, max: u32, line: impl FnOnce() -> String) {
    if super::log_on(super::LogCat::Animation) && counter.fetch_add(1, Ordering::Relaxed) < max {
        log!(Animation, "left Keyblade: {}", line());
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
    // (Its weapon object comes from KH1's own pool, at Steam addresses: Steam only.)
    let steam = super::addr::Build::current() == super::addr::Build::Steam;
    let patched = if !left {
        log!(Animation, "left Keyblade: off (valor_left_blade = off)");
        false
    } else if !steam || std::slice::from_raw_parts((base + OBJ_MODEL_STEAM) as *const u8, OBJ_MODEL_PROLOGUE.len()) != OBJ_MODEL_PROLOGUE {
        log!(Animation, "left Keyblade: not installed (KH1's weapon object code is where the Steam build has it only)");
        false
    } else if !super::crash::readable(slot) || !super::hook::patch(slot, &want, &ours, "the .wpn release handler (left Keyblade)") {
        log!(Animation, "left Keyblade: KH1's .wpn release handler not where expected: not installed");
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
    // KH1's actor draw (`actor_draw`: where KH1 drew the holder, for the trace).
    if super::hook::install(base + ACTOR_DRAW_STEAM, &ACTOR_DRAW_PROLOGUE, actor_draw as *const () as usize, &ORIG_DRAW, "actor draw (left Keyblade)").is_none() {
        super::hook::patch(slot, &ours, &want, "the .wpn release handler (back)");
        return;
    }
    INSTALLED.store(true, Ordering::Relaxed);
    // The trail, KH1's own (fails alone: the Keyblade is still drawn): its request asked for the
    // holder after Sora; its step hooked for `cosmetics`.
    if super::hook::install(base + TRAIL_START_RVA, &TRAIL_START_PROLOGUE, trail_start as *const () as usize, &ORIG_TRAIL_START, "Keyblade trail request (left Keyblade)").is_none() {
        log!(Animation, "left Keyblade: trail off (KH1's trail request not where expected)");
    } else if super::hook::install(base + TRAIL_STEP_STEAM, &TRAIL_STEP_PROLOGUE, trail_step as *const () as usize, &ORIG_TRAIL_STEP, "Keyblade trail step (left Keyblade)").is_none() {
        log!(Animation, "left Keyblade: trail off (KH1's trail step not where expected)");
    } else {
        TRAIL_HOOKED.store(true, Ordering::Relaxed);
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
    let Some(item) = item else {
        MESH.store(0, Ordering::Relaxed);
        return;
    };
    let base = BASE.load(Ordering::Relaxed);
    let Ok(mut st) = STATE.try_lock() else { return };
    // A load that never answered: given up (its buffer is kept, never reused).
    if let Some((it, buf, since)) = st.pending {
        if since.elapsed().as_secs_f32() > LOAD_TIMEOUT_S {
            log!(Animation, "left Keyblade: the load of {it:#x} never answered ({:.0} s): given up", since.elapsed().as_secs_f32());
            st.pending = None;
            st.failed.push(it);
            if PENDING_BUF.load(Ordering::Relaxed) == buf {
                PENDING_BUF.store(0, Ordering::Relaxed);
            }
        }
    }
    if let Some(l) = st.loaded.as_ref() {
        if l.item == item {
            make_current(l);
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
        end_trails();
        clear_current();
        release(&old);
    }
    let Some(name) = file_name(base, item) else {
        log!(Animation, "left Keyblade: no weapon file for item {item:#x}");
        st.failed.push(item);
        return;
    };
    let buf = super::sys::VirtualAlloc(std::ptr::null_mut(), BUF_SIZE, 0x3000, 0x04) as usize;
    if buf == 0 {
        log!(Animation, "left Keyblade: no memory for the model");
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
    log!(Animation, "left Keyblade: load of {name} (item {item:#x}) requested into {buf:#x} (request {id})");
}

/// Back from Valor (`valor::ended`): not drawn; the model stays loaded.
pub fn hide() {
    SHOWN.store(false, Ordering::Relaxed);
}

/// The left Keyblade's owner for KH1's own bindings (`kh2dpx`: KH2's second-weapon points 0x4004 /
/// 0x4006 / 0x4008 on KH1's blade binding; `lefthits`): the holder actor (`holder`: KH1's own actor
/// on Sora's left palm, posed by KH1 with the left Keyblade's bones, its weapon object the left
/// Keyblade's, so KH1's blade code 0x30 reads the left blade's range) while it carries the left
/// Keyblade's object (t833). None while the left Keyblade isn't drawn (out of the two-Keyblade
/// forms, or its model not in yet) or with no holder actor (KH1's pool full). (t1035: the old
/// controller's copy of Sora, `fill_owner`, is gone; every form runs on KH1's states.)
pub fn left_owner() -> Option<usize> {
    if !INSTALLED.load(Ordering::Relaxed) || !(super::form::two_blades() && SHOWN.load(Ordering::Relaxed) && MESH.load(Ordering::Relaxed) != 0) {
        return None;
    }
    if !super::form::native_now() {
        return None;
    }
    let a = super::holder::actor()?;
    let obj = LEFT_OBJ.load(Ordering::Relaxed);
    let carried = obj != 0 && unsafe { rd::<u32>(a + ACTOR_WEAPON) } == super::pose::handle(obj) && super::holder::posed_instance().is_some();
    carried.then_some(a)
}

/// `left_owner`'s handle (KH1's 32-bit handle of the holder actor), for KH1's own code that takes
/// the bone-source actor by handle (`lefthits`: an attack object's `+0x90`).
pub fn left_owner_handle() -> Option<u32> {
    let a = left_owner()?;
    Some(super::pose::handle(a)).filter(|&h| h != 0)
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
        log!(Animation, "left Keyblade: busy when the load answered: model not used");
        return;
    };
    let Some((item, _, _)) = st.pending.take() else { return };
    let row = weapon_row(BASE.load(Ordering::Relaxed), item).unwrap_or(0);
    match set_up(size, buf) {
        Ok((container, mesh, effects)) => {
            let l = Loaded { item, buf, container, mesh, effects, row };
            if SHOWN.load(Ordering::Relaxed) {
                make_current(&l);
            }
            st.loaded = Some(l);
        }
        Err(e) => {
            log!(Animation, "left Keyblade: item {item:#x}'s model not used: {e}");
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
    log!(Animation,
        "left Keyblade: model set up: container {container:#x}, mesh {mesh:#x}, {bones} bones, {} meshes, {}",
        rd::<i32>(mesh + 0xC),
        if hd { "remastered (HD) table hooked up" } else { "no remastered table (PS2 mesh)" }
    );
    // Its effect data (`+4` up to the model), as KH1's handler sets up its weapon's: the trail's
    // look is the Keyblade's own (the user: "it doesnt play keyblade specific trails for the
    // second keyblade").
    let effects = if o8 > o4 && o4 >= 0x10 && readable(buf + o4, 0x14) {
        let at = buf + o4;
        let n = rd::<i32>(at + 0xC);
        if (0..=256).contains(&n) && o4 + 0x10 + n as usize * 0x20 <= o8 {
            let setup: unsafe extern "system" fn(usize) = std::mem::transmute(BASE.load(Ordering::Relaxed) + EFFECTS_SETUP_RVA);
            setup(at);
            log!(Animation, "left Keyblade: its effect data set up at {at:#x} ({n} entries)");
            at
        } else {
            log!(Animation, "left Keyblade: effect data at {o4:#x} looks wrong ({n} entries): the right Keyblade's trail look used");
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
        log!(Animation, "left Keyblade: item {:#x}'s model released (record {record:#x})", old.item);
    } else {
        log!(Animation, "left Keyblade: item {:#x}'s file record not found ({record:#x}): left loaded", old.item);
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
        end_trails();
        clear_current();
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
// The weapon object (KH1's own)

/// The pool's object `i` and its in-use bit.
fn pool_obj(i: usize) -> usize {
    BASE.load(Ordering::Relaxed) + POOL_STEAM + i * WEAPON_OBJ_SIZE
}
fn pool_index(obj: usize) -> Option<usize> {
    let first = pool_obj(0);
    (obj >= first && obj < first + POOL_COUNT * WEAPON_OBJ_SIZE && (obj - first) % WEAPON_OBJ_SIZE == 0).then(|| (obj - first) / WEAPON_OBJ_SIZE)
}
unsafe fn pool_bits() -> u32 {
    rd::<u32>(BASE.load(Ordering::Relaxed) + POOL_BITS_STEAM)
}
unsafe fn set_pool_bits(v: u32) {
    std::ptr::write_unaligned((BASE.load(Ordering::Relaxed) + POOL_BITS_STEAM) as *mut u32, v);
}

/// `obj` is still ours: its pool bit is set and it still holds our model and row (KH1's area start
/// clears every bit, `0x140286B40`, and a party member's weapon may take the place after it).
unsafe fn object_ours(obj: usize, container: usize, row: usize) -> bool {
    let Some(i) = pool_index(obj) else { return false };
    pool_bits() & (1 << i) != 0 && rd::<u32>(obj + OBJ_CONTAINER) == super::pose::handle(container) && rd::<u64>(obj + OBJ_ROW) as usize == row
}

/// The left Keyblade's weapon object: one of KH1's own pool, made as KH1 makes a party member's
/// (`0x140286D60`, Steam, Checked (decomp)) on the left `.wpn`'s model: a free bit of the pool's
/// mask taken, `+0x18` its effect data (the right Keyblade's while its own file has none: the
/// trail's look), `+0x20` its weapon row, `+0x40` / `+0x48` cleared, `+0` its container's handle and
/// `+4` its mesh model through KH1's `0x1401D6760`. The draw colour (`+0x0C..`) and effect data are
/// brought up to date each call (the right object's colour: KH1's fade follows). Made again when
/// KH1 took it back (area start). None while no model is loaded or the pool is full (logged).
unsafe fn left_object() -> Option<usize> {
    static N: AtomicU32 = AtomicU32::new(0);
    let (container, row) = (OBJ_FROM.load(Ordering::Relaxed), OBJ_ROW_NOW.load(Ordering::Relaxed));
    if container == 0 || row == 0 || MESH.load(Ordering::Relaxed) == 0 {
        return None;
    }
    let mut obj = LEFT_OBJ.load(Ordering::Relaxed);
    if obj == 0 || !object_ours(obj, container, row) {
        obj = 0;
        let bits = pool_bits();
        if let Some(i) = (0..POOL_COUNT).find(|i| bits & (1 << i) == 0) {
            set_pool_bits(bits | 1 << i);
            obj = pool_obj(i);
            std::ptr::write_bytes(obj as *mut u8, 0, WEAPON_OBJ_SIZE);
            std::ptr::write_unaligned((obj + OBJ_ROW) as *mut u64, row as u64);
            // (No trail asked for yet: `0x1402C1F20` reads -1 as none.)
            std::ptr::write_unaligned((obj + OBJ_TRAIL_ID) as *mut i32, -1);
            std::ptr::write_unaligned((obj + OBJ_CONTAINER) as *mut u32, super::pose::handle(container));
            let model: unsafe extern "system" fn(usize, usize, usize, usize) = std::mem::transmute(BASE.load(Ordering::Relaxed) + OBJ_MODEL_STEAM);
            let (mut a, mut b) = (0u32, 0u32);
            model(container, obj + OBJ_MESH, &mut a as *mut u32 as usize, &mut b as *mut u32 as usize);
            log_n(&N, 6, || format!("its KH1 weapon object made: pool object {i} at {obj:#x}, mesh {:#x}, row {row:#x}", super::pose::ptr(rd::<u32>(obj + OBJ_MESH))));
        } else {
            log_n(&N, 6, || format!("KH1's weapon object pool is full ({bits:#x}): no object for it"));
        }
        LEFT_OBJ.store(obj, Ordering::Relaxed);
        if obj == 0 {
            return None;
        }
    }
    // Its look and colour, from the right Keyblade's object where it has none of its own.
    let sora = super::takeover::sora_actor();
    let right = if readable(sora + ACTOR_WEAPON, 4) { super::pose::ptr(rd::<u32>(sora + ACTOR_WEAPON)) } else { 0 };
    let fx = match EFFECTS.load(Ordering::Relaxed) {
        0 if readable(right + OBJ_EFFECTS, 8) => rd::<u64>(right + OBJ_EFFECTS),
        fx => fx as u64,
    };
    std::ptr::write_unaligned((obj + OBJ_EFFECTS) as *mut u64, fx);
    if right != 0 && readable(right + OBJ_COLOUR, 4) {
        std::ptr::write_unaligned((obj + OBJ_COLOUR) as *mut u32, rd::<u32>(right + OBJ_COLOUR));
    }
    Some(obj)
}

/// The left Keyblade's KH1 weapon object for the holder actor (`holder`, t832), made or still ours.
pub fn left_object_now() -> Option<usize> {
    if !INSTALLED.load(Ordering::Relaxed) || !left_shown() {
        return None;
    }
    unsafe { left_object() }
}

/// The left Keyblade's trail ended at once (`holder`, before its actor ends: the trail follows it).
pub fn end_left_trail() {
    unsafe { end_trails() }
}

/// Our weapon object back to KH1's pool (KH1's own release order, `0x1402868E0`: its pool bit
/// cleared; the model's `0x1401D5E40` does nothing for an `MENV`), if it is still ours.
unsafe fn free_object() {
    let obj = LEFT_OBJ.swap(0, Ordering::Relaxed);
    let (container, row) = (OBJ_FROM.load(Ordering::Relaxed), OBJ_ROW_NOW.load(Ordering::Relaxed));
    if obj != 0 && object_ours(obj, container, row) {
        if let Some(i) = pool_index(obj) {
            set_pool_bits(pool_bits() & !(1 << i));
            static N: AtomicU32 = AtomicU32::new(0);
            log_n(&N, 6, || format!("its KH1 weapon object {i} given back"));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Posing

/// `pose`, a baked slot of a two-Keyblade form with no holder actor out: KH2's left Keyblade this
/// frame (the rig's second weapon), if there is one, for KH2's player model (`model`). (t1035: the
/// old controller's left draw and trail windows it also fed are gone.)
pub fn kh2_left(bones: Option<&[Mat4]>) {
    super::model::note_left(bones);
}

/// A matrix with its translation times `k` (KH2 units to KH1's).
fn scaled(m: Mat4, k: f32) -> Mat4 {
    let mut m = m;
    m.w_axis = (m.w_axis.truncate() * k).extend(1.0);
    m
}

/// The left hand's 13 weapon bones (KH1 model space) for one frame, and the grip's distance from
/// the left hand, as the old per-frame draw had them, used at load to bake them into the holder
/// motions of a form without KH2's own left hold (`pose::Baker::holder_poses`, `kh2slots`; since
/// t1035 Valor, Master and Final all have one). `kh1` / `kh1_bind`: KH1's pose and bind (model
/// space); `b2`: KH2's bind of the form's skeleton; `(sr, sl)`: KH2's right / left weapon hand
/// bones; `kh2_body`, `right_root`, `left_root`: KH2's body and both weapon roots (KH2 model
/// space); `ratio`: KH1's size against KH2's. `None` when it isn't finite.
#[allow(clippy::too_many_arguments)]
pub fn left_bones(kh1: &[Mat4], kh1_bind: &[Mat4], b2: &[Mat4], (sr, sl): (usize, usize), kh2_body: &[Mat4], right_root: Mat4, left_root: Mat4, ratio: f32) -> Option<([Mat4; WEAPON_BONES], f32)> {
    if kh1.len() < SORA_BONES || kh1_bind.len() < SORA_BONES || kh2_body.len() <= sr.max(sl) || b2.len() <= sr.max(sl) {
        return None;
    }
    let k = if ratio.is_finite() && ratio > 0.1 { ratio } else { 1.0 };
    // KH2's hand bone in KH1's hand frame (bind poses).
    let offset = |kh1_hand: usize, kh2_hand: usize| kh1_bind[kh1_hand].inverse() * scaled(b2[kh2_hand], k);
    // Weapon root relative to its KH2 hand bone now.
    let rel = |kh2_hand: usize, root: Mat4| scaled(kh2_body[kh2_hand].inverse() * root, k);
    let right = kh1[RIGHT_HAND] * offset(RIGHT_HAND, sr) * rel(sr, right_root);
    let turn = right.inverse() * kh1[GRIP];
    let grip = kh1[LEFT_HAND] * offset(LEFT_HAND, sl) * rel(sl, left_root) * turn;
    if !grip.is_finite() {
        return None;
    }
    let from_hand = (grip.w_axis - kh1[LEFT_HAND].w_axis).truncate().length();
    let inv_grip = kh1[GRIP].inverse();
    let mut out = [Mat4::IDENTITY; WEAPON_BONES];
    for (i, m) in out.iter_mut().enumerate() {
        *m = grip * inv_grip * kh1[GRIP + i];
    }
    Some((out, from_hand))
}

/// KH1 Sora's left hand bone and the first weapon bone (the bake's holder motions).
pub const LEFT_HAND_BONE: usize = LEFT_HAND;
/// KH1 Sora's left wrist (205), the left palm's parent and a drawn bone: the holder hangs on it
/// (`holder`, t849).
pub const LEFT_WRIST_BONE: usize = 205;
pub const GRIP_BONE: usize = GRIP;
pub const WEAPON_BONE_COUNT: usize = WEAPON_BONES;

// ---------------------------------------------------------------------------------------------
// The holder (t831): KH1's own actor (`holder`), posed by KH1's own pose task

/// The holder actor's model instance, or any model KH1's maker poses while making the holder
/// (t838) (KH1's passes on it go straight to KH1's: `pose`'s hooks).
pub fn is_holder(instance: usize) -> bool {
    instance != 0 && (super::holder::making() || super::holder::instance() == Some(instance))
}

/// The left Keyblade is out (Valor's or Final's, its model in): `holder` makes its actor then.
pub fn left_out() -> bool {
    left_shown()
}



// ---------------------------------------------------------------------------------------------
// Drawing

type Queue = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;

unsafe extern "system" fn queue(instance: usize, view: usize, params: usize, actor: usize) -> usize {
    let f: Queue = std::mem::transmute(ORIG_QUEUE.load(Ordering::Acquire));
    // KH2's player model: Sora's own draw (the field's, or the pause menu's copy) with his
    // meshes held back, and his Keyblade shrunk into his grip when KH2's replaces it.
    let first = readable(params, P_SIZE);
    let ours = if first && actor != 0 && actor == super::takeover::sora_actor() && instance == super::pose::SORA_INSTANCE.load(Ordering::Relaxed) {
        super::model::hide(params)
    } else if first && super::model::is_menu_copy(instance) {
        super::model::hide_menu(instance, params)
    } else {
        None
    };
    let collapsed = ours.as_ref().is_some_and(|h| h.collapse).then(|| collapse_weapon(instance)).flatten();
    let r = f(instance, view, ours.as_ref().map_or(params, |h| h.params.0.as_ptr() as usize), actor);
    if let Some((at, saved)) = collapsed {
        std::ptr::copy_nonoverlapping(saved.as_ptr(), at as *mut u8, saved.len());
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

/// KH1's party actor draw `(actor, hidden-mesh mask, p3, colour float[4], p5)` (Steam
/// `0x1402A1630`, Checked (decomp)): its weapon is the actor's weapon object (`+0x14C`) while its
/// flag `0x4000` is set (the object's mesh model `+4`, its colour `+0x0C`), queued with the body by
/// KH1's model queue on the instance's weapon bones (`+0x15C`'s world matrices, 280..292).
type ActorDraw = unsafe extern "system" fn(usize, u32, u32, usize, f32);

/// KH1's actor draw, hooked: where KH1 drew the holder, for the trace (t839). (t1035: the old
/// controller's second draw of the left Keyblade on our copy of Sora, `left_draw`, is gone: KH1's
/// own draw task draws the holder actor.)
unsafe extern "system" fn actor_draw(actor: usize, mask: u32, p3: u32, colour: usize, p5: f32) {
    let f: ActorDraw = std::mem::transmute(ORIG_DRAW.load(Ordering::Acquire));
    f(actor, mask, p3, colour, p5);
    super::holder::note_drawn(actor);
}

// ---------------------------------------------------------------------------------------------
// Trail

/// KH1's trail request `(actor, id)` (Steam `0x1402C2310`, Checked (decomp)): it ends the trail the
/// actor's weapon object runs (`+0x28`, `0x1401EC470`), and puts the object's effect data (`+0x18`)
/// at `+0x30` and `id | 0x40000000` at `+0x38` for KH1's trail step to make. Asked for Sora (his
/// row's start, `0x1402A17F0`, and `combat` for KH2's swings), it is asked the same for the left
/// Keyblade's holder while it is out: the left Keyblade's own weapon object gets the request, with
/// its own effect data (its own look).
unsafe extern "system" fn trail_start(actor: usize, id: u32) {
    let f: unsafe extern "system" fn(usize, u32) = std::mem::transmute(ORIG_TRAIL_START.load(Ordering::Acquire));
    f(actor, id);
    if actor != 0 && actor == super::takeover::sora_actor() && !IN_TRAIL.load(Ordering::Relaxed) && left_shown() {
        if let Some(holder) = trail_holder() {
            static N: AtomicU32 = AtomicU32::new(0);
            log_n(&N, 6, || format!("trail: KH1's request {id:#x} for Sora asked for the left Keyblade too"));
            IN_TRAIL.store(true, Ordering::Relaxed);
            f(holder, id);
            IN_TRAIL.store(false, Ordering::Relaxed);
        }
    }
}

/// KH1's trail step `(actor)` (Steam `0x1402C1F20`, from the actor step `0x140297FE0`; Checked
/// (decomp)): with no trail running it takes a fresh request (`+0x38` bit 0x40000000: cleared, one
/// frame), then, while the actor's `+0x1CC` bit 0x800000 is set and it is alive (`0x140291FF0`),
/// makes the trail (`0x1402AE290 (actor, +0x30, +0x38)`) into `+0x28`; a running one is kept while
/// the actor's flags have `0xC001` and the bit is set, told to end (its end byte) when the bit goes,
/// ended at once otherwise, and forgotten once it stops. KH1's own actor step runs it on the
/// holder actor (t832), its bit from the holder motion's 7.0. (t1035: the old controller's second
/// run on our copy of Sora is gone.)
unsafe extern "system" fn trail_step(actor: usize) {
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_TRAIL_STEP.load(Ordering::Acquire));
    // (Cosmetics > Keyblade Trail: Always asks Sora's trail ahead of KH1's step; it hooks the step
    // itself only when this hook isn't in.)
    super::cosmetics::before_step(actor);
    f(actor);
}

/// KH1's trail step is hooked here (`cosmetics` leaves it to this hook).
pub fn trail_step_hooked() -> bool {
    ORIG_TRAIL_STEP.load(Ordering::Acquire) != 0
}

/// The left Keyblade is out (Valor's or Final's, its model in).
fn left_shown() -> bool {
    SHOWN.load(Ordering::Relaxed) && super::form::two_blades() && MESH.load(Ordering::Relaxed) != 0
}

/// The left Keyblade's holder for KH1's trail code: the holder actor (`holder`) holding the left
/// Keyblade's KH1 weapon object, still ours (`object_ours`), on KH1's states (t832 / t833).
unsafe fn trail_holder() -> Option<usize> {
    if !super::form::native_now() {
        return None;
    }
    let a = super::holder::actor()?;
    let obj = LEFT_OBJ.load(Ordering::Relaxed);
    let ok = obj != 0 && TRAIL_HOOKED.load(Ordering::Relaxed) && object_ours(obj, OBJ_FROM.load(Ordering::Relaxed), OBJ_ROW_NOW.load(Ordering::Relaxed)) && rd::<u32>(a + ACTOR_WEAPON) == super::pose::handle(obj);
    ok.then_some(a)
}

/// The left Keyblade's trail ended at once (its effect data or object is about to go), as KH1's
/// trail request ends a running one (`0x1401EC470`).
unsafe fn end_trails() {
    let obj = LEFT_OBJ.load(Ordering::Relaxed);
    if obj == 0 || !object_ours(obj, OBJ_FROM.load(Ordering::Relaxed), OBJ_ROW_NOW.load(Ordering::Relaxed)) {
        return;
    }
    let effect = rd::<u64>(obj + OBJ_TRAIL) as usize;
    if effect != 0 {
        let kill: unsafe extern "system" fn(usize) = std::mem::transmute(BASE.load(Ordering::Relaxed) + EFFECT_KILL_RVA);
        kill(effect);
        std::ptr::write_unaligned((obj + OBJ_TRAIL) as *mut u64, 0);
    }
}
