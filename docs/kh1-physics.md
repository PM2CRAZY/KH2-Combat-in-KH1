# KH1's physics state, and KH2's physics written into it (testing branch)

The user (testing, after s284): "currently, kh2 handles all of kh2s physics unless specified
this is causing a bug where if sora does a kh1 air move from a kh2 air move then he gets no
momentum during his kh1 move. Lets rewrite how kh2 can use physics so that kh2s physics gets
written into kh1s internal physics system instead".

Code: `crates/kh1-hook/src/kh1phys.rs` (the model, as plain numbers, with tests),
`shared::physics` in `crates/kh1-hook/src/win/shared.rs` (the actor's fields, the switch point),
and the calls from `takeover.rs` and `combat.rs`.

Source: static reading of the user's Epic exe (Ghidra and objdump), cross-checked against the
existing notes (`kh1-launch.md` has KH1's arc too, `movement-model.md` KH2's) and `btltbl.bin`.
Every address has its Steam RVA beside it (matched as `addr.rs`'s table is). Confidence: [high]
read from the code; [med] read but not seen in play; [low] a guess.

## 1. Where KH1 keeps an actor's motion

Actor fields (Sora is the party's first actor, `0x142D37C80`):

| Field | What | Notes |
|---|---|---|
| `+0x10` | position (x, y, z), KH1's world is y-down | [high] |
| `+0x20` / `+0x2C` | move direction / its speed, written by KH1's control step before the physics step | the ground step's input [high] |
| `+0x30`, `+0x3C`, `+0x150` | facing vector, yaw, the yaw it turns toward | [high] |
| `+0x70` | movement state (index into the state table) | [high] |
| `+0x120` | this frame's push (added to the move, then cleared) | [high] |
| `+0x160` bit 4, `+0x200` | riding a platform, the platform's motion this frame | [high] |
| `+0x2D0` bit 0 | the collision found no ground under him | with `+0x398` (the object stood on) it decides "airborne" [high] |
| `+0x320`, `+0x3D4` | floor point under him, his height above it (or above the object stood on) | written by the mover [high] |
| `+0x378` | the state before the last change | [high] |
| `+0x3B0` | **air time `t`** (60 Hz frames) | [high] |
| `+0x3B4` | **arc start height `y0`** | the frame's vertical push is added to it each step [high] |
| `+0x3B8` | **time to the apex `T`** | [high] |
| `+0x3BC` | **apex offset `h`** (negative: up) | [high] |
| `+0x3C0` | **sideways velocity** (x, y, z, w = its length), units per 60 Hz frame | [high] |
| `+0x3C4` | the vertical step KH1 made this frame (dt included; its `+0x3C0`'s y slot) | read by flight's start and the ceiling bonk [high] |
| `+0x3C8`, `+0x3CC` | state 3's z velocity and drag | [med] |
| `+0x3D0` | the jump's motion base (0 jump, 5 High Jump) | [high] |
| `+0x490` | position at the start of the frame's physics | [high] |
| flags `+0x0` | `4` an action runs; `0x1000` held in the air; `0x2000`; `0x10000` jumped off something; `0x40000` early-release cut open; `0x400` state changed this frame | [high] for 4 / 0x400 / 0x10000 / 0x40000, [med] others |

Sora's stats block (`+0x6C`): `+4` walk 2.0, `+8` run 8.0, `+0x10` jump height, `+0x14` air
acceleration, `+0x18` air drag (per 30 Hz frame). KH1's takeoff (`0x1402ADC10`) reads only
`+0x10` for every jump (no High Jump branch: its High Jump raises the stat; the s261 log's 290 at
High Jump). With physics KH1 all five are read each frame KH2's controller has him
(`shared::physics::kh2_controller`, logged when they change); `+0x14` / `+0x18` are also logged by
`shared::physics::takeoff`. [high: offsets, the takeoff; values at runtime]

There is **no vertical velocity**. The vertical motion is a curve of height against air time:

- rising (`t < T`): `y = y0 + h · sin(π/2 · sin(π/2 · t / T))`, the same curve KH2's jump rises on;
- after: `y = y0 + h + g/2 · (t − T)²`, `g = 0.48469388` (`0x1403EAB54` / Steam `0x3E9D14`),
  a frame's drop capped at `20 × dt` (`0x1404308B8` / `0x42F968`).

A launch of height `H` (`0x1402AD7D0` / Steam `0x2AF960`): `t 0, y0 = y, T = sqrt(2H / g), h =
−H`, state 2, flag `0x10000` off. A launch at speed (`0x1402AD760` / `0x2AF8F0`, called with 0
for a fall from rest): `T` the speed, `h = −g/2 · T²`.

## 2. The frame scale

`0x1423405DC` (Steam `0x233FBDC`): KH1's 60 Hz frames per game frame (1.0 at 60 fps, 2.0 at 30;
the logs show 1, 2 and a 4 at a hitch). `0x142867D78` (`0x2867374`) holds its inverse (set by
`0x140282520` / `0x284690`). Velocities (`+0x3C0`, `+0x2C`) are per 60 Hz frame and multiplied
by it; the air time advances by it. The air drag is per 30 Hz frame: below a scale of 2.0
(`0x140430808`) KH1 uses its square root. [high]

## 3. The state table and the physics step

**The table** `0x142DB4240` (Steam `0x2DB3840`), 0x40 bytes a state, filled by
`0x1402AF9F0 (state, flags, enter, exit, pre-step, step, transition, +0x30, +0x38)` (`0x2B1B80`):

| Offset | What |
|---|---|
| `+0x00` | flags (bit `8`: a platform's carry is added, `0x1402AB1C0` / `0x2AD350`) |
| `+0x08` / `+0x10` | enter / exit handlers, run by the state change `0x1402AF7D0` (`0x2B1960`), which also sets flag `0x400` and `+0x378` |
| `+0x18` | pre-step `(actor, input)`, from the control step |
| `+0x20` | step `(actor, &displacement)` |
| `+0x28` | transition `(actor, airborne) -> state` |
| `+0x38` | state 0's jump press (`0x1402CA3F0`), state 2's `0x1402ADBD0` |

`0x1402AD150` (`0x2AF2E0`) registers states 1, 2, 3 and 5:

| State | Flags | Pre-step | Step | Transition |
|---|---|---|---|---|
| 1 takeoff | 0x34 | `0x1402ADC10` (`0x2AFDA0`) | none (the ground step) | `0x1402ADC00` (`0x2AFD90`): 2 when airborne |
| 2 air | 0x3D | `0x1402ADA90` (`0x2AFC20`) | `0x1402AD510` (`0x2AF6A0`) | `0x1402ADA30` (`0x2AFBC0`) |
| 3 fall (linear) | 5 | `0x1402ADA90` | `0x1402AD950` (`0x2AFAE0`) | `0x1402AD940` (`0x2AFAD0`) |
| 5 | 2 | none | `0x1402AD950` | `0x1402AD910` |

The glide (state 0x15) is registered by `0x1402B0560` (`0x2B26F0`): pre-step `0x1402B06F0`
(`0x2B2880`), step `0x1402B0890` (`0x2B2A20`), started by `0x1402B0BA0` (`0x2B2D30`). States 0
(ground), 0x15 and the jump steps are the ones `shared::physics` wraps (below).

**The physics step** `0x140298230 (actor)` (Steam `0x29A3C0`), once a game frame per actor:

1. `+0x490` = position; `+0x90` (touched objects) cleared.
2. An actor not under physics (`0x14028FE80` / `0x292010`): `+0x3C0` zeroed, moved by `+0x20 ×
   +0x2C × dt` plus carry and push. Done.
3. The displacement by state: 9, 10, 0x16, 0x17: `+0x20 × +0x2C × dt`; 0x12: `+0x20 × dt`;
   0x13: none; 0x14: `+0x20`; any other: the table's step, or the ground step
   `0x1402AF6C0` (`0x2B1850`: `+0x20` laid on the floor's plane, `× +0x2C × dt`) when it has none.
4. Added: the platform carry (table flag 8), `0x1402C4EC0` when `+0x88` bit 0x40 [low: not
   read], the platform's `+0x200` (`+0x160` bit 4), the push `+0x120`. One case (a world flag
   set, Sora) replaces it with `+0x20 × 8.6` [low: not looked into].
5. The mover `0x140297710 (actor, displacement, 0)` (`0x2998A0`): sub-steps through the wall
   solver (`0x140299350` / `0x29B4E0`, `0x140298A70` / `0x29AC00`), collision by state
   (`0x140297F20` / `0x29A0B0`: the air states collide as air, no snap to the floor), writes the
   floor (`+0x320`, `+0x3D4`).
6. A depth check (`+0x14` against `+0x2D4`, flag `0x400000`, `0x1402C0020` / `0x2C21D0`) [low].
7. Airborne = `+0x2D0` bit 0 and no object stood on. The transition picks the state (none: 2 when
   airborne, else 0, or 2 with flag `0x1000`). **Into state 2:** a fall from rest (`0x1402AD760
   (actor, 0)`), the action flag 4 off, and `+0x3C0` = this frame's displacement / dt, its length
   capped at 8 (`0x14043089C` / `0x42F94C`): KH1's walk-off momentum.
8. `+0x120` cleared.

**State 1's pre-step** (the takeoff): 4 frames (`0x140430880`) counted on `+0x3B0`; then, if the
motion is the jump's (4 / 9), a launch from the stats' jump height (`t 0, y0 = y, T, h`), state
2, `+0x3D0` kept, flag `0x10000` off and `0x60000` on; else back to state 0. [high]

**State 2's step** `0x1402AD510` (the jump's rise and every fall):
- Flag `0x1000` (held in the air): the arc starts over at his height, no move.
- Sideways, stick let go (`+0x2C` ≤ 0): `+0x3C0 ×= drag` (`stats[+0x18]`, its square root at 60
  fps). Stick held: `+0x3C0 += +0x20 × stats[+0x14] × +0x2C` (once a game frame, no frame scale
  on the add), its length kept to the larger of its old length and the stats' run `+8` (so
  speed above the run is kept while steering, never gained). Moved by `+0x3C0 × dt`. No action
  test: it runs in KH1's air attacks too.
- Vertical: `t += dt`, the curve above (§1), `+0x3C4` = the frame's drop; rising, the apex
  motion (`0x1402AD4A0` / `0x2AF630`: 6 frames before the apex, `0x14043088C`).
- `y0 += +0x124` (the push's y).

**State 2's pre-step** `0x1402ADA90`: the grabs in order (vine `0x1402C5710` / `0x2C78C0`, ledge
`0x1402C68A0` / `0x2C8A50`, bar `0x1402C7E00` / `0x2C9FB0`, hang `0x1402C17B0` / `0x2C3960`,
water `0x1402BF410` / `0x2C15C0`), skipped with flag `0x2000`, when he can't act, in the 15 frames
(`0x1404308B0` / `0x42F960`) after jumping off something (flag `0x10000`), from the start of a
rise until 6 frames past its apex (`T ≠ 0` and `t ≤ T + 6`), or during an action (flag 4). Else
it faces the stick.

**State 2's transition** `0x1402ADA30`: grounded, not in water (`0x1402BF530` / `0x2C16E0`) and no
flag `0x1000`: state 0 (6 with flag `0x2000`); flight (world flag 0x20, `0x1402AFC10` /
`0x2B1DA0`): 8.

**State 3's step** `0x1402AD950`: linear: moves by `+0x3C0 × dt`, x and z times `+0x3CC` (its
root at 60 fps), `+0x3C4 += +0x3B4 × dt` capped at 20. Used by `takeover` only for its collision
(the air kind) around KH2's moves. [med]

## 4. KH1's own air moves and momentum

- **Air attack** (`0x1402A5380` / `0x2A7510` picks it in state 2 through `0x1402A4AE0` /
  `0x2A6C70`, rows 0xF..0x12; `0x1402A4760` / `0x2A68F0` runs it): `+0x3C0` is **replaced** by the
  direction (to the target, else the stick's move vector `0x142D5F630` / `0x2D5EC30`, zero with
  the stick let go) × 8 on the combo's first swing (`0x14043089C`), else × 12 (rows 0xF / 0x10,
  `0x1404308AC` / `0x42F95C`) or × 4 (`0x140430880`). Then the row's `+8` hop through
  `0x1402AD7D0`: for Sora (btltbl's row table, `0x142D5D4B0` / `0x2D5CAB0` built from
  `0x142D23740` / `0x2D22D40` by `0x140529880`'s offsets) rows 0xF..0x12 hop 40, 40, 20, 20.
  So a KH1 air attack replaces his momentum in vanilla too, from KH1's own jump as from anything
  else. [high: code; the table read from `btltbl.bin`]
- **Air magic** (the same step, state 2): `+0x3C0 ×= G[+0xF0]` (btltbl `+0x58 + 0xF0`: 0.5), the
  arc left alone. [high]
- **Glide** (state 0x15): its speed starts from `+0x3C0`'s length. [high]

## 5. Before this change (s284)

- While KH2's controller had him (a borrow, a Drive Form), KH1's physics step didn't run
  (`takeover` moves him through KH1's mover itself), KH1's state was kept at 0, and its fields
  were left as they were.
- KH1 taking him in the air (`combat::command`'s air path, `takeover::shared_give_back`, KH1's
  glide out of KH2's air) set state 2 with the arc zeroed (`t`, `T`, `h` 0, `y0` his height):
  **a fall from rest**, KH2's rise gone, plus KH2's sideways speed in `+0x3C0`. Only the
  give-back to a fall then put KH1's fall at KH2's down speed (`fall_from`).
- An air attack then replaced the sideways speed and hopped (§4), so the user saw "no momentum".
- With Jump at KH2 on KH1's states, KH1's arc fields were kept roughly in step (`T`, `h`, past the
  apex `T = t`); its fall was KH1's gravity against KH2's, so KH1's own step (an air attack) jumped
  his height to KH1's curve. After KH1's own air move KH2's arc started again **from rest**.
- KH2 taking him from KH1's states in the air used his travel over the game frame as a 60 Hz
  frame's: twice his speed at 30 fps.

## 6. The new flow

KH1's fields are where momentum lives, whoever moves him (`kh1phys`, `shared::physics`):

1. **KH2's controller has him** (`takeover`, every 60 Hz step, 30 fps's substeps too): after the
   step's move and floor, `shared::physics::mirror` writes KH2's motion into KH1's fields:
   - sideways: the step's travel (a 60 Hz step's, so its velocity, an attack's root motion
     included) into `+0x3C0`, kept to 30;
   - in the air: KH2's arc as KH1's (`kh1phys::from_kh2_step`): a rise exactly (the same curve,
     `t` and `T` KH2's, `y0` from where he is; mid-tick the arc's time a frame back,
     `Character::air_arc`), a fall at KH2's fall speed now (`Arc::falling`: KH1's apex put `down
     / g` frames back). When the arc isn't what moved him (root motion up or down, a blow), his
     speed goes on (`Arc::moving`);
   - on the ground: a fall from rest at his height.
   KH1's physics step still doesn't run on KH2's frames: nothing is applied twice, and at 30 fps
   the last substep's velocity is what KH1 goes on from (per 60 Hz frame; KH1 scales it).
