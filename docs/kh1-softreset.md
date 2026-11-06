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

## The way back to the title (the owner's endless loading screen; titlewatch)

The owner: "when going to the title screen from a save menu, the game gets stuck on a loading
screen indefinitely", and after t1079 "going back to the title screen still gets you stuck in a
endless loading screen". Traced in the KH1 Steam decomp (addresses Steam; not read from the exe in
that session, so no hook uses them; the watch only reads them).

- **The request.** The PC's confirm box `0x1400D94C0 (kind)` (a window message `0x8001` to KH1's
  window, the game thread sleeping 10 ms at a time until it is answered) sets the game object's
  (`[0x1421AAF18]`) `+0x1290` to 0x100 for kinds 5 and 7 (5 only when confirmed); its callers are
  `0x1400D9FE0` / `0x1400DA020` (callbacks), the save menu `0x1402E1780` (kinds 2..4) and
  `0x1402F9310`. Kind 1 is the quit (it starts `WaitTitleProject.exe`, `0x1400D9800`). Checked
  (decomp). Which box the save menu's "return to title" is: Assumption (not traced to the text).
- **The wait.** The field frame `0x140170D50` (and its copy `0x14016FFA0`, in the loop `0x14017F590`
  that the frame runs through `0x14017F150`; that this is the pause menu's loop: Assumption) sees `+0x1290 >= 0`, sets `0x1422EC090` = 2, and then each frame waits for, in
  order: a movie to end (`0x1422EC094` and `0x1423408D0`), a save being written (`0x142E205BC`, the
  save menu's step 0x47 sets it), and KH1's two job slots to empty (`0x14234AFA8`, 0x18 bytes
  each: `0x14028C390`'s file reads (`0x14028C200`'s queue) and `0x1401FA880`, run on KH1's job
  thread `0x14018DA50`, each clearing its own slot when done). `0x1400D31F0` is `return 0`. While
  it waits the frame goes on (KH1 draws and presents). Checked (decomp).
- **The teardown, all in one frame.** `0x1400D2C70 (1)` -> `0x1400DBAC0`: first every pending
  music stream delete (`0x1404D65F8`, count `0x1404D6608`) is retried in a loop until the engine
  lets it go; then every bank entry of `0x1404D65D8` is destroyed, each destructor
  (`0x1400DB470` / `0x1400DB7A0`) stopping the bank's sounds and sleeping 10 ms at a time until
  `0x14004D870 (handle, 0)` says the engine unloaded it. Then `0x14017EF70`, `0x140266530`,
  `0x1401E1490 (0)`, `0x14018C790`, the volume reset, `0x14018D1F0`, `0x14018BC90 (0)`;
  `0x14233FBEC` = 0 and the frame returns 2 to the game's loop. Checked (decomp). (tm32 guessed
  the loop then runs KH1's reset and the title: wrong, see "The way back is a restart" below.)
- **The way back is a restart (tm35, the owner's tm34 log).** The owner's stuck return (from a
  save point) was state **1**, not 2: "quit/return 1 (request -1)". State 1 comes from
  `0x1422EC0B4` (set by `0x140170220`, called from the save menu's step `0x1402DFFE0` and from
  `0x1401B5410`; which one is the save point's "return to title": Assumption) once the sound
  `0x1400DEEE0 (2, 2, 0)` has stopped. After the same waits and teardown, state 1 calls
  `0x1401B53D0` and `0x1400D9800 (1)`, which sets the game object's `+0x1298` to 1 (what
  `Axa.AppInterface.IsReBoot` `0x140284250` reads), starts `WaitTitleProject.exe` from KH1's
  folder (`ShellExecuteA`), sleeps 1 s and returns 1; the frame returns 2. The game thread's loop
  (`0x140103460`, the thread `0x1401095A0` started by `0x1400FE770`'s `_beginthreadex`) calls the
  game object's slot `+0x18` (`0x140171410`) only when the frame returns 1; on 2 it calls the
  shutdown `0x140170CA0` (slot `+0x28`; the vtable's slots read as starting at `0x1403B1AF0`),
  which returns 1, and the loop ends. The thread then jumps through `[0x140527F00]` (its target
  isn't in the decomp: Assumption, the CRT's `exit` or the host's; the quit box kind 1 calls the
  same) and `_endthreadex`. So on Steam KH1's way back to the title ends this game and
  `WaitTitleProject.exe` brings up a new one; nothing in this process ever runs KH1's reset or
  title. Checked (decomp), but for the jump's target. Our hooks: none sits on a function of that
  path (the teardown, `0x1400D9800`, the shutdown and its callees; Checked (code: no hook address,
  Steam or Epic-mapped, matches)); the owner's log shows the bank release finished (bank list 0).
  The watch's "game thread at 0x0" at 2.8 s is what a read of an ended thread gives
  (Assumption: Windows refuses to suspend or read a thread that has ended); so the game thread most likely wasn't stuck at all but
  had ended, and the hang is after it: the old process not ending, or the new one not starting.
  Assumption until the next log: tm35's lines say which (the game thread's end and the process's
  end reaching our `DllMain`, and every thread's place, see below).
- **That release runs only here.** `0x1400D2C70 (1)` has exactly four callers, `0x14016FFA0`,
  `0x140170230`, `0x140170300`, `0x140170D50`, and each runs it only on this quit / return path
  (after the same waits). Room and world changes never release the whole bank list. Checked
  (decomp). So our ~104 KH2 banks (`kh1bank`, in that list since the first room) are released by
  KH1 for the first time on the way back to the title, each through that wait-until-unloaded loop
  on the game thread. Whether KH1's engine lets ours go: not known (its unload is behind two
  engine vtables, `0x142F124E8 + 0x180` and `0x142F124E0 + 0x38`, not resolved without the exe).
  If it didn't, the game thread would sleep in that loop for ever and the screen would stay on the
  last frame drawn.
- **t1079 couldn't have changed it.** KH1's game mode stays the game's (5 in the field: Assumption)
  through the whole wait and the teardown (nothing writes `0x14233FBC0` on this path until the reset), so `kh1bank`'s "a game"
  test is true there, and the release is one call inside one frame, with no frame end of ours in
  between. Checked (decomp, code).
- **Other ways.** Game over: "Continue" is mode 0xC and "Load" mode 0x11 (`0x14018BA00` /
  `0x14018BA60`); neither goes to the title or releases the bank list. Checked (decomp). The Soft
  Reset row (mode 1 -> `0x140170630`) clears the job slots (`0x14018DC60`) without waiting and
  doesn't release the bank list either. Checked (decomp). So only the return-to-title / quit path
  has these waits.
- **KH1's Loading line** (sysmsg 0xF at 320, 404) is drawn by the 2D work `0x14017B190` while
  `0x1423404EC` (a frame copy of the script runner's skip bit `0x1423AB2D0` & 1, `0x1400D2830`) or
  `0x1423404F4` (a room data load, `0x14017D580`) is 1; the reset clears both (`0x1400D2A20`,
  `0x140171410`). Checked (decomp).
- **The title watch (`win/titlewatch.rs`, debug tools, Loading & Cache).** Once a second, while
  KH1's return or quit is pending, while KH1's Loading line has been up 5 s, or while KH1's frame
  end hasn't run for 2 s: "title watch (N s): why: mode, quit/return state and request, teardown
  done or not, what it still waits on (the movie, the save, each job slot with the file KH1 is
  reading), the Loading line's flags, the bank list's count and the stream deletes pending", and,
  when the frame has stopped, "game thread at X <- Y <- ..." (its instruction and KH1's return
  addresses, Steam). A stop in `0x1400DB470` / `0x1400DB7A0` / `0x1400DBAC0` settles the bank
  release; a job slot that never empties, a save flag that stays, or a stop elsewhere point at
  something else.
