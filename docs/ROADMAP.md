# Roadmap to 1:1

Today the numbers come from your install. What's not yet 1:1 is the *logic*: the rules that
live in the game's executable, not in its data files. There are two ways to pin those down.
Measuring the real game is the main one. Reading the executable in Ghidra comes second, kept
local with notes only, never committed.

## Phase 0: measure the real game (ground truth)

1. **Frame recorder.** A LuaBackend script (already installed in your game folder) that logs,
   once per frame:
   - Sora's position, velocity, facing, motion id and motion frame
   - the buttons and stick held
   - target HP and damage dealt

   The log is a CSV in a gitignored folder. This needs memory addresses for the PC version;
   the randomizer and tracker community already publishes many of them.
2. **Replay and diff tool.** Feeds the recorded inputs into `character-core` and reports the
   first frame where position, state or damage stops matching.
3. **Scenario checklist** to record:
   - stand jump and running jump
   - every growth ability at every level
   - ground and air combos at set distances from a Shadow
   - landing, roll cancels, guard
   - damage on a few known enemies on each difficulty

## Phase 1: settle every hypothesis

Each of these is checked against recordings, or against the executable where measuring is
impractical. Each one gets marked Checked in the docs or gets fixed.

4. Gravity, jump arc, and whether holding jump changes its height.
5. Air control, walk/run threshold and turn behaviour (including mid-air turning).
6. The glide physics formula (descent, acceleration, turning, start and end heights).
7. Dodge Roll, Quick Run and Aerial Dodge: arcs, cancel points and invulnerability.
   Landing recovery length.
8. How the combo table is read: the meaning of `kind`, `sub`, the `flag` bits and
   `combo_offset`; whether swings alternate; combo length rules; how finishers are picked.
9. Attack approach: the lunge toward a target (attack v0 values, s_range), auto-targeting, and
   turn-to-target speed.
10. Damage:
    - how ENMP min/max damage is read
    - critical-adjust scope
    - whether enemy stats come from LVPM
    - ATKP elements 4–6
    - rounding order
    - difficulty multipliers

## Phase 2: skeleton-accurate hitboxes

11. Read the MDLX skeletons of Sora and the Keyblade: bones only, no meshes or textures.
12. Fully evaluate motions: forward kinematics, IK helpers, constraints and expressions, giving
    bone transforms every frame.
13. Attach the Keyblade to the hand bone and put every collision sphere on its bone:
    Keyblade attack spheres, plus Sora's hurtboxes and guard boxes.
14. Draw the skeleton as debug lines, so movement can be compared visually with no assets
    shown.

## Phase 3: enemies and defence

15. Enemy hurtboxes from their own collision data, plus weight and knockback (objentry,
    sstm blow/hit-back values).
16. Hit reactions: map ATKP reaction ids to damage motions, hit-stun, and revenge values.
17. Enemies that attack, using their movesets with scripted timing rather than full AI.
    That lets guard, reflect and invulnerable frames be tested for real.
18. Sora taking damage: his reactions, knock-down and recovery.

## Phase 4: the rest of Sora's kit

19. Combo and finisher abilities (Combo Plus, Air Combo Plus, Finishing Plus, Negative Combo,
    Berserk, Finishing Leap, Retaliating Slash and others), using ability ids from the
    03system item table.
20. Keyblade stats and equipment (03system items), and Sora's real HP/MP.
21. Lock-on and the KH2 camera; ledge hang (hang_height/margin); ceilings and slopes.
22. Magic (MAGC, spell movesets, MP).
23. Drive Forms: each form's moveset and movement record, form levels and Anti.
    Then Limits, Summons and Reaction Commands.

## Phase 5: lock it in

24. Regression tests driven by your recordings. The recordings stay local, and the tests run
    when `KH2_TRACES` is set.
25. Every rule in `combat-model.md` and `movement-model.md` marked Checked, or explicitly
    listed as a known difference.

## Needed from you

- Running the recorder script while playing the scenarios. I'll write the script and the
  list of moves to do.
- A yes or no on looking inside the executable with Ghidra for the formulas that are hard to
  measure, such as gravity and how the combo table is read. That work stays on your machine,
  and only plain-language notes are committed.
