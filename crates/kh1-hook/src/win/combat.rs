//! Stage 10: KH2's combo on KH1's enemies.
//!
//! From Ghidra (the user's Epic exe) and the stage 9 probe:
//! - The Attack command (`0x1402A5380`, command type 1 in `0x142D54440`) picks a row of the
//!   actor's attack table and runs it with `0x1402A4760` `(actor, row, dir)`; `0x1402A17F0`
//!   reads the row (`0x142D5D4B0[object id] + row * 0x14`: the motion request at +0, the attack
//!   data at `0x142D23740 + [+4]`, sound/effect bytes at +0xC..+0xE), sets the motion and makes
//!   the attack object (`0x1402B10E0` `(actor, hit bit from 0x1402B15D0, data)`).
//! - Each frame `0x1402B22D0` tests the live attack objects against every hittable actor's
//!   body parts (`0x14028FEE0` says which actors; a part's shape from `0x1402B3CB0`: type,
//!   radius, half height, centre), and a contact runs `0x1402B1ED0` `(attack entry, body
//!   entry)`: the hit record through `0x1402BD180` (damage `0x1402BD8D0`), its push direction,
//!   the hit effect and the Keyblade's hit sound. `0x1402B13D0` frees an attack object.
//!
//! While KH2 has Sora, a regular Attack command is not run: it is KH2's attack button (`press`).
//! KH2's own hit detection (its swept attack shapes and hit records, the way the playground does
//! it) runs against KH1's body parts, and each hit KH2 lets through is landed with
//! KH1's own contact code, through an attack object made from Sora's own row (ground or air,
//! swing or finisher, as KH2's attack is), so HP, the enemy's reaction, drops, EXP, effects and
//! sounds are KH1's. The damage is scaled by the KH2 attack's power (`POWER`, applied to the
//! hit record in `probe::hit`).

use super::addr::Rva;
use super::settings::Walls;
use super::{hook, log, pose, takeover};
use character_core::{Character, Output, TargetInfo};
use combat_core::{first_overlap, AttackVolume, HitRecord, Shape};
use kh2_formats::battle::Battle;
use kh2_rig::RigAnim;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

const EXEC_RVA: Rva = Rva(0x2A4760);
/// KH1's menu sounds (`0x140177F60 (n)`; 4: can't).
const SOUND_RVA: Rva = Rva(0x177F60);
const EXEC_PROLOGUE: [u8; 11] = [0x4C, 0x8B, 0xDC, 0x57, 0x48, 0x81, 0xEC, 0x90, 0x00, 0x00, 0x00];
const COMMAND_RVA: Rva = Rva(0x2D54440);
const PARTY_RVA: Rva = Rva(0x2D37C80);
const ROWS_RVA: Rva = Rva(0x2D5D4B0);
const ATTACK_DATA_RVA: Rva = Rva(0x2D23740);
const NEXT_ACTOR_RVA: Rva = Rva(0x28F990);
const HITTABLE_RVA: Rva = Rva(0x28FEE0);
const PART_SHAPE_RVA: Rva = Rva(0x2B3CB0);
const NEW_ATTACK_RVA: Rva = Rva(0x2B10E0);
const FREE_BIT_RVA: Rva = Rva(0x2B15D0);
const LAND_RVA: Rva = Rva(0x2B1ED0);
const FREE_ATTACK_RVA: Rva = Rva(0x2B13D0);
/// Enemies farther than this from Sora are not looked at.
const NEAR: f32 = 1500.0;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_EXEC: AtomicUsize = AtomicUsize::new(0);
static ORIG_COMMAND: AtomicUsize = AtomicUsize::new(0);
/// KH1's command step `(actor, dir)`: runs the command at `0x142D54440` (type byte first).
const COMMAND_STEP_RVA: Rva = Rva(0x2A5380);
const COMMAND_STEP_PROLOGUE: [u8; 11] = [0x40, 0x55, 0x56, 0x57, 0x48, 0x81, 0xEC, 0x60, 0x01, 0x00, 0x00];
const STATE_RVA: Rva = Rva(0x2AF7D0);
/// KH1's can-act test (`takeover`'s too).
const CAN_ACT_RVA: Rva = Rva(0x2956E0);
/// The Attack command was given since KH2's last frame.
static PRESSED: AtomicBool = AtomicBool::new(false);
/// While KH2's hit is landed: its power in percent (0 otherwise).
pub static POWER: AtomicU32 = AtomicU32::new(0);
static LOGGED: AtomicU32 = AtomicU32::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// Waiting commands' log, on its own count (the shared one runs out in a long session).
fn cmd_log(s: String) {
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 200 {
        log(&s);
    }
}

fn note(s: String) {
    if LOGGED.fetch_add(1, Ordering::Relaxed) < 300 {
        log(&s);
    }
}

type ExecFn = unsafe extern "system" fn(usize, i32, usize);

unsafe extern "system" fn exec(actor: usize, row: i32, dir: usize) {
    let base = BASE.load(Ordering::Relaxed);
    let sora = rd::<u64>(base + PARTY_RVA) as usize;
    let command = rd::<u8>(base + COMMAND_RVA);
    // KH1's own combo (`stylecombo`, the Ground / Aerial Combo style set to KH1): its row runs.
    if actor == sora && command == 1 && takeover::ON.load(Ordering::Relaxed) && super::stylecombo::kh1_row(actor) {
        let f: ExecFn = std::mem::transmute(ORIG_EXEC.load(Ordering::Acquire));
        return f(actor, row, dir);
    }
    // Flying (KH1 has him, state 8): KH2's aerial combo instead of KH1's flying attack.
    if actor == sora && command == 1 && takeover::ON.load(Ordering::Relaxed) && rd::<i32>(actor + 0x70) == 8 {
        takeover::FLY_ATTACK.store(true, Ordering::Relaxed);
        note(format!("Attack command while flying (KH1 row {row}): KH2's aerial combo"));
        return;
    }
    // KH1's jump or glide (`stylejump`) with KH1's combo where he is: KH1's own row, and the
    // hand-off is that combo's from here.
    if actor == sora && command == 1 && takeover::ON.load(Ordering::Relaxed) && super::stylejump::active() && super::stylecombo::kh1_here(matches!(rd::<i32>(actor + 0x70), 1 | 2)) {
        super::stylejump::clear();
        super::stylecombo::started();
        takeover::FROM_COMMAND.store(true, Ordering::Relaxed);
        note(format!("Attack command in KH1's jump (KH1 row {row}): KH1's combo"));
        let f: ExecFn = std::mem::transmute(ORIG_EXEC.load(Ordering::Acquire));
        return f(actor, row, dir);
    }
    // KH1's roll (`styleroll`) takes an Attack (KH1's own window for it): by the Base Combo
    // style, KH1's combo from there, or KH2 takes him with the press for its combo.
    if actor == sora && command == 1 && takeover::ON.load(Ordering::Relaxed) && super::styleroll::active() {
        if super::stylecombo::kh1_here(false) {
            super::styleroll::clear();
            super::stylecombo::started();
            takeover::FROM_COMMAND.store(true, Ordering::Relaxed);
            note(format!("Attack command in KH1's roll (KH1 row {row}): KH1's combo"));
            let f: ExecFn = std::mem::transmute(ORIG_EXEC.load(Ordering::Acquire));
            return f(actor, row, dir);
        }
        super::styleroll::attack();
        takeover::ATTACK_AFTER.store(true, Ordering::Relaxed);
        note(format!("Attack command in KH1's roll (KH1 row {row}): KH2's combo, KH2 takes him"));
        return;
    }
    // KH1 still has him for one of its own moves (a spell's cast, an item): the press is KH2's
    // once that ends, not KH1's own combo (the user: KH1's Ripple Drive came out after a spell,
    // twice, finishing Riku).
    if actor == sora && command == 1 && takeover::ON.load(Ordering::Relaxed) && takeover::kh1_move() {
        takeover::ATTACK_AFTER.store(true, Ordering::Relaxed);
        note(format!("Attack command during KH1's own move (KH1 row {row}): KH2's once it ends"));
        return;
    }
    if actor == sora && command == 1 && takeover::ON.load(Ordering::Relaxed) && takeover::kh2_has() {
        PRESSED.store(true, Ordering::Relaxed);
        if let Some(r) = row_info(actor, row) {
            attack_shown(r.motion as u16 as u32);
        }
        note(format!("Attack command (KH1 row {row}, motion {}): KH2's attack button", row_info(actor, row).map_or(-1, |r| r.motion as i32)));
        return;
    }
    // KH1 is playing one of its own motions on him (a reaction, the end of a spell), or KH2 lost
    // him for a frame: the press is KH2's once KH1 lets go, never KH1's own attack row (the user:
    // Attack on the first free frame after a KH1 move gave KH1's attacks, and its combo carried
    // on from there out of order).
    if actor == sora && command == 1 && takeover::kh1_borrows(actor) {
        takeover::set_air_cast(None);
        takeover::hand_to_kh1(&format!("Attack pressed during KH1's own motion (KH1 row {row})"));
        takeover::ATTACK_AFTER.store(true, Ordering::Relaxed);
        note(format!("Attack command during KH1's own motion (KH1 row {row}): KH2's once it ends"));
        return;
    }
    // Valor Form: no KH1 attack row run for a command other than Attack (its specials, which the
    // command step runs as attack rows; `command` refuses them, this in case one comes another
    // way).
    // (Anti Form too: none of KH1's specials.)
    // (Every Drive Form: no KH1 limits in any, the user: "sora should not be able to use limits in
    // ANY drive form including anti form"; Wisdom's spells and items still run, as before. KH1's
    // limits don't come through here, their module moves him: `command` refuses them, but in
    // Limit Form, from its Limit menu.)
    let limit = super::form::in_form() && !matches!(command, 1 | 2 | ITEM_COMMAND_TYPE);
    if actor == sora && command != 1 && (super::form::two_blades() || super::form::in_anti() || limit) {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 30 {
            log(&format!("KH1 attack row {row} for command {command:#x} refused: not in a Drive Form"));
        }
        let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
        sound(4);
        return;
    }
    let f: ExecFn = std::mem::transmute(ORIG_EXEC.load(Ordering::Acquire));
    f(actor, row, dir)
}

/// The motion KH1's own swing would have started on Sora (the row's, `0x1402A17F0` ->
/// `0x14029D7F0(actor, motion | 0x1000000)`, which his `+0x164` then holds) and when, in ms.
/// Event scripts read `+0x164` (opcode 322): Dive to the Heart's "Press to attack" window waits
/// for an attack motion there, which KH2's swing never sets (the user's softlock, Steam and
/// Epic alike). `attack_seen` gives the script that motion for a moment after a KH2 attack.
static ATTACK_SHOWN: std::sync::Mutex<Option<(u32, std::time::Instant)>> = std::sync::Mutex::new(None);

fn attack_shown(motion: u32) {
    if let Ok(mut a) = ATTACK_SHOWN.lock() {
        *a = Some((motion, std::time::Instant::now()));
    }
}

/// The motion an event script should see on Sora instead of his `+0x164` (KH2 attacked within
/// the last 0.6 s: about one KH1 swing).
pub fn attack_seen() -> Option<u32> {
    let a = ATTACK_SHOWN.lock().ok()?;
    let (m, t) = (*a)?;
    (t.elapsed().as_secs_f32() < 0.6).then_some(m)
}

/// A command chosen while KH2's move couldn't take it (`command`): KH1's command record
/// (`COMMAND_RVA`, 16 bytes), when, and the KH2 move it was chosen in (`u32::MAX`: during KH1's
/// cast); given back to KH1 as soon as Sora can act (`resume_command`). It lives only as long
/// as that move (KH2's own buffer: into the move's next combo window or not at all; the user:
/// a queue of its own let magic come out however long after, apart from the combo string).
static PENDING_COMMAND: std::sync::Mutex<Option<([u8; 16], std::time::Instant, u32)>> = std::sync::Mutex::new(None);

/// The command waiting (`PENDING_COMMAND`) is a KH1 spell chosen during base Sora's KH1 cast,
/// held as KH1's own command record holds it (`kh1_spell_queues`): it waits for the cast to be
/// cut or to end, not the 0.2 s of a command taken in a cut already open.
static KH1_SPELL_QUEUED: AtomicBool = AtomicBool::new(false);

/// The longest a spell chosen during base Sora's KH1 cast waits for it (KH1's longest cast).
const KH1_SPELL_QUEUE_SECS: f32 = 3.0;

/// A spell `arg` chosen now, during one of base Sora's KH1 casts, is kept for when the cast can
/// be cut or ends, as vanilla KH1 keeps it (its menu takes a plain spell while he can't act and
/// its command step runs it once he can): pressing Fire again during Fire casts it again (the
/// player: "the fire ability from KH1 only shoots one fireball when pressing the shortcut multiple
/// times; before, it shot multiple fireballs"). s249's no-queue rule (a spell taken during
/// KH1's cast only while its cut is open, the user: no buffers) had dropped every press before
/// that: grounded Fire's cut opens at its frame 49 (`takeover::FIRE_CUT`) and the combo's last
/// spell has none, so mashing gave one fireball. Only KH1's own spells (Fire .. Aero) on base
/// Sora's KH1 casts: a Drive Form's spells and KH2's (the Magic style at KH2, Reflect) keep KH2's
/// no-buffer rule.
fn kh1_spell_queues(arg: u8) -> bool {
    arg <= 6
        && !super::form::in_form()
        && !super::formmagic::BASE_KH1_CAST.load(Ordering::Relaxed)
        && takeover::kh1_spell_now().is_some()
}

/// Keeps KH1's command record (a spell) for after the cast (`kh1_spell_queues`, or a cut open
/// now): it runs from `resume_command` once KH2 takes him back.
unsafe fn queue_spell(base: usize, kh1_queue: bool) {
    if let Ok(mut p) = PENDING_COMMAND.lock() {
        *p = Some((rd::<[u8; 16]>(base + COMMAND_RVA), std::time::Instant::now(), u32::MAX));
    }
    KH1_SPELL_QUEUED.store(kh1_queue, Ordering::Relaxed);
}

/// A spell is waiting in the queue for KH2's move to take it (`takeover`).
pub fn spell_waiting() -> bool {
    PENDING_COMMAND.try_lock().map_or(false, |p| p.is_some_and(|(r, _, _)| r[0] == 2))
}

/// Before KH1's control step: a waiting command goes back into KH1's command record once KH2's
/// move allows it (KH1's control step then runs it as if just chosen).
pub unsafe fn resume_command() {
    let Ok(mut p) = PENDING_COMMAND.lock() else { return };
    let Some((rec, t, motion)) = *p else { return };
    let kind = rec[0];
    // (Chosen during KH1's cast: it waits for that to be cut or to end, as the combo's next
    // step.)
    if takeover::kh1_casting() {
        // (A frame or two at most now: it is only stored while the cut is open, s249; a KH1
        // spell chosen during base Sora's KH1 cast waits for the cast, `kh1_spell_queues`.)
        let wait = if KH1_SPELL_QUEUED.load(Ordering::Relaxed) { KH1_SPELL_QUEUE_SECS } else { 0.2 };
        if t.elapsed().as_secs_f32() > wait {
            *p = None;
            cmd_log(format!("command {kind:#x} dropped: KH1's cast wasn't cut"));
        }
        return;
    }
    // Chosen during a KH2 move: only while that move plays.
    if motion != u32::MAX && !takeover::same_move(motion) {
        *p = None;
        cmd_log(format!("command {kind:#x} dropped: the move it was chosen in (motion {motion}) ended without a window for it"));
        return;
    }
    // (Chosen in KH1's jump or glide, `stylejump`: KH2 is taking him for it.)
    if super::stylejump::active() || super::styleroll::active() {
        if t.elapsed().as_secs_f32() > 0.2 {
            *p = None;
            cmd_log(format!("command {kind:#x} dropped: KH1's jump, glide or roll didn't let him go"));
        }
        return;
    }
    if !takeover::kh2_has() {
        *p = None;
        cmd_log(format!("command {kind:#x} dropped: KH2 no longer has Sora"));
        return;
    }
    let base = BASE.load(Ordering::Relaxed);
    if rd::<u8>(base + COMMAND_RVA) != 0 {
        // A new choice replaces it.
        *p = None;
        cmd_log(format!("command {kind:#x} dropped: another command was chosen"));
        return;
    }
    let ready = takeover::with_kh2(|c, _| if kind == 2 { c.can_cast() } else { c.can_take_command() });
    if kind == 2 && ready == Some(false) && takeover::with_kh2(|c, _| c.magic_later()) == Some(false) {
        *p = None;
        cmd_log(format!("command {kind:#x} dropped: the move playing has no window left for it"));
        return;
    }
    if ready == Some(true) {
        *p = None;
        // The menu step's own flags this frame (`0x140291660`) kept; the choice put back.
        let flags = rd::<u32>(base + COMMAND_RVA) & 0xFF00_0000 & !0x0100_0000;
        let mut r = rec;
        let v = u32::from_le_bytes([r[0], r[1], r[2], r[3]]) | flags;
        r[..4].copy_from_slice(&v.to_le_bytes());
        wr(base + COMMAND_RVA, r);
        cmd_log(format!("command {kind:#x}: KH2's move allows it now ({:.2} s after it was chosen)", t.elapsed().as_secs_f32()));
    }
}

/// The battle menu's top entries (`0x140529614`, four command ids; KH1's menu fill `0x140292760`
/// puts the context command there through `0x140290E70 (slot, id)`: Talk in place of Attack
/// (slot 0), Open / Examine and the like in the Triangle slot (3)).
const MENU_ENTRIES_RVA: Rva = Rva(0x529614);

/// `kind` is the context command the menu offers now (Talk, Open, Examine...): KH1's field
/// interaction, not one of Sora's moves.
unsafe fn context_command(base: usize, kind: u8) -> bool {
    let e = rd::<[u8; 4]>(base + MENU_ENTRIES_RVA);
    let ctx = (e[0] != 1 && e[0] == kind) || (e[3] != 0 && e[3] == kind);
    if ctx {
        cmd_log(format!("command {kind:#x}: the menu's context command (entries {e:02x?}), allowed in any form"));
    }
    ctx
}

/// Command types that don't move Sora (`0x1402A1470`): the party's (`+0x98`'s), the menu's.
const NOT_SORAS: [u8; 21] = [0x1D, 0x1E, 0x46, 0x51, 0x54, 0x64, 0x4B, 0x4D, 0x4E, 0x52, 0x57, 0x58, 0x59, 0x5A, 0x5B, 0x5E, 0x5F, 0x60, 0x4F, 0x5C, 0x5D];

