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
  `s` is the mouse sensitivity setting (Checked (decomp): KH1's camera reads the same field as
  its mouse sensitivity, see "Ours: KH2's mouse camera").
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

The controller's turn constant `0x140718bf0` is exe data (not in the decomp): 0.0523599, 3° a frame at full tilt, read from the tester's Steam KH2 exe (SHA-256 `9002b2de...49ed`, **Checked (exe)**), the same as ours (the OG rewrite's reading of the Epic exe).

### Ours against it (checked t380-t384 for mouse leaks)

- The camera and the movement read the pads' own stick bytes (`takeover::RAW_STICKS`, taken in
  `probe::pad` as the pad record is read; **Inferred:** before KH1's stick step `0x14028b4f0`
  writes the mouse into the record, which the pad read rewrites each frame). That is the
  controller path: dead zone 60, KH2's controller turn and zoom speeds, the 0.07 lag. No mouse
  branch is in it. t380 (dead zone), t381 (movement), t382 (whole tilt), t383 (reset, cut) and
  t384 (the form's run speed) are all controller-path numbers.
- Only when the pads' sticks rest do we read KH1's processed sticks (the keyboard's movement
  keys).

### Ours: KH2's mouse camera (the tester: "(b)")

KH1 PC has the same mouse code as KH2 PC (the same port), so KH2's mouse camera runs on KH1's
own mouse. Checked (decomp), Steam KH1:

- `0x140138b30` is KH2's `0x1404e51f0` line for line: the mouse is on (`0x1422DB3EC`), its
  record (`0x1421AAF18 + 0x23c`, `+0x29` 0xFF) has moved: `+8` and minus `+0xc`, each held to
  ±1, and the sensitivity, the u16 at `0x1404D8614 + 0x24` (`0x14012af20`). KH2 reads the same
  offset of its own settings block (`0x14014cde0 + 0x24`), and KH1's camera uses it as the mouse
  sensitivity (below), which settles the earlier **Assumption** about KH2's `s`.
- `0x14028b4f0` (KH1's stick step) is KH2's `0x14039c180` but for the sensitivity factor: in a
  mouse frame the right stick's bytes are the mouse (x · 24, y · 20, held to ±1, y flipped) and
  the dead zone is 1 for both sticks.
- `0x1422DB3E8` (`0x140139590`) is KH1's keyboard / mouse mode, switched by the last device used
  (`0x14013a2c0`: 1 after the keyboard or mouse, 0 after a pad button), as KH2's `0x142b44a88`.

What ours does now (`camera.rs` `mouse` / `sticks`, the `kh2-camera` crate's `Controls::mouse`
/ `Controls::kbm`; Steam only, frameadv's pattern; other builds keep the controller path):

- In a frame the mouse moved (KH1's `0x140138b30` returns 1, its prologue checked first), both
  sticks are KH1's processed ones (the mouse over the right stick, the dead zone 1: KH2's), times
  KH2's `1 + s · 0.02`, and they win over a tilted pad stick, as in KH2.
- The follow camera turns at `max(|10 x|, 1) · 3°` a frame times the tilt, and zooms at
  `8 · max(|10 y|, 1)` times the tilt (`0x140166100`); the lock-on camera keeps 3° and 8.
- In keyboard / mouse mode the look point and eye spring at `0.4 + (run − 8) · 0.0018`
  (halved at 60 fps), every mode, the first person too (`0x140165700`).
- The tilt is no longer held to 1 in the crate (KH2's `+0x2c` carries the factor).
- Trace: `kh2cam_mouse_x`, `kh2cam_mouse_y` (blank when it didn't move), `kh2cam_kbm`.
- The mouse turns Sora in KH2's first person (the tester: "check the decomp"; below).
- Not done: KH2's mouse-frame dead zone and factor on the left stick for his movement
  (`takeover::move_tilt`).
- 30 fps (the tester: "(a)"): KH2 springs once a frame, at half the rate when its frame step
  is under 2 and the whole rate from 2 up (`0x140165700`, Checked (decomp)). Ours did two half
  springs at step 2 (75% of the gap a frame with the 0.4 lag, where KH2 closes 2.5 · 0.4, all of
  it); it now does KH2's one, with the pad's 0.07 too (17.5% a frame, from about 16.7%). See
  `camera-model.md`, "Smoothing".

### The mouse in KH2's first person

Checked (decomp), Steam KH2:

- The player's first-person turn (`0x1403a9250`, called from his standing states) is 3° a
  frame times the frame step on held bits 20 / 21, whatever the tilt and with no mouse factor.
  The pad sets the stick direction bits after it writes the mouse over the right stick's bytes
  (`0x14039c180`, dead zone 1 that frame), so in a frame the mouse moved the bits are the
  mouse's. With the tester's reading that KH2's first-person turn is on the right stick (m7),
  moving the mouse left / right turns him at a steady 3° a frame while it moves (any motion
  past 1 of 127.5 after the x · 24); moving it only up / down doesn't turn him.
  **Assumption:** bits 20 / 21 of the logical word are the right stick's left / right (the
  remap table is exe data).
- The look up / down (`0x140165e30`) reads the right stick's unit y and tilt, so the mouse's
  up / down pitches the view, with the tilt's sensitivity factor (ours already did: the
  camera's look stick is the mouse in a mouse frame).

Ours (Steam only):

- `camera::first_person_mouse_x`: in KH2's first person, when KH1's mouse function
  (`0x140138b30`) says it moved, KH1's processed right stick's x (`0x14298cac0`; KH1's stick
  step `0x14028b4f0` writes the mouse in the same way, dead zone 1).
- Drive Forms (KH2's controller): `takeover::first_person_turn` turns by that x's sign, else
  by the pads' right stick as before.
- Base Sora (KH1's states): KH1's own first-person turn (`0x14028c7e0` in camera mode 2, 3° a
  frame times its processed left stick's x `0x14298cad0`, read inside its control step
  `0x1402a6220`; Checked (decomp)) gets a full left stick toward the mouse's x for that step
  (`takeover::first_person_mouse_turn`), put back after. `probe::pad` can't do it: it runs on
  the pad records before KH1 reads the mouse that frame (`0x1400da9f0` reads the pads first,
  then the cursor and the mouse record).

### KH1's own mouse camera against KH2's (the tester: "they feel pretty close")

KH1's follow camera's stick step (Steam KH1 `0x1401e7320`, Checked (decomp)), `k` = 0.5 at 60
fps, 1 at 30:

| | KH1 (its follow camera) | KH2 (follow camera) |
|---|---|---|
| Pad turn | a velocity: + 0.15° · x · k a frame, at most 2.5°, decays 0.24° · k a frame | 3° · tilt a frame, no ramp |
| Keyboard / mouse mode turn | + 0.3° · x · k a frame, at most `2.5° + 0.0875° · s`, decays 4° · k; stops at once when x is 0 or reverses | 3° · tilt a frame |
| Mouse moving | the velocity times `max(|10 x|, 1)` after its cap, every frame | the speed `max(|10 x|, 1) · 3°` |
| Sensitivity `s` | raises the cap | the tilt times `1 + 0.02 s` |
| Mouse up / down | pitch, same velocity rule but no motion factor, -15°..50° | zoom, `8 · max(|10 y|, 1)` · tilt |
| Mouse as the right stick | x · 24, y · 20, dead zone 1 | the same |
| Follow lag | KH1's own spring | 0.07 (pad) / 0.4 (keyboard / mouse) |

So a fast mouse turns KH1's camera at about `2.5° · max(|10 x|, 1)` a frame (at `s` 0) and
KH2's at `3° · max(|10 x|, 1)` · tilt: the same motion factor on nearly the same base, which is
why they feel close. They differ in KH1's ramp (a slow mouse eases in: at 60 fps it takes up
to about 17 frames to reach the cap), its instant stop, the sensitivity (KH1's raises the cap,
KH2's scales the tilt), and the vertical axis (KH1 pitches, KH2 zooms).

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

## L2 and the right stick

KH2: while L2 is held, the right stick doesn't move the camera (it works as the d-pad) and R3
doesn't reset it (thread `kh2-camera-fixes`, from KH2's behaviour).

- What the decomp shows: the camera's turn and zoom tests read logical bits through the masks
  at `0x14071aa98` / `0x14071aa88` (`0x140178ab0` / `0x140178b10`, `0x140178b50` /
  `0x140178b80`; Checked (exe): 0x200000 / 0x100000 and 0x40000 / 0x80000 / 0x200000 /
  0x100000), and the reset is pressed bit 17 (`0x140165570`). The raw word carries "L2 held" /
  "L2 not held" as bits 26 / 27 (`0x14039bbe0`). The static remap table (`0x1405c3420`, read
  from the exe: the soft reset, Start, the two PC masks, and the d-pad and stick-direction bits
  onto the d-pad bits) has no camera entries, so the camera's bits, and with them the L2 rule,
  come from the PC table built at run time (`0x140bf3330`, `0x14039c720`). **Assumption:**
  that table pairs the right stick's directions and R3 with "L2 not held" (bit 27), which is
  how L2 stops them; not read (it exists only while KH2 runs).

### Ours (the L2 Camera Hold row)

- The Config menu's Camera list: "L2 Camera Hold: Off / On" under Camera Style (`config.rs`,
  `Row::L2Hold`), kept in the ini as `l2_camera_hold` (default off: the camera as before).
- `kh1-hook` `camera::l2_held`: read exactly as `r2_held` reads R2. Steam: bit `0x100` of KH1's
  raw pad word (`0x14298C9B4`, PS2 order); other builds the pads' own records
  (`takeover::RAW_BUTTONS`). KH1's own remap gives L2 and R2 the same logical bit
  (`0x400000`), so the raw word is the only place L2 alone shows.
- With the row on and L2 held, KH2's follow and lock-on cameras get no right stick (turn and
  zoom), no mouse motion (KH2's pad puts the mouse on the same stick) and no R3 reset. KH2's
  first person (Select) is not affected. The right stick isn't turned into the d-pad (this
  version only stops the camera).
- In KH1, L2 does nothing for the camera; pressing L2 (or R2) drops KH1's soft-target cursor
  (`0x1402B11B0`), which this doesn't touch. Frame Advance takes only R2 off the pad records,
  so L2 is always seen.

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
- KH2's mouse camera: "Ours: KH2's mouse camera" above (Steam).
