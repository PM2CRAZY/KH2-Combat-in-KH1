# KH2 animations as KH1 motion slots (design, testing branch)

The user (testing): "currently, if there's an exclusive kh2 animation kh1 sora is pulled and kh2
sora is called ... instead lets have it so that our kh2 animations can run entirely through kh1s
animation system as new animation slots, that way kh2 animations can have access to everything
kh1s animation engine can do."

The problems listed:
- custom magic can't spawn an effect on a hit enemy;
- KH1 and KH2 trading places draws badly;
- switching animations between the two engines breaks the look and the timing;
- a KH2 animation that wants KH1's physics can't have it.

This doc covers how KH1 plays Sora's motions (static RE of the Epic exe, checked against the
user's `xa_ex_0010.mset`), how new slots get in, and how KH2 motions are turned into them. The
`kh1_slots` ini key switches the prototype (default on; `off` for the old look; renamed from `kh2_slots` at t311, the old key is ignored). Related docs: `animations.md` (every
slot and motion), `shared-states.md`, `animation.md`, `kh1-kh2-model.md`. The other helper's
physics work (KH2's physics inside KH1's system) is not covered here.

No game data or decompiled code is in the repo. Everything below is our own description of what
the code does.

## 1. KH1's motion system for Sora

### 1.1 The file: `xa_ex_0010.mset`

The file has three sections: `count u32 = 3`, then the section offsets.

| Section | What it is |
|---|---|
| 0, `MMTN` (`+0x80`) | The **slot table**. At `+0x40`, one offset per slot (from `+0x40`), ending at `0xFFFFFFFF`. Sora has **118 slots** (0..117). |
| 1 (`+0x252480`) | The **request table**: one `u32` per *request id*, 321 ids (0..0x140). Bits 0..11 hold the slot (`0xFFF` = none), bits 12..14 a kind, bit 15 a flag. The high half is an extra id, used by `0x14029DDAA` → `0x1402ABFD0`. |
| 2 (`+0x252A00`) | 0x80 bytes, its use unknown (it is the actor's `+0x1DC`). |

**A motion** is all offsets from its own start. When the file is loaded, these offsets are
turned into 32-bit handles (`0x1401D5CE0`).

| Offset | Field |
|---|---|
| `+0x04` | frames (60 fps) |
| `+0x08` | fps (60.0) |
| `+0x10` | bones (293) |
| `+0x14` | the per-bone joint words: an IK, constraint and skip mask. Bits 0..9 and 11..19 are indices, bits 21..22 a mode. With bit 26 set, that bone's world matrix isn't written (`0x14039256D`). |
| `+0x18` / `+0x1C` | the channels: 6 bytes each, `bone u16, kind u8, count u8, first key u16` |
| `+0x20` / `+0x24` | the helper channels, which animate the IK helpers |
| `+0x28` | the keys: 16 bytes each, `time<<8 \| interpolation`, value, out tangent, in tangent |
| `+0x2C` / `+0x30` | the IK helpers, 40 bytes each: scale, parent, rotation, translation. 11 on Sora. |
| `+0x34` / `+0x38` | the constant channels, 8 bytes each: `bone u16, kind u16, value f32` |
| `+0x3C` | **the event table** (§1.4) |

The load turns `+0x14`, `+0x1C`, `+0x24`, `+0x28`, `+0x30`, `+0x38`, `+0x3C` and each event
group's `+8` into handles. A table entry becomes the handle of `table + offset`. The table's
`0xFFFFFFFF` becomes handle 0. `kh1_data::mset` reads the file layout.

**Handles.** A handle is `0x80000000 | block << 25 | (p & 0x1FFFFFF)`, where `block` indexes a
64-entry table of 32 MB bases (`0x142EE4730`). `0x14038B080` / `0x14038B0C0` turn **any**
pointer into a handle, registering its 32 MB block if it is new. So motion data we allocate on
our own heap can be handed to KH1 as long as it goes through that call.

Sora's motion files are the model's mset only. Nothing for him comes from ANB files: ANB is
KH2's format. The actor has a second, pending motion set at `+0x1E0..+0x1EC`. When a request
resolves to `0xFFF`, it is swapped in (`0x14029DC24`, `+0x160` bit `0x20`). That is how a
world's extra motions come in: Neverland's flight, request `0x75`, has no entry in Sora's own
table.

### 1.2 Actor fields (`0x140285CC0` binds an actor to its model and motion set)

| Field | Meaning |
|---|---|
| `+0x15C` | the animation instance (pose: `pose.rs`) |
| `+0x160` | motion flags: bit 0 a new request, bit 1 playing, bit 2 blending allowed, bit 3 hold at the end (else loop), bit 6 a custom loop range |
| `+0x164` | **the request id playing** (`request & 0xFFFF`). KH1's own code tests this: attack objects (`+0x58`), event scripts (opcode 322), combo windows and our hooks. |
| `+0x168` | **the slot playing** |
| `+0x16C` | time (frames) |
| `+0x170` / `+0x174` | blend length / blend clock |
| `+0x178` | the request queue: a count, then 16-byte entries from `+0x17C` (request, start time, blend, a handle) |
| `+0x1CC` | the motion's level flags this frame (§1.4) |
| `+0x1D0` | handle of the mset file |
| `+0x1D4` | **handle of the slot table** (from `MMTN + 0x30`, `0x1401D40D0`) |
| `+0x1D8` | **handle of the request table** (section 1) |
| `+0x280` | length |
| `+0x284` | speed |
| `+0x288` / `+0x28C` / `+0x290` | the loop range (`0x14029D880` sets a custom one) |

### 1.3 How a state asks for a motion

**Asking.** KH1's states call `0x14029D7F0 (actor, request, blend)`, which starts a fresh queue.
`0x14029D8A0` / `0x14029E150` append to the queue, and `0x14029D630` is the blended form. Two
request bits change the actor's flags:
- bit 24: action flag 4 (`combat.rs` sends attacks as `motion | 0x1000000`);
- bit 23: flag `0x1000`.

**The animation step** (`0x14029CF10 (actor, dt)`, once a frame):
1. On a new request, `0x14029DAB0`:
   - `+0x164 = request & 0xFFFF`.
   - **Resolving.** `0x14029E050` looks the request up in `+0x1D8`. With request bit 29
     (`0x20000000`) the low bits are taken as a raw slot. Otherwise it reads the table entry and
     goes by its kind:
     - 0: put the weapon away (bit 27 → `0x1402A10F0`);
     - 1: draw it (bit 28 → `0x1402A0CA0`);
     - 3: field / battle / no-combat variant: slot + 0, 1 or 2 by the battle flag `+0x374`
       bit 18 and `0x142867CD8` bit `0x1000`;
     - 4: slot + 1 in battle;
     - 2: as is.
   - **Starting.** `0x14029E1C0` takes `motion = ptr(ptr(+0x1D4)[slot])`. It poses it, blending
     (`0x140393CE0` / `0x140394230`, then `0x14029E490` → the ground-IK pass each frame) when a
     blend longer than a frame was asked for. It sets `+0x280` from the motion's frames and the
     loop range, and `+0x168 = slot`.
2. `time += dt × +0x284`. The frame scale `0x1423405DC` is in `dt`.
3. The edge events between the old and the new time fire (`0x14029D9A0`, §1.4).
4. At the end, the queue's next request plays, or the motion holds or loops.
5. The pose (`0x14029DDF0` → `0x140391E20`, the plain pass) or the blend pass
   (→ `0x140392710`, the ground-IK pass). Both are hooked by `pose.rs`.
6. `+0x1CC` = the instance's level flags (`+0x14`), which the pass computed.

**The log's "set motion slot N" is the request id, not the slot.** `probe::setm_note` prints
`req & 0xFFFF`. Proof: line 658 of the s261 log "sets slot 113" (request `0x71`), then
`pose::slot_of` reads **75** while KH1 holds him (line 670).

### 1.4 Motion events (the `+0x3C` table)

The table is a list of 12-byte groups `(id u16, type u16, count u32, keys)`, ending at
`0xFFFFFFFF`. Each key is 16 bytes: `time f32, a u16, b u16, c u32, d u32`. There are two ways
to read them:
- **Levels** (`0x140394920`: the last key at or before the time). The plain pass ORs them into
  the instance's `+0x14`, which ends up in the actor's `+0x1CC`.
- **Edges** (`0x140394840`: every key in `[from, to)`). These fire through `0x14029D9A0`.

| Type.id | Kind | Effect | Seen on |
|---|---|---|---|
| 1.0 .. 1.3 | level | **hit window** of attack group 0..3: `+0x1CC` bit `0x10000 << group`. An attack object tied to a motion is live only while this is set **and** its `+0x58` request equals the owner's `+0x164` (`0x1402B2610`, `combat::shots`). | every swing, 98..116 |
| 5.0 | level | **combo window**, bit 22 (`combomods::KH1_COMBO_WINDOW`) | the swings |
| 7.0 | level | **weapon trail**, bit 23 (`0x1402BFD70`) | the swings |
| 0.0 | level | bit 20: from here the action can be left (`0x14029D399` clears action flags `0x1004`) | item 58 at 74/82, Fire 55 at 68/86, roll 117 at 30/38 |
| 2.x | edge | `0x1401D9A40 (instance, 0, a)`: a face or texture change, **unconfirmed** | |
| 4.0 .. 4.3 | edge | **foot effect**: `0x1402C04B0 (actor, kind a, bone b)` puts a dust or splash effect (`0x1402C0230`) at bone `b` (22 / 48 are the feet) when near the floor | idle, walk, run, landing, swings |
| 6.0 .. 6.3 | edge | **sound**: `0x140293290 (actor, a, b, c)`. `a = 0`: sound `b` by id; `a = 0x19`: his voice `b` from his voice set (combat info `+0x20`'s `+0x34` + `b`); otherwise a material or footstep sound (table `0x140529788[a-1]` → `0x1402939B0`) | everywhere |

**Not in the events:**
- **Attack objects.** They come from the state. KH1's Attack command picks a row of the attack
  table (`0x142D5D4B0[object] + row × 0x14`: motion request, attack data, sound / effect bytes).
  `0x1402A17F0` sets the motion and makes the attack object (`0x1402B10E0`). The object follows
  KH1's bones (`+0x1B4`, which our pose hook writes) while the motion's hit window is open.
- **The hit effect and hit sound** are played by the contact code (`0x1402B1ED0`) from the attack
  data.
- **Invulnerability.** The roll's has no event type in this table; it comes from KH1's roll
  state. **Unconfirmed.**

### 1.5 Sora's slot map, by request (from the request table)

| Requests | Slots | What |
|---|---|---|
| 0 / 1 / 2 | 0..1 / 2..4 / 5..7 | idle, walk, run. Kinds 4 and 3 pick field or battle. |
| 4..7, 9..0xC | 8..11, 12, 13, 10, 11 | high jump, jump, fall, land |
| 0x3E | 58 | the item's use |
| 0x6E | 72 | clash recoil |
| **0x71..0x74, 0x76..0x79** | **75..78** | **the glide**: slow, fast, banking either way. Superglide asks for 0x76..0x79. |
| 0xC8..0xCB | 98..101 | the ground combo |
| 0xCC..0xCE | 102..104 | the air combo |
| 0xCF..0xDA | 105..116 | KH1's other attack motions (combo modifiers and specials; which is which is **unconfirmed**) |
| 0xDC | 117 | the dodge roll |

**Bug found (fixed in phase 1b, `shared.rs` / `pose.rs`):** `shared::slot_kind` and
`pose::play_for` took **113..116** as the glide. They are attacks (requests `0xD7..0xDA`, with hit
windows). The glide is 75..78. At Glide = KH2, KH2's glide was never drawn on KH1's glide, and
four of KH1's attack motions were drawn with KH2's glide instead.

### 1.6 How a motion becomes bones (decomp, for the tier B bake)

Read from the user's Ghidra export of the Steam exe (`DECOMPS/KH1 STEAM C DECOMP.c`) and
checked against `xa_ex_0010.mset`. Steam addresses first. The Epic one is in brackets: the
whole region sits 0x1B0 lower on Steam, and `steam_map.in` confirms this for the two functions
it lists.

**The pose pass, `0x140391C70` [`0x140391E20`] `(world, instance, time, motion)`:**

1. **World matrix.** The world matrix's row scales are split off (`0x142EF2F60..68`) and its
   translation is moved aside (`0x142EF2F50..58`). Rows 1 and 2 are negated (y-down), and the
   result goes into **helper matrix 0** (`0x142EF1F50`, 0x40 apart).
2. **New motion.** When the motion is not the one playing (`instance+8`), `0x14038F550`
   [`0x14038F700`] runs:
   - every bone's sampled channels (`+0x1B0`) are reset to the **bind** (the mdls bone table:
     scale `+0`, rotation `+0x10`, translation `+0x20`);
   - the motion's **constant channels** (`+0x34`/`+0x38`) are written over that;
   - every bone with joint-word **bit 25** has its local matrix built **once** here.
   A channel that has no keys therefore stays at the bind.
3. **Keyed channels.** Each keyed channel is sampled at `time × 256` (`0x14038F820`
   [`0x14038F9D0`]; key time `>> 8` is in 1/256 frames, as `kh1_data::mset` documents) into the
   bone's sampled slot:
   - kinds 1–3: scale x, y, z at `+0x00`;
   - kinds 4–6: rotation at `+0x20`, wrapped to ±π;
   - kinds 7–9: translation at `+0x30`.

   The channel's kind byte also says what happens outside the keys: bits 4–5 before the first
   key, bits 6–7 after the last; 2 repeats, 0 or 1 holds.
4. **Helper channels.** The helper channels are sampled into the helper table (`+0x30`, 40
   bytes each: scale `+0`, parent `+0xC`, rotation `+0x10`, translation `+0x1C`). Helper `k`
   becomes matrix `k`: its parent's matrix times its own (scale, Euler X→Y→Z, translation),
   for `k ≥ 1`. Entry 0 is unused, because matrix 0 is the world.
5. **Root travel.** The **last helper's** translation (H10 on Sora), times the world scale, goes
   to `instance+0x100..0x108`. `instance+0xF0..0xF8` gets the world position plus that. The body
   is not parented to H10. **This is where KH1's root travel lives, not bone 0.** Who reads
   `+0x100` / `+0xF0`: the actor, when the request has KH1's travel flag (t365, from the decomp):
   - a request entry's bit 15 becomes `0x4000000` (`0x1402A01E0`) and then the actor's `+0x160`
     bit 4 (`0x14029FC40`). Sora's ground combo (0xC8..0xCB) has it; his air combo doesn't;
   - the actor copies `+0xF0` to `+0x50` (`0x1403948D0`), keeping this frame's step in `+0x200`
     and the travel so far in `+0x210` (end of `0x14029F0A0`);
   - the move function (`0x14029A3C0`, which calls the ground step `0x1402B1850`) adds `+0x200`
     to his displacement, so the motion moves him, with collision;
   - the draw pulls his placement back by `+0x210`. At the motion's end, `0x14029FC40` puts it
     back and clears it;
   - KH1's own motions carry the same travel in H2, which the body hangs from (98..101: H10 z
     100 / 120 / 100 / 190), so the pull-back cancels and the model stays on him.
6. **Bones.** `0x1403900B0` [`0x140390260`] builds the bones (below).
7. **Events.** Event flags go to `instance+0x14` (types `0x10000..0x10003`, 0, `0x50000`,
   `0x70000`).
8. **Translation back.** The world translation is added back to every bone's matrix (`+0x1B4`).

**The bone pass, `0x1403900B0` [`0x140390260`]:** it walks bones in order. The parent comes from
the mdls bone table (`+0x2C & 0x3FF`; `0x3FF` = helper matrix 0). The motion's **joint word**
(`+0x14`, one `u32` per bone) decides how each bone is placed:

| Bits | Meaning |
|---|---|
| 26 (`0x4000000`) | **Skip.** The bone's world matrix isn't written. |
| 25 (`0x2000000`) | **Static.** The local matrix built at motion start (step 2) is kept. Keys on the bone are ignored. |
| 24 (`0x1000000`) | Parented to helper matrix 0 (the world), not to its parent bone. |
| 23 (`0x800000`) | Keep the position already in `+0x1B4`. |
| 21–22 (`0x600000`), mode | 0 FK. 1 a one-bone aim. 2 a two-bone IK with bend flag 0 (legs). 3 the same IK with bend flag 1 (arms). |
| 0–1 (FK mode), `lo` | 0 Euler X→Y→Z from the sampled rotation and translation (`0x1401DB430`: identity, then X, Y, Z, then the translation). 1 aim at helper `A`, rolled by the sampled rotation x. 2 copy helper `A`'s rotation. |
| 2–10, 27 (as bit 9), `A` | Helper index for `lo` (1-based: `A` = matrix `A−1`). |
| 11–19, 28 (as bit 9), `P` | Position from helper `P` (1-based). |

- **Mode 1 (aim):** bone `i` aims at helper `P` of bone `i+1`'s word, at bone `i+1`'s bind
  length. Bone `i+1` gets its bind offset.
