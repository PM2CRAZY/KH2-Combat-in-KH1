# KH2's recoil, clash and parry on KH1

Research for the thread "KH2 recoil and parry system" (started from Sora's hit reactions native).
Steam addresses only. Tags: **Checked (decomp)** the Steam Ghidra decomp, **Checked (exe)** bytes
read from the Steam exe, **Checked (files)** the user's KH2 files, **Checked (trace)** a tester's
log, **Checked (code)** this repo's code, **Assumption** not verified.

## 1. What KH2 does

### 1.1 Which attacks meet (Checked (decomp) + (exe))

- `0x1403D08E0` (and its twin `0x1403D0D30`) puts each live attack into the frame's list and tests
  it against every attack already there. A pair meets when:
  - the two are on different sides (attack `+0x38`);
  - the clash table at `0x1405C5E98` (7 x 7 bytes, indexed `[listed kind * 7 + new kind]`, ATKP
    kind at `+4`) allows it;
  - neither attack nor its owner is already immune to the other (`0x1403CF770` on the attack's
    list `+0x3C` and on the owner's list `+0xC98`);
  - the shapes overlap (`0x1403C94C0`);
  - each passes its angle check `0x1403D1F40`: with an ATKP reflect range (`+0x22`, degrees) the
    other attack must come at the owner's front within it, turned by reflect angle (`+0x23` x 2
    degrees). Range 0 means any direction.
- The table, read from the exe (rows: listed attack's kind; columns: the new one's):

  | kind | 0 normal | 1 pierce | 2 guard | 3 special guard | 4..6 |
  |---|---|---|---|---|---|
  | 0 normal | yes | no | yes | yes | no |
  | 1 pierce armor | no | no | no | yes | no |
  | 2 guard | yes | no | no | no | no |
  | 3 special guard | yes | yes | no | no | no |
  | 4..6 | no | no | no | no | no |

  It is the same as `character_core::reaction::clashes`. A pierce-armor attack (Guard Break)
  meets only special guards: it goes through normal attacks and plain guards.

### 1.2 What a meeting does to each side (Checked (decomp))

For each side, `0x1403D2140 (own, other)`:
- counts the meeting on its own attack (`+0xB0`);
- makes the attack and its owner immune to the other attack, unless the other is a guard (kind 2
  or 3) with refact self 2;
- stops its own attack (flag `0x200`) unless its own ATKP refact self (`+0x13`) is 2;
- calls the owner's clash reaction (its class's vtable `+0xD0`), unless its attack has flag `0x100`;
- tells the attack's script (`+0xA4`) with event 3.

`0x1403D10C0` plays the clash effect from the attack's ATKP `+0xB`.

### 1.3 Sora's clash reaction, `0x1403D61D0 (player, own attack, other attack)` (Checked (decomp))

- Sets `+0x120` bit `0x2000000`.
- If his attack's refact self is 2, or the other's refact other (`+0x14`) is 2: clash effect 0xD
  only. No motion, no push, and the move goes on.
- If he's on the ground (`0x1403BA9A0` false), his object entry's `+0xC` bit 1 is clear,
  `+0x18C` bit 6 is clear, and either value is nonzero: the **guard repel**.
  - clash effect 0xB / 0xC (by `+0x6C8` bit 7)
  - faces the clash
  - pushed back 200 (`0x1403D4960`)
  - motion 0x2A (42)
  - `+0xD70` = 80.0
  - his attack marked `+8 | 0x40`
- Otherwise the **recoil**:
  - clash effect 0xD
  - `0x1403D4650` with motion -1 and distance -1 (50)
  - the combo counter `+0xD74` less one, if above 0

`0x1403D4650 (actor, attack, other, motion, distance)`:
- On the ground: motion 41 (0x29) unless one is given, pushed back the distance (50) along minus
  his facing.
- In the air: motion 43 (0x2B), motion mode 4, and the hop `0x1403D4800`: 50 up, and
  distance x `sstm` hit_back_to_jump (`[0x142AE5760] + 0x38`) sideways, back from his facing.
- His attack marked `+8 | 0x40`.

The wall hit `0x1403D54B0` (vtable `+0xD8`) is the same recoil with motion 0, and also takes one
off the combo counter.

