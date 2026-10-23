//! KH2's effects played by KH1's own effect system (t368, the effects port's step 1; project notes
//! `kh2-effects-in-kh1.md`). KH1 and KH2 share the effect engine: the same DPX (`0x82`) and DPD
//! (`0x96`) files, the same particle programs (1..235) and the same 30 Hz effect clock. So a KH2
//! effect is made into a KH1 DPX once (`kh2_formats::kh1dpx::build`: its textures moved into
//! KH1's own-texture window, the KH2 common textures it draws with copied in, programs KH1 lacks
//! set to its dummy program) and started with KH1's own calls, on Sora's KH1 bones. KH1 then
//! steps, binds, textures, draws, pauses and ends it like any of its own effects; nothing of
//! ours runs per frame.
//!
//! Step 1: Explosion's wrist flashes (Sora's package `obj/P_EX100.a.us`, elements 46 and 47:
//! effect 33 on KH2 bones 170 and 78), which use only programs KH1 has. KH2's bones are put on
//! KH1's by the bind poses (option A, as the base attacks): KH2 170 -> KH1 93, KH2 78 -> KH1 198.
//!
//! (Steps 3-4) Explosion's big flare (element 44, effect 31, KH2's weapon point 0x4003: KH1's point
//! along the Keyblade, binding mode 3) and dust burst (element 45, effect 32, KH2 bone 67: KH1 bone
//! 70) too, once KH2's MegaBirth is plugged into KH1's program machine (`kh2ppp`; Steam only).
//! And the burst (element 48, effect 34, KH2's object root 0x4000: KH1's 0x8000, bound until it
//! starts as KH2's flag 0x02 asks), which also runs KH2's ribbon (241).
//!
//! (Step 2) The element layer, in KH1's own instance fields (Steam decomp; notes in
//! `docs/kh2-effects.md` §16):
//! - Start wait: KH1's instance has one, `+0xD0` (game frames; counted down by KH1's effect
//!   pass, which neither steps nor draws the instance until it runs out, `0x1401ECB50` /
//!   `0x1401ECF30`; KH1 seeds it from the particle header's `+0x108` at start, `0x14031D2C0`).
//!   It is set to the KH2 element's wait (+ 0.1 when bound, as KH2 adds) after the start, less
//!   the 2 game frames KH1 adds to an element with a bone binding (the tester: exact to KH2).
//! - The element's offset and rotation: folded into a copy of its particle data's header local
//!   transform (`+0x80` / `+0x90`), which KH1 puts under the root its bone binding gives
//!   (`0x14031CEA0`), as its own effect number (`ELEMENT_NUMBER`); its scale into the instance's
//!   scale (`+0x20`), KH2's root scale.
//! - Ending: KH2's motion changing (flag 0x20) or Sora's object going (0x200, `owner_gone`) ends
//!   it the KH2 way: 0x80 sets KH1's "ended" byte (`+0x91`: no more loops, as KH1's own fades
//!   do); 0x40 fades KH1's instance colour alpha (`+0x3C`, which KH1's draws multiply every
//!   vertex colour by) to 0 over the element's fade-out frames, then stops it with KH1's stop
//!   (`0x1401EC470`). Stepped from `kh2fx::tick` (Sora's frames, KH2's motion).
//!
//! KH1 calls (Steam): the DPX relocation `0x1401E0080` (each effect's DPD offset to a handle,
//! each particle data's programs to KH1's functions; once per DPX), and the start on a bone
//! `0x1402ADCF0 (dpx, effect number, actor, bone)` (an element from KH1's list, its effect
//! instance from KH1's pool, bound to the bone each frame; bone `0x2000 | n` is bone `n`).
//!
//! (t369) Textures. KH1's PC port never draws a DPD's PS2 texture data: for each texture record
//! it asks for the HD texture of the loaded file the record lies in (`0x14010C050`: the file
//! from KH1's loaded-file list whose data holds the pointer, `0x1400E2B20`; then that file's HD
//! entry whose `+0x54` is the record's pixel offset in the file, `+0x20` of the record; KH1 runs
//! with the offset keying on, set at start in `axaAppMain`). So the DPX is entered in that list
//! the way KH1 enters a file it loads (`0x1400E1680` / `0x1400E2F60`): a resource entry for our
//! buffer with an HD entry per texture record, holding the KH2 HD image (DDS) KH2 itself draws
//! for that texture (the remastered assets of `obj/P_EX100.a.us` and `effect/texcommon.dpd`,
//! matched by the offset each asset's header gives at `+0x24`; t370: t369 took `+0x20`, which is
//! the asset's own place in the stream). KH1's own loader then makes the textures.
//! The vtables are copied from KH1's own entries; memory comes from the game's CRT (KH1 frees
//! HD data with `_aligned_free` if a file is ever dropped; ours never is).
//!
//! The bone binding's frame (`docs/kh2-effects.md` §17.7): both games place a bound effect from
//! the bone matrix their binding read on its previous run, but KH2's first run after the start
//! reads the bone first (this frame's pose) and keeps reading it while the effect waits, where
//! KH1's applies the matrix its start read (before this frame's pose pass) and doesn't read
//! while the effect waits. So for our continuous bindings (modes 2 and 3) the binding's stored
//! matrix is refreshed with KH1's own bone reads (`0x14029EA00` / `0x14029EB20`, Steam) from
//! Sora's skeleton pass (`pose`): after it on the start frame, before it on the frames the effect
//! still waits. KH1's binding then runs as it does; its own effects are not touched.
//!
//! The old effect player (`kh2fx`) is retired for these elements on Steam (the tester): it plays
//! them only while the DPX isn't built (or couldn't be), if KH2's programs couldn't be plugged
//! in, or on another build (`event`).
//!
//! (The port of every effect, `docs/kh2-effects.md` §18.) Every element of Sora's package is
//! built, not only Explosion's: one DPD per motion of KH2 Sora's moveset (the elements its frame
//! triggers start, which play together and so share one run of KH1's texture slots), the rest one
//! per effect. An element plays through KH1 once it is switched on here (`ON`, one move per
//! version), its KH2 bone has a KH1 anchor, its flags are ones KH1's binding does, and every
//! program its effect runs is one KH1 runs the same as KH2 (`KH1_SAME`, checked program by
//! program) or one of KH2's own that is plugged in (`kh2ppp`). Anything else stays with `kh2fx`,
//! logged once.
//!
//! (Lean and lazy, `docs/kh2-effects.md` §18.12.) A package is built (in the background) when its
//! effects can play (`manage`: Sora's and the Keyblade's at load, Sora's only for the moves
//! `kh2_effects` lets play; a form's as Sora changes into it; a spell's at the level he has),
//! entered in KH1's loaded files as soon as it is built, and freed once it can't play and none of
//! its effects is left in KH1 (KH1's own file removal). Each DPX sits in its own pages of one
//! reserved 32 MB region (one of KH1's handle regions for all of them); its own HD images are
//! handed to KH1 when it is entered and not kept; KH2's common images are made into KH1 textures
//! once, from a small file of ours, and shared by every package (KH1's texture lookup returns an
//! HD entry's texture as it is). Until a package is built, `kh2fx` plays its effects on the same
//! frame.

use super::addr::Rva;
use super::log;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

const RELOCATE_RVA: Rva = Rva(0x1DDF20);
const START_ON_BONE_RVA: Rva = Rva(0x2ABB60);
/// KH1's "which loaded file holds this pointer" (`0x1400E2B20`); its first instruction loads
/// the loaded-file list (`mov r11, [rip + disp]`).
const FILE_OF_RVA: Rva = Rva(0xE2B10);
/// Bone parameter bit: the low 12 bits are the bone's index in the actor's skeleton.
const RAW_BONE: u32 = 0x2000;
/// Where KH1 binds a native element (option A: KH2's bone on KH1's matching bone).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Anchor {
    /// One of Sora's KH1 bones (`0x2000 | bone` to KH1's start on a bone, its binding mode 2).
    Bone(u32),
    /// A point along the Keyblade, `t` of the way from the blade's start to its end (KH1's bone
    /// code 0x30 through its binding mode 3, `0x1402ADD60`, Steam).
    Blade(f32),
    /// One of KH1's own anchor codes, as it is (0x8000: the object root, `0x14029EA00` /
    /// `0x14029EB20`), through its binding mode 2.
    Code(u32),
}

/// The elements of Sora's package switched on for KH1 (the tester: every effect, one move per
/// version): Explosion (motion 166: 44..48), Magnet Burst (197: 51, 52), Vicinity Break (170: 49,
/// 50) and Aerial Finish (194: 35..38), the last two with KH2's pppVtMime; then the rest of his
/// moves', with KH1's position binding for KH2's flag 0x04: Guard Break (165: 30..32), Finishing
/// Leap (167: 33, 34), Aerial Recovery (40: 0), and under `kh2_effects = all` motions 169 (53..56),
/// 171 (40, 41) and 172 (39); then the Drive motion's (52: 1..13) and Revert's flashes (15..27,
/// `form`). Element 1 runs KH2's pppSnoMdlDttNorm (238), not in KH1: `missing` leaves it to kh2fx.
const ON: &[i16] = &[
    44, 45, 46, 47, 48, 51, 52, 49, 50, 35, 36, 37, 38, 30, 31, 32, 33, 34, 0, 53, 54, 55, 56, 40, 41, 39, //
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
];
/// The Keyblade's package's elements switched on: its vanish flash (`weapon_vanish`).
const WEAPON_ON: &[i16] = &[super::kh2fx::WEAPON_VANISH as i16];
/// The forms' packages (Sora's skeleton: his bones' anchors). Their weapon points stay with
/// `kh2fx` (open with the tester): 0x4003 / 0x4005 / 0x4007 the first Keyblade's (KH1's blade
/// binding would turn them with the blade, KH2's weapon bone; `kh2fx` turns them with Sora's
/// facing, as the tester asked for Valor's grounded finisher), 0x4004 / 0x4006 / 0x4008 the
/// second's (KH1 has no second Keyblade to bind to).
fn form_anchor(code: u16) -> Option<Anchor> {
    if (0x4003..=0x4008).contains(&code) {
        return None;
    }
    anchor(code)
}

/// The spell packages' anchors, as `kh2fx`'s (Sora's bones and object root, the Keyblade's
/// points) but for KH2's body centre 0x4001 (the root bone's place, unturned), for which KH1 has
/// no anchor found yet: kh2fx keeps those.
fn spell_anchor(code: u16) -> Option<Anchor> {
    anchor(code)
}

/// The Keyblade's package names its own (weapon skeleton) bones; KH2's call of its vanish flash
/// isn't traced, and `kh2fx` puts every code half way along the blade, turned as the hand. KH1's:
/// the point half way along its blade (binding mode 3, t = 0.5), turned as the blade bone.
fn weapon_anchor(_code: u16) -> Option<Anchor> {
    Some(Anchor::Blade(0.5))
}
const MB: u32 = super::kh2ppp::MEGA_BIRTH;
const SHIPOLY: u32 = super::kh2ppp::SHIPOLY;
const COLOR: u32 = super::kh2ppp::COLOR;
const VT_MIME: u32 = super::kh2ppp::VT_MIME;
/// KH2's own programs an effect needs plugged in before it plays through KH1 (KH1's versions of
/// 49, 157, 201 and 214 aren't KH2's; 241 isn't in KH1). KH2's pppColor (43) is not a gate:
/// without it KH1's runs.
const GATES: [u32; 5] = [MB, SHIPOLY, VT_MIME, super::kh2ppp::ANGLE_LOOP, super::kh2ppp::TAIL_3X];

