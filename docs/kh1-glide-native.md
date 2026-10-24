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
  (decomp)**; that `0x140142120` is `facing x stick` in that order is an **Assumption** (it decides which side
  is 206), and so is that KH1's world and KH2's turn the same way (both y-down).
- **Slots.** KH2's 205 / 206 / 207 play on KH1's glide requests 0x71 (and 0x72) / 0x73 / 0x74 at Glide KH2
  (`kh2slots::GLIDES`, slots 75..78 as templates, so KH1's own looping glide slots); the hover 208 is a request of
  ours (`kh1rows::OURS`, "Glide hover", KH1's air swing as its template). That template holds at its end, so the
  pre-step puts the hover's time back to 0 at its end, as KH2 loops it (**Assumption**: KH2 loops 208).
- **Water.** KH1's water check from the glide (`0x1402C12B0`) still runs at the end, as KH1's pre-step ends.
