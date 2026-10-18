# KH1 field camera: who drives it, its modes, input, collision, and where to take it over

Research notes (Ghidra headless on the Epic KH1 Final Mix PC exe, image base `0x140000000`,
addresses absolute; the hook adds `base + (addr - 0x140000000)`). Everything here was read
statically from the code and has not yet been checked in play. Plain-language notes only.
Raw decompiler output stays outside the repo (`/opt/re/out/k1cam_range.c`,
`/opt/re/out/cam/k1c_*.txt`).

Confidence tags: **[high]** read directly from code; **[med]** several pieces agree but the
meaning is inferred or untested; **[low]** a guess.

Conventions as in `kh1-render.md`: world is right-handed with **+y down**, units about 1 cm,
angles in radians. `0x1423405DC` is the frame time step every camera timer subtracts
(timers below are in those units; the code treats a step below 2.0 as the high frame rate and
halves some per-frame gains).

---

## 1. Summary

- The camera that the GPU sees is built by the task `0x14028ABC0` (priority 110000) from the
  **public** eye/target `0x14299C4E0` / `0x14299C4F0`, roll `0x14299C504` and FOV
  `0x14299C500` (see `kh1-render.md` section 1). That task only builds matrices. It never
  decides where the camera is. [high]
- The field camera is a separate module (code `0x1401DF7C0`..`0x1401E7000`, globals
  `0x142538840`..`0x142538B80`). Its task **`0x1401E3C30`** (priority **100000**, registered by
  `0x1401E28A0`) runs the current camera **mode** from a function table. The mode writes
  KH1's **internal** eye/target `0x142538A30` / `0x142538A40`. The task then copies those into
  the public pair, or eases from the previous mode's camera into them while a mode-switch
  blend runs. [high]
- The player's think runs before the camera, so Sora's position and facing are final when the
  camera runs. Event script tasks run at 99999. [high for the priorities, med for "think first"]
- Four other tasks can overwrite the public pair after the field camera, before 110000.
  `0x140287BC0` and `0x1403A0860` (both 100000) run event camera commands that place the
  camera relative to an actor. `0x1402C7530` and `0x1402CB910` (both 100500) are special-scene
  cameras. They are registered only in those situations. [high that they write, med on roles]
- The ordinary follow camera is **mode 1** (`0x1401DF7C0`). The camera stick is the processed
  right stick at `0x14298D4D0` (x) / `0x14298D4D4` (y), each in -1..1. Inversion signs are
  at `0x140507AA8` / `0x140507AAC`, and the options live in the config block
  `*(0x142E1FE70)`. [high / med]
- KH1's camera keeps itself out of walls with a **sphere sweep** `0x140194410` (radius 40,
  moved 20 units at a time) and the **ray** `0x140191640`, which is the same as
  `0x140191520(ray, -1)` wrapped in the collision lock. [high]
- **Recommended takeover:** replace the mode-1 entry of the mode table (a pointer in `.data`,
  `0x140507AE0`) with our own `void(void)` function. That function writes the internal
  eye/target plus KH1's yaw/pitch/distance. KH1 keeps its mode switching, blends, events and
  View build (section 8).

---

## 2. Data flow per frame [high]

```
99999   event scripts (0x1401BD700 when an event runs)
100000  0x1401E3C30  field camera task:
          if 0x142538AC4 != 0 (camera released to an event): set it to 2, return
          subject speed: 0x1425389EC (3D), 0x142538A18 (horizontal), from actor +0x10 - +0x490
          if (0x142867CD8 & 2) == 0 and the mode has an update function:
              0x1401E33D0  area-camera zone test (may switch to/from mode 9)
              force mode 10 in one room (0x142340E5C == 6 && 0x142340EC4 == 10)
              mode update  ->  writes 0x142538A30 / 0x142538A40 (+ yaw/pitch/dist state)
              if blend 0x142538A20 > 0: count it down; public eye/target =
                  lerp(blend-from 0x142538AD0 / 0x142538AE0, internal, 1 - cos(...)) eased
              else: public 0x14299C4E0 / 0x14299C4F0 = internal 0x142538A30 / 0x142538A40
100000  0x140287BC0, 0x1403A0860   (event camera commands, only while registered)
100500  0x1402C7530, 0x1402CB910   (special-scene cameras, only while registered;
                                    0x1402CB910 skips when (0x142867CD8 & 2) or mode 9)
110000  0x14028ABC0  View / View*Screen from 0x14299C4E0 / 4F0 / 504 / 500 (kh1-render 1.4)
135000  0x14028AA80  previous-slot copy (one-frame lag, kh1-render 1.4)
```

