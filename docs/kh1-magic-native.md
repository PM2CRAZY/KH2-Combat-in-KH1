# KH2 magic and Reflect native (testing branch)

The tester (thread `kh2-magic-and-reflect-native`): KH2's magic and Reflect were a borrow (KH2's
controller casts), our copy of KH2's spell scripts (`kh2-magic` / `kh2-bdx`, `formmagic.rs`,
`reflect.rs`) and our own spell hit checks (`combat.rs`), and Cure's heal our own share
(`CURE_SHARE`). Native: KH1's cast state with KH2's cast motions as slots, spell objects as
KH1 attack objects hit by KH1's own collision pass, KH2's numbers 1:1. This note is the
research (how KH1 makes, steps and hits its own spells) and the plan. Steam addresses (image
base `0x140000000`) unless marked Epic. Plain-language notes only.

Labels: **Checked (decomp)** read in the Steam KH1 decomp, **Checked (exe)** read from the Steam
exe's bytes, **Checked (files)** read from the staged game files, **Assumption** not checked.

## 1. How KH1 casts (Checked (decomp))

- The command menu writes the command record `0x142D53A40` (type 2 = magic, the spell in bits
  8..23, the target at `+4`). KH1's control step `0x1402A6220` runs the command step
  `0x1402A7510` whenever the record is set (not gated by "can act": the menu decides what can be
  chosen), then clears it.
