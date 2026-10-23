# KH1: how one attack hits an actor again, and Sora's no-damage time

Found in Ghidra and capstone (the user's Epic exe), first for the user's report that Sephiroth's
flame pillar killed Sora "instantly", then for a player's report that a Large Body's ground slam
"nuked" him ("kh2 sora doesn't have damage immunity after getting hit"). Addresses are the Epic
build's (`0x140000000`); Steam's are in `src/win/steam_map.in`. Confidence: **Checked (exe)**
read from KH1's code; **Log** from the user's game logs; **Ours** the mod's choice.

## Once per attack, re-armed on an interval (Checked, exe)

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
- Each attack object has its own bit, so an attack made of several objects (a slam and its
  shockwave) hits once per object. The record says which: `+4` the attack's `+0x34`, the handle
  of the actor that made it (the constructor `0x1402B0C30` stores the owner's handle at `+0x34`
  and `+0x90`), `+0x28` the attack's bit, `+0x2C` its attack data entry (`btltbl.bin`).

## Sora's no-damage time after a hit (Checked, exe)

This is what keeps KH1's Sora from taking an attack's every part and every re-arm:
- Every one of the player's reactions (the tables the hit pass `0x140299A40` calls by movement
  state, `0x14029AAB0` .. `0x14029B5DF`) calls `0x140295DA0(actor, t)`: actor flags `+0` get
  `0x80` (and lose `0x100000`), `+0xC` = t. t is 30 for the flinches, knock-backs and launches
  (kinds 3..5, on the ground and in the air), 8 for the attack clashes (kinds 6 / 7). KH1's
  dodge roll (`0x1402C7CB0`) sets 12.
- The actor task counts it (`0x1402962EB`): with `0x80` set (and not `0x1000000`), while the
  reaction's action runs (flags `4`, set by its motion request's bit 24, `0x14029D8A0`) nothing;
  after it, `0x100000` is set and `+0xC` loses the frame step (`0x1423405DC`, 1 at 60 fps) each
  frame; at 0 both flags go. So: the reaction, then 30 frames.
- While `0x80` (or a script's `0x1000000`) is set, or KH1's command flags (`0x142D54438`) have
  `2`: the damage step `0x1402BCD30` takes no HP (`0x1402BCD5D`), the hit pass plays no
  reaction but a clash (kinds 6 / 7, `0x140299D2C`), and the hit check marks the contact 4
  instead of 1 (the attack's `+4`, and the spark it picks, `0x1402B1F22`).
- In one pass, the first record that plays a reaction sets the flag, so the pass's later
  records take no HP and play no reaction (the pass reacts only to a kind above the last one,
  `0x140299D4B`, and never with the flag set).

## Taking hits (Checked, exe)

- KH1's hit pass `0x140299A40` ignores a record when the record has `0x20` (no effect), the
  actor's flags `+0` has `0x200` (set while carried or grabbed: `0x1402AAC90`, `0x1402C3BA0`), or
  the stats' `+0` has `4` (dying).
- Second Chance (ability `0x40` in the equipped list at `0x1428695A0 + 0x44`): with HP full and
  the damage at least the HP, the damage becomes HP − 1. Only the first hit from full HP is
  spared.

## KH2 has the same time

KH2's player sets its group timer to 30 on a hit, and the timer holds while a reaction motion
(10..=39) plays (`docs/combat-model.md`, `combat_core::HitRecord`). While it runs, only the last
hit's group lands (one attacker's combo); KH1's attacks have no such groups.

## Why Sora was "nuked" (Log, exe)

KH2's reactions are played in KH1's place (`hurt.rs` clears the record's kind), so KH1 never set
its flag `0x80`, and nothing stood for it but two rules of ours: no hit while KH2's blow has him
launched, and the same attacker's hits within 20 frames of the last hit taken while he flinches.
They let through:
- every record of one pass (the 20-frame check read the last hit from before the pass);
- the same attacker's hits more than 20 frames into a flinch (a log: 28, 28 and 28 damage at
  frames 18618, 18663 and 18685, all one attacker's, HP 72 → 16 in one flinch);
- everything while getting up: KH2 plays the landing and get-up (motions 17 / 19) as a hurt, not
  a blow (`character_core` `ActionKind::Hurt` after the launch lands), so the blow's rule ended
  on landing, and the 20 frames had long run out;
- everything after the flinch, at once.

A slam with a shockwave, or any attack that meets him on many frames (a log: one attacker's
records on six frames in a row, 20072..20077), deals one hit's damage for each that gets
through. The Large Body's slam itself is made by its own script, in its enemy file (not in
`btltbl.bin` or the exe), which isn't on the research machine; the logs now name each hit's
attack data and bit (below), so a log of the slam will show its parts.

## What the mod does (Ours)

While KH2 has Sora (`hurt.rs`, `kh1_hook::mercy`):
- one record a pass is taken (damage and KH2's reaction); the pass's other records deal nothing;
- after a hit, while KH2's reaction motion plays (10..=39: flinch, air flinch, blow, landing,
  getting up) and 30 frames after, no hit deals damage or a reaction (`Fate::Mercy`), whoever's,
  as in both games. Guards, Reflect's barrier and attack clashes go on as before;
- KH1's own no-damage flag (`0x80`, `0x1000000`, or the command flags' `2`) is honoured: the
  record is left to KH1, which takes no HP, and KH2 plays no reaction (`Fate::Immune`);
- while KH2's blow has him launched, no hit lands (`Fate::Down`).

The log names each hit's attacker, attack data (an offset in `btltbl.bin`) and bit; hits stopped
by the no-damage time get a line of their own (80 at most), saying whether it was the same
attack object, another object of the same attack data, another attack of the same attacker, or
someone else, and how long after the reaction; the next hit taken says how many were stopped.
