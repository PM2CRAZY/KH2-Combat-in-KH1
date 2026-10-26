//! KH2's abilities on KH1's own moves (testing branch, shared states, base Sora). The user:
//! "when setting attack to kh1 sora is unable to use kh2s ground combo abilities (this includes
//! kh2s square combo modifers)", the same for the air combo, and "sora is unable to use kh2s
//! general modifiers (with the exception to aerial dodge)". `docs/shared-states.md`.
//!
//! With the Base / Aerial Combo style at KH1, KH1's own combo plays: its rows, motions and hits,
//! and its own abilities, which KH1 reads itself (Combo Plus, Air Combo Plus, Combo Master and
//! KH1's modifiers are the same KH1 abilities KH2's come from). KH2's abilities on top of it play
//! on KH1 rows of their own (`kh1rows`). There is no fallback (the tester, combo row selection
//! after m26, answer 1: "remove the fall back, and make them their own animations if they aren't
//! already"): every KH2 pick has its own KH1 row and slot, Finishing Plus's next finisher a row
//! free for it whatever finisher plays (`next_finisher`); a pick whose row didn't build at load
//! (`own_rows`, a missing file) is no attack on that press (logged), not KH1's row or button in
//! its place. Where none applies, KH1's own combo plays.
//!
//! - **Attack at the combo style KH1** (`picker`, from our hooks on KH1's own row pickers,
//!   `kh1picker`): KH1's picker chooses; KH2's ability rows are among its choices by KH2's own
//!   row checks at KH1's count, and `attack` runs the one it took. KH2's selector isn't asked.
//! - **Attack** (`attack`, from `combat::exec`: a press KH1's combo took, its row about to run;
//!   at the combo style KH2): KH2's combo is asked what it would take at KH1's count (`Character::would_take`, the combo
//!   carried at KH1's count, the press, KH2's targets and KH1's lock). KH2 takes it when that is
//!   - a row that needs an ability (the finisher Guard Break ...), opener, step or finisher,
//!     except the ones KH1's own rows now play natively at either combo style (`native_ability`:
//!     Slapshot, Dodge Slash, Slide Dash, Flash Step, Vicinity Break, Explosion, Aerial Sweep,
//!     Aerial Dive, Aerial Spiral, Aerial Finish, Magnet Burst);
//!   - KH2's finisher where KH1's would still swing, with Negative Combo;
//!   - KH2's step where KH1's would finish, KH2's combo being the longer (Combo Plus KH1 didn't
//!     count);
//!   - Finishing Plus: a press during KH1's finisher is KH2's next finisher (on a KH1 row of its
//!     own, `next_finisher`; KH1's own row when none can play it).
//! - **Square** in KH1's combo window (`control`): KH2's Square move there (Finishing Leap, Upper
//!   Slash, Horizontal Slash), if KH1's swing connected or Combo Master is on (as KH2's): on a
//!   KH1 row of its own (`kh1rows`), through KH1's own Attack step, no borrow.
//! - **Knocked away** in the air (`launched`, KH1's own reaction, `hurt`): the jump is KH2's
//!   Aerial Recovery, Square its Retaliating Slash, from the 16th frame (KH2's).
//! - **Quick Run** on Square while moving, with Dodge Roll at KH1 and no dodge roll of KH1's
//!   equipped (where KH2 would run, not roll): `shared::control`'s Square borrow.
//! - **After the move**, the next press in KH2's combo window (`combat::command`): KH1's next row
//!   with KH2's count carried (`next_from_kh2`, `kh1_count_in`), unless KH2's next is an ability
//!   move again; KH1's own states have him from there (`takeover::shared_now`).
//!
//! KH1's combo, from its Attack step `0x1402A5380` and row pickers `0x1402A4D50` (ground) /
//! `0x1402A4AE0` (air): the count is the byte `+0x41` of the combat info (`+0x14C`'s handle),
//! set to 1 outside a combo window, one more inside one (the actor's motion event flags `+0x1CC`
//! bit 22, `0x14029CE20`) when the swing connected (the info's `+0x50`, the attack's hit flags,
//! cleared as each attack row makes its attack object, `0x1402B10E0`) or Combo Master (65) is
//! equipped (else no swing); the finisher is picked once the count reaches the stats' (`+0x6C`)
//! `+0xD4` (ground) / `+0xD5` (air).

use super::addr::{Build, Rva};
use super::{log, pose, takeover};
use character_core::{Character, ComboRow, ComboState};
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::Mutex;

/// KH1's motion event flag: the combo window (`0x14029CE20`).
pub const KH1_COMBO_WINDOW: u32 = 0x40_0000;
const PARTY_RVA: Rva = Rva(0x2D37C80);
/// Frames into a knock-away before Aerial Recovery or Retaliating Slash (KH2's, 16), and the
/// longest a KH1 knock-away is taken for one.
const RECOVERY_AFTER: u64 = 16;
const BLOWN_MAX: u64 = 240;
/// Above the floor, KH1's world: in the air (`takeover`'s step down).
const AIR: f32 = 24.0;

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn note(s: impl FnOnce() -> String) {
    if !super::log_on(super::LogCat::Combat) {
        return;
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if super::log_on(super::LogCat::Combat) && N.fetch_add(1, Ordering::Relaxed) < 300 {
        log!(Combat, "{}", s());
    }
}

/// KH1's last swing of its own combo that ran: its count, its motion, whether it was KH1's
/// finisher, in the air.
static LAST: Mutex<Option<(u8, u32, bool, bool)>> = Mutex::new(None);
/// When KH1 last knocked him away itself (`hurt`, game frames).
static BLOWN_AT: AtomicU64 = AtomicU64::new(0);
/// Finishers in a row so far in KH1's combo (KH2's `+0xD75`, which Finishing Plus counts against:
/// `finisher_base` + Finishing Plus): 1 after a finisher, one more for each next finisher, 0 after
/// a swing.
static FINISHERS: AtomicU8 = AtomicU8::new(0);
/// A KH2 move on a KH1 row of its own (`kh1rows`) for KH1's Attack step this frame (`control`:
/// a Square move in KH1's combo window): ours, KH2's hop for it, the game frame.
static OURS: Mutex<Option<(usize, Option<character_core::Launch>, u64)>> = Mutex::new(None);

/// Our row for KH2's pick `row` (`kh1rows`), if it is one of ours and plays: the rows on Sora's
/// table and our slot of its motion on its request. By its PTYA row (the Square moves, Guard
/// Break, Counterguard...), else by its motion for every other combo row (`kh1rows::Ours::combo`:
/// the tester, combo row selection after m25, answer 3 (a), every KH2 combo row on a KH1 row of
/// its own).
fn ours_for(row: &ComboRow) -> Option<usize> {
    use super::kh1rows as k;
    // (Attack rows only: the guard's row 11 is started by its request, `guard_press`.)
    let plays = |n: usize| k::OURS[n].attack && k::OURS[n].motion == row.motion && k::installed() && super::kh2slots::has(k::request(n) as usize, row.motion);
    k::by_kh2_row(row.id).filter(|&n| plays(n)).or_else(|| k::by_combo_motion(row.motion).filter(|&n| plays(n)))
}

/// KH2's pick `pick` (in the air or not, its hop `launch`) on a combo row of ours (`ours_for`): the
/// row KH1's picker returns for it. A hop from the ground (an air row picked on the ground: rows 0
/// / 1, Aerial Sweep, Aerial Dive) needs KH2's hop worked out; else `None`. A plain hop (rows 0 /
/// 1, motions 182 / 181) plays on its own row whatever the Aerial Base Combo style (it is a move of
/// KH2's ground combo; the air combo after it is the Aerial Base Combo's): before, it needed the
/// Aerial Combo at KH2 too, and with it at KH1 KH1's row played instead (the tester, combo row
/// selection after m26, answer 1: no fallback).
fn combo_row(pick: &ComboRow, air: bool, launch: Option<character_core::Launch>) -> Option<usize> {
    let n = ours_for(pick)?;
    let o = &super::kh1rows::OURS[n];
    if !air && o.kind.air() && !o.rises {
        let hop = launch.is_some_and(|l| l.height.is_finite() && l.velocity.iter().all(|v| v.is_finite()));
        if !pick.is_jump() || !hop {
            return None;
        }
    }
    Some(n)
}

/// Our KH1 rows are on Sora's table (`kh1rows::installed`: built at load, `kh1_slots` on): every
/// KH2 pick plays on its own row, so a pick with none is no attack (no fallback, the tester, combo
/// row selection after m26, answer 1). With `kh1_slots = off` (a debug setting: nothing built)
/// KH2's plain picks keep the old path on KH1's plain rows (`kh2_row`, `select_swing`).
fn own_rows() -> bool {
    super::kh1rows::installed()
}

/// What `combat::exec` runs for a combo row of ours `n` (`combo_row`): from the ground an air row
/// is KH2's hop (KH1's air state entered with KH1's count at 1, KH2's: a hop from the ground
/// restarts it; then KH2's arc on it, `shared::physics::kh2_hop`), as `kh2_hop` ran it on KH1's
/// air swing row; else `ours_run`.
unsafe fn ours_combo_run(actor: usize, n: usize, launch: Option<character_core::Launch>, air: bool, count: u8, request_of: &dyn Fn(i32) -> Option<u32>) -> Run {
    use super::kh1rows as k;
    let o = &k::OURS[n];
    if let Some(l) = launch.filter(|_| !air && o.kind.air() && !o.rises) {
        let info = pose::ptr(rd::<u32>(actor + 0x14C));
        if info >= 0x10000 {
            wr(info + 0x41, 1u8);
        }
        let r = k::row(n);
        if let Ok(mut g) = LAST.lock() {
            *g = request_of(r).map(|m| (1, m, false, true));
        }
        FINISHERS.store(0, Ordering::Relaxed);
        note(|| format!("KH2's hop {} (motion {}) on KH1's air row {r} of its own (request {:#x}) out of KH1's air state, {:.0} high, {:.1}/frame sideways", o.name, o.motion, k::request(n), l.height, l.velocity[0].hypot(l.velocity[1])));
        return Run::Hop(r, l);
    }
    ours_run(n, launch, air, count, request_of)
}

/// What `combat::exec` runs for our row `n` (`kh1rows`) at this press: KH1's row runner on it,
/// its request our slot of KH2's motion; in the air with KH2's hop for the row (`air_hop`'s, from
/// `launch`); a row that leaves the ground by its root (`Ours::rises`, Finishing Leap) out of
/// KH1's air state, entered as for KH2's hops (`kh2_hop`) with no hop of its own (its root's
/// rise moves him, `kh2slots::air_root`). KH1's last swing noted for Finishing Plus (`LAST`).
fn ours_run(n: usize, launch: Option<character_core::Launch>, air: bool, count: u8, request_of: &dyn Fn(i32) -> Option<u32>) -> Run {
    let o = &super::kh1rows::OURS[n];
    let r = super::kh1rows::row(n);
    if let Ok(mut l) = LAST.lock() {
        *l = request_of(r).map(|m| (count, m, o.kind.finisher(), air || o.rises));
    }
    // (A next finisher's caller counts it on, `next_finisher`.)
    if !o.next_finisher {
        FINISHERS.store(o.kind.finisher() as u8, Ordering::Relaxed);
    }
    note(|| format!("KH2's {} (row {}, motion {}) on KH1's row {r} of its own (request {:#x}){}", o.name, o.kh2_row, o.motion, super::kh1rows::request(n), if o.rises && !air { ", out of KH1's air state" } else { "" }));
    if o.rises && !air {
        // (Its first frames before its no-gravity range held off the floor, `kh2slots::rise_start`.)
        super::kh2slots::start_rise(o.motion);
        let gravity = pose::kh2().map_or(0.0, |k| k.character.params.gravity);
        return Run::Hop(r, character_core::Launch { height: 0.0, velocity: [0.0; 2], gravity });
    }
    match launch.filter(|l| air && o.kind.air() && l.height.is_finite() && l.velocity.iter().all(|v| v.is_finite())) {
        Some(l) => Run::AirHop(r, l),
        None => Run::Row(r),
    }
}

/// Finishing Plus's next finisher on KH1's rows (the tester, m14 question 4: "(b)", a new row;
/// `kh1rows::Ours::next_finisher`): KH2's pick `row` at the press in KH1's finisher (KH2's
/// Finishing Plus window open, `kh2slots::finisher_window`, KH1's Attack step having taken it)
/// runs on
/// - its own KH1 row if it has one (Guard Break, `kh1rows`), its request not the one playing;
/// - else (KH2 picked the finisher playing again: a row whose `sub` is its own id passes the
///   walk's anti-repeat, Steam KH2 `0x140402C50`) our next-finisher row of this kind (ground or
///   air), our slot of KH2's motion picked on its request (`kh2slots::pick_own`), when its request
///   isn't the one playing. That row has a slot of every KH2 finisher of its kind (`kh2slots`'s
///   `prepare`: the ground 154, 155, Explosion, Guard Break; the air 183, 184, Aerial Finish,
///   Magnet Burst), so with the finisher's own row playing it is free for any pick, and with it
///   playing the pick's own row is free: there is always a row of ours for the next finisher.
/// In the air with KH2's hop for the row. `None` only when the pick's slot didn't build at load (no
/// attack then; before, KH1's own finisher row played KH2's motion here, or KH1's own row).
fn next_finisher(actor: usize, row: &ComboRow, launch: Option<character_core::Launch>, air: bool, count: u8, playing: u32, request_of: &dyn Fn(i32) -> Option<u32>) -> Option<Run> {
    use super::kh1rows as k;
    let _ = actor;
    if let Some(n) = ours_for(row).filter(|&n| k::request(n) != playing) {
        return Some(ours_run(n, launch, air, count, request_of));
    }
    let n = k::next_finisher(air).filter(|&n| k::installed() && k::request(n) != playing)?;
    unsafe { super::kh2slots::pick_own(k::request(n) as usize, row.motion) }.then(|| ours_run(n, launch, air, count, request_of))
}

/// This frame's move of ours for KH1's Attack step (`control` armed it), if any: one left from an
/// earlier frame is dropped.
fn take_ours() -> Option<(usize, Option<character_core::Launch>)> {
    let (n, l, at) = OURS.try_lock().ok().and_then(|mut g| g.take())?;
    (frame().saturating_sub(at) <= 1).then_some((n, l))
}

fn frame() -> u64 {
    takeover::GAME_FRAME.load(Ordering::Relaxed)
}

unsafe fn sora() -> usize {
    rd::<u64>(takeover::game_base() + PARTY_RVA) as usize
}

/// The Base / Aerial Base Combo style for KH1's own combo where he is: the Config row's, but always
/// KH2's in a Drive Form on KH1's own states (`form::native_now`: a Drive Form is KH2's all through
/// and the move styles don't apply to it; the tester, what-is-still-missing after m13: "our
/// configs don't adjust anything on drive forms"). Before, the raw setting was read here, so with
/// a combo style at KH1 Valor's pick from KH1's picker (KH2's, `kh1picker::at_kh2`) was dropped
/// at the Attack step: its air rows ran with no hop (the jump's rise went on under the swing) and
/// its ground rows lost KH2's length and finisher rule.
fn style_kh2(kind: super::styles::Kind) -> bool {
    super::styles::kh2(kind) || super::form::native_now()
}

/// KH1's combo now: its count, its length here (ground or air), and whether its swing connected.
/// At the combo style KH1 the length is KH1's less Negative Combo's copies (`kh1_length`), as its
/// row picker sees it (`kh1picker`).
unsafe fn kh1_combo(actor: usize, air: bool) -> Option<(u8, u8, bool)> {
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    let stats = pose::ptr(rd::<u32>(actor + 0x6C));
    if info < 0x10000 || stats < 0x10000 {
        return None;
    }
    let len = rd::<u8>(stats + if air { 0xD5 } else { 0xD4 });
    let kind = if air { super::styles::Kind::AirCombo } else { super::styles::Kind::GroundCombo };
    let len = if style_kh2(kind) { len } else { kh1_length(len) };
    Some((rd::<u8>(info + 0x41), len, rd::<u32>(info + 0x50) != 0))
}

/// Negative Combo on KH1's own combo (the combo style KH1; the tester, m19: "the ability should
/// not ignore the combat settings"): KH1's length (the stats' `+0xD4` / `+0xD5`, Combo Plus and
/// Air Combo Plus already in it) less one a copy, at least 1, as KH2's length takes it off its own
/// (`Character::combo_length`: base and Combo Plus less Negative Combo, at least 1).
pub fn kh1_length(len: u8) -> u8 {
    if len == 0 {
        return len;
    }
    let negative = takeover::with_abilities(|a| a.negative_combo).unwrap_or(0);
    len.saturating_sub(negative).max(1)
}

/// Finishing Plus's chain: another finisher is left (KH2's count, `+0xD75` against one plus
/// Finishing Plus's copies; `FINISHERS`), at both combo styles. KH2 counts it in its Attack and
/// command check (Steam KH2 `0x1403D6CA0`: the finisher window taken only while `+0xD75` is under
/// the maximum, Checked (decomp)), not in its row selection, so the KH2 setting needs the same
/// count as the KH1 setting. Before t641 the KH2 setting returned "always" here (it took KH2's
/// pick to count them, which it doesn't), so with Finishing Plus every KH2 finisher opened its
/// window again and Sora could attack out of his finishers forever (the tester, combo row
/// selection after m28: "if you set combat to kh2 for both settings you can attack out of your
/// finisher indefinitely, it needs to behave exact like kh1").
pub fn finisher_chain_open() -> bool {
    let plus = takeover::with_abilities(|a| a.finishing_plus).unwrap_or(0);
    FINISHERS.load(Ordering::Relaxed).max(1) < 1 + plus
}

/// Finishing Plus's next finisher is left (KH2's own test in its Attack and command check, Steam
/// KH2 `0x1403D6CA0`: the finisher window, range id 50, taken only while the finisher count
/// `+0xD75` is under the maximum, `[+0x5C0] + 0x202`; Checked (decomp)), whatever the combo
/// style: the finishers so far (`FINISHERS`, at least 1) under one plus Finishing Plus's copies
/// (the maximum's reading as `finisher_chain_open` takes it), Finishing Plus equipped.
pub fn finisher_left() -> bool {
    let plus = takeover::with_abilities(|a| a.finishing_plus).unwrap_or(0);
    plus > 0 && FINISHERS.load(Ordering::Relaxed).max(1) < 1 + plus
}

/// A KH2 cast on KH1's cast state took its step of KH1's combo (`formmagic::take_step`): noted as
/// a swing of the combo is (`LAST`, `FINISHERS`), so Finishing Plus's next finisher can follow a
/// finisher cast as it follows a finisher swing (the tester, kh2-magic-and-reflect-native after
/// m10, question 2 (a)). `request`: KH1's request the cast plays on (its `+0x164` while it plays);
/// `finisher`: a finisher cast; `chained`: taken in a finisher's Finishing Plus window (KH2's
/// `+0xD75` one more, `0x1403D6CA0`), else the combo's first finisher (`+0xD75` 1, KH2's new
/// combo `0x101`) or no finisher (0, as after a swing).
pub fn cast_step(step: u8, request: u32, finisher: bool, chained: bool, air: bool) {
    if let Ok(mut l) = LAST.lock() {
        *l = Some((step, request, finisher, air));
    }
    let f = if !finisher { 0 } else if chained { FINISHERS.load(Ordering::Relaxed).max(1).saturating_add(1) } else { 1 };
    FINISHERS.store(f, Ordering::Relaxed);
}

/// KH2's normal finishers whose Finishing Plus window KH1's own finishers take (the tester, m20
/// answer 4 (a): "KH2's own finisher window from 155 / 183, placed at the same point in KH1's
/// finisher"): 155 on the ground (its window 35..64), 183 in the air (21..50; Checked (files)).
const KH1_FINISHER_WINDOW_FROM: [u16; 2] = [155, 183];

/// Finishing Plus on KH1's own finishers (Base / Aerial Base Combo KH1; the tester, m22: "make
/// sure every kh1 finisher can go into other kh1/kh2 finishers"): while KH1's own finisher motion
/// plays on Sora (its plain finisher 0xCB / 0xCE or one of its ability finishers, the last row
/// KH1's Attack step ran being a finisher, `LAST`, its request the one playing, no KH2 motion of
/// ours on it), Finishing Plus equipped and another finisher left (`finisher_chain_open`), KH1's
/// combo window (`+0x1CC` bit 22, through the instance's `+0x14`, as `kh2slots::finisher_window`
/// opens it on KH2's) is open at the same share of the motion as KH2's 155 / 183 Finishing Plus
/// window is of its clip: KH1's time over its length (`+0x16C` / `+0x280`, the share KH1 itself
/// works out in `0x14029FF80`, Checked (decomp)) × the KH2 clip's length. KH1's command menu then
/// takes an Attack and its Attack step counts it on past the length; KH1's picker's finisher walk
/// picks the next finisher (KH1's, KH2's, or KH2's normal one when KH1's is the one playing).
/// Placing KH2's window by share is an **Assumption** (KH1's finishers have none of their own).
pub unsafe fn kh1_finisher_window(actor: usize) {
    if Build::current() != Build::Steam || actor < 0x10000 || !super::shared::active() || super::form::in_form() {
        return;
    }
    let Some((_, request, true, air)) = LAST.try_lock().ok().and_then(|l| *l) else { return };
    let kind = if air { super::styles::Kind::AirCombo } else { super::styles::Kind::GroundCombo };
    if request & 0xFFFF != rd::<u32>(actor + 0x164) & 0xFFFF || super::kh2slots::playing_motion(actor).is_some() {
        return;
    }
    // (At the combo style KH2 a KH1 finisher plays only when the KH2 setting's walk took one of
    // KH1's own, `kh2_finisher_walk`; another finisher is left by KH2's count, `+0xD75` against
    // one plus Finishing Plus's copies.)
    let plus = takeover::with_abilities(|a| a.finishing_plus).unwrap_or(0);
    let left = if super::styles::kh2(kind) { FINISHERS.load(Ordering::Relaxed).max(1) < 1 + plus } else { finisher_chain_open() };
    if plus == 0 || !left {
        return;
    }
    let (t, len) = (rd::<f32>(actor + 0x16C), rd::<f32>(actor + 0x280));
    if !(t >= 0.0 && len > 0.0) {
        return;
    }
    let from = KH1_FINISHER_WINDOW_FROM[air as usize];
    let Some(clip) = pose::kh2().and_then(|k| k.character.moveset.clips.get(&from)) else { return };
    let at = (t / len * clip.length as f32) as u32;
    if !clip.has(at, |k| matches!(k, character_core::WindowKind::Finisher)) {
        return;
    }
    let inst = pose::ptr(rd::<u32>(actor + 0x15C));
    if inst < 0x10000 {
        return;
    }
    wr(inst + 0x14, rd::<u32>(inst + 0x14) | KH1_COMBO_WINDOW);
    static N: AtomicU32 = AtomicU32::new(0);
    if super::log_on(super::LogCat::Combat) && N.fetch_add(1, Ordering::Relaxed) < 20 {
        log!(Combat, "Finishing Plus window on KH1's own finisher (request {:#x}, time {t:.1} of {len:.1}, KH2 {from}'s frame {at}): KH1's combo window", request & 0xFFFF);
    }
}

/// Whether KH2's pick is one of its ability moves over KH1's combo at `count` (what makes it so,
/// for the log).
fn ability_move(row: &ComboRow, kh2_len: u8, negative: u8, count: u8, kh1_len: u8) -> Option<&'static str> {
    let kh1_finisher = count >= kh1_len;
    if row.ability != 0 && !native_ability(row) {
        Some("an action ability's move")
    } else if row.is_finisher() && !kh1_finisher && negative > 0 {
        Some("KH2's finisher a swing sooner (Negative Combo)")
    } else if !row.is_finisher() && kh1_finisher && kh2_len > kh1_len {
        Some("KH2's combo goes on past KH1's (Combo Plus)")
    } else {
        None
    }
}

