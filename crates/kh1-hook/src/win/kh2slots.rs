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

/// KH1's ground combo, KH2's swings for it as `pose` draws them (`pose::swing_motion`): KH1's
/// first and third swing (98, request 0xC8) KH2's first (151); its second (99, 0xC9) KH2's second
/// (152; the user: "sora base attack 2, kh2 base attack 2"); slot 100 (0xCA, taken as another
/// swing) KH2's first; its finisher (101, 0xCB) KH2's finisher (154; "kh1 base finisher, kh2 base
/// finishers"). These are each request's own; with the Base Combo at KH2, KH2's row selection
/// picks among the other ground swings KH2's rows can take, built for each request too (`pick`,
/// `swing_rows`).
///
/// KH1's air combo (slots 102..104, requests 0xCC..0xCE), KH2's aerial swings for it as `pose`
/// draws them: 181 for KH1's first air swing (102), 182 for its second (103), 183 for its finisher
/// (104), KH2's own order in the air (181 / 182 alternate, 183 finishes). With these built, the Attack in the air at Aerial Base Combo KH2 stays in KH1's air
/// state (`covers`, `shared::kh1_combo`) instead of borrowing him for KH2's own air combo.
/// These are each request's own; with the Aerial Combo at KH2, KH2's row selection picks among
/// KH2's other air swings for them too (`pick`, `swing_rows`).
/// KH1's ground finisher request (slot 101); 0xC8..0xCA are its swings.
pub const GROUND_FINISHER: usize = 0xCB;
/// KH1's air finisher request (slot 104); 0xCC / 0xCD are its swings.
pub const AIR_FINISHER: usize = 0xCE;

/// KH1's spell casts (`docs/kh1-magic-native.md`; KH1's command step, Steam `0x1402A3DB0` /
/// `0x1402A3600` / `0x1402A4150` / `0x1402A7510`, Checked (decomp); KH1's request table,
/// `xa_ex_0010.mset`, Checked (files)): on the ground request `0x36 + spell` (Fire 0x36 slot 54,
/// Blizzard 0x37 slot 55, Thunder 0x38 slot 56, Cure 0x39 slot 56), in the air `0x83 + spell`
/// (79, 80, 81, 81); Fire's second part 0x3D (slot 57; on the ground after 0x36, or alone for a
/// Fire out of a swing or out of 0x3D itself) and 0x8A (slot 82, the air's). With the Magic /
/// Aerial Magic styles at KH2, each plays KH2's cast motion for the spell (MAGC, `00battle.bin`,
/// Checked (files): Fire 56 / air 58, Blizzard 59 / 61, Thunder 62 / 64, Cure 65 / 67; the same
/// motions at every level) in KH1's own cast state (`formmagic::kh1_spell_start`).
pub const CASTS: [(u16, usize, usize, bool); 10] = [
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
];

/// KH2's finisher casts (MAGC's finisher motions, Checked (files): Fire 57, Blizzard 60, Thunder 63;
/// Cure has none, and none in the air), each on KH1's ground request of its spell (and Fire's
/// second part, 0x3D, which KH1 plays for a Fire out of a swing): (motion, KH1's slot, request).
const CAST_FINISHERS: [(u16, usize, usize); 4] = [(57, 54, 0x36), (60, 55, 0x37), (63, 56, 0x38), (57, 57, 0x3D)];

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
/// fall-check event (`roll_free`).
const ROLL: Spec = Spec { motion: character_core::motion::DODGE_ROLL, stands_for: 117, request: 0xDC, kind: Kind::DodgeRoll };

/// KH2's glide motions on KH1's glide requests (the tester, kh2-glide-in-kh1s-glide-state: KH2's
/// glide in KH1's glide state, `glide`). KH1's request table (`xa_ex_0010.mset`, Checked (files),
/// `docs/kh1-anim-slots.md` §1.5): 0x71 / 0x72 slots 75 / 76 (gliding slow / fast), 0x73 / 0x74
/// slots 77 / 78 (banking). At Glide KH2 KH1's glide state asks 0x71 for KH2's glide 205 and
/// 0x73 / 0x74 for its banking 206 / 207 (`P_EX100.mset`, Checked (files)); 0x72 also plays 205
/// (KH2 has one glide motion at any speed). The hover 208 is a request of ours (`kh1rows`).
const GLIDES: [(u16, usize, usize); 4] = [(205, 75, 0x71), (205, 76, 0x72), (206, 77, 0x73), (207, 78, 0x74)];

const fn glide_spec(i: usize) -> Spec {
    let (motion, stands_for, request) = GLIDES[i];
    Spec { motion, stands_for, request, kind: Kind::Glide }
}

const SPECS: [Spec; 28] = [
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
];

/// KH2's walk and run cycles as slots (Mr Bugs: "can you also bake the animations for kh2s walking
/// and running cycles? its more easy for me to compare"). KH1's walk and run requests (1, 2) are
/// kind 3 in its request table: KH1 plays the entry's slot + 0, 1 or 2 (field, battle, the third
/// variant; `0x14029E050`, `docs/kh1-anim-slots.md` §1.4), so each is built as three slots in a
/// row, on KH1's three as templates (walk 2 / 3 / 4, run 5 / 6 / 7), with KH2's field cycle on the
/// field one and its battle cycle on the other two, as `pose::play_for` draws them; the request is
/// pointed at the first. All three or none.
struct Cycle {
    motion: u16,
    request: usize,
    /// KH1's first slot of the three.
    slot: usize,
}

