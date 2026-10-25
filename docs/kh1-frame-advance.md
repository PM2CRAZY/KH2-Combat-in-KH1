# KH1 frame advance (t371, t372, t377, t378, t399, t426, t445 to t447, then Live only)

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
- **Removed after t447 (the tester: "the actual pause seems to hold the image in the frame buffer so
  I wouldn't be using it, focus on getting the live version working").** The picture task, the
  capture and the Held Picture row are gone; Live (below) is the only way. What follows is kept as
  the record of KH1's pause. **t445 to t447 (removed):** every task of the main list not flagged 1 is
  held at any priority (t372's set, which is KH1's mask 1), effects paused and hidden
  (`0x1401ED6D0 (1)`, as before), and our own task at 190020 (flags 1; `0x140284670` registers it, as
  KH1 registers its pause task) captures once per hold (the frame that just ran) and draws the picture
  every held frame with `0x140183CD0 (0x80, 0)`. It doesn't draw while KH1's own pause is up (state
  `0x14232DFB8` not 0): KH1's picture and P A U S E show then. It takes itself off the list (4) on a
  frame that runs. The debug menu is drawn at the end of the frame's drawing (`debugmenu::draw_end`,
  after every task), so it is on top. **Assumption** until tested: that the picture shows (if the
  capture didn't see the last frame, held frames would be black, as t372's).
- **What still runs while held**: only KH1's pause-flagged tasks (as in KH1's pause): 10000
  `0x1402B51A0`, 10499 `0x14017EF90` (the pause's own; the field's 10499 `0x140189F80` is not flagged), 150000 `0x1402847A0`, 30000 `0x14028ACB0` (the system stop,
  below), 40000 `0x1401CB7C0`, 90000 `0x140286200`, 137000 `0x1402AEDC0`, 140000 `0x1401E0F40` (the effect
  update: effects paused), 145000 `0x140296300` (actor-bound sounds), 159998 `0x1400D2920`, 160000
  `0x1401E0FA0` (the effect draw: effects hidden), 163000 `0x1401CB5C0`, 180150
  `0x140175380`, 184990 `0x1400D2910`, 185000 `0x14028BFA0`, 190000 `0x1401D50E0`, 200000 `0x14028DC30`,
  and our picture task. **Checked (decomp)** for the flags (registrations then `+2` = 0xFFFF or 1).

### The main list's tasks, read one by one (after t447; Steam)

The tester's m22: "do not take my word for what I said; look through the C decomp." Every
registration on the main list was found (the 213 call sites of `0x140284670`, plus
`0x14028A900 (0x142867380, ...)` in the list's own start `0x1402845B0`) and each task's function
read in the Steam decomp. **Checked (decomp)** for each line unless it says Assumption. (It
replaces the t445 table, which had errors: 10499 isn't a plain clear, 20001 is the pad vibration,
110000 isn't a camera, 129999 queues render states, 164003 isn't flagged, 135000 is a view copy.)
"Update" changes the game's state; "draw" builds or draws the picture (or what only the picture
reads: matrices, caches, render states); "mixed" does both. "Pause" is the task's flags (`+2`): 1 or
`0xFFFF` run under KH1's pause (mask 1), 0 is held by it.

**KH1's render queue (found for this; it explains two of the tester's finds).** KH1 has a second,
double-buffered list of deferred draws, `0x1423E1920` + `0x1423E1910` x 0x6020 (the one being
filled at `0x1423ED960`). `0x1401CB730 (sub-priority, function)` queues an entry into it; task 40000
(`0x1401CB7C0`, flags `0xFFFF`) swaps the buffers and starts running last frame's entries in
sub-priority order; an entry that returns 2 pauses the queue, and the runner continues it after
every later task (`0x1401CB5A0`, called by the task runner `0x14028A710` after each task); 150000
(`0x1401CB770` through `0x1402847A0`) drains the rest. So a draw queued in one frame is drawn in the
next. Who queues: 110000 (the fog's GS registers, `0x14018CC10`), 120000 (the map's and the sky's
packets), 129999 (two render-state markers), the models' shadows (130000), the map parts' texture
frames (only when the frame changes). A task that queues and is held drops its draws from the next
frame. A second queue `0x1423ED978` is run at 163000 (`0x1401CB5C0`).

**The field's tasks** (made at every room's start by `0x1401705D0` / `0x14028AC10` / the room's
`0x14028AAA0`; these are what the tester's logs list):

