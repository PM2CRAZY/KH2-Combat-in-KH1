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
  300..650), **row 10 (182, primary, 0..260, a target below him)**, **row 9 (181, none, sub 10:
  the default)**. The air rows hop too (jumps 14..21).
- By id only (kind 0): **row 1 (181, a hop of 21: the drop ahead, below)**, Guard (11), Upper Slash
  (12), Finishing Leap (37), Horizontal Slash (38), Counterguard (39), Retaliating Slash (40).
- So the plain ground combo is: a target within 180 ahead or to the side: 152 / 151 alternating,
  finisher 154; 180..300 dead ahead: 153 (row 6) / 151; otherwise 151 / 153, finisher 155. The
  slots t374 builds (`kh2slots::swing_rows`: 151, 152, 153; 154, 155) are all the plain ground
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
- Not changed: the air combo keeps its fixed 181 / 182 / 183.
- Found, not added (Steam KH2, `0x140402D20`): on the ground, before the walk, KH2 tests a line
  ahead of Sora (his facing scaled by a stats length `+0x2C` plus `+0x6F0`, laid on the ground,
  `0x1403B7BD0` / `0x1403BA750`) and marks row 1 if it is blocked (not in the air, not with `+0x18C`
  bit 6, no object at `+0x9D8`). The OG rewrite doesn't have it. **Assumption:** a wall or obstacle
  just ahead.
- Found, not added: `0x1403D7780` / `0x1403D7850` also force a finisher when `+0x194` bit 5 is set
  and refuse finishers while `+0x5C0` block `+0x1DC` bit 11 and `0x1403C1D10` hold. What those
  states are: not looked at.
- Unverified in game: all of it.

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
