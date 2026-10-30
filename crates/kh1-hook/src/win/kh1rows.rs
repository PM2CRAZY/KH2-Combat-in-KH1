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
//! reaction for the move (KH1's launch, kind 5, for a KH2 blow that throws, else a flinch, 3) and
//! the Recoil row's clash rules (`rules`: the template's at KH1, KH2's attack kinds at KH2). The
//! hits its attack object lands then take KH2's numbers for the move's hit window (power and
//! launch: `combat::scale_record` through `kh2_atkp`; a Magic-based move's data takes KH1's
//! own magic stat at make, `kh1_magic`).

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
    pub fn template_row(self) -> usize {
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
    /// Guard Break (its clash type and priority are the Recoil row's, `rules` / `kh2_clash`, as
    /// every row's).
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
    /// One of KH2's combo rows of Sora's (`COMBO`, the tester, combo row selection after m25,
    /// answer 3 (a): "every KH2 combo row gets its own KH1 row"): found by its KH2 motion
    /// (`by_combo_motion`; several of KH2's PTYA rows share a motion: one KH1 row plays them all,
    /// KH2's rows differing only in when they're picked).
    pub combo: bool,
    /// KH1's own row it is like when not its kind's first (`Kind::template_row`): (row, its request,
    /// that request's slot). 152, KH2's second swing, on KH1's second (row 11, 0xC9, slot 99), as it
    /// stood on that request before.
    pub template: Option<(usize, u32, usize)>,
    /// A move of a Drive Form's only (its arrival, motion 53, and its stance, motion 6, of the
    /// form's moveset: `P_EX100_BTLF`, `P_EX100_MAGF`): built from each form's moveset alone that
    /// has a clip of it (`kh2slots::prepare`), never base Sora's.
    pub valor: bool,
}

impl Ours {
    /// KH1's row it copies (its trail, effect, sound and voice bytes and its attack data).
    pub fn template_row(&self) -> usize {
        self.template.map_or(self.kind.template_row(), |t| t.0)
    }

    /// That row's request.
    pub fn template_request(&self) -> u32 {
        self.template.map_or(self.kind.template_request(), |t| t.1)
    }

    /// The KH1 slot that request plays (our slot's template, `kh2slots`).
    pub fn template_slot(&self) -> usize {
        self.template.map_or(self.kind.template_slot(), |t| t.2)
    }
}