/// An ability row of KH2's that KH1's own states play natively, no borrow (the tester: "transfer
/// everything into kh1's systems"; the KH2-controller borrow is going away): a ground row that is
/// only a Keyblade swing (`kh2slots::swing_rows`, which builds its motion's slots only then),
/// whatever the combo styles (the tester, "transfer everything that is not on the new system
/// yet", the Base / Aerial Combo KH1 cases: an ability's move is KH2's at either style, as its
/// borrow was; the style picks KH1's or KH2's plain combo around it, `Kh1Combo::here`). A swing (Slapshot, Dodge Slash, Slide Dash, Flash
/// Step, its guard on his body taken by KH1's hit pass, `hurt`): KH1's plain ground swing row runs
/// it as it runs KH2's plain swings (`kh2_row`, `select_swing`): KH1's row, attack object, damage
/// and windows, KH2's motion and root travel. A hop (Aerial Sweep,
/// Aerial Dive): KH1's air state and air swing row with KH2's
/// hop, as KH2's plain hop (`kh2_hop`). An air row (Aerial Spiral, the finisher Aerial Finish):
/// KH1's air swing or finisher row with KH2's motion and its own
/// hop, as KH2's plain air rows (`air_hop`). Their no-gravity ranges are held on KH1's air state
/// (`kh2slots::no_gravity`). A move that hits with shapes on his body (Vicinity Break on the
/// ground swing rows, Explosion on the ground finisher row, Magnet Burst on the air finisher row,
/// `kh2slots::body_strike`): the same, with KH2's own shapes and hit numbers for those windows
/// landed through KH1's damage (`takeover::kh2_hits`) and its invulnerability in KH1's hit pass
/// (`hurt`). Each now plays on a KH1 row of its own (`kh1rows::Ours::combo`).
fn native_ability(row: &ComboRow) -> bool {
    row.ability != 0 && super::kh2slots::has_any(row.motion)
}

/// What `combat::exec` runs for an Attack KH1's own combo took (`attack`).
pub enum Run {
    /// No row on this press (Finishing Plus's last finisher already ran: KH2's rule, nothing after
    /// it until it ends; or KH2's pick has no KH1 row built for it, `own_rows`).
    Nothing,
    /// This KH1 row (its own, or its plain one KH2's row selection took, `kh2_row`).
    Row(i32),
    /// KH2's hop (`kh2_hop`): KH1's air state entered, this KH1 air row run, then KH2's arc and
    /// speed on it (`shared::physics::kh2_hop`).
    Hop(i32, character_core::Launch),
    /// KH2's air swing's own hop (`air_hop`): this KH1 air row run in KH1's air state, then KH2's
    /// row's arc and speed replace the row's own lunge and hop (`shared::physics::kh2_hop`).
    AirHop(i32, character_core::Launch),
}

/// An Attack KH1's own combo took (`combat::exec`, base Sora in KH1's own states, the combo style
/// KH1's where he is, KH1's `row` about to run): `Run::Nothing` for no attack, else the KH1 row
/// to run: `row`, or KH1's plain swing or finisher row in
/// its place where KH2's row selection takes that (`kh2_row`), or KH2's hop on KH1's air state
/// (`kh2_hop`). `request_of`: a KH1 row's motion request (its `+0`).
pub unsafe fn attack(actor: usize, row: i32, _dir: usize, request_of: &dyn Fn(i32) -> Option<u32>) -> Run {
    // (On the ground: KH2's line ahead, for its row 1, tested in KH2's trial along the
    // selector's direction, `drop_ahead`.)
    takeover::test_drop_ahead(rd::<i32>(actor + 0x70) == 0 && style_kh2(super::styles::Kind::GroundCombo));
    let run = attack_row(actor, row, request_of);
    takeover::test_drop_ahead(false);
    run
}

unsafe fn attack_row(actor: usize, row: i32, request_of: &dyn Fn(i32) -> Option<u32>) -> Run {
    // (KH1's ground state or its jump / fall: not its flight, 8, nor its other states.)
    let state = rd::<i32>(actor + 0x70);
    if !matches!(state, 0..=2) || super::form::driver_form() {
        return Run::Row(row);
    }
    let air = matches!(state, 1 | 2);
    // (KH2's pick KH1's picker made for this row, taken here so none is left for a later press.)
    let made = take_kh2_pick(row, air);
    let Some((count, kh1_len, _)) = kh1_combo(actor, air) else { return Run::Row(row) };
    // A Square move of KH2's on its own KH1 row (`control` / `valor_square` gave KH1 this Attack
    // for it). An air row's hop from the ground (Valor's row 11, motion 181 with its hop of 90:
    // `valor_square`) is entered as KH2's plain hops are (`ours_combo_run`); every other row as
    // before (`ours_run`).
    if let Some((n, launch)) = take_ours() {
        return ours_combo_run(actor, n, launch, air, count, request_of);
    }
    // Counterguard: Attack during KH2's guard's repel (KH2's own rule, Steam KH2's Hurt step on
    // motion 42; `counter_window` opened KH1's combo window for it), on its own KH1 row.
    if let Some(run) = counterguard(actor, air, count, request_of) {
        return run;
    }
    // One of KH1's own ability finishers the KH2 setting's walk took in KH1's picker
    // (`kh2_finisher_walk`): KH1's row runs whole.
    if let Some((n, chained)) = take_kh1_take().filter(|&(n, _)| n == row) {
        if let Ok(mut l) = LAST.lock() {
            *l = request_of(n).map(|m| (count, m, true, air));
        }
        let f = if chained { FINISHERS.load(Ordering::Relaxed).max(1).saturating_add(1) } else { 1 };
        FINISHERS.store(f, Ordering::Relaxed);
        return Run::Row(n);
    }
    let last = LAST.try_lock().ok().and_then(|l| *l);
    // Finishing Plus: a press during KH1's finisher (its motion still playing) is KH2's next one.
    // (At the combo style KH1 KH1's own picker picks it: below.)
    let playing = rd::<u32>(actor + 0x164) & 0xFFFF;
    let kh2_style = style_kh2(if air { super::styles::Kind::AirCombo } else { super::styles::Kind::GroundCombo });
    // At the combo style KH2, KH2's pick KH1's picker made at this press (`kh2_pick_in_picker`;
    // the tester, m24: "(a)"): KH2's selector isn't run again here, and the row KH1's picker
    // returned already plays it. Without one (KH1's picker didn't make it), KH2's selector is asked
    // here, as before.
    let made = if kh2_style { made } else { None };
    if let Some((n, _, true, _)) = last.filter(|l| kh2_style && l.3 == air && l.1 & 0xFFFF == playing) {
        let finishers = FINISHERS.load(Ordering::Relaxed).max(1);
        // (The last finisher already ran: nothing after it until it ends, as at the KH1 setting
        // and in KH2, `finisher_chain_open`; not a new combo out of the finisher.)
        if !finisher_chain_open() {
            note(|| format!("Attack in KH1's finisher (count {n}, finisher {finishers}): no next finisher left (Finishing Plus): no attack"));
            return Run::Nothing;
        }
        let from = ComboState { count: n, finishers, hit: true, finisher_next: true };
        let chained = match &made {
            Some(p) => p.as_ref().filter(|p| p.chained).map(|p| (p.row.clone(), p.len, p.neg, p.launch)),
            None => takeover::kh2_would_take_launch(actor, Some(from), super::kh2slots::playing_motion(actor), air, false).filter(|(r, ..)| r.is_finisher()),
        };
        if let Some((next, _, _, launch)) = chained {
            note(|| format!("Attack in KH1's finisher (count {n}, finisher {finishers}): KH2's next finisher, row {} motion {} (Finishing Plus)", next.id, next.motion));
            if let Some(run) = next_finisher(actor, &next, launch, air, n, playing, request_of) {
                FINISHERS.store(finishers.saturating_add(1), Ordering::Relaxed);
                return run;
            }
            // Its slot didn't build at load (a row of ours is always free for it otherwise,
            // `next_finisher`): no attack, no fallback (the tester, combo row selection after m26,
            // answer 1: "remove the fall back").
            note(|| format!("KH2's next finisher row {} motion {}: its KH1 row of its own isn't built: no attack", next.id, next.motion));
            return Run::Nothing;
        }
    }
    let combo = if air { &AIR_COMBO } else { &GROUND_COMBO };
    // At the combo style KH1: KH1's own row picker chose (`picker`, KH2's ability rows among its
    // choices by KH2's own row checks at KH1's count); KH2's selector isn't asked. Its pick, if it
    // took one of KH2's ability moves at this press.
    let kh1_style = !style_kh2(combo.style);
    let (pick, launch) = if kh1_style {
        match take_pick() {
            Some((p, l)) => (Some((p, kh1_len, 0)), l),
            None => (None, None),
        }
    } else if let Some(p) = &made {
        // (KH1's picker's pick; a Finishing Plus one was taken above.)
        match p.as_ref().filter(|p| !p.chained) {
            Some(p) => (Some((p.row.clone(), p.len, p.neg)), p.launch),
            None => (None, None),
        }
    } else {
        // KH2's combo at KH1's count (this press counted: `count` is this swing's).
        let from = ComboState { count: count.saturating_sub(1), finishers: 1, hit: true, finisher_next: false };
        // (KH2's motion KH1 plays now for its swing: KH2's anti-repeat passes over it, as over its
        // own swing playing.)
        let taken = takeover::kh2_would_take_launch(actor, Some(from), super::kh2slots::playing_motion(actor), air, false);
        let launch = taken.as_ref().and_then(|t| t.3);
        (taken.map(|(r, len, neg, _)| (r, len, neg)), launch)
    };
    // A Drive Form on KH1's own states (`form::native_now`): KH2's pick or no attack. KH2 refuses a
    // press its rows don't take (Steam KH2's Attack step `0x1403D6CA0` / row walk `0x140402D20`,
    // Checked (decomp)): after the air combo's last swing or finisher nothing more until he lands
    // (the count stays at the length in the air). KH1's own row ran in its place here (the row its
    // picker took, with KH1's own air hop of 40 / 20 and lunge 8 / 12 / 4 under Valor's motion), so
    // each press after Valor's air finisher hopped him higher (the tester: "attack doesn't
    // interrupt jumps momentum so you can fly high in the air while attacking"). Valor has no KH1
    // combo of its own (KH1's ability finishers held back, `kh1picker::at_kh2`).
    if super::form::native_now() && pick.is_none() {
        note(|| format!("KH1's {} combo in a Drive Form, swing {count} of {kh1_len}: KH2 takes nothing: no attack (KH1's row {row} not run)", if air { "aerial" } else { "ground" }));
        return Run::Nothing;
    }
    // (Where KH1's own plain rows play KH2's pick, at the combo style KH2, a pick past KH1's
    // length (Combo Plus) or a finisher before it (Negative Combo) is KH2's row selection on
    // KH1's rows, `kh2_row`, not a borrow.)
    if kh1_style {
        // Finishing Plus at the combo style KH1: a press in one of KH2's ability finishers'
        // Finishing Plus window (`kh2slots::finisher_window`, open while `finisher_chain_open`),
        // which KH1's Attack step counted on past KH1's length: the KH2 finisher KH1's picker
        // took, on a KH1 row of ours whose request isn't the one playing (`next_finisher`: its own
        // row or our next-finisher row), or KH1's own finisher it took (its plain one or one of
        // its abilities', not a swing: its fallback when its finisher's request is the one
        // playing); the last finisher already run, or KH2's pick with no row built, no attack
        // (KH2's: nothing after the last finisher until it ends; no fallback onto KH1's row, the
        // tester, after m26, answer 1).
        if let Some((_, _, true, _)) = last.filter(|l| l.3 == air && l.1 & 0xFFFF == playing && count >= kh1_len) {
            let finishers = FINISHERS.load(Ordering::Relaxed).max(1);
            // (KH1's own finisher its picker took, not a swing of its combo, its request not the
            // one playing.)
            let own = || {
                request_of(row).filter(|&q| !combo.swings.contains(&(q & 0xFFFF)) && q & 0xFFFF != playing).map(|q| {
                    if let Ok(mut l) = LAST.lock() {
                        *l = Some((count, q, true, air));
                    }
                    let _ = select_swing(combo, Some(q), None);
                    Run::Row(row)
                })
            };
            let kh2_next = pick.as_ref().filter(|(r, ..)| r.is_finisher());
            let mut ran_kh2 = false;
            let run = if !finisher_chain_open() {
                None
            } else if let Some((r, ..)) = kh2_next {
                let r = next_finisher(actor, r, launch, air, count, playing, request_of);
                ran_kh2 = r.is_some();
                r
            } else {
                own()
            };
            note(|| format!(
                "Attack in KH1's finisher at the KH1 setting (count {count}, finisher {finishers}, Finishing Plus): {}",
                match (&run, kh2_next) {
                    (None, Some((r, ..))) if finisher_chain_open() => format!("KH2's finisher row {} motion {}: its KH1 row of its own isn't built: no attack", r.id, r.motion),
                    (None, _) => "no next finisher (the last one, or KH1's own finisher's request the one playing): no attack".to_string(),
                    (Some(_), Some((r, ..))) if ran_kh2 => format!("KH2's finisher row {} motion {} from KH1's picker", r.id, r.motion),
                    (Some(_), _) => format!("KH1's own row {row} (not a swing of its combo)"),
                }
            ));
            let Some(run) = run else { return Run::Nothing };
            FINISHERS.store(finishers.saturating_add(1), Ordering::Relaxed);
            return run;
        }
        // (One with a KH1 row of its own, `kh1rows`: every combo row, and Guard Break.)
        if let Some(n) = pick.as_ref().and_then(|(r, ..)| combo_row(r, air, launch)) {
            return ours_combo_run(actor, n, launch, air, count, request_of);
        }
        if let Some(n) = pick.as_ref().and_then(|(r, ..)| ours_for(r)) {
            return ours_run(n, launch, air, count, request_of);
        }
        // (KH1's picker offers only KH2's rows that have a KH1 row, `kh1_plays`: a pick here with
        // none is a hop whose KH2 launch didn't work out, or a row lost since.)
        if let Some((r, ..)) = pick.as_ref() {
            note(|| format!("KH1's {} row picker took KH2's row {} motion {}, but no KH1 row of its own plays it now: no attack", if air { "aerial" } else { "ground" }, r.id, r.motion));
            return Run::Nothing;
        }
    } else if let Some(n) = pick.as_ref().and_then(|(r, ..)| combo_row(r, air, launch)) {
        // At the combo style KH2: KH2's pick on its own KH1 row (the tester, combo row selection
        // after m25, answer 3 (a)), the row KH1's picker returned for it (`kh2_pick_in_picker`):
        // KH1's row runner runs it, KH1's plain rows not re-pointed at it.
        let (r, len, _) = pick.as_ref().unwrap();
        note(|| format!(
            "KH1's {} combo, swing {count} of {kh1_len}: KH2's row {} motion {} (ability {}, finisher {}, KH2's length {len}) on KH1's row {} of its own",
            if air { "aerial" } else { "ground" },
            r.id,
            r.motion,
            r.ability,
            r.is_finisher(),
            super::kh1rows::row(n)
        ));
        return ours_combo_run(actor, n, launch, air, count, request_of);
    }
    let why = if kh1_style { None } else { pick.as_ref().filter(|(r, ..)| !combo.serves(r)).and_then(|(r, len, neg)| ability_move(r, *len, *neg, count, kh1_len)) };
    if !kh1_style {
        note(|| format!(
            "KH1's {} combo, swing {count} of {kh1_len}: KH2 would take {}{}",
            if air { "aerial" } else { "ground" },
            pick.as_ref().map_or("nothing".to_string(), |(r, len, _)| format!("row {} motion {} (ability {}, finisher {}, KH2's length {len})", r.id, r.motion, r.ability, r.is_finisher())),
            why.map_or(String::new(), |w| format!(": {w}"))
        ));
    }
    // No KH1 row of its own for KH2's pick (its slot didn't build at load; a pick always has one
    // otherwise, `kh1rows::Ours::combo`): no attack (the tester, combo row selection after m26,
    // answer 1: "remove the fall back"; before, KH1's row its picker chose played, showing its
    // request's default KH2 swing). With our rows not built at all (`kh1_slots = off`) KH2's plain
    // picks keep the old path below; an ability's move none.
    if let Some((r, ..)) = pick.as_ref().filter(|_| !kh1_style && (own_rows() || why.is_some())) {
        note(|| format!("KH1's {} combo, swing {count}: KH2's row {} motion {} has no KH1 row of its own built: no attack", if air { "aerial" } else { "ground" }, r.id, r.motion));
        return Run::Nothing;
    }
    if !air {
        if let Some((r, l)) = pick.as_ref().zip(launch).and_then(|((p, ..), l)| kh2_hop(actor, row, p, l, request_of)) {
            if let Ok(mut g) = LAST.lock() {
                *g = request_of(r).map(|m| (1, m, false, true));
            }
            return Run::Hop(r, l);
        }
    }
    // (KH1's picker returned the row that plays KH2's pick when it made it, `kh2_pick_in_picker`:
    // no row swapped here after the fact.)
    let row = if made.is_some() { row } else { kh2_row(actor, combo, row, pick.as_ref().map(|(r, ..)| r), request_of) };
    let plays = select_swing(combo, request_of(row), pick.as_ref().map(|(r, ..)| r));
    if let Ok(mut l) = LAST.lock() {
        // (KH1's finisher: its finisher request; KH2's row selection can run KH1's swing row past
        // KH1's length (Combo Plus) or its finisher row before it (Negative Combo).)
        let finisher = |m: u32| if combo.swings.contains(&(m & 0xFFFF)) || m & 0xFFFF == combo.finisher { m & 0xFFFF == combo.finisher } else { count >= kh1_len };
        *l = request_of(row).map(|m| (count, m, finisher(m), air));
        FINISHERS.store(l.is_some_and(|x| x.2) as u8, Ordering::Relaxed);
    }
    if air {
        if let Some(l) = pick.as_ref().zip(launch).filter(|_| plays).and_then(|((p, ..), l)| air_hop(row, p, l)) {
            return Run::AirHop(row, l);
        }
    }
    Run::Row(row)
}

