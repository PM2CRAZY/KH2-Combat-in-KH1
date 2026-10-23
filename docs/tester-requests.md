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
> 
