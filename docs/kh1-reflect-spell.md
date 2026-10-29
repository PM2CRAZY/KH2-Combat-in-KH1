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
- **Level** (Ours: the user's rule, `reflect::level`): Aero's level, read as KH1's menus read
  it (the spell table's `+0` for spell 6), else the save's `+0x498`, else 1. Each level uses
  its own KH2 files: `magic/REFLECT_<n>.mag` (script and effect package), the MAGC row of that
  level (Data: every level casts with motions 71 / 72 / 73 and voices 7 / 25), ATKP and `magi`
  at sub `level × 2 − 2` (Data: the barrier 573 kind 3 at subs 0 / 2 / 4; the counter 574 power
  50 % re-hits every 10 / 8 / 6 frames). Run offline, the three scripts differ in sizes and
  sounds: barrier radius 110 / 160 / 210, counter 240 / 320 / 380, burst 260 / 340 / 400;
  barrier sound 138 / 175 / 177, burst sound 172 / 176 / 178. A level whose file doesn't load
  falls back to the next one down.
- **The list**: after KH1's fill, `0x7D` goes after the spells, before Summon, once Sora has Aero
  (`reflect::learned`, the user's rule), its disabled bit inserted at its place in both words.
  Greyed as a KH1 spell is greyed (magic allowed in this state, not sealed), when Sora has less
  than 1 MP (`COST` = 100, KH1's own check), and while KH1 has Sora. With 9 entries the layout
  is pointed at our own 8 row panels (KH1's 7 and the 7th again), again whenever KH1 builds the
  layout anew. Brightness bytes are left to the drawer.
- **Shortcuts**: see `kh1-shortcuts.md` ("Reflect"). A shortcut holding Reflect gives KH1's
  command step the same command 0x7D as the menu, so it reaches `reflect::chosen` the same way.

## Casting (Checked, exe for KH2's rules; Ours for the hand-over)

- **On KH1's cast state (testing branch, kh2-magic-and-reflect-native m5 plan item 2):** base
  Sora in KH1's own states (`shared`), in a state KH1 casts in (0 / 6 / 0x18 ground, 2 air), on
  Steam: `combat::command` hands the command to `reflect::cast_native`, which casts as KH1's
  command step casts a spell (`0x1402A3600` / `0x1402A3DB0`, Checked (decomp)) with requests of
  ours for KH2's cast motions 71 / 72 / 73 (`kh1rows::OURS`, slots on KH1's Thunder casts 56 / 81,
  `kh2slots`), KH1's air hop of 30 and half the sideways speed in the air (btltbl `+0xE8` /
  `+0xF0`), and KH2's script run on KH1's frames (`reflect::native_step`, from
  `takeover::native_spells`). The rest of this section is the borrow, still used in Drive Forms,
  KH1's flight (state 8) and on Epic.

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

## Against KH2's own Reflect (testing branch, t572 audit)

The tester (kh2-magic-and-reflect-native m6): "reflect is currently not 1:1 with kh2, from how it
falls to how it behaves in game, please look at the steam c decomp". Steam addresses. Tags:
**Checked (decomp)** KH2 / KH1 Steam decomps, **Checked (files)** the KH2 files (`00battle.bin`
ATKP, `P_EX100.mset`, `REFLECT_1.mag` run through `kh2-magic`), **Assumption** not verified.

What KH2 does:
- **The barrier** is ATKP 573: kind 3 (special guard), power 0, refact self 2, no reflect range
  (every side), at every sub. **Checked (files).** KH2's attack-against-attack pass
  (`0x1403D08E0`) meets it with an enemy's kind 0 (normal) or 1 (pierce armor) attack (the 7 × 7
  table `0x1405C5E98`), not with guards or kinds 4..6 (`docs/kh1-kh2-recoil.md` 1.1). Each side
  then (`0x1403D2140`): the meeting is counted on the barrier (`+0xB0`, what the script reads as
  its reflect count); the barrier goes on (refact self 2); the enemy's attack stops (unless its
  own refact self is 2) and its owner plays its clash reaction (its ATKP reflected motion and hit
  back); Sora's own clash reaction is only the effect (refact self 2: no motion, no push).
  **Checked (decomp).** So KH2's Reflect stops enemy shots; it doesn't send them back.
- **The script** (REFLECT_1, any hit count): the barrier from script frame 2 for 27 frames; no
  hits, no counter; with hits the counter (574 for 1..4, 576 for 5..6, 578 more; power 50, kind 1,
  a flinch, re-armed every 10 / 8 / 6 frames by level) from 30 to 57, then the burst (667 / 668 /
  669) 58 to 63. **Checked (files).** Every cast is its own script: a second Reflect doesn't end
  the first (`0x1403C5170` starts a cast each time; **Assumption**: nothing in KH2 cancels a
  running Reflect, none found).
- **No invulnerability** of its own: motions 71 / 72 (and 73, `docs/kh1-magic-native.md` §4e)
  carry no invulnerable range (range id 27, which Aerial Recovery has 0..16). **Checked (files).**
  The barrier, covering him, is what stops hits; the finisher Reflect (72) releases at frame 28,
  so he can be hit before that, and after the barrier falls.
- **Falls** (the air cast 73): `docs/kh1-magic-native.md` §4e / §4g / §4h: a 40 hop with his
  speed kept and drifting × 0.95, held 16..74 with the arc's clock running, landing cuts the cast.

What changed (t572 and t573, base Sora on KH1's cast state and the borrowed path alike):
- Every attack of the script strikes with its own row (the counter's re-hits, then the burst),
  where main's s229 / s233 let one pop strike once per foe with the burst's numbers.
- A second Reflect leaves the first running (its own barrier, count and counter), where main's
  s175 / s182 ended it without its counter and carried its hits over.
- Each barrier counts the attacks that met it (the newest one counts what `hurt` stops).
- (t573) He is out of reach (KH1's flag 0x40) only while a barrier stands, not all through the cast
  motion (main's s179); `hurt` no longer evades hits in the cast.
- (t573) An enemy's melee attack meeting the barrier gets KH1's clash with a guard, its owner's recoil
  (`Strikes::barrier_meet`), as shots already did; main's s226 only counted it.

What still differs (asked or open):
- Shots: KH1's clash with a priority-100 guard turns a free-standing shot back (KH1's own
  reflect, `docs/kh1-reflect.md`); KH2 only stops it.
- Which attacks the barrier meets: KH2's table by ATKP kind (no guards, no kinds 4..6); KH1's
  enemy attacks have no KH2 kind, and while the barrier stands every KH1 attack misses him.
- The burst's sound (element 7's, 173) is inferred; the script starts no element for it.
- The voice: a KH1 swing grunt per cast and "Protect!" on the finisher (main's choice); KH2 plays
  MAGC voice 7 per cast (and 25 on the finisher) with Battle Quotes KH2.
- Drive Forms still cast Reflect on KH2's controller (its stop and hold, clock stopped).
- The held barrier and the combo wait (main's s172 / s180) were already off in the code.

## Shots on the barrier and Reflect's voice (testing branch, research for m8)

Steam addresses. Tags as above.

### KH2: the barrier stops a shot, it doesn't send it back
- Each side of a meeting goes through `0x1403D2140 (own, other)`: the meeting counted on its own
  attack (`+0xB0`); unless its own ATKP refact self (`+0x13`) is 2, its attack gets flag `0x200`
  and remembers the other (`+0x90`); unless the attack has flag `0x100`, its owner's clash
  reaction runs (class vtable `+0xD0`); and its script (`+0xA4`) is told with event 3.
  **Checked (decomp)**, KH2 Steam.
- Flag `0x200` is one of the flags `0x1403D1F20` reads as "this attack is over" (with `2`, `8`,
  `0x10`). **Checked (decomp)**. The barrier (ATKP 573, refact self 2) is left standing; the shot
  is over, and what the shot object does then (vanish, fall) is its own script's answer to event
  3. **Assumption**: that the shots of KH2's enemies vanish on it (each script's own).

### KH1: what the mod does now, and KH1's own way to stop a shot
- Now: `Strikes::barrier_meet` runs KH1's clash half (Steam `0x1402B4BC0 (shot, barrier)`) with
  the barrier as a priority-100 guard. For a free-standing shot (`+0x30` bit `0x400`) under
  priority 0x62 that is KH1's **reflect**: team bits 12..13 flipped (`| 0x1001000` /
  `0x1002000`), flag `0x1000000`, owner = the barrier's. **Checked (decomp)**, KH1 Steam. So the
  shot is sent back, which KH2 doesn't do.
- KH1 has no clash result that stops a free-standing shot without turning it: the clash half only
  reflects it, or leaves it alone (priority 98 / 99, or a winning directional shot); KH1 never
  marks a free-standing object spent (`0x100000` only when `+0x30 & 0x400` is clear).
  **Checked (decomp)**.
- What KH1 does when a shot strikes an actor that can't be hurt: the body contact (Steam
  `0x1402B4060 (attack entry, body entry)`) writes the result into the attack's `+4` and into its
  result word (`*(+0x60)`): `1` a hit, **`4` when the body's actor flags have `0x1000080`** (the
  30-frame no-damage flag `0x80` after a reaction, or the script hold `0x1000000`), `0x40` for a
  no-effect record. The shot keeps its team and owner, and its controller (its script) reads the
  result as it reads any hit. **Checked (decomp)**.
- Plan (not built; m8 answer 2 (a)): a shot meeting the barrier gets that "struck someone who
  can't be hurt" result (`+4` and `*(+0x60)` `|= 4`) instead of the clash half: no reflect, no
  team or owner change, counted on the barrier as now, the contact effect as now. That is KH1's
  own stop for a shot, and the nearest thing to KH2's `0x200` + script event 3.
  **Assumption**: that each KH1 shot's script ends or drops the shot on that result as it does on
  a hit (the scripts aren't read); the log would show any shot that flies on.
- Melee attacks meeting the barrier: unchanged (counted; their owner's recoil as t573 has it).

### Shots stopped by the barrier (built, m9 "go ahead")
The tester: "if you need to add more features into kh1's engine to get reflect working then do
it". Reading further in the KH1 Steam decomp:
- An attack object made by an enemy's script (the script native Steam `0x1402BBA20`, through the
  attack maker `0x1402B2DC0`) carries its script process at `+0xA0` (the process running when it
  was made, `0x142DB7B40`) and starts with no result word (`+0x60` null) and no hit label
  (`+0x38` 0; the script sets one). **Checked (decomp)**.
- When an attack strikes an actor, KH1's contact `0x1402BF330` makes the hit record and, when the
  attack has a hit label, runs the shot's script there at once: `0x1402BCC00 (process, label, 1,
  &the struck actor's handle, 0)` (the script machine run at a code offset, the handle pushed as
  its argument). That is how a KH1 shot's own script learns it hit something, the counterpart of
  KH2's event 3. The body contact `0x1402B4060` goes through it first, then writes the result
  (`4` for a body with `0x1000080`). **Checked (decomp)**.
- KH1's "this attack is over": an attack whose activity bits (`0xC`) are clear is marked by KH1's
  attack pass `0x1402B47A0` with `0x200000` (unless its owner keeps it, word bit `0x100`) and
  freed at the next pass (`0x1402B3560`, which runs its free callback `+0xB4`). KH1's own script
  command for an attack's activity is `0x1402BBC10`. **Checked (decomp)**.

What the mod does now (Steam; `combat::stop_shot`): a free-standing shot meeting the barrier gets
(1) result `4` in `+4` and in `*(+0x60)` when it has one, (2) its hit label run with Sora's handle,
as when it strikes an actor, (3) its activity cleared, so KH1 frees it as an attack that's over
(KH2's flag `0x200`). No reflect, no team or owner change, no hit record on Sora. Counted on the
barrier and the contact effect as before. Melee attacks unchanged (t573). Epic keeps the old clash
(turned back).
- **Assumption**: what each enemy's hit label does (vanish, burst, fall) is its own script's,
  as in KH2; not read per enemy. Passing Sora as the one struck is ours (KH1 has no barrier
  object of its own to pass).
- Log: "Reflect barrier stopped enemy shot ... its hit script told / none, attack switched off";
  "Reflect: enemy shot ... still flying 10 frames after meeting the barrier" if its script turns
  it back on.

### Reflect's voice in KH2
- KH2's cast start (`0x1403C5170`): the MAGC row's voice `+0x34` for a ground or air cast,
  `+0x35` for the combo finisher cast (when it has a finisher motion), `+0x36` instead when the
  caster's `+0xBF8` handle is himself; played through the owner's vtable `+0xA0 (obj, voice, 0)`
  when not negative. **Checked (decomp)**, KH2 Steam.
- Reflect's rows (every level, world 0): voice 7, finisher voice 25, no `+0x36` (255).
  **Checked (files)**, `00battle.bin`. VTBL (character 1): voice 7 = clips 12, 9, 22, 21, 11 (an
  equal share each, priority 1), the same cast voice as Fire; voice 25 = clip 31 ("Protect!").
  **Checked (files)**.
- Now (t573): a cast plays a KH1 swing grunt and the finisher always KH2's voice 25, whatever
  Battle Quotes says. Plan (m8 answer 3): Battle Quotes KH2: KH2's voice 7 per cast, 25 on the
  finisher (as `formmagic` already plays KH2's MAGC voices); Battle Quotes KH1: the grunt per
  cast, and on the finisher KH1's Aero line "Defense!" (`docs/kh1-voice.md`, clip 46 or 47, at
  the spell rows' priority 3) through KH1's clip play (Steam `0x140290930`).

### Reflect's voice (built, m9 "go ahead")
`reflect::cast_voice`, on KH1's cast state and the borrowed path alike:
- Battle Quotes KH2: the MAGC row's voice 7 on a ground or air cast (KH2's even pick of five
  clips, through KH1's voice channel as KH2's other quotes, `kh2sound`), voice 25 ("Protect!")
  on the finisher cast. Both are loaded whatever the Config says (`REFLECT_VOICES`). **Checked
  (decomp, files)**.
- Battle Quotes KH1: a KH1 swing grunt (group 4) on a cast; on the finisher KH1's Aero clip 46
  at priority 3 through KH1's clip play (`kh1voice::defense`). **Assumption**: that clip 46 (not
  47) is "Defense!"; the log line "KH1 voice: Reflect finisher, KH1's Aero line ... (clip 46 ...)"
  names the clip each time (first 40).

## The counter came before the barrier was gone (the tester, m9: "still sending its shots on the first hit")

- The trace (`testers/kh2-magic-and-reflect-native/trace_1792960190_f42073`, `..._f42322`,
  `..._f42330`): every cast's barrier attack stood 27 game frames ("barrier up" one frame after
  the cast, "barrier down 0.48 s after the cast"), and the counter's element 4 and the first 574
  hits came the next frame, while hits that met the barrier on its first frames had flashed it
  (element 1). **Checked (trace)**.
