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
Aerial Finish (§12.1). `w[i]` = the program's i-th work offset;
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
