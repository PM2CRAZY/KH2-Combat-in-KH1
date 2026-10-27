# Aerial Dodge native (KH2's Aerial Dodge on KH1's air state)

Research notes, testing branch (t480 round). No code changed yet. Addresses are the **Steam**
builds' (Ghidra, image base 0x140000000) unless marked Epic. Plain-language notes only.

## 1. What we have now

- The jump pressed in the air, with Aerial Dodge (ours, ability 67) equipped and not used this
  air time, is a **borrow**: `shared::control` sets `AIR_JUMP` (KH1 sees none of the jump /
  action bits that step), `takeover` hands Sora to KH2's controller in the air (`AIR_START`,
  `AIR_MOVES`, `JUMP_NOW`), KH1's state is set to 0, and the controller's `try_jump` runs the
  dodge (`character-core/src/controller.rs`): a held arc of `air_dodge_height`, `air_dodge_speed`
  along the stick (none with it neutral), facing the stick, the launch stored (`launch_vel`),
  motion 202. KH2's controller then has him until two frames of its plain fall, then gives him
  back into KH1's fall state (`shared_give_back`, stage 315's fall request 0x06). **Checked (code)**
- Once per air time: `shared::AIR_DODGED` / `DODGE_JUMP_UP` (also used to hold back KH1's glide
  until after the dodge, stage 319). **Checked (code)**
- No invulnerability (`AIR_DODGE_STAR` is defined but nothing uses it now); the clip's guard
  window (ATKP 406) becomes KH1 guard objects through `combat::guard_step`, which runs on KH2's
  controller's `Output`. **Checked (code)**
- Known side effects of the borrow (AGENTS.md stages 315-320): a frame of KH1's jump slot / glide
  slot at the dodge's start, the fall request at the hand-back, the "action running" bit, the
  glide request on the first frame. All come from handing Sora between two controllers.
