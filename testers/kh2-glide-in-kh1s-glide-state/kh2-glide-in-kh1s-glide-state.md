# KH2 Glide in KH1's glide state

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)
- [2. Assistant: Here's what I think:](#^m2)

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


