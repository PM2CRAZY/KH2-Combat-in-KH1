//! Limit Form's own settings (the owner, the main-branch port's round 12: "all of limits settings
//! will be under a sub menu called limit in the drive submenu"; Config > Drive > Limit). Notes:
//! `docs/kh1-forms-research-r12.md`.
//!
//! - **Cost** (t972, answer 2 (c): the owner's 3 bars only behind a toggle). KH2 (Checked
//!   (decomp)): Limit Form's command costs 4 bars, as Master's (`crate::drivegauge::cost`, form 3).
//!   KH2 (the default) = 4; 3 Bars = the owner's main-branch cost (s331). Only with Form Cost on
//!   the Drive gauge (MP mode keeps testing's 3 MP for every form).
//! - **Sonic Rave's start on KH1's limit path** (t979; `docs/kh1-drive-forms-native.md` §23). Below.

use super::log;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

/// Config > Drive > Limit > Cost: 0 not read yet, 1 KH2, 2 three bars.
static THREE: AtomicU8 = AtomicU8::new(0);

/// The 3-bar cost is on (the ini's `limit_cost = 3`; `kh2` by default).
pub fn three_bars() -> bool {
    match THREE.load(Ordering::Relaxed) {
        0 => {
            let v = super::settings::get().limit_cost_three;
            THREE.store(1 + v as u8, Ordering::Relaxed);
            v
        }
        v => v == 2,
    }
}

/// Switches Config > Drive > Limit > Cost, kept in the ini.
pub fn set_three_bars(v: bool) {
    THREE.store(1 + v as u8, Ordering::Relaxed);
    super::settings::save("limit_cost", if v { "3" } else { "kh2" });
    log!(Forms, "Config menu: Limit Form's cost {}", if v { "3 bars" } else { "KH2 (4 bars)" });
}

/// What form `form` (KH2's ids) costs in bars now: KH2's, with Limit's 3 when that row is on.
pub fn cost(form: u8) -> u8 {
    if form == crate::formlevel::form::LIMIT && three_bars() {
        3
    } else {
        crate::drivegauge::cost(form)
    }
}

/// Config > Drive > Limit > Trinity Limit (t977, the owner, round 12: "make a toggle for limit to be
/// able to used the 5th kh1 trinity"): 0 not read yet, 1 off, 2 on.
static TRINITY: AtomicU8 = AtomicU8::new(0);

/// KH1's Trinity Limit usable in Limit Form (the ini's `limit_trinity = on`; off by default: KH2's
/// Limit Form has its four only).
pub fn trinity() -> bool {
    match TRINITY.load(Ordering::Relaxed) {
        0 => {
            let v = super::settings::get().limit_trinity;
            TRINITY.store(1 + v as u8, Ordering::Relaxed);
            v
        }
        v => v == 2,
    }
}

/// Switches Config > Drive > Limit > Trinity Limit, kept in the ini; KH1's offer follows
/// (`formlimits`).
pub fn set_trinity(v: bool) {
    TRINITY.store(1 + v as u8, Ordering::Relaxed);
    super::settings::save("limit_trinity", if v { "on" } else { "off" });
    log!(Forms, "Config menu: Trinity Limit in Limit Form {}", if v { "On" } else { "Off" });
    super::formlimits::refresh();
}

/// Config > Drive > Limit > Ledges (t1011; the owner's answer 4: a Config toggle, KH2 by default):
/// 0 not read yet, 1 KH2, 2 KH1.
static LEDGES: AtomicU8 = AtomicU8::new(0);

/// A limit's move off a ledge KH1's way (the ini's `limit_ledges = kh1`).
pub fn ledges_kh1() -> bool {
    match LEDGES.load(Ordering::Relaxed) {
        0 => {
            let v = super::settings::get().limit_ledges_kh1;
            LEDGES.store(1 + v as u8, Ordering::Relaxed);
            v
        }
        v => v == 2,
    }
}

/// Switches Config > Drive > Limit > Ledges, kept in the ini (from the next limit's start).
pub fn set_ledges_kh1(v: bool) {
    LEDGES.store(1 + v as u8, Ordering::Relaxed);
    super::settings::save("limit_ledges", if v { "kh1" } else { "kh2" });
    log!(Forms, "Config menu: Limit Form's ledges {}", if v { "KH1" } else { "KH2" });
}

/// Limit Form's Limit list (t976): KH2's form 3 puts command 0x2C3 "Limit" in Magic's place
/// (`0x1403F29E0`, Checked (decomp)), whose list is its four limits (`limt` rows 27..30, Sora's
/// alone, Checked (files)): Sonic Rave (cmd 698, 60 MP), Last Arcanum (701, 75), Strike Raid (704,
/// 65), Infinity (683, 80). Here each is a KH1 command record of ours (self-cast spell type, as the
/// forms', argument `LIMIT_ARG + k`), on the next free records and words (empty in the player's
/// `btltbl.bin`, Checked (files)): (record, word, name, KH2 command, KH2 MP cost).
pub const LIMITS: [(u8, u16, &str, u16, u8); 4] =
    [(0x81, 0x1FB, "Sonic Rave", 698, 60), (0x82, 0x1FA, "Last Arcanum", 701, 75), (0x83, 0x1F9, "Strike Raid", 704, 65), (0x84, 0x1F8, "Infinity", 683, 80)];
/// The record argument of limit `k` (the forms' are 0x10..).
pub const LIMIT_ARG: u8 = 0x20;
/// The limits' follow-ups (t981), offered on Triangle by the limit's script
/// (`trap_command_override`): KH1 command records of ours after the limits' and words after
/// theirs (the records and words empty in the player's `btltbl.bin`, no record of KH1's names
/// either word, Checked (files)), argument `FOLLOW_ARG + j`: (record, word, name, KH2 command).
/// KH2's commands and names (`cmd` rows, parent 698 at `+0x22`, Checked (files)): Sonic Rave's Rave
/// 699 and Finish 700.
/// Last Arcanum's Bash 702 and The End 703 (t985, records 0x87 / 0x88, words 0x1F5 / 0x1F4, both
/// empty and unnamed too).
/// Infinity's Shoot 684, Impact 685 and Giga Impact 686 (t986, records 0x89..0x8B, words 0x1F3..0x1F1,
/// empty and unnamed too).
pub const FOLLOW_UPS: [(u8, u16, &str, u16); 9] = [
    (0x85, 0x1F7, "Rave", 699),
    (0x86, 0x1F6, "Finish", 700),
    (0x87, 0x1F5, "Bash", 702),
    (0x88, 0x1F4, "The End", 703),
    (0x89, 0x1F3, "Shoot", 684),
    (0x8A, 0x1F2, "Impact", 685),
    (0x8B, 0x1F1, "Giga Impact", 686),
    // (Strike Raid's Raid 705 and Judgment 706, t988: records 0x8C / 0x8D, words 0x1F0 / 0x1EF,
    // empty and unnamed too.)
    (0x8C, 0x1F0, "Raid", 705),
    (0x8D, 0x1EF, "Judgment", 706),
];
pub const FOLLOW_ARG: u8 = 0x28;
/// KH1's command slots (`0x140293000 (slot, record)`: the slot's record if no command took the slot
/// this frame; KH1's limit offer uses slot 3, Triangle, Checked (decomp)).
const OFFER_RVA: usize = 0x293000;
const TRIANGLE: u32 = 3;
/// The top row's name in Limit Form (KH2's 0x2C3).
pub const ROW_NAME: &str = "Limit";

/// Limit `k` chosen from the list (`combat::command`, KH1's command step): Sonic Rave starts on
/// KH1's own limit path (`start`); the other three come in their own versions, KH1's "can't" until
/// then, as for a limit that can't start now.
pub unsafe fn chosen(base: usize, k: usize) {
    let name = LIMITS.get(k).map_or("?", |l| l.2);
    let r = if FIRST_MOTION.get(k).is_some_and(|m| m.is_some()) { start(base, k) } else { Err("not built yet".to_string()) };
    match r {
        Ok(paid) => log!(Forms, "Limit list: {name} started on KH1's limit path ({paid} of KH1's MP units paid)"),
        Err(why) => {
            log!(Forms, "Limit list: {name} chosen, refused ({why}): KH1's \"can't\"");
            let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + super::addr::Rva(0x177F60));
            sound(4);
        }
    }
}

// ---------------------------------------------------------------- KH1's limit path (t979)
//
// KH1 (Steam KH1, Checked (decomp)):
// - A limit's start `0x14028D800 (actor, k)`: KH1's "in a limit" bit 16 of `0x142867364` set (bit 23
//   cleared), its motions loaded, its module started (`0x140397A50 (k, data)`: the limit's effect
//   data kept at `0x142EFB0E0`), its MP paid (`0x1402A5B60`). Each module takes KH1's limit light
//   and fades it in (`0x140397990 (colour)`: the running flag `0x142EFB0D0` = 1, a light of type 1
//   (`0x14029D880 (1, 9, 0)`, kept at `0x142EFB0E8`) faded to `colour` over 30 ticks
//   `0x14029E0D0`), then adds KH1's end task (`0x140397970`: `0x1403979E0` at priority 52000).
// - The end task, each frame while bit 16 is on and 23 off: while an effect element of the
//   limit's data still lives (`0x1402AE0C0 (data)`, the element table `0x142D678B0`) it waits; then
//   the light fades back to white over 30 ticks and is freed (`0x14029DC20`), the running flag 0.
//   KH1's limit step `0x14028D620` then sets bit 23 (running 0) and clears bits 16 / 23 the frame
//   after.
// - Bit 16 is what KH1's other systems read as "in a limit": its battle flag kept on
//   (`0x1402A2F50`), the music held (`0x1401A1A10` / `0x1401A4D00`), its action checks refused
//   (`0x1402A5FC0`), its load reset (`0x14028D4A0`, which stops a running limit), and KH1's own
//   offer (no second limit). KH2 has one limit at a time too (`0x142A24CC0`, `0x1403D88E0`).
//
// Ours runs on that path: KH1's bit 16, KH1's running flag and light, KH1's end task added as KH1's
// modules add it, our mark in KH1's limit data, and KH1's element check answering for it (`alive`):
// the limit lives while its script does (t980: KH2's `limi` as a KH1 script process,
// `formmagic::limit_start`), which plays its moves on KH1's motion requests and rows (`motion`),
// turns him (`face`) and colours KH1's limit light (`light`). KH1 ends it, fades the light back
// and clears its flags itself.

