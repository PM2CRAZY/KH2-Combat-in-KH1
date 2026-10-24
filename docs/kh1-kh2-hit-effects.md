# KH2 hit effects on enemies, in KH1's own hit-effect path

Research notes (t480 round) for the thread `testers/kh2-hit-effects-on-enemies/`. All addresses
are **Steam** (as the Steam Ghidra decomps number them) unless marked otherwise. Related:
`docs/kh2-effects.md` §16-18 (the native KH2 effect player, `kh2dpx`), `docs/kh1-attack-elements.md`
(KH1's attack data, its `+0x28` field), `docs/kh1-clash.md`.

## 1. What the mod does now (Checked (code))

- Every KH2 hit Sora lands is landed by KH1's contact code (`combat::land_hit`): a temporary KH1
  attack object made from one of Sora's KH1 rows (`new_attack`, Epic `0x1402B10E0`), whose `+8` is
  the equipped Keyblade's KH1 effect list, then KH1's contact (Epic `0x1402B1ED0`) picks and starts
  the spark. So swings show **KH1's Keyblade spark**, chosen by KH1's attack data `+0x28`.
- Spells (t345): the attack object's `+8` is the KH1 spell's own effect list from its magic slot,
  so Fire / Blizzard / Stop show KH1's spell spark; Thunder / Gravity none.
- `probe::hit_effect_note` (t338) logs which KH1 spark id a contact picks; ini `hit_fx_test` /
  `hit_effect_test` are test switches.
- No KH2 hit effect element is ever started on a target. The native effect player (`kh2dpx`)
  builds the Keyblade's package (`obj/W_EX010.a.us`, `Pkg::Weapon`) but only switches on its vanish
  flash (`WEAPON_ON`).
- `kh2_formats::pax` reads each PAX element's 64-bit word at `+8` and **drops it** (`_category`).
  That word is exactly what KH2's hit-effect lookup searches (below).

## 2. How KH2 picks and places a hit effect (Checked (decomp), KH2 Steam)

- An attack object holds its ATKP row at `+0x30` (`0x1403d61d0` reads refact self / other at row
  `+0x13` / `+0x14`, which are those fields) and a handle to **its own PAX** at `+0x28`.
- Body hit: `0x1403d1730` (attack, target, contact) makes the hit record (`0x1403d23c0`) and,
  unless the target's flags `+0xD10` / `+0xD14` bit 13 (not cancelled by `+0xD18`) say "no hit
  effect", calls `0x1403d1b40` (attack, target, record, contact, kind). The kind is **1** for a hit
  that counts (target's vtable `+0xB8` false and `0x1403cf770` a new hit: also sets the attack's
  `+8` bit `0x1000`), else **4**.
- `0x1403d1b40`:
  - the contact point is the attack centre pushed onto the target shape's surface (clamped to its
    height), stored at `0x14074c0d8`;
  - if the ATKP row's **effect_on_hit** (`+0x0B`) is non-zero and the attack has a PAX:
    - record `+0x18` bit 2 clear: the element of the attack's PAX whose 64-bit word holds
      `1 << (32 + effect_on_hit - 1 + r)` and the kind bit, where `r` is a random 0..random_effect-1
      when the row's **random_effect** (`+0x2D`) is set (only 2 of 2,713 ATKP rows use it, Checked
      (files)); found by `0x1402c6730` (first element whose word contains the whole mask; elements
      0x50 bytes, word at `+8`, returns the element's `+2`), started by `0x1403f6410` ->
      `0x1402c6b30` (priority 10000) at the contact point, then turned by the record's direction
      (`0x1403b7710` into the effect's matrix) and `0x1403d9be0`;
    - record `+0x18` bit 2 set: the system PAX's element **0x12** instead. That bit is set by
      `0x1403ce740`'s record fill when the target's element resistance for the row's element
      (`+0x09`) multiplies to 0 (an immune hit).
  - Then the **defender's** own PAX (target `+0x80`), when the row's **damage_effect** (`+0x24`) is
    non-zero and the target's `+0x120` bit 9 is clear: element `1 << (32 + damage_effect - 1) |
    0x200`, one unit above the contact point.
- Other kinds in the same word (Checked (decomp)): `0x10` attack meeting attack (`0x1403d10c0`),
  `0x100` / `0x2` a wall hit by surface class (`0x1403d1960`), `0x400` / `0x800` a reflect
  (`0x1403d12c0`, by the target's `+0x6C8` bit 7). Guard / clash sparks are fixed system elements
  0xB / 0xC / 0xD (`0x1403d61d0` -> `0x1403d0f00`).
- The system PAX is `00effect.bar` ("syst"), loaded by `0x1403f65a0` into `0x142ae7508`
  (`0x1403f6590` returns it).
- The attack's `+0x28` PAX: **Assumption** (not traced to the constructor): the PAX of the object
  whose shape made the attack (the weapon's for a Keyblade swing, the body's for body hits), which
  `trap_attack_set_pax` / `trap_attack_set_system_pax` (BDX traps 2/30, 2/94) can replace for
  scripted attacks. The files fit it (below).

### What the files hold (Checked (files), the user's KH2 install, extracted outside the repo)

| PAX | element (index / id / effect) | kind | effect_on_hit bits |
|---|---|---|---|
| `obj/W_EX010.a.us` "w_ex" (Kingdom Key) | 1 / 1 / 1 | 0x1 | 1 |
| | 2 / 2 / 2 | 0x1 | 2 |
| | 3 / 3 / 3 | 0x102 (wall) | 1, 2 |
| `obj/P_EX100.a.us` "p_ex" (Sora) | 14 / 14 / 9 | 0x1 | 1 |
| | 42 / 42 / 29 | 0x1 | 2 |
| | 43 / 43 / 30 | 0x1 | 3 |
| `00effect.bar` "syst" | 9 / 13 / 13 | 0x10 | 10 |
| `obj/M_EX010.a.us` (Soldier) | 3, 4, 7 | 0x1 | 1, 2, 3 |

- ATKP effect_on_hit over all 2,713 rows: 1 (1,296), 2 (508), 0 (428), 3 (264), 4..12 the rest;
  damage_effect 1 on 900 rows. No element answers kind 4, so a non-counting hit shows nothing.

## 3. How KH1 does the same thing (Checked (decomp), KH1 Steam)

- `0x1402b4320` (attack, kind, at): the source is the attack's `+8` effect list (a DPX), the group
  bit is `1 << data+0x28` (attack data handle `+0x5C`; < 0: none), kind 4 is looked up as 2;
  `0x1401dffe0` (dpx, kind, group, 0) finds the first DPX entry (0x20 bytes from `+0x10`) whose
  kind mask (`+0x10`) and group mask (`+0x14`) hold both; `0x1402ae650` starts it at the point
  (priority -10000). Callers `0x1402b3a00`, `0x1402b4060`, `0x1402b4400` (`0x1402b4400` first takes
  the contact point with `0x1402b3f50`). Epic equivalent: `0x1402B2190` (translated, as
  `docs/kh1-attack-elements.md` lists it).
- So KH1's lookup is KH2's with the PAX layer folded into the DPX entry: KH2's word's low half is
  KH1's kind mask, its high half KH1's group mask, and **KH1 group g = KH2 effect_on_hit g + 1**.
  Same kinds 1 / 4. KH1 has no "defender's own effect" step and no immune-hit system element
  (**Assumption**: not found in this pass; KH1 has a kind 0x40 for record `+0xC` bit 0x20).
- Nothing else reads KH1's hit-effect choice: no tally, event or script (Checked (decomp): the
  start's result goes back to the contact only). Sound is separate (`0x140293FD0` Epic, the
  Keyblade sound byte).

## 4. Plan (native: KH2's choice running through KH1's lookup)

One system per version:

1. **Keep the PAX word**: `kh2_formats::pax` keeps the `+8` word; `kh1dpx` writes an element's low
   half into its built DPX entry's kind mask (`+0x10`) and high half into its group mask (`+0x14`)
   (today it copies KH2's DPX entry masks). Offline check: W_EX010's three elements come out with
   kind 1 / 1 / 0x102 and groups 1 / 2 / 3.
2. **KH2's Keyblade hit elements built**: `Pkg::Weapon` also builds W_EX010's elements 1..3 (and
   each form's weapon package's) into its KH1 DPX through the existing loader (background thread,
   KH2 HD textures through KH1's loaded-file list, as t368-t370).
3. **KH1's lookup gets KH2's inputs**: in `land_hit`, for a KH2 swing, the attack object's `+8` is
   that DPX's handle (as t345 does for spells) and its attack data is a static copy of the KH1
   row's (one per row and effect_on_hit, never freed: KH1 reads it after the call) with `+0x28` =
   effect_on_hit - 1 (+ the random step); KH1's own `0x1402b4320` then picks and places it. Rows
   with effect_on_hit 0: no spark, as KH2. Immune hits (KH1's 0 damage by element): the system
   element 0x12 needs `00effect.bar` built too (a later version).
4. **Defender's damage effect** (`+0x24`): KH1 enemies have no KH2 PAX; nothing to start unless
   we pick KH1-side stand-ins (open question).

Performance: builds once per package on the existing background thread; per hit, one copy lookup
and KH1's own search (no new per-frame work, no new hook). KH1's 10 draw-time texture slots
(`0x1401EDB00`) are shared with every other effect: **Assumption** that two small sparks fit
beside Explosion's.

## 5. Not found / open

- The constructor line that fills the attack's `+0x28` (which PAX a swing carries) and what
  `0x1402c6b30`'s second argument is (the element's `+2` id vs its index: equal in every hit
  element listed above, so it doesn't matter here).
- KH1's Keyblades other than the Kingdom Key have no KH2 file of their own (KH2's W_EX0xx are KH2's
  Keyblades); KH2 forms' second weapons have their own packages.
- What KH2 hit effects Sora's own body hits use (P_EX100's 14 / 42 / 43) and which of Sora's ATKP
  rows carry effect_on_hit 3 (logged once step 1 is in).

## 6. The tester's answers (t480 round) and what was built

- **Keyblade swings:** KH1's own spark stays, for every Keyblade (answer 1: "(c) but for now wait
  on changing any base effects the keyblade calls", until KH2 Keyblade models and a KH1 / KH2
  Keyblade switch come). Nothing about swings changed.
- **Spells:** KH1's own spells keep KH1's spark (t345); KH2's spells (KH2-style magic and the
  forms' KH2 casts, `formmagic`; Reflect's pop, `reflect`) call their own (answer 2 (a)).
- **Defender's damage effect:** answer 3 (b), research below (§8); nothing built yet.

### 6.1 The PAX word kept (Checked (code))

`kh2_formats::pax::PaxElement::tag` keeps the `+8` word. `kh1dpx::Variant::tag` writes it into the
built DPX entry: low half at `+0x10` (KH1's kind mask), high half at `+0x14` (KH1's group mask).
Every KH2 DPX entry checked has 0 in both words (Fire 1, Thunder 3, Reflect 1, `P_EX100.a.us`,
`P_EX100_MAGF.a.us`: Checked (files)), so nothing else is overwritten.

### 6.2 KH2 spells' hit elements (Checked (files), the user's KH2 install)

| package | element (index / id / effect) | KH2 kinds | groups (effect_on_hit) |
|---|---|---|---|
| `FIRE_n.mag` "fire" (n = 1..3) | 3 / 3 / 3 | 0x1 | 1 |
| | 4 / 4 / 4 | 0x1 | 2 |
| `BLIZZARD_n.mag` "bliz" | 3 / 3 / 3 | 0x1 | 1 |
| | 4 / 4 / 4 | 0x1 | 2 |
| | 5 / 5 / 5 | 0x102 (wall) | 1, 2 |
| | 6 / 6 / 6 | 0x10 (attack meeting attack) | 1, 2 |
| `THUNDER_n.mag` "thun" | 4 / 4 / 4 | 0x1 | 1, 2 |
| | 5 / 4 / 4 | 0x1 | 2 (same id as element 4, never found first) |
| `MAGNET_n.mag` "magn" | 5 / 5 / 5, 6 / 6 / 6 | 0x1 | 1, 2 |
| `REFLECT_n.mag` "refl" | 7 / 7 / 7 | 0x1 | 1 |
| | 11 / 11 / 8 | 0x10 | 2 |
| `P_EX100_MAGF.a.us` "p_ex" (Wisdom) | 17 / 21 / 11 | 0x1 | 1 |
| | 18 / 22 / 12 | 0x6 | 1 |
| | 19 / 23 / 13 | 0x100 (wall) | 1 |
| | 20 / 24 / 14 | 0x10 | 1 |

All bound to bone 0 with no offset, so `kh2dpx` builds them like any element of the package.

**Assumption** (as §2): a spell attack carries its spell file's PAX (the script runs on an object
of that file), Wisdom's shots the form's archive. Not traced to the line that sets attack `+0x28`.

### 6.3 Through KH1's own lookup (Checked (code); KH1 side Checked (decomp))

- `combat::SpellAttack::package` names the package an attack came from; `spell_strike` hands it with
  the ATKP row's effect_on_hit / random_effect to `land_hit` (`Kh2Spark`).
- `kh2dpx::hit_list (pkg, group)`: the package's DPX once relocated by KH1. Its tagged entries get
  their masks written in KH1's numbering each time: KH2's kind 4 (a hit that doesn't count) on
  KH1's 2 (KH1's `0x1402b4320` asks for 4 as 2), KH2's 2 (a wall kind, `0x1403d1960`) dropped, as
  this list only ever answers a body hit. An entry whose effect runs a program KH1 doesn't run as
  KH2 yet is unfindable (masks 0); when that entry is the one a counting hit of the group would
  find, `hit_list` says no and KH1's spark plays instead of none.
- `land_hit`: a copy of the KH1 data the hit lands with (`hit_data`: one 4 KB page of ours near the
  battle data, `kh1rows::near`, 32 slots of 0x80, keyed by data and group, refilled on each use) with
  `+0x28` = effect_on_hit - 1 (+ KH2's random step), -1 for effect_on_hit 0 (no spark, as KH2); the
  attack object is made from it and its `+8` is the package's DPX handle. KH1's contact
  (`0x1402b4060` -> `0x1402b4320` -> `0x1401dffe0` -> `0x1402ae650`) then picks, places (contact
  point from `0x1402b3f50`) and turns (`0x1402ada80`, the record's direction) KH2's element.
- KH1's record fill also looks the attack's list up for **kind 0x80** with the same group
  (`0x1402bf630`, Steam: record `+0x34` the list, `+0x38` the entry, -1 none) and plays it on a
  random enabled body part of the target in its reactions (`0x1402b8790`, `0x1402b9340`, both
  guard `-1 < +0x38`). No KH2 element has 0x80, so with a KH2 list there is none: KH2 plays no
  attacker-side damage effect either. Checked (decomp).
- Not done: the hit sound (still the swing's for spells without KH1 data), KH2's immune-hit system
  element 0x12 (`00effect.bar`).

## 8. Defender's damage effect (answer 3 (b)): research

- KH2 (`0x1403d1b40`): the **target's** own PAX (`+0x80`), element kind 0x200, group
  `damage_effect - 1`, one unit above the contact point, unless target `+0x120` bit 9.
- KH1 has no step that looks into the **target's** effect list on a hit (Checked (decomp): the
  four callers of `0x1401dffe0` are the attack's list in the contact `0x1402b4320`, the attack's
  list for kind 0x80 in the record fill `0x1402bf630`, and KH1's global list `0x14232e090` in
  `0x1402a3ba0` / `0x1402c23e0`). KH1's own "damage effect" is the attacker's kind-0x80 entry
  above, played on the target's body.
- So (b) needs: where a KH1 enemy keeps its own effect list (its object / model; not found yet) and
  whether any KH1 enemy list has entries a KH2 damage_effect could map onto (kind bits and groups
  of KH1 enemy DPXs: not surveyed). Open; asked in the thread.
