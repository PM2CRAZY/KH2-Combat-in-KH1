# Aerial Recovery, Retaliating Slash and what Sora can do while launched

How KH2 Final Mix (PC) lets Sora recover from a launch (a "blow"). Read with Ghidra from
the executable (read-only) and checked against the install's data. Only the rules are
written here, not the code.

Confidence labels:
- **Checked (exe)**: read from the executable's code.
- **Data**: read from your install (`03system.bin`, `00battle.bin`, `obj/P_EX100.mset`,
  `obj/W_EX010.mset`).
- **Hypothesis**: our reading. It is likely, but not proven.
- **Ours**: our port's current choice. It differs from the game.

Units are 60 Hz frames and world units, as in `movement-model.md`.

## Where it is checked

**Checked (exe).** One routine, `0x1403D6030`, handles both Aerial Recovery and Retaliating
Slash.
- **Caller:** the player's input dispatcher `0x1403D62A0`. It runs every frame from the
  player update `0x1403A7D50`, after the pad has been turned into the field-command flags
  (`YS::FIELD_COMMAND`, the global object at `0x142A100A0`).
- **Order:** the routine runs before the attack, magic and Square handlers. If it fires,
  nothing else in the dispatcher runs that frame.
- **Jump goes first:** the jump-press handler (the current action's slot 9) is tried before
  it, but only when Sora is in a "free" state (see "While launched" below). During a launch
  that never happens.

## Conditions (both moves)

**Checked (exe).** All of these must hold:

1. **The current action is a blow.** Sora's current action must carry action flag `0x40`
   and not flag `0x800`.
   - Action flags are the word at `+0xC` of the static action objects (table
     `0x140750AC0`).
   - Flag `0x40` is set only on action 4 (`YS::ACTION_BLOW`, the launch) and on two blow
     variants (actions 8 and 21, flags `0x142` and `0x16B`). These variants are entered
     instead of 4 when Sora is in a special movement mode (`0x140409010`).
   - **Air flinch doesn't qualify.** The air flinch (reaction group 3 while airborne, motion
     14/15) puts Sora into `ACTION_JUMP` (action 1, flags `0x181`).
   - **Ground hits don't qualify.** Flinches and other ground reactions never set the blow
     action.
   - **So both moves exist only during a launch** (reaction group 4, motion 16 from the
     front, 18 from behind).
2. **Time since the launch began ≥ `plyr.blow_recovery_time`.**
   - The blow keeps its own timer at `+0x110`. Entering `ACTION_BLOW` sets it to 0
     (`0x140421B90`). The blow's physics step adds the step length to it, 2 per 30 Hz tick
     (`0x140421A60`).
   - The comparison is "threshold ≤ timer".
   - For Sora (`plyr` record 1) the threshold is **16 frames** (Data).
   - A fresh launch while already launched re-enters the action, so the timer restarts.
3. **The player-control lock is off.** The lock is bit 2 of `+0x9B8`. The same bit
   suppresses the whole player update and reactions; it is probably the event or cutscene
   lock (**Hypothesis**).
4. **The button was pressed this frame** (see each move).

There is **no Hyper Healing term**. The threshold is always `blow_recovery_time`.
Ours (b57) allows both moves from the launch's 2nd frame with Hyper Healing; the exe
doesn't (**Checked (exe)** for this routine).

Once Sora lands, the blow's landing slot (`0x1404219E0`) switches him to another action. So
both moves are only possible while he is still in the air. If a launch lands within 16
frames, neither can happen.

## Inputs

**Checked (exe).** `FIELD_COMMAND` (`0x1403B04C0` / `0x1403B0870`) turns the pad's
pressed-this-frame mask (`FIELD_COMMAND+0xAE8`, then `+8`) into flag bits at
`FIELD_COMMAND+0xC`:

| Flag | Meaning | Set from |
|---|---|---|
| `0x02` | **Jump** | Pad bit 3 when the command menu is at its top level. Inside a submenu, the same button backs out instead. Pad bit 8 in the special menu modes. |
| `0x08` | **Square / action button** | Pad bit 9 |

