# Movement and moveset model

This covers what `character-core` does and where each number comes from. It uses the same
tags as `combat-model.md`:
- **Data**: read from your install at runtime.
- **Checked**: tested against your install.
- **Hypothesis**: our reading of how the original uses the data, exposed as data so it can
  be corrected.
- **Approximated**: not in any table; picked by us.

Units are KH2 world units (about 1 cm) and 60 Hz game frames.

## Where the data comes from

| Data | File (inside the `kh2_*.pkg` archives) | Notes |
|---|---|---|
| Walk/run speed, jump height, turn rate, accel/brake | `03system.bin` → `pref` → `prty`, pointer 1 | Sora: walk 2, run 8, jump 185, turn 0.262 rad (15°)/frame, accel/brake 0 (instant) |
| Terminal fall speed, lock-on ranges | `pref` → `sstm` | fall_max 16, lock-on 1200 / release 1500 |
| Growth abilities LV1–MAX | `pref` → `fmab` records 0–3 | High Jump 235/310/385/585; Dodge Roll invulnerability 10/16/24/32 frames; Glide speed 16/20/24/32; Quick Run 30–38 for 14–40 frames |
| Attack targeting | `pref` → `plyr`, pointer 1 | auto lock-on 300, attack origin height 60, approach speeds 8/6/12 |
| Every move's length and root motion | `obj/P_EX100.mset` (Sora) | ANB motion header (fps, frame range) + root-position F-curves |
| Move windows: combo input, finisher chain, turn-to-target, no gravity, invulnerable, jump frame | `obj/P_EX100.mset` motion triggers | |
| Keyblade hitbox windows (ATKP id, collision group) | `obj/W_EX010.mset` motion triggers, same slot | |
| Keyblade hitbox shapes | `obj/W_EX010.mdlx` collision (type 6) | spheres along the blade per group |
| Combo table | `00battle.bin` → `ptya`, pointer 1 | 37 rows for Sora |
| Damage per hit | `00battle.bin` → `atkp` | via `combat-core` |

## Evidence

- **Trigger frames are 60 Hz frames from the clip start.** Motions play at 30 fps, but their
  trigger frames run past the motion's own frame count. For example, the Dodge Roll is 26
  frames at 30 fps (52 game frames) and has triggers up to frame 34. A101 starts at local
  frame 95 but has a hitbox at frames 5–20, so trigger frames are relative to the clip start.
  **Checked.**
- **Turn-window parameters are degrees per frame.** The turn-to-lock windows carry 15, which
  matches `prty` turn_speed 0.262 rad = 15°. **Hypothesis.**
- **Root motion is real.** The Dodge Roll moves 438 units forward along +Z on a smooth curve.
  Sora's first ground swing (A300) lunges 92 units, and the dash attack (A302) 220. The Hermite
  tangents are scaled by segment length; OpenKH marks its own handling of them as uncertain.
  **Checked** (curve shape) / **Hypothesis** (exact tangent scaling).
- **Jump takeoff.** The jump motion's "Action: Jump" frame trigger is at frame 4, so there are
  4 grounded frames before the jump leaves the ground. **Data.**
- **Landing recovery.** The landing motion is 26 frames long. Its "allow reaction command"
  window opens at frame 8; we use that as the point where input is accepted again.
  **Hypothesis.**
- **Guard.** Motion 42 (A321) has a guard-type ATKP (162) window at frames 0–40 on the Keyblade.
  **Data.**

## Approximated (not in any table)

- **Gravity.** `0.547` units/frame², chosen so the 185-unit jump peaks when the rising
  animation (A003, 26 game frames) ends. `kh2-combat sora` shows takeoff on frame 4 and a 185.1-unit apex on frame 29.
  Override it with `--gravity`.
- **Glide descent.** Each frame the descent speed grows by `glide_fall_ratio × glide speed`,
  capped at `glide_fall_max`. With the table values, MAX glides almost level and Final Form
  (fmab record 4: ratio 0, max 0) flies level, which fits the game.
- **Dodge Roll and Quick Run cancels.** Other actions can interrupt from the roll's fall-check
  event (frame 34).
- **Hitbox placement.** There is no skeleton, so the Keyblade's spheres are swept horizontally
  in front of Sora while a hitbox window is live. The sphere sizes and spacing come from the
  data; the swing arc does not.
- **Enemy hurtbox and knockback.** Dummies are 50×110 cylinders; knockback is a short push.

## Combo selection (hypothesis)

The PTYA rows are data, but how the game chooses between them is in the executable. We
use this reading, which fits the patterns in Sora's 37 rows:
- **Air vs ground.** Rows with `jump > 0` are air attacks.
- **Finishers.** `flag & 4` marks a finisher. Every finisher motion (A303, A304, A332, A333)
  carries a "finisher chain" window instead of a "combo" window.
- **Target requirement.**
  - `kind` 1 or 2: needs a target within `[near, far]` and within the height window, measured
    from `attack_y_offset` above the feet.
  - `kind` 0 or 3: no requirement.
- **Picking a row.** The highest `score` wins. On a tie, a row whose requirement is met beats
  a fallback row.
- **Combo length.** The base combo is 3 hits (two hits, then a finisher). Combo Plus adds one;
  Negative Combo removes one.
- **Input buffering.** A press at any point during a swing is buffered. The next hit starts on
  the first frame of the combo window. Finishing Plus chains another finisher from the
  finisher window.

What this produces on your install (from `kh2-combat sora`):

| Situation | Sequence |
|---|---|
| No target | A300 → A300 → A304 (finisher) |
| Target 120 ahead | A301 → A301 → A303 |
| Target 250 ahead | A302 (dash) → A301 → A303 |
| Air | A330 → A330 → A332 |

Open questions: the meaning of `sub` (it looks like a chain link: air row 9 → row 10 =
A330 → A331), bits 1 and 8 of `flag`, and `combo_offset`.