- Magic goes to `0x1402A3600`, which picks by Sora's state (`+0x70`): 0, 6, 0x18 the ground cast
  (`0x1402A3DB0`), 2 the air cast (in `0x1402A3600`), 8 the air cast with a turn to the target
  (`0x1402A4150`); any other state casts nothing. Each first asks KH1's usable check
  `0x14028EFD0 (actor, spell)` (not sealed `0x142D53A38 & 4`, the spell's slot loaded at
  `0x142D22B80 + spell × 0x38`, its attack data at `+0x18`, magic allowed `save + 0x874 & 1`, the
  spell learned, then the MP check `0x1402A4C10` with the attack data's `+0x30`).
- **KH1's cast state is no state of its own**: the ground or air state with the action flag
  (`+0` bit 2, set by a request with bit 24, `0x14029F980`) and a cast request:
  - ground `0x36 + spell` then the idle queued (`0x14029FA30 (actor, 0x40000000)`); Fire
    `0x36` then its second part `0x3D` then the idle;
  - air `0x83 + spell` then the fall (request 6; state 8's `0x75`); Fire `0x83` then `0x8A`;
  - a Fire chosen while `0x3D` / `0x8A` plays, or out of the swings `0xC8` / `0xCA` in their combo
    window (`0x14029EFB0`), plays `0x3D` / `0x8A` at once (`0x1402A7510`'s type-2 case): KH1's
    quick Fire.
  - The motion queue (`0x14029F980` new, `0x14029FA30` append): count `+0x178`, entries of 0x10
    from `+0x17C` (request with bit 24 cleared, a start time, the blend, a handle).
- Then the spell start `0x14028F140 (actor, spell, target)`. The request table
  (`xa_ex_0010.mset` section 1, Checked (files)): 0x36 → slot 54 (28 frames), 0x37 → 55 (86),
  0x38 → 56 (68), 0x39 → 56, 0x3A → 55, 0x3B / 0x3C → 56; 0x3D → 57 (60); air 0x83 → 79, 0x84 → 80,
  0x85 / 0x86 → 81, 0x8A → 82. "May be left" points (event 0.0): Blizzard 55 at 68, Fire's 57 at
  46, air 80 at 68, 82 at 46; Fire 54 / Thunder and Cure 56 none. No sound or voice events in
  them (KH1's spell sounds and voices are the scripts').
- KH1's spell voice is the command step's: with the record's bit 25, `0x1402A3600` plays Sora's
  voice group `spell + 7` (`0x1402901F0`, `docs/kh1-voice.md`). With KH2's battle quotes a native
  cast clears that bit and plays KH2's MAGC voice instead; with KH1's, KH1's group plays and
  KH2's voice doesn't (`kh2sound::voice`).
- What else of KH1's runs around a cast: nothing in the command step counts it (no tally call in
  `0x1402A3600` / `0x1402A3DB0` / `0x1402A4150` for magic). Assumption: KH1's `.mag` scripts may
  count or set things of their own (their files aren't staged); a KH2 spell replaces KH1's
  script, so whatever those do is not done for the four KH2 spells.

## 2. KH1's spell scripts (Checked (decomp) / (exe))

- `0x14028F140` reads the spell's slot (`0x142D22B80 + spell × 0x38`: `+0` level, `+8` the
  loaded `.mag`, `+0x18` / `+0x20` its attack data) and starts its script with
  `0x14028F5E0 (actor, target, arg, code, data, second data)`: a task in the list at
  `0x142D22D08` (callback `0x14028F6D0`, which sets Sora's `+0x374` bit 23 while it runs) holding
  a process of KH1's script machine (`0x1402BD420`: one of 32 process slots, flags at
  `0x142DE51E0`, a 0x800-byte stack each at `0x142DB7BE0`), stepped once a frame
  (`0x1402BCEC0`).
- The per-level attack data offsets (table `0x140528638`, 16-byte rows, Checked (exe)): Fire
  0x5F28 / 0x5F98 / 0x6008, Blizzard 0x6078.., Thunder 0x61C8.., Cure 0x6318.., Gravity 0x6468..,
  Stop 0x65B8.., Aero 0x6708 / 0x6778 (+ 0x5CF8) / 0x67E8 (+ 0x5D68); `docs/kh1-attack-elements.md`
  has the fields. Level 1 data seen in play (the tester's t463 log): Fire `1 2 3 3 -1 0x14 ..`
  (stat source 1 = max MP, element 2, reaction 3 / 3, bone −1 = free-standing, power 20),
  Blizzard `1 3 3 3 -1 0x16`, Thunder `3 4 3 3 -1 0x10` (stat source 3 = max MP × 2).
- The script syscalls (table `0x140529180`, 226 entries of (function, argument word), Checked
  (exe)) that touch attack objects:
  - 22 (`0x1402BBA00`): a free hit bit (`0x1402B3760`);
  - 23 (`0x1402BBA20`): **a new attack object** `0x1402B2DC0 (owner, hit bit, data)`, its
    `+0xA0` the process;
  - 47 (`0x1402BA250`): **pays the spell's MP** (`0x1402A5B60 (owner, data +0x30)`);
  - 76 (`0x1402BBAC0`): **puts an object at a point** (`0x1402B3790`: current and previous
    centres, start and end, the previous-valid flag: a jump, no sweep);
  - 175 (`0x1402BBBF0`): frees every object of an owner (`0x1402B3600`).
  So a KH1 spell is a script process moving its own free-standing attack objects; nothing in KH1's
  engine flies the shot (Assumption: the scripts write the centre `+0x10` through the machine's
  memory operations between the syscalls above; the `.mag` files aren't staged to confirm).

## 3. KH1's attack objects and its collision pass (Checked (decomp))

- Pool `0x142DB4710` (Epic `0x142DB5110`), 64 objects of 0xC0; live hit bits `0x142DB7710`.
  Constructor `0x1402B2DC0`: power from the data's stat source (0 Strength, 1 max MP, 3 / 4 / 5 max
  MP × 2 / 3 / 5, 2 none), element and reaction into `+0x30`, a negative bone makes it
  free-standing (`+0x30` bit 10, its activity off, the radius the data's `+0x5C`), else bound to the
  owner's bone. Free `0x1402B3560`; the wrapper with the Keyblade's bonuses `0x1402B3270` (Epic
  `0x1402B10E0`). Fields: `docs/kh1-reflect.md`.
- The pass `0x1402B4460` (a task, skipped while `0x142867364 & 0x4000`): `0x1402B47A0` walks the
  pool (activity: 1 only while `0x142867368` bit 0, 2 while the owner plays request `+0x58` with
  its hit-window level, 3 re-armed on an interval `0x1402B3380`; an inactive object dies unless
  bit 8), `0x1402B3800` places and sweeps each into sphere entries (previous centre to current,
  about a diameter a step) and meets every earlier entry (attack vs attack: the clash
  `0x1402B4A20` / `0x1402B4BC0`); then each hittable actor's body parts meet the attack entries
  (the contact, Epic `0x1402B1ED0`: a hit record from the data); then current centres become the
  previous ones and every hit-once mask (`+0x9C`, actors' `+0x3E0`) is ANDed with the live bits.
- An attack object's shape is a sphere (radius `+0xC`) swept along its move. KH2's spell attacks
  are spheres, ellipsoids (radius ≠ height: Firaga's ring 240 × 180) and are swept by KH2's own
  rule (copies one `min(radius, height)` apart): `docs/magic-model.md`. **Open** (question 3).
- Damage: the contact's record, then `0x1402BD8D0`: power + stat − Defense on KH1's damage curve,
  × the target's resistance to the element (Cure's element 11 heals the power, Aero's 12 a
  barrier). This branch lands KH2's hits as KH1's damage × KH2's power (`combat::scale_record`,
  Magic-based ones with max MP standing in: +10% a point × `magic_scale`).

## 4. KH2's casts on KH1's cast state (done: step 1)

- `kh2slots::CASTS`: KH2's cast motions (MAGC, `00battle.bin`, Checked (files): Fire 56 / air 58,
  Blizzard 59 / 61, Thunder 62 / 64, Cure 65 / 67, the same at every level; finishers 57 / 60 / 63)
  as slots on KH1's requests 0x36..0x39, 0x3D, 0x83..0x86, 0x8A (templates 54, 55, 56, 56, 57, 79,
  80, 81, 81, 82), pointed at by the Magic / Aerial Magic styles. No "may be left" point of the
  template's: KH2's cast motions have no free range (`P_EX100.mset`, Checked (files)), they hold
  him to their end, where KH1's queue takes him on.
- `combat::command`: a KH2 spell (Fire .. Cure) chosen in KH1's own states, in a state KH1 casts
  in, is KH1's command step's (no borrow) when `formmagic::native`.
- `formmagic::kh1_spell_start` (hook on `0x14028F140`, Steam only, prologue
  `48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 40`, Checked (exe)): with our slot on the request
  just asked for, KH1's script isn't started; KH2's is (trigger 20 on the next KH1 frame) with
  KH1's MP cost paid (`reflect::pay_mp`), KH2's MAGC voice (KH1's voice group off with KH2's
  battle quotes); Fire's second part (0x3D / 0x8A) is
  taken off KH1's queue. `takeover::native_spells` (each KH1 physics frame with a script of
  base Sora's running) runs the scripts' frame (`formmagic::shots_step`) with the slot's KH2
  motion and KH1's time in it; the release (trigger 21) is the clip's magic-cast event at KH1's
  time. KH2's turn to the lock (range 28: Blizzard 0..16) is `kh2slots::turn_to_lock`'s.
- Still ours in step 1: the hits (`combat::Strikes::spell_strike`), Cure's heal, Reflect.

## 4b. KH2's finisher casts (done: step 2)

- KH2 casts the MAGC finisher motion when the spell takes the combo's finishing step
  (`Character::cast` / `spell_slot`: a spell is one step of the combo string; the step is the
  finisher when it reaches the combo's length). Its sub is `+ 1` (the finisher's `magi` and ATKP
  rows) and its voice MAGC's finisher voice (Fire 11, Blizzard 12, Thunder 13), Checked (files).
- On KH1's combo (`formmagic::pick_cast`, before KH1's command step): the combo is live while the
  motion playing has KH1's combo window (`+0x1CC` bit 22), the step is KH1's count (combat info
  `+0x41`) + 1 when the last swing connected (`+0x50`) or with Combo Master, as KH1's Attack step
  counts (Checked (decomp), `0x1402A7510`), the length KH1's ground one (stats `+0xD4`). Then
  KH1's ground request of the spell (and Fire's 0x3D) points at our slot of KH2's finisher cast
  (Fire 57, Blizzard 60, Thunder 63; templates 54 / 55 / 56 / 57) for that cast. Cure has none,
  and KH2 has none in the air.
