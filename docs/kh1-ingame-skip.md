# In-game Scene Skip (t712)

The QoL row In-game Scene Skip (Off | On, ini `ingame_skip`, off by default) skips the in-game scenes: the low-poly dialog scenes you confirm line by line, which KH1's pause menu doesn't let you skip. It uses KH1's own skip, the one the pause menu's Skip Cutscene runs, so KH1 itself runs the rest of the scene and anything the scene gives (items, abilities, story flags) is still given, in the scene's own order.

## What the user asked

"you can make it a seperate config option in qol, what I want you to do is have the cutscene skip. how it should work is it skips that in game cutscene fully, then if any flags give you an item the game gives you the item after the fact." It is a separate row from Cutscene Skip (`docs/kh1-cutscene-skip.md`), which keeps the high-poly scenes.

## What KH1 does (Steam)

- **An event scene** is a script entry other than 0 (`0x142382524` holds the entry in use; 0 is the room's own script). Its threads are the contexts at `0x1423902D0`, `0x360` bytes each, their count at `0x14237F4E8`. Each has its own code (`+0x80`) and number of sub-scripts (`+0x88`); instructions are 4 bytes with the class in the last byte, class 5 ends a sub-script and class `0x18` calls the event function its operand names (table at `0x1404FA870`, 764 functions). Checked (decomp + exe).
- **Event mode.** Event function 513 sets bit `0x400` of the second scene flag word (`0x142382594`) at a scene's start and 514 clears it. In the user's t709 and t710 logs the low-poly scene in Traverse Town (room `3 0 4`, entry 1) had `0x400` from its second frame to its end, with no bit 0 and no `0x800`; talks had `0x800` and no entry; Cid's shop was entry 10 with `0xc00`. Checked (decomp + trace).
- **The skip.** Bit 0 of that word marks a skippable scene (set by event functions 84, 139, 476, 489 and 623, the high-poly scenes; 27 and 317 set it only under a condition, `0x1401A19B0`). In the t709 / t710 logs 139 ran in all ten high-poly scenes, and 27 ran in the low-poly Traverse Town scene, which never had bit 0 (Checked (trace)). The script step (`0x1401A2320`) sees bit 0 with the pause menu's flag byte (`0x14232DF89`), clears the byte and runs KH1's skip (`0x1401A22B0`): skip mode (bit 0 of `0x1423AB2D0`), the message windows cleared, a fade out. Checked (decomp + exe).
- **Skip mode.** The frame step becomes 128 (`0x14018D1F0`), so timed waits, moves and motions end at once. The message functions open nothing in skip mode (event functions 0, 1 and 177; the item message 605), and the waits on messages (33, 107) pass. The rest runs as usual: the item and ability give (583) and the story flag functions are not skip-aware. Checked (decomp). In the t709 log the low-poly scene shows the item message 605 followed by 583, so the give is its own function (Checked (trace)).
- **The end.** When every thread of the entry has ended KH1 goes back to entry 0 and ends skip mode (`0x1401A2130`: skip mode off, a fade in over a second). Checked (decomp).
- **Choices.** A choice in a message is event function 88 (`0x14018A9C0` attaches it to the window and writes the answer to a script variable). In skip mode no window opens, so the answer would stay at its old value. Checked (decomp).

## What the mod does (`win/ingameskip.rs`)

With the row on, the mod's pad read (already running) looks at the entry in use and the second flag word. The first time an entry other than 0 is in event mode (`0x400`), it decides once:

- bit 0 already on: a high-poly cutscene, the Cutscene Skip row's;
- `0x800` on: a talk started (the player may have started this), it plays;
- otherwise it reads the code of every thread of the entry once (up to 64 threads, 8192 instructions each):
  - one of the pause skip's functions (84, 139, 476, 489, 623): the Cutscene Skip row's;
  - a choice (88) or a shop or menu window (222, 268, 278, 392, 628, 629, 673): it plays;
  - anything else: the mod sets bit 0 and the flag byte, and KH1's next script step skips the scene.

When the entry is back to 0, or KH1's skip mode has ended, the mod clears the bit 0 it set (and a flag byte KH1 didn't use), so the pause menu can't offer a skip later. The Cutscene Skip row ignores a bit 0 the mod set. Off: one atomic load in the pad read. Before use the skip's first bytes, the script step's test of bit 0, its read and clear of the flag byte, and the table entries of every event function named above are checked (Checked (exe)); on a mismatch the row does nothing. Steam only.

## Not checked in game (Assumption)

- That every high-poly scene has one of the pause skip's functions in its own entry (the logs only listed each entry's first thread). If one doesn't, this row would skip it.
- A scene started from a skipped one (chained, the same skip mode) is skipped too, as after the pause menu's skip.
- After talking to someone in a room, a scene in that same room plays: KH1 keeps the talk bit until the room reloads.
- The item popup isn't shown (KH1 shows nothing in skip mode); the item is in the inventory.
- How it looks: KH1's skip fades out, runs the scene in a few frames and fades in over a second.

Test with the Cutscenes log on (`in-game skip` lines): each scene says whether it was skipped and why not.
