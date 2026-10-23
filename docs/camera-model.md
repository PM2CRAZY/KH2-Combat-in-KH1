# Camera model

The playground's camera is KH2's field camera, written from plain-language notes on the
executable (crate `kh2-camera`). Units are game units and 60 Hz frames.

Confidence labels:
- **Checked (exe)**: read from the executable's code.
- **Inferred**: reasoned from it.
- **Ours**: our choice.

## Parameters (Checked, exe)

The game keeps one row of numbers per kind of area. Row 0 is normal field and battle areas:

| | Row 0 (field) | Rows 1–3 (large rooms) |
|---|---|---|
| Horizontal field of view (4:3 frame) | 1.5 rad (69.9° vertical) | 60° |
| Default distance | 420 | 600 |
| Lock-on / reset distance | 380 | 520 |
| Distance range | 250..500 | 400..700 |
| Height coefficient `k` | 0.008 | 0.005 |

The near plane is 1 unit. The playground keeps the game's **vertical** field of view on its
16:9 window. **Inferred:** the PC version widens horizontally.

## Placement (Checked, exe)

- **Look point:** 170 above the feet.
- **Eye:** at horizontal distance `dist` from the look point, at yaw `yaw`. Its height above
  the look point is `k (dist − 250)² − 150`.

There is no pitch control: zooming in lowers the camera along this curve.

| Distance | Eye above the feet | Pitch |
|---|---|---|
| 250 | 20 | looks up 31° |
| 380 | 155 | about level |
| 420 (default) | 251 | looks down 11° |
| 500 | 520 | looks down 35° |

**Turning.** The yaw turns toward its goal at most 10° per frame.

## Free camera (Checked, exe)

- **Right stick:** turns 3° per frame at full tilt, and zooms 8 units per frame.
  - **Checked (decomp)**, Steam follow `0x140165110`, lock-on `0x140176c70`, zoom
    `0x1401654d0` / `0x140176bc0`: the turn and zoom are gated by the stick's direction bits
    (an axis past its dead zone; `0x140178ab0` / `0x140178b10` turn, `0x140178b50` /
    `0x140178b80` zoom, each pair swapped by an option bit, `0x1402e5330` horizontal,
    `0x1402e5350` vertical) and move by the stick's whole tilt (`+0x2c`), not by that axis.
    So a full diagonal turns 3° and zooms 8 a frame together.
  - **Assumption:** the masks those test (`0x14071aa98`, `0x14071aa88`, exe data) are the
    right stick's x / y direction bits.
  - The follow camera zooms after its reset too; the lock-on camera turns and zooms, then a
    reset sets its goal and distance.
  - Not known (exe data): the lock-on camera's own turn speed (`0x14071a470`); ours uses 3°.
  - **Ours:** right turns the view right. The mirrored world makes the game's own sign
    meaningless here.
- **Reset** (R3, or F on the keyboard): goal yaw = behind Sora, distance = default. It is a
  hard cut if the left stick is held.
  - **Checked (decomp)**, Steam follow `0x140165110`: the reset also puts the rendered eye on
    its last goal, so only the turn eases (10° a frame). With the left stick tilted it calls
    the placement `0x140164520` with its cut flag: the yaw is set to its goal, and
    `0x140164d70` sets the look point and eye to their goals, with no spring.
    **Inferred:** the camera's `+0x8` / `+0x18` are the rendered look point and eye (`+0x84` /
    `+0x64` their goals), from that copy.
  - The lock-on camera's reset (`0x140176c70`) sets the goal yaw and the lock distance only.
- **Swing behind.** It happens only when all of these hold:
  - the camera's auto mode is on (`--manual-camera` turns it off)
  - Sora moved this frame
  - the left stick is tilted and isn't pulled straight back
  - the right stick is idle

  Then the yaw turns toward "behind Sora" by
  `2 (|x| + 1)(|y| + 1) · tilt · (0.0043633 + (run_speed − 8) · 0.00017453)` radians per
  frame (`x`, `y` = the stick's unit direction). That is about 1° per frame for Sora and faster for faster forms. Meanwhile the
  distance drifts back to 420 at 0.5 per frame.
- **"Pulled straight back", exactly** (Checked (decomp), Steam: the follow update
  `0x140165110`, the same test in `0x1401642f0`; the pad's sticks are built in `0x14039c180`).
  - `x`, `y` are the pad's left-stick **unit direction** and `tilt` its length, at most 1.
    Each axis is the stick's byte minus 127.5, with a 60 dead zone taken off it and divided by
    67.5; `0x14039c180` then normalizes the pair in place (`0x140142080` returns the length
    and leaves the vector unit) and keeps the length, clamped to 1, beside it. (t373 read the
    pair as the dead-zoned components; corrected after a second read.)
  - The pad's `y` is positive when the stick is pulled down (toward the camera). The swing
    is skipped when `y` is 0.9 or more: the stick points within 25.8° of straight back, at
    any tilt. A part tilt straight back doesn't swing either.
  - More than 25.8° off straight back (diagonally back) the camera still swings, at the rate
    above with the unit direction's components. Sora's facing follows the stick through the
    camera, so in KH2 too this slowly curves his run.
  - "Sora moved" is his position changing in x or z since the previous frame
    (`0x1403b8940`).
  - The swing also needs the right stick's tilt to be exactly 0 and the camera's swing flag
    (`+0x42`), which is cleared by a pad bit (bit 32 of the held word, inverted by an option
    bit, `0x140178ae0` / `0x1402e5310`) whose button isn't known (the remap table is data).
  - The crate's stick has y forward, so its test is `−y < 0.9` on the stick's unit
    direction. A host has to pass y forward. KH1's stick, like KH2's pad, has y positive
    pulled back, so `kh1-hook` negates it.
- **Smoothing.** The rendered look point and eye follow their goals with the game's spring:
  - the look point closes about 16% of its gap per frame
  - the eye about 9%
  - a ~1-unit dead band
- **Walls.** A ray from Sora's chest to the eye (before and after smoothing). If it hits the
  map, the eye goes to the hit point plus 5 along the surface normal.

## Lock-on camera (Checked, exe)

- **Look point:** 30% of the way from Sora's chest (170 up) to the target's camera point
  (its type-7 collision shape, else its lock-on shape, type 3). The look point stays at the
  chest when the target is more than 300 away and steeply above.
- **Eye:** placed from Sora's chest.
- **No rotation** while the target and Sora (100 above his feet) are both on screen. The test
  uses the game's 512×416 frame with 32-pixel side and 8-pixel top/bottom margins.
- **When either leaves the screen:** the goal yaw moves the camera onto the line from the
  look point through Sora, i.e. behind him facing the target. A target escaping to a side
  adds 20°.
  - **Inferred:** the 20° is in the direction the camera is already turning (an overshoot
    that pulls it back in).
- **Distance:** drifts to 380 at 1 per frame. It shrinks 2 per frame while the target or Sora
  sits at the bottom edge.
- Manual turning, zoom and reset (to 380) still work.

## Lock-on (Checked, exe)

- **R1 (Tab) toggles.** The nearest target within `lock_on_range` (1200, sstm) is chosen.
  There is no facing preference.
- **Release:** beyond `lock_on_release_range` (1500), on the target's death, or R1 again.
- **Switching:** R2 + right stick (Z / C on the keyboard). The next target around Sora in
  that turning direction wins: the smallest positive change of angle from the current target.

## Not modelled

- The first-person look, cutscene camera modes and per-room overrides.
- The camera-collision volumes some rooms have (only the background ray is used).
- The soft auto-target camera (only the lock-on moves the camera).
