//! Chest Opening (Vanilla | Quick | Skip since t770; KH1 | KH2 before), the Config menu's QoL row (KH2 by default; it was Combat > Tweaks in t724) (the tester Mr Bugs, thread
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
//! - t768 (the tester's t743 log and traces: still held, flags `0x0130c0c3`, "runner 0x2, engine 0xc"):
//!   the script names Sora as actor 0 (the party's first), so its actions are matched through KH1's own
//!   lookup (`0x140291D30`) now, not his `+4`; its player holds (event functions 511 / 512, 113 / 115,
//!   112 / 114: the event game mode and the engine's command lock) are skipped from a treasure context;
//!   and the handler's call for the open motion goes to `open_request`, which asks for nothing on a fast
//!   open (t729's swap of the pending request never matched: no log line in that log).
//! - t769, the room scripts' boxes (the tester: "another chest type which is the evidence boxes you find
//!   in alice's world"): those are no chest of KH1's chest table but parts of the room's own script, run
//!   when a command it offered (event function 454) is picked. KH1's pick (`0x1401A4D00`) is hooked: for
//!   command 0x55 or 0x49 on KH2, the part KH1 just woke is read (`crate::kh1script::read_box`); a box
//!   (it takes the player with 511, gives with 258 or 583, gives him back with 512, and has no scene,
//!   choice, shop or menu) gets the same treatment as a chest for its script's contexts until its 512,
//!   and its give shows the notice. Anything else stays KH1's.
//! - (t724..t775) What was found was shown in KH1's level-up notice box (`menu::queue_notice`) with the
//!   script's windows skipped. (t776, the owner: "exactly like" the evidence boxes' box) The script's own
//!   message windows run as KH1 wrote them (window 7, its lines, its own close), with Sora free; on Quick
//!   a window's open waits until 100 frames into the open (vanilla's frame, t775).
//!
//! Anything else (an event chest, a full stock, a reward record that is not an item, kind 1, any other
//! script using those contexts) is KH1's as before. KH1: nothing of this is installed until the row is
//! first set to KH2; then one flag check per Open and per message function call.

use super::addr::Build;
use super::{hook, log};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use crate::kh1script::{pickup_parts, read_box, read_pickup, releases_after_give, BoxOpen, FN_ADD_ITEM, FN_GIVE_REWARD, FN_RELEASE};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// The row's value (`chest_style = vanilla | quick | skip`, t770): 0 KH1's, 1 Quick (Sora plays a chest
/// animation, then moves on), 2 Skip (no animation: as KH2).
static STYLE: AtomicU8 = AtomicU8::new(SKIP);
pub const VANILLA: u8 = 0;
pub const QUICK: u8 = 1;
pub const SKIP: u8 = 2;
/// Quick's animation (`chest_anim = kh1 | kh2`): KH2's (true) or KH1's open motion.
static ANIM_KH2: AtomicBool = AtomicBool::new(false);
/// Quick: Sora's lock is the mod's until his open motion is over (the actor it was set on, 0: none).
static QUICK_LOCK: AtomicUsize = AtomicUsize::new(0);
/// Sora's "can't act" bit (`+0x444` bit 0: KH1's action 0xE sets it, 0xD clears it; event functions 55 / 54).
const CANT_ACT_BIT: u32 = 1;
/// Quick (t773): Sora's actor flags (`+0`) just before the open, and his applied motion request then
/// (`+0x164`, low 16 bits).
static QUICK_FLAGS: AtomicU32 = AtomicU32::new(0);
static QUICK_FROM: AtomicU32 = AtomicU32::new(0);
/// Quick: his open motion has started (the applied request left the one he had at the open).
static QUICK_PLAYED: AtomicBool = AtomicBool::new(false);
/// Quick: he is back on idle after it and has been let go; from then on the open goes the Skip way.
static QUICK_FREE: AtomicBool = AtomicBool::new(false);
/// The applied motion request (`+0x164`; the trace's kh1_motion: 0xA6 the open, 0 idle, slot 0).
const ACTOR_APPLIED: usize = 0x164;
/// Quick: the "Obtained" notice shows this many 60 Hz frames after the Open (the owner: vanilla's window
/// comes 100 frames in, the open motion and 20 frames of idle).
const QUICK_NOTICE_FRAMES: u64 = 100;
/// Quick: the 60 Hz frame (`takeover::TICKS`) the open's window may open from (0: no wait).
static WINDOW_DUE: AtomicU64 = AtomicU64::new(0);
/// The actor-flag bits the open and the treasure script hold him with (the tester's t772 trace: 0x80 and
/// 0x100000 on the open's frame, 0x1000000 / 0x200000 / 0x40 the frame after; all gone together when the
/// script lets him go, flags back to what they were before the open).
const HOLD_FLAGS: u32 = 0x0100_0000 | 0x0020_0000 | 0x0010_0000 | 0x80 | 0x40;
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
/// The Open handler's motion request call goes to `open_request` (t768).
static REQUEST_REDIRECTED: AtomicBool = AtomicBool::new(false);
/// The player-hold calls skipped for the current open (for the log, t768).
static HOLDS: AtomicU32 = AtomicU32::new(0);
/// What the open in progress is (t769): the treasure script's (an Open on a chest) or a box of a room's
/// script (a picked command whose part reads as a box), and for a box its context list and count.
static MODE: AtomicU8 = AtomicU8::new(0);
const MODE_NONE: u8 = 0;
const MODE_TREASURE: u8 = 1;
const MODE_BOX: u8 = 2;
/// (t795) An item picked up off the ground: a room script's part a Triangle command woke through KH1's
/// event dispatch (QoL > Item Pickup), treated as a box, its holds skipped only from its own context.
const MODE_PICKUP: u8 = 3;
/// QoL > Item Pickup (`item_pickup = vanilla | skip`, t795): on Skip, a pickup leaves Sora free.
static PICKUP: AtomicBool = AtomicBool::new(false);
static PICKUP_INSTALLED: AtomicBool = AtomicBool::new(false);
/// The pickup's own context (its holds and its 512 are the ones skipped).
static PICKUP_CTX: AtomicUsize = AtomicUsize::new(0);
/// (t809) The pickup's sub-script (its first instruction and its end) and when its context left it (ms + 1,
/// 0: still in it). The pickup is over a short while after it has left (`PICKUP_AFTER_MS`), so what the
/// part woke on the way (Sora's getting up) is still done at once.
static PICKUP_LEFT: AtomicU64 = AtomicU64::new(0);
const PICKUP_AFTER_MS: u64 = 500;
/// (t810) The instruction ranges of the pickup's own context (`kh1script::own_subs`: the part and the
/// sub-scripts of its context it wakes).
/// (t815) Every context the part runs in, by address, with the range of each sub-script it runs there
/// (`kh1script::pickup_parts`): the slides' part runs in the room context their touch wakes.
static PICKUP_RANGES: Mutex<Vec<(usize, usize, usize)>> = Mutex::new(Vec::new());
/// (t810) A polled pickup: the touch only sets a flag, and the context's running sub-script (its main loop)
/// does the pickup when it sees it (the rope in `di01`: its touch part, sub-script 6, sets local 0 and calls
/// 26; sub-script 1, a loop, holds, gives and shows the window when local 0 is 1; Checked (files + the t809
/// log: "event 1's part (context 44, sub-script 6) isn't a pickup")). It is over at its last give-back
/// after the give (`PICKUP_RELEASES` of `PICKUP_RELEASE_FN`), or when nothing is given within
/// `PICKUP_POLL_MS` (the flag wasn't for a pickup this time).
static PICKUP_POLLED: AtomicBool = AtomicBool::new(false);
static PICKUP_GIVEN: AtomicBool = AtomicBool::new(false);
static PICKUP_RELEASES: AtomicUsize = AtomicUsize::new(0);
static PICKUP_RELEASE_FN: AtomicU32 = AtomicU32::new(0);
const PICKUP_POLL_MS: u64 = 1500;
/// (t811) KH1's window functions a pickup's part calls on window 7, and how many values each pops (Checked
/// (exe)): 2 close, 3 / 4 / 5 / 6 / 80 / 83 its settings, 107 its wait, 365 its line's placement.
const PICKUP_WINDOW_FNS: [(i32, i32); 9] = [(2, 1), (3, 3), (4, 3), (5, 2), (6, 2), (80, 2), (83, 2), (107, 1), (365, 4)];
static ORIG_EVENT: AtomicUsize = AtomicUsize::new(0);
/// KH1's event dispatch `0x1401A1A10 (object id, event, argument)` (Steam; Checked (decomp)): run for the
/// events queued by `0x14028DD80` and flushed by `0x14028E550`. Events 0 and 4 come only from the
/// Triangle command step (`0x1402A3DB0` and `0x1402A4150`, command kinds 4 and 5; Checked (decomp): no
/// other caller queues them). Event 0 wakes the object's context's sub-script 5 (`+0x90 + 5 * 4`), event 4
/// its sub-script 3, in the room list (`0x142382AD0`) or the event list as the pick does. First bytes
/// `mov [rsp+0x10], rbx; mov [rsp+0x18], rbp; push rsi; sub rsp, 0x20` (Checked (exe)).
const EVENT_STEAM: usize = 0x1A1A10;
const EVENT_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x6C, 0x24, 0x18, 0x56, 0x48, 0x83, 0xEC, 0x20];
static BOX_CTXS: AtomicUsize = AtomicUsize::new(0);
static BOX_COUNT: AtomicUsize = AtomicUsize::new(0);
static ORIG_PICK: AtomicUsize = AtomicUsize::new(0);

