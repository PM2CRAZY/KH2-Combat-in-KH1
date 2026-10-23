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
