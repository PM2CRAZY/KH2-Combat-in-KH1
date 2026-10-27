//! KH1's sounds in the log (the user: "we can do both ... if I say I want sora to say this line
//! e.g 0x23 you can just cross examine what I said with the log"). `kh1-kh2.ini`'s `sound_log`
//! (on by default on the testing branch; `off` installs nothing) hooks the three functions that
//! play KH1's sounds and writes a line for every call:
//!
//! - **menu sounds** (`0x140177F60 (id)`: the cursor, confirm, back and "can't" sounds),
//! - **voice groups** (`0x14028E060 (actor, group, other)`: KH1 picks one of the group's clips),
//! - **voice clips** (`0x14028E7A0 (actor, clip, priority)`: one clip of the speaker's bank; a
//!   group's pick shows as a clip line right after its group line, called from inside the group).
//!
//! And, on the Steam build only (their addresses are the Steam exe's, read from it), the sound
//! system under them, which says which bank file a sound came from:
//!
//! - **voice calls** (`IOP Voice Call : bank=%d no=%d`, `0x1400DC210 (bank, no)`): the voice that
//!   plays. `bank` is the bank's number (the SCD's own, `0x36`; Sora's battle voice is one bank a
//!   world, 90..103, `docs/kh1-sounds.md`) and `no` the clip number in it (the SCD's sound id).
//! - **sound effects** (`SslSeCreate` `0x1400DE460` and `SslSePlay` `0x1400DECF0`, `(bank, no, id)`,
//!   KH1's own debug strings name them): the effect's bank, with the bank's name from the game's
//!   bank list (`0x1404D65D8`) beside it. The menu sounds are `SslSeCreate (2, id)`.
//!
//! Each line says where the call came from: an address in the game's own code (`game +0x...`, an
//! RVA in the running build, which is the Steam one on Steam: `docs/kh1-sounds.md`), or the mod
//! (`kh1voice`, `trace`, `config` and `combat` play sounds too). `docs/kh1-sounds.md` is the
//! table the log fills in.
//!
//! (t354) The hooks go in whatever `sound_log` says, and each writes its line only while the sound
//! log is on (`trace::sound_log`: the ini's, or the debug menu's since), so it switches in game.
//! (The ram-use thread, m1 answer 3) Only with the debug tools loaded (the ini's `debug_menu = on`
//! at start, `debug_on`): otherwise only the voice group hook goes in, for the gameplay it carries.
//! Its lines are the trace's Sounds category (`log_line`). (The ram-use thread) The switch is now
//! the debug menu's Logs page, KH1 Sound Calls (`LogCat::Kh1Sounds`, ini `log_kh1_sounds`, off by
//! default); the old `sound_log` key and the Trace Recorder's Sound Log row are gone.

use super::addr::{Build, Rva};
use super::hook;
use super::log;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

const MENU_SOUND_RVA: Rva = Rva(0x177F60);
/// `push rbx; sub rsp, 0x30`.
const MENU_SOUND_PROLOGUE: [u8; 6] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x30];
const VOICE_GROUP_RVA: Rva = Rva(0x28E060);
/// `mov [rsp+0x20], rbx`.
const VOICE_GROUP_PROLOGUE: [u8; 5] = [0x48, 0x89, 0x5C, 0x24, 0x20];
const VOICE_CLIP_RVA: Rva = Rva(0x28E7A0);
/// `mov [rsp+8], rbx`.
const VOICE_CLIP_PROLOGUE: [u8; 5] = [0x48, 0x89, 0x5C, 0x24, 0x08];

/// The Steam exe's (RVAs; the Epic build's differ and aren't known here): the voice call, the
/// sound effect create / play, and the sound bank list's head.
const STEAM_VOICE_CALL: usize = 0xDC210;
/// `push rbp; push rsi; push rdi; push r14`.
const VOICE_CALL_PROLOGUE: [u8; 6] = [0x40, 0x55, 0x56, 0x57, 0x41, 0x56];
const STEAM_SE_CREATE: usize = 0xDE460;
/// `mov rax, rsp; push r12; push r14`.
const SE_CREATE_PROLOGUE: [u8; 7] = [0x48, 0x8B, 0xC4, 0x41, 0x54, 0x41, 0x56];
const STEAM_SE_PLAY: usize = 0xDECF0;
/// `push rdi; sub rsp, 0x70`.
const SE_PLAY_PROLOGUE: [u8; 6] = [0x40, 0x57, 0x48, 0x83, 0xEC, 0x70];
const STEAM_BANK_LIST: usize = 0x4D65D8;

