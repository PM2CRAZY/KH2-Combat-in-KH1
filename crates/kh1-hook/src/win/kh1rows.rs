//! KH2's moves as KH1 attack rows of their own (testing branch, base Sora in KH1's own states).
//! The tester (m14 question 2): "(b)", new KH1 rows and requests (from 0x200 up) with their own
//! data, so KH2's Square moves and Guard Break keep KH2's behaviour (Upper Slash's launch, the
//! guard break) instead of borrowing KH1's swing or finisher rows. `docs/kh1-anim-slots.md` §7.
//!
//! KH1's attack rows (Steam, Checked (decomp) and (exe)): one table a party character,
//! `0x142D5CAB0[object +0x4C]` (10 pointers, filled once from the offsets at `0x140528B80` by
//! `0x1402A3230`, from `0x14028AC80`'s set-up), each row 0x14 bytes: `+0` the motion request
//! (i16, -1 none), `+2` the weapon trail, `+3` an effect on him, `+4` the attack data (an offset
//! into the battle data block `0x142D22D40`, `0x14028F950`), `+8` the lunge, `+0xC..+0xE` the
//! attack object's effect and sound bytes, `+0xF` a voice. The row start (`0x1402A3980`) sets the
//! request, makes the attack object from the data (`0x1402B3270`) and plays the trail, effect and
//! voice; the row runner (`0x1402A68F0`) then moves him by the row's index (its own cases for
//! KH1's rows) or, for any other index, queues the idle (request 0) and lunges by `+8`, faces the
//! direction and counts the press in KH1's tally (`0x14028C660 (0x13)`). Sora's table is the
//! block's `+0xA9F0` with 66 rows (the next table at `+0xAF18`; the exe's offsets, Checked (exe)).
//!
//! Ours: a copy of Sora's 66 rows with ours after them (rows 66.., `row`), the table pointer for
//! his character pointed at it (and again whenever KH1's set-up writes its own back). Each of our
//! rows: KH1's own row of its kind (plain swing 13, finisher 8, air swing 18, air finisher 16) as
//! the template for its trail, effect, sound and voice bytes; its own request (`0x200 + n`,
//! `request`), whose entry in Sora's request tables is our slot of the KH2 motion
//! (`kh2slots`: built on the template's KH1 motion and request kind); no lunge (KH2's root travel
//! is in the slot, its hop KH2's, `combomods`); and its own attack data (`DATA`, a copy of the
//! template's in memory of ours near the block, its offset from the block in `+4`), with KH2's
//! reaction for the move (KH1's launch, kind 5, for a KH2 blow that throws, else a flinch, 3) and,
//! for Guard Break, KH1's priority past which guards don't stop an attack (`GUARD_THROUGH`). The
//! hits its attack object lands then take KH2's numbers for the move's hit window (power, Magic
//! scaling, launch: `combat::scale_record` through `kh2_atkp`).

use super::addr::Rva;
use super::{log, pose};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

/// KH1's attack row tables, one pointer a party character (`+0x4C` of the actor's object).
const ROWS_RVA: Rva = Rva(0x2D5D4B0);
/// The battle data block the rows' and their attack data's offsets are from (`0x14028F950`).
const BLOCK_RVA: Rva = Rva(0x2D23740);
/// Sora's own rows (the Attack step walks 0x42 of them, `0x1402A7510`).
pub const KH1_ROWS: usize = 66;
/// Our first request id (KH1's go to 0x140; its request table is 0x160 long in the file).
pub const FIRST_REQUEST: u32 = 0x200;
const ROW: usize = 0x14;
/// An attack data entry (`docs/kh1-launch.md`): `+8` / `+0xC` the reaction kind against the
/// player's side / enemies, `+0x3C` the priority.
const DATA: usize = 0x70;
/// KH1's attack-vs-attack pass (`0x1402B4BC0`) meets two attacks only while one of them is under
/// priority 0x62; KH1's guards are 100 (`combat::GUARD_PRIORITY`). So an attack of 0x62 or more
/// goes through a guard to the body: Guard Break's.
const GUARD_THROUGH: i32 = 0x62;