`+0xD74` is the combo counter: `0x1403D6CA0` raises it in the combo window, and `0x1403D7480`
reads it capped by the moveset's maximum. So **a recoil takes one swing off the combo, as KH1's
does** (§2.3).

### 1.4 The 80 at `+0xD70`: the guard repel's protection (Checked (decomp); one Assumption)

- Set to 80.0 by the guard repel (`0x1403D61D0`, and `0x1403D6AD0`, a second repel start).
- Counted down by the frame step `DAT_140717480` in `0x1403D4CC0`. The file holds 1.0.
  **Assumption:** it is still 1.0 a 60 Hz frame at run time (the step is a global the game may
  rewrite), which makes it 80 frames, 1.33 s.
- While it is above 0, the player's vtable `+0xB8` (`0x1403D65A0` / `0x1403D5310`) reports him
  protected. KH2's damage intake (`0x1403D5370` / `0x1403D60C0`) then drops every attack except
  kinds 5 and 6 (the cures). `0x1403D1730` also makes the hit record a no-damage kind (4 in
  place of 1).
- So **after a guard repel Sora can't be hurt for 80 frames.** Our repel doesn't do this.

### 1.5 The enemy's side: the parry (Checked (decomp))

An enemy's clash reaction (`0x140410700`, vtable `0x1405CF068`; its twin is at `0x140410870`):
- records the clashing attacker (`+0xD38`);
- sends its script event 0x13;
- if its attack's refact self isn't 2 and the ATKP has a **reflected motion** (`+0x15`), plays
  that motion through `0x1403D4650`, pushed back by **reflect hit-back** (`+0x16`; -1 gives 50);
- if the ATKP has a **reflect action** (`+0x18`), starts it;
- marks the attack `+8 | 0x40`.

So in KH2, a swing meeting an enemy's normal attack:
- stops it;
- makes the enemy immune to that hit (so it can't land on Sora);
- staggers the enemy into its reflected motion when it has one;
- gives Sora his recoil (41 / 43).

That is the parry.

### 1.6 Sora's ATKP rows (Checked (files), the user's `00battle.bin`)

| ATKP | move | kind | refact self | refact other | reflect range |
|---|---|---|---|---|---|
| 126..134 | plain swings and finishers | 0 | 0 | 0 | 0 |
| 135, 183 | (reflected motion 140, hit-back -1) | 0 | 0 | 0 | 0 |
| 184 / 185 | | 0 | 0 | 2 / 1 | 0 |
| 186, 189, 190 | | 0 | 2 | 1 | 0 |
| 161 | guard press 173 | 2 | 1 | 0 | 120 |
| 162 | guard repel 42, Guard Break 165's own guard | 2 | 2 | 0 | 180 |
| 406 | Aerial Dodge / Flash Step body guard | 2 | 2 | 0 | 0 |
| 124 | Aerial Recovery / Counterguard body guard | 3 | 2 | 0 | 0 |
| 876 | Guard Break's hit | 1 | 2 | 1 | 0 |
| 1581 | Counterguard's hit | 1 | 2 | 1 | 0 |

So Guard Break and Counterguard meet only special guards, and are never stopped.

## 2. What KH1 does (Checked (decomp), Steam KH1)

### 2.1 The meeting

- The collision pass `0x1402B3BE0` tests attack against attack when:
  - neither has already met the other (masks `+0x54` / `+0x9C`);
  - KH1's pairing table at `0x140528CB8` (indexed by type, flags bits 17..19) has the other
    type's bit.
- Types 1 to 4 meet no bodies (pure guards; type 3 still does).
- Type 6 and 7 meet no attack: no entry of the table has their bit. They still hit bodies.
- `0x1402B4A20` (the meeting) runs both halves, then KH1's spark `0x1402AE650` (10, or 11 for a
  big gap).

### 2.2 One side, `0x1402B4BC0 (p, q)`