/// Sora's battle voice banks by bank number (the SCDs' `0x36`), the file each plays from
/// (`voice/<file>.vsb/<file>.win32.scd` in the `kh1_*` archives).
const SORA_BANKS: [(i32, &str); 13] = [
    (90, "di_sora"),
    (92, "po_sora"),
    (93, "tw_sora"),
    (94, "aw_sora"),
    (95, "tz_sora"),
    (96, "al_sora"),
    (97, "lm_sora"),
    (98, "nm_sora"),
    (99, "he_sora"),
    (100, "pi_sora"),
    (101, "pp_sora"),
    (102, "pc_sora"),
    (103, "ew_sora"),
];

/// At most this many lines a session (a sound that never stops can't fill the log).
const MAX_LINES: u32 = 20000;

static BASE: AtomicUsize = AtomicUsize::new(0);
/// The game image's size (a caller outside it is the mod's).
static IMAGE: AtomicUsize = AtomicUsize::new(0);
static ORIG_SOUND: AtomicUsize = AtomicUsize::new(0);
static ORIG_GROUP: AtomicUsize = AtomicUsize::new(0);
static ORIG_CLIP: AtomicUsize = AtomicUsize::new(0);
static LINES: AtomicU32 = AtomicU32::new(0);
static ORIG_VOICE_CALL: AtomicUsize = AtomicUsize::new(0);
static ORIG_SE_CREATE: AtomicUsize = AtomicUsize::new(0);
static ORIG_SE_PLAY: AtomicUsize = AtomicUsize::new(0);
/// The last sound effect line's key and frame (the same call twice in one frame is one line).
static LAST_SE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static LAST_SE_FRAME: AtomicU32 = AtomicU32::new(u32::MAX);
/// The bank list's first nodes have been written out (to read its layout from the log).
static LIST_SHOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Where the call to the hooked function came from (called straight from a hook: two frames up).
#[inline(never)]
unsafe fn caller() -> String {
    let mut frame: [*mut c_void; 1] = [std::ptr::null_mut()];
    let n = super::sys::RtlCaptureStackBackTrace(2, 1, frame.as_mut_ptr(), std::ptr::null_mut());
    if n == 0 {
        return "caller unknown".to_string();
    }
    let (at, base) = (frame[0] as usize, BASE.load(Ordering::Relaxed));
    if at >= base && at < base + IMAGE.load(Ordering::Relaxed) {
        format!("from game +{:#x}", at - base)
    } else {
        "from the mod".to_string()
    }
}

/// The game code that called the hooked function, as an RVA of the running exe (0 if it isn't
/// the game's).
#[inline(never)]
unsafe fn caller_rva() -> usize {
    let mut frame: [*mut c_void; 1] = [std::ptr::null_mut()];
    let n = super::sys::RtlCaptureStackBackTrace(2, 1, frame.as_mut_ptr(), std::ptr::null_mut());
    let (at, base) = (frame[0] as usize, BASE.load(Ordering::Relaxed));
    if n == 0 || at < base || at >= base + IMAGE.load(Ordering::Relaxed) {
        0
    } else {
        at - base
    }
}

fn who(actor: usize) -> String {
    if actor != 0 && actor == super::takeover::sora_actor() {
        "Sora".to_string()
    } else {
        format!("actor {actor:#x}")
    }
}

fn emit(line: String) {
    let n = LINES.fetch_add(1, Ordering::Relaxed);
    if n < MAX_LINES {
        super::log!(Kh1Sounds, "{}", line);
    } else if n == MAX_LINES {
        log!(Kh1Sounds, "KH1 sound log: {MAX_LINES} lines this session; no more until the game restarts");
    }
}

/// The sound log is on now: the Logs page's KH1 Sound Calls (never without the debug tools).
fn live() -> bool {
    super::log_on(super::LogCat::Kh1Sounds)
}

unsafe extern "system" fn menu_sound(id: i32) -> usize {
    if live() {
        let from = caller();
        emit(format!("KH1 sound: menu sound {id} ({from})"));
    }
    let f: unsafe extern "system" fn(i32) -> usize = std::mem::transmute(ORIG_SOUND.load(Ordering::Acquire));
    f(id)
}

