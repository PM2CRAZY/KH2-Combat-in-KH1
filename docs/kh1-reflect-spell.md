# Reflect in KH1's Magic menu

KH1 has no Reflect; KH2 does. The mod adds KH2's Reflect (level 1) to KH1's battle Magic list
and casts it the KH2 way while KH2 controls Sora (`crates/kh1-hook/src/win/reflect.rs`). This
note lists what KH1 does around its magic menu and what the mod adds. Addresses are the Epic
build's (image base `0x140000000`); the Steam build's are in `src/win/steam_map.in`.

Confidence labels: **Checked (exe)** read from KH1's or KH2's code; **Data** read from the
user's files; **Ours** a choice of the mod's where the games don't say.

## KH1's commands (Checked, exe + Data)

- `btltbl.bin` is loaded to `0x142D23740` by `0x14028D840`; the command records start at
  `0x142D24198` (`*0x142D37750`), 16 bytes each, 256 of them: `0x14028D7D0 (id)` returns
  `table + id * 16`. In the user's file records `0x7D`..`0xFF` are all zero.
- A record: `+0` flags (`8`: a range check against the target, `4`: aim at the lock-on
  target), `+4` u16 the name's word index, `+6` the command type (2 = magic), `+7` its argument
  (the spell, 0..6 Fire .. Aero), `+8` a submenu to open (Cure's and Aero's 4: the party target
  list), `+0xC` / `+0xE` u16 ranges.
- Spell commands are `level + (spell + 2) * 3`: Fire 7..9, Blizzard 10..12, ... Aero 25..27
  (`0x14028D0D0 (spell)` gives the level, 1..3, 0 = not learned). Command 6 is Summon.
- Names: the command name drawer `0x1402787B0` reads the record's word index and calls
  `0x14028D820 (index)` (the Word.bin handle at `0x142D23998 + index * 4`, as text) once, at
  `0x1402787FB`. Every list's row names go through it (`0x1402769D0`, the shortcut menu).
- Choosing an entry runs `0x1402912F0 (menu, entry)`: a disabled entry (the menu's `+8` bits)
  plays the "can't" sound; else the record's type and argument go into the command record
  `0x142D54440` (`type | arg << 8`) and the menu chain closes. The player's control step
  `0x1402A4090` then calls the command step `0x1402A5380` and clears `0x142D54440`.

## The magic list (Checked, exe)

- Filled each frame by `0x140292330 (Sora's actor, his movement state's flags)` into the menu at
  `0x140529638`: `+0` type (1), `+4` / `+8` disabled bits, `+0xC` cursor, `+0x10` count, `+0x14`
  command ids (16), `+0x24` each row's brightness (stepped by the menu drawer toward 0xE6 / 0x73
  from the `+4` bits). Learned spells come from party member 0's bits at `save + 0x74`
  (`0x1402830C0`), in order; Summon (command 6, or `0x4F`) is appended last.
- A spell is usable when the state's flags have `0x20` (magic in this state) and
  `0x14028CE40 (actor, spell)`: no seal (`0x142D54438 & 4`), the spell learned and loaded, not
  in a mode without magic (`save + 0x874 & 1`), and the MP check `0x1402A2A80 (actor, cost)`
  with the spell data's (`0x142D23580 + spell * 0x38`, `+0x18` → `+0x30`) cost.
- MP: Sora's stats (`actor + 0x6C`) `+0x44` MP, `+0x48` max. A cost of 100 or more is in
  hundredths of a point (`cost / 100 <= MP`); `0x1402A39D0 (actor, cost)` pays it (`0x1402A2870
  (actor, delta)` adds points, clamped). Smaller costs come out of the charge gauge `+0xBE`
  (30 per point). The same check and payment pair with the same cost field in `0x1401CC840`.
- Drawing (`0x140279700` case 1 → `0x1402769D0`): the list rises with its count (y = 0x15C −
  22 × count, a row lower with Summon last). Rows 0 .. count − 2 each use one of the layout's
  row panels (layout `0x142859E60`: `+0x10` their count, `+0x28` the panels, 0xC0 bytes each,
  copies of one template made by `0x140278A00`); the last row has its own. The magic layout has
  7 row panels (`0x140276010`), exactly enough for 7 spells and Summon. The 8th would be read
  from the next layout's memory.

## What the mod adds

- **Command `0x7D`** (Ours): `02 00 FF 00 | FF 01 | 02 07 | 00 FF 01 02 | 00..`: flags as Cure's
  and Aero's (no lock-on aim), word index `0x1FF`, magic with spell byte 7, no submenu. Written
  over the empty record each fill.
