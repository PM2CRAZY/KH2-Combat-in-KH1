//! Valor's left Keyblade's hits: its own KH1 attack object, built and collided by KH1 exactly as
//! its own Keyblade object is (`docs/kh1-drive-forms-native.md` §7.2).
//!
//! KH2 keeps one attack per (group, shape-source object) (Steam KH2 `0x14040D810` ->
//! `0x1403DAFE0` -> `0x1403D29B0` -> `0x1403D25D0`, a new attack `0x14040F1C0`; Checked (decomp)),
//! so the two Keyblades always run separate attacks. The left Keyblade's hit windows (weapon 2)
//! play on a KH1 attack group of their own (`kh2slots::left_channel`); for each play of a Valor
//! slot with such windows, `new_play` makes one KH1 attack object of Sora's with KH1's constructor
//! `0x1402B2DC0`, and `step` re-arms it at each new KH2 attack (`rehit::new_attack`) and puts the
//! open window's data on it. Which window is open, its ATKP (damage, reaction, re-hit) and its
//! timing are KH2's.
//!
//! **Its shape is KH1's Keyblade's** (the tester, what-is-still-missing after m13: "it should fit
//! kingdom hearts 1 in this situation for both keyblades", "the keyblade should behave exactly like
//! kh1's keyblade so make sure it scales by keyblade"). KH1 builds its Keyblade object (Steam KH1,
//! Checked (decomp)):
//! - its data (Sora's swing row's): bone `+0x10` = `0x30` (the blade), blade range `+0x54` /
//!   `+0x58` (floats), radius `+0x5C` (a float: `mulss` / `movss` at `0x1402B2FD5..`, Checked (exe));
//! - the constructor `0x1402B2DC0`: position kind 1, bone `+0x20`, range `+0x24` / `+0x28` from
//!   the data, radius `+0xC` = the data's (times the actor's scale `+0x40` with `+0x374` bit
//!   `0x2000`); the bone-source actor `+0x90` and the owner `+0x34` both the maker;
//! - the row maker `0x1402B3270`, for bone `0x30`, with the actor's weapon row (`[+0x14C]+0x20`):
//!   radius += scale × row `+0x20`, range × row `+0x2C`, priority + row `+0x33` (1..99), critical
//!   × row `+0x31` × 0.05, critical bonus + row `+0x32`;
//! - each pass `0x1402B3800` takes both ends with the bone point `0x14029EB20 ([+0x90], 0x30, t)`,
//!   which maps t through the bone-source actor's weapon row (`row+0x28 + (row+0x24 − row+0x28) ×
//!   t`) and lerps along the bone that 0x30 names (`0x1401D6320`, from its parent's drawn matrix),
//!   and `0x1402B3D50` lays spheres of that radius from one end to the other, (r + r) / 2 apart,
//!   each tested with `0x1402B3BE0`; with `+0` bit 16 its own sweep back to the last place.
//!
//! So the left object is made as that, with the **left Keyblade's own row** (bonuses, blade range)
//! and with its bone-source actor `+0x90` set to `leftblade`'s owner copy (Sora's actor and
//! matrices moved onto the left Keyblade, its weapon object holding the left Keyblade's row): KH1's
//! own builder then lays KH1's one Keyblade sphere row along the left blade, scaled by that
//! Keyblade, and KH1's own sweep, wall test, contact and clash run as for the right one. No shapes
//! of ours (before: KH2's weapon-model group spheres, our sweep).
//!
//! `bodyhits` owns the pass's entry hook (`0x1402B3800`, where `before_build` brings `+0x90` up to
//! date), the pool helpers and our data copies; `takeover` calls `step`.
//!
//! (t1037) **The right Keyblade where no KH1 attack row runs.** KH1 makes its own Keyblade attack
//! object only from an attack row (its swings, our combo and ability rows), so in a slot of ours
//! that runs none (a jump, fall or landing, an item's use, a glide: `kh2slots::rowless_now`) the
//! right Keyblade's windows (weapon 1) had no object. Final Form has them (`W_EX010_ULTF_R.mset`,
//! Checked (files)): jump 3 ATKP 1273 at 2..12 then 1286 from 12 to the end, fall 4 1275 at 2..20,
//! landing 5 1277 at 0..20, item use 9 1284 at 1..38 then 1449 at 38..46, glide 205 / 206 / 207
//! 1281 all through (the left one's the same at the same frames: 1274 / 1287, 1276, 1278, 1285 /
//! 1450, 1282). There a second object of the same kind is made for the right Keyblade (`RIGHT`):
//! the same constructor and template data, on KH1's attack group 0 (the Keyblade's, whose HIT
//! track every slot of ours already has from the clip's weapon-1 windows, `kh2slots::kh1_tracks`),
//! with Sora's own weapon row as KH1's row maker puts it on his own Keyblade object and Sora as its
//! bone-source actor (the constructor's own), re-armed at each new KH2 attack as the left one. So
//! each blade's hit is made with its own Keyblade's row (blade range, radius, critical, priority),
//! as Valor's two blades are; the Strength both hit with is Sora's live one, his Keyblade's plus
//! the second one's (`valorstats::bonus`), as in Valor. **Assumption** (open): how KH2 splits the
//! two Keyblades' strength between the two blades' hits is not traced.
//! (t1037) A play of the same motion on the same request (KH1 looping it: the fall from frame 12,
//! the glide) keeps both objects, so `step`'s rule decides whether it is a new KH2 attack: a window
//! open all through the loop (the glide's) stays one attack, re-hit by its interval (`rehit`), as
//! KH2's range handler keeps asserting the same ATKP; one that closed and opens again is a new one.
//!
//! (t1043) **Final's Keyblade guards** (the owner's answer "(a)": KH1's own guard object on each
//! Keyblade). Final's ATKP 162 windows (kind 2 guard, power 0, refact self 2, reflect range 180,
//! angle 0; Checked (files) `00battle.bin`) are on both Keyblades (`W_EX010_ULTF_R` / `_L.mset`,
//! group 1, Checked (files)): 151 55..84 right / 55..89 left, 153 4..125, 183 4..165, the glide
//! hover 208 from 1 to the end. KH1 makes its guard with its row maker `0x1402B3270` on its guard
//! row 0x3D (Sora's table, `combomods::kh1_guard`), which is KH1's attack constructor
//! `0x1402B2DC0` and the Keyblade row's part (Checked (decomp)): the object of KH1's guard data
//! (bone 0x30, the blade; clash type 2, which KH1's pass never meets with a body, `0x1402B3BE0`;
//! no power), tied to his motion (activity 2) and to its data's attack group. So for each blade
//! with such windows in a Final slot, `new_play` makes that object with the same constructor (first
//! bytes checked, `bodyhits::make_ok`) on a copy of KH1's guard row's data, the copy's group the
//! blade's guard group (`kh2slots::guard_channels`), whose HIT track our slot carries over KH2's
//! window frames (`kh2slots::guard_tracks`): KH1's own pass switches it on and off at those frames
//! and frees it when the request ends; nothing of ours runs a frame for it. Its blade fields
//! (radius, blade range) as the row maker gives them from that Keyblade's row (`blade_fields`), the
//! left one's bone-source actor `leftblade`'s owner copy (`before_build`), as the left hit object.
//! KH2's 162 lets the move go on (refact self 2): the object's priority is 100, so KH1's clash
//! (`0x1402B4BC0`) returns at once for it, puts no clash record on Sora (no KH1 recoil, the motion
//! goes on) and gives the attack meeting it KH1's clash (its owner's recoil, a shot turned back),
//! as Guard Shape KH1's guard objects (`combat::Strikes::blade_guard`); KH1's guard request 0xD4
//! is never started. Its arc KH2's: 162's reflect range 180 is every side by KH2's test
//! (`character_core::reaction::clash_angle_ok`), KH1's data `+0x64` -1.
//!
//! (t1046) **Crits only on each blade's last blow, in a finisher** (the owner's answers 1 (b), 3 (b)
//! and (c)). In a move playing on a finisher request (`kh1rows::finisher_request`), each blade's
//! last blow window in the clip (`kh1rows::last_blow`: Final's 153 and 154 on both blades, 183's
//! 1421 / 1420) takes KH1's finisher row's data; every other window, and every window of a swing,
//! jump, item use, glide or cast (Final's 181, 1449 / 1450, 1286 / 1287, 1281 / 1282, 1429 / 1428),
//! the swing row's (a blow keeping kind 5): those never roll. KH1's constructor sets the object's critical from its data once
//! (`0x1402B2DC0`: `+0x94` = data `+0x18` × 0.05 × stats `+0xD8`, `+0x98` = data `+0x1C`; the row
//! maker `0x1402B3270` then × row `+0x31` × 0.05 and + row `+0x32` on a blade, Checked (decomp)), so
//! `step` puts each window's critical on the object with its data as those two would
//! (`crit_fields`), beside its blade fields. A re-hit of a window on the same target rolls no more
//! (`rehit::again`, `kh1rows::may_crit`). Before, the object kept its first window's: Final's 153
//! left blade (1403 first, a blow) rolled on every hit, its right one (a row of ours) too.

