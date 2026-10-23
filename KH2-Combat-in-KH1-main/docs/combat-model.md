# Combat model

This is what `combat-core` implements and where each piece comes from. Every rule is tagged:

- **Data**: read straight from the game's tables (layouts documented by OpenKH).
- **Checked (exe)**: read from the damage routine of your own copy of KH2 Final Mix in Ghidra.
  Only plain-language descriptions are written here. No decompiled code or Ghidra database
  is in the repo.
- **Hypothesis**: our best reading, open to change.

## Damage pipeline (`combat_core::compute_damage`)

All rounding is **up** (ceiling). There is **no minimum of 1**: a hit on a target with 0%
weakness does 0.

| Step | Rule | Status |
|---|---|---|
| 1 | `stat` = attacker Strength or Magic, chosen by the ATKP byte at offset 5 (0 Strength, 1 Magic, other none). OpenKH calls this byte "critical adjust"; the game reads it as the stat source. | Checked (exe) |
| 2 | `base = stat - defender Defense`. If the defender is at or below 25% HP, also subtract its Defender-ability bonus. | Checked (exe) |
| 3 | Cap `base` at the lower of (attacker max strength, defender max damage), then raise it to the higher of (attacker min strength, defender min damage). Players use 1 and 99999. | Checked (exe) |
| 4 | `dmg = ceil(weakness[element] × weakness[general] × power × base / 1,000,000)`. Element is the ATKP byte at offset 9, used directly as the index into the 7 weakness bytes; the 7th byte (general) always multiplies. | Checked (exe) |
| 5 | Difficulty (Standard leaves damage unchanged): damage **the player side takes** ×50% on Beginner, ×200% on Proud and Critical; damage **the player side deals** ×150% on Beginner, ×100% on Proud, ×125% on Critical. | Checked (exe) |
| 6 | Attacker Fire/Blizzard/Thunder boost percent, for elements 1/2/3 | Checked (exe) |
| 7 | Combo Boost: `+ count × pct` on combo finishers (ATKP kind flag 1). Air Combo Boost: the same for air finishers (flag 2). Reaction Boost: `+ pct` on reaction commands (flag 4). | Checked (exe) |
| 8 | Defender at or below a low-HP threshold: × a system percentage (`LowHpGuard`) | Checked (exe) |
| — | Anti Form: damage Sora takes in Anti Form is scaled by a system rate. Not modelled. | Checked (exe), not implemented |

Cure (ATKP type 5) heals `ceil(target max HP × power × stat / 10000)`. C-Cure (type 6) heals
`ceil(power × stat / 100)`. **Checked (exe).**

## Attack rows

- The first u16 of an ATKP row is two bytes: a **hit count** threshold and the **sub id**.
  **Checked (exe).**
- **Row variants.** Rows that share an id and sub are scanned in order while their threshold
  is at most the target's **hit count** (see "Hit records" below). A row with `switch` 0
  always replaces the current pick; `switch` 1 or 2 only replaces it when
  `(count - threshold)` is even or odd. `Battle::attack_for_level` implements the scan.
  **Checked (exe).**
  - *Correction (2026-10-09):* Phase 1 read the threshold as the attacker's level. The lookup
    is called from the hit routine with the defender's consecutive-hit count. Examples:
    - Sora's ATKP 12 is 150% on the first hit and 50% on repeats.
    - Fire (ATKP 502 sub 0) hits for 50% every 18 frames; the third hit is 150% and knocks
      the target away.
    - Firaga (sub 4) alternates rows with 8- and 10-frame intervals; the fifth hit is 175%.
- `power` is a percentage. **Checked.**

## Hit records (`combat_core::HitRecord`)

**Checked (exe).** Every object keeps a record of the attacks that hit it, and every live
attack has a small id (1–255, reused once the attack ends).

**Before a hit counts:**
1. The attack's bit in the target's record must be clear. Cure kinds (ATKP kind 5 and 6)
   skip this check.
2. Team and other filters must pass.

