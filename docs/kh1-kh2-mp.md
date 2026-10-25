# KH2's MP and MP Charge in KH1 (testing branch, research)

Thread: `testers/kh2-mp-and-mp-charge`. Started from "What is still missing for native integration
into KH1" ("Not in at all: KH2's MP and MP Charge"). Research only, nothing built yet. Steam
addresses (image base `0x140000000`) from the Steam decomps unless marked. Where an address in our
code is an Epic RVA it is given with its Steam match from `crates/kh1-hook/src/win/steam_map.in`.

Tags: **Checked (decomp)** Steam Ghidra decomp, **Checked (files)** read from the KH1 / KH2 game
files staged in `local/re`, **Checked (code)** our testing-branch code, **Assumption** not checked.

## 1. What we have now (Checked (code))

- KH1's own MP pool runs everything: Sora's stats block `+0x44` MP (whole points, "bars"), `+0x48`
  max MP, `+0xBE` the yellow sub-gauge (u16).
- Every KH2 spell (the t477 native casts, `formmagic`, the KH1-spell styles) and Reflect cost
  `reflect::COST` = 100, i.e. 1 KH1 point, through KH1's own check and payment
  (`reflect.rs`: Epic `0x2A2A80` / `0x2A39D0` = Steam `0x1402A4C10` / `0x1402A5B60`).
- Drive Forms on this branch cost MP (`form::COST_MP` = 3 points) and drain the yellow gauge one
  unit at a time through KH1's payment (`form::after_step`).
- No MP Charge, no KH2 MP Haste / Hastera / Hastega / MP Rage. character-core already knows
  "MP charging" for Berserk Charge (`Character::set_mp_charging`), never set in KH1.
- Hits Sora lands still give him MP the KH1 way (section 3.3), KH2 hits included (they land as KH1
  attack objects with KH1 row data).

## 2. KH2's MP (Checked (decomp) unless marked)

