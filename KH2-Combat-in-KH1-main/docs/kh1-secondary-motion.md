# KH1 port: hair, hood and keychain under KH2's moves

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

- `physics = off`: none of this, and no swing either. The parts turn with their parent as
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