use super::addr::Rva;
use super::bodyhits::{rd, wr};
use super::{log, pose};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

/// KH1's Keyblade bone id (data `+0x10`, object `+0x20`): the blade (`0x14029EB20`).
const BLADE_BONE: u32 = 0x30;
/// KH1's attack constructor (Steam), its free hit bit, its object free (Epic RVAs, mapped).
const MAKE_STEAM: usize = 0x2B2DC0;
const FREE_BIT_RVA: Rva = Rva(0x2B15D0);
const FREE_ATTACK_RVA: Rva = Rva(0x2B13D0);
/// An attack data entry's size; our copies' key mark (beside `bodyhits`' body windows' keys).
const DATA: usize = 0x70;
const KEY_MARK: u64 = 0xF0;

/// Our two objects: the left Keyblade's (weapon 2, in every slot of a two-bladed native form) and
/// (t1037) the right one's where no KH1 attack row makes KH1's own (weapon 1, `kh2slots::rowless_now`).
const LEFT: usize = 0;
const RIGHT: usize = 1;
const SIDES: [usize; 2] = [LEFT, RIGHT];

/// Each live object's address (0: none), for the per-object checks without the lock.
static OBJ: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
static STATE: [Mutex<Option<Left>>; 2] = [Mutex::new(None), Mutex::new(None)];
static LOGGED: AtomicU32 = AtomicU32::new(0);

