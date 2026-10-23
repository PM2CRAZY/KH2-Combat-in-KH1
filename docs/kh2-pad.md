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

## KH2: bit 32 of the held word (the "unknown button")

What it does (Checked (decomp), Steam KH2):

- **The swing behind.** `0x140178ae0` returns bit 32 of the held word, inverted unless the
  Field Camera option is Auto (`0x1402e5310`: bit 4 of the config word `+0x41a4` clear; the
  options menu's first row, `0x1403651a0` / `0x140365580`, whose rows match KH2's Config:
  Field Camera, the camera's up / down and left / right, a three-way row (Summon Effects),
  the map, vibration (`0x1402e53f0`, used by the pad task's rumble), ...). The follow camera
  (`0x140165110`) and one other mode (`0x140165570`) clear their swing flag when it is true.
  So with Auto, holding the button stops the swing behind; with Manual, the camera swings
  behind only while it is held.
- **Zoom while swimming.** The follow camera zooms unless the player's kind has bit 3
  (`0x1403babf0`); then only with bit 32 held (`0x140165110`, `0x140165570`, the lock-on and
  other modes). The same kind bit sends his move (`0x1403a8fa0`) through a branch where two
  other held bits (29 / 30) move him up and down at his `+0x13c`, and bit 32 held cancels that.
  The first person doesn't pitch for that kind either (`0x140165e30`). **Inferred:** that kind
  is a swimming player (KH2's Atlantica: `prty` moveset 8 has an up_down_speed of 8 and
  swimming inertia, the others 0), whose right stick's up / down swims; holding the button
  makes it zoom instead.

Which button it is: **R2** (the tester, from KH2 itself: "its R2"). The decomp can't show it:

- The held word is the pad record's raw word put through the remap table `0x1405c3420`
  (exe data, `{raw mask, logical bits}` pairs, ended by 0; `0x14039c720`) and a PC table built
  at run time (`0x140bf3330`). Neither is in the decomp.
- The raw word (`0x14039bbe0`): bits 0-15 the PS2 buttons in PS2 order. Checked (decomp): the
  XInput read `0x140134d10` maps A to Cross 0x4000, B to Circle 0x2000, X to Square 0x8000, Y
  to Triangle 0x1000, LB / RB to L1 0x400 / R1 0x800, LT / RT to L2 0x100 / R2 0x200, Back to
  Select 1, Start 8, the stick clicks to L3 2 / R3 4, the d-pad to 0x10-0x80, and two more
  buttons to 0x10000 / 0x20000; the table's first entry is 0xF09 (Select + Start + L1 + R1 +
  L2 + R2, the soft reset). Bits 16-23 the stick directions (`0x14039c180`), 24 / 25 two PC
  button masks set at run time (`0x140782740` / `0x140782744`), and 26 / 27 "L2 held" / "L2 not
  held", swapped by bit 5 of the same config word (`0x1402e53d0`).
- KH1's logical word is another layout (its table `0x140528F90`: R3 to 0x200, Select to
  0x8000, `kh1-camera.md` 5.2), so KH1's doesn't tell KH2's.
- Other logical bits the camera reads, for the record: pressed 17 (the reset; **Assumption:**
  R3), pressed 22 (first person; **Assumption:** Select), held 18 / 19 (the first person's
  pitch, with the right stick's y), held 20 / 21 (the first person's turn, `0x1403a9250`).
- (Our earlier guess was L2, through raw bit 26 / 27. Wrong: the tester says R2.)

### Ours (R2)

- `kh2-camera` `Controls::hold`: the follow camera swings behind only when `auto != hold`
  (Auto: not while R2 is held; Manual: only while it is held), as `0x140178ae0` /
  `0x140165110`. The lock-on camera has no swing, so R2 + the right stick switching targets
  there is unaffected.
- `kh1-hook` `camera::r2_held`: on Steam, bit `0x200` of KH1's raw pad word (`0x14298C9B4`,
  slot 0's held word, PS2 order, built by KH1's pad step `0x14028b0b0` from its input object's
  buttons, `+0x18`, the pads and the keyboard; it is the word KH1's button remap reads).
  Checked (decomp, and the remap table read from the exe: raw R2 `0x200` -> logical
  `0x400000`, which raw L2 `0x100` also sets, so KH1's logical word can't tell R2 alone). Other
  builds: the pads' own records (`takeover::RAW_BUTTONS`). **Assumption:** the keyboard's R2
  key reaches the raw word (KH1's input object merges the keyboard into `+0x18`; not traced
  through `0x1400D7E80`).
- Frame Advance keeps R2: while it is on `probe::pad` takes R2 off every pad record and
  `RAW_BUTTONS`, so the camera doesn't see it then.
- Not modelled: zoom while swimming with R2 held (KH2's swimming player kind). In KH1, water
  (state 24) is a KH1-only state, which keeps KH1's own camera.
- Trace: `kh2cam_hold` (1 while R2 held, as the camera read it).

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
  round 20% dead zone on the length (labelled KH2's; it wasn't).
- **Walk / run, 1:1** (Checked (decomp), Steam KH2). The player's controller copies the pad's
  left stick (direction and tilt) as it is (`0x1403b2340`, into its `+0xb50`). His move
  direction and speed (`0x1403a8fa0`, `0x1403aa700`): no stick when the tilt is 0 (he keeps his
  facing, speed 0). Otherwise the stick through the camera (`0x1403a7fb0`), at his walk speed
  (`+0x128`, `prty` walk_speed) when the tilt is under 0.8, and his run speed (`+0x12C`) from
  0.8. The two other reads of the tilt (`0x1404034b0`, `0x140403d70`) test it against 0 too.
  So there is no dead zone past the pad's, and no speed between walk and run.
  - Ours: `character-core`'s `run_threshold` is 0.8 (it was 0.5, a hypothesis), and its
    `DEAD_ZONE` is 0 (it was 0.1, ours): every tilt past the pad's dead zone walks. The ledge
    hang keeps its own 0.1 for its climb / let-go pushes (`HANG_PUSH`, ours by an earlier
    request).
  - KH1 also runs from 0.8 (`kh1phys::RUN_TILT`), so base Sora in KH1's states already split
    there.
  - Not changed: `takeover`'s 0.15 tilt tests (`moving`, KH2 taking him after a KH1 move),
    which are our hand-over rules, not KH2's player.
  - Also seen (not ours to model): a debug free-move branch of `0x1403a8fa0` (bit 3 of the
    player's kind) doubles its speed in keyboard / mouse mode.
- Not modelled: KH2's mouse camera (above); KH1's mouse still goes through KH1's processed
  right stick.