/// A KH2 combo row of Sora's on a KH1 row of its own (`COMBO`).
const fn combo(name: &'static str, motion: u16, kind: Kind) -> Ours {
    Ours { name, kh2_row: NO_ROW, motion, kind, rises: false, guard_break: false, attack: true, next_finisher: false, frames: None, own_sounds: false, combo: true, template: None, valor: false }
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
/// The Drive (Valor Form on KH1's own states, `form::native`): base Sora's Drive motion 52 (160
/// frames; its no-gravity and invulnerable ranges all through, its guard 124 and hit 125 at 18..28 on
/// his body, `P_EX100.mset`, Checked (files)) and Valor's arrival 53 (6 frames, guard and
/// invulnerable all through, `P_EX100_BTLF.mset`, Checked (files)), their requests only, on KH1's
/// ground swing as template (its sounds left out: KH2's own play, `kh2slots::tick`).
pub const OURS: [Ours; 41] = [
    Ours { name: "Upper Slash", kh2_row: 12, motion: 161, kind: Kind::Swing, rises: false, guard_break: false, attack: true, next_finisher: false, frames: None, own_sounds: false, combo: false, template: None, valor: false },
    Ours { name: "Finishing Leap", kh2_row: 37, motion: 167, kind: Kind::AirFinisher, rises: true, guard_break: false, attack: true, next_finisher: false, frames: None, own_sounds: false, combo: false, template: None, valor: false },
    Ours { name: "Horizontal Slash", kh2_row: 38, motion: 193, kind: Kind::AirSwing, rises: false, guard_break: false, attack: true, next_finisher: false, frames: None, own_sounds: false, combo: false, template: None, valor: false },
    Ours { name: "Guard Break", kh2_row: 27, motion: 165, kind: Kind::Finisher, rises: false, guard_break: true, attack: true, next_finisher: false, frames: None, own_sounds: false, combo: false, template: None, valor: false },
    Ours { name: "Retaliating Slash", kh2_row: 40, motion: 172, kind: Kind::AirSwing, rises: false, guard_break: false, attack: true, next_finisher: false, frames: None, own_sounds: false, combo: false, template: None, valor: false },
    Ours { name: "Aerial Recovery", kh2_row: NO_ROW, motion: character_core::motion::AIR_RECOVERY, kind: Kind::AirSwing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: false, combo: false, template: None, valor: false },
    Ours { name: "Finishing Plus", kh2_row: NO_ROW, motion: 154, kind: Kind::Finisher, rises: false, guard_break: false, attack: true, next_finisher: true, frames: None, own_sounds: false, combo: false, template: None, valor: false },
    Ours { name: "Finishing Plus (air)", kh2_row: NO_ROW, motion: 183, kind: Kind::AirFinisher, rises: false, guard_break: false, attack: true, next_finisher: true, frames: None, own_sounds: false, combo: false, template: None, valor: false },
    Ours { name: "Guard", kh2_row: GUARD_ROW, motion: character_core::motion::GUARD_PRESS, kind: Kind::Swing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: false, combo: false, template: None, valor: false },
    Ours { name: "Guard repel", kh2_row: NO_ROW, motion: character_core::motion::GUARD, kind: Kind::Swing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: false, combo: false, template: None, valor: false },
    Ours { name: "Counterguard", kh2_row: COUNTERGUARD_ROW, motion: 171, kind: Kind::Swing, rises: false, guard_break: false, attack: true, next_finisher: false, frames: None, own_sounds: false, combo: false, template: None, valor: false },
    Ours { name: "Aerial Dodge", kh2_row: NO_ROW, motion: character_core::motion::AERIAL_DODGE, kind: Kind::AirSwing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: Some(AERIAL_DODGE_FRAMES), own_sounds: true, combo: false, template: None, valor: false },
    Ours { name: "Quick Run", kh2_row: NO_ROW, motion: super::quickrun::RUN_MOTION, kind: Kind::Swing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true, combo: false, template: None, valor: false },
    Ours { name: "Quick Run stop", kh2_row: NO_ROW, motion: super::quickrun::STOP_MOTION, kind: Kind::Swing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true, combo: false, template: None, valor: false },
    Ours { name: "Glide hover", kh2_row: NO_ROW, motion: character_core::motion::GLIDE_HOVER, kind: Kind::AirSwing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true, combo: false, template: None, valor: false },
    Ours { name: "Reflect", kh2_row: NO_ROW, motion: super::reflect::GROUND_MOTION, kind: Kind::Cast, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true, combo: false, template: None, valor: false },
    Ours { name: "Reflect (finisher)", kh2_row: NO_ROW, motion: super::reflect::FINISHER_MOTION, kind: Kind::CastFinisher, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true, combo: false, template: None, valor: false },
    Ours { name: "Reflect (air)", kh2_row: NO_ROW, motion: super::reflect::AIR_MOTION, kind: Kind::AirCast, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true, combo: false, template: None, valor: false },    // Every KH2 combo row of Sora's on a KH1 row of its own (`combo`; the tester, combo row
    // selection after m25, answer 3 (a)). Sora's PTYA (`00battle.bin` moveset 1, Checked (files),
    // `docs/kh1-anim-slots.md` section 7): the ground swings (rows 4..6, 151..153; Slapshot 35,
    // Dodge Slash 34 / 44 / 45, Slide Dash 28 / 43, Flash Step 46 / 47 / 59, Vicinity Break 51),
    // the ground finishers (rows 2 / 3, 154 / 155; Explosion 26 / 42; Guard Break 27 is above),
    // the air swings (rows 9 / 10, 181 / 182, which the ground hops 0 / 1 play too; Aerial Spiral
    // 36; the ground-to-air hops Aerial Sweep 33 and Aerial Dive 48) and the air finishers (rows
    // 7 / 8, 184 / 183; Aerial Finish 29, Magnet Burst 49 / 50).
    combo("151", 151, Kind::Swing),
    Ours { template: Some((11, 0xC9, 99)), ..combo("152", 152, Kind::Swing) },
    combo("153", 153, Kind::Swing),
    combo("Slapshot", 162, Kind::Swing),
    combo("Dodge Slash", 163, Kind::Swing),
    combo("Slide Dash", 164, Kind::Swing),
    combo("Flash Step", 169, Kind::Swing),
    combo("Vicinity Break", 170, Kind::Swing),
    combo("154", 154, Kind::Finisher),
    combo("155", 155, Kind::Finisher),
    combo("Explosion", 166, Kind::Finisher),
    combo("181", 181, Kind::AirSwing),
    combo("182", 182, Kind::AirSwing),
    combo("Aerial Spiral", 192, Kind::AirSwing),
    combo("Aerial Sweep", 191, Kind::AirSwing),
    combo("Aerial Dive", 196, Kind::AirSwing),
    combo("183", 183, Kind::AirFinisher),
    combo("184", 184, Kind::AirFinisher),
    combo("Aerial Finish", 194, Kind::AirFinisher),
    combo("Magnet Burst", 197, Kind::AirFinisher),
    Ours { name: "Drive", kh2_row: NO_ROW, motion: DRIVE_MOTION, kind: Kind::Swing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true, combo: false, template: None, valor: false },
    Ours { name: "Valor arrival", kh2_row: NO_ROW, motion: ARRIVE_MOTION, kind: Kind::Swing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true, combo: false, template: None, valor: true },
    // (A form's short stance after its arrival on the ground, motion 6 of the form's moveset: KH2's
    // new form's first think `0x140405630` plays 53, then 6 with an 8-frame blend, then the idle;
    // Checked (decomp). Built for every form run on KH1's states that has a clip of it:
    // `P_EX100_BTLF` B010 15 frames at 30 fps, `P_EX100_MAGF` C010 20, both free (range id 2) all
    // through; Checked (files).)
    Ours { name: "Form stance", kh2_row: NO_ROW, motion: STANCE_MOTION, kind: Kind::Swing, rises: false, guard_break: false, attack: false, next_finisher: false, frames: None, own_sounds: true, combo: false, template: None, valor: true },
];

/// Sora's Drive motion and a form's arrival (`OURS`; `form::DRIVE_MOTION` / `ARRIVE_MOTION`).
pub const DRIVE_MOTION: u16 = 52;
pub const ARRIVE_MOTION: u16 = 53;
/// A Drive Form's stance after its arrival on the ground (`OURS` "Form stance"; `form::STANCE_MOTION`).
pub const STANCE_MOTION: u16 = 6;

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

/// Our KH1 row for KH2's combo motion `motion` (`Ours::combo`), if any.
pub fn by_combo_motion(motion: u16) -> Option<usize> {
    OURS.iter().position(|o| o.combo && o.motion == motion)
}

/// The KH1 request a script should see for one of our attack rows' requests (`request`), its
/// template's (KH1's own swing, finisher or air swing request): event scripts read Sora's `+0x164`
/// (opcode 322; Dive to the Heart's "Press to attack" window waits for KH1's attack request
/// there), and KH1's Attack step tests it for a spell out of a swing (`combat::command`).
pub fn kh1_request_for(request: u32) -> Option<u32> {
    let n = request.checked_sub(FIRST_REQUEST).map(|n| n as usize)?;
    OURS.get(n).filter(|o| o.attack && !o.kind.cast()).map(|o| o.template_request())
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
/// Each row's clash type (data `+0x38` bits 0..2) and priority (`+0x3C`) under KH1's rules (its
/// template's) and KH2's (`kh2_clash`), packed `type << 16 | priority`, for `rules` (the Recoil
/// row switches between them live).
static CLASH_KH1: [AtomicU32; OURS.len()] = [const { AtomicU32::new(u32::MAX) }; OURS.len()];
static CLASH_KH2: [AtomicU32; OURS.len()] = [const { AtomicU32::new(u32::MAX) }; OURS.len()];
/// Which rules the data holds now: 0 none yet, 1 KH1's, 2 KH2's.
static RULES: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
/// KH1's clash type that meets no attack (no entry of KH1's pairing table `0x140528CB8` has its
/// bit: 1F 09 09 3F 29 18 00 00, Checked (exe)) but still hits bodies (the pass `0x1402B3BE0` keeps
/// bodies away from types 1, 2 and 4 only, Checked (decomp)): KH2's pierce armor.
const PIERCE_TYPE: u32 = 6;

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
    if super::log_on(super::LogCat::Combat) && N.fetch_add(1, Ordering::Relaxed) < 60 {
        log!(Combat, "{}", s());
    }
}

/// Memory for our attack data within an i32 offset of the block (rows hold offsets, not
/// pointers): a page past it, at the first free address from 16 MB on.
pub(super) unsafe fn near(block: usize, size: usize) -> Option<usize> {
    for k in 0..512usize {
        let at = (block + 0x0100_0000 + k * 0x0010_0000) & !0xFFFF;
        let p = super::sys::VirtualAlloc(at as *mut _, size, 0x3000, 0x04) as usize;
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
    let clip = pose::kh2_moves()?.moveset.clips.get(&motion)?;
    // (t680) The right Keyblade's windows (weapon 1), else his body's (0); the second Keyblade's
    // (weapon 2) belong to its own attack object (`bodyhits::atkp`), not to the row's.
    let hits = |weapon_side: bool| {
        clip.windows.iter().filter(move |w| frame.map_or(true, |f| w.contains(f))).find_map(move |w| match w.kind {
            W::Hitbox { attack, weapon, .. } if weapon == weapon_side as u8 => Some(attack),
            _ => None,
        })
    };
    hits(true).or_else(|| hits(false))
}

/// KH2's blow (reaction group 4, Steam KH2 `0x1403CE980`, Checked (decomp)): a launch, KH1's
/// kind 5 with KH2's starting speeds, whatever its numbers (as `combat::Strikes::strike` takes a
/// KH2 hit for KH1's finisher row and KH2's launch; the tester, kh2-knockback-on-enemies: no more
/// "holding" blows).
pub fn throws(a: &kh2_formats::battle::Atkp) -> bool {
    character_core::reaction::category(a.enemy_reaction) == 4
}

/// Each frame base Sora is posed (`kh2slots::frame`, ours built): his character's row table is
/// ours (made the first time from KH1's). Only once our slots are on his request tables, so a row
/// of ours never asks for a request KH1 can't play.
/// The motion request (`+0`, i16; `None` for -1) of row `row` of `actor`'s attack table, the one
/// KH1's row start `0x1402A3980` reads (`0x142D5CAB0[object +0x4C] + row * 0x14`, Steam).
pub unsafe fn row_request(actor: usize, row: usize) -> Option<u32> {
    let base = super::takeover::game_base();
    let obj = pose::ptr(rd::<u32>(actor + 0x130));
    if base == 0 || obj < 0x10000 || !super::crash::readable(obj + 0x4C) {
        return None;
    }
    let index = rd::<u16>(obj + 0x4C);
    if index >= 10 {
        return None;
    }
    let table = rd::<u64>(base + ROWS_RVA + index as usize * 8) as usize;
    let at = table + row * ROW;
    if table < 0x10000 || !super::crash::readable(at + 2) {
        return None;
    }
    let r = rd::<i16>(at);
    (r >= 0).then_some(r as u32)
}

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
            rules();
            return;
        }
        if t.index == index && now == t.kh1 {
            wr(at, t.ours as u64);
            note(|| format!("KH1's attack rows: Sora's table ours again (KH1 set its own, {now:#x})"));
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
    for o in OURS.iter().filter(|o| !o.kind.cast()) {
        let q = rd::<i16>(kh1 + o.template_row() * ROW);
        if q as i32 as u32 != o.template_request() {
            log!(Combat, "KH1's attack rows: Sora's row {} asks for {q:#x}, not {:#x}: no rows of ours", o.template_row(), o.template_request());
            return None;
        }
    }
    let Some(kh2) = pose::kh2() else { return None };
    // (0x80 a row's data: 38 rows take 0x1300.)
    let Some(mem) = near(block, (OURS.len() * 0x80 + 0xFFF) & !0xFFF) else {
        log!(Combat, "KH1's attack rows: no memory within reach of KH1's battle data for our attack data: no rows of ours");
        return None;
    };
    let rows: &'static mut [u8] = Box::leak(vec![0u8; (KH1_ROWS + OURS.len()) * ROW].into_boxed_slice());
    std::ptr::copy_nonoverlapping(kh1 as *const u8, rows.as_mut_ptr(), KH1_ROWS * ROW);
    let mut made = Vec::new();
    for (n, o) in OURS.iter().enumerate() {
        let t = kh1 + o.template_row() * ROW;
        let r = rows.as_mut_ptr() as usize + (KH1_ROWS + n) * ROW;
        std::ptr::copy_nonoverlapping(t as *const u8, r as *mut u8, ROW);
        // Its own data: the template's, with KH2's reaction for the move.
        let src = block.wrapping_add(rd::<i32>(t + 4) as isize as usize);
        let d = mem + n * 0x80;
        std::ptr::copy_nonoverlapping(src as *const u8, d as *mut u8, DATA);
        let atkp = window_atkp(o.motion, None);
        let a = atkp.and_then(|id| kh2.battle.attack_for_level(id, 0, 0));
        // A KH2 Magic-based move (Explosion, Magnet Burst): KH1's own magic stat on its data
        // (`kh1_magic`, Ripple Drive's from KH1's row in this table).
        if a.is_some_and(|a| a.stat_source == 1) {
            kh1_magic(d, block.wrapping_add(rd::<i32>(kh1 + RIPPLE_DRIVE_ROW * ROW + 4) as isize as usize));
        }
        let kind = match a {
            Some(a) if throws(a) => 5u32,
            Some(a) if character_core::reaction::category(a.enemy_reaction) == 3 => 3,
            _ => rd::<u32>(d + 0xC),
        };
        wr(d + 8, kind);
        wr(d + 0xC, kind);
        // (Clash type and priority by the Recoil row, `rules`: KH1's the template's, KH2's from
        // the move's ATKP.)
        let (t1, p1) = (rd::<u32>(d + 0x38) & 7, rd::<i32>(d + 0x3C));
        // (Guard Break pierces at Recoil KH1 too, the way KH1 lets an attack through: a priority of
        // 0x62 or more, which KH1's clash half `0x1402B4BC0` never meets with a guard (100) and
        // never stops (Checked (decomp)). The tester, thread kh2-recoil-and-parry-system m2: "guard
        // break should function like how it does in kh2, it should pierce any normal enemies".)
        let p1 = if o.guard_break { p1.max(GUARD_THROUGH) } else { p1 };
        let (t2, p2) = kh2_clash(a, t1, p1);
        CLASH_KH1[n].store(t1 << 16 | (p1 as u32 & 0xFFFF), Ordering::Relaxed);
        CLASH_KH2[n].store(t2 << 16 | (p2 as u32 & 0xFFFF), Ordering::Relaxed);
        if o.guard_break && t2 != PIERCE_TYPE {
            log!(Combat, "KH1's attack rows: Guard Break's ATKP {atkp:?} is no pierce armor: it keeps its template's clash type under KH2's rules");
        }
        let off = d as isize - block as isize;
        if off < i32::MIN as isize || off > i32::MAX as isize {
            log!(Combat, "KH1's attack rows: our attack data is out of an offset's reach: no rows of ours");
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
            o.template_row(),
            rd::<i32>(d + 0x3C),
            atkp
        ));
    }
    log!(Combat, "KH1's attack rows: Sora's table (character {index}, KH1's at {kh1:#x}) ours with {}", made.join("; "));
    RULES.store(0, Ordering::Relaxed);
    Some(Table { index, kh1, ours: rows.as_ptr() as usize })
}

