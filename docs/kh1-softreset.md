# Soft Reset (t707)

The user: "in qol you can quick reset the game with l1 l2 select and start pushed together", then "if its in the game then use the same button combination" and, once the combo did nothing in the PC build, "the combo does not work in the pc version so restore it". The combo is the PS2's: L1 + L2 + R1 + R2 + Start + Select.

**Row.** Config menu, QoL, Soft Reset (Off | On, ini `soft_reset`, off by default). Off: nothing runs but one flag check in the mod's pad read.

**What KH1 has.** Its pad code still tests for the combo (mask `0x0F09` on its pad table, Steam `0x14012B3A7` and `0x14012BE03`) and raises a reset flag for the PS2 platform layer; in this build that flag does nothing (Checked (decomp + exe); the user saw the combo do nothing).

**What the mod does.** With the row on, the combo read from the mod's own pad read (`probe::pad`, the records merged, Select `0x0001`, L2 `0x0100`, R2 `0x0200`, L1 `0x0400`, R1 `0x0800`, Start `0x0008` as in KH1's mask) held for one second sets KH1's game mode back to its start-up step. KH1's mode step (`0x140170870`, a switch on the mode at `0x14233FBC0`) in mode 1 counts a timer (`0x1422EC0A8`) down and, below zero, runs KH1's own full reset (`0x140170630`: tasks, scene, sound and fade state, then mode 2, which loads the title screen as at start-up). The mod writes the mode, the mode-change countdown (`0x1422EC0AC`) and that timer, and KH1 steps the reset itself: no hook, nothing per frame. Checked (decomp) for the flow, (exe) for the first bytes of both functions and the mode's address, which are checked once at start and the row is unused on a mismatch or the Epic build. Nothing is reset when the mode is a start-up or title one (a game running is modes 4 to 0x12).

**Not checked (Assumption).** KH1's reset was written for start-up, so whatever a running world leaves behind is assumed to be cleared by it; not run in game. Unsaved progress is lost, as with a console reset; Continue loads the save. The buttons are not hidden from KH1 during the one second (Start may open the pause menu).

**Debug.** Lines are written under the Menus & Config log.

## What the reset leaves, checked against the reload report (docs only, no change)

The tester (auto-cutscene-skipper, after t717): after a soft reset and loading his save, the Fairy Godmother in Merlin's house acted as if already talked to, and the Cheshire Cat gave Blizzard early.

- **A load restores the whole save block.** KH1 reads the save file straight into `0x142DE9360`, `0x16C00` bytes (`0x1402E0000` area `_read`, and the slot copies `memcpy(0x142DE9360, 0x142DFFF60, 0x16C00)`). That block holds all eight areas `0x1402CEE20 (n)` returns (0 `0x142DE9360` .. 7 `0x142DFFB88`). Checked (decomp).
- **Where scripts keep their variables** (load `0x14019FBF0`, store `0x1401A0090`, bit flags `0x14019FFA0` / `0x1401A0960`): variables under `0xC80` and from `0xD40`, and every bit flag, are in that save block; `0x900..0xAFF` are the running script's own (`0x142380524`, cleared each time a script file starts, `0x1401A38A0`); `0xC80..0xD3F` are RAM only (`0x1423BBBF0`, three blocks of 64), cleared by `0x1401A42C0`, which KH1's reset (`0x140170630`) runs through `0x1401A70E0`. Checked (decomp). So no script flag survives a reset plus load on KH1's side.
- **KH1's other reset.** The game object's vtable (`0x1403B1AE8`) has a fuller one at `+0x20`, `0x140171410`: the same steps as `0x140170630` after closing the live tasks (`0x1400E3920`) and a few renderer and menu closes (`0x1402E1DF0`, `0x140202F20`, `0x14020F2D0`, a fade). The soft reset runs the shorter one (mode 1). Checked (decomp). Whether the tasks left running by the shorter one matter is not known (Assumption: they don't hold story state).
- **The mod's own state** for the skip rows is per scene and is reset when the room's script runs again (`ingameskip`'s entry 0), and the Cutscene Skip row's Seen list isn't used with All (the tester's setting in his log). Checked (code + trace).
- **Not pinned.** The game folder also holds `autosave_1..9.dat`; if the save loaded was one written after those scenes, the state is the save's own. Needed: the Cutscenes and Menus logs, loading the same save once from a fresh start of the game and once after a soft reset, entering Merlin's house each time.