/// (t1043) Final's Keyblade guard objects (`LEFT` / `RIGHT`): each live one's address (0: none).
static GUARD_OBJ: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
static GUARD: [Mutex<Option<BladeGuard>>; 2] = [Mutex::new(None), Mutex::new(None)];
/// KH1's guard row in Sora's table (`combat`'s, `combomods::kh1_guard`: request 0xD4, data 0x4DA8).
const KH1_GUARD_ROW: usize = 0x3D;
/// A KH1 guard's priority (`combat::GUARD_PRIORITY`): KH1's clash returns at once for it, no
/// clash record on its owner (`0x1402B4BC0`, Checked (decomp)).
const GUARD_PRIORITY: i32 = 100;
/// Frames between KH1's re-arms of a guard object (`combat::GUARD_REARM`): an attack met is met
/// again after that.
const GUARD_REARM: f32 = 10.0;
/// Our guard data copies' key mark (beside `KEY_MARK`).
const GUARD_MARK: u64 = 0xF1;

/// (t1043) One of Final's Keyblade guard objects: KH1's guard object on that blade for a play.
struct BladeGuard {
    obj: usize,
    data: usize,
    motion: u16,
    req: u32,
}

fn note(s: impl FnOnce() -> String) {
    if super::log_on(super::LogCat::Combat) && LOGGED.fetch_add(1, Ordering::Relaxed) < 80 {
        log!(Combat, "{}", s());
    }
}

/// Valor's left Keyblade's KH1 attack object for the motion playing: one object for all its hit
/// windows (weapon 2), tied to their KH1 attack group (`kh2slots::left_channel`), re-armed at each
/// new KH2 attack (`step`), as KH1's own Keyblade object is for the right one. (t1037) Or the
/// right Keyblade's (weapon 1, group 0) in a slot that runs no KH1 attack row (`RIGHT`).
struct Left {
    obj: usize,
    /// Its data now (the open window's).
    data: usize,
    motion: u16,
    /// The request it is tied to (`+0x58`).
    req: u32,
    windows: Vec<LeftWin>,
    /// The window open at the last step, and whether one was.
    cur: Option<usize>,
    was_open: bool,
    /// The left Keyblade's weapon row (`btltbl.bin`'s, `leftblade`'s owner copy's; the right one's:
    /// Sora's own), 0 for none.
    row: usize,
    /// Sora (the maker: the scale the radius takes).
    actor: usize,
}

/// One left-Keyblade hit window: its ATKP, its weapon-model collision group (KH2's attack key with
/// the weapon object), KH1 time span, our attack data for it.
#[derive(Clone, Copy)]
struct LeftWin {
    attack: u16,
    group: u8,
    start: u32,
    end: Option<u32>,
    data: usize,
    /// (t1053) The blade's last blow in a finisher (`blade_data`'s `finisher`): a KH2 blow here
    /// stays KH1's knock-back or launch; any other blow goes to KH1's flinch (`combat::scale_record`).
    last: bool,
}

impl LeftWin {
    fn contains(&self, f: u32) -> bool {
        f >= self.start && self.end.map_or(true, |e| f < e)
    }
}

/// Our object is still the one we made (in use, our data).
unsafe fn still_ours(l: &Left) -> bool {
    rd::<u32>(l.obj) & 0x800 != 0 && pose::ptr(rd::<u32>(l.obj + 0x5C)) == l.data
}

