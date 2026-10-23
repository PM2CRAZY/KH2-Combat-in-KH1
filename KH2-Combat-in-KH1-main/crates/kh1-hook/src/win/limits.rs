//! Limit Form's limits. `docs/drive-model.md`, "Limit Form in the KH1 port", has what is KH2's and
//! what is ours.
//!
//! **For now KH1's limits, not KH2's** (the user: "lets comment the KH2 limits, and for the moment
//! put in the equivalent KH1 limits"). KH2's Limit menu (s291 / s294, below) is switched off by
//! `KH2_LIMITS` (false): its code is all kept and still compiled, only never switched on
//! (`install` stops before it, so `ON` stays false and every entry point of it returns at once);
//! `true` brings it back as it was. In its place KH1's own limits (`KH1_LIMITS`; section "KH1's
//! limits in Limit Form" at the end), in a Limit menu of the same kind (the user: "Limit form
//! should have all the KH1 limits without having to have the abilities equipped ... access via
//! the limit menu (similar to KH2's limit menu) and they should NOT be activated, or even
//! prompted via the triangle prompts"): Magic reads "Limit" and lists all five, KH1's names,
//! equipped or not (`fill_kh1`); the shortcuts are KH2 FM's Limit Form layout
//! (`SHORTCUT_LIMITS`); KH1's Triangle offer is skipped in every form, Limit Form too
//! (`specials_prompt`); KH1 runs a chosen one as its own (`combat::command`, `takeover`).
//!
//! KH2's Limit menu, when `KH2_LIMITS` is on:
//!
//! - **Menu.** In Limit Form the top's Magic row reads "Limit" (record 2's word, `word`) and its
//!   list (the magic list, `0x140529638`) holds our four records 0x82..0x85 (spell-type records
//!   with arguments `LIMIT_ARG + k`, written over empty ones as `drive`'s forms are): Sonic Rave,
//!   Last Arcanum, Strike Raid and Infinity, KH2's names. KH1's magic list shows no costs, so none
//!   is shown. A row is greyed when it can't be used now (in the air: the commands' Land flag; MP
//!   short; a limit running).
//! - **MP.** KH2's costs (03system `cmd`: 60, 75, 65, 80 of KH2's 100) are each one of KH1's bars:
//!   the mod's KH2 spells already cost KH1's own 1 MP each whatever KH2's cost (`reflect::COST`),
//!   and KH2's cost / 100 rounded up gives the same bar. Short of it, the limit can't be used (the
//!   menu greys it; KH2's own rule for a cost above what is left isn't traced, `docs`).
//! - **Running.** A limit chosen runs when KH2's move can take a command (`can_take_command`;
//!   before that the list stays open, as for a spell): KH1's bar paid, the form model's script
//!   (`limi`) started at its limit trigger with the command (`kh2_magic::Magic::limit`). The
//!   script drives Sora (`LimitWorld`): it starts and chains his motions (`Character::play`,
//!   `push_scripted`), turns him, pushes him back (`Character::hit_back`), plays combo rows
//!   (Infinity's, `trap_obj_party_attack`: the row's motion), and offers the follow-ups. The
//!   motions carry KH2's hits (the Keyblade's ATKP rows), invulnerability, effects and sounds,
//!   through the controller as for every move; the script's own attacks (Strike Raid's thrown
//!   Keyblade, Infinity's shots) hit KH1's enemies through the spell hit records, as Wisdom's
//!   shots do (`formmagic`).
//! - **Strike Raid's Keyblade.** The script takes Sora's Keyblade from his hand as an object of its
//!   own (`WEAPON`; `kh2_magic::Loose` while it flies) and flies it; KH1's own Keyblade (its
//!   weapon bones) is drawn there (`pose_hook`, `leftblade::float_grip`), spinning on the
//!   Keyblade's own motion 235, until the catch puts it back in his hand.
//! - **Weapon anchors.** The form's effects on its Keyblade (0x4003.. codes: Last Arcanum's glow,
//!   Infinity's charge, and the thrown Keyblade's trail on its blade bone) sit on KH2's blade bone
//!   where KH1 draws the Keyblade (`drawn_blade`), its turn included, as KH2's own weapon anchors
//!   are (`kh2fx::form_bone`).
//! - **Triangle.** A follow-up the script offers (`trap_command_override`) or a motion carries
//!   (KH2's range 20) is shown in KH1's own Triangle box, the top menu's fourth entry (record
//!   0x86, its name the follow-up's), written after KH1's context fill (`context_fill`). Triangle
//!   pressed while it shows runs it (the script's trigger 23), once per window.
//! - **KH1's limits in a form.** KH1's own limits (Sonic Blade, Ars Arcanum, Strike Raid, ...)
//!   are offered by KH1 in that same Triangle box (`0x14028AF60`, called once from the context
//!   fill). In every Drive Form that call is skipped (`specials_prompt`), so they never show
//!   there; in Limit Form they come from its Limit menu and shortcuts instead. The command step
//!   refuses them in every other form (`combat::command`, `kh1_limit_allowed`).

use super::addr::Rva;
use super::reflect::{world_now, KhWorld, SORA};
use super::{combat, hook, log, pose};
use character_core::{Character, Output, WindowKind};
use glam::{Mat4, Quat, Vec3};
use kh2_formats::battle::Battle;
use kh2_magic::{Event, Magic, ObjId, SpellFile, World};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// KH2's Limit menu in Limit Form (s291 / s294: Sonic Rave, Last Arcanum, Strike Raid, Infinity
/// from the form's `limi` script). Off for now, KH1's equivalent limits in its place (the user:
/// "lets comment the KH2 limits, and for the moment put in the equivalent KH1 limits"). The code
/// stays compiled; `true` brings the menu back (and takes KH1's limits out of Limit Form again).
pub const KH2_LIMITS: bool = false;

/// A limit: its 03system `cmd`, KH2's name, our command record and word, and KH2's MP cost.
struct Limit {
    command: u16,
    name: &'static str,
    record: u8,
    word: u16,
    kh2_mp: u8,
}

/// KH2's Limit list, in its order. (Infinity is 03system's 0x2AB, KH2 FM's Ragnarok under its
/// PC name.)
const LIMITS: [Limit; 4] = [
    Limit { command: 698, name: "Sonic Rave", record: 0x82, word: 0x1FA, kh2_mp: 60 },
    Limit { command: 701, name: "Last Arcanum", record: 0x83, word: 0x1F9, kh2_mp: 75 },
    Limit { command: 704, name: "Strike Raid", record: 0x84, word: 0x1F8, kh2_mp: 65 },
    Limit { command: 683, name: "Infinity", record: 0x85, word: 0x1F7, kh2_mp: 80 },
];
/// The follow-ups (03system `cmd`, KH2's name).
const FOLLOW_UPS: [(u16, &str); 9] = [(699, "Rave"), (700, "Finish"), (702, "Bash"), (703, "The End"), (705, "Raid"), (706, "Judgment"), (684, "Shoot"), (685, "Impact"), (686, "Giga Impact")];
/// The Triangle box's record and word.
const FOLLOW_RECORD: u8 = 0x86;
const FOLLOW_WORD: u16 = 0x1F6;
/// Our records' arguments (KH1's spells 0..6, Reflect 7, the Drive forms from 0x10).
pub const LIMIT_ARG: u8 = 0x20;
pub const FOLLOW_ARG: u8 = 0x28;
/// KH1's MP cost of every limit (one bar, in the payment's hundredths; `reflect::COST`).
const COST: i32 = 100;
/// `00objentry` id of `P_EX100_KH1F` (its script checks it is Limit Form).
const ENTRY_ID: u32 = 0x95D;
/// Sora's Keyblade as the script's object (Strike Raid throws it; the enemies are from
/// `reflect::FOE0`).
const WEAPON: ObjId = 2;
/// The Keyblade's own motion while it flies (`W_EX010_KH1F`'s 235, its spin).
const THROWN_MOTION: u16 = 235;
/// Hit-record keys of the limits' own attacks (apart from Wisdom's 1 << 59, Reflect's 1 << 60).
const KEY: u64 = 1 << 58;
/// The frame trigger of a limit motion the script takes as its trigger 17 (Infinity's 247 / 248
/// carry 7 [100, 0], their shots; no other limit motion has a 7). The controller's events keep
/// no parameters, so these are passed as the motions have them.
const SHOTS_TRIGGER: u8 = 7;
const SHOTS_SCRIPT_TRIGGER: i32 = 17;
const SHOTS_PARAMS: [i32; 2] = [100, 0];
/// A follow-up shown this many frames after the script last offered it (KH1's menu fill runs on
/// its own step; the script offers it every frame its window is open).
const OFFER_FRAMES: u32 = 2;

