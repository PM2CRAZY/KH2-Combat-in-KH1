# KH1 port: hair, hood, sleeves and keychain under KH2's moves

When KH2's moves pose KH1 Sora, his hair spikes, hood and Keyblade chain now move the way KH2
animates its own hair, hood and chain in that same move. KH1's own animations are not touched:
KH1 draws those itself, and this code only runs on poses KH2's motions made.

Code: `crates/kh1-data/src/secondary.rs` (the matching and the posing),
`crates/kh1-hook/src/win/pose.rs` (where it runs). Check: `crates/kh1-data/examples/secondary_check.rs`,
`crates/kh1-data/tests/local_secondary.rs`.

## Before

The retarget (`retarget.rs`) turns KH1's body to match KH2's. It has no part for the hair, the
hood or the chain, so those just turned with the head, chest or hand. A physics swing
(`jiggle.rs`: gravity, a breeze, springs back to the pose) then moved them. It was ours, not
either game's. KH2 does animate these parts by hand in every motion; that animation was
thrown away.

## Which bones match

Both skeletons were compared in their bind poses (both are T-poses facing +z with the same
axes). We also looked at which bones each game's motions actually turn.

| Part | KH1 bones that move | KH2 bones that move | Evidence |
|---|---|---|---|
| Hair | 14 spikes under the head (151): root `r`, a joint `r+1` at the same place, a tip `r+2`, for `r` = 152, 155, ..., 191. KH1's motions turn only the joint. | 11 spikes under the head (141): a root and a leaf at the same place (142/143, 144/145, ..., 162/163). KH2's motions turn only the leaf. | KH1's own motions turn 153, 156, ..., 192 by up to 173°. KH2's turn 143, 145, ..., 163 by up to 180°. KH2's spikes sit within 20° of their bind in the idle. |
| Hood | 80, 81, 82, 83 (links to 84), hanging from the neck base 78 | 72 (link to 73) and 73 (a leaf), hanging from the chest 68 through 71 | KH1's motions turn 80 to 83. KH2's turn 72 and 73. KH2's 72 → 73 points down the back in its idle. |
| Keychain | 283 (at the pommel) to 291 (the charm; its tip is 292) | the Keyblade `W_EX010`, bones 3 to 11 | The two binds put the links at the same places, at most 0.56 apart: 3 ↔ 283, 4 ↔ 284, ..., 11 ↔ 291. KH1's motions turn 283 to 291. KH2's weapon motions turn 3, 5, 7, 9 and 11. |

**Hair.** The roots of each head's spikes are brought to one size: centred, and each axis
scaled by its spread. Each KH1 spike then follows its nearest KH2 spike, blended toward the
second nearest by inverse squared distance. That way two neighbouring KH1 spikes that share a
nearest KH2 spike still move a little differently. Found from the files:

| KH1 spike (joint) | Nearest KH2 | Second (share) |
|---|---|---|
| 153 back | 163 back | 147 (12%) |
| 156 top front | 155 top | 159 (48%) |
| 159 left low | 149 | 153 (50%) |
| 162 left back | 153 | 163 (14%) |
| 165 left side | 149 | 151 (9%) |
| 168 left front | 151 | 149 (4%) |
| 171 left top front | 159 | 151 (48%) |
| 174 right low | 147 | 143 (40%) |
| 177 right back | 147 | 163 (14%) |
| 180 right side | 147 | 145 (49%) |
| 183 right front | 145 | 157 (16%) |
| 186 front | 159 | 145 (20%) |
| 189 front fringe | 159 | 157 (20%) |
| 192 top | 161 | 155 (46%) |

KH2 has fewer spikes on the sides than KH1, so 147 leads three KH1 spikes. This match is a
judgement from positions only. Neither game says which spike is which.

**Hood.** KH1's hood runs 4.3, 7.9, 12.3 and 16 back from the neck base. KH2's runs 5 and
12.8 back from its chest, about 0.89 of that on KH1. So 80 and 81 follow 72, and 82 and 83
follow 73.

KH2's chest (68) and head (141) carry no extra built-in turn in their motions: their turn from
the bind matches the retarget's position-based chest and head frames to under 1°. So turns
measured against them are sound.

## How a KH1 link is posed

This runs every frame KH2 poses him, after the retarget, the feet, KH1's wrist (for
`flip_weapons`) and the Keyblade's teeth-up turn.

