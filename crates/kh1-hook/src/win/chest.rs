//! Chest Opening (KH1 | KH2), the Config menu's QoL row (KH2 by default; it was Combat > Tweaks in t724) (the tester Mr Bugs, thread
//! kh2-chest: "quickly open and move from the chest opening animation like in kh2 ... open the chest,
//! move then the pop up for the item plays at the top of the game letting you know what you got").
//! `docs/kh1-chest.md`.
//!
//! What KH1 does (Steam, Checked (decomp + exe)):
//!
//! - The Open command (Triangle box, command type `0x37`, offered by the chest's touch callback
//!   `0x1402B5A30`) runs `0x1402B58E0 (sora, chest)`. It reads the chest's entry in KH1's chest table
//!   (`0x140528D60`, a u16 per chest id: bits 1..3 the kind, bits 4.. the item or amount) and gives
//!   the contents itself: kind 0 an item through KH1's stock add (`0x140284E60 (item, 0xC)`, 0xFF
//!   when the stock is full), kind 3 munny (amount x 10, `0x1402E7E10`), kind 7 a reward record (an
//!   item one becomes kind 0), kind 1 `0x14020D3F0`; then marks the chest opened (`0x140285800`).
//!   It stores the open's arguments at `0x142DB7B28` (chest handle, chest id, kind `+8`, item or
//!   amount `+0xC`, "stock full" `+0x10`), sets Sora's `+0x444` bit 1 (his control step is skipped:
//!   locked), asks for his open motion (`0x14029F980 (sora, 0x900A6, 0)`) and queues event 5 on the
//!   chest (`0x14028DD80`).
//! - Event 5 (`0x1401A1A10`) starts KH1's treasure script (`exchange/<lang>_treasure.ev`, loaded at
//!   `0x1423BBCB0` by `0x1401A42C0`) in its own script contexts (`0x1423BF0B0`, `0x360` each, the
//!   count at `0x1423AB2DC`; the script in use at `0x1423BBBE0`): entry 0 for every kind but 5 (an
//!   event chest, whose entry is its item word). The script opens the lid, shows the "obtained"
//!   message window and releases Sora (Assumption: the script itself was not readable in this
//!   session; the release is KH1's actor action kind 0xF, `0x14028E6E0`, which only clears that bit).
//!
//! KH2 (this row on KH2), for a plain chest only (kind 0 or 3 with the item given, the stock not full):
//!
//! - KH1's own handler runs whole: the item or munny is given, the chest marked, the treasure script
//!   started, exactly as KH1 does. Afterwards Sora's lock bit is cleared, so KH1's own control step
//!   runs him at once (the open motion KH1 asked for plays until he moves).
//! - The treasure script runs whole too, but its message windows are not opened: while the open is
//!   recent (`FAST_MS`), KH1's message functions 0 and 177 (open a window) and 1 and 605 (add a line /
//!   an item line) take their own skip-mode branch when called from a treasure context (pop their
//!   arguments, open nothing), as KH1 does for them in skip mode. The script's waits on windows (33,
//!   107) then pass, as in skip mode, and closing (2) a window that never opened does nothing
//!   (Checked (decomp)). The four entries of KH1's event function table (`0x1404FA870`) point at
//!   wrappers of ours.
//! - The treasure script holds Sora with actions on him (KH1's action kinds 0x24 / 0x26 / 0x2B and
//!   the lock 0x10, `0x14028E6E0`; Checked (trace): t725's actor flags), so for the open's time our call
//!   handler sits in KH1's script machine's class table (class 0x18, `0x1404E5E70`; KH1's put back after):
//!   from a treasure context the actions queued on Sora go into KH1's done queue (`0x1401A4F20`), as KH1's
//!   own action step does for an action whose actor isn't there, and its wait on Sora's motion (event
//!   function 103) goes on at once. Logged in Cutscenes & Events.
//! - What was found is shown in KH1's own non-blocking notice box (the level-up box, `menu::queue_notice`,
//!   as the mod's "New Ability" boxes): "Obtained" and the item's name or the munny.
//!
//! Anything else (an event chest, a full stock, a reward record that is not an item, kind 1, any other
//! script using those contexts) is KH1's as before. KH1: nothing of this is installed until the row is
//! first set to KH2; then one flag check per Open and per message function call.