/// KH2's clash rules for a move's ATKP on KH1's data (the tester, thread
/// kh2-recoil-and-parry-system, plan step 3; `docs/kh1-kh2-recoil.md` §6.3): KH2's pierce armor
/// (kind 1: Guard Break 876, Counterguard 1581, Finishing Leap 854; meets only special guards,
/// never stopped) is KH1's clash type 6, which meets no attack or guard but hits bodies; a normal
/// attack with "refact self" 2 (Horizontal Slash 963, Retaliating Slash 1582: meets attacks and
/// guards but is never stopped) gets KH1's priority past which it isn't stopped (`GUARD_THROUGH`,
/// 0x62: KH1's clash half `0x1402B4BC0` stops only a side under it); the rest keep the template's.
/// KH2's guard kinds (2, 3) are no attack rows. Returns (type, priority).
pub(super) fn kh2_clash(a: Option<&kh2_formats::battle::Atkp>, t: u32, p: i32) -> (u32, i32) {
    match a {
        Some(a) if a.kind == 1 => (PIERCE_TYPE, p),
        Some(a) if a.kind == 0 && a.refact_self == 2 => (t, p.max(GUARD_THROUGH)),
        _ => (t, p),
    }
}

/// Each frame base Sora is posed (`install`): our rows' data hold the Recoil row's rules (KH1's:
/// the templates' clash type and priority, Guard Break's priority at least `GUARD_THROUGH`; KH2's: `kh2_clash`). KH1 reads them
/// when it makes the row's attack object (`0x1402B2DC0`: data `+0x38` bits 0..2 → the object's
/// flags bits 17..19, `+0x3C` → its priority `+0x68`, Checked (decomp)), so a switch takes the
/// next swing.
unsafe fn rules() {
    let want = if super::recoil::kh2() { 2u8 } else { 1 };
    if RULES.load(Ordering::Relaxed) == want {
        return;
    }
    let mut said = Vec::new();
    for n in 0..OURS.len() {
        let d = DATA_AT[n].load(Ordering::Relaxed);
        let v = if want == 2 { &CLASH_KH2[n] } else { &CLASH_KH1[n] }.load(Ordering::Relaxed);
        if d == 0 || v == u32::MAX {
            continue;
        }
        let (t, p) = (v >> 16, (v & 0xFFFF) as u16 as i16 as i32);
        wr(d + 0x38, (rd::<u32>(d + 0x38) & !7) | t);
        wr(d + 0x3C, p);
        if OURS[n].attack {
            said.push(format!("{} type {t} priority {p}", OURS[n].name));
        }
    }
    RULES.store(want, Ordering::Relaxed);
    log!(Combat, "KH1's attack rows: {}'s clash rules: {}", if want == 2 { "KH2" } else { "KH1" }, said.join(", "));
}

