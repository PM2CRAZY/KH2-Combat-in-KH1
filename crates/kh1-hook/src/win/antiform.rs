//! Anti Form's own rules (KH2's form 6, the main-branch port's row 14; research and KH2's numbers:
//! `docs/kh1-forms-research-r12.md`, Anti Form). The form itself runs on KH1's own states as the
//! other forms do (`form::Kind::Anti`, `kh2slots::ANTI_SPECS`).
//!
//! - **Rules** (Config > Drive > Anti > Rules; the owner's settled rule, `docs/main-port-inventory.md`:
//!   "Anti free / no revert goes behind the balance toggle"): KH2 (the default) or Balance (the
//!   owner's main-branch rule, s272: "I dont want it to cost any MP to enter, and I dont want sora
//!   to be able to revert out of it").
//!   - KH2: the Drive command greys by the cost of the form asked for and pays the cost of the form
//!     that comes out of the Anti roll (`0x140421D60`, `0x1403C2780`, Checked (decomp)); Anti's cmd
//!     0xD costs 9 (Checked (files)), which empties any gauge. The secret code stands in for a
//!     Drive (main s318: "9 bars, needs 3"), so it needs `NEEDS` bars, KH2's Valor / Wisdom cost
//!     (Assumption: the code stands for the cheapest Drive; main's 3), and pays 9. Revert greyed
//!     in battle only (`0x1403F29E0`, `0x1403F36C0`, Checked (decomp)).
//!   - Balance: free, and no Revert ever (the form gauge's end and a knock-out still revert, as
//!     main's: "reverts when empty or on a KO").
//!   - On MP (Drive > Form Cost MP) the KH2 rule is testing's 3 MP, as every form's.
//! - **The form gauge** (Checked (decomp + files)): Anti's level is sstm `+0x8C` (5) − 2 = 3
//!   (`0x1403C17C0`), so (3 + 3) × 600 = 3600 ticks; Drive orbs drain it instead of filling it
//!   (`0x1403C0B50`: in form 6 a gain is a loss).
//! - **Damage taken** × sstm `+0x94` (1.5), truncated, before the resist and Damage Control
//!   (`0x1403C12F0`, Checked (decomp + files)): `crate::drivesupport::anti_damage`.
//! - **No growth, no EXP** (no form-EXP adder for form 6, Checked (decomp)); no guard (no guard row in
//!   its PTYA, Checked (files)); no magic and no items (the cmd form mask, Magic off in Valor / Limit /
//!   Anti, Items in Anti only, Checked (files)).

use super::log;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// The bars the secret code needs on the KH2 rule (KH2's Valor and Wisdom cost, cmd `+0x1C`,
/// Checked (files); main's s318 "needs 3").
pub const NEEDS: u8 = 3;
/// Anti's form level for its form gauge (sstm `+0x8C` 5 − 2, `0x1403C17C0`, Checked (decomp + files)).
pub const LEVEL: u8 = 3;

/// Config > Drive > Anti > Rules: 0 not read yet, 1 KH2, 2 Balance.
static BALANCE: AtomicU8 = AtomicU8::new(0);

/// The owner's main-branch rules are on (the ini's `anti_rules = balance`; `kh2` by default).
pub fn balance() -> bool {
    match BALANCE.load(Ordering::Relaxed) {
        0 => {
            let v = super::settings::get().anti_balance;
            BALANCE.store(1 + v as u8, Ordering::Relaxed);
            v
        }
        v => v == 2,
    }
}

/// Switches Config > Drive > Anti > Rules, kept in the ini (a form under way keeps its payment).
pub fn set_balance(v: bool) {
    BALANCE.store(1 + v as u8, Ordering::Relaxed);
    super::settings::save("anti_rules", if v { "balance" } else { "kh2" });
    log!(Forms, "Config menu: Anti Form's rules {}", if v { "Balance (free, no Revert)" } else { "KH2" });
}

/// KH1's battle flag as the form's last frame saw it (`form::native_frame`).
static BATTLE: AtomicBool = AtomicBool::new(false);

pub fn note_battle(battle: bool) {
    BATTLE.store(battle, Ordering::Relaxed);
}

/// Revert can be chosen in Anti Form now: KH2's, out of battle only; Balance, never.
pub fn revert_allowed() -> bool {
    !balance() && !BATTLE.load(Ordering::Relaxed)
}

/// The Drive gauge bars the secret code needs now (Balance: none).
pub fn needs() -> u8 {
    if balance() {
        0
    } else {
        NEEDS
    }
}

/// Anti's entry paid on the Drive gauge (Balance: nothing): KH2's pay of its cost, 9 bars, the gauge
/// emptied when it holds fewer (`0x1403C2780`).
pub unsafe fn pay_gauge() -> bool {
    if balance() {
        log!(Forms, "Anti Form: free (Balance rules)");
        return true;
    }
    super::drivegauge::pay_all(crate::drivegauge::cost(crate::formlevel::form::ANTI), needs())
}
/// The secret code was entered (`form::pad`): taken on the form's next frame (`form::native_frame`).
static CODE: AtomicBool = AtomicBool::new(false);

pub fn code_entered() {
    CODE.store(true, Ordering::Relaxed);
}

pub fn code_waiting() -> bool {
    CODE.load(Ordering::Relaxed)
}

pub fn take_code() -> bool {
    CODE.swap(false, Ordering::Relaxed)
}