/// KH1's Open command handler `(sora, chest)` (Steam; `mov [rsp+8], rbx; mov [rsp+0x10], rbp`,
/// Checked (exe)).
const OPEN_STEAM: usize = 0x2B58E0;
const OPEN_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];
/// The open's arguments as the handler leaves them (Steam, Checked (decomp)): kind, item or amount,
/// stock full.
const ARG_KIND_STEAM: usize = 0x2DB7B30;
const ARG_ITEM_STEAM: usize = 0x2DB7B34;
const ARG_FULL_STEAM: usize = 0x2DB7B38;
/// KH1's motion request `0x14029F980 (actor, request, f32 in XMM2)` (Steam; Checked (decomp + exe): it
/// sets the one pending request, `+0x178` count 1, `+0x17C` the request) and the call of it in the Open
/// handler that asks for Sora's open motion (`0x1402B59CC`: `mov edx, 0x900A6; mov rcx, rdi; call`,
/// Checked (exe)). t768: that call goes to `open_request`, which on a fast open asks for nothing.
const REQUEST_STEAM: usize = 0x29F980;
const OPEN_REQUEST_CALL_STEAM: usize = 0x2B59CC;
/// (t807) KH1's action step's motion request (`0x14028E6E0`, action 0x11 `push request, push blend`: its call
/// of `0x14029F980` at `0x14028E8AC`, Checked (exe + decomp)). A room script's box asks Sora's open motion
/// this way (not through the Open handler).
const ACTION_REQUEST_CALL_STEAM: usize = 0x28E8AC;
/// The Open handler's stop (`0x1402B59D7`: `0x140297F30 (sora, 0.0)` clears flag 0x100000, sets 0x80 and
/// zeroes his speed `+0xC`), Checked (exe).
const OPEN_STOP_CALL_STEAM: usize = 0x2B59D7;
const STOP_STEAM: usize = 0x297F30;
/// The chest's tick's first-frame call (`0x1402B5879`: `0x1402AE160 (chest, 0)`, run once, when its touch
/// callback is registered), Checked (exe).
const CHEST_FIRST_CALL_STEAM: usize = 0x2B5879;
const CHEST_FIRST_STEAM: usize = 0x2AE160;
/// The chest's object record (handle at actor `+0x130`): `+0x44` its offer radius, `+0x5E` the shape of
/// the range test `0x140293FC0` (0 cylinder, 1 flat + height with no facing, 2 point, 3 flat; 0, 2 and 3
/// also need Sora to face it: cos > 0), `+0x5F` its kind (3: its own handler decides, the chest's touch
/// callback). Checked (exe).
const REC_RADIUS: usize = 0x44;
const REC_SHAPE: usize = 0x5E;
const REC_KIND: usize = 0x5F;
/// KH2's reach for the box's Open (t779): the box's reaction area (`F_EX030.mdlx`, its type-5 shape: a
/// sphere of radius 150, 150 above its base) touching Sora's body (radius 35): 185 across; its facing rules
/// are applied by `chest_touch` (t780). Checked (files + decomp: `0x140407DE0` walks the box's type-5 shapes).
const KH2_REACH: f32 = 185.0;
const SHAPE_FLAT_HEIGHT: u8 = 1;
/// The chest's touch callback (`0x1402B5A30 (chest, sora, code)`: code -1 = "offer?", it offers Open 0x37
/// in state 0 with the height difference in (-50, 20)), `mov [rsp+8], rbx; push rdi; sub rsp, 0x20`.
const TOUCH_STEAM: usize = 0x2B5A30;
const TOUCH_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20];
static ORIG_TOUCH: AtomicUsize = AtomicUsize::new(0);
/// KH2's box Open (03system `cmd` 0x20, the box's reaction shape's argument; Checked (files)) is usable only
/// when (`0x1404088F0`, Checked (decomp)) Sora's facing · (box - Sora) >= its `+0x10` (-0.7071: the box at
/// most 135 degrees off his facing) and (box - Sora) · (the box's facing turned by `+0x14` + pi) >= its
/// `+0x18` (6.1e-17, i.e. 0: Sora on the box's front side); otherwise it is greyed.
/// KH1's command box: the command per slot (`0x140528914`, slot 3 the Triangle box), greyed and blocked bits
/// (`0x140528904`, `0x140528908`); the chest's Open is command 0x37. Checked (exe).
const MENU_SLOTS_STEAM: usize = 0x528914;
const GREYED_STEAM: usize = 0x528904;
const BLOCKED_STEAM: usize = 0x528908;
const OPEN_COMMAND: u8 = 0x37;
const KH2_OPEN_FACING: f32 = -0.707_106_77;
const KH2_OPEN_FRONT: f32 = 6.123_031_8e-17;
const OPEN_REQUEST: u32 = 0x900A6;
const ACTOR_REQUEST: usize = 0x17C;
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
/// KH1's actor lookup `0x140291D30 (id)` (Steam; Checked (decomp + exe)): the actor an action names
/// (0, 1, 2: the party's three; `0x1xxxx`: an object by its entry; else an actor's own `+4`), as KH1's
/// action step finds it. The treasure script's context names Sora as 0 (its actor id `+0` starts at 0,
/// `0x1401A43A0`), not by his own id, so t728's match on his `+4` never caught its actions (the tester's
/// log: no "done at once" line at any open). `mov [rsp+8], rbx; mov [rsp+0x10], rbp`.
const FIND_ACTOR_STEAM: usize = 0x291D30;
const FIND_ACTOR_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10];
/// The event functions that hold the player for a script, none with arguments (Checked (exe)): 511
/// (`0x1401A7540`, the runner's bit 1: KH1's game mode becomes "event", `0x14017EFC0`), 512 (its
/// release, `0x1401A7660`), 113 / 115 (`0x1401A9A80` / `0x1401A9AA0`: the engine word `0x142867364` bits
/// 8 and 4; bit 4 stops KH1's command input, `0x1402937F0`) and 112 / 114 (their clears). The treasure
/// script calls 113 and 115 at its start and 511 first in its main part (Checked (files):
/// `UK_treasure.ev` entry 0), which is the "runner 0x2, engine 0xc" in the tester's log at every open.
const HOLD_FNS: [i32; 6] = [511, 512, 112, 113, 114, 115];
/// Event function 103 (`0x1401A96F0`): waits (returns 4) until the context's actor's motion is over,
/// then goes on (2) (Checked (decomp)).
const FN_WAIT_MOTION: usize = 103;
/// Event function 58: a motion request on the context's actor (`push request, push blend`: action 0x11),
/// Checked (exe). The treasure script asks for Sora's idle with it (context 1, sub-script 12: request 0,
/// blend 4), Checked (files).
const FN_MOTION_REQUEST: i32 = 58;
/// KH2's chest contact (t802): Sora's capture motion (`F_EX030.mset` 252) and the box's lid motion 0x99 start
/// together (`0x1404025C0`: lid 0x99 and the capture 0xFC on the same frame, Checked (decomp)); the
/// Keyblade's frame trigger 8 (sound 500034) and the lid's (sound 22) are both at frame 28 of 60, Checked
/// (files).
const KH2_CONTACT_FRAME: f32 = 28.0;
/// KH1's trail request (`0x1402C2310 (actor, trail id)`, Steam) and the actor's weapon object handle
/// (`+0x14C`), Checked (exe).
const TRAIL_REQUEST_STEAM: usize = 0x2C2310;
const ACTOR_WEAPON: usize = 0x14C;
/// (t803) The item window after the lid opens: vanilla's comes 20 frames after its open motion ends, which is
/// when KH1 opens the lid (the owner: "full chest animation + 20 frames of idle", 100 in all; Checked (trace
/// `..._f2177`): the open motion ended at f2107, the script's window call came at f2129).
const LID_TO_WINDOW: u64 = 20;
static CONTACT_LOGGED: AtomicBool = AtomicBool::new(false);

/// KH1's Triangle-box command pick `0x1401A4D00 (object, _, command)` (Steam; Checked (decomp)): for a
/// command a room script offered (event function 454), it finds the object's context (the room
/// script's list `0x142382AD0` / count `0x14237F4E4` with entry 0 in use `0x142382524`, else the event
/// list `0x1423902D0` / `0x14237F4E8`; the context whose `+0` is the object's `+4`), wakes its
/// sub-script 9 (`0x1401A0AF0`, its start at `+0x90 + 9 * 4`, the next instruction `+0x190`) and keeps
/// the command at `0x142382A38`. First bytes `mov [rsp+8], rbx; mov [rsp+0x10], rbp; mov [rsp+0x18],
/// rsi` (Checked (exe)).
const PICK_STEAM: usize = 0x1A4D00;
const PICK_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18];
const ENTRY_STEAM: usize = 0x2382524;
const ROOM_CTXS_STEAM: usize = 0x2382AD0;
const ROOM_COUNT_STEAM: usize = 0x237F4E4;
const EVENT_CTXS_STEAM: usize = 0x23902D0;
const EVENT_COUNT_STEAM: usize = 0x237F4E8;
const CTX_CODE: usize = 0x80;
const CTX_SUBS: usize = 0x88;
const CTX_SUB_STARTS: usize = 0x90;
const CTX_PC: usize = 0x190;
/// A context's waiting starts by priority (`+0x30C`, 8 of them) and its current priority (`+0x32C`)
/// (Checked (decomp): `0x1401A0AF0`).
const CTX_WAKE_SLOTS: usize = 0x30C;
const HANDLER_SUB: usize = 9;
/// The commands the room scripts' boxes are opened with (Checked (files): across the 236 room files,
/// the parts that take the player, give an item or a reward and give him back with no scene, choice,
/// shop or menu are 0x55's in Wonderland (`aw02`, `aw05`: the evidence, items 0xDF..0xE2) and End of
/// the World (`ew02`, reward records), and 0x49's in `aw02` (evidence)). Traverse Town's 0x72 reads the
/// same way but is left KH1's (not a box the tester asked for; Assumption: the mailbox).
const BOX_COMMANDS: [i32; 2] = [0x55, 0x49];
/// What makes a part a scene, not a box: a choice (88), a shop or menu (222, 268, 278, 392, 628, 629,
/// 673), an event entry (126), a pause-skippable section (84, 139, 476, 489, 623) (as
/// `ingameskip::PLAYS`).
const BOX_PLAYS: [u32; 14] = [88, 222, 268, 278, 392, 628, 629, 673, 126, 84, 139, 476, 489, 623];
/// KH1's reward records (`0x14028F9A0 (n)`: a u16 each from the pointer at `0x142D36D58`; low four bits
/// the kind, 0 an item whose id is the high byte; Checked (exe)), for the notice of a box's 583.
const REWARDS_PTR_STEAM: usize = 0x2D36D58;
/// The party's first actor (Sora), the pointer KH1's actor lookup returns for id 0 (`0x142D37280`,
/// Checked (decomp)).
const SORA_PTR_STEAM: usize = 0x2D37280;

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

/// The open is the mod's (Quick or Skip): no item window, no hold beyond the animation.
pub fn kh2() -> bool {
    STYLE.load(Ordering::Relaxed) != VANILLA
}

/// The row's value: `VANILLA`, `QUICK` or `SKIP`.
pub fn style() -> u8 {
    STYLE.load(Ordering::Relaxed)
}

/// Quick's animation is KH2's.
pub fn anim_kh2() -> bool {
    ANIM_KH2.load(Ordering::Relaxed)
}

/// Sora opens a chest on KH2's chest motion (t783): Chest Opening on Quick and its Chest Animation row at
/// KH2. `kh2slots` points KH1's open request (0xA6) at its slot then (`kh2slots::CHEST_REQUEST`).
pub fn kh2_animation() -> bool {
    style() == QUICK && anim_kh2()
}

fn style_name(v: u8) -> &'static str {
    match v {
        VANILLA => "Vanilla",
        QUICK => "Quick",
        _ => "Skip",
    }
}

pub unsafe fn install() {
    let s = super::settings::get();
    STYLE.store(s.chest_style.min(SKIP), Ordering::Relaxed);
    ANIM_KH2.store(s.chest_anim_kh2, Ordering::Relaxed);
    if kh2() {
        hook_in();
    }
    log!(Menus, "Chest Opening: {} (chest_style), Quick's animation {} (chest_anim)", style_name(style()), if anim_kh2() { "KH2" } else { "KH1" });
    PICKUP.store(s.item_pickup, Ordering::Relaxed);
    if pickup() {
        pickup_hook_in();
    }
    log!(Menus, "Item Pickup: {} (item_pickup)", if pickup() { "Skip" } else { "Vanilla" });
}

/// QoL > Item Pickup is on Skip (t795).
pub fn pickup() -> bool {
    PICKUP.load(Ordering::Relaxed)
}

/// The Config menu's Item Pickup row, kept in the ini.
pub fn set_pickup(v: bool) {
    PICKUP.store(v, Ordering::Relaxed);
    if v {
        unsafe { pickup_hook_in() };
    }
    super::settings::save("item_pickup", if v { "skip" } else { "vanilla" });
    log!(Menus, "Config menu: Item Pickup {}", if v { "Skip" } else { "Vanilla" });
}

