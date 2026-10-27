# Soft Reset (t707)

The user: "in qol you can quick reset the game with l1 l2 select and start pushed together", then "if its in the game then use the same button combination" and, once the combo did nothing in the PC build, "the combo does not work in the pc version so restore it". The combo is the PS2's: L1 + L2 + R1 + R2 + Start + Select.

**Row.** Config menu, QoL, Soft Reset (Off | On, ini `soft_reset`, off by default). Off: nothing runs but one flag check in the mod's pad read.

**What KH1 has.** Its pad code still tests for the combo (mask `0x0F09` on its pad table, Steam `0x14012B3A7` and `0x14012BE03`) and raises a reset flag for the PS2 platform layer; in this build that flag does nothing (Checked (decomp + exe); the user saw the combo do nothing).

**What the mod does.** With the row on, the combo read from the mod's own pad read (`probe::pad`, the records merged, Select `0x0001`, L2 `0x0100`, R2 `0x0200`, L1 `0x0400`, R1 `0x0800`, Start `0x0008` as in KH1's mask) held for one second sets KH1's game mode back to its start-up step. KH1's mode step (`0x140170870`, a switch on the mode at `0x14233FBC0`) in mode 1 counts a timer (`0x1422EC0A8`) down and, below zero, runs KH1's own full reset (`0x140170630`: tasks, scene, sound and fade state, then mode 2, which loads the title screen as at start-up). The mod writes the mode, the mode-change countdown (`0x1422EC0AC`) and that timer, and KH1 steps the reset itself: no hook, nothing per frame. Checked (decomp) for the flow, (exe) for the first bytes of both functions and the mode's address, which are checked once at start and the row is unused on a mismatch or the Epic build. Nothing is reset when the mode is a start-up or title one (a game running is modes 4 to 0x12).

**Not checked (Assumption).** KH1's reset was written for start-up, so whatever a running world leaves behind is assumed to be cleared by it; not run in game. Unsaved progress is lost, as with a console reset; Continue loads the save. The buttons are not hidden from KH1 during the one second (Start may open the pause menu).

**Debug.** Lines are written under the Menus & Config log.
