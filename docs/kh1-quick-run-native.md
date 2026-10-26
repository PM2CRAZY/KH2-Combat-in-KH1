# Quick Run native (research, t480 round)

Research for the thread "Quick Run native" (`testers/quick-run-native/`). No code yet. Addresses are
**Steam** (as in the Steam Ghidra decomps, base `0x140000000`) unless marked Epic. Tags: **Checked
(decomp)**, **Checked (exe)**, **Checked (files)**, **Checked (code)** (this repo), **Assumption**.

## 1. What we have now

- Quick Run is a borrow: KH2's controller (`character-core`) takes Sora for it and moves him through
  KH1's mover (`shared::QUICK_RUN`, `takeover::shared_frame`). It starts from (a) Square held
  through KH1's own roll once KH2's roll Quick Run check frame is reached or KH1's roll ends, or
  (b) Square while moving with no KH1 Dodge Roll equipped. **Checked (code)** `shared.rs` ~240-300,
  `takeover.rs` ~920-1000 and ~1867.
- The controller's Quick Run (`controller.rs`, `start_quick_run`, `ActionKind::QuickRun`): speed
  `air_slide_speed` along the stick, horizontal speed × `brake` each tick, no gravity, the hold
  counted down only while Square is held, at 0 the brake becomes `stop_brake` and motion 204 plays;
  ends in KH2's Land (s206). Invulnerable for `air_slide_star_time`, **but at least 6 frames**
  (`QUICK_RUN_STAR_MIN`, stage 104, the user's choice): not KH2's (LV1 has 0 in KH2). **Checked
  (code)**.
- Invulnerability reaches KH1 through actor flag `0x40` (`reflect::intangible`,
  `docs/kh1-intangible.md`). **Checked (code)**.

## 2. What KH2 does (Steam KH2 exe)

- The action class is `ACTION_AIRSLIDE` (RTTI string `0x140796848`); its vtable is `0x1405CDB40`
  (found from the RTTI type descriptor `0x140796838` through its object locator). **Checked (exe)**.
  - Slot 7, `0x140423500` (the per-update step): while the timer `+0x10C` is above 0, it counts
    down by KH2's frame scale (`0x140717480`, 2.0 by default) only while the held bit (`+0x124` bit
    2) is set; with it clear the timer goes to 0 at once. At 0: brake `+0x108` = stop brake
    `+0x110`, motion 0xCC (204, the end), a move request (4, 8.0). **Checked (decomp)**.
  - Slot 8, `0x140423480` (velocity): velocity `+0x20` ×= brake `+0x108` each call, the frame's move
    is that × the step's dt. **Checked (decomp)**.
  - Slot 9, `0x140422630`: shared with other actions (a move request and `+0x118` = 4, `+0x11C` =
    5); not looked into. **Assumption** that it is the generic leave/finish.