/// Program types KH1 runs the same as KH2 (`docs/kh2-effects.md` §18.2): every slot of KH1's
/// table entry (`0x14054FF00`) against KH2's (`0x140779A00`), normalised decompiles, where both
/// are in the decomps; KH1's code read in the exe where they aren't. Differences allowed: the
/// paused byte's offset in the effect (KH1 `+0xD5`, KH2 `+0xC1`) and how the decompiler wrapped
/// lines. 1 is `pppDummyFunc`.
const KH1_SAME: &[u32] = &[
    1, // pppDummyFunc
    28, 29, 30, 31, // Accele, AngAccele, SclAccele, ColAccele
    36, 37, 38, 39, // Move, AngMove, SclMove, ColMove
    40, 41, 42, // Point, Angle, Scale
    51, 63, 66, // RandFloat, RandFV, RandIV
    84, 87, 90, 91, 94, 96, // SMatrix, MatrixXYZ, MatrixYXZ, MatrixZXY, MatrixScl, RyjMatrixNoRot
    105, 106, // ParMatrix, KeParMatR
    112, 113, 115, 116, 117, 120, // DrawMatrix, DrawMatrixFront, DrawMatrixWood, KeDMat, KeDMatFr, RyjDrawMatrixWorld
    124, 125, // KeZCrct, KeZCrctShp
    137, 138, 139, 142, // DrawMdl3, DrawMdlSemi3, DrawMdlTs3, DrawShapeX
    160, // KeShpDtt
    174, 177, 184, // VertexAp, VertexApLc, KeBornRnd6
    217, 219, // DrawMatrixLoop, DrawMdlLoopZ
    // (The spells', §18.9.)
    75, 76, 85, // SRandFV, SRandUpFV, KeMatSN
    155, // KeShpTail2X
    210, 211, 213, 215, 218, // MoveLoop, AngMoveLoop, PointLoop, ScaleLoop, DrawMdlLoop
];

/// The programs of `types` KH1 can't run as KH2 does yet (empty: it can play through KH1).
fn missing(types: &[u32]) -> Vec<u32> {
    types.iter().copied().filter(|t| !KH1_SAME.contains(t) && *t != COLOR && !(GATES.contains(t) && super::kh2ppp::plugged(*t))).collect()
}

/// KH1's anchor for a KH2 bone code. The bones by the two bind poses (`docs/kh2-effects.md`
/// §17.4, §18.3: each KH2 bone on the KH1 bone of the same body part at the same joint, whose
/// bind frame is the same to 0.05): root 0 -> 0, pelvis 2 -> 2; +x leg: thigh 11 -> 43, shin 12 -> 44, foot 15
/// -> 47, toe 18 -> 50; -x leg: 40 -> 17, 41 -> 18, 44 -> 21, 47 -> 24; spine 67 -> 70; upper arms
/// 78 -> 198, 170 -> 93; hands 85 -> 203, 177 -> 98. KH2's weapon points 0x4003 / 0x4005 / 0x4007
/// (tip, root, halfway: `0x1403B43D0`) are KH1's blade points t = 1 / 0 / 0.5; its object root
/// 0x4000 is KH1's 0x8000.
fn anchor(code: u16) -> Option<Anchor> {
    Some(match code {
        0 => Anchor::Bone(0),
        2 => Anchor::Bone(2),
        11 => Anchor::Bone(43),
        12 => Anchor::Bone(44),
        15 => Anchor::Bone(47),
        18 => Anchor::Bone(50),
        40 => Anchor::Bone(17),
        41 => Anchor::Bone(18),
        44 => Anchor::Bone(21),
        47 => Anchor::Bone(24),
        67 => Anchor::Bone(70),
        78 => Anchor::Bone(198),
        170 => Anchor::Bone(93),
        85 => Anchor::Bone(203),
        177 => Anchor::Bone(98),
        0x4000 => Anchor::Code(KH1_OBJECT_ROOT),
        0x4003 => Anchor::Blade(1.0),
        0x4005 => Anchor::Blade(0.0),
        0x4007 => Anchor::Blade(0.5),
        _ => return None,
    })
}
/// KH1's start on a point along the weapon (Steam `0x1402ADD60 (dpx, number, actor, bone, t)`,
/// binding mode 3: the bone's turn and the point, every frame) and its blade bone code (KH1's
/// `0x14029EB20`: code 0x30 maps `t` into the weapon's blade range, `btltbl` `+0x28`..`+0x24`).
const START_ON_BLADE_STEAM: usize = 0x2ADD60;
/// KH1's start bound by position (Steam `0x1402ADC20 (dpx, number, actor, code, t)`, binding
/// mode 1: every frame the point `0x14029EB20 (actor, code, t)` plus the binding's world offset
/// `+0x10` as the effect's place; its turn is left as the start made it). A raw bone is
/// `0x1000 | n` for the point (`0x14029FAB0`: bit 0x1000 takes the low 12 bits as the bone; the
/// point at t = 1 is the bone's place, `0x1401D6320`), `0x2000 | n` for its matrix (`0x14029EA00`).
const START_BY_POSITION_STEAM: usize = 0x2ADC20;
const RAW_POINT: u32 = 0x1000;
const BLADE_BONE: u32 = 0x30;
/// KH1's object-root code: the actor's place with a frame from its matrix rows (each negated,
/// `+0xF0`, `+0xE0`, `+0xD0`), KH2's 0x4000.
const KH1_OBJECT_ROOT: u32 = 0x8000;
/// KH1's bone-code bit for "place once, when the wait is over, then let go" (the binding task
/// `0x1402AE910`: with it, its first run once the element no longer waits places it and ends):
/// KH2's flag 0x02, bound only until started.
const BIND_ONCE: u32 = 0x1_0000;
const BIND_UNTIL_STARTED: u32 = 0x02;
/// The effect number in our DPX of a KH2 element's own copy of its effect (its offset and
/// rotation folded in): `0x700 | element` (KH1's element start keeps the low 11 bits).
const ELEMENT_NUMBER: u32 = 0x700;
/// KH1's stop of an element's effect (Steam `0x1401EC470 (element)`: its sound and instances
/// freed, the instance marked dead; what KH1's bone binding calls when its motion ends).
const STOP_STEAM: usize = 0x1EC470;
/// KH2 element flags (`docs/kh2-effects.md` §2).
const BIND: u32 = 0x01;
const ENDS_WITH_MOTION: u32 = 0x20;
const END_FADE: u32 = 0x40;
const END_LOOP: u32 = 0x80;
const ENDS_WITH_OWNER: u32 = 0x200;
const POSITION_ONLY: u32 = 0x04;
const BONE_SCALE: u32 = 0x100;
const TWO_POINT: u32 = 0x1_0000;
/// KH1's effect instance (0xE0 bytes) and element (0x3C) fields used here.
const INST_SCALE: usize = 0x20;
const INST_ALPHA: usize = 0x3C;
const INST_ENDED: usize = 0x91;
const INST_STATE: usize = 0x92;
const INST_ELEMENT: usize = 0xB0;
const INST_WAIT: usize = 0xD0;
const EL_FLAGS: usize = 0x14;
const EL_INSTANCE: usize = 0x2C;
const EL_SERIAL: usize = 0x30;
/// Element flag: it has a bone binding (set by KH1's binding, `0x1402AE880`).
const EL_BOUND: u32 = 0x20;
/// KH1's frame unit (Steam `0x14233FBCC`, 1.0, set by `0x14018C2A0`; the frame step
/// `0x14233FBDC` is a multiple of it): a bound element starts at wait <= -2 x it.
const FRAME_UNIT_STEAM: usize = 0x233FBCC;

/// KH1's binding task list (Steam `0x142DB34B0`: `+0x18` the node added last; a node's `+0x10`
/// callback, `+0x18` actor, `+0x20` element, `+0x28` its serial, `+0x30` a handle of the 0x70-byte
/// binding record: `+4` mode, `+8` bone code, `+0xC` t, `+0x30` the matrix it applies, its
/// place row `+0x60`), filled by `0x1402AE880`; the binding callback `0x1402AE910`.
const BIND_LIST_STEAM: usize = 0x2DB34B0;
const BIND_TASK_STEAM: usize = 0x2AE910;
/// KH1's bone reads, as its binding calls them: `0x14029EA00 (actor, code, out matrix)` and
/// `0x14029EB20 (actor, code, t, out point)` (Steam).
const READ_BONE_STEAM: usize = 0x29EA00;
const READ_POINT_STEAM: usize = 0x29EB20;
/// Element flag: waiting to start (KH1's binding neither reads nor applies while it is set).
const EL_WAITING: u32 = 0x10;

/// KH1's frame unit (read on Steam; 1.0, the value KH1 sets it to, elsewhere).
fn frame_unit() -> f32 {
    if super::addr::Build::current() != super::addr::Build::Steam {
        return 1.0;
    }
    let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    let v = unsafe { rd::<f32>(base + FRAME_UNIT_STEAM) };
    if v.is_finite() && v > 0.0 {
        v
    } else {
        1.0
    }
}

/// What a native element does beside its start (from the KH2 PAX element).
#[derive(Clone, Copy, Debug)]
struct Layer {
    el: i16,
    number: u32,
    anchor: Anchor,
    flags: u32,
    /// With KH2's flag 0x04 (bound by position only): the element's offset in the bone's frame
    /// (KH2's effect sits at `B · offset`, turned by the element's rotation alone).
    bone_offset: [f32; 3],
    wait: f32,
    fadeout: f32,
    scale: [f32; 3],
}

/// A native element started: KH1's element, its serial (KH1 reuses elements), the KH2 motion it
/// started in, and its fade (alpha per game frame) once ending.
struct Run {
    element: usize,
    serial: u32,
    motion: u16,
    layer: Layer,
    fade: Option<f32>,
}

static RUNNING: Mutex<Vec<Run>> = Mutex::new(Vec::new());

/// A continuous binding of ours whose stored matrix is still read KH2's way (`posed`): KH1's
/// element and serial, the binding node and record, and Sora's skeleton passes when it started.
struct Fresh {
    element: usize,
    serial: u32,
    node: usize,
    record: usize,
    actor: usize,
    pass: u32,
    /// A position-only binding (KH1's mode 1): its anchor, the element's offset in the bone's
    /// frame, and the place it gave on the pass before.
    pos: Option<PosOnly>,
}

#[derive(Clone, Copy)]
struct PosOnly {
    anchor: Anchor,
    offset: [f32; 3],
    prev: [f32; 4],
}

static FRESH: Mutex<Vec<Fresh>> = Mutex::new(Vec::new());
static FRESH_ANY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Sora's skeleton passes (`posed`).
static PASSES: AtomicU32 = AtomicU32::new(0);
static LAST_MOTION: AtomicU32 = AtomicU32::new(u32::MAX);
/// KH1's handles cover 32 MB regions (`0x14038AD90`, 64 of them); our DPXs share one (`Arena`).
const REGION: usize = 0x200_0000;
/// KH1's removal of a loaded file (`0x1400E2E00 (entry, textures)`: its file-type handler if it
/// has one, its textures marked for release, the entry unlinked and destroyed, its HD entries
/// with it; the call `leftblade` releases its Keyblade's file with).
const DROP_FILE_RVA: Rva = Rva(0xE2DF0);
/// KH1's texture for a loaded file's texture data (Steam `0x14010C050 (textures, pixels, 0, 0,
/// width, height)`: the file holding the pixels, its HD entry by their offset, the texture made
/// from it once and kept in the entry) and its texture list (Steam `*0x1422C8FA8`), as KH1's DPD
/// texture set-up calls it (`0x14031EC10`).
const TEXTURE_OF_STEAM: usize = 0x10C050;
const TEXTURES_STEAM: usize = 0x22C8FA8;
/// KH1's effect instances (Steam `0x142678280`, 0x100 of 0xE0 bytes: `+0x98` the DPD it plays,
/// `+0x92` state, 1 live, 4 ending) and elements (Steam `0x142D678B0`, 0x100 of 0x3C: `+0x38` a
/// handle of its DPX), for whether a DPX is still in use.
const INSTANCES_STEAM: usize = 0x2678280;
const INSTANCES: usize = 0x100;
const INSTANCE_SIZE: usize = 0xE0;
const INST_DPD: usize = 0x98;
const ELEMENTS_STEAM: usize = 0x2D678B0;
const ELEMENTS: usize = 0x100;
const ELEMENT_SIZE: usize = 0x3C;
const EL_DPX: usize = 0x38;

/// Where an HD texture for one texture record comes from: the package's own HD image (held only
/// until the DPX is entered in KH1's loaded files), or one of KH2's common images (`Shared`: made
/// into a KH1 texture once, for every package).
enum HdSrc {
    Own(std::sync::Arc<Vec<u8>>),
    Common(i32),
}

