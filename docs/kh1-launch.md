# KH1: how an enemy reacts to a launching hit, and how to launch one our way

Research notes (Ghidra, Epic KH1 Final Mix PC, image base `0x140000000`, addresses absolute;
the hook adds `base + (addr - 0x140000000)`). Read statically, not yet watched in play.
KH1 world space, **y down** (up is negative y), units about 1 cm, time in 60 Hz frames
(the frame step `0x1423405DC` is 1 at 60 fps, 2 at 30). "Handle" = KH1's 32-bit compressed
pointer (`0x14038B0B0` decodes). Confidence: **[high]** read plainly from the code,
**[med]** inferred from how the code uses it, **[low]** a guess.

## Summary

- The hit record is built by `0x1402BD480` (called from `0x1402BD180`, called from the
  contact `0x1402B1ED0`); the contact then **overwrites the direction**. The record has **no
  speed and no vertical component**: only a horizontal unit direction (y 0, w 1). [high]
- The reaction kind on an **enemy** comes from the attack data's word at **`+0xC`** (not `+8`;
  `+8` is the kind used against the player's side). Nothing else decides 4 vs 5. [high]
- Every ordinary enemy (actor kind byte `+6` = 3) runs the think `0x1402B50C0`, whose hit pass
  `0x1402B6C60` sends a kind-5 record to **`0x1402B6390` (launch)**. The launch strength is
  **global**, from `btltbl.bin`, scaled by one per-enemy stat: horizontal speed 20..35
  units/frame along the record direction, launch height = speed x 1.35 (27..47 units).
  No per-hit up speed or angle exists anywhere (record, attack object or attack data). [high]
- So a per-hit vertical launch needs a hook: **hook `0x1402B6390`**, let it run (it picks the
  motion, facing, air state and down time), then overwrite the actor's jump fields
  (`+0x3B0` / `+0x3B8` / `+0x3BC`) and its horizontal knock velocity (`+0x3C0`). Forcing the
  launch itself is just record `+0xC` low 5 bits = 5. Details in "Recommendation". [high for
  the fields, med that nothing else rewrites them mid-flight]

## 1. The hit record

### Who writes what

1. Contact `0x1402B1ED0 (attack entry, body entry)` calls `0x1402BD180 (attack, target)`
   (hittable check `0x14028FEE0`, then the record, then MP for the attacker from data `+0x2C`
   and the attack's hit script).
2. `0x1402BD480 (attack, hit-once bit, target)` takes a 0x40-byte record (pool `0x142DE6810`,
   64 entries; a second pool `0x142DE7810`, 128 entries, for deferred records) and fills it:
   - `+0xC` = the attack object's attribute word `+0x30` (low 5 bits kind = data `+8`,
     bits 6..9 element, bit 10 free-standing), **then, when the target's team
     (`+0x374` bits 5..6) is 2 (the enemy side), the low 5 bits are replaced by attack data
     `+0xC`**. [high] (Sora is team 1: `0x1402B59F0` tests `& 0x60 == 0x20` for the player
     side.)
   - deferred case (target stopped, actor `+0x88` bit 0, or the head of its queue still has a
     delay): `+0xC` also gets `0x800` (deferred), bit 16 (target stopped) and a delay in
     bits 12..15; such records are handled later by `0x1402B7190`, not the switch below.
   - `+8` damage from `0x1402BD8D0` (it may set `+0xC` bit 5 / record flags, e.g. no effect).
   - `+4` attacker handle (attack `+0x34`), `+0x2C` attack data handle (attack `+0x5C`),
     `+0x28` hit-once bit, `+0x20` = -1, `+0x34` / `+0x38` effect handle / effect id.
   - `+0` = 1 (in use) with bits 2..3 = the attack's team (flags bits 12..13): the logged
     "5" is in use + team 1. Later bits: `0x2` reaction done, `0x20` no effect.
   - `+0x10` direction: target minus attack centre, flattened and normalised, if the target
     is farther than its body radius times a constant; otherwise target minus attacker. w = 1.