/// The bone-source actor's handle for our object: `leftblade`'s owner copy (filled now) while the
/// left Keyblade is drawn, else its owner's, Sora's (his right Keyblade: logged), as KH1's
/// constructor set it.
unsafe fn bone_source(obj: usize) -> u32 {
    if let Some(h) = super::leftblade::left_owner_handle() {
        return h;
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if super::log_on(super::LogCat::Combat) && N.fetch_add(1, Ordering::Relaxed) < 3 {
        log!(Combat, "left Keyblade hits: the left Keyblade isn't drawn: its hits on the right Keyblade");
    }
    rd::<u32>(obj + 0x34)
}

/// A new play of motion `m` in our slot (`bodyhits::frame`): the last play's object freed, and
/// in Valor (on KH1's own states, two Keyblades) a new one for this play's left windows.
/// (t1037) And the right Keyblade's in a Drive Form on KH1's own states, in a slot that runs no
/// KH1 attack row (`kh2slots::rowless_now`). The same motion again on the same request (KH1
/// looping it) keeps the objects KH1 hasn't freed (`step` then tells a new KH2 attack).
pub unsafe fn new_play(actor: usize, m: u16, req: u32, air: bool, clip: &character_core::Clip) {
    for side in SIDES {
        let Ok(mut g) = STATE[side].try_lock() else { continue };
        if g.as_ref().is_some_and(|l| l.motion == m && l.req == req && still_ours(l)) {
            continue;
        }
        if let Some(l) = g.take() {
            if still_ours(&l) {
                let free: unsafe extern "system" fn(usize) = std::mem::transmute(super::bodyhits::base() + FREE_ATTACK_RVA);
                free(l.obj);
                super::rehit::forget(l.obj);
            }
        }
        OBJ[side].store(0, Ordering::Relaxed);
        let want = super::form::native_now() && if side == LEFT { super::form::two_blades() } else { super::kh2slots::rowless_now(actor) };
        if want {
            *g = make(side, actor, m, req, air, clip);
            OBJ[side].store(g.as_ref().map_or(0, |l| l.obj), Ordering::Relaxed);
        }
    }
    guards_new_play(actor, m, req, clip);
}

/// (t1043) A new play of motion `m`: the last play's guard objects freed, and in Final on KH1's own
/// states one KH1 guard object for each blade with a guard window in the clip (`make_guard`). The
/// same motion again on the same request keeps them, as `new_play` keeps the hit objects.
unsafe fn guards_new_play(actor: usize, m: u16, req: u32, clip: &character_core::Clip) {
    let on = super::form::native_now() && super::form::in_final();
    let ch = if on { super::kh2slots::guard_channels(clip) } else { [None, None] };
    for side in SIDES {
        let Ok(mut g) = GUARD[side].try_lock() else { continue };
        if g.as_ref().is_some_and(|x| x.motion == m && x.req == req && guard_ours(x)) {
            continue;
        }
        if let Some(x) = g.take() {
            if guard_ours(&x) {
                let free: unsafe extern "system" fn(usize) = std::mem::transmute(super::bodyhits::base() + FREE_ATTACK_RVA);
                free(x.obj);
            }
        }
        GUARD_OBJ[side].store(0, Ordering::Relaxed);
        if !on {
            continue;
        }
        // (`guard_channels` is [right, left].)
        match ch[if side == RIGHT { 0 } else { 1 }] {
            Some(c) => {
                *g = make_guard(side, actor, m, req, c, clip);
                GUARD_OBJ[side].store(g.as_ref().map_or(0, |x| x.obj), Ordering::Relaxed);
            }
            None if !super::kh2slots::blade_guard_spans(clip, side == LEFT).is_empty() => {
                note(|| format!("Final's {} Keyblade guard: motion {m} has no free KH1 attack group: left out", if side == LEFT { "left" } else { "right" }));
            }
            None => {}
        }
    }
}

/// Our guard object is still the one we made (in use, our data).
unsafe fn guard_ours(x: &BladeGuard) -> bool {
    rd::<u32>(x.obj) & 0x800 != 0 && pose::ptr(rd::<u32>(x.obj + 0x5C)) == x.data
}

/// (t1043) Our copy of KH1's guard row's data for one blade's guard (`make_guard`): KH1's own
/// (bone 0x30, clash type 2, no power; Checked (files) and (decomp), as `combat`'s Guard Shape KH1
/// copy), its second word 0 as that copy's, its attack group `channel` (`+0x34`, which KH1's
/// constructor puts in the object's flags), the arc the window's ATKP's (`+0x60` the angle x 2
/// degrees in radians, `+0x64` cos(range), -1 for 0; KH1's angle test `0x1402B4640`, Checked
/// (decomp), as `combat::Strikes::guard_data_of`). One per (ATKP, group, blade).
unsafe fn guard_data(attack: u16, channel: u8, side: usize, range: u8, angle: i8) -> Option<usize> {
    let key = (attack as u64) << 32 | (channel as u64) << 16 | (side as u64) << 8 | GUARD_MARK;
    let d = super::bodyhits::data_copy(key)?;
    let (src, _) = super::kh1rows::kh1_row(KH1_GUARD_ROW)?;
    std::ptr::copy_nonoverlapping(src as *const u8, d as *mut u8, DATA);
    wr(d + 8, 0u32);
    if rd::<u32>(d + 0x10) != BLADE_BONE {
        let was = rd::<u32>(d + 0x10);
        note(|| format!("Final's Keyblade guard: KH1's guard row's bone is {was:#x}, not the blade: the blade taken"));
        wr(d + 0x10, BLADE_BONE);
    }
    wr(d + 0x34, (rd::<u32>(d + 0x34) & !3) | channel as u32);
    let cos = if range == 0 { -1.0f32 } else { (range as f32).to_radians().cos() };
    wr(d + 0x60, (angle as f32 * 2.0).to_radians());
    wr(d + 0x64, cos);
    Some(d)
}

/// (t1043) One blade's KH1 guard object for a play of motion `m` (`guards_new_play`): KH1's attack
/// constructor (`0x1402B2DC0`, as KH1's row maker `0x1402B3270` makes KH1's guard) on our copy of
/// KH1's guard data (`guard_data`), with a hit bit of its own; the row maker's Keyblade part from
/// that blade's own weapon row (`blade_fields`: radius, blade range; the left Keyblade's from
/// `leftblade`'s owner copy) and its weapon object as the spark's source; its priority then KH1's
/// guard's 100 (`GUARD_PRIORITY`), KH1's re-arm every `GUARD_REARM` frames, its request taken at
/// once (`+0x58`), the left one's bone-source actor the left Keyblade's (`bone_source`). His
/// "connected" word isn't given it (KH1's clash never writes it for priority 100).
unsafe fn make_guard(side: usize, actor: usize, m: u16, req: u32, ch: u8, clip: &character_core::Clip) -> Option<BladeGuard> {
    if !super::bodyhits::make_ok() {
        return None;
    }
    let attack = clip.windows.iter().find_map(|w| match w.kind {
        character_core::WindowKind::Guard { attack, weapon, .. } if if side == LEFT { weapon >= 2 } else { weapon == 1 } => Some(attack),
        _ => None,
    })?;
    let kh2 = pose::kh2()?;
    let (range, angle) = kh2.battle.attack_for_level(attack, 0, 0).map_or((0, 0), |a| (a.reflect_range, a.reflect_angle));
    let data = guard_data(attack, ch, side, range, angle)?;
    let base = super::bodyhits::base();
    let make: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + MAKE_STEAM);
    let free_bit: unsafe extern "system" fn() -> u32 = std::mem::transmute(base + FREE_BIT_RVA);
    let owner = if side == LEFT { super::leftblade::left_owner() } else { None };
    let wobj = pose::ptr(rd::<u32>(owner.unwrap_or(actor) + 0x14C));
    let bit = free_bit();
    if bit == 0 {
        note(|| format!("Final's Keyblade guard: no free hit bit (motion {m})"));
        return None;
    }
    let obj = make(actor, bit, data);
    if super::bodyhits::pool_index(obj).is_none() {
        note(|| format!("Final's Keyblade guard: KH1's constructor gave {obj:#x} (pool full?)"));
        return None;
    }
    if wobj > 0x10000 {
        let row = rd::<u64>(wobj + 0x20) as usize;
        if rd::<u32>(obj + 0x20) == BLADE_BONE && row > 0x10000 {
            blade_fields(obj, data, actor, row);
        }
        wr(obj + 8, pose::handle(rd::<u64>(wobj + 0x18) as usize));
    }
    wr(obj + 0x68, GUARD_PRIORITY);
    wr(obj + 0x50, GUARD_REARM);
    wr(obj + 0x58, req);
    if side == LEFT {
        wr(obj + 0x90, bone_source(obj));
    }
    let spans = super::kh2slots::blade_guard_spans(clip, side == LEFT);
    note(|| {
        format!(
            "Final's {} Keyblade guard: motion {m} (request {req:#x}): KH1's guard object {obj:#x} (KH1's guard row's data, ATKP {attack}, reflect range {range}) on KH1 group {ch}, KH1's blade r {:.1} range {:.2}..{:.2}, frames {spans:?}",
            if side == LEFT { "left" } else { "right" },
            rd::<f32>(obj + 0xC),
            rd::<f32>(obj + 0x24),
            rd::<f32>(obj + 0x28),
        )
    });
    Some(BladeGuard { obj, data, motion: m, req })
}

