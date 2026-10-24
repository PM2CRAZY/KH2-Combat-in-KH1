# Frame advance in the debug menu

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr. Bugs: Can you add a frame advance feature into the debug menu? whe](#^m1)
- [2. Assistant: Sounds useful, and since it's a debug menu tool the match-KH](#^m2)
- [3. Mr. Bugs: 1. holding for for a second causes it to run at full speed](#^m3)
- [4. Assistant: Got it, thanks. I checked the Steam KH1 decomp for the fps p](#^m4)
- [5. Mr. Bugs: yes so if you held square and advanced 5 frames that would b](#^m5)
- [6. Assistant: Got it, that's how I'll do the buttons: held over 5 steps is](#^m6)
- [7. Mr. Bugs: do option b](#^m7)
- [8. Assistant: t371](#^m8)
- [9. Mr. Bugs: I did what you asked and currently frame advance is not work](#^m9)
- [10. Assistant: t372](#^m10)
- [11. Assistant: t377](#^m11)
- [12. Mr Bugs: the in game states seem to be frozen, we have a new bug thou](#^m12)
- [13. Assistant: t378](#^m13)
- [14. Mr Bugs: its still black when frame advance is on until you advance a](#^m14)
- [15. Assistant: t399](#^m15)
- [16. Mr Bugs: I'll send a log but a good lead is the games own pause mecha](#^m16)
- [17. Assistant: Good lead, thanks. I followed it into the Steam exe this tim](#^m17)
- [18. Mr Bugs: 130000 0x2a23e0 - model render](#^m18)
- [19. Assistant: t426](#^m19)
- [20. Mr Bugs: also if you didn't make the correction "100000 0xle5d90" the](#^m20)
- [21. Assistant: t445 to t447. Thanks, that correction helped. I found why ef](#^m21)

---


  

> [!note] Tester Mr. Bugs

> Can you add a frame advance feature into the debug menu? when you enable it you can frame advance by 1 frame by pressing R2 or pressing the period key on the keyboard, if you hold down the frame advance button then the game plays at normal speed.

^m1


> [!tip] Assistant
> Sounds useful, and since it's a debug menu tool the match-KH2 rules don't apply to it. My plan is a Frame Advance row in the debug menu. Turning it on freezes the game (the menu, HUD and picture still draw), each tap of R2 or the period key plays one game update, and holding it plays at normal speed until you let go. Music and sounds would freeze with the game. A few things I need from you first:
>
> 1. **How long a hold before it runs at normal speed?** (a) about a third of a second, like the menu's held-direction repeat, so a quick tap is always exactly one frame; (b) right away, so holding for 3 frames plays 3 frames; (c) something else.
> 2. **What counts as one frame?** KH1 updates once per drawn frame, and at the 30 fps setting each update is two 60 Hz steps. (a) one game update, whatever your fps (at 30 fps a tap moves 2/60 s); (b) always one 60 Hz step, which at 30 fps means drawing every step. (a) is simpler and changes nothing else. If you play at 60 fps they're the same.
> 3. **R2 is already used.** The mod's KH2 lock-on uses R2 + right stick to switch targets, and KH1 reads R2 too. While frame advance is on, should R2 (a) be only frame advance (hidden from the game, so no target switching), or (b) still reach the game as well? Or (c) use a different button on the controller.
> 4. **Buttons pressed while frozen.** Held buttons go into the next frame you step, so you can hold Square and step through a swing. If you tap a button between steps and let go before stepping, should the next frame (a) still see that press, or (b) only see what's held when you step?
>
> The period key only counts while the game window is focused. Your thread's folder is `testers/frame-advance-in-the-debug-menu` if you have traces or logs to add.

^m2


> [!note] Tester Mr. Bugs
> 1. holding for for a second causes it to run at full speed
> 
> 2. it needs to work no matter the fps setting, if your unsure on how that works check the C decomp for the steam version of kingdom hearts 1
> 3. having it set on R2 should disable any inputs R2 usually takes 
> 4. if I have a button held then I advance to the next frame it will count as a button press, if I let go then advance to the next frame then its not a button press
>    
>    also trace recordings need to advance frame by frame as well, so if I have trace set to 3 seconds that timer only advances frame by frame. please let me know how you would implement it.

^m3


> [!tip] Assistant
> Got it, thanks. I checked the Steam KH1 decomp for the fps part. Here's how I'd build it:
>
> **How KH1 runs a frame.** Each pass of KH1's field loop (`0x140170D50`) runs the game's update, then draws, then a pacing step (`0x14018D1F0`), then shows the picture. The pacing step counts how many 60 Hz ticks went by since the last pass and sets KH1's frame scale (`0x14233FBDC`) from that: 1 at 60 fps, 2 at the 30 fps setting (KH1 reads which one you picked through `0x1400FDB90`), capped at 5 after a hitch. Everything in KH1, and our KH2 side, moves by that scale.
>
> **Freezing.** With Frame Advance on, the update part of that loop is skipped and the drawing and showing keep going. So the picture, HUD and debug menu stay up, the window stays responsive, and nothing in the game moves. Sounds and music pause with it. (Assumption to check before coding: exactly which calls in the loop are the update and which are the drawing. I'll confirm each one in the decomp.)
>
> **One step at any fps.** While frozen, the pacing step sees a long gap. If nothing else changed, the first step would jump 5 frames' worth. So each step gets the scale a normal frame gets at your setting: 1 at 60 fps, 2 at 30. One step is always one real game frame, the same as during play. Our KH2 side follows the same scale, so KH2 moves step the same way.
>
> **R2.** While Frame Advance is on, R2 is removed from the pad before KH1 and the KH2 side read it, in the same place the debug menu hides buttons now. Nothing else sees R2 (so no lock-on target switching). The period key only counts while the game window is focused.
>
> **Holding.** Holding R2 or period for 1 second runs the game at full speed until you let go. Then it freezes again on the next frame.
>
> **Buttons.** A stepped frame sees exactly what's held when you step. Nothing pressed and let go in between is kept. One thing to confirm: if you keep a button held over several steps, the first step counts as the press and the next ones as holding it, the way it works in normal play (so holding Square over 5 steps is one press, not 5 swings). Is that what you want?
>
> **Trace.** The trace already writes one row per game frame, but its "last 3 seconds" window is measured in real time, so slow stepping would drop rows. With Frame Advance on, the window counts game time instead (each frame's scale): 3 seconds is 180 ticks, so 90 frames at 30 fps or 180 at 60. Select + Triangle still saves the trace while frozen.
>
> **The setting.** A Frame Advance row (Off / On) on the debug menu's main page. It's not saved to the ini, so it starts Off each time you launch the game, like the stat locks. Say if you'd rather it be saved.
>
> If that all sounds right, answer the held-button question and write `Go ahead`.

^m4


> [!note] Tester Mr. Bugs
> yes so if you held square and advanced 5 frames that would be one press e.g
> square press advance for 5 frames > let go of button > advance next frame > button is counted as released
> 
> for R2 make sure the period key works the same way
> 
> in the trace menu add a little check that tells the player if in game frames or real time is on like:
> 
> game frames = ON 
> 
> let the user also disable it as well even if they're in frame advance

^m5


> [!tip] Assistant
> Got it, that's how I'll do the buttons: held over 5 steps is one press, and the first step after you let go sees it released.
>
> **Period key, same as R2.** KH1 reads the keyboard through DirectInput (it's set up at `0x1401122E0`). While Frame Advance is on, the period key is taken out of what KH1 reads, so whatever it's bound to in KH1's keyboard settings won't happen. Tapping it steps one frame, and holding it for 1 second runs at full speed until you let go, the same as R2. (Assumption to check before coding: where KH1 keeps the key states it reads each frame. I'll find that in the decomp.)
>
> **Game frames row in the trace menu.** A new row on the Trace Recorder page, `Game Frames: On / Off`. On means the trace's seconds count game time; Off means real time. You can switch it either way at any time, frame advance or not, and it's saved to the ini like the other trace settings. One question: what should it start as?
> - (a) On all the time by default. During normal play, game time and real time only differ when the game slows down or hitches.
> - (b) Off by default, turning On by itself when you turn Frame Advance on (you can still switch it back).
>
> Once you've picked, write `Go ahead` and I'll start.

^m6


> [!note] Tester Mr. Bugs
> do option b
> Go ahead

^m7


> [!tip] Assistant
> t371
>
> **Where it is.** Debug menu main page, `Frame Advance < On / Off >` (left / right or confirm). Turning it on also turns on the new `Game Frames` row on the Trace Recorder page (option b), and you can switch that back off there. Turning the debug menu off in Config also turns Frame Advance off. It's Steam only.
>
> **How it holds the game.** I used KH1's own pause. Between frames it does what KH1's pause menu does: only KH1's pause-flagged tasks run (the drawing), and every model is marked paused. The pad, drawing and frame pacing keep going, so the picture and the debug menu stay up. It hands control to KH1 while KH1's own pause menu is open.
>
> **Correction to my earlier plan.** KH1's frame pacing runs on held frames too, so there's no catch-up jump to fix. A stepped frame gets the same frame scale a normal frame gets at your fps setting (1 at 60, 2 at 30).
>
> **Check I made: decomp only.** I read the Steam KH1 decomp for the field loop, the task list and its mask, the model pause call, the frame pacing and the pad record. Nothing has been checked in game yet; a trace from you is the 1:1 check. Notes and addresses are in `docs/kh1-frame-advance.md`.
>
> **What I couldn't check or do:**
> - The mask value KH1's pause uses (1) comes from older work on the Epic build. In the Steam decomp that part of the pause code wasn't decompiled.
> - **Sound:** I couldn't find what KH1's pause does with sound, so I didn't add anything. Music most likely keeps playing, and sounds already started finish. That's different from what I told you earlier. If you want sound held too, tell me and I'll do it as its own version.
> - **Period key:** it steps the game, but KH1 itself can still see it. Hiding it from KH1 needs a hook in KH1's key mapping, and I need to read the exe's bytes for that, which I couldn't do this time. R2 is fully hidden.
> - If you open KH1's pause menu while the game is held, the frame it closes on may play once.
> - Some of the mod's timers still count real time while the game is held (for example how long Sora keeps his battle stance after a fight). The main ones that decide KH1 vs KH2 control and Sora's model are switched to game time.
>
> **Please test:** step through a KH2 combo at 30 fps and at 60 fps, and grab a Select + Triangle trace while stepping (the trace now saves while the game is held, too). Also check that the camera and right stick do nothing while the game is held.

^m8


> [!note] Tester Mr. Bugs
> I did what you asked and currently frame advance is not working for all states, right now when you frame advance only effects that are in the world advance frame by frame I've uploaded my traces and log to the testers folder for you, it has me doing attacks at 30 and 60 and with game frame off and on, on both kh1 and kh2 moves.
> 
> Whatever you need to get from the decomp do it and take your time. it make sure its the gihidra decomp of the  steam exe as I'm playing on the steam version. make sure you document your findings in a doc if you haven't already.
> 
>  I can't see if you need my input till after the task is uploaded to GitHub so just complete what you can following our philosophies 
> 
> 
> if there's also more threads in this doc deploy as many Subagents as you can, if we use the max amount of subagents, once its task is done, have that subagent move on to the next thread, if it can't be done make sure you complete the first threads task, then once its completed move onto the next thread if there is one and fulfil what its asking and so on. I'm letting you do this with go ahead
> 
> go ahead

^m9


> [!tip] Assistant
> t372
>
> Thanks for the traces; they showed it clearly. In all 8 of them, Sora's physics frames kept running every frame while the game was meant to be held. Only the effects held.
>
> **Why.** t371 held the game by writing KH1's task mask, the value KH1's pause uses to run only its pause tasks. But KH1's pause machine sets that mask again every frame, just before the tasks run, so my value was overwritten. The other thing I called was actually KH1's effect pause, not a model pause. That's why effects were the only thing that stepped.
>
> **What t372 does.** While the game is held, every task KH1's pause would hold has its function swapped for one of ours that does nothing, and gets its own back for a frame that runs. The actors (Sora, the party, enemies) all run from one of those tasks, so they hold too. The draws, the HUD and the debug menu are pause tasks, so they keep going. Effects are still paused with KH1's effect pause, as before. It's still only data writes, no new hooks. Everything else from t371 is the same: one frame per tap at your fps setting, 1 second held for full speed, the button rules, R2 hidden, Game Frames.
>
> **Check:** decomp (the Steam KH1 decomp: the task list, the actor list and its task at priority 80000) plus your t371 traces for what went wrong. Not checked in game yet. Assumption: the tasks KH1's pause keeps running don't move anything in the world. The camera's task might be one of them, so please check whether the right stick moves the camera while held. Notes are in `docs/kh1-frame-advance.md`.
>
> Still open from before: the period key isn't hidden from KH1 itself, and the sound question.
>
> I moved your files into the thread folders with the hyphen names (`testers/frame-advance-in-the-debug-menu` and so on) so each thread has one folder. I'm working through the other threads now.

^m10


> [!tip] Assistant
> t377
>
> The period key is now hidden from KH1 too, so it works exactly like R2: while Frame Advance is on, only Frame Advance sees it. I could read the Steam exe this time, so this is a small hook at the start of KH1's keyboard-to-pad mapping (`0x1400D7E80`; its first bytes are checked before it installs). It clears the period key in KH1's keyboard buffers before KH1 reads them. Check: the Steam exe and decomp; not checked in game yet.
>
> The build at the top of the testing branch (`kh1-kh2.dll`) updates by itself a few minutes after each version, so pulling gets you t377.

^m11


> [!note] Tester Mr Bugs
> the in game states seem to be frozen, we have a new bug though the game turns black when a frame advance is on, you only see a frame every frame advance or when you hold the frame advance key down
> 
> Can you also add a hard rule to the tester thread that any time there's multiple threads to have sub agents deployed on each thread when possible? 
> 
> go ahead

^m12


> [!tip] Assistant
> t378
>
> Good, the state holding is working. The black screen came from the same change: t372 held every task KH1's pause doesn't run, and most of KH1's drawing (the map, effects, HUD) is in that group. I checked every task KH1 registers in the decomp, and only about 20 carry the pause flag; the model pass is one of them, the map and effects aren't. KH1's own pause gets away with it because its screen is built from the last frame.
>
> So now only the update tasks that run before KH1's first draw pass are held: the actors, the camera, scripts and the other game logic. Every draw from there on runs each frame, so the picture should stay up while held. The log lists the tasks it held the first time ("frame advance: holding N tasks"), which tells us whether the split is right if anything still looks off.
>
> **Check:** decomp (the task registrations and their flags). Not checked in game. Assumption: nothing in the draw group moves the world. Two things there do still run while held: the HUD and menus (so the command menu still answers the pad), and KH1's own effect fade-out.
>
> The hard rule is in the assistant section now ("Several threads"): when more than one thread has a message waiting, each gets its own sub-agent working at the same time, and their work is merged one version at a time. I'm doing that for this round: the effects, combo rows, sleeves and camera threads each have one working now.

^m13


> [!note] Tester Mr Bugs
> 
> its still black when frame advance is on until you advance a frame, is there anything I can do to help log the error for you in game?
> 
> also can you make every thread a new document inside their respected names in the tester folder? that way when an agent updates the github for a different thread I can respond to it instantly without having to wait for the whole document to be finished. 
> 
> go ahead

^m14


> [!tip] Assistant
> t399
>
> Yes, you can help, and your last log already did: it lists the 22 tasks frame advance holds while the game is held ("frame advance: holding 22 tasks"). One of them must be something the picture needs every frame, but I can't tell which from the decomp alone, so t399 gives you a way to find it in game.
>
> The debug menu's main page has a new **Held Tasks** page. It lists each held task by priority and function address, each one `< Held >` or `< Runs >` (left / right or confirm switches it). A task set to Runs keeps running while the game is held. To find the one:
> 1. Turn Frame Advance on (black screen).
> 2. Open Held Tasks and switch tasks to Runs one at a time, starting from the bottom of the list (the highest numbers, nearest the drawing: 139999, 137000, 135000, 130000, 129999, 120000, 110000), until the picture comes back.
> 3. When it does, note which one, and also whether anything moves that shouldn't (Sora, enemies, the camera).
>
> Every switch is written to the log with the full list of what's running, so just send the log afterwards and I'll take it from there. The switches last until you close the game.
>
> My guesses, if you want to start there: 137000 (it runs a second list of its own, which may fill what gets drawn) and 135000 (it copies the camera into the slot the drawing reads).
>
> **Check:** decomp for the task list; not checked in game.
>
> Your other request is done too: every thread now has its own file in its folder in `testers` (this one is `testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md`), and `docs/tester-requests.md` keeps the rules, the contents and the Requests section for starting new threads. I push each thread's reply as soon as it's written, so you can answer one while the others are still being worked on.

^m15


> [!note] Tester Mr Bugs
> I'll send a log but a good lead is the games own pause mechanic, when you pause a game during a battle the whole game freezes and your still able to see the game, a text saying P A U S E appears. I'm sure you can look in the kh1 steam ghididra decomp for that. (I'm pretty sure the regular pause screen works that way too, just with some blur affects added on top)
> 
> go ahead

^m16


> [!tip] Assistant
> Good lead, thanks. I followed it into the Steam exe this time (the decomp had skipped that part), and it explains a lot, but not the black screen yet:
>
> - **KH1's pause machine** has a small state table. While the game runs (state 0) it writes the "which tasks run" mask back to "all" every frame, which is why t371 didn't hold anything. When the pause starts (state 1), it sets the mask to "pause tasks only" and adds the pause's own task, the one that shows P A U S E. So the battle pause holds exactly the same set of tasks as t372 did, and t372 went black.
> - So the paused picture isn't coming from KH1 skipping the right tasks. It comes from how KH1's renderer keeps and re-shows the last finished frame (it has a frame it can show again when nothing new was drawn). I'm reading that part now, but it's deep in the renderer and I don't want to guess at it.
> - Side find for the sound question from before: when KH1 pauses, it also calls its sound code with a "pause" setting (`0x1400DE010`). That's how KH1's pause handles music and sounds, so frame advance can use the same call later.
>
> The quickest way to pin down the black screen is the **Held Tasks** page from t399 (debug menu main page). Turn Frame Advance on, then on Held Tasks switch tasks to `Runs` one at a time, starting from the bottom (139999, 137000, 135000, 130000, 129999, 120000, 110000...), until the picture comes back. Then send me the log: every switch is in it with the list of what was running. Even "none of them brought the picture back" tells me it's the renderer side and not a task.

^m17


> [!note] Tester Mr Bugs
> 130000 0x2a23e0 - model render
> 120000 0x0d2830 - level render
> 120000 0x0d2830 - characters can move when this is run 
> 100000 0xle5d90 - seems to control camera
> 
> sent ini

^m18


> [!tip] Assistant
> t426
>
> That's exactly what I needed, thanks. So the picture needs 130000 (the models) and 120000 (the level), but 120000 also lets characters move.
>
> I looked inside 120000 in the decomp. Besides drawing the map (and the cutscene letterbox bars), it runs two small task lists of its own, one task per animated part of the map. My guess is those are what move.
>
> **So in t426:**
> - 120000 and 130000 now run while the game is held, by default, so the level and the models should draw.
> - 120000's two sub-lists are held instead. They show on the Held Tasks page as "120000 map parts A" and "120000 map parts B", each Held / Runs like the others.
> - 100000 (the camera) stays held, so the camera shouldn't move while held.
>
> **Please check:** with Frame Advance on, is the picture up now, and does anything still move while held (characters, enemies, platforms, water, anything on the map)? If something still moves, try switching the two "map parts" rows, and send the log either way.
>
> **Check:** your Held Tasks test plus the decomp for what's inside 120000. Not checked in game. Assumption: that the map-part lists are what moved.
>
> Your ini came through too, thanks; the settings look fine for this.

^m19


> [!note] Tester Mr Bugs
> also if you didn't make the correction "100000 0xle5d90" the l is actually a 1 my ocr got it wrong so 100000 0x1e5d90 and what I said is the characters moving is wrong, I meant to put in 80000 0x298940
> 
> 55000 0x2937f0 is the command menu, unsure if its the ui
> 
> even with the map map parts running I still don't see effects rendering on held frames, I also see that  every other effect (magics, keyblade swings, etc) aren't rendering so there's probably a connection, for the rest of the values and the ones I provided I want you to look into the c decomp (steam ghidra) and figure out further how everything works, what we need to work out that I know of is: stats changing, ui, effects, lock on, etc... there might be more to freezing the entire game state so look further
> 
> also I have more requests in tester-request.md Can you make sure that after a thread is given its own unique folder, that an agent checks to see if there's a request uploaded in that document and gives its own folder like you've been doing now.
> 
> go ahead

^m20


> [!tip] Assistant
> **t445 to t447.** Thanks, that correction helped. I found why effects disappeared: the effect pause I was using also hides every effect, including magic and Keyblade trails. KH1's battle pause can do that because it doesn't draw the game live. It takes a snapshot of the last frame and shows that while paused.
>
> **t445:** Frame Advance now does exactly what the battle pause does. It holds everything the pause holds (actors, stats, camera, lock-on, command menu, HUD, scripts, map animation), takes a snapshot of the last frame that ran, and shows it sharp, without the pause blur. Effects, trails, the HUD, the lock-on marker and damage numbers should all be in the picture. The Held Tasks page now labels the tasks (actors, camera, command menu, lock-on, map, models…).
> **t446:** sound pauses the way KH1's pause does it (sound effects stop, music at half volume). New row: Held Sound < Paused / Plays >.
> **t447:** a fallback, Held Picture < KH1 Pause / Live >. Live draws the game live like t426, but with effects held by KH1's own developer stop so they stay drawn, Keyblade shape trails held, and the map's sky and fades held.
>
> Checked in the exe's pause code and the decomp. Not tested in game: whether the snapshot actually shows when we take it (it's the same call KH1's pause makes). If held frames are black, that's the reason, and Live is the fallback.
>
> Please test:
> 1. Turn Frame Advance on mid-fight while casting a spell, during a Keyblade swing and with lock-on up. Is the picture up and complete, and does anything move or tick while held (HP/MP bars, damage numbers, water, sky)?
> 2. Step through a combo at 30 and 60 fps: does each step show the new frame correctly?
> 3. Open the battle pause while held, then close it: P A U S E should show, then our picture comes back.
> 4. If held frames are black, set Held Picture to Live and send the log.
> 5. Switch Held Sound between Paused and Plays and tell me how it sounds.
>
> Questions:
> 1. KH1's pause halves the music volume while held. (a) keep that, (b) leave the music alone (needs our own sound handling), or (c) stop the music completely while held?
> 2. If the KH1 Pause picture works, should I (a) keep Live as a debug option, or (b) remove it?
>
> Also: your request that new threads get their own folder is done.

^m21

> [!note] Tester Mr Bugs
>the actual pause seems to hold the image in the frame buffer so I wouldn't be using it, focus on getting the live version working 
>
>also the address you listed as camera 2 is wrong, I noticed that it stops one of the backgrounds. do not take my word for what I said make sure you look through the c decomp steam to confirm what they actually are like you did with the effects. do that for the rest of the values that are undocumented and ones you haven't documented yet.
> 
> also agents working on these threads stop posting my !note bubble 
> 
> go ahead

