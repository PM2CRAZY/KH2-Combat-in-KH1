# Movement and moveset model

This covers what `character-core` does and where each number comes from. The tags match
`combat-model.md`:
- **Data**: read from your install at runtime.
- **Checked**: tested against your install.
- **Checked (exe)**: read from your copy of the executable in Ghidra. Only plain-language
  notes appear here.
- **Hypothesis**: our reading, open to change.

Units are KH2 world units (about 1 cm) and 60 Hz game frames.

## Where the data comes from

| Data | File (inside the `kh2_*.pkg` archives) | Notes |
|---|---|---|
| Walk/run speed, jump height, turn rate | `03system.bin` → `pref` → `prty`, pointer 1 | Sora: walk 2, run 8, jump 185 |
| Fall cap, lock-on ranges, lunge factor | `pref` → `sstm` | lunge factor is float 42 ("attack distance to speed") |
| Growth abilities LV1–MAX | `pref` → `fmab` records 0–3 | |
| Attack targeting | `pref` → `plyr`, pointer 1 | auto lock-on range |
| Move length and root motion | `obj/P_EX100.mset` (Sora) | ANB header + root F-curves |
| Move windows | `obj/P_EX100.mset` motion triggers | combo input, finisher chain, turn, invulnerable, jump frame |
| Keyblade hitbox windows | `obj/W_EX010.mset` triggers, same slot | |
| Keyblade hitbox shapes | `obj/W_EX010.mdlx` collision (type 6) | |
| Combo table | `00battle.bin` → `ptya`, pointer 1 | 37 rows for Sora |

## Root motion

Root-motion curves are sampled on the motion's 60 Hz key timeline (see `animation.md`).
Sora's first ground swing (A300) moves him 150 units. **Checked (exe).**

## Physics tick

**Checked (exe).** Action logic and input run every frame. Velocities are recomputed on a
**30 Hz tick** (every second frame, `dt = 2`) and held for both frames. The exception is very
fast objects (over 100 units/frame), which tick every frame. As a result, a change of stick
direction takes effect on the next even frame.

## Ground

- Velocity is the stick direction × speed with no acceleration: instant start, instant stop.
  The body turns toward the stick at `turn_speed` per frame separately, so Sora can move
  slightly sideways while turning. **Checked (exe).**
- The run animation is used when speed ≥ (walk + run) / 2. **Checked (exe).**
- Which stick tilt selects walk or run (we use 50%) is **Hypothesis**.

## Jump

**Checked (exe):**
- The rising half is a double sine: `y = h · sin(π/2 · sin(π/2 · t/T))` with
  `T = sqrt(2h/g)` and `g = 20/49` units/frame². This means a fast launch and a long hang near
  the apex.
- After the apex it is an ordinary parabola, `y = h − g/2 · (t − T)²`. Each tick's drop is
  capped at `fall_max` per frame (16).
- Height is `prty` jump height (185), or the `fmab` High Jump height if Sora has High Jump.
- On your install: takeoff frame 4, apex exactly 185 at frame 33, landing at frame 65.
  **Checked.**
- Walking off a ledge starts the same fall from rest.

Whether holding jump changes the height is **open**. The jump action has a "can hold" check
that hasn't been traced yet; the height is fixed for now.

## Air control

**Checked (exe):** each tick, with stick input, `v = 0.9·v + 0.1·target`. With no input,
`v = 0.95·v`. Air attacks get the no-input drag. The target speed (we use the walk/run speed
for the tilt) is **Hypothesis**.

## Growth abilities

All **Checked (exe)** unless noted:
- **Aerial Dodge.** Pressing **jump** again in mid-air. Only possible if the jump *started*
  with Aerial Dodge owned. Walking off
  a ledge doesn't allow it. Once per airtime. It starts a fresh jump arc of
  `air_dodge_height` with horizontal speed `air_dodge_speed` along the stick.
- **Quick Run.** Starts at `air_slide_speed` along the stick, with no gravity, and is
  invulnerable for `air_slide_star_time`. Each tick the speed is multiplied by `brake`. After
  `air_slide_time` frames the brake switches to `stop_brake` and the end motion (204) plays.