- **Modes 2 and 3 (two-bone IK):** the solver is `0x14038FC20` [`0x14038FDD0`]. It poses bones
  `i` and `i+1`. The target is helper `P` of bone `i+2`'s word, the twist is bone `i`'s sampled
  rotation x, and the pole is helper `A` when `lo = 3`.
- **Ground lift:** with `instance+0x14 & 3`, a mode 2 bone listed in the model header's
  `+0x90..+0x9C` (bone + 1) casts down (`0x140193230`). If the ground is above the target, the
  target is lifted onto it, and `instance+0x14 |= 0x200000`. **This is KH1's foot IK.**
- **Head look-at:** with `instance+0x14 & 0xC`, `0x140391860` [`0x140391A10`] turns the head
  bone toward `instance+0xE0`, weighted by `+0xEC`. The head bone is the model header's `+0x20`,
  else `+0x10`. The bone pass clears bit 25 on that bone every frame, so the turn always lands.
  **This works in FK too.**

**Sora's rig, as the joint words show it.** The words agree across 107 of 118 motions; the
others differ only in helper numbers and the static bit. KH1 Sora is mostly **driven by 11
helpers**, not by bone keys:

| Bones | Word | What it does |
|---|---|---|
| 0 | `lo2 A4, P4` | Root: H3's rotation and position. H3 is the pelvis control under H2, the centre at 92.7 high. |
| 1 | `lo2 A6` | H5's rotation |
| 2 → 3 | mode 1; 3 has `P6` | Spine aims at H5 |
| 16, 17–18, 19, 20 | aim H9; mode 2; `P10` keep pos, world parent; copy H9 | One leg: thigh aims at the foot helper H9, two-bone IK to H9, the foot takes H9's turn |
| 42, 43–44, 45, 46 | the same with H8 | Other leg |
| 68, 69 → 70 | copy H6; mode 1 to H6 | Chest and neck to the head helper H6 |
| 92, 93–94, 95 | aim H4; mode 3; `P5` | Right arm (−x): two-bone IK to the hand helper H4 |
| 197, 198–199, 200 | aim H7; mode 3; `P8` | Left arm: IK to H7 |
| 280–292 | FK, static or keyed | **The Keyblade.** Grip 280, blade and chain 281–292. KH1's Keyblade mesh is skinned straight onto these (§1.4 of `kh1-weapon-second.md`), so it moves with whatever these bones do. |
| 9, 15, 25, 29, ... | skip | Tips and end bones. Never written. |
| the rest | FK, static or keyed | Fingers, hair, hood, face and cloth |

Helper parents: H1 is under the world. H2 (centre) and H10 (travel) are under H1. H3 (pelvis
control), H8 and H9 (feet, about 75 below the centre) are under H2. H4 and H7 (hands), H5 and
H6 are under H3.

