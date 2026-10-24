# Sora's hit reactions, native

Research notes for the thread "Sora's hit reactions native" (t480 round). Steam addresses
throughout (Ghidra's, image base 0x140000000). Where an older note or our code gives the Epic
address, it is noted as "Epic, mapped" (through `crates/kh1-hook/src/win/steam_map.in`).

Tags: **Checked (decomp)**, **Checked (exe)**, **Checked (files)**, **Checked (code)** (our repo),
**Assumption**.

## 1. What the mod does now

**Checked (code)** `crates/kh1-hook/src/win/hurt.rs` hooks KH1's player hit pass (Epic
`0x140299A40`, Steam `0x14029BBD0`). For Sora only:

- **KH1's states (base Sora, `shared`)**: KH1's pass runs as in the game, so KH1's reactions
  play. Before it runs, records inside a KH2 move's invulnerability (range 27) or body guard
  (`kh2slots::invulnerable`, `body_guard`) are marked no-effect (`+0` bit `0x20`). A kind 4 / 5
  hit is noted for Aerial Recovery (`combomods::kh1_blown`).
- **While KH2's controller has him (borrows, Drive Forms)**: each ready record is decided first
  (evaded, reflected, guarded, knocked down, re-hit). A taken hit keeps KH1's damage, but its
  reaction kind (`+0xC` low 5 bits) is cleared, so KH1 plays no reaction. Then KH2's controller
  plays its own (`Character::take_hit`: a flinch for KH1 kind 3, a blow for 4 / 5, KH2's
  default numbers), and the hook replays KH1's rumble, hit tally and hurt voice itself.
- Our own mercy rule: no hit lands during KH2's blow, and the same attacker's hits within 20
  frames don't land during a flinch (`REHIT_GRACE`, `docs/kh1-rehit.md`).

So there are two reaction systems: KH1's in KH1's states, KH2's controller's in borrows and
forms.

## 2. KH1's player hit pass (Steam `0x14029BBD0`)

**Checked (decomp)** unless tagged otherwise. Called once, from `0x1402A43A0` (the player /
party think). Ghidra shows one argument; our hook passes `rdx` too, a vector the reactions
write into (stage 46, Epic findings). The reaction functions below take
`(actor, record, out vector)`.

Per frame, for each ready record popped from the actor's queue (`+0x3EC`, popped by
`0x1402BF160`, freed by `0x1402BF220`):

1. **Skipped** when the record has no effect (`+0` bit `0x20`), the actor's flags have `0x200`
   (carried or grabbed), or the stats' `+0` has `4` (dying).
