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
  The native cast paths already count a cast after a miss (kh1-magic-native.md §4c.3.1); the
  controller's is left (deprecated, the tester's m15 answer 4).

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
window a baked motion event 7.1 read by KH1's reader). Step 4 (the tester after m15, answer 3
(a): "bake the left hand's weapon bones into Valor's moves"): each Valor move has a holder motion
baked beside it, and KH1's own pose pass poses its weapon bones each frame
(`kh1-weapon-second.md` §10.4). Still ours: the holder actor copy KH1's draw and trail run on, and
the one carry from the holder's left hand onto Sora's.

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
  1. Done in t888 / t889 (`docs/kh1-anim-slots.md` §7.z.11): KH2's walk for an Attack press (and
     the KH1-style ability rows and finisher walks) runs on KH1's state alone
     (`combomods::Kh1Press`, `character_core::HostPress`) over the moveset's records
     (`kh2table`), its targets KH1's own (the press's target, every enemy's lock points). The
     hop of the row taken too since t892 (`HostPress::hop`, KH1's own speed for a moving row).
  2. Done in t890: Finishing Plus's finisher count and the combo's last step live in KH1's
     combat info (`+0x44`, `+0x45` / `+0x46` / `+0x5C`), no state of ours.
  3. Done in t894: the Square moves, Counterguard, Retaliating Slash and Valor's row 11 on KH1's
     state too (`HostPress::square` / `forced`); his abilities read from KH1's lists at the press
     (t893).
  4. Done in t899 .. t901: the form's state (`form::DRIVE`), the ability menu's work
     (`menu::growth_frame`) and the hit state (`takeover::STRIKES`) are their own; our KH2
     controller object (`takeover`'s driver) is deleted.
  5. Done in t902: the spell scripts' world takes its enemies from KH1's own enemy walk.
  6. Left: the Keyblade's and body's guard windows are KH1 guard objects placed each frame on
     KH2's rig posed by us (`Strikes::guard_step`).

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

**Movement during casts (t870, Checked (decomp)).** KH2 adds its push (Blizzard's push of its caster)
to the move each frame (`0x1403B89A0` -> `0x1403D4570`); KH1's push replaces the frame's move
(`0x14029A3C0` -> `0x1402C7070`). So under Wisdom's ground inertia the push goes into our ground move
(`physics::inertia_push`) and his slide and steering go on under it. The velocity step runs on KH2's
clock (`+0xBA8`: the first frame, then every second 60 Hz frame, held between), not eased per frame.
The stick is kept only in a free range or range 41 (`0x1403A89A0`); Wisdom's casts have range 42 all
through (59: 41 from 10; 60: from 34; 56: from 24; Checked (files)). **Assumption**: a ground cast keeps
KH2's free-state action (the cast start sets only the motion).

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

## 9. Form levels in KH1 (what-is-still-missing after m14, answer 6 (b))

The tester: "build form levels into KH1" (KH2 uses the form's own level for its growth ability).

**KH2's rule (Steam KH2, Checked (decomp)).**
- **The add** `0x1403E8380 (player, n)`: the player's form (`+0xDE0`) record in the save
  (`0x1403A1410`: save `+0xE04 + (form − 1) × 0x38`, level byte `+2`, EXP i32 `+4`) gets `n` EXP.
  Then, while the level is under the cap, the fmlv row (form, level) is found by `bsearch` on
  `form << 4 | level` (the table pointer `0x142AE5708`, set from 00battle's `fmlv` by
  `0x1403E8A90`); EXP under the row's EXP is kept; else the level goes up and the row's EXP is
  taken off. At the cap, or with no row, EXP becomes 0.
- **The cap**: 2 + the forms 1..5 Sora owns, counted by each form's item (`0x1403C3EF0` reads the
  form's item id from `0x1405C5D08`, `0x1403C4720` whether it is owned): 7 with all five.
- **A level-up**: when the new row's growth level (fmlv byte 1, low nibble) is above the old
  row's, the form's own growth ability in its record's list (`+8`, 0x18 entries) is swapped for the
  next one, and Sora's own (his record's list `+0x54`, 0x50 entries) goes from the level two under
  to one under (an empty slot when that is none). The table is at `0x1405C9030` (Checked (exe)): 5
  u16 per form, none then LV1..MAX: Valor High Jump 94..97, Wisdom Quick Run 98..101, Limit Dodge
  Roll 564..567, Master Aerial Dodge 102..105, Final Glide 106..109. So Sora gets LV1 / LV2 / LV3
  at form LV3 / 5 / 7, and the form has LV1 / 2 / 3 / MAX at LV1 / 3 / 5 / 7. The row's ability
  (fmlv `+2`; the user's 00battle: Valor LV2 Auto Valor 385, LV4 / LV6 Combo Plus 162; Wisdom 386,
  412, 413; Limit 568, 405, 407; Master 387, 163, 163; Final 388, 398, 398) goes to Sora
  (`0x1403C3F00`). A message window and the player's effect `0x1403ED390` (`0x1402C6B30 (..,
  0x10, 1, 1, 0x2715, player)`) show it.
- **Who adds what** (each checks the player's form `+0xDE0`):
  - Valor (1): `0x1403E86B0`, from the enemy's damage callbacks `0x1403DB730` / `0x1403DBB90`:
    +1 when the hit's attack's owner is the player (`0x1403BA720`, and his `+0x6C8` bit 7).
  - Wisdom (2) / Final (5): `0x1403E8780 (enemy id, killer)`, from the enemy's death callbacks
    `0x1403DB680` / `0x1403DBAF0` with its last attacker `+0xD38`: +1 for ids 1000..2011 except
    0x417..0x423 (`0x1403BA050`), or 3000..4014.
  - Limit (3): `0x1403E8730`, from `0x1403D8B40`: +1 per hit of attacks 0x2AE, 700, 0x2BF, 0x2C2.
  - Master (4): `0x1403E8760 (n, player)`, from the prize pickup `0x1403DCA50`: + the orb's value.
  - Nothing is added while `0x1403A3C40` (the current area's flag 0x400) holds.
- The form gauge's length is `(level + 3) × 600` frames (`0x1403C17C0`, §2). The testing branch
  has no form gauge (forms drain MP), so the level doesn't lengthen a form here (asked).

**KH1 (t692).**
- **Storage** (`win/formexp.rs`): `save + 0x882` Valor, `+0x884` Wisdom, `+0x886` Final, a u16 each
  (`formlevel::Record`: level in the top 3 bits, EXP in the low 13; 0 = never entered, read as LV1).
  - KH1 describes its save's first block with a field list (`0x1402CF620`, `0x1402CF7E0`:
    "... 21s 1i 1d 10c" ending at `+0x888`, `0x142DE9BE8`); `+0x880` is a bool (written by the
    instruction at `0x1402857EC`, read at `0x1402A1D07`), and `+0x881..+0x887` are part of that field list's last "10c".
  - No instruction in the Steam exe accesses `[reg + 0x881..0x887]` other than a stack local
    (`0x14039DB25`), and none addresses `0x142DE9BE1..BE7`. Checked (exe, a full disassembly).
  - The whole `0x16C00` bytes are copied as the save (`0x1402CF7E0`), so the bytes travel with it.
  - Assumption: nothing reaches them through a computed pointer. Two bytes more (`+0x888`'s
    neighbourhood, or `+0x489`) would be needed for Limit and Master.
- **Valor**: KH1's contact `0x1402BF330` (hooked, `probe::hit`) making a record for an attack Sora
  owns on an actor of kind 3, in Valor: +1. Both Keyblades (`lefthits`) are Sora's attacks.
- **Wisdom**: the defeat is KH1's enemy scripts' EXP command, `0x1402BC9A0` (EXP shown at the
  enemy) and `0x1402BCA20` (at a point), `(args)`: `args[0]` the enemy's handle, `args[1]` the
  EXP; both skip their work with EXP Zero on and give the EXP to the party (`0x140284990`).
  Hooked at their entry (prologues `40 53 48 83 EC 20 48 8B D9` and `40 53 48 83 EC 50`, Checked
  (exe)), so EXP Zero doesn't stop it. KH1 keeps no last attacker, so the contact notes the last
  actor whose attack made a record on each enemy; Sora's last: +1 in Wisdom. Assumptions: every
  KH1 enemy gives its EXP through one of the two once when defeated; every KH1 enemy counts as a
  Heartless (KH1's ids aren't KH2's).
- **Final**: KH1 has no Nobodies: nothing (asked).
- **The cap**: Valor and Wisdom are always in the Drive list (owned), Final once entered (its
  record set): cap 4, or 5.
- **Growth**: the form's growth ability is fmlv's growth level for its form level (`form::
  form_growth`): Valor High Jump, Wisdom Quick Run, Final Glide. This replaces the old rules
  (Valor and Final at MAX, Wisdom one above Sora's own). A level gained in the form takes effect at
  once (`form::abilities`).
- **The cap, re-checked (after m15, answer 5: "kh2's rule")**: `0x1403E8380` counts, for forms
  1..5, the form's item id from `0x1405C5D08` (Checked (exe): 0x1A Valor, 0x1B Wisdom, 0x233 Limit,
  0x1F Master, 0x1D Final; index 6 is Anti's 0x1E, not counted) owned through `0x1403C4720`, and
  stops raising the level once it is at 2 + that count; the count is taken again at each step of the
  loop. `formexp::cap` is the same rule: Valor and Wisdom are always in the Drive list here (owned
  from the start), Final once entered (KH2 gives Final's item on its first change), Limit and
  Master don't exist on the testing branch. Checked (decomp + exe). Not modelled (not part of the
  cap): KH2 adds nothing while the area's flag 0x400 holds (`0x1403A3C40`).
- **Base Sora's growth and the rows' abilities (after m15, answer 6: "(c)")**. KH2's level-up
  (`0x1403E8380`, Checked (decomp)) swaps Sora's own entry of the growth table from the level two
  under the form's new growth level to one under it (from an empty slot: a new LV1; nothing if he
  doesn't have the lower one), and gives the row's ability with `0x1403C3F00`. With the user's
  fmlv (Checked (files)) that is High Jump / Quick Run / Glide LV1, LV2, LV3 at Valor / Wisdom /
  Final LV3, 5, 7 (never MAX: the table's MAX entry is only the form's), and the abilities at LV2,
  4, 6: Valor Auto Valor 385, Combo Plus 162 twice; Wisdom Auto Wisdom 386, MP Rage 412, MP Haste
  413; Final Auto Final 388, Form Boost 398 twice (Limit 568 / 405 / 407, Master 387 / 163 twice:
  not on the testing branch).
  - **Growth** (`formlevel::sora_growth`, `formexp::base_growth`, `growth::FormGrowth`): base
    Sora's High Jump / Quick Run / Glide level is the form's level less one, LV1 at least while he
    has the ability; since the vanilla progression (the tester, what-is-still-missing after m16,
    answer 6 (b), and after m17) his own level and bosses give none. KH2's swap only raises an
    entry he has, so a form never lowers it. Without the ability, KH2 puts LV1 in an empty slot;
    here a form's level no longer gives KH1's High Jump or Glide (KH1's list stays vanilla: they
    count once KH1's story gives them), and Quick Run is his anyway (all of ours are).
  - **Abilities** (`formlevel::rewards_due`, `growth::kh1_reward`): KH1's own counterparts, off until
    equipped, each with a "New Ability" box: Combo Plus 162 → KH1's Combo Plus (6), Air Combo Plus
    163 → KH1's 7, MP Rage 412 → KH1's MP Rage (24: KH1's own effect, MP when hurt; Assumption that
    it stands for KH2's). Not given yet (asked): Auto Valor / Wisdom / Final (KH2's low-HP Drive
    reaction command, no system here), MP Haste (KH2's shortens MP Charge, which KH1 lacks; KH1's MP
    Haste is +150% MP per hit), Form Boost (the form gauge; forms drain MP here).
  - **The save**: the form's record keeps the last level whose ability was given (`Record::
    rewarded`, bits 10..12 of the u16; EXP now in bits 0..9, under 1024: KH2's largest step is
    672). A t700 save reads as nothing given, so the levels it already has give their abilities
    once. A level's ability is marked given when it is in Sora's list (or has no counterpart); a
    full list keeps it (and that form's later ones) until there's room.
  - Only with `abilities = kh1`, in KH1's menu step (`menu::Growth::update`, once a second), as
    the mod's other gifts; in `test` / `all` nothing is written and the rewards wait.
- A level-up shows a "Form Level Up" box in KH1's prompt style (`menu::queue_prompt`) and logs.

## 10. Decisions from the tester (what-is-still-missing, after m15)

- **The old KH2 controller is deprecated (answer 4).** The tester: "depreciate the old kh2 controller, we're
  not going to touch final form unless I change my mind". `character-core`'s controller taking Sora (still
  what Master, Final, Limit and Anti run on) gets no new work and no fixes: new behaviour goes only into KH1's
  own states. Final Form stays as it is. Final's left trail (KH2's data) waits with it. Limit and Master move
  to KH1's states when the tester asks; the controller is removed once nothing runs on it.
- **Form level and the MP drain (answer 8).** Left as it is: a higher form level doesn't change the MP drain.
  To do later: once Drive Forms run on KH2's Drive gauge, the form gauge's length follows KH2's form-level
  rule (fmlv), not MP.
- **Final's EXP (answer 7).** Waiting on the tester: Final gains no EXP (KH2 counts Nobodies; KH1 has none).
- **Ledge carry and KH1's grabs (answer 10 (b), note for later).** While Ledge Carry (t696) holds Sora off a
  ledge, KH1's ground checks still run, so a ladder or ledge grab can take him mid-carry. Left as is (b).
  Later: decide whether the carry should block KH1's grab checks until the move's hold ends, as KH2 has no
  grab during these moves (Assumption: not traced in KH2).

## 11. The drive-forms thread's list (Valor and Wisdom on KH1's states)

The tester's traces (`testers/drive-forms/`, Select+Triangle) and the t725 log of the same session
(`testers/kh2-chest/kh1-kh2.log`, Valor from f8258, Wisdom from f17245) against KH2's files
(`P_EX100_BTLF.mset`, `P_EX100_MAGF.mset`, `W_EX010_MAGF.mset`, `W_EX010_BTLF_R.mset`, `00battle.bin`;
Checked (files) with `kh2-data`'s readers) and the Steam decomps.

- **Wisdom's Square (Quick Run).** Trace f17582: Square gave KH1's own roll (motion 0xDC, slot 117).
  `quickrun::built()` refused every Drive Form (a t501 rule from when the forms ran on KH2's
  controller), so `shared::control`'s Wisdom branch couldn't start KH1's Quick Run state and KH1's menu
  step (Steam `0x1402937F0`: Square's pressed bit `0x10` sets the command record's `0x4000000`) led
  to KH1's roll in its control step (`0x1402A7B30`, ability bit `0x20` of `0x142D5EC10`, set from the
  equipped Dodge Roll by `0x1402A6380`). Checked (decomp). Now native Wisdom takes the Quick Run state.
- **Gravity's finisher.** Trace f23582 / log: Gravity in Wisdom played 62 every time. t719's block in
  `combat::command` (Gravity / Stop / Aero's combo step at Magic Combo KH2, Thunder's finisher cast 63
  at the length) was dropped by t721's commit, made from an older `combat.rs`. Restored.
- **Wisdom's Keyblade twirl.** `W_EX010_MAGF` motion 0 turns the Keyblade root 75..246 degrees from base
  Sora's hold over the idle's 142 frames; base Sora's own stays within 8 degrees in his idle and swings;
  Valor's `W_EX010_BTLF_R` 181 holds it ~95 degrees round (Checked (files)). The bake kept KH1's rest
  grip for every motion. `pose::form_grip`: a form's bake turns KH1's grip bone (280) and what hangs
  from it by the form's root turn from base Sora's hold (motion 0, frame 0), in model axes through
  KH2's hand. Assumption: KH1's rest grip stands for base Sora's KH2 hold.
- **Wisdom's Reflect.** Log f26207: "command 0x2 (02000702) is KH2's: KH2 borrows him for it" (the
  deprecated controller, which drew base Sora's 71). `reflect::native_path`: Reflect on KH1's own cast
  state in native Wisdom too, Wisdom's own 71 / 72 / 73 on our Reflect rows, sub +10, KH2's cost.
- **Moving while attacking (Wisdom).** Trace f45615: no move at all through 151 with the stick held. Our
  combo rows stand for KH1's swing slots 98..101, so KH1's ground step took the swing branch and never
  ran the form's ground inertia. It does now (151's range 41 is 0..46, 154's 10..80).
- **Fire's dash.** Wisdom's 56 travels 384 forward, 57 836, 58 383 (base Sora's casts: none), Checked
  (files). The ground casts are baked with their travel and their request gets KH1's travel flag
  (bit 15) while Wisdom's slot is on it. In the air: the root's travel on every frame of a form's air
  move (`kh2slots::air_root_form`; 219 of 58's 383 come before its no-gravity range 16..36), as the OG
  rewrite's action step adds the root on every action frame.
- **Air attacks and casts steer.** Trace f46520: 181 dragged with the stick held. KH1's air step treats
  every action with no steering; a form's action in its range 41 (181 all through, the casts from 24,
  73 at 0..10 and from 50) now steers as in a jump (the OG rewrite's `air_control`).
- **Valor's air finisher drop.** Trace f14640: 184 (Finishing Plus after 183) started with no rise and
  fell ~50 before its no-gravity range at 16. 183 is held to its end; the hold-end fall restart
  (`kh2_fall_at`, KH2's `0x1403B4780`) on the next air step measured the held frame and replaced 184's
  row hop of 14. A fresh hop now isn't restarted. Assumption: KH2's vertical speed at that think is
  the hop's start speed.
- **Valor's ground finisher in the air.** 154: no gravity from 63, root rising 0 → 125 between 64 and
  116 (Checked (files)). `shared::physics::lift_off`: KH1's air state from the frame its root rises in
  the range, the motion kept, then KH1's fall. Assumption: KH1's state 2 runs the swing to its end.

Not done this round:
- **Valor's Square finisher effect** (155: elements 13 / 14 at frame 0, 15 at 42): the log has no
  Effects lines (the category was off), so whether KH1 refused them (a program not in KH1 yet, a layer
  left out) or never got them isn't known. Needs a log with the Effects category on.
- **The left Keyblade moving between animations**: no data in the traces for the left blade; needs a
  video or the left Keyblade log lines around a change.
- **Wisdom's Fire "physics"** on the ground beyond the dash: KH2's range 42 (`KeepMomentum`, the motion
  state's `+0x38` bit 0, read as `+0x190` bit 0 by the free-state move `0x140422350` to pick the inertia
  path, Checked (decomp)) is on every Wisdom move; ours runs the inertia for all of Wisdom's actions,
  which matches.

### 11.1 Second round (the tester's reply after m2; t757 to t761)

- **A form's magic ignores the Magic Combo row (t757).** KH2's magic command counts every cast as a
  step of the combo with no setting (Steam KH2 `0x1403D6CA0`, Checked (decomp)). `formmagic::
  wisdom_combo` no longer reads the row: native Wisdom's Gravity, Stop and Aero always take KH2's
  Thunder / Magnet / Cure roles on the ground and a step of the air combo in the air. The row is base
  Sora's only.
- **Valor's ground finisher lifts off (t758).** KH1's think (Steam `0x1402A3300`, Checked (decomp)):
  `0x1402A43A0` (the motion for a state changed last frame, then flag `0x400` cleared), the control
  step `0x1402A6220`, the state's step (table `0x142DB3858`), then the physics. t740 changed the state
  inside the ground state's step, so the air state's step (ours: the no-gravity hold and the root's
  rise) didn't run before the physics that frame. Assumption: the physics then set him down and KH1
  played its landing (no trace of t740). Now from KH1's control step, as Finishing Leap's row
  (`combat::exec`). 154's root rises about 3.5 a frame from 64 (Checked (files)).
- **A row's hop isn't held by the move it cuts (t759).** Trace f14640 (Checked (trace)): row 7's hop
  (14) for 184 is written at f14526 in the control step while KH1's slot still plays 183 at frame 48,
  inside 183's no-gravity range (16..end); that frame's air step held him and set the hold flag, and
  at f14527 (184's frame 0, no hold) the hold's end restarted his fall from rest (`kh2_fall_at`, KH2's
  `0x1403B4780`): the hop was lost and he fell 52 before 184's own hold at 16. The same happens to any
  hop started out of a held move (base Sora's air finishers out of Finishing Plus, a hop out of a cast
  hover), which is why it wasn't Valor's. KH2 starts the row's motion and hop in the same think
  (`0x1403D78D0` -> `0x1403CBB00`, Checked (decomp)). Now, on the frame a hop starts, the cut move
  holds nothing (`shared::physics::HOP_FRAME`).
- **A KH1 air row's hop isn't held by the move it cuts either (t853).** Trace f26703 (Checked (trace)),
  Aerial Combo KH1: 184 out of 183's Finishing Plus window runs KH1's air swing row, whose start sets
  its hop (14) in KH1's command step (`shared::physics::kh1_air_attack_started`), which t759 didn't
  cover. At f26573 183 (frame 53) still held him and set the hold flag; at f26574 the hold's end
  restarted his fall from rest and he dropped 52 (98.9 to 46.6) before 184's hold at 16. Now that
  start marks `HOP_FRAME` and clears the hold flag, as `kh2_hop_as` does.
- **Correction, and the real cause (t854).** t853's cause was wrong: that hop also goes through
  `kh2_hop_as` (the log at f981 of trace f1084 has both lines), so `HOP_FRAME` was already set. The
  next air step (f982) still found KH1's slot on 183 (KH1 plays 184 from f983), past `HOP_FRAME`'s
  one frame, held him and set the hold flag; at f983 the hold's end restarted his fall from rest
  (134.3 down to 82.0). Now the cut move (`shared::physics::HOP_CUT`: its motion, KH1's time in it)
  holds nothing while it still plays without starting over, at most 4 frames. Checked (trace, log).
- **A revert in a held air move (t855).** Trace f2082 (Checked (trace)): a revert during Valor's
  183 (held from its frame 16) left KH1's arc fields as KH2's arc clock had run them through the
  hold (time 53 at f1975), so KH1's air step dropped him at 16 to 19.6 a frame at once. Now a revert
  in the air restarts his fall from rest where he is if a hold had him at his last air step
  (`shared::physics::form_left_in_air`, KH2's restart `0x1403B4780`). Assumption: KH2's revert ends
  the hold through that restart.
- **Valor's ground slam effect (t760).** 155's element 15 (frame 42, effect 9 on the object root) runs
  program 65 (pppRandDownFV), which `kh2dpx::KH1_SAME` didn't list, so it never played. KH1's table
  entry 65 (`0x14054FF00` + 65 x 0x50, calc `0x14032D1C0`) and KH2's (`0x140779A00` + 65 x 0x50, calc
  `0x1404C2840`): Checked (exe) for the tables, Checked (decomp) for the code, the same but for the
  paused byte's offset and the random float and zero vector each calls (as RandFV, 63). All of
  `P_EX100_BTLF.a.us` checked: only the arrival's elements 5 and 6 still need programs (pppKeThRes16,
  pppKeDrct, pppMatrixLoc, pppKeThTp, pppKeThSft, pppKeTh). The slam's launch is its own hitboxes:
  ATKP 197 / 198 (reaction 11, a blow; knock-back 60 / 5) on both Keyblades' group 3 (spheres of 140
  at 100, 240, 380 along the blade) at 44..65 (Checked (files)).
- **The left Keyblade between moves (t761).** `leftblade::left_draw` drew nothing on a frame with no
  holder pose from the last 2 game frames (KH2's own left pose, the old fallback, isn't fed on KH1's
  states since t702). Now the last good hold (the holder's bones from its left hand) is kept on his
  left hand as drawn. A log line (Animation) says why a frame had no fresh holder pose.
- **The grip, checked, no change.** KH2's files (`W_EX010_BTLF_R` / `_L` on `W_EX010`'s skeleton,
  hands 178 / 86 from `sklt`): Valor's skeleton binds exactly as base Sora's (every arm bone within
  0.1 degrees); measured both by the hand bone and by a hand frame built from the joint positions (the
  retarget's), the right Keyblade sits about 21 degrees tilted along its blade from base Sora's hold
  in Valor's idle, walk, run and swings, 3..18 in 154 / 183 / 184, reversed (166) in 181's first 30
  frames; the left one 15..29, flipping in 153 (92..178) and 183 (141 from 40). Base Sora's own swings
  turn it too (153's 102 at frame 20), which base Sora's bakes don't show (KH1's rest grip). t734's
  `form_grip` draws Valor's right Keyblade with those turns on KH1's grip, and the left one
  (`left_bones`) is KH2's left root on KH1's left hand with the right's KH2-to-KH1 turn: both follow
  KH2. No cause for a bad grip was found in the data or the maths; not seen in game.
- **Wisdom's landing (t762).** The owner's traces f9084 / f9407 (Checked (trace)): every landing slid
  with the ground velocity kept from before the takeoff (`shared::physics::GROUND_V` wasn't cleared in
  the air). KH2 has one velocity `+0x20` for air and ground (Checked (decomp)), so the air speed
  carries into the landing. Now cleared by each air step. During the landing's frames 0..11 KH2 clears
  the stick target (`0x1403A89A0`; Wisdom's 5: range 2 from 12, 42 all through, Checked (files)), so
  the inertia brakes and he turns from 12, unchanged.

## 12. Story flags: when each form opens (main-port round 1, 2026-10-29)

Research for `docs/main-port-inventory.md` row 8 (round 1 there has the plan and the questions). Base t924.
Tags as at the top of this file; main's own "Checked" tags count as Assumption.

### 12.1 What KH2 does

**Form ownership is a form item in the inventory.** Checked (decomp).
- `0x1403C3EF0(form)` returns the form's item id from a u16 table at `0x1405C5D08` (index = form 1..6).
  The table's values are data, not in the export.
- `0x1403C4720(item)` = owned: the item record (`0x1403E39A0`, 0x18-byte rows of the item table at
  `*0x142A25370`) either has flag `+3` bit 0 (then a bit in `0x1409ACF70`) or is a count in
  `0x1409ACE30[record +0x12]`.
- Item ids: **Final Form = 0x1D, Checked (decomp)** (`0x140421D60`, `0x140422050` test and give item
  0x1D). Valor 0x1A, Wisdom 0x1B, Master 0x1F, Anti 0x1E, Limit 0x233: **Assumption** (community
  tables; only 0x1D is visible in code). Whether form items are count or bit: Assumption.
- The form item's record `+0x10` is its command; the command row (`0x1403B68B0`, 0x30-byte rows at
  `*0x142A161E8`) has `+2` kind (4 = Drive form, 5 = Revert, 0x48 = Limit-type), `+4` the form id
  (1 Valor, 2 Wisdom, 3 Limit, 4 Master, 5 Final, 6 Anti), `+0x1C` the cost in bars. Checked (decomp).

**The Drive submenu** (`0x1403F3110`, and the same code inline in the command-menu builder
`0x1403F29E0`). Checked (decomp):
- Built only if none of: global lock `0x142A10504` bit 1, lock `0x142A10508`, the player's entity
  record (`0x1404AD270(player +0xC)` `+0xC` bit 5), area flag `0x1403A3A90` (area record `+4` bit 1).
  Interpretation (no-drive worlds / states): Assumption.
- For form = 1..6 in order (**Valor, Wisdom, Limit, Master, Final, Anti**): if the form item is owned,
  take its command and run the partner rule `0x1404005E0`: result 2 → **not listed**; result 1 → listed
  **greyed** (entry state 5); 0 → listed. A form not owned is **not listed at all** (no "???" entry).
- If at least one form was listed, the menu's "no Drive" flag (`+0x24` bit 0x20) is cleared.
- Anti is listed only if its item is owned (Assumption: vanilla never gives it, so Anti never shows).
- Entry state per frame (`0x140401D60`, kind 4): greyed (5) when bars `status +0x1B1` < cost
  `cmd +0x1C`, or any of the locks above, partner rule ≠ 0, form id `+0xDE0` is 7 or 0xC, Limit
  (form 3) while `0x1403A3A90`, or `0x142A11400` bit 18. Usable test `0x140401C80` is the same.
- **Partner rule `0x1404005E0`** (and its twin `0x140400740`): skipped when the world factor
  `0x1403A3990` (`*0x142A0FF68 +0x6A8`) > 1.0, and for Limit. Valor counts partners of type 3, Wisdom
  type 2, Master/Final/Anti every partner slot. A counted slot empty → 2; a counted partner not ready
  (`0x1403C38F0` false) → 1; nobody counted → 2. Type 3 = Goofy, type 2 = Donald: Assumption (KH2's
  known absorption).
- **The command menu's fourth slot** (`0x1403F29E0`): with the "no Drive" flag clear, Drive (cmd 4),
  greyed if `+0xDE0` ≠ 0; in a form (`0x1403AA4F0`: player type 1 and `+0xDE0` 1..6) **Revert** (cmd 5),
  greyed in Anti while `0x142A11404` ≠ 0. With the flag set (no form owned, or a lock), command **0x68**
  takes the slot (0x68 is also put in Magic's slot when Sora has no magic: a placeholder entry, meaning
  Assumption). In Limit Form (`+0xDE0` = 3) the Magic slot becomes the Limit list (0x2C3).

**Choosing a form** (`0x140405440`, kind 4). Checked (decomp): `0x140421D60(player, form)` may turn the
choice into Anti (6) or Final (5): Anti's counter (record 6 `+4`) +1 per Drive, Anti chance from the Anti
parameter block `*0x142AE5760`; when Anti would happen and Final Form (item 0x1D) is not owned, story flag
`0x4842` is set (`0x14039A980`, bits at `0x1409AB540`) and the bars cover Final's cost, a second roll
(`pow(+0x9C, Final record +4) × +0x98`, capped `+0xA0`) **gives item 0x1D and drives into Final**
(the counter reset). Then the bars are paid with the result form's cost (`0x1403C2780`; drive points
`+0x1B0` zeroed if bars were full or short), `0x1403E69A0(form)` counts drives per form (`0x1409AD884
[form]`, cap 999, plus an achievement 0x94 under a world/flag condition), `0x14040CE60` starts it.

**Entry refill** `0x1403C17C0(status, form, level)` (from `0x14040CC00`, `0x14040CF20`, `0x14040D000`,
`0x14040D4E0`). Checked (decomp): HP to max (`+4`), MP to max (`+0x184`, also clearing MP Charge
`+0x1BC..` when `+0x1C0` > 0), `+0x1AF` = 2, form gauge `+0x1B3` = level + 3, form time `+0x1B4/+0x1B8`
= (level + 3) × `*0x142AE5760 +0x68`; Anti's level = `+0x8C` − 2.

**What happens when a form item is gained** (`0x1403C3FE0(item, n)`, the inventory add; `0x1403C3F00`
is the checked wrapper). Checked (decomp):
- the count / bit is added;
- kind `+2` == 0x15 (a Drive form item): **Anti's record `+4` (the Anti counter) is set to 0**;
- for forms 1, 4, 5 (Valor, Master, Final), if the form's record `+0` (its Keyblade) is 0, the **first
  owned Keyblade in item-table order** (records of kind 2, not kind 0x13) is **taken out of the inventory**
  (`0x1403C4A40`, one) and put in the record; none owned → stays 0;
- **no popup** and no level set: nothing else in this function for kind 0x15 (the form record's level
  is read as ≤ 1 by `0x1403E87F0` when 0).
- Form records: `0x1403A1410(0x1409ABDA0, form)` = `+0xE04 + (form−1) × 0x38`, 10 records; `+0` Keyblade,
  `+2` level, `+4` EXP. Checked (decomp).

**How KH2 tells the player.** Checked (decomp) unless marked:
- The bonus-level path: `0x1403EDCA0(bonus id)` (from `0x1403A41E0`, `0x1404109B0`, `0x140410AF0`) →
  `0x1403EDF90(row, status)` for each `bons` row (`*0x142AE58F8`, 0x10 bytes: `+1` character, `+2` HP,
  `+3` MP, `+4` Drive, `+5..+7` slots, `+8`/`+0xA` two items) gives the items with `0x1403C3F00`, adds
  each as a line (`0x1403ED400`, message 0x8AE8 with the item's name), and shows the window
  `0x1403EDB20` → `0x14017DD40`. So a form given as a bonus reward appears as a line in KH2's bonus
  window. Whether vanilla KH2 gives forms through bonuses or through event scripts: **Assumption** (data,
  not code).
- Event/progress item lists `0x1403E4530` / `0x1403E46B0` give items with no window of their own.
- Final's gain in battle (`0x140421D60`) has no window: the Final transformation itself is the event.
- **"New" marks in the pause menu**: `0x140369520` sets a new-mark bit (`*0x140BF0DC0 + 200`) for each
  owned form not yet seen (seen bits at menu data `+0x4274`). Missing from main and testing.

**Every other reader of the form-owned state found.** Checked (decomp):
- **Form level cap** = 2 + forms owned among 1..5 (Valor, Wisdom, **Limit**, Master, Final; Anti not
  counted): `0x1403E88B0`, inline in the EXP add `0x1403E8380`, and `0x1403E8A00` (next row for the
  status page). At the cap EXP is set to 0. (Testing's `formlevel` already follows the formula.)
- Pause-menu form pages: `0x1402FC500` (bitmask of owned forms 1..6) and `0x1402FC880(form)` return none
  in certain worlds (list at `0x1405BAC00` starting with world 0x0A; contents not in the export,
  Assumption: Pride Lands and similar); callers `0x140352350`, `0x1403527A0`, `0x140353020`, the status
  page `0x140354340` (shows each form's level `record +2`, gated by the page's bits).
- `0x1403DE170` / `0x1403DE370` / `0x1403DE460` list owned forms among Valor, Wisdom, Master, Final
  (Limit skipped) and pick one at random for a kind-0x48 command 0x59 (0x132 picks Valor), from
  `0x140405440` / `0x140405A60` and the menu builders `0x1403F3230` / `0x1403F3860` (character 0xC9).
  What this command is: not traced (Assumption: a Limit / special that uses a form's look).
- `0x1403B6920(form)` = the form's command row (cost reads in `0x140405440`, `0x140421D60`).
- `0x1403EDB90` draws a form's name/icon in the level-up window (form level-up `0x1403E8380`).

**Drive gauge maximum.** Checked (decomp): the only gameplay writer that raises `status +0x1B2` is the
bonus level `0x1403EDF90` (+ row `+4`, cap 9). Owning a form does **not** raise it. Other writers:
`0x1403C1D90` (loads it from the character's save record `+0x103A`), `0x140415640` / `0x1404158C0` (set 3
on a status they create; which object: Assumption). Main's doc claim "KH2 raises the max per form owned"
is not supported by the decomp; per-fight bars are bonus data (Assumption which bonuses carry +1).

**Other special cases:** area flag 0x400 (`0x1403A3C40`) stops form EXP; summons/other states via
`+0xDE0` 7 / 0xC grey Drive (which states: Assumption); worlds with factor > 1.0 skip the partner rule
(which worlds: Assumption). Timeless River / Space Paranoids: no explicit check found (the entity-flag
and world-list paths above are the likely ones, Assumption).

### 12.2 How KH1 does it

KH1 has no forms. What the native version touches:
- **Save:** block 0 at `0x142DE9360` (`0x1402CEE20(0)`; pointer `0x142868BA0`); area 1 =
  `0x142DE9F60` = save + 0xC00. Checked (decomp). Testing's `SAVE_RVA` / `SAVE_PTR_RVA` map to these
  (Checked (code)).
- **Story bytes `+0x1504`:** `0x1401B6B00` returns area 1 + 0x904 = save + 0x1504 (12 bytes, caller
  `0x14017D580`). Checked (decomp). The thresholds (0x2B, 0x32, 0x82, ...) are main's: Assumption.
- **Journal `+0x16E3`:** event command `0x1401B5980` sets bit `id` (high bit first) at area 1 + 0xAE3 =
  save + 0x16E3; `0x1401B6640(id)` reads it; `0x1401B61B0` too. Checked (decomp). Entry ids 0x9F etc.:
  Assumption.
- **Cups `+0xF36 + cup`:** no direct displacement found in the export (script variables reach them through
  the VM); Assumption (main's).
- **Dive choice `+0x48C`:** written only by event command `0x1401B0380` → `0x140285730`; read only by
  the level-up `0x1402B51B0` as the index of Sora's ability-order table (`0x140528D10[choice]`).
  `+0x48D` (EXP curve) by `0x1401B03B0` → `0x140285720`, read by `0x140284840`, `0x140284990`,
  `0x140284FB0` (`0x140528028[curve]`). Checked (decomp). Which value is Sword / Shield / Rod:
  Assumption. (Main's "set by 0x1402835B0" is the Epic address.)
- **Inventory `+0x499 + id`:** `0x1400D3F00` reads and writes it. Checked (decomp).
- **KH1's notification boxes:**
  - Level-up prompt boxes `0x14283B390` (count per member `0x14283B380`, `0xBA0` each, 5 per member);
    KH1's own adder `0x140272540(actor, line1, line2)` (title from message 0x127 + party id, colours
    `0x140527A10..` / `0x140527A50..` by member), called only from the level-up `0x1402B53B0`;
    `0x1402728A0` = any box up. Checked (decomp). Testing's `prompt()` writes the same record layout with
    its own title (Checked (code)) instead of calling `0x140272540` (which can only use KH1's member
    title).
  - The event reward line `0x1401B8AE0` (Epic `0x1401B6990`): adds "obtained" lines for a reward
    record (kind 0 item, 1 ability) to an event message window. Checked (decomp). A form would need a new
    reward kind there.

### 12.3 1:1 checked

| Value / rule | KH2 (Steam) | KH1 / testing | Matches? |
|---|---|---|---|
| Drive list order Valor, Wisdom, Limit, Master, Final, Anti | `0x1403F3110`, `0x1403F29E0` | `drive.rs:32` Valor, Wisdom | Yes (for the two built) |
| Listed only if the form is owned | `0x1403C4720` via `0x1403C3EF0` | always listed | **No** |
| Not owned → not shown (no placeholder entry) | `0x1403F3110` | always shown | **No** |
| Partner rule: missing → hidden, KO / not ready → greyed | `0x1404005E0`, `0x1403F3110` | none (row 10) | **No** |
| Greyed when bars < cost | `0x140401D60` | greyed when MP ≤ 3 (MP cost, owner's toggle pending) | MP stands in |
| No Keyblade needed to choose Valor | `0x140401C80` (no record read) | Valor refused without a spare Keyblade (`form.rs:340`) | **No** |
| Form Keyblade slot filled at gain (first owned Keyblade) | `0x1403C3FE0` | `+0x488` picked at use (`valor::keyblade`) | **No** |
| Anti counter reset at gain | `0x1403C3FE0` | no Anti | n/a |
| Level cap 2 + owned (1..5) | `0x1403E8380`, `0x1403E88B0` | `formlevel::cap(2 + Final)` | Formula yes, input **no** |
| HP **and MP** refilled on entry | `0x1403C17C0` | HP only (`form.rs:275`, KH1 add-HP Steam `0x1402A4920`) | **No** |
| Max bars not raised by owning forms | `0x1403EDF90` only | no gauge | n/a |
| Final Form item 0x1D, by the Anti roll after flag 0x4842 | `0x140421D60` | Final on hold | n/a |
| No popup in the item add itself | `0x1403C3FE0` | — | — |
| Story bytes save + 0x1504 | — | `0x1401B6B00`; `menu.rs:40` | Yes |
| Journal save + 0x16E3 | — | `0x1401B5980`, `0x1401B6640` | (main's offset confirmed) |
| Dive choice save + 0x48C | — | `0x1401B0380`, `0x1402B51B0` | offset yes, values Assumption |

What I couldn't check: KH2's form item ids other than Final's; whether form items are counts or bits;
what command 0x68 shows; the no-drive world list `0x1405BAC00`; the text of KH2's bonus line (message
0x8AE8); in-game behavior of anything (no traces).

### 12.4 Missing from main

1. Ownership as stored state (KH2: an item in the save). Main derived `unlocked` from story bytes on
   every ask (each Drive list rebuild) and only remembered the last set in memory for the box.
2. Partner rule in the list: hidden when the partner is missing, greyed when KO'd; Master/Final need two
   partner slots filled; skipped in world factor > 1.0 (main checked on entry only, row 10).
3. Form level cap from forms owned (Limit counts, Anti doesn't) (main had no KH2 form levels).
4. The form's Keyblade slot filled at gain from the first owned Keyblade (main s306 used the Dream Sword
   default instead, an owner rule).
5. Anti counter reset at a form gain; Final by the Anti roll after story flag 0x4842 (main used Riku 2,
   the owner's).
6. "New" marks in the pause menu for a form not yet seen (`0x140369520`).
7. Drive slot replaced by a placeholder when no form is owned or a lock holds (main showed the Drive
   list; s323 hid the gauge until the first form, an owner rule).
8. No-drive area / entity / lock checks (`0x1403A3A90`, `0x142A10504/08`, entity flag bit 5).
9. Per-form drive counters (`0x1403E69A0`), and its achievement.
10. Forms hidden from the pause-menu pages in certain worlds (`0x1402FC500`).

### 12.5 Missing on testing

1. Form-owned state, and the Drive list built from it (row 8).
2. Partner rule (row 10).
3. MP refill on entry (KH2 `0x1403C17C0` refills HP and MP; testing `heal_full` HP only). Checked
   (code + decomp).
4. Level cap reads a fixed "Valor and Wisdom owned" (`formexp.rs:155`): should read the owned count.
5. Valor usable without a spare Keyblade (KH2) vs testing's refusal; KH2's slot fill at gain.
6. KH2's lock checks (areas where Drive is unavailable) beyond `shared::active()` / `busy()`.
7. The "new" mark / any notification for a gained form (the box is the owner's replacement).
8. Drive counters (low value; ask).

### 12.6 What KH1 lacks, and the ability to add

- **A form-owned field in the save.** Free bytes, checked against testing's own uses (grep of every save
  offset in `crates/`: `+0x44`, `+0x488`, `+0x48A..B` mark, `+0x499..`, `+0x4CC..` swapped only during the
  Abilities page, `+0x844..+0x847`, `+0x874`, `+0x882..+0x887`, `+0x1504`; config block `+0x3C0..+0x3EF`):
  - **save `+0x881`** (one byte, between KH1's bool `+0x880` and testing's form levels): no Steam
    instruction reads or writes `+0x881..+0x887` (Checked (exe), testing's `docs/kh1-drive-forms-native.md`
    §9 / `formexp.rs:8-13`); unused by testing (Checked (code)). Travels with the 0x16C00-byte save.
    Bits 0..5 Valor, Wisdom, Limit, Master, Final, Anti (KH2's form ids − 1), bit 7 = "initialised".
  - Also free: save `+0x489` (main's Final Keyblade, unused on testing; better kept for Final's slot) and
    the config block `+0x78..+0x3BF`, `+0x3F0..+0x3FF` (Checked (decomp) per `docs/abilities.md`).
- **A notification for a gained form:** KH1's level-up prompt box, as testing already uses it
  (`queue_prompt`). Nothing new needed.
- (Option, more like KH2's item) KH1 item records for the forms in KH1's inventory (`+0x499 + id`):
  needs unused KH1 item ids and a check that KH1's Items menu doesn't list them (Assumption).

### 12.7 Not found, and where I looked

- KH2 form item ids except Final (0x1D): the table `0x1405C5D08` is data; grepped every literal item id
  passed to `0x1403C4720` / `0x1403C3FE0` / `0x1403C3F00` (only 0x17 and 0x1D).
- KH2's on-gain notification for event-given forms: none in `0x1403C3FE0`, `0x1403C3F00`,
  `0x1403E4530`, `0x1403E46B0`; the bonus window path exists (section 3); event scripts are data.
- The meaning of command 0x68, of `+0xDE0` values 7 / 0xC, of the world list `0x1405BAC00`, of the world
  factor > 1.0, of command 0x59 (`0x1403DE460`).
- KH1 cup bytes `+0xF36..`: no displacement in the export (script VM path); not confirmed.
- The thresholds and journal ids of main's table: not rechecked (they come from KH1's scripts and save
  snapshots, files I don't have here).
- Nothing was checked in game.

### 12.8 Built (t925 to t930)

- **t925** `win/formown.rs`: the owned forms at save `+0x881` (bits by KH2's form id − 1, bit 6 "the
  table read", bit 7 "set up"); set up once a save in `menu::Growth::update`'s once-a-second pass; the form
  level cap reads it (`formexp::cap`, KH2's 2 + forms 1..5 owned).
- **t926** Config Top > Progression > Story Unlocks (ini `story_unlocks`, off): on, `crate::storyforms`
  (the owner's table) gives forms in the save, silently on a save's first read or right after the row is
  switched on. Off: Valor and Wisdom, as before.
- **t927** Config Top > Drive > MP Refill (ini `drive_mp_refill`, off): KH2's MP refill on entry
  (`0x1403C17C0`) through KH1's MP add (Steam `0x1402A4A00`, Checked (decomp), its bytes Checked (exe)).
- **t928** the Drive list holds the owned forms only, in KH2's order; with none owned the Drive row is KH2's
  greyed "?" (command 0x68, Checked (files)).
- **t929** the "New Drive Form" box (KH1's prompt box) for a form gained during play.
- **t930** Valor gained: its empty Keyblade slot (`+0x488`) gets the first spare Keyblade (KH2's
  `0x1403C3FE0`); the refusal without one stays.

Still open: Limit and Master (stored, no form yet); the partner rule (row 10); KH2's no-Drive locks; drive
counters and the pause menu's "new" marks (later, answer 9 (b)). Nothing checked in game.

## 13. The Drive gauge's data and rules (main-port round 2, 2026-10-29)

Research for `docs/main-port-inventory.md` row 1 (round 2 there has the plan and the questions). Base t930.

### 13.0 Corrections to testing's own notes (kh1-drive-forms-native.md §2, §6, §12)

- §2 says the `sstm` numbers +0x68 (600) and +0x6C (0.5) are an Assumption. They are now **Checked
  (files)** (section 3). `sstm` starts 8 bytes into its entry: a u32 version (1), then a u32 offset (8).
  The game's pointer `0x142AE5760` is entry + that offset. Checked (decomp), from the loader at line ~768957.
- §6 says the Anti counter and roll were not located. They are `0x140421D60` (section 1.9).
- §12.1 calls command kind 0x48 "Limit-type". It is **summons**. `0x140405440` / `0x140405A60` handle
  kind 0x48 by paying bars (`0x1403C2780` with the row's +0x1C cost) and starting the summon
  (`0x1403DE460`). The cmd rows 0x58, 0x59, 0x132 and 0x1BF are kind 0x48 with cost 3 (Checked (files)).
  Command 0x59, which picks a random owned form, is the summon that copies a form (Genie, Assumption
  from KH2 knowledge). Checked (decomp + files) except that name.
- §2's "mode 4 (set at a scene)": the player-object constructor `0x140405030` sets mode 4 when
  `0x142A11400` bit 18 is set. That bit is set by recovery request kind 5 (`0x1403A5650`). Checked (decomp).

### 13.1. KH2's gauge, the whole system (Steam KH2)

#### 13.1.1 The record (status = player `+0x5C0`; the player object also keeps it at `+0x170`)
| Field | What | Tag |
|---|---|---|
| `+0x1AF` | mode: 0 none (status built, `0x1403C0010`), 1 Drive bars, 2 form, 3 summon, 4 no gauge (flag bit 18) | Checked (decomp) |
| `+0x1B0` | points in the current bar, 0..100 (100 only when the gauge is full) | Checked (decomp) |
| `+0x1B1` / `+0x1B2` | full bars / maximum bars | Checked (decomp) |
| `+0x1B3` | form gauge bars (level + 3) | Checked (decomp) |
| `+0x1B4` / `+0x1B8` | form gauge now / full, float, in 60 Hz ticks (section 1.12) | Checked (decomp) |
| `+0x1BC` / `+0x1C0` | MP Charge timers (>0 = charging, `0x1403C1D10`) | Checked (decomp) |
| `+0x1D0..` | ability factor block (`0x1403D7470`), set up by `0x1404016B0`, filled by `0x140401730` | Checked (decomp) |

The ability block fields the gauge reads (`0x140401730`, Checked (decomp)). Each sstm value is
Checked (files):
- `+0x40` Drive Boost (ability 0x18D): starts at 1.0, + sstm +0x13C (0.2) per copy.
- `+0x44` Form Boost (0x18E): starts at 1.0, × sstm +0x140 (0.8) per copy.
- `+0x58` Damage Drive (0x18C): starts at **0**, + sstm +0x138 (50.0) per copy.
- `+0x70` Summon Boost (399): 1.0, × sstm +0x16C (0.8) per copy.
- `+0x74` Drive Converter (0x21C): 1.0, × sstm +0x170 (0.5) per copy (row 5, report only).

#### 13.1.2 Where the gauge lives in KH2's save (Checked (decomp))
- The status is loaded from the save in `0x1403C1D90` (through `0x1403C0620(1)`; status `+0x26C` = the
  save `0x1409ABDA0`, set only for characters 1 and 0xE). Save `+0x1038` points, `+0x1039` bars,
  `+0x103A` max, `+0x103B` form bars, `+0x103C` form time, `+0x1040` form length.
- It is written back by `0x1403C2120` every time a status reference is let go (`0x1403C07E0`, 22
  callers). So the gauge, and a form's time left, survive room changes and saves.
- The save's current form is `+0x1034` (byte, `0x1409ACDD4`), with `+0x1035` beside it (a summon,
  Assumption).
- **A new game starts full**: `0x1403A0E70` writes `+0x1038` = 100 points, bars 3, max 3. So main's
  s323 "full by default" is also KH2's rule.

#### 13.1.3 Adding points `0x1403C0AD0 (status, n)` (Checked (decomp))
- Points carry into the next bar at 100, up to the maximum. In the last bar they hold at 100.
- A negative n takes from the current bar. If that goes under 0, one full bar is broken
  (points = result + 100). With no bars left, points go to 0.

Every caller (Checked (decomp)):
- the hit/orb add `0x1403A9CF0`;
- Drive Recovery items `0x1403C4B80` (items 0x112 / 0x113, n × 100; usable while bars < max, `0x1403C3DA0`);
- the bonus level `0x1403EDF90` (adds 0, see 1.8);
- area-program command 12 in `0x1403A31A0`: adds raw points to the player in mode 1, with no factors;
  which areas use it is data, Assumption;
- a separate player type 0xB's step (`0x140414EB0` / `0x140415DD0`): its own 3-bar gauge, 1 point every
  sstm +0xD4 (4.0) ticks while it charges. Not Sora's rule (which mode it is: Assumption, a minigame).

#### 13.1.4 The hit/orb add `0x1403A9CF0 (player, value)` (Checked (decomp))
- **Mode 1**: value × (sstm +0x78 = 2.0 × Drive Boost) while MP Charge runs, × the world factor
  `0x1403A3990`, truncated to an int, then `0x1403C0AD0`.
- **Mode 2** (form): the form gauge gets + value × sstm +0x70 (12.0) ticks. This is skipped when the
  form id `+0xDE0` is 0xC.
- **Mode 3** (summon): the same + value × 12.
- **World factor**: the area object `0x142A0FF68 +0x6A8`, set by area-program command 11
  (`0x1403A31A0`). With no area object it is 1.0. Its default when the program doesn't set it: not
  found (Assumption 1.0).

#### 13.1.5 The hit rule (Checked (decomp))
The attack owner's hit callbacks are `0x1403A7480` (by vtable) and `0x1403A8800` (from the form and
other state wrappers `0x1404042F0`, `0x140404EC0`, `0x140405DC0`, ...). They get the ATKP row as their
5th argument: hit record `+0x20` holds a handle to the row that `0x1403CF600` picked (`0x1403CE740`,
`0x1403D12C0`, `0x1403D1730`).
- The value is the ATKP row's byte **+0x29**: testing's `Atkp::drive_drain`. Main called it "ATKP
  drive value".
- Nothing is given unless the value ≠ 0 and the **target** has class bit 4 (`+0x6C8` 0x10). The base
  enemy constructor `0x1403DB840` sets that bit. The object factory `0x1403DF930` builds objentry type 3
  with `0x1403F8DA0` (bit 0x400) and type 4 with `0x140419E30` (bit 0x800), both on top of it. So bit 4 =
  any enemy, and bit 11 = the type-4 class. Checked (decomp). That type 3 is "boss" and type 4 is
  "normal enemy" comes from OpenKH's objentry names: Assumption on the names only.
- **Mode 1**: value (× sstm +0x60 = 0.75 on a bit-11 enemy), then `0x1403A9CF0`, so the result is also
  truncated and × MP Charge × world.
- **Mode 2**: the form gauge **loses** value × sstm +0x74 (15.0) ticks (× sstm +0x64 = 0.2 on a bit-11
  enemy). Hits drain a form.
- **Mode 3** with time left: as mode 1, so through `0x1403A9CF0` + value × 12 (or × 0.75 × 12).

#### 13.1.6 Damage taken (Checked (decomp))
- The HP-change callbacks `0x1403A75F0` / `0x1403A85F0` act only in mode 1 and only on a loss. They give
  `damage / max HP × block +0x58` through `0x1403A9CF0`.
- `+0x58` is 0 without Damage Drive, so **vanilla KH2 gives nothing for damage taken**. With Damage
  Drive it is 50 per copy (and × world factor, × MP Charge).
- In a form, damage never touches the form gauge.

#### 13.1.7 Paying a cost `0x1403C2780 (status, cost)` (Checked (decomp))
- If bars == max (full) or bars < cost, points are set to 0. Then bars −= cost, floored at 0.
- Callers: the Drive command `0x140405440` (kind 4, using the cost of the form that comes out of the Anti
  roll, `0x1403B6920(form) +0x1C`), and summons (kind 0x48, `0x140405440` / `0x140405A60`).
- Anti costs 9, so it empties any gauge.
- Greying (`0x140401D60`, §12 of the doc): bars < cmd cost.

#### 13.1.8 Raising the maximum `0x1403EDF90 (bons row, status)` (Checked (decomp))
- max += row +4, capped at 9, then add 0. A full gauge stays full: the new bar comes in filled.
  A gauge that isn't full gets nothing.
- On Critical (`0x1409ABD48 == 3`) only the HP and MP bonuses are halved, not the Drive bonus.
- No other writer raises `+0x1B2` for Sora: the only others are the load and the type-0xB object.
  Owning forms raises nothing (§12 confirmed).

#### 13.1.9 Entering, reverting, the Anti roll (Checked (decomp))
- **Enter** `0x1403C17C0 (status, form, level)`:
  - HP full, MP full (and MP Charge cleared);
  - mode 2; `+0x1B3` = level + 3; form gauge = (level + 3) × sstm +0x68 (600);
  - Anti's level = sstm +0x8C (5) − 2 = 3, so 6 bars.
  - The level comes from the form's save record `+2` (`0x14040CC00` etc.).
- **Revert on empty**: the form state's step `0x140404A40` (twin `0x140405320`). When form time ≤ 0, it
  is in a form (`0x1403AA4F0`), and three player checks pass, it issues the Revert command (cmd 5,
  `0x1403A9EA0`), then change-to-form-0 (`0x14040CE60`). If the command can't run that frame it tries
  again on the next.
  - A non-form 0xC state reverts through the revert task.
  - A summon (`0x1403DE250() == 2`) ends with cmd 0x13C, and mode 1 is set again.
- **At a revert the bars are untouched**: no refill, no change. Base Sora's new object is built in mode 1
  (`0x140405030`; mode 3 if a summon is out; mode 2 with level −1 for form id 0xC). His bars and points
  come back from the save record.
- **Room change in a form**: the form id and its time left are in the save record, and the form's
  arrival state `0x140405630` sets mode 2 again. So the form and its time carry over.
- **Anti roll** `0x140421D60 (player, form)`:
  - No roll at all while `0x140749804` bit 25 is set (Assumption: a mode or option flag).
  - Final chosen: Anti counter (form record 6 `+4`) −= sstm +0xA4 (10), floor 0.
  - Otherwise, if Anti is allowed (`0x140400740(6) == 0`), the chance is the area byte (`0x142A0FF68+8`)
    `+0xE` (`0x1403A3950`; 1 with no area) × the fmlv row byte +1 found from Anti's fmlv rows by the
    counter. It is 100 when ability-block bit (block `+0x14` bit 15) is set (Assumption: an "always Anti"
    ability or scene).
  - Counter +1. Anti happens only in battle (`0x142A11404 ≠ 0`, `0x1403ABB20`).
  - If Anti hits while Final is not owned, Final is allowed, story flag 0x4842 is set and bars ≥ Final's
    cost (5), there is a second roll. Its chance is min(sstm +0x98 (3.0) × sstm +0x9C (3.0)^(Final
    record `+4`), sstm +0xA0 (75.0))%. Each try adds 1 to Final's counter. On success: Final's counter
    0, item 0x1D given, Anti counter −10, and the result is Final.
  - Otherwise the result is Anti and the counter −= sstm +0x90 (6).
  - The Anti fmlv rows are in 00battle.bin, which isn't staged. Testing's `character_core::drive` says
    "10% from 5, 25% from 10" (Epic): Assumption until 00battle is read.

#### 13.1.10 The form gauge change `0x1403C0B50` and the drain (Checked (decomp))
- `0x1403C0B50` adds within 0..full length. In Anti (`+0xDE0 == 6`) a gain becomes a loss, so orbs
  drain Anti. In summon mode it counts the bars lost for summon EXP (`0x14040D530`).
- **Drain** (player step `0x1403A92C0`), in modes 2 and 3:
  - −(one step's ticks × (sstm +0x6C = 0.5 when NOT in battle, `0x142A11404 == 0`) × Form Boost
    (mode 2) or Summon Boost (mode 3)).
  - No drain while `0x142A11478 ≠ 0` (`0x1403AC220`; Assumption: an event or pause hold).
  - So it drains 1 tick per tick in battle and 0.5 outside.
- Other callers: the two hit callbacks (1.5), the prize code `0x1403DDB50` / `0x1403DEA20` (row 3).

#### 13.1.11 Where KH2 refills the bars (Checked (decomp))
The only full refill is `0x1403C2020`: bars = max, points 100, form time 0. Its only caller is
`0x1403A5790`, which also clears the save's form (`+0x1034`). That one is called only by the recovery
request `0x1403A5650(kind)`:
- Kinds 2, 6, 10, 0xC: revert + full gauge + full HP/MP.
- Kinds 1, 4, 5: the same, but **only when in a form or summon** (save `+0x1034` / `+0x1035` ≠ 0).
  Otherwise kind 1 is just full HP/MP, and 4 / 5 do nothing (5 also sets the no-gauge flag bit 18).
- Kinds 7, 8, 9 and others: full HP and MP for all 15 characters, gauge untouched.
- Kind 0xB: clears Anti's form only.

Who asks for which:
- Entering the world map (`0x1403FC350` case 0xC) asks for **kind 1**. Leaving for the world map in a
  form reverts and refills; leaving out of a form doesn't touch the gauge. Checked (decomp). That 0xC is
  the world map: Assumption, from the `worldobj` / `WM_CURSOR` loads in the same case.
- Map jumps carry a kind (`0x140152990` / `0x140152A90`), and so do area programs (command 0x1D in
  `0x1403A24C0`). These are event data, Assumption which ones.
- `0x1403E68F0` asks for kind 0xC, from a menu `0x1403A3DD0(3)` and two other menus (Assumption: the
  game-over Continue / warp menus).
- A summon's end `0x140416100` asks for kind 10 (Assumption: summon end).
- **Not found**: any save-point refill of the gauge, any level-up refill (the level-up never calls the
  add or the refill: the full caller lists above), any world-change refill out of a form.

#### 13.1.12 Time units (Checked (decomp))
- The step time `0x140717480` starts at 2.0 and is the frame's 60 Hz tick count × a speed scale
  (`0x140154740`).
- MP Charge times are stored as seconds by dividing by 60.0 (`0x1403C2120`).
- So every frame value here is in **60 Hz ticks**: a bar of form gauge (600) is 10 s in battle and 20 s
  outside. Testing's `takeover::steps()` already counts 60 Hz steps (Checked (code)). KH1's own tick
  wasn't re-checked in the KH1 decomp this round (testing's `count_steps` reads KH1's frame scale:
  Checked (code) only).

### 13.2. Where each Drive value comes from (summary)

| Source | KH2 rule | Address | Tag |
|---|---|---|---|
| Hits | ATKP +0x29 on enemies; ×0.75 on type-4 enemies; ×2 (×Drive Boost) in MP Charge; × world; truncated | `0x1403A7480`, `0x1403A8800`, `0x1403A9CF0` | Checked (decomp) |
| Hits in a form | form gauge −15 × value (×0.2 type 4) | same | Checked (decomp + files) |
| Damage taken | only with Damage Drive (50 × dmg / max HP per copy) | `0x1403A75F0`, `0x1403A85F0` | Checked (decomp + files) |
| Orbs | prize row Drive byte through `0x1403A9CF0` (row 3) | `0x1403DCA50` | Checked (decomp); the float argument is dropped by the decomp (Assumption: the byte) |
| Items | +300 / +900, bars < max | `0x1403C4B80`, `0x1403C3DA0` | Checked (decomp) |
| Area command 12 | raw points, mode 1 | `0x1403A31A0` | Checked (decomp) |
| Refills | world map in a form; scripted recoveries; Continue | `0x1403A5650` | Checked (decomp); triggers Assumption |
| Bonus levels | max +n (cap 9), full stays full | `0x1403EDF90` | Checked (decomp) |
| Drive Converter / Damage Drive / Form Boost / Drive Boost | the ability block | `0x140401730` | Checked (decomp + files); row 5 |

A Limit-type use `0x1403D8B40` (limit row byte +2 in 0xC8..0xCC, `0x1403E8300`) calls `0x1403A9CF0`
with an argument the decomp drops. The other limits pay MP (an area can override the cost, area
`+0x6A4` via command 10, `0x1403A39B0`). Assumption: those limits spend summon or form gauge. This is
not part of base Sora's rules.

### 13.3. 03system values (Checked (files): /mnt/user-data/uploads/data/kh2/03system.bin)

`pref` is a BAR inside 03system (entries plyr, fmab, prty, sstm, magi). sstm is at pref +0xF14, with
the data starting 8 bytes in.

| sstm | Value | Used by |
|---|---|---|
| +0x54 | 0.5 | `0x1403C3C30` (not the gauge) |
| +0x60 | 0.75 | hit on a type-4 enemy |
| +0x64 | 0.2 | form drain per hit on a type-4 enemy |
| +0x68 | 600.0 | form gauge ticks a bar |
| +0x6C | 0.5 | drain out of battle |
| +0x70 | 12.0 | form/summon gauge ticks a point (orbs) |
| +0x74 | 15.0 | form gauge ticks lost a point (hits) |
| +0x78 | 2.0 | MP Charge factor |
| +0x8C | 5 (int) | Anti's level = 3 (6 bars) |
| +0x90 | 6 (int) | Anti counter after Anti |
| +0x98 | 3.0 | Final roll base |
| +0x9C | 3.0 | Final roll power base |
| +0xA0 | 75.0 | Final roll cap (%) |
| +0xA4 | 10 (int) | Anti counter after Final |
| +0xD4 | 4.0 | type-0xB object's charge rate |
| +0x138 / +0x13C / +0x140 / +0x16C / +0x170 | 50 / 0.2 / 0.8 / 0.8 / 0.5 | Damage Drive / Drive Boost / Form Boost / Summon Boost / Drive Converter |

The `cmd` rows (version 2, 751 rows) have the cost at +0x1C:

| Command | id (kind, argument) | Cost in bars |
|---|---|---|
| Valor | 6 (4, 1) | 3 |
| Wisdom | 7 (4, 2) | 3 |
| Limit | 0x2A1 (4, 3) | **4** |
| Master | 0xB (4, 4) | 4 |
| Final | 0xC (4, 5) | 5 |
| Anti | 0xD (4, 6) | 9 |
| Drive (menu) | 4 | 0 |
| Revert | 5 | 0 |
| Summons | 0x58, 0x59, 0x132, 0x1BF (kind 0x48) | 3 each |

There is a second set of kind-4 rows, 0x226..0x229 and 0x2A2: costs 3/3/4/5/4, a different byte
+0x1E (0x5A vs 0x64), messages 0xC32A.. (Assumption: the Auto-form reaction commands). So Limit Form
costs 4 in KH2; main's Limit 3 (s331) is the owner's balance rule.

### 13.4. The KH1 side (Steam KH1) and testing

- **Contact `0x1402BF330 (attack, target)`** (Checked (decomp)):
  - It makes the hit record (`0x1402BF630`).
  - For an attacker with stats it adds KH1's MP: attack data (`attack +0x5C`) `+0x2C` × a
    level-difference factor (btltbl +0x94 / +0x95 MP Haste), at least 1, capped at btltbl +0x96 % of
    the base, through `0x1402A4AE0`.
  - KH2's add has no level factor, so the drive add must not reuse KH1's scaling.
  - Testing already hooks this function (`probe::hit`, Steam `0x1402BF330` via `steam_map.in`
    `0x2BD180`, checked prologue). After the original runs it calls `formexp::hit(owner, target, rec)`
    when a record was made (Checked (code)). That is the place for the hit add and the form's hit
    drain: per hit, no new hook, no per-frame work.
- **Is the ATKP's drive byte reachable at contact?** Yes, Checked (code): `kh2_formats::battle::Atkp`
  parses +0x29 as `drive_drain`, and three paths give the hit's row:
  - `kh1rows::kh2_atkp(attack)`: our own KH1 rows (`DATA_AT` copies), and `bodyhits::atkp` for KH2's
    body windows and spells as KH1 objects;
  - `kh1rows::native_atkp(sora)`: base Sora's native swings, from the slot's ATKP event;
  - `combat::landing()`: gives power only today, so it would need the drive byte added.
  - `combat::scale_record` already uses the first two at the same point.
  - KH1's own hits (KH1-style combo, KH1 spells, KH1 limits, summons) have no ATKP (question 6).
- **Target**: testing's "enemy" test is actor kind byte `+6 == 3` (`formexp::hit`, Checked (code)).
  KH1's equivalent of KH2's type-3 / type-4 split (boss / normal): not found this round (question 7).
- **HP change `0x1402A4920 (actor, delta)`** (Checked (decomp), 13 callers):
  - stats `+0x3C` HP, `+0x40` max, the HUD mirror at stats `+0xC8` byte +1.
  - This is where Damage Drive's rule belongs (row 5). With no Damage Drive, KH2 gives nothing, so the
    base rule needs no code at all.
- **Battle flag**: testing reads KH1's battle flag (`BATTLE_RVA` bit 0) in
  `takeover::form_native_frame`, which already runs each physics frame while a form runs. The form
  gauge's drain fits there with no new hook (Checked (code)).
- **What the change from MP to the gauge touches** (Checked (code), `win/form.rs`):
  - `COST_MP` 3 (paid through `reflect::pay_mp`, KH1's payment `0x1402A5B60`);
  - `usable` / `possible` / `choose` test MP > 3;
  - `drain_step` takes one yellow MP unit every 3 battle frames (2 half-steps in battle, 1 outside);
  - the out-of-MP revert in `native_frame`;
  - `drive.rs` greys the list from `form::usable`;
  - `kh2dpx` builds effects from `possible`.
  With the gauge, each of these reads "bars ≥ cost" and "form time > 0" instead. Nothing story-side
  reads MP that we know of (testing's kh1-kh2-mp.md §3.3: Assumption there).
- **Save bytes**:
  - Testing uses `+0x1C..` (accessories), `+0x36` weapon, `+0x44`, `+0x74` (Donald's record, Reflect
    bit), `+0x488`, `+0x48E`, `+0x492`, `+0x499..`, `+0x4CC`, `+0x599`, `+0x5A3`, `+0x844..+0x847`,
    `+0x874`, `+0x881`, `+0x882..+0x887`, `+0x1504` (grep of every save offset in `crates/`, Checked
    (code)). Nothing at `+0x37` or `+0xAB`.
  - KH1's save field list (`0x1402CF620`) describes each 0x74-byte party record with `+0x36..+0x39` as
    "4c" (four chars). So `+0x37` is a byte field after the weapon, Checked (decomp).
  - No Steam KH1 instruction in the export reads or writes party record `+0x37`: searched for the record
    indexing (`× 0x74 + DAT_142868BA0`) and for direct addresses `0x142DE9397` / `0x142DE940B`. KH1's
    new-game setup `0x1402852C0` zeroes the block and writes `+0x36` only. Checked (decomp). Not
    excluded: reads through a record pointer held elsewhere (Assumption, the same limit as testing's
    `+0x881` check). A full instruction scan of the exe, as §9 did for `+0x881..`, would close it
    (not done this round).
  - Main's `+0x37` (bars | 0x80 written) and `+0xAB` (points) are free on testing, and keeping them
    lets a main save keep its gauge.
  - The maximum needs no byte if it is derived (main's way). KH2 stores it (`+0x103A`) only because
    bonus levels raise it.

### 13.5. Missing from main (KH2 has it, main lacked it)

1. The world factor on every hit, Damage Drive and orb add (`0x1403A3990`). KH1 has no area factor, so
   1.0 (KH2's value without an area object). Fix: none needed beyond 1.0; note it.
2. A full gauge stays full when the maximum rises (`0x1403EDF90`'s add of 0). Main capped saved bars to
   the new max, so a full 3/3 became 3/4. Fix: in the max raise, if full before, full after.
3. The form gauge and form survive room changes and saves (`0x1403C2120`, save `+0x103B..+0x1043`).
   Main reverted on a new world (s275, the owner's) and kept no form time. Fix: optional (question 10).
4. Refill rules: KH2 refills only through recoveries: leaving for the world map in a form, scripted
   recoveries, Continue (`0x1403A5650`). Main refilled on every forced revert (s338, the owner's).
5. Area command 12 (scripted Drive points) and recovery kinds in event map jumps. KH1 has no such data,
   so there is nothing to port. Note only.
6. No drain while `0x142A11478` (an event hold, Assumption). Main drained by its own per-frame tick
   [STEER]. Fix: drain only in `native_frame` while KH1 runs Sora's frame.
7. The Anti roll's area multiplier (area `+0xE`), the "always Anti" bit, the global no-roll bit, and
   Final's roll min(3 × 3^n, 75)% after flag 0x4842 (`0x140421D60`). Main: Anti free, Final by Riku 2
   (owner rules).
8. Summons on the gauge: cost 3, mode 3, Summon Boost, summon EXP from bars lost (`0x1403C0B50`,
   `0x14040D530`). KH1's summons are KH1's and stay on MP. Note only.
9. Mode 4 (no gauge) after recovery kind 5, which greys Drive (`0x140401D60`, bit 18).
10. Limit costs 4 in KH2 (cmd 0x2A1). Main's 3 is the owner's balance rule (now behind Balance Changes).
11. Main's damage-taken detection was a per-frame HP diff [STEER]. KH1's HP change `0x1402A4920` is the
    native place (row 5).

### 13.6. Missing on testing (KH2 has it, testing's forms lack it)

1. **The whole gauge**: record, save, add, pay. Fix: a gauge record in KH1's save (`+0x37` / `+0xAB`),
   KH2's add and pay rules (1.3, 1.7), as testing's own module. `character_core::drive` exists on
   testing, unused (Checked (code)). Its add / pay / tick match the Steam decomp, but it lacks the world
   factor, the type-4 class source, Damage Drive's base 0 and the refill rules, and its Anti rates are
   Epic. Reusable only with its numbers re-tagged from this round.
2. **The cost by the gauge**: Drive greyed when bars < cost (`0x140401D60`), cost paid on entry with the
   points rule (`0x1403C2780`). Fix: behind the cost toggle.
3. **The form gauge**: (level + 3) × 600 on entry, from testing's form levels (`formexp`, `+0x882..`).
   Fix: in `native_start` / `native_arrive`.
4. **Drain**: 1 / 0.5 per 60 Hz step, × Form Boost. Fix: replace `drain_step`'s MP drain when the toggle
   is the gauge (same per-frame place, no new hook).
5. **Hits drain the form**: −15 × ATKP +0x29 (× 0.2 on normal enemies), at the contact
   (`probe::hit`). Testing has nothing like it.
6. **Revert when the form gauge is out** (`0x140404A40`), through the existing revert path (testing
   already retries "as soon as he can change").
7. **Hit add at contact** (1.5), enemies only.
8. **Refill** when leaving a world in a form (kind 1) and at Continue (question 4).
9. **No MP cost or drain with the gauge** (KH2). MP refill on entry is already t927's toggle.
10. **Orbs** (row 3), **items** (row 4), **Damage Drive / Form Boost** (row 5): report only.

### 13.9. Not found, and where I looked

- KH2's save-point refill: none on the gauge in `0x1403C2020` / `0x1403A5650` callers, `0x1403C2040`
  callers or `0x1403A5760`. Save points are probably data (an object script). Not found.
- What exactly sets recovery kinds 2 / 6 / 10 / 0xC, `0x1403A3DD0(3)`'s menu, flag `0x142A11478`, and
  `0x140749804` bit 25: Assumption.
- The world factor's default, and which areas set it (area programs are data).
- Form id 0xC's meaning (a special form-like state).
- The Anti fmlv rows: 00battle.bin is needed (not staged). Please stage it for the Anti round.
- The float that `0x1403DCA50` and `0x1403D8B40` pass (dropped by the decomp).
- KH1: a boss / normal enemy signal; the world-exit and Continue signals for the refill; KH1's tick
  rate in the decomp (testing's code only).
- Nothing was checked in game, and there were no traces this round.

### 13.10 Built (t931 to t935)

- **t931** `drivegauge.rs` (KH2's rules, pure, tested) and `win/drivegauge.rs`: bars at save `+0x37` (bit 7
  written; unwritten or points 100 read as full), points at `+0xAB`; the maximum 9 (or the six fights, t935).
  Save bytes: an objdump scan of the Steam exe found no function that loads the save pointer `0x142868BA0`
  touching `+0x37` / `+0xAB` (Checked (exe)); a pointer handed in from elsewhere can't be ruled out
  statically (Assumption).
- **t932** the hit add at KH1's contact (`probe::hit` → `drivegauge::hit`): KH2's ATKP drive byte `+0x29`
  (`kh1rows::kh2_atkp`) or KH1's attack data `+0x2C`, on KH1 actors of kind 3, × 0.75 (every KH1 enemy
  normal until KH1's boss signal is found), truncated; in a form it goes to the form gauge.
- **t933** Drive > Form Cost (Drive Gauge | MP, ini `drive_cost`, gauge by default): KH2's costs (Valor 3,
  Wisdom 3) against the bars, paid at the change; the form gauge (form level + 3) × 600 ticks, 1 a frame in
  battle and 0.5 outside, hits × 15 (× 0.2 normal); empty: the revert; no MP cost or drain.
- **t934** Drive > Gauge Refill (Off | KH2, ini `drive_refill`, KH2 by default): a form running in another
  world than it was entered in (the world map between): bars, HP and MP full and the revert (KH2's kind 1
  at the world map, `0x1403FC350`).
- **t935** Progression > Drive Gauge Growth (ini `gauge_growth`, off): 3 + the owner's six fights, at most 9.

Still open: bonus levels (later, the owner); KH1's boss signal; KH2's Continue and map-jump refills; Form
Boost, Damage Drive (row 5); the gauge on the HUD (row 2); the Anti and Final rolls (with Anti). Nothing
checked in game.

## 14. The Drive gauge on KH1's HUD (main-port round 3, 2026-10-29)

Research for `docs/main-port-inventory.md` row 2 (round 3 there has the plan and the questions). Base t935.

### 14.0 What testing has now (Checked (code))

- The gauge's data and rules (t931–t935): `crates/kh1-hook/src/drivegauge.rs` (pure), `win/drivegauge.rs`
  `get()` (bars at save `+0x37`, points `+0xAB`, max derived), the form gauge in `win/form.rs`
  `InForm.gauge: Option<(left, full)>` in ticks. Nothing draws any of it.
- Testing has no HUD module. The built-once model to copy is `slotcache.rs` (manifest + block + check
  hash, temp file renamed), `tintcache.rs` (pixels ready for KH1's texture creator, lazy on a thread),
  `dpxcache.rs` (images entered in KH1's loaded files), `docs/kh1-asset-cache.md`.
- The Config menu's Drive submenu has Form Cost, Gauge Refill, MP Refill (`win/config.rs` 643–657);
  Progression has Story Unlocks and Drive Gauge Growth.

### 14.1. KH2's own Drive gauge HUD (Steam KH2)

#### 14.1.1 Where it lives (Checked (decomp))
- The field HUD is made by `0x14016C3A0`: a sprite image object `0x140ABD240`, the **player's gauge
  object** `0x140ABD248` (0x45E0 bytes, set up by `0x14017F270`), two partner gauges `0x140ABD250 /
  58` (`0x140181D20`) and the enemy gauge `0x140ABD260` (`dk::GAUGE_ENEMY`). Torn down by `0x14016C2B0`.
- The Drive gauge is part of the **player's** gauge object, as pieces (each a sprite player, 0x200
  bytes) all parented to the HP ring's root piece `+0x1D40` (group 136 / 0x88, the portrait ring):
  - `+0x3540` the bar, `+0x3740` the fill, `+0x3940` the number (placed at the bar group's attach point,
    `0x14016B2A0` with `0x140192E40/60`), `+0x370` MAX (group 20, parented to the bar), `+0x3B40 /
    +0x3B60` the fill's mask (`0x14017E580`; its box set by `0x140160CD0(mask, 0x32, 100)`, 50 × 100),
    effect pools `+0xB50` (bar gained), `+0x1330` (revert flash), `+0xF40` (form/summon change ring),
    `+0x760` and `+0x1720` (summon start/end).
  - HP / MP pieces beside it: `+0x2D40 .. +0x3340` MP (groups 0x6D–0x78), `+0x2140 / +0x2740` HP
    damage (0x7E / 0x7F).
- Set up once by `0x14017F9B0` (called from `0x14017F270`); updated every frame by **`0x140181270`**
  (HP, MP, then `0x140181AF0` MP-charge look, then the Drive part by mode); the per-mode draw-state
  is **`0x1401816C0`**; `0x140180490` counts the number shown and flags a gain; `0x140180560` picks
  the number's group; `0x140180640` the bar's steady group; `0x1401806B0` the fill's group;
  `0x140180900` runs when the party's player object changes (a form change replaces the player
  object in KH2); `0x140180B00` the return to the plain gauge (revert, summon end); `0x14017F100`
  shows / hides pieces by mode.

#### 14.1.2 The record it reads (Checked (decomp), §13.1.1 of kh1-drive-forms-native.md)
Player `+0x5C0` → status: `+0x1AF` mode (0 none, 1 Drive bars, 2 form, 3 summon, 4 locked), `+0x1B0`
points, `+0x1B1` bars, `+0x1B2` max, `+0x1B3` form bars, `+0x1B4 / +0x1B8` form gauge left / full.
`0x1403C1D10(status)` = MP Charge running.

#### 14.1.3 Pieces and groups by mode (Checked (decomp) for the numbers, Checked (files) for what they are)
| Piece | Mode 1 (Drive) | Mode 2 (form) | Mode 3 (summon) | Mode 4 (locked) |
|---|---|---|---|---|
| Bar, first appearance (slide-in) | 21 | 50 | 140 | 18 |
| Bar, steady | 23 (26 while MP Charge) | 51 | 141 | 19 |
| Fill | 25 | 49 | 139 | hidden |
| Number | table `0x14071B108`[n] | table `0x14071B130`[n] | table `0x14071B0E0`[n] | hidden |
| MAX | 20 when bars ≥ max | hidden | hidden | hidden |
Mode 0: bar, fill and number all hidden (`0x14017F100`). The tables' values are data the export
doesn't hold; the file has the digits 0–9 as groups **28–37** (DRIVE), **54–63** (FORM), **144–153**
(SUMMON) (their sprites run left to right through the digit strip, Checked (files)), which matches
main's "28 + n", "54 + n" (Assumption that the tables are exactly base + n).

#### 14.1.4 What plays when (Checked (decomp))
- **Fill** (`0x1401816C0`): KH2 doesn't scale the fill, it turns a mask over it. Mode 1: angle =
  points × k, and **200 × k when points = 100** (the full gauge); k = float `0x14071B158` (value not
  in the export; main's 0.0040 rad = 23°/100, which matches the form rule below: Assumption). Forms:
  angle = 23° × (ticks into the current bar / ticks per bar), the whole bar's 23° when full.
- **Number**: mode 1 = full bars (`+0x1B1`); in a form = ticks left / ticks per bar, i.e. the bar now
  draining (bars − 1 when full) (`0x140180490`). A change re-plays the number's group (its pop-in).
- **Bar gained** (number up, mode 1 only): system sound **0x20** (unless MAX is reached that frame)
  and group **22** spawned on the bar (`+0xB50` pool; the fill flashing white with streaks).
- **MAX**: when bars ≥ max, group **20** shown, with system sound **0x21** the first time after a
  gain; hidden below max and in forms. (Sounds through `0x1401DD170` → `0x1401DF790(2, id, ...)`, bank 2.)
- **Form change** (`0x140180900`, a new player object): if the bar's steady group changes, group
  **130** (a ring of sparkles) on the HP root `+0x1D40`, and the bar goes straight to its steady
  group (no slide-in): FORM bar 51 + fill 49 in; on the way back, `0x140180B00` sets bar 23 + fill 25
  and spawns group **53** on the bar (`+0x1330` pool).
- **Summons**: start: 130 on the root and the SUMMON bar, then (player flag `+0x6C8` bit 22) groups
  154 / 155 (`0x14017F210` → `0x140180E00`); end: group 156 and `0x140180B00`.
- **Lock** (mode 4, area bit 18 of `0x142A11400` at player creation, `0x140405000`): chains (18 / 19),
  fill and number hidden.
- **Slide-in**: only at the HUD's creation (`0x14017F9B0`: bar 21, steady 23 kept at `+0x3718`).
- **No Anti-specific piece**: the HUD only tests modes 0..4 and MP Charge, never a form id
  (Checked (decomp), every function above).

#### 14.1.5 When it shows (Checked (decomp) unless marked)
- KH2 has **no form-owned test** for the gauge: the player object's constructor `0x140405000` sets mode
  1 (3 if a summon is out, 4 in a locked area) whenever `0x14039A850()` is true, else leaves 0 (the
  whole Drive gauge hidden). `0x14039A850` is false only when `0x1409AB580` bit 23 is set and
  `0x1409AB594` bit 10 is clear (what those bits are: Assumption, likely the Roxas/prologue mode;
  its twin `0x14039A7E0` hides the MP pieces via status `+0x25C` bit 2).
- In events and menus the gauge goes with the **whole HUD**: `0x140168B50` (hides layers 3–6: player,
  partners, enemy) / `0x140168B70` (shows them), called from 0x140391F50, 0x140392E70, 0x140398810,
  0x140398BD0 and 0x1403AC280..0x1403AD2D0 (event / menu code; which event is which: Assumption);
  status `+0x25C` bit 0 also hides it (`0x1403C1C20` → `0x14016C1F0`). The gauge has no visibility
  rule of its own beyond mode 0.
- Where: the HP root (group 136) sits at (455, 355) in KH2's screen units; the bar group is at
  (−79, +24) from it, its quads spanning x −39..64, y −22..1 around that, so the DRIVE bar sits left
  of the portrait, its top a little below the portrait's centre; the number at the bar's attach point
  (52, −2) (the bar's right end, against the ring); MAX at (21, −10) on the bar (Checked (files)).
- Animation clock: KH2's sprite player `0x140192130` advances by `0x140717484` × the piece's speed each
  run (the global's value isn't in the export; Assumption: one frame per 60 Hz frame).

#### 14.1.6 The `gaug` sequence in zz0field.2dd (Checked (files))
- `field2d/us/zz0field.2dd` is a BAR of 8 entries: images (type 24) and sequences (type 25) `fiel`,
  `gaug`, `mapj`, `miss`. `gaug`'s IMGD is 512 × 256 (PS2); its HD image is the **second** PNG,
  `US_zz0field_2dd1.png`, **1024 × 512** (×2; the others are ×4: `fiel` 256² → 1024², so the scale must
  come from each image, not a constant).
- `gaug` SEQD: 172 sprites, 253 parts, 124 part groups, 1178 animations, 157 animation groups.
- The image holds: DRIVE bar (PS2 u 1–104, v 36–59), the grey fill (105–154, 36–59), MAX plate,
  "mp"/"HP" labels, FORM and SUMMON labels, the chains, the digit strip (u 258–435, v 2–25), HP/MP
  rings, and the bonus words (GET BONUS!, LEVEL UP!, ...).
- Drive groups (frames are the group's length; "once" = plays once):
  | Group | What | Length / loop |
  |---|---|---|
  | 18 / 19 | locked bar (with chains) appear / steady | 110 once / 150, loop 30 |
  | 20 | MAX (scales in, spins, colour cycle) | 120, loop 81 |
  | 21 / 23 | DRIVE bar appear (hidden 0–69, rises 50 units 70–110) / steady (a white flash 0–30) | 110 once / 30 |
  | 22 | bar gained (fill flashing, streaks) | 100 once |
  | 24 | DRIVE bar, short one-off | 20 once |
  | 25 | fill | 40 |
  | 26 | DRIVE bar pulsing white (MP Charge) | 120, loop 40–119 |
  | 27 | an effect with streaks | 175, loop 40–119 |
  | 28–37 | digits 0–9 (pop-in: turned −0.86 rad, half size, faded in over 20 frames; 1–7 add sparkles) | 300–1000 |
  | 38–40 | a thin bar and its ends (used by another gauge: Assumption) | |
  | 49 | FORM fill (pulsing) | 59 |
  | 50 / 51 / 52 | FORM bar appear / steady (pulsing) / short one-off | 110 once / 239, loop 120 / 20 once |
  | 53 | sparkles over the fill (the revert flash) | 132 once |
  | 54–63 | form digits 0–9 | 100–300 |
  | 130 / 131 | ring of sparkles (mode change) | 160 once / 140 once |
  | 136 | HP ring root (the parent) | 50 once |
  | 139 / 140 / 141 / 142 | SUMMON fill / bar appear / steady / one-off | |
  | 144–153 | summon digits | |
  | 154 / 155 / 156 | summon start / hold / end | 16 / 60 / 16 |

#### 14.1.7 Missing from main (KH2 has it, main lacked it)
1. **The fill's mask sweep** (`0x1401816C0`: a rotating mask, 200 × k when full); main cut the quad.
2. **Sounds 0x20 (bar gained) and 0x21 (MAX)**: main's code plays none.
3. **The revert flash, group 53 on the bar** (`0x140180B00`); main played only 130.
4. **Mode 0** (no gauge at all where `0x14039A850` is false) and **mode 4** (locked, chains 18/19,
   fill and number hidden): no KH1 equivalent yet (testing lacks mode 4, round 2 note).
5. **MP Charge's bar 26**: KH1 has no MP Charge (note only).
6. **Summon pieces** (139–156): KH1's summons stay on MP (note only).
7. The gauge's visibility is the whole HUD's (KH2 has no "only while controlling" rule, and no
   form-owned rule): main's s323 / s326 rules are the owner's.
8. The form-change ring is placed on the HP ring root (portrait), the revert flash on the bar; main put
   130 on "Sora's gauge" (same idea, Assumption it matched).

### 14.2. KH1's HUD (Steam KH1)

#### 14.2.1 Sora's HP/MP gauge task `0x14026DB00` (Checked (decomp))
- Registered by pointer at `0x140268F82` (priority 170001 group, `docs/kh1-frame-advance.md`).
- Five slots: bound flags `0x1428119E0` (u32 × 5), records `0x142811A00 + n × 0xDF0`. Bound by
  `0x14026E630(slot, actor)`, which writes record `+0` = the slot number (so Sora's slot 0 has kind 0)
  and `+0xA98` = 1; unbound by `0x140268A90`.
- **Event bit**: `0x142867364 & 8`. While set, the task draws **no gauge at all** (the slot loop is
  skipped); when it sets, the three party slots' alpha `+0xA9C` goes to 0; when it clears, alpha
  1.0 and `0x14026E080(0..2)`. Set by KH1's event player (`0x14023FBB0`, `0x1402417E0`), cleared at
  its end (`0x14023FA30`, `0x1402417E0`), per event by `0x140213150`.
- For each bound slot: kinds 0–2 → **`0x14026CF90(record)`** (the gauge body), then for kinds 0–2
  **`0x14027F470(record+8, frame, record+0xF0, alpha)`** (the **portrait**: uploads the face strip to
  GS page 0x3A00 and draws it as one sprite, frame `+0xAAC` for Sora); kind 3 / 4 have their own paths.
- Record fields seen: `+0` kind, `+8` portrait image, `+0x10` sprite list (6 records, count `+0x48`),
  `+0x50` list (4, count `+0x70`), `+0x78` (2, `+0x88`), `+0x90` (8, `+0xD0`), `+0xD8` texture
  descriptor `0x1427F14C0`, `+0xE0` `0x1427F1520` (its `+0x18` = TEX0 `0x1427F1538`, the HUD image at
  TBP 0x3600), `+0xF0` portrait place, `+0x108` place index, `+0x110` an offset sprite, `+0xA98` flag,
  `+0xA9C` **alpha**, `+0xAAC` portrait frame, `+0xDE0` "HD texture" flag. Lists filled by
  `0x14026E6D0` from index tables `0x1405270E0` (4..9), `0x140527138` (0..3), `0x140527168`,
  `0x140527180` into the sprite record block `0x142815FC0` (0xC0 each; the records come from a HUD
  layout file KH1 loads, so their rectangles aren't in the exe) (Checked (decomp + exe)).

#### 14.2.2 The gauge helper `0x14026D990(record, &pivot, &place)` (Checked (decomp))
Not a "gauge record helper" in general: it gives a slot's **pivot and place**. Kind 0: pivot from
`0x1405272B0` = (443, 316) (Checked (exe)), place = table `0x140527300`[record `+0x108`] + the wide
screen's shift `0x14232DCCC`, plus the rectangle of the sprite at `+0x110` (`0x14027E430`). 16 callers:
every gauge draw places from it.

#### 14.2.3 KH1's sprite path (Checked (decomp))
- **`0x14026CE90(sprite, texture descriptor, x, y, packet, alpha, keep)`**: KH1's HUD sprite draw.
  Builds the quad (`0x14027D470(sprite, quad 0x142821BD0, descriptor)`, which takes TEX0 from
  descriptor `+0x18`), moves its four corners by (x, y) × 16, scales the corner alphas by `alpha`,
  submits with `0x1402764B0(quad, 1, packet)`, returns the packet. 26 call sites, every HUD gauge
  piece. This is the call our pieces should use.
- Sprite record (0xC0): `+0` rect `x y w h` (animated when `+0x60 & 0x14 == 0x14`, keyframes
  `0x14027E430`), `+0x10` texture rect, `+0x20 + i × 0x10` corner colours (0x80 = as is), `+0x60` flags.
- Binding a texture before the draws, as `0x14026CF90` does: `0x140386660(&tex0, 0)` (sends TEX0),
  `0x14010D720(&tex0)` (KH1's texture map: entry for that TEX0), `0x140108910(entry)` (binds it).
- `0x140269190(record, packet)`: the gauge's frame (lists `+0x50` with texture `0x1427F14F0`, and
  `+0x10` with `+0xD8`); for kind 0 it **skips list `+0x10` index 5 when `+0xA98` is 0** (KH1's own
  per-piece hide; what `+0xA98` means: Assumption, perhaps "MP shown"). Then the HP / MP bars
  (`0x1402698C0`, `0x140269F50`).

#### 14.2.4 How KH1 loads 2D art (Checked (decomp))
- HUD images are **TIM2** blocks in a packed HUD file (`0x14280B640`), copied out and given a GS
  address (`0x140274030(img, tbp, cbp)`), then **`0x140274060(tim2)`** makes the texture entry:
  `0x14010BDF0(texture manager 0x1422C8FA8, pixels, w, h)` finds **the loaded file that holds the
  pixels** (`0x1400E2B20`) and that file's **HD entry** whose `+0x54` matches the pixels' place
  (list at file `+0x670`), and returns it; `0x14010DC50(tbp, 0, entry, psm, cbp)` maps it at that GS
  address (a map keyed by tbp/psm/cbp). The PC port draws the HD image, never the PS2 pixels.
- That is the same path `kh2dpx` already uses for KH2's effect textures (a buffer entered in KH1's
  loaded files with an HD entry per texture, `win/kh2dpx.rs` 42–52, Checked (code)). So KH2's `gaug`
  image, built once as a TIM2 header + its HD image, entered the same way, becomes a KH1 HUD texture
  through KH1's own `0x140274060` / `0x14010DC50`, at a GS address of its own (Assumption: a TBP past
  PS2 VRAM, as main s328's prize art did, can't clash).
- The other route: KH1's texture creator `0x1400F9DE0` (Steam; what `tint` uses through Epic
  `0xF9F80`), then pointing an entry at it (main swapped the HUD entry's `+0x70` around each quad).

#### 14.2.5 How KH1 hides its HUD (Checked (decomp) unless marked)
- Cutscenes / events: the event bit (2.1) skips every gauge draw; slot alpha 0 → 1 around it.
- Off the field: only bound slots draw; the world map / Gummi set the HUD up with no slot bound
  (main's notes, Assumption).
- Fades: `0x140268E30(slot, mode)` 0 fade out, 1 fade in, 2 shown, 3 hidden (main's notes for the
  mode meanings, Assumption; the function and its `× 0xDF0` record are Checked (exe)).
- Menus / pause: not checked whether the task draws under KH1's pause menu (Assumption either way;
  the frame-advance doc says HUD tasks run while the world is held).
- So a piece drawn **inside slot 0's draw, with slot 0's alpha**, follows every KH1 rule above with
  no visibility code of ours.

#### 14.2.6 KH1's "MP HP" label
One (or two) of Sora's frame sprites drawn by `0x140269190` through `0x14026CE90`. Main found them by
rectangle (MP [441,316,471,331], HP [463,316,486,331]) in lists `+0x50` / `+0x10` (Assumption: the
rectangles come from KH1's HUD layout file, not in the exe, and the index isn't known). Main hid them
by zeroing the record's size around every draw (per frame) and drew a black line for the MP bar's
bottom edge the plate had drawn (s322, s324).

#### 14.2.7 Hook points and first bytes (Checked (exe))
| Address | What | First bytes |
|---|---|---|
| `0x14026DB00` | HP/MP gauge task | `48 89 5C 24 18 55 56 57 41 56 41 57` |
| `0x14026CF90` | a party slot's gauge body (only caller: the task) | `48 89 5C 24 10 57 48 83 EC 60` |
| `0x140269190` | the gauge frame sprites | `48 8B C4 56 41 56 48 81 EC 88 00 00 00` |
| `0x14027F470` | the portrait | `40 55 56 41 56 48 83 EC 60` |
| `0x14026CE90` | sprite draw (to call) | `48 89 5C 24 08 57 48 83 EC 20 41 8B D8` |
| `0x14026D990` | slot place (to call) | `40 53 48 83 EC 40 48 8B 05 EB 70 2E 00` |
| `0x14027D470` | sprite → quad | `40 53 48 83 EC 20 4D 8B 40 18` |
| `0x1402764B0` | quad submit | `40 53 48 83 EC 20 45 33 D2 49 8B D8` |
| `0x140274060` | TIM2 → texture entry | `40 53 48 83 EC 20 41 B8 04 00 00 00` |
| `0x14010BDF0` | HD entry of a loaded file's pixels | `40 53 55 56 57 41 54 41 55 41 56 41 57` |
| `0x14010DC50` | map an entry at a GS address | `48 8B C4 56 57 41 56 48 83 EC 70` |
| `0x14010D720` | texture map lookup | `40 57 48 83 EC 30 48 C7 44 24 20 FE FF FF FF` |
| `0x140386660` | send TEX0 | `48 89 5C 24 08 57 48 83 EC 20 48 8B 01` |
| `0x140108910` | bind entry | `40 53 48 83 EC 20 48 8B D9` |
| `0x1400F9DE0` | GX texture creator | `40 55 53 56 57 41 54 41 55 41 56 41 57` |
| `0x14026E630` | bind a slot | `48 89 5C 24 08 48 89 74 24 10 57` |
| `0x1400E2B20` | loaded file of a pointer | `4C 8B 1D F9 84 0C 02` (RIP-relative: compare the opcode bytes only) |

### 14.3. Native design

**Draw.** One new hook: `0x14026CF90` (only the task calls it, once per bound party slot per HUD
frame). After KH1's own body draw, when the record is slot 0 (`0x142811A00`, kind 0) and the HUD
row is on: read the gauge (`win/drivegauge::get()`, the form's `InForm.gauge`), step our few sequence
players, bind our TEX0 the way the body draw does (`0x140386660` / `0x14010D720` / `0x140108910`), and
draw each quad with KH1's `0x14026CE90` from one sprite record of ours (rect, texture rect and corner
colours set from the frame; corners rotated/scaled for KH2's keys), with slot 0's alpha `+0xA9C`,
at slot 0's place (`0x14026D990`). The portrait (`0x14027F470`) is drawn after, as in KH2 where the ring
overlaps the bar's end. Nothing else runs per frame: no visibility code (KH1's event bit, slot binding
and alpha already decide), no label juggling (section 4 Q4), no logging.

**Per-frame cost.** Only while KH1 draws Sora's gauge: 3 save bytes + the form record read, ~5 sequence
players stepped (a table walk each), and 6–20 quads through KH1's own draw (bar 2, fill 1, number 1–7,
MAX up to 8; effects up to ~14 for a second). KH1's own Sora gauge draws about the same number of
sprites. Zero when the HUD isn't drawn (events, world map, unbound). Assumption: under 0.05 ms a frame
(not measured; a timer line in the loadprof style can measure it in the first version).

**Clock.** KH1's HUD animations step by `0x14233FBE0` (the half frame scale, `docs/kh1-frame-advance.md`
§HUD, Checked (doc)); stepping ours by KH1's HUD clock keeps it in step with KH1's pause / frame advance.
KH2's step `0x140717484` value is unknown (Assumption: 1 per 60 Hz frame; to confirm).

**Art built once.** `kh1-kh2-cache\kh2-hud-gauge.bin` (slot cache layout): manifest (format, mod
version, game build, `zz0field.2dd`'s hash and size, the HD PNG's hash, where they came from), then the
`gaug` SEQD bytes (188 KB) and the HD image as ready B8G8R8A8 pixels (1024 × 512 = 2 MB) with a TIM2
header for KH1's loader. Built from the player's own KH2 install (live or the Mods Manager backup, the
`kh2_data::Install` the slot cache already uses) at the loading screen, or lazily on a thread the first
time the gauge shows (tint's way). At start: one file read + check hash; the PNG is never decoded
again. Then entered in KH1's loaded files the `kh2dpx` way so KH1's own `0x140274060` makes the
texture (fallback: KH1's creator `0x1400F9DE0`).

**Fallback.** When the art isn't built (no KH2 file, a damaged cache), either no gauge and a log line,
or KH1 pieces: KH1's own HUD sprites (bar segments from its HUD image, coloured by corner colours) in
KH2's shapes through the same `0x14026CE90` path (main s319's look). Question 1.

### 14.7. Deprecated in main's version

KH2 art decoded at every start [DECODE] (→ built once); `takeover::script_has_sora`, `GAME_FRAME`
stall counting and the 0.2 s fade [CTRL]/[STEER] (→ KH1's own slot draw and alpha); the label's size
zeroed and restored around every draw (→ a data change once); KH1's HUD texture entry's `+0x70`
swapped around each quad (→ our own texture entry mapped at its own GS address); the gauge drawn after
the task with its own quad buffer `0x142866960` (→ inside slot 0's draw with KH1's).

### 14.8. Not found, and where I looked

- The values of KH2's number tables `0x14071B108 / 30 / E0`, the mask factor `0x14071B158`, the HUD
  root table `0x14071AFE0`, and the sprite clock `0x140717484`: data, not in the C export (no KH2 exe
  staged). Please stage the Steam KH2 exe if you want these read.
- What `0x1409AB580` bit 23 / `0x1409AB594` bit 10 (KH2's no-gauge mode) mean.
- Which event/menu callers of `0x140168B50 / 70` are which.
- KH1's "MP HP" label record and its rectangle (from the HUD layout file, not in the exe; the file
  wasn't searched).
- Whether KH1 draws the gauge task under its pause menu and in talks.
- KH1's portrait radius in HUD units (for Q2 (a)); main measured its sprites in game.
- Nothing checked in game; no traces this round.

### 14.9 Built

- **t936** `hudart.rs` (pure, tested) and `win/kh2hud.rs`: the `gaug` sequence and its HD picture built once
  into `kh1-kh2-cache\kh2-hud-gauge.bin` from the player's own `zz0field.2dd` (keyed by its PKG entry), on a
  thread at start; our own PNG decoder (its pixels equal Python's zlib decode of the real picture, Checked
  (files)). Nothing drawn yet.

- **t937**: the art's first start failed on the player's files ("not a PNG"): the KH2 archive reader didn't skip the
  16-byte padding after each encrypted HD asset, so `zz0field.2dd`'s second PNG was read 2 bytes early. Fixed in
  `kh2-formats` (`pkg_span`); notes in `docs/kh1-asset-cache.md`.

- **t938**: the gauge's KH1 texture. The picture as a DDS (B8G8R8A8, the uncompressed layout KH1's
  maker `0x1400FA160` takes) in an HD entry of a small file of ours in KH1's loaded files; KH1's lookup
  `0x14010C050` makes the texture with the PS2 size 512 × 256 (the IMGD's, as the HUD loader
  `0x140274060` does with a TIM2's), kept for good as the HUD loader keeps its own, mapped with
  `0x14010DC50` at a free TBP (0x3800 first, PSMCT32) so KH1's map lookup `0x14010D720` finds it by
  our TEX0 (`kh2hud::tex0`). Made once, on the game's thread, from Sora's frames; checked once a
  second that KH1's map still has it (Checked (decomp) for KH1's calls; nothing drawn yet).

- **t939**: the draw. `seqplay.rs` plays KH2's `gaug` sequence as KH2's player does (Steam KH2 `0x140192130`,
  `0x140192050`, `0x140192250`, `0x1401969A0`, `0x140196400`, `0x140196570`, `0x14018FD20`; float arguments read
  from the assembly). KH2's fill mask (`0x14017E580`, `0x1401816C0`, `0x14016D600`): a 50 × 120 box centred
  (−25, −60) from a pivot at (50, 100) on the bar, turned by points × 0.0040143 rad (× 200 full; in a form 23° /
  ticks per bar × ticks into the bar); it writes depth only, hiding the fill under it, so the fill is drawn as
  its part outside the box. `win/gaugehud.rs` hooks KH1's gauge body `0x14026CF90` and, after Sora's, draws bar
  23 / 51, fill 25 / 49, MAX 20, number 28 + n / 54 + n through KH1's `0x14027D470` / `0x1402764B0` with our
  texture, KH2's blends by KH1's ALPHA write; KH1's MP HP label's sprites sized 0 while it shows. Placed by main's
  measured rule (portrait + 6, the label's middle − 13.5 KH2 units + 0.3 × the portrait's radius + 1.3; 384 / 448
  × 1.34). Not yet: KH2's animations on a gain, MAX's and the gain's sounds, the form change ring and the revert
  flash (t940), the MP bar's bottom edge under the hidden label.

- **t940**: KH2's one-off pieces. A bar gained (out of a form) plays 22 on the bar; into or out of a form the ring
  130 on the gauge centre and the other bar and fill at once; out of it the flash 53 on the bar and the number
  played in again; MAX plays in on first reaching the maximum, hides in a form and comes back after without
  playing in (`0x1401816C0`, `0x140180900`, `0x140180B00`, `0x1401668D0`, Checked (decomp)). Sounds 0x20 / 0x21
  still to come.

- **t941**: the art's and the texture's outcome are always in the log (`log_line`, whatever the Loading category).
- **t942**: KH2's sounds: 0x20 a bar gained, 0x21 MAX reached by a gain (system bank 2, `se/se000.win32.scd`,
  `0x1401816C0` / `0x1401DD170`), cut with the other KH2 sounds and played through KH1's effect queue
  (`kh2sound::system`). Assumption: se000's id records 32 / 33.
- **t943**: the MP and HP bars' black bottom edge, which the hidden label drew: a 2-unit black line at the
  portrait's top from 0.57 of its width left of it to the label's right end (main's measured place, Assumption),
  drawn through KH1's sprite draw from an opaque black block of our picture.

## 15. Drive orbs as KH1 prizes (main-port round 4, row 3, 2026-10-30)

### 15.1 KH2 (Steam KH2, Checked (decomp) and (exe))
- **Prize kinds** (spawn table `0x1405C7390`, 9 × 0x10: kind, size, two ids): 0 / 1 HP small / big, 2 / 3 / 4
  munny big / middle / small, 5 / 6 MP small / big, **7 / 8 Drive small / big**.
- **Pickup** (`0x1403DCA50`, reward table `0x1405C7420`, 8 bytes a kind: a float, HP, MP, Drive, munny): Drive
  small **1**, big **3** (Checked (exe)), handed as a float to `0x1403A9CF0`: out of a form (mode 1) KH2's add of
  `(int)(value × the world factor)` points (× MP Charge's factor while it runs); in a form (modes 2 / 3) the
  form gauge + value × sstm `+0x70` (**12.0**, Checked (files)) ticks (not in Anti, `+0xDE0` = 12). Then
  `0x1403E8760(value)`: **Master Form gets the value as form EXP** (`+0xDE0` = 4, `0x1403E8380`).
- **Enemy drop** (`0x1403DC170`): the enemy's `przt` row (00battle; bsearch by id, row 0x48 if none), 0x18
  bytes: `+2..+0xA` nine counts by kind, `+0xC` three (item, chance) pairs. Each count is `ceil(count × (1 +
  the party's Jackpot share, +0x68))` (`0x1403ECEF0`, `0x1403ABA40`); munny (kinds 2..4) is split by the
  party's Drive Converter share (`+0x74`) into Drive orbs by `0x1405C7978` = [7, 8, 8, 8, 7, 7, 8, 7, 8]
  (munny big and middle → big orbs, small → small). A player flag (`+0x6C8` bit 25, `0x140415840`) turns every
  kind into its Drive orb (what sets it: not found).

### 15.2 KH1 (Steam KH1, Checked (decomp))
- Types 0..6 only: make `0x1402AB2A0`, enemy drop `0x1402ABC40 (counts[7], position, mode)`, pickup
  `0x1402AC080` (rewards for 0..6), row table `0x140528BE0` (7 × 0x14: height, pull, sound, ..., munny shown)
  read at seven places (`0x1402AC080` × 4, `0x1402AC4D0` × 2, `0x1402AC830`); the word after it
  (`0x140528C6C`) is in use, so rows 7 / 8 need a copy of the table the seven reads point at.
- KH1's own Jackpot applies its count factor in its drop functions: Drive orbs made through KH1's drop get
  KH1's Jackpot as KH2's get KH2's.
- The orbs' look: KH1's prize program `pppRyjMegaBirthPrize` (per-type sprite entries) not reread on Steam
  yet (main's notes were Epic): the first version's research.

### 15.3 Built
- **t944**: types 7 / 8 in a copy of KH1's prize row table (MP's rows) with KH1's six reads pointed at it; the
  prize program's entries 7 / 8 set from MP's orbs in KH2's gold before each draw.
- **t945**: after KH1's enemy drop, a roll of KH2's 94 normal-enemy `przt` rows (Checked (files)) makes the
  Drive orbs through KH1's own prize make, each count × KH1's Jackpot factor.
- **t946**: after KH1's pickup, a Drive orb gives KH2's 1 / 3: points to the bars, or 12 ticks a point to the
  form gauge in a form. Still to come: KH2's own orb art (`obj/PRIZE.mdlx`, answer 3 (a)); Master Form's orb EXP
  when Master is built.

### 15.4 KH2's orb art: how KH1 draws a prize's shape (research, 2026-10-30; Steam KH1, Checked (decomp))
- The prize program's draw `0x140340D10` takes each type's entry `shape` as an index into KH1's **global shape
  table** (`*(0x142E9D428 + 0x28)` = `0x142687E80`, 16 bytes a slot: the shape's animation, a word), and draws
  its current frame with `0x14031E210`. The table is filled when an effect DPD is entered (`0x1401EBA70`): each
  of the DPD's shapes (and textures, the 0x38-byte table at `0x142686280`) goes to the slot its own record's
  first short names, for the DPD's slots marked in use (bitmask at `+0xC0` / `+0xD0` of its entry).
- So KH2's Drive orb can be drawn by KH1's own prize program if its shape is in that table: a KH1 DPX built
  once from KH2's prize package (`obj/PRIZE.mdlx`'s effect package: kinds 7 / 8 its Drive orbs) by the same
  builder the effects use (`kh2_formats::kh1dpx`), entered in KH1's loaded files as `kh2dpx` enters its
  packages, with its shape in a slot KH1's own DPDs don't use; entries 7 / 8 then name that slot.
- Not found yet: how KH1 picks a DPD's slots (the shape record's first short vs. the in-use bits: whether a
  slot can be chosen freely and which slots KH1's own DPDs take); KH2's prize package's layout (which shape is
  the Drive orb, its frames). Next research step before the art version.

### 15.5 KH2's orb art: found and built (t947, t948)
- **Found** (Checked (files), the player's extracted `prize.dpx` and `obj/PRIZE.mdlx`): each shape's slot id is
  its record's first short, and KH1's prize DPD's are 0..3, the same as their places in its list; the in-use
  bits (`+0xC0` shapes, `+0xB0` textures) are by place. So a shape appended as the fifth, with id 4 and bit 4
  set, takes slot 4 (Assumption, to see in game: the global table has room for slot 4; KH1's effect DPDs
  already enter more shapes than that). Both games' prize particle data run program 203, its data at the
  program file `+0x48` (a word, then 12 bytes a kind: shape slot, RGBA, scale). KH2's kinds 7 / 8 are shape 3
  (24 frames) on texture record `0x36C0` (TBP `0x3000`, CBP `0x3E00`, 64 × 32 at 0, 0), colour `80808080`,
  scale 0.9 / 1.7. KH2's HD image of TBP `0x3000` is keyed by its first record's pixels (as KH1's `-320`).
- **Built**: t947 `kh2_formats::kh1dpx::prize_with_kh2` (KH1's file with KH2's shape and record appended at TBP
  `0x3860` / CBP `0x3D54`, still one slot of each; entries 7 / 8 KH2's); t948 `crate::prizeart` (the art and
  its HD images built once into `kh1-kh2-cache\kh2-prize-orbs.bin`) and `win/driveorbs.rs` (the prize
  effect's make, the call at `0x1402AE77B`, handed our file: copied into our handle region, entered in KH1's
  loaded files with the four HD images, relocated by KH1's `0x1401E0080`). MP's orbs in gold stay as the
  fallback.
- **Scale** (Assumption, to compare in game): KH2's numbers are used as they are, as the effects port keeps
  KH2's particle sizes. KH1 draws its MP orb at 0.4 × 16 = 6.4; KH2 its MP orb at 0.5 × 20 = 10 and its Drive
  orb at 0.9 × 24 = 21.6. So in KH1 the small Drive orb is 3.4 × KH1's small MP orb, where in KH2 it is 2.2 ×
  KH2's. If it looks too big next to KH1's orbs, the fix is one factor on entries 7 / 8.

## 16. Drive Recovery items (main-port round 5, row 4, 2026-10-30: research)

### 16.1 KH2 (Steam KH2)
- **The items** (Checked (files), `03system.bin` `item`): id `0x112` Drive Recovery and `0x113` High Drive
  Recovery, kind 1 (the menu-only kind, with the four Boosts `0x114..0x117`; Potion..Megalixir are kind 0), no
  battle command (Potion's is `0x17`), buy price 0 (never sold), sell 20 / 30. Their value byte (`+4`) is 3 and 9.
- **Using one** (Checked (decomp)): `0x1403C4B80` adds value × 100 points to the Drive gauge through the points
  add `0x1403C0AD0` (bars carry at 100, up to the maximum, §13.1.3): +3 bars and +9 bars (in practice a full
  gauge). In a form it adds to the bars, not to the form gauge.
- **When it can be used** (Checked (decomp)): `0x1403C3DA0`: while the bars (`+0x1B1`) are below the maximum
  (`+0x1B2`).
- **Where they come from in KH2**: chests and rewards (they are never sold). Not looked up item by item.

### 16.2 KH1 (Steam KH1)
- Inventory: a count per item id at `save + 0x499 + id`; item table in `btltbl.bin` (255 records of 0x14:
  name word, kind, help text), `docs/kh1-equipment.md`. Checked (decomp) there.
- **Battle item use** (Checked (decomp)): an item command makes a task `0x1402C2760 (user, target, item)`, which
  takes the item from the member's item slots (`+0x26..`) or the inventory (`0x140284CE0`), then
  `0x1402C2880` waits out the use motion and applies it with `0x1402C2B50` → `0x140297CA0`
  (the actor's message to the target, item | 0x10000).
- **Pause-menu item use: not found yet.** The Items page (`0x14030C270`, from the page table `0x14052B7C0`)
  and the functions around it are only declared in the KH1 decomp, not decompiled: the use has to be read
  from the exe's assembly next. Also not checked yet: what KH1's item records `0x91` / `0x92` hold
  (main used them as free ids; Assumption until read from the player's `btltbl.bin`).

### 16.3 Main's design (a spec only)
- The items at KH1 ids `0x91` / `0x92` (+3 / +9 bars, pause menu only), sold for 80 / 120 in the item shops
  of Traverse Town and Agrabah once a form is open, dropped by enemies (3 % / 0.5 %, more from MP orbs and
  big prize drops, × Lucky Strike) and placed in 17 chests (12 / 5). The sources are the owner's design,
  not KH2's.

### 16.4 Built (t949)
- Answers (2026-10-30): 1 (a) main's sources behind a Progression toggle, 2 (b) battle use as a toggle, 3 (a)
  ids `0x91` / `0x92` (new ids only if KH1 used them: it doesn't, Checked (files)).
- KH1's use (Steam, Checked (decomp)): the per-item use test `0x1402CD3A0 (item, member) -> bits` (bit 0: it
  would do something; the Items page enables the item and picks targets by it, `0x1402E8B20`, `0x1402E8B90`) and
  the per-item effect `0x1402CD780 (item, target) -> bits`, run by the Items page's use `0x14030F160` (bit `b`
  back: one taken, sound 0xE (7 from bit 4), message 0x99 + b; none: message 0xA0, "can't use") and by the
  battle item use (`0x1402C2B50` → `0x1402CD580`). Both switch on the item id, so KH1's own items are untouched.
- t949: records `0x91` / `0x92` take KH2's values after KH1's load (kind 12 menu only, or 0 with Drive > Recovery
  in Battle on), KH2's names and help; the two functions hooked for our ids only (Sora, the gauge below its
  maximum; KH2's value × 100 points). Code: `driveitems.rs`, `win/driveitems.rs`.
- Next: where they come from (answer 1 (a), main's design behind Progression > a toggle): the shops, then the
  enemy drops, then the chests, one version each.

### 16.5 The shops (t950)
- KH1 (Steam, Checked (decomp) and (exe)): `0x1403010A0 (mode, list, ..)` opens a shop; mode 0's list comes
  from the table at `0x1404FEE00` (0xD4 bytes a list: a count, up to 52 ids), picked by the shop's event
  (`0x1401CA610`); mode 2's from `0x1404FEAB0` (its lists 0..3 the weapon / accessory shops, 4.. the same item
  lists). The item lists: 0 `1 2 3 8E`; 1 `+ 68 6C 7D 7A`; 2 and 3 more accessories; 7 `1 2 3 8E 8F`. Which
  shop (world, story point) opens which list is in the events: not mapped (Assumption: Traverse Town's and the
  others' item shops, as main found).
- t950: with Progression > Drive Recovery on and a form open, a list with a Potion and a Tent is handed to KH1 as
  a copy with `0x91`, `0x92` after KH1's. Main's design, not KH2's (KH2 never sells them).

### 16.6 The drops (t951)
- KH1 (Steam, Checked (decomp) and (exe)): the enemy drop `0x1402ABC40` makes its 7 prize counts, then rolls three
  item drops `(chance, item)` of its own data: `random < 0x142D60FA8 × chance` (the factor 1 + 0.5 a Lucky
  Strike), made by `0x1402BFF00 (item, position, mode)`. KH1's random is `0x14031BA30` (0..1).
- t951: after KH1's drop, each of ours rolled the same way with main's chances (20+ prizes 15 % / 4 %, MP orbs
  among them 5 % / 0.75 %, else 3 % / 0.5 %), the prize counts taken × KH1's count factor as KH1 makes them.
  Main's design, not KH2's (KH2's enemies never drop them). Assumption: the dropped item's model, KH1's for the
  record's `+3` (6, the tents'), is fine.

### 16.7 The chests (t952)
- KH1 (Steam, Checked (decomp) and (exe)): the chest's Open handler `0x1402B58E0` reads KH1's chest table
  `0x140528D60` (a u16 a chest: bits 1..3 the kind, bits 4.. the item; `docs/kh1-chest.md`). Main's 17 chests
  (`driveitems::CHESTS`, with their worlds and rooms) all hold the values main recorded in the Steam exe.
- t952: those 17 values are ours (kind 0, `0x910` / `0x920`) with Progression > Drive Recovery on (from the first
  load of KH1's item table), KH1's again when it is off. Not gated on a form being open (a chest's contents
  shouldn't change with progress). Main's design, not KH2's.

## 17. KH2's Drive support abilities (main-port round 6, row 5, 2026-10-30: research)

### 17.1 KH2 (Steam KH2)
- **The items** (Checked (files), `03system.bin` `item` and `msg/us/sys.bar`): Damage Drive `0x18C` (AP 3), Drive
  Boost `0x18D` (3), Form Boost `0x18E` (5), Drive Converter `0x21C` (5), Damage Control `0x21E` (5). Help:
  Damage Drive "Restores the Drive Gauge every time damage is taken. The amount restored to the Drive Gauge is
  relative to the damage received."; Form Boost "Increases the duration of each Drive Form. Equip more to
  increase the effect."; Drive Converter "Increases the drop rate of drive orbs in place of munny. Equip more to
  increase the effect."; Damage Control "Halves damage taken at critical moments. Equip two to accelerate the
  effect."
- **The ability block** (`0x140401730`, each equipped copy, Checked (decomp)): Damage Drive `+0x58` += 50
  (starts 0); Form Boost `+0x44` ×= 0.8 (starts 1); Drive Converter `+0x74` ×= 0.5 (starts 1); Damage Control
  `+0x78` += sstm `+0x174` (starts 0; the value not read yet).
- **Damage Drive** (Checked (decomp), §13.1.6): out of a form, a loss of HP gives `damage / max HP × +0x58` points
  through the hit add (× the world factor, 1 here).
- **Form Boost** (Checked (decomp)): the form gauge's drain each step × `+0x44` (`0x1403A92C0`).
- **Drive Converter** (Checked (decomp), `0x1403DC170`): each munny kind's count (after Jackpot) keeps
  `(int)(count × the party's +0x74 product)`; the rest become Drive orbs by `0x1405C7978` (big and middle munny
  → big orbs, small → small).
- **Damage Control** (Checked (decomp) and (files), found after the round): the end of KH2's damage `0x1403C12F0`
  is × sstm `+0x178` (50) %, rounded up, while the target's HP ≤ max HP × `+0x78` (`+0x78` += sstm `+0x174` = 0.25 a
  copy): half damage from a quarter of max HP with one, from half with two.
- Drive Boost works only during MP Charge (`0x1403A9CF0`), which KH1 has no counterpart for.

### 17.2 KH1 and testing
- Testing's KH2 abilities live in the KH2 list (`kh2list`, `growth::MENU`), equipped with Sora's AP, every one
  given from the start (the owner's rule, t718). Main's ids 78..81 are other abilities on testing; the four get
  new ids in testing's list (settled, inventory "Save").
- KH1's HP change `0x1402A4920 (actor, delta)` (Damage Drive's place); KH1's enemy drop `0x1402ABC40` with its 7
  counts (munny types 0..2: 1 / 5 / 20) is already hooked by `driveorbs` (Drive Converter's place); the form
  gauge's drain is ours (`win/form.rs`, Form Boost's place).

### 17.3 Built (t953–t957)
- Answers (2026-10-30): 1 (a) KH2's Damage Control rule (found, above), 2 (a) Drive Boost after the main build,
  3 (a) KH2's Drive Converter numbers; "go ahead".
- t953 the four in the KH2 list (ids 89..92, KH2's names, help and AP; `growth.rs`); t954 Damage Drive and t957
  Damage Control on KH1's hit HP `0x1402BEEE0` (Sora's); t955 Form Boost on our form drain; t956 Drive Converter
  on KH1's enemy drop (a copy of the drop with KH2's munny share, the rest Drive orbs). Rules: `drivesupport.rs`;
  game side: `win/driveabilities.rs`.
- Open: a main save's support abilities (main's ids 78..81 are Flash Step..Finishing Leap here) can't be told
  from testing's own, so they aren't moved (inventory "Save").

## 18. Partners leave during a form (main-port round 7, row 11, 2026-10-30: research)

### 18.1 KH2 (Steam KH2, Checked (decomp))
- The change `0x14040CE60 (player, form)` calls `0x140400B20 (form)`: for each of the two party slots (`0x1403C3270`),
  by the member's id (`+0x4C`: 2 Donald, 3 Goofy): Valor (1) takes Goofy only, Wisdom (2) Donald only, Limit (3)
  no one, Master, Final and Anti (4..6) **whoever is in both slots** (a world's guest too). A guest in Goofy's place
  in Valor stays.
- Taking one (`0x140400990 (member, 15.0)`): a 15-frame fade out (`0x1403B7810` on `+0xA08`), the member flagged out
  (`+0x9B8` bit 4), and his id recorded (`0x142AE9788`, count `0x142AE9790`).
- Back: at the change's end with form 0 (the revert, `0x14040CC00` → `0x140400440`), each recorded member is made
  again near the player (`0x1403C2FC0`, offsets 100 / 200 / 300) with a 15-frame fade in, and the list cleared.
- Not checked yet: what KH2 does with a taken member's HP and status, and during events and room changes.

### 18.2 KH1 (Steam KH1, Checked (decomp))
- Party actors at `0x142D37280` (Sora, then two); a member's id at its data `+0x4C` (via `+0x130`).
- KH1's own "member out" script op `0x1402BB310`: actor flags `+0` |= `0x240`, a state change (`0x1402A10B0 (actor,
  10)`), the HUD slot set hidden by KH1's slot fade `0x140268E30 (slot, 3)` (the slot from `0x140285030 (member)`),
  then `0x14029B960`. Its counterpart (member back) and what each step does: to read next.
- Main's design (a spec only) cleared `0x8000` / `0x1` / `0x2` and set `0x200` / `0x40` itself each frame and
  brought them back for every scene; its scene detection is deprecated here.

### 18.3 Open before building
- Events that command a partner while he is out (main's Pegasus Cup freeze: a scene waited on Goofy): whether KH1's
  own out/back ops handle that, or the form must end before such an event (row 6).

### 18.4 Answers and built (t958)
- Answers (2026-10-30): 1 (a) partners back for an event that commands them, taken again after it while the form
  lasts; 2 (a) KH2's rule, guests too in Master / Final / Anti; plus "make a toggle in drives for parties being
  required to go into drive forms"; "go ahead".
- KH2's Drive command test (`0x140400740`, `0x1404005E0`, Checked (decomp)) uses the same slot rule as the change:
  a needed slot empty → the form can't be chosen, a needed partner unable to act → greyed.
- t958: Drive > Party Required (KH2 by default): KH2's test on KH1's party slots (`driveparty.rs`,
  `win/driveparty.rs`, `win/drive.rs`). Off: whoever is in the party, as before.
- Next (t959): the partners leaving and coming back. Still to read in KH1: the counterpart of the "member out" op
  `0x1402BB310` (KH1's revival `0x140289520` shows the steps back: `0x140292160`, the fades, `0x140297F30`, the HUD
  slot shown), and where KH1's events take hold of a partner (answer 1 (a)).

### 18.5 Built (t959)
- KH1 (Steam, Checked (decomp) and (exe)): KH1's flag stack (push `0x140292180`, pop `0x140292160`, 3 deep, depth
  `+0x388`) is how its game over freezes everyone and its revival (`0x140289520`) brings them back with the model
  fade `0x1402A0F40` and the HUD slot fade `0x140268E30 (slot, 1)`.
- t959: a taken partner fades out (15 frames, HUD slot too), then his flags are pushed without acting, model and
  shadow, with `0x240`; back on every revert and for a cutscene (scene flag `0x142382594` bit 0), taken again after
  it while the form lasts; a new room's actors taken again. Code: `win/driveparty.rs`, called from `win/form.rs`.
- Open (Assumption): his place on return (KH1's AI); whether every scene that commands a partner sets the scene flag.

## 19. Forced reverts (main-port round 8, row 6, 2026-10-30: research)

### 19.1 KH2 (Steam KH2, Checked (decomp))
- **Room jumps carry a recovery kind.** KH2's map jump `0x140152990 (.., recovery)` hands it to `0x1403A5650 (kind)` when the
  new area starts (`0x140152A90`). Kinds 2 / 10 / 12, and 1 (the world map) when a form is on, end the form at once and
  refill the Drive bars (`0x1403A5790`: form id `0x1409ACDD4` = 0, `0x1403C2020`) and refill every member's HP and MP;
  kinds 4 / 6 (and 5) do the revert and bar refill only while a form is on. Which jumps carry which kind is per-jump data
  in KH2's areas.
- **Events can revert by script**: event function `0x14042FEA0` starts the revert task (`0x14040CD40`); `0x14042FE80`
  changes form (no callers in the decomp: both are entries of the event function table). No bar refill there.
- **Gauge out / Revert command**: the form state `0x140405320` / `0x140404A40` (no refill, §13).
- **Battle end**: nothing reverts a form at a battle's end (no caller found). A KO is a game over.

### 19.2 KH1 and testing
- Done: leaving a world in a form (t934, KH2's world-map kind 1: revert + bars, HP, MP); KO (`form::native_revert`).
- KH1 has no recovery kind on its room jumps, so KH2's per-jump list has no KH1 counterpart.
- Conflict: row 11 answer 1 (a) keeps a form through a cutscene (partners back for it); main's row 6 reverts on scenes.

### 19.3 Main's design (a spec only)
- Every revert outside the player's control (KO, new world, scenes, battle end, the Coliseum arena) refills every bar and
  brings the partners back; the Revert command and the gauge running out don't.

### 19.4 Answers and built (t961)
- Answers (2026-10-30): 1 (a) a cutscene keeps the form; 2 (a) a battle's end doesn't revert; 3 (a) the cup's start reverts
  with the bars full and the partners back, the rounds carry the form (row 7); 4 (b) a KO refills nothing. Row 6 is
  done by t934 (a world left) and t959 (the partners back on any revert).
- t961 (row 7): into a Coliseum arena room (world 0xB, rooms 2 and up) from its gates or lobby in a form: bars refilled
  and the form reverted; no time drain while one of KH1's cutscenes runs (a round's victory scene). Code:
  `win/driveparty.rs` (`cup_start`, `scene`), `win/form.rs`.

## 20. Talking, examining and KH1's limits in a form (main-port round 9, rows 18 and 19, 2026-10-30: research)

### 20.1 KH2 (Checked (files): `03system.bin` `cmd`, DisableForm byte at +44 of the 48-byte rows)
- Talk (0xA, 0x1F, 0x279, 0x282), Examine (0xE, 0x1E, 0x102, 0x103, 0x19D, 0x25C), Open (0x20) and Save (0x37, 0x1CD)
  have DisableForm 0: KH2 lets Sora use them in every form.
- KH2's limit commands (execute 81: 0x4A, 0x52, 0x8B, 0xFF, 0x11A, 0x152, ...) also have DisableForm 0. KH2 has no solo
  limits for Sora: each is a party limit with a partner (00battle `limt`), and a form takes the partners away, so in KH2
  no limit is offered in Valor, Wisdom, Master, Final or Anti (Limit Form's moves are the form's own). Assumption: the
  Limit command is hidden by the missing partner and not by another form check; the decomp check of the Limit entry's
  availability is still to do.

### 20.2 KH1 (Steam KH1, Checked (decomp) and (files): `btltbl.bin` command records at file +0xA58, 0x10 bytes each)
- The context scan `0x1402948F0` offers, by the object's interaction kind (`+0x5F`): 1 Talk (command 4) on slot 0, out
  of battle; 2 slot 3 (Examine; Assumption: command 5, the decompile drops the argument); 3 the object's own handler
  (`0x140297CA0`); 4 slot 0. The chest handler `0x1402C5D50` offers 0x1C / 0x1D on slot 3 and 0x1E on slot 0.
- The ground branch `0x1402A3DB0` runs them: 4 and 5 start the object's event (`0x14028DD80` mode 0 / 4); 0x1C plays the
  open motion by the height to the object; 0x37 opens a chest (the chest table `0x140528D60`, `0x1402B58E0`); any id it
  doesn't know turns Sora and calls the object's handler (save points and other objects; Assumption: a save point is
  one of these).
- KH1's limits: the offer `0x14028D0F0` puts one in the Triangle slot (slot 3) at the end of the scan; ids 0x4B Sonic
  Blade, 0x57 Ars Arcanum, 0x5E Strike Raid, 0x5A Ragnarok, 0x52 Trinity Limit, follow-ups 0x4D / 0x4E / 0x58 / 0x59 /
  0x5B / 0x5F / 0x60 (`docs/kh1-kh2-limits.md` §3).

### 20.3 Testing now (Checked (code))
- Valor (`combat.rs` `command_step`) refuses every command but 0 / 1 / 2 / item and `NOT_SORAS`. That refuses Talk (4),
  Examine (5), 0x1C, chest Open (0x37) and every object command on the default path. `NOT_SORAS` lets 0x1D / 0x1E
  through, and also the limit ids (0x4B, 0x52, 0x57..0x5B, 0x5E..0x60): a limit can start in Valor and its hits are then
  refused by the attack row check (`combat.rs` `exec`). Assumption: this is what the player sees as "Valor refuses
  limits"; not checked in game.
- Wisdom refuses nothing of these: KH1's limits are offered and run in Wisdom.

### 20.4 Plan (one version each, after a go-ahead)
- Row 18: Valor's refusal keeps out only KH1's attack moves (the limit ids and their follow-ups, Counterattack 99), so
  Talk, Examine, Open, Save and object commands run on KH1's own code in every form, as KH2.
- Row 19: in any form KH1's limit offer `0x14028D0F0` offers nothing (one check at its start, KH1's own call; no
  per-frame code of ours), and a limit id chosen in a form is refused as KH1 refuses an unusable command.

### 20.5 Answers and built (t962, t963)
- Answers (2026-10-30): 1 (a) talking, examining, chests and save points in every form, as KH2; 2 (b) KH1's limits in forms
  as a Config toggle.
- t962 (row 18): Valor's refusal is `formcmds::valor_allows`: Attack, magic, items, the commands that don't move Sora, Talk 4,
  Examine 5, 0x1C / 0x1D / 0x1E, a chest's Open 0x37 and every object command (KH1's ids up to 0x7C its own code doesn't run;
  0x62 kept out, Assumption). KH1's limits are now refused at the command in Valor (no start with its hits refused).
- t963 (row 19): Drive > Limits in Forms (KH1 | KH2, ini `drive_limits`, KH2 by default). KH2: entering a form writes `ret`
  over KH1's limit offer `0x14028D0F0` (first bytes `48 81 EC 98 00 00 00`), the revert puts it back; a limit id still chosen
  in a form is refused (sound 4). KH1: the offer as KH1 has it (Wisdom runs them; Valor still refuses them). Code:
  `formcmds.rs`, `win/formlimits.rs`, `win/combat.rs`, `win/form.rs`.
- Open: the KH2 decomp check that only the missing partner hides KH2's Limit command in a form; save points' command id (seen
  in game: a refusal would show in the log as "refused: not in Valor Form").

## 21. The dim during a Drive change (main-port round 10, row 21, 2026-10-30: research)

### 21.1 KH2 (Steam KH2, Checked (decomp))
- The change task (`0x14040CAA0` starts it, its step `0x14040D000`), for forms 1..6: a scene colour entry (`0x1403F6F80(0)`,
  colour (48, 48, 48), `0x1403F76E0`) whose strength goes 0 → 1 over 12 frames; a wait of 30 frames; it holds until the
  change is done; then 1 → 0 over 15 frames (`0x1403F76A0`).
- What the entry does (`0x1403F7040`'s loop, then `0x1401626F0(r, g, b)`): the scene's light colour and fog colour are
  multiplied by colour / 128 = 0.375. It dims the lighting (map and models) to 37.5%; it is not a screen overlay. Additive
  effects are not lit, so the change's glow stays bright (Assumption). The HUD isn't lit (Assumption).
- With it, a second screen effect (`0x142AE7C88`, `0x1403F7850` / `0x1403F7880`: centred at PS2 (256, 208), on at once, off
  over 20 frames at the end, `0x1403F7830`). What it draws is not looked up (Assumption: a radial blur).

### 21.2 KH1 (Steam KH1, Checked (decomp) and (exe))
- KH1's light list: 31 slots of 0x40 bytes at `0x142D5B030` (+0 type, +4 flags, +8 id, +0x30 an RGBA float colour, 1.0
  neutral). `0x14029D880 (type, flags, id)` takes a slot, `0x14029DC20` frees it, `0x14029E0D0 (light, colour, ticks,
  when done)` fades its colour with a task of KH1's (`0x14029E710`, the frame's tick delta `0x14233FBDC`).
- Every type-1 colour is multiplied into the ambient each tick (`0x14029E790` → `0x14018C9E0`, the map through
  `0x1404E3430` and `0x140198B60`'s Ambient) and into each actor's light (`0x14029E5C0`, actor `+0x430`). Flag 1 = map only,
  2 = actors only, 8 = freed at a room change (`0x14029D8D0`), 0x10 off (Assumption, from the masks).
- KH1 already dims its field this way: `0x1401CF800` (a type-1 light, flags 9) faded to (0.6, 0.8, 0.6) over 30 ticks by
  `0x1401D8450`, and back by `0x1401CD1E0`; scripts have their own calls (`0x1402BC190` ...).
- KH1's fog colour (`0x14233FE60..`, copied by `0x14018CC10`) isn't on the light list.
- KH1's tick: the light fade counts KH1's ticks (`0x14018D1F0` sets the delta to the ticks since the last frame).
  Assumption: 60 a second as KH2's frames; to see in game.

### 21.3 Plan (one version, after a go-ahead)
- At the change's start: a type-1 light of ours (flags 8, our id) faded to (0.375, 0.375, 0.375) over 12 ticks; when the
  form is on and at least 30 ticks have passed, faded back to 1.0 over 15 ticks and freed by KH1 when done. KH1's own task
  steps it; nothing of ours each frame.

### 21.4 Answers and built (t964)
- Answers (2026-10-30): 1 (a) the fog too; 2 (a) KH2's second screen effect researched for its own version.
- KH1's fog (Checked (decomp) and (exe)): set per room (`0x140180D10`, the room's own kept at `0x14233FE20..`), copied to the
  renderer every frame (`0x14018CC10`), and faded by KH1's own fog fade, the one its event scripts start (`0x1401B0880`; back
  to the room's `0x1401B0A00`): block `0x142382A50` (+0 length, +4 time, +0x10.. start, +0x40.. target), on with
  `0x142382594` bit 1, stepped by `0x1401B3840` from the field's script task `0x1401A2320`. A script's length is its frames
  × 2 (Checked (exe)), so KH1's ticks are 60 a second if its scripts count 30 (Assumption).
- t964: `win/drivedim.rs`. The change's start: a type-1 light of ours (flags 8, id 0x4B32) faded to 0.375 over 12 ticks, and
  KH1's fog fade to the colour × 48 / 128 over 12. The arrival, a revert or a KO: back over 15, KH1 freeing the light, the
  fog back to what it was (the new room's own after a room change). A script's fog fade running is left alone.

### 21.5 KH2's second effect (research for its own version)
- `0x1403F7850` / `0x1403F7880` (Checked (decomp)): KH2's full-screen feedback draw `0x14015F250` turned on: the last frame
  drawn back over the new one as a full-screen sprite at alpha 0x70 (of 0x80), offset by (2, 2) PS2 pixels, no spin
  (`0x14015F1A0` 0), from the change's start; its strength fades out over 20 frames when the change ends. What it looks
  like: a trail / smear of the moving image (Assumption).
- KH1 (Checked (decomp)): no such draw found (none of KH2's constants, no frame-copy sprite). KH1's renderer would need a new
  pass that copies the frame and draws it back: question in chat.

### 21.6 Built (t965): KH2's blur on KH1's own
- Answer (2026-10-30): (a). KH1 has the same feedback blur (Checked (decomp) and (exe)): `0x14017B190 (mode)` → `0x140185C40`,
  called from KH1's draw loop (`0x140170FF0`) while the mode `0x14233FBE8` is 1 or 4. It draws the last frame (prevFrameTex,
  `0x140108300`, copied at every frame end by `0x1401030C0`) as a strip over the new one, blended (Cs − Cd)·As + Cd, colour
  `0x14233FDE0..DEC` (0x80 = 1.0), offset `0x14233FE00` / `FE04` (PS2 pixels), spin `0x14233FDF8` over `0x14233FDFC` frames.
  KH1's own event sets alpha 0x70 (`0x14023F900`), as KH2. So no renderer change: §21.5's "KH1 has no draw" was wrong.
- t965 (`win/drivedim.rs`): 30 frames into the change, mode 1 with (0x80, 0x80, 0x80, 0x70), offset (2, 2), no spin; once the
  form arrives the alpha goes down over 20 frames on the form's own frame, then mode 0. A change cut short: off at once. Left
  alone while one of KH1's events has its own blur on.
- Open: KH1's offset direction against KH2's; KH2's fade curve (linear assumed).

## 22. The second Keyblade's slot: stats, in use, the empty slot (main-port round 11, row 20, 2026-10-30: research)

### 22.1 KH2 (Steam KH2, Checked (decomp))
- Each form keeps its own weapon in Sora's record (`0x142A252F0 + 0x16 + form × 2`, read by `0x140405190 (form)`; form 0 is
  Sora's own at `+0x10`). The change picks it (`0x14040CAA0`, `0x140405190`), and the weapon resolver `0x1403DFEB0`, case
  0x23B: the form's weapon if its slot holds one, **else Sora's own Keyblade**.
- Whether a Keyblade in a form's slot is taken off the lists for Sora and the other forms: not looked up (Assumption: yes,
  one copy per slot).

### 22.2 KH1 and testing (Steam KH1, Checked (decomp) and (code))
- Testing has the V: row (`win/equipmenu.rs`, save `+0x488`, `equip::valor_keyblade`). With the slot empty or unusable it
  takes the free Keyblade with the most Strength, and Valor can't start when none is free (a second copy is needed).
- KH1's free count `0x1402E8770 (item)` (Steam; Epic `0x1402E6330`): the copies owned minus those equipped across the ten
  members. KH1 calls it from its menus and the shop's sell list (9 callers). Testing only reads it (`equip::free`), so a
  Keyblade in the V: slot can still be sold or equipped on Sora.
- KH1's weapon compare box `0x1403050D0` (Steam; reads the item being replaced, `0x142E9A07C`): testing doesn't hook it, so
  during a V: pick it shows Sora's own figures (main's s262).
- KH1's dream weapons (Dream Sword / Shield / Rod, `0x52..0x54`) and the wooden sword aren't Keyblades in testing
  (`equip::NOT_KEYBLADES`); KH1 locks the weapon row while one is held (`0x1402E6C60`).

### 22.3 Main's design (a spec only)
- s262: the compare box shows Valor's figures during a V: pick. s286: a slotted Keyblade holds a copy (Valor first, then
  Final), taken off `equip::free`, the other form's list, Sora's list and KH1's free count (the sell list). s306: Valor's
  empty slot is the Dream Sword, holding no copy, offered at the top of V:'s list.

### 22.4 Plan (one version each, after a go-ahead)
- The compare box: KH1's own box, hooked at `0x1403050D0`, shows Valor's figures with the slot's Keyblade as "now" and the
  hovered one as "new" (valorstats' rule).
- In use: KH1's free count hooked to take the slot's copy off (Sora's list, the sell list, every KH1 caller).
- The empty slot: by the answer below.

### 22.5 Answers, KH2's in-use rule, built (t966–t968)
- Answers (2026-10-30): 1 (a) KH2's empty slot; 2 "check with the decomp and how to implement it".
- KH2 (Steam KH2, Checked (decomp)): save `0x1409A98B0`; form records at save `+0x32F4`, 10 of 0x38 bytes (`0x1403A1410`),
  `+0` the Keyblade (get `0x1402FC630`, set `0x1402FC7C0`), `+2` the level; only forms 1 / 4 / 5 take one (`0x1402FC860`).
  Equipping returns the old Keyblade to the stock (`0x1403C3F00`) and takes the new one out (`0x1403C49C0` → `0x1403C4A40`), as
  for Sora's own weapon; the count (`0x1403C4720`) is the stock itself, so a slotted copy is gone from Sora's list and the
  other forms'. The sell list: Assumption (the shop is the overlay `ovl_shop.rel`, not in the decomp). A form gained fills an
  empty slot with the first Keyblade with stock in item-table order (`0x1403C3FE0`), else 0.
- How on KH1 (native): KH1's stock counts every copy and its free count `0x1402E8770` subtracts the members' equipped copies
  when read, so the slot's copy is subtracted there too (hooked), not taken out of KH1's stock (KH1's saves and shops count
  equipped copies).
- t966: `equip::valor_holds` / `valor_keyblade`: the slot's Keyblade while a copy is free of the members, else Sora's own;
  `valor::keyblade` empties a slot with no copy; Valor no longer needs a spare Keyblade.
- t967: `win/valorslot.rs`, KH1's free count less the slot's copy (not during a Valor pick).
- t968: `win/valorslot.rs`, KH1's compare box `0x1403050D0` with Valor's "now" during a Valor pick.
- Open: the sell list in game; KH2's own list for an empty form slot (what KH2 shows there).

## 23. Limit Form's limits on KH1's limit path (main-port round 12, answer 1 (a), 2026-10-30)

Full research: `docs/kh1-forms-research-r12.md`. Tags as elsewhere.

### 23.1 KH2 (Steam KH2, Checked (decomp) and (files))
- Limit Form's four limits are one script, `limi` (BAR type 3 of `obj/P_EX100_KH1F.mdlx`), not the limit bars (those hold
  only the combo/gauge art, used by party limits alone). Trigger 0x16 sets the limit up, 0x17 takes each follow-up command.
- The start (`0x1403D8B40`): the limit object (`0x142A24CC0`, one at a time), the MP paid at the start (`0x1403D3CF0`, below 0
  = MP 0 and MP Charge), the player put in state 0 and a thread on his own script.
- Sonic Rave plays 211 (K400 start, invulnerable all through), 212 (K401 Rave, invulnerable 0..12, script ranges 1 / 2 at
  16..46 for the Rave / Finish follow-ups, ids 16 / 17) and 213 (K402 Finish, invulnerable 0..48). Its hits are its
  Keyblade's: ATKP 1878 (Rave) and 1879 (Finish), 5 HP drained a hit. The light: (64, 64, 80) over 16 frames.
- Range ids 16 / 17 / 18 set bits 0 / 1 / 2 of the motion block's `+0x154` (`0x14040D810`); the script's
  `trap_obj_motion_check_range` reads the object's `+0x2AC` bit n − 1 (`0x14042D1E0`): the same byte if the block is the
  object's `+0x158`, as trap 1:11 passes it (Assumption: no direct write found).

### 23.2 KH1 (Steam KH1, Checked (decomp) and (exe))
- A limit's start `0x14028D800`: bit 16 of `0x142867364` set (23 cleared), the module started (`0x140397A50`: its effect
  data at `0x142EFB0E0`), MP paid (`0x1402A5B60`). Each module: `0x140397990 (colour)` (running flag `0x142EFB0D0` = 1, a
  type-1 light `0x14029D880 (1, 9, 0)` at `0x142EFB0E8`, faded in over 30 ticks), then `0x140397970` adds the end task
  `0x1403979E0` (priority 52000).
- The end task waits while `0x1402AE0C0 (data)` finds a live element of the limit's data (table `0x142D678B0`, 256 × 0x3C),
  then fades the light back over 30 ticks, frees it and clears the running flag; KH1's limit step `0x14028D620` sets bit 23
  and clears 16 / 23 the frame after.
- Bit 16's readers: the battle flag (`0x1402A2F50`), the music (`0x1401A1A10` / `0x1401A4D00`), an action check
  (`0x1402A5FC0`), the load reset (`0x14028D4A0`), KH1's limit offer.

### 23.3 Built
- t978: Sonic Rave's motions as Limit Form rows of ours (`kh1rows`, `SONIC_RAVE_MOTIONS`), `WindowKind::ScriptRange`.
- t979: Sonic Rave started from the Limit list on KH1's limit path (`win/limitform.rs`): KH1's bit 16, running flag and
  limit light (KH2's colour and 16 ticks), KH1's end task added as its modules add it, our mark in KH1's limit data, and KH1's
  element check `0x1402AE0C0` hooked to answer for the mark: alive while Sonic Rave's moves play. KH1 ends it and fades the
  light back itself. The first move on its row by KH1's row start `0x1402A3980` (its attack object), KH1's idle queued after.
  MP: KH2's 60 of 100 as a share of KH1's maximum (`mpcost::pay_limit`), or all of it when short (answer 4 (a)); refused at
  0 MP (KH2 greys a limit only while MP Charge runs, which is when its MP is 0). Rows of ours with KH2's own sounds lose the
  template's voice and effect bytes.
- t980: KH2's `limi` runs the limit as a KH1 script process (`formmagic`'s runtime, `kh2_magic::Magic::limit`: trigger
  0x16 with a limit object of ours). Its calls: `trap_limit_player` (the owner) / `_friend` (none), `_start_command` (698),
  `_light` / `trap_light_set_color` (KH1's limit light faded to the colour / 128), `_time` 0 (its partner thread ends at
  once), `_reference` 0, `trap_area_world` -1 (KH2's world 13, Timeless River, isn't in KH1), `trap_obj_get_entry_id` 2397
  for Sora in Limit Form (Assumption: one entry for every world's), `trap_obj_hook` (a thread of the cast's on a KH1 child
  process; a new hook ends the last), `trap_sysobj_motion_start` (KH1's row start for an attack row of ours, else KH1's motion
  request) and `_push` (KH1's queue; motion 0 = KH1's idle), `trap_obj_wish_dir` (KH1's wanted facing `+0x150`),
  `trap_obj_stop_end_all` / `trap_puts` nothing. KH1's end task now waits on the script, not on the moves. The script's
  view of Sora's motion takes the row KH1 was asked for before KH1 applies it.
- t981: the follow-ups. `trap_obj_motion_check_range` reads our slot's clip's script ranges at KH1's time
  (`kh2slots::now_playing`); `trap_command_override` puts our record (0x85 "Rave" 699, 0x86 "Finish" 700, words 0x1F7 /
  0x1F6, argument 0x28 + j) in KH1's Triangle slot that frame (`0x140293000 (3, record)`, as KH1's limit offer); chosen,
  KH1's command step hands it to the script as KH2's trigger 0x17 with the command (`Magic::limit_command`: a context of
  its own on the limit's work memory, run to its end at once). The script counts the Raves (5) and then offers Finish.
- t982: KH2's HP drain: a hit of Sora's whose KH2 ATKP has an HP drain (`+0x2F`; Sonic Rave's 5) gives him that many HP
  through KH1's HP change (`probe::hit` → `limitform::hp_drain`). Assumption: KH2's owner method `+0xE8` adds HP.
- t983: KH2's Limit EXP, +1 per finishing follow-up used (686 / 700 / 703 / 706, `0x1403E8730`), when KH1's command
  step hands it to the script (`formexp::follow_up`).
- t984: fix, `trap_limit_start_thread`'s arguments are (pc, arg).
- t985: Last Arcanum (701). KH2 (Checked (files)): `limi` lights (80, 64, 64) over 16, starts a thread binding effect
  elements 35 / 36 on Sora (`trap_obj_effect_start_bind`) until the limit's moves end, and hooks 221 (K410) then 0. Its
  follow-ups aren't the script's: each move's range 20 offers the next command (K410..K414: 702 Bash, K415: 703 The End),
  which KH2's range step puts in the reaction slot each frame (`0x14040D810` case 0x14, Checked (decomp)). Bash plays
  221 + the Bashes so far (222..226), The End 227. Built: rows of ours for 221..227, `WindowKind::Command` (range 20),
  `limitform::window_offers` (each frame a limit runs, KH1's Triangle slot), records 0x87 / 0x88 (words 0x1F5 / 0x1F4),
  `trap_obj_effect_start_bind`, `trap_obj_is_dead`, `trap_effect_loop_end_kill`.
- t986: Infinity (683), its moves. KH2 (Checked (files) and (decomp)): `limi` lights (64, 80, 64), hooks K430 (241), then
  while each spin plays starts the next by PTYA rows 52..56 (`trap_obj_party_attack`, `0x14053EF20`: the row's motion), up
  to the hover K435 (246), whose range 20 offers Shoot 684, Impact 685 and Giga Impact 686 in turn; its frame trigger 9 starts
  the charge effect (`trap_obj_motion_check_trigger`, frame triggers 9..12 → `+0x2AD`, `0x14040DD70`). Shoot / Impact: K436
  (247) with a push back of 50 / 100, Giga Impact K437 (248) with 200, then the fall. Built: rows of ours for 241..248, records
  0x89..0x8B (words 0x1F3..0x1F1), `trap_obj_party_attack`, `trap_obj_motion_check_trigger`, `trap_obj_effect_unbind`,
  `trap_obj_set_movement` (nothing: the motions carry the movement, Assumption).
- t987: Infinity's shots. KH2 (Checked (decomp) and (files)): frame trigger 7 sends the object's script trigger 0x11 with
  (object, p0, p1) (`0x14040DD70` case 7); K436 / K437 carry (100, 0) at frame 2; `limi`'s 0x11 with 100 starts a thread a
  shot, as many as the follow-up stored (Shoot 10, Impact 17, Giga Impact 25), spread evenly round Sora's facing
  (`trap_vector_rot`). Each shot takes the lock-on target, else searches up to 10 enemies within 0..1000 and picks one at
  random, homes on it and makes its attack. Built: `EventKind::ObjectTrigger` (frame trigger 7), `Magic::limit_object_trigger`
  (run at once, as the follow-ups), `limitform::object_triggers` (each frame a limit runs), and the calls
  `trap_vector_normalize` / `_inter` / `_outer_product` / `_rot`, `trap_random_get` / `_getf` / `_range`,
  `trap_target_set_obj` / `_clear` / `_get_group` / `_dist`, `trap_btlobj_lockon_target`, `trap_target_searcher_init` /
  `_search` / `_get_old`. Assumptions: the random calls' ranges (not in the decomp), the rotation's sense, the search's
  direction and flag tests left out (nearest first).
- t988: Strike Raid (704), its moves. KH2 (Checked (files) and (decomp)): `limi` lights (80, 64, 80), hooks K420
  (231), throws with K421 (232) / K422 (233, Judgment), catches with K423 (234), whose script ranges 1 / 2 (0..38) offer Raid
  705 (3 left) and Judgment 706. Built: rows of ours for 231..234, records 0x8C / 0x8D (words 0x1F0 / 0x1EF), and the throw's
  weapon calls answering "no weapon object, still attached" (`trap_party_get_weapon` 0, `trap_obj_is_attach` true), so the
  Keyblade stays in his hand and the throws hit nothing until the thrown Keyblade is built.
- t990: the Limit list greyed (all four, KH1's own grey bits) while a limit runs (ours or KH1's) or with no MP at all,
  KH2's state 5 (`0x1403D88E0`); under the cost they stay usable.
- t991: the revert waits for a running limit (its gauge or MP out, as KH2's revert comes only from the free state,
  `0x140404A40`), and the Revert command is refused during one. A knock-out still reverts at once (the limit's script ends
  with the form, `formmagic::end_all`).
- t992: Strike Raid's thrown Keyblade, KH1's way (KH1's own Strike Raid throws it on Sora's own weapon bones, its state
  turning his cull off, Checked (decomp)): the script's weapon is his Keyblade; detached, his weapon bones are drawn whole
  where the script flies it and turned by its angles; back in his hand at the catch, when a throw is cut short, or at the
  limit's end. KH2's weapon motion 235 (its spin) isn't played (Assumption).
- t993: Strike Raid's trail (`trap_obj_effect_start_bind_other`, element 32 on the thrown Keyblade), catch flash
  (`trap_obj_effect_start_pos`, 34 where it was), and its hits: the script's attacks (Raid 1866 radius 160, Judgment 1867
  radius 190) already become KH1 attack objects moved with the flight (`attack_new` / `_set_radius` /
  `_set_pos`); `trap_attack_set_obj_pax` nothing (the limit's effects package). The thrown Keyblade's points for its effects
  are its place (`KhWorld::bone`).
- Open: the list greyed while a
  limit runs (refused with KH1's "can't" for now), the revert during a limit (KH2 waits), KH2's light fade-out time (KH1's 30
  ticks used), limits in the air (refused: Assumption).

### 23.4 Fixes after the owner's first limit tests (t994..)
- t994 / t995: Limit Form's limits (argument 0x20..) and follow-ups (0x28..) were taken for Drive forms (argument 0x10..) in
  KH1's menu entry run and in the command step, so nothing happened; only the forms' own arguments are forms now.
- t996: KH2 movesets' clips read for every motion id 0..255 (`kh2-data::sora::moves`, was 0..230): Strike Raid's 231..234
  and Infinity's 241..248 had no clips, so no slots ("slot isn't built").
- t997: the follow-ups on Triangle KH1's way. KH1's command fill `0x1402948F0` clears the command slots each frame and
  then calls the reaction provider KH1's limit states set (`0x140293180`: Sonic Blade `0x14039ED70`, Ars Arcanum
  `0x14039F1E0`, Strike Raid `0x1403A0230`, each offering its follow-up with `0x140293000` from there; Checked (decomp)).
  Ours offered from the script's step was wiped by the next fill (the owner: no reaction command). Now the script's and the
  moves' offers are noted and a provider of ours, set while one of our limits runs as KH1's limits set theirs, offers it.
- t998: a limit's hook threads step before its main context each frame (`kh2_magic::Magic::step_external`). The main loop
  ends the limit once the player's motion isn't the one the hook last started; the hook restarts the queued motion the frame
  the queue reaches it. With the main context first, every limit ended after its first move (the owner's log: "its script is
  over" about 30 frames in, so no follow-up window was ever reached). Assumption: KH2 runs an object's hooks before the
  limit's thread.
- t999: the limit runs in a KH1 state of its own, as KH1's limits do (Checked (decomp): each registers its state with
  `0x1402B1B80` at its start and enters it with `0x1402B1960`; Strike Raid's 0x27: flags 4, enter sets its reaction provider
  and turns the cull off, leave clears both, transition `0x1402B1840` keeps him in it, its last move ends it in state 0).
  Ours, `limitform::LIMIT_STATE` 0x2D (free): registered by KH1's own call each start, KH1's own stay-transition and cull
  calls, our enter / leave (provider, cull, a thrown Keyblade back) and a step adding nothing (the moves' root travel, which
  KH1 adds after the step, moves him: no walk speed, no gravity; Assumption: KH2's limit actions add none). The limit's end
  puts him in state 0. KH1's `+0` 0x200080 bits aren't set: KH2's invulnerability stays its moves' range 27.
- t1000: `trap_sysobj_motion_is_end` answers (KH2's motion block `+0x18C` bit 1, `0x14042B440`): our slot's clip at its last
  frame, or wrapped round. It was always false, so Infinity's and Strike Raid's scripts waited forever on their start move
  (nothing queued after it): the owner's "starting loop".
- t1001: the follow-ups go through when pressed (the owner, t999: "sonic rave and last arcanums reaction commands won't
  work"). KH1 (Checked (decomp) and (exe)): using a slot's command (`0x140293480`) refuses a kind-2 record with an argument
  (ours) and KH1's own limit follow-ups unless `0x1402A2EC0 (sora)` passes: Sora free (`0x140297870`: `+0` bit 4, `+0x88`
  bit 0x20, `+0x444` bit 0 clear) or in KH1's combo window (`+0x1CC` bit 22, the playing motion's event bits). A limit move
  is an action request (`0x14029F980`: bit 24 sets `+0` bit 4), so he is never free in one; KH1's limit moves carry the
  window in their motion data and their providers keep the offer only while it's open (`0x14039ED70`, `0x1403A0230`). Ours
  had none, so the offer showed but the press was refused. Now our provider opens bit 22 on `+0x1CC` for the frame it
  places the offer (the fill runs inside KH1's command step before the press is read; the next motion step rewrites it).
  Our state's flags (4) keep the Attack slot greyed (`0x140294220`), so no KH1 swing takes the window. KH1's Triangle box
  (Checked (decomp), `0x140277CF0`: hidden, sliding in, shown, sliding out in `0x142852814`) takes a record without flag
  0x10 only once it is fully shown (about 7 frames after it appears; a new record slides in again), as it does for the
  chest's Open; Assumption: KH2's reaction slot takes the press as soon as it shows. Log: "Limit: follow-up record …".
- t1002: `motion_ended` counts a queued move at KH1's count `+0x178` >= 1 (Checked (decomp): the count holds only what is
  still to play; the apply `0x14029FC40` counts the head off; KH1's end rule `0x14029F6E0` reads `0 < count`). At >= 2
  Strike Raid's throw (232, catch 234 queued) read as ended a frame early, so `limi` skipped the catch and its main loop
  ended the limit on 234 before Raid was offered. Log: "Limit: KH2 motion … ended for its script".
- t1003: `asked_motion` reads a request KH1 hasn't applied yet (`+0x160` bit 0, the request at `+0x17C`; KH1's motion step
  copies it into `+0x164` later, `0x14029FC40`, Checked (decomp)). From `+0x164` alone a Bash chosen in KH1's command step
  was seen as the move before by the next script step, so `limi`'s main loop ended Last Arcanum while KH1 played the Bash.
- t1004: KH1's Triangle box takes our follow-ups the frame they show (the owner's answer 1 (a)). KH1 exempts only Ars
  Arcanum's 0x58 / 0x59 from the box's slide-in wait (`0x140293B46`, Checked (exe)); that compare now jumps to KH1's same
  test with our records 0x85..0x8D added, then back to KH1's two exits. The box's drawing is KH1's as before.
- t1005: a limit script's wanted facing turns Sora at once (the owner's answer 2 (a)) through `kh2slots::turn_to`, the turn
  `turn_to_lock` makes (its travel so far and step turned with him). KH1's gradual turn (about pi/16 a frame) bent each
  Rave's dash when nothing was locked. Assumption: KH2 turns to the wish at once (its body turn not found in the decomp).
- t1006: KH2's motion 4 (the fall) asked by a limit's script is KH1's Ragnarok fall: motion 6 queued with bit 30 and the
  entry callback `0x14039FFB0` (state 2 as the entry starts, Checked (decomp)), or state 2 and motion 6 at once for a start.
  Dropped before, so Infinity sat in Shoot (or the hover) for good: the script's main loop runs while he's in 241..248.
- t1007: `trap_obj_party_attack` hands its motion to the script's view of Sora (as a motion start does) and pushes the
  row's next motion with its blend (KH2's `0x1403D78D0`, Checked (decomp)): `limi` restarts the motion the party attack
  gave (pc 1701..1714), and with the old view it restarted Infinity's 241.
- t1008: Infinity's spins rise (the owner's answer 3 (a)): its state steps on KH1's jump step `0x1402AF6A0` as Ragnarok's
  0x25 does (Checked (decomp)), and each party attack's KH2 hop (PTYA rows 52..56, KH2's rule on KH1's target) is written
  into KH1's arc fields (`shared::physics::limit_hop`; gravity per the Physics row, as KH2's other hops on KH1).
- t1009: Infinity's hover: KH2's no-gravity range 3 on our slot (KH1's level reader) holds him with KH1's jump step's own
  hold (`+0` bit 12, Ragnarok's way, Checked (decomp)); the arc's height and air speed zeroed at its start so he falls from
  rest at its end.
- t1010: `trap_obj_set_movement` (KH2's move direction and speed, `0x1403B9DA0`, Checked (decomp)) on KH1's move direction
  `+0x20` / speed `+0x2C` and, on the jump step, its air speed `+0x3C0`: Infinity's follow-ups stop him before the push back.
- t1011: off a ledge, Config > Drive > Limit > Ledges (KH2 by default; ini `limit_ledges`). KH1: our state's transition is
  KH1's Sonic Blade's `0x14039EBA0` (no floor: fall motion 6, state 2, the limit ends; Checked (decomp)). KH2: KH2's edge
  stop (range 29) on our step, and that fall only with no floor and no ground-hold range (KH2's mover `0x1403B9090`).
  Infinity keeps the stay transition.
- t1013: Limit Form's script effects play: `kh2dpx::pkg_of` gives the form's archive (`Pkg::Limit`) for its package number
  (19), which `spell_start` had looked for as `Spell(19)`, never built (the t1012 log: elements 20 / 23 / 34 / 35 / 36
  "isn't built"). The hit sparks of its attacks use the same mapping.
- t1014: a form script effect's sound is tied to the effect (KH2's bind node keeps it, `0x1402C6B30`, and fades it out as the
  effect goes, `0x1401DE500 (h, 0x78)`, Checked (decomp)): a kill or fade of the effect fades its sound with KH1's own
  `0x140179A20 (no, id, fade)`. Infinity's charge sound now stops at the follow-up or the hover's end.
- t1015: Strike Raid's thrown Keyblade spins: the script's start of KH2's weapon motion 235 (bone 2 turned 4 pi about its z
  every 30 frames, looping, Checked (files)) turns the whole thrown Keyblade about KH1's blade bone 282's z on KH1's pose
  (`thrown_world`). The keychain's sway and the spin's sense on KH1's bone are Assumptions.
- t1016: Strike Raid's element 32 on the thrown Keyblade: KH2 binds it to the weapon's root; ours starts it unbound (KH1's own
  start) and places it on the blade's root (its place and the script's angles) each frame it flies (`thrown_world`), as
  Wisdom's shots are placed. The catch's unbind and loop end end it as KH2's.
- t1017: Sonic Rave's KH1 quotes (Battle Quotes KH1): KH1 stand-ins for its KH2 voice keys (`kh1voice::STAND_INS`): the start
  "Give me strength!", the first Rave silent and the rest a swing grunt (KH1's Sonic Blade), the Finish KH1's big line.
- t1018: Last Arcanum's KH1 quotes by kind: the start "Give me strength!" (then a grunt and the finisher line in it), Bash
  1..4 a swing grunt, Bash 5 KH1's finisher line, The End KH1's big line (Ars Arcanum's ending).
- t1019: Strike Raid's and Infinity's KH1 quotes by the same rule: the starts "Give me strength!", each throw / spin a swing
  grunt, Judgment and Giga Impact the big line, the hover the finisher line, Shoot / Impact a grunt.

### 23.5 Limit Form: unfinished (the owner, after t1019: "note that limit is unfinished")

Paused here to move on to the next form; t1013..t1019 are untested in game. Open, to pick up later:

- **Effect slots** (the owner: "run out of effect slots"): Infinity's 10 / 17 / 25 shots (element 23 each, plus their
  attacks) never started before t1013, so whether KH1's element / effect slots run out isn't known. The t1012 log has no
  "not started (no free element or effect slot)" line, but that line is logged only for the first 20 starts of a session
  (`kh2dpx` `STARTED`), and "doesn't play" once per element. Next: a per-start count of the script's shots started and
  refused, not capped; if KH1 runs short, decide with the owner (KH1's pool size vs fewer drawn shots).
- **Strike Raid's spin** (t1015): the axis and sense on KH1's blade bone 282, the keychain's sway (bones 3..11 of KH2's
  235) and bone 0's quarter turn are Assumptions; KH2's `obj_set_xyzrot` Euler order (X, Y, Z) is an Assumption
  (t1015 / t1016). Element 33 starts twice per catch (234 restarted; KH2 likely the same, Assumption).
- **Sounds** (t1014): KH2's 0x78 fade read as KH1's unit (Assumption); whether 999045 loops (se999 not checked).
- **Sonic Rave**: the script restarts 212 on the frame KH1's queue started it (frame 0 twice); KH2's turn to the wish
  read as instant (t1005, Assumption).
- **Infinity**: the hop's fall uses KH1's gravity at Physics KH1 (the Physics rule); KH2's jump start (`+0x18` of the
  player) not followed; `trap_limit_reference` answers 0; a Revert out of MP during the shots would kill them
  (`formmagic::end_all`, untested); whether its shots hit (Magic log off in the t1012 test).
- **Strike Raid**: KH2's catch (234) has no invulnerability range (range 27 covers 231..233 only); kept KH2's.
- **Quotes** (t1017..t1019): what KH2's clips 25 / 32 / 33 say (unheard); Battle Quotes KH2 on the limits not yet
  confirmed in a log (Sound category off).
- Research notes for all of it: the research rounds' reports (Sonic Blade / Sonic Rave, Ars Arcanum / Last Arcanum,
  Strike Raid, Ragnarok / Infinity) are summed up in §23.4's t1002..t1019 notes and the AGENTS stages.

## 24. Master Form on KH1's states (the owner, after t1019: "move onto the new form", 2026-10-30)

### 24.1 KH2 (Checked (files): `P_EX100_TRIF.mset` / `.mdlx` / `.a.us`, `W_EX010_TRIF_R` / `_L`, 00battle, 03system; (decomp) where noted)

- Cost 4 bars, both partners, owned after Anti-Sora (already built). PTYA set 4: ground 185 / 181 / 182, finishers 183 / 184
  (184 the long body-hit finisher, ATKP 1100 / 1101), air 185 / 181 / 182, air finisher 184; no guard, Square or ability rows.
- Motions 41 / 42 / 43 are its repel clip D350: no guard window (Limit's and Valor's 42 have ATKP 162).
- Casts D370..D374 (one per spell, ground, finisher and air); root travel: Fire 799 forward, Blizzard out 347 and back to 106,
  Thunder 718 sideways and 769 forward, Cure none, Reflect 93. Magic sub +20 (`magi` rows 20..25, ATKP 502 subs 20 / 22 / 24).
- The off-hand Keyblade floats: `W_EX010_TRIF_L`'s bone 0 is animated away from the left palm (up to about 106) in every motion;
  `_R` is held. Data, from the left palm (`sklt` row 1: palms 178 / 86; Master's objentry joint an Assumption).
- Effects: `P_EX100_TRIF.a.us` (29 elements), started by its motions' frame triggers; no script in the model.
- EXP: a Drive orb picked up in form 4 adds its value (1 / 3) to Master's EXP (`0x1403E8760`, Checked (decomp)). Levels: Aerial
  Dodge 1, 1, 2, 2, 3, 3, 4; EXP 40 / 80 / 140 / 224 / 448 / 668; rewards Auto Master (387) at LV2, Air Combo Plus at LV4 / LV6.
- Not checked: the "+20" code, Master's objentry weapon joint, KH2's per-form command disabling for form 4.
- Correction (t1031): Master has item use 9, in its field slot only (`A110`, as Wisdom's and Limit's); KH2's lookup
  `0x1403C7C50` tries the id's four slots (table `0x1405C5D38`), so it plays its own (Checked (files) / (decomp)).

### 24.2 The owner's answers (2026-10-30)

1. Save bytes: Master's Keyblade at `+0x489`, its level at `+0x892` (next to Limit's `+0x890`).
2. Story Unlocks off: Master (and Limit) owned as Valor and Wisdom are.
3. KH1's Gravity / Stop / Aero in Master: Wisdom's mapping (the clip decides the travel).
4. The look: main's yellow spec (jumpsuit, collar and hood lining yellow, belt black).
5. The Equipment rows: V: and M: both hidden until the form is owned.
6. KH1 commands (2026-10-30, with "go ahead"): "valor and master shouldn't refuse commands, we'll hand tune which ones if
   something looks off, it should be the same as wisdom form" (Valor's two-Keyblade refusal goes, for both).
7. KH1 battle-quote stand-ins for Master: yes (by t1017..t1019's rule).
8. Item use (Master has no motion 9): what KH2 plays in its place, found first.

### 24.3 Plan and built (one system a version)

- t1020: guard only from a guard window in motion 42 (every form); Master's 42 has none.
- t1021: Master on KH1's states: `Kind::Master`, Drive record 0x8E / word 0x1EE (Checked (files)), `MASTER_SPECS` (reactions, recoils 41 / 43, fall / land, jump 3, air combo 181 / 182 / 185 on 0xCC..0xCE), Aerial Dodge 202 / Air Recovery 40 on our rows, Aerial Dodge LV1 until t1025.
  Story Unlocks off: Limit and Master owned as Valor and Wisdom. One Keyblade, Sora's look, no package, base Sora's ground combo slots (Master has none on the ground yet): test with t1022.
- t1022: Master's ground combo. PTYA set 4's ground rows all hop (41; row 1 14) and its finishers 183 / 184 are air rows (Checked (files)), so the ground combo is KH2's hops into the air combo, on our air rows as base Sora's hops 0 / 1. 185 (ground rows 5 / 6, air rows 20 / 22 / 23) gets a row of ours (`kh1rows` "185", air swing), built only from Master's moveset. No ground rows or ground slots were needed.
- t1023: KH1 commands in Valor and Master as in Wisdom (answer 6): the two-Keyblade refusal (`formcmds::valor_allows`, `combat` command step and attack-row run) is gone. Limits in Forms and Valor's no-magic rule unchanged.
- t1024: Master two-bladed: its `_L` carried from KH2's left palm 86 by `kh2_hold` in the holder bake (the float from `_L`'s bone 0, data, no per-frame code), `_L`'s hits and trail as Valor's, the second Keyblade Sora's own until the M: slot (stats added as Valor's empty slot).
- t1025: the M: slot. Master's Keyblade at save `+0x489` (answer 1), empty = Sora's own (round 11 answer 1 (a)); an old save's byte that isn't a free Keyblade is cleared. V: and M: rows in Equipment only once the form is owned (answer 5). Free count, compare box, pick list and the fill on gain as Valor's (`equip::candidates` keeps the two slots' copies apart; Valor's first). `valor::master_keyblade` reads it for the left hand and the stats.
- t1026: Master's level record at save `+0x892` (answer 1), next to Limit's. A Drive orb picked up in Master adds its value (1 / 3) to Master's EXP (KH2 `0x1403E8760`, at `driveorbs::pickup`, no per-frame code). Levels by fmlv form 4 (EXP 40 / 80 / 140 / 224 / 448 / 668); its Aerial Dodge 1, 1, 2, 2, 3, 3, 4 and base Sora's one under (menu name and box); Air Combo Plus at LV4 / LV6 given, Auto Master (LV2) has no KH1 counterpart.
- t1027: Master's magic as Wisdom's (per form: `Kind::kh2_magic`, sub + 20): KH2's Fire / Blizzard / Thunder / Cure and Reflect on its own D370..D374 casts on KH1's cast requests; KH1's Gravity / Stop / Aero on Thunder's / Cure's clips (answer 3); finisher subs 21 / 23 / 25 fall back to the plain ATKP row (Assumption).
- t1028: Master's effects and sounds as Limit's (t971): `Pkg::Master` (`obj/P_EX100_TRIF.a.us`), `kh2fx::MASTER` = 20, its trigger-1 elements and their sounds; its swings / voices loaded and played by the generic slot events (swings only with `kh2_sounds = all`, as Limit's).
- t1029: Master's look (answer 4): main's yellow spec, from main's `tint.rs` (belt black; hood lining, jumpsuit and collar strip one bright yellow by `Op::Paint`, main's values), `tint::Look::Master`, built once into the tint cache; Player Model kh2 draws `P_EX100_TRIF`.
- t1030: Master's KH1 battle quotes (answer 7): the row-less lines only, in Master: 183 voice 4 → group 4, 183 / 184 voice 5 → group 5 (`kh1voice::MASTER_STAND_INS`); the frame-0 lines are KH1's row voices already (4 on 181 / 182 / 185, 5 on 183 / 184).
- t1031: Item use in Master (answer 8): Master has motion 9 in its field slot only (`A110`, as Wisdom's and Limit's, Checked (files)); KH2's lookup `0x1403C7C50` tries the id's four slots by the table at `0x1405C5D38` and nothing else (Checked (decomp) / (exe)), so Master plays its own. `MASTER_SPECS` gets the item specs (0x3E / 0x3F).
