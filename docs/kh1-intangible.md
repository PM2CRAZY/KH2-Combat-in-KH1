# Untouchable frames (KH1's dodge roll) and Reflect's barrier

Addresses are the Epic build's (`0x140000000`); Steam's are in `src/win/steam_map.in`.
Confidence: **Checked (exe)** read from KH1's code; **Ours** the mod's choice.

## How KH1 keeps an actor out of every hit (Checked, exe)

- KH1's hit pass (`0x1402B22D0`) registers, each frame, the body parts of every actor that is
  hittable (`0x14028FEE0`): it has a collision record (`+0x134`) with parts, its byte `+6` isn't
  2, its flags word (`+0`) `& 0x8048` is exactly `0x8000`, and not every part is masked off
  (`+0x3A8`, one bit a part). Only registered parts can be met by an attack, so an actor left out
  gets no hit record (`0x1402BD480`), no hit effect, and no grab.
- Motion events `0x2B` / `0x2A` (`0x14028C550`, the motion event ops) set and clear flag `0x40`
  on the actor: the dodge roll's untouchable frames. Script ops (`0x1402B8850` / `0x1402B8890`)
  set and clear bits of the part masks `+0x3A4` / `+0x3A8` / `+0x3AC`.

## Ours

- While KH2 has Sora and its controller says he is invulnerable (Dodge Roll, Quick Run, Aerial
  Dodge, Aerial Recovery, the Drive change), and through Reflect (its cast motions and its
  barrier), Sora's flag `0x40` is set (`reflect::intangible`), and taken off after (only if we set
  it), and whenever KH1 has him. Hits, their effects on him and grabs (Ansem's Guard) miss.
- Reflect's barrier counts the enemy attacks that meet it itself (`Strikes::barrier_meet`: one per
  attack object and barrier, melee or shot), plays its contact effect there (the spell's element
  11, effect 8; its sound 174 is empty in the PC files, so element 1's chime), and no longer runs
  KH1's clash for a melee attack (that recoiled the attacker at every hit). Shots are still
  stopped by KH1's guard clash (turned back when KH1 allows it).
- The counter fires once, as the script has it, when the barrier goes, if it was hit: it no
  longer waits for Sora's combo, and a short KH1 move (a spell, an item) pauses the Reflect
  instead of ending it (more than 3 s ends it).

## KH2's invulnerability on KH1's no-damage timer (t858..t860, Steam addresses)

**KH2 (Checked (decomp)).** A hit on an invulnerable actor still lands. KH2's hit landing
(`0x1403D1730`) asks the target's vtable `+0xB8` (`0x1403D5310` / `0x1403D65A0` for the player)
and makes the result kind 4 instead of 1 when it says yes; the hit record, the contact and the
attacker's side still run. The test is true while the timer `+0xD70` is above 0 (set by
`0x1403D7E40`: the dodge roll from fmab `+0x40`, Quick Run from fmab `+0x1C`, the guard repel 80),
while `+0x18C` bit 22 is set (motion range 27's start, `0x14040D810` case 0x1B), or `+0x126` bit 0.
The timer counts down by the frame scale (1.0 a 60 Hz frame).

**KH1 (Checked (decomp)).** Two mechanisms:
- The no-damage timer (`0x140297F30`: `+0xC` frames, flag 0x80 set, 0x100000 cleared; Sora's think
  `0x140297FE0` arms it, counts `+0xC` down by the frame scale `0x14233FBDC` (60 Hz vblanks) and
  clears both flags at 0). The hit still lands: the contact's kind is 4 for flags `0x1000080`
  (`docs/kh1-hit-effects.md`), no damage (`0x1402BEEE0`), no reaction but a clash (`0x14029BBD0`).
  KH1's roll (12), reactions (30) and clash recoils (8) use it.
- The untouchable flag 0x40 (above): no hit at all. Set and cleared by event ops 0x2B / 0x2A of
  KH1's event-command queue (`0x14028E6E0`), i.e. scripts; KH1's roll doesn't use it.

So KH2's invulnerability is KH1's timer (`combomods::kh1_no_damage`):
- **t858** the dodge roll: KH1's roll timer set to the level's `dodge_roll_star_time` (10 / 16 / 24 /
  32). It runs on if the roll is left early, as KH2's timer does.
- **t859** Quick Run: its start sets the level's frames (0 / 8 / 12 / 16; 0 ends a roll's timer).
- **t860** range 27 of our slots (Explosion 32..64, Aerial Recovery 0..16, Counterguard 0..20,
  Finishing Leap 24..38): `kh2slots::invulnerable_step` sets the frames left in the range when it
  opens, a quarter frame less as our mark; a longer KH1 timer already running is kept; ours is ended
  if the move is left before the range ends (**Assumption**: KH2 drops the bit with the motion).
- The guard repel's 80 (Recoil KH2) was already on this timer (`combomods::guard_repel`).

Before t858, `hurt` marked those hits "no effect": KH1's normal spark had already played and the
attack was spent on him. Still on flag 0x40: the borrow (`reflect::intangible`, forms and KH2's
controller), not 1:1 with KH2's kind-4 rule.