const MENU_RVA: Rva = Rva(0x529600);
const MAGIC_MENU_RVA: Rva = Rva(0x529638);
const RECORD_RVA: Rva = Rva(0x28D7D0);
const PRESSED_RVA: Rva = Rva(0x298D4A8);
/// KH1's logical Triangle press (`docs/kh1-drive.md`, Input).
const TRIANGLE: u32 = 0x400_0000;
/// The top fill's tail jump into the context fill (`0x140292090` → `jmp 0x140292760`), and the
/// context fill's call of KH1's limits' Triangle offer (`0x14028AF60`, its one caller).
const CONTEXT_JMP_RVA: Rva = Rva(0x292320);
const CONTEXT_FILL_RVA: Rva = Rva(0x292760);
const SPECIALS_CALL_RVA: Rva = Rva(0x292B12);
const SPECIALS_RVA: Rva = Rva(0x28AF60);
const SUMMON: u8 = 6;
const SUMMON_OUT: u8 = 0x4F;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ON: AtomicBool = AtomicBool::new(false);
/// KH1 text for "Limit", the limits' names and the follow-ups'.
static TEXT: OnceLock<(usize, Vec<usize>, Vec<usize>)> = OnceLock::new();
/// Record 2's (Magic's) word, read at the first fill.
static MAGIC_WORD: AtomicU32 = AtomicU32::new(u32::MAX);
/// The follow-up offered now (`cmd`) and the frame count when it last was.
static OFFER: Mutex<Option<(u16, u32)>> = Mutex::new(None);
static FRAME: AtomicU32 = AtomicU32::new(0);
/// A limit chosen (index into `LIMITS`) and a follow-up taken, for the next step.
static CHOSEN: Mutex<Option<usize>> = Mutex::new(None);
static TAKEN: Mutex<Option<u16>> = Mutex::new(None);
static ORIG_SPECIALS: AtomicUsize = AtomicUsize::new(0);
static ORIG_CONTEXT: AtomicUsize = AtomicUsize::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}
fn base() -> usize {
    BASE.load(Ordering::Relaxed)
}

fn note(s: String) {
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 300 {
        log(&s);
    }
}

/// A command record argument of ours (a limit or the Triangle follow-up; or a row of KH1's
/// limits' menu, in or out of the form, so a stale one never reaches KH1's spell code).
pub fn ours(arg: u8) -> bool {
    (ON.load(Ordering::Relaxed) && ((LIMIT_ARG..LIMIT_ARG + LIMITS.len() as u8).contains(&arg) || arg == FOLLOW_ARG)) || kh1_menu_arg(arg).is_some()
}

fn follow_up_name(cmd: u16) -> Option<usize> {
    let (_, _, follow) = TEXT.get()?;
    FOLLOW_UPS.iter().position(|f| f.0 == cmd).map(|i| follow[i])
}

/// The follow-up shown now, if any.
fn offered() -> Option<u16> {
    let now = FRAME.load(Ordering::Relaxed);
    OFFER.try_lock().ok().and_then(|o| *o).filter(|(_, at)| now.wrapping_sub(*at) <= OFFER_FRAMES).map(|(c, _)| c)
}

// ---------------------------------------------------------------- the menu

/// Our records (spell-type, as `drive`'s forms), written over empty ones.
unsafe fn ensure_records(base: usize) -> bool {
    let record: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base + RECORD_RVA);
    let all = LIMITS.iter().enumerate().map(|(k, l)| (l.record, l.word, LIMIT_ARG + k as u8, 0x02u8)).chain(std::iter::once((FOLLOW_RECORD, FOLLOW_WORD, FOLLOW_ARG, 0x12u8)));
    for (id, word, arg, flags) in all {
        let r = record(id as u32);
        if r < 0x10000 {
            return false;
        }
        // (The Triangle record runs while Triangle is held, flag 0x10: KH1's step runs the
        // Triangle box only for such records; `not_yet` takes only a fresh press.)
        let want: [u8; 16] = [flags, 0x00, 0xFF, 0x00, word as u8, (word >> 8) as u8, 0x02, arg, 0x00, 0xFF, 0x01, 0x02, 0, 0, 0, 0];
        let have = rd::<[u8; 16]>(r);
        if have != want {
            if have != [0u8; 16] && have[4..8] != want[4..8] {
                static ONCE: AtomicBool = AtomicBool::new(false);
                if !ONCE.swap(true, Ordering::Relaxed) {
                    log(&format!("Limit Form: command record {id:#x} is in use ({have:02x?}): no Limit menu"));
                }
                return false;
            }
            wr(r, want);
        }
    }
    if MAGIC_WORD.load(Ordering::Relaxed) == u32::MAX {
        let m = record(2);
        if m > 0x10000 {
            MAGIC_WORD.store(rd::<u16>(m + 4) as u32, Ordering::Relaxed);
        }
    }
    true
}

/// Whether a limit can be chosen now (and why not): all four alike.
unsafe fn usable(actor: usize) -> Result<(), &'static str> {
    if script().is_none() {
        return Err("Limit Form's script not loaded");
    }
    if super::takeover::kh2_airborne() {
        return Err("in the air (KH2's Land flag)");
    }
    if running() {
        return Err("a limit is running");
    }
    if !super::reflect::can_pay(actor, COST) {
        return Err("MP short");
    }
    Ok(())
}

/// After KH1's battle-menu fill (`drive::fill`, first): in Limit Form the Magic row is "Limit"
/// and its list our limits.
pub unsafe fn fill(actor: usize) {
    if !KH2_LIMITS {
        return fill_kh1(actor);
    }
    let base = base();
    if !ON.load(Ordering::Relaxed) || base == 0 || !super::form::in_limit() || !ensure_records(base) {
        return;
    }
    let magic = base + MAGIC_MENU_RVA;
    let n = rd::<i32>(magic + 0x10);
    if !(0..=15).contains(&n) {
        return;
    }
    // (KH1's last entry, Summon or the summon-out one, stays last: the fourth row, page 2's Drive.)
    let last = (n > 0).then(|| rd::<u8>(magic + 0x14 + n as usize - 1)).filter(|e| matches!(*e, SUMMON | SUMMON_OUT));
    let (old4, old8) = (rd::<u32>(magic + 4), rd::<u32>(magic + 8));
    let mut grey = 0u32;
    for (k, l) in LIMITS.iter().enumerate() {
        wr(magic + 0x14 + k, l.record);
        if usable(actor).is_err() {
            grey |= 1 << k;
        }
    }
    let mut count = LIMITS.len();
    let (mut g4, mut g8) = (grey, grey);
    if let Some(e) = last {
        wr(magic + 0x14 + count, e);
        let b = (n - 1) as u32;
        g4 |= (old4 >> b & 1) << count;
        g8 |= (old8 >> b & 1) << count;
        count += 1;
    }
    wr(magic + 0x10, count as i32);
    wr(magic + 4, g4);
    wr(magic + 8, g8);
    if rd::<i32>(magic + 0xC) >= LIMITS.len() as i32 {
        wr(magic + 0xC, 0i32);
    }
    // The top's Magic row there and open (KH1 leaves it out with no spell).
    let top = base + MENU_RVA;
    wr(top + 0x14 + 1, 2u8);
    for off in [4usize, 8] {
        wr(top + off, rd::<u32>(top + off) & !2);
    }
    static LOGGED: AtomicU32 = AtomicU32::new(u32::MAX);
    if LOGGED.swap(grey, Ordering::Relaxed) != grey {
        let why: Vec<String> = (0..LIMITS.len()).map(|k| format!("{} {}", LIMITS[k].name, usable(actor).err().unwrap_or("usable"))).collect();
        log(&format!("Limit Form: the Limit menu ({})", why.join(", ")));
    }
}

/// A command's name by its word index: "Limit" for Magic's in Limit Form, our limits', the
/// follow-up shown. `None`: someone else's.
pub fn word(index: i32) -> Option<usize> {
    // (KH1's limits' menu: only Magic's row is ours, "Limit"; its rows have KH1's own words.)
    if KH1_ON.load(Ordering::Relaxed) && super::form::in_limit() && index as u32 == MAGIC_WORD.load(Ordering::Relaxed) {
        return KH1_TEXT.get().copied();
    }
    if !ON.load(Ordering::Relaxed) || !super::form::in_limit() {
        return None;
    }
    let (limit, names, _) = TEXT.get()?;
    if index as u32 == MAGIC_WORD.load(Ordering::Relaxed) {
        return Some(*limit);
    }
    if index == FOLLOW_WORD as i32 {
        return offered().and_then(follow_up_name).or_else(|| follow_up_name(FOLLOW_UPS[0].0));
    }
    LIMITS.iter().position(|l| l.word as i32 == index).map(|k| names[k])
}

