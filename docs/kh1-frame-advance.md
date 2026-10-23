# KH1 frame advance (t371)

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

Frame Advance writes the mask between frames (from the frame end, after the task list) while KH1's
pause machine is idle (`0x14232DFB8` = 0): 1 holds the next frame, 0 lets it run.

## Model objects

`0x1401ED6D0 (on)` marks every model object in use paused or not: 0xE0-byte entries, the first
entry's in-use byte (bit 0) at `0x142678312`; from that byte, `+0x43` bit 7 set and `+0x49` = 1 (or
cleared). The pause machine calls it with 1 as
its pause opens (through the thunk `0x1401E0C70`). Frame Advance calls it with 1 when it holds the
game and 0 when it lets a frame run. **Checked (decomp)** for what it writes; what the model pass
does with the flag is an **Assumption** (it keeps the model's animation still).

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
it. **Checked (decomp).** Not done in t371: taking the period key out of that buffer before KH1 maps
it (a hook on `0x1400D7E80`, whose first bytes need reading from the exe).