/// Our attack data for one left window: a copy of Sora's KH1 swing row's (his finisher row's for
/// a KH2 blow; ground or air), as KH1's own Keyblade object takes it: its blade bone, blade range
/// and radius kept as they are; KH2's reaction on it (as our rows have, `kh1rows::make`), the
/// left window's KH1 attack group, the Recoil row's clash rules (`kh1rows::kh2_clash`). Made the
/// first time, refreshed each time (the clash rules can change).
/// (t1037) One copy per (ATKP, KH1 group, ground or air), what its contents are made from (before:
/// per motion and window), so the two Keyblades' windows in Final's slots don't fill
/// `bodyhits`' page of copies: Final's glide 205 / 206 / 207 share one 1281.
/// (t1046) `finisher`: the blade's last blow in the clip (`kh1rows::last_blow`), on KH1's finisher
/// row's data (its critical); any other window on the swing row's, a blow keeping kind 5. (In the
/// key, bit 9: one ATKP can be a last blow in one clip and not in another.)
unsafe fn blade_data(attack: u16, air: bool, channel: u8, a: &kh2_formats::battle::Atkp, finisher: bool) -> Option<usize> {
    let key = (attack as u64) << 32 | (channel as u64) << 16 | (finisher as u64) << 9 | (air as u64) << 8 | KEY_MARK;
    let d = super::bodyhits::data_copy(key)?;
    let throws = super::kh1rows::throws(a);
    let (src, _) = super::kh1rows::kh1_row(super::bodyhits::template_row(air, throws && finisher))?;
    std::ptr::copy_nonoverlapping(src as *const u8, d as *mut u8, DATA);
    let kind = if throws {
        5u32
    } else if character_core::reaction::category(a.enemy_reaction) == 3 {
        3
    } else {
        rd::<u32>(d + 0xC)
    };
    wr(d + 8, kind);
    wr(d + 0xC, kind);
    if rd::<u32>(d + 0x10) != BLADE_BONE {
        // (Sora's swing rows are on the blade; if not, the blade, with the row's range and radius.)
        let was = rd::<u32>(d + 0x10);
        note(|| format!("left Keyblade hits: KH1's template row's bone is {was:#x}, not the blade: the blade taken"));
        wr(d + 0x10, BLADE_BONE);
    }
    wr(d + 0x34, (rd::<u32>(d + 0x34) & !3) | channel as u32);
    // A KH2 Magic-based window: KH1's own magic stat (`kh1rows::kh1_magic`, Ripple Drive's).
    if a.stat_source == 1 {
        super::kh1rows::kh1_magic_row(d);
    }
    let (t1, p1) = (rd::<u32>(d + 0x38) & 7, rd::<i32>(d + 0x3C));
    let (t, p) = if super::recoil::kh2() { super::kh1rows::kh2_clash(Some(a), t1, p1) } else { (t1, p1) };
    // (The data's own "spent" bit 16 kept: KH1's wall test as for the right Keyblade.)
    wr(d + 0x38, (rd::<u32>(d + 0x38) & !7) | t);
    wr(d + 0x3C, p);
    Some(d)
}

