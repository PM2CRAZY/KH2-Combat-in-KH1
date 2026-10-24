# Aerial Dodge native (KH2's Aerial Dodge on KH1's air state)

Research notes, testing branch (t480 round). No code changed yet. Addresses are the **Steam**
builds' (Ghidra, image base 0x140000000) unless marked Epic. Plain-language notes only.

## 1. What we have now

- The jump pressed in the air, with Aerial Dodge (ours, ability 67) equipped and not used this
  air time, is a **borrow**: `shared::control` sets `AIR_JUMP` (KH1 sees none of the jump /
  action bits that step), `takeover` hands Sora to KH2's controller in the air (`AIR_START`,
  `AIR_MOVES`, `JUMP_NOW`), KH1's state is set to 0, and the controller's `try_jump` runs the
  dodge (`character-core/src/controller.rs`): a held arc of `air_dodge_height`, `air_dodge_speed`
  along the stick (none with it neutral), facing the stick, the launch stored (`launch_vel`),
  motion 202. KH2's controller then has him until two frames of its plain fall, then gives him
  back into KH1's fall state (`shared_give_back`, stage 315's fall request 0x06). **Checked (code)**
- Once per air time: `shared::AIR_DODGED` / `DODGE_JUMP_UP` (also used to hold back KH1's glide
  until after the dodge, stage 319). **Checked (code)**
- No invulnerability (`AIR_DODGE_STAR` is defined but nothing uses it now); the clip's guard
  window (ATKP 406) becomes KH1 guard objects through `combat::guard_step`, which runs on KH2's
  controller's `Output`. **Checked (code)**
- Known side effects of the borrow (AGENTS.md stages 315-320): a frame of KH1's jump slot / glide
  slot at the dodge's start, the fall request at the hand-back, the "action running" bit, the
  glide request on the first frame. All come from handing Sora between two controllers.