**The blend path, `0x140392560` [`0x140392710`]** (`pose.rs`'s `pose_ik`):
- It blends two stored pose snapshots (`instance+0x1C4` / `+0x1C8`) by `param_3`.
- It then re-solves the aims and two-bone IKs in a list at `instance+0x1CC` (types 1 / 2 / 3,
  with targets kept in the list), with the same ground lift.
- **It doesn't write every bone** (Checked (decomp)). A bone off the IK list gets its
  matrices (`+0x1B8` unscaled, `+0x1B4` world) only when its bone-table word (`+0x2C`) has bit 30
  (`0x40000000`; the pass also sets it on each IK bone's parent); it is composed from the two
  snapshots up through the bones above it that lack the bit. Those bones keep whatever their
  matrices held: on Sora, the pose `pose.rs` drew last. 142 of Sora's 293 bones have the bit
  (Checked (files)).
- **Its last step moves every bone** (Checked (decomp)): when the world's row lengths are more
  than 0.005 off 1 it multiplies every bone's `+0x1B4` by that scale, then it adds the world
  translation to every bone's `+0x1B4`, placed or not. So a bone it skipped holds what was there
  moved by Sora's world position. The plain pass ends the same way (its scale step skips the
  bit-26 skip bones). In KH1 itself the skipped bones drift by one world position per blend;
  none carries a vertex, so it never shows. (Found for t405 / t411, whose code was taken out in
  tA with the rest of the sleeves thread after t375; the findings stand.)
- Which motion's IK that list holds while KH1 blends into or out of one of our slots is
  **open**. The check: log it in game while `+0x168` is our slot.

## 2. Adding slots

| Way | Verdict |
|---|---|
| **Grow the slot table.** A copy of the set's own slot handles plus ours, handed to KH1 when it binds the set onto Sora (t353, below). | **Yes.** Nothing bounds the slot by a count (`0x1401D40E0` counts but is never called). |
| **Grow or redirect the request table.** A copy of section 1 at `+0x1D8`. | **Yes**, and it is what makes this work (below). |
| A slot-lookup hook (`0x14029E050`). | Not needed: the tables do it with no code patch. |
| Motion data made in memory | **Yes**: a KH1-format motion on our heap, its offsets turned into handles with `0x14038B080` (§1.1). |

**Where KH1 binds a motion set, and how ours get in (decomp, t353).** The t349-t352 runs showed
the first way (a per-frame swap of Sora's actor's tables once they matched the file) never
applied: in a world Sora isn't on his own set.
- **His own set**, at his actor's creation: `0x1402894C0` → `0x140287E40 (actor, bundle, motion
  file)` (Epic `0x140285CC0`) sets `+0x1D0` the file, `+0x1D4` the slot list (the file's MMTN
  `+0x30`, `0x1401D6230`), `+0x1D8` the request table (section 1), `+0x1DC` section 2.
- **A world's set**, on entering it: `0x14029E850 (actor, file)` saves his set into
  `+0x1E0..+0x1EC` (if nothing is saved yet) and binds the world's into `+0x1D0..+0x1DC` with
  `0x1402886F0 (set, file)` (Epic `0x140286570`, from the neighbours, unchecked). Traverse Town's
  has about 88 slots: file slots 0-16, then 54-117 shifted down (the combo at 61-67), and 0xFFF for
  the requests it lacks. A request that resolves to 0xFFF brings his own back from `+0x1E0`.
- **In memory the slot list has no end marker**: the set's first motion follows it. Its count is
  the words before the lowest motion it points at.
- **Ours**: both binds are hooked. After KH1 binds a set onto an actor whose model has 293 bones
  (Sora), its slot and request handles are replaced with copies, made once per set: its own slots,
  ours right after; its request table, with each request our move stands in for pointed at ours
  (by style, every frame) where the set's own entry holds the KH1 motion ours stands in for (by
  signature). KH1's own save and restore copy the handles, so ours travel with them. Each frame
  also checks `+0x1D0` and `+0x1E0` (a bind before our motions were built). Our moves are known by
  their motion's address, since their slot numbers differ from set to set.

**Two ways a KH2 motion gets played:**
1. **Redirect** (a KH2 look on a KH1 move). A request KH1's state already makes (`0xC9`, the 2nd
   ground swing) is pointed at our slot, keeping its kind bits, while the move's style is KH2's.
   KH1's state, its attack row, its attack object (`+0x58 = 0xC9`), combomods, event scripts and
   the camera see the same request id. Only the motion is KH2's. With the style at KH1, the
   original slot.
2. **New request ids**, `0x200 + n`, past KH1's 321. These are for KH2-only moves (Aerial Dodge
   202, the ability moves). The table copy is long enough for them, so `+0x164` is a distinct id
   that matches none of KH1's: a raw-slot request (bit 29) would make `+0x164` the slot number,
   and 118+ are already request ids (`0x76` is a glide). Something has to ask for them: our
   decision layer calling `0x14029D7F0 (actor, 0x200 + n | flags, blend)` from a KH1 state,
   §4.

## 3. Turning a KH2 motion into a KH1 slot

### 3.1 The pose

KH2's rig has 228 animated bones of 256 (`P_EX100`). KH1 Sora has 293. Today
`kh1_data::retarget::Retarget::pose` measures KH2's body parts each frame and turns KH1's bones
by them:
- the limbs, aimed with their roll;
- the torso parts;
- the hands;
- the grip;
- the bones nothing turns, which hang as in KH1's battle idle.

`pose.rs` writes the result over KH1's pass output.

There are two tiers.

**Tier A, in the prototype: KH1 plays the slot, we draw it.**
- The slot's motion is a **KH1 template motion** (the KH1 motion KH2's replaces, e.g. slot 99
  for 152). Its key times are rescaled to KH2's length, so KH1's IK helpers, constants and joint
  words stay valid, and its length is KH2's.
- KH1 owns the slot: its clock, speed, queue, loop or hold, blend clock and events.
- `pose.rs` draws KH2's motion retargeted **at KH1's own time** (`actor + 0x16C` while
  `+0x168` is our slot), not by the wall clock. While KH1 blends into it (`+0x170` / `+0x174`),
  ours blends in from the last drawn pose by KH1's blend weight.
- The draw follows KH1's motion engine frame for frame, so the timing can't drift. KH1's attack
  objects ride on the bones we drew, so its hitboxes follow KH2's Keyblade.

**Tier B: a full bake (built at t349, `kh1_data::bake`, `kh1_bake = on`; untested in game).**

First build: the legs are plain FK like every other bone (no foot lift on slopes in these moves yet; tier A had none there either). The hybrid legs below are the next step once the raw bake looks right.

The user's rules: a raw bake first, behind an ini switch (`kh1_bake = on / off`; off is tier A).
None of our corrections go in (Keyblade teeth-up, `flip_weapons`, our feet, `held`). We test in
game, then decide.

- **Pose.** At load, each KH2 frame (60 Hz) goes through `Retarget::pose` plus `secondary`. The
  hair, hood, sleeves and chain are KH2's own hand animation, so they go in the bake like the
  body.
  KH1's locals are worked out (`parent⁻¹ × bone`, then Euler X→Y→Z, the order `0x1401DB430`
  builds) and written as **linear keys** (interpolation 1) per channel, reduced where a line
  fits within a tolerance. Channels that stay at the bind get no keys (step 2 of §1.6 resets
  them to it). Channels that never move become constant channels.
- **Joint words: plain FK.** For every bone KH1 doesn't skip, the word is 0: mode 0, `lo` 0, no
  helpers, bits 23–25 clear. Bones KH1 skips (bit 26) stay skipped. This turns off the
  helper-driven root, legs, arms, spine and neck for our slots only (joint words are per
  motion), so KH1 plays our keys exactly. The two-bone IK solves go away for these motions.
  The head look-at still works.
- **Helpers.** The template's helper table is kept (11 entries, the same parents), with one
  helper channel set: **H10's translation = KH2's root travel** at KH2's full distances (t367;
  t365 scaled it by the hip ratio), along the ground, for requests with the travel flag. That is where KH1 takes a motion's travel from
  (§1.6, step 5). The body carries the same travel, as KH1's own motions do through H2, so the
  pull-back cancels. The other helpers get no channels: no bone points at them any more.
- **The legs stay on KH1's IK (the hybrid).** KH1's foot ground lift acts only in mode 2 (§1.6),
  and it **is used on Sora**. In the user's t348 trace (Traverse Town, KH1's own animation,
  standing still on uneven ground, a bent leg seen in game):
  - `instance+0x14` was `0x00200001` on every row: ground lift on, and a foot target lifted;
  - the ankles were 17.4 and 29.8 above the floor under his centre (17.7 is the ankle's height
    on flat ground);
  - every pass was the plain one (`0x140391C70`), so the lift happens in the mode 2 path, not
    the blend path.

  So the legs keep their template words (16/42 aim, 17/43 mode 2, 19/45 position from H9/H8, 20/46
  H9/H8's turn), with **H8 and H9 keyed to the baked ankles** (position and the foot's turn). Bone
  16/42's aim and the IK's twist (the thigh's sampled rotation x) get worked out so the knees bend
  where KH2's do. Everything else is plain FK.
- **What `pose.rs` does for these slots: nothing.** KH1's own pass shows the motion. The
  Keyblade mesh rides 280–292.
- **Open, to check in game:** how the `+0x1CC` blend
  list treats an FK slot next to KH1's IK motions; the model header's ground-lift list for
  Sora.

Both tiers build in memory at load, from the user's install: KH2's `P_EX100.mset` and KH1's
`xa_ex_0010.mset`. Nothing is shipped.

### 3.2 Events

Every KH2 clip window or event (`character_core::Clip`) has a home:

| KH2 | KH1 event | Note |
|---|---|---|
| `Hitbox` window | 1.0 (hit window, attack group 0) | KH1's attack object for the request is live inside it. Its damage, reaction and hit effect stay KH1's (the row's data). |
| `Combo` window | 5.0 | KH1's next Attack chains in it. |
| `WeaponTrail` window | 7.0 | Else the hit window, from 4 frames before it. |
| `AllowMovement` | 0.0 | For KH2 moves KH1 must let go of early. |
| `Invulnerable` | none in KH1's events | Our hook, from the slot's own track (below). |
| `Sound`, `Voice`, `Effect` | **our track**, kept Rust side next to the slot (it could also be an event type KH1 never reads, as its readers ask only for types 0..7) | `kh2slots::tick` reads it between the last and the current KH1 time, like `0x14029D9A0`, and fires `kh2sound::event` / `kh2fx::event` for that motion. KH2's sounds and effects then fire from KH1's playing slot, on KH1's clock. |
| KH1's own sounds and voices (type 6) | kept from the template, times rescaled | The KH1 swing voice and swish. Dropped where KH2's own play (`kh2_sounds` covers the motion). |
| KH1's foot effects (type 4) | kept from the template, rescaled | |

### 3.3 Hits and effects on a hit enemy

The KH2 move's hits are now KH1's attack objects. The contact code `0x1402B1ED0 (attack entry,
body entry)` runs for every hit, with the enemy's body part. A hook there can tell:
- the attack object's owner is Sora;
- `+0x58` is a request whose slot is ours.

With that, it spawns the KH2 move's hit effect (ATKP's `effect`, or a spell's element) **at the
enemy's body part**, with `kh2fx` placed by world position rather than Sora's bone. That is what
custom magic lacks today: KH2's own hit test lands through a made-up attack object, and `kh2fx`
can only start elements on Sora's bones.

Not built yet: phase 4.

## 4. What changes in `takeover.rs` / `pose.rs`

- **No more taking KH1's Sora away to draw KH2's** for moves that have slots. KH2's controller
  stops being the owner and becomes a **decision layer**: at a press, it asks for a KH1 request
  (a redirected KH1 request, or a `0x200 + n` id) on the KH1 state that should own the move:
  - its ground or air action state;
  - the combo for swings;
  - the fall state for Aerial Dodge.

  A KH1 state that wants another KH1 motion simply plays it.
- **The physics stays with the state that owns the move.** KH1's ground step for the combo
  (`shared::physics::swing_travel` is unchanged: `pose::sora_slot` gives the *KH1 slot the
  request stands for*, 99, and `slot_frames(99)` the length that actually plays). The physics
  setting (`physics = kh1 | kh2`, `docs/kh1-physics.md` §7) sits on top of the same slot: KH2's
  swing travel with it at KH2, KH1's own travel at KH1.
- **`pose.rs`:**
  - `slot_of` knows our motions by pointer;
  - `sora_slot` gives the stand-in KH1 slot;
  - our slots are drawn by `Play::Slot` at KH1's time;
  - in tier B, not drawn at all.
- **The borrow path stays** (`kh1_slots = off`, Drive Forms, moves with no slot yet) until every
  KH2 move has one.

## 5. Phases

1. This doc. The glide slot fix (75..78).
2. **Prototype (built, `kh2slots.rs`; untested in game):** KH2's 2nd ground swing (152) as KH1
   slot 118, on request `0xC9` at Base Combo KH2, `kh1_slots = on`. Tier A: KH2's hit window,
   combo window and trail as KH1 events. KH2's sounds and effects are kept Rust side, not in
   the motion: `kh2slots::tick` reads them against KH1's time.
3. **Built (untested in game):** the rest of the ground combo, 151 on `0xC8` / `0xCA` and 154 on
   `0xCB`. With `kh1_slots = on` the slots are 118..121, in the order 152, 151 (for 98), 151 (for
   100), 154. `kh1_slots = 152` (a list of KH2 motions) builds only those, for testing one at a
   time.
   - **Next:** the glide (205..207 on `0x71..0x74` / `0x76..0x79`). It needs `Play::Slot` to
     loop with KH1's loop range (`+0x288..+0x28C`), and the template's length kept or the loop
     range set.
4. The contact hook (effects on the hit enemy). KH2-only moves through `0x200 + n`. Tier B.

## 6. Risks

- **The template's root.** The root (bone 0) and the IK helpers stretch with the template. Where
  KH1 moves him by the motion's root, the stretched KH1 travel is what moves him. Physics KH2
  gives it KH2's travel through `shared::physics`, which sees slot 99; physics KH1 leaves KH1's.
- **The `+0x164` checks.** Our redirect keeps KH1's request ids, so `+0x164` checks stay right.
  New ids (`0x200+`) are unknown to KH1's own code: any `switch` on `+0x164` treats them as
  "other".
- **The handle table.** It has 64 blocks of 32 MB. Our motions are one allocation (a few hundred
  KB): one block at most.
- **A world's own motion set** (`+0x1E0`) replaces `+0x1D4` / `+0x1D8` while it is used. Our
  tables go back on only when the actor shows Sora's own table again.
- **Tier A's blends:** tier A's draw is ours, so KH1's own blends of our slot (when KH1, not
  us, blends out of it) blend the template's pose. Tier B fixes that. Its joint words are read
  (§1.6). What remains open there is the `+0x1CC` blend list and the travel's reader.
- **Unverified in game:** everything in §1 is static reading of the code plus the user's files,
  except the request → slot proof from the s261 log (§1.3).

## 7. KH2's row selection on KH1's ground combo

The tester (`docs/tester-requests.md`): "KH2's row selection, so the right swing plays for the
target's distance". Until now each of KH1's ground requests played one fixed KH2 motion (0xC8 /
0xCA 151, 0xC9 152, 0xCB 154).

**How KH2 picks** (Steam KH2 exe, Checked; the rule is `character_core::combo::select`, from the
OG rewrite, Epic addresses there):
- The attack press runs the selector `0x140402D20`. It marks the rows that pass, then walks them.
- A row's conditions (`0x140402750`): a motion; its ability equipped (if any); air rows only in
  the air and ground rows only on the ground; a finisher row needs count (`+0xD74`) + its
  combo offset >= the combo length (`+0x5C0` block `+0x200` ground / `+0x201` air,
  `0x1403D7780`); a normal row is refused only once the count is at the length and more than
  one finisher has run (`+0xD75`, `0x1403D7850`).
- Range (`0x1404029F0` -> `0x1404027F0`), by the row's kind byte: 1 the primary target, 2 every
  targetable object (`+0x6C8` bit 17), 3 no range. Gap = distance less the target's radius,
  never below 0 (`near <= gap < far`); the height window (flags 2 / 8 / 16); the facing window
  (`inner_min < dot <= inner_max`, dot capped at 1).
- The walk (`0x140402C50`): the first marked row whose `sub` is its own id or whose motion isn't
  the one playing (`+0x180`); a marked row whose motion is playing is passed over, the first one's
  `sub` row kept for when nothing else passes. Nothing at all: row 9 (air) / 5 (ground) is marked
  and the walk runs again.
- On your install (the tester's logs): no target, 151 <-> 153; KH2's finisher at close range is
  row 3, motion 155 (A304). Motions 151..155 are A300..A304.

**How KH1 picks** (Steam KH1 exe, Checked): the Attack step (`0x1402A7510`) counts the press
(`+0x14C` info `+0x41`: 1 outside a combo window, +1 inside one when the swing connected) and asks
the ground picker (`0x1402A6EE0`; air `0x1402A6C70`). It marks KH1's own rows too: finishers
when the count reaches the stats' `+0xD4` (row 8, or rows 3 / 4 / 5 / 7 by KH1 ability bits and a
roll), a dash at a target 160 or more away (rows 0 / 1), rows for a target in a height band
(9, 10; 14 is its third plain swing, request 0xCA, marked for a target ahead), row 2 when its
sweep ahead finds no floor (below), then takes the first marked row whose request isn't
the one playing (`+0x164`); with none, rows 13 / 11 by the motion playing (its plain swings,
requests 0xC8 / 0xC9). The row's request runs through the request table, which is where our
slots go in. (Corrected after t374: row 2 is not a row for a target above.)
- KH1's row runner (`0x1402A68F0`, Steam, Checked; the hook `combat::exec`): rows 0 / 1 lunge by
  the dash distance the picker left in the direction's y (`0x1402AF960`; the plain path leaves 0
  there) and queue request 6 (the fall) to follow (`0x14029FA30`, the motion queue: bit 30 and
  the request); row 2 sets the direction's `+0xC` to 8 (the move speed the state's step takes,
  `0x1402B18F0` copies the direction into `+0x20` / `+0x2C`) and queues the fall; the plain
  swings and finishers (default case) queue request 0 (idle) and lunge by the row's own `+8`.
- KH1's row 2 (`0x1402C6E70`, Steam, Checked): Sora's collision moved along the direction for his
  `+0x2C0` plus 20, in steps (`0x140195180`); row 2 is marked when a step's result has bit 0 of
  the collision's `+0x30`, which that mover sets when no floor is found within reach under him
  (Checked in the decomp). So row 2 is KH1's attack at a drop ahead: it walks him off it, then
  his fall. KH2 has the same test for its row 1 (below).

**Sora's PTYA table** (moveset 1 of `00battle.bin`, the staged file, Checked (files); 37 rows, in
table order; "any" = kind 2, every target, measured from the primary target's direction; "primary"
= kind 1, whose facing window is 0.99996..1, i.e. dead ahead, which the selector makes true
because it faces the primary target, `0x1404028D0`; "none" = kind 3, no range; gaps are distance
less the target's radius):
- Ground hops at a target well above (target about 110 to 250 up, its height widening it): Aerial
  Sweep (33, primary, 0..260), Aerial Dive (48, primary, 0..650), and the plain one, row 0 (motion
  182, primary, 0..260, hop 17..67 high, speed -8..8).
- Ground finishers: Explosion (26, any, 0..300, facing -1..0.92), **row 2 (154, any, 0..180,
  facing -0.71..0.92)**, Guard Break (27, none), Explosion (42, none), **row 3 (155, none, sub
  2)**.
- Ground swings: Vicinity Break (51, any, 0..300, behind: facing -1..-0.17), Flash Step (46,
  primary, 180..400), Dodge Slash (44, primary, 180..240), Slide Dash (28, primary, 180..650),
  **row 6 (153, primary, 180..300)**, Dodge Slash (34, any, 0..180, facing 0.17..0.92), **row 4
  (152, any, 0..180, facing -0.71..0.92)**, Slapshot (35, primary, 0..120), Flash Step (47,
  primary, 120..400), Dodge Slash (45, primary, 120..180), Flash Step (59, any, 180..400, facing
  0.97..1), Slide Dash (43, any, 180..650, facing 0.97..1), **row 5 (151, none, sub 6: the
  default)**. Every ground row's height window is the same (-20..-110 before the target's height:
  a target on about his level).
- Air finishers: Magnet Burst (49, any, 0..650), **row 7 (184, any, 0..260, facing
  -0.71..0.92)**, Aerial Finish (29, none), Magnet Burst (50, primary, 300..650), **row 8 (183,
  none, sub 7)**. Air swings: Aerial Spiral (36, primary 0..650 any height, and again any
  300..650), **row 10 (182, primary, 0..260, a target above him**: flags 9, its window's low
  end -70 less the target's height, so the target at least 70 plus its height above his feet;
  corrected after the t409-era traces, it was written as below**)**, **row 9 (181, none, sub 10:
  the default)**. The air rows hop too (jumps 14..21).
- By id only (kind 0): **row 1 (181, a hop of 21: the drop ahead, below)**, Guard (11), Upper Slash
  (12), Finishing Leap (37), Horizontal Slash (38), Counterguard (39), Retaliating Slash (40).
- So the plain ground combo is: another target within 180 off the primary target's line (rows 2 /
  4 are kind 2, their facing window measured from the direction to the primary, which itself is
  at 1 and never passes): 152 / 151 alternating, finisher 154; the primary 180..300 away: 153
  (row 6) / 151; otherwise 151 / 153, finisher 155 (the tester's t374 traces with one enemy: 151
  / 153 / 155 only). The slots t374 builds (`kh2slots::swing_rows`: 151, 152, 153; 154, 155) are all the plain ground
  rows; the plain ones not built are the hops (rows 0, 1) and the air rows.

**Ours (Base Combo KH2, `kh1_slots` on):** at the press KH1's combo takes (`combat::exec`,
`combomods::attack`), KH2's selector is asked at KH1's count, with KH2's motion KH1 plays now as
the one playing (`Character::would_take_after`, so the anti-repeat works; before, the trial copy
never had a swing playing and always gave row 5). When KH1's row is one of its plain ground swings
(requests 0xC8..0xCA) or its plain finisher (0xCB) and KH2's pick fits it (a ground row, no hop,
no ability, a swing for a swing and a finisher for the finisher), that request is pointed at our
slot of the picked motion (`kh2slots::pick`) before KH1's row makes it. KH1 plays it: its state,
row, hits and damage, the KH2 motion's hit / combo / trail windows as the slot's events. Slots
for every such motion of KH2's rows (`kh2slots::swing_rows`) are built for each request, on the
request's own template. No new hook; nothing per frame beyond the redirect already there.
- **KH1's own rows give way** (the tester: "do option (a)", KH2 1:1; `combomods::kh2_row`): when
  KH1's ground picker took a row that isn't the kind of KH2's pick (its dash, rows 0 / 1; its
  hop, 2; its rows 9 / 10; its ability finishers 3..7; a swing where KH2 finishes or the other
  way round) and KH2's pick is a plain ground swing or finisher with a slot, `combat::exec` runs
  KH1's plain row of that kind instead: swing row 13, or 11 when 13's request is playing (KH1's
  own fallback order, so the request restarts), or finisher row 8; after a dash row the
  direction's y (the dash distance) is put back to 0. That row's request then plays KH2's motion
  as above. KH1's dash, hop and ability finishers no longer come out at Base Combo KH2; KH2's
  hops (rows 0 / 1) and ability rows aren't KH1 rows and are left to their own handling (the
  ability rows borrow, `combomods`). Seen in the tester's t374 trace f27098: KH1's dash
  (request 0xD0, about 280 units) where KH2 took row 5.
- **The air combo** (the tester: "the same KH2 row picker on the air combo"; Aerial Combo KH2):
  KH1's air picker (`0x1402A6C70`, Steam, Checked) marks rows 15..18: 18 its plain swing (no
  target, or one not more than 100 above him), 17 for a target more than 100 above (KH1's y is
  down: the target's y less his at -100 or more marks 18; corrected after the t409-era traces,
  it was written as below), 16 its finisher
  (the count at the stats' `+0xD5`), 15 a finisher with KH1 ability bit 25; it takes the first
  marked row whose request isn't playing, else 18 down to 15 by the same test. The row runner
  gives rows 15..18 a sideways speed along the direction (`+0x3C0`: 8 on the combo's first
  swing, 12 for 15 / 16, else 4), its hop (the row's `+8`, `0x1402AF960`) and queues the fall
  (request 6) to follow. Now, as on the ground: KH2's pick at the press (its air rows, `would_take`
  in the air) plays on KH1's air requests (0xCC / 0xCD the swings, 0xCE the finisher), each of
  which has a slot for every plain air row's motion (181, 182; 183, 184; `swing_rows` with
  `air`); KH1's row is swapped for its plain air row of KH2's kind when it isn't (swings 18 /
  17, finisher 16 / 15, each taken only if its request, read from KH1's row table, is that
  kind and isn't playing). KH2's air rows with an ability stay borrows. With no target: 181 /
  182 alternating (row 9's `sub` 10), finisher 183 (row 8), 184 after it with Finishing Plus;
  finisher 184 (row 7) only at a target within 260 off the primary target's line (a kind-2 row's
  facing window is measured from the direction to the primary target, so the primary itself, at
  1, never passes its -0.71..0.92); a target above him in reach: 182 (row 10).
- **KH2's hops** (the tester: "design and implement natively (KH1's air state with KH2's arc and
  numbers)"; Base Combo KH2, `combomods::kh2_hop`): KH2's plain hop rows are row 0 (motion 182,
  a target well above) and row 1 (181, the drop ahead, item 4); both play KH2's air swings and
  queue its fall (`next_motion` 4). At such a pick KH1's air state has him: `combat::exec` sets
  KH1's state 2 (`0x1402B1960`, Steam, Checked: state 2 has no enter or exit step,
  `0x1402B1B80`'s table; the hand-backs already do this, `takeover`, `combat::command`), KH1's
  count to 1 (KH2's: a hop from the ground restarts it), runs KH1's air swing row (18, or 17
  when 18's request plays) with its request pointed at our slot of KH2's motion, then puts
  KH2's hop on KH1's fields (`shared::physics::kh2_hop`): the height and sideways velocity KH2's
  controller works out for that target (`Character::would_take_launch`, `Launch`; KH2's rule,
  below), as KH2's arc on KH2's gravity (physics KH2, carried on by
  `air_step` as KH2's jump is) or KH1's rise of that height (physics KH1). KH1's air combo goes
  on from there. Needs the Aerial Combo at KH2 too (else KH1's row is kept, logged). Not checked:
  in game (that KH1's air row runs as expected right after the state change; the hand-backs do
  the same before KH1's own air attack). Ability hops (Aerial Sweep 33, Aerial Dive 48) stay
  borrows.
  - **KH1's carry into the air** (Steam KH1, Checked (decomp)): KH1's control step
    (`0x1402A3300`) runs the command (the Attack step, our row) and then the state's step; on a
    frame the state changed (flag `0x400`, set by `0x1402AF7D0`) with state 2 it sets the air
    velocity `+0x3C0` to the move direction `+0x20` times its speed `+0x2C`, which the air step
    (`0x1402AFC20`) has just copied from the stick. That is KH1 carrying the run into a jump;
    for the hop it replaced KH2's sideways speed (standing: 0). Checked (trace): the tester's
    t409 traces f3990, f4996, f5299, f5413, every hop from the ground at a ledge (row 1, 21 high,
    8/frame logged) had `+0x3C0` 0 from its first frame and rose and fell in place. Since the
    hop's patch: the velocity `kh2_hop` set is put back on Sora's next physics step, before
    KH1's (`shared::physics::ground_hop_carry`), so KH1's step moves him by KH2's from the first
    frame. The air rows' own hops (no state change) never had it.
- **KH2's hop rule** (Steam KH2, Checked (decomp); t392 had the OG rewrite's, which differed):
  after the pick, `0x1403A76B0` / `0x1403A8690` (the Attack press, both) run `0x1403D78D0 (him,
  row, his target)`, `+0xDC8`'s target: the primary target, also for a kind-2 row passed on
  another target. With the row's `jump` (`+0xC`) above 0: the count and finisher count go to 1 /
  1 unless he is in the air (`0x1403BA9A0`); then, with a target (`0x1403BDE30`):
  - `0x1403D7520`: the vector from his own target shape's centre to the target's (each its
    model's first kind-3 shape, `0x1403B5650 (…, 3, -1)` / `0x1403BE0F0`, placed by
    `0x1403C9C00`: the shape's offset on its bone, or on his root with bone 0x4000, raised by
    the shape's lift `+0xE`) and the sum of the two radii (`+0x10`). Sora's kind-3 shape
    (`P_EX100.mdlx`, Checked (files)): on his root, 70 up, radius 55 (`kh2_data::sora::
    target_shape`, `AttackParams::target_lift` / `target_radius`). Ours: the target's centre is
    KH1's lock point (as for the range checks), its radius its parts' largest.
  - Height: that vector's rise, bounded by `+0x14` below and `+0x10` above (plus his jump height,
    `0x1403B5A40`, with flag 2), in that order (`+0x14` when under it, `+0x10` when over). So the
    PTYA's `+0x10` is the upper bound and `+0x14` the lower (`PtyaEntry::jump_max` /
    `jump_min`, Checked).
  - Speed: the flat distance less both radii plus the row's `distance_adjust` (`+0x3C`; Sora's
    hop rows 85, not used by the OG rule; not held at 0) over `0x1403B73F0 (h)` =
    `sqrt(|2h / 0.40816328|)`, KH2's own gravity constant, not the character's (so at physics
    KH1 too); bounded by `+0x18` below and `+0x1C` above in the same order.
  - No target: `jump` and `+0x1C` along his facing (`0x1403B56E0`).
  - The class's method `+0x18` with (direction, height, speed): for the player classes
    `0x140423140` / `0x140423170` -> `0x1403CBB00`: the arc's rise time for that height on the
    same constant, the velocity the flat direction times the speed, then state 0x14.
    (Assumption: Sora's class is one of those; several classes share this pair.)
  Sora's hop rows (PTYA, Checked (files)): row 0 / 10 (182) 17..67, speed -8..8; rows 1 / 9 (181)
  21..71, -8..8; rows 7 / 8 (184 / 183) 21..71, 12..18; all `distance_adjust` 85. Before (OG):
  height measured from his feet, the gap less the target's radius only, at 0 at least, no
  `distance_adjust`, over the rise on the character's gravity; with a 120-high target 200 away
  (radius 30) row 0 went 67 high at 8 / frame, KH2's is 50 high at 8 (both capped there).
  Not done: the ground rows' lunge path (`jump` 0) is the same function's other half (a class
  flag or `+0x18C` bit 6, the speed `(gap + distance_adjust) x` the lunge factor); Sora's ground
  rows have no speed window, so it is left as it was.
- **KH2's per-row hop on the air swings** (the tester: "(a)", 1:1; Aerial Combo KH2,
  `combomods::air_hop`): every row with `jump` above 0 starts KH2's hop when it is used, in the
  air too (`0x1403D78D0`, the count kept there), so each of KH2's air swings rises and moves by
  its own row (KH2's hop rule above): rows 9 / 10 (181 / 182) 21 / 17 high (no target), speed
  -8..8; finishers 7 / 8 (184 / 183) 21 high, 12..18. When KH2's pick is a plain air row and
  KH1's air row plays its motion (`select_swing`, our slot on 0xCC / 0xCD / 0xCE), `combat::exec`
  runs KH1's row, then puts KH2's hop (`Launch`, from the same trial as the pick) on KH1's air
  fields (`shared::physics::kh2_hop`, as for the hop rows from the ground, without the state
  change: he is in state 2 already), in place of KH1's row's own (`+0x3C0` 8 / 12 / 4 and its
  `+8` hop). With no slot for KH2's motion KH1's swing keeps its own. Physics KH2: KH2's arc for
  that height on KH2's gravity; physics KH1: KH1's rise of that height (`kh1phys::Arc::hop`).
  Not done: a press in KH1's state 1 (its take-off frames; `kh2_hop` needs state 2) keeps KH1's
  row's own; a KH1 air swing started out of KH2's controller (`combat::command`) has KH1's
  momentum rules after it (`kh1_air_attack_started`, `seed_kh1_lunge`), left as they are (the
  tester: the KH2-controller path is going away). Unverified in game.
- **KH2's ability swings on KH1's ground combo** (the tester: "transfer everything into kh1's
  systems"; Base Combo KH2): an ability row whose motion is only a Keyblade swing plays on KH1's
  ground swing rows as the plain swings do, no borrow (`combomods::native_ability`,
  `kh2slots::blade_swing`). The test, on KH2's clip (Sora's `P_EX100.mset` ranges, Checked
  (files)): a hit window on the Keyblade, none on his own body (range id 10 / 33 on his mset), no
  guard window (an ATKP of kind 2 / 3), no invulnerability (27), no gravity / air / ground range
  (3, 24, 1, 0); a ground row, no hop, not a finisher. Sora's: Slapshot (row 35, motion 162),
  Dodge Slash (34 / 44 / 45, 163), Slide Dash (28 / 43, 164). Their PTYA rows have no speed
  window and no `distance_adjust` (Checked (files)), so KH2 moves him by the motion's root travel
  only (Slapshot 50 forward, Dodge Slash 300, Slide Dash 700, whatever the target's distance;
  Checked (files)), which the slot carries (baked into its H10, KH1's travel flag on requests 0xC8..0xCB). Each
  is built for 0xC8, 0xC9 and 0xCA (KH1's plain swing rows 13 / 11 and its row 14), about 170 KB
  and 15 ms each at load (9 more slots). Not native yet (still borrows): Flash Step (169; below),
  Vicinity Break (170: a body attack, 1869 group 21,
  power 125), Explosion (166: 911 / 912), Guard Break (165, a finisher; its KH2 guard break isn't
  KH1's attack data), Magnet Burst (197; Aerial Spiral 192 and
  Aerial Finish 194: below), the Square moves, Counterguard and Retaliating Slash.
  What stays KH1's for these, as for the plain swings: the row, its attack object on the
  Keyblade (KH1's shape and damage for that row, not KH2's ATKP: Slapshot's, Dodge Slash's and
  Slide Dash's own power and reactions are not used), its combo count and window from the slot.
- **Aerial Sweep from the ground** (the tester, t410's question: "when we transition that move to
  native kh1 it will work the same"; Base and Aerial Combo KH2): row 33 (motion 191, ability 97,
  PTYA Checked (files): primary, gap 0..260, a target 110 to 250 up, hop 17, bounds 17..117 plus
  his jump height (flag 2), speed -8..18, `distance_adjust` 55) is a hop whose motion is only a
  Keyblade swing (no body hit, no gravity range: its ranges are ids 11 and 14), so it runs as
  KH2's plain hop row 0 does (`combomods::kh2_hop`, no borrow): KH1's state 2, its count at 1,
  its air swing row 18 (or 17) with our slot of 191 (now built for 0xCC and 0xCD, about 170 KB
  each), KH2's height and speed for that target from `Character::would_take_launch` (KH2's hop
  rule above, with row 33's own numbers) on KH1's air fields, KH1's air combo after it. It wins
  over row 0 where both pass, as in KH2 (it comes first in the table). With the Aerial Combo at
  KH1 it is still a borrow. Aerial Dive (48, 196): below.
- **KH2's no-gravity ranges on KH1's air state, and Aerial Dive** (the tester: "transfer
  everything into kh1's systems"): KH2's motions hold him in the air with range id 3 (and 24,
  `WindowKind::NoGravity`); KH2's controller (the OG rewrite, `Character`'s action step) then
  sets no fall speed and doesn't move the arc on, so the rise or fall goes on from where it was
  after the range, the sideways drift unchanged; not for a clip that allows movement all through.
  KH1's air state has no such range, so it is added on KH1's own step: `shared::physics`'s air
  step (state 2, `0x1402AD510`, already wrapped) asks `kh2slots::no_gravity` (our slot playing,
  KH2's clip, KH1's time in the slot as the clip's 60 Hz frame, the slot being retimed to the
  clip's length) and holds him: physics KH1, KH1's step's drop undone and its arc's time and
  start (`+0x3B0`, `+0x3B4`) put back, `+0x3C4` 0; physics KH2, KH2's arc not advanced. Not with
  KH1's own hold (flag `0x1000`). No new hook; a few reads a frame in state 2 (none when no slot
  of ours plays). **Assumption**: KH2's exe does what the OG rewrite does in that range (not read
  in the decomp), and KH1's time for the frame is the one KH1's motion step has reached when the
  air step runs (at most a frame off). First move on it: Aerial Dive (row 48, motion 196, ability
  178; PTYA Checked (files): primary, gap 0..650, a target 110 to 250 up, hop 41, bounds 41..117
  plus his jump height, speed 0..32, `distance_adjust` 105; no gravity at frames 46..54, Keyblade
  hits only), now native as Aerial Sweep is (`kh2_hop`, slots on 0xCC / 0xCD): KH2's hop for that
  target on KH1's air state, then the hold near the end of the dive.
- **Aerial Spiral and Aerial Finish on KH1's air combo** (the tester: "transfer everything into
  kh1's systems"; Aerial Combo KH2): KH2's air ability rows whose motion is only a Keyblade swing
  (`kh2slots::blade_swing`, now with no-gravity ranges allowed, KH1's air step holding them)
  play on KH1's air rows as the plain air rows do (`combomods::native_ability`, `Kh1Combo::plain`,
  `kh2_row`, `select_swing`, `air_hop`), no borrow. PTYA and ranges (Checked (files)): Aerial
  Spiral, row 36 (twice: primary 0..650 at any height, and any target 300..650 dead ahead), motion
  192, ability 98, hop 14 (14..114), speed 4..18, no gravity 16..24, a swing (built on 0xCC /
  0xCD); Aerial Finish, row 29, motion 194, ability 100, kind 3, a finisher (built on 0xCE), hop
  14 (14..114), speed 0..18, `distance_adjust` 55, no gravity 16..100 (the hover before the last
  blow), its finisher window 89..108, its effects (frame triggers 1: elements 35..38) fired by our
  slot as KH2's. KH1's air row (18 / 17, finisher 16 / 15) runs, its own lunge and hop replaced
  by KH2's hop for the row (KH2's hop rule above), then the hold in the range. The air motions
  have no root travel (all 0: 181..184, 191, 192, 194, 196; Checked (files)). Still borrows: Magnet
  Burst (197: body attacks 1900 / 1901 / 1903 and a guard), Finishing Plus's next finisher out of
  KH1's (the press in KH1's finisher), the air Square move (Horizontal Slash, 193, a Square row).
- **KH2's guard on his body in KH1's states, and Flash Step** (the tester: "transfer everything
  into kh1's systems"; Base Combo KH2): Flash Step (rows 46 / 47 / 59, motion 169, ability 177;
  PTYA Checked (files): primary 180..400, primary 120..400, any target dead ahead 180..400, no
  speed window) is a Keyblade swing with 400 of root travel and a guard on his body at frames
  0..19 (range id 10, ATKP 406: kind 2, `refact_self` 2; Checked (files)). KH2's controller, and
  `hurt` while it has him, takes a hit inside such a guard as no effect with no reaction of his
  own (`refact_self` 2; 1 is the guard repel), the move going on, all round for a guard on his
  body. Now the same in KH1's states: KH1's hit pass (`0x140299A40`, already hooked by `hurt`)
  asks `kh2slots::body_guard` (our slot playing, its KH2 clip's guard windows at KH1's time) and
  marks each attack's hit record no-effect (`+0` bit `0x20`, as `hurt` does for KH2's guard)
  with KH1's clash spark (`combat::clang`), so KH1's damage, reaction, voice and hit count for it
  don't run. Only `refact_self` 2 guards are taken this way; a motion with another kind of guard
  is still not native (`kh2slots::blade_swing`). Flash Step then plays on KH1's ground swing rows
  as Slapshot does (above). Needs the ini's `hurt` on (as KH2's own guard does). KH2's turn
  toward the target in a swing: below.
- **KH2's turn toward the target in a swing** (the tester: "(a)"; Steam KH2, Checked (decomp)
  and (exe)): KH2's range handler (`0x14040D810`, one case per range id) runs range id 28
  (`TurnToLock`, its parameter in degrees) each motion update while the motion's time is in it
  (`0x14040E740`: start <= t < end) through `0x1403D4B10`: with a primary target (the player's
  `+0xDC8`), `0x1403D3A90` takes the flat line from his position (`+0x670` plus `+0x690`) to the
  target's point (`0x1403BDDE0`, its kind-3 shape) and, if longer than 50, turns him toward it
  (`0x1403B9F70`) by at most **twice** the parameter (radians) an update (the OG rewrite turned by
  the parameter once a 60 Hz frame, half the exe's at 60 fps); it also sets `+0x194` bit 3. One
  update a game frame (delta 2 at 30 fps). Range 22 (`TurnToTarget`) turns toward his own
  `+0xBF8` the same way (none on Sora's swings). Sora's (`P_EX100.mset`, Checked (files)): 151 /
  152 frames 0..12 at 15 degrees, 153 0..12 and 32..44 at 15, 154 0..8 at 22, 155 0..4 at 45
  and 32..38 at 5, Slapshot 0..8 at 22, Dodge Slash 0..12 at 15, 26..30 and 44..48 at 22, Slide
  Dash 0..8 at 22, Flash Step 0..8 at 22 and 18..22 at 45, Vicinity Break / Counterguard /
  Finishing Leap 0..8 at 22, 181 0..23, 182 0..22 at 15, 184 0..10 at 18 (183 none), Aerial
  Spiral 0..8 at 22, Aerial Finish 0..80, Aerial Dive 0..43, Magnet Burst 0..12 at 15.
  Ours (`kh2slots::turn_to_lock`, from `takeover::physics` before KH1's physics frame, KH1
  having him): our slot's KH2 clip at KH1's time in the slot, the target as KH2's controller
  picks it (`takeover::kh2_primary_target`: KH1's lock, else KH2's auto target, the ini's
  `auto_aim`), and KH1's facing (`+0x30`, yaw `+0x3C`, the yaw it turns toward `+0x150`) turned
  by at most twice the parameter a game frame. KH1's row still faces the target as it starts.
  Cost: his targets looked up on the frames inside such a range only. Not checked in game;
  **assumption**: `+0x690` is 0 in a swing (it is cleared outside `+0x120` bit 4).
- **Combo Plus and Negative Combo on KH1's rows** (the tester: "transfer everything that is not
  on the new system yet"; Base / Aerial Combo KH2): KH2's selector is asked at KH1's count with
  KH2's combo length (its Combo Plus / Negative Combo, `0x1403D7780`, Checked (decomp)), so its
  pick past KH1's length is a swing where KH1's picker took its finisher, and its finisher before
  KH1's length one where KH1 took a swing. Those were borrows (`combomods::ability_move`). Now,
  when KH1's plain rows serve the pick (`Kh1Combo::serves`: the combo style KH2, a plain row, a
  slot of its motion on a request of its kind), `kh2_row` runs KH1's swing row (13 / 11, air 18 /
  17) or finisher row (8, air 16 / 15) of KH2's kind with our slot, no borrow; KH1's count goes
  on as KH1's (one more a connected press), which is the count KH2 is asked at, so KH2's length
  decides where the finisher comes. KH1's last swing is taken as its finisher by its request
  (0xCB / 0xCE), no longer by the count, for the Finishing Plus press. At the combo style KH1 the
  borrows stay (the open question in the thread: which logic picks at KH1's style).
- **KH2's ability moves at the combo style KH1** (the tester: "transfer everything that is not on
  the new system yet", the Base / Aerial Combo KH1 cases): with the style at KH1, KH1's own
  combo plays (its rows, motions and picks), and KH2's selector was asked only for its ability
  moves, which borrowed him. Now the ability moves KH1's rows play natively at KH2's style do so
  at KH1's too (`combomods::native_ability` without the style test, `Kh1Combo::here`): Slapshot,
  Dodge Slash, Slide Dash, Flash Step on KH1's ground swing rows; Aerial Sweep and Aerial Dive
  as KH2's hop on KH1's air state (`kh2_hop`), KH1's own air combo after it; Aerial Spiral and
  Aerial Finish on KH1's air rows with KH2's hop (`air_hop`). KH1's request is pointed at our
  slot for that one move (`kh2slots::pick_ability`, a forced pick: the request table entry is
  ours although the style is KH1's; every other forced pick dropped at the next press, and the
  forced one 8 frames after none of ours plays), so the swings around it are KH1's own motions.
  KH2's plain picks (its row selection over KH1's plain rows, Combo Plus / Negative Combo) stay
  out at KH1's style; Combo Plus / Negative Combo there are still borrows. Not checked in game
  (`pose::follow` draws our slot's KH2 motion whatever the styles, baked or not).
- **KH2's hits on his body on KH1's rows: Vicinity Break, Explosion, Magnet Burst** (the tester,
  m14 question 1: "(a)", KH2's exact shapes and hit numbers landed through KH1's damage, and
  KH2's invulnerability, range id 27, into KH1's hit pass): these three hit with shapes on his
  own body (range id 10 with a body group; `P_EX100.mset` and `00battle.bin`, Checked (files)):
  Vicinity Break 170 (row 51, a ground swing: ATKP 1869 on group 21 at frames 20..30, power 125,
  reaction 3, knockback 150; guard 406 on his body 14..32), Explosion 166 (rows 26 / 42, a ground
  finisher: 911 on group 19 at 32..57, power 25, reaction 9, re-hit every 8; 912 at 57..64, power
  150, reaction 11; both Magic-based; invulnerable 32..64; guard 124, kind 3, `refact_self` 2, at
  32..64), Magnet Burst 197 (rows 49 / 50, an air finisher: 1900 on 22 at 16..56, reaction 5, the
  pull; 1901 on 23 at 20..56, power 25; 1903 on 3 at 64..70, power 150; guard 406 20..70; no
  gravity 16..82). KH1's attack object is one sphere on the Keyblade, so these were borrows.
  Now they are built as slots like the Keyblade-only ability moves (`kh2slots::body_strike`:
  hits on his body allowed, with guards on his body that let the hit go, invulnerability and
  no-gravity ranges; not with another guard or an air / ground range): 170 on 0xC8..0xCA, 166 on
  0xCB, 197 on 0xCE (about 170 KB each). KH1's rows run them as they run the others (`kh2_row`,
  `select_swing`, `air_hop`; KH1's row, request, count, combo window and travel), with:
  - **The hits** (`takeover::kh2_hits`, once a game frame from `kh2slots::frame`): while our slot
    plays, the clip's body hit windows open at KH1's time in it (the clip frame) are landed as
    KH2's controller's hits were (`combat::Strikes::strike_as`, the same code): KH2's rig posed
    at that frame where KH1 has him (`RigAnim::pose_at`: his position, KH1's yaw as KH2's facing,
    the clip's root travel taken out, KH1 having moved him by it), the window's group of KH2
    shapes swept from last frame's against KH1's enemies' body parts, each hit through KH1's
    contact (`0x1402B1ED0`) with Sora's KH1 swing or finisher row data and KH2's ATKP (power,
    Magic scaling, launch, the hold, Magnet's pull), once a window and target as KH2's hit
    records allow, their re-hit clocks moved on by the frame's 60 Hz steps (KH2 updates once a
    game frame, delta 2 at 30 fps). A landed hit sets the combat info's `+0x50` (KH1's "the swing
    connected", which its Attack step reads for the next press). Air or ground: KH1's state.
  - **KH1's sphere** (`kh2slots::kh1_tracks`): KH1's hit window is now made from the clip's
    Keyblade hit windows only (weapon > 0); the body ones are the hit pass's above. No change for
    the moves before (none has a body window; Checked (files) for P_EX100's side).
  - **Invulnerability** (`kh2slots::invulnerable`, `hurt`): with range 27 open at KH1's time,
    every record of KH1's hit pass on him (`0x140299A40`) is marked no effect (`+0` bit `0x20`),
    no spark, before the guard test (KH2's order: no hurt shape, no hit).
  Cost: nothing while no body window is open; with one, a pose of KH2's 228 bones and the
  enemies looked up once a game frame. Not checked: in game; the moves' Keyblade windows (the
  weapon's mset is not staged: what they hold is read at run time); KH2's guard windows on his
  body make no KH1 guard object here (as for Flash Step, `body_guard` takes the hits on him).
- **KH2's Square moves and Guard Break on KH1 rows of their own** (the tester, m14 question 2:
  "(b)", new KH1 rows and requests from 0x200 up with their own data, so they keep KH2's
  behaviour; `kh1rows`):
  - **KH1's attack rows** (Steam, Checked (decomp) and (exe)): one table a party character,
    `0x142D5CAB0[object +0x4C]` (10 pointers, set once from the offsets at `0x140528B80` by
    `0x1402A3230`, called from `0x14028AC80`); rows of 0x14: `+0` the request (i16), `+2` trail,
    `+3` effect, `+4` the attack data's offset in the battle data block `0x142D22D40`
    (`0x14028F950`), `+8` the lunge, `+0xC..+0xE` the attack object's effect / sound bytes, `+0xF`
    a voice. Sora's is the block's `+0xA9F0`, 66 rows (the next at `+0xAF18`; Checked (exe)).
    The row start (`0x1402A3980`) asks for the request, makes the attack object from the data
    (`0x1402B3270`) and plays the trail, effect and voice; the runner (`0x1402A68F0`) has cases
    for KH1's own indices and, for any other, queues the idle (`0x14029FA30 (actor, 0x40000000,
    0)`), lunges by `+8`, faces the direction (`0x140297FB0`) and counts the press in KH1's tally
    (`0x14028C660 (0x13)`).
  - **Ours**: Sora's table copied with ours after KH1's 66 (rows 66..69), his pointer set to it
    each frame KH1's own is there (`kh1rows::install`, from `kh2slots::frame`). Each row: KH1's
    own row of its kind as the template for its trail, effect, sound and voice bytes (plain
    swing 13, finisher 8, air swing 18, air finisher 16; their requests 0xC8 / 0xCB / 0xCC / 0xCE
    checked first); its own request 0x200 + n, whose entry in every set of Sora's (`kh2slots`,
    the request tables made long enough) is our slot of the KH2 motion, built on the template's
    KH1 motion (98 / 101 / 102 / 104) with the template request's kind bits and travel flag,
    whatever the styles; no lunge; its **own attack data**: a copy of the template's (0x70) in our
    memory within an offset's reach of the block (`VirtualAlloc` at the first free address from
    16 MB past it), its reaction kinds (`+8` / `+0xC`) KH1's launch, 5, for a KH2 blow that
    throws (`kh1rows::throws`, as `strike` decides KH1's finisher row and KH2's launch), a flinch,
    3, for KH2's flinch group, else the template's. The rows: Upper Slash (PTYA row 12, motion
    161, ability 18, a swing), Finishing Leap (37, 167, ability 95, as an air finisher: below),
    Horizontal Slash (38, 193, ability 99, an air swing), Guard Break (27, 165, ability 93, a
    finisher). Each move's ATKP is its clip's first hit window's (the Keyblade's first), read
    at run time (the weapon's mset isn't staged here: Upper Slash's, Horizontal Slash's and Guard
    Break's are not known offline).
  - **Their hits**: the attack object KH1's row makes (on the Keyblade, its hit window the
    slot's Keyblade windows) lands with its own data, and its record takes KH2's numbers
    (`combat::scale_record`, from `probe::hit`, through `kh1rows::kh2_atkp`: the data is one of
    ours): the ATKP of the move's window open at KH1's time in the slot (else the row's), its
    power on KH1's damage, its Magic scaling, and for a blow that throws KH2's launch height and
    speed (`Launch::kh2`, KH1's launch reaction on the record, `launched`), as KH2's hits landed
    here do. Hits on his body (Finishing Leap's 854 on group 18) are KH2's hit pass's
    (`takeover::kh2_hits`, above).
  - **Guard Break's guard break**: KH1's attack-vs-attack pass (`0x1402B4BC0`, Steam, Checked
    (decomp)) meets two attacks only while one of them is under priority 0x62; a guard object
    is 100. Its data's priority (`+0x3C`) is made 0x62 at least, so KH1's guards don't stop its
    swing and it reaches the body. **Assumption**: that this is what KH2's Guard Break does (its
    ATKP, in the weapon's mset, not read; KH2's own guard test not read in the decomp).
  - **The Square moves' press** (`combomods::control`): Square in KH1's combo window with the
    swing connected or Combo Master on (as before), KH2's Square pick there
    (`kh2_would_take_launch` with `square`) one of ours: KH1's Attack command is written as KH1's
    command menu writes it (`0x140293480`, Steam, Checked (decomp): the record `0x142D53A40`
    type 1, sub 0, its hold bit `0x1000000` clear, `+4` the lock-on's target `0x142DB3518` while
    `0x142DB3510` is set, else a handle of 0, `0x14038AD90 (0)`), so KH1's control step
    (`0x1402A6220`, which runs a command it finds through its command step `0x1402A7510` the same
    frame and clears it) runs KH1's own Attack step now: its count (one more in the combo window,
    the swing connected or Combo Master), its target, its picker; `combat::exec` then runs our
    row in place of the picker's (`combomods::attack`, `take_ours`; one frame only). Square is
    hidden from KH1's step as before. Guard Break: KH2's pick at the press KH1's combo took
    (`ability_move`'s borrow) runs our row instead.
  - **In the air** (Horizontal Slash): KH1's air state, our row, KH2's hop for the row on KH1's
    air fields (`air_hop`'s `Run::AirHop`, from the same trial as the pick); KH1's fall queued
    after it in place of the runner's idle (`kh1rows::queue_fall`: the queue's count at
    `+0x178`, entries of 0x10 from `+0x17C`, the second one's request 0x40000000 made
    0x40000006, as KH1's own air rows 15..18 queue it).
  - **Finishing Leap** leaves the ground by its root (Checked (files): up 17 / 49 / 66 by frames 8
    / 16 / 24, 0 at 32, then up to 150 at 64, and 250 forward, with no gravity from frame 1 and
    its fall after it, `next_motion` 4): KH1's air state from its start, as KH2's hops from the
    ground (`combat::exec`'s state 2, no hop of its own), and KH2's root travel there while its
    no-gravity range holds him (`kh2slots::air_root`: the clip's root over the frame's 60 Hz
    steps, turned by his facing as KH2's controller turns it, added to KH1's air move in
    `shared::physics`'s air step, either physics; the OG rewrite's action step moves him by the
    root's height too). Its slot is built on KH1's air finisher (104, request 0xCE's kind, no
    travel flag: the root is the air step's, not KH1's H10). **Assumption**: KH1's time in the
    slot when the air step runs is the frame's (at most a frame off, as for the no-gravity hold).
  - Counterguard: now on a row of its own (below, "KH2's guard on KH1's states").
- **Aerial Recovery and Retaliating Slash on KH1 requests of their own** (the tester, m14
  question 3: "(b)", new KH1 requests for KH2's motions; KH2's rules in
  `docs/aerial-recovery.md`, Checked (exe) there): both out of KH1's own knock-away (`hurt`'s
  kind 4 / 5, `combomods::launched`: 16 frames in, in the air), which KH2's controller borrowed
  him for until now. Two more of ours (`kh1rows`), requests 0x204 / 0x205, slots on KH1's air
  swing (102, request 0xCC's kind):
  - **Aerial Recovery** (motion 40, request only, `combomods::recover`): the jump with the
    ability on, KH1's air state (2): KH1's knock-away ended (its action flag `+0` bit 4 off, as
    the borrow did), our request asked for at once with no blend and KH1's action flag (KH1's
    `0x14029F980 (actor, request | 0x1000000, 0)`), KH1's fall queued after it (`0x14029FA30
    (actor, 0x40000006, 0)`: KH2's motion 4), KH2's hop on KH1's air fields
    (`shared::physics::kh2_hop`: 100 high, 10 a frame along the stick and facing it, else
    straight up; `Character`'s `AIR_RECOVERY_HOP` / `_SPEED`, plyr's `blow_recovery_h` / `_v`),
    no Aerial Dodge after it (`shared::mark_air_dodged`). Its windows from our slot:
    invulnerable 0..16 (`kh2slots::invulnerable`), guard 124 on his body 0..16 (`body_guard`),
    and KH2's "air free" state from frame 24 (range id 1) as KH1's "may be left" level (type 0,
    `+0x1CC` bit 20, from which `0x14029D399` clears the action flags `0x1004`), on our own
    requests only, in place of the template's (`kh2slots::kh1_tracks`).
  - **Retaliating Slash** (row 40, motion 172, ability 101): Square with it equipped, KH1's air
    state, KH2 taking the row there (`Character::would_take_row`, the exe's forced row, counts
    at 1 / 1; `takeover::kh2_would_take_row`): KH1's knock-away ended, KH1's Attack command
    written (as for the Square moves), and KH1's Attack step runs our row (an air swing's
    template, its own data, KH2's numbers on its hits) with KH2's hop for the row (21..121 toward
    the target, 0..18 sideways, or 21 and 18 along his facing; `air_hop`'s `Run::AirHop`) and
    KH1's fall after it.
  Either goes back to the borrow when ours isn't built or he isn't in KH1's air state.
  **Assumption**: KH1's knock-away keeps no other state that brings it back once its action
  flag is off and another request plays (none seen with the borrow's hand-back). Not checked in
  game.
- **Finishing Plus's next finisher on a KH1 row of its own** (the tester, m14 question 4: "(b)", a
  new row; `kh1rows::Ours::next_finisher`, `combomods::next_finisher`): two more rows of ours,
  ground (like KH1's finisher row 8) and air (like 16), requests 0x206 / 0x207, each with a slot
  of every KH2 finisher KH1's finisher request of its kind plays (ground: 0xCB's, 154, 155 and
  Explosion 166; air: 0xCE's, 183, 184, Aerial Finish 194 and Magnet Burst 197), the first the
  request's own, the one KH2 picks pointed at before the row runs (`kh2slots::pick_own`; a
  request of ours with several motions: each a redirect, ours whatever the styles).
  - **The window** (KH2: range id 50, `WindowKind::Finisher`, the only place a next finisher is
    taken; Sora's 154 / 155 35..64, 183 21..50, 184 23..52, Explosion 60..72, Guard Break
    48..60, Aerial Finish 89..108, Magnet Burst 70..86, Checked (files)): with Finishing Plus
    equipped and our finisher slot inside it at KH1's time, KH1's combo window level (bit 22)
    is set on the instance's level flags (`+0x14`, which KH1's skeleton pass has just worked
    out and its motion step copies to `+0x1CC`: `kh2slots::finisher_window`, from `frame`,
    which `pose`'s hooks call after the pass). So KH1's command menu takes an Attack there
    (`0x1402A2EC0`: free, or bit 22 of `+0x1CC`; Steam, Checked (decomp)) and KH1's Attack step
    counts it on (one more, the swing connected or Combo Master). Before, the press came only
    once KH1's template let him act (its own "may be left" point). KH2's Square moves take the
    same window (`0x1403A9950`, either bit), and so do ours (`control`).
  - **The press** (the Attack in KH1's finisher, `attack`): KH2's pick there (KH2's trial with
    the finisher chain, now with the finishers so far, `FINISHERS`, KH2's `+0xD75`, so Finishing
    Plus's count of extra finishers is KH2's: before it was always 1) runs on its own row if it
    has one (Guard Break), else on our next-finisher row of its kind (its request isn't the one
    playing, so KH1 starts KH2's motion fresh), else (a third finisher, ours playing) on KH1's own
    finisher row with our slot of the pick; in the air with KH2's hop for the row. The borrow is
    left only for a pick none of these can play. Its hits: its own data and KH2's numbers
    (`kh1rows`), KH2's body hits (`takeover::kh2_hits`) for Explosion and Magnet Burst.
  **Assumption**: KH1's motion step copies the level flags after the skeleton pass on every path
  (the plain and the blend pass; §1.3 step 6). Not checked in game.
- **The drop ahead, row 1** (the tester: "add that"; Steam KH2, Checked (decomp)): on the
  ground, before the walk, the selector (`0x140402D20`) takes the selector's direction (to the
  primary target, else his facing, `0x1404028D0`), scales it by `+0x150`'s `+0x2C` plus `+0x6F0`,
  lays it on the ground (`0x1403B7BD0`: across the ground normal `+0x768` twice, its length
  kept) and sweeps his collision along it from his position (`0x1403BA750` -> `0x14016FEC0`);
  the result's bit 0 marks row 1 (unchecked, in its table place: index 3 of Sora's 37, before
  every ground row but the hops at a target above, so it wins over the swings and finishers;
  Checked (files)). So t374's "blocked" guess was wrong: it is a drop ahead, and row 1 is KH2's
  hop off it (181, a hop of 21). Not done when he is in the air, with `+0x18C` bit 6, with an
  object at `+0x9D8`, or with his object entry's `+0xC` bit 1 (what those are: not known; none
  seen on Sora). `+0x6F0` is his radius from the model's first collision shape of kind 0 or 4
  (`0x1403B6310` -> `0x1403C9BC0 (…, 0x11)`: `+0x10` the radius to `+0x6F0`, `+0x12` the
  height to `+0x6F4`, `+0xE` the lift to `+0x73C`; `kh2_data::sora::bg_shape`; Sora's, entry 0
  of `P_EX100.mdlx`'s list, Checked (files): kind 4, lift 115, radius 90, height 90). `+0x150`
  is the plyr record (Checked (decomp), after t393's assumption: its other reads are plyr's by
  field order, `+0x50` / `+0x54` / `+0x58` blow recovery h / v / time in `0x1403D6BB0` /
  `0x1403D6D50`, `+0x5C` auto lock-on range in `0x1403BE2B0`, `+0x60` / `+0x64` its min / max h
  in `0x1403BE4B0`, `+0x68` its time in `0x1403DCCD0`), so `+0x2C` is `attack_j_front` (Sora's
  -5; `AttackParams::jump_front`): the line is 85 long.
  - **What counts as no floor** (Steam KH2, Checked (decomp)): `0x1403BA750` copies his feet
    (`+0x670`) and the line into the mover block `+0x6D0`, clears its flag `+0x740` bit 0 and
    moves it (`0x14016FEC0`: the move, then the floor search at the end point, `0x140172840`).
    The search is a vertical segment through the end point from the lift (`+0x73C`, 115) plus the
    line's drop (at most 100) above it to three lifts (345) below it; none found: bit 0. Found,
    with `+0x740` bit 0 clear (`0x140172640`'s first branch): bit 0 when the floor is more than
    lift less height less 1 plus `+0x754` below his feet (`+0x6D4`), his feet lowered by the
    line's drop when it runs downhill (`+0x6E4` above 0); `+0x754` is 0 without an object at
    `+0x9D8` (`0x1403B9CE0`), which the selector requires. For Sora: more than 24 below (the
    same 24 as KH2's step down, `takeover::STEP_DOWN`). Only the end point is judged, not the way
    there; no floor attribute is read on this branch.
  - **KH1's own** (row 2, `0x1402C6E70`, Steam, Checked (decomp)): KH1's collision (its model's
    shape, `+0x134`) moved flat for its radius plus 20 in steps of 0.7 of the
    smaller of its radius and height (`0x14029A0B0`), stopping at the first step whose floor is
    more than half KH1's height below, or of KH1's not-a-floor kind (attribute `0xB0`), or
    missing (`0x140195180`: bit 0 of `+0x2D0`).
  - Ours (`combomods::drop_ahead`, Steam only, the three calls' bytes checked): the trial asks it
    (`takeover::kh2_would_take_launch` with `test_drop_ahead`, the ground combo's press only)
    along KH2's selector's direction there (`Character::selector_aim`: to the primary target,
    else his facing, as `0x1404028D0`; t393 used KH1's direction for the press, the stick's
    with no target). The line, 85 long, is laid on the plane of the floor polygon KH1's mover
    finds under him (one still move; its normal by KH1's `0x1401907F0`; a map polygon only, on an
    object flat: **assumption**), KH1's set-up (`0x14029A0B0`) and mover (`0x140195180`) take
    his own collision along its flat part in KH1's steps, all the way, and the floor the last
    step finds (`+0x2DC` 0xFFFF none, `+0x308` its height: the nearest below, or up to his
    height above) is judged by KH2's rule: none, or more than 24 below his feet (lowered by the
    line's drop downhill). KH1's block (`+0x2A0..+0x318`) put back after. Differences left:
    KH1's collision shape and steps move him (walls stop it a little sooner or later than KH2's
    90-radius sphere), and with floors stacked KH1's mover takes the nearest below where KH2's
    search takes the first from 115 above. The hop plays as KH2's hops do (above).
  - **Checked (trace)**, the tester's t409 traces f3990, f4996, f5299, f5413 and their log (no
    target, stick idle, physics KH1): every row 1 was on a block 120 high (`floor_up` 120), facing
    its edge (yaw about 3.09 / -2.90 / 3.03; the edge between z 799 and 807 at x -1387, where the
    floor under him went from 0 to 120 on his way up), about 50 to 70 from it along his facing
    (z 867, 854, 875; if the edge runs straight along x): the line's end 85 ahead was about 15 to
    35 past the edge, over the floor 120 below, more than 24: KH2's row 1 there, as logged (`no floor within 85 (90 his radius)`). Presses farther from the edge
    on the same block took KH2's 151 / 153 (rows 5 / 6). KH1's own row 2 was marked at each of
    them too (the log: "KH1's row 2 replaced"), so the two tests agreed here; they part for drops
    between 24 and half KH1's height, at gaps the line crosses, and on slopes.
  - **Checked (trace)**, after t415 / t416: the tester's t430 traces f48311, f48640, f48888, f49221
    and their log (room 0 of world 0xB, the same 120 block, its edge at about z 799; no target,
    stick idle, physics KH1, Base and Aerial Combo KH2). f48640 / f48888: presses about 40 from
    the edge took row 1 ("the floor 120 below at 85 ahead"), KH1's row 2 or 11 replaced by its
    air row 18 (0xCC) playing 181; he rose exactly 21 (120 to 141) and went forward off the edge
    from the first air frame (7.8 a frame, KH2's 8 after one frame of KH1's drag x0.975, then
    the drag each frame) and landed on the floor below 32 frames on: t415's speed is kept. f48311
    / f49221: the second press about 89 from the edge (the line's end 4 short of it, on the
    floor) took row 6 (153), KH2's rule; that swing's own travel (about 26 a frame over its
    frames 21..24, about 105, KH2's root travel baked into the slot) took him past the edge,
    where KH1's mover dropped him into its fall (state 2, request 6) and the swing ended there.
    Every other ground press in the log (12) took rows 5 / 6 / 3 on flat floor. Not covered: steps
    or a ramp (the traces have only the 0 and 120 floors).
  - **KH2's edge stop** (Steam KH2, Checked (decomp) and (exe); the tester, after t431: "it
    should cancel his animation like kh1, ... look at steam kh2 ghidra decomp"): KH2's ground
    mover (`0x1403B9090`, from `0x1403B89A0`) stops the horizontal step at an edge when
    `0x1403BAB20` says so: it runs the same no-floor sweep as the line ahead (`0x1403BA750`) along
    the step lengthened by `DAT_14074B060` (0.2, Checked (exe)) times his radius (`+0x6F0`, 90:
    18 past the step) and, with no floor, keeps only the step's vertical part (and zeroes
    `+0xD24..+0xD2F` with `+0x6C8` bit 1). `0x1403BAB20` is true with the motion state word
    `+0x18C` bit 23, or standing on an object that allows it (`+0x9D8`'s `+0x18C` bit 28 clear and
    `+0x6C8` bit 15 clear), or `+0x6C8` bit 1 with `+0xD30` above 0, or the movement state
    (`0x1403B5DC0`) 0xF; not with `+0x6CC` bit 2. **Bit 23 is range id 29** (KH2's range handler
    `0x14040D810`, Steam, Checked (decomp): one case per range id, 29 sets `+0x18C` bit 23 through
    the motion block at `+0x158`; t431's assumption that no swing has it was wrong). Sora's
    (`P_EX100.mset`, Checked (files)): the finishers 154 (frames 0..38) and 155 (0..39), Guard Break
    165 (0..40), Flash Step 169 (0..17), 161 (0..28); none of 151 / 152 / 153, Slapshot, Dodge
    Slash, Slide Dash or the air motions.
  - **What KH2 does once he is off** (Steam KH2, Checked (decomp)): after the move, with no floor
    under him (`+0x744` bit 0), bit 23 clear and no no-gravity range (`+0x18C` bits 5 / 24 / 6:
    range ids 3, 30, 24) nor the object entry's `+0xC` bits 0 / 1, `0x1403B9090` sets his velocity
    (`+0x20`) to the step over the frame delta (`0x140717480`) capped at 8, clears `+0x120` bit 26,
    and calls the player's "left the ground" method (vtable `0x1405C43C0` `+0x50`: `0x1403A71B0`
    -> `0x1403D6430`): **the fall motion 4 with a 16-frame blend** (`0x1403C6DC0`, not with
    `+0x194` bit 4, range id 49) and the class's state change (`0x1403B7850`). So KH2 cuts the
    swing into its fall, as KH1 does. KH1's own (Steam `0x14029A3C0`, its physics step, Checked
    (decomp)): on the transition to state 2 it clears the action flag 4 (the swing ends), starts a
    fall from rest (`0x1402AF8F0`) and sets `+0x3C0` to the frame's step per 60 Hz frame capped at 8:
    KH2's own rule. Checked (trace): f48311 / f49221 (t430), `+0x3C0` 8.0 on the frame he left
    (the swing's 26 a frame capped), then KH1's drag; the fall request 6 two frames later (KH2:
    at once, the one difference left).
  - **Ours** (`shared::physics`'s ground step, `kh2slots::edge_stop`, `combomods::floor_gone`,
    after t436): while our slot's KH2 motion has range 29 open at KH1's time (either physics: it
    is the motion's), this frame's step (KH1's state step's result plus the motion's travel KH1
    adds after it, `+0x200` with `+0x160` bit 4) is swept on KH1's collision for 0.2 x 90 past
    its end by KH2's no-floor rule (the line ahead's sweep, made common); with no floor both lose
    their horizontal part, so he stays on the ledge and the finisher plays on. The plain swings
    are unchanged: off he goes, into KH1's fall, as in KH2. Not checked in game; the
    model's pull-back by the travel (`+0x210`, `0x14029F0A0`) is KH1's and not touched
    (**assumption**: it follows the motion, not the travel KH1 applied).
  - A detail seen in f48640 / f48888: the hop's rise starts one frame before KH2's motion does
    (KH1 starts the new request the frame after the row runs, as for every swing; the state and
    speed are set on the press), so for one frame he rises in the end of the ground swing.
- **The finisher tests' status bits** (the tester: "add that"; Steam KH2, Checked (decomp)):
  - `+0x194` bit 5: the count step (`0x1403D6CA0`) takes it with the press's hit as the
    finisher chain (count and finisher count up, while under the most finishers `+0x5C0` block
    `+0x202`; `+0x18C` bit 10 is the same for a swing's window, the count only; with neither,
    `+0x18C & 0x1C` restarts the count at 1 / 1), and the Square moves' test (`0x1403A9950`)
    takes either bit as "in a combo window". **Assumption** (no setter found in the decomp's
    text): bit 5 is "a finisher's combo window open". With it a finisher row passes at any count
    (`0x1403D7780`), and a normal row is refused once more than one finisher has run, even under
    the combo's length (`0x1403D7850`; without the bit a normal row under the length always
    passes). Ours: `Situation::finisher_window`, set when the press chains on from a finisher
    (`try_attack`'s Finishing Plus step). The other change: a normal row under the length now
    passes whatever the finisher count (exe), where the OG refused it past one finisher.
  - `+0x5C0` block `+0x1DC` bit 11 with `0x1403C1D10`: the block's `+0x1D0..` words are the
    equipped abilities by ability number (`0x140402750` tests a row's ability there), so bit 11
    of the fourth word is ability 107, item 0x18B in `03system.bin` (Checked (files)): Berserk
    Charge. `0x1403C1D10` is `+0x1C0 > 0` on the status block whose MP Charge start
    (`0x1403C20A0`) sets `+0x1C0` / `+0x1BC` and MP 0: MP charging. With both, no finisher row
    passes and every normal row does (the combo goes on). The same test (`0x1403D7820`) switches
    the Attack command to command 0x21D (`0x1403F2600`). Ours: `Situation::berserk`
    (`Character::set_mp_charging`, `BERSERK_CHARGE` among the equipped). KH1 has no MP Charge
    and no Berserk Charge in our ability list, so in KH1 it never turns on (a question to the
    tester).
- **Checked (trace)**, the tester's traces f37236, f38130, f41229, f42760, f44092, f44266 (taken
  after the t409 build; physics KH1, Base and Aerial Combo KH2, an enemy ahead or below in room 0,
  enemies above in room 5); the traces carry no target positions and no log came with them, so
  the targets are inferred from the hop numbers:
  - Picks: every air combo is three swings, the third the finisher 183 (row 8), never 184 with
    one enemy. Enemy ahead or below: 181 / 182 / 183 (row 9, its `sub` 10, row 8). Enemy above:
    182 first (row 10), then 181 (182 playing), then 183. KH1's own row each time: 18 (request
    0xCC) with the enemy below, 17 (0xCD) with it above; both kept (plain swings), our slot of
    KH2's motion on them.
  - Hops: each swing's arc on KH1's fields is KH2's height for its row, `+0x3BC` = -height,
    `+0x3B8` = `sqrt(2 height / KH1's g)` (physics KH1): no target above 21 / 17 / 21 (the rows'
    `jump_min`); above 67 (row 10's `jump_max`), 44.4, 71, 52.4, 58.9, the rise to the target
    within the rows' bounds, his climb toward it matching one target about 260 up (f44266).
    Sideways: 8 (the bound) on rows 9 / 10 at range, 3.0 / 5.2 when close, the finisher 12 (the
    bound) or 12.9 .. 14.9, along the direction he turns to (the velocity's direction is his
    yaw). Each set on the press frame, then KH1's drag (x0.975 a frame, stick idle) and KH1's
    gravity (physics KH1). His height is KH1's curve exactly in room 5; in room 0 he is 2.6 / 4.5
    above it while his sideways move is 1 to 3 of the 7 to 13 asked: KH1's collision sliding him
    over the enemy he presses into (not ours; the jump's first frame there shows it too).
  - KH1's rows giving way on the ground: not exercised (no ground combo, no drop ahead, no KH1
    ability finisher); a ground press at an enemy above (f41229) was KH2's Aerial Sweep (row 33,
    equipped, before row 0 in the table), a borrow.
  - Not seen: the ground rows, rows 0 / 1 from the ground, the finisher status bits.

