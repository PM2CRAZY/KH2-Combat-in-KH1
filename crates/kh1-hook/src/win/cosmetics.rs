//! Cosmetics: how things look, never how they play (the user: "a new menu called cosmetics ... a
//! setting in there for keyblades to always have their trail on"). `docs/kh1-cosmetics.md`.
//!
//! Keyblade Trail (Normal | Always, ini `trail_always`, the Config menu's Cosmetics list).
//!
//! KH1 makes a weapon trail with its own trail step (Steam `0x1402C1F20`, run for each actor by the
//! actor step, Checked (decomp)): with no trail running it takes a fresh request from the weapon
//! object (`+0x38` the trail id, `0x40000000` set for one frame, the weapon's effect data copied to
//! `+0x30`; what the trail request `0x1402C2310` writes), then, while the actor's `+0x1CC` bit
//! `0x800000` is set and the actor is alive (`+0x374` bit 0 set, bit 1 clear), makes the trail
//! (`0x1402AE290`) into `+0x28`. A running trail is kept while the actor's flags hold `0xC001` (alive
//! and its weapon drawn: `0x4000`) and that bit stays set, and told to end when the bit goes. The
//! bit is KH1's motion event 7.0 (`0x1403948B0` hands the motion's event bits to the actor each
//! frame), on only over a swing; nothing else reads it.
//!
//! Always: ahead of KH1's trail step for Sora, while his weapon is drawn, the bit is set and, with
//! no trail running or asked for, the same request KH1's attack rows make is written to his weapon
//! object, the trail id of KH1's plain ground swing row (`combat::swing_trail_id`: the weapon's own
//! effect data gives the look). KH1's step then makes, keeps and ends the trail itself, as on a
//! swing: the Keyblade put away (the actor's `0x4000` gone) ends it. The left Keyblade (Valor and
//! Final Form) is the same through `leftblade`'s trail step, which runs this ahead of Sora's and
//! asks the same for its holder. Normal: nothing runs but one flag check.
//!
//! The step is hooked here only when `leftblade` has not hooked it (it calls `before_step` itself),
//! and only once Always is first on (at start from the ini, or from the menu).

use super::addr::Build;
use super::{hook, log};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Always on (`trail_always = on`).
static ALWAYS: AtomicBool = AtomicBool::new(false);
/// Our own hook on KH1's trail step is in (`leftblade`'s serves otherwise).
static OWN_HOOK: AtomicBool = AtomicBool::new(false);
static ORIG_STEP: AtomicUsize = AtomicUsize::new(0);

/// KH1's trail step `(actor)` (Steam; `push rdi; sub rsp, 0x20`, Checked (exe), as `leftblade`).
const TRAIL_STEP_STEAM: usize = 0x2C1F20;
const TRAIL_STEP_PROLOGUE: [u8; 6] = [0x40, 0x57, 0x48, 0x83, 0xEC, 0x20];
/// Actor fields (Checked (decomp)): its flags (`0xC001`: alive and its weapon drawn), the "trail
/// wanted" word `+0x1CC` and its bit, its weapon object handle, its alive word (bit 0 set, bit 1
/// clear).
const ACTOR_FLAGS: usize = 0;
const ACTOR_TRAIL_FLAGS: usize = 0x1CC;
const TRAIL_FLAG: u32 = 0x80_0000;
const ACTOR_WEAPON: usize = 0x14C;
const ACTOR_ALIVE: usize = 0x374;
/// The weapon object's effect data, running trail, the data the step copies for a request and the
/// request (`id | 0x40000000`, negative when none is asked for).
const OBJ_EFFECTS: usize = 0x18;
const OBJ_TRAIL: usize = 0x28;
const OBJ_REQUEST_DATA: usize = 0x30;
const OBJ_REQUEST: usize = 0x38;
const REQUEST_FRESH: u32 = 0x4000_0000;

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// Always is on.
pub fn always() -> bool {
    ALWAYS.load(Ordering::Relaxed)
}