/// Sonic Rave's index in `LIMITS`.
pub const SONIC_RAVE: usize = 0;
/// Each limit's first move (its slot must be built for it to start), when the limit is built: Sonic
/// Rave (t979), Last Arcanum (t985).
/// Infinity (t986).
/// Strike Raid (t988).
const FIRST_MOTION: [Option<u16>; 4] = [Some(super::kh1rows::SONIC_RAVE_MOTIONS[0]), Some(super::kh1rows::LAST_ARCANUM_MOTIONS[0]), Some(super::kh1rows::STRIKE_RAID_MOTIONS[0]), Some(super::kh1rows::INFINITY_MOTIONS[0])];
const FLAGS_RVA: usize = 0x2867364;
const IN_LIMIT: u32 = 0x1_0000;
const LIMIT_DONE: u32 = 0x80_0000;
const RUNNING_RVA: usize = 0x2EFB0D0;
const DATA_RVA: usize = 0x2EFB0E0;
const LIGHT_RVA: usize = 0x2EFB0E8;
/// KH1's element check (`mov [rsp+8], rbx; mov [rsp+10h], rsi; push rdi; sub rsp, 20h`, Checked
/// (exe)).
const ALIVE_RVA: usize = 0x2AE0C0;
const ALIVE_BYTES: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20];
/// KH1's end-task adder (`lea rdx, [0x1403979E0]; mov ecx, 52000; jmp`, Checked (exe)).
const END_TASK_RVA: usize = 0x397970;
const END_TASK_BYTES: [u8; 12] = [0x48, 0x8D, 0x15, 0x69, 0x00, 0x00, 0x00, 0xB9, 0x20, 0xCB, 0x00, 0x00];
/// KH1's light calls (as `drivedim`'s, Checked (exe)).
const NEW_LIGHT_RVA: usize = 0x29D880;
const NEW_LIGHT_BYTES: [u8; 6] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20];
const FADE_RVA: usize = 0x29E0D0;
const FADE_BYTES: [u8; 10] = [0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18];
/// KH1's air states (`+0x70`: 1 jump, 2 fall; 8 flight).
const STATE: usize = 0x70;

/// A light colour for KH1's fade (RGBA floats, 1.0 unchanged).
#[repr(C, align(16))]
struct Colour([f32; 4]);
/// KH2's Limit Form entry the script asks for (`World::entry_id`).
pub const ENTRY_ID: i32 = 2397;
/// KH1's motion request (fresh queue) and queue (Checked (exe): `mov [rsp+8], rbx; push rdi; sub
/// rsp, 20h` both), and a request's action flag and a queued one's "after the last" bit (`form`).
const SET_MOTION_RVA: usize = 0x29F980;
const QUEUE_RVA: usize = 0x29FA30;
const MOTION_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];
const ACTION: u32 = 0x0100_0000;
const QUEUED: u32 = 0x4000_0000;
/// KH1's wanted facing (`+0x150`: KH1's row runner sets it from the stick, `0x140297FB0`, Checked
/// (decomp)).
const WANT_FACING: usize = 0x150;

/// Our mark in KH1's limit data `0x142EFB0E0` (no element of KH1's is ever on it).
static MARK: u8 = 0;
static ORIG_ALIVE: AtomicUsize = AtomicUsize::new(0);
/// One of ours on KH1's path now.
static RUNNING: AtomicBool = AtomicBool::new(false);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn mark() -> usize {
    &MARK as *const u8 as usize
}

/// KH1's element check answering for our limit (`alive`). Only with Drive on and on Steam.
pub unsafe fn install(base: usize) {
    if !super::settings::get().drive || super::addr::Build::current() != super::addr::Build::Steam {
        return;
    }
    super::hook::install(base + ALIVE_RVA, &ALIVE_BYTES, alive as *const () as usize, &ORIG_ALIVE, "KH1's limit element check (Limit Form's limits)");
    box_takes_ours(base);
}

/// t1004: KH1's Triangle box takes our follow-ups the frame they show, as it takes Ars Arcanum's
/// (Steam KH1, Checked (decomp) and (exe)). KH1's command step `0x1402937F0` takes a Triangle press
/// on a record without flag 0x10 only once the box has fully slid in (`0x142852814` == 2, the box's
/// state, `0x140277CF0`: hidden, about 7 frames sliding in, shown, sliding out; a new record slides
/// in again), except Ars Arcanum's 0x58 / 0x59 (`0x140293B46`: `sub dil, 0x58; cmp dil, r14b (1);
/// ja` past the press). KH2's reaction slot has no such wait (Assumption: none found in KH2's), and
/// Last Arcanum's Bash windows are 10 frames (Checked (files)): the slide-in left about 3. The owner
/// (answer 1 (a)): Triangle, taking the press at once. That compare goes to KH1's same test with our
/// follow-up records' range added (`FOLLOW_UPS`, contiguous ids), then back to KH1's two exits; the
/// box still slides in as KH1 draws it. `rdi` is left as KH1's `sub` leaves it, `eax` is dead there
/// (`0x140293B53` tests `ecx`; `0x140293BF1` reads memory; both reload `rax` before using it).
const BOX_EXEMPT_RVA: usize = 0x293B46;
const BOX_EXEMPT_BYTES: [u8; 13] = [0x40, 0x80, 0xEF, 0x58, 0x41, 0x3A, 0xFE, 0x0F, 0x87, 0x9E, 0x00, 0x00, 0x00];
/// Where KH1's compare goes on: the press test (in range) and past it (not).
const BOX_PRESS_RVA: usize = 0x293B53;
const BOX_SKIP_RVA: usize = 0x293BF1;

unsafe fn box_takes_ours(base: usize) {
    let at = base + BOX_EXEMPT_RVA;
    if !super::hook::prologue_is(at, &BOX_EXEMPT_BYTES) {
        log!(General, "KH1's Triangle box press test isn't as expected: our follow-ups wait for its slide-in");
        return;
    }
    let first = FOLLOW_UPS.iter().map(|f| f.0).min().unwrap_or(0);
    let last = FOLLOW_UPS.iter().map(|f| f.0).max().unwrap_or(0);
    if last as usize - first as usize + 1 != FOLLOW_UPS.len() || first < 0x58 + 2 {
        log!(General, "Limit: our follow-up records aren't one run: the Triangle box waits for its slide-in");
        return;
    }
    let Some(cave) = super::hook::alloc_near(at, 64) else { return };
    let mut c: Vec<u8> = Vec::new();
    c.extend_from_slice(&[0x40, 0x80, 0xEF, 0x58]); // sub dil, 0x58 (KH1's own)
    c.extend_from_slice(&[0x40, 0x80, 0xFF, 0x01]); // cmp dil, 1 (KH1's r14b)
    c.extend_from_slice(&[0x76, 0x00]); // jbe press (patched below)
    let j1 = c.len() - 1;
    c.extend_from_slice(&[0x8D, 0x47, (0x58u8).wrapping_sub(first)]); // lea eax, [rdi + 0x58 - first]
    c.extend_from_slice(&[0x3C, last - first]); // cmp al, last - first
    c.extend_from_slice(&[0x76, 0x00]); // jbe press
    let j2 = c.len() - 1;
    let jmp = |c: &mut Vec<u8>, to: usize| {
        c.extend_from_slice(&[0xFF, 0x25, 0, 0, 0, 0]);
        c.extend_from_slice(&(to as u64).to_le_bytes());
    };
    jmp(&mut c, base + BOX_SKIP_RVA);
    let press = c.len();
    jmp(&mut c, base + BOX_PRESS_RVA);
    c[j1] = (press - (j1 + 1)) as u8;
    c[j2] = (press - (j2 + 1)) as u8;
    std::ptr::copy_nonoverlapping(c.as_ptr(), cave as *mut u8, c.len());
    let mut new = [0x90u8; 13];
    new[0] = 0xE9;
    new[1..5].copy_from_slice(&((cave as i64 - (at as i64 + 5)) as i32).to_le_bytes());
    if super::hook::patch(at, &BOX_EXEMPT_BYTES, &new, "KH1's Triangle box press test") {
        log!(General, "KH1's Triangle box takes Limit Form's follow-ups ({first:#x}..={last:#x}) at once, as Ars Arcanum's");
    }
}

// ---------------------------------------------------------------- Strike Raid's thrown Keyblade (t992)
//
// KH1 (Steam KH1, Checked (decomp)): its own Strike Raid throws Sora's Keyblade on his own weapon
// bones (280..292): the limit's state 0x27 (`0x1403A01D0`) sets the actor's `+0` 0x1200080 and its
// `+0x374` 0x20000 (`0x1402A0F70`: no frustum cull, so the far blade still draws), and its throw
// (`0x1403A0290`) makes the attack on the blade point 0x30 (KH1's attack row 0x2F + count), which
// the throw motions carry away from him. KH1 has no weapon object that flies free. KH2's Strike
// Raid flies a weapon object of its own from `limi` (`docs/kh1-strike-raid-research.md`). Here, KH1's
// way: the script's weapon (`World::weapon`) is Sora's own Keyblade; detached, his weapon bones are
// drawn where the script puts it (`thrown_world`, on KH1's pose of him), his cull off as KH1's state
// sets it; attached (the catch), KH1's own hold again. A limit's end (however it ends) puts it back.

