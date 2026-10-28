# KH2 Chest

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: I want to add a config in the tweaks menu that lets use quic](#^m1)
- [2. Assistant: t724 adds a new row in Combat > Tweaks: Chest Opening (KH1  ](#^m2)
- [3. Assistant: Five versions this round. Your first trace showed why you co](#^m3)
- [4. Assistant: Two versions this round. Your log showed what still held Sor](#^m4)
- [5. Assistant: Fourteen versions since my last reply. booglechan asked for ](#^m5)

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


> [!tip] Assistant
> Two versions this round. Your log showed what still held Sora: the chest script never let go of him through the parts t728 didn't catch.
>
> - **t768, chests let go of Sora.** The chest script names Sora as "the party's first actor", not by his own number, so t728 never matched any of its actions on him (your log has no "done at once" line at any open). It also takes the player into KH1's event mode and turns off Sora's command input (the "runner 0x2, engine 0xc" in your log, right after each Open). Now its actions on Sora are found the way KH1 finds them, those holds are skipped for chests opened the KH2 way, and KH1's open motion is never asked for. The lid, the sound, the item and the opened flag are still KH1's own.
> - **t769, the evidence boxes.** They aren't chests to KH1: no chest code runs for them. They're part of each room's own script, which you start with the Triangle command. When you pick it, I read that part of the room script in place. If it's a box (it takes the player, gives something and gives him back, with no scene, choice, shop or menu), it opens the KH2 way like a chest, with the "Obtained" notice. Across all 236 room files that is Wonderland's evidence (two command types in two rooms) and some boxes in End of the World. Everything else stays KH1's.
>
> **How I checked it**
> - Your t743 log and trace `trace_1793085402_f43568` showed the cause. From the frame after the Open, Sora's flags sit at `0x0130c0c3` and KH1's open motion plays for three frames. Then he idles with the stick held and doesn't move. The log never has "done at once", has no motion-swap line, and shows "runner 0x2, engine 0xc" while the script runs.
> - The chest script itself (`UK_treasure.ev`, read from your extracted files): it takes the player at its start, holds him with actions that name "actor 0", and gives him back at the end. What those steps do, and how KH1 finds an action's actor, I checked in the KH1 Steam decomp and exe. The new hook points' first bytes are checked against the Steam exe.
> - The evidence boxes: your log at the Alice box (Triangle, then held about 2 seconds with no chest code). I also read every room file's script for which commands open a box. The part-reader has unit tests.
> - I couldn't check either version in game.
>
> **Please test:**
> 1. A plain chest, a munny chest, a gummi chest and a Dalmatian chest with Chest Opening on KH2. Can you walk off the same frame you open it? Does the lid still open with its sound? Is the item really added?
> 2. Open a chest standing still. Sora shouldn't play KH1's open motion.
> 3. The evidence boxes in Wonderland (Lotus Forest and the Bizarre Room ones). Can you move at once? Does the notice show the evidence's name? Does the evidence count for the story (the Queen's trial)?
> 4. An event chest, a chest with an item you already have the most of, and the Trinity marks. All should be unchanged (KH1's).
> 5. Please send the log with Cutscenes & Events on and a trace of a chest and an evidence box. Lines to look for: "Chest Opening: KH1's open motion not asked for", "treasure script's player hold (event function 511) skipped", "action 0x.. ... done at once", "KH2 open of a room script's box (command 0x55 ...)", "the box gives ... Obtained / ...".
>
> **Still open on my side (no answer needed)**
> - **KH2's own info bar (your (b)) isn't in yet.** That needs a reader for KH2's 2D HUD files, finding KH2's info-bar art and timing in its exe, and drawing it through KH1's own 2D path. The art isn't in the field HUD sheet I checked (`zz0field.2dd`). The item icons it uses look like the ones in `zz0command.2dd`, but I haven't confirmed that in KH2's code. It comes as its own versions next. Until then the notice is KH1's top-left box.
> - Traverse Town's command 0x72 reads like a box but I left it KH1's (I think it's the mailbox). If you know of other boxes that still hold you, tell me where.
> - Notes: `docs/kh1-chest.md` ("Round 3"). Thread folder: `testers/kh2-chest/`.

^m4


> [!tip] Assistant
> Fourteen versions since my last reply. booglechan asked for most of these directly, so this is the catch-up. The setting is now **Chest Opening: Vanilla | Quick | Skip**, and Quick has a **Chest Animation (KH1 | KH2)** row under it.
>
> - **t770, three settings.** Vanilla is KH1's chest as it was. Skip is the old KH2 setting: no animation, you move right away. Quick plays a chest animation, then lets you move. Choosing Quick shows the Chest Animation row in QoL.
> - **t771 and t772, Quick holds Sora through the animation.** KH1's open motion has no end of its own: the chest script asks for his idle after it. Quick now lets the script run its actions on Sora as KH1 does, so the animation plays out and then he goes back to idle.
> - **t773, Quick lets Sora go on idle.** Your trace showed him back on idle (slot 0) about 50 frames before the script released him. Now he's let go the moment he's back on idle, and the rest of the open runs the Skip way.
> - **t774, the debug menu's animation counter counts KH1's own animations.** It read 0 on them before.
> - **t775, the item notice on vanilla's frame.** On Quick it waits until 100 frames into the open: the chest animation plus 20 frames of idle.
> - **t776, KH1's own item window.** The evidence boxes' text box is KH1's message window 7, and a vanilla chest uses the same window. Quick and Skip now let the chest and box scripts open it as KH1 wrote them, and it closes itself, while Sora keeps moving. This replaces the top-left notice, so it also covers your (b): booglechan picked KH1's own window over a separate KH2 info bar.
> - **t777, the Dalmatian window is one line.** It used to be two lines in the middle of the screen. Now it reads "Found dalmatians no. A, B, C." and sits where the "Obtained" windows go.
> - **t778, Skip keeps Sora's run.** KH1's Open sets his speed to 0. On Skip he now runs on through the open.
> - **t779, KH2's reach.** In KH2 the box offers Open whenever its reach sphere touches Sora, 185 units across. KH1 needed the chest in front of him or a bump into it. On Quick and Skip each chest now gets KH2's reach, so you can tap Triangle while running past.
> - **t780 to t782, KH2's facing rules.** Open shows greyed (visible but unusable) if the chest is more than 135° off your facing, or if you're behind it. t781 added a log line, and t782 reads each chest's real facing (t780 read a value that's 0 on every chest).
> - **t783, KH2's chest animation.** It wasn't in Sora's own motion file. It's in the chest's (`F_EX030.mset`, entry 252: Sora's body and Keyblade, 60 frames). It's now one of KH1's own animation slots on KH1's open request. With Quick and Chest Animation on KH2, Sora plays it; on KH1 he plays KH1's.
>
> **How I checked it**
> - Your traces and logs: the hold after slot 83 (t773), the chest that never opened from a run (t779), and the throne-room chests greyed by the wrong facing (t782). Numbers are in `docs/kh1-chest.md`, rounds 4 to 17.
> - KH2's side, from the KH2 Steam decomp and KH2's files:
>   - the box's reach sphere and its Open command (0x20);
>   - its two facing limits;
>   - its lid handler;
>   - the chest animation in the box's own motion file.
> - KH1's side, from the KH1 Steam exe and decomp:
>   - the chest's offer check, its range test and the command box;
>   - the open motion request (0xA6, slot 83);
>   - the speed-zeroing call;
>   - the message window functions and the chest script's own texts.
> - Every hook's first bytes are checked against the Steam exe. None of this is checked in game.
>
> **Please test:**
> 1. Each setting with a plain chest, a gummi chest, a Dalmatian chest and an evidence box:
>    - **Vanilla:** KH1's chest, unchanged.
>    - **Quick:** the animation plays, you move as soon as Sora is back on idle, and the item window comes about 100 frames in.
>    - **Skip:** you run straight through the open.
> 2. Quick with Chest Animation on KH2: Sora plays KH2's chest animation, about 1 second. On KH1 he plays KH1's.
> 3. Run past a chest tapping Triangle, from its front and from behind:
>    - from the front it should open;
>    - from behind, or with the chest behind you, Open should show greyed.
> 4. The item window: it should look like the evidence boxes' window and close by itself. The Dalmatian one should be a single line at the top.
> 5. Please send the log with Cutscenes & Events on. Lines to look for: "Chest Opening: asked chest …", "KH2's reach …", "Quick: KH2's chest motion …".
>
> **Still open on my side (no answer needed)**
> - KH2's chest animation turns Sora toward the chest over its first 12 frames. KH1 doesn't do that yet, so he opens facing the way he was.
> - Some of that animation's other triggers aren't identified yet.
> - I haven't followed KH2's box script for its exact hold length. KH1's chest script holds Sora until the animation ends.
> - Changing the setting only affects chests that load afterwards, so re-enter the room.
> - Notes: `docs/kh1-chest.md` (rounds 4 to 17).

^m5


