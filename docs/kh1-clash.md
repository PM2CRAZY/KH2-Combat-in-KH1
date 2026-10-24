# Clanks on KH1's Keyblade, and the Aerial Dodge's parry

Addresses are the Epic build's; the Steam build's are in `src/win/steam_map.in`.
Confidence: **Checked (exe)** read from KH1's code; **Ours** a choice of the mod's.

## KH1's swing and the Keyblade's length (Checked, exe)

- Sora's swing rows' attack data put the attack object on bone `0x30` (data `+0x10`), the
  Keyblade, at two offsets along it (data `+0x54` / `+0x58`).
- Making a row's object (`0x1402B10E0`) adds the weapon's own stats when the bone is `0x30`:
  the weapon object (actor `+0x14C`) points at them (`+0x20`): `+0x20` a radius bonus (times
  the actor's size `+0x40`), `+0x2C` a length factor on the two offsets, `+0x33` a priority
  bonus, `+0x31` / `+0x32` critical.
- A point along bone `0x30` (`0x14029C990 (actor, bone, t, out)`): `t` is first mapped into the
  weapon's own blade range, `(ws+0x24 - ws+0x28) * t + ws+0x28`, then taken between the bone's
  parent and the bone through the model instance's world matrices (`0x1401D41C0`). So each
  Keyblade's reach is its own: `t` 0..1 covers its blade from start to tip.

## Our clash test (Ours)

- The right Keyblade's swing meets guards, enemy attacks and walls with KH1's blade as drawn:
  six points `t` 0..1 along bone `0x30` (Sora's drawn matrices), swept from last frame's
  points, each sphere KH1's swing radius for this weapon (row data `+0x5C` + the weapon's radius
  bonus × his size). A shorter Keyblade reaches less far, a longer one farther.
- KH2's wide hit shapes still decide what the swing hits on bodies (KH2's hits); only clashes
  use the blade. Valor's left Keyblade keeps KH2's shapes (thinned to KH1's radius).
- Enemy melee attacks (not only free-standing shots) now meet the blade, through KH1's own
  attack-vs-attack clash (`0x1402B2890`): when KH1 marks Sora's side as clashed (`+4` 0x10 /
  0x20), the swing stops there and his recoil is KH1's kind 6 / 7 record, which `hurt` plays as
  KH2's recoil.
- Walls (ini `wall_clink`): the blade's points from last frame to now through KH1's background
  ray, not KH2's shapes (those reached far beside the blade: clinks "at random moments").
- Each clank is logged with where along the blade it touched (0% the blade's start, 100% its tip).

## KH2's guard windows as KH1 guards (Checked + Ours)

KH2's clips mark their guards as `Guard` windows (an ATKP, a shape group, body or Keyblade).
From the user's KH2 data:

| motion | window | frames | ATKP | refact self |
|---|---|---|---|---|
| 202 Aerial Dodge | Guard, group 6 (body) | all of it; no Invulnerable window, no hitbox | 406 (kind 2, power 0) | 2 (nothing) |
| 40 Aerial Recovery | Guard, group 6 (body), and Invulnerable | 0..16 both | 124 (kind 3, power 0) | 2 |
| 173 guard press | Guard, group 2 (Keyblade) | 6..37 | 161 (kind 2, power 0) | 1 (guard repel) |
| 42 guard repel | Guard, group 2 (Keyblade) | 0..40 | 162 (kind 2, power 0) | 2 |

- KH1's attack-vs-attack pass meets attack objects, not the actor: Sora's untouchable flag (0x40,
  his i-frames) keeps him out of the hit pass but not his own attack objects. A guard is an
  attack object of priority 100: the guard side is unhurt (`0x1402B2A30` returns 0x10 at once)
  and the attack meeting it gets a clash (its owner a kind 6 / 7 recoil record, a free-standing
  shot turned back); the guard's clashed-with mask `+0x9C` gains that attack's hit bit.
- Clash types 1, 2 and 4 are pure guards: KH1's pass never meets them with a body
  (`0x1402B1A50`), so they hit nobody. Types 1..4 are directional (`0x1402B24B0`): the owner's
  facing, turned by the data's `+0x60`, within the cosine at `+0x64` of the direction to the other.
- Activity (flags bits 2..3): 1 is live only while `0x142867CDC` bit 0; 3 re-arms on an interval
  (`+0x50` frames, `0x1402B11F0` clears the hit-once and clashed-with marks of its bit).

Ours (`combat::guard_step`): every live KH2 `Guard` window puts a KH1 guard object on each of its
shapes (up to 4): clash type 4, priority 100, no power, no reaction, its own position (mode 0) set
to the shape each frame, activity 3 re-armed every 10 frames, attack data our copy of a swing's
(arc: all round for a body guard, cosine -1; in front for a Keyblade guard, cosine 0). Freed when
the window closes. An attack it stops (its `+0x9C` gained a bit) plays KH2's guard reaction by the
window's refact self (1 the repel, 2 nothing), logged. So Aerial Dodge is a guard only (no hitbox
on enemies), and Aerial Recovery guards on KH2's frames, through its i-frames.

`hurt`'s own guard (an attack that reaches his body during a guard window) uses the same refact
self, and a body guard is all round.

In KH1's own states (testing branch: the guard press 173, its repel 42 and Guard Break 165 as KH2
motions of ours KH1 plays, `docs/kh1-anim-slots.md` §7.x) the same objects come from those motions'
Keyblade `Guard` windows (`takeover::kh2_hits`), and a stop starts the repel on KH1's states
(`combomods::guard_repel`) instead of KH2's controller's `clash`.

Left to the controller (not this lane): KH2's Aerial Dodge has no invulnerable frames (ours gives
10..16 by level, `AIR_DODGE_STAR`), and Aerial Recovery's are its window 0..16 (ours `f <= 16`,
one frame more).
