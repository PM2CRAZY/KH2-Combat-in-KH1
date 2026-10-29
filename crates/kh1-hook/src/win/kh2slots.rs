//! KH2's motions as new slots in KH1 Sora's own motion table (ini `kh1_slots = on`; the user:
//! "lets have it so that our kh2 animations can run entirely through kh1s animation system as new
//! animation slots"). `docs/kh1-anim-slots.md` has the RE and the plan; this is its phase 2.
//!
//! KH1 plays a request through two tables on Sora's actor: the request table (`+0x1D8`, the
//! mset's section 1: request id → slot and kind) and the slot table (`+0x1D4`: slot → the
//! motion's handle). Both are handles, and `0x14038B080` makes a handle of any pointer, so Sora
//! gets copies of both: the slot table with our motions after KH1's 118, the request table with
//! the KH1 requests a KH2 move stands in for pointed at its slot while that move's style is KH2.
//! KH1's state, its attack row and attack object (tied to the request id, `+0x164`), its combo
//! windows, scripts and our own checks of `+0x164` all see KH1's request; only the motion played
//! is ours.
//!
//! Each of our motions is a KH1 motion (the slot the KH2 move stands in for, as the template)
//! retimed to the KH2 motion's length (`kh1_data::slot::retimed`), with the KH2 clip's hit,
//! combo and trail windows as KH1's motion events: KH1's clock, queue, blend clock, hit window,
//! combo window, trail and its own sounds and foot effects run it. `pose` draws the KH2 motion
//! over it at KH1's time (`actor + 0x16C`), and KH2's own sounds and effects for it fire from
//! here on that time (`tick`).
//!
//! The tables are put on Sora's actor again whenever it is a new one (each zone makes one) and
//! shows KH1's own tables (its 118 slots, its request table as in the file); never over another
//! motion set (a world's own, `+0x1E0`).

use super::addr::Rva;
use super::styles::Kind;
use super::{log, pose};
use character_core::{EventKind, WindowKind};
use kh1_data::slot::{self as ks, EventTrack};
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

const TO_HANDLE_RVA: Rva = Rva(0x38B080);
/// KH1 Sora's own slots (`xa_ex_0010.mset`).
pub const KH1_SLOTS: usize = 118;
/// KH1's request ids (`0x141` in use, the table 0x160 long); ours for KH2's moves on KH1 rows of
/// their own start at `0x200` (`kh1rows::FIRST_REQUEST`).
const KH1_REQUESTS: usize = 0x141;
const REQUESTS: usize = 0x220;
const SLOT_TABLE: usize = 0x1D4;
const SLOT_NOW: usize = 0x168;
const TIME_NOW: usize = 0x16C;
const BLEND_LEN: usize = 0x170;
const BLEND_CLOCK: usize = 0x174;

/// A KH2 motion played as a KH1 slot.
struct Spec {
    motion: u16,
    /// The KH1 slot it stands in for (its template) and KH1's request for that slot.
    stands_for: usize,
    request: usize,
    kind: Kind,
}

// KH1's ground combo, KH2's swings for it as `pose` draws them (`pose::swing_motion`): KH1's
// first and third swing (98, request 0xC8) KH2's first (151); its second (99, 0xC9) KH2's second
// (152; the user: "sora base attack 2, kh2 base attack 2"); slot 100 (0xCA, taken as another
// swing) KH2's first; its finisher (101, 0xCB) KH2's finisher (154; "kh1 base finisher, kh2 base
// finishers"). These are each request's own; KH2's other swings (and these too, for KH2's row
// selection) are on KH1 rows and requests of their own (`kh1rows::Ours::combo`; the tester, combo
// row selection after m25, answer 3 (a)), no longer picked onto these requests.
//
// KH1's air combo (slots 102..104, requests 0xCC..0xCE), KH2's aerial swings for it as `pose`
// draws them: 181 for KH1's first air swing (102), 182 for its second (103), 183 for its finisher
// (104), KH2's own order in the air (181 / 182 alternate, 183 finishes). With these built, the Attack in the air at Aerial Base Combo KH2 stays in KH1's air
// state (`covers`, `shared::kh1_combo`) instead of borrowing him for KH2's own air combo.
// These are each request's own (KH2's air rows on KH1 rows of their own, as above).

/// KH1's spell casts (`docs/kh1-magic-native.md`; KH1's command step, Steam `0x1402A3DB0` /
/// `0x1402A3600` / `0x1402A4150` / `0x1402A7510`, Checked (decomp); KH1's request table,
/// `xa_ex_0010.mset`, Checked (files)): on the ground request `0x36 + spell` (Fire 0x36 slot 54,
/// Blizzard 0x37 slot 55, Thunder 0x38 slot 56, Cure 0x39 slot 56), in the air `0x83 + spell`
/// (79, 80, 81, 81); Fire's second part 0x3D (slot 57; on the ground after 0x36, or alone for a
/// Fire out of a swing or out of 0x3D itself) and 0x8A (slot 82, the air's). With the spell's
/// row at KH2 (or, for KH1's spell, its animation row: `formmagic::anim_kh2`), each plays KH2's
/// cast motion for the spell (MAGC, `00battle.bin`, Checked (files): Fire 56 / air 58, Blizzard
/// 59 / 61, Thunder 62 / 64, Cure 65 / 67; the same motions at every level) in KH1's own cast
/// state (KH2's spell: `formmagic::kh1_spell_start`; KH1's: KH1's own script on it).
///
/// KH1's Gravity, Stop and Aero (no KH2 spell) at their animation rows KH2: KH2's Thunder cast for
/// Gravity, Cure's for Stop and Aero (the motions `formmagic::kh1_spell_motion` and `pose::follow`
/// drew over them before), on KH1's own requests (KH1's request table, `xa_ex_0010.mset`,
/// **Checked (files)**: 0x3A slot 55, 0x3B / 0x3C slot 56; air 0x87 slot 80, 0x88 / 0x89 slot 81),
/// so KH1's command step casts them on KH2's animation with KH1's own script (no borrow).
pub const CASTS: [(u16, usize, usize, bool); 16] = [
    (56, 54, 0x36, false),
    (59, 55, 0x37, false),
    (62, 56, 0x38, false),
    (65, 56, 0x39, false),
    (56, 57, 0x3D, false),
    (58, 79, 0x83, true),
    (61, 80, 0x84, true),
    (64, 81, 0x85, true),
    (67, 81, 0x86, true),
    (58, 82, 0x8A, true),
    // (Gravity, Stop and Aero on the ground: the casts of the KH2 spells whose combo roles they
    // take at Magic Combo KH2 (`formmagic::Role`; the tester, what-is-still-missing after m17:
    // "aero is the same as kh2 cure, gravity is the same as thunder, stop is a finisher like
    // magnet"): Thunder's 62 (its finisher cast 63 picked at the length), Magnet's 68, Cure's 65
    // (MAGC, Checked (files)). At Magic Combo KH1 each plays KH2's Thunder finisher cast 63, as the
    // combo's finisher they are there (`formmagic::finisher_spell`; the tester, combo row selection
    // m30: "make them play the thunder finisher animation"): `CAST_FINISHERS`, picked at the
    // command, `combat::command`.)
    (62, 55, 0x3A, false),
    (68, 56, 0x3B, false),
    (65, 56, 0x3C, false),
    (64, 80, 0x87, true),
    (67, 81, 0x88, true),
    (67, 81, 0x89, true),
];

/// The spell of one of KH1's cast requests (`CASTS`: ground `0x36 + spell`, air `0x83 + spell`,
/// Fire's second parts 0x3D / 0x8A; Fire 0 .. Aero 6), for its rows (`redirect`).
fn cast_spell(request: usize) -> Option<u8> {
    match request {
        0x36..=0x3C => Some((request - 0x36) as u8),
        0x83..=0x89 => Some((request - 0x83) as u8),
        0x3D | 0x8A => Some(0),
        _ => None,
    }
}

/// KH2's finisher casts (MAGC's finisher motions, Checked (files): Fire 57, Blizzard 60, Thunder 63;
/// Cure has none, and none in the air), each on KH1's ground request of its spell (and Fire's
/// second part, 0x3D, which KH1 plays for a Fire out of a swing): (motion, KH1's slot, request).
///
/// Also KH2's Thunder finisher cast 63 on Gravity's, Stop's and Aero's ground requests: Gravity's
/// at the combo's length at Magic Combo KH2 (Thunder's role), all three at Magic Combo KH1 (their
/// finisher rule, t644 / t651).
const CAST_FINISHERS: [(u16, usize, usize); 7] = [(57, 54, 0x36), (60, 55, 0x37), (63, 56, 0x38), (57, 57, 0x3D), (63, 55, 0x3A), (63, 56, 0x3B), (63, 56, 0x3C)];

const fn cast_spec(i: usize) -> Spec {
    let (motion, stands_for, request, air) = CASTS[i];
    Spec { motion, stands_for, request, kind: if air { Kind::AirMagic } else { Kind::Magic } }
}

/// KH2's reaction motions on KH1's reaction requests (the tester, thread
/// soras-hit-reactions-native, plan step 1: "KH2's reaction motions on KH1's reaction requests";
/// `docs/kh1-hit-reactions-native.md` §4, §7). KH1's hit pass (Steam `0x14029BBD0`) still runs
/// the reaction function for Sora's movement state and kind, which sets its motion with KH1's
/// set-motion (`0x14029F980`), its 30-frame no-damage timer (`0x140297F30`), its push and its idle
/// / fall queued after, and the pass the rumble, tally, HUD, MP and voice; only the motion played
/// is ours, by these requests (Checked (decomp)). KH1 picks the request by `dot(facing, the
/// record's push direction +0x10)`: below 0 0x48 (0x49 with the record's `+0x20` = 1) on the
/// ground, 0x4E in the air, else 0x4A (0x4B) / 0x4F; KH2's routine (`0x1403D2FE0`,
/// `character_core::reaction::respond`) the same test for its front motions 10 (11) / 14 and back
/// ones 12 (13) / 15 (Checked (decomp) both; KH2's alternate is by body part, KH1's by `+0x20`,
/// taken as KH1's). KH1's request table (`xa_ex_0010.mset`, Checked (files)): 0x48..0x4B slots
/// 61..64 (70 frames, may be left at 46), 0x4E / 0x4F slots 67 / 68 (30 frames); KH2's
/// (`P_EX100.mset`, Checked (files)): 10..13 70 frames, free from 50 (range id 2: KH1's "may be
/// left" there), 14 / 15 36 frames. KH1's knock-backs (0x4C / 0x4D on the ground, slots 65 / 66,
/// 72 frames in place; 0x50 / 0x51 in the air) are KH2 blows (16 / 18, then 17 / 19 on landing),
/// which need KH2's launch: not here (plan step 3). Sora's request table has no KO request (0x43
/// is 0xFFF in it, Checked (files)): his HP gone is KH1's game over, not the partners' KO handler.
const REACTIONS: [(u16, usize, usize); 6] = [(10, 61, 0x48), (11, 62, 0x49), (12, 63, 0x4A), (13, 64, 0x4B), (14, 67, 0x4E), (15, 68, 0x4F)];

const fn reaction_spec(i: usize) -> Spec {
    let (motion, stands_for, request) = REACTIONS[i];
    Spec { motion, stands_for, request, kind: Kind::HitReactions }
}

/// KH2's dodge roll on KH1's own roll (the tester, thread dodge-roll-guard-and-counterguard-native:
/// "dodge roll its own slot ... use kh1s as a base so its native"; `docs/kh1-roll-guard-native.md`
/// §5). KH1's roll start (Steam `0x1402C9E60`: request `0xDC | 0x1000000`, the idle queued after,
/// its 12-frame no-damage timer) runs at either Dodge Roll style; at KH2 request `0xDC` plays our
/// slot of KH2's motion 209 on KH1's roll slot 117 as template. KH1's request table
/// (`xa_ex_0010.mset`, Checked (files)): `0xDC` is `0xA075`, slot 117, kind 2, travel flag (bit 15)
/// set, so KH1 moves him by the slot's root (H10): baked, KH2's root travel (`travel`). KH1's slot
/// 117 is 38 frames at 60 fps, "may be left" at 30, four floor sounds and foot effects (Checked
/// (files)); ours keeps the floor sounds and foot effects rescaled and is left from KH2's
/// free state (range id 2 at 36; its fall-check event at 34 before, `roll_free`).
const ROLL: Spec = Spec { motion: character_core::motion::DODGE_ROLL, stands_for: 117, request: 0xDC, kind: Kind::DodgeRoll };

/// KH2's glide motions on KH1's glide requests (the tester, kh2-glide-in-kh1s-glide-state: KH2's
/// glide in KH1's glide state, `glide`). KH1's request table (`xa_ex_0010.mset`, Checked (files),
/// `docs/kh1-anim-slots.md` §1.5): 0x71 / 0x72 slots 75 / 76 (gliding slow / fast), 0x73 / 0x74
/// slots 77 / 78 (banking). At Glide KH2 KH1's glide state asks 0x71 for KH2's glide 205 and
/// 0x73 / 0x74 for its banking 206 / 207 (`P_EX100.mset`, Checked (files)); 0x72 also plays 205
/// (KH2 has one glide motion at any speed). The hover 208 is a request of ours (`kh1rows`).
const GLIDES: [(u16, usize, usize); 4] = [(205, 75, 0x71), (205, 76, 0x72), (206, 77, 0x73), (207, 78, 0x74)];

/// A request whose entry in a bound set (Sora's per-world set, not the file's 118 slots) can hold
/// another KH1 slot than its template: the tester's t530 log, "Sora's set ... (his own, bound): 88
/// slots of its own ... not in this set: 0x74", and his trace (request 0x74 drawn as KH1's slot 77):
/// in that set KH1's left bank 0x74 plays slot 77, the same as its right bank 0x73 (the file has
/// 0x74 on slot 78, Checked (files) `xa_ex_0010.mset`: 0x73 `0x204D`, 0x74 `0x204E`). So KH2's 207
/// stands in for 0x74 whichever of KH1's two banking slots the set's entry holds (Checked (trace)).
fn also_stands_for(request: usize) -> Option<usize> {
    match request {
        0x74 => Some(77),
        _ => None,
    }
}

const fn glide_spec(i: usize) -> Spec {
    let (motion, stands_for, request) = GLIDES[i];
    Spec { motion, stands_for, request, kind: Kind::Glide }
}

/// KH2's recoils on KH1's recoil requests (the tester, thread kh2-recoil-and-parry-system, plan
/// step 2; `docs/kh1-kh2-recoil.md` §6.2). KH1's attack-clash reactions (Steam, Checked (decomp)):
/// kind 6 on the ground `0x14029D720` asks for request 0x6E, in the air `0x14029D510` for 0x70
/// (then the fall); kind 7 (KH1's big recoil) `0x14029D5F0` for 0x6F. KH1 Sora's request table
/// (`xa_ex_0010.mset`, Checked (files)): 0x6E slot 72, 0x6F slot 73, 0x70 slot 74. KH2's recoil
/// (`0x1403D4650`, Checked (decomp)): motion 41 on the ground, 43 in the air (`P_EX100.mset`,
/// Checked (files): 30 and 20 frames at 30 fps). KH1's big recoil keeps KH1's own animation (the
/// tester, answer 1: "(b)"). KH1's reaction functions still run (the combo count, the motion queue,
/// the fall after the air one); `recoil` puts KH2's push, hop and protection on them.
pub const RECOILS: [(u16, usize, usize); 2] = [(41, 72, 0x6E), (43, 74, 0x70)];

const fn recoil_spec(i: usize) -> Spec {
    let (motion, stands_for, request) = RECOILS[i];
    Spec { motion, stands_for, request, kind: Kind::Recoil }
}

/// KH2's item use on KH1's item requests (the tester, thread baking-the-rest-of-the-base-animations,
/// plan step 2). KH1's command step (Steam `0x1402A3DB0`, command type 3, Checked (decomp)) asks
/// 0x3E (on himself, or no target) or 0x3F (on someone else) with its action flag (bit 24) and
/// blend 8, queues the idle with bit `0x40000000` (action over) and starts the item's effect as its
/// own task (`0x1402C2760`), not tied to the motion. KH1's request table (`xa_ex_0010.mset`,
/// Checked (files)): 0x3E slot 58 (82 frames, "may be left" at 74, the item sound at 30.8), 0x3F
/// slot 59 (72 frames, 62, the same sound). KH2's item use is motion 9 (`P_EX100.mset`, Checked
/// (files): 50 frames, free from 36 (range id 2), its voice 3 at 0 and sound 305 at 10), for both.
/// With Misc Animations at KH2, KH1 plays it in its own item step: KH1's "may be left" is KH2's free
/// state (`Leave::Kh2OrTemplate`), and its end is KH1's queued idle. KH1's item sound is left out of
/// the slot and KH2's own sound plays (the tester's answer 3, "(b)"); KH2's voice only with KH2's
/// battle quotes on ("kh2 voice plays if the toggle for kh2 voices is on", `kh2sound::event`).
const ITEMS: [(u16, usize, usize); 2] = [(KH2_ITEM_USE, 58, 0x3E), (KH2_ITEM_USE, 59, 0x3F)];
/// KH2 Sora's item-use motion (P_EX100 motion 9, `A110`).
pub const KH2_ITEM_USE: u16 = 9;

/// One of KH1's item requests (`ITEMS`).
fn is_item(request: usize) -> bool {
    ITEMS.iter().any(|i| i.2 == request)
}

const fn item_spec(i: usize) -> Spec {
    let (motion, stands_for, request) = ITEMS[i];
    Spec { motion, stands_for, request, kind: Kind::MiscAnims }
}

/// KH2's fall on KH1's fall requests (the tester, thread baking-the-rest-of-the-base-animations,
/// plan step 3). KH1 asks request 6 (its jump's base 0 + 6) or 0xB (the High Jump's base 5 + 6),
/// both slot 10 in its request table (`xa_ex_0010.mset`, `0x200A`, kind 2, Checked (files)): from
/// the player step when he walks off a ledge (state 2), from the ceiling bonk `0x1402AF3F0`, after
/// its apex (`apex`), and our own (the glide's end, the Aerial Dodge's apex). Neither request has
/// the hold bit, so KH1 loops the slot. KH2's fall is motion 4 (`P_EX100.mset`, Checked (files): 32
/// frames at 60 Hz, looping back to frame 12, its `frame_return` 6 at 30 fps), which KH1 loops from
/// there through its own loop range (`loop_range`). Style Jump. 0xB is pointed at the same slot
/// (`ALIASES`).
const FALL: Spec = Spec { motion: character_core::motion::FALL, stands_for: 10, request: 0x6, kind: Kind::Jump };

/// KH2's jump and High Jump on KH1's takeoff requests (the tester, same thread, plan step 4). KH1's
/// takeoff (`0x1402AF9F0`) asks 0x10004 (its jump) or 0x10009 (with High Jump, ability bit 0x200),
/// bit 16 holding the motion at its end; KH1's request table (`xa_ex_0010.mset`, Checked (files)):
/// 0x4 slot 8 (26 frames, the takeoff's foot effects at 4, its sounds at 2 and 14.5), 0x9 slot 12
/// (34 frames, foot effects at 4, sounds at 4, 8.8 and 27.5). KH2's jump start (Steam KH2
/// `0x140422630` with `0x1403B59B0`, Checked (decomp)) plays motion 3, or 201 (0xC9) with High
/// Jump; `P_EX100.mset` (Checked (files)): 3 is 26 frames at 60 Hz, 201 is 64, each leaving the
/// ground at frame 4 (their frame trigger 0). Both play until KH2's swap into the fall 5 frames
/// before the apex (`kh2_apex`), which replaces KH1's apex request (5 / 0xA): not built. Style
/// Jump. KH1's own sounds and foot effects at KH1's own frames (`make`: the takeoff is frame 4 in
/// both), as KH1 played them under the drawn KH2 jump before.
const JUMPS: [(u16, usize, usize); 2] = [(character_core::motion::JUMP, 8, 0x4), (character_core::motion::HIGH_JUMP, 12, 0x9)];

/// KH2's landing on KH1's landing requests (the tester, same thread, m3: "just make sure every
/// animation is baked and on the native system"; its m1 answer 2 on the landing lock: "same as kh1
/// for now"). KH1's player step asks its jump set's base + 7 with bit 24 (action flag 4: he can't
/// act) on the frame his state goes from the air to the ground, and queues the idle (`0x40000000`,
/// blend 8) (Checked (decomp), `docs/kh1-anim-baking.md` §3): request 7 (plain jump) or 0xC (High
/// Jump), both slot 11 in its request table (`0x200B`, kind 2, no hold bit; 28 frames, "may be
/// left" (type 0) at 12, the landing's foot effect and sound 6 at 0; Checked (files)). KH2's
/// landing is motion 5 (`P_EX100.mset`, Checked (files): 26 frames at 60 Hz, its reaction window
/// from 8, free from 12). KH1's events stay at KH1's own frames (`make`: style Jump, not stretched),
/// so KH1's landing lock is its own "may be left" at 12, KH2's free frame too. 0xC is pointed at the
/// same slot (`ALIASES`).
const LAND: Spec = Spec { motion: character_core::motion::LAND, stands_for: 11, request: 0x7, kind: Kind::Jump };

const fn jump_spec(i: usize) -> Spec {
    let (motion, stands_for, request) = JUMPS[i];
    Spec { motion, stands_for, request, kind: Kind::Jump }
}

/// KH1 requests pointed at another request's slot of ours (alias, request): the High Jump's fall
/// 0xB plays the plain jump's (KH1's table has both on slot 10), its landing 0xC the plain jump's
/// landing 0x7 (both slot 11).
const ALIASES: [(usize, usize); 2] = [(0xB, 0x6), (0xC, 0x7)];

/// KH2's chest-opening motion (t783, `pose::CHEST_MOTION`: the treasure box's capture motion for Sora) on
/// KH1's open request: the Open handler asks for `0x900A6` (request 0xA6, KH1's slot 83, Checked (exe +
/// trace: applied request 0xA6, slot 83)). Pointed at our slot only with Chest Opening on Quick and its
/// Chest Animation row at KH2 (`redirect`, `chest::kh2_animation`), not by a style row.
pub const CHEST_REQUEST: usize = 0xA6;
const CHEST: Spec = Spec { motion: pose::CHEST_MOTION, stands_for: 83, request: CHEST_REQUEST, kind: Kind::MiscAnims };

const SPECS: [Spec; 43] = [
    Spec { motion: 152, stands_for: 99, request: 0xC9, kind: Kind::GroundCombo },
    Spec { motion: 151, stands_for: 98, request: 0xC8, kind: Kind::GroundCombo },
    Spec { motion: 151, stands_for: 100, request: 0xCA, kind: Kind::GroundCombo },
    Spec { motion: 154, stands_for: 101, request: 0xCB, kind: Kind::GroundCombo },
    Spec { motion: 181, stands_for: 102, request: 0xCC, kind: Kind::AirCombo },
    Spec { motion: 182, stands_for: 103, request: 0xCD, kind: Kind::AirCombo },
    Spec { motion: 183, stands_for: 104, request: 0xCE, kind: Kind::AirCombo },
    cast_spec(0),
    cast_spec(1),
    cast_spec(2),
    cast_spec(3),
    cast_spec(4),
    cast_spec(5),
    cast_spec(6),
    cast_spec(7),
    cast_spec(8),
    cast_spec(9),
    cast_spec(10),
    cast_spec(11),
    cast_spec(12),
    cast_spec(13),
    cast_spec(14),
    cast_spec(15),
    reaction_spec(0),
    reaction_spec(1),
    reaction_spec(2),
    reaction_spec(3),
    reaction_spec(4),
    reaction_spec(5),
    ROLL,
    glide_spec(0),
    glide_spec(1),
    glide_spec(2),
    glide_spec(3),
    recoil_spec(0),
    recoil_spec(1),
    item_spec(0),
    item_spec(1),
    FALL,
    jump_spec(0),
    jump_spec(1),
    LAND,
    CHEST,
];