- **Name**: the call at `0x1402787FB` goes to our `word`, which returns "Reflect" (KH1 text
  codes) for index `0x1FF` and KH1's word otherwise.
- **The list**: after KH1's fill, `0x7D` goes after the spells, before Summon, once Sora has Aero
  (`reflect::learned`, the user's rule), its disabled bit inserted at its place in both words.
  Greyed as a KH1 spell is greyed, with Fire's cost (the user: Fire's MP), and also while KH1 has
  Sora. With 9 entries the layout is pointed at our own 8 row panels (KH1's 7 and the 7th
  again), again whenever KH1 builds the layout anew. Brightness bytes are left to the drawer.
- **Shortcuts**: KH1's shortcut slots hold spells 0..6 only; Reflect can't be put in one (not
  done).

## Casting (Checked, exe for KH2's rules; Ours for the hand-over)

- KH1's command step never sees command type 2 / spell 7: `combat::command` hands it to
  `reflect::chosen`, which refuses (KH1's "can't" sound) when the spell file isn't loaded, KH1
  has Sora, or the MP is short, and otherwise queues it.
- KH2 casts it at its next frame its move allows a cast (`Character::can_cast`; within 5 s, as
  KH1's queued commands): KH2's MAGC row for Reflect level 1 (Data: `magic/REFLECT_1.mag`,
  ground motion 71, finisher 72, air 73, voices 7 / 25), KH1's MP paid then. Its `sub` is
  `level × 2 − 2` (+1 for the finisher motion, as KH2); Reflect's ATKP rows have subs 0 / 2 / 4
  only, so a finisher's missing row falls back to the plain one (Ours).
- The script runs in `kh2-magic` with KH1's world: Sora's feet and facing (KH2 y-up, KH1's y
  flipped), his drawn KH2 pose for bones, KH1's enemies (their feet and lock points). Trigger 20
  at the cast, trigger 21 at the motion's magic-cast event (all three motions: frame 0).

## What the script does (Data: the user's REFLECT_1.mag and 00battle)

Run offline (`kh2-magic` test `reflect_counters_what_it_blocked`):

- Element 0 (on the ground; bone 0x4000, offset −22, 85 up, −18; sound 138) and a reflect-kind
  attack, ATKP 573 (kind 3), radius 110 on that element, for 27 frames.
- Nothing blocked: done at frame ~50.
- Blocked hits (`attack_get_reflect_count` > 0): element 1 (sound 139), then from frame 30 the
  counter ATKP 574 (radius 240; level 1: power 50 %, Magic, flinch, re-hit every **10** frames;
  level 3's every 6), then the burst ATKP 667 (radius 260, power 550 %, reaction 6, hit-back
  200) from frame 58 to 63 with elements 4..6 (sound 172).

## Blocking and the counter (Ours, after KH2's rules)

- While ATKP 573 lives, every enemy hit KH1 queues on Sora (`hurt`: kinds that are attacks) is
  marked no-effect and counted; the count goes into the barrier's reflect count before the
  script's next frame. KH2 tests the barrier's shape against the attack; here the hit already
  reached Sora, who stands inside the 110 sphere, so any hit then is one the barrier would have
  met. Attackers get no recoil and projectiles are not turned back.
- The counter's attacks hit KH1's enemies through the same hit records as KH2's swings and land
  with KH1's contact code on Sora's ground swing row (his finisher row for a hit-back of 100 or
  more: the 667 burst), KH2's power on KH1's damage, Magic-based (×(1 + 0.1 × max MP) ×
  `magic_scale`), as Explosion's.

## Effects and sounds (Data + Checked, as Sora's own)

- The spell file's PAX ("refl") is loaded beside Sora's package (`kh2fx`), its 8 textures after
  his; each effect the script starts is started from it, bound to Sora with the same bone codes
  (0x4000: the model root turned half round about x), and ended, killed or faded as the script
  does. Elements 4..6 use one program kh2-fx skips (155 pppKeShpTail2X).
- Each started element's sound (138, 139, 172; bank 0) plays as Sora's elements' do, at the
  game's master × effects volume. The cast voices (MAGC 7 / 25) only with `kh2_voice = on`. The
  cast motions have no sound triggers of their own (their triggers are 16, 19, 31 and the
  magic-cast event).

## Switch

`reflect = on` in `kh1-kh2.ini` (default); `off` leaves KH1's menu as it was.

## Open

- Not tested in the game. The first things to check: the name and greying in the list, a list
  of 9 entries drawing right (the 8th row panel), the barrier effect's place around Sora.
- KH2's own enemy recoil off the barrier and turned-back projectiles.
- Levels 2 / 3 (Reflera, Reflega) would only need a level rule and their files.
