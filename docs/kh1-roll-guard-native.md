# Dodge Roll, Guard and Counterguard native (research, t480 round)

Research for the tester thread `testers/dodge-roll-guard-and-counterguard-native/`. All addresses
are the **Steam** build's (`0x140000000` + RVA) unless marked Epic. Sections 5..7 are what was
built from it.

## 1. What we have now (Checked (code))

- With Dodge Roll at KH2 (the default), Square on the ground is a **borrow**: KH2's controller
  takes Sora (`takeover`, `shared::SQUARE`) and runs its own roll (motion 209), guard (row 11,
  motion 173), Quick Run, the guard's repel (motion 42) and Counterguard (row 39, motion 171).
  `docs/shared-states.md` "Dodge Roll".
- With Dodge Roll at KH1 (`styleroll.rs`), KH1's own roll runs (`0x1402C9E60`, motion request
  0xDC, slot 117) and is drawn with KH1's animation. KH2 borrows him out of it for Quick Run.
- KH2's guard windows become KH1 guard objects (`combat::guard_step`, clash type 4, priority 100)
  and `hurt` judges hits against KH2's guard (refact self 1 = repel, 2 = nothing).
  `docs/kh1-clash.md` "KH2's guard windows as KH1 guards".
- KH2's invulnerability while borrowed: KH1's untouchable flag `0x40` set from the controller's
  output (`reflect::intangible`). `docs/kh1-intangible.md`.
- `kh2slots` already has the pieces a native version needs: KH2 motions as KH1 slots on KH1's own
  requests, `kh2slots::invulnerable` (KH2's invulnerable windows at KH1's time in the slot),
  `body_guard`, root travel per slot (`air_root`). `kh1rows` already makes KH1 attack rows of our
  own (Guard Break etc.); its notes say "Counterguard left".

## 2. KH2 (Steam KH2 decomp, Checked (decomp) unless marked)

- **Action button handler `0x1403A81F0`.** Order, as read: a state-7 case (action 0x16,
  **unconfirmed** what it is); Glide (growth kind 3: action 0x11 hover with the stick at rest,
  0x12 glide otherwise); the Square combo rows 0x25 (37 Finishing Leap), 0x0C (12 Upper Slash),
  0x26 (38 Horizontal Slash) through `0x1403A9950`; then, **only with the stick at rest**
  (`+0x1C == 0`), the guard `0x1403A9C70`; then growth kind 4 (the dodge roll) and Quick Run (the
  rest of the function, not read line by line this round).
- **Guard `0x1403A9C70`.** Looks up combo row 11 (`0x1403D74E0(player, 0xB)`), checks it
  (`0x140402750`) and `0x1403BFB30` (a vtable test plus `+0x18C & 0x1C`; **Assumption**: "can use
  the action button here"), then starts the row facing the primary target (`0x1403D78D0`, target
  at `+0xDC8 + 4`) and zeroes the speed (`+0x10` / `+0x18`). Row 11 is motion 173 in Sora's PTYA
  (Checked (files), stage 158): ATKP 161, a Keyblade guard window frames 6..37, refact self 1.
- **Counterguard: attack press handler `0x1403A76B0`** (and its twin `0x1403A8690`): if the
  current motion (`+0x180`) is 0x2A (42, the guard's repel), it tries row 0x27 (39, motion 171).
  Then a form check (`+0xC` → `+8` == 0x16 or 0x1A) tries row 0x29. Otherwise the normal combo.
- **The repel**, motion 42: Keyblade guard window 0..40, ATKP 162, refact self 2 (Checked (files),
  `docs/kh1-clash.md`). Counterguard's window is the whole time 42 plays (the handler checks the
  motion only).
- **Dodge roll invulnerability:** fmab `dodge_roll_star_time`, 10 / 16 / 24 / 32 frames by level
  (Checked (code), `docs/movement-model.md`, our KH2 rewrite). The roll can be left from its
  fall-check event (frame 34) and turns into Quick Run at its quick-run-check event with Square
  held (Checked (code), `character_core` controller, from the OG rewrite).

## 3. KH1 (Steam KH1 decomp, Checked (decomp) unless marked)

### 3.1 The Square handler: roll or guard, `0x1402A7B30`