/// The blade's fields of the object from `data`, as KH1's constructor and row maker give them
/// (`0x1402B2DC0`, `0x1402B3270`, Checked (decomp)): radius `+0xC` (the data's, times the maker's
/// scale with its `+0x374` bit `0x2000`, plus scale × row `+0x20`), blade range `+0x24` / `+0x28`
/// (the data's × row `+0x2C`), priority `+0x68` (the data's + row `+0x33`, 1..99).
unsafe fn blade_fields(obj: usize, data: usize, actor: usize, row: usize) {
    let scale = rd::<f32>(actor + 0x40);
    let mut r = rd::<f32>(data + 0x5C);
    if rd::<u32>(actor + 0x374) & 0x2000 != 0 {
        r *= scale;
    }
    let (mut t1, mut t2) = (rd::<f32>(data + 0x54), rd::<f32>(data + 0x58));
    let mut priority = rd::<i32>(data + 0x3C);
    if row > 0x10000 {
        r += scale * rd::<f32>(row + 0x20);
        t1 *= rd::<f32>(row + 0x2C);
        t2 *= rd::<f32>(row + 0x2C);
        priority += rd::<u8>(row + 0x33) as i32;
    }
    wr(obj + 0xC, r);
    wr(obj + 0x24, t1);
    wr(obj + 0x28, t2);
    wr(obj + 0x68, priority.clamp(1, 99));
}

/// (t1046) The object's critical from `data`, as KH1's constructor and row maker give it
/// (`0x1402B2DC0`: chance `+0x94` = data `+0x18` × 0.05 × the maker's stats `+0xD8`, bonus `+0x98`
/// = data `+0x1C`; `0x1402B3270` on a blade: chance × row `+0x31` × 0.05, bonus + row `+0x32`;
/// Checked (decomp)), for `step`'s window switch (KH1 sets them only at make).
unsafe fn crit_fields(obj: usize, data: usize, actor: usize, row: usize) {
    let stats = pose::ptr(rd::<u32>(actor + 0x6C));
    let per = if stats > 0x10000 { rd::<f32>(stats + 0xD8) as f64 } else { 1.0 };
    let mut chance = (rd::<i32>(data + 0x18) as f64 * 0.05 * per) as f32;
    let mut bonus = rd::<i32>(data + 0x1C);
    if row > 0x10000 {
        chance *= rd::<u8>(row + 0x31) as f32 * 0.05;
        bonus += rd::<i8>(row + 0x32) as i32;
    }
    wr(obj + 0x94, chance);
    wr(obj + 0x98, bonus);
}