const CYCLES: [Cycle; 2] = [
    Cycle { motion: character_core::motion::WALK, request: 0x1, slot: 2 },
    Cycle { motion: character_core::motion::RUN, request: 0x2, slot: 5 },
];

/// A request table entry's kind (bits 12..14): 3 picks the slot + 0, 1 or 2 by field / battle.
const KIND_VARIANTS: u32 = 3;

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
    /// Which of a kind 3 request's three slots this is (`CYCLES`; 0 otherwise): only the first is
    /// pointed at, KH1 adds the rest.
    pub variant: u8,
    /// A request of ours (`kh1rows`: 0x200 + n, none of KH1's), not one KH1's own: its entry is
    /// ours in every set, whatever the styles, with the kind bits of KH1's request here (its
    /// template's, whose slot `stands_for` is).
    pub own: Option<usize>,
    /// The KH2 clip's sounds, voices and effects, by frame.
    events: Vec<(u32, EventKind)>,
}

struct Built {
    moves: Vec<Move>,
    /// Our motions (file layout), one after another, 16-aligned.
    blocks: Vec<Vec<u8>>,
    /// KH1's request table as in the file (to know KH1's own when the actor shows it).
    requests: Vec<u32>,
}

static BUILT: OnceLock<Option<Built>> = OnceLock::new();

/// Builds our motions (the loader thread, `pose::load_kh2`): `mset` is KH1 Sora's file; with
/// `bake` (`kh1_bake`), each KH2 motion baked into its KH1 motion, else the template retimed.
pub fn prepare(mset: &[u8], kh2: &pose::Kh2, bake: Option<&pose::Baker>) {
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
        let wanted = |motion: u16| which.as_ref().map_or(true, |w| w.contains(&motion));
        for s in SPECS.iter().filter(|s| wanted(s.motion)) {
            if let Some((m, block)) = make(mset, kh2, bake, &requests, (s.motion, s.stands_for, s.request, s.kind), true, (false, 0), KH1_SLOTS + moves.len(), false)? {
                moves.push(m);
                blocks.push(block);
            }
        }
        // KH2's walk and run, three slots in a row each (`CYCLES`).
        for c in CYCLES.iter().filter(|c| wanted(c.motion)) {
            let mut three = Vec::new();
            for v in 0..3u8 {
                match make(mset, kh2, bake, &requests, (c.motion, c.slot + v as usize, c.request, Kind::MiscAnims), true, (v == 0, v), KH1_SLOTS + moves.len() + v as usize, false)? {
                    Some(m) => three.push(m),
                    None => break,
                }
            }
            if three.len() == 3 {
                for (m, block) in three {
                    moves.push(m);
                    blocks.push(block);
                }
            } else {
                log(&format!("KH2 slots: KH2 motion {} left out: not all three of KH1's slots {}..{} built", c.motion, c.slot, c.slot + 2));
            }
        }
        // KH2's row selection on KH1's ground and air swings (`pick`): every other swing KH2's
        // combo rows can take there, built for each KH1 request of its kind (on the ground a
        // swing for 0xC8..0xCA, a finisher for 0xCB; in the air a swing for 0xCC / 0xCD, a
        // finisher for 0xCE) on that request's own template.
        for s in SPECS.iter().filter(|s| matches!(s.kind, Kind::GroundCombo | Kind::AirCombo)) {
            if !moves.iter().any(|m| m.request == s.request && m.default) {
                continue;
            }
            let air = s.kind == Kind::AirCombo;
            for (motion, finisher) in swing_rows(&kh2.character.moveset.combo_rows, &kh2.character.moveset.clips, &kh2.battle, air) {
                if finisher != matches!(s.request, GROUND_FINISHER | AIR_FINISHER) || motion == s.motion || !wanted(motion) {
                    continue;
                }
                if let Some((m, block)) = make(mset, kh2, bake, &requests, (motion, s.stands_for, s.request, s.kind), false, (false, 0), KH1_SLOTS + moves.len(), false)? {
                    moves.push(m);
                    blocks.push(block);
                }
            }
        }
        // KH2's finisher casts (`CAST_FINISHERS`) on KH1's ground cast requests, picked at a cast
        // that is the combo's finisher (`pick_cast`).
        for &(motion, stands_for, request) in CAST_FINISHERS.iter() {
            if !wanted(motion) || !moves.iter().any(|m| m.request == request && m.default) {
                continue;
            }
            if let Some((m, block)) = make(mset, kh2, bake, &requests, (motion, stands_for, request, Kind::Magic), false, (false, 0), KH1_SLOTS + moves.len(), false)? {
                moves.push(m);
                blocks.push(block);
            }
        }
        // KH2's moves on KH1 rows of their own (`kh1rows`): each on its own request (0x200 + n),
        // built on the KH1 motion and request of its row's kind (the template's travel flag too).
        for (n, o) in super::kh1rows::OURS.iter().enumerate() {
            if !wanted(o.motion) {
                continue;
            }
            let k = o.kind;
            let style = if k.air() { Kind::AirCombo } else { Kind::GroundCombo };
            // (Finishing Plus's next finisher: every finisher KH1's finisher request of its kind
            // plays, its first the request's own.)
            let mut motions = vec![o.motion];
            if o.next_finisher {
                for m in moves.iter().filter(|m| m.request == k.template_request() as usize && m.own.is_none()) {
                    if !motions.contains(&m.motion) {
                        motions.push(m.motion);
                    }
                }
            }
            for (j, &motion) in motions.iter().enumerate() {
                if let Some((mut m, block)) = make(mset, kh2, bake, &requests, (motion, k.template_slot(), k.template_request() as usize, style), j == 0, (false, 0), KH1_SLOTS + moves.len(), true)? {
                    m.own = Some(m.request);
                    m.request = super::kh1rows::request(n) as usize;
                    log(&format!("KH2 slots: KH2 motion {motion} ({}) on our request {:#x} (KH1's row {} of ours)", o.name, m.request, super::kh1rows::row(n)));
                    moves.push(m);
                    blocks.push(block);
                }
            }
        }
        log(&format!(
            "KH2 slots: KH2's row selection on KH1's ground and air combo: {}",
            [0xC8usize, 0xC9, 0xCA, 0xCB, 0xCC, 0xCD, 0xCE].iter().map(|r| format!("{r:#x} -> {:?}", moves.iter().filter(|m| m.request == *r).map(|m| m.motion).collect::<Vec<_>>())).collect::<Vec<_>>().join(", ")
        ));
        Ok(Built { moves, blocks, requests })
    })();
    match r {
        Ok(b) => {
            let _ = BUILT.set(Some(b));
        }
        Err(e) => {
            log(&format!("KH2 slots: not built ({e}); KH2's moves are drawn as before"));
            let _ = BUILT.set(None);
        }
    }
}

