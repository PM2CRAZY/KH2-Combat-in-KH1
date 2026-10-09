# Drive Form model

How Drive Forms work in this project. The rules are in `character_core::drive`; the
playground's form change is in `kh2-bevy/src/drive.rs`.

Confidence labels:
- **Checked (exe)**: read from the executable's code.
- **Data**: read from your install.
- **Ours**: our choice.

## The forms are separate player objects (Checked, exe + Data)

Each form is its own object in `00objentry.bin`, with:
- its own model and motion set (`obj/P_EX100_<suffix>`)
- a moveset id, which picks:
  - its `prty`/`plyr` records (speeds, jump, auto-target range)
  - its PTYA combo table
  - its weapon motion sets in 03system `wmst` (row `moveset × 2 + hand`)

| Form | id | Object | Moveset | Weapons (wmst) | Run speed |
|---|---|---|---|---|---|
| Valor | 1 | P_EX100_BTLF | 2 | W_EX010_BTLF_R / _L | 12 |
| Wisdom | 2 | P_EX100_MAGF | 3 | W_EX010_MAGF | 12 |
| Limit | 3 | P_EX100_KH1F | 0x42 | W_EX010_KH1F | 8 |
| Master | 4 | P_EX100_TRIF | 4 | W_EX010_TRIF_R / _L | 10 |
| Final | 5 | P_EX100_ULTF | 5 | W_EX010_ULTF_R / _L | 16 |
| Anti | 6 | P_EX100_HTLF | 6 | none | 16 |

**Weapons.**
- Both hands' weapons attach to the bones in `sklt` (right 178, left 86).
- Their motions move them, so Master's and Final's floating keyblades come out of the data.
- Attack windows name the weapon they belong to: body, first weapon or second weapon.

## Drive gauge (Checked, exe)

- **Bars.** One bar is 100 points, up to the maximum (9 at most).
  - When full, the gauge holds at 100 points in the last bar.
  - Paying a cost keeps a part-filled bar unless the gauge was full or short.
- **Sora's hits** add the ATKP row's drive value (byte 0x29):
  - ×0.75 against normal enemies
  - ×2 while MP charges (+0.2 per Drive Boost)
  - truncated, so a value-1 hit on a normal enemy adds nothing
- **Ours:** spell hits add drive too (the game's handler runs for Sora's attacks; that it
  covers spells is inferred).
- **Not modelled:** orbs (the rule is in `Drive::orb`), Damage Drive, items.

## Driving (Checked, exe)

1. **Costs** come from 03system `cmd` byte 0x1c: Valor 3, Wisdom 3, Limit 4, Master 4,
   Final 5, Anti 9.
   - A command is unavailable when the gauge is short, or Sora is already in a form or
     changing.
2. **The Anti roll.**
   - A counter rises by 1 with every drive that isn't Final.
   - The Anti chance is 0% below 5, 10% from 5, and 25% from 10 (fmlv Anti rows).
   - A Final drive lowers the counter by 10; becoming Anti lowers it by 6.
   - Anti happens only in battle. **Ours:** "in battle" means an enemy is up.
3. **The cost of the resulting form is paid.** Anti costs 9, which empties the gauge.
4. **Sora's Drive motion** (0x34, 160 frames) plays, invulnerable. The screen tints grey:
   12 frames in, then 15 frames out after the swap.
5. **The swap.** The form object replaces Sora, keeping his position, facing and lock-on.
   - It plays its arrival motion (0x35).
   - HP and MP go to full, and MP Charge is cancelled.
6. **Form gauge** = `(form level + 3) × 600` frames. Anti uses 6 bars.

## In a form (Checked, exe)

- **Draining.** The form gauge drains 1 per frame in battle, 0.5 outside (×0.8 per Form
  Boost).
- **Hits drain it too:** each of the form's hits takes `15 × drive value` frames, ×0.2 on
  normal enemies.
- **Magic.**
  - The `sub` gets +10 in Wisdom, +20 in Master and +30 in Final, which picks the spells'
    form variants and `magi` records.
  - Valor, Limit and Anti can't cast (the command's form mask).
- **Abilities.**
  - Sora's support abilities carry over (Combo Plus and the like). His action and growth
    abilities don't.
  - Each form brings its growth ability at its level: Valor High Jump, Wisdom Quick Run,
    Limit Dodge Roll, Master Aerial Dodge, Final Glide. Form Lv1/3/5/7 give LV1/2/3/MAX.
  - **Ours:** every row of the form's combo table is unlocked, and the form guards if its
    motion set has the guard motion.
- **Anti** takes 1.5× damage.
- **Reverting** happens when the gauge empties, on the Revert command (the Drive row reads
  Revert in a form), or on a KO.
  - Sora comes back at once and plays motion 4.
  - There is no refill and no cooldown.
- **Form EXP** (fmlv table, levels capped at 7):
  - Valor: +1 per hit
  - Wisdom: +1 per Heartless defeated (status ids 1000..2011, not 1047..1059)
  - Final: +1 per Nobody defeated (3000..4014)
  - Master's rule (drive orbs) and Limit's (limit finishers) aren't modelled.

## Wisdom Form's shots (Checked, exe + Data)

Wisdom's attacks have no hitbox windows. Their motions fire frame trigger 1 with an effect id
(25..43).
- **Where the logic lives.** Wisdom's model file has a script (`wisd`). The game runs its
  trigger 0x10 with the object, the id, and the effect started from the form's effect archive
  (`obj/P_EX100_MAGF.a.us`).
- **What the script does.** It fires one homing shot per id:
  - ATKP 859, sub 0, 3 or 5
  - a sphere of radius 50
  - speed 20 rising by 2 per frame to 80
  - turns 1.8° per frame toward Sora's target, and stops homing within 100
  - lives 24 frames plus an 8-frame fade
  - stops on a hit or a wall
- **How we run it.** `kh2-magic` runs this script like a spell (`Magic::object_effect`). One
  trap was added for it: `obj_effect_start`.

## Not modelled

- Form keyblade stats (forms deal Sora's base damage).
- Limit Form's limit commands, summons, party members.
- The form change's full-screen effect.
- Drive Recovery items.
