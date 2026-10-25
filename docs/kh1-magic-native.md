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
- **The cast motion before KH1 plays it** (the tester, kh2-effects-in-kh1s-own-effect-system:
  "kh2s magic doesn't get casted the first time you input said magic"; since t477). KH1 only
  queues the cast's request in the command step; its animation step plays it from the next frame
  (the trace: KH1's slot still the idle on the cast frame, our slot at a time below 0 the frame
  after). KH2's spell scripts end a cast at once when the caster isn't in the cast motion on their
  first run: `FIRE_3.mag`, `BLIZZARD_3.mag` and `THUNDER_3.mag` run with `kh2-magic` and Sora's
  motion other than 56 on the first frame (0, or none) end on that frame with no release and no
  attack; with 56 from the first frame they release and make their attacks (502; 248; 249 / 250)
  (**Checked (files)**). KH2 sets the cast motion before the script first runs. A second cast in
  the same combo worked because the cast motion was already playing (the tester's t515 log: every
  combo's step 1 made no hit, steps 2 on hit 24..26 frames in). `formmagic::shots_step` now gives
  the scripts Sora in the cast motion while the cast hasn't been seen playing (`casting`'s
  `SEEN`), only in the scripts' world (`KhWorld`), not in KH1.

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
2. **KH2's cast physics**: through the physics setting (`shared::physics`) (question 2). The
   numbers once written here (KH2 stops him, KH1 keeps 0.7) were wrong: see §4e.
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
  after. (Before t570: the Magic menu greyed by KH1's own cost.)
- **t570: KH2's cost kinds in KH1's engine** (the tester, m6 answer 1). KH1's MP check
  `0x1402A4C10` and payment `0x1402A5B60` are hooked to take two more kinds of cost besides KH1's
  (charge units under 100, bars × 100 from 100 up), tagged in the cost's high bits: "all MP, at
  least N whole bars" (`0x40000000 | N`, KH2's Cure, N = 1 by the tester's design) and "a share
  of max MP" (`0x20000000 | percent`, the scale above). KH1's usable check `0x14028EFD0` (the
  Magic menu fill `0x1402944C0`, the shortcuts and the command step's casts all go through it,
  **Checked (decomp)**) is hooked: for base Sora's Fire .. Cure whose Config row is KH2 the
  spell data's cost reads the KH2 kind for that one call, so KH1's own menu greys and refuses by
  KH2's cost, and by KH1's with the row at KH1. Our KH2 casts pay with the same kind through
  KH1's payment. Prologues **Checked (exe)**. Cure's kind pays the charge part in steps under
  100 (no bar breaks) and then every bar as one bars × 100 cost (repeated while KH1's MP change
  `0x1402A4A00` leaves one: its halving rule).
- KH2's own rule (**Checked (decomp)**, KH2 Steam): the magic command's state (`0x140401D60`,
  case 2) is 5 (greyed) while MP Charge runs, else 2 with MP under the cost, and its callers take
  any state under 4 as usable (`0x1403A9EA0`), unless a per-command flag greys it (`0x1403C5940`,
  bit table `0x1409ACFF4`, **Assumption**: not identified). The payment `0x1403D3CF0` going below
  0 sets MP to 0 and starts MP Charge. So in KH2 Cure works with any MP left, and every spell can
  be cast with less MP than its cost (emptying the pool). Ours: Cure needs one whole bar (the
  tester's design); the other spells still need their full cost (asked); no MP Charge (its own
  thread, `docs/kh1-kh2-mp.md`).

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

## 4d. KH2's stop at a cast (done: the tester's answer 2 (a), physics KH2; Drive Forms as they are)

- KH2's cast start (the controller's `cast`, the KH2 source): sideways speed 0, and in the air no
  rise or fall (the arc a fall from rest); during the cast the air drift keeps the speed at 0 and
  the cast motion's no-gravity range (range id 3: air Fire 58 16..74, Blizzard 61 16..66, Thunder /
  Cure 64 / 67 16..58, **Checked (files)**) holds him; before it he falls from rest at KH2's
  gravity, after it on.