/// KH1's row picker's pick of one of KH2's ability moves at this press (`picker`): the row, its
/// hop, the game frame. Taken by the row runner's `attack` that same frame.
static PICK: Mutex<Option<(ComboRow, Option<character_core::Launch>, u64)>> = Mutex::new(None);

fn take_pick() -> Option<(ComboRow, Option<character_core::Launch>)> {
    let (r, l, at) = PICK.try_lock().ok().and_then(|mut g| g.take())?;
    (frame().saturating_sub(at) <= 1).then_some((r, l))
}

/// KH1's own rows the picker's KH2 ability rows may stand in for: its plain ones (KH1's ground
/// picker `0x1402A6EE0`, Steam, Checked (decomp): 1 its dash with no ability, 8 its finisher, 11 /
/// 13 / 14 its swings, 11 / 13 also its fallbacks; air picker `0x1402A6C70`: 16 its finisher,
/// 17 / 18 its swings). KH1's rows that need one of its own abilities (ground 0 its ability dash,
/// 3..7 its ability finishers, 9, 10; air 15) come first in KH1's own order and stay; so does 2,
/// its walk off a drop ahead.
const GROUND_PLAIN: [i32; 5] = [1, 8, 11, 13, 14];
const AIR_PLAIN: [i32; 3] = [16, 17, 18];

/// KH2's ability row `r` plays on KH1's rows (`picker`): a KH1 row of its own (`kh1rows`), or our
/// slot of its motion on a KH1 request of its kind (a ground hop on an air swing's, `kh2_hop`).
fn kh1_plays(r: &ComboRow) -> bool {
    if ours_for(r).is_some() {
        return true;
    }
    if !native_ability(r) {
        return false;
    }
    let has = |q: u32| super::kh2slots::has(q as usize, r.motion);
    if !r.is_air() && r.is_jump() {
        return AIR_COMBO.swings.clone().any(has);
    }
    let combo = if r.is_air() { &AIR_COMBO } else { &GROUND_COMBO };
    if r.is_finisher() { has(combo.finisher) } else { combo.swings.clone().any(has) }
}

/// KH1's own row picker with KH2's ability rows in it (the tester, combo row selection m19: "we
/// need to modify kh1s so that it can choose explosion and other moves during its kh1 combo",
/// "patch the extra functions that kh2 uses in kh1"). Called from our hooks on KH1's ground and
/// air pickers (`kh1picker`) after KH1's own picker ran, with the row it picked (`row`), base Sora
/// with the combo style KH1 where he is. KH1's picker has decided everything KH1 decides: the
/// count (the Attack step counted the press before), swing or finisher by KH1's own length (the
/// count at the stats' `+0xD4` / `+0xD5`), its own ability rows by its ability bits and chances,
/// its target and direction. Where it took one of its plain rows (`GROUND_PLAIN` / `AIR_PLAIN`),
/// KH2's ability rows come next, as KH1's own ability rows come before its plain ones: the first
/// equipped one of the kind KH1's count is at, playable on KH1's rows (`kh1_plays`), not the KH2
/// motion playing, that passes KH2's own row checks (its distance, height and facing windows,
/// its kind's targets; `Character::ability_row`, KH2's `condition` / `in_range`, Steam KH2
/// `0x140402750` / `0x1404029F0`). Then the picker returns the KH1 row it runs on (KH1's plain row
/// of its kind, `kh2_row`; for its own row or a hop from the ground the row runner's `attack`
/// picks it) and the row runner plays it as before (`attack`). At KH1's finisher it is one walk
/// of KH1's and KH2's finishers (`finisher_walk`); `bits` are Sora's ability bits as KH1 set them
/// (`kh1picker` held some back for KH1's picker's call). The row KH1 runs.
pub unsafe fn picker(actor: usize, row: i32, dir: usize, air: bool, bits: u32) -> i32 {
    if let Ok(mut g) = PICK.lock() {
        *g = None;
    }
    let combo = if air { &AIR_COMBO } else { &GROUND_COMBO };
    let plain = if air { AIR_PLAIN.contains(&row) } else { GROUND_PLAIN.contains(&row) };
    let Some((count, kh1_len, _)) = kh1_combo(actor, air) else { return row };
    let finisher = count >= kh1_len;
    let what = if air { "aerial" } else { "ground" };
    // At KH1's finisher: KH1's ability finishers and KH2's in one walk (`finisher_walk`). Rows 6
    // (Blitz) and 15 (Hurricane Blast), their own KH1 tests passed in KH1's picker, join it there.
    if finisher && (plain || (!air && row == BLITZ_ROW) || (air && row == HURRICANE_ROW)) {
        return finisher_walk(actor, row, dir, air, bits, count, kh1_len);
    }
    if !plain {
        note(|| format!("KH1's {what} row picker, swing {count} of {kh1_len}: KH1's own row {row} (one of its own abilities' or its drop ahead), kept"));
        return row;
    }
    let playing = super::kh2slots::playing_motion(actor);
    let Some((pick, launch)) = takeover::kh2_ability_row(actor, finisher, air, playing, &kh1_plays) else {
        note(|| format!("KH1's {what} row picker, swing {count} of {kh1_len}: KH1's row {row}; no KH2 ability {} passes KH2's checks", if finisher { "finisher" } else { "swing" }));
        return row;
    };
    let request_of = |r: i32| super::combat::row_motion(actor, r);
    // (Its own KH1 row, the row KH1's picker returns, `combo_row`; else, our own row or a hop from
    // the ground with none: `attack` picks its row.)
    let run = if let Some(n) = combo_row(&pick, air, launch) {
        super::kh1rows::row(n)
    } else if ours_for(&pick).is_some() || (!air && pick.is_jump()) {
        row
    } else {
        kh2_row(actor, combo, row, Some(&pick), &request_of)
    };
    // (KH1's dash, row 1, left its distance in the direction's y, which only rows 0 / 1 read.)
    if run != row && row == 1 && dir >= 0x10000 {
        wr(dir + 4, 0.0f32);
    }
    note(|| format!(
        "KH1's {what} row picker, swing {count} of {kh1_len}: KH2's {} row {} motion {} (ability {}) in place of KH1's row {row}: KH1's row {run} runs it{}",
        if finisher { "finisher" } else { "swing" },
        pick.id,
        pick.motion,
        pick.ability,
        launch.map_or(String::new(), |l| format!(", KH2's hop {:.0} high, {:.1}/frame", l.height, l.velocity[0].hypot(l.velocity[1])))
    ));
    if let Ok(mut g) = PICK.lock() {
        *g = Some((pick, launch, frame()));
    }
    run
}

/// KH1's Blitz row (ground picker `0x1402A6EE0`: marked with ability bit 3 when its plain
/// finisher, row 8, is and its row 11 isn't: nothing close beside him, `0x1402A7320`'s count;
/// Checked (decomp)).
const BLITZ_ROW: i32 = 6;
/// KH1's Hurricane Blast row (air picker `0x1402A6C70`: marked with ability bit 25 in state 2 at
/// the finisher when there is a target and the target's lock point is no more than 100 above him,
/// KH1's y down: `-100 <= dy`; Checked (decomp)). Its plain finisher (16) is marked then too, and
/// its plain swing (18).
const HURRICANE_ROW: i32 = 15;

/// One move of the finisher walk.
#[derive(Clone, Copy)]
enum Step {
    /// One of KH1's own ability finisher rows, its ability bit (Sora's ability bits) and its rule.
    Kh1(i32, u32, Kh1Rule, &'static str),
    /// One of KH2's ability finisher rows (Sora's PTYA id), by KH2's own checks.
    Kh2(u8),
    /// KH1's plain finisher (what its picker took: row 8 or 16, its dash at a far target, or a
    /// swing when the finisher's request is the one playing).
    Plain,
}

#[derive(Clone, Copy)]
enum Kh1Rule {
    /// Its checks are its group's (`Group::gate`) alone (Ripple Drive, Hurricane Blast).
    Always,
    /// KH1's 30% roll with KH1 Ability Chance on (Stun Impact, Gravity Break, Zantetsuken: the
    /// tester, combo row selection after m25, "it should change on the toggle we have like the
    /// other moves, if you have ability chance off then it should be 100 percent like the other
    /// moves, if not then 30% chance is back"; t589's "always rolls" is gone).
    Chance,
    /// KH1's own test, run inside KH1's picker, which returned this row (Blitz, row 6: nothing
    /// beside him; Hurricane Blast, row 15: a target no more than 100 above).
    Kh1Test,
}

/// A place in KH2's finisher table: the row whose range checks the moves in it take (`None`:
/// none), then the moves in their order.
struct Group {
    gate: Option<u8>,
    steps: &'static [Step],
}

const STUN_IMPACT: Step = Step::Kh1(3, 1 << 27, Kh1Rule::Chance, "Stun Impact");
const GRAVITY_BREAK: Step = Step::Kh1(4, 1 << 28, Kh1Rule::Chance, "Gravity Break");
const BLITZ: Step = Step::Kh1(BLITZ_ROW, 1 << 3, Kh1Rule::Kh1Test, "Blitz");
const ZANTETSUKEN: Step = Step::Kh1(5, 1 << 29, Kh1Rule::Chance, "Zantetsuken");
const RIPPLE_DRIVE: Step = Step::Kh1(7, 1 << 26, Kh1Rule::Always, "Ripple Drive");
const HURRICANE_BLAST: Step = Step::Kh1(HURRICANE_ROW, 1 << 25, Kh1Rule::Kh1Test, "Hurricane Blast");

/// The ground finisher walk, KH1's rule set (the tester, m19 / m20 / m21): KH2's own table order
/// for Sora's ground finishers (`00battle.bin` PTYA, Checked (files); KH2's walk `0x140402C50`
/// takes the first row that passes, no chance, Checked (decomp)): Explosion at a group (row 26:
/// any target 0..300 off the line to the primary one), the normal finisher 154 at a group (row 2:
/// any target 0..180 off the line), Guard Break (27, no range), Explosion (42, no range), the
/// normal finisher 155 (row 3). KH1's twins take their KH2 move's place and checks: Stun Impact
/// and Ripple Drive Explosion's, before it ("for explosion it should come last"); Gravity Break and
/// Blitz Guard Break's, after it, Blitz last ("gravity break and blitz last, if its blitz and
/// gravity equipped then blitz last"). KH1's normal finisher stands for KH2's 154 and 155.
const GROUND_FINISHERS: [Group; 5] = [
    Group { gate: Some(26), steps: &[STUN_IMPACT, RIPPLE_DRIVE, Step::Kh2(26)] },
    Group { gate: Some(2), steps: &[Step::Plain] },
    Group { gate: Some(27), steps: &[Step::Kh2(27), GRAVITY_BREAK, ZANTETSUKEN, BLITZ] },
    Group { gate: Some(42), steps: &[STUN_IMPACT, RIPPLE_DRIVE, Step::Kh2(42)] },
    Group { gate: None, steps: &[Step::Plain] },
];
/// The air finisher walk: KH2's air finishers in table order (Checked (files)): Magnet Burst at a
/// group (49: any target 0..650 off the line), the normal finisher 184 at a group (row 7: any
/// target 0..260 off the line), Aerial Finish (29, no range), Magnet Burst at a far primary
/// target (50: 300..650), the normal finisher 183 (row 8). Hurricane Blast takes Aerial Finish's
/// place, before it ("for hurricane blast it should come first"), with its own KH1 rule (m22
/// answer 3 (b): a target no more than 100 above him, KH1's picker's test).
const AIR_FINISHERS: [Group; 5] = [
    Group { gate: Some(49), steps: &[Step::Kh2(49)] },
    Group { gate: Some(7), steps: &[Step::Plain] },
    Group { gate: Some(29), steps: &[HURRICANE_BLAST, Step::Kh2(29)] },
    Group { gate: Some(50), steps: &[Step::Kh2(50)] },
    Group { gate: None, steps: &[Step::Plain] },
];
/// Without KH2's data (no checks to run): KH1's own order (its picker's: Stun Impact, Gravity
/// Break, Zantetsuken, Blitz, Ripple Drive, its finisher; Hurricane Blast before its air finisher).
const GROUND_KH1_ONLY: [Group; 1] = [Group { gate: None, steps: &[STUN_IMPACT, GRAVITY_BREAK, ZANTETSUKEN, BLITZ, RIPPLE_DRIVE, Step::Plain] }];
const AIR_KH1_ONLY: [Group; 1] = [Group { gate: None, steps: &[HURRICANE_BLAST, Step::Plain] }];

/// KH1's own rules for its ability finishers in a finisher walk (`finisher_walk`, and the KH2
/// setting's `kh2_finisher_walk`): what KH1's picker took (`row`), Sora's ability bits as KH1 set
/// them, whether KH1 took its dash (its own ability finishers stay out then, as in KH1's walk),
/// and KH1's one random number per press (0.3 taken off it for each roll reached, as KH1's picker
/// `0x1402A6EE0` does; a failed roll isn't rolled again further down).
struct Kh1Rules {
    row: i32,
    bits: u32,
    dash: bool,
    chance: bool,
    roll: Option<f32>,
    used: f32,
    failed: Vec<i32>,
}

impl Kh1Rules {
    fn new(row: i32, bits: u32, dash: bool) -> Self {
        Kh1Rules { row, bits, dash, chance: super::kh1picker::chance(), roll: None, used: 0.0, failed: Vec::new() }
    }