3. Back in the contact, **`+0x10` is recomputed** (so a post-`0x1402BD480` edit of the
   direction is lost; edit it after the contact returns, or in the reaction hook):
   - attack flags bit 6 (`0x40`) set: target minus the attack centre, flattened;
   - attack with a host (ours: bone 0x30, host Sora): target minus Sora, flattened, then its
     yaw **clamped to within +-45 degrees** (`0x1403EEC70` / `0x1403F0534`) of Sora's yaw
     (actor `+0x3C`), rewritten as (sin, 0, cos);
   - free-standing attack: its own travel direction (current minus previous centre).
   - y set to 0 and w to 1 in every case. `+0x20` = the body part's first byte.
   - then `0x1402BCF20` tells the target's script (actors with `+0x374` bit `0x10000`) about
     the hit (event 2) with the contact point.

Record layout (0x40): `+0` flags, `+4` attacker, `+8` damage, `+0xC` kind / element /
deferral / delay, `+0x10` direction vec4, `+0x20` body part, `+0x24` the foe's hit serial (`+0x3E8`, s179), `+0x28`
hit bit, `+0x2C` attack data, `+0x30` next record, `+0x34` / `+0x38` effect,
`+0x3C` unused (0). [high for the writes; "unused" means only that `0x1402BD480` leaves them
zero]

### Attack data row (0x70 bytes in `btltbl.bin`, loaded at `0x142D23740`)

`+0` power source; `+4` element; **`+8` reaction kind against the player's side**;
**`+0xC` reaction kind against enemies**; `+0x10` bone (negative: free-standing); `+0x14`
power; `+0x18` / `+0x1C` critical; `+0x20` / `+0x24` damage min / max; `+0x28` effect;
`+0x2C` MP for the attacker; `+0x34` flag bits; `+0x38` clash type (bit 16 spent); `+0x3C`
priority; `+0x40` motion request an enemy plays for a kind 6 / 7 clash record; `+0x54` /
`+0x58` offsets; `+0x5C` radius; `+0x60` / `+0x64` directional arc; `+0x68` MP factor.
**No knock-back speed, height or angle field**: the launch code never reads the data. [high
for `+8` / `+0xC` / `+0x40` and the absence in the launch path; the rest from
`kh1-reflect.md`]

Sora's rows (from the stage logs): swings `0x4B78`, `0x4BE8`, `0x4CC8` have 3 / 3 (flinch);
finishers `0x4C58`, `0x4D38`, `0x5048` have 5 / 5 (launch); no row of his is a 4.

### Kind numbers on an enemy (`0x1402B6C60`)

Per pass the enemy reacts only to a kind higher than any already handled this pass; records
with `+0` `0x22` (done / no effect) or refused by `0x1402B59F0` are skipped. 2 `0x1402B5CC0`;
3 flinch `0x1402B7080` (in the air a small knock `0x1402B69E0`; while lying down
`0x1402B5CC0`); **4 knock-back `0x1402B5630`**; **5 launch `0x1402B6390`** (if the actor is
in a "class 2" movement state, `0x1402AF950`, it is treated as a 4); 6 / 7 clash; 8
`0x1402B5C40`; 0 / 1 / others `0x1402BD440`. [high]

## 2. How an enemy consumes a kind-5 record

Chain: the actor task `0x140295E50` (zeroes the move speed `+0x2C` each frame, then the think)
-> think `0x1402B50C0` (chosen by `0x1402957F0` for actor kind 3) -> hit pass `0x1402B6C60`
-> damage `0x1402B59F0` -> reaction. Records come off the queue at `+0x3EC` through
`0x1402BCFB0`. [high]

### `0x1402B6390 (actor, record)`: the launch [high]

- Motion base 0x52 on the ground, 0x5C if already airborne (`0x1402AF980`). If the enemy has
  no such motion (`0x14029CE80`: motion table `+0x1D8` entry is 0xFFF) it falls back to the
  kind-4 code. The base goes to `+0x3EA`.
- Facing / motion: dot of the actor's facing (`+0x30`) and the record direction: >= 0 plays
  base+5 then base+6 and faces along the direction (hit from behind); < 0 plays base+0 then
  base+1 and faces against it.
- Speed `s = G[+0x14] + (G[+0x16] - G[+0x14]) * stats[+0xC4]`, with `G` = `*0x142D23730` =
  `btltbl.bin + 0x58` and stats = the actor's `+0x6C` block. The shipped file has u16
  `+0x14` = 20, `+0x16` = 35, float `+0x18` = 1.35. So s is 20..35, by a per-enemy
  0..1 stat (probably weight / blow resistance [med]).