/// Which of KH1's own rows a row of ours is like (its template).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Swing,
    Finisher,
    AirSwing,
    AirFinisher,
    /// A spell cast of ours on KH1's cast state (KH2's Reflect, `reflect::cast_native`): KH1's
    /// ground cast (Thunder's request 0x38, slot 56) as the template, its own request only.
    Cast,
    /// The combo's finisher cast (the same template).
    CastFinisher,
    /// The air cast (KH1's air Thunder, request 0x85, slot 81).
    AirCast,
}

impl Kind {
    /// KH1's own row of this kind (`combomods`'s `GROUND_COMBO` / `AIR_COMBO`).
    fn template_row(self) -> usize {
        match self {
            Kind::Swing => 13,
            Kind::Finisher => 8,
            Kind::AirSwing => 18,
            Kind::AirFinisher => 16,
            // (A cast's row is never run, its request only: a swing's row for its data copy.)
            Kind::Cast | Kind::CastFinisher => 13,
            Kind::AirCast => 18,
        }
    }

    /// That row's request (checked against KH1's table before ours are made).
    pub fn template_request(self) -> u32 {
        match self {
            Kind::Swing => 0xC8,
            Kind::Finisher => 0xCB,
            Kind::AirSwing => 0xCC,
            Kind::AirFinisher => 0xCE,
            Kind::Cast | Kind::CastFinisher => 0x38,
            Kind::AirCast => 0x85,
        }
    }

    /// The KH1 motion slot that request plays (our slot's template, `kh2slots`).
    pub fn template_slot(self) -> usize {
        match self {
            Kind::Swing => 98,
            Kind::Finisher => 101,
            Kind::AirSwing => 102,
            Kind::AirFinisher => 104,
            Kind::Cast | Kind::CastFinisher => 56,
            Kind::AirCast => 81,
        }
    }

    pub fn air(self) -> bool {
        matches!(self, Kind::AirSwing | Kind::AirFinisher | Kind::AirCast)
    }

    pub fn finisher(self) -> bool {
        matches!(self, Kind::Finisher | Kind::AirFinisher | Kind::CastFinisher)
    }

    /// A spell cast (`Cast`, `CastFinisher`, `AirCast`): built as a cast slot (`kh2slots`: KH2's
    /// cast follow-ups, its cut and magic windows).
    pub fn cast(self) -> bool {
        matches!(self, Kind::Cast | Kind::CastFinisher | Kind::AirCast)
    }
}

/// A KH2 move on a KH1 row of its own.
pub struct Ours {
    pub name: &'static str,
    /// Its row in Sora's PTYA (`00battle.bin`, Checked (files)).
    pub kh2_row: u8,
    pub motion: u16,
    pub kind: Kind,
    /// It leaves the ground by its motion's root (Finishing Leap: up 150 over its frames, with no
    /// gravity from frame 1, then its fall, `next_motion` 4): KH1's air state from its start, the
    /// root's rise moving him (`shared::physics`'s air step, `kh2slots::air_root`).
    pub rises: bool,
    /// Guard Break: KH1's guards don't stop it (`GUARD_THROUGH`).
    pub guard_break: bool,
    /// Started by KH1's row runner (an attack: its row's attack object, the press in KH1's
    /// tally); else only its request is asked for (Aerial Recovery, `combomods::recover`), its
    /// row never run.
    pub attack: bool,
    /// Finishing Plus's next finisher (the tester, m14 question 4: "(b)", a new row): its request
    /// has a slot of each KH2 finisher KH1's finisher request of its kind plays (`kh2slots`), the
    /// one KH2 picks pointed at (`kh2slots::pick_own`); `motion` is the first.
    pub next_finisher: bool,
    /// Its slot's length in 60 Hz frames when longer than the KH2 clip (the clip loops over it
    /// as KH2 loops it: the Aerial Dodge's motion 202 loops from its frame 2 until KH2's apex
    /// swaps it for the fall, `combomods::dodge_apex`); `None` the clip's own length.
    pub frames: Option<u32>,
    /// Only KH2's own sounds for it (`kh2slots::frame`): the template's sounds left out of the
    /// slot (the Aerial Dodge: KH2's 999019 alone, not KH1's air swing's swish).
    pub own_sounds: bool,
}