The other users of flag `0x08` confirm it is Square, the guard and action button. In the
dispatcher it calls the player's Square handler (`0x1403A75A0`, through controller vtable
slot `0x100`), which does Glide, Quick Run, Dodge Roll, the Square combo rows (Upper Slash,
Finishing Leap, Horizontal Slash) and Guard.

**Hypothesis:** with the Magic or Items submenu open, a jump press closes the submenu
instead of doing Aerial Recovery.

## Aerial Recovery

### The check

- **Ability:** bit 20 of Sora's equipped-ability bitset (`*(+0x5C0) + 0x1D0`)
  (**Checked (exe)**).
  - **Which numbering:** the bitset is the same one the combo rows test (`0x140401A30`). It
    is indexed by the item record's ability number (u16 at item `+4`), **not by the item
    id**. `0x140400A10` sets bits through that field.
  - This is the numbering our PTYA constants use (Guard = 1, Upper Slash = 18, Retaliating
    Slash = 101).
  - Aerial Recovery is item 158, ability number **20** (Data).
- **Button:** jump (flag `0x02`).
- **Priority:** Aerial Recovery is tested first. If jump and Square come on the same frame,
  Aerial Recovery wins.

### What it does

**Checked (exe)** unless marked:

1. **Motion:** it plays motion **40** (`A212`) with no blend. It queues motion **4** (fall)
   to follow it.
2. **The hop** goes through the blow's "start hop" slot (`0x140408DB0` → `0x1403CADE0`):
   - **Height:** `plyr.blow_recovery_h` (Sora: **100**). It uses the normal jump arc,
     `T = sqrt(2h/g)`, which is about 22 frames to the apex.
   - **Horizontal speed:**
     - With the stick tilted: `plyr.blow_recovery_v` (Sora: **10**/frame) along the stick
       direction, whatever the amount of tilt. Sora turns to face the stick at once.
     - With the stick neutral: 0, a straight-up hop with his facing kept.
   - **The launch's own velocity is discarded.** The hop replaces it.
   - The stick vector is cleared for that frame.
3. **Action:** the action becomes **1 (`ACTION_JUMP`)**.
   - **No apex motion:** the apex-motion field is `-1`, so motion 40 isn't swapped at the
     top of the arc.
   - **Landing motion:** the landing-motion field is 5 (land) (**Hypothesis** that this
     field is what `ACTION_JUMP` lands with; it is the field Aerial Dodge also sets).
   - **No Aerial Dodge:** `ACTION_JUMP`'s entry (`0x1403CB010`) grants the once-per-jump
     Aerial Dodge flag only when the previous action wasn't airborne. The blow is airborne,
     so **no Aerial Dodge after a recovery**.
