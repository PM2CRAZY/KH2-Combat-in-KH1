# Reaction commands

How reaction commands (RCs) are offered and run. The code is in `kh2-bevy/src/rc.rs`, using
data from `kh2-data` and `kh2-formats`.

Confidence labels:
- **Checked (exe)**: read from the executable's code.
- **Data**: read from your install.
- **Ours**: our choice.

## Where they come from (Checked, exe)

An RC isn't a timer or a state given to Sora. **Every frame the best single candidate is
rebuilt from scratch.**

**Enemy RC areas.** These are an object's collision shapes of type 5 ("reaction"):
- **The command:** the shape's argument is the command id.
- **When it is live:** while its collision group is on. A group is on by default when the
  model's collision header says so (usually only group 0), or while the current motion has an
  "enable collision group" window (range trigger 4). Trigger 5 turns a group off.
- **The overlap test:** the area must overlap Sora's body shape (his first type-1 shape: an
  ellipsoid, radius 35, half-height 55, 85 above the feet).

**Choosing.**
- **Priority:** the highest command priority (03system `cmd` byte 0x1e) wins.
- **Ties:** the smaller score wins, then the first one found.
- **Score:** the horizontal distance between the area's centre and Sora's shape. It is ×0.5
  for the lock-on target and ×0.8 for the target already shown (hysteresis).
- **Greyed:** a Land-flag command (flag 0x2) while Sora is airborne is shown but can't be used.

**Low-HP Drive.** When nothing else is offered and HP ≤ max/4, a Drive Form is offered.
- **Commands:** 553 Final, 552 Master, 674 Limit, 551 Wisdom, 550 Valor, if affordable.
- **Which one:** the one already shown is kept, else one is picked at random.
- **Using it** drives normally (Anti roll included).

**No lifetime.** The prompt exists exactly on the frames where these conditions hold.

**Not modelled:**
- the facing and distance checks of the command row (every enemy RC has them off)
- per-object script/motion RC lists (triggers 6/7)
- attack-borne RCs (`trap_attack_set_rc`)
- forced RCs from motions (triggers 20/25/26)
- summon and limit fallbacks

## Running one (Checked, exe)

1. The RC button (pad Y, keyboard L) queues the command when it is usable and Sora can take a
   command.
   - **Ours (simplified rule):** he can take one when free, landing, or in a combo or
     "allow RC" window, and not when hurt, launched or in a scripted motion.
2. **The command's action.** Sora goes idle (action 1).
3. **The enemy's script runs it** (its trigger 11 with Sora, the command and the target). The
   engine plays no Sora motion and grants no invulnerability itself.
4. **The script "captures" Sora** into a motion stored in **the enemy's own motion set**: a
   nested set whose entries are tagged by character.
   - "0010" is Sora's body, "0011" and "0012" his weapons.
   - "0270"/"0271" are for Limit Form.
   - These motions carry Sora's animation, invulnerability (trigger 27) and hitboxes
     (trigger 10, ATKP rows with kind bit 4).

The playground registers every capture motion of the enemy on Sora and on each form, as
motion `1000 + slot`.

## The Soldier's Cyclone (Checked, exe + Data)

**When it's offered.**
- **The areas:** the Soldier (M_EX010) has five RC ellipsoids, all in group 6, command 34:
  radius 700, half-height 300, centred 150 above it, at (±300, 0) and (0, ±450) around it,
  plus one in the middle.
- **When they're live:** group 6 is switched on by its spinning attack (slot 203).
- **The command:** Cyclone has priority 100, the Land and Battle flags, mark 2 and the "watch"
  camera.

**What its script does when it runs.**
1. Captures slot 252 ("AA00") onto Sora: 12 frames, invulnerable. It carries a guard window
   (ATKP 406) on Sora's group 6.
2. While that plays (and is still the current capture), it moves Sora along
   `normalize(roty(Soldier − Sora, 90°))` at **30 per frame**. The game's move request adds
   direction × speed × frame time to that frame's move.
3. Then captures slot 253 ("AA01"):
   - 70 frames, root motion 800 forward
   - invulnerable to frame 108
   - the keyblade's ATKP 245 (power 400, launches) and guard 162 on frames 14–62

In the playground the Cyclone hits for 400% and launches the Soldier.

## Other enemies

`rcct` (03system) lists each enemy's RCs by its status id. **Checked (exe): only Jiminy's
Journal reads it.** Other enemies' RCs need their files and their trigger-11 handlers. Dusk's
Reversal (M_EX990, command 69) is the next candidate.