/// The thrown Keyblade: its place (KH1's world) and the script's angles (`set_weapon_rot`).
#[derive(Clone, Copy)]
struct Thrown {
    pos: [f32; 3],
    rot: Option<[f32; 3]>,
    /// KH1's frames into the weapon's spin (t1015), once the script started it.
    spin: Option<f32>,
}
static THROWN: std::sync::Mutex<Option<Thrown>> = std::sync::Mutex::new(None);
/// Sora's grip bone's last place (KH1's world), while a limit runs.
static GRIP: std::sync::Mutex<Option<[f32; 3]>> = std::sync::Mutex::new(None);
/// We set Sora's "no cull" bit for the throw (`+0x374` 0x20000).
static NO_CULL: AtomicBool = AtomicBool::new(false);
const NO_CULL_BIT: u32 = 0x2_0000;
const CULL_FLAGS: usize = 0x374;
const GRIP_BONE: usize = 280;
const WEAPON_BONES: usize = 13;

/// The Keyblade is thrown (off his hand) now.
pub fn thrown() -> bool {
    THROWN.lock().is_ok_and(|t| t.is_some())
}

/// The script's weapon back in his hand (`on`) or thrown where it is.
pub unsafe fn attach(on: bool) {
    let actor = super::takeover::sora_actor();
    if on {
        if let Ok(mut b) = BLADE_FX.lock() {
            b.clear();
        }
        let was = THROWN.lock().ok().and_then(|mut t| t.take()).is_some();
        if was {
            cull(actor, false);
            log!(Forms, "Limit: the Keyblade back in his hand");
        }
        return;
    }
    let at = weapon_pos();
    if let Ok(mut t) = THROWN.lock() {
        *t = Some(Thrown { pos: at, rot: None, spin: None });
    }
    cull(actor, true);
    log!(Forms, "Limit: the Keyblade thrown from {at:.0?}");
}

unsafe fn cull(actor: usize, off: bool) {
    if actor < 0x10000 {
        return;
    }
    let f = rd::<u32>(actor + CULL_FLAGS);
    if off && f & NO_CULL_BIT == 0 {
        wr(actor + CULL_FLAGS, f | NO_CULL_BIT);
        NO_CULL.store(true, Ordering::Relaxed);
    } else if !off && NO_CULL.swap(false, Ordering::Relaxed) {
        wr(actor + CULL_FLAGS, f & !NO_CULL_BIT);
    }
}

/// The script's weapon's place (KH1's world): where it flies, else his grip, else his feet.
pub fn weapon_pos() -> [f32; 3] {
    if let Some(t) = THROWN.lock().ok().and_then(|t| *t) {
        return t.pos;
    }
    if let Some(g) = GRIP.lock().ok().and_then(|g| *g) {
        return g;
    }
    let a = super::takeover::sora_actor();
    if a < 0x10000 {
        return [0.0; 3];
    }
    unsafe { rd::<[f32; 3]>(a + 0x10) }
}

/// The script moves the thrown Keyblade (`trap_obj_set_pos` / `trap_obj_move_request`).
pub fn set_weapon_pos(p: [f32; 3]) {
    if let Ok(mut t) = THROWN.lock() {
        if let Some(t) = t.as_mut() {
            if p.iter().all(|v| v.is_finite()) {
                t.pos = p;
            }
        }
    }
}

/// t1015: the script's motion on the thrown Keyblade (`trap_sysobj_motion_start (weapon, m, ..)`,
/// `limi` pc 4145-4155, Checked (files)): Strike Raid's spin, KH2's weapon motion 235 (`K421` of
/// `W_EX010_KH1F.mset`, entry 940): 30 frames at 60 Hz, looping, its bone 2 (the blade, the bone
/// KH2's mesh is skinned to and the one KH1's 282 stands for, `pose::kh2_hold`) turned linearly from
/// -0.785 to 11.781 about its own z: two whole turns every 30 frames (Checked (files)). It was
/// dropped (only Sora's motions were taken), so the blade flew without spinning (the owner, t1012:
/// "strike raids keyblade ... doesn't spin"). The keychain's swings (bones 3..11) and bone 0's
/// quarter turn about y are not carried (Assumption: the keychain's sway is too small to see in the
/// spin; bone 0's turn is left to the script's angles).
pub fn weapon_motion(m: u16) {
    if let Ok(mut t) = THROWN.lock() {
        if let Some(t) = t.as_mut() {
            t.spin = (m == WEAPON_SPIN).then_some(0.0);
        }
    }
}

/// t1016: the script's effects bound to the thrown Keyblade (KH2: `obj_effect_start_bind_other (Sora,
/// 32, weapon)`, `limi` pc 4262-4274 -> 3777; element 32 of Limit Form's archive, its layer on KH2's
/// code 0x4000, the object's root, offset (0, -90, -235) turned -90 degrees about x, Checked (files)
/// and the load log): KH2 binds them to the weapon object's root, which the script turns
/// (`obj_set_xyzrot`) but not its spin (the spin is the weapon's motion under the root). KH1 has no
/// free weapon object, so each is started unbound (KH1's own start) and placed every frame the
/// blade flies at its root, KH1's place of an unbound element (`kh2fx::spell_place`, as Wisdom's
/// shots). The script's unbind and loop end at the catch end them as before.
static BLADE_FX: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());

/// A script effect on the thrown Keyblade: placed on it now and each frame it flies.
pub fn blade_effect(h: u32) {
    if let Ok(mut b) = BLADE_FX.lock() {
        if !b.contains(&h) {
            b.push(h);
        }
    }
    if let Some(t) = THROWN.lock().ok().and_then(|t| *t) {
        place_blade_fx(&t);
    }
}

/// The thrown Keyblade's root in the scripts' frame (y up: its place, the script's angles; Assumption:
/// KH2's `obj_set_xyzrot` order is X, Y, Z, as `thrown_world` reads it), as KH1's place of an unbound
/// element takes it (`formmagic::place`'s flip).
fn place_blade_fx(t: &Thrown) {
    let Ok(b) = BLADE_FX.lock() else { return };
    if b.is_empty() {
        return;
    }
    let q = t.rot.map_or(glam::Quat::IDENTITY, |r| glam::Quat::from_euler(glam::EulerRot::XYZ, r[0], r[1], r[2]));
    let at = glam::Vec3::new(t.pos[0], -t.pos[1], t.pos[2]);
    let m = super::reflect::flip() * glam::Mat4::from_rotation_translation(q, at) * super::reflect::flip();
    if !m.is_finite() {
        return;
    }
    for &h in b.iter() {
        super::kh2fx::spell_place(h, m);
    }
}

/// KH2's weapon spin motion and its turn of the blade a 60 Hz frame (4 pi over 30 frames).
const WEAPON_SPIN: u16 = 235;
const SPIN_FRAMES: f32 = 30.0;
const SPIN_TURN: f32 = 4.0 * std::f32::consts::PI;
/// KH1's blade bone (KH2's weapon bone 2).
const BLADE_BONE: usize = GRIP_BONE + 2;

/// The script turns the thrown Keyblade (`trap_obj_set_xyzrot`).
pub fn set_weapon_rot(r: [f32; 3]) {
    if let Ok(mut t) = THROWN.lock() {
        if let Some(t) = t.as_mut() {
            if r.iter().all(|v| v.is_finite()) {
                t.rot = Some(r);
            }
        }
    }
}

/// On KH1's pose of Sora this frame (`pose`, KH1's world matrices at `world`): his grip's place
/// noted while a limit runs, and while the Keyblade is thrown his weapon bones drawn there, whole
/// (each kept where it is from the grip), turned by the script's angles (KH2's weapon root, whose
/// bones are KH1's 280..291 bone for bone, `docs/kh1-weapon-second.md`; Assumption: their axes
/// match). No limit running: a thrown Keyblade comes back (KH1 is never left without it).
pub unsafe fn thrown_world(world: usize) {
    if !RUNNING.load(Ordering::Relaxed) {
        if thrown() {
            attach(true);
        }
        return;
    }
    let m = |i: usize| glam::Mat4::from_cols_array(&rd::<[f32; 16]>(world + i * 0x40));
    let g = m(GRIP_BONE);
    if !g.is_finite() {
        return;
    }
    let Some(t) = THROWN.lock().ok().and_then(|t| *t) else {
        if let Ok(mut l) = GRIP.lock() {
            *l = Some(g.w_axis.truncate().to_array());
        }
        return;
    };
    // (A throw cut short, Sora hit or his move changed before the catch: KH2's script never takes
    // the weapon back then (open in KH2); KH1 is never left without its Keyblade, so it comes back,
    // and the script's flight, which runs while it isn't attached, ends with it. The owner's
    // KH1-safety rule, not KH2's.)
    let actor = super::takeover::sora_actor();
    let moves = super::kh1rows::STRIKE_RAID_MOTIONS;
    let playing = super::kh2slots::playing_motion(actor).or_else(|| asked_motion(actor));
    if !playing.is_some_and(|m| moves.contains(&m)) {
        log!(Forms, "Limit: the throw was cut short (motion {playing:?}): the Keyblade back in his hand");
        attach(true);
        return;
    }
    let (s, q, _) = g.to_scale_rotation_translation();
    let q = t.rot.map_or(q, |r| glam::Quat::from_euler(glam::EulerRot::XYZ, r[0], r[1], r[2]));
    let to = glam::Mat4::from_scale_rotation_translation(s, q, glam::Vec3::from_array(t.pos)) * g.inverse();
    // (t1015) The spin: the whole Keyblade turned about the blade bone's own z through its origin,
    // as KH2's 235 turns bone 2 (KH1's mesh, skinned to 280..292, turns with it whole), KH1's frames
    // on from the script's start of it.
    let spin = match t.spin {
        Some(f) => {
            let step = super::formmagic::frame_scale().max(1.0);
            if let Ok(mut l) = THROWN.lock() {
                if let Some(l) = l.as_mut() {
                    l.spin = Some((f + step) % SPIN_FRAMES);
                }
            }
            let blade = to * m(BLADE_BONE);
            blade.is_finite().then(|| blade * glam::Mat4::from_rotation_z(f / SPIN_FRAMES * SPIN_TURN) * blade.inverse())
        }
        None => None,
    };
    for i in GRIP_BONE..GRIP_BONE + WEAPON_BONES {
        let n = to * m(i);
        let n = spin.map_or(n, |r| r * n);
        if n.is_finite() {
            wr(world + i * 0x40, n.to_cols_array());
        }
    }
    // (t1016) Its effects on its root.
    place_blade_fx(&t);
}