/// Phase 8: KH1's own commands stay KH1's. KH1's command step (`0x1402A5380`, from its think
/// step `0x1402A4090`) runs a chosen command: Attack (type 1) becomes KH2's attack (`exec`);
/// any other that moves Sora (magic is type 2, spell in bits 8..23; items, specials,
/// summons...) is KH1's from here (`takeover::hand_to_kh1`). KH1 picks the command's ground
/// or air version by its movement state, which stays the ground's while KH2 has him: in KH2's
/// air, Sora is first put in KH1's jump state as falling from where he is (the fields KH1's
/// own ceiling bonk `0x1402AD260` sets: rise time `+0x3B8` 0, time `+0x3B0` 0, height `+0x3BC`
/// 0, base `+0x3B4` his height), so the air version plays and KH1 lands him.
unsafe extern "system" fn command(actor: usize, dir: usize) {
    let base = BASE.load(Ordering::Relaxed);
    let f: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_COMMAND.load(Ordering::Acquire));
    let sora = rd::<u64>(base + PARTY_RVA) as usize;
    let mut kind = rd::<u8>(base + COMMAND_RVA);
    // In the air KH1 allows only magic, kind 99 and kind 0x47 (`0x1402912F0`); KH1 judges the
    // air by his flags (`0x1402A0D30`), which stay the ground's while KH2 has him, and also
    // while KH1 has him falling from one of our air casts (its fall state 2, which we set), so
    // Sonic Blade and the like came out mid-jump (the user). Refused as KH1 refuses (magic of
    // every kind stays: jump casts are wanted).
    let mut full = rd::<u32>(base + COMMAND_RVA);
    // Our Reflect (magic, spell byte 7, which KH1 has no spell for): KH2's, never KH1's.
    // A Drive form (page 2's Drive list, `drive`).
    let mut arg = ((full >> 8) & 0xFF) as u8;
    // Limit Form's Limit menu (or a shortcut) chose one of KH1's limits (`limits::fill_kh1`: our
    // record, magic with the limit's argument): the command becomes KH1's own limit command (its
    // type, argument 0, the step's flags kept), which KH1's command step sends to its limit start
    // as from its Triangle box (`0x1402A1470` → `0x14028B240`; that start checks no ability, the
    // equipped test is only in the offer). It goes on below as KH1's limit does in Limit Form.
    if actor == sora && kind == 2 {
        if let Some(k) = super::limits::kh1_menu_arg(arg) {
            let l = &super::limits::KH1_LIMITS[k];
            if !super::form::in_limit() {
                cmd_log(format!("command {kind:#x} ({full:08x}, KH1's {} from the Limit menu) refused: not in Limit Form", l.name));
                let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
                sound(4);
                return;
            }
            wr(base + COMMAND_RVA, l.command);
            wr(base + COMMAND_RVA + 1, 0u8);
            cmd_log(format!("command {kind:#x} ({full:08x}): KH1's {} from Limit Form's Limit menu, as KH1's command {:#x}", l.name, l.command));
            kind = l.command;
            full = rd::<u32>(base + COMMAND_RVA);
            arg = 0;
        }
    }
    // Anti Form: only Attack (the user: "I want 1:1 combat parity"; KH2's Anti can't use magic,
    // items, summons, Limits or Revert): magic (Reflect and L1's shortcuts too), items, KH1's
    // specials, summons and the Drive list (its Revert: "I dont want sora to be able to revert
    // out of it") refused with KH1's "can't", as Valor refuses its own. The party's commands
    // and the field's context commands (talking, examining) stay, as in every form (the user,
    // "please let sora be allowed to talk to people and interact with objects in drive forms").
    if actor == sora && super::form::in_anti() && !matches!(kind, 0 | 1) && !NOT_SORAS.contains(&kind) && !context_command(base, kind) {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 30 {
            log(&format!("command {kind:#x} ({full:08x}) refused: not in Anti Form"));
        }
        let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
        sound(4);
        return;
    }
    // KH1 has him for its jump or glide (`stylejump`): a command (magic, an item, a Drive) is
    // KH2's, after KH2 takes him back (it waits for that, `resume_command`).
    // (KH1's roll, `styleroll`, too: KH2's once the roll is over.)
    if actor == sora && !matches!(kind, 0 | 1) && super::settings::get().commands && takeover::ON.load(Ordering::Relaxed) && (super::stylejump::active() || super::styleroll::active()) {
        // (No queue, s249: only when KH1 lets him act on this frame, KH2 taking him now; else
        // refused, the menu's "can't".)
        let can: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + CAN_ACT_RVA);
        if can(actor) == 0 {
            let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
            sound(4);
            cmd_log(format!("command {kind:#x} ({full:08x}) refused: KH1's jump, glide or roll has him and he can't act now (no queue)"));
            return;
        }
        if let Ok(mut p) = PENDING_COMMAND.lock() {
            *p = Some((rd::<[u8; 16]>(base + COMMAND_RVA), std::time::Instant::now(), u32::MAX));
        }
        if super::styleroll::active() {
            super::styleroll::want_kh2();
        } else {
            super::stylejump::want_kh2();
        }
        cmd_log(format!("command {kind:#x} ({full:08x}) waits: KH1's jump or glide has him, KH2 takes him for it"));
        return;
    }
    // KH1's own limits (Sonic Blade, Ars Arcanum, Strike Raid, Ragnarok, Trinity Limit) and their
    // follow-ups in a Drive Form (`limits::kh1_limit`). They move Sora through KH1's limit module,
    // not through this step (`NOT_SORAS` has them, so the form refusal below never caught them:
    // the offer's skip, `limits::specials_prompt`, kept them out). None in any form (the user:
    // "No drive form should be able to use KH1 Limits"), but in Limit Form all five, from its
    // Limit menu and shortcuts (the user: "Limit form should have all the KH1 limits without
    // having to have the abilities equipped"): Sonic Blade, Ars Arcanum, Strike Raid, Ragnarok,
    // Trinity Limit. KH1 runs them as its own (its limit, its MP cost) and has Sora for them: a
    // KH1 command's hand-off (`takeover`), kept all through the limit, the form staying on.
    if actor == sora && super::form::in_form() {
        if let Some((k, follow_up)) = super::limits::kh1_limit(kind) {
            let name = super::limits::KH1_LIMITS[k].name;
            let refuse = |why: &str| {
                cmd_log(format!("command {kind:#x} ({full:08x}, KH1's {name}{}) refused: {why}", if follow_up { "'s follow-up" } else { "" }));
                let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
                sound(4);
            };
            if !super::limits::kh1_limit_allowed(k) {
                refuse("no KH1 limits in a Drive Form but Limit Form");
                return;
            }
            // A follow-up: KH1's limit has him, its own (nothing of ours in between).
            if follow_up {
                cmd_log(format!("command {kind:#x} ({full:08x}): KH1's {name} follow-up, KH1's own (limit running {})", super::limits::kh1_limit_running()));
                return f(actor, dir);
            }
            // The start: only as KH1 could start it (on the ground, which KH2's air reads as for
            // KH1, a target, Trinity's party, the MP: `limits::kh1_usable`, what greys its menu
            // row), and when KH2's move can take a command (no queue, as every command here).
            if let Err(why) = super::limits::kh1_usable(actor, k) {
                refuse(why);
                return;
            }
            if takeover::with_kh2(|c, _| c.can_take_command()) == Some(false) {
                refuse(&format!("KH2's move (motion {}) can't be cancelled now (no queue)", takeover::KH2_MOTION.load(Ordering::Relaxed)));
                return;
            }
            trail_off(actor);
            f(actor, dir);
            if super::limits::kh1_limit_running() {
                // (As a KH1 command's hand-off below: no spell, no combo slot, KH2 back in its
                // battle stance after it.)
                takeover::SPELL_SLOT.store(0, Ordering::Relaxed);
                takeover::FROM_COMMAND.store(true, Ordering::Relaxed);
                takeover::set_air_cast(None);
                takeover::ITEM_COMMAND.store(false, Ordering::Relaxed);
                takeover::command_started(sora_shots(actor), None);
                takeover::hand_to_kh1(&format!("KH1's {name} in Limit Form (command {kind:#x}, {full:08x}): KH1's limit, the form stays on"));
            } else {
                cmd_log(format!("command {kind:#x} ({full:08x}): KH1's {name} didn't start (KH1's limit flag not set); KH2 keeps him"));
            }
            return;
        }
    }
    // Limit Form's limits and their Triangle follow-ups (`limits`: KH2's, its Limit menu).
    if actor == sora && kind == 2 && super::limits::ours(arg) {
        super::limits::command(actor, arg);
        return;
    }
    if actor == sora && kind == 2 && super::drive::on() && (super::drive::FORM_ARG..super::drive::FORM_ARG + super::drive::FORMS.len() as u8).contains(&arg) {
        super::drive::chosen(actor, (arg - super::drive::FORM_ARG) as usize);
        return;
    }
    // Fire, Blizzard, Thunder in Wisdom and Final Form: KH2's, on the form's cast motions
    // (`formmagic`; Cure is KH1's there); base Sora's with the Magic style at KH2.
    if actor == sora && kind == 2 && super::formmagic::spell_chosen(actor, arg) {
        return;
    }
    if actor == sora && kind == 2 && ((full >> 8) & 0xFF) as u8 == super::reflect::SPELL {
        super::reflect::chosen(actor);
        return;
    }
    // A spell chosen while KH1 casts one: it waits for the cast to be cut (once out) or to end,
    // then runs as the combo's next step (`takeover`'s MAGIC_AFTER; spec: magic into magic).
    if actor == sora && kind == 2 && super::settings::get().commands && takeover::ON.load(Ordering::Relaxed) && takeover::kh1_casting() {
        // (No queue, s249: taken only while KH1's spell can be cut now; else refused. But base
        // Sora's KH1 spells queue as in KH1, `kh1_spell_queues`.)
        let open = takeover::SPELL_CUT_OPEN.load(Ordering::Relaxed);
        let kh1_queue = !open && kh1_spell_queues(arg);
        if !open && !kh1_queue {
            let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
            sound(4);
            cmd_log(format!("command {kind:#x} ({full:08x}) refused: KH1's spell can't be cut yet (no queue)"));
            return;
        }
        queue_spell(base, kh1_queue);
        takeover::MAGIC_AFTER.store(true, Ordering::Relaxed);
        cmd_log(format!("command {kind:#x} ({full:08x}) waits: KH1's spell is still being cast{}", if kh1_queue { " (KH1's own spell queue)" } else { "" }));
        return;
    }
    // Valor Form: none of KH1's specials (Sonic Blade, Strike Raid, ...: the user, "dont let
    // sora use triangle moves (sonic blade, strike raid etc) in valor form"). Every command of
    // Sora's own that KH1 runs as a move (`0x1402A1470`) other than Attack, magic (refused in
    // Valor by the menu, `spell_not_yet`) and an item. (Type 99 too, which s222 let through: the
    // command step runs it as attack row 0x3E, `0x1402A4760`; the user, s229: "valor form can
    // still use strike raid, ragnarok".)
    // (Not the field's context commands, talking and examining: the user, "please let sora be
    // allowed to talk to people and interact with objects in drive forms"; the log: Talk, command
    // 0x4, refused in Final Form.)
    // (In every Drive Form now, the user: "sora should not be able to use limits in ANY drive form
    // including anti form". KH1's limits themselves are refused above, `limits::kh1_limit`, but
    // in Limit Form (its Limit menu); this is every other command of Sora's that moves him.)
    if actor == sora && super::form::in_form() && !matches!(kind, 0 | 1 | 2 | ITEM_COMMAND_TYPE) && !NOT_SORAS.contains(&kind) && !context_command(base, kind) {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 30 {
            log(&format!("command {kind:#x} ({full:08x}) refused: no limits in a Drive Form"));
        }
        let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
        sound(4);
        return;
    }
    let ours = actor == sora && super::settings::get().commands && takeover::ON.load(Ordering::Relaxed);
    let air = takeover::kh2_airborne();
    if ours && !matches!(kind, 0 | 1 | 2 | 99 | 0x47) && (air || rd::<i32>(actor + 0x70) == 2) {
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 20 {
            log(&format!("command {kind:#x} ({full:08x}) refused in the air, as KH1 would"));
        }
        let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
        sound(4);
        return;
    }
    // (Attack is KH2's attack button, unless the combo there is KH1's: then it goes to KH1 as
    // the other commands do, `stylecombo`.)
    let kh1_combo = kind == 1 && super::stylecombo::kh1_here(air);
    if actor != sora || kind == 0 || kind == 1 && !kh1_combo || NOT_SORAS.contains(&kind) || !super::settings::get().commands || !takeover::ON.load(Ordering::Relaxed) || !takeover::kh2_has() {
        return f(actor, dir);
    }
    // KH2 runs a command only when Sora can act: a spell when free, landing or in an attack's
    // "combo magic" window (`can_cast`), anything else when free or in a combo window
    // (`can_take_command`). Chosen mid-move, it waits and runs as soon as he can (the user: magic
    // cancelled any KH2 move).
    let ready = takeover::with_kh2(|c, _| if kind == 2 { c.can_cast() } else { c.can_take_command() }).unwrap_or(true);
    // A spell no window of the move playing can take (a finisher's, the combo's last): not
    // kept for after it (the user: "regardless of if a spell is inputted during a finisher it
    // should not be allowed to execute ... there shouldn't be any buffer system like that for
    // magic").
    if !ready && kind == 2 && takeover::with_kh2(|c, _| c.magic_later()) == Some(false) {
        cmd_log(format!("command {kind:#x} ({full:08x}) refused: the move playing has no window for a spell"));
        let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
        sound(4);
        return;
    }
    if !ready {
        let motion = takeover::KH2_MOTION.load(Ordering::Relaxed);
        // (No queue, s249, the user: "comment out any code that is related to queueing/buffering
        // moves". It waited for the move's next window:)
        // if let Ok(mut p) = PENDING_COMMAND.lock() {
        //     *p = Some((rd::<[u8; 16]>(base + COMMAND_RVA), std::time::Instant::now(), motion));
        // }
        let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
        sound(4);
        cmd_log(format!("command {kind:#x} ({full:08x}) refused: KH2's move (motion {motion}) can't be cancelled now (no queue)"));
        return;
    }
    static AIR_LOG: AtomicU32 = AtomicU32::new(0);
    if AIR_LOG.fetch_add(1, Ordering::Relaxed) < 40 {
        log(&format!("command {kind:#x}: KH2 has him {}", if air { "in the air" } else { "on the ground" }));
    }
    trail_off(actor);
    let before = (rd::<i32>(actor + 0x70), rd::<u32>(actor + 0x164));
    if air {
        let set_state: unsafe extern "system" fn(usize, u32) -> u32 = std::mem::transmute(base + STATE_RVA);
        let y = rd::<f32>(actor + 0x14);
        set_state(actor, 2);
        wr(actor + 0x3B0, 0.0f32);
        wr(actor + 0x3B8, 0.0f32);
        wr(actor + 0x3BC, 0.0f32);
        wr(actor + 0x3B4, y);
        // KH2's jump's sideways speed carried into KH1's air state (`+0x3C0`: x, y, z, its
        // length; KH1's command step then slows it as for its own air casts), the user: "when sora
        // does aerial magic the kh1 animation currently does not carry momentum from kh2s jump".
        let [vx, vz] = takeover::kh2_sideways();
        let n = (vx * vx + vz * vz).sqrt();
        let k = if n > 30.0 { 30.0 / n } else { 1.0 };
        wr(actor + 0x3C0, [vx * k, 0.0f32, vz * k, n * k]);
        cmd_log(format!("air command: KH2's sideways speed {:.1}/frame carried into KH1's air state", n * k));
        wr(actor + 0x3D0, 0u32);
        wr(actor, rd::<u32>(actor) & !0x1_0000);
    }
    // What KH1 doing something with him changes: his state, motion request, the script-held
    // flag 0x1000000 (spells and items set it, the user's log), his motion slot.
    // ... and above all any motion KH1 sets on him (`probe`'s set-motion hooks count them: the
    // air cast's `0x14029D7F0(actor, 0x83 + spell)` changed none of the others, the user's log).
    let look = || (rd::<i32>(actor + 0x70), rd::<u32>(actor + 0x164), rd::<u32>(actor) & 0x100_0000, pose::sora_slot(), super::probe::SORA_MOTIONS.load(Ordering::Relaxed));
    let set = look();
    super::stylecombo::starting(kh1_combo);
    f(actor, dir);
    super::stylecombo::starting(false);
    // Only if KH1 did something with him (a new state or motion request); else as before.
    let after = look();
    if after != set {
        // A spell mid-combo takes a step of it, and the combo goes on after it (the user: magic
        // cast mid-combo inserts itself in the combo, as a modifier does).
        // A spell takes its slot of the combo string as KH2's casts do (spec: one slot each).
        let slot = if kind == 2 {
            let s = takeover::with_kh2(|c, _| (c.magic_step(), c.combo_state()));
            let code = match s.map(|x| x.0) {
                Some(Some(character_core::SpellSlot::Step)) => 1,
                Some(Some(character_core::SpellSlot::Finisher { more: true })) => 2,
                _ => 3,
            };
            log(&format!("spell cast: combo slot {code} ({:?}; 1 a step, 2 a finisher with another to come, 3 the combo's last)", s));
            code
        } else {
            0
        };
        takeover::SPELL_SLOT.store(slot, Ordering::Relaxed);
        takeover::FROM_COMMAND.store(true, Ordering::Relaxed);
        // (A spell in the air: its index in the command's second byte (Fire 0 .. Aero 6).)
        takeover::set_air_cast((air && kind == 2).then_some(((full >> 8) & 0xFF) as u8));
        // (Items are command type 3, the item in the second byte: `takeover`'s item end frame.)
        takeover::ITEM_COMMAND.store(kind == ITEM_COMMAND_TYPE, Ordering::Relaxed);
        takeover::command_started(sora_shots(actor), (kind == 2).then_some(((full >> 8) & 0xFF) as u8));
        takeover::hand_to_kh1(&format!("command {kind:#x} ({full:08x}){}, KH1 motion {:#x}", if air { " in the air" } else { "" }, super::probe::SORA_LAST_SLOT.load(Ordering::Relaxed)));
        if kh1_combo {
            super::stylecombo::started();
            log(&format!("KH1's own {} combo (move style KH1)", if air { "aerial" } else { "ground" }));
        }
    } else {
        if air {
            wr(actor + 0x70, before.0);
        }
        note(format!("command {kind:#x} ({full:08x}){}: KH1 did nothing with Sora, KH2 keeps him", if air { " in the air" } else { "" }));
    }
}

