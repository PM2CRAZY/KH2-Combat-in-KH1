//! KH2's abilities on KH1's own moves (testing branch, shared states, base Sora). The user:
//! "when setting attack to kh1 sora is unable to use kh2s ground combo abilities (this includes
//! kh2s square combo modifers)", the same for the air combo, and "sora is unable to use kh2s
//! general modifiers (with the exception to aerial dodge)". `docs/shared-states.md`.
//!
//! With the Base / Aerial Combo style at KH1, KH1's own combo plays: its rows, motions and hits,
//! and its own abilities, which KH1 reads itself (Combo Plus, Air Combo Plus, Combo Master and
//! KH1's modifiers are the same KH1 abilities KH2's come from). KH2's abilities on top of it are
//! borrows, as Aerial Dodge is with Jump at KH1: at the press where KH2 would use one, KH2's
//! controller borrows him for that move (`takeover::shared_frame`), KH1's swing cut there as a
//! spell's end frames are. Where none applies, KH1's own combo plays.
//!
//! - **Attack** (`attack`, from `combat::exec`: a press KH1's combo took, its row about to run):
//!   KH2's combo is asked what it would take at KH1's count (`Character::would_take`, the combo
//!   carried at KH1's count, the press, KH2's targets and KH1's lock). KH2 takes it when that is
//!   - a row that needs an ability (the finisher Guard Break ...), opener, step or finisher,
//!     except the ones KH1's own rows now play natively at either combo style (`native_ability`:
//!     Slapshot, Dodge Slash, Slide Dash, Flash Step, Vicinity Break, Explosion, Aerial Sweep,
//!     Aerial Dive, Aerial Spiral, Aerial Finish, Magnet Burst);
//!   - KH2's finisher where KH1's would still swing, with Negative Combo;
//!   - KH2's step where KH1's would finish, KH2's combo being the longer (Combo Plus KH1 didn't
//!     count);
//!   - Finishing Plus: a press during KH1's finisher is KH2's next finisher (on a KH1 row of its
//!     own, `next_finisher`; the borrow only when none can play it).
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
use character_core::{Button, Character, ComboRow, ComboState, Input};
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

fn note(s: String) {
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 300 {
        log(&s);
    }
}

/// What KH2 borrows him for.
#[derive(Clone, Copy, Debug)]
pub enum Move {
    /// That combo row (`Character::force_row`) with the combo carried at `state`, on an Attack
    /// press.
    Row { id: u8, state: ComboState },
    /// Aerial Recovery.
    Recover,
}