Other code that writes the public pair directly: `0x1401E29D0` (restore from a snapshot,
see 6.4), `0x1401E1E60` (snap behind Sora, a script command), `0x1401E2170` (enter the
conversation camera), `0x14028A5B0` (init), and the event-script camera commands `0x1401BD8C0`
(orbit, also writes roll) and `0x1401C0520` / `0x1401C08D0` (they read it). The pause
menu's task (`0x1402D3040`, priority 159999; kh1-render 3.1) uses its own view through `0x142340ED8` /
`0x142340EE0` (kh1-render 1.1).

---

## 3. Camera module globals [high unless tagged]

| Address | Type | Meaning |
|---|---|---|
| `0x14299C508` | i32 | **camera mode** (section 4) |
| `0x142538A10` | ptr | subject actor (Sora, copied from `0x142D37C80`); 0 while the camera is released |
| `0x142538AC4` | i32 | 1 = camera released to an event (`0x1401E2560`), the task sets it to 2; 0 = normal |
| `0x142867CD8` bit 1 (`& 2`) | flag | event "camera off": the field camera task skips the mode update and copy. Set by `0x14028BC30` / event actor command 0 (`0x14028C550`), which also cancel the lock-on and call `0x1401E2560`. Cleared by `0x14028BC20` / command 1, which call `0x1401E2500` (gives the camera back to Sora, mode 1 unless an area camera applies) |
| `0x142867CD8` bit 13 (`& 0x2000`) | flag | first-person view not allowed (Select gives a buzzer, sound 4, instead) |
| `0x142867CDC` bit 4 (`& 0x10`) | flag | turns off the auto-follow pieces, the movement-state cameras and first person. Probably a special-movement world [low on the meaning] |
| `0x1425389E8` | u32 | mode-entry flags: bit 1 = "mode just entered" (each mode's first frame re-initialises and clears it); bit 0 = entered from mode 0; bit 2 = restored from a snapshot |
| `0x142538A30` | vec4 | **internal eye** |
| `0x142538A40` | vec4 | **internal target** |
| `0x142538A50` | f32 | **pitch** (eye above target when positive, see 3.1) |
| `0x142538A54` | f32 | **yaw** = `atan2(eye.x - tgt.x, eye.z - tgt.z)` (`0x14028A590`); recomputed from eye/target at the end of most modes |
| `0x142538A5C` | f32 | **distance** eye-target (3D) |
| `0x1425389F0` | f32 | default pitch (built-in 10° = 0.1745 from `0x140507AC0`; the map's camera block overrides it) |
| `0x1425389FC` | f32 | default distance (built-in 400; map block overrides) |
| `0x142538A00` | vec4 | target offset in the subject's local frame (applied through the actor matrix `+0xD0`). Built-in `(0, -100, -50)`, i.e. 100 above the feet and 50 to the side the camera looks from [med on the direction of z]. `0x142538A04` (its y) comes from the map block |
| `0x1425389B8`..`0x1425389C0` | f32 x3 | saved pitch/distance/height (`0x1401E2E80` switches to one of two presets at `0x140507A90`, `0x1401E2D30` restores) |
| `0x142538A20` / `0x142538A1C` | f32 | mode-switch blend: time left / duration. The ease is `1 - cos(t/T * pi/2)` |
| `0x142538AD0` / `0x142538AE0` | vec4 | blend-from eye / target (captured by `0x1401E3200`) |
| `0x142538850` / `0x142538854` | f32 | pitch / yaw angular velocity from the stick (section 5) |
| `0x142538AA8`..`0x142538AC0` | | R3 "recenter behind" animation: start yaw `AA8`, yaw delta `AAC`, distance `AB0`, time left `AB4`, duration `AB8` (60), kind `ABC` (0/1 = direction, 2 = blocked), start pitch `AC0` |
| `0x142538A98` / `A9C` / `AA0` | f32 | auto wall-avoid swing: start yaw, delta, time left (40) |
| `0x142538840`..`0x14253884C` | f32 | auto swing-behind: last subject yaw and the two ramp timers |
| `0x142538B50` / `B54` / `B58` | f32 | auto pitch timers |
| `0x142538870` | ptr/u64 | lock-on camera target (nonzero while the lock camera should run) |
| `0x142538880` / `0x142538878` | vec4 / f32 | lock-camera blend-back target and timer |
| `0x142538960` | vec4 | conversation camera centre (mode 7); `0x142538970` = chosen diagonal 0..3, 4 = none |
| `0x142538A28`, `0x1425389C4`/`C8`, `0x142538A78`/`A60`, `0x1425389E0`/`D0` | | map camera block (`0x1401E2FB0`): counts and tables of fixed cameras (0x20 bytes each), rail cameras (variable), and trigger zones (0x30 bytes: AABB min `+0`, max `+0xC`, camera handle `+0x20`, flags `+0x24`) |
| `0x1425389D8` / `0x142538A68` / `0x142538A70` | ptr | active fixed camera / active rail camera / current zone |
| `0x142538B60` (0x20 bytes) | | camera snapshot (`0x1401E2D80` fills it: eye, target, FOV, mode byte, area-camera index and kind); `0x1401E29D0` restores. Save/menu return uses it (`0x14017A6D0`) |

### 3.1 How mode 1 turns yaw/pitch/distance into an eye [high structure, med sign]

`eye = target + (d·cos p·sin y, −d·sin p, d·cos p·cos y)`, where `d = 0x142538A5C`,
`p = 0x142538A50` and `y = 0x142538A54`. The code rotates `(0, 0, d)` by X(pitch) and then by
Y(yaw) (`0x1400E6F20`, `0x1400E70E0`, row vectors). So positive pitch lifts the eye (smaller
y), and yaw 0 puts the eye on +z of the target. "Behind Sora" is yaw = actor facing
(`+0x3C`) + π, as used by the recenter (`0x1401E4140`) and the snap (`0x1401E1E60`).

---

## 4. Camera modes (`0x14299C508`) [high for the table, med for names]

The mode table at **`0x140507AD0`** has 16 bytes per mode: `+0` is the update and `+8` the
exit (called when leaving), absolute pointers in `.data`, so they are writable. A null update
means the task does nothing in that mode, not even the copy. The switch is
**`0x1401E3200(mode, blendTime)`**. It calls the old mode's exit, captures the blend-from
camera (internal if no blend was running, else the public pair), and sets flag bit 1. From
mode 0 it instead loads the internal pair from the public pair and forces blend 0 (a cut).

| Mode | Update | What it is |
|---|---|---|
| 0 | none | hold / released. Nothing written; events or scripts own the public pair. Entered by `0x1401E2560` (release), `0x1401E2C80` (script "hold", remembers the yaw relative to Sora in `0x1425389B4`) |
| **1** | **`0x1401DF7C0`** | **ordinary follow camera** (section 6) |
| 2 | `0x1401E0230` (exit `0x1401E04B0`) | first-person view. Select (logical bit `0x8000`) from modes 1/3/9. Sora is hidden; the left-stick y (`0x14298D4E4`) pitches -80°..+60°. Select again or R3 leaves (to 1, or 9 in a camera zone) |
| 3 | `0x1401E04F0` | **lock-on camera**. Entered by `0x1401E2040(target)` from the lock code `0x1402AE710` (R1 state 3 and state 4; blend 60), left by `0x1401E1FB0` from `0x1402AE7E0` (lock cancelled; back to mode 1 with blend 45). Aims between Sora and the lock point `0x1402AE140(0x142DB3F18)` |
| 4 | `0x1401E0C80` | Sora movement state 9/10 (vines / climbing): fixed offsets `0x140507A00` (eye) / `0x140507A10` (target) in Sora's frame |
| 5 | `0x1401E0E50` | movement state 0xF: offsets `0x140507A20` / `0x140507A30` |
| 6 | `0x1401E0F80` | movement state 0xD/0xE (ledge): offsets `0x140507A40` / `0x140507A50` |
| 7 | `0x1401E1150` | conversation camera between two actors (`0x1401E2170` from event command `0x1401B77F0`): one of the four diagonals at `0x140507BB0` |
| 8 | `0x1401E1390` | movement state 0x12: the right stick orbits digitally (logical `0x1000` / `0x2000`); R3 recenters |
| 9 | `0x1401E1850` | **area camera** from the map's zones: a fixed point (`0x1401E36B0`, FOV lerped by distance) or a rail/spline (`0x1401E5980`) |
| 10 | `0x1401E1890` | one special room (`0x142340E5C == 6 && 0x142340EC4 == 10`): a top-down offset `0x140507A80` |
| 11 | `0x1401E19D0` | actor-attached look-around (d-pad up/down tilts, R3 leaves to mode 1); only scripts enter it [low] |

Mode 1 hands itself to modes 4/5/6/8 through `0x1401E3320`, which checks Sora's movement
state `+0x70` at the end of each mode-1 frame. Those modes go back to 1 with blend 30 when
the state ends. Area zones (`0x1401E33D0`, run before every update) switch between 1 and 9 with
blend 30.

### 4.1 "Is KH1 in its ordinary follow camera right now?" [high]

All of these must hold:

1. `*(i32*)0x14299C508 == 1`
2. `*(u64*)0x142538A10 != 0` (and equal to `*(u64*)0x142D37C80`)
3. `*(i32*)0x142538AC4 == 0`
4. `(*(u32*)0x142867CD8 & 2) == 0`

Optional, depending on taste:

5. `*(f32*)0x142538A20 <= 0`: no blend running.
6. No special scene. Simplest test: at 110000, check that `0x14299C4E0` / `0x14299C4F0` still
   hold what the field camera wrote. The 100000/100500 tasks above would have changed them.
7. `0x14298D3A0 == 0`: no event (from the AGENTS notes).

Lock-on is mode 3, first person is mode 2, area/fixed cameras are mode 9, and cutscenes
release the camera (mode 0, `0x142538AC4`, `0x142867CD8 & 2`). So condition 1 alone already
excludes all of them.

---

## 5. Input [high unless tagged]

### 5.1 Sticks

The pad is processed once a frame in `0x140288F30`. Per pad slot (`0x14298D3C0 + slot*0x70`)
`0x140289360` turns the four stick bytes (order rx, ry, lx, ly; 0 = left/up, 0xFF =
right/down) into vectors. Each axis is `byte - 127.5`, a dead zone `0x14298D3B4` is removed
(set to 60.0 at init by `0x140288E40` [med that nothing changes it later]), then it is divided
by `127.5 - deadzone` and the vector is clamped to length 1. The results are copied to the
frame globals unless input is blocked (`0x142867CDC & 4`, which zeroes them):

| Address | Meaning |
|---|---|
| **`0x14298D4D0`** | **right stick** vec4: x (`-1` left .. `+1` right), y at **`0x14298D4D4`** (`-1` up .. `+1` down), magnitude ≤ 1 |
| `0x14298D4E0` | left stick, same layout (`0x14298D4E4` = y) |
| `0x14298D4F0` / `0x14298D500` | right / left stick unit directions |

On PC the mouse can stand in for the right stick: when `0x1422DBDFC` is set, `0x140138850`
writes the mouse delta into the right-stick bytes. `0x1422DBDF8` (`0x1401392B0`) is
"mouse/keyboard mode", which changes mode 1's speed limits. The mouse sensitivity is a u16
at `0x14012AC50()+0x20`. [med]

`0x140289360` also produces digital direction bits from the right stick: right `0x200000`,
left `0x800000`, up `0x100000`, down `0x400000` in the raw word. [high]

### 5.2 Buttons

The raw word `0x14298D3C4` is in PS2 order (Select 1, L3 2, R3 4, Start 8, d-pad 0x10..0x80,
L2 0x100, R2 0x200, L1 0x400, R1 0x800, Triangle 0x1000, Circle 0x2000, Cross 0x4000,
Square 0x8000) plus the stick bits above. It is remapped through the table `0x140528F90` (19
`{raw mask, logical bit}` pairs) into the logical button record **`0x14298D4A0`**:
`+4` held, `+8` pressed this frame, `+0xC` released, `+0x10` auto-repeat (`0x1402892A0`).
Camera-relevant logical bits:

| Logical bit | Raw source | Camera use |
|---|---|---|
| `0x200` | R3 | pressed: **recenter behind Sora** in mode 1 (`0x1401E4140`), leave first person / mode 11, R3 swing in mode 8 |
| `0x8000` | Select | pressed: **first-person view** (mode 2) from modes 1/3/9 |
| `0x1000` / `0x2000` | right stick left / right (digital) | held: rotate in mode 8; cancels the auto wall-avoid swing; buzzer in an area camera |

L2/R2 have no camera role here (raw L2 maps to `0x800` and `0x400000`, raw R2 to
`0x400000`). [high for the table, med for the PS2 names]

### 5.3 How mode 1 applies the stick (`0x1401E51C0`) [high]

The stick only acts when Sora's actor `+0x444` bit 0 is clear, no blend is running and no
recenter is running. The yaw velocity `0x142538854` accumulates `+stickX * accel * k`. The
pitch velocity `0x142538850` accumulates `-stickY * accel * k`. Both are clamped to ± max and
decay by `decay * k` when the stick is released. Here k = 0.5 at high frame rate, else 1.

- Pad: accel `0x1403EB1F8` (0.15°), max `0x1403EB20C` (2.5°), decay `0x1403EB1FC` (0.24°).
- Mouse: accel `0x1403EB200`, decay `0x1403EB214`, max from the sensitivity setting.

Then:

- `pitch = clamp(wrap(pitch + 0x140507AA8 * vPitch * dt), -15°, +50°)`
  (`0x1403EB278` / `0x1403EB234`)
- `yaw = wrap(yaw + 0x140507AAC * vYaw * dt)`

### 5.4 Options [high for the addresses, med for the labels]

Config block pointer **`0x142E1FE70`** (dwords; menu `0x1402F7DD0`, setter
`0x1402F8F90(item, value)`):

| Field | Meaning |
|---|---|
| `+0x08` (item 2) | **Field camera: 0 = Auto, 1 = Manual**. Read by `0x1402F8C90`. At 0, mode 1 runs the auto-follow helpers (6.2); at 1 it skips them and uses the simpler manual branch [med label, high effect] |
| `+0x14` (item 4) | **camera up/down: 0 normal, 1 reversed** → `0x140507AA8 = 2v - 1` (`0x1402F8B00`) |
| `+0x18` (item 5) | **camera left/right: 0 normal, 1 reversed** → `0x140507AAC = 1 - 2v` (`0x1402F8B30`) |

"Default settings" sets `0x140507AA8 = -1`, `0x140507AAC = +1`. The first-person view also
uses `0x140507AA8`.

---

## 6. Mode 1 in detail (`0x1401DF7C0`, `void(void)`) [high]

### 6.1 Every frame

1. **Entry (flag bit 1).** Pitch ← `0x1425389F0`, yaw ← from the public eye/target, distance ←
   `0x1425389FC`. Calls `0x14028A960(default FOV 0x1403EAC84)`. Then `0x1401E26C0(yaw, eye,
   target)`: target = Sora's matrix × `0x142538A00`, eye at the default offset; a ray from
   Sora's head to the eye (`0x140191640`, then `0x140191500`), and if it hits, a sphere sweep
   (`0x140194410`, radius 40) in 20-unit steps. Clears the area-camera pointers. Bit 0 clears
   the recenter and auto-swing timers.
2. **Target spring.**
   - With no blend running: desired = Sora's matrix × offset. If a segment test
     (`0x140191030`) from the head-height point to the full offset hits, the head-height point
     is used instead.
   - The target moves toward desired by 20 % of the horizontal gap each step. Vertically it
     moves 50 % of the gap when the gap is downward and 10 % when upward. Both are halved at
     high frame rate.
   - While a blend runs, the target snaps to desired.
3. **Stick** (5.3).
4. **R3 recenter in progress** (`0x142538AB4 > 0`): yaw = start ± delta · sin ease; pitch goes
   back to 10° (`0x140507AC0`, not the map default); distance becomes `AB0`.
5. **Auto branch** (option Auto and movement state not 0x1B):
   - The three helpers in 6.2 run.
   - eye = target + offset(yaw, pitch, distance) (3.1).
   - If the distance grew beyond default + 299 (`0x1404308E8`), the camera is re-placed with
     `0x1401E26C0`.
   - Otherwise the eye walks from the old eye toward the new one in ≤ 20-unit steps with the
     40-radius **sphere sweep**, stopping on contact. A jump of ≥ 1000 teleports.
   - `0x1401E5620`: five segment tests from Sora's head (and four offset points) to the eye.
     If all are blocked, the camera is re-placed behind on the current yaw (`0x1401E26C0`).
   - When the subject stands still and is close (pitch below 60°, horizontal gap
     < `0x1403EB018` = 40), `0x1401E4140` starts an automatic recenter.
6. **Manual branch** (option Manual or state 0x1B):
   - eye = target + offset.
   - Ray `0x140191640` from the target toward a point 60 past the eye; on a hit the eye stops
     at the hit, or 60 short of it.
   - If eye-target < 20, the target is pushed away.
7. **Buttons.**
   - R3 pressed, and Sora's `+0x444` bit 0 clear: `0x1401E4140` (60-step eased swing to
     facing + π; it tests 30° steps for walls and turns the other way when blocked).
   - Select pressed, movement state 0, 8 or 0x18, Sora's `+0x444` bit 0 clear: first person (mode 2, blend 30),
     or a buzzer when `0x142867CD8 & 0x2000`.
8. **Exit bookkeeping.**
   - In Auto: distance ← actual + (default − actual) · 0.1, so it springs back after a
     pull-in. In Manual it is the default.
   - yaw ← atan2 of the new eye/target.
   - `0x1401E3320` (movement-state camera switch).

### 6.2 Auto helpers (option "Auto" only) [high mechanics, med intent]

- `0x1401E4E90`: swing behind while moving. Above a speed threshold, the yaw eases toward
  facing + π with ramps `0x142538844`/`848`. Weaker at > 60° off; off while the recenter runs.
- `0x1401E4AB0`: wall-avoid. If a wall is right behind/beside the target (two segment tests at
  ±30°), it starts a 40-step yaw swing toward the wall's tangent (`0x142538A98`/`A9C`/`AA0`).
  Holding the right stick sideways cancels it.
- `0x1401E4510`: pitch assist. It raises the pitch when Sora drops off a ledge or the eye is
  near a floor (ray down 55 units), otherwise eases the pitch back to the default at 3 % per
  step. Clamped to ≤ 85°.

---

## 7. Collision used by the camera [high]

| Function | Inputs | Outputs | Camera use |
|---|---|---|---|
| **`0x140194410(sphere*)`** sphere sweep with slide | `+0` start, `+0x10` move, `+0x20` radius (camera: **40.0**) | `+0x30` = end after up to 3 push-out passes against background faces (`0x14018DF30` / `0x14018DB50`); returns nonzero if it touched anything | mode 1 eye walk (20-unit steps), `0x1401E26C0` |
| **`0x140191640(ray*)`** | ray as for `0x140191520` | as `0x140191520` | the line from target/head to the eye, clamping the eye at the hit point: modes 1 (manual), 3, 4, 5, 6, 8, `0x1401E26C0`. It is `0x1401916C0(ray, 3, 0, -1)` between the collision lock `0x14018F880` / unlock `0x14018F6C0` |
| `0x140191500(ray*)` | | | the same without the lock |
| `0x140191520(ray*, filter)` | ray `+0` start, `+0x10` direction (full length) | `+0x20` point, `+0x30` face (short, -1 none), `+0x34` object | **the camera passes filter `-1`**. Internally `0x1401916C0(ray, kindMask 3, objectsToo 0, filter)`. The second argument the mod calls "mode" is the filter handed to the object test `0x140186D10` (-1 = no filter); the background kind mask is fixed at 3. The nearest of the object hit and the background hit wins [med on the meaning of the filter] |
| `0x140191030(seg*)` | `+0` start, `+0x10` vector | returns 1 if objects or background (`0x140192230`, mask 8) block, else -1 | visibility and occlusion tests (`0x1401E5620`, recenter, auto helpers, area/lock cameras) |
| `0x1401941B0(capsule*)` | `+0x20` radius, `+0x24` half-height | overlap test only | auto swing (target or eye inside geometry) |

For a KH2-style "pull the camera in front of walls" use one of these:

- **`0x140191640`** from the look-at point to the desired eye, and put the eye at the hit point
  moved back toward the target by the camera radius.
- Or the 40-radius sweep `0x140194410` from the target toward the eye. That is what KH1's
  follow camera effectively ends up with.

Both are safe on the game thread inside the camera task.

---

## 8. Recommended hook

### 8.1 Primary: take over mode 1's update pointer [high mechanics, med as untested]

- **Slot:** the 8-byte pointer at **`0x140507AE0`** (mode table `0x140507AD0` + 1*0x10, `.data`,
  writable). It normally holds `0x1401DF7C0` (already rebased in memory). Swap in our
  function; restore the original to give the camera back.
- **Signature:** `void ours(void)`, Win64. It is called from the field camera task
  `0x1401E3C30` at priority 100000, on the game thread, only while the mode is 1. Return value
  ignored.
- **What ours must do each call:**
  1. Clear `0x1425389E8` bits 0..2, as KH1's mode-1 update does. Bit 1 set means the camera
     just came back to the follow mode from another mode, an event or a map load: start the
     KH2 camera from the current internal eye/target (or from Sora's facing).
  2. Compute the KH2 camera, then write the world-space eye to `0x142538A30` and the target to
     `0x142538A40` (vec4, w = 1).
  3. Keep KH1's state coherent (6.1 and section 9): pitch `0x142538A50`, yaw `0x142538A54`
     (atan2 form), distance `0x142538A5C`. Zero the velocities `0x142538850` / `0x142538854`,
     `0x142538AB4`, `0x142538AA0`.
  4. Optionally call `0x1401E3320()` at the end so KH1 still switches to its climbing, ledge
     and other state cameras and comes back.
  5. Optionally handle Select / R3 yourself, or ignore them. First person is
     `0x1401E3200(2, 30.0f)`.
- **What KH1 then does for us:**
  - copies the pair to `0x14299C4E0` / `0x14299C4F0`, or eases into it from the previous mode
    (lock-on, area camera, first person) with its own blend;
  - builds View/projection at 110000, so culling, the HUD, the lock-on cursor, sound
    listener (`0x140180070`) and depth sort all stay consistent;
  - keeps mode 3 (lock-on), 9 (area cameras), events and the special-scene tasks exactly as
    before.
- **FOV:** leave `0x14299C500` alone, or set it with `0x14028A960(f32 degrees)`. Mode 1's entry
  resets it to the default.
- To also replace the lock-on camera, swap mode 3's pointer `0x140507B00` (original
  `0x1401E04F0`) the same way.

### 8.2 Alternative: inline hooks

- **`0x14028ABC0`**, the View-build task, Win64 `u64(void)`, run as a task. Pre-hook it:
  overwrite `0x14299C4E0` / `0x14299C4F0` when 4.1 holds, then run the original. Prologue (15
  bytes, three position-independent instructions, enough for a 14-byte absolute jmp):
  `48 89 5C 24 08` (mov [rsp+8],rbx), `48 89 74 24 10` (mov [rsp+10h],rsi),
  `48 89 7C 24 18` (mov [rsp+18h],rdi). Next: `55 48 8D AC 24 20 FF FF FF`.
  This is the last point before View is built and wins over every earlier writer. Also write
  the internal pair and the state of section 9, or KH1's camera drifts away from ours and
  jumps when handed back.
- **`0x1401DF7C0`** (mode 1 itself, `void(void)`). Prologue (15 bytes, position-independent):
  `4C 8B DC` (mov r11,rsp), `55` (push rbp), `49 8D 6B A8` (lea rbp,[r11-58h]),
  `48 81 EC 50 01 00 00` (sub rsp,150h). Next: `48 8B 05 ..` (RIP-relative cookie load).
  Equivalent to 8.1, but needs a trampoline. Calling the original first and then overwriting
  the internal pair also works, and keeps KH1's R3/Select handling.
- Do **not** inline-hook `0x1401E3C30` at its entry: its first 14 bytes contain RIP-relative
  instructions (`48 8B 05 0D 1B 37 00` and `83 3D 7A 4E 35 02 00`).

---

## 9. Handing the camera back without a jump [med]

KH1's follow state is almost entirely derived from the internal eye/target each frame. If we
keep these written while we drive, KH1 continues smoothly when its own update is restored:

- **Must write:**
  - `0x142538A30` (eye) and `0x142538A40` (target);
  - `0x142538A50` pitch, `asin((tgt.y - eye.y)/d)` with KH1's sign (eye above → positive);
  - `0x142538A54` yaw, `atan2(eye.x - tgt.x, eye.z - tgt.z)`;
  - `0x142538A5C` distance (3D).
- **Should zero:** `0x142538850`, `0x142538854` (stick velocities); `0x142538AB4` (recenter);
  `0x142538AA0` (wall swing); `0x142538844`, `0x142538848`, `0x14253884C` (swing ramps);
  `0x142538B50`, `0x142538B54` (pitch timers).
- **Do not set** `0x1425389E8` bit 1 when handing back. If it is set, KH1's mode 1 re-places
  the camera at its default pitch/distance behind the current yaw, which is a visible jump.
  Leave it clear and KH1 springs from our camera: the target eases toward its own offset and
  the distance toward `0x1425389FC` at about 10 % per step. For a gentle hand-back, set the
  blend instead: `0x142538AD0`/`AE0` = our eye/target, `0x142538A1C` = `0x142538A20` = 30.
- Mode switches KH1 does while we drive (lock-on, area camera, events) capture the blend-from
  camera from the internal pair (or the public one mid-blend). Since both hold our camera,
  they blend from our view.
- KH1's own **reset behind Sora**:
  - R3 → `0x1401E4140()`: a smooth 60-step swing to facing + π.
  - Script snap → `0x1401E1E60()`: instant. Target = Sora × `(0, -100, -50)`, eye = target +
    `Ry(facing + π) × (0, -20, 400)` (`0x140507BC0` / `0x140507BD0`), mode 1 with blend 30.
  - Event return → `0x1401E2500()`: mode 1 from mode 0, a cut followed by the entry re-place.

---

## 10. Open questions / runtime checks

- Log `0x14299C508`, `0x142538AC4`, `0x142867CD8` and `0x142538A20` while walking, locking on,
  entering a fixed-camera room, using first person and running a cutscene, to confirm the
  mode numbers in section 4.
- Confirm the pitch sign (3.1) by logging `0x142538A50` against eye.y − target.y.
- Confirm the "Field camera" label of config `+0x08` by toggling it in the menu.
- Check whether `0x14298D3B4` (stick dead zone, 60/127.5 at init) is changed by the PC
  launcher's settings.
- Which situations register the 100500 tasks `0x1402C7530` and `0x1402CB910` (a chase camera
  that follows actor `0x142DE9AE8`). If KH2's camera should also run there, it needs the 8.2
  pre-hook instead of the table swap.
