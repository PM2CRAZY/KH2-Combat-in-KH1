//! Keeping KH2's attack lunges out of KH1's walk-in triggers (`docs/kh1-exits-push.md`). The
//! user: "a combo pushing you forward shouldnt bring you into a new room if you're by the
//! entrance of a room, also, if a combo pushes you forward into a box, it shouldnt go into
//! soras push animation".
//!
//! - Exits are collision faces with an event number: KH1's mover queues the event for Sora
//!   each time a move touches one (the call at `0x1402994F6`, kind -2). While `takeover`'s own
//!   move of an attack frame runs (`pre_move` .. `post_move`), a face event other than the one
//!   he already stood on before the attack is dropped, and that frame's move is undone, so the
//!   lunge stops at the trigger's edge instead of resting inside it (the event fires on every
//!   touch, so a lunge left in it would fire on the next idle frame). Walking in still works.
//!   In the air the undone move keeps his height too, and an air attack coming down more than it
//!   lunges isn't gated at all: what is under him is a floor or a pit's trigger, which KH1's own
//!   fall would meet (`docs/kh1-water-pits.md`; an air combo had sunk through the Bottomless Hall's
//!   trigger with its event dropped).
//! - Face events Sora touches while KH2 has him are logged (`note_event`).
//! - Crates start the push themselves: their think (`0x1402C1450`, called from `0x1402C10E4`)
//!   counts 12 frames of Sora touching and facing them. While KH2 has him attacking (and a
//!   moment after), the crate is told nobody touched it (its `+8` cleared), so the count starts
//!   over; a push needs fresh contact after the combo. Its hits still slide or break it.

use super::addr::Rva;
use super::{hook, log, pose};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

const EXIT_CALL_RVA: Rva = Rva(0x2994F6);
const QUEUE_RVA: Rva = Rva(0x28BBF0);
const PUSH_CALL_RVA: Rva = Rva(0x2C10E4);
const PUSH_CHECK_RVA: Rva = Rva(0x2C1450);
/// Frames after an attack during which crates still don't count him (between two swings
/// KH2 can be out of its attack state for a few frames).
const PUSH_GRACE: u32 = 20;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ON: AtomicBool = AtomicBool::new(false);
/// Sora's actor during takeover's attack move; the face event he stood on before the attack.
static LUNGE: AtomicUsize = AtomicUsize::new(0);
static ALLOWED: AtomicU32 = AtomicU32::new(0);
static DROPPED: AtomicU32 = AtomicU32::new(0);
/// Sora's actor while KH2 has him attacking (crates), and frames left of the grace after.
static ATTACKER: AtomicUsize = AtomicUsize::new(0);
static GRACE: AtomicU32 = AtomicU32::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

/// Before `takeover`'s move of Sora (`attacking`: KH2's state is an attack this frame).
pub unsafe fn pre_move(actor: usize, attacking: bool) {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    if attacking {
        LUNGE.store(actor, Ordering::Relaxed);
    } else {
        ALLOWED.store(rd::<u32>(actor + 0x448), Ordering::Relaxed);
        LUNGE.store(0, Ordering::Relaxed);
    }
    DROPPED.store(0, Ordering::Relaxed);
}

/// After it: whether a face event was dropped (the move is to be undone).
pub fn post_move() -> Option<u32> {
    LUNGE.store(0, Ordering::Relaxed);
    let e = DROPPED.swap(0, Ordering::Relaxed);
    (e != 0).then_some(e)
}

/// Each KH2 frame (`takeover`): attacking or not, for the crates; `0` when KH1 has him.
pub fn attacking(actor: usize, now: bool) {
    if now {
        ATTACKER.store(actor, Ordering::Relaxed);
        GRACE.store(PUSH_GRACE, Ordering::Relaxed);
    } else if GRACE.load(Ordering::Relaxed) > 0 {
        if GRACE.fetch_sub(1, Ordering::Relaxed) == 1 {
            ATTACKER.store(0, Ordering::Relaxed);
        }
    } else {
        ATTACKER.store(0, Ordering::Relaxed);
    }
}

/// KH1 has him: no gate.
pub fn released() {
    ATTACKER.store(0, Ordering::Relaxed);
    GRACE.store(0, Ordering::Relaxed);
}