## Native research (t480 round)

Research for the tester thread `testers/reaction-commands/reaction-commands.md`. Everything above
this section is from the OG rewrite (Epic exe, a Bevy playground); nothing of it runs in the KH1
hook yet (**Checked (code)**: no RC code in `crates/kh1-hook`; `kh2-bevy/src/rc.rs` and
`kh2-data::enemy` captures are the rewrite's). All addresses below are **Steam**.

### KH2's offer, in the Steam KH2 exe

- `0x1404081C0` is the per-frame RC update; it calls the two candidate collectors
  `0x140407B40` and `0x140407DE0` and the best-candidate keeper `0x140408080`. **Checked (decomp)**
  (call graph; what `0x140407B40` collects, likely the other source lists, not read).
- `0x140407DE0 (rc state, object)`: walks the object's collision shapes (filter mask 0x4020).
  A type-5 shape gives its argument as the command (argument 0x83 is replaced by a command from
  the object's `+0x9F0` record `+0x2C`); other shape kinds use the object's `+0xCF0` command.
  `0x1403BA600` checks the shape's group is live; the command record comes from `0x1403B68B0`
  (03system `cmd`); flags `+0xC` bit 4 / bit 6 (bit 6: only for the form whose id is `+0x22`,
  then priority raised to 0xCA when it is under 0xCA). The overlap test is `0x1403C9C00` /
  `0x1403C94C0` against the player's shape. **Checked (decomp)**.
- `0x1404088F0`: the score. Horizontal distance; ×0.5 when the object is the lock-on target,
  else ×0.8 when it is the one already shown; capped at 0x3FFFFFFF. Bit 0x40000000 (greyed / not
  usable) is set when the command's distance check (`+0x10`) or facing check (`+0x18`, around the
  facing turned by `+0x14` + π) fails, or the Land flag (`+0xC` bit 1) is set and `0x1403BA9A0`
  says the player is in the air. **Checked (decomp)**: matches the rules at the top of this doc.
- 03system `rcct` (KH2 install, `local/re-fresh/03system.bin`, 428 bytes): version 1, 35 entries
  of 12 bytes: enemy status id (u16) then up to 5 command ids. E.g. status 1000 → cmd 0x22 (34,
  Cyclone per the section above), status 3000 → 0x45 (69, Dusk's Reversal), status 1002 → 0x24,
  0x25, 0x23. **Checked (files)**. Only Jiminy's Journal reads it (above, Epic finding, not
  re-checked on Steam). The real offer data is each enemy model's type-5 shapes plus its
  script's trigger-11 handler. Which KH2 enemy each status id is was not resolved this round.

### KH1's Triangle box (the context command), in the Steam KH1 exe

- Command menu entries: four bytes at `0x140528914` (slot 0 = Attack's place, 1 Magic, 2 Items,
  3 = the Triangle box). Greyed bits `0x140528904`, blocked bits `0x140528908`, "slot taken" bits
  `0x142D53A3C`. **Checked (decomp)** (Epic `0x140529614`, `docs/kh1-drive.md`, is the same
  array; translated by the decomp's own writes).
- `0x140293000 (slot, command)`: puts a command in a slot if nothing took it this frame (first
  come wins), clears its greyed / blocked bits, marks the slot taken. This is how anything offers
  a context command. **Checked (decomp)**. Its callers: the scan below, KH1's limit offer, the
  default object handler `0x1401A4D00`, object handlers (`0x1402C5D50` offers 0x1C / 0x1D on
  slot 3 and 0x1E on slot 0), and native handlers in `0x14039xxxx` that put level-dependent
  commands in slot 0 (0x47, 0x4D/0x4E, 0x58/0x59, 0x5B, 0x5F/0x60). **Checked (decomp)**; what
  those `0x14039xxxx` commands are (summon-time commands, Assumption) not looked up.
- `0x1402948F0`: the per-frame context scan (from `0x140294220`, the top-level fill). Skipped
  when save `+0x874` bit 0x20, when slot 3 is already taken, or Sora's flag 0x2000000. Otherwise:
  every actor (iterator `0x140291B20`) whose object record (handle at actor `+0x130`) has a
  non-zero interaction kind `+0x5F` (`0x1402918E0`) and is in range (`0x140293FC0`) is listed with
  a score and sorted (qsort, comparator `0x140293190`), nearest-facing first. With the lock state
  `0x142DB3510` ≥ 3 only the locked actor (`0x142DB3518`) is tried. Then by kind: 1 slot 0 (Talk
  in Attack's place; not in battle, `0x142867364` bit 0), 2 slot 3 (Open / Examine), 3 the actor's
  own handler decides (`0x140297CA0 (actor, Sora, -1)`), 4 slot 0. The first actor that gets a
  command wins. Then `0x140268350 (type, Sora)` (the prompt's flash; Assumption) and, at the end,
  KH1's limit offer `0x14028D0F0`, which fills slot 3 only if still free. **Checked (decomp)**.
- `0x140293FC0 (actor, Sora)`: range by the record's shape `+0x5E` (0 cylinder with height, 1
  flat and height, 2 a point from `0x1402B0590`, 3 flat) against radius `+0x44`; score =
  d + d·(1 − cos)/2 with cos the angle between Sora's facing and the actor (−1 = out). **Checked
  (decomp)**.
- `0x140297CA0 (actor, Sora, command)`: the actor's answer. Command −1 = "do you offer one?",
  any other = "run it". Scripted actors (`+0x374` bit 0x10000, machine `+0x144`) get script event
  2 with (Sora's handle, command) through `0x1402BCD60`; actors with a native handler (`+0x4A0`)
  call it; others go to `0x1401A4D00`. **Checked (decomp)**. This is the same shape as KH2's
  trigger 11 (the enemy's script runs the RC).
- Running: the command dispatcher `0x1402A3600` (Epic `0x1402A1470`) and its ground branch
  `0x1402A3DB0`: commands 0x1D/0x1E/0x46/0x51/0x54 and 100 go to the target's handler; any
  command the ground branch doesn't know (its `default`) turns Sora (`0x140297FB0`) and calls the
  target's handler with it. So a new command id (a new record past KH1's, as Reflect / Drive
  added 0x7D..) chosen from slot 3 already reaches `0x140297CA0 (target, Sora, id)` natively.
  **Checked (decomp)**; that the Triangle press runs slot 3 through this same dispatcher is from
  `docs/kh1-drive.md` (Epic) and not re-traced.
- Enemies: whether KH1's enemy object records have `+0x5F` = 0 (so the scan never lists them) is
  an **Assumption** (likely: no enemy offers anything in KH1). Scripted enemies do have the
  event-2 path.

### What it would take natively

1. **The command records.** KH2's RC commands as new KH1 command records (our table extension,
   after 0x80), with KH2's names, priority, Land flag (greyed in the air) and camera as data.
2. **The offer.** KH2's rule as KH1's offer: after `0x1402948F0` decides nothing took slot 3 (or
   as a kind-3 answer for enemies), test each live KH2 RC area of nearby enemies against Sora's
   KH2 body shape, pick by priority then KH2's score (×0.5 lock, ×0.8 shown), and offer it with
   `0x140293000 (3, id)`; greyed bit for the Land flag in the air. One small pass per frame over
   the actors KH1 already walks (cheap; only enemies with an area live).
3. **The areas.** KH1 enemies have no type-5 shapes: they come from the matching KH2 enemy's
   model (as the Soldier's five ellipsoids in group 6) and are switched on when the KH1 enemy
   plays the KH1 motion that matches KH2's group-enabling motion (Soldier: KH2 slot 203, its
   spin). Needs a table KH1 object id → KH2 model, area group, KH1 motion slots.
4. **The run.** `0x140297CA0` hooked for our ids only: Sora plays KH2's capture motion (from the
   KH2 enemy's nested motion set, "0010"/"0011"/"0012") on a KH1 motion request as our other KH2
   rows do, with its invulnerability and its hitboxes as KH1 attack objects; the enemy's side
   (the Soldier's sideways move, launched by Cyclone's ATKP 245) through KH1's hit records.
5. **Low-HP Drive.** KH2 offers a Drive Form in the Triangle box at ≤ 25% HP when nothing else is
   offered; natively it is our Drive records offered in slot 3, and it competes with KH1's limit
   offer `0x14028D0F0` for the same slot.

KH1 readers to keep: Talk / Open / Examine (kinds 1–4, events and scripts answer event 2), KH1's
limit offer, the HUD's Triangle box. KH1's lock rule (only the locked actor is tried) differs
from KH2's (all areas, lock ×0.5): an RC pass of ours would use KH2's.