- KH1's air cast: **corrected in §4e** (it halves the sideways speed and starts a 30-unit hop;
  the 0.7 and "leaves the jump's arc alone" written here were wrong). The ground cast
  (`0x1402A3DB0`) sets no speed.
- `shared::physics::cast_stops` (from `formmagic::kh1_spell_start`, KH1's air state 2, physics
  KH2, base Sora): KH1's sideways speed 0 and its arc a fall from rest where he is, and KH2's arc
  for KH1's air step the same; the air step (`air_step`) then runs it with KH2's gravity and air
  drift, and the no-gravity range holds him as before. On the ground nothing is changed
  (**Assumption:** KH1's ground step moves no actor while its action flag is set).

## 7. Spell hits as KH1 attack objects: the shape choice (the tester's answer 3), not built yet

- The tester: (b) several spheres acting as one hit if it isn't costly, else (a) one sphere.
- What KH1's pass costs (`0x1402B4460` → `0x1402B47A0` / `0x1402B3800`, **Checked (decomp)**): each
  live attack object of the 64-object pool is placed and swept from its previous centre into
  sphere entries (`0x1402B3A00`, about one diameter a step) and every entry meets every earlier
  one (attack against attack, the clash) and every hittable body part. A KH2 ellipsoid covered by
  a row of spheres along its long side (Firaga's ring, 240 × 180: 2 to 3 spheres) adds a few
  entries a frame; at the pass's sizes that is tens to a few hundred sphere tests a frame in
  KH1's own native code (microseconds). **Assumption** (not measured in game): the cost on the
  game thread is negligible next to the KH2 script frame that already runs (`formmagic::shots_step`).
  The binding limit is the pool's 64 objects (the enemies' attacks share it), not time: so (b),
  with the spheres of one KH2 shape sharing one hit-once bit (`0x1402B3760`) so they hit as one.
- To build (§5 step 3): each script attack's spheres as free-standing objects of Sora's
  (`0x1402B2DC0` with KH1's spell data for the element), placed each frame as KH1's own script
  does (`0x1402B3790`: current, previous, start and end centres, bit 16 "previous valid", bit 14
  cleared), freed with the attack (`0x1402B3560`); the hit's numbers from the KH2 row in
  `combat::scale_record`; the hit told back to the script (`attack_is_hit`); KH2's re-hit
  interval and combo-group count against KH1's hit-once mask (`+0x9C`, ANDed with the live bits
  each pass) and its interval activity (3, `0x1402B3380`). Still to read: the sweep test
  `0x1402B3D50` and whether one owner's own spheres meet each other in the attack-against-attack
  pass (the pairing table `0x1405299B8` by team).

## 4f. Reflect on KH1's cast state (done: the tester's m5 plan item 2)

- KH1's command step for a spell (**Checked (decomp)**, Steam): `0x1402A7510` runs the pre-step
  `0x1402A38E0 (actor, record, dir)` (a turn to the command's target when its type has one) and
  then `0x1402A3600`. Ground (states 0 / 6 / 0x18, `0x1402A3DB0`): usable check, the cast request
  `(0x36 + spell) | 0x1000000` with blend 0, the idle queued, the spell start `0x14028F140`, then
  the move direction copied normalised (`0x1402A93D0`) with its length `+0xC` 0. Air (state 2):
  usable check, `(0x83 + spell) | 0x1000000`, the fall queued (request `0x40000006`, blend 8.0),
  the spell start, the hop `0x1402AF960 (actor, btltbl +0xE8)` and `+0x3C0 ×= btltbl +0xF0`
  (`0x140386CD0`). KH1's request ids for spell 7 would be Fire's second parts (0x3D / 0x8A), so
  Reflect can't go through KH1's own branch.
- `reflect::cast_native` does exactly those steps with our requests (`kh1rows::OURS`: "Reflect"
  motion 71 on template 56 / request 0x38, "Reflect (finisher)" 72 on the same, "Reflect (air)" 73
  on template 81 / request 0x85; request only, KH1's template sounds left out), KH2's start in
  place of KH1's spell start (cost `mpcost`, the combo step `formmagic::note_step` / `take_step`,
  the finisher cast 72 on the ground combo's last step, its voice), and the physics setting's
  air start (`shared::physics::cast_stops`). The slots are cast slots (`kh2slots`: KH2's combo
  window from range 14, none on the finisher, its cut and magic windows), so Reflect follows the
  same combo rules as the other native casts.
- The script: `takeover::native_spells` runs `reflect::native_step` each KH1 physics frame while
  a Reflect runs, with the slot's KH2 motion and KH1's time in it (its magic-cast event at frame
  0 releases the barrier; up to 4 frames are allowed for KH1 to start the slot, as `formmagic`'s).
  Sora is untouchable (KH1's flag 0x40) through the cast motion and the barrier, as before.
  `reflect::kh1_frame` no longer ends a Reflect while base Sora is in KH1's own states.
- Landing mid-cast (the tester's m5 answer 2, "the jump config controls what landing animation
  you get currently so follow that"): KH1's own landing (`0x1402A43A0`, request
  `(+0x3D0) + 7`, the hop having set `+0x3D0` to 0) cuts the cast; its slot 11 is drawn by the
  Jump row (`shared::slot_kind`), as for the other native casts.
- Still the borrow: Drive Forms, Epic. (KH1's flight, state 8, since t571, below.)
- **t571: Reflect in KH1's flight** (the tester, m6 answer 2 (a)). KH1's flight cast
  `0x1402A4150` (**Checked (decomp)**): the usable check; with a target in the command record
  (`+4` handle, `+8` lock point) the move direction is aimed at the point (`0x1402B04C0` the
  point, `0x140386D10` point − position) and the stats' `+0xD0` set to π/16; the air cast request
  `(0x83 + spell) | 0x1000000`; KH1's flight motion queued after it (`0x40000075`, blend 8.0);
  the spell start; the move direction normalised with no length. No hop and no halving (those
  are the state-2 branch's). `reflect::cast_native` now does exactly that in state 8 with our air
  request (motion 73), the rest of the cast as in state 2 (cost, combo step, script on KH1's
  frames). KH2 has no flight, so neither Air Physics setting changes it.
- **Drive Forms** stay on the borrow: a form runs wholly on KH2's controller (`shared` is off in
  a form, `docs/kh1-drive-forms-native.md` §1), so there is no KH1 cast state under him to cast
  on; it moves when the forms themselves do (the drive-forms-native thread).

## 4g. KH2's air cast start at physics KH2, none of the clip's hover at physics KH1 (done: m5 plan item 3)

- Physics KH2 (base Sora; Drive Forms as they are): `shared::physics::cast_stops` (the native
  spells from the spell start hook, Reflect from `reflect::cast_native`) is now KH2's start, not
  t487's stop: a hop of 40 (sstm `magic_jump`, **Checked (files)**) from where he is on KH2's arc
  (rise time `sqrt(2h / g)`, 14 frames), the sideways velocity kept, which `air_step` then drifts
  × 0.95 a tick (`air_drag` in a move; **Assumption:** KH2's stick target is empty during a cast
  motion with no allow-movement range). KH1's command step halves `+0x3C0` and writes its own 30
  hop after the spell start (**Checked (decomp)**), so `cast_after_command` (after KH1's command
  step, `combat::command`; at the end of `cast_native`) puts the kept velocity and KH2's arc back.
- Physics KH1: KH1's own air cast has no hover (`0x1402AF6A0`, **Checked (decomp)**), but our cast
  slots' KH2 clip range 3 held him there (`air_step`'s `held` applied at either physics). For a
  cast slot (`kh2slots::cast_playing`) at physics KH1 the hold is now off, so the 30 hop, half
  speed and KH1's fall are KH1's all through. Other KH2 motions (Aerial Dive, Finishing Leap) keep
  their hold at physics KH1 (the tester, m5 answer 1: "(b) unless you have kh1 physics on").

## 4h. KH2's hold with its clock running (done: m5 plan item 4, the tester's answer 1 "(b) unless you have kh1 physics on")

- `shared::physics::air_step` at physics KH2: while our slot's KH2 clip holds him (range id 3 /
  24, `kh2slots::no_gravity`), KH2's arc time goes on and only the move is 0, as ACTION_JUMP's step
  `0x1403CB360` does (**Checked (decomp)** KH2 Steam); after the hold he falls at once at the fall
  cap. For every KH2 motion on KH1's air state: the casts (Fire 58 16..74 ... Reflect 73
  16..74), Aerial Dive (46..54), Finishing Leap (its root's rise in the hold).
- Physics KH1: unchanged for the moves (the clock stops in the hold, the fall starts from the arc
  where it was); none for the casts (§4g).
- Not changed: KH2's controller (Drive Forms, borrows) still stops the clock in a hold
  (`character_core`'s physics, `fo.no_gravity`).

## 4e. Air casts read again: KH1's and KH2's physics (research, the tester's m4 follow-up)

The tester (after t484..t487): "reflect currently does not behave properly for kh1 physics, please
see how casting is done in the air physics wise to get the same for kh1, as for kh2 please check the
c steam decomp to fix and find any issues with the physics". Steam addresses throughout.

### KH1's own air cast (**Checked (decomp)**, values **Checked (files)**)

- KH1's command step for a command in movement state 2 (`0x1402A3600`, the air branch, command type
  2): MP check `0x14028EFD0`; the cast motion `0x14029F980 (actor, (0x83 + spell) | 0x1000000)`
  (Fire: `0x1000083`, then `0x8A` queued); KH1's fall queued after it, `0x14029FA30 (actor,
  0x40000006, 8.0)`; the spell start `0x14028F140`; **then** `0x1402AF960 (actor, btltbl +0xE8)`
  and `+0x3C0 ×= btltbl +0xF0` (`0x140386CD0`).
- btltbl's block (pointer `0x142D22D30` = file + 0x58, `local/re/kh1cmp/orig/btltbl.bin`, the live
  copy the same): `+0xE8` = **30.0**, `+0xF0` = **0.5**.
- `0x1402AF960 (actor, h)`: a fresh hop of `h` from where he is: `+0x3B0` (time) 0, `+0x3B8`
  (time to the top) `sqrt(2h / 0.48469)` = 11.1 frames for 30, `+0x3BC` = −h (y down), `+0x3B4` =
  his y, `+0x3D0` (landing motion base) 0, state 2 (`0x1402B1960`, no change: already 2), flag
  `0x10000` off. So KH1's air cast throws away the jump's arc (a cast mid-rise does not go on
  rising) and pops him 30 up.
- KH1's air step (`0x1402AF6A0`, state 2): the rise on KH1's double sine to the top, then
  `0.48469 × (t − top)² / 2` below it, at most 20 a 60 Hz frame; the apex swap `0x1402AF630` only
  for KH1's jump motion (`+0x164 == +0x3D0 + 4`), not the cast's. Sideways: with no stick speed
  (`+0x2C <= 0`) `+0x3C0` × stats `+0x18` (its square root at 60 fps); with one, toward the move
  direction (stats `+0x14` × speed, capped). No hover anywhere.
  **Assumption:** `+0x2C` stays 0 during the cast (KH1's input step `0x1402A7B30` isn't run while
  the action flag is set, `0x1402A6220` / `0x140297870`); to trace.
- Landing (`0x1402A43A0`, the state change 2 → 0): KH1's landing request `(+0x3D0) + 7 |
  0x1000000` (motion 7) and idle queued: the cast motion is cut there.

### Our air casts at physics KH1 (code)

- Fire / Blizzard / Thunder / Cure native (t477): KH1's own path, so the above exactly
  (`cast_stops` returns at physics KH1).
- Reflect is still KH2's controller (`reflect::before_step` → `Character::cast`): sideways 0 (KH1:
  × 0.5), no hop (KH1: 30 up), a fall from rest at KH1's gravity × `air_cast_fall` capped 20 (KH1:
  after the hop), KH2's `air_drag` (KH1: its own drag), landing into KH2's Land (KH1: its landing
  7). The tester's "doesn't behave properly for kh1 physics".

### KH2's own air cast (**Checked (decomp)** KH2 Steam, **Checked (exe)**, **Checked (files)**)

- The cast start is `0x1403C5170 (actor, command, finisher)` (MAGC by command `+0x26`; the
  finisher motion `+0x2C` when asked, else ground `+0x28`; voice `+0x34`/`+0x35`; sub `level × 2
  − 2`, `+1` finisher, `+10/+20/+30` by form). Airborne (`0x1403BA9A0`), before the air motion
  (`+0x30`) it calls the current action's slot 4 with xmm2 = sstm `+0x48` (exe at `0x1403C5244`;
  sstm `+0xD0` instead when actor `+0x6C8` bit 7 and `+0xDE0 == 7`), and queues motion 4 (fall,
  A004) after the cast (`0x1403C8310`).
- sstm `+0x48` is `magic_jump` = **40.0** (+0xD0 = 30.0), `03system.bin` (**Checked (files)**).
- Slot 4 is `0x140409B00` in every player action's vtable (IDLE `0x1405CCEB8`, JUMP `0x1405CCF10`,
  ...; names from the exe's RTTI): `0x1403CBC30 (actor, 40)` then `0x1403B4DA0 (actor, 1)` (action
  JUMP, whose enter `0x1403CBD30` makes the arc's base his height). `0x1403CBC30` with h ≥ 0: a hop
  of h (`+0xFC` = −h, time `+0xF0` 0, top `+0xF4` = `sqrt(2h / 0.40816)` = 14.0 frames for 40), fall
  motion swap off (`+0x118` −1), landing motion 5 (`+0x11C`), launch speed `+0xEC` 0 so the
  velocity `+0x20` is **kept**.
- ACTION_JUMP's step `0x1403CB360`: the sideways velocity through `0x1403B7AD0 (dt, 0.9, 0.95)`:
  toward the stick target `+0x10` (its length `+0x1C`) keeping 0.9 a tick, else × 0.95 a tick.
  **Assumption:** the stick target is empty during a cast motion with no allow-movement range
  (not traced: the player's input selector), so he drifts at × 0.95.
- Vertically: the arc's time `+0xF0 += dt` every step **before** the hold test; with motion flag
  `0x20` (range id 3: `0x14040D810` sets `+0x34 |= 0x20` of the motion player at actor `+0x158`, so
  actor `+0x18C`) or `0x40` (range 24) the step moves him by 0, but the clock goes on. After the
  hold the arc is far past its top, so he falls at once at the step's cap (`dt × +0x138`;
  **Assumption:** `+0x138` is the fall cap, 16 a frame from sstm `fall_max`).
- Motion 73 (Reflect air, A371, the same clip as Fire's 58; **Checked (files)** `P_EX100.mset`):
  84 frames at 60 Hz, range 3 (hold) 16..74, range 14 from 36, magic-cast event at 28. So: rise 40
  over 14 frames (from wherever he was, rising or falling), a frame or two past the top, held
  16..74, then falling at the cap; a fall loop (4) after 84.
- Landing mid-cast: ACTION_JUMP's landing slot `0x1403CB940`: KH2's landing motion 5, idle queued,
  action IDLE (unless the clip has range 49, which motion 73 hasn't): the cast is cut.

### What differs at physics KH2 (ours, t487, and Reflect's controller path)

| | KH2 (exe) | ours |
|---|---|---|
| sideways at the start | kept, then × 0.95 a tick | 0 |
| vertical at the start | a 40 hop from his height (14 frames up) | fall from rest |
| before the hold (0..16) | rising to 40, at the top | falling from rest, about 52 down by frame 16 |
| the hold (16..74) | height held, arc clock running | height held, arc clock stopped |
| after the hold | at once at the fall cap | a fall from rest again |
| landing mid-cast | KH2's landing 5, cast cut | native: KH1's landing 7 (cut); Reflect: the cast plays on (`air_magic_falls` off) |

- Order on the native path: `cast_stops` runs inside the spell start (`0x14028F140`), and KH1's
  command step then runs `0x1402AF960` (hop 30) and × 0.5 over it; t487 still works because our air
  step drives the height from its own arc (`shared::physics::AIR`) and the speed was 0 before the
  halving. A KH2 start that keeps the speed must be applied after KH1's command step returns.
- The hold rule in `shared::physics::air_step` (`kh2slots::no_gravity`) stops the arc's clock for
  every KH2 motion with range 3 (Aerial Dive's 46..54, Finishing Leap): KH2's runs it on. Changing
  that for casts only is one system; for the ability moves another (asked).

### Cure KH1 (t485) against "exactly KH1"

- t485's Cure KH1: KH1's Cure (its script, effects, heal) on KH2's Cure cast motion: requests 0x39 /
  0x86 are redirected to our KH2 slot whenever Magic / Aerial Magic is KH2 (`kh2slots::redirect`, by
  the Spec's kind), and `takeover::command_started` puts it on that motion's clock (`base_look`).
- Exactly KH1's: requests 0x39 / 0x86 left to KH1's own slots (56 / 81) when the Cure row is KH1
  (the redirect also asking `formmagic::kh2_cure`), and `base_look` false for Cure then, as the
  debug menu's `kh1_cast` does for Gravity / Stop / Aero. KH1's own MP cost then (mpcost is only
  for KH2's spells).

## 4i. Air casts by the Air Physics row, not the Physics row (the tester, kh2-effects-in-kh1s-own-effect-system m19)

"we'll remove ground magic and change air magic to air physics kh1 | kh2 so now you can cast kh2
fire with kh1 air physics like how we did for reflect. remove anything related to the main
physics setting that has magic in it and put it in this new air setting."

- Config > Combat > Magic > Air Physics (`styles::Kind::AirPhysics`, ini `magic_air_physics`),
  read through `shared::physics::cast_rule` (Drive Forms KH2's, as for the Physics row).
- What moved from the Physics row to it (every place the Physics row decided something about a
  cast, found in the code):
  - `cast_stops` (§4g: KH2's 40 hop and kept speed at a cast's start, or none: KH1's own 30 hop
    and half speed);
  - `air_step`'s while a cast plays in KH1's air state (`casting_in_air`: the action flag with
    KH1's air cast request `0x83..0x8A` applied at `+0x164`, **Checked (files)** the request
    table; or one of our cast slots): KH2's arc, gravity and drift, and the cast motion's
    no-gravity hold with the arc's clock running (§4h), or KH1's own air step (no hold);
  - the KH2 controller's borrowed air casts (`takeover`'s `air_magic_falls`: KH2's own, or KH1's
    fall).
- New: KH1's own spells in the air take the row's start too (`formmagic::spell_start` calls
  `cast_stops` for them before KH1's command step halves the speed; `cast_after_command` after
  it, as for KH2's), so KH1's Fire and KH2's Fire move him alike at either setting.
- A cast at Air Physics KH1 under Physics KH2 drops our arc (`off_air`), so KH2's fall after the
  cast goes on from KH1's arc fields rather than the jump before it. Under Physics KH1 at Air
  Physics KH2 the fall after the cast is KH1's own from the fields KH2's arc wrote.
- **Assumption**: KH1's halving of the sideways speed and its 30 hop come after the spell start
  (`0x14028F140`) in `0x1402A3600`, as §4g's `cast_stops` / `cast_after_command` already rely on.
- With a KH1 cast animation (no slot of ours) there is no KH2 hover to hold him at Air Physics
  KH2: his 40 hop, kept speed and KH2's fall only (open with the tester).
