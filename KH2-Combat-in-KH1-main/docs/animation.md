# Skeleton animation model

This is how `kh2-anim` poses Sora's skeleton. The rules were read from the motion evaluator
in your copy of KH2 Final Mix's executable, using Ghidra. Only plain-language notes are
written here; no decompiled code is in the repo. The file layouts come from OpenKH's docs
(MDLX, ANB/motion, objentry), which credit Kenjiuno's msetDoc research.

Status tags: **Checked (exe)** = read from the executable; **Checked** = verified on your
install; **Approximation** = ours.

## Data, all read at runtime

| What | Where |
|---|---|
| Sora's bones: 256 of them, with bind scale, rotation and translation; scale W is the bone length IK uses | `obj/P_EX100.mdlx`, model entry (BAR type 4) |
| Motions: initial pose, F-curves, IK helpers, joint order, constraints, limiters, expressions | `obj/P_EX100.mset`, each ANB's motion entry (BAR type 9) |
| Keyblade bones and its own motions (same motion ids) | `obj/W_EX010.mdlx`, `obj/W_EX010.mset` |
| Which bone holds the weapon | `00objentry.bin` (Sora's weapon-joint id = 1), then `03system.bin` → `sklt` (id 1 → right-hand bone 178, left 86) |
| Attack spheres and hurt spheres on bones | MDLX collision entries (kind 6 attack, kind 2 hurt) |

## Time

- **Key times are 60 Hz frames.** The motion's frame range and loop point are in its own
  rate (usually 30 fps), so the key time is `frame_start × 60 / fps + frames since the clip
  started`. **Checked (exe)**; the keys of every Sora motion run from 0 to twice
  `frame_end`.
- Looping wraps from `frame_end` back to `frame_return`.
- Root motion is sampled on the same timeline. Earlier versions sampled it at half the rate,
  so lunges were too short; Sora's first ground swing travels 150 units, not 92.

## F-curves

**Checked (exe):**
- Interpolation per key: constant, linear or Hermite. The Hermite tangents are scaled by
  the segment length.
- Before the first key / after the last: hold, extrapolate along the end key's tangent,
  repeat, or zero.
- Rotation channels jump instead of interpolating when two keys at most 2 frames apart
  differ by 3.14 or more (a wrap-around guard).

## The joint pass

**Checked (exe).** Bones are rotated X, then Y, then Z, then translated. Each joint keeps two
matrices:
- an **unscaled** one, the rotation chain, which children inherit
- a **scaled** one, the same rotation with each axis multiplied by the accumulated scale,
  plus an explicit position

Scale accumulates separately, component by component. Joints are processed in the motion's
joint-table order; model bones the motion doesn't address keep their bind offsets. For each
joint:
1. **Expressions** targeting it set channels; angles are in degrees.
2. **Active constraints** on it are collected. An activation table, if present, decides
   on/off by time.
3. **Position:** the joint's own translation, or the position of one source (or the midpoint
   of two). A position limiter can clamp the result.
4. **Rotation:**
   - **Orientation** copies the source's matrix.
   - **Direction / two-point** aims the X axis at the source, keeping Y up, then applies the
     joint's own X rotation as a twist.
   - Otherwise the joint's Euler rotation, optionally clamped by a rotation limiter. With
     the "rotation" flag it is in model space instead of relative to the parent.
5. **IK.** A joint flagged as an IK chain looks for the position constraint on the chain's
   end and solves analytically:
   - **One bone:** aim at the target.
   - **Two bones** (thigh, knee, then ankle): law of cosines with the bone lengths. Flag 3
     bends the other way. An up-vector constraint sets the bend plane.
   - The knee is then marked as computed, and the ankle's position comes from the chain.
   - This is how Sora's feet stay planted: idle has its lowest bone at exactly y = 0.
     **Checked.**

Not modelled: the game's ground adaptation for feet on slopes and stairs, and its "look at"
override for the head.

## Placement in the world

- **Root motion.** The hips move forward inside attack motions, and the game also moves the
  character by the root-motion curve. So the model is drawn at the character's position
  minus the clip's current root offset; the A300 hips track the root curve to within a few
  units. **Checked.**
  - The offset is taken out of the pose before blending, so cross-fades between clips don't
    jump.
- **Air poses.** In-air motions (jump, fall, high jump) keep the feet 20–40 units above the
  physics position, with legs tucked. That is the real data.
- **Keyblade.** Its own motion is evaluated with the same motion id and frame, then attached
  to Sora's bone 178.
  - The blade is rigid on its bone 2 and points along that bone's −X, where the attack
    spheres are.
  - Bones 3–11 are the keychain, which the weapon motions swing. The KH1 port puts this
    animation, and KH2's hair and hood animation, on KH1 Sora (`kh1-secondary-motion.md`).
- **Blending. Checked (exe).** Every motion start passes a blend length in 60 Hz frames.
  Lengths of 2 or less mean no blend.
  - **The blend clock** starts at 2 and advances 1 per frame. The weight from the old pose
    is clock ÷ length, and the blend ends when the clock reaches the length.
  - **Lengths the executable uses:**
    - attacks: PTYA `blend_time`, which is 0 for every Sora row, so swings start at once
    - idle/walk/run changes, and motions an action requests (jump, the fall after a jump,
      roll, ...): 8
    - Glide: 16
    - the general "start falling" helper: 16
    - the motion queued to follow a finished action (usually idle): 0
  - **Walk ↔ run** keeps the gait's phase: the new start frame is
    `new length × (1 + old position + blend) ÷ old length`, wrapped.
  - **After an action**, idle starts at once and then blends into walk/run. So the
    playground blends from idle's first pose.
  - **Not modelled:** the game holds the new motion at its start frame during the blend and
    starts its triggers afterwards. Our controller's timing doesn't wait for the blend
    (this only matters for 8- and 16-frame blends, never for attacks).

## What the playground draws

A simple mannequin of our own design on Sora's main bones:
- rounded torso and hips
- head
- capsule limbs
- mitten hands
- big shoes

It glows white while invulnerable and cyan while guarding. The Keyblade prop has a grip, a
guard, a blade and a swinging keychain. Fingers, hair and clothing bones are animated but
not drawn. No mesh, texture or other asset is loaded.

Debug views:
- **F1:** live attack shapes, including sweep copies (red), or guard shapes (cyan)
- **F2:** hit shapes
- **F3:** the skeleton as lines (Sora white, Keyblade gold)
- **F4:** every other collision type, colour-coded by type

Every shape uses its real kind and size: sphere, column, ellipsoid or cube. Hits use the
game's overlap test (see `combat-model.md`, "Hit detection").
