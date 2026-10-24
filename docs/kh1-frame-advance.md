# KH1 frame advance (t371, t372, t377, t378, t399, t426, then the pause picture)

How the debug menu's Frame Advance row holds KH1 still and steps it one frame at a time
(`crates/kh1-hook/src/win/frameadv.rs`). Addresses are the Steam build's (Ghidra decomp of the Steam
exe, `local/decomps/steam/kh1`). Tags: **Checked (decomp)**, read from the Steam decomp; **Assumption**,
not proven.

## KH1's field loop, one frame

`0x140170D50` (the field mode's per-frame call, slot 4 of the vtable at `0x1403B1AF0`) runs, in order:

1. Mode upkeep and the pad: `0x140170870` (the game mode's state machine), `0x14018E540`,
   `0x14018C790` (the pad: `0x14028B0B0` reads every device, `0x1400DA9F0`, and rebuilds KH1's pad
   record with `0x14028B430`), `0x14017FC50`.
2. The pause machine: `0x14017EFC0` (what kind of pause is due), `0x14017F150` (the machine, state
   `0x14232DFB8`; its states are a jump table Ghidra didn't follow).
3. `0x14018DD20`, then **the task list**: `0x140284690` sets `0x142867374` = 1 / frame scale and runs
   `0x14028A710 (list 0x142867380, mask 0x142867370)`.
4. 2D work (`0x1401791E0`, `0x14017B190`), the end of the frame's drawing (`0x1400FBBC0` ->
   `0x1400FBBE0`), the pacing (`0x14018D1F0`) and the present (`0x1400DAFF0` -> `0x140109B20` -> the
   frame end `0x140109B40`). **Checked (decomp).**

## The task mask

`0x14028A710` walks the list and runs a task only when `(task flags & mask) == mask` (flags: the
task's `+2` u16). Mask 0 runs every task. **Checked (decomp).**

KH1's pause sets the mask to 1, so only the tasks flagged for a pause run (the draws, the pause
menu's own). From the older Epic analysis (`kh1-render.md` 3.1, `kh2fx.rs`); in the Steam decomp the
writes are inside the pause machine's jump table, which wasn't decompiled, and the mask is put back
to 0 as the machine finishes (`0x14017F150`, state above 6). **Assumption** for the value 1 (from
the Epic work); **Checked (decomp)** for the reset.

t371 wrote the mask between frames (from the frame end). That didn't hold the game: the tester's t371
traces show Sora's physics frames going on every frame while the mod's clock stood still (8 traces, at
30 and 60 fps). The pause machine `0x14017F150` runs every frame before the task list, and its state
handlers (the jump table) set the mask each frame, so ours was overwritten before the list read it.
**Checked (trace)** for the result; the cause is an **Assumption** (the handlers weren't decompiled).

**t372: the held tasks.** A task of the main list (0x18 bytes, from `0x14028A900` and the runner):
`+0` in use (`0x11` when made, 0 free), `+2` flags (u16, 0 when made; some are set to `0xFFFF` after),
`+4` priority, `+8` the next task, `+0x10` its function `(task) -> u32` (result bit 2: remove the
task, bit 3: stop the list); the list's first task at list `+0x10`. **Checked (decomp).** While the
game is held, Frame Advance swaps the function of every task in use whose flags lack bit 0 for one of
its own that does nothing, and gives each its function back for a frame that runs. Those are the
tasks KH1's pause holds (mask 1), held whatever the pause machine writes. New tasks are held as they
appear (the list is walked every held frame).

**t378: the draws run.** The tester's t377 test: the game state held, but the picture was black on held
frames (a picture only on the frames that ran). Only about 20 of KH1's tasks set the pause flag (read
from the decomp's registrations, `0x140284670` then a write to `+2`): among them 140000 (`0x1401E0F40`)
and the model pass 160000 (`0x1401E0FA0`), but not the map, effect or HUD draws (150000, 164000s,
165001 / 170001, 180000+). KH1's pause doesn't need them: its screen is built from the last frame (the
frame texture, `kh1-render.md`). **Checked (decomp)** for the flags; **Assumption** for why the pause
needs no draws. So from t378 only the tasks before KH1's first draw pass (priority under 140000: the
actors at 80000, the camera at 110000, the camera copy at 135000, scripts and the rest of the updates)
are held; every task from 140000 on runs. The tasks held are written to the log once ("frame advance:
holding N tasks"), to check the split against a session. **Assumption**: no task from 140000 on moves
the world (the HUD and menus run, so the command menu still answers the pad, and KH1's own effect fade
`0x1403970D0` at 159999 runs).

**t399: finding the task the picture needs.** The tester's t378 test: still black while held. His t393 log
lists the 22 tasks held (priority, RVA): 10499 `0x189F80`, 20001 `0x1E1900`, 47000 `0x28E550`, 49999
`0x1C2BA0`, 50002 `0x1A5480` / `0x1A2320`, 53000 `0x28D620`, 55000 `0x2937F0`, 60000 `0x29E790`, 79998
`0x28E6E0`, 80000 `0x298940` (the actors), 80001 `0x2B4460`, 85000 `0x2B11B0`, 95000 `0x2ABFF0`, 100000
`0x1E5D90`, 110000 `0x28CD50` (the camera), 120000 `0xD2830`, 129999 `0x2BE500`, 130000 `0x2A23E0`, 135000
`0x28CC10` (copies the camera slot of the other parity, `0x142867360`, into the current one), 137000
`0x28F6B0` (runs the list `0x142D22D08`), 139999 `0x2AEDA0` (effect bone binding). One of them is needed
for the frame to draw (**Assumption**; which one isn't clear from the decomp). The debug menu's Held Tasks
page lists them, each Held or Runs, so a session can find it; each switch is logged.

**The pause machine's states (Checked (exe), the jump table at `0x14017F568` the decomp skipped).** State 0
(`0x14017F548`, the game running) writes the mask `0x142867370` = 0 every frame. State 1 (`0x14017F2C5`)
writes it = 1, registers task 190020 (`0x14017FD60`, flags 1: it registers 190019 `0x14017F110`, flags 1, the
P A U S E text, and calls `0x14018B370`, `0x140183CD0 (0x80)`) and goes to state 2. State 3 (`0x14017F318`) calls
`0x1400D2F80` (thunk of `0x1400DE010`, KH1's sound pause: 1 on pause) and `0x140171660 (0)`. So KH1's pause
holds the same tasks as t372 did, which went black for Frame Advance: the paused picture comes from the
renderer keeping the last finished frame (`0x140109930` / `0x1401030C0`: the frame texture `+0x78E0` is
refreshed only when the frame counters `+0x775C` / `+0x7760` / `+0x7764` say a new frame was drawn;
**Assumption**, not finished).

**t426: what the tester's Held Tasks test found (Checked (in game)).** With the page: 130000 `0x2A23E0` is needed
for the models to draw (it runs the list `0x142D5C9F0`), 120000 `0xD2830` for the level, and with 120000 let run
"characters can move"; 100000 `0x1E5D90` seems to control the camera. 120000 (`0x1400D2830`) also runs two lists
of its own, `0x14233E420` and `0x14233E7D0` (made by `0x1401829B0` / `0x1401836E0`, one task per animated map part,
counts `0x142336BB0` / `0x142336BB4`; steps `0x1401824C0` / `0x140183460`), besides drawing the map
(`0x140182230` / `0x140182250`) and the cutscene bars (`0x140178580`). **Checked (decomp).** From t426, 120000
and 130000 run while held by default, and the two sub-lists are held (each a row on the page). **Assumption**:
that the sub-lists are what moved; the page lets a session check it.

**After t426: KH1's pause picture, and every task KH1's pause holds is held.** The tester's t426 test:
the level and models draw, but no effect does on held frames (magic, Keyblade swings), and he asked what
else a full freeze needs (stats, UI, lock-on). Findings (all Steam):

- **Why effects vanish (Checked (decomp)).** `0x1401ED6D0 (1)` sets two bytes on every effect instance
  in use: `+0xD5` bit 7 (paused: every effect program's step tests `0x142E9D7E0 == 0 && +0xD5 == 0`)
  and `+0xDB` = 1. The effect draw pass (`0x1401ECF30`, task 160000) draws an instance only when
  `+0xA8` (culled) and `+0xDB` are 0: so the effect pause also hides them. KH1's pause can, because it
  shows a captured picture (below). The Keyblade trail is one of these instances too
  (`0x1402AE290` starts it in the effect pool; `kh1-weapon-second.md` §8).
- **KH1's pause picture (Checked (exe)).** The pause machine's state 1 (`0x14017F2C5`) sets the mask to 1
  and registers task 190020 `0x14017FD60` (flags 1). That task registers 190019 (P A U S E,
  `0x14017F110`), captures the last frame with `0x14018B370` (`lea rcx, 0x1404DE790; jmp 0x14018B5D0`:
  the frame copied into the pause's texture) and draws it with `0x140183CD0 (0x80, 0x10)`. From the
  next frames the 10499 task `0x14017EF90` (flags 1, registered by 190019) draws it again with
  `0x140183CD0 (0x80, 0x20)` while the machine is in state 4. `0x140183CD0 (brightness, spread)` draws
  the texture in several layers (alphas 0xFF, 0x40, 0x2A, 0x20, 0x19, 0x15, 0x12, 0x10, 0x0E) offset by
  the spread: the pause's blur. With spread 0 the layers lie on each other: the frame as it was.
  **Checked (exe)** for the calls and values; **Assumption**: the capture copies the frame shown
  last (on the pause's first frame only the flagged tasks have drawn, and the battle pause shows the
  game, the tester's t399 report), and spread 0 gives the sharp frame.
- **So Frame Advance now does what KH1's pause does:** every task of the main list not flagged 1 is
  held at any priority (t372's set, which is KH1's mask 1), effects paused and hidden
  (`0x1401ED6D0 (1)`, as before), and our own task at 190020 (flags 1; `0x140284670` registers it, as
  KH1 registers its pause task) captures once per hold (the frame that just ran) and draws the picture
  every held frame with `0x140183CD0 (0x80, 0)`. It doesn't draw while KH1's own pause is up (state
  `0x14232DFB8` not 0): KH1's picture and P A U S E show then. It takes itself off the list (4) on a
  frame that runs. The debug menu is drawn at the end of the frame's drawing (`debugmenu::draw_end`,
  after every task), so it is on top. **Assumption** until tested: that the picture shows (if the
  capture didn't see the last frame, held frames would be black, as t372's).
- **What still runs while held**: only KH1's pause-flagged tasks (as in KH1's pause): 10000
  `0x1402B51A0`, 10499 (`0x140189F80` when made by the field), 30000 `0x14028ACB0` (the system stop,
  below), 40000 `0x1401CB7C0`, 90000 `0x140286200`, 137000 `0x1402AEDC0`, 140000 `0x1401E0F40` (the effect
  update: effects paused), 145000 `0x140296300` (actor-bound sounds), 159998 `0x1400D2920`, 160000
  `0x1401E0FA0` (the effect draw: effects hidden), 163000 `0x1401CB5C0`, 164003 `0x1401842A0`, 180150
  `0x140175380`, 184990 `0x1400D2910`, 185000 `0x14028BFA0`, 190000 `0x1401D50E0`, 200000 `0x14028DC30`,
  and our picture task. **Checked (decomp)** for the flags (registrations then `+2` = 0xFFFF or 1).

### What the main list's tasks are (Steam; held unless flagged 1)

Read for this (**Checked (decomp)** where a line says what the code does; "not read" otherwise).
"Update" changes the game's state; "draw" builds or draws the picture; "mixed" does both.

| Priority | Function | What it is | Kind |
|---|---|---|---|
| 10499 | `0x140189F80` | clears a block of per-frame words (`0x14233EBF8` on, stride 0x3A) | update |
| 20001 | `0x1401E1900` | two alpha curves (`0x142537DB0`, 0x48 each) stepped by the frame scale: fades | update |
| 47000 | `0x14028E550` | 0x20 queued requests (`0x142BA2088`) applied and cleared | update |
| 49999 | `0x1401C2BA0` | runs the list `0x1423D5288` | not read |
| 50002 | `0x1401A5480`, `0x1401A2320` | a list / a large step reading the pad and the frame scale | update |
| 53000 | `0x14028D620` | the system's load / mode machine (`0x14299C220`) | update |
| 55000 | `0x1402937F0` | the command menu: reads the pad, moves the cursor, issues commands (the tester: Checked (in game)) | update |
| 60000 | `0x14029E790` | runs the list `0x142D5B810`, then sums the lights' colours | update |
| 79998 | `0x14028E6E0` | large; writes the system flags `0x142867364` | not read |
| 80000 | `0x140298940` | the actors (Sora, party, enemies: AI, physics, motion, stats; the tester: what moved) | update |
| 80001 | `0x1402B4460` | every actor's collision parts into the hit list (attack / damage checks) | update |
| 85000 | `0x1402B11B0` | the lock-on: its state `0x142DB3510`, target `0x142DB3518`, the R1 list, a 90-frame timer (`0x142DB3828` += scale); `kh1-lockon.md` | update |
| 95000 | `0x1402ABFF0` | runs a list | not read |
| 100000 | `0x1401E5D90` | the camera (the tester: Checked (in game)); uses the frame scale | update |
| 110000 | `0x14028CD50` | the camera's second step | update |
| 120000 | `0x1400D2830` | the map draw (the tester); also its two sub-lists (map animation time), the sky's turn (`0x14233E174` += scale, `0x140182190`) and the map fades (`0x14019BE70`: 32 timers less the scale) | mixed |
| 129999 | `0x1402BE500` | clears `0x142DE5DB0`, two words of a record | not read |
| 130000 | `0x1402A23E0` | runs the list `0x142D5C9F0`: the models (the tester) | draw |
| 135000 | `0x14028CC10` | copies the camera slot of the other parity into the current one | draw |
| 137000 | `0x14028F6B0` | runs the list `0x142D22D08` | not read |
| 139999 | `0x1402AEDA0` | runs the list `0x142DB34B0`: effects placed on their owners' bones (`0x1402AE910`) | update |
| 145001 | `0x140290A50` | four delayed entries, each a timer less the scale, then played (`0x1400D2CB0`): voices (assumption) | update |
| 150000, 150001, 159998, 162998 | `0x14021D050`, `0x14021CFC0`, `0x14021C640`, `0x14021C5C0` | 2D draws (`0x142694B38`) | draw |
| 159999 | `0x1402D53B0`, `0x140396FF0`, `0x1403970D0` | a pass of `0x1402D9B40`...; a special effect's place; a fade (-0.1 a frame) | mixed |
| 160000 | `0x1402E26D0` | large; uses the frame scale, writes `0x14233FE08` | mixed (not read) |
| 164000..164004 | `0x1401800E0`, `0x1401C7E10`, `0x140184420`, `0x1401844C0`, `0x140184540`, `0x140184B40`, `0x140185360` | 2D / screen overlays; `0x140185360` uses the frame scale | mixed |
| 165001 | `0x1402680F0` ... `0x140275B00` (12) | the HUD's parts (GPU draws); `0x140274540` and `0x1402758B0` step by the frame scale | mixed |
| 169999 | `0x140293750` | the command menu's flags (`0x142D53A4C`) | update |
| 170001 | `0x14026DB00`, `0x140270620`, `0x140272F10`, `0x1402739D0`, `0x14027B870` | HUD draws; `0x14027B870` steps a fade by the scale unless the system stop is on | mixed |
| 180000.. 190050 | (about 40) | 2D, menus, text, fades; several step by the frame scale | mixed (not read one by one) |

With KH1's pause picture every one of these is held while the game is held, as in KH1's pause, so
none of them needs a split: the picture covers what they would draw.

### KH1's system stop (a second, developer freeze; Checked (decomp) and (exe))

`0x142867368` bit 0 (the "system status" word): the task at 30000 (`0x14028ACB0`, flags 1) sets it
from the second pad record (`0x14298CA20`: a press of `+8` bit 3 toggles it, a repeat of `+0x10` bit
0x20 runs one frame: a developer frame advance) and with it `0x142432130`; the task at 140000 copies
`0x142432130` into the effects' held flag `0x142E9D7E0`. With it on: every effect program's step is held
but effects still draw (the effect pass doesn't test it), the HUD fades `0x14026D570` / `0x14027B870` /
`0x14027F780` hold, `0x140395360` (50000) returns, and `0x1402B47A0` keeps a flag. The exe's only
readers of bit 0 are those (xrefs, Checked (exe)). It doesn't hold the actors, the camera or the
scripts. Frame Advance doesn't use it with KH1's pause picture (everything is held there).

### Sound while held (after t426)

KH1's pause machine calls `0x1400D2F80` (a jump to `0x1400DE010 (mode)`, KH1's sound pause) with 1 in
state 3 (`0x14017F318`) and 0 in state 4 (`0x14017F4E6`) as it ends; the machine has 12 such calls
(Checked (exe), call sites). `0x1400DE010`: for each sound in its list (`0x1404D6618`), a streamed one
(`+0x3C` = 0: the music, an assumption from its handling) is set to half its volume (`+0x44` x 0.5) on 1 and back to `+0x44` on 0; a
sound effect playing is paused on 1 and resumed on 0 (Checked (decomp)). Frame Advance makes the same
calls as a hold begins (1) and ends (0), so a step plays about one frame of sound and the sound
effects don't run on while held; the music plays on at half volume while held, as in KH1's pause.
While KH1's own pause is up it makes its own calls (its 0 as it closes resumes everything), and ours
is made again after it. The debug menu's Held Sound row (Paused / Plays, this session) turns it off.
**Assumption**: the music keeps its place (KH1's pause doesn't stop it).

## Actors

Each actor is a task of a second list, `0x142D5AB00` (0x28-byte tasks, made by `0x140297980` with
priority 2000 for Sora, 3000 for the party, 4000 by default, ...; `+0x18` the actor, `+0x20` its kind's
step), whose runner is `0x140297FE0` (the actor's update, which calls the physics step). That list is
run with mask 0 by `0x140298940`, the main list's task at priority 80000 (registered by `0x140297890`
with flags 0). So holding that one task holds every actor. **Checked (decomp).**

## Effect instances

`0x1401ED6D0 (on)` marks every effect instance in use paused or not: the effect pool
`0x142678280` (0x100 instances of 0xE0 bytes); from each instance's in-use byte (`+0x92`, bit 0),
`+0x43` bit 7 set and `+0x49` = 1 (or cleared). (t371 called these model objects: they are the effect
instances, the pool in the effects notes.) The pause machine calls it with 1 as its pause opens
(through the thunk `0x1401E0C70`). Frame Advance calls it with 1 when it holds the game and 0 when it
lets a frame run. (After t426) The second byte it writes, `+0xDB` (the decomp's `pbVar1[6]` from
`+0xD5`), stops the effect draw pass drawing the instance (`0x1401ECF30`, Checked (decomp)), so held
effects are only seen through KH1's pause picture (above). KH1's other freezes call it too: the
pause machine (three sites), `0x14017FCA0`, `0x14028A2B0` (task 47000, a scripted freeze of the actors)
and its ends `0x140289060` / `0x140289750` (Checked (exe), call sites). **Checked (decomp)**; the tester's t371 test: effects held and stepped frame by
frame (**Checked (trace / in game)**).

## The frame scale

`0x14018D1F0` (end of every loop pass) counts the 60 Hz vblanks since the last pass (`0x142340EC4`)
and sets the frame scale `0x14233FBDC` = count x `0x14233FBCC` (forced to 2 x when `0x14233FBF0` is
set; times `0x14233FBE4`, the game speed, 1.0 from the field's start `0x140170630`; capped at 5;
128 when `0x1423404EC` = 1). The 30 fps setting is read through `0x1400FDB90` (30.0) in the field
loop. **Checked (decomp).** Held frames run the pacing too, so a stepped frame gets the scale a
frame played at the same setting gets.

## KH1's pad record

`0x14298CA90`: `+0` on, `+4` held, `+8` pressed, `+0xC` released, `+0x10` / `+0x14` repeats, `+0x1C` /
`+0x20` their timers (counted down by the frame scale). `0x14028B430 (record, buttons)`: pressed =
held now and not held in the record, released = the reverse. **Checked (decomp).** Frame Advance puts
`+4` and the timers back to the last frame that ran before a stepped frame, so its presses are
against what the game last saw.

## The keyboard

`0x1401122E0` makes KH1's DirectInput devices: the keyboard (`+8`, read through `0x140112080`) and
the mouse (`+0x10`, `0x140112100`). `0x1400DA9F0` reads the keyboard's 256 key bytes into the input
object at `+0x12A0 + index x 0x100` (index `+0x14A0`, two buffers) and then calls `0x1400D7E80` with
it. **Checked (decomp).** `0x1400D7E80` reads the key bytes of the buffer `+0x14A0` names. t377 hooks its
start (first bytes `40 56 41 56 48 83 EC 48`, `push rsi; push r14; sub rsp, 0x48`, **Checked (exe)**: the
Steam exe, SHA-256 `d7907462...4bac`) and, while Frame Advance is on, clears the period key's byte
(DirectInput 0x34) in both buffers, so KH1 never sees it.