/// The left Keyblade's object for a new play of motion `m`: one KH1 attack object of Sora's,
/// made with KH1's attack constructor (`0x1402B2DC0`) on our copy of his KH1 swing row's data,
/// on the clip's left attack group (`kh2slots::left_channel`), with the left Keyblade's own
/// weapon row on it as KH1's row maker puts the right one's (`0x1402B3270`), its effect data as
/// the spark's source, his "connected" word, and the left Keyblade's owner copy as its
/// bone-source actor.
/// (t1037) `RIGHT`: the right Keyblade's windows (weapon 1) on KH1's group 0, with Sora's own
/// weapon row and effect data, Sora (the constructor's own) as its bone-source actor.
unsafe fn make(side: usize, actor: usize, m: u16, req: u32, air: bool, clip: &character_core::Clip) -> Option<Left> {
    if !super::bodyhits::make_ok() {
        return None;
    }
    let mine = |weapon: u8| if side == LEFT { weapon >= 2 } else { weapon == 1 };
    let ch = if side == LEFT {
        let Some(ch) = super::kh2slots::left_channel(clip) else {
            if clip.windows.iter().any(|w| matches!(w.kind, character_core::WindowKind::Hitbox { weapon, .. } if weapon >= 2)) {
                note(|| format!("left Keyblade hits: motion {m} has no free KH1 attack group: left out"));
            }
            return None;
        };
        ch
    } else {
        // (KH1's Keyblade group, `kh2slots::kh1_tracks`' HIT track 0.)
        0
    };
    let kh2 = pose::kh2()?;
    let base = super::bodyhits::base();
    let mut windows = Vec::new();
    // (t1046) This blade's last blow, in a finisher (`kh1rows::finisher_request`): the one window
    // on the finisher's data.
    let carrier = if side == LEFT { super::kh1rows::Carrier::Left } else { super::kh1rows::Carrier::Right };
    let last = if super::kh1rows::finisher_request(req) { super::kh1rows::last_blow(clip, carrier) } else { None };
    for w in &clip.windows {
        let character_core::WindowKind::Hitbox { attack, group, weapon, .. } = w.kind else { continue };
        if !mine(weapon) {
            continue;
        }
        let Some(a) = kh2.battle.attack_for_level(attack, 0, 0).cloned() else { continue };
        let finisher = super::kh1rows::throws(&a) && last == Some(w.start);
        let Some(data) = blade_data(attack, air, ch, &a, finisher) else { continue };
        windows.push(LeftWin { attack, group, start: w.start, end: w.end, data, last: finisher });
    }
    let first = windows.first().copied()?;
    let make: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + MAKE_STEAM);
    let free_bit: unsafe extern "system" fn() -> u32 = std::mem::transmute(base + FREE_BIT_RVA);
    // The left Keyblade's weapon object (`leftblade`'s owner copy: the left Keyblade's row and
    // effect data), else Sora's own. (The right Keyblade's: Sora's own.)
    let owner = if side == LEFT { super::leftblade::left_owner() } else { None };
    let wobj = pose::ptr(rd::<u32>(owner.unwrap_or(actor) + 0x14C));
    let info = pose::ptr(rd::<u32>(actor + 0x14C));
    let bit = free_bit();
    if bit == 0 {
        note(|| format!("left Keyblade hits: no free hit bit (motion {m})"));
        return None;
    }
    let obj = make(actor, bit, first.data);
    if super::bodyhits::pool_index(obj).is_none() {
        note(|| format!("left Keyblade hits: KH1's constructor gave {obj:#x} (pool full?)"));
        return None;
    }
    // `0x1402B3270`'s weapon part, with the left Keyblade's row (Checked (decomp)).
    let mut row = 0;
    if wobj > 0x10000 {
        row = rd::<u64>(wobj + 0x20) as usize;
        if rd::<u32>(obj + 0x20) == BLADE_BONE && row > 0x10000 {
            blade_fields(obj, first.data, actor, row);
            wr(obj + 0x94, rd::<u8>(row + 0x31) as f32 * 0.05 * rd::<f32>(obj + 0x94));
            wr(obj + 0x98, rd::<i32>(obj + 0x98) + rd::<i8>(row + 0x32) as i32);
        } else {
            row = 0;
        }
        wr(obj + 8, pose::handle(rd::<u64>(wobj + 0x18) as usize));
    }
    // His "connected" word (KH1's Attack step reads it; a hit sets it), not cleared: not a new swing.
    if info > 0x10000 {
        wr(obj + 0x60, (info + 0x50) as u64);
    }
    if let Some(fx) = super::bodyhits::template_fx(air, false) {
        wr(obj, rd::<u32>(obj) | 0x40_0000);
        wr(obj + 0xA8, fx);
    }
    wr(obj + 0x58, req);
    // KH1's builder takes the blade's ends from this actor (`0x1402B3800`): the left Keyblade's.
    // (The right one's: Sora, as the constructor set it.)
    if side == LEFT {
        wr(obj + 0x90, bone_source(obj));
    }
    let list: Vec<String> = windows.iter().map(|w| format!("ATKP {} at {}..{:?}", w.attack, w.start, w.end)).collect();
    note(|| {
        format!(
            "{} Keyblade hits: motion {m} (request {req:#x}, {}): KH1 attack object {obj:#x} on KH1 group {ch}, KH1's blade r {:.1} range {:.2}..{:.2}{}: {}",
            if side == LEFT { "left" } else { "right (no KH1 attack row here)" },
            if air { "air" } else { "ground" },
            rd::<f32>(obj + 0xC),
            rd::<f32>(obj + 0x24),
            rd::<f32>(obj + 0x28),
            if side == RIGHT || owner.is_some() { "" } else { " (left Keyblade not drawn: on the right one)" },
            list.join("; ")
        )
    });
    Some(Left { obj, data: first.data, motion: m, req, windows, cur: None, was_open: false, row, actor })
}

/// Each frame KH1 has Sora (`takeover::physics`, beside `kh2slots::keyblade_rearm`, before KH1's
/// collision pass): the left window open at KH1's time in our slot. A new KH2 attack there (its
/// ATKP or group differs from the last window's, or a window opens after a frame with none: Steam KH2
/// `0x1403D25D0` keeps an attack per (group, weapon object), Checked (decomp)) re-arms the object
/// with KH1's own re-arm (`rehit::new_attack`); each window's data (its reaction, clash rules,
/// KH1 blade fields) goes on the object. (Its bone-source actor: `before_build`, once a build.)
/// (t1037) Both objects, the left one's and the right one's (`RIGHT`); nothing with neither.
pub unsafe fn step(actor: usize) {
    if !super::bodyhits::native() || actor < 0x10000 {
        return;
    }
    for side in SIDES {
        if OBJ[side].load(Ordering::Relaxed) != 0 {
            step_one(side, actor);
        }
    }
}

