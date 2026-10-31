//! Valor Form's second Keyblade's holder as one of KH1's own actors (t830, the native thread's
//! second-Keyblade plan; `docs/kh1-weapon-second.md` §12).
//!
//! KH1 gives a character one weapon on one set of weapon bones (280..292), so the left Keyblade
//! needs a second carrier. Up to t829 that was a copy of Sora's actor kept by us (`leftblade`'s
//! owner block). Here it is a real actor of KH1's pool, made, stepped, posed and freed by KH1:
//!
//! - **Its object row.** KH1's actor maker (Steam `0x140290D60 (object id)`) takes a row of an
//!   object table (`0x140287B40`: rows of 0x78 bytes; an id whose byte 2 isn't 1, 7, 8 or 0xC is
//!   looked up by its whole id in the area's general table, `DAT_14296B630`, count
//!   `DAT_14296B628`; the rows live in the loaded area file, Checked (decomp)). Ours is a copy of
//!   Sora's own row (his model, motion set and names, so his loaded model is shared: the maker
//!   uses the row's `+8` model handle as it is) with: id `0x00064B48` (type 6, which nothing in KH1
//!   tests, and the full-model path of `0x140288460`), flags `+4` with 0x1 (no asset load) and 0x2
//!   (no automatic spawn), and without 0x100 (drawn), 0x20, 0x200 (step priority) and 0x10000;
//!   member `+0x4C` = 0xFF and class `+0x4E` = 0xFFFF (no party data, no weapon object from the
//!   party slots, `0x140286D60`); kind `+0x54` = 6 (no think; the maker takes its parent from the
//!   row `+0x52` points at, `0x140287960`); spawn cost `+0x59` 0, placement `+0x5A` 0, group
//!   `+0x58` 0, chain `+0x5B` 0xFF; sizes `+0x10` / `+0x14` and mass `+0x18` 0 (no body push).
//!   The table is pointed at a copy with our row added for the maker's call only, then put back
//!   (KH1's own loops over the table never see it). Checked (decomp).
//! - **Attached to Sora's left wrist** (205; t849, the palm 206 up to t848) by KH1's own attach
//!   (`0x1402A06E0 (actor, parent, bone)`: `+0x3A0` the parent's handle, the row's `+0x50` the
//!   bone, `0x2000 | 205` a raw bone). KH1's world step (`0x1402A0A40`) then makes its world matrix
//!   its local one times the wrist's matrix (`0x14029EA00`); with `+0x374` bit 0x1000 (Euler
//!   turns), no turn, no offset and scale 1 the local one is identity, so the holder's root is
//!   Sora's wrist. The wrist, because KH1 Sora's model draws with 142 of its 293 bones (bone table
//!   `+0x2C` bit 30) and KH1's blend pose (`0x140392560`) and draw queue (`0x1401D4070`) keep only
//!   those: through a blend the others, the palm among them, keep stale matrices with the world
//!   translation added each pass (t848). KH1 skips gravity and the map for an actor with a parent
//!   (`0x140292010`). Checked (decomp, files, trace).
//! - **Stepped after Sora is posed.** KH1 steps actors from one task list in priority order (Sora
//!   2000, kind 6 at 4000), with its pose tasks at -1000, 5000 and 8000 (each poses the actors
//!   stepped since the last). At 4000 the holder would read Sora's palm from the last frame. So
//!   its step task is made again by KH1's own task maker (`0x14028A900`) at 6000, the place KH1
//!   gives its kinds 10 and 13, with KH1's actor step (`0x140297FE0`) and our think; the maker's
//!   own task at 4000 is told to end (a task function returning 4 is unlinked and freed by the
//!   list runner `0x14028A710`). It is then posed at 8000, after Sora at 5000. Checked (decomp).
//! - **Its think** (`think`, called by KH1's step as kinds' thinks are): keeps its local transform
//!   identity, plays Sora's motion's holder motion (`sync`), and ends it with KH1's actor end
//!   (`0x140292390`) once Valor's left Keyblade is gone.
//! - **Its motions (t831).** Its motion table (`+0x1D4`: a handle to an array of motion handles by
//!   slot, ended by 0, Checked (decomp)) is ours: two entries used in turn and a 0. When the holder
//!   motion Sora's motion asks for changes (`kh2slots::holder_motion` of his motion
//!   `ptr(+0x1D4)[+0x168]`; KH1's own slots with none get Valor's idle one at its first frame,
//!   `kh2slots::rest_holder`, KH2's left weapon staying on its palm), it goes in the other entry and
//!   KH1's own motion request (`0x14029F980 (actor, slot | 0x20000000 raw slot, blend f32)`) asks
//!   for it at his time (the queue entry's start `+0x180`) with no blend (t836: KH1's blend moves
//!   the actor by its model root's offset, `0x1402A0350`, which on an attached actor threw the
//!   blade off the palm); KH1's pose step (`0x14029F0A0` → `0x1402A0350`) starts it and poses it. Its time (`+0x16C`) is Sora's each frame, its own speed (`+0x284`) 0, so it never
//!   runs ahead of him. The holder motions keep the weapon bones where they sit from the wrist
//!   (`pose::Baker::holder_poses`, t849); the wrist is the actor's root, so KH1's pose leaves the
//!   left Keyblade in Sora's hand as drawn.
//! - **No hit set** (`+0x134` cleared: no hit or lock-on target, `0x140292070`), hidden-mesh mask
//!   `+0x38C` all set, so KH1's draw shows only its weapon.
//! - **Its weapon and trail (t832).** Its weapon object `+0x14C` is the left Keyblade's KH1 object
//!   (`leftblade::left_object_now`), with its actor flag 0x4000 (weapon out). KH1's per-actor draw
//!   task (`0x1402A1320` → `0x1402A1B70` → `0x1402A1630`) then queues it on the holder's weapon
//!   bones, as it draws Sora's. Its look follows Sora's each frame: drawn bit (actor `+0` bit 0),
//!   colour (`+0x350..+0x35C`, `+0x370`, `+0xA0..+0xAC`, `+0x430`), alpha `+0x60` (its fade rate
//!   `+0x64` kept 0: KH1 ends an actor that fades out, Checked (decomp)), and its frustum cull is off
//!   (`+0x374` bit 0x20000; the palm is in view whenever Sora is). KH1's actor step runs the trail
//!   step (`0x1402C1F20`) on it (it has a weapon object); its trail bit (`+0x1CC` 0x800000) is its
//!   pose's level events (`0x14029F0A0`: `+0x1CC` = the instance's levels, `0x1403948B0`), the left
//!   Keyblade's trail windows baked into the holder motions as KH1's own 7.0
//!   (`kh2slots::holder_one`); KH1's trail request for Sora is asked for it too
//!   (`leftblade::trail_start`). Cosmetics > Keyblade Trail: Always sets its bit and asks its trail
//!   in its think, as `cosmetics` does for Sora. The left hits stay `leftblade`'s copy's (t833).
//!
//! An area change marks every actor dying and KH1's freed task (`0x140292E40`) frees them; ours
//! has its weapon taken off first (t837, `freed`: KH1's weapon give-back would look for a load slot
//! our object doesn't have). The holder is made again when the left Keyblade is out and none of
//! ours is alive. Steam only, every function's first
//! bytes checked before use.