/// The pickup's way in (t795): KH1's event dispatch (Steam only, once). Nothing of it runs until a
/// Triangle command's event; on Vanilla the hook passes everything straight to KH1.
unsafe fn pickup_hook_in() {
    if PICKUP_INSTALLED.swap(true, Ordering::Relaxed) {
        return;
    }
    if Build::current() != Build::Steam {
        log!(Menus, "Item Pickup: Steam only (the addresses are the Steam build's): pickups stay KH1's");
        return;
    }
    let base = base();
    // KH1's script call handler and done queue as freeing Sora needs them (as `hook_in`).
    let entry = rd::<u64>(base + CALL_ENTRY_STEAM) as usize;
    let done = std::slice::from_raw_parts((base + DONE_PUSH_STEAM) as *const u8, DONE_PUSH_PROLOGUE.len());
    let find = std::slice::from_raw_parts((base + FIND_ACTOR_STEAM) as *const u8, FIND_ACTOR_PROLOGUE.len());
    let ours = script_call as *const () as usize;
    if (entry != base + CALL_STEAM && entry != ours) || done != DONE_PUSH_PROLOGUE || find != FIND_ACTOR_PROLOGUE {
        log!(Cutscenes, "Item Pickup: KH1's script call handler ({entry:#x}) or done queue isn't as expected: pickups stay KH1's");
        return;
    }
    // (t806) The pickup's own window is skipped for the in-game skip's "Obtained" one: KH1's message
    // functions go through ours.
    if !wrap_messages(base) {
        log!(Menus, "Item Pickup: KH1's message functions not wrapped: pickups stay KH1's");
        return;
    }
    if hook::install(base + EVENT_STEAM, &EVENT_PROLOGUE, event as *const () as usize, &ORIG_EVENT, "KH1's event dispatch (item pickup)").is_none() {
        log!(Menus, "Item Pickup: KH1's event dispatch not hooked: pickups stay KH1's");
    }
}

/// KH1's four message functions in the event table point at ours (once; t806: shared by Chest Opening and
/// Item Pickup). False when not all four could be wrapped.
static MESSAGES_WRAPPED: AtomicU8 = AtomicU8::new(0);
unsafe fn wrap_messages(base: usize) -> bool {
    match MESSAGES_WRAPPED.load(Ordering::Relaxed) {
        1 => return true,
        2 => return false,
        _ => {}
    }
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
    let ok = wrapped == MESSAGE_FNS.len();
    if !ok {
        log!(Menus, "Chest Opening / Item Pickup: only {wrapped} of KH1's message functions wrapped");
    }
    MESSAGES_WRAPPED.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
    ok
}

/// KH1's event dispatch, then, for a Triangle command's event (0 or 4), a touch (1) or event 3 on Skip (t801), the woken part read as a
/// pickup (`crate::kh1script::read_pickup`): one runs whole as KH1 wrote it (its give, its "Obtained" window
/// 7, its sound), with its holds skipped and its actions on Sora done at once, as a box's (t769 / t776).
unsafe extern "system" fn event(id: i32, ev: i32, arg: usize) {
    let orig: unsafe extern "system" fn(i32, i32, usize) = std::mem::transmute(ORIG_EVENT.load(Ordering::Acquire));
    // (t801) The sub-script each event wakes and the priority it is woken at (Checked (decomp): event 0
    // sub-script 5 at 3, event 1 (a touch, `0x140297E80`) 6 at 2, event 3 (`0x1402B0610`) 7 at 4, event 4
    // 3 at 3; event 2, a hit, is left out).
    let (sub_n, prio) = match ev {
        0 => (5usize, 3usize),
        1 => (6, 2),
        3 => (7, 4),
        4 => (3, 3),
        _ => return orig(id, ev, arg),
    };
    let at = FAST_AT.load(Ordering::Relaxed);
    // (t809) Another pickup while one is still on takes over (two things grabbed in a row).
    let other = MODE.load(Ordering::Relaxed) != MODE_PICKUP;
    if !pickup() || (other && at != 0 && ms() + 1 - at <= FAST_MS) {
        if pickup() && ev != 1 {
            log!(Cutscenes, "Item Pickup: event {ev} on object {id:#x} during an open: KH1's");
        }
        return orig(id, ev, arg);
    }
    let base = base();
    let (list, count_at) = if rd::<i32>(base + ENTRY_STEAM) == 0 { (base + ROOM_CTXS_STEAM, base + ROOM_COUNT_STEAM) } else { (base + EVENT_CTXS_STEAM, base + EVENT_COUNT_STEAM) };
    orig(id, ev, arg);
    let count = rd::<i32>(count_at).clamp(0, CTX_MAX as i32) as usize;
    // (t801, the owner's t800 log: no Item Pickup line at all at Destiny Islands' pickups) Every way out is
    // logged now, except for touches (event 1, frequent) that aren't pickups.
    let quiet = ev == 1;
    let Some(c) = (0..count).find(|&c| rd::<i32>(list + c * CTX_SIZE) == id) else {
        if !quiet {
            log!(Cutscenes, "Item Pickup: event {ev} on object {id:#x}: no context of the {} list has it: KH1's", if list == base + ROOM_CTXS_STEAM { "room" } else { "event" });
        }
        return;
    };
    let ctx = list + c * CTX_SIZE;
    if rd::<i32>(ctx + CTX_SUBS).clamp(0, 64) as usize <= sub_n {
        if !quiet {
            log!(Cutscenes, "Item Pickup: event {ev}: context {c} has no sub-script {sub_n}: KH1's");
        }
        return;
    }
    let start = rd::<i32>(ctx + CTX_SUB_STARTS + sub_n * 4);
    // KH1 woke it: its next instruction is the sub-script's start, or (t801) the context runs at a higher
    // priority and the start waits in the slot of the wake's priority (`+0x30C + priority * 4`, run when
    // that level ends; `0x1401A0AF0`, Checked (decomp)). t795 only took the first.
    let pc = rd::<i32>(ctx + CTX_PC);
    let pending = rd::<i32>(ctx + CTX_WAKE_SLOTS + prio * 4);
    if start < 0 || (pc != start && pending != start) {
        // (t806) Logged for touches too (a touch is queued once per new contact, `0x140297E80`): the t805 log's
        // rope pickup left no line.
        if !quiet || start >= 0 {
            log!(Cutscenes, "Item Pickup: event {ev}: context {c}'s sub-script {sub_n} (start {start}) wasn't woken (next {pc}, pending {pending}): KH1's");
        }
        return;
    }
    let word = |c: usize, i: usize| -> Option<u32> {
        if c >= count {
            return None;
        }
        let code = rd::<u64>(list + c * CTX_SIZE + CTX_CODE) as usize;
        let at = code.checked_add(i.checked_mul(4)?)?;
        (code >= 0x10000 && super::crash::readable(at) && super::crash::readable(at + 3)).then(|| rd::<u32>(at))
    };
    let sub = |c: usize, s: usize| -> Option<usize> {
        if c >= count || s >= rd::<i32>(list + c * CTX_SIZE + CTX_SUBS).clamp(0, 64) as usize {
            return None;
        }
        usize::try_from(rd::<i32>(list + c * CTX_SIZE + CTX_SUB_STARTS + s * 4)).ok()
    };
    let subs = rd::<i32>(ctx + CTX_SUBS).clamp(0, 64) as usize;
    let found = match read_pickup(c, start as usize, word, sub, &BOX_PLAYS) {
        BoxOpen::Give(n) => Some((start as usize, n, sub_n, false)),
        r => {
            match r {
                BoxOpen::Not(f) => log!(Cutscenes, "Item Pickup: event {ev}'s part (context {c}, sub-script {sub_n}) isn't a pickup itself (event function {f})"),
                _ => log!(Cutscenes, "Item Pickup: event {ev}'s part (context {c}, sub-script {sub_n}) couldn't be read"),
            }
            // (t810) A polled pickup: another sub-script of the same context that reads as one.
            (0..subs).filter(|&s| s != sub_n).find_map(|s| {
                let first = sub(c, s)?;
                match read_pickup(c, first, word, sub, &BOX_PLAYS) {
                    BoxOpen::Give(n) => Some((first, n, s, true)),
                    _ => None,
                }
            })
        }
    };
    let Some((first, n, s, polled)) = found else {
        log!(Cutscenes, "Item Pickup: no sub-script of context {c} reads as a pickup: KH1's");
        return;
    };
    let sora = rd::<u64>(base + SORA_PTR_STEAM) as usize;
    if sora < 0x10000 {
        return;
    }
    SORA.store(sora, Ordering::Relaxed);
    BOX_CTXS.store(list, Ordering::Relaxed);
    BOX_COUNT.store(count_at, Ordering::Relaxed);
    PICKUP_CTX.store(ctx, Ordering::Relaxed);
    let ranges: Vec<(usize, usize, usize)> = pickup_parts(c, if polled { first } else { start as usize }, word, sub).into_iter().map(|(pc, f, e)| (list + pc * CTX_SIZE, f, e)).collect();
    if let Ok(mut r) = PICKUP_RANGES.lock() {
        *r = ranges.clone();
    }
    let (releases, release_fn) = releases_after_give(c, first, word);
    PICKUP_POLLED.store(polled, Ordering::Relaxed);
    PICKUP_GIVEN.store(false, Ordering::Relaxed);
    PICKUP_RELEASES.store(releases.max(1), Ordering::Relaxed);
    PICKUP_RELEASE_FN.store(release_fn, Ordering::Relaxed);
    PICKUP_LEFT.store(0, Ordering::Relaxed);
    MODE.store(MODE_PICKUP, Ordering::Relaxed);
    WINDOW_DUE.store(0, Ordering::Relaxed);
    FAST_AT.store(ms() + 1, Ordering::Relaxed);
    SKIPPED.store(0, Ordering::Relaxed);
    RELEASED.store(0, Ordering::Relaxed);
    HOLDS.store(0, Ordering::Relaxed);
    call_handler_in(base);
    if polled {
        log!(Cutscenes, "Item Pickup: event {ev} on context {c}: its sub-script {s} ({n} instructions read) is a pickup that polls the touch: its holds skipped, Sora free; over at its {releases} give-back(s) (event function {release_fn}) after the give, or {PICKUP_POLL_MS} ms with no give");
    } else {
        log!(Cutscenes, "Item Pickup: event {ev} woke context {c}'s sub-script {s} ({n} instructions read, own ranges {ranges:?}): a pickup: its holds skipped, Sora free");
    }
}

/// The Config menu's Chest Opening row, kept in the ini.
pub fn set_style(v: u8) {
    let v = v.min(SKIP);
    STYLE.store(v, Ordering::Relaxed);
    if v != VANILLA {
        unsafe { hook_in() };
    }
    // (KH1's open request onto KH2's chest motion or back, at once: `kh2_animation`.)
    unsafe { super::kh2slots::refresh() };
    FAST_AT.store(0, Ordering::Relaxed);
    super::settings::save("chest_style", match v {
        VANILLA => "vanilla",
        QUICK => "quick",
        _ => "skip",
    });
    log!(Menus, "Config menu: Chest Opening {}", style_name(v));
}