use super::addr::Build;
use super::{hook, log};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

/// KH2 (`chest_style = kh2`).
static KH2: AtomicBool = AtomicBool::new(false);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static ORIG_OPEN: AtomicUsize = AtomicUsize::new(0);
/// KH1's message functions as the table held them (called for anything not ours).
static ORIG_FN: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];
/// When the last fast open happened (ms since the first look, plus 1; 0 = none).
static FAST_AT: AtomicU64 = AtomicU64::new(0);
/// Message calls skipped for the current open (for the log).
static SKIPPED: AtomicU32 = AtomicU32::new(0);
/// Sora's actor at the last fast open (the treasure script's actions on him are KH1's "no actor" path).
static SORA: AtomicUsize = AtomicUsize::new(0);
/// KH1's call handler while ours is in its class table (0 = KH1's own is there).
static ORIG_CALL: AtomicUsize = AtomicUsize::new(0);
/// Actions on Sora completed at once / motion waits passed for the current open (for the log).
static RELEASED: AtomicU32 = AtomicU32::new(0);

/// KH1's Open command handler `(sora, chest)` (Steam; `mov [rsp+8], rbx; mov [rsp+0x10], rbp`,
/// Checked (exe)).
const OPEN_STEAM: usize = 0x2B58E0;
const OPEN_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];
/// The open's arguments as the handler leaves them (Steam, Checked (decomp)): kind, item or amount,
/// stock full.
const ARG_KIND_STEAM: usize = 0x2DB7B30;
const ARG_ITEM_STEAM: usize = 0x2DB7B34;
const ARG_FULL_STEAM: usize = 0x2DB7B38;
/// KH1's motion request `0x14029F980 (actor, request, f32 in XMM2)` (Steam, its first 10 bytes checked;
/// Checked (decomp + exe): it replaces the one pending request, `+0x178` count 1, `+0x17C` the request) and the
/// request the handler makes, the open motion (`0x900A6`, motion `0xA6`). The motion applied last is
/// `+0x164` (low 16 bits).
const REQUEST_STEAM: usize = 0x29F980;
const REQUEST_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];
const OPEN_REQUEST: u32 = 0x900A6;
const ACTOR_PENDING: usize = 0x178;
const ACTOR_REQUEST: usize = 0x17C;
const ACTOR_APPLIED: usize = 0x164;
/// Sora's lock: actor `+0x444` bit 1 (the handler sets it; KH1's control step is skipped while it is).
const ACTOR_LOCK: usize = 0x444;
const LOCK_BIT: u32 = 2;

/// KH1's event function table (8-byte pointers, by number) and the four message functions wrapped
/// (Steam RVAs, Checked (exe)), with how many arguments each pops in its skip-mode branch
/// (Checked (decomp)).
const TABLE_STEAM: usize = 0x4FA870;
const MESSAGE_FNS: [(usize, usize, i32); 4] = [(0, 0x1B93F0, 1), (1, 0x1B8DD0, 2), (177, 0x1B9170, 1), (605, 0x1B8AE0, 2)];
/// A script context's stack pointer (`+0x194`).
const CTX_SP: usize = 0x194;

/// The treasure script's buffer, the script in use, its contexts and their count (Steam, Checked
/// (decomp)).
const TREASURE_STEAM: usize = 0x23BBCB0;
const SCRIPT_IN_USE_STEAM: usize = 0x23BBBE0;
const CONTEXTS_STEAM: usize = 0x23BF0B0;
const COUNT_STEAM: usize = 0x23AB2DC;
const CTX_SIZE: usize = 0x360;
const CTX_MAX: u32 = 64;
/// A context's actor id (`+0`, what its actions name) and its actor (`+8`) (Checked (decomp): the action
/// functions push `*ctx` as the record's actor, function 103 tests `ctx + 8`'s motion).
const CTX_ACTOR: usize = 8;