/// The Limit list greyed now (t990): KH2's grey rule `0x1403D88E0` (Steam KH2, Checked (decomp))
/// gives every limit state 5 (greyed) while a limit runs (`0x142A24CC0`) or MP Charge runs; here a
/// limit of ours or KH1's own (KH1's bit 16 or its running flag), or no MP at all (KH2 has no MP only
/// while MP Charge runs; MP Charge isn't built). Under the cost it stays usable (state 2, answer
/// 4 (a)).
pub unsafe fn greyed(base: usize, actor: usize) -> bool {
    if base == 0 || actor < 0x10000 {
        return false;
    }
    if RUNNING.load(Ordering::Relaxed) || rd::<u32>(base + FLAGS_RVA) & IN_LIMIT != 0 || rd::<u32>(base + RUNNING_RVA) != 0 {
        return true;
    }
    let stats = super::pose::ptr(rd::<u32>(actor + 0x6C));
    stats >= 0x10000 && rd::<i32>(stats + 0x44) <= 0 && rd::<u16>(stats + 0xBE) == 0
}

/// A limit of ours runs on KH1's path now.
pub fn running() -> bool {
    RUNNING.load(Ordering::Relaxed)
}

/// KH1's element check: for our mark, whether our limit still runs (non-null) or is over (null,
/// and KH1's end task ends it); for anything else KH1's own.
unsafe extern "system" fn alive(data: usize) -> usize {
    if data == mark() && data != 0 {
        if super::formmagic::limit_alive() {
            return mark();
        }
        if RUNNING.swap(false, Ordering::Relaxed) {
            set_provider(super::takeover::game_base(), false);
            leave_state(super::takeover::game_base());
            // (t1014) Its effects' sounds that are still tied go with it.
            super::kh2sound::element_sounds_stop_all();
            log!(Forms, "Limit: its script is over: KH1's end task ends the limit (its light back over 30 ticks)");
        }
        return 0;
    }
    let f: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(ORIG_ALIVE.load(Ordering::Relaxed));
    f(data)
}

/// The KH2 motion of the row of ours KH1 has been asked for on `actor`, if one is (KH1 applies a
/// request at its next motion pass; KH2's start plays it at once).
///
/// t1003: a request KH1 hasn't applied yet first (Steam KH1, Checked (decomp)): KH1's motion request
/// `0x14029F980` and row start `0x1402A3980` only write the request (`+0x17C`, the head of the
/// queue) and its "new request" bit (`+0x160` bit 0); KH1's motion step (`0x14029F0A0` ->
/// `0x14029FC40`) copies it into `+0x164` later. Read from `+0x164` alone, a follow-up chosen in
/// KH1's command step (Last Arcanum's Bash, trigger 0x17: its hook starts 222) was seen by the next
/// script step as the move before (221) when that step came before KH1's motion step, so `limi`'s
/// main loop (it runs while Sora's motion is the one its last hook started) ended the limit while
/// KH1 went on to play the Bash (the owner, t1001: "plays the next attack but ends the limit").
pub unsafe fn asked_motion(actor: usize) -> Option<u16> {
    if actor < 0x10000 {
        return None;
    }
    let pending = rd::<u32>(actor + 0x160) & 1 != 0;
    let asked = rd::<u32>(actor + if pending { 0x17C } else { 0x164 }) & 0xFFFF;
    let n = asked.checked_sub(super::kh1rows::FIRST_REQUEST & 0xFFFF)? as usize;
    super::kh1rows::OURS.get(n).map(|o| o.motion)
}

/// A limit script's motion start or push on Sora (`trap_sysobj_motion_start` / `_push`, Steam KH2
/// `0x14042B320` → `0x1403C86A0` / `0x14042B3F0` → `0x1403C8310`, Checked (decomp)), on KH1's own:
/// KH2's motion 0 is KH1's idle (request 0); any other a row of ours of the form's (`kh1rows`, its
/// slot built). A start of an attack row is KH1's row start (its attack object, `kh1rows::start_row`),
/// of any other KH1's motion request with its action flag; a push KH1's queue. Returns whether KH1
/// was asked. Assumption: KH1's row start takes no blend (KH2's 8-frame blend of the Rave's restart
/// is lost there).
pub unsafe fn motion(m: u16, blend: f32, push: bool) -> bool {
    let actor = super::takeover::sora_actor();
    let base = super::takeover::game_base();
    if actor < 0x10000 || base == 0 || !super::hook::prologue_is(base + SET_MOTION_RVA, &MOTION_BYTES) || !super::hook::prologue_is(base + QUEUE_RVA, &MOTION_BYTES) {
        return false;
    }
    if m == KH2_FALL {
        return fall(base, actor, blend, push);
    }
    let row = if m == 0 { None } else { super::kh1rows::by_motion(m) };
    let request = match row {
        None if m == 0 => 0,
        Some(n) if super::kh2slots::has(super::kh1rows::request(n) as usize, m) => super::kh1rows::request(n),
        _ => {
            log!(Forms, "Limit: its script asked for KH2 motion {m}, which has no slot of ours: not played");
            return false;
        }
    };
    let blend = if blend.is_finite() { blend.max(0.0) } else { 0.0 };
    if push {
        let queue: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + QUEUE_RVA);
        queue(actor, QUEUED | request, blend);
        return true;
    }
    if let Some(n) = row.filter(|&n| super::kh1rows::OURS[n].attack) {
        return super::kh1rows::start_row(actor, n, false);
    }
    let set: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
    set(actor, request | ACTION, blend);
    true
}

/// KH2's motion 4, the fall (A004 of Limit Form's moveset, Checked (files); `0x1403D5010` /
/// `0x1403D6430` ask for it as the fall, Checked (decomp)).
const KH2_FALL: u16 = 4;
/// KH1's fall motion request and the callback its Ragnarok queues with it (Steam KH1, Checked
/// (decomp) and (exe)): `0x14039F7A0` / `0x14039F330` queue motion 6 with bit 30 and the entry's
/// callback (`+0xC`, a handle made by `0x14038ADD0`) `0x14039FFB0` (`mov edx, 2; jmp 0x1402B1960`:
/// KH1's fall state 2), which KH1's end rule `0x14029F6E0` runs as that entry starts.
const KH1_FALL: u32 = 6;
const HANDLE_RVA: usize = 0x38ADD0;
const HANDLE_BYTES: [u8; 9] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9];
const TO_FALL_RVA: usize = 0x39FFB0;
const TO_FALL_BYTES: [u8; 6] = [0xBA, 0x02, 0x00, 0x00, 0x00, 0xE9];
/// KH1's fall state.
const FALL_STATE: i32 = 2;

/// t1006: KH2's fall asked by a limit's script (Infinity pushes it after every hop, after the hover
/// and after Shoot / Impact / Giga Impact: `limi` pc 1740.., 2296, 2461, 2548, Checked (files)), on
/// KH1's Ragnarok way: KH1's motion 6 queued (a push) with the callback that puts him in KH1's fall
/// state when the move before it ends; a start puts him there now. KH1's fall then lands him
/// (the fall state's own landing), our state's leave runs from KH1's set state, and the script,
/// seeing a move not of the limit's, ends. Until t1005 motion 4 had no row of ours and was dropped,
/// so after Shoot (or the hover with no press) KH1 played the move round again with nothing queued
/// and the script's main loop (pc 5820: while his motion is one of 241..248) never ended (the
/// owner, t1001: "infinity gets you stuck ... you get stuck in the shoot animation"). A spin started
/// after it takes KH1's queue over (`0x14029F980`: the request first, count 1), as Ragnarok's next
/// hop does.
unsafe fn fall(base: usize, actor: usize, blend: f32, push: bool) -> bool {
    use super::hook::prologue_is as is;
    if !is(base + HANDLE_RVA, &HANDLE_BYTES) || !is(base + TO_FALL_RVA, &TO_FALL_BYTES) || !is(base + SET_STATE_RVA, &SET_STATE_BYTES) {
        log!(Forms, "Limit: KH1's fall calls aren't as expected: its script's fall not played");
        return false;
    }
    let blend = if blend.is_finite() { blend.max(0.0) } else { 0.0 };
    if push {
        let queue: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + QUEUE_RVA);
        let entry = queue(actor, QUEUED | KH1_FALL, blend);
        let handle: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(base + HANDLE_RVA);
        if entry >= 0x10000 {
            wr(entry + 0xC, handle(base + TO_FALL_RVA));
        }
        return true;
    }
    let set: unsafe extern "system" fn(usize, i32) -> u64 = std::mem::transmute(base + SET_STATE_RVA);
    set(actor, FALL_STATE);
    let request: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
    request(actor, KH1_FALL, blend);
    true
}

