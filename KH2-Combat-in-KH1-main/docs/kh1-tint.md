# Recolouring Sora in KH1 (Wisdom Form's blue)

Notes from static reading of the Epic exe and of the user's own install (addresses are Epic VAs;
Steam RVAs from `python3 /opt/re/steam/gen.py`, listed where they matter). Used by
`crates/kh1-hook/src/win/tint.rs`.

## How a model mesh gets its colour

- Sora's per-frame draw (picked by actor kind byte `+6`) works out an RGBA from three things:
  the bytes `actor+0x370..0x373` (128 = neutral, eased each frame toward `+0x350..`), the floats
  `actor+0xA0..0xAC` (KH1's fades) and the scene's light product `actor+0x430`. That colour is
  passed down to the model queue (`0x1401D1F10`) and, per mesh, ends up as the shader's
  **Ambient** constant (RGBA / 128).
- When Sora counts as lit (`actor+0x420 != 0`) the mesh draw forces Ambient to white, so writing
  a tint into the actor fields does nothing in those rooms. Hence the per-draw approach below.
- The pixel is the texture times Ambient, then mixed toward a fog colour by a factor f:
  `out = tex·A·f + F·(1 − f)`. There is no colour matrix: each channel is scaled and offset on
  its own, so a colour can only be pulled toward another, not have its channels swapped.

## The constants

- `0x1400FD9C0()` returns the renderer singleton R (a static, `0x1422BB580`).
- `0x140107770(id, ptr, 0)` sets a constant and marks the buffer dirty (`R+0x76AB`):
  `0x1001` Ambient (vec4, `R+0x5BB8`), `0x1002` FogParam (vec4, `R+0x5BC8`, f in `.w`),
  `0x2000` FogColor (3 floats 0..1, `R+0x5B34`). Same offsets on Steam.
- Ambient's alpha below 0x80/128 changes sorting and alpha test: left alone.

## Which draw function Sora's meshes go through (the PC's remastered model)

The mesh draw `0x1401DC5A0` (rcx = the queued mesh record) has two branches, chosen by bit 7 of
the record's flag byte `+8`. That byte's high nibble is the low nibble of the mesh entry's third
byte (`word0 >> 16`), and every mesh of Sora's PC model has `0x08` there, so bit 7 is set:

- bit 7 clear (PS2-style meshes): program 0x12, then `0x1401DB7F0(packet list)` (call sites
  `0x1401DC77D` / `0x1401DC797`). **Sora's PC model never goes this way**, which is why the old
  hook on `0x1401DB7F0` never matched.
- bit 7 set (remastered meshes, the branch at `0x1401DC7A1`): returns early for a `0x96969696`
  header with no table at `+0x14`; otherwise sets render-state `R+0x84 = 2`, then by mode
  (lit bit and mesh flag 0x10, same table as before):
  - mode 0: program 0x12 (`0x1401DA640`, sets Ambient / FogParam from the per-mesh globals), then
    **`0x1401DB690(packet list)`** (call at `0x1401DC91D`);
  - mode 1 (lit): same, Ambient forced to white first, then `0x1401DB690`;
  - modes 2 / 3: program 0x13, then `0x1401DB730(packet list)`;
  - mode 4: program 0x14 and the same loop inlined (not used by Sora).
  Then `R+0x84 = 1` again.
- Sora's meshes have no 0x10 / 0x20 type bits, so they always take mode 0 or 1:
  **`0x1401DB690`**. Its rcx is the decoded packet list, the same value as `handle(entry+0xC)`
  that `tint.rs` already reads from the alternate table, so the existing matching works once the
  hook moves there.
- `0x1401DB690` walks the list from `+0x10`: chunks of tag 0x11 set a bone matrix (slot `+0xC`,
  matrix at `+0x10`), other chunks set blend/test bits from `+0x10` and draw the vertex buffer
  whose handle is at `+0x18` (`0x1400FBED0`, "drawVertexBuffer"); chunk size at `+4`, ends at
  tag 0x8000. Constants set before the call apply to the whole mesh.

Hook points (prologues are plain, nothing RIP-relative):

| function | Epic | Steam RVA | prologue (6 bytes, two whole instructions) |
|---|---|---|---|
| draw, programs 0x12 | `0x1401DB690` | `0x1DD7F0` | `40 53 48 83 EC 30` |
| draw, program 0x13 | `0x1401DB730` | `0x1DD890` | `40 53 48 83 EC 70` |
| old PS2-style draw | `0x1401DB7F0` | `0x1DD950` | (unchanged) |

To know the mesh index directly instead of matching packet lists, `0x1401DC5A0` (Steam
`0x1DE700`, prologue `48 89 5C 24 10` = one 5-byte instruction) gets the mesh record: `+0` the
model record (its `+0x18` is the model header), `+8` flags, `+9` second-pass flag, `+0xB` mesh
index. Setting a thread-local "current mesh" there and changing constants in the
`0x1401DB690` hook is equivalent; matching the packet pointer is simpler.

## Sora's PC model

His model file is still `xa_ex_0010.mdls`, but the PKG entry carries remastered assets next to
it (read with the same cipher as the file: each asset's bytes follow the original data in order,
the first 0x100 bytes decrypted with the entry header as seed, then raw deflate after a 2-byte
zlib header; see `kh2_formats::egs`, which currently skips them). For this file there are 48:
21 sounds, two `.cvbl` meshes and 25 DDS textures (all uncompressed 32-bit BGRA, one mip):

- `-c0.cvbl` is the remastered body: exactly the table the game logs (count 9 at `+4`,
  `0x00010002` at `+8` = skip 2, so entries at `+0x50`; packet lists at the logged offsets). In
  the file, entry word1 = word2 = **texture slot** (0, 1, 2, 3, then 1 ×5); at load they become
  handles (the …330/340/350/360 seen at run time) and the slot is put in `word0`'s top byte.
- Its textures are `-1c0.dds` .. `-1c3.dds` (slots 0..3; `-1c4..-1c6` are copies of the PS2
  model's textures 4..6). `-100..-106` / `-66380..-7e380` are the PS2 model's 7 textures under
  two names. `-575f0.cvbl` is a one-mesh untextured model (second section of the file).
- Vertices are 48 bytes: normal (3 floats), a flags word, UV (2 floats), 1, 0, position (4 floats).

Plotting each mesh's UVs on the slot's texture fits every case (mesh 0 avoids the shoe corner
of its atlas, mesh 2 lands on the pocket shapes, etc.), so the slot → file mapping is solid:

| mesh | slot / file | size | content |
|---|---|---|---|
| 0 | 0, `-1c0` 1024² | 2296 verts | face, skin, ears, hair, **plus a small red collar strip** (17 verts on red) |
| 1 | 1, `-1c1` 1024² | 1240 verts | **the clothes**: red jumpsuit, black hood/jacket with white and blue, zipper, crown, a little neck skin |
| 2 | 2, `-1c2` 1024×512 | 430 verts | the big white shorts pockets with the blue stripe |
| 3 | 3, `-1c3` 1024×512 | 548 verts | shoes and the yellow pouches |
| 4–8 | 1, `-1c1` | 3–105 verts | chain, belt, buckle (top strip of `-1c1`; drawn in both passes, blend bit set) |

So the clothes are meshes 1 and 4–8 (texture `-1c1`), with mesh 0 needing its red collar strip.
The jumpsuit shares `-1c1` with the jacket, zipper, crown and a little neck skin, so a per-mesh
Ambient/fog tint on mesh 1 tints all of those (and can't make red truly blue: the shader can't
mix channels).

## Recolour by texture swap (the way to a real blue jumpsuit)

How the PC binds a model texture (`0x1401D4690`, `0x1401D4C70`):

- mesh entry `+4` (= `+8`) → handle of a 16-byte texture descriptor X: `+0` GS base (u16),
  `+4/+6` PS2 size, **`+8` handle of the source `Axa::EntryTex`** (the HD texture, shared by every
  user of the model, menu Sora included), `+0xC` handle of a per-object copy (made on first use
  by `0x14010C930`, refreshed each draw by `0x1400F81C0` "copyTexture", only for meshes with
  texture-animation patches, e.g. the face). Without patches the source itself is bound to the
  GS address (`0x14010DD90`).
- `Axa::EntryTex`: `+0` vtable, `+0x10` magic `0x02F02EB5`, `+0x14` name (32 chars; log it, the
  exe's own list suggests names like `xa_he_1000_mdls2c0.dds`), `+0x68` the DDS file in memory
  (set at load; clones copy the pointer), **`+0x70` the GX texture object** (what is sampled and
  what copyTexture copies), `+0x7C/+0x7E` width/height (u16), `+0x94` scale divisor.
- `0x1400F9F80(R, &desc, pixels, name, 0)` (Steam `0xF9DE0`) creates a GX texture with initial
  data: `desc` is a plain `D3D12_RESOURCE_DESC` (dimension 3, width, height, depth 1, mips 1,
  format 0x57 = B8G8R8A8_UNORM, 1 sample), `pixels` tight rows (the DDS data after its 0x80-byte
  header). It is synchronous (staging texture, its own list under the critical section at
  `R+0x7A50`, executes and waits) and is how the game itself makes them at load; the result is
  the same kind of object as `EntryTex+0x70`.

Plan:

1. Find the source texture: Sora's alternate table entry 1 → X = handle(`+4`) → src =
   handle(X+8); check magic `0x02F02EB5`, size 1024×1024. Same for entry 0 (`-1c0`, for the
   collar).
2. Get the pixels: the DDS at `src+0x68` if it still reads `DDS `, 1024×1024, A8R8G8B8 masks;
   otherwise read `-1c1.dds` from the PKG (extend `egs::Archive` to return remastered assets;
   `Kh1Install::read` already finds the entry).
3. Recolour a copy: per pixel, hue within ~12° of red and saturation above ~0.55 (soft edges) →
   hue 225°, same saturation and value. On `-1c1` that is about 32 % of the texels: the whole
   jumpsuit; skin, the white/black jacket, zipper, chain and the blue trim stay. On `-1c0` it is
   0.8 %: the collar strip (and an unused pouch hole); skin and hair untouched.
4. Create the GX texture once with `0x1400F9F80` from the game thread (e.g. in `refresh` on
   entering the form), keep it for the session.
5. In the form, write ours into `src+0x70`; on leaving (fade end), put KH1's back. Both stay
   alive, so frames in flight are safe. Everything that samples or copies the source follows the
   pointer, so per-object copies and the menu's Sora come along.

The per-mesh Ambient/fog hook (moved to `0x1401DB690`) stays useful for a fade or for cooling
the white jacket panels, which the texture swap leaves as they are.

## Open runtime checks

- The name at `EntryTex+0x14` for slot 1, and that `+0x68` still holds the DDS after loading.
- That `0x1401DB690` matches Sora's 9 packet lists (it should: same values).

## Stage 195: the swap is what the mod does

`tint.rs` no longer hooks a draw. On the game thread each KH2 frame in the form it finds the
source textures of Sora's mesh-table entries 0 and 1 (`-1c0`, `-1c1`), makes a blue copy of each
once (pixels from KH1's DDS at `+0x68`, else the PKG's `-1c<n>.dds` read at start through
`Kh1Install::read_remastered`; red within 12° (soft 6°) at saturation over 0.55 (soft 0.15) moved
to hue 225°) with `0x1400F9F80`, and writes it into `+0x70`; out of the form KH1's goes back. A
model loaded again (a new source) is left to KH1 and swapped afresh.

## Per-form looks (stage 233)

Each form's look recolours its own parts of the textures (`tint::ops`), as a swapped copy per
look, KH1's own texture back out of the form:

- **Wisdom** (blue): the reds of `-1c0` / `-1c1` (hue shift to 225°), the shoes' yellow of `-1c3`,
  and the jacket's sleeves.
- **Valor** (red): the sleeves, the belt, the hood's lining, the shoes' yellow.
- **Master** (yellow and black): the jumpsuit, the collar and the hood lining yellow, the belt black
  ("Master Form's look" below).
- **Limit**: none. KH2's Limit Form (`P_EX100_KH1F`) is KH2's model of KH1 Sora, and the model
  drawn here is KH1's own, so his textures stay KH1's (`form::after_step` passes no look).

Where the parts are, from plotting the remastered mesh 1 (and 3)'s vertices on the texture by
the bone each follows (`-c0.cvbl`: tag 0x11 chunks give matrix slot → bone at `+8` / `+0xC`, a
tag 1 chunk's vertices name their slot in the low 16 bits of the flags word):

| part | texture | region (u, v fractions) | bones |
|---|---|---|---|
| sleeves (cream, black trim beside) | `-1c1` | 0.035..0.236 × 0.415..0.85 (and the cream to 0.24 above 0.62) | arms 90 / 195 and their sleeve chains (128.. / 233..) |
| belt (blue band with holes) | `-1c1` | 0.035..0.57 × 0.07..0.14 | hips (meshes 4..8) |
| hood lining (blue band) | `-1c1` | 0.755..1 × 0.72..1 | hood chains under the neck (79.., 85..) |
| shoes (all the yellow) | `-1c3` | whole | legs and feet 18..24 / 44..50 |

Sleeves are light and grey, so they are coloured ("colourize": hue and saturation given,
brightness × 0.85 kept) only where texels are light and unsaturated (the black trim and the
zipper rail stay); the belt and lining are blue, hue-shifted within their regions; the shoes'
yellow (hue 48° ± 26°) only where bright (≥ 0.42), so the dark brown beside them stays.

## Anti Form: Shadow Sora's look

The user: "make sora look like shadow sora in this form (the miniboss from neverland)", then
"lets see if you can try copying the exact textures of KH1 anti-sora onto sora's anti form model
instead of replacing the model".

### KH1's Anti-Sora: `xa_pp_3020.mdls`

Found in the user's install (Epic and Steam alike, in `kh1_fifth`). The file names come from
OpenKH's KH1 name list (`resources/kh1idx.txt` of the user's OpenKH tools); the candidates were
read from an OpenKH extraction of the same install, for study only, outside the repo.

- `xa_pp_3020.mdls` shares Sora's PS2 model layout: its remastered assets have the same
  offset-named entries as `xa_ex_0010.mdls` (`-575f0.cvbl`, `-66380.dds` .. `-7e380.dds`; of all
  the models only Sora's own variants `xa_ex_0012` / `xa_ex_0019` have them too), the same
  293-bone skeleton, and its `-c0.cvbl` posed in the bind pose has the same 2828 triangles in the
  same bounds as Sora's.
- It is **Sora's PS2 mesh with its PS2 UVs**, 9 meshes on **seven 512² textures** `-1c0..-1c6`
  (meshes 0..6 on slots 0..6, then 7 on 3 and 8 on 5, the second pass), which lie exactly like
  Sora's PS2 textures `-100..-106`: 0 skin, 1 hair and shoe, 2 hair, 3 hood/jacket, 4 jumpsuit,
  5 pockets/belt/crown/pouches, 6 the face. Each is a dark grey shading pass (the folds as soft
  greys, brightness mostly under 20 %), no colour, **no eyes or mouth** on the face (only a nose
  shadow), and no frame sheets (`-2c0`/`-2c1`) at all. The rest of its HD assets (`-887c0` ..
  `-b97a0`) are small effect textures (a blue streak, rings, notes, stars, sparks).
- The PC draws Sora with his **remastered** mesh: the same geometry re-UV'd onto four HD atlases
  (`-1c0` 1024², `-1c1` 1024², `-1c2` 1024×512, `-1c3` 1024×512). So **Anti-Sora's textures do
  not line up with Sora's**: different sizes (512² against 1024²) and a different UV layout
  (seven PS2 atlases against four HD ones). Sora's atlases each gather parts of several of
  Anti-Sora's (by the shared triangles: `-1c0` takes from his 0, 1, 2, 6; `-1c1` from 3, 4, 5;
  `-1c2` from 5; `-1c3` from 1 and 5).

### Carrying them over: by the surface (`kh1_data::texmap`)

Because the geometry is the same, each texel can be mapped exactly: a texel of Sora's atlas lies
on one of his triangles (rasterised in UV space, edge texels by up to a texel outside), so it is a
point on the bind-pose surface; the nearest point of Anti-Sora's surface (a grid of his
triangles, widened until the nearest is inside the search) gives his UV there, and his texture is
sampled (bilinear). Measured on the user's files: the gap between the two surfaces at every
covered texel is 0.0003 units on average (Sora is 152 units tall), 0.08 at most, so each texel
takes Anti-Sora's colour from the same spot of the same triangle.

- `.cvbl` details used (our own reading): after the 16-byte packet-list header, chunks of tag
  `0x11` (bone `+8` into matrix slot `+0xC`), tag 1 (`count` at `+8`, 48-byte vertices from
  `+0x20`) and `0x8000` (end). A vertex's flags: low bits the matrix slot, high 16 bits `0x1000`
  (starts a strip, no triangle) or 1 / 2 (a triangle with the two before, alternating winding).
  Positions are in their bone's space; KH2's bind pose of the `.mdls` skeleton
  (`kh2_anim::Pose::bind`) places them.
- The meshes' stored normals don't follow the winding (about half disagree), but the winding is
  the same in both models: oriented by it, 2475 of Sora's 2828 triangles have an exact twin in
  Anti-Sora's mesh facing the same way. A source triangle facing the other way counts as further
  (so the two sides of a double-sided part stay apart); the rest (strips split differently) find
  their spot by distance.
- Sora's alpha is kept (his hair cards' cut-outs belong to his UV layout). Texels no triangle
  covers take their neighbours' colour (16 passes), then the mean.
- The face's frame sheets: KH1 pastes a 320×128 eye frame at (0, 80) of `-1c0` and a 192×128
  mouth frame at (56, 240) (found at run time by matching each sheet's first frame against
  Sora's face: mean difference about 6 of 255). Anti-Sora has neither, so every frame of both
  sheets becomes the new face's own pixels there.
- The yellow eyes (`ANTI_EYES`, on): Anti-Sora's textures have none, so solid yellow blobs are
  drawn where Sora's eyes are (on his own `-1c0` and on each of his `-2c0` frames, so they still
  blink); see "Anti's eyes" below.

In the mod (`tint.rs`): at start, on its background thread, after Sora's own textures are read,
`bake_anti` reads both models and Anti-Sora's seven textures from the packages (one key table,
`kh1_data::Files`), carries them over (about 3 s on the user's PC for all four atlases) and keeps
them. Entering Anti Form swaps them in as the other looks do (KH1's own come back on leaving);
before they're ready nothing is swapped. If anything fails (a file missing, a mesh that doesn't
parse, a size that doesn't match KH1's texture), the log says why
(`form tint (Anti): Anti-Sora's textures not made, …` or `no Anti-Sora texture for …`) and that
texture gets the Shadow recolour below instead.

`kh1_fifth.hed` holds one junk entry (a length of `0xFDFDFDFD`), which `egs::Archive::open`
used to refuse with the whole package; it now leaves that entry out, so `xa_pp_3020.mdls` reads.

`cargo run --release -p kh1-data --example anti_textures -- <KH1 install dir> <out dir>` runs the
same transfer outside the game and writes the results as raw BGRA to look at.

### The fallback: Sora's textures darkened (`Op::Shadow`)

Every texel of every Sora texture becomes hue 260° (blue-violet) at saturation 0.5 with brightness
`0.03 + 0.10 × v`. So it reads as black, the folds and seams still faintly there, with a dark
sheen. Alpha is kept, so hair edges stay as they were. Both ways cover more textures than the
other looks:

| texture | what | found by |
|---|---|---|
| `-1c0`, `-1c1`, `-1c3` | face/skin/hair, clothes, shoes | mesh table entries 0, 1, 3 (as before) |
| `-1c2` (1024×512) | the shorts' pockets | mesh table entry 2 |
| `-2c0` (320×2816) | the eyes' animation frames (22 frames of 320×128: open, blinking, closed) | the model's texture-animation patches |
| `-2c1` (192×1920) | the mouth's frames | the same |

### The face's eye and mouth frames

The face mesh doesn't sample `-1c0` directly. Each draw, KH1 refreshes a per-object copy of it
(`0x1401D4C70`: copyTexture `0x1400F81C0` from the source's `+0x70`), then pastes the current eye
and mouth frames onto that copy (`0x140104450`, which also reads the frame sheet's `+0x70`). A
black `-1c0` alone would show Sora's skin-coloured eyes and mouth on a black face, so the frame
sheets are swapped too.

Where they are: the remastered model data behind the model header's `+0x14` (the object whose `+0`
is the mesh table `tint.rs` already reads) holds the texture-animation patches: a count at `+0x20`
and 32-byte records at `+0x28` (`0x1401DB370` indexes them). Each record's `+0x18` is the
`Axa::EntryTex` it pastes from. Its name ends `…_mdls2c0.dds` / `…_mdls2c1.dds` (KH1's own code
compares these names for Hercules' `xa_he_1000_mdls2c0.dds`). `tint::frames_texture` finds them
by that name. The records are filled per draw into `0x142432270` (by slot), but the model's own
array is stable.

The eyes on `-1c0` were located by plotting mesh 0's vertices by bone (as for the sleeves). Both
eyes are on the head bone (150) at u 0.01..0.30 × v 0.115..0.195 (`texmap::SORA_FACE_EYES`).

### Anti's eyes: solid yellow blobs

The user: "the anti sora model has these yellow blob eyes ... i need the eyes to be solid yellow,
and have it be the same yellow as in anti-sora's model". (Before this, s290 tinted Sora's own eye
drawing yellow, so his pupils, highlights and outlines still showed.)

Where Anti-Sora's yellow comes from, as far as his file shows (`xa_pp_3020.mdls` and its HD
assets, read from an OpenKH extraction of the user's install, outside the repo):

- Not his model: its `MOBJ` section is byte for byte Sora's (`xa_ex_0010.mdls`) up to `0x66380`;
  only the PS2 textures after that differ. No extra mesh: his remastered `-c0.cvbl` has the same 9
  meshes (all textured `-1c0..-1c6`), and every vertex's two spare words are `1.0, 0` as in
  Sora's, so no vertex colour.
- Not his textures: his seven PS2 textures (128², 8-bit, CLUTs after them) are all greys (no
  colour above 34 of 255), as are the seven HD ones.
- His file's other sections: 1 is his effects (7 of them, count at `+0xC`, 32-byte entries from
  `+0x10`; HD textures `-887c0` .. `-b97a0` are theirs), 4 a `CLS` block, 5 his battle script
  (`pp_3020.bd`), 6 a sound bank. The only yellow anywhere is in effect 2 (an untextured effect
  of thin crossed quads and squares, ~100 units across): words `R G B A` in the PS2 0..128 scale
  of `(128, 128, 64, 4..32)` (that is 255, 255, 128 at low alpha) beside a `0x00430042` word.
  Its shape reads as a glint, and nothing ties it to his head or eyes; the format isn't decoded.

So the colour is taken from the user's reference picture of Anti-Sora instead: the brightest
texels at the centre of each eye average (253, 230, 95) (left eye 254, 230, 93; right 253, 231,
97), `texmap::ANTI_EYE_RGB`. If KH1's own eye glow is ever found in his data, change it there.

How they are drawn (`texmap::solid_eyes`, `solid_eye_frames`): each half of the eye area (one
eye; a 320×128 frame of `-2c0`, or `SORA_FACE_EYES` on `-1c0`) takes the texels of Sora's eyeball
(iris: hue 220° ± 40°, saturation over 0.25, any brightness, so the pupil too; white: saturation
under 0.25, brightness over 0.55), fits an ellipse to their spread (weighted centre and second
moments; a filled ellipse has variance a²/4 along each axis), enlarges it by 1.12 and fills it with
the one flat colour, opaque (alpha kept; a one-texel edge blended). It is clipped to that eye's
half. Nothing of Sora's eye drawing is kept: the Anti-Sora face under it is his dark skin (from
`texmap::transfer`), and on the fallback Sora's darkened face. An eye with fewer than 12 texels of
eyeball is shut and gets no blob, so the blink frames stay shut: of the 22 frames, 9, 12, 19 and
21 are shut, 8 (the lids nearly closed) shows a thin slit, the rest full blobs. Done on Anti-Sora's
textures (`bake_anti`) and on the `Op::Shadow` fallback alike.