- Reflect's script keeps its barrier for as long as its element 0 is alive (`trap_effect_is_alive`
  in its barrier loop; the counter starts after the loop). **Checked (files)**, `REFLECT_3.mag`
  read with our disassembler. An element lives its particle data's length in **effect ticks**,
  which KH2 runs at 30 Hz (one per two game frames, `0x1402C1D80`, `docs/kh2-effects.md` §1).
  Element 0's data is 27 ticks long: KH2's barrier stands **54** frames (0.9 s).
- `kh2-magic`'s `Effect::alive` compared its age in game frames with that length in ticks, so
  every element died at half its length. Reflect's barrier attack went at frame 27 and the counter
  fired then, while KH1's effect engine (which runs KH2's elements on their own 30 Hz clock) still
  drew the barrier for another 27 frames: the shots came out of a barrier still standing.
- Fixed in `kh2_magic::effect` (`FRAMES_PER_TICK`): ages compare in ticks. Test
  `effects_live_their_length_in_30_hz_ticks`. The barrier now stands 54 frames, its element 4
  and counter (574 / 576 / 578, re-hit every 10 / 8 / 6 frames by level for the magi row's time)
  as it drops, then the burst, as KH2's script runs them.
- This is the spell runtime's clock, so every KH2 spell run by `kh2-magic` changes where its
  script waits on an element: Blizzard's shard flies for as long as its element lives (twice as
  long as before), and Fire / Thunder / Cure / Magnet and the forms' scripts check their elements
  the same way.