/// A limit script's follow-up window (`trap_obj_motion_check_range (obj, n)`, Steam KH2
/// `0x14042D1E0`, Checked (decomp)): script range `n` of the KH2 clip KH1 plays in our slot on Sora
/// open at KH1's time in it (`WindowKind::ScriptRange`, t978).
pub unsafe fn range_open(n: u8) -> bool {
    let actor = super::takeover::sora_actor();
    if actor < 0x10000 {
        return false;
    }
    let Some((m, t)) = super::kh2slots::now_playing(actor) else { return false };
    let Some(clip) = super::pose::kh2_moves().and_then(|c| c.moveset.clips.get(&m)) else { return false };
    t >= 0.0 && clip.has(t as u32, |k| matches!(k, character_core::WindowKind::ScriptRange(x) if *x == n))
}

/// A limit script's follow-up offer (`trap_command_override (1, command, obj, 1)`, Steam KH2
/// `0x140430010` → `0x1403B2850`, Checked (decomp)): our record for `command` in KH1's Triangle slot
/// this frame, as KH1's limit offer puts a limit there (`0x140293000 (3, record)`). The script asks
/// each frame its window is open; KH1's slot is offered anew each frame (Assumption: a frame
/// without an offer leaves Triangle to KH1's own).
///
/// t997, KH1's way (Steam KH1, Checked (decomp)): KH1's command fill `0x1402948F0` clears the slots
/// and their "taken" bits each frame (`0x142D53A3C` = 0, slot 3's byte `0x140528917` = 0) and then
/// calls the one reaction provider KH1's limit states set (`0x140293180 (fn)` → `0x142D53AA0`;
/// Sonic Blade's `0x14039ED70`, Ars Arcanum's `0x14039F1E0`, Strike Raid's `0x1403A0230`, each
/// offering its follow-up with `0x140293000 (slot, record)` from there, set on the state's enter
/// and cleared on its leave). An offer made from anywhere else is wiped by the next fill, so ours is
/// noted here (the frame the script or the move asked for it) and made by our provider, set as
/// KH1's limits set theirs while one of ours runs (`provider`).
pub unsafe fn offer(command: u16) {
    let Some(f) = FOLLOW_UPS.iter().find(|f| f.3 == command) else {
        log!(Forms, "Limit: its script offered KH2 command {command}, which has no record of ours");
        return;
    };
    if !RUNNING.load(Ordering::Relaxed) {
        return;
    }
    if let Ok(mut w) = WANT.lock() {
        *w = Some((f.0, super::takeover::GAME_FRAME.load(Ordering::Relaxed)));
    }
}

/// The follow-up asked for (its record) and the game frame it was asked on.
static WANT: std::sync::Mutex<Option<(u8, u64)>> = std::sync::Mutex::new(None);
/// KH1's reaction provider setter (`mov [rip+..], rcx; ret`, Checked (exe)) and its pointer.
const PROVIDER_SET_RVA: usize = 0x293180;
const PROVIDER_SET_BYTES: [u8; 8] = [0x48, 0x89, 0x0D, 0x19, 0x09, 0xAC, 0x02, 0xC3];
const PROVIDER_RVA: usize = 0x2D53AA0;

/// Our reaction provider (KH1's command fill calls it each frame, `0x1402948F0`): the follow-up
/// the script or the move asked for this frame or the last (the script steps after the fill or
/// before it), on Triangle, as KH1's limits offer theirs.
///
/// t1001, with KH1's combo window open for the frame it's offered (Steam KH1, Checked (decomp) and
/// (exe)): KH1's use of a slot's command (`0x140293480`) refuses a magic-kind record with an
/// argument (ours, kind 2) and KH1's own limit follow-ups alike unless KH1's act test
/// `0x1402A2EC0 (sora)` passes: Sora free (`0x140297870`: `+0` bit 4, `+0x88` bit 0x20, `+0x444`
/// bit 0 all clear) or in his combo window (`0x14029EFB0`: `+0x1CC` bit 22, the playing motion's
/// event bits, copied from its instance's `+0x14` each motion step). Every limit move is an action
/// request (`0x14029F980`, bit 24 → `+0 |= 4`), so he is never free in one; KH1's own limit moves
/// carry the window in their motion data, and their providers keep the offer only while it's open
/// (Sonic Blade's `0x14039ED70`: `0x14029EFB0`, else `0x140292FB0 (0)` takes it back). Our KH2
/// motions carry none, so a follow-up shown on Triangle was refused when pressed (the owner, t999:
/// "sonic rave and last arcanums reaction commands won't work"). The window here is KH2's own,
/// the script's or the move's offer: bit 22 set on `+0x1CC` as the offer is placed. The fill runs
/// inside KH1's command step (`0x1402937F0` → `0x140294220` → `0x1402948F0`) before that step reads
/// the Triangle press (`0x140293480`, same call), and the next motion step writes `+0x1CC` anew, so
/// it holds for that frame only. The Attack slot stays greyed in our state (its flags, 4, have no
/// bit 4: `0x140294220`), so no KH1 combo swing takes the window.
unsafe extern "system" fn provider() {
    let base = super::takeover::game_base();
    if base == 0 || !RUNNING.load(Ordering::Relaxed) {
        return;
    }
    let now = super::takeover::GAME_FRAME.load(Ordering::Relaxed);
    let Some((rec, at)) = WANT.lock().ok().and_then(|w| *w) else { return };
    if now.saturating_sub(at) > 1 {
        return;
    }
    let slot: unsafe extern "system" fn(u32, u8) -> u64 = std::mem::transmute(base + OFFER_RVA);
    if slot(TRIANGLE, rec) == 0 {
        return;
    }
    let actor = super::takeover::sora_actor();
    if actor >= 0x10000 {
        wr(actor + EVENT_BITS, rd::<u32>(actor + EVENT_BITS) | super::combomods::KH1_COMBO_WINDOW);
    }
    if SHOWN.lock().ok().map(|mut s| s.replace(rec)) != Some(Some(rec)) {
        log!(Forms, "Limit: follow-up record {rec:#x} on Triangle, KH1's combo window open for it");
    }
}

/// Sora's motion event bits (`+0x1CC`, bit 22 KH1's combo window; Checked (decomp)).
const EVENT_BITS: usize = 0x1CC;
/// The last follow-up record our provider placed this limit (for the log only).
static SHOWN: std::sync::Mutex<Option<u8>> = std::sync::Mutex::new(None);

// ---------------------------------------------------------------- the limit's KH1 state (t999)
//
// KH1 (Steam KH1, Checked (decomp)): each of KH1's limits registers a state of its own in KH1's
// state table (`0x1402B1B80 (id, flags, enter, leave, pre-step, step, transition, .., jump)`, the
// table `0x142DB3840`, 0x40 an entry) as it starts and puts Sora in it (`0x1402B1960 (actor, id)`:
// the old state's leave `(actor)`, then the new one's enter `(actor)`): Sonic Blade 0x20, Ars
// Arcanum 0x20, Ragnarok 0x25, Strike Raid 0x27 (flags 4; enter `0x1403A01D0`: its reaction
// provider set, his `+0` |= 0x1200080, cull off `0x1402A0F70`; leave `0x1403A01A0`: provider
// cleared, the flags and cull back `0x1402A0F80`, KH1's end task added `0x140397970`; transition
// `0x1402B1840`, `mov eax, [rcx+0x70]; ret`: he stays in it on the ground or in the air), and its
// last move's end puts him back in state 0. Ours the same: `LIMIT_STATE` registered by KH1's own
// call each start with KH1's own stay-transition, our enter / leave (the provider and the cull, by
// KH1's own calls) and a step of ours (KH2's limit moves move him by their root travel alone, which
// KH1 adds after the step (`0x14029A3C0`, `+0x200`): no walk velocity and no gravity of the ground
// step; Assumption: KH2's limit actions add none either, Infinity's rise and hover being its
// motions'). The limit's end (`alive`) puts him back in state 0 (KH1's ground state, whose own
// transition drops him into the air state with no floor). Not KH1's `+0` 0x200080 bits: KH2's
// invulnerability is its motions' range 27 (our slots), not the whole limit.

/// Our limit state's id: free in KH1's table (KH1 registers 0..0x22, 0x25..0x29, 0x32; ours 0x2A..0x2C,
/// `quickrun` / `magnet`; Checked (decomp) and (code)).
pub const LIMIT_STATE: i32 = 0x2D;
const STATE_FLAGS: u32 = 4;
const REGISTER_RVA: usize = 0x2B1B80;
const REGISTER_BYTES: [u8; 14] = [0x48, 0x8D, 0x05, 0xB9, 0x1C, 0xB0, 0x02, 0x4C, 0x63, 0xD1, 0x49, 0xC1, 0xE2, 0x06];
const SET_STATE_RVA: usize = 0x2B1960;
const SET_STATE_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20];
const STAY_RVA: usize = 0x2B1840;
const STAY_BYTES: [u8; 4] = [0x8B, 0x41, 0x70, 0xC3];
const CULL_OFF_RVA: usize = 0x2A0F70;
const CULL_ON_RVA: usize = 0x2A0F80;
const CULL_OFF_BYTES: [u8; 10] = [0x81, 0x89, 0x74, 0x03, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00];
const CULL_ON_BYTES: [u8; 10] = [0x81, 0xA1, 0x74, 0x03, 0x00, 0x00, 0xFF, 0xFF, 0xFD, 0xFF];

unsafe fn state_calls_ok(base: usize) -> bool {
    use super::hook::prologue_is as is;
    is(base + REGISTER_RVA, &REGISTER_BYTES) && is(base + SET_STATE_RVA, &SET_STATE_BYTES) && is(base + STAY_RVA, &STAY_BYTES) && is(base + CULL_OFF_RVA, &CULL_OFF_BYTES) && is(base + CULL_ON_RVA, &CULL_ON_BYTES)
}

