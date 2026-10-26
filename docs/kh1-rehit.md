# KH1: how one attack hits an actor again

Found in Ghidra (the user's Epic exe) while looking at the user's report that Sephiroth's flame
pillar killed Sora "instantly", even with Second Chance-style protection. Confidence: medium
(the static reading is solid; the pillar's own interval hasn't been measured in game).

## Once per attack, re-armed on an interval

- Each attack has a bit. Queuing a hit record on a target (`0x1402BD480(attack, bit, target)`)
  does nothing if the target's `+0x3E0` already has the bit, and sets the bit otherwise. So an
  attack hits each actor once.
- An attack with an interval (`+0x50`, a float, > 0) adds the frame time to `+0xA4` each frame
  (`0x1402B11F0`). Each time `+0xA4` reaches the interval, the attack's bit (`+0x54`) is cleared
  from every actor's `+0x3E0` and from every hitbox's `+0x9C` mask, and the attack can hit again.
  This is how a lasting attack (a pillar, a stream) hits more than once.
- `0x1402B22D0` masks every actor's `+0x3E0` with `0x142DB8110` (the bits still in use).
- The hit check `0x1402B1A50` also skips a hitbox pair when the target's hurtbox has flag
  `0x200000` or `0x400` set, or when the target actor's `+0x3E0` has the attack's bit (`[0x15]`).

## Taking hits

- KH1's hit pass `0x140299A40` ignores a record when the record has `0x20` (no effect), the
  actor's flags `+0` has `0x200` (set while carried or grabbed: `0x1402AAC90`, `0x1402C3BA0`), or
  the stats' `+0` has `4` (dying).
- Second Chance (ability `0x40` in the equipped list at `0x1428695A0 + 0x44`): with HP full and
  the damage at least the HP, the damage becomes HP − 1. Only the first hit from full HP is
  spared.

## What the mod does

KH1's knock-back reactions move Sora out of a lasting attack. KH2's reactions, played in their
place (`hurt.rs`), had no invulnerable frames (Sora's KH2 hurt and blow clips, motions 10–19,
carry no hurtbox-off ranges). So every re-arm of the pillar landed and its damage was taken.
KH2's engine protects a downed Sora, and `hurt.rs` now does the same:
- while KH2 has him in a blow (launched, landed, getting up), no hit lands;
- while he flinches, the same attacker's hits within 20 frames at 60 Hz (a third of a second, at any frame rate: `takeover::TICKS`) of the last one don't land.

## A KH2 swing's second hit window (t-rehit)

- **KH1's re-arm, Steam addresses** (Checked (decomp)): `0x1402B3380 (attack)` is the interval
  re-arm above (the frame step `0x14233FBDC` added to `+0xA4`; once it reaches `+0x50` it clears
  the attack's `+0x54` from every actor's `+0x3E0`, iterating with `0x140291B20`, and from every
  in-use pool attack's `+0x9C`, pool `0x142DB4710` to `0x142DB770F`, 0xC0 each). KH1 calls it for
  motion-tied attacks from `0x1402B47A0`.
- **KH2 keeps hits per window** (Checked (code), the OG rewrite's `combat_core::HitRecord`: a
  hitbox's swing id is `(swing << 8) | window`, and "already hit" is kept per id). So a move with
  two hit windows on the Keyblade hits the same foe once in each. Sora's 153 (`P_EX100.mset` /
  `W_EX010.mset`, Checked (files)): ATKP 128 at frames 20..32, 147 at 38..47. 151 and 152 have one
  each (126 at 15..23, 127 at 20..32); 154 / 155 one each.
- **On our KH1 rows** one attack object covers the whole motion, so a foe hit by 153's first slash
  kept the bit and the second slash passed through it (the tester, combo row selection after m32:
  "the 2nd hit is also missing its 2nd hitbox, its just one for now"; his log: 153 on one foe hit
  with 147 only, or 128 only). Now (`kh2slots::keyblade_rearm`, each frame KH1 has Sora, before
  KH1's physics and collision) when another Keyblade window of the same KH2 motion opens, the row's
  attack objects (owner Sora, attack data one of our rows', not a body window's) are re-armed by
  calling KH1's `0x1402B3380` with their interval made due for that call, then put back
  (`kh1rows::rearm_rows`). Log: "KH2's next hit window on the Keyblade ... re-armed".
- **Assumption**: KH2's exe keeps hits per window as the OG rewrite does (not re-checked in the
  Steam KH2 decomp). Not checked in game.
