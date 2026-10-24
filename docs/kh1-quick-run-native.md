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
