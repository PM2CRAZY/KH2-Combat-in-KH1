# KH2's combo row selection

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: In the KH2S model rendering there was a suggestion to implem](#^m1)
- [2. Assistant: t374](#^m2)
- [3. Mr Bugs: 1. do option (a)](#^m3)
- [4. Assistant: t390 to t393 (one part each; the build at the top of the bra](#^m4)
- [5. Mr Bugs: sorry I should've been more clear; don't worry about moves t](#^m5)

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


