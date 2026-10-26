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
- **Assumption** (t673): KH2's exe keeps hits per window as the OG rewrite does. Superseded by
  the decomp reading below (t-rehit2): per attack, which changes with the ATKP or a closed window.

## KH2's rule from the decomp, one path for every KH2 move (t-rehit2)

The tester (combo row selection after m33): "if other moves are multi hits in kh2 and don't have it
right now (aerial finish) then yes apply it, I also noticed that explosion was working before this
implementation so see if we are not writing two codes for the same thing. check it behind kh2 c
decomp".

- **KH2's attacks** (Steam KH2, Checked (decomp)): each frame an attack range (id 10) of the
  motion is open, the range handler `0x14040D810` calls `0x1403DAFE0 (object, ATKP, group)`, which
  marks the group asserted this frame (`+0x5A0`, cleared each motion update by `0x14040E5C0`) and
  calls the object's `+0x98` method into `0x1403D25D0 (object + 0xC10, ATKP, group, owner, flag)`:
  8 attack slots, one per (group, owner). A slot already holding the same ATKP is kept (the attack
  and what it has hit go on); another ATKP ends the old attack (`0x1403D0EB0`) and makes a new one
  (`0x14040F1C0`, a new id). An attack whose group is no longer asserted ends in its own update
  (`0x14040F610` → `0x1403D0EB0`). Ending an attack clears its bit in every hit record
  (`docs/combat-model.md`, Hit records). So: a **new attack** (everything hittable again) when
  the window's ATKP or group changes, or a window opens after a frame with none; within one
  attack, a target again after the hit row's **interval** (KH2's re-hit slots; the OG rewrite's
  `HitRecord::mark` / `tick`, Checked (code); the slot code itself not re-read in the exe).
- **Sora's multi-hit moves** (`P_EX100.mset` / `W_EX010.mset`, `00battle.bin`, Checked (files);
  ATKP, frames, interval):
  - Keyblade, new attacks: 153 (128 20..32, 147 38..47), Dodge Slash 163 (1007, 1898, 1899),
    Slide Dash 164 (968 9..20, 975 20..33), Flash Step 169 (1896, 1897 on group 6), Horizontal
    Slash 193 (963 12..22, 963 23..30, 967 36..44), Retaliating Slash 172 (1582 2..22 every 10,
    1583 36..50), Aerial Finish 194 (915 18..31, 916 31..40, 917 50..60, 927 81..89).
  - Keyblade, re-hits by interval within one window: Aerial Sweep 191 (909 every 10), Aerial
    Spiral 192 (1131 every 12), Aerial Dive 196 (1882 every 8), Retaliating Slash's 1582.
  - Body (`bodyhits`, one KH1 object per window): Explosion 166 (911 32..57 every 8, then 912),
    Magnet Burst 197 (1900, 1901, 1903), Vicinity Break 170 (1869).
- **What ran before**: two copies of KH2's re-hit and nothing for the rest. `bodyhits` kept its
  own per-target interval for its body objects (that is why Explosion re-hit before t673). t673's
  `keyblade_rearm` re-armed the Keyblade object on a window index change. The Keyblade intervals
  ran nowhere: Aerial Sweep, Aerial Spiral, Aerial Dive and Retaliating Slash's first slash hit a
  foe once.
- **Now, one path** (`rehit.rs`):
  - `rehit::on_hit` (from `probe::hit`, every hit of Sora's) notes the hit's row's interval: the
    window's ATKP through `kh1rows::kh2_atkp`, a body or spell object's through `bodyhits::atkp`.
  - `rehit::tick` (each of Sora's frames, at the top of `takeover::physics`, before KH1's
    collision pass) takes the attack's bit off a target whose interval has run.
  - `rehit::new_attack` is KH1's own re-arm (`0x1402B3380`) on an object for a new KH2 attack;
    `kh2slots::keyblade_rearm` asks for it by KH2's rule above (ATKP or group change, or a window
    after none), through `kh1rows::rearm_rows`. Body windows are new KH1 objects already.
  - `bodyhits` no longer keeps its own hits and intervals. KH1's own whole-object interval
    (`+0x50`) stays off.
- Not checked in game. Log: "KH2 re-hit: ATKP ... again after N frames", "KH2's next attack on the
  Keyblade ... re-armed".
- (t680) **Valor's left Keyblade**: its own KH1 attack object (`bodyhits`, one per play of the
  motion, its own hit bit), re-armed by `bodyhits::left_step` on a new KH2 attack of the left
  weapon only; the right object's `keyblade_rearm` now looks at weapon 1 windows only (before, a
  left window opening re-armed the right blade). KH2 keeps an attack per (group, shape source
  object), so the two blades' attacks never share hits (Steam KH2 `0x1403D25D0`, Checked (decomp));
  `docs/kh1-drive-forms-native.md` §7.1.