- The start, `0x1403A9830`: level = Quick Run's ability level (`0x140401680(…, 1)`); 0 = no Quick
  Run. From that level's fmab record: velocity = direction × `+0x10`, brake `+0x108` = `+0x14`,
  timer `+0x10C` = `+0xC`, stop brake `+0x110` = `+0x18`; motion 0xCB (203); for level > 1 a call
  `0x1403B6670(actor, level + 0x6C, 1, 0, 0)` (**Assumption**: an effect / sound element of the
  higher levels, not identified); action 0x10; the held bit set; invulnerable time from `+0x1C`
  (`0x1403D7E40`). **Checked (decomp)**. Called from three places in `0x1403A81F0` /
  `0x1403A96C0` (the action-button handler and the roll's check; not each traced).
- fmab records 0-3 (LV1-MAX) in the user's `03system.bin` (`pref` → `fmab`), Quick Run's fields
  **Checked (files)**:

| Level | time `+0xC` | speed `+0x10` | brake `+0x14` | stop brake `+0x18` | invulnerable `+0x1C` |
|---|---|---|---|---|---|
| LV1 | 14 | 30 | 0.94 | 0.93 | 0 |
| LV2 | 18 | 34 | 0.95 | 0.93 | 8 |
| LV3 | 40 | 38 | 0.96 | 0.93 | 12 |
| MAX | 40 | 38 | 0.98 | 0.93 | 16 |

  (Record 4 is a debug level 5: time 40, speed 38, brake 0.98, invulnerable 40.)
- Units: the timer is counted in KH2's frame-scale units (60 Hz frames, 2.0 a 30 Hz update).
  **Checked (decomp)** for the decrement; the brake is applied once per KH2 update, so at 60 fps
  its per-60-Hz-frame value depends on how KH2 PC steps (a 30 Hz brake would be its square root a
  60 Hz frame, as KH1 does with its own drag). **Assumption**, to confirm before porting: KH2's
  update rate and dt on PC.
- Not found yet: whether AIRSLIDE turns gravity off or keeps him on the floor (it sets no fall
  fields in the two slots read), and exactly how the end motion hands over to Land. Our controller
  says no gravity and an end into Land (s206, from play); not confirmed in the decomp.

## 3. How KH1 does the matching thing

### KH1's dodge roll is not a state

- KH1's roll starts from its button handler (Steam `0x1402A7B30`, Epic `0x1402A59A0`) in state 0
  with the Dodge Roll bit (`0x142D5EC10` bit `0x20`; Guard is bit `0x10`): turn toward the stick
  (`0x140297FB0` → `0x140297E10`), then `0x1402C9E60` (Epic `0x1402C7CB0`). **Checked (decomp)**.
- `0x1402C9E60`: actor `+0xC` = 12.0 with flag `0x80` (`0x140297F30`), flag `0x100000`, motion
  request `0x10000DC` (the roll; bit 24 sets the action flag 4), a move request. **Checked
  (decomp)**. The think step counts `+0xC` down by the frame scale while `0x80 | 0x100000` and
  clears both at 0 (`0x140297FE0`): a 12-frame timer. **Checked (decomp)**; what it gates is
  **Assumption** (not traced).
- The roll stays in **movement state 0** (the ground state). Its displacement is state 0's ground
  step (`0x1402B1850`: the move direction laid on the floor × `+0x2C` × dt). Rolling off a ledge,
  state 0's transition (`0x1402CC470`) runs `0x1402C9EB0` (ends the roll, queues the fall) and goes
  to state 2; state 2's pre-step (`0x1402AFC20`) also cuts a roll motion. **Checked (decomp)**.
- Its untouchable frames are motion events `0x2B` / `0x2A` setting actor flag `0x40`
  (`docs/kh1-intangible.md`, Epic addresses there). **Checked (exe)** in that doc.

So "reusing KH1's roll state" means: state 0 plus an action (flag 4) and a motion, like KH1's roll.

### KH1's movement state table

- Table `0x142DB3840`, 0x40 bytes a state, **59 entries** (`0x3B`: cleared by `0x1402B1A40`,
  which then calls every base registration; the table ends at `0x142DB4700`, where other data
  starts). **Checked (decomp)**.
- Registration `0x1402B1B80 (state, flags, enter, exit, pre-step, step, transition, +0x30,
  +0x38)` writes `+0x0` flags, `+0x8` enter, `+0x10` exit, `+0x18` pre-step, `+0x20` step, `+0x28`
  transition, `+0x30`, `+0x38`. Callers: the base pass above plus world / object code
  (`0x1401D9370`, `0x1402893B0`, `0x140397CA0`, `0x14039C3F0`, `0x14039EAD0`, `0x14039EEB0`,
  `0x14039F250`, `0x14039FFD0`, `0x1403A04A0`). **Checked (decomp)**.
- Registered ids: 0, 1, 2, 3, 4, 5, 7, 8, 9, 0xA, 0xB, 0xC, 0xD, 0xE, 0xF, 0x11, 0x12, 0x15,
  0x18-0x1F, 0x20 (two registrations), 0x21, 0x22, 0x25, 0x26, 0x27, 0x28, 0x29, 0x32. **Checked
  (decomp)**.
- Used by number without a registration: 0x10 (camera and physics: its own transition case; the
  pole / ladder leap seen in s206 logs), 0x13, 0x14, 0x16, 0x17 (the physics step's own
  displacement / transition cases; `0x1402B1AE0` treats 0x17 as air). **Checked (decomp)**.
- **Free ids** (no registration, no comparison by number found): 6, 0x23, 0x24, 0x2A-0x31,
  0x33-0x3A. **Checked (decomp)** by a search of every `+0x70` comparison and switch; a range test
  that would take a new number in is still possible (**Assumption** that none does, beyond the
  ones listed below).
- Flags word (`+0`): bits 0-1 the collision class (`0x1402B1AE0`: `& 3 == 2` air; `0x1402B1B10`:
  `& 3 == 1`, plus states 2, 0xB, 0x10, 0x11, 0x13, 0x14, 0x16 by number), which picks the
  collision query's mode in `0x14029A0B0` (air mode 0x10 vs ground 4, i.e. floor snap or not); bit
  8 (`0x8`) the platform carry; bits 4-7 are read by the command menu fill (`0x140294220`: bit
  0x10 the Attack entry, 0x40 another) and the pad step (`0x140288F30`, bit 0x80); bit 4 (`0x4`)
  plays sound 0x44 somewhere (`0x1402B0610`). **Checked (decomp)** for the reads; the meaning of
  bits 4-7 beyond Attack is **Assumption**.

### What reads the state number (don't break KH1's events)

- The physics step `0x14029A3C0` (displacement and transition by number, then the table).
- The camera module (`0x1401CDBD0`, `0x1401D2090`, `0x1401E2650` switches; several "standing"
  tests `(state & ~0x18) == 0 && state != 0x10`, i.e. states 0, 8, 0x18 count as standing). A new
  state number counts as "not standing" there, so KH1's camera would treat Quick Run like other
  moves. **Checked (decomp)** for the tests; the effect on the camera is **Assumption**.
- `0x14029BBD0` (a switch on 0/1/0xC/0x1C/0x20/0x26/8/0x11/0x18/0x19/0x1D-0x1F/0x27; likely the hit
  pass or the reaction picker), `0x14029CA60` (0/0x18/0x19/2/8/0x11), `0x1402A43A0` (commands; 0,
  0x1E, 0xA, 0xD, 0x11, 0x1F, 2, 3, 0x10, 0x15), and the command menu's mask 0x203000100 in
  `0x1402937F0`. A new state falls to their default case. **Checked (decomp)** for the cases;
  what each default does for Sora is not yet read.
- Event scripts: none of the event opcodes in `0x1401A0000-0x1401BFFFF` compare `+0x70`; scripts
  change it through the state setter `0x1402B1960` (85 callers). **Checked (decomp)**, first pass.
- Our own code: `takeover::kh1_mode` treats every state ≥ 29 as KH1's own mode, so a new id in
  0x2A-0x3A would need excluding there. **Checked (code)**.

## 4. Options

**(a) A new KH1 state for Quick Run** (registered in a free id, e.g. 6 or 0x2A, after
`0x1402B1A40`'s pass: hooking it or writing the entry each frame as `shared::physics` does).
- enter: KH2's start (speed along the stick, brake, timer, stop brake from the fmab record of
  Sora's Quick Run level; motion 203 through KH1's motion request; the invulnerable frames through
  flag `0x40`); pre-step: the held test (Square), the timer, the switch to the stop brake and
  motion 204; step: displacement = velocity × dt, velocity × brake; transition: to Land / state 0
  at the end, to state 2 (KH2's Aerial Dodge allowed after, as s183) when the floor goes; exit:
  clear flag `0x40`, the speed.
- The velocity has to live in one of KH1's fields: `+0x3C0` (KH1's air speed, already the
  momentum field per `docs/kh1-physics.md`) is the natural one; KH1's walk-off already carries it.
- Collision class: ground (flags `& 3 == 0`, floor snap) or air (`& 3 == 2`, no snap). Which one
  KH2 is depends on whether AIRSLIDE keeps him on the floor (open, §2).
- Menu bits: choose which commands KH1 offers during it (KH2: what can cut a Quick Run).
- Cost: one state's handlers, run only while he is in it; no second simulation. KH1's camera,
  reactions and commands see a new state number (their default cases).

**(b) KH1's roll pattern: state 0 + an action.** Quick Run as a ground action like KH1's roll
(action flag 4, motion 203, a timer field), with its speed written into `+0x2C` and the direction
into `+0x20` for state 0's ground step, and the brake applied each frame by a wrapper of state 0's
step (already wrapped by `shared::physics`).
- No new state number, so every KH1 system that knows state 0 behaves as for the roll.
- But state 0's ground step lays the move on the floor (floor snap, collision mode 4) and its
  transition ends the action on leaving the ground; and the per-frame brake, held test and end
  switch all go into the state-0 wrapper, keyed on the motion (as KH1 keys its roll on motion
  0xDC).

## 5. Open

- KH2 AIRSLIDE's gravity / floor handling and its hand-over to Land (read more of the class and
  the generic action code).