**When it hits:**
- **Group.** The attack's group is its ATKP `combo_group` when nonzero. Otherwise it is the
  value its creator passed, or a fresh number (scripts pass −1, so spells get a fresh one).
- **Effective or not.** The hit is effective if any of these holds:
  - it is a Cure kind
  - it comes from the target's last group
  - its group is 0xff
  - the target's group timer is at or below 0
  
  An ineffective hit is still registered, flagged so it does nothing.
- **Count.** If effective, the count goes up for the same group, or the group is remembered
  and the count resets to 0.
- **Group timer.** Objects with a particular flag set the timer to 30. In this project that
  is the player.
- **Row.** The ATKP row is picked with the count (above).
- **Re-hit.** If the row has an `interval`, the attack's id goes into one of 4 re-hit slots
  with that many frames.
- **Mark.** The attack's bit is set.

**Every frame:**
- The group timer counts down. It holds while the object plays a reaction motion (10–39).
- Each re-hit slot counts down by the frame time. When one runs out, it clears that attack's
  bit, so the attack can hit again.

**When an attack ends**, its bit is cleared in every object's record. Duplicated attacks
share the original's id and don't clear it.

The playground uses this for Sora's swings, the Shadows' swings and spells.

## Enemy stats (`kh2_data::enemy_combatant`)

All **Checked (exe)**, from the enemy status initialisation:
- HP = `(ENMP base HP × LVPM hp% + 99) / 100`.
- Strength, Defense, max strength and min strength come from the LVPM row for the enemy's
  level.
- ENMP max/min damage are parts-per-10000 of max HP: `max = ceil(maxHP × max / 10000)`,
  and the same for min. They bound the **base term** (step 3), not the final damage.
- Weakness bytes are copied as-is (100 = normal).
- A level of 0 in ENMP means "use the world's battle level" (BTLV); the host supplies it.

## The community worked example, revisited

The old forum example (Thundaga vs Data Saïx on Critical, 67 damage) assumed truncation and
no clamp. In the real routine:
- rounding is up
- Critical is ×1.25 on player damage
- Data Saïx's max-damage field bounds the base term

So the example's numbers don't apply exactly. The unit test `magic_and_difficulty` uses
the real order instead.

## Hit detection (`combat_core::Shape`)

**Checked (exe).** Read from the attack update and the overlap test in your copy of KH2 Final
Mix's executable; only the rules are written here.

**Which shapes.** Every model's collision list (MDLX, OpenKH's layout) holds shapes fixed to
bones. Each entry has a group, a type, a shape kind, a bone, an offset, a lift
(`position_height`), a radius and a height.
- **An attack window** switches on its ATKP's collision group. The attack collects that
  group's shapes of type 6 (attack) and 13 (guard), in list order. Sora's swings use the
  Keyblade's list, and some moves use Sora's own.
- **Guard windows** (ATKP kind guard) work the same way: Guard uses Keyblade group 2;
  Aerial Dodge uses Sora's group 6.
- **The defender's shapes** are its type 2 (hit) entries.
- A collision group is only live while enabled. Sora's list header enables group 0 by
  default, and attack windows enable the others.

**Placing a shape.**
- The offset is transformed by the bone's matrix. Bone `0x4000` means the object root:
  feet, facing.
- Other special anchors exist: `0x4003`/`0x4004` are the right/left weapon tip, and
  `0x4001`/`0x4009` sit on the first OBJ / ITEM shape. Sora doesn't use them.
- Then the centre is raised by the lift.
- `radius` is the horizontal size. `height` is the vertical half-size: a column reaches
  `height` above and below its centre.

**The overlap test.** `d` is the gap between centres. All comparisons are strict.

| Pair | Hit when |
|---|---|
| sphere–sphere | `|d| < r₁ + r₂` |
| column–column | `|d.y| < h₁ + h₂` and horizontal `|d| < r₁ + r₂` |
| ellipsoid–ellipsoid or sphere | `(dx/R)² + (dy/H)² + (dz/R)² < 1`, with `R = r₁ + r₂`, `H = h₁ + h₂` |
| ellipsoid or sphere vs column | no hit if `|d.y| ≥ hₑ + h_c`; otherwise horizontal `|d| < r' + r_c`, where `r' = rₑ` beside the column and `rₑ·√(1 − t²)` past its end (`t = (|d.y| − h_c)/hₑ`) |
| anything with a cube | never |

