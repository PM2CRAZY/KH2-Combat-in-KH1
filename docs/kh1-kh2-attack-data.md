# KH2 attack data and damage on KH1's swings

Research notes (t480 round, the thread "KH2 attack data and damage on KH1's swings"). Steam
addresses unless marked; image base `0x140000000`. Where an address was first found on the Epic
build it is given as "(Epic 0x...)" and the Steam one was found in the Steam decomp. Tags:
**Checked (decomp)** Steam Ghidra decomp, **Checked (exe)**, **Checked (files)** the user's
`btltbl.bin` (BackupImage original), **Checked (code)** our testing branch, **Assumption**.
No decompiled code here: plain-language notes only.

## 1. What we do now (Checked (code))

- **Native swings on KH1's own rows** (KH2 motions in KH1's request slots, `kh2slots`): KH1 makes
  the attack object at the row's start from the row's attack data, and every hit is KH1's from
  start to end: KH1's damage, reaction, element, hit effect, MP charge and critical. Nothing of
  KH2's ATKP is used (`docs/kh1-anim-slots.md` §3.2: "Its damage, reaction and hit effect stay
  KH1's").
- **Rows of ours** (`kh1rows`, rows 66.., Square moves, Guard Break): a copy of a KH1 template
  row's attack data with only the reaction kind (`+8` / `+0xC`: 3 flinch or 5 launch from KH2's
  reaction category) and, for Guard Break, the priority (`+0x3C`) changed. After KH1 builds the
  hit record, `combat::scale_record` (called from `probe::hit`, our hook on KH1's "attack touched
  an actor") multiplies the record's damage (`+8`) by the KH2 window's ATKP power / 100, the ini's
  `damage_scale`, and for Magic-sourced ATKP by (1 + 0.1 × max MP) × `magic_scale`; the fraction
  is carried per foe (`CARRY`). KH2 launches ride on KH1's launch reaction (`LAUNCHES`).
- **Borrowed and form moves** (`combat::land_hit`): our own hit test, then KH1's contact code on a
  temporary attack object made from a KH1 row's data (or a KH1 spell's data, `kh1_spell_data`),
  then the same `scale_record`.

So the damage is always "KH1's damage for some KH1 row × KH2's power", and every other field
(element, reaction kind except on our rows, hit effect, critical, MP charge, clash type) is KH1's.

## 2. KH1's attack data and hit pipeline

### 2.1 The record (0x70 bytes)

Layout as in `docs/kh1-attack-elements.md` §1 (read on Epic, the field reads re-checked in the
Steam damage function below). The table is `btltbl.bin`, loaded whole to **0x142D22D40** (Epic
0x142D23740; Checked (code), `steam_map.in`). Key fields: `+0` stat source, `+4` element (low 4
bits), `+8` / `+0xC` reaction kind vs player side / vs enemies, `+0x10` bone (0x30 = weapon),
`+0x14` power vs player side, `+0x18` / `+0x1C` critical chance / bonus, `+0x20` / `+0x24` min /
max vs player side, `+0x28` hit-effect group bit, `+0x2C` MP charge per hit, `+0x38` clash type,
`+0x3C` priority, `+0x44` damage to object actors, `+0x48` power vs enemies, `+0x4C` / `+0x50`
min / max vs enemies.

Sora's rows (block `+0xA9F0`, 0x14 each, `+4` the data offset), read from the user's original
`btltbl.bin` (**Checked (files)**): every plain swing row has stat source 0 (Strength), element
1, reaction 3 (flinch), **power 0** vs enemies, MP charge 3; the finishers (`0x4C58`, `0x4D38`,
`0x5048`) reaction 5, **power 4 or 6**, critical 4 / 4, MP charge 6. Other rows: element 8
(gravity %) with power 40 / 250, element 10 (stun) power 4, stat source 4 (max MP × 3) power 10.

### 2.2 Functions

