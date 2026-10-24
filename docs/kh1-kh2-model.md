# KH2's player model in KH1

The Config menu's **Player Model** row (after Misc Animations; `player_model = kh1 / kh2` in
`kh1-kh2.ini`, default `kh1`) draws Sora with KH2's own model instead of KH1's. It switches at
once. Everything is read from the user's own KH2 install at run time; no model data is in the
repository.

## What is drawn

| Sora is | Body model | Keyblade |
|---|---|---|
| base | `obj/P_EX100.mdlx` | KH2's model of the same Keyblade, else KH1's |
| Valor Form | `obj/P_EX100_BTLF.mdlx` | both hands: KH2's where KH2 has that Keyblade |
| Wisdom Form | `obj/P_EX100_MAGF.mdlx` | as base |
| Final Form | `obj/P_EX100_ULTF.mdlx` | right: his Keyblade; left: the Kingdom Key (`W_EX010`) |

A form whose model doesn't load falls back to base Sora's. Files come from the same source as
the rest of the mod's KH2 data (`pose::load_kh2`): the Mods Manager's backup of the originals
when there is one, else the live files.

**Keyblades by name.** KH2's Keyblade items (`03system.bin` `item`, type 2) carry a weapon
number; base Sora's weapon set in `03system.bin` `went` (list 1) gives the object for that
number, and `00objentry.bin` the object's model. The names come from `msg/us/sys.bar`. KH1's
held Keyblade (save `+0x36`) is matched to KH2's by its KH1 name, case aside: Kingdom Key
(`W_EX010`), Oathkeeper (`W_EX010_10`), Oblivion (`W_EX010_20`), Ultima Weapon (`W_EX010_N0`) on
the files checked. A Keyblade KH2 doesn't have stays KH1's, held as before. Its model is loaded
the first time it is held (a thread); KH1's is drawn until it is in.

## The model file (MDLX)

