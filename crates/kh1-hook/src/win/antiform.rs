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
//! - **No Keyblade, no trail** (main s272 "no weapon ... KH1's Keyblade is hidden", s273 "sora still
//!   has keyblade trails while in this form, lets not have that"): KH1 (Steam, Checked (decomp))
//!   draws an actor's weapon only while its `+0` bit 0x4000 is set (the party draw `0x1402A1630`),
//!   keeps a trail running only while `+0` has 0xC001 (the trail step `0x1402C1F20`), sets the bit
//!   only in its weapon out `0x1402A2E30 (actor, n)` (callers: the motion start `0x14029FC40` for a
//!   request of weapon mode 1, the event script's weapon ops `0x1401AA150` / `0x1401AA190`, three
//!   handlers `0x1401D8B70` / `0x1401D8CA0` / `0x1401D8DD0`; ours, t1062's `driveparty`), and clears
//!   it in `0x1402A3280` (a plain `& ~0x4000`). So: the bit cleared as `0x1402A3280` does when Anti
//!   arrives (`arrived`), KH1's weapon out wrapped to do nothing for Sora while in Anti (`weapon_out`),
//!   and KH1's weapon out (Sora, 1, as the motion start) when Anti ends if it was out (`left`). Anti's
//!   slots carry no trail level (`kh2slots::make`). KH1's own draw and trail step do the rest.

use super::log;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};

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
/// The secret code was entered (`form::code_read`): it waits, as a Drive request that waits, until
/// KH1 lets Sora change (`form::code_anti`, on the form's frames, `form::native_frame`). The code
/// clock's time it came, + 1 (0: none waiting).
static CODE: AtomicU64 = AtomicU64::new(0);
/// Why the code waits now (`form::code_anti`): 0 the form's frame hasn't run since (KH1's states
/// didn't have base Sora: a script, another body), 1 KH1's move has him and he can't change yet,
/// 2 KH1's own states don't have base Sora this frame. Logged once per code (`WAIT_LOGGED`) and
/// when the wait runs out.
static WAIT_WHY: AtomicU8 = AtomicU8::new(0);
static WAIT_LOGGED: AtomicBool = AtomicBool::new(false);

pub fn code_entered(now_ms: u64) {
    WAIT_WHY.store(0, Ordering::Relaxed);
    WAIT_LOGGED.store(false, Ordering::Relaxed);
    CODE.store(now_ms + 1, Ordering::Relaxed);
}

pub fn code_waiting() -> bool {
    CODE.load(Ordering::Relaxed) != 0
}

/// How long the waiting code has waited (None: none waiting).
pub fn code_age(now_ms: u64) -> Option<u64> {
    let at = CODE.load(Ordering::Relaxed);
    (at != 0).then(|| (now_ms + 1).saturating_sub(at))
}

/// The waiting code ended (taken, refused or cancelled): true if one was waiting.
pub fn code_clear() -> bool {
    CODE.swap(0, Ordering::Relaxed) != 0
}

/// The code waits this frame for `why` (1 or 2, `WAIT_WHY`): true the first time for this code (to log it).
pub fn code_wait(why: u8) -> bool {
    WAIT_WHY.store(why, Ordering::Relaxed);
    !WAIT_LOGGED.swap(true, Ordering::Relaxed)
}

/// Why the code was waiting, in words (`WAIT_WHY`).
pub fn code_wait_why() -> &'static str {
    match WAIT_WHY.load(Ordering::Relaxed) {
        1 => "KH1's move had him and never let him change",
        2 => "KH1's own states didn't have base Sora",
        _ => "KH1's states didn't have base Sora (a script moving him, another body, or KH2's files not loaded)",
    }
}

/// The code waited longer than `wait_ms` without KH1 letting him change: dropped (true once).
pub fn code_stale(now_ms: u64, wait_ms: u64) -> bool {
    let at = CODE.load(Ordering::Relaxed);
    at != 0 && now_ms + 1 > at + wait_ms && CODE.compare_exchange(at, 0, Ordering::Relaxed, Ordering::Relaxed).is_ok()
}