| Steam address | What it does | Tag |
|---|---|---|
| 0x1402B2DC0 (Epic 0x1402B0C30) | Attack object constructor from a data record: element into the attribute word bits 6..9, etc. | Checked (decomp) for the element bits |
| 0x1402B3270 (Epic 0x1402B10E0) | Its wrapper used by the attack rows; adds the Keyblade's bonuses for bone 0x30 | Checked (code) map |
| 0x1402BF330 (Epic 0x1402BD180) | An attack touched an actor; `probe::hit` hooks it | Checked (code) |
| 0x1402BF630 (Epic 0x1402BD480) | Builds the hit record (hit-once bits at target `+0x3E0`; record pools 0x142DE5E10 / 0x142DE6E10, 0x40 each); calls the damage function; also called from the attack-vs-attack pass 0x1402B4BC0 | Checked (decomp) |
| **0x1402BFA80** | **Damage** `(attack, target, record kind word)`; its only caller is 0x1402BF630 | Checked (decomp) |
| 0x1402BEEE0 (Epic 0x1402BCD30) | Applies a record: stop (9), stun (10), cure (11), aero (12), else HP loss through 0x1402A4920; nothing for clash kinds 6 / 7 or no-effect records; invincible actors (`+0` & 0x1000080) and a global flag (0x142D53A38 bit 2) take 0 | Checked (decomp) |
| 0x1402B4320 (Epic 0x1402B2190) | Hit spark: the data's `+0x28` as a group bit in the weapon's effect list (0x1401DFFE0) | Checked (decomp) |

### 2.3 KH1's damage formula (0x1402BFA80, Checked (decomp))

1. Object-kind targets (actor kind 4, 5, 9, 10): the data's `+0x44`, done.
2. Power = data `+0x48` (vs enemies; `+0x14` vs the player's side) + the attack's stat term
   (`+0x2C`, set by the constructor from the stat source: Strength, max MP, ...). Min / max from
   `+0x4C` / `+0x50` (or `+0x20` / `+0x24`).
3. Critical: a random roll under the attack's chance (`+0x94`) adds its bonus (`+0x98`) to the
   power, marks the record (bit 0x20) and, for Sora, counts journal tally 8 (0x14028C660 (8)).
4. Element 8 (gravity): target HP × power / 100 × resistance, floor at the target's `+0xC0`.
   Elements 9 and 11..12 (stop, cure, aero): the power itself.
5. Everything else: index = power − target Defense (`stats +0x50`), clamped to the curve; the
   damage is the **curve's value** at that index. The curve is the table at the pointer in
   **0x142DE5DC0** (Epic 0x142DE67C0): a count then u16 values. In the user's `btltbl.bin`
   (**Checked (files)**): 42 entries, 1, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 17, 20, 24, 28, 34,
   40, 48, 57, 68, 80, 95, 113, 135, 160, 190, 227, 270, then 300 to the end: about ×1.19 per
   point, flat at 300 from index 29. (That it is the start of the file is an **Assumption** taken
   from the Epic note; the values fit.)
6. Clamp by data min / max × the attack's `+0xAC`, then by the target's floor / cap (`+0xC0` /
   `+0xC2`), × resistance[element] (stats `+0x78 + element × 4`), halved when the target has
   `+0x88` bit 3 (Aero), doubled on the player's side when 0x140285210 says the mode is 2
   (**Assumption**: Proud; it reads 0x1402FAFE0, which two event opcodes also read), Second Chance
   (target `+0x148` → `+0x184` bit 0x10 leaves 1 HP), at least 1.

So KH1's damage is a **lookup on an exponential curve** of (power + Strength − Defense): power
is additive points. KH2's is linear in (stat − Defense) with power as a percentage.

### 2.4 What else reads the attack data or the record (don't break KH1's events)

- MP charge per hit (data `+0x2C`, in 0x1402BF330): KH1's MP regeneration for magic.
- The critical's journal tally (8) and the swing tally (0x13) that the Dive tutorial waits on
  (the tally is counted by the row runner, not here: unaffected by a data change).
- Enemy scripts hear of every record (event 2 at contact, event 0 when the queue is taken;
  `docs/combat-model.md`), and their reactions read the record's kind (`+0xC`), damage (`+8`) and
  data handle (`+0x2C`, e.g. 0x1402B8E10 (Epic 0x1402B6C60) reads data `+0x40` for clash kinds).
  **Assumption**: no script reads the attack data's power or element directly.
- Second Chance, Aero, invincibility and the global no-damage flag all live in the damage and
  apply functions: a replacement must keep them.

## 3. KH2's ATKP and damage

ATKP row (48 bytes, `kh2_formats::battle::Atkp`, Checked (code)): `+4` kind, `+5` stat source,
`+6` power (u16, percent), `+8` team, `+9` element, `+0xA` enemy reaction, `+0xB` hit effect,
`+0xC` / `+0xE` knockback, ... `+0x2E` attack-kind flags (finisher / air finisher / reaction
command boosts), then drive drain, revenge, hp drain, interval and the others.

KH2's damage routine is **0x1403C12F0** (Steam KH2; callers 0x1403CE740, 0x1403CEE90),
**Checked (decomp)**, and it matches `combat_core::compute_damage` / `docs/combat-model.md`
(which was checked on Epic):

- stat = the attacker's Strength (player record `+0x188`, with a form bonus) or Magic (`+0x18A`)
  by ATKP `+5`; non-player attackers take theirs from a default record;
- kind 5 (Cure): ceil(max HP × power × stat / 10000); kind 6: ceil(power × stat / 100);
- else base = stat − defender Defense (`+0x18C`), minus its low-HP Defender bonus at ≤ 25% HP;
  clamped by the attacker's min / max (`+0x198` / `+0x19C`) and the defender's (`+0x1A0` /
  `+0x1A4`); damage = ceil(weakness[element] × weakness[general] × power × base / 1,000,000)
  (weakness bytes at `+0x1A8 + element`, general `+0x1AE`);
- difficulty (0x1409ABD48: 0 Beginner 50 / 150, 2 Proud 200 / 100, 3 Critical 200 / 125, taken /
  dealt, rounded up); a co-op mode divisor / multiplier; Anti Form's rate (system table
  0x142AE5760 `+0x94`); Fire / Blizzard / Thunder boosts (`+0x218..+0x21C`); Combo Boost, Air
  Combo Boost, Reaction Boost by ATKP `+0x2E`; low-HP rate (system `+0x178`).
