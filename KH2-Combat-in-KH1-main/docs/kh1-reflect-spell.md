# Reflect in KH1's Magic menu

KH1 has no Reflect; KH2 does. The mod adds KH2's Reflect (Reflect / Reflera / Reflega) to KH1's
battle Magic list and shortcuts and casts it the KH2 way while KH2 controls Sora
(`crates/kh1-hook/src/win/reflect.rs`, `shortcuts.rs`). This
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
- MP: Sora's stats (`actor + 0x6C`) `+0x44` MP, `+0x48` max, both whole points: what the HUD
  shows as MP. A cost of 100 or more is in hundredths of a point: the check `0x1402A2A80 (actor,
  cost)` is `cost / 100 <= MP` (integer division), and `0x1402A39D0 (actor, cost)` pays
  `cost / 100` points through `0x1402A2870 (actor, -points)` (adds, clamped to 0..max; with the
  actor's `+0x88 & 0x20` status and without ability 9 the change is scaled by a factor at
  `*0x142D23730 + 0x88`, as for KH1's own spells). Smaller costs come out of the charge gauge
  `+0xBE` (30 per point; check `cost <= (MP + 1) × MP / 2 × 30 + gauge`). So **1 MP is a cost of
  100** (checked: exe). The same check and payment pair with the same cost field in
  `0x1401CC840`. KH1's own spells' costs are in their loaded data (`+0x18 → +0x30`), which isn't
  in these notes' files; the log's "Reflect chosen" line prints Fire's.
- Spell levels (Checked, exe): `save + 0x492 + spell` is each spell's level (0..3);
  `0x14028D3D0 (spell, level, done)` sets it, writes the spell table's `+0` and loads that
  level's file; `0x14028D2F0` brings the table in line with the save at load. KH1's names and
  commands use the table's level (`level + (spell + 2) × 3`: Aero 25 / 26 / 27 = Aero, Aerora,
  Aeroga).
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
- **Name**: the call at `0x1402787FB` goes to our `word`, which returns the level's name
  ("Reflect", "Reflera", "Reflega"; KH1 text codes) for index `0x1FF` and KH1's word otherwise.