use super::log;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

/// The holder's object id: type 6 (byte 2), "KH" below.
pub const HOLDER_ID: u32 = 0x0006_4B48;
/// KH1 Sora's left wrist (205), the bone the holder hangs on, and the raw-bone flag of KH1's bone
/// reads (`0x14029EA00`). (t849: the wrist, not the palm 206. KH1 Sora's model draws with 142 of
/// its 293 bones, bone table `+0x2C` bit 30, and KH1's blend pose and draw queue keep only those:
/// through a blend the others keep stale matrices with the world translation added each pass. The
/// palm is one of the others, the wrist a drawn one. Checked (files: `xa_ex_0010.mdls`; decomp
/// `0x140392560`, `0x1401D4070`; the owner's t847 traces: 151 = 293 − 142 bones off on every frame
/// the left Keyblade flew).)
const LEFT_WRIST: u32 = 205;
const RAW_BONE: u32 = 0x2000;

// KH1 (Steam) functions, with their first bytes (Checked (exe)).
const MAKE_STEAM: usize = 0x290D60;
const MAKE_PROLOGUE: [u8; 9] = [0x40, 0x55, 0x53, 0x56, 0x48, 0x8D, 0x6C, 0x24, 0xB9];
const TASK_ADD_STEAM: usize = 0x28A900;
const TASK_ADD_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];
const STEP_STEAM: usize = 0x297FE0;
const STEP_PROLOGUE: [u8; 8] = [0x48, 0x8B, 0xC4, 0x57, 0x48, 0x83, 0xEC, 0x60];
const END_STEAM: usize = 0x292390;
const END_PROLOGUE: [u8; 9] = [0x48, 0x85, 0xC9, 0x0F, 0x84, 0xC9, 0x00, 0x00, 0x00];
const ATTACH_STEAM: usize = 0x2A06E0;
const ATTACH_PROLOGUE: [u8; 11] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57];
const TO_HANDLE_STEAM: usize = 0x38AD90;
const REQUEST_STEAM: usize = 0x29F980;
const REQUEST_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];
/// KH1's "actors freed" task (`0x140292E40`, priority 163001, every frame): each actor with
/// `+0x374` bit 2 (dying) and without bit 0x800000 has its model instance freed and, with a weapon
/// object `+0x14C`, its weapon given back by `0x1402868E0`, which finds the weapon's load slot by
/// the row's member `+0x4C` among KH1's three (`DAT_14296B540`) and reads through null when none
/// matches. Checked (decomp). Hooked (t837) so the holder's weapon, the left Keyblade's object of
/// ours that no load slot of KH1's holds (our row's member is 0xFF), is off it first.
const FREED_STEAM: usize = 0x292E40;
const FREED_PROLOGUE: [u8; 5] = [0x48, 0x89, 0x5C, 0x24, 0x10];
static ORIG_FREED: AtomicUsize = AtomicUsize::new(0);
/// `+0x374`: kept by the freed task (not freed).
const KEPT: u32 = 0x80_0000;
/// A motion request's raw-slot bit (no remap through `+0x1D8`, `0x1402A01E0`).
const RAW_SLOT: u32 = 0x2000_0000;

// KH1 (Steam) data.
/// The actor step task list (`+0x10` its first task) and the general object table's count / rows.
const STEP_LIST_STEAM: usize = 0x2D5AB00;
const TABLE_COUNT_STEAM: usize = 0x296B628;
const TABLE_ROWS_STEAM: usize = 0x296B630;
/// The actor pool: 96 actors of 0x4B0 bytes.
const POOL_STEAM: usize = 0x2D372A0;
const POOL_COUNT: usize = 96;
const ACTOR_SIZE: usize = 0x4B0;
/// The holder's step priority (after Sora's pose task at 5000, before the one at 8000).
const PRIORITY: i32 = 6000;