/// The Config menu's Chest Animation row (shown on Quick), kept in the ini.
pub fn set_anim_kh2(v: bool) {
    ANIM_KH2.store(v, Ordering::Relaxed);
    unsafe { super::kh2slots::refresh() };
    super::settings::save("chest_anim", if v { "kh2" } else { "kh1" });
    log!(Menus, "Config menu: Chest Animation {}", if v { "KH2" } else { "KH1" });
}

/// Quick: the open starts on `sora` (his flags and applied request read before KH1's open changed them).
fn quick_start(sora: usize, flags: u32, from: u32) {
    QUICK_LOCK.store(sora, Ordering::Relaxed);
    QUICK_FLAGS.store(flags, Ordering::Relaxed);
    QUICK_FROM.store(from & 0xFFFF, Ordering::Relaxed);
    QUICK_PLAYED.store(false, Ordering::Relaxed);
    QUICK_FREE.store(false, Ordering::Relaxed);
    CONTACT_LOGGED.store(false, Ordering::Relaxed);
}

/// Quick, still holding: the open's actions on Sora and the waits on his motion are KH1's.
fn quick_holding() -> bool {
    style() == QUICK && !QUICK_FREE.load(Ordering::Relaxed) && MODE.load(Ordering::Relaxed) != MODE_PICKUP
}

/// Quick (t773): KH1's open motion plays, then the treasure script asks for his idle (slot 0) but keeps him
/// held for about 50 frames more (the tester's t772 trace: idle from f2108, flags back only at f2159). As
/// soon as the applied request is idle after the open motion, he is let go: `+0x444` bits 0 / 1 off and
/// the hold flags back to what they were before the open; the rest of the open goes the Skip way.
unsafe fn quick_check() {
    let sora = QUICK_LOCK.load(Ordering::Relaxed);
    if sora < 0x10000 || style() != QUICK || QUICK_FREE.load(Ordering::Relaxed) {
        return;
    }
    let now = rd::<u32>(sora + ACTOR_APPLIED) & 0xFFFF;
    if !QUICK_PLAYED.load(Ordering::Relaxed) {
        if now != 0 && now != QUICK_FROM.load(Ordering::Relaxed) {
            QUICK_PLAYED.store(true, Ordering::Relaxed);
        }
        return;
    }
    if now == 0 {
        QUICK_FREE.store(true, Ordering::Relaxed);
        let lock = rd::<u32>(sora + ACTOR_LOCK);
        let flags = rd::<u32>(sora);
        let free = (flags & !HOLD_FLAGS) | (QUICK_FLAGS.load(Ordering::Relaxed) & HOLD_FLAGS);
        std::ptr::write_unaligned((sora + ACTOR_LOCK) as *mut u32, lock & !(LOCK_BIT | CANT_ACT_BIT));
        std::ptr::write_unaligned(sora as *mut u32, free);
        log!(
            Cutscenes,
            "Chest Opening: Quick: Sora is back on idle: let go (+0x444 {lock:#x} -> {:#x}, flags {flags:#010x} -> {free:#010x}, +0x88 {:#x}); the rest of the open goes the Skip way",
            lock & !(LOCK_BIT | CANT_ACT_BIT),
            rd::<u32>(sora + 0x88)
        );
    }
}

/// Quick: the backstop at the open's end: a lock (`+0x444` bits 0 / 1) or hold flags still on then are
/// cleared, so he is never left stuck.
unsafe fn quick_release(why: &str) {
    let sora = QUICK_LOCK.swap(0, Ordering::Relaxed);
    if sora < 0x10000 || QUICK_FREE.swap(true, Ordering::Relaxed) {
        return;
    }
    let lock = sora + ACTOR_LOCK;
    let was = rd::<u32>(lock);
    if was & (LOCK_BIT | CANT_ACT_BIT) != 0 {
        std::ptr::write_unaligned(lock as *mut u32, was & !(LOCK_BIT | CANT_ACT_BIT));
        log!(Cutscenes, "Chest Opening: Quick: Sora's lock was still on at {why} (+0x444 {was:#x}): cleared");
    }
    let flags = rd::<u32>(sora);
    let free = (flags & !HOLD_FLAGS) | (QUICK_FLAGS.load(Ordering::Relaxed) & HOLD_FLAGS);
    if free != flags {
        std::ptr::write_unaligned(sora as *mut u32, free);
        log!(Cutscenes, "Chest Opening: Quick: Sora's hold flags were still on at {why} ({flags:#010x}): put back ({free:#010x})");
    }
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
    let find = std::slice::from_raw_parts((base + FIND_ACTOR_STEAM) as *const u8, FIND_ACTOR_PROLOGUE.len());
    if entry != base + CALL_STEAM || done != DONE_PUSH_PROLOGUE || find != FIND_ACTOR_PROLOGUE {
        log!(Cutscenes, "Chest Opening: KH1's script call handler ({entry:#x}) or done queue isn't as expected: the Open stays KH1's");
        return;
    }
    // The table entries first: without them the open would free Sora with KH1's window up.
    if !wrap_messages(base) {
        log!(Menus, "Chest Opening: KH1's message functions not all wrapped: the Open stays KH1's");
        return;
    }
    // The open motion's request (t768): its call in the handler goes to `open_request`.
    if hook::redirect_call(base + OPEN_REQUEST_CALL_STEAM, base + REQUEST_STEAM, open_request as *const () as usize, "Open's motion request (chest)") {
        REQUEST_REDIRECTED.store(true, Ordering::Relaxed);
    } else {
        log!(Cutscenes, "Chest Opening: the Open handler's motion request call isn't as expected: Sora plays KH1's open motion");
    }
    if !hook::redirect_call(base + ACTION_REQUEST_CALL_STEAM, base + REQUEST_STEAM, action_request as *const () as usize, "action step's motion request (chest)") {
        log!(Cutscenes, "Chest Opening: KH1's action step's motion request call isn't as expected: no Keyblade trail on a box's KH2 chest motion");
    }
    if hook::install(base + TOUCH_STEAM, &TOUCH_PROLOGUE, chest_touch as *const () as usize, &ORIG_TOUCH, "chest's touch callback (chest)").is_none() {
        log!(Cutscenes, "Chest Opening: the chest's touch callback isn't as expected: no KH2 facing rule");
    }
    if !hook::redirect_call(base + CHEST_FIRST_CALL_STEAM, base + CHEST_FIRST_STEAM, chest_first as *const () as usize, "chest's first frame (chest)") {
        log!(Cutscenes, "Chest Opening: the chest's first-frame call isn't as expected: chests keep KH1's reach");
    }
    if !hook::redirect_call(base + OPEN_STOP_CALL_STEAM, base + STOP_STEAM, open_stop as *const () as usize, "Open's stop (chest)") {
        log!(Cutscenes, "Chest Opening: the Open handler's stop call isn't as expected: Skip stops Sora for the open");
    }
    if hook::install(base + OPEN_STEAM, &OPEN_PROLOGUE, open as *const () as usize, &ORIG_OPEN, "KH1's Open command (chest)").is_none() {
        log!(Menus, "Chest Opening: KH1's Open handler not hooked: chests stay KH1's");
    }
    // The room scripts' boxes (t769): KH1's command pick.
    if hook::install(base + PICK_STEAM, &PICK_PROLOGUE, pick as *const () as usize, &ORIG_PICK, "KH1's script command pick (chest)").is_none() {
        log!(Menus, "Chest Opening: KH1's script command pick not hooked: the room scripts' boxes stay KH1's");
    }
}

/// KH1's Open handler, then KH2's fast open for a plain chest.
unsafe extern "system" fn open(sora: usize, chest: usize) {
    let orig: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_OPEN.load(Ordering::Acquire));
    let (flags_before, applied_before) = if sora >= 0x10000 { (rd::<u32>(sora), rd::<u32>(sora + ACTOR_APPLIED)) } else { (0, 0) };
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
    if style() == QUICK {
        // Quick: KH1's lock stays on through the open motion; the script asks for his idle, and he is let
        // go there (`quick_check`, t773), not when the script lets him go.
        quick_start(sora, flags_before, applied_before);
    } else {
        std::ptr::write_unaligned(lock as *mut u32, rd::<u32>(lock) & !LOCK_BIT);
    }
    SORA.store(sora, Ordering::Relaxed);
    MODE.store(MODE_TREASURE, Ordering::Relaxed);
    FAST_AT.store(ms() + 1, Ordering::Relaxed);
    SKIPPED.store(0, Ordering::Relaxed);
    RELEASED.store(0, Ordering::Relaxed);
    HOLDS.store(0, Ordering::Relaxed);
    // KH1's treasure script holds Sora through actions on him (the tester's t725 trace: his actor
    // flags gain 0x1000000 (action 0x24), 0x200000 (0x26) and 0x40 (0x2B) on the frame after the Open,
    // and lose them 132 frames later; he couldn't walk meanwhile): our call handler goes into KH1's
    // script machine for the open's time and gives the script's actions on Sora KH1's "no actor" path.
    call_handler_in(base);
    // No open motion (the tester, answer 3 (b), as KH2: KH2's Open, command 0x1A5, is a message to the
    // box, execute 421 / action 0 in 03system's cmd table, and the box plays its own lid motion 0x98;
    // KH2's player has no open action): the handler's own call for it went to `open_request` (t768).
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
    // (t776, the owner: the box KH1 itself shows, as the evidence boxes' (KH1's message window 7, opened by
    // the treasure script with its own lines); not the mod's notice. Quick: not before vanilla's frame, 100
    // frames into the open (t775, the owner), counted in 60 Hz frames from the Open.)
    let due = if style() == QUICK { super::takeover::TICKS.load(Ordering::Relaxed) + QUICK_NOTICE_FRAMES } else { 0 };
    WINDOW_DUE.store(due, Ordering::Relaxed);
    log!(Cutscenes, "Chest Opening: {} open (kind {kind}, contents {what:#x}, \"{line}\"): KH1's own window from the treasure script{}", style_name(style()), if due != 0 { ", not before 100 frames in" } else { "" });
}

/// The Open handler's request for Sora's open motion (`0x1402B59CC`'s call of `0x14029F980`, t768). On a
/// fast open (KH2, a kind the KH2 way, the stock not full: the handler has already written the open's
/// arguments by then, Checked (decomp)) nothing is asked, so he goes on with the motion he has; else
/// KH1's own request. The handler doesn't use what the request returns (Checked (exe): the next
/// instruction loads `rcx` again).
unsafe extern "system" fn open_request(actor: usize, request: u32, blend: f32) -> usize {
    let base = base();
    if style() == SKIP && request == OPEN_REQUEST && fast_kind(base) {
        log!(Cutscenes, "Chest Opening: KH1's open motion not asked for (Skip: Sora keeps his motion)");
        return actor + ACTOR_REQUEST;
    }
    if style() == QUICK && request == OPEN_REQUEST && fast_kind(base) {
        // (t783: on the Chest Animation row at KH2, KH1's open request plays KH2's chest motion, its slot of
        // ours (`kh2slots::CHEST_REQUEST`); else KH1's own.)
        let ours = super::kh2slots::has(super::kh2slots::CHEST_REQUEST, super::pose::CHEST_MOTION);
        log!(
            Cutscenes,
            "Chest Opening: Quick: {}",
            match (anim_kh2(), ours) {
                (true, true) => "KH2's chest motion (on KH1's open request)",
                (true, false) => "KH1's open motion (KH2's chest motion isn't built: KH1 motion slots off, or the box's files missing)",
                _ => "KH1's open motion",
            }
        );
    }
    let f: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + REQUEST_STEAM);
    let r = f(actor, request, blend);
    if style() == QUICK && request == OPEN_REQUEST && fast_kind(base) && kh2_animation() && super::kh2slots::has(super::kh2slots::CHEST_REQUEST, super::pose::CHEST_MOTION) {
        chest_trail(base, actor);
    }
    r
}

