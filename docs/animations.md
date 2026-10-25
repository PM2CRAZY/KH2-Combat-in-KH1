# Animations: every move, KH1 and KH2 (testing branch, s284)

Built from the code on `testing` at s284. Main sources:
- `crates/kh1-hook/src/win/`: `pose.rs`, `shared.rs`, `styles.rs`, `settings.rs`, `stylecombo.rs`,
  `stylejump.rs`, `styleroll.rs`, `combomods.rs`, `takeover.rs`, `form.rs`, `formmagic.rs`,
  `reflect.rs`, `hurt.rs`, `combat.rs`, `kh2sound.rs`, `kh2fx.rs`, and `growth.rs` one level up;
- `character-core` (`motion`, `reaction::motion`, the combo rows), `kh2-rig` (KH2's blends),
  `kh2-data` / `kh2-formats` (what a KH2 motion id resolves to).

Anything the code doesn't pin down is marked **unconfirmed**. No ID here is a guess. Where an
ID comes from a play log rather than the code, the row says so.

Related docs: `shared-states.md` (who owns base Sora), `kh1-move-styles.md`, `animation.md`
(how KH2 motions are posed), `kh1-kh2-model.md` (the Player Model row), `drive-model.md`,
`kh1-drive.md`, `magic-model.md`, `aerial-recovery.md`, `reaction-commands.md`.

## Reading the IDs

- **KH1 slot**: an index into KH1 Sora's motion table (`xa_ex_0010.mset`). `pose` finds it each
  frame from the playing motion's counts (frames, channels, helper channels, helpers:
  `pose::slot_of`). This is the number in the log's "KH1 slot N" lines.
- **KH1 motion request** (hex): the id KH1 itself asks for (`0x14029D7F0`, read back at actor
  `+0x164`). The code names only a few: `0x71..0x74` glide, `0xDC` roll, `0x75` flight,
  `0x6E` clash recoil. Request ids are not slot numbers: KH1's request table (the mset's
  section 1) maps them, e.g. the glide's `0x71..0x74` (and Superglide's `0x76..0x79`) to slots
  75..78, `0xC8..0xCB` to 98..101 (`kh1-anim-slots.md` §1.5). The log's "set motion slot N" lines
  print the request id.
- **KH2 motion**: a `character_core::motion` id, the motion number in `obj/P_EX100.mset`. Each
  number has four slots, `id × 4 + variant`: battle with weapon, battle without, field without,
  field with (`Mset::slot_for`). `pose` uses the battle slot, plus the field slot where Sora has
  one of his own (`Rig::field_motions`: idle, walk, run).
- The brief listed KH2's "idle 5 / walk 0 / run 1". The code has **idle 0, walk 1, run 2, jump 3,
  fall 4, land 5** (`character_core::motion`; the s261 log agrees: "KH1 slot 0 (Motion { motion:
  0, field: true ...})", slot 11 → motion 5). This doc uses the code's numbers.

## The settings

The move styles (`styles.rs`) apply to base Sora only. Drive Forms ignore them (`styles::kh1_now`
is false in a form). They are switched from the Config menu and saved to `kh1-kh2.ini`.

The styles choose the **animation** (and whether KH2 borrows him for a KH2-only move). The
**physics** of every move is one setting, the Physics row (`physics = kh1 | kh2`, KH2's by
default; `shared::physics::source`, `docs/kh1-physics.md` §7), whatever the move's style: in the
tables below, "Physics KH2" / "Physics KH1" are that setting's values, not the style's. Drive
Forms are always KH2's.

| Config row | ini key | No key in the ini | First-run ini (`settings::DEFAULT_INI`) |
|---|---|---|---|
| Physics | `physics` | kh2 | kh2 |
| Base Combo | `ground_combo` | kh1 | kh2 |
| Aerial Base Combo | `air_combo` | kh1 | kh2 |
| Magic | `magic` | kh1 | kh1 |
| Aerial Magic | `air_magic` | kh1 | kh1 |
| Jump | `jump_style` | kh1 | kh2 |
| Glide | `glide_style` | kh1 | kh2 |
| Dodge Roll | `roll_style` | kh1 | kh2 |
| Misc Animations | `misc_anims` | kh1 | kh2 |
| Player Model | `player_model` | kh1 | kh1 |
| Battle Quotes | `battle_quotes` | `kh2_voice`'s value (off: kh1) | kh1 |

Other ini keys that change what is drawn or who moves him:

| Key | Default | Effect on animation |
|---|---|---|
| `shared` | on | KH1's states own base Sora (this doc). Off: the old model, where KH2's controller has him whenever KH1 shows slots 0..7. |
| `kh1_slots` | on | On (or a list of KH2 motions): KH2's moves that have one (now the ground combo: 151, 152, 154, and the air combo: 183, 184, 181) play as new slots in KH1's own motion table (`kh1-anim-slots.md`): KH1 plays the slot (its clock, hit / combo / trail windows, blends, sounds), KH2's motion is drawn at KH1's time. KH1's slots then report the KH1 slot they stand in for. |
| `glide` | square | KH1's glide starts on Square while falling, or on the held jump (`jump`). |
| `hurt` | kh2 | Hits taken while KH2's controller has him play KH2's reactions. kh1: always KH1's. |
| `grabs` | on | KH1's ledge, vine and rope grabs and swimming take him out of KH2's falls. |
| `commands` | on | KH1's commands (magic, items, specials, summons) take him from KH2. |
| `reflect` | on | KH2's Reflect in the Magic menu. |
| `air_cast_fall` | 1.0 | How fast he sinks while Reflect plays in the air (with physics KH1, and in Valor / Wisdom; not with physics KH2, nor in Final). |
| `drive` | on | Loads Valor, Wisdom and Final and shows the Drive page. |
| `valor_left_blade` | on | Valor's second Keyblade in his left hand. |
| `keyblade_turn` | on | Keyblades turned teeth up under KH2's poses. |
| `flip_weapons` | 0x53 | Weapons held with KH1's wrist under KH2's poses (the Dream Shield). |
| `swing_fx`, `kh2_sounds`, `kh2_effects`, `battle_quotes` | on, abilities, magnet_explosion, kh1 | Trails, sounds, effects and voices per KH2 motion. They don't change the motion. |
| `wall_clink` | on | A KH2 swing that meets a wall plays KH2's repelled motion (41 / 43). |
| `camera`, `camera_calls` | kh2, kh2 | Whose camera follows each state (the camera notes below). |

