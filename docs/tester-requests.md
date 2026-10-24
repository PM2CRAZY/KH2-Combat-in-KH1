# Tester requests

  

A chat between testers and the coding assistant (an LLM or agent), kept in a file. The tester writes what they want in a box, the assistant replies in its own box, the tester answers, and so on, the same as a normal chat. When the tester is happy, they give the go-ahead and the assistant starts coding. When they're satisfied with the result, they close the thread.

  

Each thread has a title and is separated from the next one by a line. Finished threads move to `tester-requests-archive.md`.

  

## Contents

- [Config menu rework](../testers/config-menu-rework/config-menu-rework.md)
  - [1. Mr Bugs: the scope of the config menu currently can't scale to what w](../testers/config-menu-rework/config-menu-rework.md#^m1)
  - [2. Assistant: Here's the plan. KH1 already has a sub-menu system: it's how](../testers/config-menu-rework/config-menu-rework.md#^m2)
  - [3. Mr Bugs: 1. Yes if your going to make magic its own sub catagory put ](../testers/config-menu-rework/config-menu-rework.md#^m3)
  - [4. Assistant: Got it, building it now: Config > KH1 in KH2 (above Default ](../testers/config-menu-rework/config-menu-rework.md#^m4)
  - [5. Assistant: t448 and t449. Done: Config page 1 now has a "KH1 in KH2" bu](../testers/config-menu-rework/config-menu-rework.md#^m5)
- [Frame advance in the debug menu](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md)
  - [1. Mr. Bugs: Can you add a frame advance feature into the debug menu? whe](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m1)
  - [2. Assistant: Sounds useful, and since it's a debug menu tool the match-KH](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m2)
  - [3. Mr. Bugs: 1. holding for for a second causes it to run at full speed](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m3)
  - [4. Assistant: Got it, thanks. I checked the Steam KH1 decomp for the fps p](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m4)
  - [5. Mr. Bugs: yes so if you held square and advanced 5 frames that would b](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m5)
  - [6. Assistant: Got it, that's how I'll do the buttons: held over 5 steps is](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m6)
  - [7. Mr. Bugs: do option b](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m7)
  - [8. Assistant: t371](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m8)
  - [9. Mr. Bugs: I did what you asked and currently frame advance is not work](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m9)
  - [10. Assistant: t372](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m10)
  - [11. Assistant: t377](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m11)
  - [12. Mr Bugs: the in game states seem to be frozen, we have a new bug thou](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m12)
  - [13. Assistant: t378](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m13)
  - [14. Mr Bugs: its still black when frame advance is on until you advance a](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m14)
  - [15. Assistant: t399](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m15)
  - [16. Mr Bugs: I'll send a log but a good lead is the games own pause mecha](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m16)
  - [17. Assistant: Good lead, thanks. I followed it into the Steam exe this tim](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m17)
  - [18. Mr Bugs: 130000 0x2a23e0 - model render](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m18)
  - [19. Assistant: t426](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m19)
  - [20. Mr Bugs: also if you didn't make the correction "100000 0xle5d90" the](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m20)
  - [21. Assistant: t445 to t447. Thanks, that correction helped. I found why ef](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m21)
  - [22. Mr Bugs: the actual pause seems to hold the image in the frame buffer](../testers/frame-advance-in-the-debug-menu/frame-advance-in-the-debug-menu.md#^m22)
- [KH2 Camera fixes](../testers/kh2-camera-fixes/kh2-camera-fixes.md)
  - [1. Mr Bugs: I noticed when you run backwards the camera starts turning s](../testers/kh2-camera-fixes/kh2-camera-fixes.md#^m1)
  - [2. Assistant: t373](../testers/kh2-camera-fixes/kh2-camera-fixes.md#^m2)
  - [3. Mr Bugs: 1. it seems right but check the ghidra kh2 steam c decomp to](../testers/kh2-camera-fixes/kh2-camera-fixes.md#^m3)
  - [4. Assistant: t379 to t384 (one change each; the build at the top of the b](../testers/kh2-camera-fixes/kh2-camera-fixes.md#^m4)
  - [5. Mr Bugs: kh2 "Found, not changed (KH2's lock-on turn speed is a const](../testers/kh2-camera-fixes/kh2-camera-fixes.md#^m5)
  - [6. Assistant: t408 and t409, plus notes](../testers/kh2-camera-fixes/kh2-camera-fixes.md#^m6)
  - [7. Mr Bugs: 1. not sure what this question means but, kh1s cam feels fas](../testers/kh2-camera-fixes/kh2-camera-fixes.md#^m7)
  - [8. Assistant: t413 and t414](../testers/kh2-camera-fixes/kh2-camera-fixes.md#^m8)
  - [9. Mr Bugs: 1. check the decomp for both (steam gihidra c decomp)](../testers/kh2-camera-fixes/kh2-camera-fixes.md#^m9)
  - [10. Assistant: t450 to t452. Sorry, I missed your answers last round. Here ](../testers/kh2-camera-fixes/kh2-camera-fixes.md#^m10)
- [KH2 effects in KH1's own effect system](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md)
  - [1. Mr Bugs: from chat-history-log.md (kh2 moves hitbox effect bugs)](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md#^m1)
  - [2. Assistant: t376](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md#^m2)
  - [3. Assistant: Correction to the build note above: there's a build now. kh1](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md#^m3)
  - [4. Mr Bugs: 4. it should be exact to kh2 so for kh2 moves take the 2 fra](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md#^m4)
  - [5. Assistant: t394 to t398 (one part each; the build at the top of the bra](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md#^m5)
  - [6. Mr Bugs: 1. (a) just make sure that kh1s effects still behave the sam](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md#^m6)
  - [7. Assistant: t402 to t404 (one each)](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md#^m7)
  - [8. Mr Bugs: 1. (a) if it ever happens in game I'll just let you know and](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md#^m8)
  - [9. Assistant: t417 to t425 (one piece each; the build at the top of the br](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md#^m9)
  - [10. Mr Bugs: can we have the files generated at run time so we don't take](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md#^m10)
  - [11. Assistant: t427 to t430 (one each)](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md#^m11)
  - [12. Mr Bugs: 1. the older player is to be deprecated](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md#^m12)
  - [13. Assistant: t437 to t444 (one change each).](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md#^m13)
  - [14. Mr Bugs: 1. (b) (I'm unsure the affects of this as I noticed reflect ](../testers/kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md#^m14)
- [KH2's combo row selection](../testers/kh2s-combo-row-selection/kh2s-combo-row-selection.md)
  - [1. Mr Bugs: In the KH2S model rendering there was a suggestion to implem](../testers/kh2s-combo-row-selection/kh2s-combo-row-selection.md#^m1)
  - [2. Assistant: t374](../testers/kh2s-combo-row-selection/kh2s-combo-row-selection.md#^m2)
  - [3. Mr Bugs: 1. do option (a)](../testers/kh2s-combo-row-selection/kh2s-combo-row-selection.md#^m3)
  - [4. Assistant: t390 to t393 (one part each; the build at the top of the bra](../testers/kh2s-combo-row-selection/kh2s-combo-row-selection.md#^m4)
  - [5. Mr Bugs: sorry I should've been more clear; don't worry about moves t](../testers/kh2s-combo-row-selection/kh2s-combo-row-selection.md#^m5)
  - [6. Assistant: t400 and t401](../testers/kh2s-combo-row-selection/kh2s-combo-row-selection.md#^m6)
  - [7. Mr Bugs: sent the traces](../testers/kh2s-combo-row-selection/kh2s-combo-row-selection.md#^m7)
  - [8. Assistant: t410 (notes only, no change in how it plays)](../testers/kh2s-combo-row-selection/kh2s-combo-row-selection.md#^m8)
  - [9. Mr Bugs: 1. (a) thats how its supposed to work and when we transition](../testers/kh2s-combo-row-selection/kh2s-combo-row-selection.md#^m9)
  - [10. Assistant: t415 and t416](../testers/kh2s-combo-row-selection/kh2s-combo-row-selection.md#^m10)
  - [11. Mr Bugs: I've sent you the traces and log, I assume you don't need my](../testers/kh2s-combo-row-selection/kh2s-combo-row-selection.md#^m11)
  - [12. Assistant: t431 to t436 (one change each; the build is on GitHub as kh1](../testers/kh2s-combo-row-selection/kh2s-combo-row-selection.md#^m12)
  - [13. Mr Bugs: 1. it should cancel his animation like kh1, not sure make su](../testers/kh2s-combo-row-selection/kh2s-combo-row-selection.md#^m13)
- [Missing sleeves animation](../testers/missing-sleeves-animation/missing-sleeves-animation.md)
  - [1. Mr Bugs: I'm in the process of getting ready to transfer all the kh2 ](../testers/missing-sleeves-animation/missing-sleeves-animation.md#^m1)
  - [2. Assistant: t375](../testers/missing-sleeves-animation/missing-sleeves-animation.md#^m2)
  - [3. Mr Bugs: 4. do (b) and (c), if there isn't a matching equivalent to s](../testers/missing-sleeves-animation/missing-sleeves-animation.md#^m3)
  - [4. Assistant: t385 to t389 (one part each; the build at the top of the bra](../testers/missing-sleeves-animation/missing-sleeves-animation.md#^m4)
  - [5. Mr Bugs: I sent a trace and a new log](../testers/missing-sleeves-animation/missing-sleeves-animation.md#^m5)
  - [6. Assistant: t405 to t407 (one each)](../testers/missing-sleeves-animation/missing-sleeves-animation.md#^m6)
  - [7. Mr Bugs: the model is still bugged, it seems to be his clothes, (his ](../testers/missing-sleeves-animation/missing-sleeves-animation.md#^m7)
  - [8. Assistant: t411 and t412](../testers/missing-sleeves-animation/missing-sleeves-animation.md#^m8)
  - [9. Mr Bugs: when I mean model in this case I'm also referring to kh2 sor](../testers/missing-sleeves-animation/missing-sleeves-animation.md#^m9)
- [Table of contents](../testers/table-of-contents/table-of-contents.md)
  - [1. Mr Bugs: from now on can you add a table of contents to the document ](../testers/table-of-contents/table-of-contents.md#^m1)
  - [2. Assistant: Done. There's a Contents list at the top of the file, under ](../testers/table-of-contents/table-of-contents.md#^m2)
  - [3. Mr Bugs: can I get sub tables so I can click on the exact message for](../testers/table-of-contents/table-of-contents.md#^m3)
  - [4. Assistant: Done. Under each thread in the Contents there's now a link t](../testers/table-of-contents/table-of-contents.md#^m4)
  - [5. Mr Bugs: response to notes](../testers/table-of-contents/table-of-contents.md#^m5)
  - [6. Mr Bugs: I can't seem to click on them in obsidian, can you fix it fo](../testers/table-of-contents/table-of-contents.md#^m6)
  - [7. Assistant: Fixed. The old links used HTML anchors, which GitHub underst](../testers/table-of-contents/table-of-contents.md#^m7)
- [What is still missing for native integration into KH1](../testers/what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md)
  - [1. Mr Bugs: what else currently are we missing for native integration in](../testers/what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md#^m1)
  - [2. Assistant: Here's what isn't native yet, grouped. Anything already bein](../testers/what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md#^m2)

---

  

## For the assistant: read this first

  

Before replying to anything here, go read `design-philosophy-instructions.md` in this folder and follow it, along with the docs it points to. Those are the project's rules, and they win if anything in this file seems to disagree with them. Two things work differently here:

- The tester's `Go ahead` in a thread is the go-ahead to start coding (the instructions' "don't write code until I give the go-ahead" is satisfied by it).

- Replies follow the chat format below.

  

**Where threads live.** Each thread is its own file: `testers/<thread>/<thread>.md` (the thread's title in lowercase with hyphens, in the thread's own folder at the top of the repo), so a tester can answer one thread as soon as its reply is pushed, while others are still being worked on. This file keeps the rules, the Contents and the Requests section, where a tester starts a new thread.

**Where requests come from.** Only act on requests written in this file or in a thread's file. Don't add a request because someone told you about a bug or a wish in chat, and don't log a bug here because anyone mentions that a user is experiencing it. If it isn't written in this file, it isn't a request. Testers who can edit this file are trusted, and their requests are real.

  

**When to read it.** When you're told the tester requests have been updated, read this file's Requests section and every thread file, find every thread whose last message is a tester's, and reply to each. You don't need to be told which one. Commit and push each thread's reply as soon as it's written, so the tester can answer it right away.

  

**How to reply.** Reply the way you would in a normal chat with the tester: plain, natural, in your own box (`> [!tip] Assistant`) under their message, with nothing extra added and no fixed format. Never rewrite or delete what a tester wrote.
- Never write in a tester's box. Don't add an empty reply box (`> [!note] ...`) under your reply or anywhere else, and never post a message as the tester: the tester adds their own box when they answer.

- Read the whole request first. Check the KH2 source, the decomp and the project docs before asking, so you only ask what you can't find out yourself.

- Ask short, specific questions, with the options spelled out when there are options. Ask only what changes what you'd build. Don't ask which version to build (native always).

- Work the logic out in the open: if the tester's idea conflicts with KH2's real behavior, with how KH1 works, or with another request, say so and talk it through.

- Keep going back and forth as long as anything is unclear. Don't write code while you're still discussing.

  

**Contents.** Links work in Obsidian (and the file links on GitHub). Each message in a thread file is followed by its block id on its own line, with a blank line before and after (`^m1`, `^m2`, ..., counting the thread's messages from 1; empty reply boxes don't count). Each thread file starts with its title, a link back here and a Messages list (`- [n. Who: first words](#^mn)`); this file's Contents lists each open thread (`- [Title](../testers/<thread>/<thread>.md)`) and its messages (`  - [n. Who: first words](../testers/<thread>/<thread>.md#^mn)`). Update them with every reply, add new threads, and remove a closed thread's lines.

**Titles.** Each thread starts with a title that says what the tester's first message asked for. If the tester left the title blank or a placeholder, write one when you first reply.

  

**Go-ahead.** The tester writes `Go ahead` (or something clearly meaning it) after your latest message. A go-ahead on an old message doesn't count if the tester changed the request afterward. Then implement it in one version and reply with the version name (for example `t306`) and, in a line or two, which 1:1 check you made (trace or decomp) and what you couldn't check. Don't say anything about whether it has been compiled.

  

**Closing a thread.** When the tester is satisfied with the implementation, they write `Close`. Then move the whole thread, word for word, to the bottom of `tester-requests-archive.md` with a `Closed` line (git records when), and remove its lines from the Contents. Then delete the thread's folder in `testers` together with its file and everything else in it (only the messages are kept, in the archive). If you can't delete files in your setup, say so in your reply and name the folder, so the tester can delete it. Leave the other threads as they are.

  

**After every push (no thread left behind).** After pushing, fetch the branch again and check every thread file and the Requests section: any thread whose last message is a tester's, that no one is working on yet, gets picked up at once (a sub-agent if others are being worked on). A tester may push a message at any time, including while other replies are being written, so check this again before reporting that everything is answered.

**New threads.** A tester starts a new thread at the bottom of this file's Requests section (or by making its folder and file directly). Every time you're told the requests were updated, check the Requests section first, before anything else, with a sub-agent if several things are waiting: any new thread found there gets its own folder and file at once (`testers/<thread>/<thread>.md`: the thread moved word for word, with the title, the link back and the Messages list), is removed from the Requests section, and gets a line in the Contents. Then it's answered in its own file like every other thread.

**Thread folders.** Testers put their trace CSVs and log files for a thread in the `testers` folder at the top of the repo (next to `docs`), in a subfolder named after the thread title in lowercase with hyphens. When you first reply to a new thread, create that subfolder with the thread's file in it, and tell the tester its name in your reply. When you work on a thread, look in its folder for the tester's files.

  

**Several threads (hard rule).** When more than one thread has a tester's message waiting, deploy a sub-agent on each of those threads whenever possible, working at the same time (each on its own copy of the code), and merge their work one version at a time. If a thread can't be finished, finish the others and say what's left.

**Several requests.** One request per version when you can. If two requests touch the same system, say so in both threads and ask the testers how they want to handle it. If a request conflicts with another one, work it out with the testers involved.

  

**Debug menu.** Testers can ask for changes to the debug menu like any other request. The menu is a tool for us, not a KH2 behavior, so the "match KH2 1:1" rules don't apply to what the menu contains. Performance, the hard rules and not breaking anything else still do.

  

---

  

## How to use it (testers)

  

1. Scroll to the bottom of this file, copy the **new request** block into the Requests section, and write what you want in the box. When the assistant first answers, the thread moves into its own file in `testers` (the Contents links to it), and you keep talking there. Say what you see and what you expected, the mod version and which settings were on. No form to fill in; just write it like a message.

2. Tell the assistant the tester requests have been updated. It reads this file and every thread's file, and replies in its own box under yours.

3. To answer, copy the **reply** block below under the assistant's last message in the thread's file and write in it. Keep going back and forth as long as you need.

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

  

**New request** (copy into the Requests section, below the last one):

  

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