/// (t807) A motion request from KH1's action step (a script's action 0x11), KH1's own; then, for a room
/// script's box on Quick (the owner: "the keyblade trail doesn't play on alice's evidence boxes"), when the
/// request is the open (`kh2slots::CHEST_REQUEST`, which plays KH2's chest motion on the Chest Animation row
/// at KH2), KH1's trail request as for a chest (`chest_trail`).
unsafe extern "system" fn action_request(actor: usize, request: u32, blend: f32) -> usize {
    let base = base();
    let f: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + REQUEST_STEAM);
    let r = f(actor, request, blend);
    if MODE.load(Ordering::Relaxed) == MODE_BOX && actor != 0 && actor == SORA.load(Ordering::Relaxed) {
        let chest = request as usize & 0xFFF == super::kh2slots::CHEST_REQUEST;
        let kh2 = kh2_animation() && super::kh2slots::has(super::kh2slots::CHEST_REQUEST, super::pose::CHEST_MOTION);
        log!(Cutscenes, "Chest Opening: the box's script asks Sora's motion {request:#x}{}", if chest && kh2 { ": KH2's chest motion, its Keyblade trail asked for" } else { "" });
        if chest && kh2 {
            chest_trail(base, actor);
        }
    }
    r
}

/// (t803) The Keyblade's trail on KH2's chest motion: KH2's Keyblade motion (`0011`) has a weapon trail over
/// frames 0..23 (range id 12), Checked (files); our slot carries it as KH1's trail level 7.0 over 0..23
/// (`kh2slots::kh1_tracks`). KH1 makes a trail only from a request in the weapon object, which only an attack
/// row's start writes (`0x1402A3980` -> `0x1402C2310 (actor, trail id)`), Checked (decomp,
/// `docs/kh1-cosmetics.md`); the chest open has no attack row, so the window drew nothing. KH1's own request is
/// asked here with the trail id of KH1's plain ground swing row (13, `combat::swing_trail_id`, the weapon's own
/// look), and KH1's trail step makes and ends the trail over the window as for a swing.
unsafe fn chest_trail(base: usize, sora: usize) {
    let weapon = super::pose::ptr(rd::<u32>(sora + ACTOR_WEAPON));
    if weapon < 0x10000 || !super::crash::readable(weapon + 0x40) {
        return;
    }
    let Some(id) = super::combat::swing_trail_id(sora) else {
        log!(Cutscenes, "Chest Opening: KH2's chest motion: no trail id (row 13), no trail");
        return;
    };
    let f: unsafe extern "system" fn(usize, u32) = std::mem::transmute(base + TRAIL_REQUEST_STEAM);
    f(sora, id);
    log!(Cutscenes, "Chest Opening: KH2's chest motion: Keyblade trail {id:#x} asked for (frames 0..23)");
}

/// The Open handler's stop (t778, the owner: open a chest from the run on Quick and Skip, like KH2). KH1
/// already takes the Open while Sora runs (the command runs in his states 0, 6 and 0x18, his run being state 0,
/// `0x1402A3768`, and the chest offers it in state 0, `0x1402B5A30`, Checked (exe + traces: running is state 0,
/// motion 2, slot 6)); what ends the run is this stop, his speed set to 0. On Skip, a fast open keeps his
/// speed and flags: he runs on. Quick plays the open motion, so it stops him as KH1 does.
unsafe extern "system" fn open_stop(actor: usize, speed: f32) {
    if style() == SKIP && fast_kind(base()) {
        log!(Cutscenes, "Chest Opening: Skip: Sora keeps his speed through the open");
        return;
    }
    let f: unsafe extern "system" fn(usize, f32) = std::mem::transmute(base() + STOP_STEAM);
    f(actor, speed)
}

/// A chest's first frame (t779, the owner: "in kh2 you can run by the chest and spam triangle and it'll
/// open"). KH1 offers Open from its per-frame scan (`0x1402948F0`) only when the chest is in range by its
/// record and in front of Sora (cos > 0), or when he walks into it (his touched actor `+0x98`); a chest he
/// runs past is beside or behind him. KH2 offers its Open whenever the box's reaction area touches him,
/// with no facing check. On Quick / Skip the chest's record is given KH2's reach: the flat-and-height shape
/// (no facing) and radius 185. Read by KH1's own scan; nothing of ours runs per frame. (A chest set up
/// while Vanilla keeps KH1's record; one set up on Quick / Skip keeps KH2's until the room reloads.)
unsafe extern "system" fn chest_first(actor: usize, b: u32) -> usize {
    if kh2() && actor >= 0x10000 {
        let rec = super::pose::ptr(rd::<u32>(actor + 0x130));
        if rec >= 0x10000 && rd::<u8>(rec + REC_KIND) == 3 {
            let (r, sh) = (rd::<f32>(rec + REC_RADIUS), rd::<u8>(rec + REC_SHAPE));
            std::ptr::write_unaligned((rec + REC_RADIUS) as *mut f32, r.max(KH2_REACH));
            std::ptr::write_unaligned((rec + REC_SHAPE) as *mut u8, SHAPE_FLAT_HEIGHT);
            log!(Cutscenes, "Chest Opening: chest {actor:#x}: KH2's reach (offer radius {r:.0} -> {:.0}, shape {sh} -> 1: no facing)", r.max(KH2_REACH));
        } else if rec >= 0x10000 {
            log!(Cutscenes, "Chest Opening: chest {actor:#x}: its record's kind is {} (not 3): KH1's reach kept", rd::<u8>(rec + REC_KIND));
        }
    }
    let f: unsafe extern "system" fn(usize, u32) -> usize = std::mem::transmute(base() + CHEST_FIRST_STEAM);
    f(actor, b)
}

/// The chest's touch callback (t780): on Quick / Skip, KH2's two facing rules for the box's Open before
/// KH1's own offer (its state and height checks stay): the chest no more than 135 degrees off Sora's facing,
/// and Sora on the chest's front side. Facings are the actors' direction vectors (`+0x30`, t782); Assumption:
/// a chest's front is that direction. Called by KH1 only when its scan asks this chest.
unsafe extern "system" fn chest_touch(chest: usize, sora: usize, code: i32) -> u64 {
    let orig: unsafe extern "system" fn(usize, usize, i32) -> u64 = std::mem::transmute(ORIG_TOUCH.load(Ordering::Acquire));
    if code != -1 || !kh2() || chest < 0x10000 || sora < 0x10000 {
        return orig(chest, sora, code);
    }
    let base = base();
    let (c, p) = (rd::<[f32; 3]>(chest + 0x10), rd::<[f32; 3]>(sora + 0x10));
    let (dx, dz) = (c[0] - p[0], c[2] - p[2]);
    let len = (dx * dx + dz * dz).sqrt().max(1e-3);
    let (dx, dz) = (dx / len, dz / len);
    // (t782: an actor's facing as KH1's own range test reads it, the direction at `+0x30` (`0x140293FC0`
    // dots Sora's with the way to the actor, Checked (exe)); t780 read the yaw `+0x3C`, which is 0 for every
    // chest (the owner's t781 log), so the front rule was the same for all three throne room chests.)
    let flat = |v: [f32; 3]| {
        let l = (v[0] * v[0] + v[2] * v[2]).sqrt();
        (l > 0.1).then(|| (v[0] / l, v[2] / l))
    };
    let (sf, cf) = (flat(rd::<[f32; 3]>(sora + 0x30)), flat(rd::<[f32; 3]>(chest + 0x30)));
    let facing = sf.map_or(1.0, |(x, z)| dx * x + dz * z);
    let front = cf.map_or(1.0, |(x, z)| -(dx * x + dz * z));
    let (sy, cy) = (rd::<f32>(sora + 0x3C), rd::<f32>(chest + 0x3C));
    // (t808) A room script's box still running its part (its window up, its 512 not reached): KH1 couldn't
    // start the treasure script for a chest opened then (the owner's t805 log: the chest's Open at f3849
    // during the evidence box's window, f3748..f3893, gave no lid, no item, and the chest was never offered
    // again). In KH1 the box holds Sora until its 512, so the two never met; the Open is greyed until then.
    let busy = box_running();
    let grey = busy || len > 1.0 && (facing < KH2_OPEN_FACING || front < KH2_OPEN_FRONT);
    let r = orig(chest, sora, code);
    let offered = rd::<u8>(base + MENU_SLOTS_STEAM + 3) == OPEN_COMMAND;
    if grey && offered {
        // Greyed, as KH2's (shown, not usable): KH1's own offer, then its slot 3 greyed and blocked
        // (bit 3 of `0x140528904` / `0x140528908`, as KH1's scan greys a Talk, Checked (exe)).
        for at in [GREYED_STEAM, BLOCKED_STEAM] {
            std::ptr::write_unaligned((base + at) as *mut u32, rd::<u32>(base + at) | 8);
        }
    }
    // (t781, diagnosis: the owner saw no prompt at all; at most one line a second.)
    static LAST: AtomicU64 = AtomicU64::new(0);
    let now = ms();
    if now.saturating_sub(LAST.load(Ordering::Relaxed)) >= 1000 {
        LAST.store(now, Ordering::Relaxed);
        log!(
            Cutscenes,
            "Chest Opening: asked chest {chest:#x} at ({:.0},{:.0},{:.0}) yaw {cy:.2} facing dir {:?}: Sora at ({:.0},{:.0},{:.0}) yaw {sy:.2} facing dir {:?}, {len:.0} away, facing {facing:.2}, front {front:.2}, state {}; KH1 offered Open: {offered}{}",
            c[0], c[1], c[2], cf, p[0], p[1], p[2], sf, rd::<i32>(sora + 0x70), if busy && offered { " (greyed: a box's part still runs)" } else if grey && offered { " (greyed: KH2's facing rule)" } else { "" }
        );
    }
    r
}

/// (t808) A room script's box opened the KH2 way is still running its part (until its 512, or the open's time).
fn box_running() -> bool {
    let at = FAST_AT.load(Ordering::Relaxed);
    MODE.load(Ordering::Relaxed) == MODE_BOX && at != 0 && ms() + 1 - at <= FAST_MS
}

/// The open in progress goes the KH2 way: kinds 0..3 (an item, a gummi block, Dalmatians, munny) with
/// the stock not full (the handler's arguments, `0x142DB7B30` / `0x142DB7B38`).
unsafe fn fast_kind(base: usize) -> bool {
    let kind = rd::<u32>(base + ARG_KIND_STEAM);
    let full = rd::<u32>(base + ARG_FULL_STEAM);
    full == 0 && matches!(kind, 0..=3)
}