unsafe extern "system" fn voice_group(actor: usize, group: i32, other: usize) -> usize {
    // (Sora's hurt voice in his hit pass, Battle Quotes KH2: KH2's line instead, `hitreact`.)
    if super::hitreact::hurt_line(actor, group) {
        if live() {
            emit(format!("KH1 sound: voice group {group} on Sora: KH2's hurt line instead"));
        }
        return 0;
    }
    // (KH1's swing voice under KH2's quotes, `kh2sound::holds_back`; t746: KH1's spell line under
    // them, `kh2sound::spell_line`.)
    let from_rva = caller_rva();
    let held = super::kh2sound::spell_line(actor, group, from_rva) || super::kh2sound::holds_back(actor, group, from_rva);
    if live() {
        let from = caller();
        let with = if other == 0 { String::new() } else { format!(", other {}", who(other)) };
        emit(format!("KH1 sound: voice group {group} on {}{with} ({from}){}", who(actor), if held { ", left out for KH2's quote" } else { "" }));
    }
    if held {
        return 0;
    }
    let f: unsafe extern "system" fn(usize, i32, usize) -> usize = std::mem::transmute(ORIG_GROUP.load(Ordering::Acquire));
    f(actor, group, other)
}

unsafe extern "system" fn voice_clip(actor: usize, clip: i32, priority: i32) -> usize {
    if live() {
        let from = caller();
        emit(format!("KH1 sound: voice clip {clip} ({clip:#x}) priority {priority} on {} ({from})", who(actor)));
    }
    let f: unsafe extern "system" fn(usize, i32, i32) -> usize = std::mem::transmute(ORIG_CLIP.load(Ordering::Acquire));
    f(actor, clip, priority)
}

unsafe fn rd<T: Copy>(at: usize) -> T {
    std::ptr::read_unaligned(at as *const T)
}

/// The text at `at` (printable ASCII, at most 48 bytes).
unsafe fn text(at: usize) -> String {
    let mut s = String::new();
    for i in 0..48 {
        if !super::crash::readable(at + i) {
            break;
        }
        let b = rd::<u8>(at + i);
        if !(0x20..0x7F).contains(&b) {
            break;
        }
        s.push(b as char);
    }
    s
}

/// The game's sound bank list's node for `bank` (the list at `0x1404D65D8`: a node's next is at
/// `+8`, its bank number at `+0x10`, its second number at `+0x14`, its name at `+0x30`, as
/// `SslSeCreate` reads them), described for the log.
unsafe fn bank_node(bank: i32, no: i32) -> String {
    let base = BASE.load(Ordering::Relaxed);
    let mut node = rd::<u64>(base + STEAM_BANK_LIST) as usize;
    let (mut hit, mut n) = (None, 0);
    while node >= 0x10000 && super::crash::readable(node) && super::crash::readable(node + 0x40) && n < 2048 {
        if rd::<i32>(node + 0x10) == bank && hit.map_or(true, |_| rd::<i32>(node + 0x14) == no) {
            hit = Some(node);
            if rd::<i32>(node + 0x14) == no {
                break;
            }
        }
        node = rd::<u64>(node + 8) as usize;
        n += 1;
    }
    match hit {
        Some(h) => format!("bank list: [{}] (+0x10 {}, +0x14 {})", text(h + 0x30), rd::<i32>(h + 0x10), rd::<i32>(h + 0x14)),
        None => "bank not in the bank list".to_string(),
    }
}

/// The first nodes of the bank list, once, so the log shows the list's layout.
unsafe fn show_list() {
    if LIST_SHOWN.swap(true, Ordering::Relaxed) {
        return;
    }
    let base = BASE.load(Ordering::Relaxed);
    let mut node = rd::<u64>(base + STEAM_BANK_LIST) as usize;
    let mut n = 0;
    while node >= 0x10000 && super::crash::readable(node) && super::crash::readable(node + 0x40) && n < 12 {
        log!(Kh1Sounds,
            "KH1 sound: bank list node {n}: +0x10 {} +0x14 {} +0x18 {} +0x30 [{}] +0x3C {}",
            rd::<i32>(node + 0x10),
            rd::<i32>(node + 0x14),
            rd::<i32>(node + 0x18),
            text(node + 0x30),
            rd::<i32>(node + 0x3C)
        );
        node = rd::<u64>(node + 8) as usize;
        n += 1;
    }
}