/// `Ours::kh2_row` for a move with no PTYA row (Aerial Recovery; Sora's rows go to 59).
pub const NO_ROW: u8 = u8::MAX;

/// Ours (Sora's PTYA rows and motions, Checked (files)): the Square moves Upper Slash (row 12,
/// motion 161, ability 18), Finishing Leap (37, 167, ability 95, `combo_offset` 1), Horizontal
/// Slash (38, 193, in the air, ability 99, a hop of 14) and the finisher Guard Break (27, 165,
/// ability 93). Out of KH1's knock-away (`docs/aerial-recovery.md`): Retaliating Slash (row 40,
/// motion 172, ability 101, an air row with a hop of 21..121) and Aerial Recovery (motion 40, no
/// row: its request only). KH2's guard (the tester, combo row selection m15 question 3: "(a)",
/// Guard native, then Counterguard on its own KH1 row; `combomods::guard_press`): the guard press
/// (row 11, motion 173, its request only), its repel (motion 42, no row, its request only, asked
/// for when the guard stops a hit, `combomods::guard_repel`) and Counterguard (row 39, motion 171,
/// ability 96: Attack during the repel, an attack row). The Aerial Dodge (motion 202, its request
/// only, `combomods::aerial_dodge`) and Quick Run (the tester, quick-run-native: its run 203 and its
/// stop 204, their requests only, asked by our KH1 state for it, `quickrun`; KH1's ground swing as
/// their template, its sounds left out). KH2's glide hover (the tester, kh2-glide-in-kh1s-glide-state:
/// motion 208, its request only, asked by KH1's glide state at Glide KH2, `glide`; KH1's air swing
/// as its template, its sounds left out).
/// KH2's Reflect on KH1's cast state (the tester,
/// kh2-magic-and-reflect-native m5 plan item 2; `reflect::cast_native`): its MAGC cast motions
/// (Checked (files), every level: ground 71, combo finisher 72, air 73), their requests only, on
/// KH1's Thunder cast slots 56 / 81 as templates (their sounds left out: KH2's own play).
pub const OURS: [Ours; 18] = [
    Ours { name: "Upper Slash", kh2_row: 12, motion: 161, kind: Kind::Swing, rises: false, guard_break: false, attack: true, next_finisher: false, frames: None, own_sounds: false },
    Ours { name: "Finishing Leap", kh2_row: 37, motion: 167, kind: Kind::AirFinisher, rises: true, guard_break: false, attack: true, next_finisher: false, frames: None, own_sounds: false },
    Ours { name: "Horizontal Slash", kh2_row: 38, motion: 193, kind: Kind::AirSwing, rises: false, guard_break: false, attack: true, next_finisher: false, frames: None, own_sounds: false },
    Ours { name: "Guard Break", kh2_row: 27, motion: 165, kind: Kind::Finisher, rises: false, guard_break: true, attack: true, next_finisher: false, frames: None, own_sounds: false },
    Ours { name: "Retaliating Slash", kh2_row: 40, motion: 172, kind: Kind::AirSwing, rises: false, guard_break: false, attack: true, next_finisher: false, frames: None, own_sounds: false },
    Ours { name: "Aerial Recovery", kh2_row: NO_ROW, motion: character_core::motion::AIR_RECOVERY, kind: Kind::AirSwing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: false },
    Ours { name: "Finishing Plus", kh2_row: NO_ROW, motion: 154, kind: Kind::Finisher, rises: false, guard_break: false, attack: true, next_finisher: true, frames: None, own_sounds: false },
    Ours { name: "Finishing Plus (air)", kh2_row: NO_ROW, motion: 183, kind: Kind::AirFinisher, rises: false, guard_break: false, attack: true, next_finisher: true, frames: None, own_sounds: false },
    Ours { name: "Guard", kh2_row: GUARD_ROW, motion: character_core::motion::GUARD_PRESS, kind: Kind::Swing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: false },
    Ours { name: "Guard repel", kh2_row: NO_ROW, motion: character_core::motion::GUARD, kind: Kind::Swing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: false },
    Ours { name: "Counterguard", kh2_row: COUNTERGUARD_ROW, motion: 171, kind: Kind::Swing, rises: false, guard_break: false, attack: true, next_finisher: false, frames: None, own_sounds: false },
    Ours { name: "Aerial Dodge", kh2_row: NO_ROW, motion: character_core::motion::AERIAL_DODGE, kind: Kind::AirSwing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: Some(AERIAL_DODGE_FRAMES), own_sounds: true },
    Ours { name: "Quick Run", kh2_row: NO_ROW, motion: super::quickrun::RUN_MOTION, kind: Kind::Swing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true },
    Ours { name: "Quick Run stop", kh2_row: NO_ROW, motion: super::quickrun::STOP_MOTION, kind: Kind::Swing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true },
    Ours { name: "Glide hover", kh2_row: NO_ROW, motion: character_core::motion::GLIDE_HOVER, kind: Kind::AirSwing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true },
    Ours { name: "Reflect", kh2_row: NO_ROW, motion: super::reflect::GROUND_MOTION, kind: Kind::Cast, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true },
    Ours { name: "Reflect (finisher)", kh2_row: NO_ROW, motion: super::reflect::FINISHER_MOTION, kind: Kind::CastFinisher, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true },
    Ours { name: "Reflect (air)", kh2_row: NO_ROW, motion: super::reflect::AIR_MOTION, kind: Kind::AirCast, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true },
];