Sora's per-frame button step (called from the control step `0x1402A6220`). For Square
(`0x142D53A40 & 0x4000000`, **Assumption**: the "pressed" bit) in movement state 0 and not in deep
water (`0x1402C16E0`):

- **Guard** if the Guard ability bit is on (`0x142D5EC10 & 0x10`) and either Dodge Roll is off
  (`& 0x20`) or the stick's tilt (the move vector's `+0xC`) is under a threshold from his stats
  (actor `+0x6C` → `+8`). So KH1 guards standing *and* on a light tilt; KH2 only at rest.
- **Roll** otherwise, with Dodge Roll (`& 0x20`): face along the stick (`0x140297FB0` gives the
  angle, `0x140297E10` sets it), then `0x1402C9E60`.
- The ability bits come from Sora's equipped abilities each set-up (`0x140284BE0(id)`): 0x13
  Counterattack → bit 0x4, 0x15 Guard → 0x10, 0x16 Dodge Roll → 0x20 (the function before
  `0x1402A68D0`). (Older notes give `0x142D5F610`, which is the Epic address.)

### 3.2 KH1's roll, `0x1402C9E60`

`0x140297F30(actor, 12.0)` (**unconfirmed**: a speed or turn), actor flag `0x100000`, motion
request `0xDC | 0x1000000` (`0x14029F980`), then the idle queued after it (`0x14029FA30(0x40000000,
8.0)`). No movement-state change: the roll runs in ground state 0 as a motion. Slot 117 (request
table, Checked (files) per `docs/kh1-anim-slots.md`). A second roll start `0x1402C9EB0` (from
`0x1402CC470`) uses the same request; **not read** what situation that is.

**Invulnerability:** actor flag `0x40` (left out of the hit pass). The actor op handler
`0x14028E6E0` sets it on op 0x2B and clears it on 0x2A. Where the roll's ops come from (its motion
or a script) was **not found** this round; `docs/kh1-anim-slots.md` couldn't find it among the
motion's events either.

### 3.3 KH1's guard is an attack row, not a state

There is **no guard movement state**. The Square handler's guard branch:

1. faces the lock-on target (`0x1402A3B00` with `0x142DB3518`, the target handle and point), or
   keeps his facing;
2. starts attack row **0x3D** of Sora's attack table (`0x1402A3980`, the row starter);
3. queues the idle after it (`0x14029FA30(0x40000000, 0)`) and sets combat state `+0x41` to 1.

Row 0x3D (btltbl.bin, Sora's table at `+0xA9F0`, Checked (files)): motion request **0xD4** (slot
110 by `docs/kh1-anim-slots.md`'s request ranges, **Assumption** for the exact slot), attack data
**0x4DA8**: its `+0x38` clash byte is 2 (a pure guard: meets attacks, never bodies), arc cosine
`+0x64` = -0.174 (about +-100 degrees around his facing), no power, no reaction. The row starter
makes the attack object from that data (`0x1402B3270`), so KH1's guard is **an attack object of
priority 100** tied to the guard motion's hit window, exactly the kind our `guard_step` already
makes for KH2's windows.

### 3.4 What a guard does when hit, and KH1's Counterattack

- The attack-vs-attack meeting `0x1402B4A20` runs the clash `0x1402B4BC0` both ways. The guard
  side (priority 100) returns at once (marks only); the attacker's owner gets a clash record
  (`0x1402BF060`: kind 6, or 7 when the priority gap is over 40) and pushes back. If the guard's
  owner is Sora and the attack is a melee kind, the guard tally (`0x14028C660(10)`) counts it.
  The spark: `0x1402AE650`.
- Kind 6 on Sora (his own swing clashed) is `0x14029D720` from the hit pass `0x14029BBD0`: motion
  request **0x6E** (slot 72, the clash recoil), pushed back by the priority gap, combat `+0x41`
  less one.
- **KH1's Counterattack** (`0x140294220`, the command menu's top entry): the Attack entry
  `0x140528914` becomes **command 99** instead of 1 when Counterattack is on (`& 0x4`), the
  lock-on target is an enemy, Sora's motion request (`+0x164`) is **0x6E**, and his attack
  object's result word (combat state `+0x50`, which the attack object points at through `+0x60`)
  has any bit but 0x1 / 0x100. Command 99 runs attack row **0x3E** (stage 230's notes): request
  **0xD5**, attack data 0x4E18 (reaction 3, a swing). Checked (files) for the row.
- **Not found:** what puts Sora into 0x6E after his *guard* stops a hit (the clash gives the guard
  side no record). Either the guard motion reads its object's `+0x9C` / result bits itself, or
  KH1's Counterattack only comes after his swing is clashed. Needs a trace (Square guard, get hit,
  watch `+0x164` and combat `+0x50`) or more decomp.