- KH2's update rate on PC (the brake's per-frame value at 60 fps).
- The level > 1 call `0x1403B6670(level + 0x6C)`.
- What `0x14029BBD0` / `0x1402A43A0` do for a state number they don't know.
- The roll's own travel source (`+0x2C` or the motion's root) was not traced.

## 6. Settled from the decomp, and built (after the tester's go-ahead)

The tester's answers: "do your first purposed route" (the new KH1 state) and question 1 (a), KH2's
real invulnerable frames (LV1 none). The §5 open points, from the Steam KH2 exe:

- **What cuts a Quick Run in KH2: nothing but letting go of Square.** The AIRSLIDE vtable
  (`0x1405CDB40`) is slots 0..9 (slot 10 is the next class's locator, `ACTION_GLIDEHOVER`).
  Slot 9 (`0x140422630`) is its jump press (a jump: `0x1403CBB00`, apex motion 4, landing 5), but
  the input step `0x1403D6FC0` calls it only while the motion state word `+0x18C` has bits
  `0x1C`, and commands go through `0x1403D6CA0`, which needs the same bits. Those bits are range
  ids 0 / 1 / 2 (the motion's "free" states: the range handler `0x14040D810` sets `+0x34` of the
  motion controller at `+0x158`, i.e. `+0x18C`, bit 4 / 8 / 0x10 for them). Motions 203 and 204
  have none: 203 (A150, 30 frames at 30 fps) has range 3 (no gravity) all through and range 14
  (reaction commands, bit `0x80000`) from frame 8; 204 (A151, 18 frames, holds its last) has
  range 3 for 0..28 and range 14 all through, and a sound at 6. **Checked (decomp)** and **Checked
  (files)** (`P_EX100.mset`). So KH1's menu offers nothing during it (our state's flags have no
  command bits) and the jump does nothing.