| Priority | Function | What it is (Steam decomp) | Kind | Pause |
|---|---|---|---|---|
| 10000 | `0x1402B51A0` | returns 0: an empty anchor | none | `0xFFFF` |
| 10499 | `0x140189F80` | clears the 32 "transformed this frame" words of the map collision cache (`0x14233EBF8`, stride 0x40). `0x140188460` (collision queries: physics, camera, the shadows' floor test) transforms a collision group once a frame into a block of the aged heap that it releases at once (`0x14028DB70`: freed by 200000) | draw (a per-frame cache) | 0 |
| 20001 | `0x1401E1900` | the pad vibration: two curves (`0x142537DB0`, 0x48 each, stepped by the frame scale) into the motor bytes `0x14298C9B0 + pad x 0x70 + 2 / 3`, which the pad read (`0x14028B0B0`) sends and clears every frame | update | 0 |
| 30000 | `0x14028ACB0` | KH1's system stop toggle (section below) | update | 1 |
| 40000 | `0x1401CB7C0` | the render queue: swap, run last frame's first part | draw | `0xFFFF` |
| 47000 | `0x14028E550` | 0x20 queued event triggers (`0x142BA2080`, 0x18 each) handed to the event system (`0x1401A1A10`: an event entry started; `0x1401A1E30`: a room change); then cleared | update | 0 |
| 47000 | `0x14028A2B0` | (from a script) a scripted freeze of the actors (`0x1401ED6D0` too) | update | 0 |
| 49999 | `0x1401C2BA0` | runs `0x1423D5288`: the event scripts' effect requests (`0x1401BFF70`: load an effect file, start it, place it) | update | 0 |
| 50002 | `0x1401A5480` | runs `0x1423D3E18`: the event scripts' threads (`0x1401A2CF0`) | update | 0 |
| 50002 | `0x1401A2320` | the event zones: the room's event entries (`0x142382AD0`, 0x360 each), their timers (less the frame scale), the confirm-button checks | update | 0 |
| 53000 | `0x14028D620` | the room data loader (`0x14299C220` state; loads `xl_limit.dat`) and the room change bit `0x142867364` 0x10000 | update | 0 |
| 55000 | `0x1402937F0` | the command menu: the pad (`0x14298CA94` / `CA98`), the cursor, the commands it issues (`0x142D53A40`), the shortcut buttons; not while the system status has bit 2. The tester: Checked (in game) | update | 0 |
| 60000 | `0x14029E790` | the lights: runs `0x142D5B810` (light colour fades, less the frame scale), adds the ambient colours, steps the light flicker phases (`0x142340930`..: + a rate a frame) | update | 0 |
| 79998 | `0x14028E6E0` | the event scripts' actor commands (show / hide, motion, turn, place, collision on / off) | update | 0 |
| 79999 | `0x1402AACD0` | the enemy spawner: spawn groups, their respawn timers (less the frame scale), `0x140290D60` makes the actors | update | 0 |
| 80000 | `0x140298940` | the actors: runs `0x142D5AB00` (Sora, party, enemies: AI, physics, motion, stats). The tester: Checked (in game) | update | 0 |
| 80001 | `0x1402B4460` | every actor's collision parts into the hit list (attacks, damage) | update | 0 |
| 85000 | `0x1402B11B0` | the lock-on: state `0x142DB3510`, target `0x142DB3518`, the R1 list, its timer (`kh1-lockon.md`) | update | 0 |
| 90000 | `0x140286200` | runs `0x142868BB0`: the file loads in flight (`0x140286420`) | update (I/O) | `0xFFFF` |
| 95000 | `0x1402ABFF0` | runs `0x142D60FB0` (`0x1402ACAD0`): the dropped prizes (orbs, munny): wait, then fly to Sora (`0x8003` point) and are taken. **Assumption** that these are the prizes (from the code's shape) | update | 0 |
| 99999 | `0x1401BD8D0`, `0x1401BF850` | the event camera (a path played by the frame scale; made by event commands) | update | 0 |
| 100000 | `0x1401E5D90` | the field camera: the camera mode's step (table `0x140506CE0`), its blend into the eye / target `0x14299BAF0` / `0x14299BAE0`. The tester: Checked (in game) | update | 0 |
| 110000 | `0x14028CD50` | **not a camera (the tester was right: "it stops one of the backgrounds").** The view set-up: the view and projection matrices for this frame's parity slot (`0x14299C1A0 + parity x 0x40`, `0x14299BC40`, the clip `0x14233FD00`) and the yaw `0x14299C194`, all from the camera; inside (`0x14018D410` -> `0x14018CC10`) the fog values, **the fog packet queued** (render queue, sub-priority 0x271A) and **the map's texture scroll** (`0x142336C00`, 16 u / v pairs, += the frame scale x the rates `0x142336D00`, wrapped at 1); first, the screen zoom step (`0x140183BD0`: a delay `0x14233E834`, the zoom `0x1404DD56C` moved by `0x1404DD574` a frame, the viewport through `0x1401703C0`) | mixed (draw, with two steps) | 0 |
| 120000 | `0x1400D2830` | the map draw (the tester); its sub-lists (map parts: texture frames by the frame scale), the sky's turn and the map fades, queued packets (`0x140182350`), the cutscene bars (`0x140178580`); registers 164002 / 164004 below when they are on | mixed | 0 |
| 129999 | `0x1402BE500` | queues two render-state markers (`0x1402BEE60` 0x1020 at sub-priorities 45499 / 60499: GS test / alpha registers reset between draw groups) | draw | 0 |
| 130000 | `0x1402A23E0` | the models: runs `0x142D5C9F0` (`0x1402A1320`, one per object): its matrix, then its colour stepped toward its target by 4 a frame (`+0x370`..`+0x373` toward `+0x350`..`+0x35C`, once `+0x374` bit 0x4000), then its draw (`+0x20`: `0x1402A1B70` & co., with the shadow `0x1402BD6F0`). The tester: Checked (in game) | draw (one step) | 0 |
| 135000 | `0x14028CC10` | the view copy: the other parity's matrices into the ones the draws from here on use (`0x14299BAA0`, `0x14299BB30`): the effects and 2D draw with last frame's view | draw | 0 |
| 137000 | `0x14028F6B0` | runs `0x142D22D08` (`0x14028F6D0`): the map objects' behaviour scripts (`0x1402BCEC0` -> `0x1402CC620`, KH1's object script machine) | update | 0 |
| 137000 | `0x1402AEDC0` | (made by the menu code, `0x1402FE750`) places one effect on a matrix | draw | `0xFFFF` |
| 139999 | `0x1402AEDA0` | runs `0x142DB34B0`: effects bound to actors: placed on bones (`0x1402AE910`), delayed starts (`0x1402AEBE0`, less the scale), fade-outs (`0x1402AEF20` / `0x1402AEFB0`, less the scale) | mixed | 0 |
| 140000 | `0x1401E0F40` | the effect update (`0x1401ED720` / `0x1401ECB50`); copies the system stop into `0x142E9D7E0` | update | `0xFFFF` |
| 145000 | `0x140296300` | the sounds bound to actors: follow them, stop with them | update (sound) | `0xFFFF` |
| 145001 | `0x140290A50` | four delayed voice / sound plays, each a timer less the frame scale (`0x1400D2CB0`) | update | 0 |
| 150000 | `0x1402847A0` | the render queue drained (`0x1401CB770`) | draw | `0xFFFF` |
| 159998 | `0x1400D2920` | two 2D packet buffers allocated for the frame (`0x14017E2E0`) | draw | `0xFFFF` |
| 160000 | `0x1401E0FA0` | the effect draw pass (`0x1401ECF30`) | draw | `0xFFFF` |
| 163000 | `0x1401CB5C0` | the second render queue run | draw | `0xFFFF` |
| 163001 | `0x140292E40` | frees the actors taken off this frame (their models, tasks) | update | 0 |
| 164003 | `0x1401842A0` | the screen fade overlay (`0x14233E804`: its alpha, colour) | draw | 0 |
| 165001 | `0x1402680F0`, `0x1402684A0`, `0x14026FE80`, `0x140270F80`, `0x1402719F0`, `0x140272330`, `0x140274400`, `0x140274540`, `0x1402747D0`, `0x1402758B0`, `0x140275B00` | the HUD's parts (made by `0x140266DC0`): pop-ups, numbers (`0x140270F80`: 10 number pop-ups at an actor's point; `0x140272330`: the munny count), gauges. Each is a small state machine (states and timers + 1 a frame) plus sprite animations (`0x14027E460`: frame time += `0x14233FBE0`, the half frame scale) plus its GPU draw. Their data is `0x1427F13D0`..`0x142866D18` (read for this: only HUD functions write there, but the command menu's `0x142852814`; the Gummi Ship's `0x1428119C0` block is read by its own code) | mixed | 0 |
| 169999 | `0x140293750` | opens / closes the HUD's command menu window when the command menu's flags change (`0x142D53A4C`) | mixed | 0 |
| 170001 | `0x14026DB00`, `0x140270620`, `0x140272F10`, `0x1402739D0`, `0x14027B870` | more HUD parts, as above (`0x14027B870` draws the command menu's window from the command menu's state `0x142D539F0`; its fades hold on the system stop) | mixed | 0 |
| 180150 | `0x140175380` | keeps a texture request up (`0x14010DC50`) | draw | `0xFFFF` |
| 180200 | `0x140172A20` | runs `0x1422EC1A0`: the event message windows (`0x140173680`: open / close by the frame scale, text) | mixed | 0 |
| 184990 | `0x1400D2910` | the 2D buffers of 159998 released | draw | `0xFFFF` |
| 185000 | `0x14028BFA0` | runs `0x14299B360`: the finished file loads' callbacks | update (I/O) | `0xFFFF` |
| 190000 | `0x1401D50E0` | the 2D sprite lists (double-buffered `0x1423FD7D4`): last frame's freed, this frame's built | draw | `0xFFFF` |
| 190000 | `0x1401EB750` | the screen shake: timers less the frame scale, the offset `0x14018BCC0` | mixed | 0 |
| 190000 | `0x14018B010` / `0x14018B220` | (a room's start, `0x14018AFE0`) the fade-in: 64 frames less the frame scale | mixed | 0 |
| 190050 | `0x140170580` | runs `0x1423404C0`: the screen colour fades (`0x14017AA10` & co.: time += the frame scale) | mixed | 0 |
| 200000 | `0x14028DC30` | the aged heap: ages and packs its blocks, frees the released ones | draw (memory) | `0xFFFF` |

**On demand in the field:** 164002 `0x140184540` and 164004 `0x140184B40` / `0x140185360` (made by
120000 each frame they're on, one frame each): screen overlays; `0x140185360` steps three pulse
phases (`0x14233E820` / `824` / `828`, + a rate a frame) and one by the frame scale (`0x14233E838`).
164000 `0x1401800E0` / `0x1401C7E10` and 164002 `0x140184420` / `0x1401844C0`: event picture and
screen effects. 100000 `0x140289D40` (from `0x14028A410`, 52000) and `0x1403A06B0` (from `0x1403A10F0`,
53000): two scenes' own cameras; 100500 `0x1402C96E0` / `0x1402CDAC0`: an actor kind's camera
follow (`0x1402C9DC0`). 52000 `0x14028A410`: a scene's own list `0x14298C7B0`. 137000 `0x140399890`,
`0x14039A270`, `0x14039A4A0`, `0x14039AB50`, `0x14039B610`, `0x14039FB60`, 159999 `0x140396FF0` /
`0x1403970D0`, 164999 `0x140399490`, 50000 `0x140395360` / `0x1403963C0`, 52000 `0x140395A10`,
`0x140395E70`, `0x140396250`, `0x140396AA0`, `0x1403979E0`, `0x14039C2C0`, `0x14039FEF0`: the special
battles and scenes of `0x140395800` & co. (`0x14039x`: their state `0x142EFB0F0`, a countdown
`0x14298C7A4` less the frame scale, their own effects); all update but the 159999 / 164999 places.

**Room changes and event commands** (one-shot or until done; made by the room change
`0x1401A4780` / `0x140192520` and by event script commands, the `0x1401A8`.. / `0x1401AB`.. /
`0x1401C3`.. to `0x1401CA`.. handlers, which the decomp has no callers for: they are called through
the script's table): 50000 `0x14017D860` (load the next room's file), `0x140180860` (its music),
`0x140192440` (its map), `0x1401B5320`, `0x1401BA320`, `0x1401BADF0` (an event camera step, + 1 a
frame), `0x1401C44A0`, `0x1401C4500`, `0x1401C79F0`, `0x1401C9650`, `0x1401C9680`, `0x1401C9770`,
`0x1401C9810`, `0x1401CA810`, `0x1401CB130` (wait for a button, a timer, then a window or a sound);
50001 `0x1401A3400` (the event start); 50002 `0x1401B3C90`, `0x1401C41B0`.. `0x1401C42F0`,
`0x1401C9290` (wait for a sound or a load, then go on); 10000 `0x14017A150` (from four commands);
30000 `0x14018BA00` / `0x14018BA60`; 189999 `0x14017D860`; 190000 `0x14017D990`, `0x14017E170`,
`0x1401810D0`, `0x14018AD70`, `0x14018B100`, `0x14018B1A0`, `0x14018B2F0`, `0x1401C6EB0`..`0x1401C77A0`
(the event pictures, the fades, the game over); 190001 `0x1401B39B0`; 190022 `0x14017AF50`. All
update (they step the event), the 190000 ones draw too.

**KH1's pause and menus:** 10499 `0x14017EF90` (flags 1: redraws the pause picture), 190019
`0x14017F110` (flags 1: P A U S E), 190018 `0x14017FD40`, 190019 `0x14017FCE0` (flags `0xFFFF`), 190020
`0x14017FD60` (the capture); 159999 `0x1402D53B0` (the camp menu, from the pause machine's states:
its flags are the pause's); 160000 `0x1402E26D0` (a menu's background shake, from an event command);
180100 `0x140301620` / `0x140302610`, 180199 `0x1402F9220` / `0x14030BC70`, 180201 `0x1402F64B0`,
`0x1402F64E0`, `0x1402F9940`, `0x14030BFA0`, 180250 `0x1402FED10`, 180300 `0x140180240` / `0x1401802C0`,
180499 `0x14018E8B0` / `0x14018EE80`, 180000 `0x1401CB500` / `0x14038D9B0` (the world / room words):
menu screens (Jiminy's journal, shops, the save point, the world map). 190050 `0x140170580` is also
made by the title (`0x140170BB0`, `0x140171410`). **Gummi Ship mode** (`0x14020x`..`0x14025x`):
120001 `0x140211830` / `0x140220540`, 150000 `0x14021D050`, 150001 `0x14021CFC0`, 159998
`0x14021C640`, 162998 `0x14021C5C0`, 180200 `0x140221200`, 180201 `0x1401EE930`, `0x1401F8490`,
`0x1401F9980`, 180250 `0x140215BC0`. Not read line by line (not the field): menus and the Gummi Ship
are outside what Frame Advance is for (**Assumption**: it's used in the field).

### KH1's system stop (a second, developer freeze; Checked (decomp) and (exe))

`0x142867368` bit 0 (the "system status" word): the task at 30000 (`0x14028ACB0`, flags 1) sets it
from the second pad record (`0x14298CA20`: a press of `+8` bit 3 toggles it, a repeat of `+0x10` bit
0x20 runs one frame: a developer frame advance) and with it `0x142432130`; the task at 140000 copies
`0x142432130` into the effects' held flag `0x142E9D7E0`. With it on: every effect program's step is held
but effects still draw (the effect pass doesn't test it), the HUD fades `0x14026D570` / `0x14027B870` /
`0x14027F780` hold, `0x140395360` (50000) returns, and `0x1402B47A0` keeps a flag. The exe's only
readers of bit 0 are those (xrefs, Checked (exe)). It doesn't hold the actors, the camera or the
scripts. Frame Advance uses it (Live, below) to hold the effects while they still draw.

### Live: the draws run while held (t447; the only way after t447)

(t447 added it as the `Held Picture < KH1 Pause / Live >` row; after t447 the row is gone and this is
how every hold works.) The picture is drawn live, as t426 did, with what those draws step held where
KH1 has a way:

- Held: the tasks before 140000, and two known update tasks after it (145001 `0x140290A50`: four
  delayed entries, a timer less the frame scale each, then played; 159999 `0x1403970D0`: a fade, -0.1 a
  frame). 130000 (models) and 120000 (map) run by default; 120000's sub-lists stay held.
- Effects: KH1's system stop (`0x142867368` bit 0 set, `0x142432130` = 1, both put back after; set again
  every held frame in case KH1's 30000 toggle took it off): the effects' held flag `0x142E9D7E0`
  follows at 140000, every program's step stops and the pass still draws them (no `+0xDB`). The HUD
  fades that test bit 0 (`0x14026D570`, `0x14027B870`, `0x14027F780`) stop too. **Checked (decomp)**.
- The shape trails (KH1 programs 156 KeShpTail3 `0x140337450` and 157 KeShpTail3X `0x140338160`) step
  their animation time (work 0 `+0x1F0`, by the key's `+8`) in their draws, on every draw with a model,
  held or not (**Checked (decomp)**). The other programs' draws were searched for a step (a field
  added to from the key, or the frame scale): only these two showed one (a search, not a full read:
  **Assumption** for the rest); the ones not decompiled (50, 140, 143, 144, 146..151, 158, 159) are
  `ret 0` stubs (**Checked (exe)**). While held their slots in KH1's program table
  (`0x14054FF00` + i x 0x50 + 0x18) point at ours, which run KH1's and take the step back (only when the
  slot holds KH1's own; put back after). The ribbon trails (161 RyjDrawShipoly, 200 RyjDrawShipolyBone)
  advance their ring only when not held (`0x14033AF90`: `0x142E9D7E0`, `+0xD5`, `+0xD8`), and take the
  bones' places in the draw (the same while the models are held). **Checked (decomp)**.
  **Assumption**: KH1's Keyblade trail is one of the ribbon programs (the weapon's effect data isn't
  staged here).
- 120000 runs through `map_live`: KH1's own, then its steps put back: the sky's turn `0x14233E174`
  (`0x140182190`: += scale x `0x14233E010`), the 32 map fades `0x142356370` (timer, alpha, rate; the draw
  reads the alpha before the step) and their done bits `0x14232DE00` (`0x14019BE70`). **Checked (decomp)**.
- Not covered in Live (all run while held): the HUD's other steps (165001 `0x140274540`,
  `0x1402758B0`; 164004 `0x140185360`: the frame scale), the 160000 pass `0x1402E26D0`, the 180000s and
  190000s (2D, menus, text, fades; 180000 `0x14038D9B0` writes the world / room words), and anything
  else in a draw task that steps by frames.

### Every update held, every draw run (after t447)

The tester's m22: "focus on getting the live version working" and freeze the whole game state.
From the table above (the Steam decomp):

- **Held** (function swapped for one that does nothing, as since t372): every task of the main
  list before 140000 not flagged for KH1's pause, but those below; from 140000 on, the updates
  145001 `0x140290A50` (delayed plays), 159999 `0x1403970D0` (a fade, -0.1 a frame), 163001
  `0x140292E40` (the actors taken off freed) and 190000 `0x1401EB750` (the screen shake: its offset
  stays set). 120000's two map part lists stay held. The pad vibration (20001) is held, and the
  pad read clears the motor bytes every frame (`0x14028B0B0`), so the pad stops vibrating while
  held. **Checked (decomp).**
- **Run** before 140000 (only the picture uses them): 10499 `0x140189F80` (the collision cache's
  reset: its blocks are released every frame, so held it would leave the shadows' floor test,
  which runs in the models' draw, on freed memory), 110000 `0x14028CD50` (the view set-up: its
  matrices are rebuilt from the held camera, the same ones; the fog packet it queues is drawn the
  next frame, so held it was missing: the tester's "it stops one of the backgrounds"), 120000
  `0x1400D2830` (the map, through `map_live`), 129999 `0x1402BE500` (its render-state markers in
  the render queue), 130000 `0x1402A23E0` (the models), 135000 `0x14028CC10` (the view copy). The
  Held Tasks page still switches any of them. **Checked (decomp)**; **Assumption** for the
  background: that the tester's background is the fog packet or the texture scroll (both in
  110000; the texture scroll stands now by the frame scale).
- **The frame scale is 0 on a held frame.** The pacing `0x14018D1F0` sets the frame scale
  `0x14233FBDC` and its half `0x14233FBE0` (min(scale / 2, `0x14233FBCC`)) at the end of every pass,
  before the frame end where Frame Advance decides; on a held frame both are written 0 there, and
  the next pass's pacing sets them again. Every draw that runs and steps by them stands: 110000's
  texture scroll (`0x142336C00`), the sky's turn and the map fades (120000), the map parts'
  texture frames, 164004's `0x14233E838`, the HUD's sprite animations (`0x14027E460`: frame time +=
  the half scale; the only readers of the half in the field are the HUD's), the HUD fades
  `0x1402758B0` / `0x140274540`, the event message windows (180200), the room fade-in (190000
  `0x14018B010` / `0x14018B220`), the screen fades (190050), the bound effects' timers. Divisions by
  the scale: only in the camera (`0x1401E5D90`) and the actors (`0x1402C`.., `0x14029A3C0`,
  `0x1402B2A20`), all held; `0x142867374` (1 / scale, written as the list starts) is read only by
  actors and one scene, held. The mod's own readers: the KH2 Drive sphere's fade test reads
  `frameadv::shown_scale` (the last run frame's); the KH2 camera, the physics and the KH2 steps run
  in held tasks. **Checked (decomp)**; **Assumption**: no running draw needs a scale above 0
  (none found that divides by it).
- **Put back at every held frame's end** (saved as the hold begins, after the last frame that
  ran; saved again after KH1's own pause closes during a hold, since its menu can change what
  the HUD shows): 110000's screen zoom (`0x140183BD0`: the delay `0x14233E834`, the zoom state
  `0x1404DD560`..`0x1404DD577`, `0x14233EBA0`), 164004's pulse phases (`0x140185360`: `0x14233E820`,
  `824`, `828`, + a rate a frame), the HUD's data `0x1427F13D0`..`0x142866D18` (its parts' states
  and timers, + 1 a frame, and the sprite animations; written only by the HUD's code, the decomp's
  xrefs: the only other writer in it is the command menu's `0x142852814`, held), and each model's
  colour (`+0x370`, stepped 4 a frame toward its target by `0x1402A1320`) and its bit 0x4000. Each
  held frame's draws step them once from the same values, so every held frame shows the same
  picture, and the next frame that runs starts from where the hold began. About 0.5 MB copied a
  held frame, nothing when not held. **Checked (decomp)**.
- **Not covered**: menus and the Gummi Ship (their tasks are listed, not split); KH1's own pause
  during a hold shows its own picture, as before, and the frame it closes on has the frame scale
  KH1 gave it (the draws' time steps once then; the updates stay held).

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

### The debug menu's Frame Advance page (after t463)

The tester ("make frame advance a submenu and throw it plus frame advance option in there"): the
main page has one row, Frame Advance `>`, opening a page of its own (`debugmenu::Page::FrameAdvance`)
with the Frame Advance row (On / Off), Held Sound (Paused / Plays), Game Frames (the Trace Recorder's
row, also still on that page; Frame Advance turns it on) and the Held Tasks page (kept as a debug
tool, the tester's answer 2). Back (or the cancel button) goes up to the main page; Held Tasks goes
back to the Frame Advance page.

### The camera while held: the Camera row (after t463)

The tester: "on your previous build ... if the cam 1 + cam 2 bit was running then we could freely move
the camera" (the Held Tasks labels before t462: 100000 `0x1E5D90` "camera", 110000 `0x28CD50` "camera 2").
Why (**Checked (decomp)**):
- KH1's pad step (`0x14018C790`) runs every frame outside the task list, held or not, so the pad
  record (`0x14298CA90`) and the processed sticks (right `0x14298CAC0`) are live while held.
- 100000 (`0x1401E5D90`) runs the camera mode's step from the table at `0x140506CE0` (16 bytes a mode:
  1 follow `0x1401E1920`, 2 first person `0x1401E2390` / exit `0x1401E2610`, 3 lock-on `0x1401E2650`, ...;
  read from the exe, **Checked (exe)**; the KH2 camera puts its own in 1 to 6 and 8, `camera.rs`), then
  blends and copies its internal eye / look point (`0x142537E60` / `0x142537E70`) into the public
  pair (`0x14299BAE0` / `0x14299BAF0`). The stick and mouse turn (`0x1401E7320`: `0x14298CAC0` /
  `0x14298CAC4`, the mouse `0x140138B30`, x the frame scale) and the follow camera's rules
  orbit the eye round the subject (Sora, `0x142537E48`), who is held where he stands. Its writes are
  the camera's own: `0x142537E44`..`0x14253810C`, the mode `0x14299BB08`, the public pair, the field of
  view (`0x14028CAF0`), mode 2's offsets in `0x140506DB0`.. (the decomp of the modes and their helpers
  `0x1401E5530`, `0x1401E6670`, `0x1401E6C10`, `0x1401E6FF0`, `0x1401E62A0`, `0x1401E4820`, `0x1401E5360`).
  The exceptions: Select in modes 1 and 3 enters first person (`0x1401E5360 (2, 30)`; with `0x142867364`
  bit 0x2000 a sound, `0x14017A040 (4)`), whose first frame clears the lock-on (`0x1402B0970`) and
  whose fade writes Sora's alpha step (`+0x64`) and clears his draw bit (`0x1425380F8` counting down by
  the frame scale); R3 or Select in mode 2 leaves it (its exit fades Sora in).
- 110000 (the view set-up) builds this frame's view from the public pair. Before t463 it was held by
  default, so the camera's moves showed only with both running; since t463 it runs.
- Since t463 the frame scale is 0 on a held frame, and the camera steps by it (and divides the
  subject's speed by it: `0x142537E50` / `E54` = distance / scale), so 100000 let run alone would stand.

The Camera row (debug menu > Frame Advance, Free / Held, Held by default, this session): Free runs
100000 on held frames through `frameadv::camera_live`: the frame scale the pacing set for that frame
(read at the frame end before it is written 0) for the call only, 0 again after; Select taken off the
pad record's pressed word (`+8`, `0x8000`) for the call, and R3 (`0x200`) too in mode 2, put back
after; while KH1's first-person fade counts (mode 2, `0x1425380F8` > 0) it is held that frame. So the
right stick / mouse move KH1's camera (or KH2's: its follow, lock-on and first person read the same
scale, `camera.rs` `STEP_RVA`) round the held Sora, and nothing else steps. The Held Tasks page's
100000 row is the same switch. KH2's camera's `run` / `run_first_person` write only the camera and
the field of view while Select is off (its Select fades Sora, `camera::fade`). **Checked (decomp)**;
**Assumption**: the area-camera check (`0x1401E5530`, by the subject's place) and the movement-state
cameras (`0x1401E3320`, by Sora's state) switch nothing while Sora is held, as both read only him.
Not covered: event and scene cameras (99999, 53000, 100500) stay held, so during an event the view
doesn't move.

KH2's camera's own windows (`camera.rs`: the "was ours" 300 ms, the "untouched" 5 s that lets it
carry on when KH1's eye / target pair is as it wrote it, the 150 ms between KH1's follow calls, its
trace and jerk logs) are timed on the mod's clock (`frameadv::clock_ms`) since t480, not real time, so
a hold reads as no gap at all: before, a hold over 5 s (camera held) started KH2's camera over behind
Sora on the next step. Select while held stays blocked (the tester left it open; nothing changed).
**Assumption** (ours, not KH1's): no window there wants real time across a hold.

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
(through the thunk `0x1401E0C70`). Frame Advance called it (t371 to t446) with 1 when it held the game and 0 when it
let a frame run. (After t426) The second byte it writes, `+0xDB` (the decomp's `pbVar1[6]` from
`+0xD5`), stops the effect draw pass drawing the instance (`0x1401ECF30`, Checked (decomp)), so held
effects are only seen through a captured picture (KH1's pause, above), so Frame Advance no longer calls it (Live uses the system stop). KH1's other freezes call it too: the
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

## The info panel (tester, "Missing sleeves animation" m19)

While Frame Advance is on, a small box in the screen's top right corner (apart from the debug
menu's box, drawn whether the menu is open or not, in the same overlay pass at the end of the
frame's drawing; `fapanel.rs`, `debugmenu::draw_end`) shows:

- the animation drawn: KH2's motion, battle or field, with KH1's slot it stands for (and ours when
  it is one of our slots), or KH1's own animation / KH2's controller's pose;
- how it is drawn: baked (KH1 plays our baked motion), live (our pose written over KH1's), KH2's
  controller, or KH1's own;
- the frame within the animation of its length and how many times it has looped since it began,
  on a line labelled "Animation" ("Animation: field run, frame 12 / 40, loop 3"; before tNNN it
  read "Frame: field run 12 / 40   Loop 3", which the tester read as a "field frame run" count);
- the trace's frame number (the trace's `frame` column), so a stretch can be named "trace frames X
  to Y";
- the cloth physics' settings id and steps when it runs (`clothlive`).

The frame is the clip's own time (KH1's motion time `+0x16C` for our slots; our clock for KH2's
clips drawn with no slot), floored. A loop is counted when that time comes round by more than half
the length (Assumption: KH1 wraps a looping motion's time at its length; a time past the length is
also taken round). Cost: only while Frame Advance is on; a handful of short strings a frame.

## Seconds shown as Frames (tester, "Missing sleeves animation" m19)

With the Trace Recorder's Game Frames row on (Frame Advance turns it on), the clip's length row
reads "Frames" and shows the clip in game frames (the seconds x 60: the trace counts 60 Hz frames
then, `trace::game_frames`). Left / right still change it a second (60 frames) at a time, as the
ini keeps it in seconds (`trace_seconds`). With Game Frames off it reads "Seconds" as before.
