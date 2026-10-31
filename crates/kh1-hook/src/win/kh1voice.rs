//! Sora's KH1 voice (the user: "sfx is calling kh2 battle quotes but I need kh1 battle quotes to
//! be called"), played by KH1's own code, `docs/kh1-voice.md`:
//!
//! - **Groups** (`0x14028E060 (actor, group, other)`): a speaker's voice table is in `btltbl.bin`
//!   at the offset `0x140529560[character]` gives (Sora's `0xBC48`, 20 groups), 11 bytes a
//!   group: a priority, then five (clip, weight) pairs. KH1 picks one clip: a weight above 0 is a
//!   percentage chance, 0 an equal share of what's left; clip 0xFD "the other's line", 0xFE
//!   silence, 0xFF none. So a group is already a random choice of clips.
//! - **Clips** (`0x14028E7A0 (actor, clip, priority)`): one clip of the speaker's bank, played if
//!   nothing of a higher priority is speaking for him.
//!
//! Sora's groups, by what calls them: 0 / 1 hurt (light, heavy), 2 knocked out, 4 his combo
//! swings (motion 200's row: clips 21, 22, 23, 24, 28, an equal share each), 5 the finishers,
//! 6 the air finishers, 7..13 the seven spells (Cure's and Aero's on an ally another line), 15 / 17
//! the summons' calls (the summon sequences `0x1403959B0`.. for xs_dumbo .. xs_simba): 17 is clip
//! 37 three times in four, else the plain grunt 30; clip 37, which no other group plays, is his
//! "Give me strength!".

// (`give_me_strength` and `dodge_roll` are called from the Drive's start and KH2's dodge roll.)
#![allow(dead_code)]

use super::addr::Rva;
use super::log;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// KH1's voice group play `(actor, group, other actor or 0)`.
const GROUP_RVA: Rva = Rva(0x28E060);
/// KH1's voice clip play `(actor, clip, priority)`.
const CLIP_RVA: Rva = Rva(0x28E7A0);
/// Sora's combo swing grunts (motion 200's attack row, its voice byte `+0xF`).
pub const SWING_GROUP: i32 = 4;
/// "Give me strength!": the summons' call (group 17)'s own clip, and that group's priority.
pub const GIVE_ME_STRENGTH_CLIP: i32 = 37;
const CALL_PRIORITY: i32 = 3;
/// "Defense!": one of Sora's two own Aero clips (group 13: clips 46 / 47, 50 / 50, `btltbl.bin`,
/// Checked (files)); which of the two is "Defense!" is not heard yet (**Assumption**: 46), so
/// every play is logged with its clip. The spell rows' priority is 3.
pub const DEFENSE_CLIP: i32 = 46;
const SPELL_PRIORITY: i32 = 3;

fn base() -> usize {
    unsafe { super::sys::GetModuleHandleA(std::ptr::null()) as usize }
}

/// Whether `actor` can speak now: a live actor with its stats and object (the voice code reads
/// `+0x6C` and `+0x130`).
unsafe fn speaker(actor: usize) -> bool {
    actor >= 0x10000
        && super::crash::readable(actor)
        && super::crash::readable(actor + 0x370)
        && std::ptr::read_unaligned((actor + 0x6C) as *const u32) != 0
        && std::ptr::read_unaligned((actor + 0x130) as *const u32) != 0
}

/// Logs the first few calls of each kind.
fn note(n: &AtomicU32, line: impl FnOnce() -> String) {
    if super::log_on(super::LogCat::Sound) && n.fetch_add(1, Ordering::Relaxed) < 6 {
        log!(Sound, "KH1 voice: {}", line());
    }
}

/// KH1's voice group `group` on `actor` (KH1 picks the clip). Does nothing for no actor.
pub unsafe fn play(actor: usize, group: i32) {
    if !speaker(actor) || group < 0 {
        return;
    }
    let f: unsafe extern "system" fn(usize, i32, usize) = std::mem::transmute(base() + GROUP_RVA);
    f(actor, group, 0);
}

/// KH1's voice clip `clip` on `actor` at `priority`.
pub unsafe fn clip(actor: usize, clip: i32, priority: i32) {
    if !speaker(actor) || clip < 0 {
        return;
    }
    let f: unsafe extern "system" fn(usize, i32, i32) = std::mem::transmute(base() + CLIP_RVA);
    f(actor, clip, priority);
}

/// One of his combo swing grunts, at random (KH1's own pick of group 4).
pub unsafe fn swing_grunt(actor: usize) {
    static N: AtomicU32 = AtomicU32::new(0);
    note(&N, || format!("a swing grunt (group {SWING_GROUP})"));
    play(actor, SWING_GROUP);
}