/// KH1's command pick, then, for a box command on KH2, KH2's quick open of the room script's box: its
/// part is read from the context KH1 just woke; a box (it takes the player, gives, gives him back, with
/// no scene, choice, shop or menu) runs whole as KH1 wrote it, with its holds, actions on Sora and
/// windows taken as a chest's are (t769).
unsafe extern "system" fn pick(object: usize, b: usize, command: i32) -> u32 {
    let orig: unsafe extern "system" fn(usize, usize, i32) -> u32 = std::mem::transmute(ORIG_PICK.load(Ordering::Acquire));
    if !kh2() || !BOX_COMMANDS.contains(&command) || object < 0x10000 {
        return orig(object, b, command);
    }
    let base = base();
    let (list, count_at) = if rd::<i32>(base + ENTRY_STEAM) == 0 { (base + ROOM_CTXS_STEAM, base + ROOM_COUNT_STEAM) } else { (base + EVENT_CTXS_STEAM, base + EVENT_COUNT_STEAM) };
    let r = orig(object, b, command);
    let count = rd::<i32>(count_at).clamp(0, CTX_MAX as i32) as usize;
    let id = rd::<i32>(object + 4);
    let Some(c) = (0..count).find(|&c| rd::<i32>(list + c * CTX_SIZE) == id) else { return r };
    let ctx = list + c * CTX_SIZE;
    let subs = rd::<i32>(ctx + CTX_SUBS).clamp(0, 64) as usize;
    if subs <= HANDLER_SUB {
        return r;
    }
    let start = rd::<i32>(ctx + CTX_SUB_STARTS + HANDLER_SUB * 4);
    // KH1 woke the handler (its next instruction is the sub-script's start).
    if start < 0 || rd::<i32>(ctx + CTX_PC) != start {
        return r;
    }
    let word = |c: usize, i: usize| -> Option<u32> {
        if c >= count {
            return None;
        }
        let code = rd::<u64>(list + c * CTX_SIZE + CTX_CODE) as usize;
        let at = code.checked_add(i.checked_mul(4)?)?;
        (code >= 0x10000 && super::crash::readable(at) && super::crash::readable(at + 3)).then(|| rd::<u32>(at))
    };
    let sub = |c: usize, s: usize| -> Option<usize> {
        if c >= count || s >= rd::<i32>(list + c * CTX_SIZE + CTX_SUBS).clamp(0, 64) as usize {
            return None;
        }
        usize::try_from(rd::<i32>(list + c * CTX_SIZE + CTX_SUB_STARTS + s * 4)).ok()
    };
    match read_box(c, start as usize, command as u32, word, sub, &BOX_PLAYS) {
        BoxOpen::Give(n) => {
            let sora = rd::<u64>(base + SORA_PTR_STEAM) as usize;
            if sora < 0x10000 {
                return r;
            }
            SORA.store(sora, Ordering::Relaxed);
            if style() == QUICK {
                // Quick: the box's own part holds him and plays his motion (its actions on him are KH1's);
                // he is let go on idle after it (t773), the backstop at its 512.
                quick_start(sora, rd::<u32>(sora), rd::<u32>(sora + ACTOR_APPLIED));
            }
            BOX_CTXS.store(list, Ordering::Relaxed);
            BOX_COUNT.store(count_at, Ordering::Relaxed);
            MODE.store(MODE_BOX, Ordering::Relaxed);
            WINDOW_DUE.store(0, Ordering::Relaxed);
            FAST_AT.store(ms() + 1, Ordering::Relaxed);
            SKIPPED.store(0, Ordering::Relaxed);
            RELEASED.store(0, Ordering::Relaxed);
            HOLDS.store(0, Ordering::Relaxed);
            call_handler_in(base);
            log!(Cutscenes, "Chest Opening: KH2 open of a room script's box (command {command:#x}, context {c}, {n} instructions read): its holds and windows skipped, Sora free");
        }
        BoxOpen::Not(f) => log!(Cutscenes, "Chest Opening: command {command:#x}'s part (context {c}) isn't a box (event function {f}): KH1's"),
        BoxOpen::Unread => log!(Cutscenes, "Chest Opening: command {command:#x}'s part (context {c}) couldn't be read: KH1's"),
    }
    r
}

/// The notice for a box's give (event function 258 `push item, push count`, or 583 `push record`),
/// read from the context's stack before KH1's function runs (Checked (exe): 258 takes the item one below
/// the top, the count on top; 583 the record on top).
unsafe fn box_notice(base: usize, ctx: usize, n: u32) -> Option<String> {
    let sp = rd::<i32>(ctx + CTX_SP);
    if !(1..64).contains(&sp) {
        return None;
    }
    let top = rd::<u32>(ctx + CTX_SP + 4 + sp as usize * 4);
    let below = rd::<u32>(ctx + CTX_SP + sp as usize * 4);
    let (item, count) = if n == FN_ADD_ITEM {
        (Some(below), top)
    } else {
        let table = rd::<u64>(base + REWARDS_PTR_STEAM) as usize;
        let rec = if table >= 0x10000 && top < 0x1000 { rd::<u16>(table + top as usize * 2) } else { 0xFFFF };
        ((rec & 0xF == 0).then_some((rec >> 8) as u32), 1)
    };
    let line = match item {
        Some(i) if i < 0x100 => {
            let name = super::equip::item_name(base, i as u8).map(|n| strip_codes(&n)).unwrap_or_else(|| format!("Item {i:#x}"));
            if count > 1 {
                format!("{name} x{count}")
            } else {
                name
            }
        }
        _ => "Reward".to_string(),
    };
    log!(Cutscenes, "Chest Opening: the box gives (event function {n}): \"{line}\" (KH1's own window from the box's script)");
    Some(line)
}

/// The item name without the codes `item_name` writes as `{xx}`.
pub(super) fn strip_codes(s: &str) -> String {
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
    if at == 0 || !mode_on() {
        return false;
    }
    if ms() + 1 - at > FAST_MS {
        quick_release("the open's time is over");
        FAST_AT.store(0, Ordering::Relaxed);
        MODE.store(MODE_NONE, Ordering::Relaxed);
        log!(
            Cutscenes,
            "Chest Opening: the open's time is over ({} message call(s) skipped, {} action(s) on Sora done at once / motion waits passed)",
            SKIPPED.load(Ordering::Relaxed),
            RELEASED.load(Ordering::Relaxed)
        );
        call_handler_out(base());
        return false;
    }
    open_ctx(base(), ctx)
}

/// (t809) The pickup's context has left its sub-script (its next instruction outside it) for
/// `PICKUP_AFTER_MS`.
unsafe fn pickup_over() -> bool {
    let ctx = PICKUP_CTX.load(Ordering::Relaxed);
    if ctx < 0x10000 {
        return true;
    }
    let left = PICKUP_LEFT.load(Ordering::Relaxed);
    if PICKUP_POLLED.load(Ordering::Relaxed) {
        // (t810) Over a short while after its last give-back, or when nothing was given in time.
        if !PICKUP_GIVEN.load(Ordering::Relaxed) && ms() + 1 - FAST_AT.load(Ordering::Relaxed) > PICKUP_POLL_MS {
            return true;
        }
        return left != 0 && ms() + 1 - left > PICKUP_AFTER_MS;
    }
    // (t815) Inside while any of the part's contexts is in one of its sub-scripts.
    let inside = PICKUP_RANGES
        .lock()
        .map(|r| {
            r.iter().any(|&(c, f, e)| {
                let pc = rd::<i32>(c + CTX_PC);
                pc >= 0 && (f..=e).contains(&(pc as usize))
            })
        })
        .unwrap_or(false);
    if inside {
        PICKUP_LEFT.store(0, Ordering::Relaxed);
        return false;
    }
    if left == 0 {
        PICKUP_LEFT.store(ms() + 1, Ordering::Relaxed);
        return false;
    }
    ms() + 1 - left > PICKUP_AFTER_MS
}

/// (t815) `ctx` is one the pickup's part runs in (its own, or one it woke).
fn pickup_ctx(ctx: usize) -> bool {
    ctx == PICKUP_CTX.load(Ordering::Relaxed) || PICKUP_RANGES.lock().map(|r| r.iter().any(|&(c, _, _)| c == ctx)).unwrap_or(false)
}

/// The open in progress is still wanted: a chest's or a box's on Quick / Skip, a pickup's on Item
/// Pickup's Skip (t795).
fn mode_on() -> bool {
    if MODE.load(Ordering::Relaxed) == MODE_PICKUP {
        pickup()
    } else {
        kh2()
    }
}