    /// KH1's ability finisher row `n` (ability bit `bit`) passes its KH1 rule now: its bit set,
    /// its request not the one playing (KH1's anti-repeat), and its rule (KH1's own test, which
    /// its picker ran and returned `n` for; KH1's 30% roll; or none).
    unsafe fn pass(&mut self, n: i32, bit: u32, rule: Kh1Rule, name: &'static str, plays_now: &dyn Fn(i32) -> bool, trail: &mut Vec<String>) -> bool {
        if self.dash || self.bits & bit == 0 || self.failed.contains(&n) || plays_now(n) {
            return false;
        }
        match rule {
            Kh1Rule::Always => true,
            Kh1Rule::Kh1Test => {
                let ok = self.row == n;
                if !ok {
                    trail.push(format!("{name}: KH1's own test fails"));
                }
                ok
            }
            Kh1Rule::Chance if !self.chance => true,
            Kh1Rule::Chance => {
                let r = *self.roll.get_or_insert_with(|| super::kh1picker::random().unwrap_or(1.0));
                let won = r - self.used < 0.3;
                self.used += 0.3;
                if !won {
                    self.failed.push(n);
                    trail.push(format!("{name}: KH1's 30% roll failed"));
                }
                won
            }
        }
    }
}

/// What the walk took.
enum Took {
    Kh1(i32, &'static str),
    Kh2(ComboRow, Option<character_core::Launch>),
    Plain,
}

/// KH1's finisher with KH1's and KH2's ability finishers in one walk (Base / Aerial Base Combo
/// KH1; the tester, m21: "do 1", one walk in KH1's picker with KH1's rule set). KH1's picker has
/// run with its own ability finishers held back (`kh1picker::HELD_GROUND`; Blitz's and, in the
/// air, Hurricane Blast's excepted) and took `row`: its plain finisher (8 / 16), its dash at a far
/// target (1), a swing (its finisher's request playing), Blitz (6) or Hurricane Blast (15), whose
/// KH1 tests it ran. The walk goes through
/// `GROUND_FINISHERS` / `AIR_FINISHERS` in order: a group whose gate row's range checks fail now
/// (KH2's `in_range` against base Sora's targets and KH1's lock, `Character::row_in_range`) is
/// passed over; in a group the first move that passes wins:
/// - KH1's: its ability bit set (`bits`, as KH1 set them), its row's request not the one playing
///   (KH1's own anti-repeat), and its rule: Stun Impact / Gravity Break / Zantetsuken KH1's 30% roll
///   with KH1 Ability Chance on (`kh1picker::chance`; one KH1 random number per press, 0.3 taken off it
///   for each roll reached, as KH1's picker does, so each keeps exactly 30%; a failed roll isn't
///   rolled again further down);
///   Blitz / Hurricane Blast only when KH1's picker found its test passing (row 6 / 15, KH1's
///   own rule); none of KH1's when its picker
///   took its dash (row 1, which comes before them in KH1's own walk);
/// - KH2's: KH2's own checks (`Character::ability_row_id`: equipped, not the KH2 motion playing,
///   playable on KH1's rows, `kh1_plays`), its hop;
/// - the plain one: KH1's own pick (row 8 / 16 when it took Blitz / Hurricane Blast).
/// KH1's row then runs it (KH2's on KH1's rows as `picker` runs them, `attack`).
unsafe fn finisher_walk(actor: usize, row: i32, dir: usize, air: bool, bits: u32, count: u8, kh1_len: u8) -> i32 {
    let what = if air { "aerial" } else { "ground" };
    let combo = if air { &AIR_COMBO } else { &GROUND_COMBO };
    let request_of = |r: i32| super::combat::row_motion(actor, r);
    let playing = rd::<i32>(actor + 0x164);
    let plays_now = |r: i32| request_of(r).is_some_and(|q| q as u16 as i32 == playing & 0xFFFF);
    // (KH1's plain finisher when its picker took Blitz or Hurricane Blast: 8 / 16 (marked with
    // them, Checked (decomp)); with its request the one playing, its plain swing (13 / 18): KH1's
    // finisher isn't free.)
    let plain = match row {
        BLITZ_ROW => if plays_now(8) { 13 } else { 8 },
        HURRICANE_ROW => if plays_now(16) { 18 } else { 16 },
        _ => row,
    };
    // (KH1's dash at a far target, row 1, comes before its ability finishers in KH1's own walk:
    // KH1's stay out there, as in KH1. KH2's take its place, as since t557.)
    let dash = row == 1;
    let chance = super::kh1picker::chance();
    let kh2_playing = super::kh2slots::playing_motion(actor);
    let mut rules = Kh1Rules::new(row, bits, dash);
    let mut trail: Vec<String> = Vec::new();
    let mut kh1_step = |n: i32, bit: u32, rule: Kh1Rule, name: &'static str, trail: &mut Vec<String>| -> bool { rules.pass(n, bit, rule, name, &plays_now, trail) };
    // Finishing Plus's next finisher with KH1's own normal finisher the one playing (its picker
    // took a swing, its finisher's request not free): KH2's normal finisher in its place (the
    // tester, m20 answer 3: "have kh1 play a kh2 normal finisher"; m21 answer 2 (a): KH2's own
    // check, 154 with a foe in its window, else 155, in the air 184 / 183;
    // `Character::normal_finisher`).
    let kh1_finisher_free = request_of(plain).is_some_and(|q| q & 0xFFFF == combo.finisher);
    let kh2_normal = count > kh1_len && !kh1_finisher_free;
    let walked = takeover::kh2_rows(actor, air, |ch, stick, env| {
        let plain_step = |ch: &mut Character| -> Took {
            if kh2_normal {
                if let Some((r, l)) = ch.normal_finisher(air, kh2_playing, stick, env) {
                    return Took::Kh2(r, l);
                }
            }
            Took::Plain
        };
        let groups: &[Group] = if air { &AIR_FINISHERS } else { &GROUND_FINISHERS };
        for g in groups {
            if let Some(gate) = g.gate {
                if !ch.row_in_range(gate, env) {
                    continue;
                }
            }
            for st in g.steps {
                match *st {
                    Step::Kh1(n, bit, rule, name) => {
                        if kh1_step(n, bit, rule, name, &mut trail) {
                            return Took::Kh1(n, name);
                        }
                    }
                    Step::Kh2(id) => {
                        if let Some((r, l)) = ch.ability_row_id(id, true, air, kh2_playing, &kh1_plays, stick, env) {
                            return Took::Kh2(r, l);
                        }
                    }
                    Step::Plain => return plain_step(ch),
                }
            }
        }
        plain_step(ch)
    });
    let no_kh2 = walked.is_none();
    let took = walked.unwrap_or_else(|| {
        let groups: &[Group] = if air { &AIR_KH1_ONLY } else { &GROUND_KH1_ONLY };
        for st in groups[0].steps {
            match *st {
                Step::Kh1(n, bit, rule, name) if kh1_step(n, bit, rule, name, &mut trail) => return Took::Kh1(n, name),
                Step::Plain => return Took::Plain,
                _ => {}
            }
        }
        Took::Plain
    });
    let tried = if trail.is_empty() { String::new() } else { format!(" ({})", trail.join("; ")) };
    let (run, line) = match took {
        Took::Plain => (plain, format!("KH1's own finisher row {plain}")),
        Took::Kh1(n, name) => (n, format!("KH1's {name} (row {n})")),
        Took::Kh2(pick, launch) => {
            let run = if let Some(n) = combo_row(&pick, air, launch) {
                super::kh1rows::row(n)
            } else if ours_for(&pick).is_some() || (!air && pick.is_jump()) {
                plain
            } else {
                kh2_row(actor, combo, plain, Some(&pick), &request_of)
            };
            let line = format!(
                "KH2's row {} motion {} (ability {}): KH1's row {run} runs it{}",
                pick.id,
                pick.motion,
                pick.ability,
                launch.map_or(String::new(), |l| format!(", KH2's hop {:.0} high, {:.1}/frame", l.height, l.velocity[0].hypot(l.velocity[1])))
            );
            if let Ok(mut g) = PICK.lock() {
                *g = Some((pick, launch, frame()));
            }
            (run, line)
        }
    };
    // (KH1's dash, row 1, left its distance in the direction's y, which only rows 0 / 1 read.)
    if run != row && row == 1 && dir >= 0x10000 {
        wr(dir + 4, 0.0f32);
    }
    note(|| format!(
        "KH1's {what} finisher walk, swing {count} of {kh1_len} (KH1's picker took row {row}{}, KH1 Ability Chance {}): {line}{tried}",
        if no_kh2 { "; no KH2 data: KH1's own order" } else { "" },
        if chance { "on" } else { "off" }
    ));
    run
}

/// KH1's own ability finisher the KH2 setting's walk took at this press (`kh2_finisher_walk`):
/// KH1's row, whether it chains on from a finisher (Finishing Plus), the game frame. Taken by the
/// row runner's `attack` that same frame.
static KH1_TAKE: Mutex<Option<(i32, bool, u64)>> = Mutex::new(None);

fn take_kh1_take() -> Option<(i32, bool)> {
    let (n, chained, at) = KH1_TAKE.try_lock().ok().and_then(|mut g| g.take())?;
    (frame().saturating_sub(at) <= 1).then_some((n, chained))
}

/// KH2's pick at an Attack press at the combo style KH2 (`kh2_press`): the row, KH2's combo length
/// and Negative Combo's copies, its hop, and whether it chains on from a finisher (Finishing Plus).
#[derive(Clone)]
pub struct Press {
    pub row: ComboRow,
    pub len: u8,
    pub neg: u8,
    pub launch: Option<character_core::Launch>,
    pub chained: bool,
}

/// KH2's pick at this press at the combo style KH2, made once, in KH1's picker (`kh1picker`), at
/// KH1's count (this press counted); Finishing Plus's next finisher when KH1's last row was a
/// finisher still playing. `None` without KH2's data or a pick.
pub unsafe fn kh2_press(actor: usize, air: bool) -> Option<Press> {
    let (count, _, _) = kh1_combo(actor, air)?;
    let playing = rd::<u32>(actor + 0x164) & 0xFFFF;
    let kh2_playing = super::kh2slots::playing_motion(actor);
    if let Some((n, _, true, _)) = LAST.try_lock().ok().and_then(|l| *l).filter(|l| l.3 == air && l.1 & 0xFFFF == playing) {
        let from = ComboState { count: n, finishers: FINISHERS.load(Ordering::Relaxed).max(1), hit: true, finisher_next: true };
        if let Some((row, len, neg, launch)) = takeover::kh2_would_take_launch(actor, Some(from), kh2_playing, air, false).filter(|(r, ..)| r.is_finisher()) {
            return Some(Press { row, len, neg, launch, chained: true });
        }
    }
    takeover::test_drop_ahead(!air && rd::<i32>(actor + 0x70) == 0);
    let from = ComboState { count: count.saturating_sub(1), finishers: 1, hit: true, finisher_next: false };
    let taken = takeover::kh2_would_take_launch(actor, Some(from), kh2_playing, air, false);
    takeover::test_drop_ahead(false);
    taken.map(|(row, len, neg, launch)| Press { row, len, neg, launch, chained: false })
}

/// KH2's pick KH1's picker made at this press (`kh2_pick_in_picker`), for the row runner's
/// `attack` that same frame: the pick (`None`: KH2 takes nothing), the row KH1's picker returned,
/// in the air, the game frame.
struct Kh2Pick {
    press: Option<Press>,
    row: i32,
    air: bool,
    at: u64,
}

static KH2_PICK: Mutex<Option<Kh2Pick>> = Mutex::new(None);

/// The pick KH1's picker made for this row run (`row`, `air`), if it made one this frame.
fn take_kh2_pick(row: i32, air: bool) -> Option<Option<Press>> {
    let p = KH2_PICK.try_lock().ok().and_then(|mut g| g.take())?;
    (frame().saturating_sub(p.at) <= 1 && p.row == row && p.air == air).then_some(p.press)
}

/// KH2's whole pick in KH1's picker at the combo style KH2 (the tester, combo row selection m24:
/// "(a)", step 1 of the native plan): after KH1's picker ran (with KH2's length, `kh1picker`) and
/// the finisher walk left KH2's pick standing (`kh2_finisher_walk`), KH1's picker returns the KH1
/// row that plays KH2's move: for a plain swing or finisher of KH2's (`Kh1Combo::plain`), KH1's
/// plain row of that kind whose request isn't the one playing (`kh2_row`, which the row runner
/// used to do after the fact); for a Finishing Plus next finisher, a hop from the ground or a move
/// on a KH1 row of its own, the row KH1's picker took (the row runner's `attack` picks the row for
/// those, as before). The pick is kept for `attack` (`KH2_PICK`), which no longer runs KH2's
/// selector again. KH1's dash (rows 0 / 1) replaced: its distance in the direction's y cleared, as
/// `picker` does. The row KH1 runs.
pub unsafe fn kh2_pick_in_picker(actor: usize, row: i32, dir: usize, air: bool, press: Option<Press>) -> i32 {
    let combo = if air { &AIR_COMBO } else { &GROUND_COMBO };
    let request_of = |r: i32| super::combat::row_motion(actor, r);
    let mut run = row;
    let playing = rd::<u32>(actor + 0x164) & 0xFFFF;
    // (KH2's pick on its own KH1 row: the row KH1's picker returns; the tester, combo row selection
    // after m25, answer 3 (a). Finishing Plus's next finisher only while that row's request isn't
    // the one playing, KH1's anti-repeat, as `next_finisher` takes it.)
    let own = press.as_ref().and_then(|p| combo_row(&p.row, air, p.launch).filter(|&n| !p.chained || super::kh1rows::request(n) != playing));
    if let Some(n) = own {
        run = super::kh1rows::row(n);
    } else if let (Some(p), Some((count, kh1_len, _)), false) = (press.as_ref().filter(|p| !p.chained), kh1_combo(actor, air), own_rows()) {
        // (Only with our rows not built at all, `kh1_slots = off`: with them, a pick with no row
        // of its own is no attack, `attack_row`, and KH1's row isn't re-pointed for it.)
        let borrow = !combo.serves(&p.row) && ability_move(&p.row, p.len, p.neg, count, kh1_len).is_some();
        if !borrow && !(!air && p.row.is_jump()) {
            run = kh2_row(actor, combo, row, Some(&p.row), &request_of);
        }
    }
    if run != row && matches!(row, 0 | 1) && dir >= 0x10000 {
        wr(dir + 4, 0.0f32);
    }
    if let Ok(mut g) = KH2_PICK.lock() {
        *g = Some(Kh2Pick { press, row: run, air, at: frame() });
    }
    run
}

/// KH1's flight attack rows (Steam KH1's Attack step `0x1402A7510`, Checked (decomp)): in flight
/// (state 8) it asks the air picker `0x1402A6C70` and runs its row + 3, so its plain swing 18, its
/// swing at a target above 17 and its finisher 16 become rows 21, 20 and 19 (15, a finisher with
/// ability bit 25, is marked only in state 2). The row runner `0x1402A68F0` has one case for 19..21:
/// the direction made level and the row's lunge (8 in all three) as his speed along it
/// (`0x1402B1E20`), `0x1402A6DF0 (actor, 0.5, 8.0)`, and KH1's flight motion 0x75 queued (blend
/// 16). Their requests are the air combo's own 0xCE, 0xCD, 0xCC (btltbl `+0xA9F0`, Checked (files)):
/// KH1 has no flight attack animation of its own.
pub const FLIGHT_FINISHER_PICK: i32 = 16;
pub const FLIGHT_SWING_PICK: i32 = 18;

/// KH2's air pick for a flight Attack (`flight_pick`): our row and the game frame.
static FLIGHT: Mutex<Option<(usize, u64)>> = Mutex::new(None);

/// A flight Attack at the Aerial Base Combo KH2 (the tester, what-is-still-missing m6, answer 2
/// (a): "KH1's own flying Attack, with KH2's air combo picks on KH1's air rows"). Called by our air
/// picker hook in state 8 with `run`, the row KH2's pick plays (`kh2_pick_in_picker` /
/// `kh2_finisher_walk`, as in state 2), and `own`, the row KH1's picker took. A KH2 air move on a
/// row of ours: KH1's flight row of its kind runs (`FLIGHT_FINISHER_PICK` / `FLIGHT_SWING_PICK`,
/// the Attack step adds the 3) with our row's request, attack data and bytes in it for that call
/// (`take_flight`, `kh1rows::run_as`), so KH1's flight case moves him and puts him back in flight
/// after it. Anything else (no KH2 pick, one of KH1's own rows): KH1's own flight row, as unmodded.
pub fn flight_pick(own: i32, run: i32) -> i32 {
    let Some(n) = usize::try_from(run).ok().and_then(|r| r.checked_sub(super::kh1rows::KH1_ROWS)).filter(|&n| n < super::kh1rows::OURS.len()) else {
        return own;
    };
    let o = &super::kh1rows::OURS[n];
    if !o.attack || o.kind.cast() || !o.kind.air() {
        note(|| format!("flight Attack: KH2's pick {} (motion {}) isn't an air move: KH1's own flight row {}", o.name, o.motion, own + 3));
        return own;
    }
    if let Ok(mut g) = FLIGHT.lock() {
        *g = Some((n, frame()));
    }
    if o.kind.finisher() {
        FLIGHT_FINISHER_PICK
    } else {
        FLIGHT_SWING_PICK
    }
}

/// The KH2 move `flight_pick` put on this flight row (`row`: 19 or 21) this frame, for
/// `combat::exec`; its bookkeeping for the next press (`LAST`, `FINISHERS`, as `ours_run` keeps it).
pub unsafe fn take_flight(actor: usize, row: i32) -> Option<usize> {
    if !matches!(row, 19 | 21) {
        return None;
    }
    let (n, at) = FLIGHT.try_lock().ok().and_then(|mut g| g.take())?;
    let o = &super::kh1rows::OURS[n];
    if frame().saturating_sub(at) > 1 || (row == 19) != o.kind.finisher() {
        return None;
    }
    let count = kh1_combo(actor, true).map_or(1, |c| c.0);
    let request_of = |r: i32| super::combat::row_motion(actor, r);
    let _ = ours_run(n, None, true, count, &request_of);
    note(|| format!("flight Attack: KH2's {} (motion {}) on KH1's flight row {row}", o.name, o.motion));
    Some(n)
}

/// KH1's own ability finishers at the combo style KH2 (the tester, combo row selection m23: "I
/// can't do any kh1 finisher on kh2 combat settings", "the goal is to patch features into kh1's
/// engine"). Called from our hook on KH1's picker (`kh1picker`) after KH1's picker ran with its
/// ability finisher bits held back and KH2's combo length on the length it reads, so KH1's own
/// finisher (and Blitz's and Hurricane Blast's own tests) fall where KH2's does. When KH2's pick
/// at this press (`kh2_press`) is a finisher, the same walk as the KH1 setting's (`GROUND_FINISHERS`
/// / `AIR_FINISHERS`: KH2's table order, KH1's twins in their KH2 move's place, each group gated by
/// its KH2 row's checks) runs with KH1's rules (`Kh1Rules`, KH1's dash aside: there is none at this
/// setting); where it reaches a KH2 row that passes, or the plain finisher, KH2's own pick stands
/// (the row runner's `attack` runs it as before); where it reaches one of KH1's that passes, KH1's
/// own row is what the picker returns, and `attack` runs it whole (KH1's motion, attack data,
/// damage). KH1's row when one of its moves won; `None` when KH2's pick stands
/// (`kh2_pick_in_picker` then picks its row).
pub unsafe fn kh2_finisher_walk(actor: usize, row: i32, air: bool, bits: u32, press: Option<&Press>) -> Option<i32> {
    if let Ok(mut g) = KH1_TAKE.lock() {
        *g = None;
    }
    let (pick, kh2_len, chained) = press.filter(|p| p.row.is_finisher()).map(|p| (&p.row, p.len, p.chained))?;
    let what = if air { "aerial" } else { "ground" };
    let request_of = |r: i32| super::combat::row_motion(actor, r);
    let playing = rd::<i32>(actor + 0x164);
    let plays_now = |r: i32| request_of(r).is_some_and(|q| q as u16 as i32 == playing & 0xFFFF);
    let kh2_playing = super::kh2slots::playing_motion(actor);
    let mut rules = Kh1Rules::new(row, bits, false);
    let mut trail: Vec<String> = Vec::new();
    let walked = takeover::kh2_rows(actor, air, |ch, stick, env| {
        let groups: &[Group] = if air { &AIR_FINISHERS } else { &GROUND_FINISHERS };
        for g in groups {
            if g.gate.is_some_and(|gate| !ch.row_in_range(gate, env)) {
                continue;
            }
            for st in g.steps {
                match *st {
                    Step::Kh1(n, bit, rule, name) => {
                        if rules.pass(n, bit, rule, name, &plays_now, &mut trail) {
                            return Some((n, name));
                        }
                    }
                    Step::Kh2(id) => {
                        if let Some((r, _)) = ch.ability_row_id(id, true, air, kh2_playing, &|_| true, stick, env) {
                            trail.push(format!("KH2's row {} motion {} first", r.id, r.motion));
                            return None;
                        }
                    }
                    Step::Plain => return None,
                }
            }
        }
        None
    })
    .flatten();
    let tried = if trail.is_empty() { String::new() } else { format!(" ({})", trail.join("; ")) };
    let Some((n, name)) = walked.filter(|&(n, _)| request_of(n).is_some()) else {
        note(|| format!("KH2's {what} finisher walk at the KH2 setting (KH1's picker took row {row}): KH2's own pick, row {} motion {} (KH2's length {kh2_len}{}){tried}", pick.id, pick.motion, if chained { ", Finishing Plus" } else { "" }));
        return None;
    };
    note(|| format!("KH2's {what} finisher walk at the KH2 setting (KH1's picker took row {row}, KH2 would take row {} motion {}): KH1's {name} (row {n}){}{tried}", pick.id, pick.motion, if chained { ", Finishing Plus" } else { "" }));
    if let Ok(mut g) = KH1_TAKE.lock() {
        *g = Some((n, chained, frame()));
    }
    Some(n)
}

/// KH2's combo length on the length byte KH1's picker reads (the stats' `+0xD4` ground / `+0xD5`
/// air, the pickers' own finisher test `len <= count`, Steam `0x1402A6EE0` / `0x1402A6C70`,
/// Checked (decomp)) for its call at the combo style KH2 (`kh1picker`): the byte's address and
/// value, to put back.
pub unsafe fn kh2_length_for_picker(actor: usize, air: bool, kh2_len: u8) -> Option<(usize, u8)> {
    let stats = pose::ptr(rd::<u32>(actor + 0x6C));
    if stats < 0x10000 || kh2_len == 0 {
        return None;
    }
    let at = stats + if air { 0xD5 } else { 0xD4 };
    let was = rd::<u8>(at);
    if was == kh2_len {
        return None;
    }
    wr(at, kh2_len);
    Some((at, was))
}

/// KH2's air swing's own hop on KH1's air swing (Aerial Combo KH2; the tester: "(a)", KH2's own
/// per-row hop on each air swing, 1:1): KH2's pick is a plain air row (rows 7..10: 181 / 182,
/// finishers 183 / 184; or an ability's KH1 plays natively, `native_ability`: Aerial Spiral 192,
/// Aerial Finish 194) and KH1's air row plays its motion (`select_swing`), so the
/// hop KH2 starts with it (Steam KH2 `0x1403D78D0`: every row with `jump` above 0 starts one, in
/// the air too, the count kept; `launch`, its height and speed for the primary target, or the
/// row's `jump` and `speed_max` along his facing with none) replaces the one KH1's row starts
/// (`0x1402A68F0`: `+0x3C0` 8 / 12 / 4 along the direction and the row's own `+8` hop), on
/// KH1's air fields (`shared::physics::kh2_hop`, after the row ran). The launch, if so; `None`
/// leaves KH1's row's own (not a plain air row of KH2's, or no hop).
fn air_hop(row: i32, pick: &ComboRow, launch: character_core::Launch) -> Option<character_core::Launch> {
    if !AIR_COMBO.plain(pick) || !pick.is_jump() || !launch.height.is_finite() || !launch.velocity.iter().all(|v| v.is_finite()) {
        return None;
    }
    note(|| format!(
        "KH2's row selection: KH2's air row {} motion {} on KH1's air row {row}: KH2's hop, {:.0} high, {:.1}/frame sideways, for KH1's row's own",
        pick.id,
        pick.motion,
        launch.height,
        launch.velocity[0].hypot(launch.velocity[1])
    ));
    Some(launch)
}

/// KH1's collision block on Sora (`+0x2A0`: `+0` position, `+0x10` the move, `+0x20` radius,
/// `+0x24` height, `+0x28` flags, `+0x2C` the object he stands on (all ones: the map), `+0x30`
/// result bits, `+0x3C` the floor polygon (0xFFFF none), `+0x40` the result position, `+0x68`
/// the floor's height; Steam `0x140195180`, Checked (decomp)), the part the mover writes, put
/// back after our test.
const COLLISION: usize = 0x2A0;
const COLLISION_LEN: usize = 0x78;
/// KH1's set-up of that block from his model's shape (`0x14029A0B0 (actor, block)`: position,
/// radius, height, flags; returns the step, 0.7 of the smaller of radius and height), the mover
/// (`0x140195180 (block)`), and a map polygon's normal (`0x1401907F0 (polygon, out)`): the
/// calls KH1's own sweep `0x1402C6E70` makes, in its order. Steam only (no Epic address); their
/// bytes are checked before each test.
const SETUP_STEAM: usize = 0x29A0B0;
const SETUP_BYTES: [u8; 10] = [0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20];
const MOVER_STEAM: usize = 0x195180;
const MOVER_BYTES: [u8; 10] = [0x40, 0x55, 0x53, 0x57, 0x41, 0x54, 0x48, 0x8D, 0xAC, 0x24];
const NORMAL_STEAM: usize = 0x1907F0;
const NORMAL_BYTES: [u8; 11] = [0xF3, 0x0F, 0x10, 0x15, 0xC4, 0x27, 0x25, 0x00, 0x48, 0x63, 0xC1];

/// KH2's line ahead at an attack press on the ground (the tester: "add that"; Steam KH2
/// `0x140402D20`, Checked (decomp)): `aim` (KH2's selector's direction, to the primary target or
/// his facing, `Character::selector_aim`, x / z) `attack_j_front` (plyr `+0x2C`, Sora's -5) plus
/// his collision radius (`+0x6F0`, the model's first kind 0 / 4 shape, Sora's 90) long, laid on
/// the floor's plane under him (`0x1403B7BD0`: across the ground normal twice, its length kept)
/// and swept with his collision from his feet (`0x1403BA750` -> `0x14016FEC0`). Row 1 is marked
/// when the mover's bit 0 comes back (`0x140172640`, its branch for this sweep, the collision
/// flag `+0x740` bit 0 cleared first): no floor in its search at the end point (from 115 above
/// to 345 below, `0x140172840`: three times the shape's lift `+0x73C`), or the floor there more
/// than lift less height less 1 (115 - 90 - 1 = 24, `+0x73C` - `+0x6F4`, `+0x754` being 0 with
/// no object at `+0x9D8`) below his feet, his feet lowered by the line's own drop when it runs
/// downhill (`+0x14` above 0). So a step down of 24 or less is floor, deeper is a drop, wherever
/// along the line the edge is; only the end point counts.
/// Here: the same line on KH1's collision. KH1's mover moves his collision block (KH1's shape)
/// along its flat part in KH1's own steps (`0x1402C6E70`'s), the floor plane from the polygon
/// KH1's mover finds under him (a map polygon; on an object, flat: **assumption**), and the floor it finds
/// under the end (`+0x3C` / `+0x68`: the nearest below, or up to his height above) is judged by
/// KH2's rule. KH1's own test (row 2) differs: any step with the floor more than half KH1's
/// height below. KH1's block put back after. False off Steam or with no KH2 data.
pub unsafe fn drop_ahead(actor: usize, aim: [f32; 2]) -> bool {
    let Some(kh2) = pose::kh2() else { return false };
    let Some(shape) = kh2_data::sora::bg_shape(&kh2.rig.body_shapes) else { return false };
    let length = kh2.character.attack.jump_front + shape.radius;
    match floor_gone(actor, aim, length) {
        Some(Gone { found: true, below, reach }) => {
            note(|| format!("KH2's line ahead: the floor {below:.0} below at {length:.0} ahead (more than {reach:.0}: none), KH2's row 1 marked"));
            true
        }
        Some(_) => {
            note(|| format!("KH2's line ahead: no floor under the point {length:.0} ahead, KH2's row 1 marked"));
            true
        }
        None => false,
    }
}

/// KH2's no-floor result for a line (`floor_gone`): whether a floor was found under its end, how
/// far below his feet it is, and KH2's limit (more than that below is no floor).
pub struct Gone {
    pub found: bool,
    pub below: f32,
    pub reach: f32,
}

/// KH2's sweep for "no floor" (`0x1403BA750` -> `0x14016FEC0`, the rule in `drop_ahead`'s notes)
/// along `aim` (x / z), `length` long, on KH1's collision: `Some` when there is no floor at its
/// end (none, or more than KH2's lift less height less 1 below his feet), `None` when there is
/// floor (or it couldn't be tested). Shared by KH2's line ahead (`drop_ahead`) and its edge stop
/// (`shared::physics`'s ground step, `kh2slots::edge_stop`). KH1's block put back after. Steam
/// only, the three calls' bytes checked.
pub unsafe fn floor_gone(actor: usize, aim: [f32; 2], length: f32) -> Option<Gone> {
    if Build::current() != Build::Steam {
        return None;
    }
    let base = takeover::game_base();
    if rd::<[u8; 10]>(base + SETUP_STEAM) != SETUP_BYTES || rd::<[u8; 10]>(base + MOVER_STEAM) != MOVER_BYTES || rd::<[u8; 11]>(base + NORMAL_STEAM) != NORMAL_BYTES {
        note(|| "KH2's floor sweep: KH1's collision calls (0x14029A0B0 / 0x140195180 / 0x1401907F0) aren't the expected code here; not tested".to_string());
        return None;
    }
    let kh2 = pose::kh2()?;
    let shape = kh2_data::sora::bg_shape(&kh2.rig.body_shapes)?;
    // (KH2's mover: the floor more than this below his feet is none.)
    let reach = shape.lift - shape.height - 1.0;
    let flat = aim[0].hypot(aim[1]);
    if !(flat > 1e-3) || !(length > 0.0) || !reach.is_finite() {
        return None;
    }
    let v = glam::Vec3::new(aim[0] / flat, 0.0, aim[1] / flat);
    let saved: [u8; COLLISION_LEN] = rd(actor + COLLISION);
    let setup: unsafe extern "system" fn(usize, usize) -> f32 = std::mem::transmute(base + SETUP_STEAM);
    let mover: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(base + MOVER_STEAM);
    // The floor's plane under him (KH1 y-down; the projection doesn't care which way it points):
    // KH1's mover where he stands (KH1's own sweep in its picker has just left the block at its
    // end), its floor polygon's normal.
    let step = setup(actor, actor + COLLISION);
    wr(actor + COLLISION + 0x10, [0.0f32, 0.0, 0.0, 1.0]);
    wr(actor + COLLISION + 0x28, rd::<u32>(actor + COLLISION + 0x28) & !1);
    mover(actor + COLLISION);
    let on_object = rd::<u32>(actor + COLLISION + 0x2C) != u32::MAX;
    let polygon = rd::<u16>(actor + COLLISION + 0x3C);
    let mut normal = glam::Vec3::NEG_Y;
    if !on_object && polygon != 0xFFFF {
        let normal_of: unsafe extern "system" fn(i32, *mut [f32; 4]) = std::mem::transmute(base + NORMAL_STEAM);
        let mut n = [0.0f32; 4];
        normal_of(polygon as i16 as i32, &mut n);
        let n = glam::Vec3::new(n[0], n[1], n[2]).normalize_or_zero();
        if n.y.abs() > 0.2 {
            normal = n;
        }
    }
    // (KH2's `0x1403B7BD0`: across the normal, back across it, normalized, at the length.)
    let line = normal.cross(v).normalize_or_zero().cross(normal).normalize_or_zero() * length;
    let across = glam::Vec2::new(line.x, line.z);
    // (From his position again, as KH1's sweep starts.)
    setup(actor, actor + COLLISION);
    let start_y: f32 = rd(actor + COLLISION + 4);
    let mut left = across.length();
    if !(step > 0.5) || !start_y.is_finite() || !(left > 0.0) {
        wr(actor + COLLISION, saved);
        return None;
    }
    let dir = across / left;
    let mut steps = 0;
    while left > 0.0 && steps < 64 {
        let s = step.min(left);
        wr(actor + COLLISION + 0x10, [dir.x * s, 0.0f32, dir.y * s, 1.0f32]);
        wr(actor + COLLISION + 0x28, rd::<u32>(actor + COLLISION + 0x28) & !1);
        mover(actor + COLLISION);
        wr(actor + COLLISION, rd::<[f32; 4]>(actor + COLLISION + 0x40));
        left -= step;
        steps += 1;
    }
    let found = rd::<u16>(actor + COLLISION + 0x3C) != 0xFFFF;
    let below = rd::<f32>(actor + COLLISION + 0x68) - (start_y + line.y.max(0.0));
    wr(actor + COLLISION, saved);
    (!found || !(below <= reach)).then_some(Gone { found, below, reach })
}

/// KH2's hop out of KH1's ground combo (Base Combo KH2; the tester: "design and implement
/// natively (KH1's air state with KH2's arc and numbers)"): KH2's pick is a ground row that hops
/// with no ability (row 0, motion 182, at a target well above; row 1, 181, the drop ahead) or an
/// ability hop KH1 plays natively (`native_ability`: Aerial Sweep, row 33, motion 191; Aerial
/// Dive, 48, 196, its no-gravity frames held by KH1's air step, `kh2slots::no_gravity`), so
/// KH1's air state has him for it: KH1's own air swing row (18, or 17 when 18's request is
/// playing: its air picker's fallback order), whose request then plays our slot of KH2's motion
/// (KH2's hop rows play its air swings, built for KH1's air requests, `kh2slots`), started out of
/// KH1's air state (`combat::exec`: KH1's state set to 2 as the hand-backs do) with KH1's combo
/// count at 1 (KH2's: a hop from the ground restarts the count), then KH2's arc and sideways speed
/// on it (`launch`: the height and speed KH2's controller works out for the target,
/// `shared::physics::kh2_hop`). KH1's air combo goes on from there (`AIR_COMBO`). The KH1 air row
/// to run, if so; `None` (KH1's row as before) with the Aerial Combo at KH1 (the slot plays KH1's
/// air swing there) or no slot for the motion.
unsafe fn kh2_hop(actor: usize, row: i32, pick: &ComboRow, launch: character_core::Launch, request_of: &dyn Fn(i32) -> Option<u32>) -> Option<(i32, character_core::Launch)> {
    use super::styles::Kind;
    let kh2 = style_kh2;
    // (An ability's hop is KH2's at either style, `native_ability`; KH2's plain hops are KH2's
    // row selection, with both combo styles at KH2.)
    let ability = pick.ability != 0;
    if pick.is_air() || !pick.is_jump() || (ability && !native_ability(pick)) || (!ability && !kh2(Kind::GroundCombo)) {
        return None;
    }
    if !ability && !kh2(Kind::AirCombo) {
        note(|| format!("KH2's row selection: KH2's hop, row {} motion {}, not played: the Aerial Combo style is KH1's (KH1's row {row} kept)", pick.id, pick.motion));
        return None;
    }
    let playing = rd::<u32>(actor + 0x164) & 0xFFFF;
    let Some((r, q)) = AIR_COMBO.swing_rows.iter().filter_map(|&r| Some((r, request_of(r)?))).find(|&(_, q)| AIR_COMBO.swings.contains(&q) && q != playing && super::kh2slots::has(q as usize, pick.motion)) else {
        note(|| format!("KH2's row selection: KH2's hop, row {} motion {}, not played: no air swing row of KH1's free with a slot for it (KH1's row {row} kept)", pick.id, pick.motion));
        return None;
    };
    super::kh2slots::pick_ability(q as usize, Some(pick.motion), ability && !kh2(Kind::AirCombo));
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    if info >= 0x10000 {
        wr(info + 0x41, 1u8);
    }
    note(|| format!(
        "KH2's row selection: KH2's hop, row {} motion {}: KH1's row {row} replaced by its air row {r} (request {q:#x}) in its air state, {:.0} high, {:.1}/frame sideways",
        pick.id,
        pick.motion,
        launch.height,
        launch.velocity[0].hypot(launch.velocity[1])
    ));
    Some((r, launch))
}

/// KH1's combo, ground or air, for KH2's row selection: its style, its swing requests and its
/// finisher request, and its plain rows in the order its picker falls back to them.
struct Kh1Combo {
    style: super::styles::Kind,
    swings: std::ops::RangeInclusive<u32>,
    finisher: u32,
    swing_rows: &'static [i32],
    finisher_rows: &'static [i32],
}

impl Kh1Combo {
    /// KH2's `row` is a plain swing or finisher of this combo: no ability (those are borrows), or
    /// an ability row KH1's rows play natively (`native_ability`); on the ground no hop (KH2's
    /// hops aren't KH1 rows), in the air an air row.
    fn plain(&self, row: &ComboRow) -> bool {
        let air = self.style == super::styles::Kind::AirCombo;
        let here = if air { row.is_air() } else { !row.is_air() && !row.is_jump() };
        here && (row.ability == 0 || native_ability(row))
    }