**Sweep.**
- Each attack remembers its shapes' centres from the previous frame, up to 32 shapes.
- If a shape moved more than 1.5 × its radius, copies are added one radius apart. They step
  from the new centre back toward the old one until the remaining gap is at most
  1.5 × the radius.
- The first frame of an attack has no previous centres.
- At most 256 attack shapes are live at once.

**Order.** For each defender, the attack shapes are tried in order, and for each one the
defender's hit shapes in order. The first overlap registers, and the defender is done for
that frame. Already-hit and team filters apply before this.

**Also there, not modelled yet:**
- Attack shapes from different sides that overlap can clash: a 7×7 table by attack kind
  decides it.
- On the second frame and after, a ray from the old to the new position of the first shape
  is tested against the map, for wall hits.

**Playground.** The training dummies have no skeleton, so their hit shape is a column of our
own (radius 50, 110 tall) until Phase 3 reads enemies' collision lists.

## Hit reactions (`character_core::reaction`)

**Checked (exe).** Sora and enemies share one reaction routine, and the reaction itself
isn't scripted. (Enemy scripts also receive a "damaged" event, which this project doesn't
run.)

**Which reaction.** The ATKP reaction byte is folded into groups:

| Group | ATKP reaction ids | Response |
|---|---|---|
| 1 | 1, 0xc | hit-back push only |
| 2 | 2 | flinch |
| 3 | 3, 6, 8, 10, 0xd, 0x10, 0x12 | flinch + hit-back; in the air, the air flinch |
| 4 | 4, 7, 9, 0xb, 0xf, 0x11 | launch |
| 0xe | 0xe | motions 23/24 + hit-back |

**Push direction** is the attacker's facing. "From the front" means the defender faces
against it.

**Motions** (the same ids for every character):
- **Flinch:** 10 from the front, 12 from behind. 11 and 13 are alternates for some body
  parts, and a missing motion falls back to its pair.
- **Air flinch:** 14 / 15, then fall. The defender turns to face along the push and hops 50
  up, drifting at hit-back × `hit_back_to_jump`.
- **Launch:** 16 from the front (facing the attacker), 18 from behind. On landing the next
  motion (17 / 19: down and up) plays, then idle.

**Hit-back distance:**
- the ATKP's first knockback value, unless it is 32767
- otherwise `sstm` hit_back (200), or `hit_back_small` (200) for group 3
- for launches: the launch speed × `blow_to_hit_back` (8)
- halved for reactions 0xc and 0xd

The push starts at distance × 0.1 per frame, decays ×0.9 per frame, and stops below 1. That
is about 0.9 × the distance in all.

**Launch:**
- **Starting speed:** horizontal = ATKP knockback 2, up = knockback 1. When both are 0, the
  `sstm` defaults apply (40 and 24).
- **Each 30 Hz tick:**
  - horizontal speed × `blow_brake_xz` (0.8), floored at `blow_min_xz` (0.1) × the start
  - rising speed × `blow_brake_up` (0.8)
  - then gravity 20/49 is added and the fall is capped at `fall_max`

**Examples on your install:**
- Sora's first swing (ATKP 126: reaction 3, knockback 125) flinches a Shadow and slides it
  about 110 back.
- His finishers (129/130: reaction 7, knockback 3–5 / 45) launch it about 300.

## Clashes and guarding

**Checked (exe).** When an attack's shapes meet another attack's shapes, a 7×7 table by
ATKP kind decides whether they stop each other:
- normal stops normal, guard and special guard
- guard stops normal
- special guard stops normal and pierce-armor
- pierce armor is stopped only by special guard

**Angle check:** an attack with a reflect range only clashes when the other attack comes at
its owner's front within that angle (reflect angle turns the cone).