2. **KH1 takes him in the air** (`kh1_takes_air`: an air command of KH1's out of a KH2 move, a
   give-back to the fall, KH1's glide out of KH2's air): state 2 and the last mirror, moved to
   where he is. KH1's own fall or KH2's numbers on it go on at KH2's speed, rising or falling.
3. **KH1's air attack out of KH2's motion** (`kh1_air_attack_started`, from `combat::command`:
   out of a KH2 move, or out of KH2's jump arc on KH1's states): KH1's start sets its lunge and hop
   (§4); the faster sideways speed and the higher rise are kept (`kh1phys::keep_momentum`). So a
   KH1 air move never has less momentum than it took over, nor less than KH1 gives it. Spells
   keep KH1's own rule (half the sideways speed) and the carried arc. Only the first KH1 move out
   of KH2's motion: KH1's later swings are KH1's.
4. **KH2's numbers on KH1's states** (physics KH2, `air_step`; before §7's setting, Jump at
   KH2): each frame KH1's arc and air speed are KH2's (`Arc::from_kh2`), moved to where he is
   before KH1's own step reads them (a floor, a ceiling, a push), unless KH1 changed them (its own
   move's start): then KH2's arc goes on from KH1's (`kh1phys::kh1_arc_on_kh2`). KH1's own action
   in the air (flag 4) keeps its own lunge and hop on KH2's gravity, fall and drift (§7; before,
   KH1's numbers moved it and KH2's arc picked up from KH1's fields after it).