- **Level** (Ours: the user's rule, `reflect::found` / `reflect::level`): one level
  per fight won of three (the user: "Reflect should be found in these places: 1. After defeating
  Pot Centipede 2. After defeating Hercules (Hercules cup) 3. After defeating Ice Titan (Gold
  match)"): the first won gives Reflect, the second Reflera, the third Reflega, in whatever order
  they come. The flags (`crate::story`, `docs/drive-model.md`, "When the forms open"): Agrabah's
  story byte ≥ 0x35, the Hercules Cup's state `save + 0xF38` ≥ 1, and Ice Titan's journal entry
  0xA6 with Diamond Dust owned. It no longer follows Aero (before: Aero's level). A box
  shows it as the bosses' gifts' do ("New Magic: Reflect", "Magic Up: Reflera" / "Reflega",
  `menu::Growth::reflect_box`), for one more fight won while the same save plays. Each level uses
  its own KH2 files: `magic/REFLECT_<n>.mag` (script and effect package), the MAGC row of that
  level (Data: every level casts with motions 71 / 72 / 73 and voices 7 / 25), ATKP and `magi`
  at sub `level × 2 − 2` (Data: the barrier 573 kind 3 at subs 0 / 2 / 4; the counter 574 power
  50 % re-hits every 10 / 8 / 6 frames). Run offline, the three scripts differ in sizes and
  sounds: barrier radius 110 / 160 / 210, counter 240 / 320 / 380, burst 260 / 340 / 400;
  barrier sound 138 / 175 / 177, burst sound 172 / 176 / 178. A level whose file doesn't load
  falls back to the next one down.
- **The list**: after KH1's fill, `0x7D` goes after the spells, before Summon, once Sora has
  found Reflect (`reflect::learned`, the user's rule: the first of its three fights won), its
  disabled bit inserted at its place in both words.
  Greyed as a KH1 spell is greyed (magic allowed in this state, not sealed), when Sora has less
  than 1 MP (`COST` = 100, KH1's own check), and while KH1 has Sora. With 9 entries the layout
  is pointed at our own 8 row panels (KH1's 7 and the 7th again), again whenever KH1 builds the
  layout anew. Brightness bytes are left to the drawer.
- **Shortcuts**: see `kh1-shortcuts.md` ("Reflect"). A shortcut holding Reflect gives KH1's
  command step the same command 0x7D as the menu, so it reaches `reflect::chosen` the same way.

## Casting (Checked, exe for KH2's rules; Ours for the hand-over)

- KH1's command step never sees command type 2 / spell 7: `combat::command` hands it to
  `reflect::chosen`, which refuses (KH1's "can't" sound) when the spell file isn't loaded, KH1
  has Sora, or the MP is short, and otherwise queues it.
- KH2 casts it at its next frame its move allows a cast (`Character::can_cast`; within 5 s, as
  KH1's queued commands): KH2's MAGC row for the level (ground motion 71, finisher 72, air 73,
  voices 7 / 25), 1 MP paid then with KH1's own payment (`0x1402A39D0 (actor, 100)`). Its `sub`
  is `level × 2 − 2` (+1 for the finisher motion, as KH2); Reflect's ATKP rows have subs 0 / 2 /
  4 only, so a finisher's missing row falls back to the plain one (Ours).
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

## The barrier held while hits keep coming (Ours)

The user: "can we have reflect keep shielding until attacks have stopped". KH2's barrier stands
27 frames whatever happens (every level's script, offline). Ours stretches it:

- From the barrier's 22nd frame of script time (`HOLD_AT`, a few frames before the script would
  free it), while the last blocked hit is at most 20 frames old (`HOLD_FRAMES`), the cast is
  held: `kh2_magic::Magic::hold` stops its script where it is (its waits don't count down) and
  stops its effects from ageing; its attacks stay, so the barrier attack lives on and `hurt` goes
  on stopping and counting hits. At most 5 s (`HOLD_MAX_S`), once per cast.
- Let go (20 frames without a hit, or the cap), the script carries on from where it stood: the
  barrier drops a few frames later and the counter fires, as strong as the hits counted (the
  script's own `attack_get_reflect_count` logic; offline test `reflect_barrier_held`: held 100
  frames, the barrier and its element 0 stay through the hold, the counter starts 6 frames after).
- The drawn barrier effect: if kh2-fx's copy of it plays out on its own during the hold, it is
  started again (no sound), so it shows the whole time.
- Log: "Reflect barrier held: last hit N frames ago (M blocked so far)", one line per hit while
  held, and "Reflect barrier let go: last hit N frames ago, held F frames (… s), M blocked".

## Effects and sounds (Data + Checked, as Sora's own)

- Each level's spell file's PAX ("refl") is loaded beside Sora's package (`kh2fx`), its textures
  after his (and the previous levels'); each effect the script starts is started from that level's
  package, bound to Sora with the same bone codes
  (0x4000: the model root turned half round about x), and ended, killed or faded as the script
  does. Elements 4..6 use one program kh2-fx skips (155 pppKeShpTail2X).
- Nothing is drawn while KH1's pause machine (`0x14232E9B4`, see `kh1-render.md` 3.1) is up or
  Sora has had no physics frame for 120 ms: the pause menu is drawn before the world pass, so
  effects drawn after it showed through the menu (the user). They are frozen meanwhile anyway.
- Each started element's sound (level 1: 138, 139, 172; bank 0) plays as Sora's elements' do, at the
  game's master × effects volume. The cast voices (MAGC 7 / 25) only with `kh2_voice = on`. The
  cast motions have no sound triggers of their own (their triggers are 16, 19, 31 and the
  magic-cast event).

## Switch

`reflect = on` in `kh1-kh2.ini` (default); `off` leaves KH1's menu as it was.

## Open

- Not tested in the game. The first things to check: the name and greying in the list, a list
  of 9 entries drawing right (the 8th row panel), the barrier effect's place around Sora.
- KH2's own enemy recoil off the barrier and turned-back projectiles.
- Shortcuts, levels 2 / 3, the 1 MP cost and the held barrier: not tested in the game yet.

## Reflect in a combo (s180)

The user: "if sora does a combo with reflect, any attack done to every reflect should burst out
of the final reflect in the chain". A Reflect that blocked something, reaching the point where
its barrier would drop (`HOLD_AT`) while Sora's combo still has steps left
(`Output::combo_live`), is held (`Magic::hold`) with its barrier hidden and stopping nothing; its
counter waits. A Reflect cast later in the combo takes its blocked hits over (the spam chain of
s175) and waits in its turn; once the combo is over (its last step, 60 frames on the ground with
no swing, a landing, a hit taken; at most 6 s) the waiting one goes on and bursts for them all.

s182: the chain's carry now really reaches the last Reflect (`Magic::cancel` frees the cancelled
cast's attacks at once; before, the carry was counted on the old barrier and the chain never
burst), and the burst (ATKP 667 / 668 / 669) plays element 7's sound (173), inferred as the
shatter: the script starts no element for it and element 11's 174 plays nothing on PC.

## One pop per cast (Data + Ours)

- Reflect's script per cast (all three levels, the sample files): the barrier (ATKP 573) stands to
  frame ~29, where element 4 starts; the counter attack 574 then stands frames 30..57 (power 50, a
  flinch, re-armed every 10 / 8 / 6 frames by level), and the burst (667 / 668 / 669, its tier
  by the barrier's count; power 550, never re-armed) frames 58..63, as the cast ends. Held
  (`Magic::hold`) the same, later.
- The port lets only the burst strike (`reflect::BURSTS`): one pop per cast, once per foe, at the
  end, however many attacks met the barrier (the user: the second attack on one Reflect made it
  pop again; 574's re-arms hit each foe 3..5 times). 574 still runs in the script and its effect
  plays. kh2-magic test `reflect_pops_once_for_two_hits`.

## One pop, at the barrier's end, whatever the hits (s233)

- The script's timeline is the same for any number of hits (the sample files, every level): the
  barrier (element 0, ATKP 573) from frame 1 to 29; the first hit flashes it (element 1); at 29
  the counter's animation (element 4 / 5 / 6 by tier) and at 30 the counter attack (574 for 1..4
  hits, 576 for 5..6, 578 for more); the burst (667 / 668 / 669, the same tier) at 58.
- Ours: the pop strikes once per foe as the counter appears (frame 30, with its animation), with
  its tier's burst row for damage; the burst at 58 no longer strikes. The hold while hits keep
  coming (stage 172) is gone: it moved the pop with the hits. Test
  `reflect_pop_frame_ignores_hit_count`.
- Voices (always, not `kh2_voice`'s): a ground or air cast plays one of his swings' grunts at
  random (VTBL voice 4: clips 9..13), the finisher Reflect KH2's MAGC finisher line for Reflect
  (voice 25, clip 31; Fire / Blizzard / Thunder have 11 / 12 / 13, their name calls).
