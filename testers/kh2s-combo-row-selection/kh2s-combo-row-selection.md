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
- [17. Assistant: I looked into both in KH2's Steam decomp and your KH2 files.](#^m17)
- [18. Assistant: t536 and t537.](#^m18)
- [19. Assistant: t557 and t558. You're right, I had it backwards: at the KH1 ](#^m19)
- [20. Assistant: You're right that there's still a seam. Here's what the code](#^m20)
- [21. Assistant: Got your picks. Here's how I read them:](#^m21)

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


1. (b)
2. make repel a switchable option in the config menu, for kh2 setting do option (a)
   
   I will get back to you on the logs
   
   go ahead


> [!tip] Assistant
> I looked into both in KH2's Steam decomp and your KH2 files. Both times KH2 turned out to do what we already do, so there's no new version this round. Details are in `docs/kh1-anim-slots.md` §7.
>
> 1. **Guarding in the air (1 (b)).** KH2 does have an air recoil: a small hop back, motion 43, 50 up and about 2.4 a frame backwards (`0x1403D61D0` → `0x1403D4650` / `0x1403D4800`). But it only plays when the guard isn't marked "the move goes on", and every guard Sora can have in the air is marked that way: Aerial Dodge, Aerial Recovery, Magnet Burst and motion 8 (I read every motion in your files). His only guards that repel are the guard press and Flash Step, and both are ground moves. So in KH2 a block in the air just stops the hit and he carries on, which is what happens here already. The air recoil only matters for his own air swings meeting an enemy's guard, which isn't this.
> 2. **Square during the repel.** I had this wrong last time, sorry. In KH2's code, guard, roll and Quick Run out of the repel all wait for the repel's free frame, 40 (`0x1403A81F0` → `0x1403BFB30`). That's the same frame you can leave it here now. The frame-8 window I mentioned only lets reaction commands through, not Square. So a Guard Repel row would have KH1 and KH2 settings that play exactly the same, and I didn't build it.
>
> Question:
> 1. For the repel row: (a) skip it, since it already plays like KH2's; (b) make the KH1 setting use KH1's own reaction when its guard blocks (no repel animation, no push back), so the row switches between KH1's block and KH2's (I'd research what KH1's guard does on a block first); or (c) did you want the switch to change something else?
>
> Logs and traces of guarding, on the ground and in the air, are still welcome whenever you have them.

^m17


(b)

also I noticed that changing the combat settings back to kh1 doesn't seem to change the row selection for sora, (double check to make sure I'm right) he still does kh2 moves in the air and on the ground and also what we were working on earlier was his transition from ground to air when hes by a ledge still behaves like kh2 (double check) 

go ahead

> [!tip] Assistant
> **t536 and t537.**
>
> 1. **t536, (b): Guard Repel row.** Config > Movement > Guard Repel, KH1 or KH2 (default KH2, so nothing changes until you switch it). It only matters with Dodge Roll on KH2, since that's when Square standing is KH2's guard.
>    - **KH2:** as now. A block plays KH2's repel (42), pushes you back 200, and Counterguard works out of it.
>    - **KH1:** a block gets exactly what KH1's own guard gets. I traced it in KH1's decomp, and it turned out different from what I'd assumed. KH1's guard isn't an unbreakable guard: it has your Keyblade's priority. So when it stops a hit, KH1 treats Sora's side as a clash, the same as when one of his swings gets clashed. He plays KH1's clash recoil (0x6E), gets KH1's 8 frames of no damage, and his combo count drops by one. If the attacker has the higher priority he's pushed back by the gap; if it's more than 40 stronger he gets KH1's big recoil (0x6F) with no push. That recoil is also what KH1's own Counterattack waits for, so with KH1's Counterattack equipped you should get KH1's counter instead of KH2's Counterguard.
>    - Checked in the KH1 decomp (the clash, the recoils, the guard row's data in btltbl) and the exe (the recoils' first bytes). One assumption: I take an enemy attack's priority from its attack data. The enemy's own recoil is the same at both settings.
> 2. **The KH1 settings: you were partly right.** I went through the code:
>    - **What does switch at KH1:** the plain swings. KH1's own picker chooses the row and KH1's motions play. At a ledge, KH2's drop-ahead test is off and KH1's own row runs. That's KH1's swing that walks him off the edge, so it can look a lot like KH2's little hop.
>    - **What still comes from KH2, on purpose:**
>      - With **Finishing Plus or Negative Combo** equipped, both combos are KH2's whatever the Config rows say (t488, your (b) from m15). That includes the ledge hop. The log says it once: "move style: a combo ability only KH2 has is equipped". I think this is the most likely reason for what you saw.
>      - **KH2's ability moves** stay KH2's at either setting (t460): Slide Dash, Flash Step, Slapshot, Dodge Slash, Aerial Sweep / Dive / Spiral / Finish, Vicinity Break, Explosion, Magnet Burst and Guard Break. Some of these are KH1's own abilities under KH2 names (Sliding Dash, Vortex, Ripple Drive, Hurricane Blast, Stun Impact, Gravity Break), so having those equipped also brings KH2 moves.
>    - **What was actually wrong, t537:** at KH1, the code still let KH2's combo length decide when one of those ability moves came. With Combo Plus past KH1's length it could even hand Sora to KH2's controller for a plain KH2 swing, and a finisher like Explosion could come before KH1's finisher. Now, at KH1, a KH2 pick only counts if it's an ability move at the point KH1's own count is at: a swing before KH1's length, or a finisher at it. Anything else is KH1's own row. The log says "... combo at the KH1 setting ... not taken, KH1's own row" when it drops one.
>
> I couldn't check any of this in game. Please test, with the log and a Select + Triangle trace:
> - Guard Repel on KH1 and on KH2: block weak and strong enemies, then try KH1's Counterattack after a KH1 block. Log lines: "Guard Repel KH1: KH1's block" and "KH2's guard stopped a hit".
> - Both combos on KH1 with no KH2 abilities equipped, on flat ground and next to a ledge, then again with only Finishing Plus equipped.
>
> Questions:
> 1. Did you have Finishing Plus or Negative Combo equipped when you saw KH2 moves at KH1? (a) yes, then that's t488 working, (b) no, then please send that log.
> 2. At the KH1 setting, what should happen to KH2's ability moves? (a) keep them, as now (KH2's picker decides when they come, inside KH1's count), (b) let only KH1's own picker bring an ability move, so a KH2 move comes only where KH1 would put its own ability row and KH2-only ones like Slapshot never come at KH1, or (c) none at all at KH1.
> 3. Should Finishing Plus / Negative Combo keep overriding the KH1 rows? (a) yes, as you chose in m15, or (b) no, the rows always win (those abilities would then do nothing at KH1).

^m18


I think you fundamentally don't understand what I want, if an ability is equipped thats kh2s and your combat config is set to kh1 then you should be able to do kh1s moves, and its row picks, we need to modify kh1s so that it can choose explosion and other moves during its kh1 combo. the ability should not ignore the combat settings, 

do a new reimplementation and come back to me when your finished

UPDATE:  if your still unsure I need you to patch the extra functions that kh2 uses in kh1, thats the goal of native not to have separate systems, but if possible allow to patch in extra mechanics into the game while keeping the 1:1 feel of kh2 and kh1 this seems to have been done with kh2s effects, the extra functions they call are now apart of kh1. 

go ahead


> [!tip] Assistant
> t557 and t558. You're right, I had it backwards: at the KH1 setting KH2's picker was still running beside KH1's and handing it moves. That's gone now. KH1's own picker does the picking, and KH2's ability moves are new choices inside it.
>
> How it works now, with Base / Aerial Base Combo on KH1:
>
> 1. **t557, KH1's picker picks KH2's moves.** I hooked KH1's two row pickers (ground and air, the functions its Attack step calls for every swing).
>    - KH1's picker runs first, exactly as in vanilla: its count, its combo length, when the finisher comes, its own abilities' rows and their chances, its dash, its target.
>    - Then the new part runs inside the picker. If KH1 picked one of its plain rows (a normal swing, its dash or its finisher), your equipped KH2 ability moves get checked next, the same way KH1 checks its own ability rows before its plain ones.
>    - Each KH2 move uses KH2's own check for that row (distance, height, facing, one enemy or any enemy around, the ability equipped). The kind is decided by KH1's count: before KH1's finisher only KH2 ability swings (Slapshot, Slide Dash, Vicinity Break, Aerial Spiral...), at KH1's finisher only KH2 ability finishers (Explosion, Guard Break, Aerial Finish, Magnet Burst...).
>    - The first one that passes is what the picker returns, and it plays on KH1's row (KH2's animation, KH1's hit and timing), like the native moves already did.
>    - If none passes, or KH1 picked one of its own ability rows, KH1's row plays as normal.
>    - KH2's own selector, and KH2's combo length, aren't used for the combo at this setting anymore. t537's filter is gone too.
>    - Check: KH1's pickers and Attack step in the Steam KH1 decomp, the hook bytes in your exe, and KH2's row checks are the same ones checked against the KH2 decomp earlier.
> 2. **t558, the abilities follow your settings.** Finishing Plus and Negative Combo no longer switch the combo to KH2's (that was t488). At KH1 they now work on KH1's own combo:
>    - **Negative Combo** shortens KH1's own combo length by one per copy (never below 1, KH2's rule). It's patched into the length KH1's picker reads, so KH1 itself brings the finisher a swing sooner.
>    - **Finishing Plus:** after one of KH2's ability finishers (Explosion etc.), its Finishing Plus window lets KH1 take another press. KH1's picker then picks the next finisher, a KH2 one or KH1's own, up to 1 + your copies.
>    - Check: KH2's length and finisher-count rules from the KH2 source, and where KH1's pickers read the length in the decomp.
>
> Neither is tested in game. Please try these, with the log and a Select + Triangle trace:
> - Base Combo KH1 with Explosion / Vicinity Break / Slide Dash equipped, near one enemy and in a group.
> - Aerial Base Combo KH1 with Aerial Spiral / Aerial Finish / Magnet Burst.
> - Negative Combo, then Finishing Plus, at KH1.
>
> Useful log lines start with "KH1's ground row picker", "KH1's aerial row picker" and "Attack in KH1's finisher at the KH1 setting".
>
> Questions:
> 1. Order inside KH1's picker: (a) KH1's own ability rows first, then KH2's, then KH1's plain rows (what I did); (b) KH2's ability rows before KH1's own ones; or (c) KH1's own ability rows and KH2's ones on the same random chance?
> 2. Finishing Plus after KH1's own finisher: KH1's finisher animation has no Finishing Plus window, so right now only KH2's ability finishers can chain. (a) leave it like that; (b) give KH1's finishers a window (I'd need to pick the frames, since KH1 has none); or (c) something else?
> 3. The Square moves in KH1's combo window (Upper Slash, Finishing Leap, Horizontal Slash) still use KH2's selector at this setting, because KH1 has no Square combo moves to patch. (a) leave them; (b) move them into KH1's picker the same way, as Square-only choices?

^m19


1.  (c) it should follow the logic of how kh2 picks I will tell you what moves should have what:
	1. gravity break and blitz follow guard break
	2. ripple drive, stun impact should follow explosions logic
	3. hurricane blast should follow aerial finish 
		-an ability should always take priority over the normal finisher, if you have a finishing plus equipped but not an extra ability to fill that slot then the ability is a normal finisher (like kh2) if you have a finishing plus and no abilities then both are normal finishers, make sure you check your notes, if you still are unsure check the c steam decomp to make sure I'm right with kh2
2. (b)
3. (b) find a way to make it native with kh1's picker.

when it comes to it we now have two pickers, if its native thats good but what I don't get is how come you don't just patch it to where kh2 moves can have the same logic they do inside of kh1's picker it should be as if your patching extra features into kh1's mechanics, not taking away from kh1 but adding more that the engine can do. 


> [!tip] Assistant
> You're right that there's still a seam. Here's what the code does today at the KH1 setting:
> - **Attack:** KH1's picker is the only picker. But when it lands on a plain row, it calls a separate list of KH2 ability rows, which runs KH2's checks on its own KH2 object after KH1's own ability rows. So the KH2 rows are tacked on after KH1's walk, not inside it.
> - **Square moves, Counterguard and Retaliating Slash** still call KH2's selector, then hand the pick to KH1's Attack step, which skips KH1's picker for them.
>
> Your idea is the right fix: one list inside KH1's own walk, with KH1's rows and the KH2 rows in a single order. KH2's row checks become an extra test KH1's walk runs on a row, and KH2's separate selector goes away for these. Nothing gets taken out of KH1.
>
> I checked KH2's rule in the Steam decomp (`0x140402C50` walks the rows, `0x1403D7850` is the finisher test). You're right, with one catch:
> - KH2 walks its rows in table order and takes the first one that passes. There's no random chance.
> - With one enemy, an equipped ability finisher always beats the normal finisher.
> - With Finishing Plus, the next finisher skips the move that's still playing. So it's another ability if you have one, otherwise the normal finisher. With no abilities you get 155, then 154. With two Finishing Plus the first ability comes back on the third. The same move never plays twice in a row.
> - The catch: in a group, KH2's own table puts its normal finisher 154 ahead of Guard Break when a second enemy is close and off to the side, and 184 ahead of Aerial Finish in the air. Explosion and Magnet Burst still come first.
>
> The plan, one version each:
> 1. **Your mapping.** Stun Impact and Ripple Drive take Explosion's place and checks, Gravity Break and Blitz take Guard Break's, and Hurricane Blast takes Aerial Finish's. That replaces KH1's 30% roll on Stun Impact and Gravity Break, and Blitz's "no enemies around" test. Those are KH1's own rules, so this changes how KH1's abilities behave at the KH1 setting. You asked for it, but I want to be clear about it.
> 2. **A Finishing Plus window on KH1's own finishers.**
> 3. **Square moves through KH1's picker.** Square in KH1's combo window gives KH1 an Attack press, and KH1's picker then only looks at the Square rows (Upper Slash, Horizontal Slash, Finishing Leap) with KH2's checks. Counterguard and Retaliating Slash would go the same way.
>
> Questions:
> 1. In a group: (a) KH2's exact order, so 154 / 184 can come before Guard Break / Aerial Finish when a second enemy is beside you, or (b) abilities always first, as you wrote?
> 2. When a KH1 ability and its KH2 twin are both equipped (say Explosion and Ripple Drive), which goes first: (a) the KH2 one, or (b) the KH1 one? Either way, with Finishing Plus they'd take turns.
> 3. KH1 has only one normal ground finisher, so a second normal finisher has nothing to pick, and right now you'd get no attack there. (a) KH1's finisher plays again, the way KH2's 155 hands over to 154, or (b) Finishing Plus only adds a slot when an ability can fill it?
> 4. The window on KH1's finishers:
>    - (a) KH2's own finisher window from 155 / 183, placed at the same point in KH1's finisher (I'd read the frames from your KH2 files; the placement is my assumption).
>    - (b) the same spot as KH1's combo window on its normal swings.
>    - (c) the last 6 frames, the rule we already use for Wisdom's finisher (ours, not KH2's).
> 5. Square when no Square move passes: (a) Square does its normal roll or guard, as in KH2, or (b) nothing?
>
> If that sounds right, give me a go-ahead with your picks and I'll do the three items as their own versions.

^m20


1. (a) like kh2 (I had an assumption and it was wrong)
2. for explosion it should come last, for hurricane blast it should come first. 
3. have kh1 play a kh2 normal finisher
4. (a)
5. (a)

one thing why can't we include kh2's combat configs in kh1's picker? shouldn't the goal be to have kh1's engine have more abilities and the config just switches what the logic does in kh1's picker so that its 1:1 with kh2?


> [!tip] Assistant
> Got your picks. Here's how I read them:
>
> 1. **(a):** KH2's exact order in a group. With a second enemy close and off to the side, 154 can come before Guard Break, and 184 before Aerial Finish.
> 2. **The twins:** with Explosion and its KH1 twins (Ripple Drive, Stun Impact) all equipped, the KH1 ones go first and **Explosion comes last**. With Hurricane Blast and Aerial Finish both equipped, **Hurricane Blast comes first**. With Finishing Plus they take turns in that order. Tell me if I've got that backwards.
> 3. The second normal finisher is a KH2 normal finisher on KH1's finisher row.
> 4. **(a):** KH2's finisher window from 155 / 183, put at the same point in KH1's finisher.
> 5. **(a):** if no Square move passes, Square does the normal roll or guard.
>
> **On your question: yes, that should be the goal, and the code isn't there yet.** Here's what the two settings do today (checked in the code):
>
> - **Base Combo KH1:** KH1's own picker (`0x1402A6EE0` ground, `0x1402A6C70` air) makes the real choice. Our hook then asks a copy of the KH2 controller to check KH2's ability rows. KH2's checks are right, but they run on that copy, not inside KH1's walk.
> - **Base Combo KH2:** this is the bigger seam. KH1's picker still runs and picks a KH1 row. Then, in the step that runs the row:
>   - we copy the whole KH2 controller and have that copy "press Attack" at KH1's count;
>   - KH1's pick is thrown away and swapped for a KH1 row that plays KH2's motion;
>   - KH2's combo length is faked by letting KH1's swing row run past KH1's own length;
>   - a few ability moves with no KH1 row still hand Sora to the KH2 controller for that one move.
>
>   So it plays like KH2, but it's two pickers, and the second overrides the first after the fact.
>
> **What you're describing, as I'd build it:** one picker inside KH1's walk, with the Config row only switching which rule set that walk uses.
> - **KH1 setting:** KH1's rows, count, length and its own abilities. KH2's ability rows go into the same list with your mapping and order, and KH2's checks run as a test on each row.
> - **KH2 setting:** KH2's table and order in the same walk: first passing row wins, no random chance, KH2's anti-repeat and its fallback rows. KH2's length goes into the length KH1's picker reads (the way Negative Combo is already patched in), so KH1 itself brings the finisher where KH2 would. KH2's finisher count runs in the same walk.
> - **Either setting:** the row checks are fed from KH1's own data (its count, the request playing, its target and lock, the enemies around him). The copied KH2 controller, the after-the-fact swap and the leftover hand-offs all go away. Nothing is taken out of KH1: its engine gets a bigger table and a second rule set.
>
> **So the 3-item plan becomes part of a bigger change.** In this order, one system per version:
> 1. **One walk in KH1's picker, KH1 rule set:** KH1's rows and KH2's ability rows in one list, with your mapping, the twin order and KH2's group order. This replaces KH1's 30% roll on Stun Impact and Gravity Break, and Blitz's no-enemies test.
> 2. **KH2 rule set in the same walk:** Base / Aerial Base Combo KH2 moves onto it, and KH2's separate selector, the row swap and the last hand-offs go.
> 3. **Square moves, Counterguard and Retaliating Slash** as Square-only rows in that walk, at both settings. Square with nothing passing does the normal roll or guard.
> 4. **Finishing Plus on KH1's finishers:** KH2's 155 / 183 window at the same point in KH1's finisher, the next normal finisher a KH2 one.
>
> What should change in play: at KH2 it should play the same as now, minus the hand-off seams on the moves that still borrowed Sora, and with less work per press. At KH1, KH1's own ability finishers follow KH2's order instead of KH1's 30% rolls (your mapping). Drive Forms are untouched.
>
> Still to check, not guess: where KH2's finisher count lives on KH1's side (KH2 keeps it at `+0xD75`; I'll look for a spare field in KH1's combat info before deciding), and placing the 155 / 183 window inside KH1's finisher is my assumption, as I said last time.
>
> Questions:
> 1. Gravity Break and Blitz next to Guard Break, when both are equipped: (a) KH1's first, Guard Break last, like the Explosion group; (b) Guard Break first, like Hurricane Blast / Aerial Finish; or (c) KH2's own row order decides, with Gravity Break and Blitz taking Guard Break's place?
> 2. The second normal finisher on the ground: (a) KH2's own check picks it, 154 with an enemy in its window, else 155 (in the air 184 or 183 the same way), as KH2 does; or (b) always 155?
>
> Say go ahead and I'll start with version 1, the one walk in KH1's picker with the KH1 rule set.

^m21
do 1. if its the best way to implement more features into kh1's already existing engine, you need to keep blitz's rule and lets make a toggle in the config for the 30 percent chance to exist, if it does then if the 30 percent chance roll fails then it picks a similar kh2 move. these will be in the gameplay config in a sub menu called Tweaks 

1. gravity break and blitz last, if its blitz and gravity equipped then blitz last, let me know how kh2 prioritizes moves, the reason I picked the ones I did is because:
		-one does aoe damage (explosion group), the other focuses on one enemy (guard break, etc.)
	let me know if my priority list aligns with kh2, check your notes and if your unsure check the c steam decomp
2. (a)