5. **KH2 takes him from KH1's air state** (`takeover::kh1_fall_speed`): KH1's air speed and its
   arc's slope (`kh1_velocity`); outside state 2, his travel over the frame divided by the frame
   scale.

Floors, walls, ledges, platforms and pushes: unchanged. KH2's moves still go through KH1's mover
with `takeover`'s floor rules (step-down, object tops, ceilings, slopes, platform carry once a
game frame, the push). KH1's own states still move him with KH1's physics step.

**Why not KH1's physics step for KH2's moves too:** that step also runs the state's transition
(state changes with their motion requests: KH1's fall and landing motions under KH2's), its
mover snaps a grounded actor to the floor (it undid KH2's first small rises, stage 8), its carry
and push would be added once per 60 Hz substep at 30 fps (twice), and `+0x490` would be the last
substep's. `takeover`'s floor rules exist for those. With the fields written each step, KH1 has
the same motion to go on from either way.

## 7. The physics setting (`physics = kh1 | kh2`)

The user (testing): "lets add a setting to have the physics either be kh1 or kh2, if its kh1 then
all the animations (vanilla, and our kh2 animations were now adding as a custom animation with
our rewrite) will run with kh1s physics (with exceptions to drive forms) if its kh2 then all the
animations (including vanilla animations) will run with kh2s physics instead"; then, on s295:
"BUG: In the config there's not a setting to switch between KH1 and KH2 physics".

- **Where:** ini `physics = kh1 | kh2` (KH2's when the key is missing or is anything else, an
  older ini's hair switch `physics = on / off` included); the Config menu's **Physics** row (KH1 /
  KH2, right after Camera Style), switched live and saved to the ini as the other rows are; the
  log says "physics: ..." at start and "Config menu: physics KH1 (was KH2): ..." on a change. The
  hair's switches (`hair_physics`, `kh2_hair`) are gone since t388: KH2's animation of the hair
  always runs (`docs/kh1-secondary-motion.md`).
- **Code:** `kh1phys::Rule` (`parse`, `source(in_form)`), `shared::physics::rule` / `set_rule` /
  `source()`. `source()` is the one switch point: the setting, and KH2's in a Drive Form. The
  per-move styles choose the animations only (and whether KH2 borrows him for a KH2-only move);
  the old per-style rule (`Rule::Style`) is gone.

### Physics KH2: KH2's numbers on everything base Sora does

On KH1's own states (`shared::physics`'s wrappers, whatever the styles):

| Move | Numbers |
|---|---|
| walk, run | KH2's walk / run speed (KH1 still picks walk or run, and so the animation) |
| jump, High Jump | KH2's arc: its height (KH2's jump, or High Jump's growth height at Sora's level), gravity 20/49, early-release hop, fall cap 16 |
| falls (off a ledge, out of a ladder, after a KH1 move) | KH2's arc from KH1's fields (`Arc::to_kh2`) |
| air drift | KH2's `air_keep` toward the stick at KH2's walk / run speed, `air_drag` without it, per KH2 tick (two 60 Hz frames; earlier builds applied them per 60 Hz frame, twice KH2's) |
| ground combo swings (98..101), either Base Combo style, `kh2_slots` too | the root travel of KH2's swing for that slot (our `kh2slots` slot playing for it, KH2's row selection's pick, `kh1-anim-slots.md` §7; else `pose::swing_motion`: 98 / 100 → 151, 99 → 152, 101 → 154), stretched over KH1's swing |
| glide, Superglide | KH2's glide speed, fall and turn at Sora's Glide level (level 1 at least: KH1 glides him only with its Glide) |
| KH2's borrowed moves | KH2's controller as it is; its air casts KH2's own (the cast motions' no-gravity frames), not KH1's fall (before: only with Aerial Magic at KH2) |

