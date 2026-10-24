# KH2 effects: PAX, DPX and DPD

How Kingdom Hearts II Final Mix (PC, 1.5+2.5 Steam/Epic build) plays the effects in PAX
packages: the element layer that binds an effect to a bone, the particle program machine
inside DPD particle data (the "ppp" system), and how sprites and models are drawn. The goal is
to re-create Sora's real Explosion and Magnet Burst effects in another engine.

**Sources.** Container layouts and the 242 program type names are from OpenKH
(`OpenKh.Kh2/Pax.cs`, `Dpx.cs`, `Dpd*.cs`, `DpdPData.cs`, `DpdModel.cs`, `DpdVsf.cs`,
Apache-2.0, OpenKH Team and Contributors). Everything about behaviour is our own reading of
the user's executable in Ghidra. Function addresses are for that build and are given only as
references. No decompiled code is reproduced here. The data examples come from Sora's
`obj/P_EX100.a.us` (entry `p_ex`).

**Confidence.** **Checked**: read in the code and consistent with the data. **Inferred**:
follows from the code but a detail was not traced to the end. **Unknown**: listed at the end.

**Code.** `crates/kh2-formats/src/dpd.rs` parses everything described here (particle data,
emitters, programs and keys, textures with palette decoding, shapes, models, vertex sets).
`cargo run -p kh2-formats --example pax_fx -- <file> [element ids]` prints an effect's
emitters and programs. `crates/kh2-fx` plays them (the runtime of §2, §5, §7, §8) and
outputs world-space triangles; §15 lists what was learnt while writing it.

---

## 1. Overview

```
motion trigger 1 (element id) ──> PAX element ──> effect instance ──> particle data (DPD)
                                    bone, offset,    root matrix,         emitters ──> instances
                                    flags, sound     colour/fade,         programs run on each
                                                     start delay          instance every tick
```

- A **PAX element** names a DPX effect and says where to put it (bone, offset, rotation,
  scale) and how it ends.
- A **DPX effect** names one **particle data** inside one **DPD**.
- A particle data is a small program file: a list of **emitters**. An emitter spawns one
  **instance** ("pObj") at its start frame. Each instance runs the emitter's **programs**, in
  list order, every tick. Programs are things like "add this key to the scale", "build the
  matrix", "draw shape 20 with this matrix", "spawn sparks".
- **Timing (checked).** The effect clock runs at **30 Hz**: every displayed frame adds the game
  frame delta (1.0 per 60 Hz frame) to an accumulator, and one effect tick runs per 2.0
  (`0x1402c1d80`, step constant 2.0). Between ticks nothing advances. All frame numbers in
  particle data (start, total, keys) are effect ticks in fixed point, `4096 = 1 tick`.
  So an emitter with `total 20` lives 20 ticks = 40 game frames.
- The element's own timers (start wait, fade-out) count in 60 Hz game frames.

## 2. PAX elements

Layout per OpenKH (80 bytes each, after the 16-byte `PAX_` header). Meanings checked in
`0x1402c5fd0` (start), `0x1402c4d90` / `0x1402c5080` (per frame), `0x1402c3e70` (transform).

| Off | Type | Field | Use |
|---|---|---|---|
| 0x00 | u16 | effect | DPX effect number |
| 0x02 | u16 | id | Motion trigger 1 passes this id; the **first** element with that id starts. |
| 0x04 | u8 | group | |
| 0x05 | u8 | fadeout | Fade-out length in game frames (used by flag 0x40). |
| 0x06 | u16 | bone | Bone index, or an anchor: 0x4000 object root (object rotation and position), 0x4001 body centre, 0x4003+ weapon points (`0x1403b43d0`). |
| 0x08 | u64 | category | |
| 0x10 | u32 | flags | see below |
| 0x14 | f32 | start wait | Game frames before the effect starts ticking. With flag 0x01 the game adds 0.1. |
| 0x18 | i32 | sound | Sound id played when the start wait has elapsed (if > 0). |
| 0x1C | 3×f32 | translation | In bone space, KH2 axes (y down). |
| 0x28 | 3×f32 | rotation | Radians, applied X then Y then Z. |
| 0x34 | 3×f32 | scale | |
| 0x40 | u16, u16 | bone 2, bone 3 | Only with flag 0x10000 (two-point effects). |
| 0x44, 0x48 | ... | | offsets for those bones |
| 0x4C | u8 | | two-point option |

Flags (checked where noted):

| Bit | Meaning |
|---|---|
| 0x0001 | **Bind**: re-read the bone every frame and move the effect with it. |
| 0x0002 | Bind only until the effect has started (then it stays where it is). |
| 0x0004 | Bind position only: rotation is the element rotation, not the bone's. |
| 0x0020 | Registered with the owner so a motion change ends it. |
| 0x0040 | End = **fade out** over `fadeout` game frames. |
| 0x0080 | End = **end the loop** (stop spawning, let live instances finish). Takes priority over 0x40. |
| 0x0100 | Multiply the element scale by the bone's scale. |
| 0x0200 | End when the owner is gone. |
| 0x0800 | Effect alpha follows the weapon's visibility (weapon anchors). |
| 0x10000 | Two-point element (bones at 0x40/0x42). |
| 0x20000 | Variant of the two-point mode. |

**Effect transform (checked, `0x1402c3e70`).** With a bone matrix `B` (world) and the element's
local matrix `L = T(translation) · R(rotation) · S(1)`, the effect root position is the
translation of `B · L`, its rotation is the Euler decomposition of `B · L`, and its scale is
the element scale (times the bone scale with flag 0x100). With flag 0x04 the position is
`B · translation` and the rotation is the element rotation. Non-bound elements compute this
once when they start.

**Ending (checked).** "End" calls `0x1402c67a0`: flag 0x80 → set the particle data's
*ended* flag (no more loops, endless emitters die, see §5.6); else flag 0x40 → fade:
the effect alpha multiplier goes from its current value to 0 linearly over `fadeout` game
frames (`0x1402bdb40`), and the effect is destroyed when it reaches 0. Without either flag the
effect plays to its own end.

## 3. DPX

Per OpenKH: `version | 0 | 0 | count`, then `count × 32` bytes:
`DPD offset (from DPX start) | particle data index | effect number | ...`, then the DPD
offset list. Effect lookup is by **number** (`0x1402bc9b0`).
`dpd::effect_source(pax, number)` returns the DPD bytes and particle data index.

## 4. DPD container

```
0x00  u32  0x96
      5 × (i32 count, count × u32 offset):  particle data, textures, shapes, models, vertex sets
      4 × i32  texture / CLUT VRAM ranges (editor data)
```

Offsets are from the DPD start. At load (`0x1402c1680`) the game relocates each particle
data's program file (`0x1404b20c0`) and creates textures (`0x1404b06c0`).

**Resource numbering (checked, `0x1402c0ab0`).** Programs name shapes, models and vertex sets
by the **number in the resource's own header** (shape: i16 at +0; model: i16 at +0; vertex
set: i32 at +0), not by list position. In the files we looked at the two agree. When a
particle data becomes active, the resources whose bits are set in its `use_*` masks are put
into number-indexed tables.

**Common textures (checked, `0x1402c0ca0`, `0x1402c0ff0`, `0x1402c1810`).** Shapes and models
carry PS2 GS `TEX0` registers. In the file's own address space:

- `TBP < 0x3000` (and `CBP < 0x3E00`) address the shared package **`effect/texcommon.dpd`**,
  which the game keeps resident.
- `TBP ≥ 0x3000` (`CBP ≥ 0x3E00`) address textures in the effect's own DPD.

At run time both sides are moved by the same amounts into per-effect VRAM slots, so the file
values can be matched directly: a frame's `TBP` equals the `tbp` field of the texture that
holds its first page, and `CBP` equals the texture whose palette it uses (§9). Most of
Explosion's and Magnet's sprites and several models use common textures, so
**`texcommon.dpd` is required** to draw them.

## 5. Particle data

### 5.1 Header (0x110 bytes, OpenKH names, checked uses)

| Off | Type | Field | Use |
|---|---|---|---|
| 0x00 | 32×f32 | bounding box | |
| 0x80 | 3×f32 (+pad) | offset | Local transform under the effect root (translation). |
| 0x90 | 3×i32 (+pad) | angle | 65536 = 360°, applied X, Y, Z. |
| 0xA0 | 3×f32 (+pad) | scale | |
| 0xB0 | 16 bytes | use texture | bit `i` (byte `i/8`, bit `i%8`) = needs texture `i` |
| 0xC0 | 16 bytes | use shape | |
| 0xD0 | 16 bytes | use model | |
| 0xE0 | 16 bytes | use vertex set | |
| 0xF0 | i32, i32 | work / packet memory size | |
| 0xF8 | u8 | view clip stop | Skip ticking while off screen. |
| 0xF9 | u8 | character scale | Multiply the root by the owner's character scale. |
| 0xFA, 0xFB | u8 | bind position only, non-ambient | |
| 0xFC.. | | sound, volume, sound wait, bone, wait | |

### 5.2 Program file (from +0x110)

```
+0x00 i32  end frame (×4096)     the effect ends when its clock reaches this (unless looping)
+0x04 i32  flags                 bit 0: looping effect
+0x08 u32  -> type list          count, then the distinct program types used
+0x0C u32  -> emitter list       count, then offsets of the emitters
+0x10      first emitter         (emitters are also chained by their first word)
```

All offsets in the program file are relative to its start (+0x110 of the particle data).
The type list lets each type run optional per-effect hooks (none of the types used by
Explosion or Magnet have them).

### 5.3 Emitter

| Off | Type | Field | Meaning |
|---|---|---|---|
| 0x00 | u32 | next | Offset of the next emitter (0 ends). |
| 0x04 | u32 | cv offsets | Table of per-type work offsets (editor data; programs carry their own). |
| 0x08 | u32 | flag | Editor data, not read at run time. |
| 0x0C | u32 | status | Editor data (4 = child, 8 = endless, ...). |
| 0x10 | i32 | start | Effect frame (×4096) at which **one** instance spawns. `0x7FFFFFFF` = never by the clock; another program spawns it (child emitter). |
| 0x14 | i32 | total | Instance lifetime (×4096). `0x70000000` = endless. |
| 0x18 | i32 | loop start | Both below `0x70000000` → looping instance (§5.6). |
| 0x1C | i32 | loop end | |
| 0x20 | i32 | work size | Bytes of an instance block: a 0xA0-byte header plus the programs' work area. |
| 0x24 | i16 | priority | Instances are kept in a list sorted by this. |
| 0x26 | i16 | program count | |
| 0x28 | 16×n | programs | see below |

Program record (16 bytes):