// An object row.
const ROW_SIZE: usize = 0x78;
const ROW_ID: usize = 0x00;
const ROW_FLAGS: usize = 0x04;
const ROW_SIZES: usize = 0x10;
const ROW_MASS: usize = 0x18;
const ROW_PLACE: usize = 0x1C;
const ROW_MEMBER: usize = 0x4C;
const ROW_CLASS: usize = 0x4E;
const ROW_BONE: usize = 0x50;
const ROW_PARENT_ROW: usize = 0x52;
const ROW_KIND: usize = 0x54;
const ROW_GROUP: usize = 0x58;
const ROW_COST: usize = 0x59;
const ROW_PLACEMENT: usize = 0x5A;
const ROW_CHAIN: usize = 0x5B;
const ROW_5F: usize = 0x5F;
/// No asset load, no automatic spawn; not hidden (0x100), nor 0x20, 0x200, 0x10000.
const FLAGS_SET: u32 = 0x1 | 0x2;
const FLAGS_CLEAR: u32 = 0x100 | 0x20 | 0x200 | 0x10000;
const KIND_ATTACHED: u8 = 6;

// An actor.
const A_FLAGS0: usize = 0x00;
const A_ID: usize = 0x04;
const A_ALPHA: usize = 0x60;
const A_ALPHA_RATE: usize = 0x64;
const A_TINT: usize = 0xA0;
const A_TRAIL_FLAGS: usize = 0x1CC;
const A_COLOUR_TO: usize = 0x350;
const A_COLOUR: usize = 0x370;
const A_COLOUR_MUL: usize = 0x430;
/// Actor `+0`: drawn, weapon out. `+0x1CC`: the trail wanted. `+0x374`: no frustum cull.
const DRAWN: u32 = 0x1;
const WEAPON_OUT: u32 = 0x4000;
/// Actor `+0` bit 0x2: KH1's per-actor draw (`0x1402A1B70`) runs the shadow pass
/// (`0x1402BD6F0`) for it, at `+0x50` with the whole model's bones (t842). Checked (decomp).
const SHADOW: u32 = 0x2;
const TRAIL_BIT: u32 = 0x80_0000;
const NO_CULL: u32 = 0x2_0000;
const A_POS: usize = 0x10;
const A_VEL: usize = 0x20;
const A_SPEED_K: usize = 0x2C;
const A_TURN: usize = 0x30;
const A_SCALE: usize = 0x40;
const A_ROW: usize = 0x130;
const A_HITS: usize = 0x134;
const A_WEAPON: usize = 0x14C;
const A_INSTANCE: usize = 0x15C;
const A_MOTION_SPEED: usize = 0x284;
const A_MOTION_FLAGS: usize = 0x160;
const A_SLOT: usize = 0x168;
const A_TIME: usize = 0x16C;
const A_BLEND_LEN: usize = 0x170;
const A_QUEUE_TIME: usize = 0x180;
const A_MOTIONS: usize = 0x1D4;
const A_REMAP: usize = 0x1D8;
/// `+0x160`: the alternate motion tables (`+0x1E0..`) in use.
const ALT_TABLES: u32 = 0x20;
const A_MASK: usize = 0x38C;
const A_FLAGS2: usize = 0x374;
const A_PARENT: usize = 0x3A0;
/// `+0x374`: in use, dying, Euler turns.
const LIVE: u32 = 0x1;
const DYING: u32 = 0x2;
const EULER: u32 = 0x1000;
/// `+0x374`: posed at least once (KH1's pose task sets it, never clears it).
const POSED_ONCE: u32 = 0x800;
const TEAM: u32 = 0x60;
// A task record.
const T_FN: usize = 0x10;
const T_NEXT: usize = 0x08;
const T_ACTOR: usize = 0x18;
const T_THINK: usize = 0x20;

static BASE: AtomicUsize = AtomicUsize::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// The holder actor (0: none made), and our copy of the table with our row (0: not made yet) and
/// how many rows it holds.
static ACTOR: AtomicUsize = AtomicUsize::new(0);
static TABLE: AtomicUsize = AtomicUsize::new(0);
static TABLE_ROWS: AtomicUsize = AtomicUsize::new(0);
/// The holder actor our motion table is on, the table (three u32 handles) and its entry in use,
/// the holder motion playing (0: none asked yet), and the actor a holder motion of ours was asked
/// on (`posed_instance`).
static MOTIONS: AtomicUsize = AtomicUsize::new(0);
static ENTRY: AtomicUsize = AtomicUsize::new(0);
static PLAYING: AtomicUsize = AtomicUsize::new(0);
static POSED: AtomicUsize = AtomicUsize::new(0);
/// KH1's maker is making the holder (t838): it poses the new model inside the call, before we know
/// its actor, and `pose`'s Sora check (a 293-bone model near his actor) would take it for his.
static MAKING: AtomicBool = AtomicBool::new(false);
/// A make that failed (pool full, a handle that didn't resolve): not tried again for a while.
static RETRY_AT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}
fn readable(at: usize, len: usize) -> bool {
    at >= 0x10000 && unsafe { super::crash::readable(at) && super::crash::readable(at + len.max(1) - 1) }
}
fn log_n(counter: &AtomicU32, max: u32, line: impl FnOnce() -> String) {
    if super::log_on(super::LogCat::Animation) && counter.fetch_add(1, Ordering::Relaxed) < max {
        log!(Animation, "left Keyblade holder: {}", line());
    }
}

