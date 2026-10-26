# Drive Forms native (research, t480 round)

Research notes for the tester thread `testers/drive-forms-native/`. Plain language only, no
decompiled code. Addresses are **Steam** (image base `0x140000000`) unless marked Epic. Tags:
**Checked (decomp)** read in the Steam Ghidra decomp, **Checked (exe)** read from the Steam exe's
bytes, **Checked (code)** read in the testing branch's code, **Assumption** not checked.

## 1. What the testing branch has now (Checked (code))

- `crates/kh1-hook/src/win/form.rs`, `drive.rs`, `formmagic.rs`, `valor.rs`, `valorstats.rs`,
  `leftblade.rs`, `tint.rs`.
- Forms: Valor and Wisdom in the Drive list (`drive::FORMS`, records 0x7F / 0x7E), Final only by
  the secret pad code (`form::pad`, called from `probe.rs`). No Limit, Master or Anti.
- The whole form runs on KH2's controller (`takeover`'s Driver: `form::character` / `form::battle`
  swap the Character and its Battle data for the form's moveset). `shared::on()` is false in a form,
  so none of the base-Sora native work (KH1 states, `kh2slots` slot/request tables, `kh1rows`
  rows, `kh1phys` physics) applies in a form. The shared-states header says so: "Drive Forms keep
  KH2's controller all through".
- Cost and length: 3 MP to enter (`COST_MP`), drained one yellow MP unit every 3 frames of battle
  through KH1's own payment (`after_step`, Epic `0x1402A39D0` = Steam `0x1402A5B60`), revert at 0 MP,
  on a KO and on the Revert command. No Drive gauge, no form gauge, no Drive orbs.
- Levels: a fixed `LEVEL = 1`; the growth ability is always MAX (`high_jump: 4`, `quick_run: 4`,
  `glide: 4`). No form EXP.
- Wisdom's shots: KH2's `wisd` script run by `kh2-magic` (`formmagic.rs`), hits through our own
  strike checks. Final's glide: KH2's controller's glide.

## 2. KH2's Drive gauge and form gauge (Steam KH2 decomp)

The gauge lives in the party member's status record (the player object's `+0x5C0` points at it).

| Field | What | Tag |
|---|---|---|
| status `+0x1AF` | gauge mode: 1 normal (Drive bars), 2 in a form, 3 a summon, 4 another (set at a scene) | Checked (decomp) |
| status `+0x1B0` | points in the current bar, 0..100 | Checked (decomp) |
| status `+0x1B1` | full bars | Checked (decomp) |
| status `+0x1B2` | maximum bars | Checked (decomp) |
| status `+0x1B3` | the form gauge's bars (form level + 3) | Checked (decomp) |
| status `+0x1B4` / `+0x1B8` | form gauge now / its full length, frames (floats) | Checked (decomp) |

- **Adding Drive points** `0x1403C0AD0 (status, n)`: points carry into the next bar at 100 up to the
  maximum, where the gauge holds at 100 in the last bar; a negative n takes from the bar and breaks
  a full one. Checked (decomp).