- Height: `0x1402AD7D0(actor, s * G[+0x18])`. That function takes a **height** h and sets up
  the air arc: `+0x3B0` time 0, `+0x3B4` start y (= `+0x14`), `+0x3B8` rise time
  `T = sqrt(2h / g)`, `+0x3BC` peak offset `-h` (up), `+0x3D0` 0, movement state 2 (air,
  `0x1402AF7D0`), flag `0x10000` cleared. g = 0.48469388 units/frame^2 (`0x1403EAB54`).
- Horizontal: `+0x20` = record direction, then `+0x3C0` = direction x s (w = s): the knock
  velocity, units per frame.
- Down time `+0x3E4` = random 0..30 frames (unless `+0x88` bit 1). Record marked done (`|2`).

### The blow's sideways sign (Aerial Finish)

**Checked (decomp).** KH2's reaction routine's blow case (Steam KH2 `0x1403D2FE0`, group 4 from
`0x1403CE980`) takes its two starting speeds from `0x1403CEA00` (the ATKP's `+0xC` up and `+0xE`
sideways, sstm's 24 / 40 only when both are 0), then lays the sideways one along the hit's direction,
flattened and normalised, **as a signed number**: `+0x20` = direction × it, `+0x24` = −up. The facing
and the front / back motion (0x10 / 0x12) come from the direction alone. So a negative sideways
number sends the foe back toward the attacker. **Checked (files)** (`00battle.bin` ATKP): Aerial
Finish's spins 915 / 916 / 917 are reaction 9 (group 4) with up 10 / 5 / 12 and sideways −1 / −1 /
−10, its last hit 927 reaction 11, up 3, sideways 45; Explosion's ticks 911 are 3 / −1. So in KH2 the
spins lift the foe a little and pull it in by 1 / 1 / 10 a frame (braked as every blow, floored at a
tenth), and only the last hit throws it. Ours took the number's size (`Launch::kh2` had `abs()`), so
917 threw the foe 10 a frame away; now the sign is kept (`+0x3C0`'s w stays the size).

### KH1's scripted launch

**Checked (decomp + exe).** Actors of kinds 4 / 5 / 9 / 10 (the enemies in the t515 hit dumps of thread
kh2-effects-in-kh1s-own-effect-system are kind 4, object 200; the kind of this tester's object 321
isn't in his log: Assumption that it is one too) don't run KH1's enemy hit pass (`0x1402B8E10`, from
the kind-3 think `0x1402B7260`): their think is `0x1402C2C90`, which hands each hit record to the
actor's script (event 0, `0x1402BCD60`; without a script `0x1401A1A10`). The script picks the
reaction through KH1's script calls, whose table entries point at small wrappers: `0x1402BBD10`
flinch (`0x1402B8460`), `0x1402BBD80` push (`0x1402B7E60`), `0x1402BBF40` knock-back
(`0x1402B77D0`), `0x1402BBEC0` launch (`0x1402B8530`) and `0x1402BBC80`, **the scripted launch
`0x1402B79F0 (actor, record, motion base, height, speed, down time)`** (height in XMM3, speed and
down time on the stack; pointer at `0x1405293E0` in the script-call table, Checked (exe)). That is
KH1's launch with the script's own numbers: motion base (0x52 / 0x5C when 0), `+0x3EA`, facing as
`0x1402B8530`, the height through `0x1402AF960`, `+0x3C0` = direction × speed, `+0x3E4` = down time;
without the motion, KH1's knock-back `0x1402B77D0`. It wasn't hooked, so a KH2 blow on such an enemy
flew by its script's height and speed: the tester's log has 16 "KH2 launch asked" lines and no line
at all from the launch hook (`0x1402B8530`), the same in the t515 log of another thread (17 / 0).
Now hooked (Steam only, prologue `48 89 5C 24 10 48 89 74 24 18 57 48 83 EC 40`, Checked (exe)): KH1's
runs, then for a fresh KH2 launch's record that did launch (its `+0x3EA` the base it used, state 2)
the arc and flight are KH2's blow, as after `0x1402B8530`. **Assumption:** these scripts pick the
scripted launch for a kind-5 record (inferred from the missing lines and the flinch pushes, which
do reach `0x1402B7E60`, logged); the next log says ("KH1's scripted launch on ...").
**Assumption:** such an enemy's air state step is KH1's state 2 like the others' (so
`knockback::air_step` flies it).

### The flight (movement state 2's update `0x1402AD510`, registered by `0x1402AD150`) [high]

- Vertical: while `t < T`, `y = start + peak * sin(pi/2 * sin(pi/2 * t/T))` (an ease-out to the
  apex); after, `y = start + peak + g/2 * (t - T)^2`, the fall step capped at 20 units per
  frame. t advances by the frame step.
- Horizontal (when the move speed `+0x2C` is 0, which the actor task ensures [med]):
  `+0x3C0` is multiplied every frame by `stats[+0x18]` (its square root at 60 fps) and moved
  by. So the push decays; total slide is about `v * k / (1 - k)`, k the per-frame factor.
- Landing: `0x1402B50C0` sees the landing flag and plays base+2/+3 (or +7/+8), lies for
  `+0x3E4` frames (motions with `(motion - base) % 5 == 3`), then gets up (+4 / +9).

### Kind 4 for comparison (`0x1402B5630`)

Grounded: `0x1402B6B10`: motion 0x4C / 0x4D, no velocity (`+0x3CC` 0); without 0x4C a launch
(if 0x52) or a flinch. Airborne: motion 0x50 / 0x51 with `+0x3C0` = direction x `G[+0x3C]`
(16) and a 20-unit hop, or 0x5C (launch), or `0x1402B69E0` (0x4E / 0x4F, speed
`G[+0x38]` (4) x stats `+0xC4`, 20-unit hop). So 4 vs 5 is purely the data's `+0xC` (or
whatever we write into the record). [high]

