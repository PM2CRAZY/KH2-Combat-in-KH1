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
| 7..13 | 40..47 (75) with 30 / 18 / 19 (25) | the seven spells in order (Cure 10 and Aero 13 have the ally branch `0xFD`): see "Spell casts" below |
| 15 | 35, 38 | a summon sequence's step (Mushu's) |
| 17 | 37 (75), 30 (25) | the summons' call (the sequences `0x1403959B0`.. listed with xs_dumbo .. xs_simba) |
| 19 | 24 | one special's row (motion 212) |

**"Give me strength!"** (Inferred): Sora's call when he summons. The summon sequences play
group 17, whose own clip 37 no other group has (30 is a plain grunt shared with the spells).
The mod plays clip 37 alone at the group's priority (3) as a Drive starts.

## Spell casts (testing branch, Steam addresses)

- KH1's command step for a spell (Steam `0x1402A3600`, reached from the command step
  `0x1402A7510`) plays the caster's voice when the command word has bit 25: for a magic command
  (type 2) whose command record byte `+2` is negative, the group is **spell index + 7** (spell in
  bits 8..23 of the command word), through the group play (Steam `0x1402901F0 (actor, group,
  other)`, `other` the command's target handle `param[1]`); other commands use the record's `+2`
  byte as the group. **Checked (decomp)**, KH1 Steam.
- So Fire 7, Blizzard 8, Thunder 9, Cure 10, Gravity 11, Stop 12, Aero 13. Sora's rows from
  `btltbl.bin` (`+0xBC48 + group * 11`), **Checked (files)**:
  - 7 Fire: clip 40 (75), 30 (25); 8 Blizzard: 41 (75), 30 (25); 9 Thunder: 42 (75), 18 (25);
  - 10 Cure: `0xFD` (100, the target ally's line when he cures an ally), 43 (75), 19 (25);
  - 11 Gravity: 44 (75), 30 (equal share); 12 Stop: 45 (75), 18 (equal share);
  - **13 Aero: `0xFD` (100: the ally's line when cast on an ally), else clips 46 and 47, 50 / 50**;
    priority 3 (every spell row).
- The tester (kh2-magic-and-reflect-native m8): KH1's Sora says "Defense!" as one of his Aero
  lines. That makes it clip 46 or 47 (**Checked**: they are Aero's only two own clips).
  **Assumption / open**: which of the two it is; not listened to. The sound log's "KH1 sound:
  voice call bank 0 no N" line names the clip each Aero cast plays, so one self-cast Aero heard in
  game settles it.
- Clip play for a single line: Steam `0x140290930 (actor, clip, priority)` (Epic `0x14028E7A0`),
  as "Give me strength!" uses.

**Reflect's finisher, "Protect!"**: KH1 has no Reflect and no such line, so it stays KH2's
(MAGC finisher voice 25); Reflect's other casts take a KH1 swing grunt (group 4).
(Testing branch, m8 asked: with Battle Quotes at KH1 the finisher would take KH1's Aero
"Defense!" clip instead; with Battle Quotes at KH2, KH2's own MAGC voices, below in
`docs/kh1-reflect-spell.md`.)

**Reflect's voice (testing branch, built)**: Battle Quotes KH2: KH2's MAGC voice 7 per cast and 25
("Protect!") on the finisher. Battle Quotes KH1: a swing grunt (group 4) per cast and KH1's Aero
clip 46 on the finisher (`kh1voice::DEFENSE_CLIP`; **Assumption** that 46 is "Defense!", the
other Aero clip is 47). `docs/kh1-reflect-spell.md` "Reflect's voice".

**Dodge roll**: a KH1 swing grunt (group 4, KH1's own random pick of its five clips).
