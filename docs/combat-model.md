# Combat model

This is what `combat-core` implements and where each piece comes from.
Every rule is tagged:

- **Data**: read straight from the game's tables, layout documented by OpenKH.
- **Checked**: tested against your install's tables (see the "Evidence" notes below).
- **Community**: from forum or wiki posts, not yet checked in-game.
- **Hypothesis**: our best reading, open to change. These are exposed as
  settings so they can be changed without code edits.

Nothing here was taken from decompiled code.

## Damage pipeline (`combat_core::compute_damage`)

| Step | Rule | Status |
|---|---|---|
| 1 | `base = attack stat - defender Defense`, floored at 0. Spells (Fire/Blizzard/Thunder) use Magic, everything else uses Strength | Community (formula) / Hypothesis (which elements use Magic) |
| 1a | Optional `BaseClamp` on `base` | Hypothesis (alternative reading of ENMP min/max) |
| 2 | `x power`, where `power = ATKP.power / 100` | Checked |
| 3 | `x resistance% / 100` for the attack's element | Data (ENMP weakness columns) |
| 4 | `x elemental boost` (abilities such as Fire Boost, supplied by the host) | Community |
| 5 | `x difficulty multiplier` (player dealt: 1.25 on Critical; player taken: 0.5 / 1 / 1.5 / 2) | Community / Hypothesis |
| 5a | On Critical, the attack's `critical_adjust` (normal / half / none) scales damage **the player takes** | Hypothesis |
| 6 | Clamp to the defender's `HitClamp` (fraction of max HP) | Hypothesis (default) |
| 7 | At least `min_damage` (1) unless the target is immune (0%) | Hypothesis |

Every multiplication step truncates to an integer, matching the community worked example.

## Enemy stats (`kh2_data::enemy_combatant`)

- HP = `(ENMP.health[0] * LVPM[level].hp + 99) / 100`. **Data** (OpenKH-documented formula).
- Strength and Defense = `LVPM[level].strength / defense`. **Hypothesis**, with partial
  evidence: the level-99 row's Defense is 51, the same number the community worked example
  subtracts for a level-99 Data Organization fight.
- A level of 0 in ENMP means "use the world's battle level" (BTLV). The host has to supply it.

## Evidence notes

**ATKP power is a percentage.** Across 2713 rows on your install, the most common values are
100 (537 rows), 50, 25, 150 and 200. The Thunder-element (3) spell rows use 375, which matches
the ×3.75 Thundaga multiplier from the community.

**ENMP min/max damage are probably parts-per-10000 of max HP.**
- `min_damage × base HP` comes out at about 50,000 for nearly every enemy.
- `max_damage × base HP` clusters at 150k, 200k, 300k and 625k.
- Regular enemies use `max = 10000`, which would mean "100%, no cap".

Read this way, the designers set a minimum hit of 5% and a maximum of 15–30% of a fixed HP
budget, which is a coherent design. The other reading treats them as raw bounds on
`attack - defense`. It matches a forum guess of a "~80 cap" on Data fights (their max is 79),
but it would make every Shadow (min 1000) die in one hit. Both readings are implemented; pick
one with `--clamp hp|base|off`.

**Critical adjust.** Every spell row has `critical_adjust = 1` (half). If that halved
player spells on Critical, the worked example (Thundaga vs Data Saïx, 67 damage) would come
out at 33. So we apply it only to damage the player takes.

## Community worked example

The test `forum_worked_example` encodes it: 80 Magic vs 51 Defense, Thundaga (×3.75), 50%
Thunder resistance, Critical (×1.25) gives 29 → 108 → 54 → 67. With the default HP-fraction
clamp, Data Saïx (max 79/10000 of 3400 HP) caps this hit at 26. **This disagreement is
the top item to verify in-game**; see the README.

## Not modelled yet

- Hitboxes and their timing: these live in MSET/ANB files (motion triggers), not parsed yet.
- Combo trees (PTYA for party members; Sora's combos are in his MSET).
- Reaction animations and knockback physics: the ids are passed to the host as raw numbers.
- Enemy AI (BDX scripts).
- Revenge thresholds per enemy (`revenge_limit` exists, but its source table is unknown).
- Drive forms, limits and summons beyond reading their tables.
