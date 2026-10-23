# Tester requests

  

A chat between testers and the coding assistant (an LLM or agent), kept in a file. The tester writes what they want in a box, the assistant replies in its own box, the tester answers, and so on, the same as a normal chat. When the tester is happy, they give the go-ahead and the assistant starts coding. When they're satisfied with the result, they close the thread.

  

Each thread has a title and is separated from the next one by a line. Finished threads move to `tester-requests-archive.md`.

  

---

  

## For the assistant: read this first

  

Before replying to anything here, go read `design-philosophy-instructions.md` in this folder and follow it, along with the docs it points to. Those are the project's rules, and they win if anything in this file seems to disagree with them. Two things work differently here:

- The tester's `Go ahead` in a thread is the go-ahead to start coding (the instructions' "don't write code until I give the go-ahead" is satisfied by it).

- Replies follow the chat format below.

  

**Where requests come from.** Only act on requests written in this file. Don't add a request because someone told you about a bug or a wish in chat, and don't log a bug here because anyone mentions that a user is experiencing it. If it isn't written in this file, it isn't a request. Testers who can edit this file are trusted, and their requests are real.

  

**When to read it.** When you're told the tester requests have been updated, read the file, find every thread whose last message is a tester's, and reply to each. You don't need to be told which one.

  

**How to reply.** Reply the way you would in a normal chat with the tester: plain, natural, in your own box (`> [!tip] Assistant`) under their message, with nothing extra added and no fixed format. Never rewrite or delete what a tester wrote.
- After every reply you write, add an empty reply box right under it, with the name of the tester you're talking to already filled in (`> [!note] Tester name`, then a line with just `> `), so they can type their answer without copying a block. If several testers are in the thread, use the name of whoever wrote last. An empty box isn't a message: ignore it when you read the file, and don't add a second one if it's already there.

- Read the whole request first. Check the KH2 source, the decomp and the project docs before asking, so you only ask what you can't find out yourself.

- Ask short, specific questions, with the options spelled out when there are options. Ask only what changes what you'd build. Don't ask which version to build (native always).

- Work the logic out in the open: if the tester's idea conflicts with KH2's real behavior, with how KH1 works, or with another request, say so and talk it through.

- Keep going back and forth as long as anything is unclear. Don't write code while you're still discussing.

  

**Titles.** Each thread starts with a title that says what the tester's first message asked for. If the tester left the title blank or a placeholder, write one when you first reply.

  

**Go-ahead.** The tester writes `Go ahead` (or something clearly meaning it) after your latest message. A go-ahead on an old message doesn't count if the tester changed the request afterward. Then implement it in one version and reply with the version name (for example `t306`) and, in a line or two, which 1:1 check you made (trace or decomp) and what you couldn't check. Don't say anything about whether it has been compiled.

  

**Closing a thread.** When the tester is satisfied with the implementation, they write `Close`. Then move the whole thread, word for word, to the bottom of `tester-requests-archive.md` with a `Closed` line (git records when), and remove it from this file along with its separator line. Then delete the thread's folder in `testers` together with the files in it (only the messages are kept, in the archive). If you can't delete files in your setup, say so in your reply and name the folder, so the tester can delete it. Leave the other threads as they are.

  

**New threads.** A tester can start a new thread at any time at the bottom of this file, below the last thread, including right after another one was closed.

