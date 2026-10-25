# Sora's hit reactions native

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)
- [2. Assistant: t506 to t508, the first batch. Everything below runs on KH1'](#^m2)
- [3. Assistant: t550 and t551. Your 3 needed no code: flight and swimming al](#^m3)

---


> [!tip] Assistant
> I started this thread from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md), as you asked. This first message is the research.
>
> **What we have now.** When Sora is in KH1's own states, KH1's hit pass (`0x14029BBD0`) plays KH1's reactions. When KH2's controller has him (borrows, Drive Forms), the hook keeps KH1's damage, cancels KH1's reaction, and has the controller play KH2's flinch or blow.
>
> Canceling it costs more than I thought. When no KH1 reaction runs, the pass stops early, so these are lost:
> - KH1's 30-frame no-damage timer after a hit
> - the HUD portrait's flash
> - MP from damage
> - the "low HP" voice
>
> Our own re-hit grace was patched in to stand in for the missing timer.
>
> **What KH2 does** (KH2 Steam decomp):
> - Players and enemies share one reaction routine, `0x1403D2FE0`.
> - It folds the ATKP reaction byte into groups (`0x1403CE980`): push, flinch, flinch with hit-back (or the air flinch, motions 14 / 15), and launch (motion 16 from the front, 18 from behind, then 17 / 19 on landing), plus Magnet.
> - The hit-back distance comes from the attack's ATKP, or sstm's 200 when it has none.
> - The push is 0.1 of the distance a frame, ×0.9 each frame.
> - A launch is a real blow arc: up 24, 40 sideways, brakes 0.8, KH2's gravity.
>
> All of that matches our KH2 rewrite.
>
> **What KH1 does** (KH1 Steam decomp):
> - Kind 1 hits become a flinch (3) or a knock-back (4 / 5) by how much of his max HP the hit took: under 20%, 20..29%, 30% and up (btltbl's numbers).
> - The reaction comes from a table by movement state: ground, air, flight, swimming and a few others.
> - Ground flinch: requests 0x48..0x4B, pushed back 750 × the damage's share of max HP, capped at 150, on KH1's own push curve (0.223 a frame, ×0.8).
> - Ground knock-back: a knock-down motion (0x4C / 0x4D) with no launch.
> - Air knock-back: a hop of 20 and 16 sideways.
> - Every reaction also starts KH1's 30-frame no-damage timer (actor flag 0x80).
>
> **The plan, one system per version:**
> 1. KH2's reaction motions on KH1's reaction requests, through the baked slots (10 / 12, 14 / 15, 16..19, the KO). KH1 keeps the timer, HUD, MP, voices and tallies because KH1 still runs the reactions.
> 2. KH2's push on KH1's push: KH2's distance and its 0.1 / 0.9 curve, for Sora only.
> 3. KH2's blow as a new KH1 movement state (KH1 has no launch for Sora). KH2's arc and landing, and Aerial Recovery / Retaliating Slash read it.
> 4. Borrows and forms go through the same pass. When a hit lands, Sora is handed to KH1's states for the reaction, and the controller's own reaction and our re-hit grace go away. Guard, Reflect and invulnerability still stop hits before the pass, as now.
>
> No new per-frame cost: the hit pass is already hooked, and step 4 removes a parallel system.
>
> **What I couldn't find yet:**
> - KH2's own rule against Sora being hit again while he's reeling (I haven't located it in the Steam decomp).
> - What lifts Sora into the air in KH1's ground knock-back.
> - A second Second Chance rule tied to a flag I haven't traced (`0x140528ADC`).
>
> Notes: `docs/kh1-hit-reactions-native.md`.
>
> Questions:
> 1. KH1's enemy attacks don't carry KH2's knock-back numbers. For the push and launch, should we use (a) KH2's defaults for every hit (hit-back 200, blow 24 / 40), (b) KH1's own per-hit numbers (750 × the damage's share, cap 150) on KH2's curve, or (c) KH2's defaults scaled by KH1's damage share?
> 2. KH1's middle kind (4, a hit of 20..29% of his HP): (a) a KH2 blow, as now, or (b) a KH2 flinch, with only kind 5 (30% and up) launching him?
> 3. After a hit, should the no-damage time be (a) KH1's 30 frames, or (b) KH2's own rule once I find it in the KH2 decomp, with KH1's timer kept only if they match?

^m1


1. unsure make it a switch in the config and when I play test it more I can figure out what to do, still make it native of course. your judgement on how to do it 
2. how sora receives knock back should also be a config
3. if the rules are different make it part of the switch in the config for sora knockback, if not keep the same. 

I tested guard break and it using kh2s collisions (what ever causes a recoil) and not kh1s is not working properly in the engine, we need a separate thread investigating how to make kh2s recoil system work properly, right now it doesn't behave the same so you can't parry enemy attacks from what I remember. 

when it comes to drive forms its not yet native so its deprecated right now, don't worry about it til we start working on drive forms

go ahead 
> [!tip] Assistant
> **t506 to t508**, the first batch. Everything below runs on KH1's own hit pass, so KH1 still handles the 30-frame no-damage time, the HUD flash, MP, voices and tallies. Drive Forms aren't touched, as you said.
>
> - **t506, Hit Reactions row** (Config > Combat, KH1 / KH2). On KH2, Sora plays KH2's flinch animations on KH1's own reaction slots: 10 to 13 on the ground, 14 / 15 in the air. He can act again from KH2's frame 50. I checked in both decomps that KH1's "front" flinch is the same as KH2's front.
> - **t507, KH2's push, and the Knock-back Numbers row** (your 1, my call). The push now uses KH2's curve (start at 0.1 × the distance, ×0.9 each frame), through KH1's own push and collision. In the air it's KH2's hop of 50 and its drift. The row picks the distance:
>   - KH1: KH1's own number for each hit;
>   - Scaled: KH2's 200 scaled by the damage;
>   - KH2: 200 for every hit.
>   The default is KH2, because that's what KH2 itself does when an attack has no knock-back number, and KH1's attacks never have one.
> - **t508, Medium Hits row** (your 2). Blow / Flinch. On Flinch, KH1's middle hits (20 to 29% of your HP) only flinch you, and only 30% and up knock you away.
> - **Your 3, the re-hit rule:** both games give 30 frames of no damage after the reaction ends. The one difference is that KH2 still lets hits from the same attack group through, and KH1's attacks have no groups. So I kept KH1's timer, with no switch.
>
> **Still to do:** KH2's blow (the launch, and the landing into 17 / 19) as a new KH1 state, and the moves that still borrow KH2's controller going through the same pass.
>
> The recoil and parry problem you found with Guard Break has its own thread now: [KH2 recoil and parry system](../kh2-recoil-and-parry-system/kh2-recoil-and-parry-system.md). The research is there, including a bug that explains why you can't parry.
>
> Please try: hits from the front and from behind, on the ground and in the air, with each Knock-back Numbers value, and Medium Hits on Flinch.
>
> Questions:
> 1. With Battle Quotes on KH2, should Sora's hurt line be (a) KH1's, as now, or (b) KH2's?
> 2. In an air flinch, KH2 turns Sora toward the attacker (or away, if he was hit from behind). Should we (a) do that too, or (b) leave his facing alone, like KH1?
> 3. Flinches while flying or swimming: (a) KH2's numbers too, or (b) keep KH1's?

^m2


1. (b)
2. (a)
3. (b)

go ahead


> [!tip] Assistant
> t550 and t551. Your 3 needed no code: flight and swimming already kept KH1's numbers, so I just checked it.
>
> - **t550, KH2's hurt line.** With Battle Quotes on KH2, Sora says KH2's line when he's hit, the same line KH2's reaction motion says:
>   - voice 29 on a flinch;
>   - voice 30 on the alternate flinches and an air flinch from behind;
>   - voice 1 on a knock-back.
>
>   It plays through KH1's own voice path at the moment KH1 would say its line, and KH1's line is skipped. The low-HP line and the knocked-out line stay KH1's, because KH2's reaction motions don't have them. In the moves that still borrow KH2's controller, KH1's line was playing on top of KH2's. Now only KH2's plays.
>   Check: the voice numbers come from your P_EX100.mset and 00battle.bin, and KH1's voice call from the KH1 decomp's hit pass.
> - **t551, the air flinch turn.** With Hit Reactions on KH2, an air flinch now turns him at once, as KH2 does: toward the attacker if he was hit from the front, away from him if he was hit from behind. It uses the same turn KH1's own knock-back does. KH2's ground flinch doesn't turn him, so that's unchanged.
>   Check: the KH2 decomp's air and ground flinch, matched to the KH2 rewrite, and KH1's knock-back turn. One assumption: Ghidra drops one argument of KH2's vector copy, and I read it as the hit direction, which is what the rewrite does.
> - **Your 3, flight and swimming:** these already used KH1's numbers. KH2's push and hop only run on the ground flinch and in his normal air state. t551's turn is limited the same way.
>
> Please try:
> - Battle Quotes on KH2: get hit from the front, from behind and in the air, and take a big hit. Listen for KH2's line, and make sure only one line plays.
> - Hit Reactions on KH2: get hit in the air from the front and from behind, and check which way he ends up facing.
>
> Questions:
> 1. With Hit Reactions on KH1 but Battle Quotes on KH2, KH1's reaction animation now plays with KH2's line. Should it be (a) KH2's line, as now (the Battle Quotes row decides), or (b) KH2's line only when Hit Reactions is also on KH2?
> 2. Flinches while flying still play KH2's air-flinch animation, because KH1 asks for the same animation there. Should flight (a) keep KH2's animation, or (b) go back to KH1's?
> 3. Medium Hits on Flinch also turns medium hits into flinches while flying and swimming. Should it (a) apply there too, or (b) apply only on the ground and in normal air?

^m3