/// Valor Form's motions on KH1's own requests (Valor on KH1's own states, `form::native`; the
/// tester, what-is-still-missing after m11: "I want you to transfer drive forms as well"), each
/// the same KH1 request and template as base Sora's move of that kind, from `P_EX100_BTLF.mset`
/// (Checked (files): its plain combo 151..155 / 181..184, reactions 10..15, recoils 41 / 43, item
/// use 9, fall 4, landing 5, High Jump 201). Played on their requests only while he is in Valor
/// (`redirect`), whatever the move styles (a Drive Form is KH2's all through, as on KH2's
/// controller before). Its jump: KH2's jump start plays 201 with High Jump (Steam KH2
/// `0x140422630` / `0x1403B59B0`, Checked (decomp), `JUMPS`), which Valor always has (its growth
/// ability at MAX, `form::form_abilities`), so KH1's jump and High Jump takeoffs (0x4 / 0x9) both
/// play it.
const VALOR_SPECS: [Spec; 21] = [
    Spec { motion: 152, stands_for: 99, request: 0xC9, kind: Kind::GroundCombo },
    Spec { motion: 151, stands_for: 98, request: 0xC8, kind: Kind::GroundCombo },
    Spec { motion: 151, stands_for: 100, request: 0xCA, kind: Kind::GroundCombo },
    Spec { motion: 154, stands_for: 101, request: 0xCB, kind: Kind::GroundCombo },
    Spec { motion: 181, stands_for: 102, request: 0xCC, kind: Kind::AirCombo },
    Spec { motion: 182, stands_for: 103, request: 0xCD, kind: Kind::AirCombo },
    Spec { motion: 183, stands_for: 104, request: 0xCE, kind: Kind::AirCombo },
    reaction_spec(0),
    reaction_spec(1),
    reaction_spec(2),
    reaction_spec(3),
    reaction_spec(4),
    reaction_spec(5),
    recoil_spec(0),
    recoil_spec(1),
    item_spec(0),
    item_spec(1),
    FALL,
    Spec { motion: character_core::motion::HIGH_JUMP, stands_for: 8, request: 0x4, kind: Kind::Jump },
    Spec { motion: character_core::motion::HIGH_JUMP, stands_for: 12, request: 0x9, kind: Kind::Jump },
    LAND,
];

/// A Drive Form's moves on KH1's own states (`form::Kind::native_capable`): one table for every
/// form run natively, so Valor, Wisdom, ... are built, pointed at and played by the same code
/// (`prepare`, `redirect`, `clip_of`, `live`, `form_built`). Each form's slots are its own
/// moveset's motions (`pose::Kh2::form_src`), on the same KH1 requests and templates as base
/// Sora's move of that kind (`specs`), its idle / walk / run on KH1's cycles (`CYCLES`, its battle
/// variants on every slot: no form has field variants), its plain combo rows on our combo rows
/// (`kh1rows::Ours::combo`), the form-only rows of ours (`kh1rows::Ours::valor`: its arrival), and
/// `ours`: other rows of ours whose motion the form has its own clip of.
pub struct FormSlots {
    pub kind: super::form::Kind,
    specs: &'static [Spec],
    ours: &'static [u16],
}

pub const FORM_SLOTS: [FormSlots; 2] = [
    FormSlots { kind: super::form::Kind::Valor, specs: &VALOR_SPECS, ours: &[] },
    // (Wisdom's Quick Run, its growth ability: its own 203 and its stop 204 on our Quick Run rows,
    // `quickrun`; its own Reflect 71 / 72 / 73 on our Reflect rows, `reflect::native_path`.)
    FormSlots {
        kind: super::form::Kind::Wisdom,
        specs: &WISDOM_SPECS,
        ours: &[super::quickrun::RUN_MOTION, super::quickrun::STOP_MOTION, super::reflect::GROUND_MOTION, super::reflect::FINISHER_MOTION, super::reflect::AIR_MOTION],
    },
];

/// Wisdom Form's motions on KH1's own requests (`FORM_SLOTS`; the tester, what-is-still-missing
/// after m13: "finish valor and wisdom form"), each the same KH1 request and template as base
/// Sora's move of that kind, from `P_EX100_MAGF.mset` (moveset 3, Checked (files)): its PTYA
/// (Checked (files), `00battle.bin`) has row 5 (151, its shot) and row 3 (154, the finisher) on
/// the ground and rows 1 / 9 / 40 (181, the air shot) in the air, no finisher in the air; its
/// reactions 10..15, item use 9, jump 3, fall 4, landing 5. No recoils 41 / 43 and no High Jump
/// 201 in its file: KH2's jump start plays 3 without High Jump (Steam KH2 `0x140422630` /
/// `0x1403B59B0`, Checked (decomp) at t627), and Wisdom's growth is Quick Run, so both of KH1's
/// takeoffs (0x4 / 0x9) play 3.
const WISDOM_SPECS: [Spec; 35] = [
    Spec { motion: 151, stands_for: 99, request: 0xC9, kind: Kind::GroundCombo },
    Spec { motion: 151, stands_for: 98, request: 0xC8, kind: Kind::GroundCombo },
    Spec { motion: 151, stands_for: 100, request: 0xCA, kind: Kind::GroundCombo },
    Spec { motion: 154, stands_for: 101, request: 0xCB, kind: Kind::GroundCombo },
    Spec { motion: 181, stands_for: 102, request: 0xCC, kind: Kind::AirCombo },
    Spec { motion: 181, stands_for: 103, request: 0xCD, kind: Kind::AirCombo },
    Spec { motion: 181, stands_for: 104, request: 0xCE, kind: Kind::AirCombo },
    reaction_spec(0),
    reaction_spec(1),
    reaction_spec(2),
    reaction_spec(3),
    reaction_spec(4),
    reaction_spec(5),
    item_spec(0),
    item_spec(1),
    FALL,
    jump_spec(0),
    Spec { motion: character_core::motion::JUMP, stands_for: 12, request: 0x9, kind: Kind::Jump },
    LAND,
    // (Its casts on KH1's cast requests, as base Sora's `CASTS`: its own clips of the same MAGC
    // motions, Checked (files): 56..73 in `P_EX100_MAGF.mset`; its finisher casts 57 / 60 / 63
    // are built after, `CAST_FINISHERS`.)
    cast_spec(0),
    cast_spec(1),
    cast_spec(2),
    cast_spec(3),
    cast_spec(4),
    cast_spec(5),
    cast_spec(6),
    cast_spec(7),
    cast_spec(8),
    cast_spec(9),
    cast_spec(10),
    cast_spec(11),
    cast_spec(12),
    cast_spec(13),
    cast_spec(14),
    cast_spec(15),
];

/// KH2's walk and run cycles as slots (Mr Bugs: "can you also bake the animations for kh2s walking
/// and running cycles? its more easy for me to compare"). KH1's walk and run requests (1, 2) are
/// kind 3 in its request table: KH1 plays the entry's slot + 0, 1 or 2 (field, battle, the third
/// variant; `0x14029E050`, `docs/kh1-anim-slots.md` §1.4), so each is built as three slots in a
/// row, on KH1's three as templates (walk 2 / 3 / 4, run 5 / 6 / 7), with KH2's field cycle on the
/// field one and its battle cycle on the other two, as `pose::play_for` draws them; the request is
/// pointed at the first. All three or none.
///
/// KH1's idle, request 0 (the baking thread, m3: "just make sure every animation is baked and on
/// the native system"), is kind 4 in its table (`0x4000`, Checked (files) `xa_ex_0010.mset`): KH1
/// plays the entry's slot in the field and slot + 1 in battle (the resolver `0x1402A01E0`, Checked
/// (decomp), `docs/kh1-anim-baking.md` §2), slots 0 (field idle, 120 frames) and 1 (battle idle,
/// 170). So two slots in a row: KH2's field idle (`P_EX100.mset` motion 0's field variant, 180
/// frames at 60 Hz) on 0, its battle idle (300 frames, Checked (files)) on 1, as `pose::play_for`
/// drew them. Both or none.
struct Cycle {
    motion: u16,
    request: usize,
    /// KH1's first slot of the run.
    slot: usize,
    /// How many slots in a row (kind 3: three; kind 4: two).
    count: u8,
}

const CYCLES: [Cycle; 3] = [
    Cycle { motion: character_core::motion::IDLE, request: 0x0, slot: 0, count: 2 },
    Cycle { motion: character_core::motion::WALK, request: 0x1, slot: 2, count: 3 },
    Cycle { motion: character_core::motion::RUN, request: 0x2, slot: 5, count: 3 },
];

/// A request table entry's kind (bits 12..14): 3 picks the slot + 0, 1 or 2 by field / battle; 4
/// the slot, or slot + 1 in battle.
const KIND_VARIANTS: u32 = 3;
const KIND_BATTLE: u32 = 4;

pub struct Move {
    pub motion: u16,
    pub stands_for: usize,
    pub request: usize,
    pub kind: Kind,
    /// Its slot in KH1's table.
    pub slot: usize,
    pub frames: u32,
    /// Baked into a KH1 motion (`kh1_bake`, `kh1_data::bake`): KH1 shows it, nothing of ours drawn.
    pub baked: bool,
    /// Baked with KH2's root travel in its H10 (`bake_one`): KH1 moves him by it, nothing of ours.
    pub travel: bool,
    /// The request's own (`SPECS`); else one of the swings KH2's row selection picks (`pick`).
    pub default: bool,
    /// KH2's field (out-of-combat) variant of the motion (a `CYCLES` field slot).
    pub field: bool,
    /// Which of a kind 3 / 4 request's slots this is (`CYCLES`; 0 otherwise): only the first is
    /// pointed at, KH1 adds the rest.
    pub variant: u8,
    /// A request of ours (`kh1rows`: 0x200 + n, none of KH1's), not one KH1's own: its entry is
    /// ours in every set, whatever the styles, with the kind bits of KH1's request here (its
    /// template's, whose slot `stands_for` is).
    pub own: Option<usize>,
    /// Off Steam only (`tick`): the KH2 clip's sounds, voices and effects, by frame. On Steam they
    /// are sound events of the motion's own (`kh2_trigger_keys`), which KH1's motion pass fires.
    events: Vec<(u32, EventKind)>,
    /// Where KH2 loops it back to (60 Hz frames from its start), when KH1 loops it on a request
    /// with no hold (`loop_range`; the fall, `FALL`).
    loop_from: Option<f32>,
    /// A Drive Form's motion (Valor's `P_EX100_BTLF`, ...), played on its request only while Sora
    /// is in that form on KH1's own states (`form::native_form`, `redirect`); `None`: base Sora's.
    pub form: Option<super::form::Kind>,
    /// A two-Keyblade form's move (Valor's): its holder motion was baked (`holder_one`), the left
    /// hand's weapon bones for KH1's own pass to pose (`leftblade::pose_holder`).
    pub holder: bool,
    /// That holder motion's block until `place` copies it into KH1's memory.
    holder_block: Mutex<Option<crate::slotcache::Block>>,
}

struct Built {
    moves: Vec<Move>,
    /// Our motions (file layout), one per move, until `place` has copied them where KH1 plays
    /// them from: it takes them then (and they are freed), so the baked motions aren't kept twice
    /// (the tester, ram-use m1 answer 2 (a)). Shared with the slot cache's list while loading
    /// (`slotcache::Block`), not copied.
    blocks: Mutex<Vec<crate::slotcache::Block>>,
    /// KH1's request table as in the file (to know KH1's own when the actor shows it).
    requests: Vec<u32>,
}

static BUILT: OnceLock<Option<Built>> = OnceLock::new();

/// Builds our motions (the loader thread, `pose::load_kh2`): `mset` is KH1 Sora's file; with
/// `bake` (`kh1_bake`), each KH2 motion baked into its KH1 motion, else the template retimed.
pub fn prepare(mset: &[u8], kh2: &pose::Kh2, bake: Option<&pose::Baker>, mut cache: Option<&mut crate::slotcache::Cache>) {
    let Some(which) = super::settings::get().kh1_slots.clone() else {
        let _ = BUILT.set(None);
        return;
    };
    let r = (|| -> Result<Built, String> {
        let requests = ks::request_table(mset).map_err(|e| e.to_string())?;
        if requests.len() < KH1_REQUESTS {
            return Err(format!("KH1's request table has {} entries", requests.len()));
        }
        let mut moves = Vec::new();
        let mut blocks = Vec::new();
        let base = kh2.src();
        let wanted = |motion: u16| which.as_ref().map_or(true, |w| w.contains(&motion));
        // The combo rows' motions KH1's rows can play natively (`swing_rows`: the plain ones, and
        // the ability rows that are a Keyblade swing or hit with his body).
        let combo_ok: Vec<u16> = [false, true].iter().flat_map(|&air| swing_rows(&kh2.character.moveset.combo_rows, &kh2.character.moveset.clips, &kh2.battle, air)).map(|(m, _)| m).collect();
        // (Each row of ours' motions: its own, and with Finishing Plus's next finisher every KH2
        // finisher of its kind that has a row of its own (the ground 154, 155, Explosion, Guard
        // Break; the air 183, 184, Aerial Finish, Magnet Burst), its first its own motion. KH2 may
        // pick the finisher playing again (a row whose `sub` is its own id), and its own row's
        // request is then the one playing: this row plays it, so a next finisher always has a row
        // of ours (the tester, combo row selection after m26, answer 1: no fallback onto KH1's
        // finisher row).)
        let ours_motions = |o: &super::kh1rows::Ours| -> Vec<u16> {
            // (A Valor-only move, its arrival: built from Valor's moveset only, below.)
            if o.valor || !wanted(o.motion) || (o.combo && !combo_ok.contains(&o.motion)) {
                return Vec::new();
            }
            let mut motions = vec![o.motion];
            if o.next_finisher {
                let air = o.kind.air();
                for f in super::kh1rows::OURS.iter().filter(|f| f.attack && f.kind.finisher() && !f.kind.cast() && f.kind.air() == air && !f.next_finisher && !f.rises) {
                    if wanted(f.motion) && (!f.combo || combo_ok.contains(&f.motion)) && !motions.contains(&f.motion) {
                        motions.push(f.motion);
                    }
                }
            }
            motions
        };
        // Drive Forms on KH1's own states (`form::native`; the tester, what-is-still-missing after
        // m11: "I want you to transfer drive forms as well"; after m13: "if something we've
        // implemented is the same way for drive forms make it a part of that code"): each form of
        // `FORM_SLOTS`, its own moveset (`pose::Kh2::form_src`) as slots of their own, on the same
        // KH1 requests and our own requests as base Sora's, played while he is in that form
        // (`redirect`).
        let forms: Vec<(&FormSlots, pose::Src)> = if super::form::native_enabled() {
            FORM_SLOTS.iter().filter(|f| f.kind.native_capable()).filter_map(|f| Some((f, kh2.form_src(f.kind)?))).collect()
        } else {
            Vec::new()
        };
        let form_has = |v: &pose::Src, m: u16| v.character.moveset.clips.contains_key(&m) && v.rig.motions.contains_key(&m);
        // (Each form's plain combo rows' motions: its PTYA, e.g. Valor's moveset 2, Checked (files):
        // 151..155 on the ground, 181..184 in the air, no ability rows of its own.)
        let form_combo = |v: &pose::Src| -> Vec<u16> { v.character.moveset.combo_rows.iter().filter(|r| r.ability == 0).map(|r| r.motion).collect() };
        let form_ours = |f: &FormSlots, v: &pose::Src, o: &super::kh1rows::Ours| -> Vec<u16> {
            if !wanted(o.motion) {
                return Vec::new();
            }
            let combo = form_combo(v);
            if o.valor || f.ours.contains(&o.motion) {
                return if form_has(v, o.motion) { vec![o.motion] } else { Vec::new() };
            }
            if o.next_finisher {
                let air = o.kind.air();
                let mut motions: Vec<u16> = super::kh1rows::OURS.iter().filter(|x| x.combo && x.kind.finisher() && x.kind.air() == air && combo.contains(&x.motion) && form_has(v, x.motion) && wanted(x.motion)).map(|x| x.motion).collect();
                // (Its first is its own motion, as base Sora's, `ours_motions`.)
                motions.sort_by_key(|&m| m != o.motion);
                motions.dedup();
                return if motions.first() == Some(&o.motion) { motions } else { Vec::new() };
            }
            if o.combo && combo.contains(&o.motion) && form_has(v, o.motion) {
                return vec![o.motion];
            }
            Vec::new()
        };
        // (How many motions this makes, for the loading screen's count, `loadscreen`.)
        super::loadscreen::plan(
            SPECS.iter().filter(|s| wanted(s.motion)).count()
                + CYCLES.iter().filter(|c| wanted(c.motion)).map(|c| c.count as usize).sum::<usize>()
                + CAST_FINISHERS.iter().filter(|f| wanted(f.0)).count()
                + super::kh1rows::OURS.iter().map(|o| ours_motions(o).len()).sum::<usize>()
                + forms
                    .iter()
                    .map(|(f, v)| {
                        f.specs.iter().filter(|s| wanted(s.motion) && form_has(v, s.motion)).count()
                            + CYCLES.iter().filter(|c| wanted(c.motion) && form_has(v, c.motion)).map(|c| c.count as usize).sum::<usize>()
                            + super::kh1rows::OURS.iter().map(|o| form_ours(f, v, o).len()).sum::<usize>()
                    })
                    .sum::<usize>(),
        );
        for s in SPECS.iter().filter(|s| wanted(s.motion)) {
            if let Some((m, block)) = make(mset, &base, bake, cache.as_deref_mut(), &requests, (s.motion, s.stands_for, s.request, s.kind), true, (false, 0), KH1_SLOTS + moves.len(), false)? {
                moves.push(m);
                blocks.push(block);
            }
        }
        // KH2's idle (two slots in a row), walk and run (three each) (`CYCLES`).
        for c in CYCLES.iter().filter(|c| wanted(c.motion)) {
            let mut three = Vec::new();
            for v in 0..c.count {
                match make(mset, &base, bake, cache.as_deref_mut(), &requests, (c.motion, c.slot + v as usize, c.request, Kind::MiscAnims), true, (v == 0, v), KH1_SLOTS + moves.len() + v as usize, false)? {
                    Some(m) => three.push(m),
                    None => break,
                }
            }
            if three.len() == c.count as usize {
                for (m, block) in three {
                    moves.push(m);
                    blocks.push(block);
                }
            } else {
                log!(Animation, "KH2 slots: KH2 motion {} left out: not all {} of KH1's slots {}..{} built", c.motion, c.count, c.slot, c.slot + c.count as usize - 1);
            }
        }
        // (KH2's other swings are no longer built on KH1's own ground and air requests for KH2's
        // row selection to point them at: every KH2 combo row has a KH1 row and request of its
        // own, below (`kh1rows::Ours::combo`; the tester, combo row selection after m25, answer
        // 3 (a)). KH1's requests keep only their own move above, `SPECS`.)
        // KH2's finisher casts (`CAST_FINISHERS`) on KH1's ground cast requests, picked at a cast
        // that is the combo's finisher (`pick_cast`).
        for &(motion, stands_for, request) in CAST_FINISHERS.iter() {
            if !wanted(motion) || !moves.iter().any(|m| m.request == request && m.default) {
                continue;
            }
            if let Some((m, block)) = make(mset, &base, bake, cache.as_deref_mut(), &requests, (motion, stands_for, request, Kind::Magic), false, (false, 0), KH1_SLOTS + moves.len(), false)? {
                moves.push(m);
                blocks.push(block);
            }
        }
        // KH2's moves on KH1 rows of their own (`kh1rows`): each on its own request (0x200 + n),
        // built on the KH1 motion and request of its row's kind (the template's travel flag too).
        for (n, o) in super::kh1rows::OURS.iter().enumerate() {
            let motions = ours_motions(o);
            if motions.is_empty() {
                continue;
            }
            let k = o.kind;
            // (A cast of ours, Reflect's, `kh1rows::Kind::cast`: built as KH1's cast slots are.)
            let style = match (k.cast(), k.air()) {
                (true, true) => Kind::AirMagic,
                (true, false) => Kind::Magic,
                (false, true) => Kind::AirCombo,
                (false, false) => Kind::GroundCombo,
            };
            for (j, &motion) in motions.iter().enumerate() {
                if let Some((mut m, block)) = make(mset, &base, bake, cache.as_deref_mut(), &requests, (motion, o.template_slot(), o.template_request() as usize, style), j == 0, (false, 0), KH1_SLOTS + moves.len(), true)? {
                    m.own = Some(m.request);
                    m.request = super::kh1rows::request(n) as usize;
                    log!(Animation, "KH2 slots: KH2 motion {motion} ({}) on our request {:#x} (KH1's row {} of ours)", o.name, m.request, super::kh1rows::row(n));
                    moves.push(m);
                    blocks.push(block);
                }
            }
        }
        // Each Drive Form's own (`FORM_SLOTS`: its specs, its idle / walk / run, its combo rows and
        // form-only rows on our own requests), slots of their own after base Sora's.
        for (f, v) in forms.iter() {
            let before = moves.len();
            for s in f.specs.iter().filter(|s| wanted(s.motion) && form_has(v, s.motion)) {
                if let Some((m, block)) = make(mset, v, bake, cache.as_deref_mut(), &requests, (s.motion, s.stands_for, s.request, s.kind), true, (false, 0), KH1_SLOTS + moves.len(), false)? {
                    moves.push(m);
                    blocks.push(block);
                }
            }
            // (Its finisher casts on its cast requests, as base Sora's below, where it built the
            // request's own cast.)
            for &(motion, stands_for, request) in CAST_FINISHERS.iter() {
                if !wanted(motion) || !form_has(v, motion) || !moves[before..].iter().any(|m| m.request == request && m.default) {
                    continue;
                }
                if let Some((m, block)) = make(mset, v, bake, cache.as_deref_mut(), &requests, (motion, stands_for, request, Kind::Magic), false, (false, 0), KH1_SLOTS + moves.len(), false)? {
                    moves.push(m);
                    blocks.push(block);
                }
            }
            for c in CYCLES.iter().filter(|c| wanted(c.motion) && form_has(v, c.motion)) {
                let mut all = Vec::new();
                for vr in 0..c.count {
                    // (No form has field variants of its idle, walk or run, Checked (files) for
                    // Valor and Wisdom: its battle ones on each of KH1's slots.)
                    match make(mset, v, bake, cache.as_deref_mut(), &requests, (c.motion, c.slot + vr as usize, c.request, Kind::MiscAnims), true, (false, vr), KH1_SLOTS + moves.len() + vr as usize, false)? {
                        Some(m) => all.push(m),
                        None => break,
                    }
                }
                if all.len() == c.count as usize {
                    for (m, block) in all {
                        moves.push(m);
                        blocks.push(block);
                    }
                }
            }
            for (n, o) in super::kh1rows::OURS.iter().enumerate() {
                // (A row of ours that isn't a swing, Quick Run's, keeps the style base Sora's has.)
                let style = if o.combo || o.valor || o.next_finisher {
                    if o.kind.air() { Kind::AirCombo } else { Kind::GroundCombo }
                } else {
                    match (o.kind.cast(), o.kind.air()) {
                        (true, true) => Kind::AirMagic,
                        (true, false) => Kind::Magic,
                        (false, true) => Kind::AirCombo,
                        (false, false) => Kind::GroundCombo,
                    }
                };
                for (j, &motion) in form_ours(f, v, o).iter().enumerate() {
                    if let Some((mut m, block)) = make(mset, v, bake, cache.as_deref_mut(), &requests, (motion, o.template_slot(), o.template_request() as usize, style), j == 0, (false, 0), KH1_SLOTS + moves.len(), true)? {
                        m.own = Some(m.request);
                        m.request = super::kh1rows::request(n) as usize;
                        moves.push(m);
                        blocks.push(block);
                    }
                }
            }
            log!(Animation,
                "KH2 slots: {}'s moves on KH1's states: {} slots ({})",
                f.kind.name(),
                moves.len() - before,
                moves[before..].iter().map(|m| format!("{} on {:#x}", m.motion, m.request)).collect::<Vec<_>>().join(", ")
            );
        }
        log!(Animation,
            "KH2 slots: KH2's row selection on KH1's ground and air combo: {}",
            [0xC8usize, 0xC9, 0xCA, 0xCB, 0xCC, 0xCD, 0xCE].iter().map(|r| format!("{r:#x} -> {:?}", moves.iter().filter(|m| m.request == *r).map(|m| m.motion).collect::<Vec<_>>())).collect::<Vec<_>>().join(", ")
        );
        Ok(Built { moves, blocks: Mutex::new(blocks), requests })
    })();
    match r {
        Ok(b) => {
            let _ = BUILT.set(Some(b));
        }
        Err(e) => {
            log!(Animation, "KH2 slots: not built ({e}); KH2's moves are drawn as before");
            let _ = BUILT.set(None);
        }
    }
}