### 7.x KH2's guard and Counterguard on KH1's states (testing branch)

The tester (m15 question 3: "(a)", Guard native, then Counterguard on its own KH1 row). With the
Dodge Roll style at KH2, Square on the ground with the stick at rest and Guard equipped no longer
borrows KH2's controller (`shared::control` -> `combomods::guard_press`); a moving stick still
borrows it for the roll / Quick Run as before.

- **What KH2 does** (OG rewrite `Character::try_action_button` / `clash` / its Hurt step; the
  motion files, **Checked (files)**, read from the Steam install's `obj/P_EX100.mset` and
  `obj/W_EX010.mset`): the press is PTYA row 11 (kind 0, motion 173, ability 1): its Keyblade
  guard window ATKP 161 (kind 2, `refact_self` 1) at frames 6..37 (60 Hz), its trail 8..37, its
  free state (range id 2) from 52, KH2's controller facing its lock first and stopping him. A hit
  stopped there (`refact_self` 1, on the ground) starts the repel: motion 42, facing the attacker,
  pushed back 200 (`hit_back`: 20 a frame, x 0.9 a frame), its own Keyblade guard 162 (kind 2,
  `refact_self` 2: stops, nothing more) at 0..40, free from 40. Attack on any frame of the repel on
  the ground is Counterguard (row 39, kind 0, motion 171, ability 96): body hit 1581 (kind 1,
  power 100) at 20..28, invulnerable and body guard 124 (`refact_self` 2) at 0..20, its combo
  window from 34.