/// Checks KH1's functions (Steam only); nothing is hooked.
pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    if !super::settings::get().valor_left_blade {
        return;
    }
    if super::addr::Build::current() != super::addr::Build::Steam {
        log!(Animation, "left Keyblade holder: not installed (Steam addresses only)");
        return;
    }
    // Our own probe hooks the motion request (log line "hooked set motion"), so its first bytes
    // are our jump by now: `prologue_is` also accepts the bytes a hook of ours moved (t834).
    let ok = |at: usize, want: &[u8]| super::hook::prologue_is(base + at, want);
    let checks: [(&str, usize, &[u8]); 6] = [
        ("motion request", REQUEST_STEAM, &REQUEST_PROLOGUE),
        ("actor maker", MAKE_STEAM, &MAKE_PROLOGUE),
        ("task maker", TASK_ADD_STEAM, &TASK_ADD_PROLOGUE),
        ("actor step", STEP_STEAM, &STEP_PROLOGUE),
        ("actor end", END_STEAM, &END_PROLOGUE),
        ("attach", ATTACH_STEAM, &ATTACH_PROLOGUE),
    ];
    for (what, at, want) in checks {
        if !ok(at, want) {
            log!(Animation, "left Keyblade holder: not installed (KH1's {what} at {:#x} isn't the Steam build's)", base + at);
            return;
        }
    }
    // (t837: without it an area change would crash on the holder's weapon, so no holder.)
    if super::hook::install(base + FREED_STEAM, &FREED_PROLOGUE, freed as *const () as usize, &ORIG_FREED, "actors freed (left Keyblade holder)").is_none() {
        log!(Animation, "left Keyblade holder: not installed (KH1's actors-freed task couldn't be hooked)");
        return;
    }
    INSTALLED.store(true, Ordering::Relaxed);
    log!(Animation, "left Keyblade holder: KH1's actor maker, task maker, step, end and attach found");
}

/// KH1's freed task: the holder's weapon taken off it when KH1 is about to free it (an area change
/// marks every actor dying, Checked (log: the t835 crash, `rcx` 0xFF our row's member)), then
/// KH1's own pass. One load and compare a frame while no holder is dying.
unsafe extern "system" fn freed() -> u64 {
    let a = ACTOR.load(Ordering::Relaxed);
    if a != 0 && readable(a, ACTOR_SIZE) && rd::<u32>(a + A_ID) == HOLDER_ID {
        let f2 = rd::<u32>(a + A_FLAGS2);
        if f2 & DYING != 0 && f2 & KEPT == 0 {
            static N: AtomicU32 = AtomicU32::new(0);
            let had = rd::<u32>(a + A_WEAPON) != 0;
            wr::<u32>(a + A_WEAPON, 0);
            wr::<u32>(a + A_FLAGS0, rd::<u32>(a + A_FLAGS0) & !WEAPON_OUT);
            ACTOR.store(0, Ordering::Relaxed);
            log_n(&N, 10, || format!("freed by KH1 (dying: an area change or its end){}", if had { "; the left Keyblade's object taken off it first" } else { "" }));
        }
    }
    let f: unsafe extern "system" fn() -> u64 = std::mem::transmute(ORIG_FREED.load(Ordering::Acquire));
    f()
}

/// (t839, a measure for the trace: the owner's "it leaves Sora's hand at idle to run") Where KH1
/// draws the holder this frame, from Sora's left wrist (t849; his palm up to t848): the holder actor's world place
/// (`+0xD0`, its translation `+0x100`), its model's root (bone 0) and its grip (280) and blade
/// base (282), each `+0x1B4` world matrix's translation, KH1 units; with its slot, time, blend
/// length and time (`+0x170` / `+0x174`) and queued requests (`+0x178`). A line in the trace's
/// events while it records log lines; nothing otherwise.
pub unsafe fn note_drawn(a: usize) {
    if !super::trace::enabled() || ACTOR.load(Ordering::Relaxed) != a || !ours(a) {
        return;
    }
    let sora = super::takeover::sora_actor();
    if !readable(sora, ACTOR_SIZE) {
        return;
    }
    let mat_of = |actor: usize, bone: usize| -> Option<glam::Mat4> {
        let inst = super::pose::ptr(rd::<u32>(actor + A_INSTANCE));
        if !readable(inst, 0x1C0) {
            return None;
        }
        let w = super::pose::ptr(rd::<u32>(inst + 0x1B4));
        readable(w + bone * 0x40, 0x40).then(|| glam::Mat4::from_cols_array(&rd::<[f32; 16]>(w + bone * 0x40)))
    };
    let world_of = |actor: usize, bone: usize| mat_of(actor, bone).map(|m| m.w_axis.truncate().to_array());
    let Some(palm) = world_of(sora, LEFT_WRIST as usize) else { return };
    // (t840) The blade's turn from the palm: bone 282's axes in the palm's frame, each one's
    // change since the last draw in degrees (the largest), and its x axis there; and what KH1's
    // draw tests for a weapon (`0x1402A1630`: `+0x14C` and its model, `+0` bit 0x4000), the drawn
    // bit, alpha, the hidden-mesh mask and the instance's world handle.
    static LAST: std::sync::Mutex<Option<glam::Mat3>> = std::sync::Mutex::new(None);
    let turn = match (mat_of(sora, LEFT_WRIST as usize), mat_of(a, 282)) {
        (Some(p), Some(b)) => {
            let rel = glam::Mat3::from_mat4(p).inverse() * glam::Mat3::from_mat4(b);
            let n = |v: glam::Vec3| v.normalize_or_zero();
            let mut step = 0f32;
            if let Ok(mut l) = LAST.lock() {
                if let Some(prev) = *l {
                    for (u, v) in [(prev.x_axis, rel.x_axis), (prev.y_axis, rel.y_axis), (prev.z_axis, rel.z_axis)] {
                        step = step.max(n(u).dot(n(v)).clamp(-1.0, 1.0).acos().to_degrees());
                    }
                }
                *l = Some(rel);
            }
            let x = n(rel.x_axis);
            format!("turn step {step:.1} deg, x in wrist [{:.2} {:.2} {:.2}]", x.x, x.y, x.z)
        }
        _ => String::from("turn -"),
    };
    let obj = super::pose::ptr(rd::<u32>(a + A_WEAPON));
    let obj_model = readable(obj, 4) && super::pose::ptr(rd::<u32>(obj)) != 0;
    let f0 = rd::<u32>(a + A_FLAGS0);
    let look = format!(
        "weapon {} model {} out {} drawn {} alpha {:.2} mask {:#x} flags2 {:#x}",
        (obj != 0) as u8,
        obj_model as u8,
        (f0 & WEAPON_OUT != 0) as u8,
        (f0 & DRAWN) as u8,
        rd::<f32>(a + A_ALPHA),
        rd::<u32>(a + A_MASK),
        rd::<u32>(a + A_FLAGS2)
    );
    let d = |p: Option<[f32; 3]>| p.map_or(String::from("-"), |p| format!("{:.1}", ((p[0] - palm[0]).powi(2) + (p[1] - palm[1]).powi(2) + (p[2] - palm[2]).powi(2)).sqrt()));
    let place = Some(rd::<[f32; 3]>(a + 0x100));
    super::trace::note(&format!(
        "holder drawn, from Sora's left wrist: place {} root {} grip {} blade {} | {turn} | {look} | slot {} t {:.1} blend {:.1}/{:.1} queued {} pos {:?}",
        d(place),
        d(world_of(a, 0)),
        d(world_of(a, 280)),
        d(world_of(a, 282)),
        rd::<u16>(a + A_SLOT),
        rd::<f32>(a + A_TIME),
        rd::<f32>(a + A_BLEND_LEN),
        rd::<f32>(a + A_BLEND_LEN + 4),
        rd::<i32>(a + 0x178),
        rd::<[f32; 3]>(a + 0x50).map(|v| (v * 10.0).round() / 10.0),
    ));
}

