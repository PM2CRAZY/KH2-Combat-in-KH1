# KH1: how a melee swing knocks back a projectile

Research notes (Ghidra, Epic KH1 Final Mix PC, image base `0x140000000`, addresses absolute;
the hook adds `base + (addr - 0x140000000)`). Read from the code statically, not yet watched
in play. KH1 world space, y down. "Handle" = KH1's 32-bit compressed pointer
(`0x14038B0B0` decodes, `0x14038B080` encodes; `pose::ptr` in the hook).

## Summary

- A reflectable shot (Wakka's blitzball and the like) is not a body. What can be hit is a
  **KH1 attack object in the attack pool** (`0x142DB5110`, 64 slots of 0xC0), almost
  certainly a *free-standing* one: attribute word `+0x30` bit `0x400`, made from attack data
  whose bone index (`+0x10`) is negative, so the object has no host bone and its controller
  writes its centre (`+0x10`) every frame. The thing that flies and is drawn is that
  controller: an AI-script process (KH1's enemies and field objects run a bytecode VM) or a
  script-driven object actor. Our `bodies()` only looks at actors' collision parts, so it
  never finds the shot.
- A KH1 swing knocks it back through the **attack-vs-attack clash**, not the body contact:
  the per-frame collision pass meets the swing's attack object with the shot's, calls
  `0x1402B2890` (clash of two entries), which calls `0x1402B2A30` both ways. With the shot as
  the first argument, `0x1402B2A30` sees a free-standing object with priority below 98 and
  **reflects** it: it moves the object to the other team (team bits 12..13 of the flags:
  1 becomes 2, anything else becomes 1), makes the swing's owner its owner (`+0x34`), sets
  flag `0x1000000` ("reflected, waiting for the controller") and writes the clash result
  (0x10, or 0x20 when the swing out-prioritises it by more than 40) to its `+4`. There is no
  velocity in the attack object: the controller sees the flag / result, turns the ball round
  and clears `0x1000000` (the collision pass skips the object until it does). From then on
  the shot is on Sora's side and hits Wakka's body like any of Sora's attacks.
- Nothing in the attack data says "reflectable" beyond that: free-standing (`+0x30` bit
  `0x400`) plus priority `+0x68` below 98 (98 / 99 are unbreakable, 100 is a guard) plus a clash
  type the swing's type may meet (table below).
- Our `land()` objects never take part: they are made, used for one body contact
  (`0x1402B1ED0`, attack vs *body*) and freed in the same call, so the collision pass never
  sees them, and the body contact cannot touch a pool object anyway. But the same object
  works as the swing side of a clash: build the two 0x40-byte entries and call
  `0x1402B2890(swing_entry, shot_entry)` (call sequence below).

Confidence: high for the clash and reflect mechanics (the code is unambiguous); medium that
Wakka's ball specifically is a free-standing pool object driven by a script (strongly implied:
the reflect branch exists only for free-standing objects, it counts a tally and gives MP only
when the other side is Sora's element-1 attack, i.e. his Keyblade, and the pass leaves the
reflected object alone until something outside native code clears the flag). The probe at the
end settles it in one run.

## KH1's collision pass

`0x1402B22D0` (once a frame, skipped while `0x142867CD8` bit `0x4000`):

1. A 0x4000-byte scratch array of 0x40-byte entries.
2. `0x1402B2610` walks the 64 pool slots. For each: clears "done this pass" (`0x8000`);
   skips free slots (`0x800` clear); frees dying ones (`0x200000`); an inactive one
   (activity bits 2..3 = 0) is marked dying unless bit `0x100`; clears a dead owner; an object
   with a host (`+0x30` bit `0x400` clear) is freed when its host (`+0x90`) is gone; **an object
   with `0x1000000` set is skipped entirely** (its `+4` is not even reset); otherwise `+4` is
   cleared, objects of a host with actor `+0x88` bit 0 (probably Stop) are left out unless they
   are guard types, the activity type is applied (1: only while `0x142867CDC` bit 0;
   2: only while the host plays motion `+0x58`; 3: interval re-arm `0x1402B11F0`), then
   `0x1402B1670` updates its position (mode bits 0..1: 0 its own `+0x10`; 1 a bone of the host)
   and sweeps it into sphere entries with `0x1402B1BC0` (previous centre `+0x70` to current
   `+0x10`, step about one diameter). **Each new entry is tested against all earlier entries
   with `0x1402B1A50`**, so attack-vs-attack happens here. Then the wall test `0x1402B1870`.
3. Every hittable actor's body parts (`0x14028FEE0`, parts from `+0x134`) become entries
   tested against the attack entries with `0x1402B1A50` (contact `0x1402B1ED0`).
4. Afterwards every live object's current centres are copied to the previous ones
   (`+0x70` <- `+0x10`, `+0x80` <- `+0x40`, flag `0x10000`), and every clashed-with mask
   (`+0x9C`) and actor hit-once mask (`+0x3E0`) is ANDed with the live hit bits
   (`0x142DB8110`).

### Entry (0x40 bytes)

| off | attack entry | body entry |
|---|---|---|
| +0x00 u32 | team = flags bits 12..13 | team = actor `+0x374` bits 5..6 |
| +0x08 u64 | owner actor pointer (decoded `+0x34`) | the actor |
| +0x10 u32 | the object's hit bit (`+0x54`) | part's first byte |
| +0x18 u64 | attack object pointer | 0 |
| +0x20 | shape: type u32 (1 sphere), radius f32 `+0x24`, radius / half height f32 `+0x28`, centre vec4 `+0x30` | same, from `0x1402B3CB0` |

### `0x1402B1A50 (new entry, entries, count)`: what may meet what

For the new entry against each earlier (attack) entry: teams differ; owners differ; neither
object is dying (`0x200000`); for a body, its hit-once mask `+0x3E0` has not got the attack's
bit; the earlier object is not suspended (flags bit 10). Then:

- body: the attack's `+4` has no clash result (`& 0x30`) and its clash type is not 1, 2 or 4
  (those are pure guards that hit no body);
- attack vs attack: neither has already clashed with the other (`+0x9C` vs the other's
  `+0x54`, both ways) and the clash-type table allows the pair: the byte at
  `0x1405299B8 + type(earlier)` must have bit `type(new)`. The table (symmetric) is
  `1F 09 09 3F 29 18 00 00`: type 0 meets 0-4; 1 and 2 meet 0 and 3; 3 meets 0-5; 4 meets 0, 3, 5;
  5 meets 3 and 4; 6 and 7 meet nothing.
- either way: when the earlier object has flags bit 7 (one contact a pass) its `+4` must be
  empty; the shapes must overlap (`0x1402B3D50`).

A body goes to `0x1402B1ED0`; two attacks go to `0x1402B2890(earlier, new)`.

### `0x1402B2890 (entry a, entry b)` (void)

1. Directional gate both ways, `0x1402B24B0(obj a, obj b)` and `(obj b, obj a)`: if either
   returns 0 nothing happens. It returns 1 at once unless the first object's clash type is
   1..4 and the second's flags byte 0 lacks bit `0x40`; then the first's owner facing
   (`+0x30`), turned by its data's `+0x60`, must be within the cosine at data `+0x64` of the
   direction to the second (owner to owner, or for a free-standing second object, from its
   current centre back to its previous one, `+0x70 - +0x10`: it must be coming at the front).
2. `ra = 0x1402B2A30(a, b)`, `rb = 0x1402B2A30(b, a)`. A positive result plays that side's
   clash effect (`0x1402B2270`: contact point `0x1402B1DC0`, effect from its data's `+0x28`
   through `0x1402B2190`).
3. If only one side got a result, the other gets the same bits in `+4` and in `*(+0x60)` and
   is marked spent (`0x100000`, unless free-standing).
4. If no effect played: KH1's clash spark `0x1402AC4C0(*0x142D67400, 10 or 11, 20000, midpoint)`
   (11 when the results include 0x20).

### `0x1402B2A30 (attack p, attack q) -> result` (the clash proper)

Nothing (0) unless p's `+4` has no clash bits, at least one priority (`+0x68`) is below 98
and q's priority is not 0. Then each records the other's hit bit in its clashed-with mask
`+0x9C`, and:

- p priority 100 (a guard): returns 0x10 (the guard is unaffected).
- p's owner alive with actor `+0x88` bit 0: p is not affected (0x10 / 0).
- p priority above q's and p of clash type 1, 3 or 4 (directional): **p wins**: a hit record
  from q on p's owner, made no-effect (`0x1402BD480(q, 0, owner)`, record `+0` |= 0x20); p
  marked (0x10, spent). (This is how a guard-type swing "parries" with no recoil.)
- otherwise, p priority below 98:
  - p has a host (`+0x30` bit `0x400` clear): a **clash reaction** record on p's owner
    (`0x1402BCEB0(owner, 0)`): kind 6, or 7 when p's priority is more than 40 below q's;
    `+8` the priority difference, `+4` q's owner, `+0xC` q's attributes, `+0x2C` p's data,
    `+0x10` a push direction from the owner's facing. Sora plays motion 0x6E for kind 6
    (`0x14029B590`, pushed back when the difference is negative); enemies get the record in
    their script.
  - p free-standing (`+0x30` bit `0x400`): **reflected**. Team bits 12..13 to 1 (or to 2 when
    they were 1), flag `0x1000000`, owner `+0x34` = q's owner. Nothing about speed.
  - result 0x10 (0x20 when more than 40 below) into p's `+4` and `*(+0x60)` (the owner's
    combat state `+0x50` for row attacks; 0 for script-made ones); p marked spent unless
    free-standing.
  - when q's owner is Sora (object id 0) and q's element is 1 (`+0x30` bits 6..9; the
    Keyblade rows' data `+4` is 1): tally `0x14028A4D0(10)` and, when p's `+0xB0` > 0 and
    ability 0x40 is not equipped, MP for Sora (`0x14026E820` / `0x140282820`, amount
    `+0xB0 * (bits 19..21 of 0x142D5F610 + 1)`).
- p priority 98 / 99: unaffected (0x10, or 0 if q's owner has `+0x88` bit 0).

So for a shot vs Sora's swing: the shot side reflects (if free-standing) or gets a kind 6/7
record (if it has a host, e.g. a ball actor's own attack: then that actor's script reacts);
Sora's side either wins (directional swing with the higher priority: harmless no-effect
record) or gets a kind 6/7 recoil.

## The attack object (0xC0, pool `0x142DB5110`)

Made by `0x1402B0C30 (actor, hit bit, data)`; `0x1402B10E0` (attack rows, our `land()`) adds
the weapon's bonuses when the data's bone is 0x30 (radius, priority +`ws+0x33` clamped
1..99, critical) and points `+8` / `+0x60` at the combat state. Script-made ones come from
syscall `0x1402B9870` (table `0x140529E80`, entry 23; `+0xA0` = the script process).

| off | meaning |
|---|---|
| +0x00 | flags: 0..1 position mode; 2..3 activity (0 off, 1 global, 2 motion-bound, 3 interval); 4..5 from data `+0x34`; 7 one contact a pass; 8 keep when off; 10 suspended; 11 in use; 12..13 team; 14 always; 15 done this pass; 16 previous centre valid; 17..19 clash type (data `+0x38` & 7); 20 spent (data `+0x38` bit 16 sets it at birth); 21 dying; 22 hit sound on walls; **24 reflected, waiting for its controller**; 26 snap to wall; 27 double sweep step |
| +0x04 | this pass's results: 0x10 / 0x20 clash, 2 / 8 / 0x100 walls (reset each pass) |
| +0x08 | effect-points handle (combat state `+0x18`) |
| +0x0C | radius |
| +0x10 | centre (vec4, current) |
| +0x20 / +0x24 / +0x28 | bone, offsets (data `+0x10`, `+0x54`, `+0x58`) |
| +0x2C | power |
| +0x30 | attributes: 0..4 reaction kind (data `+8`), 6..9 element (data `+4`), **10 free-standing** (data bone < 0) |
| +0x34 | owner actor handle |
| +0x3C / +0x40 | end radius / end centre (swept segment) |
| +0x50 / +0xA4 | re-arm interval / timer |
| +0x54 | hit bit (`0x1402B15D0`; live bits in `0x142DB8110`) |
| +0x58 | motion request it is bound to (0xFFF any) |
| +0x5C | attack data handle |
| +0x60 | pointer to a result word (combat state `+0x50`) or 0 |
| +0x68 | priority (data `+0x3C`): 100 guard, 98..99 unbreakable |
| +0x70 / +0x80 | previous centre / previous end |
| +0x90 | host actor handle (whose bone it follows) |
| +0x94 / +0x98 | critical chance / bonus |
| +0x9C | clashed-with mask (other objects' hit bits) |
| +0xA0 | script process handle (script-made) |
| +0xA8..+0xAA | effect / sound bytes |
| +0xAC | damage scale |
| +0xB0 | MP given to Sora when he clashes with it (data `+0x68` x owner stats `+0x68`) |
| +0xB4 / +0xB8 | free callback (`0x1402B13D0` calls it) / its argument |

Attack data (0x70 bytes at `0x142D23740 + offset`; common entries by index through
`0x14028D7A0` / `0x1405294D0`): `+0` power source, `+4` element, `+8` reaction kind, `+0x10`
bone (negative: free-standing), `+0x18` / `+0x1C` critical, `+0x28` clash / hit effect,
`+0x34` flag bits, `+0x38` clash type (low 3 bits; bit 16 spent), `+0x3C` priority, `+0x54` /
`+0x58` offsets, `+0x5C` radius (int), `+0x60` / `+0x64` directional arc (turn, cosine),
`+0x68` MP factor.

(Aside for `combat.rs`: the `((f & 0xE0000) - 0x20000) & 0xFFF9FFFF == 0` test is true for
clash types 1 to 4, not only 1 and 3.)

## Reflecting a shot from a KH2 swing

### Finding shots (each frame, beside `guards()`)

Scan the 64 slots (`0x142DB5110 + k * 0xC0`) and keep an object when:

- flags has `0x800`, activity bits (`& 0xC`) non-zero, and none of `0x400` (suspended),
  `0x200000` (dying), `0x1000000` (already reflected, not yet turned);
- attributes `+0x30` has `0x400` (free-standing; drop this test only after the probe shows a
  shot with a host);
- team (flags bits 12..13) differs from Sora's (`(+0x374 >> 5) & 3`), owner (`+0x34`) is not
  Sora;
- priority `+0x68` is 1..97;
- the clash table allows it against Sora's swing type (`data +0x38 & 7` of the row we land
  with): `TABLE[type(shot)] & (1 << type(swing)) != 0`;
- `+4 & 0x30 == 0` (and `+4 == 0` if flags bit 7).

Its shape: a sphere of radius `+0xC` at `+0x10`; when flags has `0x10000`, KH1 sweeps it from
`+0x70` to `+0x10`, so a capsule over that segment is the faithful test (fast balls). Flip y
for KH2 space.

### When a KH2 swept swing overlaps one

1. `bit = 0x1402B15D0()`; `obj = 0x1402B10E0(sora, bit, 0x142D23740 + row.data)` (as
   `land()`).
2. Put the swing there: `obj+0x10` = contact centre (KH1 world, y down, w = 1); also
   `obj+0x40` the same and `obj+0xC` the swing radius. Keep the hit bit (`+0x54`) as is: it
   only goes into the shot's `+0x9C` and is cleared from it by the next pass once freed.
3. Swing entry S (0x40 zeroed): `+0` = `(obj flags >> 12) & 3`, `+8` = Sora's actor
   pointer, `+0x10` = `obj+0x54`, `+0x18` = obj, `+0x20` = 1, `+0x24` = `+0x28` = radius,
   `+0x30` = centre.
4. Shot entry B: `+0` = `(shot flags >> 12) & 3`, `+8` = decoded `shot+0x34`, `+0x10` =
   `shot+0x54`, `+0x18` = shot, `+0x20` = 1, `+0x24` = `+0x28` = `shot+0xC`, `+0x30` =
   `shot+0x10`.
5. `0x1402B2890(&S, &B)` (rcx, rdx). Swing first, so Sora's side is decided while the shot
   still has its thrower as owner, then the shot is reflected; effect or spark included.
6. Reflected when `shot flags & 0x1000000` (owner now Sora, team flipped); `shot+4 & 0x30`
   says the clash happened either way.
7. `0x1402B13D0(obj)`.
8. KH2 side: count it as a landed hit if wanted (`register_hit`), not as a repel.

Preconditions the call checks by itself (the reflect does not happen if they fail): Sora
facing (`+0x30`) and the shot coming at his front when the swing's clash type is 1..4
(`0x1402B24B0`); the shot not winning (it wins only if it is directional and out-prioritises
the swing); the thrower without actor `+0x88` bit 0.

Side effect on Sora: unless the swing wins (clash type 1, 3 or 4 and the higher priority), the
swing side makes a kind 6 / 7 record on Sora, which `hurt.rs` turns into KH2's recoil. That is
what KH1 itself would do with those values. To reflect without any recoil, call only the shot's
half instead of step 5: `0x1402B24B0(shot, obj)` and `0x1402B24B0(obj, shot)` for the facing
gate if wanted, then `0x1402B2A30(shot, obj)` (rcx shot, rdx swing; result > 0 when it acted),
then our `clang()` at the midpoint. That keeps the reflect, the tally and the MP gain (they are
in the shot's half) and makes no record on Sora.

## Probe to confirm (one run of the Wakka duel with KH1 driving Sora)

Hook `0x1402B2A30` (two pointer args, returns u32). When either object's owner is Sora, log
for both objects: flags, `+4`, `+0x30`, `+0x34` (owner's object id `[[owner+0x130]+0x4C]` and
kind byte `+6`), `+0x68`, `+0x90`, `+0xA0`, `+0xC`, `+0x10`, the data's `+0x10` / `+0x38` /
`+0x3C`, and the result and the shot's flags after the call. Expected: the ball with `+0x30`
bit `0x400`, `+0xA0` non-zero (script-made) or a host that is a separate object actor, and
`0x1000000` set after the call; Sora's swing type and priority (the row data's `+0x38` / the
object's `+0x68`) tell whether KH1 makes him recoil.