### 3.5 What reads these (don't break KH1's events)

- The guard tally `0x14028C660(10)` and the attack tally `0x13` (the row runner) are save
  counters read by event scripts (opcode 474, AGENTS.md stages 50 / 144). A native guard through
  row 0x3D keeps tally 10 counting; a native Counterguard through row 0x3E / command 99 keeps
  KH1's own counting.
- Op handler `0x14028E6E0` is shared with scripts: flag `0x40` set by us must be cleared only if
  we set it (as `reflect::intangible` does now).

## 4. Plan (one system per version)

1. **Roll.** Square at Dodge Roll KH2 runs KH1's roll (`0x1402C9E60`, or the handler's own roll
   branch) instead of the borrow; request 0xDC's slot is a `kh2slots` slot of KH2's 209 (template
   slot 117), retimed; KH2's travel from the clip root (`air_root`'s ground counterpart); KH2's
   invulnerable frames by level as flag `0x40` from the slot's time (`kh2slots::invulnerable`
   style); KH1's "can be left" level event at KH2's fall-check frame (34); Quick Run's check frame
   keeps the existing Quick Run borrow (Quick Run's own thread). KH2's rule for roll vs guard
   (stick at rest = guard) in place of KH1's tilt threshold.
2. **Guard.** Square standing runs KH1's guard row 0x3D; request 0xD4's slot is KH2's 173; the
   guard object's data copied (as `kh1rows` does) with KH2's numbers: live on 173's guard frames
   6..37 (the slot's hit window event), the arc KH2's. `guard_step` then has nothing to add for
   the guard press.
3. **Repel and Counterguard.** A blocked hit (refact self 1) plays KH2's 42 on a request of our
   own (0x6E is KH1's swing-clash recoil, which KH2 draws as 41, so it can't be shared); Attack
   during it runs a KH1 row of our own for Counterguard (171), or KH1's row 0x3E / command 99 with
   171 in its slot.

Performance: each step is a slot (built once), a row copy (once) and a check in hooks we already
have (control step, hurt, guard_step); no new per-frame hook. It removes the borrow's per-frame
KH2 controller run during rolls and guards.

## 5. Built: KH2's dodge roll on KH1's roll (tA)

- **KH1's request table** (`xa_ex_0010.mset`, Checked (files)): request `0xDC` is `0xA075`: slot
  117, kind 2, **travel flag (bit 15) set**: KH1 moves him by the slot's root (H10), as its ground
  combo. Slot 117: 38 frames at 60 fps, "may be left" (type 0) at 30, foot effects (type 4) and four
  floor sounds (type 6, material kinds 5 / 26 / 1 / 4). No untouchable event in the motion: where
  KH1's flag `0x40` comes from for its roll is still **not found**.
- **KH1's roll start `0x1402C9E60`** (Checked (decomp)): `0x140297F30 (actor, 12.0)` is KH1's
  no-damage timer (`+0xC` = 12.0, flags `0x80 | 0x100000`; the same function every reaction calls
  with 30.0, `docs/kh1-hit-reactions-native.md` §3), then flag `0x100000` again, request
  `0xDC | 0x1000000`, the idle queued (blend 8). So KH1's roll is 12 frames of no damage (hits still
  arrive, no damage and no reaction), not a speed (§3.2's "unconfirmed" settled).
- **What runs now** with Dodge Roll at KH2 (`combomods::roll_press`, from `shared::control`), Square
  fresh on the ground with the stick pushed and KH2's Dodge Roll equipped, after the guard press
  (stick at rest) didn't take it: KH1's deep-water test, facing set along the stick, `+0x2C` 0, KH1's
  roll start, KH1's timer flags and `+0xC` put back as they were. `kh2slots`' roll spec points
  `0xDC` at our slot of KH2's 209 on template 117 while the style is KH2 (KH1's own slot at KH1):
  baked with KH2's root travel in its H10 (the request's travel flag), KH1's floor sounds and foot
  effects rescaled, KH1's "may be left" from KH2's fall-check event (frame 34) to the end in place of
  KH1's 30. `hurt` evades every hit while our slot's time is under the level's
  `dodge_roll_star_time` (10 / 16 / 24 / 32, `combomods::roll_invulnerable`), and while its request
  is pending. Square held through it: KH2's Quick Run (`quickrun::start`) at KH2's check frame,
  even before KH1 lets him act. The borrow stays for forms, an unbuilt slot, or KH2's Square with no
  roll (Quick Run only).