/// One of our motions: KH2's `motion` on KH1's slot `stands_for` (its template) for KH1's
/// `request`, as our slot `slot`; `default`: the request's own (`SPECS`), else one KH2's row
/// selection can pick (`pick`); `field`: KH2's field variant of the motion; `variant`: which of a
/// kind 3 / 4 request's slots (`CYCLES`). `None` if it can't be built (logged).
#[allow(clippy::too_many_arguments)]
fn make(mset: &[u8], src: &pose::Src, bake: Option<&pose::Baker>, cache: Option<&mut crate::slotcache::Cache>, requests: &[u32], (motion, stands_for, request, kind): (u16, usize, usize, Kind), default: bool, (field, variant): (bool, u8), slot: usize, own: bool) -> Result<Option<(Move, crate::slotcache::Block)>, String> {
    super::loadscreen::making(motion);
    let Some(clip) = src.character.moveset.clips.get(&motion) else {
        log!(Animation, "KH2 slots: KH2 motion {motion} has no clip, left out");
        return Ok(None);
    };
    let entry = requests.get(request).copied();
    let base = stands_for.checked_sub(variant as usize);
    let kind_ok = |k: u32| (k == KIND_VARIANTS && variant <= 2) || (k == KIND_BATTLE && variant <= 1);
    if entry.map(|e| (e & 0xFFF) as usize) != base || (variant > 0 && !entry.is_some_and(|e| kind_ok((e >> 12) & 7))) {
        // (Left out, not a failure of the rest: one wrong request shouldn't drop the others.)
        log!(Animation, "KH2 slots: request {request:#x} is not KH1's slot {stands_for}; KH2 motion {motion} left out");
        return Ok(None);
    }
    let template = ks::motion_bytes(mset, stands_for).map_err(|e| e.to_string())?;
    let old = u32::from_le_bytes(template[4..8].try_into().unwrap());
    // (A field variant's own length: KH2's field walk and run are their own motions.)
    let field_len = field.then(|| src.rig.field_motions.get(&motion)).flatten().map(|m| kh2_anim::length_frames(m).round() as u32);
    // (A move of ours with a slot longer than its clip, `kh1rows::Ours::frames`: the Aerial
    // Dodge, its clip looped over it as KH2 loops it.)
    let ours = if own { super::kh1rows::by_motion(motion).map(|n| &super::kh1rows::OURS[n]) } else { None };
    let frames = ours.and_then(|o| o.frames).filter(|&f| f > clip.length).or(field_len).unwrap_or(clip.length).max(1);
    let cast = matches!(kind, Kind::Magic | Kind::AirMagic);
    // (KH2's finisher casts end the combo: `cast_follow`.)
    // (Ours too: Reflect's finisher cast, `kh1rows::Kind::CastFinisher`.)
    // (KH1's Gravity, Stop and Aero: their own cast's windows from its clip (Thunder's 62, Magnet's
    // 68, Cure's 65), their Thunder finisher cast 63 as a finisher cast, `CAST_FINISHERS`.)
    let finisher_cast = cast && ((!default && CAST_FINISHERS.iter().any(|f| f.0 == motion && f.2 == request)) || ours.is_some_and(|o| o.kind.finisher()));
    // (A hit reaction, `REACTIONS`: KH2's free state as KH1's "may be left", as on our own requests.)
    // (A recoil, `RECOILS`, the same: KH1's reaction function asked for it.)
    let reaction = matches!(kind, Kind::HitReactions | Kind::Recoil);
    // (An item's use, `ITEMS`: KH2's free state as KH1's "may be left", and only KH2's own sound.)
    let item = is_item(request) && !own;
    // (Where KH1 may leave it, `Leave`: KH2's free state only for KH2's swings, ability moves and
    // casts; KH2's, else the template's, for the reactions.)
    let leave = if own || cast || matches!(kind, Kind::GroundCombo | Kind::AirCombo) {
        Leave::Kh2
    } else if reaction || item {
        Leave::Kh2OrTemplate
    } else {
        Leave::Template
    };
    // (The jump's takeoffs, `JUMPS`: the template's sounds and foot effects at KH1's own frames, the
    // takeoff being frame 4 in both games; not stretched over KH2's longer High Jump.)
    let r = if kind == Kind::Jump { 1.0 } else { frames as f32 / old.max(1) as f32 };
    let mut tracks = kh1_tracks(&ks::events(&template).map_err(|e| e.to_string())?, clip, r, leave);
    // (Only KH2's own sounds for it: the template's left out, `kh1rows::Ours::own_sounds`.)
    // (KH2's chest motion, t802: only KH2's own sounds, its Keyblade's 500047 at frame 1 and 500034 at 28; the
    // owner: "the sound effect from the keyblade needs to be the one from kh2".)
    if ours.is_some_and(|o| o.own_sounds) || item || motion == pose::CHEST_MOTION {
        tracks.retain(|t| t.kind != ks::SOUND);
    }
    // (The Aerial Dodge: KH1's "may be left" one frame before KH2's free frame, `INPUT_LEAD`.)
    if own && motion == character_core::motion::AERIAL_DODGE {
        if let Some(f) = clip.first_free() {
            tracks.retain(|t| t.kind != ks::CAN_LEAVE);
            let from = (f as f32 - INPUT_LEAD).max(0.0);
            tracks.push(EventTrack::level(ks::CAN_LEAVE, 0, &[(from, clip.length.max(1) as f32)]));
        }
    }
    // (KH2's dodge roll on KH1's roll: KH2's free frame as KH1's "may be left", `roll_free`.)
    if kind == Kind::DodgeRoll {
        tracks.retain(|t| t.kind != ks::CAN_LEAVE);
        tracks.push(EventTrack::level(ks::CAN_LEAVE, 0, &[(roll_free(clip) as f32, clip.length.max(1) as f32)]));
    }
    if cast {
        // (The cast's own KH1 combo window, `cast_follow`, in place of the one `kh1_tracks` made
        // from range id 11 alone.)
        tracks.retain(|t| t.kind != ks::COMBO);
        tracks.extend(cast_follow(clip, finisher_cast));
    }
    // (t-evts) KH2's frame triggers (sounds, voices, effect elements) as sound events of the
    // motion's own, which KH1's motion pass fires and KH1's sound event handler, wrapped
    // (`sound_event`), plays through `kh2sound` / `kh2fx` (`kh2_trigger_keys`).
    // (Steam only: KH1's handler is wrapped there. Elsewhere the old way, `tick` reading them.)
    let steam = super::addr::Build::current() == super::addr::Build::Steam;
    let triggers = kh2_trigger_keys(motion, clip, reaction, src.form.is_some());
    let events: Vec<(u32, EventKind)> = if steam { Vec::new() } else { kh2_trigger_events(clip, reaction) };
    if steam {
        let dropped = ks::add_sound_keys(&mut tracks, &triggers);
        if dropped > 0 {
            log!(Animation, "KH2 slots: KH2 motion {motion}: {dropped} of its {} frame triggers left out (more than KH1 reads in one frame)", triggers.len());
        }
    }
    // (KH1 moves the actor by the motion's H10 when the request has bit 15, `travels`.)
    // (KH1's roll request has it too: KH2's roll moves him by its root, `ROLL`.)
    // (A Drive Form's ground cast with root travel of its own: KH1 moves him by it too, the request
    // given KH1's travel flag while the form's slot is on it, `redirect`. Wisdom's Fire 56 goes 384
    // forward, its finisher cast 57 836 (Checked (files) `P_EX100_MAGF.mset`; base Sora's casts have
    // none). The tester (drive-forms): "fires physics are missing".)
    let form_cast_travel = kind == Kind::Magic && src.form.is_some() && clip.root.iter().any(|r| r[0].hypot(r[2]) > 1.0);
    let travel = (matches!(kind, Kind::GroundCombo | Kind::DodgeRoll) && travels(requests[request])) || form_cast_travel;
    // (From the motion slot cache when it has this exact bake, `slot_key`; baked and kept there
    // otherwise.)
    let mut cache = cache;
    let baking = bake.map(|b| match cache.as_deref_mut() {
        Some(c) => c.get_or_bake(slot_key(&template, (motion, field), frames, &tracks, travel, src.form), || {
            super::loadscreen::baking();
            bake_one(b, src, &template, (motion, field), frames, &tracks, travel)
        }),
        None => {
            super::loadscreen::baking();
            bake_one(b, src, &template, (motion, field), frames, &tracks, travel).map(crate::slotcache::Block::new)
        }
    });
    let (block, baked) = match baking {
        Some(Ok(block)) => (block, true),
        Some(Err(e)) => {
            log!(Animation, "KH2 slots: KH2 motion {motion} not baked ({e}); drawn over its template");
            (crate::slotcache::Block::new(ks::retimed(&template, frames, &tracks).map_err(|e| e.to_string())?), false)
        }
        None => (crate::slotcache::Block::new(ks::retimed(&template, frames, &tracks).map_err(|e| e.to_string())?), false),
    };
    log!(Animation,
        "KH2 slots: KH2 motion {motion}{} as KH1 slot {slot} (for KH1's slot {stands_for} / request {request:#x}{}, {old} -> {frames} frames): {}",
        if field { " (field)" } else { "" },
        if default { "" } else { ", by KH2's row selection" },
        tracks.iter().map(|t| format!("{}.{} {:?}", t.kind, t.id, t.keys.iter().map(|k| (k.time, k.a)).collect::<Vec<_>>())).collect::<Vec<_>>().join("; ")
    );
    // (KH2's own loop point, for the fall: `loop_range`.)
    let loop_from = (kind == Kind::Jump && !own).then(|| src.rig.motions.get(&motion)).flatten().and_then(|m| {
        let k = 60.0 / m.fps.max(1e-3);
        let from = (m.frame_return - m.frame_start) * k;
        (from > 0.5 && from < frames as f32 - 0.5).then_some(from)
    });
    // (Valor's moves: the left Keyblade's holder motion too, from the slot cache when it has this
    // exact bake, `holder_key`; plan step 4 of `docs/kh1-weapon-second.md` §7.3.)
    let two = src.form.is_some_and(|k| k.two_blades() && k.native_capable());
    // (t832: the left Keyblade's trail windows, our 7.1 on the move, as KH1's own trail level 7.0
    // on the holder motion: KH1's pass then sets the holder actor's own trail bit.)
    let left_trail: Vec<EventTrack> = tracks.iter().filter(|t| t.kind == LEFT_TRAIL_KIND && t.id == LEFT_TRAIL_ID).map(|t| EventTrack { id: 0, ..t.clone() }).collect();
    let holder_block = match (bake, baked && two) {
        (Some(b), true) => {
            let r = match cache {
                Some(c) => c.get_or_bake(holder_key(&template, (motion, field), frames, src.form, &left_trail), || {
                    super::loadscreen::baking();
                    holder_one(b, src, &template, (motion, field), frames, &left_trail)
                }),
                None => holder_one(b, src, &template, (motion, field), frames, &left_trail).map(crate::slotcache::Block::new),
            };
            match r {
                Ok(block) => Some(block),
                Err(e) => {
                    log!(Animation, "KH2 slots: KH2 motion {motion}: no holder motion ({e}); the left Keyblade follows KH2's pose there");
                    None
                }
            }
        }
        _ => None,
    };
    let holder = holder_block.is_some();
    Ok(Some((Move { motion, stands_for, request, kind, slot, frames, baked, travel: travel && baked, default, field, variant, own: None, events, loop_from, form: src.form, holder, holder_block: Mutex::new(holder_block) }, block)))
}

/// One holder motion's key in the motion slot cache: the main bake's inputs it reads (the template,
/// the motion, its variant and length, the moveset) and that it is a holder motion.
fn holder_key(template: &[u8], (motion, field): (u16, bool), frames: u32, form: Option<super::form::Kind>, events: &[EventTrack]) -> u128 {
    let mut h = crate::slotcache::Hasher::default();
    h.str("left Keyblade holder motion").bytes(template).u32(motion as u32).u32(field as u32).u32(frames).str(&format!("{events:?}"));
    if let Some(k) = form {
        h.str(k.model());
        // (t829: Valor's left Keyblade from KH2's weapon bone 2, `pose::kh2_hold`; t831: in the
        // left palm's own space, for the holder actor.)
        if k == super::form::Kind::Valor {
            h.str("kh2 hold");
            h.str("palm space");
            // (t841: no root travel, the template's travel helper's channels dropped.)
            h.str("no travel");
            // (t849: the weapon bones from the left wrist, the holder's bone.)
            h.str("wrist space");
            // (t850: through the skeleton pass's row flip of the holder's world matrix.)
            h.str("pass row flip");
            // (t851: the hold placed from the middle knuckle, `pose::kh2_hold`.)
            h.str("knuckle anchor");
            // (t852: the left keychain from KH2's left weapon motion.)
            h.str("left chain");
        }
    }
    h.finish()
}

/// The left Keyblade's holder motion for a two-Keyblade form's `motion` (Valor's; plan step 4 of
/// `docs/kh1-weapon-second.md` §7.3): a KH1 motion on the same template and length as the move's
/// own bake, its body at KH1 Sora's bind and its weapon bones (280..292) the left hand's Keyblade
/// from KH2's left weapon motion, as they sit from the left hand (`pose::Baker::holder_poses`),
/// with `events` only (t832: the left Keyblade's trail windows as KH1's own trail level 7.0) and no
/// root travel (t841). KH1 poses it on the holder actor (`holder`) at Sora's motion's time.
fn holder_one(b: &pose::Baker, src: &pose::Src, template: &[u8], (motion, field): (u16, bool), frames: u32, events: &[EventTrack]) -> Result<Vec<u8>, String> {
    let (poses, _) = b.poses_of(src, motion, field, frames, false).ok_or("KH2 has no animation for it")?;
    let (held, reach) = b.holder_poses(src, motion, field, &poses)?;
    // (t841) No root travel: the template's travel helper (H10) translation channels are replaced
    // by zeros, so none is written (`kh1_data::bake`). With them, KH1's pose pass wrote the KH1
    // slot's own travel (walk, run) into the holder instance's root travel (`+0xF0`), which the
    // pose step copies into the actor's place `+0x50` (`0x1403948D0`, from `0x14029F0A0`), where
    // KH1's draw puts the model and its shadow: the left Keyblade flew off from Sora's palm at
    // every walk / run (the owner's t840 trace: `+0x50` thousands of units out while `+0x100`,
    // the palm, held). The holder hangs on the palm and travels with Sora's palm only.
    let still = vec![glam::Vec3::ZERO; held.len()];
    let (block, r) = match kh1_data::bake::baked(template, frames, events, &b.skeleton, &held, &still, false) {
        Ok(v) => v,
        // (A template with no helpers has no travel to drop.)
        Err(e) if e.to_string().contains("no helpers") => kh1_data::bake::baked(template, frames, events, &b.skeleton, &held, &[], false).map_err(|e| e.to_string())?,
        Err(e) => return Err(e.to_string()),
    };
    log!(Animation,
        "KH2 slots: KH2 motion {motion}'s left Keyblade holder motion baked ({frames} frames, {} bytes): {} keyed channels, {} keys, {} constants; the left grip at most {reach:.1} units from the left hand; KH1's pass from it within {:.3} units (worst bone {})",
        block.len(),
        r.keyed_channels,
        r.keys,
        r.constant_channels,
        r.max_error,
        r.worst_bone
    );
    Ok(block)
}

/// The swings KH2's combo rows can take: on the ground (`air` false) the ground rows with no hop
/// (a row with `jump` leaves the ground), in the air the air rows (each hops a little, as KH1's
/// air swings do): (motion, finisher), each once. The rows the selector marks (kind 1, 2, 3) and
/// the rows their `sub` swaps to (any kind; the exe's anti-repeat keeps the `sub` row, Steam
/// `0x140402C50`). With no ability, and the ground ability rows that are only a Keyblade swing
/// (`blade_swing`): on the ground its swings (Slapshot, Dodge Slash, Slide Dash), which KH1's
/// ground swing rows play natively as they do the plain ones, in the air its hops from the ground
/// (Aerial Sweep, Aerial Dive), which KH1's air swing rows play as KH2's plain hop, and its air
/// rows (Aerial Spiral, the finisher Aerial Finish), which KH1's air swing and finisher rows play
/// as the plain air rows (`combomods::native_ability`, `kh2_hop`, `air_hop`); the other ability
/// rows are still borrows (`combomods`).
fn swing_rows(rows: &[character_core::ComboRow], clips: &std::collections::HashMap<u16, character_core::Clip>, battle: &kh2_formats::battle::Battle, air: bool) -> Vec<(u16, bool)> {
    let picked = |r: &character_core::ComboRow| r.kind != 0;
    let subs: Vec<i8> = rows.iter().filter(|r| picked(r)).map(|r| r.sub).collect();
    let mut out: Vec<(u16, bool)> = Vec::new();
    for r in rows {
        let reachable = picked(r) || subs.contains(&(r.id as i8));
        // (In the air also the ground rows that hop: KH1's air swing rows play them from the
        // ground, `combomods::kh2_hop`; the plain ones' motions are the air rows' own.)
        let here = if air { r.is_air() || r.is_jump() } else { !r.is_air() && !r.is_jump() };
        // (A ground finisher of an ability's that is only a Keyblade swing, Guard Break, is left a
        // borrow: its guard break isn't KH1's finisher's attack data. One with hits on his body,
        // Explosion, plays on KH1's finisher row: KH2's hits land through `takeover::kh2_hits`.)
        let ability_ok = r.ability == 0 || clips.get(&r.motion).is_some_and(|c| ((r.is_air() || !r.is_finisher()) && blade_swing(c, battle)) || body_strike(c, battle));
        if reachable && r.motion != 0 && here && ability_ok && !out.iter().any(|o| o.0 == r.motion) {
            out.push((r.motion, r.is_finisher()));
        }
    }
    out
}

/// A KH2 ability row's motion is only a Keyblade swing, so KH1's own swing row can play it as it
/// plays the plain swings (its attack object on the Keyblade, timed by the slot's hit windows,
/// and KH2's root travel baked in): it hits with the Keyblade's shapes only (a `Hitbox` window on
/// a weapon, none on his own body), and has no invulnerability, no air / ground range of its own
/// (each of those is KH2 behaviour KH1's swing row doesn't have), and no guard on his body but
/// one whose ATKP lets the hit go with no effect (`refact_self` 2). Its no-gravity ranges and
/// those guards are KH1's now (`no_gravity`, `shared::physics`'s air step; `body_guard`, `hurt`),
/// and so are its guards on the Keyblade (`keyblade_guard`, `hurt`, at the Recoil row's rule,
/// `recoil::guard_window_on`). (t-rows) Those Keyblade guards used to keep the move off our rows:
/// with the Keyblade's own windows read (`W_EX010.mset`, Checked (files)), Flash Step 169 has
/// ATKP 162 at 17..22 and 161 at 22..30 on the Keyblade and Vicinity Break 170 has 162 at 14..32,
/// so neither was built and KH2's pick of them was "no attack" (the tester's log, f25752: row 51
/// motion 170 "has no KH1 row of its own built"; no build since t631 lists 169 or 170).
/// Sora's (`P_EX100.mset`, `00battle.bin`, Checked (files)): Slapshot 162, Dodge Slash 163, Slide
/// Dash 164, Flash Step 169 (a guard on his body, ATKP 406, kind 2, `refact_self` 2, frames
/// 0..19), Aerial Sweep 191, Aerial Dive 196 (no gravity at frames 46..54), Aerial Spiral 192
/// (16..24) and Aerial Finish 194 (16..100) pass; Magnet Burst 197 (body attacks 1900 / 1901 /
/// 1903), Vicinity Break 170 (a body attack, 1869 on group 21), Explosion 166 (911 / 912),
/// Finishing Leap 167 and Counterguard 171 don't (`body_strike` takes the first three); Guard
/// Break 165 is a ground finisher, left out above.
fn blade_swing(clip: &character_core::Clip, battle: &kh2_formats::battle::Battle) -> bool {
    use character_core::WindowKind as W;
    let blade = clip.windows.iter().any(|w| matches!(w.kind, W::Hitbox { weapon, .. } if weapon > 0));
    let lets_go = |attack: u16| battle.attack_for_level(attack, 0, 0).is_some_and(|a| a.refact_self == 2);
    let other = clip.windows.iter().any(|w| match w.kind {
        W::Guard { attack, weapon: 0, .. } => !lets_go(attack),
        // (A guard on the Keyblade is KH1's hit pass's, `keyblade_guard` / `hurt`.)
        W::Guard { .. } => false,
        W::Hitbox { weapon: 0, .. } | W::Invulnerable | W::Air | W::Grounded => true,
        _ => false,
    });
    blade && !other
}

/// A KH2 ability row's motion that hits with shapes on his own body (`Hitbox` windows with weapon
/// 0: Vicinity Break, Explosion, Magnet Burst), which KH1's one sphere on the Keyblade can't do.
/// KH1's row runs it as it runs the Keyblade swings, and each of those windows is a KH1 attack
/// object of ours with KH2's shapes, on a KH1 attack group of its own, that KH1's collision pass
/// collides (`bodyhits`; KH2's own sweep, `takeover::kh2_hits`, where that isn't in); its invulnerability (range 27) is KH1's hit pass's (`invulnerable`, `hurt`), its
/// guards on his body that let the hit go are `body_guard`'s, its no-gravity ranges `no_gravity`'s.
/// Not with a body guard of another kind or an air / ground range (a guard on the Keyblade is
/// `keyblade_guard`'s, as in `blade_swing`). Sora's
/// (`P_EX100.mset`, `00battle.bin`, Checked (files)): Vicinity Break 170 (1869 on group 21,
/// frames 20..30; guard 406 14..32), Explosion 166 (911 on group 19 32..57, 912 57..64;
/// invulnerable and guard 124, kind 3, `refact_self` 2, 32..64), Magnet Burst 197 (1900 on 22
/// 16..56, 1901 on 23 20..56, 1903 on 3 64..70; guard 406 20..70; no gravity 16..82); Finishing
/// Leap 167 and Counterguard 171 too, but their rows are Square rows (kind 0), not the selector's.
/// Their Keyblade windows (`W_EX010.mset`) are KH1's swing's as for the others: Vicinity Break's
/// guard 162 at 14..32 among them.
fn body_strike(clip: &character_core::Clip, battle: &kh2_formats::battle::Battle) -> bool {
    use character_core::WindowKind as W;
    let body = clip.windows.iter().any(|w| matches!(w.kind, W::Hitbox { weapon: 0, .. }));
    let lets_go = |attack: u16| battle.attack_for_level(attack, 0, 0).is_some_and(|a| a.refact_self == 2);
    let other = clip.windows.iter().any(|w| match w.kind {
        W::Guard { attack, weapon: 0, .. } => !lets_go(attack),
        W::Guard { .. } => false,
        W::Air | W::Grounded => true,
        _ => false,
    });
    body && !other
}

/// KH2's motion and KH1's time in it (its clip frame) for our slot playing on `actor` now
/// (`takeover::kh2_hits`).
pub unsafe fn now_playing(actor: usize) -> Option<(u16, f32)> {
    let m = current(actor)?;
    let t: f32 = rd(actor + TIME_NOW);
    (t.is_finite() && t >= 0.0).then_some((m.motion, t))
}

/// The KH2 motion whose invulnerable range holds KH1's no-damage flag now (`invulnerable_step`),
/// for the log and to know the hold is ours to end.
static RANGE_ON: Mutex<Option<u16>> = Mutex::new(None);
/// Our timer's mark: we write whole frames less this, so its fraction tells ours from KH1's own
/// (whole frames: 12, 30, 8). KH1 counts down by whole 60 Hz frames, so the fraction stays.
const RANGE_MARK: f32 = 0.25;
/// The no-damage timer written while the range holds (re-written each frame): enough to last until
/// the next frame's check through KH1's countdown (`0x140297FE0`, by the frame scale, 1 or 2 a
/// frame), carrying `RANGE_MARK`.
const RANGE_HOLD: f32 = 3.0 - RANGE_MARK;