/// During KH1's cast: an Attack (1) or a spell (2) just chosen in the command menu, taken off
/// KH1's command record (KH1 runs none while it casts); a spell goes to the queue
/// (`resume_command`) to run once KH2 has Sora.
pub unsafe fn take_chosen_command() -> Option<u8> {
    let base = BASE.load(Ordering::Relaxed);
    let kind = rd::<u8>(base + COMMAND_RVA);
    match kind {
        // (In KH1's own combo its presses stay in the record: KH1's command step chains them.)
        1 if super::stylecombo::active() => None,
        1 => {
            wr(base + COMMAND_RVA, 0u8);
            cmd_log("Attack chosen during KH1's cast: KH2's once the spell is out".to_string());
            Some(1)
        }
        2 if ((rd::<u32>(base + COMMAND_RVA) >> 8) & 0xFF) as u8 >= super::drive::FORM_ARG && super::drive::on() => {
            // A Drive form chosen in KH1's move's cut window (`takeover::kh1_cut_window`): the
            // form's, KH2 takes him to change (`form::choose` sets `takeover::DRIVE_CUT`).
            let arg = ((rd::<u32>(base + COMMAND_RVA) >> 8) & 0xFF) as u8;
            wr(base + COMMAND_RVA, 0u8);
            let sora = rd::<u64>(base + PARTY_RVA) as usize;
            super::drive::chosen(sora, (arg - super::drive::FORM_ARG) as usize);
            cmd_log("Drive chosen during KH1's move".to_string());
            None
        }
        2 if takeover::SPELL_CUT_OPEN.load(Ordering::Relaxed) => {
            queue_spell(base, false);
            wr(base + COMMAND_RVA, 0u8);
            cmd_log("spell chosen during KH1's cast while it can be cut: cuts it now".to_string());
            Some(2)
        }
        // Base Sora's KH1 spell chosen during his KH1 cast: kept for when the cast can be cut or
        // ends, as KH1 keeps it (`kh1_spell_queues`; Fire again and again).
        2 if kh1_spell_queues(((rd::<u32>(base + COMMAND_RVA) >> 8) & 0xFF) as u8) => {
            queue_spell(base, true);
            wr(base + COMMAND_RVA, 0u8);
            cmd_log("KH1 spell chosen during base Sora's KH1 cast: waits for the cast to be cut or to end (KH1's own spell queue)".to_string());
            Some(2)
        }
        // (No queue, s249: before KH1's spell can be cut, dropped.)
        2 => {
            wr(base + COMMAND_RVA, 0u8);
            cmd_log("spell chosen during KH1's cast before it can be cut: dropped (no queue)".to_string());
            None
        }
        _ => None,
    }
}

/// The Attack command since the last call (KH2's attack button press).
pub fn take_press() -> bool {
    PRESSED.swap(false, Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug)]
struct Row {
    motion: i16,
    data: i32,
    /// Bytes +0xC..+0xE (the attack object's +0xA8..+0xAA: effect and sound).
    fx: [u8; 3],
    /// What the row's start plays (`0x1402A17F0`): +2 the weapon trail (`0x1402C0160`), +3 an
    /// effect on Sora (`0x1402ABB60`), +0xF a voice (`0x14028E060`); -1 none.
    trail: i8,
    effect: i8,
    voice: i8,
}

unsafe fn row_info(actor: usize, row: i32) -> Option<Row> {
    let base = BASE.load(Ordering::Relaxed);
    let obj = pose::ptr(rd::<u32>(actor + 0x130));
    if obj < 0x10000 || !(0..0x80).contains(&row) {
        return None;
    }
    let table = rd::<u64>(base + ROWS_RVA + rd::<u16>(obj + 0x4C) as usize * 8) as usize;
    if table < 0x10000 {
        return None;
    }
    let r = table + row as usize * 0x14;
    Some(Row { motion: rd(r), data: rd(r + 4), fx: rd(r + 0xC), trail: rd(r + 2), effect: rd(r + 3), voice: rd(r + 0xF) })
}

/// Sora's own rows to land KH2's hits with: [ground swing, ground finisher, air swing, air
/// finisher]: KH1's combo motions 200 / 202 on the ground, 204 / 205 in the air (the Attack
/// command's own picks in the user's log; 203's first row carries a much stronger attack).
unsafe fn sora_rows(sora: usize) -> Option<[Row; 4]> {
    let mut rows: Vec<Row> = Vec::new();
    let mut all = Vec::new();
    for i in 0..0x40 {
        let Some(r) = row_info(sora, i) else { break };
        if r.motion >= 0 {
            all.push(format!("{i}:{}/{:#x}/{:02x?}", r.motion, r.data, r.fx));
            rows.push(r);
        }
    }
    log(&format!("Sora's KH1 attack rows (row:motion/data/fx): {}", all.join(" ")));
    // The attack data the rows use (first words: strength source, element, reaction, ...).
    let base = BASE.load(Ordering::Relaxed);
    let mut seen = HashSet::new();
    let datas: Vec<String> = rows
        .iter()
        .filter(|r| seen.insert(r.data))
        .map(|r| format!("{:#x}: [{}]", r.data, (0..12).map(|k| format!("{:x}", rd::<u32>(base + ATTACK_DATA_RVA + r.data as usize + k * 4))).collect::<Vec<_>>().join(" ")))
        .collect();
    log(&format!("their attack data: {}", datas.join(" | ")));
    let first = |m: i16| rows.iter().find(|r| r.motion == m).copied();
    // The attack data's third word is KH1's reaction kind (into the attack's `+0x30` low bits;
    // 3 a flinch, 5 the finishers' knock-back, the user's stage 26 log). KH1's combo finishers
    // are motions 203 (ground) and 206 (air), both with a kind-5 data (0x4C58, +4 strength);
    // a finisher lands with the first kind-5 row of that motion.
    let kind = |r: &Row| rd::<u32>(base + ATTACK_DATA_RVA + r.data as usize + 8);
    let finisher = |m: i16| rows.iter().find(|r| r.motion == m && kind(r) == 5).copied().or_else(|| rows.iter().find(|r| (200..=206).contains(&r.motion) && kind(r) == 5).copied());
    let ground = first(200).or_else(|| first(201))?;
    let air = first(204).or(Some(ground))?;
    Some([ground, finisher(203).unwrap_or(ground), air, finisher(206).unwrap_or(air)])
}

/// One enemy's body part: KH1's entry for its contact code and the shape in KH2 space.
struct Part {
    entry: [u8; 0x40],
    shape: Shape,
}

struct Body {
    actor: usize,
    parts: Vec<Part>,
    target: TargetInfo,
    /// Its hit parts' top above its root.
    tall: f32,
    /// Too big to move: many parts, or its root far beyond its near parts (the user: Aerial
    /// Finish on Oogie's Manor's blobs pulled the whole house into the floor). Our pulls
    /// (Magnet, the Aerial Finish hold) leave it be.
    fixed: bool,
}

/// KH1's lock point `(actor, point, out) -> radius` (`docs/kh1-lockon.md`).
const LOCK_POINT_RVA: Rva = Rva(0x2AE330);

/// KH1's hittable actors on other teams than Sora's, near him.
unsafe fn bodies(sora: usize) -> Vec<Body> {
    let base = BASE.load(Ordering::Relaxed);
    let next: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(base + NEXT_ACTOR_RVA);
    let hittable: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(base + HITTABLE_RVA);
    let part_shape: unsafe extern "system" fn(usize, usize, *mut u32) = std::mem::transmute(base + PART_SHAPE_RVA);
    let team = |a: usize| (rd::<u32>(a + 0x374) >> 5) & 3;
    let ours = team(sora);
    let at = rd::<[f32; 3]>(sora + 0x10);
    let mut out = Vec::new();
    let mut a = next(0);
    let mut n = 0;
    while a != 0 && n < 256 {
        n += 1;
        let this = a;
        a = next(a);
        if this == sora || team(this) == ours || hittable(this) & 0xFF == 0 {
            continue;
        }
        let p = rd::<[f32; 3]>(this + 0x10);
        // (Near or not by its parts, below: a big one's root can be far from parts within
        // reach, the user: Oogie's Manor's glowing blobs, parts of the manor itself.)
        let col = pose::ptr(rd::<u32>(this + 0x134));
        if col < 0x10000 {
            continue;
        }
        let count = rd::<u8>(col + 6) as usize;
        let list = pose::ptr(rd::<u32>(col + 0x18));
        if list < 0x10000 {
            continue;
        }
        let off = rd::<u32>(this + 0x3A8);
        let feet = [p[0], -p[1], p[2]];
        let (mut radius, mut top) = (0.0f32, feet[1]);
        let mut parts = Vec::new();
        for i in 0..count.min(32) {
            if off & (1 << i) != 0 {
                continue;
            }
            let part = list + i * 0x14;
            let mut entry = [0u8; 0x40];
            let e = entry.as_mut_ptr() as usize;
            wr(e, team(this));
            wr(e + 8, this as u64);
            wr(e + 0x10, rd::<u8>(part) as u32);
            wr(e + 0x20, 1u32);
            part_shape(this, part, (e + 0x20) as *mut u32);
            let kind = rd::<u32>(e + 0x20);
            let (r, h) = (rd::<f32>(e + 0x24), rd::<f32>(e + 0x28));
            let c = rd::<[f32; 3]>(e + 0x30);
            if !(r > 0.0 && r < 2000.0 && h >= 0.0 && h < 2000.0 && c.iter().all(|v| v.is_finite())) {
                continue;
            }
            let centre = [c[0], -c[1], c[2]];
            let shape = if kind == 1 { Shape::sphere(centre, r) } else { Shape::column(centre, r, h.max(r * 0.25)) };
            radius = radius.max(r);
            top = top.max(centre[1] + h);
            parts.push(Part { entry, shape });
        }
        let near = |c: [f32; 3]| {
            let (dx, dz) = (c[0] - at[0], c[2] - at[2]);
            dx * dx + dz * dz <= NEAR * NEAR
        };
        if parts.is_empty() || !parts.iter().any(|q| near([q.shape.center[0], 0.0, q.shape.center[2]])) {
            continue;
        }
        let tall = (top - feet[1]).max(radius);
        let root_far = (p[0] - at[0]).powi(2) + (p[2] - at[2]).powi(2) > NEAR * NEAR;
        // Or not an enemy at all (KH1 actor kind 3 are the fighters; attackable objects are
        // others): the user, nothing immovable is pulled or thrown.
        // Once big, always (the user: Dragon Maleficent was pulled into a wall: she is 615 tall
        // standing and under 600 with her head down); kept by actor and its object id.
        static BIG: std::sync::Mutex<Vec<(usize, u32)>> = std::sync::Mutex::new(Vec::new());
        let key = (this, super::probe::object_id(this));
        let big_now = count > 8 || tall > 600.0 || radius > 300.0;
        let big = BIG.lock().map_or(big_now, |mut v| {
            if big_now && !v.contains(&key) {
                v.retain(|(a, _)| *a != this);
                v.push(key);
            }
            v.contains(&key)
        });
        let fixed = big || root_far || rd::<u8>(this + 6) != 3;
        if fixed {
            static SEEN: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());
            if let Ok(mut v) = SEEN.lock() {
                if !v.contains(&this) && v.len() < 20 {
                    v.push(this);
                    log(&format!("big target {this:#x} (object {}, not pulled): {} of {count} parts hittable, {tall:.0} tall, radius {radius:.0}, root {:.0} away", super::probe::object_id(this), parts.len(), ((p[0] - at[0]).powi(2) + (p[2] - at[2]).powi(2)).sqrt()));
                }
            }
        }
        let mut target = TargetInfo { id: this as u32, pos: feet, radius, height: tall };
        // KH2's combo table measures a target by its target shape (`0x1403C8EE0`: its point
        // and size), not its feet and full height; KH1's lock point is that shape (the user:
        // Aerial Sweep / Aerial Dive, which want the target well above Sora, hardly came out
        // when a flying foe counted from its feet with its whole height).
        let point: unsafe extern "system" fn(usize, i32, *mut [f32; 4]) -> f32 = std::mem::transmute(base + LOCK_POINT_RVA);
        let mut v = [0.0f32; 4];
        let r = point(this, 0, &mut v);
        if r > 0.0 && r < 1000.0 && v[..3].iter().all(|x| x.is_finite()) {
            target.pos = [v[0], -v[1], v[2]];
            target.height = r.max(10.0);
        }
        out.push(Body { actor: this, parts, target, tall, fixed });
    }
    out
}

/// KH2's hit detection and hit records for Sora's swings.
#[derive(Default)]
pub struct Strikes {
    volumes: HashMap<(u64, u8), AttackVolume>,
    records: HashMap<usize, HitRecord>,
    live: HashSet<u64>,
    rows: Option<[Row; 4]>,
    rows_tried: bool,
    bodies: Vec<Body>,
    guards: Vec<Guard>,
    /// Swings a guard repelled: they hit nothing more.
    repelled: HashSet<u64>,
    /// (swing, weapon) that met a wall and showed KH1's wall spark (`walls = kh1`): once a swing.
    sparked: HashSet<(u64, u8)>,
    /// KH1's swing rows by motion (200..206), for `swing_start`, and the model they are for.
    swing_rows: Option<(u16, HashMap<i16, Row>)>,
    /// Frames since a trail was asked for.
    trail_asked: Option<u32>,
    /// The trail this swing asked for (KH1's row's), and whether its hit window was open last
    /// frame: asked again as the window opens if KH1 dropped it meanwhile (a long wind-up).
    trail_id: Option<u32>,
    was_swinging: bool,
    /// Enemies a Magnet (ATKP reaction 5: Magnet Burst) caught: the swing, orbit radius and
    /// swirl. Pulled every frame while the swing lasts.
    pulled: HashMap<usize, (u64, f32, f32)>,
    /// Enemies a holding blow (Aerial Finish's spin) hit: frames left to keep them beside Sora.
    held: HashMap<usize, u32>,
    /// (swing, shot) pairs already met (once a swing).
    met_shots: HashSet<(u64, usize)>,
    /// (Reflect barrier, enemy attack) pairs already met (`barrier_meet`), by barrier key.
    barrier_met: HashSet<(u64, usize)>,
    shots: Vec<Shot>,
    /// KH1's Keyblade as drawn last frame (`blade_points`, KH1 world): the swing's clash sweep
    /// runs from there to now.
    blade_prev: Option<Vec<[f32; 3]>>,
    /// KH2's live guard windows as KH1 guard attack objects (`guard_step`): by (window, shape),
    /// the object and the clashed-with mask seen last frame.
    guard_objs: HashMap<(u64, usize), (usize, u32)>,
    /// Our two copies of a swing's attack data for them (front-only, all round), made once.
    guard_mem: Option<(usize, u32, u32)>,
}

/// KH1's point along an actor's bone `(actor, bone, t, out)` (`0x14029C990`): for bone 0x30 (the
/// Keyblade) `t` runs along the blade between the weapon's own start and end (its weapon stats
/// `+0x28` / `+0x24`: each Keyblade's length), as KH1's own swing places its attack object.
const BONE_POINT_RVA: Rva = Rva(0x29C990);
/// KH1's blade bone id, and the points taken along it for the clash sweep.
const BLADE_BONE: u32 = 0x30;
const BLADE_POINTS: usize = 6;

/// Sora's KH1 Keyblade as drawn (KH1 world, y down): `BLADE_POINTS` points from the blade's start
/// to its tip (KH1's own blade range for this Keyblade, so a longer one reaches farther), and the
/// radius KH1's swing of `data` has with this weapon (the data's `+0x5C` plus the weapon's radius
/// bonus × Sora's size, as `0x1402B10E0` makes it).
unsafe fn blade_points(sora: usize, data: usize) -> Option<(Vec<[f32; 3]>, f32)> {
    let base = BASE.load(Ordering::Relaxed);
    let wobj = pose::ptr(rd::<u32>(sora + 0x14C));
    if wobj < 0x10000 {
        return None;
    }
    let ws = rd::<u64>(wobj + 0x20) as usize;
    if ws < 0x10000 {
        return None;
    }
    let point: unsafe extern "system" fn(usize, u32, f32, *mut [f32; 4]) = std::mem::transmute(base + BONE_POINT_RVA);
    let mut pts = Vec::with_capacity(BLADE_POINTS);
    for i in 0..BLADE_POINTS {
        let mut out = [0.0f32; 4];
        point(sora, BLADE_BONE, i as f32 / (BLADE_POINTS - 1) as f32, &mut out);
        let p = [out[0], out[1], out[2]];
        if !p.iter().all(|v| v.is_finite()) {
            return None;
        }
        pts.push(p);
    }
    let radius = rd::<i32>(data + 0x5C) as f32 + rd::<f32>(sora + 0x40) * rd::<f32>(ws + 0x20);
    let radius = if radius.is_finite() { radius.clamp(5.0, 80.0) } else { 20.0 };
    static LOGGED: std::sync::Mutex<Option<usize>> = std::sync::Mutex::new(None);
    if let Ok(mut l) = LOGGED.lock() {
        if *l != Some(ws) {
            *l = Some(ws);
            let len = ((pts[BLADE_POINTS - 1][0] - pts[0][0]).powi(2) + (pts[BLADE_POINTS - 1][1] - pts[0][1]).powi(2) + (pts[BLADE_POINTS - 1][2] - pts[0][2]).powi(2)).sqrt();
            log(&format!(
                "clash blade: KH1's Keyblade from its start to its tip {len:.0} long (weapon blade range {:.2}..{:.2}, length factor {:.2}), radius {radius:.0}",
                rd::<f32>(ws + 0x28),
                rd::<f32>(ws + 0x24),
                rd::<f32>(ws + 0x2C)
            ));
        }
    }
    Some((pts, radius))
}

