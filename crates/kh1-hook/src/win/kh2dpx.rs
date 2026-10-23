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
const MB: u32 = super::kh2ppp::MEGA_BIRTH;
const SHIPOLY: u32 = super::kh2ppp::SHIPOLY;
const COLOR: u32 = super::kh2ppp::COLOR;
const VT_MIME: u32 = super::kh2ppp::VT_MIME;
/// KH2's own programs an effect needs plugged in before it plays through KH1 (KH1's versions of
/// 49 and 201 aren't KH2's; 241 isn't in KH1). KH2's pppColor (43) is not a gate: without it
/// KH1's runs.
const GATES: [u32; 3] = [MB, SHIPOLY, VT_MIME];

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
];

/// The programs of `types` KH1 can't run as KH2 does yet (empty: it can play through KH1).
fn missing(types: &[u32]) -> Vec<u32> {
    types.iter().copied().filter(|t| !KH1_SAME.contains(t) && *t != COLOR && !(GATES.contains(t) && super::kh2ppp::plugged(*t))).collect()
}

/// KH1's anchor for a KH2 bone code. The bones by the two bind poses (`docs/kh2-effects.md`
/// §17.4, §18.3: each KH2 bone on the KH1 bone of the same body part at the same joint, whose
/// bind frame is the same to 0.05): pelvis 2 -> 2; +x leg: thigh 11 -> 43, shin 12 -> 44, foot 15
/// -> 47, toe 18 -> 50; -x leg: 40 -> 17, 41 -> 18, 44 -> 21, 47 -> 24; spine 67 -> 70; upper arms
/// 78 -> 198, 170 -> 93; hands 85 -> 203, 177 -> 98. KH2's weapon points 0x4003 / 0x4005 / 0x4007
/// (tip, root, halfway: `0x1403B43D0`) are KH1's blade points t = 1 / 0 / 0.5; its object root
/// 0x4000 is KH1's 0x8000.
fn anchor(code: u16) -> Option<Anchor> {
    Some(match code {
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
/// KH1's handles cover 32 MB regions; the DPX stays inside one.
const REGION: usize = 0x200_0000;
/// The name our DPX has in KH1's loaded-file list.
const FILE_NAME: &[u8] = b"kh2fx_p_ex100_dpx";

/// An HD texture for one texture record: the record's pixel offset in the DPX, the DDS.
struct Hd {
    offset: u32,
    dds: Vec<u8>,
}

struct Dpx {
    /// Owns the bytes; never freed or moved (KH1's effects point into it).
    _buf: Vec<u8>,
    at: usize,
    len: usize,
    md5: [u8; 16],
    hd: Vec<Hd>,
    layers: Vec<Layer>,
    /// Each built element's program types (`kh1dpx::Kh1Dpx::types`), by element.
    types: Vec<(i16, Vec<u32>)>,
    /// Elements whose KH1 play has been refused (logged once each).
    refused: Vec<i16>,
    /// Where KH2's own programs are (`kh1dpx::Kh1Dpx::programs`), for `kh2ppp::plug`.
    programs: Vec<(usize, u32)>,
    relocated: bool,
}

static DPX: Mutex<Option<Dpx>> = Mutex::new(None);
static STARTED: AtomicU32 = AtomicU32::new(0);

/// Builds the KH1 DPX of Sora's package in the background (from the KH2 install, as `kh2fx`),
/// once KH2 Sora's moveset is loaded (its motions group the elements).
pub fn start() {
    if super::settings::get().kh2_effects.is_none() {
        return;
    }
    std::thread::spawn(|| {
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
        let t = std::time::Instant::now();
        match build(kh2) {
            Ok((r, hd, hd_report, layers)) => {
                let dpx = &r.bytes;
                log(&format!("KH2 effects (native): Sora's DPX built, {} bytes, {} effects in {:.2} s; {}", dpx.len(), r.effects.len(), t.elapsed().as_secs_f32(), r.report));
                log(&format!("KH2 effects (native): HD textures: {hd_report}"));
                let mut buf = vec![0u8; dpx.len() * 2 + 16];
                let p = buf.as_ptr() as usize;
                let mut at = (p + 15) & !15;
                if (at & !(REGION - 1)) != ((at + dpx.len() - 1) & !(REGION - 1)) {
                    at = (at + dpx.len()) & !(REGION - 1);
                }
                let off = at - p;
                buf[off..off + dpx.len()].copy_from_slice(dpx);
                let types = layers.iter().filter_map(|l| r.effects.iter().position(|&n| n == l.number).map(|i| (l.el, r.types[i].clone()))).collect();
                if let Ok(mut g) = DPX.lock() {
                    *g = Some(Dpx { _buf: buf, at, len: dpx.len(), md5: r.md5, hd, layers, types, refused: Vec::new(), programs: r.programs.clone(), relocated: false });
                }
            }
            Err(e) => log(&format!("KH2 effects (native): Sora's DPX not built ({e}); kh2fx plays his effects")),
        }
    });
}

/// A native element's layer and its copy of its effect: the element's offset and rotation
/// folded into the header's local transform (`H' = S⁻¹ · T(t) · R · S · H`, S the element's
/// scale, which KH1 takes as the instance's: KH2's root is `T(B·t) · R(B·R) · S`, KH1's the
/// bound bone's `B`, times the header).
fn layer(pax: &[u8], parsed: &kh2_formats::pax::Pax, el: i16) -> Result<(Layer, kh2_formats::kh1dpx::Variant, String), String> {
    use glam::{Mat3, Vec3};
    use kh2_fx::math;
    let e = parsed.element(el as u16).ok_or(format!("no element {el}"))?;
    let effect = e.effect as u32;
    if !(0..0x100).contains(&el) {
        return Err(format!("element {el}: no effect number for it (0x700 | element)"));
    }
    if let Some(why) = unbindable(e.flags) {
        return Err(format!("element {el}: {why} (flags {:#x})", e.flags));
    }
    let bone = anchor(e.bone).ok_or(format!("element {el}: no KH1 anchor for KH2 bone {:#x}", e.bone))?;
    let (h_off, h_ang, _) = kh2_formats::kh1dpx::header_local(pax, effect).map_err(|e| e.to_string())?;
    let s = Vec3::from(e.scale);
    let safe = Vec3::new(if s.x != 0.0 { s.x } else { 1.0 }, if s.y != 0.0 { s.y } else { 1.0 }, if s.z != 0.0 { s.z } else { 1.0 });
    let r_e = math::euler_xyz(Vec3::from(e.rotation));
    let r_h = math::euler_xyz_i(h_ang);
    let position_only = e.flags & POSITION_ONLY != 0;
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
    let l = Layer { el, number, anchor: bone, flags: e.flags, bone_offset: if position_only { e.translation } else { [0.0; 3] }, wait: e.start_wait, fadeout: e.fadeout_frames as f32, scale: e.scale };
    Ok((l, kh2_formats::kh1dpx::Variant { number, effect, offset: offset.to_array(), angle }, note))
}

/// KH2 element flags KH1's binding doesn't do yet (`docs/kh2-effects.md` §2, §18.3).
fn unbindable(flags: u32) -> Option<&'static str> {
    if flags & (BIND | BIND_UNTIL_STARTED) == 0 {
        return Some("not bound (placed once at its start)");
    }
    if flags & BONE_SCALE != 0 {
        return Some("scaled by its bone");
    }
    if flags & TWO_POINT != 0 {
        return Some("a two-point element");
    }
    None
}

fn build(kh2: &super::pose::Kh2) -> Result<(kh2_formats::kh1dpx::Kh1Dpx, Vec<Hd>, String, Vec<Layer>), String> {
    let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
    let install = kh2_data::Install::locate(&exe).map_err(|e| e.to_string())?;
    let src = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
    let pack = install.read_any(src, "obj/P_EX100.a.us").map_err(|e| e.to_string())?;
    let common = install.read_any(src, "effect/texcommon.dpd").map_err(|e| format!("texcommon.dpd: {e}"))?;
    let bar = kh2_formats::bar::Bar::parse(&pack).map_err(|e| e.to_string())?;
    let pax = bar
        .entries
        .iter()
        .find(|e| e.name == "p_ex" && e.data.starts_with(b"PAX_"))
        .or_else(|| bar.entries.iter().find(|e| e.data.starts_with(b"PAX_")))
        .ok_or("no PAX in P_EX100")?;
    let pax_at = pax.data.as_ptr() as usize - pack.as_ptr() as usize;
    let parsed = kh2_formats::pax::Pax::parse(pax.data).map_err(|e| e.to_string())?;
    // Every element (the first with its id, as KH2's start takes): its layer and its copy of its
    // effect.
    let mut layers: Vec<Layer> = Vec::new();
    let mut variants: Vec<kh2_formats::kh1dpx::Variant> = Vec::new();
    let mut left = Vec::new();
    for e in &parsed.elements {
        let el = e.id as i16;
        if layers.iter().any(|l| l.el == el) || left.iter().any(|x: &String| x.starts_with(&format!("element {el}:"))) {
            continue;
        }
        match layer(pax.data, &parsed, el) {
            Ok((l, v, note)) => {
                if ON.contains(&el) {
                    log(&format!("KH2 effects (native): {note}"));
                }
                layers.push(l);
                variants.push(v);
            }
            Err(why) => left.push(why),
        }
    }
    if !left.is_empty() {
        log(&format!("KH2 effects (native): not built (kh2fx plays them): {}", left.join("; ")));
    }
    // Groups: the elements each of KH2 Sora's motions starts (in motion order; an element in
    // the first motion that starts it), then Revert's, then each other effect's elements; each
    // group split by the DPD its effects are in.
    let mut motions: Vec<(&u16, &character_core::Clip)> = kh2.character.moveset.clips.iter().collect();
    motions.sort_by_key(|m| *m.0);
    let mut sets: Vec<Vec<i16>> = motions
        .iter()
        .map(|(_, c)| c.events.iter().filter_map(|e| if let character_core::EventKind::Effect(el) = e.kind { Some(el) } else { None }).collect())
        .collect();
    sets.push(super::form::REVERT_ELEMENTS.to_vec());
    let dpd_of = |v: &kh2_formats::kh1dpx::Variant| kh2_formats::dpd::effect_source(pax.data, v.effect).ok().flatten().map(|(d, _)| d.as_ptr() as usize);
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
    for s in &sets {
        add(s, &mut groups);
    }
    let mut effects: Vec<u32> = variants.iter().map(|v| v.effect).collect();
    effects.dedup();
    for e in effects {
        let els: Vec<i16> = layers.iter().zip(&variants).filter(|(_, v)| v.effect == e).map(|(l, _)| l.el).collect();
        add(&els, &mut groups);
    }
    let r = kh2_formats::kh1dpx::build_groups(pax.data, &common, &groups).map_err(|e| e.to_string())?;
    // Elements of groups left out (too many texture slots, a texture missing) aren't KH1's.
    for (gi, why) in &r.skipped {
        let els: Vec<i16> = groups[*gi].iter().filter_map(|v| layers.iter().find(|l| l.number == v.number).map(|l| l.el)).collect();
        log(&format!("KH2 effects (native): elements {els:?} not built ({why}); kh2fx plays them"));
    }
    layers.retain(|l| r.effects.contains(&l.number));
    // KH2's HD textures for the two files, each keyed by the offset in its file of the texture
    // record's pixels it replaces.
    let own_hd = install.read_remastered_any(src, "obj/P_EX100.a.us").map_err(|e| format!("P_EX100 HD textures: {e}"))?;
    let common_hd = install.read_remastered_any(src, "effect/texcommon.dpd").map_err(|e| format!("texcommon HD textures: {e}"))?;
    let mut hd = Vec::new();
    let mut report = Vec::new();
    // Each source DPD's texture records (own: in the PAX; common: in texcommon), by where it starts.
    let mut recs_of: Vec<((bool, usize), Vec<(usize, i32)>)> = Vec::new();
    for t in &r.textures {
        // A texture's HD image is the one for its TBP (KH1 and KH2 make one texture per TBP):
        // the asset whose offset is any of the source DPD's records with that TBP.
        let src_file: &[u8] = if t.own { pax.data } else { &common };
        if !recs_of.iter().any(|x| x.0 == (t.own, t.src_dpd_at)) {
            let recs = kh2_formats::kh1dpx::texture_records(&src_file[t.src_dpd_at..]).map_err(|e| e.to_string())?;
            recs_of.push(((t.own, t.src_dpd_at), recs));
        }
        let recs = &recs_of.iter().find(|x| x.0 == (t.own, t.src_dpd_at)).unwrap().1;
        let (assets, base) = if t.own { (&own_hd, pax_at + t.src_dpd_at) } else { (&common_hd, t.src_dpd_at) };
        let exact = base + t.src_offset + 0x20;
        let found = assets.iter().find(|a| a.offset as usize & 0xFF_FFFF == exact).or_else(|| {
            assets.iter().find(|a| recs.iter().any(|&(o, tbp)| tbp == t.src_tbp && base + o + 0x20 == a.offset as usize & 0xFF_FFFF))
        });
        let what = if t.own { "own" } else { "common" };
        match found {
            Some(a) => {
                report.push(format!("{what} {:#x} (tbp {:#x}) <- {} ({} bytes{})", exact, t.src_tbp, a.name, a.data.len(), if a.offset as usize & 0xFF_FFFF == exact { "" } else { ", same TBP" }));
                hd.push(Hd { offset: (t.dpx_offset + 0x20) as u32, dds: a.data.clone() });
            }
            None => report.push(format!("{what} {:#x} (tbp {:#x}) <- none", exact, t.src_tbp)),
        }
    }
    let list = |v: &Vec<kh2_formats::egs::HdAsset>| v.iter().map(|a| format!("{} for {:#x} ({} bytes, at {:#x})", a.name, a.offset, a.data.len(), a.pkg_offset)).collect::<Vec<_>>().join(", ");
    report.push(format!("P_EX100 assets [{}], texcommon assets [{}]", list(&own_hd), list(&common_hd)));
    Ok((r, hd, report.join("; "), layers))
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// Enters the DPX in KH1's loaded-file list with its HD textures (see the module notes).
unsafe fn register(base: usize, d: &Dpx) -> Result<usize, String> {
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
    // KH1's own entries: the vtables of a resource entry, its HD-entry list and (the commonest
    // one among them) an HD entry.
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
    let malloc: unsafe extern "C" fn(usize) -> usize = std::mem::transmute(malloc);
    let aligned: unsafe extern "C" fn(usize, usize) -> usize = std::mem::transmute(aligned);
    // The resource entry (0x688 bytes, as `0x1400E2F60` makes it).
    let rec = malloc(0x688);
    if rec == 0 {
        return Err("out of memory".into());
    }
    std::ptr::write_bytes(rec as *mut u8, 0, 0x688);
    wr(rec, res_vt);
    wr(rec + 0x10, d.at);
    wr(rec + 0x18, d.len as u32);
    wr(rec + 0x1C, u32::MAX);
    for (i, &b) in FILE_NAME.iter().enumerate() {
        wr(rec + 0x20 + i, b);
        wr(rec + 0x1C4 + i, b);
        wr(rec + 0x620 + i, b);
    }
    wr(rec + 0x208, d.md5);
    wr(rec + 0x218, u32::MAX);
    wr(rec + 0x640, d.len as u32);
    wr(rec + 0x644, rd::<u32>(list + 0x20));
    wr(rec + 0x64C, u32::MAX);
    wr(rec + 0x658, d.at);
    wr(rec + 0x668, list_vt);
    // Its HD entries (0x78 bytes each, as `0x1400E1680` makes them).
    let mut tail = 0usize;
    for (i, h) in d.hd.iter().enumerate() {
        let e = malloc(0x78);
        let data = aligned(h.dds.len(), 16);
        if e == 0 || data == 0 {
            return Err("out of memory".into());
        }
        std::ptr::copy_nonoverlapping(h.dds.as_ptr(), data as *mut u8, h.dds.len());
        std::ptr::write_bytes(e as *mut u8, 0, 0x78);
        wr(e, entry_vt);
        let name = format!("-{:x}.dds\0", h.offset);
        for (k, &b) in name.as_bytes().iter().take(0x3F).enumerate() {
            wr(e + 0x10 + k, b);
        }
        wr(e + 0x50, h.dds.len() as u32);
        wr(e + 0x54, h.offset & 0xFF_FFFF);
        wr(e + 0x60, data);
        wr(e + 0x68, 999u32);
        if i < 0x100 {
            wr(rec + 0x21C + i * 4, h.offset);
        }
        if tail == 0 {
            wr(rec + 0x670, e);
        } else {
            wr(tail + 8, e);
        }
        tail = e;
        wr(rec + 0x678, e);
        wr(rec + 0x680, rd::<i32>(rec + 0x680) + 1);
    }
    // Appended to the list as `0x1400E2F60` does.
    if rd::<usize>(list + 8) == 0 {
        wr(list + 8, rec);
    }
    let last = rd::<usize>(list + 0x10);
    if last != 0 {
        wr(last + 8, rec);
    }
    wr(list + 0x10, rec);
    wr(list + 0x18, rd::<i32>(list + 0x18) + 1);
    Ok(rec)
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
    if !ON.contains(&el) {
        return false;
    }
    let Ok(mut g) = DPX.lock() else { return false };
    let Some(d) = g.as_mut() else { return false };
    let Some(&layer) = d.layers.iter().find(|l| l.el == el) else { return false };
    let (number, bone) = (layer.number, layer.anchor);
    let steam = super::addr::Build::current() == super::addr::Build::Steam;
    // KH1's blade and position bindings are used on Steam only (their addresses are Steam's):
    // elsewhere kh2fx plays them.
    let position_only = layer.flags & POSITION_ONLY != 0;
    if (matches!(bone, Anchor::Blade(_)) || position_only) && !steam {
        return false;
    }
    let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    if !d.relocated {
        match unsafe { register(base, d) } {
            Ok(rec) => log(&format!("KH2 effects (native): Sora's DPX entered in KH1's loaded files ({:#x}) with {} HD textures", rec, d.hd.len())),
            Err(e) => log(&format!("KH2 effects (native): Sora's DPX not entered in KH1's loaded files ({e}); its textures won't draw")),
        }
        let f: unsafe extern "system" fn(usize) = unsafe { std::mem::transmute(base + RELOCATE_RVA) };
        unsafe { f(d.at) };
        d.relocated = true;
        log(&format!("KH2 effects (native): Sora's DPX at {:#x} relocated by KH1", d.at));
        // KH2's own programs, as entries of ours in KH1's format (`kh2ppp`).
        super::kh2ppp::plug(d.at, &d.programs);
    }
    // An effect with a program KH1 doesn't run as KH2 plays through kh2fx (KH1's own version, or
    // its dummy, isn't KH2's).
    let lacks = d.types.iter().find(|t| t.0 == el).map_or_else(Vec::new, |t| missing(&t.1));
    if !lacks.is_empty() {
        if !d.refused.contains(&el) {
            d.refused.push(el);
            log(&format!("KH2 effects (native): element {el} runs programs {lacks:?}, not in KH1 as KH2's yet; kh2fx plays it"));
        }
        return false;
    }
    // (Steam: KH1's alone from here on, the old player retired for it.)
    let owned = steam;
    let sora = super::takeover::sora_actor();
    if sora < 0x10000 {
        return owned;
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
    if element != 0 && steam && (once == 0 || position_only) {
        let pos = position_only.then_some(PosOnly { anchor: bone, offset: layer.bone_offset, prev: [0.0; 4] });
        unsafe { note_binding(base, element, sora, pos) };
    }
    let set = element != 0 && unsafe { apply(element, &layer, motion) };
    if STARTED.fetch_add(1, Ordering::Relaxed) < 20 {
        log(&format!(
            "KH2 effect (native): element {el} of motion {motion} -> KH1 effect {number:#x} on {bone:?}: {}",
            if element == 0 { "not started (no free element or effect slot)".to_string() } else { format!("element {element:#x}{}", if set { "" } else { " (its instance not found: no wait, scale or ending)" }) }
        ));
    }
    element != 0 || owned
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