`kh2_formats::mdlx` reads it (container and header layouts after OpenKH's docs; our own
reading code, checked against the user's files).

- The BAR holds the model (type 4) and its textures (type 7).
- **Model**: after 0x90 reserved bytes, a header (bone count and offset, group count at
  `+0x1C`) and a 0x20-byte descriptor per group: attributes (bit 0: translucent, e.g. the
  Keyblade's shine), texture index, the DMA chain's offset and tag count.
- **DMA chain**, per packet: a reference to VIF code (tag id 3, no VIF words), then one
  reference per bone matrix (its address field is the bone index, its second VIF word an UNPACK
  to the VU address of that matrix slot), then an MSCNT.
- **VIF code**: STCYCL, STMOD, STMASK, STROW and masked UNPACKs (S-8 broadcast to xxxx, V2-16
  to xyxy, V3-32, V4-32) fill VU1 memory: a header (`q1`: index count and place, the matrix
  count list's place, the first matrix slot; `q3`: vertex count and place, matrix count),
  index entries (`u, v` in 1/4096, vertex index, strip flag) and vertices (bone-local, grouped
  by matrix: the count list says how many per matrix).
- **Strip flags**: `0x10` only feeds the strip, `0x20` and `0x30` draw a triangle with the two
  windings.
- **Textures**: GS uploads (32-bit image transfers: BITBLTBUF, TRXREG and a data reference
  each) and per texture a `TEX0` (8-bit indexed, a 256-colour 32-bit palette, CSM1). The
  transfers are replayed into a 4 MiB copy of GS memory with the 32-bit page layout and read
  back through the 8-bit one (its column table derived from where a 32-bit upload puts known
  bytes); CSM1 swaps index bits 3 and 4; alpha `0x80` is opaque.

## Drawing

- **Pose**: `pose` hands `model` each frame's KH2 pose (body and Keyblade, KH2 model space),
  KH1's placement of the model and KH1's size against KH2's (`Retarget::hip_ratio`). Valor's
  and Final's left Keyblade comes from `leftblade::kh2_left`.
- **KH1 animating him** (its own moves, a KH1 move style, scripted walks): KH1's pose is carried
  onto KH2's skeleton (`kh1_data::inverse`) through the same body parts `retarget` measures, in
  reverse. Each limb part's frame (its direction, the knee or elbow for its roll, the knuckle
  line for the hand) is measured on KH1's *joint positions*, taken at the KH1 joints nearest the
  KH2 joints that define the part (the binds, KH2's scaled by the hip ratio), and the part turns
  by its frame now against its frame in KH1's bind. The torso parts (pelvis, chest, head) take
  the turn of a KH1 bone that part alone turns; the spine and neck between them blend the two by
  their place on the chain. Every other KH2 bone (twist joints, hair, face, hood, cloth) keeps
  KH2's bind relation to its parent; joints are placed along KH2's own bind offsets, the hips
  where KH1's are. KH1's bone rotations are never copied for the limbs: KH1's own rolls, twist
  helpers and ground IK twisted the mesh when they were (the s257 version paired bones and
  copied their turns; the user: "super contorted"). A KH2 motion taken to KH1's skeleton and
  back lands its main joints within ~7 units (knees and ankles exactly; KH2 Sora is ~155 tall).
  KH1's own Keyblade is drawn in his hand then.
- **Hiding KH1's Sora**: KH1's model queue (`0x1401D1F10`, hooked by `leftblade`) gets, for
  Sora's draw, a copy of its draw params with every body mesh masked off (`+0x38` = all ones:
  each mesh's group id is a bit there, read modulo 32). When KH2's Keyblade replaces his, KH1's
  stays queued but its 13 bones are shrunk to a thousandth about the grip for that one call
  (put back right after): clearing the weapon model (`+0x18`, s257) took his swing trail and
  effects with it (the user: no VFX for his combos). KH1 still poses his skeleton every frame,
  so hitboxes, effects, the Keyblade trail and clanks are unchanged. The Drive Form tint
  (`tint`) works on KH1's textures and so shows only where KH1's model does.
- **The pause menu's Sora**: the menu draws its own copy of his model (another instance, posed
  by KH1's menu animation, `pose`'s `MENU_SORA`). `pose` carries that pose onto KH2's skeleton
  as it stands in KH1's world (no placement of ours), the queue masks the copy's meshes, and the
  world pass draws ours with the camera KH1 has repointed for the menu. Its Keyblade stays KH1's.
- **The draw**: each time KH1's Sora is held back, the draw is prepared there and then (models,
  pose, placement), and the world-model pass (`kh2fx`'s hook, before the effects) mirrors the
  latest queue: ours is drawn on every pass whose last queue of that Sora was held back (a pass
  with no new queue repeats the last within 50 ms), and never otherwise. The triangles are
  skinned on the CPU (`kh2_anim::skin`: placement × the size ratio × the bone's pose) and drawn by
  `fx12::draw_mesh`: KH1's camera and clip transform, depth-tested and depth-writing, cut out
  below half alpha, translucent groups blended after. Model textures go to their own upload
  buffer (24 MB, appended to, keyed per model and texture).
- **Interpolated frames**: KH1 can replay a recorded frame with an interpolated camera
  (`0x140102740`, `docs/kh1-render.md` §3.2). Our commands go straight into KH1's list, so a
  replayed frame would lack them with his meshes still masked: while KH1 replays (within the
  last second), KH1's model is drawn instead and the log says so once.
- **When KH1's model is drawn instead**: no pose this frame (100 ms), the game paused (for the
  field's Sora), interpolated frames, the models not loaded, or the renderer off after a
  failure.
- **Counters**: every 20 s while the model is on, the log has Sora's queues, how many were held
  back or refused (by reason), world passes, ours drawn, any held back but not drawn, the pause
  menu copy's, the frame buffers drawn into and KH1's replays.

## Limits

- No lighting: the textures are drawn as they are (KH2 bakes most shading into them).
- Cut scenes are untested: if KH1 draws Sora there through another instance or actor, KH1's
  model shows.
- Fingers on KH1's own animations are measured at the KH1 finger joints nearest KH2's: hands of
  other proportions can curl them a little differently.

## Native research (t480 round)

Research only, for the thread "KH2 player model through KH1's model system"
(`testers/kh2-player-model-through-kh1s-model-system/`). Addresses are **Steam** (the Steam KH1
decomp, image base 0x140000000). The sections above give Epic addresses from s257-s260;
the Steam ones here were found by name in the Steam decomp (the Epic -> Steam shift in this
region is +0x2160, `steam_map.in` has `0x1D1F10 -> 0x1D4070`).

### What our draw does now (Checked (code))

- `model.rs` is off (`model::on()` returns false since s259; the ini's `player_model` is
  ignored). When on, it masks Sora's meshes in KH1's model queue (the draw params' `+0x38`),
  then after KH1's world-model pass (`0x1401ECF30`, Steam) skins KH2's MDLX on the CPU every
  frame (~12k vertices, `kh2_anim::skin`, one bone per vertex) and draws them with our own
  D3D12 pipeline (`fx12::draw_mesh`: our PSOs, our 24 MB texture arena, KH1's camera).
- So it is a parallel renderer: none of KH1's lighting, fog, Ambient fades, tint, sorting,
  interpolated-frame replay or second (lit) pass applies, and the pause menu / replayed frames
  need special cases. Textures are KH2's PS2 textures replayed through GS memory, not KH2 PC's
  HD ones.

### KH1's model pipeline on PC

1. **Model header.** A `.mdls` section 0 is a `MOBJ`; its model header (`MOBJ + word 8`) holds
   the bone count, the bone table (0x30-byte bones: KH2's MDLX bone packed tighter, links
   word: parent / sibling / child in 10-bit fields, bits 30-31 a flag) and the PS2 mesh
   entries. **Checked (code)** (`kh1_data::mdls`, from the user's file).
2. **The remastered (HD) table.** At setup, `0x1401DC7F0` (called from the model setup
   `0x1401D7C30`) finds the file record holding the model (`0x1400E2B20`), looks in its
   remastered-asset list (`+0x670`) for the asset tagged with the model's offset (`+0x54`) whose
   data (`+0x60`) starts with `0x4126` (the `.cvbl`), builds a table from it with
   `0x1401DC100`, and writes marker `0x96969696` at model `+0x10` and the table's handle at
   `+0x14`. **Checked (decomp)**.
   - `0x1401DC100`: keeps the cvbl pointer, makes one texture per slot used by the meshes
     (`0x14010BDF0` from the model's own texture records: GS address, CLUT), and turns each
     mesh entry's fields into handles: entry `+0x14` -> its texture, `+0x18` -> a second texture
     (or 0), `+0x1C` -> the mesh's packet list inside the cvbl. A second, 0x20-byte table
     (count in cvbl `+8` low 16 bits when bit 16 is set) holds texture-animation records.
     Then `0x1401DEBE0(cvbl, 0)` builds the GPU vertex buffers. **Checked (decomp)**.
   - The accessors every user goes through: `0x1401DD6D0(model)` = mesh count (the HD table's
     `+4` when the marker is there, else the PS2 header's `+0xC`), `0x1401DD650(model, i)` =
     mesh entry i. **Checked (decomp)**.
3. **Vertex buffers** (`0x1401DEBE0`): per mesh, by the entry's type byte (`+0x12`):
   - type 8: one GPU vertex of 0x24 bytes per 48-byte cvbl vertex (position, UV, normal, a
     16-bit matrix slot): rigid, one matrix per vertex;
   - type 9: 0x44-byte vertices with up to 6 (matrix slot, weight) pairs: weighted skinning;
   - type 10 (and others below it in the function): not read this round.
   Buffers are made once (`0x1400FA6D0(count, stride)`, filled, `0x140109640`), freed with
   `param 1`. **Checked (decomp)**. The 48-byte layout itself: **Checked (files)** (s195,
   `docs/kh1-tint.md`).
4. **Packet lists.** A mesh's list is a run of chunks from `+0x10`, each `{tag, size, ...}`,
   the next at `+size`, ending at tag `0x8000`. Tag `0x11` carries a 64-byte matrix and the
   constant slot it goes to; any other chunk is a draw: `+0x10` draw flags, `+0x18` the vertex
   buffer's handle (drawn by `0x1400FBD30`). **Checked (decomp)** (`0x1401DD7F0`).
5. **The queue** `0x1401D4070(instance, view, params, actor)` (Steam; Epic `0x1401D1F10`):
   one 0x68-byte model record per model (with a matrix block copied from the instance's world
   matrices `instance+0x1B4`), and one 200-byte mesh record per visible mesh
   (`0x142401BB8 + (half * 0x1E0 + n) * 200`: `+0` the model record, `+8` flags, `+0xA`
   texture slot (entry `word0 >> 24`), `+0xB` mesh index; the mesh's group id is
   `word0 >> 8 & 0x1F`, tested against params `+0x38` (hidden) and `+0x3C`). The weapon is a
   second model record on the same matrix block with a bone offset. **Checked (decomp)** for
   the mesh records and mask; the matrix copy (bone 0 and flagged bones) is from the s213
   notes (Epic) and not re-read on Steam.
6. **At draw** (the world pass's sorted list, `0x1401D7400`): `0x1401D83A0` takes the mesh
   record's model record and fills the packet list's `0x11` chunks from the model record's
   matrix block, each chunk's bone index plus the record's bone offset (`0x1401D7FB0`; lit
   meshes take two matrices per bone). Then `0x1401DE700` sets the programs (0x12 / 0x13 /
   0x14, Ambient and fog from the per-mesh globals) and walks the list (`0x1401DD7F0` /
   `0x1401DD890`): each `0x11` chunk writes its matrix into constant slot `n` (renderer
   `+0x5BA8 + (n + 6) * 0x40`, `0x140107870`), each draw chunk draws its buffer. So KH1
   **skins on the GPU** with a per-draw matrix palette. **Checked (decomp)**.
7. **Palette size.** The constant block copied per draw is 0xD80 bytes (the renderer's upload
   of `+0x5BA8`, in the decomp's `1400f0000.c`), so slots 0..47: at most **48 matrices per draw chunk**.
   **Checked (decomp)**; that the shader reads all 48 is an **Assumption**.
8. **Posing.** `0x1402A0350` is the actor's skeleton step: it poses `actor+0x15C`'s instance
   with the plain pass `0x140391C70(world, instance, time, motion table)` or the blend variants
   `0x140394080` / `0x140393B30` (the ground-IK one `0x140392560` after). The instance's arrays
   are freed by `0x140393F60` (`+0x1BC..+0x1CC`). **Checked (decomp)**.

### What a KH2 model through this pipeline needs

- **Its own skeleton.** The palette chunks index the model record's matrix block by bone, and
  that block is the instance's. KH2 Sora has 256 bones laid out differently from KH1 Sora's
  293 (no one-to-one map: KH2's twist joints, face, 11 hair spikes against 14, cloth). So the
  KH2 mesh needs a second instance with KH2's skeleton (a KH1 model header with KH2's 256
  bones, which KH1's bone format holds as is), queued by a second call of `0x1401D4070` for
  Sora's draw (as `leftblade` already queues a second model). Sora's own instance stays for
  everything else: hitboxes, lock points, effects on bones, the Keyblade's bones 280..292,
  events that read his bones, ground IK.
- **A KH1-format mesh built from the MDLX** (at load, once): an HD table and packet lists in
  cvbl layout, type 8 meshes (our MDLX reader gives one bone per vertex; whether P_EX100 has
  weighted vertices is **Assumption**: our reader would have dropped them), split into draw
  chunks of at most 48 matrices, `0x11` chunks naming KH2 bones, vertex buffers built by KH1's
  own `0x1401DEBE0` (or the same calls it makes). Normals: the MDLX vertices we read carry
  none, so they would be computed from the triangles (**Assumption** that KH1's lit pass needs
  them; the unlit pass uses Ambient only).
- **Textures** through KH1's binding: a texture descriptor whose `+8` is an `Axa::EntryTex`
  (cloned from one of Sora's, `+0x70` our GX texture made with KH1's creator, Steam
  `0x1400F9DE0`, `docs/kh1-tint.md`). The pixels could be KH2 PC's own HD textures (the KH2
  PKG's remastered assets for `obj/P_EX100.mdlx`, readable with `egs::Archive::read_remastered`)
  instead of the PS2 ones: **Assumption** that KH2's PKG carries them for this file, not
  checked this round.
- **Pose for the KH2 skeleton**, three ways:
  1. each frame, KH1's posed skeleton carried onto KH2's (`kh1_data::inverse`, already written)
     into the second instance's `+0x1B4`: follows everything KH1 plays (blends, cutscenes, KH1
     moves), ~7 units off KH2's own joints after the round trip, ~256 bone matrices of CPU a
     frame;
  2. KH1's own skeleton pass on the second instance with KH2's motions baked for KH2's skeleton
     (`kh1_data::bake` takes any skeleton), at Sora's motion slot and time (`+0x168`, `+0x16C`)
     and KH1's blend state: exact KH2 joints, but every motion baked twice and the blend state
     mirrored; KH1 motions with no KH2 bake (cutscenes, KH1 moves) would still need way 1;
  3. no second skeleton: rebind KH2's vertices onto KH1's 293 bones (nearest matched bone):
     cheapest, but KH1's bone lengths stretch KH2's mesh and KH2's extra joints are lost.
- **What KH1 reads that this touches.** Draw only: KH1's events and scripts read Sora's actor
  and his own instance, which stay. The pause menu's Sora is another instance of KH1's model
  (s260): it would need the same second model. Drive Form models: one converted model each.
  The Keyblade: KH1's, on Sora's own bones 280..292, unless KH2's Keyblade models are converted
  too (on KH2's weapon bones of the second skeleton).

### Performance

- Now (when on): CPU skinning of ~12k vertices, a vertex upload and our own PSOs every frame.
- Native: vertex buffers made once, GPU skinning by KH1's shader, one more queue call a frame
  (about 10-20 more mesh records), plus the pose: way 1 ~256 matrices on the CPU, way 2 one
  more KH1 skeleton pass. KH1's Sora meshes are masked off, so the GPU draws about the same.