- **Gravity and ledges.** AIRSLIDE is a ground action (its object's `+0xC` bit 0 is clear in the
  action table `0x140750D30`, entry 0x10, **Checked (exe)**), held off gravity by its motions'
  range 3. Losing the floor runs the generic slot 4 (`0x140409B00`: `0x1403CBC30 (actor, 0)`, a
  fall from rest with no apex swap and landing motion 5, then action 1), which leaves the
  velocity `+0x20` as it was; the air actions' enter (`0x1403CBD30`) then gives the Aerial Dodge
  back, since the previous action wasn't airborne. **Checked (decomp)**. Which motion KH2 shows
  falling off a ledge mid-run isn't settled: the fall action asks for none (**Assumption**: it
  may keep 203 / 204; we ask KH1's fall).
- **The end.** At the timer's 0 KH2 plays 204 with no blend and queues motion 4 (the air fall
  loop, A004, range 1) with blend 8 (`0x1403C8310`, the motion queue `0x1403C6740`). That the
  fall on the floor then becomes KH2's landing (motion 5) is the end seen in play (s206), not
  traced: **Assumption**.
- **The update rate.** The frame scale `0x140717480` is the vsyncs per update times the speed
  (1.0 on PC at 60 fps, `0x140154600` / `0x140154740`), so the hold timer counts 60 Hz frames.
  The velocity step (slot 8) runs on the player's physics tick (`0x1403B89A0`: `+0xBA8`, 0 at each
  action change, counts down by the frame scale; below 0 slot 8 runs with dt 2.0 and the per-frame
  velocity is half its result), so the brake is per two 60 Hz frames, starting on the first.
  **Checked (decomp)**.
