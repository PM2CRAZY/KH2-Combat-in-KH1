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
/// The top row's name in Limit Form (KH2's 0x2C3).
pub const ROW_NAME: &str = "Limit";

/// Limit `k` chosen from the list (`combat::command`, KH1's command step): Sonic Rave starts on
/// KH1's own limit path (`start`); the other three come in their own versions, KH1's "can't" until
/// then, as for a limit that can't start now.
pub unsafe fn chosen(base: usize, k: usize) {
    let name = LIMITS.get(k).map_or("?", |l| l.2);
    let r = if k == SONIC_RAVE { start(base, k) } else { Err("not built yet".to_string()) };
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
// Ours runs on that path: KH1's bit 16, KH1's running flag and light (KH2's colour and time), KH1's
// end task added as KH1's modules add it, our mark in KH1's limit data, and KH1's element check
// answering for it (`alive`): the limit lives while its moves play. KH1 ends it, fades the light
// back and clears its flags itself. No code of ours runs each frame for it but that check, which
// KH1's own end task makes.

/// Sonic Rave's index in `LIMITS`.
pub const SONIC_RAVE: usize = 0;
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

/// KH2's limit light: `trap_light_set_color(64, 64, 80, 16)` in Sonic Rave's dispatch of `limi`
/// (`P_EX100_KH1F.mdlx`, Checked (files)), 128 = KH1's 1.0 (as KH2 scales its scene colour,
/// `0x1401626F0`, Checked (decomp)), over 16 frames (Assumption: KH2's trap counts 60 Hz frames,
/// KH1's fade the same ticks, as `drivedim`).
#[repr(C, align(16))]
struct Colour([f32; 4]);
static LIGHT: Colour = Colour([64.0 / 128.0, 64.0 / 128.0, 80.0 / 128.0, 1.0]);
const LIGHT_TICKS: f32 = 16.0;

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
}

/// A limit of ours runs on KH1's path now.
pub fn running() -> bool {
    RUNNING.load(Ordering::Relaxed)
}

/// KH1's element check: for our mark, whether our limit still runs (non-null) or is over (null,
/// and KH1's end task ends it); for anything else KH1's own.
unsafe extern "system" fn alive(data: usize) -> usize {
    if data == mark() && data != 0 {
        if still() {
            return mark();
        }
        if RUNNING.swap(false, Ordering::Relaxed) {
            log!(Forms, "Limit: Sonic Rave's moves are over: KH1's end task ends the limit (its light back over 30 ticks)");
        }
        return 0;
    }
    let f: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(ORIG_ALIVE.load(Ordering::Relaxed));
    f(data)
}

/// Sonic Rave's moves still play on Sora (one of its slots, or one of its requests asked and not
/// yet applied), as KH2's `limi` runs while his motion is its hook's (Checked (files)).
unsafe fn still() -> bool {
    let actor = super::takeover::sora_actor();
    if actor < 0x10000 {
        return false;
    }
    let moves = super::kh1rows::SONIC_RAVE_MOTIONS;
    if super::kh2slots::playing_motion(actor).is_some_and(|m| moves.contains(&m)) {
        return true;
    }
    let asked = rd::<u32>(actor + 0x164) & 0xFFFF;
    moves.iter().filter_map(|&m| super::kh1rows::by_motion(m)).any(|n| super::kh1rows::request(n) & 0xFFFF == asked)
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
    let Some(n) = super::kh1rows::by_motion(super::kh1rows::SONIC_RAVE_MOTIONS[0]) else {
        return Err("no row of ours".into());
    };
    if !super::kh2slots::has(super::kh1rows::request(n) as usize, super::kh1rows::SONIC_RAVE_MOTIONS[0]) {
        return Err("Sonic Rave's slot isn't built".into());
    }
    let Some(paid) = super::mpcost::pay_limit(actor, LIMITS[k].4) else {
        return Err("no MP".into());
    };
    // KH1's start: in a limit, not done.
    wr(base + FLAGS_RVA, (flags | IN_LIMIT) & !LIMIT_DONE);
    wr(base + DATA_RVA, mark());
    RUNNING.store(true, Ordering::Relaxed);
    // KH1's limit light (its running flag and light, as `0x140397990`), KH2's colour and time.
    wr(base + RUNNING_RVA, 1u32);
    let new_light: unsafe extern "system" fn(i32, i32, i32) -> usize = std::mem::transmute(base + NEW_LIGHT_RVA);
    let fade: unsafe extern "system" fn(usize, usize, f32, usize) = std::mem::transmute(base + FADE_RVA);
    let light = new_light(1, 9, 0);
    wr(base + LIGHT_RVA, light);
    if light >= 0x10000 {
        fade(light, &LIGHT as *const Colour as usize, LIGHT_TICKS, 0);
    }
    // KH1's end task, as its modules add it.
    let end_task: unsafe extern "system" fn() = std::mem::transmute(base + END_TASK_RVA);
    end_task();
    // Its first move on our row (KH1's row start: its request, its attack object).
    if !super::kh1rows::start_row(actor, n) {
        log!(Forms, "Limit: Sonic Rave's row didn't start; KH1's end task ends the limit");
        RUNNING.store(false, Ordering::Relaxed);
    }
    Ok(paid)
}