/// The context fill (KH1's Triangle box), then ours: the follow-up offered now in the box.
unsafe extern "system" fn context_fill(a: usize) -> u64 {
    let f: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(ORIG_CONTEXT.load(Ordering::Acquire));
    let r = f(a);
    let base = base();
    if super::form::in_limit() && running() {
        if offered().is_some() {
            let top = base + MENU_RVA;
            wr(top + 0x17, FOLLOW_RECORD);
            for off in [4usize, 8] {
                wr(top + off, rd::<u32>(top + off) & !8);
            }
        }
    }
    r
}

/// KH1's limits' Triangle offer (`0x14028AF60`): none in any Drive Form (the user: "KH1 limits
/// that are equipped shouldn't even prompt in any drive form"), Limit Form too (the user: "they
/// should NOT be activated, or even prompted via the triangle prompts"): there its Limit menu and
/// shortcuts have them (`fill_kh1`). Their follow-ups are offered by the running limit's own
/// code (KH1's limit module puts them in the Triangle box itself), not by this call, so they stay.
unsafe extern "system" fn specials_prompt() {
    let f: unsafe extern "system" fn() = std::mem::transmute(ORIG_SPECIALS.load(Ordering::Acquire));
    if !super::form::in_form() {
        return f();
    }
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) == 0 {
        log("KH1's limits: not offered on Triangle in a Drive Form (Limit Form has them in its Limit menu)");
    }
}

/// KH1's menu runs one of our entries (`combat::spell_not_yet`): `true` keeps it from running.
/// A limit only once KH2's move takes a command (else the list stays open, as for a spell); the
/// follow-up only on a fresh Triangle press while it is offered.
pub unsafe fn not_yet(arg: u8) -> bool {
    // KH1's limits' menu (or shortcut): out of Limit Form a stale row is refused; in it, taken
    // once KH2's move takes a command (else the list stays open, as for a spell).
    if kh1_menu_arg(arg).is_some() {
        if !super::form::in_limit() {
            super::reflect::menu_sound(4);
            return true;
        }
        return super::takeover::with_kh2(|c, _| c.can_take_command()) == Some(false);
    }
    if arg == FOLLOW_ARG {
        let fresh = rd::<u32>(base() + PRESSED_RVA) & TRIANGLE != 0;
        return !(fresh && offered().is_some());
    }
    super::takeover::with_kh2(|c, _| c.can_take_command()) == Some(false)
}

/// One of our commands reached KH1's command step (`combat::command`): a limit chosen, or the
/// follow-up on Triangle.
pub unsafe fn command(actor: usize, arg: u8) {
    if arg == FOLLOW_ARG {
        let Some(cmd) = offered() else { return };
        if let Ok(mut t) = TAKEN.lock() {
            *t = Some(cmd);
        }
        if let Ok(mut o) = OFFER.lock() {
            *o = None;
        }
        note(format!("Limit Form: {} on Triangle", FOLLOW_UPS.iter().find(|f| f.0 == cmd).map_or("?", |f| f.1)));
        return;
    }
    let k = (arg - LIMIT_ARG) as usize;
    let Some(l) = LIMITS.get(k) else { return };
    if let Err(why) = usable(actor) {
        super::reflect::menu_sound(4);
        note(format!("Limit Form: {} refused ({why})", l.name));
        return;
    }
    if let Ok(mut c) = CHOSEN.lock() {
        *c = Some(k);
    }
    note(format!("Limit Form: {} chosen", l.name));
}

/// A limit chosen and not started yet (`takeover`: an Attack meanwhile doesn't take its window).
pub fn waiting() -> bool {
    CHOSEN.try_lock().map_or(false, |c| c.is_some())
}

// ---------------------------------------------------------------- the script

fn script() -> Option<&'static Vec<u8>> {
    pose::kh2()?.limit.as_ref()?.script.as_ref()
}

/// What the script asked of Sora this frame, for the next step.
#[derive(Default)]
struct Requests {
    start: Option<u16>,
    pushes: Vec<u16>,
    face: Option<f32>,
    /// A push back (`trap_obj_hitback`: direction x / z, distance).
    hitback: Option<([f32; 2], f32)>,
}

struct Runtime {
    magic: Magic,
    file: Option<usize>,
    /// Pushed motions waiting (0: idle, the chain's end).
    queue: Vec<u16>,
    /// The motion handed to the controller as the next (not started yet).
    fed: Option<u16>,
    req: Requests,
    /// After a follow-up is taken: no offers until the motion starts again (motion, frame).
    taken_in: Option<(u16, u32)>,
    /// Script effect id → our drawn effect, and whether its end was passed on.
    fx: HashMap<u64, (Option<u32>, bool)>,
    started: u32,
    /// A limit paid for, started with the script's next frame (`after_step`, with the world).
    pending: Option<usize>,
    /// KH2's motion and its frame at the last step.
    now: (u16, u32),
    hits: u32,
}

static RT: Mutex<Option<Runtime>> = Mutex::new(None);

fn runtime(g: &mut Option<Runtime>) -> &mut Runtime {
    g.get_or_insert_with(|| {
        let mut magic = Magic::new();
        let file = pose::kh2().and_then(|k| k.limit.as_ref()).and_then(|f| {
            let script = f.script.as_ref()?;
            let made = match f.archive.as_ref().and_then(|a| SpellFile::bar_entry(a, 18)) {
                Some(pax) => SpellFile::from_parts("P_EX100_KH1F", script, &pax),
                None => SpellFile::script_only("P_EX100_KH1F", script),
            };
            match made {
                Ok(file) => Some(magic.add_spell(file)),
                Err(e) => {
                    log(&format!("Limit Form: its script didn't load ({e}): no limits"));
                    None
                }
            }
        });
        Runtime { magic, file, queue: Vec::new(), fed: None, req: Requests::default(), taken_in: None, fx: HashMap::new(), started: 0, pending: None, now: (u16::MAX, 0), hits: 0 }
    })
}

/// A limit runs.
pub fn running() -> bool {
    RT.try_lock().ok().is_some_and(|g| g.as_ref().is_some_and(|rt| rt.pending.is_some() || rt.magic.limit_running(SORA)))
}

/// Out of the form: the limit ended, its effects put out.
pub fn end_all() {
    if let Ok(mut c) = CHOSEN.lock() {
        *c = None;
    }
    if let Ok(mut t) = TAKEN.lock() {
        *t = None;
    }
    if let Ok(mut o) = OFFER.lock() {
        *o = None;
    }
    let Ok(mut g) = RT.lock() else { return };
    let Some(rt) = g.as_mut() else { return };
    rt.magic.end_objects(SORA);
    rt.queue.clear();
    rt.fed = None;
    rt.req = Requests::default();
    rt.taken_in = None;
    rt.pending = None;
    if let Ok(mut t) = THROWN.lock() {
        *t = None;
    }
    for h in rt.fx.values().filter_map(|v| v.0) {
        super::kh2fx::spell_kill(h);
    }
    rt.fx.clear();
}

/// The world as the limit's script sees it: KH1's (`reflect::world_now`), with Sora driven.
struct LimitWorld<'a> {
    kh: KhWorld,
    out: &'a Output,
    /// The motion now as the script has it (one it started this frame, else KH2's).
    motion: u16,
    frame: u32,
    clips: Option<&'a HashMap<u16, character_core::Clip>>,
    /// The form's combo rows (`trap_obj_party_attack`).
    rows: &'a [character_core::ComboRow],
    /// Pushes waiting besides this frame's (the host's queue, or a motion handed on).
    queued: bool,
    req: &'a mut Requests,
    offer: Option<u16>,
}

impl LimitWorld<'_> {
    fn clip(&self) -> Option<&character_core::Clip> {
        self.clips?.get(&self.motion)
    }

    /// Sora's Keyblade in his hand (its object frame: KH2's weapon point on his hand bone).
    fn held(&self) -> Mat4 {
        self.kh.bone(SORA, 0x4005).unwrap_or_else(|| Mat4::from_translation(self.kh.pos(SORA) + Vec3::Y * 100.0))
    }
}

