# KH1 frame advance (t371, t372, t377, t378)

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
lets a frame run. **Checked (decomp)**; the tester's t371 test: effects held and stepped frame by
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