**Inconsistency:** `styles.rs` says "kh1 is on by default for all settings", and
`kh1-move-styles.md` says `misc_anims` defaults to kh1. Both are true only when the key is
missing. The ini written on first run sets six rows to kh2.

KH1's ability menu (`growth.rs`; `abilities = kh1`) holds both games' abilities:
- KH1's own ids 1..65: High Jump 1, Glide 3, Superglide 4, Combo Plus 6, Air Combo Plus 7, Guard
  21, Dodge Roll 22, Combo Master 65 (these have a KH2 meaning too). KH1's modifiers (Vortex 17,
  Aerial Sweep 18, Counterattack 19, Blitz 20, Slapshot 53, Sliding Dash 54, Hurricane Blast 55,
  Ripple Drive 56, Stun Impact 57, Gravity Break 58) are KH1's only.
- KH2's are ours, ids 66..87 (Quick Run 66, Aerial Dodge 67, Aerial Recovery 68, ...).
- The ability rows below give both the KH2 ability id (PTYA, `growth::kh2`) and our menu id.

## 30 fps and the frame step (applies to every table)

- **KH1's own states:** KH1 steps its motions by its frame scale (`0x1423405DC`: 1.0 at 60 fps, 2.0
  at 30). Our KH2 motion drawn over a KH1 slot (`pose::follow_play`) advances by the wall clock,
  so it plays at the same speed at either frame rate. Our physics wrappers on KH1's states
  (`shared::physics`) scale by the frame step. Steering's keep and drag (per KH2 tick, two 60 Hz
  frames) are raised to half its power.
- **KH2's controller:** whole 60 Hz steps a game frame (`takeover::steps`: 1 at 60 fps, 2 at 30, at
  most 4, a fraction carried over). Each step runs the controller, KH1's mover, hits, guard
  windows and sounds. Presses go to the first step. The Keyblade's and Sora's **pose** is worked
  out once a game frame (`RigAnim::update_deferred` / `settle`: the earlier steps keep the track
  and the blend clock, and draw nothing unless a hit or guard window is open), and KH2's
  **effects** take the frame's delta in one tick (`kh2fx::tick(motion, delta)`; the effect clock
  adds it up as the game's does).
- **The game's own frame step** (Steam exe, from the Ghidra decomp): the delta
  `0x140717480` is a whole number of 60 Hz units, never fractional. The logic callback
  (`FUN_140154700`, ~59.94 Hz) is paced by a timer thread; the main thread waits until `interval`
  of them have come (interval 2 when the fps setting is exactly 30.0, else 1) and takes
  `delta = callbacks since the last update` (clamped to 6.0). So the game takes **one** update of
  delta 2.0 at its 30 fps base, where we run two 60 Hz steps; higher or uncapped frame rates only
  add presents between logic callbacks. See the project note `kh2-frame-delta.md`.
- **Counted in 60 Hz frames** (`takeover::TICKS`): the hair and cloth swing, `hurt`'s re-hit grace,
  the cut windows of KH1's moves, a form's cast clock.

## Base Sora's slot map (`pose::play_for`, `shared::slot_kind`)

| KH1 slot | What KH1 plays | Style row (`slot_kind`) | KH2 motion drawn at KH2 | Clip group |
|---|---|---|---|---|
| 0 / 1 | field / battle idle | Misc Animations | 0 (field variant / battle) | 0 |
| 2 / 3 | field / battle walk | Misc Animations | 1 (field / battle) | 0 |
| 4 | battle walk: never seen, assumed | Misc Animations | 1 (battle) | 0 |
| 5 / 6 | field / battle run | Misc Animations | 2 (field / battle) | 0 |
| 7 | battle run: never seen, assumed | Misc Animations | 2 (battle) | 0 |
| 8 / 9 | high jump | Jump | 3 (not KH2's High Jump 201) | 1 |
| 12 / 13 | jump, rise | Jump | 3 | 1 |
| 10 | fall | Jump | 4 (loops) | 0 |
| 11 | landing | Jump | 5 | 0 |
| 98 | ground combo, 1st and 3rd swing | Base Combo | 151 (`Play::Swing`) | n/a |
| 99 | ground combo, 2nd swing | Base Combo | 152 | n/a |
| 100 | assumed a 3rd swing (never seen) | Base Combo | 151 / 152 by swing count | n/a |
| 101 | ground finisher | Base Combo | 154 | n/a |
| 102 / 103 | air combo swings | Aerial Base Combo | 183 / 184 by swing count | n/a |
| 104 | air finisher | Aerial Base Combo | 181 | n/a |
| 75 / 76 | glide slow / fast (requests 0x71 / 0x72, Superglide 0x76 / 0x77) | Glide | 205 | 5 |
| 77 / 78 | glide banking either way (0x73 / 0x74, 0x78 / 0x79) | Glide | 206 / 207, else 205 | 5 |
| anything else | magic, items, reactions, ledges, specials, cutscenes ... | none | none: KH1's own animation | n/a |

Slots in the same non-zero group carry the same clip on instead of restarting it. KH1's jump
going from slot 12 to 13 doesn't restart KH2's jump. Other slots seen in the s261 log: 58 for an
item's use (drawn with KH2's motion 9, below); 72, 75 and 77 while KH1 held him still. What
72, 75 and 77 are is **unconfirmed**.