- **Range id 2** (Steam KH2 `0x14040D810`, **Checked (decomp)**): it sets bit 0x10 of the motion
  state word, beside ids 0 / 1's bits 4 / 8; the free-state tests read the three together
  (`0x140422520`: bit 4, or bit 0x10 with the ground / air test). So it is a free state too.
  `kh2-data` reads it as `WindowKind::Free`; KH2's controller takes a jump or the action button
  only in it (or ids 0 / 1), `docs/movement-model.md` "Leaving a move"; `kh2slots::kh1_tracks`
  takes it as KH1's "may be left". No other motion on our requests or casts has it (41, 42, 173
  and some field motions do).
- **Leaving KH2's moves on KH1's states** (the tester, kh2-magic-and-reflect-native m8: "you can
  jump and dodge roll out of moves during its end frames ... so its exact"). KH2 takes a jump only
  in a free state (range id 0 / 1 / 2) or the landing, and the action button's guard, roll and
  Quick Run the same (Steam KH2 `0x1403D6FC0`, `0x1403BFB30`, **Checked (decomp)**); none of
  Sora's swings, ability moves or casts has one (**Checked (files)**). `kh2slots::Leave`:
  - KH2's swings on KH1's combo requests (0xC8..0xCE, and the row picks on them), our own rows
    and the casts: KH1's "may be left" (type 0) only from KH2's free state, none if it has none.
    Before, the swings on KH1's requests kept KH1's template's own point (KH1's swing's), from
    which KH1's control step jumped, rolled or walked him out of KH2's swing. Now KH1's action
    flag clears at the motion's end: the row runner `0x1402A68F0` (and `combomods`' own starts)
    queue the idle / fall after with bit `0x40000000`, which `0x14029F6E0` takes to clear flag 4
    as the motion ends (**Checked (decomp)**, KH1 Steam). KH1's combo window (type 5) is KH2's as
    before, so Attack and commands still come in it.
  - The reactions: KH2's free state, else the template's (some of KH1's reaction idles are
    queued without that bit, `0x14029FA30 (actor, +0x3EA + 6, 0)`).
  - The dodge roll: from KH2's free state (range id 2, frame 36 of 209), not its fall-check event
    (34).
  - KH2's casts on KH1's cast state: a jump or Square cuts them only in a free state
    (`cast_cut_open`), so never for Sora's; an Attack only in the cast's combo window (range id
    11, only Blizzard's 59 from 36; `docs/kh1-magic-native.md` §4c, m9 answer 2 (a)).
  - KH1's own spells handed to KH1 (a spell out of one of KH2's moves, a Drive Form's KH1 spell,
    `takeover`'s hand-off): a jump or Square no longer ends the cast once the spell is out (s168's
    "other cut" is gone, the tester m9 answer 1 (a)); they wait for KH1 to let him act, as both
    games do. An Attack or another spell still cut it once it is out (`attack_cut`, `magic_cut`).
  **Assumption:** nothing of ours needed KH1's "can act" at the old template point during a KH2
  swing (the give-back waits for the combo's end anyway).
- **Ours**: three more KH1 requests (`kh1rows::OURS`, 0x208..0x20A): Guard (173) and Guard repel
  (42), asked for by request alone with KH1's action flag (`combomods::start_own`, KH1's idle
  queued after), and Counterguard (171) as an attack row. The guard windows: `hurt` takes a hit
  from in front (or anywhere with Guard all round) inside one (`kh2slots::keyblade_guard`) as no
  effect with KH1's clash spark; and `takeover::kh2_hits` puts the same windows up as KH1 guard
  objects (`combat::guard_step`, as for KH2's controller), so an attack meeting one is stopped
  before it reaches him; either way `refact_self` 1 starts the repel (`combomods::guard_repel`,
  once a frame), whose push back goes through KH1's per-frame push `+0x120`
  (`combomods::guard_frame`, before KH1's physics step). In the repel, Counterguard equipped,
  KH1's combo window is opened each frame with KH1's count at 0 (`combomods::counter_window`), so
  KH1's own Attack step takes the press; `combomods::counterguard` runs our row if KH2 would take
  row 39 (`Character::would_take_row`).
- **Differences / assumptions**: KH2's controller treats its whole guard action as guarding
  (`out.guarding`); here only the window (6..37) guards, as the files have it. KH1's ground step is
  taken to move him by its speed `+0x2C` while the action flag is set (zeroed at the start,
  **Assumption**). Square again inside the repel isn't taken before its free frame 40: KH1's
  can-act test is false there, and KH2 waits for the same frame (§7.x.2). A guard stopping a hit in the air does nothing (KH2's
  repel is ground-only; its air recoil isn't ported). Not checked in game.

### 7.x.1 KH2's guard reaction in the air (research for m16 question 1 (b); Checked (decomp) and (files))

KH2's party clash reaction (Steam KH2 `0x1403D61D0 (player, own attack, other attack)`, Checked
(decomp)) reads its own attack's `refact_self` (ATKP `+0x13`) and the other attack's
`refact_other` (`+0x14`):
- either is 2: nothing (no motion, no push), the move goes on;
- on the ground (`0x1403BA9A0` false: no air state class, `+0x18C & 0x1000020` clear, `+0x120`
  bit 23 clear), with one of them nonzero: the guard repel, motion 0x2A (42), faced to the clash,
  pushed back 200 (`0x1403D4960`, 0x43480000), a timer `+0xD70` = 80.0 (decremented per frame in
  `0x1403D4CC0`, read with other "not now" conditions by `0x1403D65A0` / `0x1403D5310`; what it
  stops is **not read**);
- otherwise, the recoil `0x1403D4650` with motion -1 and distance -1 (50): on the ground motion
  0x29 (41), pushed back 50; **in the air motion 0x2B (43)**, motion mode 4, and a hop
  (`0x1403D4800`: 50.0 up, 50 x `sstm.hit_back_to_jump` (0.048) = 2.4 a frame sideways), back from
  his own facing (`0x1403B56E0` times -1), no turn.

So KH2 does have an air recoil, but a guard only reaches it with `refact_self` 0 or 1. Every guard
window Sora can have in the air is `refact_self` 2 (Checked (files), the Steam install's
`P_EX100.mset` / `W_EX010.mset` / `00battle.bin`, every motion 0..250):

| motion | window | ATKP | refact self |
|---|---|---|---|
| 202 Aerial Dodge | body, all of it | 406 | 2 |
| 40 Aerial Recovery | body, 0..16 | 124 | 2 |
| 197 Magnet Burst | body, 20..70 | 406 | 2 |
| 8 | body, 5..20 | 406 | 2 |

The `refact_self` 1 guards (173's 161 at 6..37, Flash Step 169's 161 at 22..30) are ground moves.
So in KH2 a block in the air plays no recoil: the hit is stopped and the move goes on, which is
what KH1's states do here already (`guard_repel` returns for `refact` 2). Nothing ported. The air
recoil (43 with its hop) is KH2's answer to Sora's own air swing meeting a guard or an attack with
`refact` 0 / 1 (the wall-hit path), not to his guard blocking.

### 7.x.2 Square during the repel (research for m16 question 2; Checked (decomp) and (files))

KH2's action button handler (Steam KH2 `0x1403A81F0`) gates every branch that can come out of the
repel on `0x1403BFB30 (player)`: the object's vtable `+0x78` test and `+0x18C & 0x1C`, the free
state bits that range ids 0 / 1 / 2 set (KH2's range handler `0x14040D810`: 4 / 8 / 0x10 of the
motion state word, `+0x158 + 0x34`). That is the guard (`0x1403A9C70`), the dodge roll (growth kind
4, motion 0xD1) and Quick Run (growth kind 1, motion 0xCB); the Square combo moves (`0x1403A9950`)
want the combo window instead (`+0x18C` bit 10, range id 11, or `+0x194` bit 5), which the repel
doesn't have. Motion 42's ranges (Checked (files), `P_EX100.mset`): range id 2 from 40, range id
14 from 8, and effect / sound ranges (id 23). Range id 14 sets `+0x18C` bit 0x80000, which only
`0x1403BA070` reads, for combo rows with PTYA flag 0x100 (reaction commands, OpenKH's "allow RC"):
it doesn't open the guard, roll or Quick Run. So in KH2, Square takes nothing in the repel before
frame 40, the frame KH1's "may be left" opens on our request (`kh2slots::kh1_tracks`). Already the
same; nothing to switch. (The OG rewrite's controller lets any button cut a Hurt action from its
range id 14, "our reading", `character_core` `ActionKind::Hurt`; for the guard repel, borrowed
cases only, that is earlier than KH2's exe. Not changed here.)

### 7.x.3 The Guard Repel row: KH1's own block (m17 "(b)"; Checked (decomp) and (exe))

(Since the recoil thread's merge the row is gone: the Recoil row in Combat decides it, KH1's block
at Recoil KH1, KH2's repel at KH2; `docs/kh1-kh2-recoil.md` §6.6. The text below is the row's
history.)

Config > Movement > Guard Repel (KH1 | KH2, ini `guard_repel`, default KH2) picks what base Sora's
KH2 guard (Dodge Roll KH2, motion 173's window) does when it stops a hit with `refact_self` 1. KH2:
the repel above (motion 42, pushed back 200, Counterguard). KH1: what KH1's own guard gets.

- KH1's guard row 0x3D (attack data 0x4DA8 in `btltbl.bin`, Checked (files)): bone 0x30, clash
  type 2, its priority byte `+0x3C` 0, so `0x1402B3270` gives the object the Keyblade's priority
  (weapon record `+0x33`, the weapon at combat state `+0x20`), clamped 1..99, and points its result
  word (`+0x60`) at combat state `+0x50`. So KH1's guard is **not** a priority-100 guard (an older
  note in `kh1-roll-guard-native.md` §3.3 said so): it is under 0x62.
- KH1's clash side `0x1402B4BC0 (p, q)` with p the guard: p under 0x62 and type 2 (not a
  directional winner), so p's owner gets a clash record (`0x1402BF060`): gap = p's priority less
  q's, kind 7 when the gap is under -40, else 6; p's result word gets 0x10 (0x20 for kind 7).
  Checked (decomp).
- Sora's hit pass `0x14029BBD0` runs kind 6 with `0x14029D720` (request 0x6E, slot 72, the idle
  queued, KH1's no-damage timer 8 frames, KH1's combo count one less, pushed back -gap x btltbl
  `+0x1C` x stats `+0xC4` when the gap is below 0) and kind 7 with `0x14029D5F0` (request 0x6F, no
  push). Checked (decomp); first bytes Checked (exe).
- KH1's Counterattack (`0x140294220`) offers command 99 (row 0x3E) with KH1's Counterattack bit,
  request 0x6E playing and the result word set: so a KH1 block is where KH1's Counterattack comes
  from. This answers §3.4's "not found" in `kh1-roll-guard-native.md`.

At KH1, `combomods::kh1_block` runs that recoil at once (the record's `+8` the gap), marks the
result word and drops KH2's push. The attacker's priority: the pool attack whose hit bit our guard
object's mask gained, or for a hit record its attack data's `+0x3C` (**Assumption**: enemy attack
objects keep their data's priority). Unknown: gap 0. The attacker's own recoil is KH1's clash on
our guard objects (priority 100) either way, so at KH1 the attacker recoils as against KH1's
priority-100 guards, not as against KH1's own guard's Keyblade priority (**difference**, kept so
the row only changes Sora's side). KH2's Counterguard needs the repel (42), so at KH1 only KH1's
Counterattack follows a block.

### 7.y Guard Break against guards (Checked (files))

The tester (m15 question 4: "(a) but read the mset"). `obj/W_EX010.mset` motion 165 (Steam
install, read with the repo's `kh2-combat mset`): its hit is ATKP 876 (group 5, frames 28..42)
and it carries a guard of its own, 162 (group 2, frames 5..42). 876 is kind 1 (Sora's plain
swings 126..130 are kind 0; the guards 161 / 162 kind 2), power 250, reaction 11; kind 1 is
OpenKH's "pierce armor", the guard break (the name is OpenKH's; KH2's own use of kind 1 in its
clash test isn't read in the decomp, **Assumption** that it is what goes through guards). KH1's
rule stays (`kh1rows::GUARD_THROUGH`, priority 0x62). Counterguard's 1581 is kind 1 too. Guard
Break's own guard 162 now goes up as a KH1 guard object while it plays (above, `kh2_hits`).

### 7.z KH2's ability rows inside KH1's own row picker (Base / Aerial Base Combo KH1)

The tester (m19): "if an ability is equipped thats kh2s and your combat config is set to kh1 then you
should be able to do kh1s moves, and its row picks, we need to modify kh1s so that it can choose
explosion and other moves during its kh1 combo", "patch the extra functions that kh2 uses in kh1".

KH1's pickers (Steam, Checked (decomp)):
- The Attack step `0x1402A7510` counts the press first (the combat info's `+0x41`: 1 outside a combo
  window, one more inside it when the swing connected or Combo Master is on), then asks the ground
  picker `0x1402A6EE0 (actor, target, dir)` in state 0, the air picker `0x1402A6C70` in state 2 and in
  flight (8), and runs the row with `0x1402A68F0`.
- The ground picker marks rows in a flag array (one int a row, 66 rows): the finisher (8) at the
  stats' `+0xD4`, KH1's ability finishers 3 / 4 / 5 by its ability bits `0x142D5EC10` bits 27..29
  (30% each, a random number from `0x14031BA30`), 7 by bit 26, 6 by bit 3 with the finisher; the dash
  at a target over 160 away (0 with ability bit 1, else 1); 9 by bit 24 and 10 by bit 0 by the
  target's height; 11 / 13 / 14 by the targets around and ahead (`0x1402A7320`); 2 when its sweep
  `0x1402C6E70` finds a drop ahead. It walks the rows in table order and takes the first marked row
  whose request isn't the one playing; with none, 13 (11 in some cases).
- The air picker: 16 its finisher at `+0xD5`, 15 a finisher with ability bit 25, 17 a target more than
  100 above, 18 its plain swing; walked from 15 the same way, 18 down to 15 with none.
- Prologues (Checked (exe)): ground `4C 8B DC 55 56 57 49 8D AB 48 FF FF FF`, air `40 53 55 56 57 41
  54 41 56 41 57`.

What we add (`kh1picker.rs`, `combomods::picker`): both pickers are wrapped. KH1's picker runs whole;
then, base Sora with the combo style KH1 where he is (not flight, not a Drive Form), if it took one of
its plain rows (ground 1, 8, 11, 13, 14; air 16, 17, 18), KH2's ability rows come next, the way KH1's
own ability rows come before its plain ones. The candidate is the first of KH2's ability rows in
table order (`Character::ability_row`) that is equipped, of the kind KH1's count is at (a finisher at
KH1's length, else a swing), on the ground or in the air as he is, playable on KH1's rows (a KH1 row
of its own, or our slot of its motion on a KH1 request of its kind), not the KH2 motion playing (KH2's
anti-repeat), and passes KH2's own row checks: the ability and Berserk Charge test (KH2 `0x140402750`)
and its distance, height and facing windows against the primary target (kind 1) or any target (kind
2) measured as KH2's selector measures them (KH2 `0x1404029F0`, Checked (decomp) earlier in §7). Kind 0
rows (the Square moves, Counterguard, Retaliating Slash) are only forced, as in KH2. The picker then
returns the KH1 row it runs on (KH1's plain row of its kind whose request isn't playing), and the row
runner plays it as the KH2 setting's ability moves play (KH2's motion and hop, KH1's row, attack object,
damage and windows). Where KH1 took one of its own ability rows, or its drop-ahead row 2, it stays.

KH2's combo abilities at this setting (no longer turning the combo to KH2's, `docs/kh1-move-styles.md`):
Negative Combo lowers the length byte KH1's picker reads for the call (`kh1picker::shorten`);
Finishing Plus's next finisher out of a KH2 ability finisher's window is KH1's picker's finisher
(`combomods::attack`), up to one plus its copies.

KH2's selector (its whole-table walk, its combo length) isn't run at this setting. Assumption: KH2's
ability rows going after KH1's own ability rows and before its plain rows (KH1's own order for its own
ability rows) is the right place for them; KH2 has no rule for mixing the two tables.
At the finisher this is now one walk in KH2's order with KH1's twins in their KH2 move's place
(§7.z.1); swings are as above.

### 7.z.1 One finisher walk: KH1's ability finishers and KH2's in KH2's order (combo row selection m21)

The tester (m19 / m20 / m21): KH1's twins follow their KH2 move ("gravity break and blitz follow
guard break", "ripple drive, stun impact should follow explosions logic", "hurricane blast should
follow aerial finish"), KH2's exact order in a group (m20 1 (a)), "for explosion it should come
last, for hurricane blast it should come first", "gravity break and blitz last, if its blitz and
gravity equipped then blitz last", "you need to keep blitz's rule", and a toggle for KH1's 30%
chance ("if the 30 percent chance roll fails then it picks a similar kh2 move").

**KH1's ability finishers** (Steam, Checked (decomp)): Sora's ability bits `0x142D5EC10` are set
from his equipped abilities by `0x1402A6380` (`0x140284BE0 (id)` for each): bit 3 Blitz (20), 25
Hurricane Blast (55), 26 Ripple Drive (56), 27 Stun Impact (57), 28 Gravity Break (58), 29
Zantetsuken (59); also 0 Vortex (17), 1 Aerial Sweep (18), 2 Counterattack (19), 4 Guard (21), 5
Dodge Roll (22), 23 Slapshot (53), 24 Sliding Dash (54), 7 / 8 Glide / Superglide. The ground
picker `0x1402A6EE0`, at the finisher (the count at the stats' `+0xD4`): one KH1 random number
(`0x14031BA30`: three `rand` bytes x 2^-24, prologue `48 89 5C 24 08 57 48 83 EC 20`, Checked
(exe)), 0.3 taken off it for each of Stun Impact (row 3), Gravity Break (4), Zantetsuken (5)
equipped in that order, the first that goes below 0 marked; none, the plain finisher (8). Ripple
Drive (7) is marked with any finisher. Blitz (6) is marked when the plain finisher is and row 11
isn't (`0x1402A7320` counts row 11 for each lock point beside him, the dot with the direction
between `0x142D5EC28` and `0x142D5EC2C`, nearer than `0x142D5EC14`: nothing close beside him).
The walk is table order, so a dash at a far target (rows 0 / 1, marked before) beats all of
them. The air picker `0x1402A6C70` marks 15 (Hurricane Blast) with the finisher in state 2 for a
target not more than 100 above, before 16.

**KH2's order** (Sora's PTYA, `00battle.bin`, Checked (files); the walk `0x140402C50` takes the
first marked row in table order whose `sub` is its own id or whose motion isn't playing, no
chance anywhere, Checked (decomp)):
- ground finishers: Explosion 26 (any target 0..300, facing -1..0.92: a foe off the line to the
  main one, a group), 154 (row 2, a foe 0..180 off the line), Guard Break 27 (no range), Explosion
  42 (no range), 155 (row 3);
- air finishers: Magnet Burst 49 (any target 0..650 off the line), 184 (row 7, 0..260 off the
  line), Aerial Finish 29 (no range), Magnet Burst 50 (the main target 300..650), 183 (row 8).
So KH2 puts the area move first only in a group (26 / 49), then the single-target move (27 / 29),
then the area move again for one foe (42; 50 only far). The tester's "one does aoe damage
(explosion group), the other focuses on one enemy (guard break, etc.)" matches that table.

**The walk** (`combomods::finisher_walk`, Base / Aerial Base Combo KH1, base Sora, not flight):
`kh1picker` runs KH1's picker with bits 26, 27, 28 (ground) / 25 (air) held back, so it marks only
its plain finisher, its dash, its drop ahead, Zantetsuken and Blitz by its own rules, and returns
its pick. Then, at the finisher, groups in KH2's order, each passed over when its row's range
checks fail (`Character::row_in_range`, KH2's `in_range` against Sora's targets and KH1's lock):

| Ground | Moves in order | Air | Moves in order |
| --- | --- | --- | --- |
| row 26's checks | Stun Impact, Ripple Drive, Explosion | row 49's | Magnet Burst |
| row 2's | KH1's own finisher | row 7's | KH1's own finisher |
| row 27's (none) | Guard Break, Gravity Break, Zantetsuken, Blitz | row 29's (none) | Hurricane Blast, Aerial Finish |
| row 42's (none) | Stun Impact, Ripple Drive, Explosion | row 50's | Magnet Burst |
| | KH1's own finisher | | KH1's own finisher |

- KH1's moves: their bit as KH1 set it, their row's request not the one playing (KH1's own
  anti-repeat). Stun Impact and Gravity Break: with KH1 Ability Chance on (ini
  `kh1_ability_chance`, the Config menu's Tweaks list), KH1's roll: one random number per press,
  0.3 off it for each roll reached, so each keeps exactly KH1's 30% whichever comes first; a
  failed roll isn't rolled again further down (Stun Impact is in two groups). Off: they pass
  whenever their group does. Blitz / Hurricane Blast: only when KH1's picker returned 6 / 15 (its
  own test passed).
  None of KH1's when its picker took its dash (row 1): KH1's walk has the dash first.
- KH2's moves: `Character::ability_row_id` (equipped, playable on KH1's rows, not the KH2 motion
  playing, its own checks), run on KH1's rows as in §7.z.
- KH1's own finisher: its picker's pick (row 8, or 16; row 8 when it returned Blitz; its dash or a
  swing when it took one).
- Zantetsuken (bit 29, row 5; the tester, m22 answer 3: "bring back zantetsuken and make it b as
  well", its KH1 rule): held back with the others, so KH1's row 8 stays marked for Blitz's test.
  KH1's rule is the third 30% roll on the same random number, after Stun Impact and Gravity Break
  (Checked (decomp): `0x1402A6EE0` takes 0.3 off for each equipped one in that order, and only
  when all three fail marks row 8, the plain finisher). The walk rolls it the same way (one
  number per press, 0.3 off per roll reached) with KH1 Ability Chance on, and with it off it passes
  whenever its group does, like Stun Impact and Gravity Break (the tester, after m25: "it should
  change on the toggle we have like the other moves, if you have ability chance off then it should
  be 100 percent like the other moves"; t589 rolled it always). Its place: the single-target group, after Gravity Break, before Blitz (by how close each
  keeps the foe, below). Its data (row 5, `0x4FD8`, Checked (files)): stat source 2 (none), element
  8 (gravity: a share of the target's HP), power 250, reaction 3 (a flinch).
- Without KH2's data: KH1's own order (Stun Impact, Gravity Break, Zantetsuken, Blitz, Ripple Drive,
  finisher).
- Logged: "KH1's ground finisher walk ..." / "KH1's aerial finisher walk ...", with each failed
  roll and Blitz test.

What changes against KH1 alone: Ripple Drive and Stun Impact come before KH1's finisher in a
group, Gravity Break and Blitz after Guard Break; KH1's own finisher comes first in a group (the
154 / 184 place). (Hurricane Blast took Aerial Finish's checks at t580, so it came with no target
or one high above; since m22 answer 3 (b) it keeps KH1's rule, below.)

**Hurricane Blast keeps KH1's rule** (the tester, m22 answer 3 (b), "like Blitz"). KH1's air
picker `0x1402A6C70` (Steam, Checked (decomp)) marks row 15 in state 2 at the finisher only when
there is a target and its lock point (`0x1402B04C0`) less his position has y at -100 or more (KH1's
y is down: the target no more than 100 above him), its request not the one playing; its plain
finisher 16 and plain swing 18 are marked then too. `kh1picker` no longer holds bit 25 back
(`HELD_AIR` = 0), so KH1's picker runs that test itself and returns 15 when it passes; the walk
takes Hurricane Blast in Aerial Finish's place only then (`Kh1Rule::Kh1Test`, as Blitz), KH1's
plain finisher being 16 (18 when 16's request is playing). Nothing else of the walk changes. Assumptions: that KH1's own finisher stands in KH2's 154 / 184 place (the tester's
m20 1 (a)). Not checked: in game. **Which of KH1's single-target finishers comes first** (the tester, m22: "(assumption) if you have
two single hit vanilla moves then the priority should be based off of which one launches and keeps
the enemy closer, the one that keeps the enemy closer should come out first"). KH1 has no
per-attack knock-back distance: an enemy's reaction is picked by its row's attack data `+0xC`
alone (§ `docs/kh1-launch.md`), and each kind moves it a fixed way (Steam, Checked (decomp): the
enemy hit pass `0x1402B8E10`; Checked (files): `btltbl.bin`, rows at `+0xA9F0`, block `+0x58`):

| Move (row, data) | Reaction vs enemies | What it does to the enemy | Moves it |
| --- | --- | --- | --- |
| Gravity Break (4, `0x4F68`; element 8, power 40) | 8 | `0x1402B7DE0`: motion 0x66 on the ground (crushed where it stands), or KH1's air state 0x16 / 0x17 with motion 0x68 and its sideways speed `+0x2C` set to 0 | 0 |
| Zantetsuken (5, `0x4FD8`; element 8, power 250) | 3 | a flinch, `0x1402B9230` → `0x1402B7E60`: KH1's push of block `+0x2C` (80) × the enemy's push factor (stats `+0xC4`) | up to 80 |
| Blitz (6, `0x4D38`; element 1, power 6) | 5 | a launch, `0x1402B8530`: 20..35 a frame sideways (block `+0x14` / `+0x16` by stats `+0xC4`), up 1.35 × that (27..47), slowed by the enemy's own decay | the farthest |

So the order in the single-target place is Gravity Break, Zantetsuken, Blitz (after KH2's Guard
Break, the tester's m21 "gravity break and blitz last ... blitz last"), which the walk already had
for Gravity Break and Blitz; Zantetsuken goes between them. Stun Impact (3, `0x4EF8`: element 10,
reaction 3, bone 8, not the Keyblade) and Ripple Drive (7, `0x4E88`: max MP × 3, reaction 5) stay
in Explosion's places, first with a crowd and after the single-target moves against one foe (KH2's
rows 26 / 42, unchanged since t580). (Zantetsuken's gravity power 250 takes a share of the foe's HP
of 250%, floored at its `+0xC0`: most foes it hits don't live to be pushed.)

**Finishing Plus on KH1's own finishers** (the tester, m22: "make sure every kh1 finisher can go
into other kh1/kh2 finishers"; m20 answers 3 and 4 (a), m21 answer 2 (a)). Before, at the KH1
setting only KH2's ability finishers on our slots had a Finishing Plus window
(`kh2slots::finisher_window`), so KH1's own finisher and its ability finishers (Stun Impact,
Gravity Break, Zantetsuken, Blitz, Ripple Drive, Hurricane Blast) ended the combo.
- The window (`combomods::kh1_finisher_window`, every frame from `kh2slots`' frame step): while
  KH1's own finisher motion plays (the last row KH1's Attack step ran was a finisher, its request
  `+0x164`, no KH2 motion of ours on it), Finishing Plus equipped, another finisher left, KH1's
  combo window bit (`+0x1CC` bit 22, set through the instance's `+0x14`) is set at the same share
  of KH1's motion as KH2's Finishing Plus window is of 155 (ground, 35..64) / 183 (air, 21..50):
  KH2 frame = `+0x16C` / `+0x280` × the clip's length (KH1 computes that same share in
  `0x14029FF80`, Checked (decomp)). **Assumption**: placing KH2's window by share (KH1's finishers
  have none). KH1's command menu takes the Attack there and its Attack step counts it on, as for
  KH2's ability finishers.
- The pick: KH1's picker runs at the count past its length and the walk picks the next finisher
  in the same order, the one playing passed over (KH1's rows by their request, KH2's by the KH2
  motion). When KH1's own normal finisher is the one playing (its picker took a swing), the walk's
  "KH1's own finisher" places take KH2's normal finisher by KH2's own walk
  (`Character::normal_finisher`, Checked (decomp) `0x140402C50`): the first of Sora's
  no-ability finisher rows of that kind that passes (154, row 2, with a foe 0..180 off the line;
  else 155, row 3; in the air 184, row 7, else 183, row 8), the playing one skipped for its `sub`.
  It plays on KH1's rows as Finishing Plus's KH2 finishers do (`next_finisher`).
- The chain stops at 1 + Finishing Plus's copies (`FINISHERS`, KH2's `+0xD75` rule).

**Locked on, the group checks count every enemy** (the tester, m22 answer 4 (a), its own
version). Before, `Character::row_targets` kept only the locked foe for KH2's "any target" rows
(26, 2, 49, 7 and the ability swings' kind-2 rows), so the group places never passed while locked
on (R1, or KH1's small cursor, both a lock to us). KH2's selector `0x140402D20` (Steam, Checked
(decomp)) runs its kind-2 pass `0x1404029F0 (selector, target, 2)` on every target shape the
iterator `0x1403BDAC0` gives on an attackable actor (`+0x6C8` bit 17), whatever the lock; only the
kind-1 pass uses the primary target (the locked one when locked). Now every target goes in; the
primary and the selector's direction are still the locked one's, so the locked foe itself sits at
the line's centre and a group row needs another one off it, as in KH2. Only the Base / Aerial Base
Combo KH1 walk and KH1's picker's KH2 ability rows use this.

**The KH2 setting too** (the tester, m23 answer 1 (a)). KH2's own selector at the KH2 setting
(`Character::try_attack`, also the trial `would_take` the KH2-setting combo asks) counts every
target while locked, as above; stage 207's rule (only the locked foe, so Sora wouldn't lunge at
the one beside him) is gone. What stops the wrong-foe lunge now is KH2's own rule: the kind-2 pass
`0x1404029F0` only sets the row's bit in the selector's mark array (`+0x60`), it keeps no target,
and the hop and lunge (`0x1403D78D0`) are measured to the character's own target `+0xDC8` (the
primary one). `combo::select` now gives a kind-2 row the primary target as its aim (it gave the
target that passed). Checked (decomp, Steam KH2). Unlocked play changes the same way: a group row
that passed on a foe beside him is aimed at the primary foe, not at that one.

### 7.z.2 KH1's ability finishers at the KH2 setting (combo row selection m23)

The tester (m23): "I noticed I can't do any kh1 finisher on kh2 combat settings". Why, before:
with Base / Aerial Base Combo KH2, KH1's picker ran as vanilla (its own 30% rolls included), then
the row runner (`combomods::attack`) asked KH2's selector for this press and, where KH2's pick was a
plain swing or finisher, put KH1's plain row in place of KH1's (`kh2_row`, t390, the tester's m3
answer 1 (a): "KH1's own rows give way"). KH2's table has no KH1 move, so Stun Impact, Gravity
Break, Zantetsuken, Blitz, Ripple Drive and Hurricane Blast never came at this setting.

Now (`kh1picker::at_kh2`, `combomods::kh2_press`, `kh2_finisher_walk`), in KH1's own picker:
- KH2's pick at this press is asked first (the same trial the row runner asks: KH1's count, this
  press counted; Finishing Plus's next finisher when KH1's last row was a finisher still playing).
- KH1's picker then runs with its ability finisher bits held back (`HELD_GROUND`, as at the KH1
  setting) and KH2's combo length on the length byte it reads (stats `+0xD4` / `+0xD5`, its test
  `len <= count`, Checked (decomp) `0x1402A6EE0` / `0x1402A6C70`), so KH1's own finisher, and
  Blitz's and Hurricane Blast's own KH1 tests, fall at KH2's finisher.