## Reflect into Reflect: KH2's teamwork block (corrects t572's assumption)

- Reflect's script (every level) asks at its cast for KH2's teamwork block with key 0x10002
  (`trap_teamwork_alloc`, 12 bytes). KH2 keeps one block per key for every script: made zeroed
  (`size + 1` bytes) the first time, the same block every time after (Steam `0x1403F6DF0`, a
  table of (key, block) pairs at `0x142AE75D0`, count `0x142AE7650`). **Checked (decomp)**.
- What the script does with it (**Checked (files)**, `REFLECT_3.mag` read with our disassembler):
  at its release, if the block's +4 holds a barrier element, it ends it (`trap_effect_kill`), then
  puts its own there. Its barrier loop runs while its own element lives. After it, it adds its own
  count to the block's +8; only if the block's +4 is still its own element (it is the newest
  Reflect) does it clear +4 / +8 and fire the counter, its tier by the whole sum. So a Reflect
  cast while another stands ends the older barrier at once, the older script hands its hits on and
  fires nothing, and the newest one's counter counts the whole chain. t572's "each cast keeps its
  own barrier and counter" (an **Assumption** then: nothing was found that ended the last one) was
  wrong; this is where KH2 does it.
- `kh2-magic` had a block per cast (keyed by key and size), so the chain never met. Now
  `Magic::teamwork` holds one block per key, and each cast's copy is brought up to date before its
  script runs and written back after (scripts run one at a time, so this is the same as KH2's one
  block). Test `reflect_chain_shares_the_teamwork_block` (needs the extracted files; not run here).