/// Sora's own KH1 row `row` (KH1's table, not ours): its attack data's address and its effect and
/// sound bytes (`+0xC..+0xE`, which KH1's row start puts in the attack object's `+0xA8`), for the
/// KH1 attack objects of KH2's hits on his body (`bodyhits`).
pub unsafe fn kh1_row(row: usize) -> Option<(usize, [u8; 3])> {
    let kh1 = TABLE.try_lock().ok()?.as_ref().map(|t| t.kh1)?;
    let base = super::takeover::game_base();
    if kh1 < 0x10000 || base == 0 || row >= KH1_ROWS {
        return None;
    }
    let at = kh1 + row * ROW;
    let data = (base + BLOCK_RVA).wrapping_add(rd::<i32>(at + 4) as isize as usize);
    (data > 0x10000).then(|| (data, rd::<[u8; 3]>(at + 0xC)))
}

/// KH1's Ripple Drive row in Sora's table (`combomods::RIPPLE_DRIVE`): KH1's own Keyblade attack
/// that grows with magic (its data `0x4E88`: stat source 4, max MP × 3, power 10, Checked (files)).
pub const RIPPLE_DRIVE_ROW: usize = 7;

/// KH2's Magic-based hits (ATKP stat source 1: Explosion, Magnet Burst; KH2 spells only without a
/// KH1 spell's data of their own, t920) through KH1's own magic
/// stat (the owner, t917: Magic moves go through how KH1 works out magic damage, no bonus on top).
/// Our data copy `d` takes the damage fields of KH1's Ripple Drive data `ripple`: its stat source
/// (`+0`, which KH1's attack constructor `0x1402B2DC0` turns into the attack's stat term from max
/// MP, Checked (decomp)), its power (`+0x14` vs the player's side, `+0x48` vs enemies) and its
/// min / max (`+0x20` / `+0x24`, `+0x4C` / `+0x50`). KH1's damage function then adds max MP × 3
/// where a swing adds Strength, and KH2's power % applies on KH1's curve (`kh2damage`); nothing is
/// multiplied after. False (the copy left as it is) when Ripple Drive's stat source isn't one of
/// KH1's max MP ones (1, 3, 4, 5).
pub unsafe fn kh1_magic(d: usize, ripple: usize) -> bool {
    static LOGGED: AtomicU32 = AtomicU32::new(0);
    let first = LOGGED.fetch_add(1, Ordering::Relaxed) == 0;
    let src = rd::<u32>(ripple);
    if !matches!(src, 1 | 3 | 4 | 5) {
        if first {
            log!(Combat, "KH2's Magic-based hits: KH1's Ripple Drive data {ripple:#x} has stat source {src}, not a max MP one: left on the swing's Strength");
        }
        return false;
    }
    wr(d, src);
    for o in [0x14usize, 0x20, 0x24, 0x48, 0x4C, 0x50] {
        wr(d + o, rd::<u32>(ripple + o));
    }
    if first {
        log!(Combat,
            "KH2's Magic-based hits: KH1's own magic stat from Ripple Drive's data {ripple:#x}: stat source {src}, power {} vs enemies (min {}, max {})",
            rd::<i32>(ripple + 0x48), rd::<i32>(ripple + 0x4C), rd::<i32>(ripple + 0x50)
        );
    }
    true
}

