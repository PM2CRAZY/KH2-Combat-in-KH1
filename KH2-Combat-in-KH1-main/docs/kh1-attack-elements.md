# KH1: attack data, elements, and the spells' own entries

Research notes (Ghidra headless + capstone, Epic KH1 Final Mix PC, image base `0x140000000`,
addresses absolute; the hook adds `base + (addr - 0x140000000)`). Steam addresses are in the
table at the end (from `/opt/re/steam/gen.py`). Read statically, not yet watched in play.
Confidence: **[high]** read plainly from the code, **[med]** inferred from how the code uses
it, **[low]** a guess. `btltbl.bin` itself is not on the research machine, so no field
*values* of the spell entries are known here, only where they are and how they are read.

Why this matters: `combat.rs` `land_hit` lands every KH2 hit with the attack data of one of
Sora's swing rows, so a KH2 Fire / Blizzard / Thunder (or a Wisdom shot) lands as KH1 element 1
(physical) with Strength as its stat. KH1 multiplies damage by a per-element resistance of the
target, so the element is not cosmetic.

## Summary

- An attack data entry is **0x70 bytes** in `btltbl.bin` (loaded whole to `0x142D23740` by
  `0x14028D840` at boot; offsets are relative to `0x142D23740`, every entry sits at
  `offset ≡ 0x38 (mod 0x70)`). Two halves: one set of kind / power / min / max used **against
  the player's side**, one set used **against enemies** (`+0xC`, `+0x48`, `+0x4C`, `+0x50`).
  [high]
- Element = data `+4` (low 4 bits), copied into the attack's attribute word `+0x30` bits 6..9
  and from there into the hit record `+0xC`. Normal damage is multiplied by the target's
  resistance float `stats + 0x78 + element × 4` (16 floats, one per element, percent / 100).
  [high]
- Element codes: **1 physical (Keyblade)** [high], **2 fire** [med], **3 blizzard** [med],
  **4 thunder** [low-med, by elimination and order], **8 gravity (percentage of HP)** [high],
  **9 stop** [high], **10 stun** [med-high], **11 cure (heal)** [high], **12 aero (barrier)**
  [high]. 0, 5, 6, 7 and 13..15 not identified.
- KH1's spells take their attack data from a per-spell, per-level table in the exe
  (`0x140529320`): **Fire 0x5F28 / 0x5F98 / 0x6008, Blizzard 0x6078 / 0x60E8 / 0x6158,
  Thunder 0x61C8 / 0x6238 / 0x62A8** (full list below). They are offsets into the same table
  as Sora's rows, so `base + ATTACK_DATA_RVA + 0x5F28` is Fire's data. Identical in the Steam
  build. [high]
- Stat source (data `+0`): 0 Strength, **1 max MP, 3 / 4 / 5 max MP × 2 / 3 / 5**, 2 none,
  6 the party leader's Strength. KH1 has no Magic stat; spell power scales from max MP. Which
  source the spell entries use is in the data (not read here). [high for the switch]
- Passing a spell's entry to `0x1402B10E0` is safe, and so is a copy in our own memory **as long
  as the copy lives for good** (a `static`): the hit record keeps the data handle after the
  attack is freed, and the handle encoder accepts any address. Nothing compares the data
  address against the table or turns it back into an index. [high]

## 1. The attack data entry (0x70 bytes)

Readers: the attack constructor `0x1402B0C30 (actor, hit bit, data)` (wrapped by `0x1402B10E0`,
which adds the Keyblade's bonuses when the bone is 0x30; by `0x1402B0FC0 (actor, bit, index)`;
and by script syscall `0x1402B9870`), the record builder `0x1402BD480`, damage `0x1402BD8D0`,
MP on hit `0x1402BD180`, the hit spark `0x1402B2190`, the enemy hit pass `0x1402B6C60`, the
spell usable check `0x14028CE40`.

