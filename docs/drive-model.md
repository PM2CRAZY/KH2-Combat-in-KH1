# Drive Form model

How Drive Forms work in this project. The rules are in `character_core::drive`; the
playground's form change is in `kh2-bevy/src/drive.rs`.

Confidence labels:
- **Checked (exe)**: read from the executable's code.
- **Data**: read from your install.
- **Ours**: our choice.

## The forms are separate player objects (Checked, exe + Data)

Each form is its own object in `00objentry.bin`, with:
- its own model and motion set (`obj/P_EX100_<suffix>`)
- a moveset id, which picks:
  - its `prty`/`plyr` records (speeds, jump, auto-target range)
  - its PTYA combo table
  - its weapon motion sets in 03system `wmst` (row `moveset × 2 + hand`)

| Form | id | Object | Moveset | Weapons (wmst) | Run speed |
|---|---|---|---|---|---|
| Valor | 1 | P_EX100_BTLF | 2 | W_EX010_BTLF_R / _L | 12 |
| Wisdom | 2 | P_EX100_MAGF | 3 | W_EX010_MAGF | 12 |
| Limit | 3 | P_EX100_KH1F | 0x42 | W_EX010_KH1F | 8 |
| Master | 4 | P_EX100_TRIF | 4 | W_EX010_TRIF_R / _L | 10 |
| Final | 5 | P_EX100_ULTF | 5 | W_EX010_ULTF_R / _L | 16 |
| Anti | 6 | P_EX100_HTLF | 6 | none | 16 |

**Weapons.**
- Both hands' weapons attach to the bones in `sklt` (right 178, left 86).
- Their motions move them, so Master's and Final's floating keyblades come out of the data.
- Attack windows name the weapon they belong to: body, first weapon or second weapon.

## Drive gauge (Checked, exe)

- **Bars.** One bar is 100 points, up to the maximum (9 at most).
  - When full, the gauge holds at 100 points in the last bar.
  - Paying a cost keeps a part-filled bar unless the gauge was full or short.
- **Sora's hits** add the ATKP row's drive value (byte 0x29):
  - ×0.75 against normal enemies
  - ×2 while MP charges (+0.2 per Drive Boost)
  - truncated, so a value-1 hit on a normal enemy adds nothing
- **Ours:** spell hits add drive too (the game's handler runs for Sora's attacks; that it
  covers spells is inferred).
- **Drive orbs** (the prize table at `0x1405C7590`): a small orb (KH2's prize type 7) adds 1
  point, a big one (8) 3, through the same add as a hit's (no ×0.75). In a form an orb adds
  `value × 12` frames to the form gauge; in Anti Form it takes them away.
- **Damage taken** (the HP-change handler): out of a form, `damage / max HP × A` points, where
  `A` is 0 without Damage Drive and 50 per copy of it. So vanilla KH2 Sora gains nothing from
  hits he takes. In a form, damage never touches the form gauge.
- **Items** (`docs/kh1-items.md`): Drive Recovery adds 300 points, High Drive Recovery 900, through
  the same add (KH2's item use, items 274 / 275); usable while the bars are below the maximum.

## Driving (Checked, exe)

1. **Costs** come from 03system `cmd` byte 0x1c: Valor 3, Wisdom 3, Limit 4, Master 4,
   Final 5, Anti 9.
   - A command is unavailable when the gauge is short, or Sora is already in a form or
     changing.
2. **The Anti roll.**
   - A counter rises by 1 with every drive that isn't Final.
   - The Anti chance is 0% below 5, 10% from 5, and 25% from 10 (fmlv Anti rows).
   - A Final drive lowers the counter by 10; becoming Anti lowers it by 6.
   - Anti happens only in battle. **Ours:** "in battle" means an enemy is up.
3. **The cost of the resulting form is paid.** Anti costs 9, which empties the gauge.
4. **Sora's Drive motion** (0x34, 160 frames) plays, invulnerable. The screen tints grey:
   12 frames in, then 15 frames out after the swap.
5. **The swap.** The form object replaces Sora, keeping his position, facing and lock-on.
   - It plays its arrival motion (0x35).
   - HP and MP go to full, and MP Charge is cancelled.
6. **Form gauge** = `(form level + 3) × 600` frames. Anti uses 6 bars.

**The change's effects in the KH1 port (s331; Data + Log).** The player: "The transform effects in
DriveForms aren't working; Sora only transforms."
- What the logs show (s315 to s328, every Drive): motion 52's frame-0 triggers start Sora's package
  elements 1..13 ("KH2 effect: element 1 of motion 52 started" ... 13), the form's arrival (53)
  starts its package's 1..12 and 100..102, and "KH2 effects: 12 ended with Sora's object (a form
  change)" at the swap. Nothing broke them in a recent build: `form.rs`'s Drive start (s318), the
  partners' vanish (s312) and the reverts (s314, s315) leave the effects alone.
- The data (P_EX100's `p_ex`, Data): element 1 (effect 1, flags 0x3, on the object root 112 up) is
  the orb: a sphere round Sora from tick 9 that bursts into shards and sparkles, 120 frames in all;
  it is not one that ends with its owner, so it plays out across the swap. Elements 2..13 (flags
  0x241: end with the owner, fade-out 0) are the glows on his limbs, looping; KH2 ends them when the
  form's object replaces his, as the port does at its swap (frame 39 of the user's 45-frame change).
  The revert's 15..27 are the same pair: a shrinking sphere and burst (15, sound 115), then glows
  on his limbs (16..27, flags 0x241), looping.
- What was missing, now done (`kh2fx`):
  - KH2's grey screen tint (step 4 above; `kh2-bevy`'s `drive.rs`): RGB 0x30 over the screen at
    0.5, in over 12 frames from the Drive motion's start, out over 15 after the swap (or at once
    from a change cut short), drawn first in the effects' pass (the orb and the arrival draw over
    it). Without it the orb's additive light washed out over KH1's bright fields.
  - The revert's limb glows looped on base Sora until the next form change (their owner never
    goes): ended 60 frames after the revert starts them (ours; KH2's call for them isn't traced).
  - One log line per change ("KH2 effects: the Drive's orb and glows (elements 1..13) started on
    Sora", "... the revert's light (elements 15..27) ..."), as the first 20 starts' lines had long
    run out by the time a player drives.
- To check in game: Drive into each form (the screen dims, the orb closes round Sora and bursts,
  the form appears with its arrival light and aura, the screen comes back); revert (the revert's
  sphere and burst, the limb light gone after a second).

## In a form (Checked, exe)

- **Draining.** The form gauge drains 1 per frame in battle, 0.5 outside (×0.8 per Form
  Boost).
- **Hits drain it too:** each of the form's hits takes `15 × drive value` frames, ×0.2 on
  normal enemies.
- **Magic.**
  - The `sub` gets +10 in Wisdom, +20 in Master and +30 in Final, which picks the spells'
    form variants and `magi` records.
  - Valor, Limit and Anti can't cast (the command's form mask).
- **Abilities.**
  - Sora's support abilities carry over (Combo Plus and the like). His action and growth
    abilities don't.
  - Each form brings its growth ability at its level: Valor High Jump, Wisdom Quick Run,
    Limit Dodge Roll, Master Aerial Dodge, Final Glide. Form Lv1/3/5/7 give LV1/2/3/MAX.
  - **Ours:** every row of the form's combo table is unlocked, and the form guards if its
    motion set has the guard motion.
- **Anti** takes 1.5× damage.
- **Reverting** happens when the gauge empties, on the Revert command (the Drive row reads
  Revert in a form), or on a KO.
  - Sora comes back at once and plays motion 4.
  - There is no refill and no cooldown.
- **Form EXP** (fmlv table, levels capped at 7):
  - Valor: +1 per hit
  - Wisdom: +1 per Heartless defeated (status ids 1000..2011, not 1047..1059)
  - Final: +1 per Nobody defeated (3000..4014)
  - Master's rule (drive orbs) and Limit's (limit finishers) aren't modelled.

## Wisdom Form's shots (Checked, exe + Data)

Wisdom's attacks have no hitbox windows. Their motions fire frame trigger 1 with an effect id
(25..43).
- **Where the logic lives.** Wisdom's model file has a script (`wisd`). The game runs its
  trigger 0x10 with the object, the id, and the effect started from the form's effect archive
  (`obj/P_EX100_MAGF.a.us`).
- **What the script does.** It fires one homing shot per id:
  - ATKP 859, sub 0, 3 or 5
  - a sphere of radius 50
  - speed 20 rising by 2 per frame to 80
  - turns 1.8° per frame toward Sora's target, and stops homing within 100
  - lives 24 frames plus an 8-frame fade
  - stops on a hit or a wall
- **How we run it.** `kh2-magic` runs this script like a spell (`Magic::object_effect`). One
  trap was added for it: `obj_effect_start`.
- **A shot's hit is the swing's (s304; the rule ours, KH2's not traced).** The user, on a
  level-14 save without Combo Master: "in wisdom form, if a hit connects, it doesn't go into the
  full combo, it just does the first hit over and over".
  - The combo rows (Data): ground 151 (row 5) repeats until the count reaches the combo's
    length, then the finisher 154 (row 3); air 181 (row 9). Rows 5, 3 and 9 name themselves as
    their `sub`, so 151 may follow 151. 151 shoots at frames 10, 20 and 30 (triggers 25..27)
    and its combo window opens at 36; 181 shoots at 10, 20 and 30 (41..43) and has no combo
    window, so it always plays out.
  - The cause: without Combo Master, `Abilities::one_swing_on_miss` (ours, the KH1 port) lets a
    swing chain only once it has connected (`register_hit`). Melee swings report their hits from
    `combat::Strikes::strike`. Wisdom's swings have no hit windows, and their shots' hits
    (`formmagic::shots_step`, through `Strikes::spell_strike`) never reached the controller. So
    every 151 counted as a miss, refused each press, played out its three shots and went back to
    151. The s304 log shows it: the shots land ("Wisdom: ATKP 859 hit ... damage Some(2)") and
    the next presses come in 151's window, but 151 runs to its end and the next swing is 151
    again, never 154. With Combo Master (the user's earlier level-100 save) every swing counts,
    which hid it.
  - Now `shots_step` returns whether a shot of the form's model script (not a spell's) landed
    this frame, and `takeover` calls `register_hit`. A shot hit counts for the swing playing
    when it lands; a 151 cut at frame 36 may still have shots in flight, which then count for
    the next one.
  - Wisdom's spells already count: a cast takes its combo slot as a connected step. Limit
    Form's limits (Strike Raid, Infinity) play their motions by script, outside the combo, and
    are unchanged. Master and Final hit with their Keyblades' windows, as base Sora does.
  - Sim test: `wisdom_shots_landing_carry_the_combo` (151, 151, 154 with the shots landing 12
    or 30 frames after they are fired; each 151 alone, played out, when none lands).

## Final Form's floating Keyblades (Checked, Data)

The user: "sora's keyblades dont spin properly during final form glide. also his keyblades in
final form glide and in final form jump should do damage".

- **Where the spin is.** KH2's Keyblade model (`W_EX010.mdlx`) has its blade on bone 2: the mesh
  and every hit shape are there. Bones 0 and 1 place it; 3 and on are the keychain. Final's
  weapon motions spin bone 2, not the root:
  - glide (205..207): bone 2 turns about its z by −12π over the 60-frame loop (6 turns), while
    the root holds the Keyblade still beside him;
  - idle (0): bone 2 rolls once about its x over 160 frames.
- **Why ours didn't spin.** `leftblade::final_float` and `note_pose` moved KH1's Keyblades by
  KH2's weapon root only, so the blade's own turn was lost. They now follow bone 2
  (`KH2_BLADE`), matched against bone 2 of base Sora's held idle. Bone 2 is not animated in the
  other motions, so those look as before. Valor still follows the root.
- **Where the left one hangs from.** The user: "when sora hovers on the ground, the
  second keyblade follows him but it jitters a little. also, when gliding and turning, the
  second keyblade goes to the same position as the first keyblade".
  - The data is not the cause. `W_EX010_ULTF_L` has its own clip for every motion
    `W_EX010_ULTF_R` has (0..19, 40, 53, the casts, 151..154, 181..183, 205..208), and no two
    are the same. The body's motion 139 has no clip in either hand's set; both then play
    motion 0, as `kh2_rig` does for any weapon (what KH2 does there is not checked).
  - KH2's own spacing (`kh2-anim/examples/final_blades`, blade bone 2 in the body's space):

    | Motion | Apart | Left blade from the left hand |
    |---|---|---|
    | Idle (0) | 64..80 | about 38 |
    | Glide (205) | 140, one on each side | about 95 |
    | Banked turns (206, 207) | about 35, both on one side of him | about 115, across his body |
  - The cause: each Keyblade was put on KH1's hand of its side, by its KH2 hand bone. KH1's
    hands are rebuilt from KH2's bones, not copied, so each turns a little differently from
    KH2's. A Keyblade floating 40 to 120 units from its hand swings by that difference. The
    left hand's wobble showed as the jitter, and in a turn the two hands' differences came to
    the 35 units between the blades.
  - Now, in Final Form, `note_pose` hangs the left Keyblade from KH2's right-hand bone, on
    KH1's right hand, as `final_float` hangs the right one. The two keep KH2's spacing exactly,
    and the left is as steady as the right. Valor's Keyblades are held, so each still hangs from
    its own hand.
- **The attacks.** Final's weapon motions carry their own attack triggers (range trigger 10,
  group 2 on bone 2, three spheres of radius 60):

  | Motion | Right / left ATKP | Frames | Power | Re-hit interval |
  |---|---|---|---|---|
  | Jump (3) | 1273 / 1274 | 2..12 | 20 | none (once) |
  | Jump (3) | 1286 / 1287 | 12..end | 10 | 10 |
  | Fall (4) | 1275 / 1276 | 2..20 | 20 | none |
  | Land (5) | 1277 / 1278 | 0..20 | 20 | none |
  | Glide (205..207) | 1281 / 1282 | all | 100 | 30 |
  | Hover (208) | 162 (a guard, kind 2) | 1..end | 0 | — |

  KH2's weapon object plays its motion whatever Sora's action, so these hurt in the jump's rise,
  the fall and the glide. Our controller runs windows only for actions (the jump's push, the
  landing). `kh2_rig::MotionHits` reads the windows of the clip the rig shows at its frame
  (weapons only, attacks only) while KH2 has him jumping, in the air or gliding. `takeover`
  hands them to `combat::Strikes` with the controller's own, replacing the jump action's copies.
  A window open from frame to frame keeps its swing id, across the glide's banked turns too. So
  a foe is hit once per window, or again after the row's interval. They land through KH1's
  contact code with the row's power, as the swings do. The hover's guard is not modelled.
  No other form has attack windows in these motions.

## Final Form's magic (Data; the rules ours, as Wisdom's)

The user: "lets try to incorporate our rules, logic, and process from wisdom forms' magic, and
bring it to final form, with final form's animations".

- **No spell files of its own.** MAGC has rows for worlds 0 and 10 only (10: the `…lk.mag`
  variants), none per form. KH2's Final casts Sora's `magic/<SPELL>_<level>.mag` with its sub
  + 30. 03system `magi` has records 30..35, and the spells' ATKP rows (248..251, 573.., 708..)
  have subs 30..35, as Wisdom's have 10..15.
- **Its cast motions.** MAGC's row gives the same motion ids in every form. Each moveset fills
  them with its own clips. In `P_EX100_ULTF.mset` these are 60 fps clips; their lengths below are
  in 60 Hz frames.

  | Spell | Ground | Combo finisher | Air |
  |---|---|---|---|
  | Fire | 56 `E362` (76) | 57 `E363` (92) | 58 `E372` (68) |
  | Blizzard | 59 `E360` (50) | 60 `E361` (104) | 61 `E370` (42) |
  | Thunder | 62 `E364` (50) | 63 `E365` (100) | 64 `E374` (42) |
  | Cure | 65 `E362`, as Fire's | none | 67 `E372`, as Fire's |
  | Reflect | 71 `E362` | 72 `E363` | 73 `E372` |

  `E362` carries the magic-cast event for 56, 65 and 71, and `E372` for 58, 67 and 73. So Final's
  Fire, Cure and Reflect share one cast. Every Final cast has an allow-movement range all through.