- **Assumptions**: KH1's ground step moves him by `+0x2C` while the action flag is set (as
  `start_own`); nothing else reads flag `0x100000` as "rolling"; unbaked (`kh1_bake = off`), the
  slot keeps KH1's roll travel stretched over KH2's length.

## 6. Built: KH1's Counterattack command during KH2's repel (tB)

- **KH1's own swap** `0x140294220` (Checked (decomp)): each menu refresh it writes the top menu's
  Attack entry (`0x140528914`, the top menu `0x140528900` + `0x14`): 0x62 in two rooms (world 6;
  world 0x10 room 0x18), else **99** when KH1's Counterattack bit (`0x142D5EC10 & 4`) is on, the
  lock-on target (`0x142DB3518`) is an enemy (`+0x374 & 0x60 == 0x40`), Sora's request is `0x6E`
  and his attack's result word (combat state `+0x50`) has any bit but `0x1` / `0x100`; else 1. Then
  it calls the battle-menu fill `0x1402944C0` (which we hook already) and the context commands
  (`0x1402948F0`, Talk through `0x140290E70`).
- **Ours** (`combomods::counter_entry`, from `drive::fill` after KH1's fill): while our slot of KH2's
  repel (42) plays on the ground with KH2's Counterguard equipped (`counter_open`), an Attack entry
  of 1 becomes 99, so the menu shows KH1's Counterattack command there. No target is asked for (KH2's
  Counterguard needs none). Choosing it (`combat::command`) turns the command record's type into 1
  (KH1's Attack, target kept) before KH1's command step, which then runs KH2's Counterguard through
  the repel's combo window (`counter_window`, `counterguard`, t489). KH1's own row 0x3E isn't run.
- **Assumption**: the fill runs every frame the menu is up (stage 189's "rebuilt each frame"). It
  needs the fill hook, which `shortcuts`, `reflect` or `drive` puts in (all on by default).

## 7. Built: the guard's coverage, KH2's angle test (tC)

- **KH2** (Steam KH2 `0x1403D1F40`, Checked (decomp)): an attack meeting a guard is held only when
  the guard's ATKP reflect range (`+0x22`) is 0, or the other attack's direction dotted with the
  guard owner's facing (turned by the reflect angle `+0x23` x 2 degrees) is below -cos(range): the
  attacker within `range` degrees of his facing. The OG rewrite has the same rule
  (`character_core::reaction::clash_angle_ok`). **KH2 has no Guard Plus**: the OG rewrite's
  `guard_all_round` is its own ("ours") and nothing ever sets it.
- **Sora's guard ATKPs** (`00battle.bin`, Checked (files)): 161 (the guard press 173) range **120**;
  162 (the repel 42, Guard Break's) **180**; the body guards 406 (Aerial Dodge, Flash Step), 124
  (Aerial Recovery), 455 range 0 (every side). All angle 0.
- **Before** (t489): a Keyblade guard held only the front half (90 degrees each side), in `hurt` and
  in its KH1 guard objects (arc cosine 0). So the press missed hits from 90..120 degrees and the
  repel's own guard missed everything behind him.
- **Now**: `hurt::guard_faces` is KH2's test with the window's ATKP, the attack's direction being the
  hit record's push direction (`+0x10`, which KH1's own front / back test reads), on KH1's states
  (our guard slots) and in KH2's borrows alike. The KH1 guard objects (`combat::guard_data`) carry
  the arc as KH1's own directional test reads it (Steam `0x1402B4640`, Checked (decomp): clash types
  1..4, cosine between the owner's facing turned by data `+0x60` and the line to the other attack's
  owner at or above data `+0x64`): `+0x64` = cos(range), -1 for range 0; `+0x60` = angle x 2 degrees.
  **Assumption**: KH1's `+0x60` turn goes the way his yaw does (unused: every angle is 0).