/// (t868) KH2's invulnerable ranges (range id 27: Explosion 32..64, Aerial Recovery 0..16,
/// Counterguard 0..20, Finishing Leap 24..38; Checked (files)) on KH1's own no-damage flag, each
/// frame he is posed (`frame`): our level event 8.0 of the motion KH1 plays, read with KH1's own
/// level reader at KH1's time in it (`kh1_level`), holds the flag (KH1's timer, `RANGE_HOLD`) while
/// it is on and ends it the first frame it is off, if the timer is still ours.
///
/// KH2 (Checked (decomp)): every motion update clears the range bits (`0x1403C83B0` from
/// `0x14040E5C0`) and sets them again for each range the clip's time is in (`0x14040E740` ->
/// `0x14040D810`; range 27 sets `+0x18C` bit 22, which the actor's invulnerable test reads, vtable
/// `+0xB8`), at a blending motion's start time. So the invulnerability lasts exactly while the
/// time is in the range, through the blend into the move (KH1 holds its time at the new motion's
/// start there too), and ends on the first update after the move is cut. KH1 reads "no damage"
/// from the actor's flags in six places (`0x1000080`), so the flag is what it gets.
///
/// t860 set the timer once to the frames left when the range opened: it started after a blend
/// into a range from frame 0 and ran on when the move was cut (that end was an **Assumption**).
/// A longer KH1 timer of its own (a reaction's 30, the roll's) is kept and isn't ours to end, as
/// KH2 tests its timer and the range apart.
pub unsafe fn invulnerable_step(actor: usize) {
    let on = (|| {
        if !super::shared::active() {
            return None;
        }
        let m = current(actor)?;
        kh1_level(actor, ks::KH2_RANGE, ks::RANGE_INVULNERABLE)?.then_some(m.motion)
    })();
    let Ok(mut g) = RANGE_ON.try_lock() else { return };
    let left: f32 = rd(actor + 0xC);
    let running = super::combomods::kh1_no_damage_on(actor) && left > 0.0;
    let ours = running && (left.fract() - (1.0 - RANGE_MARK)).abs() < 0.01;
    match on {
        Some(m) => {
            if !(running && !ours && left >= RANGE_HOLD) {
                super::combomods::kh1_no_damage(actor, RANGE_HOLD);
            }
            if *g != Some(m) {
                log!(Combat, "KH2's invulnerable range of motion {m} (KH1's level 8.0) at frame {:.0}: KH1's no-damage flag held", rd::<f32>(actor + TIME_NOW));
            }
            *g = Some(m);
        }
        None => {
            if let Some(m) = g.take() {
                if ours {
                    super::combomods::kh1_no_damage(actor, 0.0);
                }
                log!(Combat, "KH2's invulnerable range of motion {m} over (frame {:.0}{}): KH1's no-damage flag {}", rd::<f32>(actor + TIME_NOW), if current(actor).is_some_and(|c| c.motion == m) { "" } else { ", the move left" }, if ours { "dropped" } else { "left to KH1's own timer" });
            }
        }
    }
}

/// KH2's guard on his body in our slot playing on `actor` now (`WindowKind::Guard` with weapon 0
/// at KH1's time in the slot), if its ATKP lets the hit go with no effect (`refact_self` 2: KH2's
/// guard reaction is none, the move goes on; Flash Step's 406). `hurt` stops KH1's hits there
/// as KH2's guard does (a guard on his body is all round).
pub unsafe fn body_guard(actor: usize) -> Option<u16> {
    use character_core::WindowKind as W;
    let m = current(actor)?;
    let kh2 = pose::kh2()?;
    let clip = clip_of(m)?;
    let t: f32 = rd(actor + TIME_NOW);
    if !(t >= 0.0) {
        return None;
    }
    let f = t as u32;
    clip.windows.iter().filter(|w| w.contains(f)).find_map(|w| match w.kind {
        W::Guard { attack, weapon: 0, .. } if kh2.battle.attack_for_level(attack, 0, 0).is_some_and(|a| a.refact_self == 2) => Some(attack),
        _ => None,
    })
}

/// KH2's guard on the Keyblade in our slot playing on `actor` now (a `Guard` window on the weapon
/// at KH1's time in the slot: the guard press 173's ATKP 161 at 6..37, `refact_self` 1, the repel;
/// its repel 42's 162 at 0..40 and Guard Break 165's 162 at 5..42, `refact_self` 2, nothing;
/// Checked (files), W_EX010.mset): its ATKP and "refact self". `hurt` takes a hit from in front
/// there as KH2's guard does (`combomods::guard_repel`).
pub unsafe fn keyblade_guard(actor: usize) -> Option<(u16, u8)> {
    use character_core::WindowKind as W;
    let m = current(actor)?;
    let kh2 = pose::kh2()?;
    let clip = clip_of(m)?;
    let t: f32 = rd(actor + TIME_NOW);
    if !(t >= 0.0) {
        return None;
    }
    let f = t as u32;
    clip.windows.iter().filter(|w| w.contains(f)).filter_map(|w| match w.kind {
        W::Guard { attack, weapon, .. } if weapon > 0 && super::recoil::guard_window_on(m.motion, weapon) => kh2.battle.attack_for_level(attack, 0, 0).map(|a| (attack, a.refact_self)),
        _ => None,
    }).min_by_key(|g| g.1)
}

/// Our slot playing on `actor` holds him with no gravity now: KH2's no-gravity range (range id 3 /
/// 24, `WindowKind::NoGravity`) as our level event 8.1 of the motion KH1 plays, read by KH1's own
/// level reader at KH1's time in it (`kh1_level`, t869; through a blend at the new motion's start,
/// as KH2 reads its ranges). Not for a clip that allows movement all through (Wisdom Form's shots),
/// as KH2's controller has it (`Character`'s action step): the bake leaves 8.1 out of those
/// (`no_gravity_spans`). KH1's air step holds his rise or fall there (`shared::physics`), and the
/// root travel, lift-off and rise start follow it (`air_root`, `air_root_form`, `lifts_off`,
/// `rise_start`). Off Steam (no level reader) KH2's clip is read as before t869.
pub unsafe fn no_gravity(actor: usize) -> bool {
    if super::addr::Build::current() == super::addr::Build::Steam {
        return kh1_level(actor, ks::KH2_RANGE, ks::RANGE_NO_GRAVITY) == Some(true);
    }
    let Some(m) = current(actor) else { return false };
    let Some(clip) = clip_of(m) else { return false };
    let t: f32 = rd(actor + TIME_NOW);
    t >= 0.0 && no_gravity_spans(clip).iter().any(|&(a, b)| a <= t && t < b)
}

/// KH2's no-gravity ranges of `clip` as our level event 8.1 takes them (frames, start..end): none
/// for a clip that allows movement all through (`no_gravity`).
fn no_gravity_spans(clip: &character_core::Clip) -> Vec<(f32, f32)> {
    use character_core::WindowKind as W;
    let moving = clip.windows.iter().any(|w| matches!(w.kind, W::AllowMovement) && w.start == 0 && w.end.is_none());
    if moving {
        return Vec::new();
    }
    let len = clip.length as f32;
    clip.windows.iter().filter(|w| matches!(w.kind, W::NoGravity)).map(|w| (w.start as f32, w.end.map_or(len, |e| e as f32))).collect()
}

/// KH2's allow-movement range (range id 41, `WindowKind::AllowMovement`) of our slot playing on
/// `actor` at KH1's time in it: `Some(open)` while one of our slots plays, `None` otherwise. A
/// Drive Form with ground inertia steers with the stick there (`shared::physics`, Wisdom's shots).
pub unsafe fn allow_movement(actor: usize) -> Option<bool> {
    use character_core::WindowKind as W;
    // (t872) Our level event 8.4 of the motion KH1 plays, read by KH1's own reader (`kh1_level`,
    // `Some` only while one of our slots plays); off Steam KH2's clip as before. KH2's input step
    // keeps the stick only with range 41 or a free range on (`0x1403A89A0`, Checked (decomp)).
    if super::addr::Build::current() == super::addr::Build::Steam {
        return kh1_level(actor, ks::KH2_RANGE, ks::RANGE_ALLOW_MOVEMENT);
    }
    let m = current(actor)?;
    let clip = clip_of(m)?;
    let t: f32 = rd(actor + TIME_NOW);
    Some(t >= 0.0 && clip.has(t as u32, |k| matches!(k, W::AllowMovement)))
}

/// KH2's root travel over the next `dt` 60 Hz frames of our slot playing on `actor`, while its
/// no-gravity range holds him (`no_gravity`), in the world (KH2's y up), along his facing (KH1's
/// yaw `+0x3C`, as KH2's controller turns the clip's root by its facing: right = (cos, -sin),
/// forward = (sin, cos)): KH2 moves him by the root, up and down too, where the clip has no gravity
/// (the OG rewrite's action step; Finishing Leap's root rises 150 over its frames, Checked
/// (files)). `None` with no root travel there (every other air motion of Sora's has none).
/// KH1's air step adds it to his move (`shared::physics`).
pub unsafe fn air_root(actor: usize, dt: f32) -> Option<[f32; 3]> {
    if !no_gravity(actor) || !(dt > 0.0) {
        return None;
    }
    let m = current(actor)?;
    let clip = clip_of(m)?;
    let t: f32 = rd(actor + TIME_NOW);
    root_travel(clip, t, dt, rd(actor + 0x3C))
}

/// A Drive Form's slot playing on `actor` outside its no-gravity range: its root travel over the
/// next `dt` 60 Hz frames as `air_root` gives it in the range (the tester, drive-forms: "air fires
/// physics are wrong"). The OG rewrite's action step adds the clip's root to the move on every
/// frame of an action (`character_core::Controller`'s `fo.root`), with gravity or without; Wisdom's
/// air Fire 58 goes 383 forward, 219 of it before its no-gravity range (16..36), Valor's air
/// finishers 183 / 184 600 / 220 (Checked (files)). Base Sora's air motions are left as they were
/// (none of his has root travel outside such a range, Checked (files) `P_EX100.mset`).
pub unsafe fn air_root_form(actor: usize, dt: f32) -> Option<[f32; 3]> {
    if no_gravity(actor) || !(dt > 0.0) {
        return None;
    }
    let m = current(actor).filter(|m| m.form.is_some())?;
    let clip = clip_of(m)?;
    let t: f32 = rd(actor + TIME_NOW);
    root_travel(clip, t, dt, rd(actor + 0x3C))
}

/// A Drive Form's ground move playing on `actor` leaves the ground now: its clip in its no-gravity
/// range (range id 3) with its root rising over the next `dt` 60 Hz frames. Valor's ground finisher
/// 154: no gravity from 63 to its end, its root rising from 64 to 125 at 116 (Checked (files)
/// `P_EX100_BTLF.mset`); KH2's mover keeps him off the floor in the range (`0x1403B9090` with
/// `+0x18C` bit 5 set, `ground_hold`'s note, Checked (decomp)) and the root's rise carries him up.
/// Base Sora's ground moves have no rising root but Finishing Leap's, entered in the air already
/// (`kh1rows::Ours::rises`).
pub unsafe fn lifts_off(actor: usize, dt: f32) -> bool {
    let Some(m) = current(actor).filter(|m| m.form.is_some()) else { return false };
    let Some(clip) = clip_of(m) else { return false };
    if !no_gravity(actor) || !(dt > 0.0) {
        return false;
    }
    let t: f32 = rd(actor + TIME_NOW);
    root_travel(clip, t, dt, rd(actor + 0x3C)).is_some_and(|d| d[1] > 0.01)
}

/// A Drive Form's slot playing on `actor` is in its allow-movement range now (range id 41): KH2
/// steers the air move with the stick there, as in a jump (`shared::physics`'s air step).
pub unsafe fn form_air_steer(actor: usize) -> bool {
    current(actor).is_some_and(|m| m.form.is_some()) && allow_movement(actor) == Some(true)
}

/// A clip's root travel from its frame `t` over `dt` 60 Hz frames, in the world (KH2's y up),
/// turned by the yaw as `air_root` turns it. `None` with none.
fn root_travel(clip: &character_core::Clip, t: f32, dt: f32, yaw: f32) -> Option<[f32; 3]> {
    let at = |f: f32| -> [f32; 3] {
        let n = clip.root.len();
        if n == 0 {
            return [0.0; 3];
        }
        let f = f.clamp(0.0, (n - 1) as f32);
        let (i, w) = (f.floor() as usize, f.fract());
        let (a, b) = (clip.root[i], clip.root[(i + 1).min(n - 1)]);
        [a[0] + (b[0] - a[0]) * w, a[1] + (b[1] - a[1]) * w, a[2] + (b[2] - a[2]) * w]
    };
    let (a, b) = (at(t), at(t + dt));
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    if d.iter().all(|v| v.abs() < 1e-4) || !d.iter().all(|v| v.is_finite()) {
        return None;
    }
    let (fw, right) = ([yaw.sin(), yaw.cos()], [yaw.cos(), -yaw.sin()]);
    Some([d[0] * right[0] + d[2] * fw[0], d[1], d[0] * right[1] + d[2] * fw[1]])
}

/// A row that leaves the ground by its root (`kh1rows::Ours::rises`: Finishing Leap) just started
/// out of KH1's ground combo into KH1's air state (`combomods::ours_run`): the game frame and its
/// KH2 motion.
static RISE: Mutex<Option<(u64, u16)>> = Mutex::new(None);
/// The longest the start is bridged: the slot is applied and its no-gravity range (from frame 1
/// for Finishing Leap) reached within a frame or two.
const RISE_FRAMES: u64 = 4;

pub fn start_rise(motion: u16) {
    if let Ok(mut g) = RISE.lock() {
        *g = Some((super::takeover::GAME_FRAME.load(Ordering::Relaxed), motion));
    }
}

/// The frames between such a row's start and its clip's own no-gravity range (`no_gravity`):
/// Finishing Leap's range id 3 starts at frame 1 (Checked (files), `P_EX100.mset` motion 167), and
/// before it (KH1's request queued, its slot not yet applied, or the slot's frame 0) KH1's air
/// state had him at an arc of height 0, so KH1's air step put him down at once and KH1 played its
/// landing (request 7) the next frame (the user's trace f6071..f6073: request 0x201 for one frame,
/// then 0x7). In KH2 he is on the ground there (`0x1403A9950` starts the row from his ground
/// action), with no fall. Held here as in the no-gravity range, moved up by the root's first step
/// of that range (at most a frame early, **Assumption**: KH1 would otherwise put him down with
/// no rise at all); ends once the range holds him or after `RISE_FRAMES`.
pub unsafe fn rise_start(actor: usize, dt: f32) -> Option<[f32; 3]> {
    use character_core::WindowKind as W;
    let Ok(mut g) = RISE.try_lock() else { return None };
    let (at, motion) = (*g)?;
    let now = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    if no_gravity(actor) || now.saturating_sub(at) > RISE_FRAMES || rd::<i32>(actor + 0x70) != 2 {
        *g = None;
        return None;
    }
    let clip = pose::kh2_moves()?.moveset.clips.get(&motion)?;
    let from = clip.windows.iter().filter(|w| matches!(w.kind, W::NoGravity)).map(|w| w.start).min()?;
    let dt = if dt.is_finite() && dt > 0.0 && dt < 4.0 { dt } else { 1.0 };
    Some(root_travel(clip, from as f32, dt, rd(actor + 0x3C)).unwrap_or([0.0; 3]))
}

/// KH2's clip of our slot playing on `actor` has its edge stop now (range id 29,
/// `WindowKind::EdgeStop`, at KH1's time in the slot, the clip's frame): KH2's ground mover keeps
/// him on the ledge there (`shared::physics`'s ground step). Sora's (`P_EX100.mset`, Checked
/// (files)): the finishers 154 (frames 0..38) and 155 (0..39), Guard Break 165 (0..40), Flash Step
/// 169 (0..17), and 161 (0..28).
pub unsafe fn edge_stop(actor: usize) -> bool {
    use character_core::WindowKind as W;
    // (t871) Our level event 8.2 of the motion KH1 plays, read by KH1's own reader (`kh1_level`);
    // off Steam KH2's clip as before.
    if super::addr::Build::current() == super::addr::Build::Steam {
        return kh1_level(actor, ks::KH2_RANGE, ks::RANGE_EDGE_STOP) == Some(true);
    }
    let Some(m) = current(actor) else { return false };
    let Some(clip) = clip_of(m) else { return false };
    let t: f32 = rd(actor + TIME_NOW);
    t >= 0.0 && clip.has(t as u32, |k| matches!(k, W::EdgeStop))
}

/// KH2's ground action keeps going off a ledge now (the tester, what-is-still-missing after m14,
/// answer 4 "(a)"): KH2's clip of our slot playing on `actor` has, at KH1's time in the slot (the
/// clip's 60 Hz frame), a range KH2's mover reads as "don't leave the ground": no gravity (range
/// id 3 / 24, `WindowKind::NoGravity`) or the edge stop (id 29, `WindowKind::EdgeStop`). KH2's
/// mover (Steam KH2 `0x1403B9090`, Checked (decomp)) calls the player's "left the ground" method
/// (vtable `+0x50`: the fall motion 4 and the fall action) only with no floor under him and none
/// of them: not `0x1403BA9A0` (the object entry's `+0xC` bit 0, the motion state word `+0x18C`
/// bits 5 / 24 (range ids 3 / 30), `+0x120` bit 23), not the entry's bit 1, not `+0x18C` bit 6
/// (range 24) nor bit 23 (range 29). So he goes on with the move where he is, the clip's root
/// carrying him and no gravity; the first frame without them he is cut into the fall. Range 30 is
/// in none of Sora's or the forms' motions (Checked (files): `P_EX100*.mset`, `W_EX010*.mset`);
/// the entry and `+0x120` bits aren't motion data (**Assumption**: clear for Sora). The moves
/// (Checked (files)): base Sora's 154 (edge stop 0..38), 155 (0..39), 161 (0..28), Guard Break
/// 165 (0..40), Flash Step 169 (0..17); Valor's 154 (edge stop 0..63, then no gravity from 63 to
/// its end) and 155 (0..64); the Drive 52 (no gravity all through). Returns the KH2 motion.
pub unsafe fn ground_hold(actor: usize) -> Option<u16> {
    use character_core::WindowKind as W;
    // (t871) Our level event 8.3 of the motion KH1 plays (`ground_hold_spans`), read by KH1's own
    // reader (`kh1_level`); off Steam KH2's clip as before.
    if super::addr::Build::current() == super::addr::Build::Steam {
        let m = current(actor)?;
        return (kh1_level(actor, ks::KH2_RANGE, ks::RANGE_GROUND_HOLD) == Some(true)).then_some(m.motion);
    }
    let m = current(actor)?;
    let clip = clip_of(m)?;
    let t: f32 = rd(actor + TIME_NOW);
    (t >= 0.0 && clip.has(t as u32, |k| matches!(k, W::NoGravity | W::EdgeStop))).then_some(m.motion)
}