/// The blade's sweep since last frame as KH2 shapes (y up): last frame's points, the midpoints,
/// this frame's (in that order: earlier first), and each sphere's place along the blade (0 the
/// start, 1 the tip).
fn blade_sweep(prev: &[[f32; 3]], now: &[[f32; 3]], radius: f32) -> (Vec<Shape>, Vec<f32>) {
    let up = |p: [f32; 3]| [p[0], -p[1], p[2]];
    let n = now.len();
    let along = |i: usize| i as f32 / (n.max(2) - 1) as f32;
    let mut shapes = Vec::new();
    let mut at = Vec::new();
    if prev.len() == n {
        for (i, p) in prev.iter().enumerate() {
            shapes.push(Shape::sphere(up(*p), radius));
            at.push(along(i));
        }
        for (i, (p, q)) in prev.iter().zip(now).enumerate() {
            shapes.push(Shape::sphere(up([(p[0] + q[0]) * 0.5, (p[1] + q[1]) * 0.5, (p[2] + q[2]) * 0.5]), radius));
            at.push(along(i));
        }
    }
    for (i, p) in now.iter().enumerate() {
        shapes.push(Shape::sphere(up(*p), radius));
        at.push(along(i));
    }
    (shapes, at)
}

/// An enemy's shot Sora's swings can knock back (Wakka's ball): a free-standing KH1 attack
/// object of the attack pool (attribute `+0x30` bit `0x400`), `docs/kh1-reflect.md`.
struct Shot {
    obj: usize,
    /// Its swept path (previous and current centre) and radius, KH2 y-up.
    shapes: Vec<Shape>,
    owner: usize,
    /// Free-standing (a shot); else an enemy's own attack on its body (a melee swing), which
    /// only a Reflect barrier meets (`barrier_meet`).
    free: bool,
}

/// The enemies' shots near Sora that a swing may meet (and their live melee attacks, for a
/// Reflect barrier).
unsafe fn shots(sora: usize) -> Vec<Shot> {
    let base = BASE.load(Ordering::Relaxed);
    let team = |a: usize| (rd::<u32>(a + 0x374) >> 5) & 3;
    let ours = team(sora);
    let at = rd::<[f32; 3]>(sora + 0x10);
    let mut out = Vec::new();
    for k in 0..ATTACK_SLOTS {
        let g = base + ATTACK_POOL_RVA + k * 0xC0;
        let flags = rd::<u32>(g);
        // In use and active; not switched off, freed, or already turned back.
        if flags & 0x800 == 0 || flags & 0xC == 0 || flags & (0x400 | 0x0020_0000 | 0x0100_0000) != 0 {
            continue;
        }
        // (Every live enemy attack: KH1's collision pass meets each pair of attacks, and the
        // meeting itself decides what happens: a shot turned back, a clash, nothing. Guards,
        // priority 100, have their own path.)
        let prio = rd::<i32>(g + 0x68);
        if prio == GUARD_PRIORITY || prio < 0 {
            continue;
        }
        // KH1's pass (`0x1402B1A50`) leaves out attacks with word bit 0x400, and those with
        // bit 0x80 that already met something (`+4`).
        if flags & 0x400 != 0 || (flags & 0x80 != 0 && rd::<u32>(g + 4) != 0) {
            continue;
        }
        // Free-standing shots only (`+0x30` bit 0x400: a thrown ball, a fired projectile, Wakka's
        // ball among them): an enemy's melee swing that meets KH2's broad swings clashed and
        // clinked far more often than KH2 would (the user, twice). KH2's swing hits the body
        // instead.
        let free = rd::<u32>(g + 0x30) & 0x400 != 0;
        let owner = pose::ptr(rd::<u32>(g + 0x34));
        if owner == sora || (owner > 0x10000 && team(owner) == ours) {
            continue;
        }
        // Its carrier (`+0x90`) switched off (`+0x88` bit 1): only some types stay (`0x1402B2610`).
        let t = flags & 0xE0000;
        let carrier = pose::ptr(rd::<u32>(g + 0x90));
        if carrier > 0x10000 && rd::<u8>(carrier + 0x88) & 1 != 0 && rd::<u32>(g + 0x30) & 0x400 == 0 && (((t.wrapping_sub(0x20000)) & 0xfff9_ffff) != 0 || t == 0x60000) {
            continue;
        }
        // Tied to a motion (activity 2): live only while the owner plays it and within its hit
        // window (`0x1402B2610`: owner `+0x1CC` bit `0x10000 << group`), as KH1's pass has it
        // (the user: endless clinks against the enemies' resting attack shapes).
        if (flags >> 2) & 3 == 2 {
            if owner < 0x10000 {
                continue;
            }
            let req = rd::<u32>(g + 0x58);
            let window = 0x10000u32 << ((flags >> 4) & 3);
            if req == 0xFFF || req != rd::<u32>(owner + 0x164) || rd::<u32>(owner + 0x160) & 0x100 != 0 || rd::<u32>(owner + 0x1CC) & window == 0 {
                continue;
            }
        }
        let c = rd::<[f32; 3]>(g + 0x10);
        let prev = rd::<[f32; 3]>(g + 0x70);
        let r = rd::<f32>(g + 0xC);
        if !(r > 0.0 && r < 500.0) || !c.iter().chain(prev.iter()).all(|v| v.is_finite()) {
            continue;
        }
        let (dx, dz) = (c[0] - at[0], c[2] - at[2]);
        if dx * dx + dz * dz > NEAR * NEAR {
            continue;
        }
        let up = |p: [f32; 3]| [p[0], -p[1], p[2]];
        let mid = [(c[0] + prev[0]) * 0.5, (c[1] + prev[1]) * 0.5, (c[2] + prev[2]) * 0.5];
        let shapes = vec![Shape::sphere(up(c), r), Shape::sphere(up(mid), r), Shape::sphere(up(prev), r)];
        // What it is, once a slot and kind (to tell shots apart in the log).
        static SEEN: std::sync::Mutex<Vec<(usize, u32, u32)>> = std::sync::Mutex::new(Vec::new());
        if let Ok(mut seen) = SEEN.lock() {
            let key = (g, flags & 0xFFFF_F000, rd::<u32>(g + 0x30));
            if seen.len() < 200 && !seen.contains(&key) {
                seen.push(key);
                log(&format!(
                    "enemy attack near Sora: slot {k} flags {flags:08x} +4 {:08x} +0x30 {:08x} owner {owner:#x} priority {prio} radius {r:.0} at {c:.0?} (was {prev:.0?})",
                    rd::<u32>(g + 4),
                    rd::<u32>(g + 0x30)
                ));
            }
        }
        out.push(Shot { obj: g, shapes, owner, free });
    }
    out
}

/// A swing meets a shot as KH1's own swing would (`0x1402B2890`, the attack-vs-attack clash,
/// both ways): KH1 turns the shot back (team swapped, Sora its owner, flag `0x1000000`).
/// Returns whether it was turned back.
unsafe fn meet_shot(sora: usize, shot: &Shot, at: &Shape, row: Row) -> (bool, bool) {
    let base = BASE.load(Ordering::Relaxed);
    let free_bit: unsafe extern "system" fn() -> u32 = std::mem::transmute(base + FREE_BIT_RVA);
    let new_attack: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + NEW_ATTACK_RVA);
    let free: unsafe extern "system" fn(usize) = std::mem::transmute(base + FREE_ATTACK_RVA);
    let clash: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + CLASH_RVA);
    let obj = new_attack(sora, free_bit(), base + ATTACK_DATA_RVA + row.data as usize);
    if obj < 0x10000 {
        return (false, false);
    }
    // KH1's pairing table (`0x1405299B8`, by type `flags >> 17 & 7`): either way round.
    let ty = |o: usize| (rd::<u32>(o) >> 17) & 7;
    let table = |a: u32, b: u32| rd::<u8>(base + PAIRS_RVA + a as usize) & (1u8 << b) != 0;
    let (ts, to) = (ty(obj), ty(shot.obj));
    if !table(ts, to) && !table(to, ts) {
        free(obj);
        return (false, false);
    }
    let c = [at.center[0], -at.center[1], at.center[2], 1.0f32];
    wr(obj + 0x10, c);
    wr(obj + 0x40, c);
    wr(obj + 0x70, c);
    wr(obj + 0xC, at.radius);
    let entry = |o: usize, owner: usize, centre: [f32; 4], r: f32| {
        let mut e = [0u8; 0x40];
        let p = e.as_mut_ptr() as usize;
        wr(p, (rd::<u32>(o) >> 12) & 3);
        wr(p + 8, owner as u64);
        wr(p + 0x10, rd::<u32>(o + 0x54));
        wr(p + 0x18, o as u64);
        wr(p + 0x20, 1u32);
        wr(p + 0x24, r);
        wr(p + 0x28, r);
        wr(p + 0x30, centre);
        e
    };
    let sc = rd::<[f32; 3]>(shot.obj + 0x10);
    let swing = entry(obj, sora, c, at.radius);
    let them = entry(shot.obj, shot.owner, [sc[0], sc[1], sc[2], 1.0], rd::<f32>(shot.obj + 0xC));
    clash(swing.as_ptr() as usize, them.as_ptr() as usize);
    // (KH1's results on our side, `+4`: 0x10 / 0x20 a clash: the swing is stopped there, its
    // recoil a kind 6 / 7 record on Sora that `hurt` plays as KH2's.)
    let ours = rd::<u32>(obj + 4) & 0x30 != 0;
    free(obj);
    (rd::<u32>(shot.obj) & 0x0100_0000 != 0, ours)
}

/// A wall a blade met: the point (KH1 world), the face and its object (-1 the map), and the
/// face's normal as `0x14018FC60` gives it.
struct WallHit {
    at: [f32; 3],
    face: i32,
    owner: i32,
    normal: [f32; 4],
}

/// Where the map's walls stop a blade moving `from` → `to` (KH1 world, y down): KH1's
/// background ray (`0x140191520`, mode 1 as `0x1402B1870` casts it for a swing on a bone), and
/// the hit face's normal (`0x14018FC60`) to tell walls from floors and ceilings.
unsafe fn wall_between(from: [f32; 3], to: [f32; 3]) -> Option<WallHit> {
    let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if !(0.5..400.0).contains(&len) || !from.iter().chain(to.iter()).all(|v| v.is_finite()) {
        return None;
    }
    let base = BASE.load(Ordering::Relaxed);
    let ray: unsafe extern "system" fn(*mut [u8; 0x40], u32) = std::mem::transmute(base + RAY_RVA);
    let normal: unsafe extern "system" fn(i32, i32, *mut [f32; 4]) = std::mem::transmute(base + FACE_NORMAL_RVA);
    let mut r = [0u8; 0x40];
    let p = r.as_mut_ptr() as usize;
    wr(p, [from[0], from[1], from[2], 1.0f32]);
    wr(p + 0x10, [d[0], d[1], d[2], 0.0f32]);
    ray(&mut r, 1);
    let face = rd::<i16>(p + 0x30) as i32;
    let owner = rd::<i32>(p + 0x34);
    // The map's own faces only (`-1`): objects (crates, barrels) take the swing as a hit.
    if face < 0 || owner != -1 {
        return None;
    }
    let mut n = [0.0f32; 4];
    normal(owner, face, &mut n);
    let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if !(nl > 1e-3 && nl.is_finite()) || (n[1] / nl).abs() > 0.6 {
        return None;
    }
    let at = rd::<[f32; 3]>(p + 0x20);
    at.iter().all(|v| v.is_finite()).then_some(WallHit { at, face, owner, normal: n })
}

/// KH1's own swing meeting a wall (`0x1402B1870`), without the part that stops it: the
/// Keyblade's wall spark where the blade touched, turned out of the face, and its wall sound.
/// Not the clash record on Sora (kind 6, his recoil motion 0x6E), not the swing marked spent and
/// dying, not the combat state's result word: the swing goes on and keeps hitting.
///
/// As KH1 does it, through a KH1 attack object made from Sora's swing row (as `land_hit`): the
/// face's surface class (`0x14018FC20`: 0 → wall kind 0x100, 5 → 8, else 2); the sound
/// (`0x140293FD0 (Sora, 0, row's sound byte)`: the weapon's sound set, at Sora) for kinds 0x100
/// and 2; the spark (`0x1402B2190 (attack, kind, point)`: the weapon's effect table, `+8`, picked
/// by the kind and the data's effect bit `+0x28`) turned by (0, 0, 0, 1) − the normal
/// (`0x1402AB8F0`).
unsafe fn wall_spark(sora: usize, row: Row, hit: &WallHit) -> (u32, bool) {
    let base = BASE.load(Ordering::Relaxed);
    let surface: unsafe extern "system" fn(i32, i32) -> u32 = std::mem::transmute(base + SURFACE_RVA);
    let free_bit: unsafe extern "system" fn() -> u32 = std::mem::transmute(base + FREE_BIT_RVA);
    let new_attack: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + NEW_ATTACK_RVA);
    let free: unsafe extern "system" fn(usize) = std::mem::transmute(base + FREE_ATTACK_RVA);
    let sound: unsafe extern "system" fn(usize, u32, i32) -> usize = std::mem::transmute(base + WALL_SOUND_RVA);
    let spark: unsafe extern "system" fn(usize, u32, *const [f32; 4]) -> usize = std::mem::transmute(base + WALL_SPARK_RVA);
    let turn: unsafe extern "system" fn(usize, *const [f32; 4]) = std::mem::transmute(base + EFFECT_TURN_RVA);
    let kind = match surface(hit.owner, hit.face) {
        0 => 0x100,
        5 => 8,
        _ => 2,
    };
    // (No Keyblade object: KH1's own constructor and calls would find no weapon either.)
    if pose::ptr(rd::<u32>(sora + 0x14C)) < 0x10000 {
        return (kind, false);
    }
    let obj = new_attack(sora, free_bit(), base + ATTACK_DATA_RVA + row.data as usize);
    if obj < 0x10000 {
        return (kind, false);
    }
    // The Attack command's row's sound and effect bytes and its wall-sound flag (bit 22).
    wr(obj, rd::<u32>(obj) | 0x400000);
    wr(obj + 0xA8, row.fx);
    if kind & 0x102 != 0 {
        sound(sora, 0, rd::<i8>(obj + 0xA9) as i32);
    }
    let at = [hit.at[0], hit.at[1], hit.at[2], 1.0f32];
    let fx = spark(obj, kind, &at);
    if fx > 0x10000 {
        let n = hit.normal;
        turn(fx, &[-n[0], -n[1], -n[2], 1.0 - n[3]]);
    }
    free(obj);
    (kind, fx > 0x10000)
}

/// An enemy's guard: a KH1 attack object of priority 100 (`0x1402B2A30`: such an object
/// repels whatever attack meets it), e.g. the Large Body's belly, shields. Directional ones
/// (type bits `0xE0000` of 1 or 3) only guard an arc in front of their owner (`0x1402B24B0`).
struct Guard {
    shape: Shape,
    /// KH1 world centre.
    at: [f32; 3],
    owner: usize,
    /// Its clash type (flags bits 17..19).
    kind: u32,
    /// Tied to this motion of its owner (activity type 2), if so.
    motion: Option<u32>,
}

/// KH1's clash table (`0x1405299B8`, symmetric): which clash types meet which (bit n: meets
/// type n). Types 6 and 7 meet nothing.
const CLASH_TABLE: [u8; 8] = [0x1F, 0x09, 0x09, 0x3F, 0x29, 0x18, 0x00, 0x00];

/// Whether a guard of clash type `guard` meets Sora's swing of type `swing`.
fn clashes(guard: u32, swing: u32) -> bool {
    CLASH_TABLE[(guard & 7) as usize] & (1 << (swing & 7)) != 0
}

/// Sora's own live attack objects in KH1's pool (a spell's shot among them): owner `+0x34`.
pub unsafe fn sora_shots(sora: usize) -> usize {
    let base = BASE.load(Ordering::Relaxed);
    (0..ATTACK_SLOTS)
        .filter(|k| {
            let g = base + ATTACK_POOL_RVA + k * 0xC0;
            let flags = rd::<u32>(g);
            flags & 0x800 != 0 && flags & 0xC != 0 && flags & 0x0020_0000 == 0 && pose::ptr(rd::<u32>(g + 0x34)) == sora
        })
        .count()
}

const ATTACK_POOL_RVA: Rva = Rva(0x2DB5110);
const ATTACK_SLOTS: usize = 0x40;
const GUARD_PRIORITY: i32 = 100;
/// Guard objects: the most shapes per window, and the frames between re-arms (an attack met is
/// met again after that, as KH1's interval objects are).
const GUARD_SHAPES: usize = 4;
const GUARD_REARM: f32 = 10.0;
/// A pointer to KH1's handle (`0x14038B080`).
const TO_HANDLE_RVA: Rva = Rva(0x38B080);
const CLANG_RVA: Rva = Rva(0x2AC4C0);
/// KH1's attack-vs-attack meeting (`0x1402B2890 (entry, entry)`).
const CLASH_RVA: Rva = Rva(0x2B2890);
/// One side of KH1's clash (`0x1402B2A30 (attack p, attack q)`): p turned back (free-standing)
/// or given a clash record on its owner (KH1's recoil, kinds 6 / 7), `docs/kh1-reflect.md`.
const CLASH_HALF_RVA: Rva = Rva(0x2B2A30);
/// KH1's attack pairing table (8 bytes, a bit per type that may meet).
const PAIRS_RVA: Rva = Rva(0x5299B8);
/// KH1's attack-vs-background ray (`0x1402B1870 (attack, from, to)`): spark, sound, and a
/// clash record on the owner when it meets a wall (`wall_spark` does its first two).
#[allow(dead_code)]
const WALL_RVA: Rva = Rva(0x2B1870);
/// A face's surface class (`0x14018FC20 (object, face)`, bits 4..7 of its attributes).
const SURFACE_RVA: Rva = Rva(0x18FC20);
/// An attack's wall spark (`0x1402B2190 (attack, wall kind, point) -> effect`).
const WALL_SPARK_RVA: Rva = Rva(0x2B2190);
/// One of an actor's weapon sounds at the actor (`0x140293FD0 (actor, 0, n)`: weapon stats
/// `+0x34` + n).
const WALL_SOUND_RVA: Rva = Rva(0x293FD0);
/// Turns an effect to a direction (`0x1402AB8F0 (effect, direction)`).
const EFFECT_TURN_RVA: Rva = Rva(0x2AB8F0);
/// KH1's background ray (`0x140191520 (ray, mode)`): ray from `+0`, direction `+0x10`; out:
/// the point `+0x20`, the face `+0x30` (i16, -1 none), its object `+0x34` (-1 the map).
const RAY_RVA: Rva = Rva(0x191520);
/// A face's normal (`0x14018FC60 (object, face, out)`).
const FACE_NORMAL_RVA: Rva = Rva(0x18FC60);
const CLANG_SOURCE_RVA: Rva = Rva(0x2D67400);