/// The Aerial Dodge's slot length (60 Hz frames): longer than any of its rises reaches its apex
/// (KH2's MAX, 300 high: about 77 frames on KH2's gravity, 35 on KH1's), so the slot is still
/// playing KH2's loop when the apex asks for the fall.
pub const AERIAL_DODGE_FRAMES: u32 = 160;

/// Sora's PTYA rows for the guard and Counterguard (Checked (files): row 11 kind 0, motion 173,
/// ability 1; row 39 kind 0, motion 171, ability 96).
pub const GUARD_ROW: u8 = 11;
pub const COUNTERGUARD_ROW: u8 = 39;

/// The next-finisher row of ours on the ground or in the air (`Ours::next_finisher`).
pub fn next_finisher(air: bool) -> Option<usize> {
    OURS.iter().position(|o| o.next_finisher && o.kind.air() == air)
}

/// Our `n`'s KH1 row index.
pub fn row(n: usize) -> i32 {
    (KH1_ROWS + n) as i32
}

/// Our `n`'s request id.
pub fn request(n: usize) -> u32 {
    FIRST_REQUEST + n as u32
}

/// The request ids ours need room for in Sora's request tables (`kh2slots`).
pub fn requests_end() -> usize {
    (FIRST_REQUEST as usize) + OURS.len()
}

/// Ours for KH2's PTYA row `id`, if any.
pub fn by_kh2_row(id: u8) -> Option<usize> {
    OURS.iter().position(|o| o.kh2_row == id && id != NO_ROW)
}

/// Ours for KH2's `motion` (a move with no row: Aerial Recovery).
pub fn by_motion(motion: u16) -> Option<usize> {
    OURS.iter().position(|o| o.motion == motion && !o.next_finisher)
}

/// Ours at KH1 row index `row`, if it is one of ours.
pub fn by_row(row: i32) -> Option<&'static Ours> {
    usize::try_from(row).ok().and_then(|r| r.checked_sub(KH1_ROWS)).and_then(|n| OURS.get(n))
}

/// Our table for Sora's character: KH1's table it copies, ours, our data.
struct Table {
    index: u16,
    kh1: usize,
    ours: usize,
}