/// Our state's enter (KH1's `0x1402B1960` calls it with the actor): KH1's limit states' own work,
/// the reaction provider and the cull off.
unsafe extern "system" fn state_enter(actor: usize) {
    let base = super::takeover::game_base();
    set_provider(base, true);
    let cull_off: unsafe extern "system" fn(usize) = std::mem::transmute(base + CULL_OFF_RVA);
    cull_off(actor);
    log!(Forms, "Limit: Sora in KH1's limit state {LIMIT_STATE:#x}");
}

/// t1009: Infinity's hover, KH2's no-gravity range (range 3: 246 16..100, 247 / 248 0..50, Checked
/// (files); KH2's range handler `0x14040D810` case 3 sets the motion's `+0x18C` bit 5, which holds
/// his fall) on KH1's jump step's own hold, as KH1's Ragnarok holds its hover (Checked (decomp)):
/// `+0` bit 12 (0x1000, which its request bit 23 sets, `0x14029FC40`) makes `0x1402AF6A0` add no
/// movement and keep the arc at rest from where he is. KH1's level reader gives the range on our
/// slot (`kh2slots::no_gravity`). As Ragnarok zeroes the jump fields before its hold
/// (`0x14039F7A0`, count 5), the held arc's height and air speed are zeroed when the hold starts
/// (the step's hold keeps `+0x3BC`, which its fall would add back at release), so he falls from rest
/// when it ends. Each frame one of ours runs on the jump step (`formmagic`'s limit step).
pub unsafe fn hover(actor: usize) {
    let held = HOVER.load(Ordering::Relaxed);
    let on = actor >= 0x10000 && JUMPS.load(Ordering::Relaxed) && RUNNING.load(Ordering::Relaxed) && rd::<i32>(actor + 0x70) == LIMIT_STATE && super::kh2slots::no_gravity(actor);
    if on && !held {
        wr(actor, rd::<u32>(actor) | AIR_HOLD);
        wr(actor + 0x3BC, -0.0f32);
        wr(actor + 0x3C0, [0.0f32; 4]);
        HOVER.store(true, Ordering::Relaxed);
        log!(Forms, "Limit: KH2's no-gravity range: held on KH1's jump step");
    } else if !on && held {
        release_hover(actor);
    }
}

/// t1010: a limit script's movement (`trap_obj_set_movement (obj, dir, speed)`, KH2's move direction
/// `+0x10` and speed `+0x1C`, `0x1403B9DA0`, Checked (decomp)) on KH1's own pair (the move direction
/// `+0x20`, its speed `+0x2C`) and, on KH1's jump step (Infinity), its air speed `+0x3C0`, which that
/// step moves him by (`0x1402AF6A0`, Checked (decomp)). Infinity's Shoot / Impact / Giga Impact set 0
/// (stand still) before their push back (`limi` pc 2258, 2423, 2510, Checked (files)); it was
/// nothing, so a spin's sideways speed could carry on into the recoil.
pub unsafe fn movement(x: f32, z: f32, speed: f32) {
    let actor = super::takeover::sora_actor();
    if actor < 0x10000 || !RUNNING.load(Ordering::Relaxed) || !(x.is_finite() && z.is_finite() && speed.is_finite()) {
        return;
    }
    let n = x.hypot(z);
    let d = if n > 1e-6 { [x / n, 0.0, z / n] } else { rd::<[f32; 3]>(actor + 0x20) };
    wr(actor + 0x20, d);
    wr(actor + 0x2C, speed);
    if JUMPS.load(Ordering::Relaxed) {
        wr(actor + 0x3C0, [d[0] * speed, 0.0, d[2] * speed, speed.abs()]);
    }
}

/// KH1's jump step's hold (`+0` bit 12) and whether ours holds him.
const AIR_HOLD: u32 = 0x1000;
static HOVER: AtomicBool = AtomicBool::new(false);

unsafe fn release_hover(actor: usize) {
    if HOVER.swap(false, Ordering::Relaxed) && actor >= 0x10000 {
        wr(actor, rd::<u32>(actor) & !AIR_HOLD);
    }
}

/// Its leave: the provider cleared, the cull back, a thrown Keyblade back in his hand.
unsafe extern "system" fn state_leave(actor: usize) {
    release_hover(actor);
    let base = super::takeover::game_base();
    set_provider(base, false);
    let cull_on: unsafe extern "system" fn(usize) = std::mem::transmute(base + CULL_ON_RVA);
    cull_on(actor);
    NO_CULL.store(false, Ordering::Relaxed);
    if thrown() {
        attach(true);
    }
    log!(Forms, "Limit: Sora out of KH1's limit state");
}

/// Its step `(actor, &displacement)`: none (the moves' root travel moves him, added by KH1 after).
/// t1011, Ledges at KH2: KH2's edge stop on it (range 29: Sonic Rave's 212 at 0..26 and 213 at
/// 0..32, Checked (files)), as on KH1's ground state (`shared::physics::limit_edge_stop`).
unsafe extern "system" fn state_step(actor: usize, disp: *mut [f32; 4]) -> usize {
    if !disp.is_null() {
        wr(disp as usize, [0.0f32; 4]);
        if !ledges_kh1() {
            super::shared::physics::limit_edge_stop(actor, disp);
        }
    }
    0
}

/// t1011: off a ledge (Steam KH1, Checked (decomp) and (exe)). KH1's step dispatcher (`0x14029A3C0`)
/// calls the state's transition `(actor, no floor)` after its collision; KH1's Sonic Blade's
/// (`0x14039EBA0`, `40 53 48 83 EC 20 48 8B D9 85 D2 74 25`) with no floor
/// clears the queue (`0x14029F780`), asks for the fall motion 6 (blend 8) and returns the fall state 2,
/// whose leave of the limit's state ends it. Ledges at KH1: that transition itself. At KH2: KH2's
/// mover drops him into its fall (motion 4: the script's main loop then ends the limit) only with
/// no floor and none of the ranges that keep him off it (no gravity, ids 3 / 24; the edge stop, 29:
/// `kh2slots::ground_hold`, Checked (decomp) `0x1403B9090`), so the same fall, only then.
const KH1_LEDGE_RVA: usize = 0x39EBA0;
const KH1_LEDGE_BYTES: [u8; 13] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0x85, 0xD2, 0x74, 0x25];

unsafe extern "system" fn state_transition(actor: usize, no_floor: i32) -> i32 {
    let base = super::takeover::game_base();
    if no_floor != 0 && base != 0 && super::kh2slots::ground_hold(actor).is_none() && super::hook::prologue_is(base + KH1_LEDGE_RVA, &KH1_LEDGE_BYTES) {
        let ledge: unsafe extern "system" fn(usize, i32) -> i32 = std::mem::transmute(base + KH1_LEDGE_RVA);
        log!(Forms, "Limit: no floor under him: KH1's fall, the limit ends (Ledges KH2)");
        return ledge(actor, no_floor);
    }
    rd::<i32>(actor + 0x70)
}

/// Our state registered with KH1's own call (as KH1's limits register theirs at their start) and
/// Sora put in it. False: a call isn't as expected (the limit then runs in the state he's in).
unsafe fn enter_state(base: usize, actor: usize, k: usize) -> bool {
    if !state_calls_ok(base) {
        log!(Forms, "Limit: KH1's state calls aren't as expected: the limit stays in his state");
        return false;
    }
    // (t1008) Infinity's state steps on KH1's jump step, as KH1's Ragnarok's state 0x25 does
    // (`0x1402B1B80 (0x25, 5, .., step 0x1402AF6A0, ..)`, Checked (decomp)): its spins are hops
    // (`party_attack`). The others keep ours (their moves move him by their travel alone).
    let jumps = LIMITS.get(k).is_some_and(|l| l.3 == INFINITY) && super::hook::prologue_is(base + JUMP_STEP_RVA, &JUMP_STEP_BYTES);
    JUMPS.store(jumps, Ordering::Relaxed);
    let step = if jumps { base + JUMP_STEP_RVA } else { state_step as *const () as usize };
    // (t1011) Off a ledge: Infinity stays (its rise and fall are its own, its end KH1's fall);
    // the others by Config > Drive > Limit > Ledges.
    let transition = if jumps {
        base + STAY_RVA
    } else if ledges_kh1() && super::hook::prologue_is(base + KH1_LEDGE_RVA, &KH1_LEDGE_BYTES) {
        base + KH1_LEDGE_RVA
    } else {
        state_transition as *const () as usize
    };
    if jumps {
        // (Ragnarok's enter `0x14039FAF0`: the jump fields at rest from where he is, Checked (decomp).)
        let y: f32 = rd(actor + 0x14);
        wr(actor + 0x3C0, [0.0f32; 4]);
        wr(actor + 0x3B0, 0.0f32);
        wr(actor + 0x3B8, 0.0f32);
        wr(actor + 0x3BC, -0.0f32);
        wr(actor + 0x3D0, 0u32);
        if y.is_finite() {
            wr(actor + 0x3B4, y);
        }
    }
    let register: unsafe extern "system" fn(i32, u32, usize, usize, usize, usize, usize, usize, usize) = std::mem::transmute(base + REGISTER_RVA);
    register(LIMIT_STATE, STATE_FLAGS, state_enter as *const () as usize, state_leave as *const () as usize, 0, step, transition, 0, 0);
    let set: unsafe extern "system" fn(usize, i32) -> u64 = std::mem::transmute(base + SET_STATE_RVA);
    set(actor, LIMIT_STATE);
    true
}

