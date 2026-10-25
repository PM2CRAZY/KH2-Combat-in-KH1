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

**Checked (decomp)** (§10): the count only starts once the action flag `4` is clear, so the 30
frames run from the reaction motion's end.

Our `REHIT_GRACE` and "no hit during a blow" rules stand in for this. They exist only because
KH2's reactions run without KH1's timer.

## 4. KH1's reaction functions (the ground ones)

**Checked (decomp).** All of them set the motion through `0x14029F980` (Epic `0x14029D7F0`) and
queue the next one with `0x14029FA30` (Epic `0x14029D8A0`, a blend of 8.0). Every one also sets
the 30-frame timer (section 3).

- **Ground flinch** `0x14029D210`:
  - motion request `0x48` when `dot(facing, the record's push direction +0x10)` is below 0 (hit
    from in front), `0x4A` otherwise (from behind); `0x49` / `0x4B` when the record's
    `+0x20` is 1 (another hit part, **Assumption**)
  - next motion idle (`0x40000000`); velocity `+0x3C0` zeroed
  - pushed back through KH1's push (`0x1402C7300`): distance = btltbl `+0x20` (750) × damage /
    max HP, capped at btltbl `+0x24` (150), × the stats' `+0xC4` (a push factor)
- **Ground knock-back (4 and 5)** `0x14029CEC0`:
  - motion request `0x4C` from in front (the same test below 0), `0x4D` from behind; turns to
    face the attacker (from behind: away from him)
    (`+0x150`, `0x140297E10`)
  - writes his position to the out vector, zeroes velocity
  - no push and no launch here: the motion carries him (**Assumption**: its root motion or its
    events; the air state 2 that `combomods::recover` checks must come from it)
- **Air flinch** `0x14029CD80`:
  - motion `0x4E` from in front, `0x4F` from behind (+ `0x1000000`)
  - `0x1402AF960 (actor, 20.0)` (a hop, **Assumption**)
  - `0x14029D800` sets the air velocity `+0x3C0`
- **Air knock-back** `0x14029CC40`:
  - motion `0x50` from in front, `0x51` from behind, faces the attacker
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

## 9. Step 1: KH2's reaction motions on KH1's requests (done)

The Config menu's Combat list has a **Hit Reactions** row (KH1 / KH2; ini `hit_reactions`,
default `kh1`, `styles::Kind::HitReactions`). At KH2, base Sora on KH1's states plays KH2's
reaction motions on KH1's own reaction requests, through the baked slots (`kh2slots::REACTIONS`).
KH1's hit pass and its reaction functions run as before, so its no-damage timer, push, idle /
fall queued after, rumble, tally, HUD portrait, MP from damage and voices all stay KH1's. At KH1
nothing changes.

- **Which way he was hit (Checked (decomp)).** KH1's flinch picks `0x48` when
  `dot(facing, record +0x10) < 0`, else `0x4A`; KH2's routine (`0x1403D2FE0`) calls a hit "from
  the front" on the same test (`character_core::reaction::respond`). So `0x48` = KH2's 10
  (front), `0x4A` = 12 (behind). The earlier note in §4 had them the other way round; corrected.
- **Requests and slots (Checked (files)).** KH1 `xa_ex_0010.mset`'s request table: `0x48..0x4B`
  slots 61..64 (70 frames, "may be left" at 46), `0x4E` / `0x4F` slots 67 / 68 (30 frames).
  KH2 `P_EX100.mset`: 10..13 70 frames, free state (range id 2) from 50, a reaction-command
  window (range id 14) from 8; 14 / 15 36 frames, no free state.

  | KH1 request | KH1 slot | KH2 motion |
  |---|---|---|
  | `0x48` ground flinch, front | 61 | 10 |
  | `0x49` the same, other part | 62 | 11 |
  | `0x4A` ground flinch, behind | 63 | 12 |
  | `0x4B` the same, other part | 64 | 13 |
  | `0x4E` air flinch, front | 67 | 14 |
  | `0x4F` air flinch, behind | 68 | 15 |

