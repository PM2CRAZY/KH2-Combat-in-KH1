# Auto cutscene skipper

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. booglechan: can you add an auto cutscene skipper in qol section](#^m1)
- [2. Assistant: I haven't built the cutscene skipper yet, and I haven't touched any co](#^m2)
- [3. booglechan: go ahead, make sure the log is toggled in the debug menu so we don't b](#^m3)
- [4. Assistant: **t706** is written to your testing branch. It only adds logging for c](#^m4)
- [5. booglechan: before I can do this I need a soft reset in qol you can quick reset th](#^m5)
- [6. Assistant: The reset is possible, but I haven't written any code. KH1 has no read](#^m6)
- [7. booglechan: 1. (a)](#^m7)
- [8. Assistant: Noted: Soft Reset will be an On/Off row, off by default.  I still need](#^m8)
- [9. booglechan: 2. (a)](#^m9)
- [10. Assistant: I haven't written any code, because KH1 may already have this reset. I](#^m10)
- [11. booglechan: L1+L2+R1+R2+Start+Select](#^m11)
- [12. Assistant: Noted: the combo stays L1+L2+R1+R2+Start+Select.  I still can't tell w](#^m12)
- [13. booglechan: the combo does not work in the pc version so restore it  go ahead](#^m13)
- [14. Assistant: **t707** adds Soft Reset to the QoL list: an On/Off row, off by defaul](#^m14)
- [15. booglechan: I got a log of cutscenes, these take place in traverse town right befo](#^m15)
- [16. Assistant: I haven't built the skipper yet. Your log wasn't enough to tell a stor](#^m16)
- [17. booglechan: Its been logged, you can also see my inputs to tell which ones are wha](#^m17)
- [18. Assistant: I haven't built the skipper yet. Your log shows how these events are s](#^m18)
- [19. booglechan: what I mean is in game when there's a cutscene with dialog that use's ](#^m19)
- [20. Assistant: t709 is written to your testing branch. It only adds logging, so nothi](#^m20)
- [21. booglechan: okay I started by talking to sid, talked to aireth and leon, then I we](#^m21)
- [22. Assistant: This log has no story cutscene in it, so it shows what to leave alone ](#^m22)
- [23. booglechan: update the log](#^m23)
- [24. Assistant: The new log shows a clear difference between the cutscenes you can ski](#^m24)
- [25. booglechan: 1. c 2.(b) 3.(b)  go ahead](#^m25)
- [26. Assistant: t710 is on your testing branch: a new QoL row, Cutscene Skip (Off | Se](#^m26)
- [27. booglechan: it worked for the high poly cutscenes but it waits awhile, it should s](#^m27)
- [28. Assistant: t711 removes the wait. The skip now fires the moment KH1's cutscene bi](#^m28)
- [29. booglechan: I meant the in game cutscene skip didn't work](#^m29)
- [30. booglechan: continue on from my config menu reorganization:  you can make it a sep](#^m30)
- [31. Assistant: **t712** adds a new QoL row, In-game Scene Skip (Off | On, off by defa](#^m31)
- [32. Assistant: I committed t712 in my session's copy of the repo, but I haven't pushe](#^m32)

---

> [!note] Tester booglechan
> can you add an auto cutscene skipper in qol section

^m1


> [!tip] Assistant
> I haven't built the cutscene skipper yet, and I haven't touched any code for it. In-engine scenes and movies run through two separate systems, and I couldn't find a skip route I trust in either.
>
> **In-engine events.** I read the KH1 Steam decomp (Checked decomp). KH1's scripts run on a stack-based interpreter. For example, `0x1401AC700` is one opcode handler that pops values and edits save data. The event flag the mod already reads (`0x14298C9A0`) is set once from the scene id when a field loads, so it only says an event scene is running. I found no skip, end-event or fast-forward logic. The scripts also set story flags and give items, so ending one at the wrong point could break progress.
>
> **Movies.** KH1's decomp only shows a Media Foundation movie player: it opens an `.mp4` by name and decodes it. I haven't found the code that decides when a movie ends.
>
> **What I'd need to find.** I'd have to map which opcodes wait for a frame, a text box or a movie. I'd then pick a point where the script can be pushed to its end while still running everything it sets. That is a reverse-engineering job, and I can't do it responsibly in one pass without watching it run in game.
>
> **Your two settings.** I read "a setting for 1. and a setting for 2." as one QoL row, Cutscene Skip, with three values: Off, Seen (skips only cutscenes you've already watched) and All (skips every cutscene). "Seen" would also need a per-save record of which cutscenes you've watched. If you meant two separate rows, tell me.
>
> **Plan, one version each:**
> 1. A logging-only version. It records each event script's id, its start and end, and every movie start in the log, with no behaviour change. Your next play session then gives me the real ids and timings to build on.
> 2. In-engine event skip, with a Seen/All row in QoL.
> 3. Movie skip, once the movie end is understood.
>
> Reply "go ahead" if you want the logging version first, or tell me if you'd rather I try the skip blind.

^m2


> [!note] Tester booglechan
> go ahead, make sure the log is toggled in the debug menu so we don't bloat performance

^m3


> [!tip] Assistant
> **t706** is written to your testing branch. It only adds logging for cutscenes, scenes and movies, with no change to gameplay. I haven't compiled it. It passed a syntax parse and the version check (`VERSION` = t706 = Stage 706).
>
> **What it adds:** a new "Cutscenes & Events" row on the debug menu's Logs page (ini key `log_cutscenes`). It is off by default. The three hooks only go in with `debug_menu = on` at start, and only on the Steam build. With the category off, each hook costs one flag check. The log stops after 40,000 lines in a session.
>
> The hooks log these three things:
> - **Scene starts:** the arguments KH1 starts a scene with, KH1's event flag, and its scene numbers, before and after.
> - **Event scripts:** the script's position and instruction each time it moves on, with a marker when it ends.
> - **Movies:** the file name each movie opens.
>
> **How I checked it:** I read all three function layouts in the KH1 Steam decomp. Each hook refuses to install if the function's first bytes don't match. It has not run in game. The one assumption is that the movie opener's second argument is the file path and that it is KH1's only movie path. The log will show whether that's right.
>
> **Please test:**
> 1. Turn on `debug_menu` in the ini and restart the game. In the debug menu's Logs page, switch on Cutscenes & Events.
> 2. Start a story cutscene, let it play to the end, and then do the same with a movie.
> 3. Send me `kh1-kh2.log`.
>
> **Still open:**
> - I read "a setting for each" as one QoL row, Cutscene Skip, with Off, Seen and All. Tell me if you'd rather have two rows.
> - The skipper itself needs your go-ahead after I've seen the log.
>
> The notes are in `docs/kh1-cutscene-log.md`, and the Logs table in `docs/kh1-debug-menu.md` has the new row.

^m4


> [!note] Tester booglechan
> before I can do this I need a soft reset in qol you can quick reset the game with l1 l2 select and start pushed together

^m5


> [!tip] Assistant
> The reset is possible, but I haven't written any code. KH1 has no ready-made "go back to title" function, so I'd rather you approve the plan before I build it. I read "1" as the first option on all three questions: title screen, a short hold of about 1 second, and working everywhere.
>
> **What I found in KH1's Steam decomp**
> - **Title screen:** it runs as its own task (`0x14038D9B0`, a state machine). It is started by an init function (`0x14038C610`) that the game's file loader calls once the title assets are in memory. Checked (decomp + exe).
> - **Skipping the logos:** that init has a built-in "start at the menu, skip the logos" mode. It is switched on by the game's `-reboot` command-line flag. Checked (decomp). A reset can use the same mode, so you land straight on New Game / Continue.
> - **No existing reset:** I found no named function that tears the current world down and returns to the title. Game Over and the load flow go through their own scripts, which I haven't traced yet.
> - **Still unchecked:** the world, party and script state is spread across many globals. I haven't mapped what has to be stopped before the title task can safely start.
> - **Button mapping (Assumption):** I haven't checked how KH1's input bits map to L1, L2, Select and Start. The mod already reads the pad for its L1+R1+Triangle debug combo, so I'd reuse that code.
>
> **The plan**
> - **t707, the whole feature:** a QoL row, Soft Reset, that detects L1+L2+Select+Start held about 1 second. It then runs KH1's own scene shutdown and starts the title in the skip-logos mode. No custom state machine and no restarting the process, so it stays native.
> - **Risk:** the shutdown order is the part I haven't verified. The first build may need one or two rounds from your tests, and I'll log each step under a debug-menu category (off by default) so we can see where it breaks.
>
> **Questions**
> 1. Should the row be a normal On/Off setting?
>    - (a) Off by default, so nobody resets by accident (recommended).
>    - (b) On by default.
> 2. Should I trace the Game Over to title path first and reuse its shutdown?
>    - (a) Yes (recommended): it is KH1's own tested path, but it adds one research step.
>    - (b) No: I go straight to a best-guess shutdown and fix it from your logs.
>
> Reply with "go ahead" and your answers.

^m6


> [!note] Tester booglechan
> 1. (a)

^m7


> [!tip] Assistant
> Noted: Soft Reset will be an On/Off row, off by default.
>
> I still need your answer to question 2 before I start. Should I trace the Game Over to title path first and reuse its shutdown (a, my recommendation), or go straight to a best-guess shutdown and fix it from your logs (b)? Reply with a or b and "go ahead", and I'll build t707.

^m8


> [!note] Tester booglechan
> 2. (a)

^m9


> [!tip] Assistant
> I haven't written any code, because KH1 may already have this reset. Its pad code checks for the PS2 soft-reset combo: L1+L2+R1+R2+Start+Select, with all six buttons held. This is in the Steam decomp, and the exe has the same check at two places (`0x14012B3A7` and `0x14012BE03`). When the combo is pressed, KH1 sets a reset-requested flag. That flag makes the platform layer's per-frame update (`0x14012AE60`) report that the game loop should stop, instead of the game continuing.
>
> Per your message, I'll keep the same buttons. Changing them to the four-button version would have meant editing that check, so nothing changes there.
>
> What I haven't confirmed is what the Steam build does once the loop stops. My guess, which I haven't verified, is that it restarts into the title screen using the `-reboot` flag. That flag skips the logos, and I found it in the decomp. It could also quit, or the Steam collection could ignore the request.
>
> **Please test:** in game, hold L1+L2+R1+R2+Start+Select on your controller and tell me what happens. Does it return to the title screen, quit, or do nothing?
>
> - **If it returns to the title:** the feature already exists, and I'd only add the QoL on/off row you asked for. Which of these do you want (a or b)?
>   - (a) The row switches KH1's own check on and off.
>   - (b) I leave it always on and add nothing.
> - **If it quits or does nothing:** I'll build the reset as planned, calling KH1's own title-reset routine (`0x140170630`) when that same combo is pressed. That's t707.

^m10


> [!note] Tester booglechan
> L1+L2+R1+R2+Start+Select

^m11


> [!tip] Assistant
> Noted: the combo stays L1+L2+R1+R2+Start+Select.
>
> I still can't tell whether the game already resets on that combo, and I can't run the game from here. Hold those six buttons in game and tell me which happens: it returns to the title screen, it quits, or nothing happens.
>
> If it returns to the title, I only add the QoL on/off row. Otherwise I build t707 so the same combo calls KH1's own title-reset routine.

^m12


> [!note] Tester booglechan
> the combo does not work in the pc version so restore it
>
> go ahead

^m13


> [!tip] Assistant
> **t707** adds Soft Reset to the QoL list: an On/Off row, off by default. With it on, holding L1+L2+R1+R2+Start+Select for 1 second returns the game to the title screen.
>
> **How it works:** the mod doesn't run its own reset. It sets KH1's game mode back to its start-up step, so KH1's own mode step runs its own full reset and loads the title screen as at boot. There is no new hook, and the only cost with the row off is one flag check in the pad read.
>
> **How I checked it:**
> - **Decomp:** I read the mode step, the reset routine and the title entry in the Steam decomp. The six-button combo mask, `0x0F09`, is KH1's own, and it matches the mod's pad bits (Select `0x0001`, L1 `0x0400`).
> - **Exe:** the mod checks the first bytes of the mode step and the reset routine, and the mode's address, once at start. It leaves the row unused if they don't match or the build isn't Steam. It does nothing if you're already on the boot or title screen.
> - **Not checked in game.** I'm assuming KH1's boot reset also clears whatever a running world leaves behind. The PS2 overlay loads it relies on are empty stubs in this build.
>
> **Please test:**
> 1. Turn on Config, QoL, Soft Reset, then hold the six buttons for 1 second during normal play. Check that you land on the title screen, that Continue loads your save, and that the game doesn't crash.
> 2. Try the same in a battle, a cutscene, and the Gummi map.
> 3. Release the buttons early and check that nothing happens. Start may open the pause menu during that second, since I don't hide the buttons.
> 4. If it misbehaves, switch the Menus & Config log on in the debug menu and send me `kh1-kh2.log`.
>
> **Still open on my side (no answer needed):** the cutscene skipper still waits on the t706 log you'll send.
>
> The notes are in `docs/kh1-softreset.md`.

^m14


> [!note] Tester booglechan
> I got a log of cutscenes, these take place in traverse town right before and after you fight guard armor, lets make sure not to skip item shop text, any npcs you talk in the overworld. (basically any "cutscene" (if it is called that in game) that the player voluntarily makes happen.
>
> go ahead

^m15


> [!tip] Assistant
> I haven't built the skipper yet. Your log wasn't enough to tell a story cutscene from a talk, so I made **t708**, a better log that still doesn't change gameplay.
>
> **What your log showed:**
> - **Scene and movie hooks:** the scene-prepare and movie hooks logged nothing. Traverse Town's events run as scripts inside the room, not as separate scenes. Checked (log).
> - **Script lines:** my t706 script lines were unreadable. I had read the wrong object, so they showed junk like "counter 53060 op 0xff". That was my bug, and t708 fixes it. Checked (decomp).
>
> **What t708 changes:**
> - **Each script start:** the log now records the script's entry, KH1's scene flags, and what the script is made of (its length and its distinct instructions). The Cutscenes row on the debug menu's Logs page is still off by default.
> - **Telling scripts apart:** I'm assuming a shop or NPC talk looks different from a story scene, for example short and message-only. That's why I need the log with your labels.
>
> **How I checked it:** the script starter and its context layout come from the Steam decomp. I haven't run t708 in game.
>
> **Please test:**
> 1. Turn on the Cutscenes log, start a new session, and go to Traverse Town.
> 2. Do these in order, with about 15 seconds between each: talk to a plain NPC, talk to the item shop, trigger the cutscene before Guard Armor, and trigger the one after it.
> 3. Tell me roughly how many minutes into the session each one happened. The log's "alive Ns" lines give me the clock.
> 4. Send `kh1-kh2.log`.
>
> **Still open on my side:** the skipper needs your go-ahead again after that log. I'll make sure it skips only scripts that aren't player-started.
>
> The notes are in `docs/kh1-cutscene-log.md`.

^m16


> [!note] Tester booglechan
> Its been logged, you can also see my inputs to tell which ones are what, but first I went from the shop and talked to the shop keeper (sid) then I went out and talked to aireth, then leon they both appear in the first district before guard armor. then about 1 minute or 2 later I defeated guard armor, skipped the first 2 cutscenes and played the last one out, then an in game dialog cutscene played (which we want to skip) played out 
>
> go ahead

^m17


> [!tip] Assistant
> I haven't built the skipper yet. Your log shows how these events are structured, and I need one answer from you before I can build the skip safely.
>
> **What the log showed (Checked, your log):**
> - **Room starts:** each room entry starts one script, which the log caught: Sid's shop, the first, second and third districts, and the Guard Armor arena (world 0xf, room 0x10, about 235s).
> - **NPC talks:** Aerith, Leon and Sid's talks created no new script starts. KH1 keeps one script context per NPC or object, and a talk just wakes a sub-script inside the context that is already running. My hook only sees room-level starts, so it can't tell your talks from story events yet.
> - **The dialog after Guard Armor:** you re-enter the first district at about 294s. That room load starts an extra script entry, and Sora can't act for about 35 seconds (294–329s). This is almost certainly the dialog you want skipped.
>
> **How I'd tell yours from story events (Assumption, one sample each):** that dialog's scene flags differ from your talk with Sid. Sid's talk had bit `0x800` set in the second flag word. The post-fight scene had bit `0x400` set in the first word. I don't want to rely on one sample each.
>
> **What I couldn't find:** I found no skip function in KH1's decomp, but you said you skipped the first two cutscenes. If the game has its own skip, the best native design is for the auto-skipper to trigger it, and the player-started talks would simply never offer it.
>
> **Question:** how did you skip those two cutscenes? Please tell me:
> - (a) the button you pressed (for example Start), and whether it was a pause-menu option or a direct press;
> - (b) whether it was the same button for both.
>
> **Plan after your answer:**
> - **t709:** log the sub-script starts inside each NPC and object context, plus the scene flag words at the moment you pressed the skip button. Same debug-menu log category, off by default.
> - **t710:** the skipper itself.
>
> Reply with your answer to the question and "go ahead".

^m18


> [!note] Tester booglechan
> what I mean is in game when there's a cutscene with dialog that use's the high poly models you can pause the game and select skip cutscene. for that we can make auto skip using its own functions. for the other cases you can't you have to press the confirm button through all the dialogs until it ends, in those situations the game will sometimes give you items, like during the cutscene with donald and goofy you get dodge roll.

^m19


> [!tip] Assistant
> t709 is written to your testing branch. It only adds logging, so nothing in the game plays differently. I haven't compiled it or run it in game.
>
> **What t709 does.** With the Cutscenes log on, it writes a `watch:` line whenever one of KH1's cutscene flag words, the event player state, the NPC script count or your pad buttons change. Pausing, choosing Skip Cutscene and pressing confirm should each leave a visible mark in the log. It only runs with the debug tools on, and costs one atomic load when the log is off.
>
> **How I checked it.**
> - Checked (decomp): which game words to watch.
> - I only ran a syntax check on the code. I couldn't check what the bits mean, so that stays an assumption until your log shows them.
> - I still haven't found the pause menu's "Skip Cutscene" function, because the decomp has no skip text. This log is how I'll narrow it down.
> - I'm also still assuming the single three-value QoL row (Cutscene Skip: Off | Seen | All) is what you meant by "a setting for 1 and a setting for 2". Tell me if you wanted two separate rows.
>
> **Please test:**
> 1. Turn on the debug menu's Cutscenes log, then start the game.
> 2. In a cutscene with dialog and high-poly models, pause and choose Skip Cutscene.
> 3. Play a low-poly dialog cutscene (like the Donald and Goofy one) and press confirm through every line.
> 4. Talk to an NPC and to Sid in the shop.
> 5. Send me `kh1-kh2.log` with rough times for each step.
>
> **Still open on my side.** The skipper (t710) waits for this log and your go-ahead. It will call the game's own skip function and leave talks, shop text and non-skippable dialog cutscenes alone. Movie skipping is a separate stage after that.
>
> The notes are in `docs/kh1-cutscene-log.md` and AGENTS.md Stage 709.

^m20


> [!note] Tester booglechan
> okay I started by talking to sid, talked to aireth and leon, then I went and inspected the vault that gives you a post card, then I went and inspected a chest that can't be open until you use blizzard and hit all the candles

^m21


> [!tip] Assistant
> This log has no story cutscene in it, so it shows what to leave alone but not yet how to skip anything. I read it straight from your game folder and matched it to what you did, in order. That order is an assumption for the last four events.
>
> **What your talks and inspections look like.** Every one of them (Sid, Aerith, Leon, the vault, the chest) starts after a Triangle press. Each sets bit `0x800` in KH1's second cutscene flag word and puts the engine word at `0xc` with Sora unable to act. Sid's shop also loads event file 10 and sets `0xc00`, because the shop menu runs through the nested event player. Plain talks don't do that. This is checked from the trace.
>
> **What never turned on.** Bit 0 of the second flag word, which is the cutscene-mode bit, stayed off for all of these. Whether that bit separates a story cutscene from a talk is still an assumption until a real cutscene is logged. The `0x800` bit also stays set after a talk until the next room script resets it, so it means "a talk started", not "a talk is running".
>
> **What the room changes look like.** Entering a room shows `0x400` and `0x100` in the first word and the engine word dropping to `0x8`. That is not a cutscene either.
>
> **One problem with the log.** It is full of lines that only record button presses, mostly from the menus. That drowned out the real changes. I can trim it to log button changes only while a scene flag is active, as its own tiny version, or leave it. Tell me which you want.
>
> **Next log.** I still need the part you described before: the Guard Armor aftermath. Please do these in order:
> 1. In a high-poly dialog cutscene, pause and choose Skip Cutscene.
> 2. Then play a low-poly dialog cutscene all the way through, pressing confirm for every line.
>
> Give me rough times for each, as before. That should show whether bit 0 turns on and which words move when you skip. It should also help me find the game's own skip function, which I haven't located yet.
>
> No code until you've said go ahead again. The notes are in `docs/kh1-cutscene-log.md`, which is a docs-only change with no version bump.

^m22


> [!note] Tester booglechan
> update the log

^m23


> [!tip] Assistant
> The new log shows a clear difference between the cutscenes you can skip and the ones you can't. I wrote it up in `docs/kh1-cutscene-log.md`, which is a docs-only change with no version bump. Which scene is which comes from their order and your button presses, so that part is an assumption.
>
> **Skippable (high-poly) scenes.** There are five in the log. In each, the same three things happen in the same order:
> - The second flag word gets `0x400`.
> - The engine word then gains `0x40000`.
> - Bit 0 of the second word (`0x1`) then comes on.
>
> This is checked from the trace.
>
> **The skip itself.** You pressed Start, then Down, then Confirm. On the next frame the runner word (`0x1423AB2D0`) goes from 2 to 3 and the engine word gains `0x400`. About a second later the scene ends and everything returns to normal. That fits my earlier lead that `0x1401A22B0` is the game's own skip action, because it sets bit 0 of that same word. I haven't re-read that function in the decomp yet, so it is still an assumption. Pressing Start without choosing Skip changed none of these words.
>
> **The dialog scene you played out.** After Guard Armor, in the room entered at about 190s, the second word only showed `0x400`. The engine word stayed at `0xe`, and `0x40000` and bit 0 never appeared. Talks and the shop also never set those bits. So a skipper that waits for `0x40000` and bit 0 together would only fire on skippable scenes. This is checked on your six scenes. I'm assuming it holds for every story scene in the game.
>
> Nothing is built yet. Three questions before I write t710:
>
> 1. **How should "Seen" decide a scene was already viewed?**
>    - (a) Keep a list in `kh1-kh2.ini`, keyed by room and event file. It's simple, but the list is shared across all your saves.
>    - (b) Make it Off | On only. This is the simplest.
>    - (c) Find the game's own seen-flags in the decomp first. This is the most accurate but slower.
>    - I recommend (c), then (a) if the flags can't be found.
>
> 2. **When should the skip fire?**
>    - (a) As soon as the skippable signature appears.
>    - (b) After about one second, so the scene's first frame loads cleanly.
>    - I recommend (a), because that matches how you press it. If it glitches, I'd add the delay.
>
> 3. **How should I detect the scene starting?**
>    - (a) Read the two words each frame in a hook that already runs, only while the option is on. This is cheap.
>    - (b) Hook the game function that sets `0x40000`, which costs nothing per frame but needs the decomp first.
>    - I recommend (b) if I can find the function. Otherwise (a).
>
> I can also trim the button-press noise in the log if you want. Write "go ahead" with your answers and I'll build t710.

^m24


> [!note] Tester booglechan
> 1. c
> 2.(b)
> 3.(b)
>
> go ahead

^m25


> [!tip] Assistant
> t710 is on your testing branch: a new QoL row, Cutscene Skip (Off | Seen | All, default Off). I haven't run it in game.
>
> **What it does.** With the row on, a second after a skippable cutscene starts, the mod writes the one flag byte that the pause menu's Skip Cutscene sets. KH1's own script step then runs its own skip. Talks, the shop and the confirm-every-line dialog scenes never turn on the cutscene bit, so they are never touched.
>
> **How I checked it.**
> - **Skip mechanism:** I checked it in the decomp and the exe. It also matches your log: after the menu, the runner word goes 2 → 3 and the engine word gains `0x400`.
> - **Which scenes qualify:** your two logs show bit 0 of the second flag word on in all six high-poly scenes. It was never on in the low-poly dialog scene, the talks or the shop.
> - **Before it does anything:** the mod checks the exact bytes of the game's skip routine and of the script step's read and clear of that flag byte. If they don't match, the row does nothing.
>
> **Three things differ from what you picked.**
> - **Detection (your answer 3b):** I didn't hook the function that sets the cutscene bit. It has 13 callers, its first instruction can't be copied safely, and the bit is also cleared directly in other places. Instead the mod reads that one word in its existing pad read, with one atomic load when the row is off. Tell me if you'd rather I try the hook anyway.
> - **Seen (your answer 1c):** I searched the decomp for the game's own watched-scene record and found none. So the mod keeps its own list in `kh1-kh2.ini` (`cutscene_seen`), keyed by room, event file and the scene's order in that room visit. The list is shared across all your saves, so delete that line when you start a new game.
> - **Bit 0 is the gate, not `0x40000`:** `0x40000` turned out to be an event sound mode that happens to appear alongside it.
>
> **Please test:**
> 1. Set QoL > Cutscene Skip to All and turn on the Cutscenes log. Walk into a Traverse Town cutscene; it should skip about a second after it starts. Check that the fade and the next scene look right, since I'm assuming skipping without the pause menu looks the same.
> 2. Check that Sid, Aerith, Leon, the vault and the chest still work as before.
> 3. Play the post-Guard-Armor dialog scene. It should not skip and should still give you any items.
> 4. Set it to Seen. The first run of a scene should play, and the same scene on a retry after a game over should skip. I'm assuming the room's scene counter restarts when you re-enter the room.
> 5. Send `kh1-kh2.log` with rough times for each scene.
>
> **Still open on my side.** I haven't checked how scene numbering behaves in every room, or the one second wait on a slow load. Movie skipping is a separate stage if you want it. Notes are in `docs/kh1-cutscene-skip.md` and AGENTS.md Stage 710.

^m26


> [!note] Tester booglechan
> it worked for the high poly cutscenes but it waits awhile, it should skip the first moement the cutscene loads.
>
> the other cutscene skipper didn't work at all
>
> go ahead with the change and read the decomp

^m27


> [!tip] Assistant
> t711 removes the wait. The skip now fires the moment KH1's cutscene bit turns on. I haven't run it in game. The in-game dialog scenes are a separate question: I've read the decomp but not built anything for them, and I have two questions below.
>
> **How I checked t711.** Your t710 log showed five skips that each worked and were each written 30 log frames after the bit came on, which was the wait. KH1's script step only accepts a skip once that bit is on. The scene's own script starts 15 to 100 frames earlier, so the bit is the earliest point a skip can fire. Checked from the decomp and your trace. I couldn't check the look of a skip with no wait.
>
> **Please test.** Do the same cutscenes with Cutscene Skip on All. Tell me whether anything glitches now that it skips at once (fade, sound, camera). I'd also like one test of Seen: leave a scene, retry it, and see whether it skips the second time only.
>
> **The in-game cutscene skip.** I'm reading you as meaning the low-poly dialog scenes you confirm line by line, like the one after Guard Armor.
> - **No skip exists for them in KH1.** The pause menu only offers Skip Cutscene when the bit is on, and these scenes never turn it on. Checked from the decomp and your logs.
> - **Why a plain skip could lose items.** Skipping the high-poly scenes works because the script keeps running to its end and only the presentation steps (camera, sound, fades, waits) are cut. Item and flag steps still run. Checked in the decomp. That is why these scenes can't just be cut: the same design would have to keep the item steps running.
> - **What I haven't found.** I couldn't find where the dialog lines wait for your confirm press, so I don't know whether KH1's own fast-forward would also advance the text. I won't guess.
>
> Questions:
>
> 1. **What should it do?**
>    - (a) Auto-advance every line as if you pressed confirm. Items and flags happen exactly as played, and voices are cut short.
>    - (b) Use KH1's own skip path on these scenes. This only works if the lines honor it, which I'd check first.
>    - (c) Whichever (a) or (b) matches the game's own behavior, after I read its message code.
>    - I recommend (c): I read the message window code next and report before any code.
> 2. **Where should it live?** Either a separate QoL row, Dialog Scenes (Off | On), or a third value on the Cutscene Skip row. I recommend the separate row, since this auto-advances rather than skipping.
>
> Talks and the shop would stay untouched either way, because the player starts them.
>
> **Movies.** Movies played from events turn the same cutscene bit on, and KH1's skip routine stops a playing movie, so t711 should already cover them. The opening movie already has its own button skip. No movie played in your log, so none of this is tested.
>
> Tell me "go ahead" with your answers and I'll build the next version.

^m28


> [!note] Tester booglechan
> I meant the in game cutscene skip didn't work

^m29


> [!note] Tester booglechan
> continue on from my config menu reorganization:
>
> you can make it a seperate config option in qol, what I want you to do is have the cutscene skip 
>
> how it should work is it skips that in game cutscene fully, then if any flags give you an item the game gives you the item after the fact 
>
> go ahead

^m30


> [!tip] Assistant
> **t712** adds a new QoL row, In-game Scene Skip (Off | On, off by default). It skips the low-poly dialog scenes the pause menu won't skip, and any items, abilities and story flags those scenes give are still given. The changes are written into your local testing folder, not pushed: your t708–t711 work there isn't committed yet, so it's yours to commit. I couldn't run `cargo test` in this session because the crate registry isn't reachable from here, so run it before you push.
>
> **How it works.** It uses KH1's own pause-menu skip. When the mod tells the game a scene is skippable, KH1's own skip runs the rest of the scene in a few frames. Messages never open, but the item and ability give still runs, along with every story flag, in the scene's own order. KH1 then fades back in by itself. So the item isn't added by the mod afterwards; the game gives it during the skip, which is instant. The one thing you won't see is the "Obtained" popup, because KH1 doesn't show it during a skip.
>
> **What it leaves alone.** Before skipping, the mod reads the scene's code once. These always play:
> - Talks and shops.
> - Scenes with a choice in a message. A skip would quietly keep the old answer, which matters for things like the Destiny Islands questions.
> - High-poly cutscenes. Those stay on the Cutscene Skip row.
>
> **How I checked it.**
> - **Decomp:** KH1's skip, the messages being suppressed while the item give still runs, and how it ends.
> - **Exe:** the skip function's first bytes, the step's checks, and every scene function number used. These are all checked at start, and the row does nothing on a mismatch.
> - **Your t709/t710 logs:**
>   - The Traverse Town low-poly scene (room 3 0 4) matches what the new row looks for.
>   - Its item message came before the give, so they're separate functions.
>   - The high-poly marker was in all ten high-poly scenes.
>
> **Please test** (with the Cutscenes log on, look for "in-game skip" lines):
> 1. Turn on QoL > In-game Scene Skip, then trigger the Traverse Town scene that gives Dodge Roll. It should fade out, skip, and Dodge Roll should be in your abilities.
> 2. With Cutscene Skip off, play a high-poly scene: it should play, not be skipped by the new row.
> 3. Talk to an NPC and open a shop: both should be untouched.
> 4. After a skip, pause in normal play: the pause menu shouldn't offer Skip Cutscene.
>
> **Still open (Assumptions):**
> - I'm assuming every high-poly scene in other worlds has the same marker; only Traverse Town's were in the logs.
> - A scene that chains straight from a skipped one gets skipped too, same as with the pause menu.
> - After talking to someone, a scene in that same room plays, because KH1 keeps the talk flag until the room reloads.
>
> The row is Off | On for now. If you want a Seen option like Cutscene Skip has, I can add it. Notes are in `docs/kh1-ingame-skip.md` and `docs/kh1-config-menu.md` (6.z5).

^m31

currently works in traverse town, wasn't able to test it in other worlds, but can you also make it skip trinity animations?

go ahead