/// `kh1_magic` with Ripple Drive's data from Sora's KH1 table (`kh1_row`).
pub unsafe fn kh1_magic_row(d: usize) -> bool {
    match kh1_row(RIPPLE_DRIVE_ROW) {
        Some((r, _)) => kh1_magic(d, r),
        None => false,
    }
}

/// KH1's row `row` of our table run (`f`, the row runner) with our row `n` in its place for that
/// call: its request, trail / effect / voice bytes and attack data, KH1's own lunge (`+8`) kept,
/// put back after (KH1's row start `0x1402A3980` reads the row once, at the start of the runner:
/// the attack object holds the data from then on). For KH1's flight rows 19..21
/// (`combomods::flight_pick`), whose runner case only exists for those indices. False (nothing
/// run) when our table isn't the one on Sora's character now.
pub unsafe fn run_as(row: i32, n: usize, f: impl FnOnce()) -> bool {
    let Some((index, ours)) = TABLE.try_lock().ok().and_then(|g| g.as_ref().filter(|t| t.ours != t.kh1).map(|t| (t.index, t.ours))) else {
        return false;
    };
    let base = super::takeover::game_base();
    if base == 0 || n >= OURS.len() || !(0..KH1_ROWS as i32).contains(&row) || rd::<u64>(base + ROWS_RVA + index as usize * 8) as usize != ours {
        return false;
    }
    let at = ours + row as usize * ROW;
    let from = ours + (KH1_ROWS + n) * ROW;
    let was = rd::<[u8; ROW]>(at);
    let mut now = rd::<[u8; ROW]>(from);
    now[8..12].copy_from_slice(&was[8..12]);
    wr(at, now);
    f();
    wr(at, was);
    true
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
    // (One of KH2's hit windows on his body, as a KH1 attack object of its own: that window's.)
    if let Some(a) = super::bodyhits::atkp(attack) {
        return Some(a);
    }
    let data = pose::ptr(rd::<u32>(attack + 0x5C));
    let n = DATA_AT.iter().position(|d| data != 0 && d.load(Ordering::Relaxed) == data)?;
    let kh2 = pose::kh2()?;
    let sora = super::takeover::sora_actor();
    // (The KH2 motion our slot plays: a next-finisher row's is the one KH2 picked.)
    // (t883) From our slot's level event 8.16 by KH1's own reader (`kh2slots::atkp_now`); KH2's
    // clip where that can't be read.
    let now = match super::kh2slots::atkp_now(sora) {
        Some((now, _)) => now,
        None => super::kh2slots::now_playing(sora).and_then(|(m, t)| window_atkp(m, Some(t as u32))),
    };
    let id = now.or_else(|| u16::try_from(ATKP_OF[n].load(Ordering::Relaxed)).ok())?;
    kh2.battle.attack_for_level(id, 0, 0).cloned()
}