- When KH2's pick is a finisher, the walk of §7.z.1 runs (`GROUND_FINISHERS` / `AIR_FINISHERS`,
  each group gated by its KH2 row's checks), KH1's moves by their KH1 rules (`Kh1Rules`: ability
  bit, KH1's anti-repeat, Stun Impact / Gravity Break / Zantetsuken's roll with KH1 Ability
  Chance on, Blitz / Hurricane Blast's own test). KH1's dash exception is off (no KH1 dash here).
  The first KH2 row that passes, or the plain finisher's place, leaves KH2's own pick, run as
  before; one of KH1's that passes is the row the picker returns, and the row runner runs KH1's row
  whole (`take_kh1_take`; KH1's motion, attack data and damage).
- Finishing Plus: KH1's own finisher at this setting gets the same window as at the KH1 setting
  (`kh1_finisher_window`), while KH2's count leaves another finisher (`FINISHERS` against one plus
  the copies); the next press walks again, so KH1 → KH2 and KH2 → KH1 finishers chain.
- Cost: one more KH2 trial per Attack press at this setting (a controller copy), nothing per frame.
- Assumption: KH1's moves keep their KH1 rules at the KH2 setting (the rules the tester chose for
  them at m21 / m22); KH2's own rows have no chance.

### 7.z.3 KH2's whole pick in KH1's picker at the KH2 setting (combo row selection m24, step 1)