- **How we cast them** (`form::Kind::magic_sub`, `form::casting_form`):
  - Fire, Blizzard and Thunder are KH2's. They take Wisdom's path through
    `formmagic::spell_chosen` and `before_step`: the MAGC motions, the controller's combo slots
    and cast windows, the script at sub level × 2 − 2 (+ 1 for the finisher) + 30, its effects
    and sounds, and hits with KH1's own data for the spell at its level. Level and cost are
    KH1's.
  - Cure is KH1's own, drawn with Final's Cure motion.
  - Gravity, Stop and Aero are KH1's. Gravity is drawn on Final's Thunder motion; Stop and Aero
    on its Cure motion. The air ones use the air motions. Each runs on that motion's clock,
    input window and movement (`takeover::command_started`'s `FormCast`, `pose::follow`).
  - Reflect runs at Final's sub (+ 30) now, as Wisdom's runs at + 10.
  - In the air, the controller's casts use KH2's own physics for Final (`takeover`'s
    `in_final()` air-magic block). KH1's spells in the air are handed off as they are in
    Wisdom: KH1's air state moves him while Final's air cast motion is drawn.
  - Final has no shots. Its frame triggers 1 stay its own elements (`kh2fx::form_event`).
- **Unchecked in game:** all of it. In particular, whether Final's 60 fps cast clips line up
  with KH1's cast timing for the KH1 spells.

## Anti Form in the KH1 port (Data + exe notes above; the rules ours where marked)

The user: "I want you to add anti form. I want 1:1 combat parity. I dont want it to cost any MP
to enter, and I dont want sora to be able to revert out of it." Then: "Drive forms in this mod
run out when you're out of MP, so sora will revert when he is out of MP". Since s285 it takes
half his MP to enter, rounded down (the user: "Antiform should take half of your MP"), and still
starts at any MP. Since s318 it costs Drive bars instead: it empties the gauge (KH2's 9) and
wants 3 bars to start ("The Drive gauge in KH1" below).

- **How he gets it (ours).** Only by the secret button code (`form::pad`, read in `probe::pad`).
  It is never in the Drive list. The code is taken when KH2 could change him (free, jumping,
  falling, landing or at a move's end) and he isn't in a form or changing. KH2's Anti roll (the
  counter above) isn't used.
- **Its data (Data, checked on extracted files).** `P_EX100_HTLF`, moveset 6:
  - the same 256-bone skeleton as Sora's (`pose::load_form` checks this);
  - no weapon sets (`wmst` has none for moveset 6), so it holds no Keyblade. Its attack windows
    are on its body's own shapes: see "Its attack shapes" below;
  - `prty`: run 16, jump 185, turn 0.26;
  - 13 combo rows. Rows 11 (motion 173) and 12 (motion 161) are kind 0, reached only by id
    (its Square moves, below);
  - motion 40 (Aerial Recovery) but no motion 42 (the guard's), so no guard;
  - its own sounds (bank 503, frame-trigger sounds) and frame-trigger effects from its own
    archive (`obj/P_EX100_HTLF.a.us`, `kh2fx::ANTI`).
- **Its attack shapes (Data, the Steam install's `obj/P_EX100_HTLF.mdlx`).** Anti has its own
  collision table (34 entries, BAR entry 23), not Sora's (36). Its hands and legs are its weapons:
  every hit window is in the body's motion set and switches on the body's shapes of its group.
  Bones are Sora's skeleton's (`kh2-bevy` `rig.rs`: arms 76..99 and 168..191, legs 11..17 and
  40..46). Attack (type 6) entries, by group:
  - 1: sphere r150 on bone 67 (the spine). Windows 154:477, 155:474, Side Spin 161:542 and 161:544;
  - 2: sphere r400 on bone 67 (154:478, 184:555, 184:556);
  - 3: sphere r80 on bone 2 (40:454, 151:476, 183:454 / 455, 184:455);
  - 4: right hand, spheres r55 on bone 85 and r35 on bone 110 (151:475 / 530, 152:459, 182:548);
  - 6: left hand, r55 on 177 and r35 on 202 (53:124, 151:456, 153:534, 181:549);
  - 7: right leg, r60 on 18 and r40 on 11 (152:457 / 531, 181:550);
  - 8: left leg, r60 on 47 and r40 on 40 (152:531..533, 153:460, 181:551, 182:545 / 547);
  - 10: sphere r120 on bone 2 (153:458, Side Spin's spin 161:543, 181:552);
  - 11: a column on the root, r100 (183:553); 12: r80 on 41 and r50 on 67 (182:479 / 480 / 546);
  - 13: a column on bone 2, r260 (155:538, 183:554). No group 5 or 9.

  (motion:ATKP.) Every group Anti's moves use has a shape, so all its windows can land. The
  Square-move helper read Anti's table as Sora's: `local/re`'s `.mdlx` files came from a modded
  install (the Epic one's live archives, whose form models carry Sora's 784-byte table; its
  `backup` and the Steam install have the originals). The port reads the Steam install (Mods
  Manager backup first), so this was never wrong in game. At load, `pose::log_shape_groups`
  logs each form's groups, `KH2 Anti Form attack groups b1 b2 ...: each has a shape`, or
  `NO SHAPE for` a group with its windows (`Rig::shape_groups`; a modded model would show it).
  The other forms' groups all have shapes (Data, same install): Valor body 6, both Keyblades
  2, 3, 5, 6, right also 4; Wisdom body 6 (its shots are its script's); Limit body 3, 6, 20,
  Keyblade 1, 2, 4, 6; Final body 2, 3, 6, 19, both Keyblades 1, 2, 4, 6; Sora body 2, 3, 6,
  18, 19, 21, 22, 23, Keyblade 1, 2, 5, 6. `kh2-data` test `every_attack_group_has_a_shape`
  checks it on files in `KH2_LOCAL_DATA`.
- **Square (Data + KH Wiki; the user: "lets add anti-form's square move").** Anti's PTYA rows 11
  and 12 sit where Sora's guard (11, motion 173) and Upper Slash (12, motion 161) are, with
  ability 0 (Sora's need 1 and 18). In Anti's motion set they are its own moves. The KH Wiki's
  Antiform page names them: "By pressing Square on the ground, Sora jumps backwards, spins around
  and flies forwards. If there are enemies he will home in on one and move into a position behind
  them" (Anti Glide), and "By pressing Square in the middle of a ground combo, Sora blazes in a
  circle around the enemy" (Side Spin).
  - **Anti Glide** (row 11, motion 173, F311; 108 frames at 60 fps). No hit and no guard window.
    Invulnerable on frames 0..8; no gravity from frame 2. Its root motion goes back about 270 and
    up about 95 by frame 24, then forward about 1250 by frame 80 (from the start). Turn-to-lock
    15° / 30° / 15° per frame on 52..62 / 62..84 / 84..100, so with a target he turns onto it as
    he passes it. Frame 0: trigger 21 (360°, the start's snap); frame 2: effect 27 and a footstep;
    sounds 503 038 (frame 4) and 503 039 (frame 56). Range 2 (the free state) from frame 92.
  - **Side Spin** (row 12, motion 161, F310; 120 frames). Three hit windows on his body's
    shapes: ATKP 542 (frames 9..19, group 1: power 50, physical, hit sound 503 003), 543 (26..94,
    group 10: power 20, element 4, re-hit every 8 frames) and 544 (94..120, group 1: power 20,
    element 4). All three have refact self 2 (they go on through a clash). Combo window 78..94,
    turn-to-lock 22° per frame on 0..90, no gravity from 94 (it ends on a hop, root up 67). Effect
    17 at frame 24. Its root motion circles (about 250 forward, swinging ±100 sideways).
  - **Ours, in the port.** Square on the ground, standing or moving, plays Anti Glide: the
    controller's guard-row path takes row 11 when its motion has no guard window (as Valor's,
    whose row 11 is an attack). Square in a ground combo's window plays Side Spin
    (`Character::square_swing`, set by `form::arrive`, `ANTI_SIDE_SPIN_ROW`), one swing of the
    combo as Upper Slash is: only while a swing is left before the finisher. Otherwise Square goes
    to Anti Glide, as Sora's Square goes to his guard. In the air Square does nothing (Anti has no
    air Square row). Both play through the attack path: their motions, root motion,
    invulnerability, turns, hits, effects and sounds come from the data. Neither guards: Anti
    Glide has no guard window, and s283's rule stands.
- **Growth (Data).** 00battle `fmlv`'s Anti rows give no ability at any level, so it has no
  growth ability. None of Sora's growth abilities carry over, as in every form, and the
  one-level-up rule (`form::growth_up`) doesn't apply. His support abilities carry over as in
  the other forms.
- **What it can't do (exe: the command's form mask, as Valor's magic).** No magic, items,
  summons or Drive / Revert. In the port, KH1's specials (Sonic Blade, Strike Raid ...) are
  refused too, as KH2's Limits and reaction commands are. All of them get KH1's "can't" sound:
  - `combat::command` and `exec` refuse the command;
  - `combat::spell_not_yet` refuses the menu entry and the menu stays open;
  - `drive::fill` greys Magic, Items, Summon and the Drive row.
  **Ours:** the field's context commands (talking, examining) stay, as in every form (the
  user's earlier rule for Drive Forms).
- **Damage taken (exe notes, `DriveParams::anti_damage`).** 1.5 times. This is a constant in
  the executable, not in the data. `hurt` raises each attack's damage in KH1's hit record
  (`+8`) by 1.5, rounded up, before KH1 applies it. Drains and heals are left alone.
- **Entering and its end (ours, the user's rules).**
  - It empties the Drive gauge (KH2's cost 9) and wants 3 full bars to start (s318; it took half
    his MP from s285, nothing before).
  - Otherwise it changes as the other forms do: Sora's Drive motion, HP to full, "Give me
    strength!", and the arrival.
  - It runs on its form gauge, KH2's 6 bars (3600 frames), drained as every form's is; Drive
    orbs take from it instead of adding (KH2's rule).
  - When the gauge runs out, it reverts. A KO reverts at once.
  - Revert can't end it: the Drive row is greyed, and `form::choose` and `drive::revert_row`
    refuse it.
  - KH2's own end (its 6-bar form gauge, hits draining it) isn't modelled. KH1 has no Drive
    gauge.
- **Look.** Shadow Sora's look comes from `tint` (separate work). KH1's Keyblade is shrunk
  away on his draw (`leftblade::queue`, `model::hide`). With KH2's player model, the HTLF body
  is drawn.
- **Unchecked in game:** all of it. In particular:
  - Strike Raid's thrown Keyblade as drawn (its turn and spin on KH1's Keyblade, KH1 drawing it
    far from him), its trail, the catch's flash; how far it flies (about 1300 for Raid: KH2's
    30 a frame for the rest of the throw motion);
  - Infinity's ring of shots and their hits, its push;
  - Last Arcanum's glow now on the Keyblade;
  - whether the HTLF effect archive exists on the install;
  - Anti Glide while moving: the exe notes have row 11 tried by the action button "when not
    moving"; it is taken moving too here (no Dodge Roll in Anti; Valor's rule);
  - Anti Glide's homing: how KH2 puts him behind the target (only its root motion and turns
    here, so how far past the target he ends depends on the distance), and its free state from
    frame 92 (range 2 isn't read, so it plays to its end at 108);
  - that Anti's claw and kick hits land and their reach looks right (the shapes above, read
    from the install; the load log line says whether every group found one);
  - the wiki's "can also deflect many attacks automatically": not traced (no guard window in
    Anti Glide or Side Spin);
  - whether `+8` in KH1's hit record is the final damage.

## Limit Form in the KH1 port (Data; the rules ours where marked)

The user: "lets begin working on limit form". Its unlock is the user's; its cost is KH2's 4 Drive
bars since s318 (1 MP before, as Valor and Wisdom). It opens by the Dive's choice ("When the
forms open" below: first for the Shield, second for the Sword, third for the Rod). It is the Drive
list's third entry, after Wisdom.

Read from the extracted `00battle.bin`, `03system.bin`, `00objentry.bin`, `msg/us/sys.bar`,
`obj/P_EX100_KH1F.mdlx` / `.mset` and `obj/W_EX010_KH1F.mset`, and from the install's file
lists (`.hed` names). Nothing of it is in the repository.

### Its data

- **Object.** `P_EX100_KH1F` (objentry 0x95D; the `_NM` / `_XM` / `_TR` / `_WI` variants
  0x95E..0x961 share its motion set), weapon joint 1 (right hand bone 178, as Sora's).
- **Moveset 0x42.** `wmst` row 0x42 × 2 gives `W_EX010_KH1F`, right hand only (rows 0x43..0x45
  name it too). One Keyblade: Sora's own, as in base form.
- **Skeleton.** 256 bones, the same as Sora's, so the pose's retarget is unchanged.
- **Movement (`prty` 0x42).** Walk 2, run 8 (Sora's), jump 285 (Sora's 185), turn 0.26, no
  acceleration / brake. `plyr` has only 59 pointers, so there is no record for 0x42. **Ours:**
  `kh2_data::sora::attack_params_for` takes Sora's (auto lock-on 300). Which record KH2 reads
  isn't traced.
- **Model.** BAR entries `p_ex` (model), `tim_` (textures), `limi` (type 3, an object script of
  12148 bytes: its Limit commands, below), and a type-23 entry. It looks like KH1 Sora.
- **Effects and sounds.** `obj/P_EX100_KH1F.a.us` is on the install (in `kh2_sixth`'s list). It
  isn't extracted here, so whether it has a "p_ex" PAX isn't checked; `kh2fx::form_library` falls
  back to the archive's first. The motions' sounds are the Keyblade bank's (500 0xx), plus
  footsteps and voices.
- **Motions.** Body clips 0..19, 40..43, 53, 137, 139, 151..155, 162, 164, 173, 181..183, 191,
  209, 211..213, 221..227, 231..234, 241..248. The tags of the form's own motions start with K
  (K303 = 154, K304 = 155, K322 = 173, K332 = 183, K400.. = 211.., K410.. = 221.., K420.. =
  231.., K430.. = 241..). There is no motion 52 (the Drive change is Sora's), but there is 53
  (its arrival: elements 1..12 and 100..102 from its archive). Motion 42 (the guard's repel, a
  guard window, ATKP 162) and 209 (Dodge Roll) are there. `kh2_data`'s moveset now reads motions
  up to 255, as posing does (it stopped at 230, so Strike Raid's and Infinity's had no clips); no
  other moveset has motions past 230, so the others are unchanged. The Keyblade's motion 235 (no
  body motion) is the thrown Keyblade's own, which its script starts on the weapon.
- **Combo rows (PTYA set 0x42, 21 rows).** No row needs an ability.

  | Row | Kind | Flags | Motion | ATKP (power) | What it is |
  |---|---|---|---|---|---|
  | 0 | 1 | 0xA | 191 | 909 (65, 25) | jumping slash into the air (target 0..260), then the fall |
  | 28, 43 | 1, 2 | | 164 | 968, 975 (100, 25) | the far dash (180 / 300..650) |
  | 6 | 1 | | 153 | 128, 147 | 180..300 |
  | 4 | 2 | | 152 | 127 | 0..180 |
  | 35 | 1 | | 162 | 890 (125) | 0..120 |
  | 5 | 3 | | 151 | 126 | the ground default |
  | 2 | 2 | finisher | 155 | 1870 (200, Magic, element 5, body group 20) | **Ripple Drive** ("when surrounded", "relative to magic"): any target within 500 |
  | 3 | 3 | finisher | 154 | 1877 (600) | **Zantetsuken** ("on a single target ... instantly eliminate weaker enemies"): the default finisher |
  | 10 | 1 | air | 182 | 132 | |
  | 9 | 3 | air | 181 | 131 | the air default |
  | 8 | 3 | air, finisher | 183 | 1871 ×3 (100, 25, 25; re-hit 10), 1872 (100) | **Hurricane Period** ("an aerial barrage") |
  | 11, 12 | 0 | | 173 | guard window 161 | the guard's press (Square standing) |
  | 39 | 0 | | 154 | 1877 | the **Zantetsu Counter** ("Unleashes Zantetsuken by selecting Attack while guarding"): KH2's counterguard row |
  | 1 | 0 | | 181 | | |
  | 52..56 | 0 | air | 242..246 | | Infinity's (by id only) |

  The finisher names come from the ability items 573 Zantetsuken, 574 Ripple Drive and 575
  Hurricane Period, matched to the rows by their help text and ATKP. KH2 gates none of them by
  ability.
- **Growth: Dodge Roll (Data).** Items 564..567 (Dodge Roll LV1..MAX, ability numbers 181..184).
  00battle `fmlv` form 3 (Limit) gives the growth level 1, 1, 2, 2, 3, 3, 4 at form levels 1..7.
  Its other level-up abilities are Lv2 Auto Limit (568), Lv4 Draw (405) and Lv6 Lucky Lucky (407).
  Its level EXP column reads 3, 6, 12, 19, 23, 27. Its experience rule (sys.bar): "For each limit
  command finishing move unleashed in Limit Form, 1 point of experience is gained". Dodge Roll's
  invulnerable frames by level (`fmab`): 10, 16, 24, 32.
- **Drive cost in KH2 (for reference).** 4 bars: 03system `cmd` 0x2A1 "Limit Form" (execute 4,
  argument 3, the form id), and 0x2A2 "Limit" (Auto Limit's reaction command), both cost 4.

### Its commands in KH2 (Data, and its script `limi` read)

- **Magic is replaced by "Limit"** (`cmd` 0x2C3, execute 707, submenu 12, icon 7). Its list:

  | Limit | `cmd` | MP | Follow-ups (`cmd`, no cost) | Help (sys.bar) | Motions (from `limi`'s motion calls) |
  |---|---|---|---|---|---|
  | Sonic Rave | 0x2BA (698) | 60 | Rave 0x2BB, Finish 0x2BC | "attack on an enemy as you pass by. Press [it] at the right tempo for a combo" | 211 (K400) into 212 (K401), 212 again per Rave, 213 (K402) the Finish |
  | Last Arcanum | 0x2BD (701) | 75 | Bash 0x2BE, The End 0x2BF | "a powerful combo attack. Press [it] for a consecutive combo" | 221 (K410), 222..226 one per Bash, 227 (K416) The End |
  | Strike Raid | 0x2C0 (704) | 65 | Raid 0x2C1, Judgment 0x2C2 | "throw the Keyblade at the target. Press [it] for a combo" | 231 (K420), the throw 232 (Raid) / 233 (Judgment), the catch 234; the Keyblade's own 235 |
  | Infinity | 0x2AB (683) | 80 | Shoot 0x2AC, Impact 0x2AD, Giga Impact 0x2AE | "can be unleashed by pressing [it] after an aerial combo" | 241 (K430), combo rows 52..56 (242..246), 247 (Shoot, Impact) / 248 (Giga Impact) |

  `00battle` `limt` rows 27..30 name these limits' files (`sonic_rave.bar`, `last_alcanam.bar`,
  `strike_raid.bar`, `ragnarok.bar`). On the install those are `limit/us/*.bar`, all four 72480
  bytes. That is likely their menu banners: they aren't extracted, so this isn't checked.
  Infinity's file is Ragnarok's, and its `cmd` 0x2AB is KH2FM's Ragnarok under its PC name.
- **Their MP.** The costs are plain costs (not 255, "all remaining MP"). All of KH2's limits
  that cost MP (these four, and the party limits 0x21E / 0x220 at 50) have flag 0x10000 and no
  spell has it; what it does to a cost above Sora's MP isn't traced (no KH2 executable here). The
  flags also have 0x2, the Land flag (`reaction-commands.md`: a Land command can't be used in
  the air), on the limits and their follow-ups.
- **Its script** (`limi`, BAR entry type 3 of the model, as Wisdom's `wisd`; work 128 bytes).
  Read from its code:
  - **Trigger 22** starts a limit with a limit record. It asks `trap_limit_start_command` for
    the command, keeps the limit's control block in work memory, starts a thread that offers
    the party member's commands while `trap_limit_time` lasts (none in a solo limit), and runs
    the command's code: that sets the light (`trap_limit_light`, `trap_light_set_color`; other
    colours in world 13), the follow-up pair and count (Sonic Rave: Rave, Finish, 5; Strike
    Raid: Raid, Judgment, 3), and Sora's hook.
  - **The hook** (`trap_obj_hook`, one per object, a new one replacing the old) drives his
    motions: `trap_sysobj_motion_start` / `_push` (a queue; pushing 0 ends the chain in idle),
    waits on `trap_sysobj_motion_is_end` / `_id`, turns him (`trap_obj_wish_dir`: Rave and
    Finish turn him round to pass back through), and pushes him back (`trap_obj_hitback`,
    Infinity's shots). The script checks his object is Limit Form's (`trap_obj_get_entry_id`
    0x95D..0x961) before it moves him.
  - **The main loop** runs while his motion is the one the hook last started (or one of
    Infinity's 241..248). While the count is above 0 it offers the first follow-up during the
    motion's script range 1 (`trap_obj_motion_check_range (obj, 1)`), at 0 the second during
    range 2 (`trap_command_override`, every frame the range is open). When the motion is no
    longer the hook's, the limit ends (`trap_limit_reference` waited on, its targets freed,
    `trap_obj_stop_end_all (17)`); Strike Raid then catches the Keyblade.
  - **Trigger 23** runs a follow-up command chosen (its count down by one): Rave starts 212
    again, Finish 213; Bash 222..226 in turn (a counter in work memory), The End 227; Raid and
    Judgment throw the Keyblade (232 / 233, then the catch 234); Shoot, Impact and Giga Impact
    push him back 50 / 100 / 200 (`trap_obj_hitback`, away from his facing) and start 247 / 247
    / 248 with 10 / 17 / 25 shots.
  - **Trigger 17** with 100 (frame trigger 7 [100, 0] of 247 / 248) fires Infinity's shots (ATKP
    1862, radius 80, homing): below.
  - **Last Arcanum** also starts its own glow on the Keyblade for the whole limit (a thread at its
    start: elements 35 on the weapon root 0x4005 and 36 on the tip 0x4003, 6 and 28 along the
    blade's x), ended when his motion is no longer the hook's.
  - **Strike Raid** (`cmd` 704): the hook plays 231 (K420; it has no follow-up range), then the
    first throw (232, Raid's) by itself. The throw motion's frame trigger 9 (232 frame 8, 233
    frame 27) starts a thread that takes the Keyblade from his hand as an object of its own
    (`trap_party_get_weapon`, `trap_obj_detach`, `trap_obj_motion_unsync`), starts its own motion
    235 on it (its spin) and its trail (element 32 on its blade bone,
    `trap_obj_effect_start_bind_other`), and makes it an attack: **ATKP 1866 radius 160 for
    Raid, 1867 radius 190 for Judgment** (power 200 and 500; s291's notes had the two the other
    way round). Each frame it turns toward the lock-on target at most 5° (`trap_vector_homing`;
    no more once within 120), moves 30 along its heading (`trap_vector_addf`), is kept 120 (Raid)
    / 150 (Judgment) above a floor found within that much below, and its heading becomes 0.8 of
    itself and 0.2 of the step it made (so the homing is weak: the throw goes about where he
    faced). Judgment wobbles it up and down about that path (amplitude 30, a 20-frame period).
    Its turn is `trap_strike_raid_calc_xyzrot` (heading, up) then `trap_obj_set_xyzrot`, the up
    easing from straight up to the throw's own over 33 frames; it moves by
    `trap_obj_move_request`. It flies until it is attached again: 232 / 233 end into the catch
    (234, K423), which puts it back in his hand (`trap_obj_attach`, `trap_party_hand_to_bone`)
    with a flash where it was (element 34, `trap_obj_effect_start_pos`) and ends the trail.
    234's script ranges 1 and 2 (frames 0..38) offer Raid (count 3) then Judgment; the limit
    ends with 234.
  - **`trap_strike_raid_calc_xyzrot (dir, up)`** (exe `0x14053EF90`, read): both normalized
    (`up` in place); an `up` equal to `dir` becomes (0, -1, 0); x = normalize(up × dir), z =
    -dir, `up` is written back as dir × x (square to `dir`), y = normalize(z × x); the rows x, y,
    z are turned into Euler angles (`0x1401AB290`). The script's first call (its result dropped)
    is for the up it writes back, which it then makes point up.
  - **Infinity** (`cmd` 683) has no follow-up pair: the hook plays 241 (K430), then combo rows
    52..55 (242..245) by `trap_obj_party_attack`, each started again on its first frame with the
    next pushed after it, then row 56 (246), whose frame trigger 9 starts its charge (element 20
    on the tip). 246's range 20 offers Shoot (frames 42..72 and 92..100), Impact (72..82) and
    Giga Impact (82..92). The shots (trigger 17): from the Keyblade tip, one per `n` (10, 17,
    25), each sent along his "up" turned about his facing by 2π·i / n (`trap_vector_rot`: a ring
    round his forward axis); three of them, picked at random, go for the lock-on target, the
    others for an enemy the script's target search finds (up to 10 found within 1000, one picked
    at random). Each waits 6 frames, then each frame turns at most 5° toward a point a random
    0.2..0.8 of the way from its target to the side (`trap_vector_inter` with the cross of its
    heading: they spiral in), moves 16, and ends when it hits or after 150..180 frames (a target
    reached within 64 without a hit is dropped for another).
- **The motions' own parts.** Range 16 and 17 are the script ranges 1 and 2 (Sonic Rave's 212:
  frames 16..46; Strike Raid's 234: 0..38). Range 20 (command, 0) offers a command on the
  reaction button while open: Last Arcanum's Bash (221..225) and The End (226), Infinity's Shoot,
  Impact and Giga Impact (246). Range 26 names the limit the motion belongs to. The hits are the
  Keyblade's (`W_EX010_KH1F`): Rave ATKP 1878, Finish 1879, Last Arcanum 1883..1895, Infinity's
  combo 1857..1861, each with the guard row 162. The body's ranges 27 make him invulnerable
  (211 all through, 212 frames 0..12, 213 0..48, 221 0..120, ...). Frame trigger 1 starts the
  form's effect elements (211 [40], 212 [26, 27, 28], 213 [29, 30, 31], ...), 14 its voices,
  the Keyblade's 8 its sounds.
- **Ours (inferred from the script and the data together):** `trap_obj_motion_check_range (obj,
  n)` reads range 15 + n, and `trap_obj_motion_check_trigger (obj, n)` frame trigger 8 + n (the
  throw frames of 232 / 233 and Infinity's 246 carry trigger 9 and no trigger 1). A command the
  motions offer by range 20 goes, when chosen, to the limit's trigger 23 as the script's own
  offers do (they are all `cmd` execute 707). Frame trigger 7 of a limit motion is the script's
  trigger 17 with the object and the trigger's parameters. Not checked against the executable.
- **Read from the executable** (KH2 FM PC, `KINGDOM HEARTS II FINAL MIX.exe`; the trap tables are
  16-byte records of handler and flags, table 0 at RVA `0x752E00`, 1 `0x755370`, 2 `0x756B60`,
  6 `0x787E80`, 8 `0x787530`): `trap_vector_rot (v, axis, angle)` turns `v` about `axis`
  (`0x14041D650`); `trap_vector_inter (a, b, t)` is a + (b − a)·t (`0x14041D490`);
  `trap_vector_homing (v, to, max)` writes `v` turned toward `to` by at most `max` (no
  frametime), a unit vector (`0x14041D3D0`); `trap_random_get (n)` / `_getf (x)` / `_range (a,
  b)` are an integer below n, x · [0, 1), a + (b − a) · [0, 1); `trap_obj_hitback` sets the
  object's push (dist × 0.1 a frame, × 0.9 each frame, `0x1403D4960`, as `character_core`'s
  hit-back).
- **Not read:** ranges 23 (parameters 1, 3, 78, 92), 29, 31, 44 and 49, and frame triggers 16,
  19 and 31 of these motions. Nothing in the script moves the camera.
- **Square.** Standing, it is the guard (row 11, motion 173; KH2's Limit Form keeps the guard).
  Moving, it is Dodge Roll (motion 209). Attack during the guard's repel (motion 42) is the
  Zantetsu Counter (row 39). All three are the controller's own rules (`character_core`), which
  apply to this data as they are.
- **Triangle** is the limits' follow-ups (above) and Auto Limit's reaction command.

### The port (ours, on the rules every form has here)

- `form::Kind::Limit`, `pose::Kh2::limit` (`P_EX100_KH1F` through `load_form`), `drive::FORMS`
  third entry: record 0x81, word 0x1FB. The Drive records' arguments follow the list's order, so
  Final's is now 0x13.
- The change, the revert, the form gauge and the revert when it runs out (s318; before, the MP
  drain), on a KO and in a new world, and full HP on entering are all as for every form.
- **Moveset.** KH2's controller runs its moves and combo rows, its movement numbers, its guard,
  Aerial Recovery (motion 40), the Zantetsu Counter and its finishers, as above.
- **Growth.** Dodge Roll at base Sora's level + 1, MAX at most (`form::growth_up`), the others
  none. It follows KH1's menu changes mid-form (`form::abilities`).
- **Effects and sounds.** Its archive is `kh2fx::LIMIT` (frame trigger 1 → `kh2fx::form_event`,
  weapon anchors on its one Keyblade, `form_bone`). Its elements' sounds are in
  `formmagic::element_sounds`, and its swings' sounds and voices in `kh2sound`, as Valor's.
- **Look.** No tint (`tint::refresh(None)`): Sora's drawn model is KH1's, which Limit Form is
  modelled on. With KH2's player model (off since s259) its own body `P_EX100_KH1F.mdlx` is
  drawn (`model.rs`).
- **No magic.** KH1's spells (a shortcut's too) are refused in the form (`combat::run_entry`):
  KH2's Limit Form can't cast. With KH2's Limit menu off (below), Magic's row is the Limit menu
  of KH1's limits instead ("KH1's Limit menu in Limit Form" below).
- **For now KH1's limits, not KH2's** (the user: "lets comment the KH2 limits, and for the
  moment put in the equivalent KH1 limits"). KH2's Limit menu below is switched off by
  `limits::KH2_LIMITS` (false). Its code is kept and still compiled, only never installed, so
  `true` brings it back as it was (and takes KH1's limits out of Limit Form again). In its place,
  KH1's own limits, in a Limit menu of the same kind (the user: "Limit form should have all the
  KH1 limits without having to have the abilities equipped on base sora ... in this form, the
  limits should be access via the limit menu (similar to KH2's limit menu) and they should NOT
  be activated, or even prompted via the triangle prompts"). Four stand in for KH2's (the
  user's pairs): Sonic Blade for Sonic Rave, Ars Arcanum for Last Arcanum, Strike Raid for Strike
  Raid, Ragnarok for Infinity; Trinity Limit is there too. See "KH1's limits in Limit Form"
  below.
- **The Limit menu (the user's pick: "KH2's own limits"; off for now, above).** `limits.rs`:
  - The top's Magic row reads "Limit" (record 2's word answered by `limits::word`) and opens our
    list in the magic list's place: records 0x82..0x85 (words 0x1FA..0x1F7, spell-type records
    with arguments 0x20..0x23, written over empty ones as the Drive forms are), named Sonic
    Rave, Last Arcanum, Strike Raid, Infinity. KH1's magic list has no cost column, so no cost
    is shown. A row is greyed in the air (the Land flag), with the MP short, or while a limit
    runs (Infinity's shots still flying count).
  - **MP (ours).** Each limit costs one of KH1's bars (KH1's payment `0x1402A39D0` with 100). The
    mod's KH2 spells (Wisdom's, Final's, base Sora's, Reflect) cost KH1's own 1 MP each whatever
    KH2's cost (`reflect::COST`, `kh1_spell_cost`), and KH2's cost / 100 rounded up gives the same
    bar for 60..80, so both rules agree; it is logged when a limit is paid. Short of a bar the
    limit can't be used (KH2's own rule for a cost above his MP isn't traced, above). The form's
    gauge drains on (s318: its form gauge, not MP); a form gauge running out mid-limit reverts
    only after the limit (that waits for `can_change_form`, never true in a scripted motion).
  - **Running.** Chosen, a limit waits in the list until KH2's move takes a command
    (`can_take_command`, as a spell waits for `can_cast`). Then its bar is paid and `limi` is
    started at trigger 22 with the command (`kh2_magic::Magic::limit`). Each frame after KH2's
    step the script runs (`limits::after_step`, Sora as `LimitWorld`); what it asks is done
    before the next step (`limits::before_step`): `Character::play` for a start, the controller's
    one-slot queue (`push_scripted`) fed from ours for the pushes, `set_facing` for a turn. A
    motion's hook runs before the script's main loop each frame (`kh2_magic`: an object's hook
    runs in its own update, before the scripts that run after it; the other order ended Sonic
    Rave as 212 began).
  - **Triangle.** A follow-up the script offers, or the one the motion's range 20 offers, is put
    in KH1's own Triangle box: the top menu's fourth entry, record 0x86 (word 0x1F6, its name the
    follow-up's), written after KH1's context fill (the fill's tail jump `0x140292320` pointed
    at `limits::context_fill`). Its record has KH1's "run while held" flag (0x10), the only kind
    KH1's step runs from that box; we take only a fresh press (KH1's logical Triangle
    `0x4000000` pressed that frame), once per window: after a follow-up, none is offered until
    its motion starts.
  - **All four run**, with all their follow-ups (`kh2-magic`'s `tests/limits.rs` runs them from
    the real script against the form's moveset: Rave five times, then Finish; Bash five times,
    then The End; Strike Raid's first throw, three Raids and Judgment, the Keyblade off and back
    each time; Infinity's rows 52..56, Shoot, its push and its ten shots).
  - **The script's objects (`kh2-magic`).** Sora's Keyblade is an object of its own to the script
    (`World::weapon`). Detached, `Magic` keeps it (`Loose`: where, how turned, its motion and
    frame) and answers for it itself, so the script moves it (`trap_obj_set_pos`,
    `_move_request`, `_set_xyzrot`) until it is attached again; the host only draws it. A limit
    that ends early (a revert) puts it back. The target traps are ours on the host's enemies
    (`World::foes`): the lock-on target is the lock's (nothing locked: the nearest enemy in
    front, as Wisdom's shots); a search takes the nearest enemy in its range it hasn't found yet
    (KH2's own pick isn't traced).
  - **The thrown Keyblade drawn.** KH1's own Keyblade (its weapon bones, the grip and its chain)
    is moved where the script has it (`limits::pose_hook`, `leftblade::float_grip`): KH2's blade
    bone of the Keyblade's motion 235 at its frame, on the object, put to KH1's grip by the same
    relation that holds it in the hand (`leftblade::held_blade`), as Final Form's floating
    Keyblades are. His hand is empty meanwhile.
  - **Its hits.** The script's attacks (the thrown Keyblade's sphere on its path, Infinity's
    shots) hit KH1's enemies through the spell hit records (`Strikes::spell_strike`, keys
    `1 << 58`), with KH2's ATKP rows; a hit is passed back (a shot ends there).
  - **The push.** `trap_obj_hitback` on Sora is `Character::hit_back` (KH2's: dist × 0.1 a frame,
    decaying × 0.9).
  - **A combo row** (`trap_obj_party_attack`) plays the row's motion as the script's own start;
    its hits and windows are the motion's. A start of the motion that has just begun, on its
    first frame (the script taking over a motion its push started: Sonic Rave's 212, Infinity's
    rows), is not started again (it played its first frame twice, its voice too).
  - **Weapon anchors.** Limit Form's are KH2's own, as Final's: the blade bone where KH1 draws the
    Keyblade (in his hand, or thrown; `limits::drawn_blade`), its turn included, the tip 78 along
    its −x (`kh2fx::form_bone`). They had Valor's rule (the point, Sora's facing for the turn),
    and the script's elements on them started on his hand's tip: Last Arcanum's glow on the
    Keyblade (elements 35 / 36, 6 and 28 along the blade) pointed along his facing, off the
    Keyblade (the user, s291: "the last arcanum vfx being desyncd from the keyblade"). An element
    a spell script starts now starts on its package's anchors (`kh2fx::spell_start`).
- **No KH1 limits in any form but Limit Form, and none on Triangle** (the user: "KH1 limits that
  are equipped shouldn't even prompt in any drive form. No drive form should be able to use KH1
  Limits"; then, for Limit Form only, "for the moment put in the equivalent KH1 limits", and "all
  the KH1 limits ... via the limit menu"). Read from the executable:
  - **The offer.** KH1 offers its limits in its Triangle box: `0x14028AF60`, called once, from
    the end of the context fill (`0x140292B12`). It does nothing while a limit runs (battle flags
    `0x142867CD8` bit 16), with Sora not in KH1's ground state, with the limit data not loaded
    (`0x14299CC20` not 5) or without a hittable target (`0x142DB3F18`). Its table at
    `0x140529040` holds five pairs of ability and command: Sonic Blade (ability 11, command 0x4B),
    Ars Arcanum (12, 0x57), Strike Raid (13, 0x5E), Ragnarok (14, 0x5A), Trinity Limit (15, 0x52).
    It goes through them in the order at `0x14052904C` (Trinity, Ragnarok, Ars Arcanum, Sonic
    Blade, Strike Raid), taking only those whose bit is set at `0x14299CC40` (bit k for entry k,
    set from KH1's has-ability test `0x140282A70` at `0x14028B310`: the equipped ones). Each one
    is scored against the target's distance (the command record's `+0xE`; Sonic Blade and
    Ragnarok also by its height, Trinity by the party) and its MP (`0x142D23730 + 0xF4 + 2k`,
    through `0x1402A2A80`). The best goes in the box (`0x140290E70 (3, command)`), greyed
    (`0x140290E20`) when the MP is short.
  - **The run.** The command is its own command type. KH1's command step sends it from
    `0x1402A1470` to `0x14028B240`, the limit start: `0x14028B670 (sora, k)` sets bit 16, starts
    limit k through `0x140397C00 (k, its data)` and pays its MP (`0x1402A39D0`). The follow-ups go
    the same way to the running limit (`0x140397C00 (7 | 8 | 9 | 11, 0)`): Sonic Blade 0x4D / 0x4E,
    Ars Arcanum 0x58 / 0x59, Strike Raid 0x5F / 0x60, Ragnarok 0x5B. `0x140397C00` sends each
    limit to its own code (`0x14039EC80`, `0x14039F060`, `0x1403A0180`, `0x14039F400`,
    `0x1403A0650`; not read further). `0x14028B490`, a task run every frame, clears bit 16 two
    frames after the limit stops (`0x140397C00 (5)` reading 0). (That the command type is the
    command id, as for the context commands, is inferred: the type dispatch and the offer's table
    name the same five ids. The log names the command when one is chosen in a form.)
  - **Ours, every form but Limit Form.** The offer call is skipped (`limits::specials_prompt`), so
    they never show. The command step refuses them and their follow-ups with KH1's "can't"
    (`combat::command`, `limits::kh1_limit`). They don't move Sora through the command step, so
    `NOT_SORAS` lists them, and the s278 refusal of every KH1 move but Attack, magic and items
    never caught them: before this, the offer's skip was the only thing keeping them out.
  - **Ours, Limit Form** (with `KH2_LIMITS` off): the offer call is skipped there too (s297 ran
    it with the four stand-ins left in; the user, s298: "they should NOT be activated, or even
    prompted via the triangle prompts"). The limits come from the form's Limit menu and
    shortcuts instead (below). The follow-ups still come on Triangle: the running limit's own code
    puts them in the Triangle box (its calls of `0x140290E70 (3, command)` in KH1's limit modules,
    `0x140396264` and the like), not the offer, which does nothing while a limit runs.
- **KH1's Limit menu in Limit Form (ours, on KH1's rules; `limits::fill_kh1`).**
  - **How a limit starts without its ability (read from the executable).** The equipped test is
    only in the Triangle offer: `0x14028B310` sets the offer's bits at `0x14299CC40` from KH1's
    has-ability test `0x140282A70 (ability 11..15)`, and the offer looks at nothing else of the
    abilities. The command step's dispatch (`0x1402A1470`, command type 0x4B..0x60 to
    `0x14028B240`) and the limit start `0x14028B670 (sora, k)` (bit 16, the limit started through
    `0x140397C00 (k, its data)`, MP paid) check no ability, and the limits' data is one file for
    all five (`xl_limit.dat`, loaded by the frame task `0x14028B490` whatever is equipped; its
    state `0x14299CC20` is 5 once loaded). So nothing needs to pass a check: a command of the
    limit's type reaching the command step runs it.
  - **The list.** In Limit Form the top's Magic row reads "Limit" (record 2's word, answered by
    `limits::word`) and opens KH1's five limits in the magic list's place, in KH1's order: Sonic
    Blade, Ars Arcanum, Strike Raid, Ragnarok, Trinity Limit (KH1's Summon row, page 2's Drive,
    stays last). Equipped, learned or not. Their records are ours (0x82..0x86, spell-type records
    with arguments 0x20..0x24, written over empty ones as the Drive forms are), each with the word
    of KH1's own record for its limit (`record (command) + 4`), so the names are KH1's, in KH1's
    text. KH1's magic list shows no cost.
  - **Greyed** while KH1 couldn't start it there (`limits::kh1_usable`), its own conditions for
    offering one (`0x14028AF60`): no limit running (bit 16), limits allowed (`0x142D54438` bit 1
    clear), the limit data loaded, Sora in KH1's ground state (`+0x70` 0) and not in KH2's air, a
    target (`0x142DB3F18` hittable, `+0x374 & 0x60` = 0x40), for Trinity Limit both others there
    and able (`+0x88 & 0x30` clear, hittable, not `+0x374 & 0x8000`, not `+0 & 0x200`, within 500
    of Sora), and its MP (KH1's test `0x1402A2A80` with KH1's cost, `*0x142D23730 + 0xF4 + 2k`).
    **Ours:** not the offer's distance (the record's `+0xE`) and height rules (Sonic Blade only
    with the target at most 160 above, Ragnarok only at least 160): they choose the one limit
    the box shows, and in a menu they would always grey one of those two. Where KH1 greys Magic
    itself (save `+0x874` bit 0) the Limit row stays greyed.
  - **Chosen** (or from a shortcut): KH1's menu passes it on once KH2's move can take a command
    (else the list stays open, as for a spell). `combat::command` turns it into KH1's own limit
    command (type the limit's command id, argument 0, the step's flags kept) and it goes on as
    one of KH1's limits in Limit Form does (below), at KH1's MP cost.
  - **Shortcuts: KH2 FM's Limit Form layout** (read from the KH2 FM PC executable): in Limit
    Form KH2's shortcut fill (`0x1403F2780`, the same code in the command menu build at
    `0x1403F2210`) puts entry k of its table `0x1405C97E8` (Sonic Rave, Last Arcanum, Strike Raid,
    Infinity: `cmd` 0x2BA, 0x2BD, 0x2C0, 0x2AB) in shortcut slot k (k = 0..3), unless the slot
    holds an item; its Limit submenu lists the same four in the same order (`0x14074E978`, from
    the menu build at `0x1403F20FA`). **Ours:** slot k is the mod's shortcut k (L1 + Triangle,
    Square, confirm, cancel: KH1's three and the mod's fourth), given the KH1 limit standing in
    for KH2's k-th: Sonic Blade, Ars Arcanum, Strike Raid, Ragnarok (`limits::SHORTCUT_LIMITS`, a
    four-entry table). KH1's shortcuts never hold items, so every slot is a limit. With three
    shortcuts (`shortcuts = 3`), Ragnarok has none. Each is greyed as its menu row, drawn with
    KH1's name in KH1's shortcut menu, and the shortcut flag (`0x142D5445C`) is set so L1 opens
    it. KH1 fills both menus again every frame, so Sora's own spells and shortcuts (Reflect, the
    fourth) are back the frame the form ends.
  - **Not in it: Zantetsuken.** KH1's Zantetsuken (ability 59) is a combo finisher, not a limit:
    its ability sets bit 29 of KH1's combo-ability flags `0x142D5F610` (`0x1402A4707`, with Stun
    Impact's bit 27 and Gravity Break's 28), which KH1's attack-row pick reads (`0x1402A4E3E`).
    It has no command, no place in the limit table or file, so there is nothing to start; in the
    form Attack is KH2's anyway, whose own Zantetsuken is the default ground finisher. The mod
    still takes KH1's out of Sora's ability list (`menu.rs`, s153).
