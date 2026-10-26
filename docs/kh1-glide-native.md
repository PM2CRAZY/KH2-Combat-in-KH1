# KH2's glide in KH1's glide state (native research)

Research for the tester thread "KH2 Glide in KH1's glide state" (`testers/kh2-glide-in-kh1s-glide-state/`).
All addresses are the **Steam** build's (Ghidra addresses from the Steam decomps, image base 0x140000000) unless
marked "Epic". Where an Epic address from older notes was translated, it says so. No code from the decomps is
reproduced here: plain-language notes only.

## 1. What we have now (testing branch, t480)

- **Glide KH1**: KH1's own glide state 0x15 with KH1's own numbers, at either physics setting (t321).
  **Checked (code)**
- **Glide KH2**: a *borrow* (`shared::GLIDE`, `takeover`): out of KH1's air, past the rise, a fresh Square with KH2's
  Glide (our ability id 88, `growth::kh2_glide_level`) equipped and above KH2's start height hands Sora to KH2's
  controller (`character-core`), which runs its own glide (speed, fall, turn, banking 206 / 207, hover 208, release
  into its fall), then gives him back to KH1's fall (`give_back`). KH2's stored launch speed (`LAUNCH`) is kept across
  the hand-back (t331). KH1 doesn't see its own glide inputs in the air at Glide KH2. **Checked (code)**
- The KH2 glide in `character-core` (`controller.rs`, `Mode::Glide`, `try_glide`, the glide step) was matched to the
  Steam KH2 exe in t331 (stored launch speed, no stick-scaled speed, hover only from a neutral start, never back to the
  hover). **Checked (code)**, its exe notes in AGENTS.md stage 331.

### Why the shared version (t284..t320) wasn't 1:1

t284 made the glide a "shared" move: KH1's state 0x15 ran, and two wrappers in KH1's state table (removed in t321,
commit `e40fb4c`) replaced only:
- the **pre-step**'s turn rate (KH2's glide turn speed instead of KH1's 2° / 1° a frame), and
- the **step**'s speed and fall (`(1 - a) * top * tilt + a * last`, and KH2's fall by the height over the floor).

Everything else stayed KH1's **(Checked (code), git `e40fb4c`)**:
- KH1's **start** (its own height test of 50, its speed halved, its own request 0x71), not KH2's start height,
  launch speed, or hover-versus-glide choice;
- KH1's **46-frame minimum glide** and **24-frame re-glide cooldown** (below), and its **release**: speed x 0.7 and
  a hop of 8 into its fall motion, where KH2 keeps the speed and has no hop;
- KH1's **motion picks** (requests 0x71..0x74 by a 30° turn and a speed of 8), not KH2's banking (dot < 0.98) and
  hover 208;
- the wrappers only ran at physics KH2, so at physics KH1 the tester got KH1's +0.4 a frame ramp to 16, its
  saw-tooth fall and KH1's release (t320 traces, AGENTS.md stage 321);
- the speed still scaled with the stick's tilt (KH2's doesn't, t331) and there was no stored launch speed.

## 2. KH1's glide state (Steam KH1 decomp)