## 3. Recommendation

### Forcing the launch reaction

After the record exists, set record `+0xC` low 5 bits to 5 (keep the other bits). The
existing `probe::hit` / `combat::scale_record` post-hook on `0x1402BD180` sees the record
pointer; the contact does not touch `+0xC` afterwards. Or land with a kind-5 row
(`0x4C58`). [high]

### Setting the speeds: hook `0x1402B6390 (actor rcx, record rdx)`, void

Prologue `48 89 5C 24 10 48 89 74 24 18 57 48 83 EC 60` (instruction boundaries at 5, 10, 11,
15). It is also reached from the kind-4 code and from a script syscall (`0x1402B9D10`), so
act only on records we marked: in `scale_record` remember `record pointer -> wanted launch`
(drop entries a few frames old; the pool reuses slots).

In the hook:
1. Before calling KH1, optionally write record `+0x10` = the wanted horizontal direction
   (unit, y 0, w 1). It only decides facing and the front / back motion; for a straight-up
   launch keep KH1's (Sora to the enemy).
2. Call KH1's function.
3. If it launched (actor `+0x3EA` is 0x52 or 0x5C and movement state `+0x70` is 2), overwrite:
   - `+0x3B0` = 0, `+0x3B8` = rise time T (frames), `+0x3BC` = -h (h > 0 up), leave `+0x3B4`;
   - `+0x3C0` = horizontal velocity vec4 (x, 0, z, w = speed), units per 60 Hz frame;
   - optionally `+0x3E4` = lying-down frames.

For KH2 values: up speed `vy` (units/frame) gives `h = vy^2 / (2 * 0.48469)` and `T = vy /
0.48469`, which keeps KH1's fall (gravity g) continuous at the apex. To keep KH2's own
apex height and time instead, write h and T directly; only the fall uses KH1's g. (Convert
KH2 units / per-second speeds to KH1 cm / per 60 Hz frame first.)

- (a) **Upper Slash (straight up)**: kind 5, `+0x3C0` = (0, 0, 0, 0), h / T from the wanted up
  speed. The enemy plays its normal launch animation in place.
- (b) **Normal KH2 launch**: kind 5, `+0x3C0` = direction x horizontal speed, h / T as above.
  KH1 decays `+0x3C0` each frame by the enemy's `stats[+0x18]`. For a constant horizontal
  speed through the arc, rewrite `+0x3C0` each frame from an existing per-frame hook while
  `+0x3EA` is 0x52 / 0x5C and `+0x70` is 2. Or launch with `v0 = D * (1 - k) / k` for a
  slide of D.

Not recommended: patching `G` (`0x142D23798 + 0x14 / 0x16 / 0x18`) works without a hook but
is global, ties height to horizontal speed (no vertical-only launch) and is read when the
enemy processes the record, not at contact.

### Who ignores it

- Actors whose think is not `0x1402B50C0`: object kinds 4 / 5 / 9 / 10 (records only go to
  their script, `0x1402C0AE0`), and special object types (`0x1402957F0` cases 7..0x13, e.g.
  `0x1402C0ED0`, `0x1402C4440` / `0x1402C4730`, which react natively in their own way). [high]