## Movement

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Idle | KH1 states | KH1's idle, unmodded | KH2's idle drawn on KH1's slot: its field variant on the field slot | 0 → 0 (field), 1 → 0 (battle) | `misc_anims` / Misc Animations | **Physics:** KH1's (he stands). **Field or battle:** KH1's slot decides. While KH2's controller has him, KH1's battle bit decides, and the stance stays 6 s after an attack (`BATTLE_LINGER`). Forms are always in battle stance. **Borrow:** none. **Blend:** 8/60 s between KH2 loops. |
| Walk | KH1 states | KH1's walk | KH2's walk | 2 → 1 (field), 3 / 4 → 1 (battle) | `misc_anims` / Misc Animations | **Physics KH2:** KH1's ground step (`0x1402AF6C0`) scaled to KH2's walk speed (`shared::physics::ground_step`). KH1 still picks walk or run: stick past 0.8, KH1's 2.0 / 8.0 from its stats `+4` / `+8`. **Physics KH1:** KH1's own speeds. **Playback:** clip rate = his real speed ÷ (the clip's foot speed × hip ratio), 25..150 frames/s, smoothed, so the feet don't slide. Walk to run keeps the phase. **Footsteps:** KH1's (KH2's only while KH2's controller walks him). |
| Run | KH1 states | KH1's run | KH2's run | 5 → 2 (field), 6 / 7 → 2 (battle) | `misc_anims` / Misc Animations | As walk: KH2's run speed at KH2. |
| Item use | KH1 states (KH1 runs the item: its effect, timing, logic) | KH1's use motion | KH2's item-use motion over the whole use | KH1 slot 58 (s261 log) → 9 (`A110`, 50 game frames) | `misc_anims` / Misc Animations | Forced in `pose::follow` while `takeover::item_use_now`. Group 9, so a change of KH1's slot doesn't restart it. **Physics:** KH1's (its action flag 4 turns our speed scaling off). KH2 may take him from frame 51 (`ITEM_CUT`). Refused in the air (`combat::spell_not_yet`). |

## Jump and air

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Jump (takeoff, rise) | KH1 states | KH1's jump, unmodded | KH2's jump drawn on KH1's slots | 12, 13 → 3 (one clip, group 1) | `jump_style` / Jump | **Physics KH2:** KH2's arc on KH1's jump (state 1's pre-step `0x1402ADC10`, state 2's step `0x1402AD510`): KH2's height and gravity, the early-release hop (after 6 frames, more than 5 before the apex), terminal fall, air steering. KH1's own arc fields and air speed are written from KH2's every frame (`docs/kh1-physics.md`), so KH1's apex change of slot, its air-move gating and its own air moves go on from KH2's motion. KH1's early cut is kept off. Ceilings: the arc turns into a fall. **30 fps:** the arc advances by the frame step. **Physics KH1:** KH1's own arc (its stats' jump height `+0x10`, which its High Jump raises; gravity 0.4847, fall cap 20). |
| High Jump | KH1 states | KH1's high jump | KH2's plain jump motion on KH1's high-jump slots | 8, 9 → 3 | `jump_style` / Jump; KH1 ability High Jump (1) gives KH2's High Jump level | The High Jump is detected by KH1's landing-motion base `+0x3D0 == 5`. KH2's High Jump height is used at KH2. KH2's own High Jump motion 201 (its flip) plays only while KH2's controller jumps him. |
| Fall | KH1 states | KH1's fall | KH2's fall loop | 10 → 4 | `jump_style` / Jump | **Physics KH2:** KH2's arc. A fall KH1 starts itself (off a ledge) or KH1's own air move just over: KH2's arc goes on from KH1's fields (`kh1phys::Arc::to_kh2`), not from rest. After a give-back the fall goes on at KH2's speed (`physics::kh1_takes_air`): KH2's arc with physics KH2, KH1's fall picked up at that speed with physics KH1. KH1's own air action (flag 4: an air attack, a cast, a launch) keeps its own lunge, sideways speed and hop height, on KH2's gravity, fall cap and drift (a fresh hop as KH2's hop of that height, `kh1phys::kh1_arc_on_kh2`). KH1's hold (flag `0x1000`) is KH1's. **Physics KH1:** KH1's fall; a give-back goes on at the speed KH2's controller had, on KH1's numbers. |
| Landing | KH1 states | KH1's landing | KH2's landing | 11 → 5 | `jump_style` / Jump | Landing and its collision stay KH1's at both styles. |
| Jump out of a KH2 move | KH1 states | KH1's jump | as Jump | as Jump | none | `takeover::JUMP_OUT`: a fresh jump on the ground, where KH2's move can be left, goes to KH1's control step. KH1 jumps him and its states have him from then. |
| Aerial Dodge | KH2 borrow (at both Jump styles) | KH2's | KH2's | KH2 202 | Aerial Dodge (ours 67, from beating Genie Jafar). No style. | Borrowed on a fresh jump in the air, once per air time (`shared::AIR_JUMP`). **Physics KH2:** KH2's. **Physics KH1:** its own height and speed (KH2's growth numbers: KH1 has no Aerial Dodge) on KH1's gravity, fall cap and air steering. **Hits:** KH2's. Invulnerable for 8 frames + 2 per level (`AIR_DODGE_STAR`). A guard window (ATKP 406) all round. **Give-back:** after two frames of KH2's plain fall, into KH1's fall state. **Sound:** 999019 (`kh2_sounds`). Can grab ladders and vines from it (`grabs`). |
| KH2's own jump, fall, landing, high jump | KH2 always (inside a borrow, or in a form) | n/a | n/a | 3, 4, 5, 201 | none | `RigAnim::pick_motion`: the jump (or Aerial Dodge) clip while rising and not finished, then the fall loop. The High Jump flip plays to its end. Blend: 8 frames from jump to fall. |
| Flight (Neverland) | KH1 only (state 8) | KH1's flight | KH1's flight | KH1 request 0x75 | none | An Attack while flying borrows him for KH2's aerial combo (`FLY_ATTACK`), then he goes back to flying. Tinker Bell's sparkle is kept through it. **Camera:** KH1 flies in its ordinary follow mode (KH2's camera when `camera = kh2`). |

## Ground combo (Base Combo)

KH1's states run the ground combo at either style: KH1's rows, hit objects, timing, damage and
count (combat info `+0x41`; finisher at its stats' `+0xD4`). At KH2 only the look and the travel
are KH2's.

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| 1st / 3rd swing | KH1 states | KH1's swing | KH2's 1st swing, stretched over KH1's | 98 → 151 | `ground_combo` / Base Combo | **Draw:** `Play::Swing`, rate = 60 × KH2 length ÷ KH1 length, so the blade comes round as KH1's hit lands. No blend. KH2's root travel is taken out of the pose. **Physics KH2:** KH2's clip root travel, stretched the same way, along his facing, replacing KH1's own (`physics::swing_travel`, KH1's slots 98..101 with action flag 4), at either Base Combo style (with it at KH1, KH1's swing moved by the travel of KH2's swing for that slot). **Physics KH1:** KH1's own travel. **Hits:** KH1's. **Borrow:** KH2's ability moves at either style (`combomods`, the ability table). **30 fps:** the travel steps by the frame step; the draw by the clock. |
| 2nd swing | KH1 states | KH1's | KH2's 2nd swing | 99 → 152 | as above | Same rules. The user: "sora base attack 2, kh2 base attack 2". |
| 3rd swing (slot 100) | KH1 states | KH1's | KH2's 151 or 152 by the swing count | 100 → 151 / 152 | as above | Slot 100 is **unconfirmed** (never seen). `swing_motion` (physics) uses 151 for it. `play_for` alternates 151 / 152 by the swing count. A count goes over 1.2 s between swings (`COMBO_GAP`). |
| Finisher | KH1 states | KH1's finisher | KH2's finisher | 101 → 154 | as above | The user: "kh1 base finisher, kh2 base finishers". |
| KH1's combo modifiers (Vortex, Sliding Dash, Hurricane Blast, Ripple Drive, Stun Impact, Gravity Break, Blitz, Counterattack, Slapshot, Aerial Sweep: KH1's) | KH1 only | KH1's | KH1's | KH1 slots **unconfirmed** | KH1's own ability ids (17..20, 53..58) | Only slots 98..104 get a KH2 swing drawn. These moves are KH1's whatever the styles. |
| KH2's own ground combo inside a borrow | KH2 borrow | n/a | n/a | KH2's PTYA rows (from the install; which motions is **unconfirmed** beyond the rows named here) | none | An Attack while KH2's controller has him on the ground (in its roll's recovery, its Quick Run) is KH2's combo, not KH1's (`shared-states.md`, "Not yet"). **Hits:** KH2's ATKP windows, at 60 Hz. |

