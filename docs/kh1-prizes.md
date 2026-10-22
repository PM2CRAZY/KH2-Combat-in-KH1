# KH1's prizes (HP / MP orbs, munny), and KH2's Drive orbs among them

The user: "For the drive orbs, I want them to run through KH1's engine, not an overlay". These
notes come from the Epic executable (image base `0x140000000`); every address the mod uses is in
`steam_map.in`, and the hooked prologues and the table reads were checked byte for byte in the
Steam build. They contain no decompiled code. Code: `crates/kh1-hook/src/win/prize.rs`.

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
- **Their look**: before each run of `pppRyjMegaBirthPrize`'s draw, its entries 7 and 8 are made
  MP's small and big orbs' entries (sprite, scale) in KH2's drive-orb yellow-orange (the MP
  entry's brightest channel times 1.0 / 0.66 / 0.12). The entries before are logged once.
- **Their drop**: after KH1's enemy drop (`0x1402A9AB0`, hooked) has made its own prizes, if it
  made any (its counts not all 0) and prizes aren't off, KH2's normal enemies' Drive orb rows are
  rolled (00battle `przt`, the 87 rows with an item drop: 62/87 none, 10/87 four small, 10/87 two
  big, 5/87 six small and two big; `drivegauge::DROPS`) and KH1 makes them (`0x1402A9110`, the
  same position and mode).
- **Their pickup**: KH1's pickup (`0x1402A9EF0`, hooked) runs as ever; when it returns a member for
  one of ours, the Drive gauge gets the orb (`gauge::orb`: 1 or 3 points, or `× 12` frames of the
  form gauge in a form, Anti's lost). Any party member's pickup counts (the gauge is Sora's, as in
  KH2, where the orbs are the party's).

Everything else is KH1's: the scatter and bounce, the pull toward a member, the lifetime and
blink-out, the pickup sound, the draw.

## Open

- That `0x1402A9AB0` is what every enemy's defeat goes through is read from the AI ops; the log
  ("Drive orbs: an enemy's drop ...") confirms it in game. A drop on a hit, if an enemy's script
  makes one, rolls too.
- That MP's orb sprite takes the colour (a white sprite tinted by its entry, as the HP and MP
  orbs' different colours suggest): the entries' log line shows whether they share a sprite.