pub unsafe fn install() {
    let on = super::settings::get().trail_always;
    ALWAYS.store(on, Ordering::Relaxed);
    if on {
        ensure_hook();
    }
    log!(Menus, "Cosmetics: Keyblade trail {}", if on { "always (trail_always = on)" } else { "normal (trail_always = off)" });
}

/// The Config menu's Keyblade Trail row: Always (true) or Normal, kept in the ini.
pub fn set_always(on: bool) {
    ALWAYS.store(on, Ordering::Relaxed);
    if on {
        unsafe { ensure_hook() };
    }
    super::settings::save("trail_always", if on { "on" } else { "off" });
    log!(Menus, "Config menu: Keyblade trail {}", if on { "always" } else { "normal" });
}

/// KH1's trail step hooked, unless `leftblade` has it (its hook calls `before_step`). Steam only.
unsafe fn ensure_hook() {
    if OWN_HOOK.load(Ordering::Relaxed) || super::leftblade::trail_step_hooked() {
        return;
    }
    if Build::current() != Build::Steam {
        log!(Menus, "Cosmetics: the Keyblade trail setting is Steam only (KH1's trail step is where the Steam build has it)");
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    if hook::install(base + TRAIL_STEP_STEAM, &TRAIL_STEP_PROLOGUE, step as *const () as usize, &ORIG_STEP, "Keyblade trail step (always on)").is_some() {
        OWN_HOOK.store(true, Ordering::Relaxed);
    }
}

unsafe extern "system" fn step(actor: usize) {
    before_step(actor);
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_STEP.load(Ordering::Acquire));
    f(actor)
}

/// Writes KH1's trail request into a weapon object that has no trail and none asked for (what
/// `0x1402C2310` writes then: the effect data at `+0x30`, `id | 0x40000000` at `+0x38`). False if
/// it had one or the object has no effect data.
pub unsafe fn request(obj: usize, id: u32) -> bool {
    if !super::crash::readable(obj) || !super::crash::readable(obj + OBJ_REQUEST + 3) {
        return false;
    }
    let data = rd::<u64>(obj + OBJ_EFFECTS);
    if data == 0 || rd::<u64>(obj + OBJ_TRAIL) != 0 || rd::<i32>(obj + OBJ_REQUEST) >= 0 {
        return false;
    }
    wr(obj + OBJ_REQUEST_DATA, data);
    wr(obj + OBJ_REQUEST, id | REQUEST_FRESH);
    true
}

/// Ahead of KH1's trail step for `actor`: Sora's trail wanted and asked for while his weapon is
/// drawn, if Always is on.
pub unsafe fn before_step(actor: usize) {
    if !ALWAYS.load(Ordering::Relaxed) || actor == 0 || actor != super::takeover::sora_actor() {
        return;
    }
    // (KH1's own conditions for a trail to stay: alive, weapon drawn.)
    if rd::<u32>(actor + ACTOR_FLAGS) & 0xC001 != 0xC001 {
        return;
    }
    let alive = rd::<u32>(actor + ACTOR_ALIVE);
    if alive & 1 == 0 || alive & 2 != 0 {
        return;
    }
    let obj = super::pose::ptr(rd::<u32>(actor + ACTOR_WEAPON));
    if obj < 0x10000 || !super::crash::readable(obj) || !super::crash::readable(obj + OBJ_REQUEST + 3) {
        return;
    }
    wr(actor + ACTOR_TRAIL_FLAGS, rd::<u32>(actor + ACTOR_TRAIL_FLAGS) | TRAIL_FLAG);
    if rd::<u64>(obj + OBJ_TRAIL) == 0 && rd::<i32>(obj + OBJ_REQUEST) < 0 {
        if let Some(id) = super::combat::swing_trail_id(actor) {
            if request(obj, id) {
                static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                if N.fetch_add(1, Ordering::Relaxed) < 6 {
                    log!(Menus, "Cosmetics: Keyblade trail {id:#x} asked for (always on)");
                }
            }
        }
    }
}