## Aerial combo (Aerial Base Combo)

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Air swings and finisher | KH1 states at KH1; KH2 borrow at KH2 | KH1's air combo, unmodded | KH2's controller: its aerial combo, its rows | KH1 102 / 103, 104 (finisher). KH2's rows from its PTYA. `play_for` maps 102 / 103 → 183 / 184 and 104 → 181. | `air_combo` / Aerial Base Combo | At KH2 an Attack in the air borrows him (`combat::exec` → `ATTACK_AFTER`), so KH1's air slots don't play. The 181 / 183 / 184 mapping is drawn only if KH1's own air combo plays while the style is KH2's, which is not the normal path. **Physics KH2:** KH1's air combo (at KH1) keeps its lunge and hop height, on KH2's gravity, fall and drift; KH2's (at KH2) is KH2's. **Physics KH1:** KH1's at KH1; at KH2, its rows' hops and lunges on KH1's gravity, fall and air steering. **Hits:** KH1's at KH1, KH2's at KH2. **Borrow at KH1:** KH2's air ability moves (the ability table). **Give-back:** KH1's fall, 24 frames after the last swing (`AIR_COMBO_DONE`) or when the combo is over. |

## KH2 ability moves (borrows out of KH1's moves, or inside KH2's own combo)

Owner: KH2 borrow. How they are reached (`combomods`):
- **An Attack** that KH1's combo took, where KH2's combo would use an ability move at KH1's
  count (`Character::would_take`): an opener, step or finisher that needs an ability; Negative
  Combo's earlier finisher; a longer combo (Combo Plus); Finishing Plus in KH1's finisher.
- **Square** in KH1's combo window (event flag `+0x1CC` bit 22, slots 98..104), if KH1's swing
  connected or Combo Master is on.
- **The jump or Square** while KH1 has him knocked away (kind 4 / 5), from frame 16, more than
  24 above the floor.

KH1's swing or reaction is cut at the press (its action flag 4 off, its motion request set to
its idle). The move then plays with KH2's motion, physics and ATKP hit windows, at 60 Hz. After
it, the next Attack in KH2's combo window is KH1's next row with KH2's count, unless KH2's next is
an ability move again.

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Upper Slash | KH2 borrow | KH2's (Square in KH1's combo) | KH2's | 161; PTYA row 12 | KH2 ability 18; ours 73 (Sabor) | A ground Square move. |
| Slapshot | KH2 borrow | KH2's | KH2's | 162 | 90; ours 82 | |
| Dodge Slash | KH2 borrow | KH2's | KH2's | 163 | 91; ours 69 | |
| Slide Dash | KH2 borrow | KH2's | KH2's | 164 | 92; ours 83 | An opener on a far target. |
| Guard Break | KH2 borrow | KH2's | KH2's | 165 | 93; ours 86 (Pot Centipede) | A finisher. KH2 effects 31, 30, 32 (`kh2_effects` default). |
| Explosion | KH2 borrow | KH2's | KH2's | 166 | 94; ours 84 (Dragon Maleficent) | A finisher. `magic_scale`. KH2 effect (default list). |
| Finishing Leap | KH2 borrow | KH2's | KH2's | 167; PTYA row 37 | 95; ours 81 | A ground Square finisher. Effects 33, 34. Ported to Valor (below). |
| Flash Step | KH2 borrow | KH2's | KH2's | 169 | 177; ours 78 (Sabor) | An opener on a far target. |
| Vicinity Break | KH2 borrow | KH2's | KH2's | 170 | 180; ours 85 | KH2 effect (default list). |
| Counterguard | KH2 borrow (inside KH2's guard) | only from KH2's guard | KH2's | 171; PTYA row 39 | 96; ours 80 | An Attack during the guard's repel (motion 42). Reached only where KH2's guard runs (Dodge Roll at KH2). |
| Retaliating Slash | KH2 borrow | Square while KH1 has him knocked away | Square in KH2's blow | 172; PTYA row 40 | 101; ours 75 | From frame 16 of the knock-away (`combomods::RECOVERY_AFTER`). |
| Aerial Sweep | KH2 borrow | KH2's | KH2's | 191 | 97; ours 79 (Clayton / level 28) | |
| Aerial Spiral | KH2 borrow | KH2's | KH2's | 192 | 98; ours 72 | |
| Horizontal Slash | KH2 borrow | KH2's (air Square in KH1's combo) | KH2's | 193; PTYA row 38 | 99; ours 70 | An air Square move. |
| Aerial Finish | KH2 borrow | KH2's | KH2's | 194; PTYA row 29 | 100; ours 87 (Jafar) | An air finisher. KH2 effect (default list). |
| Aerial Dive | KH2 borrow | KH2's | KH2's | 196 | 178; ours 76 (Cerberus) | From the ground at a far airborne foe. |
| Magnet Burst | KH2 borrow | KH2's | KH2's | 197 | 179; ours 77 (Captain Hook) | An air finisher. `magic_scale`. KH2 effect (default list). |
| Aerial Recovery | KH2 borrow | the jump while KH1 has him knocked away | the jump in KH2's blow | 40 (`A212`) | ours 68 (KH2 ability 20, `aerial-recovery.md`) | `Character::air_recovery`: a 100 hop, 10 / frame along the stick. Invulnerable 0..16. KH2 effect element 0. In a form only if the form has motion 40. |
| Negative Combo, Finishing Plus, Combo Plus, Air Combo Plus, Combo Master | n/a (modifiers) | change when KH2 borrows (above) | change KH2's own combo | no motion of their own | ours 71, 74; KH1's 6, 7, 65 | KH1's combo reads Combo Plus, Air Combo Plus and Combo Master itself. |

