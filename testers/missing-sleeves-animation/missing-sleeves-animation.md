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