/// KH1's script machine's class handlers (`0x1404E5DB0`, by class; Steam, Checked (exe): its three users
/// `0x1401A3517`, `0x1401A4658`, `0x1401A4917` load it by `lea`) and class `0x18`'s, the event function
/// call `0x1401B3530 (ctx, number)` (it jumps to the event table's entry).
const CALL_ENTRY_STEAM: usize = 0x4E5E70;
const CALL_STEAM: usize = 0x1B3530;
/// KH1's actor action queue (a ring of 256 records of `0x20`: kind | argument << 8, the actor's id, the
/// rest by kind; write index, read index; Steam, Checked (exe + decomp): pushed by `0x1401A5030`, run by
/// `0x14028E6E0`) and its done queue's push `0x1401A4F20 (record)` (the script runner `0x1401A2CF0`
/// clears a context's move and turn waits, `+0x33C` bits 0 and 1, from records there). An action whose
/// actor isn't found is put straight into the done queue by KH1's own action step, which is what is done
/// here with the treasure script's actions on Sora.
const ACTIONS_STEAM: usize = 0x23CFE10;
const ACTIONS_WRITE_STEAM: usize = 0x23CFE08;
const ACTIONS_READ_STEAM: usize = 0x23CFDFC;
const DONE_PUSH_STEAM: usize = 0x1A4F20;
const DONE_PUSH_PROLOGUE: [u8; 7] = [0x48, 0x63, 0x05, 0xE9, 0xEE, 0x22, 0x02];
const ACTION_SIZE: usize = 0x20;
/// Event function 103 (`0x1401A96F0`): waits (returns 4) until the context's actor's motion is over,
/// then goes on (2) (Checked (decomp)).
const FN_WAIT_MOTION: usize = 103;

/// How long after an open its message windows are still skipped. Assumption: the treasure script
/// reaches its window well within this (KH1's own open motion and lid take about a second).
const FAST_MS: u64 = 8000;

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

fn ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

fn base() -> usize {
    unsafe { super::sys::GetModuleHandleA(std::ptr::null()) as usize }
}

/// The row's value: KH2's chest opening.
pub fn kh2() -> bool {
    KH2.load(Ordering::Relaxed)
}

pub unsafe fn install() {
    let on = super::settings::get().chest_kh2;
    KH2.store(on, Ordering::Relaxed);
    if on {
        hook_in();
    }
    log!(Menus, "Chest Opening: {} (chest_style)", if on { "KH2" } else { "KH1" });
}

/// The Config menu's Chest Opening row, kept in the ini.
pub fn set(v: bool) {
    KH2.store(v, Ordering::Relaxed);
    if v {
        unsafe { hook_in() };
    }
    FAST_AT.store(0, Ordering::Relaxed);
    super::settings::save("chest_style", if v { "kh2" } else { "kh1" });
    log!(Menus, "Config menu: Chest Opening {}", if v { "KH2" } else { "KH1" });
}

/// The Open handler hook and the four table entries (Steam only, once).
unsafe fn hook_in() {
    if INSTALLED.swap(true, Ordering::Relaxed) {
        return;
    }
    if Build::current() != Build::Steam {
        log!(Menus, "Chest Opening: KH2 is Steam only (the addresses are the Steam build's): chests stay KH1's");
        return;
    }
    let base = base();
    // KH1's script call handler and done queue as the release of Sora needs them (checked before
    // anything is patched: without them the open stays KH1's).
    let entry = rd::<u64>(base + CALL_ENTRY_STEAM) as usize;
    let done = std::slice::from_raw_parts((base + DONE_PUSH_STEAM) as *const u8, DONE_PUSH_PROLOGUE.len());
    if entry != base + CALL_STEAM || done != DONE_PUSH_PROLOGUE {
        log!(Cutscenes, "Chest Opening: KH1's script call handler ({entry:#x}) or done queue isn't as expected: the Open stays KH1's");
        return;
    }
    // The table entries first: without them the open would free Sora with KH1's window up.
    let mut wrapped = 0;
    let ours: [usize; 4] = [msg0 as *const () as usize, msg1 as *const () as usize, msg177 as *const () as usize, msg605 as *const () as usize];
    for (i, &(n, rva, _)) in MESSAGE_FNS.iter().enumerate() {
        let at = base + TABLE_STEAM + n * 8;
        let was = (base + rva) as u64;
        ORIG_FN[i].store(base + rva, Ordering::Release);
        if hook::patch(at, &was.to_le_bytes(), &(ours[i] as u64).to_le_bytes(), "event function (chest)") {
            wrapped += 1;
        }
    }
    if wrapped != MESSAGE_FNS.len() {
        log!(Menus, "Chest Opening: only {wrapped} of KH1's message functions wrapped: the Open stays KH1's");
        return;
    }
    if hook::install(base + OPEN_STEAM, &OPEN_PROLOGUE, open as *const () as usize, &ORIG_OPEN, "KH1's Open command (chest)").is_none() {
        log!(Menus, "Chest Opening: KH1's Open handler not hooked: chests stay KH1's");
    }
}