- The spell's step on KH1's count: §4c.

## 5. What step 1 leaves (the plan)

1. **The cast in KH2's combo string**: the spell taking a combo step (KH1's `+0x41`, the combo
   going on after it), and KH2's magic-only window (range id 35, Fire 56 from 60, into 71 34..46): KH1's
   leave point frees him for everything (question 1).
2. **KH2's cast physics**: KH2 stops him at a cast (`vel_h` 0; in the air no rise or fall, then
   its air-magic gravity); KH1's air cast keeps 0.7 of the sideways speed and KH1's gravity
   (`0x140386CD0 (+0x3C0, 0.7)`). Through the physics setting (`shared::physics`) (question 2).
3. **Spell attacks as KH1 attack objects**: each script attack a free-standing object of Sora's
   in the pool (KH1's spell data for its element, our own data copies per KH2 ATKP as `kh1rows`
   does, `+0x10` / `+0x70` set from the script's position each frame), KH1's pass hitting them,
   `scale_record` taking KH2's ATKP row (its power, re-hit row by count); hits told back to the
   script (`attack_is_hit`), walls by KH1's wall test (`0x1402B1870` Epic). Shapes: question 3.
4. **Reflect**: the barrier (ATKP 573, kind reflect) as a KH1 guard object on Sora (priority 100,
   a guard clash type: KH1's attack-vs-attack pass meets enemy attacks with it, so each block is
   KH1's clash and a free-standing enemy shot under priority 98 is turned back by KH1's own
   reflect, `docs/kh1-reflect.md`), its count from the clashes, the counter / burst as step 3's
   objects (question 4).
5. **Cure's heal**: KH2's `ceil(power × Magic / 100)` (ATKP kind 6) through KH1's heal (element 11)
   on Sora (question 5).
6. Thunder's and Cure's turn (range id 22, 62 / 65 frames 0..16): KH2's range handler
   (`0x14040D810`, Checked (decomp)) turns id 22 through `0x1403D4B90` toward the object's own
   point at `+0xBF8` and id 28 through `0x1403D4B10` toward the player's primary target
   (`+0xDC8`); both with `0x1403D3A90` (past 50 away, twice the parameter a frame). Who writes
   `+0xBF8` isn't read yet (Assumption: the spell's target).