/// The live enemy guards near Sora.
unsafe fn guards(sora: usize) -> Vec<Guard> {
    let base = BASE.load(Ordering::Relaxed);
    let hittable: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(base + HITTABLE_RVA);
    let team = |a: usize| (rd::<u32>(a + 0x374) >> 5) & 3;
    let ours = team(sora);
    let at = rd::<[f32; 3]>(sora + 0x10);
    let mut out = Vec::new();
    for k in 0..ATTACK_SLOTS {
        let g = base + ATTACK_POOL_RVA + k * 0xC0;
        let flags = rd::<u32>(g);
        // In use, active, not being freed or switched off.
        if flags & 0x800 == 0 || flags & 0xC == 0 || flags & 0x0120_0000 != 0 {
            continue;
        }
        if rd::<i32>(g + 0x68) != GUARD_PRIORITY {
            continue;
        }
        let owner = pose::ptr(rd::<u32>(g + 0x34));
        if owner < 0x10000 || owner == sora || team(owner) == ours || hittable(owner) & 0xFF == 0 {
            continue;
        }
        // Tied to a motion (activity type 2): only while the owner plays it.
        if (flags >> 2) & 3 == 2 {
            let req = rd::<u32>(g + 0x58);
            if req != 0xFFF && req != rd::<u32>(owner + 0x164) {
                continue;
            }
        }
        let c = rd::<[f32; 3]>(g + 0x10);
        let r = rd::<f32>(g + 0xC);
        if !(r > 0.0 && r < 1000.0) || !c.iter().all(|v| v.is_finite()) {
            continue;
        }
        let (dx, dz) = (c[0] - at[0], c[2] - at[2]);
        if dx * dx + dz * dz > NEAR * NEAR {
            continue;
        }
        // Directional: Sora must be within the arc (cosine at the data's `+0x64`) around the
        // owner's facing turned by the data's `+0x60`.
        if (flags & 0xE0000).wrapping_sub(0x20000) & 0xFFF9_FFFF == 0 {
            let data = pose::ptr(rd::<u32>(g + 0x5C));
            if data > 0x10000 {
                let f = rd::<[f32; 3]>(owner + 0x30);
                let turn = rd::<f32>(data + 0x60);
                let (s, co) = (turn.sin(), turn.cos());
                let fx = f[0] * co + f[2] * s;
                let fz = -f[0] * s + f[2] * co;
                let (o, me) = (rd::<[f32; 3]>(owner + 0x50), rd::<[f32; 3]>(sora + 0x50));
                let (dx, dz) = (me[0] - o[0], me[2] - o[2]);
                let (lf, ld) = ((fx * fx + fz * fz).sqrt(), (dx * dx + dz * dz).sqrt());
                if lf > 1e-4 && ld > 1e-4 && (fx * dx + fz * dz) / (lf * ld) < rd::<f32>(data + 0x64) {
                    continue;
                }
            }
        }
        static SEEN: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());
        if let Ok(mut v) = SEEN.try_lock() {
            if v.len() < 40 && !v.contains(&owner) {
                v.push(owner);
                let obj = pose::ptr(rd::<u32>(owner + 0x130));
                let id = if obj > 0x10000 { rd::<u16>(obj + 0x4C) } else { 0xFFFF };
                log(&format!("guard seen: actor {owner:#x} (object {id}), radius {r:.0}, flags {flags:08x}"));
            }
        }
        out.push(Guard { shape: Shape::sphere([c[0], -c[1], c[2]], r), at: c, owner, kind: (flags >> 17) & 7, motion: ((flags >> 2) & 3 == 2).then(|| rd::<u32>(g + 0x58)) });
    }
    out
}

/// KH1's clash spark and sound where a swing is repelled (`0x1402B2890`'s).
pub unsafe fn clang(at: [f32; 3]) {
    let base = BASE.load(Ordering::Relaxed);
    let source = rd::<u64>(base + CLANG_SOURCE_RVA) as usize;
    if source < 0x10000 {
        return;
    }
    let f: unsafe extern "system" fn(usize, u32, u32, *const [f32; 4]) -> usize = std::mem::transmute(base + CLANG_RVA);
    let p = [at[0], at[1], at[2], 1.0];
    f(source, 10, 20000, &p);
}

const TRAIL_RVA: Rva = Rva(0x2C0160);
const EFFECT_RVA: Rva = Rva(0x2ABB60);
const VOICE_RVA: Rva = Rva(0x28E060);
/// Actor `+0x1CC` bit: the weapon trail is drawn (`0x1402BFD70`; KH1's attack motions' triggers
/// set it over their swing).
const TRAIL_FLAG: u32 = 0x80_0000;
/// The trail flag is ours (set for a KH2 swing).
static TRAIL_ON: AtomicBool = AtomicBool::new(false);

/// Clears the trail flag if we set it (KH1 has Sora again, whatever the reason).
pub unsafe fn trail_off(sora: usize) {
    if TRAIL_ON.swap(false, Ordering::Relaxed) {
        wr(sora + 0x1CC, rd::<u32>(sora + 0x1CC) & !TRAIL_FLAG);
    }
}

impl Strikes {
    /// Phase 9: a KH2 swing starts with what KH1's matching combo row plays when its motion
    /// starts (`0x1402A17F0`): the Keyblade trail asked for, the swing's effect and Sora's
    /// voice. `count` is KH2's combo count (1 on the first swing).
    pub unsafe fn swing_start(&mut self, sora: usize, air: bool, count: u8, finisher: bool, kh1_voice: bool) {
        let obj = pose::ptr(rd::<u32>(sora + 0x130));
        let model = if obj > 0x10000 { rd::<u16>(obj + 0x4C) } else { u16::MAX };
        if self.swing_rows.as_ref().map(|r| r.0) != Some(model) {
            let mut m = HashMap::new();
            for i in 0..0x40 {
                let Some(r) = row_info(sora, i) else { break };
                if (200..=206).contains(&r.motion) {
                    m.entry(r.motion).or_insert(r);
                }
            }
            log(&format!("KH1 swing rows (motion: trail / effect / voice): {}", {
                let mut v: Vec<_> = m.values().map(|r| format!("{}: {} / {} / {}", r.motion, r.trail, r.effect, r.voice)).collect();
                v.sort();
                v.join(", ")
            }));
            if m.is_empty() {
                return;
            }
            self.swing_rows = Some((model, m));
        }
        let n = count.max(1) as i16 - 1;
        let motion = match (air, finisher) {
            (false, false) => 200 + n.min(2),
            (false, true) => 203,
            (true, false) => 204 + n.min(1),
            (true, true) => 206,
        };
        let Some(r) = self.swing_rows.as_ref().and_then(|m| m.1.get(&motion)).copied() else { return };
        let base = BASE.load(Ordering::Relaxed);
        let cs = pose::ptr(rd::<u32>(sora + 0x14C));
        let source = if cs > 0x10000 { rd::<u64>(cs + 0x18) as usize } else { 0 };
        // (Anti Form has no Keyblade: no trail and no Keyblade swing effect, the user: "sora still has
        // keyblade trails while in this form, lets not have that".)
        if source > 0x10000 && !super::form::in_anti() {
            if r.trail >= 0 {
                let f: unsafe extern "system" fn(usize, u32) = std::mem::transmute(base + TRAIL_RVA);
                f(sora, r.trail as u32);
                self.trail_asked = Some(0);
                self.trail_id = Some(r.trail as u32);
            }
            if r.effect >= 0 {
                let f: unsafe extern "system" fn(usize, i32, usize, u32) -> usize = std::mem::transmute(base + EFFECT_RVA);
                f(source, r.effect as i32, sora, 0x30);
            }
        }
        if r.voice >= 0 && kh1_voice {
            let f: unsafe extern "system" fn(usize, i32, usize) = std::mem::transmute(base + VOICE_RVA);
            f(sora, r.voice as i32, 0);
        }
    }

    /// KH2's Magnet: each caught enemy is drawn toward the pulling hitbox's centre, to its
    /// orbit radius at up to 20 a frame and swirled sideways (the exe's suction step, as
    /// `character_core::npc`), through KH1's own per-frame push (`+0x120`, which its physics
    /// step adds to the actor's move and clears). Let go when the swing ends.
    unsafe fn pull(&mut self, out: &Output, anim: &RigAnim, battle: &Battle) {
        if self.pulled.is_empty() {
            return;
        }
        let mut centres: HashMap<u64, [f32; 3]> = HashMap::new();
        for h in &out.hitboxes {
            let Some(a) = battle.attack_for_level(h.attack, 0, 0) else { continue };
            if character_core::reaction::category(a.enemy_reaction) != 5 {
                continue;
            }
            if let Some(c) = anim.attack_shapes(h.group, h.weapon).first() {
                centres.entry(h.swing).or_insert(c.center);
            }
        }
        let alive: HashMap<usize, f32> = self.bodies.iter().filter(|b| !b.fixed).map(|b| (b.actor, b.tall)).collect();
        self.pulled.retain(|a, (swing, _, _)| centres.contains_key(swing) && alive.contains_key(a));
        for (&actor, &(swing, radius, swirl)) in &self.pulled {
            let c = centres[&swing];
            let p = rd::<[f32; 3]>(actor + 0x10);
            // KH2's frame (y up), from the body's middle.
            let body = [p[0], -p[1] + alive[&actor] * 0.5, p[2]];
            let d = [c[0] - body[0], c[1] - body[1], c[2] - body[2]];
            let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if len < 1e-3 {
                continue;
            }
            let n = [d[0] / len, d[1] / len, d[2] / len];
            let radial = ((len - radius) * 0.1).clamp(-20.0, 20.0);
            let tl = (n[2] * n[2] + n[0] * n[0]).sqrt();
            let t = if tl > 1e-4 { [n[2] / tl, -n[0] / tl] } else { [0.0, 0.0] };
            let m = [n[0] * radial + t[0] * swirl, n[1] * radial, n[2] * radial + t[1] * swirl];
            let push = rd::<[f32; 3]>(actor + 0x120);
            wr(actor + 0x120, [push[0] + m[0], push[1] - m[1], push[2] + m[2]]);
        }
        static SEEN: AtomicU32 = AtomicU32::new(0);
        if SEEN.fetch_add(1, Ordering::Relaxed) % 60 == 0 {
            note(format!("Magnet: {} enemies held around {} centre(s)", self.pulled.len(), centres.len()));
        }
    }

    /// KH2's holding blows (reaction 9 with a small lift and a pull toward Sora: Aerial
    /// Finish's spin) keep the foe up beside him until the throw (the user: they slipped out of
    /// it). Each held foe is drawn toward a point beside Sora at his height through KH1's
    /// per-frame push (`+0x120`), for `HOLD_FRAMES` after its last such hit.
    unsafe fn hold(&mut self, sora: usize) {
        if self.held.is_empty() {
            return;
        }
        let alive: HashSet<usize> = self.bodies.iter().filter(|b| !b.fixed).map(|b| b.actor).collect();
        self.held.retain(|a, left| {
            *left = left.saturating_sub(1);
            *left > 0 && alive.contains(a)
        });
        let s = rd::<[f32; 3]>(sora + 0x10);
        for &actor in self.held.keys() {
            let p = rd::<[f32; 3]>(actor + 0x10);
            let (dx, dz) = (p[0] - s[0], p[2] - s[2]);
            let d = (dx * dx + dz * dz).sqrt();
            // Beside him: no farther than HOLD_REACH, his feet's height a little up (y down).
            let k = if d > HOLD_REACH { HOLD_REACH / d } else { 1.0 };
            let goal = [s[0] + dx * k, s[1] - HOLD_UP, s[2] + dz * k];
            let mut m = [(goal[0] - p[0]) * 0.35, (goal[1] - p[1]) * 0.35, (goal[2] - p[2]) * 0.35];
            let len = (m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt();
            if len > 30.0 {
                for v in &mut m {
                    *v *= 30.0 / len;
                }
            }
            let push = rd::<[f32; 3]>(actor + 0x120);
            wr(actor + 0x120, [push[0] + m[0], push[1] + m[1], push[2] + m[2]]);
        }
    }

    /// The trail's flag over KH2's swing (its hit windows); only ever cleared if we set it.
    /// A trail has been asked of KH1 for the current move.
    pub fn has_trail(&self) -> bool {
        self.trail_id.is_some()
    }

    /// The guard's swish: KH1's Keyblade trail of a ground swing (the guard has no swing row).
    pub unsafe fn trail_for_guard(&mut self, sora: usize) {
        if self.swing_rows.is_none() {
            // The rows are read on the first swing; a guard first reads them the same way.
            self.swing_start(sora, false, 1, false, false);
            return;
        }
        let Some(r) = self.swing_rows.as_ref().and_then(|m| m.1.get(&200)).copied() else { return };
        let cs = pose::ptr(rd::<u32>(sora + 0x14C));
        let source = if cs > 0x10000 { rd::<u64>(cs + 0x18) as usize } else { 0 };
        if source > 0x10000 && r.trail >= 0 {
            let f: unsafe extern "system" fn(usize, u32) = std::mem::transmute(BASE.load(Ordering::Relaxed) + TRAIL_RVA);
            f(sora, r.trail as u32);
            self.trail_asked = Some(0);
            self.trail_id = Some(r.trail as u32);
        }
    }

    pub unsafe fn trail(&mut self, sora: usize, swinging: bool) {
        let flags = rd::<u32>(sora + 0x1CC);
        // The hit window opening: KH1's trail object (`+0x14C`'s `+0x28`) must be there; KH1
        // drops a trail asked for too long before (the user: most ground combo modifiers, whose
        // blows come 30+ frames into the motion, had no trail).
        if swinging && !self.was_swinging {
            let cs = pose::ptr(rd::<u32>(sora + 0x14C));
            if let Some(id) = self.trail_id {
                if cs > 0x10000 && rd::<u64>(cs + 0x28) == 0 && rd::<u64>(cs + 0x18) != 0 {
                    let f: unsafe extern "system" fn(usize, u32) = std::mem::transmute(BASE.load(Ordering::Relaxed) + TRAIL_RVA);
                    f(sora, id);
                    self.trail_asked = Some(0);
                    static N: AtomicU32 = AtomicU32::new(0);
                    if N.fetch_add(1, Ordering::Relaxed) < 12 {
                        log("Keyblade trail asked again as the hit window opened (KH1 had dropped it)");
                    }
                }
            }
        }
        self.was_swinging = swinging;
        if swinging {
            wr(sora + 0x1CC, flags | TRAIL_FLAG);
            TRAIL_ON.store(true, Ordering::Relaxed);
        } else {
            trail_off(sora);
        }
        // Whether the trail came (logged for the first few).
        if let Some(n) = self.trail_asked {
            let cs = pose::ptr(rd::<u32>(sora + 0x14C));
            let made = cs > 0x10000 && rd::<u64>(cs + 0x28) != 0;
            if made || n >= 30 {
                static SEEN: AtomicU32 = AtomicU32::new(0);
                if SEEN.fetch_add(1, Ordering::Relaxed) < 12 {
                    log(&format!("Keyblade trail {} ({n} frames after asking, trail flag {})", if made { "drawn" } else { "not drawn" }, flags & TRAIL_FLAG != 0));
                }
                self.trail_asked = None;
            } else {
                self.trail_asked = Some(n + 1);
            }
        }
    }

    /// The enemies this frame, as KH2's targets (before KH2's step).
    pub unsafe fn look(&mut self, sora: usize) -> Vec<TargetInfo> {
        self.bodies = bodies(sora);
        self.guards = guards(sora);
        self.shots = shots(sora);
        self.bodies.iter().map(|b| b.target).collect()
    }

    /// Our attack data copies for guard objects (front-only for a Keyblade's guard, all round
    /// for one on his body), from his swing's data with no power, no reaction, no direction
    /// turn; their handles. Made once.
    unsafe fn guard_mem(&mut self, sora: usize) -> Option<(usize, u32, u32)> {
        if let Some(m) = self.guard_mem {
            return Some(m);
        }
        let base = BASE.load(Ordering::Relaxed);
        let rows = self.rows?;
        let mem = super::sys::VirtualAlloc(std::ptr::null_mut(), 0x1000, 0x3000, 0x04) as usize;
        if mem == 0 {
            return None;
        }
        let to_handle: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(base + TO_HANDLE_RVA);
        let src = base + ATTACK_DATA_RVA + rows[0].data as usize;
        for (k, cos) in [(0usize, 0.0f32), (1, -1.0)] {
            let d = mem + k * 0x100;
            std::ptr::copy_nonoverlapping(src as *const u8, d as *mut u8, 0x70);
            wr(d + 8, 0u32);
            wr(d + 0x60, 0.0f32);
            wr(d + 0x64, cos);
        }
        let (h0, h1) = (to_handle(mem), to_handle(mem + 0x100));
        if pose::ptr(h0) != mem || pose::ptr(h1) != mem + 0x100 {
            log(&format!("guards: our attack data's handles don't resolve ({h0:#x} {h1:#x}): no guard objects"));
            return None;
        }
        let _ = sora;
        self.guard_mem = Some((mem, h0, h1));
        Some((mem, h0, h1))
    }

    /// KH2's guard windows as KH2 has them: every live `Guard` window of Sora's move (the guard
    /// press 173 and its repel 42 on the Keyblade; Aerial Dodge 202 all through and Aerial
    /// Recovery 40 frames 0..16 on his body; any other) puts KH1 guard attack objects on the
    /// window's shapes (the user: "aerial dodge currently has a hitbox attached to it, I only
    /// want a guard hitbox (like the guarding animation state)"; "aerial recovery is missing its
    /// guard hitbox, i frames and guard hitbox timing should be 1:1 with kh2").
    ///
    /// - A KH1 guard is an attack object of priority 100: KH1's attack-vs-attack pass
    ///   (`0x1402B2890` / `0x1402B2A30`) leaves it unhurt and gives the attack meeting it a clash
    ///   (the attacker's recoil, a shot turned back). Clash type 4, a pure guard: KH1's pass
    ///   never meets it with a body (`0x1402B1A50`), so it hits nobody. No power, no reaction.
    /// - It meets attacks whatever Sora's own state (his invulnerable frames keep him out of the
    ///   hit pass, not his attack objects): Aerial Recovery guards through its i-frames.
    /// - Always live (activity 3) and re-armed every `GUARD_REARM` frames, placed on its shape
    ///   each frame, freed when its window closes.
    /// - An attack it stops: the window's ATKP "refact self" as KH2's guard reaction
    ///   (`Character::clash`: 1 the guard repel, 2 nothing), logged.
    unsafe fn guard_step(&mut self, sora: usize, out: &Output, anim: &RigAnim, character: &mut Character, battle: &Battle) {
        static N: AtomicU32 = AtomicU32::new(0);
        let base = BASE.load(Ordering::Relaxed);
        let free: unsafe extern "system" fn(usize) = std::mem::transmute(base + FREE_ATTACK_RVA);
        let ours = |o: usize| {
            let f = rd::<u32>(o);
            f & 0x800 != 0 && f & 0x0020_0000 == 0 && pose::ptr(rd::<u32>(o + 0x34)) == sora
        };
        let mut live: Vec<((u64, usize), Shape, u8, u16)> = Vec::new();
        // Anti Form guards nothing (the user: "you shouldnt be able to guard in anti form"): its
        // moves' guard windows (the spinning finishers 183 / 184, ATKP 455) put no guard objects;
        // only the form change's own (52 / 53) are kept.
        let anti_blocked = super::form::in_anti() && out.motion != super::form::DRIVE_MOTION && out.motion != super::form::ARRIVE_MOTION;
        for g in out.guard_boxes.iter().filter(|_| !anti_blocked) {
            for (i, sh) in anim.attack_shapes(g.group, g.weapon).into_iter().take(GUARD_SHAPES).enumerate() {
                live.push(((g.swing, i), sh, g.weapon, g.attack));
            }
        }
        // Windows closed: their objects freed.
        let keys: HashSet<(u64, usize)> = live.iter().map(|l| l.0).collect();
        let gone: Vec<(u64, usize)> = self.guard_objs.keys().filter(|k| !keys.contains(k)).copied().collect();
        for k in gone {
            if let Some((o, _)) = self.guard_objs.remove(&k) {
                if ours(o) {
                    free(o);
                }
            }
        }
        if live.is_empty() {
            return;
        }
        let Some((_, front, round)) = self.guard_mem(sora) else { return };
        for (key, sh, weapon, attack) in live {
            let c = [sh.center[0], -sh.center[1], sh.center[2], 1.0f32];
            let r = sh.radius.max(sh.height).clamp(10.0, 150.0);
            let (o, last) = match self.guard_objs.get(&key) {
                Some(&(o, m)) if ours(o) => (o, m),
                _ => {
                    let free_bit: unsafe extern "system" fn() -> u32 = std::mem::transmute(base + FREE_BIT_RVA);
                    let new_attack: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + NEW_ATTACK_RVA);
                    let src = pose::ptr(if weapon == 0 { round } else { front });
                    let o = new_attack(sora, free_bit(), src);
                    if o < 0x10000 {
                        continue;
                    }
                    // Own position (mode 0), always live (activity 3), a pure guard (clash type 4),
                    // not spent; priority 100; no power, no reaction; its data ours (the arc).
                    let f = (rd::<u32>(o) & !0x3 & !0xC & !0xE0000 & !0x0010_0000 & !0x200) | 0xC | (4 << 17);
                    wr(o, f);
                    wr(o + 0x68, GUARD_PRIORITY);
                    wr(o + 0x2C, 0i32);
                    wr(o + 0x30, rd::<u32>(o + 0x30) & !0x1F);
                    wr(o + 0x50, GUARD_REARM);
                    wr(o + 0x5C, if weapon == 0 { round } else { front });
                    wr(o + 0x10, c);
                    wr(o + 0x40, c);
                    wr(o + 0x70, c);
                    wr(o + 0x80, c);
                    if N.fetch_add(1, Ordering::Relaxed) < 60 {
                        log(&format!("guard: motion {}'s window (ATKP {attack}, {}) as KH1 guard object {o:#x}, radius {r:.0}", out.motion, if weapon == 0 { "on his body, all round" } else { "on the Keyblade, in front" }));
                    }
                    self.guard_objs.insert(key, (o, 0));
                    (o, 0)
                }
            };
            // On its shape now (swept from last frame's).
            wr(o + 0x70, rd::<[f32; 4]>(o + 0x10));
            wr(o + 0x80, rd::<[f32; 4]>(o + 0x40));
            wr(o + 0x10, c);
            wr(o + 0x40, c);
            wr(o + 0xC, r);
            wr(o + 0x3C, r);
            wr(o, rd::<u32>(o) | 0x1_0000);
            // An attack it stopped since last frame (its clashed-with mask gained a bit).
            let mask = rd::<u32>(o + 0x9C);
            let new = mask & !last;
            self.guard_objs.insert(key, (o, mask));
            if new == 0 {
                continue;
            }
            let me = rd::<[f32; 3]>(sora + 0x10);
            let from = self.shots.iter().find(|s| rd::<u32>(s.obj + 0x54) & new != 0).map(|s| (s.owner, rd::<[f32; 3]>(s.obj + 0x10)));
            let toward = from.map(|(_, p)| glam::Vec2::new(p[0] - me[0], p[2] - me[2]).normalize_or_zero()).filter(|v| *v != glam::Vec2::ZERO).unwrap_or_else(|| {
                let f = rd::<[f32; 4]>(sora + 0x30);
                glam::Vec2::new(f[0], f[2])
            });
            let refact = battle.attack_for_level(attack, 0, 0).map_or(2, |a| a.refact_self);
            if refact != 2 {
                character.clash(refact, 0, toward.to_array());
            }
            if N.fetch_add(1, Ordering::Relaxed) < 60 {
                log(&format!("guard: motion {}'s guard (ATKP {attack}) stopped an attack of {:#x?} (refact self {refact})", out.motion, from.map(|f| f.0)));
            }
        }
    }

