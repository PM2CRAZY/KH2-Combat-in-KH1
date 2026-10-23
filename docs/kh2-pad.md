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
  - Mouse: while the mouse moved this frame (`0x1404e51f0`), its motion is written as the
    right stick's bytes and the dead zone is 1 for both sticks that frame; the tilt is scaled
    by the wheel. The camera's turn and zoom speeds are also scaled by the mouse's motion
    (`0x140166100`: `max(|dx · 10|, 1) · 3°` a frame, zoom `max(|dy · 10|, 1)`).
- **Who reads them.** The follow camera (`0x140165110`), the lock-on camera (`0x140176c70`),
  the first-person view (`0x140165e30`), and about 100 other functions that take pad 0 (`0x14039b780`).

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
- Not modelled: the mouse's speed scaling of the camera's turn and zoom (KH1's mouse still
  goes through KH1's processed right stick).