- The model for going native: **Aerial Recovery** (t475, `combomods::recover`): our own KH1
  request (`kh1rows::OURS`, request `0x200 + n`, a `kh2slots` slot of the KH2 motion), asked with
  `0x14029F980 (actor, request | 0x1000000, 0)`, KH1's fall queued after it (`0x14029FA30 (actor,
  0x40000006, 0)`), KH2's hop on KH1's air fields (`shared::physics::kh2_hop`, state 2 only),
  facing written (`+0x30`, `+0x150`), `mark_air_dodged`. The air combo's own hops use the same
  `kh2_hop` (`combomods::air_hop`, t391 / t435). **Checked (code)**

## 2. KH2's Aerial Dodge (Steam KH2 exe)

- **`0x1403CB9F0`**: the jump-press handler of the air actions (in 17 action vtables: the jump,
  fall, glide and others). **Checked (decomp)** for what it does; which slot of the vtable it is
  is an **Assumption** (it is referenced only as data).
  - Level: `0x140401680 (player, 2)`: byte `+0x22` of the player's growth levels (Aerial Dodge is
    growth 2), or **5** when `0x1403ABCD0` says so (world 0x12 room 0x17, or flag bit 13 of
    `0x142A11400`: a special place, not KH1's concern). **Checked (decomp)**
  - Allowed when the level is 5 (no limit), or the level is non-zero and the "dodge left" bit
    (`+0x124` bit 1) is set. Used: that bit is cleared. **Checked (decomp)**
  - The bit is set by the air actions' shared enter (`0x1403CBD30` and its copy) only when the
    previous action wasn't airborne (the action's `+0xC` bit 0). **Checked (decomp)** (So a
    dodge, glide or recovery doesn't give it back; whether the plain fall uses this enter, i.e.
    whether walking off a ledge gives the dodge, I did not check: `docs/movement-model.md` says it
    doesn't, from the Epic exe.)
  - The growth record (`0x1403E92D0 (level)`, table `0x142AE5778`, KH2's `fmab`): `+4` the dodge's
    height, `+8` its speed. Copied: the move vector (`+0x10` of the input), its y replaced by the
    height and its length (w) by the speed when the stick is tilted (0 stays 0: neutral is
    straight up). **Checked (decomp)**
  - Motion **0xCA (202)** asked (`0x1403C86A0`), the action's start (vtable `+0x18` of the action
    object, `0x1404AD270 (+0xC)`) given that vector; apex motion `+0x118` = **4** (the fall),
    landing motion `+0x11C` = **5**, flag `+0x120 |= 0x10000`. **Checked (decomp)**
  - The launch (`0x1403CBB00`, Checked (decomp)): `T = sqrt(2|h| / 0.40816328)` (KH2's gravity
    20/49), the sideways velocity normalised times the speed and stored as the launch velocity
    (`+0xE0`, flag `+0xEC` = 1, what the glide later takes back, stage 331). That the action's
    start negates the height into KH2's y-down before the launch is an **Assumption** (the
    launch rises only for a negative height).
  - The shared enter turns `+0x120` bit `0x10000` into `0x20000`; that `0x20000` is the "held
    jump" (early-release) flag is an **Assumption** (our controller already treats the dodge as a
    holdable arc, `start_held_arc`).
- **The numbers** (`03system.bin` pref `fmab`, the user's install, **Checked (files)**):

  | Level | Height | Speed (per frame) |
  |---|---|---|
  | LV1 | 145 | 18 |
  | LV2 | 155 | 24 |
  | LV3 | 200 | 30 |
  | MAX | 300 | 36 |
  | (5, the special case) | 1700 | 36 |

- **Motion 202** (`P_EX100.mset`, A160, **Checked (files)**): 30 fps keys 0..6, loops from 2;
  "air free" (range id 1) from frame 4 (60 Hz); a guard window, ATKP **406** on collision group 6
  (his body), all through; sound **999019** at frame 1 (frame trigger 8, bank 999 entry 19). No
  hitbox, no invulnerable window.
- **ATKP 406** (`00battle.bin`, Checked (files)): kind 2, power 0, refact self 2 (the move goes
  on, no guard reaction): a guard that stops hits with no effect.

## 3. How KH1 does the matching thing (Steam KH1 exe)

KH1 has no double jump. What a native dodge runs on:

- **Air state 2**, state table `0x142DB3840` (Epic `0x142DB4240`, translated through
  `steam_map.in`), 0x40 a state (`docs/kh1-physics.md` §3). **Checked (decomp)** earlier.
- **The launch** `0x1402AF960 (actor, height)`: `+0x3B0` (air time) 0, `+0x3B8` (rise time) =
  `sqrt(2|H| / 0.48469388)`, `+0x3BC` = -H, `+0x3B4` = his height, **`+0x3D0` (the jump's motion
  base) = 0**, state 2 (`0x1402B1960`), flag `0x10000` off. **Checked (decomp)**
- **The apex** `0x1402AF630`: 6 frames before the apex, if no request is pending (`+0x178` 0) and
  the request playing (`+0x164`) is the base's rise (`+0x3D0 + 4`), it asks the apex
  (`+0x3D0 + 5`) and queues the fall (`+0x3D0 + 6`). **Checked (decomp)** So a request of ours
  doesn't get KH1's apex swap: we ask the fall ourselves (as Aerial Recovery queues it).
- **State 2's pre-step** `0x1402AFC20`: the grabs (vine `0x1402C78C0`, ledge `0x1402C8A50`, bar
  `0x1402C9FB0`, hang `0x1402C3960`, water `0x1402C15C0`), skipped while the rise runs and 6 frames
  past its apex, in the 15 frames after jumping off something (flag `0x10000`), or with the
  action flag 4; else it faces the stick (`+0x150` = atan2 of the move vector). **Checked
  (decomp)** So during a native dodge's rise KH1 doesn't grab, as during its own jump (the
  borrow's own grab tries, `takeover`, stop applying).
- **State 2's jump press** `0x1402AFD60` (the table's `+0x38`): Neverland's flight
  (`0x1402B1EC0`, world flag `0x142867364` bit 0x20) or KH1's glide (`0x1402B26C0`, ability bit
  `0x80` of `0x142D5EC10`). **Checked (decomp)** The dodge must take the press before this (as
  `shared::control` already hides it).
- **Requests**: `0x14029F980` set, `0x14029FA30` queue (the ones `combomods::recover` calls).
  **Checked (code)**, Steam addresses from the decomp.
- **What reads it**: nothing of KH1's counts air jumps (no tally or event reads a jump in the air:
  **Assumption**, not searched exhaustively; KH1's attack tally `0x14028C660 (0x13)` isn't
  touched since the dodge has no attack row).

## 4. Plan (one system per version)

1. **The dodge on KH1's air state.** A row of ours, `kh1rows::OURS` "Aerial Dodge" (no PTYA row,
   request only, like Aerial Recovery), its `kh2slots` slot of motion 202. On the fresh jump in
   the air (`shared::control`'s `AIR_JUMP` branch), when KH1 has him in state 1 / 2 (or KH2's
   jump on KH1's state): our request with KH1's action flag, his facing turned to the stick,
   `kh2_hop` with the growth level's height and speed (KH2's `fmab` above; physics KH1: the same
   numbers under KH1's gravity, as now), the launch speed stored for KH2's glide (`LAUNCH`),
   `mark_air_dodged`. No borrow, no hand-back.
2. **The fall at the apex.** KH2 swaps to motion 4 at the apex (`+0x118`), not when 202's loop
   ends: our per-frame physics hook asks KH1's fall request (0x06) at 6 frames before the apex as
   KH1's own `0x1402AF630` does (or when the slot's air-free frame 4 has passed and he is falling,
   whichever the tester prefers, below). Landing stays KH1's (its landing request 7).
3. **Guard 406 on the slot.** `kh2slots::body_guard` already stops hits for our slots (`hurt`);
   the KH1 guard objects (the parry against enemy attacks, `combat::guard_step`) need a slot path
   too, since `guard_step` only runs on KH2's controller's output.
4. **The early release.** If KH2's dodge is holdable (§2, an assumption to check in the exe's
   held-jump code), `kh2_hop` gets a holdable variant at physics KH2 (`air_step` already does the
   release hop for KH2's jump on KH1's state).

Performance: one call at the press and one apex test a frame on Sora (inside the existing
physics hook); the borrow's controller step, its hand-back and its per-frame pose blend go away
for the dodge. No new hook.

## 5. Open points

- Out of KH2's glide (Glide at KH2 is a borrow, t321): the dodge inside that borrow stays KH2's
  controller's until the glide goes native.
- Neverland: KH1's jump press in the air starts flight there (`0x1402AFD60`); the dodge would come
  first and flight only after it (as now with the borrow).
- Whether walking off a ledge grants the dodge in KH2 (Steam), §2.
- The slot's template: Aerial Recovery uses KH1's air swing slot (102) as its template, whose
  sounds would come with it; a jump slot (KH1's 13) as the template, with its own sounds dropped
  so only 999019 plays, fits a dodge better (**Assumption** until the slot is built and listened
  to).

## 6. Built (testing branch, after the tester's go-ahead)

The tester's answers: 1, my pick (the fall at KH2's apex, as KH2: 1:1 and one test a frame); 2,
(b) the dodge out of KH2's glide goes native too; 3, (a) the dodge first, flight on the next press.

- **t492, the dodge on KH1's air state.** `combomods::aerial_dodge`, from `shared::control`'s
  fresh jump in the air (the borrow stays the fallback when our request isn't built or in a
  Drive Form): our request of motion 202 (`kh1rows::OURS` "Aerial Dodge", request only) with the
  action flag and no blend, KH1's jump state (1) put in its air state first (state 2, `+0x3D0`
  0, flag `0x10000` off, as KH1's launch `0x1402AF960`), facing the stick, `kh2_hop` with the
  level's `fmab` height and speed (`takeover::kh2_air_dodge_growth`), the launch velocity kept
  for KH2's glide (`takeover::note_launch`), once per air time. The slot is 160 frames long
  (`Ours::frames`), KH2's clip looped over it from its own loop point in the bake and the draw
  (`pose`), and only KH2's sound (999019) plays on it (`Ours::own_sounds`: KH1's air swing's
  sounds left out). KH1's air step steers it with the stick as a jump at physics KH2 (it's not
  treated as one of KH1's own air moves despite the action flag): KH2's dodge steers with
  `air_control` on all through. **Checked (decomp)** for KH2's side; KH1's loop at the end of a
  slot with an empty queue (`0x14029F0A0`: from `+0x288`, unless `+0x160` bit 3 holds it) is
  **Checked (decomp)**, but the slot is long enough that it shouldn't come to that.
- **t493, the fall at the apex.** `combomods::dodge_apex`, each KH1 physics frame of Sora's while
  our dodge slot plays in his air state: once KH1's arc fields say 5 frames or less to the apex
  (`+0x3B0` + 5 >= `+0x3B8`) and nothing is pending in KH1's queue, KH1's fall request (6) with
  KH2's blend 8, the action flag cleared. KH2's rule (`0x1403CB680` → `0x1403CB610`, motion 4,
  blend 8) **Checked (decomp)**. That KH1's arc fields are in the same frames as KH2's arc here
  (so the 5 is the same 5) is what `kh1phys::Arc::from_kh2` writes (Checked (code)). Landing stays
  KH1's own (its landing request on touching down).
- **t494, the guard 406 as KH1 guard objects on the slot.** `takeover::kh2_hits` (once a game
  frame with our slot playing in KH1's states) made KH1 guard objects only for guard windows on
  the Keyblade; for the Aerial Dodge's slot its body guard (ATKP 406, group 6, all through 202) is
  one too, all round, as `combat::guard_step` made it for KH2's controller's dodge. An enemy
  attack object meeting it is stopped (KH1's clash, priority 100) with no reaction (`refact_self`
  2, `combomods::guard_repel` does nothing for it); hits his hit pass still finds stay stopped by
  `hurt` (`kh2slots::body_guard`), as before. Cost: KH2's rig posed once a frame while the dodge
  plays (as for the slot moves with guards on the Keyblade). Other slots' body guards (Flash
  Step's 406, Aerial Recovery's 124, Explosion's, Vicinity Break's, Magnet Burst's) are still only
  `hurt`'s: left as they were (not this thread's).
- **t495, the early release.** KH2's dodge is holdable: it sets `+0x120` bit `0x10000`, the air
  actions' shared enter (`0x1403CBD30`) turns that into the held-jump bit `0x20000`, the input
  step (`0x1403D6FC0`) marks the release (`0x40000`) when the jump isn't held, and the air step
  (`0x1403CB680`) then, between 6 frames in and 5 before the apex, swaps to the apex motion (the
  fall, `0x1403CB610`) and cuts the rise to the release hop (`DAT_142A243D8`). **Checked
  (decomp)** (this settles the §2 assumption). Here: the dodge's hop is held
  (`shared::physics::kh2_hop_held`, the arc `holdable`), so `air_step`'s early release (the one
  KH2's jump on KH1's state already uses) cuts it, and asks KH1's fall at once
  (`combomods::dodge_fall`). Physics KH1: no release (KH1's numbers, as before).
- **t496, the dodge out of KH2's glide.** The tester's answer 2 (b). In KH2's glide's borrow (Glide
  at KH2, still KH2's controller's), the jump pressed with the dodge still there: before KH2's
  step the borrow is given back into KH1's fall state (`shared_give_back`, `Back::Air`, KH2's
  glide's launch kept), and the dodge starts natively on it (`combomods::aerial_dodge_with`, the
  level's numbers read off KH2's controller while it's held). Only when our dodge request is built
  (`combomods::native_dodge_built`); else the borrow's controller dodges as before. KH2: the jump
  handler `0x1403CB9F0` is in the glide's action table too (one of its 17), **Checked (decomp)**
  for the table refs; which entry is the glide's is an **Assumption**.