`anti_textures` (the example) now writes the frames with the eyes and `anti-1c0-eyes` (the face
with them).

Not checked in game: that the patch records' `+0x18` is set for Sora on PC and that the names
match. `form tint (Anti): …mdls2c0.dds ready` in the log confirms both. Also not checked in game:
how the carried-over textures look under KH1's lighting, and whether KH1 draws its Anti-Sora with
anything more than his textures (a tint, transparency or an effect around him); only the
textures are copied.

## Limit Form's look (s293)

The user: "change sora's outfit in limit form to have his jumpsuit be black, his belt yellow, and
his shoes black". `Look::Limit` on KH1 Sora's own textures:

- `-1c0` (collar) and `-1c1` (jumpsuit): the reds black (`Op::Grey`, brightness 0.04..0.2 with
  the folds kept, as Final's belt).
- `-1c1`'s belt (`BELT`): the dark blue made a bright yellow (`Op::Paint`: hue 45, saturation
  0.85, brightness 0.55 + 0.45 × v; a plain hue shift of the dark blue came out olive). It runs
  first, so the black pass leaves it.
- `-1c3` (shoes): the yellow, the red and the blue black. The sole's grey and the laces' metal stay.

### The jacket black too

The user: "make limit forms jacket fully black as well including the inside of his hood". Then,
on `-1c1`, the whole jacket goes black, shaded like the jumpsuit (`Op::Darken`: every texel of
the part made grey at `0.04 + 0.16 × v`, never brighter than it was, so the jacket's own black
trim stays black, a little darker). Its cream is about the hue and saturation of the silver
(hue ~40, saturation ~0.14), so no colour test can tell them apart: the jacket is found by the
mesh.

