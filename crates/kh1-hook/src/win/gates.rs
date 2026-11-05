//! Keeping KH2's attack lunges out of KH1's room exits (`docs/kh1-exits-push.md`). The user:
//! "a combo pushing you forward shouldnt bring you into a new room if you're by the entrance of a
//! room, also, if a combo pushes you forward into a box, it shouldnt go into soras push animation".
//!
//! - Exits are collision faces with an event number: KH1's mover queues the event for Sora each
//!   time a move touches one (the call at `0x1402994F6`, kind -2, Steam `0x14029B686`), whatever
//!   his state (Checked (docs: the decomp notes); KH1's attack states don't stop it). At that call
//!   (ours, `queue`), while one of our KH2 attacks plays on Sora (`kh2slots::attack_now`), a face
//!   event other than the one he was already touching when the attack began is refused, and the
//!   frame's move of KH1's physics step is undone (`takeover`'s wrapper of it, `undo`), so the lunge
//!   stops at the trigger's edge instead of resting inside it (the event fires on every touch, so
//!   a lunge left in it would fire on the first move after the attack). Walking in still works;
//!   KH1's own attacks (KH1's motions, the combo styles at KH1) are never refused.
//! - Crates: nothing of ours. A crate's own push check (`0x1402C1450`) counts Sora only while his
//!   actor flag `0x4` is clear, which KH1's set-motion sets for its action requests (bit
//!   `0x1000000`, KH1's row runner's swings); our KH2 swings run on KH1's row runner too (t363's
//!   `shared::physics::ground_step` tells a swing by that flag in state 0). Checked (docs, code).

use super::addr::Rva;
use super::{hook, log};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

const EXIT_CALL_RVA: Rva = Rva(0x2994F6);
const QUEUE_RVA: Rva = Rva(0x28BBF0);

static BASE: AtomicUsize = AtomicUsize::new(0);
/// The face event Sora touched last (from KH1's mover's queue) and the game frame
/// (`takeover::GAME_FRAME`) he last touched it: one he touches on every frame from before an
/// attack on is let through (a harmless event floor he stands on).
static TOUCHED: AtomicU32 = AtomicU32::new(0);
static TOUCHED_AT: AtomicU64 = AtomicU64::new(0);
/// A face event refused in this physics step (its move to be undone).
static DROPPED: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn queue(kind: i32, actor: usize, event: usize) {
    if kind == -2 && actor > 0x10000 && actor == super::takeover::sora_actor() {
        let e = event as u32 & 0xFFFF;
        let now = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
        let held = TOUCHED.load(Ordering::Relaxed) == e && now.saturating_sub(TOUCHED_AT.load(Ordering::Relaxed)) <= 1;
        if !held && super::kh2slots::attack_now(actor) {
            DROPPED.store(true, Ordering::Relaxed);
            static N: AtomicU32 = AtomicU32::new(0);
            if N.fetch_add(1, Ordering::Relaxed) % 60 == 0 {
                let p = std::ptr::read_unaligned((actor + 0x10) as *const [f32; 3]);
                log!(Takeover, "a KH2 attack touched map face event {e} at ({:.0}, {:.0}, {:.0}): refused, the move undone", p[0], -p[1], p[2]);
            }
            return;
        }
        TOUCHED.store(e, Ordering::Relaxed);
        TOUCHED_AT.store(now, Ordering::Relaxed);
    }
    let f: unsafe extern "system" fn(i32, usize, usize) = std::mem::transmute(BASE.load(Ordering::Relaxed) + QUEUE_RVA);
    f(kind, actor, event)
}

/// After KH1's physics step of Sora (`takeover::physics`): a face event was refused in it, so
/// he goes back to where the step began (`start`, his `+0x10` then: a spot KH1's own collision
/// resolved the frame before, short of the face).
pub unsafe fn undo(actor: usize, start: [f32; 4]) {
    if DROPPED.swap(false, Ordering::Relaxed) {
        std::ptr::write_unaligned((actor + 0x10) as *mut [f32; 4], start);
    }
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    let exits = hook::redirect_call(base + EXIT_CALL_RVA, base + QUEUE_RVA, queue as *const () as usize, "map face events (KH2 lunges)");
    log!(Takeover, "KH2 attacks: room exits {}", if exits { "kept out" } else { "not gated" });
}
