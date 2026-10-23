# KH2's pad sticks (and KH1's)

How KH2 turns the pad's stick bytes into the sticks its camera and player read, and how KH1's
PC pad step compares. Code: `character_core::pad`. Plain-language notes from the Steam decomps.

## KH2 (Steam)

- **Dead zone.** `0x14039b7f0` (pad start-up) sets the stick dead zone to 60 (`0x140bf3338`),
  and the key-repeat delays 30 and 4. Checked (decomp).
- **Sticks.** `0x14039c180` runs once per pad record each frame, called from the pad task
  `0x14039bbe0` (and `0x14039c530`). Checked (decomp):
  - The record's bytes 2-3 are the right stick, 4-5 the left (the PS2 order).
  - Each axis: the byte minus 127.5. Toward the centre by 60; an axis within 60 of the
    centre is 0. Then divided by 67.5 (127.5 - 60).
  - The pair is normalized in place (`0x140142080` returns the length and leaves the vector
    unit). The pad keeps the unit direction (`+0x20` / `+0x24` right, `+0x30` / `+0x34` left)
    and the length clamped to 1 (`+0x2c` right, `+0x3c` left).
  - y is positive pulled down.
  - Direction bits: each axis past its dead zone sets a bit in the held word (right stick
    bits 16-19, left 20-23). KH2's logical pad (`0x140bf31a0`, built by `0x14039c720`) remaps
    the record's bits through tables in the exe's data, so which logical bit is which isn't
    readable from the decomp. **Assumption:** the camera's "turn left / right" and "zoom in /
    out" masks (`0x14071aa98`, `0x14071aa88`) are the right stick's x and y direction bits.
  - Mouse: see "The mouse camera and the controller camera" below.
- **Who reads them.** The follow camera (`0x140165110`), the lock-on camera (`0x140176c70`),
  the first-person view (`0x140165e30`), and about 100 other functions that take pad 0 (`0x14039b780`).

## KH2: the mouse camera and the controller camera

KH2 PC has one camera, with a few mouse-only and keyboard / mouse-only branches in it. With a
controller none of them runs. Checked (decomp), Steam KH2:

- **Input mode.** `0x142b44a88` (read by `0x1404e5c50`) is 1 while the player uses the keyboard
  and mouse, 0 with a controller. The pad task `0x14039bbe0` switches it on whichever was used
  last (`0x1404e7bb0` / `0x1404e89d0`). **Inferred** from that switch.
- **The mouse as the right stick** (`0x14039c180`, every frame): when the mouse moved this frame
  (`0x1404e51f0`: the mouse is on, `0x142b44a8c`, and its record's `+0x29` is 0xFF), its motion
  (x · 24, y · 20, each clamped to ±1, y flipped) is written over the right stick's bytes. That
  frame the dead zone is 1, not 60, for **both** sticks, and both sticks' tilts are multiplied
  by `1 + s · 0.02`, where `s` is a short in the PC settings (`0x140715364 + 0x24`).
  **Assumption:** `s` is the mouse sensitivity option.
- **The follow camera's speeds** (`0x140166100`, each frame before the mode's update): the turn
  speed is the exe constant `0x140718bf0` and the zoom factor 1. Only when the mouse moved this
  frame they become `max(|dx · 10|, 1) · 3°` and `max(|dy · 10|, 1)`. The follow camera turns
  by `tilt · speed · step` and zooms by `tilt · 8 · factor · step` (`0x140165110`,
  `0x1401654d0`).
- **The lock-on camera** turns by `tilt · 0x14071a470 · step` and zooms by `tilt · 8 · step`
  (`0x140176c70`, `0x140176bc0`): no mouse factor.
- **The camera's lag** (`0x140165700`): the look point and eye spring at `0.07 + (run − 8) ·
  0.0006` (halved at 60 fps). In keyboard / mouse mode it is `0.4 + (run − 8) · 0.0018`: a much
  tighter follow.
- **The swing behind** (`0x140165110`) has no mouse branch. It stops while the right stick's
  tilt isn't 0, so a moving mouse stops it too.
- **Not mouse-only:** the dead zone 60, the swing's rate (`0x140718be8` / `0x140718bec`), the
  reset, the cut, the walls.

The controller's turn constant `0x140718bf0` is exe data (not in the decomp). Ours, 3° a frame
at full tilt, is the OG rewrite's reading of the Epic exe ("Checked (exe)" in
`camera-model.md`).

### Ours against it (checked t380-t384 for mouse leaks)

- The camera and the movement read the pads' own stick bytes (`takeover::RAW_STICKS`, taken in
  `probe::pad` as the pad record is read; **Inferred:** before KH1's stick step `0x14028b4f0`
  writes the mouse into the record, which the pad read rewrites each frame). That is the
  controller path: dead zone 60, KH2's controller turn and zoom speeds, the 0.07 lag. No mouse
  branch is in it. t380 (dead zone), t381 (movement), t382 (whole tilt), t383 (reset, cut) and
  t384 (the form's run speed) are all controller-path numbers.
- Only when the pads' sticks rest do we read KH1's processed sticks (the keyboard's movement
  keys; the mouse). KH1's stick step (`0x14028b4f0`) has KH2's mouse branch too: the mouse over
  the right stick, dead zone 1 for both sticks while it moves (KH1 has no sensitivity factor on
  the tilt). So with the mouse, ours turns at the controller's speed by the mouse's tilt.
- Not modelled (the mouse camera): the turn and zoom speed by the mouse's motion, the
  sensitivity factor on the tilt, the 0.4 lag in keyboard / mouse mode, and the mouse winning
  over a tilted pad stick (ours takes the pad's when it is tilted).

## KH1 (Steam)

- `0x14028b4f0`, from KH1's pad step `0x14028b0b0`, builds its sticks the same way: byte minus
  127.5, a dead zone of 60 (`0x14298cb00`, set in `0x14028afc0`), divided by 67.5, the
  length clamped to 1, and the mouse path with a dead zone of 1. KH1 stores the unit
  direction times the tilt. Its processed right / left sticks (`0x14298cac0` / `0x14298cad0`)
  are therefore KH2's sticks as one vector. Checked (decomp).

## Ours

- `character_core::pad::stick` gives KH2's unit direction and tilt from two pad bytes;
  `stick_vec` the two as one vector (KH1's form).
- KH2's camera (`kh1-hook` `camera.rs`) reads the pads' own bytes this way, and KH1's
  processed sticks when the pads rest (the keyboard; the mouse). Before this it used a round
  15% dead zone, at an earlier request so gentle tilts would move; the tester asked for 1:1.
- KH2's movement (`takeover::move_tilt`, the tilt KH2's controller walks and runs by, and
  `stick_dir`'s fallback direction) reads the left stick the same way. Before this it used a
  round 20% dead zone on the length (labelled KH2's; it wasn't). Still open: which tilt KH2's
  player takes as a run (`character-core`'s `run_threshold`, 50%, a hypothesis), and the
  controller's own `DEAD_ZONE` (a tilt of 0.1) on top of the pad's.
- Not modelled: KH2's mouse camera (above); KH1's mouse still goes through KH1's processed
  right stick.