2. **Second Chance**: walks the equipped abilities (save block `+0x44`, 48 bytes) for ability
   `0x40`. Normal rule: from full HP, a hit of at least his HP becomes HP − 1. A second rule
   applies when the byte at `0x140528ADC` is set (a divisor at `0x140528AE0`, bounded by party
   index ≤ 2 and the record's damage type). What sets that byte isn't known yet.
3. **Damage**: `0x1402BEEE0 (actor, record)` returns the new HP.
   - The damage is **0** while the actor's flags have `0x80` or `0x1000000`, or global
     `0x142D53A38` bit 2 is set.
   - Damage types (`+0xC` bits 6..9): 9 Stop-like (`+0x88 |= 1`), 10, 11 a heal, 12 (`+0x88 |= 8`,
     a timer on the combat state) take their own paths.
   - HP is written with `0x1402A4920 (actor, change)`, which also updates the HUD mirror.
4. **Death**: HP gone, party slot 0 (Sora). Either game over (flag 4 on the stats) or, with world
   flag `0x2000` and a mode test, `0x1403962C0 (4, actor)`. A partner (party index > 0) is flagged
   `+0x88 |= 0x10` (KO'd, handled by `0x14029CA60`).
5. **Kind**: `0x1402BF290` maps the record's kind. Kind 1 becomes 3 / 4 / 5 by the damage's share
   of max HP, against two bytes in btltbl's reaction block (below): under 20% a flinch (3),
   20..29% kind 4, 30% and over kind 5. Kinds 6 / 7 are attack clashes.
6. **Reaction**: runs only if the actor's flags have neither `0x80` nor `0x1000000` and
   `0x142D53A38` bit 2 is clear, or if the kind is 6 / 7. It also runs only if the kind is
   stronger than any reaction already run this pass, so only the strongest hit reacts. The
   function comes from a table picked by movement state (`+0x70`), per kind:

   | State | kind 3 | kinds 4 / 5 | 6 | 7 |
   |---|---|---|---|---|
   | 0, 1, 0xC, 0x1C, 0x20, 0x26, 0x18, 0x19 (ground, water surface) | `0x14029D210` | `0x14029CEC0` | `0x14029D720` | `0x14029D5F0` |
   | 8 (flight) | `0x14029D110` | `0x14029CFD0` | `0x14029D650` | same |
   | 0x11 (swimming) | `0x14029D340` | same | | |
   | 0x1D | `0x14029CE10` | same | | |
   | 0x1E, 0x1F | `0x14029D450` | same | | |
   | any other (air states 2, 3, ...) | `0x14029CD80` | `0x14029CC40` | `0x14029D510` | same |
   | 0x27 | none: `0x140397A50 (10)`, next record | | | |

   The ground, 0x18 and 0x19 rows first call `0x1402B1960` (the state setter, Epic
   `0x1402AF7D0`, mapped).
7. **After the loop**:
   - `0x14029CA60`: the KO handler (death motion `0x43` on the ground, `0x8D` flying, `0x6C`
     swimming; the partner revive timer from btltbl `+0x7C + member`).
   - **Stops here when no reaction ran.**
   - Otherwise, for Sora only: rumble `0x14028F8D0` (1 for a flinch, 0 for 4 / 5, 6 / 5 for the
     clashes), the hits-taken tally `0x14028C660 (0xD)`, and the command record cleared
     (`0x142D53A40` / `+8`).
   - MP from damage: `0x1402A4AE0 (actor, lost HP × n)`, where n is `+0x184` bits 1..3 of the
     combat state (`+0x148`). This is probably MP Rage (**Assumption**).
   - With `+0x88` bit 8: an effect on him (`0x1402ADC20`).
   - The HUD portrait reacts by the HP share left (`0x140268D60`: under 50%, 50..80%, over).
   - A voice: `0x1402901F0 (actor, n, 0)`, n = 0 for a flinch, 1 for a knock-back, 2 for a death,
     and `0x14` ("low HP") when a hit takes Sora from above half HP to half or less.

**What this means for our hook (Checked (decomp)).** In borrows and forms the hook clears the
kind, so step 7 stops before its second half. The hook replays the rumble, the tally and voice
0 / 1, but these are lost:
- KH1's MP from damage
- the `+0x88` bit 8 effect
- the HUD portrait's reaction
- the low-HP voice `0x14`
- KH1's 30-frame no-damage timer (section 3)

## 3. KH1's own mercy after a reaction

**Checked (decomp).** Every reaction function calls `0x140297F30 (actor, 30.0)`. That writes
30.0 to `+0xC` and sets flag `0x80` (clearing `0x100000`). The actor step `0x140297FE0`
handles it each frame:
- First frame (`0x80` without `0x100000`, not dying): sets `0x100000`.
- After that: counts `+0xC` down by the frame step `0x14233FBDC`, and at ≤ 0 clears both flags.

While `0x80` is set, KH1 takes no damage from a record and starts no reaction, except clashes
6 / 7 (section 2, steps 3 and 6). Hits still arrive: effects, records, tallies.

**Assumption:** whether `0x100000` is cleared elsewhere while the reaction motion plays (which
would start the 30 frames at the motion's end) is not checked.

Our `REHIT_GRACE` and "no hit during a blow" rules stand in for this. They exist only because
KH2's reactions run without KH1's timer.

## 4. KH1's reaction functions (the ground ones)

**Checked (decomp).** All of them set the motion through `0x14029F980` (Epic `0x14029D7F0`) and
queue the next one with `0x14029FA30` (Epic `0x14029D8A0`, a blend of 8.0). Every one also sets
the 30-frame timer (section 3).

- **Ground flinch** `0x14029D210`:
  - motion request `0x4A` from the front, `0x48` from behind; `0x4B` / `0x49` when the record's
    `+0x20` is 1 (another hit part, **Assumption**)
  - next motion idle (`0x40000000`); velocity `+0x3C0` zeroed
  - pushed back through KH1's push (`0x1402C7300`): distance = btltbl `+0x20` (750) × damage /
    max HP, capped at btltbl `+0x24` (150), × the stats' `+0xC4` (a push factor)
- **Ground knock-back (4 and 5)** `0x14029CEC0`:
  - motion request `0x4D` from the front, `0x4C` from behind; turns to face the attacker
    (`+0x150`, `0x140297E10`)
  - writes his position to the out vector, zeroes velocity
  - no push and no launch here: the motion carries him (**Assumption**: its root motion or its
    events; the air state 2 that `combomods::recover` checks must come from it)
- **Air flinch** `0x14029CD80`:
  - motion `0x4E` / `0x4F` (+ `0x1000000`)
  - `0x1402AF960 (actor, 20.0)` (a hop, **Assumption**)
  - `0x14029D800` sets the air velocity `+0x3C0`
- **Air knock-back** `0x14029CC40`:
  - motion `0x50` / `0x51`, faces the attacker
  - a launch `0x1402B7F20 (actor, 20.0, dir)`: hop 20, horizontal speed btltbl `+0x3C` (16)
    along the push direction into `+0x3C0`
- **Flight** `0x14029CFD0` / `0x14029D110`: motions `0x50` / `0x51` / `0x4E` / `0x4F` with
  next `0x40000075` (flight idle), push as the ground flinch.
- **Swimming** `0x14029D340`: motion `0x6A`, next `0x4000002D`, push as the ground flinch.

**KH1's push** (`0x1402C7300` sets it, the physics step's `0x1402C7070` runs it; Checked
(decomp)):
- direction at `+0x3F0`, speed `+0x3FC` = distance × 0.22314 per frame step
- each step moves by speed × step, then speed × 0.8^step
- ends below 1 (clears `+0x88` bit `0x40`)
- in total about the distance (0.22314 = −ln 0.8)

**btltbl's reaction block** (loaded at `0x142D22D40` by `0x14028F9D0`, block pointer
`0x142D22D30` = file + 0x58; read from `local/re/kh1cmp/orig/btltbl.bin`; Checked (files)):

| Offset | Value | Use |
|---|---|---|
| `+0x10` | 20 | flinch threshold, % of max HP |
| `+0x11` | 30 | kind 5 threshold, % of max HP |
| `+0x20` | 750.0 | flinch push per max HP |
| `+0x24` | 150.0 | flinch push cap |
| `+0x30` | 40.0 | air hit fields, used by `0x14029D510` / `0x1402B7F80` |
| `+0x34` | 8.0 | air hit fields, used by `0x14029D510` / `0x1402B7F80` |
| `+0x3C` | 16.0 | air knock-back speed |

**Assumption:** the file is loaded raw at that address, so file offsets match.

## 5. KH2's reaction routine (KH2 Steam decomp)

**Checked (decomp)**, KH2 Steam exe:

- `0x1403CE980` folds the ATKP reaction byte into groups:
  - 1 and 0xC → 1
  - 3, 6, 8, 10, 0xD, 0x10, 0x12 → 3
  - 4, 7, 9, 0xB, 0xF, 0x11 → 4
  - the rest stay as they are

  Exactly as our `character_core::reaction::category`.
- `0x1403D2FE0 (object, hit)` is the generic reaction (33 callers: player, enemies, partners).
  The hit's reaction byte is at `+0x26`. Its switch:
  - 1: push
  - 2: flinch (`0x1403D43B0`)
  - 3: ground flinch + push (`0x1403D43B0`, `0x1403D4960`), or air flinch (`0x1403D3840`,
    `0x1403D4800`)
  - 4: launch (motion 0x10 from the front, 0x12 from behind, through `0x1403C86A0`; speeds from
    `0x1403CEA00`, sstm `+0x24` defaults, or the ATKP's `+0xC` / `+0xE`)
  - 5: Magnet (`0x14040FBF0`)
  - 6..0xE: special ones
- The hit-back distance is `0x1403CEAB0` (the ATKP's `+0xC` unless 0x7FFF, else the defaults).
- The air flinch's motions are read from the table at `0x1405C63F8`: u16 14, 15. **Checked
  (exe).**
- These match `docs/combat-model.md` "Hit reactions" and our `reaction.rs` (ported from the
  Epic exe). The rest of `docs/combat-model.md`'s numbers wasn't checked again here:
  - sstm `hit_back` 200, `blow_up` 24, `blow_speed` 40
  - the push at 0.1 × distance, × 0.9 per frame
  - the blow's 0.8 brakes and gravity 20/49
- Aerial Recovery / Retaliating Slash: Epic `0x1403D6030` in `docs/aerial-recovery.md`. Its
  Steam counterpart is likely `0x1403D60C0`, a caller of the fold function, same size 0x106 as
  `0x1403D5370` (**Assumption**: not read).
- **Not found this round:** KH2's own rule against hits landing during the player's damage
  motions (a lock through motions 10..39). We have no Steam address for it yet.

**The two engines side by side:**

| | KH1 | KH2 |
|---|---|---|
| Groups | 3 flinch, 4 / 5 knock-back (by damage share) | ATKP groups 1..4 |
| Push numbers | 750 × damage / max HP, cap 150 | the ATKP's own, or sstm 200 |
| Push curve | 0.223 / 0.8 | 0.1 / 0.9 |
| Ground knock-back | a knock-down motion in place, no launch | a real launch (blow physics) for any group-4 hit, then 17 / 19 |
| Air hits | hop 20 and 16 sideways | air flinch hop 50, drift = hit-back × 0.048; blow |

KH1's enemy attacks carry only KH1's data. They have no ATKP knock-back values, so KH2's
per-attack numbers don't exist for them.

## 6. What reads KH1's reaction (don't break)

**Checked (decomp)** unless tagged:
- the hits-taken tally `0x14028C660 (0xD)` (journal / event scripts read tallies)
- the rumble, the HUD portrait, the voices (incl. low HP), MP from damage
- KO / game over (`0x14029CA60`, stats flag 4)
- KH1's action flags for "can act" (`0x140297870`: flags `4`, `+0x88 & 0x20`, `+0x444 & 1`)
- our `combomods::recover` / `launched` (Aerial Recovery on KH1's knock-back, which assumes the
  knock-back puts him in air state 2 and ends with action flag 4, **Assumption**, t475)
- the script-hold flag `0x1000000`: no reaction during events, which must stay

## 7. Plan (native)

KH1's hit pass and its reaction table stay the only reaction system. KH2's motions and numbers
go into it. One system per version:

1. **KH2's reaction motions on KH1's reaction requests.** Using `kh2slots`' baked-slot machinery:
   - ground flinch `0x4A` / `0x48` (and `0x4B` / `0x49`) → KH2 10 / 12 (11 / 13)
   - ground knock-back `0x4D` / `0x4C` → KH2's blow 16 / 18 with 17 / 19
   - air flinch `0x4E` / `0x4F` → 14 / 15
   - air knock-back `0x50` / `0x51` → 16 / 18
   - KO `0x43` → KH2's

   Retimed to KH2's lengths, with KH2's reaction-command window as KH1's "may be left".
   KH1's timer, tallies, HUD, voices and MP stay KH1's because KH1 runs them.
2. **KH2's push on KH1's push.** In the reaction functions (a hook on the table's entries for
   Sora), set KH2's hit-back distance and its 0.1 / 0.9 curve instead of KH1's 750 × share /
   0.223 / 0.8. KH1's decay of 0.8 is a constant in `0x1402C7070` shared by every actor, so it
   needs a Sora-only path there.
3. **KH2's blow as a KH1 state.** KH1 has no launch for Sora on the ground. Register a new
   movement state through KH1's state registration (`0x1402B1B80`, as KH1 does for states
   9..11 in `0x1402C73A0`) with KH2's blow physics: up 24, 40 sideways, brakes 0.8, gravity
   20/49, the landing into 17 / 19. KH1's kind 4 / 5 would enter it from both the ground and
   air rows. Aerial Recovery and Retaliating Slash (t475) then read this state instead of
   KH1's knock-down.
4. **Borrows and forms use the same pass.** Stop clearing the kind. When a hit is taken while
   KH2's controller has him, hand Sora to KH1's states for the reaction, as KH1's own commands
   do, and drop `Character::take_hit` from `hurt.rs`. KH1's 30-frame timer then replaces
   `REHIT_GRACE` and the blow lock. The pre-pass checks stay: invulnerable, Reflect, guard,
   clash, all marking records no-effect. The replayed rumble / tally / voice go.

**Performance.** No new per-frame work: the hit pass hook already exists and works only when
records are ready. Version 4 removes one parallel system: KH2's reaction state on the
controller. Version 2's hook runs once per hit. Version 3 adds one KH1 state's step, run only
while launched.

## 8. Open

- KH2's damage lock for the player during reactions: not found in the Steam decomp yet.
- Whether a KH1 kind 4 is meant as KH2's flinch or blow. KH1 picks it by damage share, and its
  attacks have no KH2 group. Stage 85 (old) made 4 a blow.
- Where KH1's ground knock-back motion takes him into the air (motion events or root motion).
- `0x140528ADC`, the second Second Chance rule.
