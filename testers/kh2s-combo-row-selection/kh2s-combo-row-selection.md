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
- [11. Mr Bugs: I've sent you the traces and log, I assume you don't need my](#^m11)
- [12. Assistant: t431 to t436 (one change each; the build is on GitHub as kh1](#^m12)
- [13. Mr Bugs: 1. it should cancel his animation like kh1, not sure make su](#^m13)
- [14. Assistant: t457 to t460 (one change each).](#^m14)
- [15. Assistant: t473 to t476 (one change each).](#^m15)
- [16. Assistant: t488 and t489.](#^m16)

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
> I've sent you the traces and log, I assume you don't need my input from this point so try to transfer everything into kh1's systems like you've been doing and looking through the steam c decomp, I'll give you the go ahead on the check. Just let me know what you changed how the behaviours are different after so I can debug and make sure it works
> 
> go ahead
> 

^m11


 


> [!tip] Assistant
> **t431 to t436** (one change each; the build is on GitHub as kh1-kh2.dll).
>
> Your ledge traces are good (t431, notes only): next to the edge he hops 21 high and goes forward off it, so t415 works. In the two where he walked off without a hop, he was about 89 from the edge, so KH2's 85 line still found floor and picked a normal swing. That swing's own lunge then carried him off, which I think KH2 does too. There were no steps or ramps in these traces, so that part is still unchecked.
>
> What changed (with Base / Aerial Combo on KH2; on KH1 these still use the old KH2 handoff):
> 1. **t432:** Slapshot, Dodge Slash and Slide Dash now run on KH1's own ground combo like the normal swings. KH2's animation and lunge (Slide Dash always goes 700), KH1's damage. There's no handoff to KH2 anymore, so no flip in the trace.
> 2. **t433:** Aerial Sweep from the ground is KH1's air state with KH2's hop numbers, and your air combo goes on after it.
> 3. **t434:** Moves that float him in KH2 now float him in KH1's air state too. Aerial Dive is native and stalls near the end.
> 4. **t435:** Aerial Spiral and Aerial Finish run on KH1's air combo, with KH2's hop and float (Aerial Finish hovers a long time before its last hit).
> 5. **t436:** Flash Step is native. During its first 19 frames, hits on him do nothing (a clang), like KH2. It needs `hurt` on in the ini.
>
> Checked against the KH2 decomp, the game files and your traces; none of it is tested in game yet. Still on the old handoff: Vicinity Break, Explosion, Magnet Burst, Guard Break, Counterguard, Finishing Leap, Upper Slash, Horizontal Slash, Retaliating Slash and Aerial Recovery.
>
> Questions:
> 1. In KH2, when a ground swing's lunge takes Sora past a ledge, does he (a) keep swinging as he falls, or (b) stop and fall like here?
> 2. KH2 turns Sora toward a locked enemy during parts of a swing; KH1 only aims at the start. (a) add KH2's turn next, or (b) leave it?
> 3. Next: (a) Vicinity Break / Explosion (KH2's body hit areas), or (b) Aerial Recovery and Retaliating Slash (new KH1 animations for KH2-only moves)?
>
> A Select + Triangle trace and the log with each of these moves equipped would help, plus one combo swinging up some steps.

^m12


> [!note] Tester Mr Bugs
>1. it should cancel his animation like kh1, not sure make sure to look at steam kh2 ghidra decomp 
>2. (a)
>3. Transfer everything that is not on the new system yet.
>   
> after you do that we need to figure out the logic for how abilities that affect soras combos work like finishing plus, I was thinking if sora has an ability like finishing plus and does a kh1 base combo then he gets the same logic as kh2 that decides what move he should do. tell me what you think
> 
> go ahead

^m13


> [!tip] Assistant
> **t457 to t460** (one change each).
>
> 1. **t457, the ledge:** I checked KH2's decomp. Normal swings work like KH1: if a swing takes Sora off the edge, KH2 cancels it into his fall and keeps his speed, capped at 8 a frame. KH1 already does exactly that, so no change there. But KH2's finishers (154 / 155), Flash Step's start and Guard Break have a hidden "stop at the edge" window. During it KH2 keeps him on the ledge and the swing carries on. That's in now. It checks 18 past his step for floor, a number I read from your KH2 exe.
> 2. **t458, (a):** he now turns toward the locked or auto-targeted enemy during each swing's turn windows, at KH2's speed. That's up to twice the window's degrees per frame (the old KH2 source we used only did half that), and only when the enemy is more than 50 away.
> 3. **t459:** Combo Plus / Negative Combo with Base / Aerial Combo on KH2 no longer hand off to KH2's controller. KH1's own rows do the longer or shorter combo.
> 4. **t460:** with Base / Aerial Combo on KH1, the ability moves already native (Slapshot, Dodge Slash, Slide Dash, Flash Step, Aerial Sweep, Aerial Dive, Aerial Spiral, Aerial Finish) now play on KH1's rows too. The swings around them stay KH1's.
>
> **Check:** the KH2 decomp, your KH2 exe and the KH2 files. Not checked in game. One small difference left: KH1 switches to the fall animation 2 frames after he leaves the floor, KH2 at once.
>
> These I couldn't do 1:1 without your call:
> 1. Vicinity Break / Explosion / Magnet Burst hit with huge oval body shapes, but KH1's attacks are only spheres. (a) KH2's exact shapes and hit numbers, landed through KH1's damage, or (b) KH1 spheres, which means different reach and no Magnet pull?
> 2. Square moves (Finishing Leap, Upper Slash, Horizontal Slash, Counterguard) and Guard Break: (a) run them on KH1's existing swing/finisher rows (KH1's damage data, so Upper Slash loses KH2's launch), or (b) add new KH1 rows for them with their own data?
> 3. Aerial Recovery / Retaliating Slash need a KH1 animation request KH1 doesn't have: (a) borrow one of KH1's, or (b) add new ones?
> 4. Finishing Plus's next finisher: (a) use KH1's spare ability-finisher rows as the second finisher, or (b) a new row?
>
> On how abilities should affect combos, my honest opinion: I'd keep KH1's own move picking for a KH1 combo and only bring over KH2's **combo rules**: the length (Combo Plus, Negative Combo), the finisher chain (Finishing Plus), and later Berserk Charge.
> - **Why not KH2's picker:** if KH2's picker took over whenever one of those abilities is equipped, a KH1 combo with Finishing Plus would become KH2's combo, with KH2's distance rules and dash picks, and the KH1 setting would stop meaning much. KH2's picks are also KH2 motions, and there's no clean way to map them onto KH1's swings.
> - **What KH1 already has:** a count against a combo length, and a few finisher rows. So Negative Combo can just shorten KH1's length, and Finishing Plus can chain into a second KH1 finisher.
> - **Trade-offs:** it's less KH2 under the hood, but each setting stays itself and it's simpler and lighter.
> - **Ability moves are separate:** Slide Dash and the like are KH2's moves at either setting, which is what t460 does.
>
> 5. So: (a) KH2's combo rules on KH1's own picker (my pick), (b) KH2's picker for everything whenever a KH2 combo ability is equipped, or (c) keep handing those cases to KH2's controller for now?

^m14


1. (a)
2. (b)
3. (b)
4. (b)
   5. I think I would go with kh2s picker (if kh2s picker is a native implementation and not our old way of doing it) The reason being is that when a player picks a kh2 move it needs to look and behave like kh2. So, messing with any of the values could feel off. I wanna give your take, would it actually be better to have b but then have kh1s rules inside of b instead? 
   
   you can do 1. 2. 3. and 4. but answer my question on 5.
   
   go ahead


> [!tip] Assistant
> **t473 to t476** (one change each).
>
> 1. **t473:** Vicinity Break, Explosion and Magnet Burst now run on KH1's own rows. Their hits use KH2's exact body shapes and hit numbers (Magnet's pull, Explosion's launch), landed through KH1's damage. KH2's invulnerability (like Explosion's 32..64) now blocks KH1's hits on him.
> 2. **t474:** Upper Slash, Finishing Leap, Horizontal Slash and Guard Break each have their own KH1 row and request (from 0x200), with their own attack data, so Upper Slash launches with KH2's numbers. Square in the combo window now goes through KH1's own Attack step. Finishing Leap rises by its own motion, as in KH2.
> 3. **t475:** Aerial Recovery and Retaliating Slash are new KH1 requests in KH1's air state, using KH2's hops (Aerial Recovery 100 high, 10 a frame along the stick). Control comes back at frame 24, as in KH2.
> 4. **t476:** Finishing Plus's next finisher plays on a new KH1 row. It's now taken inside KH2's own finisher window, and the number of extra finishers follows KH2's count (before, it had no limit).
>
> **Check:** the KH1 exe and decomp for the rows and the Attack step, the KH2 files for the moves' ranges and attack data. Not checked in game. I don't have the Keyblade's motion file (`obj/W_EX010.mset`), so its hit windows are only read in game.
>
> **On 5:** yes, KH2's picker is native now. Since t374/t390 it doesn't take Sora over: when you press attack, KH1's own Attack step runs, KH2's selection rule only decides which KH2 motion KH1's row plays, and KH1 runs the swing.
>
> What you described as (b) is basically what Base Combo KH2 already does: KH2's picker, KH1's count, KH2's combo length. So (b) at the KH1 setting means "this combo becomes the KH2 one whenever a KH2 combo ability is equipped".
>
> "(b) with KH1's rules inside" would be: KH2 picks which swing plays, but KH1 decides how long the combo is and when the finisher comes. I'd advise against that one. KH2's picker decides finishers by its own length rule, so I'd have to override that rule. The result would match neither game, and it's exactly the "messing with values could feel off" case you mentioned.
>
> Questions:
> 1. Combo abilities at the KH1 setting: (a) KH1's picker with KH2's combo rules on top, (b) KH2's picker and KH2's rules for any combo where a KH2 combo ability is equipped, so it plays as Base Combo KH2 (my pick, given what you said), or (c) KH2's picker inside KH1's rules?
> 2. If (b), which abilities switch it? (i) only the ones KH1 doesn't have: Finishing Plus, Negative Combo, Berserk Charge later, or (ii) Combo Plus, Air Combo Plus and Combo Master too? KH1 has those itself, so most saves would end up on KH2's combo.
> 3. Counterguard comes out of KH2's Guard, which still runs on KH2's controller: (a) make Guard native next, then Counterguard runs on its own KH1 row, or (b) leave Counterguard as it is for now?
> 4. Guard Break goes through enemy guards by KH1's own rule (a high attack priority), because I couldn't read KH2's exact guard-break rule here: (a) keep it like that, or (b) send me `obj/W_EX010.mset` and I'll check KH2's version?
>
> A Select + Triangle trace and the log with these moves equipped would help, mainly the lines starting with "KH2's hits on his body", "KH1's attack rows", "KH1's row of ours hit" and "KH2's Finishing Plus window".

^m15


""(b) with KH1's rules inside" would be: KH2 picks which swing plays, but KH1 decides how long the combo is and when the finisher comes. I'd advise against that one. KH2's picker decides finishers by its own length rule, so I'd have to override that rule. The result would match neither game, and it's exactly the "messing with values could feel off" case you mentioned."

1. go with the recommended choice you had then
2. yes only the ones kh2 doesn't have
3. (a)
4. (a) but read the mset to make sure, you should have access to it 

go ahead


> [!tip] Assistant
> **t488 and t489.**
>
> 1. **t488 (answers 1 and 2).** With a combo ability KH1 doesn't have equipped (Finishing Plus or Negative Combo, and Berserk Charge once it's in the ability list), your combo plays exactly like Base / Aerial Base Combo KH2, even with those set to KH1: KH2's row picker and KH2's combo length and finisher rules, on KH1's own rows. Combo Plus, Air Combo Plus and Combo Master don't switch it, since KH1 has those. The Config row still shows what you picked, and taking the ability off puts your setting back. The log says when it switches ("a combo ability only KH2 has is equipped").
> 2. **t489, Guard native (3a).** With Dodge Roll on KH2, Square standing still with Guard equipped now runs KH2's guard on KH1's own states (no handoff):
>    - It plays KH2's guard motion (173) and turns to face your lock-on.
>    - It blocks hits from the front during KH2's frames 6 to 37, the Keyblade guard window in the files: no damage, KH1's clang.
>    - A block plays KH2's repel (motion 42) and pushes you back 200, as in KH2.
>    - You can move out of the guard from KH2's free frame (52), and out of the repel from 40.
>    - **Counterguard:** press Attack during the repel, on the ground, with Counterguard equipped. It runs on its own KH1 row, with KH2's hit, invulnerability (frames 0 to 20) and body guard.
>    Checked: the motions in your KH2 files and the free-frame rule in the KH2 decomp. Not tested in game.
> 3. **Guard Break (4a).** I read `W_EX010.mset`. Its hit (ATKP 876) is attack kind 1, which OpenKH calls "pierce armor" (the guard break); your normal swings are kind 0. So KH1's go-through-guards rule stays, no code change. It also has its own guard on the Keyblade during frames 5 to 42, and with t489 that works too (front hits are blocked while it winds up). One assumption: I didn't find KH2's own use of kind 1 in its clash code.
>
> Please test, with Select + Triangle traces and the log: Finishing Plus or Negative Combo equipped with Base Combo on KH1; guarding enemies from the front and from behind; a block, then Counterguard; Guard Break into a guarding enemy. Useful log lines start with "move style:", "Square standing: KH2's guard", "KH2's guard stopped a hit", "Attack in KH2's guard repel" and "Sora hit by ... KH2's guard on the Keyblade".
>
> Questions:
> 1. Guarding in the air: KH2's repel is ground-only. (a) a block in the air just stops the hit, as now, or (b) look into KH2's air recoil next?
> 2. Square again during the repel: KH2 lets you guard or roll again after the repel's first frames; here KH1 doesn't take Square until the repel's free frame (40). (a) add KH2's earlier window next, or (b) leave it?

^m16


