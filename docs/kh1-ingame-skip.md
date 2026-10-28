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

### After t717: Phil's talk has a real question

- **KH1's choice in skip mode.** Event function 88 (`0x1401B8EC0`) attaches a choice to a message window already open (one of the four at `0x14237F4F0`); in skip mode the message functions open no window (`0x1401B93F0` and `0x1401B8DD0` test bit 0 of `0x1423AB2D0`), so 88 finds none and does nothing, and the answer variable keeps its old value. Checked (decomp). KH1 has no way to leave skip mode in the middle of a scene; it ends when the scene's threads end (`0x1401A2130`).
- **Phil's after-barrel entry 1** (`he02`, the six-entry version): its controller (context 0, sub-script 1) wakes context 1's sub-script 11, which wakes context 6's sub-scripts 11, 12, 13 and 14 in turn and waits for each, with no branch. Sub-script 14 opens message `0x4A`, attaches the choice (88, answer in variable `0x910`, then 89) and branches on the answer (one side two more lines, the other side the barrel game's start). So the question is always reached in that scene. Checked (files). In the tester's log (f43651..f43955) the D-pad Down was pressed twice during it and the next room was the barrel game's (world `0xb` room 2). Checked (trace).
- **The Thunder scene** isn't in that log: after the three barrel tries the lobby's entry 2 (`he02` version 3, no question, no item give 583) was skipped each time, and he then left for Wonderland. The other Phil talks in the log (f34971, f35566) are the Cutscene Skip row's (fn 139). Nothing is built: the only Phil talk that played has a question on its one path.

Test with the Cutscenes log on (`in-game skip` lines): each scene says whether it was skipped and why not.

### The Thunder scene (the tester's third Olympus log)

The tester: "sent a log showing the exact moment, its when you get thunder. if you do talk to phil again then he has a question for you but not during this part." He chose (a) for talk-started scenes (they play).