- **No critical hit and no random factor** in it.

## 4. The gap, field by field

| KH2 ATKP | KH1 data / record | Native route |
|---|---|---|
| power % on (stat − Def) | additive power on the curve | the damage function (§5) |
| stat source 0 / 1 | stat source 0 Strength / 1 max MP | direct (max MP stands in for Magic, as now) |
| element 0..6 | element 1 physical, 2 fire, 3 blizzard, 4 thunder, ... | map 0→1, 1→2, 2→3, 3→4 (**Assumption**: KH2's order physical, fire, blizzard, thunder, dark, neutral, general, from OpenKH); KH2 dark / neutral have no known KH1 element |
| enemy reaction | reaction kind `+0xC` (3 flinch, 4 knock-back, 5 launch) | by `reaction::category` (as `kh1rows` does) |
| knockback | none in KH1's data (`docs/kh1-launch.md`) | our launch hook, as now |
| hit effect / hit sound | `+0x28` group bit in the weapon's list | KH1's spark stays KH1's unless KH2's effect is drawn through `kh2fx` (`kh1-anim-slots.md` §3.3, not built) |
| none | critical `+0x18` / `+0x1C` (Critical Plus) | question |
| none | MP charge `+0x2C` | question |
| interval, revenge, drive drain, hp drain | none | already handled by our hit records / gauge code |

## 5. Plan to make it native (one system per version)

1. **KH2's ATKP as KH1 attack data records.** At load, one 0x70 record of ours per ATKP id that
   Sora's movesets use (a few hundred, under 64 KB, near the battle block as `kh1rows::near` does):
   template = the KH1 row of the move's kind (swing / finisher / air), with KH2's element (mapped),
   reaction kind, stat source and KH2's power kept beside it. Each native slot's KH1 attack object
   gets its data handle (`+0x5C`) pointed at the record of the KH2 window open at KH1's time
   (KH1's damage reads `+0x5C` at hit time, **Checked (decomp)**; that the object can be
   repointed mid-row safely is an **Assumption** to log first). Per-frame cost: one handle write
   per window change.
2. **KH2's damage in KH1's damage function.** Hook 0x1402BFA80 (one call per hit record): for an
   attack whose data is ours, compute the chosen formula (question 1), then run KH1's own tail
   (floor / cap, resistance, Aero, Proud, Second Chance, at least 1). This replaces
   `scale_record`'s after-the-fact rewrite of `+8` and the `CARRY` fractions (KH2 rounds up per
   hit). Prologue bytes to read from the Steam exe before writing the hook.
3. **Borrowed and form moves on the same records** (`land_hit`): their temporary attack objects
   take the ATKP record instead of a KH1 row's; goes with the threads that make those moves native.