/// The holder actor while it is alive and still ours (an area change frees the pool's actors
/// without telling, and the slot may be another actor's by then).
pub fn actor() -> Option<usize> {
    let a = ACTOR.load(Ordering::Relaxed);
    (a != 0 && unsafe { ours(a) }).then_some(a)
}

/// The holder's model instance (KH1's pose pass on it is left to KH1, `pose`).
pub fn instance() -> Option<usize> {
    let a = actor()?;
    let i = super::pose::ptr(unsafe { rd::<u32>(a + A_INSTANCE) });
    (i >= 0x10000).then_some(i)
}

/// KH1's maker is making the holder now: whatever model it poses is the holder's (t838).
pub fn making() -> bool {
    MAKING.load(Ordering::Relaxed)
}

/// The holder's instance once KH1 has posed it with a holder motion of ours (`leftblade::left_owner`).
pub fn posed_instance() -> Option<usize> {
    let a = actor()?;
    (POSED.load(Ordering::Relaxed) == a && unsafe { rd::<u32>(a + A_FLAGS2) } & POSED_ONCE != 0).then(instance).flatten()
}

unsafe fn ours(a: usize) -> bool {
    readable(a, ACTOR_SIZE) && rd::<u32>(a + A_ID) == HOLDER_ID && rd::<u32>(a + A_FLAGS2) & (LIVE | DYING) == LIVE
}

/// The left Keyblade wants a holder: Valor's on KH1's own states, its model in.
fn wanted() -> bool {
    INSTALLED.load(Ordering::Relaxed) && super::leftblade::left_out() && super::form::native_now()
}

/// Each frame, after KH1 posed Sora (`pose`'s pass hooks): the holder made when the left Keyblade
/// is out and none is alive, ended when it isn't wanted (its think ends it too).
pub unsafe fn frame() {
    if !INSTALLED.load(Ordering::Relaxed) {
        return;
    }
    let alive = actor();
    if !wanted() {
        if let Some(a) = alive {
            end(a, "the left Keyblade is put away");
        }
        return;
    }
    if alive.is_some() {
        return;
    }
    let now = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    if now < RETRY_AT.load(Ordering::Relaxed) {
        return;
    }
    if make().is_none() {
        // (Tried again in a second: a full pool frees up as enemies go.)
        RETRY_AT.store(now + 60, Ordering::Relaxed);
    }
}