7. **MP**: KH2's costs (03system `cmd`, Checked (files)): Fire 12, Blizzard 15, Thunder 18 at every
   level, Cure 255 (all of it), Reflect 10, from KH2's MP pool; KH1's MP is whole points and every
   cast here costs 1 (`reflect::COST`, KH1's own check and payment) (question 6).

## 6. KH2's MP costs on KH1's MP (done: the tester's answer 6 (b))

- KH2's costs (03system `cmd`, **Checked (files)**): Fire 12, Blizzard 15, Thunder 18 at every
  level, Cure 255 (all of it), Reflect 10; KH2 Sora's MP is 100 (00battle `plrp`, character 1,
  **Checked (files)**).
- KH1's MP (**Checked (decomp)**, Steam): stats `+0x44` whole bars of `+0x48`, and the charge part
  `+0xBE` (u16) filling toward the next bar, which comes at `(n + 1) × 30` (`0x1402A4CB0`). The
  payment `0x1402A5B60 (actor, cost)`: from 100 up `cost / 100` whole bars; below 100 `cost`
  charge units, the top bar breaking into its own worth (`n × 30`) when they run out (skipped while
  `0x142867364 & 0x800`, or for party members past 9). The test `0x1402A4C10`: from 100 up
  `cost / 100 <= bars`; below 100 `cost <= Σ k × 30 + charge`. So bar `k` is worth `k × 30` units.