/// A context of the script the open runs in: the treasure script's, or (a box of a room's script, t769;
/// a pickup, t795) one of the context list the box's handler is in.
unsafe fn open_ctx(base: usize, ctx: usize) -> bool {
    if matches!(MODE.load(Ordering::Relaxed), MODE_BOX | MODE_PICKUP) {
        let first = BOX_CTXS.load(Ordering::Relaxed);
        let count = (rd::<i32>(BOX_COUNT.load(Ordering::Relaxed)).clamp(0, CTX_MAX as i32)) as usize;
        return first != 0 && ctx >= first && ctx < first + count * CTX_SIZE && (ctx - first) % CTX_SIZE == 0;
    }
    treasure_ctx(base, ctx)
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
            quick_release("the open's time is over");
            FAST_AT.store(0, Ordering::Relaxed);
            MODE.store(MODE_NONE, Ordering::Relaxed);
            log!(
                Cutscenes,
                "Chest Opening: the open's time is over ({} message call(s) skipped, {} action(s) on Sora done at once / motion waits passed)",
                SKIPPED.load(Ordering::Relaxed),
                RELEASED.load(Ordering::Relaxed)
            );
        }
        puppy_restore(base);
        timed_restore(base);
        call_handler_out(base);
        return orig(ctx, n);
    }
    quick_check();
    if MODE.load(Ordering::Relaxed) == MODE_PICKUP && pickup_over() {
        FAST_AT.store(0, Ordering::Relaxed);
        MODE.store(MODE_NONE, Ordering::Relaxed);
        log!(
            Cutscenes,
            "Item Pickup: the pickup's part has ended ({} window call(s) skipped, {} action(s) on Sora done at once / motion waits passed)",
            SKIPPED.load(Ordering::Relaxed),
            RELEASED.load(Ordering::Relaxed)
        );
        return orig(ctx, n);
    }
    let sora = SORA.load(Ordering::Relaxed);
    if sora == 0 || !open_ctx(base, ctx) {
        return orig(ctx, n);
    }
    let picked = MODE.load(Ordering::Relaxed) == MODE_PICKUP;
    let boxed = MODE.load(Ordering::Relaxed) == MODE_BOX || picked;
    // (t811, the owner: the Seagull Egg and the Coconut "doesn't disappear when being picked up, only after you
    // hit a button does it"; t810 log: the Egg's part waited on window 7 (107 x 280 from f4028) until the jump at
    // f4174, because window 7 is the in-game skip's "Obtained" box, whose line waits for Confirm, and the item
    // goes away after that wait.) From the pickup's own context, KH1's window functions on window 7 are taken the
    // skip-mode way too: their arguments popped, 2 returned (Checked (exe): 2 and 107 pop 1, 3 and 4 pop 3, 5, 6,
    // 80 and 83 pop 2, 365 pops 4; each returns 2, 107 returns 4 while it waits). So the part neither waits on
    // nor closes nor moves the in-game skip's box.
    if picked && pickup_ctx(ctx) {
        if let Some(&(_, pops)) = PICKUP_WINDOW_FNS.iter().find(|&&(f, _)| f == n) {
            let sp = rd::<i32>(ctx + CTX_SP);
            // The window is the call's first argument (the deepest of its values).
            // (Its top is at `+0x198 + sp * 4`: the deepest of `pops` values is at index `sp - pops + 1`.)
            let window = if sp - pops + 1 >= 0 && sp < 64 { rd::<i32>(ctx + 0x198 + (sp - pops + 1) as usize * 4) } else { -1 };
            if window == crate::kh1script::WINDOW_OBTAINED as i32 {
                std::ptr::write_unaligned((ctx + CTX_SP) as *mut i32, sp - pops);
                if SKIPPED.fetch_add(1, Ordering::Relaxed) < 12 {
                    log!(Cutscenes, "Item Pickup: the pickup's window call (event function {n}) skipped");
                }
                return 2;
            }
        }
    }
    if !boxed && n == FN_PLACE_TEXT {
        puppy_text(base, ctx);
    }
    if !picked && n == FN_CLOSE_WINDOW {
        let r = orig(ctx, n);
        puppy_restore(base);
        timed_restore(base);
        return r;
    }
    if boxed && (n as u32 == FN_ADD_ITEM || n as u32 == FN_GIVE_REWARD) {
        let line = box_notice(base, ctx, n as u32);
        // (t806) A pickup's give shows in the in-game skip's "Obtained" window (its own window is skipped).
        if picked && pickup_ctx(ctx) {
            PICKUP_GIVEN.store(true, Ordering::Relaxed);
            if let Some(line) = line {
                super::ingameskip::queue_obtained(line);
            }
        }
    }
    // (t802) KH2's chest motion: the lid opens at its contact frame, and Sora finishes the motion first.
    let kh2_motion = !boxed && quick_holding() && kh2_animation() && rd::<u64>(ctx + CTX_ACTOR) as usize == sora && super::kh2slots::playing_motion(sora) == Some(super::pose::CHEST_MOTION);
    if kh2_motion && n as usize == FN_WAIT_MOTION {
        // KH1 opens the lid when this wait on Sora's motion ends (the script's context 0 then starts the
        // lid's part, context 2's sub-script 11, and his idle, context 1's 12). KH2 starts its lid with the
        // capture and its contact is frame 28: the Keyblade's 500034 and the box's own sound 22 both play
        // there. So the wait ends at frame 28 of KH2's motion.
        let t = super::kh2slots::kh1_time_now(sora).unwrap_or(0.0);
        if t >= KH2_CONTACT_FRAME {
            if !CONTACT_LOGGED.swap(true, Ordering::Relaxed) {
                // (t803) The item window comes as long after the lid as vanilla's does: `LID_TO_WINDOW`.
                let due = super::takeover::TICKS.load(Ordering::Relaxed) + LID_TO_WINDOW;
                if WINDOW_DUE.load(Ordering::Relaxed) != 0 {
                    WINDOW_DUE.store(due, Ordering::Relaxed);
                }
                log!(Cutscenes, "Chest Opening: KH2's chest motion at frame {t:.0}: the lid opens (KH2's contact frame {KH2_CONTACT_FRAME:.0}); the item window {LID_TO_WINDOW} frames after it");
            }
            return 2;
        }
        return orig(ctx, n);
    }
    if kh2_motion && n == FN_MOTION_REQUEST {
        // ...and the script's idle request after it (event function 58) waits, as KH1's own waits do (4: run
        // again next frame), until KH2's motion has played out.
        let frames = super::kh2slots::frames_for_motion(super::pose::CHEST_MOTION).unwrap_or(0) as f32;
        if super::kh2slots::kh1_time_now(sora).unwrap_or(f32::MAX) < frames - 1.0 {
            return 4;
        }
        return orig(ctx, n);
    }
    if n as usize == FN_WAIT_MOTION && rd::<u64>(ctx + CTX_ACTOR) as usize == sora {
        // Quick, until he is let go: KH1's own wait, so the script goes on with the motion's end (t770).
        if quick_holding() {
            return orig(ctx, n);
        }
        note_release(&format!("its wait on Sora's motion (function 103, context {ctx:#x}) goes on"));
        return 2;
    }
    if HOLD_FNS.contains(&n) {
        // (t795) A pickup's holds are skipped only from its own context: the room's other contexts keep theirs.
        if picked && !pickup_ctx(ctx) {
            return orig(ctx, n);
        }
        // (t810) A polled pickup's give-backs after its give count down to its end.
        if picked && PICKUP_POLLED.load(Ordering::Relaxed) && PICKUP_GIVEN.load(Ordering::Relaxed) && n as u32 == PICKUP_RELEASE_FN.load(Ordering::Relaxed) {
            if PICKUP_RELEASES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v.saturating_sub(1))).unwrap_or(0) <= 1 && PICKUP_LEFT.load(Ordering::Relaxed) == 0 {
                PICKUP_LEFT.store(ms() + 1, Ordering::Relaxed);
            }
        }
        if HOLDS.fetch_add(1, Ordering::Relaxed) < 8 {
            log!(Cutscenes, "Chest Opening: {}'s player hold (event function {n}) skipped", if picked { "the pickup's script" } else if boxed { "the box's script" } else { "treasure script" });
        }
        // A box's part ends at its 512 (the player given back): the open is over (t769). (t809) A pickup's
        // ends when its sub-script does (`pickup_over`): some hold and give back twice (the rope), some
        // only with 113 / 114.
        if boxed && !picked && n as u32 == FN_RELEASE {
            quick_release("the box's part has ended");
            FAST_AT.store(0, Ordering::Relaxed);
            MODE.store(MODE_NONE, Ordering::Relaxed);
            log!(
                Cutscenes,
                "Chest Opening: the box's part has ended ({} message call(s) skipped, {} action(s) on Sora done at once / motion waits passed)",
                SKIPPED.load(Ordering::Relaxed),
                RELEASED.load(Ordering::Relaxed)
            );
        }
        return 2;
    }
    let write = base + ACTIONS_WRITE_STEAM;
    let before = rd::<i32>(write);
    let r = orig(ctx, n);
    let after = rd::<i32>(write);
    // (Quick, until he is let go on idle (t773): the script's actions on Sora are KH1's: its lock, his open
    // motion, its idle request after the motion (event function 58: action 0x11, request 0). After that its
    // late unlocks (56, 54, 335, 337) and anything else on him are done at once, as on Skip.)
    if after != before && !quick_holding() {
        to_done(base, before, after, sora, n);
    }
    r
}