unsafe fn step_one(side: usize, actor: usize) {
    let Ok(mut g) = STATE[side].try_lock() else { return };
    if !g.as_ref().is_some_and(|l| still_ours(l)) {
        *g = None;
        OBJ[side].store(0, Ordering::Relaxed);
        return;
    }
    let obj = g.as_ref().map_or(0, |l| l.obj);
    let Some((m, _, t)) = super::kh2slots::playing_clip(actor) else { return };
    let Some(l) = g.as_mut() else { return };
    if m != l.motion {
        return;
    }
    let f = t as u32;
    let Some(o) = l.windows.iter().position(|w| w.contains(f)) else {
        l.was_open = false;
        return;
    };
    let w = l.windows[o];
    let new = match l.cur {
        Some(c) => !l.was_open || l.windows[c].attack != w.attack || l.windows[c].group != w.group,
        None => false,
    };
    let switch = l.cur != Some(o);
    l.cur = Some(o);
    l.was_open = true;
    if switch {
        wr(obj + 0x5C, pose::handle(w.data));
        blade_fields(obj, w.data, l.actor, l.row);
        crit_fields(obj, w.data, l.actor, l.row);
        l.data = w.data;
    }
    drop(g);
    if new {
        super::rehit::new_attack(obj);
        static N: AtomicU32 = AtomicU32::new(0);
        if super::log_on(super::LogCat::Combat) && N.fetch_add(1, Ordering::Relaxed) < 40 {
            log!(Combat, "{} Keyblade hits: KH2's next attack (motion {m}, ATKP {} group {}, frame {t:.0}): its KH1 object re-armed", if side == LEFT { "left" } else { "right" }, w.attack, w.group);
        }
    }
}

/// `bodyhits`' hook on KH1's `0x1402B3800 (object, entries, count)`, before KH1's own builder runs
/// for `obj`: for our object, its bone-source actor (`+0x90`) brought up to date (`leftblade`'s
/// owner copy filled from this frame's matrices). KH1's builder does the rest.
/// (t1043) And for Final's left Keyblade guard object, the same.
pub unsafe fn before_build(obj: usize) {
    if obj == 0 || (OBJ[LEFT].load(Ordering::Relaxed) != obj && GUARD_OBJ[LEFT].load(Ordering::Relaxed) != obj) {
        return;
    }
    if rd::<u32>(obj) & 0x800 != 0 {
        wr(obj + 0x90, bone_source(obj));
    }
}

/// (t1045) `bodyhits::reclaim`, before it reuses our data copies: the copies our live objects
/// use (a hit object's every window's, which `step` puts on it later; a guard object's), into
/// `out`. Ours KH1 freed are forgotten here, so none of them takes a reused copy on a reused pool
/// object for its own (`still_ours` / `guard_ours` compare the object's data). A side whose lock
/// is taken is being made now (`new_play` / `guards_new_play` took its last state first, Checked
/// (code)): its new copies are this frame's, which `reclaim` keeps.
pub(super) unsafe fn held(out: &mut Vec<usize>) {
    for side in SIDES {
        if let Ok(mut g) = STATE[side].try_lock() {
            let live = g.as_ref().map(|l| still_ours(l));
            match live {
                Some(true) => out.extend(g.iter().flat_map(|l| l.windows.iter().map(|w| w.data))),
                Some(false) => {
                    *g = None;
                    OBJ[side].store(0, Ordering::Relaxed);
                }
                None => {}
            }
        }
        if let Ok(mut g) = GUARD[side].try_lock() {
            let live = g.as_ref().map(|x| guard_ours(x));
            match live {
                Some(true) => out.extend(g.iter().map(|x| x.data)),
                Some(false) => {
                    *g = None;
                    GUARD_OBJ[side].store(0, Ordering::Relaxed);
                }
                None => {}
            }
        }
    }
}

/// Which of our objects `attack` is (`LEFT` / `RIGHT`).
fn side_of(attack: usize) -> Option<usize> {
    if attack == 0 {
        return None;
    }
    SIDES.into_iter().find(|&s| OBJ[s].load(Ordering::Relaxed) == attack)
}

/// The window's ATKP for our object (`bodyhits::atkp`, `kh1rows::kh2_atkp`): the window open at
/// the last step, else the first.
pub unsafe fn atkp(attack: usize) -> Option<kh2_formats::battle::Atkp> {
    let side = side_of(attack)?;
    let g = STATE[side].try_lock().ok()?;
    let l = g.as_ref().filter(|l| l.obj == attack && still_ours(l))?;
    let id = l.windows.get(l.cur.unwrap_or(0))?.attack;
    drop(g);
    pose::kh2()?.battle.attack_for_level(id, 0, 0).cloned()
}

/// (t1053) Whether the window open on our object (`atkp`'s) is its blade's last blow in a finisher
/// (`kh1rows::last_blow` in a move on `kh1rows::finisher_request`, as t1046's critical); `None`
/// when `attack` isn't one of ours.
pub unsafe fn last_blow(attack: usize) -> Option<bool> {
    let side = side_of(attack)?;
    let g = STATE[side].try_lock().ok()?;
    let l = g.as_ref().filter(|l| l.obj == attack && still_ours(l))?;
    Some(l.windows.get(l.cur.unwrap_or(0))?.last)
}

/// `bodyhits::hit`: KH1's contact made a hit with our object (logged).
pub unsafe fn hit(attack: usize, target: usize) {
    let Some(side) = side_of(attack) else { return };
    let Ok(g) = STATE[side].try_lock() else { return };
    let Some(l) = g.as_ref() else { return };
    let (atk, motion) = (l.windows.get(l.cur.unwrap_or(0)).map_or(0, |w| w.attack), l.motion);
    drop(g);
    note(|| format!("KH2 {} Keyblade hit: ATKP {atk} (motion {motion}) from KH1's pass on actor {target:#x}", if side == LEFT { "left" } else { "right" }));
}