**Thread folders.** Testers put their trace CSVs and log files for a thread in the `testers` folder at the top of the repo (next to `docs`), in a subfolder named after the thread title in lowercase with hyphens. When you first reply to a new thread, create that subfolder with a short `README.md` in it (git doesn't track empty folders), and tell the tester its name in your reply. When you work on a thread, look in its folder for the tester's files.

  

**Several requests.** One request per version when you can. If two requests touch the same system, say so in both threads and ask the testers how they want to handle it. If a request conflicts with another one, work it out with the testers involved.

  

**Debug menu.** Testers can ask for changes to the debug menu like any other request. The menu is a tool for us, not a KH2 behavior, so the "match KH2 1:1" rules don't apply to what the menu contains. Performance, the hard rules and not breaking anything else still do.

  

---

  

## How to use it (testers)

  

1. Scroll to the bottom, copy the **new request** block below the last thread, and write what you want in the box. Say what you see and what you expected, the mod version and which settings were on. No form to fill in; just write it like a message.

2. Tell the assistant the tester requests have been updated. It reads the file and replies in its own box under yours.

3. Type your reply in the empty box the assistant leaves under its message. If there's no empty box, copy the **reply** block below. Keep going back and forth as long as you need.

4. When you're happy with the plan, write `Go ahead` in a reply. That is the approval, and no one else needs to approve. The assistant then starts coding.

5. When you're satisfied with the result, write `Close`. The assistant moves the thread to the archive file.

  

Each message is a callout box (the lines starting with `> [!`). In an editor without callouts it still reads as plain quoted text.

  

Tips:

- One change or addition per request. Split big wishes into several.

- You don't need to know how KH1 or KH2 work inside. Describe what happens.

- If you have a trace (the Triangle + Select CSV) or a log file, put it in the thread's folder inside `testers` (the assistant creates the folder when it first replies) and name the file in your message.

- Keep decompiled code and game files out of this file. Describe what you saw in plain language.

  

---

  

## Blocks to copy

  

The grey boxes below are code blocks, so you can copy the text exactly as written. Don't copy the three backticks (```) at the top and bottom of each box into your request.

  

**New request** (copy below the last thread):

  

```

---

  

## Title (the assistant fills this in if you leave it)

  

> [!note] Tester (your name)

> Write what you want here.

```

  

**Reply** (copy under the last message in the thread; the assistant uses the same block with `> [!tip] Assistant`):

  

```

> [!note] Tester (your name)

> Your reply here.

```

  

---

  

## Requests

  

(Add new threads below this line.)

---

  

## Frame advance in the debug menu

  

> [!note] Tester Mr. Bugs

> Can you add a frame advance feature into the debug menu? when you enable it you can frame advance by 1 frame by pressing R2 or pressing the period key on the keyboard, if you hold down the frame advance button then the game plays at normal speed.

> [!tip] Assistant
> Sounds useful, and since it's a debug menu tool the match-KH2 rules don't apply to it. My plan is a Frame Advance row in the debug menu. Turning it on freezes the game (the menu, HUD and picture still draw), each tap of R2 or the period key plays one game update, and holding it plays at normal speed until you let go. Music and sounds would freeze with the game. A few things I need from you first:
>
> 1. **How long a hold before it runs at normal speed?** (a) about a third of a second, like the menu's held-direction repeat, so a quick tap is always exactly one frame; (b) right away, so holding for 3 frames plays 3 frames; (c) something else.
> 2. **What counts as one frame?** KH1 updates once per drawn frame, and at the 30 fps setting each update is two 60 Hz steps. (a) one game update, whatever your fps (at 30 fps a tap moves 2/60 s); (b) always one 60 Hz step, which at 30 fps means drawing every step. (a) is simpler and changes nothing else. If you play at 60 fps they're the same.
> 3. **R2 is already used.** The mod's KH2 lock-on uses R2 + right stick to switch targets, and KH1 reads R2 too. While frame advance is on, should R2 (a) be only frame advance (hidden from the game, so no target switching), or (b) still reach the game as well? Or (c) use a different button on the controller.
> 4. **Buttons pressed while frozen.** Held buttons go into the next frame you step, so you can hold Square and step through a swing. If you tap a button between steps and let go before stepping, should the next frame (a) still see that press, or (b) only see what's held when you step?
>
> The period key only counts while the game window is focused. Your thread's folder is `testers/frame-advance-in-the-debug-menu` if you have traces or logs to add.

> [!note] Tester Mr. Bugs
> 1. holding for for a second causes it to run at full speed
> 
> 2. it needs to work no matter the fps setting, if your unsure on how that works check the C decomp for the steam version of kingdom hearts 1
> 3. having it set on R2 should disable any inputs R2 usually takes 
> 4. if I have a button held then I advance to the next frame it will count as a button press, if I let go then advance to the next frame then its not a button press
>    
>    also trace recordings need to advance frame by frame as well, so if I have trace set to 3 seconds that timer only advances frame by frame. please let me know how you would implement it.

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

> [!note] Tester Mr. Bugs
> do option b
> Go ahead

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


---

  

## Chat history

  

> [!note] Tester Mr Bugs
> 
>I've logged all my past sessions of looking through the source code with claude, please look through the tester history to be up to date with everything I was working on 
>
>at chat-history-log.md
>
>go ahead


---

  

## Title (the assistant fills this in if you leave it)

  

> [!note] Tester Mr Bugs

> from chat-history-log.md (kh2 moves hitbox effect bugs)
> "# Porting KH2's effects into KH1's effect system
>Goal (systems-to-port: Effects): any move can play vanilla KH1 or KH2 effects, with KH2's effects running inside KH1's own effect system rather than beside it. Chosen: ladder step 1 (native). First target: Explosion. All addresses Steam. Tags: **Checked (exe)** = read in the Steam exe / Ghidra C export; **Checked (files)** = read in the user's OpenKH extraction; **Assumption** = not yet.
>## Decisions
>- 2026-10-22: ladder step 1 (native), Explosion first.
>- 2026-10-22: **bones: option A**. KH2 elements name KH2 skeleton bones; they attach to the matching **KH1** bone through KH1's own bone binding (the effect follows the Sora on screen, as the base attacks now do: KH2's motions on KH1's skeleton). Not KH2's own pose (option B, a parallel skeleton).
> [!NOTE]
> 
> ## >Where we were (ladder step 3)
> 
> `kh1-hook/src/win/kh2fx.rs` + the `kh2-fx` crate + `fx12.rs`: a full second effect player (KH2's particle machine in Rust, its own clock, instances, bones and D3D12 renderer, drawn after KH1's world-model pass, hook `0x1EADD0`).
> 
> ## Check results (2026-10-22)
> 
> ### Same engine: DPX -> DPD -> "ppp" particle programs (Checked (exe), Checked (files))
> 
> - KH1's effect files are **DPX** (strings `worldmap/eft/map_event.dpx`, `always.dpx`, `prize.dpx`, ...; loaded and relocated by `0x1401E0080`), the same format as KH2's DPX: `version | 0 | 0 | count`, 32-byte entries (DPD offset, particle index, effect number, kind mask, group mask, ...). Same version: `0x82` in both games' files. Both games' DPDs start with `0x96`. KH1 has **no PAX layer**: what KH2's PAX element does is done in KH1 by the code that starts an effect (below).
> - DPD particle data relocation: KH1 `0x14031C2F0` and KH2 `0x1404B06D0` decompile **identically**.
> - DPD texture creation (PC port, `TexCommon`): KH1 `0x14031EC10` vs KH2 `0x1404AECD0`, 90% the same; KH2 adds one extra texture case (`0xFFFD`). Helper KH1 `0x14031E090` = KH2 `0x1404AE0A0`, identical.
> - Program tables: KH1 `0x14054FF00`, KH2 `0x140779A00`, 0x50 per entry (name, calc +0x08, draw +0x18, hooks +0x20/0x28/0x30, construct +0x38, reset +0x40, destruct +0x48). Entries 1..235 same names, same indices. KH2-only: 236 VertexApLcLoop, 237 SnoScaleAll, 238 SnoMdlDttNorm, 239/240/242 RyjMegaBirthUserCtrl(1-3), 241 EiDrawShipoly.
> - **Per-program code comparison** (normalised decompiles, every slot of 1..235): **120 identical**, 115 differ. Most differences are trivial (direct call vs thunk, random-number helper, or only the offset of a field in the effect instance, whose layout differs between the games: KH1 `+0xD5` vs KH2 `+0xC1` for the same flag). Real ones:
>     - pppColor (43): KH2 clamps colour to 32767.
>     - pppRyjMegaBirth (201): KH2's draw is rewritten (particle records 0x18 -> 0x30 bytes, new matrix setup, depth handling, a third matrix mode `+0xCC` 1..3 vs KH1 1..2); calc 77%, draw 61% similar.
>     - pppRyjMatrixWorld* (98-104) construct 57%; pppAngleLoop (214) 38%; pppRyjMegaBirthPrize (203) 19%; pppKeLns* (189-193, 221-225): KH2 adds a calc slot. Not used by Explosion.
> 
> ### Clock (Checked (exe))
> 
> KH1's effect update `0x1401ECB50` steps each effect instance once per 2.0 of frame delta (accumulator at instance `+0xBC`, frame delta `DAT_14233FBDC`): 30 Hz, the same as KH2 (`docs/kh2-effects.md` §1). No frame conversion needed.
> 
> ### Element layer: what starts and places an effect (Checked (exe))
> 
> - Effect instances: pool `0x142678280`, 0x100 x 0xE0 (claimed by `0x1401EC690`, started from a DPX by number with `0x1401E0100 (dpx, element, number, priority)`, stopped by `0x1401EC470`).
> - KH1's "element" (0x3C each, 256 at `0x142D678B0`, allocator `0x1402AE5B0`): holds handles to the instance's root matrix, colour and flags; start at a position `0x1402AE650`, on an actor's bone `0x1402ADCF0` (a per-frame binding through the task list `0x142DB34B0`, callback `0x1402AE910`). The particle header's sound (`+0xFC`) is played by KH1 too.
> - KH2 PAX element features with **no KH1 counterpart found**: start delay, fade-out frames, "ends with the motion" flag (0x20), anchors 0x4001 body centre / 0x4003+ weapon points. These need a small element layer of ours that drives KH1's own instances (start, stop, root matrix, colour), not a renderer.
> 
> ### Textures: the main risk (Checked (exe), Checked (files))
> 
> - Both games have their own resident `texcommon.dpd`, with different contents.
> - Texture addressing: KH1 `0x1401EBC60` vs KH2 `0x1402C1800` (same routine, 66% similar), different VRAM layouts:
>     - KH1 common: TBP `0x3600-0x37FF`, CBP `0x3D00-0x3D3F`; KH1 own textures from TBP `0x3800` / CBP `0x3D40`.
>     - KH2 common: TBP `0x29xx-0x2Exx`, CBP `0x2Exx`; KH2 own from TBP `0x3000` / CBP `0x3E00`; KH2 moves addresses by `-0x200` / `-0x400` (TBP) and `+0x1E000` / `-0x1000` (CBP) at load.
> - So a KH2 DPD can't be handed over as is: both its own textures and the KH2 common ones fall in KH1's _common_ window. Their addresses must be moved into KH1's own window, and the KH2 common textures it uses copied into the DPD as its own (shapes' and models' TEX0 rewritten to match).
> 
> ### Handles (Checked (exe))
> 
> KH1's pointer-to-handle `0x14038AD90` registers a new 32 MB region in the 64-entry handle table when needed, so KH2 DPD/DPX buffers we allocate can be reached through KH1 handles (`pose::handle` / `pose::ptr`).
> 
> ## Explosion, from the files (Checked (files))
> 
> Files: OpenKH extraction `new openkh\data\kh2\obj\P_EX100.a.us`, `kh2\effect\texcommon.dpd`, `kh1\texcommon.dpd` (16-byte header, then the DPD), `kh1\always.dpx`.
> 
> - Explosion = PAX elements 44, 45, 46, 47, 48 -> effects 31, 32, 33, 33, 34 -> particle datas 11, 12, 13, 13, 14 of the PAX's 4th DPD (8 textures, 23 shapes, 39 models).
> - **Programs used (26):** 19 identical in KH1 (SclAccele, ColAccele, AngAccele, SclMove, ColMove, AngMove, Point, Angle, Scale, MatrixScl, ParMatrix, SMatrix, DrawMatrix, DrawMatrixFront, DrawMdl3, DrawMdlTs3, DrawShapeX, KeZCrct, KeZCrctShp). MatrixXYZ / MatrixZXY, RandIV, VertexApLc: equivalent. Color: same + KH2's clamp. **pppRyjMegaBirth: KH2's version needed** (elements 44, 45, 48). **pppEiDrawShipoly: KH2-only** (element 48).
> - Elements 46/47 use only KH1-equivalent programs: the first native test.
> - Bones (KH2 skeleton numbers): 44 weapon point 0x4003, 45 bone 0x43 (67), 46 0xAA (170), 47 0x4E (78), 48 object root 0x4000 (fade-out 8 frames, flags 0x243). 44-47 flags 0xA61.
> - Textures: shapes use KH2 common `0x2A00/0x2E34` (256x448 atlas) and `0x2C40/0x2E00`, and own `0x3060/0x3DEC`.
> 
> ### Bone mapping (option A) (Checked in our own retarget data, `kh1-data/src/retarget.rs`)
> 
> `retarget` is part-based, not a bone-to-bone table: KH2 sides +x / -x (+92 on the arms): clavicle 74/166, shoulder 76/168, elbow 79/171, wrist 80/172; KH1 per side clavicle 194/89, upper-arm bones +2..+4, elbow +5, forearm +6..+8, hand +9..+12. So KH2 **78** and **170** are upper-arm bones (between shoulder 76/168 and elbow 79/171) -> KH1's upper arm (196..198 on +x, 91..93 on -x); KH2 **67** lies on the chest/neck line (`Pole::Line(67, 137)`). Each element's offset and rotation are given in KH2's bone frame; to keep it, convert through both bind poses: local on KH1 = inverse(KH1 bone bind) x KH2 bone bind x KH2 local. The exact KH1 bone per KH2 bone (by bind position) is picked from the two bind poses when the loader is written. Weapon point 0x4003 -> the Keyblade's tip.
> 
> ## Plan for Explosion (step 1, native), one system per version
> 
> 1. **Done, t368** (below). Loader: Explosion's DPD and particle datas into a KH1 DPX we build, textures moved to KH1's own window with the KH2 common ones copied in; KH1's own code relocates and textures it (`0x1401E0080`, `0x14031EC10`). Test: elements 46/47 started on Sora's KH1 bones (option A) through KH1's element calls.
> 2. Element layer: start delay, fade-out, end-with-motion, the weapon-point and object-root anchors (element 48).
> 3. KH2's pppRyjMegaBirth as a plugged-in program (KH1 instance layout, drawing through KH1's own shape draw): elements 44, 45.
> 4. pppEiDrawShipoly (KH2-only) as a plugged-in program: element 48's ribbon.
> 5. Retire the parallel player for Explosion.
> 
> ## Step 1 done: t368 (2026-10-22)
> 
> - `kh2_formats::kh1dpx::build(pax, texcommon, effects)` (pure byte mover): a KH1 DPX of some effects of one KH2 DPD. Keeps only their particle datas (renumbered), packs the textures they draw with (DPD's own + KH2 common) from TBP `0x3800` / CBP `0x3D40` (TBP spans page-aligned, textures sharing a TBP get the largest one's space), rewrites shapes' and models' TEX0, rebuilds each particle's texture mask, sets programs > 235 to 1 (dummy). Explosion (31..34): 4 particle datas, 12 textures, vram `0x3800..0x3AC0` / `0x3D40..0x3D70` = **6 of KH1's 10 TBP slots, 2 of 6 CBP**. Offline check: every TEX0 a kept particle draws has its texture, and it's in that particle's mask.
> - Load path (Checked (exe)): `0x1401E0080` turns each entry's DPD offset into a handle and relocates each DPD's particle datas (`0x1401ECA40` -> `0x14031C2F0` with the program table); call once per DPX. Start on a bone `0x1402ADCF0 (dpx, number, actor, bone)` -> element `0x1402AE5B0` (number `& 0x7FF`, priority 20000) -> `0x1401E0100`; number bit `0x800` = end with the motion (binding `+6` = -1, then the motion at start).
> - Handles: a region is 32 MB (`ptr & ~0x1FFFFFF`), 64 regions; the DPX buffer is placed inside one region.
> - `kh2dpx.rs`: builds on a background thread; `kh2fx::event` hands elements 46/47 to it (effect 33 on KH1 bones 93 / 198, `0x2000 | bone`); falls back to `kh2fx` if not built or KH1 has no free element. Ini `kh2_fx_native`.
> - Known gaps (step 2): rotation offsets (half turn about z) not applied by KH1's bone binding; delay / fade-out / end-with-motion not applied yet (Explosion 46/47 are 10 ticks long and end on their own).
> - **Assumption, to check in game**: KH1's draw-time texture slots (`0x1401EDB00`) give an instance needing 6 slots a contiguous run; if other effects hold slots the flash may draw with the wrong textures or not at all.
> 
> ## t368 test result: untextured squares (2026-10-22, Checked (exe))
> 
> - User: Explosion's flashes show as squares at Sora's body; the rest fine. Log: DPX built, relocated, both elements started on KH1 elements. So placement and programs run; textures don't.
> - Cause: KH1's PC port never decodes PS2 texture data. Texture creation for a DPD (`0x14031EC10`, one texture object per TBP; a record's header is 0x20 bytes: TBP, CBP, bw, psm, x, y, w, h, then its own TEX0 at `+0x10` and a size word at `+0x18`) hands the record's data pointer to the remaster texture lookup `0x14010C050`. That finds the loaded _file_ the pointer lies in (`0x1400E2B20`: file list `0x1421AB020`, each file's base `+0x658`, size `+0x640`), turns the offset into an index (`+0x21C` table) and takes that file's HD texture entry (`+0x670` list, matched by `+0x54`; created by `0x14010BAF0` / `0x14010B1B0` as `Axa::EntryTex`). Our buffer isn't a loaded file, so it returns no texture and the sprites draw untextured.
> - KH2 PC works the same way: its effect textures are HD DDS images stored with each file (`remastered/obj/P_EX100.a.us/-0..-8.dds`, `kh2_formats::egs` can read them from the install).
> - Also found: the builder doesn't rewrite a texture record's own TEX0 (`+0x10`); KH1's records keep it equal to `+0`/`+2`. To fix with whichever option is chosen.
> - Options (to decide with the user): A) register the DPX with KH1 as a loaded file carrying KH2's HD textures, so KH1's own loader makes them (native); B) hook the texture lookup for our buffer and make the texture objects ourselves.
> 
> ## t369: option A, HD textures through KH1's loaded-file list (2026-10-22)
> 
> - Decision (user): option A, native.
> - KH1's resource entry (made by `0x1400E1680`, listed by `0x1400E2F60`; 0x688 bytes): `+0x10` data, `+0x18` size, `+0x20` / `+0x1C4` / `+0x620` name, `+0x208` MD5 of the data, `+0x21C` offset table, `+0x640` size, `+0x64C` -1, `+0x658` data, `+0x668` list vtable, `+0x670` / `+0x678` / `+0x680` HD-entry list head / tail / count. List object (global read through `0x1400E2B20`'s first instruction): `+8` head, `+0x10` tail, `+0x18` count.
> - HD entry (`Axa::ReplaceEntry`, 0x78 bytes): `+8` next, `+0x10` name, `+0x50` size, `+0x54` key, `+0x60` data (`_aligned_malloc`; handed to the texture, which frees it), `+0x68` 999, `+0x70` the texture once made. Key: offset of the record's pixels in the file while `0x142F43000` is set (it is, from `axaAppMain`), else an index into `+0x21C`.
> - Texture lifetime (`0x14010CDA0`): a texture is dropped only when its file is gone (found by MD5) or the file is removed (`0x1400E2E00`); ours is never removed.
> - KH2's HD assets: one per TBP (texcommon: 16 records, 12 TBPs, 12 assets), DXT5 DDS like KH1's.
> - t369 used asset header `+0x20`: wrong (it is the asset's own place in the stream; t369 log: 0 HD textures). t370: `+0x24`, **Checked (exe)**: KH1's own loader reads `+0x24` as the HD entry key (`0x1400D53F0`, low 24 bits).
> - 2026-10-22: t370 confirmed in game by the user: Explosion's wrist flashes draw with their KH2 HD textures through KH1's own effect system. Step 1 done."
>   
>   
>   It was currently going to implement a hybrid layer on top of kh1, I want you to go into kh1 steam gihidra c decomp and find a way to implement the missing features from kh2 into kh1 natively then give me a build so I can test it 
>   
>   
>   

---

  

## Title (the assistant fills this in if you leave it)

  

> [!note] Tester Mr Bugs
> In the KH2S model rendering there was a suggestion to implement "**KH2's row selection**, so the right swing plays for the target's distance. Today the swing order is fixed, which is the biggest feel difference."
> 
> I want you to start working on it to get it native in kh1 using the steam gihidra c decomp of kh1, if you need to compare with kh2's decomp (rust rewrite) do so, if not fallback to the gihidra c decomp of kh2's steam exe 
> 


---

  

## Missing sleeves animation

  

> [!note] Mr Bugs
> I'm in the process of getting ready to transfer all the kh2 animations into kh1s internals and I noticed on kh2's animations they're missing animations for the sleeves can you double check and add them to our animations? I've logged sora running in kh1 and sora running in kh2, if there's any other missing parts to an animation let me know after you've implemented the changes (logs and traces are in Missing sleeves animation)


---

  

## KH2 Camera fixes

  

> [!note] Tester Mr Bugs
> I noticed when you run backwards the camera starts turning sora in a circle and rotates this is not indented, can you please look through the kh2 decomp to verify and fix this change then if you need the kh2 ghidra c decomp steam version to find any other missing features do it. I've logged a trace for you to see whats going on in game
> 
> go ahead

---

  

## Table of contents

  

> [!note] Tester Mr Bugs
> from now on can you add a table of contents to the document  so I can quickly access each thread
> 
> go ahead



