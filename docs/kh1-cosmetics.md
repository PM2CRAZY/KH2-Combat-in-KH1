# Cosmetics: Keyblade Trail (`win/cosmetics.rs`)

The user (t704): "in the main sub menu make a new menu called cosmetics and put a setting in there
for keyblades to always have their trail on". Config menu > KH1 in KH2 > Cosmetics > Keyblade Trail
(Normal | Always), ini `trail_always = off | on` (default off = KH1's own, trails over swings), live,
kept in the ini.

## How KH1 makes a weapon trail (Steam addresses; **Checked (decomp)**)

- The weapon object (Sora's, from the actor's `+0x14C`) holds the trail: `+0x18` the `.wpn`'s effect
  data, `+0x28` the running trail effect, `+0x30` / `+0x38` a request. The attack row's start
  (`0x1402A3980`) asks for the row's trail id (byte `+2` of the row) through `0x1402C2310 (actor, id)`:
  it ends a running trail, copies `+0x18` to `+0x30` and writes `id | 0x40000000` at `+0x38`.
- KH1's trail step (`0x1402C1F20`, called for each actor by the actor step `0x140297FE0`) with no trail
  running: a request with `0x40000000` set loses the bit and waits a frame; then, while the actor's
  `+0x1CC` bit `0x800000` is set and the actor is alive (`+0x374` bit 0 set, bit 1 clear, `0x140291FF0`),
  it makes the trail (`0x1402AE290 (actor, +0x30, +0x38)`) into `+0x28` and marks the request used
  (`-1`). With a trail running: kept while the actor's flags have `0xC001` (alive, its weapon drawn:
  `0x4000`, set by the draw step `0x1402A2E30`) and the bit is set; told to end when the bit clears;
  ended and forgotten when the flags lose `0xC001`.
- The bit is KH1's motion event 7.0 (type `0x70000` -> bit `0x800000` of the motion's event word,
  `0x140394900`); the actor step assigns `+0x1CC` from the motion's event word each frame
  (`0x14029F0A0`, keeping bit 0 only), so it is on only over a swing's trail window. Its only readers
  are the two places in the trail step (Checked (decomp): no other `+0x1CC` bit 23 read found).

## Always

Ahead of KH1's trail step for Sora, while his flags have `0xC001` and he is alive (`cosmetics::
before_step`): the bit is set, and with no trail running and none asked for the request KH1's row
start writes is written to his weapon object (`cosmetics::request`) with the trail id of KH1's plain
ground swing row, row 13 (`combat::swing_trail_id`, `kh1rows`). KH1's own step then makes, keeps and
ends the trail; putting the Keyblade away (`0x4000` gone) ends it. Nothing else of KH1's changes,
nothing is drawn by us, and with Normal the only cost is one flag check where the step is hooked.

- The hook on the trail step is `leftblade`'s when Valor's left Keyblade is installed (it calls
  `before_step` itself); otherwise `cosmetics` hooks it (Steam; first bytes `40 57 48 83 EC 20`
  checked, refused on a mismatch), at start when the ini is on, or when the row is first set to Always.
- Left Keyblade (Valor, Final Form): `leftblade`'s trail step wants the holder's trail bit on whenever
  the left Keyblade is out and writes the same request into its weapon object (its own effect data,
  its own look).

## Not checked / assumptions

- **Assumption:** row 13's trail id is the right look for every Keyblade and Drive Form (the look comes
  from each weapon's own effect data; the id picks which of its trail types). Not seen in game.
- **Assumption:** an always-on trail with Sora standing, walking or in a form looks acceptable (the
  trail effect follows the blade's bones every frame, `0x1402AE290`; not read in detail).
- Not checked in game: that the trail comes the frame after the Keyblade is drawn, and that a trail
  made this way ends cleanly when the Keyblade is put away.
- (`combat.rs` and `leftblade.rs` name the trail request `0x1402C0160`: that is its Epic RVA, as every
  `Rva` in the mod; Steam's is `0x1402C2310`, the one read here. The trail step's `0x2C1F20` here is
  a Steam address, so it is used only on the Steam build.)
