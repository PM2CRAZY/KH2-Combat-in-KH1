//! KH1's limits in a Drive Form (the owner, the main-branch port's round 9, row 19: answer 2 (b),
//! a Config toggle; t963). Drive > Limits in Forms: KH2 (the default) = no KH1 limit offered or
//! used in any form, as KH2, where a form takes the partners its limits need; KH1 = KH1's offer as
//! it is (Valor keeps refusing them, `formcmds::valor_allows`). Notes:
//! `docs/kh1-drive-forms-native.md` §20; the rule: `crate::formcmds`.
//!
//! Steam KH1, Checked (decomp) and (exe):
//! - **The offer** `0x14028D0F0 ()`: from KH1's context fill each frame, it puts one equipped limit
//!   in the Triangle slot (`0x140293000 (3, command)`). Nothing else offers a limit. Its first
//!   bytes are `48 81 EC 98 00 00 00` (`sub rsp, 0x98`). With the toggle on KH2, entering a form
//!   writes `ret` (`C3`) over its first byte and the revert puts `48` back (`hook::patch`), so KH1
//!   offers none while the form lasts: no code of ours runs each frame.
//! - A limit id that still reaches KH1's command step in a form (one offered on the frame the form
//!   started) is refused there as KH1 refuses an unusable command (sound 4; `combat::command_step`).
//! - Only with Drive on (the ini's `drive`) and on Steam.

use super::log;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

const OFFER_RVA: usize = 0x28D0F0;
const OFFER_BYTES: [u8; 7] = [0x48, 0x81, 0xEC, 0x98, 0x00, 0x00, 0x00];
const RET: u8 = 0xC3;

static BASE: AtomicUsize = AtomicUsize::new(0);
/// Drive > Limits in Forms: 0 not read yet, 1 KH1, 2 KH2.
static KH2: AtomicU8 = AtomicU8::new(0);
/// The offer is ours (`ret` written).
static OFF: AtomicBool = AtomicBool::new(false);
/// A form is on (as `form` told us).
static IN_FORM: AtomicBool = AtomicBool::new(false);

fn on() -> bool {
    super::settings::get().drive && super::addr::Build::current() == super::addr::Build::Steam
}

/// Drive > Limits in Forms on KH2 (the ini's `drive_limits = kh2`, the default).
pub fn kh2() -> bool {
    match KH2.load(Ordering::Relaxed) {
        0 => {
            let v = super::settings::get().drive_limits;
            KH2.store(1 + v as u8, Ordering::Relaxed);
            v
        }
        v => v == 2,
    }
}

/// Switches Drive > Limits in Forms (the Config menu), kept in the ini; a form now follows it.
pub fn set_kh2(v: bool) {
    KH2.store(1 + v as u8, Ordering::Relaxed);
    super::settings::save("drive_limits", if v { "kh2" } else { "kh1" });
    log!(Forms, "Config menu: Limits in Forms {}", if v { "KH2" } else { "KH1" });
    unsafe { apply() };
}

pub unsafe fn install(base: usize) {
    if on() && super::hook::prologue_is(base + OFFER_RVA, &OFFER_BYTES) {
        BASE.store(base, Ordering::Relaxed);
    } else if on() {
        log!(Forms, "Limits in Forms: KH1's limit offer at {:#x} isn't as expected; left alone", base + OFFER_RVA);
    }
}

/// A form started (`true`) or ended: KH1's offer follows (`form::native_start` / `native_revert`).
pub fn form_changed(in_form: bool) {
    IN_FORM.store(in_form, Ordering::Relaxed);
    unsafe { apply() };
}

/// KH1's Trinity Limit command (`docs/kh1-kh2-limits.md` §3: the offer's pair 15 / 0x52).
const TRINITY: u8 = 0x52;
/// Its bit in KH1's equipped-limit bits (`0x14299C240`, by the pairs' index: Trinity's is 4).
const TRINITY_BIT: u32 = 1 << 4;
const EQUIPPED_RVA: usize = 0x299C240;
/// KH1's recount of the equipped-limit bits from Sora's abilities (`0x14028D550`).
const RECOUNT_RVA: usize = 0x28D550;
const RECOUNT_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10];
/// KH1's bits are masked to Trinity's (`frame`).
static MASKED: AtomicBool = AtomicBool::new(false);

/// In Limit Form with Config > Drive > Limit > Trinity Limit on (t977): KH1's own offer stays, for
/// Trinity Limit alone.
fn trinity_only() -> bool {
    IN_FORM.load(Ordering::Relaxed) && super::form::in_limit() && super::limitform::trinity()
}

/// Whether a limit command is refused now (in a form, the toggle on KH2; Trinity Limit not in
/// Limit Form with its row on).
pub fn refused(id: u8) -> bool {
    crate::formcmds::limit(id) && IN_FORM.load(Ordering::Relaxed) && kh2() && BASE.load(Ordering::Relaxed) != 0 && !(id == TRINITY && trinity_only())
}

/// The Trinity row changed (`limitform`): the offer follows.
pub fn refresh() {
    unsafe { apply() };
}

/// The form's frame (`form::native_frame`): with Trinity alone allowed, KH1's equipped-limit bits
/// kept to Trinity's (KH1 recounts them from Sora's abilities, `0x14028D550`), so KH1's own offer
/// (`0x14028D0F0`) puts only Trinity Limit on Triangle, by KH1's own conditions (its data loaded,
/// both partners able within 500, a target, its MP). Out of it, KH1's recount puts them back.
pub unsafe fn frame() {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 {
        return;
    }
    if kh2() && trinity_only() {
        let at = base + EQUIPPED_RVA;
        let v = std::ptr::read_unaligned(at as *const u32);
        if v & !TRINITY_BIT != 0 {
            std::ptr::write_unaligned(at as *mut u32, v & TRINITY_BIT);
        }
        MASKED.store(true, Ordering::Relaxed);
    } else {
        unmask(base);
    }
}

/// KH1's equipped-limit bits from Sora's abilities again (KH1's own recount).
unsafe fn unmask(base: usize) {
    if MASKED.swap(false, Ordering::Relaxed) && super::hook::prologue_is(base + RECOUNT_RVA, &RECOUNT_BYTES) {
        let f: unsafe extern "system" fn() = std::mem::transmute(base + RECOUNT_RVA);
        f();
    }
}

/// KH1's offer off in a form with the toggle on KH2, else KH1's.
unsafe fn apply() {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 {
        return;
    }
    if !trinity_only() {
        unmask(base);
    }
    let at = base + OFFER_RVA;
    let want_off = IN_FORM.load(Ordering::Relaxed) && kh2() && !trinity_only();
    if want_off == OFF.load(Ordering::Relaxed) {
        return;
    }
    let done = if want_off {
        super::hook::patch(at, &OFFER_BYTES[..1], &[RET], "KH1's limit offer (off in a form)")
    } else {
        super::hook::patch(at, &[RET], &OFFER_BYTES[..1], "KH1's limit offer (back)")
    };
    if done {
        OFF.store(want_off, Ordering::Relaxed);
        log!(Forms, "Limits in Forms: KH1's limit offer {}", if want_off { "off for the form" } else { "back" });
    }
}