- Enemies without motion 0x52 (0x5C in the air): treated as a kind 4 (knock-back, flinch or
  nothing), so the hook's "it launched" test fails and nothing should be written. Many
  bosses and big Heartless are probably here [low: needs a probe].
- Actors in a class-2 movement state (`0x1402AF950`, e.g. state 0x17): kind 5 becomes 4.
- No reaction at all: `0x1402B59F0` refuses when actor flags `& 0x1000088`, `+0x374` bit
  `0x80000`, or the global flags `0x142867CD8` bits 20 / 26; not hittable (`0x14028FEE0`).
- Stopped enemies: the record is deferred (`0x800`) and replayed by `0x1402B7190`, which does
  its own thing; the hook will not see a launch. [high]
- Script-driven enemies (`+0x374` bit `0x10000`): damage goes to the script, but the reaction
  still runs natively through `0x1402B6390`. [med]

## Probe to confirm

Hook `0x1402B6390`; for each call log the record (`+0xC`, `+0x10`), the actor's object id,
stats `+0xC4` / `+0x18`, and after KH1 returns `+0x3EA`, `+0x70`, `+0x3B0..+0x3BC`,
`+0x3C0`, `+0x3E4`. Expect Shadows / Soldiers: base 0x52, state 2, `+0x3BC` = -(1.35 x s) and
`|+0x3C0|` = s. Then try (a) on a Soldier and watch `+0x14` rise by h.

## 4. KH2's knock-back on enemies (testing branch, thread kh2-knockback-on-enemies)

Steam addresses. KH1's enemy reactions stay the only system; KH2's numbers go into them for
hits that KH2's moves land (`crates/kh1-hook/src/win/knockback.rs`). Hits of KH1's own moves
keep KH1's numbers.

### KH2's reaction routine (Steam KH2)

**Checked (decomp).** `0x1403D2FE0` (Sora and every enemy) sorts the ATKP reaction into a group
(`0x1403CE980`): push only (1, 0xC), flinch plus a push (3, 6, 8, 10, 0xD, 0x10, 0x12), blow
(4, 7, 9, 0xB, 0xF, 0x11), Magnet's pull (5).
- **Blow:** `0x1403CEA00` gives the starting speeds: up = the ATKP's first knockback number,
  sideways = its second; only when both are 0, sstm's defaults (`+0x24` 24 up, `+0x28` 40
  sideways). The blow's velocity provider (`0x140410330`) multiplies the sideways speed by sstm
  `+0x3C` (0.8) each physics tick; the rise ×0.8, gravity 20/49, fall capped at 16 (sstm,
  `character_core::reaction::Blow`, a 30 Hz tick as everywhere in KH2's physics).
- **Push:** `0x1403D4960` starts it at the distance × 0.1 (distance from `0x1403CEAB0`: the ATKP's
  first knockback number, or sstm's 200 when it is 32767); `0x1403D4570` each update stops it
  below 1, else moves by speed × the frame step (`0x140717480`) and then speed × 0.9^step.
- **Air flinch:** `0x1403D4800`: a 50 hop, drifting at the push distance × sstm `+0x38` (0.048).
- No per-enemy factor in this routine.

### KH1's per-enemy weight

**Checked (decomp).** The actor's stats block (`+0x6C`) `+0xC4` scales KH1's reactions:
- launch `0x1402B8530`: speed `s = 20 + (35 - 20) × w` (btltbl reaction block `*0x142D22D30`
  u16 `+0x14` / `+0x16`), height `1.35 s` (`+0x18`);
- flinch push `0x1402B7E60`: distance btltbl `+0x2C` (80) × w, given to KH1's push
  `0x1402C7300` (direction `+0x3F0`, speed `+0x3FC` = distance × 0.22314355, `+0x88` bit 0x40);
- air flinch `0x1402B8B90`: a hop of 20 (`0x1402AF960`), `+0x3C0` = direction × btltbl `+0x38`
  (4) × w.

The Enemy Weight setting (Config > Combat > Tweaks, ini `enemy_weight`, on by default) multiplies
KH2's numbers by it: pushes and drifts by w, launches (both speeds) by `s(w) / 35`.
**Assumption:** w = 1 is KH1's full weight, so such an enemy takes KH2's numbers as they are.
Each enemy's w is logged once ("enemy weight: ...").

### Launches

Every KH2 blow is a launch: the record gets kind 5 (KH1's launch), and after KH1's launch ran
(`0x1402B8530`, hooked through Epic `0x1402B6390`, mapped) and the enemy did launch (motion
0x52 / 0x5C, state 2), the arc is KH2's blow flown to its apex (`+0x3B8` frames, `+0x3BC`
height) and `+0x3C0` KH2's sideways speed. Gone: the "holding blow" rule (small blows flinched
in place and pulled beside Sora for 24 frames) and Explosion's last blast's override (32 up /
60 sideways): its ATKP 912 is 15 up / 30 sideways, as KH2. A KH2 flinch on one of our rows whose
data launches gets KH1's flinch (kind 3). Enemies KH1 can't launch (no motion 0x52 / 0x5C) and
the big and fixed ones keep KH1's own reaction.

### The flight

A KH2-launched enemy flies by KH2's blow in KH1's air state: after KH1's state-2 step
(`0x1402AD510` Epic / `0x1402AF6A0` Steam, through KH1's state table, wrapped by
`shared::physics::air_step` for every actor) `knockback::air_step` replaces the step's
displacement with KH2's: one `Blow` tick (30 Hz: sideways ×0.8, floored at a tenth of the start;
rise ×0.8; gravity 20/49; fall capped at 16) every second 60 Hz frame, half of it each frame
(KH1's frame scale 2 at 30 fps: one tick a game frame). `+0x3C0` holds KH2's sideways speed and
`+0x3C4` the frame's drop. KH1's mover, collision, landing (motions +2 / +3), lying and get-up
stay KH1's. The flight ends when the enemy leaves state 2, when KH1 writes another arc (a new
reaction of its own: its launch, air flinch, knock-back), or after 600 frames. **Checked
(code)**; the 30 Hz tick is the project's reading of KH2's physics (`docs/combat-model.md`),
not re-checked frame by frame here.