    /// KH2's row selection takes this pick on KH1's rows: every pick at the combo style KH2; at
    /// KH1's, only an ability's move (`native_ability`: KH2's at either style), KH1's own combo
    /// and its own picks playing around it.
    fn here(&self, row: &ComboRow) -> bool {
        style_kh2(self.style) || (row.ability != 0 && native_ability(row))
    }

    /// KH1's own plain rows of this combo play KH2's `row` natively (`kh2_row`, `select_swing`):
    /// the combo style KH2, a plain row of it (`plain`), and a slot of its motion on a request of
    /// its kind (a swing request for a swing, the finisher's for a finisher). Then a pick past
    /// KH1's combo length (Combo Plus) or a finisher before it (Negative Combo) needs no borrow:
    /// KH1's swing or finisher row of KH2's kind runs it, its count going on as KH1's.
    fn serves(&self, row: &ComboRow) -> bool {
        let fits = |q: u32| super::kh2slots::has(q as usize, row.motion);
        self.here(row) && self.plain(row) && if row.is_finisher() { fits(self.finisher) } else { self.swings.clone().any(fits) }
    }

    /// KH1's `request` is the kind of KH2's `row` (a swing for a swing, the finisher for a finisher).
    fn fits(&self, request: u32, row: &ComboRow) -> bool {
        if row.is_finisher() { request == self.finisher } else { self.swings.contains(&request) }
    }
}

/// The ground combo: requests 0xC8..0xCA its swings (the third, 0xCA, is row 14, marked for a
/// target ahead), 0xCB its finisher; its plain swing rows 13, or 11 when 13's request is playing
/// (Steam `0x1402A6EE0`, Checked; the tester's t374 traces: rows 13 / 11 / 14 run motions 200 /
/// 201 / 202), its plain finisher row 8.
const GROUND_COMBO: Kh1Combo = Kh1Combo { style: super::styles::Kind::GroundCombo, swings: 0xC8..=0xCA, finisher: 0xCB, swing_rows: &[13, 11], finisher_rows: &[8] };
/// The air combo (KH1's air picker `0x1402A6C70`, Steam, Checked): rows 15..18; 18 its plain
/// swing (no target, or one not more than 100 above him), 17 for a target more than 100 above
/// (KH1's y is down: the test is the target's y less his at -100 or more; the tester's traces
/// f42760 / f44092 / f44266, enemies above, run 17's 0xCD first, f37236 / f38130 18's 0xCC), 16
/// its finisher (the count at the stats' `+0xD5`), 15 a finisher by a KH1 ability bit (25);
/// with none free, 18 down to 15, the first whose request isn't playing. Requests 0xCC / 0xCD
/// the swings, 0xCE the finisher (`kh2slots`); which row has which is read from the row table
/// (`request_of`), not assumed.
const AIR_COMBO: Kh1Combo = Kh1Combo { style: super::styles::Kind::AirCombo, swings: 0xCC..=0xCD, finisher: 0xCE, swing_rows: &[18, 17], finisher_rows: &[16, 15] };

/// KH2's row selection over KH1's own rows (Base / Aerial Combo KH2; the tester: "do option
/// (a)", KH2 1:1, and "the same KH2 row picker on the air combo"): where KH1's picker took a row
/// of its own that isn't the kind KH2's pick is (on the ground its dash at a target 160+ away,
/// rows 0 / 1; its walk off a drop ahead, row 2; its rows 9 / 10; its ability finishers 3..7; in the air its row
/// for a target above or its ability finisher; or a swing where KH2 finishes and the other way
/// round), and KH2's pick is a plain swing or finisher of that combo (`Kh1Combo::plain`), KH1's
/// own plain row of that kind runs instead: a swing row whose request isn't the one playing
/// (KH1's own anti-repeat, so the request restarts) or the finisher row, its request then
/// pointed at our slot of KH2's motion (`select_swing`). KH1's row runner (`0x1402A68F0`) and its
/// hits, damage and windows stay KH1's; KH1's dash goes. Unchanged when KH2's motion has no slot
/// on that request.
unsafe fn kh2_row(actor: usize, combo: &Kh1Combo, row: i32, pick: Option<&ComboRow>, request_of: &dyn Fn(i32) -> Option<u32>) -> i32 {
    let Some(p) = pick.filter(|r| combo.plain(r) && combo.here(r)) else { return row };
    let fits = |q: u32| combo.fits(q, p);
    let own = request_of(row);
    if own.is_some_and(fits) {
        return row;
    }
    let playing = rd::<u32>(actor + 0x164) & 0xFFFF;
    let rows = if p.is_finisher() { combo.finisher_rows } else { combo.swing_rows };
    let Some((r, q)) = rows.iter().filter_map(|&r| Some((r, request_of(r)?))).find(|&(_, q)| fits(q) && q != playing && super::kh2slots::has(q as usize, p.motion)) else {
        note(|| format!("KH2's row selection: KH1's row {row} (request {}) kept: no plain row of KH1's free for KH2's row {} motion {}", own.map_or("none".to_string(), |q| format!("{q:#x}")), p.id, p.motion));
        return row;
    };
    note(|| format!("KH2's row selection: KH1's row {row} (request {}) replaced by its row {r} (request {q:#x}) for KH2's row {} motion {}", own.map_or("none".to_string(), |q| format!("{q:#x}")), p.id, p.motion));
    r
}

/// KH2's row selection on KH1's own swing (`attack`, no ability move borrowed): with the Base
/// Combo (ground) or Aerial Combo (air) at KH2, the swing KH2's combo takes at this press
/// (`pick`: its rows by the target's distance, height and direction, the count, the finisher rule
/// and the anti-repeat, Steam exe `0x140402D20`) plays on the row KH1 runs: KH1's request for it
/// (`request`: the row's motion; `Kh1Combo`'s swings and finisher) is pointed at our slot of that
/// KH2 motion (`kh2slots::pick`). KH1's row, its hits, damage and timing windows from the slot's
/// events, are KH1's as before. A pick that doesn't fit KH1's row (a ground row that hops, a
/// finisher on KH1's swing or a swing on its finisher, a motion with no slot) leaves the
/// request's own motion. KH1's own other rows are replaced by its plain ones first (`kh2_row`).
/// Whether KH2's motion plays.
fn select_swing(combo: &Kh1Combo, request: Option<u32>, pick: Option<&ComboRow>) -> bool {
    let Some(q) = request.filter(|q| combo.swings.contains(q) || *q == combo.finisher) else { return false };
    let req = q as usize;
    let kh2 = style_kh2(combo.style);
    let fits = pick.filter(|r| combo.plain(r) && combo.here(r) && combo.fits(q, r));
    // (At KH1's style only an ability's move plays ours there, KH1's request pointed at it for
    // that swing, `kh2slots::pick_ability`.)
    let plays = unsafe { super::kh2slots::pick_ability(req, fits.map(|r| r.motion), !kh2) };
    if !kh2 && !plays {
        return false;
    }
    note(|| format!(
        "KH2's row selection: KH1's request {req:#x} plays {}",
        match (pick, plays) {
            (Some(r), true) => format!("KH2's row {} motion {}", r.id, r.motion),
            (Some(r), false) => format!("its own motion (KH2's row {} motion {}: no slot for it, or it doesn't fit KH1's row: hop {}, ability {}, finisher {})", r.id, r.motion, r.is_jump(), r.ability, r.is_finisher()),
            (None, _) => "its own motion (KH2 takes nothing)".to_string(),
        }
    ));
    plays
}

/// KH1's control step with base Sora in KH1's own states (`shared::control`): a press that is one
/// of KH2's ability moves out of KH1's own move (a Square move in KH1's combo window, Aerial
/// Recovery or Retaliating Slash while knocked away). The bits KH1 doesn't see, if so.
pub fn control(s: &super::shared::Seen, jump_fresh: bool, square_fresh: bool) -> Option<u32> {
    if !(jump_fresh || square_fresh) || super::form::in_form() {
        return None;
    }
    let actor = unsafe { sora() };
    if actor < 0x10000 {
        return None;
    }
    let air = matches!(s.state, 1 | 2);
    let all = super::shared::KH1_JUMP_BITS | super::shared::KH1_ACTION_BIT;
    // Knocked away by KH1's own reaction: KH2's Aerial Recovery (the jump) or Retaliating Slash
    // (Square), on KH1 requests of their own (`kh1rows`) in KH1's air state.
    if s.launched && !s.can_act {
        if jump_fresh && takeover::with_abilities(|a| a.aerial_recovery) == Some(true) {
            if unsafe { recover(actor) } {
                return Some(all);
            }
            // (Its KH1 request didn't build at load: the jump does nothing, not KH1's own in its
            // place (no fallback, the tester, combo row selection after m26, answer 1). Out of
            // KH1's air state KH2 has no Aerial Recovery either: the jump stays KH1's.)
            if !recovery_built() {
                note(|| "jump while KH1 has him knocked away: KH2's Aerial Recovery has no KH1 request of its own built: nothing".to_string());
                return Some(all);
            }
            return None;
        }
        if square_fresh {
            let row = takeover::kh2_retaliating_slash();
            if let Some(id) = row {
                // Its own KH1 row: KH1's Attack command for it (as for the Square moves), its row
                // ours, KH2's hop for the row (counts at 1 / 1, as KH2's).
                let n = super::kh1rows::by_kh2_row(id).filter(|&n| super::kh1rows::installed() && super::kh2slots::has(super::kh1rows::request(n) as usize, super::kh1rows::OURS[n].motion));
                if let (Some(n), 2) = (n, s.state) {
                    if let Some((r, launch)) = unsafe { takeover::kh2_would_take_row(actor, id, true) } {
                        unsafe { end_kh1_reaction(actor) };
                        if let Ok(mut g) = OURS.lock() {
                            *g = Some((n, launch, frame()));
                        }
                        unsafe { super::kh1rows::command_attack() };
                        note(|| format!("Square while KH1 has him knocked away: KH2's Retaliating Slash (row {} motion {}) on KH1's row {} of its own, {}", r.id, r.motion, super::kh1rows::row(n), launch.map_or("no hop".to_string(), |l| format!("{:.0} high, {:.1}/frame", l.height, l.velocity[0].hypot(l.velocity[1])))));
                        return Some(all);
                    }
                }
                // (No KH1 row of its own built for it: Square does nothing, not KH1's own in its
                // place (no fallback, the tester, after m26, answer 1). Out of KH1's air state, or
                // KH2's row checks refusing it, KH2 has no Retaliating Slash: Square stays KH1's.)
                if n.is_none() {
                    note(|| format!("Square while KH1 has him knocked away: KH2's Retaliating Slash (row {id}) has no KH1 row of its own built: nothing"));
                    return Some(all);
                }
                return None;
            }
        }
        return None;
    }
    // Square in a swing of base Sora's combo: KH2's Square move there (Finishing Leap, Upper Slash,
    // Horizontal Slash), its count KH1's, by the exe's own order and checks (Steam KH2
    // `0x1403A81F0` / `0x1403A9950`, `Character::would_take_square_exe`, the same path as Valor's
    // `valor_square`): KH2's combo / finisher window and free state at KH1's time in our KH2 slot
    // playing (`kh2slots::now_playing`, the Base / Aerial Base Combo KH2 rows); with KH1's own
    // motion playing (the combo style KH1) KH1's combo window (slots 98..104) stands for KH2's
    // combo window, no finisher window (**Assumption**: KH1's own swings have no KH2 windows).
    // (Not in KH2's guard repel: KH1's combo window there is Counterguard's, `counter_window`.)
    if square_fresh && unsafe { super::kh2slots::playing_motion(actor) } != Some(character_core::motion::GUARD) {
        let kh2 = unsafe { super::kh2slots::now_playing(actor) }.and_then(|(m, t)| {
            let clip = pose::kh2_moves()?.moveset.clips.get(&m)?;
            let f = t as u32;
            Some((clip.has(f, |k| matches!(k, character_core::WindowKind::Combo)), clip.has(f, |k| matches!(k, character_core::WindowKind::Finisher)), clip.free_at(f)))
        });
        let (combo, finisher, free) = match kh2 {
            Some(w) => w,
            None => (s.combo_window && matches!(pose::sora_slot(), Some(98..=104)), false, false),
        };
        if !(combo || finisher) {
            return None;
        }
        let (count, _, hit) = unsafe { kh1_combo(actor, air) }?;
        if count == 0 {
            return None;
        }
        let from = ComboState { count, finishers: FINISHERS.load(Ordering::Relaxed).max(1), hit, finisher_next: false };
        let Some((row, launch)) = (unsafe { takeover::kh2_would_take_square(actor, from, combo, finisher, free, &[], air) }) else {
            note(|| format!("Square in a swing (count {count}, combo window {combo}, finisher window {finisher}, connected {hit}): KH2 takes no Square move here"));
            return None;
        };
        // One with a KH1 row of its own (`kh1rows`): KH1's Attack command for it, as KH1's
        // menu gives it, so KH1's control step runs its Attack step now (its count, its target,
        // its press in the combo window), whose row is ours (`attack`, `take_ours`).
        if let Some(n) = ours_for(&row) {
            if let Ok(mut g) = OURS.lock() {
                *g = Some((n, launch, frame()));
            }
            unsafe { super::kh1rows::command_attack() };
            note(|| format!("Square in KH1's combo window (swing {count}): KH2's Square move, row {} motion {}, on KH1's row {} of its own", row.id, row.motion, super::kh1rows::row(n)));
            return Some(super::shared::KH1_ACTION_BIT);
        }
        // (No KH1 row of its own built for it (its slot didn't build at load): Square does nothing,
        // not KH1's roll or guard in its place (no fallback, the tester, combo row selection after
        // m26, answer 1). With no Square move passing KH2's checks Square is the roll or guard, as
        // above, m20 answer 5 (a).)
        note(|| format!("Square in KH1's combo window (swing {count}): KH2's Square move row {} motion {} has no KH1 row of its own built: nothing", row.id, row.motion));
        return Some(super::shared::KH1_ACTION_BIT);
    }
    None
}

/// Valor Form's Square on KH1's own states (`form::native_now`; the tester, what-is-still-missing
/// after m12: "3. do both", "check the kh2 c decomp steam"). KH2's action-button handler (Steam
/// KH2 `0x1403A81F0`, Checked (decomp)) in the order it tries them, for Valor (no glide, dodge
/// roll or Quick Run of its own: `form::form_abilities`, Checked (files) its growth is High Jump):
/// 1. its Square combo rows 0x25, 0x0C, 0x26 (`0x1403A9950`): Valor's PTYA (moveset 2, Checked
///    (files)) has 12 (ground, motion 155) and 38 (air, motion 183, a hop of 14), neither flagged
///    a finisher, and no 37 of its own (Sora's Finishing Leap ported by `pose::port_moves` is left
///    out: not in KH2's table). Taken in KH2's combo window (range id 11) with a swing left and
///    the swing connected, never in the finisher window (`Character::would_take_square_exe`).
/// 2. row 11 (`0x1403A9C70`, with the stick at rest and again after the roll / Quick Run checks
///    with it pushed: so standing or moving): Valor's is motion 181 with a hop of 90 (16..24
///    sideways, flag 2), ability 0, from a free state on the ground (`0x1403BFB30`: range id 0 / 1
///    / 2; here KH1's ground state and can-act test). A new combo (`0x1403D7C60` at the handler's
///    top: counts 1 / 1).
/// Each on its KH1 row of ours with Valor's slot (`ours_for`: 155, 183 and 181 are on our combo
/// rows of those motions in Valor, `kh2slots::VALOR_SPECS` / `valor_ours`), through KH1's own
/// Attack step (`kh1rows::command_attack`), as base Sora's Square moves. The bits KH1 doesn't see.
pub fn valor_square(s: &super::shared::Seen, square_fresh: bool) -> Option<u32> {
    if !square_fresh || !super::form::native_now() || !super::kh1rows::installed() {
        return None;
    }
    let actor = unsafe { sora() };
    if actor < 0x10000 || s.launched {
        return None;
    }
    let air = matches!(s.state, 1 | 2);
    // 1. In a swing of Valor's: KH2's combo / finisher window at the motion's frame.
    let windows = unsafe { super::kh2slots::now_playing(actor) }.and_then(|(m, t)| {
        let clip = pose::kh2_moves()?.moveset.clips.get(&m)?;
        let f = t as u32;
        Some((m, f, clip.has(f, |k| matches!(k, character_core::WindowKind::Combo)), clip.has(f, |k| matches!(k, character_core::WindowKind::Finisher)), clip.free_at(f)))
    });
    if let Some((m, f, combo, finisher, free)) = windows.filter(|w| w.2 || w.3) {
        let (count, _, hit) = unsafe { kh1_combo(actor, air) }?;
        let from = ComboState { count, finishers: FINISHERS.load(Ordering::Relaxed).max(1), hit, finisher_next: false };
        let Some((row, launch)) = (unsafe { takeover::kh2_would_take_square(actor, from, combo, finisher, free, &pose::PORTED_TO_VALOR, air) }) else {
            note(|| format!("Valor: Square in KH2 motion {m} frame {f} (combo window {combo}, finisher window {finisher}, count {count}, connected {hit}): KH2 takes no Square move here"));
            return None;
        };
        let Some(n) = ours_for(&row) else {
            note(|| format!("Valor: Square: KH2's row {} motion {} has no KH1 row of its own built: nothing", row.id, row.motion));
            return None;
        };
        if let Ok(mut g) = OURS.lock() {
            *g = Some((n, launch, frame()));
        }
        unsafe { super::kh1rows::command_attack() };
        note(|| format!("Valor: Square in KH2 motion {m} frame {f} (count {count}): KH2's row {} motion {} on KH1's row {} of ours{}", row.id, row.motion, super::kh1rows::row(n), launch.map_or(String::new(), |l| format!(", hop {:.0} high", l.height))));
        return Some(super::shared::KH1_ACTION_BIT);
    }
    // 2. Free on the ground: row 11.
    if s.state != 0 || !s.can_act {
        return None;
    }
    let (row, launch) = unsafe { takeover::kh2_would_take_row(actor, 11, false) }?;
    let Some(n) = ours_for(&row) else {
        note(|| format!("Valor: Square: KH2's row 11 motion {} has no KH1 row of its own built: nothing", row.motion));
        return None;
    };
    // (An air row from the ground needs its hop: KH2's, finite.)
    let o = &super::kh1rows::OURS[n];
    if o.kind.air() && !launch.is_some_and(|l| l.height.is_finite() && l.velocity.iter().all(|v| v.is_finite())) {
        return None;
    }
    if let Ok(mut g) = OURS.lock() {
        *g = Some((n, launch, frame()));
    }
    unsafe { super::kh1rows::command_attack() };
    note(|| format!("Valor: Square free on the ground: KH2's row 11 (motion {}) on KH1's row {} of ours{}", row.motion, super::kh1rows::row(n), launch.map_or(String::new(), |l| format!(", hop {:.0} high, {:.1}/frame", l.height, l.velocity[0].hypot(l.velocity[1])))));
    Some(super::shared::KH1_ACTION_BIT)
}

/// KH1's knock-away ends here for a move of KH2's out of it (as a spell's end frames cut KH1's
/// move, and as the borrow did): its "action running" flag (`+0` bit 4) off, so the move's own
/// request and flags are what KH1 plays from now. **Assumption**: KH1's reaction keeps no other
/// state that brings its knock-away back (the borrow's hand-back saw none).
unsafe fn end_kh1_reaction(actor: usize) {
    wr(actor, rd::<u32>(actor) & !4);
    BLOWN_AT.store(0, Ordering::Relaxed);
}

/// KH1's motion request start (`0x14029F980 (actor, request, blend)`: a fresh queue; bit 24 sets
/// the action flag 4) and its queue (`0x14029FA30`: one more after it).
const SET_MOTION_RVA: Rva = Rva(0x29D7F0);
const QUEUE_MOTION_RVA: Rva = Rva(0x29D8A0);

/// Aerial Recovery's KH1 request of its own is built (`kh1rows`, `kh2slots`).
fn recovery_built() -> bool {
    super::kh1rows::by_motion(character_core::motion::AIR_RECOVERY).is_some_and(|n| super::kh2slots::has(super::kh1rows::request(n) as usize, character_core::motion::AIR_RECOVERY))
}

/// KH2's Aerial Recovery out of KH1's knock-away, on KH1's air state (the tester, m14 question 3:
/// "(b)", a new KH1 request for KH2's motion; `docs/aerial-recovery.md`, Checked (exe) there):
/// KH1's knock-away ended (`end_kh1_reaction`), our request of KH2's motion 40 asked for at once
/// with no blend and KH1's action flag (the exe: motion 40, no blend), KH1's fall queued after it
/// (motion 4), and KH2's hop on KH1's air fields (`shared::physics::kh2_hop`): 100 high
/// (`plyr.blow_recovery_h`), 10 a frame along the stick (`blow_recovery_v`) and facing it, or
/// straight up with his facing kept; the launch's own velocity gone. No Aerial Dodge after it
/// (the exe's jump action grants none out of an airborne one). Its windows are KH1's from our
/// slot: invulnerable 0..16 (`kh2slots::invulnerable`), the guard 124 on his body 0..16
/// (`body_guard`), and KH1's "may be left" from frame 24 (KH2's air free state, range id 1,
/// `kh2slots::kh1_tracks`). False (KH1's own jump then) without our request built or out of KH1's
/// air state.
unsafe fn recover(actor: usize) -> bool {
    let Some(n) = super::kh1rows::by_motion(character_core::motion::AIR_RECOVERY) else { return false };
    let request = super::kh1rows::request(n);
    if !super::kh2slots::has(request as usize, character_core::motion::AIR_RECOVERY) || rd::<i32>(actor + 0x70) != 2 {
        return false;
    }
    let base = takeover::game_base();
    end_kh1_reaction(actor);
    let set: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
    let queue: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + QUEUE_MOTION_RVA);
    set(actor, request | 0x0100_0000, 0.0);
    queue(actor, 0x4000_0006, 0.0);
    let stick = takeover::stick_now(actor);
    let tilt = stick[0].hypot(stick[1]);
    let vel = if tilt > 1e-3 {
        let f = stick[0].atan2(stick[1]);
        wr(actor + 0x30, [f.sin(), 0.0, f.cos(), f]);
        wr(actor + 0x150, f);
        [stick[0] / tilt * character_core::AIR_RECOVERY_SPEED, stick[1] / tilt * character_core::AIR_RECOVERY_SPEED]
    } else {
        [0.0, 0.0]
    };
    super::shared::physics::kh2_hop(actor, character_core::AIR_RECOVERY_HOP, vel);
    super::shared::mark_air_dodged();
    if let Ok(mut l) = LAST.lock() {
        *l = None;
    }
    note(|| format!("jump while KH1 has him knocked away: KH2's Aerial Recovery on KH1's request {request:#x} of its own, {:.0} high, {:.1}/frame", character_core::AIR_RECOVERY_HOP, vel[0].hypot(vel[1])));
    true
}

/// KH1's state change (`0x1402B1960 (actor, state)`, Steam; its exit and enter steps run).
const SET_STATE_RVA: Rva = Rva(0x2AF7D0);

/// KH2's Aerial Dodge on KH1's air state (the tester, thread aerial-dodge-native: "go ahead",
/// plan item 1; `docs/kh1-aerial-dodge-native.md`). KH2's jump press in an air action
/// (`0x1403CB9F0`, Steam KH2, Checked (decomp)): motion 202 with no blend, the growth level's
/// height (`fmab +4`) and speed (`+8`) along the stick, straight up with it neutral, the launch
/// velocity stored (`0x1403CBB00`: what KH2's glide takes back). Here: our request of KH2's
/// motion 202 (`kh1rows`, its slot `kh2slots`) asked with KH1's action flag (free from 202's
/// air-free frame 4, its `CAN_LEAVE`), his facing turned to the stick, KH2's hop on KH1's air
/// fields (`shared::physics::kh2_hop`), the launch kept for KH2's glide (`takeover::
/// note_launch`), once per air time (`shared::mark_air_dodged`). KH1's jump state (1, its
/// take-off) is put in its air state first, as KH1's own launch `0x1402AF960` does (state 2, the
/// jump's motion base `+0x3D0` 0, flag `0x10000` off). No borrow: KH2's controller doesn't take
/// him. False (the borrow then) out of KH1's jump / air state, in a Drive Form, or with our
/// request not built.
pub unsafe fn aerial_dodge(actor: usize) -> bool {
    let Some(g) = takeover::kh2_air_dodge_growth() else { return false };
    aerial_dodge_with(actor, g)
}

/// `aerial_dodge` with the level's (height, speed) given (`takeover`, out of KH2's glide's
/// borrow, where KH2's controller is held).
pub unsafe fn aerial_dodge_with(actor: usize, (height, speed): (f32, f32)) -> bool {
    use character_core::motion::AERIAL_DODGE;
    if actor < 0x10000 || super::form::in_form() {
        return false;
    }
    let Some(n) = super::kh1rows::by_motion(AERIAL_DODGE) else { return false };
    let request = super::kh1rows::request(n);
    let state = rd::<i32>(actor + 0x70);
    if !super::kh1rows::installed() || !super::kh2slots::has(request as usize, AERIAL_DODGE) || !matches!(state, 1 | 2) {
        return false;
    }
    let base = takeover::game_base();
    if state != 2 {
        let set_state: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + SET_STATE_RVA);
        set_state(actor, 2);
        wr(actor + 0x3D0, 0u32);
        wr(actor, rd::<u32>(actor) & !0x1_0000);
    }
    let set: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
    set(actor, request | 0x0100_0000, 0.0);
    let stick = takeover::stick_now(actor);
    let tilt = stick[0].hypot(stick[1]);
    let vel = if tilt > 1e-3 {
        let f = stick[0].atan2(stick[1]);
        wr(actor + 0x30, [f.sin(), 0.0, f.cos(), f]);
        wr(actor + 0x150, f);
        [stick[0] / tilt * speed, stick[1] / tilt * speed]
    } else {
        [0.0, 0.0]
    };
    // (Held: physics KH2, let go early the rise is cut as KH2's held jump is; physics KH1, the
    // full rise always, `kh2_hop_held`.)
    super::shared::physics::kh2_hop_held(actor, height, vel);
    super::shared::mark_air_dodged();
    takeover::note_launch(vel);
    DODGE_SKIP_AT.store(frame().max(1), Ordering::Relaxed);
    note(|| format!("jump in the air: KH2's Aerial Dodge on KH1's request {request:#x} of its own (KH1 state {state}), {height:.0} high, {:.1}/frame", vel[0].hypot(vel[1])));
    true
}

