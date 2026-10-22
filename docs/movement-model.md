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
| Map-collision body | `obj/P_EX100.mdlx` collision, first entry of type 0 or 4 | Sora: sphere radius 90, "height" 90, centre 115 above the feet |
| Ceiling rule | `pref` → `sstm` `ceiling_stop`, `ceiling_disable_command_time` | 0.49, 16 frames |
| Drive Forms | `prty`/`plyr` record and PTYA set by moveset id; each form's own `obj/P_EX100_*.mset/.mdlx` | see `drive-model.md` |

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

**Variable height. Checked (exe).**
- **Which jumps:** button jumps and Aerial Dodge are "holdable".
- **Early release:** if the jump button is released after more than 6 frames of rise and more
  than 5 frames before the apex, the rest of the rise is replaced by a small hop from the
  current height. The hop is `g × 0.5 × 5 × 5` ≈ 5.1 units high.
- **Hold:** keeping the button down to within 5 frames of the apex gives the full height.
- **On your install:**
  - normal jump: about 115 tapped, 185 held
  - High Jump MAX: about 214 tapped, 585 held

Our one deviation: a release *during the 4 takeoff frames* also counts. In the exe, the hold
flag only starts with the airborne jump.

## Air control

**Checked (exe):** each tick, with stick input, `v = 0.9·v + 0.1·target`. With no input,
`v = 0.95·v`. Air attacks get the no-input drag. The target speed (we use the walk/run speed
for the tilt) is **Hypothesis**.

## Growth abilities

All **Checked (exe)** unless noted:
- **Aerial Dodge.** Pressing **jump** again in mid-air. Only possible if the jump *started*
  with Aerial Dodge owned. Walking off
  a ledge doesn't allow it. Once per airtime. It starts a fresh jump arc of
  `air_dodge_height` with horizontal speed `air_dodge_speed` along the stick; with the stick
  neutral it goes straight up (s178, the user: KH2's doesn't push forward unless tilted; before,
  it went along the facing). A dodge roll or Quick Run that goes off a ledge gives the fall its
  Aerial Dodge too (s183, the user's spec; walking off still doesn't).
- **Air control (ours, s183):** with the stick held, the air speed steers toward the stick's
  run speed (`air_keep`) but fades no faster than with the stick let go (`air_drag`), unless
  pushed against (the user: an Aerial Dodge's speed, up to 36 a frame, was gone in a few frames
  as soon as a direction was pressed).
- **Square on the ground** (the action button handler `0x1403A75A0`, after Glide and the
  Square combo rows). "Standing" is the stick at rest (its tilt exactly 0 after KH2's dead
  zone; ours: under `DEAD_ZONE`), "moving" anything else:
  1. Standing: **Guard** (combo row 11), if equipped.
  2. **Dodge Roll**, if equipped: moving (along the stick), or standing without Guard (along
     his facing).
  3. **Quick Run**, if equipped: moving, or standing with neither Guard nor the roll (along his
     facing; Wisdom Form, which has no guard, dashes from a standstill).
  4. **Guard** again, if equipped, moving too: with neither growth ability Sora guards on
     the run.
  - So with Dodge Roll and Quick Run both equipped, Quick Run comes out of the roll: Square
    while moving rolls, and the button still held at the roll's frame-10 check turns it into
    Quick Run (below). Without the roll, Square while moving is Quick Run, Guard equipped or not.
  - The roll and Quick Run also need a free state (the motion's range 0/1/2) and the ground.
  - Tests: `square_priority_for_every_equipped_combination`,
    `quick_run_out_of_the_roll_with_guard_equipped`,
    `quick_run_from_a_standstill_goes_along_the_facing`.
  - **Keyboard (s300 fix, the KH1 port).** KH1 reads Square as its logical bit `0x10` too, and
    its menu step (`0x140291660`) turns a press of it into its own Square action (bit
    `0x4000000` of `0x142D54440`), its block or dodge roll by KH1's rule (block unless it has the
    roll and the stick is tilted, `0x1402A59A0`). A pad's Square is taken out of its record
    before KH1 reads it, but the keyboard's keys are merged in after that (`0x1400D7DE0`), so on
    keyboard KH1 blocked or rolled over KH2's Square (the player: Guard or the roll always, no
    Quick Run). The takeover's control step now takes that action and those bits out whenever
    the Square is KH2's. The stick itself reads fine on keyboard: KH1's key bindings set the
    stick bytes by a fixed offset each frame, no ramp.
