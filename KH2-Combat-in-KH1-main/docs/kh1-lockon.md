# KH1 lock-on: where on the enemy the cursor sits

Research notes (Ghidra, Epic KH1 Final Mix PC, image base `0x140000000`, addresses absolute;
the hook adds `base + (addr - 0x140000000)`). Read from the code statically; not yet checked
in play.

## Summary

KH1 does not lock an *actor*, it locks an **(actor handle, lock-point index)** pair. Each
actor's collision record carries a small table of *lock points* (separate from the hit
parts); each lock point is a bone (or a special anchor) plus an offset and a radius. The
cursor, the lock-on's range checks, KH1's own attack homing and the command menu all compute
the aim point with one function:

    0x1402AE330  f32 lock_point(actor*, int point, vec4* out)

It writes the world position (KH1 world, y down, w = 1) into `out` (16 bytes, unaligned
stores are fine) and returns the point's radius (0 when the actor has no lock points and the
root is used). It only reads memory.

The locked pair lives in the two dwords at `0x142DB3F18`:

| address | meaning |
|---|---|
| `0x142DB3F10` u32 | lock state (below) |
| `0x142DB3F14` u32 | distance band of the lock point: 0 near, 1 mid, 2 far (cursor colour) |
| `0x142DB3F18` u32 | target actor handle |
| `0x142DB3F1C` i32 | **lock-point index** on that actor, -1 when nothing is locked |
| `0x142DB3F20` 96 x u64 | the R1 cycle list of (handle, point) pairs; count `0x142DB422C`, cursor `0x142DB4230` |

So the current aim point every frame is `lock_point(ptr(*0x142DB3F18), *0x142DB3F1C)`.
Also mirrored: Sora's actor `+0x74` (handle) / `+0x78` (point index), written by
`0x140290950`; and the command record `0x142D54444` / `0x142D54448` (handle / point) that
the command step `0x1402A5380` aims at.

## Lock states (`0x142DB3F10`)

- 0: none. 1, 2: soft target (2 is the automatic pick by `0x1402AE870`; 1 is set by
  `0x140292760`); 4: a soft target held for 90 time units after a command was used on it
  (set by the command menu `0x1402912F0`). These draw the small cursor.
- 3: the R1 lock (set by `0x1402AE480`, the R1 handler; R1 again cycles through the list at
  `0x142DB3F20`). Cancelled by `0x1402AE7E0`.
- Frame update `0x1402AF020`: drops the lock when the pair stops being valid
  (`0x1402ADEA0`) or, for states 3/4, when the point is more than 1500 units away
  horizontally or 1000 vertically beyond its radius (`0x1402AF260`); re-picks the auto target
  in states 0/2; recomputes the distance band (`0x1402AF590`, thresholds from the u16s at
  `[0x142D54430] + 0xC / +0xE`).
- The cursor `0x1402729B0` draws at `lock_point(target, point)` projected to the screen:
  state 3 uses the big cursor coloured by the band, states 1/2/4 the small one.

The hook today treats every nonzero state as "locked"; only 3 (and arguably 4) is the
player's R1 lock.

## Data layout

Actor's collision record `col = ptr(actor + 0x134)`:

| col offset | meaning |
|---|---|
| `+5` u8 | nonzero: a "body" entry exists at `ptr(col + 0x14)` (same 0x14-byte format as a lock point) |
| `+6` u8 | hit-part count (list at `+0x18`, 0x14 bytes each; disabled bits at actor `+0x3A8`) |
| `+7` u8 | **point count**, list at `ptr(col + 0x1C)`, 0x14 bytes each; disabled bits at **actor `+0x3AC`** |

Point entry (0x14 bytes):

| offset | meaning |
|---|---|
| `+0` u8 | low nibble: kind (0 = lock-on point; 1 and 2 are other aim points, e.g. a kind-2 entry whose high nibble matches is used by `0x1402AE140`); high nibble: group |
| `+2` u16 | anchor code (bone id or special, below) |
| `+4` f32 | position along the bone, 0 = parent joint, 1 = the bone's joint |
| `+8` f32 | height offset, raises the point by `+8 * actor[+0x44]` (y down: subtracted from y) |
| `+0xC` f32 | radius, scaled by `actor[+0x40]` (the returned value) |

(`actor + 0x40 / +0x44` look like the actor's scale; radius uses the first, height the
second.)

## What `0x1402AE330(actor, point, out)` does

1. `0x1402B4B90(actor, kind 0, any group, start = point)`: the first lock-point entry at
   index ≥ `point` whose disabled bit (`actor+0x3AC`) is clear. If none, the body entry
   (`col+5`, `ptr(col+0x14)`), and if that is missing too, `out = actor+0x10` (root/feet),
   radius 0.