- **Where it is.** After the barrel game the Coliseum gates reload (world `0xb` room 1, the third value 2 in the log's scene triple: the `he02` script whose entry 1 begins `push 1, push 0x32, push 0x28, call 427`; read in place from his install). The room's own script, entry 0, context 0, sub-script 1, runs by itself at the room's load: it starts entry 1 (`push 1, call 126`), Phil's high-poly scene, which has the pause skip (event function 139), and when that has ended it goes on, still in event mode, with the Thunder part: event function 458 with spell 2 and its level (`0x1401AF730` → `0x14028F560`: writes the spell's level to save `+0x492 + spell` and loads its colour change; 459 waits for it), the "learned" message picked by the level (message ids `0x146`..`0x14C`), two more lines (`0x14E` / `0x14D`), and event function 514 (event mode off). No choice (88), shop or menu in it. Checked (files + decomp; fn 458's table entry read from the exe).
- **In the log.** f22228 entry 1 starts (no talk bit); f22239 the Cutscene Skip row skips it; f22266 KH1's skip ends (entry back to 0, `0x401` → `0x400`) and event mode stays on with entry 0: the Thunder lines, confirmed with Circle until f22551 (event mode off). The second run (f29264..f29594) is the same and the log ends there. f22591 the Triangle on Phil (talk bit `0x800`, no entry started) is the talk with his question (context 7 of entry 0 holds event function 88). Checked (trace).
- **So** the Thunder scene isn't talk-started: (a) doesn't keep it. But it isn't an event entry either. It runs in entry 0, which In-game Scene Skip never judges, and KH1's own skip can't run there: KH1 ends its skip mode only when every thread of the entry has ended and it goes back to entry 0 (`0x1401A2130`), which never happens for the room's own script. Checked (decomp, see "The end" above). Nothing is built.
- **What a skip would take (not built, asked).** KH1's skip mode (bit 0 of `0x1423AB2D0`, the message functions opening nothing, the 128 frame step) started when an entry ends with event mode still on and the script back at entry 0, and ended by the mod when event mode goes off (event function 514), with KH1's own end steps (skip mode off, the fade in). That gives KH1's skip a second way to end that KH1 itself doesn't have. Assumption: other rooms continue the same way after a scene (only this room read).

## t764: where the mod's skip ends (the tournament black screen, and the room script's follow-ups)

The tester, choosing (b) for the follow-ups: "(b) we're going to merge this with the work done on quick chests once its finished so it notifies you after the cutscene ends. also, updated the log and a trace, you get stuck to a black screen during Olympus coliseums tournament".

### Correction

Earlier notes here said KH1 has no way to leave skip mode in the middle of a scene. That was wrong. Besides the end at entry 0 (`0x1401A2130` from the script step), KH1's skippable sections have their own end: event functions 85, 140, 490 and 624 (`0x1401A8BD0`, `0x1401AA120`, `0x1401B05F0`, `0x1401A8C00`), the pairs of the skip-on functions 84, 139, 489 and 623, turn bit 0 off and call `0x1401A2130` (skip mode off, the message windows closed, a fade in over a second), returning 6 when that ended a skip. Checked (decomp + exe: the event table at `0x1404FA870`).

### The black screen (Checked (log + trace + files))

- The tournament is world `0xb` room 5 (`he06`, the script version whose entry 0 has 470 instructions in its first thread; read in place from the tester's install, nothing copied here). Each round is an event entry (1, 2, 3, ...): a short lead-in in event mode (no messages), event function 514 (event mode off), 512 (Sora given back), the fight (enemy spawns 601, waits 457), then `push 20, call 126` from the entry itself: entry 20, the between-round scene (pause skip 139), which starts the next round the same way.
- The log: f64554 entry 1 starts, f64555 In-game Scene Skip skips it (no pause skip, no choice). f64556 KH1's skip mode is on (runner `0x3`) and the frame scale goes to 4. From f64563 the fight runs (event mode off, "Sora can act: true", hits) with skip mode still on: the runner stays odd (`0x19`, `0x3`) to the end of the log, the frame scale is never 1 again, and the trace (f64603..f64663) has 4 steps every frame. Entries 20, 2, 20, 3 follow without the script ever returning to entry 0, so neither of KH1's ends ran: no fade in (black), the fight fast-forwarded.
- The Cutscene Skip row didn't act on entry 20 either: the mod's own bit 0 was still set, which that row leaves alone (`ingameskip::forced`).

### What changed

- **The mod's skip of an event entry ends at event function 512** (Sora given back, `0x1401A7660`). Its table entry points at a wrapper that runs KH1's 512 and then, if the mod's skip is running, does what 140 does: bit 0 off, the pause flag byte cleared, `0x1401A2130`, 6 instead of 2. Read across 110 script files in 38 rooms of nine worlds (Traverse Town, Olympus Coliseum, Wonderland, Agrabah, Deep Jungle, Halloween Town, Monstro, Atlantica and Destiny Islands; all of Traverse Town and Olympus, a few rooms of the others): in the entries the row skips, 512 is a scene's last step (followed by at most 514, 101, 27 or 150) except the Coliseum's fight entries (`he03`, `he06`, `he07`: the barrel game, the tournaments) and a Trinity mark's thread in Phil's entry, where play follows it. Checked (files).
- **The room script's follow-up** (the tester's (b)). When an event entry has ended and the script is back at entry 0 with event mode still on and no talk bit, the mod takes the room-script context that started that entry (noted when the entry started: its next instruction `+0x190` right after `push n, call 126`; the room script doesn't step while an entry runs) and reads on from there to its first 514 or 512, with every sub-script it wakes on the way (`21/c 22/s`, read whole). No choice (88), shop or menu on the way and an end found: bit 0 and the flag byte, so KH1's script step starts its own skip there too (`0x1401A22B0` doesn't care which entry is in use), and the wrapper on event function 514 (`0x1401A7750`) or 512 ends it the same way. Otherwise it plays, with a log line saying why. `crate::kh1script::read_follow_up` (unit-tested).
- **The Thunder scene**: in the Coliseum gates (`he02`, the version with Phil's high-poly entry 1), context 0 goes on after `push 1, call 126`: the spell (458 with spell 2, 459), the "learned" lines, a wake of context 7's sub-script 13 (two more lines, no question) and 514. No choice, an end found: it is skipped. Phil's question (context 7's sub-script 12) isn't woken from there. Checked (files).
- Across the rooms read, 34 room-script places start an entry and then show messages; 7 of them have a choice on the way (two in Traverse Town, three of Phil's talks, two more in Traverse Town's `tw20`) and play; the rest either reach 514 / 512 (skipped) or not (they play: "no 514 or 512 to end a skip at"). Checked (files); which of them actually run in event mode is only known at run time.

Not native in one way, said plainly: KH1 ends a skip only at its section ends and at entry 0; the mod adds that end at 512 / 514 for the scenes it skips (through KH1's event table, with KH1's own end routine). Everything the skip does is KH1's.

### Not checked in game (Assumption)

- The tournament: that ending the skip at 512 gives a normal fight (fade in, normal speed) and that each round's lead-in skip looks right. The between-round scenes are the Cutscene Skip row's.
- A follow-up starts one frame after KH1's fade in at the end of the entry, so a short flash may show between the two skips.
- Messages a follow-up skips show no "Obtained" box (KH1 shows nothing in skip mode); the tester's plan is to merge the Quick Chests notice later.
- Rooms not read (the rest of Wonderland, Agrabah, Deep Jungle, Monstro, Halloween Town and Atlantica, and all of Hollow Bastion, Neverland, End of the World and the 100 Acre Wood): assumed the same.

## t765: the part of a scene after its skippable section

The owner, in chat with the t764 log: "there's also in game cutscenes in alice's world that aren't skipping".

- **What played.** Wonderland room 4 (`aw05`, version `0xd`), entry 1. Context 0 turns the pause skip on (event function 139), runs the high-poly part, ends the section (140), and then wakes 18 sub-scripts in other contexts with lines confirmed one by one (message functions 0 / 1 / 177, waits 33 / 107), no choice, then 512 and 514. In the log the Cutscene Skip row skipped the section (f30680) and the lines played from f30723 ("Sora can act: false", waits 107 and 33 to past f31086). In-game Scene Skip had judged the entry the Cutscene Skip row's (it has 139) and didn't look at it again. Checked (log + files, read in place from the owner's install).
- **What changed.** The four section ends (event functions 85, 140, 490, 624) go through the event table to `ingameskip::section_end`: KH1's own runs first; then, with event mode still on, no talk bit and the mod not already skipping, the rest of the scene is read from the instruction after the call, the same way as t764's room-script follow-up (to the first 514 / 512, every woken sub-script whole). No choice, shop or menu and an end found: bit 0 and the flag byte, KH1's skip, ended at 514 / 512. This doesn't depend on whether the section itself was skipped.
- **Across the scripts read** (110 files): 112 section ends; after 84 of them the scene reaches 514 / 512 with no choice (skipped), after 28 it doesn't (they play, logged "no 514 or 512 to end a skip at"); none has a choice. Checked (files).
- **A tail run in the same step.** When the scene ends right after the section (514 / 512 a few instructions later, run in the same script step), the skip hasn't started yet: `end_skip` then takes the bit and the flag byte back so no skip starts in play.
- **Room 3's "plays (event function 88)" lines** (`aw04`): that room keeps entry 1 running the whole time (its talk handlers have a choice); event mode comes on there when Sora goes through to room 4. Not a scene: nothing was missed. Checked (log + files).

Not checked in game (Assumption): the look (KH1's fade in at the section end, then the fade out of the new skip a frame later).

## t766: longer follow-ups, and never into a pause-skippable section

The owner (chat, t765 log): "same issue on the same scene", and what the two rows are for: the Cutscene Skip row owns the scenes KH1's pause menu can skip (high-poly or not); In-game Scene Skip is for the scenes read line by line with the confirm button (like Phil's Thunder), whatever models they use.

- **Why t765 didn't skip it.** The log (f412): "entry 1 goes on in event mode after its skippable section (event function 140) (context 0), plays (its code couldn't be read)". The follow-up reader gave up after 16 woken sub-scripts; this scene (`aw05` version `0xd`, entry 1) wakes 18. The limit is now 128 (the most in the scripts read is 40). Checked (log + files).
- **Never into the Cutscene Skip row's part.** A pause-skip function (84, 139, 476, 489, 623) ahead in a follow-up now makes it play, like a choice. None of the 112 section ends read has one. So with the Cutscene Skip row off, a pause-skippable section always plays, and only the line-by-line part after it is skipped. Checked (files).

## t767: the Cutscene Skip row was dropping the flag byte

The owner (chat, t766 log): "the cutscene still did not skip". The log (f423): the part after the section was read (514 instructions, no choice) and bit 0 and the flag byte were set, but KH1's skip never started: the second flag word stayed `0x401` and the runner `0x2` until 512 at f1563. Cause: the Cutscene Skip row had fired on the section just before (f397); at its next pad read bit 0 was In-game Scene Skip's, which it takes as its scene being over, and it dropped what it took for its own unused flag byte. Fix: it leaves the byte alone while In-game Scene Skip has set bit 0; and In-game Scene Skip puts bit 0 and the byte back if its skip hasn't started and the byte is gone. Checked (log + code).

## t784: the scene reads no longer freeze the game

The owner: loading a save froze the game for a moment, only with the mod on. The t783 load profiler showed it: on the first frame after KH1's opening script hands Sora over, 97 to 99% of a 337 ms / 829 ms frame was this file's scene reading, inside Windows. Each 4-byte script word was checked readable twice (`crash::readable`, one `VirtualQuery` each), and the first scene has every thread of the room script and the entry scanned for Trinity marks and the scene's own calls (up to 64 threads of up to 8192 words). Checked (log, PDB).

- `Reach` keeps the readable region last asked about (`crash::readable_span`) and asks again only outside it: one query per region instead of two per word. `marks`, `read_scene` and `Ctxs::word` use it.
- What is read, and what is skipped or played, is unchanged.

Not checked in game (Assumption): that the freeze is gone; the next load profiler log says.

## t785: an "Obtained" box for what a skipped scene gave

The owner: "for in game cutscenes that have items in them like phil giving you thunder I want it to play a text box like our custom chest text box", then (thread m39) "1 and 2 with (a)": "Obtained" for everything, every kind of give; 3 and 4 left as they are.

- **KH1's gives in a scene** (Checked (decomp + exe)): event function 583 hands out a reward record (the u16 at `[0x142D36D58] + id * 2`: kind `& 0xF` 0 an item `>> 8`, count `>> 4 & 0xF`, through KH1's stock add; 1 an ability `>> 8` for party member `>> 4 & 0xF`); 458 sets a spell's level (`0x14028F560`: save `[0x142868BA0] + 0x492 + spell`, nothing if already that level); 444 adds munny (`[0x142E1F4D8] + 0x1C`); 570 a Gummi Block (`0x14020D3F0`, as chests). None of them shows anything itself; the scene's own message lines do, and those open nothing in skip mode.
- **What the mod does.** The four go through the event table to wrappers that always run KH1's own. While In-game Scene Skip's skip is running, each give is noted: the item's name (and "xN" if more than one), the ability's name (KH1's ability table), the spell's name at its new level, "N Munny" (what KH1 actually added), "Gummi Block". When the skip ends they go into the mod's notice box (the chests' box) as "Obtained" / the name, one after another.
- **Not done, as asked:** scenes skipped by the Cutscene Skip row get no box; a party member's ability shows only the ability.

Not checked in game (Assumption): the spell names are English (KH1's own text for them wasn't looked up); a Gummi Block from 570 is shown even if KH1 gave none (it can refuse at the most of one kind); KH1's order of gives is the scene's.

## t786: the gives in the chests' "Obtained" window

The owner (thread, after t785): the box should be the chests' own (since t776 KH1's window 7, `docs/kh1-chest.md` round 9), "(a)", go ahead.

- **The treasure script's window** (`exchange/UK_treasure.ev`, read in the owner's files; nothing of it copied here). Sub-script 0 sets window 7 up: event function 3 (7, -115, -135), 4 (7, 0, 1), 5 (7, 1), 6 (7, 0), 83 (7, 0), 80 (7, 0). For an item it sets them again, then 365 (7, text, -220, -150), 0 (7: open), 1 (7, text: the line), 107 (7: waits while it shows), 2 (7: close). Its "Obtained" texts (5..8) are `08`, `0A 00 00 00`, `07 0C 00`, "Obtained ", `0C 04` (colour), an icon (0x82 / 0x89 / 0x83 / 0x84), `0E 01` (the item 364 set), `0C FF`, "." and `06 52 00` (the window closes itself after 0x52 frames), ended by 0; Sora's ability text (9) has icon 0x86. Checked (files).
- **How KH1 starts it** (Checked (decomp)): event 5 (`0x1401A1A10`) calls `0x1401A43A0 (file, entry)`, which keeps the room script's file and text table aside (`0x142382538..`, `0x1423B8BE0`), puts the file's texts in the text table (`0x14237C4E0`), loads the entry's contexts into the machine's list (`0x1423BF0B0`, count `0x1423AB2DC`), runs each context's sub-script 0 at once and adds a task (`0x1401A2CF0`) that steps the rest; when every context has ended the task puts the room script's file and texts back. The start clears the runner word's low four bits.
- **What the mod does.** What KH1 gave during the mod's skip (t785) is kept with an icon (items 0x82, abilities 0x86, the rest none). When the skip has ended, at the next pad read with the runner word's low bits clear and the machine free (the script in use isn't the machine's file), `kh1script::obtained_script` makes a script file of exactly those window calls with one text per give (the name written out in KH1's codes in place of the item code) and `0x1401A43A0 (file, 0)` starts it. KH1 shows the lines one after another in window 7 and ends it. No hold on Sora. If `0x1401A43A0`'s first bytes don't match, the level-up box (`menu::queue_prompt`) is used instead.

Not checked in game (Assumption): the look of a line with the name written out (the treasure's take it from the item code), the lines with no icon, and a talk started while the window shows (its texts would be ours until the window ends, as during a chest's).

## t787: the gives read from Sora's save data

The owner (t786 log): "the item text box doesn't appear after it skipped in an game cutscene where you get blizzard". The log: Wonderland room 1 (`aw02`, version `0xd`), entry 3 skipped from f1653 to f1673 (ended at 512); event function 458 ran in that second (its script gives Blizzard with `push 1, push var 0x18, call 458` after 513), but the 458 wrapper noted nothing. Why wasn't found (its install result was logged under Menus, which was off; it is under Cutscenes now, and the wrapper logs each call). Instead of depending on the wrappers, the mod now keeps Sora's spell levels (`[0x142868BA0] + 0x492`, 7 bytes), his ability list (`+ 0x44`, 48 bytes) and his munny (`[0x142E1F4D8] + 0x1C`) when its skip starts and compares them when it ends. Each spell whose level went up, each ability new in the list and the munny gained become an "Obtained" line. Items and Gummi Blocks still come from their functions' wrappers (583, 570). Checked (decomp: the three are where KH1's spell, ability and munny functions write).

