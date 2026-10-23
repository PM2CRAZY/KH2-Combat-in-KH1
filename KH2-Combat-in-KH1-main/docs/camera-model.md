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
  - **Each axis on its own.** The camera turns only while the stick's digital left or right
    bit is set, and zooms only while its up or down bit is set. A bit is set when that raw
    axis byte is below 0x40 or above 0xC0, so an axis does nothing until past half tilt.
    Then the speed is the stick's tilt. **Inferred:** the value it scales by is the tilt.
  - **Options.** The Config's camera options swap those bit pairs. They are bits of a save
    word: bit 7 swaps left and right, bit 8 swaps up and down (zoom). **Inferred:** the names
    (the game's Camera Controls, Horizontal and Vertical).
  - **Mouse (PC).** While the mouse moves, its movement sets the speeds: the turn from the
    horizontal motion, the zoom from the vertical (clamped to 1).
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

## In KH1 (`kh1-hook`, `camera.rs`)

KH2's camera replaces KH1's follow and lock-on cameras, and it reads the right stick and the
mouse itself. KH1's side is in `docs/kh1-camera.md` (5.1, 5.3, 5.4).

- **KH1's camera options (Checked, KH1 exe).** The Config menu's camera up/down and
  left/right rows, Normal or Reverse. They are kept in the save's config block (`+0x14`,
  `+0x18`), one save each. KH1's camera multiplies its input by two floats: `0x140507AA8`
  (pitch, −1 Normal) and `0x140507AAC` (yaw, +1 Normal). The Config setter writes them at
  once, and so do a load and a new game. They apply to the pad and the mouse alike, because
  KH1 PC writes the mouse into the right stick. There is no separate mouse invert option.
- **Ours.** KH2's camera reads the two floats each frame, so a change in the Config menu
  counts at once. Reverse flips that axis for the pad and the mouse. KH1's Normal up/down
  is KH2's: stick up lowers the camera and looks up (KH1 by its pitch, KH2 by zooming in).
  The log prints the options at the start and on each change.
- **Mouse sensitivity (Checked, KH1 exe).** It is in KH1 PC's own settings, not the save:
  0..40, default 20. KH1's camera turns at most `2.5° + 0.0875° · sensitivity` a frame with
  the mouse, and 2.5° with the pad. KH1's Config has no camera speed option.
  - **Ours:** the mouse speed in KH2's camera is in the same proportion, normalised to 1 at
    the default 20 (0.59 at 0, 1.41 at 40).
- **Right stick, each axis on its own.** This fixes the player's report of the camera looking
  at the sky at random times.
  - Before: one round dead zone (15%) let any slant of the stick while turning zoom the
    camera in. At its closest, the eye is 20 above the feet, looking up.
  - Up/down now follows KH2's rule: nothing until past half tilt, then the tilt. This also
    applies to the mouse, since mouse movement across always moves a little up or down.
  - **Ours:** across keeps a 15% dead zone, rescaled, so gentle tilts still turn (the user).
    KH2's half-tilt rule there made gentle tilts do nothing.
- **Look-up limit (Ours).** The written view never looks up more than KH2's free camera can:
  31°, its eye at the closest distance (150 below the look point, 250 away). A steeper view
  lowers the look point and keeps the eye where it is. That happens when a slope or wall pulls
  the eye in low behind Sora, when the look point rises ahead of the eye in a high jump, or
  when a lock-on target is overhead. The log notes when the limit starts holding the view, and
  what zoomed the camera all the way in (the stick or the mouse, with the raw stick bytes).

- **Scenes and room exits (Ours, `docs/kh1-camera.md` 9.3).** When KH1's own camera takes
  the view from KH2's in a scene, KH1 blends from KH2's view over 30 frames with its own
  mode-switch blend, and the field of view eases from KH2's 69.9° vertical to KH1's 48° with
  it (both used to snap). Through a room exit's scene (Sora walked into an exit's trigger),
  KH2's camera stays on the follow camera until KH1 holds its camera for the door camera or
  the room changes, as KH1's follow camera does in KH1. The next room starts KH2's camera
  behind Sora, the way he walked in.

## Not modelled

- The first-person look, cutscene camera modes and per-room overrides.
- The camera-collision volumes some rooms have (only the background ray is used).
- The soft auto-target camera (only the lock-on moves the camera).