/// "Give me strength!" (the user: when he changes into a Drive Form).
pub unsafe fn give_me_strength(actor: usize) {
    static N: AtomicU32 = AtomicU32::new(0);
    note(&N, || format!("\"Give me strength!\" (clip {GIVE_ME_STRENGTH_CLIP})"));
    clip(actor, GIVE_ME_STRENGTH_CLIP, CALL_PRIORITY);
}

/// The dodge roll's call (the user: "make the sound one of the grunts during his first keyblade
/// swing and allow him to randomize which grunts he picks").
pub unsafe fn dodge_roll(actor: usize) {
    static N: AtomicU32 = AtomicU32::new(0);
    note(&N, || "the dodge roll's grunt".into());
    play(actor, SWING_GROUP);
}

/// What a stand-in plays (t1017).
#[derive(Clone, Copy)]
enum Line {
    /// The dodge roll's grunt.
    Roll,
    /// One of KH1's voice groups for Sora (4 swing grunts, 5 finisher lines, 6 the big line; his
    /// voice table `btltbl.bin` +0xBC48, Checked (files)).
    Group(i32),
    /// The first Rave of a Sonic Rave silent, the rest a swing grunt (`RAVES`).
    Rave,
    /// "Give me strength!", the Drive's line (a limit's start: the owner's answer 1, t1012 round,
    /// "Drive-style line").
    Drive,
}

/// KH1's finisher-line and big-line groups (KH1's Sonic Blade ends on 6, Ars Arcanum on 5 then 6:
/// their rows' `+0xF`, Checked (files)).
const FINISHER_GROUP: i32 = 5;
const BIG_GROUP: i32 = 6;

/// KH1 lines standing in for a KH2 voice (its frame trigger 13 / 14) on a KH2 move KH1 gives no
/// voice of its own: (KH2 motion, KH2 voice, what plays). KH1 plays Sora's battle voice only from
/// code (attack rows, spells, hits, summons), never from a motion, and its roll has none
/// (`docs/kh1-voice.md`, Checked (decomp + files)); a move with an attack row already has KH1's row
/// voice, so only row-less moves are here.
/// - KH2's dodge roll (motion 209, voice 7 at frame 0, Checked (files)): a swing grunt (group 4),
///   the user's pick for it before ("one of the grunts during his first keyblade swing").
/// - t1017, Sonic Rave (the owner, t1012: "sonic rave is missing quotes for kh1"; Limit Form's rows
///   carry no KH1 voice, `kh1rows`' limit moves). KH2's (Checked (files), Limit Form's motions'
///   trigger 14): the start 211 voice 8, each Rave 212 voice 4, the Finish 213 voice 6, at frame 0.
///   KH1's Sonic Blade (its rows 0x1B..0x21's `+0xF`, Checked (files)): its first dash silent, the
///   next ones a swing grunt, the last the big line; the owner's answer 2: KH1's pattern (the first
///   Rave silent, the rest grunts, the Finish the big line), the start the Drive's line (answer 1).
/// - t1018, Last Arcanum (the owner, t1012: "last arcanum is missing qoutes"). KH2's (Checked
///   (files)): the start 221 voice 6 at 0, 4 at 77, 5 at 104; Bash 1..4 (222..225) voice 7; Bash 5
///   (226) voice 5; The End (227) voice 6. The owner's answer 3: by kind (KH2's 4 and 7 a swing
///   grunt, 5 KH1's finisher line, 6 KH1's big line, as KH1's Ars Arcanum ends on 5 then 6), the
///   start's frame-0 line the Drive's (answer 1).
const STAND_INS: [(u16, i16, Line); 11] = [
    (character_core::motion::DODGE_ROLL, 7, Line::Roll),
    (211, 8, Line::Drive),
    (212, 4, Line::Rave),
    (213, 6, Line::Group(BIG_GROUP)),
    (221, 6, Line::Drive),
    (221, 4, Line::Group(SWING_GROUP)),
    (221, 5, Line::Group(FINISHER_GROUP)),
    (222, 7, Line::Group(SWING_GROUP)),
    (223, 7, Line::Group(SWING_GROUP)),
    (224, 7, Line::Group(SWING_GROUP)),
    (225, 7, Line::Group(SWING_GROUP)),
];
/// (Last Arcanum's Bash 5 and The End, t1018.)
/// t1019, Strike Raid and Infinity by the same rule (the owner's answer 4, "add them now"). KH2's
/// (Checked (files)): Strike Raid's start 231 voice 8, each throw 232 voice 4 at frame 8, Judgment
/// 233 voice 6, the catch 234 none; Infinity's start 241 voice 8, the spins 242 / 244 voice 4, the
/// hover 246 voice 5, Shoot / Impact 247 voice 7, Giga Impact 248 voice 6.
const STAND_INS_MORE: [(u16, i16, Line); 11] = [
    (226, 5, Line::Group(FINISHER_GROUP)),
    (227, 6, Line::Group(BIG_GROUP)),
    (231, 8, Line::Drive),
    (232, 4, Line::Group(SWING_GROUP)),
    (233, 6, Line::Group(BIG_GROUP)),
    (241, 8, Line::Drive),
    (242, 4, Line::Group(SWING_GROUP)),
    (244, 4, Line::Group(SWING_GROUP)),
    (246, 5, Line::Group(FINISHER_GROUP)),
    (247, 7, Line::Group(SWING_GROUP)),
    (248, 6, Line::Group(BIG_GROUP)),
];