- **The higher levels' call.** `0x1403B6670 (actor, level + 0x6C, 1, 0, 0)` starts the actor's
  effect element of that id (`0x1402C6B30` walks the actor's PAX element list for it): elements
  110 / 111 / 112 of `P_EX100.a.us` for LV2 / LV3 / MAX (element 110: effect 18 on bone 0x4000).
  **Checked (decomp)**, **Checked (files)**.
- **The invulnerable time** is `+0xD70` (`0x1403D7E40`), from the start, in frames (**Assumption**
  that it counts 60 Hz frames as the timer does; the controller already counted it so).

### Built

1. **The state** (`crates/kh1-hook/src/win/quickrun.rs`). KH1 state `0x2A` written into KH1's
   table each Sora physics frame (only into an empty slot or ours): flags `0x08` (the platform
   carry; collision class 0, the ground's; no command / shortcut bits), no enter, exit, pre-step
   or jump press; our step and transition. The start (`quickrun::start`, KH2's `0x1403A9830`):
   Sora's Quick Run level's `fmab` record (`takeover::kh2_quick_run_growth`), velocity along the
   direction × speed, brake, timer, stop brake, invulnerable time; state `0x2A`; our request of
   motion 203 with KH1's action flag and no blend; facing turned to the direction. The step:
   KH2's hold timer (Square from `takeover::pad_square`, counted by KH1's frame scale; let go,
   0), at 0 the stop brake and our request of 204 with no blend; KH2's velocity step on its tick;
   the move through KH1's own ground step (Steam `0x1402B1850`, so floors and slopes are KH1's)
   with `+0x20` / `+0x2C` the velocity's direction and length; `+0x3C0` the velocity (KH1's
   momentum field). The transition: on while one of our motions plays or is pending in KH1's
   queue (`+0x178` / `+0x17C`); 204 played out: KH1's ground state with KH1's landing (request 7,
   action flag, blend 8, idle queued); another motion put on him (a hit's reaction, an event):
   KH1's ground state or fall. Motions 203 / 204 are request-only rows of ours (`kh1rows::OURS`
   "Quick Run" / "Quick Run stop", KH1's ground swing slot as their template, its sounds left
   out). `takeover::kh1_mode` leaves state `0x2A` out of "KH1's own modes" (every state ≥ 29).