/// KH2's Infinity (its command) and KH1's jump step (Steam KH1, `mov [rsp+10h], rbx; push rdi;
/// sub rsp, 40h; test dword [rcx], 1000h`, Checked (exe)).
const INFINITY: u16 = 683;
const JUMP_STEP_RVA: usize = 0x2AF6A0;
const JUMP_STEP_BYTES: [u8; 16] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x40, 0xF7, 0x01, 0x00, 0x10, 0x00, 0x00];
/// The limit running steps on KH1's jump step (Infinity).
static JUMPS: AtomicBool = AtomicBool::new(false);

/// Out of our state at the limit's end: KH1's ground state (its own transition drops him into the
/// air state where there's no floor), as KH1's limits end in state 0.
unsafe fn leave_state(base: usize) {
    let actor = super::takeover::sora_actor();
    if base == 0 || actor < 0x10000 || rd::<i32>(actor + 0x70) != LIMIT_STATE || !state_calls_ok(base) {
        return;
    }
    let set: unsafe extern "system" fn(usize, i32) -> u64 = std::mem::transmute(base + SET_STATE_RVA);
    set(actor, 0);
}

/// Our provider set (`on`, a limit of ours starts) or KH1's cleared (it ends), as KH1's limit states
/// set and clear theirs; cleared only while ours is the one set.
unsafe fn set_provider(base: usize, on: bool) {
    if base == 0 || !super::hook::prologue_is(base + PROVIDER_SET_RVA, &PROVIDER_SET_BYTES) {
        return;
    }
    let set: unsafe extern "system" fn(usize) = std::mem::transmute(base + PROVIDER_SET_RVA);
    let ours = provider as *const () as usize;
    if on {
        set(ours);
    } else if rd::<usize>(base + PROVIDER_RVA) == ours {
        set(0);
    }
    if let Ok(mut w) = WANT.lock() {
        *w = None;
    }
    if let Ok(mut s) = SHOWN.lock() {
        *s = None;
    }
}

/// The motion-data reaction offers of the move playing on Sora (`WindowKind::Command`, KH2's range
/// 20, t985): each frame a limit runs (`formmagic`'s step), an open one's command offered on
/// Triangle (`offer`), as KH2's range step offers it each frame (Checked (decomp)).
pub unsafe fn window_offers(actor: usize) {
    if actor < 0x10000 || !RUNNING.load(Ordering::Relaxed) {
        return;
    }
    let Some((m, t)) = super::kh2slots::now_playing(actor) else { return };
    let Some(clip) = super::pose::kh2_moves().and_then(|c| c.moveset.clips.get(&m)) else { return };
    if t < 0.0 {
        return;
    }
    let f = t as u32;
    let open: Vec<u16> = clip.windows.iter().filter(|w| w.contains(f)).filter_map(|w| match w.kind {
        character_core::WindowKind::Command(c) => Some(c),
        _ => None,
    }).collect();
    for c in open {
        offer(c);
    }
}

/// A limit script's party attack on Sora (`trap_obj_party_attack (obj, row)`, Steam KH2
/// `0x14053EF20`: PTYA row `row` of his moveset, its motion started, `0x1403D78D0`, Checked
/// (decomp)): Limit Form's PTYA row `row`'s motion on its row of ours (`motion`). Infinity's spins,
/// rows 52..56 (Checked (files)). Assumption: KH2's lunge toward the target (the row's speeds and
/// distances) is left to the motion's own travel.
///
/// t1007: as KH2's party attack (`0x1403D78D0`, Checked (decomp)) the row's next motion is pushed
/// after it with the row's blend (Infinity's rows 52..56: motion 4, the fall, `fall`), and the motion
/// started is handed back so the script's view of Sora has it (`KhWorld::party_attack`, as a motion
/// start does). The view kept the move before, so `limi` (pc 1701..1714: it restarts the motion the
/// party attack gave) started Infinity's 241 again instead of the spin (Checked (code) and (files)).
pub unsafe fn party_attack(row: u16) -> Option<u16> {
    let r = super::pose::kh2().and_then(|k| k.limit_form.as_ref()).and_then(|f| f.character.moveset.combo_rows.iter().find(|r| r.id as u16 == row).cloned());
    let Some(r) = r else {
        log!(Forms, "Limit: its script's party attack row {row} isn't in Limit Form's moveset");
        return None;
    };
    if !motion(r.motion, 0.0, false) {
        return None;
    }
    // (t1008) The row's hop, KH2's (`0x1403D78D0` -> the jump start: the row's `jump` with no
    // target, else the target's height within `jump_min .. jump_max`, the sideways speed the gap
    // plus `distance_adjust` over KH2's rise time within `speed_min .. speed_max`; Checked (decomp),
    // `character_core::HostPress::hop` on KH1's target as for the combo rows), on KH1's jump step
    // in the limit's state (`JUMP_STEP`), as KH1's Ragnarok hops each spin (`0x14039F7A0`: its row's
    // height, `+0x3B8` the rise time, `+0x3BC` minus the height, `+0x3B4` his y, `+0x3C0` the
    // speed; Checked (decomp)). The owner's answer 3 (a): KH1's jump step, KH2's numbers (the
    // gravity the Physics row's, as KH2's other hops on KH1).
    let actor = super::takeover::sora_actor();
    if r.is_jump() && actor >= 0x10000 && rd::<i32>(actor + 0x70) == LIMIT_STATE && JUMPS.load(Ordering::Relaxed) {
        if let Some(l) = super::combomods::Kh1Press::read(actor, true).and_then(|p| p.hop(&r, false)) {
            super::shared::physics::limit_hop(actor, l.height, l.velocity);
            log!(Forms, "Limit: party attack row {row} (motion {}): KH2's hop, {:.0} high, {:.1}/frame sideways, on KH1's jump step", r.motion, l.height, l.velocity[0].hypot(l.velocity[1]));
        }
    }
    if r.next_motion != 0 {
        motion(r.next_motion, r.blend_time, true);
    }
    Some(r.motion)
}

/// A limit script's motion-end check (`trap_sysobj_motion_is_end (obj)`, Steam KH2 `0x14042B440`:
/// the motion block's `+0x18C` bit 1, set as the motion playing reaches its end, Checked (decomp)):
/// the clip KH1 plays in our slot on Sora at or past its last frame, or wrapped round since the last
/// ask (KH1 plays a motion with nothing queued after it round again), nothing queued after it. t1000: until now always
/// false, so the script's waits for a move's end (`limi` pc 2059: Infinity's start before its first
/// spin, Strike Raid's start before its throw, neither queueing a move after it) never ended: the
/// owner's "starting loop".
pub unsafe fn motion_ended() -> bool {
    static LAST: std::sync::Mutex<Option<(u16, f32)>> = std::sync::Mutex::new(None);
    let actor = super::takeover::sora_actor();
    if actor < 0x10000 {
        return false;
    }
    let Some((m, t)) = super::kh2slots::now_playing(actor) else { return false };
    let Some(clip) = super::pose::kh2_moves().and_then(|c| c.moveset.clips.get(&m)) else { return false };
    let step = super::formmagic::frame_scale().max(1.0);
    let wrapped = LAST.lock().ok().and_then(|mut l| l.replace((m, t))).is_some_and(|(lm, lt)| lm == m && t + step < lt);
    // (Only with nothing queued after it: KH1's queue moves on to the next at the end, as KH2's does
    // before its script looks. t1002: KH1's count `+0x178` holds only what is still to play (the
    // request `0x14029F980` sets 1, the queue `0x14029FA30` adds one, and the motion step's apply
    // `0x14029FC40` takes the head off and counts one down; Checked (decomp)), so anything queued is a
    // count of 1 or more, as KH1's own end rule `0x14029F6E0` reads it (`0 < count`). It was `>= 2`,
    // so a move with one queued after it (Strike Raid's throw 232 with the catch 234) read as ended a
    // frame before KH1 moved on: `limi` skipped the catch, its main loop then saw 234 and ended the
    // limit before Raid was ever offered.)
    let queued = rd::<u32>(actor + 0x178) >= 1;
    let ended = !queued && (wrapped || t + step >= clip.length as f32);
    if ended {
        static SAID: std::sync::Mutex<Option<u16>> = std::sync::Mutex::new(None);
        if SAID.lock().ok().map(|mut s| s.replace(m)) != Some(Some(m)) {
            log!(Forms, "Limit: KH2 motion {m} ended for its script (KH1 time {t:.1} of {})", clip.length);
        }
    }
    ended
}

/// A limit script's frame-trigger check (`trap_obj_motion_check_trigger (obj, n)`): KH2's frame
/// trigger 8 + n of the clip KH1 plays in our slot on Sora fired within KH1's last frame of time in
/// it (Steam KH2: frame triggers 9..12 set the object's `+0x2AD` bits 0..3 for the frame,
/// `0x14040DD70`, Checked (decomp)).
pub unsafe fn motion_trigger(n: u8) -> bool {
    let actor = super::takeover::sora_actor();
    if actor < 0x10000 || n == 0 {
        return false;
    }
    let Some((m, t)) = super::kh2slots::now_playing(actor) else { return false };
    let Some(clip) = super::pose::kh2_moves().and_then(|c| c.moveset.clips.get(&m)) else { return false };
    let step = super::formmagic::frame_scale().max(1.0);
    let id = 8 + n;
    clip.events.iter().any(|e| matches!(e.kind, character_core::EventKind::Other(x) if x == id) && (e.frame as f32) <= t && (e.frame as f32) > t - step)
}