## 6. Not found / not checked

- KH2's own hit-effect choice (ATKP `+0xB`, damage effect) in the KH2 decomp: not read.
- KH1 elements 0, 5, 6, 7, 13..15: still unknown, so KH2's dark / neutral have no home yet.
- KH1's difficulty values (0x1402FAFE0): only "2" is used by the damage function; which mode is 2
  is an assumption.
- How far KH2's linear formula on KH1's stats would move damage against KH1's enemy HP: not
  measured (needs a log of Strength / Defense / HP pairs in play).

## 7. Implemented: KH2's power inside KH1's damage function (step 2)

The tester's answers: damage (a), "you pick which one the player is going to feel a difference
in it feeling like kh2"; criticals a Config row; KH1's MP charge kept (a).

- **The pick: (a), KH1's curve × KH2's power.** KH2's own formula (b) is linear in Strength −
  Defense, KH1's curve is about ×1.19 a point and reaches 300 at index 29 (§2.3), and KH1's
  enemy HP is tuned to the curve: on KH1's stats (b) would leave early fights about as they are
  and make late ones several times longer. (a) keeps KH1's balance and keeps what makes KH2's
  moves feel like KH2's: each move's power against the others (Explosion's 25% ticks, the
  finishers), KH2's rounding up per hit, and (with Criticals Off) no random critical.
  **Assumption** (not measured in play): the late-game gap; a log of Strength / Defense / HP
  pairs would settle it.
- **How (Checked (decomp), Checked (exe)).** KH1's damage `0x1402BFA80` loads the curve pointer
  `0x142DE5DC0` once (`mov rcx, [rip]` at `0x1402BFBB4`, its only reference in the function,
  Checked (exe)). `win/kh2damage.rs` hooks the function (first 15 bytes
  `48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18`, Checked (exe)); for an attack of Sora's whose
  KH2 ATKP is known it points that global at a copy of the curve with every value × KH2's power
  / 100 × `damage_scale` (× the Magic stand-in, (1 + 0.1 × max MP) × `magic_scale`, for stat
  source 1), rounded up, for the one call, then puts KH1's pointer back. KH1's function runs
  whole: its critical and tally 8, the data's min / max, the target's floor / cap, resistance,
  Aero, mode 2, Second Chance, at least 1; the record builder and everything after it (MP
  charge, enemy scripts, the hit pass) are untouched. Only the game's thread calls it
  (**Assumption**, as every hit hook of ours: the record builder runs in the actors' pass).
- **Which hits.** A borrowed or form move landed through `land_hit` (its power, `combat::landing`);
  one of our rows (`kh1rows::kh2_atkp`); and now the **native swings on KH1's own rows** (attack
  data bone 0x30, our KH2 slot playing on Sora: the KH2 motion's hit window at KH1's time in it,
  else its first window, `kh1rows::native_atkp`). With Base Combo KH1 no slot of ours plays, so
  KH1's own swings keep KH1's own damage. Elements 8, 9, 11, 12 and object targets never read the
  curve, so they stay KH1's.
- **What changed.** `combat::scale_record` no longer rewrites the record's damage or carries
  fractions while this is in (ini `kh2_damage = off` brings the old after-the-fact way back; on the
  Epic build the hook isn't made and the old way stays). KH1's row power stays in the curve's
  index (it is (a) as asked): KH1's finisher rows add 4 or 6 points before KH2's finisher power
  is applied. Dropping it would be question 1's (c).
- **Not done (still the plan's steps 1 and 3):** KH2's element and reaction on the native swings'
  KH1 data (they keep KH1's row's: physical, flinch, finisher launch, which agree with KH2's for
  Sora's plain combo), and KH2's own hit effect.

## 8. Implemented: the Criticals row

Config > KH1 in KH2 > Combat > Criticals: Off / On (ini `criticals`, default Off as KH2, which has
none, §3). Off: for each attack of Sora's, the attack object's critical chance (`+0x94`, f32,
compared with KH1's random roll in `0x1402BFA80`, Checked (decomp)) is 0 for the damage call and
put back after; so no critical bonus, no record bit 0x20, no journal tally 8, and Critical Plus
does nothing. On: KH1's critical as it is, KH2 hits included (the bonus is added to the power
before the curve, then KH2's power applies). Enemies' and partners' attacks are left alone
(**Assumption** that "all moves that call it" means Sora's).