    /// After KH2's step and pose: its live attack shapes against the enemies' bodies.
    pub unsafe fn strike(&mut self, sora: usize, out: &Output, anim: &RigAnim, character: &mut Character, battle: &Battle) {
        if !self.rows_tried {
            self.rows_tried = true;
            self.rows = sora_rows(sora);
            if self.rows.is_none() {
                log("Sora's KH1 combo rows not found: KH2's hits cannot be landed");
            }
        }
        let keys: HashSet<(u64, u8)> = out.hitboxes.iter().map(|h| (h.swing, h.weapon)).collect();
        self.volumes.retain(|k, _| keys.contains(k));
        self.sparked.retain(|k| keys.contains(k));
        self.met_shots.retain(|(sw, _)| keys.iter().any(|k| k.0 == *sw));
        let air = character.airborne();
        // KH2's guard windows as KH1 guard objects (the Aerial Dodge's, Aerial Recovery's, the
        // guard's).
        self.guard_step(sora, out, anim, character, battle);
        // KH1's Keyblade as drawn, this frame and last (the user: "can you align clanks with
        // soras kh1 keyblade, it should align with the length of the keyblade as well so smaller
        // keyblades are harder to clank while longer ones clank much more easy"): the right
        // swing's clashes (guards, enemy attacks, walls) are this blade's sweep, KH1's swing's
        // radius, not KH2's wide hit shapes.
        let base = BASE.load(Ordering::Relaxed);
        let blade = self.rows.and_then(|rows| blade_points(sora, base + ATTACK_DATA_RVA + rows[(air as usize) * 2].data as usize));
        let blade_prev = self.blade_prev.take();
        let sweep = blade.as_ref().map(|(now, r)| blade_sweep(blade_prev.as_deref().unwrap_or(&[]), now, *r));
        self.blade_prev = blade.as_ref().map(|(p, _)| p.clone());
        for h in &out.hitboxes {
            let shapes = anim.attack_shapes(h.group, h.weapon);
            // (The right Keyblade's swing: KH1's blade; Valor's left one keeps KH2's shapes.)
            // (Anti Form holds no Keyblade: its attacks are its body's own shapes, KH2's.)
            let on_blade = if h.weapon == 0 && !super::form::in_anti() { sweep.as_ref() } else { None };
            let vol = self.volumes.entry((h.swing, h.weapon)).or_default();
            let before: Vec<[f32; 3]> = vol.previous().to_vec();
            let swept = vol.update(&shapes);
            if swept.is_empty() {
                continue;
            }
            let Some(first) = battle.attack_for_level(h.attack, 0, 0).cloned() else { continue };
            let group = if first.combo_group != 0 { first.combo_group as i64 } else { h.swing as i64 };
            // An enemy's guard repels the swing before it reaches a body (KH2's repel, as off a
            // wall; KH1's clash spark and sound).
            if self.repelled.contains(&h.swing) {
                continue;
            }
            // A wall in the blade's way since last frame: KH1's background ray, as its own swings
            // have it (`0x1402B1870`). Walls only: a floor or ceiling the blade dips into doesn't
            // count. What it does is the ini's `walls`:
            // - kh1 (default; the user, with a video of KH1's own swing into a wall: "lets try to
            //   get this working instead of the clinking associated with enemies"): KH1's wall
            //   spark and sound where the blade touched, and the swing goes on. Once a swing per
            //   blade, as KH1 tests its own swing's object only until its first wall.
            // - clink: KH2's repel off it, Sora bounced (the user had asked for that when swings
            //   went through walls).
            // - pass: nothing.
            let walls = super::settings::get().walls;
            let key = (h.swing, h.weapon);
            let test = match walls {
                Walls::Kh1 => !self.sparked.contains(&key),
                Walls::Clink => true,
                Walls::Pass => false,
            };
            let wall = match (test, on_blade, blade.as_ref(), blade_prev.as_ref()) {
                (false, ..) => None,
                (_, Some(_), Some((now, _)), Some(prev)) if prev.len() == now.len() => prev.iter().zip(now).find_map(|(p, q)| wall_between(*p, *q)),
                (_, Some(_), _, _) => None,
                _ => before.iter().zip(shapes.iter()).find_map(|(p, s)| wall_between([p[0], -p[1], p[2]], [s.center[0], -s.center[1], s.center[2]])),
            };
            match (walls, wall) {
                (Walls::Clink, Some(hit)) => {
                    self.repelled.insert(h.swing);
                    let bounced = character.register_wall_hit();
                    clang(hit.at);
                    note(format!("KH2 attack {} struck a wall at {:.0?} (bounced: {bounced})", h.attack, hit.at));
                    continue;
                }
                (Walls::Kh1, Some(hit)) => {
                    self.sparked.insert(key);
                    if let Some(rows) = self.rows {
                        let (kind, shown) = wall_spark(sora, rows[(air as usize) * 2], &hit);
                        note(format!("KH2 attack {} (weapon {}) met a wall at {:.0?}: KH1's wall spark, kind {kind:#x}, effect made {shown}", h.attack, h.weapon, hit.at));
                    }
                }
                _ => {}
            }
            // Only guards KH1's clash table lets meet Sora's swing (its row data's `+0x38` type;
            // the user: aerial combos on Dragon Maleficent clinked: her big guard is type 6,
            // which meets nothing).
            let swing_data = self.rows.map(|rows| BASE.load(Ordering::Relaxed) + ATTACK_DATA_RVA + rows[(air as usize) * 2].data as usize);
            let swing_type = swing_data.map(|d| rd::<u32>(d + 0x38) & 7);
            // KH1's swing is one thin sphere on the blade (its data's `+0x5C` radius); KH2's hit
            // shapes are wide. A guard is met only by the sweep thinned to KH1's radius (the
            // user's Dragon Maleficent video: half the air combos' first swings were repelled by
            // her 180-wide guard while reaching for her head).
            let blade = swing_data.map_or(25.0, |d| (rd::<i32>(d + 0x5C) as f32).clamp(8.0, 60.0));
            let thin: Vec<Shape> = match on_blade {
                Some((b, _)) => b.clone(),
                None => swept.iter().map(|s| Shape { radius: s.radius.min(blade), height: s.height.min(blade), ..*s }).collect(),
            };
            let guards: Vec<&Guard> = self.guards.iter().filter(|g| swing_type.map_or(true, |t| clashes(g.kind, t))).collect();
            static TYPES: AtomicU32 = AtomicU32::new(0);
            if !self.guards.is_empty() && TYPES.fetch_add(1, Ordering::Relaxed) < 6 {
                log(&format!("guards near: clash types {:?} against Sora's swing type {swing_type:?} (blade radius {blade}): {} can repel", self.guards.iter().map(|g| g.kind).collect::<Vec<_>>(), guards.len()));
            }
            let guard_shapes: Vec<Shape> = guards.iter().map(|g| g.shape).collect();
            // Only a guard met before any body part along this frame's sweep (KH1 tests a swing's
            // sweep entries in order, and KH1's blade is thin; KH2's wide hit shapes touched a
            // guard beside the part they hit: the user, Darkballs and Dragon Maleficent clinked
            // way too much).
            // The part this frame's sweep touches first, if any (its index along the sweep and its
            // shape).
            let body_at = self
                .bodies
                .iter()
                .filter_map(|b| first_overlap(&thin, &b.parts.iter().map(|p| p.shape).collect::<Vec<_>>()).map(|(ai, ti)| (ai, b.parts[ti].shape)))
                .min_by_key(|(ai, _)| *ai);
            // With a part touched, a guard repels only when that part lies within the guard: the
            // blade has to go through the guard to reach it (KH1's blade, thin, could reach a
            // part outside it without).
            let inside = |g: &Shape, p: &Shape| {
                let d = ((p.center[0] - g.center[0]).powi(2) + (p.center[1] - g.center[1]).powi(2) + (p.center[2] - g.center[2]).powi(2)).sqrt();
                d < g.radius
            };
            if let Some((gai, gi)) = first_overlap(&thin, &guard_shapes).filter(|(gai, gi)| body_at.map_or(true, |(b, part)| *gai < b && inside(&guard_shapes[*gi], &part))) {
                self.repelled.insert(h.swing);
                let bounced = character.register_wall_hit();
                clang(guards[gi].at);
                let g = guards[gi];
                let near_part = self
                    .bodies
                    .iter()
                    .find(|b| b.actor == g.owner)
                    .and_then(|b| b.parts.iter().map(|p| ((p.shape.center[0] - g.at[0]).hypot(p.shape.center[2] - g.at[2]).hypot(p.shape.center[1] + g.at[1]), p.shape.radius)).min_by(|a, b| a.0.total_cmp(&b.0)));
                note(format!(
                    "KH2 attack {} repelled by the guard of actor {:#x}, clash type {} radius {:.0}, motion-bound {:?} (owner motion {}), nearest hittable part {:?} (centre distance, radius) (bounced: {bounced}){}",
                    h.attack, g.owner, g.kind, g.shape.radius, g.motion, rd::<u32>(g.owner + 0x164), near_part,
                    on_blade.map_or(" (KH2's shapes)".into(), |(_, along)| format!(" at {:.0}% along KH1's blade", along.get(gai).copied().unwrap_or(0.0) * 100.0))
                ));
                continue;
            }
            // An enemy's attack met: as KH1's swing would meet it (`0x1402B2890`): a shot knocked
            // back (the user: Wakka's ball), or a clash (the user: "not allowing him to clank with
            // enemy attacks"). Enemy melee attacks only by KH1's blade (KH2's wide shapes met them
            // far too often, the user, twice).
            let mut stopped = false;
            if let Some(rows) = self.rows {
                let clash_shapes: &Vec<Shape> = on_blade.map(|(b, _)| b).unwrap_or(&swept);
                for shot in self.shots.iter().filter(|s| s.free || on_blade.is_some()) {
                    if self.met_shots.contains(&(h.swing, shot.obj)) {
                        continue;
                    }
                    let Some((ai, _)) = first_overlap(clash_shapes, &shot.shapes) else { continue };
                    self.met_shots.insert((h.swing, shot.obj));
                    let (back, ours) = meet_shot(sora, shot, &clash_shapes[ai], rows[(air as usize) * 2]);
                    if back {
                        character.register_hit();
                    }
                    static MET: AtomicU32 = AtomicU32::new(0);
                    if MET.fetch_add(1, Ordering::Relaxed) < 200 {
                        log(&format!(
                            "KH2 swing met enemy attack {:#x} of {:#x} ({}; flags now {:08x}): turned back {back}, the swing clashed {ours}{}",
                            shot.obj,
                            shot.owner,
                            if shot.free { "a shot" } else { "a melee attack" },
                            rd::<u32>(shot.obj),
                            on_blade.map_or(String::new(), |(_, along)| format!(", at {:.0}% along KH1's blade", along.get(ai).copied().unwrap_or(0.0) * 100.0))
                        ));
                    }
                    if ours {
                        stopped = true;
                        break;
                    }
                }
            }
            if stopped {
                // (Stopped there: its recoil is KH1's clash record on Sora, which `hurt` plays.)
                self.repelled.insert(h.swing);
                continue;
            }
            for b in &self.bodies {
                let rec = self.records.entry(b.actor).or_default();
                if rec.has_hit(h.swing) {
                    continue;
                }
                let targets: Vec<Shape> = b.parts.iter().map(|p| p.shape).collect();
                let Some((ai, ti)) = first_overlap(&swept, &targets) else { continue };
                let adm = rec.admit(group, false, false);
                let row = battle.attack_for_level(h.attack, 0, adm.count.min(255) as u8).cloned().unwrap_or(first.clone());
                rec.mark(h.swing, row.interval);
                if !adm.effective {
                    continue;
                }
                character.register_hit();
                // A blow that throws (KH1's finisher row, a launch): not one that holds the foe
                // up beside Sora, as Aerial Finish's spinning hits do (reaction 9, up 5..12,
                // sideways -1 / -10: toward him). KH1's finisher row sent the foe ~250 away on
                // the first, and the rest missed (the user: pushed out of the finisher).
                let blow = character_core::reaction::category(row.enemy_reaction) == 4;
                let k1 = if row.knockback_1 == i16::MAX { 0 } else { row.knockback_1 };
                let k2 = row.knockback_2;
                let throws = (k1 == 0 && k2 == 0) || k2 >= 10 || k1 >= 30;
                let finisher = blow && throws;
                let Some(rows) = self.rows else { continue };
                let kh1 = rows[(air as usize) * 2 + finisher as usize];
                // KH2's launch, its own height and speed (the user: Upper Slash didn't send them
                // up): KH1 launches with its own, so its launch is redone (`launched`).
                // (Not for the big and the immovable: KH1's own reaction stays, the user: Magnet
                // Burst and Aerial Finish moved Dragon Maleficent.)
                if let Ok(mut p) = PENDING.lock() {
                    let (k1, k2) = farther(h.attack).unwrap_or((row.knockback_1, row.knockback_2));
                    *p = (finisher && !b.fixed).then(|| Launch::kh2(k1, k2, &character.reaction));
                }
                // A holding blow (Aerial Finish's spin) flinches the foe where it is: KH1's
                // flinch knocks an airborne foe along the record's direction, and spin after
                // spin slid it out of reach (the user); the direction is zeroed (`scale_record`).
                // (Every hit lands as it is: s179's silent multi-hit ticks were taken out again in
                // s193, the user: "revert this back to before we asked to make any revenge value
                // changes".)
                HOLD.store(blow && !throws && !b.fixed, Ordering::Relaxed);
                MAGIC.store(row.stat_source == 1, Ordering::Relaxed);
                // (The move this hit belongs to: its action, `swing`'s high bits.)
                let dealt = land(sora, &b.parts[ti], &swept[ai], kh1, row.power, false, row.drive_drain);
                MAGIC.store(false, Ordering::Relaxed);
                HOLD.store(false, Ordering::Relaxed);
                // (Again once the contact is done, in case its direction came after the record.)
                let rec = HELD_REC.swap(0, Ordering::Relaxed);
                if rec > 0x10000 {
                    let w = rd::<f32>(rec + 0x1C);
                    wr(rec + 0x10, [0.0f32, 0.0, 0.0, w]);
                }
                if let Ok(mut p) = PENDING.lock() {
                    *p = None;
                }
                // A Magnet's catch (the user: Magnet Burst didn't pull them together): KH2 picks
                // the orbit radius between the two knockback values, the swirl from the next.
                if blow && !throws && dealt.is_some() && !b.fixed {
                    self.held.insert(b.actor, HOLD_FRAMES);
                }
                if character_core::reaction::category(row.enemy_reaction) == 5 && !b.fixed {
                    let (a, c) = (row.knockback_1 as f32, row.knockback_2 as f32);
                    let t = ((b.actor >> 4) % 97) as f32 / 96.0;
                    self.pulled.insert(b.actor, (h.swing, a + (c - a) * t, row.unknown_16 as f32));
                }
                note(format!(
                    "KH2 attack {} (power {}, reaction {}) hit actor {:#x} part {ti} at {:?}: landed with KH1 row motion {} -> {dealt:?}",
                    h.attack, row.power, row.enemy_reaction, b.actor, swept[ai].center, kh1.motion
                ));
            }
        }
        self.pull(out, anim, battle);
        self.hold(sora);
        let now: HashSet<u64> = out.hitboxes.iter().map(|h| h.swing).collect();
        self.repelled.retain(|s| now.contains(s));
        for ended in self.live.difference(&now) {
            for r in self.records.values_mut() {
                r.forget(*ended);
            }
        }
        for r in self.records.values_mut() {
            r.tick(1.0, false);
        }
        self.live = now;
        let alive: HashSet<usize> = self.bodies.iter().map(|b| b.actor).collect();
        self.records.retain(|a, _| alive.contains(a));
    }
}