/// A sound effect call is a line unless the same one was a line already this frame.
fn se_new(kind: u64, bank: i32, no: i32, id: i32) -> bool {
    let key = kind << 60 | (bank as u64 & 0xFFFF) << 40 | (no as u64 & 0xFFFF) << 20 | (id as u64 & 0xFFFFF);
    let frame = super::takeover::GAME_FRAME.load(Ordering::Relaxed) as u32;
    if LAST_SE.load(Ordering::Relaxed) == key && LAST_SE_FRAME.load(Ordering::Relaxed) == frame {
        return false;
    }
    LAST_SE.store(key, Ordering::Relaxed);
    LAST_SE_FRAME.store(frame, Ordering::Relaxed);
    true
}

unsafe extern "system" fn voice_call(bank: i32, no: i32) -> usize {
    if live() {
        let from = caller();
        let file = SORA_BANKS.iter().find(|b| b.0 == bank).map_or(String::new(), |b| format!(" = {}.win32.scd clip {no}", b.1));
        emit(format!("KH1 sound: voice call bank {bank} no {no}{file} ({from})"));
    }
    let f: unsafe extern "system" fn(i32, i32) -> usize = std::mem::transmute(ORIG_VOICE_CALL.load(Ordering::Acquire));
    f(bank, no)
}

unsafe extern "system" fn se_create(bank: i32, no: i32, id: i32) -> usize {
    if live() && se_new(1, bank, no, id) {
        let from = caller();
        show_list();
        emit(format!("KH1 sound: effect create bank {bank} no {no} id {id}, {} ({from})", bank_node(bank, no)));
    }
    let f: unsafe extern "system" fn(i32, i32, i32) -> usize = std::mem::transmute(ORIG_SE_CREATE.load(Ordering::Acquire));
    f(bank, no, id)
}

unsafe extern "system" fn se_play(bank: i32, no: i32, id: i32) -> usize {
    if live() && se_new(2, bank, no, id) {
        let from = caller();
        emit(format!("KH1 sound: effect play bank {bank} no {no} id {id}, {} ({from})", bank_node(bank, no)));
    }
    let f: unsafe extern "system" fn(i32, i32, i32) -> usize = std::mem::transmute(ORIG_SE_PLAY.load(Ordering::Acquire));
    f(bank, no, id)
}

pub unsafe fn install() {
    super::trace::refresh();
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    let rd32 = |at: usize| std::ptr::read_unaligned(at as *const u32);
    let pe = base + rd32(base + 0x3C) as usize;
    IMAGE.store(rd32(pe + 0x50) as usize, Ordering::Relaxed);
    // (Always: KH2's hurt line and KH2's quotes hold KH1's voice back here, `hitreact::hurt_line`,
    // `kh2sound::holds_back`.)
    hook::install(base + VOICE_GROUP_RVA, &VOICE_GROUP_PROLOGUE, voice_group as *const () as usize, &ORIG_GROUP, "voice group (KH2 quotes; sound log)");
    // (The rest only log: in only with the debug tools loaded, the ini's `debug_menu = on` at start.)
    if !super::debug_on() {
        return;
    }
    hook::install(base + MENU_SOUND_RVA, &MENU_SOUND_PROLOGUE, menu_sound as *const () as usize, &ORIG_SOUND, "menu sound log");
    hook::install(base + VOICE_CLIP_RVA, &VOICE_CLIP_PROLOGUE, voice_clip as *const () as usize, &ORIG_CLIP, "voice clip log");
    if Build::current() == Build::Steam {
        hook::install(base + STEAM_VOICE_CALL, &VOICE_CALL_PROLOGUE, voice_call as *const () as usize, &ORIG_VOICE_CALL, "voice call log");
        hook::install(base + STEAM_SE_CREATE, &SE_CREATE_PROLOGUE, se_create as *const () as usize, &ORIG_SE_CREATE, "sound effect create log");
        hook::install(base + STEAM_SE_PLAY, &SE_PLAY_PROLOGUE, se_play as *const () as usize, &ORIG_SE_PLAY, "sound effect play log");
    } else {
        log!(Kh1Sounds, "KH1 sound log: the bank-level hooks (voice call, sound effects) are for the Steam build only");
    }
}