/// One of our motions: KH2's `motion` on KH1's slot `stands_for` (its template) for KH1's
/// `request`, as our slot `slot`; `default`: the request's own (`SPECS`), else one KH2's row
/// selection can pick (`pick`); `field`: KH2's field variant of the motion; `variant`: which of a
/// kind 3 request's three slots (`CYCLES`). `None` if it can't be built (logged).
#[allow(clippy::too_many_arguments)]
fn make(mset: &[u8], kh2: &pose::Kh2, bake: Option<&pose::Baker>, requests: &[u32], (motion, stands_for, request, kind): (u16, usize, usize, Kind), default: bool, (field, variant): (bool, u8), slot: usize, own: bool) -> Result<Option<(Move, Vec<u8>)>, String> {
    let Some(clip) = kh2.character.moveset.clips.get(&motion) else {
        log(&format!("KH2 slots: KH2 motion {motion} has no clip, left out"));
        return Ok(None);
    };
    let entry = requests.get(request).copied();
    let base = stands_for.checked_sub(variant as usize);
    if entry.map(|e| (e & 0xFFF) as usize) != base || (variant > 0 && entry.map(|e| (e >> 12) & 7) != Some(KIND_VARIANTS)) {
        // (Left out, not a failure of the rest: one wrong request shouldn't drop the others.)
        log(&format!("KH2 slots: request {request:#x} is not KH1's slot {stands_for}; KH2 motion {motion} left out"));
        return Ok(None);
    }
    let template = ks::motion_bytes(mset, stands_for).map_err(|e| e.to_string())?;
    let old = u32::from_le_bytes(template[4..8].try_into().unwrap());
    // (A field variant's own length: KH2's field walk and run are their own motions.)
    let field_len = field.then(|| kh2.rig.field_motions.get(&motion)).flatten().map(|m| kh2_anim::length_frames(m).round() as u32);
    // (A move of ours with a slot longer than its clip, `kh1rows::Ours::frames`: the Aerial
    // Dodge, its clip looped over it as KH2 loops it.)
    let ours = if own { super::kh1rows::by_motion(motion).map(|n| &super::kh1rows::OURS[n]) } else { None };
    let frames = ours.and_then(|o| o.frames).filter(|&f| f > clip.length).or(field_len).unwrap_or(clip.length).max(1);
    let cast = matches!(kind, Kind::Magic | Kind::AirMagic);
    // (KH2's finisher casts end the combo: `cast_follow`.)
    let finisher_cast = cast && !default && CAST_FINISHERS.iter().any(|f| f.0 == motion && f.2 == request);
    // (A hit reaction, `REACTIONS`: KH2's free state as KH1's "may be left", as on our own requests.)
    let reaction = kind == Kind::HitReactions;
    let mut tracks = kh1_tracks(&ks::events(&template).map_err(|e| e.to_string())?, clip, frames as f32 / old.max(1) as f32, own || reaction, cast);
    // (Only KH2's own sounds for it: the template's left out, `kh1rows::Ours::own_sounds`.)
    if ours.is_some_and(|o| o.own_sounds) {
        tracks.retain(|t| t.kind != ks::SOUND);
    }
    // (KH2's dodge roll on KH1's roll: KH2's free frame as KH1's "may be left", `roll_free`.)
    if kind == Kind::DodgeRoll {
        tracks.retain(|t| t.kind != ks::CAN_LEAVE);
        tracks.push(EventTrack::level(ks::CAN_LEAVE, 0, &[(roll_free(clip) as f32, clip.length.max(1) as f32)]));
    }
    if cast {
        let more: Vec<EventTrack> = cast_follow(clip, finisher_cast).into_iter().filter(|t| !tracks.iter().any(|o| o.kind == t.kind)).collect();
        tracks.extend(more);
    }
    // (KH1 moves the actor by the motion's H10 when the request has bit 15, `travels`.)
    // (KH1's roll request has it too: KH2's roll moves him by its root, `ROLL`.)
    let travel = matches!(kind, Kind::GroundCombo | Kind::DodgeRoll) && travels(requests[request]);
    let (block, baked) = match bake.map(|b| bake_one(b, kh2, &template, stands_for, (motion, field), frames, &tracks, travel)) {
        Some(Ok(block)) => (block, true),
        Some(Err(e)) => {
            log(&format!("KH2 slots: KH2 motion {motion} not baked ({e}); drawn over its template"));
            (ks::retimed(&template, frames, &tracks).map_err(|e| e.to_string())?, false)
        }
        None => (ks::retimed(&template, frames, &tracks).map_err(|e| e.to_string())?, false),
    };
    log(&format!(
        "KH2 slots: KH2 motion {motion}{} as KH1 slot {slot} (for KH1's slot {stands_for} / request {request:#x}{}, {old} -> {frames} frames): {}",
        if field { " (field)" } else { "" },
        if default { "" } else { ", by KH2's row selection" },
        tracks.iter().map(|t| format!("{}.{} {:?}", t.kind, t.id, t.keys.iter().map(|k| (k.time, k.a)).collect::<Vec<_>>())).collect::<Vec<_>>().join("; ")
    ));
    // (A hit reaction's KH2 voice, 29 / 30 at frame 0, Checked (files), left out: KH1's hit pass plays
    // its own hurt voice for it, `0x1402901F0 (actor, 0 | 1, 0)`, Checked (decomp).)
    let events = clip.events.iter().filter(|e| matches!(e.kind, EventKind::Sound(_) | EventKind::Effect(_)) || (!reaction && matches!(e.kind, EventKind::Voice(_)))).map(|e| (e.frame, e.kind)).collect();
    Ok(Some((Move { motion, stands_for, request, kind, slot, frames, baked, travel: travel && baked, default, field, variant, own: None, events }, block)))
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
/// (each of those is KH2 behaviour KH1's swing row doesn't have), and no guard but one on his
/// body whose ATKP lets the hit go with no effect (`refact_self` 2). Its no-gravity ranges and
/// those guards are KH1's now (`no_gravity`, `shared::physics`'s air step; `body_guard`, `hurt`).
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
        W::Hitbox { weapon: 0, .. } | W::Guard { .. } | W::Invulnerable | W::Air | W::Grounded => true,
        _ => false,
    });
    blade && !other
}