/// A spell's attack this frame (`reflect`): its hit-record key and group, ATKP id and sub, and
/// its shapes (KH2 y-up, swept as the game sweeps spell attacks).
pub struct SpellAttack {
    /// KH1 attack data to land it with (an offset into `ATTACK_DATA_RVA`'s table: a KH1 spell's
    /// own entry, for its element), instead of Sora's swing rows (`kh1_spell_data`).
    pub kh1_data: Option<i32>,
    pub key: u64,
    pub group: i64,
    pub atk: u16,
    pub sub: u8,
    pub shapes: Vec<Shape>,
}

impl Strikes {
    /// This frame's enemies (from `look`): id (as KH2's targets have it), feet and aim point (KH2
    /// y-up), for the spell scripts.
    pub unsafe fn targets(&self) -> Vec<(u32, [f32; 3], [f32; 3])> {
        self.bodies
            .iter()
            .map(|b| {
                let p = rd::<[f32; 3]>(b.actor + 0x10);
                (b.target.id, [p[0], -p[1], p[2]], b.target.pos)
            })
            .collect()
    }

    /// A spell's attacks against the enemies, through the same hit records as Sora's swings
    /// (once per attack until its row's re-hit interval runs out, rows by hits in a row from its
    /// group), each hit landed with KH1's contact code like a swing's: Sora's ground swing row,
    /// or his finisher's for a row that knocks away (KH2 reaction category 4, or a hit-back of
    /// 100 or more; ours). `row` gives an ATKP row by (id, sub, hit count). Returns (ATKP id,
    /// actor, damage) for each landed hit.
    pub unsafe fn spell_strike(&mut self, sora: usize, attacks: &[SpellAttack], row: &dyn Fn(u16, u8, u8) -> Option<kh2_formats::battle::Atkp>) -> Vec<(u64, u16, usize, Option<u32>)> {
        if !self.rows_tried {
            self.rows_tried = true;
            self.rows = sora_rows(sora);
        }
        let Some(rows) = self.rows else { return Vec::new() };
        let mut out = Vec::new();
        for a in attacks {
            let Some(first) = row(a.atk, a.sub, 0) else { continue };
            for b in &self.bodies {
                let rec = self.records.entry(b.actor).or_default();
                if rec.has_hit(a.key) {
                    continue;
                }
                let targets: Vec<Shape> = b.parts.iter().map(|p| p.shape).collect();
                let Some((ai, ti)) = first_overlap(&a.shapes, &targets) else { continue };
                let adm = rec.admit(a.group, false, false);
                let r = row(a.atk, a.sub, adm.count.min(255) as u8).unwrap_or(first.clone());
                rec.mark(a.key, r.interval);
                if !adm.effective {
                    continue;
                }
                let k1 = if r.knockback_1 == i16::MAX { 0 } else { r.knockback_1 };
                let knock = character_core::reaction::category(r.enemy_reaction) == 4 || k1 >= 100;
                // A spell's own KH1 data (its element, its max-MP source, its reactions), else
                // the swing's (physical).
                let (kh1, spell_data) = match a.kh1_data {
                    Some(off) => (Row { data: off, fx: [0; 3], ..rows[knock as usize] }, true),
                    None => (rows[knock as usize], false),
                };
                MAGIC.store(!spell_data && r.stat_source == 1, Ordering::Relaxed);
                let dealt = land(sora, &b.parts[ti], &a.shapes[ai], kh1, r.power, spell_data, r.drive_drain);
                MAGIC.store(false, Ordering::Relaxed);
                note(format!("spell ATKP {} (power {}, reaction {}) hit actor {:#x} part {ti}: landed with KH1 row motion {} -> {dealt:?}", a.atk, r.power, r.enemy_reaction, b.actor, kh1.motion));
                out.push((a.key, a.atk, b.actor, dealt));
            }
        }
        out
    }

    /// A spell attack ended: whoever it hit may be hit by a new one.
    pub fn spell_forget(&mut self, key: u64) {
        for r in self.records.values_mut() {
            r.forget(key);
        }
    }

    /// A Reflect barrier (`shapes`, KH2 y-up) against the enemies' attacks this frame (KH2's
    /// reflect guard, the user: "reflect does not have a block hitbox so it can't send
    /// projectiles or parry moves"): a shot it touches is turned back as a swing turns it (KH1's
    /// clash, the shot's half: team swapped, Sora its owner); an enemy's melee attack it touches
    /// gets KH1's clash record on its owner (the recoil its script plays), with KH1's spark.
    /// Once per barrier (`key`) and attack. Returns the shots turned back (they never reach
    /// Sora, so `reflect` counts them for its counter; melee hits still reach `hurt`).
    /// Every enemy attack meeting a Reflect barrier for the first time (one per attack object
    /// and barrier): where it met it (KH1 world). A melee attack only counts (it passes through
    /// Sora, intangible meanwhile: no recoil on its owner, which read as Reflect hitting back at
    /// every hit, the user); a shot is stopped as KH1's guard stops one (turned back when KH1
    /// lets it be).
    pub unsafe fn barrier_meet(&mut self, sora: usize, key: u64, shapes: &[Shape]) -> Vec<[f32; 3]> {
        if !self.rows_tried {
            self.rows_tried = true;
            self.rows = sora_rows(sora);
        }
        let mut met = Vec::new();
        let Some(rows) = self.rows else { return met };
        if shapes.is_empty() {
            return met;
        }
        let base = BASE.load(Ordering::Relaxed);
        let free_bit: unsafe extern "system" fn() -> u32 = std::mem::transmute(base + FREE_BIT_RVA);
        let new_attack: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + NEW_ATTACK_RVA);
        let free: unsafe extern "system" fn(usize) = std::mem::transmute(base + FREE_ATTACK_RVA);
        let half: unsafe extern "system" fn(usize, usize) -> u32 = std::mem::transmute(base + CLASH_HALF_RVA);
        // (A new barrier forgets the last one's meetings.)
        self.barrier_met.retain(|(k, _)| *k == key);
        for shot in &self.shots {
            if self.barrier_met.contains(&(key, shot.obj)) {
                continue;
            }
            let Some((ai, ti)) = first_overlap(shapes, &shot.shapes) else { continue };
            self.barrier_met.insert((key, shot.obj));
            // Where they meet: the attack's side of the barrier (KH1 world, y down).
            let (b, t) = (shapes[ai].center, shot.shapes[ti].center);
            let d = [t[0] - b[0], t[1] - b[1], t[2] - b[2]];
            let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-3);
            let r = shapes[ai].radius.min(l);
            let at = [b[0] + d[0] / l * r, -(b[1] + d[1] / l * r), b[2] + d[2] / l * r];
            met.push(at);
            // KH1 leaves an attack that already clashed this pass alone (`+4`); a melee attack
            // is only counted.
            if !shot.free || rd::<u32>(shot.obj + 4) & 0x30 != 0 {
                static MELEE: AtomicU32 = AtomicU32::new(0);
                if MELEE.fetch_add(1, Ordering::Relaxed) < 100 {
                    log(&format!("Reflect barrier met enemy {} {:#x} of {:#x}: counted", if shot.free { "shot" } else { "attack" }, shot.obj, shot.owner));
                }
                continue;
            }
            let obj = new_attack(sora, free_bit(), base + ATTACK_DATA_RVA + rows[0].data as usize);
            if obj < 0x10000 {
                continue;
            }
            let c = [at[0], at[1], at[2], 1.0f32];
            wr(obj + 0x10, c);
            wr(obj + 0x40, c);
            wr(obj + 0x70, c);
            wr(obj + 0xC, shapes[ai].radius);
            // The barrier is a guard: it stands whatever meets it.
            wr(obj + 0x68, GUARD_PRIORITY);
            let res = half(shot.obj, obj);
            free(obj);
            let back = rd::<u32>(shot.obj) & 0x0100_0000 != 0;
            static MET: AtomicU32 = AtomicU32::new(0);
            if MET.fetch_add(1, Ordering::Relaxed) < 200 {
                log(&format!(
                    "Reflect barrier met enemy {} {:#x} of {:#x}: result {res:#x}, {}",
                    if shot.free { "shot" } else { "attack" },
                    shot.obj,
                    shot.owner,
                    if back { "turned back" } else { "not turned back" }
                ));
            }
        }
        met
    }
}

/// Damage of the last landed hit (set by `probe::hit` from the record).
pub static LAST_DAMAGE: AtomicU32 = AtomicU32::new(0);
/// The foe a KH2 hit is being landed on, and each foe's damage fraction left over (in
/// 1/10000ths of a point).
static TARGET: AtomicUsize = AtomicUsize::new(0);
/// The hit being landed is one of KH2's Magic-based ones.
static MAGIC: AtomicBool = AtomicBool::new(false);
static CARRY: std::sync::LazyLock<std::sync::Mutex<HashMap<usize, i64>>> = std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

/// Lands one hit with KH1's contact code, through a KH1 attack object made from Sora's row.
/// Returns the damage written to the hit record, if one was made. `drive` is the KH2 ATKP row's
/// drive value (byte 0x29), which the hit adds to the Drive gauge (`gauge::hit`, through
/// `probe::hit` and `DRIVE`).
unsafe fn land(sora: usize, part: &Part, at: &Shape, row: Row, power: u16, spell: bool, drive: u8) -> Option<u32> {
    DRIVE.store(drive as u32, Ordering::Relaxed);
    let r = land_hit(sora, part, at, row, power, spell);
    DRIVE.store(NO_DRIVE, Ordering::Relaxed);
    r
}

/// While KH2's hit is landed: its ATKP drive value (`NO_DRIVE` otherwise: a hit of KH1's own,
/// `probe::hit` gives it KH1's).
pub static DRIVE: AtomicU32 = AtomicU32::new(NO_DRIVE);
pub const NO_DRIVE: u32 = u32::MAX;

/// KH1's own attack data for a spell (Fire 0, Blizzard 1, Thunder 2, Cure 3, Gravity 4, Stop 5,
/// Aero 6) at a level (1..3): the offset in the attack data table (`docs/kh1-attack-elements.md`:
/// the per-level entries from 0x5F28, 0x70 apart, in spell order). Its element (Fire 2, Blizzard
/// 3, Thunder 4) is what makes the hit fire, ice or thunder against the target's resistances
/// (the user: Wisdom's Fire, Blizzard and Thunder did physical damage). Logged once.
/// Whether attack data at `data` is one of KH1's spells' (the seven spells' three levels from
/// 0x5F28, `kh1_spell_data`): a KH1 spell's hit, for its drive value (`probe::hit`).
pub fn is_kh1_spell_data(data: usize) -> bool {
    let start = BASE.load(Ordering::Relaxed) + ATTACK_DATA_RVA + 0x5F28;
    (start..start + 21 * 0x70).contains(&data)
}

pub fn kh1_spell_data(spell: u8, level: u8) -> Option<i32> {
    if spell > 6 || !(1..=3).contains(&level) {
        return None;
    }
    let off = 0x5F28 + (spell as i32 * 3 + level as i32 - 1) * 0x70;
    static LOGGED: AtomicBool = AtomicBool::new(false);
    if !LOGGED.swap(true, Ordering::Relaxed) {
        let base = BASE.load(Ordering::Relaxed);
        let words = |o: i32| (0..12).map(|k| format!("{:x}", unsafe { rd::<u32>(base + ATTACK_DATA_RVA + o as usize + k * 4) })).collect::<Vec<_>>().join(" ");
        log(&format!("KH1 spell attack data (level 1): Fire [{}] | Blizzard [{}] | Thunder [{}]", words(0x5F28), words(0x6078), words(0x61C8)));
    }
    Some(off)
}

unsafe fn land_hit(sora: usize, part: &Part, at: &Shape, row: Row, power: u16, spell: bool) -> Option<u32> {
    let base = BASE.load(Ordering::Relaxed);
    let free_bit: unsafe extern "system" fn() -> u32 = std::mem::transmute(base + FREE_BIT_RVA);
    let new_attack: unsafe extern "system" fn(usize, u32, usize) -> usize = std::mem::transmute(base + NEW_ATTACK_RVA);
    let contact: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + LAND_RVA);
    let free: unsafe extern "system" fn(usize) = std::mem::transmute(base + FREE_ATTACK_RVA);
    let bit = free_bit();
    let obj = new_attack(sora, bit, base + ATTACK_DATA_RVA + row.data as usize);
    if obj < 0x10000 {
        return None;
    }
    // As the Attack command's row does (`0x1402A17F0`): its effect and sound bytes (not for a
    // spell's data: no swing sound).
    if !spell {
        wr(obj, rd::<u32>(obj) | 0x400000);
        wr(obj + 0xA8, row.fx);
    }
    // KH2's hit records decide what may hit again, not KH1's hit-once bit.
    let saved = rd::<u32>(obj + 0x54);
    wr(obj + 0x54, 0u32);
    let c = [at.center[0], -at.center[1], at.center[2], 1.0f32];
    wr(obj + 0x10, c);
    let mut entry = [0u8; 0x40];
    let e = entry.as_mut_ptr() as usize;
    wr(e, (rd::<u32>(obj) >> 12) & 3);
    wr(e + 8, sora as u64);
    wr(e + 0x18, obj as u64);
    wr(e + 0x20, 1u32);
    wr(e + 0x24, at.radius);
    wr(e + 0x28, at.radius);
    wr(e + 0x30, c);
    LAST_DAMAGE.store(u32::MAX, Ordering::Relaxed);
    TARGET.store(std::ptr::read_unaligned(part.entry.as_ptr().add(8) as *const u64) as usize, Ordering::Relaxed);
    POWER.store(power.max(1) as u32, Ordering::Relaxed);
    contact(e, part.entry.as_ptr() as usize);
    POWER.store(0, Ordering::Relaxed);
    wr(obj + 0x54, saved);
    free(obj);
    let d = LAST_DAMAGE.load(Ordering::Relaxed);
    (d != u32::MAX).then_some(d)
}

/// `probe::hit`: a hit record was just made; while KH2's hit is landed, its damage is KH1's
/// scaled by KH2's power.
pub unsafe fn scale_record(rec: usize) {
    let p = POWER.load(Ordering::Relaxed);
    if p == 0 || rec < 0x10000 {
        return;
    }
    // A KH2 launch: KH1's launch reaction (kind 5) with KH2's numbers once it runs.
    if let Some(l) = PENDING.lock().ok().and_then(|g| *g) {
        wr(rec + 0xC, (rd::<u32>(rec + 0xC) & !0x1F) | 5);
        if let Ok(mut v) = LAUNCHES.lock() {
            let now = takeover_ms();
            v.retain(|(r, _, t)| *r != rec && now.saturating_sub(*t) < LAUNCH_FRESH_MS);
            v.push((rec, l, now));
        }
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 40 {
            log(&format!("KH2 launch asked: record {rec:#x} made a launch (up {:.0} in {:.0} frames, sideways {:.1}/frame)", l.h, l.t, l.speed));
        }
    }
    if HOLD.load(Ordering::Relaxed) {
        HELD_REC.store(rec, Ordering::Relaxed);
        let w = rd::<f32>(rec + 0x1C);
        wr(rec + 0x10, [0.0f32, 0.0, 0.0, w]);
    }
    // KH1's damage times KH2's power (and the ini's `damage_scale`), the fraction carried to
    // the next hit on the same foe (the user: Explosion did too little: its 25% hits on a
    // 6-damage swing were floored from 1.5 to 1, a third of the burst lost).
    let d = rd::<i32>(rec + 8);
    let set = super::settings::get();
    let mut factor = set.damage_scale;
    // KH2's Magic-based hits (ATKP stat source 1: Explosion's, Magnet Burst's; the user: they
    // scale with magic): KH1 has no Magic stat, so Sora's max MP stands in, +10% a point, times
    // the ini's `magic_scale`.
    if MAGIC.load(Ordering::Relaxed) {
        let base = BASE.load(Ordering::Relaxed);
        let mp = super::menu::save_block(base).map_or(0, |s| rd::<u8>(s + 8)) as f32;
        factor *= (1.0 + 0.1 * mp.min(30.0)) * set.magic_scale;
    }
    let scale = (factor * 100.0).round().max(1.0) as i64;
    let target = TARGET.load(Ordering::Relaxed);
    let mut carry = CARRY.lock().ok();
    let before = carry.as_ref().and_then(|c| c.get(&target).copied()).unwrap_or(0);
    let total = d.max(0) as i64 * p as i64 * scale + before;
    let scaled = (total / 10_000).max(1);
    if let Some(c) = carry.as_mut() {
        if c.len() > 64 {
            c.clear();
        }
        c.insert(target, (total - scaled * 10_000).max(0));
    }
    let scaled = scaled.min(i32::MAX as i64) as i32;
    wr(rec + 8, scaled);
    LAST_DAMAGE.store(scaled as u32, Ordering::Relaxed);
}