Mesh 1 of `-c0.cvbl` (the clothes) splits into UV islands (triangles of one mesh joined by
shared UV vertices). Plotted on `-1c1` and painted onto a render of the bind pose:

| island (u, v box) | part |
|---|---|
| 0.72–1.0, 0.0–0.53 | the shorts (red) |
| 0.26–0.77, 0.06–0.73 | the jumpsuit's top, with its zipper |
| 0.33–0.45, 0.63–0.73 | the jumpsuit's crotch (the red hexagon) |
| 0.0–0.22, 0.43–0.84 | **the sleeves** (cream, black trim on the left) |
| 0.22–0.67, 0.63–1.0 | **the vest** (black, cream hem); its open zipper (teeth and tape, u < 0.257) is painted on the same triangles |
| 0.77–1.0, 0.54–1.0 | **the hood**, outside black, lining blue |
| 0.72–0.76, 0.64–0.74 | **the sleeves' inside** (black and blue, seen at the arm hole) |
| 0.0–0.19, 0.85–1.0 | the crown |
| 0.82–0.94, 0.9–1.0 | the shoes' tongues |
| 0.21–0.57, 0.08–0.31 | the belt and its sides; meshes 4–8 the chain, belt and buckle |

The cream with the blue stripe and the metal disc (0.0–0.5, 0.18–0.42) and the cream beside the
hood are on no triangle of slot 1.