/// KH2's Aerial Dodge swaps to its fall this many frames before its arc's apex (KH2's air step
/// `0x1403CB680`: with the arc's time `+0xF0` + 5 at or past its rise time `+0xF4`, the apex
/// motion `+0x118`, which the dodge set to 4, through `0x1403CB610`; Checked (decomp)).
const DODGE_FALL_MARGIN: f32 = 5.0;
/// KH2's blend into that fall (`0x1403CB610`: motion 4 with blend 8.0, Checked (decomp)).
const DODGE_FALL_BLEND: f32 = 8.0;

/// Each KH1 physics frame of Sora's, before it (`takeover::physics`): KH2's Aerial Dodge on our
/// request (`aerial_dodge`) goes into KH1's fall (request 6) at KH2's apex swap, 5 frames before
/// the apex of his arc as KH1's air fields keep it (`+0x3B0` its time, `+0x3B8` its rise time;
/// KH2's arc writes them at physics KH2, KH1's own at physics KH1), with KH2's blend. KH1's own
/// apex swap (`0x1402AF630`) only fires for its own jump's requests, so this is ours to ask, as
/// KH2's own air step asks it. Nothing while something else is pending in KH1's queue (`+0x178`),
/// as KH1's apex swap. KH1's action flag 4 the request raises is cleared (a plain fall).
pub unsafe fn dodge_apex(actor: usize) {
    use character_core::motion::AERIAL_DODGE;
    dodge_clock(actor);
    if actor < 0x10000 || rd::<i32>(actor + 0x70) != 2 || super::kh2slots::playing_motion(actor) != Some(AERIAL_DODGE) {
        return;
    }
    let (t, up) = (rd::<f32>(actor + 0x3B0), rd::<f32>(actor + 0x3B8));
    if !(t.is_finite() && up.is_finite()) || t + DODGE_FALL_MARGIN < up || rd::<i32>(actor + 0x178) != 0 {
        return;
    }
    dodge_fall(actor, &format!("its apex, {t:.1} of {up:.1} frames up"));
}

