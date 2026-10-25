# Air physics for KH2 magic, Aerial Finish and Magnet Burst

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from your message in KH2 magic and Ref](#^m1)
- [2. Assistant: Thanks! Going ahead with your answers: (b) for question 1, D](#^m2)

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