static TABLE: Mutex<Option<Table>> = Mutex::new(None);
/// Our rows' attack data (their addresses; 0 not made), for `kh2_atkp`.
static DATA_AT: [AtomicUsize; OURS.len()] = [const { AtomicUsize::new(0) }; OURS.len()];
/// The KH2 ATKP each of our rows was made for (its first hit window's), for `kh2_atkp` when no
/// window is open at the hit.
static ATKP_OF: [AtomicU32; OURS.len()] = [const { AtomicU32::new(u32::MAX) }; OURS.len()];

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn note(s: String) {
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 60 {
        log(&s);
    }
}

/// Memory for our attack data within an i32 offset of the block (rows hold offsets, not
/// pointers): a page past it, at the first free address from 16 MB on.
pub(super) unsafe fn near(block: usize) -> Option<usize> {
    for k in 0..512usize {
        let at = (block + 0x0100_0000 + k * 0x0010_0000) & !0xFFFF;
        let p = super::sys::VirtualAlloc(at as *mut _, 0x1000, 0x3000, 0x04) as usize;
        if p == at {
            return Some(p);
        }
        if p != 0 {
            // (Not where asked: not ours to keep.)
            return None;
        }
    }
    None
}

/// The KH2 ATKP of a move's hit window at `frame` (the Keyblade's first, else his body's), or its
/// first one at all with `frame` `None`.
fn window_atkp(motion: u16, frame: Option<u32>) -> Option<u16> {
    use character_core::WindowKind as W;
    let clip = pose::kh2()?.character.moveset.clips.get(&motion)?;
    let hits = |weapon_side: bool| {
        clip.windows.iter().filter(move |w| frame.map_or(true, |f| w.contains(f))).find_map(move |w| match w.kind {
            W::Hitbox { attack, weapon, .. } if (weapon > 0) == weapon_side => Some(attack),
            _ => None,
        })
    };
    hits(true).or_else(|| hits(false))
}

/// KH2's blow that throws (a launch: as `combat::Strikes::strike` takes a KH2 hit for KH1's
/// finisher row and KH2's launch).
pub fn throws(a: &kh2_formats::battle::Atkp) -> bool {
    let k1 = if a.knockback_1 == i16::MAX { 0 } else { a.knockback_1 };
    let k2 = a.knockback_2;
    character_core::reaction::category(a.enemy_reaction) == 4 && ((k1 == 0 && k2 == 0) || k2 >= 10 || k1 >= 30)
}

/// Each frame base Sora is posed (`kh2slots::frame`, ours built): his character's row table is
/// ours (made the first time from KH1's). Only once our slots are on his request tables, so a row
/// of ours never asks for a request KH1 can't play.
pub unsafe fn install(actor: usize) {
    if actor < 0x10000 {
        return;
    }
    let base = super::takeover::game_base();
    let obj = pose::ptr(rd::<u32>(actor + 0x130));
    if obj < 0x10000 {
        return;
    }
    let index = rd::<u16>(obj + 0x4C);
    if index >= 10 {
        return;
    }
    let at = base + ROWS_RVA + index as usize * 8;
    let now = rd::<u64>(at) as usize;
    let Ok(mut g) = TABLE.try_lock() else { return };
    if let Some(t) = g.as_ref() {
        if t.index == index && now == t.ours {
            return;
        }
        if t.index == index && now == t.kh1 {
            wr(at, t.ours as u64);
            note(format!("KH1's attack rows: Sora's table ours again (KH1 set its own, {now:#x})"));
            return;
        }
    }
    if now < 0x10000 {
        return;
    }
    let Some(t) = make(base, index, now) else {
        // (Not again for this table.)
        *g = Some(Table { index, kh1: now, ours: now });
        return;
    };
    wr(at, t.ours as u64);
    *g = Some(t);
}