Not checked in game (Assumption): only Sora's abilities are compared (Donald's and Goofy's lists aren't).

## t788: a spell is shown even when its level doesn't change

**Correction to t787.** The t787 log (f753): "event function 458 during the skip: spell 1, level 1 (was Some(1))". The 458 wrapper was installed and called in t786 too; the save already had Blizzard at level 1 (the scene replayed on that save), and t785's rule noted a spell only when its level changed, as did t787's save comparison. Now 458 notes the spell whenever it runs during the mod's skip (the scene shows its "learned" lines either way). t787's save comparison is removed: abilities (583 kind 1) and munny (444, what KH1 added) come from their wrappers again, so a replayed scene shows them too. Checked (log).

## t789: the window's free check

The t788 log: f719 "KH1 gave "Blizzard" during the skip", f725 the skip ended at 512, the runner word 0 from f726, and no "window(s) started" line. t786's check for a free machine compared the script in use (`0x142382530`) with the machine's file word (`0x1423BBBE0`), but the room script's start (`0x1401A38A0`) writes both, so they were always equal and the window waited forever. Now the machine counts as free when none of its contexts (`0x1423BF0B0`, count `0x1423AB2DC`) is still running (bit 3 of `+0x33C`, set by the class 5 handler `0x14019FAB0` when a context's last sub-script ends; the machine's task ends when all have it). Checked (log + decomp).