**KH1-only moves with no KH2 numbers** (the choices):

- **KH1's air attacks** (rows 0xF..0x12): their own lunge (8 / 12 / 4 along the target or the
  stick, `+0x3C0`) and hop height (40 / 40 / 20 / 20) are the move's own, like a KH2 attack row's
  hop and speed; they move on KH2's numbers: the hop as **KH2's hop of that height** (KH2's rise
  time for it under KH2's gravity, as KH2's own attack hops: `kh1phys::kh1_arc_on_kh2`), KH2's
  fall after, capped at 16, and **KH2's drift without the stick** (`air_drag` a tick, as KH2's own
  air attacks outside an allow-movement range) in place of KH1's steering during the move. Out of
  KH2's motion, `keep_momentum` still keeps the faster speed and the higher rise.
- **KH1's air casts**: KH1's own start (half the sideways speed) is kept; the arc it leaves alone
  goes on at KH2's gravity and fall; KH2's drift as above.
- **A blow's launch in KH1's air state** (flag 4, its own hop): as an air attack's hop.
- **KH1's hold in the air** (flag `0x1000`): KH1's (no move to put numbers on).
- **KH1's other ground actions** (its roll, an item's use, a ground cast, its specials and
  summons): their own travel, as KH1 has it. KH2 has no numbers for them (its own roll is root
  motion; its items aren't moves).
- **KH1's other states** (ladders, poles, swimming, flight, ledges, state 3's linear fall):
  KH1's. Not "moves" with physics numbers of KH2's (KH2's ledge hang and climb are root motion).
- **`shared = off`** (the old model): the wrappers run only while KH1's states have him in the
  shared model (`shared::active`), so KH1's own jump and glide handed over by `stylejump` (Jump
  / Glide at KH1) stay on KH1's numbers there. Not covered.