/// An HD texture for one texture record: the record's pixel offset in the DPX, its image. Only
/// the first record of each TBP in each DPD has one: KH1 makes one texture per DPD and TBP, from
/// that record's HD entry (`0x14031EC10`: its records by TBP in a multimap, the lower bound's
/// looked up, `0x14010C050`, and its texture handed to every record of the TBP; Checked (exe)).
struct Hd {
    offset: u32,
    src: HdSrc,
}

/// Which package a native element is of: Sora's (`obj/P_EX100.a.us`), the Keyblade's
/// (`obj/W_EX010.a.us`), Valor Form's (`obj/P_EX100_BTLF.a.us`) or Final Form's
/// (`obj/P_EX100_ULTF.a.us`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pkg {
    Sora,
    Weapon,
    Valor,
    Final,
    /// One of `kh2fx`'s spell packages (its numbering): Reflect's levels 1..3, Wisdom's archive 4
    /// (`obj/P_EX100_MAGF.a.us`), then Fire, Blizzard, Thunder and Cure by level (Wisdom's and
    /// base Sora's KH2 magic, `formmagic`).
    Spell(u8),
}

struct Dpx {
    pkg: Pkg,
    /// What it was built for (Sora's: the moves `kh2_effects` lets play); when that changes, a new
    /// one is built and this one freed once nothing of it plays.
    sig: String,
    /// Its name in the log and (with `kh2fx_` / `_dpx`) in KH1's loaded-file list.
    label: String,
    /// Its elements switched on for KH1 (`None`: every one built).
    on: Option<&'static [i16]>,
    /// Its pages (KH1's effects point into them while it is entered; never moved).
    pages: Pages,
    at: usize,
    len: usize,
    md5: [u8; 16],
    /// Its HD images until it is entered (KH1 then holds its own copies).
    hd: Vec<Hd>,
    layers: Vec<Layer>,
    /// Each built element's program types (`kh1dpx::Kh1Dpx::types`), by element.
    types: Vec<(i16, Vec<u32>)>,
    /// Elements whose KH1 play has been refused (logged once each).
    refused: Vec<i16>,
    /// Where KH2's own programs are (`kh1dpx::Kh1Dpx::programs`), for `kh2ppp::plug`.
    programs: Vec<(usize, u32)>,
    /// KH1's loaded-file entry for it, once entered (0: not yet, or entering failed).
    entry: usize,
    relocated: bool,
}

/// The packages built (each its own DPX, entered in KH1's loaded files once built).
static PACKS: Mutex<Vec<Dpx>> = Mutex::new(Vec::new());
static STARTED: AtomicU32 = AtomicU32::new(0);

/// What a package is built from.
struct Source {
    pkg: Pkg,
    sig: String,
    label: String,
    /// Its file in KH2's install (BAR, or a PAX), and the name of its PAX entry if it has several;
    /// its bytes if the mod already holds them (a form's archive, a spell's file).
    path: String,
    pax_name: Option<&'static str>,
    bytes: Option<std::sync::Arc<Vec<u8>>>,
    on: Option<&'static [i16]>,
    /// The elements that play together (one DPD each).
    sets: Vec<Vec<i16>>,
    /// The other elements too, one DPD per effect (else only the sets' elements are built).
    rest: bool,
    /// KH1's anchor for one of its KH2 bone codes.
    anchor: fn(u16) -> Option<Anchor>,
    /// KH2 element flags it doesn't apply.
    ignore: u32,
}

// ---------------------------------------------------------------- memory

/// One 32 MB region of address space (one of KH1's 64 handle regions, `0x14038AD90`) reserved
/// once; every DPX of ours is committed inside it page by page and decommitted when freed. So all
/// our DPXs together take one handle region however often packages come and go, and only their
/// own pages of memory.
struct Arena {
    base: usize,
    /// Free spans (offset, length), page multiples, sorted.
    free: Vec<(usize, usize)>,
}

static ARENA: Mutex<Option<Arena>> = Mutex::new(None);
const PAGE: usize = 0x1000;
const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const MEM_DECOMMIT: u32 = 0x4000;
const MEM_RELEASE: u32 = 0x8000;
const PAGE_NOACCESS: u32 = 0x01;
const PAGE_READWRITE: u32 = 0x04;

/// A DPX's pages in the arena: exactly its size (rounded up to a page), zeroed.
struct Pages {
    at: usize,
    len: usize,
}

impl Pages {
    fn new(len: usize) -> Option<Pages> {
        use super::sys::{VirtualAlloc, VirtualFree};
        let len = (len.max(1) + PAGE - 1) & !(PAGE - 1);
        let mut g = ARENA.lock().ok()?;
        if g.is_none() {
            // A region-aligned 32 MB reservation: reserve twice that, find the boundary inside,
            // let go and take the aligned part (tried a few times, another thread may take it).
            for _ in 0..8 {
                let p = unsafe { VirtualAlloc(std::ptr::null_mut(), 2 * REGION, MEM_RESERVE, PAGE_NOACCESS) } as usize;
                if p == 0 {
                    return None;
                }
                let aligned = (p + REGION - 1) & !(REGION - 1);
                unsafe { VirtualFree(p as _, 0, MEM_RELEASE) };
                let q = unsafe { VirtualAlloc(aligned as _, REGION, MEM_RESERVE, PAGE_NOACCESS) } as usize;
                if q == aligned {
                    *g = Some(Arena { base: q, free: vec![(0, REGION)] });
                    break;
                }
                if q != 0 {
                    unsafe { VirtualFree(q as _, 0, MEM_RELEASE) };
                }
            }
        }
        let a = g.as_mut()?;
        let i = a.free.iter().position(|f| f.1 >= len)?;
        let (off, flen) = a.free[i];
        let at = a.base + off;
        if unsafe { VirtualAlloc(at as _, len, MEM_COMMIT, PAGE_READWRITE) } as usize != at {
            return None;
        }
        if flen == len {
            a.free.remove(i);
        } else {
            a.free[i] = (off + len, flen - len);
        }
        Some(Pages { at, len })
    }
}

impl Drop for Pages {
    fn drop(&mut self) {
        unsafe { super::sys::VirtualFree(self.at as _, self.len, MEM_DECOMMIT) };
        if let Ok(mut g) = ARENA.lock() {
            if let Some(a) = g.as_mut() {
                let off = self.at - a.base;
                let i = a.free.iter().position(|f| f.0 > off).unwrap_or(a.free.len());
                a.free.insert(i, (off, self.len));
                // Joined with its neighbours.
                if i + 1 < a.free.len() && a.free[i].0 + a.free[i].1 == a.free[i + 1].0 {
                    a.free[i].1 += a.free[i + 1].1;
                    a.free.remove(i + 1);
                }
                if i > 0 && a.free[i - 1].0 + a.free[i - 1].1 == a.free[i].0 {
                    a.free[i - 1].1 += a.free[i].1;
                    a.free.remove(i);
                }
            }
        }
    }
}

// ---------------------------------------------------------------- KH2's common textures, shared

/// One of KH2's common images (`effect/texcommon.dpd`, one HD image per TBP as KH2 has): its
/// first record's header (TBP, CBP, size; KH1 makes a texture from that record's place, with its
/// width and height), its HD image until KH1 has made it, and KH1's texture once made.
struct CommonImage {
    tbp: i32,
    header: [u8; 0x20],
    dds: Option<std::sync::Arc<Vec<u8>>>,
    texture: usize,
}

/// KH2's common textures for every package: `texcommon.dpd` (the builder's source), its images,
/// and (Steam) a small loaded file of ours, entered once and never removed, holding each image's
/// header: KH1 makes each image's texture once, from that file (`0x14010C050`, its own call), and
/// every package's records of that image are given it (their HD entry's texture set, which KH1's
/// texture lookup returns as it is, `0x14010BAF0`). A texture lives as long as the file it was
/// made from (`0x14010CDA0`: found by the file's MD5), so these never go.
struct Shared {
    dpd: std::sync::Arc<Vec<u8>>,
    images: Vec<CommonImage>,
    /// The file's pages and KH1's entry for it (0: not entered).
    file: Option<(Pages, usize)>,
}

static SHARED: Mutex<Option<Shared>> = Mutex::new(None);
/// An image's header in the shared file: its record header, then its "pixels" (KH1's lookup takes
/// the record's `+0x20`; the HD entry is keyed by that offset).
const SHARED_SLOT: usize = 0x30;

// ---------------------------------------------------------------- which packages, when

/// The packages wanted now (`manage`, game thread) and what each is built for; the builder makes
/// the ones missing.
static WANTED: Mutex<Vec<(Pkg, String)>> = Mutex::new(Vec::new());
static WANT_CHANGED: std::sync::Condvar = std::sync::Condvar::new();
/// Packages that couldn't be built (not tried again for the same want).
static FAILED: Mutex<Vec<(Pkg, String)>> = Mutex::new(Vec::new());
/// A package was built since `manage` last looked.
static NEW_BUILT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Sora's package is built for the moves `kh2_effects` lets play.
fn sora_sig() -> String {
    format!("{:?}", super::settings::get().kh2_effects)
}

/// The packages whose effects can play now (`docs/kh2-effects.md` §18.12): Sora's and the
/// Keyblade's always; a form's while Sora is changing into it or in it (Valor's, Final's,
/// Wisdom's archive); Reflect's level he has; and each of Fire, Blizzard, Thunder and Cure at the
/// level KH1 has for it, when KH2's magic casts it (base Sora's Magic styles at KH2, or Wisdom).
fn desired() -> Vec<(Pkg, String)> {
    let s = super::settings::get();
    if s.kh2_effects.is_none() {
        return Vec::new();
    }
    let mut v = vec![(Pkg::Sora, sora_sig()), (Pkg::Weapon, String::new())];
    let form = super::form::current().or_else(super::form::arriving);
    if s.drive {
        match form {
            Some(super::form::Kind::Valor) => v.push((Pkg::Valor, String::new())),
            Some(super::form::Kind::Final) => v.push((Pkg::Final, String::new())),
            Some(super::form::Kind::Wisdom) => v.push((Pkg::Spell(super::kh2fx::WISDOM), String::new())),
            None => {}
        }
    }
    // (The spell packages play through KH1 on Steam only.)
    if super::addr::Build::current() == super::addr::Build::Steam {
        if let Some(l) = unsafe { super::reflect::level_now() } {
            v.push((Pkg::Spell(l), String::new()));
        }
        use super::styles::{kh2, Kind};
        let base = kh2(Kind::Magic) || kh2(Kind::AirMagic);
        let wisdom = s.drive && form == Some(super::form::Kind::Wisdom);
        for spell in 0..4u8 {
            if base || (wisdom && spell <= 2) {
                let level = unsafe { super::reflect::kh1_spell_level(spell) };
                v.push((Pkg::Spell(super::formmagic::spell_package(spell, level)), String::new()));
            }
        }
    }
    v
}

/// Each game frame (`kh2fx::tick`): the packages that can play now are asked of the builder (in
/// the background), a package built is entered in KH1's loaded files at once (so a first play
/// costs nothing more than KH1's own), and one that can't play any more is freed once none of its
/// effects is left in KH1 (its file removed by KH1's own removal, `0x1400E2E00`, then its pages).
pub fn manage() {
    static FRAMES: AtomicU32 = AtomicU32::new(0);
    static LAST_FORM: AtomicU32 = AtomicU32::new(u32::MAX);
    let n = FRAMES.fetch_add(1, Ordering::Relaxed);
    let form = super::form::current().or_else(super::form::arriving).map_or(0, |k| k as u32 + 1);
    let form_changed = LAST_FORM.swap(form, Ordering::Relaxed) != form;
    let built = NEW_BUILT.swap(false, Ordering::Relaxed);
    if !(form_changed || built || n % 30 == 0) {
        return;
    }
    let want = desired();
    if let Ok(mut w) = WANTED.try_lock() {
        if *w != want {
            *w = want.clone();
            WANT_CHANGED.notify_all();
        }
    }
    let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    let Ok(mut g) = PACKS.try_lock() else { return };
    // Built and wanted: entered now. Not wanted (or built for something else): freed once idle.
    let mut i = 0;
    while i < g.len() {
        let wanted = want.iter().any(|(p, s)| *p == g[i].pkg && *s == g[i].sig);
        if wanted {
            if !g[i].relocated {
                unsafe { enter(base, &mut g[i]) };
            }
            i += 1;
            continue;
        }
        match unsafe { release(base, &g[i]) } {
            Ok(()) => {
                let d = g.remove(i);
                log(&format!("KH2 effects (native): {}'s DPX freed ({} bytes)", d.label, d.pages.len));
            }
            Err(_) => i += 1,
        }
    }
}