## t790: the give's sound

The owner: "its working, can you add the sound that plays when you get an item from those cutscenes/chests?". KH1's scripts play it themselves with event function 353 (`push id, push 0, call 353`; `0x1401C8F40` calls `0x1401787D0`, but in skip mode only sound 0x3EB0 is let through, so a skipped scene's sound is lost). Across the scripts read, the sound next to a give is 0x1F: next to the item message (605) 112 times, to 258 25 times, to 570 13 times, and the Blizzard scene (`aw02`) plays it after its "learned" line. The treasure script plays 0x1E (in its context 2, with the lid). Checked (files + decomp). The "Obtained" window's script now plays 0x1F through 353 right before each line opens.

Not checked in game (Assumption): that 0x1F is the same sound for every kind of give (spell, item, ability).

## t791: the free check looks at context 0 only

The t790 log: the Blizzard window at f853, two Dalmatians' chests (the treasure script) at f3400 and f4299, then Thunder noted at f11709, the skip ended at f11713 and no window, with the runner word 0 from f11715. The machine's task (`0x1401A2CF0`) ends a script when its context 0 has ended (bit 3 of `+0x33C`); the treasure script's contexts 1..3 idle in a loop and never end, so t789's "every context ended" never came true after a chest. Now only context 0 is checked. Checked (log + decomp).