- **KH1's limits in Limit Form: how they run (ours).**
  - **The start** (`combat::command`) is taken only as KH1 could start it (`limits::kh1_usable`,
    what greys its menu row: on the ground, a target, Trinity's party, the MP), and only when
    KH2's move can take a command (`can_take_command`, no queue, as every command here).
    Otherwise it is refused with KH1's "can't". KH1's command step then runs it as KH1 does: its
    limit, its MP cost. If its limit flag (bit 16) is set after that, KH1 takes Sora as for a KH1
    command (`hand_to_kh1`, `FROM_COMMAND`, no spell, no combo slot).
  - **While it runs** (`takeover`, `limits::kh1_limit_holds`: Limit Form and bit 16 set), KH1 has
    him whatever is pressed. Its limit module moves and animates him, and its Keyblade (Strike
    Raid's throw too), effects, hits and MP are KH1's. None of the hand-off's cuts (Attack, jump,
    Square, the stick) are taken. An Attack pressed meanwhile is dropped (no buffer). The
    follow-ups are KH1's own Triangle presses, passed straight to KH1. `pose::follow` draws KH1's
    own animation all through: Limit Form's model is KH1 Sora's, so none of the form's KH2
    motions are drawn over a walk, run or jump slot of it.
  - **The form** stays on. Its gauge drains on (s318: the form gauge, `gauge::frame`, every frame
    of Sora's). Run out meanwhile, the revert is set (`form::drain`) but waits: it needs KH2 to have him
    and `can_change_form`, so it comes on the first frame after the limit. Before this, every
    KH1 move in a form paused the drain, because the drain ran only in KH2's steps. That is still
    so for items and Wisdom / Final's KH1 spells.
  - **Never reverted mid-limit (fixed after s313).** The user: "Limit from has a bug where sora will be
    reverted to base sora in the middle of a KH1 Limit". The log (s313, every Strike Raid in Limit
    Form): KH1's limit sets Sora's script-hold bit `0x1000000` for the whole limit (flags
    `0130c087` / `01308087`, no script actions at `+0x440`). `takeover` read that as KH1's area
    script holding him before it reached the limit's branch, so `scene_frame` counted the limit
    as a scene in battle and, 2.5 s in, `form::scene_revert` reverted the form ("Revert (KH1 has
    had Sora 2.5 s for a scene out of a battle (its script holds him))"), right after Strike
    Raid's follow-up. It also meant the limit's branch (the drain, the hand-off's frames) never
    ran for a limit that sets the bit. Now:
    - while `limits::kh1_limit_holds`, the script-hold test is skipped and the limit's branch has
      him (`SCRIPTED` is still stored as KH1 set it, so the camera and `pose` are unchanged);
    - `form::limit_under_way` (a KH1 limit in Limit Form, or KH2's own Limit commands if
      `KH2_LIMITS` is turned back on) puts off every revert but a knock-out or a new world: a
      scene's (`scene_revert`), the form gauge running out (`before_step`, as KH2's gauge running out
      mid-limit ends the form only after it), and the Revert command (kept for the limit's end). The log
      says "Drive: the revert (...) waits: a limit is under way in Limit Form" once, and "Drive:
      the revert put off during a limit (...) taken now" when it comes; a scene that ended with
      the limit leaves the form on ("... no longer asks: the form stays").
    - Other forms: unchanged. No log showed the scene revert firing on magic or summons in them
      (the one other scene revert in the logs, s310 in Valor Form, is the stale-hold case after a
      battle scene, `stale_hold`).
  - **Its end.** Once bit 16 clears, the hand-off decides as at a KH1 command's end. The
    hand-off's frames count on through the limit, so the end is taken as soon as KH1 lets him go.
    On the ground, KH2 takes him on KH1's idle, the stick, an Attack, a jump or Square. In the
    air (KH1's jump or fall state), it is KH2's fall at KH1's speed. KH2's controller starts over
    in the form (`form::character`) and comes back in its battle stance.
  - **Unchanged out of a form.** Base Sora's KH1 limits still run as before: no hand-off, KH1
    keeping him while its slot is one of its own actions.
- **Unchecked in game:** all of it. In particular:
  - whether the KH1F archive loads as a PAX;
  - the Limit menu's rows and names, and the Triangle box with our record (KH1's step runs it
    only at the menu's top level);
  - the follow-ups' timing against KH2's (the press reaches the script one or two frames later);
  - the limits' hits (KH2's ATKP powers on KH1's enemies) and their invulnerability;
  - KH1's limits gone from the Triangle box in every form, Limit Form too, and their follow-ups
    still there in Limit Form;
  - KH1's Limit menu: the "Limit" row, KH1's five names, the greying (in particular that KH1's
    ground state `+0x70` reads 0 while KH2 has him, else every row stays greyed), a limit started
    without its ability equipped or learned, at KH1's cost; Trinity Limit with the party; that a
    limit chosen from our magic-type record behaves as from the Triangle box (that box's command
    may carry KH1's turn toward the target, flag `0x2000000` read at `0x1402A149A`, which ours
    lacks);
  - the shortcut layout's buttons against KH2's own (which face button KH2 draws for its slots 2
    and 3 isn't traced), the names in KH1's shortcut menu, and Sora's shortcuts back after the
    form;
  - KH1's limits in Limit Form: that KH1 has him all through each (Sonic Blade's dashes, Ars
    Arcanum's flurry, Strike Raid's throws, Ragnarok's rise and shots) and draws its own
    animation, Keyblade and effects there, its hits land, its follow-ups come on Triangle, and KH2
    takes him back cleanly after, on the ground and in the air; that bit 16 covers the whole
    limit (if KH1 drops it early, the hand-off's cuts could take him mid-limit); that the takeover
    runs on KH1's limit frames (the drain runs from there);
  - the form gauge and its revert through a KH1 limit; that no limit (Strike Raid's
    throws and follow-ups in particular) reverts the form part way, and a revert put off comes
    on the first frame after it;
  - Ripple Drive's body group 20 shapes on KH1's contact code;
  - Zantetsuken's power 600 against KH1's enemies;
  - Sora's `plyr` stand-in.

## Master Form in the KH1 port (Data; the rules ours where marked)

The user: "lets take the rules and procedures we've done for wisdom form and valor form, and lets
make master form. I want sora's jumpsuit to be yellow, and the inside of sora's jacket's hood be
yellow as well instead of blue. I want soras belt to be black. Lets make sure when sora uses his
KH2 Master Form attacks, his secondary keyblade's trail actually follows the keyblade." Its unlock
is the user's, open once Anti-Sora is beaten ("When the forms open" below); its cost is KH2's 4
Drive bars since s318 (2 MP before).

Read from fresh copies of the Steam install's `00battle.bin`, `03system.bin`, `00objentry.bin`,
`obj/P_EX100_TRIF.mdlx` / `.mset` and `obj/W_EX010_TRIF_R` / `_L.mset` (research helpers
`kh2-data/examples/form_info.rs` and `mset_slots.rs`, `kh2-anim/examples/final_blades.rs`).
Nothing of it is in the repository.

### Its data

- **Object.** `P_EX100_TRIF`, moveset 4, form id 4 (`kh2_data::sora::FORMS[3]`). The same
  256-bone skeleton as Sora's.
- **Weapons.** `wmst` rows 4 × 2 and 4 × 2 + 1: `W_EX010_TRIF_R` on the right hand bone (178,
  51 motions) and `W_EX010_TRIF_L` on the left (86, 48 motions), both with the Keyblade's 17
  shapes. KH2's blade bone (2) of each, against its hand (`final_blades`):
  - the right one is always in his hand, 6..9 units from it, in every motion checked (idle, run,
    jump, fall, the casts, 181..185, Aerial Dodge);
  - the left one floats ("held by magic"): 29..50 from the left hand standing, 30..111 in the
    attacks, 15..99 in the casts, 123 in motion 8; it is held (8 units) in the jump (3), the fall
    (4), the run (10), the air recovery (40) and Aerial Dodge (202). From the right hand it is
    15..201 away.
- **Movement (`prty` 4).** Walk 2, run 10, jump 285, turn 0.26, no acceleration / brake. `plyr`
  4: auto lock-on 500, lock-on 1200.
- **Motions.** 0..19, 40..43, 53, 56..73 (the casts), 139, 151..154, 181..185, 202. The attacks
  are 181..185 (tags D330, D331, D333, D334, D332); 151..154 are the same clips as the casts (no
  combo row names them). Motion 53 is the arrival (elements 1..12 and 100..102). Motion 202 (A160,
  a guard window ATKP 406) and 8 (A101, the same) are Aerial Dodge's.
- **No guard (Data).** Motions 41, 42 and 43 are one clip, D350 (the repel), with no guard window
  in the body's or either Keyblade's set. Sora's 42 (A321) has one (ATKP 162 on the Keyblade).
  There is no 173 (the guard's press) and no row 11. **Ours:** `form::form_abilities` now gives
  a form the guard only if its motion 42 has a guard window. That changes nothing for the others
  (Valor's and Limit's 42 have one, Wisdom and Final have no 42; Anti never guards).
- **Combo rows (PTYA set 4, 16 rows).** None needs an ability. Flags: 0x1 air, 0x4 finisher.

  | Row | Kind | Flags | Motion | Its hits (ATKP, right / left Keyblade) |
  |---|---|---|---|---|
  | 5 | 3 (ground default) | | 185 | 1077 / 1076 |
  | 4 | 1 (0..1000) | | 181 | 1060 / 1059, 1366 |
  | 6, 9, 22 | 1 | 0x10, 0x11 | 185, 181 | |
  | 0, 10, 24 | 1 (0..300) | 0x8, 0x9, 0x1 | 182 | 1062 / 1061, 1063 |
  | 8 | 3 (ground finisher default) | 0x5 | 183 | 1252, 1254, 1255, 1256 / 1257, 1368..1370, 1356 |
  | 7, 58 | 2, 1 (0..300) | 0x5 | 184, 183 | 184: the body's group 3, 1100 frames 20..210, then 1101 |
  | 21 | 3 (air default) | 0x1 | 181 | |
  | 20, 23 | 1 | 0x11, 0x1 | 185 | |
  | 25 | 1 (0..300) | 0x15 (air finisher) | 184 | |
  | 1 | 0 | | 181 | |

  184 (D334, 254 frames) is the long spinning finisher: its hits are the body's (group 3), with an
  allow-movement range 0..200.
- **Casts (Data).** MAGC gives the same motion ids in every form, and Master's moveset fills them
  with one clip each for the ground, the finisher and the air: Fire 56 / 57 / 58 (D371), Blizzard
  59..61 (D370), Thunder 62..64 (D372), Cure 65 / 67 (D374, also 68 and 70), Reflect 71..73
  (D373). Each casts with frame trigger 17 (at 14, 32, 8, 20 and 0) and swings the Keyblades
  meanwhile: Fire 1097 / 1096, Blizzard 1066 / 1065, Thunder 1098 (left), Cure 1099 (left),
  Reflect 1078 (left). A no-gravity range covers most of each. Master's magic sub is + 20
  (above).
- **Their travel (Data).** Unlike Final's and Wisdom's KH1-spell motions, Master's casts move him
  by their root curves. Fire goes about 800 forward, from frame 16, mostly by frame 28. Blizzard
  goes 390 out by frame 24, then back to 107. Thunder goes about 1050 (720 right, 770 forward)
  over 88 frames. Reflect goes 92. Cure goes nowhere. None has an allow-movement or
  keep-momentum range, so KH2's controller drops his speed going in (`Character::cast`) and the
  root travel is his movement. No-gravity ranges: Fire 0..68, Blizzard 0..60, Thunder 0..80,
  Cure 0..46, Reflect 0..54.
- **Effects and sounds.** Its motions start elements 20..32 of its own archive
  (`obj/P_EX100_TRIF.a.us`; whether the install has it and whether it holds a "p_ex" PAX isn't
  checked here). The Keyblades' sounds are bank 500's (500 040..055), with the voices (frame
  trigger 14).
- **Growth.** Aerial Dodge (the forms' table above).

### The port (ours, on the rules every form has here)

- `form::Kind::Master`, `pose::Kh2::master` (`load_form`), `drive::FORMS`' fourth entry, KH2's
  order (Valor, Wisdom, Limit, Master, Final): record 0x87 and word 0x1F5, the next free ones after
  Limit Form's limits (0x82..0x86, 0x1FA..0x1F6). The records' arguments follow the list, so
  Final's is 0x14 now. It is listed only once unlocked.
- The change, the revert, the form gauge and the revert when it runs out (s318), on a KO and in a
  new world, full HP on entering, no KH1 limits (`combat`, through
  `form::two_blades`), and talking and examining are all as for every form.
- **Growth.** Aerial Dodge at base Sora's level + 1, MAX at most (`form::growth_up`), following
  KH1's menu changes mid-form (`form::abilities`).
- **Magic.** As Wisdom's and Final's (`form::casting_form`, `formmagic`): Fire, Blizzard and
  Thunder are KH2's at sub level × 2 − 2 (+ 1 for the finisher) + 20 on Master's own casts; Cure
  is KH1's on Master's Cure motion; Gravity, Stop and Aero are KH1's on its Thunder / Cure motions
  (`pose::follow`); Reflect at sub + 20. In the air its casts are KH2's own, as Final's
  (`takeover`'s air-magic block): the clip is the same as on the ground.
- **The KH1 spells' travel.** The user: "When doing KH1 magic, Master form does the correct
  animation, but it doesn't actually move sora at all. same goes for aerial magic". KH2's casts
  (Fire, Blizzard, Thunder, Reflect) already moved him by root motion through the controller (the
  s300 log: Fire about 770, Thunder about 1040). KH1's spells on Master's motions run on
  `takeover::form_cast_frame`, which had only the stick in an allow-movement range and the speed
  going in. Master's motions have no such range, so on the ground he stood still, and in the air
  he kept only the speed going in. Now (`FormCast::travel`, `takeover::travels`):
  - on a motion that travels, KH1's mover takes him along its root curve each frame, turned to
    his facing, on the ground and in the air;
  - his speed going in is dropped, as `Character::cast` drops it for a motion with no
    allow-movement or keep-momentum range. So there is no slide out of it or after its landing
    (Master's ground inertia is none anyway);
  - in the air he is held at his height through the motion's no-gravity range. KH1's fall speed is
    cleared there, and a frame's sink is put back. After it, KH1's fall starts from rest, as KH2's
    fall follows the range;
  - the pose has the travel taken out (`pose::follow_play`'s `cast`, as a swing's lunge), so the
    body stays on the moving actor instead of running ahead and snapping back.

  Only Gravity, on Thunder's motion 62 / 64, travels today. Stop, Aero and Cure are on Cure's
  motion 65 / 67, which doesn't travel: they keep the old rules (the speed going in kept in the
  air, KH1's height). Wisdom's, Final's and base Sora's KH1-spell motions have no root curve, so
  nothing changes for them.
- **Effects and sounds.** Its archive is `kh2fx::MASTER` (frame trigger 1 → `kh2fx::form_event`).
  Its weapon anchors are KH2's, as Final's: each Keyblade's blade bone as drawn
  (`leftblade::final_blade`, filled for Master by `note_pose`: the right one where KH1's held
  grip is, the left one where it floats). Its elements' sounds are in
  `formmagic::element_sounds`, and its swings' sounds and voices in `kh2sound`.
- **Second Keyblade.** Its own slot and "M:" row (`docs/kh1-equipment.md`), Final's rules (the
  Kingdom Key when it holds none). Its Strength and MP count in the form (`valorstats`).
- **Where the second Keyblade is drawn** (`leftblade::note_pose`). It is matched by KH2's blade
  bone (2), as Final's. The right one is KH1's own grip (held: no float, unlike Final's
  `final_float`). The left one:
  - floating, it hangs from KH1's right hand by KH2's right hand bone, as Final's does (s282: a
    blade far from the left hand swung with that hand's error);
  - held (KH2's blade within `MASTER_HELD` 12 units of its left hand), it hangs from KH1's left
    hand, so it stays in his grip in the jump, the fall, the run and Aerial Dodge;
  - in between (12..22 units), the two are blended (place, turn and scale, smoothstepped), and not
    blended across a mirrored hand.
  - Its reach check is Final's (400 from the hand).
- **Look.** `tint::Look::Master` (`docs/kh1-tint.md`).
- **Square.** On the ground it does nothing: Master has no guard (above), no row 11 and no Dodge
  Roll. In the air it is Aerial Dodge. What KH2's Master does with Square on the ground isn't
  traced.

### The second Keyblade's trail (Valor, Master, Final)

The user: "Lets make sure when sora uses his KH2 Master Form attacks, his secondary keyblade's
trail actually follows the keyblade". KH1's trail point (read again: `0x14029C990` → bone `0x30`
through the model's id table → `0x1401D41C0`) is the parent's world matrix applied to the bone's
sampled local translation (`instance + 0x1B0`, KH1's own channels), lerped from the parent's
position by `t` in the weapon's blade range (Sora's: 0.00..-5.20 over 110 units in the s261
log, so the bone sits about 21 units from its parent; inferred: 283, the chain's first bone,
under the grip 280, which is 21 units from it in the same log). So it follows the world matrices
its owner has when KH1's trail follower runs. Two things kept the left ribbon off its
Keyblade:

1. **The wrong windows.** The left trail opened and closed on Valor's left weapon's windows
   (`W_EX010_BTLF_L`) whatever the form: in Final, its 154 trails 11..124 of 280 frames, Valor's
   37..95; its 183 has four windows over 360 frames, Valor's one at 19..29. Master's
   (`W_EX010_TRIF_L`: 181 0..end, 182 1..37, 183 4..83 and 102..120, 185 0..24, the casts) would
   have been Valor's too. Now each form's own left weapon set is read (`wmst` row
   `moveset × 2 + 1`: Valor 2, Master 4, Final 5) and the current form's is used.
2. **A frame behind.** Its owner (a copy of Sora's actor whose matrices are his own moved by the
   right-grip-to-left-grip turn) was filled only at the left Keyblade's draw. KH1's own right
   trail reads Sora's matrices as the pose pass leaves them. When the follower ran between the
   pose and the draw, the left ribbon was the previous frame's: a fast floating blade (Master's,
   Final's) trailed it. Now the owner is filled again right after the left Keyblade is posed
   (`note_pose`, just after Sora's matrices are written), so both trails read the same frame. It
   is still filled at the draw too.

Also: the left trail is made only in a move (not standing, walking or running). Master's left
set has a trail window all through its idle, walk and run (0..2, as KH2 draws its float), and
KH1's swing ribbon would hang on him all the time there. Final's jump, fall and glide windows now
open its left trail, as KH2's do (they never opened before: Valor's set has no windows there).

### Unchecked in game

All of it, in particular:
- the Drive list's fourth row (record 0x87, word 0x1F5) and Final's new argument 0x14;
- Master's moves and combos through KH2's controller; the 184 finisher's body hits (group 3);
- its casts on KH1's cast timing, its KH1 spells on Master's motions, the air casts' physics;
- Gravity's travel on Thunder's motion (its direction against the drawn spin, walls, ledges), the
  air hold through 0..80 and the fall after it, and the pose staying on him;
- its effects (whether the TRIF archive loads), its sounds;
- the left Keyblade's float (its blend between held and floating, the reach check) and its effects;
- the left trail: in Master's attacks, following the floating blade; in Final, now on its own
  windows (and in its jump and glide); in Valor, unchanged but filled earlier;
- Aerial Dodge as Master's growth ability; no guard (Square on the ground does nothing);
- the "M:" row and the copy counting with all three rows.

## Forms across KH1's scenes (s307)

The user: "after defeating opposite armor in wisdom form, when i get control of sora again, i get
stuck in wisdom form with KH1 animations".

**KH1's script hold bit (Checked, exe + Log).**
- Sora's actor flag `0x1000000` is set and cleared only by KH1's actor script commands 0x24 and 0x23
  (`0x14028C550`: `0x14028CB83` sets, `0x14028CB78` clears). The other writers of that bit
  (`0x140287B73` and nearby) are Donald's and Goofy's. Its can-act test (`0x1402956E0`: action flag 4, `+0x88` bit `0x20`,
  `+0x444` bit 1) doesn't read it. No read of it was found in its player control step either.
- While it is set, KH1 takes no HP from hits (`docs/kh1-rehit.md`), and its no-damage countdown
  stops (`0x1402962EB`: `0x80` counts down only without `0x1000000`). So `0x80` and `0x100000`
  (the countdown's running phase, set once the reaction's action is over) stay on with it. That
  is the "unknown" `0x00100000` in the stuck flags 0110c083: frozen by the bit, not a hold of its own.
- Every room entry sets it for the walk-in and clears it at the end (the logs: flags 01008003 on
  entering, then 00008003). A bit left set therefore clears in the next room.
- The log (s307):
  - Guard Armor's end (228 s): the killing blow sets the bit. A few seconds later the
    script's action 4 comes, the bit is cleared, KH1 holds him (`+0x444` 1) and lets go. Flags are 0000c003.
  - Opposite Armor's intro (470 s): the bit is set for about 8 s, then the same clear, hold and
    release.
  - Opposite Armor's end (570 s): the bit is set. The script's action 31 comes, its hold ends, KH1
    lets him act (`+0x444` 0, its control step reads the pad, its own run and idle play), but no
    0x23 comes. Flags stay 0110c083, with no event and no script action, for minutes.
  - LuaBackend was loaded. Whether a skipped scene left the bit set isn't known.

**What the port does (Ours).**
- **The hand-back (`takeover::stale_hold`).** The script hold bit holds Sora only while some other
  signal says KH1 is moving him:
  - its script has actions for him (`+0x440`);
  - KH1 holds him (`+0x444` bit 1);
  - he can't act;
  - KH1's player control step was skipped;
  - an event runs (this one is checked before the bit).

  If none of these holds for 1.5 s (90 frames in a row), the bit counts as left over and KH2 takes
  Sora back. This is logged: "KH1's script hold (flag 0x1000000) left set ...". The log's "KH1 has
  Sora (KH1's script moves him ...)" line ends with what holds him.

  The bit counts again on any frame Sora isn't Sora's own (an event, a ride, another body), when
  the bit clears, or when the script has actions for him again. KH1's own hold (`+0x444`) while KH2
  has him is the ordinary "KH1 holds him still".

  The bit itself is left as KH1 has it. KH1 still takes no HP while it is set, as in KH1 itself,
  until a script clears it (the next room does).
- **Forms (`takeover::scene_frame`, `form::scene_revert`).** KH2 (as played, not traced) ends a
  Drive Form when a battle ends in a scene. Here, KH1 having Sora for a scene (an event, or its
  script holding him) ends the form when the scene lasts:
  - 1 s for an event (a cutscene);
  - 2.5 s for a scene that began in battle (KH1's battle flag on within the second before), such
    as a boss falling or a story moment mid-fight;
  - 20 s for any other scene, so a form is never left on for long under KH1's own animations.

  Door walks, room entries and talks (up to about 9 s in the logs, out of battle) keep the form,
  and KH2 takes him back in it.

  The revert is the same as on a new world: the Revert effects, Sora's controller and rig back, his
  look (`tint`) and stats (`valorstats`) at once. Anti Form reverts too.

  A Drive made just before a boss's intro scene ends with the intro. Opposite Armor's intro would
  do this (it held him about 11 s from 470 s).
- **Other bosses.** Only the two armors are in the logs:
  - Guard Armor's end and Opposite Armor's intro clear the bit themselves;
  - Opposite Armor's end doesn't.

  Any other script that leaves the bit set now gives Sora back to KH2 after 1.5 s of KH1 letting
  him act.

## When the forms open (the rules the user's, the flags Checked: scripts + saves)

The user's table (`kh1-hook/src/story.rs`, `FORM_TABLE`; `form::unlocked` reads it). The Dive
to the Heart's choice is `save + 0x48C` (0 Sword, 1 Shield, 2 Rod / Wand):

| Fight | Sword | Shield | Rod |
|---|---|---|---|
| Guard Armor (first form) | Valor | Limit | Wisdom |
| Opposite Armor (second) | Limit | Wisdom | Valor |
| Pegasus Cup, Leon & Yuffie (third) | Wisdom | Valor | Limit |

Master Form opens after Anti-Sora, Final Form after Ansem-Riku (Riku 2). Anti Form is the secret
code's, as before. Each tier stands alone (a randomizer's save that wins the Pegasus Cup first
gets the third form only). The level placeholders (97 / 98 / 99) are gone. The Drive list, the
Equipment page's V: / M: / F: rows and the form slots' copy counting (`equip`) all ask
`form::unlocked`. The flags and the forms open are logged at the start and whenever they change
("Drive forms: Dive choice ...").

The flags, all relative to the save block (`menu::save_block`, `menu::story_flags`; the same
layout on Steam, so no new addresses):

| Flag | Where | How it was found |
|---|---|---|
| Guard Armor | Traverse Town's story byte (`+0x1504`) ≥ 0x2B | Third District script: 0x28 as the fight starts, 0x2B once its enemies are cleared and its prize given; KH1's own snapshots `tw03_t011` (0x28, no journal entry) and `tw03_t021` (0x2B, Guard Armor's entry 0x9E). High. |
| Opposite Armor | Heartless journal entry 0x9F (`+0x16F6` bit 0) | Added by a Second District script function of its own once the fight is over, the same shape as Guard Armor's 0x9E. No story byte changes: Traverse Town stays 0x4E from the Gizmo Shop bell to the fight (snapshot `tw02_t021` at 0x4E has no 0x9F; the next world's `al00_00` has it; three of the user's saves sit at 0x4E without it). High. |
| Pegasus Cup won | cup state `+0xF37` ≥ 1 | The cup scripts step `+0xF36 + cup` 0 → 1 (normal), 1 → 2, 2 → 3 (the other rounds; trophy flags 20 + cup). Cups in order Phil, Pegasus, Hercules, Hades: their prizes are consecutive (`btltbl` prize table `0x5E..0x61`), their unlock flags (`+0xF2E + cup`) need Deep Jungle, then Monstro, then Neverland, then Hollow Bastion's end, and Pegasus is run in the arena that loads Leon's and Yuffie's battle models. High for the byte, medium-high for the order. |
| Anti-Sora | Neverland's story byte ≥ 0x32 | The hold's event at 0x1E sets 0x28 as it starts and the fight is in it (the journal entry 0x6F is added there, and KH1's snapshots show the battle's EXP between `pp05_y021` and `pp07_y021`); the next room's event sets 0x32 as Sora walks in. 0x28 already holds during the fight, so 0x32 is the first byte only reached after it. Medium-high (the form opens one room after the fight). |
| Ansem-Riku (Riku 2) | Hollow Bastion's story byte ≥ 0x82 | After Dragon Maleficent (0x6E, the value `growth::BOSS_GIFTS` uses, set right after its prize), the next room's entry script at 0x6E restores the party, runs the fight and its scenes and ends by setting 0x82; nothing is written between 0x6E and 0x82. Medium-high. |
| Pot Centipede | Agrabah's story byte ≥ 0x35 | Set right after its prize in its room's script (`growth::BOSS_GIFTS`'s value); snapshots `al04_h011b` (0x17) and `al04_h011c` (0x35, its journal entry 0xA2). High. |
| Hercules Cup won | cup state `+0xF38` ≥ 1 | As the Pegasus Cup (cup 2, run in the arena that loads Hercules). High / medium-high. |
| Ice Titan | journal entry 0xA6 (`+0x16F7` bit 1) and Diamond Dust owned (`+0x499 + 0x65`) | The Gold Match's script adds 0xA6 as the match starts and gives prize 0x94 when it is won, which the prize table (`*0x142D37758`) makes item 0x65, Diamond Dust (`kh1-equipment.md`); the Platinum Match's is 0x95, One-Winged Angel. Both together: a lost match or a Diamond Dust from elsewhere doesn't count. High. |

How the scripts were read (Checked, exe): an event script is 4-byte words, the top byte a class
(`0x1404E6A90`'s handlers), the rest a signed argument; class 9 pushes it, 12 / 13 load and store
a byte. A byte variable `0xB00..0xC7F` is `save + 0xA00 + arg` (`0x14019DF40`), so `0xB04 +
world` is the world's story byte. Class 24 calls opcode `arg` (`0x1401B13F0`): 0x206 adds a
Heartless journal entry (`0x1401B3830`: `save + 0xC00 + 0xAE3`, high bit first), 0x247 gives a
prize, 0x251 clears the room's enemies. The scripts are each room's `.ard` in the user's own
install (read with the PKG reader; nothing kept), and `event_data/*` are KH1's own save
snapshots at its events (the same 0x16C00-byte layout as the save block).

## Not modelled

- Form keyblade stats (forms deal Sora's base damage).
- The limits' light (`trap_limit_light`); summons, party members.
- The form change's full-screen effect.

## Growth abilities count what Sora has learned (s301)

The user: "for each drive form, their respective growth ability should always be 1 lvl above base
sora ... this includes if sora doesnt have the ability actively equipped. for example, if base
sora has lv1 quick run, wisdom form should have lv2 EVEN IF SORA DOESNT ACTIVELY HAVE QUICK RUN
EQUIPPED". A form's growth ability is `growth_up` of base Sora's level counting every ability in
his lists, equipped or not (`growth::Kh1Sora::learned`, kept in `menu::LEARNED` with the ini's
`abilities = kh1`; `form::growth_of`), at least what he has on. Base Sora himself still gets only
what he has equipped.

### The "New Drive Form" box (s308)

The user: "the game should tell you when you get a drive form and reflect". When a form opens by
the story while the same save plays, a box reads "New Drive Form: <name>" (`menu::Growth::form_box`,
the same queue as Reflect's "New Magic" / "Magic Up" and the bosses' gifts). Loading a save with
forms already open shows none (its story bytes or level jump).

## MP while in a form (s310, replaced in s318)

From s269 to s317 the forms ran on Sora's MP: 2 extra MP bars while a form lasted (switched off in
s310), and a drain of one unit of yellow MP every 9 frames of battle, back to Sora out of MP. Since
s318 a form runs on KH2's form gauge ("The Drive gauge in KH1", below) and MP is for magic only,
in a form as out of one. The extra-bars code (`valorstats::DRIVE_MP`) is gone.

## The Drive gauge in KH1 (s318; the rules KH2's, the numbers' sources given)

The user: "Now that we've got the drive forms down, lets incorporate the drive gauge." Their
choices: KH2's costs in Drive bars instead of MP; filled by Sora's hits, damage he takes and Drive
orbs, never by save points; a maximum of 3 bars growing with six fights won (s325; with each form
unlocked until then); and in a form
KH2's form gauge, draining, the form ending when it runs out. The model is `character_core::drive`
(the "Drive gauge" and "In a form" sections above); the tables are `kh1-hook/src/drivegauge.rs`;
the game side `win/gauge.rs`, `win/prize.rs`, `win/gaugehud.rs`.

**Costs** (`drivegauge::cost`; KH2's 03system `cmd` byte 0x1c): Valor 3, Wisdom 3, Limit 3 (KH2's 4; the user's, s331),
Master 4, Final 5. Anti Form (the secret code's) costs 9, which empties any gauge; it wants 3 full
bars to start (ours: KH2's Anti only comes out of a Drive the gauge could pay for, the cheapest 3).
The cost is paid as the Drive motion starts, by KH2's rule: a part-filled bar survives unless the
gauge was full or short. A form the gauge can't pay for is greyed in the Drive list
(`form::usable` -> `gauge::can_pay`) and refused with the gauge in the log; each row shows its cost
at its right end (`drive::list_draw`, not in a form, where the rows read Revert).

**Maximum** (`drivegauge::MAX_TABLE`, the user's, s325: "Let's have the drive gauge start at 3.
Have the drive gauge level up at these points: 1. defeating Sabor (boss) 2. defeating Riku I 3.
Beating the game 4. defeating Sephiroth 5. Defeating Kurt Zisa 6. Defeating Unknown"; and, the
third changed: "instead of beating the game, make it after defeating Chernabog"): 3 bars, +1 for
each of those six fights won, in any order; 9 (KH2's cap) with all six. Until s324 it was +1
per form and +2 for Final. It is worked out from the story flags (`story::Flags`, the table
below) every half second and never stored, so loading a save gives that save's maximum. Logged
when it changes: "Drive gauge: maximum 5 bars (3 base, Sabor +1, Riku 1 +1)".

| Fight | Flag (relative to the save block) | How it was found | Confidence |
|---|---|---|---|
| Sabor (the boss fight, his second) | Deep Jungle's story byte (`+0x1505`) ≥ 0x42 | Room `tz03`'s script sets 0x3F as it starts the fight (enemy group 0x67), waits for the battle's end (calls 0x23C, 0x240) and sets 0x42; KH1's snapshots `tz03_y021` (0x37) and `tz03_y021a` (0x39) before, `tz00_00` (0x44) after. The first Sabor fight (as Sora lands, room `tz01`) ends at 0x0D. `growth::BOSS_GIFTS`'s Sabor value is the same 0x42. | High for the byte, medium-high that it is the fight meant (the second, full boss fight) |
| Riku 1 | Hollow Bastion's story byte (`+0x150E`) ≥ 0x32 | Room `pc05`'s script waits for the battle's end (0x199), plays the scenes and sets 0x32, then swaps three character-journal entries for their next ones (0x211 removes 3, 9, 0x0B; 0x206 adds 4, 0x0A, 0x0C); snapshots `pc05_o011` (0x28, old entries) and `pc15_o021` (0x32, new ones); nothing writes a value between 0x28 and 0x32. | Medium-high |
| Chernabog (End of the World, atop the Final Rest) | End of the World's story byte (`+0x150F`) ≥ 0x33 | Room `ew27` is his (its actor 7 is named CHERNABOG). Its entry script sets 0x32 as Sora walks in. Once the fight is over its script gives prize 0x10 (its prize row 0xB1 0x04: type 1, ability 4 of list 0xB, which is Superglide by KH1's own `(4, 0xB)` test, `takeover.rs`; KH1's reward for him) and sets 0x33. Only `ew27` writes 0x33, and nothing writes a value between 0x32 and 0x33: End of the World's stores are 0x01 (`ew02`), 0x32 and 0x33 (`ew27`), 0x5A (`ew36`), 0x64..0x6E (`ew31`), 0x93 / 0x9B (`ew34`). Snapshots `ew01_a011` (0x00) and `ew00_00` (0x01) are without Superglide; `ew01_a011a` (0x33) and every later one (`ew31_a011` 0x5A, `ew34_a011` 0x6E, `ew00_02` 0x66) have it. The user's 100% Steam save is at 0x33. The Steam install's `ew27` is the same file. Glide MAX (`growth::CHERNABOG`) reads the same flag, the same value it had before. Randomizer saves can hold 0x33 (and 0x08, which no room script writes) with the rest of the story far behind, so there it may count early. | High |
| Sephiroth | Heartless journal entry 0xA5 (`+0x16F7` bit 2) | The Platinum Match's script (room `he07`) adds it after its 0x199 wait, once the battle is over (Ice Titan's 0xA6 comes as his match starts); its prize 0x95 is One-Winged Angel. In no snapshot (the last, `he07_t011`, is as the Platinum Match starts). | High |
| Kurt Zisa | Agrabah's story byte (`+0x1508`) ≥ 0x96 | Room `al01`'s script waits for the battle's end (0x199), adds his journal entry 0xA8 and sets 0x96 (`al19` steps it on to 0xA0); Agrabah's story ends at 0x82 before him. Snapshots `al01_a041` (0x82, no 0xA8) and `pp10_o041` (0xA0, 0xA8). | High |
| The Unknown | journal entry 0xA9 (`+0x16F8` bit 6) and item 0x3C owned (`+0x499 + 0x3C`) | Room `pc13`'s script adds 0xA9 (and sets its byte variable 0x518) as his fight starts, before its 0x199 wait; his prize 0x97 is item 0x3C (the prize table, `btltbl` + 0xC6A8), an accessory. Snapshots `pc13_o031` (neither) and `he07_t011` (both). Both together, as Ice Titan: the entry alone is a fight started, the accessory alone may come from elsewhere. The inventory counts what is equipped too (KH1's free count `0x1402E6330` takes the equipped from it); selling it would take the bar away. | High for the fight, medium for the prize test |

How the new reads were made (Checked, exe): the script call table is `0x1404FB550`; 0x199
(`0x1401C8D10`) waits until no enemy is left and ends the battle; 0x23C (`0x1401AF060`) and 0x240
(`0x1401AF0D0`) wait for the battle's end too; 0x247 gives a prize (`btltbl` + 0xC6A8, u16 rows:
type in the low nibble, the item in the high byte); the store class maps byte variables below
0x800 to `save + 0xC00 + var`. The rooms' scripts and KH1's snapshots are read from the user's
install, nothing kept. (s325's third row was the game's end, `+0xD41` set by `ew32` / `ew33`;
the user moved it to Chernabog.)

**Filling, out of a form** (KH2's rules from the exe, above):
- *Sora's hits*: every hit record KH1 makes for one of Sora's attacks on a foe (`probe::hit`,
  KH1's `0x1402BD180`; a foe by KH1's own test `0x14028FE30`: not marked not-a-foe, another
  team), so KH2's hits (landed through KH1's contact code) and KH1's own (its combo, its limits,
  its spells) alike. The value is the KH2 ATKP row's drive byte (0x29) for a KH2 hit
  (`combat::land` passes it: Sora's ground combo 3 and 4, spells 4 to 9); KH1's own attacks have
  no ATKP row, so they take KH2's value for the like (00battle `atkp`): a physical hit 3 (Sora's
  combo swings 127, 128, 130, 134), a spell 5 (Thunder's level-1 rows 249 / 250), told apart by
  the attack's data (KH1's spells' attack data, `combat::is_kh1_spell_data`). KH2 multiplies by
  0.75 on a normal enemy and truncates: a 3 adds 2 points, a 4 adds 3.
- *Damage taken*: KH2's Damage Drive rule with one copy, `damage / max HP × 50` points, truncated
  (`drivegauge::damage_points`): losing a tenth of his HP adds 5. KH2 itself gives nothing
  without the ability; the user wanted damage to count, so every Sora has it once. Read as his HP
  dropping from one frame to the next (`gauge::frame`; not across a change of max HP or actor).
- *Drive orbs*: a small orb 1 point, a big one 3 (KH2's prize table), below.
- Save points do not refill it (nothing of KH1's touches it).

**In a form** (`gauge::enter`, `gauge::frame`): the form gauge is `(level + 3) × 600` frames.
The form's level (s325, the user: "if sora has High Jump level 2, this means Valor Form has high
jump level 3 Which makes Valor Form level 3") is its own growth ability's level in the form
(`form::form_level`, `drivegauge::form_level`): base Sora's learned level + 1, MAX 4, the value
`form::form_abilities` gives it (Valor High Jump, Wisdom Quick Run, Limit Dodge Roll, Master
Aerial Dodge, Final Glide). So level 1 (nothing learned) is 4 bars, 2400 frames; 2 is 5 bars,
3000; 3 is 6 bars, 3600; 4 (MAX) is 7 bars, 4200. It is set as the form arrives and kept for the
form (a growth level learned in it counts from the next Drive). Until s324 every form was level
1. Anti: its own 6 bars (KH2's Anti level). It drains 1 a
frame in battle (KH1's battle flag) and 0.5 outside, every one of Sora's frames (KH2's or KH1's,
a KH1 limit in Limit Form included); each of the form's hits on a foe takes `15 × drive × 0.2`
frames (KH2's normal-enemy rate; 9 frames for a 3); each Drive orb gives `value × 12` frames
(Anti: takes them). Damage taken doesn't touch it. When it runs out the form ends as by Revert,
as soon as he can change (`form::drain` -> `before_step`), and never mid-limit (s314's
`limit_under_way` rule). Under the form gauge the Drive gauge keeps the bars the cost left; there
is no refill or cooldown on the way out.

**The gauge in the save** (Checked: exe scan + the user's saves). The bars and the points (0..100,
100 the full marker) are two bytes of KH1's save block: `save + 0x37` (bars) and `save + 0xAB`
(points). Each is the byte after a party record's weapon, record `+0x37`, Sora's record (0) and
Donald's (1, `+0x74`):
- A record is 0x74 bytes; its weapon is the byte `+0x36` and the next field a u16 at `+0x38`, so
  `+0x37` sits between a byte and an aligned u16: padding. (KH1's stats code also indexes the
  records from `R + 4`, the pointer it keeps in `stats + 0xC8`; there the weapon is `+0x32`, the
  pad `+0x33`, and its u16 at `+0x38` is `R + 0x3C`, the MP charge.)
- Every access found after a `× 0x74` record index (`imul $0x74`, the next 60 instructions) to
  `+0x33..+0x3B` is the weapon as a byte (`0x140284894`, `0x1402848F7`, `0x1402E69DE`, ...) or a
  u16 at `+0x38` / `+0x3A`; none touches `+0x37` (nor `+0x33`, its place from the `R + 4` base
  the stats code uses).
- All nine of the user's autosaves hold 0 at both bytes, in all ten records.
- The spare bytes before it are all taken (`+0x488` Valor's Keyblade, `+0x489` Final's, `+0x48A`
  the testing branch's, `+0x48B` Master's; `+0x48C / +0x48D` KH1's), as are `+0x48E..` (the party
  list) and `+0x492..` (spell levels).
- Medium-high: a static scan can't see an access through a pointer computed elsewhere. A whole
  record copy (KH1's party code) carries the byte with it.

Every change is written to the save block at once and every use reads it back
(`gauge::with`), so a save (at a save point, outside battle) keeps the gauge, and a loaded save
brings its own; a stray byte (more than 9 bars or 100 points) reads as an empty gauge, and bars
above the maximum are capped. **The form gauge isn't saved** (the simplest safe rule): KH1 saves
outside battle, and a save made in a form keeps the bars left after the cost, so loading it gives
Sora out of the form with those bars, as KH2's gauge after a Drive.

**Shown from the first form, full by default** (s323, the user). The gauge isn't drawn, and KH1's
MP HP label isn't hidden, until a Drive Form is open (`drivegauge::shown`: any of Valor, Wisdom,
Limit, Master and Final open by `story::unlocked`; s323 read it from the maximum, which no longer
follows the forms). The bars byte carries `SET` (0x80) once the mod has written it; a byte without it (a
new game, or a save from before s323) reads as a full gauge, and nothing is written before the
first form opens, so the gauge is full at the maximum it first shows with.

**Drive orbs** (`docs/kh1-prizes.md`): two new prize types in KH1's own prize system, 7 a small
orb and 8 a big one, recoloured copies of KH1's MP orbs (their rows, sprites, sizes, pull and
sound; KH2's yellow-orange colour). An enemy's own drop (its AI's drop op) rolls KH2's normal
enemies' drive-orb rows (00battle `przt`, the 87 rows with an item drop): 62 drop none, 10 four
small, 10 two big, 5 more (here six small and two big); about 0.8 small and 0.4 big a defeat.
KH1's code scatters, pulls, times out, picks up and plays them as its own orbs; the pickup adds 1
or 3 points, or 12 or 36 frames in a form (Anti loses them).

**The HUD** (`gaugehud`, `kh2hud`): KH2's own Drive gauge, drawn from the user's KH2 install, left
of Sora's portrait where KH2 puts it against its own (the user's second mockup), over KH1's small
MP HP label, which is hidden. Without KH2's art (`drive_gauge_art = kh1`, or it can't be read) the
s319 look drawn with KH1's pieces under the portrait instead.

*KH2's Drive gauge HUD* (Checked in the user's files and KH2's exe; Ghidra on a private copy, no
code kept):
- KH2's field HUD is `field2d/<lang>/<world>0field.2dd` (the name is built by `0x140153500` from
  `field2d/%s/%s0field.2dd`; `zz` is the default world, every world has one). It is a BAR of four
  image (`IMGD`, type 24) and sequence (`SEQD`, type 25) pairs: `fiel`, `gaug`, `mapj`, `miss`;
  `0x140153300` lists them and `0x1401532D0 (1)` hands out the `gaug` sequence, which holds the
  HP and MP rings, the Drive gauge and more (157 animation groups). Its image is 512 × 256 PSMT8
  on the PS2; the PC package carries a 1024 × 512 RGBA PNG beside it (the entry's remastered
  assets, one per image in the BAR's order, each starting on a 16-byte boundary).
- The sequence format: `crates/kh2-formats/src/seqd.rs` (sprites, parts, part groups,
  animations with two keys and an easing curve, animation groups with loop points and an attach
  point).
- The player's HUD object (`0x14017E920` sets it up) keeps the Drive gauge as three pieces
  parented to the bar: the bar (`+0x3540`), the fill (`+0x3740`) and the number (`+0x3940`, at the
  bar group's attach point), plus MAX (`+0x370`) and one-off effects. `0x14017F060` starts them;
  `0x140180920` runs every frame and `0x140180D70` updates the gauge from the player's gauge record
  (`+0x1AF` its mode: 0 off, 2 a form, 3 a summon, 4 locked; `+0x1B0` points, `+0x1B1` bars,
  `+0x1B2` the maximum, `+0x1B3` the form's bars, `+0x1B4` / `+0x1B8` the form gauge left / full).
- Which animation group plays when (`0x14017FCF0`, `0x14017FD60`, `0x14017FC10`, `0x14017FB40`):
  the bar 21 when it first appears (it slides up into place), then 23 (26 while the gauge record's `+0x1C0` is above 0, likely MP Charge,
  the DRIVE label blinking red; 19 locked, with chains 18); the fill 25; the number 28 + the bars
  (a table at `0x14071AEF8`); a bar gained (the bars up, not in a form) spawns 22 on the bar (the
  fill flashing white with streaks) and sound 0x20; a full gauge (bars = maximum) shows MAX, group
  20, with sound 0x21. In a form: the FORM bar 50, then 51; its fill 49; the number 54 + the
  form's full bars left (`0x14071AF20`; summons 139 / 141 / 144 +); changing mode spawns 130 (a
  ring of sparkles) on the HP gauge.
- The fill isn't scaled: KH2 turns a mask (a 50 × 120 box, `0x14017DC30`) over it by
  `points × 0.0040 rad` (a full gauge 200 points' worth; in a form `23° / bars × the current
  bar's share`), so the fill's slanted edge sweeps across. Here the fill is cut at the share
  instead (Approximate).

*How the mod draws it* (Inferred until a log confirms it): the art is read at start
(`kh2hud::load`), the image made once into a texture in KH1's renderer with KH1's own creator
(`0x1400F9F80`, as the form tints do), and every quad of KH2's sequence at its frame goes through
KH1's sprite path inside KH1's HP/MP gauge task: built from a Drive list row's record, its corners,
texture coordinates (counted in the HUD texture's PS2 size, the entry's `+0x94` / `+0x96`) and
colours set, and the HUD texture's entry in KH1's texture map (`0x14010D860`, by the quad's TEX0)
pointed at our texture for that draw (`+0x70`, put back at once). So it follows Sora's gauge: not
drawn in events (KH1's bit `0x142867CD8` & 8), faded with his gauge's alpha (`+0xA9C` of his record
`0x142812400`), and emptied by Page Down's HUD hide with the rest of that task's draws.

*Where*: KH2's gauge centre at the middle of the largest of Sora's gauge sprites as KH1 draws them
(his record's place from `0x14026B820`, the wide screen's shift included, plus each sprite's
rectangle in the two lists KH1's gauge draws there: `+0x50`, count `+0x70`; `+0x10`, count `+0x48`),
KH2's units scaled by 384 / 448 (`drive_gauge_scale` multiplies it), the DRIVE bar level with KH1's
MP HP label when one is found (a wide, low sprite left of the portrait's middle; its record's size
is 0 while KH1 draws, `hide_hp_label`). `drive_gauge = dx, dy` moves it (s318's `548, 352` reads as
no move); `drive_gauge_debug = on` draws a magenta block over it. The log names Sora's gauge's
sprites, the portrait and the label once, and every 3 seconds the gauge's box, which art drew it,
its alpha and the vertices KH1's immediate draw got and drew.

Why s318's gauge wasn't seen (Inferred from the exe): it drew in the command-menu task
`0x140279700`, which KH1 runs *before* the HP/MP gauge task (both priority 170001; the HUD setup
`0x140264C50` registers the command menu first and `0x140288780` puts a task after every one of
equal priority), so KH1's HP/MP gauge was drawn over its place; and at 0 bars it was nine 12 × 7
frames at 16% brightness and 60% alpha (KH1's sprite colours multiply the texture, 0x80 = as it
is) with nothing lit. s319's KH1 pieces were seen (the user's screenshot), in the colours of the
panel texture's middle (green and blue), not KH2's.

**The log**: "Drive gauge: maximum ..." when the maximum changes; "Drive: Valor Form for 3 bars
(Drive gauge 4 bars + 20 of 4 -> 1 bars + 20 of 4)" on a Drive; "Valor Form level 3 (High Jump
LV2 learned): form gauge 3600 frames (6 bars ...)" on arrival; "Drive gauge: shown (a Drive Form is
open)" when the first form opens; a summary once a second while the gauge changes ("Drive
gauge: 2 bars + 35 of 5; the last second: +6 from 3 hits, +5 from damage, +2 from 2 Drive orbs",
or in a form "Valor Form's gauge: 1834 of 2400 frames (3.1 bars); the last second: 60 frames of
the form, -36 from 4 hits"); "Valor Form: the form gauge ran out" and "Revert (the form gauge ran
out)"; "Drive orbs: an enemy's drop (...): 4 small, 0 big" and "Drive orb (small) picked up by 0:
+1; Drive gauge ...".

**Approximate (ours):**
- KH1 has no boss mark this reads, so every foe takes KH2's normal-enemy rates (×0.75 drive, ×0.2
  form-gauge drain per hit); KH2's bosses give the full value.
- KH1's own attacks' drive values (3 physical, 5 spell) are KH2's for the like, not per move.
- Damage Drive is on for everyone (one copy); MP Charge's ×2 doesn't exist in KH1.
- Drive orb drops are rolled per drop from KH2's normal-enemy rows, not per enemy kind; KH1's
  bosses drop as any enemy.
- The form level is its growth ability's level (1..4, the user's rule), not KH2's form EXP (KH1
  has none; KH2's forms reach 7); Master's orb EXP isn't modelled.
- The HUD's place and scale against KH1's portrait, and KH2's fill mask (a cut here), are
  approximate; the orbs' colour is tuned blind.

**To check in game:** a new save (no gauge until the first form; then full at 3), the maximum
growing after Sabor, Riku 1, the final fight, Sephiroth, Kurt Zisa and the Unknown (log), not
as forms open; each form's gauge as long as its level (Valor with High Jump LV2 learned: 6
bars, the HUD's number 5 when full); hits
filling it (KH2 combo, KH1 combo, KH1 spells, limits), taking damage filling it; the Drive list
greying rows by cost and showing the costs; a Drive paying its cost (log), the form gauge on the
HUD draining in and out of battle, hits draining it, orbs refilling it (Anti: draining), the
revert when it runs out (in the air, mid-combo, mid-KH1-limit in Limit Form); enemies dropping
yellow-orange orbs that behave as MP orbs, picked up by Sora and by a partner; saving and loading
(the bars come back, a save made in a form loads out of it); Page Down hiding the gauge; the gauge
fading in a cutscene.

## Partners (the rule the user's, the reads Checked: exe)

The user: "Let's make drive forms require their specific party members to cast. For example,
wisdom form needs donald alive to activate in KH2". As in KH2, where each form draws on its
partners:

| Form | Needs |
|---|---|
| Valor | Goofy |
| Wisdom | Donald |
| Limit | no one (KH2 FM: the partners stay by Sora's side) |
| Master | Donald and Goofy |
| Final | Donald and Goofy |
| Anti | no one (the secret code's, as before) |

A partner counts when he is one of the two actors after Sora in KH1's field party now and isn't
knocked out. Distance doesn't matter (KH2 doesn't ask). "Specific" means the named character: a
world guest (Tarzan, Aladdin, Ariel, Jack, Peter Pan, Beast) in Donald's or Goofy's place doesn't
stand in for him, so with Tarzan for Goofy only Wisdom and Limit are there. Without its partners
the form's Drive row is greyed (`form::usable`, which the Drive list's greying asks) and a choice
of it is refused with the reason in the log ("Drive: Wisdom Form refused (Wisdom Form needs
Donald: Donald is knocked out)"). Only the start asks: a partner going down while Sora is in the
form doesn't end it. The rule table is `kh1-hook/src/partners.rs` (unit tests there); the reads
are `form::field_party`.

How the actors are read (no new addresses, so nothing new for Steam):

- The field party is `0x142D37C80` (Sora), `+8` and `+0x10` (the two others), the same array
  Trinity Limit's check reads.
- Who an actor is: his object (`actor + 0x130`, a handle) `+0x4C` (u16), KH1's party member id,
  the number of the save's party records (0 Sora, 1 Donald, 2 Goofy, then the guests;
  `kh1-equipment.md`). KH1 reads it as the member itself: the stats refresh `0x1402A2CC0` passes
  it for each of the three actors to `0x1402A2FA0 (member, stats)`, which takes record `save +
  member × 0x74`; the HUD code `0x1402873A0` looks it up in the save's party list with
  `0x140282EC0`. Donald and Goofy being 1 and 2: the member-indexed weapon table `0x140528D60`
  (Sora `0x94F8`, Donald `0x9C88`, Goofy `0xA208` as entries 0, 1, 2; `kh1-equipment.md`). The object id is the character's, not a model's, so it
  holds in the worlds where Donald and Goofy wear other models (stats there come from the same
  records).
- The save's party list `save + 0x48E` (4 bytes: Sora, the two in the field, a fourth; 0xFF
  empty) is set by `0x1402835C0 (a, b, c)` (ids above 9 become 0xFF), cleared to Sora alone by
  `0x1402832E8`, and a member is taken out by `0x140283530`. It is logged beside the actors as a
  cross-check, not used for the rule (the actors are what is on the field).
- Knocked out: actor `+0x88 & 0x30` (KH1's own test `0x140299A30`). Bit 0x10 is set on the hit
  that empties his HP (`0x140299CA1`); his step turns it into 0x20 with a lie-down timer (`+0x3E4`
  from the battle parameters' per-character byte `+0x7C`, `0x14029A94E`) until he gets up. HP 0
  (stats `+0x3C`) counts as down too.

The party is logged whenever it changes: "Drive partners: Donald (member 1), Goofy (member 2);
save party list [00, 01, 02, ff]; partners there for Valor Form, Wisdom Form, ...".

## Partners vanish in the form (the rule the user's and KH2's; the reads Checked: exe)

The user: "lets make it so that the party member associated with the drive from also disappears
(along with their hud icon) for the duration of the drive from". As in KH2, the partners a form
takes leave the field while it lasts: Valor takes Goofy, Wisdom Donald, Master and Final both,
Limit no one, Anti Form both (KH2's Anti; only those in the field party). A guest in their place
never vanishes (they are found by member id, 1 Donald, 2 Goofy). They go when the change starts
(Sora's Drive motion) and come back beside Sora when the form ends, however it ends: the Revert
command, the form gauge running out, knocked out, a new world, a long KH1 scene (`form::scene_revert`), game over.
The log: "Drive: Goofy leaves for Valor Form" / "Drive: Goofy is back". The rule table is
`partners::taken` (unit test there); the code `kh1-hook/src/win/vanish.rs`.

**KH1's actor bits (Checked, exe).** The actor's flags word (`+0`):
- `0x8000`: the actor acts. KH1's actor step (`0x140295E50`) skips everything for an actor
  without it (`0x1402960E1` jumps to its end, `0x140296526`): the per-type control and AI
  callback, the motion, the physics step (`0x140298230`), the stop timer. The hit pass's test
  (`0x14028FEE0`: flags `& 0x8048 == 0x8000`) and the body push's (`0x14028FF80`: `& 0x8020 ==
  0x8000`, used by the push loop `0x1402991BC`) leave him out. KH1's game over takes it off every
  actor but the one going down (`0x140288130`, through its flag stack, below), and its Continue
  gives it back (`0x1402875D0` for the others, `0x1402874A0` for the party members, which also
  plays their revive and fades their HUD entry in, `0x140266CC0 (slot, 1)`).
- `0x1` and `0x2`: his model and his shadow. The party draw (`0x14029F360`) draws the model only
  with `0x1` (`0x14029F421`) and the shadow (`0x1402BB540`) only with `0x2` (`0x14029F466`). The
  first-person view hides Sora by clearing `0x1` (`0x1401E0307`, back at `0x1401E04C2`).
- `0x200`: not a foe. KH1's hostility test (`0x14028FE30 (a, b)`) is false for a `b` with it (and
  for the same team, `+0x374` bits 5-6); Trinity Limit's check wants it clear.
- `0x40`: untouchable (`kh1-intangible.md`).
- KH1's script op `0xCF` (`0x1402B9160`, table `0x140529E80`, 16-byte entries) takes a member out
  for good: it sets `0x240`, plays a motion and a state, marks `+0x374` bit 28 (nothing clears
  it) and hides his HUD entry with `0x140266CC0 (slot, 3)`. Its HUD and flag parts are the ones
  used here; its one-way parts are not.
- The flag stack: `0x14028FFF0 (actor, flags)` pushes the flags word to `+0x37C + depth × 4` (depth
  `+0x388`, three places) and sets the new one; `0x14028FFD0` pops. Only the game over pushes.
- Deleting an actor (`0x140290200`: `+0x374` bit 1, his party slot emptied) can't be undone in the
  room (a new party actor is made only by a room's load), so it isn't used.

**KH1's HUD gauge slots (Checked, exe).** `0x1428123E0`: a u32 "bound" flag per slot, and slot n's
record at `+ n × 0xDF0`, its actor `+0x28` (bound by `0x140266960 (actor, slot)` ->
`0x14026C4C0`), its alpha `+0xABC` and fade speed `+0xAC0`, which every gauge draw reads (`+0xA9C`
from the record's `+0x20`, `0x14026708C` and on). `0x140266CC0 (slot, mode)`: 0 fade out, 1 fade
in, 2 shown, 3 hidden (alpha and speed 0). Slot 0 is Sora, 1 and 2 the field party (their place
in the save's party list, `0x140282EC0`). KH1 sets all three at once at times (`0x1402872BE`,
`0x140287B3B`).

**What the mod does (Ours).**
- Away: flags `0x8000 | 0x1 | 0x2` off, `0x200 | 0x40` on, his old bits kept; his HUD slot (the
  one bound to his actor) to mode 3. Nothing is called on him and nothing in the save is
  written: the save's party list stays, so a save made in the form has him, and his HP, MP and
  state wait on his actor (no regen, no item use, no spells: his step doesn't run).
- Each of Sora's frames (`takeover`'s physics step) and at once on the form's end
  (`form::revert`, the change's end too) the state is checked:
  - the form (or the change) takes him and KH1's script doesn't have Sora (below): he is taken
    if he isn't yet; while
    away his model, shadow and HUD entry are kept hidden if KH1 turns them back on (it shows the
    whole HUD after an event or its game over);
  - else he is back: his bits as they were, placed 60 units beside Sora (on the side he was on,
    at Sora's height), his HUD entry faded in as KH1's revive does (or left hidden if Sora's own
    entry is hidden then);
  - his actor gone (a new room, the party changed: no member of his id in the field party) or
    given `0x8000` back by KH1 itself (a new room's actor at the same place, the game over's
    pop): nothing of ours is left on it; if the form still takes him he is taken again. KH1
    holding him frozen (no `0x8000`, its game over) is waited out.
  - KH1's game over pushed his flags while he was away: on his return the pushed copies get his
    bits back too, and `0x8000` stays as KH1 holds it, so its Continue pops him back whole.
- Whenever KH1's script has Sora they are back at once, and taken again if the form outlasts
  the scene (a long one ends the form, `scene_revert`) once KH1 has let go of him for 0.5 s
  (s314, below). A room change in the form: the new room's actors come in whole; they are taken
  once the walk-in (the script hold bit) is over.

**KH1's scripts command the party (s314; Checked: exe + Log).** The user: "weird bug in the
Pegasus cup, i beat a round and its frozen on donald doing his victory pose". The round was
fought in Valor Form with Goofy away.
- The log: at the round's end KH1 holds Sora (`+0x444` 1), its script has one action for him
  (31) for about a second, then only holds him (its control step skipped, no event, no script
  actions: `event 0x0, script 0x0`). Event op 103 then runs every frame for good.
- Event op 103 (`0x1401A7430`, the event op table at `0x1404FB550`) waits for its script actor's
  motion to end (`context + 8`, `0x14029CE30`: not looping, speed 0, at its last frame); it yields
  until then. The victory scene waits on each member's pose. Goofy, without `0x8000`, never
  stepped, so his motion never ended and the scene never did. Donald's pose played on.
- The partners came back only for an event (KH1's event flag), and this scene isn't one.
- `+0x444` bit 1 is set and cleared only by KH1's actor script commands (`0x14028C800`,
  `0x14028C7F4`) and zeroed by its resets: it is a script's hold, never set by KH1's own moves.
  In the logs it is on only at scene holds (fights' ends, story moments, talks); the plain "KH1
  holds Sora still" (its control step skipped) is on at many other moments (menus, its own moves)
  and isn't used.

**What the port does (Ours).**
- **The partners (`takeover::script_has_sora`, `vanish`).** KH1's script has Sora on a frame
  when an event runs, its script has actions for him (`+0x440`), its hold bit holds him
  (`0x1000000`, not left over: `stale_hold`), or it holds him (`+0x444` bit 1); not during one of
  KH1's limits in Limit Form (its limit module sets the hold bit; Limit takes no one anyway).
  Then the partners a form took are back on that very frame, their actors stepping before the
  script waits on them. They are left where they are if a script moved them while they were
  away (it placed them for its scene), else placed beside Sora as on a form's end. The log:
  "Drive: Goofy is back (KH1's script takes Sora)". If the form outlasts the scene they are taken
  again once KH1's script has let go of Sora for 30 frames (its own short gaps don't flicker
  them), with "Drive: Goofy leaves for Valor Form" again. Door walks and room entries in a form
  now show them for the walk, then they go.
- **A backstop (`vanish::script_waits_on`).** Event op 103 on a partner who is away (a script
  waiting on him without holding Sora) brings them back there and then: "Drive: Goofy is back
  (a KH1 script waits on him)".
- **The form at a cup round's end (`takeover::scene_frame`).** A frame on which KH1 only holds Sora
  still for its script (`+0x444`, no actions) now carries the scene's run on. Before, the run
  ended with the script's actions about 1 s in, so the 2.5 s rule for a scene that began in
  battle never came, and Valor stayed on through the victory scene. Such held frames end a form
  only in a scene that began in battle (2.5 s, as a boss's end), not by the 20 s rule (a long
  talk or a held menu keeps the form). A limit under way in Limit Form still puts the revert off
  (`form::limit_under_way`: its branch comes before the held one).
- Covered the same way: every cup's rounds (the same victory scene), Hercules' and Cerberus'
  scenes, Deep Jungle's (Tarzan, a guest, never vanishes; Donald or Goofy beside him do), and any
  scene that commands Donald or Goofy while it holds Sora.
- Trinity Limit and the other party checks see them as gone (KH1's own tests: no `0x8000`, `0x200`
  set); only Limit Form uses KH1's limits, and it takes no one.
- HUD hiding (`hud.rs`, Page Down) empties the HUD's draws: a slot hidden here stays hidden under
  it, and comes back with the HUD.

**Open.** Whether placing him beside Sora can put him behind a wall (KH1's own catch-up warp for
a partner left far behind wasn't looked for).

To check in game: each form with the full party (who goes, who stays, the HUD entries, the log
lines); Anti with one partner knocked out or away; Wisdom in Deep Jungle with Tarzan for Goofy
(Donald goes, Tarzan stays); revert by command, the form gauge running out, knocked out
(game over and Continue), a room change and a world change in the form, a cutscene in the form,
a save point (talking to it) in the form, then saving and loading; enemies ignoring where the
partner stood; Limit Form's Trinity Limit as before (Limit takes no one); Page Down's HUD hiding
in and out of a form; a cup round won in Valor, Wisdom and Master (the victory scene plays, the
partners are back at its start, the form carries into the next round: "Drive Forms in the
Coliseum" below); a door and a room entry in a form (the partners show for the walk, then go).

**Partners left invisible (s331).** The player: "In the Olympus Coliseum, if Sora uses Drive Forms
mid-match, in the next round Sora reverts to normal and allies become invisible."
- The cause (Ours, from the code; no log of it): a partner's return wrote back the flag bits saved
  when he was taken, whole. A partner taken again after a scene (30 quiet frames) while KH1 had his
  model off for a moment (`0x1`: a round's or a room's setup moving the party) had that saved, and
  came back with his model off for good, acting but unseen. The revert then had nothing left to do.
- Now he is taken only with his model on (`0x8001` both set), and his return always turns acting,
  model and shadow on (`0x8000 | 0x1 | 0x2`), the foe and touch bits as they were. KH1's own
  scripts turn them off again after that if a scene wants him hidden.
- A safety net (`vanish::stray`): a Donald or Goofy we hold no record of (his actor replaced or
  reset), when he should be here (no form takes him, or KH1's script has Sora), still showing
  exactly our bits (acting, model and shadow off, `0x240` on), with no game over's flag stack and
  not KH1's own leaving (`+0x374` bit 28): the bits are cleared and his HUD entry faded in as
  Sora's is. The log: "Drive: Goofy found with our away bits out of a form ...: shown again".

## Drive Forms in the Coliseum (the rule the user's; the rooms Checked: Log)

The user (s331): "in the coliseum, you should be able to maintain your drive form throughout
rounds. if you enter the cup in a drive form you should be reverted before the first battle, but
if you defeat a round in a drive form it should carry over into the next round". `win/cup.rs`.
- **Where (Checked: the s314 / s315 logs).** KH1's world `0xB` is Olympus Coliseum: room 0 its
  gates (Sora lands there), room 1 the lobby (Phil's cups), room 5 the Pegasus Cup's arena. Every
  match set has an arena room of its own (Pegasus "is run in the arena that loads Leon's and
  Yuffie's battle models", above); the arena is taken as any room from 2 on (medium: only room 5 was
  seen).
- **The cup's start.** Into an arena room from outside the arena in a form (or changing): back to
  Sora there and then, before the first battle. Log: "Coliseum: into the arena (room 0x1 -> 0x5)
  in a form: back to Sora before the first battle".
- **A round won.** `takeover::scene_frame`'s reverts (an event 1 s, a battle's end 2.5 s, a long
  scene 20 s) don't apply in the arena (`cup::carries`): the victory scene and the next round's
  setup keep the form. One line per scene: "Coliseum: a round's scene (a battle's end) in Valor
  Form: the form carries over into the next round (no revert)". The form gauge doesn't drain while
  KH1's script has Sora there (`cup::gauge_held`). The partners the form took are back for the
  scene as for any scene (it waits on their poses) and go again 30 frames after it, on the next
  round's actors if KH1 made new ones (the partners' rules above).
- **The cup's end.** A cup's state byte (`story::CUPS`, `save + 0xF36 + cup`, stepped by the cup's
  script when it is won) changing while in the arena: the scene rules apply again, so the trophy
  scene ends the form as a battle's end does. Log: "Coliseum: a cup's state changed ...". (When
  in the trophy scene the byte steps isn't traced; leaving the arena reverts anyway.)
- **Leaving the arena** (the cup over, lost or quit): out of an arena room in a form, back to Sora.
  Log: "Coliseum: out of the arena ...".
- Moving between two arena rooms keeps the form (logged), in case KH1 changes rooms between
  rounds.
- Knocked out still reverts at once, as anywhere.

**Open.** A story scene in the arena (Hercules', Cerberus') no longer ends a form by the scene
rules either; leaving the arena does. The arena's rooms besides 5 and how KH1 runs a round's change
(the same room kept or re-entered, the party's actors kept or made again) weren't traced.

To check in game: enter a cup in a form (reverted on entering the arena, partners back); Drive in
round 1, win it (the victory scene plays with the partners, the form and its gauge carry over, the
partners vanish again once the next round starts, never invisible afterwards); revert in a later
round (partners back and visible); win the cup in a form (the trophy scene ends it); lose a round
in a form (knocked out: reverted) and quit a cup in a form (reverted on leaving the arena).
