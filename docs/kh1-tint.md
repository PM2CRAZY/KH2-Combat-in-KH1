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

## Memory: pixels on first need, one texture per slot and look (ram-use m1 answer 2 (a))

- **Before**: at every start (with `drive` on) a thread read Sora's three HD textures from the PKG
  and kept them decoded for the whole session (2 × 1024×1024 and 1 × 1024×512, 4 bytes a pixel:
  about 10.5 MB), though they are only the fallback for when KH1's own DDS at `+0x68` doesn't fit.
  **Checked (code)**.
- **Now**: nothing is read at start. When a form needs a slot's texture and KH1's copy in memory
  doesn't fit, that one slot is read from the PKG on a thread (`tint::pkg_pixels`; the swap waits
  for it, KH1's colours meanwhile), recoloured in place (no second copy) and dropped once the GPU
  texture is made. With KH1's copy in memory the PKG is never read. **Checked (code)**.
  **Assumption**: how often KH1 still has its copy at `+0x68` in play isn't known (the log says
  which: "(KH1's copy in memory)" or "(the PKG's)", "read from the PKG ... on first need").
- **The leak**: our recoloured GPU textures were kept per source address, so every area change
  (Sora's model loaded again: a new source) made new ones, and none was ever released (the mod has
  no release call for a GX texture). **Checked (code)**. Now each is kept per slot and look and
  used again for a later source of the same texture (name at `+0x14` and size): at most one per
  slot and look for the session (6 here). The one exception stays as before: when the old source
  goes away while it still holds ours (an area change mid-form), ours isn't used again, because KH1
  may have released it with its texture, and a new one is made. **Assumption**: whether KH1
  releases the object in `+0x70` with the source (then KH1's own original leaks instead) isn't
  checked; releasing ours ourselves needs KH1's GX texture release function, not looked up yet.
  These are GPU textures (VRAM, 4 MB a 1024² one), not process RAM.

## Per-form looks (stage 233)

Each form's look recolours its own parts of the textures (`tint::ops`), as a swapped copy per
look, KH1's own texture back out of the form:

- **Wisdom** (blue): the reds of `-1c0` / `-1c1` (hue shift to 225°), the shoes' yellow of `-1c3`,
  and the jacket's sleeves.
- **Valor** (red): the sleeves, the belt, the hood's lining, the shoes' yellow.

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