| Off | Type | Field |
|---|---|---|
| 0x0 | u32 | type (index into the program table, 1..242; names in `dpd::PROGRAM_NAMES`) |
| 0x4 | u16 | key size in bytes |
| 0x6 | u16 | key count |
| 0x8 | u32 | -> keys (key count × key size, then the word `0xFFFFF000`) |
| 0xC | u32 | -> work offsets (i32 list; runs up to the next program's keys) |

**Keys.** Every key starts with its frame (i32, ×4096), sorted ascending. The rest is the
program's parameters. Two kinds of programs exist:

- *Delta keys* (Point, Angle, Scale, Color, the Move and Accele families, MdlTs scroll,
  Shipoly offsets): the key's values are **added** to the instance's state on the exact tick
  the key's frame equals the instance's frame. Data authors bake curves into keys this way
  (value at 0, velocity changes at later keys, tiny correction terms to land on exact values).
- *Parameter keys* (draw programs, MegaBirth, VertexApLc, KeZCrct, Rand*): the program reads
  the **current** key (the latest one whose frame has been reached) every tick.

**Work offsets.** Each program lists byte offsets into the instance's work area, which starts
at +0xA0 of the instance block. Offsets are shared between programs to wire them together,
e.g. in Explosion's first emitter: SclAccele `[32, 0]` (accel at 0, adds into 32), SclMove
`[80, 32]` (velocity 32 adds into scale 80), Scale `[80]`, MatrixScl `[64, 80]` (position 64,
scale 80), Color `[92]`, DrawShapeX `[176, 92, 104, 112]`.

### 5.4 Program table

`0x1407797C0`, 243 entries of 0x50 bytes: name pointer, then nine function slots:

| Slot | Called | When |
|---|---|---|
| +0x08 | calc(instance, key, program) | every tick |
| +0x18 | draw(instance, key, program) | every displayed tick, after calc |
| +0x20 / 0x28 / 0x30 | per-effect hooks | init / before a tick / after a tick |
| +0x38 | construct(instance, program) | instance spawned |
| +0x40 | reset(instance, program) | instance loop restart |
| +0x48 | destruct(instance, program) | instance destroyed |

### 5.5 The tick (checked, `0x1402c1d80`, `0x1404b29c0`, `0x1404b22d0`, `0x1404b23d0`)

Per effect, every displayed frame:

1. If the start wait is > 0, subtract the frame delta and do nothing else (not drawn either).
2. `acc += delta`. At 60 fps this alternates: on one frame `acc < 2` and the calc pass
   computes the current tick (then each instance's frame gets bit 0 set, meaning "already
   computed"); on the next frame `acc` reaches 2, the calc programs see bit 0 and skip,
   MegaBirth updates (it tests the frame flag of the effect rather than bit 0), the frame is
   drawn again and all frames advance. With larger deltas, `floor(acc/2) − 1` extra ticks run
   without drawing. **Net effect: each tick is computed once and shown for two game frames.**
   A re-implementation can simply run one whole tick (steps below) every second game frame
   and draw every frame.
3. A tick:
   1. **Spawn.** For each emitter whose `start ≤ effect frame` (each fires once): allocate an
      instance (frame 0), call every program's construct, link it in priority order.
   2. **Calc pass.** For each emitter, for each program **that has no draw slot**, for each
      instance of that emitter: advance the program's key cursor if the *next* key's frame equals
      the instance frame, then call calc with the current key.
   3. **Draw pass.** Same loops for programs **with** a draw slot (draw programs, MegaBirth,
      Shipoly): advance cursor, calc, then draw (draw only on the drawing tick).
   4. Every instance's frame += 4096 (instances spawned by another program this tick are
      skipped once, so they start next tick at frame 0).
   5. Instance end: if the emitter loops and `frame ≥ loop end`, reset it (§5.6); else if
      `frame ≥ total`, call destructs and free it.
   6. Effect frame += 4096. A non-looping effect is finished when its frame reaches the end
      frame; all remaining instances are destroyed. A looping effect is finished when it has
      been ended and no instance is left.

Because programs run in list order, a program sees the values that earlier programs in the
same emitter wrote this tick.

### 5.6 Loops, children, end

- **Instance loop (checked).** When `frame ≥ loop end` (both loop values valid and the effect
  not ended): frame = 0, every program's reset is called, and the instance is fast-forwarded
  with calc-only steps until `frame ≥ loop start` (MegaBirth and Shipoly are skipped while
  fast-forwarding).
- **Ended effect.** Once ended, no instance loops; endless instances (total `0x70000000`) of a
  looping effect are destroyed when only endless ones remain.
- **Children.** Emitters with start `0x7FFFFFFF` are spawned by programs (VertexApLc, VertexAp,
  the other `*Ap*`/`BornRnd*` types) through `0x1404b1ce0`. The child keeps a link to its parent, used by
  ParMatrix.

### 5.7 Randomness (checked, `0x1404de100`)

`rand01 = ((rand() & 255) << 16 | (rand() & 255) << 8 | (rand() & 255)) / 16777215` using the
C runtime `rand`. Any uniform `[0, 1]` source reproduces it in distribution.

### 5.8 Instance block

| Off | Meaning |
|---|---|
| +0x00 | next instance |
| +0x04 | parent instance (children) |
| +0x08 | emitter record |
| +0x0C | frame (×4096; bit 0 = repeat frame) |
| +0x10 | 4×4 local matrix `M` (rows x, y, z; translation row at +0x40) |
| +0x50 | 4×4 draw matrix `D` |
| +0x80 | view-space position (billboards) |
| +0x90, +0x94 | optional vertex / normal overrides for models (VtMime sets +0x90, §7.4) |
| +0x9C | "spawned this tick" flag |
| +0xA0 | work area (offsets from the program records) |
| work size | one key cursor (pointer) per program |

## 6. Coordinate conventions (checked)

- **Angles** in programs are i32 with **65536 = 360°** (sine table of 4096 entries indexed by
  `angle >> 4`). MegaBirth's float angles are degrees and are converted to the same units.
- **Rotation order.** `MatrixXYZ` applies X, then Y, then Z (column form `Rz·Ry·Rx`);
  `YXZ` = Y, X, Z; `ZXY` = Z, X, Y. Element rotations and the root use X, Y, Z too.
  Rx maps `(y, z) → (y·c − z·s, y·s + z·c)`; Ry `(x, z) → (x·c + z·s, −x·s + z·c)`;
  Rz `(x, y) → (x·c − y·s, x·s + y·c)`.
- **Matrices** are row-vector style (`v' = v·M`, translation in the 4th row). The program
  matrix is `M = S · R · T` in that convention: rows of the rotation matrix scaled by the scale
  vector, translation = position.
- **View space** (camera matrix built in `0x14019BFC0`): z = toward the look point, x =
  `forward × up`, y = `forward × x` (= −up). With KH2's world up `(0,−1,0)` this is **x right,
  y down, z into the screen**. Sprite and shape local y therefore points down the screen.
  (The camera's up vector is data; `(0,−1,0)` is assumed.)
- **Global matrices per effect** (`0x1404b2c70`, `0x1402c27b0`, `0x1402c2680`):
  - `Root` = `T(effect position) · Rz·Ry·Rx(effect rotation) · S(effect scale [× character
    scale])` × particle-data header transform (offset, angle, scale). Its per-axis scale
    lengths `rs = (|row x|, |row y|, |row z|)` are kept for billboards.
  - `V` = camera view, `P` = projection, `VR = V·Root` (column form), `PVR = P·V·Root`,
    `Bill` = an upright camera-facing frame derived from `VR` (checked, `0x1402c27b0`): its
    y axis is `VR`'s y axis (the root's y in view space) normalised; its x axis is
    `(y.y, −y.x, 0)` normalised, i.e. perpendicular to y in the screen plane; z = x × y
    (normalised); origin = `VR`'s translation. The game also keeps `P·Bill`. Only
    DrawMatrixWood (§7.3) uses it among the programs described here.

## 7. Program reference

The types Explosion and Magnet Burst use, plus those of Guard Break, Finishing Leap and
Aerial Finish (§12.1), and (§7.5–§7.10) those of the spell packages the mod plays
(`magic/FIRE_n`, `BLIZZARD_n`, `THUNDER_n`, `CURE_n`, `REFLECT_n.mag`, Sora's `P_EX100.a.us`
and Wisdom Form's `P_EX100_MAGF.a.us`). `w[i]` = the program's i-th work offset;
"state" lives at `+0xA0 + w[i]` of the instance. Keys start with the frame word; offsets
below are from the key start. All checked unless marked.

### 7.1 Integrators

| Type | Name | Key | Work | Tick behaviour |
|---|---|---|---|---|
| 40 | pppPoint | 3×f32 @4 | `w0` position f32×3 | On a key tick: position += key. Construct: 0. |
| 41 | pppAngle | 3×i32 @4 | `w0` angle i32×3 | Same, integer angles. |
| 42 | pppScale | 3×f32 @4 | `w0` scale f32×3 | Same. |
| 43 | pppColor | 4×u16 @4 | `w0` colour | On a key tick the four u16 channels (R, G, B, A, 8.8 with 0x4000 = 128 = 1.0) += key (wrapping). Every tick it writes 4 bytes at `w0+8`: `min(channel × effect colour multiplier, 32767) / 128`, truncated. These bytes are the instance colour (0x80 = 1.0). |
| 36/37/38/39 | pppMove, AngMove, SclMove, ColMove | like Point/Angle/Scale/Color | `w0` value, `w1` velocity | On a key tick velocity += key; then every tick value += velocity. |
| 28/29/30/31 | pppAccele, AngAccele, SclAccele, ColAccele | same | `w0` velocity, `w1` acceleration | On a key tick acceleration += key; then velocity += acceleration. |

The Accele program of a channel comes before its Move, which comes before the value program,
so a tick does: accel (+key) → velocity += accel → (+key) → value += velocity → (+key).
The "effect colour multiplier" is the instance's RGBA (1,1,1,1) with its alpha multiplied by
the fade value (§2).

### 7.2 Random

| Type | Name | Key | Behaviour |
|---|---|---|---|
| 51 | pppRandFloat | @4 i32 target offset (−1 = a global), @8 f32 range, @12 u8 mode | On frame 0 draw `r = 2·rand01` (mode 0) or `rand01 + rand01` (mode ≠ 0), store it at `w0`. On frame 0 and on every later key tick: `target += range·r − range`, i.e. uniform (or triangular) in `[−range, range]`. |
| 66 | pppRandIV | @4 target offset, @8/@12/@16 i32 ranges, @20 u8 mode | On each key tick: new `r` (as above), each of the 3 ints at the target += `trunc(range·r − range)`. |

### 7.3 Matrices

| Type | Name | Work | Builds |
|---|---|---|---|
| 87 | pppMatrixXYZ | `w0` position, `w1` angles, `w2` scale | `M` = rotation X→Y→Z, rows scaled by scale, translation = position. |
| 90 / 91 | pppMatrixYXZ / ZXY | same | other orders. |
| 94 | pppMatrixScl | `w0` position, `w1` scale | `M` = diag(scale) + translation. |
| 105 | pppParMatrix | – | `M.translation = parent.M · M.translation` (point transform). Only the position is inherited. |
| 84 | pppSMatrix | `w0` | Stores a pointer that nothing reads on PC; no effect. |
| 112, 217 | pppDrawMatrix, DrawMatrixLoop | – | `D = PVR · M` (fully 3D oriented). |
| 113 | pppDrawMatrixFront | – | **Billboard**: rotation rows of `M` scaled by `rs`, used as is in view space; translation = `VR · M.translation`; `D = P · that`. The shape faces the camera, keeping only `M`'s own rotation (normally none). |
| 115 | pppDrawMatrixWood | – | **Upright billboard** (calc `0x1404c8020`; its "before a tick" hook `0x1404c80d0` is empty). Rotation rows of `M` scaled by `rs`, translation = `M.translation` (not scaled), then `D = P · Bill · that` (§6). The model turns to face the camera about the root's y axis only (a "tree" billboard). The view position at +0x80 is `PVR · M.translation`. |
| 120 | pppRyjDrawMatrixWorld | `w1` 4×4 saved root | Construct (`0x1404c8440`) copies the current `Root` into the 64 bytes at `w1`. Calc (`0x1404c83d0`): `D = P · V · saved · M`. The instance stays where the root was when it was **spawned**, however the effect moves later (bound elements). Calc runs every call. |
| 124 | pppKeZCrct | `w0` 4×4 out | Key @4 3×f32 offset, @16 3×f32 scale, @28 u8 mode. Rows of `M` scaled by key scale. Mode 0: offset added to the local translation, then `VR·`. Mode 1: `VR·` first, then offset added **in view space**. Mode 2: offset × `rs` after `Root`. Mode 3: camera-facing frame. Out = `P · result`. |
| 125 | pppKeZCrctShp | `w0` 4×4 out | Billboard version: rows of `M` × `rs`, translation × key scale then as above. |

**Z correction (inferred use).** KeZCrct writes a second matrix that is `D` moved along view
z by the key offset (−30 to −80 in these effects, i.e. toward the camera). Draw programs pass
it as a second matrix (§10.3). We believe position comes from `D` and depth from the
corrected matrix (PS2 practice and the names), so the flare is not cut by nearby geometry.
If unsure, drawing everything with `D` and disabling depth test for these effects is close.

### 7.4 Drawing

**142 pppDrawShapeX** (`calc 0x1404cb520`, `draw 0x1404cb3b0`, construct `0x1404cb5f0`).
Key (20 bytes): @4 i32 shape number (0xFFFF = none), @8 i16 animation speed, @12 u8 →
flag 0x800, @13 u8 → flag 0x400, @14 u8 use the Z matrix at `w3`, @15 u8 (0 = depth-sorted
draw, else immediate), @16 u8 (1 = no depth test). Work: `w0` animation state (u16 time,
u16 step, u16 previous step), `w1` colour (bytes at +8), `w2` (matrix input), `w3` Z matrix.
Calc: `time += speed`; if `time ≥ wait(step)`: `time −= wait`, `step += 1`; past the last step:
`time = 0` and step = 0 if the last step has the loop bit, else stay on the last step. The
step drawn is the one *before* this tick's advance. Draw: all sprites of that step's frame with
matrix `D` (and the Z matrix), colour = instance colour (§10).

**141 pppDrawShape**: same with a 16-byte key (@12 sorted flag, @13 no-depth flag).

**137 pppDrawMdl3** (`draw 0x1404ca050`). Key (16 bytes): @4 i32 model number, @8 u8 bits
(bit 1: draw immediately; else bit 0: sort per triangle; neither: sort the model as a whole
by its depth), @9 → flag 0x200, @10 → also
pass `VR·M` (lighting/env, unused here), @11 → flag 0x100, @12 Z matrix at `w2`, @13/@14
passed through. Draws the model with `D`, colour = instance RGB, alpha = instance alpha; skipped
when the instance alpha byte is 0.

**139 pppDrawMdlTs3** (texture scroll; `calc 0x1404ca790`). Key: @4 model, @8..@28 six f32
deltas `(u, du, ddu, v, dv, ddv)`, @32 bits as DrawMdl3 @8, @33/@34/@35 as @9/@10/@11,
@36 Z matrix at `w3`, @37. Work `w2`: `u, du, ddu, v, dv, ddv`. Each tick `du += ddu;
u += du; dv += ddv; v += dv`, and on a key tick the six key deltas are added. The draw
offsets the model's texture coordinates by `(u, v)` **texels** (stored as
`trunc(u·65536) >> (log2 width + 4)` in 1/4096 units).

**219 pppDrawMdlLoopZ** (`calc 0x1404cad40`). Key (40 bytes): @4 model, @8 f32 initial u,
@12/@16 f32 u velocity/accel deltas, @20 initial v, @24/@28 v deltas, @32 i16 blend override
(low byte 0xFF = the model's), @34 bits as DrawMdl3 @8, @35/@36 → 0x200/0x100, @37 env
matrix, @38 Z matrix. Like Ts3, but the state is initialised from the key on the first call
(u from @8, v from @20, rates zero), key ticks add @12/@16 to the u velocity/acceleration and
@24/@28 to v's, every tick the acceleration is added to the velocity and the velocity to the
position, and positions wrap modulo 32768 (see §15).

**201 pppRyjMegaBirth** – the particle burst, §8.

**177 pppVertexApLc** (`calc 0x1404d57f0`). Key (20 bytes): @4 i16 vertex set number,
@6 u8 count per burst, @7 u8 interval (ticks), @8 u8 order (0 = walk the set in order,
1 = random), @12 i32 child emitter index, @16 i32 work offset in the child. Work `w0`: u16
cursor, u16 countdown. When the countdown is 0: spawn `count` children of emitter @12, each
getting the chosen vertex position (model units, the vertex set's model) as 3 floats at child
offset @16 (the child's Point value), then countdown = interval − 1; else countdown −= 1.
Children use ParMatrix, so the vertex is carried into the parent's frame. Keys with count 0
switch spawning off.

**174 pppVertexAp** (`calc 0x1404d5520`, construct `0x1404d5700`, spawn `0x1404d5410`).
Same key, work, countdown, order and fast-forward rule as VertexApLc, with one difference:
the chosen vertex is **transformed by the spawning instance's `M`** (as a point) before it
is written to the child's offset @16, so the child gets an effect-space position and needs no
ParMatrix. Construct zeroes `w0`; there is **no reset slot**, so the countdown and cursor
carry over an instance loop (VertexApLc has one). Finishing Leap (element 34) spawns its
five upward flame spikes this way, one per vertex of a pentagon vertex set flattened by the
spawner's scale `(0.18, 0, 0.18)` into a ring around Sora; Guard Break (element 31) spawns its
fan of light every 3 ticks at the origin (spawner scale 0.0001).

**49 pppVtMime** – vertex morph between two models (calc `0x1404c3330`, draw
`0x1404c31a0`, construct `0x1404c33f0`, reset `0x1404c3430`, destruct `0x1404c3450`). It
has a draw slot, so it runs in the draw pass. Key (24 bytes): @4 i32 model A, @8 i32 model B
(both 0xFFFF = off), @12 f32 `t` delta, @16 f32 rate delta (below −1e38, e.g. −FLT_MAX, means
"leave the rate"), @20 f32 acceleration delta. Work `w0`: f32 `t`, f32 rate, f32
acceleration, u32 handle of a vertex buffer.
- Calc (once per tick, not while fast-forwarding): `rate += accel; t += rate`; on a key
  tick `t += @12`, `rate += @16`, `accel += @20`.
- Draw: on first use allocate a buffer of model A's vertex count; fill it with
  `trunc(A[i] + (B[i] − A[i]) · t)` per component, stored as i16; set the instance's
  vertex override (+0x90) to it.
- Every later DrawMdl draw of that instance (DrawMdl3, Ts3, LoopZ) passes the override, and
  the model packet (`0x1404b8250`) uses it **in place of the drawn model's own vertex list**
  (the vertex count stays the drawn model's).
- Construct and reset zero `t`, rate and acceleration (construct also clears the handle);
  destruct frees the buffer.
Aerial Finish uses it to open its swirl: model 24 morphs into model 23 (the one drawn, both
246 vertices) as `t` runs from 0 to about 1 over the first ticks.

**241 pppEiDrawShipoly** – ribbon trail (`calc 0x1404bb5f0`, `draw 0x1404bac50`). Records
two points per tick (`±width/2` along the instance's local y, key @12 width) from the
instance matrix into a ring buffer (key @11 length, 4..128), optionally into root or another
space (@17), can drift / attract the history (@29, @30 with key vectors), and draws a strip
through the history with spline smoothing (key @18 subdivisions), textured from a DPD model's
texture (key @4 model number). **Partially traced**; see Unknowns.

### 7.5 Shape trails (the spells' flame and light tails)

Fire's main elements (0, 2, 5, 8–12 of `magic/FIRE_n.mag`), Blizzard's shard (element 2) and
Reflect's bursts (elements 4–16 of `REFLECT_n.mag`) draw their tails with these two programs.
Both have a calc slot and a draw slot, so both run in the draw pass (§5.5); the calc records,
the draw walks the record. All **checked** in the code unless marked.

**157 pppKeShpTail3X** (calc `0x1404d04a0`, draw `0x1404d08e0`, construct `0x1404d0830`, reset
`0x1404d08a0`). Work `w0` (offsets from it):

| Off | Type | Meaning |
|---|---|---|
| +0x00 | 4×i16 | head colour R, G, B, A (8.8 like pppColor: 0x4000 = 0x80) |
| +0x08 | 4×i16 | tail colour |
| +0x10, +0x18 | 4×i16 each | head colour velocity, acceleration |
| +0x20, +0x28 | 4×i16 each | tail colour velocity, acceleration |
| +0x30 | 28 × 16 bytes | ring of past positions (x, y, z, w = 1) |
| +0x1F0 | u32 | animation time |
| +0x1F8 | u16 | random seed (one C `rand()` at construct) |
| +0x1FA | u8 | index of the newest point (counts down, wraps 0 → 27) |

`w1` is the instance colour (pppColor's work; only its alpha byte is read), `w3` the Z matrix.
Key (0x58 bytes):

| Off | Type | Meaning |
|---|---|---|
| @4 | i32 | shape number (0xFFFF = off) |
| @8 | i32 | animation speed (added to the time once per drawn tick) |
| @0xC, @0x10 | f32 | size of the first and of the last sprite |
| @0x14 | f32 | random size change: each sprite is `size · (1 − r · @0x14)`, `r` in 0..1 |
| @0x18 | f32 | spacing between sprites along the trail (at the first sprite's size) |
| @0x1C | u8 | sprite count |
| @0x1D | u8 | draw a sprite at the newest point first (else the count includes it but it is skipped) |
| @0x1E | i16 | non-zero: turn each sprite by its random value |
| @0x20 / @0x28 | 4×i16 | deltas added on a key tick to the head / tail colour |
| @0x30 / @0x38 | 4×i16 | to the head colour's velocity / acceleration |
| @0x40 / @0x48 | 4×i16 | to the tail colour's velocity / acceleration |
| @0x50, @0x51 | u8 | → draw flags 0x800 / 0x400 (as DrawShapeX @12/@13) |
| @0x52 | u8 | use the Z matrix at `w3` |
| @0x53 | u8 | record in world space (the point through `Root`) and draw through `V`; else effect space, drawn through `V·Root` |
| @0x54 | u8 | 0 = depth-sorted draw, else immediate |
| @0x55 | u8 | 1 = no depth test |

- *Calc* (once per tick): on the instance's frame 0, if it has never looped (instance byte
  +0x9D, set at every instance loop), all 28 points are set to the current position. Then the
  newest index moves back by one and the current position (`M`'s translation, or `Root ·` it)
  is written there. Each colour channel: `velocity += acceleration; value += velocity`
  (wrapping 16-bit), then on a key tick the key's six deltas are added.
- *Draw* (`0x1404d08e0`): colours are the channels `>> 7` (bytes, 0x80 = 1.0), alphas times
  the instance alpha / 128. With `n` = the count, the colour steps by `(head − tail)/(n−1)`,
  the size by `(first − last)/(n−1)` and the spacing by `size step / first size · spacing`.
  Starting at the newest point it walks back through the ring: before each further sprite the
  colour, spacing and size step; it stops when the count runs out, the spacing reaches 0 or
  it would have to pass the newest point again (the history ran out). The sprite goes at
  distance `offset` into the current segment (linear), where `offset` starts at the key
  spacing and grows by the current spacing after each sprite, while the "enough history"
  check uses the remaining length minus the current spacing. (So the second sprite sits one
  full spacing back; the slight mismatch is the game's.)
- Each sprite: a 16-bit generator `seed = seed · 0x80D + 7` (from the stored seed, so every
  sprite keeps its value from frame to frame) gives `r = seed / 65535` for the size change,
  the rotation angle `seed` (65536 = one turn, sine table) and the animation phase: the step
  shown is `((time + seed) / wait of step 0) mod steps`. The sprite matrix is a billboard:
  axes `M.x · rs.x · size` and `M.y · rs.y · size` used as they are in view space (turned by
  the angle: `x' = x·cos + y·sin`, `y' = y·cos − x·sin`), z axis zero, position `V·Root·p` (or
  `V·p` in world mode); the shape is drawn with the packet colour (the trail colour, not the
  instance colour).
- After drawing, the time += @8 (only on the first drawn frame of a tick).
- Reset (instance loop) clears the 48 colour bytes; the ring, seed and time stay.
- Fire 1's element 0: shape 10, 15 sprites 6 units apart, size 2 → 1.5, ±40 % size, turned,
  orange (0x83, 0x7B, 0x5D, 0x80) to faint red (0x5D, 0x33, 0x22, 0x10), world space, Z offset
  −30 (KeZCrctShp): about 75 units of flame behind the ball.

**155 pppKeShpTail2X** (calc `0x1404cef40`, draw `0x1404cf150`, construct `0x1404cf120`, no
reset). The same trail with fixed colours and one animated shape. Work `w0`: u8 ring length
(construct: 31), u8 newest index, u16 animation time, u16 step, u16 step drawn, ring at +0x10
(31 × 16 bytes). Key (0x2C bytes): @4 shape, @8 i16 animation speed, @0xC/@0x10 first/last
size, @0x14 4×u8 head colour, @0x18 4×u8 tail colour, @0x1C f32 spacing, @0x20 u16 count,
@0x22 draw the first sprite, @0x23/@0x24 → 0x800/0x400, @0x25 Z matrix at `w3`, @0x26 world
space, @0x27 0 = sorted, @0x28 1 = no depth test.
- Calc: the same ring recording (in effect space only x, y, z are written), then the shape
  animation like DrawShapeX but reading the wait and loop bit of the step it is on (no lag):
  `drawn = step; time += speed; if time ≥ wait(step): step += 1, time −= wait; past the last
  step: time = 0, step = 0 if that step loops else stays`.
- Draw: the same walk; all sprites show step `drawn`, are batched, and have the axes of `M`
  (x, y and z) times `rs` and the size.

**Not traced**: what the PC's shape packet does with the instance's Z matrix for sprites that
are elsewhere along the trail; `kh2-fx` applies the instance's own depth shift to all of them.

### 7.6 Loop integrators (Blizzard's shard, Wisdom's Blizzard)

Same keys and work as Point / Angle / Scale / Move / AngMove (§7.1); what differs is what an
instance loop (§5.6) does to them. **Checked.**

| Type | Name | calc, construct, reset | Difference |
|---|---|---|---|
| 210 | pppMoveLoop | `0x1404c1f00`, `0x1404c1ff0`, `0x1404c2010` | While fast-forwarding after a loop (`0x142b0ee24` set) the velocity takes its keys but the value is not advanced. Construct and reset clear only the velocity (`w1`). |
| 211 | pppAngMoveLoop | `0x1404c2030`, `0x1404c20b0`, `0x1404c20d0` | Same for integer angles. |
| 213 | pppPointLoop | `0x1404c2390`, `0x1404c2460`, `0x1404c2490` | On its first run (a key handle at `w0 + 12` still 0) the value is cleared and the current key remembered at `w0 + 12`. Key ticks add as Point does. Reset **subtracts** the remembered key; the replay from frame 0 adds it back, so the value runs on across loops instead of restarting. |
| 214 | pppAngleLoop | `0x1404c2590`, `0x1404c26c0`, `0x1404c26f0` | As PointLoop with integer angles; after a key each angle above 0x10000 is brought down by whole turns (`while a > 0x10000: a −= 0x10000`). |
| 215 | pppScaleLoop | `0x1404c27d0`, `0x1404c28a0`, `0x1404c28d0` | As PointLoop. |

For comparison, the plain Move's construct and reset (`0x1404c1d30`) also clear only the
velocity, and Point's (`0x1404c2370`) the value.

### 7.7 Random vectors (Thunder, Wisdom's Thunder)

**Checked.** Key: @4 i32 target offset (−1 = a scratch global), @8 three f32 ranges, @20 u8
mode. Calc, not on a computed frame: on instance frame 0 the factors are drawn and stored at
`w0`; on frame 0 and on every later key tick each target float gets an addition.

| Type | Name | Calc | Factor(s) at `w0` | Added to target `i` |
|---|---|---|---|---|
| 63 | pppRandFV | `0x1404c4000` | one: `2u` (mode 0) or `u + u` | `range_i · r − range_i` |
| 75 | pppSRandFV | `0x1404c4d60` (factor `0x1404c4e80`) | three, one per axis, as RandFV | `range_i · r_i − range_i` |
| 76 | pppSRandUpFV | `0x1404c4ed0` (factor `0x1404c4fd0`) | three: `u` (mode 0) or `(u + u) / 2` | `range_i · r_i` (0..range) |

`u` is `rand01` (§5.7). So SRandFV scatters each axis independently in ±range and SRandUpFV
in 0..range.

### 7.8 More matrices

All calc-pass programs. **Checked** unless marked.

- **96 pppRyjMatrixNoRot** (`0x1404c6ad0`; Fire 3, Reflect, Sora's element 42). `M` = the
  camera view's inverse (`0x14047a6d0`: rotation transposed, translation inverted), rows scaled
  by the scale at `w2`, translation = the position at `w0` (`w1` is not read). Drawn through
  `V·Root` (DrawMatrix) it cancels the camera's rotation: the instance faces the camera in 3D
  (only the root's own rotation remains). MegaBirths under it emit in that frame. `kh2-fx`
  uses the view of the last frame built (the calc runs before the frame's camera is known);
  a host can call `Effects::set_camera` before `tick` to remove the one-frame lag.
- **106 pppKeParMatR** (`0x1404c7ae0`; children of Sora's element 42, Thunder 3). ParMatrix
  that also inherits the parent's rotation without its scale: the translation becomes
  `parent.M · M.translation` (as ParMatrix), and `M`'s axes are rotated by the parent's three
  axes normalised (`M = R_parent · M`). The key is not read.
- **85 pppKeMatSN** (`0x1404c5d70`, construct `0x1404c5e30`; Cure 3). Builds, at `w0 + 0x40`,
  the rotation (X, Y, Z order) by `previous angles − the angles at w1` with a (0, 0, 0, 1)
  translation row, stores the angles at `w0 + 0x80` for next time and points a global at the
  matrix. That global (`0x142b0edf8`) is written only by this and pppSMatrix and never read on
  PC, so the program has no visible effect; `kh2-fx` fills the work area only.
- **116 pppKeDMat** (`0x1404c80e0`) and **117 pppKeDMatFr** (`0x1404c8150`; Sora's and
  Wisdom's elements 1, 110–112). DrawMatrix and DrawMatrixFront (§7.3) that also copy the view
  space matrix (`V·Root·M`, or the billboard one before `P`) into the 64 bytes at `w1`. The
  distortion programs (§7.10) read it from there.

### 7.9 More model draws

- **138 pppDrawMdlSemi3** (draw `0x1404ca2b0`; Sora's element 1). DrawMdl3 (§7.4) whose blend
  always comes from the key: @4 model, @8 u16 = GS ALPHA low byte and FIX (in place of the
  model's), @10 bits as DrawMdl3 @8, @11 → 0x200, @12 environment matrix, @13 → 0x100, @14 Z
  matrix at `w2`. Colour from `w0`; skipped when the instance alpha is 0. **Checked.**
- **218 pppDrawMdlLoop** (calc `0x1404ca8f0`, draw `0x1404caae0`, construct `0x1404caa80`,
  reset `0x1404caab0`; Wisdom's element 20). DrawMdlLoopZ (§7.4) without the Z matrix: the
  same key up to @37, the same scroll calc (u from @8 and v from @20 on first use, velocities
  and accelerations from the key, wrapped at 32768, held while fast-forwarding). **Checked.**
  Its reset (and LoopZ's, `0x1404caf00`) clears only the velocities and accelerations: the
  scroll position and the "initialised" word carry over the loop.

### 7.10 KeBornRnd6 and the distortion programs

**184 pppKeBornRnd6** (calc `0x1404d6b90`, construct `0x1404d6c50`, picker `0x1404e3170`, per
child `0x1404d6c70`; Sora's element 42). VertexApLc's spawner (§7.4) that also turns each
child toward its vertex. Key (28 bytes): @4 i16 vertex set, @6 u8 children per burst, @7 u8
interval (0 = the program does nothing), @8 u8 order (0: in turn from the cursor at `w0`,
wrapping; else `rand() mod count`, from the C `rand`), @12 i32 child emitter (−1 = none), @16
i32 child offset for the vertex, @20 i32 (−1 = none), @24 i32 child offset for three angles.
Work `w0`: u16 cursor, u8 countdown. Not while fast-forwarding, once per tick: if the
countdown is 0 it is set to the interval and (unless @12 or @20 is −1) a burst is spawned;
then the countdown goes down by one. The vertices come from the instance's VtMime override
when it has one, else from the set's model. Each child gets:
- the vertex `(x, y, z)` (floats, model units) at @16;
- at @24 + 8 the z angle that turns its y axis toward the vertex in the x-y plane, and at @24
  + 0 the x angle of the vertex's elevation; both are atan-based quadrant cases written out in
  the code (`kh2-fx`: `born6_angles`); an angle whose vector is zero is left as it is; @24 + 4
  (y) is set to 0. Angles: `trunc(a · 32768 / π)`.

**160 pppKeShpDtt** (calc `0x1404d1450` (empty), draw `0x1404d1480`, construct/reset
`0x1404d1460`) and **238 pppSnoMdlDttNorm** (draw `0x1404cd590`, construct `0x1404cd550`,
reset `0x1404cd570`, packet `0x1404cc710`): **screen distortion**. Both build GS packets whose
texture is the frame buffer itself (a `TEX0` made from the display buffer's address and
width), so they redraw what is behind a quad (Dtt) or a model's triangles (DttNorm, offset
along the model's normals) with shifted coordinates: a heat-haze ripple. Their state is a
strength `t`, its rate and acceleration at `w0` (`rate += accel; t += rate`, plus key deltas:
KeShpDtt @4, @8, @12 once per drawn tick; SnoMdlDttNorm @8, @12, @16 on every drawn call, both
frames of a tick, no computed check). KeShpDtt reads the view matrix KeDMatFr left at `w2`;
key @0x10/@0x14 scale the quad by `rs`, @0x18 picks one of two frame buffers, @0x19 flags.
SnoMdlDttNorm: @4 model, @0x15, @0x16 (scale the strength by `rs.x`), @0x17, @0x1C. **Inferred**
(the packet code was skimmed, not traced in full). `kh2-fx` runs their state but draws nothing
(`kh2_fx::draws_nothing`): its output is textured triangles, which cannot read the frame.

## 8. MegaBirth (type 201): sprite particle bursts

`calc 0x1404bdda0` → `0x1404bd5d0`, birth `0x1404bc630`, update `0x1404bd2a0`, draw
`0x1404bd7a0`, per-particle matrix `0x1404be100`. Work: `w1` colour (instance colour bytes at
+8), `w2` state (emitter base matrix, drift, particle arrays, counters).

### 8.1 Parameters (220-byte key)

| Off | Type | Meaning |
|---|---|---|
| 0x04 | i32 | shape number (0xFFFF = off) |
| 0x08 | i16 | shape animation speed (per particle) |
| 0x0C/0x0D | u8 | → draw flags 0x800 / 0x400 |
| 0x0E | u8 | use the view-space Z offset at 0x10 |
| 0x0F | u8 | 0 = depth-sorted draw, else immediate |
| 0x10 | 3×f32 | view-space offset for the Z matrix |
| 0x1C | u16 | particle slots |
| 0x1E | u16 | births per burst |
| 0x20 | u16 | burst interval (ticks) |
| 0x22 | u16 | particle life (ticks; 0 = endless) |
| 0x24 | u8 | fade-in ticks |
| 0x25 | u8 | fade-out ticks |
| 0x26 | u8 | emission type (below) |
| 0x27 | u8 | cone spread, degrees |
| 0x28 | 4×f32 | colour velocity (RGBA, colour units) |
| 0x38 | 4×f32 | colour acceleration |
| 0x48 | 4×u8 | random colour ranges |
| 0x4C | 2×f32 | scale x, y |
| 0x58 | 2×f32 | scale velocity x, y |
| 0x64 | 2×f32 | scale acceleration x, y |
| 0x70 | 2×f32 | random scale ranges |
| 0x7C/0x80/0x84 | f32 | spin angle, velocity, acceleration (degrees) |
| 0x88 | f32 | random spin range (degrees) |
| 0x8C | 3×f32 | base direction |
| 0x98 | 3×f32 | drift direction (normalised) |
| 0xA4/0xA8/0xAC | f32 | speed, acceleration, random range (±) |
| 0xB0/0xB4/0xD8 | f32 | drift speed, its acceleration, random range (±) |
| 0xB8 | f32 | emission radius |
| 0xBC | 3×f32 | per-axis scale of direction / position |
| 0xC8 | u8 | radius distribution (0 fixed; 1 `r·u`; 2 `r·u·u`; 3 `r − 0.7·r·u·u`; 4 `r·u⁴`; 5 `r − 0.5·r·u³`, `u` = rand01) |
| 0xC9 | u8 | per-particle colour velocity arrays on |
| 0xCA | u8 | scale random flags |
| 0xCB | u8 | spin random flags |
| 0xCC | u8 | matrix mode (0..3, §8.4) |
| 0xCD | u8 | 1 = no depth test |
| 0xCE | u8 | speed may change sign (else clamped at 0 when the acceleration would flip it) |
| 0xD0 | i16 | vertex set number (types 6..9) |
| 0xD2 | u8 | random mirroring of the sprite: the scale x (1), y (2) or each (3) is negated with probability ½ |
| 0xD3 | u8 | grow-in ticks |
| 0xD4 | u8 | shrink-out ticks |

### 8.2 Each tick

`counter += 1`. For each slot in order: a dead slot (life 0) is born if
`counter ≥ interval` and fewer than "births per burst" were born this tick; a live slot is
updated and its shape animation advanced (same rule as DrawShapeX). If anything was born,
`counter = 0`. The base matrix `B` is refreshed every tick: for emission types 1, 3, 5, 7, 9
it is `diag(rs)` with the root translation (follows the effect but not its rotation), else it
is `Root`. Particles live inside the instance: when the instance dies, all its particles go.

### 8.3 Birth

- **Direction** (types < 8): three angles each uniform in `±cone` degrees (types 2 and 3: the
  x **and** y angles are 0, only z is random), a rotation applied to the base direction,
  multiplied per axis by 0xBC, normalised. This rotation is built Z first, then Y, then X
  (the opposite order to MatrixXYZ; §15).
- **Position**: types 0–3: `direction × radius × distribution`. Types 4–5: each axis
  `distribution(radius) − radius/2`, times 0xBC. Types 6–9: a vertex of the vertex set's model
  (in order, or random for the distribution modes), times 0xBC; types 8 and 9 also set
  direction = normalised position.
- **Colour offsets**: per channel `trunc(rand01 · 2b − floor(b/2))` stored as a signed byte
  (`b` = range at 0x48+i). This is asymmetric and can wrap; that is what the PC code does.
- **Scale**: `(sx, sy) = 0x4C`; random `rx = rand01·0x70`, `ry = rand01·0x74` (flag 0x20: the
  same draw for both); sign: flags & 3 = 3 → random sign each, & 2 → negative; flag 4 adds
  `(rx, ry)` to the scale, flag 8 to the scale velocity, flag 0x10 to the acceleration every
  tick. Scale velocity starts at 0x58.
- **Spin**: angle 0x7C, velocity 0x80 (degrees → 65536 units); random `±` (flags 0xCB like the
  scale flags, bit 4/8 = add to angle/velocity, 0x10 = add to acceleration).
- **Speed** `0xA4 ± 0xAC`, drift speed `0xB0 ± 0xD8`.
- **Life** = 0x22 (0 → endless). Fade-in: alpha subtract starts at the instance alpha;
  grow-in: size factor starts at 0.
- Matrix mode 1/2/3: remember `B`, `M` or `B·M` at birth.

### 8.4 Update (per tick, live particles)

```
colour_offset += colour_velocity;  colour_velocity += 0x38          (if 0xC9)
angle += spin;  spin += 0x84 (+ random)                             (16-bit angles)
scale += scale_velocity;  scale_velocity += 0x64/0x68 (+ random)
speed += 0xA8   (clamped at 0 when it would change sign, unless 0xCE)
drift += 0xB4
position += direction × speed;  position += drift_direction × drift
life −= 1 (if finite);  age += 1
fade-in:   while age ≤ fade_in:  alpha_sub −= alpha / fade_in
fade-out:  while life ≤ fade_out: alpha_sub += alpha / fade_out
grow-in:   while age ≤ grow_in:   size = age / grow_in
shrink:    while life ≤ shrink:   size = life / shrink
```

### 8.5 Draw

Every particle is a **camera-facing quad**: in view space, scale `(rs.x·sx·size,
rs.y·sy·size)`, rotated by `angle` around the view axis, at the view-space position of the
particle:

| Matrix mode | View-space position |
|---|---|
| 0 | `V · B_now · M_now · p` |
| 1 | `V · B_birth · M_now · p` |
| 2 | `V · B_now · M_birth · p` |
| 3 | `V · (B·M)_birth · M_now · p` |

Then `P ·`. With 0x0E set, a second matrix moved by the view-space offset 0x10 is built (Z
correction). The shape step drawn is the particle's own animation step. Colour per particle:
`rgb = instance rgb + colour offset (+ colour velocity array)`, clamped 0..255;
`alpha = instance alpha + alpha offset − alpha_sub`, clamped 0..127.

## 9. Textures

Layout per OpenKH (checked against the data):

```
+0x00 u16 tbp   +0x02 u16 cbp   +0x04 i16 buffer width   +0x06 i16 format
+0x08 i16 x, y  +0x0C u16 width, height   +0x10 u64 TEX0   +0x18 i32 clut start
+0x1C i16, i16 VRAM sizes   +0x20 pixels   then palette (4 bytes per colour)
```

`x, y` is the upload position inside the buffer that starts at `tbp` (the GS transfer's
destination offset): small textures share a page (§15.6).

- Format 0x13: 8-bit indices, 256 colours. 0x14: 4-bit (low nibble first), 16 colours.
- **Pixels are linear** (row-major), not GS-swizzled.
- **256-colour palettes are in GS CSM1 order**: palette entry for index `i` is at
  `i` with bits 3 and 4 swapped. (Without this the image is speckled; checked visually.)
- Colours are RGBA with **alpha 0x80 = opaque**; double it for 8-bit alpha.
- Effect textures are 128×64 at 8 bits: exactly one PS2 8-bit **page**. A shape or model
  `TEX0` may span several pages: `TBW` (×64 px) is the row width, `TW`/`TH` the size. Page
  `k` of the texture lives at `TBP + 32k` and sits at column `k mod (TBW/2)`, row
  `k div (TBW/2)` (each page 128×64). Build the atlas by placing each DPD (or common) texture
  whose `tbp` falls in that range at its page position. Sprites in Explosion use a 256×512
  common atlas (`TBP 0x2A00`, `TBW 4`). The palette is the texture whose `cbp` equals
  `TEX0.CBP` (+ the sprite's palette offset byte).
- **PC remaster (checked, `0x140104fb0`).** The PC build does not upload these pixels when a
  remastered replacement exists: the PKG entry's HD asset list is searched for an asset whose
  original offset equals the texture data's offset in the file, and that HD image is used
  instead (same UVs, normalised). The PS2 data decodes correctly and is a faithful fallback;
  using the HD assets would need the egs reader to expose them (see Unknowns).

## 10. Shapes (sprites)

### 10.1 Layout (OpenKH names, our meanings)

```
shape +0x00 i16 number   +0x02 i16 TEX0 base (0 in files)   +0x04 i16 CLUT base
      +0x08 i32 packet size
anim  = shape + 0x10:  u8 id, u8 version, i16 flag, i16 sprite-set count, i16 step count (@+6),
                       ..., then step records at anim+0x10, 8 bytes each:
      i16 frame offset (from anim), u16 wait, u8 flags (bit 7: loop to step 0), 3 pad
frame (anim + offset): i16 flags (bit 3: free quads), i16 sprite count,
                       i32 attribute offset (from the frame), 8 bytes,
                       u64 TEX0 @+0x10, 8 bytes, sprites @+0x20
sprite (flag 8 clear, 16 bytes): i16 x0, y0, x3, y3, s, t, w, h
sprite (flag 8 set,   24 bytes): i16 x0, y0, x1, y1, x2, y2, x3, y3, s, t, w, h
attribute (8 bytes per sprite):  R, G, B, A (0x80 = 1.0), ALPHA register low byte,
                                 ALPHA FIX, a code byte, palette offset (added to CBP)
```

### 10.2 Quad construction (checked, `0x1404af1a0`, `0x1404af400`)

- Positions are **12.4 fixed point** (`/16`), z = 0, in the plane of the draw matrix.
- Corners: 0 = top-left `(x0,y0)`, 1 = top-right `(x1,y1)` (`(x3,y0)` for rectangles),
  2 = bottom-left `(x2,y2)` (`(x0,y3)`), 3 = bottom-right `(x3,y3)`.
- Texture coordinates are **normalised, 4096 = the TEX0 texture size** (`2^TW × 2^TH`):
  top-left `(s, t)`, top-right `(s+w, t)`, bottom-right `(s+w, t+h)`, bottom-left `(s, t+h)`.
  (Checked by rendering Magnet shape 17 against its 128×64 texture.)
- The four vertices are drawn as one quad (TL, TR, BR, BL) with the sprite's colour.

### 10.3 Draw state (checked, `0x1404afc10`, `0x1404b03a0`, defaults `0x1404b0fd0`)

- The draw packet holds `D` (matrix 0), the Z-corrected matrix or `D` again (matrix 1), the
  instance colour as floats `/128`, and the shape frame. The PC renderer multiplies both
  matrices by `inverse(P·V)` and passes world matrices plus `P·V` to its shader.
- **Blend** per sprite from the GS ALPHA register byte: `Cv = (A − B)·C + D` with A, B, D ∈
  {0 = source, 1 = destination, 2 = zero} and C ∈ {0 = source alpha, 1 = destination alpha,
  2 = FIX}, two bits each from the low bit up. Common values: **0x48 = additive**
  (`src·As + dst`), 0x44 = normal alpha blend, 0x42 = subtractive (`dst − src·As`). Alpha
  0x80 = 1.0 in this equation.
- **Depth**: default test on (pass if nearer), **no depth write**; draw flag 0x10 (the
  "no depth test" key bytes) turns the test off. Writing depth needs flag 0x100 without 0x10.
- **Alpha test**: discard alpha 0.
- **Order**: immediate draws go out in program order. Sorted draws (key byte 0) go into 256
  buckets by projected depth and are drawn back to front (bucket direction inferred).
- **Colour** (PS2 semantics, which the PC reproduces; the exact PC shader was not read):
  `rgb = texture.rgb × (sprite rgb / 128) × (instance rgb / 128)`,
  `alpha = (texture.a / 128) × (sprite a / 128) × (instance a / 128)`; values above 1 are
  allowed before blending and clamp at the output.

## 11. Models

### 11.1 Layout

```
model +0x00 i16 number   +0x02 i16 TEX0 base   +0x04 i16 CLUT base   +0x08 i32 packet size
OMD = model + 0x10:
  i32 flags (bit 0 set: no depth write; bit 1 clear: alternate cull mode; bit 2: see Unknowns)
  u32 -> primitive groups, u32 -> vertices, u32 -> normals   (from the OMD start)
  i16 primitive count, i16 vertex count, i16 normal count
  u8 ALPHA register low byte, u8 ALPHA FIX, ...
group: u8 flag, u8 kind (0xFF ends), i16 count, u8 tno, u8, 2 bytes, u64 TEX0, then prims
vertices: i16 x, y, z (model units, no scale)
```

Primitive kinds (sizes checked in `0x1404b8250`):

| Kind | Name | Bytes | Contents (in order) |
|---|---|---|---|
| 0 | G3 | 0x14 | 3 colours, 3 vertex indices, flag |
| 1 | G3L | 0x1C | + 3 normal indices, pad |
| 2 | GT3 | 0x20 | 3 colours, 3 indices, flag, 3 × (s, t) |
| 3 | GT3L | 0x28 | + normals |
| 4 | G4 | 0x18 | 4 colours, 4 indices |
| 5 | G4L | 0x20 | + 4 normals |
| 6 | GT4 | 0x28 | 4 colours, 4 indices, 4 × (s, t) |
| 7 | GT4L | 0x30 | + 4 normals |

- Triangles are drawn as triangles; quads as a 4-vertex **strip** (v0 v1 v2, v1 v2 v3).
- Colours per vertex, 0x80 = 1.0; textured kinds use **normalised (4096) s, t** like shapes
  plus the program's scroll offset.
- Draw state: blend from the OMD ALPHA byte unless the program overrides it; depth write
  only if OMD bit 0 is clear; depth test as shapes. The instance colour multiplies (RGB, A /
  128). If a primitive's first vertex alpha is above 0x80 the PC draws it in three passes (a
  glow trick; not used by these models).
- **Vertex sets** (`VsfNo i32, model i32, count i32, u16 indices`) list model vertices used by
  VertexApLc and MegaBirth types 6–9.

## 12. Explosion and Magnet Burst

From `P_EX100.a.us` (`p_ex`). All five Explosion and both Magnet elements live in the
4th DPD of the PAX. "common" = texture in `texcommon.dpd`.

### Explosion (motion 166)

| Element | Effect → data | Bone | Flags | Contents |
|---|---|---|---|---|
| 44 | 31 → 11 | 0x4003 | 0xA61 | 30 ticks. Big flare (shape 20 via DrawShapeX, ticks 0–19) + flat glow model 17 (ticks 0–19), a second flash at tick 17 (shapes 12 and 20, model 17), and MegaBirth sparks (shape 13, 12 slots, one per tick, life 12, radius 30, full-sphere cone, matrix mode 1). All additive, all common textures, all billboards with Z offset −30/−40. |
| 45 | 32 → 12 | 67 | 0xA61 | 30 ticks. Flare (shape 20), MegaBirth dust (shape 16, 32 slots, life 16, radius 80, rising drift), four 10-tick flashes (shape 8). |
| 46, 47 | 33 → 13 | 170 / 78 | 0xA61 | 10 ticks. Two 10-tick flashes (shape 8). Rotated ±180° about z. |
| 48 | 34 → 14 | 0x4000 | 0x243 | 30 ticks. The burst: model 19 with texture scroll (DrawMatrix, 3D, from tick 6), an endless spawner that emits children at the vertices of vertex set 11 (shockwave sprites: shapes 9 (local tex), 10, 11, 12; a ribbon trail; model 28) and MegaBirth sparks (shape 13). Fade-out 8 frames. |

### Magnet Burst (motion 197)

| Element | Effect → data | Bone | Flags | Contents |
|---|---|---|---|---|
| 51 | 37 → 17 | 0x4000, y −127 | 0x261 | 41 ticks. Rotating rings and spheres: models 27, 30 (local textures, scrolled), 28, 31, 32 (common), flare shape 20, MegaBirth swirl (shape 15, 24 slots, 2 per tick, life 14, random spin) and dust (shape 16, 32 slots, speed −20, radius 500, no depth test), plus two endless vertex spawners (sets 13, 14) feeding pink glows (shapes 17, 18, local), small flares (shape 19) and ribbon trails (model 29 texture). Fade-out 4 frames. |
| 52 | 38 → 18 | 0x4000, y −125 | 0x003 | 25 ticks. Two MegaBirths that put 16 growing sparks at the vertices of model 18 (shape 21, emission type 8, no speed), a MegaBirth of shape 22 (radius 130, every 2nd tick), flare shape 20, models 34, 35, 36 (common, 35/36 scrolled). |

Run `pax_fx` on the file for the full program lists.

### 12.1 Guard Break, Finishing Leap, Aerial Finish

Same DPD. Only the parts that use the programs added for them are listed.

| Motion | Element | Bone, offset | Uses |
|---|---|---|---|
| Guard Break (165) | 31 | 0x4000, y −77, bound | VertexAp (every 3 ticks) spawns fans of light rays (model 37, a flat ±800 fan, texture-scrolled) drawn with RyjDrawMatrixWorld, so each stays where it was spawned while Sora lunges. |
| Guard Break | 32 | 0x4000, y −90, z −235, rotated −90° about x | Two DrawMatrixWood models (1, 2: long along the root's y, which the rotation lays along the thrust axis) spiral around the thrust line and turn to face the camera about it; a stretched flare (shape 19) along the same axis. |
| Finishing Leap (167) | 34 | 0x4000, z −9 | Eight VertexAp spawners. Five each place one flame spike (model 7, DrawMatrix, ZXY angles tilting it up and out, spinning about its long axis, about 340 units long at full size) at one corner of a pentagon flattened into a ring of radius about 160 at y −80. The other three put streaks (shape 2), white flares (shape 19) and, from tick 10, gold MegaBirth sparks at all five corners. Plus one DrawMatrixWood model (emitter 18). |
| Aerial Finish (194) | 35–38 | 0x4000, rotated swirls | VtMime morphs model 24 → 23 in the DrawMdlTs3 swirl rings. |

### 12.2 The spells

Programs the spell packages use beyond §12 and §12.1, by the elements the mod plays (all
three levels of each spell unless a level is named):

| Package | Elements | Programs |
|---|---|---|
| `FIRE_n.mag` | 0, 2, 5, 8–12 (the fireballs and their bursts) | KeShpTail3X flame tails (§7.5); Fire 3 element 4 RyjMatrixNoRot |
| `BLIZZARD_n.mag` | 2 (the ice shard) | the Loop integrators (§7.6); Blizzard 1 also MoveLoop and a KeShpTail3X sparkle trail |
| `THUNDER_n.mag` | 0 (the bolt); Thunder 3 elements 3, 4 | SRandFV; KeParMatR; SRandUpFV |
| `CURE_3.mag` | 3 | KeMatSN (no visible effect) |
| `REFLECT_n.mag` | 4–6, 8–10, 14–16; 7 | KeShpTail2X streaks; RyjMatrixNoRot |
| `P_EX100_MAGF.a.us` (Wisdom) | 20; 100; 110–112 | Loop integrators and DrawMdlLoop; SRandFV; KeDMatFr + KeShpDtt (distortion) |
| `P_EX100.a.us` (Sora) | 1; 42; 56; 110, 111 | KeDMat, DrawMdlSemi3, SnoMdlDttNorm; KeBornRnd6, KeParMatR, RyjMatrixNoRot; RandFV; KeDMatFr + KeShpDtt |

With these every element of those packages runs (`kh2-fx`'s test
`spell_packages_fully_supported`, which needs the user's files in `local/re/`).

## 13. Function index

| Address | What |
|---|---|
| 0x14040D050 | Motion trigger dispatcher; trigger 1 starts a PAX element. |
| 0x1402C5FD0 | Start element by id (first match), create the effect, set transform, register. |
| 0x1402BC9B0, 0x1402BE120 | Create effect (DPX lookup by number), allocate its particle-data player. |
| 0x1402C14C0, 0x1402C2910 | Player defaults (colour 1, scales 1); attach particle data. |
| 0x1404B2FF0 | Player init from the program file. |
| 0x1402C1680, 0x1404B20C0, 0x1404B06C0 | DPD load: relocate programs, create textures. |
| 0x1407797C0 | Program table (§5.4). |
| 0x1402BC630 | Per-frame effect update for all effects (delta, camera, bind, tick, fade). |
| 0x1402C1D80 | Start wait and 30 Hz accumulator. |
| 0x1404B29C0, 0x1404B1910 | Calc pass, spawning. |
| 0x1404B22D0, 0x1404B23D0 | Draw pass, frame advance, instance loop/expiry, effect end. |
| 0x1404B1F40, 0x1404B1B10, 0x1404B1E80, 0x1404B1BB0, 0x1404B1CE0 | Fast-forward step, reset, destroy, destroy all, spawn child. |
| 0x1402BCCD0, 0x1402C1F80 | Draw all effects; per-effect draw (VRAM slots, resource tables). |
| 0x1402C0AB0, 0x1402C0FF0, 0x1402C0CA0 | Resource tables by number; shape and model TEX0 relocation. |
| 0x1404B2C70, 0x1402C27B0, 0x1402C2680 | Root matrix; derived matrices; camera matrices. |
| 0x1402C4D90, 0x1402C5080, 0x1402C3E70, 0x1402C5410, 0x1403B43D0 | Element per frame; bone → effect transform; bone matrix; anchor bones. |
| 0x1402BDB40, 0x1402BDD90, 0x1402C67A0 | Fade out; end loop; end on owner gone. |
| 0x1404DE100 | Random number. |
| 0x1404C1C40.. 0x1404C2B30 | Integrator programs (Accele, Move, Point, Angle, Scale, Color). |
| 0x1404C34D0, 0x1404C4340 | RandFloat, RandIV. |
| 0x1404C5E50, 0x1404C61D0, 0x1404C62B0, 0x1404C6A20, 0x1404C7AB0 | MatrixXYZ, YXZ, ZXY, Scl, ParMatrix. |
| 0x1404C7E40, 0x1404C7E80, 0x1404C8BF0, 0x1404C8DC0 | DrawMatrix, DrawMatrixFront, KeZCrct, KeZCrctShp. |
| 0x1404C8020, 0x1404C83D0, 0x1404C8440 | DrawMatrixWood; RyjDrawMatrixWorld calc, construct. |
| 0x1404D5520, 0x1404D5410, 0x1404D5700 | VertexAp calc, spawn, construct. |
| 0x1404C3330, 0x1404C31A0, 0x1404C33F0, 0x1404C3430, 0x1404C3450 | VtMime calc, draw (morph), construct, reset, destruct. |
| 0x1404CB1B0, 0x1404CB3B0, 0x1404CB520 | DrawShape, DrawShapeX draw, shape animation. |
| 0x1404CA050, 0x1404CA500, 0x1404CA790, 0x1404CAF30, 0x1404CAD40 | DrawMdl3, DrawMdlTs3 (draw, scroll), DrawMdlLoopZ (draw, scroll). |
| 0x1404BDDA0, 0x1404BD5D0, 0x1404BC630, 0x1404BD2A0, 0x1404BD7A0, 0x1404BE100 | MegaBirth. |
| 0x1404D57F0, 0x1404D5740 | VertexApLc. |
| 0x1404BB5F0, 0x1404BAC50, 0x1404BC480 | Shipoly ribbon: record, draw, spline. |
| 0x1404D04A0, 0x1404D08E0, 0x1404D0830, 0x1404D08A0 | KeShpTail3X calc, draw, construct, reset (§7.5). |
| 0x1404CEF40, 0x1404CF150, 0x1404CF120 | KeShpTail2X calc, draw, construct. |
| 0x1404C1F00.. 0x1404C28D0 | MoveLoop, AngMoveLoop, PointLoop, AngleLoop, ScaleLoop (§7.6). |
| 0x1404C4000, 0x1404C4D60, 0x1404C4ED0 | RandFV, SRandFV, SRandUpFV (§7.7). |
| 0x1404C6AD0, 0x1404C7AE0, 0x1404C5D70, 0x1404C80E0, 0x1404C8150 | RyjMatrixNoRot, KeParMatR, KeMatSN, KeDMat, KeDMatFr (§7.8). |
| 0x1404CA2B0, 0x1404CA8F0, 0x1404CAAE0 | DrawMdlSemi3; DrawMdlLoop calc, draw (§7.9). |
| 0x1404D6B90, 0x1404E3170, 0x1404D6C70 | KeBornRnd6: countdown, vertex picker, child set-up (§7.10). |
| 0x1404D1480, 0x1404CD590, 0x1404CC710 | KeShpDtt draw, SnoMdlDttNorm draw and packet (frame-buffer distortion). |
| 0x140779730 | The calc slots skipped while fast-forwarding: MegaBirth, `0x1404bfbe0`, Shipoly. |
| 0x1404AFC10, 0x1404B03A0, 0x1404AF1A0, 0x1404AF400 | Shape packet, immediate draw, quads. |
| 0x1404B8250, 0x1404B8AC0, 0x1404B3F10.. 0x1404B5730 | Model packet and primitive emitters. |
| 0x1404B0FD0 | Default GS state (TEST, ZBUF, TEX1, CLAMP, TEXA, FOGCOL). |
| 0x140104FB0 | Texture creation with remastered (HD) replacement lookup. |

## 14. Unknowns and confidence

| Topic | Status |
|---|---|
| Element layer, timing (30 Hz, start wait, fade), program machine, key semantics, integrators, random, matrices, MegaBirth birth/update/draw, VertexApLc, DrawShape(X), DrawMdl* | **Checked** in code; MegaBirth and shape layouts cross-checked against the data. |
| DrawMatrixWood (and the `Bill` frame), RyjDrawMatrixWorld, VertexAp, VtMime (calc, morph, use of the override by the model packet) | **Checked (exe)**; rendered offline for Guard Break, Finishing Leap and Aerial Finish. The look was compared only with descriptions of the game, not side by side. |
| Model → effect world mapping of Sora's pose | **Hypothesis**: the `fx_render` example maps the y-up skeleton by a mirror (y negated) and pins 0x4000 at the origin. Guard Break's element 32 (z −235, axis along −z) then lands behind Sora's lunge; with a half turn about x (`--flip-z`) and the root following the motion (`--follow-root`) it lands on the thrust. The game's own mapping was not traced. |
| Texture decode (linear, CSM1 palette, alpha ×2), normalised UVs, page atlas rule, file-space TBP/CBP matching | **Checked** (decode and UVs visually; atlas and matching from the relocation code and the data). |
| Exact PC pixel/vertex shader maths (vertex colour scale, how matrix 1 is used) | **Inferred**: PS2 modulate semantics; position from `D`, depth from the Z matrix. |
| Camera up vector | Data-driven; `(0,−1,0)` assumed (y down on screen). |
| `effect/texcommon.dpd` | Extracted (`local/re/fx/texcommon.dpd`) and used by `kh2-fx`. Several of its textures share a `TBP`: they are packed side by side in one page through their upload offsets (§15.6). |
| Remastered (HD) textures | The PC uses them when present; finding/decoding them needs the egs reader to expose HD asset headers (name, original offset, sizes). |
| Spell programs (§7.5–§7.9, KeBornRnd6) | **Checked (exe)**; the trails rendered offline for Fire, Blizzard and Reflect and look as described, not compared side by side with the game. The tails' Z matrix use for sprites away from the instance is inferred. |
| KeShpDtt, SnoMdlDttNorm (§7.10) | State **checked**; the drawing (a frame-buffer distortion) only skimmed, and not drawn by `kh2-fx`. |
| EiDrawShipoly (ribbon) | Recording, spline, width, strip, UV and alpha mapping traced (§15). Not traced: the scroll-draw variant (key @8 ≠ 0), mode ≥ 2's exact smoothing, space modes 2/3, the reset hook. None are used by Explosion or Magnet. |
| Model OMD flag bit 2, draw flags 0x400/0x800, model flags 0x100/0x200/0x2000 (lighting, env), sorted-model path details | Not traced. |
| Two-point elements (flag 0x10000), sound ids above 999000 | Not traced (not needed for these effects). |
| Element flags 0x20 on motion change | The owner keeps a list; the code that ends them on motion change was not followed, but it goes through the same end path. |
| `pax.rs` emitter view | Its `Emitter::start == None` means "child emitter", not "starts at loop end"; the effect length is the particle data's end frame (`dpd::ParticleData::end`), and looping is its flag bit. |

## 15. Findings from the `kh2-fx` implementation

Read in the same executable while writing `crates/kh2-fx`, to settle points §1-§14 left open.
Function addresses as before; all **checked** unless marked.

### 15.1 Tick phases

- Within one 30 Hz tick, the first display frame (accumulator below 2) runs the calc pass and
  the draw pass with the "repeat" flag set: programs compute and the instances are marked
  computed. The second frame (accumulator reaches 2) runs both passes again: programs that
  test the computed mark do nothing, but **MegaBirth steps its particles only on this second
  frame** (it tests the repeat flag, not the mark). Then the frames advance and instances
  expire, at the end of that frame's draw pass. So a tick's programs are visible on both frames
  and its particle step on the second.
- Unconditioned programs (the matrix builders, ParMatrix, DrawMatrix*) and the Color
  program's byte output run on both frames; Color's output therefore follows the element fade
  every frame.
- Clock-spawned instances run in the tick they are spawned; instances spawned by a program
  (VertexApLc) are skipped until the frames advance and start at frame 0 on the next tick.
- Instance groups: an emitter's instances are drawn in group order, which is the first
  instance, then the **newest** children, back to the second-oldest (new ones are linked right
  after the group's first instance).
- **Draw slots run only on drawn passes** (`0x1404b23d0`, its second argument): the catch-up
  ticks of a long frame call the calc slots of the draw programs but not their draw slots.
  KeShpTail3X's animation time and SnoMdlDttNorm's strength live in their draw slots, so they
  do not advance on those ticks.
- **Fast-forward** (`0x1404b1f40`) calls every program's calc slot except the three listed at
  `0x140779730` (MegaBirth, `0x1404bfbe0`, Shipoly), never a draw slot, with the flag at
  `0x142b0ee24` set (MoveLoop, AngMoveLoop, the DrawMdlLoop scroll, VertexAp(Lc) and
  KeBornRnd6 test it).
- **Instance byte +0x9D** is 0 for a new instance and set to 1 at every instance loop; the
  shape trails fill their history with the current point only on frame 0 of a new instance.

### 15.2 Programs

- **Accele / Move** (all four channels) use one routine: on a key tick `w1 += key`, then every
  tick `w0 += w1`. Colour channels are u16 and wrap.
- **RandFloat**: on instance frame 0 it draws its factor (`2·u` or `u + u`), stores it at `w0`
  and applies it; afterwards it reapplies the stored factor on key ticks. The target offset
  `-1` writes to a scratch global (ignored). **RandIV** draws anew on each key tick.
- **VertexApLc**: a countdown at `w0+2`. When it is 0, `count` children are spawned and the
  countdown is set to the interval; it is then decremented every tick, so interval 1 spawns
  every tick and interval 0 never again. Order 0 walks the set with a cursor at `w0`, order 1
  picks `trunc(count · u)`. The spawn is skipped while fast-forwarding after a loop.
- **Shape animation** (DrawShape(X), MegaBirth particles): the step's wait and loop bit are read
  from the step drawn last (`w0+4`), not the current one: a one-step lag that only matters when
  waits differ.
- **DrawMdlTs3** state at `w2`: `u, du, ddu, v, dv, ddv`; every tick `du += ddu; u += du;
  dv += ddv; v += dv`, then on a key tick the six key values are added. The draw offsets the
  texture by `(u, v)` texels.
- **KeZCrct (model)**: rows of `M` times the key scale. Mode 0 adds the offset to the local
  translation and applies `VR`; mode 1 applies `VR` and adds the offset in view space; mode 2
  applies `Root`, adds `rs ⊙ offset`, then `V`. **KeZCrctShp** uses the billboard rows
  (`M`'s rows × `rs`) and moves only the translation (times the key scale) the same three
  ways. Explosion and Magnet use mode 1 with offsets −30 to −80, or mode 0 with zero offset.
- **Z correction on PC.** For MegaBirth the PC build keeps the particle's projected position and
  lowers only its projected depth in proportion to the key's view-z offset (divided by the
  clip w and the depth range): position from `D`, depth from the corrected matrix, as inferred
  in §7.3. `kh2-fx` reports the offset per draw for the host renderer to apply to depth only.
- **DrawMdlTs3 / DrawMdlLoopZ resets**: Ts3's reset (`0x1404ca890`) clears its 24 bytes; LoopZ's
  (`0x1404caf00`) only the velocities and accelerations (earlier `kh2-fx` versions cleared
  LoopZ's position and "initialised" word too, restarting the scroll from the key at a loop).

### 15.3 MegaBirth details

- **Allocation** happens on the program's first run with that key's slot count; later keys
  cannot resize it. The base matrix is refreshed on every run (both frames).
- **Particle matrix** (`0x1404be100`): the sprite plane is **rotated by the spin first and
  scaled after** (`(rs.x·sx·size, rs.y·sy·size, rs.x·sx·size)`), so a non-square scale
  stretches the spun sprite along the screen axes. The translation is the particle position
  through the matrix-mode chain of §8.5.
- **Alpha**: `inst a + offset − fade` clamped to 0..127 (not 128); rgb clamped 0..255. The
  fade-out step divides by the key's fade-out length (0x25), the fade-in step by the
  particle's.
- **Speed clamp**: when the key's speed and acceleration have opposite signs, a particle
  whose speed crosses zero stays at zero (unless 0xCE).
- **Vertex emission** (types 6-9) shares the sequential cursor at `w2+0x62` between births;
  the random modes reset it but do not advance it.

### 15.4 EiDrawShipoly (ribbon)

- Ring buffer of `clamp(@11, 4, 128)` points (two per tick in mode @9 = 0). Mode ≠ 0 records
  the instance's `M` translation; space @17 = 0 keeps the points in effect space (they follow
  the root when drawn through `VR`), 1 transforms them by `Root` when recorded (world space,
  drawn through `V`). With @28 set, every point also carries an outward speed (the current
  value of an expand accumulator that key @0x2C..0x34 drives) and moves away from the
  centroid of the history by it each tick, except the newest and the two oldest.
- Drawing (`0x1404bac50`, `0x1404b9d70`): the history, newest first, is fitted with a natural
  cubic spline (uniform parameter) and sampled `@18 + 1` times; in mode 1 each sample gets a
  perpendicular in the screen plane, `width(@12) × instance scale x × rs.x` long, from the
  3D direction to the next sample. The strip is `@19 + 1` columns wide, interpolated between
  the two edges. Optional taper (@16): the newest end narrows to the middle line, linearly over
  the length.
- Texture: the first primitive group's `TEX0` of model @4. u runs along the ribbon from 0.01
  to 0.99 (key @0x9C..0xA1 choose sub-regions, direction and transposition), v across from
  0.01 to 0.99; @0x94/0x98 scroll. Blend @21: 0 alpha (0x44), 1 additive (0x48), 2
  subtractive (0x42), 3 (0x41), 4 darken (0x46). @20 turns the depth test off.
- Colour: the instance colour; alpha optionally ramps up over the first `@24 · subdivisions`
  segments and down over the rest (@23), and across the strip from 0 at the edges to full in
  the middle (@22). The PC scales these vertex colours by 1/225 where its other paths use
  1/255 (about 13% brighter than neutral).

### 15.5 Drawing on PC

- **Particle vertex colour** (`0x1404b0080`, the batched sprite path): `clamp(sprite_rgba ×
  (instance_rgba / 128) / 255, 0, 1)` per channel, i.e. the PS2 modulate clamped at 0xFF.
  `kh2-fx` outputs `min(sprite × instance / 128, 255) / 128` (1.0 = 0x80) and leaves the
  texture multiply to the renderer.
- **Models**: OMD flag bit 0 clear sets the draw flag that selects the depth-writing ZBUF
  register; bit 1 clear sets flag 0x10, which for shapes turns the depth test off (assumed the
  same for models, **inferred**). Sorted draws go to an ordering table bucketed by projected
  depth; `kh2-fx` places them after the immediate draws, farthest first (**inferred**).
- The ZBUF value used without the write flag has the GS mask bit set: effects never write
  depth unless asked.

### 15.6 Textures

- Each DPD texture is uploaded into the buffer that starts at its `tbp`, with its own buffer
  width, at its `(x, y)` (texture header +0x08). texcommon packs small textures this way: the
  page at `0x2C40` holds a 32×32 at (0, 0), a 32×32 at (0, 32), a 32×64 at (32, 0) and a
  64×64 at (64, 0); the page at `0x2C60` two 64×64 at x 0 and 64. A shape or model `TEX0`
  covers the whole page and its UVs pick the part they mean (e.g. Magnet's model 31 samples
  the right half of `0x2C60`). The `TEX0`'s palette (`CBP`) applies to every pixel it reads,
  as on the GS; where uploads overlap, `kh2-fx` prefers the texture that palette belongs to.

### 15.7 Spaces

- KH2 model (MDLX) space is y-up; effects and their bone codes are in the y-down world. The
  `kh2-fx` example maps Sora's posed bones with a y flip; a host must hand `kh2-fx` bone
  matrices in the y-down world.

## 16. KH2's element layer inside KH1's effect system (Steam KH1)

The effects port's step 2 (`kh1-hook/src/win/kh2dpx.rs`): what KH2's PAX element does, done
with what KH1's own effect instance and element already have. All addresses are the Steam KH1
build's. Tags: **Checked (exe)** = read in the Steam Ghidra export; **Checked (files)** = read in
the user's extraction; **Assumption** = not yet checked.

### 16.1 KH1's element and instance (Checked (exe))

- Element (0x3C bytes, 256 at `0x142D678B0`, taken by `0x1402AE5B0`, filled by the effect start
  `0x1401E0100`): handles to the instance's position `+0x00`, angles `+0x04` (65536 = 360°),
  scale `+0x08`, colour `+0x0C`, "ended" byte `+0x10`; `+0x14` flags (1 running, 2 ending, 0x10
  waiting to start, 0x20 has a bone binding), `+0x2C` a handle to the instance, `+0x30` a serial
  (a global counter; elements are reused), `+0x38` the DPX.
- Instance (0xE0 bytes, pool `0x142678280`): `+0x00` position, `+0x10` angles, `+0x20` scale
  (all 1.0 at start), `+0x30` colour RGBA floats (1.0), `+0x91` ended byte, `+0x92` state (1 live,
  2 dead, 4 ending its loop), `+0xA0` particle data, `+0xB0` its element, `+0xBC` the 30 Hz
  accumulator, `+0xD0` **start wait**, `+0xD5` paused.

### 16.2 The features

| KH2 element feature | KH1's own | Done |
|---|---|---|
| Start wait (`+0x14`, game frames; +0.1 when bound) | **Yes: instance `+0xD0`.** KH1's effect update (`0x1401ECB50`) neither steps nor draws an instance while its wait is above a threshold, and its draw pass (`0x1401ECF30`) counts it down by the frame step (`0x14233FBDC`, 1.0 a 60 Hz frame) unless paused. KH1 seeds it at start from the particle header's `+0x108` (`0x14031D2C0`); KH2's instance start (`0x1404B1600`) seeds nothing: KH2's only wait is the element's. | Set to the element's wait (+0.1 bound) right after the start. |
| KH1's extra wait for bound effects | KH1 only: an element with a bone binding (flag 0x20) starts at wait ≤ −2 × `0x14233FBCC` (−2 game frames; the frame unit, 1.0, set by `0x14018C2A0`), so the binding has placed it first. KH2 has no such rule. While the element waits (its flag 0x10, set by the update pass while the wait is above the threshold and more than one frame step) the binding task (`0x1402AE910`) doesn't move it, so it keeps the place the start gave it (`0x1402AE7F0` / `0x1402ADD60` place it at once, `0x1402ADB40`). | Taken off (the tester: exact to KH2): the wait written is KH2's (+0.1 bound) − 2 × the frame unit (read from KH1), only for an element KH1 binds. Each frame both games test the wait before counting it down (KH1: update pass tests, draw pass counts down by `0x14233FBDC`; KH2: the effect update), so the first tick is on KH2's frame at any frame step. **Assumption**: our start (from the KH2 controller, inside KH1's actor task, priority 80000) comes before KH1's effect update (task priority 140000) in the same frame, as KH2's motion trigger comes before its effect update. KH1's binding task list (`0x142DB34B0`, run by `0x1402AEDA0` at task priority 139999, just before the effect update) moves the element every frame its flag 0x10 is clear; with the shorter wait the flag is clear from the start frame on (it is set only while the wait is more than a frame step above the threshold), so the first tick uses that frame's bone, as KH2's bound element re-reads its bone every frame. |
| Offset and rotation (`+0x1C`, `+0x28`) | No field in the binding (mode 2 takes the bone's matrix as it is). But KH1 builds the root (`0x14031CEA0`) as instance position·angles·scale times the particle header's local transform (`+0x80` offset, `+0x90` angles XYZ, `+0xA0` scale), as KH2 does. | Folded into a copy of the particle data per element (its own effect number `0x700 | element`): offset = element offset ÷ element scale + element rotation × header offset; angles = element rotation × header rotation. Exact for a uniform element scale. |
| Scale (`+0x34`) | Instance `+0x20` (the root's scale, as KH2's). | Written after the start when not 1. |
| Fade-out (flag 0x40, `+0x05` frames) | Instance colour `+0x30..+0x3C`: KH1's draw programs multiply the vertex colour by it (e.g. `0x14032B940`, `0x14033F250`). KH1 fades its own effects this way (`0x1403970D0`: alpha down each frame, then the ended byte). | Alpha from its value to 0 linearly over the fade-out frames (KH2's rate), then KH1's stop `0x1401EC470 (element)`. Stepped from Sora's frames (`kh2fx::tick`). |
| End the loop (flag 0x80) | Instance `+0x91` (KH1's ended byte: no more loops, it ends when its pieces do, `0x14031CE10`). | Set on end. Not used by Explosion. |
| Ends with the motion (0x20) | KH1's binding watches the actor's motion request (`+0x164`, binding `+6`; number bit 0x800) and stops the effect at once when it changes (`0x1402AE910`). That is KH1's request, not KH2's motion, and a stop, not KH2's fade. | KH2's motion change (from `kh2fx::tick`) ends it the KH2 way (above). KH1's bit 0x800 is not used. |
| Ends with the owner (0x200) | KH1's binding stops the effect when the actor is gone (`0x140291FF0`). | A form change (`kh2fx::owner_gone`) fades or stops it (as `kh2-fx`). |
| Object root 0x4000 | **Yes: bone code `0x8000`** (`0x14029EA00` / `0x14029EB20`): the actor's position `+0x10` with a frame made from its matrix rows `+0xF0`, `+0xE0`, `+0xD0` (each negated, in that order). KH2's 0x4000 (`0x1403B50F0`): the object's rotation `+0x40` and position. | Element 48 (§17.5). The two frames are a quarter turn apart about y: KH2's offsets and turns are turned into KH1's (§18.17). |
| Weapon point 0x4003 / 0x4004 | **Yes: bone `0x30`, distance `t`** (`0x14029EB20`): `t` 0..1 mapped into the weapon's blade range (`btltbl` record `+0x28` start, `+0x24` end), the point between the blade bone and its parent (`0x1401D6320`). KH1's binding mode 3 (`0x1402ADD60 (dpx, number, actor, bone, t)`) takes the bone's rotation and that point every frame. KH2's 0x4003 (`0x1403B50F0`): the weapon object's weapon-shape bone (shape kind 0xF) matrix, moved along its x by a length; 0x4005 its root, 0x4007 halfway. | Element 44 (§17.4): 0x4003 → bone 0x30, t = 1.0; 0x4005 → t = 0; 0x4007 → t = 0.5. |
| Bind position only (0x04) | Binding mode 1 (`0x1402ADC20`): position only, plus a world offset. | Not used by Explosion. |
| Bind until started (0x02) | **Yes: bone-code bit 0x10000**: KH1's binding task (`0x1402AE910`) then places the element on its first run once the element no longer waits (its flag 0x10 clear) and ends. | Element 48 (§17.5). |
| Alpha follows the weapon (0x800) | No. | Done (§18.18): KH1 Sora's draw bit, alpha and weapon-out flag. |

### 16.3 What steps 3 and 4 need (Checked (exe))

- KH1's relocation (`0x14031C2F0`) turns each program record's type into a handle of its
  0x50-byte entry in the program table (`0x14054FF00`); KH1 calls the entry's functions through
  that handle (construct `+0x20` at start, `0x14031D2C0`). The table can't grow in place (entry
  241 would run into the security cookie at `0x140554A88`). So a KH2-only program (241
  pppEiDrawShipoly) or KH2's version of one (201 pppRyjMegaBirth, its 0x30-byte particles and
  third matrix mode) goes in as our own 0x50-byte entry: after KH1 relocates the DPX, those
  records' handles are pointed at it (a handle made by `0x14038AD90`). No hook; KH1's dispatch
  calls it with KH1's instance (its layout, `+0xD5` for KH2's `+0xC1`), and it draws through
  KH1's shape draw.

## 17. KH2's own programs in KH1's program machine (Steam KH1, steps 3 and 4)

`kh1-hook/src/win/kh2ppp.rs`. Tags as in §16; KH2 addresses are the Steam KH2 decomp's.

### 17.1 How they go in (Checked (exe))

- KH1's relocation (`0x14031C2F0`, called per DPD by `0x1401ECA40` with the table `0x14054FF00`)
  turns each program record's type (and each word of the particle data's type list, `+0x118`)
  into a handle (`0x14038AD90`) of `table + type × 0x50`, and each record's keys / work-offsets
  words into offsets from the record itself. KH1's machine (`0x14031CBE0` calc pass,
  `0x14031C5F0` draw pass, `0x14031D2C0` instance start) calls an entry's slots through that
  handle: calc `+0x08` and draw `+0x18` with (instance, current key, record), construct `+0x38`
  and destruct `+0x48` with (instance, record). Nothing else looks at which entry a record has,
  but for the fast-forward list (`0x14054FEE0`, calc slots skipped while an instance loop
  fast-forwards with `0x142E9D804` set): KH1's MegaBirth calc is on it.
- `kh1dpx::build_with` lists every word whose KH2 type is 201 or 241 (`Kh1Dpx::programs`,
  `KH2_OWN_PROGRAMS`); after KH1 relocates the DPX, `kh2ppp::plug` points each 201 word that KH1
  pointed at its own entry 201 at our entry (checked: KH1's entry 201 calc is `0x1403212F0`
  first). Our entry is a 0x50-byte static in KH1's layout. Steam only.

### 17.2 pppRyjMegaBirth, KH2's

KH2: calc `0x1404BC3B0`, step `0x1404BBBE0`, birth `0x1404BAC40`, update `0x1404BB8B0`, draw
`0x1404BBDB0`, particle matrix `0x1404BC710`, construct `0x1404BC5F0`, destruct `0x1404BC670`.
KH1's: `0x1403212F0`, `0x140320C30`, `0x14031FD00`, `0x140320900`, `0x140320DF0` (matrix inline),
`0x140321520`, `0x1403215A0`. **Checked (decomp)**, compared function by function:

- Same in both: the state at work offset 2 (base matrix `+0x00`, drift direction `+0x40`,
  handles of the particle / birth-matrix / colour-velocity arrays `+0x50` / `+0x54` / `+0x58`,
  slots `+0x5C`, counter `+0x60`, vertex cursor `+0x62`), the arrays' sizes (0x60, 0x40, 0x20 a
  slot, from the effect's allocator `0x1401EC330`), the base matrix refresh, the step (counter,
  births per burst, the shape animation), construct and destruct (so ours uses KH1's), and the
  draw packet given to the shape draw (KH2's PC batch `0x1404AE550`, KH1's `0x14031E210`).
- KH2's changes: the particle record (0x60 bytes) laid out anew: position `+0x00`, direction
  `+0x10` (w 0; KH1 leaves 1), animation time / step / step drawn / life `+0x20..+0x26`, colour
  offsets `+0x28`, age `+0x2C`, spin angle, velocity, random part as 16-bit angles `+0x2E` /
  `+0x30` / `+0x32` (KH1: float degrees, wrapped to ±180), scale, scale velocity, random parts
  `+0x34..+0x48`, speed `+0x4C`, drift `+0x50`, alpha taken off `+0x54`, size `+0x58`, fade-in,
  fade-out, grow-in, shrink-out lengths `+0x5C..+0x5F`. New key fields: `0xD2` random mirroring,
  `0xD3` / `0xD4` grow-in / shrink-out (the size factor), `0xD8` the drift speed's random range;
  matrix mode 3 (birth matrix = base ∘ instance). The draw builds the particle matrix as
  diag(rs.x·sx·size, rs.y·sy·size, rs.x·sx·size) turned by the spin through the sine table
  (KH1's `0x14054BED0` has the same values), clears the projected matrix's z row, and (in two
  KH2 rooms only, `0x140ABB920` = 1 or 6) moves the Z-corrected depth differently: KH1's rooms are
  none of those, so the ordinary path is taken.
- Explosion's MegaBirths (from the file): effect 31 emitter 5 (shape 13, 12 slots, matrix mode
  1, shrink-out 4), effect 32 emitter 1 (shape 16, 32 slots, mode 1, grow-in 3, shrink-out 4,
  drift acceleration −0.1), effect 34 emitter 2 (shape 13, 8 slots, mode 2). Random scale flags
  0x26 in all. **Checked (files)**.
- KH1's math functions are KH2's same library (same sizes and code: product `0x1400E65E0` /
  `0x14013F850`, point `0x140386A40` / `0x14013F900`, normalise (3D, w set 0) `0x140386C40` /
  `0x140142080`, angles `0x1400E6730` / `0x140140560`, random `0x14031BA30` / `0x1404DC710`), so ours
  calls KH1's. The update's drift step scales the drift direction by the particle's drift
  (checked in KH1's exe, `0x140320B68`; the KH2 decomp drops that operand).
- **Assumptions** (operands the KH2 decomp drops, no KH2 exe here): the particle matrix's
  spin product is (scale ∘ spin), i.e. turned first, then scaled (as §15.3 read it), and its
  `+0x1B0` vector is the root's axis lengths (KH1's draw reads them there). KH2's table entry 201
  shows only calc and destruct in the decomp's data references; draw and construct are taken as
  KH1's entry has them (`+0x18`, `+0x38`).
- Not used yet: no native element runs effects 31, 32 or 34 until their elements are switched
  on (elements 44, 45, 48; they still play through `kh2fx`).

### 17.3 pppEiDrawShipoly (241), KH2 only

KH2: calc `0x1404B9C00`, draw `0x1404B9260`, strip `0x1404B8380`, spline `0x1404B8F90` (set-up) /
`0x1404BAA90` (value), construct `0x1404BA790`, reset `0x1404BA8C0`, destruct `0x1404BAA40`; the
scroll-draw variant `0x1404B7530` (key `+8`) is not ported (logged once, not drawn; no Explosion or
Magnet ribbon uses it). **Checked (decomp)**; ours is `kh2ppp::sp_*`.

- State (work offset 3, 0xE0 bytes): head `+0x00`, ring length `+0x04` (u8, the key's `+11`
  clamped 4..128), count `+0x06`, ring handle `+0x08` (from the effect allocator, 32 or 16 bytes a
  slot), expand / velocity / acceleration `+0x0C..+0x14`, the variant's scroll `+0x18..+0x20`,
  scroll u / v `+0x24` / `+0x28` (random start by key `+0x9F`), the instance matrix copy
  `+0x30`, drift, its velocity and acceleration `+0x70` / `+0x80` / `+0x90`, the attract point,
  its velocity and acceleration `+0xA0` / `+0xB0` / `+0xC0`, strength and its rates
  `+0xD0..+0xD8`, the first key's handle `+0xDC` (the reset takes each accumulator's first key
  value back off). Construct also sets the effect's "not culled" byte (KH2 `+0xC3`, KH1 `+0xD7`,
  which KH1's update reads the same way).
- Calc (once a tick; skipped while fast-forwarding, as KH2's list `0x140779990` skips it):
  records two points ±width/2 along the instance's y (key `+9` = 0) or its place (else, with the
  expand value as `w` when key `+0x1C`), in effect space (`+0x11` 0) or through the root (1);
  then, in the one-point mode, expand (each point but the newest and the two oldest away from the
  history's centre at its recorded speed), drift (`+0x1D`), attract (`+0x1E`: a step of
  `(1 − clamp(distance / key +0x8C)) × strength` toward the point, in the instance's frame unless
  2), each with key deltas at its key frame.
- Draw: the history, newest first, through the space's view matrix (KH1's `0x142E9D580` view ∘
  root, or the view), a natural cubic spline (KH2's own elimination, diagonal 4; ours checked to
  4e-6 against `kh2-fx`'s form and through every point), `sub + 1` samples; in the one-point
  modes a perpendicular across the screen, half the width long; optional taper. Then the strip
  through KH1's renderer, the same library KH2's PC port uses (matched by code in both decomps):
  GIF/PRIM `0x140386330` (KH2 `0x140472DA0`), TEST `0x140386630` (`0x1404730A0`), ZBUF
  `0x140386960` (`0x1404733D0`, base `0x1425381D4` / `0x140B60258`), TEX0 `0x140386660`
  (`0x1404730D0`, the model's first group TEX0, which KH1's draw pass has moved to the effect's
  texture slot, `0x1401EBC60`), ALPHA `0x140386060` (`0x140472AD0`), CLAMP `0x140386210`
  (`0x140472C80`), shader `0x140108700` (`0x140127030`, 0x2B, or 0x2C in monochrome
  `0x142538200` / `0x140B60240`, the batched-shape shader), constant 4 `0x1401075D0`
  (`0x140125F00`), camera `0x14031EAA0` (`0x1404AEB30`; KH1's effect `+0xDC` = KH2's `+0xC8`),
  the inverse view `0x1400E5570` (`0x14013F3A0`), vertices `0x1400F3040` (`0x140111970`, 0x3C
  bytes: place twice, colour, uv × 4096, the first two flagged 0x1000) and their draw
  `0x1400FB9C0` (`0x14011A2F0`, primitive 6, 4 after the first segment). Per segment
  `2 × (key +0x13 + 1)` vertices across, u along (`+0x9C..+0xA1`, `+0x90` by length, scrolled
  `+0x94` / `+0x98`), v across, alpha along (`+0x17`, `+0x18`) and across (`+0x16`).
- **Assumptions** (operands the KH2 decomp drops; no KH2 exe here): the half width is key `+12`
  × the instance scale x (work offset 1) × the root's x length, and the vertex colour scale 1/225
  (both as `kh2-fx` read them, §15.4); a column's place is `A·(1 − t) + B·t` with `t` its
  fraction across (the decomp's operand order; `kh2-fx` runs it from B); the taper's middle is
  half the sum. Where the decomp reads differently from §15.4 the decomp is followed: the taper
  narrows the old end (t from 1 at the newest down), the attract step is the normalised
  direction (not the distance) times the factor, key `+0x9E` = 1 starts u at 0.5, v runs
  0.01..0.99.
- Not ported: spaces 2 (an object KH2 sets, `0x140B60270`) and 3 (not drawn), the scroll-draw
  variant.

### 17.4 Explosion's elements 44 and 45 through KH1

- Element 44 (effect 31, KH2 bone 0x4003, flags 0xA61, no offset or turn): KH1's start on a
  point along the weapon (`0x1402ADD60 (dpx, number, actor, 0x30, t)`, Steam; checked in the
  exe: `t` is the fifth argument, a float on the stack, kept at the binding's `+0xC`), binding
  mode 3: every frame the blade bone's turn (`0x14029EA00`, code 0x30 through KH1's named-bone
  table `0x14029FAB0`) with the point `t` along the blade as its place (`0x14029EB20`: code 0x30
  maps `t` into the weapon's blade range, its `btltbl` record `+0x28` start, `+0x24` end, then
  the point between the blade bone and its parent, `0x1401D6320`). 0x4003 (KH2's tip) → t = 1;
  0x4005 (root) → 0; 0x4007 (halfway) → 0.5. **Assumption**: KH1's blade end is where KH2's
  0x4003 tip is on the Keyblade (each game's own tip), and the blade bone's frame stands for
  KH2's weapon-shape bone's (44 is sprites facing the camera and a full-sphere spark burst, so
  the frame's turn hardly shows).
- Element 45 (effect 32, KH2 bone 67, offset 18.75 along x, turned −90° x / +90° y): KH1 bone
  70. **Checked (files)**: in the bind poses KH2 67 (a spine bone, 0.30 of the way from the
  pelvis 102.6 to the chest 68 at 126.5) has the same frame as KH1 70 (0.24 of the way from 92.7
  to 114.2; x up the spine (0, 0.97, −0.23), y (0, 0.23, 0.97), z (1, 0, 0) in both); KH1 71 has
  the same frame at 0.47. The same check holds for the wrist flashes: KH2 170 / 78 and KH1 93 /
  198 have the same bind frames to 0.05, so their turn needs no conversion.
- Both run KH2's MegaBirth: they play through KH1 only when it is plugged in (`kh2ppp::plugged`,
  Steam); otherwise, and on other builds, `kh2fx` plays them as before.
- Flag 0x800 (alpha follows the weapon's visibility): §18.18.
- KH1's pppColor (43) and KH2's differ: §17.6 (KH2's now runs for our DPX's words).
- KH1's bone binding and KH2's place an effect from different frames' bones on the first frame
  only: §17.7.

### 17.5 Explosion's element 48 (the burst) through KH1

- Effect 34, KH2 bone 0x4000 (object root), flags 0x243 (bound until started, fade-out 8
  frames, ends with the owner), no offset or turn. KH1's start on a bone with KH1's object-root
  code 0x8000 (binding mode 2: `0x14029EA00` takes the actor's place `+0x10` with the frame
  (−row `+0xF0`, −row `+0xE0`, −row `+0xD0`); `0x14029EB20` with code 0x8000 gives the place) and
  bit 0x10000 for KH2's 0x02 (KH1's binding places it once, on its first run with the wait over,
  then ends: with the start frame's wait off (§16.2) that is the start frame itself, the binding
  task running at priority 139999 before the effect update). Plays through KH1 only with both
  KH2 programs plugged in (MegaBirth, the ribbon). **Assumptions** (the frame: corrected in §18.17): KH1's 0x8000 frame is KH2's
  0x4000 object frame; KH2's 0x02 element is placed for the last time on the frame before its
  first tick, as KH1's is here.

### 17.6 pppColor (43), KH2's, for KH2's effects only

- KH1's (`0x14032B940`, entry 43: calc `+0x08`, construct and reset `0x14032BAE0`, no draw) and
  KH2's (`0x1404C0F60`) **Checked (decomp)**, KH1's also **Checked (exe)**. Work offset 0: four
  u16 channels, then the four colour bytes the draws read (`+8`). Both add the key's deltas
  (`+4..+0xA`) only when not held (`0x142E9D7E0`), not paused (effect `+0xD5`, KH2 `+0xC1`), bit
  0 of the instance's frame word `+0xC` clear (the not-repeat test) and that word equal to the
  key's frame. KH1 then writes the bytes inside
  that same test (except the key-frame part); KH2 writes them on every call, each channel × the
  effect's colour (`+0x30`), **clamped at 32767**, × the scale. Monochrome (`0x142538200`, KH2
  `0x140B60240`): RGB = 0.3 r + 0.59 g + 0.11 b of the bytes. So under KH1's an element's fade
  (the instance alpha) shows only on calls that pass the not-repeat test, under KH2's on every call.
- Scale: KH1's 1/128 (`0x1403B30B8`, **Checked (exe)**). The KH2 decomp drops the operand of its
  scale call (`0x14013FCC0`): **Assumption** 1/128 (KH1's, and `kh2-fx`'s, `0x80` = 1.0).
  KH2's construct isn't in the decomp's references: **Assumption** it is KH1's (the channels
  zeroed, as `kh2-fx` does), so ours uses KH1's `0x14032BAE0`.
- `kh2ppp::col_calc` is KH2's calc on KH1's instance, as our third entry. `kh1dpx` lists the
  type-43 words of our DPX (`KH2_OWN_PROGRAMS` 43, 201, 241) and `kh2ppp::plug` points each one
  KH1 relocated to its entry 43 (checked: its calc is `0x14032B940`) at ours. KH1's own DPXs
  are relocated by KH1 to its own entry and never touched, and nothing in KH1 looks at which
  entry a record has (no other reference to `0x14032B940` or the entry in the decomp), so KH1's
  own effects keep KH1's pppColor exactly. Not a gate: if it isn't plugged (table not as
  expected, or not Steam) the element still plays through KH1, with KH1's pppColor (logged).

### 17.7 Which frame's bone a bound effect sits on

- KH2 (Steam, **Checked (decomp)**): the element task `0x1402C4280` (main task list, priority
  20300, registered by `0x1402C4330`) runs `0x1402C58F0` per bound element (`+0x70` bit 0): while
  its timer `+0x58` is 0 (its first run) it reads the bone (`0x1402C5F70` → `0x1403B50F0`, the
  bone's matrix into the element record `+0x00`) and then applies it to the effect
  (`0x1402C49D0`: position, angles, scale); after that it applies the matrix it read on its run
  before, then reads the next, every frame, also while the effect waits (it stops only for flag
  0x02 once the effect has started, `0x1402BE880`: the instance's wait `+0xBC` > 0). The timer
  goes up by the frame step at the end of each run. The start (`0x1402C6B30`) only reads. Order
  in a field frame: the object update (`0x1403BF550`, 20000: the motion advances and fires its
  triggers (`0x1403C6740`), then the pose, `0x1403C73C0`), the element task (20300), the effect
  update (`0x1402BD830`, 60001, `0x140162C50`). **Assumption**: `0x1403C73C0` is where the
  object's bone matrices for the frame are made (its writes not traced to the end).
- So in KH2 the first tick sits on the start frame's pose, every later one on the pose of the
  frame before (the matrix read one run earlier).
- KH1 (Steam, **Checked (exe)**): the start on a bone (`0x1402AE7F0`, mode 3 `0x1402ADD60`) reads
  the bone and places the element at once, keeping the matrix in its binding record (`+0x30`,
  place `+0x60`; record from `0x1402AE880`, a node in the list `0x142DB34B0`: `+0x10` callback,
  `+0x18` actor, `+0x20` element, `+0x28` serial, `+0x30` record handle; the list's `+0x18` is
  the node added last, `0x14028A900`). The binding task (`0x1402AE910`, run by `0x1402AEDA0` at
  139999) applies `+0x30` (`0x1402ADB40`), then reads the next (`0x14029EA00`, mode 3 also
  `0x14029EB20 (actor, code, t in xmm2, out)`), and does nothing while the element waits (flag
  0x10). The bone matrices it reads are the model's `+0x1B4` array, the one KH1's skeleton pass
  (`0x140391C70`) and our `pose` write. Inside KH1's actor task (80000) the actor list
  `0x142D5AB00` runs each actor's update (sub-priority -2000..3000, or 6000 / 7000, by kind, set around
  `0x140297BCF`; our KH2 controller and its element starts) before the skeleton passes of the
  actors queued (`0x140298730` at -1000, 5000, 8000); the starts of our slots' KH2 events
  (`kh2slots::frame`) come inside Sora's pass (after KH1's own bone pass, before our pose is
  written), so the re-read after the pass (below) covers them too. So KH1's start reads the pose
  of the frame before, and KH1's first tick sits on that; later ticks sit on the frame before,
  as KH2's.
- Difference (KH2's elements only): the first tick (KH1 a frame behind) and, for an element that
  waits more than a frame, its first tick (KH1: the bone at the start; KH2: the frame before).
  Explosion's elements all wait 0 (+0.1 bound; **Checked (files)**).
- Done (`kh2dpx::posed`, Steam): each continuous binding of ours (modes 2 and 3; not 0x10000,
  which already reads first, then applies and lets go, as KH2's 0x02 element) is noted at its
  start (the list's last node, checked: callback, element). Sora's skeleton pass (`pose` hooks,
  his own instance) calls `posed`: after the pass, a binding started before it gets its record
  re-read with KH1's own calls (this frame's pose: KH2's first run); before later passes, the
  record is re-read (the frame before's pose: what KH2 applies) while the element waits, and let
  go once it doesn't. After that nothing of ours touches it: KH1's apply-then-read is KH2's.
  KH1's own effects and bindings are never touched. Per frame: an atomic check while nothing is
  noted. **Assumption**: one skeleton pass of Sora a frame (with two, a waiting element's
  re-read before the second takes this frame's pose).

### 17.8 The old player retired for Explosion

- On Steam, once our DPX is built and KH2's MegaBirth and ribbon are plugged in, `kh2dpx::event`
  owns elements 44..48: KH1 plays them, and if KH1 can't start one (no free element or effect
  slot, no Sora actor) it doesn't play, as with KH1's own effects; `kh2fx` is never asked. The
  ini switch `kh2_fx_native` is gone.
- `kh2fx` still plays Explosion only where KH1 can't: the DPX not built yet (its background
  build, the first seconds) or not at all (KH2 install or files not read: logged), KH2's
  programs not plugged in (KH1's program table not as expected: logged), or another build
  (Epic: the Keyblade-point binding and our programs are Steam-only, so elements 44, 45 and 48
  go to `kh2fx`; 46 and 47 play through KH1 and fall back to `kh2fx` if KH1 has no slot).
- Everything else in `kh2fx` is unchanged and still runs through it (its world-pass hook
  `0x1401EADD0` and D3D12 draw `fx12`): Sora's other move effects (`kh2_effects`: Vicinity Break
  170, Magnet Burst 197, Aerial Finish 194, Guard Break 165, Finishing Leap 167, Aerial Recovery
  40, or every move with `all`), the form revert flashes (`form`, elements 15..27), Valor's and
  Final's own packages (`form_event`), Reflect's packages (`reflect`), Wisdom's shots and spells
  and base Sora's KH2 magic (`formmagic`), the Keyblade's vanish flash (`weapon_vanish`, the
  `W_EX010` package), `fx_test`, and the endings and fades it steps for those (`tick`,
  `owner_gone`). `kh2fx::tick` / `owner_gone` also step `kh2dpx`'s endings and fades; `paused`
  is used by `model`.

## 18. Every KH2 effect through KH1 (Steam KH1)

The port of every effect the mod plays onto KH1's own effect system, the way Explosion went
(§16, §17). Tags as in §16.

### 18.1 The builder for any effects (`kh2_formats::kh1dpx::build_groups`)

- A KH1 DPX holds any number of DPDs (its DPD list after the entries): KH1's DPX relocation
  (`0x1401E0080`) turns every entry's DPD offset into a handle and relocates each DPD of the list
  (`0x1401ECA40`, its particle datas through `0x14031C2F0`). **Checked (exe)**.
- KH1 gives an instance its texture slots when it draws (`0x1401ECF30` → `0x1401EDB00`): 10 TBP
  and 6 CBP slots, a run for each DPD (keyed by the instance's DPD `+0x98`), shared by the
  instances of that DPD. **Checked (exe)**. So the builder makes one DPD per group of effects,
  each with only its particle datas and the textures they draw (the source DPD's own and KH2's
  common ones), packed from KH1's own window (TBP `0x3800`, CBP `0x3D40`), as for Explosion. A
  group's effects come from one DPD of the PAX; different groups may come from different DPDs.
  A group needing more slots than KH1 has (or missing a texture) is left out and reported
  (`Kh1Dpx::skipped`); the rest are built.
- Each entry is a copy of a KH2 effect's particle data under its own number (`Variant`: the
  element's offset and turn folded into the header's local transform, §16.2), its program types
  listed (`Kh1Dpx::types`), KH2's own program words listed (`Kh1Dpx::programs`), and each
  texture's source DPD noted (`TexOrigin::src_dpd_at`) for its HD image.
- Explosion's DPX from it is byte for byte the one t368..t404 built (checked offline with a
  stand-in for `texcommon.dpd`, which isn't staged here).

### 18.2 Which programs KH1 runs as KH2 does

Every slot of KH1's program table entry (`0x14054FF00`, read from the exe) against KH2's
(`0x140779A00`, each slot from the KH2 decomp's data references, else the KH2 function whose
decompile matches), as normalised decompiles (addresses, variable names and line wrapping taken
out; the effect's paused byte KH1 `+0xD5` = KH2 `+0xC1`). Where a function isn't in either
decomp, KH1's code was read in the exe. The programs of Sora's package (`P_EX100.a.us`, all its
elements; **Checked (files)**):

| Programs | Result |
|---|---|
| 87 MatrixXYZ, 90 MatrixYXZ, 91 MatrixZXY, 94 MatrixScl, 96 RyjMatrixNoRot, 105 ParMatrix, 106 KeParMatR, 113 DrawMatrixFront, 115 DrawMatrixWood (calc), 116 KeDMat, 117 KeDMatFr, 120 RyjDrawMatrixWorld (calc), 124 KeZCrct, 125 KeZCrctShp, 51 RandFloat, 63 RandFV, 66 RandIV, 174 VertexAp (calc), 177 VertexApLc (calc), 184 KeBornRnd6 (calc), 219 DrawMdlLoopZ (calc, draw, construct) | Same code. **Checked (decomp)**. |
| 137 DrawMdl3, 138 DrawMdlSemi3, 139 DrawMdlTs3, 142 DrawShapeX, 160 KeShpDtt (draws), 49 VtMime (draw, construct, destruct) | Same code (the KH2 function found by its decompile). **Checked (decomp)**. |
| 28 Accele, 36 Move | Not in either decomp. KH1's are byte for byte its SclAccele / SclMove (exe), which t368 found the same as KH2's. **Checked (exe)**, KH2's **Assumption** (as its siblings). |
| 217 DrawMatrixLoop | Not in either decomp. KH1's is byte for byte its DrawMatrix (112), the same globals (exe). KH2's **Assumption** (as §7.4 reads it). |
| 29-31, 37-42, 84, 112, 139 / 142 calcs and constructs | Not in either decomp; t368's comparison (Explosion's programs) found them the same. |
| 49 VtMime calc | **Differs**: KH1's (`0x14032C2E0`, exe) adds the key's rate delta (`@16`) always; KH2's leaves the rate when it is below −1e38 (§7.4, the keep marker; KH2's calc isn't in the Steam decomp). Sora's VtMime keys use the marker (−FLT_MAX on their last key, **Checked (files)**: Vicinity Break, Aerial Finish), so KH1's would throw the morph away there. Ported (§18.4). |
| 43 Color, 201 RyjMegaBirth, 241 EiDrawShipoly | KH2's own, ported (§17). |
| 238 SnoMdlDttNorm | KH2 only (a frame-buffer distortion along a model's normals). Only element 1 (the Drive motion's sphere) runs it. Ported (§18.14). |

`kh2dpx::KH1_SAME` lists the ones KH1 runs as KH2; an element whose effect runs anything else
(not KH1_SAME, not KH2's own plugged in) stays with `kh2fx`, logged once.

### 18.3 Sora's package through KH1 (`kh2dpx`)

- Every element of `P_EX100.a.us` is built at load, each as its own copy (`0x700 | element`, its
  offset and turn folded in, §16.2). The groups (one DPD each): the elements each of KH2 Sora's
  motions starts (its frame triggers 1, from the moveset the mod loads; an element in the first
  motion that starts it), Revert's thirteen (`form::REVERT_ELEMENTS`), then each remaining
  effect's elements; each split by source DPD.
- KH2 bones on KH1's (option A), each KH2 bone on the KH1 bone of the same body part at the
  same joint (retarget's parts), their bind frames the same to 0.05 (**Checked (files)**: KH2
  `P_EX100.mdlx`, KH1 `xa_ex_0010.mdls`), so no turn conversion: pelvis 2 → 2; +x leg thigh 11
  → 43, shin 12 → 44, foot 15 → 47, toe 18 → 50; −x leg 40 → 17, 41 → 18, 44 → 21, 47 → 24;
  spine 67 → 70; upper arms 78 → 198, 170 → 93; hands 85 → 203, 177 → 98; 0x4000 → KH1 0x8000,
  0x4003 / 0x4005 / 0x4007 → KH1's blade points. The effect sits at KH1's joint (KH1's limbs are
  shorter: the hips 78.9 high against KH2's 88.5).
- Flags KH1's binding does: 0x01 (mode 2 / 3), 0x02 (KH1's bind-once bit 0x10000), the endings
  (§16.2). Not yet: unbound elements (KH2 places them once at the start; Sora's 14, 42, 43, none
  of which a motion starts), 0x100 (scale by the bone), 0x10000 (two points); those stay with
  `kh2fx`. 0x04 (position only): §18.5.
- Switched on (`kh2dpx::ON`): Explosion, Magnet Burst (51: object root, y −127, fade-out 4;
  52: bound until started). Magnet Burst's programs are all in KH1_SAME or KH2's own.
- Cost: built in the background (more DPDs than Explosion's one: each motion's copies of the
  source DPD's shapes and models). Per frame nothing more than Explosion's. Since §18.12 only the
  motions `kh2_effects` lets play are built, and the HD images are held as §18.12 says.

### 18.4 pppVtMime (49), KH2's calc

- KH1's calc (`0x14032C2E0`, **Checked (exe)**: not while held `0x142E9D7E0`, paused (effect
  `+0xD5`) or on a computed frame (instance `+0xC` bit 0); rate += acceleration, t += rate; on the
  key's frame t += `@12`, rate += `@16`, acceleration += `@20`) adds the rate delta always. KH2's
  leaves the rate when `@16` is below −1e38 (§7.4; KH2's calc is not in the Steam decomp, so this
  is the earlier read of KH2's code, `kh2-fx`'s `vt_mime_step`: **Assumption** on the exact
  comparison). Sora's VtMime keys end with −FLT_MAX there (**Checked (files)**: Vicinity Break's
  elements 49, 50 at ticks 11 / 14, Aerial Finish's 35..38 at tick 13), so under KH1's the morph's
  `t` runs off to −∞ for the rest of the effect.
- `kh2ppp::vt_calc` is KH2's calc on KH1's instance, as our fourth entry; draw, construct, reset
  and destruct are KH1's (`0x14032C150`, `0x14032C380`, `0x14032C3C0`, `0x14032C3E0`: draw,
  construct and destruct the same code as KH2's, §18.2). `kh1dpx` lists type-49 words
  (`KH2_OWN_PROGRAMS`) and `kh2ppp::plug` points the ones KH1 relocated to its entry 49 (checked:
  calc and draw as above) at ours. KH1's own effects keep KH1's. Neither game's fast-forward list
  holds a VtMime calc (KH1 `0x14054FEE0`: MegaBirth, `0x140322E50`; KH2 `0x140779990`: MegaBirth,
  `0x1404BE1F0`, the ribbon; **Checked (exe)** / **Checked (decomp)**).
- A gate (`kh2dpx::GATES`): an element running VtMime plays through KH1 only with it plugged in.
- Switched on: Vicinity Break (motion 170: 49 object root y −8 and 50 y −97, both bound until
  started) and Aerial Finish (194: 35..38, object root, rotated swirls, fade-out 2 / 0).

### 18.5 KH2's position-only elements (flag 0x04) on KH1's binding mode 1

- KH2 (§2, **Checked**): the effect sits at `B · offset` (the element's offset in the bone's
  frame) turned by the element's rotation alone; with 0x01 every frame, with 0x02 until started.
- KH1 (**Checked (exe)**): the start bound by position, `0x1402ADC20 (dpx, number, actor, code,
  t)` (Steam; t the fifth argument, kept at the record's `+0xC`, mode `+4` = 1, code `+8`, world
  offset `+0x10` = (0, 0, 0, 1), its first place at `+0x20` copied to the element's place). The
  binding task (`0x1402AE910`) in mode 1 applies `+0x20`, then reads the point `0x14029EB20 (actor,
  code, t)` and adds `+0x10` into `+0x20` (`0x140386A10`, four floats); with the bind-once bit it
  reads, adds and applies once, then ends. It never writes the element's turn. The point for a
  raw bone is code `0x1000 | n` (`0x14029FAB0`: bit 0x1000 takes the low 12 bits; `0x2000 | n` would
  be looked up as a named bone there), at t = 1 the bone's place (`0x1401D6320`: its parent's
  matrix times its own offset); for the blade, code 0x30 maps t into the blade (as mode 3).
- KH1's world offset is fixed, KH2's offset turns with the bone. So for our mode-1 bindings
  (`kh2dpx::posed`, after Sora's skeleton pass, Steam): the record's offset is set to (the bone's
  place + the bone's turn × the element's offset − the point KH1 reads), from KH1's own reads
  (`0x14029EA00` with `0x2000 | n`, `0x14029EB20` with the record's code), so KH1's next read lands
  on KH2's `B · offset` exactly; for the blade, the point plus the blade's turn × the offset. Its
  place for the next apply is this pass's on the start pass (KH2's first run reads first) and the
  pass before's after it (what KH2 applies). One per live position-only element per frame: two
  KH1 bone reads.
- The element's own rotation goes into its copy's header turn (`H' = R · H`; the offset is not
  folded in), the instance unturned, as KH2's root `T(B · offset) · R · S · H`.
- Switched on with it: Guard Break (165: 30 bound until started, 31, 32), Finishing Leap (167:
  33 the Keyblade tip by position, 34 bound until started), Aerial Recovery (40: 0), and the
  moves only `kh2_effects = all` plays: 169 (53..55 by position on the toes, KH2 18 / 47, bound
  until started; 56), 171 (40, 41 by position on the spine), 172 (39). Their programs are all
  KH1_SAME or KH2's own (**Checked (files)**).
- KH1 instances start unturned: the pool's claim (`0x1401EC690`) clears the instance, and the
  start (`0x1401E0100`) only hands out handles of its place, turn and scale. **Checked (exe)**.
- 0x04 on KH2's object root (Fire 3's elements 2, 6, 7, 10 in the t430 log; none in Sora's
  package): KH2's place is the object's place plus its frame times the offset. KH1's point for code
  0x8000 (**Checked (exe)**, `0x14029EB20`'s last case, neither 0x8002 nor 0x8008) is the actor's
  place (`+0x10`) plus its frame rows (`+0xD0`) times (0, 0, 100 t): at t = 0 its place. So KH1's
  position binding on 0x8000 with t = 0, its offset set by `posed` through the object frame
  (`0x14029EA00` with 0x8000), is KH2's.

### 18.6 The Drive motion's and Revert's flashes through KH1

- The Drive motion (52, `form::DRIVE_MOTION`) starts elements 1..13 at its frame 0; Revert
  (`form::revert`) starts 15..27 at once, right after Sora's object is handed back (so their
  "ends with the owner" flag, 0x200, sees the new owner). All in the package's second DPD, which
  has no textures of its own: everything they draw is KH2's common textures (**Checked (files)**).
- Bones: 2..13 and 16..27 on the limbs and spine (§18.3: 67, 2, 170 / 78, 177 / 85, 40 / 11,
  41 / 12, 44 / 15), flags 0x241 (bound, fade-out on the owner going); 15 on the spine by position
  (0x245, §18.5). Their programs are all KH1_SAME, KH2's pppColor and MegaBirth.
- Element 1 (the Drive motion's sphere at the object root, bound until started) runs KH2's
  pppSnoMdlDttNorm (238, a frame-buffer distortion along the model's normals), which KH1 doesn't
  have: ported as a KH1 program (§18.14), so element 1 plays through KH1 too.

### 18.7 More packages; the Keyblade's vanish flash; unbound elements

- `kh2dpx` builds one DPX per package (`Source`: its file, its PAX, the sets that play
  together, its anchors, flags it doesn't apply), each entered in KH1's loaded files under its own
  name (`kh2fx_<package>_dpx`) with its own HD textures (the package file's remastered assets and
  texcommon's), relocated and plugged on first use (since §18.12: as soon as it is built, and the
  common textures shared). `kh2ppp::plug` now only ever turns a "plugged
  in" flag on, so a second DPX without, say, MegaBirth words doesn't turn it off.
- The Keyblade's package (`obj/W_EX010.a.us`, its first PAX): only its vanish flash is built
  (the only element that plays; the t430 log's DPX held all four of its effects, 616,048 bytes,
  and its element 10 was left out as a two-point element). Its vanish flash (element 0,
  `kh2fx::weapon_vanish`, started as KH2 puts the Keyblade away, before KH1's put-away). Its
  bones are the weapon skeleton's; KH2's own call of the flash isn't traced, and `kh2fx` put every
  code half way along the blade turned as the hand. Through KH1: the point half way along KH1's
  blade, turned as KH1's blade bone (binding mode 3, t = 0.5). **Assumption** (as `kh2fx`'s). Its
  "ends with the motion" flag isn't applied (as `kh2fx` never ended it: the motion is the
  Keyblade's own object's, which we don't have). Not checked offline: `W_EX010.a.us` isn't staged
  here, so its element's flags and programs are read at load (logged); anything not in KH1 keeps
  it with `kh2fx`.
- Unbound elements (neither 0x01 nor 0x02: KH2 places them once at the start, from the bone as
  it is then, `0x1402C6B30` only reads, §17.7): KH1's start on the bone places it the same way
  (at once, from the bone as the start reads it: the pose before Sora's skeleton pass, as KH2's
  start before its pose); its binding (the list's last node) is then disarmed: mode 0 with the
  bind-once bit, which KH1's binding task (`0x1402AE910`) answers on its first run by freeing the
  record and ending, without moving the element. **Checked (exe)**. Steam only (the binding list's
  address). KH2 bone 0 (the root, at the hips) is KH1's bone 0 (the same bind frame).
- Unbound and by position only (flag 0x04 without 0x01 / 0x02: KH2 places it once at its start at
  `B · offset`, turned by its own rotation alone, §2): KH1's start bound by position (mode 1,
  §18.5) places it at the bare point (its world offset is (0, 0, 0, 1) then); its place is then set
  to KH2's from KH1's own bone and point reads at that moment (`by_position_now`, as for the
  bound ones), and its binding disarmed as above. **Checked (exe)** for KH1's calls; until t430
  these stayed with `kh2fx` ("placed once at its start by position only").

### 18.8 Valor's and Final's packages

- With Drive on, `kh2dpx` also builds Valor Form's (`obj/P_EX100_BTLF.a.us`) and Final Form's
  (`obj/P_EX100_ULTF.a.us`) packages (PAX "p_ex", else the first), grouped by their own movesets'
  motions (the frame triggers 1 the mod's form motions fire, `form::effect` →
  `kh2fx::form_event`). Every element built is switched on; the gate (anchor, flags, programs)
  decides at play. The element's sound is still `kh2sound::spell_element`'s.
- Anchors: the forms have Sora's skeleton (`pose::Form`), so his bones' table (Â§18.3). Their
  weapon points (the tester, 1 (a)): 0x4003 / 0x4005 / 0x4007 (the first Keyblade) on KH1's blade
  binding of Sora (mode 3, code 0x30, t = 1 / 0 / 0.5), turned with the blade as KH2's weapon bone
  turns them (`0x1403B50F0` â `0x1403B5450`, the weapon's shape bone; Checked (decomp)); 0x4004 /
  0x4006 / 0x4008 (the second weapon's same points, `(code â 1) & 1` = 1 there) on the second
  Keyblade (Â§18.13).
- Not checked offline: neither file is staged here, so which bones, flags and programs their
  elements use is only known at load (logged: each element switched on, each left out and why).

### 18.9 The spells' programs

The programs §12.2 lists for the spell packages (Reflect, Fire, Blizzard, Thunder, Cure, Wisdom's),
compared as in §18.2:

| Programs | Result |
|---|---|
| 75 SRandFV, 76 SRandUpFV, 85 KeMatSN (calc), 155 KeShpTail2X (calc; draw found by its decompile), 213 PointLoop, 215 ScaleLoop, 218 DrawMdlLoop (calc, construct; draw found by its decompile) | Same code. **Checked (decomp)**. In `KH1_SAME`. |
| 210 MoveLoop, 211 AngMoveLoop | Not in either decomp. KH1's (`0x14032AF90`, construct `0x14032B080`) is §7.6's KH2 reading (keys always, the value not while fast-forwarding `0x142E9D804`, construct clearing the velocity). **Checked (exe)** against §7.6. In `KH1_SAME`. |
| 214 AngleLoop | **Differs** (calc; construct and reset the same): KH2's (`0x1404C0BA0`) brings each angle above 0x10000 down by whole turns after a key; KH1's (`0x14032B620`) doesn't. **Checked (decomp)**, KH1's also **Checked (exe)**. Ported: `kh2ppp::al_calc`. |
| 157 KeShpTail3X | **Differs** (draw; calc and construct the same): KH2's (`0x1404CEEF0`) steps its trail time (work 0 `+0x1F0` += key `+8`, at the draw's end, after drawing with the old time) only when not held, not paused and not on a computed frame; KH1's (`0x140338160`) on every draw with a model (key `+4` not 0xFFFF; every path of it reaches the step). **Checked (decomp)**. Ported: `kh2ppp::tail3x_draw` runs KH1's draw and takes the step back where KH2 makes none. |

Both are plugged like the others (`KH2_OWN_PROGRAMS`, each KH1 entry checked slot by slot first)
and are gates. Which spells use them (§12.2): AngleLoop the Loop integrators of Blizzard's shard and
Wisdom's Blizzard; KeShpTail3X Fire's flame tails and Blizzard 1's sparkle. Not checked offline:
the spell files aren't staged here.

### 18.10 The spell scripts' effects (Reflect, Wisdom, KH2 magic) through KH1

- `kh2dpx` builds each spell package `kh2fx` plays for the spell scripts (Steam only): Reflect's
  levels (`reflect::spell_path`: the MAGC row's file, else `magic/REFLECT_<n>.mag`), Wisdom's archive
  (`obj/P_EX100_MAGF.a.us`, PAX "p_ex"), and Fire, Blizzard, Thunder and Cure by level
  (`formmagic::spell_path`), one group per effect, their "ends with the motion" flag not applied
  (the scripts end them; `kh2fx` never ended them on Sora's motion either).
- `kh2fx`'s spell API tries KH1 first (`kh2dpx::spell_start`): an element that passes the gate
  (anchor, flags, programs) is started on Sora as his own are and gets a handle of ours
  (`NATIVE_SPELL` set, inside `kh2fx`'s package bits); the others stay `kh2fx`'s. On our handles:
  - place (`spell_place`, a script moving an unbound effect: Wisdom's shots, Reflect's contact
    flash): KH2's `at` stands for the bone from then on (`kh2-fx`'s `set_anchor`). KH1's binding
    is let go (disarmed, §18.7) on the first one, then KH1's own apply (`0x1402ADB40 (element,
    matrix)`: the turn from the matrix's rows through `0x1402A8EF0`, in 65536ths of a turn, the
    place from its last row; **Checked (exe)**) puts the element there; by position only, the
    place `at · offset`, unturned.
  - end: KH2's end (§2: 0x80 the loops, else 0x40 the fade-out, else it plays on), as `kh2dpx::end`;
    kill: KH1's stop (`0x1401EC470`); alpha: the instance's alpha (`+0x3C`); running: the instance
    still the element's.
- Not checked offline: none of these files is staged here, so their elements' bones, flags and
  programs are only known at load. What KH1 doesn't have stays with `kh2fx` (logged per element):
  KH2-only programs not ported, and the programs §12.2 lists that aren't compared yet
  (KeShpTail3X and AngleLoop now are, §18.9). KH2's body centre 0x4001: §18.15.
- The second weapon's points 0x4004 / 0x4006 / 0x4008 in a spell package (Fire 3's elements 9 and
  12, Blizzard 3's 7 in the t430 log): KH2 reads them from the owner's second weapon for every
  package (`0x1403B50F0` → `0x1403B5450 (object, 1)`: with the object's weapon flag (`+0x6C8`
  bit 2) the weapon object in slot 1, `+0xD68`, whose first kind-15 collision shape names its
  bone, `0x1403B5D40`; **Checked (decomp)**). So they are the second Keyblade's (§18.13), as in
  the forms' packages: Final Form's two Keyblades. With one Keyblade (base Sora, Wisdom) KH1 has
  none to bind them to and they don't play (**Assumption**: KH2's scripts start them only with two
  weapons; with slot 1 empty KH2's read would fail).

### 18.11 The old player removed; what doesn't play through KH1

The tester: "the older player is to be deprecated". Since then (the version after t430) every KH2
effect plays through KH1's own effect system or not at all; the old player (the `kh2-fx` particle
machine run by `kh2fx` beside KH1's, its own instances, bones and clock, and its D3D12 draw after
KH1's world-model pass, `fx12::draw`) is gone from the hook. `kh2fx` is now only the front the
game's events call (moves, forms, Revert, the Keyblade, the spell scripts, Sora's frames), each
handed to `kh2dpx`. The world-model pass hook (`0x1401EADD0`) moved to `model` and is installed
only while KH2's player model can be drawn (off since s259, so not hooked); `fx12` keeps the
model's and the debug menu overlay's draws. The ini's `fx_test` (which started the old player's
element 51) is gone.

Memory and work that went with it (the t430 log): the old player loaded Sora's package and every
spell, form and Keyblade package at start (57 + 17 × 3 + 43 + 30 + 61 + 5 elements), decoded their
176 textures to RGBA (16,704 KB, held in memory and copied to a GPU upload buffer of the same size
when the renderer came up), and stepped and drew its own effects every frame.

What doesn't play through KH1 (each logged once, by package and element, with the reason):

- An element left out when its package is built (its load line): a KH2 bone with no KH1 anchor
  (Sora's, the forms' and the spells' bones: §18.3's table; Sora's package has none left out), a
  flag KH1's binding doesn't do (0x100, scaled by its bone; 0x10000, a two-point element: none of
  the elements that play in the t430 log; the Keyblade's element 10, which never plays, is one),
  or a group over KH1's 10 / 6 texture slots.
- An element whose effect runs a program KH1 doesn't run as KH2 (not in `KH1_SAME`, not one of
  KH2's own plugged in, §18.2, §18.9): none in the t430 log's packages; the forms' and Wisdom's
  aren't in it (no Drive).
- The second Keyblade's points (0x4004 / 0x4006 / 0x4008) while there is no second Keyblade (its
  model not in yet, or one Keyblade: §18.10).
- An element played before its package is built: Sora's and the Keyblade's are built at load
  (t430 log: built at frame 0, entered at frame 1, KH2's controller had Sora at frame 19), the
  forms' once Sora could change into them (§18.12), the spells' at load and at a level up.
- A package that didn't build (logged: the file not read, its HD textures not read).
- Off Steam (Epic): elements on KH1's blade, position or unbound bindings, or running KH2's own
  programs (Steam addresses); those on bones with KH1's own programs play.
- Flag 0x800: applied since §18.18. Not applied: Thunder 3's TBPs 0x3000 / 0x3020 draw untextured until their HD images are found (§18.12).

### 18.12 Built when needed, lean, freed when not (`kh2dpx::manage`)

The tester: the files made at run time, no extra memory kept. Sizes from the code, Sora's package
built offline from the staged `P_EX100.a.us` (texcommon not staged: a stand-in with 4 KB records,
and its HD sizes from the t409 log; `0x2d40`'s assumed 16 KB):

| Sora's package | t417..t425 | now |
|---|---|---|
| DPX | 3.12 MB (all 24 DPDs), in a buffer twice its size: 6.2 MB | 1.67 MB with the default `kh2_effects` (9 DPDs: its 7 moves, the Drive motion, Revert), 3.12 MB with `all`, in its own pages |
| HD images held by us | 84, one per texture record: 4.27 MB, for the whole session | its own images, once each (66 KB), until it is entered (at once), then none |
| HD images given to KH1 (KH1 makes a texture of each on first draw) | the same 84: 4.27 MB more, from the first play | one per DPD and TBP of its own: 14 (116 KB); the common ones once for every package: 7 (271 KB) |

The other packages (forms, Wisdom, spells, Reflect) were all built at load and kept the same
way (each its own DPX ×2 and its own copies of the common images); now each exists only while it
can play.

- **Which record KH1 looks up** (Checked (exe), `0x14031EC10`): a DPD's texture records go into a
  multimap by TBP; for each TBP the lower bound (the first record of that TBP in the list) is
  looked up (`0x14010C050 (textures, record + 0x20, 0, 0, width, height)`, the textures list
  `*0x1422C8FA8`) and the texture handle written to every record of the TBP (`+8`, `+4` = 0xFFFE).
  So only that record needs an HD entry.
- **KH2's extra case** (Checked (decomp), `0x1404AECD0`, the one case KH1's `0x14031EC10` lacks):
  after the TBP's first record, the i-th further record of the same TBP is looked up at the first
  record's pixels + i (`0x14012AA20 (textures, first + 0x20 + i, ...)`); if that finds an HD
  entry, the record gets its own texture and `+4` = 0xFFFD, else the first's and 0xFFFE. So KH2 PC
  can give each palette (CBP) of one TBP its own HD image, keyed one byte apart. KH1 gives every
  record of a TBP the first's texture.
- **The t430 log**: Thunder 3's own TBPs 0x3000 (three records, CBPs 0x3E00 / 0x3E04) and 0x3020
  found no HD image by any of their records' pixel offsets, while its 0x3040 and 0x3080 did (its
  file has seven images, `-0`..`-6`); so those textures draw untextured through KH1. Not fixed
  yet: the file isn't staged here. The build now logs, for such a TBP, each of its records (pixel
  offset, CBP, format, size) and the file's images no record took (their keys), to tell from the
  next log how KH2 keys them (**Assumption** to check: by KH2's extra case above, or otherwise).
- **The texture lookup** (Checked (exe), `0x14010C050` → `0x14010BAF0`): the file holding the
  pointer (`0x1400E2B20`), the offset in it, the HD entry with that key (`+0x54`); if the entry
  already has its texture (`+0x70`) it is returned as it is, else one is made from its data
  (`+0x60`, handed over and cleared) and kept there. A made texture keeps its file's MD5 (`+0x58`)
  and pixel pointer (`+0x80`); it is released only when that file is gone (`0x14010CDA0`: no file
  with the MD5, or the pointer no longer in it).
- **Shared common textures**: one small file of ours (`kh2fx_texcommon_dpx`: each common image's
  first record header in a 0x30-byte slot, never removed). The first package that needs an image
  gets an HD entry added to that file for it and KH1's lookup called on its slot (KH1 makes the
  texture); every package's HD entry for that image is then given that texture (`+0x70`, no data).
  So each common image is one KH1 texture for the session, made from our file, alive as long as
  it. **Assumption** (not checked in game): calling KH1's lookup at that moment (the game thread,
  where `manage` runs, not KH1's effect draw pass) is safe, as the draw pass's own call is.
- **Removal** (Checked (exe), `0x1400E2E00 (entry, 1)`): runs the file type's handler (none for
  ours, `+0x64C` = −1), marks each HD entry's texture for release after 10 frames (`+0xB0` = 10,
  `+0xB4` = 1), unlinks the entry and destroys it (`0x1400E2280`: its HD entries `_aligned_free`
  their data and are deleted; `0x1400E1EB0` writes nothing with `+0x218` = −1; the entry deleted).
  Our buffer isn't freed by KH1. Before removing a package, its HD entries holding a shared
  texture let go of it (so it isn't marked).
- **In use**: a package is freed only when no effect instance (Steam `0x142678280`, 0x100 × 0xE0:
  `+0x92` state 1 live / 4 ending, `+0x98` its DPD) and no KH1 element (`0x142D678B0`, 0x100 ×
  0x3C: `+0x14` & 3, `+0x38` a handle of its DPX) is on it (Checked (exe), `0x1401E0100`). KH1's
  texture slot table (`0x1401EDB00`) keys a run by the DPD's address and isn't cleared: a later
  DPX at the same address would reuse a stale run (**Assumption**: harmless on PC, where the slot
  is only where KH1 puts the textures it binds each draw, `0x14010DC50`).
- **Memory**: one 32 MB region reserved once (one of KH1's 64 handle regions, `0x14038AD90`, for
  all our DPXs, however often packages come and go); each DPX committed in it page by page and
  decommitted when freed.
- **When** (`desired`, checked every 30 frames and at once when the form changes): Sora's (the
  moves `kh2_effects` allows, the Drive motion, Revert; rebuilt if that setting is reloaded) and
  the Keyblade's always; Valor's, Final's and Wisdom's archive (with Wisdom's
  spells) from the moment Sora could change into them (`form::possible`: out of a form, not
  changing, MP for a form, its data loaded, Valor's second Keyblade), so each is built and
  entered before he picks it; in a form, that form's alone, until reverted (it is the one
  `arriving` from the Drive command on); Reflect's level Sora has; Fire, Blizzard,
  Thunder and Cure at KH1's level for each while base Sora's Magic styles are KH2's or he is in
  Wisdom (Fire, Blizzard, Thunder). A package is entered in KH1's loaded files (its own images
  copied to KH1, the common ones shared, KH1's relocation, KH2's programs plugged) right after it
  is built, on the game thread: a play never pays for it.
- **First use**: each package is built before its effects can first play: Sora's and the
  Keyblade's at load (the t430 log: built at frame 0 in 0.08 s and entered at frame 1, before
  KH2's controller had Sora at frame 19), a form's once Sora could change into it (MP for it, out
  of a form: built and entered long before the change's first effect), a spell's at load or at the
  level up (**Assumption**: KH1's spell levels change outside battle, from the story). The old
  player, which played them until then, is gone (§18.11). No hitch: the build is off the game
  thread, the entering is a copy of the package's own images (tens of KB) and KH1's relocation.
  KH1 makes each texture on its first draw as for its own effects.

### 18.13 The second Keyblade's points (Valor's left Keyblade, Final's second)

- KH1 has one weapon per actor; the second Keyblade is ours (`leftblade`: its model drawn in the
  left hand, its trail KH1's own on a copy of Sora). That copy (the "left owner": Sora's actor and
  model instance copied, every one of his world matrices moved by the turn from his right grip to
  the left one, its weapon object Sora's with the left Keyblade's `btltbl` row) is what KH1's own
  trail of the left Keyblade follows, so it is KH1's way to have a second weapon to bind to: KH1's
  blade binding (`0x1402ADD60 (dpx, number, actor, 0x30, t)`, mode 3) on it reads the left blade's
  bone and its range along it (`0x14029EB20`: code 0x30 through the actor's weapon object `+0x14C`
  â row `+0x20`, `+0x28`..`+0x24`; bones through its instance `+0x15C`, matrices `+0x1B4`; alive by
  `+0x374`, copied from Sora: Checked (exe)).
- So 0x4004 / 0x4006 / 0x4008 are `Anchor::LeftBlade(1 / 0 / 0.5)`: KH1's binding on the left
  owner, filled at the start (the pose before Sora's pass, as KH1's start reads Sora's) and after
  each of Sora's skeleton passes while one of them plays (this frame's pose, before KH1's binding
  task at 139999 reads it, as for the right Keyblade); the first-frame re-read (Â§17.7) and
  position-only offsets (Â§18.5) work on it as on Sora. Steam only. Without the left owner (not
  drawn yet) `kh2fx` plays them; when it goes (out of the form) KH1 stops them (**Assumption**:
  KH2's second-weapon effects end with the weapon). Not checked offline: the forms' files aren't
  staged, so which of their elements use 0x4004..0x4008 is only in the load log.

### 18.14 pppSnoMdlDttNorm (238), KH2 only: the Drive sphere's screen distortion

- **The KH2 functions** (Steam KH2 decomp, found by their frame-buffer TEX0 `0xAA000 << 14`, which
  only the distortion programs build): KeShpDtt's draw `0x1404CFA90` (KH1 `0x140338CE0`), KeMdlDtt's
  draw `0x1404CA9E0` and triangles `0x1404C9E90` (KH1 145: `0x140334DF0`, `0x1403342A0`, its PC
  draw `0x1403340A0`), and a third pair with no KH1 counterpart: draw `0x1404CBBA0`, triangles
  `0x1404CAD20`, the only reader of model kind 1 (triangles with normals). That is 238 (key @4 a
  model as KeMdlDtt's, @0x15..0x17 and @0x1C as §7.10 lists). §7.10's earlier addresses
  (`0x1404CD590`, `0x1404CD550`, `0x1404CC710`) are not these functions in this decomp.
  **Checked (decomp)**.
- **What it does**: the strength `t` (work 0, with its rate and acceleration) steps on every draw
  unless effects are held (rate += acceleration, `t` += rate; at the key's frame `t`, rate,
  acceleration += @8, @0xC, @0x10; no paused or computed-frame test); with a model (@4 ≠ 0xFFFF)
  and the colour's alpha (work 1 `+0xB`) not 0: the colour is its bytes / 128 (the decomp shows
  that operand), the strength × the root's scale x if @0x16. For each triangle of the model's
  leading kind-1 groups (0x1C bytes: three colours, three vertex indices `+0xC`, three normal
  indices `+0x14`): each vertex `p` goes through the model's matrix (work 2) and the projection
  (`0x142B0EDE0`, KH1 `0x142E9D600`) for its place; its normal (the model's, or the line from the
  model's origin with @0x16) through the model's matrix, normalised; the push `d` = that normal (with
  @0x16 the unit one; with @0x15 its x, y times the unit normal's z) × the strength, from the
  vertex's matrix place; its texture place = `d` through the projection and the screen map
  (renderer `+0x10` size, `+0xA0` viewport, `+0x38` screen: u = (x / w) · sx + ox), clamped to the
  renderer's scale (`+0xB0`); its colour = the triangle's colour × the colour, alpha eased to
  `(2 − a) a` when the frame step is under 2 (two draws a tick at 60 fps). Then per triangle KH2's
  PC draw: PRIM, TEX0 = a frame buffer (@0x14: 0 the first, bound with `0x140126890 (0)`; else
  the second, bound with "setPrevFrameTex", as KH1's KeMdlDtt binds its), ALPHA (@0x1C, 0 → 0x44), CLAMP (the distortion region), the
  shader 8 (9 monochrome), three vertices (clip place, uv × 4096, colour, 0x1000 0x1000 0), drawn
  as KH1's KeMdlDtt draws (`0x1400FB9C0 (3, 6, 0)`). Before the triangles, TEST (@0x17: 0x71001,
  else 0x21001).
- **Through KH1** (`kh2ppp::dtt_draw`, our seventh entry; draw slot only, construct and reset
  KH1's KeMdlDtt's `0x140334DB0` / `0x140334DD0`, which clear the same three floats): every KH2
  call by KH1's matching one, matched by code: PRIM field `0x140472E90` → `0x140386420`, TEX0 /
  ALPHA / CLAMP / TEST (§17.3), frame texture `0x140126890` → `0x140107F60`, setPrevFrameTex
  `0x140126C30` → `0x140108300` (KH1's KeMdlDtt draw calls it), renderer getters `0x14011BA10` /
  `0x14011C290` / `0x14011CC00` / `0x14011C700` → `0x1400FD0E0` / `0x1400FD960` / `0x1400FE2D0` /
  `0x1400FDDD0`, shader, vertices and their draw (§17.3); the frame buffers `0x140B60250` /
  `0x140B60248` / `0x140B6025C` / `0x140B60260` → `0x1425381CC` / `0x1425381C4` / `0x1425381D8` /
  `0x1425381DC`, the CLAMP region `0x140B607D0` → `0x142538218`, the frame step `0x140B60244`
  (KH2 takes it off the instance's 30 Hz accumulator) → KH1's `0x14233FBDC`; the vector maths
  (`0x14013F900`, `0x14013F850`, …) done in place, as KH1's same library does it
  (`0x1400E5AD0`: the rows weighted by the vector; normalise `0x1400E8250`, a zero vector kept).
  KH1 draws it in its effect draw pass when the instance's program runs, as KH2 does (no
  depth-sorted queue, unlike KeMdlDtt's). A gate (`kh2dpx::GATES`): element 1 plays through KH1
  only with it plugged in.
- **Assumptions** (no KH2 exe here, operands the KH2 decomp drops): the normal's scale before the
  model matrix is 1/4096 (Sora's sphere, model 5 of his second DPD, has |n| = 4096 for all 114
  normals: **Checked (files)**); the push's scale is the strength (the one value the triangle code
  reads and uses nowhere else); the vertex buffer and draw take KH1's KeMdlDtt arguments (3, 0x20,
  0) and (3, 6, 0) (the decomp shows only the count); KH2's construct and reset are KeMdlDtt's.
  Not ported: a branch for one KH2 setting (`0x140ABB020`, set from `0x140ABB278` by
  `0x140156A10`, a PC option not in KH1) that fades vertices at the far left of a wide screen.
- Element 1's key (**Checked (files)**): model 5 (224 triangles with normals), @0xC −120 (the rate
  at frame 0: `t` falls by 120 a draw), @0x14 = 1 (the second buffer), @0x15 = 1, @0x16 = 0,
  @0x17 = 0, @0x1C = 0.

### 18.15 KH2's body centre (0x4001) on KH1's (0x8005)

- KH2 (**Checked (decomp)**, `0x1403B50F0` case 0x4001): the object's turn (`+0x40`, as for 0x4000)
  at the centre of its first collision shape of kind 1 whose group is on (`0x1403C9B80 (list, 1)`,
  `0x1403BA550`), else at the object's place (`+0x670`). A shape's centre (`0x1403C9C00`): its
  offset (`+8`) through its bone's matrix (`0x4000` the object root's), then its lift (`+0xE`)
  off the world y (up). Sora's (**Checked (files)**, `P_EX100.mdlx`): entry 1, kind 1, group 0
  (the only group on by default), on the object root, offset 0, lift 85 (radius 35, height 55).
- KH1 has the same point (**Checked (exe)**, `0x14029EB20` code 0x8005): the actor's first
  collision part (`+0x134` → `+0x1C`, 0x14 bytes each, count `+7`; kind the low nibble of `+0`)
  of kind 1 not switched off (`+0x3AC` bits), else of kind 0 (`0x1402B6D30`), else its main part
  (`+0x14`), else its place: its point along its bone (`+2` code, `+4` t), raised by its lift
  (`+8`) × the actor's scale (`+0x44`). With the object-root bit (0x8000) KH1's binding takes the
  object root's frame (§16.2: − rows `+0xF0`, `+0xE0`, `+0xD0`) at that point (`0x14029EA00`), so
  KH1's start on a bone with 0x8005 (binding mode 2) is KH2's 0x4001: the object's turn at the body
  part's centre. So `kh2dpx::anchor` maps 0x4001 to KH1's 0x8005 for every package (option A, as
  the bones: each game's own body; KH1 Sora's centre is from KH1's own collision data, not
  KH2's 85), and position-only elements on it use KH1's position binding (mode 1, t 0). The first
  start logs both heights (KH1's measured on Sora, KH2's from his model). **Assumption** (as for
  0x8000, corrected in §18.17): KH1's object frame is KH2's object frame turned; KH1 Sora's kind-1 part is his body's (not
  checked offline: KH1's collision data isn't read here).
- Sora's own package has no 0x4001 element (its codes: bones, 0x4000, 0x4003; **Checked
  (files)**). Which spell, form or Keyblade elements use it isn't known offline (their files aren't
  staged): the load log names each element's anchor.

### 18.16 KH2's effect point (0x4009)

- KH2 (**Checked (decomp)**, `0x1403B50F0` case 0x4009): the object's turn (`+0x40`) at the centre
  of its first collision shape of kind 9 whose group is on (`0x1403C9B80 (list, 9)`,
  `0x1403BA550`, the centre `0x1403C9C00`); without one, only the turn is written. Kind 9 is read
  nowhere else in the KH2 decomp: a shape only for effects. Sora's (**Checked (files)**,
  `P_EX100.mdlx`): entry 11, kind 9, group 0, on the object root, offset 0, lift 105 (radius 70).
- KH1's anchor codes (`0x14029EB20`: 0x8000..0x8008, §18.15) have no point for it. The point on the root is KH2's
  0x4000 moved by the shape's centre in the object frame, so `kh2dpx` starts it as 0x4000 (KH1's
  object root 0x8000, §16.2) with the element's offset moved by (offset − lift up), from the
  owner's KH2 model (`effect_point_of`: Sora's for his, the Keyblade's and the spells' packages,
  Wisdom's for its archive, Valor's and Final's for theirs). KH2's own number on KH1's object
  frame: 105 above Sora's place (KH1 Sora is smaller: his hips 78.9 high against KH2's 88.5).
  **Assumption**: the shape's offset is in the object frame the element offsets are in (0 for
  Sora). Cure 3's elements 3 and 4 use it (t430 log).

### 18.17 KH2's object frame on KH1's object root (a quarter turn)

The tester (m14): "reflect is misaligned with sora". Reflect's 17 elements are all on KH2's object
root 0x4000 (t430 log), none on 0x4001 or 0x4009, with KH2's own offsets (barrier pieces 109, 85
and 68 above the feet, the 85 ones 22 to the side and 18 ahead, the 68 ones 14.7 ahead).

- KH1 (**Checked (exe)**, `0x14029EA00` with bit 0x8000, bytes at `0x14029EA2C..0x14029EA71`): the
  frame's rows are `0 − row +0xF0`, `0 − row +0xE0`, `0 − row +0xD0` (`0x140386D10` with
  `(0, 0, 0, 1)`: `0x1400E5EF0` subtracts), the place `0x14029EB20 (actor, code, 0)`. `+0xD0` is the
  matrix KH1's skeleton pass is given (`0x1402A0350` → `0x140391C70 (actor + 0xD0, ...)`,
  **Checked (decomp)**), so its rows are the model's x, y, z in the world; `+0xF0` (its z) is
  forward: KH1's 0x8002 point ahead is `+0x50` plus `(0, 0, 100 t)` through it (`0x14029EB20`).
  So KH1's object frame is (−forward, −y, −x): its x points back.
- KH2's object frame on the same matrix: x, −y, −z (forward along its −z). **From the old
  player** (`kh2fx`'s 0x4000, `w · diag(1, −1, −1)` on the same model → world, set from the tester's
  Guard Break check: the spiral 235 along −z in front of him, not behind), x from the frame being a
  rotation; the KH2 decomp's 0x4000 (`0x1403B50F0`) is the object's `+0x40` turn, whose relation to
  the model isn't followed here (**Assumption**, as the old player's).
- So KH2's object frame = KH1's × C, C a quarter turn about y (KH2 x → KH1 −z, KH2 z → KH1 x).
  Without it every KH2 offset and turn on the object root was turned 90° about Sora: Reflect's
  side and front pieces landed in front and to the side, its pieces' own turn 90° off. Also on it:
  Explosion's burst (no offset; its turn), Fire 3's root pieces (position only), Cure's 0x4009 pieces,
  every 0x4001 element (KH1's 0x8005 uses the same frame), the forms' root effects.
- `kh2dpx`: an element on 0x8000 / 0x8005 has its copy's offset and turn taken through C at build
  (`B₂ · T(t) · R = B₁ · T(C t) · C R`); position only, its offset through C each read
  (`by_position_now`; its turn is the world's); a script's place (`spell_place`, KH2's matrix)
  through C⁻¹. No per-frame cost beyond a 3×3 multiply per position-only read.

### 18.18 KH2's flag 0x800: the effect's alpha follows its owner's

The tester (m14, 2 (a)). Elements with it in the t430 log: Fire 3's 8 and 11 (0xa45, Keyblade tip,
by position), Explosion's wrist and chest flashes and the other 0xa61 elements of Sora's package
(bones), one 0xa65.

- KH2 (**Checked (decomp)**): the element's binding (`0x1402C58F0`, every run, after it places the
  effect; also `0x1402C6A10`), while the element is bound (its `+0x70` bit 0) and the effect list is
  the field's (`0x140B63628` = 1, set by `0x1402C4330` / `0x1402C4540` for the main task list),
  writes the effect's colour alpha (`0x1402BE1E0` → `+0xC`): the object is the owner's weapon for
  0x4003 / 0x4005 / 0x4007 (`+0xD60`), its second for 0x4004 / 0x4006 / 0x4008 (`+0xD68`), else the
  owner itself; 0 if it isn't live (`0x1403BA720`: `+0x120` & 0x10080000) or is hidden
  (`0x1403BA840`), else its alpha (`0x1403CC950`: `+0xA10` × `+0xA08`, times its parent's while
  attached: a weapon's includes its owner's). KH2's fade-out is apart from that colour (the
  effect's `+0x38` level and `+0x3C` rate, `0x1402BE6A0`), so the two multiply.
- KH1's (`kh2dpx::follow_alpha`, each of Sora's frames in `tick`, only for an element bound with
  0x01 and not yet placed by a script): 0 unless Sora's actor has its draw bit (`+0` bit 0), else
  his alpha (`+0x60`, KH1's fade, `0x140297FE0`); for the Keyblade also 0 while his "weapon out"
  flag (`+0` bit 0x4000, `0x1402A0CA0` / `0x1402A10F0`) is clear, for the second Keyblade while
  there aren't two (`form::two_blades`). Written to the instance alpha (`+0x3C`) times the
  element's fade level, which KH2's fade-out steps instead of the alpha for these elements.
- **Assumptions**: KH1's Keyblade has no alpha of its own that changes in play (its combat-state
  byte `+0x14C` → `+0xF`, 0x80 when drawn, isn't read); KH2's "hidden" cases (`0x1403BA840`: its
  `+0x124`, `+0x6C8`, `+0x120` bit 10 tests) are KH1's draw bit. Cost: one read of three actor
  fields per such live element per frame.