- **Hair and hood: the same turn, relative to the head or chest.** KH2's spikes are single
  bones with no child, so their direction can't be read from positions. So we take the KH2
  bone's turn away from its bind, with the head's own turn taken out (the chest's for the
  hood). That is what the spike does relative to the head. The same turn goes on the KH1 joint
  relative to KH1's head as it is posed now. Twist included, it becomes the joint's turn from
  its own bind. KH1 keeps its own hair style (its spikes point where KH1's point at rest) and
  moves the way KH2's moves.
- **Keychain: the same direction, from KH1's pommel.** The two chains are built alike, so each
  KH1 link points along its KH2 link in model space. KH1's Keyblade sits in KH1's grip, a
  little off KH2's. So the directions are first turned by the swing from KH2's handle (grip →
  pommel, bones 0 → 3) to KH1's (280 → 283). The chain then leaves the pommel as KH2's does.
  KH2's last link, the charm (11), has no child. Its direction is the chain's bind direction,
  turned as that bone turned.
- Each KH1 bone turns about its own joint, carrying everything under it. Every length stays
  KH1's.

## Guards

- **Hair into the head.** A spike may turn at most 150° from its bind relative to the head.
  KH1's own motions turn them up to 173°. Its middle and tip must also stay at least 0.9 of
  their bind distance from the head's centre (the middle of the spike roots). Past that, its
  turn is cut back just far enough. Over all 83 KH2 motions this happens on a few frames of
  big swings, such as the finisher 154.
- **Hood into the back or the head.** The hood's joints keep at least 7 off the spine line
  (71 → 78). That is how far KH1's own rest pose holds them, which was found clear of the back.
  They also keep 10 from the head's centre. KH2's idle lays its hood closer, about 5.8 on KH1.
  When a joint comes too near, the whole hood swings about its root just far enough away,
  keeping KH2's bends. In the idle that is about 15° of push; the hood still sways with KH2's
  idle.
- **Bad data.** Non-finite poses or turns are skipped. A part whose KH2 bones are missing
  stays as the retarget left it.

## Switches

- `hair_physics = off` (an older ini's `physics = off`): none of this, and no swing either. The parts turn with their parent as
  before.
- `kh2_hair = off`: the older physics swing (`jiggle.rs`) instead of KH2's animation.
- The swing still runs for any chain KH2 didn't pose this frame. That is only the keychain
  while a dream weapon or the wooden sword is held (`0x52..=0x55`): those have no Keyblade
  chain, and KH2's chain motion would swing whatever KH1 hangs on those bones.

## What the checks show

From `secondary_check` (KH1's battle idle as the rest pose, as in the game) on idle 0, run 2,
jump 3, attacks 151 and 154, air attack 181 and High Jump 201:

- no NaN, and no bone length changed (worst 0.0002);
- each part moves within each motion, measured in its reference bone's frame:
  - hair: 25° in the idle, up to 91°;
  - hood: 7° in the idle, up to 128°;
  - chain: 77–177°;
- the chain's links agree with KH2's to 0.0° in every frame;
- the hair and hood agree to 0.0° except where a guard held them back;
- stick figures of these frames put the hair on the head, the hood on the back (flying back
  in the run, up in the spins, as KH2's does) and the chain hanging from the pommel in the
  shape of KH2's.

Across all 83 motions (1994 frames), no NaN appeared. Only one sample turned a spike more than
120° from its bind.

Not checked in game yet.

## Sleeves

Tester request (Mr Bugs, "Missing sleeves animation"): under KH2's moves Sora's sleeves didn't
move. His two runs (t371, KH1 model, Misc Animations KH2 with KH2's run, motion 2, then KH1 with
KH1's run, slot 2) show the same run; neither trace logs bones, so the cause was found in the
files.

**Why they were still.** KH1's sleeve bones are outside every retarget part and not in
`secondary`, so they kept their bind relation to the upper arm: a rigid tube. Both games animate
the sleeves by hand. Checked (files):

| | KH1 (`xa_ex_0010`) | KH2 (`P_EX100`) |
|---|---|---|
| Bones | Four chains per arm from the upper arm (93 / 198): front 128 / 232, under 132 / 236, back 136 / 240, top 140 / 244. Each: root, a joint at the same place, one 8.7 down the arm, a tip about 15 down. | Per arm, from the upper arm (78 / 170): 109 (at the elbow) → 110 (back at the shoulder) → the sleeve root 111 / 203 → two flaps: front 112 → 113 (204 → 205), back 114 → 115 (206 → 207), 2.5 out from the shoulder. |
| What the motions turn | The joint at the shoulder and the one down the arm, in 59-93 of KH1's motions, up to 89°. Its run: 4-10°. | 110 / 202 (the whole sleeve about the shoulder) in 53-56 motions, and the flap leaves 113, 115, 205, 207 in 76-83. KH2's run: 110 11°, 113 33°, 115 17° (-x side 3-13°). 109, 111, 112, 114 are never keyed. |
| Constraints or expressions on them | - | None in any of the 89 motions: KH2 keys its sleeves, no cloth or physics drives them. |

Not checked in the KH2 exe: that nothing at run time (outside the motion data) moves the
sleeves. Assumption, as for the hair and hood; the keys alone already move them.

**How KH1's sleeve is posed now** (`secondary.rs`, `Parts::sleeves`, every frame KH2 poses him
and in the bake, after the retarget, with the hair and hood):

- The joint at the shoulder (129, 133, ... / 233, 237, ...) takes KH2's sleeve root's (111 / 203)
  turn from its bind, in model space, carried by the swing from KH2's upper arm direction
  (78 → 79) onto KH1's (198 → 199). KH1's upper arm already points along KH2's, so this swing is
  about none. It is model space and not relative to the upper arm because KH2's sleeve root
  turns about the arm's own axis against its twist (up to 178°, motions 8 and 181; Checked
  (files): the turn's axis lies along the arm) and KH1's upper arm, posed from
  positions, twists differently from KH2's 78 (7-11° on average, at worst about 170°): measured
  against KH1's arm, the sleeve would be twisted round with that difference.
- The joint down the sleeve (130, 134, ... / 234, 238, ...) takes the flap's turn relative to
  KH2's root, on KH1's root as just posed.
- Flaps: KH2 has two, KH1 four chains. Each chain blends KH2's two flaps by where its root and
  the flap pivots sit round the arm in the binds (inverse squared distance, as the hair): front
  98% the front flap, back 98% the back flap, under and top about half each. A judgement from
  positions; neither game says which is which.
- Lengths stay KH1's. No guard was needed: over all 89 KH2 motions (3272 frames, both arms) the
  sleeve's tube stays within 29° of KH1's upper arm; KH1's own motions take it to 41° and the
  bind holds it at 11°.

Not carried: KH2's scale keys on the flaps (113 in 3 motions down to 0.70, 115 in 1, 205 in 8,
207's 0.9 in its initial pose). The retarget and the bake carry turns only. Checked (files).

**Switches.** As the hair and hood: `hair_physics = off` or `kh2_hair = off`, and the sleeves
keep their old rigid relation to the arm (there is no swing for them).

**Checks** (`kh1-data` unit tests `sleeves_match_their_flaps_round_the_arm`,
`a_sleeve_hangs_and_flaps_as_kh2_s`; on the files, a scratch run of every KH2 motion): no NaN,
no length change (worst 0.0001), each link within 0.03° of the direction KH2's bones give, KH2's
run swinging the sleeves 2-8° against the arm. Baking KH2's run adds 48 keyed channels (16 bones,
three turns each) with the bake's worst joint error unchanged (0.23, bone 155). Not checked in
game.

## Other parts KH1 animates that still don't move under KH2's moves

Found with the sleeves (bones KH1's own motions turn in 5 or more motions that no retarget part,
`secondary` part or KH2 match drives). Checked (files); what each is from bone positions only
(Assumption).

| KH1 bones | What (Assumption) | KH1 motions | Now | KH2's counterpart (keyed) |
|---|---|---|---|---|
| 27-28, 31-32, 35-36, 39-40 (-x); 53-54, 57-58, 61-62, 65-66 (+x) | the shorts' four panels per leg, from the thigh | 51-81, up to 97° | rigid on the thigh | the trouser panels under each thigh (27, 29, 32, 34; 56, 58, 61, 63), 38-82 motions, run up to 112° |
| 249-250 ... 277-278, 73-74 | the jacket's hem panels from the spine base | 76-105, up to 163° | rigid, pushed off the thighs (`keep_clear`) | cloth from the pelvis (5, 7, 9; 36, 38), 35-78 motions |
| 11-14 | the belt chain | 106-111, up to 157° | carried by the +x thigh | none found |
| 5, 8 | belt-front pieces | 22-66 | rigid | none found |
| 86-87 | the necklace | 97, up to 115° | rigid | none found by position (KH2's chest pieces 116-135 / 208-227 and 164-165 have no KH1 bones) |

Also not carried: KH2's scale keys (its trouser and pelvis cloth 7, 9, 32, 36, 38, 61 in 18-72
motions, 0.29-3.7; KH1's own motions key scale on its shorts panels too), and, the other way
round, KH1's own animation of all these parts when KH2's model is drawn (`inverse` keeps KH2's
cloth, hair and hood at its bind relation).

## Back to t375 (tA)

The tester ("Missing sleeves animation", after t613-t616): "revert changes made in this thread to
t375 ... and bake the animations for me. we'll figure out a way later, keep the debug menu frame
and frame count changes". Everything above is t375's state again, live and in the bake: KH2's
sleeve flaps (four KH1 chains from KH2's two flaps), the hair, hood and keychain from KH2's keys,
the switches `hair_physics` / `kh2_hair` and the old swing (`jiggle.rs`) for chains KH2 doesn't
pose. Taken out with it: the necklace match (t385), KH2's squash and stretch (t386), KH1's own
animation on its own parts and its blend-pass fixes (t387, t405, t406, t411), "only what KH2's
model shows" (t407, t412), the coat and shorts rules (t453, t490, t491, t519, t552, the research
tool t500), the thighs' and shins' roll from KH2's skinned leg bones (t499, a body retarget change
made in this thread), the test bake of the hood, coat and shorts from KH2's hidden keys (t569),
and the live cloth physics with its trace category and debug page (t592, t595, t596, t613, t615,
t616). Kept: KH2's walk and run baked into KH1's slots (t389; other threads' slot work builds on
it), the Frame Advance info panel (t593, its cloth line removed), the Trace Recorder's Frames row
(t594) and the panel's "Animation" line (t614).

The bake (`pose::Baker`, `kh2slots`, ini `kh1_bake = on`) is made at load from the player's own
KH1 and KH2 files, so every baked slot now carries t375's sleeves, hair, hood and keychain with no
other step: nothing game-derived is stored in the repo.

The research below (how KH2's exe animates Sora, read in the Steam decomp after t453) is kept as a
finding; it changed no behaviour.

## How KH2's exe animates Sora (Steam decomp, research after t453)

The tester, after t453: "investigate into the kh2 steam c decomp how exactly sora's animations work
(the hair, cloth, coat, etc) do not make assumptions". Read in the Steam KH2 decomp (the tester's exe,
SHA-256 `9002b2de...49ed`) and checked against `P_EX100.mdlx` / `P_EX100.mset`. Class names come
from the exe's run-time type names (`kn::Motion`, `kn::MotionPrototype0`, `kn::MotionRAW`,
`kn::ModelSKL`, `kn::AnimationWork`); there is no class with a cloth, spring, hair or physics name.

### The per-frame path (Checked (decomp))

1. **Object loop** `0x1403BEEC0` (also `0x1403BFAB0`): per object, `0x1403CCB90` builds the object's
   world matrix (position, facing), then `0x1403C6AA0` updates its motion, then (objects of kind
   0x65 only) `0x14040A380` places collision parts. Nothing after the motion update in this loop
   writes bones.
2. **Motion update** `0x1403C6AA0`, on the model at object `+0x7B0`:
   - collects the **ground-adaptation entries** (`0x1403C8070`): the model's collision entries of
     kind 10 and 11 (`0x1403B4F10` / `0x1403C9B80`: 20-byte entries, kind at +4, joint at +6, a
     height at +0x12), only when the object's flags allow it (object `+0x168` bit 0 and two more
     checks in `0x1403C8070`, not read further). Sora's
     `P_EX100.mdlx` (its `p_ex` entry, 36 entries) has two of kind 10, on joints 12 and 41 (the
     legs), height 15, and none of kind 11 (Checked (files)). So this is the feet only;
   - reads the **look-at target** at object `+0x5D0` (`0x1403E4E60`: a position, a joint number and a
     weight eased as (1 − cos(πt)) / 2). It is passed into the motion evaluation; where the joint
     pass applies it was not found (the parameter looks unused in the decomp). Not a cloth part;
   - then either plays one motion (`0x1401A4D40` → the motion's evaluate, below) or, while a
     blend runs (object `+0x44` / `+0x48`, the clock over the length), blends two
     (`0x1401A4CD0` → `0x1401C3A30`).
3. **One motion**, `kn::MotionPrototype0` (vtable `0x1405B4958`), evaluate `0x1401D2C80`:
   - on a new motion: each bone's local scale / rotation / translation reset to the model's bind,
     then the motion's initial pose written over it;
   - the F-curves sampled at the time (`0x1401DAD10` → `0x1401D30C0`; the ±3.14 wrap guard);
   - each constraint switched on or off by its activation table at that time;
   - the joint pass `0x1401D3B80`: per joint in the motion's joint order, expressions
     (`0x1401D9950`), constraints (kinds 0..13), limiters (`0x1401D36F0`: a soft clamp of a position
     or rotation, sphere or box, with an exponential edge; no time in it), IK (`0x1401C57B0`), the
     ground adaptation for IK ends (a line test against the map's collision, `0x1401712B0`: two
     collision queries with mask 0x200; also in the blend), then the world
     matrices;
   - a motion whose header word +4 is 1 takes `0x1401D60F0` instead; all 89 of Sora's are 0
     (Checked (files)).
4. **Blend** `0x1401C3A30`: per bone, the two motions' local channels lerped by the blend weight,
   rotations along the short way (the difference wrapped to ±π), then the IK chains re-solved with the
   ground adaptation (`0x1401C5190` re-places the bones under them).
5. **No state between frames.** The model keeps two matrix buffers (`+0x50` / `+0x58`, swapped each
   update by `0x1401A4D40` / `0x1401A4CD0`); neither the joint pass nor the blend reads the previous
   one, and their work buffers are allocated and zeroed each call. Game code reads bone matrices
   through copies (`0x1401A7950`, `0x1401A7AF0`); no other function found writing them. So a bone's
   pose is a function of the motion(s), the time, the blend weight, the IK and ground targets only:
   nothing swings, lags or settles on its own.

### What drives each of Sora's parts (Checked (files), all 89 motions of `P_EX100.mset`)

- The motions address 228 bones (0..227); the model's table lists 256, bones 228-255 sitting at the
  root with no length (Checked (files)). Joints from 228 up in a motion are its IK helpers.
- **Constraints** hold only these model bones: the root 0, the legs (10, 13, 14, 17, 18, 39, 42, 43,
  46, 47), the spine and chest (66, 67, 68), the arms and hands (77, 80, 85, 86, 169, 172, 177, 178),
  the head 140 (an aim at helpers), and in one motion each the pelvis pieces 7, 9 and the trouser
  bones 27, 29, 36, 38, 56, 58. **Expressions** write only IK helpers. **IK chain flags** sit only on
  the legs (11-13, 40-42), spine (65-68), arms (78-80, 170-172) and hands (85, 86, 177, 178).
- So the hair (141-163), the hood (71-73), the sleeves and flaps (109-115, 201-207), the necklace
  (164-165) and the coat ring (70, 128-135, 220-227) move by their F-curve keys and their parents only.
- **Which bones show** (the earlier table, checked again with a bind-pose pass over every vertex):
  each vertex is on one bone; the 102 vertex packets are all of type 1, the third row of every
  packet header is all zeros, and each packet's vertex count equals the sum of its per-bone counts.
  The mesh is on the body, the hair spikes 145-157 (odd) and 163, the sleeves 110 / 202 (whole), the
  necklace 165 and the head 140 (2614 corners: face, hair, back of the hair). The torso 67 carries the
  jacket down to about 104 high (the pelvis is at 102.6); the coat ring, the hood bones, the flaps,
  the spikes 143 / 159 / 161 and the trouser and pelvis pieces carry nothing. The model has one
  sub-model (no second model chained after it).
- **How the hidden bones move** (a scratch run of the battle and field idle, walk and run, angle
  against the bind relation to the parent part): the coat ring swings up to 51° within a cycle (the
  field idle under 5°), its median 1-62° off its bind; the hood 73's median is 61-108° off its bind;
  the flap 113 swings 5-40°; the hidden spike 159 moves like the shown ones
  (up to 36°); the trouser bone 27 swings up to 113° in the battle walk. Shown parts for comparison:
  spikes 145 / 147 up to 45°, the sleeve 110 up to 28°, the necklace up to 55°.
- **Not known:** why KH2 keys bones its model doesn't show. A guess to check, not a finding: other
  Sora models on the same skeleton (the world outfits or the forms) could carry mesh there. How to
  find out: read those models' vertex packets the same way.
- **KH1's blend, for comparison** (`0x140392560`, Checked (decomp) in part): it lerps two stored
  snapshots per bone (`0x140386B70`, and `0x1400E64D0`, a component-wise lerp of three floats that
  keeps the fourth). Not yet read: whether those floats are Euler angles and whether KH1 takes the
  short way round as KH2 does. It matters on the frames a blend runs, for parts that turn far (hair up
  to 180°).