- **Quick Run.** Ground only.
  - **Start:** `air_slide_speed` along the stick, with no gravity, invulnerable for
    `air_slide_star_time`.
  - **No cancel** (Data + exe): its motions (203 dash, 204 end) carry no free state (ranges
    0/1/2) and no combo window, only the reaction-command range from frame 8, so Attack, magic,
    jump and Square are all refused until it ends; letting go of the button is the way out
    (then the end motion and KH2's landing, which takes a jump at once). Test
    `quick_run_ends_only_on_release`.
  - **Each tick:** the speed is multiplied by `brake`.
  - **Hold:** the dash lasts while the action button is held, up to `air_slide_time`
    frames.
  - **End:** on release or timeout, the brake switches to `stop_brake` and the end motion
    (204) plays.
- **Dodge Roll.** Ground only.
  - **Movement:** it faces the stick and moves by the animation's root motion (435 units).
  - **Invulnerability:** `dodge_roll_star_time` (10/16/24/32 frames by level).
  - **Hold:** the roll's frame-10 "quick run check" event turns it into a Quick Run along
    the roll direction if the action button is still held and Sora has Quick Run.
    Ours: held since the roll's own press. A second press made during the roll is no hold, so
    a quick double tap stays a roll (the player: it went into Quick Run, which KH2 doesn't do);
    a press once the roll lets him go (its fall-check frame) is a new roll. Test
    `a_double_tap_of_the_roll_is_no_quick_run`.
- **Glide:**
  - **Start:** airborne, jump pressed, and height above ground strictly greater than
    `glide_start_height`.
  - **Turning:** uses `glide_turn_speed`.
  - **Speed** each tick: `(1 − a)·glide_speed·tilt + a·|v|`, along the facing; `tilt` is the
    stick's (0 when neutral), so letting go glides in place (the user: KH2 does). The exe's
    glide step (`0x140422C00`) has no tilt: `(1 − a)·glide_speed + a·|v|`.
  - **Gliding in place (Checked (exe), s178):** a glide started with the stick neutral is
    ACTION_GLIDEHOVER (`0x1403A8F44`: action 0x11, else 0x12 the glide), which plays motion
    208 (`0x140422B20`), slows its horizontal speed ×0.9 a tick (`0x140422930`, `0x1405866E0`)
    and falls as the glide does; the stick takes it into the glide. Ours also: a glide let go
    hovers once it has slowed under 2 per frame (the exe's glide has no way back).
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

**Checked (exe).** Touching down switches straight to the idle action. The landing motion
(26 frames) is only an animation:
- **Stick held:** walk/run takes over after a short hold: 14 frames standing in the landing
  (the user's KH2 video at 60 fps: touchdown, about 16 frames of the landing crouch, then the
  run). This replaces "at once", which the exe reading had suggested but play doesn't show.
  A jump ends the hold at any frame (the user).
- **Stick neutral:** the landing motion plays and velocity is the stick's, so Sora stops dead.
  Air momentum isn't carried (this was the "slide on landing").
- Buttons are accepted again from the motion's "allow reaction command" window (frame 8);
  **Hypothesis**.

## Ledge grab

**Checked (exe)** unless marked.
- **When it's checked:** every frame of a jump or fall (the jump action), once the arc is 6
  frames past its apex (15 frames into a plain fall), with the stick held.
- **The search:**
  - centred `hang_height` (150) above the feet
  - horizontally within `hang_range_xz` (20) + the body radius (90, from the model's type-4
    collision shape)
  - vertically from 60 below to 30 above the centre (`hang_range_l` / `hang_range_h`)
  - so the ledge must be between 90 and 180 above the feet
- **Too low:** the ledge must also be at least `hang_margin` (210) above the ground under
  Sora. Lower edges are just jumped onto.
- **Hanging:** Sora is placed `hang_height` below the edge and 90 out from the wall, facing
  it, and plays motion 7 (looping).
- **Climbing:** pushing toward the wall plays motion 8. Its root motion lifts Sora 200 and
  carries him 110 forward, then he falls onto the top. So holding toward the wall grabs and
  climbs in one go.
- **Letting go by pulling away** is ours: the game's release input isn't traced.
- **Ledges** come from the host. Our arena offers its boxes' top edges. KH2 takes them from
  map collision.

## Map collision (slopes, steps, ceilings)

**Checked (exe)** unless marked. Implemented in the `map-collision` crate.

**The body.** Sora moves against the map as **one sphere**: radius 90, centre 115 above his
feet, so it covers 25 to 205 above them. His shape is type 4 ("map"), which adds a **lower
sphere** that only meets wall-type polygons:
- grounded: centre 25 above the feet
- airborne: centre 45 above the feet (faces), 91.8 above (edges)

The Soldier's shape is type 0: the same sphere, no lower sphere.

**Polygon types.** The low nibble of a polygon's attribute is its surface type:
| Type | Behaviour |
|---|---|
| 1 floor | only the main sphere meets it |
| 2 | can't be landed on from the air |
| 4 | ignored |
| anything else | wall (both spheres) |

The player and party pass through attribute bit 0x10. Bit 0x100000 is seen only by the floor
ray.

**One frame's move:**
1. **Grounded moves follow the ground.** The move is laid on the plane of last frame's ground
   with its length kept, so the speed along a slope equals the flat speed.
2. **Sweep.** The sphere moves in `floor(|move| / 90) + 1` sub-steps. After each one it is
   pushed out of polygons:
   - faces first: when the centre is 0..90 in front of the plane and its foot point is inside
     the polygon, it is moved to 90.008 in front
   - then edges and corners: to 90.008 from the nearest point
   - up to 6 passes per sub-step
   - more than 9 pushes in one grounded pass means stuck, and the sub-step is cancelled
3. **No tunnelling.** If the centre's path crosses any polygon from the front, the whole move
   is cancelled.
4. **Floor ray.** A ray goes straight down from `115 + min(|move.y|, 100)` above the new feet
   to 345 below them, and takes the highest upward-facing polygon.
   - **Grounded:** the feet snap onto it when it is at most **24** below the start (plus any
     downward move). Floors above the feet are always taken. Anything lower starts a fall.
   - **Airborne:** Sora lands only when the feet went below the plane and the move isn't going
     up.

**What follows from that (Inferred from the geometry, and tested in `map-collision`):**
- **Step-up height is 25.** A wall-type riser up to 25 high stays under the lower sphere, and
  the floor ray lifts the feet onto it. Higher risers stop the lower sphere like a wall.
- **Stair treads must be deeper than 90**, or the next riser meets the lower sphere before the
  feet reach the step. Our stairs use 20-unit risers and 100-unit treads.
- **Slopes.**
  - A floor-type slope up to **acos(90/115) = 38.5°** is walked like flat ground.
  - On a steeper one, the main sphere touches the slope and is pushed out each frame. Standing
    still, Sora slides down; at 45° even running can't beat the slide.
  - Sora can't walk onto a steep slope from flat ground: the sphere meets it first, like a
    wall.
  - There is no explicit slope limit in the code. The sphere's size sets it.
- **Ceilings.** When a rise makes less than **49%** of its intended height (sstm
  `ceiling_stop`), the head hit a ceiling. A jump or launch that is more than 6 frames into
  its rise then falls from rest at the current height. Air commands are refused for **16**
  frames (sstm `ceiling_disable_command_time`) or until he lands. Earlier in the rise the
  ceiling only blocks.

**Ours / not modelled:**
- moving platforms and the check-ahead probe used on them
- the per-room camera and limiter objects
- the collision tree: our arena is small enough to test every polygon

## Hitting walls

**Checked (exe).** Attacks whose ATKP flag 0x01 is set ("hits background"; Sora's 126, 128
and 131) cast a ray each frame from the first attack shape's previous centre to its new one.
This starts on the window's second frame and only runs until the swing has hit something.
If the ray misses, a ray straight down by ATKP `floor_check` is tried.
- **On contact** the swing stops, the combo counts it as connected, and Sora is repelled:
  - **Ground:** motion 41 (its combo window opens at frame 16) and a hit-back push of 50.
    That is 5 units/frame decaying ×0.9 per frame, stopping below 1, about 40 units in all.
  - **Air:** motion 43, a 50-unit hop, drifting back at 50 × `hit_back_to_jump` (0.048).
- **Background materials:** one surface class (attribute 1) doesn't repel. The playground's
  boxes all repel.

## Combo selection

**Checked (exe).** Each press runs these steps in order:

1. **Counter update** (before picking):
   - From a non-attack state: count = 1, finishers = 1.
   - From an attack whose swing **connected**: count + 1.
   - From a finisher that connected, if finishers < max: also finishers + 1.
   - **On the ground, a whiffed swing does not advance the count** (read from the exe).
     Combo Master makes every swing count as a hit.
   - **In the air, whiffed swings advance it too**, so a whiffed air combo still ends in its
     finisher (181 → 182 → 183). This matches the real game as played; the exe path for it
     hasn't been found yet.
   - **Ours: an air swing that ends in the air keeps its combo through the fall.** The
     air rows queue the fall (`next_motion` 4) rather than idle, so the attack is read as going
     on until the landing: a press in the fall counts on from it, and after the air combo's
     last finisher nothing new starts before the landing (`Character::air_combo`). Before, a
     press after the swing had ended was "from a non-attack state" and restarted the count, and
     a jump row (Aerial Spiral, 14..114 toward the target) lifted Sora again on every press: an
     endless air combo while a foe flew near. Inferred from the table, not traced in the exe.
2. **Row conditions:**
   - The motion must be nonzero.
   - The required ability, if any, must be equipped.
   - `flag & 1` (air row) must match airborne.
   - A finisher row (`flag & 4`) needs `count + combo_offset ≥ max`.
   - A normal row needs finishers ≤ 1.
3. **Range check** by `kind`:
   - 1: against the primary target.
   - 2: against every target. Ours: locked on, a kind-2 row with no ability sees only the
     locked target (`combat-model.md`, "Combo rows for the foes around him").
   - 3: no range check.
   - 0: never picked directly.
   
   A range check requires all of:
   - `near ≤ distance − target radius < far`
   - a height window in y-down: `high′ < (target.y − player.y) ≤ low′` (`0x140401AD0`; the
     target point is its target shape's, `0x1403C8EE0`)
     - `flag & 2` lowers `high` by the jump height
     - `flag & 8` / `flag & 16` shrink or grow the window by the target's height
   - `inner_min < dot(facing, dir) ≤ inner_max`, where the facing is the direction to the
     primary target when there is one, else the player's facing (`0x140401BB0`); the primary's
     own rows (kind 1, window 0.99996..1) therefore always face it
4. **Walk the valid rows in table order** (`0x140401F30`) and take the first whose motion
   is not the one playing (or whose `sub` is its own id). `score` is not used.
5. **Anti-repeat:** a valid row whose motion is already playing is passed over; the first
   such row's `sub` row is kept, and used only if no other valid row is found. This is why
   Sora alternates swings, and why Finishing Plus and the modifiers mix (Explosion, then the
   next valid finisher, then Explosion again). Corrected in b48: before, the first valid row
   was swapped for its `sub` and a row without one repeated.
6. **Fallback** (`0x140402000`): if the walk finds nothing, row 9 (air) or row 5 (ground) is
   marked valid too and the walk runs again.
7. **Rows reached from a state** (kind 0), in the attack-press handler (`0x1403A6A60`):
   attack while the guard's repel (motion 42) plays takes row 39, Counterguard, if it passes
   its conditions. Retaliating Slash (row 40) is checked in a separate routine
   (`0x1403A8D00`, row id in a register); ours: attack while launched.
   The Square moves (Finishing Leap 37, Upper Slash 12, Horizontal Slash 38) are taken by
   Square in a combo window. **Ours (s177):** a Square move that isn't a finisher stands for
   one swing of the combo, so it is only there while `count < max` after the count update;
   at the combo's end Square does nothing and Attack's finisher is next. Without this,
   Horizontal Slash (a jump row that hops 14..114) could be pressed forever in the air.

**Magic in the combo string (ours, s183, the user's spec):** a spell takes one slot as a
swing does (`Character::spell_slot`), KH2's casts (`cast`) and KH1's spells (`magic_step` at the
hand-off) alike:
- with no combo going it is a new combo's first step (count 1);
- in a going combo (a swing or spell playing, or one ended within 60 frames on the ground /
  before landing) it is the next step, counted as a swing is; on the combo's last step it is
  its finisher (KH2's finisher cast motion, 72 for Reflect);
- during a finisher (a swing's finisher window, or a finisher cast's) only as the next finisher
  while Finishing Plus leaves one; after the combo's last finisher nothing new, swing or spell,
  starts until that animation has ended (`can_cast`, `try_attack`, `cancel_allowed`);
- a swing after a spell counts on from it (`hit` set), and after a spell that took a finisher
  slot with another to come it is that finisher (`carry_finisher`).
A finisher cast (`cast_finisher`) can't be cancelled (no jump, roll, swing or spell) except into
the next finisher in its finisher window; when it ends the combo is over. The aerial Reflect's
fall and landing (s174 / s177) are kept: landing ends it.

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

- **Reaction-command acceptance** (`Character::can_take_command`): free movement, landing, and
  combo or "allow RC" windows. The game's full rule also checks the command's flags (Land,
  Force, No-combo).

- **Auto-target selection** (nearest within reach).
- **Dodge Roll cancel point** (ours: from its fall-check event, frame 34; its free state, range
  2, starts at 36). Quick Run's is settled: none but the release (above).


## Ground inertia and moving attacks (stage 192)

- `prty` acceleration / brake (Sora 0 / 0; Wisdom Form 0.85 / 0.97): KH2's idle-action move
  (exe, `0x140421630` → `0x1403B6DB0`) eases the ground velocity toward the stick
  (`target × (1 − a) + v × a`) and, with the stick let go, multiplies it by the brake. With
  inertia, walk / run / idle go by what the stick asks, so he slides to a stop in his idle.
- Motion range 41 (allow movement) lets an attack or a cast move with the stick (same inertia),
  turning with it when nothing is targeted; past the range, with inertia, the speed brakes off.
  Wisdom's shots (0..46 of 151) and casts carry it; Sora's swings don't.

(Stage 193: the stick-held momentum keeping of stage 183 is reverted, the user; air control is
KH2's plain steer, `air_keep` 0.9 toward the stick's speed.)

## The combo keeps one direction (stage 197, the KH1 port)

`Character::set_combo_direction_lock` (the user: "in kh2 sora will actually stay locked in one
direction during a combo until either ... he ends his combo and returns to his idle or freefall
animation [or] if the orange or blue reticle is on, his next attack will snap him towards the
direction of the enemy (he can not snap to an enemy mid attack ...)"): the stick aims only a
combo's first swing (one started from idle or a fall); a swing that follows another in the
combo keeps its facing unless a target is there, which it faces at its start; no turning during
a swing (the motions' turn windows and the moving attacks' stick turn are skipped). Test
`combo_keeps_its_direction`.

## Back in idle the combo is over; no spell through a finisher (stage 198)

- `COMBO_GRACE` is 2 frames (the hand-over), not 60: a swing or a spell that ends into idle ends
  the combo; a spell played to its end on the ground resets it (`reset_combo`). Only an input
  during the move (a buffered Attack, a spell in the swing's combo window) carries it on.
- `Character::magic_later`: whether a spell chosen now can come in the move playing, now or at a
  later window of it. Hosts drop a spell chosen when it can't (a finisher with no next finisher,
  the combo's last step) instead of holding it for after the move.
- Tests `spell_mid_combo_is_a_step_of_it` (rewritten), `no_magic_buffered_through_a_finisher`.
- Stage 199: mid-swing turning comes back with a target only (the user: "change the direction
  he's facing mid move only if ... an entity having a blue or orange reticle on it"): the
  motions' turn windows (TurnToLock) run while a target is locked (the hook locks KH2 on KH1's
  reticle, small blue or big orange); with none they don't, and the stick never turns a swing.