The tester (m24): "(a) is there a way for kh2's tables to be included in kh1's tables as extra
features to the engine?" and "go ahead" on the native plan's steps 1 → 2 → 3.

**KH1's side** (Checked (decomp), Steam `0x1402A7510`, KH1's Attack step): for the Attack command
it counts the press, then asks its ground picker `0x1402A6EE0` in state 0 or its air picker
`0x1402A6C70` in states 2 / 8, and runs the row the picker returns with the row runner
`0x1402A68F0` (its only caller; the pickers' only caller too). State 1 runs no row. So for every
row the row runner runs on an Attack press in states 0 / 2, the picker ran on the same frame just
before.

**Before (t612):** at the KH2 setting the pick was asked twice per press (once in the picker for the
finisher walk, once again in the row runner hook), and the row runner hook swapped KH1's row for
KH1's plain row of KH2's kind after the picker returned (`kh2_row`).

**Now** (`kh1picker::at_kh2`, `combomods::kh2_press` / `kh2_pick_in_picker`):
- KH2's pick is made once, in the picker hook, before KH1's picker runs (KH1's count, this press
  counted; Finishing Plus's next finisher first when KH1's last row was a finisher still playing).
- KH1's picker runs with KH2's length and its ability finisher bits held back, then the finisher
  walk (§7.z.2). Where KH2's pick stands, the picker itself returns the KH1 row that plays it:
  for a plain swing or finisher, KH1's plain row of that kind whose request isn't the one playing
  (`kh2_row`). For a Finishing Plus next finisher, a hop from the ground, or a move on one of our
  own KH1 rows (`kh1rows`), it returns KH1's own pick and the row runner hook picks the row from
  the same kept pick, as before. Replacing KH1's dash (rows 0 / 1) clears its distance in the
  direction's y, as `picker` does at the KH1 setting.
- The row runner hook (`attack`) takes the kept pick (`KH2_PICK`: same frame, same row, same
  air); it no longer runs KH2's selector or swaps the row. If no pick was kept (the picker hook
  didn't run, which only happens when the hook isn't installed), it asks as before.
