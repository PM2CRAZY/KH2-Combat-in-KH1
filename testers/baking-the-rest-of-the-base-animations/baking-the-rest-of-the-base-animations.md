# Baking the rest of the base animations

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)

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