## The barrier as a KH1 guard object (t796; the tester, m12 question 2 "(a)")

- **KH2** (Steam KH2, **Checked (decomp)**): the barrier is its own attack (ATKP 573, kind 3, a
  special guard, refact self 2). KH2's attack meeting `0x1403D2140` meets it with every enemy
  attack: a shot is marked over (`0x200`) and its script told, a melee attack is stopped and its
  owner plays its clash reaction, and each meeting counts on the barrier (`+0xB0`).
- **KH1** (Steam KH1, **Checked (decomp)**): a guard is an attack object of priority 100. KH1's
  collision pass `0x1402B4460` builds each live object into its entry list (`0x1402B3800`), tests
  every new entry against the ones already in (`0x1402B3BE0`) and hands an overlapping pair that
  passes the pairing table (`0x1405299B8`) to its meeting `0x1402B4A20`: the two directional tests
  `0x1402B4640`, then each side's half `0x1402B4BC0` (the enemy attack stopped, its owner's clash
  record; a free-flying shot met by a guard is turned back, KH1's reflect).
- **Now:** the barrier is a KH1 guard object of Sora's (`combat::Strikes::barrier_guard`), made
  the way the body guards are (`guard_step`): KH1's row maker on a copy of his swing row with the
  barrier ATKP's reflect arc, clash type 4 (a pure guard: KH1's pass never meets it with a body),
  priority 100, always live. KH1's builder puts the barrier's own shapes in as KH2's ellipsoids
  (`bodyhits::guard_shapes`, the hook on `0x1402B3800` the spell attacks already use). KH1's pass
  finds every enemy attack that meets it. At KH1's meeting (`recoil`'s hook, now first
  `combat::barrier_meeting`):
  - a free-standing shot (`+0x30` bit 0x400) is stopped as t582 stops one (`stop_shot`: KH1's
    "struck, no damage" result, its hit script told, its attack switched off), and KH1's halves
    are skipped, since they would turn it back;
  - anything else (melee) goes to KH1's own halves: stopped, its owner's clash record, the barrier
    standing;
  - either way the meeting counts on the barrier (its counter) and the contact effect plays where
    the two entries met, once per barrier and enemy attack.