/// KH1's Open handler, then KH2's fast open for a plain chest.
unsafe extern "system" fn open(sora: usize, chest: usize) {
    let orig: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_OPEN.load(Ordering::Acquire));
    let before = if sora != 0 { rd::<u32>(sora + ACTOR_APPLIED) & 0xFFFF } else { 0 };
    orig(sora, chest);
    if !kh2() || sora == 0 {
        FAST_AT.store(0, Ordering::Relaxed);
        return;
    }
    let base = base();
    let kind = rd::<u32>(base + ARG_KIND_STEAM);
    let what = rd::<u32>(base + ARG_ITEM_STEAM);
    let full = rd::<u32>(base + ARG_FULL_STEAM);
    // Kinds (KH1's chest table, Checked (exe + decomp)): 0 an item (KH1's handler gives it), 1 a gummi
    // block (the handler gives it with `0x14020D3F0`; "full" when the hangar holds the most), 2 a set of
    // Dalmatians (33 entries, contents 1..33: 99 puppies in threes; the handler gives nothing, the
    // treasure script does, and it still runs whole here), 3 munny (none in the table), 5 an event chest
    // (its own script entry), 7 a reward record (an item one becomes kind 0 in the handler).
    if full != 0 || !matches!(kind, 0..=3) {
        FAST_AT.store(0, Ordering::Relaxed);
        log!(Cutscenes, "Chest Opening: KH1's open (kind {kind}, contents {what:#x}, stock full {full})");
        return;
    }
    let lock = sora + ACTOR_LOCK;
    std::ptr::write_unaligned(lock as *mut u32, rd::<u32>(lock) & !LOCK_BIT);
    SORA.store(sora, Ordering::Relaxed);
    FAST_AT.store(ms() + 1, Ordering::Relaxed);
    SKIPPED.store(0, Ordering::Relaxed);
    RELEASED.store(0, Ordering::Relaxed);
    // KH1's treasure script holds Sora through actions on him (the tester's t725 trace: his actor
    // flags gain 0x1000000 (action 0x24), 0x200000 (0x26) and 0x40 (0x2B) on the frame after the Open,
    // and lose them 132 frames later; he couldn't walk meanwhile): our call handler goes into KH1's
    // script machine for the open's time and gives the script's actions on Sora KH1's "no actor" path.
    call_handler_in(base);
    // No open motion (the tester, answer 3 (b), as KH2: KH2's Open, command 0x1A5, is a message to the
    // box, execute 421 / action 0 in 03system's cmd table, and the box plays its own lid motion 0x98;
    // KH2's player has no open action): the handler's request for KH1's open motion is replaced by the
    // motion Sora had, through KH1's own request.
    let req = std::slice::from_raw_parts((base + REQUEST_STEAM) as *const u8, REQUEST_PROLOGUE.len());
    if req == REQUEST_PROLOGUE && rd::<i32>(sora + ACTOR_PENDING) == 1 && rd::<u32>(sora + ACTOR_REQUEST) == OPEN_REQUEST {
        let f: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + REQUEST_STEAM);
        f(sora, before, 0.0);
        log!(Cutscenes, "Chest Opening: KH1's open motion request replaced by Sora's own motion {before:#x}");
    }
    let line = if kind == 3 {
        format!("{what} Munny")
    } else if kind == 1 {
        "Gummi Block".to_string()
    } else if kind == 2 {
        // Assumption: set n holds puppies 3n-2 .. 3n (KH1's numbering of the 99).
        let n = what.max(1) * 3;
        format!("Dalmatians {:02}, {:02}, {:02}", n - 2, n - 1, n)
    } else {
        let name = if what < 0x100 { super::equip::item_name(base, what as u8) } else { None };
        match name {
            Some(n) => strip_codes(&n),
            None => format!("Item {what:#x}"),
        }
    };
    super::menu::queue_notice("Obtained", &line);
    log!(Cutscenes, "Chest Opening: KH2 open (kind {kind}, contents {what:#x}): Sora free at once, notice \"Obtained / {line}\", the treasure script's windows skipped");
}