impl World for LimitWorld<'_> {
    fn exists(&self, o: ObjId) -> bool {
        o == WEAPON || self.kh.exists(o)
    }
    fn pos(&self, o: ObjId) -> Vec3 {
        if o == WEAPON {
            self.held().w_axis.truncate()
        } else {
            self.kh.pos(o)
        }
    }
    fn facing(&self, o: ObjId) -> f32 {
        match (o, self.req.face) {
            (SORA, Some(f)) => f,
            _ => self.kh.facing(o),
        }
    }
    fn motion(&self, o: ObjId) -> u16 {
        if o == SORA {
            self.motion
        } else {
            self.kh.motion(o)
        }
    }
    fn airborne(&self, o: ObjId) -> bool {
        self.kh.airborne(o)
    }
    fn jumping(&self, o: ObjId) -> bool {
        self.kh.jumping(o)
    }
    fn bone(&self, o: ObjId, code: u16) -> Option<Mat4> {
        if o == WEAPON {
            Some(self.held())
        } else {
            self.kh.bone(o, code)
        }
    }
    fn target_point(&self, o: ObjId) -> Vec3 {
        if o == WEAPON {
            self.pos(o)
        } else {
            self.kh.target_point(o)
        }
    }
    fn target_of(&self, o: ObjId) -> Option<ObjId> {
        self.kh.target_of(o)
    }
    fn party(&self, n: u32) -> Option<ObjId> {
        self.kh.party(n)
    }
    fn has_ability(&self, o: ObjId, n: u32) -> bool {
        self.kh.has_ability(o, n)
    }
    fn ground(&self, from: Vec3, depth: f32) -> Option<Vec3> {
        self.kh.ground(from, depth)
    }
    fn bg_hit(&self, from: Vec3, to: Vec3) -> bool {
        self.kh.bg_hit(from, to)
    }
    fn hitback(&mut self, o: ObjId, dir: Vec3, dist: f32) {
        // (Sora pushed: KH2's hit-back, `Character::hit_back`, his speed dist × 0.1 decaying.)
        if o == SORA {
            self.req.hitback = Some(([dir.x, dir.z], dist));
        } else {
            self.kh.hitback(o, dir, dist)
        }
    }
    fn random(&mut self) -> f32 {
        self.kh.random()
    }
    fn motion_start(&mut self, o: ObjId, m: u16, _blend: f32) {
        if o == SORA {
            self.req.start = Some(m);
            self.req.pushes.clear();
            self.motion = m;
            self.frame = 0;
            self.queued = false;
        }
    }
    fn motion_push(&mut self, o: ObjId, m: u16, _blend: f32) {
        if o == SORA {
            self.req.pushes.push(m);
        }
    }
    fn motion_ended(&self, o: ObjId) -> bool {
        if o != SORA || self.req.start.is_some() {
            return false;
        }
        let len = self.clip().map_or(1, |c| c.length);
        self.frame + 1 >= len && !self.queued && self.req.pushes.iter().all(|m| *m == 0)
    }
    fn motion_range(&self, o: ObjId, n: u8) -> bool {
        o == SORA && self.req.start.is_none() && self.clip().is_some_and(|c| c.has(self.frame, |k| *k == WindowKind::ScriptRange(n)))
    }
    fn motion_trigger(&self, o: ObjId, n: u8) -> bool {
        o == SORA && self.req.start.is_none() && self.out.events.iter().any(|e| *e == character_core::EventKind::Other(8 + n))
    }
    fn face(&mut self, o: ObjId, dir: Vec3) {
        if o == SORA && dir.x.abs() + dir.z.abs() > 1e-6 {
            self.req.face = Some(dir.x.atan2(dir.z));
        }
    }
    fn entry_id(&self, o: ObjId) -> u32 {
        if o == SORA {
            ENTRY_ID
        } else {
            0
        }
    }
    fn offer_command(&mut self, o: ObjId, command: u16) {
        // (Infinity has no follow-up pair: its script offers command 0 in range 2, which its
        // motions don't have.)
        if o == SORA && command != 0 {
            self.offer = Some(command);
        }
    }
    fn party_attack(&mut self, o: ObjId, row: u16) {
        // The row's motion, as the script's own start (its hits and windows are the motion's).
        match self.rows.iter().find(|r| r.id as u16 == row) {
            Some(r) => self.motion_start(o, r.motion, 0.0),
            None => note(format!("Limit Form: the script's combo row {row} isn't in the moveset")),
        }
    }
    fn weapon(&self, o: ObjId, n: u32) -> Option<ObjId> {
        (o == SORA && n == 0).then_some(WEAPON)
    }
    fn foes(&self) -> Vec<ObjId> {
        self.kh.foes()
    }
}

/// Before KH2's step: a limit chosen starts, a follow-up taken goes to the script, and what the
/// script asked of Sora last frame is done (his motion started or chained, his facing).
pub unsafe fn before_step(actor: usize, ch: &mut Character) {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    let Ok(mut g) = RT.try_lock() else { return };
    if !super::form::in_limit() {
        if g.as_ref().is_some_and(|rt| rt.magic.limit_running(SORA)) {
            drop(g);
            end_all();
        }
        return;
    }
    let rt = runtime(&mut g);
    let chosen = CHOSEN.try_lock().ok().and_then(|mut c| c.take());
    if let (Some(k), Some(_)) = (chosen, rt.file) {
        let l = &LIMITS[k];
        if !ch.can_take_command() || usable(actor).is_err() {
            note(format!("Limit Form: {} dropped (KH2's move can't take it now, or it can't be used)", l.name));
        } else {
            let before = mp(actor);
            super::reflect::pay_mp(actor, COST);
            rt.pending = Some(k);
            log(&format!(
                "Limit Form: {} (cmd {}): KH2's {} MP is KH1's 1 bar (the mod's KH2 spells' 1 MP each; KH2's cost / 100 rounded up), MP {before:?} -> {:?}",
                l.name,
                l.command,
                l.kh2_mp,
                mp(actor)
            ));
        }
    }
    if let Some(cmd) = TAKEN.try_lock().ok().and_then(|mut t| t.take()) {
        if rt.magic.limit_command(SORA, cmd) {
            rt.taken_in = Some((super::takeover::KH2_MOTION.load(Ordering::Relaxed) as u16, 0));
        }
    }
    // The script's requests.
    let req = std::mem::take(&mut rt.req);
    if let Some(f) = req.face {
        ch.set_facing(f);
    }
    if let Some((dir, dist)) = req.hitback {
        ch.hit_back(dir, dist);
    }
    if let Some(m) = req.start {
        // (A start of the motion that has just begun, on its first frame: the script taking over
        // a motion its own push started, as Sonic Rave's 212 and Infinity's rows do. Started again
        // it played its first frame twice, its frame-0 voice and effects too.)
        if rt.now == (m, 0) && ch.scripted() {
            rt.queue = req.pushes;
            rt.fed = None;
        } else if ch.play(m, false) {
            rt.queue = req.pushes;
            rt.fed = None;
        } else {
            note(format!("Limit Form: the script's motion {m} isn't in the moveset"));
            rt.queue.clear();
        }
    } else {
        rt.queue.extend(req.pushes);
    }
    // The next pushed motion to the controller (it starts when the one playing ends).
    if !ch.scripted() {
        rt.queue.clear();
        rt.fed = None;
    } else if ch.scripted_next().is_none() && rt.fed.is_none() {
        if let Some(&m) = rt.queue.first() {
            rt.queue.remove(0);
            // (Idle and the fall end the chain: free movement from there.)
            if matches!(m, 0 | 4) {
                rt.queue.clear();
                ch.push_scripted(None);
            } else if ch.push_scripted(Some(m)) {
                rt.fed = Some(m);
            }
        }
    }
}

/// Sora's MP (stats `+0x44` of `+0x48`), for the log.
unsafe fn mp(sora: usize) -> (i32, i32) {
    let stats = pose::ptr(rd::<u32>(sora + 0x6C));
    if stats < 0x10000 {
        return (0, 0);
    }
    (rd::<i32>(stats + 0x44), rd::<i32>(stats + 0x48))
}

// ---------------------------------------------------------------- the Keyblade as drawn

/// The thrown Keyblade (Strike Raid): its model → KH1's world (y down), its motion's frame and
/// the motion, and when (`after_step`).
static THROWN: Mutex<Option<(Mat4, f32, u16, std::time::Instant)>> = Mutex::new(None);
/// KH2's blade bone where KH1 draws the Keyblade this frame (KH1's world, y down; in his hand, or
/// thrown), and when (`pose_hook`).
static DRAWN: Mutex<Option<(Mat4, std::time::Instant)>> = Mutex::new(None);

/// The Keyblade's blade bone as drawn, if drawn in the last quarter second: Limit Form's weapon
/// anchors (`kh2fx::form_bone`; KH2's own are the blade bone, its turn included, as Final's).
pub fn drawn_blade() -> Option<Mat4> {
    let g = DRAWN.try_lock().ok()?;
    g.filter(|(_, t)| t.elapsed().as_secs_f32() < 0.25).map(|(m, _)| m)
}

