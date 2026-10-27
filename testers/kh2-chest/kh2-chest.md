# KH2 Chest

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: I want to add a config in the tweaks menu that lets use quic](#^m1)
- [2. Assistant: t724 adds a new row in Combat > Tweaks: Chest Opening (KH1  ](#^m2)
- [3. Assistant: Five versions this round. Your first trace showed why you co](#^m3)

---


  

> [!note] Tester Mr Bugs
> I want to add a config in the tweaks menu that lets use quickly open and move from the chest opening animation like in kh2, right now in kh1 when you open a chest it plays an animation then your locked in idle and a window appears telling what you got (player assumption) this version I want it to be like kh2 where you open the chest, move then the pop up for the item plays at the top of the game letting you know what you got. start coding for it
> 
> go ahead

^m1


> [!tip] Assistant
> **t724** adds a new row in Combat > Tweaks: **Chest Opening** (KH1 | KH2). It stays on KH1 until you switch it.
>
> **How KH1 opens a chest** (from its code): the Open command gives you the item right away, before any animation. Then it locks Sora, plays his open motion, and starts a small chest script for the lid and the "obtained" window, which holds you until you close it.
>
> **On KH2,** for a normal chest (an item or munny):
> - KH1's own code still opens it and gives the item, the same way as before.
> - Sora is let go at once, so you can walk off straight away.
> - The chest script still runs (the lid, the sounds), but its "obtained" window doesn't open.
> - "Obtained" plus the item name (or the munny) shows in KH1's own notice box, the one level-ups use, with nothing to press.
>
> Chests that start a scene, reward chests that aren't items, and a full inventory still play KH1's way.
>
> **What I checked:** KH1's chest code in the decomp and exe. What I couldn't check: the chest's own script file (too big to fetch this time), KH2's exact chest timing (KH2's chest code wasn't in the part of the decomp I had), and anything in game.
>
> **Please test:**
> - A few plain chests and a munny chest. Do you walk off right away? Does the lid still open, with its sound? Is the item really in your inventory? Does anything pull Sora back?
> - An event chest, and a chest with an item you already have the most of. Both should be unchanged.
> - Log lines (Menus log): "Chest Opening: KH2 open (kind ...)", "... message function N skipped", "... window time is over".
>
> **Questions:**
> 1. Where should the notice go? (a) KH1's level-up box where it is now, or (b) the top of the screen like KH2's info bar. (b) means teaching KH1's HUD a new notice spot.
> 2. Default: (a) KH1 by default, as now, or (b) KH2 by default.
> 3. The open motion: (a) Sora plays KH1's open motion until you move, as now, or (b) no open motion at all, like KH2.

^m2


wasn't able to move when opening a chest, I stayed in idle for a bit. the other parts of the code played except for chests that have gummi's in it and Dalmatians 

1. (b)
2. (b) and move it in QOL
3. (b)
   
   look into what you can remember the c steam decomp and you have access to kh2's files, sora still doesn't play his kh2 chest opening animation. fix all these problems so I can properly test; Sent a log and trace.
   
   go ahead
   
   


> [!tip] Assistant
> Five versions this round. Your first trace showed why you couldn't walk off: KH1's chest script was still holding Sora. His "held by a script" flags came on the frame after you opened the chest and stayed on for about 2.2 seconds, even though t724 had let him go. (Your log had no chest lines: the Menus log was off, and the newest log in this folder looks like your Drive Forms session. Chest lines now go to the Cutscenes & Events log.)
>
> - **t727:** Chest Opening is in QoL now, KH2 by default. If your ini already says `chest_style = kh1`, flip the row once.
> - **t728:** the chest script still runs in full (lid, sound, the item, the opened flag), but everything it asks Sora to do is marked done at once, through KH1's own route for "that actor isn't here". Its wait on Sora's motion passes at once too. So nothing holds, locks, turns or animates him.
> - **t729:** no open motion. I checked KH2's files and exe: KH2's Open command is a message to the chest, the chest plays its own lid animation, and KH2 has no chest action for Sora. So there's no KH2 chest-opening animation for Sora to port. Your answer 3 (b) is exactly what KH2 does: he just keeps moving.
> - **t730:** gummi and Dalmatian chests go the KH2 way too. The notice reads "Gummi Block" or "Dalmatians 01, 02, 03".
> - **t731:** KH1's notice box already sits in the top-left corner, where KH2's info bar goes; the row's help text now says so. KH2's own bar art isn't built yet (see the question).
>
> **What I checked:** KH1's script, action and chest code in the Steam decomp and exe; KH2's Open command in 03system; KH2's chest code in the Steam KH2 decomp; your trace for the cause. **Not checked:** KH1's chest script file itself (which actions it queues, in what order), the Dalmatian numbering, the gummi names, KH2's info-bar timing, and anything in game.
>
> **Please test:** a plain chest, a munny chest, a gummi chest and a Dalmatian chest. Do you walk off right away? Does the lid open with its sound? Do you really get the item (check the gummi hangar and the Dalmatian count)? Open one standing still: Sora shouldn't play KH1's open motion. Try one with a full stack (it should stay KH1's). Please send a trace and the log with Cutscenes & Events on. Lines to look for: "Chest Opening: KH2 open (kind ...)", "treasure script: action 0x.. on Sora ... done at once", "its wait on Sora's motion ... goes on", "open motion request replaced".
>
> **Question:** for the notice, should I (a) keep KH1's top-left box as it is now, or (b) build KH2's own info bar (its art and timing) as its own version? (b) means adding a reader for KH2's 2D HUD files on testing first.

^m3

(b)

I sent a log and traces, still unable to move on the chests we've modified,  I also found another chest type which is the evidence boxes you find in alice's world

do these changes without checking in, once you think you have a completed version upload it to github 

go ahead