/// The item name without the codes `item_name` writes as `{xx}`.
fn strip_codes(s: &str) -> String {
    let mut out = String::new();
    let mut skip = false;
    for c in s.chars() {
        match c {
            '{' => skip = true,
            '}' => skip = false,
            _ if !skip => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// The call comes from the treasure script during a recent fast open.
unsafe fn ours_to_skip(ctx: usize) -> bool {
    let at = FAST_AT.load(Ordering::Relaxed);
    if at == 0 || !kh2() {
        return false;
    }
    if ms() + 1 - at > FAST_MS {
        FAST_AT.store(0, Ordering::Relaxed);
        log!(
            Cutscenes,
            "Chest Opening: the open's time is over ({} message call(s) skipped, {} action(s) on Sora done at once / motion waits passed)",
            SKIPPED.load(Ordering::Relaxed),
            RELEASED.load(Ordering::Relaxed)
        );
        call_handler_out(base());
        return false;
    }
    let base = base();
    if rd::<u64>(base + SCRIPT_IN_USE_STEAM) as usize != base + TREASURE_STEAM {
        return false;
    }
    let first = base + CONTEXTS_STEAM;
    let count = rd::<u32>(base + COUNT_STEAM).min(CTX_MAX) as usize;
    ctx >= first && ctx < first + count * CTX_SIZE && (ctx - first) % CTX_SIZE == 0
}

/// A context of the treasure script (whatever the open's time).
unsafe fn treasure_ctx(base: usize, ctx: usize) -> bool {
    if rd::<u64>(base + SCRIPT_IN_USE_STEAM) as usize != base + TREASURE_STEAM {
        return false;
    }
    let first = base + CONTEXTS_STEAM;
    let count = rd::<u32>(base + COUNT_STEAM).min(CTX_MAX) as usize;
    ctx >= first && ctx < first + count * CTX_SIZE && (ctx - first) % CTX_SIZE == 0
}

/// Our call handler into KH1's class table (for an open's time; one data word, KH1's put back after).
unsafe fn call_handler_in(base: usize) {
    let at = base + CALL_ENTRY_STEAM;
    if rd::<u64>(at) as usize == base + CALL_STEAM {
        ORIG_CALL.store(base + CALL_STEAM, Ordering::Release);
        let _ = hook::patch(at, &((base + CALL_STEAM) as u64).to_le_bytes(), &(script_call as *const () as u64).to_le_bytes(), "script call handler (chest)");
    }
}

unsafe fn call_handler_out(base: usize) {
    let at = base + CALL_ENTRY_STEAM;
    if rd::<u64>(at) as usize == script_call as *const () as usize {
        let _ = hook::patch(at, &(script_call as *const () as u64).to_le_bytes(), &((base + CALL_STEAM) as u64).to_le_bytes(), "script call handler (chest, back)");
    }
}

#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Action([u8; ACTION_SIZE]);

/// KH1's event function call `(ctx, number)` while an open's time runs. From a treasure context: its
/// wait on Sora's motion (103) goes on at once, and the actions it queues on Sora are put in KH1's done
/// queue instead, as KH1's action step does for an action whose actor isn't there (so the script's own
/// move / turn waits end at once, and Sora is neither held, locked, turned nor animated). Everything
/// else the script does (the lid, sounds, the give, its flags) is KH1's.
unsafe extern "system" fn script_call(ctx: usize, n: i32) -> u64 {
    let orig: unsafe extern "system" fn(usize, i32) -> u64 = std::mem::transmute(ORIG_CALL.load(Ordering::Acquire));
    let base = base();
    let at = FAST_AT.load(Ordering::Relaxed);
    if at == 0 || ms() + 1 - at > FAST_MS {
        if at != 0 {
            FAST_AT.store(0, Ordering::Relaxed);
            log!(
                Cutscenes,
                "Chest Opening: the open's time is over ({} message call(s) skipped, {} action(s) on Sora done at once / motion waits passed)",
                SKIPPED.load(Ordering::Relaxed),
                RELEASED.load(Ordering::Relaxed)
            );
        }
        call_handler_out(base);
        return orig(ctx, n);
    }
    let sora = SORA.load(Ordering::Relaxed);
    if sora == 0 || !treasure_ctx(base, ctx) {
        return orig(ctx, n);
    }
    if n as usize == FN_WAIT_MOTION && rd::<u64>(ctx + CTX_ACTOR) as usize == sora {
        note_release(&format!("its wait on Sora's motion (function 103, context {ctx:#x}) goes on"));
        return 2;
    }
    let write = base + ACTIONS_WRITE_STEAM;
    let before = rd::<i32>(write);
    let r = orig(ctx, n);
    let after = rd::<i32>(write);
    if after != before {
        to_done(base, before, after, rd::<u32>(sora + 4), n);
    }
    r
}

/// The actions queued from `from` to `to` (ring indices): those on Sora (`id`) go to KH1's done queue,
/// the others are kept in order.
unsafe fn to_done(base: usize, from: i32, to: i32, id: u32, n: i32) {
    let ring = base + ACTIONS_STEAM;
    let slot = |i: i32| ring + (i as usize & 0xFF) * ACTION_SIZE;
    let mut kept: Vec<Action> = Vec::new();
    let mut moved: Vec<Action> = Vec::new();
    let mut i = from;
    let mut guard = 0;
    while i != to && guard < 256 {
        let a = std::ptr::read_unaligned(slot(i) as *const Action);
        if u32::from_le_bytes([a.0[4], a.0[5], a.0[6], a.0[7]]) == id {
            moved.push(a);
        } else {
            kept.push(a);
        }
        i = (i + 1) & 0xFF;
        guard += 1;
    }
    if moved.is_empty() {
        return;
    }
    let push: unsafe extern "system" fn(*const Action) -> i32 = std::mem::transmute(base + DONE_PUSH_STEAM);
    let mut back: Vec<Action> = Vec::new();
    for a in moved {
        let a = a;
        if push(&a) != 0 {
            back.push(a); // done queue full: left to KH1's action step
        } else {
            let kind = a.0[0];
            let arg = u32::from_le_bytes([a.0[1], a.0[2], a.0[3], 0]);
            note_release(&format!("action {kind:#x} (argument {arg:#x}) on Sora from function {n} done at once"));
        }
    }
    kept.extend(back);
    let mut w = from;
    for a in kept {
        std::ptr::write_unaligned(slot(w) as *mut Action, a);
        w = (w + 1) & 0xFF;
    }
    // (The ring's read index can't have passed `from`: the action step runs outside the script step.)
    let _ = ACTIONS_READ_STEAM;
    std::ptr::write_unaligned((base + ACTIONS_WRITE_STEAM) as *mut i32, w);
}

fn note_release(what: &str) {
    if RELEASED.fetch_add(1, Ordering::Relaxed) < 24 {
        log!(Cutscenes, "Chest Opening: treasure script: {what}");
    }
}

/// KH1's skip-mode branch of a message function: its arguments popped, nothing opened.
unsafe fn skip(ctx: usize, which: usize) -> u64 {
    let sp = ctx + CTX_SP;
    std::ptr::write_unaligned(sp as *mut i32, rd::<i32>(sp) - MESSAGE_FNS[which].2);
    if SKIPPED.fetch_add(1, Ordering::Relaxed) < 8 {
        log!(Cutscenes, "Chest Opening: treasure script's message function {} skipped", MESSAGE_FNS[which].0);
    }
    2
}

unsafe fn call(ctx: usize, which: usize) -> u64 {
    if ours_to_skip(ctx) {
        return skip(ctx, which);
    }
    let f: unsafe extern "system" fn(usize) -> u64 = std::mem::transmute(ORIG_FN[which].load(Ordering::Acquire));
    f(ctx)
}

unsafe extern "system" fn msg0(ctx: usize) -> u64 {
    call(ctx, 0)
}
unsafe extern "system" fn msg1(ctx: usize) -> u64 {
    call(ctx, 1)
}
unsafe extern "system" fn msg177(ctx: usize) -> u64 {
    call(ctx, 2)
}
unsafe extern "system" fn msg605(ctx: usize) -> u64 {
    call(ctx, 3)
}

#[cfg(test)]
mod tests {
    use super::strip_codes;

    #[test]
    fn strips_unknown_codes() {
        assert_eq!(strip_codes("Hi-Potion"), "Hi-Potion");
        assert_eq!(strip_codes("Elixir{7f}"), "Elixir");
    }
}
