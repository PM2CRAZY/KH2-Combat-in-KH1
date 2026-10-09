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
  - **Ours:** right turns the view right. The mirrored world makes the game's own sign
    meaningless here.
- **Reset** (R3, or F on the keyboard): goal yaw = behind Sora, distance = default. It is a
  hard cut if the left stick is held.
- **Swing behind.** It happens only when all of these hold:
  - the camera's auto mode is on (`--manual-camera` turns it off)
  - Sora moved this frame
  - the left stick is tilted and isn't pulled straight back
  - the right stick is idle

  Then the yaw turns toward "behind Sora" by
  `2 (|x| + 1)(|y| + 1) · tilt · (0.0043633 + (run_speed − 8) · 0.00017453)` radians per
  frame (`x`, `y` = the stick's unit direction). That is about 1° per frame for Sora and
  faster for faster forms. Meanwhile the distance drifts back to 420 at 0.5 per frame.
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