// ---------------------------------------------------------------------------------------------
// No Keyblade (module notes)

/// KH1's weapon out (Steam `0x1402A2E30 (actor, n)`) and its first bytes (`driveparty`'s, Checked (exe)).
const WEAPON_OUT_RVA: usize = 0x2A2E30;
const WEAPON_OUT_BYTES: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0x8B, 0xFA];
/// An actor's `+0` bit: its weapon is out (drawn, its trail kept).
const WEAPON_BIT: u32 = 0x4000;
static ORIG_WEAPON_OUT: AtomicUsize = AtomicUsize::new(0);
/// Sora's Keyblade was out when Anti arrived (it comes out again when Anti ends).
static WAS_OUT: AtomicBool = AtomicBool::new(false);

unsafe fn rd_u32(p: usize) -> u32 {
    std::ptr::read_unaligned(p as *const u32)
}
unsafe fn wr_u32(p: usize, v: u32) {
    std::ptr::write_unaligned(p as *mut u32, v)
}

/// KH1's weapon out wrapped (Steam only, its first bytes checked): nothing for Sora in Anti Form.
pub unsafe fn install(base: usize) {
    if super::addr::Build::current() != super::addr::Build::Steam {
        return;
    }
    if super::hook::install(base + WEAPON_OUT_RVA, &WEAPON_OUT_BYTES, weapon_out as *const () as usize, &ORIG_WEAPON_OUT, "weapon out (Anti Form)").is_none() {
        log!(Forms, "Anti Form: KH1's weapon out isn't where expected: the Keyblade may show in Anti");
    }
}

unsafe extern "system" fn weapon_out(actor: usize, n: i32) {
    let f: unsafe extern "system" fn(usize, i32) = std::mem::transmute(ORIG_WEAPON_OUT.load(Ordering::Acquire));
    if super::form::in_anti() && actor != 0 && actor == super::takeover::sora_actor() {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 6 {
            log!(Forms, "Anti Form: KH1's weapon out for Sora left undone (no Keyblade in Anti)");
        }
        return;
    }
    f(actor, n);
}

/// Anti has arrived (`form::native_arrive`): Sora's Keyblade away, as KH1's `0x1402A3280`.
pub unsafe fn arrived(sora: usize) {
    if sora < 0x10000 || !super::crash::readable(sora + 3) {
        return;
    }
    let f0 = rd_u32(sora);
    WAS_OUT.store(f0 & WEAPON_BIT != 0, Ordering::Relaxed);
    wr_u32(sora, f0 & !WEAPON_BIT);
    log!(Forms, "Anti Form: Sora's Keyblade away (it was {})", if f0 & WEAPON_BIT != 0 { "out" } else { "away" });
}

/// Anti has ended (`form::native_revert`, after the form is gone): Sora's Keyblade out again with
/// KH1's own weapon out (Sora, 1: the motion start's arguments) if it was out when Anti came.
pub unsafe fn left(sora: usize) {
    if !WAS_OUT.swap(false, Ordering::Relaxed) || sora < 0x10000 || !super::crash::readable(sora + 3) || rd_u32(sora) & WEAPON_BIT != 0 {
        return;
    }
    let base = super::takeover::game_base();
    if base == 0 || !super::hook::prologue_is(base + WEAPON_OUT_RVA, &WEAPON_OUT_BYTES) {
        log!(Forms, "Anti Form: Sora's Keyblade not brought out (KH1's weapon out not where expected)");
        return;
    }
    let out: unsafe extern "system" fn(usize, i32) = std::mem::transmute(base + WEAPON_OUT_RVA);
    out(sora, 1);
    log!(Forms, "Anti Form ended: Sora's Keyblade out again (KH1's weapon out)");
}