/// t1030, Master Form (the owner, §24.2 answer 7: KH1 stand-ins by t1017..t1019's rule). KH2's
/// (Checked (files), `P_EX100_TRIF.mset` trigger 14, re-dumped): 181 / 182 / 185 voice 4 at frame 0;
/// 183 voice 6 at 0, 4 at 50, 5 at 100; 184 voice 8 at 0, 5 at 190; the arrival 53 none. Each of
/// these moves runs a KH1 attack row (the air combo on KH1's 0xCC..0xCE rows, ours "181" / "182" /
/// "185" / "184" from KH1's air rows), whose start already says the row's voice at frame 0
/// (`btltbl.bin` +0xA9F0 `+0xF`, Checked (files): rows 17 / 18 (0xCD / 0xCC) group 4, rows 16 / 19
/// (0xCE) group 5; our combo rows keep their template's byte, `kh1rows`). So, as the rule's "only
/// row-less moves", the frame-0 voices get no stand-in here (one would double the row's line);
/// only the lines KH2 says later in the move, which no row covers: 183's 4 (a swing grunt) and 5
/// (KH1's finisher line), 184's 5. Only in Master (`form::in_master`): the motion ids are shared
/// with base Sora's and the other forms' moves, whose voices are left as they were.
const MASTER_STAND_INS: [(u16, i16, Line); 3] = [(183, 4, Line::Group(SWING_GROUP)), (183, 5, Line::Group(FINISHER_GROUP)), (184, 5, Line::Group(FINISHER_GROUP))];

/// The Raves of a Sonic Rave so far (the start's line resets it) and the game frame the last fired
/// (the script starts the first Rave over on the frame KH1's queue started it, so a second key
/// within two frames is the same Rave).
static RAVES: AtomicU32 = AtomicU32::new(0);
static RAVE_AT: AtomicU64 = AtomicU64::new(0);

/// A KH2 voice key of a baked slot fired with Battle Quotes KH1 (`kh2slots::sound_event`): KH1's
/// stand-in line, if the move has one (`STAND_INS`), on KH1's motion pass as the KH2 voice would be.
pub unsafe fn stand_in(actor: usize, motion: u16, voice: i16) {
    // (t1030: Master's own, only in Master, `MASTER_STAND_INS`.)
    let master: &[(u16, i16, Line)] = if super::form::in_master() { &MASTER_STAND_INS } else { &[] };
    let Some(&(_, _, line)) = master.iter().chain(STAND_INS.iter()).chain(STAND_INS_MORE.iter()).find(|&&(m, v, _)| m == motion && v == voice) else { return };
    static N: AtomicU32 = AtomicU32::new(0);
    match line {
        Line::Roll => dodge_roll(actor),
        Line::Group(g) => {
            note(&N, || format!("KH2 motion {motion}'s voice {voice}: KH1's group {g}"));
            play(actor, g);
        }
        Line::Drive => {
            RAVES.store(0, Ordering::Relaxed);
            give_me_strength(actor);
        }
        Line::Rave => {
            let now = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
            let last = RAVE_AT.swap(now, Ordering::Relaxed);
            if last != 0 && now.saturating_sub(last) <= 2 {
                return;
            }
            let n = RAVES.fetch_add(1, Ordering::Relaxed);
            note(&N, || format!("Sonic Rave's Rave {}: {}", n + 1, if n == 0 { "silent (KH1's first dash)" } else { "a swing grunt" }));
            if n > 0 {
                play(actor, SWING_GROUP);
            }
        }
    }
}

/// Reflect's finisher with Battle Quotes KH1 (the tester: "the finisher will say "defence!" its
/// one of the lines he picks when casting aero"): KH1's Aero clip, through KH1's clip play.
pub unsafe fn defense(actor: usize) {
    static N: AtomicU32 = AtomicU32::new(0);
    if super::log_on(super::LogCat::Sound) && N.fetch_add(1, Ordering::Relaxed) < 40 {
        log!(Sound, "KH1 voice: Reflect finisher, KH1's Aero line \"Defense!\" (clip {DEFENSE_CLIP}, priority {SPELL_PRIORITY}; if it isn't \"Defense!\" it is the other Aero clip, 47)");
    }
    clip(actor, DEFENSE_CLIP, SPELL_PRIORITY);
}