### Flinch pushes

KH1's enemy flinch push `0x1402B7E60 (actor, record)` (Steam, prologue `48 89 5C 24 18 57 48 83
EC 40`, Checked (exe)) is hooked. KH1's runs first (its motion, its push started through
`0x1402C7300`, the record marked done); for a record a KH2 hit made (noted per record in
`combat::scale_record`, fresh for 250 ms), KH1's push speed `+0x3FC` is set to KH2's distance ×
0.1 (× the enemy's weight with Enemy Weight on), and before each of the enemy's physics steps
(`takeover::physics`, Steam `0x14029A3C0`) KH2's speed for the frame goes into KH1's field, so
KH1's own push step `0x1402C7070` moves it (with its collision), KH2's ×0.9 a frame (raised to
the frame step) kept by us. Ends below 1, as both games do. The distance: the ATKP's first
knock-back number, sstm's 200 when it has none, halved for reactions 0xC / 0xD; a plain flinch
(group 2) none. Left to KH1: an enemy whose weight is 0 (KH1 never pushes it), a KH2 blow on an
enemy KH1 can't launch (KH1's fallback), and any push KH1 starts itself meanwhile. **Checked
(decomp)** for both games' push rules; the hand-over between the two is **Checked (code)**.
Spells' hits (`Strikes::spell_strike`) are not included: they keep KH1's rows' reactions.

### Air flinches

KH1's enemy air flinch `0x1402B8B90 (actor, record, front)` (Steam; the third argument a float in
XMM2, its sign the front / back motion 0x4E / 0x4F; prologue `48 89 5C 24 18 48 89 74 24 20 57 48
83 EC 60`, Checked (exe)) is hooked. KH1's runs first (its motion, its hop of 20, its drift); for
a KH2 hit's record, KH1's arc fields become KH2's hop (`+0x3B0` 0, `+0x3B4` its height, `+0x3B8`
the 50 hop's rise time under KH2's gravity, `+0x3BC` −50), `+0x3C0` the drift, and the flight
(`knockback::air_step`, as for launches) is KH2's: a 50 hop on KH2's jump curve and gravity
(20/49, fall capped at 16), drifting at a constant push distance × 0.048 (× the enemy's weight with
Enemy Weight on) along the record's direction (Steam KH2 `0x1403D4800`: 50.0 and sstm `+0x38`,
Checked (decomp); the constant drift is `character_core::npc`'s reading of KH2's air flinch,
`docs/combat-model.md`). Not done: KH2's turn of the flinched body (`0x1403D3840` faces it along
the push, negated from the front); KH1's air flinch keeps its facing.
