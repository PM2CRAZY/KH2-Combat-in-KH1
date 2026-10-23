# KH1's prizes (HP / MP orbs, munny), and KH2's Drive orbs among them

The user: "For the drive orbs, I want them to run through KH1's engine, not an overlay", then
"keep the drive orbs running through KH1s engine, but if possible, let's try to get the texture
from KH2". These notes come from the Epic executable (image base `0x140000000`); every address
the mod uses is in `steam_map.in`, and the hooked prologues and the table reads were checked byte
for byte in the Steam build. They contain no decompiled code. Code:
`crates/kh1-hook/src/win/prize.rs` and `prizeart.rs`.

## How KH1's prizes work (Checked, exe)

**The objects.** A prize is a task in its own task list (`0x142D619B0`): 256 entries of 0x40
bytes at `0x142D619D0`, each stepped by `0x1402AA940`. Beside each is a draw record, 0x20 bytes
at `0x142432B40 + index × 0x20`: its position, its scale, `+0x18` its type, `+0x19` bits (live,
shown). More than 0xCC live prizes and the oldest settled one is dropped (`0x1402A9E60`).

**The types** (the task's `+0x1C`; the switch in the pickup, `0x1402AA16D`):

| Type | What | Reward | Row (`0x1405298E0 + type × 0x14`: height, pull range, sound, second id, munny shown) |
|---|---|---|---|
| 0 | munny | 1 (`0x1402E59E0`) | 10, 0, 24, 47, 1 |
| 1 | munny | 5 | 15, 0, 24, 47, 5 |
| 2 | munny | 20 | 30, 0, 24, 47, 20 |
| 3 | HP orb | 1 (`0x1402A2790`) | 15, 60, 25, 48, 0 |
| 4 | HP orb | 5 | 30, 120, 25, 48, 0 |
| 5 | MP orb | 2 (`0x1402A2950`) | 15, 60, 26, 48, 0 |
| 6 | MP orb | 15 | 30, 120, 26, 48, 0 |

The row table is 7 rows; the dword after it (`0x14052996C`) is the "load `map_prize.bin`" flag.
It is read at six places only: `0x1402A9F63`, `0x1402AA217`, `0x1402AA2E4`, `0x1402AA30B` (in the
pickup, as `[r14 + type × 0x14 + table + field]` with r14 the image base) and `0x1402AA37D`,
`0x1402AA6DD` (the steps' `lea reg, [rip + table]`). Nothing writes it.

**Making them.** `0x1402A9110 (type, position, mode)` takes a free entry, sets its draw record and
its scatter (a random spread and an upward speed by `mode`). The drop functions loop over the
seven counts and call the same set-up inline:
- `0x1402A9AB0 (counts, position, mode)`: seven int counts, then three (chance, item) pairs. It is
  the enemies' drop: their AI script's drop ops (`0x1402BA490`, mode from `0x14298D3A0`, and
  `0x1402BA500`, mode 1) jump to it.
- `0x1402A97B0 (row, position)`: a row of `map_prize.bin` (loaded at `0x142D659F0`, 13-byte rows:
  seven byte counts and three item pairs, all times `0x142D619A4`), from an event script's op
  (`0x1401AED40`) and from `0x1402A9A20 (actor, which)`, an object's drop (its object's `+0x48`
  row when broken, `+0x4A` when hit; breakable objects, `0x1402C0FD6` and on).
- `0x1402BEEB0` makes the first non-empty type of a row only (a single prize).
- All of them stop while `0x142D619AC` is set.

**Drawing.** No model: the effect program `pppRyjMegaBirthPrize` (in `prize.dpx`, loaded with
`always.dpx` by `0x1402AC600`; its functions `0x14033E8D0` draw and `0x14033EBE0` clear, by the ppp
name table at `0x140554B50`) draws every live, shown draw record as a sprite. Its data block has
10 entries of 12 bytes from `+4`: an int sprite index (`0xFFFF` none), the RGBA bytes (the alpha
times the record's `+0x14`), and a float scale; the record's type picks the entry. KH1 uses
entries 0..6.

**Picking up.** Each step, `0x1402A9EF0 (prize, position)` looks at the three party actors
(`0x142D37C80`.., acting and not knocked out) and, for the first close enough (by the row's
range, wider with an ability, `0x140282A70 (5, member)`), gives the reward by type (a
type above 6 gets none: `ja` past the switch), plays the row's sound (`0x140293450`), shows the
munny count (`0x14026F9E0`, munny only) and returns that actor; the step then takes the prize to
him and ends it (its states 2..3; inferred from the step).

## What the mod adds

- **Two types**: 7 a small Drive orb, 8 a big one. KH1's seven rows are copied into a table of
  ours (memory near the game, 9 rows) with rows 7 and 8 = MP's 5 and 6 (height, pull, sound), and
  the six reads above are pointed at it (their displacements rewritten, each checked first; if any
  isn't as expected nothing is dropped). KH1's own types read the same values as before.
- **Their look**: KH2's own Drive orb (below, "The Drive orbs' look"), drawn by KH1's prize
  program. Until KH2's art is read, or if it can't be, entries 7 and 8 are MP's small and big
  orbs' entries (sprite, scale) in KH2's drive-orb yellow-orange (the MP entry's brightest
  channel times 1.0 / 0.66 / 0.12), as before. The entries before a change are logged.
- **Their drop**: after KH1's enemy drop (`0x1402A9AB0`, hooked) has made its own prizes, if it
  made any (its counts not all 0) and prizes aren't off, KH2's normal enemies' Drive orb rows are
  rolled (00battle `przt`, the 87 rows with an item drop: 62/87 none, 10/87 four small, 10/87 two
  big, 5/87 six small and two big; `drivegauge::DROPS`) and KH1 makes them (`0x1402A9110`, the
  same position and mode).
- **Their pickup**: KH1's pickup (`0x1402A9EF0`, hooked) runs as ever; when it returns a member for
  one of ours, the Drive gauge gets the orb (`gauge::orb`: 1 or 3 points, or `× 12` frames of the
  form gauge in a form, Anti's lost). Any party member's pickup counts (the gauge is Sora's, as in
  KH2, where the orbs are the party's).

- **Drive Converter** (KH2FM's support ability, equipped on base Sora): the same hook gives the
  drop's munny partly as Drive orbs, by KH2's rule (`docs/drive-model.md`, "Support
  abilities"): of the munny KH1 would make (each count times KH1's Jackpot factor
  `0x142D619A4`, as its loop does), `× 0.5` a copy is kept, the rest are Drive orbs (KH1's 20
  and 5 munny big ones, 1 munny small ones). KH1's drop then runs on a copy of the counts with no
  munny, and KH1's own spawn (`0x1402A9110`) makes the munny kept and the orbs.

The same hook on `0x1402A9AB0` drops the Drive Recovery items (KH1's field items, after its own
item pairs): `docs/kh1-items.md`, section 7. It goes in even if the orbs' parts don't.

Everything else is KH1's: the scatter and bounce, the pull toward a member, the lifetime and
blink-out, the pickup sound, the draw.

## KH2's prize art (Checked, the user's install and KH2's exe)

KH2's prizes are its object `PRIZE` (`00objentry`, an object of type 22 with no model of its own;
there are world variants, `PRIZE_TT` and the like, whose classes are in the exe). Its file
`obj/PRIZE.mdlx` (in `kh2_first`, 25 744 bytes in the user's Epic and Steam installs) is a BAR
with one entry, a PAX named `priz`, and no model. The PAX's one effect runs one program,
**`pppRyjMegaBirthPrize`**: KH2 draws its prizes with the same effect program as KH1 (the name is
in both executables), in the same data layout: per prize type an entry of 12 bytes from `+4`
(shape number, colour with `0x80` = 1.0, float scale), the shape animation speed at `+0x7C` (512).
KH2's ten entries:

| KH2 type | What | Shape | Scale |
|---|---|---|---|
| 0, 1 | HP orb small, big | 1 (a green orb with a white glow added in red) | 0.7, 1.5 |
| 2, 3, 4 | munny | 0 (the crystal) | 1.2, 0.8, 0.4 |
| 5, 6 | MP orb small, big | 2 (the bubble, its colour cycling) | 0.5, 0.9 |
| **7, 8** | **Drive orb small, big** | **3** | **0.9, 1.7** |
| 9 | none | `0xFFFF` | |

So KH2 has one Drive orb sprite, at two sizes (the big 1.89 × the small). Shape 3 is 24 steps of
two sprites each: a gold orb (26 × 26 texels of the page, normally blended, its colour pulsing
from about half to one and a half brightness) under a grey swirl ring (30 × 30 texels, added
in orange, turned a step further each step: a full turn over the 24). Its colours are all
`0x80`: the look is the texture's and the sprites' own.

**The textures**: the shapes' textures are 8-bit PS2 textures sharing two 128 × 64 pages
(GS block `0x3000`: munny on the right half, the Drive orb top left, the HP orb bottom left, each
with its own palette; `0x3020`: the MP bubble). The package's remastered images are those pages
with their palettes applied, at twice the size: **`-0.dds`** (256 × 128, DXT5) is page `0x3000`,
`-1.dds` page `0x3020` (the distinct pages in texture order). The Drive orb's sprites address
page `0x3000` with palette `0x3E00`, in its top-left quarter.

Read by `kh2_formats::prize` (the effect, its entries, a shape's raw animation, a page drawn from
the PS2 textures) and `kh2_formats::dds` (the remastered image); `cargo run -p kh2-formats
--example prize_art -- <PRIZE.mdlx> [-0.dds]` prints them from an extracted copy.

## The Drive orbs' look (`prizeart.rs`)

KH1's prize draw (`0x14033E8D0`, per live, shown draw record, in depth order) takes the record's
type's entry, and **its sprite index is an index into the running effect's shape table**:
`*0x142E9DE08` is the effect's DPD, `+0x28` its shape table, 16 bytes a shape (the animation;
at `+8` a packet size the sort buffer checks). The index is read sign-extended and scaled by 16 in
64 bits, and only `0xFFFF` means none. It steps the record's animation (`+0x1C` timer by the
program's speed, `+0x1E` the step; a new record's both 0) and queues a packet with the frame's
address, drawn later (`0x14031BDD0`, then `0x14031C4E0` / `0x14031B360` for rectangles,
`0x14031B5C0` for free quads): the frame's texture register (`+0x10`, its palette moved by the
sprite's palette offset) is set (`0x140384300`), which looks the texture up in KH1's texture
map (`0x14010D860`: by page, format and palette; the entry's GPU texture at `+0x70`, bound by
`0x140108AD0`). KH1's shape animations have KH2's layout (`kh2_formats::dpd::Shape`).

What the mod does:
- **At start** (a thread): `obj/PRIZE.mdlx` from the user's KH2 install (the backup archives if
  a mod manager keeps them, else the live ones; KH2's exe beside KH1's for the cipher),
  types 7 and 8's shape (its raw animation) and scales, and the page its frames name: the
  remastered image (`-<page>.dds`, DXT5, decoded by our own reader) if it has the page's
  shape, else the PS2 page drawn from its textures and palettes. Every frame's texture register
  gets page and palette `0x3FFF` (past the end of the GS's 16 K blocks, so no KH1 texture has it).
- **KH1's texture lookup is hooked** (`0x14010D860`): that register answers with a texture entry
  of ours; everything else goes to KH1's.
- **At the first prize draw with the art read** (on the game thread): our entry is a copy of KH1's
  own prize texture's entry (the one KH1's MP orb frame names) with KH2's image as its GPU
  texture (made by KH1's creator, `tint::make_texture`), its HD size and its PS2 size (128 × 64,
  what the frames' coordinates count in). KH2's animation is copied near the game, its steps'
  waits scaled by KH1's speed over KH2's (both 512 if KH1's is too).
- **Each draw**: two 16-byte slots of ours (KH2's animation; KH1's MP slot's packet size), placed
  at the shape table's alignment, and entries 7 and 8 = the slots' indices from the table, colour
  `0x80` (KH2's), and KH2's scales times how KH1 sizes its MP orb against KH2's (their first
  frames' reach × their small + big scales). When the shape behind 7 / 8 changes, their live
  records' step and timer go back to 0 (as a new prize's), so no step past the new shape's last.

So KH1 runs them as before: its program steps KH2's animation, scales them, fades and blinks them
(the record's alpha times the colour), sorts and draws them. HP and MP orbs and munny are KH1's
own, untouched. If anything is missing (the file, its program, the lookup's prologue, KH1's MP
texture entry, memory near the game, the table out of a 32-bit index's reach), the log says why
("Drive orbs: KH2's art not used (...)") and they stay KH1's MP orbs tinted. The log names the
file, the shape, its steps, the page, the image used, the sizes and both speeds.

## Open

- That `0x1402A9AB0` is what every enemy's defeat goes through is read from the AI ops; the log
  ("Drive orbs: an enemy's drop ...") confirms it in game. A drop on a hit, if an enemy's script
  makes one, rolls too.
- That MP's orb sprite takes the colour (a white sprite tinted by its entry, as the HP and MP
  orbs' different colours suggest): the entries' log line shows whether they share a sprite
  (only the fallback depends on it now).
- KH2's art in game (inferred from the code read, not yet seen): that KH1's renderer counts a
  sprite's coordinates in the frame register's page size (ours is the entry's PS2 size too, so
  either reading of it fits); that a copied entry with another GPU texture binds as KH1's own
  (`kh2hud` swaps the same field); the size factor from the MP orbs' first frames; and the
  animation's rate (KH2 runs its effects at one tick per two frames; how often KH1 runs the prize
  draw isn't measured, so the ring may turn faster than in KH2).
