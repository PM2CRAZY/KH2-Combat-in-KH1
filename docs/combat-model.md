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

- The first u16 of an ATKP row is two bytes: the **minimum attacker level** and the **sub id**.
  **Checked (exe).**
- **Level variants.** Rows that share an id and sub are scanned in order while their minimum
  level is at most the attacker's level. A row with `switch` 0 always replaces the current
  pick; `switch` 1 or 2 only replaces it when `(level - min level)` is even or odd.
  `Battle::attack_for_level` implements this. **Checked (exe).**
- `power` is a percentage. **Checked.**

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

**Sora's Guard** is ATKP 162: kind guard, refact self 2, reflect range 180°.
- **Against a Shadow's swipe** (ATKP 5: kind normal, reflected motion 140): the swipe stops,
  the Shadow is repelled into motion 140, and Sora stays in his guard.
- **Against its dash** (ATKP 7: refact self 2, refact other 1): the dash carries on, Sora is
  guard-repelled and becomes immune to that dash.

## Enemies (`character_core::npc`, `kh2_data::enemy`)

Each enemy comes from its own files:
- **Moveset:** `obj/M_EX010.mset` for the Shadow. Enemy movesets index motions by slot.
- **Skeleton and collision shapes:** its MDLX.
- **Stats:** ENMP / LVPM.
- **Weight:** objentry; read, not used yet.

**Hurt shapes** are its type-2 shapes. For the Shadow these are three spheres: bone 2 r40,
bone 28 r45 and bone 31 r40.

**Attacks** are its motions' hitbox windows, with their ATKP rows at the enemy's level:
- **201 swipe:** ATKP 5, group 1 (a claw)
- **202:** ATKP 1313 / 1314
- **203 dash:** a guard window (ATKP 405), then ATKP 7, which launches
- **204:** ATKP 8, which launches

**When an enemy attacks is ours.** In the game, each enemy runs a script (BDX) that picks
its moves. The playground's Shadows walk up and swipe when close, sometimes dash from
300–550 away, and wait 1.5–3 s between attacks. Damage to Sora goes through
`combat-core`, and his reaction uses the same routine as above.

## Not modelled yet

- Enemy AI (BDX scripts) and revenge limits per enemy. Enemy weight and per-enemy
  resistance to being pushed.
- Reaction 5 (a push away from the attack's position) and the alternate-part flinches (11 /
  13).
- How long Sora is locked in a damage motion. The playground reads the "allow reaction
  command" window (frame 8): **Hypothesis**.
- Multiplayer/co-op damage scaling (a global mode flag divides or multiplies damage).
- Drive forms, limits and summons beyond reading their tables.