/// An attack object of one of our rows whose KH2 hit now is a normal attack with "refact self" 2
/// (Horizontal Slash 963, Retaliating Slash 1582, ...: KH2 meets attacks and guards with it but
/// never stops it; `docs/kh1-kh2-recoil.md` §6.3). `recoil`'s meeting hook makes a guard's owner
/// immune to it at Recoil KH2.
pub unsafe fn unstoppable(attack: usize) -> bool {
    kh2_atkp(attack).is_some_and(|a| a.kind == 0 && a.refact_self == 2)
}

/// The KH2 ATKP for a native swing on one of KH1's own rows (`kh2damage`): the hit window of the
/// KH2 motion our slot plays on `actor` now at KH1's time in it, else that motion's first one;
/// `None` when no slot of ours plays (Base Combo KH1: KH1's own swing, KH1's own damage).
pub unsafe fn native_atkp(actor: usize) -> Option<kh2_formats::battle::Atkp> {
    if actor < 0x10000 {
        return None;
    }
    let (m, t) = super::kh2slots::now_playing(actor)?;
    // (t883) From our slot's level event 8.16 by KH1's own reader (`kh2slots::atkp_now`); KH2's
    // clip where that can't be read.
    let id = match super::kh2slots::atkp_now(actor) {
        Some((now, first)) => now.or(first),
        None => window_atkp(m, Some(t as u32)).or_else(|| window_atkp(m, None)),
    }?;
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

/// KH1's attack pool (Steam `0x142DB4710`, 64 entries of 0xC0; in use `+0` bit `0x800`, owner
/// handle `+0x34`, attack data handle `+0x5C`).
const POOL_STEAM: usize = 0x2DB4710;
const POOL_SLOTS: usize = 0x40;

/// (t-rehit) A new KH2 attack on the Keyblade in our slot (`kh2slots::keyblade_rearm`): the
/// Keyblade attack objects of Sora's rows of ours (their data one of ours, not a body window's,
/// `bodyhits`) re-armed as a new attack (`rehit::new_attack`: KH1's own re-arm), so it can hit
/// what the last one hit, as KH2's new attack does. Returns how many.
pub unsafe fn rearm_rows(sora: usize) -> usize {
    if super::addr::Build::current() != super::addr::Build::Steam || sora < 0x10000 {
        return 0;
    }
    let base = super::takeover::game_base();
    let mut n = 0;
    for k in 0..POOL_SLOTS {
        let a = base + POOL_STEAM + k * 0xC0;
        if rd::<u32>(a) & 0x800 == 0 || pose::ptr(rd::<u32>(a + 0x34)) != sora || super::bodyhits::atkp(a).is_some() {
            continue;
        }
        let data = pose::ptr(rd::<u32>(a + 0x5C));
        if data == 0 || !DATA_AT.iter().any(|d| d.load(Ordering::Relaxed) == data) || rd::<u32>(a + 0x54) == 0 {
            continue;
        }
        super::rehit::new_attack(a);
        n += 1;
    }
    n
}