/// `pose`, before Sora's bones are written (in Limit Form): the Keyblade thrown is drawn where it
/// flies (KH1's own weapon bones moved there, `leftblade::float_grip`), spinning on its motion
/// (the Keyblade's 235, its blade bone `leftblade::KH2_BLADE`); and where the Keyblade is drawn is
/// noted for the effects on it (`drawn_blade`). `kh1`: KH1's bones (model space); `ratio`: KH1's
/// size against KH2's.
pub fn pose_hook(kh1: &mut [Mat4], kh1_bind: &[Mat4], to_world: &Mat4, ratio: f32) {
    // (With KH2's Limit menu off too: where the Keyblade is drawn is all Limit Form's effects'
    // weapon anchors, its combos' as well. Nothing is thrown then: only KH2's script throws it.)
    if !super::form::in_limit() {
        return;
    }
    static N: AtomicU32 = AtomicU32::new(0);
    let k = if ratio.is_finite() && ratio > 0.1 { ratio } else { 1.0 };
    let thrown = THROWN.try_lock().ok().and_then(|g| *g).filter(|t| t.3.elapsed().as_secs_f32() < 0.25);
    let blade_world = match thrown {
        Some((model, frame, motion, _)) => {
            let local = pose::kh2().and_then(|k| k.limit.as_ref()).and_then(|f| {
                let w = f.rig.weapons.first()?;
                let m = w.motions.get(&motion).or_else(|| w.motions.get(&THROWN_MOTION))?;
                let bones = kh2_anim::evaluate(&w.skeleton, m, kh2_anim::key_time(m, frame, true), kh2_anim::Options::default()).bones;
                bones.get(super::leftblade::KH2_BLADE).or(bones.first()).copied()
            });
            let Some(local) = local else {
                if N.fetch_add(1, Ordering::Relaxed) < 5 {
                    log("Limit Form: the thrown Keyblade's motion isn't loaded: drawn in his hand");
                }
                return;
            };
            let blade = model * super::leftblade::scaled(local, k);
            if !super::leftblade::float_grip(kh1, kh1_bind, to_world.inverse() * blade, k) {
                return;
            }
            if N.fetch_add(1, Ordering::Relaxed) < 3 {
                log(&format!("Limit Form: the thrown Keyblade drawn at {:.0?}, {:.0} from Sora", blade.w_axis.truncate(), (blade.w_axis - to_world.w_axis).truncate().length()));
            }
            blade
        }
        None => match super::leftblade::held_blade(kh1, kh1_bind, k) {
            Some(b) => *to_world * b,
            None => return,
        },
    };
    let m = super::leftblade::unit(blade_world);
    if m.is_finite() {
        if let Ok(mut d) = DRAWN.try_lock() {
            *d = Some((m, std::time::Instant::now()));
        }
    }
}

/// An unbound effect's root where the script has it (y up) → KH2's world (y down), turned along
/// its direction (as `formmagic`'s).
fn place(pos: Vec3, dir: Vec3) -> Mat4 {
    let r = Quat::from_rotation_arc(Vec3::Z, dir.try_normalize().unwrap_or(Vec3::Z));
    super::reflect::flip() * Mat4::from_rotation_translation(r, pos) * super::reflect::flip()
}

/// After KH2's step and the swings: the limit script's frame (its requests for the next step, the
/// follow-up offered, its effects).
pub unsafe fn after_step(actor: usize, out: &Output, airborne: bool, strikes: &mut combat::Strikes, battle: &Battle, lock: Option<u32>) {
    FRAME.fetch_add(1, Ordering::Relaxed);
    if !ON.load(Ordering::Relaxed) || !super::form::in_limit() {
        return;
    }
    let Ok(mut g) = RT.try_lock() else { return };
    let Some(rt) = g.as_mut() else { return };
    rt.now = (out.motion, out.clip_frame);
    if rt.pending.is_none() && rt.magic.casts.is_empty() && rt.magic.effects.is_empty() && rt.fx.is_empty() {
        return;
    }
    if rt.fed == Some(out.motion) {
        rt.fed = None;
    }
    let form = pose::kh2().and_then(|k| k.limit.as_ref());
    let clips = form.map(|f| &f.character.moveset.clips);
    let rows = form.map_or(&[][..], |f| f.character.moveset.combo_rows.as_slice());
    // (Seeded by the frame too: Infinity's shots draw new random numbers every frame, their wobble.)
    let seed = 0x1A17_0000 ^ rt.started.wrapping_mul(2_654_435_761) ^ FRAME.load(Ordering::Relaxed).wrapping_mul(0x9E37_79B9);
    let mut kh = world_now(actor, out, airborne, strikes, lock, seed | 1);
    // (Nothing locked on: the nearest enemy in front, as Wisdom's shots take; Strike Raid's throw
    // and Infinity's lock-on shots go for it. Ours: KH2's lock-on is the player's.)
    if kh.target.is_none() {
        kh.target = super::formmagic::auto_target(&kh, out.facing);
    }
    let queued = rt.fed.is_some() || !rt.queue.is_empty();
    let mut req = std::mem::take(&mut rt.req);
    let mut w = LimitWorld { kh, out, motion: out.motion, frame: out.clip_frame, clips, rows, queued, req: &mut req, offer: None };
    // Infinity's shots: the motion's frame trigger 7, the script's trigger 17 (a thread of the
    // limit's run, first run in this frame's update).
    if out.events.iter().any(|e| *e == character_core::EventKind::Other(SHOTS_TRIGGER)) && rt.magic.limit_motion_trigger(SORA, SHOTS_SCRIPT_TRIGGER, &SHOTS_PARAMS) {
        note(format!("Limit Form: motion {}'s shots (frame trigger {SHOTS_TRIGGER}, the script's trigger {SHOTS_SCRIPT_TRIGGER})", out.motion));
    }
    // A limit paid for: its script's limit trigger, its first frame now.
    if let (Some(k), Some(file)) = (rt.pending.take(), rt.file) {
        match rt.magic.limit(file, SORA, LIMITS[k].command, &mut w) {
            Ok(_) => {
                rt.started += 1;
                rt.queue.clear();
                rt.fed = None;
                rt.taken_in = None;
            }
            Err(e) => log(&format!("Limit Form: {}'s script didn't start ({e})", LIMITS[k].name)),
        }
    }
    let events = rt.magic.update(&mut w);
    let mut offer = w.offer;
    // A follow-up the motion carries (KH2's range 20, the engine's own offer): Last Arcanum's
    // Bash and The End, Infinity's.
    if offer.is_none() && rt.magic.limit_running(SORA) {
        offer = clips.and_then(|c| c.get(&out.motion)).and_then(|c| c.open(out.clip_frame).find_map(|win| if let WindowKind::OfferCommand(cmd) = win.kind { Some(cmd) } else { None }));
    }
    rt.req = req;
    // One press per window: after a follow-up, none until its motion starts again.
    if let Some((m, f)) = rt.taken_in {
        let restarted = rt.req.start.is_some() || out.motion != m || out.clip_frame < f;
        if restarted {
            rt.taken_in = None;
        } else {
            rt.taken_in = Some((m, out.clip_frame));
            offer = None;
        }
    }
    if let Some(cmd) = offer {
        if let Ok(mut o) = OFFER.lock() {
            *o = Some((cmd, FRAME.load(Ordering::Relaxed)));
        }
    }
    for e in events.into_iter().chain(rt.magic.take_events()) {
        match e {
            Event::Missing(n) => note(format!("Limit Form's script used {n} (not modelled)")),
            Event::Error(e) => log(&format!("Limit Form's script error: {e}")),
            Event::Strike { .. } => {}
        }
    }
    // The thrown Keyblade, for its draw (`pose_hook`): its model in KH1's world and its motion's
    // frame.
    let thrown = rt.magic.loose.get(&WEAPON).map(|l| (l.game_matrix() * Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0)), l.frame, l.motion));
    if let Ok(mut t) = THROWN.lock() {
        let was = t.is_some();
        *t = thrown.map(|(m, f, motion)| (m, f, motion, std::time::Instant::now()));
        if was != t.is_some() {
            note(format!("Limit Form: the Keyblade {}", if t.is_some() { "thrown (Strike Raid)" } else { "back in his hand" }));
        }
    }
    // Effects the script starts: from the form's archive, bound ones on Sora's bones (and the
    // Keyblade's, `kh2fx::form_bone`); one on the thrown Keyblade (its trail) on its blade bone as
    // drawn; an unbound one (a shot, the catch's flash) where the script puts it. (One the script
    // unbinds from Sora, Infinity's charge as it fades, stays bound: ours has no place for it but
    // his hand's.)
    let package = super::kh2fx::LIMIT;
    let blade = drawn_blade();
    let mut seen = Vec::new();
    for e in &rt.magic.effects {
        seen.push(e.id);
        let placed = match e.owner {
            Some(WEAPON) => blade,
            None => Some(place(e.pos, e.dir)),
            Some(_) => None,
        };
        match rt.fx.get(&e.id) {
            None => {
                let h = if e.alive() { super::kh2fx::spell_start(package, e.element.id) } else { None };
                if let (Some(h), Some(at)) = (h, placed) {
                    super::kh2fx::spell_place(h, at);
                }
                if e.alive() {
                    super::kh2sound::spell_element(package, e.element.id);
                }
                rt.fx.insert(e.id, (h, false));
            }
            Some((Some(h), ended)) => {
                let (h, ended) = (*h, *ended);
                if let Some(at) = placed {
                    super::kh2fx::spell_place(h, at);
                }
                if e.killed {
                    super::kh2fx::spell_kill(h);
                } else if e.fade.is_some() {
                    super::kh2fx::spell_alpha(h, e.alpha());
                    if !e.alive() {
                        super::kh2fx::spell_kill(h);
                    }
                } else if (e.ended_at.is_some() || !e.alive()) && !ended {
                    super::kh2fx::spell_end(h);
                    rt.fx.insert(e.id, (Some(h), true));
                }
            }
            Some((None, _)) => {}
        }
    }
    rt.fx.retain(|id, (h, _)| seen.contains(id) || h.is_some_and(super::kh2fx::spell_running));
    // The script's attacks (Strike Raid's thrown Keyblade, Infinity's shots) against KH1's
    // enemies, through the spell hit records as Wisdom's shots (`formmagic`); one that hits is
    // told so (a shot ends there).
    let attacks: Vec<combat::SpellAttack> = rt
        .magic
        .attacks
        .iter()
        .filter(|a| !a.freed && a.owner == SORA)
        .filter_map(|a| {
            let first = battle.attack_for_level(a.atk, a.sub, 0)?;
            if !combat_core::AttackKind::from_id(first.kind).deals_damage() {
                return None;
            }
            let shapes = a.shapes();
            (!shapes.is_empty()).then(|| combat::SpellAttack { kh1_data: None, key: a.id | KEY, group: if first.combo_group != 0 { first.combo_group as i64 } else { a.group }, atk: a.atk, sub: a.sub, shapes })
        })
        .collect();
    if !attacks.is_empty() {
        let hits = strikes.spell_strike(actor, &attacks, &|atk, sub, count| battle.attack_for_level(atk, sub, count).cloned());
        for (key, atk, foe, dealt) in hits {
            rt.hits += 1;
            if let Some(a) = rt.magic.attacks.iter_mut().find(|a| a.id | KEY == key) {
                a.hit = true;
            }
            if rt.hits <= 60 {
                note(format!("Limit Form: ATKP {atk} hit {foe:#x} -> damage {dealt:?}"));
            }
        }
    }
    for a in rt.magic.freed().filter(|a| a.owns_id) {
        strikes.spell_forget(a.id | KEY);
    }
    if !rt.magic.limit_running(SORA) && rt.queue.is_empty() {
        if let Ok(mut o) = OFFER.lock() {
            *o = None;
        }
    }
}