/// The frame triggers 7 (`EventKind::ObjectTrigger`) of the clip KH1 plays in our slot on Sora
/// passed within KH1's last frame of time in it (as `motion_trigger`): KH2's object-script
/// triggers 0x11 to send the limit's script this frame (Infinity's shots, t987).
pub unsafe fn object_triggers(actor: usize) -> Vec<(i16, i16)> {
    if actor < 0x10000 || !RUNNING.load(Ordering::Relaxed) {
        return Vec::new();
    }
    let Some((m, t)) = super::kh2slots::now_playing(actor) else { return Vec::new() };
    let Some(clip) = super::pose::kh2_moves().and_then(|c| c.moveset.clips.get(&m)) else { return Vec::new() };
    let step = super::formmagic::frame_scale().max(1.0);
    clip.events
        .iter()
        .filter(|e| (e.frame as f32) <= t && (e.frame as f32) > t - step)
        .filter_map(|e| match e.kind {
            character_core::EventKind::ObjectTrigger(p0, p1) => Some((p0, p1)),
            _ => None,
        })
        .collect()
}

/// A follow-up chosen on Triangle (`combat::command`): to the limit's script (KH2's trigger 0x17,
/// `formmagic::limit_command`), else KH1's "can't".
pub unsafe fn follow_up(base: usize, j: usize) {
    let Some(f) = FOLLOW_UPS.get(j) else { return };
    if RUNNING.load(Ordering::Relaxed) && super::formmagic::limit_command(f.3) {
        log!(Forms, "Limit: {} ({}) to the limit's script", f.2, f.3);
        // (KH2's Limit EXP for a finish, t983.)
        super::formexp::follow_up(f.3);
        return;
    }
    log!(Forms, "Limit: {} chosen with no limit running: KH1's \"can't\"", f.2);
    let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + super::addr::Rva(0x177F60));
    sound(4);
}

/// KH1's HP change (`0x1402A2790 (actor, amount)`, clamped to the maximum, its HUD mirror kept;
/// `form`'s).
const HP_ADD_RVA: super::addr::Rva = super::addr::Rva(0x2A2790);

/// KH2's HP drain on a hit of Sora's (t982): after a hit lands, an ATKP's `+0x2F` (its HP drain)
/// that isn't 0 is handed to the attack's owner's method `+0xE8` (Steam KH2 `0x1403D1730`, Checked
/// (decomp)); only Limit Form's limits' rows have one (Sonic Rave's 1878 / 1879: 5, Checked (files)).
/// Here: that many HP to Sora through KH1's own HP change, for the KH2 hit KH1's contact just made
/// (`kh1rows::kh2_atkp`). Assumption: method `+0xE8` adds that many HP (main's reading); KH2's and
/// KH1's HP are the same points.
pub unsafe fn hp_drain(attack: usize) {
    let Some(a) = super::kh1rows::kh2_atkp(attack) else { return };
    if a.hp_drain == 0 {
        return;
    }
    let (base, sora) = (super::takeover::game_base(), super::takeover::sora_actor());
    if base == 0 || sora < 0x10000 {
        return;
    }
    let stats = super::pose::ptr(rd::<u32>(sora + 0x6C));
    if stats < 0x10000 || rd::<i32>(stats + 0x3C) <= 0 {
        return;
    }
    let add: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + HP_ADD_RVA);
    let was = rd::<i32>(stats + 0x3C);
    add(sora, a.hp_drain as i32);
    log!(Forms, "Limit: HP drain {} (ATKP {}): HP {was} -> {}", a.hp_drain, a.id, rd::<i32>(stats + 0x3C));
}

/// A limit script's wanted facing for Sora (`trap_obj_wish_dir`, the game's vector `(x, z)`): KH1's
/// wanted facing, as KH1's row runner sets it (Assumption: KH1 turns him to it as for a swing).
///
/// t1005: during one of our limits he turns there at once (the owner, t1001: "sonic rave has really
/// strange movement when not locked onto an enemy"; answer 2 (a), the instant turn). Only the wanted
/// facing was written, and KH1's think turns him toward it at about pi/16 a frame (`0x1402A3300` →
/// `0x1402A8300`, Checked (decomp)), so the Rave's turn round (`limi`: the wish the reverse of his
/// facing, then 212 again) took about 16 frames while K401's travel already moved him, and KH1's
/// motion pass lays the travel so far along his facing as it is then (`kh2slots::turn_to_lock`'s
/// t-turn notes): an arc, then a dash at an odd angle. With a target, K401's range 28 turned him in
/// about 3 frames, which is why only the no-lock case showed it. Now the turn is `kh2slots::turn_to`,
/// the same turn `turn_to_lock` makes (facing, travel so far and this frame's step turned with him).
/// Assumption: KH2's own turn to the wish is at once (not found in KH2's decomp; main's controller
/// set it at once).
pub unsafe fn face(x: f32, z: f32) {
    let actor = super::takeover::sora_actor();
    if actor < 0x10000 || !(x.is_finite() && z.is_finite()) || x * x + z * z < 1e-6 {
        return;
    }
    let f = x.atan2(z);
    let yaw: f32 = rd(actor + 0x3C);
    if RUNNING.load(Ordering::Relaxed) && yaw.is_finite() {
        super::kh2slots::turn_to(actor, yaw, f);
        return;
    }
    wr(actor + WANT_FACING, f);
}

/// A limit script's light colour (`trap_light_set_color (light, r, g, b, frames)`, the light
/// `trap_limit_light` gave): KH1's limit light faded to it over `frames` (KH2's colour / 128, as
/// KH2 scales its scene colour, `0x1401626F0`, Checked (decomp); Assumption: KH2's frames are
/// KH1's ticks, as `drivedim`).
pub unsafe fn light(_light: u32, rgb: [i32; 3], frames: f32) {
    let base = super::takeover::game_base();
    if base == 0 || !RUNNING.load(Ordering::Relaxed) {
        return;
    }
    let l = rd::<usize>(base + LIGHT_RVA);
    if l < 0x10000 || !super::hook::prologue_is(base + FADE_RVA, &FADE_BYTES) {
        return;
    }
    let c = Colour([rgb[0].clamp(0, 255) as f32 / 128.0, rgb[1].clamp(0, 255) as f32 / 128.0, rgb[2].clamp(0, 255) as f32 / 128.0, 1.0]);
    let fade: unsafe extern "system" fn(usize, usize, f32, usize) = std::mem::transmute(base + FADE_RVA);
    fade(l, &c as *const Colour as usize, if frames.is_finite() { frames.max(0.0) } else { 0.0 }, 0);
    log!(Forms, "Limit: its light to {rgb:?} / 128 over {frames} ticks");
}

/// Limit `k` (Sonic Rave) started on KH1's limit path, if it can start now; the units paid.
unsafe fn start(base: usize, k: usize) -> Result<i32, String> {
    let actor = super::takeover::sora_actor();
    if base == 0 || actor < 0x10000 {
        return Err("no Sora".into());
    }
    if !super::form::in_limit() {
        return Err("not in Limit Form".into());
    }
    if ORIG_ALIVE.load(Ordering::Relaxed) == 0 {
        return Err("KH1's element check isn't hooked".into());
    }
    if !super::hook::prologue_is(base + END_TASK_RVA, &END_TASK_BYTES) || !super::hook::prologue_is(base + NEW_LIGHT_RVA, &NEW_LIGHT_BYTES) || !super::hook::prologue_is(base + FADE_RVA, &FADE_BYTES) {
        return Err("KH1's limit calls aren't as expected".into());
    }
    // (KH2: one limit at a time, `0x142A24CC0`; KH1's own, or ours, running.)
    let flags = rd::<u32>(base + FLAGS_RVA);
    if flags & IN_LIMIT != 0 || rd::<u32>(base + RUNNING_RVA) != 0 || RUNNING.load(Ordering::Relaxed) {
        return Err("a limit runs".into());
    }
    // (Assumption: on the ground only. KH2's grey rule `0x1403D88E0` has no air test; its moves
    // K400..K402 are ground motions, `P_EX100_KH1F.mset`, Checked (files).)
    if matches!(rd::<i32>(actor + STATE), 1 | 2 | 8) {
        return Err("in the air".into());
    }
    let first = FIRST_MOTION.get(k).copied().flatten().ok_or("not built yet")?;
    let Some(n) = super::kh1rows::by_motion(first) else {
        return Err("no row of ours".into());
    };
    if !super::kh2slots::has(super::kh1rows::request(n) as usize, first) {
        return Err(format!("{}'s slot isn't built", LIMITS[k].2));
    }
    if !super::formmagic::limit_ready() {
        return Err("Limit Form's script isn't loaded or KH1's script machine isn't hooked".into());
    }
    let Some(paid) = super::mpcost::pay_limit(actor, LIMITS[k].4) else {
        return Err("no MP".into());
    };
    // KH1's start: in a limit, not done.
    wr(base + FLAGS_RVA, (flags | IN_LIMIT) & !LIMIT_DONE);
    wr(base + DATA_RVA, mark());
    RUNNING.store(true, Ordering::Relaxed);
    // (t997: our reaction provider in KH1's command fill, as KH1's limit states set theirs; t999:
    // set by our limit state's enter, KH1's way, else here.)
    if !enter_state(base, actor, k) {
        set_provider(base, true);
    }
    // KH1's limit light (its running flag and light, as `0x140397990`); its colour the script's
    // (`light`).
    wr(base + RUNNING_RVA, 1u32);
    let new_light: unsafe extern "system" fn(i32, i32, i32) -> usize = std::mem::transmute(base + NEW_LIGHT_RVA);
    let l = new_light(1, 9, 0);
    wr(base + LIGHT_RVA, l);
    // The script (KH2's trigger 0x16 with the limit's command), at our next step.
    if !super::formmagic::limit_start(LIMITS[k].3) {
        log!(Forms, "Limit: its script didn't take the limit; KH1's end task ends it");
    }
    // KH1's end task, as its modules add it.
    let end_task: unsafe extern "system" fn() = std::mem::transmute(base + END_TASK_RVA);
    end_task();
    let _ = n;
    Ok(paid)
}