/// Builds the KH1 DPXs in the background (from the KH2 install, as `kh2fx`), each when it is
/// wanted (`manage`): Sora's and the Keyblade's once KH2 Sora is loaded, the others when their
/// effects can play.
pub fn start() {
    if super::settings::get().kh2_effects.is_none() {
        return;
    }
    if let Ok(mut w) = WANTED.lock() {
        *w = vec![(Pkg::Sora, sora_sig()), (Pkg::Weapon, String::new())];
    }
    std::thread::spawn(builder);
}

fn builder() {
    let t = std::time::Instant::now();
    let kh2 = loop {
        if let Some(k) = super::pose::kh2() {
            break k;
        }
        if t.elapsed().as_secs() > 300 {
            log("KH2 effects (native): KH2 Sora never loaded; kh2fx plays his effects");
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    };
    let opened = (|| -> Result<(kh2_data::Install, kh2_data::Source), String> {
        let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
        let install = kh2_data::Install::locate(&exe).map_err(|e| e.to_string())?;
        let src = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
        let common = install.read_any(src, "effect/texcommon.dpd").map_err(|e| format!("texcommon.dpd: {e}"))?;
        let assets = install.read_remastered_any(src, "effect/texcommon.dpd").map_err(|e| format!("texcommon HD textures: {e}"))?;
        let images = common_images(&common, &assets)?;
        log(&format!(
            "KH2 effects (native): KH2's common textures: {} images ({} bytes of HD images, made into KH1 textures once, as packages need them)",
            images.len(),
            images.iter().filter_map(|i| i.dds.as_ref().map(|d| d.len())).sum::<usize>()
        ));
        if let Ok(mut g) = SHARED.lock() {
            *g = Some(Shared { dpd: std::sync::Arc::new(common), images, file: None });
        }
        Ok((install, src))
    })();
    let (install, src) = match opened {
        Ok(c) => c,
        Err(e) => {
            log(&format!("KH2 effects (native): no DPX built ({e}); kh2fx plays the effects"));
            return;
        }
    };
    loop {
        let want = {
            let Ok(g) = WANTED.lock() else { return };
            let g = WANT_CHANGED.wait_timeout(g, std::time::Duration::from_secs(1)).map(|r| r.0).unwrap_or_else(|e| e.into_inner().0);
            g.clone()
        };
        for (pkg, sig) in want {
            let have = PACKS.lock().map_or(true, |g| g.iter().any(|d| d.pkg == pkg && d.sig == sig));
            let failed = FAILED.lock().map_or(true, |g| g.contains(&(pkg, sig.clone())));
            if have || failed {
                continue;
            }
            let t = std::time::Instant::now();
            let made = source(kh2, pkg, &sig).ok_or_else(|| "its files didn't load".to_string()).and_then(|s| build(&install, src, &s));
            match made {
                Ok(d) => {
                    let own: usize = d.hd.iter().filter_map(|h| if let HdSrc::Own(x) = &h.src { Some(x.len()) } else { None }).sum();
                    log(&format!(
                        "KH2 effects (native): {}'s DPX built, {} bytes, {} effects in {:.2} s ({} HD textures: {} own, {} bytes until entered; {} of the common images)",
                        d.label,
                        d.len,
                        d.layers.len(),
                        t.elapsed().as_secs_f32(),
                        d.hd.len(),
                        d.hd.iter().filter(|h| matches!(h.src, HdSrc::Own(_))).count(),
                        own,
                        d.hd.iter().filter(|h| matches!(h.src, HdSrc::Common(_))).count()
                    ));
                    if let Ok(mut g) = PACKS.lock() {
                        g.push(d);
                    }
                    NEW_BUILT.store(true, Ordering::Relaxed);
                }
                Err(e) => {
                    log(&format!("KH2 effects (native): {pkg:?}'s DPX not built ({e}); kh2fx plays its effects"));
                    if let Ok(mut g) = FAILED.lock() {
                        g.push((pkg, sig));
                    }
                }
            }
        }
    }
}

/// KH2's common images: each TBP of `texcommon.dpd`'s records, its first record's header, and
/// the HD asset for it (the one whose offset is that record's pixels, else any record's with that
/// TBP, as each package's own are found).
fn common_images(common: &[u8], assets: &[kh2_formats::egs::HdAsset]) -> Result<Vec<CommonImage>, String> {
    let at = if common.get(0..4) == Some(&0x96u32.to_le_bytes()[..]) { 0 } else { 16 };
    let recs = kh2_formats::kh1dpx::texture_records(&common[at..]).map_err(|e| e.to_string())?;
    let mut out: Vec<CommonImage> = Vec::new();
    for &(o, tbp) in &recs {
        if out.iter().any(|i| i.tbp == tbp) {
            continue;
        }
        let exact = at + o + 0x20;
        let asset = assets.iter().find(|a| a.offset as usize & 0xFF_FFFF == exact).or_else(|| assets.iter().find(|a| recs.iter().any(|&(r, t)| t == tbp && at + r + 0x20 == a.offset as usize & 0xFF_FFFF)));
        let mut header = [0u8; 0x20];
        header.copy_from_slice(common.get(at + o..at + o + 0x20).ok_or("texcommon record out of range")?);
        out.push(CommonImage { tbp, header, dds: asset.map(|a| std::sync::Arc::new(a.data.clone())), texture: 0 });
    }
    Ok(out)
}

/// What package `pkg` is built from (`sig`: Sora's, the moves it is for).
fn source(kh2: &'static super::pose::Kh2, pkg: Pkg, sig: &str) -> Option<Source> {
    let s = |pkg, label: &str, path: &str, pax_name, bytes, on, sets, rest, anchor, ignore| Source { pkg, sig: sig.to_string(), label: label.to_string(), path: path.to_string(), pax_name, bytes, on, sets, rest, anchor, ignore };
    Some(match pkg {
        // Sora's: only the elements of the motions that can play them (`kh2fx::event`: the moves
        // `kh2_effects` allows, the Drive motion), then Revert's.
        Pkg::Sora => {
            let mut sets: Vec<Vec<i16>> = motion_sets(&kh2.character).into_iter().filter(|(m, _)| super::kh2fx::allowed(*m) || super::form::own_motion(*m)).map(|x| x.1).collect();
            sets.push(super::form::REVERT_ELEMENTS.to_vec());
            s(pkg, "p_ex100", "obj/P_EX100.a.us", Some("p_ex"), None, Some(ON), sets, false, anchor, 0)
        }
        Pkg::Weapon => s(pkg, "w_ex010", "obj/W_EX010.a.us", None, None, Some(WEAPON_ON), Vec::new(), true, weapon_anchor, ENDS_WITH_MOTION),
        Pkg::Valor | Pkg::Final => {
            let (label, path, f) = if pkg == Pkg::Valor { ("p_ex100_btlf", "obj/P_EX100_BTLF.a.us", kh2.valor.as_ref()?) } else { ("p_ex100_ultf", "obj/P_EX100_ULTF.a.us", kh2.final_form.as_ref()?) };
            let sets = motion_sets(&f.character).into_iter().map(|x| x.1).collect();
            s(pkg, label, path, Some("p_ex"), f.archive.clone(), None, sets, true, form_anchor, 0)
        }
        // The spell packages `kh2fx` plays for the spell scripts (`reflect`, `formmagic`): their
        // effects don't end with Sora's motion (the scripts end them).
        Pkg::Spell(level) => {
            let (path, pax_name, bytes) = if level == super::kh2fx::WISDOM {
                ("obj/P_EX100_MAGF.a.us".to_string(), Some("p_ex"), kh2.wisdom.as_ref()?.archive.clone())
            } else if let Some((sp, l)) = super::form::package_spell(level) {
                (super::formmagic::spell_path(sp, l), None, super::formmagic::spell_file(sp, l))
            } else if (1..=3).contains(&level) {
                (super::reflect::spell_path(level), None, super::reflect::spell_file(level))
            } else {
                return None;
            };
            let label = path.rsplit('/').next().unwrap_or(&path).replace('.', "_").to_lowercase();
            s(pkg, &label, &path, pax_name, bytes, None, Vec::new(), true, spell_anchor, ENDS_WITH_MOTION)
        }
    })
}

/// The elements each motion of a moveset starts (its frame triggers 1), in motion order.
fn motion_sets(c: &character_core::Character) -> Vec<(u16, Vec<i16>)> {
    let mut motions: Vec<(&u16, &character_core::Clip)> = c.moveset.clips.iter().collect();
    motions.sort_by_key(|m| *m.0);
    motions.iter().map(|(m, c)| (**m, c.events.iter().filter_map(|e| if let character_core::EventKind::Effect(el) = e.kind { Some(el) } else { None }).collect())).collect()
}

/// A native element's layer and its copy of its effect: the element's offset and rotation
/// folded into the header's local transform (`H' = S⁻¹ · T(t) · R · S · H`, S the element's
/// scale, which KH1 takes as the instance's: KH2's root is `T(B·t) · R(B·R) · S`, KH1's the
/// bound bone's `B`, times the header).
fn layer(pax: &[u8], parsed: &kh2_formats::pax::Pax, el: i16, anchor_of: fn(u16) -> Option<Anchor>, ignore: u32) -> Result<(Layer, kh2_formats::kh1dpx::Variant, String), String> {
    use glam::{Mat3, Vec3};
    use kh2_fx::math;
    let e = parsed.element(el as u16).ok_or(format!("no element {el}"))?;
    let effect = e.effect as u32;
    let flags = e.flags & !ignore;
    if !(0..0x100).contains(&el) {
        return Err(format!("element {el}: no effect number for it (0x700 | element)"));
    }
    if let Some(why) = unbindable(flags) {
        return Err(format!("element {el}: {why} (flags {:#x})", e.flags));
    }
    let bone = anchor_of(e.bone).ok_or(format!("element {el}: no KH1 anchor for KH2 bone {:#x}", e.bone))?;
    let (h_off, h_ang, _) = kh2_formats::kh1dpx::header_local(pax, effect).map_err(|e| e.to_string())?;
    let s = Vec3::from(e.scale);
    let safe = Vec3::new(if s.x != 0.0 { s.x } else { 1.0 }, if s.y != 0.0 { s.y } else { 1.0 }, if s.z != 0.0 { s.z } else { 1.0 });
    let r_e = math::euler_xyz(Vec3::from(e.rotation));
    let r_h = math::euler_xyz_i(h_ang);
    let position_only = flags & POSITION_ONLY != 0;
    if position_only && matches!(bone, Anchor::Code(_)) {
        return Err(format!("element {el}: bound by position only to KH2's {:#x}, which KH1's position binding has no point for yet", e.bone));
    }
    // Position only: the root is `T(B · t) · R · S · H` (KH2), KH1's `T(point + offset) · S · H'`
    // with the instance unturned, so `H' = S⁻¹ · R · S · H` and the offset goes through the
    // binding (`posed`).
    let offset = if position_only { r_e * Vec3::from(h_off) } else { Vec3::from(e.translation) / safe + r_e * Vec3::from(h_off) };
    let rot: Mat3 = r_e * r_h;
    let angle = math::xyz_i_of(rot);
    let number = ELEMENT_NUMBER | el as u32;
    let note = format!(
        "element {el}: effect {effect} as {number:#x} on KH1 {bone:?} (KH2 bone {:#x}), flags {:#x}, wait {}, fade-out {}, offset {:?} turned {:?} (header {:?} / {:?} -> {:?} / {:?}), scale {:?}{}",
        e.bone, e.flags, e.start_wait, e.fadeout_frames, e.translation, e.rotation, h_off, h_ang, offset.to_array(), angle, e.scale,
        if s.x != s.y || s.y != s.z { " (not uniform: the offset is exact, the turn approximate)" } else { "" }
    );
    let l = Layer { el, number, anchor: bone, flags, bone_offset: if position_only { e.translation } else { [0.0; 3] }, wait: e.start_wait, fadeout: e.fadeout_frames as f32, scale: e.scale };
    Ok((l, kh2_formats::kh1dpx::Variant { number, effect, offset: offset.to_array(), angle }, note))
}

/// KH2 element flags KH1's binding doesn't do yet (`docs/kh2-effects.md` §2, §18.3).
fn unbindable(flags: u32) -> Option<&'static str> {
    if flags & (BIND | BIND_UNTIL_STARTED) == 0 && flags & POSITION_ONLY != 0 {
        return Some("placed once at its start by position only");
    }
    if flags & BONE_SCALE != 0 {
        return Some("scaled by its bone");
    }
    if flags & TWO_POINT != 0 {
        return Some("a two-point element");
    }
    None
}

