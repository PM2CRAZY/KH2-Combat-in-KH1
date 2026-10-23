# KH1's voice (Sora's battle quotes)

How KH1 plays its characters' battle voice, and which of Sora's lines the mod uses. Addresses
are the Epic build's (image base `0x140000000`); the Steam build's are in
`crates/kh1-hook/src/win/steam_map.in`. Code: `crates/kh1-hook/src/win/kh1voice.rs`.

Confidence: **Checked (exe)** read from KH1's code; **Data** read from the user's `btltbl.bin`;
**Inferred** from where a line is played, not heard.

## The calls (Checked, exe)

- `0x14028E060 (actor, group, other)`: plays one of the speaker's voice **groups**. The speaker's
  table is in `btltbl.bin` (loaded at `0x142D23740`) at the offset `0x140529560[character]`
  holds, the character being the actor's object `+0x4C`. A group is 11 bytes: a priority, then
  five (clip, weight) pairs. KH1 draws a number 0..100 and walks the pairs: a weight above 0
  takes that clip with that chance; weight 0 puts the clip in an equal-share pool drawn from if
  nothing was taken; clip `0xFD` plays the line of `other` (a party member) instead, `0xFE` is
  silence, `0xFF` an empty pair. A group is thus already a random choice.
- `0x14028E7A0 (actor, clip, priority)`: plays one **clip** of the speaker's bank, unless a voice
  of the speaker's at a higher priority is playing (a per-speaker slot at `0x142D37BD0`, by the
  stats' `+0xBD`); then his mouth moves (`0x14029EF20 (actor, 0x23)`).
- Callers: an attack row's voice byte (`+0xF`, `0x1402A17F0`), the damage code (hurt, knock-out:
  `0x140299A40`, `0x14029A8D0`), the party AI, and the summon sequences.

## Sora's groups (Data; meaning from their callers)

Sora's table is at `btltbl.bin + 0xBC48`, 20 groups (Donald's follows at `0xBD28`).

| Group | Clips (weight) | Played by |
|---|---|---|
| 0 | 10, 11, 12 | a light hurt |
| 1 | 13, 14 | a heavy hurt |
| 2 | 16 | knocked out |
| 4 | 21, 22, 23, 24, 28 (equal) | his combo swings (every swing row of motions 200, 201, 204, 205...) |
| 5 | 25, 26, 27, 33 | the combo finishers (rows of 203, 206, ...) |
| 6 | 29, 31, 32 | air finishers and others (rows of 202, 207, 210, 216..218) |
| 7..13 | 40..47 (75) with 30 / 18 / 19 (25) | **Inferred**: the seven spells in order (Cure 10 and Aero 13 have the ally branch `0xFD`) |
| 15 | 35, 38 | a summon sequence's step (Mushu's) |
| 17 | 37 (75), 30 (25) | the summons' call (the sequences `0x1403959B0`.. listed with xs_dumbo .. xs_simba) |
| 19 | 24 | one special's row (motion 212) |

**"Give me strength!"** (Inferred): Sora's call when he summons. The summon sequences play
group 17, whose own clip 37 no other group has (30 is a plain grunt shared with the spells).
The mod plays clip 37 alone at the group's priority (3) as a Drive starts.

**Reflect's finisher, "Protect!"**: KH1 has no Reflect and no such line, so it stays KH2's
(MAGC finisher voice 25); Reflect's other casts take a KH1 swing grunt (group 4).

**Dodge roll**: a KH1 swing grunt (group 4, KH1's own random pick of its five clips).