unsafe extern "system" fn queue(kind: i32, actor: usize, event: usize) {
    let lunge = LUNGE.load(Ordering::Relaxed);
    let drop = lunge != 0 && actor == lunge && kind == -2 && event as u32 & 0xFFFF != ALLOWED.load(Ordering::Relaxed) & 0xFFFF;
    if kind == -2 && actor == super::takeover::sora_actor() && super::takeover::kh2_has() {
        note_event(actor, event as u32 & 0xFFFF, drop);
    }
    if drop {
        DROPPED.store((event as u32 & 0xFFFF).max(1), Ordering::Relaxed);
        return;
    }
    if kind == -2 && event as u32 & 0xFFFF >= 100 && actor == super::takeover::sora_actor() {
        EXIT_EVENT.store(event as u32 & 0xFFFF, Ordering::Relaxed);
        EXIT_MS.store(clock_ms(), Ordering::Relaxed);
    }
    let f: unsafe extern "system" fn(i32, usize, usize) = std::mem::transmute(BASE.load(Ordering::Relaxed) + QUEUE_RVA);
    f(kind, actor, event)
}

/// The last face event logged and when (`note_event`).
static NOTED: AtomicU32 = AtomicU32::new(0);
static NOTED_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// The last face event of KH1's second area script (100 and up: room exits) queued for Sora,
/// KH2's or KH1's move, and when (`clock_ms`, 0 never).
static EXIT_EVENT: AtomicU32 = AtomicU32::new(0);
static EXIT_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn clock_ms() -> u64 {
    static CLOCK: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    CLOCK.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64 + 1
}

/// The room exit Sora last walked into (its face event, how many ms ago): the exit's script
/// starts from it (`camera::exit_owns`).
pub fn exit_touched() -> Option<(u32, u64)> {
    let at = EXIT_MS.load(Ordering::Relaxed);
    (at != 0).then(|| (EXIT_EVENT.load(Ordering::Relaxed), clock_ms().saturating_sub(at)))
}

/// A map face event Sora touched while KH2 has him (room exits, pits' triggers, scene walls): in
/// the log on a new event or every 2 s of the same, 200 at most (`docs/kh1-water-pits.md`).
unsafe fn note_event(actor: usize, event: u32, dropped: bool) {
    let now = clock_ms();
    let same = NOTED.swap(event, Ordering::Relaxed) == event;
    if same && now.saturating_sub(NOTED_MS.load(Ordering::Relaxed)) < 2000 {
        return;
    }
    NOTED_MS.store(now, Ordering::Relaxed);
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 200 {
        log(&format!(
            "map face event {event} touched while KH2 has Sora: {} ({} {:.0} above the floor, collision flags {:08x})",
            if dropped { "dropped (an attack's lunge)" } else { "queued" },
            if super::takeover::kh2_airborne() { "in the air," } else { "on the ground," },
            rd::<f32>(actor + 0x3D4),
            rd::<u32>(actor + 0x2D0)
        ));
    }
}

unsafe extern "system" fn push_check(object: usize) {
    let sora = ATTACKER.load(Ordering::Relaxed);
    if sora != 0 && object > 0x10000 && pose::ptr(rd::<u32>(object + 8)) == sora {
        std::ptr::write_unaligned((object + 8) as *mut u32, 0);
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) % 120 == 0 {
            log(&format!("a KH2 attack touched pushable {object:#x}: no push"));
        }
    }
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(BASE.load(Ordering::Relaxed) + PUSH_CHECK_RVA);
    f(object)
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    let exits = hook::redirect_call(base + EXIT_CALL_RVA, base + QUEUE_RVA, queue as *const () as usize, "map face events (KH2 lunges)");
    let push = hook::redirect_call(base + PUSH_CALL_RVA, base + PUSH_CHECK_RVA, push_check as *const () as usize, "crate push check (KH2 attacks)");
    ON.store(exits, Ordering::Relaxed);
    log(&format!("KH2 attacks: room exits {}, crate pushes {}", if exits { "kept out" } else { "not gated" }, if push { "kept out" } else { "not gated" }));
}