/// A KH2 ability row's motion that hits with shapes on his own body (`Hitbox` windows with weapon
/// 0: Vicinity Break, Explosion, Magnet Burst), which KH1's one sphere on the Keyblade can't do.
/// KH1's row runs it as it runs the Keyblade swings, and KH2's own shapes and hit numbers for those
/// windows are landed through KH1's damage each frame (`takeover::kh2_hits`, `combat::Strikes::
/// strike_as`); its invulnerability (range 27) is KH1's hit pass's (`invulnerable`, `hurt`), its
/// guards on his body that let the hit go are `body_guard`'s, its no-gravity ranges `no_gravity`'s.
/// Not with a guard of another kind, a guard on the Keyblade, or an air / ground range. Sora's
/// (`P_EX100.mset`, `00battle.bin`, Checked (files)): Vicinity Break 170 (1869 on group 21,
/// frames 20..30; guard 406 14..32), Explosion 166 (911 on group 19 32..57, 912 57..64;
/// invulnerable and guard 124, kind 3, `refact_self` 2, 32..64), Magnet Burst 197 (1900 on 22
/// 16..56, 1901 on 23 20..56, 1903 on 3 64..70; guard 406 20..70; no gravity 16..82); Finishing
/// Leap 167 and Counterguard 171 too, but their rows are Square rows (kind 0), not the selector's.
/// Their Keyblade windows (the weapon's mset, not staged here) are KH1's swing's as for the others.
fn body_strike(clip: &character_core::Clip, battle: &kh2_formats::battle::Battle) -> bool {
    use character_core::WindowKind as W;
    let body = clip.windows.iter().any(|w| matches!(w.kind, W::Hitbox { weapon: 0, .. }));
    let lets_go = |attack: u16| battle.attack_for_level(attack, 0, 0).is_some_and(|a| a.refact_self == 2);
    let other = clip.windows.iter().any(|w| match w.kind {
        W::Guard { attack, weapon: 0, .. } => !lets_go(attack),
        W::Guard { .. } | W::Air | W::Grounded => true,
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

/// KH2's invulnerability (range id 27, `WindowKind::Invulnerable`: its hurt shapes off) in our
/// slot playing on `actor` now, at KH1's time in the slot (Explosion 32..64, Counterguard 0..20,
/// Finishing Leap 24..38; Checked (files)). `hurt` lets no hit reach him there, as KH2's hit pass
/// finds no hurt shape of his.
pub unsafe fn invulnerable(actor: usize) -> bool {
    use character_core::WindowKind as W;
    let Some(m) = current(actor) else { return false };
    let Some(clip) = pose::kh2().and_then(|k| k.character.moveset.clips.get(&m.motion)) else { return false };
    let t: f32 = rd(actor + TIME_NOW);
    t >= 0.0 && clip.has(t as u32, |k| matches!(k, W::Invulnerable))
}

/// KH2's guard on his body in our slot playing on `actor` now (`WindowKind::Guard` with weapon 0
/// at KH1's time in the slot), if its ATKP lets the hit go with no effect (`refact_self` 2: KH2's
/// guard reaction is none, the move goes on; Flash Step's 406). `hurt` stops KH1's hits there
/// as KH2's guard does (a guard on his body is all round).
pub unsafe fn body_guard(actor: usize) -> Option<u16> {
    use character_core::WindowKind as W;
    let m = current(actor)?;
    let kh2 = pose::kh2()?;
    let clip = kh2.character.moveset.clips.get(&m.motion)?;
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
    let clip = kh2.character.moveset.clips.get(&m.motion)?;
    let t: f32 = rd(actor + TIME_NOW);
    if !(t >= 0.0) {
        return None;
    }
    let f = t as u32;
    clip.windows.iter().filter(|w| w.contains(f)).filter_map(|w| match w.kind {
        W::Guard { attack, weapon, .. } if weapon > 0 => kh2.battle.attack_for_level(attack, 0, 0).map(|a| (attack, a.refact_self)),
        _ => None,
    }).min_by_key(|g| g.1)
}

/// KH2's clip of our slot playing on `actor` holds him with no gravity now (its range id 3 / 24,
/// `WindowKind::NoGravity`, at KH1's time in the slot, which runs in the clip's 60 Hz frames: the
/// slot is retimed to its length). Not for a clip that allows movement all through (Wisdom
/// Form's shots), as KH2's controller has it (`Character`'s action step). KH1's air step holds
/// his rise or fall there (`shared::physics`).
pub unsafe fn no_gravity(actor: usize) -> bool {
    use character_core::WindowKind as W;
    let Some(m) = current(actor) else { return false };
    let Some(clip) = pose::kh2().and_then(|k| k.character.moveset.clips.get(&m.motion)) else { return false };
    let t: f32 = rd(actor + TIME_NOW);
    if !(t >= 0.0) || !clip.windows.iter().any(|w| matches!(w.kind, W::NoGravity)) {
        return false;
    }
    let moving = clip.windows.iter().any(|w| matches!(w.kind, W::AllowMovement) && w.start == 0 && w.end.is_none());
    !moving && clip.has(t as u32, |k| matches!(k, W::NoGravity))
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
    let clip = pose::kh2()?.character.moveset.clips.get(&m.motion)?;
    let t: f32 = rd(actor + TIME_NOW);
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
    let yaw: f32 = rd(actor + 0x3C);
    let (fw, right) = ([yaw.sin(), yaw.cos()], [yaw.cos(), -yaw.sin()]);
    Some([d[0] * right[0] + d[2] * fw[0], d[1], d[0] * right[1] + d[2] * fw[1]])
}

/// KH2's clip of our slot playing on `actor` has its edge stop now (range id 29,
/// `WindowKind::EdgeStop`, at KH1's time in the slot, the clip's frame): KH2's ground mover keeps
/// him on the ledge there (`shared::physics`'s ground step). Sora's (`P_EX100.mset`, Checked
/// (files)): the finishers 154 (frames 0..38) and 155 (0..39), Guard Break 165 (0..40), Flash Step
/// 169 (0..17), and 161 (0..28).
pub unsafe fn edge_stop(actor: usize) -> bool {
    use character_core::WindowKind as W;
    let Some(m) = current(actor) else { return false };
    let Some(clip) = pose::kh2().and_then(|k| k.character.moveset.clips.get(&m.motion)) else { return false };
    let t: f32 = rd(actor + TIME_NOW);
    t >= 0.0 && clip.has(t as u32, |k| matches!(k, W::EdgeStop))
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
    let Some(clip) = pose::kh2().and_then(|k| k.character.moveset.clips.get(&m.motion)) else { return };
    let t: f32 = rd(actor + TIME_NOW);
    if !(t >= 0.0) {
        return;
    }
    let Some(rate) = clip.open(t as u32).find_map(|w| match w.kind {
        W::TurnToLock(r) => Some(r),
        _ => None,
    }) else {
        return;
    };
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
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 40 {
        log(&format!("KH2's turn to the target (KH2 motion {} frame {:.0}, up to {:.2} a frame): {:.2} -> {f:.2} (the target at {:.2})", m.motion, t, max, yaw, want));
    }
}

/// KH2's cast motion our slot plays for KH1's spell request `request` (`CASTS`), if built.
pub fn cast_motion(request: usize) -> Option<u16> {
    built()?.moves.iter().find(|m| m.request == request && m.default && matches!(m.kind, Kind::Magic | Kind::AirMagic)).map(|m| m.motion)
}

/// KH2's `motion` has a slot of ours on one of KH1's requests (built: an ability row's only when
/// it is a Keyblade swing, `swing_rows`).
pub fn has_any(motion: u16) -> bool {
    built().is_some_and(|b| b.moves.iter().any(|m| m.motion == motion))
}

/// KH2's `motion` baked into a KH1 motion on `template` (KH1's slot `stands_for`, whose own
/// animation of KH1's own parts it keeps, `Baker::poses`; `kh1_data::bake`), with its report
/// logged. `travel`: the move's request has KH1's travel flag (`travels`): KH2's root travel goes
/// in its H10.
#[allow(clippy::too_many_arguments)]
fn bake_one(b: &pose::Baker, kh2: &pose::Kh2, template: &[u8], stands_for: usize, (motion, field): (u16, bool), frames: u32, tracks: &[EventTrack], travel: bool) -> Result<Vec<u8>, String> {
    let t = std::time::Instant::now();
    let (poses, moved, scales) = b.poses(kh2, motion, field, frames, travel, Some(stands_for)).ok_or("KH2 has no animation for it")?;
    let (block, r) = kh1_data::bake::baked(template, frames, tracks, &b.skeleton, &poses, &moved, &scales).map_err(|e| e.to_string())?;
    log(&format!(
        "KH2 slots: KH2 motion {motion} baked ({frames} frames, {} bytes, in {:.0} ms): {} keyed channels, {} keys, {} constants, {} bones built once; travel {} ({} H10 keys); KH1's FK from it within {:.3} units of the pose (worst bone {})",
        block.len(),
        t.elapsed().as_secs_f32() * 1000.0,
        r.keyed_channels,
        r.keys,
        r.constant_channels,
        r.static_bones,
        moved.last().map_or("none (KH1 doesn't move it)".to_string(), |v| format!("{:.1?} model units", v.to_array())),
        r.travel_keys,
        r.max_error,
        r.worst_bone
    ));
    Ok(block)
}

/// KH1's events for a KH2 clip on a retimed template (`r`: new length / the template's): the
/// clip's hit windows on the Keyblade as KH1's hit window (attack group 0, as KH1's own combo
/// motions: its row's one sphere on the blade), its combo window, its trail (else its Keyblade hit
/// windows from 4 frames before); the template's sounds, foot effects, face changes and the point
/// its action may be left, rescaled. Hit windows on his body (`body_strike`) are not KH1's sphere's:
/// KH2's own shapes land them (`takeover::kh2_hits`).
///
/// A spell's cast (`cast`, `CASTS`): no "may be left" point of the template's (Blizzard's 55 has
/// one at 68 of 86, Fire's second part 57 at 46 of 60, Checked (files)): KH2's cast motions hold
/// him to their end (none has a free range, id 0 / 1; `P_EX100.mset`, Checked (files)), where
/// KH1's queue takes him on. (KH2's magic-only window, range id 35, is not KH1's yet.)
fn kh1_tracks(template: &[EventTrack], clip: &character_core::Clip, r: f32, own: bool, cast: bool) -> Vec<EventTrack> {
    let len = clip.length as f32;
    let span = |w: &character_core::Window| (w.start as f32, w.end.map_or(len, |e| e as f32));
    let hits: Vec<(f32, f32)> = clip.windows.iter().filter(|w| matches!(w.kind, WindowKind::Hitbox { weapon, .. } if weapon > 0)).map(span).collect();
    let combo: Vec<(f32, f32)> = clip.windows.iter().filter(|w| matches!(w.kind, WindowKind::Combo)).map(span).collect();
    let mut trail: Vec<(f32, f32)> = clip.windows.iter().filter(|w| matches!(w.kind, WindowKind::WeaponTrail)).map(span).collect();
    if trail.is_empty() {
        trail = hits.iter().map(|&(a, b)| ((a - 4.0).max(0.0), b)).collect();
    }
    // On a request of ours (`own`, `kh1rows`), KH2's "free" state (range id 0 / 1 / 2,
    // `WindowKind::Grounded` / `Air` / `Free`: control comes back there, Aerial Recovery's from
    // frame 24, the guard's from 52, its repel's from 40;
    // `docs/aerial-recovery.md`) as KH1's "may be left" (type 0, `+0x1CC` bit 20: `0x14029D399`
    // clears the action flags `0x1004`) from its start, in place of the template's; a clip with
    // none keeps the template's, as every slot on KH1's own requests does.
    let free: Vec<(f32, f32)> = if own || cast { clip.windows.iter().filter(|w| matches!(w.kind, WindowKind::Air | WindowKind::Grounded | WindowKind::Free)).map(span).collect() } else { Vec::new() };
    let mut out: Vec<EventTrack> = ks::scaled(template, r).into_iter().filter(|t| matches!(t.kind, ks::FOOT | ks::SOUND | ks::FACE) || (t.kind == ks::CAN_LEAVE && free.is_empty() && !cast)).collect();
    if !free.is_empty() {
        out.push(EventTrack::level(ks::CAN_LEAVE, 0, &free));
    }
    for (kind, w) in [(ks::HIT, hits), (ks::COMBO, combo), (ks::TRAIL, trail)] {
        if !w.is_empty() {
            out.push(EventTrack::level(kind, 0, &w));
        }
    }
    out
}

/// The frame KH2's dodge roll may be left from: its fall-check event (the OG rewrite's DodgeRoll
/// action: other actions may cut it there and the stick walks out of it; frame 34 for Sora), else
/// its last frame. KH1's "may be left" (type 0) from there clears the action flags, so KH1's
/// control step walks, jumps, rolls or attacks out of it.
fn roll_free(clip: &character_core::Clip) -> u32 {
    let len = clip.length.max(1);
    clip.events.iter().find(|e| e.kind == EventKind::FallCheck).map_or(len - 1, |e| e.frame).min(len - 1)
}

/// What may follow a KH2 cast on KH1's cast state (the tester, thread kh2-magic-and-reflect-native,
/// answer 1 (a): the cast counts as a step and the combo goes on after it). KH2's rule (the
/// controller's Magic action, `character_core`): from the cast motion's reaction-command window
/// (range id 14, `AllowReactionCommand`) other actions may cut it and an Attack goes on with the
/// combo; with none, only on its last frame. Only presses cut it there (jump, the action button,
/// Attack), not the stick. On KH1's side: the motion's combo window (type 5, `+0x1CC` bit 22, in
/// which KH1's menu takes a command with the action flag still set, `0x1402A2EC0`, and KH1's
/// Attack step `0x1402A7510` counts on from `+0x41`, Checked (decomp)) from there to the end; no
/// "may be left" (type 0 clears the action flags, `0x14029D399`, and the stick would walk out of
/// the cast, which KH2's doesn't): a fresh jump or Square there ends the action flag for its own
/// move (`cast_cut_open`, `formmagic::cast_cut`). A finisher cast that ends the combo has no
/// combo window: KH1's Attack starts a new one (its jump and Square still cut it). Spells follow
/// only from KH2's magic window (range id 35, `ComboMagic`): `cast_magic_window`, in
/// `combat::command`.
fn cast_follow(clip: &character_core::Clip, finisher: bool) -> Vec<EventTrack> {
    if finisher {
        return Vec::new();
    }
    vec![EventTrack::level(ks::COMBO, 0, &[(cut_from(clip) as f32, clip.length.max(1) as f32)])]
}

/// The first frame of a KH2 cast that presses may cut (range id 14; with none, its last frame).
fn cut_from(clip: &character_core::Clip) -> u32 {
    let len = clip.length.max(1);
    clip.first(|k| matches!(k, WindowKind::AllowReactionCommand)).unwrap_or(len - 1).min(len - 1)
}

/// Our cast slot playing on `actor` is in its cut window (`cut_from` on).
pub unsafe fn cast_cut_open(actor: usize) -> bool {
    let Some(m) = current(actor).filter(|m| matches!(m.kind, Kind::Magic | Kind::AirMagic)) else { return false };
    let Some(clip) = pose::kh2().and_then(|k| k.character.moveset.clips.get(&m.motion)) else { return false };
    let t: f32 = rd(actor + TIME_NOW);
    t.is_finite() && t >= cut_from(clip) as f32
}

/// Our cast slot playing on `actor`: whether a spell may follow it now (KH2's magic window,
/// range id 35, `ComboMagic`, from its first frame; none: not during the cast). `None`: no cast
/// of ours plays.
pub unsafe fn cast_magic_window(actor: usize) -> Option<bool> {
    let m = current(actor).filter(|m| matches!(m.kind, Kind::Magic | Kind::AirMagic))?;
    let clip = pose::kh2()?.character.moveset.clips.get(&m.motion)?;
    let t: f32 = rd(actor + TIME_NOW);
    let from = clip.first(|k| matches!(k, WindowKind::ComboMagic));
    Some(t.is_finite() && from.is_some_and(|f| t >= f as f32))
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
    /// `slot`), whether it is the request's own).
    redirects: Vec<(usize, u32, u32, Kind, usize, bool, bool)>,
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
/// Our slot playing on Sora (`usize::MAX` none), and its KH2 motion (for `kh2fx::tick`).
static PLAYING: AtomicUsize = AtomicUsize::new(usize::MAX);
static PLAYING_MOTION: AtomicU32 = AtomicU32::new(u32::MAX);

fn built() -> Option<&'static Built> {
    BUILT.get()?.as_ref()
}

fn moved(slot: usize) -> Option<&'static Move> {
    built()?.moves.iter().find(|m| m.slot == slot)
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
    entry & 0x8000 != 0
}

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
unsafe fn place(b: &Built) -> Placed {
    let mut motions = Vec::new();
    let mut at = Vec::new();
    for (m, block) in b.moves.iter().zip(&b.blocks) {
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
    }
    if let Ok(mut v) = MOTION_AT.lock() {
        *v = at;
    }
    log(&format!("KH2 slots: {} motions placed (handles {:08x?})", motions.len(), motions.iter().map(|m| m.1).collect::<Vec<_>>()));
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
            if N.fetch_add(1, Ordering::Relaxed) < 20 {
                log(&format!("KH2 slots: set at {at:#x} ({why}) not extended: {e}"));
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
                redirects.push((m.request, e, (count + i) as u32, m.kind, m.slot, m.default, true));
                if let Some(x) = requests.get_mut(m.request).filter(|_| m.default) {
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
        let holds = own < count && pose::file_slot_of_motion(pose::ptr(slots[own])) == Some(m.stands_for);
        if holds {
            redirects.push((m.request, e, (count + i) as u32, m.kind, m.slot, m.default, false));
        } else if m.variant == 0 {
            missing.push(format!("{:#x}", m.request));
        }
    }
    let t = Set { kh1_slots: sh, kh1_requests: rh, slots_handle: handle(slots.as_ptr() as usize), requests_handle: handle(requests.as_ptr() as usize), requests, redirects };
    log(&format!(
        "KH2 slots: Sora's set {f:#x} ({why}): {count} slots of its own, ours at {}..{}, {} requests; ours stand in for {}; our own requests {}{}",
        count,
        count + p.motions.len() - 1,
        len,
        t.redirects.iter().filter(|r| r.5).map(|r| format!("{:#x} (its slot {})", r.0, r.1 & 0xFFF)).collect::<Vec<_>>().join(", "),
        if own_requests.is_empty() { "none".to_string() } else { own_requests.join(", ") },
        if missing.is_empty() { String::new() } else { format!("; not in this set: {}", missing.join(", ")) }
    ));
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
}

/// Each frame Sora is posed (`pose`, the game thread): ours on the set bound and the one KH1
/// keeps (in case a bind came before our motions were built), the requests pointed at ours by
/// their moves' styles in every set, and our slot's KH2 sounds and effects.
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
    finisher_window(actor);
    // KH2's Counterguard window in its guard's repel (`combomods::counter_window`).
    super::combomods::counter_window(actor);
    // KH2's hits on his body in the KH2 motion KH1 plays (`body_strike`).
    super::takeover::kh2_hits(actor);
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

/// The cast KH2 picked for each of KH1's spell requests (`CASTS`; our move's `slot`,
/// `usize::MAX`: the request's own).
static CAST_PICKED: [AtomicUsize; CASTS.len()] = [const { AtomicUsize::new(usize::MAX) }; CASTS.len()];

/// KH1's spell request `request` plays KH2's finisher cast from now (`finisher`, its slot built:
/// `CAST_FINISHERS`) or its own cast (the entry pointed there at once, before KH1's command step
/// asks for it; `formmagic::pick_cast`). Returns the KH2 motion it plays.
pub unsafe fn pick_cast(request: usize, finisher: bool) -> Option<u16> {
    let i = CASTS.iter().position(|c| c.2 == request)?;
    let fin = CAST_FINISHERS.iter().find(|f| f.2 == request).and_then(|f| built()?.moves.iter().find(|m| m.request == request && m.motion == f.0 && !m.default)).filter(|_| finisher);
    CAST_PICKED[i].store(fin.map_or(usize::MAX, |m| m.slot), Ordering::Relaxed);
    if let Ok(mut g) = PLACED.try_lock() {
        if let Some(p) = g.as_mut() {
            redirect(p);
        }
    }
    fin.map(|m| m.motion).or_else(|| cast_motion(request))
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
    let m = built().and_then(|b| b.moves.iter().find(|m| m.request == request && m.motion == motion)).map(|m| m.slot);
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
/// finisher. KH2 takes the Square moves there too (either bit, `0x1403A9950`). Sora's (Checked
/// (files)): 154 / 155 35..64, 183 21..50, 184 23..52, Explosion 60..72, Guard Break 48..60,
/// Aerial Finish 89..108, Magnet Burst 70..86.
unsafe fn finisher_window(actor: usize) {
    use character_core::WindowKind as W;
    let Some(m) = current(actor) else { return };
    let Some(clip) = pose::kh2().and_then(|k| k.character.moveset.clips.get(&m.motion)) else { return };
    let t: f32 = rd(actor + TIME_NOW);
    if !(t >= 0.0) || !clip.has(t as u32, |k| matches!(k, W::Finisher)) || super::takeover::with_abilities(|a| a.finishing_plus > 0) != Some(true) || !super::shared::active() {
        return;
    }
    let inst = pose::ptr(rd::<u32>(actor + 0x15C));
    if inst < 0x10000 {
        return;
    }
    wr(inst + 0x14, rd::<u32>(inst + 0x14) | super::combomods::KH1_COMBO_WINDOW);
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 20 {
        log(&format!("KH2's Finishing Plus window on KH2 motion {} (frame {t:.0}): KH1's combo window", m.motion));
    }
}

/// The requests by style (base Sora under shared states; a change applies from KH1's next
/// request, never to the motion playing), each to its own move or the one KH2's row selection
/// picked for it.
unsafe fn redirect(p: &mut Placed) {
    let on = super::shared::on();
    for t in p.sets.iter_mut() {
        for i in 0..t.redirects.len() {
            let (req, kh1, ours, kind, id, default, own) = t.redirects[i];
            let want = picked(req).filter(|m| t.redirects.iter().any(|r| r.0 == req && r.4 == *m));
            if want.map_or(!default, |m| m != id) {
                continue;
            }
            if let Some(e) = t.requests.get_mut(req) {
                // (A request of ours is ours whatever the styles.)
                *e = if own || on && (super::styles::kh2(kind) || (forced(req) && want == Some(id))) { (kh1 & !0xFFF) | ours } else { kh1 };
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
    let m = motion.and_then(|mo| built()?.moves.iter().find(|m| m.request == request && m.motion == mo)).map(|m| m.slot);
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
    built().is_some_and(|b| b.moves.iter().any(|m| m.request == request && m.motion == motion))
}

/// The KH2 motion of our slot `actor` plays now (KH2's anti-repeat passes over it, `pick`).
pub unsafe fn playing_motion(actor: usize) -> Option<u16> {
    current(actor).map(|m| m.motion)
}

/// Our slot's KH2 sounds, voices and effects between KH1's last time and its time now.
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
        if N.fetch_add(1, Ordering::Relaxed) < 40 {
            log(&format!(
                "KH2 slots: KH1 plays ours, KH2 motion {} (for KH1's {} / request {:#x}; its slot {} in the set; Sora's request {:#x})",
                m.motion,
                m.stands_for,
                m.request,
                rd::<u32>(actor + SLOT_NOW),
                rd::<u32>(actor + 0x164)
            ));
        }
    }
    *l = Some((slot, now));
    for (f, e) in &m.events {
        let f = *f as f32;
        if f > from && f <= now {
            super::kh2sound::event(m.motion, *e);
            super::kh2fx::event(m.motion, *e);
        }
    }
    super::kh2sound::tick();
}
