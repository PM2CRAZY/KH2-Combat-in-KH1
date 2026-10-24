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
//!   - a row that needs an ability (an action ability's move: Flash Step, Vicinity Break, Aerial
//!     Spiral; the finishers Explosion, Guard Break, Aerial Finish, Magnet Burst ...), opener,
//!     step or finisher, except the ones KH1's own rows now play natively (`native_ability`:
//!     Slapshot, Dodge Slash, Slide Dash, Aerial Sweep, Aerial Dive at Base Combo KH2);
//!   - KH2's finisher where KH1's would still swing, with Negative Combo;
//!   - KH2's step where KH1's would finish, KH2's combo being the longer (Combo Plus KH1 didn't
//!     count);
//!   - Finishing Plus: a press during KH1's finisher is KH2's next finisher.
//! - **Square** in KH1's combo window (`control`): KH2's Square move there (Finishing Leap, Upper
//!   Slash, Horizontal Slash), if KH1's swing connected or Combo Master is on (as KH2's).
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
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
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
/// only a Keyblade swing (`kh2slots::swing_rows`, which builds its motion's slots only then), with
/// the Base Combo at KH2 (where our slots play). A swing (Slapshot, Dodge Slash, Slide Dash): KH1's
/// plain ground swing row runs it as it runs KH2's plain swings (`kh2_row`, `select_swing`): KH1's
/// row, attack object, damage and windows, KH2's motion and root travel. A hop (Aerial Sweep,
/// Aerial Dive), with the Aerial Combo at KH2 too: KH1's air state and air swing row with KH2's
/// hop, as KH2's plain hop (`kh2_hop`). The rest still borrow.
fn native_ability(row: &ComboRow) -> bool {
    use super::styles::{kh2, Kind};
    row.ability != 0 && !row.is_air() && !row.is_finisher() && kh2(Kind::GroundCombo) && (!row.is_jump() || kh2(Kind::AirCombo)) && super::kh2slots::has_any(row.motion)
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
    let last = LAST.try_lock().ok().and_then(|l| *l);
    // Finishing Plus: a press during KH1's finisher (its motion still playing) is KH2's next one.
    let playing = rd::<u32>(actor + 0x164) & 0xFFFF;
    if let Some((n, _, true, _)) = last.filter(|l| l.3 == air && l.1 & 0xFFFF == playing) {
        let from = ComboState { count: n, finishers: 1, hit: true, finisher_next: true };
        if let Some((row, ..)) = takeover::kh2_would_take(actor, Some(from), super::kh2slots::playing_motion(actor), air, false).filter(|(r, ..)| r.is_finisher()) {
            note(format!("Attack in KH1's finisher (count {n}): KH2's next finisher, row {} motion {} (Finishing Plus)", row.id, row.motion));
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
    let why = pick.as_ref().and_then(|(r, len, neg)| ability_move(r, *len, *neg, count, kh1_len));
    note(format!(
        "KH1's {} combo, swing {count} of {kh1_len}: KH2 would take {}{}",
        if air { "aerial" } else { "ground" },
        pick.as_ref().map_or("nothing".to_string(), |(r, len, _)| format!("row {} motion {} (ability {}, finisher {}, KH2's length {len})", r.id, r.motion, r.ability, r.is_finisher())),
        why.map_or(String::new(), |w| format!(": {w}, KH2 borrows him for it"))
    ));
    if let Some((r, ..)) = pick.as_ref().filter(|_| why.is_some()) {
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
    let combo = if air { &AIR_COMBO } else { &GROUND_COMBO };
    let row = kh2_row(actor, combo, row, pick.as_ref().map(|(r, ..)| r), request_of);
    let plays = select_swing(combo, request_of(row), pick.as_ref().map(|(r, ..)| r));
    if let Ok(mut l) = LAST.lock() {
        *l = request_of(row).map(|m| (count, m, count >= kh1_len, air));
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
/// finishers 183 / 184; no ability) and KH1's air row plays its motion (`select_swing`), so the
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
    if Build::current() != Build::Steam {
        return false;
    }
    let base = takeover::game_base();
    if rd::<[u8; 10]>(base + SETUP_STEAM) != SETUP_BYTES || rd::<[u8; 10]>(base + MOVER_STEAM) != MOVER_BYTES || rd::<[u8; 11]>(base + NORMAL_STEAM) != NORMAL_BYTES {
        note("KH2's line ahead: KH1's collision calls (0x14029A0B0 / 0x140195180 / 0x1401907F0) aren't the expected code here; not tested".to_string());
        return false;
    }
    let Some(kh2) = pose::kh2() else { return false };
    let Some(shape) = kh2_data::sora::bg_shape(&kh2.rig.body_shapes) else { return false };
    let length = kh2.character.attack.jump_front + shape.radius;
    // (KH2's mover: the floor more than this below his feet is none.)
    let reach = shape.lift - shape.height - 1.0;
    let flat = aim[0].hypot(aim[1]);
    if !(flat > 1e-3) || !(length > 0.0) || !reach.is_finite() {
        return false;
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
        return false;
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
    let drop = !found || !(below <= reach);
    if drop {
        note(if found {
            format!("KH2's line ahead: the floor {below:.0} below at {length:.0} ahead (more than {reach:.0}: none), KH2's row 1 marked")
        } else {
            format!("KH2's line ahead: no floor under the point {length:.0} ahead, KH2's row 1 marked")
        });
    }
    drop
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
    if !kh2(Kind::GroundCombo) || pick.is_air() || !pick.is_jump() || (pick.ability != 0 && !native_ability(pick)) {
        return None;
    }
    if !kh2(Kind::AirCombo) {
        note(format!("KH2's row selection: KH2's hop, row {} motion {}, not played: the Aerial Combo style is KH1's (KH1's row {row} kept)", pick.id, pick.motion));
        return None;
    }
    let playing = rd::<u32>(actor + 0x164) & 0xFFFF;
    let Some((r, q)) = AIR_COMBO.swing_rows.iter().filter_map(|&r| Some((r, request_of(r)?))).find(|&(_, q)| AIR_COMBO.swings.contains(&q) && q != playing && super::kh2slots::has(q as usize, pick.motion)) else {
        note(format!("KH2's row selection: KH2's hop, row {} motion {}, not played: no air swing row of KH1's free with a slot for it (KH1's row {row} kept)", pick.id, pick.motion));
        return None;
    };
    super::kh2slots::pick(q as usize, Some(pick.motion));
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
    /// an ability swing KH1's swing row plays natively (`native_ability`, on the ground); on the
    /// ground no hop (KH2's hops aren't KH1 rows), in the air an air row.
    fn plain(&self, row: &ComboRow) -> bool {
        let air = self.style == super::styles::Kind::AirCombo;
        let here = if air { row.is_air() } else { !row.is_air() && !row.is_jump() };
        here && (row.ability == 0 || (!air && native_ability(row)))
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
    if !super::styles::kh2(combo.style) {
        return row;
    }
    let Some(p) = pick.filter(|r| combo.plain(r)) else { return row };
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
    if !super::styles::kh2(combo.style) {
        unsafe { super::kh2slots::pick(req, None) };
        return false;
    }
    let fits = pick.filter(|r| combo.plain(r) && combo.fits(q, r));
    let plays = unsafe { super::kh2slots::pick(req, fits.map(|r| r.motion)) };
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
    // (Square).
    if s.launched && !s.can_act {
        if jump_fresh && takeover::with_abilities(|a| a.aerial_recovery) == Some(true) {
            note("jump while KH1 has him knocked away: KH2's Aerial Recovery".to_string());
            want(Move::Recover);
            return Some(all);
        }
        if square_fresh {
            let row = takeover::kh2_retaliating_slash();
            if let Some(id) = row {
                note("Square while KH1 has him knocked away: KH2's Retaliating Slash".to_string());
                want(Move::Row { id, state: ComboState { count: 0, finishers: 1, hit: false, finisher_next: false } });
                return Some(all);
            }
        }
        return None;
    }
    // Square in KH1's combo window (a swing of its combo, slots 98..104): KH2's Square move there,
    // its count KH1's.
    if square_fresh && s.combo_window && matches!(pose::sora_slot(), Some(98..=104)) {
        let (count, _, hit) = unsafe { kh1_combo(actor, air) }?;
        let master = takeover::with_abilities(|a| a.combo_master) == Some(true);
        if count == 0 || !(hit || master) {
            return None;
        }
        let from = ComboState { count, finishers: 1, hit: true, finisher_next: false };
        let (row, ..) = unsafe { takeover::kh2_would_take(actor, Some(from), None, air, true) }?;
        note(format!("Square in KH1's combo window (swing {count}): KH2's Square move, row {} motion {}", row.id, row.motion));
        want(Move::Row { id: row.id, state: from });
        return Some(super::shared::KH1_ACTION_BIT);
    }
    None
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
