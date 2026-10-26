# Baking the rest of the base animations

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)
- [2. Assistant: Thanks! Three versions, one step of the plan each:](#^m2)
- [3. Assistant: Thanks! Built as you picked, in t632: KH1's own leg IK on th](#^m3)
- [4. Assistant: Done, three versions:](#^m4)

---


> [!tip] Assistant
> I started this thread from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md), as you asked. It covers the base animations that aren't baked into KH1's slots yet. The skeleton and coat stay in the sleeves thread.
>
> **What we have now.** The combos, walk, run, the KH2 Fire / Blizzard / Thunder / Cure casts and some ability swings are already KH1 slots that KH1 plays itself. Idle, jump, high jump, fall, landing, glide, item use, and the Gravity / Stop / Aero casts are still drawn over KH1's pose every frame. Each of those costs a full retarget plus our foot IK per frame, and it's the KH1/KH2 switching the slot work was meant to get rid of.
>
> **How KH1 asks for these** (KH1 Steam decomp and Sora's own motion file):
> - **Idle** is request 0. KH1 asks for it when he stands still, and also queues it after almost every action. It has two slots, field 0 and battle 1, and KH1 picks between them itself.
> - **Jump takeoff** is request 4 (high jump: 9), started at `0x1402AF9F0`. The takeoff holds at its end.
> - **Apex:** `0x1402AF630` asks for the apex slot 6 frames before the top of the rise and queues the fall after it. Our physics already writes KH1's jump clock from KH2's arc, so that request fires on KH2's timing.
> - **Fall** is request 6, **landing** request 7. The landing locks him until its own "may be left" event, which is frame 12 of 28 in KH1's slot. That event *is* KH1's landing lock.
> - **Glide** is 0x71 slow / 0x72 fast, and 0x73 / 0x74 for the banks past 30°. KH1 re-picks them every frame. It ignores a request whose slot is already playing.
> - **Item** is 0x3E (on himself) / 0x3F (on a partner). The heal is a separate task with its own wait, so it isn't tied to the motion's timing. The motion only decides when he can act again and when the item sound plays.
> - **Gravity / Stop / Aero** have their own requests: 0x3A / 0x3B / 0x3C, and 0x87 / 0x88 / 0x89 in the air. They borrow Blizzard's and Thunder's slots. So they can be baked one by one without touching the KH2 casts we already have.
>
> **What KH2 does** (our rewrite of its controller): the jump clip plays while he rises, and the fall loop starts the frame he starts falling. Landing takes buttons from frame 8 and a jump at any frame. The glide doesn't restart when his speed changes. KH2 also has a glide-in-place (hover) that KH1 has no request for.
>
> **Plan (one system per version):**
> 1. Gravity / Stop / Aero on KH2's Thunder / Cure cast motions, as KH1 slots.
> 2. Item use, both the self and partner versions. Its end comes from KH1's own action end instead of our timer.
> 3. Fall and landing.
> 4. Jump and high jump. The apex slot becomes "KH2's jump held for 6 frames", so KH1's own early apex request hands over to the fall exactly at KH2's top. That replaces our per-frame swap.
> 5. Glide. Slow and fast point at one slot, so KH1's speed switch no longer restarts it, natively. The hover waits for the KH2-glide-in-KH1's-glide-state work.
> 6. Idle (two slots, field and battle).
>
> **The catch for idle and landing:** a raw bake has plain legs. Today's drawn-over idle gets our foot placement on slopes and stairs; a baked one would stand flat. The native fix is the "hybrid legs" step: the legs keep KH1's own leg IK and ground lift, aimed at KH2's ankles.
>
> **Couldn't find yet:** which move uses 0x76..0x79 in Sora's own set (Superglide or flight). Also a third jump set KH1 uses with some actor flag, and how long the item heal waits.
>
> The notes are in `docs/kh1-anim-baking.md`.
>
> Questions:
> 1. Idle and landing feet: (a) build the hybrid legs first (KH1's own leg IK on our baked slots), then bake idle and landing on top of it; (b) bake idle now with flat feet, hybrid legs later; (c) bake everything but idle, and leave idle drawn over until the hybrid legs exist.
> 2. Where should KH1's landing lock end? (a) at KH2's reaction window (frame 8: buttons and stick both free from there); (b) at 14, our current stick hold; (c) at 8, with the stick still held to 14 by our layer as now.
> 3. KH1's item sound on the KH2 item motion: (a) keep KH1's, moved to the same point of KH2's motion; (b) drop it and play only KH2's own item sound and voice.

^m1


1. I don't want a hybrid layer just do (a) without baking on top of it
2. same as kh1 for now
3. (b) kh2 voice plays if the toggle for kh2 voices is on

go ahead


> [!tip] Assistant
> Thanks! Three versions, one step of the plan each:
>
> - **t625, item use.** Using an item now plays KH2's item animation right inside KH1's own item move, on yourself and on a partner, when Misc Animations is KH2. You can move again where KH2 lets you (frame 36 of 50), and the move ends where KH1's own action ends, not on a timer of ours. As you picked (b), KH1's item sound is gone and KH2's own item sound plays; KH2's item voice only plays with the KH2 voices (Battle Quotes) toggle on. Checked against KH1's item command code (Steam `0x1402A3DB0`), Sora's motion file and KH2's item motion. I couldn't check when the heal itself lands, since it runs on its own timer.
> - **t626, falling.** KH2's fall is now a real KH1 slot, and KH1 loops it the way KH2 does (from frame 12, not from the start). The switch from the jump into the fall is KH2's too: 5 frames before the top with an 8-frame blend, or right away when you let go of jump early. KH1's own "apex" animation no longer plays at Jump KH2. Checked in both games' code (KH1's apex step `0x1402AF630`, KH2's jump update `0x1403CB680`) and the files. One side effect: a drawn animation right after a baked one now blends instead of snapping. You'll see it from the fall into the landing, and from walk/run into idle, which used to snap.
> - **t627, jump and High Jump.** KH2's jump and High Jump are now KH1's own takeoff slots. One difference from before: in KH2's exe the High Jump's flip also switches to the fall 5 frames before the top, like the normal jump, while our old rewrite let it play to the end. I went with the exe. KH1's takeoff sounds and dust stay where KH1 has them (the takeoff is frame 4 in both games).
> - **Gravity / Stop / Aero and the glide's slow/fast** were already KH1 slots before this round (t563 and the glide thread), so nothing changed there.
> - **Landing lock:** stays KH1's own, as you said.
>
> None of it is checked in game yet. Please test:
> - items on yourself and on a partner (the sound, the voice with the toggle on and off, when you can move again);
> - plain jumps, tapped jumps and High Jumps (when the fall starts; the High Jump's flip cut at the top);
> - walking off ledges and a long fall (it should loop smoothly);
> - the landing blend, and walk/run into idle.
>
> One question about your answer 1 ("I don't want a hybrid layer just do (a) without baking on top of it"). It decides idle and the landing, so I want to be sure:
> - (a) build KH1's own leg IK onto the baked slots (feet on slopes and stairs), but don't bake idle and landing yet;
> - (b) bake idle and landing now with flat feet, and no leg layer at all;
> - (c) leave idle and landing drawn over like now.
>
> Until you answer, idle and the landing stay drawn over as before.

^m2


(a)

go ahead
> [!tip] Assistant
> Thanks! Built as you picked, in t632: **KH1's own leg IK on the baked animations.** Idle and the landing aren't baked yet; they're drawn over like before.
>
> - **What changed.** All of KH1 Sora's own animations move his legs through KH1's leg rig: each foot follows a target, and when the ground under it is higher, KH1 bends the leg so the foot lands on it. That's what keeps his feet on slopes and stairs. Our baked animations (walk, run, combos, casts, item, fall, jump, High Jump) used to have plain legs. Now they keep KH1's leg rig, with its targets set to KH2's ankles and feet from the animation. So on flat ground you see KH2's legs, and on slopes and stairs KH1 bends them onto the ground itself, the same as in its own animations. Nothing of ours runs per frame for this.
> - **Our own foot fix** still only runs on the animations we draw over KH1 (idle, landing, glide). It never ran on the baked ones, so the two never stack.
> - **Check.** KH1's bone pass and IK solver in the Steam decomp and exe (`0x1403900B0`, `0x14038FC20`); Sora's own model data says which bones KH1 lifts (the thighs) and by how much (his ankle height, 17.4). Offline, KH1's replayed pass rebuilds KH2's legs within about half a unit on ten baked animations.
> - **Two small differences you might notice:** the small hip joint above each thigh now points at the ankle, the way KH1 always points it; and where a KH2 swing dips a foot slightly into the floor (a couple of units in some swings), KH1 now lifts it out, as it does in its own animations.
> - **Switch.** `kh1_leg_ik = off` in kh1-kh2.ini brings back the old plain legs, if you want to compare.
> - **Not checked:** in game. One assumption: KH1's switch that turns its ground lift on should be on during our animations as during its own (it was on in your earlier trace).
>
> Please test:
> - walking and running up and down stairs and slopes, in and out of battle;
> - ground combos and casts on uneven ground (the feet should sit on the steps);
> - jumping and falling onto a slope (the take-off and fall are baked, the landing is still drawn);
> - anything odd around the hips or the top of the shorts.
>
> Questions:
> 1. Do you see any change at the hips or the top of the shorts compared with the last build? If it looks wrong, we can look at KH1's skin weights on that hip joint.
> 2. Once this looks right, should idle and the landing be baked next on top of it? Then they'd use KH1's own lift too, and our drawn-over foot fix could go.

^m3


lets actually have the foot iks off we'll work on this later when we do soras cloth animations, just make sure every animation is baked and on the native system

go ahead
> [!tip] Assistant
> Done, three versions:
>
> - **t634, foot IK off.** Both kinds of foot placement are off now: the baked animations get plain legs again instead of KH1's leg IK, and our own foot fix on the drawn-over clips is off too. The code is kept for when we do the cloth. `foot_ik = on` in kh1-kh2.ini turns both back on if you want to compare. Nothing else of ours moves the feet; KH1's own animations still use KH1's own lift, since that's KH1's.
> - **t635, idle baked.** KH2's idle is now two real KH1 slots: KH1 picks field or battle idle by itself, so KH2's field idle (no Keyblade) is on KH1's field idle and KH2's battle idle on its battle one. Everything that asks KH1 for its idle now gets KH2's, played by KH1's own animation system: standing still, the end of landings, items, casts and hits, and scripts. KH2's battle idle is longer than anything we'd baked before (300 frames), so the bake now handles long animations too. Checked against KH1's idle request in Sora's motion file and KH1's code (`0x1402A01E0`), and rebuilt offline from your files (within about a third of a unit of KH2's pose).
> - **t636, landing baked.** KH2's landing is now KH1's landing slot, for the normal jump and the High Jump. As you asked earlier, the landing lock is KH1's own: you can act again at KH1's frame 12, and KH1's landing sound and dust stay where they were. Checked against KH1's landing request in Sora's motion file and KH1's player step.
> - **Glide** was already fully baked (slow, fast, both banks and the hover), so nothing changed there.
>
> With these, every base-Sora animation that has a KH1 move to sit on is baked: idle, walk, run, jump, High Jump, fall, landing, combos, ability moves, Aerial Dodge, Aerial Recovery, guard, Counterguard, dodge roll, Quick Run, glide, every cast (Gravity, Stop, Aero and Reflect too), items, flinches and recoils. The drawn-over code is only a fallback now, for a slot that fails to build or a world whose motion set lacks the move. The audit is in `docs/kh1-anim-baking.md` §12.
>
> None of it is checked in game yet. Please test:
> - standing still in and out of battle (field idle without the Keyblade, battle idle with it), and going into walk and run;
> - landing from jumps, High Jumps and long falls: when you can move again, the sound and dust;
> - slopes and stairs: the feet won't adjust now, that's expected;
> - cutscenes and talks: KH1's idle requests there now show KH2's idle too.
>
> Two I couldn't bake without your call:
> 1. **Getting knocked back** (the big hits that send Sora flying). KH1 plays one 72-frame motion; KH2 plays a flying motion, then a separate landing motion when he touches down, and KH2's launch arc isn't on KH1's system yet. (a) leave KH1's knock-back animation for now; (b) bake KH2's flying motion onto KH1's knock-back now (the landing part later, with KH2's launch); (c) do the whole KH2 knock-back (arc, flying, landing) as its own job next.
> 2. **Ledge grabs.** KH1-only right now, but KH2 has its own ledge hang and climb. (a) keep KH1's; (b) find KH1's ledge slots and bake KH2's hang and climb onto them.

^m4