- The model for going native: **Aerial Recovery** (t475, `combomods::recover`): our own KH1
  request (`kh1rows::OURS`, request `0x200 + n`, a `kh2slots` slot of the KH2 motion), asked with
  `0x14029F980 (actor, request | 0x1000000, 0)`, KH1's fall queued after it (`0x14029FA30 (actor,
  0x40000006, 0)`), KH2's hop on KH1's air fields (`shared::physics::kh2_hop`, state 2 only),
  facing written (`+0x30`, `+0x150`), `mark_air_dodged`. The air combo's own hops use the same
  `kh2_hop` (`combomods::air_hop`, t391 / t435). **Checked (code)**

## 2. KH2's Aerial Dodge (Steam KH2 exe)

- **`0x1403CB9F0`**: the jump-press handler of the air actions (in 17 action vtables: the jump,
  fall, glide and others). **Checked (decomp)** for what it does; which slot of the vtable it is
  is an **Assumption** (it is referenced only as data).
  - Level: `0x140401680 (player, 2)`: byte `+0x22` of the player's growth levels (Aerial Dodge is
    growth 2), or **5** when `0x1403ABCD0` says so (world 0x12 room 0x17, or flag bit 13 of
    `0x142A11400`: a special place, not KH1's concern). **Checked (decomp)**
  - Allowed when the level is 5 (no limit), or the level is non-zero and the "dodge left" bit
    (`+0x124` bit 1) is set. Used: that bit is cleared. **Checked (decomp)**
  - The bit is set by the air actions' shared enter (`0x1403CBD30` and its copy) only when the
    previous action wasn't airborne (the action's `+0xC` bit 0). **Checked (decomp)** (So a
    dodge, glide or recovery doesn't give it back; whether the plain fall uses this enter, i.e.
    whether walking off a ledge gives the dodge, I did not check: `docs/movement-model.md` says it
    doesn't, from the Epic exe.)
  - The growth record (`0x1403E92D0 (level)`, table `0x142AE5778`, KH2's `fmab`): `+4` the dodge's
    height, `+8` its speed. Copied: the move vector (`+0x10` of the input), its y replaced by the
    height and its length (w) by the speed when the stick is tilted (0 stays 0: neutral is
    straight up). **Checked (decomp)**
  - Motion **0xCA (202)** asked (`0x1403C86A0`), the action's start (vtable `+0x18` of the action
    object, `0x1404AD270 (+0xC)`) given that vector; apex motion `+0x118` = **4** (the fall),
    landing motion `+0x11C` = **5**, flag `+0x120 |= 0x10000`. **Checked (decomp)**
  - The launch (`0x1403CBB00`, Checked (decomp)): `T = sqrt(2|h| / 0.40816328)` (KH2's gravity
    20/49), the sideways velocity normalised times the speed and stored as the launch velocity
    (`+0xE0`, flag `+0xEC` = 1, what the glide later takes back, stage 331). That the action's
    start negates the height into KH2's y-down before the launch is an **Assumption** (the
    launch rises only for a negative height).
  - The shared enter turns `+0x120` bit `0x10000` into `0x20000`; that `0x20000` is the "held
    jump" (early-release) flag is an **Assumption** (our controller already treats the dodge as a
    holdable arc, `start_held_arc`).
- **The numbers** (`03system.bin` pref `fmab`, the user's install, **Checked (files)**):

  | Level | Height | Speed (per frame) |
  |---|---|---|
  | LV1 | 145 | 18 |
  | LV2 | 155 | 24 |
  | LV3 | 200 | 30 |
  | MAX | 300 | 36 |
  | (5, the special case) | 1700 | 36 |

- **Motion 202** (`P_EX100.mset`, A160, **Checked (files)**): 30 fps keys 0..6, loops from 2;
  "air free" (range id 1) from frame 4 (60 Hz); a guard window, ATKP **406** on collision group 6
  (his body), all through; sound **999019** at frame 1 (frame trigger 8, bank 999 entry 19). No
  hitbox, no invulnerable window.
- **ATKP 406** (`00battle.bin`, Checked (files)): kind 2, power 0, refact self 2 (the move goes
  on, no guard reaction): a guard that stops hits with no effect.

## 3. How KH1 does the matching thing (Steam KH1 exe)

KH1 has no double jump. What a native dodge runs on:

- **Air state 2**, state table `0x142DB3840` (Epic `0x142DB4240`, translated through
  `steam_map.in`), 0x40 a state (`docs/kh1-physics.md` §3). **Checked (decomp)** earlier.
- **The launch** `0x1402AF960 (actor, height)`: `+0x3B0` (air time) 0, `+0x3B8` (rise time) =
  `sqrt(2|H| / 0.48469388)`, `+0x3BC` = -H, `+0x3B4` = his height, **`+0x3D0` (the jump's motion
  base) = 0**, state 2 (`0x1402B1960`), flag `0x10000` off. **Checked (decomp)**
- **The apex** `0x1402AF630`: 6 frames before the apex, if no request is pending (`+0x178` 0) and
  the request playing (`+0x164`) is the base's rise (`+0x3D0 + 4`), it asks the apex
  (`+0x3D0 + 5`) and queues the fall (`+0x3D0 + 6`). **Checked (decomp)** So a request of ours
  doesn't get KH1's apex swap: we ask the fall ourselves (as Aerial Recovery queues it).
- **State 2's pre-step** `0x1402AFC20`: the grabs (vine `0x1402C78C0`, ledge `0x1402C8A50`, bar
  `0x1402C9FB0`, hang `0x1402C3960`, water `0x1402C15C0`), skipped while the rise runs and 6 frames
  past its apex, in the 15 frames after jumping off something (flag `0x10000`), or with the
  action flag 4; else it faces the stick (`+0x150` = atan2 of the move vector). **Checked
  (decomp)** So during a native dodge's rise KH1 doesn't grab, as during its own jump (the
  borrow's own grab tries, `takeover`, stop applying).
- **State 2's jump press** `0x1402AFD60` (the table's `+0x38`): Neverland's flight
  (`0x1402B1EC0`, world flag `0x142867364` bit 0x20) or KH1's glide (`0x1402B26C0`, ability bit
  `0x80` of `0x142D5EC10`). **Checked (decomp)** The dodge must take the press before this (as
  `shared::control` already hides it).
- **Requests**: `0x14029F980` set, `0x14029FA30` queue (the ones `combomods::recover` calls).
  **Checked (code)**, Steam addresses from the decomp.
- **What reads it**: nothing of KH1's counts air jumps (no tally or event reads a jump in the air:
  **Assumption**, not searched exhaustively; KH1's attack tally `0x14028C660 (0x13)` isn't
  touched since the dodge has no attack row).

## 4. Plan (one system per version)

1. **The dodge on KH1's air state.** A row of ours, `kh1rows::OURS` "Aerial Dodge" (no PTYA row,
   request only, like Aerial Recovery), its `kh2slots` slot of motion 202. On the fresh jump in
   the air (`shared::control`'s `AIR_JUMP` branch), when KH1 has him in state 1 / 2 (or KH2's
   jump on KH1's state): our request with KH1's action flag, his facing turned to the stick,
   `kh2_hop` with the growth level's height and speed (KH2's `fmab` above; physics KH1: the same
   numbers under KH1's gravity, as now), the launch speed stored for KH2's glide (`LAUNCH`),
   `mark_air_dodged`. No borrow, no hand-back.
2. **The fall at the apex.** KH2 swaps to motion 4 at the apex (`+0x118`), not when 202's loop
   ends: our per-frame physics hook asks KH1's fall request (0x06) at 6 frames before the apex as
   KH1's own `0x1402AF630` does (or when the slot's air-free frame 4 has passed and he is falling,
   whichever the tester prefers, below). Landing stays KH1's (its landing request 7).
3. **Guard 406 on the slot.** `kh2slots::body_guard` already stops hits for our slots (`hurt`);
   the KH1 guard objects (the parry against enemy attacks, `combat::guard_step`) need a slot path
   too, since `guard_step` only runs on KH2's controller's output.
4. **The early release.** If KH2's dodge is holdable (§2, an assumption to check in the exe's
   held-jump code), `kh2_hop` gets a holdable variant at physics KH2 (`air_step` already does the
   release hop for KH2's jump on KH1's state).

Performance: one call at the press and one apex test a frame on Sora (inside the existing
physics hook); the borrow's controller step, its hand-back and its per-frame pose blend go away
for the dodge. No new hook.

## 5. Open points

- Out of KH2's glide (Glide at KH2 is a borrow, t321): the dodge inside that borrow stays KH2's
  controller's until the glide goes native.
- Neverland: KH1's jump press in the air starts flight there (`0x1402AFD60`); the dodge would come
  first and flight only after it (as now with the borrow).
- Whether walking off a ledge grants the dodge in KH2 (Steam), §2.
- The slot's template: Aerial Recovery uses KH1's air swing slot (102) as its template, whose
  sounds would come with it; a jump slot (KH1's 13) as the template, with its own sounds dropped
  so only 999019 plays, fits a dodge better (**Assumption** until the slot is built and listened
  to).

## 6. Built (testing branch, after the tester's go-ahead)

The tester's answers: 1, my pick (the fall at KH2's apex, as KH2: 1:1 and one test a frame); 2,
(b) the dodge out of KH2's glide goes native too; 3, (a) the dodge first, flight on the next press.

- **t492, the dodge on KH1's air state.** `combomods::aerial_dodge`, from `shared::control`'s
  fresh jump in the air (the borrow stays the fallback when our request isn't built or in a
  Drive Form): our request of motion 202 (`kh1rows::OURS` "Aerial Dodge", request only) with the
  action flag and no blend, KH1's jump state (1) put in its air state first (state 2, `+0x3D0`
  0, flag `0x10000` off, as KH1's launch `0x1402AF960`), facing the stick, `kh2_hop` with the
  level's `fmab` height and speed (`takeover::kh2_air_dodge_growth`), the launch velocity kept
  for KH2's glide (`takeover::note_launch`), once per air time. The slot is 160 frames long
  (`Ours::frames`), KH2's clip looped over it from its own loop point in the bake and the draw
  (`pose`), and only KH2's sound (999019) plays on it (`Ours::own_sounds`: KH1's air swing's
  sounds left out). KH1's air step steers it with the stick as a jump at physics KH2 (it's not
  treated as one of KH1's own air moves despite the action flag): KH2's dodge steers with
  `air_control` on all through. **Checked (decomp)** for KH2's side; KH1's loop at the end of a
  slot with an empty queue (`0x14029F0A0`: from `+0x288`, unless `+0x160` bit 3 holds it) is
  **Checked (decomp)**, but the slot is long enough that it shouldn't come to that.
- **t493, the fall at the apex.** `combomods::dodge_apex`, each KH1 physics frame of Sora's while
  our dodge slot plays in his air state: once KH1's arc fields say 5 frames or less to the apex
  (`+0x3B0` + 5 >= `+0x3B8`) and nothing is pending in KH1's queue, KH1's fall request (6) with
  KH2's blend 8, the action flag cleared. KH2's rule (`0x1403CB680` → `0x1403CB610`, motion 4,
  blend 8) **Checked (decomp)**. That KH1's arc fields are in the same frames as KH2's arc here
  (so the 5 is the same 5) is what `kh1phys::Arc::from_kh2` writes (Checked (code)). Landing stays
  KH1's own (its landing request on touching down).
- **t494, the guard 406 as KH1 guard objects on the slot.** `takeover::kh2_hits` (once a game
  frame with our slot playing in KH1's states) made KH1 guard objects only for guard windows on
  the Keyblade; for the Aerial Dodge's slot its body guard (ATKP 406, group 6, all through 202) is
  one too, all round, as `combat::guard_step` made it for KH2's controller's dodge. An enemy
  attack object meeting it is stopped (KH1's clash, priority 100) with no reaction (`refact_self`
  2, `combomods::guard_repel` does nothing for it); hits his hit pass still finds stay stopped by
  `hurt` (`kh2slots::body_guard`), as before. Cost: KH2's rig posed once a frame while the dodge
  plays (as for the slot moves with guards on the Keyblade). Other slots' body guards (Flash
  Step's 406, Aerial Recovery's 124, Explosion's, Vicinity Break's, Magnet Burst's) are still only
  `hurt`'s: left as they were (not this thread's).
- **t495, the early release.** KH2's dodge is holdable: it sets `+0x120` bit `0x10000`, the air
  actions' shared enter (`0x1403CBD30`) turns that into the held-jump bit `0x20000`, the input
  step (`0x1403D6FC0`) marks the release (`0x40000`) when the jump isn't held, and the air step
  (`0x1403CB680`) then, between 6 frames in and 5 before the apex, swaps to the apex motion (the
  fall, `0x1403CB610`) and cuts the rise to the release hop (`DAT_142A243D8`). **Checked
  (decomp)** (this settles the §2 assumption). Here: the dodge's hop is held
  (`shared::physics::kh2_hop_held`, the arc `holdable`), so `air_step`'s early release (the one
  KH2's jump on KH1's state already uses) cuts it, and asks KH1's fall at once
  (`combomods::dodge_fall`). Physics KH1: no release (KH1's numbers, as before).
- **t496, the dodge out of KH2's glide.** The tester's answer 2 (b). In KH2's glide's borrow (Glide
  at KH2, still KH2's controller's), the jump pressed with the dodge still there: before KH2's
  step the borrow is given back into KH1's fall state (`shared_give_back`, `Back::Air`, KH2's
  glide's launch kept), and the dodge starts natively on it (`combomods::aerial_dodge_with`, the
  level's numbers read off KH2's controller while it's held). Only when our dodge request is built
  (`combomods::native_dodge_built`); else the borrow's controller dodges as before. KH2: the jump
  handler `0x1403CB9F0` is in the glide's action table too (one of its 17), **Checked (decomp)**
  for the table refs; which entry is the glide's is an **Assumption**.

## 7. Second round (the tester's m2 answers)

- **The levels** (the tester: "unsure about the levels, check the decomp"). KH2's jump press in
  the air actions (`0x1403CB9F0`, Steam KH2) reads the Aerial Dodge level with `0x140401680
  (player, 2)` (byte `+0x22` of the player's growth levels; 5 in KH2's special place) and the
  record with `0x1403E92D0 (level)`: the table `0x142AE5778` is a count and offsets, and level L
  takes offset L (record L - 1); the height is the record's `+4`, the speed its `+8` (used only
  with the stick tilted). **Checked (decomp)**. The table is `03system.bin`'s pref `fmab` (count
  5, offsets 24 / 92 / 160 / 228 / 296), and its `+4` / `+8` in the user's file (`local/re-fresh`
  and `local/re` the same): LV1 145 / 18, LV2 155 / 24, LV3 200 / 30, MAX 300 / 36, the special
  5th 1700 / 36. **Checked (files)**. Ours: `pref::FMAB_FIELDS` puts `air_dodge_height` /
  `air_dodge_speed` at `+4` / `+8` and `Character::growth` takes record `level - 1`: the same.
  No change.
- **The dodge cut short at physics KH1** (the tester: "sora in kh1 mode cancels his air dodge";
  traces f80604 and f80807: Physics KH1, Jump KH1). In both traces every dodge's arc was rewritten
  at its air time 6 to a 5-frame rise of 6.06 (KH1's release hop: `0x1402AF8F0 (actor, 5.0)`,
  0.5 x 0.48469 x 25), the jump still held in the second, and the fall swap followed. That is
  KH1's variable jump in Sora's button handler (`0x1402A7B30`, called from the control step
  `0x1402A6220`, **Checked (decomp)**): with `+0` flag `0x40000` (the cut open) and the arc more
  than 5 frames from its apex in state 2, `0x20000` (held) on and the jump let go clears it,
  `0x20000` off and the air time at 6 or more cuts the rise; past the apex margin it closes
  `0x40000`. KH1's takeoff (`0x1402AFDA0`) sets both. The dodge left the jump's own flags, and
  the jump had been let go once before the dodge (`0x40000` on, `0x20000` off, the trace's
  `actor_flags` `0x6c003` -> `0x4c003`), so KH1 cut the dodge 6 frames in whatever the button did.
  Fix (`shared::physics::kh1_held_jump`, after KH1's control step in `takeover::shared_control`):
  a dodge started at physics KH1 gets `0x60000` like KH1's own takeoff, so KH1's own variable jump
  runs on it: held, the full rise; let go between 6 frames in and 5 before the apex, KH1's release
  hop, and the fall swap (`combomods::dodge_apex`) at once. That is KH2's held-dodge rule
  (`0x1403CB680`, same 6 / 5 frames) with KH1's hop, as physics KH1 gives KH1's numbers. At
  physics KH2 nothing changes (`air_step` keeps KH1's cut off and runs KH2's release). Set after
  the step since the dodge's own press is hidden from it (KH1 would read it as let go at once).
- **Walking off a ledge.** KH2 has no fall action of its own (Steam KH2 RTTI: the air actions are
  ACTION_JUMP, HOP, BLOW, GLIDE, GLIDEHOVER, AIRSLIDE, ... no ACTION_FALL), so a walk-off fall is
  ACTION_JUMP (**Assumption**, strong: no other air action fits), whose enter is the shared enter
  `0x1403CBD30` (vtable slot `+8`, **Checked (exe)** by RTTI), which sets the "dodge left" bit when
  the previous action wasn't airborne: the dodge is there after walking off. Ours already gives it
  (`shared::control` clears `AIR_DODGED` on the ground, any KH1 air state 1 / 2 allows it).
  `docs/movement-model.md`'s "it doesn't" came from the Epic exe and isn't confirmed here.
- **Which table is the glide's** (t496's assumption). By RTTI on the Steam KH2 exe, the jump-press
  handler's 17 references are slot `+0x48` of the vtables of ACTION_JUMP, PRIZEBOX, BLOW, HOP,
  CARPETJUMP, FLYJUMP, TORNADO, GLIDEHOVER, GLIDE, CARPETBLOW, DUSKJUMP, LKDASH_JUMP and some world
  ones; the glide's is ACTION_GLIDE's (`0x1405CDC38`, vtable `0x1405CDBF0`), and the hover's
  (`0x1405CDBE0`) too. **Checked (exe)**. So the dodge out of the glide and the hover is KH2's.
- **The 160-frame slot.** The dodge's slot only plays until the fall swap 5 frames before the apex:
  at most ~33 frames for MAX at KH1's gravity (rise sqrt(600 / 0.48469) = 35.2) or ~33 at KH2's
  (38.3), 86 for the 1700 special. 160 covers every rise. **Checked (arithmetic)**.

## 8. Third round (the tester's m3 answer: no release at physics KH1)

- The tester chose "(b) No release at Physics KH1: the dodge always rises fully" over t545's KH1
  release. `shared::physics::kh1_held_jump` (after KH1's control step on the frame the dodge
  starts at physics KH1) now clears `+0` flag `0x40000` instead of setting `0x60000`. KH1's button
  handler (`0x1402A7B30`, Steam) runs its variable-jump cut only while `0x40000` is on (and clears
  it itself past 5 frames before the apex or out of state 2), so with it off the dodge's rise is
  never cut, held or let go; the fall swap stays at the apex (`combomods::dodge_apex`, 5 frames
  before it). No other KH1 code reads that bit of the actor's `+0` word: the other `0xE0000`
  tests in the decomp are on attack objects. **Checked (decomp)**. Physics KH2 is unchanged
  (KH2's own held-dodge release, `0x1403CB680`, in `air_step`).

## 9. Fourth round: the glide out of the dodge (the tester, after m4)

The tester: the glide can't come out of the dodge as fast as in KH2. What KH2 needs, and when:

- **The glide start** is the player's action-button handler `0x1403A81F0` (PLAYER vtable `+0x100`
  through `0x1403A7820`; Steam KH2). With Glide (growth 3) above 0, airborne (`0x1403BA9A0`), the
  height over the floor past the level's start height (`fmab +0x34`), and `0x1403BFB30`: the object
  may act and the motion state has a free bit (`+0x18C & 0x1C`). **Checked (decomp)**. No minimum
  time, no "rising" rule, no buffer: it runs on a Square press in the input step
  (`0x1403D6FC0`: input bit 8 → the object's `+0x100`). **Checked (decomp)**
- **The free bits** come from the motion's ranges (`0x14040D810`: range id 0 → `+0x34` bit 4, 1 → 8,
  2 → 0x10, on the motion holder at `+0x158`, so the player's `+0x18C`), cleared each update before
  the ranges run (`0x1403C83B0`) and at a motion's start (`0x1403C8A40`). **Checked (decomp)**.
  202's only free range is id 1 from frame 4 (`P_EX100.mset` A160, **Checked (files)**).
- **The frame order.** PLAYER's frame (`0x1403BFD30`): the motion update `0x1403C6740` (with its
  range step), then the input step (PLAYER vtable `+0x20` = `0x1403A7090` → `0x1403A89A0`), then the
  action update. The vtable by RTTI: `0x1405C4A80` is `VTABLE<PLAYER>@PLAYER`, `+0x20` = `0x1403A7090`,
  `+0x100` = `0x1403A7820`. **Checked (decomp + exe)**.
- **The clock.** 202 is asked with no blend at time 0 (`0x1403C8A40`: `+0x34` = 1, which makes the
  same frame's update skip, `0x1403C6740`; cleared by `0x1403C73C0`), and the next update turns
  0 → 1 into 2. So, with the press on frame N: N time 0, N+1 time 2, N+2 3, N+3 4 → the free bit
  is on before N+3's input step: **Square glides on N+3**. **Checked (decomp)**; one 60 Hz frame
  per update is the doc's earlier **Assumption** (`kh1-glide-native.md` §8.10).
- **Ours before** (the tester's traces f80604 / f80807, sampled after the physics step): press at
  N (flag 4 on), our slot's time 0 at N's animation, 1, 2, 3, then 4 at N+4's animation, where its
  "may be left" (KH2's frame 4) clears the action flag; KH1's control step, which runs before the
  animation (`0x140295E50`), sees it on **N+5** (the traces: `actor_flags` `c007` → `c003` at the
  fifth frame after the press, the slot's frame 4 drawn). **Checked (trace)**. Two frames late: one
  from KH2's skip of frame 1, one from KH2 reading input after its motion update.

**Changed (testing branch, tNNN):**
- `combomods::dodge_clock` (from `dodge_apex`, each physics frame): on the first physics frame after
  KH1 applied our dodge request (time 0, no blend, a 60 Hz step), the slot's time `+0x16C` is put at
  1, so KH1's update makes it KH2's 2 (KH2's skip of frame 1, as `glide::kh2_clock` does for the
  glide). The sound at frame 1 plays because 202's frame 0 and 1 triggers are keyed at 1.0, not 1 - 1/64 (t726: since t677 KH1's motion pass fires them, and its update from 1 to 2 reads keys with `from <= t < to`; at 0.984 they were never in range and the dodge was silent). **Checked (decomp)**: KH1 `0x14029F0A0` passes the time at `+0x16C` before the step as `from` to `0x14029FB30`.
- `kh2slots` (`INPUT_LEAD`): the dodge slot's KH1 "may be left" starts one frame before KH2's free
  frame (3, not 4), since KH1's control step sees it one frame after the animation fires it while
  KH2's input step sees its free bit on the same frame.
- Together: the action flag clears in KH1's animation on N+2 and Square glides on **N+3**, as in KH2.
  KH1's other moves out of the dodge (an air attack, KH1's own glide at Glide KH1) get the same
  frame, which is also KH2's (its Attack check takes a free state too).
- Not changed: the other KH2 slots' free frames carry the same one-frame order lag (each KH2 free
  frame F is KH1's control step's at F + 1); not this thread's (asked of the tester).