// ---------------------------------------------------------------- KH1's limits in Limit Form

/// One of KH1's own limits: its command, the follow-up commands it takes while it runs, KH1's
/// name, and the KH2 Limit Form limit it stands in for (`None`: none).
pub struct Kh1Limit {
    pub command: u8,
    follow_ups: &'static [u8],
    pub name: &'static str,
    kh2: Option<&'static str>,
}

/// KH1's limits, in the order of its Triangle offer's table (`0x140529040`: five pairs of KH1
/// ability and command, abilities 11..15; read from the executable), which is also the order of
/// their data in `xl_limit.dat` and their MP costs. The command is also the command type KH1's
/// command step dispatches on (`0x1402A1470` sends these to its limit start `0x14028B240`, which
/// starts limit 0..4 through `0x140397C00` and pays its MP). The follow-ups are the commands that
/// dispatcher passes to the running limit (`0x140397C00 (7 | 8 | 9 | 11, 0)`): Sonic Blade 0x4D /
/// 0x4E, Ars Arcanum 0x58 / 0x59, Strike Raid 0x5F / 0x60, Ragnarok 0x5B; Trinity Limit has none.
/// The first four stand in for Limit Form's KH2 limits (the user's pairs): Sonic Blade for Sonic
/// Rave, Ars Arcanum for Last Arcanum, Strike Raid for Strike Raid, Ragnarok for Infinity.
///
/// KH1's Zantetsuken (ability 59) is not one of them: it is a combo finisher, not a limit (its
/// ability sets bit 29 of KH1's combo-ability flags `0x142D5F610`, `0x1402A4707`, which KH1's
/// attack-row pick reads, `0x1402A4E3E`, as it reads Stun Impact's and Gravity Break's); it has no
/// command, no entry in the limit table or file, so nothing here can start it. (The mod also takes
/// it out of Sora's ability list, `menu.rs`; Limit Form has KH2's own Zantetsuken, its default
/// ground finisher.)
pub const KH1_LIMITS: [Kh1Limit; 5] = [
    Kh1Limit { command: 0x4B, follow_ups: &[0x4D, 0x4E], name: "Sonic Blade", kh2: Some("Sonic Rave") },
    Kh1Limit { command: 0x57, follow_ups: &[0x58, 0x59], name: "Ars Arcanum", kh2: Some("Last Arcanum") },
    Kh1Limit { command: 0x5E, follow_ups: &[0x5F, 0x60], name: "Strike Raid", kh2: Some("Strike Raid") },
    Kh1Limit { command: 0x5A, follow_ups: &[0x5B], name: "Ragnarok", kh2: Some("Infinity") },
    Kh1Limit { command: 0x52, follow_ups: &[], name: "Trinity Limit", kh2: None },
];
/// KH1's battle flags (`takeover::BATTLE_RVA`); bit 16: a KH1 limit runs (set by its limit start
/// `0x14028B670`, cleared by its frame task `0x14028B490` two frames after the limit's module
/// stops, `0x140397C00 (5)` reading 0; its offer is skipped while it is set).
const BATTLE_FLAGS_RVA: Rva = Rva(0x2867CD8);
const KH1_LIMIT_ON: u32 = 0x1_0000;

/// KH1's command (type) `kind` is one of its limits (`false`) or a limit's follow-up (`true`):
/// its index in `KH1_LIMITS`.
pub fn kh1_limit(kind: u8) -> Option<(usize, bool)> {
    KH1_LIMITS.iter().enumerate().find_map(|(k, l)| if l.command == kind { Some((k, false)) } else if l.follow_ups.contains(&kind) { Some((k, true)) } else { None })
}

/// Whether a KH1 limit may run in the form Sora is in: only in Limit Form (all five, from its
/// Limit menu or shortcuts), and only while KH2's Limit menu is off (out of a form this isn't
/// asked: KH1's own rules).
pub fn kh1_limit_allowed(k: usize) -> bool {
    !KH2_LIMITS && super::form::in_limit() && k < KH1_LIMITS.len()
}

// ------------------------------------------------ KH1's limits' menu in Limit Form

/// The Limit menu (KH1's limits) is in: its records can be written, "Limit"'s text is ready.
static KH1_ON: AtomicBool = AtomicBool::new(false);
/// KH1 text for "Limit" (Magic's row in the form).
static KH1_TEXT: OnceLock<usize> = OnceLock::new();
/// The fill put the form's limits in the shortcut menu this frame (`shortcuts::fill` then leaves
/// it alone).
static SHORTCUTS_OURS: AtomicBool = AtomicBool::new(false);
/// The menu's command records, one per `KH1_LIMITS` entry (spell-type records with arguments
/// `LIMIT_ARG + k`, written over empty ones as `drive`'s forms are; KH2's menu would use the same
/// ones, and only one of the two is ever on). Each takes KH1's own word for its limit (the word
/// of KH1's record for the limit's command), so the rows read KH1's names in KH1's text.
const KH1_RECORDS: [u8; 5] = [0x82, 0x83, 0x84, 0x85, 0x86];
/// KH2 FM's Limit Form shortcuts, by the mod's shortcut slot (L1 + Triangle, Square, confirm,
/// cancel: KH1's three and the mod's fourth), as indices into `KH1_LIMITS`. KH2 (FM PC
/// executable, read): in Limit Form its shortcut fill (`0x1403F2780`, and the same code in the
/// menu build at `0x1403F2210`) puts entry k of its table at `0x1405C97E8` (Sonic Rave, Last
/// Arcanum, Strike Raid, Infinity: `cmd` 0x2BA, 0x2BD, 0x2C0, 0x2AB) in shortcut slot k, unless
/// the slot holds an item (KH1's shortcuts never do). So slot k is the KH1 limit standing in for
/// KH2's k-th (the user's pairs). Change a slot here to change its limit; Trinity Limit (4) can
/// go in one too.
pub const SHORTCUT_LIMITS: [usize; 4] = [0, 1, 2, 3];
/// The limit data's load state (`0x14299CC20`, the frame task `0x14028B490`'s state): 5 once
/// `xl_limit.dat` (all five limits) is loaded; KH1's offer needs it.
const LIMIT_STATE_RVA: Rva = Rva(0x299CC20);
const LIMIT_LOADED: i32 = 5;
/// KH1's battle parameters (`*0x142D23730`): `+0xF4 + 2k` limit k's MP cost (u16, hundredths of
/// a bar; KH1's offer checks it, its start pays it).
const BATTLE_PARAMS_RVA: Rva = Rva(0x2D23730);
const LIMIT_COST_OFF: usize = 0xF4;
/// KH1's command flags (`0x142D54438`): bit 1, no limits now (KH1's offer stops on it).
const COMMAND_FLAGS_RVA: Rva = Rva(0x2D54438);
/// The party (`0x142D37C80`: Sora, then the two others), the target the menu aims at
/// (`0x142DB3F18`, a handle), the shortcut menu and the flag KH1 sets when it has a shortcut.
const PARTY_RVA: Rva = Rva(0x2D37C80);
const TARGET_RVA: Rva = Rva(0x2DB3F18);
const SHORTCUT_MENU_RVA: Rva = Rva(0x529718);
const HAS_SHORTCUTS_RVA: Rva = Rva(0x2D5445C);
const SAVE_PTR_RVA: Rva = Rva(0x28695A0);
/// Trinity Limit's reach to the two others (KH1's offer: squared distance at most this).
const TRINITY_REACH_SQ: f32 = 250_000.0;