- The barrier's object is freed when the barrier goes, and while it is hidden waiting for the
  combo (as before, it stops nothing then). Our own overlap test (`barrier_meet`) only runs where
  KH1's meeting or builder hook isn't in (Epic, or unexpected bytes).
- Same KH2 rule as before for what happens to each attack; what changed is who finds the meeting
  (KH1's pass, its pairing table and directional tests, at the barrier's real shape). Not checked
  in game. **Assumption:** KH1's pairing table lets a clash type 4 guard meet every enemy attack
  type the old test met (the body guards rely on the same, `docs/kh1-clash.md`).
- The offline test of the chain (`reflect_chain_shares_the_teamwork_block`) ran for the first time
  with the tester's own extracted files during t796..t800: it fails ("the first barrier still
  stands" at frame 20) both before these versions and after them, so t610's chain is still
  unconfirmed. Open.


### Every enemy attack type meets the barrier (t813, t814; the owner: "do (a) then also let it meet any guard types it can't like (b)")

- KH1's pairing table (Steam `0x140528CB8`, **Checked (exe)**): rows by clash type 0..7 are
  `1F 09 09 3F 29 18 00 00` (bit n: meets type n). KH1's entry test (`0x1402B3BE0`, **Checked
  (decomp)**) reads the existing entry's row with the new entry's bit. Types 6 and 7 meet nothing.