`Mask::Jacket` (`JACKET` in `tint.rs`): one point (u, v) in each bold island; at start the hook
reads `-c0.cvbl` from the PKG (`texmap::cvbl_uv_triangles`), and when the Limit texture is made,
`texmap::island_mask` rasterises the islands holding those points (a texel past their edges, for
filtering), less the vest's zipper (u 0.2–0.257, v ≥ 0.6). Until the mesh is read the Limit
`-1c1` isn't made; if it can't be read (or none of the points lands on a triangle) the jacket
keeps its colours and the log says so (`form tint: … -c0.cvbl not read`, `… of the jacket's 4
parts found`). Checked on the user's files in a numpy preview and a software render of the bind
pose: the jacket and the hood's inside black, the chain, crown, zipper and its rail silver, the
belt yellow. Blue still left on triangles of slot 1 is all the belt's (s293's `BELT` rect
leaves its less saturated rim and the strap end by the buckle, u 0.67–0.77, v 0.0–0.07).

## Master Form's look

The user: "I want sora's jumpsuit to be yellow, and the inside of sora's jacket's hood be yellow as
well instead of blue. I want soras belt to be black". `Look::Master` on KH1 Sora's own textures:

- `-1c1`'s belt (`BELT`): the blue black first (`Op::Grey`, brightness 0.04..0.2, as Final's
  belt), so the passes after leave it.