- `mpcost`: base Sora's KH2 spell costs `KH2 cost / 100` of his KH1 maximum, in bars, taken from
  the top down (the charge first at the next bar's worth, then each bar at its own), paid through
  KH1's payment 30 units at a time (each step breaks at most one bar, as KH1's own payment
  does). Cure takes every unit, and needs one. Drive Forms keep 1 MP a spell.
- KH1's command step judges a cast with its usable check `0x14028EFD0` (the spell's attack data
  `+0x30`, slot `0x142D22B80 + spell × 0x38 + 0x18`): for a native KH2 cast `combat::command` checks
  KH2's cost first, then has the data read it (in units, at most 99) for that one step, put back
  after. **Open:** the Magic menu still greys KH1's spells by KH1's own cost (that check, called from
  the menu fill), so a spell KH2's cost allows may show grey with less than KH1's cost left.

## 4c. The cast as a step of the combo (done: the tester's answer 1 (a))

- KH2's rule (the controller's Magic action and `spell_slot` / `take_spell_slot` in
  `character_core`, the KH2 source): a cast takes the combo's next step (the first of a new
  combo when none is going; in the air every step counts), the step after it counts on as after
  a connected swing, and the combo goes on; from the cast motion's reaction-command window
  (range id 14) other actions may cut it and an Attack goes on with the combo (with none, only
  its last frame); a spell only in its magic window (range id 35); a finisher cast with no
  finisher to come ends the combo.
- The windows (`P_EX100.mset`, **Checked (files)**, KH2 frames at 60 per second): range 14 Fire
  56 from 30, Blizzard 59 from 24, Thunder 62 / Cure 65 from 26, air 58 from 36, 61 from 44,
  64 / 67 from 38, finishers 57 from 36, 60 from 48, 63 from 42; range 35 Fire 56 34..46 (into
  71) and from 60 (into 56), Thunder 62 / Cure 65 from 42 (into 62), none on Blizzard 59, the air
  casts or the finishers. (Range 35's parameter names the cast it leads to; the controller, and
  so this port, takes any spell there.)
- On KH1's side (`kh2slots::cast_follow`): our cast slots get KH1's combo window (event type 5,
  `+0x1CC` bit 22: KH1's menu takes a command there with the action flag still set) from range 14
  to the end; the finisher casts none. No "may be left" (type 0 clears the action flags, and the
  stick would then walk out of the cast; KH2's takes only presses): a fresh jump or Square there
  ends the cast's action flag (`+0` bit 2) before KH1's control step reads it
  (`formmagic::cast_cut`, from `takeover::shared_control`), so KH1's jump, or the Square move KH1
  or a borrow of KH2's has, starts at once. `formmagic::take_step` (at the native spell start):
  KH1's count (combat info `+0x41`) set to the cast's step and `+0x50` (the swing's "connected"
  flag, which the Keyblade attack wrapper `0x1402B3270` points its attack object at and clears,
  **Checked (decomp)**) set, so KH1's Attack step (`0x1402A7510`: in the combo window one more
  when `+0x50` is set or Combo Master is equipped, else no swing; outside it 1, **Checked
  (decomp)**) counts on from the cast; the combo's last step sets the count to 0 (the next
  counted Attack is a new combo's first). `combat::command` refuses a spell during our cast
  before its range 35 (`kh2slots::cast_magic_window`), with KH1's "can't" sound.
- **Open:** Finishing Plus after a finisher cast (KH2's next finisher in range 50) isn't built;
  a finisher cast ends the combo.