/// KH1's motion fields: the time in the slot playing, the blend length (0: no blend under way)
/// and the motion's speed (KH1's update `0x14029F0A0` and apply `0x1402A0350`, Steam; Checked
/// (decomp), as `glide::kh2_clock` reads them).
const SLOT_TIME: usize = 0x16C;
const BLEND_LEN: usize = 0x170;
const MOTION_SPEED: usize = 0x284;

/// The game frame our Aerial Dodge request was asked on (`aerial_dodge_with`), until KH1's apply
/// of it has been seen (`dodge_clock`); 0: none.
static DODGE_SKIP_AT: AtomicU64 = AtomicU64::new(0);

/// KH2's skip of frame 1 on the Aerial Dodge's motion 202 (the tester, aerial-dodge-native after
/// m4: the glide out of the dodge came later than in KH2). KH2 starts 202 with no blend at time 0
/// in its input step, and its next motion update (`0x1403C6740`, Steam KH2) turns a time stepping
/// from exactly 0 to 1 into 2, so 202 plays 0, 2, 3, 4 and its air-free range (id 1, from frame 4,
/// `+0x18C` bit 8 through the range step `0x14040D810`) is on 3 updates after the press; KH1
/// plays our slot 0, 1, 2, 3, 4, one frame behind (Checked (decomp) both). On the first physics
/// frame after KH1 applied our dodge request (still at time 0, no blend, a 60 Hz step of 1), its
/// time is put at 1, so KH1's update makes it KH2's 2: pose, sound (999019 at frame 1, fired
/// between 0 and 2 as KH2 does, `kh2slots::tick`), guard and the free frame keep KH2's timing.
/// At 30 fps (a step of 2) KH2 doesn't skip either; nothing is done.
unsafe fn dodge_clock(actor: usize) {
    use character_core::motion::AERIAL_DODGE;
    let at = DODGE_SKIP_AT.load(Ordering::Relaxed);
    if at == 0 || actor < 0x10000 {
        return;
    }
    if frame().saturating_sub(at) > 3 {
        DODGE_SKIP_AT.store(0, Ordering::Relaxed);
        return;
    }
    let Some((m, t)) = super::kh2slots::now_playing(actor) else { return };
    if m != AERIAL_DODGE {
        return;
    }
    DODGE_SKIP_AT.store(0, Ordering::Relaxed);
    let (len, speed) = (rd::<f32>(actor + BLEND_LEN), rd::<f32>(actor + MOTION_SPEED));
    let step = super::shared::physics::dt() * if speed.is_finite() && speed > 0.0 { speed } else { 1.0 };
    let skip = t == 0.0 && !(len > 0.0) && (step - 1.0).abs() < 1e-4;
    if skip {
        wr(actor + SLOT_TIME, 1.0f32);
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if super::log_on(super::LogCat::Combat) && N.fetch_add(1, Ordering::Relaxed) < 12 {
        log!(Combat, "{}", if skip {
            "KH2's Aerial Dodge: KH2's skip of frame 1 (KH1's update makes its time 2)".to_string()
        } else {
            format!("KH2's Aerial Dodge: no skip of frame 1 (time {t:.2}, blend {len:.1}, step {step:.2})")
        });
    }
}

/// KH2's Aerial Dodge into KH1's fall now (request 6, KH2's blend; KH1's action flag 4 the
/// request raises off): at its apex (`dodge_apex`), or as the held jump is let go early
/// (`shared::physics`'s air step: KH2's release also swaps to the apex motion before its hop,
/// `0x1403CB680` → `0x1403CB610`, Checked (decomp)). Only while our dodge slot plays.
pub unsafe fn dodge_fall(actor: usize, why: &str) {
    if actor < 0x10000 || super::kh2slots::playing_motion(actor) != Some(character_core::motion::AERIAL_DODGE) {
        return;
    }
    let set: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(takeover::game_base() + SET_MOTION_RVA);
    set(actor, 0x06, DODGE_FALL_BLEND);
    wr(actor, rd::<u32>(actor) & !4);
    note(|| format!("KH2's Aerial Dodge: {why}, KH1's fall (request 6, blend {DODGE_FALL_BLEND})"));
}

/// KH1 knocked him away itself (`hurt`: a knock-back or launch while KH1's states have him).
pub fn kh1_blown() {
    BLOWN_AT.store(frame().max(1), Ordering::Relaxed);
}

/// KH1 has him knocked away in the air, far enough in for KH2's Aerial Recovery.
pub unsafe fn launched(actor: usize) -> bool {
    let at = BLOWN_AT.load(Ordering::Relaxed);
    if at == 0 || actor < 0x10000 {
        return false;
    }
    let f = frame().saturating_sub(at);
    if f > BLOWN_MAX {
        BLOWN_AT.store(0, Ordering::Relaxed);
        return false;
    }
    f >= RECOVERY_AFTER && rd::<f32>(actor + 0x3D4) > AIR
}

/// An Attack in KH2's move with the combo style KH1's where he is (`combat::command`): what comes
/// next.
#[derive(Clone, Copy, Debug)]
pub enum Next {
    /// KH2's next is one of its ability moves again: KH2 keeps the press.
    Kh2,
    /// KH1's next row, its combo at KH2's count.
    Kh1 { count: u8, hit: bool },
    /// No combo of KH2's going: KH1's own combo from its start.
    Fresh,
}

pub unsafe fn next_from_kh2(actor: usize, air: bool) -> Next {
    let Some((state, going, master, in_finisher)) = takeover::kh2_combo() else { return Next::Fresh };
    // In KH2's finisher: its next finisher with Finishing Plus (KH2's), else a new combo of KH1's.
    if in_finisher {
        let pick = takeover::kh2_would_take(actor, None, None, air, false).filter(|(r, ..)| r.is_finisher());
        if let Some((r, ..)) = pick {
            note(|| format!("Attack in KH2's finisher with the combo style KH1's: KH2's next finisher, row {} motion {} (Finishing Plus)", r.id, r.motion));
            return Next::Kh2;
        }
        return Next::Fresh;
    }
    if !going || state.count == 0 {
        return Next::Fresh;
    }
    let kh1_len = kh1_combo(actor, air).map_or(u8::MAX, |c| c.1);
    let pick = takeover::kh2_would_take(actor, None, None, air, false);
    let next = state.count.saturating_add(1);
    let why = pick.as_ref().and_then(|(r, len, neg)| ability_move(r, *len, *neg, next, kh1_len));
    let out = if why.is_some() { Next::Kh2 } else { Next::Kh1 { count: state.count, hit: state.hit || master } };
    note(|| format!(
        "Attack in KH2's combo (count {}) with the combo style KH1's: KH2 would take {}: {out:?}{}",
        state.count,
        pick.as_ref().map_or("nothing".to_string(), |(r, ..)| format!("row {} motion {}", r.id, r.motion)),
        why.map_or(String::new(), |w| format!(" ({w})"))
    ));
    out
}

/// What `kh1_count_in` changed, put back after KH1's step.
pub struct Saved {
    info: usize,
    /// The hit flags before, if we set them.
    hit: Option<u32>,
    window: bool,
}

/// Before KH1's Attack step out of KH2's combo: KH1's combo at KH2's count (its count, its combo
/// window, its swing's hit), so the step goes on from there (one more, its finisher at its
/// length).
pub unsafe fn kh1_count_in(actor: usize, count: u8, hit: bool) -> Option<Saved> {
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    if info < 0x10000 {
        return None;
    }
    let flags = rd::<u32>(actor + 0x1CC);
    let before = rd::<u32>(info + 0x50);
    let saved = Saved { info, hit: (hit && before == 0).then_some(before), window: flags & KH1_COMBO_WINDOW != 0 };
    wr(info + 0x41, count);
    wr(actor + 0x1CC, flags | KH1_COMBO_WINDOW);
    if saved.hit.is_some() {
        wr(info + 0x50, 1u32);
    }
    Some(saved)
}

pub unsafe fn restore(actor: usize, s: Saved) {
    // (Only if KH1's step made no attack object of its own swing, which clears them itself.)
    if let Some(h) = s.hit.filter(|_| rd::<u32>(s.info + 0x50) == 1) {
        wr(s.info + 0x50, h);
    }
    if !s.window {
        wr(actor + 0x1CC, rd::<u32>(actor + 0x1CC) & !KH1_COMBO_WINDOW);
    }
    note(|| format!("KH1's combo goes on from KH2's: count now {}", rd::<u8>(s.info + 0x41)));
}

/// KH2's guard press, its repel and Counterguard on KH1's states (the tester, combo row selection
/// m15 question 3: "(a)", Guard native, then Counterguard on its own KH1 row). KH2's rules (the
/// OG rewrite's `Character::try_action_button` / `clash` / its Hurt step; the windows from the
/// motion files, Checked (files): P_EX100.mset and W_EX010.mset):
/// - **The press**: Square on the ground with the stick at rest (KH2's dead zone 0), Guard
///   equipped (KH2's row 11, motion 173), the Dodge Roll style KH2 (where Square is KH2's): our
///   request for 173 asked for at once with KH1's action flag (no blend, his speed 0, as KH2's
///   `vel_h = 0`), facing KH1's locked target if one is held. Its Keyblade guard 161 (frames
///   6..37) stops a hit from in front (`hurt`, `kh2slots::keyblade_guard`, and KH1 guard objects
///   from the same window, `takeover::kh2_hits` -> `combat::guard_step`). KH2's free state from
///   52 (range id 2) is KH1's "may be left".
/// - **The repel** (`guard_repel`, the guard's `refact_self` 1): our request for motion 42,
///   facing the attacker, pushed back 200 (KH2's hit-back: 20 a frame, x 0.9 a frame) through
///   KH1's per-frame push (`+0x120`, `guard_frame`). Its own Keyblade guard 162 (0..40,
///   `refact_self` 2) stops hits from in front with nothing more. Free from 40.
/// - **Counterguard** (row 39, motion 171, ability 96): Attack any frame of the repel on the
///   ground. KH1's Attack step takes it in KH1's combo window (`counter_window`, as Finishing
///   Plus's), and our row runs it (`counterguard`): its body hit 1581, its invulnerability and
///   body guard 0..20 are our slot's (`takeover::kh2_hits`, `hurt`).
static PUSH: Mutex<Option<([f32; 2], f32)>> = Mutex::new(None);
/// KH2's Counterguard ability (its PTYA row 39's ability, Checked (files)).
const COUNTERGUARD_ABILITY: u16 = 96;
/// KH2's guard repel's push back (`Character::clash`: `hit_back(.., 200)`, speed 0.1 of it, x 0.9 a
/// frame).
const REPEL_BACK: f32 = 200.0;

/// Our request for KH2's `motion` (a move of ours started by its request alone) asked for now on
/// the ground, facing `face` (yaw) if given: KH1's action flag, no blend, the idle queued after.
unsafe fn start_own(actor: usize, motion: u16, face: Option<f32>) -> bool {
    let Some(n) = super::kh1rows::by_motion(motion) else { return false };
    let request = super::kh1rows::request(n);
    if !super::kh1rows::installed() || !super::kh2slots::has(request as usize, motion) || rd::<i32>(actor + 0x70) != 0 {
        return false;
    }
    let base = takeover::game_base();
    if let Some(f) = face.filter(|f| f.is_finite()) {
        wr(actor + 0x30, [f.sin(), 0.0, f.cos(), f]);
        wr(actor + 0x150, f);
    }
    // (KH2 stops him: `vel_h = 0`. **Assumption**: KH1's ground step moves him by `+0x2C` while
    // its action flag is set, as its rows' lunges do.)
    wr(actor + 0x2C, 0.0f32);
    let set: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
    let queue: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + QUEUE_MOTION_RVA);
    set(actor, request | 0x0100_0000, 0.0);
    queue(actor, 0x4000_0000, 0.0);
    if let Ok(mut l) = LAST.lock() {
        *l = None;
    }
    FINISHERS.store(0, Ordering::Relaxed);
    true
}

/// The yaw (KH1's `+0x3C`, as `recover` writes it) toward `d` (x / z).
fn yaw_to(d: [f32; 2]) -> Option<f32> {
    (d[0].hypot(d[1]) > 1e-3).then(|| d[0].atan2(d[1]))
}

/// Square on the ground with the Dodge Roll style KH2 (`shared::control`): KH2's guard press on
/// KH1's states, if KH2 would guard here (stick at rest, Guard equipped) and our request is built.
pub fn guard_press() -> bool {
    if super::form::in_form() || takeover::move_tilt() > 0.0 || takeover::with_abilities(|a| a.guard) != Some(true) {
        return false;
    }
    unsafe {
        let actor = sora();
        if actor < 0x10000 {
            return false;
        }
        let me = rd::<[f32; 3]>(actor + 0x10);
        let face = super::kh1rows::lock_actor().and_then(|t| {
            let p = rd::<[f32; 3]>(t + 0x10);
            yaw_to([p[0] - me[0], p[2] - me[2]])
        });
        if !start_own(actor, character_core::motion::GUARD_PRESS, face) {
            return false;
        }
        if let Ok(mut p) = PUSH.lock() {
            *p = None;
        }
        note(|| format!("Square standing: KH2's guard (motion 173) on KH1's request {:#x} of its own{}", super::kh1rows::by_motion(character_core::motion::GUARD_PRESS).map_or(0, super::kh1rows::request), if face.is_some() { ", facing KH1's lock" } else { "" }));
        true
    }
}