2. **The start points** (`shared::control`): Square held through KH1's roll (along his facing,
   as KH2's roll turns into it; KH1's roll timer flags `0x80 | 0x100000` off as its think clears
   them, `0x140297FE0`), Square while moving without KH1's Dodge Roll, and Square while moving with
   the Dodge Roll style KH2 where KH2's action button picks Quick Run (no dodge roll of KH2's;
   KH2's order `0x1403A75A0`: guard at rest, roll, Quick Run). The borrow stays the fallback (a
   Drive Form, our requests not built, the Epic build). KH2's own roll held into Quick Run inside
   its borrow (Dodge Roll KH2 with KH2's roll) is still KH2's controller's.
3. **Off a ledge** (the transition, airborne): KH1's fall from rest written as `0x1402AF8F0 (actor,
   0)` does, state 2 set by us (so the physics step's 8-unit walk-off cap doesn't run), action
   flag off, `+0x3C0` the slide's velocity uncapped, KH1's fall request with blend 8, the Aerial
   Dodge left.
4. **Invulnerable frames 1:1**: `hurt` stops KH1's hits during the native run's `fmab` frames
   (0 / 8 / 12 / 16), as our slots' range 27; the controller's 6-frame minimum (stage 104) is gone
   (forms and the borrow fallback get KH2's too).
5. **The start effect above LV1**: elements 110 / 111 / 112 started through `kh2fx` (Sora's package
   built with them; 203 in `kh2_effects`' default list).
   MAX shows nothing, as in KH2: `0x1402C6B30` (through `0x1403B6670`) plays every element of
   the owner's list whose id matches and nothing when none does, and base Sora's `P_EX100.a.us`
   (`p_ex`) has 110 and 111 but no 112 (**Checked (decomp)**, **Checked (files)**: parsed; the
   owner's log "p_ex100 element 112 doesn't play" on MAX runs, **Checked (log)**).

Cost: one state step and transition a frame while he runs (no controller step, no hand-back);
one table write a frame (a compare when it is already ours).

Not checked in game: all of it. Open: the motion KH2 shows off a ledge mid-run (above); whether
KH1's camera, which counts only states 0 / 8 / 0x18 as standing, behaves for state `0x2A`; what
KH1's hit reaction switch `0x14029BBD0` does for a state number it doesn't know (our transition
puts him back in state 0 the frame after another motion lands on him).

### Out of KH1's own roll at KH2's frame (the tester's trace, Dodge Roll KH1)

- **What the trace showed.** With Dodge Roll KH1, KH1's roll motion `0xDC` played from frame 9384
  and the Quick Run started at 9414, 30 frames in: the end of KH1's roll motion (its travel 17.8 a
  frame for 10 frames, 11.3 for 12, then 5 still frames). With Dodge Roll KH2 (log, f7301 / f7311)
  it started 10 frames into the roll. **Checked (trace)**, **Checked (log)**.
- **Why.** `shared::control` let the held roll into Quick Run only on a frame KH1's can-act test
  passed, and KH1 holds its action flag until its roll motion ends. KH2's roll on KH1's roll was
  already let through at the check frame (it bypassed the can-act test). **Checked (code)**.
- **KH2's rule.** KH2's dodge roll (motion 209, `P_EX100` A180) turns into Quick Run at its Quick Run
  check, frame trigger 34 at frame 10 (60 Hz), while Square is held. **Checked (files)** (motion 209:
  length 52, trigger 34 at 10, trigger 35 the fall check at 34). The OG rewrite's DodgeRoll action
  does the same.
- **Now.** KH1's own roll held into Quick Run is cut at the same frame (10 frames from the roll's
  first frame, the same counter as KH2's roll on KH1's roll), whatever KH1's can-act test says;
  the Quick Run itself (start speed, brake, hold timer, stop) was already identical both ways (log:
  speed 30, brake 0.94, time 14, the stop after 13 frames at 19.5 in both).
- **Distance.** What still differs is the roll part before the cut: KH2's roll's root travel is
  220.8 by frame 10 (23.1, 22.8, ... 22.1 a frame; 438.4 at its end, frame 24), KH1's roll ~160 by
  its frame 10 (17.8 a frame, its first frame still). **Checked (files)** (motion 209's root),
  **Checked (trace)** (KH1's roll). That is KH1's own roll speed, which the KH1 row keeps.

## Testing a level (debug menu)

With `debug_menu = on`, Debug Menu > Growth Abilities > Quick Run forces LV1..MAX (or Off) whatever
Sora's level, given even when not learned or equipped (`docs/kh1-debug-menu.md`, "Growth Abilities
page"). The native state reads the forced level (`takeover::kh2_quick_run_growth`), so its fmab
record, the LV2+ start effect and the invulnerable frames follow it. **Checked (code).**