/// Our menu argument `arg` is a row of KH1's limits' menu: its index in `KH1_LIMITS`.
pub fn kh1_menu_arg(arg: u8) -> Option<usize> {
    if KH2_LIMITS || !KH1_ON.load(Ordering::Relaxed) {
        return None;
    }
    (LIMIT_ARG..LIMIT_ARG + KH1_LIMITS.len() as u8).contains(&arg).then(|| (arg - LIMIT_ARG) as usize)
}

/// KH1's MP cost of limit `k` (hundredths of a bar), as its start pays it.
pub unsafe fn kh1_cost(k: usize) -> Option<i32> {
    let params = rd::<u64>(base() + BATTLE_PARAMS_RVA) as usize;
    let at = params + LIMIT_COST_OFF + 2 * k;
    (params > 0x10000 && k < KH1_LIMITS.len() && super::crash::readable(at)).then(|| rd::<u16>(at) as i32)
}

/// A KH1 object can be hit and aimed at (`0x14028FE60`: `+0x374` bit 0 set, bit 1 clear).
unsafe fn hittable(obj: usize) -> bool {
    obj > 0x10000 && super::crash::readable(obj + 0x378) && rd::<u32>(obj + 0x374) & 3 == 1
}

/// Whether KH1 can start its limit `k` for Sora now (in Limit Form), and why not: KH1's own
/// conditions for offering one (`0x14028AF60`), equipped or not. No limit running, limits allowed
/// (`0x142D54438` bit 1), the limit data loaded, Sora in KH1's ground state (`+0x70` 0) and on
/// KH2's ground, a target to aim at (`0x142DB3F18` hittable, `+0x374 & 0x60` = 0x40), for
/// Trinity Limit both others there (not KO'd, `+0x88 & 0x30` clear; hittable, not `+0x374 &
/// 0x8000`, not `+0 & 0x200`; within 500), and the MP (KH1's test `0x1402A2A80` with the cost it
/// pays). Not KH1's offer's distance and height rules: they choose which one limit to offer,
/// which a menu doesn't need (Sonic Blade and Ragnarok are told apart there by the target's
/// height, so one of the two would always be greyed).
pub unsafe fn kh1_usable(actor: usize, k: usize) -> Result<(), &'static str> {
    let base = base();
    if base == 0 || actor < 0x10000 || k >= KH1_LIMITS.len() {
        return Err("no Sora");
    }
    if kh1_limit_running() {
        return Err("a limit is running");
    }
    if rd::<u8>(base + COMMAND_FLAGS_RVA) & 2 != 0 {
        return Err("KH1 allows no limits now");
    }
    if rd::<i32>(base + LIMIT_STATE_RVA) != LIMIT_LOADED {
        return Err("KH1's limit data isn't loaded");
    }
    if super::takeover::kh2_airborne() || rd::<i32>(actor + 0x70) != 0 {
        return Err("not on the ground");
    }
    let target = pose::ptr(rd::<u32>(base + TARGET_RVA));
    if !hittable(target) || rd::<u32>(target + 0x374) & 0x60 != 0x40 {
        return Err("no target");
    }
    if KH1_LIMITS[k].command == 0x52 {
        let me = Vec3::from_array(rd::<[f32; 3]>(actor + 0x10));
        for slot in 1..=2usize {
            let m = rd::<u64>(base + PARTY_RVA + slot * 8) as usize;
            if m < 0x10000 || !super::crash::readable(m + 0x378) {
                return Err("Trinity Limit: the party isn't all there");
            }
            if rd::<u32>(m + 0x88) & 0x30 != 0 || !hittable(m) || rd::<u32>(m + 0x374) & 0x8000 != 0 || rd::<u32>(m) & 0x200 != 0 {
                return Err("Trinity Limit: a party member can't join in");
            }
            if (Vec3::from_array(rd::<[f32; 3]>(m + 0x10)) - me).length_squared() > TRINITY_REACH_SQ {
                return Err("Trinity Limit: a party member is too far");
            }
        }
    }
    match kh1_cost(k) {
        Some(c) if super::reflect::has_mp(actor, c) => Ok(()),
        Some(_) => Err("MP short"),
        None => Err("its MP cost isn't readable"),
    }
}

/// The menu's records (`KH1_RECORDS`), each with the word of KH1's record for its limit.
unsafe fn kh1_records(base: usize) -> bool {
    let record: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base + RECORD_RVA);
    for (k, l) in KH1_LIMITS.iter().enumerate() {
        let theirs = record(l.command as u32);
        let r = record(KH1_RECORDS[k] as u32);
        if theirs < 0x10000 || r < 0x10000 {
            return false;
        }
        let word = rd::<u16>(theirs + 4);
        static CHECKED: AtomicBool = AtomicBool::new(false);
        if !CHECKED.swap(true, Ordering::Relaxed) {
            // (That the command type is the command id is inferred, `docs/drive-model.md`.)
            log(&format!(
                "Limit Form: KH1's limit records {}",
                KH1_LIMITS.iter().map(|l| { let p = record(l.command as u32); format!("{:#x} {:02x?}", l.command, rd::<[u8; 16]>(p)) }).collect::<Vec<_>>().join(", ")
            ));
        }
        if word == 0 {
            static ONCE: AtomicBool = AtomicBool::new(false);
            if !ONCE.swap(true, Ordering::Relaxed) {
                log(&format!("Limit Form: KH1's record for {} has no name: no Limit menu", l.name));
            }
            return false;
        }
        let want: [u8; 16] = [0x02, 0x00, 0xFF, 0x00, word as u8, (word >> 8) as u8, 0x02, LIMIT_ARG + k as u8, 0x00, 0xFF, 0x01, 0x02, 0, 0, 0, 0];
        let have = rd::<[u8; 16]>(r);
        if have != want {
            if have != [0u8; 16] && have[6..8] != want[6..8] {
                static ONCE: AtomicBool = AtomicBool::new(false);
                if !ONCE.swap(true, Ordering::Relaxed) {
                    log(&format!("Limit Form: command record {:#x} is in use ({have:02x?}): no Limit menu", KH1_RECORDS[k]));
                }
                return false;
            }
            wr(r, want);
        }
    }
    if MAGIC_WORD.load(Ordering::Relaxed) == u32::MAX {
        let m = record(2);
        if m > 0x10000 {
            MAGIC_WORD.store(rd::<u16>(m + 4) as u32, Ordering::Relaxed);
        }
    }
    true
}

/// The shortcut menu holds the form's limits this frame (`shortcuts::fill` leaves it then).
pub fn shortcuts_ours() -> bool {
    SHORTCUTS_OURS.load(Ordering::Relaxed)
}

