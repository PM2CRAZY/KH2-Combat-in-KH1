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

**Dodge roll**: a KH1 swing grunt (group 4, KH1's own random pick of its five clips). Correction
below: this was the old KH2-controller path's choice (`takeover.rs` calling `kh1voice::dodge_roll`),
not KH1's; KH1's own roll has no voice.

## Where KH1's voices come from (the grunt research, after t726)

The question: with a KH2 animation and Battle Quotes KH1, which KH1 voice should a move carry,
taken from the KH1 motion it replaces? Read from KH1 Sora's `xa_ex_0010.mset` and the KH1 Steam
decomp.

- **KH1's motions carry no battle voice.** A sound key's `a` 0x19 (`kh1_data::slot`'s comment calls
  it "voice") is a character sound effect: KH1's sound event handler (Steam `0x140295420`) adds `b`
  to the character's sound base (its combat state `+0x20` -> `+0x34`) and plays it through the
  positioned effect queue (`0x1402961F0`, `0x140179AC0`), like `a` 0. It never reaches the voice
  calls (`0x1402901F0` group, `0x140290930` clip). **Checked (decomp)**. 41 of Sora's 118 motions
  have such keys (the combo swings 98..104 at time 0, spells, ...); a baked slot already keeps them
  from its template unless the row has `own_sounds`. **Checked (files)**
- **KH1's battle voice is played by code only.** Every call of the group play (`0x1402901F0`): the
  attack row start's voice byte (`0x1402A3980`, the row's `+0xF`), the spell command (`0x1402A3600`,
  groups 7..13), the damage code (`0x14029BBD0`: groups 0 / 1, 2, 0x17; `0x14029CA60`), another hit
  path (`0x1402C2B50`, 0x17), `0x1402BC7A0` (2), the party / guest scripts (`0x1401CDBD0`,
  `0x1401CF570` .. `0x1401D07E0`, group 0x1A) and the summon sequences (`0x140395360`, `0x140396510`:
  0x11; `0x140396760`: 0xF; `0x14039BF70`: 5). The clip play's only caller is the group play.
  **Checked (decomp)**
- **KH1's roll is silent.** Its start (Steam `0x1402C9E60`) sets the roll's timer and flag and asks
  request 0xDC; no voice call. Its motion (slot 117) has only material sounds (`a` 5, 26, 1, 4) and
  foot effects. **Checked (decomp + files)**
- So KH1 has nothing to copy for the moves without an attack row: the roll, Quick Run (KH1 has
  none), the Aerial Dodge, the glide hover, the guard, the jump takeoffs, items. Moves with a row
  already get KH1's row voice (`holds_back` drops it only with Battle Quotes KH2, for the KH2
  quote). KH2's own voices on a KH2 clip are its frame triggers 13 / 14, baked as `KH2_VOICE`
  keys and played with Battle Quotes KH2.
- Not checked: which of KH2's clips (the roll, Quick Run) carry a voice trigger.

## Mixing: KH1's animations with KH2's voices and the other way (t742, t743)

Battle Quotes picks whose voice Sora has; each move's style picks whose animation plays. The four
cases:

- **KH1 animation, Battle Quotes KH1**: KH1's own (row voices, spells, hits). Nothing of ours.
- **KH2 animation, Battle Quotes KH2**: the KH2 clip's own voices (`KH2_VOICE` keys); KH1's row
  grunt left out on those rows (`kh2sound::holds_back`).
- **KH2 animation, Battle Quotes KH1**: KH1's row voice on moves with a row. A row-less KH2 move
  with a KH2 voice gets KH1's stand-in from its `KH2_VOICE` key (t742, `kh1voice::STAND_INS`): the
  dodge roll (KH2 voice 7) says a swing grunt (group 4). Quick Run (KH2 voice 36) stays silent.
- **KH1 animation, Battle Quotes KH2** (t743): the row's KH1 group is said as KH2's voice of the
  same kind (`kh2sound::KH1_TO_KH2`: 4 -> 4 swings, 5 -> 5 finishers, 6 -> 6 air finishers, from
  KH1's `btltbl.bin` groups and KH2's `P_EX100.mset` triggers, Checked (files)); other groups stay
  KH1's.

KH2's voice triggers on Sora's moves (`P_EX100.mset`, battle slots, Checked (files)): swings 151..153,
161..164, 169..172, 181 / 182, 191..193, 196 voice 4; finishers 154 / 155, 183 / 184, 194 voice 5; air
finishers 165..167 voice 6 (194 and 197 add 6 later); the roll 209 and 40 / 173 voice 7; Quick Run 203
voice 36; the hurt motions 10..15 voices 29 / 30; the Aerial Dodge 202 and the Quick Run stop 204
none.

Not covered yet: KH1's roll (Dodge Roll style KH1) with Battle Quotes KH2. KH1 starts it with no
voice call and its motion has no voice key, so there is no KH1 point to say KH2's voice 7 from but
our own call of KH1's roll start (`takeover`) or a new hook on it. Spells with Battle Quotes KH2
keep KH1's spell groups (7..13).