**What each side does:**
- **Both attacks** stop, unless their ATKP "refact self" is 2. Each owner becomes immune to
  the other attack.
- **Sora (party):** if his attack's "refact self" or the other's "refact other" is 2, nothing
  happens. Otherwise:
  - on the ground with either one nonzero: the guard repel (motion 42, face the clash, pushed
    back 200)
  - otherwise the wall recoil (motion 41 / 43)
  - either way the clash counts as a connected swing for the combo
- **Enemy:** unless its "refact self" is 2, it plays its ATKP "reflected motion" and is
  pushed back by "reflect hit-back" (−1 = 50).

**Pressing Guard** (the action button standing) starts combo row 11 (exe `0x1403A9020`, tried
from the action-button handler `0x1403A75A0` when not moving), whose motion is 173 (A322): its
Keyblade carries a guard window (ATKP 161: kind guard, refact self 1, so a blocked attack
starts the repel; reflect range 120°) on frames 6..37, the trail on 8..37 and a swish sound
at frame 1. Motion 42 (A321) is what plays when an attack meets the guard (the repel below),
and Counterguard is row 39 taken while 42 plays (the same handler checks the motion is 0x2A).
Checked (exe, PTYA set 1 row 11). Until stage 158 we played 42 for the press (the user: "the
animation he uses when somebody actually hits his guard").

**The repel's guard window** (motion 42) is ATKP 162: kind guard, refact self 2, reflect range 180°.
- **Against a Shadow's swipe** (ATKP 5: kind normal, reflected motion 140): the swipe stops,
  the Shadow is repelled into motion 140, and Sora stays in his guard.
- **Against its dash** (ATKP 7: refact self 2, refact other 1): the dash carries on, Sora is
  guard-repelled and becomes immune to that dash.

## Enemies (`character_core::npc`, `kh2_data::enemy`)

Each enemy comes from its own files:
- **Moveset:** `obj/M_EX010.mset` for the Shadow. Enemy movesets index motions by slot.
- **Skeleton and collision shapes:** its MDLX.
- **Stats:** ENMP / LVPM.
- **Weight:** objentry. When two bodies overlap they are pushed apart, each moving in
  inverse proportion to its weight (exe). Weight 0 means immovable.

**Hurt shapes** are its type-2 shapes. For the Shadow these are three spheres: bone 2 r40,
bone 28 r45 and bone 31 r40.

**Attacks** are its motions' hitbox windows. The ATKP row is picked by Sora's hit count when
the attack lands:
- **201 swipe:** ATKP 5, group 1 (a claw)
- **202:** ATKP 1313 / 1314
- **203 dash:** a guard window (ATKP 405), then ATKP 7, which launches
- **204:** ATKP 8, which launches

**When an enemy attacks is ours.** In the game, each enemy runs a script (BDX) that picks
its moves. The playground's Shadows walk up and swipe when close, sometimes dash from
300–550 away, and wait 1.5–3 s between attacks. Damage to Sora goes through
`combat-core`, and his reaction uses the same routine as above.

## Magnet's pull (reaction 5)

**Checked (exe).** Reaction 5 catches the target:
- **Setup:**
  - motion 20, or 21 when hit from behind
  - an orbit radius picked at random between the ATKP's knockback 1 and 2
  - a swirl speed from the ATKP's next field (signed; −3 / −6 / −12 for Magnet / Magnera /
    Magnega)
- **Each frame:** it moves toward the orbit radius around the attack's centre by up to 20,
  and sideways (a quarter turn about the vertical) by the swirl speed.
- **Release:** when the attack ends, the target falls (ours: as from a launch with no speed).

The pull itself doesn't use weight.

## Combo rows for the foes around him (Vicinity Break, Magnet Burst)

The rule that picks a combo row is in `movement-model.md` ("Combo selection"). Some rows
depend on the foes around Sora; this section covers those.