/// The move KH2 is to borrow him for (taken by `takeover::shared_frame` this frame), and the one
/// it borrowed him for, to set on KH2's first step (`before_step`), with its game frame.
static WANT: Mutex<Option<(Move, u64)>> = Mutex::new(None);
static APPLY: Mutex<Option<(Move, u64)>> = Mutex::new(None);
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
/// table and our slot of its motion on its request.
fn ours_for(row: &ComboRow) -> Option<usize> {
    // (Attack rows only: the guard's row 11 is started by its request, `guard_press`.)
    super::kh1rows::by_kh2_row(row.id).filter(|&n| super::kh1rows::OURS[n].attack && super::kh1rows::OURS[n].motion == row.motion && super::kh1rows::installed() && super::kh2slots::has(super::kh1rows::request(n) as usize, row.motion))
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
    note(format!("KH2's {} (row {}, motion {}) on KH1's row {r} of its own (request {:#x}){}", o.name, o.kh2_row, o.motion, super::kh1rows::request(n), if o.rises && !air { ", out of KH1's air state" } else { "" }));
    if o.rises && !air {
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
/// - else our next-finisher row of this kind (ground or air), our slot of KH2's motion picked on
///   its request (`kh2slots::pick_own`), when its request isn't the one playing (KH1's finisher's
///   is, 0xCB / 0xCE: a new request, so KH1 starts KH2's motion fresh);
/// - else (a third finisher, ours playing) KH1's own finisher row of this kind, our slot of the
///   pick on its request (KH1's finisher, as `kh2_row` / `select_swing` play KH2's finishers).
/// In the air with KH2's hop for the row. `None` (the borrow) when none of them can play it.
fn next_finisher(actor: usize, row: &ComboRow, launch: Option<character_core::Launch>, air: bool, count: u8, playing: u32, request_of: &dyn Fn(i32) -> Option<u32>) -> Option<Run> {
    use super::kh1rows as k;
    if let Some(n) = ours_for(row).filter(|&n| k::request(n) != playing) {
        return Some(ours_run(n, launch, air, count, request_of));
    }
    if let Some(n) = k::next_finisher(air).filter(|&n| k::installed() && k::request(n) != playing) {
        if unsafe { super::kh2slots::pick_own(k::request(n) as usize, row.motion) } {
            return Some(ours_run(n, launch, air, count, request_of));
        }
    }
    let combo = if air { &AIR_COMBO } else { &GROUND_COMBO };
    let fin = combo.finisher_rows.iter().filter_map(|&r| Some((r, request_of(r)?))).find(|&(_, q)| q == combo.finisher && q != playing && super::kh2slots::has(q as usize, row.motion))?;
    let plays = unsafe { super::kh2slots::pick_ability(fin.1 as usize, Some(row.motion), !super::styles::kh2(combo.style)) };
    if !plays {
        return None;
    }
    let _ = actor;
    if let Ok(mut l) = LAST.lock() {
        *l = Some((count, fin.1, true, air));
    }
    note(format!("KH2's next finisher (row {} motion {}) on KH1's finisher row {} (request {:#x}): ours is playing", row.id, row.motion, fin.0, fin.1));
    Some(match launch.filter(|l| air && l.height.is_finite() && l.velocity.iter().all(|v| v.is_finite())) {
        Some(l) => Run::AirHop(fin.0, l),
        None => Run::Row(fin.0),
    })
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

fn want(m: Move) {
    if let Ok(mut w) = WANT.lock() {
        *w = Some((m, frame()));
    }
}

/// This frame's ability borrow (`takeover::shared_frame`; one left from an earlier frame, KH1's
/// physics step not run since, is dropped: no buffer).
pub fn take_want() -> Option<Move> {
    let (m, at) = WANT.try_lock().ok().and_then(|mut w| w.take())?;
    if frame().saturating_sub(at) > 1 {
        note(format!("KH2's ability move {m:?} dropped: KH1's states didn't let it go that frame"));
        return None;
    }
    Some(m)
}

/// KH2 borrowed him for it: set on its first step.
pub fn lent(m: Move) {
    if let Ok(mut a) = APPLY.lock() {
        *a = Some((m, frame()));
    }
    BLOWN_AT.store(0, Ordering::Relaxed);
    if let Ok(mut l) = LAST.lock() {
        *l = None;
    }
}

/// Right before KH2's step: the move it borrowed him for (a few frames at most after the borrow:
/// KH1 holding him still in between drops it), as the step's press.
pub fn before_step(ch: &mut Character, input: Input) -> Input {
    let Some((m, at)) = APPLY.try_lock().ok().and_then(|mut a| a.take()) else { return input };
    if frame().saturating_sub(at) > 3 {
        note(format!("KH2's ability move {m:?} dropped: its step came too late"));
        return input;
    }
    let release = |b: Button| Button { pressed: false, held: b.held };
    match m {
        Move::Row { id, state } => {
            ch.set_combo_state(state);
            ch.force_row(id);
            Input { attack: Button { pressed: true, held: true }, action: release(input.action), jump: release(input.jump), ..input }
        }
        Move::Recover => {
            if !ch.air_recovery(input.stick) {
                note("Aerial Recovery out of KH1's knock-away: KH2 couldn't (not in the air)".to_string());
            }
            Input { jump: release(input.jump), action: release(input.action), ..input }
        }
    }
}

unsafe fn sora() -> usize {
    rd::<u64>(takeover::game_base() + PARTY_RVA) as usize
}

/// KH1's combo now: its count, its length here (ground or air), and whether its swing connected.
unsafe fn kh1_combo(actor: usize, air: bool) -> Option<(u8, u8, bool)> {
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    let stats = pose::ptr(rd::<u32>(actor + 0x6C));
    if info < 0x10000 || stats < 0x10000 {
        return None;
    }
    Some((rd::<u8>(info + 0x41), rd::<u8>(stats + if air { 0xD5 } else { 0xD4 }), rd::<u32>(info + 0x50) != 0))
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
/// (`hurt`). The rest still borrow.
fn native_ability(row: &ComboRow) -> bool {
    row.ability != 0 && super::kh2slots::has_any(row.motion)
}

/// What `combat::exec` runs for an Attack KH1's own combo took (`attack`).
pub enum Run {
    /// KH2 borrows him for one of its ability moves: no KH1 row.
    Kh2,
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
/// KH1's where he is, KH1's `row` about to run): `Run::Kh2` when KH2 borrows him for one of its
/// ability moves instead, else the KH1 row to run: `row`, or KH1's plain swing or finisher row in
/// its place where KH2's row selection takes that (`kh2_row`), or KH2's hop on KH1's air state
/// (`kh2_hop`). `request_of`: a KH1 row's motion request (its `+0`).
pub unsafe fn attack(actor: usize, row: i32, _dir: usize, request_of: &dyn Fn(i32) -> Option<u32>) -> Run {
    // (On the ground: KH2's line ahead, for its row 1, tested in KH2's trial along the
    // selector's direction, `drop_ahead`.)
    takeover::test_drop_ahead(rd::<i32>(actor + 0x70) == 0 && super::styles::kh2(super::styles::Kind::GroundCombo));
    let run = attack_row(actor, row, request_of);
    takeover::test_drop_ahead(false);
    run
}

unsafe fn attack_row(actor: usize, row: i32, request_of: &dyn Fn(i32) -> Option<u32>) -> Run {
    // (KH1's ground state or its jump / fall: not its flight, 8, nor its other states.)
    let state = rd::<i32>(actor + 0x70);
    if !matches!(state, 0..=2) || super::form::in_form() {
        return Run::Row(row);
    }
    let air = matches!(state, 1 | 2);
    let Some((count, kh1_len, _)) = kh1_combo(actor, air) else { return Run::Row(row) };
    // A Square move of KH2's on its own KH1 row (`control` gave KH1 this Attack for it).
    if let Some((n, launch)) = take_ours() {
        return ours_run(n, launch, air, count, request_of);
    }
    // Counterguard: Attack during KH2's guard's repel (KH2's own rule, Steam KH2's Hurt step on
    // motion 42; `counter_window` opened KH1's combo window for it), on its own KH1 row.
    if let Some(run) = counterguard(actor, air, count, request_of) {
        return run;
    }
    let last = LAST.try_lock().ok().and_then(|l| *l);
    // Finishing Plus: a press during KH1's finisher (its motion still playing) is KH2's next one.
    let playing = rd::<u32>(actor + 0x164) & 0xFFFF;
    if let Some((n, _, true, _)) = last.filter(|l| l.3 == air && l.1 & 0xFFFF == playing) {
        let finishers = FINISHERS.load(Ordering::Relaxed).max(1);
        let from = ComboState { count: n, finishers, hit: true, finisher_next: true };
        if let Some((row, _, _, launch)) = takeover::kh2_would_take_launch(actor, Some(from), super::kh2slots::playing_motion(actor), air, false).filter(|(r, ..)| r.is_finisher()) {
            note(format!("Attack in KH1's finisher (count {n}, finisher {finishers}): KH2's next finisher, row {} motion {} (Finishing Plus)", row.id, row.motion));
            if let Some(run) = next_finisher(actor, &row, launch, air, n, playing, request_of) {
                FINISHERS.store(finishers.saturating_add(1), Ordering::Relaxed);
                return run;
            }
            want(Move::Row { id: row.id, state: from });
            return Run::Kh2;
        }
    }
    // KH2's combo at KH1's count (this press counted: `count` is this swing's).
    let from = ComboState { count: count.saturating_sub(1), finishers: 1, hit: true, finisher_next: false };
    // (KH2's motion KH1 plays now for its swing: KH2's anti-repeat passes over it, as over its
    // own swing playing.)
    let taken = takeover::kh2_would_take_launch(actor, Some(from), super::kh2slots::playing_motion(actor), air, false);
    let launch = taken.as_ref().and_then(|t| t.3);
    let pick = taken.map(|(r, len, neg, _)| (r, len, neg));
    // (Where KH1's own plain rows play KH2's pick, at the combo style KH2, a pick past KH1's
    // length (Combo Plus) or a finisher before it (Negative Combo) is KH2's row selection on
    // KH1's rows, `kh2_row`, not a borrow.)
    let combo = if air { &AIR_COMBO } else { &GROUND_COMBO };
    // (At the combo style KH1 KH1's own count rules: KH2's pick counts only when it is one of its
    // ability moves of the kind KH1's own count is at, a swing before KH1's length, a finisher at
    // it. KH2's plain swings, and its length's say (Combo Plus past KH1's, Negative Combo before
    // it), are nothing there: KH1's row and motion as unmodded, never a borrow.)
    let pick = if super::styles::kh2(combo.style) {
        pick
    } else {
        let kept = pick.clone().filter(|(r, ..)| r.ability != 0 && r.is_finisher() == (count >= kh1_len));
        if let (Some((r, ..)), None) = (pick.as_ref(), kept.as_ref()) {
            note(format!("KH1's {} combo at the KH1 setting, swing {count} of {kh1_len}: KH2's row {} motion {} (ability {}, finisher {}) not taken, KH1's own row", if air { "aerial" } else { "ground" }, r.id, r.motion, r.ability, r.is_finisher()));
        }
        kept
    };
    let why = pick.as_ref().filter(|(r, ..)| !combo.serves(r)).and_then(|(r, len, neg)| ability_move(r, *len, *neg, count, kh1_len));
    note(format!(
        "KH1's {} combo, swing {count} of {kh1_len}: KH2 would take {}{}",
        if air { "aerial" } else { "ground" },
        pick.as_ref().map_or("nothing".to_string(), |(r, len, _)| format!("row {} motion {} (ability {}, finisher {}, KH2's length {len})", r.id, r.motion, r.ability, r.is_finisher())),
        why.map_or(String::new(), |w| format!(": {w}, KH2 borrows him for it"))
    ));
    if let Some((r, ..)) = pick.as_ref().filter(|_| why.is_some()) {
        // (One with a KH1 row of its own, `kh1rows`: Guard Break.)
        if let Some(n) = ours_for(r) {
            return ours_run(n, launch, air, count, request_of);
        }
        want(Move::Row { id: r.id, state: from });
        return Run::Kh2;
    }
    if !air {
        if let Some((r, l)) = pick.as_ref().zip(launch).and_then(|((p, ..), l)| kh2_hop(actor, row, p, l, request_of)) {
            if let Ok(mut g) = LAST.lock() {
                *g = request_of(r).map(|m| (1, m, false, true));
            }
            return Run::Hop(r, l);
        }
    }
    let row = kh2_row(actor, combo, row, pick.as_ref().map(|(r, ..)| r), request_of);
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
    note(format!(
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
            note(format!("KH2's line ahead: the floor {below:.0} below at {length:.0} ahead (more than {reach:.0}: none), KH2's row 1 marked"));
            true
        }
        Some(_) => {
            note(format!("KH2's line ahead: no floor under the point {length:.0} ahead, KH2's row 1 marked"));
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
        note("KH2's floor sweep: KH1's collision calls (0x14029A0B0 / 0x140195180 / 0x1401907F0) aren't the expected code here; not tested".to_string());
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
    use super::styles::{kh2, Kind};
    // (An ability's hop is KH2's at either style, `native_ability`; KH2's plain hops are KH2's
    // row selection, with both combo styles at KH2.)
    let ability = pick.ability != 0;
    if pick.is_air() || !pick.is_jump() || (ability && !native_ability(pick)) || (!ability && !kh2(Kind::GroundCombo)) {
        return None;
    }
    if !ability && !kh2(Kind::AirCombo) {
        note(format!("KH2's row selection: KH2's hop, row {} motion {}, not played: the Aerial Combo style is KH1's (KH1's row {row} kept)", pick.id, pick.motion));
        return None;
    }
    let playing = rd::<u32>(actor + 0x164) & 0xFFFF;
    let Some((r, q)) = AIR_COMBO.swing_rows.iter().filter_map(|&r| Some((r, request_of(r)?))).find(|&(_, q)| AIR_COMBO.swings.contains(&q) && q != playing && super::kh2slots::has(q as usize, pick.motion)) else {
        note(format!("KH2's row selection: KH2's hop, row {} motion {}, not played: no air swing row of KH1's free with a slot for it (KH1's row {row} kept)", pick.id, pick.motion));
        return None;
    };
    super::kh2slots::pick_ability(q as usize, Some(pick.motion), ability && !kh2(Kind::AirCombo));
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    if info >= 0x10000 {
        wr(info + 0x41, 1u8);
    }
    note(format!(
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
        super::styles::kh2(self.style) || (row.ability != 0 && native_ability(row))
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
        note(format!("KH2's row selection: KH1's row {row} (request {}) kept: no plain row of KH1's free for KH2's row {} motion {}", own.map_or("none".to_string(), |q| format!("{q:#x}")), p.id, p.motion));
        return row;
    };
    note(format!("KH2's row selection: KH1's row {row} (request {}) replaced by its row {r} (request {q:#x}) for KH2's row {} motion {}", own.map_or("none".to_string(), |q| format!("{q:#x}")), p.id, p.motion));
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
    let kh2 = super::styles::kh2(combo.style);
    let fits = pick.filter(|r| combo.plain(r) && combo.here(r) && combo.fits(q, r));
    // (At KH1's style only an ability's move plays ours there, KH1's request pointed at it for
    // that swing, `kh2slots::pick_ability`.)
    let plays = unsafe { super::kh2slots::pick_ability(req, fits.map(|r| r.motion), !kh2) };
    if !kh2 && !plays {
        return false;
    }
    note(format!(
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
            note("jump while KH1 has him knocked away: KH2's Aerial Recovery".to_string());
            want(Move::Recover);
            return Some(all);
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
                        note(format!("Square while KH1 has him knocked away: KH2's Retaliating Slash (row {} motion {}) on KH1's row {} of its own, {}", r.id, r.motion, super::kh1rows::row(n), launch.map_or("no hop".to_string(), |l| format!("{:.0} high, {:.1}/frame", l.height, l.velocity[0].hypot(l.velocity[1])))));
                        return Some(all);
                    }
                }
                note("Square while KH1 has him knocked away: KH2's Retaliating Slash".to_string());
                want(Move::Row { id, state: ComboState { count: 0, finishers: 1, hit: false, finisher_next: false } });
                return Some(all);
            }
        }
        return None;
    }
    // Square in KH1's combo window (a swing of its combo, slots 98..104): KH2's Square move there,
    // its count KH1's.
    // (Not in KH2's guard repel: KH1's combo window there is Counterguard's, `counter_window`.)
    if square_fresh && s.combo_window && matches!(pose::sora_slot(), Some(98..=104)) && unsafe { super::kh2slots::playing_motion(actor) } != Some(character_core::motion::GUARD) {
        let (count, _, hit) = unsafe { kh1_combo(actor, air) }?;
        let master = takeover::with_abilities(|a| a.combo_master) == Some(true);
        if count == 0 || !(hit || master) {
            return None;
        }
        let from = ComboState { count, finishers: 1, hit: true, finisher_next: false };
        let (row, _, _, launch) = unsafe { takeover::kh2_would_take_launch(actor, Some(from), None, air, true) }?;
        // One with a KH1 row of its own (`kh1rows`): KH1's Attack command for it, as KH1's
        // menu gives it, so KH1's control step runs its Attack step now (its count, its target,
        // its press in the combo window), whose row is ours (`attack`, `take_ours`).
        if let Some(n) = ours_for(&row) {
            if let Ok(mut g) = OURS.lock() {
                *g = Some((n, launch, frame()));
            }
            unsafe { super::kh1rows::command_attack() };
            note(format!("Square in KH1's combo window (swing {count}): KH2's Square move, row {} motion {}, on KH1's row {} of its own", row.id, row.motion, super::kh1rows::row(n)));
            return Some(super::shared::KH1_ACTION_BIT);
        }
        note(format!("Square in KH1's combo window (swing {count}): KH2's Square move, row {} motion {}", row.id, row.motion));
        want(Move::Row { id: row.id, state: from });
        return Some(super::shared::KH1_ACTION_BIT);
    }
    None
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
/// `kh2slots::kh1_tracks`). False (the borrow then) without our request built or out of KH1's
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
    note(format!("jump while KH1 has him knocked away: KH2's Aerial Recovery on KH1's request {request:#x} of its own, {:.0} high, {:.1}/frame", character_core::AIR_RECOVERY_HOP, vel[0].hypot(vel[1])));
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
    // (Held: let go early, the rise is cut as KH2's held jump is, `kh2_hop_held`.)
    super::shared::physics::kh2_hop_held(actor, height, vel);
    super::shared::mark_air_dodged();
    takeover::note_launch(vel);
    note(format!("jump in the air: KH2's Aerial Dodge on KH1's request {request:#x} of its own (KH1 state {state}), {height:.0} high, {:.1}/frame", vel[0].hypot(vel[1])));
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
    if actor < 0x10000 || rd::<i32>(actor + 0x70) != 2 || super::kh2slots::playing_motion(actor) != Some(AERIAL_DODGE) {
        return;
    }
    let (t, up) = (rd::<f32>(actor + 0x3B0), rd::<f32>(actor + 0x3B8));
    if !(t.is_finite() && up.is_finite()) || t + DODGE_FALL_MARGIN < up || rd::<i32>(actor + 0x178) != 0 {
        return;
    }
    dodge_fall(actor, &format!("its apex, {t:.1} of {up:.1} frames up"));
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
    note(format!("KH2's Aerial Dodge: {why}, KH1's fall (request 6, blend {DODGE_FALL_BLEND})"));
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
            note(format!("Attack in KH2's finisher with the combo style KH1's: KH2's next finisher, row {} motion {} (Finishing Plus)", r.id, r.motion));
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
    note(format!(
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
    note(format!("KH1's combo goes on from KH2's: count now {}", rd::<u8>(s.info + 0x41)));
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
        note(format!("Square standing: KH2's guard (motion 173) on KH1's request {:#x} of its own{}", super::kh1rows::by_motion(character_core::motion::GUARD_PRESS).map_or(0, super::kh1rows::request), if face.is_some() { ", facing KH1's lock" } else { "" }));
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
        note(format!("Square while moving: KH2's dodge roll LV{level} on KH1's roll (request 0xDC, KH2 motion {DODGE_ROLL}), invulnerable {star:.0} frames"));
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
        note("KH2's guard repel: not started (not on the ground, or no request of ours for motion 42)".to_string());
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
    note(format!("KH2's guard stopped a hit: its repel (motion 42), pushed back {REPEL_BACK:.0}{}", if super::recoil::kh2() { ", 80 frames without damage (Recoil KH2)" } else { "" }));
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
        note("KH1's block: not run (not the Steam build, not on the ground, or a Drive Form)".to_string());
        return;
    }
    let base = takeover::game_base();
    if rd::<[u8; 15]>(base + CLASH_RECOIL_STEAM) != CLASH_RECOIL_BYTES || rd::<[u8; 10]>(base + BIG_RECOIL_STEAM) != BIG_RECOIL_BYTES {
        note("KH1's block: KH1's clash recoils (0x14029D720 / 0x14029D5F0) aren't the expected code here; not run".to_string());
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
    note(format!(
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
            note("KH2's guard repel: the command menu's Attack shows KH1's Counterattack (KH2's Counterguard)".to_string());
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
        note("Attack in KH2's guard repel: KH2 wouldn't take Counterguard (row 39) now".to_string());
        return None;
    };
    if let Ok(mut p) = PUSH.lock() {
        *p = None;
    }
    note(format!("Attack in KH2's guard repel: KH2's Counterguard (row {} motion {})", row.id, row.motion));
    Some(ours_run(n, launch, false, count, request_of))
}