4. **Motion 40 windows** (Data, 60 Hz frames from the motion's start):
   - **Invulnerable:** frames 0–16 (range trigger 27).
   - **Repel window:** frames 0–16, ATKP 124 on Sora's collision group 6. ATKP 124 is kind
     3, power 0, the guard/repel kind we already treat like Aerial Dodge's ATKP 406.
   - **Reaction commands:** allowed from frame 8 (range 14).
   - **Control returns at frame 24** (range trigger 1, the "air free" state). From then on
     he can air-attack, cast, use items and glide, and the stick steers again.
   - **Length:** the motion is 30 frames. Its loop point is its last frame, then motion 4
     (fall, "air free" throughout) plays.
5. **The ending:** an ordinary fall that ends in a landing (motion 5) (**Hypothesis** for
   the exact landing motion, see 3).

## Retaliating Slash

### The check

**Checked (exe).** It is in the same routine. It is reached only if the Aerial Recovery
branch didn't fire.
- **Button:** Square (flag `0x08`). It is not attack and not jump.
- **Counts:** Sora's combo / finisher counts (`+0xD74`) are reset to 1 / 1.
- **Row:** it looks up PTYA row **40** in Sora's set, and the row must pass the normal row
  conditions (`0x140401A30`):
  - **Motion:** nonzero (172).
  - **Ability:** ability **101** (item 273, Retaliating Slash) must be equipped.
  - **Air flag:** the row's air flag (`flag & 1` = 1) must match airborne. The blow
    action is airborne.
  - **Combo count:** the normal non-finisher count check (`0x1403D6B30`). It passes with
    the counts just reset.
- **Same conditions as Aerial Recovery:** blow action, ≥ 16 frames into the launch, and
  control lock off.

### What it does

**Checked (exe)** (`0x1403D6BB0`, the normal combo-row starter) and **Data**.

**Row 40:**

| Field | Value |
|---|---|
| Motion | 172 (`A344`) |
| Next motion | 4 (fall) |
| Air flag | 1 (air row) |
| `jump` | 21 |
| `jump_min` / `jump_max` | 21 / 121 |
| Speed window | 0–18 |
| `distance_adjust` | 85 |
| Ability | 101 |

1. **Motion:** it plays motion 172 with no blend and queues motion 4 (fall) after it.
2. **The hop** goes through the blow's start-hop slot, so the action becomes
   **`ACTION_JUMP`** (landing-motion field 5).
   - **With a target:**
     - Height = the target's height above Sora, clamped to 21–121.
     - Horizontal speed = (horizontal gap − target radius + 85) ÷ the time to the apex,
       clamped to 0–18.
     - The direction is toward the target.
   - **With no target:** height 21, speed 18 along his facing.
3. **Motion 172 windows** (Data):
   - **Hitboxes** (on the Keyblade, group 2):
     - Frames 2–22: ATKP 1582, power 100, reaction 4 (**launches**).
     - Frames 36–50: ATKP 1583, power 65, reaction 3 (flinch).
   - **No gravity:** frames 20–36.
   - **Turn to lock-on:** frames 22–50.
   - **Combo windows:** frames 22–36 and from 44 (range trigger 11), the usual way to chain
     into air swings.
   - **Turn at the start:** a frame-0 trigger 21 with 360°, probably a snap toward the
     target (**Hypothesis**).
   - **No invulnerability window, and no "free" state.**
   - **Length:** 72 frames (36 at 30 fps). Full control returns when the fall motion
     starts.

## While launched without either ability

**Checked (exe) + Data.** Sora can do nothing at all until he lands, not even a ledge grab.

**Why:**
- **The "free" bits.** The launch motions 16 and 18 carry no "state" range triggers (ids 0,
  1, 2). Those triggers set bits `0x04` / `0x08` / `0x10` of the motion state word
  (`+0x18C`). The range-trigger handler is `0x14040CAF0`, and every motion start resets the
  word to 1.
- **The combo bits.** The launch motions carry no combo windows (range 11 → bit `0x400`)
  either.

**So during the launch:**
- **Attack, magic, items:** the command gate `0x1403D5F80` passes only with a free bit, an
  open combo window, or the finisher window. All are closed, so the command is dropped.
- **Jump:** the jump-press handler is only called with a free bit (or while the landing
  motion 5 plays). Only the Aerial Recovery check sees the press.
- **Square moves:** Glide, Quick Run and Dodge Roll go through `0x1403BEE10`, which also
  needs a free bit. The Square combo rows go through `0x1403A8D00`, which needs the combo
  window bits. Only the Retaliating Slash check sees the press.
- **Steering:** the stick vector is zeroed every frame when no free bit is set
  (`0x1403A7D50`), so there is no air steering.
- **Ledge grab:** the blow's ledge-grab check (`0x14040DD00`, from the blow update
  `0x1403CA960`) also requires a free bit, so it is off.
- **Reaction commands:** not offered while launched (see `reaction-commands.md`).

**After landing:** the get-up motions 17 and 19 get the "free" state (range 2) at their
frame 40. Our reading of where that frame counts from is unconfirmed (**Hypothesis**).

**Air flinch:** for comparison, motion 14/15 also has no free state. Control returns when
it ends and the fall motion 4 begins (range 1 from its frame 0), with no Aerial Recovery.

## Differences in our port (Ours, to fix)

- **Aerial Recovery's motion:** it plays the aerial-dodge flip (202) with a 50-unit hop.
  The game uses motion 40, hop 100, stick speed 10, and returns control at its frame 24.
- **Aerial Recovery from an air flinch:** the game has none.
- **The wait:** ours is 6 frames (2 with Hyper Healing). The game's is 16, with no Hyper
  Healing change.
- **Retaliating Slash's button:** ours also accepts attack. The game uses Square only.
- **Attacking while launched:** ours lets it through. The game drops every command, the
  jump-press handler, Square moves and stick input until landing, apart from the two moves
  above.