The two only meet if p hasn't clashed yet, one priority (`+0x68`) is under 0x62, and q's priority
isn't 0. Then:
- p is a guard (priority 100): 0x10, nothing more. The guard is unhurt.
- p wins by priority and is directional (types 1 / 3): q's side gets a record (`0x1402BF630`).
- p is under 0x62:
  - a melee attack: a clash record on its owner (`0x1402BF060`), kind 6, or kind 7 when q is
    more than 40 stronger;
  - a shot: turned back (team swapped, owner taken over).
  - Either way it is marked clashed (`+4` 0x10 / 0x20) and spent (`0x100000`).
  - If q's owner is Sora and q is a melee attack: KH1's guard tally (`0x14028C660 (10)`) and an
    MP bonus from p's `+0xB0`.
- p is 0x62 or more: 0x10, not stopped.

So KH1 is **by priority**, and both sides recoil whenever both are under 0x62.

### 2.3 Sora's recoils

| | ground | air |
|---|---|---|
| kind 6 | `0x14029D720`: request 0x6E (slot 72), pushed back by the gap (btltbl `+0x1C`) | `0x14029D510`: request 0x70 then the fall, a hop of 20, pushed back by the gap (btltbl `+0x30`, capped `+0x34`) |
| kind 7 | `0x14029D5F0`: request 0x6F (KH1's big recoil), no push | as kind 6 |

- Each one takes one off KH1's combo count (combat state `+0x41`), the same as KH2's `+0xD74`
  (§1.3).
- Each one starts KH1's no-damage timer with **8 frames** (`0x140297F30 (actor, 8.0)`). KH1's
  flinches use 30. KH2's recoil gives none.

## 3. What we do now (Checked (code) unless marked)

### 3.1 KH1's own states (the native slots)

- Plain swings and our Keyblade ability rows hit through KH1's own row attack object: one sphere on
  the blade, with the KH1 template row's priority and clash type. KH1's own pass meets enemy
  attacks with them (§2).
- So a parry happens, but by KH1's priority rule. Sora plays **KH1's own recoil animations** (0x6E,
  0x6F, 0x70), with KH1's push and KH1's 8 frames of protection. These are not KH2's 41 / 43 with
  their push of 50 and hop of 50.
- Whether the enemy staggers is KH1's: a kind 6 / 7 record for its script, and only when its
  attack is under 0x62. An attack of 0x62 or more is never stopped by a swing (KH1's behaviour too).
- Guard Break (`kh1rows`, its row's priority raised to `GUARD_THROUGH`, 0x62):
  - it passes plain guards (priority 100), as KH2's pierce does;
  - but it still meets every enemy attack under 0x62, and gives that enemy a kind-7 record (KH1's
    big recoil) without stopping itself. KH2's pierce meets no normal attack at all (§1.1).
- Moves with refact self 2 (Counterguard, ATKP 186 / 189 / 190) still use the template's priority.
  KH1 stops them and recoils Sora; KH2 doesn't.
- Body-hit moves (`takeover::kh2_hits`: Explosion, Vicinity Break, Magnet Burst) go through
  `Strikes::strike_as`:
  - an enemy guard they meet only marks the swing repelled and calls `register_wall_hit` on the
    native driver's own controller, which doesn't drive Sora, so he plays no recoil;
  - enemy attacks are met with a temporary attack object made from the plain swing row's data.
- Our guard repel (`combomods::guard_repel`) has KH2's motion 42 and 200 push, but not the 80
  frames of protection (§1.4).

### 3.2 KH2's controller (the borrows)

- `Strikes::strike_as` takes `h.weapon == 0` as "the right Keyblade" for the KH1 blade sweep. In
  KH2's data, weapon 0 is the **body**, 1 the right Keyblade and 2 the left (`kh2_rig::attack_shapes`,
  `kh2_data::sora`).
- So borrowed Keyblade swings:
  - meet guards and walls with KH2's wide shapes, not KH1's blade (the tester logs' "repelled by the
    guard ... (KH2's shapes)" lines, **Checked (trace)**);
  - **never meet an enemy's melee attack**. The melee filter is `s.free || on_blade.is_some()`, so
    only shots reach them: the logs show only "a shot" meetings, **Checked (trace)**.
- Body hit windows get the blade sweep instead.
- Enemy guards repel any borrowed swing whose KH1 *plain swing row's* clash type the table lets
  meet, whatever the KH2 ATKP kind. A borrowed Guard Break is repelled by guards.

### 3.3 Why "you can't parry enemy attacks"

- In borrowed moves the swing never meets an enemy's melee attack (§3.2). This is a bug: the weapon
  index.
- In KH1's states the parry is KH1's rule and KH1's animation (§3.1). It isn't KH2's, and against
  any enemy attack of priority 0x62 or more it can't happen.
- **Assumption:** which of the two the tester saw. No log or trace from the Guard Break test is in
  the thread.

## 4. Native plan (one system per version)

1. **Weapon index in `strike_as`.** The KH1 blade sweep goes on weapon 1 (the right Keyblade).
   - Borrowed Keyblade swings then meet enemy melee attacks through KH1's own meeting
     (`0x1402B4A20`).
   - Body windows keep KH2's shapes.
   - Valor's left blade (weapon 2) keeps its own.
   - A bug fix; it touches the borrow path only.
2. **KH2's recoil on KH1's recoil requests.**
   - Slots of KH2's 41 on 0x6E and 43 on 0x70 (with 0x6F: question 1).
   - KH2's push of 50 on the ground, on KH1's per-frame push `+0x120` (0.1 a frame, x 0.9; the
     same path as `combomods::guard_frame`), in place of KH1's gap push.
   - KH2's air hop on KH1's air arc: 50 up, 50 x 0.048 sideways, back from his facing.
   - KH1's combo count minus one stays: KH2 does the same.
   - KH1 still runs the hit pass, so the HUD, rumble and tallies stay KH1's.
3. **KH2's clash rules on our attack data.** Our rows' data (`kh1rows`) get clash type and
   priority from the move's ATKP, so KH1's own pass gives KH2's outcome:
   - pierce (kind 1: Guard Break, Counterguard): clash type 6 (meets no attack or guard, still
     hits bodies), in place of `GUARD_THROUGH`;
   - refact self 2 on a normal attack: priority 0x62 or more (not stopped, no recoil on Sora);
   - the rest keep KH1's swing type and priority.
   - Open: special guards (question 2) and the enemy's recoil kind (question 3).
4. **The guard repel's 80 frames.** `guard_repel` calls KH1's own timer `0x140297F30 (sora, 80.0)`
   (KH2's `+0xD70`). KH1's pass then gives no damage and no reaction for 80 frames (clashes still
   arrive).
5. **Body-hit moves against guards and attacks on KH1's states.** A guard repel or an attack clash
   met by `strike_as` in `kh2_hits` puts a KH1 clash record on Sora (kind 6, as KH1's half does).
   KH1's hit pass then plays it, with step 2's KH2 recoil, instead of the controller call that
   does nothing.

Performance: no new per-frame hook. Steps 2 and 4 use hooks that already exist (the hit pass,
`guard_frame`); steps 1, 3 and 5 change existing code paths. Step 3 is done once at load.

## 5. Open questions

1. KH1's big recoil (kind 7, 0x6F: q more than 40 stronger) has no KH2 counterpart.
   - (a) KH2's 41 for kind 7 too.
   - (b) Keep KH1's 0x6F for kind 7.
   - (c) A Config row.
2. KH2's special guards (kind 3) are the only thing Guard Break meets. KH1's guards have no kind,
   only clash type and priority.
   - (a) Guard Break meets no KH1 guard at all.
   - (b) Treat KH1's directional guards (types 1 / 3) as special guards. No type in KH1's table
     (`0x140528CB8`; our copy reads 1F 09 09 3F 29 18 00 00) meets only those two, so it would
     need a pairing entry of ours (**Assumption** that a spare type can be added safely).
   - (c) Find KH1 enemies that should count as special guards, one by one.
3. A refact-self-2 swing at priority 0x62 or more gives the enemy KH1's kind 7 (the big recoil). In
   KH2 it would be the enemy's plain reflected motion.
   - (a) Accept KH1's kind 7.
   - (b) Lower the priority to within 40 of the enemy's, accepting that KH1 can then stop the swing.
   - (c) Hook the clash half for our attacks to write kind 6.
4. KH1 gives Sora 8 frames of protection after an attack-clash recoil; KH2 gives none.
   - (a) KH2's (none).
   - (b) KH1's 8.
   - (c) Under the knockback switch from Sora's hit reactions native.

## 6. What's built (testing branch)

### 6.1 The weapon index in `strike_as` (plan step 1)

- `combat::Strikes::strike_as` now puts KH1's blade sweep on KH2's weapon 1 (the right Keyblade)
  instead of weapon 0 (his body). **Checked (code)**: `kh2_rig::attack_shapes` and `kh2_data::sora`
  number the body 0 and the weapons from 1.
- So a borrowed Keyblade swing meets an enemy's melee attack through KH1's own meeting
  (`meet_shot` → KH1's clash `0x1402B4A20`), and its guards and walls are tested on KH1's blade.
- Body windows (weapon 0: Explosion, Vicinity Break, Magnet Burst's body hits) keep KH2's shapes
  for guards and walls and still meet only shots; Valor's left blade (weapon 2) keeps KH2's shapes.

### 6.2 The Recoil row and KH2's recoil on KH1's recoil requests (plan step 2)

- Config > KH1 in KH2 > Combat > Recoil, ini `recoil = kh1 | kh2` (KH1 by default, as every row).
  The tester (answer 4): one switch for every move, KH1's rules exactly at KH1, KH2's at KH2.
- KH1's request table (`xa_ex_0010.mset`, **Checked (files)**): 0x6E slot 72, 0x6F slot 73, 0x70
  slot 74. KH2's 41 / 43 (`P_EX100.mset`, **Checked (files)**): 30 / 20 frames at 30 fps.
- At KH2, `kh2slots::RECOILS` plays KH2's 41 on 0x6E and 43 on 0x70. 0x6F (KH1's big recoil, kind
  7) keeps KH1's animation (answer 1 "(b)").
- KH1's reaction functions still run (**Checked (decomp)**, §2.3): the combo count less one (KH2's
  `+0xD74` the same), the queued idle / fall. After the pass (`recoil::after_pass`, from `hurt`):
  - 0x6E: KH1's gap push (`0x1402C7300`, `+0x3FC` / `+0x88` bit 0x40) cleared; KH2's 50 back along
    minus his facing on KH1's per-frame push `+0x120` (`combomods::push_back`, the repel's path:
    0.1 × 50 a frame, × 0.9 a frame, `0x1403D4960`, **Checked (decomp)**).
  - 0x70: KH2's hop (`0x1403D4800`: 50 up, 50 × sstm's `hit_back_to_jump` back from his facing)
    through `shared::physics::kh2_hop`, in place of KH1's 20 and gap drift.
  - All three: KH1's 8-frame no-damage timer (`0x140297F30`: actor `+0xC` and flag `+0` 0x80) put
    back as it was before the pass. KH2 gives none (answer 4). **Assumption:** KH1's reaction runs
    once per pass, so nothing else in the same pass set that timer.
- At KH1 nothing changes.
- Not covered: a Drive Form or a borrowed KH2 move (KH2's controller has Sora) plays KH2's
  controller's recoil (41 / 43) in both settings, as KH1's recoil motions aren't in its movesets.

### 6.3 KH2's clash rules on our attacks (plan step 3; answers 2 "(c)", 3 "(a)")

KH2's ATKP rows of Sora's moves (`00battle.bin` and the clips' hit windows in `W_EX010.mset` /
`P_EX100.mset`, **Checked (files)**):

| kind | refact self | moves (ATKP) |
|---|---|---|
| 1 pierce armor | 2 | Guard Break 876, Counterguard 1581, Finishing Leap 854, Explosion 911 / 912, Vicinity Break 1869, Magnet Burst 1900 / 1901 / 1903 |
| 0 normal | 2 | Slapshot 890, Dodge Slash 1007 / 1898, Flash Step 1896, Retaliating Slash 1582, Aerial Sweep 909, Horizontal Slash 963 / 967, Aerial Finish 915 / 927, Aerial Dive 1882 |
| 0 normal | 0 | the plain swings and finishers 126..134 / 147, Upper Slash 170, Slide Dash 968, Aerial Spiral 1131, Retaliating Slash's second hit 1583 |

At Recoil KH2:
- **Our KH1 rows** (`kh1rows::rules`, from the move's first hit window's ATKP; KH1 reads the data
  when it makes the row's attack object, `0x1402B2DC0`: `+0x38` bits 0..2 → object flags 17..19,
  `+0x3C` → priority `+0x68`, **Checked (decomp)**):
  - pierce armor: KH1 clash type 6. No entry of KH1's pairing table (`0x140528CB8`: 1F 09 09 3F 29
    18 00 00, **Checked (exe)**) has its bit, and the pass (`0x1402B3BE0`) lets type 6 hit bodies
    (only types 1, 2 and 4 don't), **Checked (decomp)**. So it meets no enemy attack and no guard
    and is never stopped: KH2's pierce armor against KH2's normal attacks and plain guards.
  - refact self 2: priority at least 0x62 (KH1 adds the Keyblade's byte and caps a swing at 99,
    `0x1402B3270`). KH1's clash half `0x1402B4BC0` doesn't stop a side at 0x62 or more, so the swing
    goes on; the enemy's attack (under 0x62) is stopped and its owner recoils by KH1's rule, kind 7
    (big) when our side is more than 40 stronger: the tester's answer 3 "(a)".
  - the rest: the template's type and priority.
  - One attack object a row: a move's rules are its first hit window's (Retaliating Slash's
    second hit, 1583, is normal in KH2 but rides on the first's).
- **KH2's hit shapes** (`combat::Strikes::strike_as`: borrowed moves and the body-hit moves on
  KH1's states), by each hit window's own ATKP:
  - pierce armor meets only special guards (`SPECIAL_GUARDS`) and no enemy attack;
  - refact self 2: a guard it meets doesn't repel it, but that guard's owner is immune to the swing
    (KH2's `0x1403D2140` makes the guard's side immune to the attack it met, unless the attack is
    itself a guard with refact self 2); an enemy attack it meets goes through KH1's clash with our
    side at priority 99 (not stopped).
- At Recoil KH1: every row has its template's type and priority (Guard Break included: it no
  longer passes guards at KH1), and the KH2 shapes' path uses its KH1 row's clash type, as before.

**Special guards** (answer 2 "(c)": enemy by enemy). KH2's kind 3 rows (47 in `00battle.bin`,
**Checked (files)**) are on Sora and his allies: Aerial Recovery's body guard 124 (every form),
Reflect's barrier 573, Anti Form's 454 / 1749, Riku P_EX110's 425, King Mickey P_EX200 / P_EX220's
1197, the Hollow Bastion allies (N_HB530 / 550 / 570 / 580), Hercules N_HE010's 1695 / 324, the
light cycle, N_EX760's; and three enemy models: Xemnas B_EX170 (1976, range 120), M_EX420 (599),
M_EX600_LC (971). So in KH2 almost every enemy's guard is a plain guard that Guard Break breaks,
and a special guard is the kind of block that even pierce armor can't get through (an ally's or
Sora's own invulnerable guard, Reflect, Xemnas's shield). No KH1 enemy is on the list yet, so at
KH2 Guard Break meets no KH1 guard. **Assumption** (not checked enemy by enemy): which KH1 enemy
guards should count. Our rows' type 6 can't meet any guard in KH1's own pass, so a listed enemy
would only stop KH2's shapes (`strike_as`) until the pass learns it too.

**Not KH2:**
- A refact-self-2 KH1 row at priority 0x62+ never meets a guard (priority 100: KH1 meets two
  attacks only when one is under 0x62), so it hits through the guard to the body. In KH2 the
  guard's owner would be immune. Needs a hook on KH1's clash half or the pass to match.
- An enemy attack of priority 0x62+ is never stopped by a normal swing (KH1's half), where KH2
  stops every normal attack a normal attack meets.

### 6.4 The guard repel's 80 frames (plan step 4)

- At Recoil KH2, `combomods::guard_repel` (KH2's repel, motion 42, on KH1's states) starts KH1's own
  no-damage timer at 80: actor `+0xC` = 80.0, flag `+0` 0x80 on and 0x100000 off, as
  `0x140297F30` does (**Checked (decomp)**). While 0x80 is up KH1 takes no damage from a record and
  starts no reaction, clashes excepted (`docs/kh1-hit-reactions-native.md` §3, **Checked
  (decomp)**), which is KH2's `+0xD70` rule (every attack dropped but the cures).
- **Assumption:** KH2's `+0xD70` counts 60 Hz frames (its step `DAT_140717480` is 1.0 in the file),
  so 80 frames on KH1's 60 Hz timer.
- At Recoil KH1 the repel gives none, as before.

### 6.5 Body-hit moves meeting a guard on KH1's states (plan step 5)

- KH2's own data makes Explosion, Vicinity Break and Magnet Burst pierce armor (§6.3, **Checked
  (files)**), so at Recoil KH2 they meet no guard and no attack: no recoil, as in KH2.
- Where a KH2 hit on KH1's states (`takeover::kh2_hits` → `strike_as(.., native)`) is stopped by a
  guard (Recoil KH1, or a normal KH2 hit at KH2), the guard now meets it through KH1's own clash
  (`meet_shot` with the guard's attack object against a swing of his KH1 row): KH1's half
  `0x1402B4BC0` stops the swing and queues a kind 6 / 7 record on Sora (**Checked (decomp)**), and
  KH1's hit pass plays the recoil (KH1's at Recoil KH1; KH2's 41 / 43 at KH2, §6.2). Before, the
  swing was only marked repelled and `register_wall_hit` went to a controller that wasn't driving
  him.
- A priority-100 guard against a swing under 60 is a gap over 40: kind 7, KH1's big recoil, which
  is what KH1's own swings get from it (**Assumption:** the swing row's priority; it is the KH1
  template's plus the Keyblade's byte).
- Borrowed moves (KH2's controller) keep `register_wall_hit` (KH2's repel off the guard).

### 6.6 One Recoil row for recoil, parry and the guard repel

- The tester (thread m2 answer 1): "we need to also combine guard repel and recoil into the same
  config". The Guard Repel row (Movement, t536, ini `guard_repel`) is gone; `combomods::repel_kh2`
  is now the Recoil row. What base Sora's KH2 guard does on a block (`combomods::guard_repel`):
  - Recoil KH1: KH1's own block (`combomods::kh1_block`: KH1's clash recoil 0x6E / 0x6F by the
    priority gap, its 8 frames, KH1's Counterattack after it; `docs/kh1-anim-slots.md` §7.x.3).
  - Recoil KH2: KH2's repel (motion 42, pushed back 200, KH2's 80 frames of §6.4, Counterguard).
- The old ini default was `guard_repel = kh2` while `recoil` defaults to KH1, so a fresh ini (and
  the tester's, `recoil = kh1`) now gets KH1's block. **Checked (code).**

### 6.7 Recoil KH1: no guard windows in his attacks

- The tester (m2): "guard break and other kh moves still use the kh2s recoil mechanics when I set
  recoil to kh1". His trace (`trace_1792887810_f39547`) and log (t544, `recoil = kh1`): Guard
  Break (KH2 motion 165 on our row 69, request 0x203) put its own Keyblade guard up (ATKP 162,
  frames 5..42, kind 2 refact self 2) as three KH1 guard objects, and `hurt` took the enemy's three
  hits on it ("KH2's guard on the Keyblade (ATKP 162, refact self 2), no effect"). **Checked
  (trace).** KH2's own data puts guard windows in some attacks (Guard Break 165's 162, Vicinity
  Break 170's three 162s; **Checked (files)**, `W_EX010.mset` via the log's guard lines).
- KH1's swings carry no guard: an enemy attack meeting a KH1 swing is KH1's clash by priority
  (`0x1402B4BC0`, both sides under 0x62 both recoil; a side at 0x62 or more isn't stopped), or it
  reaches his body. **Checked (decomp).**
- So at Recoil KH1 (`recoil::guard_window_on`) a Keyblade guard window (KH2 weapon 1 / 2) guards
  only in Sora's guard moves (the guard press 173 and its repel 42); in attacks it is neither a KH1
  guard object (`combat::guard_step`) nor a guard `hurt` takes a hit on (`kh2slots::keyblade_guard`).
  The swing's own KH1 row object meets the enemy's attack in KH1's pass as any KH1 swing does.
- Not changed: his body guards (weapon 0: Aerial Dodge 202 / Flash Step 169 / Vicinity Break 170's
  406, Aerial Recovery 40 / Counterguard 171's 124), which are another thread's (body guards), and
  KH2's controller (Drive Forms, borrowed moves), which keeps KH2's recoil and guards in both
  settings as before.
- Recoil KH2: every window as KH2 has it.