- **Dodge Roll.** Ground only. It faces the stick and moves by the animation's root motion
  (438 units). It is invulnerable for `dodge_roll_star_time` (10/16/24/32 frames by level).
- **Glide:**
  - **Start:** airborne, jump pressed, and height above ground strictly greater than
    `glide_start_height`.
  - **Turning:** uses `glide_turn_speed`.
  - **Speed** each tick: `(1 − a)·glide_speed + a·|v|`, along the facing.
  - **Descent per frame:** `min(max(height − glide_fall_height, 0) × glide_fall_ratio,
    glide_fall_max)`. Descent slows as the ground nears, so the glide settles at
    `glide_fall_height` above the ground.
  - **End:** when height drops below `glide_end_height` or jump is released (release is
    **Hypothesis**).
  - On your install, levels 2–MAX hover above their end height and glide indefinitely.
    Level 1 lands after about 300 frames.
  - Motions: 205 straight, 206/207 turning.
- **Ability levels.** A level-check helper returns 5 under a debug flag. The "level > 4"
  branches only matter there.

Unit question still open: the invulnerability timers are treated as 60 Hz frames.

## Landing

The landing motion is 26 frames long. Input is accepted again from its "allow reaction
command" window (frame 8). **Hypothesis**; the landing action itself has not been traced.

## Combo selection

**Checked (exe).** Each press runs these steps in order:

1. **Counter update** (before picking):
   - From a non-attack state: count = 1, finishers = 1.
   - From an attack whose swing **connected**: count + 1.
   - From a finisher that connected, if finishers < max: also finishers + 1.
   - **A whiffed swing does not advance the count.** Combo Master makes every swing count as
     a hit.
2. **Row conditions:**
   - The motion must be nonzero.
   - The required ability, if any, must be equipped.
   - `flag & 1` (air row) must match airborne.
   - A finisher row (`flag & 4`) needs `count + combo_offset ≥ max`.
   - A normal row needs finishers ≤ 1.
3. **Range check** by `kind`:
   - 1: against the primary target.
   - 2: against every target.
   - 3: no range check.
   - 0: never picked directly.
   
   A range check requires all of:
   - `near ≤ distance − target radius < far`
   - a height window in y-down: `high′ < (player.y − target.y) ≤ low′`
     - `flag & 2` lowers `high` by the jump height
     - `flag & 8` / `flag & 16` shrink or grow the window by the target's height
   - `inner_min < dot(facing, dir) ≤ inner_max`
4. **Pick the first valid row in table order.** `score` is not used.
5. **Anti-repeat:** if that row's motion is already playing, use the row whose id equals its
   `sub`. This is why Sora alternates swings.
6. **Fallback:** if nothing is valid, use row 9 in the air or row 5 on the ground.

Combo length:
- Ground and air are 3 each.
- Combo Plus / Air Combo Plus add 1.
- Negative Combo removes 1 from both.
- Finishers: 1, plus Finishing Plus.

**Attack movement** (Checked (exe)):
- **Ground rows** only lunge when their speed window is nonzero, at
  `clamp((gap + distance_adjust) × lunge factor, speed_min, speed_max)`. Sora's ground rows
  are all 0, so his ground swings move by root motion alone.
- **Rows with `jump > 0`** start a jump arc:
  - **Height:** with a target, the height difference clamped to `[jump_min, jump_max]`;
    otherwise `jump`.
  - **Horizontal speed:** `gap / uptime`, clamped to the speed window, or `speed_max` with no
    target.
  - Launching one from the ground resets the count.

What this produces on your install (`kh2-combat sora`):

| Situation | Sequence |
|---|---|
| No target (whiffing) | 151 ↔ 153 alternating forever, count stays 1 |
| Target 120 ahead, connecting | 151 → 153 → 155 (finisher) |
| Target 250 ahead, connecting | 153 → 151 → 155 |
| Air, no target | 181 ↔ 182, sinking slowly |

## Still approximated

- **Hitbox placement.** No skeleton yet (Phase 2). Spheres are swept in front of Sora.
- **Enemy hurtbox and knockback** in the playground.
- **Auto-target selection** (nearest within reach).
- **Dodge Roll / Quick Run cancel points.**
