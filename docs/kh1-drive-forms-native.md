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