/// KH1's roll start (Steam `0x1402C9E60`, Checked (decomp)): its 12-frame no-damage timer
/// (`0x140297F30 (actor, 12.0)`: `+0xC` 12.0, flags `0x80 | 0x100000`), request `0xDC | 0x1000000`
/// (KH1's action flag 4), the idle queued after (blend 8).
const ROLL_START_RVA: Rva = Rva(0x2C7CB0);
/// KH1's deep-water test (Steam `0x1402C16E0`), which keeps KH1's Square handler from rolling.
const DEEP_WATER_RVA: Rva = Rva(0x2BF530);
/// KH1's no-damage timer flags (`0x140297F30` sets both; `0x140297FE0` counts `+0xC` down and
/// clears them).
const KH1_TIMER_FLAGS: u32 = 0x80 | 0x10_0000;
/// The native roll's KH2 invulnerable frames (`fmab` `dodge_roll_star_time` of its level, f32
/// bits), from its start.
static ROLL_STAR: AtomicU32 = AtomicU32::new(0);

/// Square on the ground while moving with the Dodge Roll style KH2 (`shared::control`, after
/// `guard_press`: KH2's order, `0x1403A81F0`, the guard only with the stick at rest, then the
/// dodge roll): KH2's dodge roll natively on KH1's own roll (the tester, thread
/// dodge-roll-guard-and-counterguard-native, plan step 1; `docs/kh1-roll-guard-native.md` §5).
/// KH1's roll start runs (no borrow), its request `0xDC` playing our slot of KH2's motion 209
/// (`kh2slots`'s roll spec, pointed there by the style), which KH1 moves by KH2's root travel and
/// lets go from KH2's fall-check frame. KH2's start (the OG rewrite's `try_action_button`): facing
/// along the stick at once, his speed 0. KH1's 12-frame no-damage timer the start sets is put back
/// as it was (KH2's roll has none: its invulnerability is the level's `dodge_roll_star_time`
/// frames, `roll_invulnerable`, in `hurt`). Only with KH2's Dodge Roll equipped (KH2's ability
/// check, not KH1's), base Sora, KH1's ground state, not in deep water (KH1's own test), our slot
/// built and the style KH2. False: the borrow, as before.
pub fn roll_press() -> bool {
    use character_core::motion::DODGE_ROLL;
    if super::form::in_form() || !super::styles::kh2(super::styles::Kind::DodgeRoll) || !super::kh2slots::has(0xDC, DODGE_ROLL) {
        return false;
    }
    let tilt = takeover::move_tilt();
    let Some((level, star)) = takeover::kh2_dodge_roll_growth() else { return false };
    if tilt <= 0.0 {
        return false;
    }
    unsafe {
        let actor = sora();
        if actor < 0x10000 || rd::<i32>(actor + 0x70) != 0 {
            return false;
        }
        let base = takeover::game_base();
        let deep: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + DEEP_WATER_RVA);
        if deep(actor) != 0 {
            return false;
        }
        let stick = takeover::stick_now(actor);
        let Some(f) = yaw_to(stick) else { return false };
        wr(actor + 0x30, [f.sin(), 0.0, f.cos(), f]);
        wr(actor + 0x150, f);
        // (KH2: `vel_h = 0`, the clip's root moves him. **Assumption**, as `start_own`: KH1's
        // ground step moves him by `+0x2C` while its action flag is set.)
        wr(actor + 0x2C, 0.0f32);
        let flags = rd::<u32>(actor) & KH1_TIMER_FLAGS;
        let timer = rd::<f32>(actor + 0xC);
        let start: unsafe extern "system" fn(usize) = std::mem::transmute(base + ROLL_START_RVA);
        start(actor);
        // (KH1's roll timer undone; a timer already running, a reaction's, kept. **Assumption**:
        // nothing else reads `0x100000` as "rolling", `styleroll`'s note.)
        wr(actor, (rd::<u32>(actor) & !KH1_TIMER_FLAGS) | flags);
        wr(actor + 0xC, timer);
        ROLL_STAR.store(star.to_bits(), Ordering::Relaxed);
        note(|| format!("Square while moving: KH2's dodge roll LV{level} on KH1's roll (request 0xDC, KH2 motion {DODGE_ROLL}), invulnerable {star:.0} frames"));
        true
    }
}

/// KH2's dodge roll on KH1's roll (`roll_press`) in its invulnerable frames now: our roll slot
/// playing at KH1's time under the level's `dodge_roll_star_time`, or its request still pending
/// (KH1 applies a request in its animation update, after the physics step: `+0x178` pending,
/// `+0x17C` the request). `hurt` lets no hit reach him there, as KH2's roll (the OG rewrite's
/// DodgeRoll action: invulnerable while its frame is under the star time).
pub unsafe fn roll_invulnerable(actor: usize) -> bool {
    use character_core::motion::DODGE_ROLL;
    if actor < 0x10000 || !super::styles::kh2(super::styles::Kind::DodgeRoll) {
        return false;
    }
    let star = f32::from_bits(ROLL_STAR.load(Ordering::Relaxed));
    if let Some((m, t)) = super::kh2slots::now_playing(actor) {
        if m == DODGE_ROLL {
            return t < star;
        }
    }
    rd::<u32>(actor + 0x178) != 0 && rd::<u32>(actor + 0x17C) & 0xFFFF == 0xDC && star > 0.0 && super::kh2slots::has(0xDC, DODGE_ROLL)
}

/// KH2's guard stopped a hit in our guard slot (`hurt`, or a KH1 guard object of its window,
/// `combat::guard_step`): `toward` the attacker (x / z), the guard's `refact_self`. 1 (the press's
/// 161): KH2's repel (`Character::clash`, on the ground): motion 42 facing the attacker, pushed
/// back 200. 2 (the repel's own 162): nothing more.
pub unsafe fn guard_repel(actor: usize, toward: [f32; 2], refact: u8, attacker_priority: Option<i32>) {
    if refact != 1 || actor < 0x10000 || super::form::in_form() {
        return;
    }
    // (Once a frame: the hit pass and a guard object of the same window can both stop something
    // in it; the repel's own guard is `refact_self` 2.)
    static AT: AtomicU64 = AtomicU64::new(0);
    let now = frame() + 1;
    let last = AT.swap(now, Ordering::Relaxed);
    if last != 0 && now.saturating_sub(last) <= 1 {
        return;
    }
    // (Recoil KH1: KH1's own block reaction instead, `kh1_block`.)
    if !repel_kh2() {
        kh1_block(actor, attacker_priority);
        return;
    }
    let face = yaw_to(toward);
    if !start_own(actor, character_core::motion::GUARD, face) {
        note(|| "KH2's guard repel: not started (not on the ground, or no request of ours for motion 42)".to_string());
        return;
    }
    let len = toward[0].hypot(toward[1]);
    let back = if len > 1e-3 { [-toward[0] / len, -toward[1] / len] } else { [0.0, 0.0] };
    if let Ok(mut p) = PUSH.lock() {
        *p = Some((back, REPEL_BACK * 0.1));
    }
    // KH2's 80 frames without damage after the repel (`0x1403D61D0` sets `+0xD70` = 80.0; while it
    // runs KH2's intake drops every attack but the cures, Checked (decomp); the tester, thread
    // kh2-recoil-and-parry-system, plan step 4), with the Recoil row at KH2: KH1's own no-damage
    // timer, as its reactions start it (`0x140297F30`: `+0xC` = the frames, flag `+0` 0x80 on and
    // 0x100000 off; while 0x80 is up KH1 takes no damage and starts no reaction but clashes,
    // `docs/kh1-hit-reactions-native.md` §3, Checked (decomp)). Assumption: KH2's step is 1.0 a
    // 60 Hz frame at run time, as KH1's.
    if super::recoil::kh2() {
        wr(actor + 0xC, REPEL_PROTECTION);
        wr(actor, (rd::<u32>(actor) & !0x10_0000) | 0x80);
    }
    note(|| format!("KH2's guard stopped a hit: its repel (motion 42), pushed back {REPEL_BACK:.0}{}", if super::recoil::kh2() { ", 80 frames without damage (Recoil KH2)" } else { "" }));
}

/// What a block of base Sora's KH2 guard does: KH2's repel (motion 42, pushed back 200, its 80
/// frames, Counterguard) at Recoil KH2, KH1's own block (`kh1_block`) at Recoil KH1. The Guard
/// Repel row (t536) is part of the Recoil row now (the tester, thread kh2-recoil-and-parry-system:
/// "we need to also combine guard repel and recoil into the same config"); ini `guard_repel` is no
/// longer read.
pub fn repel_kh2() -> bool {
    super::recoil::kh2()
}

/// KH1's clash recoils (Steam; the hit pass `0x14029BBD0` runs them for a clash record of kind 6
/// / 7, its handler table, Checked (decomp)): `0x14029D720 (actor, record, out)` requests 0x6E
/// (slot 72) with the idle queued, 8 frames of KH1's no-damage timer (`0x140297F30`), KH1's combo
/// count (combat `+0x41`) one less and, when the record's gap (`+8`) is below 0, a push back of
/// -gap x btltbl `+0x1C` x his stats' `+0xC4` (`0x1402C7300`); `0x14029D5F0` the big recoil 0x6F,
/// the same with no push. Both write `out + 0xC` = 0. Their first bytes are checked before a call
/// (Checked (exe)).
const CLASH_RECOIL_STEAM: usize = 0x29D720;
const CLASH_RECOIL_BYTES: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x20, 0x57, 0x48, 0x83, 0xEC, 0x40];
const BIG_RECOIL_STEAM: usize = 0x29D5F0;
const BIG_RECOIL_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];

/// KH1's own block (Recoil KH1), what KH1's clash gives the owner of a guard that stopped an
/// attack (`0x1402B4BC0 (guard, attack)`, Steam, Checked (decomp)): KH1's guard row 0x3D's object
/// (data 0x4DA8: bone 0x30, clash type 2, priority byte 0) is under 0x62 (its priority is the
/// Keyblade's `+0x33`, clamped 1..99 by `0x1402B3270`), so the guard side is a melee attack that
/// clashed: a clash record on its owner (`0x1402BF060`) of kind 6, or 7 when the attacker is more
/// than 40 stronger (gap = guard - attacker priority), and the guard's result word (combat state
/// `+0x50`, the object's `+0x60`) gets 0x10 (0x20 for the big one), which KH1's Counterattack reads
/// with request 0x6E playing (`0x140294220`: command 99, row 0x3E). Here the record's reaction is
/// run at once (KH1's recoil above), the result word marked, KH2's repel push dropped. The attacker
/// side (its own recoil) is KH1's clash on our guard objects as before. On the ground only (KH2's
/// guard is). `attacker`: the stopped attack's priority if known (**Assumption** for a hit record:
/// its attack data's `+0x3C`, which KH1's player rows add the weapon to but enemy attacks keep);
/// unknown, a gap of 0 (the small recoil, no push).
unsafe fn kh1_block(actor: usize, attacker: Option<i32>) {
    if Build::current() != Build::Steam || rd::<i32>(actor + 0x70) != 0 || super::form::in_form() {
        note(|| "KH1's block: not run (not the Steam build, not on the ground, or a Drive Form)".to_string());
        return;
    }
    let base = takeover::game_base();
    if rd::<[u8; 15]>(base + CLASH_RECOIL_STEAM) != CLASH_RECOIL_BYTES || rd::<[u8; 10]>(base + BIG_RECOIL_STEAM) != BIG_RECOIL_BYTES {
        note(|| "KH1's block: KH1's clash recoils (0x14029D720 / 0x14029D5F0) aren't the expected code here; not run".to_string());
        return;
    }
    let combat = pose::ptr(rd::<u32>(actor + 0x14C));
    if combat < 0x10000 {
        return;
    }
    let weapon = rd::<u64>(combat + 0x20) as usize;
    let guard = if weapon >= 0x10000 { (rd::<u8>(weapon + 0x33) as i32).clamp(1, 99) } else { 1 };
    let gap = attacker.map_or(0, |a| guard - a);
    let big = gap < -40;
    let mut record = [0i32; 16];
    record[2] = gap;
    let mut out = [0i32; 16];
    let recoil: unsafe extern "system" fn(usize, *mut i32, *mut i32) = std::mem::transmute(base + if big { BIG_RECOIL_STEAM } else { CLASH_RECOIL_STEAM });
    recoil(actor, record.as_mut_ptr(), out.as_mut_ptr());
    wr(combat + 0x50, rd::<u32>(combat + 0x50) | if big { 0x20 } else { 0x10 });
    if let Ok(mut p) = PUSH.lock() {
        *p = None;
    }
    note(|| format!(
        "KH2's guard stopped a hit, Recoil KH1: KH1's block, its {} (request {:#x}), guard priority {guard} against {} (gap {gap}){}",
        if big { "big recoil" } else { "clash recoil" },
        if big { 0x6F } else { 0x6E },
        attacker.map_or("unknown".to_string(), |a| a.to_string()),
        if !big && gap < 0 { ", pushed back by the gap" } else { "" }
    ));
}

/// KH2's guard repel's protection (`+0xD70` = 80.0 in `0x1403D61D0` / `0x1403D6AD0`, Checked (decomp)).
const REPEL_PROTECTION: f32 = 80.0;

/// KH2's push back by `distance` along `back` (`0x1403D4960`: 0.1 × the distance a frame, × 0.9 a
/// frame) on KH1's per-frame push, as the repel's (`guard_frame`): KH2's recoil (`recoil`).
pub fn push_back(back: [f32; 2], distance: f32) {
    if let Ok(mut p) = PUSH.lock() {
        *p = Some((back, distance * 0.1));
    }
}

/// Each KH1 physics frame of Sora's, before it (`takeover::physics`): the repel's push back on
/// KH1's per-frame push (`+0x120`, which KH1's physics step adds to the move and clears), KH2's
/// 60 Hz steps for KH1's frame step.
pub unsafe fn guard_frame(actor: usize) {
    let Ok(mut g) = PUSH.try_lock() else { return };
    let Some((dir, mut v)) = *g else { return };
    if !super::shared::active() || rd::<i32>(actor + 0x70) != 0 {
        *g = None;
        return;
    }
    let steps = super::shared::physics::dt().round().clamp(1.0, 4.0) as u32;
    let mut add = [0.0f32; 2];
    for _ in 0..steps {
        if v < 1.0 {
            break;
        }
        add[0] += dir[0] * v;
        add[1] += dir[1] * v;
        v *= 0.9;
    }
    let p = rd::<[f32; 3]>(actor + 0x120);
    wr(actor + 0x120, [p[0] + add[0], p[1], p[2] + add[1]]);
    *g = (v >= 1.0).then_some((dir, v));
}

/// Our slot of KH2's repel (42) playing on the ground, Counterguard equipped (KH2 takes Attack
/// any frame of it): KH1's combo window this frame (as Finishing Plus's, `kh2slots`), KH1's count
/// at 0 and its last swing counted as connected, so KH1's Attack step takes the press as a
/// combo's first (KH2's count after a guard: a new combo).
pub unsafe fn counter_window(actor: usize) {
    if !super::shared::active() || super::form::in_form() || rd::<i32>(actor + 0x70) != 0 || super::kh2slots::playing_motion(actor) != Some(character_core::motion::GUARD) {
        return;
    }
    if takeover::with_abilities(|a| a.equipped.contains(&COUNTERGUARD_ABILITY)) != Some(true) {
        return;
    }
    let inst = pose::ptr(rd::<u32>(actor + 0x15C));
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    if inst < 0x10000 || info < 0x10000 {
        return;
    }
    wr(inst + 0x14, rd::<u32>(inst + 0x14) | KH1_COMBO_WINDOW);
    wr(info + 0x41, 0u8);
    wr(info + 0x50, rd::<u32>(info + 0x50) | 1);
}

/// KH1's top command menu (Steam `0x140528900`): its entries from `+0x14`, Attack's first.
const MENU_RVA: Rva = Rva(0x529600);
/// KH1's Counterattack command (KH1's Attack entry swap, Steam `0x140294220`, Checked (decomp)).
const KH1_COUNTERATTACK: u8 = 99;

/// Attack does KH2's Counterguard now: our slot of KH2's repel (42) playing on the ground, base
/// Sora on KH1's states, Counterguard equipped (`counter_window`'s test).
pub unsafe fn counter_open(actor: usize) -> bool {
    actor > 0x10000
        && super::shared::active()
        && !super::form::in_form()
        && rd::<i32>(actor + 0x70) == 0
        && super::kh2slots::playing_motion(actor) == Some(character_core::motion::GUARD)
        && takeover::with_abilities(|a| a.equipped.contains(&COUNTERGUARD_ABILITY)) == Some(true)
}

/// After KH1's battle-menu fill (`drive::fill`, from whichever of `shortcuts` / `reflect` / `drive`
/// hooked it): while Attack does KH2's Counterguard (`counter_open`), the menu's Attack entry shows
/// KH1's Counterattack command (99), as KH1 swaps it in during its own clash recoil (the tester,
/// thread dodge-roll-guard-and-counterguard-native, question 2: "(a)"). KH1's own swap
/// (`0x140294220`, Checked (decomp)): entry `+0x14` = 99 instead of 1 with KH1's Counterattack on
/// (ability bit `0x4` of `0x142D5EC10`), the lock-on target an enemy, his request `0x6E` (its clash
/// recoil) and his attack's result word with any bit but `0x1` / `0x100`; it then calls the fill
/// (`0x1402944C0`), so ours runs after it. Only over Attack (1): a context command (Talk) or a
/// world's own entry (0x62) stays. KH2 needs no target for Counterguard, so none is asked for here.
/// Choosing it runs Counterguard (`combat::command` turns it into KH1's Attack).
pub unsafe fn counter_entry(actor: usize) {
    if !counter_open(actor) {
        return;
    }
    let e = takeover::game_base() + MENU_RVA + 0x14;
    if rd::<u8>(e) == 1 {
        wr(e, KH1_COUNTERATTACK);
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 10 {
            note(|| "KH2's guard repel: the command menu's Attack shows KH1's Counterattack (KH2's Counterguard)".to_string());
        }
    }
}

/// An Attack KH1's step took in the repel's window (`counter_window`): KH2's Counterguard on its
/// own KH1 row, if KH2 would take row 39 now.
unsafe fn counterguard(actor: usize, air: bool, count: u8, request_of: &dyn Fn(i32) -> Option<u32>) -> Option<Run> {
    use super::kh1rows as k;
    if air || super::kh2slots::playing_motion(actor) != Some(character_core::motion::GUARD) {
        return None;
    }
    let n = k::by_kh2_row(k::COUNTERGUARD_ROW).filter(|&n| k::installed() && super::kh2slots::has(k::request(n) as usize, k::OURS[n].motion))?;
    let Some((row, launch)) = takeover::kh2_would_take_row(actor, k::COUNTERGUARD_ROW, false) else {
        note(|| "Attack in KH2's guard repel: KH2 wouldn't take Counterguard (row 39) now".to_string());
        return None;
    };
    if let Ok(mut p) = PUSH.lock() {
        *p = None;
    }
    note(|| format!("Attack in KH2's guard repel: KH2's Counterguard (row {} motion {})", row.id, row.motion));
    Some(ours_run(n, launch, false, count, request_of))
}
