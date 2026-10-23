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
| Limit Form | `obj/P_EX100_KH1F.mdlx` | as base |
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
  How the copy is told from Sora: a Sora model other than the field one while KH1's pause
  machine is up (`0x14232E9B4` non-zero, `kh2fx::pause_menu_up`), or one drawn more than 150
  units from his actor. The distance alone failed: the menu draws its copy about the world's
  origin, so with Sora standing within 150 units of it (the user's log, in Final Form) the copy
  was taken for him, rebuilt as "Sora's instance", given KH2's last pose (frozen: the game is
  paused) and his own model rebuilt again on resume (the user: "pausing in final form plays
  sora's Final form idle ... freeze framed"). (high: the log; the origin from the distances
  logged against his position, medium.)
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