| off | meaning | where | conf |
|---|---|---|---|
| +0x00 | stat source (table below) → attack `+0x2C` | `0x1402B0C30` | high |
| +0x04 | element (low 4 bits) → attack `+0x30` bits 6..9 | `0x1402B0C30` | high |
| +0x08 | reaction kind against the player's side → attack `+0x30` bits 0..4 | `0x1402B0C30` | high |
| +0x0C | reaction kind against enemies (replaces the low 5 bits of the record's `+0xC` when the target's team `+0x374 & 0x60` is `0x40`) | `0x1402BD480` | high |
| +0x10 | bone (≥ 0 bound to the owner's bone; 0x30 = the weapon, which gets the Keyblade's radius / priority / critical bonuses; < 0 free-standing: attribute bit `0x400`, activity off until a script turns it on) | `0x1402B0C30`, `0x1402B10E0` | high |
| +0x14 | power against the player's side | `0x1402BD8D0` | high |
| +0x18 | critical chance (× 0.05 × owner stats `+0xD8`) → attack `+0x94` | `0x1402B0C30` | high |
| +0x1C | critical bonus → attack `+0x98` (added to power on a critical) | `0x1402B0C30` | high |
| +0x20 / +0x24 | damage min / max against the player's side (× attack `+0xAC`) | `0x1402BD8D0` | high |
| +0x28 | hit effect bit (`1 << n` in the attack's effect group; < 0 none) | `0x1402B2190`, `0x1402BD480` | high |
| +0x2C | MP charge the attacker gets per hit (scaled by level difference, capped) | `0x1402BD180` | high |
| +0x30 | MP cost (hundredths) — read only for a spell's entry | `0x14028CE40` | high |
| +0x34 | low 2 bits → attack flags bits 4..5 | `0x1402B0C30` | high |
| +0x38 | clash type (low 3 bits → flags 17..19); bit 16 spent at birth | `0x1402B0C30` | high |
| +0x3C | priority → attack `+0x68` | `0x1402B0C30` | high |
| +0x40 | motion request an enemy plays for a kind 6 / 7 (clash) record | `0x1402B6C60` | high |
| +0x44 | damage to object-kind actors (actor kind byte 4, 5, 9, 10) | `0x1402BD8D0` | high |
| +0x48 | **power against enemies** | `0x1402BD8D0` | high |
| +0x4C / +0x50 | **damage min / max against enemies** | `0x1402BD8D0` | high |
| +0x54 / +0x58 | bone offsets | `0x1402B0C30` | high |
| +0x5C | radius (int) | `0x1402B0C30` | high |
| +0x60 / +0x64 | directional arc (turn, cosine) | `kh1-reflect.md` | high |
| +0x68 | MP-on-clash factor (× owner stats `+0x68`) → attack `+0xB0` | `0x1402B0C30` | high |
| +0x6C | not seen read | | |

No knock-back speed, height or angle field (see `kh1-launch.md`).

Stat source `+0` (into attack `+0x2C`, which is added to the power) [high]:

| value | source (owner's stats, actor `+0x6C`) |
|---|---|
| 0 | Strength `+0x4C` (+ a bonus from `btltbl + 0x58 + 0xFE` with an ability flag) |
| 1 | max MP `+0x48` |
| 2 | none (0) |
| 3 / 4 / 5 | max MP × 2 / × 3 / × 5 |
| 6 | the party leader's (`*0x142D37C80`) Strength |

Stats block (actor `+0x6C`, filled for the party by `0x1402A2FA0`, for enemies by
`0x1402A3750`): `+0x3C` HP, `+0x40` max HP, `+0x44` MP, `+0x48` max MP, `+0x4C` Strength,
`+0x50` Defense, `+0x78..+0xB4` **resistance per element 0..15** (floats, file value / 100),
`+0xC0` / `+0xC2` the enemy's damage floor / cap, `+0xC4` launch stat, `+0xD8` critical
factor, `+0xE0` damage scale. Party resistances come from the party record's bytes `+8..+0x17`
and are multiplied by each equipped item's bytes `+6..+0x15` (percent). [high]

## 2. Elements and where they matter

Damage `0x1402BD8D0 (attack, target, record kind word)`, by the attack's element [high]:

- **8**: `HP × (power / 100) × resist[8]`, at least 1 (or the floor `+0xC0`): percentage
  damage, i.e. Gravity. Defense and the curve are skipped.
- **9**: the power itself (no HP lost, see below).
- **11, 12**: the power itself (a heal / barrier amount).
- everything else: `power + stat` minus Defense `+0x50` indexes the damage curve at the start of
  `btltbl.bin` (`*0x142DE67C0`); clamped by data min / max × attack `+0xAC`, by the enemy's
  floor / cap `+0xC0` / `+0xC2`, then **× resist[element]**, halved when the target has `+0x88`
  bit 3 (Aero), doubled on the player's side in one mode, Second Chance; at least 1.

Applying it, `0x1402BCD30 (actor, record)` [high]:

- 9: actor `+0x3E4` = `power × 60 × resist[9]` frames and `+0x88` bit 0 (stopped), when
  `resist[9]` > 0; no HP change. While stopped,
  `0x140295E50` still feeds element-9 records to the actor. → **Stop**.
- 10: HP loss, plus motion 0x42 with down time `resist[10] × 1800` frames (`0x1402B6590`). This
  is the explosive barrel's attack (`0x1402C3740`, common entry 0x13), which KH1 sources call
  Stun-elemental against Heartless. → **Stun** [med-high].
- 11: HP **gain** of the power; 11 and 12 make a party member say thanks (`0x140299A40`, voice
  0x17). → **Cure**.
- 12: no HP change; sets the party control block's `+0x180` = power × 60 (a duration) and actor
  `+0x88` bit 3, which halves damage taken. → **Aero** (KH1's Aero is a barrier; Cure and Aero
  both open the party target list).

Other uses: element 1 alone gives enemy prize drops on kind 1..5 hits (`0x1402B59F0`), and is
what the reflect code treats as Sora's Keyblade (`0x1402B2A30`) → **physical**. The explosive
barrel's hit handler `0x1402C3860`: element 2 sets it off (when its fuse value `+0x48` > 0),
element 3 resets its counter (puts it out) → **2 fire, 3 blizzard** [med]. No code tests 4;
by the spell order (Fire, Blizzard, Thunder) it is **thunder** [low-med]. Confirm with the
runtime log suggested below.

KH2 → KH1: KH2's ATKP elements are 0 physical, 1 fire, 2 blizzard, 3 thunder, 4 dark,
5 light, 6 neutral (`combat-core` `Element`). From `00battle.bin`: Fire ATKP 502 is element 1,
Blizzard 248 is 2, Thunder 249 / 250 are 3, **Wisdom's shot 859 is 5 (light)**. KH1 has no
element known to be light; the candidates are 0, 5, 6, 7 (not identified).

## 3. KH1's spells and their entries

- Spell state: 7 records of 0x38 bytes at `0x142D23580` (Fire 0, Blizzard 1, Thunder 2, Cure 3,
  Gravity 4, Stop 5, Aero 6): `+0` level (0 = not learned), `+8` the loaded `.mag`, `+0x10` its
  buffer, `+0x18` **the attack data pointer**, `+0x20` a second attack data pointer or 0,
  `+0x30` a load callback. [high]
- Per-level table at `0x140529320`, 16 bytes per row, row `spell × 3 + level` (row 0 is a dummy):
  `+0` file name, `+8` attack data offset, `+0xC` second offset (−1 none). When the level's
  `.mag` finishes loading, `0x14028D5C0` turns both offsets into pointers
  (`0x14028D7C0 (offset)` = `0x142D23740 + offset`) and stores them in the spell state. [high]
- Casting: the command step `0x1402A5380` calls `0x14028CFB0 (actor, spell, target)`, which
  starts the `.mag`'s script through `0x14028D450` with the arguments (actor, target, arg, code,
  **data, second data**). The script makes its attack objects with syscall `0x1402B9870`, which
  calls the same constructor `0x1402B0C30` with the data it is given. The usable check
  `0x14028CE40` reads the MP cost from the first entry's `+0x30`. [high for the plumbing; that
  the script's damaging attack uses the first pointer is med: the `.mag` scripts were not read]

| spell | level 1 | level 2 | level 3 | file |
|---|---|---|---|---|
| Fire | 0x5F28 | 0x5F98 | 0x6008 | `xm_fire0N.mag` |
| Blizzard | 0x6078 | 0x60E8 | 0x6158 | `xm_blizzard0N.mag` |
| Thunder | 0x61C8 | 0x6238 | 0x62A8 | `xm_thunder0N.mag` |
| Cure | 0x6318 | 0x6388 | 0x63F8 | `xm_cure0N.mag` |
| Gravity | 0x6468 | 0x64D8 | 0x6548 | `xm_gravity0N.mag` |
| Stop | 0x65B8 | 0x6628 | 0x6698 | `xm_stop0N.mag` |
| Aero | 0x6708 | 0x6778 (+ 0x5CF8) | 0x67E8 (+ 0x5D68) | `xm_air01.mag` (all levels) |

The second entries (Aerora / Aeroga only) are probably the wind's damaging part [med]. The same
offsets are in the Steam exe's table. The entries are in `btltbl.bin`, which is always loaded,
so they are valid whether or not the spell is learned or its `.mag` loaded. [high]

Sora's swing rows (`0x4B78`, `0x4BE8`, `0x4CC8`, finishers `0x4C58`, `0x4D38`, `0x5048`) are
in the same table.

Common entries by index (`0x14028D7A0 (i)` = `0x142D23740 + *(0x1405294D0 + 4i)`, 27 of them;
also script syscall `0x1402B8E00`): 0x5EB8, 0x5DD8, 0x5E48, 0x57B8, 0x5828, 0x5898, 0x5908,
0x5978, 0x59E8, 0x5358, 0x53C8, 0x5438, 0x54A8, 0x5518, 0x5588, 0x5748, 0x56D8, 0x8DF8,
0x8E68, 0x8ED8, 0x8F48, 0x8FB8, 0x9028, 0x9098, 0x9108, 0x9178, 0x91E8. Used by world objects
(index 0x13 the explosive barrel) and minigames, not by the spells. [high]

## 4. Safety of passing other data to `0x1402B10E0`

- The constructor stores the data as a handle at attack `+0x5C`. The contact copies it into the
  hit record (`+0x2C`), and **records outlive the attack**: the target's hit pass reads them
  later (kind 6 / 7 reads data `+0x40`), and deferred records (stopped targets, queued delays)
  wait in a 128-entry pool. So the data must stay valid well after `land_hit` returns. [high]
- Handles (`0x14038B080` / `0x14038B0B0`) are a slot (64 slots of 32 MB regions, `0x142EE4730`)
  plus the low 25 bits. Any address encodes; a new region takes a slot on first use and keeps
  it. A `static` in the hook DLL costs one slot. Don't build data on the stack or in short-lived
  heap blocks. [high]
- Nothing compares a data address against the table: the only references to `0x142D23740` are
  the loader and the two offset → pointer helpers (`0x14028D7A0`, `0x14028D7C0`); the index
  table `0x1405294D0` is read only by `0x14028D7A0`; the spell state's pointers are read only for
  the cost and to pass to the script. No code turns a data pointer back into an index. [high]
- The spark: `0x1402B2190` looks the data's `+0x28` bit up in the attack's effect group
  (`0x1402B10E0` sets it to Sora's combat state's group). A spell's bit may find no effect in
  that group (then none plays) or a different spark. Harmless. [high]

## Recommendation

1. **Pass the spell's own entry**: `new_attack(sora, bit, base + ATTACK_DATA_RVA + off)` with
   `off` from the table above for the KH1 spell matching the KH2 spell and the KH1 spell level
   (`*(0x142D23580 + spell × 0x38)`, 1..3; or level 1 if 0), e.g. Fire level 2 → `0x5F98`.
   This gives KH1's element, its stat source (max MP based, if the data says so), its power
   against enemies, its reaction kinds and no Keyblade bonuses (its bone is not 0x30), and no
   MP charge on hit if `+0x2C` is 0. KH2's power scaling (`POWER` in `probe::hit`) still
   applies on top. Alternatively read the live pointer from the spell state `+0x18`
   (`0x142D23598 + spell × 0x38`); it is 0 until the level's `.mag` has loaded, while the
   table offset is always valid.
2. Or **copy a swing's 0x70 bytes into a `static`** and change `+4` (element). Consider also
   `+0` (1 for max MP), `+0x2C` = 0 (no MP charge from spell hits) and, since `+0x10` = 0x30
   adds the Keyblade's critical / radius / priority, a bone other than 0x30 if spells should not
   crit. Keep it a static: see §4.
3. For a Fire / Blizzard / Thunder hit, drop the row's `fx` / `0x400000` (the Keyblade's hit
   sound) or keep KH2's sound only.
