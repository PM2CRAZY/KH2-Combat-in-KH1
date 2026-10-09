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

## Not modelled yet

- Reaction animations and knockback physics: the ids are passed to the host as raw numbers.
- Enemy AI (BDX scripts) and revenge limits per enemy.
- Multiplayer/co-op damage scaling (a global mode flag divides or multiplies damage).
- Drive forms, limits and summons beyond reading their tables.