/// The actions queued from `from` to `to` (ring indices): those on Sora (the actor KH1's own lookup finds
/// for the record's id) go to KH1's done queue, the others are kept in order.
unsafe fn to_done(base: usize, from: i32, to: i32, sora: usize, n: i32) {
    let find: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base + FIND_ACTOR_STEAM);
    let ring = base + ACTIONS_STEAM;
    let slot = |i: i32| ring + (i as usize & 0xFF) * ACTION_SIZE;
    let mut kept: Vec<Action> = Vec::new();
    let mut moved: Vec<Action> = Vec::new();
    let mut i = from;
    let mut guard = 0;
    while i != to && guard < 256 {
        let a = std::ptr::read_unaligned(slot(i) as *const Action);
        if find(u32::from_le_bytes([a.0[4], a.0[5], a.0[6], a.0[7]])) == sora {
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

/// (t776) The open's message windows are KH1's own (window 7, as the evidence boxes show it); on Quick a
/// window's open (functions 0 / 177) waits, with KH1's own "wait" return (4: the instruction runs again
/// next frame, as 107 does, Checked (exe)), until vanilla's frame (`WINDOW_DUE`).
unsafe fn call(ctx: usize, which: usize) -> u64 {
    // (t806, the owner: the pickup's box "needs to be the same size as our in game cutscene skip text boxes so
    // it doesn't obstruct gameplay") A pickup's own window (KH1's centred window 7) isn't opened: from the
    // pickup's context, KH1's message functions take their skip-mode way (their arguments popped, nothing
    // shown: Checked (decomp), 0 and 177 pop 1, 1 and 605 pop 2, and return 2), its waits on the window (107)
    // pass and its close (2) closes nothing, as in KH1's skip mode. What it gave shows in the in-game skip's
    // "Obtained" window instead (`script_call`).
    if MODE.load(Ordering::Relaxed) == MODE_PICKUP && pickup_ctx(ctx) && ours_to_skip(ctx) {
        let pops = MESSAGE_FNS[which].2;
        let sp = rd::<i32>(ctx + CTX_SP);
        // (t816, the owner: "when you pick up any world items the potion icon appears instead of the general item
        // icon that was in vanilla") The line the part would show (message function 1: window one below the top,
        // text on top) carries the icon vanilla shows (`{0C 04}` then the icon); it goes to the box's line.
        if which == 1 && (1..64).contains(&sp) && rd::<i32>(ctx + 0x198 + (sp - 1) as usize * 4) == crate::kh1script::WINDOW_OBTAINED as i32 {
            let text = rd::<i32>(ctx + 0x198 + sp as usize * 4);
            match line_icon(base(), text) {
                Some(icon) => super::ingameskip::set_pickup_icon(icon),
                None => log!(Cutscenes, "Item Pickup: text {text}'s icon not found: the box keeps the item icon"),
            }
        }
        std::ptr::write_unaligned((ctx + CTX_SP) as *mut i32, (sp - pops).max(0));
        if SKIPPED.fetch_add(1, Ordering::Relaxed) < 8 {
            log!(Cutscenes, "Item Pickup: the pickup's window call (message function {}) skipped", MESSAGE_FNS[which].0);
        }
        return 2;
    }
    if matches!(which, 0 | 2) && ours_to_skip(ctx) {
        let due = WINDOW_DUE.load(Ordering::Relaxed);
        if due != 0 {
            if super::takeover::TICKS.load(Ordering::Relaxed) < due {
                return 4;
            }
            WINDOW_DUE.store(0, Ordering::Relaxed);
            SKIPPED.fetch_add(1, Ordering::Relaxed);
            log!(Cutscenes, "Chest Opening: Quick: KH1's window (message function {}) opens 100 frames into the open", MESSAGE_FNS[which].0);
        }
    }
    if which == 1 && MODE.load(Ordering::Relaxed) != MODE_PICKUP && ours_to_skip(ctx) {
        timed_text(base(), ctx);
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

// ---- The Dalmatians' window (t777) ----

/// KH1's message pointers by text id for the script in use (`0x14237C4E0`, 8 bytes each: KH1's script
/// loader fills it from the file's text list, `0x1401A3DA6`, Checked (exe)).
const TEXTS_STEAM: usize = 0x237C4E0;
/// The treasure script's Dalmatians text (`UK_treasure.ev` text 4: "Found dalmatians." and "Puppy no. A,
/// no. B, and no. C returned home!", two lines, centred: 365 places it at (-230, 0)), Checked (files).
const PUPPY_TEXT: i32 = 4;
/// Event function 365 (`0x1401B7C80`): window, text, x, y (it measures the text and places the window),
/// and 2 (`0x1401B8100`): closes a window. Checked (exe).
const FN_PLACE_TEXT: i32 = 365;
const FN_CLOSE_WINDOW: i32 = 2;
/// Where the treasure script places its one-line "Obtained" windows (365's x and y), Checked (files).
const OBTAINED_AT: (i32, i32) = (-220, -150);
/// The text pointer KH1 had for text 4 while ours stands in (0: none).
static PUPPY_WAS: AtomicUsize = AtomicUsize::new(0);
static PUPPY_LINE: OnceLock<usize> = OnceLock::new();

/// Our one line for the Dalmatians, in the "Obtained" texts' own form (the same codes before the words,
/// the same 60-frame close, `{05} 0x3C` since t804): "Found dalmatians no. A, B, C." with KH1's own number codes
/// (`{0E} 0x10..0x12`, the values the script sets with event function 475). Never freed.
fn puppy_line() -> usize {
    *PUPPY_LINE.get_or_init(|| {
        let mut b: Vec<u8> = vec![0x08, 0x0A, 0x00, 0x00, 0x00, 0x07, 0x0C, 0x00];
        let text = |b: &mut Vec<u8>, s: &str| {
            for c in s.chars() {
                b.push(match c {
                    'A'..='Z' => 0x2B + (c as u8 - b'A'),
                    'a'..='z' => 0x45 + (c as u8 - b'a'),
                    '.' => 0x68,
                    ',' => 0x69,
                    _ => 0x01,
                });
            }
        };
        text(&mut b, "Found dalmatians no. ");
        b.extend_from_slice(&[0x0E, 0x10]);
        text(&mut b, ", ");
        b.extend_from_slice(&[0x0E, 0x11]);
        text(&mut b, ", ");
        b.extend_from_slice(&[0x0E, 0x12]);
        text(&mut b, ".");
        // (t804) KH1's own wait that no button ends (`{05}`, see `timed_copy`), the same 60 frames.
        b.extend_from_slice(&[0x05, 0x3C, 0x00, 0x00]);
        Box::leak(b.into_boxed_slice()).as_ptr() as usize
    })
}

/// Quick / Skip (the owner: the puppy window "the same size as our new text box so its not obstructing
/// gameplay"): when the treasure script places text 4, our one line stands in for it and the window goes
/// where its "Obtained" windows go; KH1 measures, opens, numbers and closes it as its own.
unsafe fn puppy_text(base: usize, ctx: usize) {
    let sp = rd::<i32>(ctx + CTX_SP);
    if !(3..0x40).contains(&sp) {
        return;
    }
    let at = |k: usize| ctx + 0x18C + sp as usize * 4 + k * 4;
    if rd::<i32>(at(1)) != PUPPY_TEXT {
        return;
    }
    let slot = base + TEXTS_STEAM + PUPPY_TEXT as usize * 8;
    let was = rd::<u64>(slot) as usize;
    let ours = puppy_line();
    if was != ours {
        PUPPY_WAS.store(was, Ordering::Relaxed);
        std::ptr::write_unaligned(slot as *mut u64, ours as u64);
    }
    std::ptr::write_unaligned(at(2) as *mut i32, OBTAINED_AT.0);
    std::ptr::write_unaligned(at(3) as *mut i32, OBTAINED_AT.1);
    log!(Cutscenes, "Chest Opening: the Dalmatians' window: one line, placed as the \"Obtained\" windows");
}

/// KH1's own text 4 back (the window closed, or the open is over).
unsafe fn puppy_restore(base: usize) {
    let was = PUPPY_WAS.swap(0, Ordering::Relaxed);
    if was == 0 {
        return;
    }
    let slot = base + TEXTS_STEAM + PUPPY_TEXT as usize * 8;
    if rd::<u64>(slot) as usize == puppy_line() {
        std::ptr::write_unaligned(slot as *mut u64, was as u64);
    }
}

// ---- The window closes on its own (t804) ----

/// The texts we stand in for while an open's window shows: (table slot, KH1's pointer, ours).
static TIMED_HELD: Mutex<Vec<(usize, usize, usize)>> = Mutex::new(Vec::new());
static TIMED_ANY: AtomicBool = AtomicBool::new(false);
/// Our copies by KH1's text (its pointer and bytes): made once, never freed (KH1's window keeps the
/// pointer it was given).
static TIMED_COPIES: Mutex<Vec<(usize, Vec<u8>, usize)>> = Mutex::new(Vec::new());
/// The longest text we copy.
const TIMED_MAX: usize = 1024;

/// How many bytes a KH1 text code takes (the code byte included), as KH1's text parser
/// (`0x1401767B0`) steps over it, Checked (decomp). `None`: the end of the text (code 0) or a code we
/// don't copy past (4, which ends the parse there as 0 does).
/// (t816) The icon in the room script's text `text` (the glyph after its `{0C 04}`, the colour code before an
/// item's name), read with KH1's text parser's code lengths (`code_len`).
unsafe fn line_icon(base: usize, text: i32) -> Option<u8> {
    if !(0..0x400).contains(&text) {
        return None;
    }
    let p = rd::<u64>(base + TEXTS_STEAM + text as usize * 8) as usize;
    if p < 0x10000 || !super::crash::readable(p) || !super::crash::readable(p + 0x7F) {
        return None;
    }
    let at = |i: usize| rd::<u8>(p + i);
    let mut i = 0;
    while i + 2 < 0x80 {
        let c = at(i);
        if c == 0x0C && at(i + 1) == 0x04 {
            let icon = at(i + 2);
            return (icon >= 0x20).then_some(icon);
        }
        i += code_len(c, || at(i + 1))?;
    }
    None
}

fn code_len(c: u8, arg: impl Fn() -> u8) -> Option<usize> {
    Some(match c {
        0 | 4 => return None,
        0x20.. => 1,
        1 | 2 | 3 | 8 | 0x10 | 0x11 | 0x13..=0x17 => 1,
        5 | 6 | 7 | 0x12 => 3,
        9 | 0x0C | 0x0E | 0x0F | 0x18..=0x1F => 2,
        0x0A => if arg() == 0 { 4 } else { 2 },
        0x0B => if matches!(arg(), 0 | 1 | 0x10) { 4 } else { 2 },
        0x0D => if matches!(arg(), 0 | 1) { 4 } else { 2 },
    })
}

/// A copy of a KH1 text whose button-skippable waits (`{06} lo hi`) are KH1's own unskippable form
/// (`{05} lo hi`, the same frames). Checked (decomp): the parser stores `{06}`'s frames as the wait's
/// value and `{05}`'s as frames + 1,000,000; the window step (`0x1401744D0`) ends a wait under 1,000,000
/// at once on Confirm (`0x14298CA98` bit 0 or bit 26) and counts one at or over it down the same way
/// (to 1,000,000) whatever is pressed. `None`: no `{06}` in it, or it ends with code 4 / not within
/// `TIMED_MAX` bytes (KH1's text is left as it is). `at(i)` reads the text's byte `i`; only the bytes the
/// parser itself would read are read.
fn timed_copy(at: impl Fn(usize) -> u8) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut changed = false;
    loop {
        let i = out.len();
        if i >= TIMED_MAX {
            return None;
        }
        let c = at(i);
        match code_len(c, || at(i + 1)) {
            None if c == 0 => {
                out.push(0);
                break;
            }
            None => return None,
            Some(n) => {
                out.push(if c == 6 { 5 } else { c });
                changed |= c == 6;
                for k in 1..n {
                    out.push(at(i + k));
                }
            }
        }
    }
    changed.then_some(out)
}

/// Quick / Skip (the owner: "on skip and quick settings your able to close the text box by pressing a
/// button but we just want it to go away on its own"): when an open's script shows its line (message
/// function 1: window, text), the text KH1 reads for it is our copy with KH1's unskippable wait, until
/// the window is closed (function 2) or the open is over. KH1 shows, times and closes it as its own.
unsafe fn timed_text(base: usize, ctx: usize) {
    // (t805) Function 1's two values are the stack's top two: window at `+0x194 + sp * 4`, text at
    // `+0x198 + sp * 4`, so `sp` is 1 when they're all the stack holds (t804 wanted 2 or more and never ran).
    let sp = rd::<i32>(ctx + CTX_SP);
    if !(1..0x40).contains(&sp) {
        log!(Cutscenes, "Chest Opening: the window's line: script stack {sp} unreadable: KH1's (Confirm can close it)");
        return;
    }
    let text = rd::<i32>(ctx + 0x198 + sp as usize * 4);
    if !(0..0x400).contains(&text) {
        log!(Cutscenes, "Chest Opening: the window's line: text {text} out of range: KH1's (Confirm can close it)");
        return;
    }
    let slot = base + TEXTS_STEAM + text as usize * 8;
    let was = rd::<u64>(slot) as usize;
    if was < 0x10000 || was == puppy_line() {
        return;
    }
    let mut held = TIMED_HELD.lock().unwrap_or_else(|e| e.into_inner());
    if held.iter().any(|&(s, _, ours)| s == slot && ours == was) {
        return;
    }
    let byte = |i: usize| rd::<u8>(was + i);
    let mut copies = TIMED_COPIES.lock().unwrap_or_else(|e| e.into_inner());
    let ours = match copies.iter().find(|(p, b, _)| *p == was && b.iter().enumerate().all(|(i, &v)| byte(i) == v)) {
        Some(&(_, _, ours)) => ours,
        None => {
            let Some(copy) = timed_copy(byte) else {
                log!(Cutscenes, "Chest Opening: text {text}'s window has no button-closable wait we can read: KH1's");
                return;
            };
            let orig: Vec<u8> = (0..copy.len()).map(byte).collect();
            let ours = Box::leak(copy.into_boxed_slice()).as_ptr() as usize;
            copies.push((was, orig, ours));
            ours
        }
    };
    std::ptr::write_unaligned(slot as *mut u64, ours as u64);
    held.push((slot, was, ours));
    TIMED_ANY.store(true, Ordering::Relaxed);
    log!(Cutscenes, "Chest Opening: text {text}'s window closes on its own (KH1's unskippable wait, {{05}})");
}

/// KH1's texts back (the window closed, or the open is over).
unsafe fn timed_restore(_base: usize) {
    if !TIMED_ANY.swap(false, Ordering::Relaxed) {
        return;
    }
    let mut held = TIMED_HELD.lock().unwrap_or_else(|e| e.into_inner());
    for (slot, was, ours) in held.drain(..) {
        if rd::<u64>(slot) as usize == ours {
            std::ptr::write_unaligned(slot as *mut u64, was as u64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::strip_codes;
    use super::timed_copy;

    #[test]
    fn timed_copy_makes_the_wait_unskippable() {
        // An "Obtained" line's form: {08} {0A 00 00 00} {07 0C 00} "Ab" {06 3C 00} end.
        let t = [0x08, 0x0A, 0x00, 0x00, 0x00, 0x07, 0x0C, 0x00, 0x2B, 0x46, 0x06, 0x3C, 0x00, 0x00, 0x99];
        let c = timed_copy(|i| t[i]).unwrap();
        assert_eq!(c, [0x08, 0x0A, 0x00, 0x00, 0x00, 0x07, 0x0C, 0x00, 0x2B, 0x46, 0x05, 0x3C, 0x00, 0x00]);
        // An argument byte 6 isn't a code.
        let t = [0x0E, 0x06, 0x2B, 0x00];
        assert_eq!(timed_copy(|i| t[i]), None);
        // Code 4 ends the copy: KH1's text stays.
        let t = [0x06, 0x3C, 0x00, 0x04, 0x2B, 0x00];
        assert_eq!(timed_copy(|i| t[i]), None);
    }

    #[test]
    fn strips_unknown_codes() {
        assert_eq!(strip_codes("Hi-Potion"), "Hi-Potion");
        assert_eq!(strip_codes("Elixir{7f}"), "Elixir");
    }
}
