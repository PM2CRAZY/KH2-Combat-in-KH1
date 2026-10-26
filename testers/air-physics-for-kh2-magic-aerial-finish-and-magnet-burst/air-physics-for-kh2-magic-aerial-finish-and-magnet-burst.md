# Air physics for KH2 magic, Aerial Finish and Magnet Burst

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from your message in KH2 magic and Ref](#^m1)
- [2. Assistant: Thanks! Going ahead with your answers: (b) for question 1, D](#^m2)
- [3. Assistant: Sorry for the confusion, and thanks for sticking with it.](#^m3)
- [4. Assistant: Thanks for the videos, they settled it, and you were right a](#^m4)

---


> [!tip] Assistant
> I started this thread from your message in [KH2 magic and Reflect native](../kh2-magic-and-reflect-native/kh2-magic-and-reflect-native.md): "I need a new thread for an agent to start working on air physics for kh2's magic as well as aerial finish and magnet burst, its still not 1:1 with kh2". Research only so far, no code. Traces and logs for this go in `testers/air-physics-for-kh2-magic-aerial-finish-and-magnet-burst/`.
>
> **What KH2 does** (Steam KH2 decomp and your KH2 files)
> - There's no separate attack or cast state in the air. Every air move, a cast, Aerial Finish or Magnet Burst, plays as an animation inside KH2's jump state, and that state's step (`0x1403CB360`) moves him the same way every frame:
>   - the jump arc's clock always runs;
>   - while the animation has a no-gravity range, his height is held, but the clock keeps running, so when the hover ends he's long past the top and drops at once at the fall cap (16 a frame);
>   - sideways, his speed fades ×0.95 unless something steers him;
>   - landing mid-move cuts it into the landing (none of these animations has the range that would let it go on).
> - **Air casts** (`0x1403C5170`): a fresh 40 hop from wherever he is, rising or falling, his sideways speed kept and fading, then the cast's hover: Fire / Reflect frames 16–74, Blizzard 16–66, Thunder / Cure 16–58.
> - **Aerial Finish** (motion 194) and **Magnet Burst** (motion 197) start with their combo row's hop (`0x1403D78D0`):
>   - Aerial Finish: with a target, a height of 14–114 by how far above you it is, and a speed of the gap ÷ the rise time, up to 18 a frame; with no target, a 14 hop at 18 a frame along his facing. Hover 16–100 of 116, then the drop at 16 a frame.
>   - Magnet Burst: always a 14 hop with no sideways speed, so he stops. Hover 16–82 of 90.
>
> **What this branch does now**
> - **Base Sora's air casts** run on KH1's own cast state, and the **Air Physics** row decides: at KH2 it's the list above (40 hop, kept speed ×0.95, the hover with the clock running, the drop at the cap, landing cuts it); at KH1 it's KH1's 30 hop and half speed.
> - **Base Sora's Aerial Finish and Magnet Burst** run on KH1's air combo row with KH2's hop for the row. They follow the **Physics** row, not Air Physics. At Physics KH1 they use KH1's gravity on that hop and the hover freezes the clock, so out of it he eases into the fall (starting at about 4 a frame, capped at 20) instead of dropping at once at 16; the hop also tops out a little sooner (7.6 frames against KH2's 8.3).
> - **Drive Forms, and any move KH2's controller still borrows Sora for**, aren't KH2's: an air cast stops his sideways speed, falls from rest with no 40 hop, and its hover freezes the clock, so the fall after it starts gently again.
>
> So if your Physics row is on KH1, that alone would make Aerial Finish and Magnet Burst feel different from KH2 in the air.
>
> **Not checked yet (assumptions)**
> - That nothing steers him with the stick during these moves. None has an "allow movement" range, but I haven't found where KH2 clears the stick's pull.
> - That KH2's ×0.95 fade (and the fall cap) is per 30 Hz tick, as we take it. If it's per 60 Hz frame, he loses speed twice as fast in KH2.
> - That the jump state's fall cap is the 16 from your `03system.bin`.
>
> **Plan, one system per version**
> 1. Settle those three in the decomp, then fix whichever is wrong for base Sora's casts, Aerial Finish and Magnet Burst.
> 2. Make Aerial Finish and Magnet Burst follow the row you pick (question 1).
> 3. Make the air cast and hover of the moves KH2's controller still runs (Drive Forms, borrows) KH2's own, until the forms move onto KH1's states (the Drive Forms native thread).
>
> A trace would help a lot: one air Fire, one Aerial Finish and one Magnet Burst, each started from the top of a jump, with Movement & Physics, KH1 State, Animation Draw and Log Lines on. Tell me which setting the Physics and Air Physics rows were on and what looks wrong.
>
> Questions:
> 1. Which row should decide Aerial Finish and Magnet Burst in the air? (a) the Physics row, as now, (b) the Air Physics row, so it covers air casts and these two finishers, or (c) always KH2?
> 2. Drive Forms' air casts: (a) fix KH2's controller now so forms fall like KH2 meanwhile, or (b) leave them until the forms run on KH1's states?
> 3. What looks wrong to you: (a) the height or the hover, (b) the sideways speed or the stick, (c) the fall after, or (d) the landing? Or just send the trace and I'll find it.
>
> Say go ahead with your answers and I'll start with step 1.

^m1


1. (b)
2. don't worry about drive forms, that's getting transferred over to the new systems once we get base sora done. 
3. (c) sent a trace


go ahead

> [!tip] Assistant
> Thanks! Going ahead with your answers: (b) for question 1, Drive Forms left for the new systems, and your trace for the fall.
>
> **t617, Aerial Finish and Magnet Burst follow the Air Physics row:**
> - These two now follow Combat > Magic > Air Physics instead of the Physics row, the same as the air casts.
> - At KH2 they get KH2's hop on KH2's gravity, then the hover with KH2's clock running, then the drop at 16 a frame as soon as the hover ends.
> - At KH1 they get KH1's gravity, a hover that pauses the clock, and KH1's gentler fall after it.
> - The Physics and Air Physics help lines say so now.
> - Check: in KH2's decomp both moves run on the same jump-state step as the air casts, so one row for all of them is how KH2 groups them. I couldn't check them against a trace (yours only has Reflect) or in game.
>
> **The assumptions from my last message**, read again in KH2's decomp (notes only, no code change, because the mod already did it this way):
> - the fall cap really is the 16 from your 03system.bin: Sora's setup copies it into the value the jump step caps with;
> - the ×0.95 sideways fade is applied once per 30 Hz tick, as we do;
> - the cast's 40 hop starts from his current height, even mid-jump;
> - the stick during a cast is still unconfirmed: I found what writes KH2's stick target, but it's driven by a script, so I can't yet say whether it's cut off during a cast.
>
> **Your trace (the fall):** both air Reflects match KH2's numbers frame by frame: 40 up over 14 frames, then a tiny dip; held from frame 16 to 73; 16 a frame down from frame 74 to the floor; sideways fading ×0.95 a tick. So I didn't change the fall's speed or timing. What isn't KH2 there is the look: with Jump set to KH1, KH1's own fall animation plays after the cast, then KH1's landing, where KH2 plays its own fall loop and lands into its own landing.
>
> Could you test:
> - one Aerial Finish and one Magnet Burst from the top of a jump, with Air Physics at KH2 and again at KH1, with a trace;
> - an air Fire with Jump set to KH2, to see if the fall then looks right to you.
>
> Questions:
> 1. What looks wrong about the fall after a cast? (a) it drops too fast, (b) it drops too suddenly (KH2's drop starts at full speed the moment the hover ends), (c) the animation (KH1's fall because Jump is KH1), or (d) something else (please describe it)?
> 2. If it's (c): should KH2's fall loop play after KH2's air casts even with Jump at KH1? (a) yes, KH2's fall after a KH2 move, or (b) no, the Jump row decides every fall.

^m2


I misunderstood, abilities should follow the main physics config.

your still not understanding me, I need you to look through the steam c decomp again because the same issue is still here:
I cast air magic > air magic keep me suspended in the air > after its done suspending me I fall straight down (not like kh2)

keep going until you find the problem 

go ahead

> [!tip] Assistant
> Sorry for the confusion, and thanks for sticking with it.
>
> **t621: Aerial Finish and Magnet Burst follow the main Physics row again.** It's back to exactly how it was before t617, help texts included; the Air Physics row only covers air casts now. Check: a straight undo of t617's code. Not checked in game.
>
> **The fall after an air cast.** I went through KH2's Steam decomp again, line by line this time, not just the numbers: everything that moves Sora each frame while an air cast plays (the order of a frame, the stick, the clip's root motion, the vertical step and the cast's end), plus the clips in your KH2 files. Here's what KH2 does:
> - **The stick:** KH2 reads it every frame, then throws it away unless the animation has a "free movement" or "allow movement" window. The air-cast animation has neither, so the stick does nothing during the whole cast. KH2's fall loop after the cast does have one, so the stick steers him again only once the cast animation is over (frame 84).
> - **Sideways speed:** the speed he had when he cast fades ×0.95 every two frames. The cast animation has no root motion, so nothing else moves him sideways.
> - **Height:** he's held in place from frame 16 to 74 while the jump's clock keeps running, and nothing resets that clock when the hold ends. So on the very first step after frame 74 he's already falling at KH2's top speed of 16 a frame. Unless he still has sideways speed, that's straight down.
>
> Your trace does exactly this, frame for frame, so I couldn't find a physics difference, and I didn't change it (the findings are in the notes, `docs/kh1-magic-native.md` §4n). The one thing in your trace that isn't KH2 comes after frame 84: KH1's own fall and landing animations play there (your Jump row is on KH1), where KH2 plays its own fall loop and landing.
>
> I don't want to guess, so could you help me see what you're seeing in KH2?
> 1. Could you record a short clip in KH2 itself of an air Fire (or Reflect) from the top of a jump, and the same in KH1, so I can compare frame by frame?
> 2. In KH2, what does Sora do after the hover that ours doesn't? (a) he keeps drifting forward, (b) he starts falling slowly and speeds up, (c) he hops up a little first, (d) the stick moves him while he falls, or (e) something else (please describe it).
> 3. Were you holding the stick when you saw it, and in which direction?

^m3


I sent you the videos, both are titled by their games abbreviation kh1 and kh2 and are in the same folder as this thread. if your narrowing your search to just how magic is handled and can't find anything feel free to step outside and look for more answers in the kh2 c steam decomp. 

> [!tip] Assistant
> Thanks for the videos, they settled it, and you were right all along. I went through both frame by frame, measuring Sora's feet against his shadow. In KH2, after the hover ends (cast frame 74), Sora starts falling from a standstill and speeds up, landing about half a second later (around 31 frames). Ours dropped at full speed the moment the hover ended and hit the ground in about 8 frames. That's the "falls straight down".
>
> I'd been looking in the wrong place. The jump code does keep its clock running during the hover, but KH2's player update has a separate check: on the first frame after a hover ends, it restarts the fall from his current falling speed. After a hover that speed is zero, so he falls from rest. It's `0x1403B4780` in the Steam KH2 decomp, reached through Sora's own update (`0x1403DAD10`, his class's vtable `0x1405CBA28`, confirmed in the exe).
>
> **t653** does the same: when a KH2 hover ends in the air, his fall restarts from rest with KH2's gravity, and his sideways speed carries on as before. It applies to every move with that hover at Physics KH2: air casts, and also Aerial Finish, Magnet Burst, Aerial Dive and Finishing Leap.
> - Checked: the KH2 decomp (the whole call chain), the exe (Sora's vtable, the jump action's flags), and your KH2 video (the fall's curve).
> - Not checked: in game. Drive Forms aren't covered yet, since they still run on KH2's controller. I'm also taking the restart speed from his last frame's actual move, where KH2 takes the move it made; they only differ if a wall or floor cut that move.
>
> Please test:
> - An air Fire and an air Reflect from the top of a jump with Air Physics on KH2: the fall after the hover should feel like your KH2 clip.
> - Aerial Finish and Magnet Burst with Physics on KH2.
> - If you can, a trace with Movement & Physics and Log Lines on. The log says "KH2's hold ended on KH1's air state" when the hover ends.
>
> Questions:
> 1. Does the fall feel right now? (a) yes, or (b) something's still off (the speed, the landing, or the animation: KH1's fall pose plays if Jump is set to KH1).
> 2. Air Physics on KH1: (a) keep KH1's own way (no hover, KH1's fall), or (b) change something there too?

^m4