- A body meets an attack of types 0, 3, 5, 6 and 7 in that test (pure guards: 1, 2, 4), unless the
  attack's result word (`+4`) has 0x30; KH1's attack pass clears that word for every object right
  before building its entries (`0x1402B47A0`). **Checked (decomp)**.
- Enemy attacks in the testers' logs (lines "enemy attack near Sora", 203 of them, **Checked
  (trace)**): type 0 (melee and shots) 157, type 1 melee 7, type 2 melee 19, type 4 melee 2, type 6
  shots 18. t796's type 4 guard met only 0, 3, 5: 46 of them went through the barrier uncounted.
- **t813:** the barrier's guard is clash type 3 (meets 0..5). Its result word is set to "clashed"
  (0x10) in the entry hook each pass, so the body branch skips it; the one-hit bit 0x80 is cleared
  so the attack branch still meets it; KH1's half for it then does nothing (the barrier stands),
  and the "spent" bit a half sets is cleared each pass.
- **t814:** enemy attacks of types 6 and 7 (which KH1 meets with nothing, its own guards included)
  are tested against the barrier's entries in the same pass with KH1's own overlap test
  (`0x1402B5EF0`) and, on contact, handed to KH1's meeting (`0x1402B4A20`), where the barrier's rule
  runs as for any other attack.
- **Assumption:** KH1's pass skips an attack of a body-striking type whose carrier is switched off
  (`+0x88` bit 0, `0x1402B47A0`); with type 3 the barrier then meets nothing while Sora is in that
  state (which states set it isn't known yet).
