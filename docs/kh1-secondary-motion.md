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
(`jiggle.rs`: gravity, a breeze, springs back to the pose; removed in t388) then moved them. It
was ours, not either game's. KH2 does animate these parts by hand in every motion; that animation was
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

## No switches, no physics (t388)

The tester (point 6 of "Missing sleeves animation"): "kh2 does not have physics but is instead
baked so remove those switches out of the code, they're unneeded." Removed: the ini's
`hair_physics` and `kh2_hair`, their `Settings` fields, and our physics swing (`jiggle.rs`:
gravity, a breeze, springs) with them. KH2's animation of these parts now always runs on KH2's
poses, live and in the bake. An older ini's `hair_physics` / `kh2_hair` lines are ignored, and
its `physics = on / off` (the hair's switch before) still reads as the physics setting's KH2.

Checked that KH2 has no run-time physics on them:
- the OG Rust rewrite (KH2's source of truth here) has none: its evaluator plays the motions'
  keys (and their constraints and IK), and nothing else moves hair, hood, sleeves or cloth
  (Checked: no cloth, spring or swing code outside the camera);
- in KH2's own files: no constraint or expression in any of Sora's 83 motions targets the hair,
  the hood, the sleeves, the necklace or the hem ring (they constrain only body joints and IK
  helpers; the trouser and pelvis cloth bones 7, 9, 27, 29, 36, 38, 56, 58 in one motion each);
  and the hood, the flaps and the cloth have no vertices at all ("Which KH2 bones show on KH2's
  model"), so a simulation on them would show nothing. Checked (files);
- the KH2 Steam decomp has no named cloth or physics strings (Checked (decomp), strings only:
  the exe is stripped, so this is weak on its own). Since then the whole per-frame path from
  the object update to the bone matrices has been read in the Steam decomp ("How KH2's exe
  animates Sora" at the end of this file): no physics, spring or cloth step on it.

The keychain while a dream weapon or the wooden sword is held (`0x52..=0x55`; KH2's chain motion
would swing whatever KH1 hangs on those bones) used to get the swing; it now keeps KH1's rest
relation (the battle idle's), with the weapon held whole (`dream_hold`). KH1's own animation
(KH1's style, or anything KH1 shows itself) never ran any of this and is unchanged.

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

KH2's scale keys on the flaps (113 in 3 motions down to 0.70, 115 in 1, 205 in 8, 207's 0.9 in
its initial pose) are carried since t386 ("Squash and stretch" below). Checked (files).

**Switches.** None since t388 (as the hair and hood).

**Checks** (`kh1-data` unit tests `sleeves_match_their_flaps_round_the_arm`,
`a_sleeve_hangs_and_flaps_as_kh2_s`; on the files, a scratch run of every KH2 motion): no NaN,
no length change (worst 0.0001), each link within 0.03° of the direction KH2's bones give, KH2's
run swinging the sleeves 2-8° against the arm. Baking KH2's run adds 48 keyed channels (16 bones,
three turns each) with the bake's worst joint error unchanged (0.23, bone 155). Not checked in
game.

## Necklace (t385)

KH1's necklace (85 at the front of the neck base, the link 86, the pendant 87, its tip 88)
now follows KH2's necklace. Checked (files):

- KH2's necklace is one bone, 165 (a leaf under 164, from the chest 68 at its middle, 126.3
  high). Its mesh is on it alone: 246 vertices from 114 to 123 high, 2 to 12 in front of the
  chest, the crown pendant. KH2's motions turn it in 79 of 83, up to 114° from its bind
  (median 31°).
- Both bind pointing down and forward: KH1's link 50° below level, KH2's bone's x axis 56°.
  No constraint or expression in any KH2 motion touches it.

How (`secondary.rs`, `Parts::necklace`, live and in the bake, with the hood): KH1's link 86
takes 165's turn from its bind relative to KH2's chest, on KH1's neck base (78) as posed now
(the hood's method). The pendant 87 keeps its relation to 86 (KH2 has no second bone for it).

Guard: KH2's necklace hangs from the middle of its chest, KH1's from the front of its neck, so
a swing back that clears KH2's chest can reach into KH1's. The pendant and tip keep 0.85 of
their bind distance from the spine (71 → 78), the link swinging about its joint just far
enough. Over all 83 KH2 motions (5991 frames) this held it on 3 frames (motion 165); no NaN,
lengths kept (worst 0.0001). KH1's link then turns 31° from its bind relative to the chest in
the median frame, 77° at the 95th percentile, 114° at most; KH1's own motions turn it 6°, 58°
and 105°. Not checked in game.

## Which KH2 bones show on KH2's model (t385)

KH2 Sora's model (`P_EX100.mdlx`) puts each vertex on one bone, with no blending between
bones (every vertex packet's blend count is 0). Only these bones carry vertices: 2, 12, 15,
18, 24, 41, 44, 47, 53, 65, 66, 67, 75, 78, 79, 82, 85, 88-108 (fingers), 110, 137, 140, 145,
147, 149, 151, 153, 155, 157, 163, 165, 167, 170, 171, 174, 177, 180-200 (fingers), 202.
Checked (files). Assumption: the Steam game draws this same model (it is the one the mod's
KH2-model option draws).

So many bones KH2's motions key move nothing you can see in KH2:

- the hood bones 72 and 73 (KH2's model has no hood standing off the back: what it shows of
  one is the top of the back, on the torso 67; see "KH2's hood" below);
- the sleeve flaps 113, 115, 205, 207 (each sleeve's mesh is on 110 / 202 alone);
- the hair spikes 143, 159 and 161 (the front and top of KH2's hair are on the head);
- the cloth ring round the lower chest, 129, 131, 133, 135 and 221, 223, 225, 227, and the back
  piece 70: the same layout as KH1's jacket hem (four a side and one at the back, each bone's
  bind axis within 7° of KH1's 249, 253, 257, 261 and 73);
- the trouser bones 27, 29, 32, 34, 20, 22 (and the other leg's), and the pelvis pieces 5, 7,
  9, 36, 38;
- every scale key KH2's motions have (on 7, 9, 32, 36, 38, 61, 113, 115, 120, 127, 129, 131,
  205, 212, 219): all on bones with no vertices.

These keyed-but-hidden bones also sit far from their bind in KH2's motions (the front of the
chest ring 59° in the median frame, the trouser bone 27 twisted 43°), where KH1's own hem and
shorts sit 0-17° from theirs in the median frame: copied onto KH1's visible hem and shorts they would hold the hem flared out
and the shorts' panels twisted. They are not copied (`docs/tester-requests.md`, "Missing
sleeves animation", the tester's rule: nothing KH1 has no visible KH2 match for).

### Only what KH2's model shows

The tester's answer to "keep copying the hidden bones, or follow only what KH2 visibly moves":
"(b) only do what kh2 does so its 1:1". `secondary::KH2_SHOWN_ON`: each keyed KH2 bone with no
mesh is read as held to the bone its region's mesh is on, at their bind relation, since that is
how KH2 draws it. The hood 72 / 73 on the torso 67 (t407 had them on the head 140; see "KH2's
hood"), spikes 143 / 159 / 161 on the head 140, the flaps 113 / 115 on the sleeve 110, 205 / 207
on 202. So, live and baked:

- KH1's hood (80-83) keeps its bind relation to KH1's neck base, as KH2's hood keeps its place
  on KH2's torso (its guards stay: in High Jump, 201, the head comes back onto it for 2 frames,
  9° of push);
- KH1's sleeves turn as KH2's sleeve (110 / 202, through the unkeyed root 111 / 203) and no
  more: the joints down each sleeve keep their bind relation to the shoulder joint, with no
  flap turn and no flap squash;
- the KH1 spikes matched to 143, 159 or 161 stay with the head for that share (171, 186, 189,
  192 mostly, 156 and 174 in part); the rest follow their visible KH2 spike as before.

KH2's parents (Checked (files)): 72 → 71 (under the chest); 141 → 140, each spike root under
141; 113 → 112 → 111 → 110, 205 → 204, 207 → 206 → 203 → 202. Over all of KH2's motions (5908
frames, a scratch run) no NaN, and no scale is left for `scales` to carry (worst 1.0): the flaps
were the only matched KH2 bones with scale keys.

### KH2's hood (after t409)

The tester, on t407: "the hoodie doesn't seem to be animated anymore". t407 had read KH2's hood
as on the head 140. Checked again in the files (`P_EX100.mdlx`, a scratch render of the bind
mesh coloured by texture and by bone):

- nothing stands off KH2's back as a hood: behind the neck (y 118-132, z below -6) the only mesh
  is on the head 140 (42 triangles, the colour of the hair: it is the back of the hair hanging
  to the nape) and on the torso 67 (36 triangles, the top of the back between the shoulders,
  coloured apart from the rest of the torso: what KH2 shows of its hood);
- 67 carries the whole upper body; 68 above it (the chest these parts are measured against)
  carries no mesh and no KH2 motion turns it off 67 (idle, walk, run, jump, 151, 152, 154, 181,
  battle and field: 0.0°);
- so KH2's hood never moves on its own or with the head: it goes with the torso. The head 140
  does move against the torso (field run 45° of swing, battle run 17°), which is what t407 put on
  KH1's hood.

So the 1:1 hood is still on the chest. Measured (scratch run on KH2's 0, 1, 2, 3, 151, 154, 181,
201): KH1's hood stays within 0.04° of its bind relation to the neck base, except High Jump (the
guard, 9.5°, 2 frames); no NaN. For comparison KH1's own motions turn its hood root 13-31° in the
walk (slot 2) and up to 72° in the run (slot 5). Not checked in game. Assumption: the patch on
the top of the back is the hood (judged from its colour and place; the model has no other
candidate).

### Every KH2 part now copied onto KH1

| KH1 part | KH1 bones | KH2 bones | Shows on KH2's model |
|---|---|---|---|
| Hair (14 spikes, `Parts::hair`) | joints 153, 156, ..., 192 | nearest spike and second (share), table above: 163, 155, 149, 153, 151, 159, 147, 145, 157, 161; seconds 147, 159, 153, 163, 151, 149, 143, 145, 157, 155 | yes, except 143, 159, 161 (on the head), now read as the head (`KH2_SHOWN_ON`): KH1's 171, 186, 189 and 192 follow 159 or 161 for more than half their turn, 156 for 48%, 174 follows 143 for 40% |
| Hood (`Parts::hood`) | 80, 81 / 82, 83 | 72 / 73, relative to the chest 68, read as the torso 67 (`KH2_SHOWN_ON`) | no: KH2's hood is the top of its back, on the torso |
| Keychain (`Parts::chain`) | 283-291 | the Keyblade `W_EX010`'s 3-11 | not checked (no weapon model here) |
| Sleeves, the shoulder joints (`Parts::sleeves`) | 129, 133, 137, 141 / 233, 237, 241, 245 | 111 / 203 (turned by 110 / 202) | yes (the sleeve's mesh is on 110 / 202) |
| Sleeves, the joints down the sleeve | 130, 134, 138, 142 / 234, 238, 242, 246 | flaps 113, 115 / 205, 207, relative to the root, read as the sleeve 110 / 202 (`KH2_SHOWN_ON`): no turn of their own | no |
| Necklace (`Parts::necklace`, t385) | 86 | 165, relative to the chest 68 | yes |
| The body (`retarget.rs`) | root, spine, neck, head, arms, hands, fingers, legs, feet | KH2's body, measured from positions | yes |

Not copied (KH1's own bones, no visible KH2 match): the jacket hem (249-278, 73-74), the
shorts' panels (27-40, 53-66), the belt chain (11-14), the belt-front pieces (5, 8), the
necklace's pendant (87). Since t387 the shorts take KH1's own animation of the move ("KH1's own
parts" below); the hem (the coat), the belt chain, the belt-front pieces and the pendant did too
until the tester set them aside or asked for the coat as before, and hang as before t387 again.

## Squash and stretch (t386)

KH2's scale on the bones matched above now goes on their KH1 bones, live and in the bake (the
tester: "yes do it").

**How both games scale a bone** (the same): a bone's scale is its parent's times its own, and
it stretches that bone's axes only; where the bone and its children sit is unscaled.
- KH1, Checked (decomp): the bone pass `0x1403900B0` multiplies each bone's sampled scale by its
  parent's (`0x1401DB310`, every bone not skipped, static ones too), builds the place unscaled,
  then `0x140394CF0` multiplies the matrix's three axis rows by that scale. A motion's scale
  channels are kinds 1-3 (`docs/kh1-anim-slots.md` §1.6 step 3).
- KH2: the same in the OG Rust rewrite's evaluator (`kh2-anim`, `Pose::bind` / `evaluate`), which
  this repo's `kh2_anim` copies. Not read in the KH2 decomp.

**What goes where** (`Secondary::scales`, `secondary::apply_scale`): each matched KH1 bone takes
its KH2 bone's own scale (its scale over its parent's), on the first KH1 bone that KH2 bone turns
(its KH1 subtree carries it, as KH2's carries KH2's): a spike's joint (its two KH2 spikes
blended by their share), the hood's 80 (72) and 82 (73), each chain link (the Keyblade's 3-11),
a sleeve's shoulder joint (its root) and the joint down it (the two flaps blended), the
necklace's 86 (165). KH2's three scales are laid onto the KH1 bone's axes: each KH1 axis takes
them weighted by how much of it lies along each KH2 axis (squared, in the binds); the same scale
where the axes match (the sleeves' do).

- Live (`pose.rs`): after everything else, just before the bones are written, so the guards and
  the hand's adjustments work on the unscaled pose.
- Bake (`bake.rs`, `Baker::poses`): scale channels (kinds 1-3) for bones whose scale moves, a
  constant where it holds, nothing at 1; the poses given are unscaled. A bone whose only channel
  is a constant scale may still be built once (bit 25): KH1 applies the scale to static bones too.

**What this carries today: nothing.** Since KH2's hidden bones are read as held to their mesh
bone ("Only what KH2's model shows" above), the flaps' scale isn't carried, and no other matched
KH2 bone has scale keys; the pass stays for KH1's own scale on the hem and shorts. Before that
(Checked (files), all 83 KH2 motions, 5991 frames) it carried only the sleeves'
joints down the sleeve: 130, 134, 138, 142 (KH2's 207 held at 0.9 in 17 motions' initial pose,
KH1's 0.81-1.01) and 234, 238, 242, 246 (113 / 115, 0.59-1.14), in 43 motions. No other matched
KH2 bone has scale keys (the hair, hood, necklace: none; the Keyblade's chain: not checked, no
weapon file here). KH2's other scale keys are all on bones with no vertices that nothing of KH1
follows. Positions are unchanged by it (checked: no bone moved). Not checked in game.

## KH1's own parts (t387)

The tester (point 6 of the thread): "bake a custom animation for the small parts kh2 sora
doesn't have that kh1 has following the motion of each animation to give them similar looking
results to kh1's animations ... use kh1's animations as a reference."

**Set aside** (the tester, after t393: "for the custom animation parts for kh1s model lets set
that aside and work on it later, remove them from animation for now", with "(a)" to keeping KH1's
own animation on the hem and shorts): the belt chain (11-14), the belt-front pieces (5, 8) and the
necklace's pendant (87) are out of `KH1_OWN` and hang as before t387 (the chain carried by the +x
thigh, the others rigid on their parent), live and baked. The hem and shorts kept it until the
coat went back (below). Of the table below, only the shorts' row is still in use.

**The coat as before** (the tester, after t412: "before I prompted you to adjust the bake
animations the coat was working how it should", "when I mean model in this case I'm also referring
to kh2 sora's skeleton"): the jacket hem's panels (249-278) and back centre (73-74) are out of
`KH1_OWN` (16 bones left, the shorts). The coat is posed again exactly as from s284 to t386: each
panel's root (72, 248, 252, ..., 276) turns with KH2's pelvis (the front panels 25 / 15 / 5% with
the thigh as it rises), every joint below at its bind relation, and the chains swung off KH1's
thighs (`keep_clear`), live and baked. KH2's own skeleton hangs its coat ring (129-135 / 221-227
and the back piece 70, 10-13 long, at 109-111 high) from the chest 68, not the pelvis; those bones
carry no mesh in `P_EX100.mdlx` and sit far from their bind in KH2's motions (above), which is why
the coat follows KH2's pelvis and not them. Checked (files): over every KH2 motion in
`P_EX100.mset` (battle and field slots, 11906 frames, a scratch run), with KH1's own run fed to the
retarget as the bake does, the coat's 36 bones (72-75, 248-279) come out identical to t386's
retarget (worst 0 units); t412's differed on every frame, by up to 27.7. Why t387-t412 broke it:
the coat took its relation to its parent from KH1's own motion of the move (baked: KH1's
template; live: what KH1's pass just placed), made for KH1's body and strides, not KH2's, and live
that hung on which bones KH1's blend pass places (t405's stretch, t411's map-origin flash). The
coat no longer reads KH1's pass at all, so neither can reach it. Not checked in game.

**Which parts** (`retarget::KH1_OWN`, 16 bones now, 34 at t406, 41 at t387): those KH1's own motions turn in 5 or more
motions that nothing of KH2's drives. Checked (files); what each is from bone positions only
(Assumption). In every one of KH1's 118 motions their joint words are plain FK (0, or 25: built
once), so KH1's keys are the whole of their animation. Checked (files).

| KH1 bones | What (Assumption) | KH1 motions | Before t387 | KH2's counterpart |
|---|---|---|---|---|
| 27-28, 31-32, 35-36, 39-40 (-x); 53-54, 57-58, 61-62, 65-66 (+x) | the shorts' four panels per leg (two joints each), from the thigh | 51-90, up to 139° | rigid on the thigh | trouser bones under each thigh (27, 29, 32, 34; 56, 58, 61, 63), keyed but with no vertices |
| 249-250 ... 277-278 (two joints a panel), 73-74 | the jacket's hem panels from the spine, and its back centre | 74-101, up to 162° | rigid, turned with the hips, pushed off the thighs (`keep_clear`) | the chest ring 129-135 / 221-227 and 70, the same layout, keyed but with no vertices |
| 11-14 | the belt chain | 104-109, up to 223° | carried by the +x thigh | none |
| 5, 8 | belt-front pieces | 22 and 66 | rigid | none (KH2's pelvis pieces 4-9 have no vertices and no KH1 match) |
| 87 | the necklace's pendant | 96, up to 125° | rigid on the link | none (KH2's necklace is one bone, 165, which the link 86 follows) |

**The design: KH1's own animation of the same move.** Each KH2 move KH1 plays stands in for one
of KH1's (a slot's template: KH1's run for KH2's run, KH1's first swing for KH2's first). These
parts take, relative to their parent, exactly what KH1's own motion of that move gives them,
timed to KH2's motion as KH1 times the template (stretched to KH2's length, `slot::retimed`).
So KH2's run gets KH1's run's belt chain swing, shorts and hem, once per KH2 stride; a KH2 swing
gets KH1's swing's. Their parents are KH2-posed (the hips, the thighs, the spine, the necklace
link), so they ride on KH2's body and add KH1's own motion on top. KH1's scale keys on them come
too (KH1 squashes its shorts' panels, 0.21-1.36). This is the most faithful version: it is KH1's
own animation, not a made-up one, and needs nothing at run time beyond reading KH1's bones. The
guards stay: the belt chain's end and the hem's panels are still swung off the thighs
(`keep_clear`).

- **Baked** (`Baker::poses`, its template given by `kh2slots::bake_one`): the template's FK at
  `frame × template frames / KH2 frames`, each part's relation to its parent (`own_relation`)
  given to the retarget (`Retarget::set_own`), its scale to the bake's scale channels.
- **Live** (`pose.rs`, `own_from_kh1`): KH1's own pass has just placed these bones from the
  motion it plays (our slot: the template retimed; otherwise KH1's own motion for the slot KH2's
  is drawn over); their relation to their parent there goes to the retarget, their scale to the
  final scale pass. Not on frames where KH1's pass didn't run (the last relation stays).
  **KH1's blend pass:** it leaves every bone without bit 30 in its bone-table word
  unwritten (`docs/kh1-anim-slots.md` §1.6, the blend path), and every own part's parent is one
  (the shorts' 26 / 30 / 34 / 38 and 52 / 56 / 60 / 64, the hem's 72 and 248-276 with 71 above,
  and, while they were own parts, the belt's 4 / 7 / 10). Before this fix the parts were measured against what we drew there last, so
  on every blend (each return to the idle, the tester: "standing and returning to neutral cause
  parts of soras model to stretch like crazy") they were put where KH1's own blended pose has
  them in the model, off KH2's hips and thighs. Now a bone still holding what we wrote
  (`Sora::wrote`) is placed from the bone above it by the relation KH1 last gave it on a pass
  that placed both (KH1 keys none of these bones, constants only: Checked (files)), and a part
  whose parent can't be placed keeps its last relation. Checked (decomp, files); not in game.
  **"Still holding what we wrote" (fixed after t409):** both passes end by adding their world
  translation to every bone's world matrix, the ones they didn't place too (blend `0x140392560`,
  plain `0x140391C70`; first multiplying by the world's scale when it is more than 0.005 off 1:
  every bone in the blend, all but the skip bones in the plain pass). Checked (decomp). So a
  bone the blend left holds our matrix moved by Sora's world position, not our matrix; t405
  compared it as written, took every such bone as placed, and measured the shorts and hem from
  parents one world position off, so they were drawn at the map origin on every blend-pass
  frame (the tester's t409 picture and trace f20377: each stop from the walk to the idle is a
  plain pass plus a blend, then blend-only frames). `pose::kept` now tests for that move (and
  the scaled move). Assumption: on which side the scale is multiplied (only matters off scale 1).
- `Retarget::attach` (the belt chain, carried by the +x thigh): the root rigidly as before, the
  links under it by their relation to their parent, KH1's own where given (the bind's before).

Checks (files; `kh1-data` test `local_own`, a scratch run): KH2's run (2) on KH1's run (slot 5,
36 frames → 32), its walk (1) on KH1's (2, 60 → 72), the swings 151 / 152 / 154 / 181 on 98 / 99 /
101 / 102, the idle (0) on 1: no NaN; every part's relation to its parent is KH1's motion's
(float noise only) but where `keep_clear` swung a chain off a thigh; each part's distance from
its parent is KH1's motion's (KH1 keys some panels' lengths); no cloth node came nearer a thigh
than 12.2. Over KH2's run the belt chain swings 115°, the hem panels 45°, the pendant 70°, the
shorts 5° (KH1's run: 23-54°, 10-34°, 37°, 3-12°). Not checked in game.

Not done: matching KH1's stride phase to KH2's within a cycle (the two runs are stretched end to
end: Assumption that both start on the same foot), and anything for KH2 moves that stand in for
no KH1 move (they take whatever KH1 plays meanwhile, live).

Also not carried: the other way
round, KH1's own animation of all these parts when KH2's model is drawn (`inverse` keeps KH2's
cloth, hair and hood at its bind relation).

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