unsafe fn make(base: usize, index: u16, kh1: usize) -> Option<Table> {
    let block = base + BLOCK_RVA;
    // KH1's rows of each kind are what we take them for.
    for k in [Kind::Swing, Kind::Finisher, Kind::AirSwing, Kind::AirFinisher] {
        let q = rd::<i16>(kh1 + k.template_row() * ROW);
        if q as i32 as u32 != k.template_request() {
            log(&format!("KH1's attack rows: Sora's row {} asks for {q:#x}, not {:#x}: no rows of ours", k.template_row(), k.template_request()));
            return None;
        }
    }
    let Some(kh2) = pose::kh2() else { return None };
    let Some(mem) = near(block) else {
        log("KH1's attack rows: no memory within reach of KH1's battle data for our attack data: no rows of ours");
        return None;
    };
    let rows: &'static mut [u8] = Box::leak(vec![0u8; (KH1_ROWS + OURS.len()) * ROW].into_boxed_slice());
    std::ptr::copy_nonoverlapping(kh1 as *const u8, rows.as_mut_ptr(), KH1_ROWS * ROW);
    let mut made = Vec::new();
    for (n, o) in OURS.iter().enumerate() {
        let t = kh1 + o.kind.template_row() * ROW;
        let r = rows.as_mut_ptr() as usize + (KH1_ROWS + n) * ROW;
        std::ptr::copy_nonoverlapping(t as *const u8, r as *mut u8, ROW);
        // Its own data: the template's, with KH2's reaction for the move.
        let src = block.wrapping_add(rd::<i32>(t + 4) as isize as usize);
        let d = mem + n * 0x80;
        std::ptr::copy_nonoverlapping(src as *const u8, d as *mut u8, DATA);
        let atkp = window_atkp(o.motion, None);
        let a = atkp.and_then(|id| kh2.battle.attack_for_level(id, 0, 0));
        let kind = match a {
            Some(a) if throws(a) => 5u32,
            Some(a) if character_core::reaction::category(a.enemy_reaction) == 3 => 3,
            _ => rd::<u32>(d + 0xC),
        };
        wr(d + 8, kind);
        wr(d + 0xC, kind);
        if o.guard_break {
            wr(d + 0x3C, rd::<i32>(d + 0x3C).max(GUARD_THROUGH));
        }
        let off = d as isize - block as isize;
        if off < i32::MIN as isize || off > i32::MAX as isize {
            log("KH1's attack rows: our attack data is out of an offset's reach: no rows of ours");
            return None;
        }
        // (A move started by its request alone has a row that asks for nothing, should it run.)
        wr(r, if o.attack { request(n) as i16 } else { -1i16 });
        wr(r + 4, off as i32);
        wr(r + 8, 0.0f32);
        DATA_AT[n].store(d, Ordering::Relaxed);
        ATKP_OF[n].store(atkp.map_or(u32::MAX, |a| a as u32), Ordering::Relaxed);
        made.push(format!(
            "row {} {} (KH2 row {} motion {}, request {:#x}, like KH1's row {}: data {off:#x}, reaction {kind}, priority {}, KH2 ATKP {:?})",
            KH1_ROWS + n,
            o.name,
            o.kh2_row,
            o.motion,
            request(n),
            o.kind.template_row(),
            rd::<i32>(d + 0x3C),
            atkp
        ));
    }
    log(&format!("KH1's attack rows: Sora's table (character {index}, KH1's at {kh1:#x}) ours with {}", made.join("; ")));
    Some(Table { index, kh1, ours: rows.as_ptr() as usize })
}

/// Our rows are on Sora's character now.
pub fn installed() -> bool {
    TABLE.try_lock().ok().and_then(|g| g.as_ref().map(|t| t.ours != t.kh1)).unwrap_or(false)
}

/// The KH2 ATKP for a hit KH1's contact just made with `attack` (an attack object of Sora's), if
/// its data is one of our rows' (`combat::scale_record`): the move's hit window open now at KH1's
/// time in our slot (`kh2slots::now_playing`), else the one the row was made for.
pub unsafe fn kh2_atkp(attack: usize) -> Option<kh2_formats::battle::Atkp> {
    if attack < 0x10000 {
        return None;
    }
    let data = pose::ptr(rd::<u32>(attack + 0x5C));
    let n = DATA_AT.iter().position(|d| data != 0 && d.load(Ordering::Relaxed) == data)?;
    let kh2 = pose::kh2()?;
    let sora = super::takeover::sora_actor();
    // (The KH2 motion our slot plays: a next-finisher row's is the one KH2 picked.)
    let now = super::kh2slots::now_playing(sora).and_then(|(m, t)| window_atkp(m, Some(t as u32)));
    let id = now.or_else(|| u16::try_from(ATKP_OF[n].load(Ordering::Relaxed)).ok())?;
    kh2.battle.attack_for_level(id, 0, 0).cloned()
}