/// KH2's turn toward the target in our slot playing on `actor` (the tester: "(a)", KH2's turn
/// during swings on KH1's rows; Steam KH2, Checked (decomp)): KH2's range handler (`0x14040D810`)
/// runs range id 28 (`TurnToLock`, `WindowKind::TurnToLock`, its parameter in degrees) each motion
/// update while the motion's time is in it (`0x14040E740`: start <= t < end) through `0x1403D4B10`:
/// with a primary target (the player's `+0xDC8`), `0x1403D3A90` takes the flat line from his
/// position (`+0x670`, plus `+0x690`, 0 outside a few states) to the target's point
/// (`0x1403BDDE0`) and, longer than 50, turns him toward it (`0x1403B9F70`) by at most twice the
/// parameter (radians) an update, wherever he faced. KH2 takes one update a game frame (delta 2
/// at 30 fps, `docs/animations.md`), so this is per game frame too. Here: KH1's time in the slot
/// as the clip's frame, the target as KH2's controller picks it (`takeover::kh2_primary_target`:
/// KH1's lock, else KH2's auto target), and KH1's facing fields set (`+0x30`, its yaw `+0x3C`
/// and the yaw it turns toward `+0x150`, as `takeover` sets them for KH2's facing). Before KH1's
/// physics frame (`takeover::physics`). The OG rewrite turned by the parameter once a 60 Hz
/// frame, half the exe's at 60 fps.
pub unsafe fn turn_to_lock(actor: usize) {
    use character_core::WindowKind as W;
    let Some(m) = current(actor) else { return };
    let t: f32 = rd(actor + TIME_NOW);
    if !(t >= 0.0) {
        return;
    }
    // (t878) Our level event 8.5 of the motion KH1 plays, its rate in the key's `c`, read by
    // KH1's own reader (`kh1_key`); off Steam KH2's clip as before.
    let rate = if super::addr::Build::current() == super::addr::Build::Steam {
        let Some(Some(key)) = kh1_key(actor, ks::KH2_RANGE, ks::RANGE_TURN_TO_LOCK) else { return };
        if rd::<u16>(key + 4) == 0 {
            return;
        }
        f32::from_bits(rd::<u32>(key + 8))
    } else {
        let Some(clip) = clip_of(m) else { return };
        let Some(r) = clip.open(t as u32).find_map(|w| match w.kind {
            W::TurnToLock(r) => Some(r),
            _ => None,
        }) else {
            return;
        };
        r
    };
    if !(rate.is_finite() && rate > 0.0) {
        return;
    }
    let Some(target) = super::takeover::kh2_primary_target(actor) else { return };
    let p: [f32; 3] = rd(actor + 0x10);
    let d = [target[0] - p[0], target[2] - p[2]];
    let yaw: f32 = rd(actor + 0x3C);
    if !(d[0].hypot(d[1]) > 50.0) || !yaw.is_finite() {
        return;
    }
    let want = d[0].atan2(d[1]);
    let tau = std::f32::consts::TAU;
    let diff = (want - yaw + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI;
    let max = 2.0 * rate;
    let f = yaw + diff.clamp(-max, max);
    let f = (f + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI;
    if !f.is_finite() {
        return;
    }
    wr(actor + 0x30, [f.sin(), 0.0, f.cos(), f]);
    wr(actor + 0x150, f);
    // (t-turn) KH1's move vector (`+0x20`: its x / z; its y and the speed `+0x2C` kept) turned with
    // him. During an action KH1's control step gives no stick input, so its input filter
    // (`0x1402A5F10`) hands the ground state's input step (`0x1402CC1A0`) this vector, and that sets
    // the yaw he turns toward (`+0x150`, `0x140297FB0`) from it every frame; the player's think
    // (`0x1402A3300`, at its end through `0x1402A8300`) then turns him toward it at pi/16 a frame
    // (Checked (decomp), Steam). Left at the swing's start direction, it pulled him back 0.2 a frame
    // against KH2's turn (the tester's log, motion 153 frames 32..43: each turn starting about 0.2
    // short of the last one's result), and after the window turned him back to the start
    // direction, which KH2 never does (its facing changes only in this window).
    // (On the ground only, state 0: the air state's input step (`0x1402AFC20`) sets `+0x150` only
    // from a tilted stick (Checked (decomp)); whether its air moves read `+0x20` is not checked,
    // so it is left alone there. **Assumption**.)
    let mv: [f32; 3] = rd(actor + 0x20);
    if rd::<i32>(actor + 0x70) == 0 && mv[0].hypot(mv[2]) > 1e-4 {
        wr(actor + 0x20, [f.sin(), mv[1], f.cos()]);
    }
    // (t-turn) The motion's travel so far (`+0x210`) turned by the same angle, while KH1 moves him
    // by our slot's root travel (the request's travel flag, `+0x160` bit 4). KH1's motion pass
    // (`0x14029F0A0`, its end; Checked (decomp), Steam) takes his place less `+0x210` as the travel
    // origin, puts the motion's whole travel so far (helper H10) on it with his facing as it is
    // now, and moves him next frame by that less `+0x210` (`+0x200`), keeping it in `+0x210`. So a
    // turn mid-motion swung all the travel made so far round that origin: KH2's 153 has about 220
    // of travel by its turn window (frames 32..43), and each turn threw him 70..160 sideways (the
    // tester's trace f23068, frame 23027: moved (71, -92) on a 0.52 turn, what that rotation of
    // the travel gives). KH2 adds each frame's root step along the facing it has then (the OG
    // rewrite's action step, `fo.root`), so the travel already made stays put: with `+0x210`
    // turned too, KH1's next step is the motion's own step along the new facing, and the model
    // (drawn pulled back by `+0x210`, its body on the same travel) stays on him.
    let turned = f - yaw;
    if rd::<u32>(actor + 0x160) & 4 != 0 && turned.abs() > 1e-6 {
        let tr: [f32; 4] = rd(actor + 0x210);
        let (s, c) = turned.sin_cos();
        // (KH1's world: his facing is (sin yaw, cos yaw) on x / z, so a turn by `turned` takes
        // (x, z) to (x c + z s, -x s + z c).)
        wr(actor + 0x210, [tr[0] * c + tr[2] * s, tr[1], -tr[0] * s + tr[2] * c, tr[3]]);
        // (t-turn2) This frame's step (`+0x200`, made by the last motion pass along the facing
        // before this turn, added by KH1's physics step after us) turned too: KH2 lays a frame's
        // root step along the facing its turn has just set (Steam KH2, Checked (decomp): the
        // object update `0x1403BFD30` runs the motion update `0x1403C6740`, which samples the
        // root step into the motion's `+0xFC` and then runs the ranges, the turn `0x1403D4B10`
        // among them, `0x14040E740`; after the action's handlers its mover `0x1403B89A0` takes
        // that step through `0x1403B7CA0`, turned by his yaw `+0x68C` as it is then).
        let st: [f32; 4] = rd(actor + 0x200);
        wr(actor + 0x200, [st[0] * c + st[2] * s, st[1], -st[0] * s + st[2] * c, st[3]]);
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if super::log_on(super::LogCat::Animation) && N.fetch_add(1, Ordering::Relaxed) < 40 {
        log!(Animation, "KH2's turn to the target (KH2 motion {} frame {:.0}, up to {:.2} a frame): {:.2} -> {f:.2} (the target at {:.2}); travel so far turned with him", m.motion, t, max, yaw, want);
    }
}

/// KH2's attack on the Keyblade last seen in our slot on Sora: (KH2 motion, its ATKP, its
/// collision group, KH1's time then, whether a window of it was open on the last frame seen).
static BLADE_ATTACK: Mutex<Option<(u16, u16, u8, f32, bool)>> = Mutex::new(None);

/// (t-rehit, t-rehit2) KH2's next attack on the Keyblade in the same swing hits again what the last
/// one hit (the tester, combo row selection after m32: "the 2nd hit is also missing its 2nd
/// hitbox"; after m33: "if other moves are multi hits in kh2 ... (aerial finish) then yes apply
/// it ... check it behind kh2 c decomp"). KH2 (Steam, Checked (decomp)): each frame an attack range
/// is open its ATKP is asserted for its group (`0x14040D810` case 10 → `0x1403DAFE0` →
/// `0x1403D25D0`); the same ATKP keeps the group's attack, another one ends it and makes a new
/// attack (new id, nothing hit yet), and an attack whose group is no longer asserted ends
/// (`0x14040F610` → `0x1403D0EB0`). So a new attack starts when the open window's ATKP or group
/// differs from the last frame's, or a window opens after a frame with none. Sora's Keyblade
/// windows (`P_EX100.mset` / `W_EX010.mset`, Checked (files)): 153 128 at 20..32, 147 at 38..47;
/// Dodge Slash 1007 / 1898 / 1899; Slide Dash 968 then 975 (back to back); Flash Step 1896 then 1897
/// (group 6); Horizontal Slash 963 at 12..22 and 23..30 (a frame apart: two attacks), 967;
/// Retaliating Slash 1582, 1583; Aerial Finish 915 at 18..31, 916 at 31..40 (back to back), 917,
/// 927. KH1's row makes one attack object for the whole motion, whose hit bit a target keeps once
/// hit (`0x1402BD480`), so before t673 only a foe's first hit of the motion landed. On a new attack
/// the row's attack objects are re-armed (`kh1rows::rearm_rows` → `rehit::new_attack`, KH1's own
/// re-arm); hits within one attack again after its row's interval (`rehit`). Each frame KH1 has
/// him (`takeover::physics`), before KH1's physics and collision passes: a few reads unless an
/// attack changes.
pub unsafe fn keyblade_rearm(actor: usize) {
    use character_core::WindowKind as W;
    let now = (|| {
        let m = current(actor)?;
        let clip = clip_of(m)?;
        let t: f32 = rd(actor + TIME_NOW);
        if !(t >= 0.0) {
            return None;
        }
        let f = t as u32;
        let open = clip.windows.iter().filter(|w| w.contains(f)).find_map(|w| match w.kind {
            // (t680) The right Keyblade's (weapon 1); the left one's object re-arms itself
            // (`lefthits::step`). Before, a left window opening re-armed the right object.
            W::Hitbox { attack, group, weapon: 1, .. } => Some((attack, group)),
            _ => None,
        });
        Some((m.motion, open, t))
    })();
    let Ok(mut g) = BLADE_ATTACK.lock() else { return };
    let Some((motion, open, t)) = now else {
        *g = None;
        return;
    };
    let same_play = |lm: u16, lt: f32| lm == motion && t >= lt;
    let Some((atk, group)) = open else {
        // (Between windows: the last attack is kept for the same play, marked closed; a new play
        // (another motion, or the time gone back) forgets it.)
        *g = match *g {
            Some((lm, la, lg, lt, _)) if same_play(lm, lt) => Some((lm, la, lg, t, false)),
            _ => None,
        };
        return;
    };
    let new = match *g {
        Some((lm, la, lg, lt, was_open)) if same_play(lm, lt) => !was_open || la != atk || lg != group,
        _ => false,
    };
    *g = Some((motion, atk, group, t, true));
    if new {
        let n = super::kh1rows::rearm_rows(actor);
        static N: AtomicU32 = AtomicU32::new(0);
        if super::log_on(super::LogCat::Combat) && N.fetch_add(1, Ordering::Relaxed) < 40 {
            log!(Combat, "KH2's next attack on the Keyblade (KH2 motion {motion}, ATKP {atk} group {group}, frame {t:.0}): {n} attack object(s) of ours re-armed with KH1's re-arm");
        }
    }
}

/// KH2's cast motion our slot plays for KH1's spell request `request` (`CASTS`), if built.
pub fn cast_motion(request: usize) -> Option<u16> {
    let b = built()?;
    b.moves.iter().find(|m| m.request == request && m.default && matches!(m.kind, Kind::Magic | Kind::AirMagic) && live(b, m)).map(|m| m.motion)
}

/// KH2's `motion` has a slot of ours on one of KH1's requests (built: an ability row's only when
/// it is a Keyblade swing, `swing_rows`).
pub fn has_any(motion: u16) -> bool {
    built().is_some_and(|b| b.moves.iter().any(|m| m.motion == motion))
}

/// KH2's `motion` baked into a KH1 motion on `template` (`kh1_data::bake`), with its report logged.
/// `travel`: the move's request has KH1's travel flag (`travels`): KH2's root travel goes in its H10.
/// One bake's key in the motion slot cache (`slotcache`): what `bake_one` reads besides the
/// manifest's files and settings (the KH1 template motion's bytes, the KH2 motion, its variant,
/// length, KH1 events and travel). A Drive Form's motion (`form`: Valor's, the same motion ids as
/// Sora's, other animations) has the moveset in its key too; base Sora's keys are as before.
fn slot_key(template: &[u8], (motion, field): (u16, bool), frames: u32, tracks: &[EventTrack], travel: bool, form: Option<super::form::Kind>) -> u128 {
    let mut h = crate::slotcache::Hasher::default();
    h.bytes(template).u32(motion as u32).u32(field as u32).u32(frames).u32(travel as u32).str(&format!("{tracks:?}"));
    // (Valor's keys as before: its model's name, "P_EX100_BTLF".)
    if let Some(k) = form {
        h.str(k.model());
        // (The form's own Keyblade hold on KH1's grip, `pose::form_grip`: the forms' bakes once.)
        h.str("form grip");
        // (t829: Valor's Keyblade from KH2's weapon bone 2, `pose::kh2_hold`.)
        if k == super::form::Kind::Valor {
            h.str("kh2 hold");
            // (t851: the hold placed from the middle knuckle.)
            h.str("knuckle anchor");
        }
    }
    h.finish()
}

fn bake_one(b: &pose::Baker, src: &pose::Src, template: &[u8], (motion, field): (u16, bool), frames: u32, tracks: &[EventTrack], travel: bool) -> Result<Vec<u8>, String> {
    let t = std::time::Instant::now();
    let (poses, moved) = b.poses_of(src, motion, field, frames, travel).ok_or("KH2 has no animation for it")?;
    // (t-legs) KH1's own leg IK and ground lift kept on the baked motion (`kh1_data::legs`, ini
    // `kh1_leg_ik`): the template's leg joint words stay, its foot helpers H8 / H9 keyed to the
    // baked ankles and feet, so KH1's pass puts his feet on slopes and stairs in our motions as in
    // its own (`docs/kh1-anim-baking.md` §8).
    let legs = super::settings::get().kh1_leg_ik;
    let (block, r) = kh1_data::bake::baked(template, frames, tracks, &b.skeleton, &poses, &moved, legs).map_err(|e| e.to_string())?;
    log!(Animation,
        "KH2 slots: KH2 motion {motion} baked ({frames} frames, {} bytes, in {:.0} ms): {} keyed channels, {} keys, {} constants, {} bones built once; travel {} ({} H10 keys); KH1's legs {} ({} helper keys; KH1's hip aim {:.0} deg off the retarget's hip, thigh / shin within {:.2} deg); KH1's pass from it within {:.3} units of the pose (worst bone {})",
        block.len(),
        t.elapsed().as_secs_f32() * 1000.0,
        r.keyed_channels,
        r.keys,
        r.constant_channels,
        r.static_bones,
        moved.last().map_or("none (KH1 doesn't move it)".to_string(), |v| format!("{:.1?} model units", v.to_array())),
        r.travel_keys,
        if legs { r.kh1_legs.to_string() } else { "off (kh1_leg_ik)".to_string() },
        r.leg_keys,
        r.leg_hip_turn.to_degrees(),
        r.leg_turn.to_degrees(),
        r.max_error,
        r.worst_bone
    );
    Ok(block)
}

/// The KH2 clip's sounds (frame trigger 8), voices (13 / 14) and effect elements (1) as KH1 sound
/// event keys (type 6) of ours (`ks::KH2_SOUND` / `KH2_VOICE` in `a`, the KH2 motion in `b`, the
/// value in `c`; effects as one `ks::KH2_EFFECTS` key per frame, `c` the frame and the moveset), at
/// KH2's frames (`ks::kh2_trigger_time`: the slot's time is KH2's frame, 60 Hz, as the bake keeps
/// the clip's length). A hit reaction's KH2 voice, 29 / 30 at frame 0 (Checked (files)), is left
/// out: KH1's hit pass plays its own hurt voice for it, `0x1402901F0 (actor, 0 | 1, 0)` (Checked
/// (decomp)). (KH2's footsteps, trigger 2, aren't here: a baked slot keeps the template's own KH1
/// foot effects and floor sounds, `kh1_tracks`.)
///
/// Effects (t-vfx): KH2's trigger pass fires every trigger at a frame in the one update (Steam KH2
/// `0x14040E960`, no limit) and trigger 1 starts the element from the motion's owner's own effect
/// package (`0x14040DD70` case 1: the object's `+0x148`, `0x1402C6B30`; Checked (decomp)). Sora's
/// Drive motion 52 starts 13 elements at frame 0 and Valor's arrival 53 fifteen (Checked (files)),
/// past KH1's 8 edges a call (`ks::EDGES_PER_CALL`): one key per frame keeps them all, and the
/// moveset tag (`form`: Valor's slot) sends them to Valor's own package (`form::effect`), not
/// Sora's.
///
/// KH2's pad vibration (frame trigger 31, the tester's answer 9 (a) after m14) is a key too
/// (`ks::KH2_RUMBLE`, the pattern's id), every baked slot that carries one: KH1's own vibration
/// player plays KH2's pattern (`rumble`).
fn kh2_trigger_keys(motion: u16, clip: &character_core::Clip, reaction: bool, form: bool) -> Vec<ks::EventKey> {
    // (The Aerial Dodge's KH2 start skips frame 1, `combomods::dodge_clock`: its frame 0 and 1
    // triggers are keyed for the update from 1, `ks::kh2_trigger_time_skipped`.)
    let at = |frame: u32| if motion == character_core::motion::AERIAL_DODGE { ks::kh2_trigger_time_skipped(frame) } else { ks::kh2_trigger_time(frame) };
    let mut keys: Vec<ks::EventKey> = clip
        .events
        .iter()
        .filter_map(|e| {
            let (a, c) = match e.kind {
                EventKind::Sound(id) => (ks::KH2_SOUND, id),
                EventKind::Voice(v) if !reaction => (ks::KH2_VOICE, v as u16 as u32),
                // (KH2's pad vibration, frame trigger 31: KH1's own vibration player, `rumble`.)
                EventKind::Rumble(n) => (ks::KH2_RUMBLE, n as u16 as u32),
                _ => return None,
            };
            Some(ks::EventKey { time: at(e.frame), a, b: motion, c, d: 0 })
        })
        .collect();
    let mut frames: Vec<u32> = clip.events.iter().filter(|e| matches!(e.kind, EventKind::Effect(_))).map(|e| e.frame).collect();
    frames.sort_unstable();
    frames.dedup();
    keys.extend(frames.into_iter().map(|f| ks::EventKey { time: at(f), a: ks::KH2_EFFECTS, b: motion, c: ks::kh2_effects_value(f, form), d: 0 }));
    keys.sort_by(|x, y| x.time.total_cmp(&y.time));
    keys
}

/// The same triggers as KH2 events by frame (off Steam: `tick` reads them).
fn kh2_trigger_events(clip: &character_core::Clip, reaction: bool) -> Vec<(u32, EventKind)> {
    clip.events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Sound(_) | EventKind::Effect(_)) || matches!(e.kind, EventKind::Voice(_)) && !reaction)
        .map(|e| (e.frame, e.kind))
        .collect()
}

/// A `ks::KH2_EFFECTS` key fired on Sora (`sound_event`): every effect trigger of KH2 `motion` at
/// that frame, from the moveset's clip (Valor's when tagged), started from that moveset's package.
unsafe fn effects_at(motion: u16, c: u32) {
    let (frame, form) = ks::kh2_effects_of(c);
    let Some(kh2) = pose::kh2() else { return };
    // (A form's slots play only in that form, `redirect`: its clip is the form's he is in.)
    let character = if form { super::form::native_form().and_then(|k| k.data(kh2)).map(|f| &f.character) } else { Some(&kh2.character) };
    let Some(clip) = character.and_then(|c| c.moveset.clips.get(&motion)) else { return };
    for e in clip.events.iter().filter(|e| e.frame == frame) {
        if let EventKind::Effect(n) = e.kind {
            if form {
                super::form::effect(motion, n);
            } else {
                super::kh2sound::event(motion, e.kind);
                super::kh2fx::event(motion, e.kind);
            }
        }
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if super::log_on(super::LogCat::Effects) && N.fetch_add(1, Ordering::Relaxed) < 40 {
        let els: Vec<i16> = clip.events.iter().filter(|e| e.frame == frame).filter_map(|e| if let EventKind::Effect(n) = e.kind { Some(n) } else { None }).collect();
        log!(Effects, "KH2 slots: KH1's motion pass fired {} motion {motion}'s effects at frame {frame}: elements {els:?}{}", if form { "the form's" } else { "Sora's" }, if form && super::form::native_form().is_none() { " (not in the form: left out)" } else { "" });
    }
}

/// One of our sound event keys (`kh2_trigger_keys`) back as the KH2 event it was; `None` for KH1's.
fn kh2_trigger(a: i32, c: u32) -> Option<EventKind> {
    match a {
        a if a == ks::KH2_SOUND as i32 => Some(EventKind::Sound(c)),
        a if a == ks::KH2_EFFECT as i32 => Some(EventKind::Effect(c as u16 as i16)),
        a if a == ks::KH2_VOICE as i32 => Some(EventKind::Voice(c as u16 as i16)),
        _ => None,
    }
}

/// KH1's events for a KH2 clip on a retimed template (`r`: new length / the template's): the
/// clip's hit windows on the Keyblade as KH1's hit window (attack group 0, as KH1's own combo
/// motions: its row's one sphere on the blade), its combo window, its trail (else its Keyblade hit
/// windows from 4 frames before); the template's sounds, foot effects, face changes and the point
/// its action may be left, rescaled. Hit windows on his body (`body_strike`) are not KH1's sphere's:
/// each is KH1's hit window of its own attack group (`body_windows`: 1, 2, 3 in the clip's order),
/// which KH1 attack objects of ours with KH2's shapes are tied to (`bodyhits`).
///
/// A spell's cast (`cast`, `CASTS`): no "may be left" point of the template's (Blizzard's 55 has
/// one at 68 of 86, Fire's second part 57 at 46 of 60, Checked (files)): KH2's cast motions hold
/// him to their end (none has a free range, id 0 / 1; `P_EX100.mset`, Checked (files)), where
/// KH1's queue takes him on. (KH2's magic-only window, range id 35, is not KH1's yet.)
fn kh1_tracks(template: &[EventTrack], clip: &character_core::Clip, r: f32, leave: Leave) -> Vec<EventTrack> {
    let len = clip.length as f32;
    let span = |w: &character_core::Window| (w.start as f32, w.end.map_or(len, |e| e as f32));
    // (t680) The right Keyblade's windows (weapon 1) only: Valor's left Keyblade's (weapon 2)
    // are a KH1 attack object of their own on their own attack group (`left_channel`, `bodyhits`).
    let hits: Vec<(f32, f32)> = clip.windows.iter().filter(|w| matches!(w.kind, WindowKind::Hitbox { weapon: 1, .. })).map(span).collect();
    let combo: Vec<(f32, f32)> = clip.windows.iter().filter(|w| matches!(w.kind, WindowKind::Combo)).map(span).collect();
    let mut trail: Vec<(f32, f32)> = clip.windows.iter().filter(|w| matches!(w.kind, WindowKind::WeaponTrail { .. })).map(span).collect();
    if trail.is_empty() {
        trail = hits.iter().map(|&(a, b)| ((a - 4.0).max(0.0), b)).collect();
    }
    // On a request of ours (`own`, `kh1rows`), KH2's "free" state (range id 0 / 1 / 2,
    // `WindowKind::Grounded` / `Air` / `Free`: control comes back there, Aerial Recovery's from
    // frame 24, the guard's from 52, its repel's from 40;
    // `docs/aerial-recovery.md`) as KH1's "may be left" (type 0, `+0x1CC` bit 20: `0x14029D399`
    // clears the action flags `0x1004`) from its start, in place of the template's; a clip with
    // none keeps the template's, as every slot on KH1's own requests does.
    let free: Vec<(f32, f32)> = if leave == Leave::Template { Vec::new() } else { clip.windows.iter().filter(|w| matches!(w.kind, WindowKind::Air | WindowKind::Grounded | WindowKind::Free)).map(span).collect() };
    let keep_template = leave == Leave::Template || (leave == Leave::Kh2OrTemplate && free.is_empty());
    let mut out: Vec<EventTrack> = ks::scaled(template, r).into_iter().filter(|t| matches!(t.kind, ks::FOOT | ks::SOUND | ks::FACE) || (t.kind == ks::CAN_LEAVE && keep_template)).collect();
    if !free.is_empty() {
        out.push(EventTrack::level(ks::CAN_LEAVE, 0, &free));
    }
    for (kind, w) in [(ks::HIT, hits), (ks::COMBO, combo), (ks::TRAIL, trail)] {
        if !w.is_empty() {
            out.push(EventTrack::level(kind, 0, &w));
        }
    }
    // KH2's hit windows on his body, each on KH1's hit window of its own attack group (`+0x1CC` bit
    // `0x10000 << group`, which a motion-tied attack object of that group needs, `0x1402B47A0`).
    for b in body_windows(clip) {
        let end = b.end.map_or(len, |e| e as f32);
        out.push(EventTrack::level(ks::HIT, b.channel as u16, &[(b.start as f32, end)]));
    }
    // (t680) The second Keyblade's hit windows (weapon 2: Valor's left), all on one KH1 attack
    // group after the body windows' (`left_channel`): the KH1 attack object `bodyhits` makes for
    // the left Keyblade is tied to it, as KH1's own Keyblade object is to group 0.
    if let Some(ch) = left_channel(clip) {
        let w: Vec<(f32, f32)> = clip.windows.iter().filter(|w| matches!(w.kind, WindowKind::Hitbox { weapon, .. } if weapon >= 2)).map(span).collect();
        out.push(EventTrack::level(ks::HIT, ch as u16, &w));
    }
    // The second Keyblade's own trail windows (its weapon motion's range id 12) as a level event of
    // ours, 7.1, beside KH1's 7.0 (the right trail, bit 23): KH1's level reader `0x140394770` finds
    // it by type and id, and vanilla KH1 never asks for 7.1 (its pass reads only 7.0, and its edge
    // reader `0x140394900` only types 2, 3, 4 and 6; Steam, Checked (decomp)). `leftblade` reads it
    // with KH1's reader for the left Keyblade's holder's trail bit.
    let left_trail: Vec<(f32, f32)> = clip.windows.iter().filter(|w| matches!(w.kind, WindowKind::WeaponTrail { weapon } if weapon >= 2)).map(span).collect();
    if !left_trail.is_empty() {
        out.push(EventTrack::level(ks::TRAIL, LEFT_TRAIL_ID, &left_trail));
    }
    // (t868) KH2's invulnerable ranges (range id 27, `WindowKind::Invulnerable`) as a level event
    // of ours, 8.0 (`ks::KH2_RANGE`): on from the range's start, off at its end, as KH2's
    // `0x14040E740` takes a range (start <= t < end; no end: to the motion's end). Read with KH1's
    // own level reader on the motion KH1 plays (`kh1_level`, `invulnerable_step`).
    let invulnerable: Vec<(f32, f32)> = clip.windows.iter().filter(|w| matches!(w.kind, WindowKind::Invulnerable)).map(span).collect();
    if !invulnerable.is_empty() {
        out.push(EventTrack::level(ks::KH2_RANGE, ks::RANGE_INVULNERABLE, &invulnerable));
    }
    // (t869) KH2's no-gravity ranges (range ids 3 / 24) as our level event 8.1, the same way
    // (`no_gravity_spans`: none for a clip that allows movement all through), read by `no_gravity`.
    let hold = no_gravity_spans(clip);
    if !hold.is_empty() {
        out.push(EventTrack::level(ks::KH2_RANGE, ks::RANGE_NO_GRAVITY, &hold));
    }
    // (t871) KH2's edge stop (range id 29) as 8.2, read by `edge_stop`, and the ranges KH2's ground
    // mover reads as "don't leave the ground" (no gravity 3 / 24 on every clip, or the edge stop) as
    // 8.3, read by `ground_hold` (`ground_hold_spans`).
    let edge: Vec<(f32, f32)> = clip.windows.iter().filter(|w| matches!(w.kind, WindowKind::EdgeStop)).map(span).collect();
    if !edge.is_empty() {
        out.push(EventTrack::level(ks::KH2_RANGE, ks::RANGE_EDGE_STOP, &edge));
    }
    let ground = ground_hold_spans(clip);
    if !ground.is_empty() {
        out.push(EventTrack::level(ks::KH2_RANGE, ks::RANGE_GROUND_HOLD, &ground));
    }
    // (t872) KH2's allow-movement range (range id 41) as 8.4, read by `allow_movement` (the
    // forms' ground steering in their actions, `shared::physics::ground_inertia`, and their air
    // steering, `form_air_steer`).
    let steer: Vec<(f32, f32)> = clip.windows.iter().filter(|w| matches!(w.kind, WindowKind::AllowMovement)).map(span).collect();
    if !steer.is_empty() {
        out.push(EventTrack::level(ks::KH2_RANGE, ks::RANGE_ALLOW_MOVEMENT, &steer));
    }
    // (t878) KH2's turn toward the target (range id 28) as 8.5, each range's rate (radians) in its
    // key's `c` (`EventTrack::valued`: KH2's rate changes between ranges of one motion), read by
    // `turn_to_lock`.
    let turn: Vec<(f32, f32, u32)> = clip
        .windows
        .iter()
        .filter_map(|w| match w.kind {
            WindowKind::TurnToLock(rate) if rate.is_finite() => {
                let (a, b) = span(w);
                Some((a, b, rate.to_bits()))
            }
            _ => None,
        })
        .collect();
    if !turn.is_empty() {
        out.push(EventTrack::valued(ks::KH2_RANGE, ks::RANGE_TURN_TO_LOCK, &turn));
    }
    out
}

/// KH2's "don't leave the ground" ranges of `clip` as our level event 8.3 takes them (frames,
/// start..end): no gravity (range ids 3 / 24, `WindowKind::NoGravity`, every clip) or the edge stop
/// (29, `WindowKind::EdgeStop`), the ranges KH2's ground mover reads (`ground_hold`).
fn ground_hold_spans(clip: &character_core::Clip) -> Vec<(f32, f32)> {
    let len = clip.length as f32;
    clip.windows.iter().filter(|w| matches!(w.kind, WindowKind::NoGravity | WindowKind::EdgeStop)).map(|w| (w.start as f32, w.end.map_or(len, |e| e as f32))).collect()
}

/// KH1's motion event level reader (Steam `0x140394770 (motion, type << 16 | id, time f32) -> key`,
/// the last key of that type and id at or before the time; its pose pass reads 0.0, 1.x, 5.0 and
/// 7.0 with it; Checked (decomp)) and its first bytes (Checked (exe), as `leftblade` checks them).
const LEVEL_READER_STEAM: usize = 0x394770;
const LEVEL_READER_PROLOGUE: [u8; 5] = [0x48, 0x89, 0x5C, 0x24, 0x10];
/// The actor's motion table handle (`+0x1D4`; the motion of slot `+0x168` is its entry there).
const ACTOR_MOTIONS: usize = 0x1D4;
/// The reader's first bytes checked: 0 not yet, 1 as expected, 2 not (nothing is read then).
static LEVEL_READER_OK: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// (t868, t878) The key of a level event of the motion KH1 plays on `actor` now (its slot `+0x168`
/// in its table `+0x1D4`) at KH1's time in it (`+0x16C`), from KH1's own level reader: the last key
/// at or before the time, as a pointer into the motion's own key list (16 bytes: time, `a`, `b`,
/// `c`, `d`; Checked (decomp): `0x140394770` returns the key's address, 0 before the first key).
/// `Some(None)` when there is no such key.
/// While a blend runs KH1 holds that time at the new motion's start (`0x14029F0A0`, Checked
/// (decomp)), as KH2 reads its ranges at a blending motion's start time (`0x1403C6740`: `+0x4C`
/// while `+0x48` > 0, Checked (decomp)). Only on one of our slots (`current`), whose pointers are
/// KH1's own; `None` off Steam, off our slots, or with the reader not where expected.
pub unsafe fn kh1_key(actor: usize, kind: u16, id: u16) -> Option<Option<usize>> {
    if super::addr::Build::current() != super::addr::Build::Steam {
        return None;
    }
    current(actor)?;
    let base = super::takeover::game_base();
    match LEVEL_READER_OK.load(Ordering::Relaxed) {
        0 => {
            let ok = std::slice::from_raw_parts((base + LEVEL_READER_STEAM) as *const u8, LEVEL_READER_PROLOGUE.len()) == LEVEL_READER_PROLOGUE;
            LEVEL_READER_OK.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
            if !ok {
                log!(Animation, "KH2 slots: KH1's motion event reader not where expected: our level events (KH2's ranges) aren't read");
                return None;
            }
        }
        1 => {}
        _ => return None,
    }
    let table = pose::ptr(rd::<u32>(actor + ACTOR_MOTIONS));
    if table < 0x10000 {
        return None;
    }
    let slot = rd::<u16>(actor + SLOT_NOW) as usize;
    let motion = pose::ptr(rd::<u32>(table + slot * 4));
    let time: f32 = rd(actor + TIME_NOW);
    if motion < 0x10000 || !time.is_finite() {
        return None;
    }
    let reader: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + LEVEL_READER_STEAM);
    let key = reader(motion, (kind as u32) << 16 | id as u32, time);
    Some((key >= 0x10000).then_some(key))
}

/// (t868) A level event of the motion KH1 plays on `actor` now, read by KH1's own level reader
/// (`kh1_key`): whether it is on (the key's `a`).
pub unsafe fn kh1_level(actor: usize, kind: u16, id: u16) -> Option<bool> {
    Some(kh1_key(actor, kind, id)?.is_some_and(|key| rd::<u8>(key + 4) != 0))
}

/// The id of our left-Keyblade trail level event (type 7, `ks::TRAIL`; KH1's own trail is 7.0).
pub const LEFT_TRAIL_ID: u16 = 1;
/// Its type (KH1's trail level, `ks::TRAIL`).
pub const LEFT_TRAIL_KIND: u16 = ks::TRAIL;

/// (t680) The KH1 attack group the clip's second-Keyblade hit windows (weapon 2, Valor's left
/// Keyblade) play on: the first after its body windows' (`body_windows`: 1..), if one is left
/// (KH1 has groups 1..3 besides the Keyblade's 0). None when the clip has no such window, or no
/// group is free (none of Valor's moves has body windows, `P_EX100_BTLF.mset`, Checked (files)).
pub fn left_channel(clip: &character_core::Clip) -> Option<u8> {
    if !clip.windows.iter().any(|w| matches!(w.kind, WindowKind::Hitbox { weapon, .. } if weapon >= 2)) {
        return None;
    }
    let ch = body_windows(clip).len() + 1;
    (ch <= BODY_CHANNELS).then_some(ch as u8)
}

/// (t680) The KH2 motion our slot plays on `actor` now, its clip (Valor's in Valor on KH1's own
/// states, `clip_of`) and KH1's time in it (frames).
pub unsafe fn playing_clip(actor: usize) -> Option<(u16, &'static character_core::Clip, f32)> {
    let m = current(actor)?;
    let clip = clip_of(m)?;
    let t: f32 = rd(actor + TIME_NOW);
    (t.is_finite() && t >= 0.0).then_some((m.motion, clip, t))
}

/// One of KH2's hit windows on Sora's own body (a `Hitbox` window with weapon 0) and the KH1 attack
/// group it is played on (`kh1_tracks`, `bodyhits`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyWindow {
    /// Its index in the clip's windows.
    pub index: usize,
    pub attack: u16,
    /// KH2's collision group of his body shapes it switches on.
    pub group: u8,
    pub start: u32,
    pub end: Option<u32>,
    /// KH1's attack group (1..=3; KH1's 0 is the Keyblade's).
    pub channel: u8,
}