- Expected in play: nothing different. One KH2 trial per press instead of two.

### 7.z.4 KH2's table walk without a controller copy (combo row selection m24, step 2)

Before, every KH2 pick for a press of KH1's (`Character::would_take_launch`, `would_take_row`,
and through them `ability_row_id` / `normal_finisher`) copied the whole KH2 controller, set the
copy up as KH1 had him, pressed Attack on it (`try_attack`) and read back the row and hop.

Now `Character::plan_press` is KH2's press step as a plain function: from a press state (the count
and finisher count, the last swing's hit, the carried combo, in the air, the facing, a forced row,
the motion KH1 plays now) and the targets it works out the count step, the lock turn, the
selector's walk over the row table (`combo::select`, Steam KH2 `0x140402D20`, Checked (decomp) as
in §7.z.1), the forced-row and Square-move rules, the hop (`0x1403D78D0`) or lunge, without
changing anything. `try_attack` (KH2's own press when the controller has him) is that plan,
applied; the host trials build the press state from KH1's data (KH1's count `+0x41`, its hit flag,
the request playing through our slot, KH1's facing, its lock and the enemies around him, as before)
and run the plan, with no copy. The row table and Sora's numbers (`00battle.bin`'s PTYA, his
params and abilities) are still read from the KH2 controller object, where KH2's data is loaded.

Checked: a unit test (`host_trials_walk_the_table_as_the_copy_did`) runs both ways over 34,000
cases (targets near, far, above, a second one; lock on / off; ground / air; counts 0..4; hit /
missed / finisher carried; host motions; stick; Attack, Square and forced rows; free or mid-swing)
and they give the same row and hop. Expected in play: nothing different, less work per press.

**Not done here:** KH1's table itself isn't extended with KH2's rows (see the reply in m24's
thread: what that would mean).

## 8. KH2's walk and run as slots (t389)

The tester (`docs/tester-requests.md`, "Missing sleeves animation", point 6): "can you also bake
the animations for kh2s walking and running cycles? its more easy for me to compare." Before,
KH2's walk and run were only drawn over KH1's own (the follow route, `pose::play_for`, at a rate
matched to his speed).

**KH1's side** (Checked (files): `xa_ex_0010.mset`'s request table; Checked (decomp): §1.3's
resolver `0x14029E050`):
- Walk is request `0x1`, entry `0x3002`; run is `0x2`, entry `0x3005`: kind 3, so KH1 plays the
  entry's slot + 0, 1 or 2 (field, battle, the third variant by `0x142867CD8` bit `0x1000`):
  walk slots 2 / 3 / 4 (60, 60, 48 frames), run 5 / 6 / 7 (36 each).
- The actor's motion rate (`+0x284`) is 1: `0x14029EF50` sets it, and otherwise only event
  script commands do (`0x1401AB830`, `0x1402BA370`, through `0x14029E8B0`). So KH1 plays its own
  walk and run, and ours, at their own speed, whatever his speed.

**Ours** (`kh2slots::CYCLES`): each request gets three slots in a row on KH1's three as templates
(retimed to KH2's length, baked with `kh1_bake`, `Baker::poses`): KH2's field cycle on the field
one (2, 5), its battle cycle on the other two (3, 4 / 6, 7), as the follow route draws them; the
request is pointed at the first (`redirect`), and KH1's kind 3 adds 0, 1 or 2 itself. All three
or none (a slot missing would make KH1's +1 / +2 land on another of ours). Its style is Misc
Animations (`shared::slot_kind`, slots 0..7), the same switch as before. KH2 motion 1 (walk: 72
frames, 30 fps, keys 20..56) and 2 (run: 32 frames, keys 0..16): a KH2 cycle per KH1 cycle.
- KH1 keeps its own: the request id (`+0x164`), its states, its movement, its footstep and sound
  events (the template's, stretched to KH2's length: Assumption that both cycles start on the
  same foot), its loop. KH1's own parts (belt chain, shorts, hem) take KH1's walk or run, stretched
  the same way (`docs/kh1-secondary-motion.md`, "KH1's own parts").
- `Play::Slot` (not baked: `kh1_bake = off`) and `follow_baked` take the field variant for the
  field slots (`kh2slots::is_field`).

Differences from before, for the tester to look at:
- The cycle now plays at KH2's own speed (KH1's rate 1), not stretched to his speed over the
  ground as the follow route did, so at speeds other than KH2's run the planted foot may slide.
- KH1 blends into and out of the cycle itself (its blend clock), as with the swings.

Not checked: in game; that KH1's walk and run states loop our slots over their whole length
(Assumption: no custom loop range, `+0x288..+0x28C`, is set for them); which of KH1's states
picks the third variant (4 / 7).
