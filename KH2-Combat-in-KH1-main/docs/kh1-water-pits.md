# KH1: water, pits and KH1's ground step while KH2 moves Sora

Research notes for the water and pit fixes (disassembly of the Epic KH1 Final Mix PC exe, image base
`0x140000000`, addresses absolute; read statically, then checked against the user's own logs).
KH1's world is y-down. Confidence: **[high]** read plainly from the code, **[med]** inferred,
**[low]** a guess.

The player's report (public build): in a "certain modded animation" Sora went under the water
outside Merlin's house and walked on the bottom; fell into the Bottomless Hall's dark (Agrabah)
without reaching the chambers below; and in Hook's fight sank under the sea, came up swimming
later and was stuck there for good, or hung in midair in a climbing pose.

## 1. How KH1 runs a player's frame [high]

- The think step `0x1402A1170` runs every frame for Sora. It calls the control step
  `0x1402A4090` (the pad, commands), then **the current movement state's input step** (state
  table `0x142DB4240`, 0x40 a state, `+0x18`) with the move vector. Nothing skips the input step.
- The physics step `0x140298230` (hooked by `takeover`) runs the state's move (`+0x20`), the
  mover, the splash and in-water flag, and the state's next-state step (`+0x28`).
- While KH2 has Sora, `takeover` replaces the physics step and keeps KH1's state at **0** (its
  ground state), in the air too (`AIR_STATE` 3 only for its own mover calls). So **state 0's
  input step `0x1402CA350` runs every frame while KH2 has him, on the ground and in the air.**

State 0's input step tries, in order: `0x1402C52D0` and `0x1402C5510` (ladders and climbable
walls, states 9 / 10), `0x1402C6760` (the hang off an edge he walks over, state 13),
`0x1402C8AB0`, **`0x1402BF300` (wading into deep water, state 17)**, `0x1402C90A0` (state 12,
skipped while `0x14298D3A8`), then its walk.

The fall state's (2, also 3) input step `0x1402ADA90` tries the air grabs (vine, ledge, bar,
hang points, only while he can act and not just off something) and then, **outside those
tests, every frame, `0x1402BF410` (the swim from the air)**. Its next-state step `0x1402ADA30`:
deep or still in the air, in a flight world (`0x142867CD8` bit `0x20`) with the room's height
passed (`0x1402AFC10`), flight (state 8).

## 2. Water [high]

- **Water level**: KH1's floor query `0x140194920` (the query block at actor `+0x2A0`, run by
  the mover's sub-steps) sets `+0x2D4` to the water surface over his feet (a ray against the
  water layer when the map has one, `0x14234F138`), 123456 when none. The same query fills the
  floor point `+0x320`, the floor face `+0x2DC` (0xFFFF none), the face event `+0x2D8` and the
  floor's surface nibble (bits 18-21 of the face's attribute word) at `+0x360`.
- **Deep**: `0x1402BF530`: the actor can swim (`+0x374` bit `0x1000000`) and its feet are more
  than a set depth under the surface (`+0x14 - +0x2D4` over a table by body size at
  `0x14052AD40`; 50 to 100).
- **Ways in**: `0x1402BF410 (actor, vector)` from the air (fall state) and `0x1402BF100` (a
  glide's bone under the surface) both **refuse world 0xD room 8**: Hook's ship, where the
  fall turns to flight below the room's height instead (`0x140529970`: world 0xD rooms 8 and 9,
  y over 100 and 0). `0x1402BF300 (actor, vector)` from the ground needs him able to act
  (`0x1402956E0`) and **refuses no room**: vanilla only reaches it standing, never over the sea.
  All set state 17 and its motion (slots 45 to 47 in the logs).
- **Splash**: the physics step, after the move: feet at or under the surface and not yet flagged
  (`+0` bit `0x400000`), `0x1402C0020` (the effect only for air states 2-5, 10 and 13); the flag
  is kept while under.
- **Way out**: the swim state (17) has no next-state step; the physics step's own switch keeps
  17 while deep or in the air, else state 0 (2 with flag `0x1000`). Ledges out of the water are
  KH1's climb (13, 16, then 2: the s308 settle).

### What went wrong under KH2 [high for the mechanism, med for each room]