- `-1c1`'s hood lining (`HOOD_LINING`): the blue painted yellow (`Op::Paint`, Limit's belt
  yellow: hue 45, saturation 0.85, brightness 0.55 + 0.45 × v, the bright yellow near the
  shoes'). A plain hue shift of the dark blue comes out olive.
- `-1c1`'s reds (the jumpsuit, its top and shorts): the same yellow, the folds kept by the
  brightness. A lower floor (0.2) was tried for the dark folds and gave a mustard jumpsuit, so the
  one yellow stays everywhere.
- `-1c0`: only the collar's red strip (`COLLAR`, u 0..0.025, v 0.34..0.70: the texture's one
  large red run, 3460 texels at u 0..0.018, v 0.35..0.69), in the same yellow. `-1c0`'s other red
  is an oval on the shoe piece (u 0.40..0.49, v 0.63..0.70, 5233 texels), which the other looks'
  red passes recolour too. It is left out here: the user asked for no change to the shoes.
- `-1c3` (shoes), the sleeves, the jacket and its blue trim outside the lining: unchanged.

Checked on the user's files (`xa_ex_0010.mdls`'s `-1c0` / `-1c1`) in a numpy copy of the ops and a
software render of the bind pose (the helper's `/home/claude/logs/master-look/`, outside the repo):
the jumpsuit and collar yellow, the hood's lining yellow at the neck front and back, the belt
black, the shoes as KH1's. Left: a few texels of dark red that are too unsaturated for the pass
(saturation under 0.55) inside the shorts' leg hole, now brown-yellow specks, seen only from below.
