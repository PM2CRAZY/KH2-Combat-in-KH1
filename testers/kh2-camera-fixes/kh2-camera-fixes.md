# KH2 Camera fixes

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: I noticed when you run backwards the camera starts turning s](#^m1)
- [2. Assistant: t373](#^m2)
- [3. Mr Bugs: 1. it seems right but check the ghidra kh2 steam c decomp to](#^m3)
- [4. Assistant: t379 to t384 (one change each; the build at the top of the b](#^m4)
- [5. Mr Bugs: kh2 "Found, not changed (KH2's lock-on turn speed is a const](#^m5)
- [6. Assistant: t408 and t409, plus notes](#^m6)
- [7. Mr Bugs: 1. not sure what this question means but, kh1s cam feels fas](#^m7)
- [8. Assistant: t413 and t414](#^m8)
- [9. Mr Bugs: 1. check the decomp for both (steam gihidra c decomp)](#^m9)
- [10. Assistant: t450 to t452. Sorry, I missed your answers last round. Here ](#^m10)
- [11. Assistant: Thanks. Nothing in this thread is waiting on a go-ahead righ](#^m11)
- [12. Assistant: I went back through everything in this thread against KH2's ](#^m12)
- [13. Assistant: Got it. Here's what I found for each, so you can give the go](#^m13)

---


  

> [!note] Tester Mr Bugs
> I noticed when you run backwards the camera starts turning sora in a circle and rotates this is not indented, can you please look through the kh2 decomp to verify and fix this change then if you need the kh2 ghidra c decomp steam version to find any other missing features do it. I've logged a trace for you to see whats going on in game
> 
> go ahead

^m1


> [!tip] Assistant
> t373
>
> Your trace showed it exactly. With the stick held straight down, our camera read it as pushing forward. So it kept swinging round behind Sora at 1° a frame, and since Sora runs relative to the camera, he turned with it and ran in a circle. The cause was the stick's up/down being passed to the KH2 camera the wrong way round.
>
> **What KH2 does.** I checked KH2's camera code in the Steam KH2 decomp (the follow update at `0x140165110`). KH2 doesn't swing the camera while you're running almost straight at it, at nearly full tilt. If you run diagonally backwards, or pull back only part way, KH2 does still swing slowly, so you'll get a gentle curve there. That's KH2's own behaviour, and ours now does the same.
>
> **Check:** the turn per frame in your trace matches KH2's swing formula exactly, and the condition was read in the KH2 decomp. Not checked in game yet. Notes are in `docs/kh1-kh2-camera.md` (9a) and `docs/camera-model.md`.
>
> Two questions:
> 1. Please try running straight back and diagonally back. Does the diagonal curve feel like what you remember from KH2?
> 2. **Stick dead zone.** KH2 ignores about the first half of the stick's tilt (60 of 127.5 on each axis). Ours ignores 15%, because gentle tilts used to do nothing. At full tilt they're the same, but at part tilt the swing differs. (a) switch to KH2's exact dead zone for 1:1, or (b) keep ours?
>
> Other things I found in KH2's camera, not changed yet: the right stick may turn the camera at full speed even when tilted diagonally (I need a closer read before I'd change it), there's a reset branch whose purpose I don't know yet, and I haven't read KH2's zoom code. Tell me if you want any of those looked at.

^m2


> [!note] Tester Mr Bugs
> 1. it seems right but check the ghidra kh2 steam c decomp to make sure
> 2. it should be 1:1 
> 
> yes, look at everything related to the camera that your unsure about, use ghidra kh2 steam c decomp for it.
> 
> go ahead

^m3


> [!tip] Assistant
> t379 to t384 (one change each; the build at the top of the branch has them all)
>
> I went back through the KH2 decomp like you asked, and I got one thing wrong last time. KH2 looks at which way the stick points, not how far it's tilted. So running straight back never swings the camera, even at part tilt, as long as you're within about 26° of straight back. Further round than that (diagonally back), it still swings slowly. Ours now does exactly that (t379).
>
> Also done, all from KH2's code (Steam KH2 decomp):
> - **t380:** the camera uses KH2's exact stick dead zone (60 of 127.5 on each axis, so about the inner half), read the way KH2's pad code reads it. KH1's own sticks are built the same way, so the keyboard and mouse still work as before.
> - **t381:** KH2 movement now reads the stick the same way, so very gentle tilts won't walk Sora any more, like KH2. This changes how gentle tilts feel, so tell me if anything's off.
> - **t382:** the right stick turns and zooms by its whole tilt, so a full diagonal turns and zooms at full speed together, like KH2.
> - **t383:** a reset (R3) puts the eye straight on its goal and only the turn eases; while you're moving it's an instant cut, with no drift after.
> - **t384:** faster forms swing and follow faster, from the form's own run speed (before, it was fixed at Sora's).
>
> **Check:** decomp for all of them; not checked in game. Assumptions: which pad bits KH2's turn and zoom test are in the exe's data tables, which the decomp doesn't show, so that they're the right stick's x and y is an assumption, and that the run speed KH2's camera reads is the form's. Notes are in `docs/camera-model.md`, `docs/kh1-kh2-camera.md` and a new `docs/kh2-pad.md`.
>
> Found, not changed (KH2's lock-on turn speed is a constant I can't read here; ours is 3°, and KH2 scales turning by the mouse's motion, which ours doesn't):
> 1. **KH2 has its own first-person view:** Sora fades out over 30 frames, you look from his head 350 units ahead, the right stick tilts the view up and down, and he fades back in when you leave it. Ours uses KH1's first person on Select. (a) switch to KH2's first person, or (b) keep KH1's?
> 2. **Walk / run split:** which tilt KH2 counts as a run isn't confirmed yet (we use half tilt). Want me to look it up in the decomp next?
> 3. **An unknown button** can stop KH2's auto-swing and allow zoom in places it's normally blocked; which button it is sits in KH2's remap tables. Want me to track it down?

^m4


> [!note] Tester Mr Bugs
> kh2 "Found, not changed (KH2's lock-on turn speed is a constant I can't read here; ours is 3°, and KH2 scales turning by the mouse's motion, which ours doesn't):" Make sure your not mixing up the mouse camera and the controller camera, those are seperate cameras (at least from a user's experince, unsure how it is in code)
> 1. (a)
> 2. yes
> 3. unsure what an "unknown button" is but yes go ahead and do it
>    
>    go ahead

^m5


> [!tip] Assistant
> t408 and t409, plus notes
>
> **The mouse:** you're right, they're separate in KH2 too. KH2 PC has mouse-only parts: the mouse takes over the right stick (and the stick dead zone drops for that frame), the follow camera's turn and zoom speed up the faster you move it, and in keyboard/mouse mode the camera follows much more tightly. The lock-on camera has no mouse speed-up, and the swing-behind has no mouse branch. Ours runs KH2's controller numbers only. I checked t380 to t384, and none of the mouse parts leaked into them. With the mouse, ours just treats it like a tilted stick. The notes are in `docs/kh2-pad.md`.
>
> 1. **t408, KH2's first person** is in, through KH1's own first-person mode. Sora fades out over 30 frames, you look from his head (KH2's eye height for each form) 350 units ahead, the right stick's up/down looks up and down, R3 levels the view, and Select again fades him back in. He can't walk in it and turns 3° a frame.
> 2. **t409, walk / run:** KH2 walks Sora at any tilt past the stick's dead zone and runs him from 80% tilt. Ours ran from 50%, and had an extra 10% dead zone KH2 doesn't have; both are fixed now. (Base Sora in KH1's own states already ran from 80%, as KH1 does.)
> 3. **The unknown button:** with Field Camera on Auto, holding it stops the swing-behind (on Manual it's the only time the camera swings), and while swimming it makes the right stick zoom. Which button it is sits in KH2's remap tables, which I can't read here. My best guess is L2, but that's a guess, so I haven't added it.
>
> **Check:** the Steam KH2 decomp for all of it, and the game files for the eye heights. Not checked in game. Assumptions: the first person's look up/down speed and limits are KH1's (80° up, 60° down, 3° a frame), because KH2's are in data I can't read, and that the right stick's left/right turns him.
>
> Questions:
> 1. First person, look up/down: (a) keep KH1's speed and limits, or (b) tell me how KH2's felt different.
> 2. First person, turning Sora: in KH2, which stick did it? (a) the right stick left/right (what I did), or (b) the left stick.
> 3. If you can, try this in KH2: Field Camera on Auto, run in a circle while holding L2. If the camera stops swinging behind Sora, it's L2 and I'll add it. If not, which button does it? Or (b) leave it out.
> 4. The mouse camera: (a) leave it as is, or (b) add KH2's mouse speed-up and its tighter keyboard/mouse follow.

^m6


> [!note] Tester Mr Bugs
> 1. not sure what this question means but, kh1s cam feels faster and rigid, controls on the left stick while kh2s cam is slower and smooth, controls on the right stick.
> 2. going to send you a trace for 1 and 2
> 3. its R2
> 4. (b) make sure you compare it with the kh1s steam c ghidra decomps mouse cam because they feel pretty close to each other movement wise. 
>    
>    go ahead

^m7


> [!tip] Assistant
> t413 and t414
>
> Thanks for the traces and the answers.
>
> - **First person:** your two traces show exactly what you described. KH1's is on the left stick, follows Sora rigidly and turns by how far you push (3° × tilt, no smoothing). Ours (KH2's, t408) is on the right stick, turns a steady 3° a frame and eases the view with KH2's smoothing. Everything about it I can read in KH2's code already matches, so no change there. The one thing I can't read is how fast KH2 looks up and down and how far it can go (it's in KH2's exe data), so ours uses KH1's: 3° a frame, 80° up, 60° down.
> - **t413, R2:** with Field Camera on Auto, holding R2 stops the camera swinging behind Sora; on Manual it only swings while you hold R2. The keyboard's R2 key should count too. With Frame Advance on, R2 still only steps frames. (Zooming with R2 while swimming isn't in: swimming is KH1's own state, so KH1's camera runs there.)
> - **t414, mouse (Steam):** the camera turns and zooms faster the faster you move the mouse, the sensitivity setting counts, and with keyboard and mouse the camera follows Sora much more tightly, like KH2 PC. You were right that it's close to KH1's mouse camera: KH1 PC and KH2 PC share the same mouse code, and both speed up with mouse speed by the same factor from about the same base (KH1 about 2.5° a frame, KH2 3°). The differences: KH1's ramps up to speed and stops dead when the mouse stops, and its mouse up/down tilts the camera where KH2's zooms. The comparison table is in `docs/kh2-pad.md`.
>
> **Check:** the Steam KH2 decomp for both, the Steam KH1 decomp and exe for the mouse code and button table, and your traces for first person. Not checked in game. Assumption: that the keyboard's R2 key reaches the same pad bit.
>
> Questions:
> 1. First-person look up/down: (a) it feels like KH2's, or (b) KH2's is slower. If so, roughly how much (half?), and does KH2 let you look less far up or down?
> 2. In KH2 in first person with the mouse, does moving it left/right turn Sora? (a) yes, add it, or (b) no / leave it.
> 3. At 30 fps, ours follows a little less tightly than KH2's with keyboard and mouse. (a) fix it to KH2's 30 fps behaviour, or (b) leave it?

^m8


> [!note] Tester Mr Bugs
> 1. check the decomp for both (steam gihidra c decomp)
> 2. check the steam gihidra c decomp
> 3. (a)
>    
>   go ahead

^m9


> [!tip] Assistant
> **t450 to t452.** Sorry, I missed your answers last round. Here they are now.
>
> 1. **t450, mouse in first person:** KH2's code turns Sora a steady 3° a frame from the right stick, and KH2's pad puts the mouse over the right stick whenever it moves. So in KH2, moving the mouse left or right turns him at that same steady speed, with no extra speed for moving it faster. Ours does that now, for base Sora and in forms. Moving it up and down already looked up and down.
> 2. **t451, 30 fps:** at 30 fps KH2 eases the camera once a frame at the full rate. Ours eased twice at half the rate, which is why it felt looser. It's KH2's now. With keyboard and mouse it follows much more tightly at 30 fps (about 91% of the gap a frame instead of 75%). With a controller the difference is tiny. 60 fps is unchanged.
> 3. **t452, looking up and down in first person:** KH2's code works the same way as ours, but its speed and top/bottom limits are numbers stored in KH2's exe, which the decomp doesn't show. Your KH2 exe is in the game folder you connected, so I read them straight from it: 3° a frame (same as before) and 70° up and 70° down. Ours used KH1's 80° up / 60° down until now.
>
> **Check:** the Steam KH2 decomp for t450 and t451, and your KH2 exe for t452 (the same function the decomp shows reads those three numbers). Not checked in game. Assumption: KH2's first-person turn is on the right stick's left/right, from your trace.

^m10


go ahead


> [!tip] Assistant
> Thanks. Nothing in this thread is waiting on a go-ahead right now: t450 to t452 (the mouse turn in first person, KH2's 30 fps follow, KH2's 70° look limits) are already in the build on GitHub. Give them a try and tell me how they feel, or what you'd like changed next.

^m11


if you think everything is one to one then close this thread, if not let me know


> [!tip] Assistant
> I went back through everything in this thread against KH2's Steam decomp, and this time I also read the numbers straight out of your KH2 exe. Almost all of it checks out, but it isn't fully one to one yet, so I'd hold off closing.
>
> Matching KH2 (checked):
> - The swing-behind when you run back or diagonally back: the rule and its exact rates.
> - KH2's stick dead zone, and the turn and zoom moving by the stick's whole tilt.
> - The R3 reset, and the swing speeding up with the form's run speed.
> - First person: Select to enter, the right stick turns and looks, 3° a frame, 70° up and down.
> - R2 holding the swing, the mouse camera, and the 30 fps follow.
> - The lock-on camera's turn speed: last time I couldn't read it; it's in your exe, 3°, the same as ours.
>
> A few guesses I'd marked earlier are now confirmed from KH2's button tables: the turn and zoom really are the right stick, R3 is the reset, and Select is first person.
>
> What's still different:
> 1. **Lock-on side kick.** When the enemy (or Sora) goes off the side of the screen, KH2 turns the camera an extra 20°, and which way depends on which edge it left by. Ours picks the direction from whichever way the camera was already turning. (a) switch to KH2's rule (I'd check the direction with a trace from you), or (b) leave it.
> 2. **L2 and the right stick.** In KH2, while L2 is held, the right stick stops moving the camera (it acts like the d-pad instead) and R3 doesn't reset it. Ours moves the camera whatever L2 is doing. (a) copy KH2's (I'd first check what L2 does in KH1 with the mod, so nothing clashes), or (b) leave it.
> 3. **Camera distance.** This one's on purpose: we shrink all of KH2's camera distances to 89% because KH1's Sora is smaller, so he fills the screen the way he does in KH2. KH2 also pulls the camera further back in some big rooms; KH1's rooms don't have that flag, so ours always uses KH2's normal-room distances. (a) keep the 89%, so Sora looks the same size on screen as in KH2, or (b) use KH2's raw distances.
>
> Not checked yet: the 16:9 field of view, the exact wall pull-in on Steam, and whether the keyboard's R2 key counts. None of t450 to t452 has been tried in game yet either, so please give those a go too.
>
> Tell me your picks and "go ahead" for the ones you want changed; if you're happy with the differences as they are, say so and I'll close the thread.

^m12


1. (a)
2. (a) but make it a toggle in the cameras config menu
3. keep  (a) 
   
   one thing about our design, we need to keep everything that kh1 can do in tact without breaking or changing how kh1 plays, if all settings are set to kh1 with our new systems and it still feels like kh1 then we succeeded, the exception is when kh2 systems that are native interact with kh1s. we need to add missing features to kh1s engine and still keep features that were taken out of kh1s engine when they ported everything to kh2. both systems need to be native both need to be combined when they can, both need to share the same systems (like how effects added more calls so kh2s render can work properly) and if there's a critical design choice that affects game feel, then we can make it a toggle for the player to change.  

> [!tip] Assistant
> Got it. Here's what I found for each, so you can give the go-ahead.
>
> 1. **Lock-on side kick, (a).** I found KH2's exact rule in the Steam decomp (`0x140176EC0`, with its screen test `0x140176710`). The enemy is tested first and Sora only if the enemy is still on screen. If it went off by the **left** edge, the camera's goal is turned **+20°**; off by the **right** edge, **−20°**. If it's off both at once, left wins. If it only left by the top or bottom, there's no kick, just the usual swing behind. Ours instead picks the direction from the way the camera was already turning, and I'd switch it to KH2's rule. The only part I can't be sure of from the code is whether KH2's "+" turns the same way as ours, though both use the same yaw formula, so I expect it does. A short trace afterwards would settle it: lock on, then let the enemy leave by the left edge once and the right edge once.
> 2. **L2 and the right stick, (a) as a toggle.** In KH1, even with the mod, L2 does nothing for the camera. The only use I found is that pressing L2 (or R2) drops KH1's small soft-target cursor (`0x1402B11B0`), and KH2's rule doesn't touch that. So nothing clashes. The plan: a new row on the Config menu's Camera page, **"L2 Camera Hold: On / Off"**. With it On and L2 held, the right stick doesn't move the KH2 camera and R3 doesn't reset it, as in KH2. One question: should the right stick under L2 do anything else, like KH2's d-pad use (moving the command menu cursor)? (a) yes, copy that too; (b) no, just stop the camera. Also, which default? (c) On, KH2's way; (d) Off.
> 3. **Camera distance:** staying at 89%, as you said.
>
> On your design note: understood, and it matches how I've been working. With every row on KH1, KH1 should play exactly as it does without the mod. Where KH2's systems meet KH1's, they share KH1's systems, the way the effects did. KH1's own features stay, and KH2's missing ones are added to KH1's engine. A design choice that changes the feel gets a toggle. Point 2 is an example: a row instead of changing the camera for everyone.
>
> Write "go ahead" (with your picks for the L2 questions) and I'll build 1 and 2 as one version each.

^m13