KH1's per-state table: Epic `0x142DB4240` -> Steam `0x142DB3840` (translated through `steam_map.in`), 0x40 bytes a
state, so state 0x15's entry is at `+0x540`. Known slots: `+0x18` pre-step `(actor, input)`, `+0x20` step `(actor,
&displacement)`, `+0x28` transition (**Checked (code)**: `shared.rs` wrapped them in t284). The registration call
below passes, in order, enter, exit, pre-step, step, transition and one more handler, so `+0x08` enter, `+0x10` exit,
`+0x30` the button check is the likely layout (**Assumption**, from the argument order; verify against the table).

| Steam address | What it does | Tag |
|---|---|---|
| `0x1402B26F0` | Registers state 0x15 with `0x1402B1B80`: flags 5; enter `0x1402B2D00`, exit `0x1402B29F0`, pre-step `0x1402B2880`, step `0x1402B2A20`, transition `0x1402B2780`, button check `0x1402B2BF0`. | Checked (decomp) |
| `0x1402B2D30` | The common start: only when the combat state's glide timer (`[+0x148] +0x100`) is 0 or less and the height above the floor (`+0x3D4`) is over 50: asks motion request 0x71 (blend 16), halves the air speed `+0x3C0`, sets state 0x15. Returns whether it started. | Checked (decomp) |
| `0x1402B26C0` | Glide start: the common start, then clears the combat state's `+0x104` bit 0 (Superglide off). | Checked (decomp) |
| `0x1402B2750` | Superglide start: the common start, then sets `+0x104` bit 0. | Checked (decomp) |
| `0x1402AFD60` | The air state's jump-button glide start: Glide only (ability bits `0x142D5EC10` bit 0x80). Superglide's start is called from `0x1402A7B30` (Square). | Checked (decomp) |
| `0x1402B2D00` (enter) | Sets the glide timer to 46 frames, clears `+0x1CC` bit 0. | Checked (decomp) |
| `0x1402B29F0` (exit) | Sets the glide timer to 24 frames (the cooldown before the next glide, read by the start), sets `+0x1CC` bit 0. | Checked (decomp) |
| `0x1402B2880` (pre-step) | Turn: angle from his facing (`+0x3C`) to the stick; once the last motion blend is done (`+0x170` = 0) asks 0x73 (turn over +30°), 0x74 (under -30°), else 0x71 / 0x72 by air speed over 8. Turns at most 2° a frame (Glide) or 1° (Superglide) x the frame step `0x14233FBDC`, sets the facing (`0x140297E10`) and `+0x150`. Then `0x1402C12B0`: entering water ends the glide into the water state 0x11 with its splash (not in world 0xD room 8). | Checked (decomp) |
| `0x1402B2A20` (step) | Speed: top 8 / fall 1.2 (Glide), top 16 / fall 2.0 (Superglide; the fall eased down below 70 over the floor). Speed ramps +0.4 a frame to the top, along the facing (`+0x30`). Fall: a rise is damped x0.8, then KH1's gravity 0.48469 a frame is added until past the fall cap, then halved (the saw-tooth the tester saw). Writes `+0x3C0` / `+0x3C4`, and the frame's displacement length to `0x142DB4700` (Steam). | Checked (decomp) |
| `0x1402B2BF0` (button check) | Counts the 46-frame timer down; after it, when the button bit isn't held (4, the jump, for Glide; 0x10 for Superglide), releases: air speed x0.7, `0x1402AF960 (actor, 8.0)` (a hop of 8 into the fall), fall motion 6 (blend 8). | Checked (decomp); "4 = jump held, 0x10 = Square" is an **Assumption** from the Glide / Superglide buttons |
| `0x1402B2780` (transition) | Neverland (world flags `0x142867364` bit 0x20) with `0x1402B1DA0` true: flight state 8. Not airborne (its second argument 0): the same release as above. Airborne: once the timer is out, a frame that moved less than 70% of the last step's displacement (or with a negative component, against a wall) ends the glide (`0x1402B2670`, the same release). | Checked (decomp); the wall reading is an **Assumption** |
| `0x1402B2670` | The release (x0.7, hop 8, motion 6), also called by area objects (`0x1401CD050`). | Checked (decomp) |
| `0x1401CF9E0`, `0x1401CD050`, `0x1401CECC0` | Area-object code (collision entries with flag 0x100 at `+0x58`): starts KH1's glide or Superglide from the air state (Sora or member 2 only, not in Neverland) and ends it on some objects. Which objects these are is not known. | Checked (decomp); purpose **Assumption** |

The frame step `0x14233FBDC` is 1.0 at 60 fps (AGENTS.md stage 233, KH1's air state). KH1's glide numbers are per
60 Hz frame. **Checked (decomp)**

## 3. KH2's glide (Steam KH2 decomp and exe)

Action vtables (read from the Steam exe): `ACTION_GLIDEHOVER` `0x1405CDB98`, `ACTION_GLIDE` `0x1405CDBF0`.
**Checked (exe)**

| Steam address | What it does | Tag |
|---|---|---|
| `0x1403A8369` | The start: action 0x11 (hover) with the stick neutral, else 0x12 (glide). | Checked (exe), stage 331 |
| `0x1403CBD30` | The air actions' shared enter: puts the stored launch speed (`+0xE0`, flag `+0xEC`) back into his velocity. | Checked (decomp), stage 331 |
| `0x140423B90` (glide enter) | The shared enter, then his turn rate = the growth level's `+0x3C`. | Checked (decomp) |
| `0x140423A70` (glide update) | Motion: 205 when the stick is let go or the facing is within dot 0.98 of it, else 206 / 207 by the side of the turn (blend 16); then the end check `0x1404235B0`. | Checked (decomp) |
| `0x1404235B0` | End: grounded, or the height over the floor under the growth level's `+0x38` (end height): fall motion 4 (blend 16) and the next action. | Checked (decomp) |
| `0x140423920` (glide step) | Speed `(1 - a) * speed + a * |v|` (growth `+0x30` a, `+0x20` speed) along his facing; fall from `0x1404237A0`. | Checked (decomp) |
| `0x1404237A0` | Fall: `(height over the floor - +0x28) * +0x24`, at most `+0x2C`, 0 below `+0x28`. | Checked (decomp) |
| `0x140423840` / `0x140423650` | Hover: the stick takes it into the glide (launch speed back); its step is velocity x0.9. | Checked (decomp), stage 331 |
| `0x1403CBC30` | Into the fall: keeps the velocity, clears the launch flag. No hop, no speed cut. | Checked (exe), stage 331 |

The growth level record (`+0x108`): `+0x20` speed, `+0x24` fall ratio, `+0x28` fall height, `+0x2C` fall max,
`+0x30` acceleration, `+0x38` end height, `+0x3C` turn speed; `+0x34` start height is an **Assumption** (the one glide
field not seen in these functions), matching `kh2-data`'s `glide_*` order.

Not found in the KH2 decomp: any minimum glide time or re-glide cooldown like KH1's 46 / 24 frames, and what KH2 does
when the glide meets a wall. **Not checked** (to look for in the action-button handler `0x1403A75A0` and the
collision response).

## 4. What reads KH1's glide state (don't break)

- Neverland's flight start from the glide (transition, state 8). **Checked (decomp)**
- Water entry from the glide (pre-step's `0x1402C12B0`, state 0x11). **Checked (decomp)**
- Area objects that start or end a glide (`0x1401CF9E0`, `0x1401CD050`). **Checked (decomp)**
- KH1's glide slots 75..78 (requests 0x71..0x74, Superglide's 0x76..0x79) and its motion events. **Checked (code)**,
  `docs/kh1-anim-slots.md`
- Our own code: `takeover`'s `KH1_GLIDE_STATE` checks (a KH1 glide under KH2 is turned back to 0), `shared::control`'s
  hides, `stylejump`'s KH1 glide. **Checked (code)**
- No event-script opcode reading state 0x15 was found (only the object code above compares `+0x70` with 0x15 outside
  the glide's own functions). **Checked (decomp)**, a grep of `+0x70 == 0x15`; tallies not checked.

## 5. Plan (native: KH2's glide logic in state 0x15, one system per version)

At Glide KH2 only (Glide KH1 stays vanilla). Every wrapper below is on state 0x15's table entry, for Sora, and runs
only while he is in that state: no per-frame cost outside the glide, and it replaces KH2's controller running the
whole glide (the borrow).

1. **Start**: KH2's start into KH1's state. Square in KH1's air state past the rise (as now) calls KH1's common start
   path but with KH2's rules: KH2's start height (growth `+0x34`) instead of 50, no KH1 cooldown, no halving; the
   stored launch speed put into `+0x3C0`; hover or glide chosen by the stick. The launch speed needs a home: KH1 has
   no `+0xE0`, so a per-actor field of ours, written where KH1's jump and our Aerial Dodge launch him (KH2's
   `0x1403CBB00` points). Enter / exit: KH1's 46-frame timer and 24-frame cooldown not set.
2. **Step**: KH2's speed `(1 - a) * speed + a * |v|` along the facing and KH2's fall by `+0x3D4` (KH1's own height
   over the floor), the hover's x0.9, written into `+0x3C0` / `+0x3C4` and the displacement; KH1's mover and
   collision move him. KH2's numbers per 60 Hz frame (KH2's growth values are per KH2 tick: the conversion the
   controller already makes, checked against KH1's frame step `0x14233FBDC`).
3. **Pre-step**: KH2's turn (growth `+0x3C`, toward the stick) and KH2's motion choice (205 / 206 / 207 at dot 0.98,
   208 for the hover), still calling KH1's water check. Motions as KH1 requests on KH2 glide slots (the
   `kh2slots` / `docs/kh1-anim-slots.md` route), so no pose is drawn over KH1.
4. **End**: the button check and transition: let go of Square, or under KH2's end height, or grounded: KH2's fall
   (KH1's state 2 with the speed kept, no x0.7, no hop, motion 4, the launch flag cleared). Neverland's flight and the
   area objects kept.
5. **Take the borrow out**: `shared::GLIDE`'s hand-over to KH2's controller, `LAUNCH`'s hand-back logic and the
   controller's glide path for base Sora (Drive Forms keep theirs until the forms move).

The physics setting doesn't touch any of it (the tester's t321 rule).

## 6. Open

- KH2's handling of a wall during the glide, and whether KH2 has any glide cooldown: not found yet.
- The button bits 4 / 0x10 in KH1's button check: reading not confirmed.
- KH1's area objects with the glide flag: which ones (and whether KH2's glide should still be started / ended by them).

## 7. Who reads state 0x15, and KH2's wall rule (research for the tester's answers)

All Steam addresses.

| Steam address | What it does | Tag |
|---|---|---|
| `0x1402A3300` | Player / party think. When the actor's object (`+0x130`) has its `+0x54` AI byte set, it runs the party AI `0x1401CDBD0` (not in world-flag `0x20000` worlds, nor with `0x142867368` bit 8); otherwise player control `0x1402A6220`. | Checked (decomp) |
| `0x1401CDBD0` | Party-member AI brain; calls `0x1401CECC0`, `0x1401CFB30`, `0x1401D1400` and others. | Checked (decomp) |
| `0x1401CF9E0` | AI jump / glide decision, members 1 and 2 only (object `+0x4C`; Sora is 0): in air state 2 with the AI record's `+0x58` bit 0x100 it starts KH1's Glide `0x1402B26C0` or Superglide `0x1402B2750` by Sora's ability bits `0x142D5EC10` (0x80 Glide, 0x100 Superglide); in Neverland flight `0x1402B1EC0` instead; otherwise jumps toward the follow point (`0x1402A3AD0`). | Checked (decomp) |
| `0x1401CECC0` / `0x1401CD050` | AI step; while the AI's own actor is in 0x15 its glide steering toward the follow point ends that glide (`0x1402B2670`) once close or past it. | Checked (decomp) |
| `0x1401D3120` | AI jump decision; mask `0x1220104` (states 2, 8, 0x11, 0x15, 0x18) = already airborne or in water. | Checked (decomp) |
| `0x1401E6670`, `0x1401E2650` | KH1's follow-camera helper and lock-on camera: the followed actor's `+0x70` in 4, 5, 8, 0x15, 0x18, 0x1A is one "airborne" case. Only at Camera Style KH1. That `0x1401E1920` (its caller) is the follow camera is an Assumption. | Checked (decomp) |

- **The "area objects" in §2 / §4 are the party AI**, not map objects. No KH1 code starts or ends Sora's own glide outside the glide state's own handlers and its start calls. **Checked (decomp)** by grep for `+0x70` against 0x15, `case 0x15` and state sets with 0x15 (other spellings may be missed: Assumption).
- The hover as a sub-mode of 0x15 is covered by every reader above (and Neverland / water, inside 0x15's own handlers); a new state would fall outside the camera's airborne case.
- Our 0x15 wrappers must check the actor is Sora: Donald and Goofy run the same state handlers.
- **KH2's glide has no wall rule. Checked (decomp).** ACTION_GLIDE's vtable `0x1405CDBF0`: enter `0x140423B90`, exit `0x140423B70`, update `0x140423A70`, step `0x140423920`, input `0x140423BC0`, plus the shared air slots `0x140409AD0`, `0x140409B00`, `0x1403CB8B0`, `0x1403CB940`, `0x1403CB9F0` (Aerial Dodge out of the air). None tests a wall; it ends grounded / under the end height (`0x1404235B0`) or on release. Whether a wall cuts KH2's speed depends on its collision writing the blocked velocity back into `+0x20`: Assumption, not checked. At Glide KH2 KH1's transition wall end (moved < 70%) is skipped.

## 8. Implementation (testing branch, `crates/kh1-hook/src/win/glide.rs`)

Built after the tester's "go ahead" (hover as a sub-mode of 0x15, walls slide, Donald and Goofy glide after
him with KH2's Glide). Every wrapper sits on a slot of KH1's state table (Steam `0x142DB3840`) and acts only for
Sora at Glide KH2 in base form; Donald and Goofy (and Sora at Glide KH1) run KH1's own handler. The table layout,
read again in the registration `0x1402B1B80`: `+0` flags, `+8` enter, `+0x10` exit, `+0x18` pre-step, `+0x20`
step, `+0x28` transition, `+0x30` button check, `+0x38` jump press. **Checked (decomp)** (this settles §2's
**Assumption** on the layout). The button check is called only by the player's control step `0x1402A7B30`, with
`(actor, buttons, input)`. **Checked (decomp)**

### 8.1 Start, enter, exit

- `glide::start`: from KH1's jump or fall state, above KH2's start height (growth `+0x34`), KH1's state set to
  0x15 with KH1's set-state `0x1402B1960` (its enter and exit run), KH1's slow glide request 0x71 with KH2's blend
  16. No KH1 timer read, his speed not halved. His velocity: KH2's stored launch speed when KH2 has one this air
  time (the Aerial Dodge's, `takeover::LAUNCH`), else his speed as it is (KH1 keeps no launch speed for its own
  jump, as the borrow did since t331). **Checked (decomp)** KH2's enter `0x1403CBD30`; that KH2's jump's own
  `+0xE0` (written by `0x1403CBB00`) isn't carried is a known difference kept from the borrow (open).
- Enter (`0x1402B2D00` wrapped): `+0x1CC` bit 0 cleared as KH1 does; the combat state's timer `+0x100` set to 0
  instead of KH1's 46-frame minimum. A glide KH1 started on Sora itself (its Superglide on Square,
  `0x1402A7B30`) is taken over as KH2's start.
- Exit (`0x1402B29F0` wrapped): `+0x1CC` bit 0 set as KH1 does; the timer 0 instead of KH1's 24-frame wait (KH2
  has neither: none in `ACTION_GLIDE` / `ACTION_GLIDEHOVER`'s functions). **Checked (decomp)**
- The fall state's jump press (`0x1402AFD60`, state 2's `+0x38`, wrapped): for Sora at Glide KH2 it no longer
  starts KH1's Glide on the jump (KH2's glide is on Square, the jump in the air is the Aerial Dodge); Neverland's
  flight start (world flag `0x20`) still runs. **Checked (decomp)**

### 8.2 The step

State 0x15's step (`0x1402B2A20`) wrapped. For Sora at Glide KH2: KH2's velocity step on KH2's physics tick (once
every two 60 Hz frames with dt 2, `0x1403B89A0`, as Quick Run's): the glide `facing x ((1 - a) * speed + a * |v|)`
(`0x140423920`), the hover `x 0.9` (`0x140423650`), the fall from KH1's own height over the floor `+0x3D4`
(`0x1404237A0`). The displacement handed back to KH1's physics step is that velocity and fall times KH1's frame
step, so KH1's mover and its collision move him; against a wall KH1's mover slides him along it (KH2's glide has
no wall rule, §7). KH1's +0.4 ramp, its 8 / 16 top speeds and its saw-tooth fall don't run for Sora. **Checked
(decomp)** both games. Not checked: whether KH2's collision cuts `+0x20` on a head-on wall (our velocity isn't cut
by KH1's mover: **Assumption** that KH2's isn't either).

### 8.3 The pre-step: turn, hover, motions

State 0x15's pre-step (`0x1402B2880`) wrapped. For Sora at Glide KH2, each frame:
- **Hover or glide.** The first pre-step picks the hover with the stick neutral, else the glide (`0x1403A8369`).
  The stick takes the hover into the glide with KH2's stored launch speed put back (`0x140423840` → the glide's
  enter `0x1403CBD30`); the glide never goes back to the hover. **Checked (decomp)**
- **Turn.** Toward the stick (`takeover::stick_now`, the stick as KH2 reads it for base Sora) at most the growth
  level's turn speed (`+0x3C`, set by the glide's enter `0x140423B90`) a 60 Hz frame, through KH1's facing set
  `0x140297E10` (its `+0x374` bit `0x1000` lock kept) and `+0x150`, as KH1's pre-step sets them. None in the hover.
  **Checked (decomp)**
- **Motion.** KH2's pick (`0x140423A70`): 205; 206 / 207 when the stick is more than dot 0.98 off his facing, 206
  with the cross product of facing and stick pointing up in KH2's y-down world; 208 in the hover; blend 16. Asked
  every frame through KH1's request `0x14029F7C0` (Steam), which leaves a motion already playing alone. **Checked
  (decomp)**; `0x140142120` is `facing x stick` in that order (**Checked (decomp)**, t-glide fix below); that
  KH1's world and KH2's turn the same way (both y-down) is an **Assumption** (it decides which side is 206).
- **Slots.** KH2's 205 / 206 / 207 play on KH1's glide requests 0x71 (and 0x72) / 0x73 / 0x74 at Glide KH2
  (`kh2slots::GLIDES`, slots 75..78 as templates, so KH1's own looping glide slots); the hover 208 is a request of
  ours (`kh1rows::OURS`, "Glide hover", KH1's air swing as its template). That template holds at its end, so the
  pre-step puts the hover's time back to 0 at its end, as KH2 loops it (**Assumption**: KH2 loops 208).
- **The left bank (207) in a bound set.** KH1 asks 0x73 for a turn to the positive side and 0x74 for the other
  (its pre-step `0x1402B2880`: `'s'` past +30°, `'t'` past -30°, **Checked (decomp)**). The file has 0x74 on slot
  78 (`xa_ex_0010.mset`: 0x73 `0x204D`, 0x74 `0x204E`, **Checked (files)**), but the set Sora is bound to in a
  world (88 slots of its own in the tester's t530 log) has 0x74's entry on slot 77, the right bank's: the log's
  "not in this set: 0x74", and the trace drawing KH1's own slot 77 (KH2's 206 through `pose`) for every left
  bank (**Checked (trace)**). So 207 never played and both sides showed 206. `kh2slots::also_stands_for`: 207
  stands in for 0x74 whether the set's entry holds slot 78 or 77. KH2's pick itself was right: 206 when
  `(facing x stick).y > 0`, else 207 (`0x140142120` is `a x b` with a = facing from `0x1403B56E0`, b = the stick
  `+0x10`, **Checked (decomp)**).
- **Water.** KH1's water check from the glide (`0x1402C12B0`) still runs at the end, as KH1's pre-step ends.

### 8.4 The end

- **Letting go.** State 0x15's button check (`0x1402B2BF0`, called by the player's control step) wrapped: for
  Sora at Glide KH2, Square let go ends the glide or the hover at once (no 46-frame minimum), KH2's glide button
  (the rewrite's binding, t327; KH2's controller ends it so, matched to the exe in t331). **Checked (code)**; where
  KH2's exe tests the release isn't found in the glide action's own functions (an air action's shared slot):
  **Assumption** that it's at once.
- **Transition.** State 0x15's transition (`0x1402B2780`) wrapped: Neverland's flight first, exactly as KH1's
  (world flag `0x20` with `0x1402B1DA0`, action flag 4 off, state 8); then on the ground, or under KH2's end height
  (growth `+0x38`, `0x1404235B0`), the end; else gliding on. KH1's wall end (moved under 70% of the last step)
  doesn't run: KH2's glide has none, he slides along the wall. **Checked (decomp)**
- **Into the fall** (`glide::end`, KH2's `0x1403CBC30`): KH1's fall from rest `0x1402AF8F0 (actor, 0)` (state 2,
  its arc fields from rest, as KH2's fall from rest), `+0x3C0` his velocity kept (no x0.7, no hop of 8), KH1's fall
  request 6 with KH2's blend 16 (KH2 asks its fall motion 4), KH2's launch flag cleared for the rest of the air
  time. KH2's end (`0x1403CBC30` with 0) touches only the arc (`+0xF0` / `+0xF8` / `+0xFC` from rest) and the
  launch flag `+0xEC`, never the velocity `+0x20`: the glide's whole velocity goes on into ACTION_JUMP's fall.
  **Checked (decomp)** KH2 Steam.
- **KH1's carry into the fall undid it (fixed).** A release comes in KH1's control step, inside KH1's think
  `0x1402A3300`: that think clears flag `0x400` first (`0x1402A43A0`), runs the control step (our end sets state
  2, so `0x400`), the new state's pre-step (state 2's `0x1402AFC20` copies the stick into `+0x20`), then, with
  `0x400` and state 2, sets `+0x3C0` = `+0x20` x `+0x2C` (the stick's direction times its tilt, at most 1).
  **Checked (decomp)** KH1 Steam. The tester's t530 traces: 30.1 a frame at the release, 1.1 the next frame,
  then KH1's air steering adding 0.8 a frame ("I just drop"). `glide::carry` puts the glide's velocity back on
  `+0x3C0` in Sora's physics hook before KH1's physics step (as `shared::physics::ground_hop_carry` does for the
  same carry), so KH1's fall step (physics KH1) or KH2's arc on it (physics KH2), which both start from `+0x3C0`,
  go on at the glide's speed. Ends in KH1's physics step (the transition: the ground, the end height) aren't hit:
  `0x400` is cleared at the next think's start (the Aerial Dodge's apex, also in the physics step, kept its speed
  in the same traces). **Checked (trace)** Landing: on the ground KH1's fall state lands him the next frame with its own landing, as KH2's fall action
  does. **Checked (decomp)** both games.

### 8.5 The borrow taken out

- Square in the air at Glide KH2 (`shared::control`, fresh, KH1 letting him act, above KH2's start height) calls
  `glide::start`; the `GLIDE` borrow, its hand-back of the launch speed and the Aerial Dodge out of the borrow
  (t496) are gone. KH2's stored launch speed is still kept from the borrows that launch him (`takeover::LAUNCH`).
- The jump in KH2's glide with the Aerial Dodge left (KH2's shared air slot `0x1403CB9F0`): `glide::end` into the
  fall, then the native Aerial Dodge on KH1's air state (`combomods::aerial_dodge`), once per air time.
- In the glide state at Glide KH2 KH1's control step sees neither the jump nor Square (both KH2's).
- KH2's controller no longer glides base Sora (its Glide level kept aside while he is in KH1's states,
  `KH2_GLIDE_KEPT`, as at Glide KH1): a Square in one of its other borrows (an air combo's) doesn't glide; the
  glide comes once KH1 has him back. Drive Forms keep the controller's glide (their own movesets).
- The Steam build only (the KH1 addresses are Steam's); elsewhere Glide KH2 has no glide.

### 8.6 Donald and Goofy glide after him (the tester's "(b)")

Their AI's glide (`0x1401CF9E0`, §7) reads Sora's ability bits `0x142D5EC10` (`0x80` KH1's Glide, `0x100`
Superglide), which KH1 rebuilds from its menu in `0x1402A6380` (a menu closing `0x1402D53B0` → `0x1402A4E50`, map
setup `0x1402888B0`, an event `0x1402A68D0`). **Checked (decomp)** At Glide KH2, base Sora with KH2's Glide and
neither of KH1's, `glide::party` sets `0x80` each Sora physics frame, so they glide after his jumps with KH1's
glide on their own bodies (KH1's handlers run for them); it's taken off again (only if we set it, and only if he
doesn't have KH1's Glide himself) when he can't glide with KH2's. Sora's only other reader of `0x80` is the fall
state's jump press, which no longer starts KH1's glide on him at Glide KH2 (§8.1). **Checked (decomp)**, a grep of
every read of `0x142D5EC10`'s `0x80` / `0x100` bits (the AI's three functions, `0x1402AFD60`, `0x1402A7B30`).

### 8.7 The values, checked frame by frame (the tester's t530 traces)

KH2's growth table (`03system.bin` `pref` / `fmab`, records 0..3 = Glide LV1..MAX, **Checked (files)**): speed
16 / 20 / 24 / 32, fall ratio 0.015 / 0.015 / 0.015 / 0.01, fall height 0 / 30 / 30 / 30, fall max 3 / 2.5 / 2 / 1,
acceleration 0.91 / 0.93 / 0.95 / 0.97, start height 60, end height 10, turn speed 0.0524 (record 4, speed 64, is
not one of Sora's levels). Against the traces (Glide LV4 in the log), **Checked (trace)**:
- Speed on KH2's tick (a new value every second frame): 18.42 → 18.83 → 19.22 ... = 0.03 x 32 + 0.97 x |v|.
- Hover: 16.20 → 14.58 → 13.12 ... = x 0.9 a tick.
- Fall: 1.00 a frame while (height - 30) x 0.01 is over 1; 0.98 at 128 over the floor, 0.96 at 126 ...
- Turn: 0.052 rad a frame toward the stick (KH2's turn `0x1403B9F70` doubles `+0x134` for its 2-frame tick).
- Banking past dot 0.98 (a stick about 11° off his facing); the start with KH2's stored launch speed (18 after
  an Aerial Dodge, 0 after a neutral one), back when the stick takes the hover into the glide.
- Start height 60 (every start in the log was above it).
Wrong in t530: the left bank (§8.3) and the release (§8.4), both fixed.

### 8.8 The ability list follows the Glide style at once (glide thread m6)

The swap of KH1's Glide / Superglide for KH2's own Glide (id 88, `growth::glide_swap`, t323) ran only from
`menu::Growth::update`, every 60th call, and every call comes from one of Sora's physics frames
(`takeover::shared_frame` → `refresh_kh2_numbers`, the borrow steps). While KH1's pause menu is up Sora has no
physics frames, so a Glide style switched in the Config menu left KH1's Abilities screen showing KH1's Glide and
Superglide and no KH2 Glide until the game ran again for a second. **Checked (code)**; the logs agree: every
"Glide style" line comes a few dozen frames after the Config save, never while paused (the tester's t544 log,
f16914 → f16963, f17162 → f17203; the PC's t567 log likewise). Now `config::set` for the Glide row calls
`menu::glide_style_now` on the spot (KH1's menu update, the game's thread): the same swap, KH2's Glide's name
with its level, logged ("abilities: Glide style ... set in the Config menu"). KH1 rebuilds Sora's ability bits
(`0x142D5EC10`) itself when the menu closes (`0x1402A6380`, §8.6). Assumption: that this is what the tester saw
(no log of the report's session; the swap itself works in both logs I have).

### 8.9 Changing bank mid-glide (glide thread m6: "he resets his animation")

What KH2 does, **Checked (decomp)**: the glide update `0x140423A70` asks every frame for 205, or 206 / 207 by the
side of the turn, through `0x1403C6DC0 (holder, motion, 16.0, 0)`. That call does nothing when the motion asked
is already playing; otherwise it starts the new one at time 0 (its last argument) with a 16-frame blend. The only
motion pair whose play time carries over is ids 1 / 2 (walk / run: `fmodf` of the time scaled by the two
lengths); 205..208 aren't. So in KH2 a bank change (206 → 207, and any 205 between when the stick crosses the
middle or comes within dot 0.98 of his facing) restarts the new loop from frame 0, blended over 16 frames.
205 / 206 / 207 are 30-frame loops at 30 fps with return frame 0, 208 a 50-frame loop (`P_EX100.mset` A170..A173,
**Checked (files)**).

What KH1 does with the same requests, **Checked (decomp)**: `0x14029F7C0` returns when the slot asked is
playing (`+0x168`) and otherwise queues it with start 0 (for the player; only enemies get a walk / run phase
sync there), and `0x1402A0350` applies it: a blend of `+0x170` = 16 frames, or, when a blend is still under way,
its mid-blend path `0x140393B30`, which keeps the sampled channels of `+0x1B0` as the pose it blends from. KH1's
own glide pre-step only asks when `+0x170` is 0 (no blend under way); ours asks every frame, as KH2's does.

So the restart at frame 0 is KH2's own rule, and nothing in the code shows our bank change doing anything KH2's
doesn't. Not checked: how KH2's blender treats a request in the middle of a blend (its layered holder
`0x1403C88C0` / `0x1403C8CD0`), against KH1's mid-blend path. The glide now logs every motion change with KH1's
slot, time and blend fields before the request ("glide (native) motion: ..."), to compare with a trace.

### 8.10 KH2's blend clock on the glide's motions (glide thread m7: "make it 1:1")

The tester's t596 trace (f60998..f61178) and log, read against both decomps:
- **The bank restart is KH2's.** KH2's motion update `0x1403C6740` holds the new motion at its start
  time (`+0x4C`) while a blend runs: `+0x44` is then the blend clock, and only when it reaches the
  length `+0x48` does the time become `clock - length + start`. KH1's update (Steam `0x14029F0A0`)
  does the same with `+0x174` / `+0x170` and `+0x16C`. **Checked (decomp)** both; the log agrees
  (KH1's time 18 thirty-four frames after a bank request: held 16 frames, then played). The trace's
  one-frame 205s between 206 and 207 are the stick passing within dot 0.98 of his facing, as KH2
  picks too. **Checked (trace)**.
- **KH2's clock starts at 2** (`0x1403C8A40`: `+0x44 = 2.0`, the length in `+0x48`, the start time
  in `+0x4C`), and the glide's request comes from its action update, the object's vtable `+0x28`
  (`0x1403B2DA0` -> `0x1403B4780` -> the action's `+0x38`, `0x140423A70`; OBJ / STDOBJ / ACTOR
  vtables, RTTI), which `0x1403BFD30` calls after that frame's motion update. So KH2 draws the
  frame of the request at weight 2/16 and ends the blend 14 frames later. KH1's apply
  (`0x1402A0350`) starts `+0x174` at one frame step (`dt x +0x284`): weight 1/16, the end 15
  frames on, one frame behind. **Checked (decomp / exe: the vtable bytes)**.
- **KH2 skips frame 1 of a start.** Its update turns a time stepping from exactly 0 to 1 into 2,
  so a motion coming out of its blend at time 0 plays 0, 2, 3 ...; KH1 plays 0, 1, 2.
  **Checked (decomp)**. Assumption: KH2's motion step per 60 Hz frame is 1 (the parameter isn't
  visible in the decomp's call).
- **Now** (`glide::kh2_clock`, in the pre-step before its request): on the first frame after KH1
  applied a glide motion (205..208) with a blend, its clock is put at KH2's 2; on the first frame
  after such a blend ended at time 0 (at 60 fps), its time is put at 1, which KH1's update makes 2.
  From the second frame on, weight, blend end and loop phase are KH2's; the request frame itself
  still draws KH1's 1/16 (KH1 applies and poses inside its update). KH1's events between time 0
  and 1 of these loops aren't fired on that frame (KH2 fires 0..2; none known on 205..208).
  Logged: "glide (native) motion ...: KH1's blend clock ... -> KH2's 2.0", "... KH2's skip of
  frame 1" (first 12).

### 8.11 Bank to bank: a fresh blend from the pose on screen (glide thread, after m7) — replaced by §8.13

**Replaced (t646 removed):** forcing KH1's fresh path mid-blend garbled Sora (glide thread m8); see §8.13 for
why and for the fix that replaced it. The findings below on the two paths still hold.

The tester's t6xx trace (f109997..f110535, every bank change 207 -> 205 -> 206 or 206 -> 205 -> 207, the
205 lasting one frame as the stick crosses his facing) and "I'm still seeing the same phenomenon":
- **KH1 chains a request that comes during a blend.** KH1's request `0x14029F7C0` empties the queue
  (`+0x178`) and queues the request only when its slot isn't the one playing (`+0x168`); the queue is
  applied in KH1's animation update `0x14029F0A0` by `0x14029FC40` -> `0x1402A0350`. With a blend under
  way (`+0x170 > 0`) the apply goes to `0x140393B30`, which moves the last blend's *target* buffers
  (`+0x1C0` local matrices, `+0x1C8` channels: the interrupted motion's start frame) into the new
  blend's *source* (`+0x1BC`, `+0x1C4`). With no blend (`+0x170` 0) it goes to `0x140394080`, which
  frees all five blend buffers and takes the source from `+0x1B8` / `+0x1B0`. KH1's blend pass
  (`0x1402A0620` -> `0x140392560`) writes the blended pose into `+0x1B0` (channels, lerped from
  `+0x1C4` to `+0x1C8`) and `+0x1B8` each frame, so the fresh blend starts from the pose on screen.
  **Checked (decomp)**, Steam KH1.
- So the second change of a bank switch (205 -> 207, one frame after 206 -> 205) blended from 205's
  frame 0, not from the 206 lean on screen: he snapped to the straight glide's first frame, then
  blended into the new bank. That is the "reset".
- **KH2** asks its glide motion through `0x1403C6DC0` -> `0x1403C86A0`, which ends every layer of the
  holder (`0x1403C70D0`) and starts one new blend (`0x1403C88C0` -> `0x1403C8CD0`); nothing is
  chained. Its blend source is the pose on screen in the OG rewrite's rig (`kh2-rig`,
  `decompose(&self.bones)`, matched to the exe for the Phase 2 blending notes). **Checked (decomp)**
  that nothing chains; that the source is the drawn pose: **Checked (code)** (the rig port), not
  re-traced in the decomp.
- **Now** (`glide::fresh_blend`, in the pre-step right after its request): when KH1 queued the glide
  motion (`+0x178` >= 1 after the call) while a blend was under way (`+0x170` > 0), `+0x170` is put at
  0, so KH1's apply takes its fresh path. The new motion still starts at frame 0 and is held there
  through KH2's 16-frame blend, as in both games. Logged: "glide (native) motion N asked ... into
  KH1's blend: a fresh blend from the pose on screen" (first 20). Glide motions (205..208) only, for
  Sora at Glide KH2.

### 8.12 The hover after KH2's fall (glide thread, after m7)

"neutral glide resets its animation wrong when setting jump to kh2 and doing a glide out of jump": at
Jump KH2 the fall before the glide is our baked KH2 fall, whose own loop range (12..32) leaked onto
the hover (208 looped 13..32 of its 100 frames; the same would have hit 205 started out of that
fall). Not the glide's code: `kh2slots::loop_range`, fixed there; `docs/kh1-anim-baking.md` §7.2.
**Checked (decomp, trace)**.

### 8.13 Bank to bank: KH1's chained start, its source the pose on screen (glide thread m9)

"sora now disappears and gets all garbled when turning in glide" (t646). The tester's log (t651 build,
f2019..f2265: 13 "asked N of 16 frames into KH1's blend: a fresh blend" lines, every rocking bank change
206 -> 205 -> 206 four to fifteen frames into the last blend) and trace (f13715..f13812: the 205 / 206 /
207 changes one to nine frames apart, the pose path "pass x1 + ik x1" on each change, "ik" between).

- **Why t646 garbled him.** KH1's blend pass (Steam `0x140392560`, from `0x1402A0620`) does not leave local
  bone matrices in `+0x1B8`. For each bone with flag bit 30 (bone table `+0x2C`; 142 of Sora's 293, in
  `xa_ex_0010.mdls` the animated joints; the pass also flags the parents of the IK list's bones for its
  call), it lerps the source and target *products* of the bone's chain up to the next flagged bone
  (`+0x1BC` / `+0x1C0`), orthonormalises, and writes `+0x1B8[bone]` as that times the flagged ancestor's
  `+0x1B8` (and the scaled world `+0x1B4`); IK bones get their solved matrix. The 151 unflagged bones are
  not written, and both blend starts end by allocating a new `+0x1B8` (`0x14028DA20`, not cleared). The
  fresh start `0x140394080` moves `+0x1B8` into the source `+0x1BC` and reads it as local matrices, which
  is right after a plain pass (`0x140391C70`) but not during a blend: accumulated matrices and
  uninitialised blocks chained together. **Checked (decomp)**; that this is what the tester saw is
  inferred (every garbled moment is a mid-blend change in the log, and nothing else changed in t646).
- **The buffers** (Checked (decomp)): `0x14028DA20 (size, &slot)` allocates from KH1's movable heap and
  records the slot as the block's owner; `0x14028DB20 (block, &slot)` re-owns a block to another slot;
  `0x14028DB70 (block)` clears the owner slot (the block is reclaimed later). `0x140393F60` frees all
  five blend buffers (the apply's no-blend branch). The chained start `0x140393B30`: frees the old
  source, re-owns the old target (`+0x1C0` -> `+0x1BC`, `+0x1C8` -> `+0x1C4`), the root offset's source
  `+0x110` = the old target `+0x120`, keeps the IK list entries whose type the new motion shares with
  their goal's source = the old target (`+0x10 = +0x20`), poses the new motion (`+0x1B8` / `+0x1B0` ->
  the new target `+0x1C0` / `+0x1C8`, IK goals' targets from `+0x1B4`), and allocates a new `+0x1B8`.
  KH1's apply `0x1402A0350` then sets `+0x170` / `+0x174` and runs the pass at once (the request frame).
- **The lerp** (Checked (decomp)): `0x1400E64D0` / `0x1400E7CB0` are `out = a * t + b * (1 - t)` with
  `a` the target (the vec3 one keeps `a.w`); the pass's weight is `+0x174 / +0x170`.
- **Now** (`glide::chain_snapshot` / `chain_source`; t646's `+0x170` clearing removed; KH1 keeps its
  chained path): in the glide pre-step, when the glide motion just asked was queued while a blend runs
  (`+0x178` >= 1, `+0x170` > 0), the pose drawn last frame is kept in the source buffers' own formats:
  each bone's last target and source matrices lerped at the drawn weight (KH1's clock read before
  `kh2_clock` moves it), the first vec4 of each `+0x1B0` channel record (what the fresh path takes),
  the root offset `+0x100` (the fresh path's `+0x110 = +0x100`), and each IK goal lerped as the pass
  does. The first blend pass on Sora's instance after that (KH1's apply calls it right after the chained
  start; our `pose::pose_ik` hook, before KH1's pass) checks the start happened (`+0x1BC` is the old
  target's block, `+0x1C4` the old target channels', `+0x1C0` a new block, same bone count) and writes
  them over `+0x1BC`, `+0x1C4`, `+0x110` and the IK goals' sources. Otherwise (KH1 took another path, or
  its heap moved the blocks) nothing is written and KH1's own start stands (logged). Logged: "glide
  (native) motion N chained into KH1's blend at W of it: its source is the pose on screen" (first 20).
  Glide motions (205..208) for Sora at Glide KH2 only; no new hook (the blend pass was already hooked).
- **Assumption:** a flagged bone whose chain has more than one link that changes between the two
  motions (an unflagged link animated too) gets the product of the lerped links as its source, not the
  lerp of the products the pass drew: equal when the other links match, otherwise a second-order
  difference (small for one-frame bank changes). That KH2 blends from the drawn pose: as §8.11.

