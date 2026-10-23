# KH2's combo row selection

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: In the KH2S model rendering there was a suggestion to implem](#^m1)
- [2. Assistant: t374](#^m2)
- [3. Mr Bugs: 1. do option (a)](#^m3)
- [4. Assistant: t390 to t393 (one part each; the build at the top of the bra](#^m4)
- [5. Mr Bugs: sorry I should've been more clear; don't worry about moves t](#^m5)
- [6. Assistant: t400 and t401](#^m6)
- [7. Mr Bugs: sent the traces](#^m7)
- [8. Assistant: t410 (notes only, no change in how it plays)](#^m8)
- [9. Mr Bugs: 1. (a) thats how its supposed to work and when we transition](#^m9)
- [10. Assistant: t415 and t416](#^m10)

---


  

> [!note] Tester Mr Bugs
> In the KH2S model rendering there was a suggestion to implement "**KH2's row selection**, so the right swing plays for the target's distance. Today the swing order is fixed, which is the biggest feel difference."
> 
> I want you to start working on it to get it native in kh1 using the steam gihidra c decomp of kh1, if you need to compare with kh2's decomp (rust rewrite) do so, if not fallback to the gihidra c decomp of kh2's steam exe 
> 

^m1


> [!tip] Assistant
> t374
>
> KH2's row picker now runs on KH1's own ground combo. When you swing on the ground with Base Combo set to KH2, KH2's rule picks which swing plays: the distance, height and direction to the target, the combo count, the finisher rule, and KH2's "don't repeat the same swing" step. KH1 still runs the swing itself (its hits, damage and timing); it just plays the motion KH2 would pick there. No new hooks.
>
> **Something you'll notice.** With no target, the swings now go 151 ↔ 153 like real KH2, and the close-range finisher is 155. Before, they were fixed at 151 / 152 / 154, which were mapped by name, not by KH2's rule. Your logs show KH2's default rows really are 151 and 153, with 155 as the close finisher.
>
> **Check:** decomp. KH2's picker is checked in the Steam KH2 decomp (the selector at `0x140402D20` and its range, finisher and walk checks), and it matches the KH2 source's rule except for one thing I fixed: KH2 never lets the gap to the target go below 0, so a target closer than its own radius no longer fails every ranged row. KH1's combo step and ground picker are checked in the Steam KH1 decomp. Not checked: Sora's full row table (I don't have `00battle.bin` here, so I don't know every row's range), anything in game, and whether a picked swing's combo window lines up with KH1's next press. Notes are in `docs/kh1-anim-slots.md` §7.
>
> Questions:
> 1. **KH1's own rows still win in some cases:** its dash at a target 160 or more away, its jump at an enemy above you, and its ability finishers. So at range you get KH1's dash, not KH2's pick. (a) replace those with KH2's pick too (closer to KH2, KH1's dash goes), or (b) keep them as they are?
> 2. **KH2's rows that hop up to an enemy in the air** don't play yet. They need KH1's air state with KH2's arc. Want me to work out a design for that next?
> 3. **The air combo is still fixed** (181 / 182 / 183). Want the same picker there?
> 4. **Things KH2 does that our KH2 source didn't have**, not added yet: KH2 picks a different row when something blocks the way just ahead of Sora (my guess is a wall or obstacle), and there are two status bits that force or block finishers that I haven't identified yet. Want those looked into?
>
> A Select + Triangle trace of a few combos at different distances would help, along with the log lines that start with "KH2's row selection".

^m2


> [!note] Tester Mr Bugs
> 1. do option (a)
>  2. yes
>  3. yes
>  4. yes add that
>
>go ahead

^m3


> [!tip] Assistant
> t390 to t393 (one part each; the build at the top of the branch has them all)
>
> Your traces showed t374 working: with no target it goes 151 → 153 → 155, and with an enemy 120–400 away KH2 picks Flash Step (you have it equipped), so KH2 takes Sora for that move. The one wrong thing was KH1's own dash winning in trace f27098 (the lunge of about 280). I also read Sora's full row table from `00battle.bin` this time (37 rows); it's written up in `docs/kh1-anim-slots.md` §7.
>
> - **t390, option (a):** at Base Combo KH2, KH1's own rows give way: its dash, its ability finishers, and the row I'd called a jump at a target above, which is really KH1's attack when there's no floor ahead (it walks him off the edge). KH2's pick always plays.
> - **t391:** the air combo uses KH2's picker too (181 / 182, finishers 183 / 184), with KH1's air rows giving way the same way.
> - **t392:** KH2's swing that hops up at an enemy above now plays in KH1's own air state, with KH2's hop height and speed. It needs Aerial Combo on KH2 as well.
> - **t393:** the two KH2 status bits: one keeps finishers chaining, the other is Berserk Charge (no finishers while MP charges). Also, what I'd guessed was "a wall ahead" is really "no floor ahead": KH2 does a small hop swing off an edge. That's in too, tested with KH1's own collision sweep.
>
> **Check:** decomp (KH2's picker and its status bits, KH1's ground and air pickers and row runner) and the row table from the game files. Not checked in game. Assumptions: the hop heights come from the KH2 source's rule, not KH2's exe; which length KH2 uses for the "floor ahead" test; that the finisher-chain bit means what it seems to (I found nothing that sets it); and that KH1's direction for the press matches the direction to KH2's target.
>
> A trace near a ledge, one at a flying enemy, and some air combos would help.
>
> Questions:
> 1. Air swings: (a) KH2's own little hop on each air swing too (1:1), or (b) keep KH1's air lunge and hop under KH2's gravity, as now?
> 2. Berserk Charge needs KH2's MP Charge, which KH1 doesn't have: (a) add MP Charge and Berserk Charge to the ability list as their own system next, or (b) leave it off?
> 3. The hop swing with Aerial Combo set to KH1: (a) keep KH1's swing, as now, or (b) play KH2's hop anyway?

^m4


> [!note] Tester Mr Bugs
> sorry I should've been more clear; don't worry about moves that are still in kh2 entirely, that system is going to be deprecated for when we get everything implemented and 1:1 in kh1 
> 
> 1. (a)
> 2. don't worry about mp mechanics yet until we're ready to start working on it
> 3. (a) 
>    
>    go ahead

^m5


> [!tip] Assistant
> t400 and t401
>
> Understood on the KH2-only moves: I'm not spending time on the parts that hand Sora to KH2's controller.
>
> - **t400:** I checked t392's hop numbers against KH2's exe and they were off (they came from the old KH2 source's rule). KH2 measures the hop from Sora's own target point (about 70 up) to the enemy's, takes both bodies' sizes off the gap, adds each row's reach bonus (85 on all of Sora's hop rows), and times it on KH2's own fixed gravity. It's 1:1 with the exe now. For example, an enemy 120 up and 200 away used to get a 67-high hop; KH2's is 50.
> - **t401, (a):** every KH2 air swing now does its own little hop, rise and drift on KH1's air state, like KH2. With no target that's 21 high for 181 / 183 and 17 for 182, with the finishers moving faster. KH1's air lunge is gone there.
> - **3 (a):** that was already how it works. With Aerial Combo on KH1 the hop swing stays KH1's, so nothing changed.
> - **2:** left for later, as you said.
>
> **Check:** KH2 decomp (the hop at `0x1403D78D0`) and the game files for Sora's rows and his target shape. Not checked in game. Assumption: which of KH2's player classes is Sora's (several share the same code). One small thing: an attack pressed in the first frames of a jump, before KH1 counts you as airborne, still uses KH1's own air swing movement.
>
> A Select + Triangle trace of some air combos, with and without an enemy above you, would help.

^m6


> [!note] Tester Mr Bugs
> sent the traces 
> 
> do the check
> 
> go ahead 

^m7


> [!tip] Assistant
> t410 (notes only, no change in how it plays)
>
> Checked all 7 (they're t409, from their timestamps; physics KH1, Base and Aerial Combo on KH2). Everything in them matches KH2's rule:
> - No enemy above: 181 → 182 → 183. An enemy above: 182 → 181 → 183. An enemy far above, out of 182's reach: 181 → 182 → 183. The other finisher (184) never comes with one enemy, which is KH2's rule too: it needs a second enemy off to the side of the main one.
> - Each swing's hop is KH2's height for that row (21 / 17 / 21 with nothing above, 44 to 71 toward an enemy overhead, within each row's limits) and KH2's sideways speed, aimed the way he turns. The rise is on KH1's gravity because physics is on KH1.
> - KH1's own air rows give way where they should.
>
> Nothing to fix in the code. I only corrected a few notes and comments that had "above" and "below" swapped. You might have noticed him floating a couple of units high on some swings in the first area: that's KH1's collision sliding him over the enemy he's pushing into, not the combo (KH1's own jump does it there too).
>
> One trace (f41229) had a ground press at an enemy above give KH2's Aerial Sweep, because it's equipped and comes first; that's the KH2-only path you said to ignore.
>
> What these don't cover yet: ground combos, a ledge (the drop-ahead hop), and a log. A ground combo trace and a ledge one, with the log lines that start with "KH2's row selection", would let me check those too.
>
> 1. Aerial Sweep always wins over the plain hop at an enemy above while it's equipped. (a) you unequip it for a trace so I can check the plain hop from the ground, or (b) skip it?

^m8


> [!note] Tester Mr Bugs
> 1. (a) thats how its supposed to work and when we transition that move to native kh1 it will work the same 
>    
>    Okay I recorded it and sent a log I noticed that sora next to a ledge will start a short jump attack
>    
>    go ahead

^m9


> [!tip] Assistant
> t415 and t416
>
> Checked your log and the 4 traces. The short jump attack at the ledge is real KH2: when the floor gives out at the end of an 85-long line ahead of Sora, KH2 does a little hop swing off the edge instead of a ground swing, even mid-combo (that row comes first in Sora's table). In your traces he was 50 to 70 from a 120 drop each time, so KH2 would do it there too. The ground combos in the log are KH2's too: 151, 153, 151, then the finisher 155.
>
> Two things were wrong, though:
> - **t415:** the hop is meant to carry him forward off the ledge at 8 a frame, but KH1 resets his air speed from the stick on the frame he leaves the ground, so with the stick idle he just hopped in place (seven times in a row in one trace). That's fixed: he now goes off the edge with the swing, like KH2.
> - **t416:** the "no floor ahead" test was KH1's, not KH2's, and it ran along the stick's direction. KH2 only looks at the spot at the end of the 85 line and counts no floor, or a floor more than 24 below his feet, as a drop. It follows the slope under him and aims where KH2 aims (the target, or the way he's facing). It's KH2's now, so small steps, stairs and slopes shouldn't set it off.
>
> **Check:** your traces and log, the KH1 decomp (why the speed was wiped) and the KH2 decomp (the floor rule and the line's length, which confirms the 85 I'd assumed in t393). Not checked in game. Assumptions: the line is laid flat on moving platforms, and KH1's collision shape moves it rather than KH2's, so walls may stop it a little sooner or later.
>
> A Select + Triangle trace walking off a ledge with an attack, and one swinging near some steps or a ramp, would let me check both.

^m10


> [!note] Tester Mr Bugs
> 