/// Our row: a copy of Sora's with the changes above, at the end of a copy of the area's table.
/// Returns the table copy, its row count and our row's index.
unsafe fn table_with_row(base: usize, sora: usize) -> Option<(usize, usize, usize)> {
    static N: AtomicU32 = AtomicU32::new(0);
    let rows = rd::<u64>(base + TABLE_ROWS_STEAM) as usize;
    let count = rd::<u32>(base + TABLE_COUNT_STEAM) as usize;
    let sora_row = super::pose::ptr(rd::<u32>(sora + A_ROW));
    if !readable(sora_row, ROW_SIZE) || count > 0x4000 || (count > 0 && !readable(rows, count * ROW_SIZE)) {
        log_n(&N, 5, || format!("no row made (Sora's row {sora_row:#x}, the table {rows:#x} of {count})"));
        return None;
    }
    let need = (count + 1) * ROW_SIZE;
    let mut buf = TABLE.load(Ordering::Relaxed);
    if buf == 0 || TABLE_ROWS.load(Ordering::Relaxed) < count + 1 {
        // (A bigger area table: a new copy; the old one is kept, an actor made from it may point
        // at its row.)
        let size = (need + 0xFFFF) & !0xFFFF;
        buf = super::sys::VirtualAlloc(std::ptr::null_mut(), size, 0x3000, 0x04) as usize;
        if buf == 0 {
            log_n(&N, 5, || "no memory for the row table".into());
            return None;
        }
        TABLE.store(buf, Ordering::Relaxed);
        TABLE_ROWS.store(size / ROW_SIZE, Ordering::Relaxed);
    }
    // KH1's maker keeps the row by handle (the actor's `+0x130`): it has to resolve.
    let to_handle: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(base + TO_HANDLE_STEAM);
    let at = buf + count * ROW_SIZE;
    let h = to_handle(at);
    if super::pose::ptr(h) != at {
        log_n(&N, 5, || format!("the row's handle doesn't resolve ({h:#x} for {at:#x})"));
        return None;
    }
    if count > 0 {
        std::ptr::copy_nonoverlapping(rows as *const u8, buf as *mut u8, count * ROW_SIZE);
    }
    std::ptr::copy_nonoverlapping(sora_row as *const u8, at as *mut u8, ROW_SIZE);
    // Sora's row's place in the table (the maker's parent of a kind-6 row), else ours (the attach
    // right after the make sets the parent anyway).
    let sora_index = (sora_row >= rows && sora_row < rows + count * ROW_SIZE && (sora_row - rows) % ROW_SIZE == 0).then(|| (sora_row - rows) / ROW_SIZE).unwrap_or(count);
    wr::<u32>(at + ROW_ID, HOLDER_ID);
    wr::<u32>(at + ROW_FLAGS, (rd::<u32>(at + ROW_FLAGS) & !FLAGS_CLEAR) | FLAGS_SET);
    wr::<[f32; 2]>(at + ROW_SIZES, [0.0; 2]);
    wr::<f32>(at + ROW_MASS, 0.0);
    wr::<[f32; 4]>(at + ROW_PLACE, [0.0; 4]);
    wr::<u16>(at + ROW_MEMBER, 0xFF);
    wr::<u16>(at + ROW_CLASS, 0xFFFF);
    wr::<u16>(at + ROW_BONE, (RAW_BONE | LEFT_WRIST) as u16);
    wr::<i16>(at + ROW_PARENT_ROW, sora_index as i16);
    wr::<u8>(at + ROW_KIND, KIND_ATTACHED);
    wr::<u8>(at + ROW_GROUP, 0);
    wr::<u8>(at + ROW_COST, 0);
    wr::<u8>(at + ROW_PLACEMENT, 0);
    wr::<u8>(at + ROW_CHAIN, 0xFF);
    wr::<u8>(at + ROW_5F, 0);
    Some((buf, count + 1, count))
}

/// A free actor in KH1's pool (its maker writes through a null pointer when there is none).
unsafe fn pool_has_room(base: usize) -> bool {
    (0..POOL_COUNT).any(|i| {
        let a = base + POOL_STEAM + i * ACTOR_SIZE;
        rd::<u32>(a + A_FLAGS2) & LIVE == 0
    })
}

/// Makes the holder with KH1's own maker from our row, attaches it to Sora's left wrist and moves
/// its step task to 6000 with our think.
unsafe fn make() -> Option<usize> {
    static N: AtomicU32 = AtomicU32::new(0);
    let base = BASE.load(Ordering::Relaxed);
    let sora = super::takeover::sora_actor();
    if !readable(sora, ACTOR_SIZE) || rd::<u32>(sora + A_FLAGS2) & (LIVE | DYING) != LIVE || super::pose::ptr(rd::<u32>(sora + A_INSTANCE)) != super::pose::SORA_INSTANCE.load(Ordering::Relaxed) {
        return None;
    }
    if !pool_has_room(base) {
        log_n(&N, 10, || "KH1's actor pool is full: no left Keyblade until a place frees up".into());
        return None;
    }
    let (table, rows, index) = table_with_row(base, sora)?;
    // KH1's maker, with the table pointed at our copy for the call (then put back).
    let (old_rows, old_count) = (rd::<u64>(base + TABLE_ROWS_STEAM), rd::<u32>(base + TABLE_COUNT_STEAM));
    wr::<u64>(base + TABLE_ROWS_STEAM, table as u64);
    wr::<u32>(base + TABLE_COUNT_STEAM, rows as u32);
    let make: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base + MAKE_STEAM);
    MAKING.store(true, Ordering::Relaxed);
    let a = make(HOLDER_ID);
    MAKING.store(false, Ordering::Relaxed);
    wr::<u64>(base + TABLE_ROWS_STEAM, old_rows);
    wr::<u32>(base + TABLE_COUNT_STEAM, old_count);
    if a == 0 || !ours(a) {
        log_n(&N, 10, || format!("KH1's maker gave no actor for our row ({a:#x})"));
        return None;
    }
    // No hit set (no hit or lock-on target), nothing drawn yet, still in place.
    wr::<u32>(a + A_HITS, 0);
    wr::<u32>(a + A_MASK, u32::MAX);
    wr::<u32>(a + A_FLAGS0, rd::<u32>(a + A_FLAGS0) & !SHADOW);
    wr::<u32>(a + A_WEAPON, 0);
    // (Its team bits, which the maker sets from the model's hit spheres: out of KH1's team lists,
    // `0x140291A50`.)
    wr::<u32>(a + A_FLAGS2, (rd::<u32>(a + A_FLAGS2) | EULER) & !TEAM);
    hold_still(a);
    // Its motions: our table (t831), KH1's remap off, its alternate tables off.
    if !motions_on(base, a) {
        end(a, "its motion table's handle doesn't resolve");
        return None;
    }
    // KH1's attach: Sora's left wrist (t849).
    let attach: unsafe extern "system" fn(usize, usize, u16) = std::mem::transmute(base + ATTACH_STEAM);
    attach(a, sora, (RAW_BONE | LEFT_WRIST) as u16);
    if !retask(base, a) {
        // (Left at 4000: it would read the palm a frame late; ended instead.)
        end(a, "its step task couldn't be moved after Sora's pose");
        return None;
    }
    ACTOR.store(a, Ordering::Relaxed);
    let parent = super::pose::ptr(rd::<u32>(a + A_PARENT));
    log_n(&N, 10, || format!("made at {a:#x} (KH1's actor from our row {index} of {rows}), attached to Sora's left wrist (parent {parent:#x}, Sora {sora:#x}), stepped at {PRIORITY} by KH1's actor step with our think"));
    Some(a)
}

