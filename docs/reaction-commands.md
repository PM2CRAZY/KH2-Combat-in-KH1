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