1. Mid-air, KH1's ground step waded him in (`0x1402BF300`) as soon as KH2's move put his feet
   deep, in any room, Hook's sea included. `takeover` let KH2 keep him while airborne (its slot
   test spares a fall), so KH2 carried him on down to the bottom with KH1's state at 17
   ("walked on the bottom"); on landing KH1 had him in its swim and floated him up ("after a
   while it'll put you on the surface swimming"). In Hook's sea that swim has no shore and no
   flight: stuck.
2. KH2's own air swim check (`grab`) ran only in its Air / Jump / Aerial Dodge / Glide states,
   with the stick held or not, only while KH1 would let him act, and only with the ini's
   `grabs` on. An air combo, a spell or a reaction going under had none.
3. No splash: the physics step that plays it doesn't run for him.

The user's own logs show the working path: Traverse Town room 8 (`0x3-0x8`), a KH2 air combo
(motion 181) sank 500 under the surface, then the first Air frame's `grab` took him into KH1's
swim; the Cave of Wonders' water (`0x8-0xD`..`0xF`) the same, out over a ledge (13, 16, 2).

### The fixes (`takeover`)

- `ground_moves_in_air`: on KH2's frame after one with him in the air, KH1's state found at one
  of state 0's input-step moves (9, 10, 12, 13, 17) is undone: state 0, the position KH2's last
  frame left him at, KH1's idle motion asked. Logged.
- `water` (after KH2's move, every frame, any KH2 state, whatever `grabs` says): the splash and
  the in-water flag as KH1's physics step has them; deep: in the air `0x1402BF410`, on the
  ground `0x1402BF300`. One that takes him is a hand-off (`HANDOFF`, as a grab: KH1 keeps him
  until its swim has him standing again, then KH2). Refused in the air in a flight world under
  the room's height: KH1's flight (state 8), as its fall's next step. Logged (entry, refusal,
  flight, and the hand-back "out of KH1's swim").
- KH1's ground step wading him in while KH2 has him on the ground is now a hand-off too.
- `swim_rescue`: state 17 in world 0xD room 8 with flight on is turned into KH1's flight (rise
  speed cleared); a hand-off running ends there, KH1 flies him.

## 3. Pits [med]

- Map exits and walk-in triggers are collision faces with an event number (`docs/kh1-exits-push.md`):
  KH1's mover queues `(kind -2, Sora, event)` whenever a move touches one, and the area script
  plays the exit. In the air only wall-like contacts count (floor events need the ground query).
  The user's own log of the Cave of Wonders (`0x8-0xA` to `0x8-0xE`, KH2 jumping off a ledge
  some 1800 over the floor) shows a fall's trigger firing under KH2's plain jump and fall. [med:
  which room is the Bottomless Hall]
- `gates` keeps KH2's attack lunges out of those triggers: an attack frame's face event is
  dropped and the move undone. But in the air `takeover` then set his height again from KH2's
  displacement, so an **air combo kept sinking through the trigger with its event dropped**,
  under it, into the dark, and then walked to the catch-all exit back to the Hall. (Aerial
  Dodge, glide and plain falls were never gated.)
- Fixes: an undone move keeps his height too (no fall, no ceiling or landing read from it), and
  an air attack coming down more than it lunges isn't gated at all (a pit or a floor is under
  him, not a doorway). Every face event touched while KH2 has him is logged
  (`gates::note_event`: queued or dropped, in the air or not, height, collision flags), and a
  landing with no floor face under him (`+0x2DC` 0xFFFF) too.
- Not found: any fall-out height test in the physics step, the fall state's steps or the event
  queue's other kinds. If a pit still fails, the face-event lines in the log say which event
  was (or wasn't) touched.
- Face attribute `0xB0` (the face record's `+0x10` & 0xF0) makes the query report "in the air"
  even on it and skip its floor event (`0x140193FDC`..`0x140194075`). [med: what uses it]

## 4. Hanging in midair at Hook's fight [med]

- Mid-air, state 0's input step also caught ladders and climbable walls (9 / 10) and, above all,
  **the hang off an edge walked over (13, `0x1402C6760`)** wherever KH2's jump passed an edge
  (the ship's rails): KH2 kept him while airborne and moved him off, and on landing KH1 had him
  in its hang with no edge there. Undone by `ground_moves_in_air`.
- In a flight world, vanilla's fall becomes flight over 100 above the floor (or under the
  room's height) before any of its grabs; KH2's Aerial Dodge and glide still tried KH1's grabs
  there (the mast, the rigging). Now no grab while coming down where KH1 would be flying him.
- Not done: a check that a hang KH1 holds has a ledge (none found in KH1's code to call).

## Steam

New RVAs (Epic, Steam): `0x2BF300` / `0x2C14B0` (the walk-in), `0x2C0020` / `0x2C21D0` (the
splash), in `steam_map.in` (generated).