/// One package's KH1 DPX: its elements (the first with each id, as KH2's start takes) each its
/// own copy of its effect, grouped (`Source::sets`, then, with `rest`, one group per effect), in
/// its own pages, with the HD images its DPDs' textures need: its own (held until entered) and
/// the common ones by TBP (`Shared`).
fn build(install: &kh2_data::Install, src: kh2_data::Source, s: &Source) -> Result<Dpx, String> {
    let read;
    let pack: &[u8] = match &s.bytes {
        Some(b) => b,
        None => {
            read = install.read_any(src, &s.path).map_err(|e| e.to_string())?;
            &read
        }
    };
    let common = SHARED.lock().ok().and_then(|g| g.as_ref().map(|c| c.dpd.clone())).ok_or("no common textures")?;
    let (pax_data, pax_at): (&[u8], usize) = if pack.starts_with(b"PAX_") {
        (pack, 0)
    } else {
        let bar = kh2_formats::bar::Bar::parse(pack).map_err(|e| e.to_string())?;
        let pax = bar
            .entries
            .iter()
            .find(|e| s.pax_name.is_some_and(|n| e.name == n) && e.data.starts_with(b"PAX_"))
            .or_else(|| bar.entries.iter().find(|e| e.data.starts_with(b"PAX_")))
            .ok_or(format!("no PAX in {}", s.path))?;
        (pax.data, pax.data.as_ptr() as usize - pack.as_ptr() as usize)
    };
    let parsed = kh2_formats::pax::Pax::parse(pax_data).map_err(|e| e.to_string())?;
    // The elements built: the sets' (and with `rest` every one), the first with each id.
    let wanted = |el: i16| s.rest || s.sets.iter().any(|set| set.contains(&el));
    let mut layers: Vec<Layer> = Vec::new();
    let mut variants: Vec<kh2_formats::kh1dpx::Variant> = Vec::new();
    let mut left = Vec::new();
    for e in &parsed.elements {
        let el = e.id as i16;
        if !wanted(el) || layers.iter().any(|l| l.el == el) || left.iter().any(|x: &String| x.starts_with(&format!("element {el}:"))) {
            continue;
        }
        match layer(pax_data, &parsed, el, s.anchor, s.ignore) {
            Ok((l, v, note)) => {
                if s.on.is_none_or(|on| on.contains(&el)) {
                    log(&format!("KH2 effects (native): {}: {note}", s.label));
                }
                layers.push(l);
                variants.push(v);
            }
            Err(why) => left.push(why),
        }
    }
    if !left.is_empty() {
        log(&format!("KH2 effects (native): {}: not built (kh2fx plays them): {}", s.label, left.join("; ")));
    }
    if layers.is_empty() {
        return Err("no element of it can be KH1's".into());
    }
    // Groups: the source's sets (an element in the first that holds it), then (with `rest`) each
    // other effect's elements; each group split by the DPD its effects are in.
    let dpd_of = |v: &kh2_formats::kh1dpx::Variant| kh2_formats::dpd::effect_source(pax_data, v.effect).ok().flatten().map(|(d, _)| d.as_ptr() as usize);
    let mut groups: Vec<Vec<kh2_formats::kh1dpx::Variant>> = Vec::new();
    let mut placed: Vec<u32> = Vec::new();
    let mut add = |els: &[i16], groups: &mut Vec<Vec<kh2_formats::kh1dpx::Variant>>| {
        let mut by_dpd: Vec<(Option<usize>, Vec<kh2_formats::kh1dpx::Variant>)> = Vec::new();
        for &el in els {
            let Some(i) = layers.iter().position(|l| l.el == el) else { continue };
            let v = variants[i];
            if placed.contains(&v.number) {
                continue;
            }
            placed.push(v.number);
            let d = dpd_of(&v);
            match by_dpd.iter_mut().find(|x| x.0 == d) {
                Some(x) => x.1.push(v),
                None => by_dpd.push((d, vec![v])),
            }
        }
        groups.extend(by_dpd.into_iter().map(|x| x.1));
    };
    for set in &s.sets {
        add(set, &mut groups);
    }
    if s.rest {
        let mut effects: Vec<u32> = variants.iter().map(|v| v.effect).collect();
        effects.dedup();
        for e in effects {
            let els: Vec<i16> = layers.iter().zip(&variants).filter(|(_, v)| v.effect == e).map(|(l, _)| l.el).collect();
            add(&els, &mut groups);
        }
    }
    let r = kh2_formats::kh1dpx::build_groups(pax_data, &common, &groups).map_err(|e| e.to_string())?;
    log(&format!("KH2 effects (native): {}'s DPX: {}", s.label, r.report));
    // Elements of groups left out (too many texture slots, a texture missing) aren't KH1's.
    for (gi, why) in &r.skipped {
        let els: Vec<i16> = groups[*gi].iter().filter_map(|v| layers.iter().find(|l| l.number == v.number).map(|l| l.el)).collect();
        log(&format!("KH2 effects (native): {}: elements {els:?} not built ({why}); kh2fx plays them", s.label));
    }
    layers.retain(|l| r.effects.contains(&l.number));
    // The HD images: one per DPD and TBP (the first record of the TBP in the DPD, the one KH1
    // looks up). Own ones from the package file's remastered assets (each by the offset of its
    // record's pixels in the file, else a record's of the same TBP), each kept once; common ones
    // by their source TBP.
    let own_hd = install.read_remastered_any(src, &s.path).map_err(|e| format!("{} HD textures: {e}", s.path))?;
    let dpds: Vec<usize> = {
        let b = &r.bytes;
        let n = u32::from_le_bytes(b[12..16].try_into().unwrap()) as usize;
        let list = 16 + n * 32;
        let nd = u32::from_le_bytes(b[list..list + 4].try_into().unwrap()) as usize;
        (0..nd).map(|i| u32::from_le_bytes(b[list + 4 + i * 4..list + 8 + i * 4].try_into().unwrap()) as usize).collect()
    };
    let common_tbps: Vec<i32> = SHARED.lock().ok().and_then(|g| g.as_ref().map(|c| c.images.iter().filter(|i| i.dds.is_some() || i.texture != 0).map(|i| i.tbp).collect())).unwrap_or_default();
    let mut hd = Vec::new();
    let mut report = Vec::new();
    let mut seen: Vec<(usize, u16)> = Vec::new();
    let mut kept: Vec<(String, std::sync::Arc<Vec<u8>>)> = Vec::new();
    let mut recs_of: Vec<(usize, Vec<(usize, i32)>)> = Vec::new();
    for t in &r.textures {
        let dpd = dpds.iter().rposition(|&p| p <= t.dpx_offset).unwrap_or(0);
        let tbp = u16::from_le_bytes([r.bytes[t.dpx_offset], r.bytes[t.dpx_offset + 1]]);
        if seen.contains(&(dpd, tbp)) {
            continue;
        }
        seen.push((dpd, tbp));
        let offset = (t.dpx_offset + 0x20) as u32;
        if !t.own {
            if common_tbps.contains(&t.src_tbp) {
                report.push(format!("DPD {dpd} common tbp {:#x} <- shared", t.src_tbp));
                hd.push(Hd { offset, src: HdSrc::Common(t.src_tbp) });
            } else {
                report.push(format!("DPD {dpd} common tbp {:#x} <- none", t.src_tbp));
            }
            continue;
        }
        if !recs_of.iter().any(|x| x.0 == t.src_dpd_at) {
            let recs = kh2_formats::kh1dpx::texture_records(&pax_data[t.src_dpd_at..]).map_err(|e| e.to_string())?;
            recs_of.push((t.src_dpd_at, recs));
        }
        let recs = &recs_of.iter().find(|x| x.0 == t.src_dpd_at).unwrap().1;
        let base = pax_at + t.src_dpd_at;
        let exact = base + t.src_offset + 0x20;
        let found = own_hd.iter().find(|a| a.offset as usize & 0xFF_FFFF == exact).or_else(|| own_hd.iter().find(|a| recs.iter().any(|&(o, tbp)| tbp == t.src_tbp && base + o + 0x20 == a.offset as usize & 0xFF_FFFF)));
        match found {
            Some(a) => {
                let data = match kept.iter().find(|k| k.0 == a.name) {
                    Some(k) => k.1.clone(),
                    None => {
                        let d = std::sync::Arc::new(a.data.clone());
                        kept.push((a.name.clone(), d.clone()));
                        d
                    }
                };
                report.push(format!("DPD {dpd} own {exact:#x} (tbp {:#x}) <- {} ({} bytes)", t.src_tbp, a.name, a.data.len()));
                hd.push(Hd { offset, src: HdSrc::Own(data) });
            }
            None => report.push(format!("DPD {dpd} own {exact:#x} (tbp {:#x}) <- none", t.src_tbp)),
        }
    }
    log(&format!("KH2 effects (native): {}'s HD textures: {}", s.label, report.join("; ")));
    // The DPX in its own pages (inside our one handle region).
    let pages = Pages::new(r.bytes.len()).ok_or("no memory for it")?;
    let at = pages.at;
    unsafe { std::ptr::copy_nonoverlapping(r.bytes.as_ptr(), at as *mut u8, r.bytes.len()) };
    let types = layers.iter().filter_map(|l| r.effects.iter().position(|&n| n == l.number).map(|i| (l.el, r.types[i].clone()))).collect();
    Ok(Dpx { pkg: s.pkg, sig: s.sig.clone(), label: s.label.clone(), on: s.on, pages, at, len: r.bytes.len(), md5: r.md5, hd, layers, types, refused: Vec::new(), programs: r.programs.clone(), entry: 0, relocated: false })
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// What entering a file in KH1's loaded-file list takes: the list, KH1's own vtables (a resource
/// entry's, its HD-entry list's, and the commonest HD entry's), and the game's CRT allocators
/// (KH1 frees what it is given with them: `delete`, `_aligned_free`).
struct Kh1Files {
    list: usize,
    res_vt: usize,
    list_vt: usize,
    entry_vt: usize,
    malloc: unsafe extern "C" fn(usize) -> usize,
    aligned: unsafe extern "C" fn(usize, usize) -> usize,
}

unsafe fn kh1_files(base: usize) -> Result<Kh1Files, String> {
    use super::sys::{GetModuleHandleA, GetProcAddress};
    let f = base + FILE_OF_RVA;
    if rd::<[u8; 3]>(f) != [0x4C, 0x8B, 0x1D] {
        return Err("unexpected code at the file lookup".into());
    }
    let list_at = (f as isize + 7 + rd::<i32>(f + 3) as isize) as usize;
    let list = rd::<usize>(list_at);
    if list == 0 {
        return Err("no loaded-file list".into());
    }
    let first = rd::<usize>(list + 8);
    if first == 0 {
        return Err("the loaded-file list is empty".into());
    }
    let (res_vt, list_vt) = (rd::<usize>(first), rd::<usize>(first + 0x668));
    let mut counts: Vec<(usize, u32)> = Vec::new();
    let mut r = first;
    for _ in 0..4096 {
        if r == 0 {
            break;
        }
        let mut e = rd::<usize>(r + 0x670);
        for _ in 0..4096 {
            if e == 0 {
                break;
            }
            let vt = rd::<usize>(e);
            match counts.iter_mut().find(|c| c.0 == vt) {
                Some(c) => c.1 += 1,
                None => counts.push((vt, 1)),
            }
            e = rd::<usize>(e + 8);
        }
        r = rd::<usize>(r + 8);
    }
    let Some(&(entry_vt, _)) = counts.iter().max_by_key(|c| c.1) else { return Err("no HD entry in KH1's files to copy".into()) };
    let crt = GetModuleHandleA(b"ucrtbase.dll\0".as_ptr());
    let malloc = GetProcAddress(crt, b"malloc\0".as_ptr());
    let aligned = GetProcAddress(crt, b"_aligned_malloc\0".as_ptr());
    if crt.is_null() || malloc.is_null() || aligned.is_null() {
        return Err("the game's CRT not found".into());
    }
    Ok(Kh1Files { list, res_vt, list_vt, entry_vt, malloc: std::mem::transmute(malloc), aligned: std::mem::transmute(aligned) })
}

/// A resource entry for our bytes `at..at + len` named `name`, appended to KH1's loaded-file list
/// as `0x1400E2F60` does (0x688 bytes, as `0x1400E1680` makes them), without HD entries yet.
unsafe fn new_file(k: &Kh1Files, name: &str, at: usize, len: usize, md5: [u8; 16]) -> Result<usize, String> {
    let rec = (k.malloc)(0x688);
    if rec == 0 {
        return Err("out of memory".into());
    }
    std::ptr::write_bytes(rec as *mut u8, 0, 0x688);
    wr(rec, k.res_vt);
    wr(rec + 0x10, at);
    wr(rec + 0x18, len as u32);
    wr(rec + 0x1C, u32::MAX);
    for (i, &b) in name.as_bytes().iter().take(0x40).enumerate() {
        wr(rec + 0x20 + i, b);
        wr(rec + 0x1C4 + i, b);
        wr(rec + 0x620 + i, b);
    }
    wr(rec + 0x208, md5);
    // (-1: KH1's teardown writes no record of it, `0x1400E1EB0`.)
    wr(rec + 0x218, u32::MAX);
    wr(rec + 0x640, len as u32);
    wr(rec + 0x644, rd::<u32>(k.list + 0x20));
    // (-1: no file-type handler is called when it is removed, `0x1400E2E00`.)
    wr(rec + 0x64C, u32::MAX);
    wr(rec + 0x658, at);
    wr(rec + 0x668, k.list_vt);
    if rd::<usize>(k.list + 8) == 0 {
        wr(k.list + 8, rec);
    }
    let last = rd::<usize>(k.list + 0x10);
    if last != 0 {
        wr(last + 8, rec);
    }
    wr(k.list + 0x10, rec);
    wr(k.list + 0x18, rd::<i32>(k.list + 0x18) + 1);
    Ok(rec)
}

/// An HD entry (0x78 bytes, as `0x1400E1680` makes them) for the texture data at `offset` in
/// file `rec`: with a copy of `dds` (KH1 makes the texture from it on first use and owns it), or
/// with KH1's `texture` already made (KH1's lookup then hands that over, `0x14010BAF0`).
unsafe fn add_hd(k: &Kh1Files, rec: usize, offset: u32, dds: Option<&[u8]>, texture: usize) -> Result<usize, String> {
    let e = (k.malloc)(0x78);
    if e == 0 {
        return Err("out of memory".into());
    }
    std::ptr::write_bytes(e as *mut u8, 0, 0x78);
    if let Some(dds) = dds {
        let data = (k.aligned)(dds.len(), 16);
        if data == 0 {
            return Err("out of memory".into());
        }
        std::ptr::copy_nonoverlapping(dds.as_ptr(), data as *mut u8, dds.len());
        wr(e + 0x50, dds.len() as u32);
        wr(e + 0x60, data);
    }
    wr(e, k.entry_vt);
    let name = format!("-{offset:x}.dds\0");
    for (i, &b) in name.as_bytes().iter().take(0x3F).enumerate() {
        wr(e + 0x10 + i, b);
    }
    wr(e + 0x54, offset & 0xFF_FFFF);
    wr(e + 0x68, 999u32);
    wr(e + 0x70, texture);
    let n = rd::<i32>(rec + 0x680);
    if (0..0x100).contains(&n) {
        wr(rec + 0x21C + n as usize * 4, offset);
    }
    let tail = rd::<usize>(rec + 0x678);
    if tail == 0 {
        wr(rec + 0x670, e);
    } else {
        wr(tail + 8, e);
    }
    wr(rec + 0x678, e);
    wr(rec + 0x680, n + 1);
    Ok(e)
}

/// KH1's texture of the common image of source TBP `tbp` (Steam): made once, from our shared file
/// (entered the first time), by KH1's own texture lookup (`0x14010C050 (textures, pixels, 0, 0,
/// width, height)`, the call KH1's DPD texture set-up makes, `0x14031EC10`). 0 if it can't be.
unsafe fn shared_texture(base: usize, k: &Kh1Files, tbp: i32) -> usize {
    if super::addr::Build::current() != super::addr::Build::Steam {
        return 0;
    }
    let Ok(mut g) = SHARED.lock() else { return 0 };
    let Some(sh) = g.as_mut() else { return 0 };
    let Some(i) = sh.images.iter().position(|im| im.tbp == tbp) else { return 0 };
    if sh.images[i].texture != 0 {
        return sh.images[i].texture;
    }
    if sh.file.is_none() {
        let len = sh.images.len() * SHARED_SLOT;
        let Some(pages) = Pages::new(len) else { return 0 };
        for (j, im) in sh.images.iter().enumerate() {
            std::ptr::copy_nonoverlapping(im.header.as_ptr(), (pages.at + j * SHARED_SLOT) as *mut u8, 0x20);
        }
        let bytes = std::slice::from_raw_parts(pages.at as *const u8, len);
        match new_file(k, "kh2fx_texcommon_dpx", pages.at, len, kh2_formats::kh1dpx::md5(bytes)) {
            Ok(rec) => {
                log(&format!("KH2 effects (native): KH2's common textures entered in KH1's loaded files ({rec:#x}, {} images)", sh.images.len()));
                sh.file = Some((pages, rec));
            }
            Err(e) => {
                log(&format!("KH2 effects (native): KH2's common textures not entered ({e}); each package gets its own"));
                return 0;
            }
        }
    }
    let Some((pages, rec)) = sh.file.as_ref() else { return 0 };
    let (at, rec) = (pages.at, *rec);
    let Some(dds) = sh.images[i].dds.clone() else { return 0 };
    let key = (i * SHARED_SLOT + 0x20) as u32;
    if add_hd(k, rec, key, Some(&dds), 0).is_err() {
        return 0;
    }
    let h = &sh.images[i].header;
    let (w, ht) = (i16::from_le_bytes([h[0xC], h[0xD]]) as i32, i16::from_le_bytes([h[0xE], h[0xF]]) as i32);
    let make: unsafe extern "system" fn(usize, usize, usize, usize, i32, i32) -> usize = std::mem::transmute(base + TEXTURE_OF_STEAM);
    let tex = make(rd::<usize>(base + TEXTURES_STEAM), at + key as usize, 0, 0, w, ht);
    log(&format!("KH2 effects (native): common image tbp {tbp:#x} made into KH1 texture {tex:#x} ({} bytes)", dds.len()));
    if tex != 0 {
        sh.images[i].texture = tex;
        // (KH1 holds the image now.)
        sh.images[i].dds = None;
    }
    tex
}

/// Enters the DPX in KH1's loaded-file list with its HD entries (see the module notes), then has
/// KH1 relocate it and plugs KH2's own programs in. Its own images are copied to KH1 (which makes
/// a texture of each on its first draw, as for its own effects) and let go here; its common ones
/// are given KH1's shared textures (or, where none can be made, a copy of their image).
unsafe fn enter(base: usize, d: &mut Dpx) {
    let entered = (|| -> Result<(usize, usize, usize, usize), String> {
        let k = kh1_files(base)?;
        let rec = new_file(&k, &format!("kh2fx_{}_dpx", d.label), d.at, d.len, d.md5)?;
        let (mut copied, mut bytes, mut shared) = (0, 0, 0);
        for h in &d.hd {
            match &h.src {
                HdSrc::Own(dds) => {
                    add_hd(&k, rec, h.offset, Some(dds), 0)?;
                    copied += 1;
                    bytes += dds.len();
                }
                HdSrc::Common(tbp) => {
                    let tex = shared_texture(base, &k, *tbp);
                    if tex != 0 {
                        add_hd(&k, rec, h.offset, None, tex)?;
                        shared += 1;
                    } else {
                        let dds = SHARED.lock().ok().and_then(|g| g.as_ref().and_then(|s| s.images.iter().find(|i| i.tbp == *tbp).and_then(|i| i.dds.clone())));
                        if let Some(dds) = dds {
                            add_hd(&k, rec, h.offset, Some(&dds), 0)?;
                            copied += 1;
                            bytes += dds.len();
                        }
                    }
                }
            }
        }
        Ok((rec, copied, bytes, shared))
    })();
    match entered {
        Ok((rec, copied, bytes, shared)) => {
            d.entry = rec;
            log(&format!("KH2 effects (native): {}'s DPX entered in KH1's loaded files ({rec:#x}): {copied} HD images copied ({bytes} bytes), {shared} common textures shared", d.label));
        }
        Err(e) => log(&format!("KH2 effects (native): {}'s DPX not entered in KH1's loaded files ({e}); its textures won't draw", d.label)),
    }
    d.hd = Vec::new();
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(base + RELOCATE_RVA);
    f(d.at);
    d.relocated = true;
    log(&format!("KH2 effects (native): {}'s DPX at {:#x} relocated by KH1", d.label, d.at));
    // KH2's own programs, as entries of ours in KH1's format (`kh2ppp`).
    super::kh2ppp::plug(d.at, &d.programs);
}

/// Lets a package go (Steam; elsewhere only one not yet entered): Err while KH1 still has one of
/// its effects (an effect instance on one of its DPDs, live or ending, or an element of KH1's on
/// it). Its HD entries that hold a shared texture let go of it first (so KH1's removal doesn't
/// mark it), then KH1's own removal of the file (`0x1400E2E00 (entry, 1)`: its HD entries freed,
/// its own textures released by KH1 after their delay).
unsafe fn release(base: usize, d: &Dpx) -> Result<(), ()> {
    if !d.relocated {
        return Ok(());
    }
    if super::addr::Build::current() != super::addr::Build::Steam {
        return Err(());
    }
    let inside = |p: usize| p >= d.at && p < d.at + d.len;
    for i in 0..INSTANCES {
        let inst = base + INSTANCES_STEAM + i * INSTANCE_SIZE;
        if rd::<u16>(inst + INST_STATE) & 5 != 0 && inside(rd::<usize>(inst + INST_DPD)) {
            return Err(());
        }
    }
    for i in 0..ELEMENTS {
        let el = base + ELEMENTS_STEAM + i * ELEMENT_SIZE;
        if rd::<u32>(el + EL_FLAGS) & 3 != 0 && inside(super::pose::ptr(rd::<u32>(el + EL_DPX))) {
            return Err(());
        }
    }
    if d.entry != 0 {
        let shared: Vec<usize> = SHARED.lock().ok().and_then(|g| g.as_ref().map(|s| s.images.iter().map(|i| i.texture).filter(|&t| t != 0).collect())).unwrap_or_default();
        let mut e = rd::<usize>(d.entry + 0x670);
        for _ in 0..4096 {
            if e == 0 {
                break;
            }
            if shared.contains(&rd::<usize>(e + 0x70)) {
                wr::<usize>(e + 0x70, 0);
            }
            e = rd::<usize>(e + 8);
        }
        let drop_file: unsafe extern "system" fn(usize, u8) = std::mem::transmute(base + DROP_FILE_RVA);
        drop_file(d.entry, 1);
    }
    Ok(())
}

/// A KH2 element of Sora's package (`kh2fx::event`, game thread): true if it is KH1's (`kh2fx`
/// doesn't play it). On Steam, once the DPX is built and the element is switched on (`ON`), with
/// a KH1 anchor and every program its effect runs one KH1 runs as KH2 (`missing`), it is KH1's
/// alone: KH1 plays it, or (no free element or effect slot, no Sora) it doesn't play, as KH1's own
/// effects. `kh2fx` plays it only where KH1 can't: the DPX not built (yet, or it failed), a
/// program not in KH1 yet (or KH2's not plugged in: KH1's program table not as expected), or
/// another build (Epic: no blade binding, no KH2 programs; there elements without those still
/// play through KH1 and fall back to `kh2fx` if KH1 has no slot).
pub fn event(motion: u16, el: i16) -> bool {
    play(Pkg::Sora, motion, el)
}

/// Element `el` of package `pkg` started by KH1 (see `event`): true if it is KH1's.
fn play(pkg: Pkg, motion: u16, el: i16) -> bool {
    !matches!(start_el(pkg, motion, el), Started::NotMine)
}

/// What `start_el` did.
enum Started {
    /// KH1 plays it: its element and layer.
    Kh1(usize, Layer),
    /// KH1's, but KH1 had no free element or effect slot, or no Sora: it doesn't play.
    NotStarted,
    /// Not KH1's (`kh2fx` plays it).
    NotMine,
}

fn start_el(pkg: Pkg, motion: u16, el: i16) -> Started {
    let Ok(mut g) = PACKS.lock() else { return Started::NotMine };
    // (Sora's built for the moves allowed now, else the one before while it is rebuilt.)
    let sig = if pkg == Pkg::Sora { sora_sig() } else { String::new() };
    let i = g.iter().position(|d| d.pkg == pkg && d.sig == sig).or_else(|| g.iter().rposition(|d| d.pkg == pkg));
    let Some(d) = i.map(|i| &mut g[i]) else { return Started::NotMine };
    if !d.on.is_none_or(|on| on.contains(&el)) {
        return Started::NotMine;
    }
    let Some(&layer) = d.layers.iter().find(|l| l.el == el) else { return Started::NotMine };
    let (number, bone) = (layer.number, layer.anchor);
    let steam = super::addr::Build::current() == super::addr::Build::Steam;
    // KH1's blade and position bindings, and the binding list (unbound elements), are used on
    // Steam only (their addresses are Steam's): elsewhere kh2fx plays them.
    let position_only = layer.flags & POSITION_ONLY != 0;
    let unbound = layer.flags & (BIND | BIND_UNTIL_STARTED) == 0;
    if (matches!(bone, Anchor::Blade(_)) || position_only || unbound) && !steam {
        return Started::NotMine;
    }
    let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    // (Entered by `manage` as soon as it is built; here if it was built since.)
    if !d.relocated {
        unsafe { enter(base, d) };
    }
    // An effect with a program KH1 doesn't run as KH2 plays through kh2fx (KH1's own version, or
    // its dummy, isn't KH2's).
    let lacks = d.types.iter().find(|t| t.0 == el).map_or_else(Vec::new, |t| missing(&t.1));
    if !lacks.is_empty() {
        if !d.refused.contains(&el) {
            d.refused.push(el);
            log(&format!("KH2 effects (native): {} element {el} runs programs {lacks:?}, not in KH1 as KH2's yet; kh2fx plays it", d.label));
        }
        return Started::NotMine;
    }
    // (Steam: KH1's alone from here on, the old player retired for it.)
    let owned = steam;
    let sora = super::takeover::sora_actor();
    if sora < 0x10000 {
        return if owned { Started::NotStarted } else { Started::NotMine };
    }
    let once = if layer.flags & BIND_UNTIL_STARTED != 0 { BIND_ONCE } else { 0 };
    let on_bone = |code: u32| {
        let f: unsafe extern "system" fn(usize, u32, usize, u32) -> usize = unsafe { std::mem::transmute(base + START_ON_BONE_RVA) };
        unsafe { f(d.at, number, sora, code | once) }
    };
    let by_position = |code: u32, t: f32| {
        let f: unsafe extern "system" fn(usize, u32, usize, u32, f32) -> usize = unsafe { std::mem::transmute(base + START_BY_POSITION_STEAM) };
        unsafe { f(d.at, number, sora, code | once, t) }
    };
    let element = match bone {
        Anchor::Bone(b) if position_only => by_position(RAW_POINT | b, 1.0),
        Anchor::Blade(t) if position_only => by_position(BLADE_BONE, t),
        Anchor::Bone(b) => on_bone(RAW_BONE | b),
        Anchor::Code(c) => on_bone(c),
        Anchor::Blade(t) => {
            let f: unsafe extern "system" fn(usize, u32, usize, u32, f32) -> usize = unsafe { std::mem::transmute(base + START_ON_BLADE_STEAM) };
            unsafe { f(d.at, number, sora, BLADE_BONE | once, t) }
        }
    };
    drop(g);
    // Not bound (KH2 places it once, at its start, from the bone as it is then): KH1's start has
    // just placed it the same way; its binding is disarmed so its first run lets go.
    if element != 0 && unbound {
        unsafe { disarm(base, element) };
    } else if element != 0 && steam && (once == 0 || position_only) {
        let pos = position_only.then_some(PosOnly { anchor: bone, offset: layer.bone_offset, prev: [0.0; 4] });
        unsafe { note_binding(base, element, sora, pos) };
    }
    let set = element != 0 && unsafe { apply(element, &layer, motion) };
    if STARTED.fetch_add(1, Ordering::Relaxed) < 20 {
        log(&format!(
            "KH2 effect (native): {pkg:?} element {el} of motion {motion} -> KH1 effect {number:#x} on {bone:?}: {}",
            if element == 0 { "not started (no free element or effect slot)".to_string() } else { format!("element {element:#x}{}", if set { "" } else { " (its instance not found: no wait, scale or ending)" }) }
        ));
    }
    if element != 0 {
        Started::Kh1(element, layer)
    } else if owned {
        Started::NotStarted
    } else {
        Started::NotMine
    }
}

/// A frame trigger 1 of Valor's or Final's motion (`kh2fx::form_event`): its element from that
/// form's own package (`play`): true if KH1 plays it.
pub fn form_event(final_form: bool, motion: u16, el: i16) -> bool {
    play(if final_form { Pkg::Final } else { Pkg::Valor }, motion, el)
}

/// The Keyblade's vanish flash (`kh2fx::weapon_vanish`, as KH2 puts the Keyblade away, before
/// KH1's put-away): true if KH1 plays it (`play`).
pub fn weapon_vanish(motion: u16) -> bool {
    play(Pkg::Weapon, motion, super::kh2fx::WEAPON_VANISH as i16)
}

/// The binding KH1's start just made for `element` (the binding list's last node), noted for
/// `posed`. Steam only.
unsafe fn note_binding(base: usize, element: usize, actor: usize, pos: Option<PosOnly>) {
    let node = rd::<usize>(base + BIND_LIST_STEAM + 0x18);
    let ok = node != 0 && rd::<usize>(node + 0x10) == base + BIND_TASK_STEAM && rd::<usize>(node + 0x20) == element;
    let record = if ok { super::pose::ptr(rd::<u32>(node + 0x30)) } else { 0 };
    if record == 0 || !matches!(rd::<u8>(record + 4), 1 | 2 | 3) || pos.is_some() != (rd::<u8>(record + 4) == 1) {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 5 {
            log(&format!("KH2 effects (native): element {element:#x}'s binding not found (node {node:#x}); it keeps KH1's first placement"));
        }
        return;
    }
    if let Ok(mut f) = FRESH.lock() {
        f.push(Fresh { element, serial: rd::<u32>(element + EL_SERIAL), node, record, actor, pass: PASSES.load(Ordering::Relaxed), pos });
        FRESH_ANY.store(true, Ordering::Relaxed);
    }
}

/// The binding KH1's start just made for `element` made to let go on its first run without
/// moving it: its mode set to none with the bind-once bit (KH1's binding task, `0x1402AE910`,
/// then frees the record and ends, as for a mode it doesn't know).
unsafe fn disarm(base: usize, element: usize) {
    let node = rd::<usize>(base + BIND_LIST_STEAM + 0x18);
    let ok = node != 0 && rd::<usize>(node + 0x10) == base + BIND_TASK_STEAM && rd::<usize>(node + 0x20) == element;
    let record = if ok { super::pose::ptr(rd::<u32>(node + 0x30)) } else { 0 };
    if record == 0 || !matches!(rd::<u8>(record + 4), 1 | 2 | 3) {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 5 {
            log(&format!("KH2 effects (native): element {element:#x}'s binding not found (node {node:#x}); it stays bound"));
        }
        return;
    }
    wr::<u8>(record + 4, 0);
    wr::<u32>(record + 8, rd::<u32>(record + 8) | BIND_ONCE);
}

/// The binding is still `f`'s: its element running with its serial, its node KH1's binding of it.
unsafe fn fresh_live(base: usize, f: &Fresh) -> bool {
    instance(f.element, f.serial).is_some()
        && rd::<u16>(f.node) != 0
        && rd::<usize>(f.node + 0x10) == base + BIND_TASK_STEAM
        && rd::<usize>(f.node + 0x20) == f.element
        && rd::<u32>(f.node + 0x28) == f.serial
        && super::pose::ptr(rd::<u32>(f.node + 0x30)) == f.record
}

/// The binding record's matrix read now, as KH1's binding reads it (mode 2: the bone's matrix;
/// mode 3: that, with the point along the blade as its place).
unsafe fn read_now(base: usize, f: &Fresh) {
    let (mode, code, t) = (rd::<u8>(f.record + 4), rd::<u32>(f.record + 8), rd::<f32>(f.record + 0xC));
    let bone: unsafe extern "system" fn(usize, u32, usize) = std::mem::transmute(base + READ_BONE_STEAM);
    bone(f.actor, code, f.record + 0x30);
    if mode == 3 {
        let point: unsafe extern "system" fn(usize, u32, f32, usize) = std::mem::transmute(base + READ_POINT_STEAM);
        point(f.actor, code, t, f.record + 0x60);
    }
}

/// A position-only binding's offset for this pose (written to its record `+0x10`) and the place
/// it gives (KH2's `B · offset`): for a bone, its matrix (`0x14029EA00`, `0x2000 | n`) less the
/// point KH1's binding reads (`0x14029EB20` with the record's code and t), so the two agree
/// exactly; for the blade, the point along it with the blade bone's turn (KH1's mode 3 frame).
/// Rows of KH1's matrix are the bone's axes (row-vector form), translation in the last.
unsafe fn by_position_now(base: usize, actor: usize, record: usize, p: &PosOnly) -> [f32; 4] {
    let bone: unsafe extern "system" fn(usize, u32, usize) = std::mem::transmute(base + READ_BONE_STEAM);
    let point: unsafe extern "system" fn(usize, u32, f32, usize) = std::mem::transmute(base + READ_POINT_STEAM);
    let mut m = [0f32; 16];
    let mut pt = [0f32; 4];
    let (code, t) = (rd::<u32>(record + 8) & !BIND_ONCE, rd::<f32>(record + 0xC));
    let rcode = match p.anchor {
        Anchor::Bone(b) => RAW_BONE | b,
        _ => code,
    };
    bone(actor, rcode, m.as_mut_ptr() as usize);
    point(actor, code, t, pt.as_mut_ptr() as usize);
    let o = p.offset;
    let mut place = [0f32; 3];
    for k in 0..3 {
        let anchor = if matches!(p.anchor, Anchor::Bone(_)) { m[12 + k] } else { pt[k] };
        place[k] = anchor + o[0] * m[k] + o[1] * m[4 + k] + o[2] * m[8 + k];
    }
    // (w as KH1's own: its start sets the offset to (0, 0, 0, 1), and the binding adds all four.)
    let off = [place[0] - pt[0], place[1] - pt[1], place[2] - pt[2], rd::<f32>(record + 0x1C)];
    wr::<[f32; 4]>(record + 0x10, off);
    [place[0], place[1], place[2], pt[3] + off[3]]
}

/// Sora's skeleton pass (`pose`, the game thread), `after` it or before. KH2's binding reads its
/// bone first on its first run (Steam KH2 `0x1402C58F0`: `0x1402C5F70` then `0x1402C49D0` while
/// the element's timer `+0x58` is 0, after the object's pose; then apply, then read) and keeps
/// reading while the effect waits; KH1's (`0x1402AE910`) applies, then reads, and skips a waiting
/// element. So a binding of ours started this pass gets this pose after it (KH2's first run); on
/// later passes, before the pass (the pose KH2's binding read on its run before), while it still
/// waits, then it is let go (from then on KH1's apply-then-read is KH2's).
pub fn posed(after: bool) {
    if !FRESH_ANY.load(Ordering::Relaxed) {
        if after {
            PASSES.fetch_add(1, Ordering::Relaxed);
        }
        return;
    }
    let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    let pass = PASSES.load(Ordering::Relaxed);
    if let Ok(mut g) = FRESH.try_lock() {
        g.retain_mut(|f| unsafe {
            if !fresh_live(base, f) {
                return false;
            }
            if let Some(p) = f.pos.as_mut() {
                // Position only: after each pass, the offset KH1's binding adds to its point is
                // set to this pose's (bone place + bone turn x offset − the point); the place it
                // applies next is this pass's on the start pass (KH2's first run reads first),
                // else the pass before's (what KH2 applies), the binding then reading on.
                if after {
                    let cur = by_position_now(base, f.actor, f.record, p);
                    wr::<[f32; 4]>(f.record + 0x20, if f.pass == pass { cur } else { p.prev });
                    p.prev = cur;
                }
                return true;
            }
            if after {
                if f.pass == pass {
                    read_now(base, f);
                }
                return true;
            }
            if f.pass == pass {
                return true;
            }
            read_now(base, f);
            rd::<u32>(f.element + EL_FLAGS) & EL_WAITING != 0
        });
        FRESH_ANY.store(!g.is_empty(), Ordering::Relaxed);
    }
    if after {
        PASSES.fetch_add(1, Ordering::Relaxed);
    }
}

/// KH1's instance of a started element, if it is still that element's.
unsafe fn instance(element: usize, serial: u32) -> Option<usize> {
    if rd::<u32>(element + EL_FLAGS) & 3 != 1 || rd::<u32>(element + EL_SERIAL) != serial {
        return None;
    }
    let inst = super::pose::ptr(rd::<u32>(element + EL_INSTANCE));
    (inst != 0 && rd::<usize>(inst + INST_ELEMENT) == element && rd::<u16>(inst + INST_STATE) & 1 != 0).then_some(inst)
}

/// The element's wait and scale on KH1's instance just started, and its ending registered.
unsafe fn apply(element: usize, l: &Layer, motion: u16) -> bool {
    let serial = rd::<u32>(element + EL_SERIAL);
    let Some(inst) = instance(element, serial) else { return false };
    // KH2 adds a tenth of a frame to a bound element's wait. KH1 starts an element with a
    // bone binding only once its wait is down to -2 x its frame unit (`0x1401ECB50` / `0x1401ECF30`,
    // 2 game frames later than KH2, which starts it once its wait is down to 0): the tester's rule
    // (exact to KH2), so those 2 frames come off the wait. Each frame both games test the wait
    // before counting it down by the frame step, so the start frame is KH2's at any frame rate.
    let kh1_extra = if rd::<u32>(element + EL_FLAGS) & EL_BOUND != 0 { 2.0 * frame_unit() } else { 0.0 };
    wr::<f32>(inst + INST_WAIT, l.wait + if l.flags & BIND != 0 { 0.1 } else { 0.0 } - kh1_extra);
    if l.scale != [1.0; 3] {
        wr::<[f32; 3]>(inst + INST_SCALE, l.scale);
    }
    if l.flags & (ENDS_WITH_MOTION | ENDS_WITH_OWNER) != 0 {
        if let Ok(mut r) = RUNNING.lock() {
            r.retain(|x| unsafe { instance(x.element, x.serial) }.is_some());
            r.push(Run { element, serial, motion, layer: *l, fade: None });
        }
    }
    true
}

/// KH1's stop of the element (Steam); elsewhere it is left invisible and its loops ended.
unsafe fn stop(element: usize, inst: usize) {
    if super::addr::Build::current() == super::addr::Build::Steam {
        let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
        let f: unsafe extern "system" fn(usize) = std::mem::transmute(base + STOP_STEAM);
        f(element);
    } else {
        wr::<f32>(inst + INST_ALPHA, 0.0);
        wr::<u8>(inst + INST_ENDED, 1);
    }
}

/// KH2's end of an element (`docs/kh2-effects.md` §2): 0x80 ends its loops, else 0x40 fades it
/// over its fade-out frames (none: at once); else it plays to its own end. True if it's gone.
unsafe fn end(r: &mut Run, inst: usize) -> bool {
    if r.layer.flags & END_LOOP != 0 {
        wr::<u8>(inst + INST_ENDED, 1);
    } else if r.layer.flags & END_FADE != 0 && r.fade.is_none() {
        if r.layer.fadeout > 0.0 {
            r.fade = Some(rd::<f32>(inst + INST_ALPHA) / r.layer.fadeout);
        } else {
            stop(r.element, inst);
            return true;
        }
    }
    false
}

/// One of Sora's frames (`kh2fx::tick`): KH2's motion now and the frame's delta in 60 Hz frames.
/// Elements that end with their motion end when it changes; fades step.
pub fn tick(motion: u16, delta: f32) {
    manage();
    let changed = LAST_MOTION.swap(motion as u32, Ordering::Relaxed) != motion as u32;
    let Ok(mut g) = RUNNING.lock() else { return };
    if g.is_empty() {
        return;
    }
    g.retain_mut(|r| unsafe {
        let Some(inst) = instance(r.element, r.serial) else { return false };
        if changed && r.layer.flags & ENDS_WITH_MOTION != 0 && r.motion != motion && end(r, inst) {
            return false;
        }
        if let Some(rate) = r.fade {
            let a = rd::<f32>(inst + INST_ALPHA) - rate * delta.max(0.0);
            if a <= 0.0 {
                stop(r.element, inst);
                return false;
            }
            wr::<f32>(inst + INST_ALPHA, a);
        }
        true
    });
}

/// Sora's object replaced (`kh2fx::owner_gone`, a form change): elements that end with their
/// owner fade over their fade-out frames, or stop (as `kh2_fx::Effects::owner_gone`).
pub fn owner_gone() {
    let Ok(mut g) = RUNNING.lock() else { return };
    g.retain_mut(|r| unsafe {
        let Some(inst) = instance(r.element, r.serial) else { return false };
        if r.layer.flags & ENDS_WITH_OWNER == 0 || r.fade.is_some() {
            return true;
        }
        if r.layer.fadeout > 0.0 {
            r.fade = Some(rd::<f32>(inst + INST_ALPHA) / r.layer.fadeout);
            true
        } else {
            stop(r.element, inst);
            false
        }
    });
}

/// A spell effect KH1 plays for the spell scripts (`kh2fx`'s spell API): our handle, KH1's
/// element and its serial, its layer, its binding's node and record (`0` once let go: placed by
/// the script).
struct Spell {
    handle: u32,
    element: usize,
    serial: u32,
    layer: Layer,
    node: usize,
    record: usize,
}

static SPELLS: Mutex<Vec<Spell>> = Mutex::new(Vec::new());
static NEXT_SPELL: AtomicU32 = AtomicU32::new(1);
/// Our handles' bit in `kh2fx`'s (its own count from 1 below it).
pub const NATIVE_SPELL: u32 = 0x0400_0000;
/// KH1's apply of a binding's matrix to an element (Steam `0x1402ADB40 (element, matrix)`: its
/// turn from the matrix's rows (`0x1402A8EF0`, in 65536ths of a turn) and its place from the last).
const APPLY_STEAM: usize = 0x2ADB40;

/// What `spell_start` did.
pub enum SpellStart {
    /// KH1 plays it: our handle (`NATIVE_SPELL` set).
    Kh1(u32),
    /// KH1's, not started (no free element or slot).
    NotStarted,
    /// `kh2fx` plays it.
    NotMine,
}

/// Element `el` of spell package `level` (`kh2fx::spell_start`, the script's `pax_start_bind`):
/// on Sora by its bone code, through KH1 when its package is built and it passes `start_el`'s
/// gate (Steam only). Its endings are the script's (`spell_end` / `spell_kill` / `spell_alpha`),
/// and Sora's object going.
pub fn spell_start(level: u8, el: u16) -> SpellStart {
    let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    match start_el(Pkg::Spell(level), super::kh2fx::NO_MOTION, el as i16) {
        Started::NotMine => SpellStart::NotMine,
        Started::NotStarted => SpellStart::NotStarted,
        Started::Kh1(element, layer) => unsafe {
            let node = rd::<usize>(base + BIND_LIST_STEAM + 0x18);
            let ok = node != 0 && rd::<usize>(node + 0x10) == base + BIND_TASK_STEAM && rd::<usize>(node + 0x20) == element;
            let record = if ok { super::pose::ptr(rd::<u32>(node + 0x30)) } else { 0 };
            let handle = NATIVE_SPELL | (NEXT_SPELL.fetch_add(1, Ordering::Relaxed) & (NATIVE_SPELL - 1));
            if let Ok(mut g) = SPELLS.lock() {
                g.retain(|x| instance(x.element, x.serial).is_some());
                g.push(Spell { handle, element, serial: rd::<u32>(element + EL_SERIAL), layer, node: if record != 0 { node } else { 0 }, record });
            }
            SpellStart::Kh1(handle)
        },
    }
}

fn with_spell<R>(h: u32, f: impl FnOnce(&mut Spell, usize) -> R) -> Option<R> {
    let mut g = SPELLS.lock().ok()?;
    let i = g.iter().position(|x| x.handle == h)?;
    let sp = &mut g[i];
    let inst = unsafe { instance(sp.element, sp.serial) }?;
    Some(f(sp, inst))
}

/// The script puts its effect at `at` (KH2's world, which is KH1's; `kh2fx::spell_place`): KH2's
/// `at` stands for the effect's bone from now on. KH1's: its binding let go (disarmed, as an
/// unbound element's), then the element's place and turn from `at` with KH1's own apply
/// (`0x1402ADB40`); by position only, `at · offset` unturned.
pub fn spell_place(h: u32, at: glam::Mat4) {
    let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    with_spell(h, |sp, _| unsafe {
        if sp.record != 0 {
            let node = sp.node;
            if rd::<u16>(node) != 0 && rd::<usize>(node + 0x10) == base + BIND_TASK_STEAM && rd::<usize>(node + 0x20) == sp.element && super::pose::ptr(rd::<u32>(node + 0x30)) == sp.record {
                wr::<u8>(sp.record + 4, 0);
                wr::<u32>(sp.record + 8, rd::<u32>(sp.record + 8) | BIND_ONCE);
            }
            sp.record = 0;
            if let Ok(mut f) = FRESH.lock() {
                f.retain(|x| x.element != sp.element);
            }
        }
        if sp.layer.flags & POSITION_ONLY != 0 {
            let p = at.transform_point3(glam::Vec3::from(sp.layer.bone_offset));
            let place = super::pose::ptr(rd::<u32>(sp.element));
            let turn = super::pose::ptr(rd::<u32>(sp.element + 4));
            if place != 0 {
                wr::<[f32; 3]>(place, p.to_array());
            }
            if turn != 0 {
                wr::<[u32; 3]>(turn, [0; 3]);
            }
        } else {
            let m = at.to_cols_array();
            let apply: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + APPLY_STEAM);
            apply(sp.element, m.as_ptr() as usize);
        }
    });
}

