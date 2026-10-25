# KH2 Glide in KH1's glide state

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)
- [2. Assistant: Here's what I think:](#^m2)
- [3. Assistant: A correction to my last message, before you answer it. I che](#^m3)
- [4. Assistant: Done, as planned, with (b) for Donald and Goofy. One version](#^m4)
- [5. Assistant: Thanks for the trace and the log. They showed exactly what w](#^m5)
- [6. Assistant: Thanks. (a) it is: after you let go, the fall keeps followin](#^m6)

---


> [!tip] Assistant
> I started this thread from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md), as you asked. Here's the first round of research.
>
> **What we have now.** At Glide KH1 it's KH1's own glide. At Glide KH2 it's still a borrow (t321): Square in the air hands Sora to KH2's controller, which runs its own glide and then gives him back to KH1's fall.
>
> **Why the shared version (t284 to t320) wasn't 1:1.** It only swapped two things in KH1's glide state: the turn rate and the speed / fall. Everything else stayed KH1's: its start (higher than 50, speed halved), a 46-frame minimum glide, a 24-frame wait before you can glide again, its release (speed x0.7 plus a little hop), its turn animations (picked at 30°), and no hover. On top of that, at physics KH1 none of the swap ran.
>
> **What KH2 does (KH2 decomp, t331's exe notes).**
> - Start: hover (208) if the stick is neutral, glide otherwise. The stored launch speed (jump or Aerial Dodge) is put back.
> - Speed: `(1 - a) * speed + a * |v|` along his facing. The stick only turns him, at the growth level's turn speed.
> - Fall: comes from the height above the floor (the fall height, ratio and max).
> - Animation: banks into 206 / 207 once the stick is more than about 11° off his facing (dot 0.98), else 205.
> - End: letting go, dropping under the end height, or landing goes into the fall with his speed kept. No hop, no speed cut.
>
> **How KH1 does it (KH1 decomp).** State 0x15 is registered at `0x1402B26F0`.
> - Start `0x1402B2D30` (Glide `0x1402B26C0`, Superglide `0x1402B2750`).
> - Pre-step `0x1402B2880`: the turn, the animation pick and the water entry.
> - Step `0x1402B2A20`: KH1's speed ramp and its saw-tooth fall.
> - Button check `0x1402B2BF0`: the 46-frame timer and the release.
> - Transition `0x1402B2780`: Neverland flight, and ending the glide on a wall.
>
> Neverland's flight, water entry and some area objects read or end this state, so they stay working.
>
> **Plan, one version each, all inside KH1's state 0x15 at Glide KH2:**
> 1. Start: KH2's start height, its hover-or-glide choice and the stored launch speed, with no KH1 timer and no wait.
> 2. Step: KH2's speed and fall written into KH1's velocity fields. KH1's mover and collision still move him.
> 3. Pre-step: KH2's turn and KH2's motion choice (205 / 206 / 207 / 208) as KH1 animation slots. KH1's water check stays.
> 4. End: KH2's release into KH1's fall state, keeping his speed.
> 5. Take the borrow out.
>
> The physics setting stays out of it, as you asked in t321. It's also cheaper: only a few wrappers run, and only while he's gliding.
>
> **What I couldn't find yet:** whether KH2 has any glide cooldown or minimum time like KH1's, and what KH2 does when a glide hits a wall. The notes are in `docs/kh1-glide-native.md`.
>
> Questions:
> 1. KH2's hover (208) has no KH1 counterpart. Should it be (a) a sub-mode inside KH1's glide state 0x15, or (b) a new KH1 state of its own next to it? Neverland and the area objects only know 0x15, so (a) keeps them working with the hover too.
> 2. Until I find KH2's wall rule, what should a glide into a wall do? (a) Keep KH1's rule (the glide ends), or (b) keep gliding and slide along the wall?
> 3. Some KH1 area objects start or end KH1's glide. At Glide KH2 they would (a) start and end KH2's glide, or (b) be left alone (no glide from them).

^m1


1. unsure I was thinking b was the best way while letting neverland and area objects see kh2s hover but tell me what you think
2. (b) until you find the wall rule
3. unsure what area objects are, option (a) but tell me what you think
> [!tip] Assistant
> Here's what I think:
>
> 1. **The hover.** I'd go with (a): the hover as a sub-mode inside KH1's glide state 0x15, which gets you what you were after with (b). Neverland's flight check and the area objects only look for state 0x15. A separate state for the hover would be invisible to them unless I also changed each of those checks. As a sub-mode, they see the hover as a glide with no extra work: you can still go into flight from it in Neverland, and an object that ends a glide ends the hover too. Underneath it's still KH2's hover (motion 208, KH2's slow-down and its switch back to the glide when you push the stick).
> 2. **Walls:** (b) then. The glide keeps going and slides along the wall until I find KH2's own rule. When I do, I'll tell you what it is before changing anything.
> 3. **Area objects.** These are invisible zones built into some rooms, KH1 collision pieces with a "glide" flag (`0x1401CF9E0`, `0x1401CD050`). When Sora flies into one in the air, KH1 itself starts his glide or Superglide, and some of them end it. I don't know yet which rooms have them; I'll log every time one fires so we find out. I agree with (a): at Glide KH2 they start and end KH2's glide, so a room that glides you somewhere still works.
>
> If that sounds right, write "go ahead" and I'll build the plan from my last message, one version each: the start, the step, the turn and animations, the end, then taking the borrow out. The hover sub-mode comes with the start.

^m2


> [!tip] Assistant
> A correction to my last message, before you answer it. I checked both points again in the Steam decomps:
>
> - **The "area objects" were my mistake.** `0x1401CF9E0` and `0x1401CD050` aren't room zones. They're **Donald's and Goofy's AI** following you: when you jump a gap their AI jumps after you, and if you own Glide or Superglide they glide on their own bodies in KH1's glide state, then their AI ends that glide when they catch up with you. The code only acts on party members 1 and 2 (Sora is 0). Nothing in KH1 starts or ends Sora's own glide from outside the glide state's own code. So question 3 goes away: every change only acts on Sora, and Donald and Goofy keep KH1's glide. (Checked (decomp): the AI think `0x1402A3300` → `0x1401CDBD0` → those functions.)
> - **What else reads the glide state:** besides Neverland's flight and the water check (both inside the glide's own steps), only KH1's follow and lock-on cameras (`0x1401E6670`, `0x1401E2650`), which treat 0x15 as "in the air" at Camera Style KH1. That's one more reason for the hover as a sub-mode (a): they all see it with no patching.
> - **Walls:** KH2's glide has no wall rule at all (its glide action's own steps only end on letting go, the end height or landing; the shared air handlers don't test walls either). So (b), sliding along the wall, is KH2's real behaviour, not a stopgap. KH1's own "moved less than 70%, end the glide" check will be skipped at Glide KH2. Still to check: whether KH2's speed drops when you hit a wall head-on.
>
> The plan is unchanged otherwise. One small question: Donald and Goofy only glide after you when Sora owns KH1's Glide or Superglide. With Glide KH2, should they (a) stay as they are now, or (b) also glide after you whenever you can glide with KH2's?
>
> Write "go ahead" with that answer and I'll build it. The notes are in `docs/kh1-glide-native.md` §7.

^m3


(b)

go ahead

> [!tip] Assistant
> Done, as planned, with (b) for Donald and Goofy. One version per part:
>
> - **t526, the start.** Square in the air puts Sora into KH1's glide state (0x15) with KH2's start: KH2's start height for your Glide level instead of KH1's 50, no KH1 46-frame minimum and no 24-frame wait (KH2 has neither), and his speed isn't halved. If an Aerial Dodge launched him this jump, KH2's stored launch speed comes back as the glide starts, as in KH2. KH1 also stops starting its own glide on him from the jump button.
> - **t527, the step.** KH2's speed (`(1 - a) * speed + a * |v|` along his facing, whatever the stick's tilt) and KH2's fall from his height over the floor, on KH2's 30 Hz tick. KH1's mover and collision still move him. Against a wall he slides along it (I haven't found KH2's own wall rule yet: see the note at the end).
> - **t528, turning and animations.** KH2's turn speed for your level, and KH2's motions as KH1 slots: 205 gliding, 206 / 207 banking once the stick is more than about 11° off his facing, 208 for the hover. The hover is the sub-mode we agreed on: start with the stick neutral and he hovers (slowing x0.9), push the stick and it turns into the glide. KH1's water check still runs, so gliding into water still drops him in.
> - **t529, the end.** Letting go of Square, dropping under KH2's end height, or touching down puts him in KH1's fall with his speed kept: no x0.7 speed cut and no little hop. Neverland's flight from the glide still works exactly as KH1 does it.
> - **t530, the borrow is gone.** Square in the air now starts the glide in KH1's state, and the jump during the glide (with the dodge left) ends it into the native Aerial Dodge. KH2's controller doesn't glide base Sora any more; Drive Forms keep their own glide.
> - **t531, Donald and Goofy.** With Glide set to KH2 and KH2's Glide equipped, they now glide after you on jumps, even when you don't have KH1's Glide. Their glide itself is still KH1's (that's their own AI).
>
> How I checked it: every number and rule comes from the KH2 Steam decomp (the glide and hover actions' enter, update, step, fall and end), and every KH1 hook point from the KH1 Steam decomp (the state table's layout, the glide state's handlers, the party AI's ability check). There are no new code hooks, only KH1's own state-table entries for 0x15, wrapped for Sora alone. I couldn't check it in game. Two things are still assumptions:
> - which bank (206 or 207) goes with which side of the turn. I matched what the old borrow did.
> - that KH2 ends the glide the moment you let go. That check isn't in the glide action's own code; I matched KH2's controller (the t331 exe match).
>
> Please test:
> 1. A glide from a jump, and one right after an Aerial Dodge (the dodge's speed should carry into the glide).
> 2. Start with the stick neutral to get the hover, then push the stick.
> 3. Sharp turns: does he bank into the turn, not away from it?
> 4. Glide into a wall: he should slide along it, not stop.
> 5. Let go mid-air (no hop, speed kept), and glide down to the floor.
> 6. Jump during a glide (the Aerial Dodge), water, Neverland's flight, and Donald and Goofy following you over a gap.
>
> Still open on my side (no answer needed):
> - KH2's jump also stores a launch speed (separately from the dodge's). The old borrow never carried it, and neither does this yet, so after a plain jump the glide starts from his speed at that moment. I'll check in KH2's code whether its glide reads that speed and carry it the same way if it does.
> - I haven't found yet whether KH2 cuts the glide's speed when you hit a wall head-on. Right now the speed isn't cut and he slides. If it feels wrong against walls, tell me and I'll dig into KH2's collision.

^m4


I've recorded a trace and sent a log, should be in the same folder as this thread. I noticed sora can't bank to the left but can to the right
	-momentum currently doesn't work with letting go mid glide I just drop.
	 -the other systems seem to work but not sure if the values are correct so just double check
	
keep looking for 

go ahead


> [!tip] Assistant
> Thanks for the trace and the log. They showed exactly what was wrong with both. Two fixes:
>
> **t548, the left bank.** KH2 picks the bank correctly: 206 for one side, 207 for the other. The problem was on KH1's side. KH1 asks for a different motion for each side of a turn, but the motion set Sora uses in a world sends the left one to the same KH1 slot as the right one. The file has them on separate slots. So KH2's 207 never got hooked up, and both sides played 206. Your log even said it ("not in this set: 0x74"). Now 207 plays on the left whichever slot the set uses.
> Checked: KH2 decomp (the bank pick, and which way round the turn's cross product goes), KH1 decomp (the turn requests), KH1's motion file, your log and trace.
> Not checked: in game. Which side counts as "left" assumes KH1's and KH2's worlds turn the same way. Your trace fits that: stick right, he turns right, 206.
>
> **t549, momentum when you let go.** KH2's glide end keeps the whole glide velocity going into the fall. We did write it, but KH1 overwrote it in the same frame. When Sora enters the fall state during KH1's input step, KH1 resets his air speed to the stick's direction times its tilt, which is at most 1 a frame. Your trace shows it: 30.1 a frame at the release, 1.1 the next. Now his glide speed goes back on before KH1 moves him. We already handle KH1's carry the same way for the hop out of a ground combo. Landing and dropping under the end height weren't affected, since those ends happen at another point in the frame.
> Checked: KH2 decomp (the glide end only resets the arc and the launch flag), KH1 decomp (the think step's carry into the fall), your trace frame by frame.
> Not checked: in game. After the release, how he slows down follows your Physics setting. Your traces were on Physics KH1: KH1's fall keeps the speed while you hold the stick and slows it a little each frame without.
>
> **The other values.** I checked them against KH2's growth table and your traces, frame by frame, for Glide MAX:
> - **Speed:** heads toward 32 at KH2's 0.97, once every two frames (18.42 → 18.83 → 19.22).
> - **Hover:** x0.9 every two frames.
> - **Fall:** 1.0 a frame until he's under about 130 over the floor, then (height − 30) x 0.01.
> - **Turn:** 0.052 a frame.
> - **Banking:** kicks in past about 11° off his facing.
> - **Start height:** 60.
> - **Aerial Dodge carry-over:** the dodge's speed (18, or 0 after a neutral dodge) comes back as the glide starts, and again when the hover turns into the glide.
>
> All of these match KH2, so there's nothing else to change there. The full table is in `docs/kh1-glide-native.md` §8.7.
>
> Please test:
> 1. Glide and turn hard left, then hard right: he should bank into each side with its own animation.
> 2. Let go mid-glide, with the stick held and with it neutral. He should keep his speed and fly on, not drop.
> 3. Let go right after an Aerial Dodge into the glide, and glide all the way down to the floor.
>
> Still open (no answer needed): KH2's plain jump also stores a launch speed, which its glide would take back. Ours doesn't carry that yet, so a glide right after a jump starts from his current speed. I'll check it in KH2's code next.
>
> One question: after you let go, should the fall follow (a) your Physics setting, as it does now (KH1's air steering at Physics KH1), or (b) always KH2's fall steering once you've glided with KH2's Glide?

^m5


lets have (a) for now

also glide and super glide don't get removed anymore when setting glide to kh2, and kh2's glide growth ability isn't there.

I also noticed when sora turns to another bank after hes banking in a direction he resets his animation which doesn't seem like kh2. please fix 

go ahead


> [!tip] Assistant
> Thanks. (a) it is: after you let go, the fall keeps following your Physics setting, which is how it works now, so nothing changes there.
>
> **t574, the ability list.** I found why Glide and Superglide stayed and KH2's Glide wasn't there. The swap only ran once a second while the game was actually running Sora, so if you switched Glide to KH2 in the Config menu and looked at Abilities in the same pause, it hadn't run yet. Now it happens the moment you change the setting: KH1's Glide and Superglide go out and KH2's Glide (with its level, e.g. "Glide MAX") goes in, and back the other way at KH1.
> - Checked: the code path; both logs I have (yours and one from the PC) show the swap working once the game runs.
> - Not checked: that this is what you saw, since I don't have a log from that session. If they still show after you unpause and play for a second, please send that log.
>
> **t575, banking.** I went through KH2's glide code before changing anything. Every frame KH2 picks 205 (straight) or 206 / 207 (bank left / right). Whenever the pick changes, it starts the new motion from its first frame and blends into it over 16 frames. KH2 only keeps an animation's place when switching between walk and run, not in the glide, and all three glide motions are 1-second loops. So restarting the bank animation is KH2's own rule, and our glide asks KH1 for exactly that. In your earlier traces, every left / right change also went through one frame of straight glide as the stick crossed the middle, which KH2 does too.
>
> What I couldn't check is how KH2 blends when a new motion is asked mid-blend, compared with KH1. So t575 doesn't change the animation: it logs every glide motion change with what KH1 was playing and how far its blend was, so we can find exactly where it looks different.
>
> Please test:
> 1. Switch Glide to KH2 in Config, then open Abilities without unpausing: KH1's Glide and Superglide should be gone and KH2's Glide there. Then switch back to KH1.
> 2. Glide, bank hard one way, then sweep the stick to the other side a few times: once through the middle of the stick and once around the edge. A Select + Triangle trace during it plus the log would help most.
>
> Log lines: "abilities: Glide style ... set in the Config menu", "glide (native) motion: ...".
>
> Questions:
> 1. When he switches bank in KH2, does it look like (a) a smooth lean from one side to the other, or (b) the lean "starting over", only softer than ours? If (a), I'll compare KH2's blend with KH1's using your trace.
> 2. If KH2's rule still looks wrong in game: (a) keep KH2's rule and only fix the blend, or (b) keep the bank animation's place when switching sides? (b) isn't KH2's rule; KH2 only does that for walk / run.
> 3. Were you in the paused Config menu when you saw Glide and Superglide stay: (a) yes, or (b) no, they were still there in normal play?

^m6