2. `0x14029C990(actor, entry.code, entry.t, out)`: the anchor's world position.
   - Requires `actor+0x374` bit 0 (alive/active), else returns a constant default vector.
   - Plain bone codes (bit 15 clear): bone index = code when bit 12 or 14 is set
     (`code & 0xFFF`), else looked up through the model's bone id table
     (`0x1401D4180`: `[[[inst]] + 8] + 0x10 + code*4`, needs `actor+0x1D0` set), where
     `inst = ptr(actor+0x15C)` is the animation instance. A bone index ≤ 0 falls back to
     `actor+0x10`. Then `0x1401D41C0(inst, bone, t, out)`: parent = skeleton bone
     record `+0x2C & 0x3FF` (records 0x30 bytes at `ptr([[inst]] + 4)`). The bone's world
     point is the parent's world matrix (`ptr(inst+0x1B4)`, 0x40 per bone) applied to the
     bone's local translation (`ptr(inst+0x1B0)` + bone*0x40 + 0x30); the result is
     `parent_world_pos * (1 - t) + bone_world_pos * t`. (With no parent and t == 1 it reads
     the bone's world translation, row `+0x30` of its matrix in `+0x1B4`, directly.)
   - Special codes: `0x8003` centre of the body collision shape (`0x1402B3CB0` on the body
     entry, else `actor+0x50`); `0x8004` that centre raised by the shape's half height (`+8` of
     the shape output); `0x8006` raised by 1.5 half heights; `0x8005` the first kind-1 point (else lock point 0) recursively;
     `0x8007` lock point 0; `0x8002` / `0x8008` / others: `actor+0x50` / `+0x10` plus
     `(0, 100 t, 0)` or `(100 t, 0, 0)` turned by the actor's world matrix `+0xD0`.
3. `out.y -= entry[+8] * actor[+0x44]`; returns `entry[+0xC] * actor[+0x40]`.

Duplicates of the same body: `0x1402AE070`, and inlined in the auto-pick `0x1402AE870`.
`0x1402AE260(from_actor, &(handle, point), out_point, out_delta)` wraps it and gives the
gap from an actor to the point minus the radius (vertical in `delta.y`, horizontal in
`delta.w`); KH1's own command step uses that to approach / home on the target.

## How targets and points are picked

`0x1402AE870` (auto) and `0x1402AF3B0` (R1) walk every actor (`0x14028F990`) that is an
enemy (`(actor+0x374 & 0x60) == 0x40`) or a lockable object (`[ptr(actor+0x130)] + 4`
bit `0x8000`), passes `0x14028FEE0` (has an enabled hit part, flags) and `0x14028FE60`
(alive), then every enabled kind-0 point of it (`0x1402ADF80` iterates them; an actor with
no points yields index 0, which resolves to the body entry or the root). The best
(actor, point) by distance and facing wins. Validity of a held pair (`0x1402ADEA0`): actor
alive, not flagged `+0x374 & 0x8000` or `+0 & 0x200`, and the point not disabled in
`+0x3AC`.

So a multi-part boss modelled as **one actor** (several lock points on hand / head bones)
is locked as (boss, n): the handle is the boss and only the index says "hand". A boss
modelled as **several actors** (a limb with its own HP) is locked as (limb actor, its point):
the handle is the limb's, but its root `+0x10` is still not necessarily where the limb is
drawn (a sub-actor can sit at its parent's origin and be posed through bones), so the
lock point is the only safe aim either way.

## Why the mod pulls Sora to the body centre

- `takeover.rs` converts `0x142DB3F18` to the actor and lets KH2 home on that actor's
  `TargetInfo`, whose `pos` is the root (`actor+0x10`) and whose `radius` is the largest
  hit part. The lock-point index `0x142DB3F1C` is ignored, so with a one-actor boss
  (the likely Darkside case) every lock point aims at the root.
- The lock is also dropped (`filter(|id| targets...)`) when the locked actor is not in
  `bodies()`: its root is more than `NEAR` (1500) horizontally from Sora (KH1 measures
  1500 from the lock point minus its radius, so a huge boss's hand can be locked while its
  root is out of range), it is on Sora's team (a lockable object), `0x14028FEE0` says no
  (all its hit parts disabled), or none of its parts gives a sane shape. Then KH2 picks its
  own target, again a root.

## Recommendation for the hook

Each frame while `*0x142DB3F10 >= 3` (or `!= 0` to keep today's behaviour):

1. `h = *0x142DB3F18`, `n = *0x142DB3F1C as i32` (treat `n < 0` as 0), `a = ptr(h)`.
2. Call `0x1402AE330(a, n, &mut out: [f32; 4]) -> f32 r` (Windows x64: rcx actor, edx
   point, r8 out; returns xmm0). Use it on the game thread (the physics hook is fine).
3. KH2 world: `p = [out[0], -out[1], out[2]]`. Feed the locked target to the controller as
   `TargetInfo { id: a, pos: [p.x, p.y - r, p.z], radius: r.max(small), height: 2r }`
   (a ball of radius `r` around the point), replacing that actor's root-based entry, and do
   not require it to be in `bodies()` (or include it regardless of `NEAR`).

A pure-read alternative (no call): walk the point table as described above. Only plain
bone anchors and the `0x8003` body centre need handling for most enemies; fall back to the
call or to `actor+0x10` for other codes. The call is simpler and exactly matches the cursor.

Confidence: high for the addresses, the (handle, index) pair, the table layout and the
function's maths (read directly from the code); medium for the meaning of `actor+0x40/+0x44`
(scale) and of the kind-1/2 points; whether Darkside's hands are separate actors was not
checked (no Darkside data here). Log `n` and the computed point on a Darkside fight to
confirm.