- **The hit's add** `0x1403A9CF0 (player, value)`: mode 1: × the MP-Charge factor (`sstm +0x78` ×
  Drive Boost's record `+0x40`) while MP charges, × a world factor (`0x1403A3990`), truncated, then
  `0x1403C0AD0`. Mode 2 (a form, skipped when the player's form id `+0xDE0` is 0xC) and 3: the form gauge
  gets `value × sstm +0x70` frames. Callers include the prize pickup and the attack paths. Checked
  (decomp). (`+0xDE0` is 6 in Anti, `0x1403C0B50`; what 0xC is wasn't checked.)
- **The form gauge change** `0x1403C0B50 (status, frames)`: adds to `+0x1B4` within 0..`+0x1B8`; in
  Anti (`+0xDE0 == 6`) a gain is turned into a loss (orbs drain Anti). In summon mode it also counts
  bars lost for the summon's level. Checked (decomp).
- **The form's drain**, in the player step `0x1403A92C0`: in modes 2 / 3, a step's time (`DAT
  0x140717480`, one frame) × `sstm +0x6C` out of battle, × Form Boost (`+0x44`) in a form or
  Summon Boost (`+0x70`) for a summon, taken through `0x1403C0B50`. Checked (decomp). `sstm` is the
  pointer at `0x142AE5760` (03system `pref`); its numbers (`+0x68` 600 a bar, `+0x6C` 0.5) come
  from the data, not checked here (Assumption, matches `character_core::drive` and `drive-model.md`).
- **Entering a form** `0x1403C17C0 (status, form, level)`: HP and MP refilled (`0x1403C0C30`,
  `0x1403C0CD0`), MP Charge cut, mode 2, `+0x1B3` = level + 3, the form gauge full at
  `(level + 3) × sstm +0x68`. Anti (form 6) uses `sstm +0x8C − 2` as its level. Called from the
  change task `0x14040CC00`, which reads the level from the save. Checked (decomp).
- **Reverting when the gauge is out**: the form state's think `0x140404A40` checks `+0x1B4 <= 0`
  and starts the revert task (`0x14040CD40`, made by `0x14040CAA0`). Checked (decomp).
- **Form records in the save**: `0x1403A1410 (save, form)` = save `+0xE04 + (form − 1) × 0x38`
  (10 records), level byte `+2`; the save is `0x1409ABDA0`. Checked (decomp). Form EXP adds (fmlv
  table) not located on Steam this round; the rules are in `character_core::drive` (Epic-derived)
  and `docs/drive-model.md`. Summon EXP is `0x14040D530` (not forms). Checked (decomp).
- **Raising the maximum bars**: `0x1403EDF90` adds a bonus's bars to `+0x1B2`. Checked (decomp).
- **Drive Recovery items**: `0x1403C4B80` adds `n × 100` points. Checked (decomp).
- **Drive orbs**: the prize pickup `0x1403DCA50` reads an 8-byte row per prize type from
  `0x1405C7420`: a float, then HP, MP, Drive and munny bytes, and gives the Drive byte through
  `0x1403A9CF0`. Rows (Checked (exe)): types 7 and 8 are the Drive orbs, Drive 1 and 3 (types 0/1
  HP 1/5, 2..4 munny 10/5/1, 5/6 MP 1/3). The munny→orb table for Drive Converter is at
  `0x1405C7978`, used by the drop `0x1403DC170`. Checked (decomp).
- **The Anti counter and roll**: not located on Steam this round. The rule (counter +1 per non-Final
  Drive, 10% from 5, 25% from 10, Final −10, Anti −6, battle only) is in `character_core::drive`
  from the Epic exe. To find on Steam next.

## 3. KH1's matching systems (Steam KH1 decomp)

KH1 has no Drive gauge, no form gauge and no form objects. What it has that a native port can use:

- **Hit contact** `0x1402BF330 (attack, target)`: the attack touched an actor; queues the hit
  record and, for an attacker with stats, charges MP with `0x1402A4AE0 (actor, units)` from the
  attack data's `+0x2C` (capped by a battle-data factor). This is where KH2 adds Drive on a hit, so
  the gauge's hit add belongs here (or right after it), on the same record. Checked (decomp).
- **HP change** `0x1402A4920 (actor, amount)` (keeps the HUD mirror): the place for Damage Drive and
  for the refill on entering a form. Checked (decomp, callers).
- **MP payment** `0x1402A5B60` (what the form uses now). Checked (code map + decomp).
- **Prizes (orbs)**: make `0x1402AB2A0 (type, position, mode)`; enemy drop `0x1402ABC40` (from the
  AI script's drop ops `0x1402BC640` / `0x1402BC6B0`); pickup `0x1402AC080 (prize, position)`, whose
  switch gives the reward for types 0..6 only (0..2 munny `0x1402E7E10`, 3/4 HP `0x1402A4920`,
  5/6 MP `0x1402A4AE0`); row table `0x140528BE0` (7 rows of 0x14: height, pull range, sound, ...),
  read in the pickup and the prize steps; the prize task `0x1402ACAD0`. Checked (decomp). The
  drawing (the `pppRyjMegaBirthPrize` program's 10-entry data block, KH1 uses 0..6) is from the
  main branch's Epic notes and the strings: Assumption on Steam until reread. KH2 has the same
  program name (`0x1405FB700` string in KH2), so its Drive orb art fits the same entry layout
  (Assumption).
- **HUD**: Sora's HP/MP gauge task `0x14026DB00` (no direct callers, registered by pointer from
  `0x140268F82`), the gauge record helper `0x14026D990` (16 callers: every HUD gauge). A Drive gauge
  drawn with KH1's own sprite calls inside that task is the native draw. Checked (decomp) for the
  functions; what each field is was read on Epic in the main branch (Assumption on Steam).
- **Motion tables, rows, states** (what base Sora's native moves already use): the slot table
  `+0x1D4` and request table `+0x1D8` copies (`kh2slots`), the attack row tables
  `0x142D5CAB0[object +0x4C]` with Sora's 66 rows and ours from 0x200 (`kh1rows`), the state table
  and setter (`0x1402B1960`, Epic `0x1402AF7D0`), physics fields (`docs/kh1-physics.md`,
  `kh1phys`). Checked (code), with the addresses in those docs.
- **Intangible** (actor flag 0x40, `docs/kh1-intangible.md`): KH1's own invulnerability for the
  Drive motion. Checked (code / doc).

What KH1 reads that a form would change (don't break KH1's events):
- The attack tally `0x14028C660 (0x13)` (Dive to the Heart's tutorial) is counted by KH1's row
  runner; form rows run through the same runner keep counting. Checked (code / kh1rows doc).
- Sora's motion request `+0x164` (scripts' opcode 322, motion-bound attack objects): KH1 rows keep
  KH1 request ids, so scripts see what they expect. Checked (code doc).
- Sora's object id `+0x4C` (party, menus, row table index): must not change; swap table pointers,
  not the object. Assumption (the readers weren't all listed this round).
- MP: nothing story-side reads it that we know of; the gauge moving off MP removes a side effect
  (MP was drained by forms). Assumption.
- Events, room changes and actor re-creation reset the tables (`kh2slots` reapplies per new actor);
  a form must re-point them the same way, or revert (the existing scene reverts). Checked (code).

## 4. How a form could run through KH1's own states

The model that already works for base Sora: KH1's state machine owns him; KH2's motions are KH1
slots (`kh2slots`), KH2's moves are KH1 attack rows (`kh1rows`) with KH2's numbers on their hits,
KH2's physics written into KH1's physics fields (`kh1phys`). A form is the same thing with another
set of tables:

1. On the change, Sora's actor keeps its object; his slot table and request table point at the
   form's copies (the form's KH2 motions, retimed onto KH1 templates as base Sora's are), his row
   table pointer at the form's rows (its PTYA rows as KH1 rows, ids from 0x200 as now), and
   `kh1phys` gets the form's `prty` numbers (run speed, jump, gravity). On revert they go back.
2. The change itself: KH2's Drive motion (52) and arrival (53) as KH1 slots on a KH1 action state,
   intangible through it (flag 0x40); the screen tint and effects as now.
3. What KH1 has no state for: KH2's form-only moves (Final's glide hits, Valor's two-blade swings,
   Wisdom's shots, Limit's moves, Anti's) need KH1 equivalents:
   - two weapons: a second attack object per row (KH1's row start makes one from the data; a second
     one tied to the left-weapon bone). Assumption that KH1's attack object can take another bone;
     to check in `0x1402B3270` (Steam make attack object).
   - Wisdom's shots: KH1's free-standing attack objects (pool `+0x30` bit 0x400, what Wakka's ball
     and KH1's spells use) instead of `kh2-magic`'s script; the homing (1.8° a frame, speed 20 → 80)
     would be ours on KH1's object. Partly native.
   - Final's glide: KH1's glide state (0x15) with Final's glide numbers, its Keyblades' hits as
     motion-bound attack objects of the glide slot.
4. Hits and reactions on Sora in a form: KH1's hit pass, as the "hits" thread does for base Sora.

Performance: this removes the second simulation in forms (KH2's controller, rig and hit checks
every frame), which is the main cost on low-end PCs now. It adds per-form table copies (made once
at load, a few KB each) and no new per-frame hooks beyond base Sora's.

## 5. Staged plan (one system per version)

1. **Drive gauge data**: bars, points, maximum in Sora's KH1 save (free bytes to find and check),
   KH2's add rules (`0x1403C0AD0`), the hit add at KH1's contact (`0x1402BF330`) with the move's
   ATKP drive value; forms cost KH2's bars instead of MP.
2. **The gauge on KH1's HUD** (`0x14026DB00`, KH1 sprite calls; KH2's gauge art if it fits).
3. **Form gauge and form levels**: `(level + 3) × 600` frames, KH2's drain and hit drain, KH2's
   form records' levels and EXP rules (stored in KH1's save), growth ability by level (Lv1/3/5/7 →
   LV1/2/3/MAX); refill HP/MP on entry.
4. **Drive orbs**: KH1 prize types 7 / 8 (a copied row table, the pickup extended past type 6),
   KH2's drop rows, KH2's orb values (1 / 3; × the form factor in a form, a loss in Anti).
5. **One form native at a time**, Valor first (no magic, two blades): its tables, rows, physics.
   Then Wisdom (shots as KH1 attack objects), Final (glide), then the missing forms: Master, Limit,
   Anti (with the Anti roll).

## 6. Not found this round

- KH2 Steam: the Drive command's cost check, the Anti counter / roll, form EXP adds, the orb add's
  float argument (the call passes it in a register the decomp drops).
- KH1 Steam: free save bytes for the gauge and levels; whether KH1's attack object can be bound to
  a second weapon bone.

## 7. Valor Form on KH1's own states (first step of plan item 5)

The tester (what-is-still-missing, after m11): "I want you to transfer drive forms as well", "go
ahead". Valor first; Wisdom and Final keep KH2's controller for now. Nothing of the Drive gauge,
cost or unlocks changed (open questions in the Drive Forms thread): the form still costs 3 MP and
drains yellow MP as before.

**What runs where now (Valor, Steam, `shared = on`, `kh1_slots` on):**

- **Slots** (`kh2slots::VALOR_SPECS`, the Valor part of `prepare`). Valor's own motions from
  `P_EX100_BTLF.mset`, baked into KH1 motions like base Sora's (`pose::Baker::poses_of` with Valor's
  rig, weapons and root travel, `pose::Src`), as slots of their own after base Sora's. On KH1's
  requests: idle 0 / walk 1 / run 2 (Valor has no field variants, its battle ones on every KH1
  slot), the jump and High Jump takeoffs 0x4 / 0x9 (both Valor's 201), fall 0x6 (and 0xB), landing
  0x7 (and 0xC), the reactions 0x48..0x4B / 0x4E / 0x4F (10..15), the recoils 0x6E / 0x70 (41 /
  43), the item use 0x3E / 0x3F (9), the ground combo 0xC8..0xCB and air combo 0xCC..0xCE; on our
  own rows (`kh1rows::OURS`) Valor's 151, 152, 153, 154, 155, 181, 182, 183, 184 and Finishing
  Plus's next finishers, and Valor's arrival 53 (a new row of ours, its request only). Base Sora's
  Drive motion 52 is a new row of ours too (`kh1rows` "Drive"). Checked (files): every one of those
  motions is in Valor's file (and 52 only in Sora's). Each Valor motion is a `Move` with `form`
  set; its key in the slot cache has the moveset in it, and the cache's manifest notes Valor's files
  (`pose::load_form_noting`), so the first start after this version bakes everything again once.
- **The request tables** (`kh2slots::redirect`): in Valor (`form::native_now`) every request with a
  Valor move plays Valor's, whatever the move styles (a Drive Form is KH2's all through, as on KH2's
  controller); every other request keeps base Sora's. Out of Valor, Valor's are left alone. Clips
  looked up for the slot playing come from its own moveset (`kh2slots::clip_of`, `pose::kh2_moves`):
  KH2's turn to the lock, edge stop, no-gravity hold, Finishing Plus window, re-arm, invulnerable
  and guard windows, and the ATKP of each hit (`kh1rows::kh2_atkp`), are Valor's.
- **The combo pick**: KH1's own pickers with KH2's pick in them (`kh1picker::at_kh2`), from Valor's
  PTYA (moveset 2, Checked (files): rows 0..10 the plain combo, 12 / 38 the Square finishers, 11
  the rising slash, 40 its Aerial Recovery; no ability rows). KH2's controller's data is swapped to
  Valor's (`form::native_data`), only read (KH2's row checks and lengths), never stepped. In Valor
  none of KH1's own ability finishers come out (their bits held back, no KH1 walk).
- **Physics** (`takeover::refresh_kh2_numbers`): KH2's numbers on KH1's states are Valor's (`prty`
  moveset 2, Checked (files): walk 2, run 12, jump 235, gravity 20/49, fall cap 16, air keep 0.9,
  drag 0.95), its High Jump at MAX (585, `fmab` growth 4, as KH2's controller gave Valor), for both
  of KH1's takeoffs (KH2's jump start with High Jump, Steam KH2 `0x140422630` / `0x1403B59B0`,
  Checked (decomp) at t627). The physics row's KH1 setting doesn't apply in Valor (a Drive Form is
  KH2's, `shared::physics::source`).
- **The change** (`form::native_start` / `native_frame` / `native_arrive`, from
  `takeover::shared_frame` each physics frame of Sora's): the Drive command reaches KH1's command
  step (`combat::command`, no borrow); KH1's request of ours for Sora's Drive motion 52 with KH1's
  action flag, KH1's idle (or fall) queued; MP paid, HP refilled, KH1's voice, as before. Motion 52's
  own ranges (Checked (files): no gravity and invulnerable all through, guard 124 on his body, hit
  125 at 18..28) work through KH1's states as every KH2 move of ours: held in the air
  (`kh2slots::no_gravity`), no hit reaches him (`hurt`, `kh2slots::invulnerable`), its hit a KH1
  attack object (`bodyhits`). After 39 60 Hz frames (the tester's 45-frame change, as before), the
  swap and Valor's arrival 53 on its request of ours.
- **Revert** (`form::native_revert`): base Sora's slots and data again; what was playing cut into
  KH1's idle (ground) or fall (air), action flag off; the revert's light and sound as before. On
  the Revert command (`form::accept_now`: KH1's can-act or KH1's combo window,
  `native_can_change`), out of MP, or knocked out.
- **The second Keyblade**: still drawn by `leftblade` (a second queue of the model; not native),
  now fed from the baked frame (`pose::follow_baked` evaluates Valor's rig at KH1's time in the slot,
  `leftblade::kh2_left` / `note_pose`).

**Not native yet / not done in this step:**

- (The left Keyblade's hits: done in t680, a KH1 attack object of their own, §7.2.)
- (Valor's effects: from Valor's own package, the Drive's and arrival's whole frame-0 sets, t-vfx,
  docs/kh1-anim-baking.md §13.5.)
- (Valor's Square moves: done on KH1's rows in t679, §7.1.)
- Valor's guard windows (arrival 53, 155's left-blade guard 162) are not KH1 guard objects
  (`takeover::kh2_hits` skips a form's).
- The arrival 53 isn't held in the air (KH2's `play_held`): it has no no-gravity range of its own,
  so in the air he falls through its 6 frames. **Assumption** that this is unnoticeable.
- `native_can_change`: KH1's can-act or combo window stands for KH2's `can_change_form`.
  **Assumption**.
- Not checked in game; offline the Valor bakes weren't run (KH1's skeleton and motion table aren't
  in the extracted files here).

### 7.1 Valor's Square moves on KH1's rows

The tester (what-is-still-missing, after m12): "3. do both", "make sure when your moving these over
to be native that you double check everything ... check the kh2 c decomp steam".

**KH2 (Steam KH2 decomp, Checked (decomp)).** The player's action-button handler `0x1403A81F0`
(the Square press) tries, in this order:

1. a state-7 case (`+0xDE0 == 7`, action 0x16; not identified, **Assumption**: not Valor's);
2. the glide (growth kind 3, airborne): Valor has none;
3. the Square combo rows 0x25, 0x0C, 0x26 through `0x1403A9950 (player, id)`;
4. with the stick at rest (`+0x1C == 0`), row 11 through `0x1403A9C70`;
5. the dodge roll (growth kind 4, motion 0xD1), else Quick Run (growth kind 1, motion 0xCB), else
   row 11 through `0x1403A9C70` again. So with no roll or Quick Run, row 11 is tried moving too.

`0x1403A9950`: the row by id in the moveset's PTYA (`0x1403D74E0`, 0x44-byte rows); its check
`0x140402750` (motion set, ability `+0x40` equipped if any, air flag `+4` bit 0 equal to airborne,
then a non-finisher `0x1403D7850` / a finisher `0x1403D7780(p, offset)`); a non-finisher (`+4`
bit 2 clear) refused by `0x1403D7780(p, 0)` (the finisher window `+0x194` bit 5, or the count
`+0xD74` at the length `[+0x5C0] + 0x200` ground / `+0x201` air); then the combo window
(`+0x18C` bit 10, range id 11) or the finisher window, and the Attack step's count check
`0x1403D6CA0 (p, connected)` (connected: `+0x120` bit 25). Note these checks read the count
before `0x1403D6CA0` moves it (the Attack walk checks after), so a non-finisher Square row is taken
while the count so far is under the length. Started by `0x1403D78D0` (facing the primary target
`+0xDC8`, its hop).

`0x1403A9C70`: row 11 by id, its check `0x140402750`, the free state `0x1403BFB30` (vtable `+0x78`
and `+0x18C & 0x1C`, range ids 0 / 1 / 2), started by `0x1403D78D0`, then the stick input
(`+0x10..+0x1F`) cleared. No combo window: only from a free state, where the handler's top has
reset the counts (`0x1403D7C60`: 1 / 1).

**Valor's rows (Checked (files), `00battle.bin` PTYA moveset 2):**

| row | kind | flag | motion | hop | speed | ability |
|---|---|---|---|---|---|---|
| 11 | 0 | 0x2 (ground, jump adjust) | 181 | 90 (90..90) | 16..24 | 0 |
| 12 | 0 | 0 (ground, not a finisher) | 155 | none | | 0 |
| 38 | 0 | 0x1 (air, not a finisher) | 183 | 14 (14..64) | 0 | 0 |

No row 0x25 (37) of its own. So in KH2's Valor, Square free on the ground (standing or moving) is
181 with a hop of 90; Square in a ground combo window with a swing left is 155 as a step (its
finisher motion, not counted a finisher); in an air combo window 183 the same.

**What the old controller path had wrong** (`Character::square_finishers`, `try_action_button`,
Checked (code) against the above):
- rows 12 / 38 were taken as finishers (`form_finisher`): allowed at the combo's end and in the
  finisher window, both of which KH2 refuses for them;
- base Sora's Square step rule (stage 177, `sit.count < max` on the count after the press) is one
  swing stricter than the exe's (count before the press under the length). Fixed for base Sora in
  its own version (§7.1.1, the tester's "3. (a)");
- Sora's Finishing Leap (row 37) was ported into Valor (`pose::port_moves`, s214, asked when the
  real cause was row 11, found at s216): not in KH2's Valor. Left out on KH1's states;
- row 11 was right: an attack from Square standing or moving (s216).

**On KH1's states now** (`combomods::valor_square`, from `shared::control` while
`form::native_now`; KH1 still sees no Square in Valor):
- In one of Valor's slots with KH2's combo or finisher window at KH1's time in it
  (`kh2slots::now_playing`, the clip of Valor's moveset): `Character::would_take_square_exe`
  (the exe's order and checks above, row 37 skipped) from KH1's count (`+0x41` of the combat
  info), KH1's connected flag (`+0x50`) or Combo Master, and the finishers so far
  (`combomods::FINISHERS`). The row found plays on our KH1 row of its motion (`ours_for`: the
  combo rows "155" and "183", which play Valor's slots in Valor) through KH1's own Attack step
  (`kh1rows::command_attack`), as base Sora's Square moves; 183 with KH2's hop for row 38
  (`Run::AirHop`).
- Free on the ground (KH1's state 0 and its can-act test, **Assumption** standing for KH2's free
  state): row 11 (`takeover::kh2_would_take_row`, a new combo), on our combo row "181" (Valor's
  181) entered from the ground with KH2's hop for the row (`combomods::ours_combo_run`:
  `Run::Hop`, KH1's air state, count 1), as KH2's plain ground hops (rows 0 / 1).
- After 155 (no combo window, a finisher window 65..96): Attack there only with Finishing Plus
  (KH2's `0x1403D6CA0` finisher path, finishers so far under one plus Finishing Plus), which is
  what our row (a finisher row of ours) already gives; without it nothing until it ends.
- Not checked in game. Not checked: whether Combo Master sets KH2's connected bit (taken as
  connected, as elsewhere); Valor's combo length (`+0x200` / `+0x201`, set through script-call
  setters `0x1401E72E0` / `0x1401E7320`; our controller's `combo_rules` is used, not re-read for
  Valor).

### 7.1.1 Base Sora's Square moves by the same rule

The tester (what-is-still-missing, after m13): "3. (a)" (fix base Sora to KH2's exact rule, its
own version).

**KH2 (Steam KH2 decomp, Checked (decomp)), the parts §7.1 left out:**
- `0x1403A9950` first refuses a non-finisher row through `0x1403D7780(p, 0)`: the finisher window
  `+0x194` bit 5, or the count `+0xD74` (before the press) at the length; `0x140402750`'s own
  `0x1403D7850` refuses only a subset of that (the same, and more than one finisher run).
- `0x1403D7780` / `0x1403D7850` both start with `[+0x5C0] + 0x1DC` bit 11 and `0x1403C1D10`
  (the MP-charge timer `+0x1C0` over 0): Berserk Charge while MP charges. Then `0x1403D7780` is
  false (a finisher row refused, a step let through whatever the count) and `0x1403D7850` false
  (a step passes `0x140402750`). So with MP charging and Berserk Charge no Square finisher, and
  Upper Slash / Horizontal Slash at any count. (That bit 11 is Berserk Charge's: **Assumption**,
  the same reading as the controller's `condition`.)
- `0x1403D6CA0 (p, connected)`, the step: the finisher window, connected and finishers `+0xD75`
  under `[+0x5C0] + 0x202`: count and finishers + 1; else the combo window `+0x18C` bit 10 and
  connected: count + 1; else, if a free range is open (`+0x18C & 0x1C`, range ids 0 / 1 / 2),
  counts back to 1 / 1 (a new combo) and taken; else refused. It clears connected either way.
- Connected (`+0x120` bit 25) is set by a hit (`0x1403D12C0`) or a clash (`0x1403D61D0`), and by
  two object-class callbacks (`0x1403D54B0`, `0x1403D63E0`, which also take one off the count;
  not identified), and cleared by a swing's start (`0x1403D78D0`). No air-whiff setter found:
  a Square row after an air swing that missed is refused. Combo Master is `[+0x5C0] + 0x1E4`
  bit 13: `[+0x5C0] + 0x1D0` is a bit set indexed by the ability number (`0x140402750` tests a
  row's ability as bit `n & 31` of word `n >> 5`), and Combo Master (item 539) is number 173,
  Berserk Charge (item 395) number 107, i.e. `+0x1DC` bit 11 (**Checked (decomp + files)**,
  `03system.bin` item table; both were **Assumptions** before).

### 7.1.2 An Attack after a swing that missed (the tester, what-is-still-missing m14 answer 7 (a))

- **KH2's rule, Checked (decomp)**, Steam KH2. The input step `0x1403D6FC0` runs the Attack
  command (its command row's kind 1) through `0x1403D6CA0 (p, connected)` with connected = the
  owner's `+0x120` bit 0x2000000 or Combo Master (above). The magic command (kind 2) always asks
  with connected = 1. `0x1403D6CA0` (60 Hz range bits from the motion update): the finisher window
  (range id 50) with connected and a finisher left: the next finisher; else the combo window
  (range id 11) with connected: the next swing; else a free state (range ids 0 / 1 / 2): a new
  combo, counts 1 / 1; else refused (nothing, no buffer). There is no airborne case: the air is
  the ground's rule. None of KH2's swings or finishers has a free range (**Checked (files)**,
  `P_EX100`, `P_EX100_BTLF`, `P_EX100_MAGF` msets: the only move with both a combo window and a
  free range is the recoil 41, 16.. and 40..), so after a miss the press is refused until the
  swing has ended, and the next press is a new combo. A whiffed air combo never reaches its
  finisher (without Combo Master).
- **KH1's Attack step already is that rule, Checked (decomp)**, Steam KH1 `0x1402A7510`: in the
  combo window (`+0x1CC` bit 22) it counts on only when the combat info's `+0x50` is set (a hit
  of his attack object, whose `+0x60` points there) or Combo Master (KH1's 65, `0x41`, in Sora's
  list, equipped) is, else it returns with no swing; outside it the count is 1. KH1's menu takes
  the command only free or in the combo window (`0x1402A2EC0`). The row makers
  (`0x1402B3150`, `0x1402B3270`) clear `+0x50` as each swing's object is made, as KH2's step
  clears its bit. Our slots put KH1's combo window on KH2's range 11 (and range 50 while Finishing
  Plus leaves a finisher, `kh2slots::finisher_window`), and no "may be left" on swings (KH2 has
  no free range there), so base Sora, Valor and Wisdom on KH1's states follow KH2's rule through
  KH1's own step, in the air too; KH2's pick in KH1's picker (`combomods::kh2_press`) only runs
  after that step has let the press through. Nothing of ours sets `+0x50` on a miss (writers:
  his attack objects' hits, KH2's body hits that landed, Wisdom's shot hits, the Counterguard
  window's new combo, the old KH2-controller hand-back with KH2's own hit).
- **Changed:** the KH2 controller's own walk (`Character::plan_press`, still used by the
  forms not yet on KH1's states: Master, Final, Limit, Anti) took a whiffed air swing as
  connected (an old reading of the game, never traced) and let a whiffed ground swing restart in
  its window. Both now follow `0x1403D6CA0`: in a swing's window without connected or Combo
  Master the press does nothing. `Abilities::one_swing_on_miss` (the KH1 port's own version of
  this rule) is gone. Tests `whiffed_swings_do_not_advance_the_combo`,
  `whiffed_air_swings_do_not_chain`.
- **Not changed (found, asked):** a clash and two object-class callbacks set connected and take
  one off the count (`0x1403D61D0`, `0x1403D54B0`, `0x1403D63E0`), so in KH2 the next press
  repeats the step; the recoil 41 has a combo window from 16 and a free range from 40. KH1's own
  wall / clash handling doesn't do this. The controller's spell step (`swing_counts`) still takes
  an air spell as counting and a ground one only after a hit; KH2's magic command always counts.

**Sora's rows (Checked (files), `00battle.bin` PTYA moveset 1):**

| row | kind | offset | flag | motion | hop | ability |
|---|---|---|---|---|---|---|
| 11 | 0 | 0 | 0 | 173 (guard) | | 1 (Guard) |
| 12 | 0 | 0 | 0 (ground step) | 161 | | 18 (Upper Slash) |
| 37 | 0 | 1 | 0x4 (ground finisher) | 167 | | 95 (Finishing Leap) |
| 38 | 0 | 0 | 0x1 (air step) | 193 | 14 (14..114), speed 0..18 | 99 (Horizontal Slash) |

So with a combo of 3 (no Combo Plus): Upper Slash in the first or the second swing's combo window
(count 1 or 2 before the press); Finishing Leap (offset 1) from the second swing on (count + 1 at
the length) and in a finisher's finisher window (with Finishing Plus left), and, tried first, it
takes the second swing's press over Upper Slash when both are equipped; Horizontal Slash after the
first and second air swings. Stage 177's rule (the count after the press) refused Upper Slash /
Horizontal Slash at the second swing, and its finisher test (`condition` on the count after the
press, count + 2 at the length) let Finishing Leap in at the first swing of a combo of 3: each one
swing off.

**Now** one path, `Character::would_take_square_exe` (`square_exe_plan`), for:
- base Sora on KH1's own states (`combomods::control`): KH2's combo / finisher window and free
  state at KH1's time in our KH2 slot playing (`kh2slots::now_playing`, the combo style KH2);
  with KH1's own swing playing (the combo style KH1) KH1's combo window (slots 98..104) stands for
  KH2's combo window, no finisher window (**Assumption**: KH1's swings have no KH2 windows). The
  count is KH1's (`+0x41` of the combat info, read as KH2's count before the press, as Valor's),
  the finishers so far `combomods::FINISHERS`, connected KH1's `+0x50`;
- Valor (`combomods::valor_square`), now with the free-state step too;
- the controller's own action button (base Sora when KH2's controller has him; forms with
  `square_finishers` keep their path).
`Character::would_take_launch(.., square: true)` (stage 177's rule) is no longer used for base
Sora's Square moves. Not checked in game.

### 7.2 The left Keyblade's hits as a KH1 attack object (t680)

**KH2 (Steam KH2 decomp, Checked (decomp)):** each of Sora's weapons is its own object with its own
motion (Valor: `W_EX010_BTLF_R` / `_L`). A weapon motion's attack range (id 10) runs the range step
`0x14040D810` on the object whose motion it is, which asserts the ATKP through `0x1403DAFE0` → the
`+0x98` call (`0x1403D29B0`) → `0x1403D25D0` (the 8 attack slots at `+0xC10`). A slot is kept only
for the same **group and the same shape source object** (`attack+0xC8 == group` and the attack's
`+0x14` handle resolving to that object); any other makes a new attack (`0x14040F1C0`) with a new id.
So the right and left Keyblades always run **separate attacks**: each blade may hit the same foe in
the same swing, each with its own re-hit interval, and a left window never ends or renews the right
blade's attack.

**KH2's data (Checked (files): `P_EX100_BTLF.mset`, `W_EX010_BTLF_R/L.mset`, `W_EX010.mdlx`,
`00battle.bin`):** Valor's left windows (weapon 2) are their own ATKPs, e.g. 151: right 178 at
32..40, left 177 at 9..20; 152: left 172 2..11, 176 38..49; 153: left 180 28..66; 154: left 190
41..53, 192 64..85, 527 85..97 (right 189 / 191 / 526 at overlapping frames); 155: left 198 44..65;
181: left 227 21..30; 182: left 202 19..32; 183: left 199 21..29; 184: left 204 29..41, 238 54..63,
529 77..89. Both weapons are `W_EX010.mdlx`, whose attack shapes are spheres on weapon bone 2
(at the grip, the blade along its -x, tip point 78), by group: 1 four of r 40 at 0 / 34 / 40 / 68;
2 three of r 60 at 0 / 34 / 68; 3 three of r 140 at 100 / 240 / 380; 4 r 140 at -50 and r 100 at
90; 5 r 120 at 80 and r 80 at 190; 6 r 120 at 80; 7 r 130 at 42. None of Valor's moves has a hit
window on his body.

**What the old system did wrong (Checked (code) against the above):**
- `kh2slots::kh1_tracks` put the left windows on KH1's group 0 with the right ones, so they hit with
  KH1's one sphere on the **right** blade.
- `kh1rows::window_atkp` took the first weapon > 0 window open: where both blades' windows overlap
  (154 at 41..47 and 64..97, 184 at 77..89) the right object took the right ATKP and the left hit
  never happened; where only a left one was open, the right blade hit with the left's ATKP.
- `kh2slots::keyblade_rearm` re-armed the right object whenever any weapon > 0 window changed, so a
  left window opening let the right blade hit a foe again within the same KH2 attack.

**Now (`lefthits.rs`, `kh2slots::left_channel`):**
- `kh1_tracks`: group 0 (the right Keyblade's object) only for weapon 1 windows; the weapon 2
  windows play on a KH1 attack group of their own (`left_channel`: the first after the body
  windows', 1 for every Valor move).
- On each new play of a Valor slot with left windows (`bodyhits::frame` → `lefthits::new_play`), one KH1 attack object of
  Sora's is made with KH1's attack constructor `0x1402B2DC0` (bone `0x30`, so bone-tied and
  motion-tied: KH1's pass `0x1402B47A0` keeps it live only while his request is the one it took and
  his motion events hold its group's bit), on our copy of his KH1 swing row's data per window
  (finisher row's for a KH2 blow; KH2's reaction and the Recoil row's clash rules, as the body
  windows' data). The left Keyblade's own weapon row (from `leftblade`'s owner copy) goes on it as
  KH1's row maker `0x1402B3270` puts the right one's (radius / length factor / priority / critical
  bonuses), its effect data is the spark's source, his "connected" word is linked.
- Each frame KH1 has him (`lefthits::step`, beside `keyblade_rearm`): the left window open at
  KH1's time; a new KH2 attack (ATKP or group differs, or a window after a frame with none)
  re-arms the object with KH1's re-arm (`rehit::new_attack`) and starts KH2's sweep afresh; the
  window's data goes on it. Re-hits within an attack by the window's interval: `rehit` (the one
  path), through `bodyhits::atkp`.
- (Until the "fit KH1" change below: KH2's weapon-model group spheres laid along the left blade by
  our own entries in t638's hook, with our own sweep. Gone.)

**Both Keyblades' shapes are KH1's own (after m13).** The tester (what-is-still-missing, after
m13): "1. it should fit kingdom hearts 1 in this situation for both keyblades", "2. the keyblade
should behave exactly like kh1's keyblade so make sure it scales by keyblade", "for 2nd keyblade it
should behave 1:1 with how the main keyblade works". So KH2's windows, ATKPs and re-hit timing stay
(which window is open, its damage / reaction / re-hit), and the **shape** each blade hits with is
KH1's Keyblade's, built by KH1 itself. The right Keyblade already is (Sora's KH1 row object, made by
KH1's row maker; nothing of ours touches its shape). The left one now gets the same.

How KH1 builds its Keyblade object (Steam KH1, Checked (decomp) unless marked):
- Data (Sora's swing rows in `btltbl.bin`, 0x70 bytes): bone `+0x10` (`0x30`, the blade), blade
  range `+0x54` / `+0x58` and radius `+0x5C`, all floats (`+0x5C` read with `mulss` / `movss` at
  `0x1402B2FD5..0x1402B2FF1`, Checked (exe); the t680 body-window data wrote it as an int, harmless
  there because our hook gave those objects their shapes).
- `0x1402B2DC0 (actor, hit bit, data)`, the constructor: with a bone (data `+0x10` >= 0) the object
  is position kind 1 (`+0` bits `| 9`), bone `+0x20`, range `+0x24` / `+0x28` from the data, radius
  `+0xC` = data `+0x5C` (times the actor's scale `+0x40` when its `+0x374` has `0x2000`), and both
  the owner `+0x34` and the **bone-source actor `+0x90`** are the maker's handle.
- `0x1402B3270 (actor, ...)`, the row maker (KH1's Attack rows), adds the weapon for bone `0x30`
  from the actor's weapon object (`[+0x14C]`, its row `+0x20` in `btltbl.bin`'s weapon table):
  radius += scale × row `+0x20`; range × row `+0x2C`; priority `+0x68` + row `+0x33` (kept 1..99);
  critical chance `+0x94` × row `+0x31` × 0.05; critical bonus `+0x98` + row `+0x32`; the effect
  data `+8` from the weapon object's `+0x18`; his "connected" word `+0x60`.
- `0x1402B47A0` (the pass's attack walk) keeps an object only while its `+0x90` actor is alive
  (`0x140291FF0`: `+0x374` bit 0 set, bit 1 clear; else the object is freed), builds it only with
  that actor's `+0` bit `0x8000`, and (for most types) not while that actor's `+0x88` bit 0 is set.
- `0x1402B3800` (the entry build) takes both ends with the bone point `0x14029EB20 ([+0x90], bone,
  t)`: for `0x30` it maps t through **the `+0x90` actor's weapon row**, `row+0x28 + (row+0x24 −
  row+0x28) × t`, and takes the point at that fraction along the bone `0x30` names, from its
  parent's drawn matrix (`0x14029FAB0` → `0x1401D62E0`, `0x1401D6320` on the actor's instance
  `+0x15C`: matrices `+0x1B4`, local translations `+0x1B0`). Then `0x1402B3D50` lays spheres of
  radius `+0xC` from the first end (`+0x10`) to the second (`+0x40`), (r + r) / 2 apart (doubled
  with `+0` bit 27), each tested at once with `0x1402B3BE0`; with `+0` bit 16 (set on the first
  match) and a non-fixed type its own sweep back to the last place (`+0x70`); then the wall test
  `0x1402B3A00` unless the object's `+0` bit 20 is set (the constructor sets it from the data's
  `+0x38` bit 16, "spent").

**Now (`lefthits.rs`):** the left object is that object exactly.
- Its data per window: a copy of the same KH1 swing row (finisher row for a KH2 blow, ground / air)
  with its blade bone, blade range and radius **kept as they are** (before: range 0, KH2's radius);
  KH2's reaction, the left window's KH1 attack group and the Recoil row's clash rules on it as
  before; the data's own "spent" bit kept, so KH1's wall test runs for it as for the right blade.
- Made with `0x1402B2DC0`, then the row maker's weapon part with **the left Keyblade's own weapon
  row** (`leftblade`'s owner copy: its weapon object holds the left Keyblade's row and effect data):
  so the radius, blade length factor, priority and critical bonuses are the left Keyblade's.
- Its **bone-source actor `+0x90` is `leftblade`'s owner copy** (a copy of Sora's actor and model
  instance whose drawn matrices are his moved onto the left grip, and whose weapon object holds the
  left Keyblade's row): KH1's own builder then lays KH1's Keyblade sphere row along the **left**
  blade, its ends mapped through the **left Keyblade's own blade range**, so it scales by that
  Keyblade exactly as the right one does by Sora's. The copy's handle comes from KH1's handle maker
  (`0x14038B080`); `bodyhits`' entry hook only brings `+0x90` up to date before KH1's builder runs
  for it (`lefthits::before_build`, which refills the copy) and builds nothing itself.
- On a window switch the new data's blade fields go on the object as the constructor and row maker
  would give them (`blade_fields`: radius, range, priority).
- Contact, record, damage, reaction, spark, sound, clash and wall test are KH1's; KH2's ATKP for the
  hit comes through `bodyhits::atkp` → `lefthits::atkp`; re-hits through `rehit`.
- **Assumptions:** the owner copy (alive bits, `+0x8000`, `+0x88` copied from Sora each fill) passes
  the pass's checks as Sora does; with the left Keyblade not drawn (`valor_left_blade = off`, model
  not loaded) the object's `+0x90` stays Sora's: its hits come off the right Keyblade (logged). KH1
  time in our slot = KH2 frame (as everywhere). Not checked in game.
- Per frame: one fill of the owner copy per build (as t680), no shapes or sweep of ours.
- **Still open:** Valor's guard windows on the left Keyblade (42 / 155, ATKP 162) are unchanged.

### 7.3 The second Keyblade as KH1's own weapon (research and plan)

The tester (after m13): "for 2nd keyblade it should behave 1:1 with how the main keyblade works, make
sure its a file as well". The hits are KH1's own now (§7.2), and the code is its own file
(`lefthits.rs`; the draw and trail are `leftblade.rs`). Where the rest stands:

**How KH1 holds a weapon (Steam KH1, Checked (decomp)):**
- `.wpn` files: three load slots of 0x30 bytes at `0x14296B540` (one per party member: `+0` flags,
  `+4` member id, `+8` users, `+0x18` the model, `+0x20` the effect data). Released / swapped by
  `0x140286720` / `0x1402868E0`.
- The weapon object: `0x140286D60 (actor)` (from `0x140290D60`) takes a free one of **nine** 0x60-byte
  objects at `0x14296B1E0` (free bits `0x14296B5D8`), for the member's slot: `+0` the model instance
  handle (set up with `0x1401D6760`), `+0x18` the effect data, `+0x20` the weapon's row
  (`btltbl.bin`'s weapon table + (weapon − 1) × 0x58); the actor's `+0x14C` holds its handle and
  `+0x374` gets bit `0x10`.
- Who reads it: the bone point (`0x14029EB20`, the blade range), the row maker (`0x1402B3270`, the
  bonuses and spark effect), the trail (`0x1402C2310`, Steam; request id at combat state `+0x38`),
  and the model queue (`0x1401D4070` Steam), which draws **one** weapon per actor with the body,
  skinned to bones 280..292.

**What still runs along (ours beside KH1's):**
- The left `.wpn` is loaded by KH1's own loader into our buffer (`leftblade`), not into a KH1 slot.
- Its weapon object is our 0x60-byte copy of Sora's with the left row and effect data, not one from
  KH1's pool; its bone source is our copy of Sora's actor and instance (moved onto the left grip).
- Its draw is a second call of KH1's model queue with the body hidden (`leftblade::second`).
- Its trail: KH1's trail effect on our owner copy, started / ended by us on KH2's left trail windows.

**Done (the tester after m14: "all three, make it native to kh1 like the base keyblade"):** steps 1-3
below, `kh1-weapon-second.md` §10: the left weapon object is one of KH1's pool, it is drawn by KH1's
actor draw `0x1402A1630` on its holder, and its trail is KH1's request and step on that holder (its
window a baked motion event 7.1 read by KH1's reader). Step 4 (the holder, the bone source) is still
ours.

**Plan (one system per version, each KH1-run):**
1. The left weapon object from KH1's own pool (`0x14296B1E0`, a free bit of `0x14296B5D8`, made as
   `0x140286D60` makes one, on our `.wpn` buffer), released with KH1's own release order; our copy
   goes. (Small; no new hook.)
2. The left trail through KH1's own trail step (`0x1402C2310` path on the owner copy, KH1 deciding
   start / end from the copy's `+0x1CC` trail bit, which the left windows' KH1 motion events would
   set, as the right's do): our start / end calls go.
3. The draw: teach KH1's model queue a second weapon per actor (a wrapper on `0x1401D4070`'s weapon
   part that queues the second weapon object's model on the left weapon bones) instead of a second
   queue of the whole body. Needs the left bones in KH1's own pose (a second weapon bone set baked
   into Valor's slots, or the owner copy's matrices).
4. The bone source: once 3 exists, the owner copy can become a real second bone set on Sora's own
   instance; until then it is the one piece of ours KH1 reads every frame.

- KH1's pass builds its entries through t638's hook (`0x1402B3800`): the open window's group of
  KH2's weapon shapes, laid along the left Keyblade as drawn, from KH1's grip bone (280) toward the
  blade's tip (KH1's blade point `0x30` at t 1, the left Keyblade's own blade range), both read
  with KH1's bone point `0x14029EB20` on `leftblade`'s owner copy (Sora's matrices moved onto the
  left Keyblade); KH2's sweep between frames (`combat_core::AttackVolume`); each entry tested with
  KH1's `0x1402B3BE0` (bodies, and attack against attack: KH1's clash / guards as for the right
  blade). Contact, record, damage, reaction, spark and sound are KH1's.
- **Assumptions:** KH2's distances along bone 2 are laid along KH1's grip-to-tip line in KH1 units
  unscaled (as the body shapes), whatever the Keyblade's length; with the left Keyblade not drawn
  (`valor_left_blade = off`, model not loaded) its windows are placed on the right Keyblade.
  KH1 time in our slot = KH2 frame (as everywhere). The left object has no KH1 wall test (as the
  body windows; the right blade's walls are `walls`'). Not checked in game.
- **Still open:** the right Keyblade still hits with KH1's row sphere (its data's radius, KH1's blade
  range), not KH2's group shapes above (groups 3..7 are far wider: Valor's 155 finisher reaches
  380). Valor's guard windows on the left Keyblade (42 / 155, ATKP 162) are unchanged.

### 7.4 Valor's movement and attack physics on KH1's states (vphys)

The tester (what-is-still-missing, after m13): "no proper physics on grounded finisher, attack
doesn't interrupt jumps momentum so you can fly high in the air while attacking ... compare to how
kh2 does it in the c decomp". Checked against the Steam KH2 decomp, KH2's files and our KH1-side
code; not checked in game.

**How KH2 starts an attack row** (`0x1403D78D0 (player, row, target)`, from the Attack step
`0x1403D6FC0` and the Square paths `0x1403A9950` / `0x1403A9C70`; **Checked (decomp)**):
- A row with a hop (`jump` +0xC above 0): if not airborne (`0x1403BA9A0`) the counts restart
  (`+0xD74` = 0x101). Height: with a target, his height below it (`0x1403D7520`) held between
  `jump_min` (+0x14) and `jump_max` (+0x10, plus his jump height `0x1403B5A40` with row flag 2);
  without, `jump`. Sideways speed: with a target, (gap + `distance_adjust` +0x3C) over KH2's rise
  time for that height (`0x1403B73F0`: `sqrt(2h / 0.40816)`) held between `speed_min` (+0x18) and
  `speed_max` (+0x1C); without, `speed_max`, along his facing. Then the current action's slot
  +0x18 (`0x140409AD0`: `0x1403CBB00` then ACTION_JUMP): the arc **replaced** (time 0, height h,
  base his height, rise time from h), the velocity **set** to the direction times the speed, the
  landing motion 5. So an air swing ends the jump's (or High Jump's) rise and drift, every time.
- A row without a hop: no velocity set, except a lunge toward a target (motion entry `+0xC` bit 1 or
  `+0x18C` bit 6): `+0x20` = direction × the gap × `0x142AE5760[+0xA8]` held between the row's
  speeds. Valor's ground rows have speed 0..0, so none (**Checked (files)**, below).
- The motion then plays (`0x1403C8310`, the row's blend `+0x38`).
- While it plays: KH2's move adds the clip's root travel every frame in every action
  (`0x1403B7CA0`, `docs/kh1-magic-native.md`); in ACTION_JUMP (the air rows) the arc goes on,
  except where range id 3 / 24 holds him (`0x1403CB360`), with the drift `0x1403B7AD0 (dt, 0.9,
  0.95)`; on the ground ACTION_IDLE's step (`0x140422350`) sets the velocity from the stick target
  (empty during a swing: **Assumption**, as in `docs/kh1-magic-native.md`), so only the root moves
  him. After a hold, the fall restarts from his actual speed (`0x1403B4780`).
- The Attack press (`0x1403D6CA0`): in the combo window with the swing connected the count goes on;
  outside it, only in a free state (`+0x18C & 0x1C`: range ids 0 / 1 / 2) a new combo (counts 1 / 1),
  else refused. The fall (motion 4) has range id 1 all through (`P_EX100.mset`, **Checked (files)**),
  so after an air finisher ends a new air combo may start in the fall, as in KH2.

**Valor's data** (`00battle.bin` PTYA moveset 2, `P_EX100_BTLF.mset`, `03system.bin` prty record 1;
**Checked (files)**):

| row | kind | flag | motion | jump / max / min | speed min..max | dist adj |
|---|---|---|---|---|---|---|
| 0 | 1 | 0xA (jump adjust; ground, at a target above) | 182 | 35 / 85 / 35 | -12..12 | 85 |
| 1 | 0 | 0 (ground hop) | 181 | 40 / 90 / 40 | -12..12 | 85 |
| 2 | 2 | 0x4 (finisher) | 154 | none | 0..0 | 0 |
| 3 | 3 | 0x4 (finisher) | 155 | none | 0..0 | 0 |
| 4, 5, 6 | 2, 3, 1 | 0 | 152, 151, 153 | none | 0..0 | 0 |
| 7, 8 | 2, 3 | 0x5 (air finisher) | 184, 183 | 14 / 64 / 14 | 0..0 | 85 |
| 10, 9 | 1, 3 | 0x9 / 0x1 (air) | 182, 181 | 35 / 85 / 35, 40 / 90 / 40 | -12..12 | 85 |
| 11 | 0 | 0x2 | 181 | 90 / 90 / 90 | 16..24 | 85 |
| 12, 38 | 0 | 0 / 0x1 | 155, 183 | none / 14..64 | 0..0 | 85 |
| 40 | 0 | 0x1 | 182 | 100 / 200 / 100 | 0..18 | 55 |

Every air row hops, so in KH2 each of Valor's air swings replaces the jump's arc with a hop of 35
(40, 14) with no target. Ranges (60 Hz frames) and root travel (forward):
- 151: turn 0..8, 22..30; combo from 38; root 187 by its end. 152: combo from 46; 148. 153:
  turn 0..8, 8..19, 19..66; combo from 36; 513.
- 154 (finisher): turn 0..8, 53..61; **edge stop 0..63**; **no gravity 63..end**; Finishing
  Plus 73..85, 97..120; root 0 to frame ~54, then 220 by 120.
- 155: edge stop 0..64; Finishing Plus 65..96; root 180 (frames ~48..96).
- 181 / 182 (air swings): turn; combo from 26 / 24; no root travel.
- 183 / 184 (air finishers): **no gravity 16..end**; Finishing Plus 46..90 / 63..75, 89..112;
  root 437 / 137 forward inside the hold.
- prty record 1 (Valor): walk 2, run 12, jump 235, no ground inertia (acceleration 0, brake 0).

**What KH1's states did** (our code, against the above):
- Ground swings and finishers: our rows (`kh1rows`) have no KH1 lunge (`+8` 0) and KH2's root travel
  baked into the slot's H10 (template requests 0xC8..0xCB carry KH1's travel flag), the edge stop
  (`shared::physics::edge_stop`), the turn windows (`kh2slots`): the same as KH2 on level floor.
- Air swings: our air rows run with KH2's hop for the row (`combomods::Run::AirHop` ->
  `shared::physics::kh2_hop`: the arc and velocity replaced, as `0x1403CBB00`), the no-gravity
  holds and their root travel (`kh2slots::no_gravity`, `air_root`), the fall from rest after.
  The same as KH2 **when the press reaches that path**; two paths didn't (§7.4.1, §7.4.2).

#### 7.4.1 The combo styles reached into Valor (fixed)

`combomods` read the Base / Aerial Base Combo rows' setting itself (`styles::kh2`), not the form:
with a combo style at KH1 the Attack step dropped KH2's pick that KH1's picker had made in Valor
(`kh1picker::at_kh2`, which does ignore the styles) and ran the picker's row with no hop. In the air
that is our air row with **no hop at all**: the jump's (High Jump's, 585) rise went on under every
swing, the tester's "fly high in the air while attacking". On the ground, KH1's length rule and its
Finishing Plus path replaced KH2's. Now `combomods::style_kh2` is the style there: KH2's in a Drive
Form on KH1's states (`form::native_now`), the setting otherwise (base Sora unchanged). Every
combo-style read on the Attack path (`kh1_combo`'s length, the Attack step's pick, KH2's row
selection on KH1's rows, the line ahead, the ground hop) goes through it; `kh1_finisher_window`
and the Dodge Roll reads stay out of forms as before.

#### 7.4.2 No KH1 row when KH2 takes nothing (fixed)

KH2 refuses an Attack its press rule or rows don't take (`0x1403D6CA0`, the row walk
`0x140402D20`): nothing happens. On KH1's states, when KH2's pick in KH1's picker came back empty
(`kh2_pick_in_picker` with no press), the Attack step still ran the row KH1's own picker took: in
the air one of KH1's air rows (requests 0xCC..0xCE, which play Valor's 181 / 182 / 183 in Valor)
with **KH1's own** air numbers, `+0x3C0` = direction × 8 / 12 / 4 and a hop of 40 / 40 / 20
(`docs/kh1-physics.md` §4), not Valor's. In a Drive Form there is no KH1 combo of its own (its
ability finishers are held back), so now KH2 taking nothing is no attack there
(`combomods::attack_row`, logged "KH2 takes nothing: no attack"). **Open (base Sora):** at the
combo style KH2 base Sora has the same fallback onto KH1's row; not changed (asked).

**KH1's picker refuses itself (the tester, what-is-still-missing m14 answer 8).** The tester:
"patch the features kh1's combo row and other related logic are missing ... so that valor and
wisdom can behave correctly without overwriting kh1's vanilla logic". Step 1:
- **Checked (decomp)**, Steam KH1's Attack step `0x1402A7510`: in states 0 and 2 the picker's
  return (`0x1402A6EE0` ground, `0x1402A6C70` air) goes straight to the row runner `0x1402A68F0`;
  in flight (state 8) the step runs the air picker's return + 3. Each picker has that one caller.
- KH1's picker now answers "no attack" itself when KH2's walk takes no row (`kh1picker::at_kh2`,
  `combomods::Ask::Refused`, `refuse_in_picker`): it returns `NO_ROW` (-1) without running KH1's
  own picker, and our wrapper on the row runner (`combat::exec`, installed on every build) ends
  there before anything else. One rule for base Sora at the Base / Aerial Base Combo KH2 setting
  and the Drive Forms on KH1's states (the t685 check at the row runner only remains for a press
  KH2's walk couldn't be asked about in a form). Not in flight (the step adds 3), not when the row
  runner has a move of its own for this press (a Square move given to KH1's Attack step,
  Counterguard in the guard repel), not when the row runner's wrapper isn't in.
- As KH2's `0x1403D6CA0` clears the "connected" bit on the press it takes (Checked (decomp)),
  a refused press clears KH1's "connected" word (combat info `+0x50`; KH1 itself only clears it
  when a row's attack object is made, `0x1402B3150` / `0x1402B3270`), so the next press in the
  window is refused as KH2 refuses it (without Combo Master).
- A press KH2's walk couldn't be asked about (no KH2 data, its tables busy that instant:
  `takeover::kh2_walk` gives `None`) keeps KH1's own row for base Sora, as unmodded.
- No second ask of KH2's walk at the row runner: `attack_row` asked it again (the plain pick and
  Finishing Plus's next finisher) when the picker's pick for that row wasn't found; with the
  picker hooked (`kh1picker::hooked`) it no longer does (KH1's own row, as unasked).
- Combo style KH1, vanilla KH1 and flight: unchanged.
- **Still running along (what's left, one working step each):**
  1. KH2's walk still runs on our KH2 controller object (`takeover::DRIVER`'s `Character`): its
     row table and numbers, with KH1's count, facing, lock and targets set on it each press, and
     its own last speed for the hop of a row that keeps his speed (Wisdom's moving air shots).
     Next: the walk as a function of KH1's state alone (no controller object), its targets from
     KH1's actor list.
  2. Finishing Plus's finisher count (`combomods::FINISHERS`, `LAST`) is ours: KH1 has no
     finisher counter. Next: a field of KH1's combat info its Attack step reads.
  3. The Square moves, Counterguard and the KH1-style ability rows ask the same controller
     object (`kh2_would_take_square`, `kh2_would_take_row`, `kh2_ability_row`): moved with 1.

#### 7.4.3 The Drive and the arrival in the air

**KH2 (Steam KH2 decomp, Checked (decomp)):**
- The change `0x14040CE60 (player, form)`: ACTION_IDLE (`0x1403B4DA0 (p, 0)`), motion 52, velocity
  `+0x20..+0x2F` zeroed. Motion 52 holds him (no gravity and invulnerable all through,
  `P_EX100.mset`, Checked (files)): in the air he stays where he is.
- The swap `0x14040D000` sets the new form's `+0xE10` to 1 or 2 by the change's flag bit 0, and the
  new form's first think `0x140405630` plays the arrival 53: with 1, motion 6 (B010, a stance motion
  free all through, 8-frame blend) and then the idle 0 queued; with 2, `0x140421CE0 (p, zero
  vector, 1.0)` (velocity 0, keep 1.0, ACTION_FLY, whose step `0x140421C80` has no vertical step)
  and the fall motion 4 queued (8-frame blend) with a start callback calling the action's slot +0x20
  (`0x140409B00 (p, 0)` -> `0x1403CBC30`: a hop of 0, a fall from rest, ACTION_JUMP). **Assumption**
  (from what each branch does): 2 is the change made in the air, 1 on the ground.
- So in the air KH2 holds him, with no speed, through the Drive **and** the arrival, then he falls
  from rest.

**Ours before:** the Drive held (motion 52's range on KH1's air state, speed zeroed at the start);
the arrival 53 has no no-gravity range of its own (Checked (files)), so KH1's air step let him fall
through its frames.

**Now** (`form::arrival_held`, read by `shared::physics::air_step`): an arrival asked in the air
(`native_arrive`, his speed `+0x2C` and air velocity `+0x3C0` zeroed) holds him while Valor's 53
plays (or its request waits to be applied); at its end the hold's end restarts the fall from rest,
as KH2's `0x1403B4780` / hop of 0. KH1's fall request after it was already queued (`ask_own`).

**The stance after the arrival on the ground (done, the tester's answer 3 (a) after m14: "On the
ground, KH2 plays Valor's short stance motion (6) after the arrival, before the idle. Add it").**
- **KH2 (Steam KH2 `0x140405630`, Checked (decomp)).** With `+0xE10` 1 (the change on the ground)
  the new form's first think plays the arrival 53 (`0x1403C86A0`), queues motion 6 with a blend of
  8.0 and then the idle 0 with none (`0x1403C8310`). The same branch runs for **every form**
  (`+0xDE0` 1..6) and, with `+0xE10` 0, for a form started without the change (the same 53 → 6 →
  0). In the air (`+0xE10` 2) there is no stance: the fall from rest (above).
- **The clips (Checked (files)).** Valor `P_EX100_BTLF` motion 6 is B010, 15 frames at 30 fps (30
  at 60 Hz), Wisdom `P_EX100_MAGF` C010, 20 (40); Master D010 9, Final E010 18, Limit A010 13. Each
  is range id 2 (free) from 0 to its end (Wisdom's and Final's also 42, keep momentum), so in KH2 the
  stick, a jump or an Attack leave it at any frame. Valor's carries a footstep trigger (2) at 12,
  which a baked slot doesn't take (KH1's own foot events stay, §13.4 of `kh1-anim-baking.md`).
  Anti's motion 6 is its idle (F000).
- **Ours.** A row of ours, "Form stance" (`kh1rows::STANCE_MOTION`, form-only like the arrival:
  `Ours::valor`), built for every form on KH1's states whose moveset has a clip of 6 (`kh2slots`
  `form_ours`: Valor's and Wisdom's now). Its slot's "may be left" runs from frame 0 (KH2's free
  range, `Leave::Kh2`). `form::native_arrive` on the ground asks the arrival with KH1's action flag,
  queues the stance with bit 30 (KH1 clears the action flag as the arrival ends) and blend 8.0, then
  KH1's idle with blend 0 (`ask_own_then`), the order and blends of KH2's. In the air nothing
  changes. Logged "...; its arrival, then its stance (motion 6)". One rebake (two new slots).
- **KH1 keeps it playing while he stands (Checked (decomp)).** KH1's ground motion step
  (`0x1402A9740`, state 0) asks for the idle (`0x14029F7C0 (actor, 0, 10.0)`) only with no speed
  **and an empty motion queue** (`+0x178` 0); the queue's next entry is taken off as it is applied
  (`0x14029FC40`), so while the stance plays the idle is still queued and nothing asks over it. With
  speed it asks for the walk / run, which empties the queue (`0x14029F7C0` sets `+0x178` 0): the
  stick leaves the stance at once, as in KH2. The queue keeps the blend as given (`0x14029FA30`
  `+0x184`), and the apply passes it on; 0.0 is a cut.
- **Assumption.** KH1's blend counts 60 Hz frames as KH2's does (KH1's own reactions queue with 8.0).

#### 7.4.4 Checked unchanged

- Valor's jump and High Jump (KH2's jump start `0x140422630` -> `0x1403B59B0` -> `0x1403CBB00`, the
  same hop function as the rows), gravity 20/49 (`0x1403CBB00`'s 0.40816), fall cap 16, air
  steering 0.9 / 0.95 per 30 Hz tick (ACTION_JUMP's step, `0x1403B7AD0`), run 12 / walk 2 (prty
  record 1), landing motion 5 (`+0x11C`): what `takeover::refresh_kh2_numbers` and `air_step` give
  KH1's states (§7). **Checked (decomp + files)**.
- The air rows' hop (`character_core::Character::plan_press`, used for KH2's pick on KH1's
  picker): the height and speed bounds in the exe's order, `speed_max` with no target, the rise
  time on KH2's constant gravity. **Checked (decomp)**. One difference found, not Valor's: a row whose
  clip has an allow-movement range keeps his speed instead of the row's (stage 192, the user, for
  Wisdom); KH2's `0x1403CBB00` always sets it. Valor's clips have no such range (Checked (files)).
- **Ledges:** see §7.4.5 (changed after m14).

#### 7.4.5 Ground moves carried off a ledge (m14 answer 4)

The tester: "(a) and can you make it in the debug".

**KH2 (Steam KH2 decomp, Checked (decomp)):** the mover `0x1403B9090` cuts a ground move into the
fall (velocity the frame's step over the frame delta capped at 8, the player's "left the ground"
method, vtable `+0x50`: the fall motion 4, 16-frame blend, the fall action) only with no floor
under him and none of: `0x1403BA9A0` (object entry `+0xC` bit 0, motion state word `+0x18C` bits 5 /
24, `+0x120` bit 23), the entry's bit 1, `+0x18C` bit 6, `+0x18C` bit 23. The motion's bits are its
ranges: no gravity (ids 3 / 24, and 30) and the edge stop (29). While one is open he goes on with
the move where he is (no vertical step: no gravity on the ground action), the root carrying him;
the first frame none is open, the cut.

**Which moves (Checked (files), `P_EX100*.mset.txt`, `W_EX010*.mset.txt`):** base Sora 154 (edge
stop 0..38), 155 (0..39), 161 (0..28), Guard Break 165 (0..40), Flash Step 169 (0..17), the Drive
52 (no gravity all through); Valor 154 (edge stop 0..63, no gravity 63 to its end: the whole move)
and 155 (edge stop 0..64). Final's and Master's no-gravity motions (casts, air rows) follow the same
rule once they run on KH1's states. Range 30 is in none of them. Wisdom has none.

**KH1 (Steam KH1 decomp, Checked (decomp)):** the physics step `0x14029A3C0`, after the move, calls
the state's transition from KH1's state table (`0x142DB3840 + state * 0x40`, `+0x28`, set by
`0x1402B1B80`'s seventh argument) with "in the air" (collision `+0x2D0` bit 0, no object under him
`+0x398`); state 0's is `0x1402CC470` (registered by `0x1402CC410`): in the air (or flag `0x1000`)
without flag `0x100` it returns 2. On a change to 2 the step clears the action flag 4, starts a fall
from rest (`0x1402AF8F0 (actor, 0)`) and puts the frame's step per 60 Hz frame, capped at 8, in
`+0x3C0`: KH2's cut, number for number.

**Now:** state 0's transition is wrapped in the table (`shared::physics::ground_next`, Steam only,
the slot checked to hold `0x1402CC470` and its first bytes `40 53 48 83 EC 20 48 8B D9`, Checked
(exe)). For Sora in his ground action (flag 4, not `0x1000` / `0x100`, not KH1's roll request 0xDC)
on one of our slots whose KH2 clip has no gravity or the edge stop open at KH1's time
(`kh2slots::ground_hold`), it returns 0: the ground step and the motion's root travel (`+0x200`)
go on moving him, and KH1's ground mover keeps his height with no floor (as on its own walk-off
frame) or steps him down onto one. The first frame the ranges are shut (or the move has ended),
KH1's own transition and cut run: the fall from rest and the speed capped at 8. KH1's own moves
are unchanged (none of our slots). No new per-frame hook: one table slot, KH1's step calls it.
The debug menu's Ledge Carry row turns it off (`docs/kh1-debug-menu.md`).

**Assumptions:** the entry / `+0x120` bits are clear for Sora; KH1's ground pre-step (`0x1402CC500`:
pushes, grabs, climbs) does nothing odd with no floor under him for those frames; KH1's ground
mover's step-down onto a near floor stands for KH2's. **Not checked:** in game.
### 7.5 One path for every form on KH1's states

The tester (what-is-still-missing, after m13): "if something we've implemented is the same way for
drive forms make it a part of that code". The Valor-only pieces of §7 are one table now, used by
every form run on KH1's states (Checked (code)):

- `form::Kind::native_capable` names the forms that run on KH1's states; `form::native_ok(kind)`
  needs its slots built (`kh2slots::form_built(kind)`); `form::native_form()` is the form he is in
  on KH1's states now.
- `kh2slots::FORM_SLOTS`: per form its specs (KH1 requests and templates, as base Sora's moves of
  that kind), and its extra rows of ours. `prepare` builds every form's slots with the same code:
  its specs, its idle / walk / run cycles, its plain combo rows and form-only rows on our requests
  (`kh1rows::Ours::valor` now means "a Drive Form's own row": the arrival), and the extra rows.
- `Move::form` is the form a slot belongs to; `clip_of`, `live`, `form_of`, `redirect` and the
  request tables' per-form lists (`Set::form_requests`) read it. The slot cache's key carries the
  form's model name (`form::Kind::model`), so Valor's keys are as before (no rebake for Valor).
- `pose::Kh2::form_src(kind)` and `pose::kh2_moves` give the form's moveset; the left Keyblade is
  drawn for a two-bladed form's slot (`Kind::two_blades`).
- The change, the swap (`native_data`), the arrival, the revert, MP and look are already one path
  for every form (`form::native_start` / `native_arrive` / `native_revert` / `drain_step`).

## 8. Wisdom Form on KH1's own states

The tester (what-is-still-missing, after m13): "finish valor and wisdom form ... make sure when its
in the new system that its 1:1 with kh2, our configs don't adjust anything on drive forms". Wisdom
runs on the per-form path of §7.3 (`form::Kind::native_capable`), no KH2 controller step.

### 8.1 Its moves, physics and Square

**KH2's data (Checked (files): `P_EX100_MAGF.mset`, `00battle.bin`, `03system.bin`; moveset 3,
form 2 in `kh2_data::sora::FORMS`):**
- `prty` moveset 3: walk 2, run 12, jump 185, gravity 20/49, fall cap 16, air keep 0.9, drag 0.95,
  **acceleration 0.85, brake 0.97** (base Sora's are 0 / 0).
- PTYA: row 5 kind 3 motion 151 (the ground shot), row 3 kind 3 flag 0x4 motion 154 (the ground
  finisher), rows 1 (kind 0), 9 (kind 3, air) and 40 (kind 0, air) motion 181 with a hop of 14 (the
  air shot; no finisher in the air). No row 11 (no guard), no Square rows, no ability rows.
- Its motions: idle 0 / 1 / 2 (no field variants), jump 3, fall 4, landing 5, item 9, reactions
  10..19, Aerial Recovery 40, arrival 53, casts 56..73 (no 66 / 69), 151, 154, 181, Quick Run 203 /
  204. No High Jump 201 and no recoils 41 / 43.
- 151: allow movement 0..46, combo window from 36, shots (frame trigger 1) at 10 / 20 / 30. 154:
  allow movement 10..80, Finishing Plus window 92..124. 181: allow movement all through, no gravity
  16..40.

**On KH1's states (`kh2slots::WISDOM_SPECS`, `FORM_SLOTS`):** 151 on KH1's ground combo requests
0xC8..0xCA, 154 on 0xCB, 181 on 0xCC..0xCE; reactions, item use, fall, landing; jump 3 on both of
KH1's takeoffs (0x4 / 0x9): KH2's jump start plays 201 only with High Jump (Steam KH2
`0x140422630` / `0x1403B59B0`, Checked (decomp) at t627) and Wisdom's growth ability is Quick Run.
Its plain combo rows and the Finishing Plus row (154) on our combo rows, its arrival 53 on ours,
its Quick Run 203 / 204 on our Quick Run rows (`quickrun`). KH2's pick in KH1's picker reads
Wisdom's PTYA (`form::native_data`). The slot cache's manifest notes Wisdom's files now
(`pose::load_form_noting`): one rebake.

**Physics** (`takeover::refresh_kh2_numbers`): the numbers above on KH1's states.
**Ground inertia** (`shared::physics::ground_inertia`, new): KH2's free-state move (Steam KH2
`0x140422350`) with the moveset's inertia flag calls `0x1403B7AD0 (player, out, dt, acceleration,
brake)` (prty `+0x28` / `+0x2C` through the player's `+0x148`): stick pushed, velocity = target ×
(1 − acceleration) + velocity × acceleration (`0x140141AE0`), target the stick's direction at the
walk or run speed (`0x1403A8FA0`, run from 0.8); stick at rest, velocity × brake; move = velocity
× dt. Checked (decomp). Taken per KH2 physics tick (two 60 Hz frames, as KH1's air steering here).
KH1 keeps its walk / run / idle animation choice; the move is KH2's, so he slides to a stop in his
idle. In KH1's actions on our slots (the shots, casts, landing) it steers inside the clip's
allow-movement range (`kh2slots::allow_movement`) and brakes outside it. **Assumption**: KH2's
attack actions use the same velocity function with the stick cleared outside range 41 (the caller
in the attack action wasn't traced); KH1's air speed `+0x3C0` seeds the velocity on landing.

**Square** (`shared::control`): KH2's action-button handler `0x1403A81F0` (Checked (decomp)) finds
no glide, no Square rows, no row 11 and no dodge roll in Wisdom, so Square is Quick Run (growth kind
1): on the ground from a free state, along the stick vector (none: his facing), stick pushed or not.
On KH1's Quick Run state (`quickrun::start`) with Wisdom's level's numbers. KH1 sees no Square in
Wisdom; in the air past the rise no jump (no Aerial Dodge, no glide).

**Quick Run's level**: one level above Sora's own, MAX at most (`form::form_abilities`), the
tester's rule for the forms' growth. **Assumption**: Sora's level is the equipped one KH1's menu
gives (`menu::Growth`); KH2's own rule is the form's level (Lv 1 / 3 / 5 / 7 → LV1..MAX), which KH1
has no form levels for.

**Not done**: Aerial Recovery (40) isn't built for Wisdom (the native Aerial Recovery path isn't
reached in a form, as in Valor). The no-gravity range of 181 is skipped because the clip allows
movement all through (`kh2slots::no_gravity`, a main-branch rule from the user's s194, **not
checked in KH2's decomp**).

### 8.2 Wisdom's shots

**KH2 (Checked (files)):** Wisdom's 151 carries frame trigger 1 with 25 / 26 / 27 at frames 10 /
20 / 30, its 154 with 28..40 between 16 and 88, the arrival 53 with 1..12 and 100..102 at 0. KH2
runs each through the form model's object script (`wisd`, BAR entry 3 of `P_EX100_MAGF.mdlx`): a
homing shot (ATKP 859) with its effect from the form's archive.

**On KH1's states now:** the triggers are KH1 motion events of the baked slots (t677) and KH1's
motion pass fires them (`kh2slots::sound_event`, the wrapped `0x140295420`); in Wisdom on KH1's
states a trigger 1 of one of its slots goes to the form's script (`form::effect` →
`formmagic::effect`), not to Sora's effect package. The script's frame runs on KH1's frames
(`takeover::native_spells`, now also in a native form, with the form's clips and ATKP rows). Each
shot's attack is a free-standing KH1 attack object that KH1's own collision pass collides
(`bodyhits::spell_attacks`, since t-spell-hits): KH1 does contact, damage, reaction and spark, and
a hit sets Sora's KH1 "connected" word (the object's `+0x60`), so KH1's Attack step chains the
combo on a hit without Combo Master, as KH2 does (s305). The effects play in KH1's effect engine
(`kh2fx::spell_start` → `kh2dpx`, package `kh2fx::WISDOM`).

**Running along (not native yet):** the shot's flight (its homing, speed, lifetime and when its
effect ends) is KH2's script run by our interpreter (`kh2-magic`'s `Magic::object_effect`), whose
results are copied into the KH1 attack object's shapes and the effect's place each frame. The plan
to move it into KH1: KH1's own free-standing shots (Wakka's ball, KH1's spell projectiles) are
attack objects moved by KH1's script machine (`0x14028F5E0` list, syscall 23 `0x1402B2DC0`), so
the shot becomes a KH1 spell script of ours (a `.mag`-like KH1 script made at build time from
`wisd`'s numbers: speed 20 → 80, turn 1.8° a frame, life) started from the trigger with KH1's spell
start (`0x14028F140`), KH1 then stepping, colliding and freeing it. Not done this round: it needs
KH1's script format for a homing shot worked out first (an open question in the report).

### 8.3 Wisdom's magic on KH1's cast state

The same path as base Sora's KH2 magic on KH1's own cast state (`docs/kh1-magic-native.md`), with
the form's own clips: KH1's command step casts it (no borrow), KH1's request plays our slot of the
cast motion, KH1's spell start (`0x14028F140`, hooked) starts KH2's `.mag` script instead of KH1's
(`formmagic::kh1_spell_start`), released at the motion's magic-cast event.

- **Which spells are KH2's:** in Wisdom on KH1's states Fire, Blizzard, Thunder and Cure all are
  (`formmagic::wisdom_kh2_spell`), whatever the Config rows (a Drive Form is KH2's all through).
  KH1's Gravity, Stop and Aero (no KH2 spell) are KH1's own on Wisdom's cast motions for them
  (`anim_kh2` is true in a native form), as base Sora's at KH2: ground 63, air 64 / 67 (`CASTS`).
- **The motions:** `WISDOM_SPECS` puts Wisdom's own clips of the MAGC motions on KH1's cast
  requests (Checked (files): 56 / 58, 59 / 61, 62 / 64, 65 / 67, 63; its finisher casts 57 / 60 /
  63 on the ground requests, built per form now). `pick_cast` and `cast_motion` take the moveset
  playing's (`live`). Wisdom's clips carry their own allow-movement ranges (56: from 24; 59: from
  10; ...), steered by the form's ground inertia (§8.1).
- **The sub:** level × 2 − 2, + 1 for a finisher cast, + 10 in Wisdom (`form::magic_sub`). Checked
  (files): the spells' ATKP rows exist at those subs (Fire ATKP 502 at sub 10, Reflect's at 10 /
  14); the rule itself is from the Epic-era research (s192), not re-traced in the Steam decomp.
- **The cost:** KH2's MP cost scaled onto KH1's MP (`mpcost`), as base Sora's KH2 spells (KH2's
  costs are the same in Wisdom).
- **Air casts:** KH2's air-cast physics (`shared::physics::cast_rule` is KH2's in a form).
- **Not changed:** the old controller path's rule "Wisdom's Cure is KH1's Cure on KH2's motion"
  (main branch, the user) doesn't apply on KH1's states: KH2's Wisdom casts KH2's Cure. An open
  question in the report.