4. Wisdom's shots (KH2 light): no KH1 light element is known. Physical (1) is the faithful
   fallback; to choose another, log a few enemies' `stats + 0x78..+0xB4` first.
5. Before switching, log the spell entries once (as `sora_rows` does for the swings): 12 words
   of `0x5F28`, `0x6078`, `0x61C8` should show `+4` = 2 / 3 / 4 and `+0` in {1, 3, 4, 5}. That
   confirms the fire / blizzard / thunder codes and the stat source.

## Steam addresses (gen.py)

| Epic RVA | Steam RVA | what |
|---|---|---|
| 0x2D23740 | 0x2D22D40 | `btltbl.bin` (attack data base) |
| 0x2D23580 | 0x2D22B80 | spell state (7 × 0x38) |
| 0x529320 | 0x528620 | spell file / attack data table (same contents) |
| 0x5294D0 | 0x5287D0 | common entry offsets |
| 0x2DE67C0 | 0x2DE5DC0 | damage curve pointer |
| 0x28D7A0 | 0x28F930 | common entry by index |
| 0x28D7C0 | 0x28F950 | offset → pointer |
| 0x28D5C0 | 0x28F750 | `.mag` loaded: sets the data pointers |
| 0x28D3D0 | 0x28F560 | set spell level |
| 0x28CE40 | 0x28EFD0 | spell usable (cost `+0x30`) |
| 0x28CFB0 | 0x28F140 | cast: start the spell script |
| 0x28D450 | 0x28F5E0 | spell script launcher |
| 0x2B0C30 | 0x2B2DC0 | attack constructor |
| 0x2B10E0 | 0x2B3270 | attack with the Keyblade's bonuses |
| 0x2B0FC0 | 0x2B3150 | attack from a common index |
| 0x2B9870 | 0x2BBA20 | script syscall: make attack |
| 0x2B8E00 | 0x2BAFB0 | script syscall: common entry by index |
| 0x2BD180 | 0x2BF330 | hit (record + MP on hit) |
| 0x2BD480 | 0x2BF630 | record builder |
| 0x2BD8D0 | 0x2BFA80 | damage |
| 0x2BCD30 | 0x2BEEE0 | apply a record (stop, stun, heal, aero) |
| 0x2B59F0 | 0x2B7B90 | enemy damage step (prizes on element 1) |
| 0x2B2190 | 0x2B4320 | hit spark |
| 0x2A2FA0 | 0x2A5130 | party stats init |
| 0x2A3750 | 0x2A58E0 | enemy stats init |
| 0x2C3740 | 0x2C58F0 | explosive barrel blast |
| 0x2C3860 | 0x2C5A10 | explosive barrel hit (elements 2 / 3) |
| 0x38B080 | 0x38AD90 | pointer → handle |
| 0x38B0B0 | 0x38ADC0 | handle → pointer |