**KH2's rows (PTYA set 1, read from the install's `00battle`; Checked (data)).** Neither
ability uses a random chance. The selector has none (`score` isn't read). Both depend only
on where the foes are:
- **Vicinity Break** (ability 180) is one ground row of kind 2. It is the first ground swing
  row in the table, ahead of Flash Step, Dodge Slash and Slide Dash. It needs any foe within
  3 m, at a usual height, more than about 100° away from the direction Sora aims (the
  direction to the primary target). The aimed foe is always at 0°, so it takes a second
  foe, behind or beside him: KH2's "surrounded".
- **Magnet Burst** (ability 179) is an air finisher with two rows, ahead of the base air
  finishers and Aerial Finish:
  - kind 2: any foe within 6.5 m more than about 22° off the aim, so a group;
  - kind 1: the aimed foe alone, 3 to 6.5 m away.
  Against a single foe it therefore comes only when that foe is a few metres off.

**What went wrong (s206 to s300).** Locked on, the rows were measured against the locked
target alone. The hook takes KH1's soft target as a lock too (lock states 1, 2 and 4, the
small cursor: `kh1-lockon.md`), so this was nearly always the case.
- Vicinity Break's row only ever saw the aimed foe at 0°. It never came.
- Magnet Burst came only from its kind-1 row: one foe 3 to 6.5 m away, never a group. To
  the player this looked like a chance.

The menu ids were right all along (Ripple Drive 56 → 180, our Magnet Burst 77 → 179; tested in
`growth.rs`).

**Ours now (`character_core::combo`, `Situation::lock`):**
- Every target the host passes counts for the kind-2 rows that an ability brings
  (Vicinity Break, Magnet Burst, Explosion, Dodge Slash, Flash Step, Slide Dash, Aerial
  Spiral), locked on or not. This is KH2's rule.
- Kind-2 rows that need no ability (the base finishers and swings for a foe beside him) are
  still measured against the locked target alone. This is the user's s206 wish: the locked
  foe's Aerial Finish, not the base finisher for the one beside it.
- Locked on, every swing goes for the locked target, whichever target its row passed on.
- With no primary target (nothing locked and the host's `auto_aim` off), the rows are
  measured from the direction to the nearest target, the one KH2 would aim at, instead of
  from Sora's facing. So one foe behind him is not "surrounded".
- Only targets within a row's own range count (3 m for Vicinity Break). The hook's list
  reaches 15 m.

**Log.** `Character::take_combo_note`, written by the hook as `combo rows: ...` (at most 300
lines), covers Vicinity Break and Magnet Burst while equipped. Once per combo, and only at a
step where the row could be used (ground or air; a finisher row at the combo's end), it
says that the row was taken or why not:
- not equipped;
- the combo's step;
- per target: distance, height, or angle from the aim, with the window each needs;
- the row passed but an earlier row came first, or its motion was already playing.

## Not modelled yet

- Enemy AI (BDX scripts) and revenge limits per enemy.
- The alternate-part flinches (11 / 13).
- How long Sora is locked in a damage motion. The playground reads the "allow reaction
  command" window (frame 8): **Hypothesis**.
- Multiplayer/co-op damage scaling (a global mode flag divides or multiplies damage).
- Drive forms, limits and summons beyond reading their tables.

## Revenge value in KH1 (s179)

KH1 keeps no revenge value in its engine. A landed hit makes a record queued on the foe (`+0x3EC`,
records linked by handles at `+0x30`; the foe's hit flags `+0x3E0` and a hit serial `+0x3E8`,
copied to the record's `+0x24`). A foe with a script (`+0x374` bit `0x10000`, its machine at
`+0x144`) is told of the hit at once (event 2, `0x1402BCF20`, from the contact `0x1402B1ED0`) and
again when it takes its queue in its own update (event 0, `0x1402B59F0` from the hit pass
`0x1402B6C60`); the script applies the damage (`0x1402BCD30`) and counts hits however it does.
KH2's ATKP revenge value marks a multi-hit move's ticks (0) and its last hit (> 0); the hook lands
the ticks on scripted foes without the script hearing of them and carries their damage to the
next hit (`combat::land`).
