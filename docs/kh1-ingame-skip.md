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

## Trinities

The user: "currently works in traverse town, wasn't able to test it in other worlds, but can you also make it skip trinity animations?". They are on the same row (In-game Scene Skip): a Trinity is an event entry like the in-game scenes, so KH1's own skip runs it.

### How a Trinity runs in KH1 (Steam)

- **The mark is a context of the room's own script** (entry 0; its contexts at `0x142382AD0`, `0x360` each, count `0x14237F4E4`, the ones the script step `0x1401A2320` and `0x1401A4D00` use while `0x142382524` is 0). Checked (decomp).
- **It offers the command through event function 454** (`0x1401AF530`, not a function in Ghidra's listing; read from the exe): it puts the context's actor and the two values pushed before it into the room's object table at `0x142382938` (8 slots of `0x20`: actor `+0`, command `+0xC`, flag `+0x14`). The Triangle-box scan's script path `0x1401A4D00` offers that command in slot 3 (`0x140293000`), and when it is used wakes the context's event 9 with the command in `0x142382A38`, which event function 455 (`0x1401AF640`) pushes back to the script. Checked (exe + decomp).
- **The Trinity command is 0x53.** In the user's own room scripts (read locally from his install; nothing of them is in the repo) `push 0x53, push 1, call 454` appears in 17 rooms of six worlds (Traverse Town, Wonderland, Deep Jungle, Agrabah, Olympus Coliseum, Monstro), and every such context, when the command is used, starts (after holding the party, event function 593, and a few frames in the Traverse Town ones read in full) an event entry with event function 126 (`push N, call 126`; `0x1401A9D90` → `0x1401A38A0`), N = 50 in every room read, with the mark's number in a script variable. Checked (files). That KH1 shows it as "Trinity" is an Assumption (the message text wasn't decoded); the structure below is why.
- **The started entry is the Trinity itself:** its first three threads bind to party members 0, 1 and 2 (Sora, Donald, Goofy) and place them 0, 120 and 240 degrees round the mark; the next threads are the marks, the last the controller. Checked (files).
- **Why t712 didn't skip them:** some story versions of a room run the Trinity entry with no event mode (no event function 513 in it: Traverse Town's First and Second District in two of their versions), and t712 waited for event mode before judging; and a talk earlier in the room leaves `0x800` on until the room reloads, which t712 read as "the player started it". Checked (files) for the first; the second is t712's own rule. One Olympus version (Coliseum, `he02`) has the pause menu's skip (event function 139) in its Trinity entry: that one stays the Cutscene Skip row's.

- **A mark can also live in an event entry.** In Olympus's Coliseum after the barrel game (`he02`, the version whose entry 1 is started by talking to Phil), entry 1's context 16 is a Trinity mark that starts entry 50 itself. Event function 126 run from an event entry puts the new entry's contexts over the old ones (`0x1423902D0`, `0x1401A38A0` with an entry other than 0), so that mark is gone by the time its Trinity runs. Checked (files + decomp).

### What the mod does

When an event entry starts, `ingameskip` reads the room script's contexts once (up to 64, 8192 instructions each) for one that both offers command 0x53 (`push 0x53, push 1, call 454`) and starts this entry (`push entry, call 126`), and reads the new entry's own contexts for marks the same way, noting the entries they start (up to 4) for the next entry (`crate::kh1script::trinity_call`, unit-tested). If one does, or the entry before noted it, the entry is judged at once, without waiting for event mode and without the talk rule; the rest is t712's: the pause skip's functions make it the Cutscene Skip row's, a choice, shop or menu makes it play, anything else gets bit 0 and the flag byte and KH1's script step runs its own skip. Event functions 454 and 126 are checked in the event table (`0x1404FA870`) with the others before use. Off: nothing more than t712's one load.

Log (Cutscenes): `in-game skip: entry 50 is a Trinity (started by context N of the room's script, which offers command 0x53)` (or `... by a mark of the event entry before it ...`), then `... (a Trinity, n instructions read, flags ...) skipped ...`.

### Not checked in game (Assumption)

- That every Trinity type's result (the Jump's ledge, the Ladder, the Charge's broken wall, the Push, the Detect's chest, the journal's Trinity count, the mark going out) is set by the entry's own script steps, which skip mode runs; that Sora ends where the played Trinity leaves him (moves finish at once in skip mode).
- That 0x53 is only ever the Trinity command (in the rooms read, every context offering it started such an entry).
- Rooms not read (Neverland, Hollow Bastion, Halloween Town, Atlantica, End of the World, 100 Acre Wood): assumed the same.

## Talks that lead into a scene (the Phil report)

The tester: "talking to phil after the barrel mini game still plays the in game cutscene where he gives you thunder". His log (`testers/auto-cutscene-skipper/kh1-kh2.log`):

- f43649: Triangle on Phil (world `0xb` room 1, the `he02` version with 6 entries) starts entry 1 with the talk bit `0x800` already on; f43651 event mode comes on and the row says "plays (a talk's bit is set ...)". Checked (trace).
- Entry 1 of that version (17 threads) is Phil's whole after-barrel scene: Phil's thread (context 6) holds the talk lines and, in sub-script 14, a choice (event function 88 with its answer in variable `0x910`, then 89); context 16 is the Coliseum's Trinity mark (above). Checked (files). So even without the talk rule the row would have kept it for the choice.
- f34971 and f35566 (entries 1 and 3 of the first `he02` version, also talks to Phil) played on the talk rule too, but the Cutscene Skip row skipped them a moment later (they turn on bit 0, event function 139). Checked (trace).
- f32384 (Traverse Town, world 3 room `0x13`, entry 50 with the talk bit on) was a Trinity: the Trinity change above skips it now. Checked (trace + files).

**No clean line in the data.** A plain talk in Traverse Town runs inside the room script (no entry starts; t709); Phil's talk starts its own event entry. That part is clean. But the entry Phil's talk starts mixes the reward lines with a choice and a Trinity mark, so whether a choice is reached on this visit depends on which sub-scripts run, which the code alone doesn't say (Assumption until the VM's wake-ups are traced). Skipping it would answer that choice with the variable's old value. Nothing is built for it; the tester picks (thread reply).

Test with the Cutscenes log on (`in-game skip` lines): each scene says whether it was skipped and why not.