Motion ids are from `kh2sound::DEFAULT_MOTIONS` and `kh2fx::DEFAULT_MOTIONS`. PTYA rows are from
`character_core::controller` (`SQUARE_MOVES`, `GUARD_ROW`, `COUNTERGUARD_ROW`,
`RETALIATING_SLASH_ROW`) and `kh2fx` (row 29). The other moves' rows come from the install's
PTYA table. The code doesn't name them.

**Inconsistency:** `pose.rs` (`PORTED_TO_VALOR`) and `form.rs` (`UPPER_SLASH = 95`) call row
37 / ability 95 / motion 167 "Upper Slash". `character-core`, `growth.rs`, `kh2sound` and `kh2fx`
all call it Finishing Leap; Upper Slash is ability 18, row 12, motion 161.

## Square moves

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Dodge Roll | KH1 states at KH1; KH2 borrow at KH2 | KH1's roll: Square with the stick, on the ground, out of deep water | KH2's roll (moving) | KH1 request 0xDC (its slot **unconfirmed**) → none. KH2: 209. | `roll_style` / Dodge Roll. KH1's roll needs KH1's Dodge Roll (22: bit `0x20` of `0x142D5F610`). KH2's needs Dodge Roll, Guard or Quick Run equipped. | **Physics:** each roll's own travel at either setting (KH1's roll at KH1; KH2's root motion at KH2: no numbers of the other game's for it). Its placement is pinned to his position for 0.6 s (`pose::on_actor`, the s244 decouple fix). KH2's at KH2. **Invulnerability:** KH1's motion events (flag `0x40`) at KH1; KH2's clip windows at KH2. **Draw:** no stale KH2 pose over KH1's roll; it blends in from the last drawn pose. Without anything of KH2's for Square there is no borrow (s261's "unusable" roll; the log says why). |
| Guard | KH1 states at KH1; KH2 borrow at KH2 | KH1's guard (KH1's Guard 21, bit `0x10`; with Dodge Roll too, the stick's tilt picks) | KH2's guard (standing Square) | KH1 slot **unconfirmed** → none. KH2: press 173 (row 11; guard window 6..37), block / repel 42 (ATKP 162) | `roll_style` / Dodge Roll; Guard (21) | `hurt = kh2`: KH2's guard windows stop hits as KH1 guard objects, merged across the frame's 60 Hz steps. A guard on his body guards all round. The swish sound is in `kh2_sounds`. |
| Quick Run | KH2 always (a borrow; no style of its own) | Square held through KH1's roll; or Square while moving without KH1's Dodge Roll equipped | out of KH2's roll, Square held | 203, end 204 | Quick Run (ours 66, from level 12) | Out of KH1's roll: taken at KH2's roll Quick Run check frame (its clip's event, else 16) or when KH1's roll is over (`shared::QUICK_RUN`). It starts along his facing (`quick_run_now`); KH1's roll motion is set to its idle (`CUT_LEFT`). At least 6 invulnerable frames (`QUICK_RUN_STAR_MIN`). **Physics:** its own speed, time and brakes (KH2's growth numbers, no gravity in it) at either setting: KH1 has no Quick Run. |
| Square modifiers (Upper Slash, Finishing Leap, Horizontal Slash) | KH2 borrow | Square in KH1's combo window | Square in KH2's combo window | 161 / 167 / 193 | abilities 18 / 95 / 99 | See the ability table. |

## Glide

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Glide (slow, fast) | KH1 states (glide state 0x15) | KH1's glide or Superglide, unmodded | KH2's glide motion on KH1's glide state | 75 (0x71), 76 (0x72) → 205 | `glide_style` / Glide; ini `glide`; KH1's Glide (3) / Superglide (4) | **Start:** `glide = square`, Square while KH1 falls (`takeover::kh1_glide_start`, Superglide when he has it). `glide = jump`, KH1's own held jump. Once per hold of Square. **Physics KH2:** the pre-step `0x1402B06F0` turns him at KH2's glide turn rate. The step `0x1402B0890` moves him at KH2's speed, `(1 - a) × top × tilt + a × last`, and fall, `(height - fall height) × ratio`, capped. These come from KH2's growth numbers at Sora's Glide level (1 at least). At either Glide style. **Physics KH1:** KH1's own glide. KH1 keeps the start, end, slots and collision. **Blend:** 16/60 s into the glide. Slots 75..78 share group 5, so a change between them keeps the clip. |
| Glide banking | KH1 states | KH1's | KH2's banked glides | 77 (0x73) → 206, 78 (0x74) → 207 | as above | Banked glides missing from a rig: 205. **Unverified:** slots 76 and 78 (75 and 77 seen in the s261 log), and which way each bank faces. Before the s284 fix the code took 113..116 (attack motions) for the glide. |
| Glide hover | KH2 always (Final Form only) | n/a | n/a | 208 | none | No KH1 slot maps to it. Base Sora's KH2 glide is off under shared states (its level kept in `KH2_GLIDE_KEPT`). Final Form glides with KH2's own glide, 205..208. |

## Magic

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Fire, Blizzard, Thunder, Cure (ground) | KH1 only at KH1; KH2 borrow at KH2 | KH1 casts its own spell, its own animation | KH2's base-form spell: its MAGC cast motion, its script | KH1 slots **unconfirmed**. KH2: MAGC row (ground / combo finisher / air). The ids come from the install; if the row has none, 56 ground, 58 air. | `magic` / Magic | At KH2: the command makes KH2 borrow him (`shared::want_kh2`, `combat::resume_command`), then `formmagic::spell_chosen`. KH1's level and MP cost. Sub = level × 2 − 2 (+1 finisher). **Hits:** KH2's `.mag` script, released at the cast's magic-cast event. A missing file or MAGC row: KH1 casts its own. **At KH1:** grounded Fire can be cut from frame 49 (`FIRE_CUT`). |
| Gravity, Stop, Aero | KH1 casts at both styles | KH1's own animation | KH1's spell drawn with base Sora's KH2 cast motion, on that motion's clock | Gravity → 62 ground / 64 air (Thunder's). Stop, Aero → 65 / 67 (Cure's). | `magic` / `air_magic` | `formmagic::BASE_KH1_CAST`, `takeover::command_started` (`FormCast`: the motion's length, input window, allow-movement range). **Inconsistency:** the clock reads the motion from MAGC (`kh1_spell_motion`, these numbers only if the row lacks one); `pose::follow` draws the hard-coded 62 / 64 / 65 / 67. |
| Aerial Magic | KH1 casts at KH1; KH2 borrow at KH2 | KH1's air cast, its pose held until he lands | KH2's air cast motion | KH1: slot 80 was the air cast in one log (`takeover` comment; the spell isn't named). KH2: the MAGC air motion. | `air_magic` / Aerial Magic | At KH1, KH2 takes him on landing. **Physics KH2:** KH2's casts' own air physics (the cast motions' no-gravity frames, KH2's gravity), not KH1's fall; KH1's air casts (at KH1) keep KH1's halved sideways speed, on KH2's gravity, fall and drift. **Physics KH1:** KH1's fall (KH2's casts: KH1's gravity, `air_cast_fall` of it). |

## Reflect

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Reflect / Reflera / Reflega | KH2 always (a borrow by command) | KH2's | KH2's | MAGC: ground 71, combo finisher 72, air 73 (also the defaults if the row is missing) | ini `reflect`; level = KH1's Aero level; `air_cast_fall`. The Magic styles don't apply. | **Hits:** `magic/REFLECT_<n>.mag`, released at frame 0. The barrier (ATKP 573) stops and counts hits, then the counter fires (574, then the 667 burst). No harm all through the cast motion (`reflect::cast_motion`). **Air:** his fall is stopped as it starts, then he falls at `air_cast_fall` × KH1's gravity. Physics KH2 (whatever the Aerial Magic style), or Final Form: KH2's own air physics. |

## Items

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Item use | KH1 states | KH1's | KH2's 9 | 58 (s261 log) → 9 | `misc_anims` | See Movement. On the ground only. |

## Reactions and hurt

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Hit while KH1's states have him | KH1 only | KH1's reaction (`0x140299A40` by movement state) | KH1's (no style) | KH1 reaction slots **unconfirmed**. Clash recoil (kind 6): KH1 request 0x6E (`kh1-reflect.md`). | `hurt` doesn't apply here | A knock-back or launch (kind 4 / 5) starts the window for KH2's Aerial Recovery or Retaliating Slash (`combomods::kh1_blown`). |
| Ground flinch | KH2 always (while KH2's controller has him) | n/a | n/a | 10, 11 (front), 12, 13 (behind) | `hurt = kh2` | KH1's damage is applied; its reaction is cleared; KH2 plays its own (`hurt.rs`). Kind 3 → a flinch. The same attacker's hits within 20 60 Hz frames don't land (`REHIT_GRACE`). |
| Air flinch | KH2 always | n/a | n/a | 14 (front), 15 (behind) | `hurt = kh2` | |
| Knocked away (blow) | KH2 always | n/a | n/a | 16 (front), 18 (behind); +1 on landing (17, 19) | `hurt = kh2` | Kind 4 / 5 → a blow. No hit lands while down (`Fate::Down`). Aerial Recovery and Retaliating Slash come out of it. |
| Special reaction (0xe) | KH2 always | n/a | n/a | 23 / 24 | `hurt = kh2` | `reaction::motion::SPECIAL`. Whether the hook ever sends one is **unconfirmed**: it sends only kinds 3 and 4. |
| Guard repel / repelled | KH2 always | n/a | n/a | 42 (guard repel), 41 ground / 43 air (repelled by a wall or an attack) | `hurt`, `wall_clink` | KH1's attack clash (kind 6 / 7) → KH2's recoil. A wall clink plays 41 / 43; Final Form has neither, so no bounce. |
| Evaded / reflected | KH2 always | n/a | n/a | no motion | `hurt` | Invulnerable frames, Reflect's barrier and the whole Reflect cast drop the hit. |
| KO | KH1 only | KH1's | KH1's | KH1's | none | KH1's dying pass. Out of a Drive Form at once (`form::knocked_out`). |

## Ledges, climbing, swimming and other KH1-only states

All KH1 only, at either style: KH1's own animation, physics and collision. No KH2 motion is
drawn (their slots have no `play_for` entry). KH2's controller has no ledges in the hook
(`ledges: &[]`), so KH2's Hang (7) and Climb (8) never play.

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Ledge grab and hang | KH1 only (state 13) | KH1's | KH1's | KH1 slots **unconfirmed** | `grabs` | Tried from KH2's falls too: KH2's Air / Jump / Aerial Dodge / Glide states, after a 20-frame wait off something (`NO_GRAB`). **Camera:** KH1's ledge camera is KH2's follow camera (`camera_calls = kh2`). |
| Vines, climbable walls | KH1 only (state 10) | KH1's | KH1's | **unconfirmed** | `grabs` | As ledges. |
| Ladders, poles; leaping off | KH1 only (states 15, 16) | KH1's | KH1's | **unconfirmed** | `grabs` | Leaping off: KH2's fall once he drops, or KH1's fall state under shared states. Camera as ledges. |
| Bars, objects to hang from | KH1 only | KH1's | KH1's | **unconfirmed** | `grabs` | Bars are skipped where KH1 turns them off (`0x298D3A8`). |
| Swimming, deep water | KH1 only (state 17) | KH1's | KH1's | **unconfirmed** | `grabs` | No roll in deep water. **Camera:** KH1's (water is a KH1-only camera state). |
| Carpet, Jungle Slider, water states | KH1 only (states 24, 25, 29, 30, 31; which is which is **unconfirmed**) | KH1's | KH1's | **unconfirmed** | none | `takeover::kh1_mode` keeps KH1 in charge; `kh1_camera_mode` gives KH1's camera. |
| Summons | KH1 only (38 Simba's charge, 40 the call) | KH1's | KH1's | **unconfirmed** | `commands` | Keep KH2's camera. |
| KH1's specials (Sonic Blade, Ars Arcanum, Strike Raid, Ragnarok, Trinity Limit) | KH1 only | KH1's | KH1's | **unconfirmed** | KH1's abilities 11..15; `commands` | |
| Mermaid Kick, field actions (examine, talk, open) | KH1 only | KH1's | KH1's | **unconfirmed** | none | `kh1-move-styles.md`: no clean KH2 counterpart. |
| Events, cutscenes, area-script walks (doors, zone entries), the Heartless body | KH1 only | KH1's | KH1's | KH1's | none | `takeover` lets KH1 have him (event flag, `+0x440` scripts, `0x1000000`, another body). `pose` draws KH1's own animation (`SCRIPTED`). |

## Drive Forms

All forms: KH2 always. The form's controller has him all through (`shared::on` is false in a
form), and the move styles don't apply. The draw is the form controller's pose. When KH1 has him
in a form (a KH1 spell, an item), KH1's slot is drawn through `play_for` with the form's motion
of that number if its rig has one, else base Sora's (`shared::kh2_look` is true for every slot in
a form). Slots with no `play_for` entry (most spells) show KH1's own animation.

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Drive (the change) | KH2 always | n/a | n/a | base Sora's 52 (cut at frame 39), then the form's arrival 53 (6 frames) | `drive`; 3 MP | Invulnerable. Held in place in the air. Taken only when he can change (`can_change_form`), never buffered. |
| Revert | KH2 always | n/a | n/a | no motion (base Sora's 4 is his fall loop, `A004`) | `drive` | He comes back free, his movement kept. Effects and sound are Sora's elements 15..27 under `REVERT_FX`. |
| Form abilities | n/a | n/a | n/a | n/a | n/a | Sora's support abilities carry over. Guard only if the form has motion 42. Aerial Recovery only if it has 40 and Sora has it. Every row of the form's PTYA table is unlocked. |

### Valor Form (`P_EX100_BTLF`, moveset 2)

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Valor's moveset | KH2 always | n/a | n/a | Valor's own motions and PTYA rows (its kind-0 rows 12, 38, 40, 11 are its finishers and guard). Other ids **unconfirmed**. | `drive`, `valor_left_blade` | Two Keyblades: KH2's second-weapon hitboxes, KH1's model of the slot's Keyblade in his left hand (`leftblade`). High Jump at MAX. No magic (Magic greyed). |
| Finishing Leap (ported) | KH2 always | n/a | n/a | Sora's row 37 / motion 167 copied in (`PORTED_TO_VALOR`) | Finishing Leap equipped (95; ours 81) | Only while equipped. The code labels it Upper Slash (inconsistency above). |

### Wisdom Form (`P_EX100_MAGF`, moveset 3)

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Shots (its combo) | KH2 always | n/a | n/a | Wisdom's 151 (shots at frames 10/20/30), finisher 154, air 181 (`kh1-drive.md`) | `drive` | Frame trigger 1 runs the form's `wisd` script. ATKP 859 homing shots. Ground inertia: acceleration 0.85, brake 0.97. |
| Fire, Blizzard, Thunder | KH2 always | n/a | n/a | the form's MAGC cast motions (56..67, `kh1-drive.md`) | `drive` | Sub +10. KH1's level and cost. |
| Cure, Gravity, Stop, Aero | KH1 casts | n/a | n/a | Gravity → 62 / 64; Cure, Stop, Aero → 65 / 67 (ground / air) | `drive` | KH1's spell on the form's cast motion's clock and slide (`FormCast`). The user: "kh1 cure with kh2s cures animation". |
| Quick Run | KH2 always | n/a | n/a | 203 / 204 | n/a | At MAX. |

### Final Form (`P_EX100_ULTF`, moveset 5)

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Final's moveset | KH2 always | n/a | n/a | Final's own motions and rows (ids **unconfirmed**) | a secret pad code; `drive` | Two floating Keyblades where KH2's weapon motions put them (`leftblade::final_float`); the second is the Kingdom Key. No repelled motion, so a wall doesn't bounce him. |
| Glide | KH2 always | n/a | n/a | 205..208 | n/a | At MAX. KH2's own glide (not KH1's glide state). |
| Magic | KH1 casts | n/a | n/a | KH1's own animation (spell slots have no `play_for`) | n/a | No KH2 magic in Final (`spell_chosen` covers Wisdom and base only). Reflect, and air casts, use KH2's air physics in Final. |

### Anti Form, Limit Form, Master Form

Not in the KH1 port. `pose::load_kh2` loads only Valor, Wisdom and Final.
`character_core::drive` models Anti's gauge rules, and `kh2-data` lists every form's files, for the
playground only.

## Menus and pause

| Move | Owner | At KH1 | At KH2 | Animation ID(s) | Setting | Notes |
|---|---|---|---|---|---|---|
| Pause menu's Sora | KH1 only | KH1's menu animation | the same | KH1's | `player_model` | A second instance of his model, more than 150 from his actor, left to KH1 (`pose::at_sora`, `MENU_SORA`). With Player Model KH2 its pose is carried onto KH2's skeleton (`model::note_menu`). Its Keyblade stays KH1's. |
| Pausing mid KH2 move | KH2 | n/a | n/a | the KH2 pose last drawn | none | Who had him is counted in game frames (`FRAME_KIND`), so KH2's pose stays up through a pause instead of going back to KH1's slot. |
| Battle menu, Drive page, Config rows | n/a | n/a | n/a | no animation | `config_row`, the style rows | A style row switches at once (`styles::set`) and is saved to the ini. Spells, items and summons wait or are refused while KH2's move can't take one. |
| Debug views (the Insert key) | n/a | n/a | n/a | `pose::MODES` | none | 0 normal; 1 KH1 controls with KH2's motions on top; 2 KH1's own animation; 3 KH1's bind pose; 4 KH2's idle (0); 5 KH2's run (2). KH2's controller is on only in mode 0. |
| Replayed (interpolated) frames | n/a | n/a | n/a | KH1's model | `player_model` | KH1's model is drawn while KH1 replays a frame (`kh1-kh2-model.md`). |

## How animations are chosen

### Each frame: whose controller has him

`takeover::physics` runs on Sora's physics frame. `takeover::takeover` decides, in this order:

1. A ride (KH1's control comes and goes), an event, another body, or an area script walking him:
   **KH1**, KH1's own animation.
2. Base Sora with `shared = on`: **KH1's states**, unless KH2's controller is in one of its moves
   (`shared::kh2_move`). While KH1's states have him, `shared_frame` checks once a frame:
   - an ability borrow (`combomods::take_want`), taken even if KH1 isn't letting him act;
   - on a frame KH1 lets him act: Quick Run out of KH1's roll, an Attack for KH2's air combo, a
     command of KH2's, or the Square and air-jump borrows (`shared::control`).

   If one fires, **KH2's controller** starts fresh where he is. After each KH2 step,
   `shared::give_back` hands him back after two frames in idle / walk / run / landing on the
   ground, or in a plain fall.
3. One of KH1's own movement modes (state ≥ 29, 24, 25, flight 8), or a KH1 slot above 7 that
   isn't a leftover fall or a cut move's remainder: **KH1**.
4. Otherwise **KH2's controller** (Drive Forms, the old model, and borrows).

### Each frame: what is drawn

KH1's skeleton pass is hooked (`pose::pose`, `pose::pose_ik`).

1. **The model.** Only his 293-bone model standing at his actor is drawn by us; any other copy
   is left to KH1. Its slot is found by signature (`slot_of`).
2. **`pose::wanted`, mode 0.**
   - Scripted: KH1's own animation.
   - Else, if KH2's controller drew a pose this frame (or within 0.15 s) and no KH1 move is
     running (`stylecombo`, `styleroll`, an item, `shared::active`): KH2's pose, retargeted
     (`Retarget::pose_grip`).
   - Else `follow`.
3. **`pose::follow`.**
   - With Misc Animations at KH2 during an item's use: KH2's motion 9.
   - Old-path KH1 combo (`stylecombo::active`) on slots 98..104: KH1's own swing.
   - In Wisdom Form, or a base KH1 spell drawn KH2's way: 62 / 64 / 65 / 67.
   - Otherwise the style lookup: `shared::kh2_look(slot)` is true in a Drive Form. Else it is
     `styles::kh2(slot_kind(slot))`: idle / walk / run → Misc Animations; 8..13 → Jump; 98..101 →
     Base Combo; 102..104 → Aerial Base Combo; 75..78 → Glide; anything else → no style
     (KH1's own).
   - At KH1, or with no style: KH1's own animation.
   - At KH2: `follow_play(play_for(slot))`.
4. **`follow_play`.**
   - The rig is the form's if it has that motion, else base Sora's.
   - The clip is the field variant if asked for and present, else the battle one.
   - Swings are stretched to KH1's slot length and lose their root travel. Loops keep their place
     and change rate with his speed.
5. **After the pose.** In order: the feet (`feet::place`), KH1's wrist for `flip_weapons`, the
   Keyblade turned teeth up, hair, hood and keychain (KH2's own, else the swing), dream weapons
   held rigidly, Final's floating blade. Then the bones are written over KH1's.
6. **Player Model KH2** (`model`). KH2's own pose is used when there is one. Otherwise KH1's pose
   is carried onto KH2's skeleton (`kh1_data::inverse`).

### Blending

- **Between KH2 motions drawn on KH1's slots** (`follow_play`): 8/60 s between movement
  motions, 16/60 s into the glide, none into a swing. Same-group slots and a looping motion
  that doesn't change keep the clip. A loop's rate eases toward his speed (15% a frame).
- **From our pose to KH1's own animation:** 0.2 s, eased out (`KH1_BLEND`), from the last pose
  drawn. That covers going into a ladder, a spell, KH1's roll or KH1's combo.
- **KH2's controller** (`kh2_rig::blend`, from the exe), in 60 Hz frames:
  - 8 for idle / walk / run changes and requested motions (jump, roll, the fall after a jump);
  - 16 into a glide;
  - 0 for the motion queued after an action (idle), and for attacks (PTYA `blend_time`, 0 for
    every Sora row);
  - walk ↔ run keeps the gait's phase;
  - an action into walk or run blends from idle's first pose.
- **KH1's own blends** between its motions stay KH1's. They used to mix our last pose back in
  (Sora vanished for a moment), so the skeleton pass's own world matrix is used once it has
  agreed with the bones for 30 frames.

### KH1 motions drawn when KH2 has none

Every KH1 slot without a `play_for` entry plays KH1's own animation, at either style:
- magic casts and air casts (except Gravity / Stop / Aero, and Wisdom's KH1 spells, drawn KH2's
  way);
- KH1's reactions, guard and roll;
- ledges, vines, ladders, poles, bars, swimming, flight, the carpet and the Slider;
- specials, summons, field actions, cutscenes and scripted walks;
- KH1's combo modifiers.

With Player Model KH2, those KH1 poses are still shown on KH2's model through the inverse
retarget.

### Missing-motion fallbacks

- **KH2 Sora didn't load,** or KH1's motion table didn't (`pose::load_kh2`): KH1's own animation
  everywhere.
- **Slot unknown** (its signature not in KH1's table, as for another moveset like the Dream
  Shield's): KH1's own.
- **Clip lookup** in `follow_play`:
  1. the field variant;
  2. the battle motion;
  3. for a banked glide, the plain glide 205;
  4. idle 0.

  A Drive Form's rig falls back to base Sora's rig for a motion it lacks.
- **Inside a motion set** (`Mset::slot_for`): each motion falls back across its four variants in
  the game's documented order.
- **Keyblade** (`pose::kh2_weapon`): the weapon's motion of the same id, else its motion 0, else
  its bind pose.
- **KH2's controller** (`RigAnim::update`): a motion the rig lacks keeps the motion that was
  playing.
- **Cast motions:**
  - Reflect: 71 / 72 / 73 without a MAGC row;
  - base and Wisdom spells: 56 / 58 when the row has no motion;
  - KH1-only spells: 62 / 64 / 65 / 67.
  - A spell whose file or MAGC row is missing is KH1's to cast.
- **Quick Run check:** frame 16 if KH2's roll clip has no Quick Run check event.
- **Form abilities:** Guard and Aerial Recovery only where the form has motions 42 / 40. A
  repelled motion (41 / 43) only where the moveset has one.

## Gaps (unconfirmed)

- KH1 slots for: KH1's roll (its request is 0xDC), its guard, ground casts, reactions, ledges,
  vines, ladders and poles, swimming, specials, summons, the combo modifiers. The code doesn't
  name them.
- Slot 100 (assumed a third ground swing), slots 4 and 7 (assumed battle walk / run), glide slots
  76 and 78, and which way each banked glide faces. None of them seen in a log.
- Slot 80 as the air cast comes from a comment quoting one log, not from the code. Slot 58 for
  the item's use and slots 72, 75 and 77 come from the s261 log.
- Which of KH1's states 24, 25, 29, 30 and 31 is the carpet, the Jungle Slider or the water.
- Whether KH1 request ids equal slot numbers outside the glide.
- KH2's plain combo motions when KH2's controller runs its own combo (air combo at KH2, a ground
  combo inside a borrow): these come from PTYA rows read from the install. The only ones the code
  names are 151 / 152 / 154 (ground) and 181 / 183 / 184 (air), all in `pose::play_for`, and the
  ability moves.
- Base Sora's Fire / Blizzard / Thunder / Cure cast motion ids: read from the MAGC rows; the code
  names only fallbacks.
- Valor's and Final's own motion ids.
- Whether the hook ever plays KH2's special reaction 23 / 24 (it sends only flinches and blows).
- Inconsistencies in the code's comments: row 37 / ability 95 / motion 167 is called "Upper
  Slash" in `pose.rs` and `form.rs`; it is Finishing Leap everywhere else. And the first-run ini
  versus the "KH1 by default" comments (settings above).
- `aerial-recovery.md`'s "Differences in our port" says Aerial Recovery plays 202. Today's
  `character-core` names motion 40 (`motion::AIR_RECOVERY`). That section was not re-checked
  here.