/// The script ends its effect (`kh2fx::spell_end`): KH2's end (`end`: 0x80 its loops, else 0x40
/// its fade-out, else it plays on).
pub fn spell_end(h: u32) {
    let Some((element, serial, layer)) = with_spell(h, |sp, _| (sp.element, sp.serial, sp.layer)) else { return };
    let Ok(mut g) = RUNNING.lock() else { return };
    let i = match g.iter().position(|r| r.element == element && r.serial == serial) {
        Some(i) => i,
        None => {
            g.push(Run { element, serial, motion: super::kh2fx::NO_MOTION, layer, fade: None });
            g.len() - 1
        }
    };
    let gone = unsafe { instance(element, serial) }.is_none_or(|inst| unsafe { end(&mut g[i], inst) });
    if gone || (g[i].fade.is_none() && layer.flags & ENDS_WITH_OWNER == 0) {
        g.remove(i);
    }
}

/// The script kills its effect (`kh2fx::spell_kill`): KH1's stop.
pub fn spell_kill(h: u32) {
    with_spell(h, |sp, inst| unsafe { stop(sp.element, inst) });
}

/// The script sets its effect's alpha (`kh2fx::spell_alpha`): KH1's instance alpha, which its
/// draws multiply every vertex colour by.
pub fn spell_alpha(h: u32, alpha: f32) {
    with_spell(h, |_, inst| unsafe { wr::<f32>(inst + INST_ALPHA, alpha) });
}

/// The effect still plays (`kh2fx::spell_running`).
pub fn spell_running(h: u32) -> bool {
    with_spell(h, |_, _| ()).is_some()
}
