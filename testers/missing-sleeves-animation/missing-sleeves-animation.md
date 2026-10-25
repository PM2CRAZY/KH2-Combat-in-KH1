# Missing sleeves animation

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: I'm in the process of getting ready to transfer all the kh2 ](#^m1)
- [2. Assistant: t375](#^m2)
- [3. Mr Bugs: 4. do (b) and (c), if there isn't a matching equivalent to s](#^m3)
- [4. Assistant: t385 to t389 (one part each; the build at the top of the bra](#^m4)
- [5. Mr Bugs: I sent a trace and a new log](#^m5)
- [6. Assistant: t405 to t407 (one each)](#^m6)
- [7. Mr Bugs: the model is still bugged, it seems to be his clothes, (his ](#^m7)
- [8. Assistant: t411 and t412](#^m8)
- [9. Mr Bugs: when I mean model in this case I'm also referring to kh2 sor](#^m9)
- [10. Assistant: t453. Sorry I missed your message last round; it should have](#^m10)
- [11. Assistant: I went through the KH2 Steam decomp step by step, from where](#^m11)
- [12. Assistant: t490 and t491. Everything now follows what KH2's own model s](#^m12)
- [13. Assistant: t499 and t500.](#^m13)
- [14. Assistant: t519.](#^m14)
- [15. Assistant: t552.](#^m15)
- [16. Assistant: Sorry, I see the mix-up now. Two things changed along the wa](#^m16)

---


  

> [!note] Mr Bugs
> I'm in the process of getting ready to transfer all the kh2 animations into kh1s internals and I noticed on kh2's animations they're missing animations for the sleeves can you double check and add them to our animations? I've logged sora running in kh1 and sora running in kh2, if there's any other missing parts to an animation let me know after you've implemented the changes (logs and traces are in Missing sleeves animation)

^m1


> [!tip] Assistant
> t375
>
> Good catch, you were right. Under KH2's moves the sleeves had no animation at all: they stayed fixed to the upper arm like a solid tube. Both games animate the sleeves in their moves (KH2 does it with keyframes, not cloth physics); we just never copied KH2's over. Now each sleeve hangs and flaps the way KH2's sleeve does in the same move, both live and in the baked animations. KH2 has 2 flaps per sleeve and KH1 has 4, so the front and back follow KH2's front and back flaps, and the top and bottom are a mix of the two. The sleeves follow the same switches as the hair (`hair_physics`, `kh2_hair`).
>
> **Check:** the game files. I ran it over all 89 of KH2's motions on both arms: nothing breaks, every sleeve follows KH2's direction within 0.03°, and the sleeves never move further from the arm than KH1's own motions take them. Not checked: in game, and whether KH2's exe moves the sleeves any other way at runtime (the motion files have no constraints on them). How KH1's 4 flaps share KH2's 2 is my judgement from where they sit. Notes are in `docs/kh1-secondary-motion.md`. Your traces had no bone data in them, so the cause came from the model and motion files.
>
> **Other parts that still don't move under KH2's moves:**
> - the shorts panels (KH2 animates its own trouser panels, so we could copy those)
> - the jacket hem (KH2 has matching cloth at the hips)
> - the belt chain, the necklace and the pieces on the front of the belt (KH2 has nothing matching these)
> - KH2 also squashes and stretches some cloth, the sleeve flaps included. We only copy turns for now.
> - The other way round: with the KH2 player model on KH1's own moves, KH2's sleeves, hair, hood and cloth stay still.
>
> Questions:
> 1. What should I do next: (a) the shorts panels from KH2's trousers, (b) the jacket hem from KH2's hip cloth, (c) both, or (d) leave them for now?
> 2. Should I copy KH2's squash and stretch too? Yes or no.
> 3. The belt chain and necklace: leave them still, or give them a little swing? A swing would be physics, which goes against the "baked, not physics" rule, so I'd only do it if you want it.

^m2


> [!note] Mr Bugs
> 4. do (b) and (c), if there isn't a matching equivalent to something on kh1's soras body we shouldn't add it, after your done give me a list of similarities that are shared between models so I can make sure nothing is being copied that shouldn't.
> 5. yes do it
> 6. if you can bake a custom animation for the small parts kh2 sora doesn't have that kh1 has following the motion of each animation to give them similar looking results to kh1's animations that would be great, use kh1's animations as a reference. 
>    
>    "H2 has 2 flaps per sleeve and KH1 has 4, so the front and back follow KH2's front and back flaps, and the top and bottom are a mix of the two. The sleeves follow the same switches as the hair (`hair_physics`, `kh2_hair`)."
>    
>    this was a misunderstanding from the user's part, kh2 does not have "physics" but is instead baked so remove those switches out of the code, they're unneeded. If your unsure check the rust decomp, if it isn't there check the kh2 steam gihidra c decomp.
>    
>    can you also bake the animations for kh2s walking and running cycles? its more easy for me to compare.
> 
> go ahead

^m3


> [!tip] Assistant
> t385 to t389 (one part each; the build at the top of the branch has them all)
>
> First, something I found that changes the picture. In KH2's own model, every vertex is attached to exactly one bone, and a lot of the cloth bones KH2 animates have no mesh on them at all: the hood bones, the sleeve flaps, three hair spikes, the ring of cloth round the chest that lines up with KH1's jacket hem, all the trouser bones and the pelvis pieces. So in KH2 they don't visibly move anything. Copying them onto KH1 would add motion KH2's Sora doesn't show (the hem would sit flared out), so by your rule I didn't copy them for the hem and shorts.
>
> - **t385, shared parts.** New: KH1's necklace now swings with KH2's necklace (that one really does move on KH2's model). Everything copied from KH2 to KH1 now: hair spikes (KH1 153–192 from KH2 145–163), hood (KH1 80–83 from KH2 72/73), keychain (KH1 283–291 from the Keyblade's 3–11), sleeves (KH1 129/133/137/141 and 233/237/241/245 from KH2 110/202; KH1 130/134/138/142 and 234/238/242/246 from the flaps 113/115/205/207), the necklace (KH1 86 from KH2 165), plus the body. Of those, the hood, the flaps and spikes 143/159/161 have no mesh in KH2. The full table with every bone, and whether each KH2 bone shows on KH2's model, is in `docs/kh1-secondary-motion.md`.
> - **t386, squash and stretch** is carried now, live and baked. Right now it only shows on the sleeve flaps, because every other KH2 scale key is on a bone with no mesh.
> - **t387, KH1-only parts** (belt chain, belt-front pieces, shorts, jacket hem, the necklace's pendant) now play KH1's own animation of the same move, stretched to KH2's timing. So KH2's run gets KH1's run's chain swing and hem flap, and a KH2 swing gets KH1's swing's. They ride on KH2's body and are still kept off the thighs. I went with KH1's own animation rather than inventing one, as the closest to what you described.
> - **t388, the hair and physics switches are gone** (`hair_physics`, `kh2_hair`), and so is our physics swing. KH2 really does just key these parts: the KH2 source has no cloth or hair physics, and nothing in KH2's motions drives them another way. One side effect: the keychain on a Dream weapon or the wooden sword now keeps KH1's rest pose instead of swinging.
> - **t389, KH2's walk and run are baked** into KH1's walk and run slots (KH2's field cycle on the field version, its battle cycle on the battle ones), under the Misc Animations style like before.
>
> **Check:** the game files (all of KH2's motions) and the KH1 decomp (how it scales bones and plays the walk and run). Not checked in game. Assumptions: that the Steam game draws this same KH2 model, and that KH2's and KH1's runs start on the same foot.
>
> Questions:
> 1. The hood, sleeve flaps and three hair spikes we already copy have no mesh in KH2. (a) keep copying them, or (b) follow only what KH2 visibly moves (the hood stays with the head, the sleeves without the flap motion, those spikes stay with the head)?
> 2. Hem and shorts: (a) keep KH1's own animation, or (b) copy KH2's hidden cloth bones anyway (flared hem, twisted panels)?
> 3. The baked walk and run play at KH2's own pace, so the feet can slide when you move at other speeds. (a) keep it exactly 1:1, or (b) speed the animation up and down with your movement, like the old version did?

^m4


> [!note] Mr Bugs
> I sent a trace and a new log 
> 
> for some reason, standing  and returning to neutral cause parts of soras model to stretch like crazy
> 
> for the custom animation parts for kh1s model lets set that aside and work on it later, remove them from animation for now.
> 
> 1. (b) only do what kh2 does so its 1:1
> 2. (a)
> 3. (a)

^m5


> [!tip] Assistant
> t405 to t407 (one each)
>
> 1. **t405, the stretching.** Found it, from your trace: it was the shorts, the jacket hem and the belt pieces. Every time Sora goes back to idle (and while standing), KH1 blends between motions, and that blend skips the small joint bones those parts hang from. So we were measuring them against our own last frame instead of KH1's, and they got pulled to where KH1's idle has them, off KH2's hips, stretching the skin between. They're now measured against KH1's real placement. Your other suspects checked out clean (the squash values stay between 0.59 and 1.71).
> 2. **t406, KH1's own animation:** taken off the belt chain, the belt-front pieces and the necklace pendant. They hang like before t387. I kept it on the jacket hem and shorts because of your "2. (a)". If you meant those too, say so and I'll take them off as well.
> 3. **t407, 1:1 with KH2:** the hood now just turns with the head, the sleeves move as a whole with no flapping or squashing, and the three hair spikes that have no mesh in KH2 stay with the head. That's how KH2's model actually shows them. The walk and run stay exactly as they are.
>
> **Check:** the KH1 decomp (which bones KH1's blend pass writes) and the game files (all of KH2's motions, no broken poses). Not checked in game.
>
> Questions:
> 1. Hem and shorts: (a) keep KH1's own animation on them, or (b) remove it too, so they're rigid like the belt?
> 2. With the flaps now rigid, nothing on KH1's model gets KH2's squash and stretch any more (the flaps were the only matched parts with it). Is that fine? (a) yes, or (b) look for something else that should carry it?

^m6


> [!note] Mr Bugs
> the model is still bugged, it seems to be his clothes, (his coat, his pants) and return to the map origin for 1 frame (actually don't know if that is the map origin but its returning to one spot every time)
> 
> I've attached a picture, sent a log and a trace.
> 
>1. (a) you can keep the animation but like I said earlier I think its bugged when you transition from a running state to a neutral one
> 
> 2. Yes you can leave it for now, when I know the animations are done we can start adding custom polish to it. 
>    
>    another thing: the hoodie doesn't seem to be animated anymore
>    
>    go ahead

^m7


> [!tip] Assistant
> t411 and t412
>
> 1. **t411, the coat and pants jumping to one spot.** Found it, and you were right that it's the run-to-neutral change. When KH1 blends between motions, it ends by adding Sora's world position to every joint, even the joints the blend didn't move. We were measuring the coat and pants off those joints, so they were a whole world position off and got drawn at the map origin. In your trace that's every stop from walk to idle and the few blended frames after each one, so a short flash, not just one frame. Fixed now: those joints are recognised as KH1 not having placed them.
> 2. **t412, the hood.** I looked at how KH2's model is actually built, and it has no hood that sticks out from the back. What it shows of one is part of the upper back on the torso, so in KH2 it moves exactly with the torso and never on its own. In t407 I'd mistaken the back of the hair for it and had the hood turning with the head; that's what you were seeing go. So 1:1, the hood now stays put on the back, which is what KH2 shows but is even less animated than before.
>
> **Check:** the KH1 decomp for how its passes place joints, and KH2's model file for the hood. Not checked in game. Assumption: that the patch on KH2's upper back is the hood (I judged it from where it sits; there's nothing else it could be).
>
> Question:
> 1. The hood: (a) keep it 1:1 (still on the back, like KH2's model), (b) give it KH1's own hood animation of the move, like the hem and shorts have, or (c) put back KH2's hood keys (they move, but nothing on KH2's model shows them)?

^m8


> [!note] Mr Bugs
> when I mean model in this case I'm also referring to kh2 sora's skeleton, before I prompted you to adjust the bake animations the coat was working how it should, please double check so that we're on the same page.
> 
> I'll start being more technical and referring to his skeleton  
> 
> I saw you also ignored this message after I pushed it to github, make sure to have an agent check the github after it pushes to see if there's a thread not being worked on thats been recently pushed
> 
> go ahead

^m9


 


> [!tip] Assistant
> **t453.** Sorry I missed your message last round; it should have been picked up after you pushed, and now it is checked after every push.
>
> You're right, and we're on the same page now. The coat broke in t387, the update where I gave KH1's small parts KH1's own animation. From then on the coat copied KH1's own coat animation for each move, which was made for KH1's body and stride, not KH2's. The stretching and the jump to the map origin both came from that, and my t405 and t411 fixes only patched the symptoms. The coat is now exactly what it was before t387: it turns with KH2's hips, the front panels lift a little with the thighs, and it's kept off the legs. I compared it against the old version over every one of KH2's motions and it matches exactly.
>
> About the skeleton: KH2's skeleton does have its own coat bones, a ring of bones hanging from the chest. They have no mesh on KH2's model, and KH2's motions keep them flared out, which is why the old coat follows KH2's hips instead.
>
> **Check:** the KH2 files (every motion in Sora's mset against the t386 coat) and his skeleton. Not checked in game. The pants are unchanged (question 1).
>
> Questions:
> 1. Pants: (a) keep KH1's own animation on them, or (b) put them back to how they were before t387, fixed to the thigh, like the coat?
> 2. Now that you mean the skeleton too: the hood, the sleeve flaps and three hair spikes are animated in KH2's skeleton but have no mesh, so I've been holding them still. (a) follow KH2's skeleton again (hood animated, sleeves flapping and squashing), or (b) keep them still?
> 3. Coat: (a) keep it like before (follows KH2's hips), or (b) follow KH2's own coat bones one-to-one (they'd hold it flared out)?

^m10


I want you to investigate into the kh2 steam c decomp how exactly soras animations work (the hair, cloth, coat, etc) do not make assumptions, come back once you understand how to implement everything properly so that the animations look proper. 

go ahead


> [!tip] Assistant
> I went through the KH2 Steam decomp step by step, from where the game updates Sora each frame to where his bones end up. The short version: KH2 has no physics, springs or cloth simulation anywhere in that path. Each frame it plays the motion's keyframes, then its constraints, limits and IK (only on the body: legs, spine, arms, hands, head), adjusts the feet to the ground, and blends between motions for a few frames when one changes. Nothing carries over from the last frame, so nothing swings or settles on its own. The hair, hood, sleeves, necklace and coat are only ever moved by their keyframes.
>
> The bigger finding is what KH2's model actually shows. Its hair spikes, sleeves (each sleeve as one piece) and necklace have mesh and move. Its jacket is part of the torso mesh, so it moves rigidly with the torso. The skeleton also has coat bones, hood bones, sleeve flaps, three hair spikes and pants bones that are animated in every motion but have no mesh, so in KH2 you never see them move. I don't know why KH2 animates bones it doesn't show. My guess is that another Sora model on the same skeleton uses them; I can check that if you put his world-outfit models (Halloween Town, Christmas Town, Space Paranoids, Timeless River) or the Drive Form models in this thread's folder.
>
> So in KH2 terms, our current coat (following his hips) is our own version, not KH2's, and the pants with KH1's own animation aren't KH2's either. Everything KH2 does here is keyframes, so whichever way you pick, it gets built into KH1's own animations with no extra system running.
>
> All of this is in `docs/kh1-secondary-motion.md` ("How KH2's exe animates Sora"), checked in the decomp and the KH2 files. No code changed this round.
>
> Questions:
> 1. Pants: (a) keep KH1's own animation, or (b) fixed to the thigh, which is exactly what KH2 shows?
> 2. The animated bones KH2 doesn't show (hood, sleeve flaps, 3 hair spikes): (a) follow KH2's skeleton anyway, so they move on KH1's model (the hood needs new matching work, its keys sit far from its rest pose), or (b) keep them still, like KH2's model shows them?
> 3. Coat: (a) keep it as it is now (follows his hips), (b) make it move with his torso like KH2's jacket, or (c) follow KH2's coat bones (they move, but nothing on KH2's model shows them)?
>
> I also noticed KH1 may blend between animations differently from KH2: KH2 always turns the short way, and I haven't confirmed KH1 does. I'll check that next and tell you if it differs.

^m11


to answer your last response, removing the test:

make everything 1:1 with how its animated in kh2, if there's no equivalences like the ones we talked about then don't add them.

go ahead


> [!tip] Assistant
> **t490 and t491.** Everything now follows what KH2's own model shows: where KH2 animates a bone but draws nothing on it, that motion isn't copied.
>
> - **t490, pants.** KH2 draws its shorts on its hips and on bones the thighs carry; its trouser bones are keyed but have no mesh. So the shorts hang fixed on the thigh again, and KH1's own animation is off them. One small difference: in a few KH2 moves (203 to 208, 42, 171) the thigh's shorts bone is keyed 9° to 30° off the leg. On KH1 that would turn the whole thigh, so it isn't carried.
> - **t491, coat.** KH2's jacket is drawn on its torso bone only, and its coat bones have no mesh. So the coat now moves rigidly with Sora's chest, like KH2's jacket. The old follow-the-hips motion, the thigh lift on the front panels and the push off the legs are gone, because they were ours.
> - **No change:** the hood, the sleeve flaps and the three hair spikes stay still, because KH2 has no mesh on those bones. The whole sleeves, hair, necklace and keychain keep following KH2; I checked the keychain, KH2's chain has mesh on every link. The parts only KH1 has (belt chain, belt-front pieces, pendant) get no new motion.
>
> Checked: KH2's model and all 83 of its motions in the game files. Not tested in game.
>
> Please test: high kicks, landings and swings, and watch the front of the coat around the thighs; also running and the idle.
>
> Question: KH1's coat is longer than KH2's jacket. It hangs down to where KH2 has its waist and the top of the shorts, so when a leg comes up its front tips go into the thigh (kicks, landings and some swings: motions 155, 8, 167, 196, ...). KH2's own jacket never gets that low. (a) keep it exactly like KH2's jacket, clipping included; (b) keep it rigid with the chest but push the panels off the thighs only when they'd go in (our own guard, not KH2's); or (c) have the lower part follow what KH2 draws at that height (its waist and hips) and the upper part the chest?
>
> Separately, a finding for the body retarget (not changed here, it's another system): its thigh roll differs from KH2's visible thigh bone by up to 43° to 56° in a few motions (7, 8, 40, 183 and some swings). I can look at it in its own thread if you want.

^m12


keep researching until we have a good looking animations that emulate kh2s animations as close as possible while still making it look natural to kh1's model

go ahead
> [!tip] Assistant
> **t499 and t500.**
>
> - **t499, the legs:** KH2 draws its thighs on the leg bones of its shorts and its shins on its knee bones. Our body retarget was guessing their twist from where the knee pointed. In some moves (the ledge climb, Aerial Recovery, some air moves and swings) that was 40° to 110° off what KH2 actually draws. Now each thigh and shin twists exactly as KH2's bone does: within 0.2° in all 83 of KH2's motions, and nothing snaps more than KH2's own keys do. KH1's shorts hang on the thigh, so they now turn exactly like KH2's shorts too. It covers the live poses and the baked animations.
> - **t500, the coat:** I measured eleven ways of moving the coat over every KH2 motion: following KH2's waist, following the hips (turned, placed or only twisted), hanging, letting the thighs lead the front panels, and several ways of pushing it off the thighs. Keeping it rigid on the chest, like KH2's jacket, came out best. It only goes into the shorts for a few frames, right at the top of a thigh, in side kicks, splits and spins (about 0.15% of the time). At that spot the coat could go either over the thigh or around it. So every push I tried either flipped from one side to the other within a frame (even in the first swing) or arrived after the kick was over. Following the hips made it clip more, not less. So the coat stays as it is. t500 only adds the measuring tool.
>
> **Check:** your KH2 model and all 83 motions in the game files. Not tested in game.
>
> Please test: kicks, splits, spins and the ledge climb. Watch the shorts and how the legs twist.
>
> Question: the coat's quick clips at the top of the thigh. (a) leave it like KH2's jacket; (b) try a slow, smoothed push anyway (it barely helps and lags behind); or (c) send me a clip of the move where it bothers you, and I'll look at that one?

^m13


the coat is still sticking out on kh2 animations

don't worry about the kh2 model for now

go ahead

> [!tip] Assistant
> **t519.**
>
> You were right, and my tool wasn't looking at it. It only measured the coat going into the thighs, never the coat standing away from the body. I added that measure, and it showed the problem straight away. KH2 leans Sora's chest forward of his hips in the idle, walk and run, and the coat was fixed to the chest. So the back of the coat stood about 6 units out behind his hips and the front sank about 7 into them, the whole time, not just in kicks. In KH2's frames it stood out in 42% of them. KH1's own coat, in KH1's own animations, does that in about 15%.
>
> Now the top of the coat still moves with the chest like KH2's jacket, but the bottom hangs from the hips. The tips sit where the hips carry them, the middle halfway, and nothing stretches. Why this is the KH2 match: KH1's coat is longer than KH2's jacket. Its tips reach the height of KH2's waist and the top of its shorts, and KH2 moves the top of its shorts with the hips. After the change it never stands out more than 1.5 units in any KH2 move, and the front no longer sinks into the hips.
>
> The cost: the front tips touch a thigh that swings forward a bit more often than before (about 0.6% of the time instead of 0.2%). That's in kicks and big steps, and up to about 2 units into the shorts in the run. I also measured seven other ways (the whole lower coat turning with the hips, hanging from the spine, the front panels following the thighs, KH2's waist bone, ...). This one was the best overall. All the numbers are in `docs/kh1-secondary-motion.md` ("The coat's lower half").
>
> **Check:** your KH1 and KH2 files, all 83 of KH2's motions, every half frame. Not tested in game, and your message had no new picture or trace, so I worked from what the measure shows. Assumptions: how thick KH1's shorts are (I can't read the mesh yet), and that KH1's own animations measure the same without KH1's leg IK.
>
> Please test: standing in battle and out of it, walking, running and turning, and look at the coat from the side and from behind. Also try a few kicks and big swings and watch the front tips against the thighs.
>
> Questions:
> 1. If the front tips going into the thighs bother you in the run or kicks: (a) leave it; (b) let the front panels follow the thighs a little (fewer clips, but they tuck into the belt more, and it's our own rule, not KH2's); or (c) send a clip of the move and I'll look at that one.
> 2. If I read "sticking out" wrong (for example you meant the thigh sticking out through the coat), tell me which move and roughly where on the coat, and I'll measure that instead.

^m14


1. we'll fix the clipping later
2. when I mean by sticking out is that the coat is stiff and not animated at all. redo the coat so that its actually animated
   
   I've sent you a log and a trace showing a kh2 animation with the coat sticking out and stiff, this is happening on all kh2 animations that we've baked so far. its in the same folder as this thread.

go ahead


> [!tip] Assistant
> **t552.**
>
> Thanks, that clears it up: you meant the coat never moves, not that it stands off his hips. You're right, since t491 it's been stuck rigid to his chest in every KH2 move, live and baked.
>
> Now it moves the way KH2 animates its own coat. KH2's skeleton has a ring of coat bones under the chest (a back piece and four panels a side), laid out just like KH1's coat, and every KH2 motion keys them. I'd read them wrong before: I said they "hold it flared out", but they're actually keyed to hang down from the chest when he leans forward in the battle stance, and they sway with every step and swing. KH2's own jacket is too short to show it, but KH1's coat is long enough. So each of KH1's nine coat panels now turns exactly as its KH2 bone does relative to the chest. The panel keeps KH1's own shape and length, and the top stays on the chest.
>
> How much it moves now: about 10 to 20 degrees of sway in the idle and the run, about 33 on average across all of KH2's moves, and up to about 110 in big swings, jumps and when he gets hit. The field idle barely moves, same as KH2's.
>
> **Check:** KH2's skeleton and the coat bones' keys in all 83 of Sora's KH2 motions (game files). A test checks every KH1 panel stays within 0.3° of its KH2 bone and really swings in the run and the swings. Both the baked animations and the live pose use it. Not tested in game, and your new trace had no bone data in it.
>
> Since it's 1:1, it clips more than before (you said to leave that for later):
> - In the run and walk, his chest is turned a little against his hips, so the right front panel's tip sits in the front of the shorts and the left one swings out.
> - In jumps, spins and hit reactions the panels fly up past horizontal for a few frames (about 1% of the time).
>
> Please test: the battle idle, walking, running and turning, a few combos, a jump and getting hit. Watch whether the coat moves now and whether the motion looks right, ignoring the clipping for now.
>
> Question for when we get to the clipping: (a) leave it fully 1:1 for now, (b) keep KH2's motion but limit how far up a panel can fly, or (c) keep KH2's motion but keep the front tips out of the shorts (that part would be our own, not KH2's).

^m15


your currently fundamentally misunderstanding me, sora's hoodie still stays rigid, its currently worse than it was when we had physics around t375, that had better animations what do you think instead we approximate the animation and put it on soras coat, hoodie, legs etc. it needs to look natural and his clothes not clip. 


> [!tip] Assistant
> Sorry, I see the mix-up now. Two things changed along the way:
>
> - **What you liked around t375 wasn't physics.** The hood followed KH2's own hidden hood bones (KH2 keys them in every move but draws no mesh on them). On top of that, a rule of ours kept it off his back and head. That's why it swayed so much.
> - **Why it's rigid now.** In t407 / t412 we went fully 1:1 with what KH2's model *shows*. KH2 shows no hood, so the hood got locked to his chest, and it hasn't moved since. That's stiffer than KH1's own animations, which turn it 13° to 31°. The shorts went rigid on the thigh the same way (t490). The coat follows KH2's hidden coat bones now (t552), but those clip.
>
> So you're right: "only what KH2 shows" leaves the hood dead, because KH2 never shows one. Making these parts look natural without clipping has to be our own rule on top of KH2's motion, since KH2 has nothing to copy for it. I'll mark it as ours in the docs.
>
> How we could do it:
> - (a) **Bring back a physics layer.** The clothes lag and settle behind KH2's pose and are kept out of the body. It feels the most alive. But KH2 has no physics, and it brings back the splaying, clipping and stutter we had with the old hair physics.
> - (b) **Borrow KH1's own cloth animation** from the closest KH1 move (KH1's run for KH2's run). We tried this for the coat in t387, and it stretched and jumped because KH1's moves don't line up with KH2's stride.
> - (c) **My pick.** Start from KH2's hidden hood, coat and trouser keys, which really do move with every step and swing. Then smooth them and add clearance, so the hood stays off his back and head and the coat tips and shorts stay off the thighs. Everything KH2 visibly animates (hair, sleeves, necklace, keychain) stays exactly KH2's. It's all worked out from the motion, so it bakes into KH1's own animations with nothing extra running each frame.
>
> Questions:
> 1. Is (c) the look you want, or do you want the clothes to lag and settle after he stops, like physics? That needs (a) or something like it.
> 2. Which parts: (a) hood, coat and shorts; (b) those plus the belt chain and the belt-front pieces (KH2 has nothing for those, so their motion would be fully ours); or (c) the hood first, so you can judge the look?
> 3. The log in your folder is from t544, before the coat change. The screenshot there (Olympus Coliseum) shows the Keyblade's long red swing trail, not his clothes, so I think it belongs to another thread. If you can, send a t552 or later trace of a run, the idle and a swing.
>
> If (c) sounds right, say go ahead and I'll start with the hood.

^m16

do (c)

1. no it should be like kh2
2. (a) just do a test bake into soras running and walking animations for now so I can see how it looks.
3. don't worry about it

go ahead