/// After KH1's battle-menu fill (`drive::fill`), in Limit Form: the Magic row reads "Limit" and
/// its list is KH1's five limits, whatever is equipped or learned (KH1's names; greyed while KH1
/// couldn't start it, `kh1_usable`); the shortcuts are KH2 FM's Limit Form ones
/// (`SHORTCUT_LIMITS`). KH1 fills both menus again every frame, so out of the form Sora's own
/// spells and shortcuts are back at once.
unsafe fn fill_kh1(actor: usize) {
    SHORTCUTS_OURS.store(false, Ordering::Relaxed);
    let base = base();
    if !KH1_ON.load(Ordering::Relaxed) || base == 0 || !super::form::in_limit() || !kh1_records(base) {
        return;
    }
    let why: Vec<Result<(), &'static str>> = (0..KH1_LIMITS.len()).map(|k| kh1_usable(actor, k)).collect();
    let grey = why.iter().enumerate().fold(0u32, |g, (k, w)| if w.is_err() { g | 1 << k } else { g });
    let magic = base + MAGIC_MENU_RVA;
    let n = rd::<i32>(magic + 0x10);
    if !(0..=15).contains(&n) {
        return;
    }
    // (KH1's last entry, Summon or the summon-out one, stays last: the fourth row, page 2's Drive.)
    let last = (n > 0).then(|| rd::<u8>(magic + 0x14 + n as usize - 1)).filter(|e| matches!(*e, SUMMON | SUMMON_OUT));
    let (old4, old8) = (rd::<u32>(magic + 4), rd::<u32>(magic + 8));
    for (k, r) in KH1_RECORDS.iter().enumerate() {
        wr(magic + 0x14 + k, *r);
    }
    let mut count = KH1_RECORDS.len();
    let (mut g4, mut g8) = (grey, grey);
    if let Some(e) = last {
        wr(magic + 0x14 + count, e);
        let b = (n - 1) as u32;
        g4 |= (old4 >> b & 1) << count;
        g8 |= (old8 >> b & 1) << count;
        count += 1;
    }
    wr(magic + 0x10, count as i32);
    wr(magic + 4, g4);
    wr(magic + 8, g8);
    if rd::<i32>(magic + 0xC) >= count as i32 {
        wr(magic + 0xC, 0i32);
    }
    // The top's Magic row there and open (KH1 leaves it out with no spell; `drive::fill` greyed
    // it). Not where KH1 has no magic at all (save `+0x874` bit 0, which greys Magic itself).
    let save = rd::<u64>(base + SAVE_PTR_RVA) as usize;
    let flags = if save > 0x10000 { rd::<u8>(save + 0x874) } else { 0xFF };
    let top = base + MENU_RVA;
    wr(top + 0x14 + 1, 2u8);
    if flags & 1 == 0 {
        for off in [4usize, 8] {
            wr(top + off, rd::<u32>(top + off) & !2);
        }
    }
    // The shortcuts (KH1 fills them only with magic allowed here, `+0x874 & 9` clear).
    let mut slots = 0;
    if flags & 9 == 0 {
        let menu = base + SHORTCUT_MENU_RVA;
        slots = rd::<i32>(menu + 0x10).clamp(0, SHORTCUT_LIMITS.len() as i32) as usize;
        for (slot, &k) in SHORTCUT_LIMITS.iter().enumerate().take(slots) {
            wr(menu + 0x14 + slot, KH1_RECORDS[k]);
            for at in [menu + 4, menu + 8] {
                let v = rd::<u32>(at);
                wr(at, if grey >> k & 1 != 0 { v | 1 << slot } else { v & !(1 << slot) });
            }
        }
        if slots > 0 {
            wr(base + HAS_SHORTCUTS_RVA, 1u32);
            SHORTCUTS_OURS.store(true, Ordering::Relaxed);
        }
    }
    static LOGGED: AtomicU32 = AtomicU32::new(u32::MAX);
    let key = grey | (slots as u32) << 8;
    if LOGGED.swap(key, Ordering::Relaxed) != key {
        let rows: Vec<String> = KH1_LIMITS
            .iter()
            .zip(&why)
            .enumerate()
            .map(|(k, (l, w))| format!("{} {} MP {}", l.name, kh1_cost(k).map_or("?".into(), |c| format!("{}", c as f32 / 100.0)), w.err().unwrap_or("usable")))
            .collect();
        log(&format!(
            "Limit Form: the Limit menu ({}); shortcuts {}",
            rows.join(", "),
            if slots == 0 { "not shown here".to_string() } else { SHORTCUT_LIMITS.iter().take(slots).map(|&k| KH1_LIMITS[k].name).collect::<Vec<_>>().join(" / ") }
        ));
    }
}

/// A KH1 limit runs now (KH1's battle flags' bit 16).
pub fn kh1_limit_running() -> bool {
    let base = base();
    base != 0 && unsafe { rd::<u32>(base + BATTLE_FLAGS_RVA) } & KH1_LIMIT_ON != 0
}

/// In Limit Form with one of KH1's limits running: KH1 has Sora for all of it (`takeover`), its
/// own animation drawn (`pose::follow`), the form on and draining (`form::drain`).
pub fn kh1_limit_holds() -> bool {
    !KH2_LIMITS && super::form::in_limit() && kh1_limit_running()
}

// ---------------------------------------------------------------- install

/// Points the `jmp rel32` at `site` (which must go to `to`) at `ours`, through a stub near the
/// executable (`hook::redirect_call`'s, for a tail jump).
unsafe fn redirect_jump(site: usize, to: usize, ours: usize, what: &str) -> bool {
    if rd::<u8>(site) != 0xE9 {
        log(&format!("redirect {what} at {site:#x}: not a jump"));
        return false;
    }
    let rel = rd::<i32>(site + 1);
    if (site as i64 + 5 + rel as i64) as usize != to {
        log(&format!("redirect {what} at {site:#x}: goes elsewhere"));
        return false;
    }
    let Some(stub) = hook::alloc_near(site, 16) else {
        log(&format!("redirect {what}: no memory near {site:#x}"));
        return false;
    };
    let mut go = [0u8; 14];
    go[0] = 0xFF;
    go[1] = 0x25;
    go[6..].copy_from_slice(&(ours as u64).to_le_bytes());
    std::ptr::copy_nonoverlapping(go.as_ptr(), stub as *mut u8, go.len());
    let mut want = [0u8; 5];
    want[0] = 0xE9;
    want[1..].copy_from_slice(&rel.to_le_bytes());
    let mut jmp = [0u8; 5];
    jmp[0] = 0xE9;
    jmp[1..].copy_from_slice(&((stub as i64 - (site as i64 + 5)) as i32).to_le_bytes());
    hook::patch(site, &want, &jmp, what)
}

/// After `drive::install` (its word hook answers ours first).
pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    if !super::settings::get().drive {
        return;
    }
    // KH1's limits never offered on Triangle in a form (the user), whatever else goes in
    // (`specials_prompt`); Limit Form has them in its Limit menu.
    ORIG_SPECIALS.store(base + SPECIALS_RVA, Ordering::Release);
    if !hook::redirect_call(base + SPECIALS_CALL_RVA, base + SPECIALS_RVA, specials_prompt as *const () as usize, "KH1's limits' Triangle offer (none in a form)") {
        log("KH1's limits: their Triangle offer wasn't as expected: they still show in a form (refused when used, but in Limit Form)");
    }
    if !super::drive::on() {
        log("Limit Form: no Drive menu, no Limit menu");
        return;
    }
    // KH2's Limit menu off for now (`KH2_LIMITS`): none of it is installed, `ON` stays false;
    // KH1's limits' menu in its place.
    if !KH2_LIMITS {
        let _ = KH1_TEXT.set(Box::leak(super::menu::encode("Limit").into_boxed_slice()).as_ptr() as usize);
        KH1_ON.store(true, Ordering::Release);
        log(&format!(
            "Limit Form: KH2's Limit menu off for now (KH2_LIMITS); the Limit menu holds KH1's {} (equipped or not, KH1's MP), the shortcuts {}; no Triangle offer",
            KH1_LIMITS.iter().map(|l| l.kh2.map_or(l.name.to_string(), |k| format!("{} (for {k})", l.name))).collect::<Vec<_>>().join(", "),
            SHORTCUT_LIMITS.iter().map(|&k| KH1_LIMITS[k].name).collect::<Vec<_>>().join(" / ")
        ));
        return;
    }
    let _ = TEXT.set((
        Box::leak(super::menu::encode("Limit").into_boxed_slice()).as_ptr() as usize,
        LIMITS.iter().map(|l| Box::leak(super::menu::encode(l.name).into_boxed_slice()).as_ptr() as usize).collect(),
        FOLLOW_UPS.iter().map(|f| Box::leak(super::menu::encode(f.1).into_boxed_slice()).as_ptr() as usize).collect(),
    ));
    ORIG_CONTEXT.store(base + CONTEXT_FILL_RVA, Ordering::Release);
    if !redirect_jump(base + CONTEXT_JMP_RVA, base + CONTEXT_FILL_RVA, context_fill as *const () as usize, "context fill (Limit Form's Triangle)") {
        log("Limit Form: the context fill wasn't as expected: no Triangle follow-ups");
    }
    ON.store(true, Ordering::Release);
    log(&format!(
        "Limit Form: the Limit menu ({}; each KH1's 1 bar of MP, KH2's {} MP)",
        LIMITS.iter().map(|l| l.name).collect::<Vec<_>>().join(", "),
        LIMITS.iter().map(|l| l.kh2_mp.to_string()).collect::<Vec<_>>().join(" / ")
    ));
}