The player's status block is `object + 0x5C0`. Fields: `+0x180` MP (int), `+0x184` max MP (int),
`+0x1BC` MP Charge time left, `+0x1C0` MP Charge length (floats), `+0x20C` the charge rate (float,
1.0 at the block's init, `0x140400000.c` area; who raises it for MP Haste is not found yet),
`+0x22C` the MP Rage rate (float), `+4` max HP.

- **MP change** `0x1403D3CF0 (object, delta, flag)`: if MP + delta would go **below 0**, MP Charge
  starts (`0x1403C20A0`) with the length taken from `[object + 0x148] + 0x1C`, and MP is not
  paid. Otherwise `0x1403C0890` adds it, clamped 0..max. With flag set and delta > 0 it starts the
  MP-gain effect (`0x1403D37F0 (obj, 0x14)`).
  - `[object + 0x148] + 0x1C` is the prty record's 8th float ("mp_drive" in our pref reader):
    **3000 for every prty record** (Checked (files), `03system.bin` `pref/prty`, records 1..8).
- **MP Charge start** `0x1403C20A0 (status, length)`: `+0x1C0 = +0x1BC = length`, MP = 0.
  `0x1403C1D10` "charging" = `+0x1C0 > 0`. `0x1403C20E0` = left / length (the HUD bar's fill).
- **The charge tick** (Sora's per-frame steps `0x1403D4CC0`, `0x1403D68B0`, `0x1403D7370`): while
  charging, time left -= dt × `+0x20C` (`0x1403C0DE0`). dt `0x140717480` is 2.0 per 30 fps tick
  (60 Hz units; Checked (decomp) `0x140150000.c`), so the base charge is 3000 / 60 = **50 s** at
  rate 1.0 (Assumption: the base rate is the init's 1.0; no setter for MP Haste found). The tick is
  skipped when `+0x6C8` bit 7 and `0x1403D8AD0` say so (Assumption: during events / form changes).
- **Charge end**: time reaches 0 → `0x1403C0CD0 (status, max MP)`: MP back to full.
- **MP gained while charging** (orbs, MP Rage, items) doesn't add MP: each point takes
  `sstm + 0x7C` = **12** units (0.2 s) off the time left (`0x1403C0890`; Checked (files): sstm
  record at its pointer `+8`, `+0x7C` = 12.0).
- **Spell costs**: the command's cost byte (`03system cmd`, Fire 12, Blizzard 15, Thunder 18,
  Cure 255 "all", Reflect 10; Checked (files) per `docs/kh1-magic-native.md` 7) paid as
  `0x1403D3CF0 (obj, -cost)` (`0x1403D7E90`, `0x1403D8B40`; `0x1403A39B0` lets a debug override
  replace it). Cure's 255 is always more than he has, so Cure empties MP and starts the charge
  (follows from the rule above).
- **Menu state** `0x1403D88E0`: charging → the magic entry is greyed (state 5); MP below the cost →
  state 2 (Assumption: 2 is "usable, will start the charge", which is what lets Cure be cast; not
  confirmed which states grey the entry).
- **MP Rage** (`0x1403D5600`, `0x1403D5E50`, on damage taken, not while charging): MP +=
  damage / max HP × `+0x22C`, truncated. The value `+0x22C` gets per copy is not found yet.
- **Hits don't give MP**: `0x1403D3CF0`'s callers are only the charge end, damage taken (MP Rage),
  spell / command costs and the prize / item table (`0x1403DCA50`, `DAT_1405C7425` the MP of each
  prize type). No hit-dealt path. (Checked (decomp), caller list.)
- **Berserk Charge**: `0x1403C1D10` charging + ability 107 → no finisher rows (already in
  character-core, `docs/kh1-anim-slots.md`).
- MP Haste / Hastera / Hastega multipliers: **not found**. They must end up in `+0x20C`; the
  setter wasn't reached by searching for writes of that field (Assumption: written through a
  pointer the decomp doesn't show as `+0x20C`). Next step: search the ability-bit test around the
  status block's ability words `+0x1D0..`.

## 3. KH1's MP (Checked (decomp))

Stats block = the handle at actor `+0x6C` resolved by `0x14038ADC0`; it mirrors into the save's member record through
`+0xC8` (record `+3` MP, `+0x38` yellow gauge). Built by `0x1402A5130` (Epic `0x2A2FA0`): MP from the
record clamped to max, the yellow gauge clamped to max × 30.

### 3.1 The yellow gauge: KH1's "charge"

- KH1's next MP point needs **(MP + 1) × 30** gauge units. `0x1402A4CB0 (actor)`, run each frame by
  the player step `0x1402A43A0`, turns a full gauge into one point (`0x1402A4A00 (actor, +1)`) and
  keeps the rest. So the more MP he has, the slower the next point.
- **Gauge add** `0x1402A4AE0 (actor, units)`: adds units, capped at max × 30; a negative result
  breaks one MP point into the gauge (point −1, gauge + MP × 30). Skipped when `0x142867364` bit
  `0x800` is set, for members past 9, and for member 0 when its record's first byte is 0.
- **MP change** `0x1402A4A00 (actor, points)` (Assumption: Epic `0x2A2870`, the one `reflect.rs` names; not in the Steam map): clamp 0..max; while the actor has
  `+0x88` bit `0x20` and no Second Wind (ability 9) the change is scaled by btltbl's 0.5
  (`DAT_142D22D30 + 0x88`). Writes the record's MP byte.
- **MP check** `0x1402A4C10 (actor, cost)`: cost ≥ 100 → `cost / 100 <= MP`; under 100 →
  `cost <= gauge + 30 × MP(MP+1)/2` (every point broken down in turn).
- **MP payment** `0x1402A5B60 (actor, cost)`: cost ≥ 100 takes `cost / 100` points; under 100
  takes gauge units (breaking a point if needed). Skipped when the `DebugParam.bin` free-MP flag is
  set. Callers: the spell scripts' syscall 47 (`0x1402B37F0`), `0x14028D800`, `0x1401CE9A0`,
  `0x1401D86B0..0x1401DA5B0` (Assumption: summons / party AI casts).

### 3.2 Where KH1's MP comes from

- **Landing a hit** (`0x1402BF330`, the attack-touched-actor step, Epic `0x2BD180`): gauge units = attack data
  `+0x2C` × (1 + (target `+0xDC` − attacker's record byte 0) × 20 / 100), + 150% with MP Haste,
  at least 1 and at most 250% of `+0x2C` (btltbl `+0x94` = 20, `+0x95` = 150, `+0x96` = 250;
  Checked (files) `btltbl.bin` at `+0x58`). Assumption: `+0xDC` / byte 0 are the two levels.
- **Taking damage** (Sora's hit pass `0x14029BBD0`): gauge += damage × MP Rage copies.
- **MP Haste** (ability 0x17) → combat state (`actor + 0x148`) `+0x184` bit 0; **MP Rage** (0x18)
  → bits 1..3 (copies); set by `0x1402A6380` from the equipped counts (`0x140284BE0`).
- **Prizes** (`0x1402AC080`): MP orb small 2 units, large 15.
- **Refills**: `0x1402AD0B0` (Assumption: save points: +1 point a step and the gauge emptied),
  `0x1403A0BF0` (same pattern), event script command `0x1401AEFE0` (gauge add from the script).

### 3.3 What reads KH1's MP (don't break KH1's events)

- Spells (usable check `0x14028EFD0` → `0x1402A4C10`; payment from the spell script), summons,
  KH1's specials / limits (their costs), party members' casts, items (Ether, Elixir), Cure.
- The HUD's MP bar (blue points and the yellow part, from the stats / record `+0x38`).
- Event script command `0x1401AEFE0` (gives MP gauge); save points / full heals.
- Our own: Drive Forms (cost and drain), Reflect, Wisdom / KH2 spells, the debug menu's MP lock.
- No event condition reading MP was found in this pass (Assumption: none; only the refill command).

## 4. The two systems side by side

| | KH2 | KH1 |
|---|---|---|
| Pool | 100+ MP, int | a few points (bars), whole |
| Costs | 10..18, Cure all | 1 point (or gauge units under 100) |
| Running out | MP Charge, 50 s, full refill | nothing: refill by hits |
| Hits dealt | no MP | yellow gauge from attack data `+0x2C` |
| Damage taken | MP Rage: damage / max HP × rate | MP Rage: damage × copies, gauge |
| MP Haste | faster charge (rate unknown) | +150% gauge per hit |
| Orbs | MP, or 0.2 s off the charge | gauge units |

## 5. Options

- **(A) KH2's rules on KH1's pool** (native): keep KH1's MP field, bar and every reader, and add
  KH2's behaviour in KH1's own functions:
  1. costs scaled onto KH1's points (the magic thread's choice 6b; section 6);
  2. running out: a payment that would go below 0 (`0x1402A5B60` / `0x1402A4A00`) starts an MP
     Charge timer for Sora (one timer, ours, since KH1 has no field for it), MP 0;
  3. the tick on KH1's player step: `0x1402A4CB0` is per frame and player-only, so a hook there
     runs the timer (time left −= the frame's 60 Hz units × rate) and refills max MP at the end;
  4. hits stop charging Sora (`0x1402A4AE0`'s call in `0x1402BF330` skipped for him) and prizes
     shorten the charge instead of adding (12 units a KH2 MP point, scaled);
  5. the charge shown on KH1's own bar: the yellow sub-gauge as the charge's fill (Assumption: the
     HUD draws `+0x38` as the yellow part; needs a look at the HUD draw);
  6. KH1's MP Haste / MP Rage abilities given KH2's meanings (rate / damage share), Hastera /
     Hastega as new abilities if wanted.
  Cost: two or three small hooks, one timer, no per-frame scans. Party members, summons and KH1's
  own spells keep KH1's rules (only Sora's actor changes).
- **(B) KH1's MP as it is**, only the costs scaled (6b). Nothing new; no MP Charge.
- **(C) A separate KH2 MP pool**: a second number with its own bar. Not native (a parallel
  system), and every KH1 reader would see the wrong value. Not recommended.

## 6. How this relates to the magic thread's (6b)

The magic thread (`testers/kh2-magic-and-reflect-native`, question 6) chose (b): KH2's costs scaled
onto KH1's MP. That is the cost half of this. This thread is the pool's behaviour (running out,
the charge, what refills it). They have to agree on one scale: KH2's cost × (KH1 max MP / KH2 max
MP) in KH1 points, or in KH1's gauge units (under 100, 30 a point) so 12 / 15 / 18 can differ.
The scale is built once here (or there) and both use it; Cure "all" then means "empty KH1's MP and
start the charge" under (A).

## 6b. KH2's cost kinds in KH1's engine (t570, the magic thread)

KH1's MP check `0x1402A4C10` and payment `0x1402A5B60` take two KH2 kinds of cost besides KH1's
own (`mpcost::encode`, high-bit tags no KH1 cost has): every unit of MP with at least one whole
bar first (KH2's Cure; the bar is the tester's rule, KH2 needs only "not charging"), and a share
of max MP. KH1's usable check `0x14028EFD0` reads the KH2 kind for base Sora's spells whose Magic
row is KH2. **Checked (decomp)** both games; **Checked (exe)** the three prologues. KH2's "below
0 starts MP Charge" isn't there yet (this thread's plan item 2).

## 7. Not found / open

- MP Haste / Hastera / Hastega's rates (`+0x20C` setter), MP Rage's `+0x22C` per copy.
- KH2's Sora max MP per level (for the scale; `03system` / `00battle` lvup tables not read here).
- Which per-command flag `0x1403C5940` greys a magic command with MP under its cost (states
  under 4 are usable: `0x1403A9EA0`, Checked (decomp); 2 = "MP under the cost, still usable"); how KH2's HUD draws MP Charge (pink bar, text).
- KH1's HUD MP bar draw (to put the charge on it).
- The KH1 refill callers `0x1402AD0B0` / `0x1403A0BF0` (save point or elixir).