/// KH1 has four attack groups a motion's events switch (`+0x1CC` bits 16..19); 0 is the Keyblade's.
pub const BODY_CHANNELS: usize = 3;

/// The clip's hit windows on his body, in the clip's order, each with its KH1 attack group (1, 2,
/// 3; a fourth and later are left out: none of Sora's moves has more than three, Magnet Burst's
/// 1900 / 1901 / 1903, Checked (files)).
pub fn body_windows(clip: &character_core::Clip) -> Vec<BodyWindow> {
    clip.windows
        .iter()
        .enumerate()
        .filter_map(|(index, w)| match w.kind {
            WindowKind::Hitbox { attack, group, weapon: 0, .. } => Some(BodyWindow { index, attack, group, start: w.start, end: w.end, channel: 0 }),
            _ => None,
        })
        .take(BODY_CHANNELS)
        .enumerate()
        .map(|(i, b)| BodyWindow { channel: i as u8 + 1, ..b })
        .collect()
}

/// Where KH1 may leave one of our motions early (KH1's "may be left", type 0, `+0x1CC` bit 20: its
/// control step walks, jumps, rolls or takes Square from there).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Leave {
    /// KH2's free state (range id 0 / 1 / 2) only; with none, nothing before the motion's end, as in
    /// KH2 (the tester, kh2-magic-and-reflect-native m8: "you can jump and dodge roll out of moves
    /// during its end frames ... so its exact"; Steam KH2's input step `0x1403D6FC0` takes a jump,
    /// and `0x1403BFB30` the guard, roll and Quick Run, only in a free state, Checked (decomp)):
    /// KH2's swings, ability moves and casts, none of which has one (`P_EX100.mset`, Checked
    /// (files)). KH1's action flag then clears at the motion's end: KH1's row runner
    /// (`0x1402A68F0`) and our own starts (`combomods`) queue the idle / fall after it with bit
    /// `0x40000000`, which `0x14029F6E0` takes as "clear the action flag 4" when the motion ends
    /// (Checked (decomp)). Before, KH2's swings on KH1's combo requests kept the KH1 template's own
    /// "may be left" point (KH1's swing's), and KH1 let him jump or roll out there.
    Kh2,
    /// KH2's free state, else the template's (the reactions: KH1's reaction functions queue some
    /// of their idles without bit `0x40000000`, `0x14029FA30 (actor, +0x3EA + 6, 0)`, so the
    /// template's point is what lets him go there).
    Kh2OrTemplate,
    /// The template's (KH1's own point: glides, cycles, the roll before its own override).
    Template,
}

/// How much earlier KH1's "may be left" comes on the Aerial Dodge's slot than KH2's free frame
/// (the tester, aerial-dodge-native after m4: the glide out of the dodge came later than in KH2).
/// KH2's player frame (`0x1403BFD30`, Steam KH2) runs the motion update (`0x1403C6740`, which sets
/// the range bits of `+0x18C` from the frame the clip reaches) before the input step (PLAYER's
/// vtable `+0x20`, `0x1403A7090` -> `0x1403A89A0` -> `0x1403D6FC0`, whose action button runs the
/// glide start `0x1403A81F0` only with a free bit `+0x18C & 0x1C`, `0x1403BFB30`): Square glides on
/// the very frame 202 reaches its air-free frame 4 (Checked (decomp); the vtable by RTTI, Checked
/// (exe)). KH1's actor frame runs its control step before its animation update (`0x140295E50`),
/// so a "may be left" fired at frame F is seen by the control step one frame later: at F - 1 it
/// is seen on the frame KH2's input sees F.
const INPUT_LEAD: f32 = 1.0;

/// The frame KH2's dodge roll may be left from: its free state (range id 2, frame 36 of Sora's 209,
/// `Clip::first_free`; KH2 takes a jump or the action button only there, Checked (decomp) and
/// (files)), else its fall-check event (34, the OG rewrite's reading), else its last frame. KH1's "may be left" (type 0) from there clears the action flags, so KH1's
/// control step walks, jumps, rolls or attacks out of it.
fn roll_free(clip: &character_core::Clip) -> u32 {
    let len = clip.length.max(1);
    clip.first_free().or_else(|| clip.events.iter().find(|e| e.kind == EventKind::FallCheck).map(|e| e.frame)).unwrap_or(len - 1).min(len - 1)
}

/// What may follow a KH2 cast on KH1's cast state (the tester, thread kh2-magic-and-reflect-native,
/// m3 answer 1 (a): a cast counts as a step; m9 answer 2 (a): Attack out of it exactly as KH2).
/// Steam KH2's Attack check `0x1403D6CA0` takes an Attack only in the combo window (range id 11,
/// `+0x18C` bit 0x400; the combo goes on), the finisher window (range id 50, `+0x194` bit 0x20,
/// with Finishing Plus left) or a free state (range ids 0 / 1 / 2; a new combo, count 1); the
/// reaction-command window (range id 14) is read only for reaction-command rows (Checked
/// (decomp)). Range id 35 is the combo window too, while the motion playing is the one it names
/// (`combo_kind`, t642); none of Sora's casts has a free state (Checked (files), `P_EX100.mset`).
/// A spell is taken in the same window (KH2's magic command, `0x1403D6FC0`).
/// On KH1's side: the motion's combo window (type 5, `+0x1CC` bit 22, in which KH1's menu takes a
/// command with the action flag still set, `0x1402A2EC0`, and KH1's Attack step `0x1402A7510`
/// counts on from `+0x41`, Checked (decomp)) over KH2's combo window; `combat::command` refuses an
/// Attack (`cast_attack_window`) and a spell (`cast_magic_window`) outside it. Outside it, KH1's Attack step waits for the cast's end and then starts a new combo (count 1), as
/// KH2's free state does. No "may be left" (type 0 clears the action flags, `0x14029D399`, and
/// the stick would walk out of the cast, which KH2's doesn't). A fresh jump or Square ends the
/// action flag for its own move only in a free state (`cast_cut_open`, `formmagic::cast_cut`):
/// never during Sora's casts. A finisher cast that ends the combo has no combo window: KH1's
/// Attack starts a new one (KH2's finisher window with Finishing Plus is not built).
fn cast_follow(clip: &character_core::Clip, finisher: bool) -> Vec<EventTrack> {
    if finisher {
        return Vec::new();
    }
    let len = clip.length.max(1) as f32;
    let spans: Vec<(f32, f32)> = clip
        .windows
        .iter()
        .filter(|w| combo_kind(clip, &w.kind))
        .map(|w| (w.start as f32, w.end.map_or(len, |e| e as f32).min(len)))
        .filter(|(a, b)| b > a)
        .collect();
    if spans.is_empty() {
        return Vec::new();
    }
    vec![EventTrack::level(ks::COMBO, 0, &spans)]
}

/// A cast's window kind that is KH2's combo window on it (Steam KH2's range step `0x14040D810`,
/// Checked (decomp)): range id 11 (`Combo`), or range id 35 naming this very motion
/// (`ComboMagic(motion)`; its parameter is compared with the motion playing, so the copies of
/// another motion's ranges a cast carries are inert: Reflect 71 has Fire's [56] from 60 and its own
/// [71] 34..46, Cure 65 Thunder's [62] from 42, Checked (files) `P_EX100.mset`). Sora's ground casts
/// then (60 Hz frames): Fire 56 from 60, Blizzard 59 from 36, Thunder 62 from 42, Reflect 71 34..46,
/// Cure 65 none; KH2's Attack (with the press's "connected" or Combo Master) and spells
/// (`0x1403D6FC0`: the magic command always with the window test on) are taken there alike
/// (`0x1403D6CA0`). Before t642 every range 35 was taken as a spells-only window from its start,
/// whatever its parameter (the tester, combo row selection after m29: "you can combo kh2 magic
/// into each other faster than going from a magic into attack ... its not 1:1").
fn combo_kind(clip: &character_core::Clip, k: &WindowKind) -> bool {
    match *k {
        WindowKind::Combo => true,
        WindowKind::ComboMagic(m) => m == clip.motion,
        _ => false,
    }
}

/// KH2's finisher window (range id 50) on its motion `motion`, as shares of the motion's length
/// (Thunder's finisher cast 63 and Magnet's 68: 42..78, Checked (files)), for Gravity and Stop as
/// the combo's finisher (`formmagic::finisher_spell_window`).
pub fn finisher_share(motion: u16) -> Option<(f32, f32)> {
    let clip = pose::kh2_moves()?.moveset.clips.get(&motion)?;
    let w = clip.windows.iter().find(|w| matches!(w.kind, WindowKind::Finisher))?;
    let len = clip.length.max(1) as f32;
    Some((w.start as f32 / len, w.end.map_or(len, |e| e as f32) / len))
}

/// Where KH2's combo window on its motion `motion` opens, as a share of the motion's length
/// (`combo_kind`'s first frame; Fire 56: frame 60, Checked (files)), for KH1's own casts at the
/// Magic Combo row (`formmagic::kh1_cast_window`).
pub fn combo_share(motion: u16) -> Option<f32> {
    let clip = pose::kh2_moves()?.moveset.clips.get(&motion)?;
    let from = clip.first(|k| combo_kind(clip, k))?;
    (clip.length > 0).then(|| from as f32 / clip.length as f32)
}

/// Our cast slot playing on `actor`: whether an Attack may follow it now (KH2's combo window,
/// `combo_kind`: range id 11, or range id 35 naming this motion; Steam KH2 `0x1403D6CA0`, Checked (decomp)). `None`: no cast
/// of ours plays.
pub unsafe fn cast_attack_window(actor: usize) -> Option<bool> {
    let m = current(actor).filter(|m| matches!(m.kind, Kind::Magic | Kind::AirMagic))?;
    // (Gravity and Stop, the combo's finisher: only their Finishing Plus window, t644.)
    if let Some(open) = super::formmagic::finisher_spell_window(actor) {
        return Some(open);
    }
    let clip = clip_of(m)?;
    let t: f32 = rd(actor + TIME_NOW);
    Some(t.is_finite() && t >= 0.0 && clip.has(t as u32, |k| combo_kind(clip, k)))
}

/// Our finisher cast playing on `actor` is in its Finishing Plus window now (the tester,
/// kh2-magic-and-reflect-native after m10, question 2 (a)): KH2's finisher window (range id 50,
/// `WindowKind::Finisher`; the ground finisher casts Fire 57 54..84, Blizzard 60 46..76, Thunder
/// 63 / Magnet's 68 42..78, Reflect 72 54..84, Checked (files) `P_EX100.mset`) with Finishing Plus's
/// next finisher left (`combomods::finisher_left`), base Sora on KH1's states. Steam KH2's Attack
/// and command check `0x1403D6CA0` takes an Attack, and a spell (command kind 2, the same check
/// from the input step `0x1403D6FC0`), there: the count and the finisher count one more
/// (Checked (decomp)). `finisher_window` opens KH1's combo window there.
pub unsafe fn cast_finisher_open(actor: usize) -> bool {
    let Some(m) = current(actor).filter(|m| matches!(m.kind, Kind::Magic | Kind::AirMagic)) else { return false };
    let Some(clip) = clip_of(m) else { return false };
    let t: f32 = rd(actor + TIME_NOW);
    t.is_finite() && t >= 0.0 && clip.has(t as u32, |k| matches!(k, WindowKind::Finisher)) && super::shared::active() && super::combomods::finisher_left()
}

/// One of our cast slots plays on `actor` (KH2's spells', Reflect's).
pub unsafe fn cast_playing(actor: usize) -> bool {
    current(actor).is_some_and(|m| matches!(m.kind, Kind::Magic | Kind::AirMagic))
}

/// Our cast slot playing on `actor` may be left by a jump or Square now: KH2's free state (range
/// id 0 / 1 / 2, `Clip::free_at`; Steam KH2 `0x1403D6FC0` / `0x1403BFB30`, Checked (decomp)), not
/// its combo window (`cast_attack_window`, which only an Attack takes). None of Sora's cast
/// motions (56..73) has one (`P_EX100.mset`, Checked (files)), so a jump or Square waits for the
/// cast's end, as in KH2.
pub unsafe fn cast_cut_open(actor: usize) -> bool {
    let Some(m) = current(actor).filter(|m| matches!(m.kind, Kind::Magic | Kind::AirMagic)) else { return false };
    let Some(clip) = clip_of(m) else { return false };
    let t: f32 = rd(actor + TIME_NOW);
    t.is_finite() && t >= 0.0 && clip.free_at(t as u32)
}

/// Our cast slot playing on `actor`: whether a spell may follow it now (KH2's combo window on it,
/// `combo_kind`: range id 11, or range id 35 naming this motion; none: not during the cast).
/// `None`: no cast of ours plays.
pub unsafe fn cast_magic_window(actor: usize) -> Option<bool> {
    let m = current(actor).filter(|m| matches!(m.kind, Kind::Magic | Kind::AirMagic))?;
    if let Some(open) = super::formmagic::finisher_spell_window(actor) {
        return Some(open);
    }
    let clip = clip_of(m)?;
    let t: f32 = rd(actor + TIME_NOW);
    // (KH2's magic command takes the same combo window as its Attack, `combo_kind`; t642.)
    Some(t.is_finite() && t >= 0.0 && clip.has(t as u32, |k| combo_kind(clip, k)))
}

/// One of Sora's motion sets with ours added (`docs/kh1-anim-slots.md` §2): KH1 binds a set onto
/// his actor (his own at creation, `0x140287E40`; a world's on entering it, `0x1402886F0` from
/// `0x14029E850`, his own saved at `+0x1E0`), and we hand it these copies instead, built once per
/// set: its own slots, then ours right after them; its request table, with the requests our moves
/// stand in for pointed at ours by their styles.
struct Set {
    /// KH1's own handles for its tables (to know the set again).
    kh1_slots: u32,
    kh1_requests: u32,
    /// Ours.
    slots_handle: u32,
    requests_handle: u32,
    requests: &'static mut [u32],
    /// (request, KH1's entry, our slot index in this set, the move's style, the move (its
    /// `slot`), whether it is the request's own, whether it is one of ours (a request of ours),
    /// the Drive Form whose it is).
    redirects: Vec<(usize, u32, u32, Kind, usize, bool, bool, Option<super::form::Kind>)>,
    /// The requests with a move of each Drive Form's (`redirect`: in that form those are its).
    form_requests: Vec<(super::form::Kind, Vec<usize>)>,
}

/// Our motions placed in memory (the game thread) and the sets made with them.
struct Placed {
    /// Each motion's address (ours, leaked) and its handle.
    motions: Vec<(usize, u32)>,
    sets: Vec<Set>,
}

static PLACED: Mutex<Option<Placed>> = Mutex::new(None);
/// Our motions' addresses, for `pose::slot_of` (slot by the instance's motion pointer).
static MOTION_AT: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
/// Each placed motion with a holder motion (Valor's left Keyblade, `holder_one`): (the motion, its
/// holder motion), as KH1 has them in memory.
static HOLDER_AT: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());

/// The holder motion placed for our motion at `motion` (KH1's address of the motion Sora plays),
/// if it has one (`leftblade::pose_holder`).
pub fn holder_motion(motion: usize) -> Option<usize> {
    if motion == 0 {
        return None;
    }
    HOLDER_AT.try_lock().ok()?.iter().find(|h| h.0 == motion).map(|h| h.1)
}

/// The holder motion of Valor's idle (KH2 motion 0), for a slot of KH1's own with none (`holder`,
/// t831: the left Keyblade held at KH2's Valor idle grip, KH2's left weapon staying on its palm),
/// else any Valor holder motion.
pub fn rest_holder() -> Option<usize> {
    let b = built()?;
    let at = MOTION_AT.try_lock().ok()?;
    let main = |m: &Move| at.iter().find(|x| x.1 == m.slot).map(|x| x.0);
    let valor = |m: &&Move| m.form == Some(super::form::Kind::Valor) && m.holder;
    let idle = b.moves.iter().filter(valor).find(|m| m.motion == character_core::motion::IDLE).and_then(main);
    drop(at);
    if let Some(h) = idle.and_then(holder_motion) {
        return Some(h);
    }
    HOLDER_AT.try_lock().ok()?.first().map(|h| h.1)
}

/// Our slot playing on Sora (`usize::MAX` none), and its KH2 motion (for `kh2fx::tick`).
static PLAYING: AtomicUsize = AtomicUsize::new(usize::MAX);
static PLAYING_MOTION: AtomicU32 = AtomicU32::new(u32::MAX);

fn built() -> Option<&'static Built> {
    BUILT.get()?.as_ref()
}

fn moved(slot: usize) -> Option<&'static Move> {
    built()?.moves.iter().find(|m| m.slot == slot)
}

/// KH2's clip of our move `m`: its Drive Form's moveset for one of a form's (`Move::form`; the
/// motion ids are the same as base Sora's), else base Sora's.
fn clip_of(m: &Move) -> Option<&'static character_core::Clip> {
    let k = pose::kh2()?;
    let c = match m.form {
        Some(kind) => &kind.data(k)?.character,
        None => &k.character,
    };
    c.moveset.clips.get(&m.motion)
}

/// `m` belongs to the moveset playing now: in a Drive Form on KH1's own states
/// (`form::native_form`) the form's where it has a move for that request, else base Sora's.
fn live(b: &Built, m: &Move) -> bool {
    let Some(now) = super::form::native_form() else { return m.form.is_none() };
    m.form == Some(now) || (m.form.is_none() && !b.moves.iter().any(|x| x.form == Some(now) && x.request == m.request))
}