/// A KH2 launch for KH1's launch reaction: apex height above the start, frames to it, and the
/// starting horizontal speed (KH1 units, 60 Hz frames). `docs/kh1-launch.md`.
#[derive(Clone, Copy, Debug)]
struct Launch {
    h: f32,
    t: f32,
    speed: f32,
}

impl Launch {
    /// From an ATKP launch's knockback values (up speed, horizontal speed; both 0: the
    /// defaults), flown as KH2's blow flies (`character_core::reaction::Blow`), to its apex.
    fn kh2(k1: i16, k2: i16, p: &character_core::reaction::ReactionParams) -> Launch {
        let (up, speed) = if k1 == 0 && k2 == 0 { (p.blow_up, p.blow_speed) } else { (k1 as f32, k2 as f32) };
        let mut b = character_core::reaction::Blow::new([0.0, speed.abs()], up, p);
        let (mut y, mut h, mut t, mut frames) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for _ in 0..200 {
            let d = b.tick(2.0, p);
            frames += 2.0;
            y += d[1];
            if y > h {
                h = y;
                t = frames;
            }
            if d[1] <= 0.0 {
                break;
            }
        }
        Launch { h: h.max(1.0), t: t.max(2.0), speed: speed.abs() }
    }
}

/// KH2 launches sent farther than their ATKP's knockback (up, sideways): Explosion's last blast
/// (ATKP 912: reaction 11, up 15, sideways 30) left the foe beside Sora (the user: "the finishing
/// hitbox on explosion doesn't send the enemies far away enough to give them a chance to
/// recover"): twice KH2's default blow's distance, and a third higher than its height (the user,
/// s226: "the launch distance is good but now can you have the enemies launch a little higher").
const FARTHER: [(u16, (i16, i16)); 1] = [(912, (32, 60))];

fn farther(attack: u16) -> Option<(i16, i16)> {
    FARTHER.iter().find(|(id, _)| *id == attack).map(|(_, k)| *k)
}

/// How long a holding blow keeps its foe beside Sora (frames), how far out and how high.
const HOLD_FRAMES: u32 = 24;
const HOLD_REACH: f32 = 90.0;
const HOLD_UP: f32 = 30.0;
/// The hit being landed holds the foe in place (its record's push direction is zeroed).
static HOLD: AtomicBool = AtomicBool::new(false);
static HELD_REC: AtomicUsize = AtomicUsize::new(0);
static PENDING: std::sync::Mutex<Option<Launch>> = std::sync::Mutex::new(None);
/// Records made with a KH2 launch: (record, launch, when).
static LAUNCHES: std::sync::Mutex<Vec<(usize, Launch, u64)>> = std::sync::Mutex::new(Vec::new());
/// How long a KH2 launch waits for KH1's launch reaction (it runs within a frame or two).
const LAUNCH_FRESH_MS: u64 = 250;
static ORIG_LAUNCH: AtomicUsize = AtomicUsize::new(0);
/// KH1's launch reaction `0x1402B6390 (actor, record)`.
const LAUNCH_RVA: Rva = Rva(0x2B6390);
const LAUNCH_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x60];

fn takeover_ms() -> u64 {
    static T0: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    T0.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64
}

/// KH1's launch, then KH2's height, rise time and horizontal speed for our records, when the
/// enemy did launch (its launch motion 0x52 / 0x5C, air state 2).
unsafe extern "system" fn launched(actor: usize, rec: usize) {
    let f: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_LAUNCH.load(Ordering::Acquire));
    f(actor, rec);
    // Only a launch asked just now: KH1 reuses its hit records, so one left over (a boss that
    // can't be launched) would otherwise land on a later hit's enemy.
    let now = takeover_ms();
    let ours = LAUNCHES.lock().ok().and_then(|mut v| {
        v.retain(|(_, _, t)| now.saturating_sub(*t) < LAUNCH_FRESH_MS);
        v.iter().position(|(r, _, _)| *r == rec).map(|i| v.remove(i).1)
    });
    let Some(l) = ours else {
        static M: AtomicU32 = AtomicU32::new(0);
        if M.fetch_add(1, Ordering::Relaxed) < 20 {
            log(&format!("KH1 launch reaction on {actor:#x} for record {rec:#x} (not a KH2 launch)"));
        }
        return;
    };
    let motion = rd::<u16>(actor + 0x3EA);
    let state = rd::<i32>(actor + 0x70);
    static N: AtomicU32 = AtomicU32::new(0);
    if !matches!(motion, 0x52 | 0x5C) || state != 2 {
        if N.fetch_add(1, Ordering::Relaxed) < 30 {
            log(&format!("KH2 launch on {actor:#x}: KH1 didn't launch it (motion {motion:#x}, state {state})"));
        }
        return;
    }
    let dir = rd::<[f32; 3]>(actor + 0x20);
    let n = (dir[0] * dir[0] + dir[2] * dir[2]).sqrt();
    let (dx, dz) = if n > 1e-3 { (dir[0] / n, dir[2] / n) } else { (0.0, 0.0) };
    wr(actor + 0x3B0, 0.0f32);
    wr(actor + 0x3B8, l.t);
    wr(actor + 0x3BC, -l.h);
    wr(actor + 0x3C0, [dx * l.speed, 0.0, dz * l.speed, l.speed]);
    if N.fetch_add(1, Ordering::Relaxed) < 30 {
        log(&format!("KH2 launch on {actor:#x}: up {:.0} in {:.0} frames, sideways {:.1}/frame", l.h, l.t, l.speed));
    }
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + LAUNCH_RVA, &LAUNCH_PROLOGUE, launched as *const () as usize, &ORIG_LAUNCH, "enemy launch reaction");
    hook::install(base + EXEC_RVA, &EXEC_PROLOGUE, exec as *const () as usize, &ORIG_EXEC, "execute attack row");
    hook::install(base + COMMAND_STEP_RVA, &COMMAND_STEP_PROLOGUE, command as *const () as usize, &ORIG_COMMAND, "command step");
    hook::install(base + RUN_ENTRY_RVA, &RUN_ENTRY_PROLOGUE, run_entry as *const () as usize, &ORIG_RUN_ENTRY, "menu entry run (spells only in KH2's windows)");
}

/// KH1's menu runs a chosen entry (`0x1402912F0 (menu, entry)`, `docs/kh1-drive.md`): from the
/// Magic list, a party target list (type 4: its spell is the parent list's cursor entry) or a
/// shortcut. Returning 0 without writing the command record leaves the menu where it is.
const RUN_ENTRY_RVA: Rva = Rva(0x2912F0);
/// KH1's item command type (the log: "command 0x3 (02000803)", a Potion).
const ITEM_COMMAND_TYPE: u8 = 3;
/// `mov [rsp+0x10], rbx; mov [rsp+0x18], rbp; push rsi; push rdi; push r12`.
const RUN_ENTRY_PROLOGUE: [u8; 14] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x6C, 0x24, 0x18, 0x56, 0x57, 0x41, 0x54];
const RECORD_RVA: Rva = Rva(0x28D7D0);
/// The menu chain's top (`[top - 8]` is the list under it), and its first slot (the top level).
const CHAIN_RVA: Rva = Rva(0x2D54450);
const MENU_ROOT_RVA: Rva = Rva(0x2D543F0);
/// The Summon list (menu 3, `docs/kh1-drive.md`; on the Drive page it holds the forms).
const SUMMON_MENU_RVA: Rva = Rva(0x5296A8);
static ORIG_RUN_ENTRY: AtomicUsize = AtomicUsize::new(0);

/// KH2's menu takes a spell only when the move playing can be cut by one; before that the
/// press does nothing and the Magic menu stays open (the user: "in kh2 the game won't let you
/// select a magic until the game lets you, keeping you in the menu until you back out"). So
/// a spell is never chosen to wait, and an Attack pressed meanwhile is the combo's.
unsafe extern "system" fn run_entry(menu: usize, entry: i32) -> u64 {
    let f: unsafe extern "system" fn(usize, i32) -> u64 = std::mem::transmute(ORIG_RUN_ENTRY.load(Ordering::Acquire));
    if spell_not_yet(menu, entry) {
        return 0;
    }
    f(menu, entry)
}

/// The list under the menu chain's top (a party target list's parent).
unsafe fn entry_parent(base: usize) -> Option<usize> {
    let top = rd::<u64>(base + CHAIN_RVA) as usize;
    if top < 0x10008 {
        return None;
    }
    let parent = rd::<u64>(top - 8) as usize;
    (parent > 0x10000).then_some(parent)
}

unsafe fn spell_not_yet(menu: usize, entry: i32) -> bool {
    if menu < 0x10000 || !(0..16).contains(&entry) {
        return false;
    }
    // Page 2's Drive row in a form: Revert at once, no list (`drive::revert_row`).
    if rd::<u32>(menu) != 4 && rd::<u32>(menu + 8) >> entry & 1 == 0 && super::drive::is_drive_row(menu, rd::<u8>(menu + 0x14 + entry as usize)) {
        let sora = rd::<u64>(BASE.load(Ordering::Relaxed) + PARTY_RVA) as usize;
        if super::drive::revert_row(sora) {
            return true;
        }
    }
    let disabled = rd::<u32>(menu + 8) >> entry & 1 != 0;
    let base = BASE.load(Ordering::Relaxed);
    let record_of: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base + RECORD_RVA);
    let id = if rd::<u32>(menu) == 4 {
        let Some(parent) = entry_parent(base) else { return false };
        rd::<u8>(parent + 0x14 + (rd::<i32>(parent + 0x34) & 0xF) as usize)
    } else {
        rd::<u8>(menu + 0x14 + entry as usize)
    };
    let record = record_of(id as u32);
    if record < 0x10000 {
        return false;
    }
    let (kind, arg, sub) = (rd::<u8>(record + 6), rd::<u8>(record + 7), rd::<i8>(record + 8));
    // Anti Form: no magic (the Drive list and its Revert are magic records too), items or
    // summons (Summon, command 6, and the summon-out 0x4F; the summons themselves type 6): KH1's
    // "can't", the menu stays (`drive::fill` greys them; this in case one runs all the same).
    if super::form::in_anti() && (matches!(kind, 2 | ITEM_COMMAND_TYPE | 6) || matches!(id, 6 | 0x4F)) {
        let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
        sound(4);
        static A: AtomicU32 = AtomicU32::new(0);
        if A.fetch_add(1, Ordering::Relaxed) % 30 == 0 {
            cmd_log(format!("command {id:#x} (type {kind:#x}) refused: not in Anti Form"));
        }
        return true;
    }
    // In one of KH1's moves at its cut frame (`takeover::kh1_cut_window`: KH1 Fire from frame
    // 49): KH1's menu takes nothing while Sora can't act (`0x1402912F0`: its can-act test lets
    // only plain spells through; the user: Attack only came out when Fire ended, the Drive list
    // was greyed), so Attack and a Drive form are taken here, as KH2's.
    // (KH2 has him again while KH1's leftover cast still plays, `takeover::cut_left`: KH1 greys
    // its menu then, so Attack is KH2's attack button from here; the user: an Attack waited 21-25
    // frames after the form's KH1 spell.)
    if kind == 1 && takeover::cut_left() {
        PRESSED.store(true, Ordering::Relaxed);
        cmd_log("Attack while KH1's cut cast plays on: KH2's attack".to_string());
        return true;
    }
    // During one of KH1's spells (any, in a form too): Attack is taken here as KH2's, its cut
    // then decided by `takeover` (once the spell is out, or a form's spell at its cast motion's
    // input frame; the combo goes on from the spell's step). KH1's menu refused it while it holds
    // him (the user: "kh1s magic animation states are hard coded to where you can't interrupt
    // them into an attack ... make it so that kh1s magic animation states are able to be
    // interrupted by an attack and are part of a combo string"). Pressed too early it is dropped
    // (no buffer).
    if kind == 1 && takeover::kh1_casting() && takeover::kh1_spell_now().is_some() {
        takeover::ATTACK_AFTER.store(true, Ordering::Relaxed);
        cmd_log("Attack during KH1's spell: KH2's, to cut it once it may be".to_string());
        return true;
    }
    if takeover::kh1_cut_window() {
        // (Not in KH1's own combo, whose window is open for a Drive only: its Attack is KH1's.)
        if kind == 1 && !super::stylecombo::active() {
            takeover::ATTACK_AFTER.store(true, Ordering::Relaxed);
            cmd_log("Attack in KH1's move's cut window: KH2 takes him for the combo".to_string());
            return true;
        }
        if kind == 2 && arg >= super::drive::FORM_ARG && !super::limits::ours(arg) && super::drive::on() && super::form::accept_now() {
            let sora = rd::<u64>(base + PARTY_RVA) as usize;
            super::drive::chosen(sora, (arg - super::drive::FORM_ARG) as usize);
            // The menu back to its top, as KH1 leaves it after a command.
            wr(base + CHAIN_RVA, (base + MENU_ROOT_RVA) as u64);
            cmd_log("Drive in KH1's move's cut window: KH2 takes him to change".to_string());
            return true;
        }
    }
    // Disabled entries: KH1's own "can't".
    if disabled {
        return false;
    }
    // Limit Form's limits: taken once KH2's move takes a command (the list stays open till then);
    // the Triangle follow-up only on a fresh press while it is offered (`limits`).
    if kind == 2 && super::limits::ours(arg) {
        return super::limits::not_yet(arg);
    }
    // No magic in Valor Form (KH2's Valor can't cast): the "can't" sound, the menu stays.
    // (Nor in Limit Form: KH2's has its Limit commands there instead, `limits`; a KH1 spell from
    // a shortcut is refused here. With KH2's Limit menu off, `limits::KH2_LIMITS`, Magic's list
    // and the shortcuts there are KH1's limits instead, `limits::fill_kh1`, taken above.)
    if kind == 2 && arg < super::drive::FORM_ARG && (super::form::in_valor() || super::form::in_limit()) {
        let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
        sound(4);
        cmd_log(format!("spell {arg} refused: no magic in {}", if super::form::in_limit() { "Limit Form" } else { "Valor Form" }));
        return true;
    }
    // A form from the Drive list: only on a frame Sora can change (`form::accept_now`: KH2's,
    // or a KH1 move's cut frame reached), the same as a spell (the user: no queue).
    if kind == 2 && arg >= super::drive::FORM_ARG && !super::limits::ours(arg) && super::drive::on() {
        if super::form::accept_now() {
            return false;
        }
        static D: AtomicU32 = AtomicU32::new(0);
        if D.fetch_add(1, Ordering::Relaxed) % 30 == 0 {
            cmd_log("Drive not taken yet: Sora can't change on this frame; the menu stays open".to_string());
        }
        return true;
    }
    if !super::settings::get().commands || !takeover::ON.load(Ordering::Relaxed) || !takeover::kh2_has() || takeover::kh1_casting() {
        return false;
    }
    // The Items and Summon lists as the Magic list (the user: "the item and summon sub menus
    // should follow the same rules as the magic sub menu where if sora isn't available to input
    // that action then the submenu doesn't go away with the confirm button"): KH2's move playing
    // can't be cut by a command (`can_take_command`, what `command` would have made it wait
    // for), or one KH1 refuses in the air: the press does nothing and the list stays open (no
    // waiting command). A target list opening is let through; its own entry is the press.
    let summon_list = base + SUMMON_MENU_RVA;
    let from_summons = menu == summon_list || (rd::<u32>(menu) == 4 && entry_parent(base) == Some(summon_list));
    if (kind == ITEM_COMMAND_TYPE || from_summons) && (rd::<u32>(menu) == 4 || sub <= 0) {
        let what = if kind == ITEM_COMMAND_TYPE { "item" } else { "summon" };
        let sora = rd::<u64>(base + PARTY_RVA) as usize;
        // (`command`'s own air refusal, before the menu closes.)
        if !matches!(kind, 0 | 1 | 2 | 99 | 0x47) && (takeover::kh2_airborne() || (sora > 0x10000 && rd::<i32>(sora + 0x70) == 2)) {
            let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
            sound(4);
            cmd_log(format!("{what} (command {kind:#x}) refused in the air; the list stays open"));
            return true;
        }
        if takeover::with_kh2(|c, _| c.can_take_command()) == Some(false) {
            static M: AtomicU32 = AtomicU32::new(0);
            if M.fetch_add(1, Ordering::Relaxed) % 30 == 0 {
                cmd_log(format!("{what} (command {kind:#x}) not taken yet: KH2's move (motion {}) can't be cut by a command now; the list stays open", takeover::KH2_MOTION.load(Ordering::Relaxed)));
            }
            return true;
        }
        return false;
    }
    // Only spells (Reflect's too); a target list opening is no cast.
    if kind != 2 || (rd::<u32>(menu) != 4 && sub > 0) {
        return false;
    }
    let free = takeover::with_kh2(|c, _| c.can_cast());
    if free != Some(false) {
        return false;
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) % 30 == 0 {
        cmd_log(format!("spell {arg} not taken yet: KH2's move (motion {}) can't be cut by magic now; the menu stays open", takeover::KH2_MOTION.load(Ordering::Relaxed)));
    }
    true
}