- **When he may act again.** KH2's free state (frame 50) is KH1's "may be left" on the flinch slots
  (KH1's own was 46). The air flinches have none in either game; KH1 queues its fall when the slot
  ends (`0x40000006`).
- **Voices.** KH2's clips start voice 29 / 30 at frame 0 (Checked (files)). Left out: KH1's pass
  already plays its own hurt voice group 0 / 1 (`0x1402901F0`). With Battle Quotes at KH2 that
  call says KH2's line instead (§14).
- **Not in this step:** KH1's knock-backs `0x4C` / `0x4D` (slots 65 / 66, 72 frames, in place) and
  `0x50` / `0x51`. KH2 plays a launch there (16 / 18, then 17 / 19 when he lands), which needs
  KH2's blow (step 3). Sora has no KO request: `0x43` is `0xFFF` in his table (Checked (files)). His
  HP gone is KH1's game over (`0x14029BBD0`, Checked (decomp)); `0x14029CA60`'s death motion is
  the partners'.

## 10. KH2's rule against re-hits while reeling (question 3)

**Checked (decomp), KH2 Steam:**
- `0x1403CF600`, the hit admission: a hit counts if it is a Cure kind (ATKP kind 5 / 6), or its
  attack's bit is clear and one of these holds: its group is the target's last group, its group is
  `0xFF`, or the target's group timer (`+0x2C` of the record) is at or below 0. A hit that counts
  on an object with `+0x6C8` bit 2 (the player) sets the timer to 30.
- `0x1403CF230` / `0x1403CF4B0`: each frame the timer counts down by the frame step
  (`0x140717480`), except while the object plays a reaction motion (`0x1403D4880`: its motion
  `+0x180` is 10..39).

**Checked (decomp), KH1 Steam:** every reaction sets flag `0x80` and 30.0 at `+0xC`
(`0x140297F30`). The actor step (`0x140297FE0`) starts the count only once the action flag `4`
is clear, then counts the 30 down by the frame step. Flag `4` is the reaction's action flag: KH1's
set-motion (`0x14029F980`) sets it for a request with `0x1000000`, as every reaction's is
(Checked (decomp)). So the count starts at the reaction's end. While `0x80`
is set, no damage and no reaction but a clash. (This answers §3's assumption.)

**Comparison.** The timing is the same: 30 frames after the reaction motion ends. The one
difference is KH2's group rule: hits from the same group as the last one still land and react.
KH1's enemy attacks carry no KH2 group (no ATKP), so that rule has nothing to compare. The
mod keeps KH1's timer.

## 11. Step 2: KH2's push on KH1's push (done)

`hitreact.rs`. With Hit Reactions at KH2 (base Sora on KH1's states), after KH1's hit pass started
a reaction:

- **Ground flinch** (`0x48..0x4B`): KH1's flinch has just started its push (`0x1402C7300`: speed
  `+0x3FC` = distance × 0.22314, flag `+0x88` bit `0x40`). The speed becomes KH2's start, 0.1 × the
  distance (`0x1403D4960`, Checked (decomp) KH2). Before each of Sora's KH1 physics frames
  (`takeover::physics`, before the original step), KH1's push field holds KH2's speed for that
  frame. KH1's push step (`0x1402C7070`) moves him by it with his own collision. KH2's decay
  (×0.9 per frame step, stop below 1: `0x1403D4570`, Checked (decomp)) is kept on our side, and
  KH1's flag is kept on until KH2's speed is below 1. If KH1 starts a push of its own meanwhile
  (its field isn't what its step leaves of ours), ours is dropped.
- **Air flinch** (`0x4E` / `0x4F`, KH1's air state 2): KH1's hop (20, `0x1402AF960`) and drift
  (`0x14029D800`: min(40 × share, 8) × push factor along the push) are replaced through
  `shared::physics::kh2_hop` (KH2's arc at physics KH2, KH1's gravity at KH1).
  KH2's own air flinch (`0x1403D4800`, Checked (decomp)) is a hop of 50 and a drift of the hit-back
  distance × sstm `+0x38` (0.048).
- **Frame step.** KH1's step (`0x14233FBDC`) and KH2's (`0x140717480`) are both 1.0 per 60 Hz
  frame, so the curves are the same per frame (Checked (decomp): both decays are `powf(k, step)`).

**Knock-back Numbers** (Config > Combat, three values; ini `knockback_numbers`, default `kh2`):

| | Ground push | Air flinch |
|---|---|---|
| KH1 | KH1's own distance (750 × share, cap 150, × stats `+0xC4`) | KH1's hop 20, KH1's drift |
| Scaled | 200 × KH1's distance / its cap (btltbl `+0x24`) | hop 50, 9.6 × KH1's drift / its cap (btltbl `+0x34`) |
| KH2 | sstm's 200 | hop 50, drift 200 × 0.048 = 9.6 |

KH2 is the default because it is KH2's own rule for an attack that gives no knock-back number:
`0x1403CEAB0` falls back to sstm's (Checked (decomp)), and KH1's attacks give none. btltbl's block
pointer is Steam `0x142D22D30` (read as `0x14029D210` reads it, Checked (decomp)); its values from
the file (Checked (files)).

**Not covered:** KH1's flight and swimming flinches (`0x14029CFD0`, `0x14029D110`, `0x14029D340`)
still use KH1's push numbers; the hook only acts on the ground and air flinch requests.

## 12. Medium Hits (question 2) (done)

Config > Combat > **Medium Hits** (Blow / Flinch; ini `medium_hits`, default `blow`).
KH1's middle kind (4: a hit of 20..29% of his max HP; btltbl `+0x10` = 20, `+0x11` = 30) is
mapped from kind 1 by `0x1402BF290` inside the pass. Any other kind passes through unchanged
(Checked (decomp)).

- **On KH1's states with Hit Reactions KH2, at Flinch:** before KH1's pass, each ready record KH1
  would map to 4 gets kind 3 (`hitreact::medium_hits`). KH1's pass then runs its flinch for it:
  KH2's flinch motion (step 1), KH2's push (step 2), KH1's no-damage time, the light rumble and
  voice. A kind 5 in the same pass still wins (KH1's strongest-hit rule). Aerial Recovery's note
  of a KH1 knock-back (`combomods::kh1_blown`) doesn't see those hits.
- **In KH2's borrowed moves** (base Sora only, not Drive Forms): KH2's controller plays a flinch for
  4 instead of a blow, with KH1's flinch rumble and voice.
- **At Blow:** as before. On KH1's states that is KH1's own knock-down (`0x4C` / `0x4D`) until
  KH2's blow is in (step 3); in a borrow, KH2's controller's blow.
- At Hit Reactions KH1 the row does nothing on KH1's states.

## 13. Step 3 groundwork: KH1's movement state table (not built yet)

**Checked (decomp)**, KH1 Steam. `0x1402B1B80 (id, flags, a, b, c, d, e, f, g)` writes state `id`'s
0x40-byte entry at `0x142DB3840 + id × 0x40`: `+0` the flags (u32), `+8` a, `+0x10` b, `+0x18` c,
`+0x20` d, `+0x28` e, `+0x30` f, `+0x38` g. `shared::physics` already hooks `+0x18` (the per-state
input / pre-step) and `+0x20` (the movement step) of states 0..2. Registrations found (id: flags):
0: 0xFC, 1: 0x34, 2: 0x3D, 3: 5, 4: 2, 5: 2, 7: 1, 8 (flight): 0x76, 9..0xB: 0x101, 0xC: 0,
0xD / 0xE: 1, 0xF: 1, 0x11 (swimming): 1, 0x12: 0, 0x15 (glide): 5, 0x18: 0x7E, 0x19: 0xE,
0x1A: 5, 0x1B: 0, 0x1C: 4, 0x1D: 0x1E, 0x1E: 0x18, 0x1F: 0x19, 0x20: 4, 0x21: 6, 0x22: 1, 0x26: 4,
0x28: 0x100, 0x32: 2. Ids 6, 0x10, 0x13, 0x14, 0x16, 0x17 and 0x23..0x25 had no registration in the
callers read. **Assumption:** that they are free (other callers may register them), and what each
flag bit means.

KH1's hit pass (§2) runs the "any other state" reactions for a state it has no row for, so a new
blow state takes KH1's air reactions. Step 3 would register KH2's blow there (KH2's arc: up 24,
40 sideways, brakes 0.8 down to 0.1 of the start, gravity 20/49, fall cap 16;
`character_core::reaction::Blow`), moved through KH1's mover, with motion 16 / 18 and 17 / 19 on
landing, entered from KH1's knock-back reactions (`0x14029CEC0` on the ground, `0x14029CC40` in the
air) for Sora.

## 14. KH2's hurt line with Battle Quotes KH2 (the tester's m2 answer 1: "(b)")

**What KH2 says (Checked (files)).** KH2's reaction motions say their line from frame trigger 14
(a VTBL battle voice) at frame 0 (`P_EX100.mset`):

| KH2 motion | voice | | KH2 motion | voice |
|---|---|---|---|---|
| 10 ground flinch, front | 29 | | 14 air flinch, front | 29 |
| 11 alternate | 30 | | 15 air flinch, behind | 30 |
| 12 ground flinch, behind | 29 | | 16 / 18 blow, front / behind | 1 |
| 13 alternate | 30 | | 17 / 19 (blow landings) | none |

`00battle.bin`'s VTBL rows for Sora (character 1): 29 is clip 6, 30 clip 5, 1 clip 7, each the
only choice, priority 3. The same motions also carry frame trigger 31 at frame 0 (params 3 on the
flinches, 4 on the blows); what 31 does isn't known (**Assumption**: not a voice).

**Where KH1 says its line (Checked (decomp)).** The last call of KH1's hit pass `0x14029BBD0` is
the voice group `0x1402901F0 (actor, group, 0)`: group 2 when his HP is gone, 0x14 (low HP) when
the hit takes the party leader from above half HP to half or less, else 0 for a flinch (kind 3)
and 1 for a knock-back (kind 4 / 5); clashes 6 / 7 say nothing. It returns nothing. By then the
reaction function has queued the reaction's request (actor `+0x178` count, `+0x17C` request).

**What the mod does.** With Battle Quotes at KH2, `hitreact::hurt_line` (run from the voice
group's hook, `soundlog::voice_group`, only while Sora's hit pass on KH1's states runs) turns
groups 0 / 1 into KH2's voice for the reaction just asked for: 1 → voice 1; 0 → voice 30 for
requests `0x49` / `0x4B` / `0x4F` (KH2's 11 / 13 / 15), else 29. It goes through KH1's own voice
path (`kh2sound::hurt_voice` → `kh1bank::voice`, as every KH2 battle quote does); if KH2's voice
isn't loaded, KH1's line is said. The low-HP (0x14) and HP-gone (2) lines stay KH1's (KH2's
reaction motions have none). The rule follows the Battle Quotes row whatever the Hit Reactions
row says, so with Hit Reactions at KH1 KH1's reaction motion plays with KH2's line
(**open**: the tester may want it only at Hit Reactions KH2).

In KH2's borrowed moves (`hurt`'s other branch) KH2's controller plays KH2's reaction motion,
whose frame trigger 14 already says KH2's line through `kh2sound::event`; the hook's replay of
KH1's group 0 / 1 is now skipped then (it doubled the line).

## 15. KH2's air flinch turn (the tester's m2 answer 2: "(a)")

**KH2 (Checked (decomp), Steam KH2 `0x1403D3840`).** The air flinch takes the hit's push
direction (the vector copy `0x1401A8E60`; Ghidra drops its source argument, read as the hit's
direction: **Assumption**, matching the OG rewrite's `reaction::respond`), negates it when
dot(facing, push) < 0 (hit from the front), and sets it as the object's facing (`0x1403B6420`) at
once, then plays 14 (front) / 15 (behind). So he faces the attacker when hit from the front and
faces away when hit from behind. KH2's ground flinch (`0x1403D43B0`) doesn't turn him (Checked
(decomp)), so only the air flinch does.

**KH1 (Checked (decomp)).** KH1's air flinch `0x14029CD80` doesn't turn him; its knock-back
`0x14029CEC0` turns him by the same rule, with `atan2(x, z)` of the (negated) push direction into
KH1's facing set `0x140297E10 (actor, angle)` (`+0x30` = sin, `+0x38` = cos, `+0x3C` the angle;
skipped while `+0x374` bit `0x1000` holds his facing) and `+0x150`.

**What the mod does.** With Hit Reactions at KH2 (base Sora on KH1's states), after KH1's air
flinch (requests `0x4E` / `0x4F`, his air state 2), `hitreact::air_flinch_turn` makes that same
turn the way KH1's knock-back makes it: the push direction is the one KH1's air flinch just gave
his air velocity (`0x14029D800`: record `+0x10` × the drift); `0x4E` (front) faces him along its
negation, `0x4F` (behind) along it. If KH1 gave him no drift (a push factor of 0), the direction
isn't known and he isn't turned. Any Knock-back Numbers value. Steam only. At Hit Reactions KH1
nothing changes.

## 16. Flinches while flying or swimming (the tester's m2 answer 3: "(b)", KH1's)

KH1's flight and swimming reactions (`0x14029D110` / `0x14029CFD0` flight, `0x14029D340`
swimming, Checked (decomp)) keep KH1's numbers: KH2's push (§11) acts only on the ground flinch
requests in KH1's ground reaction, the air flinch hop / drift and turn (§11, §15) only in his
air state 2 (the flight flinch asks `0x4E` / `0x4F` too, from state 8, and is skipped by that
test). Two KH2 parts still reach them (**open**, asked): the flight flinch's `0x4E` / `0x4F`
play KH2's air flinch motions 14 / 15 (the slots are by request, `kh2slots::REACTIONS`), and the
Medium Hits row's kind 4 → 3 also applies in flight and swimming.