/// The Drive Form our slot `slot` belongs to (`pose`: the form's rig for its KH2 pose).
pub fn form_of(slot: usize) -> Option<super::form::Kind> {
    moved(slot)?.form
}

/// Drive Form `kind`'s moves are built (it can run on KH1's own states: `form::native_ok`): its
/// idle, walk, run, fall and landing, its arrival, and base Sora's Drive motion.
pub fn form_built(kind: super::form::Kind) -> bool {
    built().is_some_and(|b| {
        let has = |r: usize| b.moves.iter().any(|m| m.form == Some(kind) && m.request == r);
        let arrive = super::kh1rows::by_motion(super::kh1rows::ARRIVE_MOTION).map(|n| super::kh1rows::request(n) as usize);
        let drive = super::kh1rows::by_motion(super::kh1rows::DRIVE_MOTION).map(|n| super::kh1rows::request(n) as usize);
        [0x0, 0x1, 0x2, 0x6, 0x7].iter().all(|&r| has(r)) && arrive.is_some_and(has) && drive.is_some_and(|d| b.moves.iter().any(|m| m.form.is_none() && m.request == d))
    })
}

/// `request` has a slot in the request table bound on `actor` now (its entry not 0xFFF, none):
/// asked of KH1 only then (`form`'s Drive and arrival).
pub unsafe fn request_bound(actor: usize, request: usize) -> bool {
    let t = pose::ptr(rd::<u32>(actor + SET + 8));
    t >= 0x10000 && super::crash::readable(t + request * 4 + 3) && rd::<u32>(t + request * 4) & 0xFFF != 0xFFF
}

/// The requests pointed again for the moveset now (a Drive Form's change: `form`), at once rather
/// than at Sora's next pose.
pub unsafe fn refresh() {
    // (A blocking lock: only the game thread takes it, and not around this call.)
    if let Ok(mut g) = PLACED.lock() {
        if let Some(p) = g.as_mut() {
            redirect(p);
        }
    }
}

/// Every slot of this kind of move is built (so KH1's state can own the whole move at KH2's
/// style, not just part of its combo).
pub fn covers(kind: Kind) -> bool {
    let Some(b) = built() else { return false };
    SPECS.iter().any(|s| s.kind == kind) && SPECS.iter().filter(|s| s.kind == kind).all(|s| b.moves.iter().any(|m| m.request == s.request))
}

/// `slot` is one of ours, baked into a KH1 motion (KH1 shows it; `pose` draws nothing over it).
pub fn is_baked(slot: usize) -> bool {
    moved(slot).is_some_and(|m| m.baked)
}

/// A request table entry (file layout) with KH1's travel flag: bit 15, which `0x1402A01E0`
/// turns into the request's `0x4000000` and `0x14029FC40` into the actor's `+0x160` bit 4: KH1
/// moves the actor by the motion's H10 (`0x14029A3C0`, `+0x200`) and pulls the model back by it
/// (`0x14029F0A0`, `+0x210`). Sora's ground combo (0xC8..0xCB) has it; his air combo doesn't.
pub fn travels(entry: u32) -> bool {
    entry & TRAVEL_FLAG != 0
}

/// KH1's travel flag on a request table entry (bit 15: KH1 moves the actor by the motion's H10).
const TRAVEL_FLAG: u32 = 0x8000;

/// Sora (`actor`) plays one of ours baked with KH2's travel: KH1 moves him by it (`shared`'s
/// swing travel stays out).
pub unsafe fn moves_itself(actor: usize) -> bool {
    current(actor).is_some_and(|m| m.travel)
}

/// Our slot plays KH2's field variant of its motion (`CYCLES`).
pub fn is_field(slot: usize) -> bool {
    moved(slot).is_some_and(|m| m.field)
}

/// Our slot's KH2 motion, if `slot` is one of ours.
pub fn kh2_motion(slot: usize) -> Option<u16> {
    moved(slot).map(|m| m.motion)
}

/// The KH1 slot ours stands in for (callers that test KH1's slots see that one).
pub fn stands_for(slot: usize) -> Option<usize> {
    moved(slot).map(|m| m.stands_for)
}

/// Our slot playing on Sora now, if any.
pub fn playing() -> Option<usize> {
    let s = PLAYING.load(Ordering::Relaxed);
    (s != usize::MAX).then_some(s)
}

/// The length of what plays for KH1's `slot` now, if ours stands in for it.
/// The length (frames) of our slot of KH2 `motion` (its first), if it was built.
pub fn frames_for_motion(motion: u16) -> Option<u32> {
    built()?.moves.iter().find(|m| m.motion == motion).map(|m| m.frames)
}

pub fn frames_for(slot: usize) -> Option<u32> {
    let m = moved(playing()?)?;
    (m.stands_for == slot || m.slot == slot).then_some(m.frames)
}

/// The KH2 motion of our slot playing, for `kh2fx::tick` while KH1 has him.
pub fn fx_motion() -> Option<u16> {
    let m = PLAYING_MOTION.load(Ordering::Relaxed);
    (m != u32::MAX).then_some(m as u16)
}

/// The slot of a motion of ours (the instance's motion pointer).
pub fn slot_of_motion(motion: usize) -> Option<usize> {
    MOTION_AT.try_lock().ok()?.iter().find(|(p, _)| *p == motion).map(|(_, s)| *s)
}

/// KH1's blend into the motion playing: its weight so far, while it blends.
pub unsafe fn kh1_blend(actor: usize) -> Option<f32> {
    let (len, clock) = (rd::<f32>(actor + BLEND_LEN), rd::<f32>(actor + BLEND_CLOCK));
    (len > 1.0 && clock.is_finite() && clock < len).then(|| (clock / len).clamp(0.0, 1.0))
}

/// The move of ours `actor` plays now: the motion its slot (`+0x168`) holds in the set bound
/// (`+0x1D4`), by its address (our slot numbers differ from set to set).
unsafe fn current(actor: usize) -> Option<&'static Move> {
    let t = pose::ptr(rd::<u32>(actor + SLOT_TABLE));
    let i = rd::<u32>(actor + SLOT_NOW) as usize;
    if t < 0x10000 || i > 0xFFF || !super::crash::readable(t + i * 4 + 3) {
        return None;
    }
    slot_of_motion(pose::ptr(rd::<u32>(t + i * 4))).and_then(moved)
}

/// KH1's time in the motion playing on `actor` now (frames, `+0x16C`), whatever slot it is (KH1's own
/// too): KH1's actor motion code adds to it each frame (Checked (exe): the motion step near
/// `0x14029F300` reads and writes `+0x16C`).
pub unsafe fn kh1_time_now(actor: usize) -> Option<f32> {
    if actor < 0x10000 {
        return None;
    }
    Some(rd::<f32>(actor + TIME_NOW)).filter(|t| t.is_finite() && *t >= 0.0)
}

/// KH1's time in our slot playing on `actor` (frames), if it is `slot` (our move's id).
pub unsafe fn kh1_time(actor: usize, slot: usize) -> Option<f32> {
    current(actor).filter(|m| m.slot == slot).map(|_| rd::<f32>(actor + TIME_NOW)).filter(|t| t.is_finite() && *t >= 0.0)
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

unsafe fn handle(p: usize) -> u32 {
    let f: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(super::takeover::game_base() + TO_HANDLE_RVA);
    f(p)
}

/// Our motions in memory, their offsets turned into handles as KH1's loader does (`0x1401D5CE0`).
/// Runs once (`PLACED` is never emptied again): the built blocks are taken and each freed once
/// copied (nothing reads them after this).
unsafe fn place(b: &Built) -> Placed {
    let mut motions = Vec::new();
    let mut at = Vec::new();
    let mut holders = Vec::new();
    let blocks = b.blocks.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default();
    let bytes: usize = blocks.iter().map(|b| b.len()).sum();
    for (m, block) in b.moves.iter().zip(blocks) {
        // (16-aligned, never freed.)
        let words: &'static mut [u128] = Box::leak(vec![0u128; block.len().div_ceil(16)].into_boxed_slice());
        let p = words.as_mut_ptr() as usize;
        std::ptr::copy_nonoverlapping(block.as_ptr(), p as *mut u8, block.len());
        // Event groups' key offsets first (the group list is found by +0x3C's offset).
        let mut g = p + rd::<u32>(p + ks::EVENTS) as usize;
        while rd::<u32>(g) != ks::END {
            wr(g + 8, handle(p + rd::<u32>(g + 8) as usize));
            g += 12;
        }
        for f in ks::OFFSET_FIELDS {
            wr(p + f, handle(p + rd::<u32>(p + f) as usize));
        }
        motions.push((p, handle(p)));
        at.push((p, m.slot));
        // Its holder motion (Valor's left Keyblade), placed the same way, found by this motion.
        if let Some(hb) = m.holder_block.lock().ok().and_then(|mut g| g.take()) {
            let words: &'static mut [u128] = Box::leak(vec![0u128; hb.len().div_ceil(16)].into_boxed_slice());
            let q = words.as_mut_ptr() as usize;
            std::ptr::copy_nonoverlapping(hb.as_ptr(), q as *mut u8, hb.len());
            let mut g = q + rd::<u32>(q + ks::EVENTS) as usize;
            while rd::<u32>(g) != ks::END {
                wr(g + 8, handle(q + rd::<u32>(g + 8) as usize));
                g += 12;
            }
            for f in ks::OFFSET_FIELDS {
                wr(q + f, handle(q + rd::<u32>(q + f) as usize));
            }
            holders.push((p, q));
        }
    }
    if let Ok(mut v) = MOTION_AT.lock() {
        *v = at;
    }
    if let Ok(mut v) = HOLDER_AT.lock() {
        *v = holders;
    }
    log!(Animation, "KH2 slots: {} motions placed ({bytes} bytes; the build's copy freed) (handles {:08x?}); {} left Keyblade holder motions", motions.len(), motions.iter().map(|m| m.1).collect::<Vec<_>>(), HOLDER_AT.lock().map_or(0, |v| v.len()));
    Placed { motions, sets: Vec::new() }
}

/// Sora's bone count (his model and motions).
const SORA_BONES: u32 = 293;

/// KH1's model on `actor` is Sora's (293 bones: the actor's instance `+0x15C`, its model slot,
/// the model header's count; every step checked, as the set bind hook sees other structures too).
unsafe fn is_sora(actor: usize) -> bool {
    let ok = |p: usize, n: usize| p >= 0x10000 && super::crash::readable(p) && super::crash::readable(p + n - 1);
    if !ok(actor, 0x1F0) {
        return false;
    }
    let inst = pose::ptr(rd::<u32>(actor + 0x15C));
    if !ok(inst, 0x10) {
        return false;
    }
    let slot = rd::<u64>(inst) as usize;
    if !ok(slot, 8) {
        return false;
    }
    let header = rd::<u64>(slot) as usize;
    ok(header, 2) && rd::<u16>(header) as u32 == SORA_BONES
}

/// The set bound at `at` (four handles: file, slot list, request table, a third section): its
/// file, slot list and request table addresses, its slot count and its request table's length,
/// checked.
unsafe fn read_set(at: usize) -> Result<(usize, usize, usize, usize, usize), String> {
    let (fh, sh, rh) = (rd::<u32>(at), rd::<u32>(at + 4), rd::<u32>(at + 8));
    let (f, s, r) = (pose::ptr(fh), pose::ptr(sh), pose::ptr(rh));
    if f < 0x10000 || s < 0x10000 || r < 0x10000 || !super::crash::readable(f + 0x14) || !super::crash::readable(s + 3) || !super::crash::readable(r + 3) {
        return Err(format!("handles {fh:08x} / {sh:08x} / {rh:08x} don't read"));
    }
    // Its slot count: the words before its first motion (KH1 keeps no end marker in memory; the
    // motions follow the list, t352).
    let mut first = usize::MAX;
    let mut i = 0usize;
    while i < 4096 && s + i * 4 < first {
        if !super::crash::readable(s + i * 4 + 3) {
            break;
        }
        let h = rd::<u32>(s + i * 4);
        if h & 0x8000_0000 != 0 {
            let m = pose::ptr(h);
            if m > s && m < first && super::crash::readable(m + 0x40) && rd::<u32>(m + 0x10) == SORA_BONES {
                first = m;
            }
        }
        i += 1;
    }
    if first == usize::MAX {
        return Err(format!("no motion in slot list {sh:08x}"));
    }
    let count = (first - s) / 4;
    // Its request table: the file's section 1 (sections at +4, +8, +0xC, +0x10).
    let (r0, r1) = (rd::<u32>(f + 8) as usize, rd::<u32>(f + 0xC) as usize);
    if r1 <= r0 || f + r0 != r || (r1 - r0) / 4 > REQUESTS * 8 || !super::crash::readable(r + (r1 - r0) - 1) {
        return Err(format!("request table {rh:08x} isn't the file's section 1 ({r0:#x}..{r1:#x})"));
    }
    Ok((f, s, r, count, (r1 - r0) / 4))
}

/// Ours for the set bound at `at` (made the first time), its handles written there. Whether ours
/// are there now.
unsafe fn extend(b: &Built, p: &mut Placed, at: usize, why: &str) -> bool {
    let (sh, rh) = (rd::<u32>(at + 4), rd::<u32>(at + 8));
    if p.sets.iter().any(|t| t.slots_handle == sh && t.requests_handle == rh) {
        return true;
    }
    if let Some(t) = p.sets.iter().find(|t| t.kh1_slots == sh && t.kh1_requests == rh) {
        wr(at + 4, t.slots_handle);
        wr(at + 8, t.requests_handle);
        return true;
    }
    if p.sets.len() >= 32 {
        return false;
    }
    let (f, s, r, count, len) = match read_set(at) {
        Ok(v) => v,
        Err(e) => {
            static N: AtomicU32 = AtomicU32::new(0);
            if super::log_on(super::LogCat::Animation) && N.fetch_add(1, Ordering::Relaxed) < 20 {
                log!(Animation, "KH2 slots: set at {at:#x} ({why}) not extended: {e}");
            }
            return false;
        }
    };
    let slots: &'static mut [u32] = Box::leak(vec![0u32; count + p.motions.len() + 1].into_boxed_slice());
    for i in 0..count {
        slots[i] = rd::<u32>(s + i * 4);
    }
    for (i, m) in p.motions.iter().enumerate() {
        slots[count + i] = m.1;
    }
    // (Room for our own requests past KH1's, `kh1rows`; 0xFFF, none, between.)
    let ours_end = if b.moves.iter().any(|m| m.own.is_some()) { super::kh1rows::requests_end() } else { 0 };
    let requests: &'static mut [u32] = Box::leak(vec![0xFFFu32; len.max(ours_end)].into_boxed_slice());
    for i in 0..len {
        requests[i] = rd::<u32>(r + i * 4);
    }
    // The requests ours stand in for, where this set's own entry holds the KH1 motion they stand
    // in for (by its signature, `pose::file_slot_of_motion`).
    let mut redirects = Vec::new();
    let mut missing = Vec::new();
    let mut own_requests = Vec::new();
    for (i, m) in b.moves.iter().enumerate() {
        // Our own request: its template's kind bits here and our slot (where the template's entry
        // holds the KH1 motion it stands for).
        if let Some(t) = m.own {
            let e = requests.get(t).copied().unwrap_or(0xFFF);
            let held = (e & 0xFFF) as usize;
            if held < count && pose::file_slot_of_motion(pose::ptr(slots[held])) == Some(m.stands_for) {
                // (Each of its motions a redirect too, for KH2's pick on a request with several:
                // Finishing Plus's, `pick_own`.)
                redirects.push((m.request, e, (count + i) as u32, m.kind, m.slot, m.default, true, m.form));
                // (A form's own entries only by `redirect`, while he is in it; base Sora's here.)
                if let Some(x) = requests.get_mut(m.request).filter(|_| m.default && m.form.is_none()) {
                    *x = (e & !0xFFF) | (count + i) as u32;
                    own_requests.push(format!("{:#x}", m.request));
                }
            } else if m.default {
                missing.push(format!("{:#x} (as {t:#x})", m.request));
            }
            continue;
        }
        let e = requests.get(m.request).copied().unwrap_or(0xFFF);
        let own = (e & 0xFFF) as usize;
        let held = if own < count { pose::file_slot_of_motion(pose::ptr(slots[own])) } else { None };
        let holds = held.is_some() && (held == Some(m.stands_for) || held == also_stands_for(m.request));
        if holds {
            redirects.push((m.request, e, (count + i) as u32, m.kind, m.slot, m.default, false, m.form));
        } else if m.variant == 0 {
            missing.push(format!("{:#x}", m.request));
        }
        // (KH1 requests that play the same slot of ours, `ALIASES`, where their own entry holds it.)
        for &(alias, _) in ALIASES.iter().filter(|a| a.1 == m.request && m.default) {
            let e = requests.get(alias).copied().unwrap_or(0xFFF);
            let own = (e & 0xFFF) as usize;
            if own < count && pose::file_slot_of_motion(pose::ptr(slots[own])) == Some(m.stands_for) {
                redirects.push((alias, e, (count + i) as u32, m.kind, m.slot, true, false, m.form));
            } else {
                missing.push(format!("{alias:#x} (as {:#x})", m.request));
            }
        }
    }
    let mut form_requests: Vec<(super::form::Kind, Vec<usize>)> = Vec::new();
    for f in FORM_SLOTS.iter() {
        let mut v: Vec<usize> = redirects.iter().filter(|r| r.7 == Some(f.kind)).map(|r| r.0).collect();
        v.sort_unstable();
        v.dedup();
        if !v.is_empty() {
            form_requests.push((f.kind, v));
        }
    }
    let t = Set { kh1_slots: sh, kh1_requests: rh, slots_handle: handle(slots.as_ptr() as usize), requests_handle: handle(requests.as_ptr() as usize), requests, redirects, form_requests };
    log!(Animation,
        "KH2 slots: Sora's set {f:#x} ({why}): {count} slots of its own, ours at {}..{}, {} requests; ours stand in for {}; our own requests {}{}",
        count,
        count + p.motions.len() - 1,
        len,
        t.redirects.iter().filter(|r| r.5).map(|r| format!("{:#x} (its slot {})", r.0, r.1 & 0xFFF)).collect::<Vec<_>>().join(", "),
        if own_requests.is_empty() { "none".to_string() } else { own_requests.join(", ") },
        if missing.is_empty() { String::new() } else { format!("; not in this set: {}", missing.join(", ")) }
    );
    wr(at + 4, t.slots_handle);
    wr(at + 8, t.requests_handle);
    p.sets.push(t);
    true
}

/// Ours for a set KH1 just bound onto Sora's actor (`at` = `actor + 0x1D0`), from the bind hooks.
unsafe fn bound(actor: usize, why: &str) {
    if !is_sora(actor) {
        return;
    }
    let Some(b) = built() else { return };
    let Ok(mut g) = PLACED.try_lock() else { return };
    let p = g.get_or_insert_with(|| place(b));
    extend(b, p, actor + SET, why);
}

/// Where an actor's bound set is (file, slot list, request table, a third section), and where
/// KH1 keeps his own while a world's is bound.
const SET: usize = 0x1D0;
const SAVED_SET: usize = 0x1E0;

/// KH1 binding Sora's own set at his actor's creation (`0x140287E40 (actor, bundle, motion
/// file)`; Epic `0x140285CC0`): `mov [rsp+8], rbx; mov [rsp+10h], rbp`.
const BIND_OWN_RVA: Rva = Rva(0x285CC0);
const BIND_OWN_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];
/// KH1 binding a motion set into an actor's four handles (`0x1402886F0 (set, file)`; a world's
/// onto Sora from `0x14029E850`; Epic `0x140286570`, worked out from the neighbours, unchecked):
/// `mov [rsp+8], rbx; push rdi; sub rsp, 20h`.
const BIND_SET_RVA: Rva = Rva(0x286570);
const BIND_SET_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];
static ORIG_BIND_OWN: AtomicUsize = AtomicUsize::new(0);
static ORIG_BIND_SET: AtomicUsize = AtomicUsize::new(0);

unsafe extern "system" fn bind_own(actor: usize, bundle: usize, file: usize) {
    let f: unsafe extern "system" fn(usize, usize, usize) = std::mem::transmute(ORIG_BIND_OWN.load(Ordering::Acquire));
    f(actor, bundle, file);
    if file != 0 {
        bound(actor, "his own, bound");
    }
}

unsafe extern "system" fn bind_set(set: usize, file: usize) {
    let f: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_BIND_SET.load(Ordering::Acquire));
    f(set, file);
    if file != 0 && set > SET {
        bound(set - SET, "a world's, bound");
    }
}

/// The bind hooks (they do nothing until our motions are built, `kh1_slots` on).
pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    super::hook::install(base + BIND_OWN_RVA, &BIND_OWN_PROLOGUE, bind_own as *const () as usize, &ORIG_BIND_OWN, "actor motion set bind (KH2 slots)");
    super::hook::install(base + BIND_SET_RVA, &BIND_SET_PROLOGUE, bind_set as *const () as usize, &ORIG_BIND_SET, "motion set bind (KH2 slots)");
    // (KH2's swap into the fall in KH1's jump, `kh2_apex`: Steam only.)
    if super::addr::Build::current() == super::addr::Build::Steam {
        super::hook::install(base + APEX_STEAM, &APEX_PROLOGUE, apex as *const () as usize, &ORIG_APEX, "jump apex (KH2's fall swap)");
        super::hook::install(base + SOUND_EVENT_STEAM, &SOUND_EVENT_PROLOGUE, sound_event as *const () as usize, &ORIG_SOUND_EVENT, "motion sound event (KH2's frame triggers)");
    }
}

/// Each frame Sora is posed (`pose`, the game thread): ours on the set bound and the one KH1
/// keeps (in case a bind came before our motions were built), the requests pointed at ours by
/// their moves' styles in every set, and which of ours plays (`tick`).
pub unsafe fn frame(actor: usize) {
    let Some(b) = built() else { return };
    if !is_sora(actor) {
        return;
    }
    let Ok(mut g) = PLACED.try_lock() else { return };
    let p = g.get_or_insert_with(|| place(b));
    extend(b, p, actor + SET, "found on his actor");
    if rd::<u32>(actor + SAVED_SET) != 0 {
        extend(b, p, actor + SAVED_SET, "kept by KH1 on his actor");
    }
    redirect(p);
    drop(g);
    // KH2's moves on KH1 rows of their own, once their requests are ours (`kh1rows`).
    if b.moves.iter().any(|m| m.own.is_some()) {
        super::kh1rows::install(actor);
    }
    tick(actor);
    loop_range(actor);
    finisher_window(actor);
    // The same on KH1's own finishers at the combo style KH1 (`combomods::kh1_finisher_window`).
    super::combomods::kh1_finisher_window(actor);
    // KH2 Fire's combo window on KH1's own casts at the Magic Combo row (`formmagic`).
    super::formmagic::kh1_cast_window(actor);
    // KH2's Counterguard window in its guard's repel (`combomods::counter_window`).
    super::combomods::counter_window(actor);
    // KH2's invulnerable ranges on KH1's no-damage flag (`invulnerable_step`, t860; KH1's level reader on our 8.0 since t868).
    invulnerable_step(actor);
    // KH2's hits on his body in the KH2 motion KH1 plays (`body_strike`): KH1 attack objects of
    // ours (`bodyhits`), else KH2's own sweep (`kh2_hits`, which keeps the guards and the pull).
    super::bodyhits::frame(actor);
    super::takeover::kh2_hits(actor);
}

/// KH1's motion flags (`+0x160`): bit 3 holds the motion at its end (else it loops), bit 6 a loop
/// range of its own (`+0x288` start, `+0x28C` end, `+0x290`), which KH1's apply (`0x14029FC40`)
/// clears with each new request and its start (`0x1402A0350`) then sets to the whole motion; KH1's
/// own setter is `0x14029FA10 (actor, start, end, third)` (Checked (decomp), Steam).
const MOTION_FLAGS: usize = 0x160;
const HOLD_AT_END: u32 = 8;
const OWN_LOOP: u32 = 0x40;
/// A request queued and not yet applied, or being applied (`+0x160` bit 0). Checked (decomp).
const APPLYING: u32 = 1;
const LOOP_START: usize = 0x288;
const LOOP_END: usize = 0x28C;
const LOOP_THIRD: usize = 0x290;