### Physics KH1: KH1's numbers on everything base Sora does

KH1's own states are KH1's (none of the wrappers run). KH2's controller, while it has base Sora
out of a form, gets KH1's numbers each frame before its step (`shared::physics::kh2_controller`;
`kh1phys::kh1_params`, `kh1_growth`; back to the moveset's own with physics KH2 or in a form):

| KH2's number | KH1's | Source |
|---|---|---|
| `gravity` | 0.48469388 | `0x1403EAB54` [high] |
| `fall_max` | 20 a 60 Hz frame | `0x1404308B8` [high] |
| `jump_height`, every growth level's `high_jump_height` | stats `+0x10` (KH1 has one jump height; its High Jump raises it) | the takeoff `0x1402ADC10` [high] |
| `walk_speed`, `run_speed` | stats `+4`, `+8` | [high] |
| `run_threshold` | 0.8 | KH1's ground step's walk / run pick [med] |
| `air_accel` (new in `character-core`), `air_drag` | stats `+0x14`, `+0x18` | `0x1402AD510` [high: the rule; values at runtime] |

KH2's arcs are KH1's with these exactly: the same rise curve with `T = sqrt(2H / g)`, the same
fall `g/2 (t - T)²` (tests below). `air_accel` is KH1's steering in `character-core`'s air control
(0 = KH2's `air_keep`): the stick adds `direction × walk or run speed × air_accel` a 60 Hz frame
(two a tick), the length kept to the larger of the old length and the run speed; with the stick let
go, `air_drag` a tick (KH1's drag is per 30 Hz frame, a KH2 tick). A stat that isn't a sane number
keeps KH2's (logged); the steering takes both `+0x14` and `+0x18` or neither.

So **KH2's borrowed moves and the `kh2_slots` moves** run on KH1's physics: Aerial Dodge, KH2's
air combo, its roll and guard, Quick Run, its spells and Reflect, its jump out of them, a fall
handed to KH1 (`mirror` writes KH1's fields from KH1's numbers now). With `shared = off` (the old
model) KH2's controller has him for idle / walk / run / jump too: those get KH1's numbers the same
way.

**KH2-only numbers kept** (the choices; each under KH1's gravity, fall cap and air steering):

- **Aerial Dodge**: its height and speed (KH2's growth table; KH1 has no double jump). Its rise
  is `Arc::new(height, KH1's g)`: the same height, a shorter rise.
- **KH2's attack rows** (the air combo's hops and lunges, `jump`, `speed_min / max`, the target
  height clamp): theirs; their arcs under KH1's gravity (their rise time to the target from it).
- **Root motion** (the roll, KH2's swings' travel, Quick Run's start, the finishers): the
  animation's own travel, unchanged.
- **Quick Run**: its speed, time and brakes (KH2's growth table; no gravity in it).
- **The glide** (only with `shared = off`; with it on the glide is KH1's glide state): KH2's glide
  numbers (KH1's glide numbers are in its glide step's code, not read).
- **The early-release hop** of KH2's jump (`release_hop`: `g × 12.5`, so KH1's g's): KH2's rule.
  KH1's own early cut (flag `0x40000`) isn't read.
- **Air casts**: KH1's fall (`air_magic_falls`, `air_cast_fall` of KH1's gravity, cap 20), as
  KH1's spells sink.
- **The turn rate, ledge grabs, blows, wall hops, ceilings, hit-back**: KH2's (KH1's not read).

### Drive Forms

Always KH2's: `source()` is KH2's in a form, so none of KH1's numbers go into a form's controller
(its moveset's own `MoveParams`, swapped in by `form::swap`; `kh2_controller` puts the form's back
if KH1's were in). Their air casts are as before (Final Form KH2's own; Valor / Wisdom KH1's fall,
`air_cast_fall`: unchanged by this setting).

### Tests (`kh1phys`)

- `the_physics_setting_reads_kh1_or_else_kh2_and_forms_are_kh2s`
- `kh1_params_are_kh1s_numbers_and_keep_kh2s_where_kh1_has_none`
- `kh2s_controller_on_kh1s_numbers_jumps_kh1s_arc` (KH2's controller's jump with KH1's numbers:
  KH1's height, rise time and curve)
- `kh2s_controller_on_kh1s_numbers_falls_at_kh1s_cap` (20, and 16 with KH2's)
- `kh2s_controller_on_kh1s_numbers_steers_as_kh1s_air_state` (added up to the run, faster kept,
  KH1's drag)
- `a_kh1_hop_on_kh2s_numbers_is_kh2s_hop_of_that_height`

## 8. Unverified

- In game: all of it. The sim tests (`kh1phys` tests, `character-core`'s
  `air_arc_matches_where_he_is`) check the arithmetic: KH2's rise goes on exactly in KH1's fields
  at 60 and 30 fps, a fall at its speed, the hand-over of an Aerial Dodge into KH1's air attack,
  KH2's controller on KH1's numbers (§7).
- That nothing of KH1's reads `+0x3C0` or the arc fields in state 0 while KH2 has him (the mirror
  writes them every step). Searched: the takeover, flight and ceiling code; KH1's ground step
  doesn't.
- KH1's air attack keeping KH2's speed: whether the user wants KH1's lunge to win when it is the
  slower (a lock-on target in another direction). `keep_momentum` takes the faster vector.
- `stats[+0x14]` / `[+0x18]` values (logged by `kh2_controller` with physics KH1, and per jump with
  physics KH2). If they read out of range, KH2's air steering stays (logged).
- Physics KH2: that KH1's own air attacks look right on KH2's slower hop and drift; that KH1's
  fields changed between our writes only by KH1's own moves (the "KH1's own air move set its arc"
  log line says when).
- Physics KH1: KH1's run tilt (0.8) [med]; KH2's walk / run animation pick in its controller
  (half way between the two speeds) with KH1's speeds.
- §3's [low] items.