/// The KH2 ATKP for a native swing on one of KH1's own rows (`kh2damage`): the hit window of the
/// KH2 motion our slot plays on `actor` now at KH1's time in it, else that motion's first one;
/// `None` when no slot of ours plays (Base Combo KH1: KH1's own swing, KH1's own damage).
pub unsafe fn native_atkp(actor: usize) -> Option<kh2_formats::battle::Atkp> {
    if actor < 0x10000 {
        return None;
    }
    let (m, t) = super::kh2slots::now_playing(actor)?;
    let id = window_atkp(m, Some(t as u32)).or_else(|| window_atkp(m, None))?;
    pose::kh2()?.battle.attack_for_level(id, 0, 0).cloned()
}

/// After KH1's row runner ran one of our air rows (`combat::exec`): the follow-up it queued for
/// any row not its own (the idle, request 0, `0x14029FA30 (actor, 0x40000000, 0)`) is KH1's fall
/// instead (request 6), as its own air rows queue (rows 15..18). The queue (`0x14029F980`): its
/// count at `+0x178`, entries of 0x10 from `+0x17C` (the request first).
pub unsafe fn queue_fall(actor: usize) {
    let count = rd::<u32>(actor + 0x178);
    if count != 2 {
        return;
    }
    let e = actor + 0x17C + 0x10;
    if rd::<u32>(e) == 0x4000_0000 {
        wr(e, 0x4000_0006u32);
    }
}

/// KH1's command record (`0x142D53A40`): type byte, sub id (bits 8..23), flags; `+4` the target
/// (8 bytes: a handle and its part).
const COMMAND_RVA: Rva = Rva(0x2D54440);
/// KH1's lock-on: its state and its target (the menu's Attack takes it, `0x140293480`).
const LOCK_STATE_RVA: Rva = Rva(0x2DB3F10);
const LOCK_TARGET_RVA: Rva = Rva(0x2DB3F18);
const TO_HANDLE_RVA: Rva = Rva(0x38B080);

/// KH1's Attack command, as its command menu writes it (`0x140293480`, Steam, Checked (decomp)):
/// the type byte 1, sub 0, the record's high flags kept but its hold bit (`0x1000000`, which keeps
/// the control step from running it) clear, and the target: the lock-on's while it holds one
/// (`0x142DB3510` set, `0x142DB3518`), else none (a handle of 0, `0x14038AD90 (0)`). KH1's control
/// step (`0x1402A6220`) runs a command it finds there through its command step (`0x1402A7510`)
/// the same frame, then clears the record.
pub unsafe fn command_attack() {
    let base = super::takeover::game_base();
    let rec = base + COMMAND_RVA;
    let flags = rd::<u32>(rec) & 0xFF00_0000 & !0x0100_0000;
    wr(rec, flags | 1);
    let target: u64 = if rd::<u32>(base + LOCK_STATE_RVA) != 0 {
        rd(base + LOCK_TARGET_RVA)
    } else {
        let to_handle: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(base + TO_HANDLE_RVA);
        to_handle(0) as u64
    };
    wr(rec + 4, target);
}

/// KH1's locked target's actor (the lock-on's while it holds one, `0x142DB3510` / `0x142DB3518`
/// as `command_attack` reads them), if any: what KH2's guard turns him toward (KH2's guard faces
/// its lock only, `Character::try_action_button`).
pub unsafe fn lock_actor() -> Option<usize> {
    let base = super::takeover::game_base();
    if rd::<u32>(base + LOCK_STATE_RVA) == 0 {
        return None;
    }
    let a = pose::ptr(rd::<u32>(base + LOCK_TARGET_RVA));
    (a >= 0x10000 && super::crash::readable(a + 0x1F)).then_some(a)
}