/// The maker's step task for `a` (KH1's actor step at the kind's priority, 4000) told to end, and
/// one made by KH1's task maker at 6000 with KH1's step, `a` and our think, as KH1's own
/// registration (`0x140297980`) makes one.
unsafe fn retask(base: usize, a: usize) -> bool {
    static N: AtomicU32 = AtomicU32::new(0);
    let list = base + STEP_LIST_STEAM;
    let step = base + STEP_STEAM;
    let mut t = rd::<u64>(list + 0x10) as usize;
    let mut old = 0;
    for _ in 0..0x400 {
        if t == 0 || !readable(t, 0x28) {
            break;
        }
        if rd::<u64>(t + T_FN) as usize == step && rd::<u64>(t + T_ACTOR) as usize == a {
            old = t;
            break;
        }
        t = rd::<u64>(t + T_NEXT) as usize;
    }
    if old == 0 {
        log_n(&N, 5, || format!("its step task wasn't found for {a:#x}"));
        return false;
    }
    let add: unsafe extern "system" fn(usize, i32, usize, i32) -> usize = std::mem::transmute(base + TASK_ADD_STEAM);
    let new = add(list, PRIORITY, step, 0);
    if new == 0 || !readable(new, 0x28) {
        log_n(&N, 5, || "KH1's task maker had no room".into());
        return false;
    }
    wr::<u64>(new + T_ACTOR, a as u64);
    wr::<u64>(new + T_THINK, think as *const () as usize as u64);
    wr::<u64>(old + T_FN, retired as *const () as usize as u64);
    true
}

/// The maker's own step task for the holder: ended (the list runner unlinks a task returning 4).
unsafe extern "system" fn retired(_task: usize) -> u32 {
    4
}

/// Its local transform identity (no offset, no turn, scale 1, no velocity), so its root is the
/// palm, and its own motion stopped (t830: it plays nothing of its own).
unsafe fn hold_still(a: usize) {
    wr::<[f32; 3]>(a + A_POS, [0.0; 3]);
    wr::<[f32; 3]>(a + A_VEL, [0.0; 3]);
    wr::<f32>(a + A_SPEED_K, 0.0);
    wr::<[f32; 3]>(a + A_TURN, [0.0; 3]);
    wr::<[f32; 3]>(a + A_SCALE, [1.0; 3]);
    wr::<f32>(a + A_MOTION_SPEED, 0.0);
}

/// The holder's think, run by KH1's actor step each frame (at 6000).
unsafe extern "system" fn think(a: usize) {
    if !wanted() || ACTOR.load(Ordering::Relaxed) != a {
        end(a, "the left Keyblade is put away");
        return;
    }
    hold_still(a);
    sync(a);
    look(a);
}

/// Its weapon (the left Keyblade's KH1 object; none while KH2's model draws it, `model`) and its
/// look as Sora's (t832).
unsafe fn look(a: usize) {
    let sora = super::takeover::sora_actor();
    if !readable(sora, ACTOR_SIZE) {
        return;
    }
    let obj = if super::model::left_is_ours() { None } else { super::leftblade::left_object_now() };
    wr::<u32>(a + A_WEAPON, obj.map_or(0, super::pose::handle));
    // (t842: no shadow. The holder is a copy of Sora's model with every mesh hidden, its body at
    // the bind, so KH1's shadow pass drew a T-posed Sora shadow off it; KH2's left weapon casts
    // none of its own, and Sora's own shadow is his actor's.)
    let f0 = rd::<u32>(a + A_FLAGS0) & !(DRAWN | WEAPON_OUT | SHADOW);
    let drawn = rd::<u32>(sora + A_FLAGS0) & DRAWN;
    wr::<u32>(a + A_FLAGS0, f0 | drawn | if obj.is_some() { WEAPON_OUT } else { 0 });
    std::ptr::copy_nonoverlapping((sora + A_COLOUR_TO) as *const u8, (a + A_COLOUR_TO) as *mut u8, 0x10);
    std::ptr::copy_nonoverlapping((sora + A_COLOUR) as *const u8, (a + A_COLOUR) as *mut u8, 4);
    std::ptr::copy_nonoverlapping((sora + A_TINT) as *const u8, (a + A_TINT) as *mut u8, 0x10);
    std::ptr::copy_nonoverlapping((sora + A_COLOUR_MUL) as *const u8, (a + A_COLOUR_MUL) as *mut u8, 0x10);
    wr::<f32>(a + A_ALPHA, rd::<f32>(sora + A_ALPHA));
    wr::<f32>(a + A_ALPHA_RATE, 0.0);
    wr::<u32>(a + A_FLAGS2, rd::<u32>(a + A_FLAGS2) | NO_CULL);
    // (Cosmetics > Keyblade Trail: Always, as `cosmetics` asks Sora's ahead of his trail step.)
    if let (Some(obj), true) = (obj, super::cosmetics::always()) {
        wr::<u32>(a + A_TRAIL_FLAGS, rd::<u32>(a + A_TRAIL_FLAGS) | TRAIL_BIT);
        if let Some(id) = super::combat::swing_trail_id(sora) {
            super::cosmetics::request(obj, id);
        }
    }
}