/// Our slot playing on `actor` loops from KH2's own loop point (`Move::loop_from`: the fall's 12 of
/// its 32 frames), not from its start: KH1's own loop range for it (`MOTION_FLAGS`), set once after
/// KH1 started it (each frame KH1 posed him checks it; KH1's step wraps the time to the loop start
/// when it passes the end, `0x14029F0A0`, Checked (decomp)). Nothing on a request that holds.
///
/// Not while a request is being applied (`+0x160` bit 0, set by KH1's request `0x14029F7C0` and
/// cleared by its update `0x14029F0A0` only after the queue step `0x14029FC40` returns): this runs
/// from KH1's pose pass, which KH1's apply `0x1402A0350` calls on the new motion before it writes
/// the new slot to `+0x168`, after `0x14029FC40` cleared bit 6. `current` then still named the
/// motion going out (the fall), its loop range went on the motion coming in, and a blended start
/// keeps the range it finds (only the unblended start sets the whole motion): the hover 208 out of
/// KH2's fall looped 13..32 of its 100 frames (glide thread, after m7: "neutral glide resets its
/// animation wrong when setting jump to kh2"). Checked (decomp, trace).
unsafe fn loop_range(actor: usize) {
    if rd::<u32>(actor + MOTION_FLAGS) & APPLYING != 0 {
        return;
    }
    let Some(m) = current(actor) else { return };
    let Some(from) = m.loop_from else { return };
    let flags = rd::<u32>(actor + MOTION_FLAGS);
    if flags & (HOLD_AT_END | OWN_LOOP) != 0 {
        return;
    }
    wr(actor + LOOP_START, from);
    wr(actor + LOOP_END, m.frames as f32);
    wr(actor + LOOP_THIRD, 0.0f32);
    wr(actor + MOTION_FLAGS, flags | OWN_LOOP);
    static N: AtomicU32 = AtomicU32::new(0);
    if super::log_on(super::LogCat::Animation) && N.fetch_add(1, Ordering::Relaxed) < 10 {
        log!(Animation, "KH2 slots: KH2 motion {} loops from frame {from:.0} of {} (KH1's loop range)", m.motion, m.frames);
    }
}

/// KH1's apex step (Steam `0x1402AF630 (actor)`, from its jump's rise each frame: `0x1402AF530`,
/// `0x1402AF6A0`, `0x1402AFAE0`): with its arc's time `+0x3B0` + 6 at or past the rise time
/// `+0x3B8`, nothing queued (`+0x178`) and its takeoff request playing (`+0x164` = base `+0x3D0` +
/// 4), it asks its apex (base + 5, blend 6.0, the constant at `0x14042F93C`) and queues its fall
/// (base + 6, blend 0). Checked (decomp, exe).
const APEX_STEAM: usize = 0x2AF630;
const APEX_PROLOGUE: [u8; 6] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20];
static ORIG_APEX: AtomicUsize = AtomicUsize::new(0);
/// KH1's motion request (`0x14029F980 (actor, request, blend)`, Steam; a fresh queue).
const SET_MOTION_STEAM: usize = 0x29F980;
/// KH2's swap into the fall (Steam KH2 `0x1403CB680`, the jump's update; Checked (decomp)): with the
/// arc's time `+0xF0` + 5 at or past its rise time `+0xF4`, while the jump's motion is still up
/// (`+0x124` bit 0, cleared by the swap), it asks the fall motion (`+0x118`, which the jump's start
/// `0x140422630` sets to 4 for the jump and the High Jump alike) through `0x1403CB610` with blend
/// 8.0; on an early release (after 6 frames, more than 5 before the apex) the same swap at once,
/// before the release's hop.
const KH2_FALL_MARGIN: f32 = 5.0;
const KH2_FALL_BLEND: f32 = 8.0;

/// KH2's jump rule in KH1's jump (Jump style KH2, base Sora, our fall slot built): KH1's apex and
/// its 6-frame apex motion give way to KH2's swap, KH1's fall request asked 5 frames before the
/// apex with KH2's blend. True when it is ours to decide (KH1's apex step then doesn't run).
unsafe fn kh2_apex(actor: usize) -> bool {
    if !super::shared::on() || !(super::styles::kh2(Kind::Jump) || super::form::native_now()) || !is_sora(actor) || !has(FALL.request, FALL.motion) {
        return false;
    }
    let base = rd::<i32>(actor + 0x3D0);
    if base != 0 && base != 5 {
        return false;
    }
    let (t, up) = (rd::<f32>(actor + 0x3B0), rd::<f32>(actor + 0x3B8));
    if t.is_finite() && up.is_finite() && up <= t + KH2_FALL_MARGIN {
        fall_now(actor, &format!("{t:.1} of {up:.1} frames up"));
    }
    true
}

/// KH1's motion sound event handler (Steam `0x140295420 (actor, a, b, c)`), which KH1's motion
/// pass calls for each type-6 key whose time it passed: `0x14029FB30 (actor, from, to)` (from the
/// animation step `0x14029F0A0`: the frame's step, the rest after a loop, and a blend's end; and
/// from `0x14029FF80`) collects the keys of types 4 and 6, ids 0..3, with `from <= time < to`
/// through `0x140394900`, then runs foot effects (`0x1402C2660`) and this. It plays a sound by id
/// (`a` 0), his voice (`a` 0x19) or a material sound (`0x140295B40`, `a` 1..0x1D), after its
/// own checks (the flags `0x142867364` bit 0x200, the actor's mute bits `+0x9C` by `c`,
/// `0x14017AF20`, the actor's side). Checked (decomp). `mov [rsp+8], rbx; mov [rsp+0x10], rsi;
/// push rdi; sub rsp, 0x20`, Checked (exe).
const SOUND_EVENT_STEAM: usize = 0x295420;
const SOUND_EVENT_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20];
static ORIG_SOUND_EVENT: AtomicUsize = AtomicUsize::new(0);

/// KH1's sound event handler with our codes (`kh2_trigger_keys`): a KH2 frame trigger of our
/// slot, played as KH2's (`kh2sound::event`, `kh2fx::event`, as their own gates say) on Sora;
/// never handed to KH1's (whose table would read past its end). KH1's own keys go on as before.
/// (KH1's motion pass itself skips every type-6 key while `0x142867364` bit 0x400 is set, which
/// `0x1401A22B0` sets with a stop of every sound and `0x1401A2130` / `0x1401B02C0` clear,
/// Checked (decomp); Assumption: an event or cutscene. Our triggers are skipped then too.)
unsafe extern "system" fn sound_event(actor: usize, a: i32, b: i32, c: u32) {
    if a == ks::KH2_RUMBLE as i32 {
        if actor == super::takeover::sora_actor() {
            super::rumble::play(b as u16, c as u16 as i16);
        }
        return;
    }
    if a == ks::KH2_EFFECTS as i32 {
        if actor == super::takeover::sora_actor() {
            effects_at(b as u16, c);
        }
        return;
    }
    if let Some(e) = kh2_trigger(a, c) {
        if actor == super::takeover::sora_actor() {
            let motion = b as u16;
            // (Wisdom's frame triggers 1, its `wisd` shots, come as `KH2_EFFECTS` keys: `effects_at` →
            // `form::effect` → `formmagic::effect`.)
            super::kh2sound::event(motion, e);
            super::kh2fx::event(motion, e);
            // (t742: Battle Quotes KH1, a KH2 voice of a move KH1 has no voice of its own for: KH1's
            // stand-in line, `kh1voice::stand_in`.)
            if let EventKind::Voice(v) = e {
                if !super::kh2sound::kh2_voice() {
                    super::kh1voice::stand_in(actor, motion, v);
                }
            }
            static N: AtomicU32 = AtomicU32::new(0);
            if super::log_on(super::LogCat::Animation) && N.fetch_add(1, Ordering::Relaxed) < 40 {
                log!(Animation, "KH2 slots: KH1's motion pass fired KH2 motion {motion}'s {e:?} at KH1's time {:.2}", rd::<f32>(actor + TIME_NOW));
            }
        }
        return;
    }
    let f: unsafe extern "system" fn(usize, i32, i32, u32) = std::mem::transmute(ORIG_SOUND_EVENT.load(Ordering::Acquire));
    f(actor, a, b, c);
}

unsafe extern "system" fn apex(actor: usize) {
    if kh2_apex(actor) {
        return;
    }
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_APEX.load(Ordering::Acquire));
    f(actor);
}

/// KH2's swap into the fall now (`kh2_apex`, or KH2's early release in our arc,
/// `shared::physics`), while KH1's takeoff request plays with nothing queued: KH1's fall request
/// (base + 6) with KH2's blend. Only at Jump KH2 with our fall slot.
pub unsafe fn fall_now(actor: usize, why: &str) {
    if !super::shared::on() || !(super::styles::kh2(Kind::Jump) || super::form::native_now()) || !has(FALL.request, FALL.motion) || super::addr::Build::current() != super::addr::Build::Steam {
        return;
    }
    let base = rd::<i32>(actor + 0x3D0);
    if (base != 0 && base != 5) || rd::<i32>(actor + 0x164) != base + 4 || rd::<i32>(actor + 0x178) != 0 {
        return;
    }
    let set: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(super::takeover::game_base() + SET_MOTION_STEAM);
    set(actor, (base + 6) as u32, KH2_FALL_BLEND);
    static N: AtomicU32 = AtomicU32::new(0);
    if super::log_on(super::LogCat::Animation) && N.fetch_add(1, Ordering::Relaxed) < 20 {
        log!(Animation, "KH2's jump: into the fall ({why}): KH1's request {:#x}, blend {KH2_FALL_BLEND}", base + 6);
    }
}

/// The swing KH2's row selection picked for each of KH1's combo requests 0xC8..0xCE (ground
/// 0xC8..0xCB, air 0xCC..0xCE; our move's `slot`; `usize::MAX`: the request's own).
static PICKED: [AtomicUsize; 7] = [const { AtomicUsize::new(usize::MAX) }; 7];

/// The game frame (`takeover::GAME_FRAME`, + 1) a pick was forced on each of those requests at
/// KH1's combo style (`pick_ability`: an ability's move of KH2's, which plays ours at either
/// style), 0 none.
static FORCED: [AtomicU64; 7] = [const { AtomicU64::new(0) }; 7];
/// A forced pick outlives the move it was for by no more than this (game frames with none of
/// ours playing): KH1 starts the request the frame after its row ran.
const FORCED_FOR: u64 = 8;

fn forced(request: usize) -> bool {
    request.checked_sub(0xC8).and_then(|i| FORCED.get(i)).is_some_and(|f| f.load(Ordering::Relaxed) != 0)
}

fn picked(request: usize) -> Option<usize> {
    let m = if let Some(i) = CASTS.iter().position(|c| c.2 == request) {
        CAST_PICKED[i].load(Ordering::Relaxed)
    } else {
        match request.checked_sub(super::kh1rows::FIRST_REQUEST as usize) {
            Some(n) => OWN_PICKED.get(n)?.load(Ordering::Relaxed),
            None => PICKED.get(request.checked_sub(0xC8)?)?.load(Ordering::Relaxed),
        }
    };
    (m != usize::MAX).then_some(m)
}

/// Whether KH1's request `request` plays a KH2 motion of ours now (KH2's row selection picked one
/// for it, or it is one of our own rows' requests with its move picked).
pub fn plays_kh2(request: usize) -> bool {
    picked(request).is_some()
}

/// The cast KH2 picked for each of KH1's spell requests (`CASTS`; our move's `slot`,
/// `usize::MAX`: the request's own).
static CAST_PICKED: [AtomicUsize; CASTS.len()] = [const { AtomicUsize::new(usize::MAX) }; CASTS.len()];

/// KH1's spell request `request` plays KH2's finisher cast from now (`finisher`, its slot built:
/// `CAST_FINISHERS`) or its own cast (the entry pointed there at once, before KH1's command step
/// asks for it; `formmagic::pick_cast`). Returns the KH2 motion it plays.
pub unsafe fn pick_cast(request: usize, finisher: bool) -> Option<u16> {
    let i = CASTS.iter().position(|c| c.2 == request)?;
    // (The moveset playing's: a native Drive Form's own finisher cast, `live`.)
    let fin = CAST_FINISHERS.iter().find(|f| f.2 == request).and_then(|f| {
        let b = built()?;
        b.moves.iter().find(|m| m.request == request && m.motion == f.0 && !m.default && live(b, m))
    }).filter(|_| finisher);
    CAST_PICKED[i].store(fin.map_or(usize::MAX, |m| m.slot), Ordering::Relaxed);
    if let Ok(mut g) = PLACED.try_lock() {
        if let Some(p) = g.as_mut() {
            redirect(p);
        }
    }
    fin.map(|m| m.motion).or_else(|| cast_motion(request))
}

/// KH2's finisher cast is picked for KH1's spell request `request` now (`pick_cast` keeps a slot only
/// for a finisher; t746, the spell voice, `kh2sound::spell_line`).
pub fn finisher_picked(request: usize) -> bool {
    CASTS.iter().position(|c| c.2 == request).is_some_and(|i| CAST_PICKED[i].load(Ordering::Relaxed) != usize::MAX)
}

/// KH2's motion our slot plays on KH1's spell request `request` now (its pick, else its own).
pub fn cast_now(request: usize) -> Option<u16> {
    let i = CASTS.iter().position(|c| c.2 == request)?;
    let s = CAST_PICKED[i].load(Ordering::Relaxed);
    if s != usize::MAX {
        return moved(s).map(|m| m.motion);
    }
    cast_motion(request)
}

/// The motion KH2 picked for each request of ours (`pick_own`; our move's `slot`, `usize::MAX`:
/// the request's own).
static OWN_PICKED: [AtomicUsize; super::kh1rows::OURS.len()] = [const { AtomicUsize::new(usize::MAX) }; super::kh1rows::OURS.len()];

/// A request of ours with several motions (`kh1rows::Ours::next_finisher`) plays our slot of
/// KH2's `motion` from now (its entry pointed there at once, before KH1's row asks for it).
/// Whether it does.
pub unsafe fn pick_own(request: usize, motion: u16) -> bool {
    let Some(i) = request.checked_sub(super::kh1rows::FIRST_REQUEST as usize).filter(|i| *i < OWN_PICKED.len()) else { return false };
    let m = built().and_then(|b| b.moves.iter().find(|m| m.request == request && m.motion == motion && live(b, m))).map(|m| m.slot);
    OWN_PICKED[i].store(m.unwrap_or(usize::MAX), Ordering::Relaxed);
    if let Ok(mut g) = PLACED.try_lock() {
        if let Some(p) = g.as_mut() {
            redirect(p);
        }
    }
    m.is_some()
}

/// KH2's Finishing Plus window (range id 50, `WindowKind::Finisher`) on our finisher slot playing
/// on `actor`, Finishing Plus equipped: KH1's combo window for this frame (`+0x1CC` bit 22, the
/// level the motion step copies from the instance's `+0x14`, which KH1's skeleton pass has just
/// worked out: `pose`'s hooks call `frame` after it), so KH1's command menu takes an Attack there
/// (`0x1402A2EC0`: free, or in the combo window) and KH1's Attack step counts it on as KH1's
/// combo does (one more, the swing connected or Combo Master), where `combomods` takes KH2's next
/// finisher. Only while another finisher is left (one plus Finishing Plus's copies,
/// `combomods::finisher_chain_open`, at both combo styles since t641). KH2 takes the
/// Square moves there too (either bit, `0x1403A9950`). Sora's (Checked
/// (files)): 154 / 155 35..64, 183 21..50, 184 23..52, Explosion 60..72, Guard Break 48..60,
/// Aerial Finish 89..108, Magnet Burst 70..86.
unsafe fn finisher_window(actor: usize) {
    use character_core::WindowKind as W;
    let Some(m) = current(actor) else { return };
    let Some(clip) = clip_of(m) else { return };
    let t: f32 = rd(actor + TIME_NOW);
    if !(t >= 0.0) || !clip.has(t as u32, |k| matches!(k, W::Finisher)) || super::takeover::with_abilities(|a| a.finishing_plus > 0) != Some(true) || !super::shared::active() || !super::combomods::finisher_chain_open() {
        return;
    }
    // (A finisher cast's: only while Finishing Plus leaves another finisher, KH2's own test,
    // `cast_finisher_open`.)
    if matches!(m.kind, Kind::Magic | Kind::AirMagic) && !super::combomods::finisher_left() {
        return;
    }
    let inst = pose::ptr(rd::<u32>(actor + 0x15C));
    if inst < 0x10000 {
        return;
    }
    wr(inst + 0x14, rd::<u32>(inst + 0x14) | super::combomods::KH1_COMBO_WINDOW);
    static N: AtomicU32 = AtomicU32::new(0);
    if super::log_on(super::LogCat::Animation) && N.fetch_add(1, Ordering::Relaxed) < 20 {
        log!(Animation, "KH2's Finishing Plus window on KH2 motion {} (frame {t:.0}): KH1's combo window", m.motion);
    }
}

/// The requests by style (base Sora under shared states; a change applies from KH1's next
/// request, never to the motion playing), each to its own move or the one KH2's row selection
/// picked for it.
unsafe fn redirect(p: &mut Placed) {
    let on = super::shared::on();
    // (A Drive Form on KH1's own states, `form::native_form`: a request with a move of the form's
    // plays the form's, whatever the styles (a Drive Form is KH2's all through); every other request
    // base Sora's as before. Out of the form, its moves are left alone.)
    let now = super::form::native_form();
    for t in p.sets.iter_mut() {
        for i in 0..t.redirects.len() {
            let (req, kh1, ours, kind, id, default, own, form) = t.redirects[i];
            let here = now.filter(|k| t.form_requests.iter().any(|(fk, v)| fk == k && v.binary_search(&req).is_ok()));
            if form != here {
                continue;
            }
            let want = picked(req).filter(|m| t.redirects.iter().any(|r| r.0 == req && r.4 == *m && r.7 == here));
            if want.map_or(!default, |m| m != id) {
                continue;
            }
            if let Some(e) = t.requests.get_mut(req) {
                // (A request of ours is ours whatever the styles.)
                // (KH1's spell requests by their spell's rows, `formmagic::anim_kh2`: KH2's cast
                // motion for a spell cast KH2's way, else the spell's animation row; the tester,
                // m19. KH1's own slot otherwise: KH1's whole cast, as the Cure row at KH1, m4.)
                let style = match cast_spell(req) {
                    Some(spell) if matches!(kind, Kind::Magic | Kind::AirMagic) => super::formmagic::anim_kh2(spell),
                    // (KH1's open request: the Chest Animation row, t783.)
                    _ if req == CHEST_REQUEST => super::chest::kh2_animation(),
                    _ => super::styles::kh2(kind),
                };
                *e = if own || form.is_some() || on && (style || (forced(req) && want == Some(id))) { (kh1 & !0xFFF) | ours } else { kh1 };
                // (A form's cast baked with its root travel (`make`'s `form_cast_travel`): KH1's
                // travel flag on the request while that slot is on it, so KH1 moves him by its H10.)
                if form.is_some() && *e & 0xFFF == ours && moved(id).is_some_and(|m| m.travel) {
                    *e |= TRAVEL_FLAG;
                }
            }
        }
    }
}

/// KH2's row selection on KH1's own ground or air swing (`combomods::attack`, Base / Aerial
/// Combo KH2): KH1's `request` (0xC8..0xCE, the row about to run) plays our slot of KH2's `motion` from now (its
/// request table entry pointed there at once, before KH1's row makes the request). `None`, or a
/// motion with no slot built for that request: the request's own. Whether `motion` plays.
pub unsafe fn pick(request: usize, motion: Option<u16>) -> bool {
    pick_ability(request, motion, false)
}

/// `pick`, and with `force` (the request's combo style KH1: an ability's move of KH2's, which
/// plays ours at either style, `combomods::native_ability`) the request pointed at ours for that
/// move even so, every other forced pick dropped (KH1's own motions there again). Not forced:
/// the style decides, as before.
pub unsafe fn pick_ability(request: usize, motion: Option<u16>, force: bool) -> bool {
    let Some(i) = request.checked_sub(0xC8).filter(|i| *i < PICKED.len()) else { return false };
    let m = motion.and_then(|mo| built().and_then(|b| b.moves.iter().find(|m| m.request == request && m.motion == mo && live(b, m)))).map(|m| m.slot);
    PICKED[i].store(m.unwrap_or(usize::MAX), Ordering::Relaxed);
    for f in &FORCED {
        f.store(0, Ordering::Relaxed);
    }
    if force && m.is_some() {
        FORCED[i].store(super::takeover::GAME_FRAME.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
    }
    if let Ok(mut g) = PLACED.try_lock() {
        if let Some(p) = g.as_mut() {
            redirect(p);
        }
    }
    m.is_some()
}

/// A slot of ours plays KH2's `motion` for KH1's `request` (`pick` can point it there).
pub fn has(request: usize, motion: u16) -> bool {
    built().is_some_and(|b| b.moves.iter().any(|m| m.request == request && m.motion == motion && live(b, m)))
}

/// The KH2 motion of our slot `actor` plays now (KH2's anti-repeat passes over it, `pick`).
pub unsafe fn playing_motion(actor: usize) -> Option<u16> {
    current(actor).map(|m| m.motion)
}

/// Which of ours plays (`PLAYING`, for `kh2fx::tick`, the trace and the panel), forced picks whose
/// move is over put back, and (off Steam only) our slot's KH2 sounds, voices and effects between
/// KH1's last time and its time now. On Steam KH1's own motion pass fires those: they are sound
/// events of the baked motion (`kh2_trigger_keys`, `sound_event`).
unsafe fn tick(actor: usize) {
    static LAST: Mutex<Option<(usize, f32)>> = Mutex::new(None);
    let Some(m) = current(actor) else {
        // (A forced pick whose move is over: KH1's own motion on that request again.)
        let now = super::takeover::GAME_FRAME.load(Ordering::Relaxed) + 1;
        let stale = FORCED.iter().any(|f| {
            let at = f.load(Ordering::Relaxed);
            at != 0 && now.saturating_sub(at) > FORCED_FOR
        });
        if stale {
            for f in &FORCED {
                f.store(0, Ordering::Relaxed);
            }
            if let Ok(mut g) = PLACED.try_lock() {
                if let Some(p) = g.as_mut() {
                    redirect(p);
                }
            }
        }
        PLAYING.store(usize::MAX, Ordering::Relaxed);
        PLAYING_MOTION.store(u32::MAX, Ordering::Relaxed);
        if let Ok(mut l) = LAST.try_lock() {
            *l = None;
        }
        return;
    };
    let slot = m.slot;
    PLAYING.store(slot, Ordering::Relaxed);
    PLAYING_MOTION.store(m.motion as u32, Ordering::Relaxed);
    let now: f32 = rd(actor + TIME_NOW);
    let Ok(mut l) = LAST.try_lock() else { return };
    let from = match *l {
        Some((s, t)) if s == slot && t <= now => t,
        _ => -1.0,
    };
    if from == -1.0 {
        static N: AtomicU32 = AtomicU32::new(0);
        if super::log_on(super::LogCat::Animation) && N.fetch_add(1, Ordering::Relaxed) < 40 {
            log!(Animation,
                "KH2 slots: KH1 plays ours, KH2 motion {} (for KH1's {} / request {:#x}; its slot {} in the set; Sora's request {:#x})",
                m.motion,
                m.stands_for,
                m.request,
                rd::<u32>(actor + SLOT_NOW),
                rd::<u32>(actor + 0x164)
            );
        }
    }
    *l = Some((slot, now));
    // (Empty on Steam.)
    for (f, e) in &m.events {
        let f = *f as f32;
        if f > from && f <= now {
            // (Valor's own effect elements are its archive's, `form::effect` (`kh2fx::form_event`),
            // as KH2's controller played them in the form; its sounds by `kh2sound`'s Valor banks.)
            match e {
                EventKind::Effect(n) if m.form.is_some() => super::form::effect(m.motion, *n),
                _ => {
                    super::kh2sound::event(m.motion, *e);
                    super::kh2fx::event(m.motion, *e);
                }
            }
        }
    }
}