/// Our motion table on the holder (`+0x1D4`): two entries used in turn, then 0.
unsafe fn motions_on(base: usize, a: usize) -> bool {
    static N: AtomicU32 = AtomicU32::new(0);
    let mut t = MOTIONS.load(Ordering::Relaxed);
    if t == 0 {
        t = super::sys::VirtualAlloc(std::ptr::null_mut(), 0x1000, 0x3000, 0x04) as usize;
        if t == 0 {
            return false;
        }
        MOTIONS.store(t, Ordering::Relaxed);
    }
    let to_handle: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(base + TO_HANDLE_STEAM);
    let h = to_handle(t);
    if super::pose::ptr(h) != t {
        log_n(&N, 5, || format!("its motion table's handle doesn't resolve ({h:#x} for {t:#x})"));
        return false;
    }
    wr::<[u32; 3]>(t, [0; 3]);
    wr::<u32>(a + A_MOTIONS, h);
    wr::<u32>(a + A_REMAP, 0);
    wr::<u32>(a + A_MOTION_FLAGS, rd::<u32>(a + A_MOTION_FLAGS) & !ALT_TABLES);
    ENTRY.store(0, Ordering::Relaxed);
    PLAYING.store(0, Ordering::Relaxed);
    POSED.store(0, Ordering::Relaxed);
    true
}

/// Sora's motion now (KH1's address of the motion his slot plays), his time in it and his blend
/// length (0: none running).
unsafe fn sora_motion(sora: usize) -> Option<(usize, f32, f32)> {
    let table = super::pose::ptr(rd::<u32>(sora + A_MOTIONS));
    let slot = rd::<u16>(sora + A_SLOT) as usize;
    if !readable(table + slot * 4, 4) {
        return None;
    }
    let m = super::pose::ptr(rd::<u32>(table + slot * 4));
    let t = rd::<f32>(sora + A_TIME);
    let blend = rd::<f32>(sora + A_BLEND_LEN);
    (m != 0 && t.is_finite()).then_some((m, t, if blend.is_finite() && blend > 0.0 { blend } else { 0.0 }))
}

/// The holder plays the holder motion of Sora's motion at his time (KH1's own request and
/// pose; `MOTIONS`), or Valor's idle one at its first frame on a slot of KH1's own.
unsafe fn sync(a: usize) {
    static N: AtomicU32 = AtomicU32::new(0);
    let sora = super::takeover::sora_actor();
    let t = MOTIONS.load(Ordering::Relaxed);
    if !readable(sora, ACTOR_SIZE) || t == 0 || super::pose::ptr(rd::<u32>(a + A_MOTIONS)) != t {
        return;
    }
    let Some((motion, time, blend)) = sora_motion(sora) else { return };
    let (want, at) = match super::kh2slots::holder_motion(motion) {
        Some(h) => (h, time),
        None => match super::kh2slots::rest_holder() {
            Some(h) => (h, 0.0),
            None => return,
        },
    };
    if want != PLAYING.load(Ordering::Relaxed) {
        let h = super::pose::handle(want);
        if super::pose::ptr(h) != want {
            log_n(&N, 5, || format!("holder motion {want:#x} has no handle"));
            return;
        }
        let entry = (ENTRY.load(Ordering::Relaxed) + 1) % 2;
        wr::<u32>(t + entry * 4, h);
        let request: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(BASE.load(Ordering::Relaxed) + REQUEST_STEAM);
        // (t836: no blend of KH1's on the holder. KH1's blend start `0x1402A0350` keeps the model
        // root's offset from the actor (`+0x270`, `+0x220`) and moves the actor by it until the
        // blend ends, root-motion handling for an actor standing on its own; on the holder,
        // attached to the palm, that threw the left Keyblade off the hand for the blend's frames.
        // KH2's left weapon sits on its palm with no blend of its own; Sora's own blend still
        // carries the palm.)
        request(a, entry as u32 | RAW_SLOT, 0.0);
        wr::<f32>(a + A_QUEUE_TIME, at);
        ENTRY.store(entry, Ordering::Relaxed);
        PLAYING.store(want, Ordering::Relaxed);
        POSED.store(a, Ordering::Relaxed);
        if super::trace::enabled() {
            super::trace::note(&format!("holder motion {want:#x} asked (entry {entry}, at {at:.1}; Sora's motion {motion:#x} at {time:.1}, his blend {blend:.1})"));
        }
        log_n(&N, 8, || format!("holder motion {want:#x} asked of KH1 (Sora's motion {motion:#x} at {time:.1}, his blend {blend:.1}, ours none{})", if at == 0.0 && time != 0.0 { ", his own slot: Valor's idle hold" } else { "" }));
    } else {
        wr::<f32>(a + A_TIME, at);
    }
}

/// Ended by KH1's own actor end (its tasks then unlink themselves and its slot is freed).
unsafe fn end(a: usize, why: &str) {
    static N: AtomicU32 = AtomicU32::new(0);
    if !ours(a) {
        return;
    }
    // (Its trail, which follows it, ended first; then nothing of ours on it for KH1's teardown to
    // give back.)
    if rd::<u32>(a + A_WEAPON) != 0 {
        super::leftblade::end_left_trail();
    }
    wr::<u32>(a + A_WEAPON, 0);
    wr::<u32>(a + A_FLAGS0, rd::<u32>(a + A_FLAGS0) & !WEAPON_OUT);
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(BASE.load(Ordering::Relaxed) + END_STEAM);
    f(a);
    if ACTOR.load(Ordering::Relaxed) == a {
        ACTOR.store(0, Ordering::Relaxed);
    }
    log_n(&N, 10, || format!("ended ({why})"));
}
